// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_active_sensing::{
    ActiveSensingConfig, CandidatePolicyDecision, DecisionWorld, MissingCause, PolicyOutcome,
    SensorAction, SensorPrediction, assess_active_sensing, candidate_policy_map,
    evaluator_prediction_audit,
};

const SCENARIO_ID: &str = "three-world-sensor-dual-control-v1";
const SOURCE_DIGEST: &str = "sha256-three-world-sensor-source-v1";
const CSV_COLUMNS: usize = 27;

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
        CSV_COLUMNS,
        "active-sensing receipt schema mismatch"
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

fn prediction(
    sensor_id: &str,
    world_id: &str,
    observation_id: &str,
    missing_cause: Option<MissingCause>,
    latency_service_loss_per_tick: f64,
) -> SensorPrediction {
    SensorPrediction {
        world_id: world_id.to_owned(),
        world_digest: format!("sha256-{world_id}"),
        observation_id: observation_id.to_owned(),
        missing_cause,
        latency_service_loss_per_tick,
        prediction_receipt_digest: format!("sha256-{sensor_id}-{world_id}-prediction"),
    }
}

fn sensor(
    id: &str,
    energy_cost_kwh: f64,
    service_units_per_kwh: f64,
    latency_ticks: u32,
    predictions: Vec<SensorPrediction>,
) -> SensorAction {
    SensorAction {
        sensor_id: id.to_owned(),
        protocol_digest: format!("sha256-{id}-protocol"),
        energy_cost_kwh,
        service_units_per_kwh,
        latency_ticks,
        predictions,
    }
}

fn fixture() -> (ActiveSensingConfig, Vec<DecisionWorld>, Vec<SensorAction>) {
    let config = ActiveSensingConfig {
        scenario_id: SCENARIO_ID.to_owned(),
        source_digest: SOURCE_DIGEST.to_owned(),
        service_unit: "service_units".to_owned(),
        energy_unit: "kWh".to_owned(),
        maximum_energy_kwh: 1.0,
        maximum_latency_ticks: 2,
        maximum_penalty_service_units: 8.0,
        tie_tolerance: 1.0e-9,
    };

    let worlds = vec![
        world("healthy", 100.0, 85.0),
        world("degraded", 0.0, 80.0),
        world("healthy_sensor_fault", 100.0, 85.0),
    ];

    let sensors = vec![
        sensor(
            "remote_telemetry",
            0.5,
            2.0,
            2,
            vec![
                prediction("remote_telemetry", "healthy", "healthy", None, 1.0),
                prediction(
                    "remote_telemetry",
                    "degraded",
                    "missing",
                    Some(MissingCause::CommonCausePowerLoss),
                    3.0,
                ),
                prediction(
                    "remote_telemetry",
                    "healthy_sensor_fault",
                    "missing",
                    Some(MissingCause::TelemetryNetworkFault),
                    2.0,
                ),
            ],
        ),
        sensor(
            "independent_field_inspection",
            0.5,
            2.0,
            1,
            vec![
                prediction(
                    "independent_field_inspection",
                    "healthy",
                    "healthy",
                    None,
                    1.0,
                ),
                prediction(
                    "independent_field_inspection",
                    "degraded",
                    "degraded",
                    None,
                    3.0,
                ),
                prediction(
                    "independent_field_inspection",
                    "healthy_sensor_fault",
                    "healthy",
                    None,
                    1.0,
                ),
            ],
        ),
        sensor(
            "uninformative_status_check",
            0.0,
            2.0,
            0,
            vec![
                prediction(
                    "uninformative_status_check",
                    "healthy",
                    "status_unknown",
                    None,
                    0.0,
                ),
                prediction(
                    "uninformative_status_check",
                    "degraded",
                    "status_unknown",
                    None,
                    0.0,
                ),
                prediction(
                    "uninformative_status_check",
                    "healthy_sensor_fault",
                    "status_unknown",
                    None,
                    0.0,
                ),
            ],
        ),
        sensor(
            "over_budget_lab_test",
            2.0,
            2.0,
            1,
            vec![
                prediction("over_budget_lab_test", "healthy", "healthy", None, 1.0),
                prediction("over_budget_lab_test", "degraded", "degraded", None, 3.0),
                prediction(
                    "over_budget_lab_test",
                    "healthy_sensor_fault",
                    "healthy",
                    None,
                    1.0,
                ),
            ],
        ),
    ];

    (config, worlds, sensors)
}

