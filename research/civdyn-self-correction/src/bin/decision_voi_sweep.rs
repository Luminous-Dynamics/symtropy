// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::decision_criticality::OrderedFloatValue;
use civdyn_self_correction::decision_voi::{
    assess_decision_value_of_information, DecisionModelWorld, DecisionPolicyState,
    DecisionVoiConfig, DecisionVoiReport, ExperimentOutcomePrediction, ExperimentSpec,
};
use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::policy_sensitivity::{ModelAdequacyState, ModelWorld, ObjectiveDirection};

const CALIBRATION_DIGEST: &str = "sha256-decision-voi-calibration";
const SOURCE_DIGEST: &str = "sha256-decision-voi-source";
const CSV_COLUMNS: usize = 27;

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
        "decision VOI receipt must match the frozen schema"
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

fn build_model(
    id: &str,
    digest: &str,
    role: SplitRole,
    discrepancy: bool,
) -> Result<ModelWorld, Box<dyn std::error::Error>> {
    let registry = StatisticRegistry {
        version: "decision-voi-adequacy-v1".to_string(),
        statistics: vec![StatisticSpec {
            id: "service-model-fit".to_string(),
            construct: "essential service".to_string(),
            measurement_unit: "service_units".to_string(),
            population: "synthetic-district".to_string(),
            context: "decision-voi-known-answer".to_string(),
            source_family: "synthetic-voi-source".to_string(),
            source_digest: SOURCE_DIGEST.to_string(),
            independent_unit: IndependentUnitKind::Region,
            role,
            primary: true,
            required: true,
            minimum_draws: 5,
        }],
    };
    let evaluation_unit = format!("{id}-evaluation");
    let manifest = SplitManifest {
        id: format!("{id}-split"),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: format!("sha256-{id}-manifest"),
        calibration_units: vec![format!("{id}-calibration")],
        validation_units: if role == SplitRole::Validation {
            vec![evaluation_unit.clone()]
        } else {
            vec![format!("{id}-reserved-validation")]
        },
        challenge_units: if role == SplitRole::Challenge {
            vec![evaluation_unit.clone()]
        } else {
            Vec::new()
        },
    };
    let observation = ObservedStatistic {
        statistic_id: "service-model-fit".to_string(),
        independent_unit_id: evaluation_unit.clone(),
        measurement_unit: "service_units".to_string(),
        source_digest: SOURCE_DIGEST.to_string(),
        sample_size: 100,
        value: 5.0,
        measurement_standard_error: Some(0.1),
        observation_digest: format!("sha256-{id}-observation"),
    };
    let draw_values = if discrepancy {
        [0.0, 1.0, 2.0, 3.0, 4.0]
    } else {
        [3.0, 4.0, 5.0, 6.0, 7.0]
    };
    let draws: Vec<PredictiveDraw> = draw_values
        .iter()
        .enumerate()
        .map(|(index, value)| PredictiveDraw {
            kind: PredictiveKind::Split,
            statistic_id: "service-model-fit".to_string(),
            independent_unit_id: evaluation_unit.clone(),
            sample_size: 100,
            draw_index: index as u64,
            value: *value,
            model_digest: digest.to_string(),
            draw_set_digest: format!("sha256-{id}-draw-set"),
            calibration_data_digest: Some(CALIBRATION_DIGEST.to_string()),
        })
        .collect();
    let assessment = assess_predictive_partition(
        &registry,
        &manifest,
        &[observation],
        &draws,
        role,
        PredictiveKind::Split,
        0.9,
    )?;
    Ok(ModelWorld::from_adequacy_assessment(
        id,
        format!("family-{id}"),
        digest,
        format!("sha256-{id}-adequacy-receipt"),
        &registry,
        &manifest,
        &assessment,
    )?)
}

fn model_world(
    model: ModelWorld,
    values: [f64; 3],
    feasible: [bool; 3],
) -> Result<DecisionModelWorld, Box<dyn std::error::Error>> {
    let policy_ids = ["expand", "maintain", "conserve"];
    let model_id = model.id().to_string();
    let policies = policy_ids
        .iter()
        .enumerate()
        .map(|(index, policy_id)| {
            Ok(DecisionPolicyState {
                policy_id: (*policy_id).to_string(),
                objective_value: OrderedFloatValue::new(values[index])?,
                source_digest: SOURCE_DIGEST.to_string(),
                outcome_receipt_digest: format!("sha256-{model_id}-{policy_id}-policy"),
                feasible: feasible[index],
                violated_constraints: if feasible[index] {
                    Vec::new()
                } else {
                    vec!["water-reserve-floor".to_string()]
                },
            })
        })
        .collect::<Result<Vec<_>, civdyn_self_correction::decision_criticality::DecisionCriticalityError>>()?;
    Ok(DecisionModelWorld { model, policies })
}

