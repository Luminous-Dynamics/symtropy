//! Deterministic synthetic spatiotemporal observations for HoloFrac.
//!
//! This fixture keeps evaluator-only truth separate from the observation stream.
//! It models irregular sampling, missing readings, delivery delay and explicit
//! correction events. It is synthetic benchmark input, never measured building data.

use std::collections::{BTreeMap, HashMap, HashSet};

pub const TEMPORAL_FIXTURE_SCHEMA_VERSION: u32 = 1;
pub const TEMPORAL_FIXTURE_GENERATOR: &str = "holofrac-spatiotemporal-fixture-v0";

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpatialChannel {
    Temperature,
    Illuminance,
    Humidity,
    Moisture,
    Airflow,
    OccupancyDensity,
}

impl SpatialChannel {
    pub const ALL: [Self; 6] = [
        Self::Temperature,
        Self::Illuminance,
        Self::Humidity,
        Self::Moisture,
        Self::Airflow,
        Self::OccupancyDensity,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Temperature => "temperature",
            Self::Illuminance => "illuminance",
            Self::Humidity => "humidity",
            Self::Moisture => "moisture",
            Self::Airflow => "airflow",
            Self::OccupancyDensity => "occupancy_density",
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Self::Temperature => "degC",
            Self::Illuminance => "lux",
            Self::Humidity => "%RH",
            Self::Moisture => "fraction",
            Self::Airflow => "m/s",
            Self::OccupancyDensity => "persons_per_100m2",
        }
    }

    const fn discriminant(self) -> u64 {
        match self {
            Self::Temperature => 0,
            Self::Illuminance => 1,
            Self::Humidity => 2,
            Self::Moisture => 3,
            Self::Airflow => 4,
            Self::OccupancyDensity => 5,
        }
    }

    fn noise_scale(self) -> f64 {
        match self {
            Self::Temperature => 0.25,
            Self::Illuminance => 12.0,
            Self::Humidity => 1.0,
            Self::Moisture => 0.012,
            Self::Airflow => 0.035,
            Self::OccupancyDensity => 0.12,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationKind {
    Observed,
    Missing,
    Correction,
}

impl ObservationKind {
    const fn discriminant(self) -> u64 {
        match self {
            Self::Observed => 0,
            Self::Missing => 1,
            Self::Correction => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordinateFrame {
    BuildingLocalMetric,
}

impl CoordinateFrame {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuildingLocalMetric => "building_local_metric",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SensorLocation {
    pub sensor_id: u32,
    pub x: f64,
    pub y: f64,
    pub calibration_id: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationEvent {
    pub event_id: u64,
    pub sensor_id: u32,
    pub channel: SpatialChannel,
    pub x: f64,
    pub y: f64,
    pub coordinate_frame: CoordinateFrame,
    pub valid_at_micros: i64,
    pub recorded_at_micros: i64,
    pub source_revision: u32,
    pub calibration_revision: u32,
    pub kind: ObservationKind,
    /// None is an explicit missing value; it is never interpreted as zero.
    pub value: Option<f64>,
    /// Corrections reference the exact event they supersede.
    pub supersedes_event_id: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GroundTruthRecord {
    pub sensor_id: u32,
    pub channel: SpatialChannel,
    pub x: f64,
    pub y: f64,
    pub valid_at_micros: i64,
    pub value: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixtureConfig {
    pub seed: u64,
    pub sensor_count: usize,
    pub time_steps: usize,
    pub nominal_step_seconds: i64,
    pub max_delivery_delay_seconds: i64,
    pub correction_delay_seconds: i64,
    pub missing_probability: f64,
    pub correction_probability: f64,
}

impl Default for FixtureConfig {
    fn default() -> Self {
        Self {
            seed: 0x484f_4c4f_4652_4143,
            sensor_count: 24,
            time_steps: 96,
            nominal_step_seconds: 900,
            max_delivery_delay_seconds: 2_700,
            correction_delay_seconds: 3_600,
            missing_probability: 0.12,
            correction_probability: 0.08,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FixtureManifest {
    pub schema_version: u32,
    pub generator: &'static str,
    pub config: FixtureConfig,
    /// Stable regression identifier only; this is not a cryptographic digest.
    pub digest_non_security: u64,
    pub event_count: usize,
    pub truth_record_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalFixture {
    pub manifest: FixtureManifest,
    pub sensors: Vec<SensorLocation>,
    /// Source event log, ordered by recorded time then event ID.
    pub events: Vec<ObservationEvent>,
    /// Evaluator-only truth. Never copied into decision snapshots.
    pub evaluator_truth: Vec<GroundTruthRecord>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TemporalSnapshot {
    pub decision_cutoff_micros: i64,
    /// Latest known event for each sensor/channel/valid-time key at this cutoff.
    pub observations: Vec<ObservationEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ObservationKey {
    sensor_id: u32,
    channel: SpatialChannel,
    valid_at_micros: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TemporalFixtureError {
    InvalidSensorCount,
    InvalidTimeSteps,
    InvalidTimeStep,
    InvalidDelay,
    InvalidProbability,
    DuplicateEventId(u64),
    InvalidEventTime(u64),
    InvalidEventValue(u64),
    InvalidEventShape(u64),
    UnknownCorrectionParent(u64),
    CorrectionKeyMismatch(u64),
    NonIncreasingRevision(u64),
}

impl std::fmt::Display for TemporalFixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidSensorCount => f.write_str("sensor_count must be at least four"),
            Self::InvalidTimeSteps => f.write_str("time_steps must be at least three"),
            Self::InvalidTimeStep => f.write_str("nominal_step_seconds must be positive"),
            Self::InvalidDelay => {
                f.write_str("delivery and correction delays must be non-negative")
            }
            Self::InvalidProbability => {
                f.write_str("missing and correction probabilities must lie in [0, 1]")
            }
            Self::DuplicateEventId(id) => write!(f, "duplicate temporal event ID: {id}"),
            Self::InvalidEventTime(id) => {
                write!(f, "event {id} has recorded time before valid time")
            }
            Self::InvalidEventValue(id) => {
                write!(f, "event {id} has a non-finite observation value")
            }
            Self::InvalidEventShape(id) => {
                write!(f, "event {id} has inconsistent kind/value/parent fields")
            }
            Self::UnknownCorrectionParent(id) => {
                write!(f, "correction {id} references an unknown parent")
            }
            Self::CorrectionKeyMismatch(id) => write!(
                f,
                "correction {id} changes the identity of its parent sample"
            ),
            Self::NonIncreasingRevision(id) => {
                write!(f, "correction {id} does not advance its source revision")
            }
        }
    }
}

impl std::error::Error for TemporalFixtureError {}

#[derive(Clone, Debug)]
struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9e37_79b9_7f4a_7c15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 7;
        self.state ^= self.state << 17;
        self.state
    }

    fn unit_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64)
    }

    fn signed_unit(&mut self) -> f64 {
        self.unit_f64() * 2.0 - 1.0
    }

    fn range_i64(&mut self, max_inclusive: i64) -> i64 {
        if max_inclusive <= 0 {
            0
        } else {
            (self.next_u64() % (max_inclusive as u64 + 1)) as i64
        }
    }
}

fn fract(value: f64) -> f64 {
    value - value.floor()
}

fn sensor_location(sensor_id: u32) -> SensorLocation {
    let index = sensor_id as f64;
    SensorLocation {
        sensor_id,
        x: fract(0.5 + (index + 1.0) * 0.618_033_988_749_894_8),
        y: fract(0.5 + (index + 1.0) * 0.754_877_666_246_692_7),
        calibration_id: 1 + sensor_id % 3,
    }
}

fn gaussian(x: f64, y: f64, cx: f64, cy: f64, radius: f64) -> f64 {
    let dx = x - cx;
    let dy = y - cy;
    (-(dx * dx + dy * dy) / (2.0 * radius * radius)).exp()
}

/// Deterministic analytic fixture truth at one position/time/channel.
///
/// Values are generated for benchmark falsification only; they are not a
/// calibrated model of any real building or material.
pub fn synthetic_field_value(channel: SpatialChannel, x: f64, y: f64, time_seconds: f64) -> f64 {
    let day_phase = std::f64::consts::TAU * time_seconds / 86_400.0;
    let heat_x = 0.5 + 0.28 * (day_phase * 0.5).sin();
    let heat_y = 0.5 + 0.22 * (day_phase * 0.37).cos();
    let heat = gaussian(x, y, heat_x, heat_y, 0.19);
    let moisture = gaussian(x, y, 0.18 + 0.08 * (day_phase * 0.3).sin(), 0.78, 0.16);
    let daylight = (0.5 + 0.5 * (day_phase - std::f64::consts::FRAC_PI_2).sin()).max(0.0);

    match channel {
        SpatialChannel::Temperature => 22.0 + 2.6 * (day_phase - 0.8).sin() + 3.2 * heat,
        SpatialChannel::Illuminance => {
            850.0 * daylight * (0.48 + 0.52 * (1.0 - 0.55 * gaussian(x, y, 0.68, 0.4, 0.24)))
        }
        SpatialChannel::Humidity => 48.0 + 7.0 * (day_phase + 0.6).sin() + 8.0 * moisture,
        SpatialChannel::Moisture => {
            (0.2 + 0.12 * (1.0 - x) + 0.035 * (day_phase * 0.4).sin() + 0.11 * moisture)
                .clamp(0.0, 1.0)
        }
        SpatialChannel::Airflow => {
            0.12 + 0.3 * (0.5 + 0.5 * (day_phase * 1.8 + x * 3.0).sin()) + 0.45 * heat
        }
        SpatialChannel::OccupancyDensity => {
            let occupied = (0.5 + 0.5 * (day_phase - 1.4).sin()).powi(2);
            (0.4 + 3.1 * occupied * gaussian(x, y, 0.58, 0.52, 0.29)
                + 1.2 * occupied * gaussian(x, y, 0.19, 0.22, 0.15))
            .max(0.0)
        }
    }
}

fn stable_digest(config: &FixtureConfig, events: &[ObservationEvent]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut feed = |value: u64| {
        for byte in value.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    };

    feed(TEMPORAL_FIXTURE_SCHEMA_VERSION as u64);
    feed(config.seed);
    feed(config.sensor_count as u64);
    feed(config.time_steps as u64);
    feed(config.nominal_step_seconds as u64);
    feed(config.max_delivery_delay_seconds as u64);
    feed(config.correction_delay_seconds as u64);
    feed(config.missing_probability.to_bits());
    feed(config.correction_probability.to_bits());

    for event in events {
        feed(event.event_id);
        feed(event.sensor_id as u64);
        feed(event.channel.discriminant());
        feed(event.x.to_bits());
        feed(event.y.to_bits());
        feed(event.valid_at_micros as u64);
        feed(event.recorded_at_micros as u64);
        feed(event.source_revision as u64);
        feed(event.calibration_revision as u64);
        feed(event.kind.discriminant());
        feed(event.value.map_or(u64::MAX, f64::to_bits));
        feed(event.supersedes_event_id.unwrap_or(u64::MAX));
    }
    hash
}

impl TemporalFixture {
    pub fn generate(config: FixtureConfig) -> Result<Self, TemporalFixtureError> {
        validate_config(&config)?;
        let mut rng = DeterministicRng::new(config.seed);
        let sensors: Vec<_> = (0..config.sensor_count)
            .map(|index| sensor_location(index as u32))
            .collect();
        let mut events = Vec::new();
        let mut evaluator_truth = Vec::new();
        let mut next_event_id = 1_u64;

        for sensor in &sensors {
            for channel in SpatialChannel::ALL {
                let mut previous_valid_at = -1_i64;

                for step in 0..config.time_steps {
                    let nominal = step as i64 * config.nominal_step_seconds;
                    let jitter_bound = (config.nominal_step_seconds / 10).max(1);
                    let jitter =
                        (rng.unit_f64() * (2 * jitter_bound + 1) as f64) as i64 - jitter_bound;
                    let valid_seconds = (nominal + jitter).max(previous_valid_at + 1).max(0);
                    previous_valid_at = valid_seconds;
                    let valid_at_micros = valid_seconds.saturating_mul(1_000_000);
                    let truth =
                        synthetic_field_value(channel, sensor.x, sensor.y, valid_seconds as f64);
                    evaluator_truth.push(GroundTruthRecord {
                        sensor_id: sensor.sensor_id,
                        channel,
                        x: sensor.x,
                        y: sensor.y,
                        valid_at_micros,
                        value: truth,
                    });

                    let recorded_at_micros = valid_at_micros.saturating_add(
                        rng.range_i64(config.max_delivery_delay_seconds)
                            .saturating_mul(1_000_000),
                    );
                    let missing = rng.unit_f64() < config.missing_probability;
                    let event_id = next_event_id;
                    next_event_id += 1;

                    if missing {
                        events.push(ObservationEvent {
                            event_id,
                            sensor_id: sensor.sensor_id,
                            channel,
                            x: sensor.x,
                            y: sensor.y,
                            coordinate_frame: CoordinateFrame::BuildingLocalMetric,
                            valid_at_micros,
                            recorded_at_micros,
                            source_revision: 1,
                            calibration_revision: sensor.calibration_id,
                            kind: ObservationKind::Missing,
                            value: None,
                            supersedes_event_id: None,
                        });
                        continue;
                    }

                    let raw_value = truth + rng.signed_unit() * channel.noise_scale();
                    events.push(ObservationEvent {
                        event_id,
                        sensor_id: sensor.sensor_id,
                        channel,
                        x: sensor.x,
                        y: sensor.y,
                        coordinate_frame: CoordinateFrame::BuildingLocalMetric,
                        valid_at_micros,
                        recorded_at_micros,
                        source_revision: 1,
                        calibration_revision: sensor.calibration_id,
                        kind: ObservationKind::Observed,
                        value: Some(raw_value),
                        supersedes_event_id: None,
                    });

                    if rng.unit_f64() < config.correction_probability {
                        let correction_recorded_at = recorded_at_micros.saturating_add(
                            config
                                .correction_delay_seconds
                                .saturating_add(
                                    rng.range_i64(config.max_delivery_delay_seconds / 2),
                                )
                                .saturating_mul(1_000_000),
                        );
                        let correction_id = next_event_id;
                        next_event_id += 1;
                        let corrected_value =
                            truth + rng.signed_unit() * channel.noise_scale() * 0.2;
                        events.push(ObservationEvent {
                            event_id: correction_id,
                            sensor_id: sensor.sensor_id,
                            channel,
                            x: sensor.x,
                            y: sensor.y,
                            coordinate_frame: CoordinateFrame::BuildingLocalMetric,
                            valid_at_micros,
                            recorded_at_micros: correction_recorded_at,
                            source_revision: 2,
                            calibration_revision: sensor.calibration_id + 1,
                            kind: ObservationKind::Correction,
                            value: Some(corrected_value),
                            supersedes_event_id: Some(event_id),
                        });
                    }
                }
            }
        }

        events.sort_by_key(|event| (event.recorded_at_micros, event.event_id));
        evaluator_truth
            .sort_by_key(|record| (record.valid_at_micros, record.sensor_id, record.channel));

        let fixture = Self {
            manifest: FixtureManifest {
                schema_version: TEMPORAL_FIXTURE_SCHEMA_VERSION,
                generator: TEMPORAL_FIXTURE_GENERATOR,
                config: config.clone(),
                digest_non_security: 0,
                event_count: events.len(),
                truth_record_count: evaluator_truth.len(),
            },
            sensors,
            events,
            evaluator_truth,
        };

        fixture.validate()?;
        let mut fixture = fixture;
        fixture.manifest.digest_non_security =
            stable_digest(&fixture.manifest.config, &fixture.events);
        Ok(fixture)
    }

    pub fn validate(&self) -> Result<(), TemporalFixtureError> {
        let mut event_ids = HashSet::with_capacity(self.events.len());
        let by_id: HashMap<u64, &ObservationEvent> = self
            .events
            .iter()
            .map(|event| (event.event_id, event))
            .collect();

        for event in &self.events {
            if !event_ids.insert(event.event_id) {
                return Err(TemporalFixtureError::DuplicateEventId(event.event_id));
            }
            if event.recorded_at_micros < event.valid_at_micros {
                return Err(TemporalFixtureError::InvalidEventTime(event.event_id));
            }
            if event.value.is_some_and(|value| !value.is_finite()) {
                return Err(TemporalFixtureError::InvalidEventValue(event.event_id));
            }

            match (event.kind, event.value, event.supersedes_event_id) {
                (ObservationKind::Observed, Some(_), None) => {}
                (ObservationKind::Missing, None, None) => {}
                (ObservationKind::Correction, Some(_), Some(parent_id)) => {
                    let Some(parent) = by_id.get(&parent_id) else {
                        return Err(TemporalFixtureError::UnknownCorrectionParent(
                            event.event_id,
                        ));
                    };
                    if parent.sensor_id != event.sensor_id
                        || parent.channel != event.channel
                        || parent.valid_at_micros != event.valid_at_micros
                    {
                        return Err(TemporalFixtureError::CorrectionKeyMismatch(event.event_id));
                    }
                    if event.source_revision <= parent.source_revision {
                        return Err(TemporalFixtureError::NonIncreasingRevision(event.event_id));
                    }
                    if event.recorded_at_micros <= parent.recorded_at_micros {
                        return Err(TemporalFixtureError::InvalidEventTime(event.event_id));
                    }
                }
                _ => return Err(TemporalFixtureError::InvalidEventShape(event.event_id)),
            }
        }
        Ok(())
    }

    /// Build the exact information available at a historical decision cutoff.
    ///
    /// Events recorded after the cutoff are excluded even when their valid time
    /// is earlier. Corrections supersede prior events only after they arrive.
    pub fn snapshot_as_of(&self, decision_cutoff_micros: i64) -> TemporalSnapshot {
        let mut latest: BTreeMap<ObservationKey, &ObservationEvent> = BTreeMap::new();

        for event in self
            .events
            .iter()
            .filter(|event| event.recorded_at_micros <= decision_cutoff_micros)
        {
            let key = ObservationKey {
                sensor_id: event.sensor_id,
                channel: event.channel,
                valid_at_micros: event.valid_at_micros,
            };
            let should_replace = latest.get(&key).is_none_or(|current| {
                (
                    event.source_revision,
                    event.recorded_at_micros,
                    event.event_id,
                ) > (
                    current.source_revision,
                    current.recorded_at_micros,
                    current.event_id,
                )
            });
            if should_replace {
                latest.insert(key, event);
            }
        }

        TemporalSnapshot {
            decision_cutoff_micros,
            observations: latest.values().map(|event| (*event).clone()).collect(),
        }
    }

    pub fn correction_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.kind == ObservationKind::Correction)
            .count()
    }

    pub fn missing_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.kind == ObservationKind::Missing)
            .count()
    }

    pub fn delayed_count(&self) -> usize {
        self.events
            .iter()
            .filter(|event| event.recorded_at_micros > event.valid_at_micros)
            .count()
    }
}

impl TemporalSnapshot {
    pub fn missing_count(&self) -> usize {
        self.observations
            .iter()
            .filter(|event| event.kind == ObservationKind::Missing)
            .count()
    }

    pub fn correction_count(&self) -> usize {
        self.observations
            .iter()
            .filter(|event| event.kind == ObservationKind::Correction)
            .count()
    }

    pub fn validate_cutoff(&self) -> bool {
        self.observations
            .iter()
            .all(|event| event.recorded_at_micros <= self.decision_cutoff_micros)
    }
}

fn validate_config(config: &FixtureConfig) -> Result<(), TemporalFixtureError> {
    if config.sensor_count < 4 {
        return Err(TemporalFixtureError::InvalidSensorCount);
    }
    if config.time_steps < 3 {
        return Err(TemporalFixtureError::InvalidTimeSteps);
    }
    if config.nominal_step_seconds <= 0 {
        return Err(TemporalFixtureError::InvalidTimeStep);
    }
    if config.max_delivery_delay_seconds < 0 || config.correction_delay_seconds < 0 {
        return Err(TemporalFixtureError::InvalidDelay);
    }
    if !(0.0..=1.0).contains(&config.missing_probability)
        || !(0.0..=1.0).contains(&config.correction_probability)
        || !config.missing_probability.is_finite()
        || !config.correction_probability.is_finite()
    {
        return Err(TemporalFixtureError::InvalidProbability);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_is_deterministic_for_a_fixed_seed_and_config() {
        let config = FixtureConfig::default();
        let first = TemporalFixture::generate(config.clone()).unwrap();
        let second = TemporalFixture::generate(config).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.manifest.digest_non_security,
            second.manifest.digest_non_security
        );
    }

    #[test]
    fn missing_observations_are_explicit_and_never_zero_filled() {
        let fixture = TemporalFixture::generate(FixtureConfig {
            seed: 7,
            missing_probability: 1.0,
            correction_probability: 0.0,
            ..FixtureConfig::default()
        })
        .unwrap();

        assert_eq!(fixture.missing_count(), fixture.events.len());
        assert!(
            fixture
                .events
                .iter()
                .all(|event| { event.kind == ObservationKind::Missing && event.value.is_none() })
        );
    }

    #[test]
    fn late_corrections_do_not_leak_into_an_earlier_decision_snapshot() {
        let fixture = TemporalFixture::generate(FixtureConfig {
            seed: 11,
            sensor_count: 4,
            time_steps: 3,
            missing_probability: 0.0,
            correction_probability: 1.0,
            max_delivery_delay_seconds: 0,
            correction_delay_seconds: 3_600,
            ..FixtureConfig::default()
        })
        .unwrap();

        let correction = fixture
            .events
            .iter()
            .find(|event| event.kind == ObservationKind::Correction)
            .unwrap();
        let parent_id = correction.supersedes_event_id.unwrap();
        let parent = fixture
            .events
            .iter()
            .find(|event| event.event_id == parent_id)
            .unwrap();

        let before_correction = fixture.snapshot_as_of(parent.recorded_at_micros);
        let before = before_correction
            .observations
            .iter()
            .find(|event| {
                event.sensor_id == parent.sensor_id
                    && event.channel == parent.channel
                    && event.valid_at_micros == parent.valid_at_micros
            })
            .unwrap();
        assert_eq!(before.event_id, parent.event_id);

        let after_correction = fixture.snapshot_as_of(correction.recorded_at_micros);
        let after = after_correction
            .observations
            .iter()
            .find(|event| {
                event.sensor_id == parent.sensor_id
                    && event.channel == parent.channel
                    && event.valid_at_micros == parent.valid_at_micros
            })
            .unwrap();
        assert_eq!(after.event_id, correction.event_id);
    }

    #[test]
    fn evaluator_truth_is_not_embedded_in_observation_snapshots() {
        let fixture = TemporalFixture::generate(FixtureConfig::default()).unwrap();
        let snapshot = fixture.snapshot_as_of(i64::MAX);
        assert_eq!(
            fixture.evaluator_truth.len(),
            fixture.manifest.truth_record_count
        );
        assert_eq!(
            snapshot.observations.len(),
            fixture.events.len() - fixture.correction_count()
        );
        assert!(snapshot.validate_cutoff());
    }

    #[test]
    fn generator_rejects_invalid_probability_and_clock_configuration() {
        assert!(matches!(
            TemporalFixture::generate(FixtureConfig {
                missing_probability: f64::NAN,
                ..FixtureConfig::default()
            }),
            Err(TemporalFixtureError::InvalidProbability)
        ));
        assert!(matches!(
            TemporalFixture::generate(FixtureConfig {
                nominal_step_seconds: 0,
                ..FixtureConfig::default()
            }),
            Err(TemporalFixtureError::InvalidTimeStep)
        ));
    }
}
