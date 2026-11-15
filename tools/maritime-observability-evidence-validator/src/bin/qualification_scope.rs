// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic scope evaluation for previously qualified maritime capabilities.
//!
//! This is a read-only metadata compiler. It does not authenticate issuers,
//! execute machinery, authorize maintenance, or establish physical validity.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};

const VERSION: &str = "maritime-qualification-scope-v0.1";
#[cfg(test)]
const FIXTURE: &str = include_str!(
    "../../../../benchmarks/maritime-observability/qualification-scope-cases.v0.1.json"
);

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_git_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Accept only real calendar instants in canonical UTC form.
fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || [4, 7].iter().any(|i| bytes[*i] != b'-')
        || bytes[10] != b'T'
        || [13, 16].iter().any(|i| bytes[*i] != b':')
        || bytes[19] != b'Z'
        || bytes
            .iter()
            .enumerate()
            .any(|(i, b)| !matches!(i, 4 | 7 | 10 | 13 | 16 | 19) && !b.is_ascii_digit())
    {
        return false;
    }

    let number = |range: std::ops::Range<usize>| -> Option<u32> { value.get(range)?.parse().ok() };
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0..4),
        number(5..7),
        number(8..10),
        number(11..13),
        number(14..16),
        number(17..19),
    ) else {
        return false;
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day) && hour <= 23 && minute <= 59 && second <= 59
}

fn exact_keys(value: &Value, allowed: &[&str]) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    object.keys().all(|key| allowed.contains(&key.as_str()))
}

fn valid_case_shape(case: &Value) -> bool {
    if !exact_keys(
        case,
        &[
            "case_id",
            "qualification",
            "request",
            "expected_disposition",
        ],
    ) || text(case, "case_id").is_none()
        || text(case, "expected_disposition").is_none()
    {
        return false;
    }
    let Some(qualification) = case.get("qualification") else {
        return false;
    };
    let Some(request) = case.get("request") else {
        return false;
    };
    if !exact_keys(
        qualification,
        &[
            "qualification_id",
            "state",
            "claim_class",
            "asset_id",
            "configuration_sha256",
            "scope_sha256",
            "valid_from_utc",
            "valid_until_utc",
            "operating_regimes",
            "capability_ids",
            "retention_suite_sha256",
            "max_worst_case_regression_pp",
            "dependencies",
            "required_dependency_ids",
        ],
    ) || !exact_keys(
        request,
        &[
            "request_id",
            "asset_id",
            "configuration_sha256",
            "requested_regime",
            "dependency_evidence",
            "model_update_retention_receipt",
        ],
    ) {
        return false;
    }

    let Some(dependencies) = qualification.get("dependencies").and_then(Value::as_array) else {
        return false;
    };
    if dependencies.iter().any(|dependency| {
        !exact_keys(dependency, &["dependency_id", "sha256"])
            || text(dependency, "dependency_id").is_none()
            || text(dependency, "sha256").is_none()
    }) {
        return false;
    }

    const ALL_DEPENDENCY_IDS: &[&str] = &[
        "sensor_topology",
        "calibration",
        "timebase",
        "source_payload",
        "diagnostic_model",
        "evaluator",
        "validation_profile",
    ];
    let required: BTreeSet<String> =
        if let Some(value) = qualification.get("required_dependency_ids") {
            let Some(values) = value.as_array() else {
                return false;
            };
            let mut set = BTreeSet::new();
            for item in values {
                let Some(id) = item.as_str() else {
                    return false;
                };
                if !ALL_DEPENDENCY_IDS.contains(&id) || !set.insert(id.to_owned()) {
                    return false;
                }
            }
            if set.is_empty() {
                return false;
            }
            set
        } else {
            ALL_DEPENDENCY_IDS
                .iter()
                .map(|id| (*id).to_owned())
                .collect()
        };
    let actual_dependencies: BTreeSet<String> = dependencies
        .iter()
        .filter_map(|item| text(item, "dependency_id").map(str::to_owned))
        .collect();
    if actual_dependencies.len() != dependencies.len() || actual_dependencies != required {
        return false;
    }

    let Some(evidence_map) = request
        .get("dependency_evidence")
        .and_then(Value::as_object)
    else {
        return false;
    };
    let expected_evidence_ids: BTreeSet<String> = required.clone();
    let actual_evidence_ids: BTreeSet<String> = evidence_map.keys().cloned().collect();
    if actual_evidence_ids != expected_evidence_ids {
        return false;
    }
    const IDS: &[&str] = &[
        "sensor_topology",
        "calibration",
        "timebase",
        "source_payload",
        "diagnostic_model",
        "evaluator",
        "validation_profile",
    ];
    for (id, evidence) in evidence_map {
        if !IDS.contains(&id.as_str())
            || !exact_keys(
                evidence,
                &[
                    "sha256",
                    "evidence_state",
                    "evidence_class",
                    "valid_until_utc",
                    "payload_verified",
                    "independence_status",
                    "review_status",
                ],
            )
            || text(evidence, "sha256").is_none()
            || text(evidence, "evidence_state").is_none()
            || text(evidence, "evidence_class").is_none()
        {
            return false;
        }
    }

    let retention_receipt_shape_valid = request
        .get("model_update_retention_receipt")
        .filter(|value| !value.is_null())
        .map(|receipt| {
            exact_keys(
                receipt,
                &[
                    "receipt_id",
                    "prior_qualification_id",
                    "old_model_sha256",
                    "new_model_sha256",
                    "scope_sha256",
                    "verification_suite_sha256",
                    "evidence_profile_sha256",
                    "review_status",
                    "result",
                    "retained_capability_ids",
                    "tested_regimes",
                    "worst_case_regression_pp",
                    "maximum_allowed_regression_pp",
                    "unknown_fault_rejection_passed",
                    "calibration_retention_passed",
                    "scope_unchanged",
                ],
            ) && text(receipt, "receipt_id").is_some()
        })
        .unwrap_or(true);
    if !retention_receipt_shape_valid {
        return false;
    }
    true
}

