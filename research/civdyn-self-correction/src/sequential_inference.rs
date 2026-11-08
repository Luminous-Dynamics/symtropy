// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Anytime-valid sequential tests for the frozen Gaussian paired-outcome model.
//!
//! Three one-sided Gaussian e-processes distinguish a negative treatment
//! effect, an effect below the declared action-benefit threshold, and evidence
//! of a beneficial effect. Each is a nonnegative test supermartingale under
//! its stated composite null when paired differences are independent Gaussian
//! observations with the declared known variance. This is not distribution-free.

use crate::{
    CorrectionEvent, CorrectionStage, validate_full_correction_trace,
    causal_drift::{ACTION_COST, CAUSAL_DRIFT_TICKS, CausalRegime},
    noisy_causal_drift::{
        MATCHED_PAIRS_PER_LOOK, NOISY_SEEDS, OUTCOME_NOISE_STANDARD_DEVIATION,
        paired_difference_stream, noisy_episode_seed,
    },
};

pub const PREDECLARED_LOOK_TICKS: [usize; 4] = [2, 4, 6, 8];
pub const SEQUENTIAL_HYPOTHESES_PER_LOOK: usize = 3;
pub const SEQUENTIAL_ALPHA: f64 = 0.05;
pub const ALPHA_PER_HYPOTHESIS: f64 =
    SEQUENTIAL_ALPHA / (PREDECLARED_LOOK_TICKS.len() * SEQUENTIAL_HYPOTHESES_PER_LOOK) as f64;
pub const E_VALUE_CROSSING_THRESHOLD: f64 = 240.0;
pub const PAIRED_DIFFERENCE_KNOWN_VARIANCE: f64 =
    2.0 * OUTCOME_NOISE_STANDARD_DEVIATION * OUTCOME_NOISE_STANDARD_DEVIATION;
pub const NEGATIVE_EFFECT_BETTING_PARAMETER: f64 =
    -1.0 / PAIRED_DIFFERENCE_KNOWN_VARIANCE;
pub const BELOW_THRESHOLD_BETTING_PARAMETER: f64 =
    -0.2 / PAIRED_DIFFERENCE_KNOWN_VARIANCE;
pub const BENEFICIAL_EFFECT_BETTING_PARAMETER: f64 =
    0.5 / PAIRED_DIFFERENCE_KNOWN_VARIANCE;
pub const MAX_PAIRS_PER_LOOK: usize = MATCHED_PAIRS_PER_LOOK;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialDiagnosis {
    NegativeCausalEffect,
    EffectNotAboveActionThreshold,
    ConfounderShiftWithBeneficialEffect,
}

