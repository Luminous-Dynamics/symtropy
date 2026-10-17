// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Strict validator for the frozen CyPort benchmark-task contract.
//!
//! This validates definitions and safeguards only. It does not build features,
//! fit models, calculate scores, or establish physical truth.

use super::{
    JsonValue, array, exact_keys, expect_text, field, integer, parse_strict, require_nonempty_text,
};
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;

const MAX_CONTRACT_BYTES: usize = 256 * 1024;
const SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-benchmark-tasks";
const SCHEMA_VERSION: &str = "0.1.0";
const SOURCE_REVISION: &str = "5cb1d8778d3968c5e50d0b8cb8213f2414d692d3";
const SOURCE_PATH: &str = "Tropical_cyclone_interaction/interaction_data.csv";
const SOURCE_BLOB_SHA1: &str = "21beb5b7673918143a0ab108b03acc3cccaeb330";
const SOURCE_BYTES: u64 = 643_047;
const SPLIT_PATH: &str = "docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json";
const SPLIT_MANIFEST_SCHEMA_ID: &str = "luminous.symtropy.maritime.cyport-episode-split";
const SPLIT_MANIFEST_SCHEMA_VERSION: &str = "0.1.0";

const TASK_IDS: &[&str] = &[
    "impact_window_marker_state",
    "rounded_impact_occurrence",
    "rounded_impact_severity",
    "recovery_duration_descriptive",
];
const TARGET_FIELDS: &[&[&str]] = &[
    &["start_recovery_date", "end_recovery_date", "day_of_recover"],
    &["total_impact"],
    &["total_impact"],
    &["day_of_recover", "start_recovery_date", "end_recovery_date"],
];
const TASK_STATUSES: &[&str] = &[
    "DEFINED_NOT_READY_FOR_MODEL_FIT",
    "DEFINED_NOT_READY_FOR_MODEL_FIT",
    "DEFINED_NOT_READY_FOR_MODEL_FIT",
    "DESCRIPTIVE_ONLY_CENSORING_UNRESOLVED",
];
const FORBIDDEN_FEATURES: &[&str] = &[
    "total_impact",
    "total_impact_value",
    "max_impact",
    "start_recovery_date",
    "end_recovery_date",
    "day_of_recover",
    "ID",
    "SID",
    "PID",
];

#[derive(Debug, Serialize)]
struct BenchmarkContractReport {
    status: &'static str,
    contract_sha256: String,
    task_count: usize,
    task_ids: Vec<String>,
    split_manifest_schema_id: &'static str,
    split_storm_count: u64,
    split_row_count: u64,
    feature_whitelist_empty: bool,
    point_in_time_audit_required: bool,
    source_identity_bound: bool,
    model_fit_performed: bool,
    prediction_authorized: bool,
    claim_ceiling: &'static str,
}

