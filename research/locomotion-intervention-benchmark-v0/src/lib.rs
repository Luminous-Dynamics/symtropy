//! Synthetic known-truth benchmark for intervention-based locomotor
//! mechanism discrimination.
//!
//! These toy mechanisms are falsification controls, not biological models.
//! In particular, matching a short periodic trace is deliberately not treated
//! as evidence that the generating system contains an intrinsic oscillator.

use std::collections::{BTreeMap, BTreeSet};

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
pub enum OpenSetAbstentionReason {
    /// The supplied family is not adequate for the observed held-out suffix.
    PoorAbsoluteFit,
    /// At least two candidates remain predictively indistinguishable.
    InsufficientSeparation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpenSetSelectionDecision {
    Selected {
        mechanism: Mechanism,
        log_score: f64,
        mean_log_density_per_coordinate: f64,
        margin_over_runner_up: f64,
    },
    Abstained {
        best_candidate: Mechanism,
        reason: OpenSetAbstentionReason,
        log_score: f64,
        mean_log_density_per_coordinate: f64,
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
    InvalidInterventionCandidateId,
    DuplicateInterventionCandidate,
    InvalidInterventionCost,
    EmptyInterventionSet,
    NoFeasibleInterventions,
    InvalidDesignThreshold,
    EmptyParameterScenarioSet,
    InvalidParameterScenarioGrid,
    EmptyHypothesisSet,
    DuplicateHypothesis,
    InvalidHypothesisPrior,
    InvalidPosteriorUpdate,
    InvalidMonteCarloDraws,
    InvalidInterventionIndex,
    InvalidPhasePulse,
    InvalidScoreMargin,
    InvalidAbsoluteFitThreshold,
    InvalidFitGrid,
    InvalidProfileLikelihoodCutoff,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrequencyProfileInterval {
    pub lower_angular_frequency: f64,
    pub upper_angular_frequency: f64,
    pub grid_points: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrequencyIdentifiability {
    /// One bounded, relatively narrow profile-likelihood component.
    Resolved,
    /// Two or more separated accepted frequency components remain.
    Multimodal,
    /// The accepted set touches either edge of the searched frequency range.
    BoundaryLimited,
    /// One broad accepted component covers at least half of the search range.
    Broad,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FrequencyProfile {
    pub best_angular_frequency: f64,
    pub best_initial_phase: f64,
    pub maximum_log_score: f64,
    /// User-declared likelihood drop from the maximum defining the accepted
    /// profile set. For a regular one-parameter 95% likelihood-ratio set,
    /// 1.920729 is the usual asymptotic log-likelihood drop (chi-square_1/2).
    pub delta_log_likelihood_cutoff: f64,
    pub accepted_intervals: Vec<FrequencyProfileInterval>,
    pub accepted_grid_points: usize,
    pub total_grid_points: usize,
    pub accepted_fraction: f64,
    pub touches_lower_boundary: bool,
    pub touches_upper_boundary: bool,
    pub identifiability: FrequencyIdentifiability,
}

/// Profile over initial phase at each angular-frequency grid point using only
/// the supplied baseline observations. The returned set includes all grid
/// frequencies whose profiled log score is within the declared cutoff of the
/// global maximum. Disconnected components expose aliases/multiple modes
/// rather than collapsing them to a single point estimate.
///
/// This is a grid approximation to a profile-likelihood confidence set, not a
/// posterior interval. Standard asymptotic likelihood-ratio cutoffs require
/// regularity/identifiability assumptions that may fail on short or aliased
/// records; boundary/multimodal flags are therefore part of the result.
pub fn profile_angular_frequency(
    observed_prefix: &[SyntheticSample],
    mechanism: Mechanism,
    base_config: SimulationConfig,
    grid: FrequencyPhaseGrid,
    delta_log_likelihood_cutoff: f64,
) -> Result<FrequencyProfile, BenchmarkError> {
    base_config.validate()?;
    grid.validate()?;
    validate_trace(observed_prefix)?;
    if observed_prefix.len() < 2 || observed_prefix.len() > base_config.sample_count {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    if !delta_log_likelihood_cutoff.is_finite() || delta_log_likelihood_cutoff < 0.0 {
        return Err(BenchmarkError::InvalidProfileLikelihoodCutoff);
    }
    for (index, sample) in observed_prefix.iter().enumerate() {
        let expected_timestamp = index as f64 * base_config.sample_interval;
        if sample.timestamp != expected_timestamp {
            return Err(BenchmarkError::TimestampMismatch);
        }
    }

    let mut profile = Vec::with_capacity(grid.frequency_steps);
    let mut best_score = f64::NEG_INFINITY;
    let mut best_frequency = grid.minimum_angular_frequency;
    let mut best_phase = -std::f64::consts::PI;

    for frequency_index in 0..grid.frequency_steps {
        let fraction = frequency_index as f64 / (grid.frequency_steps - 1) as f64;
        let frequency = grid.minimum_angular_frequency
            + (grid.maximum_angular_frequency - grid.minimum_angular_frequency) * fraction;
        let mut best_phase_score = f64::NEG_INFINITY;
        let mut best_phase_at_frequency = -std::f64::consts::PI;

        for phase_index in 0..grid.phase_steps {
            let phase = -std::f64::consts::PI
                + std::f64::consts::TAU * phase_index as f64 / grid.phase_steps as f64;
            let fit_config = SimulationConfig {
                angular_frequency: frequency,
                initial_phase: phase,
                ..base_config
            };
            let prediction = simulate(mechanism, Intervention::None, fit_config)?;
            let score = predictive_log_score(
                observed_prefix,
                &prediction.samples[..observed_prefix.len()],
                base_config.observation_variance,
            )?;
            if score > best_phase_score {
                best_phase_score = score;
                best_phase_at_frequency = phase;
            }
        }

        profile.push((frequency, best_phase_at_frequency, best_phase_score));
        if best_phase_score > best_score {
            best_score = best_phase_score;
            best_frequency = frequency;
            best_phase = best_phase_at_frequency;
        }
    }
    if !best_score.is_finite() {
        return Err(BenchmarkError::NumericalFailure);
    }

    let cutoff_score = best_score - delta_log_likelihood_cutoff;
    let accepted = profile
        .iter()
        .map(|(_, _, score)| *score >= cutoff_score)
        .collect::<Vec<_>>();
    let accepted_grid_points = accepted.iter().filter(|is_accepted| **is_accepted).count();
    if accepted_grid_points == 0 {
        return Err(BenchmarkError::NumericalFailure);
    }

    let mut intervals = Vec::new();
    let mut interval_start = None;
    for (index, is_accepted) in accepted.iter().copied().enumerate() {
        match (interval_start, is_accepted) {
            (None, true) => interval_start = Some(index),
            (Some(start), false) => {
                intervals.push(FrequencyProfileInterval {
                    lower_angular_frequency: profile[start].0,
                    upper_angular_frequency: profile[index - 1].0,
                    grid_points: index - start,
                });
                interval_start = None;
            }
            _ => {}
        }
    }
    if let Some(start) = interval_start {
        intervals.push(FrequencyProfileInterval {
            lower_angular_frequency: profile[start].0,
            upper_angular_frequency: profile[profile.len() - 1].0,
            grid_points: profile.len() - start,
        });
    }

    let touches_lower_boundary = accepted.first().copied().unwrap_or(false);
    let touches_upper_boundary = accepted.last().copied().unwrap_or(false);
    let accepted_fraction = accepted_grid_points as f64 / grid.frequency_steps as f64;
    let identifiability = if intervals.len() > 1 {
        FrequencyIdentifiability::Multimodal
    } else if touches_lower_boundary || touches_upper_boundary {
        FrequencyIdentifiability::BoundaryLimited
    } else if accepted_fraction >= 0.5 {
        FrequencyIdentifiability::Broad
    } else {
        FrequencyIdentifiability::Resolved
    };

    Ok(FrequencyProfile {
        best_angular_frequency: best_frequency,
        best_initial_phase: best_phase,
        maximum_log_score: best_score,
        delta_log_likelihood_cutoff,
        accepted_intervals: intervals,
        accepted_grid_points,
        total_grid_points: grid.frequency_steps,
        accepted_fraction,
        touches_lower_boundary,
        touches_upper_boundary,
        identifiability,
    })
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

/// Open-set alternative to the closed-set frozen forecast selector.
///
/// The caller supplies a minimum acceptable average log predictive density
/// per scalar coordinate. This threshold must be calibrated on separate
/// controls with the same observation variance, horizon and preprocessing.
/// A large score margin is insufficient when even the best candidate is a
/// poor fit: the result is then PoorAbsoluteFit, not a closed-set winner.
pub fn select_frozen_forecast_open_set(
    observed_suffix: &[SyntheticSample],
    forecasts: &[FrozenForecast],
    observation_variance: f64,
    minimum_score_margin: f64,
    minimum_mean_log_density_per_coordinate: f64,
) -> Result<OpenSetSelectionDecision, BenchmarkError> {
    if forecasts.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if !minimum_score_margin.is_finite() || minimum_score_margin < 0.0 {
        return Err(BenchmarkError::InvalidScoreMargin);
    }
    if !minimum_mean_log_density_per_coordinate.is_finite() {
        return Err(BenchmarkError::InvalidAbsoluteFitThreshold);
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
        let coordinate_count = (2 * observed_suffix.len()) as f64;
        let mean_log_density_per_coordinate = score / coordinate_count;
        if !mean_log_density_per_coordinate.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        scores.push((forecast.mechanism, score, mean_log_density_per_coordinate));
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

    if best.2 < minimum_mean_log_density_per_coordinate {
        Ok(OpenSetSelectionDecision::Abstained {
            best_candidate: best.0,
            reason: OpenSetAbstentionReason::PoorAbsoluteFit,
            log_score: best.1,
            mean_log_density_per_coordinate: best.2,
            margin_over_runner_up: margin,
        })
    } else if margin <= minimum_score_margin {
        Ok(OpenSetSelectionDecision::Abstained {
            best_candidate: best.0,
            reason: OpenSetAbstentionReason::InsufficientSeparation,
            log_score: best.1,
            mean_log_density_per_coordinate: best.2,
            margin_over_runner_up: margin,
        })
    } else {
        Ok(OpenSetSelectionDecision::Selected {
            mechanism: best.0,
            log_score: best.1,
            mean_log_density_per_coordinate: best.2,
            margin_over_runner_up: margin,
        })
    }
}

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
            cross_covariance[0][0] * inverse[0][0] + cross_covariance[0][1] * inverse[1][0],
            cross_covariance[0][0] * inverse[0][1] + cross_covariance[0][1] * inverse[1][1],
        ],
        [
            cross_covariance[1][0] * inverse[0][0] + cross_covariance[1][1] * inverse[1][0],
            cross_covariance[1][0] * inverse[0][1] + cross_covariance[1][1] * inverse[1][1],
        ],
    ];
    let intercept = [
        next_mean[0] - transition[0][0] * previous_mean[0] - transition[0][1] * previous_mean[1],
        next_mean[1] - transition[1][0] * previous_mean[0] - transition[1][1] * previous_mean[1],
    ];
    if !transition.iter().flatten().all(|value| value.is_finite())
        || !intercept.iter().all(|value| value.is_finite())
    {
        return Err(BenchmarkError::NumericalFailure);
    }

    let mut training_prediction = Vec::with_capacity(observed_prefix.len() - 1);
    for index in 1..observed_prefix.len() {
        let previous = observed_prefix[index - 1];
        let prediction = apply_vector_ar1(transition, intercept, [previous.x, previous.y]);
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

fn apply_vector_ar1(transition: [[f64; 2]; 2], intercept: [f64; 2], state: [f64; 2]) -> [f64; 2] {
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

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionCandidate {
    pub id: String,
    pub intervention: Intervention,
    /// First sample at which the intervention may affect model outputs.
    pub onset_index: usize,
    /// Comparable resource, risk or time cost in caller-defined units.
    pub cost: f64,
    /// Infeasible actions are excluded before scoring.
    pub feasible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairwiseSeparation {
    pub scenario_index: usize,
    pub left: Mechanism,
    pub right: Mechanism,
    pub post_intervention_rmse: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterventionDesign {
    pub candidate_id: String,
    pub intervention: Intervention,
    pub onset_index: usize,
    pub cost: f64,
    pub worst_pairwise_rmse: f64,
    pub mean_pairwise_rmse: f64,
    /// Worst-pair separation divided by action cost.
    pub maximin_utility: f64,
    pub scenario_count: usize,
    pub pairwise: Vec<PairwiseSeparation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InterventionDesignDecision {
    Selected {
        design: InterventionDesign,
    },
    Abstained {
        best_candidate_id: String,
        maximin_utility: f64,
        worst_pairwise_rmse: f64,
    },
}

/// Rank candidate interventions by robust pairwise forecast disagreement.
///
/// Each candidate mechanism receives the same initial condition, time grid,
/// intervention and fixed model parameters. The objective is the minimum
/// post-onset trajectory RMSE over all pairs of candidate mechanisms, divided
/// by the declared action cost. This maximin rule favours interventions that
/// separate the hardest-to-distinguish pair instead of maximizing average
/// disagreement while leaving one pair observationally equivalent.
///
/// This is a deterministic disagreement heuristic, not Bayesian expected
/// information gain: it has no posterior over models, parameters or outcomes.
/// It is useful as a transparent first-stage action selector and an oracle
/// benchmark for later posterior-predictive planners.
pub fn design_discriminating_intervention(
    mechanisms: &[Mechanism],
    candidates: &[InterventionCandidate],
    config: SimulationConfig,
    minimum_maximin_utility: f64,
) -> Result<InterventionDesignDecision, BenchmarkError> {
    design_discriminating_intervention_across_scenarios(
        mechanisms,
        candidates,
        &[config],
        minimum_maximin_utility,
    )
}

/// Robust variant of the intervention selector. Every candidate action is
/// evaluated under all supplied plausible parameter scenarios using the same
/// time grid. The objective takes the minimum separation over both mechanism
/// pairs and parameter scenarios, so a perturbation is not selected merely
/// because it is informative at one favourable nuisance-parameter setting.
///
/// Scenarios may vary frequency, phase, feedback decay and coupling. They must
/// share the same sample count and interval so paired trajectories remain
/// aligned. This is a finite-scenario worst-case approximation, not a
/// posterior predictive distribution or a Bayesian credible set.
pub fn design_discriminating_intervention_across_scenarios(
    mechanisms: &[Mechanism],
    candidates: &[InterventionCandidate],
    parameter_scenarios: &[SimulationConfig],
    minimum_maximin_utility: f64,
) -> Result<InterventionDesignDecision, BenchmarkError> {
    if parameter_scenarios.is_empty() {
        return Err(BenchmarkError::EmptyParameterScenarioSet);
    }
    let primary_config = parameter_scenarios[0];
    primary_config.validate()?;
    for scenario in parameter_scenarios.iter().skip(1) {
        scenario.validate()?;
        if scenario.sample_count != primary_config.sample_count
            || scenario.sample_interval != primary_config.sample_interval
        {
            return Err(BenchmarkError::InvalidParameterScenarioGrid);
        }
    }
    if mechanisms.len() < 2 {
        return Err(BenchmarkError::EmptyCandidateSet);
    }
    if candidates.is_empty() {
        return Err(BenchmarkError::EmptyInterventionSet);
    }
    if !minimum_maximin_utility.is_finite() || minimum_maximin_utility < 0.0 {
        return Err(BenchmarkError::InvalidDesignThreshold);
    }

    let mut ordered_mechanisms = mechanisms.to_vec();
    ordered_mechanisms.sort();
    let unique_mechanisms: BTreeSet<_> = ordered_mechanisms.iter().copied().collect();
    if unique_mechanisms.len() != ordered_mechanisms.len() {
        return Err(BenchmarkError::DuplicateCandidate);
    }

    let mut unique_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            return Err(BenchmarkError::InvalidInterventionCandidateId);
        }
        if !unique_ids.insert(candidate.id.clone()) {
            return Err(BenchmarkError::DuplicateInterventionCandidate);
        }
        if !candidate.cost.is_finite() || candidate.cost <= 0.0 {
            return Err(BenchmarkError::InvalidInterventionCost);
        }
        if candidate.onset_index >= primary_config.sample_count {
            return Err(BenchmarkError::InvalidInterventionIndex);
        }
        let action_index = match candidate.intervention {
            Intervention::FeedbackCutFrom { sample_index }
            | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
            Intervention::None => return Err(BenchmarkError::InvalidInterventionAlignment),
        };
        if action_index != candidate.onset_index {
            return Err(BenchmarkError::InvalidInterventionAlignment);
        }
        for scenario in parameter_scenarios {
            validate_intervention(candidate.intervention, *scenario)?;
        }
    }

    let mut designs = Vec::new();
    for candidate in candidates.iter().filter(|candidate| candidate.feasible) {
        let mut pairwise = Vec::new();
        let mut worst_pairwise_rmse = f64::INFINITY;
        let mut pairwise_sum = 0.0;

        for (scenario_index, scenario) in parameter_scenarios.iter().enumerate() {
            let mut traces = Vec::with_capacity(ordered_mechanisms.len());
            for mechanism in &ordered_mechanisms {
                traces.push((
                    *mechanism,
                    simulate(*mechanism, candidate.intervention, *scenario)?,
                ));
            }

            for left_index in 0..traces.len() {
                for right_index in (left_index + 1)..traces.len() {
                    let divergence = post_intervention_rmse(
                        &traces[left_index].1.samples,
                        &traces[right_index].1.samples,
                        candidate.onset_index,
                    )?;
                    if !divergence.is_finite() {
                        return Err(BenchmarkError::NumericalFailure);
                    }
                    worst_pairwise_rmse = worst_pairwise_rmse.min(divergence);
                    pairwise_sum += divergence;
                    pairwise.push(PairwiseSeparation {
                        scenario_index,
                        left: traces[left_index].0,
                        right: traces[right_index].0,
                        post_intervention_rmse: divergence,
                    });
                }
            }
        }

        let pair_count = pairwise.len();
        if pair_count == 0 {
            return Err(BenchmarkError::EmptyCandidateSet);
        }
        let mean_pairwise_rmse = pairwise_sum / pair_count as f64;
        let maximin_utility = worst_pairwise_rmse / candidate.cost;
        if !mean_pairwise_rmse.is_finite() || !maximin_utility.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }

        designs.push(InterventionDesign {
            candidate_id: candidate.id.clone(),
            intervention: candidate.intervention,
            onset_index: candidate.onset_index,
            cost: candidate.cost,
            worst_pairwise_rmse,
            mean_pairwise_rmse,
            maximin_utility,
            scenario_count: parameter_scenarios.len(),
            pairwise,
        });
    }

    if designs.is_empty() {
        return Err(BenchmarkError::NoFeasibleInterventions);
    }
    designs.sort_by(|left, right| {
        right
            .maximin_utility
            .total_cmp(&left.maximin_utility)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });

    let best = designs.remove(0);
    if best.maximin_utility <= minimum_maximin_utility {
        Ok(InterventionDesignDecision::Abstained {
            best_candidate_id: best.candidate_id,
            maximin_utility: best.maximin_utility,
            worst_pairwise_rmse: best.worst_pairwise_rmse,
        })
    } else {
        Ok(InterventionDesignDecision::Selected { design: best })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MechanismHypothesis {
    pub id: String,
    pub mechanism: Mechanism,
    pub config: SimulationConfig,
    /// Prior probability; weights are normalized by the design routine.
    pub prior_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HypothesisPosteriorWeight {
    pub hypothesis_id: String,
    pub probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HypothesisPosteriorUpdate {
    /// Sorted by hypothesis ID, independent of caller order.
    pub posterior: Vec<HypothesisPosteriorWeight>,
    /// Log marginal predictive density of this intervention outcome.
    pub log_marginal_evidence: f64,
    pub prior_entropy_nats: f64,
    pub posterior_entropy_nats: f64,
    pub information_gain_nats: f64,
    /// True when an extremely small normalized probability underflowed to
    /// zero and was floored at f64::MIN_POSITIVE so the posterior remains a
    /// valid reusable prior for a subsequent design step.
    pub probability_floor_applied: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExpectedInformationDesign {
    pub candidate_id: String,
    pub intervention: Intervention,
    pub onset_index: usize,
    pub cost: f64,
    pub draws: usize,
    pub expected_information_nats: f64,
    pub standard_error_nats: f64,
    pub information_per_cost: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExpectedInformationDecision {
    Selected {
        design: ExpectedInformationDesign,
    },
    Abstained {
        best_candidate_id: String,
        information_per_cost: f64,
        standard_error_nats: f64,
    },
}

/// Update a finite mechanism/parameter posterior after observing one
/// intervention outcome on a new, reset session.
///
/// Hypotheses are fixed mechanism/parameter scenarios with an explicit prior.
/// The intervention assignment and delivery must have been independently
/// verified by the caller. This function scores only the observed suffix,
/// requires its timestamps to match the declared intervention grid, and
/// uses the same independent Gaussian observation variance for both
/// coordinates. The returned posterior can be passed to
/// apply_hypothesis_posterior and then back into the EIG action selector.
///
/// This supports sequential experiment selection across independent
/// sessions. It does not propagate latent state across interventions within
/// one uninterrupted locomotor episode.
pub fn update_hypothesis_posterior_after_intervention(
    hypotheses: &[MechanismHypothesis],
    intervention: Intervention,
    observed_suffix: &[SyntheticSample],
    observation_variance: f64,
) -> Result<HypothesisPosteriorUpdate, BenchmarkError> {
    if hypotheses.len() < 2 {
        return Err(BenchmarkError::EmptyHypothesisSet);
    }
    if !observation_variance.is_finite() || observation_variance <= 0.0 {
        return Err(BenchmarkError::InvalidObservationVariance);
    }

    let mut ids = BTreeSet::new();
    let first_config = hypotheses[0].config;
    first_config.validate()?;
    let onset_index = match intervention {
        Intervention::FeedbackCutFrom { sample_index }
        | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
        Intervention::None => return Err(BenchmarkError::InvalidInterventionAlignment),
    };
    if onset_index >= first_config.sample_count {
        return Err(BenchmarkError::InvalidInterventionIndex);
    }

    let expected_len = first_config.sample_count - onset_index;
    if observed_suffix.len() != expected_len {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    validate_trace(observed_suffix)?;
    for (offset, sample) in observed_suffix.iter().enumerate() {
        let expected_timestamp = (onset_index + offset) as f64 * first_config.sample_interval;
        if sample.timestamp != expected_timestamp {
            return Err(BenchmarkError::TimestampMismatch);
        }
    }

    let mut prior_total = 0.0;
    for hypothesis in hypotheses {
        if hypothesis.id.trim().is_empty() {
            return Err(BenchmarkError::InvalidInterventionCandidateId);
        }
        if !ids.insert(hypothesis.id.clone()) {
            return Err(BenchmarkError::DuplicateHypothesis);
        }
        hypothesis.config.validate()?;
        if hypothesis.config.sample_count != first_config.sample_count
            || hypothesis.config.sample_interval != first_config.sample_interval
        {
            return Err(BenchmarkError::InvalidParameterScenarioGrid);
        }
        validate_intervention(intervention, hypothesis.config)?;
        if !hypothesis.prior_weight.is_finite() || hypothesis.prior_weight <= 0.0 {
            return Err(BenchmarkError::InvalidHypothesisPrior);
        }
        prior_total += hypothesis.prior_weight;
        if !prior_total.is_finite() {
            return Err(BenchmarkError::InvalidHypothesisPrior);
        }
    }
    if prior_total <= 0.0 {
        return Err(BenchmarkError::InvalidHypothesisPrior);
    }

    let mut prior_entropy_nats = 0.0;
    let mut log_joint = Vec::with_capacity(hypotheses.len());
    for hypothesis in hypotheses {
        let prior = hypothesis.prior_weight / prior_total;
        prior_entropy_nats -= prior * prior.ln();

        // Predictions are regenerated from the hypothesis and the verified
        // action. No observed sample is used to tune this prediction.
        let predicted = simulate(hypothesis.mechanism, intervention, hypothesis.config)?;
        let predicted_suffix = &predicted.samples[onset_index..];
        let log_likelihood =
            predictive_log_score(observed_suffix, predicted_suffix, observation_variance)?;
        log_joint.push((hypothesis.id.clone(), prior.ln() + log_likelihood));
    }

    let log_evidence = log_sum_exp(
        &log_joint
            .iter()
            .map(|(_, value)| *value)
            .collect::<Vec<_>>(),
    )?;
    let mut probability_floor_applied = false;
    let mut posterior = log_joint
        .into_iter()
        .map(|(hypothesis_id, log_probability)| {
            let raw_probability = (log_probability - log_evidence).exp();
            if raw_probability == 0.0 {
                probability_floor_applied = true;
            }
            HypothesisPosteriorWeight {
                hypothesis_id,
                probability: raw_probability.max(f64::MIN_POSITIVE),
            }
        })
        .collect::<Vec<_>>();
    posterior.sort_by(|left, right| left.hypothesis_id.cmp(&right.hypothesis_id));

    let posterior_total = posterior.iter().map(|item| item.probability).sum::<f64>();
    if !posterior_total.is_finite() || posterior_total <= 0.0 {
        return Err(BenchmarkError::NumericalFailure);
    }
    for item in &mut posterior {
        item.probability /= posterior_total;
    }
    let posterior_entropy_nats = posterior
        .iter()
        .filter(|item| item.probability > 0.0)
        .map(|item| -item.probability * item.probability.ln())
        .sum::<f64>();
    let information_gain_nats = prior_entropy_nats - posterior_entropy_nats;
    if !prior_entropy_nats.is_finite()
        || !posterior_entropy_nats.is_finite()
        || !information_gain_nats.is_finite()
        || !log_evidence.is_finite()
    {
        return Err(BenchmarkError::NumericalFailure);
    }

    Ok(HypothesisPosteriorUpdate {
        posterior,
        log_marginal_evidence: log_evidence,
        prior_entropy_nats,
        posterior_entropy_nats,
        information_gain_nats,
        probability_floor_applied,
    })
}

/// Replace hypothesis prior weights with a posterior update. Every ID must
/// match exactly once; omissions, extras and duplicate IDs fail closed.
pub fn apply_hypothesis_posterior(
    hypotheses: &[MechanismHypothesis],
    update: &HypothesisPosteriorUpdate,
) -> Result<Vec<MechanismHypothesis>, BenchmarkError> {
    if hypotheses.len() < 2 || hypotheses.len() != update.posterior.len() {
        return Err(BenchmarkError::InvalidPosteriorUpdate);
    }
    let mut posterior_by_id = BTreeMap::new();
    let mut total_probability = 0.0;
    for item in &update.posterior {
        if item.hypothesis_id.trim().is_empty()
            || !item.probability.is_finite()
            || item.probability <= 0.0
            || posterior_by_id
                .insert(item.hypothesis_id.as_str(), item.probability)
                .is_some()
        {
            return Err(BenchmarkError::InvalidPosteriorUpdate);
        }
        total_probability += item.probability;
    }
    if !total_probability.is_finite() || (total_probability - 1.0).abs() > 1.0e-9 {
        return Err(BenchmarkError::InvalidPosteriorUpdate);
    }

    let mut seen = BTreeSet::new();
    let mut updated = Vec::with_capacity(hypotheses.len());
    for hypothesis in hypotheses {
        if !seen.insert(hypothesis.id.as_str()) {
            return Err(BenchmarkError::DuplicateHypothesis);
        }
        let probability = posterior_by_id
            .get(hypothesis.id.as_str())
            .copied()
            .ok_or(BenchmarkError::InvalidPosteriorUpdate)?;
        updated.push(MechanismHypothesis {
            id: hypothesis.id.clone(),
            mechanism: hypothesis.mechanism,
            config: hypothesis.config,
            prior_weight: probability,
        });
    }
    if seen.len() != posterior_by_id.len() {
        return Err(BenchmarkError::InvalidPosteriorUpdate);
    }
    Ok(updated)
}

/// Monte Carlo estimate of discrete-hypothesis Bayesian expected information
/// gain for each candidate intervention.
///
/// A hypothesis is a declared mechanism plus one fixed parameter scenario and
/// a prior weight. The observation model is independent Gaussian noise with
/// the same declared variance for both observed coordinates at every sample.
/// The likelihood is the common predictive Gaussian likelihood used elsewhere
/// in this crate. The expected information gain is:
///
/// ```text
/// E_{h ~ prior, y ~ p(y|h,a)} [log p(y|h,a) - log sum_j p(h_j)p(y|h_j,a)]
/// ```
///
/// This is a genuine finite-hypothesis expected information gain calculation
/// under the supplied discrete prior and Gaussian observation model. It is
/// not continuous parameter inference, posterior calibration, or proof that
/// the hypothesis set contains the true biological mechanism.
pub fn design_intervention_by_expected_information_gain(
    hypotheses: &[MechanismHypothesis],
    candidates: &[InterventionCandidate],
    observation_variance: f64,
    draws: usize,
    seed: u64,
    minimum_information_per_cost: f64,
) -> Result<ExpectedInformationDecision, BenchmarkError> {
    if hypotheses.len() < 2 {
        return Err(BenchmarkError::EmptyHypothesisSet);
    }
    if candidates.is_empty() {
        return Err(BenchmarkError::EmptyInterventionSet);
    }
    if !observation_variance.is_finite() || observation_variance <= 0.0 {
        return Err(BenchmarkError::InvalidObservationVariance);
    }
    if draws < 2 {
        return Err(BenchmarkError::InvalidMonteCarloDraws);
    }
    if !minimum_information_per_cost.is_finite() || minimum_information_per_cost < 0.0 {
        return Err(BenchmarkError::InvalidDesignThreshold);
    }

    let mut hypothesis_ids = BTreeSet::new();
    let mut total_prior_weight = 0.0;
    let first_config = hypotheses[0].config;
    first_config.validate()?;
    for hypothesis in hypotheses {
        if hypothesis.id.trim().is_empty() {
            return Err(BenchmarkError::InvalidInterventionCandidateId);
        }
        if !hypothesis_ids.insert(hypothesis.id.clone()) {
            return Err(BenchmarkError::DuplicateHypothesis);
        }
        hypothesis.config.validate()?;
        if hypothesis.config.sample_count != first_config.sample_count
            || hypothesis.config.sample_interval != first_config.sample_interval
        {
            return Err(BenchmarkError::InvalidParameterScenarioGrid);
        }
        if !hypothesis.prior_weight.is_finite() || hypothesis.prior_weight <= 0.0 {
            return Err(BenchmarkError::InvalidHypothesisPrior);
        }
        total_prior_weight += hypothesis.prior_weight;
        if !total_prior_weight.is_finite() {
            return Err(BenchmarkError::InvalidHypothesisPrior);
        }
    }
    if total_prior_weight <= 0.0 {
        return Err(BenchmarkError::InvalidHypothesisPrior);
    }

    let mut prior = hypotheses
        .iter()
        .map(|hypothesis| hypothesis.prior_weight / total_prior_weight)
        .collect::<Vec<_>>();
    // Normalize once more to avoid accumulated floating point drift.
    let normalized_total = prior.iter().sum::<f64>();
    for probability in &mut prior {
        *probability /= normalized_total;
    }

    let mut candidate_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            return Err(BenchmarkError::InvalidInterventionCandidateId);
        }
        if !candidate_ids.insert(candidate.id.clone()) {
            return Err(BenchmarkError::DuplicateInterventionCandidate);
        }
        if !candidate.cost.is_finite() || candidate.cost <= 0.0 {
            return Err(BenchmarkError::InvalidInterventionCost);
        }
        if candidate.onset_index >= first_config.sample_count {
            return Err(BenchmarkError::InvalidInterventionIndex);
        }
        let action_index = match candidate.intervention {
            Intervention::FeedbackCutFrom { sample_index }
            | Intervention::PhasePulseAt { sample_index, .. } => sample_index,
            Intervention::None => return Err(BenchmarkError::InvalidInterventionAlignment),
        };
        if action_index != candidate.onset_index {
            return Err(BenchmarkError::InvalidInterventionAlignment);
        }
        for hypothesis in hypotheses {
            validate_intervention(candidate.intervention, hypothesis.config)?;
        }
    }

    let mut ordered_candidates = candidates
        .iter()
        .filter(|candidate| candidate.feasible)
        .collect::<Vec<_>>();
    ordered_candidates.sort_by(|left, right| left.id.cmp(&right.id));
    if ordered_candidates.is_empty() {
        return Err(BenchmarkError::NoFeasibleInterventions);
    }

    let mut designs = Vec::with_capacity(ordered_candidates.len());
    for candidate in ordered_candidates {
        // Freeze every hypothesis forecast before generating or scoring any
        // candidate outcome. No held-out outcome can alter a prediction.
        let mut forecasts = Vec::with_capacity(hypotheses.len());
        for hypothesis in hypotheses {
            let trace = simulate(
                hypothesis.mechanism,
                candidate.intervention,
                hypothesis.config,
            )?;
            forecasts.push(trace.samples[candidate.onset_index..].to_vec());
        }

        // Derive a stable candidate-local seed, so reordering candidates does
        // not change the Monte Carlo outcomes attached to a candidate ID.
        let candidate_seed = stable_seed(seed, &candidate.id);
        let mut random = SeededNormal::new(candidate_seed);
        let log_prior = prior.iter().map(|value| value.ln()).collect::<Vec<_>>();
        let mut information_sum = 0.0;
        let mut squared_information_sum = 0.0;

        for _ in 0..draws {
            let selection = random.uniform_open();
            let mut cumulative = 0.0;
            let mut true_index = prior.len() - 1;
            for (index, probability) in prior.iter().enumerate() {
                cumulative += probability;
                if selection < cumulative {
                    true_index = index;
                    break;
                }
            }

            let noise_seed = random.next_u64();
            let observation = add_gaussian_observation_noise(
                &forecasts[true_index],
                observation_variance,
                noise_seed,
            )?;

            let mut log_likelihoods = Vec::with_capacity(hypotheses.len());
            for forecast in &forecasts {
                log_likelihoods.push(predictive_log_score(
                    &observation,
                    forecast,
                    observation_variance,
                )?);
            }

            let log_joint = log_prior
                .iter()
                .zip(&log_likelihoods)
                .map(|(log_probability, log_likelihood)| log_probability + log_likelihood)
                .collect::<Vec<_>>();
            let log_marginal = log_sum_exp(&log_joint)?;
            let information_sample = log_likelihoods[true_index] - log_marginal;
            if !information_sample.is_finite() {
                return Err(BenchmarkError::NumericalFailure);
            }
            information_sum += information_sample;
            squared_information_sum += information_sample * information_sample;
        }

        let expected_information_nats = information_sum / draws as f64;
        let sample_variance = ((squared_information_sum
            - draws as f64 * expected_information_nats.powi(2))
            / (draws - 1) as f64)
            .max(0.0);
        let standard_error_nats = (sample_variance / draws as f64).sqrt();
        let information_per_cost = expected_information_nats / candidate.cost;
        if !expected_information_nats.is_finite()
            || !standard_error_nats.is_finite()
            || !information_per_cost.is_finite()
        {
            return Err(BenchmarkError::NumericalFailure);
        }

        designs.push(ExpectedInformationDesign {
            candidate_id: candidate.id.clone(),
            intervention: candidate.intervention,
            onset_index: candidate.onset_index,
            cost: candidate.cost,
            draws,
            expected_information_nats,
            standard_error_nats,
            information_per_cost,
        });
    }

    designs.sort_by(|left, right| {
        right
            .information_per_cost
            .total_cmp(&left.information_per_cost)
            .then_with(|| left.candidate_id.cmp(&right.candidate_id))
    });
    let best = designs.remove(0);
    if best.information_per_cost <= minimum_information_per_cost {
        Ok(ExpectedInformationDecision::Abstained {
            best_candidate_id: best.candidate_id,
            information_per_cost: best.information_per_cost,
            standard_error_nats: best.standard_error_nats,
        })
    } else {
        Ok(ExpectedInformationDecision::Selected { design: best })
    }
}

fn stable_seed(seed: u64, identifier: &str) -> u64 {
    let mut value = seed ^ 0xCBF2_9CE4_8422_2325;
    for byte in identifier.bytes() {
        value ^= u64::from(byte);
        value = value.wrapping_mul(0x0000_0100_0000_01B3);
    }
    value
}

fn log_sum_exp(values: &[f64]) -> Result<f64, BenchmarkError> {
    if values.is_empty() || values.iter().any(|value| !value.is_finite()) {
        return Err(BenchmarkError::NumericalFailure);
    }
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum = values
        .iter()
        .map(|value| (*value - maximum).exp())
        .sum::<f64>();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(BenchmarkError::NumericalFailure);
    }
    let result = maximum + sum.ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(BenchmarkError::NumericalFailure)
    }
}

fn post_intervention_rmse(
    left: &[SyntheticSample],
    right: &[SyntheticSample],
    onset_index: usize,
) -> Result<f64, BenchmarkError> {
    if onset_index >= left.len() || left.len() != right.len() {
        return Err(BenchmarkError::InvalidTraceLength);
    }
    validate_trace(left)?;
    validate_trace(right)?;
    validate_aligned_timestamps(left, right)?;
    trajectory_rmse(&left[onset_index..], &right[onset_index..])
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
    fn open_set_gate_abstains_when_every_candidate_is_a_poor_fit_even_with_a_large_margin() {
        let observed = (0..8)
            .map(|index| SyntheticSample {
                timestamp: index as f64 * 0.05,
                x: 0.0,
                y: 0.0,
            })
            .collect::<Vec<_>>();
        let forecasts = vec![
            FrozenForecast {
                mechanism: Mechanism::IntrinsicOscillator,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed
                    .iter()
                    .map(|sample| SyntheticSample { x: 1.0, ..*sample })
                    .collect(),
            },
            FrozenForecast {
                mechanism: Mechanism::SensoryFeedbackDriven,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed
                    .iter()
                    .map(|sample| SyntheticSample { x: 2.0, ..*sample })
                    .collect(),
            },
        ];

        let decision =
            select_frozen_forecast_open_set(&observed, &forecasts, 0.01, 0.0, -10.0).unwrap();
        assert!(matches!(
            decision,
            OpenSetSelectionDecision::Abstained {
                reason: OpenSetAbstentionReason::PoorAbsoluteFit,
                margin_over_runner_up,
                ..
            } if margin_over_runner_up > 100.0
        ));
    }

    #[test]
    fn open_set_gate_selects_when_best_fit_is_adequate_and_separated() {
        let observed = (0..8)
            .map(|index| SyntheticSample {
                timestamp: index as f64 * 0.05,
                x: 0.0,
                y: 0.0,
            })
            .collect::<Vec<_>>();
        let forecasts = vec![
            FrozenForecast {
                mechanism: Mechanism::IntrinsicOscillator,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed.clone(),
            },
            FrozenForecast {
                mechanism: Mechanism::SensoryFeedbackDriven,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed
                    .iter()
                    .map(|sample| SyntheticSample { x: 1.0, ..*sample })
                    .collect(),
            },
        ];

        let decision =
            select_frozen_forecast_open_set(&observed, &forecasts, 0.01, 1.0, -10.0).unwrap();
        assert!(matches!(
            decision,
            OpenSetSelectionDecision::Selected {
                mechanism: Mechanism::IntrinsicOscillator,
                ..
            }
        ));
    }

    #[test]
    fn open_set_gate_distinguishes_adequate_but_ambiguous_from_poor_fit() {
        let observed = (0..8)
            .map(|index| SyntheticSample {
                timestamp: index as f64 * 0.05,
                x: 0.0,
                y: 0.0,
            })
            .collect::<Vec<_>>();
        let forecasts = vec![
            FrozenForecast {
                mechanism: Mechanism::IntrinsicOscillator,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed.clone(),
            },
            FrozenForecast {
                mechanism: Mechanism::SensoryFeedbackDriven,
                fitted_angular_frequency: 4.0,
                fitted_initial_phase: 0.0,
                baseline_log_score: 0.0,
                baseline_parameter_score_margin: 0.0,
                predicted_suffix: observed.clone(),
            },
        ];

        let decision =
            select_frozen_forecast_open_set(&observed, &forecasts, 0.01, 1.0e-6, -10.0).unwrap();
        assert!(matches!(
            decision,
            OpenSetSelectionDecision::Abstained {
                reason: OpenSetAbstentionReason::InsufficientSeparation,
                ..
            }
        ));
    }

    #[test]
    fn open_set_gate_rejects_nonfinite_absolute_fit_threshold() {
        let observed = (0..3)
            .map(|index| SyntheticSample {
                timestamp: index as f64 * 0.05,
                x: 0.0,
                y: 0.0,
            })
            .collect::<Vec<_>>();
        let forecasts = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ]
        .iter()
        .map(|mechanism| FrozenForecast {
            mechanism: *mechanism,
            fitted_angular_frequency: 4.0,
            fitted_initial_phase: 0.0,
            baseline_log_score: 0.0,
            baseline_parameter_score_margin: 0.0,
            predicted_suffix: observed.clone(),
        })
        .collect::<Vec<_>>();

        assert_eq!(
            select_frozen_forecast_open_set(&observed, &forecasts, 0.01, 1.0e-6, f64::NAN,),
            Err(BenchmarkError::InvalidAbsoluteFitThreshold)
        );
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

    fn quick_fit_grid() -> FrequencyPhaseGrid {
        FrequencyPhaseGrid {
            minimum_angular_frequency: 3.5,
            maximum_angular_frequency: 4.5,
            frequency_steps: 21,
            phase_steps: 72,
        }
    }

    #[test]
    fn vector_ar1_recovers_the_rotational_baseline_and_freezes_its_suffix() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let forecast =
            fit_vector_ar1_forecast(&generated.samples[..30], intervention, config()).unwrap();
        let angle = config().angular_frequency * config().sample_interval;
        let (sin_angle, cos_angle) = angle.sin_cos();

        assert!((forecast.transition[0][0] - cos_angle).abs() < 1.0e-10);
        assert!((forecast.transition[0][1] + sin_angle).abs() < 1.0e-10);
        assert!((forecast.transition[1][0] - sin_angle).abs() < 1.0e-10);
        assert!((forecast.transition[1][1] - cos_angle).abs() < 1.0e-10);
        assert!(forecast.intercept.iter().all(|value| value.abs() < 1.0e-10));
        assert_eq!(forecast.predicted_suffix.len(), config().sample_count - 30);
        assert_eq!(
            forecast.predicted_suffix[0].timestamp,
            30.0 * config().sample_interval
        );
    }

    #[test]
    fn vector_ar1_is_a_predictive_equivalent_not_evidence_for_an_oscillator() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let mechanistic = predict_intervention_candidates(
            &generated.samples[..30],
            &CANDIDATES,
            intervention,
            config(),
            quick_fit_grid(),
        )
        .unwrap();
        let ar1 =
            fit_vector_ar1_forecast(&generated.samples[..30], intervention, config()).unwrap();

        let oscillator = mechanistic
            .iter()
            .find(|forecast| forecast.mechanism == Mechanism::IntrinsicOscillator)
            .unwrap();
        assert!(
            trajectory_rmse(&oscillator.predicted_suffix, &ar1.predicted_suffix).unwrap() < 0.03
        );
        let decision = select_competing_models(
            &generated.samples[30..],
            &mechanistic,
            &ar1,
            config().observation_variance,
            2.0,
        )
        .unwrap();
        assert!(matches!(decision, CompetingModelDecision::Abstained { .. }));
    }

    #[test]
    fn fitted_sensory_model_beats_action_blind_var1_after_feedback_cut() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 30 };
        let generated = simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        let candidates = [
            Mechanism::SensoryFeedbackDriven,
            Mechanism::IntrinsicOscillator,
        ];
        let mechanistic = predict_intervention_candidates(
            &generated.samples[..30],
            &candidates,
            intervention,
            config(),
            quick_fit_grid(),
        )
        .unwrap();
        let ar1 =
            fit_vector_ar1_forecast(&generated.samples[..30], intervention, config()).unwrap();
        let decision = select_competing_models(
            &generated.samples[30..],
            &mechanistic,
            &ar1,
            config().observation_variance,
            1.0e-6,
        )
        .unwrap();

        assert!(matches!(
            decision,
            CompetingModelDecision::Selected {
                family: CandidateFamily::Mechanistic(Mechanism::SensoryFeedbackDriven),
                ..
            }
        ));
    }

    #[test]
    fn vector_ar1_rejects_singular_and_misaligned_baseline_records() {
        let constant = (0..30)
            .map(|index| SyntheticSample {
                timestamp: index as f64 * config().sample_interval,
                x: 1.0,
                y: 0.0,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            fit_vector_ar1_forecast(
                &constant,
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
            ),
            Err(BenchmarkError::SingularVectorAr1Fit)
        );

        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let mut shifted = generated.samples[..30].to_vec();
        shifted[4].timestamp += 0.001;
        assert_eq!(
            fit_vector_ar1_forecast(
                &shifted,
                Intervention::FeedbackCutFrom { sample_index: 30 },
                config(),
            ),
            Err(BenchmarkError::TimestampMismatch)
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

    fn design_candidate(
        id: &str,
        intervention: Intervention,
        onset_index: usize,
        cost: f64,
        feasible: bool,
    ) -> InterventionCandidate {
        InterventionCandidate {
            id: id.to_owned(),
            intervention,
            onset_index,
            cost,
            feasible,
        }
    }

    #[test]
    fn robust_design_selects_feedback_cut_over_weaker_phase_pulse() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ];
        let candidates = [
            design_candidate(
                "phase-pulse",
                Intervention::PhasePulseAt {
                    sample_index: 20,
                    delta_phase_radians: 0.6,
                },
                20,
                1.0,
                true,
            ),
            design_candidate(
                "feedback-cut",
                Intervention::FeedbackCutFrom { sample_index: 20 },
                20,
                1.0,
                true,
            ),
        ];
        let decision =
            design_discriminating_intervention(&mechanisms, &candidates, config(), 0.0).unwrap();
        let InterventionDesignDecision::Selected { design } = decision else {
            panic!("expected a discriminating action");
        };
        assert_eq!(design.candidate_id, "feedback-cut");
        assert_eq!(design.pairwise.len(), 1);
        assert!(design.worst_pairwise_rmse > 0.1);
        assert_eq!(
            design.maximin_utility,
            design.worst_pairwise_rmse / design.cost
        );
    }

    #[test]
    fn intervention_design_normalizes_by_declared_cost() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ];
        let candidates = [
            design_candidate(
                "expensive-cut",
                Intervention::FeedbackCutFrom { sample_index: 20 },
                20,
                100.0,
                true,
            ),
            design_candidate(
                "cheap-pulse",
                Intervention::PhasePulseAt {
                    sample_index: 20,
                    delta_phase_radians: 0.6,
                },
                20,
                0.01,
                true,
            ),
        ];
        let decision =
            design_discriminating_intervention(&mechanisms, &candidates, config(), 0.0).unwrap();
        let InterventionDesignDecision::Selected { design } = decision else {
            panic!("expected a feasible action");
        };
        assert_eq!(design.candidate_id, "cheap-pulse");
    }

    #[test]
    fn intervention_design_abstains_when_a_model_pair_remains_equivalent() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::HybridFeedbackCoupled,
        ];
        let candidates = [design_candidate(
            "feedback-cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        )];
        let decision =
            design_discriminating_intervention(&mechanisms, &candidates, config(), 1.0e-6).unwrap();
        assert!(matches!(
            decision,
            InterventionDesignDecision::Abstained {
                worst_pairwise_rmse,
                ..
            } if worst_pairwise_rmse <= 1.0e-12
        ));
    }

    #[test]
    fn infeasible_interventions_are_excluded_before_utility_ranking() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ];
        let candidates = [
            design_candidate(
                "forbidden-cut",
                Intervention::FeedbackCutFrom { sample_index: 20 },
                20,
                1.0e-6,
                false,
            ),
            design_candidate(
                "allowed-pulse",
                Intervention::PhasePulseAt {
                    sample_index: 20,
                    delta_phase_radians: 0.6,
                },
                20,
                1.0,
                true,
            ),
        ];
        let decision =
            design_discriminating_intervention(&mechanisms, &candidates, config(), 0.0).unwrap();
        let InterventionDesignDecision::Selected { design } = decision else {
            panic!("expected one feasible action");
        };
        assert_eq!(design.candidate_id, "allowed-pulse");
    }

    #[test]
    fn intervention_design_rejects_duplicate_ids_invalid_cost_and_no_feasible_actions() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ];
        let pulse = Intervention::PhasePulseAt {
            sample_index: 20,
            delta_phase_radians: 0.5,
        };
        assert_eq!(
            design_discriminating_intervention(
                &mechanisms,
                &[
                    design_candidate("same", pulse, 20, 1.0, true),
                    design_candidate("same", pulse, 20, 1.0, true),
                ],
                config(),
                0.0
            ),
            Err(BenchmarkError::DuplicateInterventionCandidate)
        );
        assert_eq!(
            design_discriminating_intervention(
                &mechanisms,
                &[design_candidate("zero-cost", pulse, 20, 0.0, true)],
                config(),
                0.0
            ),
            Err(BenchmarkError::InvalidInterventionCost)
        );
        assert_eq!(
            design_discriminating_intervention(
                &mechanisms,
                &[design_candidate("blocked", pulse, 20, 1.0, false)],
                config(),
                0.0
            ),
            Err(BenchmarkError::NoFeasibleInterventions)
        );
    }

    #[test]
    fn robust_design_phase_pulse_separates_intrinsic_from_hybrid() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::HybridFeedbackCoupled,
        ];
        let nominal = config();
        let weaker_coupling = SimulationConfig {
            feedback_coupling: 1.0,
            ..nominal
        };
        let candidates = [
            design_candidate(
                "feedback-cut",
                Intervention::FeedbackCutFrom { sample_index: 20 },
                20,
                1.0,
                true,
            ),
            design_candidate(
                "phase-pulse",
                Intervention::PhasePulseAt {
                    sample_index: 20,
                    delta_phase_radians: 0.6,
                },
                20,
                1.0,
                true,
            ),
        ];
        let decision = design_discriminating_intervention_across_scenarios(
            &mechanisms,
            &candidates,
            &[nominal, weaker_coupling],
            0.0,
        )
        .unwrap();
        let InterventionDesignDecision::Selected { design } = decision else {
            panic!("expected a robustly discriminating action");
        };
        assert_eq!(design.candidate_id, "phase-pulse");
        assert_eq!(design.scenario_count, 2);
        assert_eq!(design.pairwise.len(), 2);
        assert_eq!(design.pairwise[0].scenario_index, 0);
        assert_eq!(design.pairwise[1].scenario_index, 1);
    }

