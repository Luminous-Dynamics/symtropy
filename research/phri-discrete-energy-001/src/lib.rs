#![forbid(unsafe_code)]

//! Sampled-data storage/port-work accounting reference.
//!
//! This crate checks a caller-supplied storage transition against a
//! caller-supplied conservative lower bound on physical-port work. It does
//! not derive that lower bound from sensors and does not authorize actuation.

use core::fmt;

/// Identifies a monotonic clock instance and boot epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyClock {
    pub clock_id: u64,
    pub boot_epoch: u64,
}

/// Immutable identity for one sampled physical interaction-port ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergyPortIdentity {
    pub sensor_id: u64,
    pub port_id: u64,
    pub spatial_frame_id: u64,
    pub clock: EnergyClock,
    pub calibration_digest: [u8; 32],
    pub measurement_profile_digest: [u8; 32],
}

/// Initial sample and storage state for a new observer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscreteEnergyBaseline {
    pub identity: EnergyPortIdentity,
    pub sample_sequence: u64,
    pub timestamp_ns: u64,
    pub storage_j: f64,
}

/// One interval from the previous accepted sample to this sample.
///
/// external_work_lower_j must already be a defensible lower bound on net
/// energy supplied through the declared physical port during this interval.
/// uncertainty_allowance_j is a separately justified metrology/numerical
/// allowance; it is not inferred or enlarged by this crate.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscreteEnergyStep {
    pub identity: EnergyPortIdentity,
    pub sample_sequence: u64,
    pub timestamp_ns: u64,
    pub storage_after_j: f64,
    pub external_work_lower_j: f64,
    pub uncertainty_allowance_j: f64,
}

/// Arithmetic and provenance receipt for one accepted interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscreteEnergyReceipt {
    pub sample_sequence: u64,
    pub interval_duration_ns: u64,
    pub storage_change_j: f64,
    pub external_work_lower_j: f64,
    pub uncertainty_allowance_j: f64,
    /// storage_change - external_work_lower - uncertainty_allowance.
    pub interval_residual_j: f64,
    pub cumulative_storage_change_j: f64,
    pub cumulative_external_work_lower_j: f64,
    pub cumulative_uncertainty_allowance_j: f64,
    /// Cumulative residual at this prefix.
    pub cumulative_residual_j: f64,
    /// Greatest positive cumulative residual seen since the baseline.
    pub maximum_prefix_deficit_j: f64,
    pub prefix_within_declared_energy_bound: bool,
}

/// Fail-closed identity, chronology and arithmetic errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiscreteEnergyError {
    ZeroSensorIdentity,
    ZeroPortIdentity,
    ZeroSpatialFrameIdentity,
    ZeroClockIdentity,
    MissingCalibrationDigest,
    MissingMeasurementProfileDigest,
    InvalidMaximumSampleGap,
    InvalidBaselineStorage,
    SensorIdentityMismatch,
    PortIdentityMismatch,
    SpatialFrameMismatch,
    ClockIdentityMismatch,
    CalibrationDigestMismatch,
    MeasurementProfileMismatch,
    NonConsecutiveSequence,
    SequenceOverflow,
    NonIncreasingTimestamp,
    MaximumSampleGapExceeded,
    InvalidStorage,
    NonFiniteWorkBound,
    InvalidUncertaintyAllowance,
    ArithmeticOverflow,
}

impl fmt::Display for DiscreteEnergyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DiscreteEnergyError {}

/// Single-port prefix observer for the sampled storage/port-work contract.
///
/// A rejected step does not mutate the observer. This class only verifies
/// arithmetic against the work lower bound supplied by the caller.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DiscreteEnergyObserver {
    identity: EnergyPortIdentity,
    maximum_sample_gap_ns: u64,
    last_sequence: u64,
    last_timestamp_ns: u64,
    last_storage_j: f64,
    cumulative_storage_change_j: f64,
    cumulative_external_work_lower_j: f64,
    cumulative_uncertainty_allowance_j: f64,
    cumulative_residual_j: f64,
    maximum_prefix_deficit_j: f64,
    accepted_steps: u64,
}

impl DiscreteEnergyObserver {
    /// Create an observer from a fully identified initial sample.
    pub fn new(
        baseline: DiscreteEnergyBaseline,
        maximum_sample_gap_ns: u64,
    ) -> Result<Self, DiscreteEnergyError> {
        validate_identity(baseline.identity)?;
        if maximum_sample_gap_ns == 0 {
            return Err(DiscreteEnergyError::InvalidMaximumSampleGap);
        }
        if !baseline.storage_j.is_finite() || baseline.storage_j < 0.0 {
            return Err(DiscreteEnergyError::InvalidBaselineStorage);
        }

        Ok(Self {
            identity: baseline.identity,
            maximum_sample_gap_ns,
            last_sequence: baseline.sample_sequence,
            last_timestamp_ns: baseline.timestamp_ns,
            last_storage_j: baseline.storage_j,
            cumulative_storage_change_j: 0.0,
            cumulative_external_work_lower_j: 0.0,
            cumulative_uncertainty_allowance_j: 0.0,
            cumulative_residual_j: 0.0,
            maximum_prefix_deficit_j: 0.0,
            accepted_steps: 0,
        })
    }

