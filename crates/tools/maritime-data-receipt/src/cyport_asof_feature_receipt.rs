// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Validates point-in-time feature evidence and hashes captured artifacts.
//!
//! A valid receipt proves metadata consistency and local byte identity only. It
//! does not prove that the referenced CyPort row exists, that an archive was
//! accessible to an operator at the declared time, or that a feature is approved.

use super::{
    JsonValue, array, exact_keys, expect_text, field, integer, parse_strict, require_nonempty_text,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

const MAX_RECEIPT_BYTES: usize = 256 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 32 * 1024 * 1024;
const SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-asof-feature-receipt";
const SCHEMA_VERSION: &str = "0.1.0";
const SOURCE_DATASET_ID: &str = "cyport_us_port_tropical_cyclone_resilience";
const SOURCE_REVISION: &str = "5cb1d8778d3968c5e50d0b8cb8213f2414d692d3";
const SOURCE_PATH: &str = "Tropical_cyclone_interaction/interaction_data.csv";
const SOURCE_BLOB_SHA1: &str = "21beb5b7673918143a0ab108b03acc3cccaeb330";
const SOURCE_BYTES: u64 = 643_047;
const FEATURE_KEYS: &[&str] = &[
    "feature_name",
    "source_kind",
    "source_uri",
    "source_revision_or_product_id",
    "artifact_relative_path",
    "artifact_sha256",
    "artifact_byte_length",
    "source_publication_time_utc",
    "source_retrieval_time_utc",
    "feature_available_at_utc",
    "valid_start_utc",
    "valid_end_utc",
    "units",
    "value",
    "missing_reason",
    "derivation_id",
    "join_method",
    "storm_crosswalk_evidence_id",
    "target_date_semantics_review_id",
];

#[derive(Debug)]
struct FeatureFact {
    name: String,
    relative_path: String,
    sha256: String,
    byte_length: u64,
}

#[derive(Debug)]
struct ReceiptFacts {
    receipt_sha256: String,
    evidence_scope: String,
    cutoff_utc: String,
    crosswalk_status: String,
    features: Vec<FeatureFact>,
}

#[derive(Debug, Serialize)]
struct FeatureReceiptReport {
    status: &'static str,
    receipt_sha256: String,
    evidence_scope: String,
    feature_count: usize,
    artifacts_checked: usize,
    exact_artifact_bytes_match: bool,
    cutoff_time_format_and_order_verified: bool,
    source_identity_bound_to_pinned_dataset: bool,
    source_row_membership_verified: bool,
    crosswalk_declared_verified: bool,
    target_event_time_alignment_verified: bool,
    predictor_eligibility_authorized: bool,
    model_fit_performed: bool,
    replay_performed: bool,
    operational_authority: bool,
    claim_ceiling: &'static str,
}

fn text_field<'a>(object: &'a JsonValue, name: &str, parent: &str) -> Result<&'a str, String> {
    let path = format!("{parent}.{name}");
    require_nonempty_text(field(object, name, parent)?, &path)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_calendar_date(value: &str, path: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
    {
        return Err(format!("{path}: expected canonical YYYY-MM-DD date"));
    }
    let year = value[0..4]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid year"))?;
    let month = value[5..7]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid month"))?;
    let day = value[8..10]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid day"))?;
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > days_in_month {
        return Err(format!(
            "{path}: date is not a valid Gregorian calendar date"
        ));
    }
    Ok(())
}

