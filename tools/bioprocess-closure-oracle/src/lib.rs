// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent, synthetic-first verifier for full-process resource closure.
//!
//! This crate validates declared accounting/evidence semantics. It is not a process
//! simulator, cell-kinetics model, food-safety evaluator, cost model, LCA, or plant design.
//! Numeric resource values are canonical decimal strings to avoid JSON floating-point drift.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub const RUNNER_VERSION: &str = "0.1.0";
pub const EXPECTED_CORPUS_ID: &str = "BIOPROC-CLOSURE-CORPUS-V1";
pub const EXPECTED_SCHEMA_VERSION: &str = "1.0.0";
const CLAIM_CEILING: &str = "Synthetic accounting/evidence contract only; no cell productivity, biological scale transfer, food safety, regulatory authorization, real production cost, or lifecycle outcome is established.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    EligibleForDeclaredComparison,
    Indeterminate,
    NotComparable,
    Blocked,
    InvalidEvidence,
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evaluation {
    pub case_id: String,
    pub disposition: Disposition,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaseReceipt {
    pub case_id: String,
    pub expected_disposition: String,
    pub actual_disposition: Disposition,
    pub expected_reason_codes: Vec<String>,
    pub actual_reason_codes: Vec<String>,
    pub contract_match: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunReceipt {
    pub runner_version: String,
    pub corpus_id: String,
    pub schema_version: String,
    pub corpus_sha256: String,
    pub declared_head: String,
    pub actual_head: String,
    pub exact_head_match: bool,
    pub case_count: usize,
    pub matched_case_count: usize,
    pub failed_case_ids: Vec<String>,
    pub cases: Vec<CaseReceipt>,
    pub synthetic_only_claim_ceiling: String,
    pub contract_passed: bool,
}

fn get<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |at, key| at.get(*key))
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    get(value, path)?.as_str()
}

fn nonempty(value: &Value, path: &[&str]) -> bool {
    text(value, path).is_some_and(|s| !s.trim().is_empty())
}

fn bool_value(value: &Value, path: &[&str]) -> Option<bool> {
    get(value, path)?.as_bool()
}

fn array<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    get(value, path)?.as_array()
}

fn invalid(case_id: &str, reason: &str) -> Evaluation {
    Evaluation {
        case_id: case_id.to_owned(),
        disposition: Disposition::InvalidInput,
        reason_codes: vec![reason.to_owned()],
    }
}

fn result(case_id: &str, disposition: Disposition, reason: &str) -> Evaluation {
    Evaluation {
        case_id: case_id.to_owned(),
        disposition,
        reason_codes: vec![reason.to_owned()],
    }
}

fn canonical_unsigned(value: &Value) -> Result<i128, &'static str> {
    let Some(raw) = value.as_str() else {
        return Err("RESOURCE_QUANTITY_NOT_DECIMAL_STRING");
    };
    if raw.is_empty()
        || !raw.bytes().all(|byte| byte.is_ascii_digit())
        || (raw.len() > 1 && raw.starts_with('0'))
    {
        return Err("RESOURCE_QUANTITY_NON_CANONICAL");
    }
    raw.parse::<i128>()
        .map_err(|_| "RESOURCE_QUANTITY_OUT_OF_RANGE")
}

fn canonical_signed(value: &Value) -> Result<i128, &'static str> {
    let Some(raw) = value.as_str() else {
        return Err("RESOURCE_QUANTITY_NOT_DECIMAL_STRING");
    };
    if raw == "0" {
        return Ok(0);
    }
    let digits = raw.strip_prefix('-').unwrap_or(raw);
    if digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || digits.starts_with('0')
    {
        return Err("RESOURCE_QUANTITY_NON_CANONICAL");
    }
    raw.parse::<i128>()
        .map_err(|_| "RESOURCE_QUANTITY_OUT_OF_RANGE")
}

