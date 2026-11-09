// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Small known-answer comparison across structural model variants.
//!
//! The candidate set deliberately contains one model excluded by its
//! adequacy result. The output reports stability only across the two
//! qualified models and makes the partial model-set coverage explicit.

use civdyn_robust_policy::model_set_robustness::{
    CandidateModel, ConstraintComparison, DecisionCase, DecisionMetricRole, DecisionMetricSpec,
    DecisionMetricValue, DecisionPolicy, ModelPolicyOutcome, ModelQualification, ModelSetCoverage,
    ModelSetManifest, analyze_model_set_decisions,
};

const MODEL_SET_ID: &str = "civdyn-structural-decision-fixture-v0-1";
const COLUMN_COUNT: usize = 22;

fn blank(record_type: &str) -> Vec<String> {
    let mut row = vec![String::new(); COLUMN_COUNT];
    row[0] = record_type.to_string();
    row[1] = MODEL_SET_ID.to_string();
    row
}

fn emit(row: Vec<String>) {
    assert_eq!(row.len(), COLUMN_COUNT, "model-set CSV schema changed");
    println!("{}", row.join(","));
}

fn metric_value(metric_id: &str, unit: &str, value: f64) -> DecisionMetricValue {
    DecisionMetricValue {
        metric_id: metric_id.to_string(),
        unit: unit.to_string(),
        value: Some(value),
    }
}

fn outcome(
    model_id: &str,
    model_digest: &str,
    case_id: &str,
    case_digest: &str,
    policy_id: &str,
    policy_digest: &str,
    unmet_water_m3: f64,
    essential_coverage: f64,
) -> ModelPolicyOutcome {
    ModelPolicyOutcome {
        model_id: model_id.to_string(),
        model_digest: model_digest.to_string(),
        case_id: case_id.to_string(),
        case_digest: case_digest.to_string(),
        policy_id: policy_id.to_string(),
        policy_digest: policy_digest.to_string(),
        outcome_digest: format!("receipt-{model_id}-{case_id}-{policy_id}"),
        values: vec![
            metric_value("unmet-water", "m3", unmet_water_m3),
            metric_value("essential-coverage", "fraction", essential_coverage),
        ],
    }
}