fn feasibility_lift_fixture() -> (ActiveSensingConfig, Vec<DecisionWorld>, Vec<SensorAction>) {
    let config = ActiveSensingConfig {
        scenario_id: "two-world-feasibility-lift-v1".to_owned(),
        source_digest: "sha256-two-world-feasibility-lift-source-v1".to_owned(),
        service_unit: "service_units".to_owned(),
        energy_unit: "kWh".to_owned(),
        maximum_energy_kwh: 1.0,
        maximum_latency_ticks: 1,
        maximum_penalty_service_units: 2.0,
        tie_tolerance: 1.0e-9,
    };
    let worlds = vec![
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
    ];
    let sensors = vec![
        sensor(
            "decision_separating_inspection",
            0.25,
            2.0,
            0,
            vec![
                prediction(
                    "decision_separating_inspection",
                    "world_a",
                    "world_a_detected",
                    None,
                    0.0,
                ),
                prediction(
                    "decision_separating_inspection",
                    "world_b",
                    "world_b_detected",
                    None,
                    0.0,
                ),
            ],
        ),
        sensor(
            "uninformative_feasibility_check",
            0.0,
            2.0,
            0,
            vec![
                prediction(
                    "uninformative_feasibility_check",
                    "world_a",
                    "unknown",
                    None,
                    0.0,
                ),
                prediction(
                    "uninformative_feasibility_check",
                    "world_b",
                    "unknown",
                    None,
                    0.0,
                ),
            ],
        ),
    ];
    (config, worlds, sensors)
}

fn summary_fields(
    report: &civdyn_active_sensing::ActiveSensingReport,
    assessment: &civdyn_active_sensing::SensorAssessment,
) -> Vec<String> {
    vec![
        "sensor_summary".to_owned(),
        report.scenario_id.clone(),
        assessment.sensor_id.clone(),
        assessment.status.id().to_owned(),
        assessment.affordable.to_string(),
        (report.recommended_sensor_id.as_deref() == Some(assessment.sensor_id.as_str()))
            .to_string(),
        report
            .baseline
            .selected_policy_id
            .clone()
            .unwrap_or_default(),
        report
            .baseline
            .worst_case_regret
            .map_or_else(String::new, |v| format!("{v:.6}")),
        assessment.protocol_digest.clone(),
        format!("{:.6}", assessment.energy_cost_kwh),
        format!("{:.6}", assessment.service_units_per_kwh),
        format!("{:.6}", assessment.energy_cost_service_units),
        assessment.latency_ticks.to_string(),
        format!("{:.6}", assessment.maximum_latency_service_loss),
        format!("{:.6}", assessment.maximum_total_penalty_service_units),
        assessment
            .worst_case_post_sensing_regret
            .map_or_else(String::new, |v| format!("{v:.6}")),
        assessment
            .worst_case_regret_reduction
            .map_or_else(String::new, |v| format!("{v:.6}")),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        assessment.resolves_no_sensing_infeasibility.to_string(),
    ]
}

