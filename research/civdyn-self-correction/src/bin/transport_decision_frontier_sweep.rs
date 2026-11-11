// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    AssumptionDisposition, AssumptionEvidence, CausalTransportAssumptions, CausalTransportInput,
    SourceEffectStratum, TargetPopulationStratum,
};
use civdyn_self_correction::causal_transport_sensitivity::{
    assess_bernoulli_outcome_shift_transport,
};
use civdyn_self_correction::transport_decision_frontier::{
    classify_target_effect_decision, find_recommendation_tipping_point,
    DecisionThresholdEvidence,
};

const SOURCE_DIGEST: &str = "sha256-causal-transport-source-rct";
const TARGET_DIGEST: &str = "sha256-causal-transport-target-population";
const PROTOCOL_DIGEST: &str = "sha256-causal-transport-protocol-v0.1";
const SENSITIVITY_DIGEST: &str = "sha256-predeclared-bernoulli-lambda-scenario";
const CSV_COLUMNS: usize = 24;
const THRESHOLDS: [f64; 5] = [0.0, 0.1, 0.2, 0.3, 0.5];
const LAMBDAS: [f64; 3] = [1.0, 2.0, 4.0];

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
        "decision-frontier receipt must match the frozen schema"
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

fn fmt(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.10}"))
        .unwrap_or_default()
}

fn input() -> CausalTransportInput {
    let evidence = |disposition| AssumptionEvidence {
        disposition,
        evidence_digest: "sha256-assumption-evidence".to_string(),
    };
    CausalTransportInput {
        source_data_digest: SOURCE_DIGEST.to_string(),
        target_population_digest: TARGET_DIGEST.to_string(),
        protocol_digest: PROTOCOL_DIGEST.to_string(),
        assumptions: CausalTransportAssumptions {
            source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
            selection_exchangeability: evidence(AssumptionDisposition::Unknown),
            no_interference: evidence(AssumptionDisposition::SupportedByDesign),
            treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
        },
        source_strata: vec![
            SourceEffectStratum {
                modifier_id: "low-infrastructure".to_string(),
                source_population_count: 80,
                control_sample_size: 40,
                control_events: 4,
                treatment_sample_size: 40,
                treatment_events: 12,
            },
            SourceEffectStratum {
                modifier_id: "high-infrastructure".to_string(),
                source_population_count: 20,
                control_sample_size: 10,
                control_events: 5,
                treatment_sample_size: 10,
                treatment_events: 8,
            },
        ],
        target_strata: vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 80,
            },
        ],
    }
}

fn threshold_evidence(value: f64) -> DecisionThresholdEvidence {
    DecisionThresholdEvidence {
        threshold: format!("{value:.2}"),
        effect_unit: "risk_difference".to_string(),
        rationale: "Predeclared minimum worthwhile target-population risk reduction".to_string(),
        source_digest: format!("sha256-minimum-benefit-threshold-{value:.2}"),
        disposition: AssumptionDisposition::SupportedByEvidence,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let candidate = input();
    let sensitivity_evidence = AssumptionEvidence {
        disposition: AssumptionDisposition::SupportedByEvidence,
        evidence_digest: SENSITIVITY_DIGEST.to_string(),
    };
    println!(
        "threshold,lambda,decision,lower_effect_bound,upper_effect_bound,interval_width,baseline_decision,decision_at_horizon,frontier_status,frontier_tipping_lambda,frontier_lower_bracket,frontier_upper_bracket,frontier_tolerance,effect_unit,threshold_rationale,threshold_source_digest,sensitivity_scenario_digest,source_data_digest,target_population_digest,protocol_digest,source_internal_validity,selection_exchangeability,no_interference,treatment_version_invariance"
    );

    for threshold in THRESHOLDS {
        let evidence = threshold_evidence(threshold);
        let frontier = find_recommendation_tipping_point(
            |lambda| {
                assess_bernoulli_outcome_shift_transport(
                    &candidate,
                    lambda,
                    &sensitivity_evidence,
                )
                .map_err(|_| {
                    civdyn_self_correction::transport_decision_frontier::DecisionFrontierError::SensitivityBoundsMissing
                })
            },
            &evidence,
            8.0,
            1e-8,
        )?;

        for lambda in LAMBDAS {
            let sensitivity = assess_bernoulli_outcome_shift_transport(
                &candidate,
                lambda,
                &sensitivity_evidence,
            )?;
            let decision = classify_target_effect_decision(&sensitivity, &evidence)?;
            emit(vec![
                format!("{threshold:.2}"),
                format!("{lambda:.8}"),
                decision.decision.name().to_string(),
                fmt(decision.lower_effect_bound),
                fmt(decision.upper_effect_bound),
                fmt(decision.interval_width),
                frontier.baseline_decision.name().to_string(),
                frontier.decision_at_horizon.name().to_string(),
                frontier.status.name().to_string(),
                fmt(frontier.tipping_point.map(|value| value.lambda)),
                fmt(frontier.tipping_point.map(|value| value.lower_bracket)),
                fmt(frontier.tipping_point.map(|value| value.upper_bracket)),
                fmt(frontier.tipping_point.map(|value| value.tolerance)),
                decision.effect_unit,
                decision.threshold_rationale,
                decision.threshold_source_digest,
                decision.sensitivity_scenario_digest,
                SOURCE_DIGEST.to_string(),
                TARGET_DIGEST.to_string(),
                PROTOCOL_DIGEST.to_string(),
                sensitivity
                    .assumptions
                    .source_internal_validity
                    .disposition
                    .name()
                    .to_string(),
                sensitivity
                    .assumptions
                    .selection_exchangeability
                    .disposition
                    .name()
                    .to_string(),
                sensitivity
                    .assumptions
                    .no_interference
                    .disposition
                    .name()
                    .to_string(),
                sensitivity
                    .assumptions
                    .treatment_version_invariance
                    .disposition
                    .name()
                    .to_string(),
            ]);
        }
    }
    Ok(())
}
