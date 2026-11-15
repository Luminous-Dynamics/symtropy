//! Aviation-specific temporal evidence admission for heterogeneous sensors.
//!
//! This crate distinguishes bounded temporal relationships from software-cycle association
//! and file-order-only evidence. It does not infer a clock from a filename, implement a
//! clock service, authenticate timing attestations, or perform sensor fusion.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Native sensor modality in a multi-sensor observation cycle.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SensorModality {
    /// Continuous-wave radar.
    ContinuousWaveRadar,
    /// Frequency-modulated continuous-wave radar.
    FmcwRadar,
    /// Radio-frequency receiver.
    RfReceiver,
    /// ADS-B or other cooperative receiver feed.
    CooperativeReceiver,
    /// Electro-optical / infrared sensor.
    ElectroOpticalInfrared,
    /// Other modality defined by an owner-controlled schema.
    Other,
}

/// Whether source evidence contains raw samples or a transformed representation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DataRepresentation {
    /// Raw acquisition samples or raw bytes from the declared sensor interface.
    Raw,
    /// Processed representation with an exact preprocessing identity.
    Processed,
}

/// Temporal relation class declared by the source/evidence owner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalEvidenceClass {
    /// Independent qualified hardware timing reference supplies bounded time per sensor.
    IndependentHardwareBounded,
    /// Sensors are related through one qualified shared clock profile with bounded uncertainty.
    SharedClockBounded,
    /// A shared software acquisition cycle establishes a common batch, but not bounded simultaneity.
    SoftwareCycleAssociated,
    /// Only file order or unknown timing is available.
    OrderOnlyOrUnknown,
}

/// Closed interval for a timestamp on one explicitly named time scale.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimeInterval {
    /// Earliest possible timestamp, in integer nanoseconds on the declared scale.
    pub lower_ns: i128,
    /// Latest possible timestamp, in integer nanoseconds on the declared scale.
    pub upper_ns: i128,
}

impl TimeInterval {
    fn is_valid(self) -> bool {
        self.lower_ns <= self.upper_ns
    }

    fn worst_case_separation(self, other: Self) -> u128 {
        let first = self.lower_ns.abs_diff(other.upper_ns);
        let second = self.upper_ns.abs_diff(other.lower_ns);
        first.max(second)
    }
}

/// Bounded capture interval for a single sensor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureWindow {
    /// Bounded capture-start timestamp.
    pub start: TimeInterval,
    /// Bounded capture-end timestamp.
    pub end: TimeInterval,
}

impl CaptureWindow {
    fn is_valid(self) -> bool {
        self.start.is_valid()
            && self.end.is_valid()
            && self.start.upper_ns <= self.end.lower_ns
    }
}

/// One sensor's contribution to a common acquisition cycle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SensorCapture {
    /// Stable physical/logical sensor identity within the acquisition profile.
    pub sensor_id: String,
    /// Native sensor modality.
    pub modality: SensorModality,
    /// Stable acquisition-cycle ID; a join key, not a timestamp.
    pub acquisition_cycle_id: String,
    /// Per-source file/record sequence; ordering only, not time.
    pub file_sequence: u64,
    /// Raw versus processed representation.
    pub representation: DataRepresentation,
    /// Preprocessing pipeline digest for processed data; absent for raw data.
    pub preprocessing_sha256: Option<String>,
    /// Calibration/configuration profile digest when the source provides one.
    pub calibration_profile_sha256: Option<String>,
    /// Native or normalized time-scale identity when bounded timestamps are present.
    pub time_scale_id: Option<String>,
    /// Sensor clock-domain identity when supplied.
    pub clock_domain_id: Option<String>,
    /// Qualified independent reference identity for hardware-bounded timing.
    pub hardware_time_reference_sha256: Option<String>,
    /// Capture window on the declared common time scale; absent when not measured.
    pub capture_window: Option<CaptureWindow>,
}

