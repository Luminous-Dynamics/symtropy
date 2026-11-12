// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_closed_loop_policy::{HORIZON, PolicyOutcome, run_reference_portfolio};
use std::error::Error;

const CSV_COLUMNS: usize = 36;
const SYNTHETIC_SOURCE_ID: &str = "synthetic-civdyn-closed-loop-known-answer-v1";

fn csv_field(value: &str) -> String {
    if value
        .chars()
        .any(|character| matches!(character, ',' | '"' | '\n' | '\r'))
    {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: &[String]) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "closed-loop policy receipt must match frozen schema"
    );
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn optional<T: ToString>(value: Option<T>) -> String {
    value.map(|item| item.to_string()).unwrap_or_default()
}

fn action_trace(outcome: &PolicyOutcome) -> String {
    outcome
        .actions
        .iter()
        .map(|action| action.id())
        .collect::<Vec<_>>()
        .join("|")
}

fn emit_outcome(outcome: &PolicyOutcome) {
    let errors = outcome.forecast_errors;
    let error_fields = errors.map_or(
        [
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ],
        |values| values.map(|value| format!("{value:.6}")),
    );

    emit(&[
        outcome.world_id.to_string(),
        outcome.world_digest.to_string(),
        outcome.policy.id().to_string(),
        optional(outcome.candidate_model_id),
        optional(outcome.candidate_model_digest),
        outcome.evaluator_oracle_access.to_string(),
        HORIZON.to_string(),
        action_trace(outcome),
        outcome.cumulative_service_points.to_string(),
        outcome.oracle_service_points.to_string(),
        outcome.raw_service_regret.to_string(),
        outcome.terminal_feasible.to_string(),
        outcome.minimum_water_service.to_string(),
        outcome.minimum_energy_service.to_string(),
        outcome.water_floor_violation_ticks.to_string(),
        outcome.energy_floor_violation_ticks.to_string(),
        outcome.worst_water_shortfall.to_string(),
        outcome.worst_energy_shortfall.to_string(),
        outcome.ending_water_reserve.to_string(),
        outcome.ending_energy_reserve.to_string(),
        outcome.ending_water_debt.to_string(),
        outcome.ending_energy_debt.to_string(),
        outcome.ending_maintenance_credits.to_string(),
        outcome.action_ranking_matches.to_string(),
        outcome.decision_count.to_string(),
        outcome.replan_count.to_string(),
        outcome.abstention_ticks.to_string(),
        error_fields[0].clone(),
        error_fields[1].clone(),
        error_fields[2].clone(),
        error_fields[3].clone(),
        error_fields[4].clone(),
        error_fields[5].clone(),
        outcome.scenario_manifest_digest.to_string(),
        outcome.split_manifest_digest.to_string(),
        format!("{}-paired-run-001", outcome.world_id),
    ]);
}

fn main() -> Result<(), Box<dyn Error>> {
    println!(
        "true_world_id,true_world_digest,policy_id,candidate_model_id,candidate_model_digest,evaluator_oracle_access,horizon,action_trace,cumulative_service_points,oracle_service_points,raw_service_regret,terminal_feasible,minimum_water_service,minimum_energy_service,water_floor_violation_ticks,energy_floor_violation_ticks,worst_water_shortfall,worst_energy_shortfall,ending_water_reserve,ending_energy_reserve,ending_water_debt,ending_energy_debt,ending_maintenance_credits,action_ranking_matches,decision_count,replan_count,abstention_ticks,water_service_mae,energy_service_mae,water_debt_mae,energy_debt_mae,water_reserve_mae,energy_reserve_mae,scenario_manifest_digest,split_manifest_digest,independent_unit_id"
    );
    let outcomes = run_reference_portfolio()?;
    assert_eq!(outcomes.len(), 10);
    for outcome in &outcomes {
        emit_outcome(outcome);
    }
    Ok(())
}
