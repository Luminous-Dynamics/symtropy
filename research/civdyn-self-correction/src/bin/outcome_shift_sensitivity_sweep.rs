// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::causal_transport::{
    AssumptionDisposition, AssumptionEvidence,
};
use civdyn_self_correction::outcome_shift_sensitivity::{
    assess_outcome_shift_frontier, OutcomeShiftAssumptions, OutcomeShiftSensitivityInput,
    OutcomeShiftSensitivityResult, TreatmentArm, WeightedOutcomeObservation,
};

const CSV_COLUMNS: usize = 35;
const LAMBDA_GRID: [f64; 5] = [1.0, 1.5, 2.0, 3.0, 5.0];

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
        "outcome-shift sensitivity receipt must match the frozen schema"
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

fn evidence(disposition: AssumptionDisposition, digest: &str) -> AssumptionEvidence {
    AssumptionEvidence {
        disposition,
        evidence_digest: digest.to_string(),
    }
}

fn fixture() -> OutcomeShiftSensitivityInput {
    OutcomeShiftSensitivityInput {
        source_data_digest: "sha256-outcome-shift-source-rct".to_string(),
        target_population_digest: "sha256-outcome-shift-target-population".to_string(),
        protocol_digest: "sha256-outcome-shift-protocol-v0.1".to_string(),
        weight_estimator_digest: "sha256-generalization-weight-estimator-v0.1".to_string(),
        measurement_unit: "risk_difference".to_string(),
        outcome_support_lower: 0.0,
        outcome_support_upper: 1.0,
        assumptions: OutcomeShiftAssumptions {
            source_randomization: evidence(
                AssumptionDisposition::SupportedByDesign,
                "sha256-source-randomization-evidence",
            ),
            covariate_overlap: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-target-covariate-overlap-evidence",
            ),
            no_interference: evidence(
                AssumptionDisposition::SupportedByDesign,
                "sha256-no-interference-evidence",
            ),
            treatment_version_invariance: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-treatment-version-evidence",
            ),
            outcome_measurement_invariance: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-outcome-measurement-evidence",
            ),
        },
        observations: vec![
            WeightedOutcomeObservation {
                observation_id: "control-low".to_string(),
                arm: TreatmentArm::Control,
                outcome: 0.1,
                generalization_weight: 1.0,
            },
            WeightedOutcomeObservation {
                observation_id: "control-high".to_string(),
                arm: TreatmentArm::Control,
                outcome: 0.4,
                generalization_weight: 1.0,
            },
            WeightedOutcomeObservation {
                observation_id: "treatment-low".to_string(),
                arm: TreatmentArm::Treatment,
                outcome: 0.2,
                generalization_weight: 1.0,
            },
            WeightedOutcomeObservation {
                observation_id: "treatment-high".to_string(),
                arm: TreatmentArm::Treatment,
                outcome: 0.8,
                generalization_weight: 1.0,
            },
        ],
    }
}

fn optional_number(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.12}"))
        .unwrap_or_default()
}

fn emit_result(
    scenario: &str,
    result: &OutcomeShiftSensitivityResult,
    sign_tipping_point: Option<f64>,
    assumptions: &OutcomeShiftAssumptions,
) {
    emit(vec![
        scenario.to_string(),
        format!("{:.6}", result.sensitivity_lambda),
        result.measurement_unit.clone(),
        result.control.observation_count.to_string(),
        result.treatment.observation_count.to_string(),
        format!("{:.12}", result.control.baseline_weighted_mean),
        format!("{:.12}", result.control.lower_mean),
        format!("{:.12}", result.control.upper_mean),
        format!("{:.12}", result.control.effective_sample_size),
        format!("{:.12}", result.treatment.baseline_weighted_mean),
        format!("{:.12}", result.treatment.lower_mean),
        format!("{:.12}", result.treatment.upper_mean),
        format!("{:.12}", result.treatment.effective_sample_size),
        format!("{:.12}", result.baseline_weighted_ate),
        format!("{:.12}", result.target_ate_lower),
        format!("{:.12}", result.target_ate_upper),
        result.effect_direction.name().to_string(),
        optional_number(sign_tipping_point),
        result.source_data_digest.clone(),
        result.target_population_digest.clone(),
        result.protocol_digest.clone(),
        result.weight_estimator_digest.clone(),
        format!("{:.12}", result.outcome_support_lower),
        format!("{:.12}", result.outcome_support_upper),
        assumptions.source_randomization.disposition.name().to_string(),
        assumptions.covariate_overlap.disposition.name().to_string(),
        assumptions.no_interference.disposition.name().to_string(),
        assumptions
            .treatment_version_invariance
            .disposition
            .name()
            .to_string(),
        assumptions
            .outcome_measurement_invariance
            .disposition
            .name()
            .to_string(),
        assumptions.source_randomization.evidence_digest.clone(),
        assumptions.covariate_overlap.evidence_digest.clone(),
        assumptions.no_interference.evidence_digest.clone(),
        assumptions
            .treatment_version_invariance
            .evidence_digest
            .clone(),
        assumptions
            .outcome_measurement_invariance
            .evidence_digest
            .clone(),
        result.estimand.to_string(),
    ]);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = fixture();
    let frontier = assess_outcome_shift_frontier(&input, &LAMBDA_GRID)?;

    println!(
        "scenario,lambda,measurement_unit,control_n,treatment_n,control_baseline_mean,control_lower_mean,control_upper_mean,control_effective_sample_size,treatment_baseline_mean,treatment_lower_mean,treatment_upper_mean,treatment_effective_sample_size,baseline_weighted_ate,target_ate_lower,target_ate_upper,effect_direction,sign_tipping_point,source_data_digest,target_population_digest,protocol_digest,weight_estimator_digest,outcome_support_lower,outcome_support_upper,source_randomization,covariate_overlap,no_interference,treatment_version_invariance,outcome_measurement_invariance,source_randomization_evidence_digest,covariate_overlap_evidence_digest,no_interference_evidence_digest,treatment_version_evidence_digest,outcome_measurement_evidence_digest,estimand"
    );

    for result in &frontier.results {
        emit_result(
            "binary-risk-difference-known-answer",
            result,
            frontier.sign_tipping_point,
            &input.assumptions,
        );
    }
    Ok(())
}
