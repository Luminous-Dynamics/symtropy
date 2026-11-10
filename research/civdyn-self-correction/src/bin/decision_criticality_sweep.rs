// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::decision_criticality::{
    assess_decision_criticality, DecisionCriticalityConfig, DecisionCriticalityReport,
    DecisionPolicyOutcome, DecisionWorldEvidence, OrderedFloatValue,
};
use civdyn_self_correction::model_adequacy::{
    assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
    PredictiveKind, SplitManifest, SplitRole, StatisticRegistry, StatisticSpec,
};
use civdyn_self_correction::policy_sensitivity::{
    ModelAdequacyState, ModelWorld, ObjectiveDirection,
};

const CALIBRATION_DIGEST: &str = "sha256-decision-criticality-calibration";
const SOURCE_DIGEST: &str = "sha256-decision-criticality-source";
const CSV_COLUMNS: usize = 28;

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
        "decision criticality receipt must match the frozen schema"
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

fn qualified_model(
    id: &str,
    model_digest: &str,
    role: SplitRole,
    discrepancy: bool,
) -> Result<ModelWorld, Box<dyn std::error::Error>> {
    let registry = StatisticRegistry {
        version: "decision-criticality-adequacy-v1".to_string(),
        statistics: vec![StatisticSpec {
            id: "service-model-fit".to_string(),
            construct: "essential service state".to_string(),
            measurement_unit: "service_units".to_string(),
            population: "synthetic-region".to_string(),
            context: "decision-criticality-known-answer".to_string(),
            source_family: "synthetic-policy-source".to_string(),
            source_digest: SOURCE_DIGEST.to_string(),
            independent_unit: IndependentUnitKind::Region,
            role,
            primary: true,
            required: true,
            minimum_draws: 5,
        }],
    };
    let manifest = SplitManifest {
        id: format!("{id}-split"),
        independent_unit: IndependentUnitKind::Region,
        calibration_data_digest: CALIBRATION_DIGEST.to_string(),
        manifest_digest: format!("sha256-{id}-manifest"),
        calibration_units: vec![format!("{id}-calibration")],
        validation_units: if role == SplitRole::Validation {
            vec![format!("{id}-evaluation")]
        } else {
            vec![format!("{id}-reserved-validation")]
        },
        challenge_units: if role == SplitRole::Challenge {
            vec![format!("{id}-evaluation")]
        } else {
            Vec::new()
        },
    };
    let evaluation_unit = format!("{id}-evaluation");
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
            model_digest: model_digest.to_string(),
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
        model_digest,
        format!("sha256-{id}-adequacy-receipt"),
        &registry,
        &manifest,
        &assessment,
    )?)
}

fn decision_world(
    model: ModelWorld,
    values: [f64; 3],
    feasible: [bool; 3],
) -> Result<DecisionWorldEvidence, Box<dyn std::error::Error>> {
    let policy_ids = ["expand", "maintain", "conserve"];
    let model_id = model.id().to_string();
    let outcomes = policy_ids
        .iter()
        .enumerate()
        .map(|(index, policy_id)| {
            Ok(DecisionPolicyOutcome {
                policy_id: (*policy_id).to_string(),
                objective_value: OrderedFloatValue::new(values[index])?,
                source_digest: SOURCE_DIGEST.to_string(),
                outcome_receipt_digest: format!("sha256-{model_id}-{policy_id}-outcome"),
                feasible: feasible[index],
                violated_constraints: if feasible[index] {
                    Vec::new()
                } else {
                    vec!["water-reserve-floor".to_string()]
                },
            })
        })
        .collect::<Result<Vec<_>, civdyn_self_correction::decision_criticality::DecisionCriticalityError>>()?;
    Ok(DecisionWorldEvidence { model, outcomes })
}

fn ids(values: &[String]) -> String {
    values.join(";")
}

