//! Rights-, snapshot- and metadata-aware admission gate for navigation datasets.
//!
//! JSON Schema validates the record shape. This semantic layer prevents a
//! paper/repository citation from being treated as an admitted benchmark
//! payload without explicit payload rights, immutable content hashes, declared
//! time/frame/calibration/ground-truth state and a leakage-resistant split.
//! Candidate records are allowed to remain incomplete but are never counted as
//! admitted. Unknown metadata is preserved as unknown, not upgraded.

use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAdmissionIssue {
    pub path: String,
    pub message: String,
}

impl fmt::Display for DatasetAdmissionIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatasetAdmissionSummary {
    pub requested_use: String,
    pub record_count: usize,
    pub admitted_count: usize,
    pub candidate_count: usize,
    pub quarantined_count: usize,
    pub unqualified_time_count: usize,
    pub unknown_time_count: usize,
    pub unqualified_frame_count: usize,
    pub unknown_frame_count: usize,
    pub clock_state_truth_not_provided_count: usize,
    pub clock_state_truth_unknown_count: usize,
    pub partial_ground_truth_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatasetUse {
    ResearchNonCommercial,
    Commercial,
}

impl DatasetUse {
    fn parse(value: &str) -> Result<Self, Vec<DatasetAdmissionIssue>> {
        match value {
            "research_noncommercial" => Ok(Self::ResearchNonCommercial),
            "commercial" => Ok(Self::Commercial),
            _ => Err(vec![DatasetAdmissionIssue {
                path: "$requested_use".to_owned(),
                message: "requested use must be research_noncommercial or commercial".to_owned(),
            }]),
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::ResearchNonCommercial => "research_noncommercial",
            Self::Commercial => "commercial",
        }
    }

    fn admitted_by_scope(&self, scope: &str) -> bool {
        match self {
            Self::ResearchNonCommercial => {
                matches!(scope, "research_noncommercial" | "both")
            }
            Self::Commercial => matches!(scope, "commercial" | "both"),
        }
    }