fn fixture() -> (ModelSetManifest, Vec<ModelPolicyOutcome>) {
    let manifest = ModelSetManifest {
        model_set_id: MODEL_SET_ID.to_string(),
        manifest_digest: "sha256-model-set-manifest".to_string(),
        adequacy_registry_digest: "sha256-adequacy-registry".to_string(),
        case_manifest_digest: "sha256-case-manifest".to_string(),
        policy_manifest_digest: "sha256-policy-manifest".to_string(),
        metric_registry_digest: "sha256-metric-registry".to_string(),
        models: vec![
            CandidateModel {
                model_id: "model-a".to_string(),
                model_family_id: "hydrology-family".to_string(),
                model_digest: "sha256-model-a".to_string(),
                adequacy_receipt_digest: "sha256-adequacy-a".to_string(),
                qualification: ModelQualification::NoPrimaryDiscrepancyDetected,
            },
            CandidateModel {
                model_id: "model-b".to_string(),
                model_family_id: "network-family".to_string(),
                model_digest: "sha256-model-b".to_string(),
                adequacy_receipt_digest: "sha256-adequacy-b".to_string(),
                qualification: ModelQualification::NoPrimaryDiscrepancyDetected,
            },
            CandidateModel {
                model_id: "model-c".to_string(),
                model_family_id: "unqualified-family".to_string(),
                model_digest: "sha256-model-c".to_string(),
                adequacy_receipt_digest: "sha256-adequacy-c".to_string(),
                qualification: ModelQualification::PrimaryDiscrepancyDetected,
            },
        ],
        cases: vec![
            DecisionCase {
                case_id: "drought".to_string(),
                case_digest: "sha256-drought".to_string(),
            },
            DecisionCase {
                case_id: "energy-shock".to_string(),
                case_digest: "sha256-energy-shock".to_string(),
            },
        ],
        policies: vec![
            DecisionPolicy {
                policy_id: "reserve".to_string(),
                policy_digest: "sha256-reserve".to_string(),
            },
            DecisionPolicy {
                policy_id: "demand-first".to_string(),
                policy_digest: "sha256-demand-first".to_string(),
            },
            DecisionPolicy {
                policy_id: "growth-first".to_string(),
                policy_digest: "sha256-growth-first".to_string(),
            },
        ],
        metrics: vec![
            DecisionMetricSpec {
                metric_id: "unmet-water".to_string(),
                unit: "m3".to_string(),
                role: DecisionMetricRole::Objective {
                    direction:
                        civdyn_robust_policy::model_set_robustness::ObjectiveDirection::Minimize,
                },
            },
            DecisionMetricSpec {
                metric_id: "essential-coverage".to_string(),
                unit: "fraction".to_string(),
                role: DecisionMetricRole::HardConstraint {
                    comparison: ConstraintComparison::AtLeast,
                    threshold: 0.95,
                },
            },
        ],
    };

    let rows = vec![
        outcome(
            "model-a",
            "sha256-model-a",
            "drought",
            "sha256-drought",
            "reserve",
            "sha256-reserve",
            10.0,
            0.99,
        ),
        outcome(
            "model-a",
            "sha256-model-a",
            "drought",
            "sha256-drought",
            "demand-first",
            "sha256-demand-first",
            8.0,
            0.98,
        ),
        outcome(
            "model-a",
            "sha256-model-a",
            "drought",
            "sha256-drought",
            "growth-first",
            "sha256-growth-first",
            2.0,
            0.90,
        ),
        outcome(
            "model-a",
            "sha256-model-a",
            "energy-shock",
            "sha256-energy-shock",
            "reserve",
            "sha256-reserve",
            7.0,
            0.98,
        ),
        outcome(
            "model-a",
            "sha256-model-a",
            "energy-shock",
            "sha256-energy-shock",
            "demand-first",
            "sha256-demand-first",
            6.0,
            0.97,
        ),
        outcome(
            "model-a",
            "sha256-model-a",
            "energy-shock",
            "sha256-energy-shock",
            "growth-first",
            "sha256-growth-first",
            1.0,
            0.91,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "drought",
            "sha256-drought",
            "reserve",
            "sha256-reserve",
            9.0,
            0.98,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "drought",
            "sha256-drought",
            "demand-first",
            "sha256-demand-first",
            1.0,
            0.90,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "drought",
            "sha256-drought",
            "growth-first",
            "sha256-growth-first",
            6.0,
            0.99,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "energy-shock",
            "sha256-energy-shock",
            "reserve",
            "sha256-reserve",
            8.0,
            0.97,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "energy-shock",
            "sha256-energy-shock",
            "demand-first",
            "sha256-demand-first",
            2.0,
            0.92,
        ),
        outcome(
            "model-b",
            "sha256-model-b",
            "energy-shock",
            "sha256-energy-shock",
            "growth-first",
            "sha256-growth-first",
            5.0,
            0.98,
        ),
    ];

    (manifest, rows)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (manifest, outcomes) = fixture();
    let report = analyze_model_set_decisions(&manifest, &outcomes)?;

    println!(
        "record_type,model_set_id,model_id,model_family_id,qualification,eligible,case_id,policy_id,objective_id,unit,status,robust_feasible,expected_count,pass_count,fail_count,unknown_count,best_policy_ids,mean_regret,worst_case_regret,minimax_policy_ids,model_set_coverage,distinct_winner_count"
    );

    for receipt in &report.qualification_receipts {
        let mut row = blank("qualification");
        row[2] = receipt.model_id.clone();
        row[3] = receipt.model_family_id.clone();
        row[4] = receipt.qualification.name().to_string();
        row[5] = receipt.eligible_for_comparison.to_string();
        row[20] = report.coverage.name().to_string();
        emit(row);
    }

    for summary in &report.policy_robust_feasibility {
        let mut row = blank("policy_robust_feasibility");
        row[7] = summary.policy_id.clone();
        row[10] = "across_qualified_models_and_cases".to_string();
        row[11] = summary
            .feasible_across_qualified_models
            .map_or_else(|| "unknown".to_string(), |value| value.to_string());
        row[12] = summary.expected_model_case_count.to_string();
        row[13] = summary.pass_count.to_string();
        row[14] = summary.fail_count.to_string();
        row[15] = summary.unknown_count.to_string();
        row[20] = report.coverage.name().to_string();
        emit(row);
    }

    for assessment in &report.case_objectives {
        let mut row = blank("case_objective");
        row[2] = assessment.model_id.clone();
        row[6] = assessment.case_id.clone();
        row[8] = assessment.objective_id.clone();
        row[9] = assessment.unit.clone();
        row[10] = assessment.status.name().to_string();
        row[16] = assessment.best_policy_ids.join("|");
        row[20] = report.coverage.name().to_string();
        emit(row);
    }

    for summary in &report.objective_stability {
        let mut row = blank("objective_stability");
        row[8] = summary.objective_id.clone();
        row[9] = summary.unit.clone();
        row[10] = summary
            .ranking_sensitive
            .map_or_else(|| "not_assessed".to_string(), |value| value.to_string());
        row[12] = summary.expected_model_case_count.to_string();
        row[13] = summary.assessed_model_case_count.to_string();
        row[14] = summary.no_feasible_policy_count.to_string();
        row[15] = summary.not_assessed_count.to_string();
        row[16] = summary.best_policy_intersection.join("|");
        row[20] = report.coverage.name().to_string();
        row[21] = summary.distinct_winner_policy_count.to_string();
        emit(row);
    }

    for summary in &report.policy_objective_regret {
        let mut row = blank("policy_objective_regret");
        row[7] = summary.policy_id.clone();
        row[8] = summary.objective_id.clone();
        row[9] = summary.unit.clone();
        row[10] = "metric_specific".to_string();
        row[11] = summary
            .feasible_across_qualified_models
            .map_or_else(|| "unknown".to_string(), |value| value.to_string());
        row[12] = summary.expected_model_case_count.to_string();
        row[13] = summary.assessed_model_case_count.to_string();
        row[15] = summary.unknown_model_case_count.to_string();
        row[17] = summary
            .mean_regret
            .map(|value| format!("{value:.6}"))
            .unwrap_or_default();
        row[18] = summary
            .worst_case_regret
            .map(|value| format!("{value:.6}"))
            .unwrap_or_default();
        row[20] = report.coverage.name().to_string();
        emit(row);
    }

    for summary in &report.minimax_regret {
        let mut row = blank("minimax_regret");
        row[8] = summary.objective_id.clone();
        row[9] = summary.unit.clone();
        row[10] = "metric_specific".to_string();
        row[16] = summary.minimax_policy_ids.join("|");
        row[18] = summary
            .minimum_worst_case_regret
            .map(|value| format!("{value:.6}"))
            .unwrap_or_default();
        row[19] = summary.minimax_policy_ids.join("|");
        row[20] = report.coverage.name().to_string();
        emit(row);
    }

    if report.coverage != ModelSetCoverage::PartialQualifiedModelSet {
        return Err("known-answer fixture must disclose its excluded model".into());
    }
    Ok(())
}
