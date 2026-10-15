// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Metadata-only contract for the UNCTADstat quarterly Port Liner Shipping
//! Connectivity Index (PLSCI). A pass verifies declared semantics and receipt
//! binding only; it never admits operational or navigational use.

use serde_json::Value;
use std::collections::BTreeSet;

const ROOT_KEYS: &[&str] = &[
    "schema_id", "schema_version", "profile_id", "source_kind", "receipt_binding",
    "series", "selection", "missingness", "terms", "intended_use",
];
const BINDING_KEYS: &[&str] = &["receipt_id", "receipt_sha256", "artifact_sha256"];
const SERIES_KEYS: &[&str] = &[
    "provider", "table_id", "indicator_id", "spatial_level", "temporal_frequency",
    "unit_semantics", "benchmark_period", "benchmark_value", "methodology_vintage",
    "source_basis", "components",
];
const SELECTION_KEYS: &[&str] = &[
    "geographic_scope", "period_start", "period_end", "source_port_ids",
    "selection_specification",
];
const MISSINGNESS_KEYS: &[&str] = &[
    "availability", "missing_value_policy", "missing_is_zero", "estimate_policy",
    "suppressed_policy", "forward_fill", "interpolation",
];
const TERMS_KEYS: &[&str] = &[
    "review_status", "license_name", "license_uri", "reviewer", "reviewed_at_utc",
    "terms_sha256", "attribution_statement",
];
const INTENDED_USE_KEYS: &[&str] = &[
    "analysis_scope", "scenario_only", "live_operational_feed", "navigation_authorized",
    "physical_truth_claim_authorized", "berth_availability_inference_authorized",
    "port_call_inference_authorized", "custody_or_delivery_inference_authorized",
    "independent_review_required",
];
const COMPONENTS: &[&str] = &[
    "scheduled_ship_calls_per_week",
    "deployed_annual_capacity_teu",
    "regular_liner_services",
    "liner_service_providers",
    "largest_deployed_ship_teu",
    "directly_connected_ports",
];
const VALID: &str = "PLSCI_METADATA_VALID_NOT_ADMITTED";

fn exact_keys(value: &Value, expected: &[&str]) -> Result<(), &'static str> {
    let object = value.as_object().ok_or("INVALID_OBJECT")?;
    if object.keys().any(|key| !expected.contains(&key.as_str())) {
        return Err("UNKNOWN_FIELD");
    }
    if expected.iter().any(|key| !object.contains_key(*key)) {
        return Err("MISSING_REQUIRED_FIELD");
    }
    Ok(())
}

fn member<'a>(value: &'a Value, key: &str) -> Result<&'a Value, &'static str> {
    value.as_object().ok_or("INVALID_OBJECT")?.get(key).ok_or("MISSING_REQUIRED_FIELD")
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    member(value, key)?
        .as_str()
        .filter(|item| !item.trim().is_empty())
        .ok_or("MISSING_REQUIRED_FIELD")
}

fn nullable_text<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, &'static str> {
    match member(value, key)? {
        Value::Null => Ok(None),
        Value::String(item) if !item.trim().is_empty() => Ok(Some(item)),
        _ => Err("INVALID_FIELD_TYPE"),
    }
}

fn bool_is(value: &Value, key: &str, expected: bool) -> Result<(), &'static str> {
    if member(value, key)?.as_bool() == Some(expected) {
        Ok(())
    } else {
        Err("AUTHORITY_OR_POLICY_ESCALATION")
    }
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T'
        || bytes[13] != b':' || bytes[16] != b':' || bytes[19] != b'Z'
    {
        return false;
    }
    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let number = |start: usize, end: usize| value[start..end].parse::<u32>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) = (
        number(0, 4), number(5, 7), number(8, 10),
        number(11, 13), number(14, 16), number(17, 19),
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

fn quarter_ordinal(value: &str) -> Result<u32, &'static str> {
    let (year, quarter) = value.split_once("-Q").ok_or("INVALID_QUARTER")?;
    if year.len() != 4 || !year.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("INVALID_QUARTER");
    }
    let year = year.parse::<u32>().map_err(|_| "INVALID_QUARTER")?;
    let quarter = quarter.parse::<u32>().map_err(|_| "INVALID_QUARTER")?;
    if year == 0 || !(1..=4).contains(&quarter) {
        return Err("INVALID_QUARTER");
    }
    year.checked_mul(4).and_then(|base| base.checked_add(quarter)).ok_or("INVALID_QUARTER")
}