fn emit_report(report: &DecisionCriticalityReport) {
    for world in &report.worlds {
        for consequence in &world.consequences {
            emit(vec![
                world.reference_model_id.clone(),
                world.reference_model_digest.clone(),
                world.reference_adequacy_receipt_digest.clone(),
                world.alternative_model_id.clone(),
                world.alternative_model_digest.clone(),
                world.alternative_adequacy_receipt_digest.clone(),
                world.alternative_adequacy.name().to_string(),
                world.objective_id.clone(),
                world.measurement_unit.clone(),
                match world.direction {
                    ObjectiveDirection::Maximize => "maximize",
                    ObjectiveDirection::Minimize => "minimize",
                }
                .to_string(),
                format!("{:.6}", world.materiality_threshold.get()),
                ids(&world.reference_optimal_policy_ids),
                ids(&world.alternative_optimal_policy_ids),
                world.impact.name().to_string(),
                world.impact.is_decision_critical().to_string(),
                consequence.policy_id.clone(),
                format!("{:.6}", consequence.reference_value.get()),
                format!("{:.6}", consequence.alternative_value.get()),
                consequence.reference_feasible.to_string(),
                consequence.alternative_feasible.to_string(),
                consequence
                    .regret
                    .map(|value| format!("{:.6}", value.get()))
                    .unwrap_or_default(),
                consequence.feasibility_reversed.to_string(),
                consequence.reference_source_digest.clone(),
                consequence.alternative_source_digest.clone(),
                consequence.reference_outcome_receipt_digest.clone(),
                consequence.alternative_outcome_receipt_digest.clone(),
                consequence.reference_violations.join(";"),
                consequence.alternative_violations.join(";"),
            ]);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reference = decision_world(
        qualified_model(
            "reference",
            "sha256-decision-reference-model",
            SplitRole::Validation,
            false,
        )?,
        [100.0, 90.0, 80.0],
        [true, true, true],
    )?;

    let stable = decision_world(
        qualified_model(
            "stable-challenge",
            "sha256-decision-stable-model",
            SplitRole::Challenge,
            false,
        )?,
        [99.0, 91.0, 79.0],
        [true, true, true],
    )?;
    let ranking_reversal = decision_world(
        qualified_model(
            "ranking-reversal",
            "sha256-decision-ranking-model",
            SplitRole::Challenge,
            true,
        )?,
        [70.0, 95.0, 80.0],
        [true, true, true],
    )?;
    let feasibility_reversal = decision_world(
        qualified_model(
            "feasibility-reversal",
            "sha256-decision-feasibility-model",
            SplitRole::Challenge,
            true,
        )?,
        [120.0, 90.0, 80.0],
        [false, true, true],
    )?;
    let not_assessed = decision_world(
        ModelWorld::out_of_domain(
            "out-of-domain",
            "family-out-of-domain",
            "sha256-decision-out-of-domain-model",
            "sha256-out-of-domain-receipt",
            "challenge outside registered applicability domain",
        )?,
        [100.0, 90.0, 80.0],
        [true, true, true],
    )?;

    let report = assess_decision_criticality(
        &reference,
        &[stable, ranking_reversal, feasibility_reversal, not_assessed],
        &DecisionCriticalityConfig {
            objective_id: "essential-service-coverage".to_string(),
            measurement_unit: "service_units".to_string(),
            direction: ObjectiveDirection::Maximize,
            materiality_threshold: 10.0,
            tie_tolerance: 0.0,
        },
    )?;

    println!(
        "reference_model_id,reference_model_digest,reference_adequacy_receipt_digest,alternative_model_id,alternative_model_digest,alternative_adequacy_receipt_digest,alternative_adequacy_state,objective_id,measurement_unit,direction,materiality_threshold,reference_optimal_policy_ids,alternative_optimal_policy_ids,impact,decision_critical,policy_id,reference_value,alternative_value,reference_feasible,alternative_feasible,regret,feasibility_reversed,reference_source_digest,alternative_source_digest,reference_outcome_receipt_digest,alternative_outcome_receipt_digest,reference_violations,alternative_violations"
    );
    emit_report(&report);
    Ok(())
}
