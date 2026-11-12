// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_active_sensing::contingent::{
    CandidatePolicyNode, ContingentPredictionReceipt, ContingentSearchLimits, ContingentSensor,
    assess_contingent_sensing,
};
use civdyn_active_sensing::{
    ActiveSensingConfig, DecisionWorld, MissingCause, PolicyOutcome, SensorAction, SensorPrediction,
};

const SCENARIO_ID: &str = "contingent-three-world-screen-and-inspect-v1";
const SOURCE_DIGEST: &str = "sha256-contingent-three-world-source-v1";
const EVALUATOR_COLUMNS: usize = 36;

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        EVALUATOR_COLUMNS,
        "contingent-sensing evaluator receipt schema mismatch"
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

fn world(id: &str, defer: f64, repair: f64) -> DecisionWorld {
    DecisionWorld {
        world_id: id.to_owned(),
        world_digest: format!("sha256-{id}"),
        service_floor: 70.0,
        policies: vec![
            PolicyOutcome {
                policy_id: "defer".to_owned(),
                service_units: defer,
            },
            PolicyOutcome {
                policy_id: "repair".to_owned(),
                service_units: repair,
            },
        ],
    }
}

fn worlds() -> Vec<DecisionWorld> {
    vec![
        world("healthy", 100.0, 85.0),
        world("degraded", 40.0, 80.0),
        world("healthy_sensor_fault", 100.0, 85.0),
    ]
}

fn sensor(
    id: &str,
    energy: f64,
    conversion: f64,
    latency: u32,
    prerequisite: Option<&str>,
    outcomes: Vec<(&str, &str, Option<MissingCause>, f64)>,
) -> ContingentSensor {
    let predictions = outcomes
        .into_iter()
        .map(
            |(world_id, observation_id, missing_cause, latency_loss)| SensorPrediction {
                world_id: world_id.to_owned(),
                world_digest: format!("sha256-{world_id}"),
                observation_id: observation_id.to_owned(),
                missing_cause,
                latency_service_loss_per_tick: latency_loss,
                prediction_receipt_digest: format!("sha256-{id}-{world_id}-receipt"),
            },
        )
        .collect();
    ContingentSensor {
        sensor: SensorAction {
            sensor_id: id.to_owned(),
            protocol_digest: format!("sha256-{id}-protocol"),
            energy_cost_kwh: energy,
            service_units_per_kwh: conversion,
            latency_ticks: latency,
            predictions,
        },
        required_previous_observation: prerequisite.map(str::to_owned),
    }
}

fn sensors() -> Vec<ContingentSensor> {
    vec![
        sensor(
            "screen",
            0.2,
            2.0,
            1,
            None,
            vec![
                ("healthy", "healthy", None, 0.2),
                (
                    "degraded",
                    "missing",
                    Some(MissingCause::CommonCausePowerLoss),
                    0.5,
                ),
                (
                    "healthy_sensor_fault",
                    "missing",
                    Some(MissingCause::TelemetryNetworkFault),
                    0.1,
                ),
            ],
        ),
        sensor(
            "independent_inspection",
            0.15,
            2.0,
            1,
            Some("missing"),
            vec![
                ("healthy", "healthy", None, 0.2),
                ("degraded", "degraded", None, 0.2),
                ("healthy_sensor_fault", "healthy", None, 0.1),
            ],
        ),
        sensor(
            "uninformative_followup",
            0.1,
            2.0,
            1,
            Some("missing"),
            vec![
                ("healthy", "healthy", None, 0.1),
                (
                    "degraded",
                    "missing",
                    Some(MissingCause::SensorDeviceFailure),
                    0.1,
                ),
                (
                    "healthy_sensor_fault",
                    "missing",
                    Some(MissingCause::InspectionNonresponse),
                    0.1,
                ),
            ],
        ),
        sensor(
            "over_budget_followup",
            1.0,
            2.0,
            1,
            Some("missing"),
            vec![
                ("healthy", "healthy", None, 0.1),
                ("degraded", "degraded", None, 0.1),
                ("healthy_sensor_fault", "healthy", None, 0.1),
            ],
        ),
    ]
}

