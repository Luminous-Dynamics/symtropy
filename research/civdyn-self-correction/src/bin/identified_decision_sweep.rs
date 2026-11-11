// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    assess_risk_difference_transport, AssumptionDisposition, AssumptionEvidence,
    CausalTransportAssumptions, CausalTransportInput, SourceEffectStratum,
    TargetPopulationStratum,
};
use civdyn_self_correction::identified_decision::{
    evaluate_identified_decision, HardConstraintRelation, HardConstraintSpec,
    IdentifiedDecisionInput, PolicyConstraintInterval, PolicyUtilityBounds, UtilityDirection,
};

const CSV_COLUMNS: usize = 18;

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
        "identified-decision receipt must match the frozen schema"
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

fn assumptions() -> CausalTransportAssumptions {
    let evidence = |disposition| AssumptionEvidence {
        disposition,
        evidence_digest: "sha256-policy-assumption-evidence".to_string(),
    };
    CausalTransportAssumptions {
        source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
        selection_exchangeability: evidence(AssumptionDisposition::SupportedByEvidence),
        no_interference: evidence(AssumptionDisposition::SupportedByDesign),
        treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
    }
}

fn transport_input(
    source_digest: &str,
    protocol_digest: &str,
    low_treatment_events: u64,
    high_treatment_events: u64,
    target_strata: Vec<TargetPopulationStratum>,
) -> CausalTransportInput {
    CausalTransportInput {
        source_data_digest: source_digest.to_string(),
        target_population_digest: "sha256-shared-target-population".to_string(),
        protocol_digest: protocol_digest.to_string(),
        assumptions: assumptions(),
        source_strata: vec![
            SourceEffectStratum {
                modifier_id: "low-infrastructure".to_string(),
                source_population_count: 80,
                control_sample_size: 40,
                control_events: 4,
                treatment_sample_size: 40,
                treatment_events: low_treatment_events,
            },
            SourceEffectStratum {
                modifier_id: "high-infrastructure".to_string(),
                source_population_count: 20,
                control_sample_size: 10,
                control_events: 5,
                treatment_sample_size: 10,
                treatment_events: high_treatment_events,
            },
        ],
        target_strata,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let partial = assess_risk_difference_transport(&transport_input(
        "sha256-maintenance-source",
        "sha256-maintenance-transport-protocol",
        12,
        8,
        vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 60,
            },
            TargetPopulationStratum {
                modifier_id: "remote-isolated".to_string(),
                target_population_count: 20,
            },
        ],
    ))?;

    let identified = assess_risk_difference_transport(&transport_input(
        "sha256-infrastructure-source",
        "sha256-infrastructure-transport-protocol",
        8,
        8,
        vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 80,
            },
        ],
    ))?;

    let utilities = vec![
        PolicyUtilityBounds::from_causal_transport(
            "maintenance",
            "essential-service-benefit",
            "risk_difference",
            UtilityDirection::Maximize,
            &partial,
        )?,
        PolicyUtilityBounds::from_causal_transport(
            "infrastructure",
            "essential-service-benefit",
            "risk_difference",
            UtilityDirection::Maximize,
            &identified,
        )?,
    ];

    let constraints = vec![
        HardConstraintSpec {
            id: "daily-water-use".to_string(),
            measurement_unit: "litres_per_household_day".to_string(),
            relation: HardConstraintRelation::AtMost,
            threshold: 100.0,
            evidence_digest: "sha256-water-limit".to_string(),
        },
        HardConstraintSpec {
            id: "minimum-service-coverage".to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            relation: HardConstraintRelation::AtLeast,
            threshold: 0.85,
            evidence_digest: "sha256-service-floor".to_string(),
        },
    ];

    let constraint_intervals = vec![
        PolicyConstraintInterval {
            policy_id: "maintenance".to_string(),
            constraint_id: "daily-water-use".to_string(),
            measurement_unit: "litres_per_household_day".to_string(),
            lower: 55.0,
            upper: 70.0,
            evidence_digest: "sha256-maintenance-water".to_string(),
        },
        PolicyConstraintInterval {
            policy_id: "maintenance".to_string(),
            constraint_id: "minimum-service-coverage".to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            lower: 0.86,
            upper: 0.90,
            evidence_digest: "sha256-maintenance-service".to_string(),
        },
        PolicyConstraintInterval {
            policy_id: "infrastructure".to_string(),
            constraint_id: "daily-water-use".to_string(),
            measurement_unit: "litres_per_household_day".to_string(),
            lower: 45.0,
            upper: 65.0,
            evidence_digest: "sha256-infrastructure-water".to_string(),
        },
        PolicyConstraintInterval {
            policy_id: "infrastructure".to_string(),
            constraint_id: "minimum-service-coverage".to_string(),
            measurement_unit: "coverage_fraction".to_string(),
            lower: 0.86,
            upper: 0.92,
            evidence_digest: "sha256-infrastructure-service".to_string(),
        },
    ];

    let report = evaluate_identified_decision(&IdentifiedDecisionInput {
        metric_id: "essential-service-benefit".to_string(),
        measurement_unit: "risk_difference".to_string(),
        direction: UtilityDirection::Maximize,
        utilities,
        constraints,
        constraint_intervals,
    })?;

    println!(
        "policy_id,metric_id,utility_unit,direction,utility_lower,utility_upper,utility_evidence_digest,source_data_digest,target_population_digest,transport_protocol_digest,robust_feasibility,constraint_statuses,worst_case_regret,minimax_selected,claim_ceiling,uncertainty_set,robustly_feasible_policy_ids,minimax_regret_policy_ids"
    );

    for policy in &report.policies {
        let constraint_statuses = policy
            .constraints
            .iter()
            .map(|constraint| {
                format!("{}:{}", constraint.constraint_id, constraint.status.name())
            })
            .collect::<Vec<_>>()
            .join(";");
        emit(vec![
            policy.policy_id.clone(),
            report.metric_id.clone(),
            report.measurement_unit.clone(),
            report.direction.name().to_string(),
            format!("{:.8}", policy.utility_lower),
            format!("{:.8}", policy.utility_upper),
            policy.utility_evidence_digest.clone(),
            policy.source_data_digest.clone(),
            policy.target_population_digest.clone(),
            policy.transport_protocol_digest.clone(),
            policy.feasibility.name().to_string(),
            constraint_statuses,
            policy
                .worst_case_regret
                .map(|value| format!("{value:.8}"))
                .unwrap_or_default(),
            policy.selected_as_minimax_regret.to_string(),
            report.claim_ceiling.name().to_string(),
            report.uncertainty_set.to_string(),
            report.robustly_feasible_policy_ids.join(";"),
            report.minimax_regret_policy_ids.join(";"),
        ]);
    }
    Ok(())
}
