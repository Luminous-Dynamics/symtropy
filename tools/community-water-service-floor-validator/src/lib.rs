// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only evaluator for synthetic, non-compensatory community water service floors.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_ID: &str = "community-water-service-floor-v0.1";
const ORACLE_ID: &str = "community-water-service-floor-disposition-oracle-v0.1";
const EXPECTED_CASES: usize = 12;

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn nonempty(value: &Value, key: &str) -> bool {
    text(value, key).is_some_and(|v| !v.trim().is_empty())
}

fn push(errors: &mut Vec<String>, context: &str, message: &str) {
    errors.push(format!("{context}: {message}"));
}

fn evaluate_area(area: &Value, profile: &Value, context: &str, errors: &mut Vec<String>) -> String {
    if area.get("required").and_then(Value::as_bool) != Some(true) {
        push(errors, context, "required area flag must be true");
        return "invalid".to_owned();
    }

    let profile_metric = text(profile, "metric").unwrap_or("");
    let profile_unit = text(profile, "unit").unwrap_or("");
    let profile_grid = text(profile, "time_grid_id").unwrap_or("");
    let profile_floor = finite(profile, "minimum_floor").unwrap_or(f64::NAN);

    let floor_ref_ok = nonempty(area, "floor_ref");
    let metric_ok = text(area, "metric") == Some(profile_metric);
    let unit_ok = text(area, "unit") == Some(profile_unit);
    let floor = finite(area, "minimum_floor").unwrap_or(f64::NAN);
    let floor_ok = floor.is_finite() && (floor - profile_floor).abs() <= 1.0e-12;
    let grid_ok = text(area, "time_grid_id") == Some(profile_grid);
    let geometry_ok = nonempty(area, "geometry_revision_ref");
    let weight = finite(area, "diagnostic_weight").unwrap_or(f64::NAN);
    let weight_ok = weight.is_finite() && weight > 0.0;

    if !floor_ref_ok || !metric_ok || !unit_ok || !floor_ok || !grid_ok || !geometry_ok || !weight_ok {
        return "invalid".to_owned();
    }

    let evidence_state = text(area, "evidence_state").unwrap_or("");
    if !["observed_current", "missing", "stale", "conflicting", "unknown"]
        .contains(&evidence_state)
    {
        return "invalid".to_owned();
    }

    let numerator = match area.get("coverage_numerator").and_then(Value::as_u64) {
        Some(value) => value,
        None => return "invalid".to_owned(),
    };
    let denominator = match area.get("coverage_denominator") {
        Some(value) if value.is_null() => None,
        Some(value) => match value.as_u64() {
            Some(value) if value > 0 => Some(value),
            _ => return "invalid".to_owned(),
        },
        None => return "invalid".to_owned(),
    };
    if denominator.is_some_and(|value| numerator > value) {
        return "invalid".to_owned();
    }

    let lower = match area.get("observation_lower_bound") {
        None => {
            push(errors, context, "lower observation bound field is missing");
            return "invalid".to_owned();
        }
        Some(value) if value.is_null() => None,
        Some(value) => match value.as_f64() {
            Some(bound) if bound.is_finite() && (0.0..=1.0).contains(&bound) => Some(bound),
            _ => {
                push(errors, context, "lower observation bound must be finite and in [0, 1]");
                return "invalid".to_owned();
            }
        },
    };
    let upper = match area.get("observation_upper_bound") {
        None => {
            push(errors, context, "upper observation bound field is missing");
            return "invalid".to_owned();
        }
        Some(value) if value.is_null() => None,
        Some(value) => match value.as_f64() {
            Some(bound) if bound.is_finite() && (0.0..=1.0).contains(&bound) => Some(bound),
            _ => {
                push(errors, context, "upper observation bound must be finite and in [0, 1]");
                return "invalid".to_owned();
            }
        },
    };

    let complete_coverage = denominator.is_some_and(|value| numerator == value);
    if evidence_state != "observed_current"
        || !complete_coverage
        || lower.is_none()
        || upper.is_none()
        || !nonempty(area, "observation_ref")
        || !nonempty(area, "validity_window_ref")
    {
        return "indeterminate".to_owned();
    }

    let lower = lower.unwrap_or(f64::NAN);
    let upper = upper.unwrap_or(f64::NAN);
    if lower > upper {
        push(errors, context, "lower observation bound exceeds upper bound");
        return "invalid".to_owned();
    }
    if lower >= floor {
        "meets_floor".to_owned()
    } else if upper < floor {
        "breached_floor".to_owned()
    } else {
        "indeterminate".to_owned()
    }
}

