// SPDX-License-Identifier: Apache-2.0 OR MIT
//! As-of-time integrity gate for ecological forecasts and hindcasts.
//!
//! Distinguishes ecological valid/event time from source/QC/rights/ingestion
//! availability. It validates a declared synthetic input set; it is not a clock
//! service, bitemporal database, or empirical AFEX archive adjudicator.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug)]
struct SourceVersion {
    version_id: String,
    logical_observation_id: String,
    version_number: u64,
    role: String,
    valid_start: u64,
    valid_end: u64,
    collected_at: u64,
    source_released_at: Option<u64>,
    qa_released_at: Option<u64>,
    rights_effective_at: Option<u64>,
    ingested_at: Option<u64>,
    rights_status: String,
    driver_forecast_issued_at: Option<u64>,
    driver_target_start: Option<u64>,
    driver_target_end: Option<u64>,
}

impl SourceVersion {
    fn effective_available_at(&self) -> Option<u64> {
        Some([
            self.source_released_at?,
            self.qa_released_at?,
            self.rights_effective_at?,
            self.ingested_at?,
        ].into_iter().max()?)
    }
}

#[derive(Clone, Debug)]
struct InputLatency {
    version_id: String,
    collected_at: u64,
    available_at: u64,
    latency_seconds: u64,
}

#[derive(Clone, Debug)]
struct CaseResult {
    case_id: String,
    disposition: String,
    reasons: Vec<String>,
    selected_version_ids: Vec<String>,
    input_latencies: Vec<InputLatency>,
    maximum_input_latency_seconds: Option<u64>,
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn u64_at(value: &Value, key: &str) -> Result<u64, String> {
    value.get(key).and_then(Value::as_u64)
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}

fn optional_u64_at(value: &Value, key: &str) -> Result<Option<u64>, String> {
    match value.get(key) {
        Some(Value::Null) | None => Ok(None),
        Some(number) => number.as_u64().map(Some)
            .ok_or_else(|| format!("{key} must be a non-negative integer or null")),
    }
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("String write cannot fail");
    }
    output
}

fn parse_source_version(value: &Value) -> Result<SourceVersion, String> {
    let version = SourceVersion {
        version_id: string_at(value, "version_id")?,
        logical_observation_id: string_at(value, "logical_observation_id")?,
        version_number: u64_at(value, "version_number")?,
        role: string_at(value, "role")?,
        valid_start: u64_at(value, "valid_start")?,
        valid_end: u64_at(value, "valid_end")?,
        collected_at: u64_at(value, "collected_at")?,
        source_released_at: optional_u64_at(value, "source_released_at")?,
        qa_released_at: optional_u64_at(value, "qa_released_at")?,
        rights_effective_at: optional_u64_at(value, "rights_effective_at")?,
        ingested_at: optional_u64_at(value, "ingested_at")?,
        rights_status: string_at(value, "rights_status")?,
        driver_forecast_issued_at: optional_u64_at(value, "driver_forecast_issued_at")?,
        driver_target_start: optional_u64_at(value, "driver_target_start")?,
        driver_target_end: optional_u64_at(value, "driver_target_end")?,
    };
    if version.version_id.trim().is_empty()
        || version.logical_observation_id.trim().is_empty()
        || version.version_number == 0
        || version.valid_start > version.valid_end
    {
        return Err(format!("source version {} has invalid identity or temporal bounds", version.version_id));
    }
    Ok(version)
}

fn disposition(case_id: String, name: &str, reason: &str, selected: Vec<String>, latencies: Vec<InputLatency>) -> CaseResult {
    let maximum = latencies.iter().map(|item| item.latency_seconds).max();
    CaseResult {
        case_id,
        disposition: name.to_string(),
        reasons: vec![reason.to_string()],
        selected_version_ids: selected,
        input_latencies: latencies,
        maximum_input_latency_seconds: maximum,
    }
}

