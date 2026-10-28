// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Metadata-only IHO S-100 profile contract checks.
//!
//! These checks do not parse S-100 payloads or qualify a dataset for navigation.
//! A structurally valid profile is always reported as not admitted.

use serde_json::Value;

const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "profile_id",
    "source_kind",
    "receipt_binding",
    "product",
    "coverage",
    "temporal",
    "quality",
    "terms",
    "transform",
    "intended_use",
];
const PRODUCT_KEYS: &[&str] = &[
    "code",
    "s100_edition",
    "specification_edition",
    "producer_code",
    "dataset_id",
    "revision",
    "native_encoding",
];
const COVERAGE_KEYS: &[&str] = &[
    "coverage_id",
    "horizontal_crs",
    "vertical_reference",
    "measurement_units",
];
const TEMPORAL_KEYS: &[&str] = &[
    "data_epoch_start_utc",
    "data_epoch_end_utc",
    "valid_from_utc",
    "valid_until_utc",
    "assessment_time_utc",
    "freshness_policy_id",
];
const QUALITY_KEYS: &[&str] = &[
    "uncertainty_state",
    "uncertainty_ref",
    "quality_metadata_ref",
    "statement",
];
const TERMS_KEYS: &[&str] = &["review_status", "terms_sha256", "reviewed_at_utc"];
const TRANSFORM_KEYS: &[&str] = &[
    "kind",
    "transform_receipt_id",
    "input_crs",
    "output_crs",
    "input_vertical_reference",
    "output_vertical_reference",
];
const USE_KEYS: &[&str] = &[
    "navigation_authorized",
    "physical_truth_claim_authorized",
    "independent_review_required",
];
const BINDING_KEYS: &[&str] = &["receipt_id", "receipt_sha256", "artifact_sha256"];
#[cfg(test)]
const VALID: &str = "METADATA_CONTRACT_VALID_NOT_ADMITTED";

fn exact_keys(value: &Value, keys: &[&str]) -> Result<(), &'static str> {
    let object = value.as_object().ok_or("INVALID_OBJECT")?;
    if object.keys().any(|key| !keys.contains(&key.as_str())) {
        return Err("UNKNOWN_FIELD");
    }
    if keys.iter().any(|key| !object.contains_key(*key)) {
        return Err("MISSING_REQUIRED_FIELD");
    }
    Ok(())
}

fn member<'a>(value: &'a Value, key: &str) -> Result<&'a Value, &'static str> {
    value
        .as_object()
        .ok_or("INVALID_OBJECT")?
        .get(key)
        .ok_or("MISSING_REQUIRED_FIELD")
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    member(value, key)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or("MISSING_REQUIRED_FIELD")
}

fn nullable_text<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, &'static str> {
    match member(value, key)? {
        Value::Null => Ok(None),
        Value::String(value) if !value.trim().is_empty() => Ok(Some(value)),
        _ => Err("INVALID_FIELD_TYPE"),
    }
}

fn pinned_revision(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    !value.trim().is_empty()
        && !["latest", "current", "head", "unknown", "unversioned"].contains(&lower.as_str())
}

fn numeric_edition(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }
    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let number = |a: usize, b: usize| value[a..b].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4),
        number(5, 7),
        number(8, 10),
        number(11, 13),
        number(14, 16),
        number(17, 19),
    ) else {
        return false;
    };
    if !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days).contains(&day)
}

fn timestamp<'a>(value: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    let value = text(value, key)?;
    if !valid_utc_timestamp(value) {
        return Err("INVALID_UTC_TIMESTAMP");
    }
    Ok(value)
}

fn optional_timestamp<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, &'static str> {
    let value = nullable_text(value, key)?;
    if value.is_some_and(|item| !valid_utc_timestamp(item)) {
        return Err("INVALID_UTC_TIMESTAMP");
    }
    Ok(value)
}

