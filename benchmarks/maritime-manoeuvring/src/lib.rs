#![forbid(unsafe_code)]

//! Evidence-bounded maritime manoeuvring contracts and independent trajectory metrics.
//!
//! This crate deliberately does not simulate vessel hydrodynamics. It validates
//! declared evidence envelopes and recomputes a small set of geometric/time-domain
//! metrics from externally produced trajectories.

use std::collections::BTreeSet;
use std::f64::consts::{PI, TAU};

const EPSILON: f64 = 1.0e-9;

/// A required evidence family for a manoeuvring case.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EvidenceRole {
    VesselConfiguration,
    ManoeuvringModel,
    ModelValidation,
    ChannelGeometry,
    Bathymetry,
    WaterLevel,
    Current,
    WindAndWaves,
    TugCapability,
}

/// Admission status assigned by a source/profile review process.
///
/// Reviewed means the reference passed that declared intake review. It does
/// not authenticate a publisher, prove a licence, or establish physical validity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceState {
    Reviewed,
    Unknown,
    Stale,
    Unsupported,
}

/// Context-of-use profile. Each profile fixes its required evidence roles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextOfUse {
    DeepWaterStandard,
    ShallowWater,
    RestrictedWater,
    EnvironmentalForcing,
    TugAssisted,
}

impl ContextOfUse {
    /// Evidence roles required before this case can be admitted for comparison.
    #[must_use]
    pub const fn required_roles(self) -> &'static [EvidenceRole] {
        use EvidenceRole as R;
        match self {
            Self::DeepWaterStandard => &[
                R::VesselConfiguration,
                R::ManoeuvringModel,
                R::ModelValidation,
            ],
            Self::ShallowWater => &[
                R::VesselConfiguration,
                R::ManoeuvringModel,
                R::ModelValidation,
                R::ChannelGeometry,
                R::Bathymetry,
                R::WaterLevel,
                R::Current,
            ],
            Self::RestrictedWater => &[
                R::VesselConfiguration,
                R::ManoeuvringModel,
                R::ModelValidation,
                R::ChannelGeometry,
                R::Bathymetry,
                R::WaterLevel,
                R::Current,
            ],
            Self::EnvironmentalForcing => &[
                R::VesselConfiguration,
                R::ManoeuvringModel,
                R::ModelValidation,
                R::ChannelGeometry,
                R::Bathymetry,
                R::WaterLevel,
                R::Current,
                R::WindAndWaves,
            ],
            Self::TugAssisted => &[
                R::VesselConfiguration,
                R::ManoeuvringModel,
                R::ModelValidation,
                R::ChannelGeometry,
                R::Bathymetry,
                R::WaterLevel,
                R::Current,
                R::WindAndWaves,
                R::TugCapability,
            ],
        }
    }
}

/// A reference to one immutable source/model/configuration artifact.
#[derive(Clone, Debug, PartialEq)]
pub struct EvidenceRef {
    pub id: String,
    /// Exact frozen case to which this admission record is bound.
    pub case_id: String,
    pub role: EvidenceRole,
    pub source_id: String,
    pub source_revision: String,
    /// SHA-256 syntax is checked, but this crate does not fetch or hash raw bytes.
    pub sha256: String,
    /// Time the evidence was published/made available to the decision-maker.
    pub published_at_s: f64,
    /// Inclusive start of the evidence's valid-time interval.
    pub valid_from_s: f64,
    /// Inclusive end of the evidence's valid-time interval.
    pub valid_until_s: f64,
    pub state: EvidenceState,
}

/// Frozen case manifest. All time values share the declared scenario time basis.
#[derive(Clone, Debug, PartialEq)]
pub struct ManoeuvringManifest {
    pub case_id: String,
    pub context: ContextOfUse,
    /// Latest information available to the decision process.
    pub decision_cutoff_s: f64,
    /// Planned operation interval, which may be in the future.
    pub operation_start_s: f64,
    pub operation_end_s: f64,
    pub evidence: Vec<EvidenceRef>,
}

/// Disposition of the manifest for a comparison workflow.
///
/// ReadyForComparison is deliberately not a physical qualification or
/// navigation-safety claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestDisposition {
    ReadyForComparison,
    NeedsEvidence,
    Unsupported,
}

