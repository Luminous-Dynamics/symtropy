// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CPU-first reference models for biological motility research.
//!
//! This crate provides CPU-first reference models for bacterial movement in
//! porous media. Physical event mechanism, observed event-duration bin, and
//! evidence status are distinct model dimensions. It includes explicit
//! renewal waiting-time laws, right-censor-aware survival estimation, and
//! finite-window model comparison. It is not a validated flagellar motor,
//! pore-resolved hydrodynamics, or biological prediction system.
//!
//! The one-second boundary is an observational analysis convention. It never
//! assigns a physical mechanism; duration alone cannot prove a motor event or
//! a passive mechanical trap.

#![forbid(unsafe_code)]

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

mod renewal;
pub use renewal::{
    DirectionalWaitingTimes, SurvivalCurve, SurvivalError, SurvivalObservation,
    SurvivalOutcome, SurvivalPoint, WaitingTimeCandidate, WaitingTimeCandidateScore,
    WaitingTimeComparison, WaitingTimeComparisonError, WaitingTimeLaw, WaitingTimeLawError,
    compare_waiting_time_models, kaplan_meier,
};

pub const EVENT_CLASS_THRESHOLD_S: f64 = 1.0;
/// Numerical deadband around a mathematically cross-gradient direction.
pub const DIRECTION_ALIGNMENT_EPSILON: f64 = 1.0e-12;
/// Hard guard against non-advancing or unreasonably fine integration loops.
pub const MAX_INTEGRATION_STEPS_PER_RUN: usize = 1_000_000;
const DEFAULT_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Direction of a run relative to the chemoattractant gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionClass {
    UpGradient,
    CrossGradient,
    DownGradient,
}

/// Physical explanation represented by a simulated or inferred event.
///
/// This is intentionally independent of the observed duration bin. A long
/// active motor event and a short passive trap are both representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventMechanism {
    ActiveMotorEvent,
    PassiveMechanicalTrap,
    Unresolved,
}

/// Observational duration subgroup, derived independently of physical cause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventDurationBin {
    Short,
    Long,
    ExactBoundaryExcluded,
}

/// Evidence status attached to a mechanism assignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventEvidenceStatus {
    /// Mechanism is directly supported by an appropriate measurement.
    DirectlyObserved,
    /// Mechanism is inferred by a documented analysis procedure.
    AlgorithmInferred,
    /// Mechanism is a known latent variable in a controlled simulation.
    ModelLatent,
    /// Evidence does not support assigning a mechanism.
    Unresolved,
}

/// Probabilities of the three physical event mechanisms.
///
/// The values must be finite, lie in [0, 1], and sum to one within the
/// declared floating-point validation tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MechanismProbabilities {
    pub active_motor: f64,
    pub passive_trap: f64,
    pub unresolved: f64,
}

impl MechanismProbabilities {
    fn validate(self) -> Result<(), ProfileError> {
        let values = [self.active_motor, self.passive_trap, self.unresolved];
        if values
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(ProfileError::InvalidMechanismProbabilities);
        }
        let sum = self.active_motor + self.passive_trap + self.unresolved;
        if (sum - 1.0).abs() > 1.0e-12 {
            return Err(ProfileError::InvalidMechanismProbabilities);
        }
        Ok(())
    }

    fn sample(self, rng: &mut SeededRng) -> EventMechanism {
        let draw = rng.next_unit_f64();
        if draw < self.active_motor {
            EventMechanism::ActiveMotorEvent
        } else if draw < self.active_motor + self.passive_trap {
            EventMechanism::PassiveMechanicalTrap
        } else {
            EventMechanism::Unresolved
        }
    }
}

/// The cited papers use both <= 1 s and < 1 s in different figure/method
/// descriptions. The equality convention must therefore be explicit in every
/// source-bound benchmark manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdBoundaryPolicy {
    /// Exactly one second is assigned to short reorientation.
    ShortInclusive,
    /// Exactly one second is assigned to long trapping.
    LongInclusive,
    /// Exactly one second is excluded because the source convention is unresolved.
    ExcludeExactBoundary,
}

/// A finite empirical sample set used as a transparent non-parametric kernel.
///
/// The samples should come from a source-pinned, rights-admitted dataset. The
/// crate never treats a synthetic sample set as biological calibration.
#[derive(Debug, Clone, PartialEq)]
pub struct SampleSet {
    values: Vec<f64>,
}

impl SampleSet {
    pub fn new(values: Vec<f64>) -> Self {
        Self { values }
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn validate_finite(&self, name: &'static str) -> Result<(), ProfileError> {
        if self.values.is_empty() {
            return Err(ProfileError::EmptySamples(name));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(ProfileError::NonFiniteSample(name));
        }
        Ok(())
    }

    fn validate_positive(&self, name: &'static str) -> Result<(), ProfileError> {
        self.validate_finite(name)?;
        if self.values.iter().any(|value| *value <= 0.0) {
            return Err(ProfileError::NonPositiveSample(name));
        }
        Ok(())
    }

    fn sample(&self, rng: &mut SeededRng) -> f64 {
        let index = rng.index(self.values.len());
        self.values[index]
    }
}

/// Empirical samples indexed by an explicitly declared direction observable.
/// The profile field determines whether that observable is run-start heading
/// or completed-run displacement direction.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectionalSamples {
    pub up_gradient: SampleSet,
    pub cross_gradient: SampleSet,
    pub down_gradient: SampleSet,
}

impl DirectionalSamples {
    pub fn for_direction(&self, direction: DirectionClass) -> &SampleSet {
        match direction {
            DirectionClass::UpGradient => &self.up_gradient,
            DirectionClass::CrossGradient => &self.cross_gradient,
            DirectionClass::DownGradient => &self.down_gradient,
        }
    }

    fn validate_finite(&self, name: &'static str) -> Result<(), ProfileError> {
        self.up_gradient.validate_finite(name)?;
        self.cross_gradient.validate_finite(name)?;
        self.down_gradient.validate_finite(name)?;
        Ok(())
    }

    fn validate_positive(&self, name: &'static str) -> Result<(), ProfileError> {
        self.up_gradient.validate_positive(name)?;
        self.cross_gradient.validate_positive(name)?;
        self.down_gradient.validate_positive(name)?;
        Ok(())
    }

    fn sample(&self, direction: DirectionClass, rng: &mut SeededRng) -> f64 {
        self.for_direction(direction).sample(rng)
    }
}

/// Which directional chemotaxis mechanisms are enabled for a matched ablation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BiasMode {
    /// Both run-duration and short-turn-angle distributions are direction-independent.
    None,
    /// Only run-duration distributions depend on the preceding run direction.
    RunTimeOnly,
    /// Only short-event turn-angle distributions depend on the preceding run direction.
    TurnAngleOnly,
    /// Both run-duration and short-turn-angle distributions are direction-conditioned.
    Combined,
}

impl BiasMode {
    const fn uses_run_time_bias(self) -> bool {
        matches!(self, Self::RunTimeOnly | Self::Combined)
    }

    const fn uses_turn_angle_bias(self) -> bool {
        matches!(self, Self::TurnAngleOnly | Self::Combined)
    }
}

/// A sampled event profile for one declared biological condition.
///
/// Run-time samples are keyed by run-start heading. Active motor-event turn
/// samples are keyed by the realized net displacement direction of the
/// completed run. Physical event mechanisms own their own duration samples;
/// duration bins are derived afterward and never establish mechanism.
#[derive(Debug, Clone, PartialEq)]
pub struct PorousMotilityProfile {
    pub swimming_speed_um_s: f64,
    pub rotational_diffusion_rad2_s: f64,
    pub integration_step_s: f64,
    pub run_duration_s: DirectionalWaitingTimes,
    pub run_duration_unconditional_s: WaitingTimeLaw,
    pub mechanism_probabilities: MechanismProbabilities,
    pub threshold_boundary_policy: ThresholdBoundaryPolicy,
    /// Duration samples conditioned on an active motor event.
    pub active_motor_event_duration_s: WaitingTimeLaw,
    /// Duration law conditioned on passive mechanical trapping.
    pub passive_trap_event_duration_s: WaitingTimeLaw,
    /// Duration law when event cause remains unresolved.
    pub unresolved_event_duration_s: WaitingTimeLaw,
    /// Direction-conditioned turns for active motor events.
    pub active_motor_turn_angle_rad: DirectionalSamples,
    /// Pooled active-turn angles for the turn-angle-bias ablation.
    pub active_motor_turn_angle_unconditional_rad: SampleSet,
    /// Passive-trap escape/reorientation angles; never chemotactically biased here.
    pub passive_trap_turn_angle_rad: SampleSet,
    /// Pooled turn angles for unresolved events.
    pub unresolved_turn_angle_rad: SampleSet,
}

impl PorousMotilityProfile {
    pub fn validate(&self) -> Result<(), ProfileError> {
        if !self.swimming_speed_um_s.is_finite() || self.swimming_speed_um_s <= 0.0 {
            return Err(ProfileError::InvalidSwimmingSpeed);
        }
        if !self.rotational_diffusion_rad2_s.is_finite()
            || self.rotational_diffusion_rad2_s < 0.0
        {
            return Err(ProfileError::InvalidRotationalDiffusion);
        }
        if !self.integration_step_s.is_finite() || self.integration_step_s <= 0.0 {
            return Err(ProfileError::InvalidIntegrationStep);
        }

        self.mechanism_probabilities.validate()?;
        self.run_duration_s.validate()?;
        self.run_duration_unconditional_s.validate()?;
        self.active_motor_event_duration_s.validate()?;
        self.passive_trap_event_duration_s.validate()?;
        self.unresolved_event_duration_s.validate()?;
        self.active_motor_turn_angle_rad
            .validate_finite("active_motor_turn_angle_rad")?;
        self.active_motor_turn_angle_unconditional_rad
            .validate_finite("active_motor_turn_angle_unconditional_rad")?;
        self.passive_trap_turn_angle_rad
            .validate_finite("passive_trap_turn_angle_rad")?;
        self.unresolved_turn_angle_rad
            .validate_finite("unresolved_turn_angle_rad")?;
        Ok(())
    }

    pub fn duration_bin(&self, duration_s: f64) -> Result<EventDurationBin, ProfileError> {
        classify_event_duration(duration_s, EVENT_CLASS_THRESHOLD_S, self.threshold_boundary_policy)
    }
}