/// Validates only the versioned metadata envelope. It never admits operational use.
fn validate_profile(profile: &Value) -> Result<(), &'static str> {
    exact_keys(profile, ROOT_KEYS)?;
    if text(profile, "schema_id")? != "luminous.symtropy.maritime-s100-profile"
        || text(profile, "schema_version")? != "0.1.0"
    {
        return Err("UNSUPPORTED_PROFILE_SCHEMA");
    }
    text(profile, "profile_id")?;
    let source_kind = text(profile, "source_kind")?;
    if !["synthetic_fixture", "public_dataset"].contains(&source_kind) {
        return Err("UNSUPPORTED_SOURCE_KIND");
    }

    let binding = member(profile, "receipt_binding")?;
    exact_keys(binding, BINDING_KEYS)?;
    text(binding, "receipt_id")?;
    if !sha256(text(binding, "receipt_sha256")?) || !sha256(text(binding, "artifact_sha256")?) {
        return Err("INVALID_RECEIPT_BINDING");
    }

    let product = member(profile, "product")?;
    exact_keys(product, PRODUCT_KEYS)?;
    let code = text(product, "code")?;
    if !["S-102", "S-104", "S-111"].contains(&code) {
        return Err("UNSUPPORTED_PRODUCT");
    }
    let s100_edition = text(product, "s100_edition")?;
    let specification_edition = text(product, "specification_edition")?;
    if !pinned_revision(s100_edition) || !pinned_revision(specification_edition) {
        return Err("MUTABLE_PRODUCT_EDITION");
    }
    if !numeric_edition(s100_edition) || !numeric_edition(specification_edition) {
        return Err("INVALID_PRODUCT_EDITION");
    }
    text(product, "producer_code")?;
    text(product, "dataset_id")?;
    if !pinned_revision(text(product, "revision")?) {
        return Err("MUTABLE_REVISION");
    }
    text(product, "native_encoding")?;

    let coverage = member(profile, "coverage")?;
    exact_keys(coverage, COVERAGE_KEYS)?;
    text(coverage, "coverage_id")?;
    if coverage
        .get("horizontal_crs")
        .and_then(Value::as_str)
        .is_none_or(|value| value.trim().is_empty())
    {
        return Err("MISSING_CRS");
    }
    let vertical_reference = nullable_text(coverage, "vertical_reference")?;
    if ["S-102", "S-104"].contains(&code) && vertical_reference.is_none() {
        return Err("MISSING_VERTICAL_REFERENCE");
    }
    let units = member(coverage, "measurement_units")?
        .as_array()
        .ok_or("INVALID_FIELD_TYPE")?;
    if units.is_empty() {
        return Err("MISSING_UNITS");
    }
    let mut seen_units = std::collections::BTreeSet::new();
    for unit in units {
        let unit = unit.as_str().ok_or("INVALID_FIELD_TYPE")?;
        let allowed = match code {
            "S-102" => ["m", "cm", "mm", "ft", "in"].contains(&unit),
            "S-104" => ["m", "cm", "mm", "ft", "in"].contains(&unit),
            "S-111" => ["m/s", "cm/s", "kn", "knot", "knots"].contains(&unit),
            _ => false,
        };
        if !allowed {
            return Err("DOMAIN_INCOMPATIBLE_UNITS");
        }
        if !seen_units.insert(unit) {
            return Err("DUPLICATE_UNIT");
        }
    }

    let temporal = member(profile, "temporal")?;
    exact_keys(temporal, TEMPORAL_KEYS)?;
    let data_start = timestamp(temporal, "data_epoch_start_utc")?;
    let data_end = timestamp(temporal, "data_epoch_end_utc")?;
    let assessment = timestamp(temporal, "assessment_time_utc")?;
    if data_start > data_end {
        return Err("INVALID_TIME_INTERVAL");
    }
    text(temporal, "freshness_policy_id")?;
    let valid_from = optional_timestamp(temporal, "valid_from_utc")?;
    let valid_until = optional_timestamp(temporal, "valid_until_utc")?;
    if ["S-104", "S-111"].contains(&code) && (valid_from.is_none() || valid_until.is_none()) {
        return Err("MISSING_VALIDITY_WINDOW");
    }
    if let (Some(start), Some(end)) = (valid_from, valid_until) {
        if start > end {
            return Err("INVALID_TIME_INTERVAL");
        }
        if assessment < start {
            return Err("NOT_YET_VALID");
        }
        if assessment > end {
            return Err("STALE_OR_EXPIRED");
        }
    }

    let quality = member(profile, "quality")?;
    exact_keys(quality, QUALITY_KEYS)?;
    let uncertainty_state = text(quality, "uncertainty_state")?;
    if !["provided", "not_provided", "not_applicable_synthetic"].contains(&uncertainty_state) {
        return Err("INVALID_QUALITY_STATE");
    }
    if uncertainty_state == "not_provided"
        || (source_kind == "public_dataset" && uncertainty_state == "not_applicable_synthetic")
    {
        return Err("QUALITY_METADATA_UNKNOWN");
    }
    if code == "S-102"
        && (uncertainty_state != "provided" || nullable_text(quality, "uncertainty_ref")?.is_none())
    {
        return Err("MISSING_UNCERTAINTY_REFERENCE");
    }
    let _ = nullable_text(quality, "uncertainty_ref")?;
    let _ = nullable_text(quality, "quality_metadata_ref")?;
    text(quality, "statement")?;

    let terms = member(profile, "terms")?;
    exact_keys(terms, TERMS_KEYS)?;
    let review_status = text(terms, "review_status")?;
    if source_kind == "public_dataset" {
        if review_status != "reviewed_permitted" {
            return Err("TERMS_NOT_REVIEWED");
        }
        if !sha256(nullable_text(terms, "terms_sha256")?.ok_or("TERMS_NOT_REVIEWED")?) {
            return Err("TERMS_NOT_REVIEWED");
        }
        let reviewed_at =
            optional_timestamp(terms, "reviewed_at_utc")?.ok_or("TERMS_NOT_REVIEWED")?;
        let _ = reviewed_at;
    } else if review_status != "not_applicable_synthetic" {
        return Err("SYNTHETIC_TERMS_STATE_MISMATCH");
    }

    let transform = member(profile, "transform")?;
    exact_keys(transform, TRANSFORM_KEYS)?;
    let transform_kind = text(transform, "kind")?;
    let receipt = nullable_text(transform, "transform_receipt_id")?;
    let input_crs = nullable_text(transform, "input_crs")?;
    let output_crs = nullable_text(transform, "output_crs")?;
    let input_vertical = nullable_text(transform, "input_vertical_reference")?;
    let output_vertical = nullable_text(transform, "output_vertical_reference")?;
    match transform_kind {
        "source_native" => {
            if input_crs != output_crs && (input_crs.is_some() || output_crs.is_some()) {
                return Err("SILENT_TRANSFORM");
            }
            if input_vertical != output_vertical
                && (input_vertical.is_some() || output_vertical.is_some())
            {
                return Err("SILENT_TRANSFORM");
            }
            if receipt.is_some()
                || input_crs.is_some()
                || output_crs.is_some()
                || input_vertical.is_some()
                || output_vertical.is_some()
            {
                return Err("SOURCE_NATIVE_TRANSFORM_METADATA_FORBIDDEN");
            }
        }
        "derived" => {
            if receipt.is_none() {
                return Err("TRANSFORM_RECEIPT_REQUIRED");
            }
            if input_crs.is_none() || output_crs.is_none() {
                return Err("TRANSFORM_DETAILS_MISSING");
            }
            if ["S-102", "S-104"].contains(&code)
                && (input_vertical.is_none() || output_vertical.is_none())
            {
                return Err("TRANSFORM_VERTICAL_REFERENCE_MISSING");
            }
        }
        _ => return Err("INVALID_TRANSFORM_KIND"),
    }

    let intended_use = member(profile, "intended_use")?;
    exact_keys(intended_use, USE_KEYS)?;
    let navigation_authorized = member(intended_use, "navigation_authorized")?
        .as_bool()
        .ok_or("INVALID_FIELD_TYPE")?;
    let physical_truth_authorized = member(intended_use, "physical_truth_claim_authorized")?
        .as_bool()
        .ok_or("INVALID_FIELD_TYPE")?;
    let independent_review_required = member(intended_use, "independent_review_required")?
        .as_bool()
        .ok_or("INVALID_FIELD_TYPE")?;
    if navigation_authorized || physical_truth_authorized || !independent_review_required {
        return Err("AUTHORITY_ESCALATION_ATTEMPT");
    }

    Ok(())
}