fn base_unit(resource: &str) -> Option<&'static str> {
    match resource {
        "mass" => Some("g"),
        "water" => Some("mL"),
        "energy" => Some("Wh"),
        "time" => Some("s"),
        _ => None,
    }
}

fn evaluate_resource_balance(balance: &Value) -> Result<(), &'static str> {
    let resource = text(balance, &["resource"]).ok_or("RESOURCE_KIND_MISSING")?;
    let unit = text(balance, &["unit"]).ok_or("RESOURCE_UNIT_MISSING")?;
    let expected_unit = base_unit(resource).ok_or("RESOURCE_KIND_UNKNOWN")?;
    if unit != expected_unit {
        return Err("RESOURCE_UNIT_MISMATCH");
    }

    let inflow = canonical_unsigned(get(balance, &["inflow"]).ok_or("RESOURCE_INFLOW_MISSING")?)?;
    let additions =
        canonical_unsigned(get(balance, &["additions"]).ok_or("RESOURCE_ADDITIONS_MISSING")?)?;
    let outflow =
        canonical_unsigned(get(balance, &["outflow"]).ok_or("RESOURCE_OUTFLOW_MISSING")?)?;
    let loss = canonical_unsigned(get(balance, &["loss"]).ok_or("RESOURCE_LOSS_MISSING")?)?;
    let storage_delta = canonical_signed(
        get(balance, &["observed_storage_delta"]).ok_or("RESOURCE_STORAGE_DELTA_MISSING")?,
    )?;
    let tolerance =
        canonical_unsigned(get(balance, &["tolerance"]).ok_or("RESOURCE_TOLERANCE_MISSING")?)?;

    let left = inflow
        .checked_add(additions)
        .ok_or("RESOURCE_BALANCE_OVERFLOW")?;
    let right = outflow
        .checked_add(loss)
        .and_then(|sum| sum.checked_add(storage_delta))
        .ok_or("RESOURCE_BALANCE_OVERFLOW")?;
    let residual = left
        .checked_sub(right)
        .ok_or("RESOURCE_BALANCE_OVERFLOW")?
        .unsigned_abs();
    if residual > tolerance as u128 {
        return Err("RESOURCE_BALANCE_MISMATCH");
    }
    Ok(())
}

fn product_breakdown_is_closed(process: &Value) -> Result<(), &'static str> {
    let gross =
        canonical_unsigned(get(process, &["gross_product_g"]).ok_or("GROSS_PRODUCT_MISSING")?)?;
    let usable = canonical_unsigned(
        get(process, &["usable_product_g"]).ok_or("USABLE_PRODUCT_BREAKDOWN_MISSING")?,
    )?;
    let rejected = canonical_unsigned(
        get(process, &["rejected_product_g"]).ok_or("REJECTED_BATCH_BURDEN_MISSING")?,
    )?;
    let recovery_loss = canonical_unsigned(
        get(process, &["downstream_recovery_loss_g"]).ok_or("DOWNSTREAM_RECOVERY_LOSS_MISSING")?,
    )?;
    let accounted = usable
        .checked_add(rejected)
        .and_then(|sum| sum.checked_add(recovery_loss))
        .ok_or("PRODUCT_BREAKDOWN_OVERFLOW")?;
    if gross != accounted {
        return Err("PRODUCT_BREAKDOWN_MISMATCH");
    }
    Ok(())
}

fn valid_unique_strings(values: &[Value]) -> bool {
    let mut found = BTreeSet::new();
    values.iter().all(|value| {
        value
            .as_str()
            .is_some_and(|s| !s.trim().is_empty() && found.insert(s))
    })
}

