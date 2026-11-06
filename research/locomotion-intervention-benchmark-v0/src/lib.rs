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
    SensoryFeedbackDriven,
    HybridFeedbackCoupled,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Intervention {
    None,
    FeedbackCutFrom { sample_index: usize },
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
    pub timestamp: f64,
    /// Observable coordinates; score models on these rather than latent truth.
    pub x: f64,
    pub y: f64,
    /// Generator-known quantities, retained for known-truth diagnostics only.
    pub latent_phase: f64,
    pub latent_amplitude: f64,
    pub feedback_present: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticTrace {
    pub mechanism: Mechanism,
    pub intervention: Intervention,
    pub samples: Vec<SyntheticSample>,
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
    InvalidTraceLength,
    NonFiniteTrace,
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
/// SensoryFeedbackDriven follows a periodic external drive and stops cycling
/// when that feedback is removed.
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

    let mut phase = config.initial_phase;
    let mut amplitude = 1.0_f64;
    let mut samples = Vec::with_capacity(config.sample_count);
    let mut previous_time = 0.0;

    for sample_index in 0..config.sample_count {
        let timestamp = sample_index as f64 * config.sample_interval;
        if !timestamp.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        let feedback_present = match intervention {
            Intervention::FeedbackCutFrom { sample_index: cut_index } => {
                sample_index < cut_index
            }
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
        {
            if sample_index == pulse_index {
                phase += delta_phase_radians;
            }
        }

        let x = amplitude * phase.cos();
        let y = amplitude * phase.sin();
        if !phase.is_finite() || !amplitude.is_finite() || !x.is_finite() || !y.is_finite() {
            return Err(BenchmarkError::NumericalFailure);
        }
        samples.push(SyntheticSample {
            timestamp,
            x,
            y,
            latent_phase: phase,
            latent_amplitude: amplitude,
            feedback_present,
        });
        previous_time = timestamp;
    }

    Ok(SyntheticTrace {
        mechanism,
        intervention,
        samples,
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

    let squared_error = left
        .iter()
        .zip(right)
        .map(|(left_sample, right_sample)| {
            (left_sample.x - right_sample.x).powi(2)
                + (left_sample.y - right_sample.y).powi(2)
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
        let score = predictive_log_score(
            observed,
            &prediction.samples,
            config.observation_variance,
        )?;
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
    if trace.iter().any(|sample| {
        !sample.timestamp.is_finite()
            || !sample.x.is_finite()
            || !sample.y.is_finite()
            || !sample.latent_phase.is_finite()
            || !sample.latent_amplitude.is_finite()
    }) {
        return Err(BenchmarkError::NonFiniteTrace);
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
        let oscillator = simulate(Mechanism::IntrinsicOscillator, Intervention::None, config())
            .unwrap();
        let feedback =
            simulate(Mechanism::SensoryFeedbackDriven, Intervention::None, config()).unwrap();
        assert!(trajectory_rmse(&oscillator.samples, &feedback.samples).unwrap() < 1.0e-12);
    }

    #[test]
    fn selector_abstains_when_baseline_observations_do_not_identify_mechanism() {
        let observed = simulate(
            Mechanism::IntrinsicOscillator,
            Intervention::None,
            config(),
        )
        .unwrap();
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
        let oscillator =
            simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let feedback =
            simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        let final_oscillator = oscillator.samples.last().unwrap();
        let final_feedback = feedback.samples.last().unwrap();

        assert!((final_oscillator.latent_amplitude - 1.0).abs() < 1.0e-12);
        assert!(final_feedback.latent_amplitude < 0.01);
        assert!(trajectory_rmse(&oscillator.samples, &feedback.samples).unwrap() > 0.1);
    }

    #[test]
    fn held_out_feedback_cut_selects_the_intrinsic_oscillator_when_it_generated_data() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let observed =
            simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
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
        let observed =
            simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
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
        let pulse_intrinsic =
            simulate(Mechanism::IntrinsicOscillator, pulse, config()).unwrap();
        let baseline_feedback =
            simulate(Mechanism::SensoryFeedbackDriven, Intervention::None, config()).unwrap();
        let pulse_feedback =
            simulate(Mechanism::SensoryFeedbackDriven, pulse, config()).unwrap();

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
        assert!(trace.samples[1].latent_amplitude < 1.0);
        assert!(trace.samples.last().unwrap().latent_amplitude < 0.01);
        assert!(trace.samples[5].x.is_finite());
        assert!(trace.samples[5].y.is_finite());
    }

    #[test]
    fn hybrid_feedback_coupling_corrects_a_phase_pulse() {
        let pulse = Intervention::PhasePulseAt {
            sample_index: 20,
            delta_phase_radians: 0.6,
        };
        let baseline =
            simulate(Mechanism::HybridFeedbackCoupled, Intervention::None, config()).unwrap();
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
        let baseline =
            simulate(Mechanism::SensoryFeedbackDriven, Intervention::None, config()).unwrap();
        let cut =
            simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
        let offset = phase_offset(&cut.samples, &baseline.samples, config().sample_count - 1)
            .unwrap();
        assert_eq!(offset, None);
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
    fn rejects_invalid_candidate_sets_and_score_margin() {
        let observed = simulate(
            Mechanism::IntrinsicOscillator,
            Intervention::None,
            config(),
        )
        .unwrap();
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

    #[test]
    fn predictive_score_rewards_the_matching_intervention_prediction() {
        let intervention = Intervention::FeedbackCutFrom { sample_index: 20 };
        let observed =
            simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let correct =
            simulate(Mechanism::IntrinsicOscillator, intervention, config()).unwrap();
        let wrong =
            simulate(Mechanism::SensoryFeedbackDriven, intervention, config()).unwrap();
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
}