/// Deterministic explanation of manifest admission.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestReport {
    pub disposition: ManifestDisposition,
    pub missing_roles: Vec<EvidenceRole>,
    pub unresolved_roles: Vec<EvidenceRole>,
    pub unsupported_roles: Vec<EvidenceRole>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ManifestError {
    EmptyCaseId,
    NonFiniteTime,
    InvalidOperationWindow,
    DecisionAfterOperationStart,
    EmptyEvidenceField,
    EvidenceCaseMismatch(String),
    InvalidDigest(String),
    InvalidEvidenceInterval(String),
    FutureEvidence(String),
    DuplicateEvidenceId(String),
    DuplicateEvidenceRole(EvidenceRole),
}

impl ManoeuvringManifest {
    /// Validate structure and report whether declared evidence covers the case.
    ///
    /// The decision cutoff prevents use of evidence published after the
    /// decision. Valid-time coverage is checked against the full operation
    /// interval, not merely against the publication timestamp.
    pub fn evaluate(&self) -> Result<ManifestReport, ManifestError> {
        if self.case_id.trim().is_empty() {
            return Err(ManifestError::EmptyCaseId);
        }
        if !self.decision_cutoff_s.is_finite()
            || !self.operation_start_s.is_finite()
            || !self.operation_end_s.is_finite()
        {
            return Err(ManifestError::NonFiniteTime);
        }
        if self.operation_start_s >= self.operation_end_s {
            return Err(ManifestError::InvalidOperationWindow);
        }
        if self.decision_cutoff_s > self.operation_start_s {
            return Err(ManifestError::DecisionAfterOperationStart);
        }

        let mut ids = BTreeSet::new();
        let mut roles = BTreeSet::new();
        for evidence in &self.evidence {
            if evidence.case_id != self.case_id {
                return Err(ManifestError::EvidenceCaseMismatch(evidence.id.clone()));
            }
            if evidence.id.trim().is_empty()
                || evidence.source_id.trim().is_empty()
                || evidence.source_revision.trim().is_empty()
            {
                return Err(ManifestError::EmptyEvidenceField);
            }
            if !is_sha256(&evidence.sha256) {
                return Err(ManifestError::InvalidDigest(evidence.id.clone()));
            }
            if !evidence.published_at_s.is_finite()
                || !evidence.valid_from_s.is_finite()
                || !evidence.valid_until_s.is_finite()
                || evidence.valid_from_s > evidence.valid_until_s
            {
                return Err(ManifestError::InvalidEvidenceInterval(evidence.id.clone()));
            }
            if evidence.published_at_s > self.decision_cutoff_s {
                return Err(ManifestError::FutureEvidence(evidence.id.clone()));
            }
            if !ids.insert(evidence.id.as_str()) {
                return Err(ManifestError::DuplicateEvidenceId(evidence.id.clone()));
            }
            if !roles.insert(evidence.role) {
                return Err(ManifestError::DuplicateEvidenceRole(evidence.role));
            }
        }

        let mut missing_roles = Vec::new();
        let mut unresolved_roles = Vec::new();
        let mut unsupported_roles = Vec::new();

        for required in self.context.required_roles() {
            let Some(evidence) = self.evidence.iter().find(|item| item.role == *required) else {
                missing_roles.push(*required);
                continue;
            };

            match evidence.state {
                EvidenceState::Unsupported => unsupported_roles.push(*required),
                EvidenceState::Unknown | EvidenceState::Stale => unresolved_roles.push(*required),
                EvidenceState::Reviewed => {
                    if evidence.valid_from_s > self.operation_start_s
                        || evidence.valid_until_s < self.operation_end_s
                    {
                        unresolved_roles.push(*required);
                    }
                }
            }
        }

        let disposition = if !unsupported_roles.is_empty() {
            ManifestDisposition::Unsupported
        } else if !missing_roles.is_empty() || !unresolved_roles.is_empty() {
            ManifestDisposition::NeedsEvidence
        } else {
            ManifestDisposition::ReadyForComparison
        };

        Ok(ManifestReport {
            disposition,
            missing_roles,
            unresolved_roles,
            unsupported_roles,
        })
    }
}

/// Validate digest syntax (exactly 64 hexadecimal characters).
///
/// This does not fetch, hash, authenticate or establish rights for raw bytes.
#[must_use]
pub fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// One local planar sample from a specialist manoeuvring simulator.
///
/// Coordinates are metres in a declared local Cartesian frame: +x east-like,
/// +y north-like. Heading is radians counter-clockwise from +x. Time is seconds.
/// Coordinate-frame and unit identity must be bound by the calling run manifest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrajectorySample {
    pub time_s: f64,
    pub x_m: f64,
    pub y_m: f64,
    pub heading_rad: f64,
    pub surge_mps: f64,
    pub sway_mps: f64,
    pub yaw_rate_rad_s: f64,
}

