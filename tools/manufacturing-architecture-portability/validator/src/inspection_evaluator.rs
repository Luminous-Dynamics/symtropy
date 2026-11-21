// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent synthetic semantic checks for explicit machine-cycle to QIF joins.
//!
//! This module consumes a candidate projection. It does not parse MTConnect/QIF,
//! authenticate an association authority, compute clock transforms, qualify an
//! instrument, establish part conformity or authorize product release.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JoinViolation {
    pub code: &'static str,
    pub field: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionJoinEvaluation {
    pub admissible_for_declared_synthetic_join: bool,
    pub metrology_reference_state: &'static str,
    pub physical_metrology_qualified: bool,
    pub violations: Vec<JoinViolation>,
}

fn text_at<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn integer_at(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

/// Check explicit cycle/result joins and scoped time/uncertainty prerequisites.
///
/// This is an independent candidate-projection evaluator, not a QIF parser or
/// measurement-system authority. Receipt digests are checked structurally by the
/// JSON Schema; this function does not cryptographically authenticate their issuers.
pub fn evaluate_inspection_join(candidate: &Value) -> InspectionJoinEvaluation {
    let mut violations = Vec::new();
    let equalities = [
        ("cycle_part_id", "qif_part_id", "PART_ID_MISMATCH"),
        (
            "cycle_id",
            "association_cycle_id",
            "ASSOCIATION_CYCLE_ID_MISMATCH",
        ),
        (
            "qif_result_id",
            "association_qif_result_id",
            "ASSOCIATION_QIF_RESULT_ID_MISMATCH",
        ),
        (
            "cycle_part_id",
            "association_part_id",
            "ASSOCIATION_PART_ID_MISMATCH",
        ),
        (
            "cycle_lot_id",
            "association_lot_id",
            "ASSOCIATION_LOT_ID_MISMATCH",
        ),
        (
            "cycle_operation_id",
            "association_operation_id",
            "ASSOCIATION_OPERATION_ID_MISMATCH",
        ),
        (
            "cycle_source_document_sha256",
            "association_cycle_source_document_sha256",
            "ASSOCIATION_CYCLE_DOCUMENT_MISMATCH",
        ),
        (
            "mtconnect_document_sha256",
            "association_mtconnect_document_sha256",
            "ASSOCIATION_MTCONNECT_DOCUMENT_MISMATCH",
        ),
        (
            "qif_document_sha256",
            "association_qif_document_sha256",
            "ASSOCIATION_QIF_DOCUMENT_MISMATCH",
        ),
        ("cycle_lot_id", "qif_lot_id", "LOT_ID_MISMATCH"),
        (
            "cycle_operation_id",
            "qif_operation_id",
            "OPERATION_ID_MISMATCH",
        ),
        (
            "qif_characteristic_id",
            "association_characteristic_id",
            "CHARACTERISTIC_ID_MISMATCH",
        ),
        (
            "cycle_reference_clock_id",
            "qif_reference_clock_id",
            "REFERENCE_CLOCK_MISMATCH",
        ),
    ];
    for (left, right, code) in equalities {
        if text_at(candidate, left).is_none()
            || text_at(candidate, right).is_none()
            || text_at(candidate, left) != text_at(candidate, right)
        {
            violations.push(JoinViolation { code, field: right });
        }
    }

    if text_at(candidate, "inspection_result_status") != Some("MEASURED") {
        violations.push(JoinViolation {
            code: "INSPECTION_RESULT_NOT_MEASURED",
            field: "inspection_result_status",
        });
    }

    let cycle_start = integer_at(candidate, "cycle_start_ns");
    let cycle_end = integer_at(candidate, "cycle_end_ns");
    let measured_at = integer_at(candidate, "measured_at_reference_ns");
    match (cycle_start, cycle_end, measured_at) {
        (Some(start), Some(end), Some(measured)) if start <= measured && measured <= end => {}
        (Some(start), Some(end), _) if start > end => violations.push(JoinViolation {
            code: "INVALID_CYCLE_INTERVAL",
            field: "cycle_start_ns",
        }),
        _ => violations.push(JoinViolation {
            code: "MEASUREMENT_OUTSIDE_CYCLE",
            field: "measured_at_reference_ns",
        }),
    }

    let association_start = integer_at(candidate, "association_valid_from_ns");
    let association_end = integer_at(candidate, "association_valid_until_ns");
    let decision_time = integer_at(candidate, "decision_time_ns");
    match (
        association_start,
        association_end,
        measured_at,
        decision_time,
    ) {
        (Some(start), Some(end), Some(measured), Some(decision))
            if start <= end
                && start <= measured
                && measured <= end
                && start <= decision
                && decision <= end => {}
        _ => violations.push(JoinViolation {
            code: "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION",
            field: "association_valid_from_ns",
        }),
    }

    // Unknown correlation between clock transforms is handled conservatively:
    // add the two declared absolute uncertainty bounds rather than assuming
    // statistical independence and combining them by root-sum-square.
    let expected_combined = integer_at(candidate, "cycle_clock_uncertainty_ns")
        .zip(integer_at(candidate, "qif_clock_uncertainty_ns"))
        .and_then(|(cycle, qif)| cycle.checked_add(qif));
    match (
        expected_combined,
        integer_at(candidate, "combined_clock_uncertainty_ns"),
        integer_at(candidate, "max_admitted_combined_clock_uncertainty_ns"),
    ) {
        (Some(expected), Some(observed), Some(maximum))
            if observed == expected && observed <= maximum => {}
        _ => violations.push(JoinViolation {
            code: "CLOCK_UNCERTAINTY_NOT_CONSERVATIVELY_RECONSTRUCTED_OR_OVER_BOUND",
            field: "combined_clock_uncertainty_ns",
        }),
    }

    // Original inspection value and unit must remain available alongside the
    // derived conversion; a conversion may never overwrite source measurement.
    if candidate.get("actual_value").is_none()
        || text_at(candidate, "original_unit").is_none()
        || candidate.get("derived_value").is_none()
        || text_at(candidate, "derived_unit").is_none()
        || text_at(candidate, "unit_transform_revision").is_none()
    {
        violations.push(JoinViolation {
            code: "ORIGINAL_OR_DERIVED_MEASUREMENT_PROVENANCE_MISSING",
            field: "actual_value",
        });
    }

    let measurement_evidence_class = text_at(candidate, "measurement_evidence_class");
    if measurement_evidence_class != Some("synthetic") {
        violations.push(JoinViolation {
            code: "NON_SYNTHETIC_EVIDENCE_OUTSIDE_THIS_FIXTURE_PROFILE",
            field: "measurement_evidence_class",
        });
    }

    let calibration_reference_present = text_at(candidate, "instrument_calibration_disposition")
        == Some("qualified_reference")
        && text_at(candidate, "calibration_attestation_id").is_some()
        && text_at(candidate, "calibration_attestation_sha256").is_some();
    let uncertainty_reference_present = text_at(candidate, "measurement_uncertainty_disposition")
        == Some("qualified_reference")
        && text_at(candidate, "uncertainty_budget_id").is_some()
        && text_at(candidate, "uncertainty_budget_sha256").is_some();
    let calibration_applies = if calibration_reference_present {
        let interval_valid = integer_at(candidate, "calibration_valid_from_ns")
            .zip(integer_at(candidate, "calibration_valid_until_ns"))
            .is_some_and(|(start, end)| {
                start <= end
                    && integer_at(candidate, "measured_at_reference_ns")
                        .is_some_and(|time| start <= time && time <= end)
                    && integer_at(candidate, "decision_time_ns")
                        .is_some_and(|time| start <= time && time <= end)
            });
        let instrument_matches = text_at(candidate, "calibration_subject_instrument_id")
            == text_at(candidate, "instrument_resource_id");
        let clock_matches = text_at(candidate, "calibration_validity_clock_id")
            == text_at(candidate, "qif_reference_clock_id");
        if !interval_valid {
            violations.push(JoinViolation {
                code: "CALIBRATION_ATTESTATION_OUTSIDE_EFFECTIVITY",
                field: "calibration_valid_from_ns",
            });
        }
        if !instrument_matches {
            violations.push(JoinViolation {
                code: "CALIBRATION_ATTESTATION_WRONG_INSTRUMENT",
                field: "calibration_subject_instrument_id",
            });
        }
        if !clock_matches {
            violations.push(JoinViolation {
                code: "CALIBRATION_VALIDITY_CLOCK_MISMATCH",
                field: "calibration_validity_clock_id",
            });
        }
        interval_valid && instrument_matches && clock_matches
    } else {
        false
    };

    let uncertainty_applies = if uncertainty_reference_present {
        let interval_valid = integer_at(candidate, "uncertainty_valid_from_ns")
            .zip(integer_at(candidate, "uncertainty_valid_until_ns"))
            .is_some_and(|(start, end)| {
                start <= end
                    && integer_at(candidate, "measured_at_reference_ns")
                        .is_some_and(|time| start <= time && time <= end)
                    && integer_at(candidate, "decision_time_ns")
                        .is_some_and(|time| start <= time && time <= end)
            });
        let characteristic_matches = text_at(candidate, "uncertainty_characteristic_id")
            == text_at(candidate, "qif_characteristic_id");
        let unit_matches =
            text_at(candidate, "uncertainty_original_unit") == text_at(candidate, "original_unit");
        let clock_matches = text_at(candidate, "uncertainty_validity_clock_id")
            == text_at(candidate, "qif_reference_clock_id");
        if !interval_valid {
            violations.push(JoinViolation {
                code: "UNCERTAINTY_BUDGET_OUTSIDE_EFFECTIVITY",
                field: "uncertainty_valid_from_ns",
            });
        }
        if !characteristic_matches {
            violations.push(JoinViolation {
                code: "UNCERTAINTY_BUDGET_WRONG_CHARACTERISTIC",
                field: "uncertainty_characteristic_id",
            });
        }
        if !unit_matches {
            violations.push(JoinViolation {
                code: "UNCERTAINTY_BUDGET_WRONG_UNIT",
                field: "uncertainty_original_unit",
            });
        }
        if !clock_matches {
            violations.push(JoinViolation {
                code: "UNCERTAINTY_VALIDITY_CLOCK_MISMATCH",
                field: "uncertainty_validity_clock_id",
            });
        }
        interval_valid && characteristic_matches && unit_matches && clock_matches
    } else {
        false
    };

    let metrology_reference_state = match (
        calibration_reference_present,
        uncertainty_reference_present,
        calibration_applies,
        uncertainty_applies,
    ) {
        (true, true, true, true) => "applicability_matched_references_not_authenticated",
        (false, false, _, _) => "unknown_or_unqualified",
        _ => "partial_or_inapplicable_metrology_references",
    };

    InspectionJoinEvaluation {
        admissible_for_declared_synthetic_join: violations.is_empty(),
        metrology_reference_state,
        // This evaluator checks source-reference structure only. It never
        // authenticates the attester, calibration scope, uncertainty budget,
        // instrument applicability or independent qualification.
        physical_metrology_qualified: false,
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const SCHEMA: &str = include_str!("../../inspection-join.schema.json");
    const CANDIDATE: &str = include_str!("../tests/fixtures/qif-cycle-join.candidate.json");

    fn schema() -> Value {
        serde_json::from_str(SCHEMA).expect("join schema JSON must parse")
    }

    fn candidate() -> Value {
        serde_json::from_str(CANDIDATE).expect("join candidate JSON must parse")
    }

    fn validate_schema(value: &Value) -> bool {
        let schema = schema();
        jsonschema::draft202012::options()
            .should_validate_formats(true)
            .build(&schema)
            .expect("join projection validator must build")
            .is_valid(value)
    }

    #[test]
    fn join_schema_is_valid_draft_2020_12_and_accepts_candidate() {
        let schema = schema();
        jsonschema::draft202012::meta::validate(&schema)
            .expect("inspection join schema must be valid Draft 2020-12");
        assert!(validate_schema(&candidate()));
    }

    #[test]
    fn exact_explicit_cycle_qif_join_is_admissible_for_fixture() {
        let result = evaluate_inspection_join(&candidate());
        assert!(result.admissible_for_declared_synthetic_join);
        assert_eq!(result.metrology_reference_state, "unknown_or_unqualified");
        assert!(!result.physical_metrology_qualified);
        assert!(result.violations.is_empty());
    }

    #[test]
    fn qualified_metrology_references_are_reported_but_never_authenticated() {
        let mut actual = candidate();
        actual["instrument_calibration_disposition"] = json!("qualified_reference");
        actual["calibration_attestation_id"] = json!("CAL-001");
        actual["calibration_attestation_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        actual["calibration_subject_instrument_id"] = json!("CMM-01");
        actual["calibration_valid_from_ns"] = json!(1750000000_u64);
        actual["calibration_valid_until_ns"] = json!(1750001000_u64);
        actual["calibration_validity_clock_id"] = json!("PLANT-UTC-001");
        actual["measurement_uncertainty_disposition"] = json!("qualified_reference");
        actual["uncertainty_budget_id"] = json!("MSA-001");
        actual["uncertainty_budget_sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        actual["uncertainty_characteristic_id"] = json!("SHAFT-DIAMETER-01");
        actual["uncertainty_original_unit"] = json!("mm");
        actual["uncertainty_valid_from_ns"] = json!(1750000000_u64);
        actual["uncertainty_valid_until_ns"] = json!(1750001000_u64);
        actual["uncertainty_validity_clock_id"] = json!("PLANT-UTC-001");
        assert!(validate_schema(&actual));
        let result = evaluate_inspection_join(&actual);
        assert!(result.admissible_for_declared_synthetic_join);
        assert_eq!(
            result.metrology_reference_state,
            "applicability_matched_references_not_authenticated"
        );
        assert!(!result.physical_metrology_qualified);
    }

    #[test]
    fn missing_metrology_reference_for_qualified_claim_fails_schema() {
        let mut actual = candidate();
        actual["instrument_calibration_disposition"] = json!("qualified_reference");
        assert!(!validate_schema(&actual));

        let mut uncertainty_missing = candidate();
        uncertainty_missing["measurement_uncertainty_disposition"] = json!("qualified_reference");
        assert!(!validate_schema(&uncertainty_missing));
    }

    #[test]
    fn non_synthetic_measurement_cannot_pass_this_synthetic_join_profile() {
        let mut actual = candidate();
        actual["measurement_evidence_class"] = json!("physical_observation");
        let result = evaluate_inspection_join(&actual);
        assert!(!result.admissible_for_declared_synthetic_join);
        assert!(
            result
                .violations
                .iter()
                .any(|v| { v.code == "NON_SYNTHETIC_EVIDENCE_OUTSIDE_THIS_FIXTURE_PROFILE" })
        );
    }

    #[test]
    fn qualified_references_must_match_exact_instrument_characteristic_unit_and_time() {
        let mut actual = candidate();
        actual["instrument_calibration_disposition"] = json!("qualified_reference");
        actual["calibration_attestation_id"] = json!("CAL-001");
        actual["calibration_attestation_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        actual["calibration_subject_instrument_id"] = json!("CMM-01");
        actual["calibration_valid_from_ns"] = json!(1750000000_u64);
        actual["calibration_valid_until_ns"] = json!(1750001000_u64);
        actual["calibration_validity_clock_id"] = json!("PLANT-UTC-001");

        actual["measurement_uncertainty_disposition"] = json!("qualified_reference");
        actual["uncertainty_budget_id"] = json!("MSA-001");
        actual["uncertainty_budget_sha256"] =
            json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        actual["uncertainty_characteristic_id"] = json!("SHAFT-DIAMETER-01");
        actual["uncertainty_original_unit"] = json!("mm");
        actual["uncertainty_valid_from_ns"] = json!(1750000000_u64);
        actual["uncertainty_valid_until_ns"] = json!(1750001000_u64);
        actual["uncertainty_validity_clock_id"] = json!("PLANT-UTC-001");

        assert!(validate_schema(&actual));
        let valid = evaluate_inspection_join(&actual);
        assert_eq!(
            valid.metrology_reference_state,
            "applicability_matched_references_not_authenticated"
        );
        assert!(!valid.physical_metrology_qualified);

        actual["calibration_subject_instrument_id"] = json!("OTHER-CMM");
        let wrong_instrument = evaluate_inspection_join(&actual);
        assert!(
            wrong_instrument
                .violations
                .iter()
                .any(|v| { v.code == "CALIBRATION_ATTESTATION_WRONG_INSTRUMENT" })
        );

        actual["calibration_subject_instrument_id"] = json!("CMM-01");
        actual["uncertainty_characteristic_id"] = json!("OTHER-CHAR");
        let wrong_characteristic = evaluate_inspection_join(&actual);
        assert!(
            wrong_characteristic
                .violations
                .iter()
                .any(|v| { v.code == "UNCERTAINTY_BUDGET_WRONG_CHARACTERISTIC" })
        );

        actual["uncertainty_characteristic_id"] = json!("SHAFT-DIAMETER-01");

        actual["calibration_validity_clock_id"] = json!("CMM-LOCAL-BOOT-7");
        let wrong_calibration_clock = evaluate_inspection_join(&actual);
        assert!(
            wrong_calibration_clock
                .violations
                .iter()
                .any(|v| { v.code == "CALIBRATION_VALIDITY_CLOCK_MISMATCH" })
        );

        actual["calibration_validity_clock_id"] = json!("PLANT-UTC-001");
        actual["uncertainty_validity_clock_id"] = json!("CMM-LOCAL-BOOT-7");
        let wrong_uncertainty_clock = evaluate_inspection_join(&actual);
        assert!(
            wrong_uncertainty_clock
                .violations
                .iter()
                .any(|v| { v.code == "UNCERTAINTY_VALIDITY_CLOCK_MISMATCH" })
        );

        actual["uncertainty_validity_clock_id"] = json!("PLANT-UTC-001");

        actual["uncertainty_original_unit"] = json!("um");
        let wrong_unit = evaluate_inspection_join(&actual);
        assert!(
            wrong_unit
                .violations
                .iter()
                .any(|v| { v.code == "UNCERTAINTY_BUDGET_WRONG_UNIT" })
        );

        actual["uncertainty_original_unit"] = json!("mm");
        actual["calibration_valid_until_ns"] = json!(1750000480_u64);
        let expired = evaluate_inspection_join(&actual);
        assert!(
            expired
                .violations
                .iter()
                .any(|v| { v.code == "CALIBRATION_ATTESTATION_OUTSIDE_EFFECTIVITY" })
        );
    }

    #[test]
    fn partial_metrology_reference_set_is_not_complete() {
        let mut actual = candidate();
        actual["instrument_calibration_disposition"] = json!("qualified_reference");
        actual["calibration_attestation_id"] = json!("CAL-001");
        actual["calibration_attestation_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        let result = evaluate_inspection_join(&actual);
        assert_eq!(
            result.metrology_reference_state,
            "partial_or_inapplicable_metrology_references"
        );
        assert!(!result.physical_metrology_qualified);
    }

    #[test]
    fn unknown_or_missing_metrology_remains_visible_without_promoting_physical_claim() {
        let mut actual = candidate();
        actual["measurement_evidence_class"] = json!("recorded");
        let result = evaluate_inspection_join(&actual);
        assert!(!result.admissible_for_declared_synthetic_join);
        assert_eq!(result.metrology_reference_state, "unknown_or_unqualified");
        assert!(!result.physical_metrology_qualified);
    }

    #[test]
    fn rejects_part_lot_operation_and_characteristic_mismatches() {
        for (field, wrong) in [
            ("qif_part_id", "PART-WRONG"),
            ("qif_lot_id", "LOT-WRONG"),
            ("qif_operation_id", "OP-WRONG"),
            ("association_characteristic_id", "CHAR-WRONG"),
        ] {
            let mut actual = candidate();
            actual[field] = json!(wrong);
            let result = evaluate_inspection_join(&actual);
            assert!(!result.admissible_for_declared_synthetic_join, "{field}");
        }
    }

    #[test]
    fn rejects_unmeasured_qif_result() {
        let mut actual = candidate();
        actual["inspection_result_status"] = json!("NOT_MEASURED");
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.code == "INSPECTION_RESULT_NOT_MEASURED")
        );
    }

    #[test]
    fn rejects_measurement_outside_cycle_time_window() {
        let mut actual = candidate();
        actual["measured_at_reference_ns"] = json!(1750000700_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.code == "MEASUREMENT_OUTSIDE_CYCLE")
        );
    }

    #[test]
    fn rejects_expired_or_not_yet_valid_association() {
        let mut actual = candidate();
        actual["association_valid_until_ns"] = json!(1750000500_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| { v.code == "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION" })
        );
    }

    #[test]
    fn rejects_mismatched_clock_domain() {
        let mut actual = candidate();
        actual["qif_reference_clock_id"] = json!("CMM-LOCAL-BOOT-7");
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.code == "REFERENCE_CLOCK_MISMATCH")
        );
    }

    #[test]
    fn rejects_clock_uncertainty_above_declared_bound() {
        let mut actual = candidate();
        actual["qif_clock_uncertainty_ns"] = json!(401);
        actual["combined_clock_uncertainty_ns"] = json!(501);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| {
            v.code == "CLOCK_UNCERTAINTY_NOT_CONSERVATIVELY_RECONSTRUCTED_OR_OVER_BOUND"
        }));
    }

    #[test]
    fn rejects_understated_combined_clock_uncertainty() {
        let mut actual = candidate();
        actual["combined_clock_uncertainty_ns"] = json!(150);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| {
            v.code == "CLOCK_UNCERTAINTY_NOT_CONSERVATIVELY_RECONSTRUCTED_OR_OVER_BOUND"
        }));
    }

    #[test]
    fn rejects_association_replayed_for_different_source_documents_or_result() {
        for (field, wrong) in [
            ("association_cycle_id", "CYCLE-OTHER"),
            ("association_qif_result_id", "QIF-RESULT-OTHER"),
            (
                "association_cycle_source_document_sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            (
                "association_mtconnect_document_sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            (
                "association_qif_document_sha256",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
        ] {
            let mut actual = candidate();
            actual[field] = json!(wrong);
            let result = evaluate_inspection_join(&actual);
            assert!(!result.admissible_for_declared_synthetic_join, "{field}");
        }
    }

    #[test]
    fn rejects_invalid_or_missing_original_unit_provenance() {
        let mut actual = candidate();
        actual.as_object_mut().unwrap().remove("original_unit");
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| { v.code == "ORIGINAL_OR_DERIVED_MEASUREMENT_PROVENANCE_MISSING" })
        );
    }

    #[test]
    fn projection_schema_rejects_missing_receipt_digest_and_unknown_root_fields() {
        let mut missing_digest = candidate();
        missing_digest
            .as_object_mut()
            .unwrap()
            .remove("association_receipt_sha256");
        assert!(!validate_schema(&missing_digest));

        let mut unknown = candidate();
        unknown["untyped_join_hint"] = json!("nearest timestamp");
        assert!(!validate_schema(&unknown));
    }

    #[test]
    fn decision_time_must_be_inside_association_effectivity() {
        let mut actual = candidate();
        actual["decision_time_ns"] = json!(1800000001_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| { v.code == "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION" })
        );
    }

    #[test]
    fn malformed_cycle_interval_is_rejected() {
        let mut actual = candidate();
        actual["cycle_start_ns"] = json!(1750000700_u64);
        actual["cycle_end_ns"] = json!(1750000600_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(
            result
                .violations
                .iter()
                .any(|v| v.code == "INVALID_CYCLE_INTERVAL")
        );
    }
}