/// Derive an observational duration bin without inferring physical cause.
pub fn classify_event_duration(
    duration_s: f64,
    threshold_s: f64,
    policy: ThresholdBoundaryPolicy,
) -> Result<EventDurationBin, ProfileError> {
    if !duration_s.is_finite() || duration_s <= 0.0 {
        return Err(ProfileError::InvalidEventDuration);
    }
    if !threshold_s.is_finite() || threshold_s <= 0.0 {
        return Err(ProfileError::InvalidEventThreshold);
    }

    if duration_s < threshold_s {
        Ok(EventDurationBin::Short)
    } else if duration_s > threshold_s {
        Ok(EventDurationBin::Long)
    } else {
        match policy {
            ThresholdBoundaryPolicy::ShortInclusive => Ok(EventDurationBin::Short),
            ThresholdBoundaryPolicy::LongInclusive => Ok(EventDurationBin::Long),
            ThresholdBoundaryPolicy::ExcludeExactBoundary => {
                Ok(EventDurationBin::ExactBoundaryExcluded)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileError {
    EmptySamples(&'static str),
    NonFiniteSample(&'static str),
    NonPositiveSample(&'static str),
    InvalidSwimmingSpeed,
    InvalidRotationalDiffusion,
    InvalidIntegrationStep,
    InvalidMechanismProbabilities,
    InvalidWaitingTimeLaw(WaitingTimeLawError),
    InvalidEventDuration,
    InvalidEventThreshold,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples(name) => {
                write!(formatter, "{name} must contain at least one sample")
            }
            Self::NonFiniteSample(name) => {
                write!(formatter, "{name} contains a non-finite sample")
            }
            Self::NonPositiveSample(name) => {
                write!(formatter, "{name} must contain positive samples")
            }
            Self::InvalidSwimmingSpeed => {
                write!(formatter, "swimming_speed_um_s must be finite and positive")
            }
            Self::InvalidRotationalDiffusion => {
                write!(formatter, "rotational_diffusion_rad2_s must be finite and non-negative")
            }
            Self::InvalidIntegrationStep => {
                write!(formatter, "integration_step_s must be finite and positive")
            }
            Self::InvalidMechanismProbabilities => {
                write!(
                    formatter,
                    "event mechanism probabilities must be finite, bounded and sum to one"
                )
            }
            Self::InvalidWaitingTimeLaw(error) => {
                write!(formatter, "invalid waiting-time law: {error}")
            }
            Self::InvalidEventDuration => {
                write!(formatter, "event duration must be finite and positive")
            }
            Self::InvalidEventThreshold => {
                write!(formatter, "event duration threshold must be finite and positive")
            }
        }
    }
}

impl Error for ProfileError {}

impl From<WaitingTimeLawError> for ProfileError {
    fn from(value: WaitingTimeLawError) -> Self {
        Self::InvalidWaitingTimeLaw(value)
    }
}

/// Canonical two-dimensional agent state.
///
/// Positions are in micrometres, heading is in radians, and elapsed time is
/// in seconds. The environment is treated as stationary during one run/event
/// pair. Spatial pore geometry and hydrodynamic interactions are not yet
/// resolved by this initial event-based kernel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AgentState {
    pub x_um: f64,
    pub y_um: f64,
    pub heading_rad: f64,
    pub elapsed_s: f64,
}

impl AgentState {
    pub fn new(x_um: f64, y_um: f64, heading_rad: f64) -> Self {
        Self {
            x_um,
            y_um,
            heading_rad: wrap_angle(heading_rad),
            elapsed_s: 0.0,
        }
    }

    fn is_valid(self) -> bool {
        self.x_um.is_finite()
            && self.y_um.is_finite()
            && self.heading_rad.is_finite()
            && self.elapsed_s.is_finite()
            && self.elapsed_s >= 0.0
    }
}

/// One completed run followed by one reorientation or trap event.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EventRecord {
    /// Heading class at run start; used to select the causal run-time policy.
    pub direction_before_run: DirectionClass,
    /// Direction class of the realized net run displacement; used to condition
    /// empirical turn-angle distributions and post-event outcome statistics.
    pub completed_run_direction: DirectionClass,
    /// Net run-displacement heading from the first to last simulated position.
    /// None means the run had no numerically resolvable net displacement.
    pub run_displacement_heading_rad: Option<f64>,
    /// Magnitude of the net run displacement in micrometres.
    pub run_displacement_um: f64,
    /// Instantaneous tangent at the end of the run, before the interruption.
    pub incoming_heading_rad: f64,
    pub run_duration_s: f64,
    /// Physical mechanism selected by the model; never inferred from duration.
    pub mechanism: EventMechanism,
    /// Evidence class for the mechanism assignment. Simulated ground-truth
    /// labels are MODEL_LATENT, not direct biological observations.
    pub evidence_status: EventEvidenceStatus,
    /// Duration subgroup derived independently from the physical mechanism.
    pub duration_bin: EventDurationBin,
    pub event_duration_s: f64,
    pub turn_angle_rad: f64,
    pub heading_after_event_rad: f64,
}

/// Time-stamped position along a simulated path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryPoint {
    pub time_s: f64,
    pub x_um: f64,
    pub y_um: f64,
}

/// Result of a deterministic, seeded event-based simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    pub initial_state: AgentState,
    pub final_state: AgentState,
    pub gradient_heading_rad: f64,
    pub events: Vec<EventRecord>,
    /// Run integration points plus stationary-event endpoints. This trace is
    /// the source for time-origin-averaged trajectory statistics.
    pub trace: Vec<TrajectoryPoint>,
}

/// Summary measures calculated from a simulated trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MechanismDurationContingency {
    pub active_motor_short: usize,
    pub active_motor_long: usize,
    pub active_motor_exact_boundary_excluded: usize,
    pub passive_trap_short: usize,
    pub passive_trap_long: usize,
    pub passive_trap_exact_boundary_excluded: usize,
    pub unresolved_short: usize,
    pub unresolved_long: usize,
    pub unresolved_exact_boundary_excluded: usize,
}

/// Counts mechanisms and duration bins on independent axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventEvidenceCounts {
    pub directly_observed: usize,
    pub algorithm_inferred: usize,
    pub model_latent: usize,
    pub unresolved: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BehaviorSummary {
    pub event_count: usize,
    /// Event counts by observed duration bin (not physical cause).
    pub short_duration_event_count: usize,
    pub long_duration_event_count: usize,
    pub exact_boundary_excluded_event_count: usize,
    /// Event counts by physical mechanism (not duration).
    pub active_motor_event_count: usize,
    pub passive_trap_event_count: usize,
    pub unresolved_mechanism_event_count: usize,
    /// Full mechanism × duration cross-tabulation.
    pub mechanism_duration_contingency: MechanismDurationContingency,
    /// Provenance/evidence-status marginal counts.
    pub evidence_status_counts: EventEvidenceCounts,
    /// Number of completed runs with a resolved net displacement direction.
    pub resolved_run_direction_count: usize,
    pub up_gradient_run_count: usize,
    pub up_gradient_run_fraction: Option<f64>,
    /// Next-run orientation conditioned on the previous event's duration bin.
    pub short_duration_followed_run_count: usize,
    pub short_duration_followed_up_gradient_count: usize,
    pub short_duration_followed_up_gradient_fraction: Option<f64>,
    pub long_duration_followed_run_count: usize,
    pub long_duration_followed_up_gradient_count: usize,
    pub long_duration_followed_up_gradient_fraction: Option<f64>,
    /// Next-run orientation conditioned on the previous event's mechanism.
    pub active_motor_followed_run_count: usize,
    pub active_motor_followed_up_gradient_count: usize,
    pub active_motor_followed_up_gradient_fraction: Option<f64>,
    pub passive_trap_followed_run_count: usize,
    pub passive_trap_followed_up_gradient_count: usize,
    pub passive_trap_followed_up_gradient_fraction: Option<f64>,
    pub unresolved_mechanism_followed_run_count: usize,
    pub unresolved_mechanism_followed_up_gradient_count: usize,
    pub unresolved_mechanism_followed_up_gradient_fraction: Option<f64>,
    pub mean_run_duration_s: Option<f64>,
    pub projected_drift_um_s: Option<f64>,
}

/// Diagnostic for treating SHORT as an active-motor proxy and LONG as
/// a passive-trap proxy.
///
/// Only DIRECTLY_OBSERVED and MODEL_LATENT events with known active/passive
/// mechanisms contribute to the confusion matrix. Algorithm-inferred,
/// unresolved, and exact-boundary-excluded events are reported as excluded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DurationMechanismDiagnostic {
    pub true_positive_active_short: usize,
    pub false_positive_passive_short: usize,
    pub true_negative_passive_long: usize,
    pub false_negative_active_long: usize,
    pub excluded_algorithm_inferred: usize,
    pub excluded_unresolved: usize,
    pub excluded_exact_boundary: usize,
    pub known_event_count: usize,
    pub sensitivity: Option<f64>,
    pub specificity: Option<f64>,
    pub positive_predictive_value: Option<f64>,
    pub negative_predictive_value: Option<f64>,
    pub balanced_accuracy: Option<f64>,
}

/// Evaluate the one-second duration bin as a descriptive proxy for mechanism.
///
/// This is a diagnostic, never a mechanism-assignment rule. Events with
/// ALGORITHM_INFERRED or UNRESOLVED evidence are not treated as ground truth.
pub fn duration_mechanism_diagnostic(
    events: &[EventRecord],
) -> DurationMechanismDiagnostic {
    let mut diagnostic = DurationMechanismDiagnostic {
        true_positive_active_short: 0,
        false_positive_passive_short: 0,
        true_negative_passive_long: 0,
        false_negative_active_long: 0,
        excluded_algorithm_inferred: 0,
        excluded_unresolved: 0,
        excluded_exact_boundary: 0,
        known_event_count: 0,
        sensitivity: None,
        specificity: None,
        positive_predictive_value: None,
        negative_predictive_value: None,
        balanced_accuracy: None,
    };

    for event in events {
        if event.duration_bin == EventDurationBin::ExactBoundaryExcluded {
            diagnostic.excluded_exact_boundary += 1;
            continue;
        }

        if event.mechanism == EventMechanism::Unresolved
            || event.evidence_status == EventEvidenceStatus::Unresolved
        {
            diagnostic.excluded_unresolved += 1;
            continue;
        }

        if event.evidence_status == EventEvidenceStatus::AlgorithmInferred {
            diagnostic.excluded_algorithm_inferred += 1;
            continue;
        }

        diagnostic.known_event_count += 1;
        match (event.mechanism, event.duration_bin) {
            (EventMechanism::ActiveMotorEvent, EventDurationBin::Short) => {
                diagnostic.true_positive_active_short += 1
            }
            (EventMechanism::PassiveMechanicalTrap, EventDurationBin::Short) => {
                diagnostic.false_positive_passive_short += 1
            }
            (EventMechanism::PassiveMechanicalTrap, EventDurationBin::Long) => {
                diagnostic.true_negative_passive_long += 1
            }
            (EventMechanism::ActiveMotorEvent, EventDurationBin::Long) => {
                diagnostic.false_negative_active_long += 1
            }
            (_, EventDurationBin::ExactBoundaryExcluded) => {}
            (EventMechanism::Unresolved, _) => {}
        }
    }

    let tp = diagnostic.true_positive_active_short as f64;
    let fp = diagnostic.false_positive_passive_short as f64;
    let tn = diagnostic.true_negative_passive_long as f64;
    let fn_ = diagnostic.false_negative_active_long as f64;

    diagnostic.sensitivity = (tp + fn_ > 0.0).then(|| tp / (tp + fn_));
    diagnostic.specificity = (tn + fp > 0.0).then(|| tn / (tn + fp));
    diagnostic.positive_predictive_value = (tp + fp > 0.0).then(|| tp / (tp + fp));
    diagnostic.negative_predictive_value = (tn + fn_ > 0.0).then(|| tn / (tn + fn_));
    diagnostic.balanced_accuracy = match (diagnostic.sensitivity, diagnostic.specificity) {
        (Some(sensitivity), Some(specificity)) => Some((sensitivity + specificity) / 2.0),
        _ => None,
    };

    diagnostic
}

