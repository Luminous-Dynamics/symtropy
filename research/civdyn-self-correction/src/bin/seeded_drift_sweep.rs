// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Export the disjoint-seed, noise-aware causal-drift confirmatory report.

use civdyn_self_correction::seeded_monte_carlo::{
    SeededAggregateRow, SeededBenchmarkReport, run_seeded_confirmatory_benchmark,
};

const CSV_COLUMNS: usize = 34;

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn row(report: &SeededBenchmarkReport, aggregate: &SeededAggregateRow) -> Vec<String> {
    vec![
        number(report.tuning.config.allowance),
        number(report.tuning.config.threshold),
        report.tuning.development_false_alarms.to_string(),
        report.tuning.development_stable_episodes.to_string(),
        report.tuning.development_signal_detections.to_string(),
        report.tuning.development_changed_episodes.to_string(),
        report.tuning.false_alarm_constraint_met.to_string(),
        report.confirmatory_seed_start.to_string(),
        report.confirmatory_seed_count.to_string(),
        aggregate.regime.name().to_string(),
        aggregate.controller.name().to_string(),
        aggregate.episodes.to_string(),
        aggregate.expected_signal_change.to_string(),
        aggregate.signal_alarm.numerator.to_string(),
        number(aggregate.signal_alarm.rate),
        number(aggregate.signal_alarm.lower_95),
        number(aggregate.signal_alarm.upper_95),
        aggregate.signal_false_alarms.to_string(),
        aggregate.correct_diagnoses.numerator.to_string(),
        number(aggregate.correct_diagnoses.rate),
        number(aggregate.correct_diagnoses.lower_95),
        number(aggregate.correct_diagnoses.upper_95),
        aggregate.false_diagnoses.to_string(),
        aggregate.missed_diagnoses.to_string(),
        aggregate
            .mean_signal_detection_tick
            .map(number)
            .unwrap_or_default(),
        aggregate
            .mean_diagnosis_tick
            .map(number)
            .unwrap_or_default(),
        number(aggregate.mean_active_ticks),
        number(aggregate.mean_nonpositive_effect_action_ticks),
        number(aggregate.mean_missed_beneficial_action_ticks),
        number(aggregate.mean_probe_count),
        number(aggregate.mean_net_utility),
        number(aggregate.net_utility_ci_lower_95),
        number(aggregate.net_utility_ci_upper_95),
        number(aggregate.mean_oracle_regret),
    ]
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "seeded causal-drift CSV contract changed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    let report = run_seeded_confirmatory_benchmark();
    eprintln!(
        "CUSUM tuned on {} development seeds: allowance={:.2}, threshold={:.2}, stable false alarms={}/{}, constraint_met={}",
        report.tuning.development_changed_episodes,
        report.tuning.config.allowance,
        report.tuning.config.threshold,
        report.tuning.development_false_alarms,
        report.tuning.development_stable_episodes,
        report.tuning.false_alarm_constraint_met
    );
    println!(
        "cusum_allowance,cusum_threshold,development_false_alarms,development_stable_episodes,development_signal_detections,development_changed_episodes,development_false_alarm_constraint_met,confirmatory_seed_start,confirmatory_seed_count,regime,controller,episodes,expected_signal_change,signal_count,signal_rate,signal_wilson_lower_95,signal_wilson_upper_95,signal_false_alarms,correct_diagnosis_count,diagnosis_accuracy,diagnosis_wilson_lower_95,diagnosis_wilson_upper_95,false_diagnoses,missed_diagnoses,mean_signal_detection_tick,mean_diagnosis_tick,mean_active_ticks,mean_nonpositive_effect_action_ticks,mean_missed_beneficial_action_ticks,mean_probe_count,mean_net_utility,net_utility_ci_lower_95,net_utility_ci_upper_95,mean_oracle_regret"
    );
    for aggregate in &report.rows {
        emit(row(&report, aggregate));
    }
}