fn evaluate_case(case: &Value) -> Result<CaseResult, String> {
    let case_id = string_at(case, "case_id")?;
    let forecast = case.get("forecast").ok_or_else(|| format!("case {case_id} forecast required"))?;
    let reference = u64_at(forecast, "reference_time")?;
    let cutoff = u64_at(forecast, "information_cutoff_time")?;
    let issued = u64_at(forecast, "issued_at")?;
    let target_start = u64_at(forecast, "target_start")?;
    let target_end = u64_at(forecast, "target_end")?;
    let horizon = u64_at(forecast, "target_horizon_seconds")?;

    if reference > cutoff || cutoff > issued || issued >= target_start
        || target_start > target_end || target_start.saturating_sub(reference) != horizon
    {
        return Ok(disposition(
            case_id, "FORECAST_TIME_ORDER_REFUSED", "INVALID_FORECAST_TIME_ORDER",
            Vec::new(), Vec::new()
        ));
    }

    let source_values = case.get("source_versions").and_then(Value::as_array)
        .ok_or_else(|| format!("case {case_id} source_versions must be an array"))?;
    if source_values.is_empty() {
        return Ok(disposition(
            case_id, "NOT_ESTIMABLE", "NO_SOURCE_VERSIONS",
            Vec::new(), Vec::new()
        ));
    }

    let mut versions = Vec::with_capacity(source_values.len());
    let mut version_ids = BTreeSet::new();
    let mut logical_version_numbers = BTreeSet::new();
    for value in source_values {
        let parsed = parse_source_version(value)?;
        if !version_ids.insert(parsed.version_id.clone())
            || !logical_version_numbers.insert((parsed.logical_observation_id.clone(), parsed.version_number))
        {
            return Err(format!("case {case_id} has duplicate source version identity/number"));
        }
        versions.push(parsed);
    }

    let selected_values = case.get("selected_input_version_ids").and_then(Value::as_array)
        .ok_or_else(|| format!("case {case_id} selected_input_version_ids must be an array"))?;
    if selected_values.is_empty() {
        return Ok(disposition(
            case_id, "NOT_ESTIMABLE", "NO_SELECTED_INPUTS",
            Vec::new(), Vec::new()
        ));
    }
    let mut selected_ids = Vec::with_capacity(selected_values.len());
    let mut selected_unique = BTreeSet::new();
    for item in selected_values {
        let id = item.as_str().ok_or_else(|| "selected version IDs must be strings".to_string())?.to_string();
        if id.trim().is_empty() || !selected_unique.insert(id.clone()) {
            return Err(format!("case {case_id} has empty or duplicate selected input version"));
        }
        selected_ids.push(id);
    }

    let mut selected_logical_ids = BTreeSet::new();
    let mut latencies = Vec::new();
    for selected_id in &selected_ids {
        let Some(selected) = versions.iter().find(|version| &version.version_id == selected_id) else {
            return Ok(disposition(
                case_id, "NOT_ESTIMABLE", "SELECTED_SOURCE_VERSION_NOT_FOUND",
                selected_ids, latencies
            ));
        };

        if !selected_logical_ids.insert(selected.logical_observation_id.clone()) {
            return Ok(disposition(
                case_id, "MULTIPLE_VERSIONS_OF_ONE_OBSERVATION_SELECTED",
                "DUPLICATE_LOGICAL_OBSERVATION_INPUT", selected_ids, latencies
            ));
        }

        if selected.rights_status != "PERMITTED" {
            return Ok(disposition(
                case_id, "REFUSED_RIGHTS", "RIGHTS_NOT_CLEARED",
                selected_ids, latencies
            ));
        }

        match selected.role.as_str() {
            "evaluation_only_target" => {
                return Ok(disposition(
                    case_id, "EVALUATION_TARGET_USED_AS_INPUT",
                    "EVALUATION_TARGET_USED_AS_INPUT", selected_ids, latencies
                ));
            }
            "historical_observation" | "training_label" | "state_initialization" | "realized_observation" => {
                if selected.valid_start > reference || selected.valid_end > reference {
                    return Ok(disposition(
                        case_id, "REALIZED_FUTURE_OBSERVATION_USED_AS_DRIVER",
                        "REALIZED_FUTURE_OBSERVATION_USED_AS_DRIVER", selected_ids, latencies
                    ));
                }
            }
            "forecasted_driver" => {}
            _ => {
                return Ok(disposition(
                    case_id, "REFUSED_UNKNOWN_INPUT_ROLE",
                    "UNKNOWN_INPUT_ROLE", selected_ids, latencies
                ));
            }
        }

        let Some(available_at) = selected.effective_available_at() else {
            return Ok(disposition(
                case_id, "NOT_ESTIMABLE_MISSING_AVAILABILITY_EVIDENCE",
                "MISSING_AVAILABILITY_EVIDENCE", selected_ids, latencies
            ));
        };

        if selected.collected_at > available_at {
            return Ok(disposition(
                case_id, "NOT_ESTIMABLE_INVALID_AVAILABILITY_ORDER",
                "COLLECTION_AFTER_AVAILABILITY", selected_ids, latencies
            ));
        }

        if available_at > cutoff {
            let earlier_version_available = versions.iter().any(|candidate| {
                candidate.logical_observation_id == selected.logical_observation_id
                    && candidate.version_number < selected.version_number
                    && candidate.rights_status == "PERMITTED"
                    && candidate.effective_available_at().is_some_and(|time| time <= cutoff)
            });
            let (name, reason) = if earlier_version_available {
                ("SELECTED_VERSION_NOT_AVAILABLE_AS_OF_CUTOFF", "SELECTED_VERSION_NOT_AVAILABLE_AS_OF_CUTOFF")
            } else {
                ("INPUT_NOT_AVAILABLE_BY_CUTOFF", "INPUT_NOT_AVAILABLE_BY_CUTOFF")
            };
            return Ok(disposition(case_id, name, reason, selected_ids, latencies));
        }

        if selected.role == "forecasted_driver" {
            let Some(driver_issued) = selected.driver_forecast_issued_at else {
                return Ok(disposition(
                    case_id, "NOT_ESTIMABLE_MISSING_AVAILABILITY_EVIDENCE",
                    "MISSING_DRIVER_FORECAST_ISSUE_TIME", selected_ids, latencies
                ));
            };
            let (Some(driver_start), Some(driver_end)) =
                (selected.driver_target_start, selected.driver_target_end)
            else {
                return Ok(disposition(
                    case_id, "NOT_ESTIMABLE_MISSING_AVAILABILITY_EVIDENCE",
                    "MISSING_DRIVER_TARGET_SUPPORT", selected_ids, latencies
                ));
            };
            if driver_issued > cutoff || driver_start > target_start || driver_end < target_end {
                return Ok(disposition(
                    case_id, "INVALID_FORECASTED_DRIVER_SUPPORT",
                    "DRIVER_FORECAST_NOT_AVAILABLE_OR_TARGET_SUPPORT_MISMATCH",
                    selected_ids, latencies
                ));
            }
        }

        latencies.push(InputLatency {
            version_id: selected.version_id.clone(),
            collected_at: selected.collected_at,
            available_at,
            latency_seconds: available_at - selected.collected_at,
        });
    }

    // For each logical observation, the selected version must be the newest
    // permitted, fully available version at the fold's information cutoff.
    for selected_id in &selected_ids {
        let selected = versions.iter().find(|version| &version.version_id == selected_id)
            .ok_or_else(|| "selected source version disappeared".to_string())?;
        let mut latest_available: Option<u64> = None;
        for candidate in versions.iter().filter(|candidate|
            candidate.logical_observation_id == selected.logical_observation_id
        ) {
            if candidate.rights_status != "PERMITTED" {
                continue;
            }
            if let Some(available_at) = candidate.effective_available_at() {
                if available_at <= cutoff {
                    latest_available = Some(latest_available.unwrap_or(0).max(candidate.version_number));
                } else if candidate.version_number > selected.version_number {
                    // A known later correction after cutoff is harmless; it must
                    // not replace the as-of version for this forecast origin.
                }
            } else if candidate.version_number > selected.version_number {
                return Ok(disposition(
                    case_id, "NOT_ESTIMABLE_MISSING_AVAILABILITY_EVIDENCE",
                    "CANNOT_ESTABLISH_LATEST_VERSION_AS_OF_CUTOFF",
                    selected_ids, latencies
                ));
            }
        }
        if latest_available != Some(selected.version_number) {
            return Ok(disposition(
                case_id, "SELECTED_VERSION_NOT_LATEST_AS_OF_CUTOFF",
                "SELECTED_VERSION_NOT_LATEST_AS_OF_CUTOFF", selected_ids, latencies
            ));
        }
    }

    let transforms = case.get("fitted_transforms").and_then(Value::as_array)
        .ok_or_else(|| format!("case {case_id} fitted_transforms must be an array"))?;
    for transform in transforms {
        let transform_id = string_at(transform, "transform_id")?;
        let fit_cutoff = u64_at(transform, "fit_data_cutoff_at")?;
        let fit_completed = u64_at(transform, "fit_completed_at")?;
        if fit_cutoff > cutoff || fit_completed > issued {
            return Ok(disposition(
                case_id, "TRANSFORM_FIT_AFTER_CUTOFF",
                "TRANSFORM_FIT_AFTER_CUTOFF", selected_ids, latencies
            ));
        }
        let dependencies = transform.get("training_input_version_ids").and_then(Value::as_array)
            .ok_or_else(|| format!("transform {transform_id} training_input_version_ids must be an array"))?;
        for dependency in dependencies {
            let id = dependency.as_str().ok_or_else(|| "transform dependency IDs must be strings".to_string())?;
            if !selected_unique.contains(id) {
                return Ok(disposition(
                    case_id, "NOT_ESTIMABLE_TRANSFORM_DEPENDENCY_NOT_ADMITTED",
                    "TRANSFORM_DEPENDENCY_NOT_IN_SELECTED_INPUT_SET", selected_ids, latencies
                ));
            }
        }
    }

    let target = case.get("evaluation_target")
        .ok_or_else(|| format!("case {case_id} evaluation_target required"))?;
    let evaluation_start = u64_at(target, "valid_start")?;
    let evaluation_end = u64_at(target, "valid_end")?;
    let evaluation_available = u64_at(target, "available_at")?;
    if evaluation_start < target_start || evaluation_end > target_end || evaluation_start > evaluation_end {
        return Ok(disposition(
            case_id, "EVALUATION_TARGET_SUPPORT_MISMATCH",
            "EVALUATION_TARGET_SUPPORT_MISMATCH", selected_ids, latencies
        ));
    }
    if evaluation_available <= issued {
        return Ok(disposition(
            case_id, "EVALUATION_TARGET_AVAILABLE_AT_ISSUE",
            "TARGET_OBSERVATION_AVAILABLE_BEFORE_FORECAST_ISSUE", selected_ids, latencies
        ));
    }

    Ok(CaseResult {
        case_id,
        disposition: "VALID_AS_OF".to_string(),
        reasons: Vec::new(),
        selected_version_ids: selected_ids,
        maximum_input_latency_seconds: latencies.iter().map(|item| item.latency_seconds).max(),
        input_latencies: latencies,
    })
}

