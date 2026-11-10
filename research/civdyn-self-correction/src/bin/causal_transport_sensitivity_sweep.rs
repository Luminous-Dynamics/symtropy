// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    AssumptionDisposition, AssumptionEvidence, CausalTransportAssumptions, SourceEffectStratum,
    TargetPopulationStratum,
};
use civdyn_self_correction::causal_transport_sensitivity::{
    OutcomeShiftSensitivityReport, assess_binary_outcome_shift_sensitivity,
};

const CSV_COLUMNS: usize = 40;
const LAMBDA_VALUES: [f64; 5] = [1.0, 1.25, 1.5, 2.0, 4.0];
const TIPPING_SEARCH_MAX: f64 = 16.0;
const TIPPING_TOLERANCE: f64 = 1e-8;
const SOURCE_DIGEST: &str = "sha256-causal-transport-source-rct";
const PROTOCOL_DIGEST: &str = "sha256-causal-transport-sensitivity-v0.1";

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
        "outcome-shift sensitivity receipt must match frozen schema"
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
        .map(|number| format!("{number:.10}"))
        .unwrap_or_default()
}

fn emit_report(scenario: &str, report: &OutcomeShiftSensitivityReport) {
    for point in &report.frontier {
        for stratum in &point.strata {
            let tipping = report.null_tipping_point;
            emit(vec![
                scenario.to_string(),
                format!("{:.8}", point.lambda),
                stratum.modifier_id.clone(),
                stratum.target_population_count.to_string(),
                format!("{:.10}", stratum.target_population_share),
                stratum.source_support.to_string(),
                optional(stratum.control_source_risk),
                optional(stratum.treatment_source_risk),
                optional(stratum.control_target_risk_lower),
                optional(stratum.control_target_risk_upper),
                optional(stratum.treatment_target_risk_lower),
                optional(stratum.treatment_target_risk_upper),
                format!("{:.10}", stratum.risk_difference_lower),
                format!("{:.10}", stratum.risk_difference_upper),
                format!("{:.10}", point.target_ate_lower),
                format!("{:.10}", point.target_ate_upper),
                format!("{:.10}", point.interval_width),
                point.supported_target_population_count.to_string(),
                point.unsupported_target_population_count.to_string(),
                format!("{:.10}", point.supported_target_mass),
                format!("{:.10}", point.unsupported_target_mass),
                tipping.map(|value| format!("{:.10}", value.lambda)).unwrap_or_default(),
                tipping.map(|value| format!("{:.10}", value.lower_bracket)).unwrap_or_default(),
                tipping.map(|value| format!("{:.10}", value.upper_bracket)).unwrap_or_default(),
                tipping.map(|value| format!("{:.8}", value.search_max)).unwrap_or_default(),
                tipping.map(|value| format!("{:.12}", value.tolerance)).unwrap_or_default(),
                format!("{:.10}", report.source_ate),
                report.source_population_total.to_string(),
                report.target_population_total.to_string(),
                report.source_data_digest.clone(),
                report.target_population_digest.clone(),
                report.protocol_digest.clone(),
                report.assumptions.source_internal_validity.disposition.name().to_string(),
                report.assumptions.source_internal_validity.evidence_digest.clone(),
                report.assumptions.selection_exchangeability.disposition.name().to_string(),
                report.assumptions.selection_exchangeability.evidence_digest.clone(),
                report.assumptions.no_interference.disposition.name().to_string(),
                report.assumptions.no_interference.evidence_digest.clone(),
                report.assumptions.treatment_version_invariance.disposition.name().to_string(),
                report.assumptions.treatment_version_invariance.evidence_digest.clone(),
            ]);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,lambda,modifier_id,target_population_count,target_population_share,source_support,control_source_risk,treatment_source_risk,control_target_risk_lower,control_target_risk_upper,treatment_target_risk_lower,treatment_target_risk_upper,stratum_risk_difference_lower,stratum_risk_difference_upper,target_ate_lower,target_ate_upper,interval_width,supported_target_population_count,unsupported_target_population_count,supported_target_mass,unsupported_target_mass,null_tipping_lambda,tipping_lower_bracket,tipping_upper_bracket,tipping_search_max,tipping_tolerance,source_ate,source_population_total,target_population_total,source_data_digest,target_population_digest,protocol_digest,source_internal_validity,source_internal_validity_digest,selection_exchangeability,selection_exchangeability_digest,no_interference,no_interference_digest,treatment_version_invariance,treatment_version_invariance_digest"
    );

    let assumptions = assumptions();
    let source = source_strata();
    let full = assess_binary_outcome_shift_sensitivity(
        SOURCE_DIGEST,
        "sha256-causal-transport-target-full-support",
        PROTOCOL_DIGEST,
        &assumptions,
        &source,
        &full_support_target(),
        &LAMBDA_VALUES,
        TIPPING_SEARCH_MAX,
        TIPPING_TOLERANCE,
    )?;
    emit_report("full-support", &full);

    let partial = assess_binary_outcome_shift_sensitivity(
        SOURCE_DIGEST,
        "sha256-causal-transport-target-partial-support",
        PROTOCOL_DIGEST,
        &assumptions,
        &source,
        &partial_support_target(),
        &LAMBDA_VALUES,
        TIPPING_SEARCH_MAX,
        TIPPING_TOLERANCE,
    )?;
    emit_report("partial-support", &partial);
    Ok(())
}