fn evaluate_case(value: &Value) -> Evaluation {
    let case_id = text(value, &["case_id"]).unwrap_or("unknown-case");
    if case_id.trim().is_empty() {
        return invalid(case_id, "CASE_ID_MISSING");
    }
    let Some(request) = get(value, &["request"]) else {
        return invalid(case_id, "REQUEST_MISSING");
    };
    let Some(process) = get(value, &["process"]) else {
        return invalid(case_id, "PROCESS_MISSING");
    };

    if !nonempty(process, &["process_profile_id"])
        || !nonempty(process, &["scenario_digest"])
        || !nonempty(process, &["source_assumption_digest"])
        || !nonempty(request, &["scenario_digest"])
        || !nonempty(request, &["comparison_scenario_digest"])
    {
        return invalid(case_id, "PROCESS_OR_SCENARIO_IDENTITY_MISSING");
    }

    let scenario = text(request, &["scenario_digest"]).unwrap_or_default();
    let comparison = text(request, &["comparison_scenario_digest"]).unwrap_or_default();
    let process_scenario = text(process, &["scenario_digest"]).unwrap_or_default();
    if scenario != comparison || scenario != process_scenario {
        return result(
            case_id,
            Disposition::NotComparable,
            "SCENARIO_DIGEST_MISMATCH",
        );
    }

    let Some(evidence_kind) = text(process, &["evidence_kind"]) else {
        return invalid(case_id, "EVIDENCE_KIND_MISSING");
    };
    let Some(reported_as) = text(process, &["reported_as"]) else {
        return invalid(case_id, "REPORTED_EVIDENCE_KIND_MISSING");
    };
    if evidence_kind != reported_as {
        return result(
            case_id,
            Disposition::InvalidEvidence,
            "EVIDENCE_KIND_MISMATCH",
        );
    }

    if bool_value(process, &["cfd_used"]) == Some(true)
        && bool_value(process, &["cfd_treated_as_biological_validation"]) == Some(true)
    {
        return result(
            case_id,
            Disposition::InvalidEvidence,
            "CFD_NOT_BIOLOGICAL_VALIDATION",
        );
    }

    let Some(required_stages) = array(process, &["required_stages"]) else {
        return invalid(case_id, "REQUIRED_STAGES_MISSING");
    };
    let Some(stage_coverage) = array(process, &["stage_coverage"]) else {
        return invalid(case_id, "STAGE_COVERAGE_MISSING");
    };
    if required_stages.is_empty() || !valid_unique_strings(required_stages) {
        return invalid(case_id, "REQUIRED_STAGES_INVALID");
    }
    let stage_names: Vec<String> = stage_coverage
        .iter()
        .filter_map(|stage| text(stage, &["stage"]).map(str::to_owned))
        .collect();
    let stage_set: BTreeSet<&str> = stage_names.iter().map(String::as_str).collect();
    if stage_set.len() != stage_names.len() {
        return invalid(case_id, "DUPLICATE_STAGE_COVERAGE");
    }
    for stage in required_stages {
        let Some(name) = stage.as_str() else {
            return invalid(case_id, "REQUIRED_STAGE_INVALID");
        };
        let entry = stage_coverage
            .iter()
            .find(|entry| text(entry, &["stage"]) == Some(name));
        if entry.is_none_or(|entry| text(entry, &["state"]) != Some("accounted")) {
            return result(case_id, Disposition::Indeterminate, "PROCESS_STAGE_MISSING");
        }
    }

    let claim = text(request, &["claim"]).unwrap_or_default();
    if get(process, &["gross_product_g"]).is_none_or(Value::is_null)
        || get(process, &["usable_product_g"]).is_none_or(Value::is_null)
    {
        return result(
            case_id,
            Disposition::Indeterminate,
            "USABLE_PRODUCT_BREAKDOWN_MISSING",
        );
    }
    if get(process, &["rejected_product_g"]).is_none_or(Value::is_null) {
        return result(
            case_id,
            Disposition::Indeterminate,
            "REJECTED_BATCH_BURDEN_MISSING",
        );
    }
    if get(process, &["downstream_recovery_loss_g"]).is_none_or(Value::is_null) {
        return result(
            case_id,
            Disposition::Indeterminate,
            "DOWNSTREAM_RECOVERY_LOSS_MISSING",
        );
    }
    if let Err(reason) = product_breakdown_is_closed(process) {
        return if reason.ends_with("_MISSING") {
            result(case_id, Disposition::Indeterminate, reason)
        } else {
            result(case_id, Disposition::Blocked, reason)
        };
    }

    let Some(required_resources) = array(process, &["required_resources"]) else {
        return invalid(case_id, "REQUIRED_RESOURCES_MISSING");
    };
    let Some(balances) = array(process, &["resource_balances"]) else {
        return invalid(case_id, "RESOURCE_BALANCES_MISSING");
    };
    if required_resources.is_empty() || !valid_unique_strings(required_resources) {
        return invalid(case_id, "REQUIRED_RESOURCES_INVALID");
    }
    let required_resource_names: Vec<&str> = required_resources
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if required_resource_names
        .iter()
        .any(|resource| base_unit(resource).is_none())
    {
        return invalid(case_id, "REQUIRED_RESOURCE_UNKNOWN");
    }

    let mut stream_ids = BTreeSet::new();
    let mut seen_resources = BTreeSet::new();
    for balance in balances {
        let Some(stream_id) = text(balance, &["stream_id"]) else {
            return invalid(case_id, "RESOURCE_STREAM_ID_MISSING");
        };
        if stream_id.trim().is_empty() || !stream_ids.insert(stream_id) {
            return invalid(case_id, "DUPLICATE_RESOURCE_STREAM_ID");
        }
        let Some(resource) = text(balance, &["resource"]) else {
            return invalid(case_id, "RESOURCE_KIND_MISSING");
        };
        seen_resources.insert(resource);
        if let Err(reason) = evaluate_resource_balance(balance) {
            if reason == "RESOURCE_UNIT_MISMATCH" {
                return result(case_id, Disposition::NotComparable, reason);
            }
            if reason == "RESOURCE_BALANCE_MISMATCH" {
                return result(case_id, Disposition::Blocked, reason);
            }
            return invalid(case_id, reason);
        }
    }
    for required in &required_resource_names {
        if !seen_resources.contains(required) {
            return result(
                case_id,
                Disposition::Indeterminate,
                "REQUIRED_RESOURCE_BALANCE_MISSING",
            );
        }
    }

    match text(process, &["residual_stream_status"]) {
        Some("accounted") => {}
        Some("unknown") | None => {
            return result(
                case_id,
                Disposition::Indeterminate,
                "RESIDUAL_STREAM_UNKNOWN",
            );
        }
        Some(_) => {
            return result(
                case_id,
                Disposition::Indeterminate,
                "RESIDUAL_STREAM_NOT_CLOSED",
            );
        }
    }

    if claim == "scale_projection"
        && text(process, &["scale_transfer_assessment"]) == Some("explicit_unvalidated")
    {
        return result(
            case_id,
            Disposition::Indeterminate,
            "SCALE_TRANSFER_UNQUALIFIED",
        );
    }

    match text(process, &["quality_gate"]) {
        Some("pass") => {}
        Some("fail") => return result(case_id, Disposition::Blocked, "QUALITY_GATE_FAILED"),
        _ => return result(case_id, Disposition::Indeterminate, "QUALITY_GATE_UNKNOWN"),
    }
    if text(process, &["source_traceability"]) != Some("complete") {
        return result(
            case_id,
            Disposition::Indeterminate,
            "SOURCE_TRACEABILITY_INCOMPLETE",
        );
    }

    if matches!(claim, "scenario_comparison" | "scale_projection")
        && (bool_value(process, &["cost_inputs_complete"]) != Some(true)
            || !nonempty(process, &["cost_basis"]))
    {
        return result(
            case_id,
            Disposition::Indeterminate,
            "COST_INPUTS_INCOMPLETE",
        );
    }

    result(
        case_id,
        Disposition::EligibleForDeclaredComparison,
        "ALL_DECLARED_SYNTHETIC_GATES_PASS",
    )
}

