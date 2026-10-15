// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Versioned port-identity crosswalk metadata guard.
//!
//! This validates the structure and internal consistency of a declared crosswalk.
//! It does not authenticate source snapshots, resolve a real port/terminal, or
//! grant operational authority. Valid output is always NOT ADMITTED.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

const ROOT_KEYS: &[&str] = &[
    "schema_id", "schema_version", "crosswalk_id", "fixture_kind",
    "source_snapshot", "locode_registry", "mapping_policy", "records",
];
const SOURCE_KEYS: &[&str] = &[
    "source_system", "series_or_table_id", "source_revision", "artifact_sha256",
    "retrieved_at_utc", "source_uri", "terms_review_status",
];
const REGISTRY_KEYS: &[&str] = &[
    "release", "release_kind", "artifact_sha256", "terms_review_status",
];
const POLICY_KEYS: &[&str] = &[
    "policy_id", "version", "name_match_can_resolve",
    "port_statistics_can_target_terminal", "unresolved_is_zero",
];
const RECORD_KEYS: &[&str] = &[
    "source_entity_id", "source_level", "candidate_locode", "locode_function",
    "locode_status", "target_feature_id", "target_level", "relationship",
    "mapping_state", "match_method", "evidence_refs",
];
const VALID_STATUS: &str = "CROSSWALK_METADATA_VALID_NOT_ADMITTED";

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
    value
        .as_object()
        .ok_or("INVALID_OBJECT")?
        .get(key)
        .ok_or("MISSING_REQUIRED_FIELD")
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, &'static str> {
    member(value, key)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or("MISSING_REQUIRED_FIELD")
}

fn nullable_text<'a>(value: &'a Value, key: &str) -> Result<Option<&'a str>, &'static str> {
    match member(value, key)? {
        Value::Null => Ok(None),
        Value::String(text) if !text.trim().is_empty() => Ok(Some(text)),
        _ => Err("INVALID_FIELD_TYPE"),
    }
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn pinned_revision(value: &str) -> bool {
    let value = value.trim().to_ascii_lowercase();
    !value.is_empty()
        && !["latest", "current", "head", "unknown", "unversioned"].contains(&value.as_str())
}

fn numeric_version(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit())
        })
}

fn locode_release(value: &str) -> bool {
    let Some((year, cycle)) = value.split_once('-') else {
        return false;
    };
    year.len() == 4
        && year.bytes().all(|byte| byte.is_ascii_digit())
        && ["1", "2"].contains(&cycle)
}