fn worlds() -> Result<Vec<DecisionModelWorld>, Box<dyn std::error::Error>> {
    Ok(vec![
        model_world(
            build_model(
                "source",
                "sha256-decision-voi-source-model",
                SplitRole::Validation,
                false,
            )?,
            [100.0, 90.0, 70.0],
            [true, true, true],
        )?,
        model_world(
            build_model(
                "alternative",
                "sha256-decision-voi-alternative-model",
                SplitRole::Challenge,
                true,
            )?,
            [120.0, 80.0, 100.0],
            [false, true, true],
        )?,
    ])
}

fn experiment(
    id: &str,
    cost: f64,
    source_outcome: &str,
    alternative_outcome: &str,
) -> ExperimentSpec {
    ExperimentSpec {
        id: id.to_string(),
        protocol_digest: format!("sha256-{id}-protocol"),
        cost,
        predictions: vec![
            ExperimentOutcomePrediction {
                model_id: "source".to_string(),
                model_digest: "sha256-decision-voi-source-model".to_string(),
                outcome_id: source_outcome.to_string(),
                prediction_receipt_digest: format!("sha256-{id}-source-prediction"),
            },
            ExperimentOutcomePrediction {
                model_id: "alternative".to_string(),
                model_digest: "sha256-decision-voi-alternative-model".to_string(),
                outcome_id: alternative_outcome.to_string(),
                prediction_receipt_digest: format!("sha256-{id}-alternative-prediction"),
            },
        ],
    }
}

fn emit_report(report: &DecisionVoiReport) {
    for experiment in &report.experiments {
        for prediction in &experiment.predictions {
            let partition = experiment
                .partitions
                .iter()
                .find(|partition| partition.outcome_id == prediction.outcome_id)
                .expect("every prediction belongs to one outcome partition");
            let model = report
                .model_receipts
                .iter()
                .find(|receipt| receipt.model_id == prediction.model_id)
                .expect("every prediction has a registered model receipt");
            emit(vec![
                experiment.experiment_id.clone(),
                experiment.protocol_digest.clone(),
                format!("{:.6}", experiment.cost.get()),
                report.cost_unit.clone(),
                format!("{:.6}", report.maximum_experiment_cost.get()),
                experiment.status.name().to_string(),
                experiment.affordable.to_string(),
                report.objective_id.clone(),
                report.measurement_unit.clone(),
                match report.direction {
                    ObjectiveDirection::Maximize => "maximize",
                    ObjectiveDirection::Minimize => "minimize",
                }
                .to_string(),
                report.current_minimax_decision.optimal_policy_ids.join(";"),
                report
                    .current_minimax_decision
                    .worst_case_regret
                    .map(|value| format!("{:.6}", value.get()))
                    .unwrap_or_default(),
                report.recommended_experiment_id.clone().unwrap_or_default(),
                experiment.model_pairs_separated.to_string(),
                experiment.model_pairs_total.to_string(),
                experiment
                    .worst_case_post_experiment_regret
                    .map(|value| format!("{:.6}", value.get()))
                    .unwrap_or_default(),
                experiment
                    .worst_case_regret_reduction
                    .map(|value| format!("{:.6}", value.get()))
                    .unwrap_or_default(),
                prediction.model_id.clone(),
                prediction.model_digest.clone(),
                model.adequacy_state.name().to_string(),
                model.adequacy_role.name().to_string(),
                model.adequacy_receipt_digest.clone(),
                prediction.outcome_id.clone(),
                prediction.prediction_receipt_digest.clone(),
                partition.model_ids.join(";"),
                partition.minimax_optimal_policy_ids.join(";"),
                partition
                    .minimax_regret
                    .map(|value| format!("{:.6}", value.get()))
                    .unwrap_or_default(),
            ]);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let worlds = worlds()?;
    let report = assess_decision_value_of_information(
        &worlds,
        &[
            experiment("discriminate", 5.0, "no-breach", "water-breach"),
            experiment("uninformative", 2.0, "same-result", "same-result"),
            experiment("over-budget", 25.0, "no-breach", "water-breach"),
        ],
        &DecisionVoiConfig {
            objective_id: "essential-service-coverage".to_string(),
            measurement_unit: "service_units".to_string(),
            direction: ObjectiveDirection::Maximize,
            cost_unit: "analyst_hours".to_string(),
            maximum_experiment_cost: 20.0,
            tie_tolerance: 0.0,
        },
    )?;

    println!(
        "experiment_id,protocol_digest,cost,cost_unit,maximum_experiment_cost,status,affordable,objective_id,measurement_unit,direction,current_minimax_policy_ids,current_minimax_regret,recommended_experiment_id,model_pairs_separated,model_pairs_total,worst_case_post_experiment_regret,worst_case_regret_reduction,model_id,model_digest,model_adequacy_state,model_adequacy_role,model_adequacy_receipt_digest,predicted_outcome_id,prediction_receipt_digest,partition_model_ids,partition_minimax_policy_ids,partition_minimax_regret"
    );
    emit_report(&report);
    Ok(())
}