/// Circular statistics for realized run-displacement orientations relative
/// to the chemoattractant gradient axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientationSummary {
    pub run_count: usize,
    pub up_gradient_count: usize,
    pub down_gradient_count: usize,
    pub cross_gradient_count: usize,
    pub up_gradient_fraction: f64,
    /// Real and imaginary components of the first circular Fourier moment.
    pub first_harmonic_real: f64,
    pub first_harmonic_imag: f64,
    pub first_harmonic_magnitude: f64,
    /// Undefined when the first harmonic magnitude is numerically zero.
    pub first_harmonic_phase_rad: Option<f64>,
}

/// Percentile confidence interval for a clustered bootstrap statistic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootstrapInterval {
    pub estimate: f64,
    pub lower: f64,
    pub upper: f64,
}

/// Cluster-level bootstrap over independent biological recording units.
///
/// Each inner vector contains realized run-displacement headings in radians,
/// already expressed relative to the gradient axis, for one independent
/// recording/plate/day cluster. Clusters are sampled with replacement; all
/// runs from a selected cluster remain together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClusteredOrientationBootstrap {
    pub up_gradient_fraction: BootstrapInterval,
    pub first_harmonic_magnitude: BootstrapInterval,
    pub independent_cluster_count: usize,
    pub resample_count: usize,
}

/// Errors returned by the clustered orientation bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootstrapError {
    NoClusters,
    InsufficientIndependentClusters,
    EmptyCluster,
    NonFiniteAngle,
    InvalidConfidenceLevel,
    ZeroResamples,
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoClusters => {
                write!(formatter, "bootstrap requires at least one independent cluster")
            }
            Self::InsufficientIndependentClusters => {
                write!(formatter, "cluster bootstrap requires at least two independent clusters")
            }
            Self::EmptyCluster => {
                write!(formatter, "each bootstrap cluster must contain at least one run")
            }
            Self::NonFiniteAngle => write!(formatter, "orientation angles must be finite"),
            Self::InvalidConfidenceLevel => {
                write!(formatter, "confidence level must be strictly between zero and one")
            }
            Self::ZeroResamples => {
                write!(formatter, "bootstrap resample count must be positive")
            }
        }
    }
}

impl Error for BootstrapError {}

/// Summarize net run-displacement headings relative to the gradient.
///
/// This matches the first-harmonic representation used in the porous-agar
/// chemotaxis study: p(phi) is approximated by a constant plus its first
/// circular Fourier mode. Run orientation is the first-to-last displacement
/// vector, not the instantaneous endpoint tangent.
pub fn summarize_orientations(result: &SimulationResult) -> Option<OrientationSummary> {
    let relative_headings: Vec<f64> = result
        .events
        .iter()
        .filter_map(|event| {
            event
                .run_displacement_heading_rad
                .map(|heading| wrap_angle(heading - result.gradient_heading_rad))
        })
        .collect();
    summarize_relative_headings(&relative_headings)
}

fn summarize_relative_headings(headings: &[f64]) -> Option<OrientationSummary> {
    if headings.is_empty() || headings.iter().any(|heading| !heading.is_finite()) {
        return None;
    }

    let mut up_gradient_count = 0;
    let mut down_gradient_count = 0;
    let mut cross_gradient_count = 0;
    let mut cosine_sum = 0.0;
    let mut sine_sum = 0.0;

    for &heading in headings {
        match classify_direction(heading, 0.0)? {
            DirectionClass::UpGradient => up_gradient_count += 1,
            DirectionClass::DownGradient => down_gradient_count += 1,
            DirectionClass::CrossGradient => cross_gradient_count += 1,
        }
        cosine_sum += heading.cos();
        sine_sum += heading.sin();
    }

    let run_count = headings.len();
    let first_harmonic_real = cosine_sum / run_count as f64;
    let first_harmonic_imag = sine_sum / run_count as f64;
    let first_harmonic_magnitude =
        first_harmonic_real.hypot(first_harmonic_imag);

    Some(OrientationSummary {
        run_count,
        up_gradient_count,
        down_gradient_count,
        cross_gradient_count,
        up_gradient_fraction: up_gradient_count as f64 / run_count as f64,
        first_harmonic_real,
        first_harmonic_imag,
        first_harmonic_magnitude,
        first_harmonic_phase_rad: (first_harmonic_magnitude > 1.0e-14)
            .then(|| first_harmonic_imag.atan2(first_harmonic_real)),
    })
}

/// Estimate confidence intervals for up-gradient fraction and first Fourier
/// harmonic magnitude by resampling independent recording clusters.
///
/// The point estimate pools all run observations. Each bootstrap draw samples
/// the same number of clusters with replacement and includes all runs from
/// each selected cluster. This preserves within-recording dependence.
pub fn cluster_bootstrap_orientations(
    clusters: &[Vec<f64>],
    resample_count: usize,
    confidence_level: f64,
    seed: u64,
) -> Result<ClusteredOrientationBootstrap, BootstrapError> {
    if clusters.is_empty() {
        return Err(BootstrapError::NoClusters);
    }
    if clusters.len() < 2 {
        return Err(BootstrapError::InsufficientIndependentClusters);
    }
    if resample_count == 0 {
        return Err(BootstrapError::ZeroResamples);
    }
    if !confidence_level.is_finite() || !(0.0..1.0).contains(&confidence_level) {
        return Err(BootstrapError::InvalidConfidenceLevel);
    }
    if clusters.iter().any(Vec::is_empty) {
        return Err(BootstrapError::EmptyCluster);
    }
    if clusters.iter().flatten().any(|angle| !angle.is_finite()) {
        return Err(BootstrapError::NonFiniteAngle);
    }

    let pooled: Vec<f64> = clusters.iter().flatten().copied().collect();
    let point = summarize_relative_headings(&pooled)
        .expect("validated non-empty finite orientation clusters");

    let mut rng = SeededRng::new(seed);
    let mut up_fraction_samples = Vec::with_capacity(resample_count);
    let mut first_harmonic_samples = Vec::with_capacity(resample_count);

    for _ in 0..resample_count {
        let mut resampled = Vec::with_capacity(pooled.len());
        for _ in 0..clusters.len() {
            let cluster_index = rng.index(clusters.len());
            resampled.extend_from_slice(&clusters[cluster_index]);
        }
        let summary = summarize_relative_headings(&resampled)
            .expect("resampled clusters are non-empty and finite");
        up_fraction_samples.push(summary.up_gradient_fraction);
        first_harmonic_samples.push(summary.first_harmonic_magnitude);
    }

    let alpha = 1.0 - confidence_level;
    Ok(ClusteredOrientationBootstrap {
        up_gradient_fraction: BootstrapInterval {
            estimate: point.up_gradient_fraction,
            lower: percentile(&mut up_fraction_samples, alpha / 2.0),
            upper: percentile(&mut up_fraction_samples, 1.0 - alpha / 2.0),
        },
        first_harmonic_magnitude: BootstrapInterval {
            estimate: point.first_harmonic_magnitude,
            lower: percentile(&mut first_harmonic_samples, alpha / 2.0),
            upper: percentile(&mut first_harmonic_samples, 1.0 - alpha / 2.0),
        },
        independent_cluster_count: clusters.len(),
        resample_count,
    })
}

fn percentile(samples: &mut [f64], quantile: f64) -> f64 {
    samples.sort_by(f64::total_cmp);
    let position = quantile * (samples.len() - 1) as f64;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    if lower_index == upper_index {
        samples[lower_index]
    } else {
        let fraction = position - lower_index as f64;
        samples[lower_index] + fraction * (samples[upper_index] - samples[lower_index])
    }
}

/// Per-lag, time-origin-averaged ensemble MSD estimate.
///
/// Each trajectory contributes one time-origin average, so frames and time
/// origins are not treated as independent biological replicates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MsdEstimate {
    pub lag_time_s: f64,
    pub mean_squared_displacement_um2: f64,
    pub trajectory_count: usize,
    /// Sample standard deviation across trajectory-level MSD estimates.
    pub between_trajectory_sd_um2: Option<f64>,
}

const MAX_MSD_TIME_ORIGINS_PER_TRAJECTORY: usize = 1_000_000;

/// Errors returned by the trajectory-level MSD evaluator.
#[derive(Debug, Clone, PartialEq)]
pub enum MsdError {
    EmptyEnsemble,
    InvalidLagTime,
    InvalidTimeOriginStep,
    InvalidAgeingTime,
    InvalidTrace,
    TooManyTimeOrigins,
    NoTrajectorySupportsLag(f64),
    NonFiniteStatistic,
}

impl fmt::Display for MsdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyEnsemble => {
                write!(formatter, "MSD requires at least one trajectory")
            }
            Self::InvalidLagTime => {
                write!(formatter, "MSD lag times must be finite and non-negative")
            }
            Self::InvalidTimeOriginStep => {
                write!(formatter, "MSD time-origin step must be finite and positive")
            }
            Self::InvalidAgeingTime => {
                write!(formatter, "MSD ageing time must be finite and non-negative")
            }
            Self::InvalidTrace => {
                write!(formatter, "trajectory trace must be finite and strictly time-ordered")
            }
            Self::TooManyTimeOrigins => {
                write!(formatter, "MSD time-origin count exceeds the configured limit")
            }
            Self::NoTrajectorySupportsLag(lag) => {
                write!(formatter, "no trajectory supports lag time {lag} seconds")
            }
            Self::NonFiniteStatistic => {
                write!(formatter, "MSD calculation produced a non-finite statistic")
            }
        }
    }
}

impl Error for MsdError {}

/// Errors returned before or during simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidProfile(ProfileError),
    InvalidInitialState,
    InvalidGradientHeading,
    NonFiniteTrajectory,
    IntegrationStepBudgetExceeded,
    IntegrationStepDidNotAdvance,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => {
                write!(formatter, "invalid motility profile: {error}")
            }
            Self::InvalidInitialState => {
                write!(
                    formatter,
                    "initial agent state must be finite with non-negative elapsed time"
                )
            }
            Self::InvalidGradientHeading => write!(formatter, "gradient heading must be finite"),
            Self::NonFiniteTrajectory => {
                write!(
                    formatter,
                    "simulation produced a non-finite state; check profile magnitudes"
                )
            }
            Self::IntegrationStepBudgetExceeded => {
                write!(formatter, "run exceeded the maximum integration-step budget")
            }
            Self::IntegrationStepDidNotAdvance => {
                write!(formatter, "integration step is too small to advance the remaining run time")
            }
        }
    }
}

