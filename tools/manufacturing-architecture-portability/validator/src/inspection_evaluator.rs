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
        ("cycle_lot_id", "qif_lot_id", "LOT_ID_MISMATCH"),
        ("cycle_operation_id", "qif_operation_id", "OPERATION_ID_MISMATCH"),
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
    match (association_start, association_end, measured_at, decision_time) {
        (Some(start), Some(end), Some(measured), Some(decision))
            if start <= end && start <= measured && measured <= end && start <= decision && decision <= end => {}
        _ => violations.push(JoinViolation {
            code: "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION",
            field: "association_valid_from_ns",
        }),
    }

    match (
        integer_at(candidate, "combined_clock_uncertainty_ns"),
        integer_at(candidate, "max_admitted_combined_clock_uncertainty_ns"),
    ) {
        (Some(observed), Some(maximum)) if observed <= maximum => {}
        _ => violations.push(JoinViolation {
            code: "CLOCK_UNCERTAINTY_EXCEEDS_ADMITTED_BOUND",
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

    InspectionJoinEvaluation {
        admissible_for_declared_synthetic_join: violations.is_empty(),
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const SCHEMA: &str = include_str!("../../inspection-join.schema.json");
    const CANDIDATE: &str =
        include_str!("../tests/fixtures/qif-cycle-join.candidate.json");

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
        assert!(result.violations.is_empty());
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
        assert!(result.violations.iter().any(|v| v.code == "INSPECTION_RESULT_NOT_MEASURED"));
    }

    #[test]
    fn rejects_measurement_outside_cycle_time_window() {
        let mut actual = candidate();
        actual["measured_at_reference_ns"] = json!(1750000700_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| v.code == "MEASUREMENT_OUTSIDE_CYCLE"));
    }

    #[test]
    fn rejects_expired_or_not_yet_valid_association() {
        let mut actual = candidate();
        actual["association_valid_until_ns"] = json!(1750000500_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| {
            v.code == "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION"
        }));
    }

    #[test]
    fn rejects_mismatched_clock_domain() {
        let mut actual = candidate();
        actual["qif_reference_clock_id"] = json!("CMM-LOCAL-BOOT-7");
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| v.code == "REFERENCE_CLOCK_MISMATCH"));
    }

    #[test]
    fn rejects_clock_uncertainty_above_declared_bound() {
        let mut actual = candidate();
        actual["combined_clock_uncertainty_ns"] = json!(501);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| {
            v.code == "CLOCK_UNCERTAINTY_EXCEEDS_ADMITTED_BOUND"
        }));
    }

    #[test]
    fn rejects_invalid_or_missing_original_unit_provenance() {
        let mut actual = candidate();
        actual.as_object_mut().unwrap().remove("original_unit");
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| {
            v.code == "ORIGINAL_OR_DERIVED_MEASUREMENT_PROVENANCE_MISSING"
        }));
    }

    #[test]
    fn projection_schema_rejects_missing_receipt_digest_and_unknown_root_fields() {
        let mut missing_digest = candidate();
        missing_digest.as_object_mut().unwrap().remove("association_receipt_sha256");
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
        assert!(result.violations.iter().any(|v| {
            v.code == "ASSOCIATION_NOT_VALID_AT_MEASUREMENT_AND_DECISION"
        }));
    }

    #[test]
    fn malformed_cycle_interval_is_rejected() {
        let mut actual = candidate();
        actual["cycle_start_ns"] = json!(1750000700_u64);
        actual["cycle_end_ns"] = json!(1750000600_u64);
        let result = evaluate_inspection_join(&actual);
        assert!(result.violations.iter().any(|v| v.code == "INVALID_CYCLE_INTERVAL"));
    }
}