pub fn evaluate_corpus(corpus: &Value) -> Vec<Evaluation> {
    corpus
        .get("cases")
        .and_then(Value::as_array)
        .map(|cases| cases.iter().map(evaluate_case).collect())
        .unwrap_or_default()
}


fn validate_corpus_header(corpus: &Value) -> Result<&[Value], &'static str> {
    if text(corpus, &["corpus_id"]) != Some(EXPECTED_CORPUS_ID)
        || text(corpus, &["schema_version"]) != Some(EXPECTED_SCHEMA_VERSION)
    {
        return Err("CORPUS_ID_OR_SCHEMA_VERSION_MISMATCH");
    }
    if text(corpus, &["status"]) != Some("synthetic_design_fixture_only") {
        return Err("CORPUS_STATUS_MISMATCH");
    }
    if text(corpus, &["case_schema_file"]) != Some("bioprocess-closure-case-schema-v1.json") {
        return Err("CASE_SCHEMA_FILE_MISMATCH");
    }
    if text(corpus, &["unit_contract", "mass"]) != Some("g")
        || text(corpus, &["unit_contract", "water"]) != Some("mL")
        || text(corpus, &["unit_contract", "energy"]) != Some("Wh")
        || text(corpus, &["unit_contract", "time"]) != Some("s")
    {
        return Err("CORPUS_UNIT_CONTRACT_MISMATCH");
    }
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("CORPUS_CASES_MISSING")?;
    if get(corpus, &["case_count"]).and_then(Value::as_u64) != Some(cases.len() as u64) {
        return Err("CORPUS_CASE_COUNT_MISMATCH");
    }
    let mut ids = BTreeSet::new();
    for case in cases {
        let Some(case_id) = text(case, &["case_id"]) else {
            return Err("CASE_IDS_MISSING_OR_DUPLICATED");
        };
        if case_id.trim().is_empty() || !ids.insert(case_id) {
            return Err("CASE_IDS_MISSING_OR_DUPLICATED");
        }
    }
    Ok(cases.as_slice())
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn expected_fields(case: &Value) -> (String, Vec<String>) {
    let disposition = text(case, &["expected", "disposition"])
        .unwrap_or("invalid_input")
        .to_owned();
    let reasons = array(case, &["expected", "reason_codes"])
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    (disposition, reasons)
}