impl Error for SimulationError {}

impl From<ProfileError> for SimulationError {
    fn from(value: ProfileError) -> Self {
        Self::InvalidProfile(value)
    }
}

/// Classify a heading by the sign of its projection onto the gradient axis.
///
/// Up-gradient and down-gradient are the two opposing half-planes. A narrow
/// numerical deadband only catches effectively perpendicular headings; it is
/// not a biological angular cone. The cited paper reports up/down-gradient
/// groups and angular distributions, but the precise classification rule must
/// still be confirmed against its data-analysis supplement before reproduction.
pub fn classify_direction(
    heading_rad: f64,
    gradient_heading_rad: f64,
) -> Option<DirectionClass> {
    if !heading_rad.is_finite() || !gradient_heading_rad.is_finite() {
        return None;
    }

    let relative = wrap_angle(heading_rad - gradient_heading_rad);
    let gradient_alignment = relative.cos();

    if gradient_alignment > DIRECTION_ALIGNMENT_EPSILON {
        Some(DirectionClass::UpGradient)
    } else if gradient_alignment < -DIRECTION_ALIGNMENT_EPSILON {
        Some(DirectionClass::DownGradient)
    } else {
        Some(DirectionClass::CrossGradient)
    }
}

/// Run a seeded event-based simulation for a fixed number of run/event cycles.
///
/// Each cycle:
/// 1. samples a run duration conditional on the preceding heading;
/// 2. advances the cell at constant swimming speed;
/// 3. samples a short reorientation or long mechanical trap;
/// 4. applies the corresponding conditional or pooled heading change.
///
/// This is a deliberately bounded reference kernel. It does not yet resolve
/// flagellar motor state, cell-body hydrodynamics, pore geometry, growth or
/// nutrient consumption.
pub fn simulate(
    profile: &PorousMotilityProfile,
    initial_state: AgentState,
    gradient_heading_rad: f64,
    event_count: usize,
    seed: u64,
) -> Result<SimulationResult, SimulationError> {
    simulate_with_bias_mode(
        profile,
        initial_state,
        gradient_heading_rad,
        event_count,
        seed,
        BiasMode::Combined,
    )
}

/// Run the same seeded model with a declared mechanistic ablation.
///
/// All four modes use the same profile, seed and event interface. Ablation
/// modes switch only the direction-conditioning mechanism; the unconditional
/// sample sets must be admitted from the same declared source condition.
pub fn simulate_with_bias_mode(
    profile: &PorousMotilityProfile,
    initial_state: AgentState,
    gradient_heading_rad: f64,
    event_count: usize,
    seed: u64,
    bias_mode: BiasMode,
) -> Result<SimulationResult, SimulationError> {
    profile.validate()?;
    if !initial_state.is_valid() {
        return Err(SimulationError::InvalidInitialState);
    }
    if !gradient_heading_rad.is_finite() {
        return Err(SimulationError::InvalidGradientHeading);
    }

    let mut rng = SeededRng::new(seed);
    let gradient_heading_rad = wrap_angle(gradient_heading_rad);
    let mut state = initial_state;
    state.heading_rad = wrap_angle(state.heading_rad);
    let initial_state = state;
    let mut events = Vec::with_capacity(event_count);
    let mut trace = vec![TrajectoryPoint {
        time_s: state.elapsed_s,
        x_um: state.x_um,
        y_um: state.y_um,
    }];

    for _ in 0..event_count {
        let direction = classify_direction(state.heading_rad, gradient_heading_rad)
        .expect("validated finite headings produce a direction class");

        let run_start_x_um = state.x_um;
        let run_start_y_um = state.y_um;

        let run_duration = if bias_mode.uses_run_time_bias() {
            profile.run_duration_s.sample_with_rng(direction, &mut rng)
        } else {
            profile.run_duration_unconditional_s.sample_with_rng(&mut rng)
        };
        let run_end_time_s = state.elapsed_s + run_duration;
        if !run_end_time_s.is_finite() {
            return Err(SimulationError::NonFiniteTrajectory);
        }
        if run_end_time_s <= state.elapsed_s {
            return Err(SimulationError::IntegrationStepDidNotAdvance);
        }

        let mut integration_steps = 0;
        while state.elapsed_s < run_end_time_s {
            if integration_steps >= MAX_INTEGRATION_STEPS_PER_RUN {
                return Err(SimulationError::IntegrationStepBudgetExceeded);
            }
            let remaining_run_s = run_end_time_s - state.elapsed_s;
            let nominal_step_s = remaining_run_s.min(profile.integration_step_s);
            let next_elapsed_s = if remaining_run_s <= profile.integration_step_s {
                run_end_time_s
            } else {
                state.elapsed_s + nominal_step_s
            };
            if !next_elapsed_s.is_finite() || next_elapsed_s <= state.elapsed_s {
                return Err(SimulationError::IntegrationStepDidNotAdvance);
            }
            let step_s = next_elapsed_s - state.elapsed_s;
            let angular_std = (2.0 * profile.rotational_diffusion_rad2_s * step_s).sqrt();
            let angular_increment = angular_std * rng.next_standard_normal();
            let midpoint_heading = wrap_angle(state.heading_rad + 0.5 * angular_increment);
            let distance = profile.swimming_speed_um_s * step_s;
            state.x_um += distance * midpoint_heading.cos();
            state.y_um += distance * midpoint_heading.sin();
            state.heading_rad = wrap_angle(state.heading_rad + angular_increment);
            state.elapsed_s = next_elapsed_s;
            if !state.is_valid() {
                return Err(SimulationError::NonFiniteTrajectory);
            }
            trace.push(TrajectoryPoint {
                time_s: state.elapsed_s,
                x_um: state.x_um,
                y_um: state.y_um,
            });
            integration_steps += 1;
        }

        let run_displacement_x_um = state.x_um - run_start_x_um;
        let run_displacement_y_um = state.y_um - run_start_y_um;
        let run_displacement_um =
            run_displacement_x_um.hypot(run_displacement_y_um);
        let run_displacement_heading_rad = if run_displacement_um > f64::MIN_POSITIVE {
            Some(run_displacement_y_um.atan2(run_displacement_x_um))
        } else {
            None
        };
        let completed_run_direction = match run_displacement_heading_rad {
            Some(heading) => classify_direction(heading, gradient_heading_rad)
                .expect("validated finite headings produce a direction class"),
            None => DirectionClass::CrossGradient,
        };
        let incoming_heading_rad = state.heading_rad;

        // Mechanism is sampled independently of event duration. Duration
        // bins are an observational analysis view, never a causal label.
        let mechanism = profile.mechanism_probabilities.sample(&mut rng);
        let (event_duration, turn_angle) = match mechanism {
            EventMechanism::ActiveMotorEvent => {
                let angle = if bias_mode.uses_turn_angle_bias() {
                    profile
                        .active_motor_turn_angle_rad
                        .sample(completed_run_direction, &mut rng)
                } else {
                    profile
                        .active_motor_turn_angle_unconditional_rad
                        .sample(&mut rng)
                };
                (
                    profile.active_motor_event_duration_s.sample_with_rng(&mut rng),
                    angle,
                )
            }
            EventMechanism::PassiveMechanicalTrap => (
                profile.passive_trap_event_duration_s.sample_with_rng(&mut rng),
                profile.passive_trap_turn_angle_rad.sample(&mut rng),
            ),
            EventMechanism::Unresolved => (
                profile.unresolved_event_duration_s.sample_with_rng(&mut rng),
                profile.unresolved_turn_angle_rad.sample(&mut rng),
            ),
        };
        let duration_bin = profile.duration_bin(event_duration)?;

        // The event turn is applied relative to the preceding run's net
        // displacement heading. If no net displacement was resolvable, retain
        // the incoming tangent as the only available orientation reference.
        let turn_reference_heading_rad =
            run_displacement_heading_rad.unwrap_or(incoming_heading_rad);
        state.heading_rad =
            wrap_angle(turn_reference_heading_rad + wrap_angle(turn_angle));
        let event_end_time_s = state.elapsed_s + event_duration;
        if !event_end_time_s.is_finite() {
            return Err(SimulationError::NonFiniteTrajectory);
        }
        if event_end_time_s <= state.elapsed_s {
            return Err(SimulationError::IntegrationStepDidNotAdvance);
        }
        state.elapsed_s = event_end_time_s;
        if !state.is_valid() {
            return Err(SimulationError::NonFiniteTrajectory);
        }
        trace.push(TrajectoryPoint {
            time_s: state.elapsed_s,
            x_um: state.x_um,
            y_um: state.y_um,
        });

        events.push(EventRecord {
            direction_before_run: direction,
            completed_run_direction,
            run_displacement_heading_rad,
            run_displacement_um,
            incoming_heading_rad,
            run_duration_s: run_duration,
            mechanism,
            evidence_status: EventEvidenceStatus::ModelLatent,
            duration_bin,
            event_duration_s: event_duration,
            turn_angle_rad: turn_angle,
            heading_after_event_rad: state.heading_rad,
        });
    }

    Ok(SimulationResult {
        initial_state,
        final_state: state,
        gradient_heading_rad: wrap_angle(gradient_heading_rad),
        events,
        trace,
    })
}

