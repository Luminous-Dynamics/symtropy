// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Versioned port-identity crosswalk metadata guard.
//!
//! This validates the structure and internal consistency of a declared crosswalk.
//! It does not authenticate source snapshots, resolve a real port/terminal, or
//! grant operational authority. Valid output is always NOT ADMITTED.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "crosswalk_id",
    "fixture_kind",
    "source_snapshot",
    "locode_registry",
    "mapping_policy",
    "records",
];
const SOURCE_KEYS: &[&str] = &[
    "source_system",
    "series_or_table_id",
    "source_revision",
    "artifact_sha256",
    "retrieved_at_utc",
    "source_uri",
    "terms_review_status",
];
const REGISTRY_KEYS: &[&str] = &[
    "release",
    "release_kind",
    "artifact_sha256",
    "terms_review_status",
];
const POLICY_KEYS: &[&str] = &[
    "policy_id",
    "version",
    "name_match_can_resolve",
    "port_statistics_can_target_terminal",
    "unresolved_is_zero",
];
const RECORD_KEYS: &[&str] = &[
    "source_entity_id",
    "source_level",
    "candidate_locode",
    "locode_function",
    "locode_status",
    "target_feature_id",
    "target_level",
    "relationship",
    "mapping_state",
    "match_method",
    "evidence_refs",
];
const VALID_STATUS: &str = "CROSSWALK_METADATA_VALID_NOT_ADMITTED";
const VALID: &str = VALID_STATUS;

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
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn locode_release(value: &str) -> bool {
    let Some((year, cycle)) = value.split_once('-') else {
        return false;
    };
    year.len() == 4 && year.bytes().all(|byte| byte.is_ascii_digit()) && ["1", "2"].contains(&cycle)
}

fn utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes.last() != Some(&b'Z')
    {
        return false;
    }
    for (start, end) in [(0, 4), (5, 7), (8, 10), (11, 13), (14, 16), (17, 19)] {
        if !bytes[start..end].iter().all(u8::is_ascii_digit) {
            return false;
        }
    }
    let fractional_seconds_valid = match bytes.get(19) {
        Some(b'Z') => bytes.len() == 20,
        Some(b'.') => {
            bytes.len() >= 22 && bytes[20..bytes.len() - 1].iter().all(u8::is_ascii_digit)
        }
        _ => false,
    };
    if !fractional_seconds_valid {
        return false;
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
        && bytes[2..5]
            .iter()
            .all(|byte| byte.is_ascii_uppercase() || (b'2'..=b'9').contains(byte))
}

fn valid_function(value: &str) -> bool {
    value.len() == 8
        && value
            .bytes()
            .all(|byte| (b'0'..=b'8').contains(&byte) || byte == b'A' || byte == b'-')
}