fn validate_profile(profile: &Value) -> Result<(), &'static str> {
    exact_keys(profile, ROOT_KEYS)?;
    if text(profile, "schema_id")? != "luminous.symtropy.maritime-plsci-profile"
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

    let series = member(profile, "series")?;
    exact_keys(series, SERIES_KEYS)?;
    if text(series, "provider")? != "UNCTADstat" || text(series, "table_id")? != "US.PLSCI" {
        return Err("UNSUPPORTED_SERIES");
    }
    if text(series, "indicator_id")? != "index_average_q1_2023_100" {
        return Err("UNSUPPORTED_INDICATOR");
    }
    if text(series, "spatial_level")? != "port" {
        return Err("WRONG_SPATIAL_LEVEL");
    }
    if text(series, "temporal_frequency")? != "quarterly" {
        return Err("WRONG_TEMPORAL_FREQUENCY");
    }
    if text(series, "unit_semantics")? != "index_points" {
        return Err("WRONG_UNIT_SEMANTICS");
    }
    if text(series, "benchmark_period")? != "2023-Q1"
        || member(series, "benchmark_value")?.as_f64() != Some(100.0)
    {
        return Err("WRONG_INDEX_BASE");
    }
    if text(series, "methodology_vintage")? != "rescaled-2024-six-component" {
        return Err("UNSUPPORTED_METHODOLOGY_VINTAGE");
    }
    if text(series, "source_basis")? != "published-container-liner-shipping-schedules" {
        return Err("UNSUPPORTED_SOURCE_BASIS");
    }
    let components = member(series, "components")?.as_array().ok_or("INVALID_FIELD_TYPE")?;
    let component_set: BTreeSet<&str> = components
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|name| !name.trim().is_empty())
                .ok_or("INVALID_FIELD_TYPE")
        })
        .collect::<Result<_, _>>()?;
    let expected_components: BTreeSet<&str> = COMPONENTS.iter().copied().collect();
    if component_set.len() != components.len() {
        return Err("DUPLICATE_COMPONENT");
    }
    if component_set != expected_components {
        return Err("COMPONENT_SET_MISMATCH");
    }

    let selection = member(profile, "selection")?;
    exact_keys(selection, SELECTION_KEYS)?;
    text(selection, "geographic_scope")?;
    text(selection, "selection_specification")?;
    let start = quarter_ordinal(text(selection, "period_start")?)?;
    let end = quarter_ordinal(text(selection, "period_end")?)?;
    if start > end {
        return Err("PERIOD_RANGE_INVALID");
    }
    let port_ids = member(selection, "source_port_ids")?.as_array().ok_or("INVALID_FIELD_TYPE")?;
    if port_ids.is_empty() {
        return Err("EMPTY_PORT_SELECTION");
    }
    let mut unique_ports = BTreeSet::new();
    for item in port_ids {
        let id = item.as_str().filter(|id| !id.trim().is_empty()).ok_or("INVALID_FIELD_TYPE")?;
        if !unique_ports.insert(id) {
            return Err("DUPLICATE_SOURCE_PORT_ID");
        }
    }

    let missingness = member(profile, "missingness")?;
    exact_keys(missingness, MISSINGNESS_KEYS)?;
    if text(missingness, "availability")? != "not_all_ports_for_every_quarter" {
        return Err("UNSUPPORTED_COVERAGE_SEMANTICS");
    }
    if text(missingness, "missing_value_policy")? != "preserve_missing_as_unknown" {
        return Err("INVALID_MISSINGNESS_POLICY");
    }
    if member(missingness, "missing_is_zero")?.as_bool() != Some(false) {
        return Err("MISSING_MUST_NOT_BE_ZERO");
    }
    if text(missingness, "estimate_policy")? != "preserve_explicit_source_flag"
        || text(missingness, "suppressed_policy")? != "preserve_suppressed_state"
        || text(missingness, "forward_fill")? != "forbidden"
        || text(missingness, "interpolation")? != "forbidden"
    {
        return Err("INVALID_MISSINGNESS_POLICY");
    }

    let terms = member(profile, "terms")?;
    exact_keys(terms, TERMS_KEYS)?;
    let review_status = text(terms, "review_status")?;
    let license_name = nullable_text(terms, "license_name")?;
    let license_uri = nullable_text(terms, "license_uri")?;
    let reviewer = nullable_text(terms, "reviewer")?;
    let reviewed_at = nullable_text(terms, "reviewed_at_utc")?;
    let terms_digest = nullable_text(terms, "terms_sha256")?;
    let attribution = text(terms, "attribution_statement")?;
    match source_kind {
        "synthetic_fixture" => {
            if review_status != "not_applicable_synthetic"
                || license_name != Some("not_applicable_synthetic_fixture")
                || license_uri.is_some()
                || reviewer.is_some()
                || reviewed_at.is_some()
                || terms_digest.is_some()
                || !attribution.to_ascii_lowercase().contains("synthetic")
            {
                return Err("SYNTHETIC_TERMS_MISMATCH");
            }
        }
        "public_dataset" => {
            if review_status != "reviewed_permitted" {
                return Err("TERMS_NOT_REVIEWED");
            }
            if license_name != Some("CC BY 3.0 IGO")
                || license_uri != Some("https://creativecommons.org/licenses/by/3.0/igo/")
                || reviewer.is_none()
                || !reviewed_at.is_some_and(utc_timestamp)
                || !terms_digest.is_some_and(sha256)
                || !attribution.contains("United Nations Trade and Development Data Hub")
            {
                return Err("PUBLIC_TERMS_INCOMPLETE");
            }
        }
        _ => return Err("UNSUPPORTED_SOURCE_KIND"),
    }

    let usage = member(profile, "intended_use")?;
    exact_keys(usage, INTENDED_USE_KEYS)?;
    if text(usage, "analysis_scope")? != "comparative_network_analysis_and_synthetic_scenarios" {
        return Err("UNSUPPORTED_INTENDED_USE");
    }
    bool_is(usage, "scenario_only", true)?;
    for key in [
        "live_operational_feed",
        "navigation_authorized",
        "physical_truth_claim_authorized",
        "berth_availability_inference_authorized",
        "port_call_inference_authorized",
        "custody_or_delivery_inference_authorized",
    ] {
        bool_is(usage, key, false)?;
    }
    bool_is(usage, "independent_review_required", true)?;
    Ok(())
}