fn string_set(value: &Value, key: &str) -> Option<BTreeSet<String>> {
    let values = value.get(key)?.as_array()?;
    let mut set = BTreeSet::new();
    for item in values {
        let value = item.as_str()?.trim();
        if value.is_empty() || !set.insert(value.to_owned()) {
            return None;
        }
    }
    Some(set)
}

fn dependency_by_id<'a>(dependencies: &'a Value, id: &str) -> Option<&'a Value> {
    dependencies
        .as_array()?
        .iter()
        .find(|item| text(item, "dependency_id") == Some(id))
}

fn request_evidence<'a>(request: &'a Value, id: &str) -> Option<&'a Value> {
    request.get("dependency_evidence")?.get(id)
}

fn retention_receipt_valid(
    qualification: &Value,
    request: &Value,
    old_model_sha256: &str,
    new_model_sha256: &str,
) -> bool {
    let Some(receipt) = request
        .get("model_update_retention_receipt")
        .filter(|v| !v.is_null())
    else {
        return false;
    };

    let Some(capabilities) = string_set(qualification, "capability_ids") else {
        return false;
    };
    let Some(regimes) = string_set(qualification, "operating_regimes") else {
        return false;
    };
    let Some(retained_capabilities) = string_set(receipt, "retained_capability_ids") else {
        return false;
    };
    let Some(tested_regimes) = string_set(receipt, "tested_regimes") else {
        return false;
    };

    let Some(profile) = dependency_by_id(
        qualification.get("dependencies").unwrap_or(&Value::Null),
        "validation_profile",
    )
    .and_then(|v| text(v, "sha256")) else {
        return false;
    };
    let Some(suite) = text(qualification, "retention_suite_sha256") else {
        return false;
    };
    let Some(scope) = text(qualification, "scope_sha256") else {
        return false;
    };
    let Some(max_regression) = qualification
        .get("max_worst_case_regression_pp")
        .and_then(Value::as_f64)
    else {
        return false;
    };
    let Some(measured_regression) = receipt
        .get("worst_case_regression_pp")
        .and_then(Value::as_f64)
    else {
        return false;
    };
    let Some(receipt_limit) = receipt
        .get("maximum_allowed_regression_pp")
        .and_then(Value::as_f64)
    else {
        return false;
    };

    text(receipt, "prior_qualification_id") == text(qualification, "qualification_id")
        && text(receipt, "old_model_sha256") == Some(old_model_sha256)
        && text(receipt, "new_model_sha256") == Some(new_model_sha256)
        && text(receipt, "scope_sha256") == Some(scope)
        && text(receipt, "verification_suite_sha256") == Some(suite)
        && text(receipt, "evidence_profile_sha256") == Some(profile)
        && text(receipt, "review_status") == Some("INDEPENDENTLY_REVIEWED")
        && text(receipt, "result") == Some("PASS")
        && is_sha256(suite)
        && is_sha256(profile)
        && is_sha256(scope)
        && capabilities == retained_capabilities
        && regimes == tested_regimes
        && receipt_limit.is_finite()
        && measured_regression.is_finite()
        && max_regression.is_finite()
        && receipt_limit >= 0.0
        && receipt_limit <= max_regression
        && measured_regression >= 0.0
        && measured_regression <= receipt_limit
        && receipt
            .get("unknown_fault_rejection_passed")
            .and_then(Value::as_bool)
            == Some(true)
        && receipt
            .get("calibration_retention_passed")
            .and_then(Value::as_bool)
            == Some(true)
        && receipt.get("scope_unchanged").and_then(Value::as_bool) == Some(true)
}

