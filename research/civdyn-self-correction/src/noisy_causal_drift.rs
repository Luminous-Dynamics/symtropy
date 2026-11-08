// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Seeded noisy matched-probe evaluation for causal-drift diagnosis.
//!
//! Unlike the deterministic known-answer controls, this module draws finite
//! samples with independent outcome noise and uses a predeclared sequential
//! confidence rule. The PRNG and scenario seeds are fixed so the entire sweep
//! is replayable. Results remain synthetic and do not establish external
//! validity or real-world causal identification.

use crate::{
    CorrectionEvent, CorrectionStage,
    causal_drift::{
        ACTION_COST, CAUSAL_DRIFT_TICKS, CausalDiagnosis, CausalDriftController, CausalRegime,
        run_causal_drift_scenario,
    },
    validate_full_correction_trace,
};

pub const NOISY_SEEDS: usize = 256;
pub const MATCHED_PAIRS_PER_LOOK: usize = 2048;
pub const OUTCOME_NOISE_STANDARD_DEVIATION: f64 = 2.0;
pub const MAX_PREDECLARED_LOOKS: usize = 4;
/// Conservative one-sided critical value. At each of four planned looks,
/// z=2.5 is more conservative than Bonferroni alpha=0.05/4 under a normal
/// approximation. The sample has 1024 matched pairs per look.
pub const FAMILYWISE_CRITICAL_VALUE: f64 = 2.8;
pub const NOISY_PROBE_COST: f64 = 0.25;
pub const NOISY_BASE_SEED: u64 = 0x4349_5644_594e_3236;

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
    spare_normal: Option<f64>,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self {
            state: seed,
            spare_normal: None,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn uniform_open(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        (mantissa as f64 + 0.5) / 9_007_199_254_740_992.0
    }

    fn standard_normal(&mut self) -> f64 {
        if let Some(spare) = self.spare_normal.take() {
            return spare;
        }
        let radius = (-2.0 * self.uniform_open().ln()).sqrt();
        let angle = core::f64::consts::TAU * self.uniform_open();
        let first = radius * angle.cos();
        let second = radius * angle.sin();
        self.spare_normal = Some(second);
        first
    }

    fn normal(&mut self, standard_deviation: f64) -> f64 {
        self.standard_normal() * standard_deviation
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MatchedProbeEstimate {
    pub mean_difference: f64,
    pub standard_error: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub sample_pairs: usize,
}

/// Generate a deterministic prefix of matched treatment/control differences.
///
/// Every look is keyed by (regime, episode seed, tick), so fixed-look and
/// anytime-valid methods can consume identical nested prefixes. Each pair
/// shares the same confounder value; the treatment and control noises are
/// independent Gaussian draws with the declared standard deviation.
pub fn paired_difference_stream(
    regime: CausalRegime,
    tick: usize,
    seed_index: usize,
    pair_count: usize,
) -> Option<Vec<f64>> {
    if pair_count < 2 {
        return None;
    }
    const CONFOUNDER_VALUES: [f64; 8] = [-3.0, -2.0, -1.0, 0.0, 0.0, 1.0, 2.0, 3.0];
    const LOOK_SEED_MIX: u64 = 0xd6e8_feb8_6659_fd93;
    let look_seed = scenario_seed(regime, seed_index)
        ^ (tick as u64).wrapping_mul(LOOK_SEED_MIX);
    let mut rng = SplitMix64::new(look_seed);
    let mut differences = Vec::with_capacity(pair_count);

    for pair_index in 0..pair_count {
        let confounder = CONFOUNDER_VALUES[pair_index % CONFOUNDER_VALUES.len()];
        let treatment_noise = rng.normal(OUTCOME_NOISE_STANDARD_DEVIATION);
        let control_noise = rng.normal(OUTCOME_NOISE_STANDARD_DEVIATION);
        let treatment_outcome = 10.0 + confounder + regime.causal_effect(tick) + treatment_noise;
        let control_outcome = 10.0 + confounder + control_noise;
        differences.push(treatment_outcome - control_outcome);
    }
    Some(differences)
}

fn summarize_matched_differences(differences: &[f64]) -> MatchedProbeEstimate {
    let n = differences.len() as f64;
    let mean = differences.iter().sum::<f64>() / n;
    let sum_squares = differences.iter().map(|value| value * value).sum::<f64>();
    let sample_variance = ((sum_squares - n * mean * mean) / (n - 1.0)).max(0.0);
    let standard_error = (sample_variance / n).sqrt();
    let margin = FAMILYWISE_CRITICAL_VALUE * standard_error;

    MatchedProbeEstimate {
        mean_difference: mean,
        standard_error,
        lower_bound: mean - margin,
        upper_bound: mean + margin,
        sample_pairs: differences.len(),
    }
}

fn matched_probe(
    regime: CausalRegime,
    tick: usize,
    seed_index: usize,
) -> MatchedProbeEstimate {
    let differences = paired_difference_stream(
        regime,
        tick,
        seed_index,
        MATCHED_PAIRS_PER_LOOK,
    )
    .expect("the frozen matched-pair sample size is greater than one");
    summarize_matched_differences(&differences)
}

#[derive(Debug, Clone, Copy, Default)]
struct NoisyCusum {
    baseline: Option<f64>,
    positive: f64,
    negative: f64,
    detection_tick: Option<usize>,
}

impl NoisyCusum {
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

#[derive(Debug, Clone, PartialEq)]
pub struct NoisyCausalDriftOutcome {
    pub seed: u64,
    pub regime: CausalRegime,
    pub final_true_causal_effect: f64,
    pub final_observational_association: f64,
    pub signal_detection_tick: Option<usize>,
    pub diagnosis: Option<CausalDiagnosis>,
    pub diagnosis_tick: Option<usize>,
    pub correction_latency_ticks: Option<usize>,
    pub active_ticks: usize,
    pub active_after_nonpositive_effect_ticks: usize,
    pub missed_beneficial_action_ticks: usize,
    pub probe_looks: usize,
    pub total_probe_cost: f64,
    pub final_probe: Option<MatchedProbeEstimate>,
    pub policy_stopped: bool,
    pub trace: Vec<CorrectionEvent>,
    pub trace_valid: bool,
    /// Deterministic no-noise comparator under the same regime.
    pub deterministic_diagnosis_tick: Option<usize>,
    pub deterministic_active_ticks: usize,
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
    diagnosis: CausalDiagnosis,
) -> Vec<CorrectionEvent> {
    let observed_tick = signal_tick.unwrap_or(diagnosis_tick);
    let (diagnosis_code, update_code, action_code) = match diagnosis {
        CausalDiagnosis::CausalSignReversal => (
            "sampled_matched_probe_confirms_negative_effect",
            "replace_positive_effect_belief_with_negative_effect",
            "suspend_nonbeneficial_intervention",
        ),
        CausalDiagnosis::NullEffectWithConfounding => (
            "sampled_matched_probe_bounds_effect_below_action_threshold",
            "remove_unsubstantiated_benefit_belief",
            "suspend_nonbeneficial_intervention",
        ),
        CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange => (
            "sampled_matched_probe_confirms_effect_stable_despite_signal_shift",
            "update_observational_model_for_confounder_shift",
            "retain_beneficial_intervention_and_recalibrate_signal",
        ),
    };
    vec![
        event(
            CorrectionStage::Observed,
            observed_tick,
            "observational_signal",
        ),
        event(
            CorrectionStage::Detected,
            observed_tick,
            "predeclared_signal_or_probe_trigger",
        ),
        event(CorrectionStage::Diagnosed, diagnosis_tick, diagnosis_code),
        event(
            CorrectionStage::Discriminated,
            diagnosis_tick,
            "matched_randomized_probe_with_sampling_uncertainty",
        ),
        event(CorrectionStage::BeliefUpdated, diagnosis_tick, update_code),
        event(CorrectionStage::Intervened, diagnosis_tick + 1, action_code),
        event(
            CorrectionStage::Audited,
            CAUSAL_DRIFT_TICKS,
            "sample_receipt_and_action_ledger_reconciled",
        ),
    ]
}

fn scenario_seed(regime: CausalRegime, seed_index: usize) -> u64 {
    let regime_index = CausalRegime::ALL
        .iter()
        .position(|candidate| *candidate == regime)
        .expect("regime must be in the frozen portfolio") as u64;
    NOISY_BASE_SEED
        ^ (seed_index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ regime_index.wrapping_mul(0xbf58_476d_1ce4_e5b9)
}

pub fn run_noisy_causal_drift(regime: CausalRegime, seed_index: usize) -> NoisyCausalDriftOutcome {
    let seed = scenario_seed(regime, seed_index);
    let mut cusum = NoisyCusum::default();
    let mut policy_enabled = true;
    let mut active_ticks = 0;
    let mut active_after_nonpositive_effect_ticks = 0;
    let mut missed_beneficial_action_ticks = 0;
    let mut probe_looks = 0;
    let mut previous_probe_lower = None;
    let mut final_probe = None;
    let mut diagnosis = None;
    let mut diagnosis_tick = None;
    let mut trace = Vec::new();
    let mut final_true_causal_effect = 0.0;
    let mut final_observational_association = 0.0;

    for tick in 1..=CAUSAL_DRIFT_TICKS {
        let true_effect = regime.causal_effect(tick);
        let confounder_imbalance = regime.observational_confounder_imbalance(tick);
        let observed_association = true_effect + confounder_imbalance;
        cusum.observe(observed_association, tick);
        final_true_causal_effect = true_effect;
        final_observational_association = observed_association;

        if policy_enabled {
            active_ticks += 1;
            if true_effect <= 0.0 {
                active_after_nonpositive_effect_ticks += 1;
            }
        } else if true_effect > ACTION_COST {
            missed_beneficial_action_ticks += 1;
        }

        if policy_enabled
            && diagnosis.is_none()
            && tick % (CAUSAL_DRIFT_TICKS / MAX_PREDECLARED_LOOKS) == 0
        {
            probe_looks += 1;
            let estimate = matched_probe(regime, tick, seed_index);
            final_probe = Some(estimate);

            let identified = if estimate.upper_bound <= 0.0 {
                Some(CausalDiagnosis::CausalSignReversal)
            } else if estimate.upper_bound <= ACTION_COST {
                Some(CausalDiagnosis::NullEffectWithConfounding)
            } else if cusum.detection_tick.is_some()
                && previous_probe_lower.is_some_and(|previous| previous > ACTION_COST)
                && estimate.lower_bound > ACTION_COST
            {
                Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
            } else {
                None
            };

            if let Some(identified) = identified {
                diagnosis = Some(identified);
                diagnosis_tick = Some(tick);
                trace = correction_trace(cusum.detection_tick, tick, identified);
                if matches!(
                    identified,
                    CausalDiagnosis::CausalSignReversal
                        | CausalDiagnosis::NullEffectWithConfounding
                ) {
                    // The current tick's action has already occurred. The
                    // confidence-qualified policy change starts next tick.
                    policy_enabled = false;
                }
            }
            previous_probe_lower = Some(estimate.lower_bound);
        }
    }

    let deterministic = run_causal_drift_scenario(regime, CausalDriftController::MechanismAware);
    let trace_valid = !trace.is_empty() && validate_full_correction_trace(&trace).is_ok();
    let signal_detection_tick = cusum.detection_tick;
    let correction_latency_ticks = diagnosis_tick.map(|diagnosed| {
        signal_detection_tick
            .map(|detected| {
                if diagnosis == Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange) {
                    diagnosed - detected
                } else {
                    diagnosed + 1 - detected
                }
            })
            .unwrap_or(1)
    });

    NoisyCausalDriftOutcome {
        seed,
        regime,
        final_true_causal_effect,
        final_observational_association,
        signal_detection_tick,
        diagnosis,
        diagnosis_tick,
        correction_latency_ticks,
        active_ticks,
        active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks,
        probe_looks,
        total_probe_cost: probe_looks as f64 * NOISY_PROBE_COST,
        final_probe,
        policy_stopped: !policy_enabled,
        trace,
        trace_valid,
        deterministic_diagnosis_tick: deterministic.diagnosis_tick,
        deterministic_active_ticks: deterministic.active_ticks,
    }
}

pub fn run_noisy_causal_drift_portfolio() -> Vec<NoisyCausalDriftOutcome> {
    let mut outcomes = Vec::with_capacity(CausalRegime::ALL.len() * NOISY_SEEDS);
    for regime in CausalRegime::ALL {
        for seed_index in 0..NOISY_SEEDS {
            outcomes.push(run_noisy_causal_drift(regime, seed_index));
        }
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noisy_probe_is_seed_replayable() {
        let first = run_noisy_causal_drift(CausalRegime::SignReversal, 17);
        let second = run_noisy_causal_drift(CausalRegime::SignReversal, 17);
        assert_eq!(first, second);
    }

    #[test]
    fn stable_positive_effect_is_not_stopped_by_noisy_sequential_probes() {
        for seed in 0..NOISY_SEEDS {
            let outcome = run_noisy_causal_drift(CausalRegime::StablePositive, seed);
            assert!(!outcome.policy_stopped);
            assert!(outcome.diagnosis.is_none());
            assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
            assert_eq!(outcome.probe_looks, MAX_PREDECLARED_LOOKS);
        }
    }

    #[test]
    fn confounder_shift_preserves_the_beneficial_policy_after_noisy_probe() {
        for seed in 0..NOISY_SEEDS {
            let outcome = run_noisy_causal_drift(CausalRegime::ConfounderShift, seed);
            assert_eq!(
                outcome.diagnosis,
                Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
            );
            assert!(!outcome.policy_stopped);
            assert_eq!(outcome.missed_beneficial_action_ticks, 0);
            assert!(outcome.trace_valid);
        }
    }

    #[test]
    fn noisy_sign_reversal_and_null_effects_are_detected_with_frozen_seed_suite() {
        for seed in 0..NOISY_SEEDS {
            let reversal = run_noisy_causal_drift(CausalRegime::SignReversal, seed);
            assert_eq!(
                reversal.diagnosis,
                Some(CausalDiagnosis::CausalSignReversal)
            );
            assert!(reversal.policy_stopped);
            assert_eq!(reversal.active_after_nonpositive_effect_ticks, 1);

            let gradual = run_noisy_causal_drift(CausalRegime::GradualSignReversal, seed);
            assert_eq!(gradual.diagnosis, Some(CausalDiagnosis::CausalSignReversal));
            assert!(gradual.policy_stopped);

            let null = run_noisy_causal_drift(CausalRegime::ConfoundedNull, seed);
            assert!(matches!(
                null.diagnosis,
                Some(
                    CausalDiagnosis::NullEffectWithConfounding
                        | CausalDiagnosis::CausalSignReversal
                )
            ));
            assert!(null.policy_stopped);
        }
    }

    #[test]
    fn null_effect_false_sign_reversal_remains_within_preregistered_error_budget() {
        let false_sign_reversals = (0..NOISY_SEEDS)
            .filter(|seed| {
                run_noisy_causal_drift(CausalRegime::ConfoundedNull, *seed).diagnosis
                    == Some(CausalDiagnosis::CausalSignReversal)
            })
            .count();
        assert!(
            false_sign_reversals <= 5,
            "false causal sign diagnoses: {false_sign_reversals} / {NOISY_SEEDS}"
        );
    }

    #[test]
    fn every_noisy_correction_has_a_valid_complete_evidence_trace() {
        for outcome in run_noisy_causal_drift_portfolio() {
            if outcome.diagnosis.is_some() {
                assert!(outcome.trace_valid);
                assert_eq!(outcome.trace.len(), 7);
            }
        }
    }

    #[test]
    fn sample_size_and_repeated_look_contract_are_explicit() {
        assert_eq!(MATCHED_PAIRS_PER_LOOK, 2048);
        assert_eq!(MAX_PREDECLARED_LOOKS, 4);
        assert_eq!(FAMILYWISE_CRITICAL_VALUE, 2.8);
        assert_eq!(NOISY_SEEDS, 256);
    }
}
