// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! CPU-first reference models for biological motility research.
//!
//! This crate starts with an event-based model for bacterial movement in porous
//! media. It deliberately separates run-time bias, short reorientation events,
//! and long mechanical-trap events. It is a research reference, not a validated
//! flagellar motor or pore-resolved hydrodynamics solver.
//!
//! The event classes are operational: the one-second boundary follows the
//! analysis convention in Beier et al. (2026). A short event is not, by itself,
//! proof that a motor mechanism caused it; that interpretation needs source
//! data and independent biological evidence.

#![forbid(unsafe_code)]

use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

pub const EVENT_CLASS_THRESHOLD_S: f64 = 1.0;
pub const DIRECTION_CONE_HALF_ANGLE_DEG: f64 = 36.0;
const DEFAULT_RNG_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// Direction of a run relative to the chemoattractant gradient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionClass {
    UpGradient,
    CrossGradient,
    DownGradient,
}

/// Operational class of a reorientation event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventClass {
    /// Event duration is at most EVENT_CLASS_THRESHOLD_S.
    ShortReorientation,
    /// Event duration is greater than EVENT_CLASS_THRESHOLD_S.
    LongMechanicalTrap,
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

/// Empirical samples indexed by the direction of the preceding run.
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
/// Run durations and short-turn angles are conditioned on the direction of
/// the preceding run. Separate unconditional empirical distributions enable
/// matched no-bias, run-time-only, turn-angle-only and combined ablations.
/// Long mechanical-trap angles are intentionally pooled.
#[derive(Debug, Clone, PartialEq)]
pub struct PorousMotilityProfile {
    /// Swimming speed in micrometres per second.
    pub swimming_speed_um_s: f64,
    /// Rotational diffusion coefficient during runs, in radians squared per second.
    /// Set to zero for the straight-run ablation.
    pub rotational_diffusion_rad2_s: f64,
    /// Maximum Euler-Maruyama integration step during a run, in seconds.
    pub integration_step_s: f64,
    /// Empirical run-duration samples by preceding-run direction, in seconds.
    pub run_duration_s: DirectionalSamples,
    /// Pooled run-duration samples used by ablations with run-time bias disabled.
    pub run_duration_unconditional_s: SampleSet,
    /// Fraction of interruptions represented by short reorientation events.
    /// Calibrate this from admitted event observations; do not infer it from
    /// the published paper's headline values.
    pub short_event_probability: f64,
    /// Duration samples for short events (all must be <= 1 second).
    pub short_event_duration_s: SampleSet,
    /// Duration samples for long mechanical traps (all must be > 1 second).
    pub long_trap_duration_s: SampleSet,
    /// Signed heading changes in radians for short events, conditional on the
    /// direction of the run immediately before the event.
    pub short_turn_angle_rad: DirectionalSamples,
    /// Pooled short-event turn-angle samples used when turn-angle bias is disabled.
    pub short_turn_angle_unconditional_rad: SampleSet,
    /// Signed heading changes in radians for long traps, not direction-biased
    /// by this model.
    pub long_trap_turn_angle_rad: SampleSet,
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
        if !self.short_event_probability.is_finite()
            || !(0.0..=1.0).contains(&self.short_event_probability)
        {
            return Err(ProfileError::InvalidShortEventProbability);
        }

        self.run_duration_s.validate_positive("run_duration_s")?;
        self.run_duration_unconditional_s
            .validate_positive("run_duration_unconditional_s")?;
        self.short_event_duration_s
            .validate_positive("short_event_duration_s")?;
        self.long_trap_duration_s
            .validate_positive("long_trap_duration_s")?;
        self.short_turn_angle_rad
            .validate_finite("short_turn_angle_rad")?;
        self.short_turn_angle_unconditional_rad
            .validate_finite("short_turn_angle_unconditional_rad")?;
        self.long_trap_turn_angle_rad
            .validate_finite("long_trap_turn_angle_rad")?;

        if self
            .short_event_duration_s
            .values()
            .iter()
            .any(|duration| *duration > EVENT_CLASS_THRESHOLD_S)
        {
            return Err(ProfileError::ShortEventExceedsThreshold);
        }
        if self
            .long_trap_duration_s
            .values()
            .iter()
            .any(|duration| *duration <= EVENT_CLASS_THRESHOLD_S)
        {
            return Err(ProfileError::LongTrapDoesNotExceedThreshold);
        }
        Ok(())
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
    InvalidShortEventProbability,
    ShortEventExceedsThreshold,
    LongTrapDoesNotExceedThreshold,
}

