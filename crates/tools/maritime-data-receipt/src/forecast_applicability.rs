// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Decision-time applicability checks for versioned water-level prediction runs.
//!
//! This module evaluates declared temporal metadata only. A temporally applicable
//! forecast remains NotAdmitted for physical truth, engineering or navigation.

use serde::Serialize;
use std::env;
use std::fs;
use std::path::PathBuf;

const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "profile_id",
    "source_kind",
    "receipt_binding",
    "forecast",
    "quality",
    "intended_use",
];
const BINDING_KEYS: &[&str] = &["receipt_id", "receipt_sha256", "artifact_sha256"];
const FORECAST_KEYS: &[&str] = &[
    "product_kind",
    "provider_product_id",
    "dataset_id",
    "dataset_revision",
    "model_run_id",
    "model_initialization_time_utc",
    "issued_at_utc",
    "assessment_time_utc",
    "target_time_utc",
    "valid_from_utc",
    "valid_until_utc",
    "max_issue_age_seconds",
    "max_forecast_lead_seconds",
    "freshness_policy_id",
];
const QUALITY_KEYS: &[&str] = &["status", "uncertainty_statement"];
const BINDING_PATH: &str = "forecast_profile.receipt_binding";
const FORECAST_PATH: &str = "forecast_profile.forecast";
const QUALITY_PATH: &str = "forecast_profile.quality";
const USE_KEYS: &[&str] = &[
    "scenario_analysis_only",
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "berth_availability_inference_authorized",
    "independent_review_required",
];

#[derive(Debug, Serialize)]
struct ForecastApplicabilityReport {
    status: &'static str,
    profile_id: String,
    profile_sha256: String,
    receipt_id: String,
    receipt_sha256: String,
    artifact_sha256: String,
    product_kind: String,
    provider_product_id: String,
    dataset_id: String,
    dataset_revision: String,
    model_run_id: String,
    model_initialization_time_utc: String,
    issued_at_utc: String,
    assessment_time_utc: String,
    target_time_utc: String,
    valid_from_utc: String,
    valid_until_utc: String,
    issue_age_seconds: Option<u64>,
    forecast_lead_seconds: Option<u64>,
    max_issue_age_seconds: u64,
    max_forecast_lead_seconds: u64,
    freshness_policy_id: String,
    findings: Vec<&'static str>,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    berth_availability_inference_authorized: bool,
    independent_review_required: bool,
    source_commit_binding: String,
    execution_performed: bool,
}

fn text_field<'a>(value: &'a super::JsonValue, key: &str, path: &str) -> Result<&'a str, String> {
    super::require_nonempty_text(super::field(value, key, path)?, &format!("{path}.{key}"))
}

fn digest_is_lower_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn pinned_identifier(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    !normalized.is_empty()
        && ![
            "latest",
            "current",
            "head",
            "unknown",
            "unspecified",
            "unversioned",
        ]
        .contains(&normalized.as_str())
}

