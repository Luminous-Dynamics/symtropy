// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::policy_sensitivity::{
    evaluate_policy_sensitivity, ConstraintRelation, MetricKind, MetricSpec,
    ModelAdequacyState, ModelWorld, ObjectiveDirection, PairwisePolicyOrdering,
    PolicyFeasibility, PolicyMetricOutcome, PolicySensitivityReport, PolicySpec,
};

const CSV_COLUMNS: usize = 29;

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), CSV_COLUMNS, "CSV receipt must match the frozen schema");
    println!("{}", fields.join(","));
}

fn model_outcome_row(
    outcome: &PolicyMetricOutcome,
    model: &ModelWorld,
    feasible: bool,
    violations: &[String],
    ceiling: &str,
) -> Vec<String> {
    vec![
        "model_outcome".to_string(),
        outcome.model_id.clone(),
        outcome.policy_id.clone(),
        outcome.metric_id.clone(),
        outcome.measurement_unit.clone(),
        format!("{value:.6}", value = outcome.value),
        feasible.to_string(),
        violations.join(";"),
        model.model_digest.clone(),
        model.adequacy_model_digest.clone(),
        model.adequacy_receipt_digest.clone(),
        outcome.source_digest.clone(),
        model.adequacy.name().to_string(),
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
        ceiling.to_string(),
        String::new(),
        String::new(),
    ]
}
fn summary_row(
    policy_id: &str,
    metric_id: &str,
    unit: &str,
    feasibility: PolicyFeasibility,
    feasible_count: usize,
    model_count: usize,
    regret: Option<f64>,
    best_rank: Option<usize>,
    worst_rank: Option<usize>,
    models_where_best: usize,
    ceiling: &str,
    unqualified_models: &str,
    any_model_without_feasible_policy: bool,
) -> Vec<String> {
    vec![
        "objective_summary".to_string(),
        String::new(),
        policy_id.to_string(),
        metric_id.to_string(),
        unit.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        feasibility.name().to_string(),
        feasible_count.to_string(),
        model_count.to_string(),
        regret.map(|value| format!("{value:.6}")).unwrap_or_default(),
        best_rank.map(|value| value.to_string()).unwrap_or_default(),
        worst_rank.map(|value| value.to_string()).unwrap_or_default(),
        models_where_best.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        ceiling.to_string(),
        unqualified_models.to_string(),
        any_model_without_feasible_policy.to_string(),
    ]
}

fn pairwise_row(
    pair: &PairwisePolicyOrdering,
    ceiling: &str,
    unqualified_models: &str,
    any_model_without_feasible_policy: bool,
) -> Vec<String> {
    vec![
        "pairwise_ordering".to_string(),
        String::new(),
        String::new(),
        pair.metric_id.clone(),
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
        pair.policy_a.clone(),
        pair.policy_b.clone(),
        pair.models_where_a_better.to_string(),
        pair.models_where_b_better.to_string(),
        pair.tied_models.to_string(),
        pair.discordant_model_pairs.to_string(),
        ceiling.to_string(),
        unqualified_models.to_string(),
        any_model_without_feasible_policy.to_string(),
    ]
}