    /// Validate and account one consecutive sample interval.
    ///
    /// All derived values are computed and checked before any observer state
    /// is mutated. An error therefore leaves the previous accepted state
    /// unchanged.
    pub fn record(
        &mut self,
        step: DiscreteEnergyStep,
    ) -> Result<DiscreteEnergyReceipt, DiscreteEnergyError> {
        validate_identity(step.identity)?;
        validate_matching_identity(self.identity, step.identity)?;

        let expected_sequence = self
            .last_sequence
            .checked_add(1)
            .ok_or(DiscreteEnergyError::SequenceOverflow)?;
        if step.sample_sequence != expected_sequence {
            return Err(DiscreteEnergyError::NonConsecutiveSequence);
        }
        if step.timestamp_ns <= self.last_timestamp_ns {
            return Err(DiscreteEnergyError::NonIncreasingTimestamp);
        }
        let interval_duration_ns = step.timestamp_ns - self.last_timestamp_ns;
        if interval_duration_ns > self.maximum_sample_gap_ns {
            return Err(DiscreteEnergyError::MaximumSampleGapExceeded);
        }
        if !step.storage_after_j.is_finite() || step.storage_after_j < 0.0 {
            return Err(DiscreteEnergyError::InvalidStorage);
        }
        if !step.external_work_lower_j.is_finite() {
            return Err(DiscreteEnergyError::NonFiniteWorkBound);
        }
        if !step.uncertainty_allowance_j.is_finite()
            || step.uncertainty_allowance_j < 0.0
        {
            return Err(DiscreteEnergyError::InvalidUncertaintyAllowance);
        }

        let storage_change_j = step.storage_after_j - self.last_storage_j;
        let interval_residual_j =
            storage_change_j - step.external_work_lower_j - step.uncertainty_allowance_j;
        let cumulative_storage_change_j =
            self.cumulative_storage_change_j + storage_change_j;
        let cumulative_external_work_lower_j =
            self.cumulative_external_work_lower_j + step.external_work_lower_j;
        let cumulative_uncertainty_allowance_j =
            self.cumulative_uncertainty_allowance_j + step.uncertainty_allowance_j;
        let cumulative_residual_j = self.cumulative_residual_j + interval_residual_j;
        let maximum_prefix_deficit_j =
            self.maximum_prefix_deficit_j.max(cumulative_residual_j.max(0.0));

        if [
            storage_change_j,
            interval_residual_j,
            cumulative_storage_change_j,
            cumulative_external_work_lower_j,
            cumulative_uncertainty_allowance_j,
            cumulative_residual_j,
            maximum_prefix_deficit_j,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(DiscreteEnergyError::ArithmeticOverflow);
        }
        let accepted_steps = self
            .accepted_steps
            .checked_add(1)
            .ok_or(DiscreteEnergyError::SequenceOverflow)?;

        // State mutation occurs only after all validation and arithmetic pass.
        self.last_sequence = step.sample_sequence;
        self.last_timestamp_ns = step.timestamp_ns;
        self.last_storage_j = step.storage_after_j;
        self.cumulative_storage_change_j = cumulative_storage_change_j;
        self.cumulative_external_work_lower_j = cumulative_external_work_lower_j;
        self.cumulative_uncertainty_allowance_j = cumulative_uncertainty_allowance_j;
        self.cumulative_residual_j = cumulative_residual_j;
        self.maximum_prefix_deficit_j = maximum_prefix_deficit_j;
        self.accepted_steps = accepted_steps;

        Ok(DiscreteEnergyReceipt {
            sample_sequence: step.sample_sequence,
            interval_duration_ns,
            storage_change_j,
            external_work_lower_j: step.external_work_lower_j,
            uncertainty_allowance_j: step.uncertainty_allowance_j,
            interval_residual_j,
            cumulative_storage_change_j,
            cumulative_external_work_lower_j,
            cumulative_uncertainty_allowance_j,
            cumulative_residual_j,
            maximum_prefix_deficit_j,
            prefix_within_declared_energy_bound: maximum_prefix_deficit_j == 0.0,
        })
    }

    pub const fn accepted_steps(&self) -> u64 {
        self.accepted_steps
    }

    pub const fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub const fn last_storage_j(&self) -> f64 {
        self.last_storage_j
    }