/// A normalized trajectory from an external simulator or measurement campaign.
#[derive(Clone, Debug, PartialEq)]
pub struct Trajectory {
    pub vessel_id: String,
    pub vessel_length_m: f64,
    pub samples: Vec<TrajectorySample>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TrajectoryError {
    EmptyVesselId,
    InvalidVesselLength,
    TooFewSamples,
    NonFiniteSample(usize),
    NonIncreasingTime(usize),
    AmbiguousHeadingIncrement(usize),
    NoTurningDirection,
    HeadingTargetNotReached(u16),
    InvalidCommandIndex,
    InvalidSpeedThreshold,
    VesselAlreadyBelowStopThreshold,
    StopThresholdNotReached,
}

/// Metrics from the standard 90°/180° turning-circle geometry.
///
/// These are trajectory descriptors, not an IMO compliance verdict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurningCircleMetrics {
    pub advance_at_90_m: f64,
    pub transfer_at_90_m: f64,
    pub tactical_diameter_at_180_m: f64,
    pub time_to_90_s: f64,
    pub time_to_180_s: f64,
}

/// Track-relative displacement when speed first crosses the supplied stop threshold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoppingMetrics {
    pub time_from_command_s: f64,
    pub along_track_displacement_m: f64,
    pub cross_track_displacement_m: f64,
    pub speed_at_threshold_mps: f64,
}

pub fn validate_trajectory(trajectory: &Trajectory) -> Result<(), TrajectoryError> {
    if trajectory.vessel_id.trim().is_empty() {
        return Err(TrajectoryError::EmptyVesselId);
    }
    if !trajectory.vessel_length_m.is_finite() || trajectory.vessel_length_m <= 0.0 {
        return Err(TrajectoryError::InvalidVesselLength);
    }
    if trajectory.samples.len() < 2 {
        return Err(TrajectoryError::TooFewSamples);
    }

    for (index, sample) in trajectory.samples.iter().enumerate() {
        if !sample.time_s.is_finite()
            || !sample.x_m.is_finite()
            || !sample.y_m.is_finite()
            || !sample.heading_rad.is_finite()
            || !sample.surge_mps.is_finite()
            || !sample.sway_mps.is_finite()
            || !sample.yaw_rate_rad_s.is_finite()
        {
            return Err(TrajectoryError::NonFiniteSample(index));
        }
        if index > 0 && sample.time_s <= trajectory.samples[index - 1].time_s {
            return Err(TrajectoryError::NonIncreasingTime(index));
        }
    }

    let _ = unwrap_headings(&trajectory.samples)?;
    Ok(())
}

/// Recompute advance, transfer and tactical diameter from a sampled trajectory.
///
/// The first non-zero heading increment determines turn direction. The function
/// interpolates the first 90° and 180° heading crossings. Sampling must be
/// frequent enough that no inter-sample heading change is ambiguous.
pub fn turning_circle_metrics(
    trajectory: &Trajectory,
) -> Result<TurningCircleMetrics, TrajectoryError> {
    validate_trajectory(trajectory)?;
    let headings = unwrap_headings(&trajectory.samples)?;
    let initial_heading = headings[0];

    let direction = headings
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .find(|delta| delta.abs() > EPSILON)
        .map(f64::signum)
        .ok_or(TrajectoryError::NoTurningDirection)?;

    let at_90 = heading_crossing(trajectory, &headings, direction, 90.0)
        .ok_or(TrajectoryError::HeadingTargetNotReached(90))?;
    let at_180 = heading_crossing(trajectory, &headings, direction, 180.0)
        .ok_or(TrajectoryError::HeadingTargetNotReached(180))?;

    let (sin_h, cos_h) = initial_heading.sin_cos();
    let dx90 = at_90.x_m - trajectory.samples[0].x_m;
    let dy90 = at_90.y_m - trajectory.samples[0].y_m;
    let dx180 = at_180.x_m - trajectory.samples[0].x_m;
    let dy180 = at_180.y_m - trajectory.samples[0].y_m;

    let advance_at_90_m = dx90 * cos_h + dy90 * sin_h;
    let transfer_at_90_m = -dx90 * sin_h + dy90 * cos_h;
    let tactical_diameter_at_180_m = (-dx180 * sin_h + dy180 * cos_h).abs();

    Ok(TurningCircleMetrics {
        advance_at_90_m,
        transfer_at_90_m,
        tactical_diameter_at_180_m,
        time_to_90_s: at_90.time_s - trajectory.samples[0].time_s,
        time_to_180_s: at_180.time_s - trajectory.samples[0].time_s,
    })
}