fn config() -> ActiveSensingConfig {
    ActiveSensingConfig {
        scenario_id: SCENARIO_ID.to_owned(),
        source_digest: SOURCE_DIGEST.to_owned(),
        service_unit: "service_units".to_owned(),
        energy_unit: "kWh".to_owned(),
        maximum_energy_kwh: 0.5,
        maximum_latency_ticks: 2,
        maximum_penalty_service_units: 2.0,
        tie_tolerance: 1.0e-9,
    }
}

fn limits() -> ContingentSearchLimits {
    ContingentSearchLimits {
        maximum_sensor_depth: 2,
        maximum_search_nodes: 1_000,
    }
}

fn base_fields(report: &civdyn_active_sensing::contingent::ContingentSensingReport) -> Vec<String> {
    vec![
        "summary".to_owned(),                                     // 0
        report.scenario_id.clone(),                               // 1
        report.source_digest.clone(),                             // 2
        report.status.id().to_owned(),                            // 3
        report.recommended.to_string(),                           // 4
        report.baseline.common_feasible_policy_count.to_string(), // 5
        report
            .baseline
            .selected_policy_id
            .clone()
            .unwrap_or_default(), // 6
        report
            .baseline
            .worst_case_regret
            .map_or_else(String::new, |v| format!("{v:.6}")), // 7
        report
            .worst_case_post_sensing_regret
            .map_or_else(String::new, |v| format!("{v:.6}")), // 8
        report
            .worst_case_regret_reduction
            .map_or_else(String::new, |v| format!("{v:.6}")), // 9
        report.resolves_no_sensing_infeasibility.to_string(),     // 10
        report.maximum_sensor_depth.to_string(),                  // 11
        report.search_nodes_evaluated.to_string(),                // 12
        report.selected_tree_depth.to_string(),                   // 13
        format!("{:.6}", report.maximum_path_energy_kwh),         // 14
        report.maximum_path_latency_ticks.to_string(),            // 15
        format!("{:.6}", report.maximum_path_penalty_service_units), // 16
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(), // 17..21
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(), // 22..26
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(), // 27..31
        String::new(),
        String::new(),
        String::new(),
        String::new(), // 32..35
    ]
}

fn node_fields(
    report: &civdyn_active_sensing::contingent::ContingentSensingReport,
    receipt: &civdyn_active_sensing::contingent::ContingentNodeReceipt,
) -> Vec<String> {
    let mut fields = base_fields(report);
    fields[0] = "node".to_owned();
    fields[17] = receipt.observation_history.join(">");
    fields[18] = receipt.node_kind.clone();
    fields[19] = receipt.active_world_ids.len().to_string();
    fields[20] = receipt.active_world_ids.join(";");
    fields[21] = receipt.sensor_id.clone().unwrap_or_default();
    fields[22] = receipt.action_id.clone().unwrap_or_default();
    fields[23] = receipt.protocol_digest.clone().unwrap_or_default();
    fields[24] = format!("{:.6}", receipt.path_energy_kwh);
    fields[25] = receipt.path_latency_ticks.to_string();
    fields[26] = format!("{:.6}", receipt.maximum_path_penalty_service_units);
    fields
}

fn prediction_fields(
    report: &civdyn_active_sensing::contingent::ContingentSensingReport,
    receipt: &ContingentPredictionReceipt,
) -> Vec<String> {
    let mut fields = base_fields(report);
    fields[0] = "prediction".to_owned();
    fields[17] = receipt.observation_history.join(">");
    fields[21] = receipt.sensor_id.clone();
    fields[27] = receipt.world_id.clone();
    fields[28] = receipt.observation_id.clone();
    fields[29] = receipt.prediction_receipt_digest.clone();
    fields[30] = receipt
        .missing_cause
        .as_ref()
        .map_or_else(|| "none".to_owned(), |cause| cause.id().to_owned());
    fields
}