impl SequentialDiagnosis {
    pub const fn name(self) -> &'static str {
        match self {
            Self::NegativeCausalEffect => "negative_causal_effect",
            Self::EffectNotAboveActionThreshold => "effect_not_above_action_threshold",
            Self::ConfounderShiftWithBeneficialEffect => {
                "confounder_shift_with_beneficial_effect"
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EProcess {
    pub log_value: f64,
    pub betting_parameter: f64,
    pub null_boundary: f64,
    pub observations: usize,
}

impl EProcess {
    pub const fn new(betting_parameter: f64, null_boundary: f64) -> Self {
        Self {
            log_value: 0.0,
            betting_parameter,
            null_boundary,
            observations: 0,
        }
    }

    /// Update log E_n using the Gaussian likelihood-ratio increment:
    /// lambda * (X - boundary) - lambda^2 * variance / 2.
    ///
    /// Under a Gaussian null on the null side of the boundary, the expected
    /// multiplicative increment is exp(lambda * (mu - boundary)) <= 1.
    pub fn observe(&mut self, paired_difference: f64) {
        let lambda = self.betting_parameter;
        self.log_value += lambda * (paired_difference - self.null_boundary)
            - 0.5 * lambda * lambda * PAIRED_DIFFERENCE_KNOWN_VARIANCE;
        self.observations += 1;
    }

    pub fn crossed(&self) -> bool {
        self.log_value >= E_VALUE_CROSSING_THRESHOLD.ln()
    }

    pub fn e_value(&self) -> f64 {
        self.log_value.exp()
    }
}

/// Exact expectation of one e-process multiplicative increment under the
/// declared Gaussian model. Used by analytic null-side unit tests.
pub fn expected_e_increment(
    betting_parameter: f64,
    true_mean: f64,
    null_boundary: f64,
) -> f64 {
    (betting_parameter * (true_mean - null_boundary)).exp()
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnytimeCausalOutcome {
    pub seed_index: usize,
    pub seed: u64,
    pub regime: CausalRegime,
    pub signal_detection_tick: Option<usize>,
    pub diagnosis: Option<SequentialDiagnosis>,
    pub diagnosis_tick: Option<usize>,
    pub correction_latency_ticks: Option<usize>,
    pub active_ticks: usize,
    pub active_after_nonpositive_effect_ticks: usize,
    pub missed_beneficial_action_ticks: usize,
    pub sample_pairs_per_look: [usize; 4],
    pub total_sample_pairs: usize,
    pub sample_looks_used: usize,
    pub final_log_e_negative_effect: f64,
    pub final_log_e_below_threshold: f64,
    pub final_log_e_beneficial_effect: f64,
    pub policy_stopped: bool,
    pub trace: Vec<CorrectionEvent>,
    pub trace_valid: bool,
}

fn event(stage: CorrectionStage, tick: usize, evidence_code: &'static str) -> CorrectionEvent {
    CorrectionEvent {
        stage,
        tick,
        evidence_code,
    }
}

fn correction_trace(
    signal_tick: Option<usize>,
    diagnosis_tick: usize,
    diagnosis: SequentialDiagnosis,
) -> Vec<CorrectionEvent> {
    let observed_tick = signal_tick.unwrap_or(diagnosis_tick);
    let (diagnosis_code, update_code, action_code) = match diagnosis {
        SequentialDiagnosis::NegativeCausalEffect => (
            "negative_effect_e_process_crossed_composite_null",
            "record_negative_causal_effect_with_anytime_valid_evidence",
            "suspend_intervention_from_next_tick",
        ),
        SequentialDiagnosis::EffectNotAboveActionThreshold => (
            "below_benefit_threshold_e_process_crossed_composite_null",
            "record_insufficient_benefit_without_claiming_negative_effect",
            "suspend_intervention_from_next_tick",
        ),
        SequentialDiagnosis::ConfounderShiftWithBeneficialEffect => (
            "observational_shift_with_positive_effect_evidence",
            "revise_observational_model_not_the_causal_effect",
            "retain_beneficial_intervention",
        ),
    };
    vec![
        event(CorrectionStage::Observed, observed_tick, "observational_signal_or_paired_outcomes"),
        event(CorrectionStage::Detected, observed_tick, "anytime_valid_e_process_boundary"),
        event(CorrectionStage::Diagnosed, diagnosis_tick, diagnosis_code),
        event(
            CorrectionStage::Discriminated,
            diagnosis_tick,
            "matched_pair_e_processes_with_known_variance",
        ),
        event(CorrectionStage::BeliefUpdated, diagnosis_tick, update_code),
        event(CorrectionStage::Intervened, diagnosis_tick + 1, action_code),
        event(
            CorrectionStage::Audited,
            CAUSAL_DRIFT_TICKS,
            "sample_count_e_value_and_policy_receipt_reconciled",
        ),
    ]
}

#[derive(Debug, Clone, Copy, Default)]
struct SignalCusum {
    baseline: Option<f64>,
    positive: f64,
    negative: f64,
    detection_tick: Option<usize>,
}

impl SignalCusum {
    fn observe(&mut self, value: f64, tick: usize) {
        let Some(baseline) = self.baseline else {
            self.baseline = Some(value);
            return;
        };
        if self.detection_tick.is_some() {
            return;
        }
        let deviation = value - baseline;
        self.positive = (self.positive + deviation - 0.25).max(0.0);
        self.negative = (self.negative - deviation - 0.25).max(0.0);
        if self.positive >= 2.0 || self.negative >= 2.0 {
            self.detection_tick = Some(tick);
        }
    }
}

pub fn run_anytime_causal_episode(
    regime: CausalRegime,
    seed_index: usize,
) -> AnytimeCausalOutcome {
    let seed = noisy_episode_seed(regime, seed_index);
    let mut signal_detector = SignalCusum::default();
    let mut policy_enabled = true;
    let mut active_ticks = 0;
    let mut active_after_nonpositive_effect_ticks = 0;
    let mut missed_beneficial_action_ticks = 0;
    let mut sample_pairs_per_look = [0; 4];
    let mut total_sample_pairs = 0;
    let mut sample_looks_used = 0;
    let mut diagnosis = None;
    let mut diagnosis_tick = None;
    let mut previous_look_confirmed_benefit = false;
    let mut final_log_e_negative_effect = 0.0;
    let mut final_log_e_below_threshold = 0.0;
    let mut final_log_e_beneficial_effect = 0.0;
    let mut trace = Vec::new();
    let mut look_index = 0usize;

    for tick in 1..=CAUSAL_DRIFT_TICKS {
        let true_effect = regime.causal_effect(tick);
        let observed_association =
            true_effect + regime.observational_confounder_imbalance(tick);
        signal_detector.observe(observed_association, tick);

        if policy_enabled {
            active_ticks += 1;
            if true_effect <= 0.0 {
                active_after_nonpositive_effect_ticks += 1;
            }
        } else if true_effect > ACTION_COST {
            missed_beneficial_action_ticks += 1;
        }

        if !policy_enabled || diagnosis.is_some() || !PREDECLARED_LOOK_TICKS.contains(&tick) {
            continue;
        }

        let mut negative_effect = EProcess::new(NEGATIVE_EFFECT_BETTING_PARAMETER, 0.0);
        let mut below_threshold =
            EProcess::new(BELOW_THRESHOLD_BETTING_PARAMETER, ACTION_COST);
        let mut beneficial_effect =
            EProcess::new(BENEFICIAL_EFFECT_BETTING_PARAMETER, ACTION_COST);
        let mut look_confirmed_benefit = false;

        let stream = paired_difference_stream(
            regime,
            tick,
            seed_index,
            MAX_PAIRS_PER_LOOK,
        )
        .expect("maximum paired sample count must be at least two");

        for paired_difference in stream {
            negative_effect.observe(paired_difference);
            below_threshold.observe(paired_difference);
            beneficial_effect.observe(paired_difference);
            sample_pairs_per_look[look_index] += 1;
            total_sample_pairs += 1;

            if negative_effect.crossed() {
                diagnosis = Some(SequentialDiagnosis::NegativeCausalEffect);
                break;
            }
            if below_threshold.crossed() {
                diagnosis = Some(SequentialDiagnosis::EffectNotAboveActionThreshold);
                break;
            }
            if beneficial_effect.crossed() {
                look_confirmed_benefit = true;
                break;
            }
        }

        final_log_e_negative_effect = negative_effect.log_value;
        final_log_e_below_threshold = below_threshold.log_value;
        final_log_e_beneficial_effect = beneficial_effect.log_value;
        sample_looks_used += 1;

        if diagnosis.is_none()
            && signal_detector.detection_tick.is_some()
            && previous_look_confirmed_benefit
            && look_confirmed_benefit
        {
            diagnosis = Some(SequentialDiagnosis::ConfounderShiftWithBeneficialEffect);
        }

        previous_look_confirmed_benefit = look_confirmed_benefit;

        if let Some(identified) = diagnosis {
            diagnosis_tick = Some(tick);
            trace = correction_trace(signal_detector.detection_tick, tick, identified);
            if matches!(
                identified,
                SequentialDiagnosis::NegativeCausalEffect
                    | SequentialDiagnosis::EffectNotAboveActionThreshold
            ) {
                // The current tick action has already occurred; stop next tick.
                policy_enabled = false;
            }
        }

        look_index += 1;
    }

    let signal_detection_tick = signal_detector.detection_tick;
    let correction_latency_ticks = diagnosis_tick.map(|diagnosed| {
        signal_detection_tick
            .map(|detected| {
                if diagnosis == Some(SequentialDiagnosis::ConfounderShiftWithBeneficialEffect) {
                    diagnosed - detected
                } else {
                    diagnosed + 1 - detected
                }
            })
            .unwrap_or(1)
    });
    let trace_valid = !trace.is_empty() && validate_full_correction_trace(&trace).is_ok();

    AnytimeCausalOutcome {
        seed_index,
        seed,
        regime,
        signal_detection_tick,
        diagnosis,
        diagnosis_tick,
        correction_latency_ticks,
        active_ticks,
        active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks,
        sample_pairs_per_look,
        total_sample_pairs,
        sample_looks_used,
        final_log_e_negative_effect,
        final_log_e_below_threshold,
        final_log_e_beneficial_effect,
        policy_stopped: !policy_enabled,
        trace,
        trace_valid,
    }
}

pub fn run_anytime_causal_portfolio() -> Vec<AnytimeCausalOutcome> {
    let mut outcomes = Vec::with_capacity(CausalRegime::ALL.len() * NOISY_SEEDS);
    for regime in CausalRegime::ALL {
        for seed_index in 0..NOISY_SEEDS {
            outcomes.push(run_anytime_causal_episode(regime, seed_index));
        }
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noisy_causal_drift::paired_difference_stream;

    #[test]
    fn gaussian_e_process_is_a_supermartingale_on_each_composite_null_side() {
        for true_mean in [0.0, 0.5, 1.0, 2.0] {
            assert!(
                expected_e_increment(
                    NEGATIVE_EFFECT_BETTING_PARAMETER,
                    true_mean.max(0.0),
                    0.0
                ) <= 1.0
            );
            assert!(
                expected_e_increment(
                    BELOW_THRESHOLD_BETTING_PARAMETER,
                    true_mean.max(ACTION_COST),
                    ACTION_COST
                ) <= 1.0
            );
            assert!(
                expected_e_increment(
                    BENEFICIAL_EFFECT_BETTING_PARAMETER,
                    true_mean.min(ACTION_COST),
                    ACTION_COST
                ) <= 1.0
            );
        }
        assert_eq!(
            E_VALUE_CROSSING_THRESHOLD,
            1.0 / ALPHA_PER_HYPOTHESIS
        );
        assert_eq!(
            PREDECLARED_LOOK_TICKS.len() * SEQUENTIAL_HYPOTHESES_PER_LOOK,
            12
        );
    }

    #[test]
    fn e_process_decisions_use_identical_nested_paired_streams() {
        let short: Vec<_> = paired_difference_stream(CausalRegime::SignReversal, 4, 19, 64)
            .expect("stream")
            .collect();
        let long: Vec<_> = paired_difference_stream(CausalRegime::SignReversal, 4, 19, 128)
            .expect("stream")
            .collect();
        assert_eq!(short, long[..64]);
    }

    #[test]
    fn negative_effect_is_distinguished_from_below_threshold() {
        for seed in 0..NOISY_SEEDS {
            let negative = run_anytime_causal_episode(CausalRegime::SignReversal, seed);
            assert_eq!(
                negative.diagnosis,
                Some(SequentialDiagnosis::NegativeCausalEffect)
            );
            assert!(negative.policy_stopped);
            assert!(negative.trace_valid);

            let gradual =
                run_anytime_causal_episode(CausalRegime::GradualSignReversal, seed);
            assert_eq!(
                gradual.diagnosis,
                Some(SequentialDiagnosis::NegativeCausalEffect)
            );
            assert!(gradual.policy_stopped);

            let null = run_anytime_causal_episode(CausalRegime::ConfoundedNull, seed);
            assert!(matches!(
                null.diagnosis,
                Some(
                    SequentialDiagnosis::EffectNotAboveActionThreshold
                        | SequentialDiagnosis::NegativeCausalEffect
                )
            ));
            assert!(null.policy_stopped);
        }
    }

    #[test]
    fn false_negative_sign_diagnoses_under_the_null_respect_the_evalue_budget() {
        let false_sign = (0..NOISY_SEEDS)
            .filter(|seed| {
                run_anytime_causal_episode(CausalRegime::ConfoundedNull, *seed).diagnosis
                    == Some(SequentialDiagnosis::NegativeCausalEffect)
            })
            .count();
        assert!(
            false_sign <= 5,
            "false negative-effect diagnoses under null: {false_sign} / {NOISY_SEEDS}"
        );
    }

    #[test]
    fn stable_and_confounder_shift_controls_do_not_stop_beneficial_actions() {
        for seed in 0..NOISY_SEEDS {
            let stable = run_anytime_causal_episode(CausalRegime::StablePositive, seed);
            assert_eq!(stable.diagnosis, None);
            assert!(!stable.policy_stopped);
            assert_eq!(stable.active_ticks, CAUSAL_DRIFT_TICKS);

            let shift = run_anytime_causal_episode(CausalRegime::ConfounderShift, seed);
            assert_eq!(
                shift.diagnosis,
                Some(SequentialDiagnosis::ConfounderShiftWithBeneficialEffect)
            );
            assert!(!shift.policy_stopped);
            assert_eq!(shift.missed_beneficial_action_ticks, 0);
            assert!(shift.trace_valid);
        }
    }

    #[test]
    fn anytime_sampling_uses_less_than_the_fixed_maximum_in_each_known_answer_regime() {
        let stable = run_anytime_causal_episode(CausalRegime::StablePositive, 3);
        assert!(stable.total_sample_pairs < 4 * MAX_PAIRS_PER_LOOK);

        let reversal = run_anytime_causal_episode(CausalRegime::SignReversal, 3);
        assert!(reversal.total_sample_pairs < 2 * MAX_PAIRS_PER_LOOK);

        let null = run_anytime_causal_episode(CausalRegime::ConfoundedNull, 3);
        assert!(null.total_sample_pairs < MAX_PAIRS_PER_LOOK);
    }

    #[test]
    fn all_anytime_correction_traces_are_complete_and_replayable() {
        for regime in CausalRegime::ALL {
            let first = run_anytime_causal_episode(regime, 31);
            let second = run_anytime_causal_episode(regime, 31);
            assert_eq!(first, second);
            if first.diagnosis.is_some() {
                assert!(first.trace_valid);
                assert_eq!(first.trace.len(), 7);
            }
        }
    }
}
