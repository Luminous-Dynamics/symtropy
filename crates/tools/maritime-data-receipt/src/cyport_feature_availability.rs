// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Strict inventory and fail-closed eligibility gate for CyPort's 56 source fields.
//! This validates field accounting and policy metadata, not upstream scientific semantics.

use super::{
    JsonValue, array, exact_keys, expect_text, field, integer, parse_strict, require_nonempty_text,
    text,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;

const MAX_REGISTER_BYTES: usize = 256 * 1024;
const SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-feature-availability";
const SCHEMA_VERSION: &str = "0.1.0";
const SOURCE_REVISION: &str = "5cb1d8778d3968c5e50d0b8cb8213f2414d692d3";
const SOURCE_BLOB_SHA1: &str = "21beb5b7673918143a0ab108b03acc3cccaeb330";
const SOURCE_BYTES: u64 = 643_047;
const EXPECTED_FIELDS: &[&str] = &[
    "ID",
    "SID",
    "PID",
    "port",
    "PORT_LON",
    "PORT_LAT",
    "start_date",
    "end_date",
    "start_recovery_date",
    "end_recovery_date",
    "total_impact",
    "total_impact_value",
    "max_impact",
    "day_of_recover",
    "DD",
    "CCD",
    "BCD",
    "SSHS",
    "SSHS_1",
    "SSHS_2",
    "SSHS_3",
    "SSHS_4",
    "SSHS_5",
    "IF_LANDFALL_CLOSE2PORT",
    "IF_LANDFALL",
    "WIND",
    "PRESSURE",
    "DISTANCE",
    "Wind_speed",
    "Surge_height",
    "Rainfall",
    "Port_ranking",
    "D_normal",
    "D_cyclone",
    "C_normal",
    "C_cyclone",
    "B_normal",
    "B_cyclone",
    "E_normal",
    "E_cyclone",
    "Seaport",
    "Pop_C",
    "Pop_Tract",
    "PCT_Pov",
    "WF",
    "PCT_TA",
    "PCT_ACEW",
    "PCT_TI",
    "PCT_SV",
    "SL",
    "Railway_Length",
    "Highway_Length",
    "Dock_Count",
    "Coast_Gulf_of_Mexico",
    "Coast_Atlantic",
    "Coast_Pacific",
];
const ALLOWED_ELIGIBILITY: &[&str] = &[
    "JOIN_OR_GROUP_KEY_ONLY",
    "FORBIDDEN_TARGET_OR_POST_EVENT",
    "BLOCKED_UNTIL_POINT_IN_TIME_EVIDENCE",
    "BLOCKED_UNTIL_NETWORK_WINDOW_AUDIT",
    "BLOCKED_UNTIL_VINTAGE_AND_JOIN_AUDIT",
    "BLOCKED_UNTIL_SEMANTIC_DICTIONARY",
];
const EXPECTED_LABEL_FIELDS: &[&str] = &[
    "start_recovery_date",
    "end_recovery_date",
    "total_impact",
    "total_impact_value",
    "max_impact",
    "day_of_recover",
];

#[derive(Debug, Serialize)]
struct FeatureAvailabilityReport {
    status: &'static str,
    manifest_sha256: String,
    source_revision: &'static str,
    source_resource_git_blob_sha1: &'static str,
    source_resource_byte_length: u64,
    field_count: usize,
    eligibility_counts: std::collections::BTreeMap<String, usize>,
    category_counts: std::collections::BTreeMap<String, usize>,
    approved_predictor_count: usize,
    all_fields_blocked_or_join_only: bool,
    point_in_time_evidence_required: bool,
    source_semantics_fully_validated: bool,
    predictive_model_fit_performed: bool,
    claim_ceiling: &'static str,
}

fn require_exact_string_set(
    value: &JsonValue,
    path: &str,
    expected: &[&str],
) -> Result<(), String> {
    let values = array(value, path)?;
    let mut actual = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let item = require_nonempty_text(item, &format!("{path}[{index}]"))?;
        if !actual.insert(item.to_owned()) {
            return Err(format!("{path}: duplicate entry {item:?}"));
        }
    }
    let actual: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected {
        return Err(format!(
            "{path}: field set mismatch; missing={:?}, unknown={:?}",
            expected.difference(&actual).copied().collect::<Vec<_>>(),
            actual.difference(&expected).copied().collect::<Vec<_>>()
        ));
    }
    Ok(())
}

