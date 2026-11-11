// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::closed_loop_policy::{
    evaluate_closed_loop_policy, CivicCandidateModel, CivicPolicyKind, CivicScenario,
    CivicState, InfrastructureDynamics, ServiceDemand, ServiceFloor,
};

const CSV_COLUMNS: usize = 32;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "closed-loop policy receipt must match the frozen schema"
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

fn energy_stress_scenario() -> CivicScenario {
    CivicScenario {
        scenario_id: "energy-stress".to_string(),
        scenario_digest: "sha256-civdyn-closed-loop-energy-scenario".to_string(),
        true_world_id: "energy-infrastructure-degradation".to_string(),
        true_world_digest: "sha256-civdyn-true-energy-world".to_string(),
        initial_state: CivicState {
            water_reserve: 6,
            energy_reserve: 6,
            maintenance_debt: 0,
        },
        true_dynamics: InfrastructureDynamics {
            water_decay_per_tick: 0,
            energy_decay_per_tick: 4,
            water_repair_gain: 8,
            energy_repair_gain: 8,
            maintenance_debt_accrual: 1,
            water_repair_debt_relief: 1,
            energy_repair_debt_relief: 1,
        },
        demand: ServiceDemand {
            water: 3,
            energy: 3,
        },
        service_floor: ServiceFloor {
            water: 3,
            energy: 3,
        },
        service_capacity_limit: 20,
        maximum_maintenance_debt: 2,
        horizon: 2,
        intervention_budget: 2,
    }
}

fn water_stress_scenario() -> CivicScenario {
    CivicScenario {
        scenario_id: "water-stress".to_string(),
        scenario_digest: "sha256-civdyn-closed-loop-water-scenario".to_string(),
        true_world_id: "water-infrastructure-degradation".to_string(),
        true_world_digest: "sha256-civdyn-true-water-world".to_string(),
        initial_state: CivicState {
            water_reserve: 8,
            energy_reserve: 6,
            maintenance_debt: 0,
        },
        true_dynamics: InfrastructureDynamics {
            water_decay_per_tick: 4,
            energy_decay_per_tick: 0,
            water_repair_gain: 8,
            energy_repair_gain: 8,
            maintenance_debt_accrual: 1,
            water_repair_debt_relief: 1,
            energy_repair_debt_relief: 1,
        },
        demand: ServiceDemand {
            water: 3,
            energy: 3,
        },
        service_floor: ServiceFloor {
            water: 3,
            energy: 3,
        },
        service_capacity_limit: 20,
        maximum_maintenance_debt: 2,
        horizon: 2,
        intervention_budget: 2,
    }
}

fn matching_model(scenario: &CivicScenario, id: &str) -> CivicCandidateModel {
    CivicCandidateModel {
        model_id: id.to_string(),
        model_digest: format!("sha256-model-{id}"),
        dynamics: scenario.true_dynamics,
    }
}

fn wrong_energy_model() -> CivicCandidateModel {
    CivicCandidateModel {
        model_id: "wrong-water-mechanism".to_string(),
        model_digest: "sha256-model-wrong-water-mechanism".to_string(),
        dynamics: InfrastructureDynamics {
            water_decay_per_tick: 4,
            energy_decay_per_tick: 0,
            water_repair_gain: 8,
            energy_repair_gain: 8,
            maintenance_debt_accrual: 1,
            water_repair_debt_relief: 1,
            energy_repair_debt_relief: 1,
        },
    }
}

fn actions_csv(actions: &[civdyn_self_correction::closed_loop_policy::CivicAction]) -> String {
    actions
        .iter()
        .map(|action| action.name())
        .collect::<Vec<_>>()
        .join(";")
}

fn forecast_error(
    result: &civdyn_self_correction::closed_loop_policy::CivicPolicyEvaluation,
    water: bool,
) -> String {
    let mut total = 0_u64;
    let mut count = 0_u64;
    for step in &result.steps {
        let predicted = if water {
            step.predicted_water_service
        } else {
            step.predicted_energy_service
        };
        if let Some(predicted) = predicted {
            let actual = if water {
                step.actual_water_service
            } else {
                step.actual_energy_service
            };
            total += u64::from(predicted.abs_diff(actual));
            count += 1;
        }
    }
    if count == 0 {
        String::new()
    } else {
        format!("{:.6}", total as f64 / count as f64)
    }
}

