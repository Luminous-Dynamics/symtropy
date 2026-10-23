// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic scalar-channel sensor emulation for replayable robotics tests.
//!
//! This models the effects declared by SensorProfileV1: sample timing, bounded
//! timestamp jitter, fixed latency, frame dropout, bounded zero-mean noise,
//! and scalar saturation. It is not a camera renderer, lidar ray caster,
//! IMU bias/random-walk model, acoustic propagation model, or calibrated
//! physical sensor. Those require modality-specific implementations.

use crate::world_contract::SensorProfileV1;
use serde::{Deserialize, Serialize};
use std::fmt;

const NANOS_TIMES_NANO_HERTZ: u128 = 1_000_000_000_000_000_000;
const NANO_HERTZ_PER_HERTZ: f64 = 1_000_000_000.0;
const MAX_SENSOR_CHANNELS: usize = 4096;
const SPLITMIX_INCREMENT: u64 = 0x9e37_79b9_7f4a_7c15;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorTruthChannelV1 {
    pub quantity: String,
    pub value: f64,
    pub unit: String,
    pub frame_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorReadingV1 {
    pub quantity: String,
    pub value: f64,
    pub unit: String,
    pub frame_id: String,
    /// Declared scalar noise standard deviation in the same unit as value.
    pub uncertainty_std: f64,
    pub saturation_applied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorSampleDispositionV1 {
    Delivered,
    Dropped,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorSampleV1 {
    pub schema_version: u16,
    pub sensor_id: String,
    pub sample_index: u64,
    pub scheduled_time_ns: u64,
    pub acquisition_time_ns: u64,
    pub delivery_time_ns: Option<u64>,
    /// Scheduled samples skipped because the caller advanced the world clock.
    pub missed_sample_count: u64,
    pub disposition: SensorSampleDispositionV1,
    pub readings: Vec<SensorReadingV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorModelError {
    InvalidProfile(&'static str),
    InvalidState,
    CaptureNotDue { now_ns: u64, next_due_ns: u64 },
    InvalidTruthChannelCount { actual: usize },
    InvalidTruthChannel { index: usize },
    NoiseUnitMismatch { index: usize },
    NonFiniteReading { index: usize },
    ClockOverflow,
    SampleIndexOverflow,
}

impl fmt::Display for SensorModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(reason) => write!(formatter, "invalid sensor profile: {reason}"),
            Self::InvalidState => write!(
                formatter,
                "serialized sensor state does not match its profile or sample schedule"
            ),
            Self::CaptureNotDue { now_ns, next_due_ns } => write!(
                formatter,
                "sensor capture at {now_ns} ns is early; next sample is due at {next_due_ns} ns"
            ),
            Self::InvalidTruthChannelCount { actual } => write!(
                formatter,
                "sensor truth has {actual} channels; expected 1 to {MAX_SENSOR_CHANNELS}"
            ),
            Self::InvalidTruthChannel { index } => {
                write!(formatter, "invalid sensor truth channel at index {index}")
            }
            Self::NoiseUnitMismatch { index } => write!(
                formatter,
                "sensor noise unit does not match truth channel {index}"
            ),
            Self::NonFiniteReading { index } => {
                write!(formatter, "sensor model produced a non-finite reading at index {index}")
            }
            Self::ClockOverflow => write!(formatter, "sensor timestamp arithmetic overflowed"),
            Self::SampleIndexOverflow => write!(formatter, "sensor sample index overflowed"),
        }
    }
}

impl std::error::Error for SensorModelError {}

/// Counter-based randomness means a sample is a pure function of world seed,
/// sensor identity and scheduled sample index. Skipped samples cannot perturb
/// future noise or dropout draws; separate sensors receive separate streams.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorModelV1 {
    schema_version: u16,
    profile: SensorProfileV1,
    world_seed: u64,
    sample_rate_nanohz: u64,
    next_sample_index: u64,
    next_due_time_ns: u64,
}

impl SensorModelV1 {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(
        world_seed: u64,
        profile: SensorProfileV1,
    ) -> Result<Self, SensorModelError> {
        validate_profile(&profile)?;
        let sample_rate_nanohz = (profile.sample_rate_hz * NANO_HERTZ_PER_HERTZ).round() as u64;
        if sample_rate_nanohz == 0 {
            return Err(SensorModelError::InvalidProfile(
                "sample rate is below the one-nanohertz clock resolution",
            ));
        }
        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            profile,
            world_seed,
            sample_rate_nanohz,
            next_sample_index: 0,
            next_due_time_ns: 0,
        })
    }

    pub fn profile(&self) -> &SensorProfileV1 {
        &self.profile
    }

    pub fn next_sample_index(&self) -> u64 {
        self.next_sample_index
    }

    pub fn next_due_time_ns(&self) -> u64 {
        self.next_due_time_ns
    }

    fn validate_state(&self) -> Result<(), SensorModelError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(SensorModelError::InvalidState);
        }
        validate_profile(&self.profile)?;
        let expected_rate =
            (self.profile.sample_rate_hz * NANO_HERTZ_PER_HERTZ).round() as u64;
        if expected_rate == 0 || expected_rate != self.sample_rate_nanohz {
            return Err(SensorModelError::InvalidState);
        }
        if sample_time_ns(self.next_sample_index, self.sample_rate_nanohz)?
            != self.next_due_time_ns
        {
            return Err(SensorModelError::InvalidState);
        }
        Ok(())
    }

    /// Acquire at the supplied simulation timestamp.
    ///
    /// If the world has stepped past due times, the latest due sample is
    /// generated using truth at the actual acquisition time, and skipped slots
    /// are counted. No earlier truth is fabricated and no catch-up burst occurs.
    pub fn capture_at(
        &mut self,
        acquisition_time_ns: u64,
        truth: &[SensorTruthChannelV1],
    ) -> Result<SensorSampleV1, SensorModelError> {
        self.validate_state()?;
        validate_truth(&self.profile, truth)?;
        if acquisition_time_ns < self.next_due_time_ns {
            return Err(SensorModelError::CaptureNotDue {
                now_ns: acquisition_time_ns,
                next_due_ns: self.next_due_time_ns,
            });
        }

        // Find the latest sample index whose quantized timestamp is <= the
        // supplied world clock. u128 avoids overflow for u64 clock values.
        let due_index = (((acquisition_time_ns as u128 + 1)
            * self.sample_rate_nanohz as u128)
            - 1)
            / NANOS_TIMES_NANO_HERTZ;
        let due_index = u64::try_from(due_index).map_err(|_| SensorModelError::ClockOverflow)?;
        let missed_sample_count = due_index
            .checked_sub(self.next_sample_index)
            .ok_or(SensorModelError::ClockOverflow)?;
        let scheduled_time_ns = sample_time_ns(due_index, self.sample_rate_nanohz)?;
        let next_sample_index = due_index
            .checked_add(1)
            .ok_or(SensorModelError::SampleIndexOverflow)?;
        let next_due_time_ns = sample_time_ns(next_sample_index, self.sample_rate_nanohz)?;

        let mut rng = SplitMix64::new(sample_seed(
            self.world_seed,
            &self.profile.sensor_id,
            due_index,
        ));
        let dropped = rng.unit_f64() < self.profile.dropout_probability;

        let (disposition, delivery_time_ns, readings) = if dropped {
            (SensorSampleDispositionV1::Dropped, None, Vec::new())
        } else {
            let jitter = rng.symmetric_integer(self.profile.jitter_ns);
            let latency = self.profile.latency_ns as i128;
            // Profile validation guarantees |jitter| <= latency, so this is
            // truly symmetric jitter with no truncation-induced timing bias.
            let delivery = acquisition_time_ns as i128 + latency + jitter;
            if delivery < acquisition_time_ns as i128 || delivery > u64::MAX as i128 {
                return Err(SensorModelError::ClockOverflow);
            }
            let mut readings = Vec::with_capacity(truth.len());
            for (index, channel) in truth.iter().enumerate() {
                // Twelve independent U[0,1) draws minus six give a bounded,
                // zero-mean, unit-variance approximation to Gaussian noise.
                // No platform-dependent transcendental functions are needed.
                let unit_noise = (0..12)
                    .map(|_| rng.unit_f64())
                    .sum::<f64>()
                    - 6.0;
                let noisy = channel.value + self.profile.noise_std * unit_noise;
                if !noisy.is_finite() {
                    return Err(SensorModelError::NonFiniteReading { index });
                }
                let value = match (self.profile.saturation_min, self.profile.saturation_max) {
                    (Some(min), Some(max)) => noisy.clamp(min, max),
                    _ => noisy,
                };
                if !value.is_finite() {
                    return Err(SensorModelError::NonFiniteReading { index });
                }
                readings.push(SensorReadingV1 {
                    quantity: channel.quantity.clone(),
                    value,
                    unit: channel.unit.clone(),
                    frame_id: channel.frame_id.clone(),
                    uncertainty_std: self.profile.noise_std,
                    saturation_applied: value != noisy,
                });
            }
            (
                SensorSampleDispositionV1::Delivered,
                Some(delivery as u64),
                readings,
            )
        };

        // Commit state only after successfully constructing the complete sample.
        // An error leaves the exact sample available for retry.
        self.next_sample_index = next_sample_index;
        self.next_due_time_ns = next_due_time_ns;
        Ok(SensorSampleV1 {
            schema_version: Self::SCHEMA_VERSION,
            sensor_id: self.profile.sensor_id.clone(),
            sample_index: due_index,
            scheduled_time_ns,
            acquisition_time_ns,
            delivery_time_ns,
            missed_sample_count,
            disposition,
            readings,
        })
    }
}