/// Frozen temporal-alignment policy for a sensor dataset or evaluation profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalAlignmentPolicy {
    /// Declared temporal evidence class.
    pub evidence_class: TemporalEvidenceClass,
    /// Exact policy/profile manifest digest.
    pub policy_sha256: String,
    /// Common time-scale identity required for bounded timing.
    pub time_scale_id: Option<String>,
    /// Shared clock-domain identity for SHARED_CLOCK_BOUNDED.
    pub shared_clock_domain_id: Option<String>,
    /// Exact shared-clock profile digest.
    pub shared_clock_profile_sha256: Option<String>,
    /// Exact independent hardware timing reference digest.
    pub independent_hardware_reference_sha256: Option<String>,
    /// Maximum accepted conservative inter-sensor capture-start/end skew, nanoseconds.
    pub maximum_pairwise_skew_ns: u64,
    /// Maximum permitted clock drift during the experiment, parts per billion.
    pub maximum_drift_ppb: u64,
}

/// Temporal association/fusion status for one acquisition cycle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalDisposition {
    /// A bounded cross-modal time relation is qualified within the frozen policy.
    BoundedFusionTimeEligible,
    /// Records share a software acquisition cycle, but sub-cycle timing is not established.
    PerCycleAssociationOnly,
    /// No defensible cross-modal temporal relation; preserve modality-specific evidence only.
    ModalitySpecificOnly,
    /// Metadata conflicts with the declared temporal evidence class or its bound.
    Indeterminate,
}

/// Reasons that cross-modal timing cannot be promoted.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum TemporalGap {
    /// Two or more records are required for the declared fusion claim.
    InsufficientSensors,
    /// Sensor IDs are duplicated in one acquisition cycle.
    DuplicateSensorId,
    /// Acquisition-cycle identities do not match.
    AcquisitionCycleMismatch,
    /// The declared common time scale is absent or inconsistent.
    TimeScaleMissingOrMismatch,
    /// A capture window is absent for a bounded-time claim.
    CaptureWindowMissing,
    /// A capture interval is malformed.
    InvalidCaptureWindow,
    /// The independent hardware reference is absent or inconsistent.
    HardwareReferenceMissingOrMismatch,
    /// The shared clock domain/profile is absent or inconsistent.
    SharedClockMissingOrMismatch,
    /// Conservative pairwise skew exceeds the policy bound.
    PairwiseSkewExceedsBound,
    /// Processed record lacks exact preprocessing identity.
    PreprocessingIdentityMissing,
    /// Capture source has no calibration/configuration profile.
    CalibrationProfileMissing,
    /// A raw/processed provenance or acquisition profile mismatch exists.
    RepresentationMetadataInvalid,
    /// The source has only ordering, not a defensible capture-time relation.
    TimeRelationUnknown,
}

/// Result for one acquisition cycle, retaining modality-specific assessment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalAssessment {
    /// Cycle being evaluated.
    pub acquisition_cycle_id: String,
    /// Temporal disposition.
    pub disposition: TemporalDisposition,
    /// Number of distinct sensor modalities represented.
    pub modality_count: usize,
    /// Whether each sensor has a syntactically valid calibration/configuration profile.
    /// This is metadata completeness only, not proof of physical calibration.
    pub measurement_metadata_complete: bool,
    /// Conservative maximum pairwise start/end skew, if bounded timing was evaluated.
    pub maximum_pairwise_skew_ns: Option<u128>,
    /// Sorted unique sensor IDs in the assessment.
    pub sensor_ids: Vec<String>,
    /// Explicit evidence gaps.
    pub gaps: Vec<TemporalGap>,
    /// Frozen policy digest.
    pub policy_sha256: String,
}

/// Typed input failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalError {
    /// Policy identity or required bound metadata is malformed.
    InvalidPolicy,
    /// Sensor identity, cycle identity or source metadata is malformed.
    InvalidSensorCapture,
    /// Duplicate sensor IDs make the cycle ambiguous.
    DuplicateSensorId,
}

