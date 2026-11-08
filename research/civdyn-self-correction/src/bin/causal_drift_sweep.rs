// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Export the fixed synthetic causal-drift portfolio as a separate CSV.

use civdyn_self_correction::causal_drift::{
    CausalDriftOutcome, run_causal_drift_portfolio,
};

const CSV_COLUMNS: usize = 21;

fn number(value: f64) -> String {
    format!("{value:.6}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_default()
}

fn optional_tick(value: Option<usize>) -> String {
    value.map(|tick| tick.to_string()).unwrap_or_default()
}

struct CausalDriftCsvRecord {
    regime: String,
    controller: String,
    final_true_causal_effect: f64,
    final_observational_association: f64,
    last_matched_probe_estimate: Option<f64>,
    active_ticks: usize,
    active_after_nonpositive_effect_ticks: usize,
    cumulative_causal_contribution: f64,
    action_cost: f64,
    diagnostic_probe_count: usize,
    diagnostic_probe_cost: f64,
    net_utility: f64,
    oracle_net_utility: f64,
    regret_to_oracle: f64,
    false_causal_attribution: bool,
    mechanism_identified: bool,
    diagnosis: Option<String>,
    detection_tick: Option<usize>,
    correction_latency_ticks: Option<usize>,
    trace_valid: bool,
    trace_event_count: usize,
}

impl CausalDriftCsvRecord {
    fn into_fields(self) -> Vec<String> {
        vec![
            self.regime,
            self.controller,
            number(self.final_true_causal_effect),
            number(self.final_observational_association),
            optional_number(self.last_matched_probe_estimate),
            self.active_ticks.to_string(),
            self.active_after_nonpositive_effect_ticks.to_string(),
            number(self.cumulative_causal_contribution),
            number(self.action_cost),
            self.diagnostic_probe_count.to_string(),
            number(self.diagnostic_probe_cost),
            number(self.net_utility),
            number(self.oracle_net_utility),
            number(self.regret_to_oracle),
            self.false_causal_attribution.to_string(),
            self.mechanism_identified.to_string(),
            self.diagnosis.unwrap_or_default(),
            optional_tick(self.detection_tick),
            optional_tick(self.correction_latency_ticks),
            self.trace_valid.to_string(),
            self.trace_event_count.to_string(),
        ]
    }
}

fn row(outcome: &CausalDriftOutcome) -> Vec<String> {
    CausalDriftCsvRecord {
        regime: outcome.regime.name().to_string(),
        controller: outcome.controller.name().to_string(),
        final_true_causal_effect: outcome.final_true_causal_effect,
        final_observational_association: outcome.final_observational_association,
        last_matched_probe_estimate: outcome.last_matched_probe_estimate,
        active_ticks: outcome.active_ticks,
        active_after_nonpositive_effect_ticks: outcome.active_after_nonpositive_effect_ticks,
        cumulative_causal_contribution: outcome.cumulative_causal_contribution,
        action_cost: outcome.action_cost,
        diagnostic_probe_count: outcome.diagnostic_probe_count,
        diagnostic_probe_cost: outcome.diagnostic_probe_cost,
        net_utility: outcome.net_utility,
        oracle_net_utility: outcome.oracle_net_utility,
        regret_to_oracle: outcome.regret_to_oracle,
        false_causal_attribution: outcome.false_causal_attribution,
        mechanism_identified: outcome.mechanism_identified,
        diagnosis: outcome.diagnosis.map(|diagnosis| diagnosis.name().to_string()),
        detection_tick: outcome.detection_tick,
        correction_latency_ticks: outcome.correction_latency_ticks,
        trace_valid: outcome.trace_valid,
        trace_event_count: outcome.trace.len(),
    }
    .into_fields()
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "causal-drift CSV contract changed"
    );
    println!("{}", fields.join(","));
}

fn main() {
    println!(
        "regime,controller,final_true_causal_effect,final_observational_association,last_matched_probe_estimate,active_ticks,active_after_nonpositive_effect_ticks,cumulative_causal_contribution,action_cost,diagnostic_probe_count,diagnostic_probe_cost,net_utility,oracle_net_utility,regret_to_oracle,false_causal_attribution,mechanism_identified,diagnosis,detection_tick,correction_latency_ticks,trace_valid,trace_event_count"
    );
    for outcome in run_causal_drift_portfolio() {
        emit(row(&outcome));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_self_correction::causal_drift::{
        CausalRegime, run_causal_drift_portfolio,
    };
    use civdyn_self_correction::ControllerKind;

    #[test]
    fn every_causal_drift_row_has_the_fixed_twenty_one_column_contract() {
        for outcome in run_causal_drift_portfolio() {
            assert_eq!(row(&outcome).len(), CSV_COLUMNS);
        }
    }

    #[test]
    fn portfolio_covers_every_regime_and_controller() {
        assert_eq!(
            run_causal_drift_portfolio().len(),
            CausalRegime::ALL.len() * ControllerKind::ALL.len()
        );
    }
}