fn physical_evidence_is_admissible(qualification: &Value, request: &Value) -> bool {
    if text(qualification, "claim_class") != Some("PHYSICAL_ENGINEERING_CAPABILITY") {
        return true;
    }

    let physical_class = |id: &str| {
        request_evidence(request, id).is_some_and(|evidence| {
            matches!(
                text(evidence, "evidence_class"),
                Some("MEASURED_PHYSICAL" | "PUBLISHED_PHYSICAL_DATASET")
            )
        })
    };

    physical_class("sensor_topology")
        && physical_class("calibration")
        && physical_class("timebase")
        && physical_class("source_payload")
        && request_evidence(request, "sensor_topology").and_then(|v| text(v, "independence_status"))
            == Some("INDEPENDENTLY_AUDITED")
        && request_evidence(request, "source_payload")
            .and_then(|v| v.get("payload_verified"))
            .and_then(Value::as_bool)
            == Some(true)
        && request_evidence(request, "validation_profile").and_then(|v| text(v, "review_status"))
            == Some("INDEPENDENTLY_REVIEWED")
}

fn evaluate(
    qualification: &Value,
    request: &Value,
    as_of_utc: &str,
) -> (&'static str, &'static str) {
    match text(qualification, "state") {
        Some("QUALIFIED") => {}
        Some("EXPIRED") => return ("EXPIRED", "prior qualification is explicitly expired"),
        Some("REVOKED") => {
            return (
                "REQUALIFICATION_REQUIRED",
                "prior qualification was revoked",
            );
        }
        _ => {
            return (
                "NOT_EVALUATED",
                "prior qualification state is missing or unsupported",
            );
        }
    }
    if !matches!(
        text(qualification, "claim_class"),
        Some("SYNTHETIC_CONTRACT" | "PHYSICAL_ENGINEERING_CAPABILITY")
    ) {
        return (
            "NOT_EVALUATED",
            "qualification claim class is missing or unsupported",
        );
    }
    if string_set(qualification, "capability_ids").is_none()
        || !is_sha256(text(qualification, "scope_sha256").unwrap_or(""))
        || !is_sha256(text(qualification, "retention_suite_sha256").unwrap_or(""))
    {
        return (
            "NOT_EVALUATED",
            "qualification capability scope or retention metadata is invalid",
        );
    }

    let (Some(q_asset), Some(r_asset)) =
        (text(qualification, "asset_id"), text(request, "asset_id"))
    else {
        return ("NOT_EVALUATED", "asset identity is missing");
    };
    if q_asset != r_asset {
        return (
            "OUT_OF_SCOPE",
            "request refers to a different physical asset",
        );
    }

    let (Some(q_config), Some(r_config)) = (
        text(qualification, "configuration_sha256"),
        text(request, "configuration_sha256"),
    ) else {
        return ("NOT_EVALUATED", "asset configuration digest is missing");
    };
    if !is_sha256(q_config) || !is_sha256(r_config) {
        return ("NOT_EVALUATED", "asset configuration digest is invalid");
    }
    if q_config != r_config {
        return (
            "REQUALIFICATION_REQUIRED",
            "installed configuration digest changed",
        );
    }

    let Some(regime) = text(request, "requested_regime") else {
        return ("NOT_EVALUATED", "requested operating regime is missing");
    };
    let Some(regimes) = string_set(qualification, "operating_regimes") else {
        return ("NOT_EVALUATED", "qualified operating-regime set is invalid");
    };
    if !regimes.contains(regime) {
        return (
            "OUT_OF_SCOPE",
            "requested operating regime is outside the qualified envelope",
        );
    }

    if !is_utc_timestamp(as_of_utc) {
        return ("NOT_EVALUATED", "evaluation time is not canonical UTC");
    }
    let (Some(valid_from), Some(valid_until)) = (
        text(qualification, "valid_from_utc"),
        text(qualification, "valid_until_utc"),
    ) else {
        return (
            "NOT_EVALUATED",
            "qualification validity interval is incomplete",
        );
    };
    if !is_utc_timestamp(valid_from) || !is_utc_timestamp(valid_until) || valid_from > valid_until {
        return (
            "NOT_EVALUATED",
            "qualification validity interval is invalid",
        );
    }
    if as_of_utc < valid_from {
        return ("NOT_EVALUATED", "qualification is not yet valid");
    }
    if as_of_utc > valid_until {
        return ("EXPIRED", "qualification validity interval has elapsed");
    }

    let Some(qualified_dependencies) = qualification.get("dependencies").and_then(Value::as_array)
    else {
        return ("NOT_EVALUATED", "qualification dependency list is missing");
    };
    if qualified_dependencies.is_empty() {
        return (
            "NOT_EVALUATED",
            "qualification has no declared dependencies",
        );
    }

    let mut seen = BTreeSet::new();
    let mut model_change: Option<(String, String)> = None;
    const ALL_DEPENDENCIES: [&str; 7] = [
        "sensor_topology",
        "calibration",
        "timebase",
        "source_payload",
        "diagnostic_model",
        "evaluator",
        "validation_profile",
    ];
    let required_dependencies: BTreeSet<String> =
        if let Some(values) = qualification.get("required_dependency_ids") {
            let Some(values) = values.as_array() else {
                return ("NOT_EVALUATED", "required dependency set is malformed");
            };
            let mut required = BTreeSet::new();
            for value in values {
                let Some(id) = value.as_str() else {
                    return ("NOT_EVALUATED", "required dependency identity is malformed");
                };
                if !ALL_DEPENDENCIES.contains(&id) || !required.insert(id.to_owned()) {
                    return (
                        "NOT_EVALUATED",
                        "required dependency set contains an unknown or duplicate identity",
                    );
                }
            }
            if required.is_empty() {
                return ("NOT_EVALUATED", "required dependency set is empty");
            }
            required
        } else {
            ALL_DEPENDENCIES.iter().map(|id| (*id).to_owned()).collect()
        };
    for qualified in qualified_dependencies {
        let Some(id) = text(qualified, "dependency_id") else {
            return ("NOT_EVALUATED", "dependency identity is missing");
        };
        if !required_dependencies.contains(id) || !seen.insert(id.to_owned()) {
            return (
                "NOT_EVALUATED",
                "qualification contains an unknown or duplicate dependency identity",
            );
        }
        let Some(expected_digest) = text(qualified, "sha256") else {
            return ("NOT_EVALUATED", "qualified dependency digest is missing");
        };
        if !is_sha256(expected_digest) {
            return ("NOT_EVALUATED", "qualified dependency digest is invalid");
        }
        let Some(actual) = request_evidence(request, id) else {
            return (
                "NOT_EVALUATED",
                "a required dependency is absent from the request",
            );
        };
        match text(actual, "evidence_state") {
            Some("CONFLICTED") => {
                return ("CONFLICTED", "a required evidence source is conflicted");
            }
            Some("EXPIRED" | "STALE") => {
                return ("EXPIRED", "a required evidence source is stale or expired");
            }
            Some("UNKNOWN" | "NOT_EVALUATED" | "UNVERIFIED") | None => {
                return (
                    "NOT_EVALUATED",
                    "a required evidence source is unqualified or unknown",
                );
            }
            Some("REVOKED") => {
                return (
                    "REQUALIFICATION_REQUIRED",
                    "a required evidence qualification was revoked",
                );
            }
            Some("QUALIFIED") => {}
            Some(_) => return ("NOT_EVALUATED", "a required evidence state is unsupported"),
        }

        if let Some(expiry) = text(actual, "valid_until_utc") {
            if !is_utc_timestamp(expiry) {
                return ("NOT_EVALUATED", "dependency validity timestamp is invalid");
            }
            if as_of_utc > expiry {
                return ("EXPIRED", "a required evidence qualification has elapsed");
            }
        }

        let Some(actual_digest) = text(actual, "sha256") else {
            return ("NOT_EVALUATED", "request dependency digest is missing");
        };
        if !is_sha256(actual_digest) {
            return ("NOT_EVALUATED", "request dependency digest is invalid");
        }
        if actual_digest != expected_digest {
            if id == "diagnostic_model" {
                model_change = Some((expected_digest.to_owned(), actual_digest.to_owned()));
            } else {
                return (
                    "REQUALIFICATION_REQUIRED",
                    "a non-model prerequisite digest changed",
                );
            }
        }
    }
    if seen != required_dependencies {
        return (
            "NOT_EVALUATED",
            "qualification omits one or more required dependency identities",
        );
    }

    if !physical_evidence_is_admissible(qualification, request) {
        return (
            "NOT_EVALUATED",
            "physical claim lacks independently qualified physical source evidence",
        );
    }

    if let Some((old_model, new_model)) = model_change {
        if retention_receipt_valid(qualification, request, &old_model, &new_model) {
            return (
                "REQUALIFIED",
                "exact-scope independent model-retention receipt passed",
            );
        }
        return (
            "REQUALIFICATION_REQUIRED",
            "model changed without a complete exact-scope retention receipt",
        );
    }

    (
        "SUPPORTED",
        "all declared scope and dependency predicates match",
    )
}

