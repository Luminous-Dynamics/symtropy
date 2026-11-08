// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic causal-sign-reversal reference with an intentionally misleading
//! observational association. This module is a diagnostic control, not an
//! empirical causal-inference method.

use crate::{
    ControllerKind, CorrectionEvent, CorrectionStage, validate_full_correction_trace,
};

pub const CAUSAL_DRIFT_TICKS: usize = 8;
pub const CAUSAL_SIGN_REVERSAL_TICK: usize = 4;
pub const MATCHED_PROBE_INTERVAL_TICKS: usize = 2;
pub const CONFOUNDER_ASSOCIATION_BIAS: f64 = 5.0;
pub const ACTION_COST: f64 = 0.5;
pub const DIAGNOSTIC_PROBE_COST: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalRegime {
    StablePositive,
    SignReversal,
    GradualSignReversal,
    ConfoundedNull,
}

impl CausalRegime {
    pub const ALL: [Self; 3] = [
        Self::StablePositive,
        Self::SignReversal,
        Self::GradualSignReversal,
        Self::ConfoundedNull,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::StablePositive => "stable_positive",
            Self::SignReversal => "sign_reversal",
            Self::GradualSignReversal => "gradual_sign_reversal",
            Self::ConfoundedNull => "confounded_null",
        }
    }

    pub const fn causal_effect(self, one_based_tick: usize) -> f64 {
        match self {
            Self::StablePositive => 2.0,
            Self::SignReversal if one_based_tick < CAUSAL_SIGN_REVERSAL_TICK => 2.0,
            Self::SignReversal => -3.0,
            Self::GradualSignReversal if one_based_tick <= 2 => 2.0,
            Self::GradualSignReversal if one_based_tick <= 4 => 1.0,
            Self::GradualSignReversal if one_based_tick <= 6 => -1.0,
            Self::GradualSignReversal => -2.0,
            Self::ConfoundedNull => 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalDiagnosis {
    CausalSignReversal,
    NullEffectWithConfounding,
}

impl CausalDiagnosis {
    pub const fn name(self) -> &'static str {
        match self {
            Self::CausalSignReversal => "causal_sign_reversal",
            Self::NullEffectWithConfounding => "null_effect_with_confounding",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalDriftOutcome {
    pub regime: CausalRegime,
    pub controller: ControllerKind,
    pub final_true_causal_effect: f64,
    pub final_observational_association: f64,
    pub last_matched_probe_estimate: Option<f64>,
    pub active_ticks: usize,
    pub active_after_nonpositive_effect_ticks: usize,
    pub cumulative_causal_contribution: f64,
    pub action_cost: f64,
    pub diagnostic_probe_count: usize,
    pub diagnostic_probe_cost: f64,
    pub net_utility: f64,
    pub oracle_net_utility: f64,
    pub regret_to_oracle: f64,
    pub proxy_driven_action_under_nonpositive_effect: bool,
    pub mechanism_identified: bool,
    pub diagnosis: Option<CausalDiagnosis>,
    pub detection_tick: Option<usize>,
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

/// The observational estimate includes a five-unit confounder imbalance.
/// A matched randomized probe holds that common cause fixed and recovers the
/// declared treatment effect exactly. This makes the control deterministic
/// and known-answer; it does not model sampling noise or prove real-world
/// identification assumptions.
fn potential_outcome(
    regime: CausalRegime,
    one_based_tick: usize,
    treatment: bool,
    confounder: f64,
) -> f64 {
    // A deterministic structural equation with an explicit common cause.
    // The controller only receives observed outcomes and the probe estimate;
    // it does not receive the regime label or this function's causal effect.
    let treatment_value = if treatment { 1.0 } else { 0.0 };
    10.0 + confounder + regime.causal_effect(one_based_tick) * treatment_value
}

fn observational_association(regime: CausalRegime, one_based_tick: usize) -> f64 {
    // The treated observational group has a +5 confounder imbalance.
    // This leaves its association positive after the causal effect reverses.
    potential_outcome(regime, one_based_tick, true, CONFOUNDER_ASSOCIATION_BIAS)
        - potential_outcome(regime, one_based_tick, false, 0.0)
}

fn matched_randomized_probe_estimate(
    regime: CausalRegime,
    one_based_tick: usize,
) -> f64 {
    // Treatment is balanced within each confounder-matched pair. The estimator
    // is computed from synthetic observed outcomes, not by reading the hidden
    // regime coefficient directly. There is deliberately no sampling noise
    // in this first known-answer control.
    const MATCHED_CONFOUNDER_VALUES: [f64; 5] = [-2.0, -1.0, 0.0, 1.0, 2.0];
    let paired_differences = MATCHED_CONFOUNDER_VALUES.map(|confounder| {
        potential_outcome(regime, one_based_tick, true, confounder)
            - potential_outcome(regime, one_based_tick, false, confounder)
    });
    paired_differences.iter().sum::<f64>() / paired_differences.len() as f64
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
    detection_tick: usize,
    diagnosis: CausalDiagnosis,
) -> Vec<CorrectionEvent> {
    let (diagnosis_code, update_code) = match diagnosis {
        CausalDiagnosis::CausalSignReversal => (
            "matched_probe_confirms_sign_reversal",
            "replace_positive_effect_belief_with_negative_effect",
        ),
        CausalDiagnosis::NullEffectWithConfounding => (
            "matched_probe_finds_zero_effect_despite_positive_association",
            "remove_spurious_benefit_belief",
        ),
    };
    vec![
        event(
            CorrectionStage::Observed,
            detection_tick,
            "observational_association_remains_positive",
        ),
        event(
            CorrectionStage::Detected,
            detection_tick,
            "matched_probe_disagrees_with_observational_association",
        ),
        event(CorrectionStage::Diagnosed, detection_tick, diagnosis_code),
        event(
            CorrectionStage::Discriminated,
            detection_tick,
            "paired_randomized_treatment_control_probe",
        ),
        event(
            CorrectionStage::BeliefUpdated,
            detection_tick,
            update_code,
        ),
        event(
            CorrectionStage::Intervened,
            detection_tick + 1,
            "suspend_nonbeneficial_intervention",
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
    controller: ControllerKind,
) -> CausalDriftOutcome {
    let mut policy_enabled = true;
    let mut active_ticks = 0usize;
    let mut active_after_nonpositive_effect_ticks = 0usize;
    let mut cumulative_causal_contribution = 0.0;
    let mut action_cost = 0.0;
    let mut diagnostic_probe_count = 0usize;
    let mut last_matched_probe_estimate = None;
    let mut diagnostic_probe_cost = 0.0;
    let mut diagnosis = None;
    let mut detection_tick = None;
    let mut trace = Vec::new();
    let mut proxy_driven_action_under_nonpositive_effect = false;
    let mut final_true_causal_effect = 0.0;
    let mut final_observational_association = 0.0;

    for tick in 1..=CAUSAL_DRIFT_TICKS {
        let true_effect = regime.causal_effect(tick);
        let observed_association = observational_association(regime, tick);
        final_true_causal_effect = true_effect;
        final_observational_association = observed_association;

        if policy_enabled {
            active_ticks += 1;
            cumulative_causal_contribution += true_effect;
            action_cost += ACTION_COST;
            if true_effect <= 0.0 {
                active_after_nonpositive_effect_ticks += 1;
                if controller == ControllerKind::OutcomeReactive
                    && observed_association > 0.0
                {
                    proxy_driven_action_under_nonpositive_effect = true;
                }
            }
        }

        match controller {
            ControllerKind::FrozenPolicy => {}
            ControllerKind::OutcomeReactive => {
                // The proxy stays positive because the confounder imbalance
                // is larger than the reversed or null causal effect.
                if observed_association <= ACTION_COST {
                    policy_enabled = false;
                }
            }
            ControllerKind::MechanismAware => {
                if policy_enabled && tick % MATCHED_PROBE_INTERVAL_TICKS == 0 {
                    diagnostic_probe_count += 1;
                    diagnostic_probe_cost += DIAGNOSTIC_PROBE_COST;
                    let probe_estimate = matched_randomized_probe_estimate(regime, tick);
                    last_matched_probe_estimate = Some(probe_estimate);
                    if probe_estimate <= ACTION_COST && diagnosis.is_none() {
                        let identified = if probe_estimate < 0.0 {
                            CausalDiagnosis::CausalSignReversal
                        } else {
                            CausalDiagnosis::NullEffectWithConfounding
                        };
                        diagnosis = Some(identified);
                        detection_tick = Some(tick);
                        trace = correction_trace(tick, identified);
                        // The action already occurred for this tick. The
                        // corrected decision applies from the next tick.
                        policy_enabled = false;
                    }
                }
            }
        }
    }

    let oracle_net_utility = oracle_utility(regime);
    let net_utility =
        cumulative_causal_contribution - action_cost - diagnostic_probe_cost;
    let trace_valid = !trace.is_empty() && validate_full_correction_trace(&trace).is_ok();
    CausalDriftOutcome {
        regime,
        controller,
        final_true_causal_effect,
        final_observational_association,
        last_matched_probe_estimate,
        active_ticks,
        active_after_nonpositive_effect_ticks,
        cumulative_causal_contribution,
        action_cost,
        diagnostic_probe_count,
        diagnostic_probe_cost,
        net_utility,
        oracle_net_utility,
        regret_to_oracle: oracle_net_utility - net_utility,
        proxy_driven_action_under_nonpositive_effect,
        mechanism_identified: diagnosis.is_some(),
        diagnosis,
        detection_tick,
        correction_latency_ticks: detection_tick.map(|_| 1),
        trace,
        trace_valid,
    }
}

pub fn run_causal_drift_portfolio() -> Vec<CausalDriftOutcome> {
    let mut outcomes = Vec::with_capacity(CausalRegime::ALL.len() * ControllerKind::ALL.len());
    for regime in CausalRegime::ALL {
        for controller in ControllerKind::ALL {
            outcomes.push(run_causal_drift_scenario(regime, controller));
        }
    }
    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(regime: CausalRegime, controller: ControllerKind) -> CausalDriftOutcome {
        run_causal_drift_scenario(regime, controller)
    }

    #[test]
    fn confounder_keeps_observational_association_positive_after_sign_reversal() {
        let outcome = run(
            CausalRegime::SignReversal,
            ControllerKind::OutcomeReactive,
        );
        assert_eq!(outcome.final_true_causal_effect, -3.0);
        assert_eq!(outcome.final_observational_association, 2.0);
        assert!(outcome.proxy_driven_action_under_nonpositive_effect);
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 5);
    }

    #[test]
    fn outcome_reactive_controller_fails_to_detect_sign_reversal() {
        let outcome = run(
            CausalRegime::SignReversal,
            ControllerKind::OutcomeReactive,
        );
        assert!(!outcome.mechanism_identified);
        assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
        assert_eq!(outcome.detection_tick, None);
    }

    #[test]
    fn matched_probe_identifies_sign_reversal_and_suspends_next_tick() {
        let outcome = run(
            CausalRegime::SignReversal,
            ControllerKind::MechanismAware,
        );
        assert!(outcome.mechanism_identified);
        assert_eq!(outcome.diagnosis, Some(CausalDiagnosis::CausalSignReversal));
        assert_eq!(outcome.detection_tick, Some(CAUSAL_SIGN_REVERSAL_TICK));
        assert_eq!(outcome.correction_latency_ticks, Some(1));
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 1);
        assert_eq!(outcome.active_ticks, 4);
        assert!(outcome.trace_valid);
        assert_eq!(outcome.trace.len(), 7);
        assert!(
            outcome.net_utility
                > run(
                    CausalRegime::SignReversal,
                    ControllerKind::FrozenPolicy
                )
                .net_utility
        );
    }

    #[test]
    fn gradual_sign_reversal_is_detected_when_paired_probe_crosses_threshold() {
        let outcome = run(
            CausalRegime::GradualSignReversal,
            ControllerKind::MechanismAware,
        );
        assert_eq!(outcome.final_true_causal_effect, -2.0);
        assert_eq!(outcome.final_observational_association, 3.0);
        assert_eq!(
            outcome.diagnosis,
            Some(CausalDiagnosis::CausalSignReversal)
        );
        assert_eq!(outcome.detection_tick, Some(6));
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 2);
        assert_eq!(outcome.active_ticks, 6);
        assert!(outcome.trace_valid);
    }

    #[test]
    fn stable_positive_mechanism_is_a_negative_control_for_overcorrection() {
        let outcome = run(
            CausalRegime::StablePositive,
            ControllerKind::MechanismAware,
        );
        assert!(!outcome.mechanism_identified);
        assert_eq!(outcome.diagnosis, None);
        assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
        assert_eq!(outcome.active_after_nonpositive_effect_ticks, 0);
        assert!(outcome.trace.is_empty());
        assert!(!outcome.trace_valid);
        assert_eq!(outcome.last_matched_probe_estimate, Some(2.0));
    }

    #[test]
    fn matched_probe_rejects_confounded_null_effect_without_blanket_shutdown() {
        let outcome = run(
            CausalRegime::ConfoundedNull,
            ControllerKind::MechanismAware,
        );
        assert_eq!(
            outcome.diagnosis,
            Some(CausalDiagnosis::NullEffectWithConfounding)
        );
        assert!(outcome.mechanism_identified);
        assert_eq!(outcome.final_observational_association, 5.0);
        assert_eq!(outcome.last_matched_probe_estimate, Some(0.0));
        assert_eq!(outcome.detection_tick, Some(2));
        assert_eq!(outcome.active_ticks, 2);
        assert!(outcome.trace_valid);
    }

    #[test]
    fn outcome_reactive_controller_mistakes_confounded_null_association_for_benefit() {
        let outcome = run(
            CausalRegime::ConfoundedNull,
            ControllerKind::OutcomeReactive,
        );
        assert!(outcome.proxy_driven_action_under_nonpositive_effect);
        assert_eq!(outcome.active_ticks, CAUSAL_DRIFT_TICKS);
        assert_eq!(outcome.final_true_causal_effect, 0.0);
        assert_eq!(outcome.final_observational_association, 5.0);
    }

    #[test]
    fn portfolio_is_complete_and_deterministic() {
        let first = run_causal_drift_portfolio();
        let second = run_causal_drift_portfolio();
        assert_eq!(first, second);
        assert_eq!(
            first.len(),
            CausalRegime::ALL.len() * ControllerKind::ALL.len()
        );
    }
}