    #[test]
    fn robust_design_rejects_empty_or_mismatched_parameter_scenarios() {
        let mechanisms = [
            Mechanism::IntrinsicOscillator,
            Mechanism::SensoryFeedbackDriven,
        ];
        let candidates = [design_candidate(
            "feedback-cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        )];
        assert_eq!(
            design_discriminating_intervention_across_scenarios(&mechanisms, &candidates, &[], 0.0),
            Err(BenchmarkError::EmptyParameterScenarioSet)
        );

        let mismatched = SimulationConfig {
            sample_interval: 0.04,
            ..config()
        };
        assert_eq!(
            design_discriminating_intervention_across_scenarios(
                &mechanisms,
                &candidates,
                &[config(), mismatched],
                0.0
            ),
            Err(BenchmarkError::InvalidParameterScenarioGrid)
        );
    }

    fn information_hypothesis(
        id: &str,
        mechanism: Mechanism,
        config: SimulationConfig,
        prior_weight: f64,
    ) -> MechanismHypothesis {
        MechanismHypothesis {
            id: id.to_owned(),
            mechanism,
            config,
            prior_weight,
        }
    }

    #[test]
    fn finite_hypothesis_eig_selects_a_discriminating_perturbation() {
        let hypotheses = [
            information_hypothesis("intrinsic", Mechanism::IntrinsicOscillator, config(), 0.5),
            information_hypothesis("sensory", Mechanism::SensoryFeedbackDriven, config(), 0.5),
        ];
        let candidates = [
            design_candidate(
                "feedback-cut",
                Intervention::FeedbackCutFrom { sample_index: 20 },
                20,
                1.0,
                true,
            ),
            design_candidate(
                "phase-pulse",
                Intervention::PhasePulseAt {
                    sample_index: 20,
                    delta_phase_radians: 0.6,
                },
                20,
                1.0,
                true,
            ),
        ];
        let decision = design_intervention_by_expected_information_gain(
            &hypotheses,
            &candidates,
            0.04,
            256,
            0xE16A_2026,
            0.0,
        )
        .unwrap();
        let ExpectedInformationDecision::Selected { design } = decision else {
            panic!("expected a positive-information action");
        };
        assert!(design.expected_information_nats > 0.5);
        assert!(design.expected_information_nats <= std::f64::consts::LN_2 + 0.05);
        assert!(design.standard_error_nats.is_finite());
        assert!(design.information_per_cost > 0.5);
        assert_eq!(design.draws, 256);
    }

    #[test]
    fn posterior_update_moves_weight_toward_the_intervention_response_that_occurred() {
        let cfg = config();
        let hypotheses = vec![
            MechanismHypothesis {
                id: "intrinsic".to_owned(),
                mechanism: Mechanism::IntrinsicOscillator,
                config: cfg,
                prior_weight: 1.0,
            },
            MechanismHypothesis {
                id: "sensory".to_owned(),
                mechanism: Mechanism::SensoryFeedbackDriven,
                config: cfg,
                prior_weight: 1.0,
            },
        ];
        let action = Intervention::FeedbackCutFrom { sample_index: 30 };
        let observed = simulate(Mechanism::IntrinsicOscillator, action, cfg).unwrap();
        let update = update_hypothesis_posterior_after_intervention(
            &hypotheses,
            action,
            &observed.samples[30..],
            cfg.observation_variance,
        )
        .unwrap();

        let intrinsic = update
            .posterior
            .iter()
            .find(|item| item.hypothesis_id == "intrinsic")
            .unwrap();
        let sensory = update
            .posterior
            .iter()
            .find(|item| item.hypothesis_id == "sensory")
            .unwrap();
        assert!(intrinsic.probability > 0.999);
        assert!(sensory.probability < 0.001);
        assert!(update.information_gain_nats > 0.0);
        assert!(update.log_marginal_evidence.is_finite());
    }

    #[test]
    fn posterior_update_preserves_prior_for_identical_hypotheses() {
        let cfg = config();
        let hypotheses = vec![
            MechanismHypothesis {
                id: "prior-a".to_owned(),
                mechanism: Mechanism::IntrinsicOscillator,
                config: cfg,
                prior_weight: 3.0,
            },
            MechanismHypothesis {
                id: "prior-b".to_owned(),
                mechanism: Mechanism::IntrinsicOscillator,
                config: cfg,
                prior_weight: 1.0,
            },
        ];
        let action = Intervention::PhasePulseAt {
            sample_index: 30,
            delta_phase_radians: 0.3,
        };
        let observed = simulate(Mechanism::IntrinsicOscillator, action, cfg).unwrap();
        let update = update_hypothesis_posterior_after_intervention(
            &hypotheses,
            action,
            &observed.samples[30..],
            cfg.observation_variance,
        )
        .unwrap();

        let a = update
            .posterior
            .iter()
            .find(|item| item.hypothesis_id == "prior-a")
            .unwrap();
        let b = update
            .posterior
            .iter()
            .find(|item| item.hypothesis_id == "prior-b")
            .unwrap();
        assert!((a.probability - 0.75).abs() < 1.0e-10);
        assert!((b.probability - 0.25).abs() < 1.0e-10);
        assert!(update.information_gain_nats.abs() < 1.0e-10);
    }

    #[test]
    fn posterior_update_is_caller_order_invariant_and_can_be_applied_for_next_eig_step() {
        let cfg = config();
        let a = MechanismHypothesis {
            id: "oscillator".to_owned(),
            mechanism: Mechanism::IntrinsicOscillator,
            config: cfg,
            prior_weight: 1.0,
        };
        let b = MechanismHypothesis {
            id: "sensory".to_owned(),
            mechanism: Mechanism::SensoryFeedbackDriven,
            config: cfg,
            prior_weight: 1.0,
        };
        let action = Intervention::FeedbackCutFrom { sample_index: 30 };
        let observed = simulate(Mechanism::SensoryFeedbackDriven, action, cfg).unwrap();
        let update_ab = update_hypothesis_posterior_after_intervention(
            &[a.clone(), b.clone()],
            action,
            &observed.samples[30..],
            cfg.observation_variance,
        )
        .unwrap();
        let update_ba = update_hypothesis_posterior_after_intervention(
            &[b.clone(), a.clone()],
            action,
            &observed.samples[30..],
            cfg.observation_variance,
        )
        .unwrap();
        assert_eq!(update_ab.posterior, update_ba.posterior);

        assert!(update_ab.probability_floor_applied);
        let posterior_hypotheses = apply_hypothesis_posterior(&[a, b], &update_ab).unwrap();
        let total = posterior_hypotheses
            .iter()
            .map(|hypothesis| hypothesis.prior_weight)
            .sum::<f64>();
        assert!((total - 1.0).abs() < 1.0e-12);
        assert!(
            posterior_hypotheses
                .iter()
                .any(|hypothesis| hypothesis.id == "sensory" && hypothesis.prior_weight > 0.999)
        );
    }

    #[test]
    fn posterior_update_rejects_missing_or_misaligned_observations_and_invalid_hypotheses() {
        let cfg = config();
        let hypotheses = vec![
            MechanismHypothesis {
                id: "one".to_owned(),
                mechanism: Mechanism::IntrinsicOscillator,
                config: cfg,
                prior_weight: 1.0,
            },
            MechanismHypothesis {
                id: "two".to_owned(),
                mechanism: Mechanism::SensoryFeedbackDriven,
                config: cfg,
                prior_weight: 1.0,
            },
        ];
        let action = Intervention::FeedbackCutFrom { sample_index: 30 };
        let observed = simulate(Mechanism::IntrinsicOscillator, action, cfg).unwrap();
        assert_eq!(
            update_hypothesis_posterior_after_intervention(
                &hypotheses,
                action,
                &observed.samples[29..],
                cfg.observation_variance
            ),
            Err(BenchmarkError::InvalidTraceLength)
        );
        let mut shifted = observed.samples[30..].to_vec();
        shifted[0].timestamp += 0.001;
        assert_eq!(
            update_hypothesis_posterior_after_intervention(
                &hypotheses,
                action,
                &shifted,
                cfg.observation_variance
            ),
            Err(BenchmarkError::TimestampMismatch)
        );
        assert_eq!(
            update_hypothesis_posterior_after_intervention(
                &hypotheses,
                Intervention::None,
                &observed.samples[30..],
                cfg.observation_variance
            ),
            Err(BenchmarkError::InvalidInterventionAlignment)
        );
    }

    #[test]
    fn applying_a_posterior_rejects_duplicate_or_missing_hypothesis_ids() {
        let cfg = config();
        let hypotheses = vec![
            MechanismHypothesis {
                id: "one".to_owned(),
                mechanism: Mechanism::IntrinsicOscillator,
                config: cfg,
                prior_weight: 1.0,
            },
            MechanismHypothesis {
                id: "two".to_owned(),
                mechanism: Mechanism::SensoryFeedbackDriven,
                config: cfg,
                prior_weight: 1.0,
            },
        ];
        let invalid = HypothesisPosteriorUpdate {
            posterior: vec![
                HypothesisPosteriorWeight {
                    hypothesis_id: "one".to_owned(),
                    probability: 0.5,
                },
                HypothesisPosteriorWeight {
                    hypothesis_id: "extra".to_owned(),
                    probability: 0.5,
                },
            ],
            log_marginal_evidence: 0.0,
            prior_entropy_nats: 0.0,
            posterior_entropy_nats: 0.0,
            information_gain_nats: 0.0,
            probability_floor_applied: false,
        };
        assert_eq!(
            apply_hypothesis_posterior(&hypotheses, &invalid),
            Err(BenchmarkError::InvalidPosteriorUpdate)
        );
    }

    #[test]
    fn finite_hypothesis_eig_is_zero_for_observationally_identical_hypotheses() {
        let hypotheses = [
            information_hypothesis(
                "oscillator-copy-a",
                Mechanism::IntrinsicOscillator,
                config(),
                0.8,
            ),
            information_hypothesis(
                "oscillator-copy-b",
                Mechanism::IntrinsicOscillator,
                config(),
                0.2,
            ),
        ];
        let candidates = [design_candidate(
            "feedback-cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        )];
        let decision = design_intervention_by_expected_information_gain(
            &hypotheses,
            &candidates,
            0.04,
            64,
            99,
            1.0e-8,
        )
        .unwrap();
        assert!(matches!(
            decision,
            ExpectedInformationDecision::Abstained {
                information_per_cost,
                ..
            } if information_per_cost.abs() < 1.0e-10
        ));
    }

    #[test]
    fn finite_hypothesis_eig_is_deterministic_and_candidate_order_invariant() {
        let hypotheses = [
            information_hypothesis("intrinsic", Mechanism::IntrinsicOscillator, config(), 0.7),
            information_hypothesis("sensory", Mechanism::SensoryFeedbackDriven, config(), 0.3),
        ];
        let cut = design_candidate(
            "cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        );
        let pulse = design_candidate(
            "pulse",
            Intervention::PhasePulseAt {
                sample_index: 20,
                delta_phase_radians: 0.6,
            },
            20,
            1.0,
            true,
        );
        let forward = design_intervention_by_expected_information_gain(
            &hypotheses,
            &[cut.clone(), pulse.clone()],
            0.05,
            128,
            1234,
            0.0,
        )
        .unwrap();
        let reverse = design_intervention_by_expected_information_gain(
            &hypotheses,
            &[pulse, cut],
            0.05,
            128,
            1234,
            0.0,
        )
        .unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn finite_hypothesis_eig_rejects_bad_priors_scenarios_and_duplicate_ids() {
        let mut hypotheses = [
            information_hypothesis("intrinsic", Mechanism::IntrinsicOscillator, config(), 0.5),
            information_hypothesis("sensory", Mechanism::SensoryFeedbackDriven, config(), 0.5),
        ];
        let candidates = [design_candidate(
            "feedback-cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        )];
        hypotheses[1].prior_weight = 0.0;
        assert_eq!(
            design_intervention_by_expected_information_gain(
                &hypotheses,
                &candidates,
                0.04,
                32,
                1,
                0.0
            ),
            Err(BenchmarkError::InvalidHypothesisPrior)
        );

        hypotheses[1].prior_weight = 0.5;
        hypotheses[1].id = hypotheses[0].id.clone();
        assert_eq!(
            design_intervention_by_expected_information_gain(
                &hypotheses,
                &candidates,
                0.04,
                32,
                1,
                0.0
            ),
            Err(BenchmarkError::DuplicateHypothesis)
        );

        hypotheses[1].id = "sensory".to_owned();
        hypotheses[1].config.sample_interval = 0.04;
        assert_eq!(
            design_intervention_by_expected_information_gain(
                &hypotheses,
                &candidates,
                0.04,
                32,
                1,
                0.0
            ),
            Err(BenchmarkError::InvalidParameterScenarioGrid)
        );
    }

    #[test]
    fn finite_hypothesis_eig_respects_nonuniform_prior_entropy_within_sampling_error() {
        let hypotheses = [
            information_hypothesis("intrinsic", Mechanism::IntrinsicOscillator, config(), 0.9),
            information_hypothesis("sensory", Mechanism::SensoryFeedbackDriven, config(), 0.1),
        ];
        let candidates = [design_candidate(
            "feedback-cut",
            Intervention::FeedbackCutFrom { sample_index: 20 },
            20,
            1.0,
            true,
        )];
        let decision = design_intervention_by_expected_information_gain(
            &hypotheses,
            &candidates,
            0.04,
            2048,
            0xB4A9_2026,
            0.0,
        )
        .unwrap();
        let ExpectedInformationDecision::Selected { design } = decision else {
            panic!("expected information gain for identifiable hypotheses");
        };
        let prior_entropy = -(0.9_f64 * 0.9_f64.ln() + 0.1_f64 * 0.1_f64.ln());
        assert!(design.expected_information_nats > 0.2);
        assert!(design.expected_information_nats <= prior_entropy + 0.06);
        assert!(design.standard_error_nats < 0.06);
    }

    #[test]
    fn finite_hypothesis_eig_selects_lower_cost_action_and_ignores_infeasible_candidates() {
        let hypotheses = [
            information_hypothesis("intrinsic", Mechanism::IntrinsicOscillator, config(), 0.5),
            information_hypothesis("sensory", Mechanism::SensoryFeedbackDriven, config(), 0.5),
        ];
        let cut = Intervention::FeedbackCutFrom { sample_index: 20 };
        let candidates = [
            design_candidate("expensive-cut", cut, 20, 10.0, true),
            design_candidate("cheap-cut", cut, 20, 1.0, true),
            design_candidate("blocked-cheap-cut", cut, 20, 0.01, false),
        ];
        let decision = design_intervention_by_expected_information_gain(
            &hypotheses,
            &candidates,
            0.04,
            512,
            0xC057_2026,
            0.0,
        )
        .unwrap();
        let ExpectedInformationDecision::Selected { design } = decision else {
            panic!("expected a positive information-per-cost action");
        };
        assert_eq!(design.candidate_id, "cheap-cut");
        assert!(design.information_per_cost > 0.0);
    }

    #[test]
    fn profile_likelihood_resolves_a_well_sampled_frequency() {
        let mut known = config();
        known.sample_count = 160;
        known.sample_interval = 0.025;
        known.angular_frequency = 4.0;
        known.observation_variance = 0.0025;
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, known).unwrap();
        let observed =
            add_gaussian_observation_noise(&generated.samples, 0.0025, 0xF10F_2026).unwrap();
        let profile = profile_angular_frequency(
            &observed,
            Mechanism::IntrinsicOscillator,
            known,
            FrequencyPhaseGrid {
                minimum_angular_frequency: 3.5,
                maximum_angular_frequency: 4.5,
                frequency_steps: 101,
                phase_steps: 96,
            },
            1.920_729_410_347_062,
        )
        .unwrap();

        assert!((profile.best_angular_frequency - 4.0).abs() <= 0.02);
        assert_eq!(profile.identifiability, FrequencyIdentifiability::Resolved);
        assert!(!profile.touches_lower_boundary);
        assert!(!profile.touches_upper_boundary);
        assert_eq!(profile.accepted_intervals.len(), 1);
    }

    #[test]
    fn profile_likelihood_reports_two_frequency_modes_under_exact_sampling_alias() {
        let alias_frequency = 2.5_f64;
        let mut alias_config = config();
        alias_config.sample_count = 12;
        alias_config.sample_interval = std::f64::consts::PI;
        alias_config.angular_frequency = 0.5;
        let generated = simulate(
            Mechanism::IntrinsicOscillator,
            Intervention::None,
            alias_config,
        )
        .unwrap();
        let profile = profile_angular_frequency(
            &generated.samples,
            Mechanism::IntrinsicOscillator,
            alias_config,
            FrequencyPhaseGrid {
                minimum_angular_frequency: 0.4,
                maximum_angular_frequency: 2.6,
                frequency_steps: 221,
                phase_steps: 64,
            },
            1.0e-7,
        )
        .unwrap();

        assert!(
            profile
                .accepted_intervals
                .iter()
                .any(|interval| (interval.lower_angular_frequency - 0.5).abs() <= 0.02)
        );
        assert!(
            profile
                .accepted_intervals
                .iter()
                .any(|interval| (interval.lower_angular_frequency - alias_frequency).abs() <= 0.02)
        );
        assert_eq!(
            profile.identifiability,
            FrequencyIdentifiability::Multimodal
        );
    }

    #[test]
    fn profile_likelihood_marks_a_best_fit_on_the_search_boundary() {
        let mut outside = config();
        outside.angular_frequency = 5.0;
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, outside).unwrap();
        let profile = profile_angular_frequency(
            &generated.samples,
            Mechanism::IntrinsicOscillator,
            outside,
            FrequencyPhaseGrid {
                minimum_angular_frequency: 3.5,
                maximum_angular_frequency: 4.5,
                frequency_steps: 51,
                phase_steps: 64,
            },
            1.920_729_410_347_062,
        )
        .unwrap();

        assert_eq!(
            profile.identifiability,
            FrequencyIdentifiability::BoundaryLimited
        );
        assert!(profile.touches_upper_boundary);
    }

    #[test]
    fn profile_likelihood_rejects_invalid_cutoff_and_misaligned_time_grid() {
        let generated =
            simulate(Mechanism::IntrinsicOscillator, Intervention::None, config()).unwrap();
        let grid = quick_fit_grid();
        assert_eq!(
            profile_angular_frequency(
                &generated.samples,
                Mechanism::IntrinsicOscillator,
                config(),
                grid,
                -1.0,
            ),
            Err(BenchmarkError::InvalidProfileLikelihoodCutoff)
        );
        let mut shifted = generated.samples.clone();
        shifted[4].timestamp += 0.001;
        assert_eq!(
            profile_angular_frequency(
                &shifted,
                Mechanism::IntrinsicOscillator,
                config(),
                grid,
                1.0,
            ),
            Err(BenchmarkError::TimestampMismatch)
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
