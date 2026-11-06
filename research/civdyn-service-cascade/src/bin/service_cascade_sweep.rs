// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic synthetic service-cascade and component-criticality portfolio.

use civdyn_service_cascade::{
    NodeCriticality, NodeSummary, Scenario, TickOutcome, node_criticalities, reference_scenario,
    run_scenario,
};

const CSV_COLUMNS: usize = 17;

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "service-cascade CSV schema must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn empty_row(record_type: &str, scenario_id: &str) -> Vec<String> {
    vec![
        record_type.into(),
        scenario_id.into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn emit_tick(scenario: &Scenario, tick: &TickOutcome) {
    for (node_index, node) in scenario.nodes.iter().enumerate() {
        let mut row = empty_row("tick", &scenario.id);
        row[2] = node.id.clone();
        row[3] = node.service_type.clone();
        row[4] = tick.tick_index.to_string();
        row[5] = fmt(scenario.own_availability_by_tick[tick.tick_index][node_index]);
        row[6] = fmt(tick.service_levels[node_index]);
        row[8] = fmt(node.beneficiary_demand_weight);
        emit(row);
    }
}

fn emit_summary(scenario_id: &str, summary: &NodeSummary) {
    let mut row = empty_row("node_summary", scenario_id);
    row[2] = summary.node_id.clone();
    row[3] = summary.service_type.clone();
    row[7] = fmt(summary.minimum_service);
    row[8] = fmt(summary.beneficiary_demand_weight);
    row[9] = fmt(summary.deficit_area_service_ticks);
    row[10] = summary.ticks_below_50_percent.to_string();
    row[11] = summary.ticks_below_90_percent.to_string();
    row[12] = summary
        .recovery_after_ticks
        .map_or_else(String::new, |value| value.to_string());
    emit(row);
}

fn emit_criticality(scenario_id: &str, criticality: &NodeCriticality) {
    let mut row = empty_row("criticality", scenario_id);
    row[2] = criticality.node_id.clone();
    row[13] = fmt(criticality.baseline_service_deficit_area);
    row[14] = fmt(criticality.outage_service_deficit_area);
    row[15] = fmt(criticality.marginal_service_deficit_area);
    emit(row);
}

fn main() {
    println!(
        "record_type,scenario_id,node_id,service_type,tick_index,own_availability,effective_service,minimum_service,beneficiary_demand_weight,deficit_area_service_ticks,ticks_below_50,ticks_below_90,recovery_after_ticks,baseline_service_deficit_area,outage_service_deficit_area,marginal_service_deficit_area,demand_weighted_service_deficit_area"
    );

    let scenario = reference_scenario();
    let outcome = run_scenario(&scenario).expect("reference service cascade must validate");
    for tick in &outcome.ticks {
        emit_tick(&scenario, tick);
    }
    for summary in &outcome.nodes {
        emit_summary(&scenario.id, summary);
    }

    let mut baseline_scenario = scenario.clone();
    baseline_scenario.own_availability_by_tick = vec![vec![1.0; scenario.nodes.len()]; 6];
    baseline_scenario.recovery_start_tick = 0;
    let criticalities = node_criticalities(&baseline_scenario, 1, 3)
        .expect("component outage counterfactuals must validate");
    for criticality in &criticalities {
        emit_criticality(&scenario.id, criticality);
    }

    let mut aggregate = empty_row("scenario_summary", &scenario.id);
    aggregate[8] = fmt(outcome.total_beneficiary_demand_weight);
    aggregate[16] = fmt(outcome.demand_weighted_service_deficit_area);
    emit(aggregate);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_schema_has_seventeen_columns() {
        assert_eq!(CSV_COLUMNS, 17);
    }

    #[test]
    fn reference_portfolio_has_nine_nodes_and_six_ticks() {
        let scenario = reference_scenario();
        let outcome = run_scenario(&scenario).unwrap();
        assert_eq!(scenario.nodes.len(), 9);
        assert_eq!(outcome.ticks.len(), 6);
        assert_eq!(outcome.nodes.len(), 9);
    }
}