    fn permitted_by_rights(&self, rights: &Value) -> bool {
        let key = match self {
            Self::ResearchNonCommercial => "research_use",
            Self::Commercial => "commercial_use",
        };
        rights.get(key).and_then(Value::as_str) == Some("permitted")
    }
}

fn issue(
    issues: &mut Vec<DatasetAdmissionIssue>,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(DatasetAdmissionIssue {
        path: path.into(),
        message: message.into(),
    });
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn is_https(value: &str) -> bool {
    value.starts_with("https://") && value.len() > "https://".len()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_split(
    metadata: &Value,
    path: &str,
    issues: &mut Vec<DatasetAdmissionIssue>,
) {
    let train: BTreeSet<String> = metadata
        .get("train_group_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let test: BTreeSet<String> = metadata
        .get("test_group_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();

    for group in train.intersection(&test) {
        issue(
            issues,
            path,
            format!("train/test leakage: group {group:?} appears in both splits"),
        );
    }
}

fn validate_admitted_record(
    dataset: &Value,
    dataset_id: &str,
    requested_use: &DatasetUse,
    issues: &mut Vec<DatasetAdmissionIssue>,
) {
    let base = format!("$.datasets[{dataset_id}]");
    let source_url = string_at(dataset, "source_url").unwrap_or_default();
    if !is_https(source_url) {
        issue(issues, format!("{base}.source_url"), "an admitted dataset requires an HTTPS source URL");
    }

    let null = Value::Null;
    let snapshot = dataset.get("snapshot").unwrap_or(&null);
    if snapshot.get("immutable").and_then(Value::as_bool) != Some(true) {
        issue(
            issues,
            format!("{base}.snapshot.immutable"),
            "admission requires an immutable versioned snapshot",
        );
    }
    if string_at(snapshot, "release_tag").is_none_or(str::is_empty) {
        issue(
            issues,
            format!("{base}.snapshot.release_tag"),
            "admission requires a nonempty immutable release identifier",
        );
    }
    let hashes = snapshot.get("file_sha256").and_then(Value::as_array);
    match hashes {
        Some(values) if !values.is_empty() => {
            for (index, value) in values.iter().enumerate() {
                if !value.as_str().is_some_and(is_sha256) {
                    issue(
                        issues,
                        format!("{base}.snapshot.file_sha256[{index}]"),
                        "each admitted payload file requires a 64-character SHA-256 digest",
                    );
                }
            }
        }
        _ => issue(
            issues,
            format!("{base}.snapshot.file_sha256"),
            "admission requires at least one payload file SHA-256 digest",
        ),
    }

    let rights = dataset.get("rights").unwrap_or(&null);
    if string_at(rights, "status") != Some("explicit") {
        issue(
            issues,
            format!("{base}.rights.status"),
            "a paper-only or unknown licence statement cannot admit dataset payloads",
        );
    }
    if string_at(rights, "scope") != Some("dataset_payload") {
        issue(
            issues,
            format!("{base}.rights.scope"),
            "licence evidence must explicitly cover the dataset payload, not just code or article text",
        );
    }
    if string_at(rights, "license_id").is_none_or(str::is_empty) {
        issue(
            issues,
            format!("{base}.rights.license_id"),
            "an explicit dataset payload licence identifier is required",
        );
    }
    if !string_at(rights, "evidence_url").is_some_and(is_https) {
        issue(
            issues,
            format!("{base}.rights.evidence_url"),
            "admission requires an HTTPS rights-evidence reference",
        );
    }
    if !requested_use.permitted_by_rights(rights) {
        let right_name = requested_use.as_str();
        issue(
            issues,
            format!("{base}.rights"),
            format!("licence does not explicitly permit requested use: {right_name}"),
        );
    }
    let scope = string_at(dataset, "admission_scope").unwrap_or("none");
    if !requested_use.admitted_by_scope(scope) {
        issue(
            issues,
            format!("{base}.admission_scope"),
            format!("dataset admission scope does not permit requested use: {}", requested_use.as_str()),
        );
    }

    let metadata = dataset.get("metadata").unwrap_or(&null);
    for (field, explanation) in [
        ("time_basis_status", "time-basis status must be explicitly qualified or explicitly unqualified"),
        ("spatial_reference_status", "spatial-reference status must be explicitly qualified or explicitly unqualified"),
        ("calibration_status", "calibration status must be complete, partial or not provided"),
    ] {
        if string_at(metadata, field) == Some("unknown") {
            issue(
                issues,
                format!("{base}.metadata.{field}"),
                explanation,
            );
        }
    }
    validate_split(metadata, &format!("{base}.metadata"), issues);
    let split_basis = string_at(metadata, "split_basis").unwrap_or("none");
    let train_count = metadata
        .get("train_group_ids")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let test_count = metadata
        .get("test_group_ids")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if split_basis == "none" || train_count == 0 || test_count == 0 {
        issue(
            issues,
            format!("{base}.metadata"),
            "admission requires nonempty train and test groups with a declared split basis",
        );
    }
    if !string_at(metadata, "split_sha256").is_some_and(is_sha256) {
        issue(
            issues,
            format!("{base}.metadata.split_sha256"),
            "admission requires a SHA-256 commitment to the frozen split manifest",
        );
    }
    if matches!(
        string_at(metadata, "ground_truth_status"),
        Some("unknown" | "unavailable") | None
    ) {
        issue(
            issues,
            format!("{base}.metadata.ground_truth_status"),
            "ground-truth availability/quality must be explicitly characterized for admitted benchmark use",
        );
    }
}

pub fn validate_dataset_admission(
    schema: &Value,
    manifest: &Value,
    requested_use: &str,
) -> Result<DatasetAdmissionSummary, Vec<DatasetAdmissionIssue>> {
    let requested_use = DatasetUse::parse(requested_use)?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| {
            vec![DatasetAdmissionIssue {
                path: "$schema".to_owned(),
                message: format!("invalid Draft 2020-12 dataset-admission schema: {error}"),
            }]
        })?;

    let mut errors = validator
        .iter_errors(manifest)
        .map(|error| DatasetAdmissionIssue {
            path: format!("{}{}", "$", error.instance_path()),
            message: error.to_string(),
        })
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        return Err(errors);
    }

    let datasets = manifest
        .get("datasets")
        .and_then(Value::as_array)
        .expect("schema validated datasets array");
    let mut ids = BTreeSet::new();
    let mut admitted_count = 0;
    let mut candidate_count = 0;
    let mut quarantined_count = 0;
    let mut unqualified_time_count = 0;
    let mut unknown_time_count = 0;
    let mut unqualified_frame_count = 0;
    let mut unknown_frame_count = 0;
    let mut clock_state_truth_not_provided_count = 0;
    let mut clock_state_truth_unknown_count = 0;
    let mut partial_ground_truth_count = 0;

    for (index, dataset) in datasets.iter().enumerate() {
        let dataset_id = string_at(dataset, "dataset_id").unwrap_or_default();
        let base = format!("$.datasets[{index}]");
        if !ids.insert(dataset_id.to_owned()) {
            issue(
                &mut errors,
                format!("{base}.dataset_id"),
                format!("duplicate dataset ID: {dataset_id}"),
            );
        }

        let metadata = dataset.get("metadata").expect("schema validated metadata");
        match string_at(metadata, "time_basis_status") {
            Some("unqualified") => unqualified_time_count += 1,
            Some("unknown") => unknown_time_count += 1,
            _ => {}
        }
        match string_at(metadata, "spatial_reference_status") {
            Some("unqualified") => unqualified_frame_count += 1,
            Some("unknown") => unknown_frame_count += 1,
            _ => {}
        }
        match string_at(metadata, "clock_state_truth_status") {
            Some("not_provided") => clock_state_truth_not_provided_count += 1,
            Some("unknown") => clock_state_truth_unknown_count += 1,
            _ => {}
        }
        if string_at(metadata, "ground_truth_status") == Some("partial") {
            partial_ground_truth_count += 1;
        }

        match string_at(dataset, "status") {
            Some("candidate_not_admitted") => {
                candidate_count += 1;
                if string_at(dataset, "admission_scope") != Some("none") {
                    issue(
                        &mut errors,
                        format!("{base}.admission_scope"),
                        "candidate records must not claim an admission scope",
                    );
                }
            }
            Some("quarantined") => {
                quarantined_count += 1;
                if string_at(dataset, "admission_scope") != Some("none") {
                    issue(
                        &mut errors,
                        format!("{base}.admission_scope"),
                        "quarantined datasets cannot have an admission scope",
                    );
                }
            }
            Some("admitted") => {
                admitted_count += 1;
                validate_admitted_record(dataset, dataset_id, &requested_use, &mut errors);
            }
            _ => issue(&mut errors, format!("{base}.status"), "unsupported dataset admission status"),
        }
    }


    let expected_manifest_status = if admitted_count == 0 {
        "rights_and_snapshot_candidates_only"
    } else if admitted_count == datasets.len() {
        "admitted"
    } else {
        "partially_admitted"
    };
    if string_at(manifest, "manifest_status") != Some(expected_manifest_status) {
        issue(
            &mut errors,
            "$.manifest_status",
            format!(
                "manifest status must be {expected_manifest_status:?} for the record statuses"
            ),
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(DatasetAdmissionSummary {
        requested_use: requested_use.as_str().to_owned(),
        record_count: datasets.len(),
        admitted_count,
        candidate_count,
        quarantined_count,
        unqualified_time_count,
        unknown_time_count,
        unqualified_frame_count,
        unknown_frame_count,
        clock_state_truth_not_provided_count,
        clock_state_truth_unknown_count,
        partial_ground_truth_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/dataset-admission.schema.json"
        ))
        .unwrap()
    }

    fn candidates() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/datasets.v0.1.json"
        ))
        .unwrap()
    }

    fn hash() -> String {
        "a".repeat(64)
    }

    fn admit_first_dataset(manifest: &mut Value) {
        manifest["manifest_status"] = json!("partially_admitted");
        let record = &mut manifest["datasets"][0];
        record["status"] = json!("admitted");
        record["admission_scope"] = json!("both");
        record["snapshot"]["immutable"] = json!(true);
        record["snapshot"]["release_tag"] = json!("v1.0-frozen");
        record["snapshot"]["file_sha256"] = json!([hash(), "b".repeat(64)]);
        record["metadata"]["time_basis_status"] = json!("unqualified");
        record["metadata"]["spatial_reference_status"] = json!("unqualified");
        record["metadata"]["split_basis"] = json!("experiment_id");
        record["metadata"]["train_group_ids"] = json!(["experiment-01", "experiment-02"]);
        record["metadata"]["test_group_ids"] = json!(["experiment-03"]);
        record["metadata"]["split_sha256"] = json!(hash());
    }

    #[test]
    fn candidate_registry_passes_but_does_not_admit_any_payload() {
        let summary =
            validate_dataset_admission(&schema(), &candidates(), "research_noncommercial")
                .unwrap();
        assert_eq!(summary.record_count, 6);
        assert_eq!(summary.candidate_count, 6);
        assert_eq!(summary.admitted_count, 0);
        assert_eq!(summary.quarantined_count, 0);
        assert!(summary.unknown_time_count >= 3);
        assert!(summary.unknown_frame_count >= 3);
        assert_eq!(summary.partial_ground_truth_count, 3);
        assert_eq!(summary.clock_state_truth_not_provided_count, 1);
        assert_eq!(summary.clock_state_truth_unknown_count, 5);
    }

    #[test]
    fn hk_urbanspoof_remains_rights_blocked_and_not_admitted() {
        let manifest = candidates();
        let record = manifest["datasets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["dataset_id"] == "hk-urbanspoof-2026")
            .unwrap();
        assert_eq!(record["status"], "candidate_not_admitted");
        assert_eq!(record["admission_scope"], "none");
        assert_eq!(record["rights"]["status"], "unknown");
        assert_eq!(record["rights"]["research_use"], "unknown");
        assert_eq!(record["rights"]["commercial_use"], "unknown");
        assert_eq!(record["rights"]["redistribution"], "unknown");
        assert_eq!(record["snapshot"]["repository_commit"], "95def2b6a42eee6299bbc1a5b880a9f3bfc070d6");

        let summary =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial")
                .unwrap();
        assert_eq!(summary.record_count, 6);
        assert_eq!(summary.candidate_count, 6);
        assert_eq!(summary.admitted_count, 0);
        assert_eq!(summary.partial_ground_truth_count, 3);
    }

    #[test]
    fn urbannav_rights_are_explicit_but_payload_snapshot_is_not_admitted() {
        let manifest = candidates();
        let record = manifest["datasets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["dataset_id"] == "urbannav-hk-medium-urban-1")
            .unwrap();
        assert_eq!(record["status"], "candidate_not_admitted");
        assert_eq!(record["admission_scope"], "none");
        assert_eq!(record["rights"]["license_id"], "MIT");
        assert_eq!(record["rights"]["status"], "explicit");
        assert_eq!(record["rights"]["scope"], "dataset_payload");
        assert_eq!(record["rights"]["research_use"], "permitted");
        assert_eq!(record["rights"]["commercial_use"], "permitted");
        assert_eq!(record["rights"]["redistribution"], "permitted");
        assert_eq!(record["snapshot"]["immutable"], false);
        assert_eq!(record["snapshot"]["repository_commit"], "e53717a40c94803cd479a140433f6d803b1a0876");
        assert_eq!(record["snapshot"]["file_sha256"].as_array().unwrap().len(), 0);

        let summary = validate_dataset_admission(
            &schema(),
            &manifest,
            "research_noncommercial",
        )
        .unwrap();
        assert_eq!(summary.record_count, 6);
        assert_eq!(summary.admitted_count, 0);
        assert_eq!(summary.candidate_count, 6);
    }

    #[test]
    fn complete_cc_by_payload_can_be_admitted_for_declared_use() {
        let mut manifest = candidates();
        admit_first_dataset(&mut manifest);
        let summary =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap();
        assert_eq!(summary.admitted_count, 1);
        assert_eq!(summary.candidate_count, 5);
    }

    #[test]
    fn noncommercial_only_dataset_cannot_enter_commercial_use_path() {
        let mut manifest = candidates();
        manifest["manifest_status"] = json!("partially_admitted");
        let record = &mut manifest["datasets"][2];
        record["status"] = json!("admitted");
        record["admission_scope"] = json!("research_noncommercial");
        record["snapshot"]["immutable"] = json!(true);
        record["snapshot"]["release_tag"] = json!("tank-release-frozen");
        record["snapshot"]["file_sha256"] = json!([hash()]);
        record["metadata"]["train_group_ids"] = json!(["sequence-01"]);
        record["metadata"]["test_group_ids"] = json!(["sequence-02"]);
        record["metadata"]["split_sha256"] = json!(hash());

        let errors = validate_dataset_admission(&schema(), &manifest, "commercial").unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("does not explicitly permit requested use")));
    }

    #[test]
    fn article_or_code_license_is_not_enough_to_admit_dataset_payload() {
        let mut manifest = candidates();
        admit_first_dataset(&mut manifest);
        manifest["datasets"][0]["rights"]["scope"] = json!("article_text_only");
        let errors =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap_err();
        assert!(errors.iter().any(|error| error.path.ends_with(".rights.scope")));
    }

    #[test]
    fn admitted_dataset_requires_immutable_snapshot_and_file_hashes() {
        let mut manifest = candidates();
        admit_first_dataset(&mut manifest);
        manifest["datasets"][0]["snapshot"]["file_sha256"] = json!([]);
        let errors =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap_err();
        assert!(errors.iter().any(|error| error.path.ends_with(".snapshot.file_sha256")));
    }

    #[test]
    fn overlapping_train_and_test_groups_are_rejected() {
        let mut manifest = candidates();
        admit_first_dataset(&mut manifest);
        manifest["datasets"][0]["metadata"]["test_group_ids"] =
            json!(["experiment-02", "experiment-03"]);
        let errors =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("train/test leakage")));
    }

    #[test]
    fn duplicate_dataset_ids_are_rejected() {
        let mut manifest = candidates();
        manifest["datasets"][1]["dataset_id"] = json!("miluv-figshare-v1");
        let errors =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("duplicate dataset ID")));
    }


    #[test]
    fn unknown_time_frame_or_calibration_state_blocks_admission() {
        for (field, expected_path) in [
            ("time_basis_status", ".metadata.time_basis_status"),
            ("spatial_reference_status", ".metadata.spatial_reference_status"),
            ("calibration_status", ".metadata.calibration_status"),
        ] {
            let mut manifest = candidates();
            admit_first_dataset(&mut manifest);
            manifest["datasets"][0]["metadata"][field] = json!("unknown");
            let errors = validate_dataset_admission(
                &schema(),
                &manifest,
                "research_noncommercial",
            )
            .unwrap_err();
            assert!(errors.iter().any(|error| error.path.ends_with(expected_path)));
        }
    }

    #[test]
    fn unqualified_source_time_remains_a_visible_quality_limitation() {
        let mut manifest = candidates();
        admit_first_dataset(&mut manifest);
        manifest["datasets"][0]["metadata"]["time_basis_status"] = json!("unqualified");
        let summary =
            validate_dataset_admission(&schema(), &manifest, "research_noncommercial").unwrap();
        assert_eq!(summary.admitted_count, 1);
        assert_eq!(summary.unqualified_time_count, 1);
    }
}