fn utc_timestamp(value: &str) -> bool {
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
    let number = |start, end| value[start..end].parse::<u32>().ok();
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

fn valid_locode(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 5
        && bytes[0..2].iter().all(u8::is_ascii_uppercase)
        && bytes[2..5].iter().all(|byte| {
            byte.is_ascii_uppercase() || (b'2'..=b'9').contains(byte)
        })
}

fn valid_function(value: &str) -> bool {
    value.len() == 8
        && value
            .bytes()
            .all(|byte| (b'0'..=b'8').contains(&byte) || byte == b'A' || byte == b'-')
}

fn valid_status(value: &str) -> bool {
    [
        "AA", "AC", "AF", "AI", "AM", "AQ", "AS", "RL", "RN",
        "RQ", "RR", "QQ", "UR", "XX",
    ]
    .contains(&value)
}

fn validate_crosswalk(crosswalk: &Value) -> Result<usize, &'static str> {
    exact_keys(crosswalk, ROOT_KEYS)?;
    if text(crosswalk, "schema_id")? != "luminous.symtropy.maritime-port-crosswalk"
        || text(crosswalk, "schema_version")? != "0.1.0"
    {
        return Err("UNSUPPORTED_CROSSWALK_SCHEMA");
    }
    text(crosswalk, "crosswalk_id")?;
    let fixture_kind = text(crosswalk, "fixture_kind")?;
    if !["synthetic_fixture", "public_capture"].contains(&fixture_kind) {
        return Err("UNSUPPORTED_FIXTURE_KIND");
    }

    let source = member(crosswalk, "source_snapshot")?;
    exact_keys(source, SOURCE_KEYS)?;
    text(source, "source_system")?;
    text(source, "series_or_table_id")?;
    let source_revision = text(source, "source_revision")?;
    if !pinned_revision(source_revision) {
        return Err("UNPINNED_SOURCE_REVISION");
    }
    if !sha256(text(source, "artifact_sha256")?) {
        return Err("INVALID_SNAPSHOT_DIGEST");
    }
    let source_uri = nullable_text(source, "source_uri")?;
    let retrieved_at = nullable_text(source, "retrieved_at_utc")?;
    let source_terms = text(source, "terms_review_status")?;
    if ![
        "reviewed_permitted",
        "review_required",
        "not_applicable_synthetic",
    ]
    .contains(&source_terms)
    {
        return Err("INVALID_TERMS_REVIEW_STATE");
    }
    if let Some(timestamp) = retrieved_at {
        if !utc_timestamp(timestamp) {
            return Err("INVALID_UTC_TIMESTAMP");
        }
    }

    let registry = member(crosswalk, "locode_registry")?;
    exact_keys(registry, REGISTRY_KEYS)?;
    let release = text(registry, "release")?;
    if !locode_release(release) {
        return Err("UNPINNED_LOCODE_RELEASE");
    }
    if text(registry, "release_kind")? != "production" {
        return Err("NON_PRODUCTION_LOCODE_RELEASE");
    }
    if !sha256(text(registry, "artifact_sha256")?) {
        return Err("INVALID_SNAPSHOT_DIGEST");
    }
    let registry_terms = text(registry, "terms_review_status")?;
    if ![
        "reviewed_permitted",
        "review_required",
        "not_applicable_synthetic",
    ]
    .contains(&registry_terms)
    {
        return Err("INVALID_TERMS_REVIEW_STATE");
    }

    if fixture_kind == "synthetic_fixture" {
        if source_terms != "not_applicable_synthetic"
            || registry_terms != "not_applicable_synthetic"
            || source_uri.is_some()
            || retrieved_at.is_some()
        {
            return Err("SYNTHETIC_SOURCE_METADATA_MISMATCH");
        }
    } else {
        if source_terms != "reviewed_permitted" || registry_terms != "reviewed_permitted" {
            return Err("TERMS_NOT_REVIEWED");
        }
        if !source_uri.is_some_and(|uri| uri.starts_with("https://"))
            || !retrieved_at.is_some_and(utc_timestamp)
        {
            return Err("PUBLIC_SOURCE_SNAPSHOT_INCOMPLETE");
        }
    }

    let policy = member(crosswalk, "mapping_policy")?;
    exact_keys(policy, POLICY_KEYS)?;
    text(policy, "policy_id")?;
    if !numeric_version(text(policy, "version")?) {
        return Err("INVALID_POLICY_VERSION");
    }
    if member(policy, "name_match_can_resolve")?.as_bool() != Some(false) {
        return Err("POLICY_FORBIDS_NAME_MATCH_RESOLUTION");
    }
    if member(policy, "port_statistics_can_target_terminal")?.as_bool() != Some(false) {
        return Err("POLICY_FORBIDS_TERMINAL_GRANULARITY_PROMOTION");
    }
    if member(policy, "unresolved_is_zero")?.as_bool() != Some(false) {
        return Err("POLICY_FORBIDS_UNRESOLVED_ZERO");
    }

    let records = member(crosswalk, "records")?
        .as_array()
        .ok_or("INVALID_FIELD_TYPE")?;
    if records.is_empty() {
        return Err("EMPTY_CROSSWALK");
    }
    let mut source_candidate_pairs = BTreeSet::new();
    let mut candidate_counts: BTreeMap<String, usize> = BTreeMap::new();
    for record in records {
        exact_keys(record, RECORD_KEYS)?;
        let source_id = text(record, "source_entity_id")?;
        let source_level = text(record, "source_level")?;
        if !["country", "port", "terminal", "unknown"].contains(&source_level) {
            return Err("INVALID_SOURCE_LEVEL");
        }

        let candidate = nullable_text(record, "candidate_locode")?;
        if !source_candidate_pairs.insert((source_id.to_owned(), candidate.map(str::to_owned))) {
            return Err("DUPLICATE_SOURCE_ENTITY_CANDIDATE");
        }
        *candidate_counts.entry(source_id.to_owned()).or_insert(0) += 1;
        let function = nullable_text(record, "locode_function")?;
        let status = nullable_text(record, "locode_status")?;
        match candidate {
            Some(code) => {
                if !valid_locode(code) {
                    return Err("INVALID_LOCODE");
                }
                let function = function.ok_or("INCOMPLETE_LOCODE_METADATA")?;
                let status = status.ok_or("INCOMPLETE_LOCODE_METADATA")?;
                if !valid_function(function) {
                    return Err("INVALID_LOCODE_FUNCTION");
                }
                if !valid_status(status) {
                    return Err("INVALID_LOCODE_STATUS");
                }
            }
            None => {
                if function.is_some() || status.is_some() {
                    return Err("INCOMPLETE_LOCODE_METADATA");
                }
            }
        }

        let target = nullable_text(record, "target_feature_id")?;
        let target_level = text(record, "target_level")?;
        if !["country", "port_area", "terminal", "unknown"].contains(&target_level) {
            return Err("INVALID_TARGET_LEVEL");
        }
        let relationship = text(record, "relationship")?;
        if !["same_entity", "parent_area", "none"].contains(&relationship) {
            return Err("INVALID_RELATIONSHIP");
        }
        let state = text(record, "mapping_state")?;
        if !["resolved", "ambiguous", "unresolved"].contains(&state) {
            return Err("INVALID_MAPPING_STATE");
        }
        let method = text(record, "match_method")?;
        if ![
            "official_identifier",
            "manual_review",
            "name_similarity_only",
            "none",
        ]
        .contains(&method)
        {
            return Err("INVALID_MATCH_METHOD");
        }
        let evidence = member(record, "evidence_refs")?
            .as_array()
            .ok_or("INVALID_FIELD_TYPE")?;
        let mut evidence_set = BTreeSet::new();
        for item in evidence {
            let item = item.as_str().filter(|item| !item.trim().is_empty()).ok_or("INVALID_FIELD_TYPE")?;
            if !evidence_set.insert(item) {
                return Err("DUPLICATE_EVIDENCE_REFERENCE");
            }
        }

        match state {
            "resolved" => {
                if status == Some("XX") {
                    return Err("LOCode_STATUS_NOT_RESOLVABLE");
                }
                if candidate.is_none()
                    || target.is_none()
                    || relationship == "none"
                    || target_level == "unknown"
                    || evidence.is_empty()
                    || method == "none"
                {
                    return Err("RESOLVED_MAPPING_INCOMPLETE");
                }
                if method == "name_similarity_only" {
                    return Err("NAME_MATCH_CANNOT_RESOLVE");
                }
                if (source_level == "port" && target_level == "terminal")
                    || (source_level == "country" && target_level != "country")
                {
                    return Err("GRANULARITY_MISMATCH");
                }
                let same_level = source_level == target_level
                    || (source_level == "port" && target_level == "port_area");
                if relationship == "same_entity" && !same_level {
                    return Err("RELATIONSHIP_LEVEL_MISMATCH");
                }
                if relationship == "parent_area"
                    && !(source_level == "terminal" && target_level == "port_area")
                {
                    return Err("RELATIONSHIP_LEVEL_MISMATCH");
                }
            }
            "ambiguous" => {
                if candidate.is_none() {
                    return Err("AMBIGUOUS_CANDIDATE_MISSING");
                }
                if target.is_some() || relationship != "none" || evidence.is_empty() || method == "none" {
                    return Err("AMBIGUOUS_MAPPING_INCOMPLETE");
                }
            }
            "unresolved" => {
                if target.is_some() || relationship != "none" {
                    return Err("UNRESOLVED_MAPPING_HAS_TARGET");
                }
                if method != "none" {
                    return Err("UNRESOLVED_MAPPING_HAS_MATCH_METHOD");
                }
            }
            _ => unreachable!("mapping state was checked above"),
        }
    }

    for record in records {
        let source_id = text(record, "source_entity_id")?;
        if candidate_counts.get(source_id).copied().unwrap_or(0) > 1 {
            if text(record, "mapping_state")? != "ambiguous"
                || nullable_text(record, "target_feature_id")?.is_some()
                || text(record, "relationship")? != "none"
            {
                return Err("MULTIPLE_CANDIDATES_MUST_BE_AMBIGUOUS");
            }
        }
    }

    Ok(records.len())
}