fn timestamp_seconds(value: &str, path: &str) -> Result<i64, String> {
    if value.len() != 20 || !super::valid_utc_timestamp(value) {
        return Err(format!(
            "{path}: expected canonical whole-second UTC timestamp YYYY-MM-DDTHH:MM:SSZ"
        ));
    }

    let number = |start: usize, end: usize| {
        value[start..end]
            .parse::<i64>()
            .map_err(|_| format!("{path}: invalid timestamp digits"))
    };
    let year = number(0, 4)?;
    if year == 0 {
        return Err(format!(
            "{path}: year zero is not a valid RFC3339 timestamp"
        ));
    }
    let month = number(5, 7)?;
    let day = number(8, 10)?;
    let hour = number(11, 13)?;
    let minute = number(14, 16)?;
    let second = number(17, 19)?;

    // Proleptic Gregorian calendar conversion to days. The timestamp parser
    // above already checks month/day bounds; this conversion only compares instants.
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era;
    Ok(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn validate_and_evaluate(
    profile: &super::JsonValue,
    facts: &super::ReceiptFacts,
    receipt_sha256: &str,
    profile_sha256: &str,
    source_commit: &str,
) -> Result<ForecastApplicabilityReport, String> {
    super::exact_keys(profile, ROOT_KEYS, "forecast_profile")?;
    if text_field(profile, "schema_id", "forecast_profile")?
        != "luminous.symtropy.maritime-forecast-applicability"
        || text_field(profile, "schema_version", "forecast_profile")? != "0.1.0"
    {
        return Err("forecast_profile: unsupported schema identity/version".to_owned());
    }
    let profile_id = text_field(profile, "profile_id", "forecast_profile")?.to_owned();
    let source_kind = text_field(profile, "source_kind", "forecast_profile")?;
    if !["synthetic_fixture", "public_dataset"].contains(&source_kind) {
        return Err("forecast_profile.source_kind: unsupported value".to_owned());
    }
    if source_kind != facts.source_kind.as_str() {
        return Err("forecast_profile.source_kind does not match bound receipt".to_owned());
    }
    if facts.product != "water_level_prediction" || facts.domain != "water_level_prediction" {
        return Err(
            "forecast applicability profile requires a water_level_prediction receipt".to_owned(),
        );
    }

    let binding = super::field(profile, "receipt_binding", "forecast_profile")?;
    super::exact_keys(binding, BINDING_KEYS, BINDING_PATH)?;
    let receipt_id = text_field(binding, "receipt_id", BINDING_PATH)?;
    let declared_receipt_sha = text_field(binding, "receipt_sha256", BINDING_PATH)?;
    let declared_artifact_sha = text_field(binding, "artifact_sha256", BINDING_PATH)?;
    if !digest_is_lower_sha256(declared_receipt_sha)
        || !digest_is_lower_sha256(declared_artifact_sha)
    {
        return Err("forecast_profile.receipt_binding: invalid lowercase SHA-256".to_owned());
    }
    if receipt_id != facts.receipt_id {
        return Err("forecast_profile.receipt_binding.receipt_id mismatch".to_owned());
    }
    if declared_receipt_sha != receipt_sha256 {
        return Err("forecast_profile.receipt_binding.receipt_sha256 mismatch".to_owned());
    }
    if declared_artifact_sha != facts.artifact.sha256 {
        return Err("forecast_profile.receipt_binding.artifact_sha256 mismatch".to_owned());
    }

    let forecast = super::field(profile, "forecast", "forecast_profile")?;
    super::exact_keys(forecast, FORECAST_KEYS, FORECAST_PATH)?;
    let product_kind = text_field(forecast, "product_kind", FORECAST_PATH)?.to_owned();
    let provider_product_id =
        text_field(forecast, "provider_product_id", FORECAST_PATH)?.to_owned();
    let dataset_id = text_field(forecast, "dataset_id", FORECAST_PATH)?.to_owned();
    let dataset_revision = text_field(forecast, "dataset_revision", FORECAST_PATH)?.to_owned();
    let model_run_id = text_field(forecast, "model_run_id", FORECAST_PATH)?.to_owned();
    let model_initialization_time =
        text_field(forecast, "model_initialization_time_utc", FORECAST_PATH)?.to_owned();
    let issued_at = text_field(forecast, "issued_at_utc", FORECAST_PATH)?.to_owned();
    let assessment_time = text_field(forecast, "assessment_time_utc", FORECAST_PATH)?.to_owned();
    let target_time = text_field(forecast, "target_time_utc", FORECAST_PATH)?.to_owned();
    let valid_from = text_field(forecast, "valid_from_utc", FORECAST_PATH)?.to_owned();
    let valid_until = text_field(forecast, "valid_until_utc", FORECAST_PATH)?.to_owned();
    let max_issue_age = super::integer(
        super::field(forecast, "max_issue_age_seconds", FORECAST_PATH)?,
        "forecast_profile.forecast.max_issue_age_seconds",
    )?;
    let max_forecast_lead = super::integer(
        super::field(forecast, "max_forecast_lead_seconds", FORECAST_PATH)?,
        "forecast_profile.forecast.max_forecast_lead_seconds",
    )?;
    let freshness_policy_id =
        text_field(forecast, "freshness_policy_id", FORECAST_PATH)?.to_owned();

    if product_kind != facts.product.as_str() {
        return Err(
            "forecast_profile.forecast.product_kind does not match receipt product kind".to_owned(),
        );
    }
    if dataset_id != facts.dataset_id || dataset_revision != facts.dataset_revision {
        return Err("forecast profile dataset ID/revision does not match bound receipt".to_owned());
    }
    if !pinned_identifier(&provider_product_id)
        || !pinned_identifier(&dataset_revision)
        || !pinned_identifier(&model_run_id)
    {
        return Err(
            "forecast profile requires pinned dataset revision and model-run identity".to_owned(),
        );
    }
    if max_issue_age == 0 || max_forecast_lead == 0 {
        return Err("forecast policy bounds must be positive integers".to_owned());
    }

    let quality = super::field(profile, "quality", "forecast_profile")?;
    super::exact_keys(quality, QUALITY_KEYS, QUALITY_PATH)?;
    let quality_status = text_field(quality, "status", QUALITY_PATH)?;
    let uncertainty_statement = text_field(quality, "uncertainty_statement", QUALITY_PATH)?;
    if !["prediction", "synthetic", "unknown"].contains(&quality_status) {
        return Err("forecast_profile.quality.status: unsupported value".to_owned());
    }
    if source_kind == "synthetic_fixture" && quality_status == "prediction" {
        return Err("synthetic forecast cannot claim public prediction quality status".to_owned());
    }
    if source_kind == "public_dataset" && quality_status == "synthetic" {
        return Err("public dataset cannot claim synthetic quality status".to_owned());
    }
    if uncertainty_statement != facts.uncertainty_statement {
        return Err(
            "forecast profile uncertainty statement does not match bound receipt".to_owned(),
        );
    }

    let intended = super::field(profile, "intended_use", "forecast_profile")?;
    super::exact_keys(intended, USE_KEYS, "forecast_profile.intended_use")?;
    let scenario_only = super::bool_value(
        super::field(intended, "scenario_analysis_only", "intended_use")?,
        "intended_use.scenario_analysis_only",
    )?;
    if !scenario_only {
        return Err("scenario_analysis_only must remain true".to_owned());
    }
    for key in [
        "navigation_authorized",
        "physical_truth_claim_authorized",
        "berth_availability_inference_authorized",
    ] {
        if super::bool_value(
            super::field(intended, key, "intended_use")?,
            &format!("intended_use.{key}"),
        )? {
            return Err(format!("intended_use.{key} must remain false"));
        }
    }
    let independent_review_required = super::bool_value(
        super::field(intended, "independent_review_required", "intended_use")?,
        "intended_use.independent_review_required",
    )?;
    if !independent_review_required {
        return Err("independent_review_required must remain true".to_owned());
    }

    let init_seconds =
        timestamp_seconds(&model_initialization_time, "model_initialization_time_utc")?;
    let issue_seconds = timestamp_seconds(&issued_at, "issued_at_utc")?;
    let assessment_seconds = timestamp_seconds(&assessment_time, "assessment_time_utc")?;
    let target_seconds = timestamp_seconds(&target_time, "target_time_utc")?;
    let valid_from_seconds = timestamp_seconds(&valid_from, "valid_from_utc")?;
    let valid_until_seconds = timestamp_seconds(&valid_until, "valid_until_utc")?;
    if init_seconds > issue_seconds {
        return Err("model initialization time cannot be after issue time".to_owned());
    }
    if valid_from_seconds > valid_until_seconds {
        return Err("forecast valid interval start must not be after end".to_owned());
    }

    // The declared receipt interval is also bound semantically, not just by hashes.
    if facts.time_coverage_start_utc.as_deref() != Some(valid_from.as_str())
        || facts.time_coverage_end_utc.as_deref() != Some(valid_until.as_str())
    {
        return Err(
            "forecast valid interval does not match the receipt coverage interval".to_owned(),
        );
    }

    let issue_age =
        (issue_seconds <= assessment_seconds).then(|| (assessment_seconds - issue_seconds) as u64);
    let forecast_lead =
        (target_seconds >= init_seconds).then(|| (target_seconds - init_seconds) as u64);

    let mut findings = Vec::new();
    let mut applicable = true;
    if issue_seconds > assessment_seconds {
        applicable = false;
        findings.push("FORECAST_NOT_ISSUED_AT_DECISION_TIME");
    }
    if target_seconds < assessment_seconds {
        applicable = false;
        findings.push("TARGET_PRECEDES_DECISION_TIME");
    }
    if target_seconds < valid_from_seconds || target_seconds > valid_until_seconds {
        applicable = false;
        findings.push("TARGET_OUTSIDE_VALID_INTERVAL");
    }
    if issue_age.is_some_and(|age| age > max_issue_age) {
        applicable = false;
        findings.push("FORECAST_ISSUE_TOO_OLD");
    }
    if forecast_lead.is_none_or(|lead| lead > max_forecast_lead) {
        applicable = false;
        findings.push("FORECAST_LEAD_EXCEEDS_POLICY");
    }
    if quality_status == "unknown" {
        applicable = false;
        findings.push("FORECAST_QUALITY_UNKNOWN");
    }
    if findings.is_empty() {
        findings.push("DECLARED_TIME_POLICY_BOUNDS_SATISFIED");
    }

    Ok(ForecastApplicabilityReport {
        status: if applicable {
            "TEMPORALLY_APPLICABLE_NOT_ADMITTED"
        } else {
            "INDETERMINATE_NOT_ADMITTED"
        },
        profile_id,
        profile_sha256: profile_sha256.to_owned(),
        receipt_id: facts.receipt_id.clone(),
        receipt_sha256: receipt_sha256.to_owned(),
        artifact_sha256: facts.artifact.sha256.clone(),
        product_kind,
        provider_product_id,
        dataset_id,
        dataset_revision,
        model_run_id,
        model_initialization_time_utc: model_initialization_time,
        issued_at_utc: issued_at,
        assessment_time_utc: assessment_time,
        target_time_utc: target_time,
        valid_from_utc: valid_from,
        valid_until_utc: valid_until,
        issue_age_seconds: issue_age,
        forecast_lead_seconds: forecast_lead,
        max_issue_age_seconds: max_issue_age,
        max_forecast_lead_seconds: max_forecast_lead,
        freshness_policy_id,
        findings,
        engineering_admission: "NotAdmitted",
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        berth_availability_inference_authorized: false,
        independent_review_required: true,
        source_commit_binding: source_commit.to_owned(),
        execution_performed: false,
    })
}

pub(super) fn run_cli(
    profile_path: &str,
    receipt_path: &str,
    artifact_override: Option<&str>,
) -> Result<(), String> {
    let profile_metadata = fs::metadata(profile_path)
        .map_err(|error| format!("cannot stat forecast profile {profile_path}: {error}"))?;
    if profile_metadata.len() > super::MAX_RECEIPT_BYTES as u64 {
        return Err("forecast profile exceeds the configured size bound".to_owned());
    }
    let profile_bytes = fs::read(profile_path)
        .map_err(|error| format!("cannot read forecast profile {profile_path}: {error}"))?;
    let profile = super::parse_strict(&profile_bytes)?;

    let receipt_metadata = fs::metadata(receipt_path)
        .map_err(|error| format!("cannot stat receipt {receipt_path}: {error}"))?;
    if receipt_metadata.len() > super::MAX_RECEIPT_BYTES as u64 {
        return Err("receipt exceeds the configured size bound".to_owned());
    }
    let receipt_bytes = fs::read(receipt_path)
        .map_err(|error| format!("cannot read receipt {receipt_path}: {error}"))?;
    let receipt_value = super::parse_strict(&receipt_bytes)?;
    let facts = super::validate_receipt(&receipt_value)?;
    let artifact_path = artifact_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&facts.artifact.relative_path));
    let source_commit =
        env::var("MARITIME_SOURCE_COMMIT").unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());

    // Do not evaluate applicability unless the receipt and artifact bytes verify first.
    super::verify(&receipt_bytes, &artifact_path, receipt_path, &source_commit)?;
    let report = validate_and_evaluate(
        &profile,
        &facts,
        &super::sha256_bytes(&receipt_bytes),
        &super::sha256_bytes(&profile_bytes),
        &source_commit,
    )?;
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize forecast applicability report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn receipt_bytes() -> Vec<u8> {
        include_bytes!("../tests/fixtures/synthetic-water-level-forecast-receipt.json").to_vec()
    }

    fn base_profile() -> Value {
        serde_json::from_slice(include_bytes!(
            "../tests/fixtures/synthetic-water-level-forecast-profile.json"
        ))
        .expect("synthetic forecast profile is valid JSON")
    }

    fn facts_and_binding() -> (Vec<u8>, super::super::ReceiptFacts, String) {
        let receipt = receipt_bytes();
        let value = super::super::parse_strict(&receipt).expect("receipt must parse strictly");
        let facts = super::super::validate_receipt(&value).expect("receipt metadata must validate");
        let hash = super::super::sha256_bytes(&receipt);
        (receipt, facts, hash)
    }

    fn evaluate_value(profile: &Value) -> Result<ForecastApplicabilityReport, String> {
        let (_receipt, facts, receipt_hash) = facts_and_binding();
        let profile_bytes = serde_json::to_vec(profile).expect("profile serializes");
        let strict_profile = super::super::parse_strict(&profile_bytes)?;
        validate_and_evaluate(
            &strict_profile,
            &facts,
            &receipt_hash,
            &super::super::sha256_bytes(&profile_bytes),
            "unit-test",
        )
    }

    fn mutate(profile: &mut Value, path: &str, replacement: Value) {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = profile;
        for part in &parts[..parts.len() - 1] {
            current = current.get_mut(*part).expect("corpus path exists");
        }
        *current
            .get_mut(parts[parts.len() - 1])
            .expect("corpus leaf exists") = replacement;
    }

    fn oracle_field<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
        path.split('.')
            .try_fold(value, |current, key| current.get(key))
    }

    fn oracle_text<'a>(value: &'a Value, path: &str) -> Option<&'a str> {
        oracle_field(value, path)?.as_str()
    }

    fn oracle_uint(value: &Value, path: &str) -> Option<u64> {
        oracle_field(value, path)?.as_u64()
    }

    fn oracle_pinned(value: &str) -> bool {
        let value = value.trim().to_ascii_lowercase();
        !value.is_empty()
            && ![
                "latest",
                "current",
                "head",
                "unknown",
                "unspecified",
                "unversioned",
            ]
            .contains(&value.as_str())
    }

    // Independent calendar parser/oracle: does not call the production timestamp
    // parser or epoch conversion. Years are counted from Gregorian year 1.
    fn oracle_utc_seconds(value: &str) -> Option<i64> {
        let bytes = value.as_bytes();
        if bytes.len() != 20
            || bytes.get(4) != Some(&b'-')
            || bytes.get(7) != Some(&b'-')
            || bytes.get(10) != Some(&b'T')
            || bytes.get(13) != Some(&b':')
            || bytes.get(16) != Some(&b':')
            || bytes.get(19) != Some(&b'Z')
        {
            return None;
        }
        for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
            if !bytes[start..end].iter().all(u8::is_ascii_digit) {
                return None;
            }
        }
        let number = |start: usize, end: usize| value[start..end].parse::<i64>().ok();
        let (year, month, day, hour, minute, second) = (
            number(0, 4)?,
            number(5, 7)?,
            number(8, 10)?,
            number(11, 13)?,
            number(14, 16)?,
            number(17, 19)?,
        );
        if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        let leap =
            |candidate: i64| candidate % 4 == 0 && (candidate % 100 != 0 || candidate % 400 == 0);
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap(year) => 29,
            2 => 28,
            _ => return None,
        };
        if day < 1 || day > days_in_month {
            return None;
        }
        let mut days = 0_i64;
        for candidate in 1..year {
            days += if leap(candidate) { 366 } else { 365 };
        }
        for prior_month in 1..month {
            days += match prior_month {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap(year) => 29,
                2 => 28,
                _ => return None,
            };
        }
        days += day - 1;
        Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
    }

    fn oracle_disposition(profile: &Value) -> &'static str {
        const RECEIPT_SHA256: &str =
            "62d4f04259b3beaf0c462038317a9863e7ebc3e79509a9d44f13c08326778fd2";
        const ARTIFACT_SHA256: &str =
            "45dbbb8cabb009dfaf17b697f24aec156b9baaae0a5dc29dcbce20023c02f714";
        const UNCERTAINTY: &str = concat!(
            "Invented forecast values for temporal applicability tests only; ",
            "no observation or forecast skill claim."
        );

        if oracle_text(profile, "schema_id")
            != Some("luminous.symtropy.maritime-forecast-applicability")
            || oracle_text(profile, "schema_version") != Some("0.1.0")
            || oracle_text(profile, "source_kind") != Some("synthetic_fixture")
            || oracle_text(profile, "receipt_binding.receipt_id")
                != Some("synthetic-water-level-forecast-v0")
            || oracle_text(profile, "receipt_binding.receipt_sha256") != Some(RECEIPT_SHA256)
            || oracle_text(profile, "receipt_binding.artifact_sha256") != Some(ARTIFACT_SHA256)
            || oracle_text(profile, "forecast.product_kind") != Some("water_level_prediction")
            || oracle_text(profile, "forecast.dataset_id")
                != Some("synthetic-water-level-forecast-example-v0")
            || oracle_text(profile, "forecast.dataset_revision") != Some("synthetic-model-run-001")
            || oracle_text(profile, "quality.uncertainty_statement") != Some(UNCERTAINTY)
        {
            return "INVALID";
        }

        for path in [
            "forecast.provider_product_id",
            "forecast.dataset_revision",
            "forecast.model_run_id",
        ] {
            if !oracle_text(profile, path).is_some_and(oracle_pinned) {
                return "INVALID";
            }
        }
        if oracle_uint(profile, "forecast.max_issue_age_seconds").unwrap_or(0) == 0
            || oracle_uint(profile, "forecast.max_forecast_lead_seconds").unwrap_or(0) == 0
        {
            return "INVALID";
        }
        if oracle_text(profile, "quality.status")
            .is_none_or(|status| !["synthetic", "unknown"].contains(&status))
        {
            return "INVALID";
        }
        if oracle_field(profile, "intended_use.scenario_analysis_only").and_then(Value::as_bool)
            != Some(true)
            || oracle_field(profile, "intended_use.navigation_authorized").and_then(Value::as_bool)
                != Some(false)
            || oracle_field(profile, "intended_use.physical_truth_claim_authorized")
                .and_then(Value::as_bool)
                != Some(false)
            || oracle_field(
                profile,
                "intended_use.berth_availability_inference_authorized",
            )
            .and_then(Value::as_bool)
                != Some(false)
            || oracle_field(profile, "intended_use.independent_review_required")
                .and_then(Value::as_bool)
                != Some(true)
        {
            return "INVALID";
        }

        let mut times = Vec::new();
        for path in [
            "forecast.model_initialization_time_utc",
            "forecast.issued_at_utc",
            "forecast.assessment_time_utc",
            "forecast.target_time_utc",
            "forecast.valid_from_utc",
            "forecast.valid_until_utc",
        ] {
            let Some(raw) = oracle_text(profile, path) else {
                return "INVALID";
            };
            let Some(seconds) = oracle_utc_seconds(raw) else {
                return "INVALID";
            };
            times.push(seconds);
        }
        let [init, issue, assessment, target, valid_from, valid_until] =
            <[i64; 6]>::try_from(times).expect("six temporal fields collected");
        if init > issue
            || valid_from > valid_until
            || oracle_text(profile, "forecast.valid_from_utc") != Some("2026-10-15T00:00:00Z")
            || oracle_text(profile, "forecast.valid_until_utc") != Some("2026-10-15T06:00:00Z")
        {
            return "INVALID";
        }
        if issue > assessment
            || target < assessment
            || target < valid_from
            || target > valid_until
            || target < init
            || assessment - issue
                > oracle_uint(profile, "forecast.max_issue_age_seconds").unwrap_or(0) as i64
            || target - init
                > oracle_uint(profile, "forecast.max_forecast_lead_seconds").unwrap_or(0) as i64
            || oracle_text(profile, "quality.status") == Some("unknown")
        {
            return "INDETERMINATE_NOT_ADMITTED";
        }
        "TEMPORALLY_APPLICABLE_NOT_ADMITTED"
    }

    #[test]
    fn frozen_corpus_dispositions_match_independent_expected_results() {
        let corpus: Value = serde_json::from_slice(include_bytes!(
            "../tests/fixtures/forecast-applicability-corpus-v0.1.json"
        ))
        .expect("forecast corpus JSON must parse");
        assert_eq!(
            corpus["corpus_id"],
            "luminous.symtropy.maritime-forecast-applicability-corpus"
        );
        let cases = corpus["cases"].as_array().expect("cases array");
        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let expected = case["expected"].as_str().expect("expected disposition");
            let mut profile = base_profile();
            for change in case["changes"].as_array().expect("changes array") {
                mutate(
                    &mut profile,
                    change["path"].as_str().expect("path"),
                    change["value"].clone(),
                );
            }
            let oracle = oracle_disposition(&profile);
            assert_eq!(oracle, expected, "independent oracle case {id}");
            let actual = evaluate_value(&profile);
            match expected {
                "TEMPORALLY_APPLICABLE_NOT_ADMITTED" => {
                    assert_eq!(actual.expect(id).status, expected, "case {id}");
                }
                "INDETERMINATE_NOT_ADMITTED" => {
                    assert_eq!(actual.expect(id).status, expected, "case {id}");
                }
                "INVALID" => {
                    assert!(actual.is_err(), "case {id} unexpectedly passed: {actual:?}");
                }
                _ => panic!("unknown frozen corpus expectation: {expected}"),
            }
        }
    }

    fn assert_schema_contract(object: &Value, expected_keys: &[&str]) {
        use std::collections::BTreeSet;
        let properties = object["properties"]
            .as_object()
            .expect("schema properties object");
        let actual: BTreeSet<&str> = properties.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = expected_keys.iter().copied().collect();
        assert_eq!(actual, expected, "schema property set drift");
        let required: BTreeSet<&str> = object["required"]
            .as_array()
            .expect("schema required array")
            .iter()
            .map(|item| item.as_str().expect("required field string"))
            .collect();
        assert_eq!(required, expected, "schema required set drift");
    }

    #[test]
    fn schema_property_and_required_sets_match_rust_contracts() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../docs/tech/MARITIME_FORECAST_APPLICABILITY_V0_1.schema.json"
        ))
        .expect("forecast schema must be valid JSON");
        assert_schema_contract(&schema, ROOT_KEYS);
        assert_schema_contract(&schema["properties"]["receipt_binding"], BINDING_KEYS);
        assert_schema_contract(&schema["properties"]["forecast"], FORECAST_KEYS);
        assert_schema_contract(&schema["properties"]["quality"], QUALITY_KEYS);
        assert_schema_contract(&schema["properties"]["intended_use"], USE_KEYS);
        assert_eq!(
            schema["properties"]["schema_id"]["const"],
            "luminous.symtropy.maritime-forecast-applicability"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
    }

    #[test]
    fn unknown_profile_fields_fail_closed() {
        let mut profile = base_profile();
        profile["unreviewed_extension"] = Value::Bool(true);
        assert!(evaluate_value(&profile).is_err());
    }

    // Independent test-only Gregorian conversion. It intentionally does not call
    // the production timestamp parser or epoch conversion.
    fn oracle_epoch_seconds(value: &str) -> i64 {
        assert_eq!(value.len(), 20, "canonical UTC timestamp length");
        assert_eq!(&value[19..20], "Z");
        let year = value[0..4].parse::<i64>().unwrap();
        let month = value[5..7].parse::<i64>().unwrap();
        let day = value[8..10].parse::<i64>().unwrap();
        let hour = value[11..13].parse::<i64>().unwrap();
        let minute = value[14..16].parse::<i64>().unwrap();
        let second = value[17..19].parse::<i64>().unwrap();
        assert!(year >= 1970 && (1..=12).contains(&month) && (1..=31).contains(&day));
        let leap = |y: i64| y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
        let mut days = 0_i64;
        for y in 1970..year {
            days += if leap(y) { 366 } else { 365 };
        }
        for m in 1..month {
            days += match m {
                1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
                4 | 6 | 9 | 11 => 30,
                2 if leap(year) => 29,
                2 => 28,
                _ => unreachable!(),
            };
        }
        days += day - 1;
        days * 86_400 + hour * 3_600 + minute * 60 + second
    }

    #[test]
    fn independent_oracle_recomputes_time_age_and_forecast_lead() {
        let profile = base_profile();
        let forecast = &profile["forecast"];
        let init =
            oracle_epoch_seconds(forecast["model_initialization_time_utc"].as_str().unwrap());
        let issue = oracle_epoch_seconds(forecast["issued_at_utc"].as_str().unwrap());
        let assessment = oracle_epoch_seconds(forecast["assessment_time_utc"].as_str().unwrap());
        let target = oracle_epoch_seconds(forecast["target_time_utc"].as_str().unwrap());
        let valid_from = oracle_epoch_seconds(forecast["valid_from_utc"].as_str().unwrap());
        let valid_until = oracle_epoch_seconds(forecast["valid_until_utc"].as_str().unwrap());
        let max_age = forecast["max_issue_age_seconds"].as_u64().unwrap();
        let max_lead = forecast["max_forecast_lead_seconds"].as_u64().unwrap();
        let oracle_pass = init <= issue
            && issue <= assessment
            && target >= assessment
            && target >= valid_from
            && target <= valid_until
            && (assessment - issue) as u64 <= max_age
            && (target - init) as u64 <= max_lead
            && profile["quality"]["status"] == "synthetic";
        let result = evaluate_value(&profile).expect("base fixture metadata should be valid");
        assert_eq!(
            result.status == "TEMPORALLY_APPLICABLE_NOT_ADMITTED",
            oracle_pass
        );
        assert_eq!(result.issue_age_seconds, Some((assessment - issue) as u64));
        assert_eq!(result.forecast_lead_seconds, Some((target - init) as u64));
        assert_eq!(result.engineering_admission, "NotAdmitted");
        assert!(!result.navigation_authorized);
        assert!(!result.physical_truth_claim_authorized);
        assert!(!result.berth_availability_inference_authorized);
    }
}