/// Compute dimensionally explicit summary measures for a simulated result.
pub fn summarize(result: &SimulationResult) -> BehaviorSummary {
    let short_duration_event_count = result
        .events
        .iter()
        .filter(|event| event.duration_bin == EventDurationBin::Short)
        .count();
    let long_duration_event_count = result
        .events
        .iter()
        .filter(|event| event.duration_bin == EventDurationBin::Long)
        .count();
    let exact_boundary_excluded_event_count = result.events.len()
        - short_duration_event_count
        - long_duration_event_count;

    let active_motor_event_count = result
        .events
        .iter()
        .filter(|event| event.mechanism == EventMechanism::ActiveMotorEvent)
        .count();
    let passive_trap_event_count = result
        .events
        .iter()
        .filter(|event| event.mechanism == EventMechanism::PassiveMechanicalTrap)
        .count();
    let unresolved_mechanism_event_count = result.events.len()
        - active_motor_event_count
        - passive_trap_event_count;

    let mut mechanism_duration_contingency = MechanismDurationContingency {
        active_motor_short: 0,
        active_motor_long: 0,
        active_motor_exact_boundary_excluded: 0,
        passive_trap_short: 0,
        passive_trap_long: 0,
        passive_trap_exact_boundary_excluded: 0,
        unresolved_short: 0,
        unresolved_long: 0,
        unresolved_exact_boundary_excluded: 0,
    };
    let mut evidence_status_counts = EventEvidenceCounts {
        directly_observed: 0,
        algorithm_inferred: 0,
        model_latent: 0,
        unresolved: 0,
    };

    for event in &result.events {
        let cell = match (event.mechanism, event.duration_bin) {
            (EventMechanism::ActiveMotorEvent, EventDurationBin::Short) => {
                &mut mechanism_duration_contingency.active_motor_short
            }
            (EventMechanism::ActiveMotorEvent, EventDurationBin::Long) => {
                &mut mechanism_duration_contingency.active_motor_long
            }
            (
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::ExactBoundaryExcluded,
            ) => &mut mechanism_duration_contingency
                .active_motor_exact_boundary_excluded,
            (EventMechanism::PassiveMechanicalTrap, EventDurationBin::Short) => {
                &mut mechanism_duration_contingency.passive_trap_short
            }
            (EventMechanism::PassiveMechanicalTrap, EventDurationBin::Long) => {
                &mut mechanism_duration_contingency.passive_trap_long
            }
            (
                EventMechanism::PassiveMechanicalTrap,
                EventDurationBin::ExactBoundaryExcluded,
            ) => &mut mechanism_duration_contingency
                .passive_trap_exact_boundary_excluded,
            (EventMechanism::Unresolved, EventDurationBin::Short) => {
                &mut mechanism_duration_contingency.unresolved_short
            }
            (EventMechanism::Unresolved, EventDurationBin::Long) => {
                &mut mechanism_duration_contingency.unresolved_long
            }
            (EventMechanism::Unresolved, EventDurationBin::ExactBoundaryExcluded) => {
                &mut mechanism_duration_contingency
                    .unresolved_exact_boundary_excluded
            }
        };
        *cell += 1;

        match event.evidence_status {
            EventEvidenceStatus::DirectlyObserved => {
                evidence_status_counts.directly_observed += 1
            }
            EventEvidenceStatus::AlgorithmInferred => {
                evidence_status_counts.algorithm_inferred += 1
            }
            EventEvidenceStatus::ModelLatent => evidence_status_counts.model_latent += 1,
            EventEvidenceStatus::Unresolved => evidence_status_counts.unresolved += 1,
        }
    }

    let resolved_run_directions: Vec<_> = result
        .events
        .iter()
        .filter_map(|event| {
            event
                .run_displacement_heading_rad
                .map(|_| event.completed_run_direction)
        })
        .collect();
    let resolved_run_direction_count = resolved_run_directions.len();
    let up_gradient_run_count = resolved_run_directions
        .iter()
        .filter(|direction| **direction == DirectionClass::UpGradient)
        .count();

    // Event i precedes run i+1. The final event has no observed following
    // run and is excluded from every conditional denominator.
    let mut short_duration_followed_run_count = 0;
    let mut short_duration_followed_up_gradient_count = 0;
    let mut long_duration_followed_run_count = 0;
    let mut long_duration_followed_up_gradient_count = 0;

    let mut active_motor_followed_run_count = 0;
    let mut active_motor_followed_up_gradient_count = 0;
    let mut passive_trap_followed_run_count = 0;
    let mut passive_trap_followed_up_gradient_count = 0;
    let mut unresolved_mechanism_followed_run_count = 0;
    let mut unresolved_mechanism_followed_up_gradient_count = 0;

    for pair in result.events.windows(2) {
        let previous_event = pair[0];
        let following_run = pair[1];
        if following_run.run_displacement_heading_rad.is_none() {
            continue;
        }
        let is_up_gradient =
            following_run.completed_run_direction == DirectionClass::UpGradient;

        match previous_event.duration_bin {
            EventDurationBin::Short => {
                short_duration_followed_run_count += 1;
                if is_up_gradient {
                    short_duration_followed_up_gradient_count += 1;
                }
            }
            EventDurationBin::Long => {
                long_duration_followed_run_count += 1;
                if is_up_gradient {
                    long_duration_followed_up_gradient_count += 1;
                }
            }
            EventDurationBin::ExactBoundaryExcluded => {}
        }

        match previous_event.mechanism {
            EventMechanism::ActiveMotorEvent => {
                active_motor_followed_run_count += 1;
                if is_up_gradient {
                    active_motor_followed_up_gradient_count += 1;
                }
            }
            EventMechanism::PassiveMechanicalTrap => {
                passive_trap_followed_run_count += 1;
                if is_up_gradient {
                    passive_trap_followed_up_gradient_count += 1;
                }
            }
            EventMechanism::Unresolved => {
                unresolved_mechanism_followed_run_count += 1;
                if is_up_gradient {
                    unresolved_mechanism_followed_up_gradient_count += 1;
                }
            }
        }
    }

    let total_run_duration: f64 = result
        .events
        .iter()
        .map(|event| event.run_duration_s)
        .sum();

    let elapsed = result.final_state.elapsed_s - result.initial_state.elapsed_s;
    let dx = result.final_state.x_um - result.initial_state.x_um;
    let dy = result.final_state.y_um - result.initial_state.y_um;
    let projected_displacement =
        dx * result.gradient_heading_rad.cos() + dy * result.gradient_heading_rad.sin();

    BehaviorSummary {
        event_count: result.events.len(),
        short_duration_event_count,
        long_duration_event_count,
        exact_boundary_excluded_event_count,
        active_motor_event_count,
        passive_trap_event_count,
        unresolved_mechanism_event_count,
        mechanism_duration_contingency,
        evidence_status_counts,
        resolved_run_direction_count,
        up_gradient_run_count,
        up_gradient_run_fraction: (resolved_run_direction_count > 0)
            .then(|| up_gradient_run_count as f64 / resolved_run_direction_count as f64),
        short_duration_followed_run_count,
        short_duration_followed_up_gradient_count,
        short_duration_followed_up_gradient_fraction: (short_duration_followed_run_count > 0)
            .then(|| {
                short_duration_followed_up_gradient_count as f64
                    / short_duration_followed_run_count as f64
            }),
        long_duration_followed_run_count,
        long_duration_followed_up_gradient_count,
        long_duration_followed_up_gradient_fraction: (long_duration_followed_run_count > 0)
            .then(|| {
                long_duration_followed_up_gradient_count as f64
                    / long_duration_followed_run_count as f64
            }),
        active_motor_followed_run_count,
        active_motor_followed_up_gradient_count,
        active_motor_followed_up_gradient_fraction: (active_motor_followed_run_count > 0)
            .then(|| {
                active_motor_followed_up_gradient_count as f64
                    / active_motor_followed_run_count as f64
            }),
        passive_trap_followed_run_count,
        passive_trap_followed_up_gradient_count,
        passive_trap_followed_up_gradient_fraction: (passive_trap_followed_run_count > 0)
            .then(|| {
                passive_trap_followed_up_gradient_count as f64
                    / passive_trap_followed_run_count as f64
            }),
        unresolved_mechanism_followed_run_count,
        unresolved_mechanism_followed_up_gradient_count,
        unresolved_mechanism_followed_up_gradient_fraction:
            (unresolved_mechanism_followed_run_count > 0).then(|| {
                unresolved_mechanism_followed_up_gradient_count as f64
                    / unresolved_mechanism_followed_run_count as f64
            }),
        mean_run_duration_s: (!result.events.is_empty())
            .then(|| total_run_duration / result.events.len() as f64),
        projected_drift_um_s: (elapsed > 0.0)
            .then(|| projected_displacement / elapsed),
    }
}

/// Compute time-origin-averaged MSD for an ensemble of independently
/// simulated trajectories.
///
/// A trajectory-level time-origin average is computed first; the reported
/// mean and sample standard deviation are then calculated across trajectories.
/// This avoids treating correlated frames from one trajectory as independent
/// biological replicates. The trace is linearly interpolated at requested
/// origin and lag times.
///
/// This evaluator does not bootstrap biological plates/days. The ensemble
/// supplied by the caller must already represent the intended independent
/// simulation or experimental units.
/// Compute time-origin-averaged MSD for an ensemble of trajectories.
///
/// This convenience form evaluates the zero-age curve. Use
/// `mean_squared_displacement_with_age` when the observation begins after a
/// non-zero renewal-process age.
pub fn mean_squared_displacement(
    trajectories: &[SimulationResult],
    lag_times_s: &[f64],
    time_origin_step_s: f64,
) -> Result<Vec<MsdEstimate>, MsdError> {
    mean_squared_displacement_with_age(trajectories, 0.0, lag_times_s, time_origin_step_s)
}

/// Compute time-origin-averaged MSD at a declared renewal-process age.
///
/// `ageing_time_s` is measured from the first trace point. Origin times begin
/// at first_time + ageing_time_s; the trace must therefore contain the
/// pre-observation history as well as the later displacement intervals.
///
/// A trajectory-level time-origin average is computed first; the reported
/// mean and sample standard deviation are then calculated across trajectories.
/// Overlapping time origins and frames are not treated as independent
/// biological replicates.
pub fn mean_squared_displacement_with_age(
    trajectories: &[SimulationResult],
    ageing_time_s: f64,
    lag_times_s: &[f64],
    time_origin_step_s: f64,
) -> Result<Vec<MsdEstimate>, MsdError> {
    if trajectories.is_empty() {
        return Err(MsdError::EmptyEnsemble);
    }
    if !ageing_time_s.is_finite() || ageing_time_s < 0.0 {
        return Err(MsdError::InvalidAgeingTime);
    }
    if !time_origin_step_s.is_finite() || time_origin_step_s <= 0.0 {
        return Err(MsdError::InvalidTimeOriginStep);
    }
    if lag_times_s
        .iter()
        .any(|lag| !lag.is_finite() || *lag < 0.0)
    {
        return Err(MsdError::InvalidLagTime);
    }

    for trajectory in trajectories {
        validate_trace(&trajectory.trace)?;
    }

    let mut estimates = Vec::with_capacity(lag_times_s.len());
    for &lag_time_s in lag_times_s {
        let mut trajectory_means = Vec::with_capacity(trajectories.len());

        for trajectory in trajectories {
            let first = trajectory.trace[0].time_s;
            let last = trajectory.trace[trajectory.trace.len() - 1].time_s;
            let first_origin = first + ageing_time_s;
            if !first_origin.is_finite() {
                return Err(MsdError::InvalidAgeingTime);
            }
            let latest_origin = last - lag_time_s;
            if first_origin > latest_origin {
                continue;
            }

            let origin_count_f64 =
                ((latest_origin - first_origin) / time_origin_step_s).floor() + 1.0;
            if !origin_count_f64.is_finite()
                || origin_count_f64 > MAX_MSD_TIME_ORIGINS_PER_TRAJECTORY as f64
            {
                return Err(MsdError::TooManyTimeOrigins);
            }
            let origin_count = origin_count_f64 as usize;
            if origin_count == 0 {
                continue;
            }

            let mut squared_displacement_sum = 0.0;
            for origin_index in 0..origin_count {
                let origin_time =
                    first_origin + origin_index as f64 * time_origin_step_s;
                let Some((x0, y0)) = interpolate_position(&trajectory.trace, origin_time)
                else {
                    return Err(MsdError::InvalidTrace);
                };
                let Some((x1, y1)) =
                    interpolate_position(&trajectory.trace, origin_time + lag_time_s)
                else {
                    return Err(MsdError::InvalidTrace);
                };
                let dx = x1 - x0;
                let dy = y1 - y0;
                squared_displacement_sum += dx * dx + dy * dy;
            }

            let track_mean = squared_displacement_sum / origin_count as f64;
            if !track_mean.is_finite() {
                return Err(MsdError::NonFiniteStatistic);
            }
            trajectory_means.push(track_mean);
        }

        if trajectory_means.is_empty() {
            return Err(MsdError::NoTrajectorySupportsLag(lag_time_s));
        }

        let trajectory_count = trajectory_means.len();
        let mean = trajectory_means.iter().sum::<f64>() / trajectory_count as f64;
        if !mean.is_finite() {
            return Err(MsdError::NonFiniteStatistic);
        }
        let between_trajectory_sd_um2 = if trajectory_count > 1 {
            let squared_deviations = trajectory_means
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f64>();
            Some((squared_deviations / (trajectory_count - 1) as f64).sqrt())
        } else {
            None
        };

        estimates.push(MsdEstimate {
            lag_time_s,
            mean_squared_displacement_um2: mean,
            trajectory_count,
            between_trajectory_sd_um2,
        });
    }

    Ok(estimates)
}