#[derive(serde::Serialize)]
struct CrosswalkReport {
    status: &'static str,
    crosswalk_id: String,
    crosswalk_sha256: String,
    source_series_or_table_id: String,
    source_revision: String,
    locode_release: String,
    record_count: usize,
    structural_checks_passed: bool,
    evidence_admission: &'static str,
    source_authenticity_established: bool,
    port_identity_established: bool,
    operational_authority: bool,
    claim_ceiling: Vec<&'static str>,
}

pub(crate) fn run_cli(path: &str) -> Result<(), String> {
    use std::fs;

    let bytes = fs::read(path).map_err(|error| format!("cannot read crosswalk {path}: {error}"))?;
    if bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("crosswalk exceeds the bounded file-size limit".to_owned());
    }
    let value = super::parse_strict(&bytes)?;
    let count = validate_crosswalk(&value).map_err(str::to_owned)?;
    let source = member(&value, "source_snapshot").map_err(str::to_owned)?;
    let registry = member(&value, "locode_registry").map_err(str::to_owned)?;
    let report = CrosswalkReport {
        status: VALID,
        crosswalk_id: text(&value, "crosswalk_id").map_err(str::to_owned)?.to_owned(),
        crosswalk_sha256: super::sha256_bytes(&bytes),
        source_series_or_table_id: text(source, "series_or_table_id").map_err(str::to_owned)?.to_owned(),
        source_revision: text(source, "source_revision").map_err(str::to_owned)?.to_owned(),
        locode_release: text(registry, "release").map_err(str::to_owned)?.to_owned(),
        record_count: count,
        structural_checks_passed: true,
        evidence_admission: "NotAdmitted",
        source_authenticity_established: false,
        port_identity_established: false,
        operational_authority: false,
        claim_ceiling: vec![
            "Crosswalk structure, declared release pins and internal mapping-policy consistency only.",
            "Snapshot digests are checked for syntax here, not against downloaded source artifacts.",
            "No real port identity, terminal association, source authenticity, inventory, custody, delivery, port availability, safety or navigation authority is established.",
        ],
    };
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|error| format!("cannot serialize report: {error}"))?
    );
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
        current = if let Some(array) = current.as_array_mut() {
            let index = part.parse::<usize>().map_err(|_| "INVALID_CORPUS_MUTATION")?;
            array.get_mut(index).ok_or("INVALID_CORPUS_MUTATION")?
        } else {
            current
                .as_object_mut()
                .ok_or("INVALID_CORPUS_MUTATION")?
                .get_mut(*part)
                .ok_or("INVALID_CORPUS_MUTATION")?
        };
    }
    let leaf = parts[parts.len() - 1];
    if let Some(array) = current.as_array_mut() {
        let index = leaf.parse::<usize>().map_err(|_| "INVALID_CORPUS_MUTATION")?;
        let target = array.get_mut(index).ok_or("INVALID_CORPUS_MUTATION")?;
        match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
            "set" => *target = mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?,
            _ => return Err("INVALID_CORPUS_MUTATION"),
        }
        return Ok(());
    }
    let object: &mut Map<String, Value> = current.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?;
    match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
        "set" => {
            if !object.contains_key(leaf) {
                return Err("INVALID_CORPUS_MUTATION");
            }
            object.insert(leaf.to_owned(), mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?);
        }
        "insert" => {
            object.insert(leaf.to_owned(), mutation.get("value").cloned().ok_or("INVALID_CORPUS_MUTATION")?);
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
    use super::{mutate_path, validate_crosswalk, VALID};
    use serde_json::Value;

    #[test]
    fn frozen_port_identity_corpus_matches_expected_dispositions() {
        let corpus: Value = serde_json::from_str(include_str!("../tests/fixtures/port-crosswalk-corpus-v0.1.json"))
            .expect("frozen port crosswalk corpus must be valid JSON");
        assert_eq!(corpus["corpus_id"], "luminous.symtropy.maritime-port-crosswalk-corpus");
        assert_eq!(corpus["corpus_version"], "0.1.0");
        let bases = corpus["base_crosswalks"].as_object().expect("base crosswalk map");
        let cases = corpus["cases"].as_array().expect("crosswalk case array");
        assert_eq!(cases.len(), 23, "frozen port crosswalk count is a contract");
        for case in cases {
            let id = case["id"].as_str().expect("case id");
            let base = case["base"].as_str().expect("base crosswalk");
            let mut candidate = bases.get(base).expect("declared base").clone();
            for mutation in case["mutations"].as_array().expect("mutation array") {
                mutate_path(
                    &mut candidate,
                    mutation["path"].as_str().expect("path"),
                    mutation,
                )
                .unwrap_or_else(|error| panic!("{id}: invalid corpus mutation: {error}"));
            }
            let actual = match validate_crosswalk(&candidate) {
                Ok(_) => VALID,
                Err(code) => code,
            };
            assert_eq!(case["expected_code"].as_str(), Some(actual), "{id}");
        }
    }

    #[test]
    fn schema_is_valid_json_and_has_same_contract_identity() {
        let schema: Value = serde_json::from_str(include_str!("../../../../docs/tech/MARITIME_PORT_CROSSWALK_V0_1.schema.json"))
            .expect("crosswalk schema must be valid JSON");
        assert_eq!(
            schema["properties"]["schema_id"]["const"],
            "luminous.symtropy.maritime-port-crosswalk"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
    }

    #[test]
    fn synthetic_valid_mapping_is_still_not_admitted() {
        let corpus: Value = serde_json::from_str(include_str!("../tests/fixtures/port-crosswalk-corpus-v0.1.json")).unwrap();
        let valid = &corpus["base_crosswalks"]["valid_resolved"];
        assert!(validate_crosswalk(valid).is_ok());
        assert_eq!(valid["fixture_kind"], "synthetic_fixture");
    }
}
