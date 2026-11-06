//! Evidence-qualified pairwise comparison for public mining performance claims.
//!
//! This crate refuses to rank metrics with mismatched definitions, denominators,
//! operating boundaries or task/geology/geometry context. It does not estimate
//! product performance, normalize incomparable quantities or create a composite
//! industry score.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairReport {
    pub pair_id: String,
    pub record_a_id: String,
    pub record_b_id: String,
    pub comparison_status: String,
    pub outcome: String,
    pub reason_codes: Vec<String>,
    pub difference_lower: Option<f64>,
    pub difference_upper: Option<f64>,
    pub source_disposition: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub comparison_profile_ref: String,
    pub record_count: usize,
    pub pair_count: usize,
    pub comparable_pair_count: usize,
    pub insufficient_context_pair_count: usize,
    pub non_comparable_pair_count: usize,
    pub definite_winner_count: usize,
    pub pair_reports: Vec<PairReport>,
    pub pooled_ranking: String,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn valid_source_class(value: &str) -> bool {
    matches!(
        value,
        "INDEPENDENT_FIELD_TRIAL"
            | "PEER_REVIEWED_TRIAL"
            | "OPERATOR_REPORTED"
            | "MANUFACTURER_PUBLISHED"
            | "SECONDARY_REPORT"
            | "MARKETING_CAPABILITY"
            | "SYNTHETIC_CONFORMANCE"
    )
}

fn comparison_fields() -> [&'static str; 12] {
    [
        "metric_definition_ref",
        "unit",
        "numerator_basis_ref",
        "denominator_basis_ref",
        "operating_boundary_ref",
        "task_profile_ref",
        "geology_profile_ref",
        "geometry_profile_ref",
        "acceptance_profile_ref",
        "comparison_protocol_ref",
        "machine_configuration_class_ref",
    ]
}