pub fn run_cli() -> i32 {
    let mut expected_head = None::<String>;
    let mut receipt_path = None::<PathBuf>;
    let mut corpus_path = None::<PathBuf>;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--expected-head" => expected_head = args.next(),
            "--receipt" => receipt_path = args.next().map(PathBuf::from),
            "--corpus" => corpus_path = args.next().map(PathBuf::from),
            _ => {
                eprintln!("unknown argument: {arg}");
                return 2;
            }
        }
    }
    let Some(expected_head) = expected_head else {
        eprintln!("missing --expected-head");
        return 2;
    };
    let Some(receipt_path) = receipt_path else {
        eprintln!("missing --receipt");
        return 2;
    };
    let Some(corpus_path) = corpus_path else {
        eprintln!("missing --corpus");
        return 2;
    };

    let actual_head = match Command::new("git").args(["rev-parse", "HEAD"]).output() {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => {
            eprintln!("could not establish actual Git HEAD");
            return 2;
        }
    };
    if actual_head != expected_head {
        eprintln!("exact-head mismatch: expected={expected_head} actual={actual_head}");
        return 2;
    }

    let corpus_bytes = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus {}: {error}", corpus_path.display());
            return 2;
        }
    };
    let corpus: Value = match serde_json::from_slice(&corpus_bytes) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("cannot parse corpus: {error}");
            return 2;
        }
    };
    let cases = match validate_corpus_header(&corpus) {
        Ok(cases) => cases,
        Err(error) => {
            eprintln!("invalid corpus header: {error}");
            return 2;
        }
    };

    let evaluations = evaluate_corpus(&corpus);
    let receipts: Vec<CaseReceipt> = cases
        .iter()
        .zip(evaluations.iter())
        .map(|(case, evaluation)| {
            let (expected_disposition, expected_reason_codes) = expected_fields(case);
            let actual_disposition = format!("{:?}", evaluation.disposition);
            let actual_disposition = to_snake_case(&actual_disposition);
            let contract_match = expected_disposition == actual_disposition
                && expected_reason_codes == evaluation.reason_codes;
            CaseReceipt {
                case_id: evaluation.case_id.clone(),
                expected_disposition,
                actual_disposition: evaluation.disposition,
                expected_reason_codes,
                actual_reason_codes: evaluation.reason_codes.clone(),
                contract_match,
            }
        })
        .collect();
    let failed_case_ids: Vec<String> = receipts
        .iter()
        .filter(|receipt| !receipt.contract_match)
        .map(|receipt| receipt.case_id.clone())
        .collect();
    let matched_case_count = receipts.len() - failed_case_ids.len();
    let contract_passed = failed_case_ids.is_empty();
    let exact_head_match = actual_head == expected_head;
    let receipt = RunReceipt {
        runner_version: RUNNER_VERSION.to_owned(),
        corpus_id: EXPECTED_CORPUS_ID.to_owned(),
        schema_version: EXPECTED_SCHEMA_VERSION.to_owned(),
        corpus_sha256: sha256_hex(&corpus_bytes),
        declared_head: expected_head,
        actual_head,
        exact_head_match,
        case_count: receipts.len(),
        matched_case_count,
        failed_case_ids,
        cases: receipts,
        synthetic_only_claim_ceiling: CLAIM_CEILING.to_owned(),
        contract_passed,
    };
    if let Some(parent) = receipt_path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        eprintln!("cannot create receipt directory: {error}");
        return 2;
    }
    let receipt_json = match serde_json::to_vec_pretty(&receipt) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot serialize receipt: {error}");
            return 2;
        }
    };
    if let Err(error) = fs::write(&receipt_path, &receipt_json) {
        eprintln!("cannot write receipt {}: {error}", receipt_path.display());
        return 2;
    }
    print!("{}", String::from_utf8_lossy(&receipt_json));
    println!();
    if contract_passed { 0 } else { 1 }
}

