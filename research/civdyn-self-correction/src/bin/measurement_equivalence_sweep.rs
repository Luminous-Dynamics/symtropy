// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_self_correction::measurement_equivalence::{
    BridgeDesign, BridgeStudyEvidence, EvidenceDisposition, MeasurementClaim,
    MeasurementDefinition, MeasurementEquivalenceEvidence, MeasurementFamily,
    MeasurementInvarianceEvidence, MeasurementGroup, MeasurementEquivalenceReport,
    SameProtocolEvidence, SurveyInvarianceEvidence, assess_measurement_equivalence,
};

const CSV_COLUMNS: usize = 36;

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn optional<T: ToString>(value: Option<T>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "measurement-equivalence receipt must match the frozen schema"
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

fn measurement(
    id: &str,
    population: &str,
    context: &str,
    operationalization: &str,
    instrument: &str,
    family: MeasurementFamily,
) -> MeasurementDefinition {
    MeasurementDefinition {
        measurement_id: id.to_string(),
        construct_id: "household-service-security".to_string(),
        construct_version: "v1".to_string(),
        operationalization_digest: operationalization.to_string(),
        instrument_id: "service-security-instrument".to_string(),
        instrument_digest: instrument.to_string(),
        family,
        measurement_unit: "standardized_score".to_string(),
        population_id: population.to_string(),
        context_id: context.to_string(),
    }
}

fn group(population_id: &str, sample_size: u64) -> MeasurementGroup {
    MeasurementGroup {
        population_id: population_id.to_string(),
        sample_size,
    }
}

fn invariance(
    configural: EvidenceDisposition,
    metric: EvidenceDisposition,
    scalar: EvidenceDisposition,
    residual: EvidenceDisposition,
) -> SurveyInvarianceEvidence {
    SurveyInvarianceEvidence {
        study_digest: "sha256-invariance-study-v1".to_string(),
        data_digest: "sha256-invariance-data-v1".to_string(),
        protocol_digest: "sha256-invariance-protocol-v1".to_string(),
        analysis_digest: "sha256-invariance-analysis-v1".to_string(),
        minimum_group_sample_size: 100,
        groups: vec![group("population-a", 500), group("population-b", 450)],
        configural,
        metric,
        scalar,
        residual,
        anchor_item_ids: vec!["anchor-item-1".to_string(), "anchor-item-2".to_string()],
        non_invariant_item_ids: vec!["item-4".to_string()],
    }
}

fn bridge(population_invariance: EvidenceDisposition, wide_bias: bool) -> BridgeStudyEvidence {
    BridgeStudyEvidence {
        bridge_study_digest: "sha256-bridge-study-v1".to_string(),
        data_digest: "sha256-bridge-data-v1".to_string(),
        protocol_digest: "sha256-bridge-protocol-v1".to_string(),
        analysis_digest: "sha256-bridge-analysis-v1".to_string(),
        design: BridgeDesign::Paired,
        paired_sample_size: Some(200),
        minimum_group_sample_size: 100,
        groups: vec![group("population-a", 250), group("population-b", 240)],
        confidence_level: 0.95,
        mean_bias_ci_lower: if wide_bias { -0.20 } else { -0.04 },
        mean_bias_ci_upper: 0.03,
        mean_equivalence_margin: 0.10,
        slope_ci_lower: 0.94,
        slope_ci_upper: 1.05,
        slope_equivalence_margin: 0.10,
        individual_error_upper_bound: Some(0.08),
        individual_score_margin: Some(0.12),
        population_invariance,
    }
}

fn emit_report(scenario: &str, report: &MeasurementEquivalenceReport) {
    let group_sizes = report
        .groups
        .iter()
        .map(|group| format!("{}:{}", group.population_id, group.sample_size))
        .collect::<Vec<_>>()
        .join(";");
    let mean_bias_ci = report
        .mean_bias_ci
        .map(|(lower, upper)| format!("{lower:.6}:{upper:.6}"))
        .unwrap_or_default();
    let slope_ci = report
        .slope_ci
        .map(|(lower, upper)| format!("{lower:.6}:{upper:.6}"))
        .unwrap_or_default();

    for claim in &report.claims {
        emit(vec![
            scenario.to_string(),
            report.evidence_kind.to_string(),
            report.left_measurement_id.clone(),
            report.right_measurement_id.clone(),
            report.construct_id.clone(),
            report.construct_version.clone(),
            report.family.name().to_string(),
            report.measurement_unit.clone(),
            report.left_population_id.clone(),
            report.right_population_id.clone(),
            report.left_context_id.clone(),
            report.right_context_id.clone(),
            report.left_operationalization_digest.clone(),
            report.right_operationalization_digest.clone(),
            report.left_instrument_digest.clone(),
            report.right_instrument_digest.clone(),
            report.evidence_digest.clone(),
            report.data_digest.clone().unwrap_or_default(),
            report.protocol_digest.clone(),
            report.analysis_digest.clone().unwrap_or_default(),
            report
                .calibration_receipt_digest
                .clone()
                .unwrap_or_default(),
            report
                .bridge_design
                .map(|design| design.name().to_string())
                .unwrap_or_default(),
            optional(report.confidence_level.map(|value| format!("{value:.6}"))),
            claim.claim.name().to_string(),
            claim.status.name().to_string(),
            claim.reason.name().to_string(),
            report
                .achieved_invariance
                .map(|value| value.name().to_string())
                .unwrap_or_default(),
            report.anchor_item_ids.join(";"),
            report.non_invariant_item_ids.join(";"),
            group_sizes.clone(),
            mean_bias_ci.clone(),
            optional(report.mean_equivalence_margin.map(|value| format!("{value:.6}"))),
            slope_ci.clone(),
            optional(report.slope_equivalence_margin.map(|value| format!("{value:.6}"))),
            optional(report.individual_error_upper_bound.map(|value| format!("{value:.6}"))),
            optional(report.individual_score_margin.map(|value| format!("{value:.6}"))),
        ]);
    }
}

fn emit_case(
    scenario: &str,
    left: &MeasurementDefinition,
    right: &MeasurementDefinition,
    evidence: &MeasurementEquivalenceEvidence,
) {
    let report = assess_measurement_equivalence(left, right, evidence)
        .expect("frozen measurement-equivalence fixture must be valid");
    emit_report(scenario, &report);
}

fn main() {
    println!(
        "scenario,evidence_kind,left_measurement_id,right_measurement_id,construct_id,construct_version,family,measurement_unit,left_population_id,right_population_id,left_context_id,right_context_id,left_operationalization_digest,right_operationalization_digest,left_instrument_digest,right_instrument_digest,evidence_digest,data_digest,protocol_digest,analysis_digest,calibration_receipt_digest,bridge_design,confidence_level,claim,status,reason,achieved_invariance,anchor_item_ids,non_invariant_item_ids,group_sample_sizes,mean_bias_ci,mean_equivalence_margin,slope_ci,slope_equivalence_margin,individual_error_upper_bound,individual_score_margin"
    );

    let exact = measurement(
        "physical-meter-v1",
        "population-a",
        "same-context",
        "sha256-physical-protocol",
        "sha256-meter-instrument",
        MeasurementFamily::DirectPhysicalMeasure,
    );
    emit_case(
        "exact_same_protocol",
        &exact,
        &exact,
        &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
            protocol_digest: "sha256-exact-protocol".to_string(),
            calibration_receipt_digest: Some("sha256-physical-calibration-certificate".to_string()),
        }),
    );

    let survey_a = measurement(
        "survey-scale-a",
        "population-a",
        "baseline",
        "sha256-survey-op-a",
        "sha256-survey-instrument-a",
        MeasurementFamily::SurveyLatentScale,
    );
    let survey_b = measurement(
        "survey-scale-b",
        "population-b",
        "baseline",
        "sha256-survey-op-b",
        "sha256-survey-instrument-b",
        MeasurementFamily::SurveyLatentScale,
    );

    let same_protocol_cross_population = measurement(
        "survey-scale-a-copy",
        "population-b",
        "baseline",
        "sha256-survey-op-a",
        "sha256-survey-instrument-a",
        MeasurementFamily::SurveyLatentScale,
    );
    emit_case(
        "same_protocol_cross_population",
        &survey_a,
        &same_protocol_cross_population,
        &MeasurementEquivalenceEvidence::SameProtocol(SameProtocolEvidence {
            protocol_digest: "sha256-survey-protocol".to_string(),
            calibration_receipt_digest: None,
        }),
    );

    emit_case(
        "configural_only",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::NotAssessed,
            EvidenceDisposition::NotAssessed,
            EvidenceDisposition::NotAssessed,
        )),
    );
    emit_case(
        "metric_partial",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::NotSupported,
            EvidenceDisposition::NotAssessed,
        )),
    );
    emit_case(
        "scalar_partial",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::NotAssessed,
        )),
    );
    emit_case(
        "full_invariance",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::SurveyInvariance(invariance(
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
            EvidenceDisposition::Supported,
        )),
    );
    emit_case(
        "bridge_equivalent",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::BridgeStudy(bridge(
            EvidenceDisposition::Supported,
            false,
        )),
    );
    emit_case(
        "bridge_mean_inconclusive",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::BridgeStudy(bridge(
            EvidenceDisposition::Supported,
            true,
        )),
    );
    emit_case(
        "bridge_population_noninvariant",
        &survey_a,
        &survey_b,
        &MeasurementEquivalenceEvidence::BridgeStudy(bridge(
            EvidenceDisposition::NotSupported,
            false,
        )),
    );
}