fn validate_trace(trace: &[TrajectoryPoint]) -> Result<(), MsdError> {
    if trace.is_empty()
        || trace.iter().any(|point| {
            !point.time_s.is_finite() || !point.x_um.is_finite() || !point.y_um.is_finite()
        })
        || trace.windows(2).any(|pair| pair[1].time_s <= pair[0].time_s)
    {
        return Err(MsdError::InvalidTrace);
    }
    Ok(())
}

fn interpolate_position(
    trace: &[TrajectoryPoint],
    time_s: f64,
) -> Option<(f64, f64)> {
    let first = *trace.first()?;
    let last = *trace.last()?;
    if time_s < first.time_s || time_s > last.time_s {
        return None;
    }
    if time_s == first.time_s {
        return Some((first.x_um, first.y_um));
    }
    if time_s == last.time_s {
        return Some((last.x_um, last.y_um));
    }

    for pair in trace.windows(2) {
        let left = pair[0];
        let right = pair[1];
        if time_s <= right.time_s {
            let span = right.time_s - left.time_s;
            if span <= 0.0 {
                return None;
            }
            let fraction = (time_s - left.time_s) / span;
            return Some((
                left.x_um + fraction * (right.x_um - left.x_um),
                left.y_um + fraction * (right.y_um - left.y_um),
            ));
        }
    }
    None
}

fn wrap_angle(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}