/// Recompute a stopping track descriptor from an explicit command sample.
///
/// speed_threshold_mps is an explicit analysis parameter, not an IMO-prescribed
/// universal stop definition. The first crossing is linearly interpolated between
/// adjacent samples. The returned displacement is relative to the command point.
pub fn stopping_metrics(
    trajectory: &Trajectory,
    command_sample_index: usize,
    speed_threshold_mps: f64,
) -> Result<StoppingMetrics, TrajectoryError> {
    validate_trajectory(trajectory)?;
    if !speed_threshold_mps.is_finite() || speed_threshold_mps <= 0.0 {
        return Err(TrajectoryError::InvalidSpeedThreshold);
    }
    let Some(command) = trajectory.samples.get(command_sample_index).copied() else {
        return Err(TrajectoryError::InvalidCommandIndex);
    };

    let command_speed = speed(command);
    if command_speed <= speed_threshold_mps {
        return Err(TrajectoryError::VesselAlreadyBelowStopThreshold);
    }

    let mut crossing = None;
    for index in (command_sample_index + 1)..trajectory.samples.len() {
        let previous = trajectory.samples[index - 1];
        let current = trajectory.samples[index];
        let previous_speed = speed(previous);
        let current_speed = speed(current);
        if previous_speed > speed_threshold_mps && current_speed <= speed_threshold_mps {
            let fraction =
                (previous_speed - speed_threshold_mps) / (previous_speed - current_speed);
            crossing = Some((
                previous.time_s + fraction * (current.time_s - previous.time_s),
                previous.x_m + fraction * (current.x_m - previous.x_m),
                previous.y_m + fraction * (current.y_m - previous.y_m),
                speed_threshold_mps,
            ));
            break;
        }
    }

    let (stop_time_s, stop_x_m, stop_y_m, measured_speed) =
        crossing.ok_or(TrajectoryError::StopThresholdNotReached)?;
    let dx = stop_x_m - command.x_m;
    let dy = stop_y_m - command.y_m;
    let (sin_h, cos_h) = command.heading_rad.sin_cos();

    Ok(StoppingMetrics {
        time_from_command_s: stop_time_s - command.time_s,
        along_track_displacement_m: dx * cos_h + dy * sin_h,
        cross_track_displacement_m: -dx * sin_h + dy * cos_h,
        speed_at_threshold_mps: measured_speed,
    })
}

fn speed(sample: TrajectorySample) -> f64 {
    sample.surge_mps.hypot(sample.sway_mps)
}

#[derive(Clone, Copy)]
struct Crossing {
    time_s: f64,
    x_m: f64,
    y_m: f64,
}

fn heading_crossing(
    trajectory: &Trajectory,
    headings: &[f64],
    direction: f64,
    target_degrees: f64,
) -> Option<Crossing> {
    let target = target_degrees.to_radians();
    for index in 1..headings.len() {
        let before = direction * (headings[index - 1] - headings[0]);
        let after = direction * (headings[index] - headings[0]);
        if before <= target && after >= target && after > before {
            let fraction = (target - before) / (after - before);
            let a = trajectory.samples[index - 1];
            let b = trajectory.samples[index];
            return Some(Crossing {
                time_s: a.time_s + fraction * (b.time_s - a.time_s),
                x_m: a.x_m + fraction * (b.x_m - a.x_m),
                y_m: a.y_m + fraction * (b.y_m - a.y_m),
            });
        }
    }
    None
}

fn unwrap_headings(samples: &[TrajectorySample]) -> Result<Vec<f64>, TrajectoryError> {
    let mut result = Vec::with_capacity(samples.len());
    result.push(samples[0].heading_rad);
    for (index, pair) in samples.windows(2).enumerate() {
        let raw_delta = pair[1].heading_rad - pair[0].heading_rad;
        let delta = (raw_delta + PI).rem_euclid(TAU) - PI;
        if (delta.abs() - PI).abs() <= EPSILON {
            return Err(TrajectoryError::AmbiguousHeadingIncrement(index + 1));
        }
        result.push(result[index] + delta);
    }
    Ok(result)
}
