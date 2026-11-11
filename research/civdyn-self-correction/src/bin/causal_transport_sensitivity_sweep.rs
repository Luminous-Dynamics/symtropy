// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    AssumptionDisposition, AssumptionEvidence, CausalTransportAssumptions, CausalTransportInput,
    SourceEffectStratum, TargetPopulationStratum,
};
use civdyn_self_correction::causal_transport_sensitivity::{
    assess_bernoulli_outcome_shift_transport, find_null_tipping_point,
    CausalTransportSensitivityResult, LambdaTippingPoint,
};

const SOURCE_DIGEST: &str = "sha256-causal-transport-source-rct";
const TARGET_DIGEST: &str = "sha256-causal-transport-target-population";
const PROTOCOL_DIGEST: &str = "sha256-causal-transport-protocol-v0.1";
const SENSITIVITY_DIGEST: &str = "sha256-predeclared-bernoulli-lambda-sensitivity-v0.1";
const CSV_COLUMNS: usize = 42;
const LAMBDA_GRID: [f64; 3] = [1.0, 2.0, 4.0];

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
        "causal transport sensitivity receipt must match the frozen schema"
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

fn assumptions() -> CausalTransportAssumptions {
    let evidence = |disposition| AssumptionEvidence {
        disposition,
        evidence_digest: "sha256-assumption-evidence".to_string(),
    };
    CausalTransportAssumptions {
        source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
        // Exact conditional outcome invariance is deliberately not asserted.
        // The sensitivity envelope replaces that equality assumption.
        selection_exchangeability: evidence(AssumptionDisposition::Unknown),
        no_interference: evidence(AssumptionDisposition::SupportedByDesign),
        treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
    }
}

fn sensitivity_evidence() -> AssumptionEvidence {
    AssumptionEvidence {
        disposition: AssumptionDisposition::SupportedByEvidence,
        evidence_digest: SENSITIVITY_DIGEST.to_string(),
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

fn input(partial_support: bool) -> CausalTransportInput {
    let target_strata = if partial_support {
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
    } else {
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
    };
    CausalTransportInput {
        source_data_digest: SOURCE_DIGEST.to_string(),
        target_population_digest: if partial_support {
            "sha256-target-population-with-unsupported-stratum".to_string()
        } else {
            TARGET_DIGEST.to_string()
        },
        protocol_digest: PROTOCOL_DIGEST.to_string(),
        assumptions: assumptions(),
        source_strata: source_strata(),
        target_strata,
    }
}

fn emit_result(
    scenario: &str,
    result: &CausalTransportSensitivityResult,
    tipping: Option<LambdaTippingPoint>,
) {
    for stratum in &result.strata {
        emit(vec![
            scenario.to_string(),
            format!("{:.8}", result.likelihood_ratio_bound),
            stratum.modifier_id.clone(),
            stratum.source_support.to_string(),
            stratum.target_population_count.to_string(),
            format!("{:.10}", stratum.target_population_share),
            fmt(stratum.control.as_ref().map(|arm| arm.source_risk)),
            fmt(stratum.control.as_ref().map(|arm| arm.target_risk_lower)),
            fmt(stratum.control.as_ref().map(|arm| arm.target_risk_upper)),
            fmt(stratum.treatment.as_ref().map(|arm| arm.source_risk)),
            fmt(stratum.treatment.as_ref().map(|arm| arm.target_risk_lower)),
            fmt(stratum.treatment.as_ref().map(|arm| arm.target_risk_upper)),
            fmt(stratum.risk_difference_lower),
            fmt(stratum.risk_difference_upper),
            format!("{:.10}", result.source_ate),
            fmt(result.target_ate_point),
            fmt(result.target_ate_lower),
            fmt(result.target_ate_upper),
            fmt(result.sensitivity_interval_width),
            result.identification_status.name().to_string(),
            result.identification_reason.name().to_string(),
            result.source_population_total.to_string(),
            result.target_population_total.to_string(),
            result.supported_target_population_count.to_string(),
            result.unsupported_target_population_count.to_string(),
            format!("{:.10}", result.supported_target_mass),
            format!("{:.10}", result.unsupported_target_mass),
            result
                .sensitivity_evidence
                .disposition
                .name()
                .to_string(),
            result.sensitivity_scenario_digest.clone(),
            result.source_data_digest.clone(),
            result.target_population_digest.clone(),
            result.protocol_digest.clone(),
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
            result
                .assumptions
                .no_interference
                .disposition
                .name()
                .to_string(),
            result
                .assumptions
                .treatment_version_invariance
                .disposition
                .name()
                .to_string(),
            fmt(tipping.map(|value| value.lambda)),
            fmt(tipping.map(|value| value.lower_bracket)),
            fmt(tipping.map(|value| value.upper_bracket)),
            fmt(tipping.map(|value| value.tolerance)),
            result.estimand.to_string(),
        ]);
    }
}

fn emit_scenario(
    scenario: &str,
    candidate: &CausalTransportInput,
) -> Result<(), Box<dyn std::error::Error>> {
    let tipping = find_null_tipping_point(
        candidate,
        &sensitivity_evidence(),
        8.0,
        1e-8,
    )?;
    for lambda in LAMBDA_GRID {
        let result = assess_bernoulli_outcome_shift_transport(
            candidate,
            lambda,
            &sensitivity_evidence(),
        )?;
        emit_result(scenario, &result, tipping);
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "scenario,lambda,modifier_id,source_support,target_population_count,target_population_share,control_source_risk,control_target_lower,control_target_upper,treatment_source_risk,treatment_target_lower,treatment_target_upper,stratum_risk_difference_lower,stratum_risk_difference_upper,source_ate,target_ate_point,target_ate_lower,target_ate_upper,interval_width,identification_status,identification_reason,source_population_total,target_population_total,supported_target_population_count,unsupported_target_population_count,supported_target_mass,unsupported_target_mass,sensitivity_evidence_disposition,sensitivity_scenario_digest,source_data_digest,target_population_digest,protocol_digest,source_internal_validity,selection_exchangeability,no_interference,treatment_version_invariance,tipping_lambda,tipping_lower_bracket,tipping_upper_bracket,tipping_tolerance,estimand"
    );
    emit_scenario("full-support", &input(false))?;
    emit_scenario("target-support-failure", &input(true))?;
    Ok(())
}