/// Validate records and the requested pairwise comparison set.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();
    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("PUBLIC_EVIDENCE_WITH_SYNTHETIC_CONTROLS" | "SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence_class is outside v0.1 vocabulary");
    }

    let Some(profile) = case.get("comparison_profile") else {
        push(&mut errors, "MISSING_COMPARISON_PROFILE", "$.comparison_profile", "comparison profile is required");
        return errors;
    };
    if text(profile, "comparison_profile_ref").is_none()
        || text(profile, "decision_owner_ref").is_none()
    {
        push(&mut errors, "INCOMPLETE_COMPARISON_PROFILE", "$.comparison_profile", "profile and decision owner references are required");
    }
    if profile.get("no_implicit_normalization").and_then(Value::as_bool) != Some(true) {
        push(&mut errors, "IMPLICIT_NORMALIZATION_FORBIDDEN", "$.comparison_profile.no_implicit_normalization", "comparison profile must explicitly prohibit implicit normalization");
    }

    let Some(records) = case.get("records").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_RECORDS", "$.records", "records must be an array");
        return errors;
    };
    if records.len() < 2 {
        push(&mut errors, "INSUFFICIENT_RECORDS", "$.records", "at least two evidence records are required");
    }
    let mut record_ids = BTreeSet::new();
    let mut record_by_id: BTreeMap<String, &Value> = BTreeMap::new();

    for (index, record) in records.iter().enumerate() {
        let path = format!("$.records[{index}]");
        let Some(id) = text(record, "record_id") else {
            push(&mut errors, "MISSING_RECORD_ID", &format!("{path}.record_id"), "record_id is required");
            continue;
        };
        if !record_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_RECORD_ID", &format!("{path}.record_id"), "record identifiers must be unique");
        }
        record_by_id.insert(id.to_owned(), record);

        for key in [
            "machine_model_ref",
            "manufacturer_ref",
            "display_name",
            "metric_family",
            "source_ref",
            "published_at",
            "source_class",
            "value_semantics",
            "uncertainty_status",
            "direction",
        ] {
            if text(record, key).is_none() {
                push(&mut errors, "MISSING_RECORD_PROVENANCE_OR_IDENTITY", &format!("{path}.{key}"), "machine, metric, source and reported-value semantics must be explicit");
            }
        }

        if !matches!(
            text(record, "metric_family"),
            Some("ADVANCE" | "VOLUME_RATE" | "DOWNTIME" | "POWER" | "ROCK_STRENGTH" | "ENERGY_INTENSITY" | "WEAR" | "AVAILABILITY" | "OTHER")
        ) {
            push(&mut errors, "UNKNOWN_METRIC_FAMILY", &format!("{path}.metric_family"), "metric family is outside v0.1 vocabulary");
        }
        if !matches!(
            text(record, "direction"),
            Some("HIGHER_IS_BETTER" | "LOWER_IS_BETTER" | "REFERENCE_ONLY")
        ) {
            push(&mut errors, "INVALID_METRIC_DIRECTION", &format!("{path}.direction"), "metric direction must be explicit");
        }
        if !matches!(
            text(record, "source_class"),
            Some(value) if valid_source_class(value)
        ) {
            push(&mut errors, "UNKNOWN_SOURCE_CLASS", &format!("{path}.source_class"), "source class is outside v0.1 vocabulary");
        }
        let lower = finite(record, "value_lower");
        let upper = match record.get("value_upper") {
            Some(Value::Null) => None,
            Some(value) => value.as_f64().filter(|v| v.is_finite()),
            None => {
                push(&mut errors, "MISSING_VALUE_UPPER", &format!("{path}.value_upper"), "upper value must be finite or explicitly null for an unbounded report");
                None
            }
        };
        match (lower, upper) {
            (Some(lo), Some(hi)) if hi >= lo => {}
            (Some(_), None) if record.get("value_upper") == Some(&Value::Null) => {}
            _ => push(&mut errors, "INVALID_VALUE_INTERVAL", &path, "value interval must be finite and ordered, or have explicit null upper bound for an unbounded lower-bound report"),
        }
        if text(record, "unit").is_none() {
            push(&mut errors, "MISSING_METRIC_UNIT", &format!("{path}.unit"), "metric unit is required for comparison");
        }
    }

    let Some(pairs) = case.get("requested_pairs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_REQUESTED_PAIRS", "$.requested_pairs", "requested_pairs must be an array");
        return errors;
    };
    if pairs.is_empty() {
        push(&mut errors, "EMPTY_REQUESTED_PAIRS", "$.requested_pairs", "at least one requested pair is required");
    }
    let mut pair_ids = BTreeSet::new();
    for (index, pair) in pairs.iter().enumerate() {
        let path = format!("$.requested_pairs[{index}]");
        let Some(id) = text(pair, "pair_id") else {
            push(&mut errors, "MISSING_PAIR_ID", &format!("{path}.pair_id"), "pair_id is required");
            continue;
        };
        if !pair_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_PAIR_ID", &format!("{path}.pair_id"), "pair identifiers must be unique");
        }
        let a = text(pair, "record_a_id");
        let b = text(pair, "record_b_id");
        if a.is_none() || b.is_none() || a == b {
            push(&mut errors, "INVALID_PAIR_MEMBERS", &path, "pair requires two different record IDs");
        }
        for record_id in [a, b].into_iter().flatten() {
            if !record_by_id.contains_key(record_id) {
                push(&mut errors, "UNKNOWN_PAIR_RECORD", &path, format!("pair references undeclared record {record_id}"));
            }
        }
    }

    errors
}

fn pair_source_disposition(a: &Value, b: &Value) -> String {
    let a_class = text(a, "source_class").unwrap_or("UNKNOWN");
    let b_class = text(b, "source_class").unwrap_or("UNKNOWN");
    if a_class == "SYNTHETIC_CONFORMANCE" && b_class == "SYNTHETIC_CONFORMANCE" {
        "SYNTHETIC_ARITHMETIC_ONLY".to_owned()
    } else if a_class == "SECONDARY_REPORT" || b_class == "SECONDARY_REPORT" {
        "PUBLIC_EVIDENCE_WITH_SECONDARY_SOURCE".to_owned()
    } else {
        "PUBLIC_EVIDENCE_SOURCE_ATTRIBUTED".to_owned()
    }
}