fn valid_utc_timestamp(value: &str, path: &str) -> Result<(), String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes[0..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return Err(format!(
            "{path}: expected canonical UTC timestamp YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    valid_calendar_date(&value[0..10], path)?;
    let hour = value[11..13]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid hour"))?;
    let minute = value[14..16]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid minute"))?;
    let second = value[17..19]
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid second"))?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!("{path}: timestamp time component is out of range"));
    }
    Ok(())
}

fn sha256_is_lower_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_value_and_missing_reason(feature: &JsonValue, index: usize) -> Result<(), String> {
    let path = format!("features[{index}]");
    let value = field(feature, "value", &path)?;
    let reason = field(feature, "missing_reason", &path)?;
    match (value, reason) {
        (JsonValue::Null, JsonValue::String(reason)) if !reason.trim().is_empty() => Ok(()),
        (JsonValue::Number(_) | JsonValue::String(_) | JsonValue::Bool(_), JsonValue::Null) => {
            Ok(())
        }
        (JsonValue::Null, _) => Err(format!(
            "{path}.missing_reason: a missing value requires a non-empty reason"
        )),
        (_, _) => Err(format!(
            "{path}: value must be a scalar and missing_reason must be null unless value is missing"
        )),
    }
}

fn validate_receipt(bytes: &[u8]) -> Result<ReceiptFacts, String> {
    if bytes.len() > MAX_RECEIPT_BYTES {
        return Err(format!("receipt exceeds {MAX_RECEIPT_BYTES} bytes"));
    }
    let root = parse_strict(bytes)?;
    exact_keys(
        &root,
        &[
            "schema_id",
            "schema_version",
            "status",
            "evidence_scope",
            "source_dataset",
            "decision_cutoff_utc",
            "storm_crosswalk_status",
            "features",
            "claim_ceiling",
        ],
        "root",
    )?;
    expect_text(
        field(&root, "schema_id", "root")?,
        "root.schema_id",
        SCHEMA_ID,
    )?;
    expect_text(
        field(&root, "schema_version", "root")?,
        "root.schema_version",
        SCHEMA_VERSION,
    )?;
    expect_text(
        field(&root, "status", "root")?,
        "root.status",
        "EVIDENCE_RECEIPT_NOT_FEATURE_APPROVAL",
    )?;
    let evidence_scope = text_field(&root, "evidence_scope", "root")?;
    if evidence_scope != "SYNTHETIC_FIXTURE_ONLY"
        && evidence_scope != "CANDIDATE_EVIDENCE_NOT_SOURCE_ROW_VERIFIED"
    {
        return Err("root.evidence_scope: unknown evidence scope".to_owned());
    }

    let source = field(&root, "source_dataset", "root")?;
    exact_keys(
        source,
        &[
            "dataset_id",
            "revision",
            "resource_path",
            "git_blob_sha1",
            "byte_length",
            "interaction_id",
            "storm_sid",
            "port_pid",
            "target_start_date",
            "target_date_precision",
            "target_timezone_status",
        ],
        "source_dataset",
    )?;
    expect_text(
        field(source, "dataset_id", "source_dataset")?,
        "source_dataset.dataset_id",
        SOURCE_DATASET_ID,
    )?;
    expect_text(
        field(source, "revision", "source_dataset")?,
        "source_dataset.revision",
        SOURCE_REVISION,
    )?;
    expect_text(
        field(source, "resource_path", "source_dataset")?,
        "source_dataset.resource_path",
        SOURCE_PATH,
    )?;
    expect_text(
        field(source, "git_blob_sha1", "source_dataset")?,
        "source_dataset.git_blob_sha1",
        SOURCE_BLOB_SHA1,
    )?;
    if integer(
        field(source, "byte_length", "source_dataset")?,
        "source_dataset.byte_length",
    )? != SOURCE_BYTES
    {
        return Err("source_dataset.byte_length: does not match pinned source".to_owned());
    }
    for name in ["interaction_id", "storm_sid", "port_pid"] {
        text_field(source, name, "source_dataset")?;
    }
    let target_date = text_field(source, "target_start_date", "source_dataset")?;
    valid_calendar_date(target_date, "source_dataset.target_start_date")?;
    expect_text(
        field(source, "target_date_precision", "source_dataset")?,
        "source_dataset.target_date_precision",
        "DATE_ONLY",
    )?;
    expect_text(
        field(source, "target_timezone_status", "source_dataset")?,
        "source_dataset.target_timezone_status",
        "UNVERIFIED",
    )?;

    let cutoff = text_field(&root, "decision_cutoff_utc", "root")?;
    valid_utc_timestamp(cutoff, "decision_cutoff_utc")?;
    let crosswalk_status = text_field(&root, "storm_crosswalk_status", "root")?;
    if !["VERIFIED", "UNRESOLVED", "SYNTHETIC_ONLY"].contains(&crosswalk_status) {
        return Err("root.storm_crosswalk_status: unknown status".to_owned());
    }

    let feature_values = array(field(&root, "features", "root")?, "features")?;
    if feature_values.is_empty() || feature_values.len() > 256 {
        return Err("features: require between 1 and 256 feature receipts".to_owned());
    }
    let mut seen_names = BTreeSet::new();
    let mut facts = Vec::with_capacity(feature_values.len());
    for (index, feature) in feature_values.iter().enumerate() {
        let path = format!("features[{index}]");
        exact_keys(feature, FEATURE_KEYS, &path)?;
        let name = text_field(feature, "feature_name", &path)?;
        if !seen_names.insert(name.to_owned()) {
            return Err(format!(
                "{path}.feature_name: duplicate feature name {name:?}"
            ));
        }
        let source_kind = text_field(feature, "source_kind", &path)?;
        if ![
            "ARCHIVED_FORECAST",
            "PUBLISHED_OBSERVATION",
            "STATIC_CONTEXT",
        ]
        .contains(&source_kind)
        {
            return Err(format!(
                "{path}.source_kind: retrospective best-track and post-event data are forbidden"
            ));
        }
        let uri = text_field(feature, "source_uri", &path)?;
        let hostname = uri
            .strip_prefix("https://")
            .unwrap_or("")
            .split('/')
            .next()
            .unwrap_or("");
        if hostname.is_empty() {
            return Err(format!(
                "{path}.source_uri: expected an HTTPS URI with a host"
            ));
        }
        text_field(feature, "source_revision_or_product_id", &path)?;
        let relative_path = text_field(feature, "artifact_relative_path", &path)?;
        let artifact_path = Path::new(relative_path);
        if artifact_path.is_absolute()
            || relative_path.contains('\\')
            || artifact_path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "{path}.artifact_relative_path: unsafe relative path"
            ));
        }
        let artifact_sha256 = text_field(feature, "artifact_sha256", &path)?;
        if !sha256_is_lower_hex(artifact_sha256) {
            return Err(format!(
                "{path}.artifact_sha256: expected 64 lowercase hex characters"
            ));
        }
        let artifact_byte_length = integer(
            field(feature, "artifact_byte_length", &path)?,
            &format!("{path}.artifact_byte_length"),
        )?;
        if artifact_byte_length == 0 || artifact_byte_length > MAX_ARTIFACT_BYTES {
            return Err(format!(
                "{path}.artifact_byte_length: outside allowed size bounds"
            ));
        }

        let publication = text_field(feature, "source_publication_time_utc", &path)?;
        let retrieval = text_field(feature, "source_retrieval_time_utc", &path)?;
        let available = text_field(feature, "feature_available_at_utc", &path)?;
        let valid_start = text_field(feature, "valid_start_utc", &path)?;
        let valid_end = text_field(feature, "valid_end_utc", &path)?;
        for (name, value) in [
            ("source_publication_time_utc", publication),
            ("source_retrieval_time_utc", retrieval),
            ("feature_available_at_utc", available),
            ("valid_start_utc", valid_start),
            ("valid_end_utc", valid_end),
        ] {
            valid_utc_timestamp(value, &format!("{path}.{name}"))?;
        }
        if publication > cutoff {
            return Err(format!(
                "{path}.source_publication_time_utc: product postdates the decision cutoff"
            ));
        }
        if available < publication {
            return Err(format!(
                "{path}.feature_available_at_utc: cannot precede publication"
            ));
        }
        if available > cutoff {
            return Err(format!(
                "{path}.feature_available_at_utc: feature was unavailable at the decision cutoff"
            ));
        }
        if retrieval < publication {
            return Err(format!(
                "{path}.source_retrieval_time_utc: retrieval predates source publication"
            ));
        }
        if valid_start > valid_end {
            return Err(format!(
                "{path}: forecast/valid interval starts after it ends"
            ));
        }
        let units = text_field(feature, "units", &path)?;
        if ["unknown", "unverified", "unspecified"].contains(&units.to_ascii_lowercase().as_str()) {
            return Err(format!(
                "{path}.units: unresolved units are not admissible evidence"
            ));
        }
        text_field(feature, "derivation_id", &path)?;
        text_field(feature, "join_method", &path)?;
        text_field(feature, "storm_crosswalk_evidence_id", &path)?;
        text_field(feature, "target_date_semantics_review_id", &path)?;
        validate_value_and_missing_reason(feature, index)?;
        facts.push(FeatureFact {
            name: name.to_owned(),
            relative_path: relative_path.to_owned(),
            sha256: artifact_sha256.to_owned(),
            byte_length: artifact_byte_length,
        });
    }

    let claim_ceiling = array(field(&root, "claim_ceiling", "root")?, "claim_ceiling")?;
    if claim_ceiling.len() < 3 {
        return Err("claim_ceiling must state provenance and prediction non-claims".to_owned());
    }
    for (index, claim) in claim_ceiling.iter().enumerate() {
        require_nonempty_text(claim, &format!("claim_ceiling[{index}]"))?;
    }

    Ok(ReceiptFacts {
        receipt_sha256: sha256_hex(bytes),
        evidence_scope: evidence_scope.to_owned(),
        cutoff_utc: cutoff.to_owned(),
        crosswalk_status: crosswalk_status.to_owned(),
        features: facts,
    })
}

