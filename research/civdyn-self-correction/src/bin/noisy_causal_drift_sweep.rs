// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Emit one source-replayable CSV row per seeded noisy causal-drift episode.

use civdyn_self_correction::causal_drift::CausalDiagnosis;
use civdyn_self_correction::noisy_causal_drift::{
    FAMILYWISE_CRITICAL_VALUE, MATCHED_PAIRS_PER_LOOK, NOISY_SEEDS,
    NoisyCausalDriftOutcome, run_noisy_causal_drift_portfolio,
};

const CSV_COLUMNS: usize = 25;

fn number(value: f64) -> String {
    format!("{value:.8}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_default()
}

fn optional_tick(value: Option<usize>) -> String {
    value.map(|tick| tick.to_string()).unwrap_or_default()
}

fn optional_text(value: Option<&str>) -> String {
    value.unwrap_or_default().to_string()
}

struct NoisyCsvRecord {
    seed: u64,
    regime: String,
    final_true_effect: f64,
    final_observational_association: f64,
    signal_detection_tick: Option<usize>,
    diagnosis: Option<String>,
    diagnosis_tick: Option<usize>,
    correction_latency_ticks: Option<usize>,
    active_ticks: usize,
    active_after_nonpositive_effect_ticks: usize,
    missed_beneficial_action_ticks: usize,
    probe_looks: usize,
    final_probe_mean: Option<f64>,
    final_probe_standard_error: Option<f64>,
    final_probe_lower: Option<f64>,
    final_probe_upper: Option<f64>,
    final_probe_pairs: usize,
    familywise_critical_value: f64,
    total_probe_cost: f64,
    policy_stopped: bool,
    trace_valid: bool,
    trace_event_count: usize,
    deterministic_diagnosis_tick: Option<usize>,
    deterministic_active_ticks: usize,
    false_sign_diagnosis: bool,
}

impl NoisyCsvRecord {
    fn into_fields(self) -> Vec<String> {
        vec![
            self.seed.to_string(),
            self.regime,
            number(self.final_true_effect),
            number(self.final_observational_association),
            optional_tick(self.signal_detection_tick),
            optional_text(self.diagnosis.as_deref()),
            optional_tick(self.diagnosis_tick),
            optional_tick(self.correction_latency_ticks),
            self.active_ticks.to_string(),
            self.active_after_nonpositive_effect_ticks.to_string(),
            self.missed_beneficial_action_ticks.to_string(),
            self.probe_looks.to_string(),
            optional_number(self.final_probe_mean),
            optional_number(self.final_probe_standard_error),
            optional_number(self.final_probe_lower),
            optional_number(self.final_probe_upper),
            self.final_probe_pairs.to_string(),
            number(self.familywise_critical_value),
            number(self.total_probe_cost),
            self.policy_stopped.to_string(),
            self.trace_valid.to_string(),
            self.trace_event_count.to_string(),
            optional_tick(self.deterministic_diagnosis_tick),
            self.deterministic_active_ticks.to_string(),
            self.false_sign_diagnosis.to_string(),
        ]
    }
}

fn row(outcome: &NoisyCausalDriftOutcome) -> Vec<String> {
    let diagnosis = outcome.diagnosis.map(|value| value.name().to_string());
    let false_sign_diagnosis = outcome.regime
        == civdyn_self_correction::causal_drift::CausalRegime::ConfoundedNull
        && outcome.diagnosis == Some(CausalDiagnosis::CausalSignReversal);
    let probe = outcome.final_probe;

    NoisyCsvRecord {
        seed: outcome.seed,
        regime: outcome.regime.name().to_string(),
        final_true_effect: outcome.final_true_causal_effect,
        final_observational_association: outcome.final_observational_association,
        signal_detection_tick: outcome.signal_detection_tick,
        diagnosis,
        diagnosis_tick: outcome.diagnosis_tick,
        correction_latency_ticks: outcome.correction_latency_ticks,
        active_ticks: outcome.active_ticks,
        active_after_nonpositive_effect_ticks: outcome.active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks: outcome.missed_beneficial_action_ticks,
        probe_looks: outcome.probe_looks,
        final_probe_mean: probe.map(|value| value.mean_difference),
        final_probe_standard_error: probe.map(|value| value.standard_error),
        final_probe_lower: probe.map(|value| value.lower_bound),
        final_probe_upper: probe.map(|value| value.upper_bound),
        final_probe_pairs: probe.map_or(0, |value| value.sample_pairs),
        familywise_critical_value: FAMILYWISE_CRITICAL_VALUE,
        total_probe_cost: outcome.total_probe_cost,
        policy_stopped: outcome.policy_stopped,
        trace_valid: outcome.trace_valid,
        trace_event_count: outcome.trace.len(),
        deterministic_diagnosis_tick: outcome.deterministic_diagnosis_tick,
        deterministic_active_ticks: outcome.deterministic_active_ticks,
        false_sign_diagnosis,
    }
    .into_fields()
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "noisy causal-drift CSV contract changed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "seed,regime,final_true_causal_effect,final_observational_association,signal_detection_tick,diagnosis,diagnosis_tick,correction_latency_ticks,active_ticks,active_after_nonpositive_effect_ticks,missed_beneficial_action_ticks,probe_looks,final_probe_mean,final_probe_standard_error,final_probe_lower_bound,final_probe_upper_bound,final_probe_pairs,familywise_critical_value,total_probe_cost,policy_stopped,trace_valid,trace_event_count,deterministic_diagnosis_tick,deterministic_active_ticks,false_sign_diagnosis"
    );

    let outcomes = run_noisy_causal_drift_portfolio();
    assert_eq!(
        outcomes.len(),
        NOISY_SEEDS
            * civdyn_self_correction::causal_drift::CausalRegime::ALL.len()
    );
    assert_eq!(MATCHED_PAIRS_PER_LOOK, 2048);

    for outcome in &outcomes {
        emit(row(outcome));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_self_correction::noisy_causal_drift::{
        NOISY_SEEDS, run_noisy_causal_drift_portfolio,
    };

    #[test]
    fn exported_noisy_episode_rows_obey_the_twenty_five_column_contract() {
        for outcome in run_noisy_causal_drift_portfolio().iter().take(12) {
            assert_eq!(row(outcome).len(), CSV_COLUMNS);
        }
    }

    #[test]
    fn fixed_portfolio_retains_all_seeded_episodes() {
        assert_eq!(
            run_noisy_causal_drift_portfolio().len(),
            NOISY_SEEDS * civdyn_self_correction::causal_drift::CausalRegime::ALL.len()
        );
    }
}
