// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic THEMP passport conformance sweep. No empirical data are imported.

use civdyn_themp_passport::{
    ConformanceReport, EvidenceClass, ModelPassport, evaluate_passport, reference_passport,
};

const CSV_COLUMNS: usize = 14;

fn csv_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "THEMP passport CSV schema must remain fixed"
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

fn scenarios() -> Vec<(&'static str, ModelPassport)> {
    let reference = reference_passport();

    let mut leakage = reference.clone();
    leakage.evidence_uses[2].partition_id = "calibration_wave_01".into();

    let mut unoperationalized = reference.clone();
    unoperationalized.constructs[0]
        .operationalization_ids
        .clear();

    let mut unfalsifiable = reference.clone();
    unfalsifiable.mechanisms[0].falsifier.clear();

    let mut non_discriminating_alternative = reference.clone();
    non_discriminating_alternative.alternatives[0].discriminating_predictions[0]
        .expected_direction = civdyn_themp_passport::ExpectedDirection::Positive;

    let mut inference_as_validation = reference.clone();
    inference_as_validation.evidence_uses[2].evidence_class = EvidenceClass::ModelInference;

    vec![
        ("reference", reference),
        ("calibration_holdout_leakage", leakage),
        ("unoperationalized_construct", unoperationalized),
        ("mechanism_without_falsifier", unfalsifiable),
        (
            "non_discriminating_alternative",
            non_discriminating_alternative,
        ),
        (
            "model_inference_as_external_validation",
            inference_as_validation,
        ),
    ]
}

fn summary_row(scenario_id: &str, report: &ConformanceReport) {
    emit(vec![
        "summary".into(),
        scenario_id.into(),
        report.passport_id.clone(),
        report.purpose.name().into(),
        String::new(),
        if report.has_errors() {
            "error".into()
        } else {
            "none".into()
        },
        String::new(),
        String::new(),
        report.error_count().to_string(),
        report.warning_count().to_string(),
        report.checked_constructs.to_string(),
        report.checked_indicators.to_string(),
        report.checked_mechanisms.to_string(),
        report.checked_alternatives.to_string(),
    ]);
}

fn finding_row(scenario_id: &str, report: &ConformanceReport, index: usize) {
    let finding = &report.findings[index];
    emit(vec![
        "finding".into(),
        scenario_id.into(),
        report.passport_id.clone(),
        report.purpose.name().into(),
        finding.code.into(),
        finding.severity.name().into(),
        finding.subject_id.clone(),
        finding.detail.clone(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]);
}

fn main() {
    println!(
        "record_type,scenario_id,passport_id,model_purpose,finding_code,severity,subject_id,detail,error_count,warning_count,checked_constructs,checked_indicators,checked_mechanisms,checked_alternatives"
    );

    for (scenario_id, passport) in scenarios() {
        let report = evaluate_passport(&passport).expect("synthetic passport ID must be valid");
        summary_row(scenario_id, &report);
        for index in 0..report.findings.len() {
            finding_row(scenario_id, &report, index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_reference_scenarios_are_present() {
        assert_eq!(scenarios().len(), 6);
    }

    #[test]
    fn reference_passport_has_no_errors() {
        let report = evaluate_passport(&reference_passport()).unwrap();
        assert!(!report.has_errors());
        assert_eq!(report.error_count(), 0);
    }

    #[test]
    fn adversarial_scenarios_trigger_their_declared_controls() {
        let values = scenarios();
        let expected = [
            (
                "calibration_holdout_leakage",
                "calibration_external_partition_leakage",
            ),
            (
                "unoperationalized_construct",
                "construct_not_operationalized",
            ),
            (
                "mechanism_without_falsifier",
                "mechanism_explanation_incomplete",
            ),
            (
                "model_inference_as_external_validation",
                "non_empirical_external_validation",
            ),
        ];
        for (scenario_id, code) in expected {
            let passport = values
                .iter()
                .find(|(id, _)| *id == scenario_id)
                .unwrap()
                .1
                .clone();
            let report = evaluate_passport(&passport).unwrap();
            assert!(report.findings.iter().any(|finding| finding.code == code));
        }
    }

    #[test]
    fn output_records_preserve_the_fourteen_column_contract() {
        assert_eq!(CSV_COLUMNS, 14);
        let report = evaluate_passport(&reference_passport()).unwrap();
        assert_eq!(report.checked_indicators, 2);
    }
}