fn overall_disposition(area_states: &[String]) -> String {
    if area_states.iter().any(|state| state == "invalid") {
        return "invalid".to_owned();
    }
    let breach = area_states.iter().any(|state| state == "breached_floor");
    let unknown = area_states.iter().any(|state| state == "indeterminate");
    match (breach, unknown) {
        (true, true) => "breached_with_indeterminate".to_owned(),
        (true, false) => "breached_floor".to_owned(),
        (false, true) => "indeterminate".to_owned(),
        (false, false) => "meets_all_floors".to_owned(),
    }
}

fn verify_aggregate(scenario: &Value, areas: &[Value], context: &str, errors: &mut Vec<String>) {
    let diagnostic = match scenario.get("aggregate_diagnostic") {
        Some(value) => value,
        None => {
            push(errors, context, "aggregate_diagnostic is required");
            return;
        }
    };
    if diagnostic.get("authoritative").and_then(Value::as_bool) != Some(false) {
        push(errors, context, "aggregate diagnostic must never be authoritative");
        return;
    }
    match text(diagnostic, "method") {
        Some("not_computable") => {
            if !diagnostic.get("value").is_some_and(Value::is_null) {
                push(errors, context, "not_computable diagnostic must have null value");
            }
        }
        Some("weighted_average_by_synthetic_connection_count") => {
            let declared = finite(diagnostic, "value");
            if declared.is_none() {
                push(errors, context, "weighted average requires a finite diagnostic value");
                return;
            }
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for area in areas {
                let lower = area.get("observation_lower_bound").and_then(Value::as_f64);
                let weight = finite(area, "diagnostic_weight");
                let evidence_current =
                    text(area, "evidence_state") == Some("observed_current");
                let complete_coverage = match (
                    area.get("coverage_numerator").and_then(Value::as_u64),
                    area.get("coverage_denominator").and_then(Value::as_u64),
                ) {
                    (Some(numerator), Some(denominator)) if denominator > 0 => {
                        numerator == denominator
                    }
                    _ => false,
                };
                if !evidence_current || !complete_coverage {
                    push(
                        errors,
                        context,
                        "weighted average must not use stale, missing or partially covered area evidence",
                    );
                    return;
                }
                match (lower, weight) {
                    (Some(value), Some(weight))
                        if value.is_finite()
                            && (0.0..=1.0).contains(&value)
                            && weight > 0.0 =>
                    {
                        numerator += value * weight;
                        denominator += weight;
                    }
                    _ => {
                        push(
                            errors,
                            context,
                            "weighted average supplied when one or more area bounds/weights are unavailable",
                        );
                        return;
                    }
                }
            }
            if denominator <= 0.0 {
                push(errors, context, "weighted average has no positive weight");
                return;
            }
            let recomputed = numerator / denominator;
            if (recomputed - declared.unwrap_or_default()).abs() > 5.0e-5 {
                push(errors, context, "weighted average does not match the supplied synthetic diagnostic");
            }
        }
        _ => push(errors, context, "unsupported aggregate diagnostic method"),
    }
}