fn validate_expected(actual: &CaseResult, expected: &Value) -> Result<(), String> {
    if actual.disposition != string_at(expected, "expected_disposition")? {
        return Err(format!("case {} disposition mismatch: actual {}, expected {}", actual.case_id, actual.disposition, string_at(expected, "expected_disposition")?));
    }
    let expected_reasons = expected.get("expected_reasons").and_then(Value::as_array)
        .ok_or_else(|| "expected_reasons must be an array".to_string())?;
    let expected_reasons = expected_reasons.iter().map(|item|
        item.as_str().map(ToOwned::to_owned).ok_or_else(|| "expected reason must be a string".to_string())
    ).collect::<Result<Vec<_>, _>>()?;
    if actual.reasons != expected_reasons {
        return Err(format!("case {} reason oracle mismatch", actual.case_id));
    }
    Ok(())
}

pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    if string_at(manifest, "schema_version")? != "0.1.0"
        || string_at(manifest, "benchmark_id")? != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || string_at(manifest, "corpus_id")? != "FOREST-AS-OF-FORECAST-INTEGRITY-001"
    {
        return Err("forest as-of integrity corpus identity mismatch".to_string());
    }
    let ceiling = string_at(manifest, "claim_ceiling")?;
    if !ceiling.contains("All times, source/version IDs")
        || !ceiling.contains("No AFEX data versions")
    {
        return Err("forest as-of integrity claim ceiling is incomplete".to_string());
    }
    if string_at(manifest, "time_unit")? != "UTC Unix seconds" {
        return Err("synthetic as-of time unit must be UTC Unix seconds".to_string());
    }
    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    let expected_count = manifest.get("case_count").and_then(Value::as_u64)
        .ok_or_else(|| "case_count must be an integer".to_string())? as usize;
    if cases.len() != expected_count || cases.is_empty() {
        return Err("as-of case count mismatch".to_string());
    }
    let mut ids = BTreeSet::new();
    let mut results = Vec::new();
    for case in cases {
        let case_id = string_at(case, "case_id")?;
        if !ids.insert(case_id.clone()) {
            return Err("duplicate as-of case ID".to_string());
        }
        let result = evaluate_case(case)?;
        validate_expected(&result, case)?;
        let latency_json = result.input_latencies.iter().map(|item| json!({
            "version_id":item.version_id,
            "collected_at":item.collected_at,
            "available_at":item.available_at,
            "latency_seconds":item.latency_seconds
        })).collect::<Vec<_>>();
        results.push(json!({
            "case_id":result.case_id,
            "disposition":result.disposition,
            "reasons":result.reasons,
            "selected_version_ids":result.selected_version_ids,
            "input_latencies":latency_json,
            "maximum_input_latency_seconds":result.maximum_input_latency_seconds
        }));
    }
    Ok(json!({
        "schema_version":"0.1.0",
        "benchmark_id":"LUMINOUS-TROPICAL-FOREST-ECO-QUAL",
        "corpus_id":"FOREST-AS-OF-FORECAST-INTEGRITY-001",
        "result":"PASS_SYNTHETIC_AS_OF_FORECAST_INTEGRITY",
        "source_manifest_sha256":sha256_hex(raw),
        "case_count":results.len(),
        "valid_as_of_count":results.iter().filter(|result|result["disposition"]=="VALID_AS_OF").count(),
        "not_estimable_count":results.iter().filter(|result|result["disposition"].as_str().unwrap_or("").starts_with("NOT_ESTIMABLE")).count(),
        "rights_refusal_count":results.iter().filter(|result|result["disposition"]=="REFUSED_RIGHTS").count(),
        "leakage_refusal_count":results.iter().filter(|result|result["disposition"].as_str().unwrap_or("").contains("LEAKAGE") || result["disposition"]=="INPUT_NOT_AVAILABLE_BY_CUTOFF" || result["disposition"]=="SELECTED_VERSION_NOT_AVAILABLE_AS_OF_CUTOFF" || result["disposition"]=="TRANSFORM_FIT_AFTER_CUTOFF" || result["disposition"]=="EVALUATION_TARGET_USED_AS_INPUT").count(),
        "case_results":results,
        "claim_ceiling":ceiling
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    const FIXTURE:&str=include_str!("../../cases/forest-as-of-forecast-integrity-synthetic-v0.1.json");

    #[test]
    fn as_of_policy_reproduces_all_twelve_known_truth_cases() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture parses");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        assert_eq!(receipt["result"],"PASS_SYNTHETIC_AS_OF_FORECAST_INTEGRITY");
        assert_eq!(receipt["case_count"],12);
        assert_eq!(receipt["valid_as_of_count"],3);
        assert_eq!(receipt["not_estimable_count"],1);
        assert_eq!(receipt["rights_refusal_count"],1);
        assert_eq!(receipt["source_manifest_sha256"],sha256_hex(FIXTURE.as_bytes()));
    }

    #[test]
    fn post_cutoff_lab_qc_release_is_not_historically_available() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let result=receipt["case_results"].as_array().expect("results")
            .iter().find(|case|case["case_id"]=="LAB_QA_AFTER_CUTOFF").expect("late lab result");
        assert_eq!(result["disposition"],"INPUT_NOT_AVAILABLE_BY_CUTOFF");
    }

    #[test]
    fn later_correction_does_not_replace_the_latest_version_available_at_cutoff() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let result=receipt["case_results"].as_array().expect("results")
            .iter().find(|case|case["case_id"]=="REVISION_AS_OF_SELECTS_PRIOR_VERSION").expect("revision case");
        assert_eq!(result["disposition"],"VALID_AS_OF");
        assert_eq!(result["selected_version_ids"],json!(["TREE-CENSUS-V1"]));
    }

    #[test]
    fn realized_future_driver_is_refused_but_a_prior_forecast_driver_is_accepted() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let results=receipt["case_results"].as_array().expect("results");
        let realized=results.iter().find(|case|case["case_id"]=="FUTURE_REALIZED_DRIVER").expect("realized case");
        let forecasted=results.iter().find(|case|case["case_id"]=="FORECASTED_DRIVER_AS_OF").expect("forecasted case");
        assert_eq!(realized["disposition"],"REALIZED_FUTURE_OBSERVATION_USED_AS_DRIVER");
        assert_eq!(forecasted["disposition"],"VALID_AS_OF");
    }

    #[test]
    fn missing_availability_or_rights_evidence_never_defaults_to_early_availability() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let results=receipt["case_results"].as_array().expect("results");
        let missing=results.iter().find(|case|case["case_id"]=="MISSING_QA_TIMESTAMP_NOT_ESTIMABLE").expect("missing case");
        let rights=results.iter().find(|case|case["case_id"]=="UNCLEARED_RIGHTS_REFUSED").expect("rights case");
        assert_eq!(missing["disposition"],"NOT_ESTIMABLE_MISSING_AVAILABILITY_EVIDENCE");
        assert_eq!(rights["disposition"],"REFUSED_RIGHTS");
    }

    #[test]
    fn transform_fit_after_cutoff_is_rejected() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let result=receipt["case_results"].as_array().expect("results")
            .iter().find(|case|case["case_id"]=="TRANSFORM_FUTURE_FIT").expect("transform case");
        assert_eq!(result["disposition"],"TRANSFORM_FIT_AFTER_CUTOFF");
    }

    #[test]
    fn an_evaluation_target_cannot_be_used_as_a_feature() {
        let manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        let receipt=validate_corpus(FIXTURE.as_bytes(),&manifest).expect("receipt");
        let result=receipt["case_results"].as_array().expect("results")
            .iter().find(|case|case["case_id"]=="EVALUATION_TARGET_USED_AS_INPUT").expect("target case");
        assert_eq!(result["disposition"],"EVALUATION_TARGET_USED_AS_INPUT");
    }
    #[test]
    fn a_later_revision_cannot_be_selected_before_its_availability_time() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][3]["selected_input_version_ids"]=json!(["TREE-CENSUS-V2"]);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

    #[test]
    fn a_forecasted_driver_must_be_issued_by_the_information_cutoff() {
        let mut manifest:Value=serde_json::from_str(FIXTURE).expect("fixture");
        manifest["cases"][6]["source_versions"][0]["driver_forecast_issued_at"]=json!(1210);
        let raw=serde_json::to_vec(&manifest).expect("serialize");
        assert!(validate_corpus(&raw,&manifest).is_err());
    }

}