fn evaluate_document(input: &Value, source_commit: &str, case_sha256: &str) -> Value {
    let mut counts = BTreeMap::<String, usize>::new();
    let mut results = Vec::new();
    let as_of = text(input, "as_of_utc").unwrap_or("");
    let version_ok = text(input, "schema_version") == Some(VERSION)
        && text(input, "evidence_class") == Some("SYNTHETIC_KNOWN_TRUTH")
        && exact_keys(
            input,
            &["schema_version", "evidence_class", "as_of_utc", "cases"],
        )
        && is_utc_timestamp(as_of);
    let cases = input.get("cases").and_then(Value::as_array);

    if let Some(cases) = cases {
        for case in cases {
            let request_id = text(case, "case_id").unwrap_or("missing-case-id");
            let expected = text(case, "expected_disposition").unwrap_or("INVALID_EXPECTATION");
            let (actual, reason) = match (case.get("qualification"), case.get("request")) {
                (Some(q), Some(r)) if version_ok && valid_case_shape(case) => evaluate(q, r, as_of),
                _ => (
                    "NOT_EVALUATED",
                    "case schema version, evidence class, or strict case shape is invalid",
                ),
            };
            *counts.entry(actual.to_owned()).or_default() += 1;
            results.push(json!({
                "case_id": request_id,
                "expected_disposition": expected,
                "actual_disposition": actual,
                "passed": expected == actual,
                "reason": reason
            }));
        }
    }

    let passed = version_ok
        && cases.is_some()
        && !results.is_empty()
        && results
            .iter()
            .all(|item| item["passed"].as_bool() == Some(true));

    json!({
        "schema_version": VERSION,
        "result": if passed { "pass" } else { "fail" },
        "source_commit": source_commit,
        "case_sha256": case_sha256,
        "as_of_utc": as_of,
        "case_count": results.len(),
        "disposition_counts": counts,
        "cases": results,
        "mutation_performed": false,
        "authorization_granted": false,
        "physical_qualification_established": false,
        "claim_ceiling": "synthetic scope-calculus conformance only"
    })
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!(
            "usage: qualification_scope <fixture.json> --source-commit <sha> --case-sha256 <sha256>"
        );
        return ExitCode::from(2);
    };
    let mut source_commit = String::new();
    let mut expected_digest = String::new();
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--source-commit" => source_commit = args.next().unwrap_or_default(),
            "--case-sha256" => expected_digest = args.next().unwrap_or_default(),
            _ => {
                eprintln!("unknown argument: {flag}");
                return ExitCode::from(2);
            }
        }
    }
    if !is_git_sha(&source_commit) || !is_sha256(&expected_digest) {
        eprintln!("source commit and case digest must be valid hexadecimal digests");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let actual_digest = format!("{:x}", Sha256::digest(&bytes));
    if actual_digest != expected_digest {
        eprintln!("case digest mismatch: expected {expected_digest}, got {actual_digest}");
        return ExitCode::from(2);
    }
    let input: Value = match serde_json::from_slice(&bytes) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid JSON fixture: {error}");
            return ExitCode::from(2);
        }
    };

    let report = evaluate_document(&input, &source_commit, &actual_digest);
    let passed = report["result"].as_str() == Some("pass");
    println!(
        "{}",
        serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_owned())
    );
    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("frozen qualification-scope fixture must parse")
    }

    #[test]
    fn frozen_scope_cases_match_all_expected_dispositions() {
        let input = fixture();
        let report = evaluate_document(
            &input,
            "0123456789abcdef0123456789abcdef01234567",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert_eq!(report["result"], "pass");
        assert_eq!(report["case_count"], 20);
        assert_eq!(report["disposition_counts"]["SUPPORTED"], 1);
        assert_eq!(report["disposition_counts"]["REQUALIFIED"], 1);
        assert_eq!(report["disposition_counts"]["REQUALIFICATION_REQUIRED"], 7);
        assert_eq!(report["disposition_counts"]["OUT_OF_SCOPE"], 2);
        assert_eq!(report["disposition_counts"]["NOT_EVALUATED"], 5);
        assert_eq!(report["disposition_counts"]["CONFLICTED"], 1);
        assert_eq!(report["disposition_counts"]["EXPIRED"], 3);
    }

    #[test]
    fn explicit_dependency_subset_supports_capability_specific_qualification() {
        let input = fixture();
        let mut case = input["cases"][0].clone();
        case["qualification"]["capability_ids"] = serde_json::json!(["sail.monitor_wind"]);
        case["qualification"]["required_dependency_ids"] =
            serde_json::json!(["sensor_topology", "calibration", "timebase"]);
        let retained = ["sensor_topology", "calibration", "timebase"];
        case["qualification"]["dependencies"] = serde_json::Value::Array(
            case["qualification"]["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|dependency| {
                    retained.contains(&dependency["dependency_id"].as_str().unwrap())
                })
                .cloned()
                .collect(),
        );
        let evidence = case["request"]["dependency_evidence"]
            .as_object_mut()
            .unwrap();
        evidence.retain(|key, _| retained.contains(&key.as_str()));
        assert!(valid_case_shape(&case));
        assert_eq!(
            evaluate(
                &case["qualification"],
                &case["request"],
                "2026-11-14T00:00:00Z"
            )
            .0,
            "SUPPORTED"
        );
    }

    #[test]
    fn explicit_dependency_subset_cannot_omit_a_declared_prerequisite() {
        let input = fixture();
        let mut case = input["cases"][0].clone();
        case["qualification"]["required_dependency_ids"] =
            serde_json::json!(["sensor_topology", "calibration", "timebase"]);
        case["qualification"]["dependencies"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(!valid_case_shape(&case));
    }

    #[test]
    fn explicit_dependency_subset_rejects_missing_required_evidence() {
        let input = fixture();
        let mut case = input["cases"][0].clone();
        case["qualification"]["required_dependency_ids"] =
            serde_json::json!(["sensor_topology", "calibration", "timebase"]);
        let retained = ["sensor_topology", "calibration", "timebase"];
        case["qualification"]["dependencies"] = serde_json::Value::Array(
            case["qualification"]["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|dependency| {
                    retained.contains(&dependency["dependency_id"].as_str().unwrap())
                })
                .cloned()
                .collect(),
        );
        let evidence = case["request"]["dependency_evidence"]
            .as_object_mut()
            .unwrap();
        evidence.retain(|key, _| key != "calibration" && retained.contains(&key.as_str()));
        assert!(!valid_case_shape(&case));
        assert_eq!(
            evaluate(
                &case["qualification"],
                &case["request"],
                "2026-11-14T00:00:00Z"
            )
            .0,
            "NOT_EVALUATED"
        );
    }

    #[test]
    fn legacy_cases_without_dependency_subset_retain_the_seven_dependency_default() {
        let input = fixture();
        let case = input["cases"][0].clone();
        assert!(
            case["qualification"]
                .get("required_dependency_ids")
                .is_none()
        );
        assert!(valid_case_shape(&case));
        assert_eq!(
            evaluate(
                &case["qualification"],
                &case["request"],
                "2026-11-14T00:00:00Z"
            )
            .0,
            "SUPPORTED"
        );
    }

    #[test]
    fn model_update_without_retention_evidence_never_inherits_qualification() {
        let input = fixture();
        let case = input["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["case_id"] == "model-changed-no-retention")
            .unwrap();
        let (actual, _) = evaluate(
            &case["qualification"],
            &case["request"],
            "2026-11-14T00:00:00Z",
        );
        assert_eq!(actual, "REQUALIFICATION_REQUIRED");
    }

    #[test]
    fn retention_receipt_cannot_expand_the_qualified_capability_set() {
        let input = fixture();
        let case = input["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["case_id"] == "model-update-widened-capabilities")
            .unwrap();
        let (actual, _) = evaluate(
            &case["qualification"],
            &case["request"],
            "2026-11-14T00:00:00Z",
        );
        assert_eq!(actual, "REQUALIFICATION_REQUIRED");
    }

    #[test]
    fn simulator_payload_cannot_support_a_physical_capability_claim() {
        let input = fixture();
        let case = input["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["case_id"] == "physical-claim-simulator-only")
            .unwrap();
        let (actual, _) = evaluate(
            &case["qualification"],
            &case["request"],
            "2026-11-14T00:00:00Z",
        );
        assert_eq!(actual, "NOT_EVALUATED");
    }

    #[test]
    fn invalid_calendar_date_is_not_accepted_as_canonical_utc() {
        assert!(!is_utc_timestamp("2026-02-30T00:00:00Z"));
        assert!(!is_utc_timestamp("2026-13-01T00:00:00Z"));
        assert!(!is_utc_timestamp("2026-11-14T25:00:00Z"));
        assert!(is_utc_timestamp("2026-11-14T23:59:59Z"));
    }

    #[test]
    fn unknown_evidence_dependency_keys_fail_closed() {
        let input = fixture();
        let mut case = input["cases"][0].clone();
        case["request"]["dependency_evidence"]["unregistered_shadow_channel"] = serde_json::json!({"sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                               "evidence_state":"QUALIFIED","evidence_class":"MEASURED_PHYSICAL"});
        assert!(!valid_case_shape(&case));
    }

    #[test]
    fn qualification_must_declare_the_complete_dependency_set() {
        let input = fixture();
        let case = input["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["case_id"] == "qualification-omits-required-dependency")
            .unwrap();
        let (actual, _) = evaluate(
            &case["qualification"],
            &case["request"],
            "2026-11-14T00:00:00Z",
        );
        assert_eq!(actual, "NOT_EVALUATED");
    }

    #[test]
    fn receipts_are_deterministic_for_frozen_inputs() {
        let input = fixture();
        let commit = "0123456789abcdef0123456789abcdef01234567";
        let digest = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        assert_eq!(
            evaluate_document(&input, commit, digest),
            evaluate_document(&input, commit, digest)
        );
    }
}