/// Digest syntax only; this crate does not recompute source bytes.
fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_policy(policy: &TemporalAlignmentPolicy) -> Result<(), TemporalError> {
    if !is_sha256(&policy.policy_sha256) || policy.maximum_pairwise_skew_ns == 0 {
        return Err(TemporalError::InvalidPolicy);
    }
    match policy.evidence_class {
        TemporalEvidenceClass::IndependentHardwareBounded => {
            if !is_sha256(
                policy
                    .independent_hardware_reference_sha256
                    .as_deref()
                    .unwrap_or_default(),
            ) || policy.time_scale_id.as_deref().unwrap_or_default().trim().is_empty()
            {
                return Err(TemporalError::InvalidPolicy);
            }
        }
        TemporalEvidenceClass::SharedClockBounded => {
            if policy.shared_clock_domain_id.as_deref().unwrap_or_default().trim().is_empty()
                || !is_sha256(
                    policy
                        .shared_clock_profile_sha256
                        .as_deref()
                        .unwrap_or_default(),
                )
                || policy.time_scale_id.as_deref().unwrap_or_default().trim().is_empty()
            {
                return Err(TemporalError::InvalidPolicy);
            }
        }
        TemporalEvidenceClass::SoftwareCycleAssociated
        | TemporalEvidenceClass::OrderOnlyOrUnknown => {}
    }
    Ok(())
}

fn validate_sensor(sensor: &SensorCapture) -> Result<(), TemporalError> {
    if sensor.sensor_id.trim().is_empty() || sensor.acquisition_cycle_id.trim().is_empty() {
        return Err(TemporalError::InvalidSensorCapture);
    }
    match sensor.representation {
        DataRepresentation::Raw if sensor.preprocessing_sha256.is_some() => {
            Err(TemporalError::InvalidSensorCapture)
        }
        DataRepresentation::Processed
            if !sensor
                .preprocessing_sha256
                .as_deref()
                .is_some_and(is_sha256) =>
        {
            Err(TemporalError::InvalidSensorCapture)
        }
        _ => Ok(()),
    }
}

