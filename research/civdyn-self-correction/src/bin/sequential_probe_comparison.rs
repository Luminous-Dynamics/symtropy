// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Paired fixed-look versus anytime-valid e-process episode comparison.

use civdyn_self_correction::causal_drift::{CausalDiagnosis, CausalRegime};
use civdyn_self_correction::noisy_causal_drift::{
    MATCHED_PAIRS_PER_LOOK, NOISY_SEEDS, run_noisy_causal_drift,
};
use civdyn_self_correction::sequential_inference::{
    AnytimeCausalOutcome, SequentialDiagnosis, run_anytime_causal_episode,
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

fn optional_text(value: Option<String>) -> String {
    value.unwrap_or_default()
}

struct ComparisonRecord {
    seed: u64,
    regime: String,
    fixed_diagnosis: Option<String>,
    fixed_diagnosis_tick: Option<usize>,
    fixed_policy_stopped: bool,
    fixed_sample_pairs: usize,
    anytime_diagnosis: Option<String>,
    anytime_diagnosis_tick: Option<usize>,
    anytime_policy_stopped: bool,
    anytime_sample_pairs: usize,
    sample_reduction_fraction: f64,
    signal_detection_tick: Option<usize>,
    active_ticks: usize,
    active_after_nonpositive_effect_ticks: usize,
    missed_beneficial_action_ticks: usize,
    look_1_pairs: usize,
    look_2_pairs: usize,
    look_3_pairs: usize,
    look_4_pairs: usize,
    log_e_negative_effect: f64,
    log_e_below_threshold: f64,
    log_e_beneficial_effect: f64,
    trace_valid: bool,
    trace_event_count: usize,
    seed_identity: u64,
}

impl ComparisonRecord {
    fn into_fields(self) -> Vec<String> {
        vec![
            self.seed.to_string(),
            self.regime,
            optional_text(self.fixed_diagnosis),
            optional_tick(self.fixed_diagnosis_tick),
            self.fixed_policy_stopped.to_string(),
            self.fixed_sample_pairs.to_string(),
            optional_text(self.anytime_diagnosis),
            optional_tick(self.anytime_diagnosis_tick),
            self.anytime_policy_stopped.to_string(),
            self.anytime_sample_pairs.to_string(),
            number(self.sample_reduction_fraction),
            optional_tick(self.signal_detection_tick),
            self.active_ticks.to_string(),
            self.active_after_nonpositive_effect_ticks.to_string(),
            self.missed_beneficial_action_ticks.to_string(),
            self.look_1_pairs.to_string(),
            self.look_2_pairs.to_string(),
            self.look_3_pairs.to_string(),
            self.look_4_pairs.to_string(),
            number(self.log_e_negative_effect),
            number(self.log_e_below_threshold),
            number(self.log_e_beneficial_effect),
            self.trace_valid.to_string(),
            self.trace_event_count.to_string(),
            self.seed_identity.to_string(),
        ]
    }
}

fn row(
    fixed: &civdyn_self_correction::noisy_causal_drift::NoisyCausalDriftOutcome,
    anytime: &AnytimeCausalOutcome,
) -> Vec<String> {
    let fixed_diagnosis = fixed.diagnosis.map(|value: CausalDiagnosis| value.name().to_string());
    let anytime_diagnosis = anytime
        .diagnosis
        .map(|value: SequentialDiagnosis| value.name().to_string());
    let fixed_sample_pairs = fixed.probe_looks * MATCHED_PAIRS_PER_LOOK;
    let sample_reduction_fraction = if fixed_sample_pairs == 0 {
        0.0
    } else {
        1.0 - anytime.total_sample_pairs as f64 / fixed_sample_pairs as f64
    };

    ComparisonRecord {
        seed: fixed.seed,
        regime: fixed.regime.name().to_string(),
        fixed_diagnosis,
        fixed_diagnosis_tick: fixed.diagnosis_tick,
        fixed_policy_stopped: fixed.policy_stopped,
        fixed_sample_pairs,
        anytime_diagnosis,
        anytime_diagnosis_tick: anytime.diagnosis_tick,
        anytime_policy_stopped: anytime.policy_stopped,
        anytime_sample_pairs: anytime.total_sample_pairs,
        sample_reduction_fraction,
        signal_detection_tick: anytime.signal_detection_tick,
        active_ticks: anytime.active_ticks,
        active_after_nonpositive_effect_ticks: anytime.active_after_nonpositive_effect_ticks,
        missed_beneficial_action_ticks: anytime.missed_beneficial_action_ticks,
        look_1_pairs: anytime.sample_pairs_per_look[0],
        look_2_pairs: anytime.sample_pairs_per_look[1],
        look_3_pairs: anytime.sample_pairs_per_look[2],
        look_4_pairs: anytime.sample_pairs_per_look[3],
        log_e_negative_effect: anytime.final_log_e_negative_effect,
        log_e_below_threshold: anytime.final_log_e_below_threshold,
        log_e_beneficial_effect: anytime.final_log_e_beneficial_effect,
        trace_valid: anytime.trace_valid,
        trace_event_count: anytime.trace.len(),
        seed_identity: anytime.seed,
    }
    .into_fields()
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "sequential comparison CSV contract changed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "seed,regime,fixed_diagnosis,fixed_diagnosis_tick,fixed_policy_stopped,fixed_sample_pairs,anytime_diagnosis,anytime_diagnosis_tick,anytime_policy_stopped,anytime_sample_pairs,sample_reduction_fraction,signal_detection_tick,active_ticks,active_after_nonpositive_effect_ticks,missed_beneficial_action_ticks,look_1_pairs,look_2_pairs,look_3_pairs,look_4_pairs,log_e_negative_effect,log_e_below_threshold,log_e_beneficial_effect,trace_valid,trace_event_count,seed_identity"
    );

    let mut rows = 0usize;
    for regime in CausalRegime::ALL {
        for seed_index in 0..NOISY_SEEDS {
            let fixed = run_noisy_causal_drift(regime, seed_index);
            let anytime = run_anytime_causal_episode(regime, seed_index);
            assert_eq!(fixed.seed, anytime.seed, "paired episode identity changed");
            emit(row(&fixed, &anytime));
            rows += 1;
        }
    }
    assert_eq!(rows, CausalRegime::ALL.len() * NOISY_SEEDS);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comparison_row_has_the_fixed_twenty_five_column_contract() {
        let fixed = run_noisy_causal_drift(CausalRegime::SignReversal, 7);
        let anytime = run_anytime_causal_episode(CausalRegime::SignReversal, 7);
        assert_eq!(row(&fixed, &anytime).len(), CSV_COLUMNS);
        assert_eq!(fixed.seed, anytime.seed);
    }

    #[test]
    fn anytime_probe_reduces_sample_count_for_a_sign_reversal_episode() {
        let fixed = run_noisy_causal_drift(CausalRegime::SignReversal, 11);
        let anytime = run_anytime_causal_episode(CausalRegime::SignReversal, 11);
        let fixed_pairs = fixed.probe_looks * MATCHED_PAIRS_PER_LOOK;
        assert!(anytime.total_sample_pairs < fixed_pairs);
    }
}