fn verify_artifact_bytes(facts: &ReceiptFacts, artifact_root: &Path) -> Result<(), String> {
    let root = artifact_root.canonicalize().map_err(|error| {
        format!(
            "cannot resolve artifact root {}: {error}",
            artifact_root.display()
        )
    })?;
    if !root.is_dir() {
        return Err("artifact root must be a directory".to_owned());
    }
    for feature in &facts.features {
        let joined = root.join(&feature.relative_path);
        let canonical = joined.canonicalize().map_err(|error| {
            format!(
                "feature {:?}: cannot resolve artifact {}: {error}",
                feature.name,
                joined.display()
            )
        })?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(format!(
                "feature {:?}: artifact must be a regular file under the declared artifact root",
                feature.name
            ));
        }
        let metadata = fs::metadata(&canonical)
            .map_err(|error| format!("cannot stat artifact {}: {error}", canonical.display()))?;
        if metadata.len() > MAX_ARTIFACT_BYTES {
            return Err(format!(
                "artifact {} exceeds size limit",
                canonical.display()
            ));
        }
        let bytes = fs::read(&canonical)
            .map_err(|error| format!("cannot read artifact {}: {error}", canonical.display()))?;
        if bytes.len() as u64 != feature.byte_length {
            return Err(format!(
                "feature {:?}: artifact length mismatch (declared {}, observed {})",
                feature.name,
                feature.byte_length,
                bytes.len()
            ));
        }
        let observed = sha256_hex(&bytes);
        if observed != feature.sha256 {
            return Err(format!(
                "feature {:?}: artifact SHA-256 mismatch",
                feature.name
            ));
        }
    }
    Ok(())
}