    pub const fn cumulative_residual_j(&self) -> f64 {
        self.cumulative_residual_j
    }

    pub const fn maximum_prefix_deficit_j(&self) -> f64 {
        self.maximum_prefix_deficit_j
    }
}

fn validate_identity(identity: EnergyPortIdentity) -> Result<(), DiscreteEnergyError> {
    if identity.sensor_id == 0 {
        return Err(DiscreteEnergyError::ZeroSensorIdentity);
    }
    if identity.port_id == 0 {
        return Err(DiscreteEnergyError::ZeroPortIdentity);
    }
    if identity.spatial_frame_id == 0 {
        return Err(DiscreteEnergyError::ZeroSpatialFrameIdentity);
    }
    if identity.clock.clock_id == 0 || identity.clock.boot_epoch == 0 {
        return Err(DiscreteEnergyError::ZeroClockIdentity);
    }
    if identity.calibration_digest == [0; 32] {
        return Err(DiscreteEnergyError::MissingCalibrationDigest);
    }
    if identity.measurement_profile_digest == [0; 32] {
        return Err(DiscreteEnergyError::MissingMeasurementProfileDigest);
    }
    Ok(())
}

fn validate_matching_identity(
    expected: EnergyPortIdentity,
    actual: EnergyPortIdentity,
) -> Result<(), DiscreteEnergyError> {
    if actual.sensor_id != expected.sensor_id {
        return Err(DiscreteEnergyError::SensorIdentityMismatch);
    }
    if actual.port_id != expected.port_id {
        return Err(DiscreteEnergyError::PortIdentityMismatch);
    }
    if actual.spatial_frame_id != expected.spatial_frame_id {
        return Err(DiscreteEnergyError::SpatialFrameMismatch);
    }
    if actual.clock != expected.clock {
        return Err(DiscreteEnergyError::ClockIdentityMismatch);
    }
    if actual.calibration_digest != expected.calibration_digest {
        return Err(DiscreteEnergyError::CalibrationDigestMismatch);
    }
    if actual.measurement_profile_digest != expected.measurement_profile_digest {
        return Err(DiscreteEnergyError::MeasurementProfileMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALIBRATION: [u8; 32] = [0x11; 32];
    const PROFILE: [u8; 32] = [0x22; 32];

    fn identity() -> EnergyPortIdentity {
        EnergyPortIdentity {
            sensor_id: 7,
            port_id: 11,
            spatial_frame_id: 13,
            clock: EnergyClock {
                clock_id: 17,
                boot_epoch: 19,
            },
            calibration_digest: CALIBRATION,
            measurement_profile_digest: PROFILE,
        }
    }

    fn baseline() -> DiscreteEnergyBaseline {
        DiscreteEnergyBaseline {
            identity: identity(),
            sample_sequence: 100,
            timestamp_ns: 1_000,
            storage_j: 5.0,
        }
    }

    fn step(sequence: u64, timestamp_ns: u64, storage_after_j: f64,
             external_work_lower_j: f64) -> DiscreteEnergyStep {
        DiscreteEnergyStep {
            identity: identity(),
            sample_sequence: sequence,
            timestamp_ns,
            storage_after_j,
            external_work_lower_j,
            uncertainty_allowance_j: 0.0,
        }
    }

    #[test]
    fn exact_known_answer_interval_has_zero_energy_residual() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        let receipt = observer.record(step(101, 1_010, 6.0, 1.0)).unwrap();
        assert_eq!(receipt.storage_change_j, 1.0);
        assert_eq!(receipt.interval_residual_j, 0.0);
        assert_eq!(receipt.cumulative_residual_j, 0.0);
        assert_eq!(receipt.maximum_prefix_deficit_j, 0.0);
        assert!(receipt.prefix_within_declared_energy_bound);
    }

    #[test]
    fn uncertainty_allowance_is_applied_once_and_recorded() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        let receipt = observer.record(DiscreteEnergyStep {
            uncertainty_allowance_j: 0.1,
            ..step(101, 1_010, 6.05, 1.0)
        }).unwrap();
        assert!((receipt.interval_residual_j + 0.05).abs() < 1.0e-12);
        assert_eq!(receipt.cumulative_uncertainty_allowance_j, 0.1);
    }

    #[test]
    fn positive_prefix_deficit_is_retained_even_after_later_surplus() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        let first = observer.record(step(101, 1_010, 7.0, 1.0)).unwrap();
        assert_eq!(first.cumulative_residual_j, 1.0);
        assert_eq!(first.maximum_prefix_deficit_j, 1.0);

        let second = observer.record(step(102, 1_020, 7.0, 2.0)).unwrap();
        assert_eq!(second.cumulative_residual_j, -1.0);
        assert_eq!(second.maximum_prefix_deficit_j, 1.0);
        assert!(!second.prefix_within_declared_energy_bound);
        assert_eq!(observer.maximum_prefix_deficit_j(), 1.0);
    }

    #[test]
    fn negative_external_work_is_preserved_with_its_sign() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        let receipt = observer.record(step(101, 1_010, 5.0, -0.5)).unwrap();
        assert_eq!(receipt.interval_residual_j, 0.5);
        assert_eq!(receipt.cumulative_external_work_lower_j, -0.5);
        assert_eq!(receipt.maximum_prefix_deficit_j, 0.5);
    }

    #[test]
    fn sequence_gap_is_rejected_without_mutating_observer() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        assert_eq!(
            observer.record(step(102, 1_020, 6.0, 1.0)),
            Err(DiscreteEnergyError::NonConsecutiveSequence)
        );
        assert_eq!(observer.accepted_steps(), 0);
        assert_eq!(observer.last_sequence(), 100);
        assert_eq!(observer.last_storage_j(), 5.0);

        assert!(observer.record(step(101, 1_010, 6.0, 1.0)).is_ok());
    }

    #[test]
    fn clock_epoch_change_is_rejected() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        let mut sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.clock.boot_epoch += 1;
        assert_eq!(
            observer.record(sample),
            Err(DiscreteEnergyError::ClockIdentityMismatch)
        );
    }

    #[test]
    fn sensor_port_frame_calibration_and_profile_mismatches_are_rejected() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();

        let mut sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.sensor_id += 1;
        assert_eq!(observer.record(sample), Err(DiscreteEnergyError::SensorIdentityMismatch));

        sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.port_id += 1;
        assert_eq!(observer.record(sample), Err(DiscreteEnergyError::PortIdentityMismatch));

        sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.spatial_frame_id += 1;
        assert_eq!(observer.record(sample), Err(DiscreteEnergyError::SpatialFrameMismatch));

        sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.calibration_digest[0] ^= 0x01;
        assert_eq!(observer.record(sample), Err(DiscreteEnergyError::CalibrationDigestMismatch));

        sample = step(101, 1_010, 6.0, 1.0);
        sample.identity.measurement_profile_digest[0] ^= 0x01;
        assert_eq!(observer.record(sample), Err(DiscreteEnergyError::MeasurementProfileMismatch));

        assert_eq!(observer.accepted_steps(), 0);
    }

    #[test]
    fn nonincreasing_timestamp_and_excessive_gap_are_rejected() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 10).unwrap();
        assert_eq!(
            observer.record(step(101, 1_000, 6.0, 1.0)),
            Err(DiscreteEnergyError::NonIncreasingTimestamp)
        );
        assert_eq!(
            observer.record(step(101, 1_011, 6.0, 1.0)),
            Err(DiscreteEnergyError::MaximumSampleGapExceeded)
        );
        assert_eq!(observer.accepted_steps(), 0);
    }

    #[test]
    fn invalid_storage_work_and_allowance_are_rejected() {
        let mut observer = DiscreteEnergyObserver::new(baseline(), 100).unwrap();
        assert_eq!(
            observer.record(step(101, 1_010, -1.0, 1.0)),
            Err(DiscreteEnergyError::InvalidStorage)
        );
        assert_eq!(
            observer.record(step(101, 1_010, 6.0, f64::NAN)),
            Err(DiscreteEnergyError::NonFiniteWorkBound)
        );
        let mut invalid_allowance = step(101, 1_010, 6.0, 1.0);
        invalid_allowance.uncertainty_allowance_j = -0.01;
        assert_eq!(
            observer.record(invalid_allowance),
            Err(DiscreteEnergyError::InvalidUncertaintyAllowance)
        );
        assert_eq!(observer.accepted_steps(), 0);
    }

    #[test]
    fn arithmetic_overflow_is_rejected_without_mutation() {
        let mut initial = baseline();
        initial.storage_j = 0.0;
        let mut observer = DiscreteEnergyObserver::new(initial, 100).unwrap();
        let result = observer.record(step(101, 1_010, f64::MAX, -f64::MAX));
        assert_eq!(result, Err(DiscreteEnergyError::ArithmeticOverflow));
        assert_eq!(observer.accepted_steps(), 0);
        assert_eq!(observer.last_storage_j(), 0.0);
    }

    #[test]
    fn missing_identity_and_invalid_baseline_are_rejected() {
        let mut initial = baseline();
        initial.identity.measurement_profile_digest = [0; 32];
        assert_eq!(
            DiscreteEnergyObserver::new(initial, 100),
            Err(DiscreteEnergyError::MissingMeasurementProfileDigest)
        );

        initial = baseline();
        initial.storage_j = f64::NAN;
        assert_eq!(
            DiscreteEnergyObserver::new(initial, 100),
            Err(DiscreteEnergyError::InvalidBaselineStorage)
        );
    }
}