fn valid_status(value: &str) -> bool {
    [
        "AA", "AC", "AF", "AI", "AM", "AQ", "AS", "RL", "RN", "RQ", "RR", "QQ", "UR", "XX",
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
    if let Some(timestamp) = retrieved_at
        && !utc_timestamp(timestamp)
    {
        return Err("INVALID_UTC_TIMESTAMP");
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
            let item = item
                .as_str()
                .filter(|item| !item.trim().is_empty())
                .ok_or("INVALID_FIELD_TYPE")?;
            if !evidence_set.insert(item) {
                return Err("DUPLICATE_EVIDENCE_REFERENCE");
            }
        }

        match state {
            "resolved" => {
                if status == Some("XX") {
                    return Err("UNKNOWN_LOCODE_STATUS_NOT_RESOLVABLE");
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
                if target.is_some()
                    || relationship != "none"
                    || evidence.is_empty()
                    || method == "none"
                {
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
        if candidate_counts.get(source_id).copied().unwrap_or(0) > 1
            && (text(record, "mapping_state")? != "ambiguous"
                || nullable_text(record, "target_feature_id")?.is_some()
                || text(record, "relationship")? != "none")
        {
            return Err("MULTIPLE_CANDIDATES_MUST_BE_AMBIGUOUS");
        }
    }

    Ok(records.len())
}

fn verify_snapshot_digest(path: &str, expected: &str, label: &str) -> Result<(), String> {
    let (_, observed) = super::sha256_file(std::path::Path::new(path))?;
    if observed != expected {
        return Err(format!(
            "{label} SHA-256 mismatch: crosswalk declares {expected}, observed {observed}"
        ));
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct CrosswalkReport {
    status: &'static str,
    crosswalk_id: String,
    crosswalk_sha256: String,
    source_commit_binding: String,
    fixture_kind: String,
    source_series_or_table_id: String,
    source_revision: String,
    source_snapshot_sha256: String,
    locode_release: String,
    locode_registry_sha256: String,
    source_snapshot_path_checked: Option<String>,
    locode_registry_path_checked: Option<String>,
    declared_snapshot_hashes_match_local_bytes: bool,
    record_count: usize,
    resolved_count: usize,
    ambiguous_count: usize,
    unresolved_count: usize,
    structural_checks_passed: bool,
    evidence_admission: &'static str,
    source_authenticity_established: bool,
    port_identity_established: bool,
    operational_authority: bool,
    claim_ceiling: Vec<&'static str>,
}

fn build_crosswalk_report(
    value: &Value,
    bytes: &[u8],
    record_count: usize,
    source_snapshot_path_checked: Option<String>,
    locode_registry_path_checked: Option<String>,
    snapshots_match: bool,
    source_commit: String,
) -> Result<CrosswalkReport, String> {
    let source_checked = source_snapshot_path_checked.is_some();
    let registry_checked = locode_registry_path_checked.is_some();
    if source_checked != registry_checked || snapshots_match != (source_checked && registry_checked)
    {
        return Err(
            "snapshot verification flag and checked-path evidence are inconsistent".to_owned(),
        );
    }

    let source = member(value, "source_snapshot").map_err(str::to_owned)?;
    let registry = member(value, "locode_registry").map_err(str::to_owned)?;
    let mut resolved_count = 0usize;
    let mut ambiguous_count = 0usize;
    let mut unresolved_count = 0usize;
    let records = member(value, "records")
        .map_err(str::to_owned)?
        .as_array()
        .ok_or_else(|| "crosswalk records must be an array".to_owned())?;
    for record in records {
        match text(record, "mapping_state").map_err(str::to_owned)? {
            "resolved" => resolved_count += 1,
            "ambiguous" => ambiguous_count += 1,
            "unresolved" => unresolved_count += 1,
            _ => return Err("unexpected validated mapping state".to_owned()),
        }
    }

    Ok(CrosswalkReport {
        status: VALID,
        crosswalk_id: text(value, "crosswalk_id")
            .map_err(str::to_owned)?
            .to_owned(),
        crosswalk_sha256: super::sha256_bytes(bytes),
        source_commit_binding: source_commit,
        fixture_kind: text(value, "fixture_kind")
            .map_err(str::to_owned)?
            .to_owned(),
        source_series_or_table_id: text(source, "series_or_table_id")
            .map_err(str::to_owned)?
            .to_owned(),
        source_revision: text(source, "source_revision")
            .map_err(str::to_owned)?
            .to_owned(),
        source_snapshot_sha256: text(source, "artifact_sha256")
            .map_err(str::to_owned)?
            .to_owned(),
        locode_release: text(registry, "release").map_err(str::to_owned)?.to_owned(),
        locode_registry_sha256: text(registry, "artifact_sha256")
            .map_err(str::to_owned)?
            .to_owned(),
        source_snapshot_path_checked,
        locode_registry_path_checked,
        declared_snapshot_hashes_match_local_bytes: snapshots_match,
        record_count,
        resolved_count,
        ambiguous_count,
        unresolved_count,
        structural_checks_passed: true,
        evidence_admission: "NotAdmitted",
        source_authenticity_established: false,
        port_identity_established: false,
        operational_authority: false,
        claim_ceiling: vec![
            if snapshots_match {
                "Crosswalk structure, declared release pins, local snapshot-byte equality, and internal mapping-policy consistency only."
            } else {
                "Crosswalk structure, declared release pins and internal mapping-policy consistency only."
            },
            if snapshots_match {
                "Declared digests match the supplied local bytes; this does not establish publisher authenticity, licence permission, semantic correctness or freshness."
            } else {
                "Snapshot digests are checked for syntax here, not against local source artifacts."
            },
            "No real port identity, terminal association, source authenticity, inventory, custody, delivery, port availability, safety or navigation authority is established.",
        ],
    })
}

pub(crate) fn run_cli(path: &str) -> Result<(), String> {
    run_cli_inner(path, None)
}

pub(crate) fn run_cli_with_snapshots(
    crosswalk_path: &str,
    source_snapshot_path: &str,
    locode_registry_path: &str,
) -> Result<(), String> {
    run_cli_inner(
        crosswalk_path,
        Some((source_snapshot_path, locode_registry_path)),
    )
}

fn run_cli_inner(path: &str, snapshot_paths: Option<(&str, &str)>) -> Result<(), String> {
    use std::fs;

    let bytes = fs::read(path).map_err(|error| format!("cannot read crosswalk {path}: {error}"))?;
    if bytes.len() > super::MAX_RECEIPT_BYTES {
        return Err("crosswalk exceeds the bounded file-size limit".to_owned());
    }
    let _strict = super::parse_strict(&bytes)?;
    let value: Value =
        serde_json::from_slice(&bytes).map_err(|e| format!("crosswalk JSON invalid: {e}"))?;
    let count = validate_crosswalk(&value).map_err(str::to_owned)?;
    let source = member(&value, "source_snapshot").map_err(str::to_owned)?;
    let registry = member(&value, "locode_registry").map_err(str::to_owned)?;
    let (source_snapshot_path_checked, locode_registry_path_checked, snapshots_match) =
        if let Some((source_path, registry_path)) = snapshot_paths {
            verify_snapshot_digest(
                source_path,
                text(source, "artifact_sha256").map_err(str::to_owned)?,
                "source snapshot",
            )?;
            verify_snapshot_digest(
                registry_path,
                text(registry, "artifact_sha256").map_err(str::to_owned)?,
                "UN/LOCODE registry",
            )?;
            (
                Some(source_path.to_owned()),
                Some(registry_path.to_owned()),
                true,
            )
        } else {
            (None, None, false)
        };
    let source_commit = std::env::var("MARITIME_SOURCE_COMMIT")
        .unwrap_or_else(|_| "UNBOUND_LOCAL_SOURCE".to_owned());
    let report = build_crosswalk_report(
        &value,
        &bytes,
        count,
        source_snapshot_path_checked,
        locode_registry_path_checked,
        snapshots_match,
        source_commit,
    )?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))?
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
        if current.is_array() {
            let index = part
                .parse::<usize>()
                .map_err(|_| "INVALID_CORPUS_MUTATION")?;
            current = current
                .as_array_mut()
                .unwrap()
                .get_mut(index)
                .ok_or("INVALID_CORPUS_MUTATION")?;
        } else {
            current = current
                .as_object_mut()
                .ok_or("INVALID_CORPUS_MUTATION")?
                .get_mut(*part)
                .ok_or("INVALID_CORPUS_MUTATION")?;
        }
    }
    let leaf = parts[parts.len() - 1];
    if let Some(array) = current.as_array_mut() {
        let index = leaf
            .parse::<usize>()
            .map_err(|_| "INVALID_CORPUS_MUTATION")?;
        let target = array.get_mut(index).ok_or("INVALID_CORPUS_MUTATION")?;
        match mutation["op"].as_str().ok_or("INVALID_CORPUS_MUTATION")? {
            "set" => {
                *target = mutation
                    .get("value")
                    .cloned()
                    .ok_or("INVALID_CORPUS_MUTATION")?
            }
            _ => return Err("INVALID_CORPUS_MUTATION"),
        }
        return Ok(());
    }
    let object = current.as_object_mut().ok_or("INVALID_CORPUS_MUTATION")?;
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
    use super::{
        VALID, member, mutate_path, text, utc_timestamp, validate_crosswalk, verify_snapshot_digest,
    };
    use serde_json::Value;

    #[test]
    fn frozen_port_identity_corpus_matches_expected_dispositions() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/port-crosswalk-corpus-v0.1.json"
        ))
        .expect("frozen port crosswalk corpus must be valid JSON");
        assert_eq!(
            corpus["corpus_id"],
            "luminous.symtropy.maritime-port-crosswalk-corpus"
        );
        assert_eq!(corpus["corpus_version"], "0.1.0");
        let bases = corpus["base_crosswalks"]
            .as_object()
            .expect("base crosswalk map");
        let cases = corpus["cases"].as_array().expect("crosswalk case array");
        assert_eq!(cases.len(), 25, "frozen port crosswalk count is a contract");
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
        let schema: Value = serde_json::from_str(include_str!(
            "../../../../docs/tech/MARITIME_PORT_CROSSWALK_V0_1.schema.json"
        ))
        .expect("crosswalk schema must be valid JSON");
        assert_eq!(
            schema["properties"]["schema_id"]["const"],
            "luminous.symtropy.maritime-port-crosswalk"
        );
        assert_eq!(schema["properties"]["schema_version"]["const"], "0.1.0");
    }

    #[test]
    fn synthetic_valid_mapping_is_still_not_admitted() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../tests/fixtures/port-crosswalk-corpus-v0.1.json"
        ))
        .unwrap();
        let valid = &corpus["base_crosswalks"]["valid_resolved"];
        assert!(validate_crosswalk(valid).is_ok());
        assert_eq!(valid["fixture_kind"], "synthetic_fixture");
    }
    #[test]
    fn fractional_utc_timestamps_follow_rfc3339_utc_shape() {
        assert!(utc_timestamp("2026-10-15T14:28:00.123456Z"));
        assert!(utc_timestamp("2024-02-29T23:59:59.1Z"));
        assert!(!utc_timestamp("2026-10-15T14:28:00.Z"));
        assert!(!utc_timestamp("2026-10-15T14:28:00+00:00"));
        assert!(!utc_timestamp("2026-02-31T14:28:00.1Z"));
    }

    #[test]
    fn local_snapshot_hashes_must_match_exact_bytes() {
        let source = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-source-snapshot.json"
        );
        let registry = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-locode-registry-snapshot.json"
        );

        let source_match = verify_snapshot_digest(
            source,
            "853e9639988bd96dead635195dc37390fd4d48b30ec29b062a7b1dfe43dbf73c",
            "source snapshot",
        );
        assert!(source_match.is_ok());

        let registry_match = verify_snapshot_digest(
            registry,
            "06432a56a5d354570bb431215af2e8637cb2d18fb05b93ccda26b09c7b8d79de",
            "UN/LOCODE registry",
        );
        assert!(registry_match.is_ok());

        let mismatch = verify_snapshot_digest(
            source,
            "0000000000000000000000000000000000000000000000000000000000000000",
            "source snapshot",
        )
        .expect_err("a wrong digest must fail closed");
        assert!(mismatch.contains("SHA-256 mismatch"));
    }

    #[test]
    fn registry_snapshot_mismatch_fails_closed_independently() {
        let registry = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-locode-registry-snapshot.json"
        );
        let error = verify_snapshot_digest(
            registry,
            "0000000000000000000000000000000000000000000000000000000000000000",
            "UN/LOCODE registry",
        )
        .expect_err("a wrong registry digest must fail closed");
        assert!(error.contains("UN/LOCODE registry SHA-256 mismatch"));
        assert!(error.contains("crosswalk declares"));
    }

    #[test]
    fn matching_snapshot_hashes_never_admit_port_identity_or_operation() {
        use super::build_crosswalk_report;

        let bytes = include_bytes!("../tests/fixtures/synthetic-port-crosswalk.json");
        let crosswalk: Value = serde_json::from_slice(bytes).expect("synthetic crosswalk fixture");
        let record_count = validate_crosswalk(&crosswalk).expect("metadata is valid");
        let source = member(&crosswalk, "source_snapshot").expect("source snapshot");
        let registry = member(&crosswalk, "locode_registry").expect("registry snapshot");
        let source_digest = text(source, "artifact_sha256").expect("source digest");
        let registry_digest = text(registry, "artifact_sha256").expect("registry digest");
        let source_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-source-snapshot.json"
        );
        let registry_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/synthetic-locode-registry-snapshot.json"
        );

        // Actually recompute both digests before constructing a positive report.
        verify_snapshot_digest(source_path, source_digest, "source snapshot")
            .expect("source snapshot bytes match the declaration");
        verify_snapshot_digest(registry_path, registry_digest, "UN/LOCODE registry")
            .expect("registry bytes match the declaration");

        let report = build_crosswalk_report(
            &crosswalk,
            bytes,
            record_count,
            Some(source_path.to_owned()),
            Some(registry_path.to_owned()),
            true,
            "test-commit".to_owned(),
        )
        .expect("verified-byte report");

        assert_eq!(report.status, VALID);
        assert!(report.declared_snapshot_hashes_match_local_bytes);
        assert_eq!(report.evidence_admission, "NotAdmitted");
        assert!(!report.source_authenticity_established);
        assert!(!report.port_identity_established);
        assert!(!report.operational_authority);
        assert!(
            report
                .claim_ceiling
                .iter()
                .any(|claim| claim.contains("does not establish publisher authenticity"))
        );
    }

    #[test]
    fn report_builder_rejects_unbacked_snapshot_match_claim() {
        use super::build_crosswalk_report;

        let bytes = include_bytes!("../tests/fixtures/synthetic-port-crosswalk.json");
        let crosswalk: Value = serde_json::from_slice(bytes).expect("synthetic crosswalk fixture");
        let record_count = validate_crosswalk(&crosswalk).expect("metadata is valid");

        let result = build_crosswalk_report(
            &crosswalk,
            bytes,
            record_count,
            None,
            None,
            true,
            "test-commit".to_owned(),
        );
        match result {
            Err(error) => assert!(error.contains("verification flag and checked-path evidence")),
            Ok(_) => panic!("report must reject a byte-match claim without checked paths"),
        }

        // Reject a half-present path pair and a path pair that contradicts the
        // declared match state, so malformed report states cannot be serialized.
        let partial = build_crosswalk_report(
            &crosswalk,
            bytes,
            record_count,
            Some("source-snapshot.json".to_owned()),
            None,
            false,
            "test-commit".to_owned(),
        );
        assert!(partial.is_err());
        let contradictory = build_crosswalk_report(
            &crosswalk,
            bytes,
            record_count,
            Some("source-snapshot.json".to_owned()),
            Some("locode-registry.json".to_owned()),
            false,
            "test-commit".to_owned(),
        );
        assert!(contradictory.is_err());
    }

    #[test]
    fn one_byte_source_snapshot_drift_is_detected_against_actual_file_bytes() {
        use std::fs;

        let mut changed =
            include_bytes!("../tests/fixtures/synthetic-source-snapshot.json").to_vec();
        changed.push(b' ');
        let path = std::env::temp_dir().join(format!(
            "symtropy-maritime-source-drift-{}.json",
            std::process::id()
        ));
        fs::write(&path, &changed).expect("write temporary drift fixture");
        let result = verify_snapshot_digest(
            path.to_str().expect("temporary path is UTF-8"),
            "853e9639988bd96dead635195dc37390fd4d48b30ec29b062a7b1dfe43dbf73c",
            "source snapshot",
        );
        let _ = fs::remove_file(&path);

        let error = result.expect_err("a single-byte change must fail exact-byte verification");
        assert!(error.contains("source snapshot SHA-256 mismatch"));
    }
}
