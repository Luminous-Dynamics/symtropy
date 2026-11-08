// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Emit every seeded noisy causal-drift episode and an aggregate summary.
//!
//! Usage:
//! causal-drift-monte-carlo <episodes.csv> <summary.csv>

use civdyn_self_correction::stochastic::{
    MonteCarloConfig, MonteCarloEpisode, MonteCarloSummary, run_monte_carlo,
};
use std::env;
use std::fs;
use std::process::ExitCode;

const EPISODE_COLUMNS: usize = 25;
const SUMMARY_COLUMNS: usize = 19;

fn optional_tick(value: Option<usize>) -> String {
    value.map(|tick| tick.to_string()).unwrap_or_default()
}

fn optional_number(value: Option<f64>) -> String {
    value.map(|number| format!("{number:.8}")).unwrap_or_default()
}

fn diagnosis(value: Option<impl AsRef<str>>) -> String {
    value.map(|name| name.as_ref().to_string()).unwrap_or_default()
}

fn episode_fields(episode: &MonteCarloEpisode) -> Vec<String> {
    vec![
        episode.seed.to_string(),
        episode.regime.name().to_string(),
        episode.controller.name().to_string(),
        episode.signal_detected.to_string(),
        optional_tick(episode.signal_detection_tick),
        optional_tick(episode.signal_detection_delay_ticks),
        episode.premature_signal_alarm.to_string(),
        episode
            .diagnosis
            .map(|value| value.name().to_string())
            .unwrap_or_default(),
        optional_tick(episode.diagnosis_tick),
        episode.diagnosis_correct.to_string(),
        episode.false_mechanism_diagnosis.to_string(),
        optional_tick(episode.diagnosis_after_signal_ticks),
        episode.active_ticks.to_string(),
        episode.active_after_nonpositive_effect_ticks.to_string(),
        episode.missed_beneficial_action_ticks.to_string(),
        episode.proxy_driven_action_under_nonpositive_effect.to_string(),
        episode.diagnostic_probe_count.to_string(),
        episode.probe_ci_coverage_count.to_string(),
        episode.probe_ci_count.to_string(),
        optional_number(episode.last_probe_estimate),
        optional_number(episode.last_probe_lower_95),
        optional_number(episode.last_probe_upper_95),
        format!("{:.8}", episode.net_utility),
        format!("{:.8}", episode.oracle_net_utility),
        format!("{:.8}", episode.regret_to_oracle),
    ]
}

fn summary_fields(summary: &MonteCarloSummary) -> Vec<String> {
    vec![
        summary.regime.name().to_string(),
        summary.controller.name().to_string(),
        summary.episodes.to_string(),
        format!("{:.8}", summary.signal_detection_rate),
        format!("{:.8}", summary.signal_detection_ci_low),
        format!("{:.8}", summary.signal_detection_ci_high),
        format!("{:.8}", summary.premature_alarm_rate),
        format!("{:.8}", summary.correct_diagnosis_rate),
        format!("{:.8}", summary.correct_diagnosis_ci_low),
        format!("{:.8}", summary.correct_diagnosis_ci_high),
        format!("{:.8}", summary.false_diagnosis_rate),
        optional_number(summary.mean_signal_detection_delay_ticks),
        optional_number(summary.mean_diagnosis_after_signal_ticks),
        format!("{:.8}", summary.mean_active_after_nonpositive_effect_ticks),
        format!("{:.8}", summary.mean_missed_beneficial_action_ticks),
        format!("{:.8}", summary.mean_probe_count),
        optional_number(summary.mean_probe_ci_coverage),
        format!("{:.8}", summary.mean_net_utility),
        format!("{:.8}", summary.mean_regret_to_oracle),
    ]
}

