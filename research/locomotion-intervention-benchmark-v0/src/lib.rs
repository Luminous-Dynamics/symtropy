//! Synthetic known-truth benchmark for intervention-based locomotor
//! mechanism discrimination.
//!
//! These toy mechanisms are falsification controls, not biological models.
//! In particular, matching a short periodic trace is deliberately not treated
//! as evidence that the generating system contains an intrinsic oscillator.

use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mechanism {
    IntrinsicOscillator,
    DampedFocus,
    FeedbackAttractor,
    SensoryFeedbackDriven,
    HybridFeedbackCoupled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intervention {
    None,
    FeedbackCutFrom {
        sample_index: usize,
    },
    PhasePulseAt {
        sample_index: usize,
        delta_phase_radians: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationConfig {
    pub sample_count: usize,
    pub sample_interval: f64,
    pub angular_frequency: f64,
    pub initial_phase: f64,
    pub feedback_decay_rate: f64,
    pub feedback_coupling: f64,
    pub observation_variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticSample {
    /// Observable coordinates only. Candidate selection never receives
    /// generator latent phase, amplitude or intervention truth metadata.
    pub timestamp: f64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LatentTruthSample {
    pub phase: f64,
    pub amplitude: f64,
    pub feedback_present: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticTrace {
    pub mechanism: Mechanism,
    pub intervention: Intervention,
    /// Observable-only record. Safe to pass to the candidate scorer.
    pub samples: Vec<SyntheticSample>,
    /// Generator ground truth kept on a separate side channel for test oracles.
    pub latent_truth: Vec<LatentTruthSample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SelectionDecision {
    Selected {
        mechanism: Mechanism,
        log_score: f64,
        margin_over_runner_up: f64,
    },
    Abstained {
        best_candidate: Mechanism,
        margin_over_runner_up: f64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidSampleCount,
    InvalidSampleInterval,
    InvalidAngularFrequency,
    InvalidInitialPhase,
    InvalidFeedbackDecayRate,
    InvalidFeedbackCoupling,
    InvalidObservationVariance,
    InvalidInterventionIndex,
    InvalidPhasePulse,
    InvalidScoreMargin,
    InvalidFitGrid,
    InvalidInterventionAlignment,
    EmptySeedSet,
    DuplicateSeed,
    DuplicateGenerator,
    SingularVectorAr1Fit,
    InvalidTraceLength,
    NonFiniteTrace,
    NonMonotonicTraceTimestamp,
    TimestampMismatch,
    EmptyCandidateSet,
    DuplicateCandidate,
    NumericalFailure,
}

impl SimulationConfig {
    pub fn validate(self) -> Result<(), BenchmarkError> {
        if self.sample_count < 2 {
            return Err(BenchmarkError::InvalidSampleCount);
        }
        if !self.sample_interval.is_finite() || self.sample_interval <= 0.0 {
            return Err(BenchmarkError::InvalidSampleInterval);
        }
        if !self.angular_frequency.is_finite() {
            return Err(BenchmarkError::InvalidAngularFrequency);
        }
        if !self.initial_phase.is_finite() {
            return Err(BenchmarkError::InvalidInitialPhase);
        }
        if !self.feedback_decay_rate.is_finite() || self.feedback_decay_rate <= 0.0 {
            return Err(BenchmarkError::InvalidFeedbackDecayRate);
        }
        if !self.feedback_coupling.is_finite() || self.feedback_coupling < 0.0 {
            return Err(BenchmarkError::InvalidFeedbackCoupling);
        }
        if !self.observation_variance.is_finite() || self.observation_variance <= 0.0 {
            return Err(BenchmarkError::InvalidObservationVariance);
        }
        Ok(())
    }
}

/// Simulate one synthetic mechanism under one declared intervention.
///
/// IntrinsicOscillator is a stable unit-amplitude limit-cycle surrogate.
/// DampedFocus is a transient rotating focus whose amplitude decays.
/// FeedbackAttractor is a stable state tracking a periodic external target;
/// without feedback it relaxes to the origin.
/// SensoryFeedbackDriven directly follows a periodic external drive and stops
/// cycling when that feedback is removed.
/// HybridFeedbackCoupled has an intrinsic phase plus Kuramoto-style sensory
/// phase coupling. These deliberately simple generators provide known-truth
/// controls; they are not asserted to reproduce any animal's neural circuit.
pub fn simulate(
    mechanism: Mechanism,
    intervention: Intervention,
    config: SimulationConfig,
) -> Result<SyntheticTrace, BenchmarkError> {
    config.validate()?;
    validate_intervention(intervention, config)?;

    if mechanism == Mechanism::FeedbackAttractor && config.feedback_coupling <= 0.0 {
        return Err(BenchmarkError::InvalidFeedbackCoupling);
    }

    let mut phase = config.initial_phase;
    let mut amplitude = 1.0_f64;
    let mut attractor_state = [config.initial_phase.cos(), config.initial_phase.sin()];
    let mut samples = Vec::with_capacity(config.sample_count);
    let mut latent_truth = Vec::with_capacity(config.sample_count);
    let mut previous_time = 0.0;

    for sample_index in 0..config.sample_count {
        let timestamp = sample_index as f64 * config.sample_interval;
        if !timestamp.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        let feedback_present = match intervention {
            Intervention::FeedbackCutFrom {
                sample_index: cut_index,
            } => sample_index < cut_index,
            _ => true,
        };

        if sample_index > 0 {
            let elapsed = timestamp - previous_time;
            match mechanism {
                Mechanism::IntrinsicOscillator => {
                    phase += config.angular_frequency * elapsed;
                }
                Mechanism::DampedFocus => {
                    phase += config.angular_frequency * elapsed;
                    amplitude *= (-config.feedback_decay_rate * elapsed).exp();
                }
                Mechanism::FeedbackAttractor => {
                    let drive_phase = config.initial_phase + config.angular_frequency * timestamp;
                    let relaxation = (-config.feedback_coupling * elapsed).exp();
                    let drive = [drive_phase.cos(), drive_phase.sin()];
                    if feedback_present {
                        attractor_state[0] =
                            relaxation * attractor_state[0] + (1.0 - relaxation) * drive[0];
                        attractor_state[1] =
                            relaxation * attractor_state[1] + (1.0 - relaxation) * drive[1];
                    } else {
                        let decay = (-config.feedback_decay_rate * elapsed).exp();
                        attractor_state[0] *= decay;
                        attractor_state[1] *= decay;
                    }
                    phase = attractor_state[1].atan2(attractor_state[0]);
                    amplitude = attractor_state[0].hypot(attractor_state[1]);
                }
                Mechanism::SensoryFeedbackDriven => {
                    if feedback_present {
                        phase = config.initial_phase + config.angular_frequency * timestamp;
                        amplitude = 1.0;
                    } else {
                        amplitude *= (-config.feedback_decay_rate * elapsed).exp();
                    }
                }
                Mechanism::HybridFeedbackCoupled => {
                    phase += config.angular_frequency * elapsed;
                    if feedback_present {
                        let drive_phase =
                            config.initial_phase + config.angular_frequency * timestamp;
                        let phase_error = wrap_angle(drive_phase - phase);
                        phase += config.feedback_coupling * phase_error.sin() * elapsed;
                    }
                }
            }
        }

        if mechanism == Mechanism::SensoryFeedbackDriven && feedback_present {
            phase = config.initial_phase + config.angular_frequency * timestamp;
            amplitude = 1.0;
        }

        if let Intervention::PhasePulseAt {
            sample_index: pulse_index,
            delta_phase_radians,
        } = intervention
            && sample_index == pulse_index
        {
            if mechanism == Mechanism::FeedbackAttractor {
                let (sin_phase, cos_phase) = delta_phase_radians.sin_cos();
                attractor_state = [
                    cos_phase * attractor_state[0] - sin_phase * attractor_state[1],
                    sin_phase * attractor_state[0] + cos_phase * attractor_state[1],
                ];
                phase = attractor_state[1].atan2(attractor_state[0]);
                amplitude = attractor_state[0].hypot(attractor_state[1]);
            } else {
                phase += delta_phase_radians;
            }
        }

        let (x, y) = if mechanism == Mechanism::FeedbackAttractor {
            (attractor_state[0], attractor_state[1])
        } else {
            (amplitude * phase.cos(), amplitude * phase.sin())
        };
        if !phase.is_finite() || !amplitude.is_finite() || !x.is_finite() || !y.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        samples.push(SyntheticSample { timestamp, x, y });
        latent_truth.push(LatentTruthSample {
            phase,
            amplitude,
            feedback_present,
        });
        previous_time = timestamp;
    }

    Ok(SyntheticTrace {
        mechanism,
        intervention,
        samples,
        latent_truth,
    })
}

fn validate_intervention(
    intervention: Intervention,
    config: SimulationConfig,
) -> Result<(), BenchmarkError> {
    match intervention {
        Intervention::None => Ok(()),
        Intervention::FeedbackCutFrom { sample_index } => {
            if sample_index >= config.sample_count {
                Err(BenchmarkError::InvalidInterventionIndex)
            } else {
                Ok(())
            }
        }
        Intervention::PhasePulseAt {
            sample_index,
            delta_phase_radians,
        } => {
            if sample_index >= config.sample_count {
                return Err(BenchmarkError::InvalidInterventionIndex);
            }
            if !delta_phase_radians.is_finite() {
                return Err(BenchmarkError::InvalidPhasePulse);
            }
            Ok(())
        }
    }
}

pub fn trajectory_rmse(
    left: &[SyntheticSample],
    right: &[SyntheticSample],
) -> Result<f64, BenchmarkError> {
    if left.len() != right.len() || left.is_empty() {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    validate_trace(left)?;
    validate_trace(right)?;
    validate_aligned_timestamps(left, right)?;

    let squared_error = left
        .iter()
        .zip(right)
        .map(|(left_sample, right_sample)| {
            (left_sample.x - right_sample.x).powi(2) + (left_sample.y - right_sample.y).powi(2)
        })
        .sum::<f64>();
    let rmse = (squared_error / (2.0 * left.len() as f64)).sqrt();
    if rmse.is_finite() {
        Ok(rmse)
    } else {
        Err(BenchmarkError::NumericalFailure)
    }
}

/// Return the wrapped observed phase offset from a matched baseline trace.
/// Returns None when either trace has effectively zero observable amplitude.
pub fn phase_offset(
    trace: &[SyntheticSample],
    baseline: &[SyntheticSample],
    sample_index: usize,
) -> Result<Option<f64>, BenchmarkError> {
    if trace.len() != baseline.len() || sample_index >= trace.len() {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    validate_trace(trace)?;
    validate_trace(baseline)?;
    validate_aligned_timestamps(trace, baseline)?;
    let sample = trace[sample_index];
    let reference = baseline[sample_index];
    let amplitude = sample.x.hypot(sample.y);
    let reference_amplitude = reference.x.hypot(reference.y);
    if amplitude <= 1.0e-12 || reference_amplitude <= 1.0e-12 {
        return Ok(None);
    }
    let angle = sample.y.atan2(sample.x);
    let reference_angle = reference.y.atan2(reference.x);
    Ok(Some(wrap_angle(angle - reference_angle)))
}

/// Independent two-coordinate Gaussian log score with a declared observation
/// variance. The score is comparable only when all candidates use the same
/// observation model and variance.
pub fn predictive_log_score(
    observed: &[SyntheticSample],
    predicted: &[SyntheticSample],
    observation_variance: f64,
) -> Result<f64, BenchmarkError> {
    if observed.len() != predicted.len() || observed.is_empty() {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    if !observation_variance.is_finite() || observation_variance <= 0.0 {
        return Err(BenchmarkError::InvalidObservationVariance);
    }
    validate_trace(observed)?;
    validate_trace(predicted)?;
    validate_aligned_timestamps(observed, predicted)?;

    let normalization = (2.0 * std::f64::consts::PI * observation_variance).ln();
    let score = observed
        .iter()
        .zip(predicted)
        .map(|(actual, forecast)| {
            let dx = actual.x - forecast.x;
            let dy = actual.y - forecast.y;
            -0.5 * (2.0 * normalization + (dx * dx + dy * dy) / observation_variance)
        })
        .sum::<f64>();
    if score.is_finite() {
        Ok(score)
    } else {
        Err(BenchmarkError::NumericalFailure)
    }
}

/// Add deterministic, seeded, independent Gaussian observation noise to the
/// observable coordinates. Latent truth is not accepted by or returned from
/// this function, preserving the observation/truth boundary.
pub fn add_gaussian_observation_noise(
    observed: &[SyntheticSample],
    observation_noise_variance: f64,
    seed: u64,
) -> Result<Vec<SyntheticSample>, BenchmarkError> {
    validate_trace(observed)?;
    if !observation_noise_variance.is_finite() || observation_noise_variance < 0.0 {
        return Err(BenchmarkError::InvalidObservationVariance);
    }
    if observation_noise_variance == 0.0 {
        return Ok(observed.to_vec());
    }

    let standard_deviation = observation_noise_variance.sqrt();
    let mut random = SeededNormal::new(seed);
    let mut noisy = Vec::with_capacity(observed.len());
    for sample in observed {
        let x = sample.x + standard_deviation * random.standard_normal();
        let y = sample.y + standard_deviation * random.standard_normal();
        if !x.is_finite() || !y.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        noisy.push(SyntheticSample {
            timestamp: sample.timestamp,
            x,
            y,
        });
    }
    Ok(noisy)
}

struct SeededNormal {
    state: u64,
    spare: Option<f64>,
}

impl SeededNormal {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / ((1_u64 << 53) as f64)
    }

    fn standard_normal(&mut self) -> f64 {
        if let Some(spare) = self.spare.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = std::f64::consts::TAU * self.uniform_open();
        let (sin_angle, cos_angle) = angle.sin_cos();
        self.spare = Some(radius * sin_angle);
        radius * cos_angle
    }
}

/// Equal-budget grid used to fit baseline angular frequency and initial phase.
/// Feedback decay and coupling stay frozen in the shared SimulationConfig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyPhaseGrid {
    pub minimum_angular_frequency: f64,
    pub maximum_angular_frequency: f64,
    /// Number of equally spaced frequencies, including both endpoints.
    pub frequency_steps: usize,
    /// Number of equally spaced phases over [-pi, pi).
    pub phase_steps: usize,
}

impl FrequencyPhaseGrid {
    pub fn validate(self) -> Result<(), BenchmarkError> {
        if !self.minimum_angular_frequency.is_finite()
            || !self.maximum_angular_frequency.is_finite()
            || self.minimum_angular_frequency >= self.maximum_angular_frequency
            || self.frequency_steps < 2
            || self.phase_steps < 4
        {
            return Err(BenchmarkError::InvalidFitGrid);
        }
        Ok(())
    }
}

/// Outcome-blind forecast produced from baseline observations only.
#[derive(Debug, Clone, PartialEq)]
pub struct FrozenForecast {
    pub mechanism: Mechanism,
    pub fitted_angular_frequency: f64,
    pub fitted_initial_phase: f64,
    pub baseline_log_score: f64,
    /// Gap between the best and second-best parameter-grid scores. Zero
    /// signals a tie/aliasing ambiguity at the selected grid resolution.
    pub baseline_parameter_score_margin: f64,
    /// Only samples at and after the declared intervention index.
    pub predicted_suffix: Vec<SyntheticSample>,
}

/// Fit the same two baseline parameters for each candidate over the same
/// prefix and fixed search budget, then freeze its response to one action.
/// The action index must equal prefix.len(); held-out outcomes are not inputs.
pub fn predict_intervention_candidates(
    observed_prefix: &[SyntheticSample],
    candidates: &[Mechanism],
    intervention: Intervention,
    base_config: SimulationConfig,
    grid: FrequencyPhaseGrid,
) -> Result<Vec<FrozenForecast>, BenchmarkError> {
    base_config.validate()?;
    grid.validate()?;
    validate_trace(observed_prefix)?;

    if observed_prefix.len() < 2 || observed_prefix.len() >= base_config.sample_count {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    for (index, sample) in observed_prefix.iter().enumerate() {
        let expected_timestamp = index as f64 * base_config.sample_interval;
        if sample.timestamp != expected_timestamp {
            return Err(BenchmarkError::TimestampMismatch);
        }
    }

    let intervention_index = match intervention {
        Intervention::FeedbackCutFrom { sample_index }
        | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
        Intervention::None => return Err(BenchmarkError::InvalidInterventionAlignment),
    };
    if intervention_index != observed_prefix.len() {
        return Err(BenchmarkError::InvalidInterventionAlignment);
    }
    validate_intervention(intervention, base_config)?;

    if candidates.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(*candidate) {
            return Err(BenchmarkError::DuplicateCandidate);
        }
    }

    let mut frozen = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let mut best: Option<(SimulationConfig, f64)> = None;
        let mut second_best_score = f64::NEG_INFINITY;

        for frequency_index in 0..grid.frequency_steps {
            let frequency_fraction = frequency_index as f64 / (grid.frequency_steps - 1) as f64;
            let frequency = grid.minimum_angular_frequency
                + (grid.maximum_angular_frequency - grid.minimum_angular_frequency)
                    * frequency_fraction;

            for phase_index in 0..grid.phase_steps {
                let phase = -std::f64::consts::PI
                    + std::f64::consts::TAU * phase_index as f64 / grid.phase_steps as f64;
                let fit_config = SimulationConfig {
                    angular_frequency: frequency,
                    initial_phase: phase,
                    ..base_config
                };
                let baseline_prediction = simulate(*candidate, Intervention::None, fit_config)?;
                let score = predictive_log_score(
                    observed_prefix,
                    &baseline_prediction.samples[..observed_prefix.len()],
                    base_config.observation_variance,
                )?;

                if best.is_none_or(|(_, best_score)| score > best_score) {
                    if let Some((_, previous_best_score)) = best {
                        second_best_score = previous_best_score;
                    }
                    best = Some((fit_config, score));
                } else if score > second_best_score {
                    second_best_score = score;
                }
            }
        }

        let (fitted_config, baseline_log_score) = best.ok_or(BenchmarkError::NumericalFailure)?;
        let parameter_margin = if second_best_score.is_finite() {
            (baseline_log_score - second_best_score).max(0.0)
        } else {
            0.0
        };

        // No held-out outcome is available during this fit/forecast operation.
        let action_prediction = simulate(*candidate, intervention, fitted_config)?;
        let predicted_suffix = action_prediction.samples[observed_prefix.len()..].to_vec();
        if predicted_suffix.is_empty() {
            return Err(BenchmarkError::InvalidTraceLength);
        }

        frozen.push(FrozenForecast {
            mechanism: *candidate,
            fitted_angular_frequency: fitted_config.angular_frequency,
            fitted_initial_phase: fitted_config.initial_phase,
            baseline_log_score,
            baseline_parameter_score_margin: parameter_margin,
            predicted_suffix,
        });
    }
    Ok(frozen)
}

/// Score already-frozen predictions against a held-out post-intervention
/// suffix. Kept separate from fitting so outcomes cannot tune the baseline fit.
pub fn select_frozen_forecast(
    observed_suffix: &[SyntheticSample],
    forecasts: &[FrozenForecast],
    observation_variance: f64,
    minimum_score_margin: f64,
) -> Result<SelectionDecision, BenchmarkError> {
    if forecasts.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if !minimum_score_margin.is_finite() || minimum_score_margin < 0.0 {
        return Err(BenchmarkError::InvalidScoreMargin);
    }

    let mut seen = BTreeSet::new();
    let mut scores = Vec::with_capacity(forecasts.len());
    for forecast in forecasts {
        if !seen.insert(forecast.mechanism) {
            return Err(BenchmarkError::DuplicateCandidate);
        }
        let score = predictive_log_score(
            observed_suffix,
            &forecast.predicted_suffix,
            observation_variance,
        )?;
        scores.push((forecast.mechanism, score));
    }
    scores.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let best = scores[0];
    let runner_up = scores[1];
    let margin = best.1 - runner_up.1;
    if !margin.is_finite() {
        return Err(BenchmarkError::NumericalFailure);
    }
    if margin <= minimum_score_margin {
        Ok(SelectionDecision::Abstained {
            best_candidate: best.0,
            margin_over_runner_up: margin,
        })
    } else {
        Ok(SelectionDecision::Selected {
            mechanism: best.0,
            log_score: best.1,
            margin_over_runner_up: margin,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]


/// Candidate identity for a mechanistic generator or a purely predictive
/// baseline. A selected VAR(1) means predictive equivalence, not a biological
/// oscillator or other neural mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidateFamily {
    Mechanistic(Mechanism),
    VectorAr1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorAr1Forecast {
    pub transition: [[f64; 2]; 2],
    pub intercept: [f64; 2],
    pub baseline_log_score: f64,
    /// Action-blind autoregressive continuation, frozen at the action index.
    pub predicted_suffix: Vec<SyntheticSample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CompetingModelDecision {
    Selected {
        family: CandidateFamily,
        log_score: f64,
        margin_over_runner_up: f64,
    },
    Abstained {
        best_candidate: CandidateFamily,
        margin_over_runner_up: f64,
    },
}

/// Fit a bivariate VAR(1) with intercept using only the pre-action prefix.
/// The model then continues its fitted recurrence without representing the
/// intervention. This is an intentionally strong predictive baseline, not a
/// mechanistic claim.
pub fn fit_vector_ar1_forecast(
    observed_prefix: &[SyntheticSample],
    intervention: Intervention,
    base_config: SimulationConfig,
) -> Result<VectorAr1Forecast, BenchmarkError> {
    base_config.validate()?;
    validate_trace(observed_prefix)?;
    if observed_prefix.len() < 3 || observed_prefix.len() >= base_config.sample_count {
        return Err(BenchmarkError::InvalidTraceLength);
    }

    for (index, sample) in observed_prefix.iter().enumerate() {
        let expected_timestamp = index as f64 * base_config.sample_interval;
        if sample.timestamp != expected_timestamp {
            return Err(BenchmarkError::TimestampMismatch);
        }
    }
    let action_index = match intervention {
        Intervention::FeedbackCutFrom { sample_index }
        | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
        Intervention::None => return Err(BenchmarkError::InvalidInterventionAlignment),
    };
    if action_index != observed_prefix.len() {
        return Err(BenchmarkError::InvalidInterventionAlignment);
    }
    validate_intervention(intervention, base_config)?;

    let pair_count = (observed_prefix.len() - 1) as f64;
    let mut previous_mean = [0.0; 2];
    let mut next_mean = [0.0; 2];
    for pair_index in 0..(observed_prefix.len() - 1) {
        let previous = observed_prefix[pair_index];
        let next = observed_prefix[pair_index + 1];
        previous_mean[0] += previous.x;
        previous_mean[1] += previous.y;
        next_mean[0] += next.x;
        next_mean[1] += next.y;
    }
    for coordinate in 0..2 {
        previous_mean[coordinate] /= pair_count;
        next_mean[coordinate] /= pair_count;
    }

    let mut covariance_previous = [[0.0; 2]; 2];
    let mut cross_covariance = [[0.0; 2]; 2];
    for pair_index in 0..(observed_prefix.len() - 1) {
        let previous = [
            observed_prefix[pair_index].x - previous_mean[0],
            observed_prefix[pair_index].y - previous_mean[1],
        ];
        let next = [
            observed_prefix[pair_index + 1].x - next_mean[0],
            observed_prefix[pair_index + 1].y - next_mean[1],
        ];
        for row in 0..2 {
            for column in 0..2 {
                covariance_previous[row][column] += previous[row] * previous[column];
                cross_covariance[row][column] += next[row] * previous[column];
            }
        }
    }

    let determinant = covariance_previous[0][0] * covariance_previous[1][1]
        - covariance_previous[0][1] * covariance_previous[1][0];
    if !determinant.is_finite() || determinant <= 1.0e-12 {
        return Err(BenchmarkError::SingularVectorAr1Fit);
    }
    let inverse = [
        [
            covariance_previous[1][1] / determinant,
            -covariance_previous[0][1] / determinant,
        ],
        [
            -covariance_previous[1][0] / determinant,
            covariance_previous[0][0] / determinant,
        ],
    ];
    let transition = [
        [
            cross_covariance[0][0] * inverse[0][0]
                + cross_covariance[0][1] * inverse[1][0],
            cross_covariance[0][0] * inverse[0][1]
                + cross_covariance[0][1] * inverse[1][1],
        ],
        [
            cross_covariance[1][0] * inverse[0][0]
                + cross_covariance[1][1] * inverse[1][0],
            cross_covariance[1][0] * inverse[0][1]
                + cross_covariance[1][1] * inverse[1][1],
        ],
    ];
    let intercept = [
        next_mean[0]
            - transition[0][0] * previous_mean[0]
            - transition[0][1] * previous_mean[1],
        next_mean[1]
            - transition[1][0] * previous_mean[0]
            - transition[1][1] * previous_mean[1],
    ];
    if !transition.iter().flatten().all(|value| value.is_finite())
        || !intercept.iter().all(|value| value.is_finite())
    {
        return Err(BenchmarkError::NumericalFailure);
    }

    let mut training_prediction = Vec::with_capacity(observed_prefix.len() - 1);
    for index in 1..observed_prefix.len() {
        let previous = observed_prefix[index - 1];
        let prediction = apply_vector_ar1(
            transition,
            intercept,
            [previous.x, previous.y],
        );
        training_prediction.push(SyntheticSample {
            timestamp: observed_prefix[index].timestamp,
            x: prediction[0],
            y: prediction[1],
        });
    }
    let baseline_log_score = predictive_log_score(
        &observed_prefix[1..],
        &training_prediction,
        base_config.observation_variance,
    )?;

    let mut state = [
        observed_prefix.last().unwrap().x,
        observed_prefix.last().unwrap().y,
    ];
    let mut predicted_suffix = Vec::with_capacity(base_config.sample_count - action_index);
    for index in action_index..base_config.sample_count {
        state = apply_vector_ar1(transition, intercept, state);
        if !state.iter().all(|value| value.is_finite()) {
            return Err(BenchmarkError::NumericalFailure);
        }
        predicted_suffix.push(SyntheticSample {
            timestamp: index as f64 * base_config.sample_interval,
            x: state[0],
            y: state[1],
        });
    }

    Ok(VectorAr1Forecast {
        transition,
        intercept,
        baseline_log_score,
        predicted_suffix,
    })
}

fn apply_vector_ar1(
    transition: [[f64; 2]; 2],
    intercept: [f64; 2],
    state: [f64; 2],
) -> [f64; 2] {
    [
        transition[0][0] * state[0] + transition[0][1] * state[1] + intercept[0],
        transition[1][0] * state[0] + transition[1][1] * state[1] + intercept[1],
    ]
}

/// Compare frozen mechanistic forecasts against one fitted vector-AR(1)
/// predictor on the same held-out suffix and common likelihood.
pub fn select_competing_models(
    observed_suffix: &[SyntheticSample],
    mechanistic_forecasts: &[FrozenForecast],
    vector_ar1_forecast: &VectorAr1Forecast,
    observation_variance: f64,
    minimum_score_margin: f64,
) -> Result<CompetingModelDecision, BenchmarkError> {
    if mechanistic_forecasts.is_empty() {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if !minimum_score_margin.is_finite() || minimum_score_margin < 0.0 {
        return Err(BenchmarkError::InvalidScoreMargin);
    }

    let mut seen = BTreeSet::new();
    let mut scores = Vec::with_capacity(mechanistic_forecasts.len() + 1);
    for forecast in mechanistic_forecasts {
        if !seen.insert(forecast.mechanism) {
            return Err(BenchmarkError::DuplicateCandidate);
        }
        let score = predictive_log_score(
            observed_suffix,
            &forecast.predicted_suffix,
            observation_variance,
        )?;
        scores.push((CandidateFamily::Mechanistic(forecast.mechanism), score));
    }
    let ar1_score = predictive_log_score(
        observed_suffix,
        &vector_ar1_forecast.predicted_suffix,
        observation_variance,
    )?;
    scores.push((CandidateFamily::VectorAr1, ar1_score));

    scores.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let best = scores[0];
    let runner_up = scores[1];
    let margin = best.1 - runner_up.1;
    if !margin.is_finite() {
        return Err(BenchmarkError::NumericalFailure);
    }
    if margin <= minimum_score_margin {
        Ok(CompetingModelDecision::Abstained {
            best_candidate: best.0,
            margin_over_runner_up: margin,
        })
    } else {
        Ok(CompetingModelDecision::Selected {
            family: best.0,
            log_score: best.1,
            margin_over_runner_up: margin,
        })
    }
}

pub struct MonteCarloSpec {
    pub generating_mechanisms: Vec<Mechanism>,
    pub candidates: Vec<Mechanism>,
    /// Unique deterministic seeds paired across all generating mechanisms.
    pub seeds: Vec<u64>,
    pub intervention: Intervention,
    pub base_config: SimulationConfig,
    pub fit_grid: FrequencyPhaseGrid,
    pub observation_noise_variance: f64,
    pub minimum_score_margin: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinomialInterval {
    pub successes: usize,
    pub trials: usize,
    pub rate: f64,
    pub lower_95: f64,
    pub upper_95: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MonteCarloSummary {
    pub generating_mechanism: Mechanism,
    pub trials: usize,
    pub selected_count: usize,
    pub correct_count: usize,
    pub incorrect_confident_count: usize,
    pub abstention_count: usize,
    /// Conditional accuracy among non-abstained trials.
    pub accuracy_when_selected_95: Option<BinomialInterval>,
    /// Incorrect confident selections divided by all trials.
    pub false_confident_rate_95: BinomialInterval,
    /// Abstentions divided by all trials.
    pub abstention_rate_95: BinomialInterval,
    pub mean_score_margin: f64,
}

/// Run paired-seed synthetic trials. Each generator is evaluated with the
/// same seeds, parameter grid, candidate set, action, prefix length and noise
/// variance. Per-trial parameters are fitted only to the pre-action prefix;
/// the suffix is scored after the forecasts have been frozen.
pub fn run_paired_seed_evaluation(
    spec: MonteCarloSpec,
) -> Result<Vec<MonteCarloSummary>, BenchmarkError> {
    spec.base_config.validate()?;
    spec.fit_grid.validate()?;
    if spec.generating_mechanisms.is_empty() {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if spec.candidates.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if spec.seeds.is_empty() {
        return Err(BenchmarkError::EmptySeedSet);
    }
    if !spec.observation_noise_variance.is_finite() || spec.observation_noise_variance < 0.0 {
        return Err(BenchmarkError::InvalidObservationVariance);
    }
    if !spec.minimum_score_margin.is_finite() || spec.minimum_score_margin < 0.0 {
        return Err(BenchmarkError::InvalidScoreMargin);
    }

    let mut unique_candidates = BTreeSet::new();
    for candidate in &spec.candidates {
        if !unique_candidates.insert(*candidate) {
            return Err(BenchmarkError::DuplicateCandidate);
        }
    }

    let mut unique_generators = BTreeSet::new();
    for mechanism in &spec.generating_mechanisms {
        if !unique_generators.insert(*mechanism) {
            return Err(BenchmarkError::DuplicateGenerator);
        }
    }

    let mut unique_seeds = BTreeSet::new();
    for seed in &spec.seeds {
        if !unique_seeds.insert(*seed) {
            return Err(BenchmarkError::DuplicateSeed);
        }
    }

    let mut summaries = Vec::with_capacity(spec.generating_mechanisms.len());
    for generating_mechanism in &spec.generating_mechanisms {
        let mut correct_count = 0;
        let mut incorrect_confident_count = 0;
        let mut abstention_count = 0;
        let mut score_margin_sum = 0.0;

        for seed in &spec.seeds {
            let generated = simulate(*generating_mechanism, spec.intervention, spec.base_config)?;
            let noisy_observations = add_gaussian_observation_noise(
                &generated.samples,
                spec.observation_noise_variance,
                *seed,
            )?;
            let prefix_len = match spec.intervention {
                Intervention::FeedbackCutFrom { sample_index }
                | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
                Intervention::None => {
                    return Err(BenchmarkError::InvalidInterventionAlignment);
                }
            };
            let forecasts = predict_intervention_candidates(
                &noisy_observations[..prefix_len],
                &spec.candidates,
                spec.intervention,
                spec.base_config,
                spec.fit_grid,
            )?;
            let decision = select_frozen_forecast(
                &noisy_observations[prefix_len..],
                &forecasts,
                spec.base_config.observation_variance,
                spec.minimum_score_margin,
            )?;

            match decision {
                SelectionDecision::Selected {
                    mechanism,
                    margin_over_runner_up,
                    ..
                } => {
                    score_margin_sum += margin_over_runner_up;
                    if mechanism == *generating_mechanism {
                        correct_count += 1;
                    } else {
                        incorrect_confident_count += 1;
                    }
                }
                SelectionDecision::Abstained {
                    margin_over_runner_up,
                    ..
                } => {
                    score_margin_sum += margin_over_runner_up;
                    abstention_count += 1;
                }
            }
        }

        let trials = spec.seeds.len();
        let selected_count = correct_count + incorrect_confident_count;
        let accuracy_when_selected_95 = if selected_count > 0 {
            Some(wilson_interval(correct_count, selected_count))
        } else {
            None
        };
        summaries.push(MonteCarloSummary {
            generating_mechanism: *generating_mechanism,
            trials,
            selected_count,
            correct_count,
            incorrect_confident_count,
            abstention_count,
            accuracy_when_selected_95,
            false_confident_rate_95: wilson_interval(incorrect_confident_count, trials),
            abstention_rate_95: wilson_interval(abstention_count, trials),
            mean_score_margin: score_margin_sum / trials as f64,
        });
    }
    Ok(summaries)
}

fn wilson_interval(successes: usize, trials: usize) -> BinomialInterval {
    // Wilson score interval with z = 1.959963984540054 (two-sided 95%).
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z_squared = z * z;
    let denominator = 1.0 + z_squared / n;
    let center = (p + z_squared / (2.0 * n)) / denominator;
    let half_width = z * ((p * (1.0 - p) / n + z_squared / (4.0 * n * n)).sqrt()) / denominator;
    BinomialInterval {
        successes,
        trials,
        rate: p,
        lower_95: (center - half_width).max(0.0),
        upper_95: (center + half_width).min(1.0),
    }
}

/// Compare candidate mechanisms on the same observations and intervention.
/// A score margin below the caller's threshold yields abstention rather than
/// forcing a winner among observationally equivalent or weakly identified
/// mechanisms.
pub fn select_mechanism(
    observed: &[SyntheticSample],
    candidates: &[Mechanism],
    intervention: Intervention,
    config: SimulationConfig,
    minimum_score_margin: f64,
) -> Result<SelectionDecision, BenchmarkError> {
    if candidates.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if !minimum_score_margin.is_finite() || minimum_score_margin < 0.0 {
        return Err(BenchmarkError::InvalidScoreMargin);
    }

    let mut seen = BTreeSet::new();
    for candidate in candidates {
        if !seen.insert(*candidate) {
            return Err(BenchmarkError::DuplicateCandidate);
        }
    }

    let mut scores = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let prediction = simulate(*candidate, intervention, config)?;
        let score =
            predictive_log_score(observed, &prediction.samples, config.observation_variance)?;
        scores.push((*candidate, score));
    }
    scores.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let best = scores[0];
    let runner_up = scores[1];
    let margin = best.1 - runner_up.1;
    if !margin.is_finite() {
        return Err(BenchmarkError::NumericalFailure);
    }
    if margin <= minimum_score_margin {
        Ok(SelectionDecision::Abstained {
            best_candidate: best.0,
            margin_over_runner_up: margin,
        })
    } else {
        Ok(SelectionDecision::Selected {
            mechanism: best.0,
            log_score: best.1,
            margin_over_runner_up: margin,
        })
    }
}

fn validate_trace(trace: &[SyntheticSample]) -> Result<(), BenchmarkError> {
    let mut previous_timestamp = None;
    for sample in trace {
        if !sample.timestamp.is_finite() || !sample.x.is_finite() || !sample.y.is_finite() {
            return Err(BenchmarkError::NonFiniteTrace);
        }
        if previous_timestamp.is_some_and(|previous| sample.timestamp <= previous) {
            return Err(BenchmarkError::NonMonotonicTraceTimestamp);
        }
        previous_timestamp = Some(sample.timestamp);
    }
    Ok(())
}

fn validate_aligned_timestamps(
    observed: &[SyntheticSample],
    predicted: &[SyntheticSample],
) -> Result<(), BenchmarkError> {
    if observed.len() != predicted.len() {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    if observed
        .iter()
        .zip(predicted)
        .any(|(left, right)| left.timestamp != right.timestamp)
    {
        return Err(BenchmarkError::TimestampMismatch);
    }
    Ok(())
}

fn wrap_angle(angle: f64) -> f64 {
    angle.sin().atan2(angle.cos())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANDIDATES: [Mechanism; 2] = [
        Mechanism::IntrinsicOscillator,
        Mechanism::SensoryFeedbackDriven,
    ];

    fn config() -> SimulationConfig {
        SimulationConfig {
            sample_count: 80,
            sample_interval: 0.05,
            angular_frequency: 4.0,
            initial_phase: 0.37,
            feedback_decay_rate: 3.0,
            feedback_coupling: 4.0,
            observation_variance: 0.01,
        }
    }

    #[test]
    fn observationally_matched_oscillator_and_feedback_models_are_indistinguishable() {
        let oscillator =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let feedback = simulate(
            Mechanism::SensoryFeedbackDriven,
            Intervention::None,
            config(),
        )
        .unwrap();
        assert!(trajectory_rmse(&oscillator.samples, &feedback.samples).unwrap() < 1.0e-12);
    }

    #[test]
    fn selector_abstains_when_baseline_observations_do_not_identify_mechanism() {
        let observed =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let decision = select_mechanism(
            &observed.samples,
            &CANDIDATES,
            Intervention::None,
            config(),
            1.0e-6,
        )
        .unwrap();
        assert!(matches!(decision, SelectionDecision::Abstained { .. }));
    }

    #[test]
    fn feedback_cut_separates_intrinsic_oscillator_from_sensory_driven_rhythm() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let oscillator = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let feedback = simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        assert!((oscillator.latent_truth.last().unwrap().amplitude - 1.0).abs() < 1.0e-12);
        assert!(feedback.latent_truth.last().unwrap().amplitude < 0.01);
        assert!(trajectory_rmse(&oscillator.samples, &feedback.samples).unwrap() > 0.1);
    }

    #[test]
    fn feedback_attractor_tracks_drive_but_relaxes_without_sensory_input() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let trace = simulate(Mechanism::FeedbackAttractor, intervention, config()).unwrap();
        assert!(trace.latent_truth[10].amplitude > 0.5);
        assert!(trace.latent_truth[20].amplitude > 0.5);
        assert!(trace.latent_truth.last().unwrap().amplitude < 0.01);
        assert!(trace.samples[10].x.is_finite());
        assert!(trace.samples[10].y.is_finite());
    }

    #[test]
    fn feedback_attractor_phase_pulse_is_relaxed_toward_external_drive() {
        let pulse = Intervention::PhasePulseAt {
            sample_index: 20,
            delta_phase_radians: 0.6,
        };
        let baseline =
            simulate(Mechanism::FeedbackAttractor, Intervention::None, config()).unwrap();
        let perturbed = simulate(Mechanism::FeedbackAttractor, pulse, config()).unwrap();
        let offset = phase_offset(
            &perturbed.samples,
            &baseline.samples,
            config().sample_count - 1,
        )
        .unwrap()
        .unwrap();
        assert!(offset.abs() < 0.02);
    }

    #[test]
    fn held_out_feedback_cut_selects_the_intrinsic_oscillator_when_it_generated_data() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let observed = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let decision = select_mechanism(
            &observed.samples,
            &CANDIDATES,
            intervention,
            config(),
            1.0e-6,
        )
        .unwrap();
        assert!(matches!(
            decision,
            SelectionDecision::Selected {
                mechanism: Mechanism::IntrinsicOscillator,
                ..
            }
        ));
    }

    #[test]
    fn held_out_feedback_cut_selects_the_sensory_mechanism_when_it_generated_data() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let observed = simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        let decision = select_mechanism(
            &observed.samples,
            &CANDIDATES,
            intervention,
            config(),
            1.0e-6,
        )
        .unwrap();
        assert!(matches!(
            decision,
            SelectionDecision::Selected {
                mechanism: Mechanism::SensoryFeedbackDriven,
                ..
            }
        ));
    }

    #[test]
    fn phase_pulse_persists_in_intrinsic_oscillator_but_is_corrected_by_feedback() {
        let pulse = Intervention::PhasePulseAt {
            sample_index: 20,
            delta_phase_radians: 0.6,
        };
        let baseline_intrinsic =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let pulse_intrinsic = simulate(Mechanism::IntrinsicOscillator, pulse, config()).unwrap();
        let baseline_feedback = simulate(
            Mechanism::SensoryFeedbackDriven,
            Intervention::None,
            config(),
        )
        .unwrap();
        let pulse_feedback = simulate(Mechanism::SensoryFeedbackDriven, pulse, config()).unwrap();

        let intrinsic_offset = phase_offset(
            &pulse_intrinsic.samples,
            &baseline_intrinsic.samples,
            config().sample_count - 1,
        )
        .unwrap()
        .unwrap();
        let feedback_offset = phase_offset(
            &pulse_feedback.samples,
            &baseline_feedback.samples,
            config().sample_count - 1,
        )
        .unwrap()
        .unwrap();
        assert!((intrinsic_offset - 0.6).abs() < 1.0e-10);
        assert!(feedback_offset.abs() < 1.0e-10);
    }

    #[test]
    fn damped_focus_can_look_rhythmic_early_but_loses_amplitude() {
        let trace = simulate(Mechanism::DampedFocus, Intervention::None, config()).unwrap();
        assert!(trace.latent_truth[1].amplitude < 1.0);
        assert!(trace.latent_truth.last().unwrap().amplitude < 0.01);
        assert!(trace.samples[5].x.is_finite());
        assert!(trace.samples[5].y.is_finite());
    }

    #[test]
    fn hybrid_feedback_coupling_corrects_a_phase_pulse() {
        let pulse = Intervention::PhasePulseAt {
            sample_index: 20,
            delta_phase_radians: 0.6,
        };
        let baseline = simulate(
            Mechanism::HybridFeedbackCoupled,
            Intervention::None,
            config(),
        )
        .unwrap();
        let perturbed = simulate(Mechanism::HybridFeedbackCoupled, pulse, config()).unwrap();
        let offset = phase_offset(
            &perturbed.samples,
            &baseline.samples,
            config().sample_count - 1,
        )
        .unwrap()
        .unwrap();
        assert!(offset.abs() < 0.02);
    }

    #[test]
    fn phase_offset_abstains_when_feedback_driven_output_has_decayed_to_zero() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 2 };
        let mut low_amplitude_config = config();
        low_amplitude_config.feedback_decay_rate = 20.0;
        let baseline = simulate(
            Mechanism::SensoryFeedbackDriven,
            Intervention::None,
            low_amplitude_config,
        )
        .unwrap();
        let cut = simulate(
            Mechanism::SensoryFeedbackDriven,
            intervention,
            low_amplitude_config,
        )
        .unwrap();
        let offset = phase_offset(
            &cut.samples,
            &baseline.samples,
            low_amplitude_config.sample_count - 1,
        )
        .unwrap();
        assert_eq!(offset, None);
    }

    #[test]
    fn seeded_observation_noise_is_reproducible_and_preserves_timestamps() {
        let trace = simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let a = add_gaussian_observation_noise(&trace.samples, 0.01, 42).unwrap();
        let b = add_gaussian_observation_noise(&trace.samples, 0.01, 42).unwrap();
        assert_eq!(a, b);
        assert_eq!(
            a.iter().map(|sample| sample.timestamp).collect::<Vec<_>>(),
            trace
                .samples
                .iter()
                .map(|sample| sample.timestamp)
                .collect::<Vec<_>>()
        );
        assert_ne!(a, trace.samples);
    }

    #[test]
    fn seeded_gaussian_noise_recovers_declared_variance_on_large_fixture() {
        let mut large_config = config();
        large_config.sample_count = 10_000;
        let trace = simulate(
            Mechanism::IntrinsicOscillator,
            Intervention::None,
            large_config,
        )
        .unwrap();
        let noisy = add_gaussian_observation_noise(&trace.samples, 0.04, 0xC0FFEE).unwrap();

        let mut residuals = Vec::with_capacity(noisy.len() * 2);
        for (actual, expected) in noisy.iter().zip(&trace.samples) {
            residuals.push(actual.x - expected.x);
            residuals.push(actual.y - expected.y);
        }
        let mean = residuals.iter().sum::<f64>() / residuals.len() as f64;
        let variance = residuals
            .iter()
            .map(|residual| (residual - mean).powi(2))
            .sum::<f64>()
            / residuals.len() as f64;
        assert!(mean.abs() < 0.01);
        assert!((variance - 0.04).abs() < 0.002);
    }

    #[test]
    fn noisy_held_out_feedback_cut_still_discriminates_the_generating_mechanism() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let noisy = add_gaussian_observation_noise(&generated.samples, 0.01, 0x5EED).unwrap();
        let decision =
            select_mechanism(&noisy, &CANDIDATES, intervention, config(), 1.0e-6).unwrap();
        assert!(matches!(
            decision,
            SelectionDecision::Selected {
                mechanism: Mechanism::IntrinsicOscillator,
                ..
            }
        ));
    }

    #[test]
    fn zero_variance_noise_is_an_exact_copy_and_invalid_noise_fails_closed() {
        let trace = simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        assert_eq!(
            add_gaussian_observation_noise(&trace.samples, 0.0, 99).unwrap(),
            trace.samples
        );
        assert_eq!(
            add_gaussian_observation_noise(&trace.samples, -0.1, 99),
            Err(BenchmarkError::InvalidObservationVariance)
        );
    }

    #[test]
    fn rejects_invalid_configuration_and_intervention() {
        let mut invalid = config();
        invalid.sample_count = 1;
        assert_eq!(
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, invalid),
            Err(BenchmarkError::InvalidSampleCount)
        );

        assert_eq!(
            simulate(
                Mechanism::IntrinsicOscillator,
                Intervention::FeedbackCutFrom { sample_index: 80 },
                config()
            ),
            Err(BenchmarkError::InvalidInterventionIndex)
        );
        assert_eq!(
            simulate(
                Mechanism::IntrinsicOscillator,
                Intervention::PhasePulseAt {
                    sample_index: 2,
                    delta_phase_radians: f64::NAN,
                },
                config()
            ),
            Err(BenchmarkError::InvalidPhasePulse)
        );
    }

    #[test]
    fn rejects_nonmonotonic_observation_timestamps() {
        let trace = simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let mut invalid = trace.samples.clone();
        invalid[2].timestamp = invalid[1].timestamp;
        assert_eq!(
            trajectory_rmse(&invalid, &trace.samples),
            Err(BenchmarkError::NonMonotonicTraceTimestamp)
        );
        assert_eq!(
            predictive_log_score(&invalid, &trace.samples, config().observation_variance),
            Err(BenchmarkError::NonMonotonicTraceTimestamp)
        );
    }

    #[test]
    fn refuses_to_compare_predictions_on_a_different_time_grid() {
        let trace = simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let mut shifted = trace.samples.clone();
        shifted[10].timestamp += 0.001;
        assert_eq!(
            trajectory_rmse(&trace.samples, &shifted),
            Err(BenchmarkError::TimestampMismatch)
        );
        assert_eq!(
            predictive_log_score(&trace.samples, &shifted, config().observation_variance),
            Err(BenchmarkError::TimestampMismatch)
        );
    }

    #[test]
    fn rejects_invalid_candidate_sets_and_score_margin() {
        let observed =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        assert_eq!(
            select_mechanism(
                &observed.samples,
                &[Mechanism::IntrinsicOscillator],
                Intervention::None,
                config(),
                0.0
            ),
            Err(BenchmarkError::EmptyCandidateSet)
        );
        assert_eq!(
            select_mechanism(
                &observed.samples,
                &[
                    Mechanism::IntrinsicOscillator,
                    Mechanism::IntrinsicOscillator,
                ],
                Intervention::None,
                config(),
                0.0
            ),
            Err(BenchmarkError::DuplicateCandidate)
        );
        assert_eq!(
            select_mechanism(
                &observed.samples,
                &CANDIDATES,
                Intervention::None,
                config(),
                -1.0
            ),
            Err(BenchmarkError::InvalidScoreMargin)
        );
    }

    fn fit_grid() -> FrequencyPhaseGrid {
        FrequencyPhaseGrid {
            minimum_angular_frequency: 3.5,
            maximum_angular_frequency: 4.5,
            frequency_steps: 101,
            phase_steps: 360,
        }
    }

    #[test]
    fn equal_budget_prefix_fit_freezes_only_the_post_action_suffix() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let forecasts = predict_intervention_candidates(
            &generated.samples[..30],
            &CANDIDATES,
            intervention,
            config(),
            fit_grid(),
        )
        .unwrap();

        assert_eq!(forecasts.len(), CANDIDATES.len());
        for forecast in &forecasts {
            assert_eq!(forecast.predicted_suffix.len(), config().sample_count - 30);
            assert_eq!(
                forecast.predicted_suffix[0].timestamp,
                30.0 * config().sample_interval
            );
            assert!((forecast.fitted_angular_frequency - 4.0).abs() <= 0.011);
            assert!(forecast.baseline_log_score.is_finite());
            assert!(forecast.baseline_parameter_score_margin.is_finite());
        }
    }

    #[test]
    fn observationally_equivalent_candidates_fit_equally_but_forecast_differently() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let forecasts = predict_intervention_candidates(
            &generated.samples[..30],
            &CANDIDATES,
            intervention,
            config(),
            fit_grid(),
        )
        .unwrap();

        assert!((forecasts[0].baseline_log_score - forecasts[1].baseline_log_score).abs() < 1.0e-9);
        assert!(
            trajectory_rmse(
                &forecasts[0].predicted_suffix,
                &forecasts[1].predicted_suffix
            )
            .unwrap()
                > 0.1
        );
    }

    #[test]
    fn held_out_suffix_selects_generator_after_equal_budget_baseline_fit() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let noisy = add_gaussian_observation_noise(&generated.samples, 0.01, 0xA11CE).unwrap();
        let forecasts = predict_intervention_candidates(
            &noisy[..30],
            &CANDIDATES,
            intervention,
            config(),
            fit_grid(),
        )
        .unwrap();
        let decision = select_frozen_forecast(
            &noisy[30..],
            &forecasts,
            config().observation_variance,
            1.0e-6,
        )
        .unwrap();

        assert!(matches!(
            decision,
            SelectionDecision::Selected {
                mechanism: Mechanism::IntrinsicOscillator,
                ..
            }
        ));
    }

    #[test]
    fn phase_pulse_predictions_are_frozen_before_the_suffix_is_scored() {
        let intervention = Intervention::PhasePulseAt {
            sample_index: 30,
            delta_phase_radians: 0.5,
        };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let forecasts = predict_intervention_candidates(
            &generated.samples[..30],
            &CANDIDATES,
            intervention,
            config(),
            fit_grid(),
        )
        .unwrap();
        let decision = select_frozen_forecast(
            &generated.samples[30..],
            &forecasts,
            config().observation_variance,
            1.0e-6,
        )
        .unwrap();

        assert!(matches!(
            decision,
            SelectionDecision::Selected {
                mechanism: Mechanism::IntrinsicOscillator,
                ..
            }
        ));
    }

    #[test]
    fn rejects_an_action_not_aligned_to_first_held_out_sample() {
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        assert_eq!(
            predict_intervention_candidates(
                &generated.samples[..30],
                &CANDIDATES,
                Intervention::FeedbackCutFrom { sample_index: 29 },
                config(),
                fit_grid(),
            ),
            Err(BenchmarkError::InvalidInterventionAlignment)
        );
        assert_eq!(
            predict_intervention_candidates(
                &generated.samples[..30],
                &CANDIDATES,
                Intervention::None,
                config(),
                fit_grid(),
            ),
            Err(BenchmarkError::InvalidInterventionAlignment)
        );
    }

    #[test]
    fn rejects_invalid_fit_grid_and_prefix_time_grid() {
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let mut invalid_grid = fit_grid();
        invalid_grid.frequency_steps = 1;
        assert_eq!(
            predict_intervention_candidates(
                &generated.samples[..30],
                &CANDIDATES,
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
                invalid_grid,
            ),
            Err(BenchmarkError::InvalidFitGrid)
        );

        let mut shifted_prefix = generated.samples[..30].to_vec();
        shifted_prefix[5].timestamp += 0.001;
        assert_eq!(
            predict_intervention_candidates(
                &shifted_prefix,
                &CANDIDATES,
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
                fit_grid(),
            ),
            Err(BenchmarkError::TimestampMismatch)
        );
    }

    #[test]
    fn baseline_fit_rejects_single_or_duplicate_candidate_families() {
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        assert_eq!(
            predict_intervention_candidates(
                &generated.samples[..30],
                &[Mechanism::IntrinsicOscillator],
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
                fit_grid(),
            ),
            Err(BenchmarkError::EmptyCandidateSet)
        );
        assert_eq!(
            predict_intervention_candidates(
                &generated.samples[..30],
                &[
                    Mechanism::IntrinsicOscillator,
                    Mechanism::IntrinsicOscillator,
                ],
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
                fit_grid(),
            ),
            Err(BenchmarkError::DuplicateCandidate)
        );
    }

    #[test]
    fn predictive_score_rewards_the_matching_intervention_prediction() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let observed = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let correct = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let wrong = simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        let correct_score = predictive_log_score(
            &observed.samples,
            &correct.samples,
            config().observation_variance,
        )
        .unwrap();
        let wrong_score = predictive_log_score(
            &observed.samples,
            &wrong.samples,
            config().observation_variance,
        )
        .unwrap();
        assert!(correct_score > wrong_score);
    }

    fn monte_carlo_spec() -> MonteCarloSpec {
        MonteCarloSpec {
            generating_mechanisms: vec![
                Mechanism::IntrinsicOscillator,
                Mechanism::SensoryFeedbackDriven,
            ],
            candidates: vec![
                Mechanism::IntrinsicOscillator,
                Mechanism::SensoryFeedbackDriven,
            ],
            seeds: vec![11, 23, 37, 49, 61, 73],
            intervention: Intervention::FeedbackCutFrom { sample_index: 30 },
            base_config: config(),
            fit_grid: FrequencyPhaseGrid {
                minimum_angular_frequency: 3.5,
                maximum_angular_frequency: 4.5,
                frequency_steps: 21,
                phase_steps: 48,
            },
            observation_noise_variance: 0.01,
            minimum_score_margin: 1.0e-6,
        }
    }

    #[test]
    fn paired_seed_monte_carlo_reports_accuracy_false_confidence_and_abstention() {
        let summaries = run_paired_seed_evaluation(monte_carlo_spec()).unwrap();
        assert_eq!(summaries.len(), 2);

        for summary in &summaries {
            assert_eq!(summary.trials, 6);
            assert_eq!(summary.selected_count, 6);
            assert_eq!(summary.correct_count, 6);
            assert_eq!(summary.incorrect_confident_count, 0);
            assert_eq!(summary.abstention_count, 0);
            let accuracy = summary.accuracy_when_selected_95.unwrap();
            assert_eq!(accuracy.successes, 6);
            assert_eq!(accuracy.trials, 6);
            assert_eq!(accuracy.rate, 1.0);
            assert_eq!(summary.false_confident_rate_95.rate, 0.0);
            assert_eq!(summary.abstention_rate_95.rate, 0.0);
            assert!(summary.mean_score_margin.is_finite());
        }
    }

    #[test]
    fn monte_carlo_preserves_abstention_when_score_margin_is_unresolved() {
        let mut spec = monte_carlo_spec();
        spec.minimum_score_margin = f64::MAX;
        let summaries = run_paired_seed_evaluation(spec).unwrap();

        for summary in &summaries {
            assert_eq!(summary.selected_count, 0);
            assert_eq!(summary.correct_count, 0);
            assert_eq!(summary.incorrect_confident_count, 0);
            assert_eq!(summary.abstention_count, summary.trials);
            assert_eq!(summary.accuracy_when_selected_95, None);
            assert_eq!(summary.abstention_rate_95.rate, 1.0);
        }
    }

    #[test]
    fn monte_carlo_rejects_duplicate_seeds_and_generating_mechanisms() {
        let mut duplicate_seed_spec = monte_carlo_spec();
        duplicate_seed_spec.seeds[1] = duplicate_seed_spec.seeds[0];
        assert_eq!(
            run_paired_seed_evaluation(duplicate_seed_spec),
            Err(BenchmarkError::DuplicateSeed)
        );

        let mut duplicate_generator_spec = monte_carlo_spec();
        duplicate_generator_spec
            .generating_mechanisms
            .push(Mechanism::IntrinsicOscillator);
        assert_eq!(
            run_paired_seed_evaluation(duplicate_generator_spec),
            Err(BenchmarkError::DuplicateGenerator)
        );
    }

    #[test]
    fn wilson_interval_matches_known_small_sample_reference() {
        let interval = wilson_interval(5, 10);
        assert!((interval.rate - 0.5).abs() < 1.0e-12);
        assert!((interval.lower_95 - 0.23659309).abs() < 1.0e-7);
        assert!((interval.upper_95 - 0.76340691).abs() < 1.0e-7);
        assert_eq!(interval.successes, 5);
        assert_eq!(interval.trials, 10);
    }
}