fn verify_receipt_semantics(
    profile: &Value,
    receipt: &Value,
    receipt_bytes: &[u8],
    facts: &super::ReceiptFacts,
) -> Result<(), &'static str> {
    let source_kind = text(profile, "source_kind")?;
    let binding = member(profile, "receipt_binding")?;
    if text(binding, "receipt_id")? != facts.receipt_id
        || text(binding, "receipt_sha256")? != super::sha256_bytes(receipt_bytes)
        || text(binding, "artifact_sha256")? != facts.artifact.sha256
    {
        return Err("RECEIPT_BINDING_MISMATCH");
    }
    if source_kind != facts.source_kind
        || facts.dataset_id != "US.PLSCI"
        || facts.product != "other"
        || facts.domain != "other"
        || facts.units.len() != 1
        || facts.units[0] != "dimensionless"
    {
        return Err("RECEIPT_SEMANTICS_MISMATCH");
    }
    let profile_terms = member(profile, "terms")?;
    let receipt_terms = member(receipt, "terms")?;
    for key in [
        "review_status",
        "license_name",
        "license_uri",
        "reviewer",
        "reviewed_at_utc",
        "terms_sha256",
    ] {
        if member(profile_terms, key)? != member(receipt_terms, key)? {
            return Err("RECEIPT_TERMS_MISMATCH");
        }
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct ProfileReport {
    status: &'static str,
    profile_id: String,
    table_id: String,
    indicator_id: String,
    spatial_level: String,
    temporal_frequency: String,
    methodology_vintage: String,
    receipt_id: String,
    receipt_sha256: String,
    artifact_sha256: String,
    artifact_byte_length: u64,
    source_commit_binding: String,
    profile_metadata_checks_passed: bool,
    exact_receipt_and_artifact_bytes_verified: bool,
    evidence_admission: &'static str,
    source_authenticity_established: bool,
    observed_values_semantically_validated: bool,
    port_identity_established: bool,
    navigation_authorized: bool,
    operational_authority: bool,
    claim_ceiling: Vec<&'static str>,
}

pub(crate) fn run_cli(
    profile_path: &str,
    receipt_path: &str,
    artifact_override: Option<&str>,
) -> Result<(), String> {
    use std::fs;
    use std::path::PathBuf;

    let profile_bytes = fs::read(profile_path)
        .map_err(|error| format!("cannot read PLSCI profile {profile_path}: {error}"))?;
    if profile_bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("PLSCI profile exceeds the bounded file-size limit".to_owned());
    }
    let _strict_profile = super::parse_strict(&profile_bytes)?;
    let profile: Value = serde_json::from_slice(&profile_bytes)
        .map_err(|error| format!("invalid PLSCI profile JSON: {error}"))?;
    validate_profile(&profile).map_err(|code| format!("PLSCI_PROFILE_REJECTED:{code}"))?;

    let receipt_bytes = fs::read(receipt_path)
        .map_err(|error| format!("cannot read PLSCI receipt {receipt_path}: {error}"))?;
    if receipt_bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("receipt exceeds the bounded file-size limit".to_owned());
    }
    let receipt_strict = super::parse_strict(&receipt_bytes)?;
    let facts = super::validate_receipt(&receipt_strict)?;
    let receipt_value: Value = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("invalid receipt JSON: {error}"))?;
    verify_receipt_semantics(&profile, &receipt_value, &receipt_bytes, &facts)
        .map_err(str::to_owned)?;

    let artifact_path = artifact_override
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(&facts.artifact.relative_path));
    let source_commit = std::env::var("MARITIME_SOURCE_COMMIT")
        .unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let receipt_report =
        super::verify(&receipt_bytes, &artifact_path, receipt_path, &source_commit)?;

    let series = member(&profile, "series").map_err(str::to_owned)?;
    let report = ProfileReport {
        status: "PLSCI_PROFILE_AND_BOUND_BYTES_MATCH_NOT_ADMITTED",
        profile_id: text(&profile, "profile_id").map_err(str::to_owned)?.to_owned(),
        table_id: text(series, "table_id").map_err(str::to_owned)?.to_owned(),
        indicator_id: text(series, "indicator_id").map_err(str::to_owned)?.to_owned(),
        spatial_level: text(series, "spatial_level").map_err(str::to_owned)?.to_owned(),
        temporal_frequency: text(series, "temporal_frequency").map_err(str::to_owned)?.to_owned(),
        methodology_vintage: text(series, "methodology_vintage").map_err(str::to_owned)?.to_owned(),
        receipt_id: receipt_report.receipt_id,
        receipt_sha256: receipt_report.receipt_sha256,
        artifact_sha256: receipt_report.artifact_sha256,
        artifact_byte_length: receipt_report.artifact_byte_length,
        source_commit_binding: source_commit,
        profile_metadata_checks_passed: true,
        exact_receipt_and_artifact_bytes_verified: receipt_report.byte_integrity_matches,
        evidence_admission: "NotAdmitted",
        source_authenticity_established: false,
        observed_values_semantically_validated: false,
        port_identity_established: false,
        navigation_authorized: false,
        operational_authority: false,
        claim_ceiling: vec![
            "Pinned PLSCI metadata semantics and exact source-receipt/artifact byte binding only.",
            "This metadata guard does not parse the export or validate individual index values, coverage, port identity, or publisher authenticity.",
            "Missing, suppressed, or estimated values must remain distinct; no silent zero-fill, forward-fill, or interpolation is permitted.",
            "No live vessel position, individual port call, available berth, route safety, navigation or operational authority.",
        ],
    };
    let output = serde_json::to_string(&report)
        .map_err(|error| format!("cannot serialize PLSCI profile report: {error}"))?;
    println!("{output}");
    Ok(())
}