fn require_array_strings(
    value: &JsonValue,
    path: &str,
    require_nonempty: bool,
) -> Result<Vec<String>, String> {
    let values = array(value, path)?;
    if require_nonempty && values.is_empty() {
        return Err(format!("{path}: array must not be empty"));
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::with_capacity(values.len());
    for (index, item) in values.iter().enumerate() {
        let item = require_nonempty_text(item, &format!("{path}[{index}]"))?;
        if !seen.insert(item.to_owned()) {
            return Err(format!("{path}: duplicate entry {item:?}"));
        }
        result.push(item.to_owned());
    }
    Ok(result)
}

fn require_exact_string_set(
    value: &JsonValue,
    path: &str,
    expected: &[&str],
) -> Result<(), String> {
    let actual = require_array_strings(value, path, true)?;
    let actual: BTreeSet<&str> = actual.iter().map(String::as_str).collect();
    let expected: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != expected {
        return Err(format!(
            "{path}: required set mismatch; missing={:?}, unknown={:?}",
            expected.difference(&actual).copied().collect::<Vec<_>>(),
            actual.difference(&expected).copied().collect::<Vec<_>>()
        ));
    }
    Ok(())
}

fn validate_task(task: &JsonValue, index: usize) -> Result<String, String> {
    let path = format!("tasks[{index}]");
    exact_keys(
        task,
        &[
            "id",
            "status",
            "estimand",
            "target_fields",
            "label_definition",
            "sample_policy",
            "permitted_feature_fields",
            "forbidden_feature_fields",
            "metrics",
            "baselines",
            "benchmark_goal",
        ],
        &path,
    )?;
    let task_id = require_nonempty_text(field(task, "id", &path)?, &format!("{path}.id"))?;
    if TASK_IDS.get(index).copied() != Some(task_id) {
        return Err(format!("{path}.id: unexpected task ID {task_id:?}"));
    }
    expect_text(
        field(task, "status", &path)?,
        &format!("{path}.status"),
        TASK_STATUSES[index],
    )?;
    for key in [
        "estimand",
        "label_definition",
        "sample_policy",
        "benchmark_goal",
    ] {
        require_nonempty_text(field(task, key, &path)?, &format!("{path}.{key}"))?;
    }
    require_exact_string_set(
        field(task, "target_fields", &path)?,
        &format!("{path}.target_fields"),
        TARGET_FIELDS[index],
    )?;

    let permitted = require_array_strings(
        field(task, "permitted_feature_fields", &path)?,
        &format!("{path}.permitted_feature_fields"),
        false,
    )?;
    if !permitted.is_empty() {
        return Err(format!(
            "{path}.permitted_feature_fields: must remain empty until point-in-time feature availability is independently audited"
        ));
    }
    require_exact_string_set(
        field(task, "forbidden_feature_fields", &path)?,
        &format!("{path}.forbidden_feature_fields"),
        FORBIDDEN_FEATURES,
    )?;
    for key in ["metrics", "baselines"] {
        require_array_strings(field(task, key, &path)?, &format!("{path}.{key}"), true)?;
    }
    Ok(task_id.to_owned())
}

fn validate_contract(bytes: &[u8]) -> Result<BenchmarkContractReport, String> {
    if bytes.len() > MAX_CONTRACT_BYTES {
        return Err(format!(
            "benchmark contract exceeds {MAX_CONTRACT_BYTES} bytes"
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
            "split_manifest",
            "feature_policy",
            "tasks",
            "protocol",
            "paper_reproduction",
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
        "FROZEN_BENCHMARK_CONTRACT_NOT_MODEL_VALIDATED",
    )?;

    let source = field(&root, "source", "root")?;
    exact_keys(
        source,
        &["id", "revision", "path", "git_blob_sha1", "byte_length"],
        "source",
    )?;
    expect_text(
        field(source, "id", "source")?,
        "source.id",
        "cyport_us_port_tropical_cyclone_resilience",
    )?;
    expect_text(
        field(source, "revision", "source")?,
        "source.revision",
        SOURCE_REVISION,
    )?;
    expect_text(field(source, "path", "source")?, "source.path", SOURCE_PATH)?;
    expect_text(
        field(source, "git_blob_sha1", "source")?,
        "source.git_blob_sha1",
        SOURCE_BLOB_SHA1,
    )?;
    if integer(
        field(source, "byte_length", "source")?,
        "source.byte_length",
    )? != SOURCE_BYTES
    {
        return Err("source.byte_length: pinned byte length mismatch".to_owned());
    }

    let split = field(&root, "split_manifest", "root")?;
    exact_keys(
        split,
        &[
            "path",
            "schema_id",
            "schema_version",
            "split_unit",
            "splits",
            "total_storms",
            "total_rows",
        ],
        "split_manifest",
    )?;
    expect_text(
        field(split, "path", "split_manifest")?,
        "split_manifest.path",
        SPLIT_PATH,
    )?;
    expect_text(
        field(split, "schema_id", "split_manifest")?,
        "split_manifest.schema_id",
        SPLIT_MANIFEST_SCHEMA_ID,
    )?;
    expect_text(
        field(split, "schema_version", "split_manifest")?,
        "split_manifest.schema_version",
        SPLIT_MANIFEST_SCHEMA_VERSION,
    )?;
    expect_text(
        field(split, "split_unit", "split_manifest")?,
        "split_manifest.split_unit",
        "SID",
    )?;
    let expected_splits = [
        ("train", "2015-2020", 59_u64, 1_471_u64),
        ("calibration", "2021", 13_u64, 308_u64),
        ("test", "2022-2023", 18_u64, 148_u64),
    ];
    let observed_splits = array(
        field(split, "splits", "split_manifest")?,
        "split_manifest.splits",
    )?;
    if observed_splits.len() != expected_splits.len() {
        return Err("split_manifest.splits: expected exactly three frozen partitions".to_owned());
    }
    for (index, (name, years, storms, rows)) in expected_splits.iter().enumerate() {
        let path = format!("split_manifest.splits[{index}]");
        let observed = &observed_splits[index];
        exact_keys(observed, &["name", "years", "storms", "rows"], &path)?;
        expect_text(
            field(observed, "name", &path)?,
            &format!("{path}.name"),
            name,
        )?;
        expect_text(
            field(observed, "years", &path)?,
            &format!("{path}.years"),
            years,
        )?;
        if integer(field(observed, "storms", &path)?, &format!("{path}.storms"))? != *storms
            || integer(field(observed, "rows", &path)?, &format!("{path}.rows"))? != *rows
        {
            return Err(format!("{path}: frozen storm or row totals differ"));
        }
    }
    let storms = integer(
        field(split, "total_storms", "split_manifest")?,
        "split_manifest.total_storms",
    )?;
    let rows = integer(
        field(split, "total_rows", "split_manifest")?,
        "split_manifest.total_rows",
    )?;
    if storms != 90 || rows != 1_927 {
        return Err("split_manifest: frozen total counts differ".to_owned());
    }

    let features = field(&root, "feature_policy", "root")?;
    exact_keys(
        features,
        &[
            "status",
            "forecast_issue_time_required",
            "split_group_key",
            "join_only_keys",
            "outcome_or_post_event_fields_forbidden",
            "unverified_exposure_fields",
            "allowed_feature_fields",
            "point_in_time_provenance_required",
            "data_availability_gate",
        ],
        "feature_policy",
    )?;
    expect_text(
        field(features, "status", "feature_policy")?,
        "feature_policy.status",
        "NO_FEATURES_WHITELISTED_PENDING_POINT_IN_TIME_AUDIT",
    )?;
    expect_text(
        field(features, "split_group_key", "feature_policy")?,
        "feature_policy.split_group_key",
        "SID",
    )?;
    for key in [
        "forecast_issue_time_required",
        "point_in_time_provenance_required",
    ] {
        match field(features, key, "feature_policy")? {
            JsonValue::Bool(true) => {}
            _ => return Err(format!("feature_policy.{key}: must remain true")),
        }
    }
    require_exact_string_set(
        field(features, "join_only_keys", "feature_policy")?,
        "feature_policy.join_only_keys",
        &["ID", "SID", "PID"],
    )?;
    require_exact_string_set(
        field(
            features,
            "outcome_or_post_event_fields_forbidden",
            "feature_policy",
        )?,
        "feature_policy.outcome_or_post_event_fields_forbidden",
        &[
            "total_impact",
            "total_impact_value",
            "max_impact",
            "start_recovery_date",
            "end_recovery_date",
            "day_of_recover",
        ],
    )?;
    require_array_strings(
        field(features, "unverified_exposure_fields", "feature_policy")?,
        "feature_policy.unverified_exposure_fields",
        true,
    )?;
    if !require_array_strings(
        field(features, "allowed_feature_fields", "feature_policy")?,
        "feature_policy.allowed_feature_fields",
        false,
    )?
    .is_empty()
    {
        return Err(
            "feature_policy.allowed_feature_fields must remain empty pending audit".to_owned(),
        );
    }
    require_nonempty_text(
        field(features, "data_availability_gate", "feature_policy")?,
        "feature_policy.data_availability_gate",
    )?;

    let tasks = array(field(&root, "tasks", "root")?, "tasks")?;
    if tasks.len() != TASK_IDS.len() {
        return Err(format!(
            "tasks: expected exactly {} frozen task definitions",
            TASK_IDS.len()
        ));
    }
    let task_ids = tasks
        .iter()
        .enumerate()
        .map(|(index, task)| validate_task(task, index))
        .collect::<Result<Vec<_>, _>>()?;
    let unique_tasks: BTreeSet<&str> = task_ids.iter().map(String::as_str).collect();
    if unique_tasks.len() != TASK_IDS.len() {
        return Err("tasks: duplicate task identifiers".to_owned());
    }

    let protocol = field(&root, "protocol", "root")?;
    exact_keys(
        protocol,
        &[
            "train_only_transform_fit",
            "calibration_split",
            "test_split",
            "test_once_after_protocol_lock",
            "group_key",
            "uncertainty_method",
            "bootstrap_replicates",
            "random_seed",
            "raw_accuracy_primary_metric",
            "report_confusion_matrix_for_classification",
            "report_positive_storm_cluster_count",
            "suppress_confirmatory_language_if_positive_test_storm_clusters_below",
            "feature_availability_audit_required",
            "all_tuning_and_threshold_selection_confined_to_train_and_calibration",
        ],
        "protocol",
    )?;
    for key in [
        "train_only_transform_fit",
        "test_once_after_protocol_lock",
        "report_confusion_matrix_for_classification",
        "report_positive_storm_cluster_count",
        "feature_availability_audit_required",
        "all_tuning_and_threshold_selection_confined_to_train_and_calibration",
    ] {
        match field(protocol, key, "protocol")? {
            JsonValue::Bool(true) => {}
            _ => return Err(format!("protocol.{key}: must remain true")),
        }
    }
    match field(protocol, "raw_accuracy_primary_metric", "protocol")? {
        JsonValue::Bool(false) => {}
        _ => return Err("protocol.raw_accuracy_primary_metric must remain false".to_owned()),
    }
    expect_text(
        field(protocol, "calibration_split", "protocol")?,
        "protocol.calibration_split",
        "calibration",
    )?;
    expect_text(
        field(protocol, "test_split", "protocol")?,
        "protocol.test_split",
        "test",
    )?;
    expect_text(
        field(protocol, "group_key", "protocol")?,
        "protocol.group_key",
        "SID",
    )?;
    expect_text(
        field(protocol, "uncertainty_method", "protocol")?,
        "protocol.uncertainty_method",
        "storm_cluster_bootstrap",
    )?;
    if integer(
        field(protocol, "bootstrap_replicates", "protocol")?,
        "protocol.bootstrap_replicates",
    )? < 1_000
    {
        return Err("protocol.bootstrap_replicates must be at least 1000".to_owned());
    }
    if integer(
        field(
            protocol,
            "suppress_confirmatory_language_if_positive_test_storm_clusters_below",
            "protocol",
        )?,
        "protocol.suppress_confirmatory_language_if_positive_test_storm_clusters_below",
    )? < 1
    {
        return Err(
            "protocol must suppress confirmatory claims when positive test storms are sparse"
                .to_owned(),
        );
    }
    integer(
        field(protocol, "random_seed", "protocol")?,
        "protocol.random_seed",
    )?;

    let reproduction = field(&root, "paper_reproduction", "root")?;
    exact_keys(
        reproduction,
        &[
            "status",
            "paper_doi",
            "distinction",
            "required_before_comparison",
        ],
        "paper_reproduction",
    )?;
    expect_text(
        field(reproduction, "status", "paper_reproduction")?,
        "paper_reproduction.status",
        "SEPARATE_TRACK_NOT_IMPLEMENTED",
    )?;
    expect_text(
        field(reproduction, "paper_doi", "paper_reproduction")?,
        "paper_reproduction.paper_doi",
        "10.1016/j.trd.2026.105420",
    )?;
    require_nonempty_text(
        field(reproduction, "distinction", "paper_reproduction")?,
        "paper_reproduction.distinction",
    )?;
    require_array_strings(
        field(
            reproduction,
            "required_before_comparison",
            "paper_reproduction",
        )?,
        "paper_reproduction.required_before_comparison",
        true,
    )?;
    require_array_strings(
        field(&root, "claim_ceiling", "root")?,
        "claim_ceiling",
        true,
    )?;

    Ok(BenchmarkContractReport {
        status: "BENCHMARK_TASK_CONTRACT_VALIDATED_NOT_MODEL_VALIDATED",
        contract_sha256: super::sha256_bytes(bytes),
        task_count: task_ids.len(),
        task_ids,
        split_manifest_schema_id: SPLIT_MANIFEST_SCHEMA_ID,
        split_storm_count: storms,
        split_row_count: rows,
        feature_whitelist_empty: true,
        point_in_time_audit_required: true,
        source_identity_bound: true,
        model_fit_performed: false,
        prediction_authorized: false,
        claim_ceiling: "Task definitions and evaluation safeguards only; no fit, score, causal or operational authority.",
    })
}

pub fn run_cli(path: &str) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|error| format!("cannot stat benchmark contract {path}: {error}"))?;
    if metadata.len() > MAX_CONTRACT_BYTES as u64 {
        return Err(format!(
            "benchmark contract exceeds {MAX_CONTRACT_BYTES} bytes"
        ));
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read benchmark contract {path}: {error}"))?;
    let report = validate_contract(&bytes)?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize benchmark contract report: {error}"))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const CONTRACT: &[u8] =
        include_bytes!("../../../../docs/tech/CYPORT_BENCHMARK_TASKS_V0_1.json");

    #[test]
    fn frozen_benchmark_contract_validates_without_claiming_model_evaluation() {
        let report = validate_contract(CONTRACT).expect("benchmark contract should validate");
        assert_eq!(
            report.status,
            "BENCHMARK_TASK_CONTRACT_VALIDATED_NOT_MODEL_VALIDATED"
        );
        assert_eq!(report.task_count, 4);
        assert_eq!(report.split_storm_count, 90);
        assert_eq!(report.split_row_count, 1_927);
        assert!(report.feature_whitelist_empty);
        assert!(report.point_in_time_audit_required);
        assert!(!report.model_fit_performed);
        assert!(!report.prediction_authorized);
    }

    #[test]
    fn any_nonempty_feature_whitelist_fails_closed() {
        let mut root: Value = serde_json::from_slice(CONTRACT).unwrap();
        root["feature_policy"]["allowed_feature_fields"] = serde_json::json!(["WIND"]);
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("allowed_feature_fields")
        );
    }

    #[test]
    fn outcome_leakage_into_permitted_features_is_rejected() {
        let mut root: Value = serde_json::from_slice(CONTRACT).unwrap();
        root["tasks"][0]["permitted_feature_fields"] = serde_json::json!(["total_impact"]);
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("permitted_feature_fields")
        );
    }

    #[test]
    fn a_row_randomized_split_cannot_replace_cyclone_grouping() {
        let mut root: Value = serde_json::from_slice(CONTRACT).unwrap();
        root["protocol"]["group_key"] = Value::String("PID".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("protocol.group_key")
        );
    }

    #[test]
    fn public_temporal_holdout_cannot_be_relabelled_as_blind() {
        let mut root: Value = serde_json::from_slice(CONTRACT).unwrap();
        root["paper_reproduction"]["status"] = Value::String("BLIND_TEST".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_contract(&changed)
                .unwrap_err()
                .contains("paper_reproduction.status")
        );
    }

    #[test]
    fn duplicate_json_keys_are_rejected_before_contract_validation() {
        let mut raw = String::from_utf8(CONTRACT.to_vec()).unwrap();
        raw = raw.replacen(
            "\"schema_version\": \"0.1.0\",",
            "\"schema_version\": \"0.1.0\", \"schema_version\": \"0.1.0\",",
            1,
        );
        assert!(
            validate_contract(raw.as_bytes())
                .unwrap_err()
                .contains("duplicate object key")
        );
    }
}