pub fn evaluate(
    scenarios: &Value,
    schema: &Value,
    oracle: &Value,
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();

    if text(scenarios, "schema_version") != Some("0.1.0")
        || text(scenarios, "corpus_id") != Some(CORPUS_ID)
    {
        push(&mut errors, "scenario corpus", "unexpected corpus identity/version");
    }
    if text(oracle, "schema_version") != Some("0.1.0")
        || text(oracle, "oracle_id") != Some(ORACLE_ID)
        || text(oracle, "corpus_id") != Some(CORPUS_ID)
    {
        push(&mut errors, "oracle", "unexpected oracle identity/version");
    }
    if text(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema")
        || text(schema, "type") != Some("object")
        || schema.get("additionalProperties").and_then(Value::as_bool) != Some(false)
    {
        push(&mut errors, "schema", "schema must declare Draft 2020-12 object root with no additional properties");
    }
    let schema_required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in ["schema_version", "corpus_id", "profile", "scenarios", "claim_ceiling"] {
        if !schema_required.contains(key) {
            push(&mut errors, "schema", &format!("root must require {key}"));
        }
    }

    let profile = scenarios.get("profile").unwrap_or(&Value::Null);
    if text(profile, "profile_id") != Some("SYNTHETIC-COMMUNITY-FLOOR-V0")
        || text(profile, "evidence_class") != Some("synthetic_design_profile")
        || text(profile, "metric") != Some("delivered_demand_fraction")
        || text(profile, "unit") != Some("fraction_of_required_volume_over_declared_window")
        || text(profile, "aggregation_authority") != Some("diagnostic_only_never_overrides_area_floor")
        || profile.get("values_are_normative").and_then(Value::as_bool) != Some(false)
    {
        push(&mut errors, "profile", "profile identity, metric, unit, aggregation boundary or non-normative status changed");
    }
    let minimum_floor = finite(profile, "minimum_floor");
    if !minimum_floor.is_some_and(|value| (0.0..=1.0).contains(&value)) {
        push(&mut errors, "profile", "minimum floor must be finite and in [0, 1]");
    }
    let required_areas: Vec<String> = profile
        .get("required_area_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let required_set: BTreeSet<String> = required_areas.iter().cloned().collect();
    if required_areas.len() != 3 || required_set.len() != required_areas.len() {
        push(&mut errors, "profile", "exactly three unique required area IDs are required in v0.1");
    }

    let cases = scenarios.get("scenarios").and_then(Value::as_array);
    let oracle_rows = oracle.get("results").and_then(Value::as_array);
    let Some(cases) = cases else {
        return Err(vec!["scenario corpus: scenarios must be an array".to_owned()]);
    };
    let Some(oracle_rows) = oracle_rows else {
        return Err(vec!["oracle: results must be an array".to_owned()]);
    };
    if cases.len() != EXPECTED_CASES || oracle_rows.len() != EXPECTED_CASES {
        push(
            &mut errors,
            "corpus",
            &format!("expected {EXPECTED_CASES} scenarios and oracle rows"),
        );
    }

    let mut oracle_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for row in oracle_rows {
        let Some(id) = text(row, "scenario_id") else {
            push(&mut errors, "oracle row", "scenario_id missing");
            continue;
        };
        if oracle_by_id.insert(id.to_owned(), row).is_some() {
            push(&mut errors, "oracle", &format!("duplicate scenario_id {id}"));
        }
        if row.get("aggregate_can_override_area_floor").and_then(Value::as_bool) != Some(false)
            || text(row, "evidence_ceiling") != Some("synthetic_design_only")
        {
            push(&mut errors, id, "oracle attempts to broaden authority/evidence ceiling");
        }
    }

    let mut case_ids = BTreeSet::new();
    let mut results = Vec::new();
    for case in cases {
        let Some(id) = text(case, "scenario_id") else {
            push(&mut errors, "scenario", "scenario_id missing");
            continue;
        };
        let scenario_class = text(case, "scenario_class").unwrap_or("");
        if ![
            "all_floors_met",
            "weighted_average_masks_breach",
            "single_area_breach",
            "missing_evidence",
            "stale_evidence",
            "partial_coverage",
            "interval_crosses_floor",
            "breach_plus_unknown",
            "policy_missing",
            "unit_mismatch",
            "time_grid_mismatch",
            "boundary_equal_to_floor",
        ]
        .contains(&scenario_class)
        {
            push(&mut errors, id, "unsupported scenario_class");
        }
        if !id.starts_with("WSF-") || !case_ids.insert(id.to_owned()) {
            push(&mut errors, "scenario", &format!("invalid or duplicate scenario ID {id}"));
        }
        let context = id.to_owned();
        let Some(areas) = case.get("areas").and_then(Value::as_array) else {
            push(&mut errors, id, "areas must be an array");
            continue;
        };
        if areas.len() != required_areas.len() {
            push(
                &mut errors,
                id,
                &format!("exactly {} required area records must be present", required_areas.len()),
            );
        }
        let mut area_ids = BTreeSet::new();
        let mut area_results = Vec::new();
        let mut states = Vec::new();
        for area in areas {
            let Some(area_id) = text(area, "area_id") else {
                push(&mut errors, id, "area_id missing");
                continue;
            };
            if !area_ids.insert(area_id.to_owned()) {
                push(&mut errors, id, &format!("duplicate area_id {area_id}"));
            }
            if !required_set.contains(area_id) {
                push(&mut errors, id, &format!("unexpected area_id {area_id}"));
            }
            let area_context = format!("{id}/{area_id}");
            let state = evaluate_area(area, profile, &area_context, &mut errors);
            let expected_area = text(area, "expected_area_disposition").unwrap_or("");
            if state != expected_area {
                push(
                    &mut errors,
                    &area_context,
                    &format!("expected area disposition {expected_area}, recomputed {state}"),
                );
            }
            states.push(state.clone());
            area_results.push(json!({
                "area_id": area_id,
                "disposition": state,
                "lower_bound": area.get("observation_lower_bound").cloned().unwrap_or(Value::Null),
                "upper_bound": area.get("observation_upper_bound").cloned().unwrap_or(Value::Null),
                "coverage_numerator": area.get("coverage_numerator").cloned().unwrap_or(Value::Null),
                "coverage_denominator": area.get("coverage_denominator").cloned().unwrap_or(Value::Null)
            }));
        }
        if area_ids != required_set {
            push(&mut errors, id, "required area ID set is incomplete or differs from the frozen profile");
        }

        let overall = overall_disposition(&states);
        let expected_overall = text(case, "expected_overall_disposition").unwrap_or("");
        if overall != expected_overall {
            push(
                &mut errors,
                id,
                &format!("expected overall disposition {expected_overall}, recomputed {overall}"),
            );
        }
        verify_aggregate(case, areas, id, &mut errors);

        let oracle_row = oracle_by_id.get(id).copied();
        if oracle_row.is_none() {
            push(&mut errors, id, "scenario missing from separate oracle");
        } else if let Some(row) = oracle_row {
            if text(row, "expected_overall_disposition") != Some(overall.as_str()) {
                push(&mut errors, id, "overall disposition differs from separate oracle");
            }
            let oracle_areas = row.get("expected_area_dispositions").and_then(Value::as_array);
            let oracle_ids: BTreeSet<String> = oracle_areas
                .into_iter()
                .flatten()
                .filter_map(|expected| text(expected, "area_id").map(str::to_owned))
                .collect();
            let actual_ids: BTreeSet<String> = area_results
                .iter()
                .filter_map(|actual| text(actual, "area_id").map(str::to_owned))
                .collect();
            let dispositions_match = oracle_areas.is_some_and(|rows| {
                rows.len() == area_results.len()
                    && rows.iter().all(|expected| {
                        let expected_id = text(expected, "area_id");
                        let expected_state = text(expected, "disposition");
                        area_results.iter().any(|actual| {
                            text(actual, "area_id") == expected_id
                                && text(actual, "disposition") == expected_state
                        })
                    })
            });
            if oracle_ids != actual_ids || !dispositions_match {
                push(&mut errors, id, "per-area IDs/dispositions differ from separate oracle");
            }
        }

        results.push(json!({
            "scenario_id": id,
            "verification_state": "Matched",
            "overall_disposition": overall,
            "area_results": area_results,
            "aggregate_diagnostic_authoritative": false,
            "evidence_ceiling": "synthetic_design_only"
        }));
    }

    if case_ids.len() != EXPECTED_CASES
        || case_ids.len() != oracle_by_id.len()
        || oracle_by_id.keys().any(|id| !case_ids.contains(id))
    {
        push(&mut errors, "corpus", "scenario and oracle ID sets are not exactly equal");
    }
    if !text(scenarios, "claim_ceiling")
        .is_some_and(|value| value.to_ascii_lowercase().contains("synthetic"))
    {
        push(&mut errors, "corpus", "claim ceiling must remain synthetic");
    }

    if errors.is_empty() {
        Ok(json!({
            "schema_version": "community-water-service-floor-evaluation-v0.1",
            "verification_state": "Matched",
            "qualification_status": "NotRun",
            "scenario_count": cases.len(),
            "matched_scenario_count": results.len(),
            "required_area_ids": required_areas,
            "results": results,
            "evidence_class": "synthetic_semantic_contract_only",
            "claim_ceiling": "per_area_floor_and_overall_disposition_semantics_only_no_service_or_field_qualification",
            "nonclaims": [
                "No hydraulic model or control policy is executed.",
                "No real household, community or utility service is measured.",
                "No legal service standard or allocation policy is established.",
                "No rationing, shutoff, relocation or utility actuation is authorized."
            ]
        }))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIOS: &str =
        include_str!("../../../../research/water-service-engineering/community-service-floors/scenarios-v0.1.json");
    const SCHEMA: &str =
        include_str!("../../../../research/water-service-engineering/community-service-floors/scenarios-v0.1.schema.json");
    const ORACLE: &str =
        include_str!("../../../../research/water-service-engineering/community-service-floors/disposition-oracle-v0.1.json");

    fn fixtures() -> (Value, Value, Value) {
        (
            serde_json::from_str(SCENARIOS).expect("scenarios parse"),
            serde_json::from_str(SCHEMA).expect("schema parses"),
            serde_json::from_str(ORACLE).expect("oracle parses"),
        )
    }

    #[test]
    fn all_twelve_frozen_scenarios_match_the_oracle() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("canonical fixture validates");
        assert_eq!(value["scenario_count"], EXPECTED_CASES);
        assert_eq!(value["matched_scenario_count"], EXPECTED_CASES);
        assert_eq!(value["qualification_status"], "NotRun");
    }

    #[test]
    fn omitted_required_area_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"].as_array_mut().expect("areas").pop();
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("omission must fail");
        assert!(errors.iter().any(|e| e.contains("required area ID set is incomplete")));
    }

    #[test]
    fn duplicate_area_id_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][2]["area_id"] = json!("DMA-B");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("duplicate area must fail");
        assert!(errors.iter().any(|e| e.contains("duplicate area_id")));
    }

    #[test]
    fn high_weighted_mean_cannot_hide_a_local_breach() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-002").expect("WSF-002");
        assert_eq!(case["overall_disposition"], "breached_floor");
        assert_eq!(case["aggregate_diagnostic_authoritative"], false);
    }

    #[test]
    fn interval_crossing_floor_is_indeterminate() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-007").expect("WSF-007");
        assert_eq!(case["overall_disposition"], "indeterminate");
    }

    #[test]
    fn stale_evidence_cannot_meet_floor() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][4]["areas"][2]["expected_area_disposition"] = json!("meets_floor");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("stale cannot pass");
        assert!(errors.iter().any(|e| e.contains("expected area disposition meets_floor")));
    }

    #[test]
    fn partial_coverage_cannot_meet_floor() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][5]["areas"][2]["expected_area_disposition"] = json!("meets_floor");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("partial cannot pass");
        assert!(errors.iter().any(|e| e.contains("expected area disposition meets_floor")));
    }

    #[test]
    fn breach_and_unknown_are_both_preserved() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-008").expect("WSF-008");
        assert_eq!(case["overall_disposition"], "breached_with_indeterminate");
    }

    #[test]
    fn missing_floor_policy_is_invalid() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-009").expect("WSF-009");
        assert_eq!(case["overall_disposition"], "invalid");
    }

    #[test]
    fn unit_mismatch_is_invalid() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-010").expect("WSF-010");
        assert_eq!(case["overall_disposition"], "invalid");
    }

    #[test]
    fn time_grid_mismatch_is_invalid() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = value["results"].as_array().expect("results")
            .iter().find(|item| item["scenario_id"] == "WSF-011").expect("WSF-011");
        assert_eq!(case["overall_disposition"], "invalid");
    }

    #[test]
    fn weighted_average_arithmetic_is_recomputed() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][1]["aggregate_diagnostic"]["value"] = json!(0.99);
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("wrong aggregate must fail");
        assert!(errors.iter().any(|e| e.contains("weighted average does not match")));
    }

    #[test]
    fn normative_profile_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["profile"]["values_are_normative"] = json!(true);
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("normative profile must fail");
        assert!(errors.iter().any(|e| e.contains("non-normative status changed")));
    }
    #[test]
    fn missing_coverage_denominator_cannot_be_treated_as_complete() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["areas"][0]["coverage_denominator"] = Value::Null;
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("unknown denominator must not pass");
        assert!(errors.iter().any(|e| e.contains("expected area disposition meets_floor")));
    }

    #[test]
    fn unsupported_scenario_class_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["scenario_class"] = json!("aggregate_pass");
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("unknown scenario class must fail");
        assert!(errors.iter().any(|e| e.contains("unsupported scenario_class")));
    }

    #[test]
    fn duplicate_area_ids_in_oracle_are_rejected() {
        let (scenarios, schema, mut oracle) = fixtures();
        oracle["results"][0]["expected_area_dispositions"][2]["area_id"] = json!("DMA-B");
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("oracle area set must be exact");
        assert!(errors.iter().any(|e| e.contains("per-area IDs/dispositions differ")));
    }

    #[test]
    fn one_malformed_bound_is_rejected_even_when_the_other_is_null() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][3]["areas"][1]["observation_lower_bound"] = json!("not-a-number");
        scenarios["scenarios"][3]["areas"][1]["observation_upper_bound"] = Value::Null;
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("malformed lower bound must fail despite explicit null upper bound");
        assert!(errors.iter().any(|e| e.contains("lower observation bound must be finite")));
    }

    #[test]
    fn weighted_average_cannot_use_stale_or_partial_area_evidence() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][1]["areas"][2]["evidence_state"] = json!("stale");
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("stale evidence cannot contribute to a positive aggregate diagnostic");
        assert!(errors.iter().any(|e| e.contains("weighted average must not use stale")));
    }

}
