// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Export every held-out seed/controller episode, including missed diagnoses.

use civdyn_self_correction::seeded_monte_carlo::{
    SeededEpisodeOutcome, run_seeded_confirmatory_benchmark,
};

const CSV_COLUMNS: usize = 17;

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn optional_tick(value: Option<usize>) -> String {
    value.map(|tick| tick.to_string()).unwrap_or_default()
}

fn row(outcome: &SeededEpisodeOutcome) -> Vec<String> {
    vec![
        outcome.regime.name().to_string(),
        outcome.controller.name().to_string(),
        outcome.seed.to_string(),
        optional_tick(outcome.signal_detection_tick),
        optional_tick(outcome.covariate_shift_tick),
        outcome
            .diagnosis
            .map(|diagnosis| diagnosis.name().to_string())
            .unwrap_or_default(),
        optional_tick(outcome.diagnosis_tick),
        outcome.correct_diagnosis.to_string(),
        outcome.false_diagnosis.to_string(),
        outcome.missed_diagnosis.to_string(),
        outcome.active_ticks.to_string(),
        outcome.nonpositive_effect_action_ticks.to_string(),
        outcome.missed_beneficial_action_ticks.to_string(),
        outcome.diagnostic_probe_count.to_string(),
        number(outcome.net_utility),
        number(outcome.oracle_net_utility),
        number(outcome.regret_to_oracle),
    ]
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "seeded replicate CSV contract changed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    let report = run_seeded_confirmatory_benchmark();
    eprintln!(
        "writing {} confirmatory episode receipts from seeds {}..{}",
        report.replicates.len(),
        report.confirmatory_seed_start,
        report.confirmatory_seed_start + report.confirmatory_seed_count as u64 - 1
    );
    println!(
        "regime,controller,seed,signal_detection_tick,covariate_shift_tick,diagnosis,diagnosis_tick,correct_diagnosis,false_diagnosis,missed_diagnosis,active_ticks,nonpositive_effect_action_ticks,missed_beneficial_action_ticks,diagnostic_probe_count,net_utility,oracle_net_utility,regret_to_oracle"
    );
    for outcome in &report.replicates {
        emit(row(outcome));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_self_correction::seeded_monte_carlo::run_seeded_confirmatory_benchmark;

    #[test]
    fn every_confirmatory_replicate_has_the_fixed_seventeen_column_contract() {
        let report = run_seeded_confirmatory_benchmark();
        assert_eq!(report.replicates.len(), 20_000);
        for outcome in &report.replicates {
            assert_eq!(row(outcome).len(), CSV_COLUMNS);
        }
    }

    #[test]
    fn confirmatory_seed_receipts_are_unique_per_regime_controller_pair() {
        let report = run_seeded_confirmatory_benchmark();
        for regime in civdyn_self_correction::causal_drift::CausalRegime::ALL {
            for controller in
                civdyn_self_correction::causal_drift::CausalDriftController::ALL
            {
                let count = report
                    .replicates
                    .iter()
                    .filter(|outcome| {
                        outcome.regime == regime && outcome.controller == controller
                    })
                    .count();
                assert_eq!(count, report.confirmatory_seed_count);
            }
        }
    }
}