pub fn run_cli(receipt_path: &str, artifact_root: &str) -> Result<(), String> {
    let metadata = fs::metadata(receipt_path)
        .map_err(|error| format!("cannot stat feature evidence receipt {receipt_path}: {error}"))?;
    if metadata.len() > MAX_RECEIPT_BYTES as u64 {
        return Err(format!(
            "feature evidence receipt exceeds {MAX_RECEIPT_BYTES} bytes"
        ));
    }
    let bytes = fs::read(receipt_path)
        .map_err(|error| format!("cannot read feature evidence receipt {receipt_path}: {error}"))?;
    let facts = validate_receipt(&bytes)?;
    verify_artifact_bytes(&facts, Path::new(artifact_root))?;
    let report = FeatureReceiptReport {
        status: "FEATURE_EVIDENCE_METADATA_AND_ARTIFACTS_MATCH_NOT_FEATURE_APPROVED",
        receipt_sha256: facts.receipt_sha256,
        evidence_scope: facts.evidence_scope,
        feature_count: facts.features.len(),
        artifacts_checked: facts.features.len(),
        exact_artifact_bytes_match: true,
        cutoff_time_format_and_order_verified: true,
        source_identity_bound_to_pinned_dataset: true,
        source_row_membership_verified: false,
        crosswalk_declared_verified: facts.crosswalk_status == "VERIFIED",
        target_event_time_alignment_verified: false,
        predictor_eligibility_authorized: false,
        model_fit_performed: false,
        replay_performed: false,
        operational_authority: false,
        claim_ceiling: "Receipt bytes and metadata only; no forecast or operational authority.",
    };
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize feature evidence report: {error}"))?
    );
    let _cutoff_recorded_for_report = facts.cutoff_utc;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const RECEIPT: &[u8] = include_bytes!("../tests/fixtures/synthetic-asof-feature-receipt.json");

    #[test]
    fn json_schema_is_valid_and_matches_the_receipt_contract() {
        let schema =
            include_str!("../../../../docs/tech/CYPORT_ASOF_FEATURE_RECEIPT_V0_1.schema.json");
        let value: serde_json::Value =
            serde_json::from_str(schema).expect("receipt schema must be valid JSON");
        assert_eq!(
            value["properties"]["schema_id"]["const"],
            serde_json::Value::String(SCHEMA_ID.to_owned())
        );
        assert_eq!(
            value["properties"]["schema_version"]["const"],
            serde_json::Value::String(SCHEMA_VERSION.to_owned())
        );
        assert_eq!(
            value["$defs"]["feature"]["required"]
                .as_array()
                .expect("feature required fields must be an array")
                .len(),
            FEATURE_KEYS.len()
        );
    }

    #[test]
    fn synthetic_evidence_receipt_passes_metadata_checks_only() {
        let facts = validate_receipt(RECEIPT).expect("synthetic receipt metadata should validate");
        assert_eq!(facts.features.len(), 1);
        assert_eq!(facts.crosswalk_status, "SYNTHETIC_ONLY");
        assert_eq!(facts.evidence_scope, "SYNTHETIC_FIXTURE_ONLY");
        assert_eq!(facts.cutoff_utc, "2022-09-23T12:30:00Z");
        assert!(!facts.receipt_sha256.is_empty());
    }

    #[test]
    fn artifact_digest_mismatch_is_rejected() {
        let mut facts = validate_receipt(RECEIPT).expect("synthetic receipt should validate");
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time should be after the Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "symtropy-asof-receipt-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("temporary artifact root should be created");
        fs::write(
            root.join("artifact.bin"),
            include_bytes!("../tests/fixtures/synthetic-asof-forecast.txt"),
        )
        .expect("temporary artifact should be written");
        facts.features[0].relative_path = "artifact.bin".to_owned();
        facts.features[0].sha256 = "0".repeat(64);
        let result = verify_artifact_bytes(&facts, &root);
        let _ = fs::remove_dir_all(&root);
        assert!(
            result.unwrap_err().contains("artifact SHA-256 mismatch"),
            "the verifier must reject a wrong artifact digest"
        );
    }

    #[test]
    fn post_cutoff_feature_availability_is_rejected() {
        let mut value: Value = serde_json::from_slice(RECEIPT).expect("valid fixture JSON");
        value["features"][0]["feature_available_at_utc"] =
            serde_json::json!("2022-09-23T12:31:00Z");
        let changed = serde_json::to_vec(&value).expect("serialize mutated receipt");
        assert!(
            validate_receipt(&changed)
                .unwrap_err()
                .contains("unavailable at the decision cutoff")
        );
    }

    #[test]
    fn retrospective_best_track_source_kind_is_rejected() {
        let mut value: Value = serde_json::from_slice(RECEIPT).expect("valid fixture JSON");
        value["features"][0]["source_kind"] = serde_json::json!("RETROSPECTIVE_BEST_TRACK");
        let changed = serde_json::to_vec(&value).expect("serialize mutated receipt");
        assert!(
            validate_receipt(&changed)
                .unwrap_err()
                .contains("retrospective best-track")
        );
    }

    #[test]
    fn invalid_calendar_timestamp_is_rejected() {
        let mut value: Value = serde_json::from_slice(RECEIPT).expect("valid fixture JSON");
        value["decision_cutoff_utc"] = serde_json::json!("2022-02-31T12:30:00Z");
        let changed = serde_json::to_vec(&value).expect("serialize mutated receipt");
        assert!(
            validate_receipt(&changed)
                .unwrap_err()
                .contains("calendar date")
        );
    }

    #[test]
    fn missing_values_require_explicit_reasons() {
        let mut value: Value = serde_json::from_slice(RECEIPT).expect("valid fixture JSON");
        value["features"][0]["value"] = serde_json::Value::Null;
        value["features"][0]["missing_reason"] = serde_json::Value::Null;
        let changed = serde_json::to_vec(&value).expect("serialize mutated receipt");
        assert!(
            validate_receipt(&changed)
                .unwrap_err()
                .contains("missing value requires")
        );
    }
}
