//! Deterministic six-leg distributed phase-network benchmark.
//!
//! This crate models phase-only coordination hypotheses. It does not model
//! joint mechanics, muscles, ground contact, neural anatomy or locomotion
//! performance. All returned traces are synthetic kinematic phase signals.

use std::collections::BTreeMap;

pub const LEG_COUNT: usize = 6;
pub const LEGS: [Leg; LEG_COUNT] = [
    Leg::LeftFront,
    Leg::LeftMiddle,
    Leg::LeftRear,
    Leg::RightFront,
    Leg::RightMiddle,
    Leg::RightRear,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Leg {
    LeftFront,
    LeftMiddle,
    LeftRear,
    RightFront,
    RightMiddle,
    RightRear,
}

impl Leg {
    pub const fn index(self) -> usize {
        match self {
            Self::LeftFront => 0,
            Self::LeftMiddle => 1,
            Self::LeftRear => 2,
            Self::RightFront => 3,
            Self::RightMiddle => 4,
            Self::RightRear => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationModel {
    /// Six independent local phase processes.
    Independent,
    /// Natural phase advance plus pairwise preferred-offset coupling.
    CentrallyCoupled,
    /// Phase follows an external sensory/contact rhythm; no autonomous phase
    /// advance remains when the feedback channel is removed.
    FeedbackDriven,
    /// Local phase advance, central coupling and sensory phase correction.
    Hybrid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SixLegConfig {
    /// Angular frequency of each local phase process, radians per time unit.
    pub natural_angular_frequencies: [f64; LEG_COUNT],
    /// Initial unwrapped phase for each leg, radians.
    pub initial_phases: [f64; LEG_COUNT],
    /// Directed coupling gains K[i][j]. Use symmetric entries for reciprocal
    /// coupling. Diagonal self-coupling is ignored and must be zero.
    pub coupling_gains: [[f64; LEG_COUNT]; LEG_COUNT],
    /// Target phase offset for each leg relative to the common phase.
    pub preferred_phase_offsets: [f64; LEG_COUNT],
    /// Proprioceptive/contact phase-attractor gains, inverse time units.
    pub feedback_gains: [f64; LEG_COUNT],
    /// Frequency of the external sensory/contact phase reference.
    pub drive_angular_frequency: f64,
    /// Initial phase of the external drive, radians.
    pub drive_initial_phase: f64,
    /// Fixed RK4 step in the declared time unit.
    pub time_step: f64,
    /// Number of samples including the initial state.
    pub sample_count: usize,
}

impl SixLegConfig {
    pub fn tripod() -> [f64; LEG_COUNT] {
        [
            0.0,
            std::f64::consts::PI,
            0.0,
            std::f64::consts::PI,
            std::f64::consts::PI,
            0.0,
        ]
    }

    pub fn validate(self) -> Result<(), CoordinationError> {
        if self.sample_count < 2 {
            return Err(CoordinationError::InvalidSampleCount);
        }
        if !self.time_step.is_finite() || self.time_step <= 0.0 {
            return Err(CoordinationError::InvalidTimeStep);
        }
        if !self.drive_angular_frequency.is_finite() {
            return Err(CoordinationError::InvalidDriveFrequency);
        }
        if !self.drive_initial_phase.is_finite() {
            return Err(CoordinationError::InvalidDrivePhase);
        }
        if !self
            .natural_angular_frequencies
            .iter()
            .all(|value| value.is_finite())
        {
            return Err(CoordinationError::InvalidNaturalFrequency);
        }
        if !self.initial_phases.iter().all(|value| value.is_finite())
            || !self
                .preferred_phase_offsets
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(CoordinationError::InvalidPhase);
        }
        if !self
            .feedback_gains
            .iter()
            .all(|value| value.is_finite() && *value >= 0.0)
        {
            return Err(CoordinationError::InvalidFeedbackGain);
        }
        for row in 0..LEG_COUNT {
            for column in 0..LEG_COUNT {
                let gain = self.coupling_gains[row][column];
                if !gain.is_finite() || gain < 0.0 {
                    return Err(CoordinationError::InvalidCouplingGain);
                }
                if row == column && gain != 0.0 {
                    return Err(CoordinationError::SelfCouplingNotAllowed);
                }
            }
        }
        let duration = (self.sample_count - 1) as f64 * self.time_step;
        if !duration.is_finite()
            || !self
                .drive_angular_frequency
                .mul_add(duration, self.drive_initial_phase)
                .is_finite()
        {
            return Err(CoordinationError::NonFiniteTimebase);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intervention {
    /// Add an instantaneous phase perturbation to one leg at the given sample.
    PhasePulse {
        sample_index: usize,
        leg: Leg,
        delta_phase_radians: f64,
    },
    /// Remove feedback from one leg, or every leg when leg is None.
    FeedbackCut {
        sample_index: usize,
        leg: Option<Leg>,
    },
    /// Remove a directed coupling edge. The reciprocal edge can be removed too.
    CouplingEdgeCut {
        sample_index: usize,
        source: Leg,
        target: Leg,
        reciprocal: bool,
    },
    /// Freeze one leg's phase and remove it from all coupling sums.
    DisableLeg {
        sample_index: usize,
        leg: Leg,
    },
}

impl Intervention {
    fn sample_index(self) -> usize {
        match self {
            Self::PhasePulse { sample_index, .. }
            | Self::FeedbackCut { sample_index, .. }
            | Self::CouplingEdgeCut { sample_index, .. }
            | Self::DisableLeg { sample_index, .. } => sample_index,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LegPhaseSample {
    pub timestamp: f64,
    /// Unwrapped phase in canonical LEGS order. A disabled leg retains its
    /// last phase rather than being replaced with a fabricated zero.
    pub phases: [f64; LEG_COUNT],
    pub enabled: [bool; LEG_COUNT],
    pub feedback_enabled: [bool; LEG_COUNT],
}

impl LegPhaseSample {
    pub fn phase(&self, leg: Leg) -> f64 {
        self.phases[leg.index()]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SixLegTrace {
    pub config: SixLegConfig,
    pub model: CoordinationModel,
    pub interventions: Vec<Intervention>,
    pub samples: Vec<LegPhaseSample>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationError {
    InvalidSampleCount,
    InvalidTimeStep,
    InvalidDriveFrequency,
    InvalidDrivePhase,
    InvalidNaturalFrequency,
    InvalidPhase,
    InvalidFeedbackGain,
    InvalidCouplingGain,
    SelfCouplingNotAllowed,
    NonFiniteTimebase,
    InvalidInterventionIndex,
    InvalidPhasePulse,
    DuplicateInterventionSample,
    SelfCouplingCut,
    CouplingEdgeAbsent,
    InsufficientActiveLegs,
    NonFiniteState,
    InvalidCoordinationThreshold,
    InvalidRecoveryWindow,
    InvalidTraceLength,
    NonMonotonicTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinationMetrics {
    /// RMS of wrapped pairwise phase-offset errors across all active leg pairs.
    pub pairwise_phase_error_radians: f64,
    /// Magnitude of the mean target-aligned phase vector; 1 indicates perfect
    /// relative coordination, regardless of common phase.
    pub target_aligned_order_parameter: f64,
    /// Largest absolute wrapped pairwise phase-offset error.
    pub maximum_pairwise_phase_error_radians: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RecoveryResult {
    pub recovered_at_sample: Option<usize>,
    pub recovered_at_time: Option<f64>,
    pub final_error_radians: f64,
}

pub fn simulate(
    config: SixLegConfig,
    model: CoordinationModel,
    interventions: &[Intervention],
) -> Result<SixLegTrace, CoordinationError> {
    config.validate()?;
    let action_schedule = validate_interventions(interventions, config)?;

    let mut state = NetworkState {
        phases: config.initial_phases,
        enabled: [true; LEG_COUNT],
        feedback_enabled: [true; LEG_COUNT],
        coupling_enabled: [[true; LEG_COUNT]; LEG_COUNT],
    };
    let mut samples = Vec::with_capacity(config.sample_count);

    for sample_index in 0..config.sample_count {
        let timestamp = sample_index as f64 * config.time_step;
        if sample_index > 0 {
            state.phases = rk4_step(&config, model, state, timestamp - config.time_step)?;
        }
        if let Some(action) = action_schedule.get(&sample_index) {
            apply_intervention(&mut state, *action)?;
        }
        if !state.phases.iter().all(|phase| phase.is_finite()) {
            return Err(CoordinationError::NonFiniteState);
        }
        samples.push(LegPhaseSample {
            timestamp,
            phases: state.phases,
            enabled: state.enabled,
            feedback_enabled: state.feedback_enabled,
        });
    }

    Ok(SixLegTrace {
        config,
        model,
        interventions: interventions.to_vec(),
        samples,
    })
}

#[derive(Debug, Clone, Copy)]
struct NetworkState {
    phases: [f64; LEG_COUNT],
    enabled: [bool; LEG_COUNT],
    feedback_enabled: [bool; LEG_COUNT],
    coupling_enabled: [[bool; LEG_COUNT]; LEG_COUNT],
}

fn validate_interventions(
    interventions: &[Intervention],
    config: SixLegConfig,
) -> Result<BTreeMap<usize, Intervention>, CoordinationError> {
    let mut schedule = BTreeMap::new();
    for intervention in interventions {
        let index = intervention.sample_index();
        if index == 0 || index >= config.sample_count {
            return Err(CoordinationError::InvalidInterventionIndex);
        }
        if schedule.insert(index, *intervention).is_some() {
            return Err(CoordinationError::DuplicateInterventionSample);
        }
        match intervention {
            Intervention::PhasePulse {
                delta_phase_radians,
                ..
            } if !delta_phase_radians.is_finite() => {
                return Err(CoordinationError::InvalidPhasePulse);
            }
            Intervention::CouplingEdgeCut {
                source, target, reciprocal, ..
            } => {
                if source == target {
                    return Err(CoordinationError::SelfCouplingCut);
                }
                if config.coupling_gains[source.index()][target.index()] <= 0.0
                    || (reciprocal
                        && config.coupling_gains[target.index()][source.index()] <= 0.0)
                {
                    return Err(CoordinationError::CouplingEdgeAbsent);
                }
            }
            _ => {}
        }
    }
    Ok(schedule)
}

fn apply_intervention(
    state: &mut NetworkState,
    intervention: Intervention,
) -> Result<(), CoordinationError> {
    match intervention {
        Intervention::PhasePulse {
            leg,
            delta_phase_radians,
            ..
        } => {
            let index = leg.index();
            if !state.enabled[index] {
                return Err(CoordinationError::InvalidPhasePulse);
            }
            state.phases[index] += delta_phase_radians;
        }
        Intervention::FeedbackCut { leg, .. } => {
            if let Some(leg) = leg {
                state.feedback_enabled[leg.index()] = false;
            } else {
                state.feedback_enabled = [false; LEG_COUNT];
            }
        }
        Intervention::CouplingEdgeCut {
            source,
            target,
            reciprocal,
            ..
        } => {
            state.coupling_enabled[source.index()][target.index()] = false;
            if reciprocal {
                state.coupling_enabled[target.index()][source.index()] = false;
            }
        }
        Intervention::DisableLeg { leg, .. } => {
            state.enabled[leg.index()] = false;
        }
    }
    if state.phases.iter().all(|phase| phase.is_finite()) {
        Ok(())
    } else {
        Err(CoordinationError::NonFiniteState)
    }
}

fn rk4_step(
    config: &SixLegConfig,
    model: CoordinationModel,
    state: NetworkState,
    time: f64,
) -> Result<[f64; LEG_COUNT], CoordinationError> {
    let dt = config.time_step;
    let k1 = phase_derivative(config, model, state, time);
    let k2 = phase_derivative(
        config,
        model,
        with_phases(state, add_scaled(state.phases, k1, dt * 0.5)),
        time + dt * 0.5,
    );
    let k3 = phase_derivative(
        config,
        model,
        with_phases(state, add_scaled(state.phases, k2, dt * 0.5)),
        time + dt * 0.5,
    );
    let k4 = phase_derivative(
        config,
        model,
        with_phases(state, add_scaled(state.phases, k3, dt)),
        time + dt,
    );

    let mut result = [0.0; LEG_COUNT];
    for index in 0..LEG_COUNT {
        result[index] = state.phases[index]
            + dt * (k1[index] + 2.0 * k2[index] + 2.0 * k3[index] + k4[index]) / 6.0;
    }
    if result.iter().all(|phase| phase.is_finite()) {
        Ok(result)
    } else {
        Err(CoordinationError::NonFiniteState)
    }
}

fn with_phases(mut state: NetworkState, phases: [f64; LEG_COUNT]) -> NetworkState {
    state.phases = phases;
    state
}

fn add_scaled(
    left: [f64; LEG_COUNT],
    right: [f64; LEG_COUNT],
    scale: f64,
) -> [f64; LEG_COUNT] {
    let mut result = [0.0; LEG_COUNT];
    for index in 0..LEG_COUNT {
        result[index] = left[index] + right[index] * scale;
    }
    result
}

fn phase_derivative(
    config: &SixLegConfig,
    model: CoordinationModel,
    state: NetworkState,
    time: f64,
) -> [f64; LEG_COUNT] {
    let drive_phase = config
        .drive_angular_frequency
        .mul_add(time, config.drive_initial_phase);
    let mut derivative = [0.0; LEG_COUNT];

    for leg_index in 0..LEG_COUNT {
        if !state.enabled[leg_index] {
            continue;
        }

        let mut rate = match model {
            CoordinationModel::Independent
            | CoordinationModel::CentrallyCoupled
            | CoordinationModel::Hybrid => config.natural_angular_frequencies[leg_index],
            CoordinationModel::FeedbackDriven => 0.0,
        };

        if matches!(
            model,
            CoordinationModel::CentrallyCoupled | CoordinationModel::Hybrid
        ) {
            for other_index in 0..LEG_COUNT {
                if other_index == leg_index
                    || !state.enabled[other_index]
                    || !state.coupling_enabled[leg_index][other_index]
                {
                    continue;
                }
                let preferred_delta = config.preferred_phase_offsets[other_index]
                    - config.preferred_phase_offsets[leg_index];
                let phase_error =
                    state.phases[other_index] - state.phases[leg_index] - preferred_delta;
                rate += config.coupling_gains[leg_index][other_index] * phase_error.sin();
            }
        }

        if matches!(
            model,
            CoordinationModel::FeedbackDriven | CoordinationModel::Hybrid
        ) && state.feedback_enabled[leg_index]
        {
            let target_phase =
                drive_phase + config.preferred_phase_offsets[leg_index];
            let phase_error = target_phase - state.phases[leg_index];
            rate += config.feedback_gains[leg_index] * phase_error.sin();
        }
        derivative[leg_index] = rate;
    }
    derivative
}

/// Compute phase coordination relative to a declared phase-offset pattern.
/// Disabled legs are excluded rather than replaced with zero observations.
pub fn coordination_metrics(
    sample: &LegPhaseSample,
    target_offsets: [f64; LEG_COUNT],
) -> Result<CoordinationMetrics, CoordinationError> {
    if !sample.phases.iter().all(|phase| phase.is_finite())
        || !target_offsets.iter().all(|phase| phase.is_finite())
    {
        return Err(CoordinationError::NonFiniteState);
    }

    let active = (0..LEG_COUNT)
        .filter(|index| sample.enabled[*index])
        .collect::<Vec<_>>();
    if active.len() < 2 {
        return Err(CoordinationError::InsufficientActiveLegs);
    }

    let mut squared_error = 0.0;
    let mut maximum_error = 0.0_f64;
    let mut pair_count = 0_usize;
    for (position, left) in active.iter().enumerate() {
        for right in active.iter().skip(position + 1) {
            let observed_delta = sample.phases[*right] - sample.phases[*left];
            let target_delta = target_offsets[*right] - target_offsets[*left];
            let error = wrap_angle(observed_delta - target_delta);
            squared_error += error * error;
            maximum_error = maximum_error.max(error.abs());
            pair_count += 1;
        }
    }

    let mean_cos = active
        .iter()
        .map(|index| (sample.phases[*index] - target_offsets[*index]).cos())
        .sum::<f64>()
        / active.len() as f64;
    let mean_sin = active
        .iter()
        .map(|index| (sample.phases[*index] - target_offsets[*index]).sin())
        .sum::<f64>()
        / active.len() as f64;

    Ok(CoordinationMetrics {
        pairwise_phase_error_radians: (squared_error / pair_count as f64).sqrt(),
        target_aligned_order_parameter: mean_cos.hypot(mean_sin),
        maximum_pairwise_phase_error_radians: maximum_error,
    })
}

pub fn coordination_error_series(
    trace: &SixLegTrace,
    target_offsets: [f64; LEG_COUNT],
) -> Result<Vec<f64>, CoordinationError> {
    validate_trace(&trace.samples)?;
    trace
        .samples
        .iter()
        .map(|sample| {
            Ok(coordination_metrics(sample, target_offsets)?.pairwise_phase_error_radians)
        })
        .collect()
}

/// Find the first sample at or after onset with a contiguous dwell window of
/// coordination errors at or below the supplied tolerance.
pub fn recovery_result(
    trace: &SixLegTrace,
    target_offsets: [f64; LEG_COUNT],
    onset_sample: usize,
    tolerance_radians: f64,
    dwell_samples: usize,
) -> Result<RecoveryResult, CoordinationError> {
    if onset_sample >= trace.samples.len() {
        return Err(CoordinationError::InvalidInterventionIndex);
    }
    if !tolerance_radians.is_finite() || tolerance_radians < 0.0 {
        return Err(CoordinationError::InvalidCoordinationThreshold);
    }
    if dwell_samples == 0 {
        return Err(CoordinationError::InvalidRecoveryWindow);
    }
    let errors = coordination_error_series(trace, target_offsets)?;
    let recovered = errors
        .windows(dwell_samples)
        .enumerate()
        .skip(onset_sample)
        .find(|(_, window)| window.iter().all(|error| *error <= tolerance_radians))
        .map(|(index, _)| index);
    let final_error_radians = *errors.last().ok_or(CoordinationError::InvalidTraceLength)?;
    let recovered_at_time = recovered.map(|index| trace.samples[index].timestamp);

    Ok(RecoveryResult {
        recovered_at_sample: recovered,
        recovered_at_time,
        final_error_radians,
    })
}

/// Count pairwise wrapped phase-error branch crossings after the given sample.
/// This is a kinematic diagnostic, not a biological slip-event classifier.
pub fn phase_slip_count(
    trace: &SixLegTrace,
    target_offsets: [f64; LEG_COUNT],
    start_sample: usize,
) -> Result<usize, CoordinationError> {
    if start_sample >= trace.samples.len() {
        return Err(CoordinationError::InvalidInterventionIndex);
    }
    validate_trace(&trace.samples)?;
    let mut slips = 0;
    for left in 0..LEG_COUNT {
        for right in (left + 1)..LEG_COUNT {
            let target_delta = target_offsets[right] - target_offsets[left];
            for index in (start_sample + 1)..trace.samples.len() {
                if !trace.samples[index - 1].enabled[left]
                    || !trace.samples[index - 1].enabled[right]
                    || !trace.samples[index].enabled[left]
                    || !trace.samples[index].enabled[right]
                {
                    continue;
                }
                let previous = wrap_angle(
                    trace.samples[index - 1].phases[right]
                        - trace.samples[index - 1].phases[left]
                        - target_delta,
                );
                let current = wrap_angle(
                    trace.samples[index].phases[right]
                        - trace.samples[index].phases[left]
                        - target_delta,
                );
                if (current - previous).abs() > std::f64::consts::PI {
                    slips += 1;
                }
            }
        }
    }
    Ok(slips)
}

fn validate_trace(samples: &[LegPhaseSample]) -> Result<(), CoordinationError> {
    if samples.is_empty() {
        return Err(CoordinationError::InvalidTraceLength);
    }
    let mut previous = None;
    for sample in samples {
        if !sample.timestamp.is_finite()
            || !sample.phases.iter().all(|phase| phase.is_finite())
        {
            return Err(CoordinationError::NonFiniteState);
        }
        if previous.is_some_and(|time| sample.timestamp <= time) {
            return Err(CoordinationError::NonMonotonicTimestamp);
        }
        previous = Some(sample.timestamp);
    }
    Ok(())
}

fn wrap_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> SixLegConfig {
        let mut coupling_gains = [[0.0; LEG_COUNT]; LEG_COUNT];
        for row in 0..LEG_COUNT {
            for column in 0..LEG_COUNT {
                if row != column {
                    coupling_gains[row][column] = 2.5;
                }
            }
        }
        SixLegConfig {
            natural_angular_frequencies: [2.0; LEG_COUNT],
            initial_phases: [0.0, 1.4, -1.0, 2.4, 0.8, -2.1],
            coupling_gains,
            preferred_phase_offsets: SixLegConfig::tripod(),
            feedback_gains: [4.0; LEG_COUNT],
            drive_angular_frequency: 2.0,
            drive_initial_phase: 0.0,
            time_step: 0.01,
            sample_count: 1201,
        }
    }

    fn final_metrics(trace: &SixLegTrace, config: SixLegConfig) -> CoordinationMetrics {
        coordination_metrics(trace.samples.last().unwrap(), config.preferred_phase_offsets)
            .unwrap()
    }

    #[test]
    fn tripod_target_encodes_two_alternating_leg_groups() {
        let target = SixLegConfig::tripod();
        let pi = std::f64::consts::PI;
        assert_eq!(target, [0.0, pi, 0.0, pi, pi, 0.0]);
        assert_eq!(target[Leg::LeftFront.index()], target[Leg::LeftRear.index()]);
        assert_eq!(target[Leg::LeftFront.index()], target[Leg::RightMiddle.index()]);
        assert_eq!(target[Leg::LeftMiddle.index()], target[Leg::RightFront.index()]);
        assert_eq!(target[Leg::LeftMiddle.index()], target[Leg::RightRear.index()]);
    }

    #[test]
    fn central_phase_coupling_converges_to_declared_tripod_offsets() {
        let config = base_config();
        let trace = simulate(config, CoordinationModel::CentrallyCoupled, &[]).unwrap();
        let metrics = final_metrics(&trace, config);
        assert!(metrics.pairwise_phase_error_radians < 0.02);
        assert!(metrics.target_aligned_order_parameter > 0.999);
    }

    #[test]
    fn independent_heterogeneous_oscillators_accumulate_relative_phase_drift() {
        let mut config = base_config();
        config.natural_angular_frequencies = [1.7, 1.85, 2.0, 2.15, 2.3, 2.45];
        config.initial_phases = config.preferred_phase_offsets;
        let trace = simulate(config, CoordinationModel::Independent, &[]).unwrap();
        let initial = coordination_metrics(&trace.samples[0], config.preferred_phase_offsets)
            .unwrap();
        let final_state = final_metrics(&trace, config);
        assert!(final_state.pairwise_phase_error_radians > initial.pairwise_phase_error_radians);
    }

    #[test]
    fn feedback_driven_phases_track_external_reference_then_stop_after_feedback_cut() {
        let config = base_config();
        let cut_sample = 600;
        let trace = simulate(
            config,
            CoordinationModel::FeedbackDriven,
            &[Intervention::FeedbackCut {
                sample_index: cut_sample,
                leg: None,
            }],
        )
        .unwrap();

        let before = &trace.samples[cut_sample - 1];
        let after = &trace.samples[cut_sample];
        let later = trace.samples.last().unwrap();
        for leg in LEGS {
            let index = leg.index();
            assert!(before.feedback_enabled[index]);
            assert!(!after.feedback_enabled[index]);
            assert!((later.phases[index] - after.phases[index]).abs() < 1.0e-12);
        }
    }

    #[test]
    fn hybrid_network_retains_autonomous_phase_progress_after_feedback_loss() {
        let config = base_config();
        let cut_sample = 600;
        let trace = simulate(
            config,
            CoordinationModel::Hybrid,
            &[Intervention::FeedbackCut {
                sample_index: cut_sample,
                leg: None,
            }],
        )
        .unwrap();

        let first = &trace.samples[cut_sample];
        let last = trace.samples.last().unwrap();
        let duration = last.timestamp - first.timestamp;
        for leg in LEGS {
            let index = leg.index();
            let observed_frequency =
                (last.phases[index] - first.phases[index]) / duration;
            assert!((observed_frequency - 2.0).abs() < 0.02);
        }
        assert!(final_metrics(&trace, config).pairwise_phase_error_radians < 0.02);
    }

    #[test]
    fn central_coupling_corrects_a_leg_specific_phase_pulse() {
        let config = base_config();
        let onset = 300;
        let trace = simulate(
            config,
            CoordinationModel::CentrallyCoupled,
            &[Intervention::PhasePulse {
                sample_index: onset,
                leg: Leg::LeftFront,
                delta_phase_radians: 0.8,
            }],
        )
        .unwrap();
        let recovery =
            recovery_result(&trace, config.preferred_phase_offsets, onset, 0.03, 20).unwrap();
        assert!(recovery.recovered_at_sample.is_some());
        assert!(recovery.final_error_radians < 0.02);
    }

    #[test]
    fn independent_oscillators_do_not_correct_a_leg_specific_phase_pulse() {
        let config = base_config();
        let onset = 300;
        let trace = simulate(
            config,
            CoordinationModel::Independent,
            &[Intervention::PhasePulse {
                sample_index: onset,
                leg: Leg::LeftFront,
                delta_phase_radians: 0.8,
            }],
        )
        .unwrap();
        let recovery =
            recovery_result(&trace, config.preferred_phase_offsets, onset, 0.03, 20).unwrap();
        assert_eq!(recovery.recovered_at_sample, None);
        assert!(recovery.final_error_radians > 0.1);
    }

    #[test]
    fn a_leg_disable_freezes_only_that_leg_and_does_not_zero_fill_its_phase() {
        let config = base_config();
        let onset = 400;
        let trace = simulate(
            config,
            CoordinationModel::CentrallyCoupled,
            &[Intervention::DisableLeg {
                sample_index: onset,
                leg: Leg::RightRear,
            }],
        )
        .unwrap();
        let after = &trace.samples[onset];
        let final_sample = trace.samples.last().unwrap();
        let disabled = Leg::RightRear.index();
        assert!(!after.enabled[disabled]);
        assert!(!final_sample.enabled[disabled]);
        assert_eq!(final_sample.phases[disabled], after.phases[disabled]);
        for leg in [
            Leg::LeftFront,
            Leg::LeftMiddle,
            Leg::LeftRear,
            Leg::RightFront,
            Leg::RightMiddle,
        ] {
            assert!(final_sample.phases[leg.index()] > after.phases[leg.index()]);
        }
    }

    #[test]
    fn removing_a_bridge_edge_allows_two_subnetworks_to_drift_apart() {
        let mut config = base_config();
        config.natural_angular_frequencies = [1.9, 1.9, 1.9, 2.1, 2.1, 2.1];
        config.coupling_gains = [[0.0; LEG_COUNT]; LEG_COUNT];
        // Two internally coupled tripods, connected only by one bridge edge.
        let left_group = [0, 2, 5];
        let right_group = [1, 3, 4];
        for group in [left_group, right_group] {
            for source in group {
                for target in group {
                    if source != target {
                        config.coupling_gains[source][target] = 3.0;
                    }
                }
            }
        }
        config.coupling_gains[5][1] = 2.0;
        config.coupling_gains[1][5] = 2.0;

        let trace = simulate(
            config,
            CoordinationModel::CentrallyCoupled,
            &[Intervention::CouplingEdgeCut {
                sample_index: 400,
                source: Leg::RightMiddle,
                target: Leg::LeftMiddle,
                reciprocal: true,
            }],
        )
        .unwrap();
        let pre = coordination_metrics(&trace.samples[399], config.preferred_phase_offsets)
            .unwrap();
        let post = final_metrics(&trace, config);
        assert!(post.pairwise_phase_error_radians > pre.pairwise_phase_error_radians + 0.1);
    }

    #[test]
    fn common_phase_offset_does_not_change_target_alignment_metrics() {
        let config = base_config();
        let common_phase = 11.7;
        let sample = LegPhaseSample {
            timestamp: 0.0,
            phases: std::array::from_fn(|index| {
                common_phase + config.preferred_phase_offsets[index]
            }),
            enabled: [true; LEG_COUNT],
            feedback_enabled: [true; LEG_COUNT],
        };
        let metrics = coordination_metrics(&sample, config.preferred_phase_offsets).unwrap();
        assert!(metrics.pairwise_phase_error_radians < 1.0e-12);
        assert!((metrics.target_aligned_order_parameter - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn recovery_requires_a_contiguous_dwell_window_after_the_perturbation() {
        let config = base_config();
        let trace = simulate(
            config,
            CoordinationModel::CentrallyCoupled,
            &[Intervention::PhasePulse {
                sample_index: 300,
                leg: Leg::LeftFront,
                delta_phase_radians: 0.8,
            }],
        )
        .unwrap();
        let short = recovery_result(&trace, config.preferred_phase_offsets, 300, 0.03, 1).unwrap();
        let sustained =
            recovery_result(&trace, config.preferred_phase_offsets, 300, 0.03, 25).unwrap();
        assert!(short.recovered_at_sample.is_some());
        assert!(sustained.recovered_at_sample.is_some());
        assert!(sustained.recovered_at_sample >= short.recovered_at_sample);
    }

    #[test]
    fn phase_slip_metric_detects_wrapped_pairwise_phase_error_crossings() {
        let target = SixLegConfig::tripod();
        let mut samples = Vec::new();
        for (index, phase) in [0.0, 0.0, 3.0, 3.4, 6.8].iter().enumerate() {
            let mut phases = target;
            phases[0] = *phase;
            samples.push(LegPhaseSample {
                timestamp: index as f64,
                phases,
                enabled: [true; LEG_COUNT],
                feedback_enabled: [true; LEG_COUNT],
            });
        }
        let trace = SixLegTrace {
            config: base_config(),
            model: CoordinationModel::Independent,
            interventions: vec![],
            samples,
        };
        assert!(phase_slip_count(&trace, target, 1).unwrap() > 0);
    }

    #[test]
    fn rk4_refinement_converges_for_the_same_physical_time_horizon() {
        let mut coarse = base_config();
        coarse.time_step = 0.02;
        coarse.sample_count = 501;
        let mut fine = base_config();
        fine.time_step = 0.01;
        fine.sample_count = 1001;
        let coarse_trace = simulate(coarse, CoordinationModel::Hybrid, &[]).unwrap();
        let fine_trace = simulate(fine, CoordinationModel::Hybrid, &[]).unwrap();
        let coarse_final = coarse_trace.samples.last().unwrap();
        let fine_final = fine_trace.samples.last().unwrap();
        for leg in LEGS {
            let index = leg.index();
            assert!((coarse_final.phases[index] - fine_final.phases[index]).abs() < 1.0e-5);
        }
    }

    #[test]
    fn rejects_duplicate_actions_at_one_sample_and_invalid_interventions() {
        let config = base_config();
        assert_eq!(
            simulate(
                config,
                CoordinationModel::Independent,
                &[
                    Intervention::PhasePulse {
                        sample_index: 100,
                        leg: Leg::LeftFront,
                        delta_phase_radians: 0.1,
                    },
                    Intervention::FeedbackCut {
                        sample_index: 100,
                        leg: None,
                    },
                ],
            ),
            Err(CoordinationError::DuplicateInterventionSample)
        );
        assert_eq!(
            simulate(
                config,
                CoordinationModel::Independent,
                &[Intervention::PhasePulse {
                    sample_index: 100,
                    leg: Leg::LeftFront,
                    delta_phase_radians: f64::NAN,
                }],
            ),
            Err(CoordinationError::InvalidPhasePulse)
        );
        assert_eq!(
            simulate(
                config,
                CoordinationModel::Independent,
                &[Intervention::CouplingEdgeCut {
                    sample_index: 100,
                    source: Leg::LeftFront,
                    target: Leg::LeftFront,
                    reciprocal: true,
                }],
            ),
            Err(CoordinationError::SelfCouplingCut)
        );
    }

    #[test]
    fn rejects_invalid_configurations_and_recovery_controls() {
        let mut config = base_config();
        config.coupling_gains[0][0] = 1.0;
        assert_eq!(
            simulate(config, CoordinationModel::CentrallyCoupled, &[]),
            Err(CoordinationError::SelfCouplingNotAllowed)
        );

        config = base_config();
        config.time_step = 0.0;
        assert_eq!(
            simulate(config, CoordinationModel::CentrallyCoupled, &[]),
            Err(CoordinationError::InvalidTimeStep)
        );

        let config = base_config();
        let trace = simulate(config, CoordinationModel::Independent, &[]).unwrap();
        assert_eq!(
            recovery_result(&trace, config.preferred_phase_offsets, 2, -0.1, 2),
            Err(CoordinationError::InvalidCoordinationThreshold)
        );
        assert_eq!(
            recovery_result(&trace, config.preferred_phase_offsets, 2, 0.1, 0),
            Err(CoordinationError::InvalidRecoveryWindow)
        );
    }

    #[test]
    fn coordination_metrics_abstain_when_only_one_leg_is_observed() {
        let mut sample = LegPhaseSample {
            timestamp: 0.0,
            phases: [0.0; LEG_COUNT],
            enabled: [false; LEG_COUNT],
            feedback_enabled: [true; LEG_COUNT],
        };
        sample.enabled[Leg::LeftFront.index()] = true;
        assert_eq!(
            coordination_metrics(&sample, SixLegConfig::tripod()),
            Err(CoordinationError::InsufficientActiveLegs)
        );
    }

    #[test]
    fn coupling_ablation_rejects_a_nonexistent_edge() {
        let config = base_config();
        assert_eq!(
            simulate(
                config,
                CoordinationModel::CentrallyCoupled,
                &[Intervention::CouplingEdgeCut {
                    sample_index: 100,
                    source: Leg::LeftFront,
                    target: Leg::LeftFront,
                    reciprocal: true,
                }]
            ),
            Err(CoordinationError::SelfCouplingCut)
        );
        let mut disconnected = config;
        disconnected.coupling_gains[0][1] = 0.0;
        assert_eq!(
            simulate(
                disconnected,
                CoordinationModel::CentrallyCoupled,
                &[Intervention::CouplingEdgeCut {
                    sample_index: 100,
                    source: Leg::LeftFront,
                    target: Leg::LeftMiddle,
                    reciprocal: false,
                }]
            ),
            Err(CoordinationError::CouplingEdgeAbsent)
        );
    }

    #[test]
    fn leg_identifier_indices_are_stable_and_canonical() {
        assert_eq!(LEGS.map(Leg::index), [0, 1, 2, 3, 4, 5]);
    }
}