fn verify_receipt_semantics(
    profile: &Value,
    facts: &super::ReceiptFacts,
) -> Result<(), &'static str> {
    let source_kind = text(profile, "source_kind")?;
    if source_kind != facts.source_kind {
        return Err("RECEIPT_SOURCE_KIND_MISMATCH");
    }

    let product = member(profile, "product")?;
    let product_code = text(product, "code")?;
    if text(product, "dataset_id")? != facts.dataset_id
        || text(product, "revision")? != facts.dataset_revision
    {
        return Err("RECEIPT_DATASET_IDENTITY_MISMATCH");
    }

    let expected = match product_code {
        "S-102" => ("hydrography", "hydrography"),
        "S-104" if facts.product == "water_level" => ("water_level", "water_level"),
        "S-104" if facts.product == "water_level_prediction" => {
            ("water_level_prediction", "water_level_prediction")
        }
        "S-111" => ("currents", "currents"),
        _ => return Err("RECEIPT_PRODUCT_MISMATCH"),
    };
    if facts.product != expected.0 || facts.domain != expected.1 {
        return Err("RECEIPT_PRODUCT_MISMATCH");
    }

    let coverage = member(profile, "coverage")?;
    if text(coverage, "horizontal_crs")?
        != facts.coordinate_reference_system.as_deref().unwrap_or("")
    {
        return Err("RECEIPT_SPATIAL_REFERENCE_MISMATCH");
    }
    if nullable_text(coverage, "vertical_reference")? != facts.vertical_datum.as_deref() {
        return Err("RECEIPT_SPATIAL_REFERENCE_MISMATCH");
    }
    let profile_units = member(coverage, "measurement_units")?
        .as_array()
        .ok_or("INVALID_FIELD_TYPE")?;
    if profile_units.len() != facts.units.len() {
        return Err("RECEIPT_UNITS_MISMATCH");
    }
    for unit in profile_units {
        let unit = unit.as_str().ok_or("INVALID_FIELD_TYPE")?;
        if !facts.units.iter().any(|source_unit| source_unit == unit) {
            return Err("RECEIPT_UNITS_MISMATCH");
        }
    }

    let quality = member(profile, "quality")?;
    if text(quality, "statement")? != facts.uncertainty_statement {
        return Err("RECEIPT_QUALITY_STATEMENT_MISMATCH");
    }

    let temporal = member(profile, "temporal")?;
    if facts.time_coverage_start_utc.as_deref() != Some(text(temporal, "data_epoch_start_utc")?)
        || facts.time_coverage_end_utc.as_deref() != Some(text(temporal, "data_epoch_end_utc")?)
    {
        return Err("RECEIPT_TIME_COVERAGE_MISMATCH");
    }
    if let Some(observed_at) = facts.survey_or_observation_time_utc.as_deref() {
        if observed_at < text(temporal, "data_epoch_start_utc")?
            || observed_at > text(temporal, "data_epoch_end_utc")?
        {
            return Err("RECEIPT_TIME_COVERAGE_MISMATCH");
        }
    } else if source_kind == "public_dataset" {
        return Err("RECEIPT_OBSERVATION_TIME_MISSING");
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct S100ProfileReport {
    status: &'static str,
    profile_id: String,
    product_code: String,
    source_kind: String,
    receipt_id: String,
    receipt_sha256: String,
    artifact_sha256: String,
    source_commit_binding: String,
    profile_metadata_checks_passed: bool,
    exact_receipt_and_artifact_bytes_verified: bool,
    engineering_admission: &'static str,
    navigation_authorized: bool,
    physical_truth_claim_authorized: bool,
    claim_ceiling: Vec<&'static str>,
}

/// CLI entry point. Binding fields are cross-checked against the receipt and its actual artifact.
pub(crate) fn run_cli(
    profile_path: &str,
    receipt_path: &str,
    artifact_override: Option<&str>,
) -> Result<(), String> {
    use std::fs;
    use std::path::PathBuf;

    let profile_bytes = fs::read(profile_path)
        .map_err(|error| format!("cannot read S-100 profile {profile_path}: {error}"))?;
    if profile_bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("S-100 profile exceeds the bounded file-size limit".to_owned());
    }
    // Run the repository's duplicate-key rejecting parser before serde_json maps object fields.
    let _strict_profile = super::parse_strict(&profile_bytes)?;
    let profile: Value = serde_json::from_slice(&profile_bytes)
        .map_err(|error| format!("invalid S-100 profile JSON: {error}"))?;
    validate_profile(&profile).map_err(|code| format!("S100_PROFILE_REJECTED:{code}"))?;

    let receipt_bytes = fs::read(receipt_path)
        .map_err(|error| format!("cannot read receipt {receipt_path}: {error}"))?;
    if receipt_bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("receipt exceeds the bounded file-size limit".to_owned());
    }
    let receipt_value = super::parse_strict(&receipt_bytes)?;
    let facts = super::validate_receipt(&receipt_value)?;
    verify_receipt_semantics(&profile, &facts).map_err(str::to_owned)?;

    let binding = member(&profile, "receipt_binding").map_err(str::to_owned)?;
    let expected_receipt_id = text(binding, "receipt_id").map_err(str::to_owned)?;
    let expected_receipt_sha = text(binding, "receipt_sha256").map_err(str::to_owned)?;
    let expected_artifact_sha = text(binding, "artifact_sha256").map_err(str::to_owned)?;
    if expected_receipt_id != facts.receipt_id
        || expected_receipt_sha != super::sha256_bytes(&receipt_bytes)
        || expected_artifact_sha != facts.artifact.sha256
    {
        return Err("RECEIPT_BINDING_MISMATCH".to_owned());
    }

    let artifact_path = artifact_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&facts.artifact.relative_path));
    let source_commit = std::env::var("MARITIME_SOURCE_COMMIT")
        .unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let receipt_report =
        super::verify(&receipt_bytes, &artifact_path, receipt_path, &source_commit)?;

    let report = S100ProfileReport {
        status: "S100_PROFILE_AND_BOUND_BYTES_MATCH_NOT_ADMITTED",
        profile_id: text(&profile, "profile_id")
            .map_err(str::to_owned)?
            .to_owned(),
        product_code: text(member(&profile, "product").map_err(str::to_owned)?, "code")
            .map_err(str::to_owned)?
            .to_owned(),
        source_kind: text(&profile, "source_kind")
            .map_err(str::to_owned)?
            .to_owned(),
        receipt_id: receipt_report.receipt_id,
        receipt_sha256: receipt_report.receipt_sha256,
        artifact_sha256: receipt_report.artifact_sha256,
        source_commit_binding: source_commit,
        profile_metadata_checks_passed: true,
        exact_receipt_and_artifact_bytes_verified: receipt_report.byte_integrity_matches,
        engineering_admission: "NotAdmitted",
        navigation_authorized: false,
        physical_truth_claim_authorized: false,
        claim_ceiling: vec![
            "Metadata profile contract and binding to the exact receipt and artifact bytes only.",
            "The tool does not decode or validate an IHO S-100 payload or independently confirm publisher authenticity.",
            "No navigational suitability, under-keel clearance, physical truth, engineering safety, or operational authority.",
        ],
    };
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize S-100 profile report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
fn mutate_path(value: &mut Value, path: &str, mutation: &Value) -> Result<(), &'static str> {
    use serde_json::Map;

    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err("INVALID_CORPUS_MUTATION");
    }
    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        current = current
            .as_object_mut()
            .ok_or("INVALID_CORPUS_MUTATION")?
            .get_mut(*part)
            .ok_or("INVALID_CORPUS_MUTATION")?;
    }
    let object: &mut Map<String, Value> =
        current.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?;
    let leaf = parts[parts.len() - 1];
    match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
        "set" => {
            if !object.contains_key(leaf) {
                return Err("INVALID_CORPUS_MUTATION");
            }
            object.insert(
                leaf.to_owned(),
                mutation
                    .get("value")
                    .cloned()
                    .ok_or("INVALID_CORPUS_MUTATION")?,
            );
        }
        "insert" => {
            object.insert(
                leaf.to_owned(),
                mutation
                    .get("value")
                    .cloned()
                    .ok_or("INVALID_CORPUS_MUTATION")?,
            );
        }
        "remove" => {
            if object.remove(leaf).is_none() {
                return Err("INVALID_CORPUS_MUTATION");
            }
        }
        _ => return Err("INVALID_CORPUS_MUTATION"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{VALID, mutate_path, validate_profile, verify_receipt_semantics};
    use serde_json::Value;

    #[test]
    fn frozen_positive_and_adversarial_corpus_matches_expected_dispositions() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/s100-profile-corpus-v0.1.json"
        ))
        .expect("frozen S-100 profile corpus must be valid JSON");
        assert_eq!(
            corpus["corpus_id"],
            "luminous.symtropy.maritime-s100-profile-corpus"
        );
        assert_eq!(corpus["corpus_version"], "0.1.0");
        let profiles = corpus["base_profiles"]
            .as_object()
            .expect("base profiles object");
        let cases = corpus["cases"].as_array().expect("case array");
        assert_eq!(
            cases.len(),
            23,
            "the frozen corpus count is part of the test contract"
        );
        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let base_name = case["base"].as_str().expect("base profile");
            let mut profile = profiles
                .get(base_name)
                .expect("declared base profile")
                .clone();
            for mutation in case["mutations"].as_array().expect("mutation list") {
                mutate_path(
                    &mut profile,
                    mutation["path"].as_str().expect("mutation path"),
                    mutation,
                )
                .unwrap_or_else(|error| panic!("{id}: invalid frozen mutation: {error}"));
            }
            let actual = match validate_profile(&profile) {
                Ok(()) => VALID,
                Err(code) => code,
            };
            assert_eq!(
                case["expected_code"].as_str(),
                Some(actual),
                "unexpected disposition for {id}"
            );
            if actual == VALID {
                assert_eq!(
                    profile["intended_use"]["navigation_authorized"], false,
                    "{id}"
                );
                assert_eq!(
                    profile["intended_use"]["physical_truth_claim_authorized"], false,
                    "{id}"
                );
            }
        }
    }

    #[test]
    fn synthetic_s104_profile_matches_the_exact_receipt_semantics() {
        let profile: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/synthetic-s104-profile.json"
        ))
        .expect("S-104 synthetic profile fixture");
        let receipt_bytes = include_bytes!("../tests/fixtures/synthetic-water-level-receipt.json");
        let receipt_root =
            super::super::parse_strict(receipt_bytes).expect("strict receipt parsing");
        let facts =
            super::super::validate_receipt(&receipt_root).expect("receipt metadata validation");
        assert_eq!(verify_receipt_semantics(&profile, &facts), Ok(()));
    }

    #[test]
    fn profile_cannot_relabel_water_level_receipt_as_currents() {
        let mut profile: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/synthetic-s104-profile.json"
        ))
        .expect("S-104 synthetic profile fixture");
        let receipt_bytes = include_bytes!("../tests/fixtures/synthetic-water-level-receipt.json");
        let receipt_root =
            super::super::parse_strict(receipt_bytes).expect("strict receipt parsing");
        let facts =
            super::super::validate_receipt(&receipt_root).expect("receipt metadata validation");
        profile["product"]["code"] = Value::String("S-111".to_owned());
        assert_eq!(
            verify_receipt_semantics(&profile, &facts),
            Err("RECEIPT_PRODUCT_MISMATCH")
        );
    }

    #[test]
    fn schema_is_valid_json_and_uses_the_same_profile_identity() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../docs/tech/MARITIME_S100_PROFILE_V0_1.schema.json"
        ))
        .expect("S-100 profile schema must be valid JSON");
        assert_eq!(
            schema["properties"]["schema_id"]["const"],
            "luminous.symtropy.maritime-s100-profile"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
        assert_eq!(
            schema["properties"]["product"]["properties"]["s100_edition"]["pattern"],
            r"^[0-9]+\.[0-9]+\.[0-9]+$"
        );
        assert_eq!(
            schema["properties"]["product"]["properties"]["specification_edition"]["pattern"],
            r"^[0-9]+\.[0-9]+\.[0-9]+$"
        );
    }

    #[test]
    fn valid_profile_never_becomes_admitted() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/s100-profile-corpus-v0.1.json"
        ))
        .unwrap();
        for name in ["s102", "s104", "s111"] {
            assert_eq!(
                validate_profile(&corpus["base_profiles"][name]),
                Ok(()),
                "{name}"
            );
        }
    }
}