fn validate_profile(profile: &SensorProfileV1) -> Result<(), SensorModelError> {
    if profile.sensor_id.trim().is_empty() {
        return Err(SensorModelError::InvalidProfile("sensor identity is empty"));
    }
    if !profile.sample_rate_hz.is_finite()
        || !(1.0e-9..=1.0e9).contains(&profile.sample_rate_hz)
    {
        return Err(SensorModelError::InvalidProfile(
            "sample rate must be between 1 nHz and 1 GHz",
        ));
    }
    if !profile.noise_std.is_finite() || profile.noise_std < 0.0 {
        return Err(SensorModelError::InvalidProfile(
            "noise standard deviation must be finite and non-negative",
        ));
    }
    if profile.noise_unit.trim().is_empty() {
        return Err(SensorModelError::InvalidProfile("noise unit is empty"));
    }
    if !profile.dropout_probability.is_finite()
        || !(0.0..=1.0).contains(&profile.dropout_probability)
    {
        return Err(SensorModelError::InvalidProfile(
            "dropout probability must be within [0, 1]",
        ));
    }
    if profile.jitter_ns > i64::MAX as u64 {
        return Err(SensorModelError::InvalidProfile(
            "jitter magnitude exceeds signed timestamp range",
        ));
    }
    if profile.jitter_ns > profile.latency_ns {
        return Err(SensorModelError::InvalidProfile(
            "jitter magnitude must not exceed nominal latency",
        ));
    }
    match (profile.saturation_min, profile.saturation_max) {
        (None, None) => {}
        (Some(min), Some(max)) if min.is_finite() && max.is_finite() && min < max => {}
        _ => {
            return Err(SensorModelError::InvalidProfile(
                "saturation bounds must both be absent or finite with min < max",
            ));
        }
    }
    Ok(())
}