impl fmt::Display for ProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySamples(name) => write!(formatter, "{name} must contain at least one sample"),
            Self::NonFiniteSample(name) => write!(formatter, "{name} contains a non-finite sample"),
            Self::NonPositiveSample(name) => write!(formatter, "{name} must contain positive samples"),
            Self::InvalidSwimmingSpeed => write!(formatter, "swimming_speed_um_s must be finite and positive"),
            Self::InvalidRotationalDiffusion => write!(formatter, "rotational_diffusion_rad2_s must be finite and non-negative"),
            Self::InvalidIntegrationStep => write!(formatter, "integration_step_s must be finite and positive"),
            Self::InvalidShortEventProbability => write!(formatter, "short_event_probability must be in [0, 1]"),
            Self::ShortEventExceedsThreshold => write!(formatter, "short-event duration exceeds the one-second class threshold"),
            Self::LongTrapDoesNotExceedThreshold => write!(formatter, "long-trap duration must exceed the one-second class threshold"),
        }
    }
}

impl Error for ProfileError {}

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
    pub direction_before_run: DirectionClass,
    pub run_duration_s: f64,
    pub event_class: EventClass,
    pub event_duration_s: f64,
    pub turn_angle_rad: f64,
    pub heading_after_event_rad: f64,
}

/// Result of a deterministic, seeded event-based simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct SimulationResult {
    pub initial_state: AgentState,
    pub final_state: AgentState,
    pub gradient_heading_rad: f64,
    pub events: Vec<EventRecord>,
}

/// Summary measures calculated from a simulated trajectory.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BehaviorSummary {
    pub event_count: usize,
    pub short_event_count: usize,
    pub long_trap_count: usize,
    /// Number of short events with an observed following run.
    pub short_event_followed_run_count: usize,
    /// Number of those following runs initially oriented up-gradient.
    pub short_event_followed_up_gradient_count: usize,
    /// Conditional fraction among observed runs following short events.
    pub short_event_followed_up_gradient_fraction: Option<f64>,
    /// Number of long traps with an observed following run.
    pub long_trap_followed_run_count: usize,
    /// Number of those following runs initially oriented up-gradient.
    pub long_trap_followed_up_gradient_count: usize,
    /// Conditional fraction among observed runs following long traps.
    pub long_trap_followed_up_gradient_fraction: Option<f64>,
    pub mean_run_duration_s: Option<f64>,
    pub projected_drift_um_s: Option<f64>,
}

/// Errors returned before or during simulation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimulationError {
    InvalidProfile(ProfileError),
    InvalidInitialState,
    InvalidGradientHeading,
    NonFiniteTrajectory,
}

impl fmt::Display for SimulationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidProfile(error) => write!(formatter, "invalid motility profile: {error}"),
            Self::InvalidInitialState => write!(formatter, "initial agent state must be finite with non-negative elapsed time"),
            Self::InvalidGradientHeading => write!(formatter, "gradient heading must be finite"),
            Self::NonFiniteTrajectory => write!(formatter, "simulation produced a non-finite state; check profile magnitudes"),
        }
    }
}

impl Error for SimulationError {}

impl From<ProfileError> for SimulationError {
    fn from(value: ProfileError) -> Self {
        Self::InvalidProfile(value)
    }
}