fn to_snake_case(value: &str) -> String {
    let mut result = String::new();
    for (index, character) in value.chars().enumerate() {
        if character.is_uppercase() && index > 0 {
            result.push('_');
        }
        result.extend(character.to_lowercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS_JSON: &str =
        include_str!("../../../docs/research/bioprocess/bioprocess-closure-corpus-v1.json");

    fn corpus() -> Value {
        serde_json::from_str(CORPUS_JSON).expect("frozen process corpus parses")
    }

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions_and_reasons() {
        let input = corpus();
        assert_eq!(evaluate_corpus(&input).len(), 12);
        for (case, actual) in input["cases"]
            .as_array()
            .unwrap()
            .iter()
            .zip(evaluate_corpus(&input))
        {
            let (expected, reasons) = expected_fields(case);
            assert_eq!(
                to_snake_case(&format!("{:?}", actual.disposition)),
                expected,
                "{}",
                actual.case_id
            );
            assert_eq!(actual.reason_codes, reasons, "{}", actual.case_id);
        }
    }

    #[test]
    fn every_declared_balance_is_exact_or_within_explicit_tolerance() {
        let input = corpus();
        let case = &input["cases"][0];
        for balance in case["process"]["resource_balances"].as_array().unwrap() {
            assert_eq!(evaluate_resource_balance(balance), Ok(()));
        }
    }

    #[test]
    fn stream_order_does_not_change_semantic_result() {
        let mut input = corpus();
        let original = evaluate_case(&input["cases"][0]);
        input["cases"][0]["process"]["resource_balances"]
            .as_array_mut()
            .unwrap()
            .reverse();
        input["cases"][0]["process"]["stage_coverage"]
            .as_array_mut()
            .unwrap()
            .reverse();
        let changed = evaluate_case(&input["cases"][0]);
        assert_eq!(original, changed);
    }

    #[test]
    fn malformed_balance_fails_closed_instead_of_becoming_zero() {
        let mut input = corpus();
        input["cases"][0]["process"]["resource_balances"][0]["inflow"] =
            Value::String("01".to_owned());
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::InvalidInput);
        assert_eq!(result.reason_codes, vec!["RESOURCE_QUANTITY_NON_CANONICAL"]);
    }

    #[test]
    fn duplicate_stream_ids_are_invalid() {
        let mut input = corpus();
        let first_stream_id =
            input["cases"][0]["process"]["resource_balances"][0]["stream_id"].clone();
        input["cases"][0]["process"]["resource_balances"][1]["stream_id"] = first_stream_id;
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::InvalidInput);
        assert_eq!(result.reason_codes, vec!["DUPLICATE_RESOURCE_STREAM_ID"]);
    }

    #[test]
    fn missing_required_energy_balance_is_not_treated_as_zero() {
        let mut input = corpus();
        input["cases"][0]["process"]["resource_balances"]
            .as_array_mut()
            .unwrap()
            .retain(|balance| balance["resource"] != "energy");
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::Indeterminate);
        assert_eq!(
            result.reason_codes,
            vec!["REQUIRED_RESOURCE_BALANCE_MISSING"]
        );
    }

    #[test]
    fn product_breakdown_must_conserve_gross_product() {
        let mut input = corpus();
        input["cases"][0]["process"]["usable_product_g"] = Value::String("799".to_owned());
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::Blocked);
        assert_eq!(result.reason_codes, vec!["PRODUCT_BREAKDOWN_MISMATCH"]);
    }

    #[test]
    fn duplicate_case_ids_fail_corpus_contract_at_runner_boundary() {
        let mut input = corpus();
        let id = input["cases"][0]["case_id"].clone();
        input["cases"][1]["case_id"] = id;
        assert_eq!(
            validate_corpus_header(&input),
            Err("CASE_IDS_MISSING_OR_DUPLICATED")
        );
    }

    #[test]
    fn corpus_count_and_units_are_part_of_the_immutable_contract() {
        let mut input = corpus();
        input["case_count"] = Value::from(11);
        assert_eq!(
            validate_corpus_header(&input),
            Err("CORPUS_CASE_COUNT_MISMATCH")
        );

        let mut input = corpus();
        input["unit_contract"]["energy"] = Value::String("kJ".to_owned());
        assert_eq!(
            validate_corpus_header(&input),
            Err("CORPUS_UNIT_CONTRACT_MISMATCH")
        );
    }

    #[test]
    fn resource_unit_conversion_is_never_implicit() {
        let mut input = corpus();
        input["cases"][0]["process"]["resource_balances"][2]["unit"] =
            Value::String("kJ".to_owned());
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert_eq!(result.reason_codes, vec!["RESOURCE_UNIT_MISMATCH"]);
    }

    #[test]
    fn scenario_mismatch_blocks_comparison() {
        let mut input = corpus();
        input["cases"][0]["request"]["comparison_scenario_digest"] =
            Value::String("sha256:different".to_owned());
        let result = evaluate_case(&input["cases"][0]);
        assert_eq!(result.disposition, Disposition::NotComparable);
        assert_eq!(result.reason_codes, vec!["SCENARIO_DIGEST_MISMATCH"]);
    }

    #[test]
    fn cfd_is_not_accepted_as_biological_validation() {
        let input = corpus();
        let result = evaluate_case(&input["cases"][8]);
        assert_eq!(result.disposition, Disposition::InvalidEvidence);
        assert_eq!(result.reason_codes, vec!["CFD_NOT_BIOLOGICAL_VALIDATION"]);
    }
}
