// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    assess_likelihood_ratio_transport_sensitivity, AssumptionDisposition, AssumptionEvidence,
    CausalTransportAssumptions, CausalTransportInput, OutcomeShiftSensitivitySpec,
    SourceEffectStratum, TargetPopulationStratum,
};

const SOURCE_DIGEST: &str = "sha256-causal-transport-source-rct";
const TARGET_DIGEST: &str = "sha256-causal-transport-target-population";
const PROTOCOL_DIGEST: &str = "sha256-causal-transport-protocol-v0.1";
const CSV_COLUMNS: usize = 33;

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
        "Lambda sensitivity receipt must match the frozen schema"
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
        evidence_digest: "sha256-causal-transport-assumption-evidence".to_string(),
    };
    CausalTransportAssumptions {
        source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
        selection_exchangeability: evidence(AssumptionDisposition::Violated),
        no_interference: evidence(AssumptionDisposition::SupportedByDesign),
        treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
    }
}

fn source_strata() -> Vec<SourceEffectStratum> {
    vec![
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
    ]
}

fn input(target_strata: Vec<TargetPopulationStratum>) -> CausalTransportInput {
    CausalTransportInput {
        source_data_digest: SOURCE_DIGEST.to_string(),
        target_population_digest: TARGET_DIGEST.to_string(),
        protocol_digest: PROTOCOL_DIGEST.to_string(),
        assumptions: assumptions(),
        source_strata: source_strata(),
        target_strata,
    }
}

fn full_support_target() -> Vec<TargetPopulationStratum> {
    vec![
        TargetPopulationStratum {
            modifier_id: "low-infrastructure".to_string(),
            target_population_count: 20,
        },
        TargetPopulationStratum {
            modifier_id: "high-infrastructure".to_string(),
            target_population_count: 80,
        },
    ]
}

fn partial_support_target() -> Vec<TargetPopulationStratum> {
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
    ]
}

fn optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.8}"))
        .unwrap_or_default()
}

fn emit_scenario(
    scenario: &str,
    target: Vec<TargetPopulationStratum>,
    lambda: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let sensitivity = OutcomeShiftSensitivitySpec {
        lambda,
        scenario_digest: format!("sha256-outcome-shift-lambda-{lambda:.2}"),
    };
    let result = assess_likelihood_ratio_transport_sensitivity(&input(target), &sensitivity)?;

    for stratum in &result.strata {
        emit(vec![
            scenario.to_string(),
            format!("{:.4}", result.lambda),
            result.sensitivity_scenario_digest.clone(),
            stratum.modifier_id.clone(),
            stratum.source_support.to_string(),
            stratum.target_population_count.to_string(),
            format!("{:.8}", stratum.target_population_share),
            stratum
                .control
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.source_risk))
                .unwrap_or_default(),
            stratum
                .control
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.target_risk_lower))
                .unwrap_or_default(),
            stratum
                .control
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.target_risk_upper))
                .unwrap_or_default(),
            stratum
                .treatment
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.source_risk))
                .unwrap_or_default(),
            stratum
                .treatment
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.target_risk_lower))
                .unwrap_or_default(),
            stratum
                .treatment
                .as_ref()
                .map(|bounds| format!("{:.8}", bounds.target_risk_upper))
                .unwrap_or_default(),
            optional(stratum.risk_difference_lower),
            optional(stratum.risk_difference_upper),
            format!("{:.8}", result.source_ate),
            format!("{:.8}", result.target_ate_lower),
            format!("{:.8}", result.target_ate_upper),
            result.source_population_total.to_string(),
            result.target_population_total.to_string(),
            result.supported_target_population_count.to_string(),
            result.unsupported_target_population_count.to_string(),
            format!("{:.8}", result.supported_target_mass),
            format!("{:.8}", result.unsupported_target_mass),
            result.status.name().to_string(),
            result.interval_collapsed.to_string(),
            result.source_internal_validity.name().to_string(),
            result.selection_exchangeability.name().to_string(),
            result.no_interference.name().to_string(),
            result.treatment_version_invariance.name().to_string(),
            result.source_data_digest.clone(),
            result.target_population_digest.clone(),
            result.protocol_digest.clone(),
        ]);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,lambda,sensitivity_scenario_digest,modifier_id,source_support,target_population_count,target_population_share,source_control_risk,target_control_lower,target_control_upper,source_treatment_risk,target_treatment_lower,target_treatment_upper,stratum_risk_difference_lower,stratum_risk_difference_upper,source_ate,target_ate_lower,target_ate_upper,source_population_total,target_population_total,supported_target_population_count,unsupported_target_population_count,supported_target_mass,unsupported_target_mass,status,interval_collapsed,source_internal_validity,selection_exchangeability,no_interference,treatment_version_invariance,source_data_digest,target_population_digest,protocol_digest"
    );

    for lambda in [1.0, 2.0, 4.0] {
        emit_scenario("full_support", full_support_target(), lambda)?;
    }
    emit_scenario("target_support_failure", partial_support_target(), 2.0)?;
    Ok(())
}