fn validate_field(field_record: &JsonValue, index: usize) -> Result<(String, String), String> {
    let path = format!("fields[{index}]");
    exact_keys(
        field_record,
        &[
            "name",
            "category",
            "eligibility",
            "semantic_note",
            "required_evidence",
        ],
        &path,
    )?;
    let name = require_nonempty_text(field(field_record, "name", &path)?, &format!("{path}.name"))?;
    if EXPECTED_FIELDS.get(index).copied() != Some(name) {
        return Err(format!(
            "{path}.name: expected pinned source column {:?}, got {:?}",
            EXPECTED_FIELDS.get(index),
            name
        ));
    }
    let category = require_nonempty_text(
        field(field_record, "category", &path)?,
        &format!("{path}.category"),
    )?;
    let eligibility = require_nonempty_text(
        field(field_record, "eligibility", &path)?,
        &format!("{path}.eligibility"),
    )?;
    if !ALLOWED_ELIGIBILITY.contains(&eligibility) {
        return Err(format!(
            "{path}.eligibility: unapproved or unknown status {eligibility:?}"
        ));
    }
    if eligibility.starts_with("APPROVED") || eligibility.contains("PREDICTOR") {
        return Err(format!(
            "{path}.eligibility: v0.1 cannot approve any predictor"
        ));
    }
    require_nonempty_text(
        field(field_record, "semantic_note", &path)?,
        &format!("{path}.semantic_note"),
    )?;
    let evidence = array(
        field(field_record, "required_evidence", &path)?,
        &format!("{path}.required_evidence"),
    )?;
    if evidence.is_empty() {
        return Err(format!(
            "{path}.required_evidence: at least one evidence gate is required"
        ));
    }
    for (evidence_index, item) in evidence.iter().enumerate() {
        require_nonempty_text(item, &format!("{path}.required_evidence[{evidence_index}]"))?;
    }
    if EXPECTED_LABEL_FIELDS.contains(&name) && eligibility != "FORBIDDEN_TARGET_OR_POST_EVENT" {
        return Err(format!(
            "{path}.eligibility: label or post-event field {name:?} must stay forbidden"
        ));
    }
    if ["ID", "SID", "PID"].contains(&name) && eligibility != "JOIN_OR_GROUP_KEY_ONLY" {
        return Err(format!(
            "{path}.eligibility: identifier {name:?} must remain join/group-only"
        ));
    }
    Ok((category.to_owned(), eligibility.to_owned()))
}