fn leaf_fields(
    report: &civdyn_active_sensing::contingent::ContingentSensingReport,
    receipt: &civdyn_active_sensing::contingent::ContingentLeafReceipt,
) -> Vec<String> {
    let mut fields = base_fields(report);
    fields[0] = "leaf".to_owned();
    fields[17] = receipt.observation_history.join(">");
    fields[22] = receipt.selected_action_id.clone();
    fields[24] = format!("{:.6}", receipt.path_energy_kwh);
    fields[25] = receipt.path_latency_ticks.to_string();
    fields[27] = receipt.world_id.clone();
    fields[31] = format!("{:.6}", receipt.gross_policy_service_units);
    fields[32] = format!("{:.6}", receipt.oracle_service_units);
    fields[33] = format!("{:.6}", receipt.sensing_penalty_service_units);
    fields[34] = format!("{:.6}", receipt.net_policy_service_units);
    fields[35] = format!("{:.6}", receipt.regret_service_units);
    fields
}

fn emit_candidate_tree(scenario_id: &str, node: &CandidatePolicyNode, history: &[String]) {
    match node {
        CandidatePolicyNode::StopAndAct { action_id } => {
            println!(
                "{},{},stop_and_act,,,{}",
                csv_field(scenario_id),
                csv_field(&history.join(">")),
                csv_field(action_id)
            );
        }
        CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches,
        } => {
            println!(
                "{},{},request_sensor,{},,",
                csv_field(scenario_id),
                csv_field(&history.join(">")),
                csv_field(sensor_id)
            );
            for branch in branches {
                let mut next_history = history.to_vec();
                next_history.push(branch.observation_id.clone());
                println!(
                    "{},{},observation_branch,{},{},",
                    csv_field(scenario_id),
                    csv_field(&next_history.join(">")),
                    csv_field(sensor_id),
                    csv_field(&branch.observation_id)
                );
                emit_candidate_tree(scenario_id, &branch.next, &next_history);
            }
        }
    }
}