/// Classify a heading relative to the gradient using a symmetric cone.
///
/// The default cone is 36 degrees, matching the directional grouping used in
/// the 2026 P. putida porous-agar analysis. The transverse class includes all
/// directions outside the up/down cones.
pub fn classify_direction(
    heading_rad: f64,
    gradient_heading_rad: f64,
    half_angle_deg: f64,
) -> Option<DirectionClass> {
    if !heading_rad.is_finite()
        || !gradient_heading_rad.is_finite()
        || !half_angle_deg.is_finite()
        || !(0.0..90.0).contains(&half_angle_deg)
    {
        return None;
    }

    let relative = wrap_angle(heading_rad - gradient_heading_rad).abs();
    let half_angle = half_angle_deg.to_radians();

    if relative <= half_angle {
        Some(DirectionClass::UpGradient)
    } else if relative >= PI - half_angle {
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
    let mut state = initial_state;
    let mut events = Vec::with_capacity(event_count);

    for _ in 0..event_count {
        let direction = classify_direction(
            state.heading_rad,
            gradient_heading_rad,
            DIRECTION_CONE_HALF_ANGLE_DEG,
        )
        .expect("validated finite headings produce a direction class");

        let run_duration = if bias_mode.uses_run_time_bias() {
            profile.run_duration_s.sample(direction, &mut rng)
        } else {
            profile.run_duration_unconditional_s.sample(&mut rng)
        };
        let mut remaining_run_s = run_duration;
        while remaining_run_s > 0.0 {
            let step_s = remaining_run_s.min(profile.integration_step_s);
            let angular_std = (2.0 * profile.rotational_diffusion_rad2_s * step_s).sqrt();
            let angular_increment = angular_std * rng.next_standard_normal();
            let midpoint_heading = wrap_angle(state.heading_rad + 0.5 * angular_increment);
            let distance = profile.swimming_speed_um_s * step_s;
            state.x_um += distance * midpoint_heading.cos();
            state.y_um += distance * midpoint_heading.sin();
            state.heading_rad = wrap_angle(state.heading_rad + angular_increment);
            state.elapsed_s += step_s;
            if !state.is_valid() {
                return Err(SimulationError::NonFiniteTrajectory);
            }
            remaining_run_s -= step_s;
        }

        let (event_class, event_duration, turn_angle) =
            if rng.next_unit_f64() < profile.short_event_probability {
                (
                    EventClass::ShortReorientation,
                    profile.short_event_duration_s.sample(&mut rng),
                    if bias_mode.uses_turn_angle_bias() {
                        profile.short_turn_angle_rad.sample(direction, &mut rng)
                    } else {
                        profile.short_turn_angle_unconditional_rad.sample(&mut rng)
                    },
                )
            } else {
                (
                    EventClass::LongMechanicalTrap,
                    profile.long_trap_duration_s.sample(&mut rng),
                    profile.long_trap_turn_angle_rad.sample(&mut rng),
                )
            };

        state.heading_rad = wrap_angle(state.heading_rad + wrap_angle(turn_angle));
        state.elapsed_s += event_duration;
        if !state.is_valid() {
            return Err(SimulationError::NonFiniteTrajectory);
        }

        events.push(EventRecord {
            direction_before_run: direction,
            run_duration_s: run_duration,
            event_class,
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
    })
}

/// Compute dimensionally explicit summary measures for a simulated result.
pub fn summarize(result: &SimulationResult) -> BehaviorSummary {
    if result.events.is_empty() {
        return BehaviorSummary {
            event_count: 0,
            short_event_count: 0,
            long_trap_count: 0,
            short_event_followed_run_count: 0,
            short_event_followed_up_gradient_count: 0,
            short_event_followed_up_gradient_fraction: None,
            long_trap_followed_run_count: 0,
            long_trap_followed_up_gradient_count: 0,
            long_trap_followed_up_gradient_fraction: None,
            mean_run_duration_s: None,
            projected_drift_um_s: None,
        };
    }

    let short_event_count = result
        .events
        .iter()
        .filter(|event| event.event_class == EventClass::ShortReorientation)
        .count();
    let long_trap_count = result.events.len() - short_event_count;
    // An event in record i precedes the run in record i+1. The final event
    // has no observed following run and is deliberately excluded from these
    // conditional fractions. This aligns the denominator with "runs after
    // short/long events" rather than classifying the run before the event.
    let mut short_event_followed_run_count = 0;
    let mut short_event_followed_up_gradient_count = 0;
    let mut long_trap_followed_run_count = 0;
    let mut long_trap_followed_up_gradient_count = 0;

    for pair in result.events.windows(2) {
        let event = pair[0];
        let following_run = pair[1];
        let is_up_gradient =
            following_run.direction_before_run == DirectionClass::UpGradient;

        match event.event_class {
            EventClass::ShortReorientation => {
                short_event_followed_run_count += 1;
                if is_up_gradient {
                    short_event_followed_up_gradient_count += 1;
                }
            }
            EventClass::LongMechanicalTrap => {
                long_trap_followed_run_count += 1;
                if is_up_gradient {
                    long_trap_followed_up_gradient_count += 1;
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
        short_event_count,
        long_trap_count,
        short_event_followed_run_count,
        short_event_followed_up_gradient_count,
        short_event_followed_up_gradient_fraction: (short_event_followed_run_count > 0)
            .then_some(
                short_event_followed_up_gradient_count as f64
                    / short_event_followed_run_count as f64,
            ),
        long_trap_followed_run_count,
        long_trap_followed_up_gradient_count,
        long_trap_followed_up_gradient_fraction: (long_trap_followed_run_count > 0)
            .then_some(
                long_trap_followed_up_gradient_count as f64
                    / long_trap_followed_run_count as f64,
            ),
        mean_run_duration_s: Some(total_run_duration / result.events.len() as f64),
        projected_drift_um_s: (elapsed > 0.0).then_some(projected_displacement / elapsed),
    }
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

    fn steering_profile() -> PorousMotilityProfile {
        PorousMotilityProfile {
            swimming_speed_um_s: 20.0,
            rotational_diffusion_rad2_s: 0.0,
            integration_step_s: 0.05,
            run_duration_s: directional(vec![0.25], vec![0.25], vec![0.25]),
            run_duration_unconditional_s: SampleSet::new(vec![0.25]),
            short_event_probability: 1.0,
            short_event_duration_s: SampleSet::new(vec![0.25]),
            long_trap_duration_s: SampleSet::new(vec![2.0]),
            short_turn_angle_rad: directional(
                vec![0.0],
                vec![-PI / 2.0, PI / 2.0],
                vec![PI],
            ),
            short_turn_angle_unconditional_rad: SampleSet::new(vec![0.0]),
            long_trap_turn_angle_rad: SampleSet::new(vec![0.0]),
        }
    }

    #[test]
    fn classifies_directions_with_the_paper_cone() {
        assert_eq!(
            classify_direction(0.0, 0.0, DIRECTION_CONE_HALF_ANGLE_DEG),
            Some(DirectionClass::UpGradient)
        );
        assert_eq!(
            classify_direction(PI, 0.0, DIRECTION_CONE_HALF_ANGLE_DEG),
            Some(DirectionClass::DownGradient)
        );
        assert_eq!(
            classify_direction(PI / 2.0, 0.0, DIRECTION_CONE_HALF_ANGLE_DEG),
            Some(DirectionClass::CrossGradient)
        );
        assert_eq!(
            classify_direction(0.0, 0.0, 90.0),
            None
        );
    }

    #[test]
    fn rejects_non_finite_and_invalid_event_samples() {
        let mut profile = steering_profile();
        profile.short_event_duration_s = SampleSet::new(vec![0.4, 1.2]);
        assert_eq!(
            profile.validate(),
            Err(ProfileError::ShortEventExceedsThreshold)
        );

        let mut profile = steering_profile();
        profile.long_trap_duration_s = SampleSet::new(vec![0.8]);
        assert_eq!(
            profile.validate(),
            Err(ProfileError::LongTrapDoesNotExceedThreshold)
        );

        let mut profile = steering_profile();
        profile.swimming_speed_um_s = f64::NAN;
        assert_eq!(
            profile.validate(),
            Err(ProfileError::InvalidSwimmingSpeed)
        );
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

        assert!((wrap_angle(result.final_state.heading_rad)).abs() < 1.0e-12);
        let summary = summarize(&result);
        assert_eq!(summary.short_event_count, 8);
        assert_eq!(summary.long_trap_count, 0);
        assert_eq!(summary.short_event_followed_run_count, 7);
        assert_eq!(summary.short_event_followed_up_gradient_count, 7);
        assert_eq!(
            summary.short_event_followed_up_gradient_fraction,
            Some(1.0)
        );
        assert_eq!(summary.long_trap_followed_up_gradient_fraction, None);
    }

    #[test]
    fn passive_trap_kernel_does_not_inherit_active_directional_turn_bias() {
        let mut profile = steering_profile();
        profile.short_event_probability = 0.0;

        let initial = AgentState::new(0.0, 0.0, PI);
        let result = simulate(&profile, initial, 0.0, 12, 42).expect("valid simulation");

        assert!((wrap_angle(result.final_state.heading_rad - PI)).abs() < 1.0e-12);
        let summary = summarize(&result);
        assert_eq!(summary.short_event_count, 0);
        assert_eq!(summary.long_trap_count, 12);
        assert_eq!(summary.long_trap_followed_run_count, 11);
        assert_eq!(summary.long_trap_followed_up_gradient_count, 0);
        assert_eq!(
            summary.long_trap_followed_up_gradient_fraction,
            Some(0.0)
        );
        assert_eq!(summary.short_event_followed_up_gradient_fraction, None);
    }

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
        profile.short_turn_angle_rad = directional(
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
        profile.run_duration_s = directional(vec![0.1], vec![0.25], vec![0.4]);
        profile.run_duration_unconditional_s = SampleSet::new(vec![0.25]);

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
        assert_eq!(summary.short_event_followed_up_gradient_fraction, None);
        assert_eq!(summary.long_trap_followed_up_gradient_fraction, None);

        let one_event = simulate(
            &steering_profile(),
            AgentState::new(0.0, 0.0, 0.0),
            0.0,
            1,
            2,
        )
        .expect("one-event simulation is valid");
        let one_event_summary = summarize(&one_event);
        assert_eq!(one_event_summary.short_event_count, 1);
        assert_eq!(one_event_summary.short_event_followed_run_count, 0);
        assert_eq!(
            one_event_summary.short_event_followed_up_gradient_fraction,
            None
        );
    }
}