/// Small, deterministic PRNG for reproducible research fixtures.
///
/// This is not cryptographic and must never be used for identity, security,
/// tokens, or scientific randomness requiring a cryptographically strong
/// generator. Benchmark manifests must record the seed and crate revision.
#[derive(Debug, Clone, Copy)]
struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        let state = if seed == 0 { DEFAULT_RNG_SEED } else { seed };
        Self { state }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value >> 12;
        value ^= value << 25;
        value ^= value >> 27;
        self.state = value;
        value.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn next_unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64) * (1.0 / ((1_u64 << 53) as f64))
    }

    fn index(&mut self, length: usize) -> usize {
        debug_assert!(length > 0);
        ((self.next_unit_f64() * length as f64) as usize).min(length - 1)
    }

    fn next_standard_normal(&mut self) -> f64 {
        // Box-Muller transform. Clamp only the logarithm input away from zero.
        let u1 = self.next_unit_f64().max(f64::MIN_POSITIVE);
        let u2 = self.next_unit_f64();
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn directional(up: Vec<f64>, cross: Vec<f64>, down: Vec<f64>) -> DirectionalSamples {
        DirectionalSamples {
            up_gradient: SampleSet::new(up),
            cross_gradient: SampleSet::new(cross),
            down_gradient: SampleSet::new(down),
        }
    }

    fn event_record(
        mechanism: EventMechanism,
        duration_bin: EventDurationBin,
        evidence_status: EventEvidenceStatus,
    ) -> EventRecord {
        EventRecord {
            direction_before_run: DirectionClass::UpGradient,
            completed_run_direction: DirectionClass::UpGradient,
            run_displacement_heading_rad: Some(0.0),
            run_displacement_um: 1.0,
            incoming_heading_rad: 0.0,
            run_duration_s: 0.25,
            mechanism,
            evidence_status,
            duration_bin,
            event_duration_s: 0.25,
            turn_angle_rad: 0.0,
            heading_after_event_rad: 0.0,
        }
    }

    fn empirical_time(values: Vec<f64>) -> WaitingTimeLaw {
        WaitingTimeLaw::empirical(values)
    }

    fn directional_times(
        up: Vec<f64>,
        cross: Vec<f64>,
        down: Vec<f64>,
    ) -> DirectionalWaitingTimes {
        DirectionalWaitingTimes {
            up_gradient: empirical_time(up),
            cross_gradient: empirical_time(cross),
            down_gradient: empirical_time(down),
        }
    }

    fn steering_profile() -> PorousMotilityProfile {
        PorousMotilityProfile {
            swimming_speed_um_s: 20.0,
            rotational_diffusion_rad2_s: 0.0,
            integration_step_s: 0.05,
            run_duration_s: directional_times(vec![0.25], vec![0.25], vec![0.25]),
            run_duration_unconditional_s: empirical_time(vec![0.25]),
            mechanism_probabilities: MechanismProbabilities {
                active_motor: 1.0,
                passive_trap: 0.0,
                unresolved: 0.0,
            },
            threshold_boundary_policy: ThresholdBoundaryPolicy::ShortInclusive,
            active_motor_event_duration_s: empirical_time(vec![0.25]),
            passive_trap_event_duration_s: empirical_time(vec![2.0]),
            unresolved_event_duration_s: empirical_time(vec![1.0]),
            active_motor_turn_angle_rad: directional(
                vec![0.0],
                vec![-PI / 2.0, PI / 2.0],
                vec![PI],
            ),
            active_motor_turn_angle_unconditional_rad: SampleSet::new(vec![0.0]),
            passive_trap_turn_angle_rad: SampleSet::new(vec![0.0]),
            unresolved_turn_angle_rad: SampleSet::new(vec![0.0]),
        }
    }

    #[test]
    fn classifies_gradient_direction_by_projection() {
        assert_eq!(classify_direction(0.0, 0.0), Some(DirectionClass::UpGradient));
        assert_eq!(
            classify_direction(PI / 4.0, 0.0),
            Some(DirectionClass::UpGradient)
        );
        assert_eq!(classify_direction(PI, 0.0), Some(DirectionClass::DownGradient));
        assert_eq!(
            classify_direction(PI * 3.0 / 4.0, 0.0),
            Some(DirectionClass::DownGradient)
        );
        assert_eq!(
            classify_direction(PI / 2.0, 0.0),
            Some(DirectionClass::CrossGradient)
        );
        assert_eq!(classify_direction(f64::NAN, 0.0), None);
    }

    #[test]
    fn validates_physical_mechanism_profiles_without_duration_causation() {
        let mut profile = steering_profile();
        profile.active_motor_event_duration_s = empirical_time(vec![0.4, 1.2]);
        profile.passive_trap_event_duration_s = empirical_time(vec![0.2, 2.0]);
        profile.unresolved_event_duration_s = empirical_time(vec![1.0]);
        assert!(profile.validate().is_ok());

        let mut invalid = steering_profile();
        invalid.mechanism_probabilities = MechanismProbabilities {
            active_motor: 0.7,
            passive_trap: 0.7,
            unresolved: 0.0,
        };
        assert_eq!(
            invalid.validate(),
            Err(ProfileError::InvalidMechanismProbabilities)
        );

        let mut invalid = steering_profile();
        invalid.active_motor_event_duration_s = empirical_time(vec![0.0]);
        assert_eq!(
            invalid.validate(),
            Err(ProfileError::InvalidWaitingTimeLaw(
                WaitingTimeLawError::InvalidEmpiricalSample
            ))
        );

        let mut invalid = steering_profile();
        invalid.swimming_speed_um_s = f64::NAN;
        assert_eq!(
            invalid.validate(),
            Err(ProfileError::InvalidSwimmingSpeed)
        );
    }

    #[test]
    fn exact_boundary_policy_classifies_duration_not_mechanism() {
        let short_inclusive = steering_profile();
        assert_eq!(
            short_inclusive.duration_bin(1.0),
            Ok(EventDurationBin::Short)
        );

        let mut long_inclusive = steering_profile();
        long_inclusive.threshold_boundary_policy = ThresholdBoundaryPolicy::LongInclusive;
        assert_eq!(
            long_inclusive.duration_bin(1.0),
            Ok(EventDurationBin::Long)
        );

        let mut excluded = steering_profile();
        excluded.threshold_boundary_policy = ThresholdBoundaryPolicy::ExcludeExactBoundary;
        assert_eq!(
            excluded.duration_bin(1.0),
            Ok(EventDurationBin::ExactBoundaryExcluded)
        );
        assert_eq!(
            excluded.duration_bin(f64::NAN),
            Err(ProfileError::InvalidEventDuration)
        );
    }

    #[test]
    fn an_active_motor_event_can_be_long_and_a_passive_trap_can_be_short() {
        let mut profile = steering_profile();
        profile.mechanism_probabilities = MechanismProbabilities {
            active_motor: 1.0,
            passive_trap: 0.0,
            unresolved: 0.0,
        };
        profile.active_motor_event_duration_s = empirical_time(vec![2.0]);

        let active = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            1,
            7,
        )
        .expect("active event simulation");
        assert_eq!(active.events[0].mechanism, EventMechanism::ActiveMotorEvent);
        assert_eq!(active.events[0].duration_bin, EventDurationBin::Long);
        assert_eq!(active.events[0].evidence_status, EventEvidenceStatus::ModelLatent);
        let active_summary = summarize(&active);
        assert_eq!(
            active_summary.mechanism_duration_contingency.active_motor_long,
            1
        );
        assert_eq!(
            active_summary.mechanism_duration_contingency.active_motor_short,
            0
        );
        assert_eq!(active_summary.evidence_status_counts.model_latent, 1);

        profile.mechanism_probabilities = MechanismProbabilities {
            active_motor: 0.0,
            passive_trap: 1.0,
            unresolved: 0.0,
        };
        profile.passive_trap_event_duration_s = empirical_time(vec![0.25]);

        let passive = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            1,
            7,
        )
        .expect("passive event simulation");
        assert_eq!(
            passive.events[0].mechanism,
            EventMechanism::PassiveMechanicalTrap
        );
        assert_eq!(passive.events[0].duration_bin, EventDurationBin::Short);
        let passive_summary = summarize(&passive);
        assert_eq!(
            passive_summary.mechanism_duration_contingency.passive_trap_short,
            1
        );
        assert_eq!(
            passive_summary.mechanism_duration_contingency.passive_trap_long,
            0
        );
    }

    #[test]
    fn unresolved_mechanism_is_preserved_as_unknown() {
        let mut profile = steering_profile();
        profile.mechanism_probabilities = MechanismProbabilities {
            active_motor: 0.0,
            passive_trap: 0.0,
            unresolved: 1.0,
        };
        let result = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            3,
            31,
        )
        .expect("unresolved event simulation");

        assert!(result.events.iter().all(|event| {
            event.mechanism == EventMechanism::Unresolved
                && event.evidence_status == EventEvidenceStatus::ModelLatent
        }));
        let summary = summarize(&result);
        assert_eq!(summary.unresolved_mechanism_event_count, 3);
        assert_eq!(summary.active_motor_event_count, 0);
        assert_eq!(summary.passive_trap_event_count, 0);
        assert_eq!(
            summary.mechanism_duration_contingency.unresolved_short,
            3
        );
        assert_eq!(summary.evidence_status_counts.model_latent, 3);
    }

    #[test]
    fn seeded_simulation_is_reproducible() {
        let profile = steering_profile();
        let initial = AgentState::new(0.0, 0.0, PI);
        let first = simulate(&profile, initial, 0.0, 100, 7).expect("valid simulation");
        let second = simulate(&profile, initial, 0.0, 100, 7).expect("valid simulation");
        assert_eq!(first, second);
    }

    #[test]
    fn active_turn_kernel_reorients_a_down_gradient_run_toward_the_gradient() {
        let profile = steering_profile();
        let initial = AgentState::new(0.0, 0.0, PI);
        let result = simulate(&profile, initial, 0.0, 8, 42).expect("valid simulation");

        assert!(wrap_angle(result.final_state.heading_rad).abs() < 1.0e-12);
        let summary = summarize(&result);
        assert_eq!(summary.short_duration_event_count, 8);
        assert_eq!(summary.long_duration_event_count, 0);
        assert_eq!(summary.active_motor_event_count, 8);
        assert_eq!(summary.passive_trap_event_count, 0);
        assert_eq!(summary.resolved_run_direction_count, 8);
        assert_eq!(summary.up_gradient_run_count, 7);
        assert_eq!(summary.up_gradient_run_fraction, Some(7.0 / 8.0));
        assert_eq!(summary.short_duration_followed_run_count, 7);
        assert_eq!(summary.short_duration_followed_up_gradient_count, 7);
        assert_eq!(summary.short_duration_followed_up_gradient_fraction, Some(1.0));
        assert_eq!(summary.active_motor_followed_run_count, 7);
        assert_eq!(summary.active_motor_followed_up_gradient_fraction, Some(1.0));
        assert_eq!(summary.passive_trap_followed_up_gradient_fraction, None);
    }

    #[test]
    fn passive_trap_turns_do_not_receive_active_directional_bias() {
        let mut profile = steering_profile();
        profile.mechanism_probabilities = MechanismProbabilities {
            active_motor: 0.0,
            passive_trap: 1.0,
            unresolved: 0.0,
        };

        let initial = AgentState::new(0.0, 0.0, PI);
        let result = simulate(&profile, initial, 0.0, 12, 42).expect("valid simulation");

        assert!((wrap_angle(result.final_state.heading_rad - PI)).abs() < 1.0e-12);
        let summary = summarize(&result);
        assert_eq!(summary.short_duration_event_count, 0);
        assert_eq!(summary.long_duration_event_count, 12);
        assert_eq!(summary.passive_trap_event_count, 12);
        assert_eq!(summary.long_duration_followed_run_count, 11);
        assert_eq!(summary.long_duration_followed_up_gradient_count, 0);
        assert_eq!(summary.long_duration_followed_up_gradient_fraction, Some(0.0));
        assert_eq!(summary.passive_trap_followed_up_gradient_fraction, Some(0.0));
        assert_eq!(summary.active_motor_followed_up_gradient_fraction, None);
    }

    #[test]
    #[test]
    fn reversing_gradient_reverses_the_selected_heading() {
        let profile = steering_profile();
        let initial = AgentState::new(0.0, 0.0, 0.0);

        let rightward =
            simulate(&profile, initial, 0.0, 5, 19).expect("valid rightward run");
        let leftward =
            simulate(&profile, initial, PI, 5, 19).expect("valid leftward run");

        assert!(wrap_angle(rightward.final_state.heading_rad).abs() < 1.0e-12);
        assert!((wrap_angle(leftward.final_state.heading_rad - PI)).abs() < 1.0e-12);
    }

    #[test]
    fn rotational_diffusion_is_seeded_and_changes_heading() {
        let mut profile = steering_profile();
        profile.rotational_diffusion_rad2_s = 0.8;
        profile.active_motor_turn_angle_rad = directional(
            vec![0.0],
            vec![0.0],
            vec![0.0],
        );
        let initial = AgentState::new(0.0, 0.0, 0.0);

        let first = simulate(&profile, initial, 0.0, 40, 1234).expect("valid noisy simulation");
        let replay = simulate(&profile, initial, 0.0, 40, 1234).expect("valid noisy simulation");
        assert_eq!(first, replay);
        assert_ne!(first.final_state.heading_rad, initial.heading_rad);
    }

    #[test]
    fn short_turn_bias_uses_realized_run_displacement_direction() {
        let mut profile = steering_profile();
        profile.rotational_diffusion_rad2_s = 0.8;
        profile.active_motor_turn_angle_rad = directional(
            vec![0.0],
            vec![0.1],
            vec![0.2],
        );
        let result = simulate(
            &profile,
            AgentState::new(0.0, 0.0, PI / 2.0),
            0.0,
            3,
            1,
        )
        .expect("valid simulation");

        let first = result.events[0];
        // The policy starts from a cross-gradient heading, but run-phase
        // rotational diffusion makes the realized net run vector point down.
        assert_eq!(first.direction_before_run, DirectionClass::CrossGradient);
        assert_eq!(first.completed_run_direction, DirectionClass::DownGradient);
        let expected_angle = match first.completed_run_direction {
            DirectionClass::UpGradient => 0.0,
            DirectionClass::CrossGradient => 0.1,
            DirectionClass::DownGradient => 0.2,
        };
        assert_eq!(first.turn_angle_rad, expected_angle);
        assert!(first.incoming_heading_rad.is_finite());
        assert!(first.run_displacement_heading_rad.is_some());
        assert!(first.run_displacement_um.is_finite());
    }

    #[test]
    fn non_advancing_integration_step_fails_closed() {
        let profile = steering_profile();
        let initial = AgentState {
            x_um: 0.0,
            y_um: 0.0,
            heading_rad: 0.0,
            elapsed_s: 1.0e300,
        };
        let result = simulate(&profile, initial, 0.0, 1, 1);
        assert_eq!(result, Err(SimulationError::IntegrationStepDidNotAdvance));
    }

    #[test]
    fn invalid_rotational_diffusion_and_step_are_rejected() {
        let mut profile = steering_profile();
        profile.rotational_diffusion_rad2_s = -0.1;
        assert_eq!(
            profile.validate(),
            Err(ProfileError::InvalidRotationalDiffusion)
        );

        let mut profile = steering_profile();
        profile.integration_step_s = 0.0;
        assert_eq!(
            profile.validate(),
            Err(ProfileError::InvalidIntegrationStep)
        );
    }

    #[test]
    fn four_bias_modes_isolate_temporal_and_angular_mechanisms() {
        let mut profile = steering_profile();
        profile.run_duration_s = directional_times(vec![0.1], vec![0.25], vec![0.4]);
        profile.run_duration_unconditional_s = empirical_time(vec![0.25]);

        let initial = AgentState::new(0.0, 0.0, PI);
        let modes = [
            BiasMode::None,
            BiasMode::RunTimeOnly,
            BiasMode::TurnAngleOnly,
            BiasMode::Combined,
        ];
        let results: Vec<_> = modes
            .into_iter()
            .map(|mode| {
                simulate_with_bias_mode(&profile, initial, 0.0, 2, 77, mode)
                    .expect("valid bias-mode simulation")
            })
            .collect();

        // The first run is down-gradient. Only the run-time-enabled modes use
        // the down-gradient duration sample (0.4 s instead of pooled 0.25 s).
        assert_eq!(results[0].events[0].run_duration_s, 0.25);
        assert_eq!(results[1].events[0].run_duration_s, 0.4);
        assert_eq!(results[2].events[0].run_duration_s, 0.25);
        assert_eq!(results[3].events[0].run_duration_s, 0.4);

        // Only turn-angle-enabled modes use the down-gradient short-turn sample
        // (pi radians); otherwise the pooled short-turn distribution is zero.
        assert_eq!(results[0].events[0].turn_angle_rad, 0.0);
        assert_eq!(results[1].events[0].turn_angle_rad, 0.0);
        assert_eq!(results[2].events[0].turn_angle_rad, PI);
        assert_eq!(results[3].events[0].turn_angle_rad, PI);
    }

    #[test]
    fn time_averaged_msd_matches_straight_run_displacement() {
        let profile = steering_profile();
        let initial = AgentState::new(0.0, 0.0, 0.0);
        let trajectories: Vec<_> = (1..=3)
            .map(|seed| simulate(&profile, initial, 0.0, 2, seed).expect("valid trajectory"))
            .collect();

        let estimates =
            mean_squared_displacement(&trajectories, &[0.25, 0.0], 10.0)
                .expect("valid MSD request");

        assert_eq!(estimates[0].trajectory_count, 3);
        assert!((estimates[0].mean_squared_displacement_um2 - 25.0).abs() < 1.0e-10);
        assert_eq!(estimates[0].between_trajectory_sd_um2, Some(0.0));
        assert_eq!(estimates[1].mean_squared_displacement_um2, 0.0);
    }

    #[test]
    fn ageing_time_changes_the_msd_origin_window() {
        let profile = steering_profile();
        let trajectory = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            4,
            5,
        )
        .expect("valid trajectory");

        let zero_age =
            mean_squared_displacement(&[trajectory.clone()], &[0.25], 0.25)
                .expect("zero-age MSD");
        let aged = mean_squared_displacement_with_age(
            &[trajectory.clone()],
            0.25,
            &[0.25],
            0.25,
        )
        .expect("aged MSD");

        assert!((zero_age[0].mean_squared_displacement_um2 - 12.5).abs() < 1.0e-10);
        assert!(
            (aged[0].mean_squared_displacement_um2 - (75.0 / 7.0)).abs()
                < 1.0e-10
        );
        assert_ne!(
            zero_age[0].mean_squared_displacement_um2,
            aged[0].mean_squared_displacement_um2
        );
    }

    #[test]
    fn msd_ageing_time_must_be_finite_and_non_negative() {
        let trajectory = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            2,
            5,
        )
        .expect("valid trajectory");
        assert_eq!(
            mean_squared_displacement_with_age(&[trajectory.clone()], -0.1, &[0.1], 0.1),
            Err(MsdError::InvalidAgeingTime)
        );
        assert_eq!(
            mean_squared_displacement_with_age(
                &[trajectory.clone()],
                f64::INFINITY,
                &[0.1],
                0.1
            ),
            Err(MsdError::InvalidAgeingTime)
        );
        assert_eq!(
            mean_squared_displacement_with_age(&[trajectory], 100.0, &[0.1], 0.1),
            Err(MsdError::NoTrajectorySupportsLag(0.1))
        );
    }

    #[test]
    fn msd_rejects_invalid_lags_and_empty_ensembles() {
        assert_eq!(
            mean_squared_displacement(&[], &[0.1], 0.1),
            Err(MsdError::EmptyEnsemble)
        );

        let trajectory = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            2,
            5,
        )
        .expect("valid trajectory");

        assert_eq!(
            mean_squared_displacement(&[trajectory.clone()], &[-0.1], 0.1),
            Err(MsdError::InvalidLagTime)
        );
        assert_eq!(
            mean_squared_displacement(&[trajectory], &[0.1], 0.0),
            Err(MsdError::InvalidTimeOriginStep)
        );
    }

    #[test]
    fn msd_reports_when_no_trajectory_covers_the_requested_lag() {
        let trajectory = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            1,
            5,
        )
        .expect("valid trajectory");
        assert_eq!(
            mean_squared_displacement(&[trajectory], &[100.0], 0.1),
            Err(MsdError::NoTrajectorySupportsLag(100.0))
        );
    }

    #[test]
    fn duration_mechanism_diagnostic_uses_only_evidence_qualified_events() {
        let events = [
            event_record(
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::Short,
                EventEvidenceStatus::DirectlyObserved,
            ),
            event_record(
                EventMechanism::PassiveMechanicalTrap,
                EventDurationBin::Short,
                EventEvidenceStatus::DirectlyObserved,
            ),
            event_record(
                EventMechanism::PassiveMechanicalTrap,
                EventDurationBin::Long,
                EventEvidenceStatus::ModelLatent,
            ),
            event_record(
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::Long,
                EventEvidenceStatus::DirectlyObserved,
            ),
            event_record(
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::Short,
                EventEvidenceStatus::AlgorithmInferred,
            ),
            event_record(
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::ExactBoundaryExcluded,
                EventEvidenceStatus::ModelLatent,
            ),
            event_record(
                EventMechanism::Unresolved,
                EventDurationBin::Long,
                EventEvidenceStatus::Unresolved,
            ),
        ];

        let diagnostic = duration_mechanism_diagnostic(&events);
        assert_eq!(diagnostic.true_positive_active_short, 1);
        assert_eq!(diagnostic.false_positive_passive_short, 1);
        assert_eq!(diagnostic.true_negative_passive_long, 1);
        assert_eq!(diagnostic.false_negative_active_long, 1);
        assert_eq!(diagnostic.excluded_algorithm_inferred, 1);
        assert_eq!(diagnostic.excluded_unresolved, 1);
        assert_eq!(diagnostic.excluded_exact_boundary, 1);
        assert_eq!(diagnostic.known_event_count, 4);
        assert_eq!(diagnostic.sensitivity, Some(0.5));
        assert_eq!(diagnostic.specificity, Some(0.5));
        assert_eq!(diagnostic.positive_predictive_value, Some(0.5));
        assert_eq!(diagnostic.negative_predictive_value, Some(0.5));
        assert_eq!(diagnostic.balanced_accuracy, Some(0.5));
    }

    #[test]
    fn duration_mechanism_diagnostic_returns_unknown_when_no_truth_is_admitted() {
        let events = [
            event_record(
                EventMechanism::ActiveMotorEvent,
                EventDurationBin::Short,
                EventEvidenceStatus::AlgorithmInferred,
            ),
            event_record(
                EventMechanism::Unresolved,
                EventDurationBin::Long,
                EventEvidenceStatus::Unresolved,
            ),
        ];
        let diagnostic = duration_mechanism_diagnostic(&events);
        assert_eq!(diagnostic.known_event_count, 0);
        assert_eq!(diagnostic.sensitivity, None);
        assert_eq!(diagnostic.specificity, None);
        assert_eq!(diagnostic.balanced_accuracy, None);
    }

    #[test]
    fn first_fourier_harmonic_matches_analytic_orientation_fixtures() {
        let aligned = summarize_relative_headings(&[0.0, 0.0, 0.0])
            .expect("non-empty orientations");
        assert_eq!(aligned.up_gradient_fraction, 1.0);
        assert!((aligned.first_harmonic_magnitude - 1.0).abs() < 1.0e-12);
        assert_eq!(aligned.first_harmonic_phase_rad, Some(0.0));

        let balanced = summarize_relative_headings(&[
            0.0,
            PI / 2.0,
            PI,
            -PI / 2.0,
        ])
        .expect("balanced orientation set");
        assert_eq!(balanced.up_gradient_fraction, 0.25);
        assert!((balanced.first_harmonic_magnitude) < 1.0e-12);
        assert_eq!(balanced.first_harmonic_phase_rad, None);
    }

    #[test]
    fn first_fourier_harmonic_is_rotation_invariant_in_magnitude() {
        let original = summarize_relative_headings(&[0.1, 0.2, 0.4, -0.2])
            .expect("original angles");
        let rotation = 1.13;
        let rotated_angles: Vec<_> = [0.1, 0.2, 0.4, -0.2]
            .into_iter()
            .map(|angle| wrap_angle(angle + rotation))
            .collect();
        let rotated = summarize_relative_headings(&rotated_angles)
            .expect("rotated angles");

        assert!(
            (original.first_harmonic_magnitude - rotated.first_harmonic_magnitude).abs()
                < 1.0e-12
        );
        let phase_difference = wrap_angle(
            rotated.first_harmonic_phase_rad.expect("non-zero harmonic")
                - original.first_harmonic_phase_rad.expect("non-zero harmonic"),
        );
        assert!((phase_difference - rotation).abs() < 1.0e-12);
    }

    #[test]
    fn cluster_bootstrap_is_deterministic_and_resamples_independent_units() {
        let clusters = vec![
            vec![0.0, 0.05, 0.1],
            vec![0.0, 0.2],
            vec![PI, PI - 0.1],
            vec![0.1, 0.15, 0.2, 0.25],
        ];
        let first = cluster_bootstrap_orientations(&clusters, 500, 0.95, 42)
            .expect("valid cluster bootstrap");
        let replay = cluster_bootstrap_orientations(&clusters, 500, 0.95, 42)
            .expect("deterministic replay");

        assert_eq!(first, replay);
        assert_eq!(first.independent_cluster_count, 4);
        assert_eq!(first.resample_count, 500);
        assert!(first.up_gradient_fraction.lower <= first.up_gradient_fraction.upper);
        assert!(
            first.first_harmonic_magnitude.lower
                <= first.first_harmonic_magnitude.upper
        );
    }

    #[test]
    fn cluster_bootstrap_rejects_invalid_cluster_structure() {
        assert_eq!(
            cluster_bootstrap_orientations(&[], 100, 0.95, 1),
            Err(BootstrapError::NoClusters)
        );
        assert_eq!(
            cluster_bootstrap_orientations(&[vec![0.0]], 100, 0.95, 1),
            Err(BootstrapError::InsufficientIndependentClusters)
        );
        assert_eq!(
            cluster_bootstrap_orientations(&[vec![], vec![0.0]], 100, 0.95, 1),
            Err(BootstrapError::EmptyCluster)
        );
        assert_eq!(
            cluster_bootstrap_orientations(&[vec![f64::NAN], vec![0.0]], 100, 0.95, 1),
            Err(BootstrapError::NonFiniteAngle)
        );
        assert_eq!(
            cluster_bootstrap_orientations(&[vec![0.0], vec![0.0]], 0, 0.95, 1),
            Err(BootstrapError::ZeroResamples)
        );
        assert_eq!(
            cluster_bootstrap_orientations(&[vec![0.0], vec![0.0]], 100, 1.0, 1),
            Err(BootstrapError::InvalidConfidenceLevel)
        );
    }

    #[test]
    fn simulator_accepts_double_exponential_and_piecewise_power_law_time_laws() {
        let mut profile = steering_profile();
        profile.run_duration_s = DirectionalWaitingTimes {
            up_gradient: WaitingTimeLaw::DoubleExponential {
                fast_weight: 0.7,
                fast_mean_s: 0.15,
                slow_mean_s: 0.45,
            },
            cross_gradient: WaitingTimeLaw::Exponential { mean_s: 0.25 },
            down_gradient: WaitingTimeLaw::Exponential { mean_s: 0.35 },
        };
        profile.run_duration_unconditional_s = WaitingTimeLaw::DoubleExponential {
            fast_weight: 0.5,
            fast_mean_s: 0.2,
            slow_mean_s: 0.4,
        };
        profile.active_motor_event_duration_s = WaitingTimeLaw::PiecewisePowerLaw {
            alpha: 0.79,
            beta: 2.63,
            min_s: 0.05,
            crossover_s: 2.0,
        };

        profile.validate().expect("parametric renewal profile is valid");
        let result = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            100,
            77,
        )
        .expect("parametric renewal simulation");

        assert_eq!(result.events.len(), 100);
        assert!(result.events.iter().all(|event| {
            event.run_duration_s > 0.0
                && event.event_duration_s >= 0.05
                && event.event_duration_s.is_finite()
        }));
        let replay = simulate(
            &profile,
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            100,
            77,
        )
        .expect("deterministic parametric renewal replay");
        assert_eq!(result, replay);
    }

    #[test]
    fn no_event_summary_is_explicitly_undefined_not_zero() {
        let result = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            0,
            1,
        )
        .expect("zero-event simulation is valid");
        let summary = summarize(&result);

        assert_eq!(summary.event_count, 0);
        assert_eq!(summary.mean_run_duration_s, None);
        assert_eq!(summary.projected_drift_um_s, None);
        assert_eq!(summary.resolved_run_direction_count, 0);
        assert_eq!(summary.up_gradient_run_fraction, None);
        assert_eq!(summary.short_duration_followed_up_gradient_fraction, None);
        assert_eq!(summary.long_duration_followed_up_gradient_fraction, None);

        let one_event = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            1,
            2,
        )
        .expect("one-event simulation is valid");
        let one_event_summary = summarize(&one_event);
        assert_eq!(one_event_summary.short_duration_event_count, 1);
        assert_eq!(one_event_summary.short_duration_followed_run_count, 0);
        assert_eq!(
            one_event_summary.short_duration_followed_up_gradient_fraction,
            None
        );
    }
}