fn compare_records(
    pair_id: &str,
    a_id: &str,
    b_id: &str,
    a: &Value,
    b: &Value,
) -> PairReport {
    let source_disposition = pair_source_disposition(a, b);
    let mut reasons = Vec::new();

    if text(a, "trial_id").is_some() && text(a, "trial_id") == text(b, "trial_id") {
        reasons.push("SAME_TRIAL_NOT_INDEPENDENT".to_owned());
    }

    if text(a, "metric_family") != text(b, "metric_family") {
        reasons.push("METRIC_FAMILY_MISMATCH".to_owned());
    }
    if text(a, "direction") != text(b, "direction") {
        reasons.push("DIRECTION_MISMATCH".to_owned());
    }
    if text(a, "direction") == Some("REFERENCE_ONLY")
        || text(b, "direction") == Some("REFERENCE_ONLY")
    {
        reasons.push("REFERENCE_ONLY_METRIC".to_owned());
    }

    let mut missing_context = false;
    for key in comparison_fields() {
        match (text(a, key), text(b, key)) {
            (Some(left), Some(right)) if left != right => {
                reasons.push(format!("{}_MISMATCH", key.to_ascii_uppercase()));
            }
            (Some(_), Some(_)) => {}
            _ => {
                missing_context = true;
                reasons.push(format!("{}_UNKNOWN", key.to_ascii_uppercase()));
            }
        }
    }

    let status;
    let outcome;
    let mut difference_lower = None;
    let mut difference_upper = None;

    if reasons.iter().any(|reason| {
        reason.ends_with("_MISMATCH")
            || reason == "SAME_TRIAL_NOT_INDEPENDENT"
            || reason == "REFERENCE_ONLY_METRIC"
    }) {
        status = "NON_COMPARABLE";
        outcome = "NO_RANKING";
    } else if missing_context {
        status = "INSUFFICIENT_CONTEXT";
        outcome = "NO_RANKING";
    } else {
        status = "COMPARABLE";
        match (
            finite(a, "value_lower"),
            a.get("value_upper").and_then(Value::as_f64),
            finite(b, "value_lower"),
            b.get("value_upper").and_then(Value::as_f64),
        ) {
            (Some(a_lo), Some(a_hi), Some(b_lo), Some(b_hi)) => {
                difference_lower = Some(a_lo - b_hi);
                difference_upper = Some(a_hi - b_lo);
                let direction = text(a, "direction").expect("validated direction");
                outcome = match direction {
                    "HIGHER_IS_BETTER" if a_lo > b_hi => "A_DEFINITELY_BETTER",
                    "HIGHER_IS_BETTER" if b_lo > a_hi => "B_DEFINITELY_BETTER",
                    "LOWER_IS_BETTER" if a_hi < b_lo => "A_DEFINITELY_BETTER",
                    "LOWER_IS_BETTER" if b_hi < a_lo => "B_DEFINITELY_BETTER",
                    _ => "NOT_RESOLVED",
                };
            }
            _ => {
                outcome = "NOT_RESOLVED";
                reasons.push("UNBOUNDED_VALUE_INTERVAL".to_owned());
            }
        }
    }

    PairReport {
        pair_id: pair_id.to_owned(),
        record_a_id: a_id.to_owned(),
        record_b_id: b_id.to_owned(),
        comparison_status: status.to_owned(),
        outcome: outcome.to_owned(),
        reason_codes: reasons,
        difference_lower,
        difference_upper,
        source_disposition,
    }
}

