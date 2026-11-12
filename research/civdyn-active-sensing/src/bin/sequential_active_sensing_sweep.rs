// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_active_sensing::sequential::{
    SequentialSensorPlan, SequentialSensorStage, assess_sequential_sensor_plan,
};
use civdyn_active_sensing::{
    ActiveSensingConfig, DecisionWorld, PolicyOutcome, SensorAction, SensorPrediction,
};

const CSV_COLUMNS: usize = 25;

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "sequential-sensing receipt schema mismatch"
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

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn config() -> ActiveSensingConfig {
    ActiveSensingConfig {
        scenario_id: "nonmyopic-two-stage-sensing-v1".to_owned(),
        source_digest: "sha256-nonmyopic-two-stage-source-v1".to_owned(),
        service_unit: "service_units".to_owned(),
        energy_unit: "kWh".to_owned(),
        maximum_energy_kwh: 1.0,
        maximum_latency_ticks: 2,
        maximum_penalty_service_units: 3.0,
        tie_tolerance: 1.0e-9,
    }
}

fn worlds() -> Vec<DecisionWorld> {
    vec![
        DecisionWorld {
            world_id: "world_a".to_owned(),
            world_digest: "sha256-world_a".to_owned(),
            service_floor: 70.0,
            policies: vec![
                PolicyOutcome {
                    policy_id: "repair_a".to_owned(),
                    service_units: 90.0,
                },
                PolicyOutcome {
                    policy_id: "repair_b".to_owned(),
                    service_units: 50.0,
                },
            ],
        },
        DecisionWorld {
            world_id: "world_b".to_owned(),
            world_digest: "sha256-world_b".to_owned(),
            service_floor: 70.0,
            policies: vec![
                PolicyOutcome {
                    policy_id: "repair_a".to_owned(),
                    service_units: 50.0,
                },
                PolicyOutcome {
                    policy_id: "repair_b".to_owned(),
                    service_units: 90.0,
                },
            ],
        },
    ]
}

fn prediction(
    sensor_id: &str,
    world_id: &str,
    observation_id: &str,
    latency_loss_per_tick: f64,
) -> SensorPrediction {
    let world = worlds()
        .into_iter()
        .find(|world| world.world_id == world_id)
        .expect("fixture world exists");
    SensorPrediction {
        world_id: world_id.to_owned(),
        world_digest: world.world_digest,
        observation_id: observation_id.to_owned(),
        missing_cause: None,
        latency_service_loss_per_tick: latency_loss_per_tick,
        prediction_receipt_digest: format!("sha256-{sensor_id}-{world_id}-receipt"),
    }
}

fn fixture_plan(second_stage_outcomes: [(&str, &str, f64); 2]) -> SequentialSensorPlan {
    let setup = SensorAction {
        sensor_id: "inspection_rig_setup".to_owned(),
        protocol_digest: "sha256-inspection-rig-setup".to_owned(),
        energy_cost_kwh: 0.1,
        service_units_per_kwh: 2.0,
        latency_ticks: 1,
        predictions: vec![
            prediction("inspection_rig_setup", "world_a", "ready", 0.3),
            prediction("inspection_rig_setup", "world_b", "ready", 0.3),
        ],
    };
    let inspection = SensorAction {
        sensor_id: "fine_inspection".to_owned(),
        protocol_digest: "sha256-fine-inspection".to_owned(),
        energy_cost_kwh: 0.15,
        service_units_per_kwh: 2.0,
        latency_ticks: 1,
        predictions: second_stage_outcomes
            .into_iter()
            .map(|(world, observation, loss)| {
                prediction("fine_inspection", world, observation, loss)
            })
            .collect(),
    };
    SequentialSensorPlan {
        plan_id: "setup-then-inspect".to_owned(),
        protocol_digest: "sha256-setup-then-inspect".to_owned(),
        stages: vec![
            SequentialSensorStage {
                sensor: setup,
                required_previous_observation: None,
            },
            SequentialSensorStage {
                sensor: inspection,
                required_previous_observation: Some("ready".to_owned()),
            },
        ],
    }
}

fn summary_fields(
    result: &civdyn_active_sensing::sequential::SequentialSensingAssessment,
) -> Vec<String> {
    vec![
        "plan_summary".to_owned(),
        result.scenario_id.clone(),
        result.plan_id.clone(),
        result.status.id().to_owned(),
        result.recommended.to_string(),
        result.myopic_stage_one_status.id().to_owned(),
        result
            .myopic_stage_one_regret
            .map_or_else(String::new, |v| format!("{v:.6}")),
        result.baseline.common_feasible_policy_count.to_string(),
        format!("{:.6}", result.total_energy_kwh),
        format!("{:.6}", result.total_energy_cost_service_units),
        result.total_latency_ticks.to_string(),
        format!("{:.6}", result.maximum_latency_service_loss),
        format!("{:.6}", result.maximum_total_penalty_service_units),
        result
            .worst_case_post_sensing_regret
            .map_or_else(String::new, |v| format!("{v:.6}")),
        result
            .worst_case_regret_reduction
            .map_or_else(String::new, |v| format!("{v:.6}")),
        result.resolves_no_sensing_infeasibility.to_string(),
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

fn main() -> Result<(), String> {
    let candidate_only = std::env::args().nth(1).as_deref() == Some("--candidate-policy-map");
    let worlds = worlds();
    let plan = fixture_plan([
        ("world_a", "trace_alpha", 0.4),
        ("world_b", "trace_beta", 0.4),
    ]);
    let result = assess_sequential_sensor_plan(&config(), &worlds, &plan)?;

    if candidate_only {
        println!("scenario_id,plan_id,observation_history,action_id");
        for decision in &result.candidate_policy {
            println!(
                "{},{},{},{}",
                csv_field(&decision.scenario_id),
                csv_field(&decision.plan_id),
                csv_field(&decision.observation_history),
                csv_field(&decision.action_id)
            );
        }
        return Ok(());
    }

    println!(
        "record_type,scenario_id,plan_id,status,recommended,myopic_stage_one_status,myopic_stage_one_regret,baseline_common_feasible_policy_count,total_energy_kwh,total_energy_cost_service_units,total_latency_ticks,maximum_latency_service_loss,maximum_total_penalty_service_units,worst_case_post_sensing_regret,regret_reduction,resolves_no_sensing_infeasibility,observation_history,selected_policy_id,partition_world_count,partition_worst_regret,world_id,world_regret,world_penalty,world_net_service,oracle_service"
    );
    emit(summary_fields(&result));

    for partition in &result.partitions {
        let mut row = summary_fields(&result);
        row[0] = "observation_partition".to_owned();
        row[16] = partition.observation_history.clone();
        row[17] = partition.selected_policy_id.clone().unwrap_or_default();
        row[18] = partition.world_regrets.len().to_string();
        row[19] = partition
            .worst_case_regret
            .map_or_else(String::new, |v| format!("{v:.6}"));
        emit(row);
        for world in &partition.world_regrets {
            let mut world_row = summary_fields(&result);
            world_row[0] = "world_regret".to_owned();
            world_row[16] = partition.observation_history.clone();
            world_row[17] = world.selected_policy_id.clone();
            world_row[18] = partition.world_regrets.len().to_string();
            world_row[20] = world.world_id.clone();
            world_row[21] = format!("{:.6}", world.regret_service_units);
            world_row[22] = format!("{:.6}", world.sensing_penalty_service_units);
            world_row[23] = format!("{:.6}", world.net_policy_service_units);
            world_row[24] = format!("{:.6}", world.oracle_service_units);
            emit(world_row);
        }
    }

    Ok(())
}