fn partition_fields(
    report: &civdyn_active_sensing::ActiveSensingReport,
    assessment: &civdyn_active_sensing::SensorAssessment,
    partition: &civdyn_active_sensing::ObservationPartition,
) -> Vec<String> {
    let mut fields = summary_fields(report, assessment);
    fields[0] = "observation_partition".to_owned();
    fields[17] = partition.observation_id.clone();
    fields[18] = partition.world_ids.len().to_string();
    fields[19] = partition.world_ids.join(";");
    fields[20] = partition.selected_policy_id.clone().unwrap_or_default();
    fields[21] = partition
        .worst_case_regret
        .map_or_else(String::new, |v| format!("{v:.6}"));
    fields
}

fn audit_fields(
    report: &civdyn_active_sensing::ActiveSensingReport,
    assessment: &civdyn_active_sensing::SensorAssessment,
    receipt: &civdyn_active_sensing::PredictionAuditReceipt,
) -> Vec<String> {
    let mut fields = summary_fields(report, assessment);
    fields[0] = "evaluator_prediction_audit".to_owned();
    fields[17] = receipt.observation_id.clone();
    fields[22] = receipt.world_id.clone();
    fields[23] = receipt.world_digest.clone();
    fields[24] = receipt.prediction_receipt_digest.clone();
    fields[25] = receipt
        .missing_cause
        .as_ref()
        .map_or_else(|| "none".to_owned(), |cause| cause.id().to_owned());
    fields
}

fn emit_candidate(decision: &CandidatePolicyDecision) {
    let fields = [
        decision.scenario_id.as_str(),
        decision.sensor_id.as_str(),
        decision.observation_id.as_str(),
        decision.action_id.as_str(),
    ];
    println!(
        "{}",
        fields
            .iter()
            .map(|field| csv_field(field))
            .collect::<Vec<_>>()
            .join(",")
    );
}

fn main() -> Result<(), String> {
    let candidate_only = std::env::args().nth(1).as_deref() == Some("--candidate-policy-map");
    if candidate_only {
        println!("scenario_id,sensor_id,observation_id,action_id");
        let (config, worlds, sensors) = fixture();
        let report = assess_active_sensing(&config, &worlds, &sensors)?;
        for decision in candidate_policy_map(&report) {
            emit_candidate(&decision);
        }
        let (lift_config, lift_worlds, lift_sensors) = feasibility_lift_fixture();
        let lift_report = assess_active_sensing(&lift_config, &lift_worlds, &lift_sensors)?;
        for decision in candidate_policy_map(&lift_report) {
            emit_candidate(&decision);
        }
        return Ok(());
    }
    println!(
        "record_type,scenario_id,sensor_id,status,affordable,recommended,baseline_policy_id,baseline_worst_regret,protocol_digest,energy_cost_kwh,service_units_per_kwh,energy_cost_service_units,latency_ticks,maximum_latency_service_loss,maximum_total_penalty_service_units,worst_case_post_sensing_regret,regret_reduction,observation_id,partition_world_count,world_ids,selected_policy_id,partition_worst_regret,receipt_world_id,world_digest,prediction_receipt_digest,evaluator_missing_cause,resolves_no_sensing_infeasibility"
    );
    let (config, worlds, sensors) = fixture();
    emit_report(&config, &worlds, &sensors)?;
    let (lift_config, lift_worlds, lift_sensors) = feasibility_lift_fixture();
    emit_report(&lift_config, &lift_worlds, &lift_sensors)?;
    Ok(())
}

fn emit_report(
    config: &ActiveSensingConfig,
    worlds: &[DecisionWorld],
    sensors: &[SensorAction],
) -> Result<(), String> {
    let report = assess_active_sensing(config, worlds, sensors)?;
    let evaluator_audit = evaluator_prediction_audit(sensors)?;
    for assessment in &report.sensors {
        emit(summary_fields(&report, assessment));
        for partition in &assessment.partitions {
            emit(partition_fields(&report, assessment, partition));
        }
    }
    for receipt in &evaluator_audit {
        let assessment = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == receipt.sensor_id)
            .expect("audit refers to declared sensor");
        emit(audit_fields(&report, assessment, receipt));
    }
    Ok(())
}