fn validate_register(bytes: &[u8]) -> Result<FeatureAvailabilityReport, String> {
    if bytes.len() > MAX_REGISTER_BYTES {
        return Err(format!(
            "feature availability register exceeds {MAX_REGISTER_BYTES} bytes"
        ));
    }
    let root = parse_strict(bytes)?;
    exact_keys(
        &root,
        &[
            "schema_id",
            "schema_version",
            "status",
            "source",
            "field_policy",
            "fields",
            "hard_gates",
            "claim_ceiling",
        ],
        "root",
    )?;
    expect_text(field(&root, "schema_id", "root")?, "schema_id", SCHEMA_ID)?;
    expect_text(
        field(&root, "schema_version", "root")?,
        "schema_version",
        SCHEMA_VERSION,
    )?;
    expect_text(
        field(&root, "status", "root")?,
        "status",
        "FIELD_REGISTER_COMPLETE_NO_PREDICTORS_APPROVED",
    )?;

    let source = field(&root, "source", "root")?;
    exact_keys(
        source,
        &[
            "dataset_repository",
            "dataset_revision",
            "interaction_resource_path",
            "interaction_resource_git_blob_sha1",
            "interaction_resource_byte_length",
            "source_description_path",
            "source_description_note",
        ],
        "source",
    )?;
    expect_text(
        field(source, "dataset_repository", "source")?,
        "source.dataset_repository",
        "https://github.com/ChenchenMobility/Maritime-Data-CyPort",
    )?;
    expect_text(
        field(source, "interaction_resource_path", "source")?,
        "source.interaction_resource_path",
        "Tropical_cyclone_interaction/interaction_data.csv",
    )?;
    expect_text(
        field(source, "source_description_path", "source")?,
        "source.source_description_path",
        "https://github.com/ChenchenMobility/Maritime-Data-CyPort/blob/5cb1d8778d3968c5e50d0b8cb8213f2414d692d3/README.md",
    )?;
    expect_text(
        field(source, "dataset_revision", "source")?,
        "source.dataset_revision",
        SOURCE_REVISION,
    )?;
    expect_text(
        field(source, "interaction_resource_git_blob_sha1", "source")?,
        "source.interaction_resource_git_blob_sha1",
        SOURCE_BLOB_SHA1,
    )?;
    if integer(
        field(source, "interaction_resource_byte_length", "source")?,
        "source.interaction_resource_byte_length",
    )? != SOURCE_BYTES
    {
        return Err("source.interaction_resource_byte_length: pinned size mismatch".to_owned());
    }
    for key in [
        "dataset_repository",
        "interaction_resource_path",
        "source_description_path",
        "source_description_note",
    ] {
        require_nonempty_text(field(source, key, "source")?, &format!("source.{key}"))?;
    }

    let policy = field(&root, "field_policy", "root")?;
    exact_keys(
        policy,
        &[
            "classification_policy",
            "point_in_time_availability_required",
            "all_predictors_blocked_until_review",
            "approved_predictor_fields",
            "issue_time_required",
            "grouping_key",
            "join_only_keys",
            "outcome_or_post_event_fields",
        ],
        "field_policy",
    )?;
    for key in [
        "point_in_time_availability_required",
        "all_predictors_blocked_until_review",
        "issue_time_required",
    ] {
        match field(policy, key, "field_policy")? {
            JsonValue::Bool(true) => {}
            _ => return Err(format!("field_policy.{key}: must remain true")),
        }
    }
    require_nonempty_text(
        field(policy, "classification_policy", "field_policy")?,
        "field_policy.classification_policy",
    )?;
    expect_text(
        field(policy, "grouping_key", "field_policy")?,
        "field_policy.grouping_key",
        "SID",
    )?;
    require_exact_string_set(
        field(policy, "join_only_keys", "field_policy")?,
        "field_policy.join_only_keys",
        &["ID", "SID", "PID"],
    )?;
    require_exact_string_set(
        field(policy, "outcome_or_post_event_fields", "field_policy")?,
        "field_policy.outcome_or_post_event_fields",
        EXPECTED_LABEL_FIELDS,
    )?;
    if !array(
        field(policy, "approved_predictor_fields", "field_policy")?,
        "field_policy.approved_predictor_fields",
    )?
    .is_empty()
    {
        return Err(
            "field_policy.approved_predictor_fields: must remain empty until reviewed".to_owned(),
        );
    }

    let field_values = array(field(&root, "fields", "root")?, "fields")?;
    if field_values.len() != EXPECTED_FIELDS.len() {
        return Err(format!(
            "fields: expected exactly {} source fields, got {}",
            EXPECTED_FIELDS.len(),
            field_values.len()
        ));
    }
    let mut names = BTreeSet::new();
    let mut categories = std::collections::BTreeMap::<String, usize>::new();
    let mut eligibility_counts = std::collections::BTreeMap::<String, usize>::new();
    for (index, record) in field_values.iter().enumerate() {
        let (category, eligibility) = validate_field(record, index)?;
        let name = text(
            field(record, "name", &format!("fields[{index}]"))?,
            &format!("fields[{index}].name"),
        )?;
        if !names.insert(name.to_owned()) {
            return Err(format!("fields: duplicate source field {name:?}"));
        }
        *categories.entry(category).or_default() += 1;
        *eligibility_counts.entry(eligibility).or_default() += 1;
    }
    if names.len() != EXPECTED_FIELDS.len() {
        return Err("fields: not all 56 source fields are uniquely represented".to_owned());
    }

    let gates = field(&root, "hard_gates", "root")?;
    exact_keys(
        gates,
        &[
            "csv_header_order_pinned",
            "feature_whitelist_empty",
            "retrospective_realized_data_blocked_without_forecast_vintage",
            "network_features_require_pre_issue_time_snapshot",
            "source_outcome_fields_never_predictors",
            "public_holdout_not_blind",
        ],
        "hard_gates",
    )?;
    for key in [
        "csv_header_order_pinned",
        "feature_whitelist_empty",
        "retrospective_realized_data_blocked_without_forecast_vintage",
        "network_features_require_pre_issue_time_snapshot",
        "source_outcome_fields_never_predictors",
        "public_holdout_not_blind",
    ] {
        match field(gates, key, "hard_gates")? {
            JsonValue::Bool(true) => {}
            _ => return Err(format!("hard_gates.{key}: must remain true")),
        }
    }
    let claims = array(field(&root, "claim_ceiling", "root")?, "claim_ceiling")?;
    if claims.is_empty() {
        return Err("claim_ceiling must preserve explicit limitations".to_owned());
    }
    for (index, claim) in claims.iter().enumerate() {
        require_nonempty_text(claim, &format!("claim_ceiling[{index}]"))?;
    }

    Ok(FeatureAvailabilityReport {
        status: "FEATURE_AVAILABILITY_REGISTER_VALIDATED_NO_PREDICTORS_APPROVED",
        manifest_sha256: super::sha256_bytes(bytes),
        source_revision: SOURCE_REVISION,
        source_resource_git_blob_sha1: SOURCE_BLOB_SHA1,
        source_resource_byte_length: SOURCE_BYTES,
        field_count: names.len(),
        eligibility_counts,
        category_counts: categories,
        approved_predictor_count: 0,
        all_fields_blocked_or_join_only: true,
        point_in_time_evidence_required: true,
        source_semantics_fully_validated: false,
        predictive_model_fit_performed: false,
        claim_ceiling: "Column accounting and conservative feature policy only; individual source meanings and forecast-time availability are not yet certified.",
    })
}