fn fixed_sequence_reference_rows(
    report: &civdyn_active_sensing::contingent::ContingentSensingReport,
) -> Result<Vec<Vec<String>>, String> {
    let catalog = sensors();
    let screen = catalog.iter()
        .find(|item| item.sensor.sensor_id == "screen")
        .ok_or_else(|| "fixed sequence lacks screen sensor".to_owned())?;
    let inspection = catalog.iter()
        .find(|item| item.sensor.sensor_id == "independent_inspection")
        .ok_or_else(|| "fixed sequence lacks inspection sensor".to_owned())?;

    let mut path_rows = Vec::new();
    let mut worst_regret = 0.0_f64;
    let mut maximum_penalty = 0.0_f64;
    let mut maximum_energy = 0.0_f64;
    let mut maximum_latency = 0_u32;

    for world in worlds() {
        let screen_prediction = screen.sensor.predictions.iter()
            .find(|prediction| prediction.world_id == world.world_id)
            .ok_or_else(|| format!("screen prediction missing for {}", world.world_id))?;
        let inspection_prediction = inspection.sensor.predictions.iter()
            .find(|prediction| prediction.world_id == world.world_id)
            .ok_or_else(|| format!("inspection prediction missing for {}", world.world_id))?;

        let action_id = match (
            screen_prediction.observation_id.as_str(),
            inspection_prediction.observation_id.as_str(),
        ) {
            ("healthy", "healthy") => "defer",
            ("missing", "degraded") => "repair",
            ("missing", "healthy") => "defer",
            _ => return Err(format!(
                "unregistered fixed-sequence observation pair for {}: {} / {}",
                world.world_id,
                screen_prediction.observation_id,
                inspection_prediction.observation_id
            )),
        };
        let policy = world.policies.iter()
            .find(|policy| policy.policy_id == action_id)
            .ok_or_else(|| format!("fixed action {action_id} missing for {}", world.world_id))?;
        let oracle = world.policies.iter()
            .filter(|candidate| candidate.service_units + 1.0e-9 >= world.service_floor)
            .max_by(|left, right| left.service_units.total_cmp(&right.service_units)
                .then_with(|| right.policy_id.cmp(&left.policy_id)))
            .ok_or_else(|| format!("no oracle action meets floor in {}", world.world_id))?;

        let energy_kwh = screen.sensor.energy_cost_kwh + inspection.sensor.energy_cost_kwh;
        let latency_ticks = screen.sensor.latency_ticks + inspection.sensor.latency_ticks;
        let energy_penalty = screen.sensor.energy_cost_kwh * screen.sensor.service_units_per_kwh
            + inspection.sensor.energy_cost_kwh * inspection.sensor.service_units_per_kwh;
        let latency_penalty = f64::from(screen.sensor.latency_ticks)
            * screen_prediction.latency_service_loss_per_tick
            + f64::from(inspection.sensor.latency_ticks)
            * inspection_prediction.latency_service_loss_per_tick;
        let penalty = energy_penalty + latency_penalty;
        let net = policy.service_units - penalty;
        if net + 1.0e-9 < world.service_floor {
            return Err(format!("fixed sequence violates service floor in {}", world.world_id));
        }
        let regret = (oracle.service_units - net).max(0.0);
        worst_regret = worst_regret.max(regret);
        maximum_penalty = maximum_penalty.max(penalty);
        maximum_energy = maximum_energy.max(energy_kwh);
        maximum_latency = maximum_latency.max(latency_ticks);

        let mut row = base_fields(report);
        row[0] = "fixed_sequence_path".to_owned();
        row[3] = "reference_baseline".to_owned();
        row[4] = "false".to_owned();
        row[17] = format!(
            "screen:{}>independent_inspection:{}",
            screen_prediction.observation_id,
            inspection_prediction.observation_id
        );
        row[18] = "execute_both_sensors".to_owned();
        row[19] = "1".to_owned();
        row[20] = world.world_id.clone();
        row[21] = "screen+independent_inspection".to_owned();
        row[22] = action_id.to_owned();
        row[23] = format!(
            "{}+{}",
            screen.sensor.protocol_digest,
            inspection.sensor.protocol_digest
        );
        row[24] = format!("{energy_kwh:.6}");
        row[25] = latency_ticks.to_string();
        row[26] = format!("{penalty:.6}");
        row[27] = world.world_id.clone();
        row[28] = format!(
            "{}>{}",
            screen_prediction.observation_id,
            inspection_prediction.observation_id
        );
        row[29] = format!(
            "{}+{}",
            screen_prediction.prediction_receipt_digest,
            inspection_prediction.prediction_receipt_digest
        );
        row[30] = screen_prediction.missing_cause.as_ref()
            .map_or_else(|| "none".to_owned(), |cause| cause.id().to_owned());
        row[31] = format!("{:.6}", policy.service_units);
        row[32] = format!("{:.6}", oracle.service_units);
        row[33] = format!("{penalty:.6}");
        row[34] = format!("{net:.6}");
        row[35] = format!("{regret:.6}");
        path_rows.push(row);
    }

    let mut summary = base_fields(report);
    summary[0] = "fixed_sequence_summary".to_owned();
    summary[3] = "reference_baseline".to_owned();
    summary[4] = "false".to_owned();
    summary[8] = format!("{worst_regret:.6}");
    summary[9] = report.baseline.worst_case_regret
        .map_or_else(String::new, |baseline| format!("{:.6}", baseline - worst_regret));
    summary[13] = "2".to_owned();
    summary[14] = format!("{maximum_energy:.6}");
    summary[15] = maximum_latency.to_string();
    summary[16] = format!("{maximum_penalty:.6}");
    let mut rows = vec![summary];
    rows.extend(path_rows);
    Ok(rows)
}

fn false_reassurance_config() -> ActiveSensingConfig {
    ActiveSensingConfig {
        scenario_id: "contingent-false-reassurance-corroboration-v1".to_owned(),
        source_digest: "sha256-false-reassurance-corroboration-v1".to_owned(),
        service_unit: "service_units".to_owned(),
        energy_unit: "kWh".to_owned(),
        maximum_energy_kwh: 0.25,
        maximum_latency_ticks: 2,
        maximum_penalty_service_units: 1.0,
        tie_tolerance: 1.0e-9,
    }
}