/// Evaluate the temporal relationship of one multi-sensor acquisition cycle.
///
/// File sequence and acquisition-cycle identity are never interpreted as timestamps.
/// A positive bounded status requires a qualified time scale, clock/reference identity,
/// capture windows, valid preprocessing provenance and a conservative skew within policy.
pub fn evaluate_cycle(
    sensors: &[SensorCapture],
    policy: &TemporalAlignmentPolicy,
) -> Result<TemporalAssessment, TemporalError> {
    validate_policy(policy)?;
    if sensors.is_empty() {
        return Err(TemporalError::InvalidSensorCapture);
    }
    for sensor in sensors {
        validate_sensor(sensor)?;
    }

    let mut sensor_ids = BTreeSet::new();
    for sensor in sensors {
        if !sensor_ids.insert(sensor.sensor_id.as_str()) {
            return Err(TemporalError::DuplicateSensorId);
        }
    }
    let sensor_ids: Vec<String> = sensor_ids.into_iter().map(str::to_owned).collect();
    let mut gaps = BTreeSet::new();
    let cycle_id = sensors[0].acquisition_cycle_id.clone();
    if sensors
        .iter()
        .any(|sensor| sensor.acquisition_cycle_id != cycle_id)
    {
        gaps.insert(TemporalGap::AcquisitionCycleMismatch);
    }

    let modality_count = sensors
        .iter()
        .map(|sensor| sensor.modality)
        .collect::<BTreeSet<_>>()
        .len();

    for sensor in sensors {
        if sensor.calibration_profile_sha256.as_deref().is_none_or(|value| !is_sha256(value)) {
            gaps.insert(TemporalGap::CalibrationProfileMissing);
        }
    }

    let mut maximum_pairwise_skew_ns = None;
    let disposition = match policy.evidence_class {
        TemporalEvidenceClass::OrderOnlyOrUnknown => {
            gaps.insert(TemporalGap::TimeRelationUnknown);
            TemporalDisposition::ModalitySpecificOnly
        }
        TemporalEvidenceClass::SoftwareCycleAssociated => {
            if sensors
                .iter()
                .any(|sensor| sensor.acquisition_cycle_id != cycle_id)
            {
                TemporalDisposition::Indeterminate
            } else {
                TemporalDisposition::PerCycleAssociationOnly
            }
        }
        TemporalEvidenceClass::IndependentHardwareBounded
        | TemporalEvidenceClass::SharedClockBounded => {
            let expected_time_scale = policy.time_scale_id.as_deref().unwrap_or_default();
            for sensor in sensors {
                if sensor.time_scale_id.as_deref() != Some(expected_time_scale) {
                    gaps.insert(TemporalGap::TimeScaleMissingOrMismatch);
                }
                if sensor.capture_window.is_none() {
                    gaps.insert(TemporalGap::CaptureWindowMissing);
                }
                if let Some(window) = sensor.capture_window
                    && !window.is_valid()
                {
                    gaps.insert(TemporalGap::InvalidCaptureWindow);
                }
                if sensor.representation == DataRepresentation::Processed
                    && sensor
                        .preprocessing_sha256
                        .as_deref()
                        .is_none_or(|value| !is_sha256(value))
                {
                    gaps.insert(TemporalGap::PreprocessingIdentityMissing);
                }
            }

            match policy.evidence_class {
                TemporalEvidenceClass::IndependentHardwareBounded => {
                    let expected = policy
                        .independent_hardware_reference_sha256
                        .as_deref()
                        .unwrap_or_default();
                    if sensors.iter().any(|sensor| {
                        sensor.hardware_time_reference_sha256.as_deref() != Some(expected)
                    }) {
                        gaps.insert(TemporalGap::HardwareReferenceMissingOrMismatch);
                    }
                }
                TemporalEvidenceClass::SharedClockBounded => {
                    let expected_clock = policy
                        .shared_clock_domain_id
                        .as_deref()
                        .unwrap_or_default();
                    if sensors.iter().any(|sensor| {
                        sensor.clock_domain_id.as_deref() != Some(expected_clock)
                    }) {
                        gaps.insert(TemporalGap::SharedClockMissingOrMismatch);
                    }
                }
                TemporalEvidenceClass::SoftwareCycleAssociated
                | TemporalEvidenceClass::OrderOnlyOrUnknown => {}
            }

            let windows: Vec<CaptureWindow> = sensors
                .iter()
                .filter_map(|sensor| sensor.capture_window)
                .collect();
            if windows.len() == sensors.len() && windows.iter().all(|window| window.is_valid()) {
                let mut maximum = 0_u128;
                for left in 0..windows.len() {
                    for right in (left + 1)..windows.len() {
                        maximum = maximum.max(
                            windows[left]
                                .start
                                .worst_case_separation(windows[right].start),
                        );
                        maximum = maximum.max(
                            windows[left]
                                .end
                                .worst_case_separation(windows[right].end),
                        );
                    }
                }
                maximum_pairwise_skew_ns = Some(maximum);
                if maximum > u128::from(policy.maximum_pairwise_skew_ns) {
                    gaps.insert(TemporalGap::PairwiseSkewExceedsBound);
                }
            }

            if gaps.is_empty() {
                TemporalDisposition::BoundedFusionTimeEligible
            } else {
                TemporalDisposition::Indeterminate
            }
        }
    };

    Ok(TemporalAssessment {
        acquisition_cycle_id: cycle_id,
        disposition,
        modality_count,
        measurement_metadata_complete: sensors.iter().all(|sensor| {
            sensor.calibration_profile_sha256.as_deref().is_some_and(is_sha256)
        }),
        maximum_pairwise_skew_ns,
        sensor_ids,
        gaps: gaps.into_iter().collect(),
        policy_sha256: policy.policy_sha256.clone(),
    })
}