/// Evaluate explicitly requested pairs without cross-normalizing metric families.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let records = case["records"].as_array().expect("validated records");
    let record_by_id: BTreeMap<&str, &Value> = records
        .iter()
        .map(|record| {
            (
                text(record, "record_id").expect("validated record ID"),
                record,
            )
        })
        .collect();

    let mut reports = Vec::new();
    for pair in case["requested_pairs"]
        .as_array()
        .expect("validated pairs")
    {
        let a_id = text(pair, "record_a_id").expect("validated record A");
        let b_id = text(pair, "record_b_id").expect("validated record B");
        reports.push(compare_records(
            text(pair, "pair_id").expect("validated pair ID"),
            a_id,
            b_id,
            record_by_id[a_id],
            record_by_id[b_id],
        ));
    }

    let comparable = reports
        .iter()
        .filter(|report| report.comparison_status == "COMPARABLE")
        .count();
    let insufficient = reports
        .iter()
        .filter(|report| report.comparison_status == "INSUFFICIENT_CONTEXT")
        .count();
    let non_comparable = reports
        .iter()
        .filter(|report| report.comparison_status == "NON_COMPARABLE")
        .count();
    let winners = reports
        .iter()
        .filter(|report| {
            matches!(
                report.outcome.as_str(),
                "A_DEFINITELY_BETTER" | "B_DEFINITELY_BETTER"
            )
        })
        .count();

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        comparison_profile_ref: text(&case["comparison_profile"], "comparison_profile_ref")
            .expect("validated comparison profile")
            .to_owned(),
        record_count: records.len(),
        pair_count: reports.len(),
        comparable_pair_count: comparable,
        insufficient_context_pair_count: insufficient,
        non_comparable_pair_count: non_comparable,
        definite_winner_count: winners,
        pair_reports: reports,
        pooled_ranking: "PROHIBITED_NO_CROSS_FAMILY_OR_UNMATCHED_CONTEXT_RANKING".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/public-industry-comparison-case.v0_1.json"
        ))
        .expect("fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn pair<'a>(report: &'a EvaluationReport, id: &str) -> &'a PairReport {
        report
            .pair_reports
            .iter()
            .find(|pair| pair.pair_id == id)
            .unwrap()
    }

    #[test]
    fn public_fixture_is_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn public_figures_are_not_pooled_into_an_industry_ranking() {
        let report = evaluate_case(&fixture()).unwrap();
        assert_eq!(
            report.pooled_ranking,
            "PROHIBITED_NO_CROSS_FAMILY_OR_UNMATCHED_CONTEXT_RANKING"
        );
        assert_eq!(report.record_count, 13);
        assert_eq!(report.pair_count, 6);
    }

    #[test]
    fn shift_and_cutting_hour_are_non_comparable() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "shift-vs-cutting-hour");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.iter().any(|code| code == "UNIT_MISMATCH"));
        assert!(result.reason_codes.iter().any(|code| code == "DENOMINATOR_BASIS_REF_MISMATCH"));
    }

    #[test]
    fn cutting_rate_and_in_situ_handling_are_non_comparable() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "cutting-vs-in-situ-handling");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.contains(&"METRIC_DEFINITION_REF_MISMATCH".to_owned()));
    }

    #[test]
    fn ucs_and_advertised_hardness_capability_are_non_comparable() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "ucs-vs-advertised-hardness");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.contains(&"METRIC_DEFINITION_REF_MISMATCH".to_owned()));
    }

    #[test]
    fn secondary_report_of_same_trial_is_not_an_independent_competitor() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "same-trial-secondary-coverage");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.contains(&"SAME_TRIAL_NOT_INDEPENDENT".to_owned()));
    }

    #[test]
    fn missing_context_prevents_ranking() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "synthetic-incomplete-context");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.iter().any(|code| code.ends_with("_MISMATCH")));
    }

    #[test]
    fn synthetic_matched_pair_is_comparable_and_a_definitely_better() {
        let report = evaluate_case(&fixture()).unwrap();
        let result = pair(&report, "synthetic-matched-pair");
        assert_eq!(result.comparison_status, "COMPARABLE");
        assert_eq!(result.outcome, "A_DEFINITELY_BETTER");
        assert_eq!(result.difference_lower, Some(1.0));
        assert_eq!(result.difference_upper, Some(3.0));
        assert_eq!(result.source_disposition, "SYNTHETIC_ARITHMETIC_ONLY");
    }

    #[test]
    fn overlapping_value_intervals_do_not_create_a_winner() {
        let mut case = fixture();
        case["records"][12]["value_lower"] = serde_json::json!(8.5);
        case["records"][12]["value_upper"] = serde_json::json!(10.5);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(pair(&report, "synthetic-matched-pair").outcome, "NOT_RESOLVED");
    }

    #[test]
    fn incomplete_value_upper_bound_does_not_create_a_winner() {
        let mut case = fixture();
        case["records"][12]["value_upper"] = Value::Null;
        let report = evaluate_case(&case).unwrap();
        let result = pair(&report, "synthetic-matched-pair");
        assert_eq!(result.comparison_status, "COMPARABLE");
        assert_eq!(result.outcome, "NOT_RESOLVED");
        assert!(result.reason_codes.contains(&"UNBOUNDED_VALUE_INTERVAL".to_owned()));
    }

    #[test]
    fn same_metric_with_missing_geology_context_is_insufficient() {
        let mut case = fixture();
        for index in [11usize, 12usize] {
            case["records"][index]["geology_profile_ref"] = Value::Null;
        }
        let report = evaluate_case(&case).unwrap();
        let result = pair(&report, "synthetic-matched-pair");
        assert_eq!(result.comparison_status, "INSUFFICIENT_CONTEXT");
        assert!(result.reason_codes.contains(&"geology_profile_ref_UNKNOWN".to_owned()));
    }

    #[test]
    fn lower_is_better_comparison_reverses_winner_direction() {
        let mut case = fixture();
        for index in [11usize, 12usize] {
            case["records"][index]["metric_family"] = serde_json::json!("ENERGY_INTENSITY");
            case["records"][index]["metric_definition_ref"] = serde_json::json!("energy-per-accepted-volume-v1");
            case["records"][index]["unit"] = serde_json::json!("kWh/m3");
            case["records"][index]["direction"] = serde_json::json!("LOWER_IS_BETTER");
            case["records"][index]["numerator_basis_ref"] = serde_json::json!("total-boundary-energy");
            case["records"][index]["denominator_basis_ref"] = serde_json::json!("accepted-solid-volume");
        }
        case["records"][11]["value_lower"] = serde_json::json!(2.0);
        case["records"][11]["value_upper"] = serde_json::json!(3.0);
        case["records"][12]["value_lower"] = serde_json::json!(4.0);
        case["records"][12]["value_upper"] = serde_json::json!(5.0);
        let report = evaluate_case(&case).unwrap();
        assert_eq!(pair(&report, "synthetic-matched-pair").outcome, "A_DEFINITELY_BETTER");
    }

    #[test]
    fn reference_only_strength_metrics_cannot_be_ranked() {
        let mut case = fixture();
        case["records"][11]["direction"] = serde_json::json!("REFERENCE_ONLY");
        case["records"][12]["direction"] = serde_json::json!("REFERENCE_ONLY");
        let report = evaluate_case(&case).unwrap();
        let result = pair(&report, "synthetic-matched-pair");
        assert_eq!(result.comparison_status, "NON_COMPARABLE");
        assert!(result.reason_codes.contains(&"REFERENCE_ONLY_METRIC".to_owned()));
    }

    #[test]
    fn reversed_value_interval_is_rejected() {
        let mut case = fixture();
        case["records"][11]["value_lower"] = serde_json::json!(20.0);
        case["records"][11]["value_upper"] = serde_json::json!(10.0);
        assert!(has(&validate_case(&case), "INVALID_VALUE_INTERVAL"));
    }

    #[test]
    fn duplicate_record_id_is_rejected() {
        let mut case = fixture();
        case["records"][1]["record_id"] = case["records"][0]["record_id"].clone();
        assert!(has(&validate_case(&case), "DUPLICATE_RECORD_ID"));
    }

    #[test]
    fn unknown_record_in_pair_is_rejected() {
        let mut case = fixture();
        case["requested_pairs"][0]["record_b_id"] = serde_json::json!("not-declared");
        assert!(has(&validate_case(&case), "UNKNOWN_PAIR_RECORD"));
    }

    #[test]
    fn implicit_normalization_must_be_prohibited() {
        let mut case = fixture();
        case["comparison_profile"]["no_implicit_normalization"] = serde_json::json!(false);
        assert!(has(&validate_case(&case), "IMPLICIT_NORMALIZATION_FORBIDDEN"));
    }

    #[test]
    fn power_is_reference_only_not_a_throughput_rank() {
        let mut case = fixture();
        case["requested_pairs"].as_array_mut().unwrap().push(serde_json::json!({
            "pair_id":"power-self-reference","record_a_id":"mc51-power-us","record_b_id":"mc51-power-us"
        }));
        assert!(has(&validate_case(&case), "INVALID_PAIR_MEMBERS"));
    }

    #[test]
    fn deterministic_pair_order_is_preserved() {
        let case = fixture();
        assert_eq!(
            evaluate_case(&case).unwrap(),
            evaluate_case(&case).unwrap()
        );
    }
}
