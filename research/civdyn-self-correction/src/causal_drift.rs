// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic causal-drift controls.
//!
//! The portfolio separates statistical signal-change detection from causal
//! mechanism diagnosis. Deterministic paired outcomes are used as known-answer
//! controls; they are not empirical causal estimates.

use crate::{CorrectionEvent, CorrectionStage, validate_full_correction_trace};

pub const CAUSAL_DRIFT_TICKS: usize = 8;
pub const CAUSAL_SIGN_REVERSAL_TICK: usize = 4;
pub const MATCHED_PROBE_INTERVAL_TICKS: usize = 2;
pub const CONFOUNDER_ASSOCIATION_BIAS: f64 = 5.0;
pub const CONFOUNDER_SHIFT_TICK: usize = 5;
pub const CONFOUNDER_SHIFT_IMBALANCE: f64 = -5.0;
pub const ACTION_COST: f64 = 0.5;
pub const DIAGNOSTIC_PROBE_COST: f64 = 0.25;
pub const CUSUM_ALLOWANCE: f64 = 0.25;
pub const CUSUM_THRESHOLD: f64 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalRegime {
    StablePositive,
    SignReversal,
    GradualSignReversal,
    ConfoundedNull,
    ConfounderShift,
}

impl CausalRegime {
    pub const ALL: [Self; 5] = [
        Self::StablePositive,
        Self::SignReversal,
        Self::GradualSignReversal,
        Self::ConfoundedNull,
        Self::ConfounderShift,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::StablePositive => "stable_positive",
            Self::SignReversal => "sign_reversal",
            Self::GradualSignReversal => "gradual_sign_reversal",
            Self::ConfoundedNull => "confounded_null",
            Self::ConfounderShift => "confounder_shift",
        }
    }

    pub const fn causal_effect(self, one_based_tick: usize) -> f64 {
        match self {
            Self::StablePositive | Self::ConfounderShift => 2.0,
            Self::SignReversal if one_based_tick < CAUSAL_SIGN_REVERSAL_TICK => 2.0,
            Self::SignReversal => -3.0,
            Self::GradualSignReversal if one_based_tick <= 2 => 2.0,
            Self::GradualSignReversal if one_based_tick <= 4 => 1.0,
            Self::GradualSignReversal if one_based_tick <= 6 => -1.0,
            Self::GradualSignReversal => -2.0,
            Self::ConfoundedNull => 0.0,
        }
    }

    const fn observational_confounder_imbalance(self, one_based_tick: usize) -> f64 {
        match self {
            Self::ConfounderShift if one_based_tick >= CONFOUNDER_SHIFT_TICK => {
                CONFOUNDER_SHIFT_IMBALANCE
            }
            _ => CONFOUNDER_ASSOCIATION_BIAS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalDriftController {
    FrozenPolicy,
    OutcomeReactive,
    CusumSignalOnly,
    MechanismAware,
}

impl CausalDriftController {
    pub const ALL: [Self; 4] = [
        Self::FrozenPolicy,
        Self::OutcomeReactive,
        Self::CusumSignalOnly,
        Self::MechanismAware,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FrozenPolicy => "frozen_policy",
            Self::OutcomeReactive => "outcome_reactive",
            Self::CusumSignalOnly => "cusum_signal_only",
            Self::MechanismAware => "mechanism_aware",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalDiagnosis {
    CausalSignReversal,
    NullEffectWithConfounding,
    ConfounderShiftWithoutCausalEffectChange,
}

impl CausalDiagnosis {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CausalSignReversal => "causal_sign_reversal",
            Self::NullEffectWithConfounding => "null_effect_with_confounding",
            Self::ConfounderShiftWithoutCausalEffectChange => {
                "confounder_shift_without_causal_effect_change"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalDriftOutcome {
    pub regime: CausalRegime,
    pub controller: CausalDriftController,
    pub final_true_causal_effect: f64,
    pub final_observational_association: f64,
    pub last_matched_probe_estimate: Option<f64>,
    pub active_ticks: usize,
    pub active_after_nonpositive_effect_ticks: usize,
    pub missed_beneficial_action_ticks: usize,
    pub cumulative_causal_contribution: f64,
    pub action_cost: f64,
    pub diagnostic_probe_count: usize,
    pub diagnostic_probe_cost: f64,
    pub net_utility: f64,
    pub oracle_net_utility: f64,
    pub regret_to_oracle: f64,
    /// True only when the outcome-reactive controller continues acting because
    /// the observational proxy remains above its action threshold.
    pub proxy_driven_action_under_nonpositive_effect: bool,
    pub signal_change_detected: bool,
    pub signal_detection_tick: Option<usize>,
    pub mechanism_identified: bool,
    pub diagnosis: Option<CausalDiagnosis>,
    pub diagnosis_tick: Option<usize>,
    pub correction_latency_ticks: Option<usize>,
    pub trace: Vec<CorrectionEvent>,
    pub trace_valid: bool,
}

fn event(
    stage: CorrectionStage,
    tick: usize,
    evidence_code: &'static str,
) -> CorrectionEvent {
    CorrectionEvent {
        stage,
        tick,
        evidence_code,
    }
}

fn potential_outcome(
    regime: CausalRegime,
    one_based_tick: usize,
    treatment: bool,
    confounder: f64,
) -> f64 {
    let treatment_value = if treatment { 1.0 } else { 0.0 };
    10.0 + confounder + regime.causal_effect(one_based_tick) * treatment_value
}

fn observational_association(regime: CausalRegime, one_based_tick: usize) -> f64 {
    let imbalance = regime.observational_confounder_imbalance(one_based_tick);
    potential_outcome(regime, one_based_tick, true, imbalance)
        - potential_outcome(regime, one_based_tick, false, 0.0)
}

fn matched_randomized_probe_estimate(regime: CausalRegime, one_based_tick: usize) -> f64 {
    // Treatment/control is balanced within each confounder-matched pair. The
    // estimate is calculated from paired outcomes, never read directly from
    // the hidden regime coefficient. There is no sampling noise in this
    // deterministic known-answer control.
    const MATCHED_CONFOUNDER_VALUES: [f64; 5] = [-2.0, -1.0, 0.0, 1.0, 2.0];
    let paired_differences = MATCHED_CONFOUNDER_VALUES.map(|confounder| {
        potential_outcome(regime, one_based_tick, true, confounder)
            - potential_outcome(regime, one_based_tick, false, confounder)
    });
    paired_differences.iter().sum::<f64>() / paired_differences.len() as f64
}

#[derive(Debug, Clone, Copy, Default)]
struct CusumState {
    baseline: Option<f64>,
    positive: f64,
    negative: f64,
    alarmed: bool,
    detection_tick: Option<usize>,
}

impl CusumState {
    fn observe(&mut self, value: f64, tick: usize) {
        let Some(baseline) = self.baseline else {
            self.baseline = Some(value);
            return;
        };
        if self.alarmed {
            return;
        }
        let deviation = value - baseline;
        self.positive = (self.positive + deviation - CUSUM_ALLOWANCE).max(0.0);
        self.negative = (self.negative - deviation - CUSUM_ALLOWANCE).max(0.0);
        if self.positive >= CUSUM_THRESHOLD || self.negative >= CUSUM_THRESHOLD {
            self.alarmed = true;
            self.detection_tick = Some(tick);
        }
    }
}

fn oracle_utility(regime: CausalRegime) -> f64 {
    (1..=CAUSAL_DRIFT_TICKS)
        .map(|tick| {
            let effect = regime.causal_effect(tick);
            if effect > ACTION_COST {
                effect - ACTION_COST
            } else {
                0.0
            }
        })
        .sum()
}

fn correction_trace(
    signal_tick: Option<usize>,
    diagnosis_tick: usize,
    diagnosis: CausalDiagnosis,
) -> Vec<CorrectionEvent> {
    let observed_tick = signal_tick.unwrap_or(diagnosis_tick);
    let (diagnosis_code, update_code, action_code) = match diagnosis {
        CausalDiagnosis::CausalSignReversal => (
            "matched_probe_confirms_sign_reversal",
            "replace_positive_effect_belief_with_negative_effect",
            "suspend_nonbeneficial_intervention",
        ),
        CausalDiagnosis::NullEffectWithConfounding => (
            "matched_probe_finds_zero_effect_despite_positive_association",
            "remove_spurious_benefit_belief",
            "suspend_nonbeneficial_intervention",
        ),
        CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange => (
            "matched_probe_confirms_causal_effect_is_stable",
            "update_observational_model_for_confounder_shift",
            "retain_intervention_and_recalibrate_signal",
        ),
    };
    vec![
        event(
            CorrectionStage::Observed,
            observed_tick,
            "observational_association_changed_or_conflicted_with_outcome",
        ),
        event(
            CorrectionStage::Detected,
            observed_tick,
            "cusum_signal_or_paired_probe_triggered_review",
        ),
        event(CorrectionStage::Diagnosed, diagnosis_tick, diagnosis_code),
        event(
            CorrectionStage::Discriminated,
            diagnosis_tick,
            "matched_randomized_treatment_control_probe",
        ),
        event(
            CorrectionStage::BeliefUpdated,
            diagnosis_tick,
            update_code,
        ),
        event(
            CorrectionStage::Intervened,
            diagnosis_tick + 1,
            action_code,
        ),
        event(
            CorrectionStage::Audited,
            CAUSAL_DRIFT_TICKS,
            "held_out_outcome_and_action_costs_reconciled",
        ),
    ]
}

pub fn run_causal_drift_scenario(
    regime: CausalRegime,
    controller: CausalDriftController,
) -> CausalDriftOutcome {
    let mut policy_enabled = true;
    let mut active_ticks = 0usize;
    let mut active_after_nonpositive_effect_ticks = 0usize;
    let mut missed_beneficial_action_ticks = 0usize;
    let mut cumulative_causal_contribution = 0.0;
    let mut action_cost = 0.0;
    let mut diagnostic_probe_count = 0usize;
    let mut last_matched_probe_estimate = None;
    let mut previous_matched_probe_estimate = None;
    let mut diagnostic_probe_cost = 0.0;
    let mut diagnosis = None;
    let mut diagnosis_tick = None;
    let mut trace = Vec::new();
    let mut proxy_driven_action_under_nonpositive_effect = false;
    let mut signal_detector = CusumState::default();
    let mut final_true_causal_effect = 0.0;
    let mut final_observational_association = 0.0;

    for tick in 1..=CAUSAL_DRIFT_TICKS {
        let true_effect = regime.causal_effect(tick);
        let observed_association = observational_association(regime, tick);
        signal_detector.observe(observed_association, tick);
        final_true_causal_effect = true_effect;
        final_observational_association = observed_association;

        if policy_enabled {
            active_ticks += 1;
            cumulative_causal_contribution += true_effect;
            action_cost += ACTION_COST;
            if true_effect <= 0.0 {
                active_after_nonpositive_effect_ticks += 1;
                if controller == CausalDriftController::OutcomeReactive
                    && observed_association > ACTION_COST
                {
                    proxy_driven_action_under_nonpositive_effect = true;
                }
            }
        } else if true_effect > ACTION_COST {
            missed_beneficial_action_ticks += 1;
        }

        match controller {
            CausalDriftController::FrozenPolicy => {}
            CausalDriftController::OutcomeReactive => {
                if observed_association <= ACTION_COST {
                    policy_enabled = false;
                }
            }
            CausalDriftController::CusumSignalOnly => {
                if signal_detector.alarmed {
                    // A signal-only policy treats any observational shift as
                    // a policy-relevant mechanism change. It cannot identify
                    // whether the intervention effect itself changed.
                    policy_enabled = false;
                }
            }
            CausalDriftController::MechanismAware => {
                if policy_enabled
                    && diagnosis.is_none()
                    && tick % MATCHED_PROBE_INTERVAL_TICKS == 0
                {
                    diagnostic_probe_count += 1;
                    diagnostic_probe_cost += DIAGNOSTIC_PROBE_COST;
                    let probe_estimate = matched_randomized_probe_estimate(regime, tick);
                    last_matched_probe_estimate = Some(probe_estimate);

                    let identified = if probe_estimate < 0.0 {
                        Some(CausalDiagnosis::CausalSignReversal)
                    } else if probe_estimate <= ACTION_COST {
                        Some(CausalDiagnosis::NullEffectWithConfounding)
                    } else if signal_detector.alarmed
                        && previous_matched_probe_estimate
                            .is_some_and(|previous| previous > ACTION_COST)
                    {
                        Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
                    } else {
                        None
                    };

                    if let Some(identified) = identified {
                        diagnosis = Some(identified);
                        diagnosis_tick = Some(tick);
                        trace = correction_trace(
                            signal_detector.detection_tick,
                            tick,
                            identified,
                        );
                        if matches!(
                            identified,
                            CausalDiagnosis::CausalSignReversal
                                | CausalDiagnosis::NullEffectWithConfounding
                        ) {
                            // The current tick's action has already happened;
                            // the corrected action takes effect next tick.
                            policy_enabled = false;
                        }
                    }
                    previous_matched_probe_estimate = Some(probe_estimate);
                }
            }
        }
    }

    let oracle_net_utility = oracle_utility(regime);
    let net_utility =
        cumulative_causal_contribution - action_cost - diagnostic_probe_cost;
    let trace_valid = !trace.is_empty() && validate_full_correction_trace(&trace).is_ok();
    let signal_detection_tick = signal_detector.detection_tick;
    let correction_latency_ticks = diagnosis_tick.map(|diagnosed| {
        signal_detection_tick
            .map(|detected| diagnosed + 1 - detected)
            .unwrap_or(1)
    });

    CausalDriftOutcome {
        regime,
        controller,
        final_true_causal_effect,
        final_observational_association,
        last_matched_probe_estimate,
        active_ticks,
        active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks,
        cumulative_causal_contribution,
        action_cost,
        diagnostic_probe_count,
        diagnostic_probe_cost,
        net_utility,
        oracle_net_utility,
        regret_to_oracle: oracle_net_utility - net_utility,
        proxy_driven_action_under_nonpositive_effect,
        signal_change_detected: signal_detector.alarmed,
        signal_detection_tick,
        mechanism_identified: diagnosis.is_some(),
        diagnosis,
        diagnosis_tick,
        correction_latency_ticks,
        trace,
        trace_valid,
    }
}

pub fn run_causal_drift_portfolio() -> Vec<CausalDriftOutcome> {
    let mut outcomes =
        Vec::with_capacity(CausalRegime::ALL.len() * CausalDriftController::ALL.len());
    for regime in CausalRegime::ALL {
        for controller in CausalDriftController::ALL {
            outcomes.push(run_causal_drift_scenario(regime, controller));
        }
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(regime: CausalRegime, controller: CausalDriftController) -> CausalDriftOutcome {
        run_causal_drift_scenario(regime, controller)
    }

    #[test]
    fn abrupt_causal_reversal_is_hidden_by_a_positive_association() {
        let outcome = run(
            CausalRegime::SignReversal,
            CausalDriftController::OutcomeReactive,
        );
        assert_eq!(outcome.final_true_causal_effect, -3.0);
        assert_eq!(outcome.final_observational_association, 2.0);
        assert!(outcome.proxy_driven_action_under_nonpositive_effect);
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 5);
        assert!(!outcome.mechanism_identified);
    }

    #[test]
    fn cusum_detects_observational_shift_but_does_not_claim_a_cause() {
        let outcome = run(
            CausalRegime::SignReversal,
            CausalDriftController::CusumSignalOnly,
        );
        assert!(outcome.signal_change_detected);
        assert_eq!(outcome.signal_detection_tick, Some(4));
        assert!(!outcome.mechanism_identified);
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 1);
    }

    #[test]
    fn matched_probe_identifies_abrupt_sign_reversal() {
        let outcome = run(
            CausalRegime::SignReversal,
            CausalDriftController::MechanismAware,
        );
        assert!(outcome.mechanism_identified);
        assert_eq!(outcome.diagnosis, Some(CausalDiagnosis::CausalSignReversal));
        assert_eq!(outcome.signal_detection_tick, Some(4));
        assert_eq!(outcome.diagnosis_tick, Some(4));
        assert_eq!(outcome.correction_latency_ticks, Some(1));
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 1);
        assert_eq!(outcome.active_ticks, 4);
        assert!(outcome.trace_valid);
        assert_eq!(outcome.trace.len(), 7);
    }

    #[test]
    fn gradual_sign_reversal_is_detected_with_a_diagnosis_delay() {
        let outcome = run(
            CausalRegime::GradualSignReversal,
            CausalDriftController::MechanismAware,
        );
        assert_eq!(outcome.final_true_causal_effect, -2.0);
        assert_eq!(outcome.final_observational_association, 3.0);
        assert_eq!(outcome.signal_detection_tick, Some(5));
        assert_eq!(outcome.diagnosis, Some(CausalDiagnosis::CausalSignReversal));
        assert_eq!(outcome.diagnosis_tick, Some(6));
        assert_eq!(outcome.correction_latency_ticks, Some(2));
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 2);
        assert_eq!(outcome.active_ticks, 6);
        assert!(outcome.trace_valid);
    }

    #[test]
    fn stable_positive_control_does_not_trigger_a_cusum_alarm_or_shutdown() {
        let outcome = run(
            CausalRegime::StablePositive,
            CausalDriftController::MechanismAware,
        );
        assert!(!outcome.signal_change_detected);
        assert!(!outcome.mechanism_identified);
        assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
        assert_eq!(outcome.last_matched_probe_estimate, Some(2.0));
        assert!(outcome.trace.is_empty());
        assert!(!outcome.trace_valid);
    }

    #[test]
    fn matched_probe_rejects_a_confounded_null_effect() {
        let outcome = run(
            CausalRegime::ConfoundedNull,
            CausalDriftController::MechanismAware,
        );
        assert_eq!(
            outcome.diagnosis,
            Some(CausalDiagnosis::NullEffectWithConfounding)
        );
        assert_eq!(outcome.last_matched_probe_estimate, Some(0.0));
        assert_eq!(outcome.diagnosis_tick, Some(2));
        assert_eq!(outcome.active_ticks, 2);
        assert!(outcome.trace_valid);
    }

    #[test]
    fn cusum_shift_detector_cannot_distinguish_a_confounder_shift() {
        let outcome = run(
            CausalRegime::ConfounderShift,
            CausalDriftController::CusumSignalOnly,
        );
        assert_eq!(outcome.final_true_causal_effect, 2.0);
        assert_eq!(outcome.final_observational_association, -3.0);
        assert_eq!(outcome.signal_detection_tick, Some(5));
        assert!(!outcome.mechanism_identified);
        assert_eq!(outcome.missed_beneficial_action_ticks, 3);
    }

    #[test]
    fn matched_probe_identifies_confounder_shift_and_preserves_beneficial_action() {
        let outcome = run(
            CausalRegime::ConfounderShift,
            CausalDriftController::MechanismAware,
        );
        assert_eq!(
            outcome.diagnosis,
            Some(CausalDiagnosis::ConfounderShiftWithoutCausalEffectChange)
        );
        assert_eq!(outcome.signal_detection_tick, Some(5));
        assert_eq!(outcome.diagnosis_tick, Some(6));
        assert_eq!(outcome.last_matched_probe_estimate, Some(2.0));
        assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
        assert_eq!(outcome.missed_beneficial_action_ticks, 0);
        assert!(outcome.trace_valid);
    }

    #[test]
    fn outcome_reactive_policy_curtails_a_still_beneficial_action_after_confounder_shift() {
        let outcome = run(
            CausalRegime::ConfounderShift,
            CausalDriftController::OutcomeReactive,
        );
        assert_eq!(outcome.missed_beneficial_action_ticks, 3);
        assert_eq!(outcome.active_ticks, 5);
        assert!(!outcome.mechanism_identified);
    }

    #[test]
    fn portfolio_is_complete_and_deterministic() {
        let first = run_causal_drift_portfolio();
        let second = run_causal_drift_portfolio();
        assert_eq!(first, second);
        assert_eq!(
            first.len(),
            CausalRegime::ALL.len() * CausalDriftController::ALL.len()
        );
    }
}