fn emit_result(
    scenario: &CivicScenario,
    kind: CivicPolicyKind,
    model: Option<&CivicCandidateModel>,
) -> Result<(), Box<dyn std::error::Error>> {
    let result = evaluate_closed_loop_policy(scenario, kind, model)?;
    let oracle = result.oracle.as_ref();
    let terminal = result.metrics.terminal_state;
    emit(vec![
        result.scenario_id,
        result.scenario_digest,
        result.true_world_id,
        result.true_world_digest,
        result.policy_kind.name().to_string(),
        result.model_id.unwrap_or_default(),
        result.model_digest.unwrap_or_default(),
        actions_csv(&result.actions),
        scenario.horizon.to_string(),
        result.metrics.total_interventions.to_string(),
        result.metrics.total_intervention_cost.to_string(),
        result.metrics.total_water_service.to_string(),
        result.metrics.total_energy_service.to_string(),
        format!("{:.8}", result.metrics.water_coverage),
        format!("{:.8}", result.metrics.energy_coverage),
        format!("{:.8}", result.metrics.minimum_service_coverage),
        result.metrics.service_floor_violation_ticks.to_string(),
        result.metrics.water_shortfall_ticks.to_string(),
        result.metrics.energy_shortfall_ticks.to_string(),
        result.metrics.maximum_water_shortfall.to_string(),
        result.metrics.maximum_energy_shortfall.to_string(),
        terminal.water_reserve.to_string(),
        terminal.energy_reserve.to_string(),
        terminal.maintenance_debt.to_string(),
        result.metrics.feasible.to_string(),
        oracle.map(|value| actions_csv(&value.actions)).unwrap_or_default(),
        oracle
            .map(|value| format!("{:.8}", value.metrics.minimum_service_coverage))
            .unwrap_or_default(),
        result
            .water_service_regret
            .map(|value| format!("{value:.8}"))
            .unwrap_or_default(),
        result
            .energy_service_regret
            .map(|value| format!("{value:.8}"))
            .unwrap_or_default(),
        result
            .minimum_service_coverage_regret
            .map(|value| format!("{value:.8}"))
            .unwrap_or_default(),
        forecast_error(&result, true),
        forecast_error(&result, false),
    ]);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let energy = energy_stress_scenario();
    let water = water_stress_scenario();

    println!(
        "scenario_id,scenario_digest,true_world_id,true_world_digest,policy_kind,model_id,model_digest,actions,horizon,total_interventions,total_intervention_cost,total_water_service,total_energy_service,water_coverage,energy_coverage,minimum_service_coverage,service_floor_violation_ticks,water_shortfall_ticks,energy_shortfall_ticks,maximum_water_shortfall,maximum_energy_shortfall,terminal_water_reserve,terminal_energy_reserve,terminal_maintenance_debt,feasible,oracle_actions,oracle_minimum_service_coverage,water_service_regret,energy_service_regret,minimum_service_coverage_regret,mean_water_forecast_abs_error,mean_energy_forecast_abs_error"
    );

    emit_result(
        &energy,
        CivicPolicyKind::ReactiveReserveBaseline,
        None,
    )?;
    emit_result(
        &energy,
        CivicPolicyKind::ActionConditionedModel,
        Some(&matching_model(&energy, "correct-energy")),
    )?;
    emit_result(
        &energy,
        CivicPolicyKind::ActionConditionedModel,
        Some(&wrong_energy_model()),
    )?;

    emit_result(
        &water,
        CivicPolicyKind::ReactiveReserveBaseline,
        None,
    )?;
    emit_result(
        &water,
        CivicPolicyKind::ActionConditionedModel,
        Some(&matching_model(&water, "correct-water")),
    )?;
    Ok(())
}
