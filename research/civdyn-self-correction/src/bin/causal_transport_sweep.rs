// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    assess_risk_difference_transport, AssumptionDisposition, AssumptionEvidence,
    CausalTransportAssumptions, CausalTransportInput, CausalTransportResult,
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
        "causal transport receipt must match the frozen schema"
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
        evidence_digest: "sha256-assumption-evidence".to_string(),
    };
    CausalTransportAssumptions {
        source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
        selection_exchangeability: evidence(AssumptionDisposition::SupportedByEvidence),
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

fn format_optional(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.8}"))
        .unwrap_or_default()
}

fn emit_result(
    scenario: &str,
    result: &CausalTransportResult,
) {
    for stratum in &result.strata {
        emit(vec![
            scenario.to_string(),
            stratum.modifier_id.clone(),
            stratum
                .source_population_count
                .map(|value| value.to_string())
                .unwrap_or_default(),
            stratum.target_population_count.to_string(),
            stratum
                .source_control_sample_size
                .map(|value| value.to_string())
                .unwrap_or_default(),
            stratum
                .source_control_events
                .map(|value| value.to_string())
                .unwrap_or_default(),
            stratum
                .source_treatment_sample_size
                .map(|value| value.to_string())
                .unwrap_or_default(),
            stratum
                .source_treatment_events
                .map(|value| value.to_string())
                .unwrap_or_default(),
            format_optional(stratum.control_risk),
            format_optional(stratum.treatment_risk),
            format_optional(stratum.conditional_risk_difference),
            format!("{:.8}", stratum.target_population_share),
            stratum.source_support.to_string(),
            format!("{:.8}", result.source_ate),
            format_optional(result.target_ate_point),
            format_optional(result.target_ate_lower),
            format_optional(result.target_ate_upper),
            format!("{:.8}", result.supported_target_mass),
            format!("{:.8}", result.unsupported_target_mass),
            result.source_population_total.to_string(),
            result.target_population_total.to_string(),
            result.supported_target_population_count.to_string(),
            result.unsupported_target_population_count.to_string(),
            result.identification_status.name().to_string(),
            result.identification_reason.name().to_string(),
            result
                .assumptions
                .source_internal_validity
                .disposition
                .name()
                .to_string(),
            result
                .assumptions
                .selection_exchangeability
                .disposition
                .name()
                .to_string(),
            result.assumptions.no_interference.disposition.name().to_string(),
            result
                .assumptions
                .treatment_version_invariance
                .disposition
                .name()
                .to_string(),
            result.source_data_digest.clone(),
            result.target_population_digest.clone(),
            result.protocol_digest.clone(),
            result.estimand.to_string(),
        ]);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,modifier_id,source_population_count,target_population_count,source_control_n,source_control_events,source_treatment_n,source_treatment_events,control_risk,treatment_risk,conditional_risk_difference,target_population_share,source_support,source_ate,target_ate_point,target_ate_lower,target_ate_upper,supported_target_mass,unsupported_target_mass,source_population_total,target_population_total,supported_target_population_count,unsupported_target_population_count,identification_status,identification_reason,source_internal_validity,selection_exchangeability,no_interference,treatment_version_invariance,source_data_digest,target_population_digest,protocol_digest,estimand"
    );

    let identified = assess_risk_difference_transport(&input(vec![
        TargetPopulationStratum {
            modifier_id: "low-infrastructure".to_string(),
            target_population_count: 20,
        },
        TargetPopulationStratum {
            modifier_id: "high-infrastructure".to_string(),
            target_population_count: 80,
        },
    ]))?;
    emit_result("full-support-standardization", &identified);

    let partial = assess_risk_difference_transport(&input(vec![
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
    ]))?;
    emit_result("target-support-failure", &partial);
    Ok(())
}