fn csv_row(fields: Vec<String>, expected: usize) -> Result<String, String> {
    if fields.len() != expected {
        return Err(format!(
            "CSV schema mismatch: expected {expected} columns, got {}",
            fields.len()
        ));
    }
    if fields.iter().any(|field| field.contains([',', '\n', '\r', '"'])) {
        return Err("CSV field contains an unescaped delimiter".to_string());
    }
    Ok(fields.join(","))
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 3 {
        return Err(
            "Usage: causal-drift-monte-carlo <episodes.csv> <summary.csv>".to_string(),
        );
    }
    let config = MonteCarloConfig::default();
    let report = run_monte_carlo(config).map_err(|error| error.to_string())?;
    let mut episodes = String::from(
        "seed,regime,controller,signal_detected,signal_detection_tick,signal_detection_delay_ticks,premature_signal_alarm,diagnosis,diagnosis_tick,diagnosis_correct,false_mechanism_diagnosis,diagnosis_after_signal_ticks,active_ticks,active_after_nonpositive_effect_ticks,missed_beneficial_action_ticks,proxy_driven_action_under_nonpositive_effect,diagnostic_probe_count,probe_ci_coverage_count,probe_ci_count,last_probe_estimate,last_probe_lower_95,last_probe_upper_95,net_utility,oracle_net_utility,regret_to_oracle\n",
    );
    for episode in &report.episodes {
        episodes.push_str(
            &csv_row(episode_fields(episode), EPISODE_COLUMNS)
                .map_err(|error| format!("episode seed {}: {error}", episode.seed))?,
        );
        episodes.push('\n');
    }

    let mut summaries = String::from(
        "regime,controller,episodes,signal_detection_rate,signal_detection_ci_low,signal_detection_ci_high,premature_alarm_rate,correct_diagnosis_rate,correct_diagnosis_ci_low,correct_diagnosis_ci_high,false_diagnosis_rate,mean_signal_detection_delay_ticks,mean_diagnosis_after_signal_ticks,mean_active_after_nonpositive_effect_ticks,mean_missed_beneficial_action_ticks,mean_probe_count,mean_probe_ci_coverage,mean_net_utility,mean_regret_to_oracle\n",
    );
    for summary in &report.summaries {
        summaries.push_str(
            &csv_row(summary_fields(summary), SUMMARY_COLUMNS)
                .map_err(|error| format!("summary {} / {}: {error}", summary.regime.name(), summary.controller.name()))?,
        );
        summaries.push('\n');
    }

    let episodes_path = &args[1];
    let summaries_path = &args[2];
    fs::write(episodes_path, episodes)
        .map_err(|error| format!("cannot write {}: {error}", episodes_path.to_string_lossy()))?;
    fs::write(summaries_path, summaries)
        .map_err(|error| format!("cannot write {}: {error}", summaries_path.to_string_lossy()))?;
    eprintln!(
        "wrote {} seed-level episodes and {} regime/controller summaries; seed range {}..{}",
        report.episodes.len(),
        report.summaries.len(),
        config.base_seed,
        config.base_seed + config.replicates_per_regime as u64 - 1
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("causal-drift-monte-carlo: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_self_correction::stochastic::{
        CausalDriftController, CausalRegime, run_monte_carlo,
    };

    #[test]
    fn default_report_exports_expected_number_of_episodes_and_summaries() {
        let report = run_monte_carlo(MonteCarloConfig::default()).expect("valid default config");
        assert_eq!(
            report.episodes.len(),
            CausalRegime::ALL.len()
                * CausalDriftController::ALL.len()
                * MonteCarloConfig::default().replicates_per_regime
        );
        assert_eq!(
            report.summaries.len(),
            CausalRegime::ALL.len() * CausalDriftController::ALL.len()
        );
        for episode in report.episodes {
            assert_eq!(episode.seed >= MonteCarloConfig::default().base_seed, true);
        }
    }

    #[test]
    fn csv_header_contracts_are_fixed() {
        assert_eq!(EPISODE_COLUMNS, 25);
        assert_eq!(SUMMARY_COLUMNS, 19);
    }
}