#[cfg(test)]
fn mutate_path(value: &mut Value, path: &str, mutation: &Value) -> Result<(), &'static str> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Err("INVALID_CORPUS_MUTATION");
    }
    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        current = current.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?
            .get_mut(*part).ok_or("INVALID_CORPUS_MUTATION")?;
    }
    let leaf = parts[parts.len() - 1];
    let object = current.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?;
    match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
        "set" => {
            if !object.contains_key(leaf) { return Err("INVALID_CORPUS_MUTATION"); }
            object.insert(
                leaf.to_owned(),
                mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?,
            );
        }
        "insert" => {
            object.insert(leaf.to_owned(), mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?);
        }
        "remove" => {
            if object.remove(leaf).is_none() { return Err("INVALID_CORPUS_MUTATION"); }
        }
        _ => return Err("INVALID_CORPUS_MUTATION"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{mutate_path, validate_profile, VALID};
    use serde_json::Value;

    #[test]
    fn frozen_plsci_profile_corpus_matches_expected_dispositions() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/plsci-profile-corpus-v0.1.json"
        ))
            .expect("frozen PLSCI profile corpus must be valid JSON");
        assert_eq!(corpus["corpus_id"], "luminous.symtropy.maritime-plsci-profile-corpus");
        assert_eq!(corpus["corpus_version"], "0.1.0");
        let bases = corpus["base_profiles"].as_object().expect("base profile map");
        let cases = corpus["cases"].as_array().expect("profile cases");
        assert_eq!(cases.len(), 20, "frozen PLSCI case count is a contract");
        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let base = case["base"].as_str().expect("base profile");
            let mut candidate = bases.get(base).expect("declared base profile").clone();
            for mutation in case["mutations"].as_array().expect("mutation list") {
                mutate_path(
                    &mut candidate,
                    mutation["path"].as_str().expect("mutation path"),
                    mutation,
                )
                    .unwrap_or_else(|error| panic!("{id}: invalid corpus mutation: {error}"));
            }
            let actual = match validate_profile(&candidate) {
                Ok(()) => VALID,
                Err(code) => code,
            };
            assert_eq!(case["expected_code"].as_str(), Some(actual), "{id}");
        }
    }

    #[test]
    fn profile_schema_is_valid_json_and_pins_the_same_identity() {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../docs/tech/MARITIME_PLSCI_PROFILE_V0_1.schema.json"
        ))
            .expect("PLSCI profile schema must be valid JSON");
        assert_eq!(
            schema["properties"]["schema_id"]["const"],
            "luminous.symtropy.maritime-plsci-profile"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
        assert_eq!(schema["properties"]["series"]["properties"]["spatial_level"]["const"], "port");
        assert_eq!(
            schema["properties"]["series"]["properties"]["temporal_frequency"]["const"],
            "quarterly"
        );
        assert_eq!(
            schema["properties"]["series"]["properties"]["benchmark_period"]["const"],
            "2023-Q1"
        );
    }

    #[test]
    fn source_port_ids_are_not_treated_as_locodes_or_terminal_ids() {
        let profile: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/synthetic-plsci-profile.json"
        ))
            .expect("synthetic profile");
        assert!(validate_profile(&profile).is_ok());
        assert_eq!(profile["selection"]["source_port_ids"][0], "SYNTH-001");
        assert!(profile["selection"].get("candidate_locode").is_none());
    }

    #[test]
    fn receipt_binding_uses_exact_receipt_and_artifact_digests() {
        let profile_bytes = include_bytes!("../tests/fixtures/synthetic-plsci-profile.json");
        let receipt_bytes = include_bytes!("../tests/fixtures/synthetic-plsci-receipt.json");
        let profile: Value = serde_json::from_slice(profile_bytes).expect("profile JSON");
        let strict_receipt = super::super::parse_strict(receipt_bytes).expect("strict receipt");
        let facts = super::super::validate_receipt(&strict_receipt).expect("receipt contract");
        let receipt: Value = serde_json::from_slice(receipt_bytes).expect("receipt JSON");
        assert!(super::verify_receipt_semantics(&profile, &receipt, receipt_bytes, &facts).is_ok());

        let mut changed = profile.clone();
        changed["receipt_binding"]["receipt_sha256"] =
            Value::String(
                "0000000000000000000000000000000000000000000000000000000000000000".to_owned(),
            );
        assert_eq!(
            super::verify_receipt_semantics(&changed, &receipt, receipt_bytes, &facts),
            Err("RECEIPT_BINDING_MISMATCH")
        );
    }

    #[test]
    fn profile_is_only_a_metadata_claim() {
        let profile: Value = serde_json::from_str(include_str!("../tests/fixtures/synthetic-plsci-profile.json"))
            .expect("synthetic profile");
        assert!(validate_profile(&profile).is_ok());
        assert!(!profile["intended_use"]["navigation_authorized"].as_bool().expect("boolean"));
        assert!(!profile["intended_use"]["physical_truth_claim_authorized"].as_bool().expect("boolean"));
    }
}