pub fn run_cli(path: &str) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat feature register {path}: {error}"))?;
    if metadata.len() > MAX_REGISTER_BYTES as u64 {
        return Err(format!(
            "feature availability register exceeds {MAX_REGISTER_BYTES} bytes"
        ));
    }
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read feature register {path}: {error}"))?;
    let report = validate_register(&bytes)?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize feature availability report: {error}"))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const REGISTER: &[u8] =
        include_bytes!("../../../../docs/tech/CYPORT_FEATURE_AVAILABILITY_V0_1.json");

    #[test]
    fn all_56_source_fields_are_accounted_for_without_approving_predictors() {
        let report =
            validate_register(REGISTER).expect("field availability register should validate");
        assert_eq!(
            report.status,
            "FEATURE_AVAILABILITY_REGISTER_VALIDATED_NO_PREDICTORS_APPROVED"
        );
        assert_eq!(report.field_count, 56);
        assert_eq!(report.approved_predictor_count, 0);
        assert!(report.all_fields_blocked_or_join_only);
        assert!(report.point_in_time_evidence_required);
        assert!(!report.source_semantics_fully_validated);
        assert!(!report.predictive_model_fit_performed);
    }

    #[test]
    fn missing_source_field_fails_closed() {
        let mut root: Value = serde_json::from_slice(REGISTER).unwrap();
        root["fields"].as_array_mut().unwrap().remove(10);
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_register(&changed)
                .unwrap_err()
                .contains("expected exactly 56 source fields")
        );
    }

    #[test]
    fn raw_outcome_field_cannot_be_reclassified_as_a_predictor() {
        let mut root: Value = serde_json::from_slice(REGISTER).unwrap();
        root["fields"][10]["eligibility"] =
            Value::String("BLOCKED_UNTIL_POINT_IN_TIME_EVIDENCE".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_register(&changed)
                .unwrap_err()
                .contains("must stay forbidden")
        );
    }

    #[test]
    fn feature_whitelist_must_remain_empty() {
        let mut root: Value = serde_json::from_slice(REGISTER).unwrap();
        root["field_policy"]["approved_predictor_fields"] = serde_json::json!(["WIND"]);
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_register(&changed)
                .unwrap_err()
                .contains("approved_predictor_fields")
        );
    }

    #[test]
    fn source_field_order_drift_is_detected() {
        let mut root: Value = serde_json::from_slice(REGISTER).unwrap();
        root["fields"][0]["name"] = Value::String("SID".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_register(&changed)
                .unwrap_err()
                .contains("expected pinned source column")
        );
    }

    #[test]
    fn duplicate_json_keys_are_rejected() {
        let raw = String::from_utf8(REGISTER.to_vec()).unwrap();
        let changed = raw.replacen(
            "\"schema_version\": \"0.1.0\",",
            "\"schema_version\": \"0.1.0\", \"schema_version\": \"0.1.0\",",
            1,
        );
        assert!(
            validate_register(changed.as_bytes())
                .unwrap_err()
                .contains("duplicate object key")
        );
    }
}