fn false_reassurance_worlds() -> Vec<DecisionWorld> {
    vec![
        world("healthy", 100.0, 85.0),
        world("degraded", 40.0, 80.0),
        world("healthy_sensor_fault", 100.0, 85.0),
        world("degraded_stuck_high", 40.0, 80.0),
    ]
}

fn false_reassurance_sensors() -> Vec<ContingentSensor> {
    vec![
        sensor(
            "screen",
            0.1,
            2.0,
            1,
            None,
            vec![
                ("healthy", "healthy", None, 0.1),
                (
                    "degraded",
                    "missing",
                    Some(MissingCause::CommonCausePowerLoss),
                    0.1,
                ),
                (
                    "healthy_sensor_fault",
                    "missing",
                    Some(MissingCause::TelemetryNetworkFault),
                    0.1,
                ),
                ("degraded_stuck_high", "healthy", None, 0.1),
            ],
        ),
        sensor(
            "stuck_high_confirmation",
            0.1,
            2.0,
            1,
            Some("healthy"),
            vec![
                ("healthy", "verified_healthy", None, 0.1),
                ("degraded", "not_requested", None, 0.1),
                ("healthy_sensor_fault", "not_requested", None, 0.1),
                ("degraded_stuck_high", "verified_degraded", None, 0.1),
            ],
        ),
        sensor(
            "independent_missing_branch_inspection",
            0.1,
            2.0,
            1,
            Some("missing"),
            vec![
                ("healthy", "not_requested", None, 0.1),
                ("degraded", "verified_degraded", None, 0.1),
                ("healthy_sensor_fault", "verified_healthy", None, 0.1),
                ("degraded_stuck_high", "not_requested", None, 0.1),
            ],
        ),
    ]
}

fn main() -> Result<(), String> {
    let reports = vec![
        assess_contingent_sensing(&config(), &limits(), &worlds(), &sensors())?,
        assess_contingent_sensing(
            &false_reassurance_config(),
            &limits(),
            &false_reassurance_worlds(),
            &false_reassurance_sensors(),
        )?,
    ];

    let candidate_only = std::env::args().nth(1).as_deref() == Some("--candidate-policy-tree");
    if candidate_only {
        println!("scenario_id,observation_history,node_type,sensor_id,observation_id,action_id");
        for report in &reports {
            if let Some(tree) = &report.candidate_policy_tree {
                emit_candidate_tree(&report.scenario_id, tree, &[]);
            }
        }
        return Ok(());
    }

    println!(
        "record_type,scenario_id,source_digest,status,recommended,baseline_common_feasible_policy_count,baseline_policy_id,baseline_worst_regret,worst_case_post_sensing_regret,regret_reduction,resolves_no_sensing_infeasibility,maximum_sensor_depth,search_nodes_evaluated,selected_tree_depth,maximum_path_energy_kwh,maximum_path_latency_ticks,maximum_path_penalty_service_units,observation_history,node_kind,active_world_count,active_world_ids,sensor_id,action_id,protocol_digest,path_energy_kwh,path_latency_ticks,node_maximum_path_penalty_service_units,world_id,observation_id,prediction_receipt_digest,evaluator_missing_cause,gross_policy_service_units,oracle_service_units,sensing_penalty_service_units,net_policy_service_units,regret_service_units"
    );

    for report in &reports {
        emit(base_fields(report));
        if report.scenario_id == "contingent-three-world-screen-and-inspect-v1" {
            for row in fixed_sequence_reference_rows(report)? {
                emit(row);
            }
        }
        for receipt in &report.evaluator_node_receipts {
            emit(node_fields(report, receipt));
        }
        for receipt in &report.evaluator_prediction_receipts {
            emit(prediction_fields(report, receipt));
        }
        for receipt in &report.evaluator_leaf_receipts {
            emit(leaf_fields(report, receipt));
        }
    }
    Ok(())
}
