//! Six-leg coordination benchmark with known-truth rhythm mechanisms.
//!
//! This is a deterministic synthetic mechanism-discrimination fixture. It
//! does not reproduce an animal connectome or a full articulated-body model.
//! Observable samples contain only six leg-plane outputs and active flags;
//! intervention and mechanism metadata remain outside the scored samples.

use std::f64::consts::{PI, TAU};

pub const LEG_COUNT: usize = 6;
pub const DEFAULT_TRIPOD_OFFSETS: [f64; LEG_COUNT] = [0.0, PI, 0.0, PI, 0.0, PI];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(usize)]
pub enum LegId {
    LeftFront = 0,
    LeftMiddle = 1,
    LeftRear = 2,
    RightFront = 3,
    RightMiddle = 4,
    RightRear = 5,
}

impl LegId {
    pub const ALL: [Self; LEG_COUNT] = [
        Self::LeftFront,
        Self::LeftMiddle,
        Self::LeftRear,
        Self::RightFront,
        Self::RightMiddle,
        Self::RightRear,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationModel {
    IndependentOscillators,
    CoupledOscillators,
    CoupledHopfOscillators,
    FeedbackAttractor,
    HybridOscillators,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinationConfig {
    pub sample_count: usize,
    pub sample_interval: f64,
    pub angular_frequency: f64,
    pub initial_phase: f64,
    pub target_offsets: [f64; LEG_COUNT],
    pub central_coupling_gain: f64,
    pub feedback_gain: f64,
    pub attractor_decay_rate: f64,
    pub hopf_radial_gain: f64,
    pub hopf_limit_cycle_amplitude: f64,
}

impl Default for CoordinationConfig {
    fn default() -> Self {
        Self {
            sample_count: 240,
            sample_interval: 0.025,
            angular_frequency: TAU,
            initial_phase: 0.0,
            target_offsets: DEFAULT_TRIPOD_OFFSETS,
            central_coupling_gain: 5.0,
            feedback_gain: 8.0,
            attractor_decay_rate: 3.0,
            hopf_radial_gain: 10.0,
            hopf_limit_cycle_amplitude: 1.0,
        }
    }
}

impl CoordinationConfig {
    pub fn validate(self, model: CoordinationModel) -> Result<(), CoordinationError> {
        if self.sample_count < 2 {
            return Err(CoordinationError::InvalidSampleCount);
        }
        if !self.sample_interval.is_finite() || self.sample_interval <= 0.0 {
            return Err(CoordinationError::InvalidSampleInterval);
        }
        if !self.angular_frequency.is_finite() {
            return Err(CoordinationError::InvalidAngularFrequency);
        }
        if !self.initial_phase.is_finite() {
            return Err(CoordinationError::InvalidInitialPhase);
        }
        if !self.target_offsets.iter().all(|value| value.is_finite()) {
            return Err(CoordinationError::InvalidTargetOffsets);
        }
        if !self.central_coupling_gain.is_finite() || self.central_coupling_gain < 0.0 {
            return Err(CoordinationError::InvalidCentralCouplingGain);
        }
        if !self.feedback_gain.is_finite()
            || self.feedback_gain < 0.0
            || (matches!(
                model,
                CoordinationModel::FeedbackAttractor | CoordinationModel::HybridOscillators
            ) && self.feedback_gain == 0.0)
        {
            return Err(CoordinationError::InvalidFeedbackGain);
        }
        if !self.attractor_decay_rate.is_finite() || self.attractor_decay_rate <= 0.0 {
            return Err(CoordinationError::InvalidAttractorDecayRate);
        }
        if matches!(model, CoordinationModel::CoupledHopfOscillators)
            && (!self.hopf_radial_gain.is_finite()
                || self.hopf_radial_gain <= 0.0
                || !self.hopf_limit_cycle_amplitude.is_finite()
                || self.hopf_limit_cycle_amplitude <= 0.0
                || self.hopf_radial_gain
                    * self.sample_interval
                    * self.hopf_limit_cycle_amplitude.powi(2)
                    > 1.0)
        {
            return Err(CoordinationError::InvalidHopfParameters);
        }
        if matches!(
            model,
            CoordinationModel::CoupledOscillators
                | CoordinationModel::CoupledHopfOscillators
                | CoordinationModel::HybridOscillators
        ) && self.central_coupling_gain == 0.0
        {
            return Err(CoordinationError::InvalidCentralCouplingGain);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhasePulse {
    pub step: usize,
    pub leg: LegId,
    pub delta_phase_radians: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AmplitudePulse {
    pub step: usize,
    pub leg: LegId,
    /// Multiplicative amplitude factor in (0, 1].
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedbackDelay {
    pub from_step: usize,
    pub delay_steps: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LegFailure {
    pub from_step: usize,
    pub leg: LegId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyDetune {
    pub from_step: usize,
    pub leg: LegId,
    pub delta_angular_frequency: f64,
}

/// Interventions are composable so the benchmark can hold out combinations
/// (for example, a local phase pulse during a delayed-feedback condition).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Intervention {
    pub phase_pulse: Option<PhasePulse>,
    pub amplitude_pulse: Option<AmplitudePulse>,
    pub coupling_cut_from: Option<usize>,
    pub feedback_cut_from: Option<usize>,
    pub feedback_delay: Option<FeedbackDelay>,
    pub leg_failure: Option<LegFailure>,
    pub frequency_detune: Option<FrequencyDetune>,
}

impl Intervention {
    pub fn validate(self, sample_count: usize) -> Result<(), CoordinationError> {
        let valid_step = |step: usize| step < sample_count;
        if let Some(pulse) = self.phase_pulse {
            if !valid_step(pulse.step) {
                return Err(CoordinationError::InvalidInterventionStep);
            }
            if !pulse.delta_phase_radians.is_finite() {
                return Err(CoordinationError::InvalidPhasePulse);
            }
        }
        if let Some(pulse) = self.amplitude_pulse {
            if !valid_step(pulse.step) {
                return Err(CoordinationError::InvalidInterventionStep);
            }
            if !pulse.scale.is_finite() || pulse.scale <= 0.0 || pulse.scale > 1.0 {
                return Err(CoordinationError::InvalidAmplitudePulse);
            }
        }
        if self.coupling_cut_from.is_some_and(|step| !valid_step(step))
            || self.feedback_cut_from.is_some_and(|step| !valid_step(step))
        {
            return Err(CoordinationError::InvalidInterventionStep);
        }
        if let Some(delay) = self.feedback_delay
            && (!valid_step(delay.from_step) || delay.delay_steps == 0)
        {
            return Err(CoordinationError::InvalidFeedbackDelay);
        }
        if self
            .leg_failure
            .is_some_and(|failure| !valid_step(failure.from_step))
        {
            return Err(CoordinationError::InvalidInterventionStep);
        }
        if let Some(detune) = self.frequency_detune {
            if !valid_step(detune.from_step) {
                return Err(CoordinationError::InvalidInterventionStep);
            }
            if !detune.delta_angular_frequency.is_finite() {
                return Err(CoordinationError::InvalidFrequencyDetune);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HexapodSample {
    pub timestamp: f64,
    /// Observable x coordinate for each leg's phase-plane output.
    pub x: [f64; LEG_COUNT],
    /// Observable y coordinate for each leg's phase-plane output.
    pub y: [f64; LEG_COUNT],
    /// False means the leg is unavailable, not a zero-valued measurement.
    pub active: [bool; LEG_COUNT],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HexapodTrace {
    pub model: CoordinationModel,
    pub intervention: Intervention,
    pub samples: Vec<HexapodSample>,
}

/// Frozen observable-only forecast vector matching the field contract used
/// by loco-intervention-design-v0::ModelPrediction. The caller supplies a
/// nonzero digest binding the timestamp grid, time units, canonical leg order
/// and per-sample active-leg mask.
#[derive(Debug, Clone, PartialEq)]
pub struct ObservableForecast {
    pub hypothesis_id: u64,
    pub observable_layout_digest: [u8; 32],
    /// Step-major, then canonical leg order, then x/y for active legs only.
    pub mean: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinationError {
    InvalidSampleCount,
    InvalidSampleInterval,
    InvalidAngularFrequency,
    InvalidInitialPhase,
    InvalidTargetOffsets,
    InvalidCentralCouplingGain,
    InvalidFeedbackGain,
    InvalidAttractorDecayRate,
    InvalidHopfParameters,
    InvalidInterventionStep,
    InvalidAmplitudePulse,
    AmplitudePulseUnsupported,
    InvalidPhasePulse,
    InvalidFeedbackDelay,
    InvalidFrequencyDetune,
    InvalidTraceLength,
    InvalidTimestamp,
    NonMonotonicTimestamp,
    TimestampGridMismatch,
    InvalidObservableLayoutDigest,
    InvalidAmplitudeThreshold,
    InvalidPhaseErrorThreshold,
    InvalidConsecutiveSampleCount,
    NumericalFailure,
}

pub fn simulate(
    model: CoordinationModel,
    intervention: Intervention,
    config: CoordinationConfig,
) -> Result<HexapodTrace, CoordinationError> {
    config.validate(model)?;
    intervention.validate(config.sample_count)?;
    if intervention.amplitude_pulse.is_some() && model != CoordinationModel::CoupledHopfOscillators
    {
        return Err(CoordinationError::AmplitudePulseUnsupported);
    }

    let mut phases =
        std::array::from_fn(|leg_index| config.initial_phase + config.target_offsets[leg_index]);
    let mut attractor_states = std::array::from_fn(|leg_index| {
        let phase = config.initial_phase + config.target_offsets[leg_index];
        [phase.cos(), phase.sin()]
    });
    let mut radii = [config.hopf_limit_cycle_amplitude; LEG_COUNT];
    let mut samples = Vec::with_capacity(config.sample_count);

    for step in 0..config.sample_count {
        let active = active_legs(step, intervention);
        apply_phase_pulse(
            step,
            model,
            intervention,
            &mut phases,
            &mut attractor_states,
        );
        apply_amplitude_pulse(step, intervention, &mut radii);

        let timestamp = step as f64 * config.sample_interval;
        if !timestamp.is_finite() {
            return Err(CoordinationError::NumericalFailure);
        }

        let (x, y) = if model == CoordinationModel::FeedbackAttractor {
            (
                std::array::from_fn(|leg_index| {
                    if active[leg_index] {
                        attractor_states[leg_index][0]
                    } else {
                        0.0
                    }
                }),
                std::array::from_fn(|leg_index| {
                    if active[leg_index] {
                        attractor_states[leg_index][1]
                    } else {
                        0.0
                    }
                }),
            )
        } else {
            (
                std::array::from_fn(|leg_index| {
                    if active[leg_index] {
                        output_radius(model, radii[leg_index]) * phases[leg_index].cos()
                    } else {
                        0.0
                    }
                }),
                std::array::from_fn(|leg_index| {
                    if active[leg_index] {
                        output_radius(model, radii[leg_index]) * phases[leg_index].sin()
                    } else {
                        0.0
                    }
                }),
            )
        };
        if !x.iter().all(|value| value.is_finite()) || !y.iter().all(|value| value.is_finite()) {
            return Err(CoordinationError::NumericalFailure);
        }
        samples.push(HexapodSample {
            timestamp,
            x,
            y,
            active,
        });

        if step + 1 < config.sample_count {
            let next_active = active_legs(step + 1, intervention);
            let context = AdvanceContext {
                next_step: step + 1,
                model,
                intervention,
                config,
                active: &next_active,
            };
            advance_state(context, &mut phases, &mut attractor_states, &mut radii)?;
        }
    }

    Ok(HexapodTrace {
        model,
        intervention,
        samples,
    })
}

fn output_radius(model: CoordinationModel, radius: f64) -> f64 {
    if model == CoordinationModel::CoupledHopfOscillators {
        radius
    } else {
        1.0
    }
}

fn apply_amplitude_pulse(step: usize, intervention: Intervention, radii: &mut [f64; LEG_COUNT]) {
    if let Some(pulse) = intervention.amplitude_pulse
        && step == pulse.step
    {
        radii[pulse.leg.index()] *= pulse.scale;
    }
}

fn active_legs(step: usize, intervention: Intervention) -> [bool; LEG_COUNT] {
    std::array::from_fn(|leg_index| {
        !intervention
            .leg_failure
            .is_some_and(|failure| step >= failure.from_step && failure.leg.index() == leg_index)
    })
}

fn apply_phase_pulse(
    step: usize,
    model: CoordinationModel,
    intervention: Intervention,
    phases: &mut [f64; LEG_COUNT],
    attractor_states: &mut [[f64; 2]; LEG_COUNT],
) {
    let Some(pulse) = intervention.phase_pulse else {
        return;
    };
    if step != pulse.step {
        return;
    }

    let leg_index = pulse.leg.index();
    if model == CoordinationModel::FeedbackAttractor {
        let (sin_delta, cos_delta) = pulse.delta_phase_radians.sin_cos();
        let [x, y] = attractor_states[leg_index];
        attractor_states[leg_index] =
            [cos_delta * x - sin_delta * y, sin_delta * x + cos_delta * y];
    } else {
        phases[leg_index] += pulse.delta_phase_radians;
    }
}

struct AdvanceContext<'a> {
    next_step: usize,
    model: CoordinationModel,
    intervention: Intervention,
    config: CoordinationConfig,
    active: &'a [bool; LEG_COUNT],
}

fn advance_state(
    context: AdvanceContext<'_>,
    phases: &mut [f64; LEG_COUNT],
    attractor_states: &mut [[f64; 2]; LEG_COUNT],
    radii: &mut [f64; LEG_COUNT],
) -> Result<(), CoordinationError> {
    let AdvanceContext {
        next_step,
        model,
        intervention,
        config,
        active,
    } = context;
    let elapsed = config.sample_interval;
    let feedback_present = intervention
        .feedback_cut_from
        .is_none_or(|cut_step| next_step < cut_step);
    let coupling_present = intervention
        .coupling_cut_from
        .is_none_or(|cut_step| next_step < cut_step);

    if model == CoordinationModel::FeedbackAttractor {
        let delay_steps = intervention
            .feedback_delay
            .filter(|delay| next_step >= delay.from_step)
            .map_or(0, |delay| delay.delay_steps);
        let reference_step = next_step.saturating_sub(delay_steps);
        let reference_time = reference_step as f64 * elapsed;
        for (leg_index, state) in attractor_states.iter_mut().enumerate() {
            if !active[leg_index] {
                continue;
            }
            if feedback_present {
                let reference_phase = config.initial_phase
                    + config.angular_frequency * reference_time
                    + config.target_offsets[leg_index];
                let target = [reference_phase.cos(), reference_phase.sin()];
                let alpha = -(-config.feedback_gain * elapsed).exp_m1();
                state[0] += alpha * (target[0] - state[0]);
                state[1] += alpha * (target[1] - state[1]);
            } else {
                let decay = (-config.attractor_decay_rate * elapsed).exp();
                state[0] *= decay;
                state[1] *= decay;
            }
            if !state.iter().all(|value| value.is_finite()) {
                return Err(CoordinationError::NumericalFailure);
            }
        }
        return Ok(());
    }

    if model == CoordinationModel::CoupledHopfOscillators {
        for (leg_index, radius) in radii.iter_mut().enumerate() {
            if active[leg_index] {
                let target_squared = config.hopf_limit_cycle_amplitude.powi(2);
                *radius +=
                    config.hopf_radial_gain * (target_squared - radius.powi(2)) * *radius * elapsed;
                if !radius.is_finite() || *radius <= 0.0 {
                    return Err(CoordinationError::NumericalFailure);
                }
            }
        }
    }

    let previous = *phases;
    let mut next = previous;
    let active_count = active.iter().filter(|enabled| **enabled).count();
    for (leg_index, next_phase) in next.iter_mut().enumerate() {
        if !active[leg_index] {
            continue;
        }

        let detune = intervention
            .frequency_detune
            .filter(|change| next_step >= change.from_step && change.leg.index() == leg_index)
            .map_or(0.0, |change| change.delta_angular_frequency);
        let mut angular_velocity = config.angular_frequency + detune;

        if coupling_present
            && active_count > 1
            && matches!(
                model,
                CoordinationModel::CoupledOscillators
                    | CoordinationModel::CoupledHopfOscillators
                    | CoordinationModel::HybridOscillators
            )
        {
            let mut coupling_sum = 0.0;
            for (other_index, other_phase) in previous.iter().enumerate() {
                if other_index == leg_index || !active[other_index] {
                    continue;
                }
                let desired_difference =
                    config.target_offsets[other_index] - config.target_offsets[leg_index];
                coupling_sum +=
                    wrap_angle((*other_phase - previous[leg_index]) - desired_difference).sin();
            }
            angular_velocity +=
                config.central_coupling_gain * coupling_sum / (active_count - 1) as f64;
        }

        if model == CoordinationModel::HybridOscillators && feedback_present {
            let delay_steps = intervention
                .feedback_delay
                .filter(|delay| next_step >= delay.from_step)
                .map_or(0, |delay| delay.delay_steps);
            let reference_step = next_step.saturating_sub(delay_steps);
            let reference_phase = config.initial_phase
                + config.angular_frequency * reference_step as f64 * elapsed
                + config.target_offsets[leg_index];
            angular_velocity +=
                config.feedback_gain * wrap_angle(reference_phase - previous[leg_index]).sin();
        }

        *next_phase = previous[leg_index] + angular_velocity * elapsed;
        if !next_phase.is_finite() {
            return Err(CoordinationError::NumericalFailure);
        }
    }
    *phases = next;
    Ok(())
}

fn wrap_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

fn observed_phases(
    sample: &HexapodSample,
    minimum_amplitude: f64,
) -> Result<[Option<f64>; LEG_COUNT], CoordinationError> {
    if !minimum_amplitude.is_finite() || minimum_amplitude < 0.0 {
        return Err(CoordinationError::InvalidAmplitudeThreshold);
    }
    if !sample.timestamp.is_finite()
        || !sample.x.iter().all(|value| value.is_finite())
        || !sample.y.iter().all(|value| value.is_finite())
    {
        return Err(CoordinationError::NumericalFailure);
    }
    Ok(std::array::from_fn(|leg_index| {
        if !sample.active[leg_index] {
            return None;
        }
        let amplitude = sample.x[leg_index].hypot(sample.y[leg_index]);
        if amplitude <= minimum_amplitude {
            None
        } else {
            Some(sample.y[leg_index].atan2(sample.x[leg_index]))
        }
    }))
}

/// Circular RMS error of the observed six-leg phase pattern after removing
/// the declared target offsets and a shared global phase. At least two
/// observable legs are required.
pub fn phase_pattern_rmse(
    sample: &HexapodSample,
    target_offsets: [f64; LEG_COUNT],
    minimum_amplitude: f64,
) -> Result<Option<f64>, CoordinationError> {
    if !target_offsets.iter().all(|value| value.is_finite()) {
        return Err(CoordinationError::InvalidTargetOffsets);
    }
    let phases = observed_phases(sample, minimum_amplitude)?;
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    let mut count = 0_usize;

    for (leg_index, phase) in phases.iter().enumerate() {
        if let Some(phase) = phase {
            let corrected = wrap_angle(*phase - target_offsets[leg_index]);
            sum_sin += corrected.sin();
            sum_cos += corrected.cos();
            count += 1;
        }
    }
    if count < 2 {
        return Ok(None);
    }

    let consensus = sum_sin.atan2(sum_cos);
    let squared_error = phases
        .iter()
        .enumerate()
        .filter_map(|(leg_index, phase)| {
            phase.map(|phase| wrap_angle(phase - target_offsets[leg_index] - consensus).powi(2))
        })
        .sum::<f64>();
    Ok(Some((squared_error / count as f64).sqrt()))
}

/// Magnitude of the circular mean of offset-corrected leg phases. This is
/// reported separately from phase-pattern error and is undefined for fewer
/// than two observable legs.
pub fn phase_locking_value(
    sample: &HexapodSample,
    target_offsets: [f64; LEG_COUNT],
    minimum_amplitude: f64,
) -> Result<Option<f64>, CoordinationError> {
    if !target_offsets.iter().all(|value| value.is_finite()) {
        return Err(CoordinationError::InvalidTargetOffsets);
    }
    let phases = observed_phases(sample, minimum_amplitude)?;
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    let mut count = 0_usize;
    for (leg_index, phase) in phases.iter().enumerate() {
        if let Some(phase) = phase {
            let corrected = wrap_angle(*phase - target_offsets[leg_index]);
            sum_sin += corrected.sin();
            sum_cos += corrected.cos();
            count += 1;
        }
    }
    if count < 2 {
        return Ok(None);
    }
    Ok(Some(sum_sin.hypot(sum_cos) / count as f64))
}

/// Global phase lag relative to a supplied reference. This detects common
/// delays that leave inter-leg phase coordination (PLV) almost unchanged.
pub fn global_phase_lag(
    sample: &HexapodSample,
    target_offsets: [f64; LEG_COUNT],
    reference_phase: f64,
    minimum_amplitude: f64,
) -> Result<Option<f64>, CoordinationError> {
    if !reference_phase.is_finite() {
        return Err(CoordinationError::InvalidInitialPhase);
    }
    if !target_offsets.iter().all(|value| value.is_finite()) {
        return Err(CoordinationError::InvalidTargetOffsets);
    }
    let phases = observed_phases(sample, minimum_amplitude)?;
    let mut sum_sin = 0.0;
    let mut sum_cos = 0.0;
    let mut count = 0_usize;
    for (leg_index, phase) in phases.iter().enumerate() {
        if let Some(phase) = phase {
            let corrected = wrap_angle(*phase - target_offsets[leg_index]);
            sum_sin += corrected.sin();
            sum_cos += corrected.cos();
            count += 1;
        }
    }
    if count < 2 {
        return Ok(None);
    }
    Ok(Some(wrap_angle(sum_sin.atan2(sum_cos) - reference_phase)))
}

/// Return the first time after an intervention when phase-pattern error stays
/// below a threshold for the requested number of consecutive observations.
pub fn recovery_time(
    trace: &HexapodTrace,
    onset_step: usize,
    phase_error_threshold: f64,
    consecutive_samples: usize,
    target_offsets: [f64; LEG_COUNT],
    minimum_amplitude: f64,
) -> Result<Option<f64>, CoordinationError> {
    validate_timestamps(&trace.samples)?;
    if onset_step >= trace.samples.len() {
        return Err(CoordinationError::InvalidInterventionStep);
    }
    if !phase_error_threshold.is_finite() || phase_error_threshold < 0.0 {
        return Err(CoordinationError::InvalidPhaseErrorThreshold);
    }
    if consecutive_samples == 0 {
        return Err(CoordinationError::InvalidConsecutiveSampleCount);
    }
    if !target_offsets.iter().all(|value| value.is_finite()) {
        return Err(CoordinationError::InvalidTargetOffsets);
    }

    let mut consecutive = 0_usize;
    for (step, sample) in trace.samples.iter().enumerate().skip(onset_step) {
        let error = phase_pattern_rmse(sample, target_offsets, minimum_amplitude)?;
        if error.is_some_and(|value| value <= phase_error_threshold) {
            consecutive += 1;
            if consecutive >= consecutive_samples {
                let recovered_step = step + 1 - consecutive_samples;
                return Ok(Some(
                    (recovered_step - onset_step) as f64 * sample_interval(trace)?,
                ));
            }
        } else {
            consecutive = 0;
        }
    }
    Ok(None)
}

fn sample_interval(trace: &HexapodTrace) -> Result<f64, CoordinationError> {
    let first = trace
        .samples
        .first()
        .ok_or(CoordinationError::InvalidTraceLength)?;
    let second = trace
        .samples
        .get(1)
        .ok_or(CoordinationError::InvalidTraceLength)?;
    let interval = second.timestamp - first.timestamp;
    if !interval.is_finite() || interval <= 0.0 {
        return Err(CoordinationError::InvalidTimestamp);
    }
    Ok(interval)
}

impl HexapodTrace {
    /// Freeze the observable phase-plane outputs for one mechanism hypothesis.
    ///
    /// No intervention label, model enum or latent state is copied into the
    /// predictive vector. Unavailable legs are omitted, never encoded as
    /// synthetic zero measurements. The caller-supplied layout digest must
    /// bind the exact timestamps, units, leg order and active-observation mask
    /// and must be identical across hypotheses being compared for one action.
    pub fn freeze_observable_forecast(
        &self,
        hypothesis_id: u64,
        observable_layout_digest: [u8; 32],
    ) -> Result<ObservableForecast, CoordinationError> {
        if observable_layout_digest.iter().all(|byte| *byte == 0) {
            return Err(CoordinationError::InvalidObservableLayoutDigest);
        }
        validate_timestamps(&self.samples)?;
        let observable_count = self
            .samples
            .iter()
            .map(|sample| sample.active.iter().filter(|active| **active).count() * 2)
            .sum::<usize>();
        if observable_count == 0 {
            return Err(CoordinationError::InvalidTraceLength);
        }

        let mut mean = Vec::with_capacity(observable_count);
        for sample in &self.samples {
            for leg in LegId::ALL {
                let leg_index = leg.index();
                if sample.active[leg_index] {
                    mean.push(sample.x[leg_index]);
                    mean.push(sample.y[leg_index]);
                }
            }
        }
        if !mean.iter().all(|value| value.is_finite()) {
            return Err(CoordinationError::NumericalFailure);
        }
        Ok(ObservableForecast {
            hypothesis_id,
            observable_layout_digest,
            mean,
        })
    }
}

/// Validate exact timestamp alignment before comparing paired traces.
pub fn validate_aligned_traces(
    left: &[HexapodSample],
    right: &[HexapodSample],
) -> Result<(), CoordinationError> {
    if left.len() != right.len() || left.is_empty() {
        return Err(CoordinationError::InvalidTraceLength);
    }
    validate_timestamps(left)?;
    validate_timestamps(right)?;
    if left
        .iter()
        .zip(right)
        .any(|(left_sample, right_sample)| left_sample.timestamp != right_sample.timestamp)
    {
        return Err(CoordinationError::TimestampGridMismatch);
    }
    Ok(())
}

fn validate_timestamps(samples: &[HexapodSample]) -> Result<(), CoordinationError> {
    if samples.is_empty() {
        return Err(CoordinationError::InvalidTraceLength);
    }
    let mut previous = None;
    for sample in samples {
        if !sample.timestamp.is_finite()
            || !sample.x.iter().all(|value| value.is_finite())
            || !sample.y.iter().all(|value| value.is_finite())
        {
            return Err(CoordinationError::InvalidTimestamp);
        }
        if previous.is_some_and(|timestamp| sample.timestamp <= timestamp) {
            return Err(CoordinationError::NonMonotonicTimestamp);
        }
        previous = Some(sample.timestamp);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pulse(leg: LegId, step: usize, delta: f64) -> Intervention {
        Intervention {
            phase_pulse: Some(PhasePulse {
                step,
                leg,
                delta_phase_radians: delta,
            }),
            ..Intervention::default()
        }
    }

    fn run(
        model: CoordinationModel,
        intervention: Intervention,
    ) -> Result<HexapodTrace, CoordinationError> {
        simulate(model, intervention, CoordinationConfig::default())
    }

    #[test]
    fn tripod_default_has_the_declared_alternating_target_pattern() {
        assert_eq!(DEFAULT_TRIPOD_OFFSETS, [0.0, PI, 0.0, PI, 0.0, PI]);
        let trace = run(
            CoordinationModel::IndependentOscillators,
            Intervention::default(),
        )
        .unwrap();
        let error = phase_pattern_rmse(&trace.samples[10], DEFAULT_TRIPOD_OFFSETS, 1.0e-9)
            .unwrap()
            .unwrap();
        let plv = phase_locking_value(&trace.samples[10], DEFAULT_TRIPOD_OFFSETS, 1.0e-9)
            .unwrap()
            .unwrap();
        assert!(error < 1.0e-12);
        assert!((plv - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn independent_oscillators_do_not_recover_a_local_phase_pulse() {
        let intervention = pulse(LegId::LeftFront, 20, 0.8);
        let trace = run(CoordinationModel::IndependentOscillators, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.08, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert_eq!(recovery, None);
        assert!(
            phase_pattern_rmse(
                trace.samples.last().unwrap(),
                DEFAULT_TRIPOD_OFFSETS,
                1.0e-9
            )
            .unwrap()
            .unwrap()
                > 0.25
        );
    }

    #[test]
    fn central_coupling_recovers_a_local_phase_pulse() {
        let intervention = pulse(LegId::LeftFront, 20, 0.8);
        let trace = run(CoordinationModel::CoupledOscillators, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.04, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert!(recovery.is_some_and(|time| time < 1.0));
    }

    #[test]
    fn coupling_ablation_prevents_recovery_after_a_local_phase_pulse() {
        let intervention = Intervention {
            phase_pulse: Some(PhasePulse {
                step: 20,
                leg: LegId::LeftFront,
                delta_phase_radians: 0.8,
            }),
            coupling_cut_from: Some(20),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::CoupledOscillators, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.08, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert_eq!(recovery, None);
    }

    #[test]
    fn sensory_attractor_recovers_a_local_phase_pulse_without_autonomous_oscillation() {
        let intervention = pulse(LegId::LeftFront, 20, 0.8);
        let trace = run(CoordinationModel::FeedbackAttractor, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.04, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert!(recovery.is_some_and(|time| time < 1.0));
    }

    #[test]
    fn sensory_feedback_cut_causes_attractor_amplitude_to_decay() {
        let intervention = Intervention {
            feedback_cut_from: Some(20),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::FeedbackAttractor, intervention).unwrap();
        let final_sample = trace.samples.last().unwrap();
        for (x, y) in final_sample.x.iter().zip(final_sample.y) {
            let amplitude = x.hypot(y);
            assert!(amplitude < 0.01);
        }
    }

    #[test]
    fn feedback_delay_changes_global_phase_lag_without_erasing_tripod_plv() {
        let baseline = run(
            CoordinationModel::FeedbackAttractor,
            Intervention::default(),
        )
        .unwrap();
        let delayed = run(
            CoordinationModel::FeedbackAttractor,
            Intervention {
                feedback_delay: Some(FeedbackDelay {
                    from_step: 20,
                    delay_steps: 6,
                }),
                ..Intervention::default()
            },
        )
        .unwrap();
        let step = 180;
        let reference = step as f64
            * CoordinationConfig::default().sample_interval
            * CoordinationConfig::default().angular_frequency;
        let baseline_lag = global_phase_lag(
            &baseline.samples[step],
            DEFAULT_TRIPOD_OFFSETS,
            reference,
            1.0e-9,
        )
        .unwrap()
        .unwrap();
        let delayed_lag = global_phase_lag(
            &delayed.samples[step],
            DEFAULT_TRIPOD_OFFSETS,
            reference,
            1.0e-9,
        )
        .unwrap()
        .unwrap();
        assert!(wrap_angle(delayed_lag - baseline_lag).abs() > 0.3);
        let plv = phase_locking_value(&delayed.samples[step], DEFAULT_TRIPOD_OFFSETS, 1.0e-9)
            .unwrap()
            .unwrap();
        assert!(plv > 0.99);
    }

    #[test]
    fn coupled_network_maintains_pattern_after_one_leg_becomes_unavailable() {
        let intervention = Intervention {
            leg_failure: Some(LegFailure {
                from_step: 40,
                leg: LegId::LeftFront,
            }),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::CoupledOscillators, intervention).unwrap();
        let final_sample = trace.samples.last().unwrap();
        assert!(!final_sample.active[LegId::LeftFront.index()]);
        let error = phase_pattern_rmse(final_sample, DEFAULT_TRIPOD_OFFSETS, 1.0e-9)
            .unwrap()
            .unwrap();
        assert!(error < 0.01);
    }

    #[test]
    fn coupled_network_rejects_one_leg_frequency_detuning() {
        let intervention = Intervention {
            frequency_detune: Some(FrequencyDetune {
                from_step: 20,
                leg: LegId::RightRear,
                delta_angular_frequency: 0.2,
            }),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::CoupledOscillators, intervention).unwrap();
        let error = phase_pattern_rmse(
            trace.samples.last().unwrap(),
            DEFAULT_TRIPOD_OFFSETS,
            1.0e-9,
        )
        .unwrap()
        .unwrap();
        assert!(error < 0.1);
    }

    #[test]
    fn six_node_hopf_network_recovers_a_suppressed_leg_amplitude() {
        let intervention = Intervention {
            amplitude_pulse: Some(AmplitudePulse {
                step: 20,
                leg: LegId::LeftFront,
                scale: 0.2,
            }),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::CoupledHopfOscillators, intervention).unwrap();
        let at_pulse = &trace.samples[20];
        let pulse_amplitude = at_pulse.x[0].hypot(at_pulse.y[0]);
        assert!((pulse_amplitude - 0.2).abs() < 1.0e-10);

        let final_sample = trace.samples.last().unwrap();
        let final_amplitude = final_sample.x[0].hypot(final_sample.y[0]);
        assert!((final_amplitude - 1.0).abs() < 1.0e-3);
        let pattern_error = phase_pattern_rmse(final_sample, DEFAULT_TRIPOD_OFFSETS, 1.0e-9)
            .unwrap()
            .unwrap();
        assert!(pattern_error < 0.01);
    }

    #[test]
    fn six_node_hopf_network_recovers_a_local_phase_pulse() {
        let intervention = pulse(LegId::RightRear, 20, 0.8);
        let trace = run(CoordinationModel::CoupledHopfOscillators, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.04, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert!(recovery.is_some_and(|time| time < 1.0));
    }

    #[test]
    fn amplitude_pulse_is_rejected_for_models_without_radial_state() {
        let intervention = Intervention {
            amplitude_pulse: Some(AmplitudePulse {
                step: 10,
                leg: LegId::LeftFront,
                scale: 0.5,
            }),
            ..Intervention::default()
        };
        assert_eq!(
            run(CoordinationModel::CoupledOscillators, intervention),
            Err(CoordinationError::AmplitudePulseUnsupported)
        );
    }

    #[test]
    fn hopf_interventions_and_parameters_fail_closed() {
        let config = CoordinationConfig::default();
        assert_eq!(
            simulate(
                CoordinationModel::CoupledHopfOscillators,
                Intervention {
                    amplitude_pulse: Some(AmplitudePulse {
                        step: 10,
                        leg: LegId::LeftMiddle,
                        scale: 0.0,
                    }),
                    ..Intervention::default()
                },
                config
            ),
            Err(CoordinationError::InvalidAmplitudePulse)
        );

        let mut invalid = config;
        invalid.hopf_radial_gain = 0.0;
        assert_eq!(
            simulate(
                CoordinationModel::CoupledHopfOscillators,
                Intervention::default(),
                invalid
            ),
            Err(CoordinationError::InvalidHopfParameters)
        );
    }

    #[test]
    fn hybrid_network_uses_feedback_after_central_coupling_is_cut() {
        let intervention = Intervention {
            phase_pulse: Some(PhasePulse {
                step: 20,
                leg: LegId::LeftFront,
                delta_phase_radians: 0.8,
            }),
            coupling_cut_from: Some(20),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::HybridOscillators, intervention).unwrap();
        let recovery = recovery_time(&trace, 20, 0.08, 3, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap();
        assert!(recovery.is_some());
    }

    #[test]
    fn frozen_forecasts_have_stable_layout_and_separate_local_pulse_predictions() {
        let layout_digest = [0xA5; 32];
        let baseline_action = Intervention::default();
        let independent_baseline =
            run(CoordinationModel::IndependentOscillators, baseline_action).unwrap();
        let coupled_baseline = run(CoordinationModel::CoupledOscillators, baseline_action).unwrap();
        let independent_baseline_forecast = independent_baseline
            .freeze_observable_forecast(1, layout_digest)
            .unwrap();
        let coupled_baseline_forecast = coupled_baseline
            .freeze_observable_forecast(2, layout_digest)
            .unwrap();

        assert_eq!(
            independent_baseline_forecast.observable_layout_digest,
            coupled_baseline_forecast.observable_layout_digest
        );
        assert!(
            independent_baseline_forecast
                .mean
                .iter()
                .zip(&coupled_baseline_forecast.mean)
                .all(|(independent, coupled)| (independent - coupled).abs() < 1.0e-12)
        );

        let phase_pulse = pulse(LegId::LeftFront, 20, 0.8);
        let independent_perturbed =
            run(CoordinationModel::IndependentOscillators, phase_pulse).unwrap();
        let coupled_perturbed = run(CoordinationModel::CoupledOscillators, phase_pulse).unwrap();
        let independent_prediction = independent_perturbed
            .freeze_observable_forecast(1, layout_digest)
            .unwrap();
        let coupled_prediction = coupled_perturbed
            .freeze_observable_forecast(2, layout_digest)
            .unwrap();

        assert_eq!(
            independent_prediction.observable_layout_digest,
            coupled_prediction.observable_layout_digest
        );
        assert_eq!(
            independent_prediction.mean.len(),
            coupled_prediction.mean.len()
        );
        assert_ne!(independent_prediction.mean, coupled_prediction.mean);
    }

    #[test]
    fn frozen_forecasts_omit_failed_leg_observations_and_reject_missing_layout_digest() {
        let intervention = Intervention {
            leg_failure: Some(LegFailure {
                from_step: 10,
                leg: LegId::RightRear,
            }),
            ..Intervention::default()
        };
        let trace = run(CoordinationModel::CoupledOscillators, intervention).unwrap();
        let forecast = trace.freeze_observable_forecast(7, [0xB6; 32]).unwrap();
        assert!(forecast.mean.len() < trace.samples.len() * LEG_COUNT * 2);
        assert_eq!(forecast.observable_layout_digest, [0xB6; 32]);
        assert_eq!(
            trace.freeze_observable_forecast(7, [0; 32]),
            Err(CoordinationError::InvalidObservableLayoutDigest)
        );
    }

    #[test]
    fn paired_metrics_reject_nonmonotonic_or_mismatched_timestamps() {
        let baseline = run(
            CoordinationModel::IndependentOscillators,
            Intervention::default(),
        )
        .unwrap();
        let mut invalid = baseline.samples.clone();
        invalid[10].timestamp = invalid[9].timestamp;
        assert_eq!(
            validate_aligned_traces(&invalid, &baseline.samples),
            Err(CoordinationError::NonMonotonicTimestamp)
        );

        let mut shifted = baseline.samples.clone();
        shifted[10].timestamp += 0.001;
        assert_eq!(
            validate_aligned_traces(&shifted, &baseline.samples),
            Err(CoordinationError::TimestampGridMismatch)
        );
    }

    #[test]
    fn interventions_fail_closed_for_invalid_steps_delay_and_nonfinite_actions() {
        let config = CoordinationConfig::default();
        assert_eq!(
            simulate(
                CoordinationModel::CoupledOscillators,
                Intervention {
                    phase_pulse: Some(PhasePulse {
                        step: config.sample_count,
                        leg: LegId::LeftFront,
                        delta_phase_radians: 0.1,
                    }),
                    ..Intervention::default()
                },
                config
            ),
            Err(CoordinationError::InvalidInterventionStep)
        );
        assert_eq!(
            simulate(
                CoordinationModel::FeedbackAttractor,
                Intervention {
                    feedback_delay: Some(FeedbackDelay {
                        from_step: 0,
                        delay_steps: 0,
                    }),
                    ..Intervention::default()
                },
                config
            ),
            Err(CoordinationError::InvalidFeedbackDelay)
        );
        assert_eq!(
            simulate(
                CoordinationModel::CoupledOscillators,
                Intervention {
                    phase_pulse: Some(PhasePulse {
                        step: 0,
                        leg: LegId::RightFront,
                        delta_phase_radians: f64::NAN,
                    }),
                    ..Intervention::default()
                },
                config
            ),
            Err(CoordinationError::InvalidPhasePulse)
        );
    }

    #[test]
    fn phase_metrics_abstain_when_fewer_than_two_legs_are_observable() {
        let mut sample = HexapodSample {
            timestamp: 0.0,
            x: [1.0; LEG_COUNT],
            y: [0.0; LEG_COUNT],
            active: [false; LEG_COUNT],
        };
        sample.active[LegId::LeftFront.index()] = true;
        assert_eq!(
            phase_pattern_rmse(&sample, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap(),
            None
        );
        assert_eq!(
            phase_locking_value(&sample, DEFAULT_TRIPOD_OFFSETS, 1.0e-9).unwrap(),
            None
        );
    }

    #[test]
    fn rejects_invalid_configuration() {
        let invalid_sample_count = CoordinationConfig {
            sample_count: 1,
            ..CoordinationConfig::default()
        };
        assert_eq!(
            simulate(
                CoordinationModel::IndependentOscillators,
                Intervention::default(),
                invalid_sample_count
            ),
            Err(CoordinationError::InvalidSampleCount)
        );

        let zero_coupling = CoordinationConfig {
            central_coupling_gain: 0.0,
            ..CoordinationConfig::default()
        };
        assert_eq!(
            simulate(
                CoordinationModel::CoupledOscillators,
                Intervention::default(),
                zero_coupling
            ),
            Err(CoordinationError::InvalidCentralCouplingGain)
        );
        assert_eq!(
            simulate(
                CoordinationModel::CoupledHopfOscillators,
                Intervention::default(),
                zero_coupling
            ),
            Err(CoordinationError::InvalidCentralCouplingGain)
        );
    }

    #[test]
    fn all_models_share_the_same_time_grid_and_leg_order() {
        let config = CoordinationConfig::default();
        let independent = simulate(
            CoordinationModel::IndependentOscillators,
            Intervention::default(),
            config,
        )
        .unwrap();
        let coupled = simulate(
            CoordinationModel::CoupledOscillators,
            Intervention::default(),
            config,
        )
        .unwrap();
        validate_aligned_traces(&independent.samples, &coupled.samples).unwrap();
        assert_eq!(LegId::ALL[0], LegId::LeftFront);
        assert_eq!(LegId::ALL[5], LegId::RightRear);
    }
}