fn validate_truth(
    profile: &SensorProfileV1,
    truth: &[SensorTruthChannelV1],
) -> Result<(), SensorModelError> {
    if truth.is_empty() || truth.len() > MAX_SENSOR_CHANNELS {
        return Err(SensorModelError::InvalidTruthChannelCount {
            actual: truth.len(),
        });
    }
    for (index, channel) in truth.iter().enumerate() {
        if channel.quantity.trim().is_empty()
            || !channel.value.is_finite()
            || channel.unit.trim().is_empty()
            || channel.frame_id.trim().is_empty()
        {
            return Err(SensorModelError::InvalidTruthChannel { index });
        }
        if channel.unit != profile.noise_unit {
            return Err(SensorModelError::NoiseUnitMismatch { index });
        }
    }
    Ok(())
}

fn sample_time_ns(index: u64, sample_rate_nanohz: u64) -> Result<u64, SensorModelError> {
    let timestamp = (index as u128)
        .checked_mul(NANOS_TIMES_NANO_HERTZ)
        .ok_or(SensorModelError::ClockOverflow)?
        / sample_rate_nanohz as u128;
    u64::try_from(timestamp).map_err(|_| SensorModelError::ClockOverflow)
}

fn sample_seed(world_seed: u64, sensor_id: &str, sample_index: u64) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in sensor_id.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let mut mixer = SplitMix64::new(world_seed ^ hash.rotate_left(17));
    mixer.next_u64() ^ sample_index.wrapping_mul(SPLITMIX_INCREMENT)
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(SPLITMIX_INCREMENT);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn unit_f64(&mut self) -> f64 {
        const TWO_POW_53: f64 = 9_007_199_254_740_992.0;
        (self.next_u64() >> 11) as f64 / TWO_POW_53
    }

    fn below(&mut self, upper_exclusive: u64) -> u64 {
        let rejection_threshold = upper_exclusive.wrapping_neg() % upper_exclusive;
        loop {
            let value = self.next_u64();
            if value >= rejection_threshold {
                return value % upper_exclusive;
            }
        }
    }

    fn symmetric_integer(&mut self, magnitude: u64) -> i128 {
        let width = magnitude * 2 + 1;
        self.below(width) as i128 - magnitude as i128
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world_contract::SensorModality;

    fn profile() -> SensorProfileV1 {
        SensorProfileV1 {
            sensor_id: "imu-front".to_owned(),
            modality: SensorModality::Imu,
            frame_id: "imu_link".to_owned(),
            model_solver_id: None,
            calibration_artifact: None,
            sample_rate_hz: 100.0,
            latency_ns: 2_000_000,
            jitter_ns: 100_000,
            noise_std: 0.02,
            noise_unit: "m/s^2".to_owned(),
            dropout_probability: 0.0,
            saturation_min: Some(-20.0),
            saturation_max: Some(20.0),
        }
    }

    fn truth(value: f64) -> Vec<SensorTruthChannelV1> {
        vec![SensorTruthChannelV1 {
            quantity: "linear_acceleration_x".to_owned(),
            value,
            unit: "m/s^2".to_owned(),
            frame_id: "imu_link".to_owned(),
        }]
    }

    #[test]
    fn same_seed_sensor_and_sample_index_replay_identically() {
        let profile = profile();
        let mut left = SensorModelV1::new(0x5eed, profile.clone()).unwrap();
        let mut right = SensorModelV1::new(0x5eed, profile).unwrap();
        let sample_left = left.capture_at(0, &truth(1.25)).unwrap();
        let sample_right = right.capture_at(0, &truth(1.25)).unwrap();
        assert_eq!(sample_left, sample_right);
        assert_eq!(sample_left.sample_index, 0);
        assert_eq!(sample_left.scheduled_time_ns, 0);
        assert_eq!(sample_left.acquisition_time_ns, 0);
    }

    #[test]
    fn jitter_larger_than_latency_is_rejected_instead_of_clipped() {
        let mut profile = profile();
        profile.latency_ns = 1;
        profile.jitter_ns = 2;
        assert!(matches!(
            SensorModelV1::new(1, profile),
            Err(SensorModelError::InvalidProfile(
                "jitter magnitude must not exceed nominal latency"
            ))
        ));
    }

    #[test]
    fn delivery_time_obeys_latency_and_jitter_bounds() {
        let mut sensor = SensorModelV1::new(7, profile()).unwrap();
        let sample = sensor.capture_at(0, &truth(0.0)).unwrap();
        let delivered = sample.delivery_time_ns.unwrap();
        assert!((1_900_000..=2_100_000).contains(&delivered));
        assert!(delivered >= sample.acquisition_time_ns);
    }

    #[test]
    fn dropout_is_explicit_and_does_not_fabricate_readings() {
        let mut profile = profile();
        profile.dropout_probability = 1.0;
        let mut sensor = SensorModelV1::new(42, profile).unwrap();
        let sample = sensor.capture_at(0, &truth(4.0)).unwrap();
        assert_eq!(sample.disposition, SensorSampleDispositionV1::Dropped);
        assert_eq!(sample.delivery_time_ns, None);
        assert!(sample.readings.is_empty());
        assert_eq!(sensor.next_sample_index(), 1);
    }

    #[test]
    fn saturation_is_reported_not_silently_hidden() {
        let mut profile = profile();
        profile.noise_std = 0.0;
        let mut sensor = SensorModelV1::new(42, profile).unwrap();
        let sample = sensor.capture_at(0, &truth(25.0)).unwrap();
        assert_eq!(sample.readings[0].value, 20.0);
        assert!(sample.readings[0].saturation_applied);
    }

    #[test]
    fn late_world_tick_counts_skipped_samples_without_catch_up_burst() {
        let mut sensor = SensorModelV1::new(42, profile()).unwrap();
        let sample = sensor.capture_at(30_000_000, &truth(1.0)).unwrap();
        assert_eq!(sample.sample_index, 3);
        assert_eq!(sample.scheduled_time_ns, 30_000_000);
        assert_eq!(sample.missed_sample_count, 3);
        assert_eq!(sample.acquisition_time_ns, 30_000_000);
        assert_eq!(sensor.next_due_time_ns(), 40_000_000);
    }

    #[test]
    fn empty_or_oversized_truth_frame_is_rejected() {
        let mut sensor = SensorModelV1::new(42, profile()).unwrap();
        assert_eq!(
            sensor.capture_at(0, &[]),
            Err(SensorModelError::InvalidTruthChannelCount { actual: 0 })
        );
        let oversized = vec![
            truth(0.0).into_iter().next().unwrap();
            MAX_SENSOR_CHANNELS + 1
        ];
        assert_eq!(
            sensor.capture_at(0, &oversized),
            Err(SensorModelError::InvalidTruthChannelCount {
                actual: MAX_SENSOR_CHANNELS + 1
            })
        );
        assert_eq!(sensor.next_sample_index(), 0);
    }

    #[test]
    fn invalid_input_does_not_advance_replay_state() {
        let profile = profile();
        let mut retryable = SensorModelV1::new(42, profile.clone()).unwrap();
        let mut fresh = SensorModelV1::new(42, profile).unwrap();
        let mut wrong_unit = truth(1.0);
        wrong_unit[0].unit = "km/s^2".to_owned();
        assert!(matches!(
            retryable.capture_at(0, &wrong_unit),
            Err(SensorModelError::NoiseUnitMismatch { index: 0 })
        ));
        assert_eq!(
            retryable.capture_at(0, &truth(1.0)).unwrap(),
            fresh.capture_at(0, &truth(1.0)).unwrap()
        );
    }

    #[test]
    fn not_due_capture_does_not_advance_state() {
        let mut sensor = SensorModelV1::new(42, profile()).unwrap();
        sensor.capture_at(0, &truth(0.0)).unwrap();
        assert!(matches!(
            sensor.capture_at(1, &truth(0.0)),
            Err(SensorModelError::CaptureNotDue {
                now_ns: 1,
                next_due_ns: 10_000_000
            })
        ));
        assert_eq!(sensor.next_sample_index(), 1);
    }

    #[test]
    fn noise_is_seeded_and_bounded_by_six_standard_deviations() {
        let mut sensor = SensorModelV1::new(1234, profile()).unwrap();
        let sample = sensor.capture_at(0, &truth(3.0)).unwrap();
        assert!((sample.readings[0].value - 3.0).abs() <= 6.0 * 0.02);
    }

    #[test]
    fn independent_world_seeds_produce_distinct_replay_streams() {
        let profile = profile();
        let mut first = SensorModelV1::new(42, profile.clone()).unwrap();
        let mut second = SensorModelV1::new(43, profile).unwrap();
        let a = first.capture_at(0, &truth(1.0)).unwrap();
        let b = second.capture_at(0, &truth(1.0)).unwrap();
        assert_ne!(a.readings[0].value, b.readings[0].value);
    }

    #[test]
    fn fractional_sample_period_does_not_accumulate_rounding_drift() {
        let mut profile = profile();
        profile.sample_rate_hz = 3.0;
        profile.noise_std = 0.0;
        let mut sensor = SensorModelV1::new(1, profile).unwrap();

        let first = sensor.capture_at(0, &truth(0.0)).unwrap();
        assert_eq!(first.scheduled_time_ns, 0);
        assert_eq!(sensor.next_due_time_ns(), 333_333_333);

        let second = sensor.capture_at(333_333_333, &truth(0.0)).unwrap();
        assert_eq!(second.scheduled_time_ns, 333_333_333);
        assert_eq!(sensor.next_due_time_ns(), 666_666_666);

        let third = sensor.capture_at(666_666_666, &truth(0.0)).unwrap();
        assert_eq!(third.scheduled_time_ns, 666_666_666);
        assert_eq!(sensor.next_due_time_ns(), 1_000_000_000);
    }

    #[test]
    fn seeded_noise_has_zero_mean_and_declared_standard_deviation_at_scale() {
        let mut sensor = SensorModelV1::new(0x51_5e_05, profile()).unwrap();
        let mut values = Vec::with_capacity(1024);
        for index in 0..1024u64 {
            let sample = sensor
                .capture_at(index * 10_000_000, &truth(0.0))
                .unwrap();
            values.push(sample.readings[0].value);
        }
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values
            .iter()
            .map(|value| (value - mean) * (value - mean))
            .sum::<f64>()
            / values.len() as f64;
        let observed_std = variance.sqrt();
        assert!(mean.abs() < 0.15 * 0.02, "unexpected noise mean {mean}");
        assert!(
            (0.9 * 0.02..=1.1 * 0.02).contains(&observed_std),
            "unexpected noise std {observed_std}"
        );
    }

    #[test]
    fn deserialized_schedule_drift_is_rejected_before_sampling() {
        let mut sensor = SensorModelV1::new(42, profile()).unwrap();
        sensor.next_due_time_ns = 1;
        assert_eq!(
            sensor.capture_at(0, &truth(0.0)),
            Err(SensorModelError::InvalidState)
        );
        assert_eq!(sensor.next_sample_index(), 0);
    }

    #[test]
    fn timestamp_overflow_fails_without_consuming_sample() {
        let mut profile = profile();
        profile.latency_ns = u64::MAX;
        profile.jitter_ns = 0;
        let mut sensor = SensorModelV1::new(42, profile).unwrap();
        assert_eq!(
            sensor.capture_at(1, &truth(0.0)),
            Err(SensorModelError::ClockOverflow)
        );
        assert_eq!(sensor.next_sample_index(), 0);
        assert_eq!(sensor.next_due_time_ns(), 0);
    }
}