fn fixture() -> (
    Vec<ModelWorld>,
    Vec<PolicySpec>,
    Vec<MetricSpec>,
    Vec<PolicyMetricOutcome>,
) {
    let models = vec![
        ModelWorld {
            id: "world-a".to_string(),
            family_id: "mechanism-family-a".to_string(),
            model_digest: "sha256-model-a".to_string(),
            adequacy_model_digest: "sha256-model-a".to_string(),
            adequacy_receipt_digest: "sha256-adequacy-a".to_string(),
            adequacy: ModelAdequacyState::NoPrimaryDiscrepancyDetected,
        },
        ModelWorld {
            id: "world-b".to_string(),
            family_id: "mechanism-family-b".to_string(),
            model_digest: "sha256-model-b".to_string(),
            adequacy_model_digest: "sha256-model-b".to_string(),
            adequacy_receipt_digest: "sha256-adequacy-b".to_string(),
            adequacy: ModelAdequacyState::PrimaryDiscrepancyDetected,
        },
    ];

    let policies = ["baseline", "conserve", "expand"]
        .into_iter()
        .map(|id| PolicySpec { id: id.to_string() })
        .collect();

    let metrics = vec![
        MetricSpec {
            id: "essential-service".to_string(),
            measurement_unit: "households_served".to_string(),
            kind: MetricKind::Objective {
                direction: ObjectiveDirection::Maximize,
            },
        },
        MetricSpec {
            id: "ecological-integrity".to_string(),
            measurement_unit: "index_points".to_string(),
            kind: MetricKind::Objective {
                direction: ObjectiveDirection::Maximize,
            },
        },
        MetricSpec {
            id: "water-withdrawal".to_string(),
            measurement_unit: "m3".to_string(),
            kind: MetricKind::Constraint {
                relation: ConstraintRelation::AtMost(12.0),
            },
        },
    ];

    let mut outcomes = Vec::new();
    for (model, rows) in [
        (
            "world-a",
            [
                ("baseline", 80.0, 90.0, 8.0),
                ("conserve", 90.0, 70.0, 11.0),
                ("expand", 100.0, 60.0, 15.0),
            ],
        ),
        (
            "world-b",
            [
                ("baseline", 75.0, 80.0, 8.0),
                ("conserve", 70.0, 95.0, 11.0),
                ("expand", 95.0, 85.0, 10.0),
            ],
        ),
    ] {
        for (policy, service, ecology, water) in rows {
            for (metric_id, unit, value) in [
                ("essential-service", "households_served", service),
                ("ecological-integrity", "index_points", ecology),
                ("water-withdrawal", "m3", water),
            ] {
                outcomes.push(PolicyMetricOutcome {
                    model_id: model.to_string(),
                    policy_id: policy.to_string(),
                    metric_id: metric_id.to_string(),
                    measurement_unit: unit.to_string(),
                    value,
                    source_digest: format!("sha256-outcome-{model}-{policy}-{metric_id}"),
                });
            }
        }
    }

    (models, policies, metrics, outcomes)
}

fn emit_report(
    models: &[ModelWorld],
    outcomes: &[PolicyMetricOutcome],
    report: &PolicySensitivityReport,
) {
    let ceiling = report.claim_ceiling.name();
    let unqualified = report.models_with_unqualified_adequacy.join(";");

    for outcome in outcomes {
        let feasibility = report
            .model_policy_feasibility
            .iter()
            .find(|row| {
                row.model_id == outcome.model_id && row.policy_id == outcome.policy_id
            })
            .expect("complete model-policy feasibility matrix");
        let model = models
            .iter()
            .find(|model| model.id == outcome.model_id)
            .expect("model identity exists");
        let fields = model_outcome_row(
            outcome,
            model,
            feasibility.feasible,
            &feasibility.violated_constraints,
            ceiling,
        );
        emit(fields);
    }

    for summary in &report.objective_summaries {
        emit(summary_row(
            &summary.policy_id,
            &summary.metric_id,
            &summary.measurement_unit,
            summary.feasibility,
            summary.feasible_model_count,
            summary.total_model_count,
            summary.worst_case_regret,
            summary.best_rank,
            summary.worst_rank,
            summary.models_where_best,
            ceiling,
            &unqualified,
            report.any_model_without_feasible_policy,
        ));
    }

    for pair in &report.pairwise_orderings {
        emit(pairwise_row(
            pair,
            ceiling,
            &unqualified,
            report.any_model_without_feasible_policy,
        ));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (models, policies, metrics, outcomes) = fixture();
    let report = evaluate_policy_sensitivity(&models, &policies, &metrics, &outcomes)?;

    println!(
        "record_type,model_id,policy_id,metric_id,measurement_unit,value,feasible,violated_constraints,model_digest,adequacy_model_digest,adequacy_receipt_digest,source_digest,adequacy_state,policy_feasibility,feasible_model_count,total_model_count,worst_case_regret,best_rank,worst_rank,models_where_best,policy_a,policy_b,models_where_a_better,models_where_b_better,tied_models,discordant_model_pairs,claim_ceiling,models_with_unqualified_adequacy,any_model_without_feasible_policy"
    );
    emit_report(&models, &outcomes, &report);
    Ok(())
}
