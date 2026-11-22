//! Fail-closed split-manifest validation for cross-domain navigation benchmarks.
//!
//! Random sample splits often leak adjacent windows, the same physical event,
//! environment, device, or failure domain across training and evaluation.
//! This validator checks declared metadata disjointness. It cannot prove that
//! derived features, raw artifacts, labels or hidden infrastructure are free
//! from leakage; those require independent provenance review.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Partition {
    Train,
    Validation,
    Test,
}

impl Partition {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "train" => Some(Self::Train),
            "validation" => Some(Self::Validation),
            "test" => Some(Self::Test),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HoldoutAxis {
    None,
    Dataset,
    Environment,
    Device,
    Firmware,
    EventClass,
    SourceFailureDomain,
}

impl HoldoutAxis {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "dataset_id" => Some(Self::Dataset),
            "environment_id" => Some(Self::Environment),
            "device_id" => Some(Self::Device),
            "firmware_id" => Some(Self::Firmware),
            "event_class" => Some(Self::EventClass),
            "source_failure_domain_id" => Some(Self::SourceFailureDomain),
            _ => None,
        }
    }

    fn expected_protocol(self) -> &'static str {
        match self {
            Self::None => "grouped_in_domain",
            Self::Dataset => "leave_one_dataset_out",
            Self::Environment => "leave_one_environment_out",
            Self::Device => "leave_one_device_out",
            Self::Firmware => "leave_one_firmware_out",
            Self::EventClass => "leave_one_event_class_out",
            Self::SourceFailureDomain => "leave_one_failure_domain_out",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitValidationIssue {
    pub path: String,
    pub message: String,
}

impl fmt::Display for SplitValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SplitValidationSummary {
    pub fold_id: String,
    pub protocol: String,
    pub holdout_axis: String,
    pub held_out_value: Option<String>,
    pub record_count: usize,
    pub train_count: usize,
    pub validation_count: usize,
    pub test_count: usize,
    pub sequence_group_count: usize,
    pub event_group_count: usize,
    pub failure_domain_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SplitRecord {
    record_id: String,
    dataset_id: String,
    sequence_group_id: String,
    event_id: String,
    environment_id: String,
    device_id: String,
    firmware_id: String,
    event_class: String,
    failure_domain_ids: Vec<String>,
    partition: Partition,
}

impl SplitRecord {
    fn axis_value(&self, axis: HoldoutAxis) -> Option<&str> {
        match axis {
            HoldoutAxis::None => None,
            HoldoutAxis::Dataset => Some(&self.dataset_id),
            HoldoutAxis::Environment => Some(&self.environment_id),
            HoldoutAxis::Device => Some(&self.device_id),
            HoldoutAxis::Firmware => Some(&self.firmware_id),
            HoldoutAxis::EventClass => Some(&self.event_class),
            HoldoutAxis::SourceFailureDomain => None,
        }
    }
}

fn push_issue(
    issues: &mut Vec<SplitValidationIssue>,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(SplitValidationIssue {
        path: path.into(),
        message: message.into(),
    });
}

fn required_string(
    object: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<SplitValidationIssue>,
) -> Option<String> {
    let field_path = format!("{path}.{key}");
    match object.get(key).and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value.to_owned()),
        Some(_) => {
            push_issue(issues, field_path, "must not be empty");
            None
        }
        None => {
            push_issue(issues, field_path, "must be a non-empty string");
            None
        }
    }
}

fn required_string_array(
    object: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<SplitValidationIssue>,
) -> Option<Vec<String>> {
    let field_path = format!("{path}.{key}");
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        push_issue(issues, field_path, "must be an array of non-empty strings");
        return None;
    };
    if values.is_empty() {
        push_issue(issues, field_path, "must not be empty");
        return None;
    }
    let mut strings = Vec::with_capacity(values.len());
    let mut unique = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        match value.as_str() {
            Some(text) if !text.trim().is_empty() => {
                if !unique.insert(text.to_owned()) {
                    push_issue(
                        issues,
                        format!("{field_path}[{index}]"),
                        format!("duplicate failure-domain ID {text}"),
                    );
                }
                strings.push(text.to_owned());
            }
            _ => push_issue(
                issues,
                format!("{field_path}[{index}]"),
                "must be a non-empty string",
            ),
        }
    }
    (!strings.is_empty()).then_some(strings)
}

fn parse_record(
    value: &Value,
    index: usize,
    issues: &mut Vec<SplitValidationIssue>,
) -> Option<SplitRecord> {
    let path = format!("$.records[{index}]");
    if !value.is_object() {
        push_issue(issues, path, "must be an object");
        return None;
    }

    let record_id = required_string(value, "record_id", &path, issues);
    let dataset_id = required_string(value, "dataset_id", &path, issues);
    let sequence_group_id = required_string(value, "sequence_group_id", &path, issues);
    let event_id = required_string(value, "event_id", &path, issues);
    let environment_id = required_string(value, "environment_id", &path, issues);
    let device_id = required_string(value, "device_id", &path, issues);
    let firmware_id = required_string(value, "firmware_id", &path, issues);
    let event_class = required_string(value, "event_class", &path, issues);
    let failure_domain_ids =
        required_string_array(value, "failure_domain_ids", &path, issues);
    let split = required_string(value, "split", &path, issues)
        .and_then(|value| {
            Partition::parse(&value).or_else(|| {
                push_issue(
                    issues,
                    format!("{path}.split"),
                    "must be one of train, validation, test",
                );
                None
            })
        });

    Some(SplitRecord {
        record_id: record_id?,
        dataset_id: dataset_id?,
        sequence_group_id: sequence_group_id?,
        event_id: event_id?,
        environment_id: environment_id?,
        device_id: device_id?,
        firmware_id: firmware_id?,
        event_class: event_class?,
        failure_domain_ids: failure_domain_ids?,
        partition: split?,
    })
}

/// Validate one split manifest.
///
/// JSON shape:
///
/// {
///   "schema_version": "nav-generalization-split/v0.1",
///   "fold_id": "leave-site-03-out",
///   "protocol": "leave_one_environment_out",
///   "holdout_axis": "environment_id",
///   "held_out_value": "site-03",
///   "records": [
///     {
///       "record_id": "rec-001",
///       "dataset_id": "dataset-a",
///       "sequence_group_id": "sequence-010",
///       "event_id": "event-02",
///       "environment_id": "site-03",
///       "device_id": "receiver-4",
///       "firmware_id": "fw-1.2",
///       "event_class": "multipath-transition",
///       "failure_domain_ids": ["site-03-power", "receiver-4-clock"],
///       "split": "test"
///     }
///   ]
/// }
///
/// Event and sequence groups must never cross train/validation/test.
/// For leave-one-* protocols, the held-out axis value must be present only
/// in test. A single manifest describes one fold and one held-out value.
pub fn validate_split_manifest(
    manifest: &Value,
) -> Result<SplitValidationSummary, Vec<SplitValidationIssue>> {
    let mut issues = Vec::new();
    if !manifest.is_object() {
        return Err(vec![SplitValidationIssue {
            path: "$".to_owned(),
            message: "manifest must be an object".to_owned(),
        }]);
    }

    let schema_version =
        required_string(manifest, "schema_version", "$", &mut issues);
    if schema_version.as_deref() != Some("nav-generalization-split/v0.1") {
        push_issue(
            &mut issues,
            "$.schema_version",
            "unsupported split-manifest schema version",
        );
    }
    let fold_id = required_string(manifest, "fold_id", "$", &mut issues);
    let protocol = required_string(manifest, "protocol", "$", &mut issues);
    let axis_text = required_string(manifest, "holdout_axis", "$", &mut issues);
    let held_out_value = match manifest.get("held_out_value") {
        Some(Value::Null) => None,
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        _ => {
            push_issue(
                &mut issues,
                "$.held_out_value",
                "must be null for grouped_in_domain or a non-empty string for holdout protocols",
            );
            None
        }
    };
    let axis = axis_text.as_deref().and_then(|value| {
        HoldoutAxis::parse(value).or_else(|| {
            push_issue(
                &mut issues,
                "$.holdout_axis",
                "unsupported holdout axis",
            );
            None
        })
    });

    if let (Some(protocol), Some(axis)) = (protocol.as_deref(), axis) {
        if protocol != axis.expected_protocol() {
            push_issue(
                &mut issues,
                "$.protocol",
                format!(
                    "protocol {protocol} does not match holdout axis {}",
                    axis.expected_protocol()
                ),
            );
        }
        match (axis, held_out_value.as_deref()) {
            (HoldoutAxis::None, None) => {}
            (HoldoutAxis::None, Some(_)) => push_issue(
                &mut issues,
                "$.held_out_value",
                "grouped_in_domain must not declare a held-out value",
            ),
            (_, Some(_)) => {}
            (_, None) => push_issue(
                &mut issues,
                "$.held_out_value",
                "holdout protocols require an explicit held-out value",
            ),
        }
    }

    let record_values = match manifest.get("records").and_then(Value::as_array) {
        Some(records) if !records.is_empty() => records,
        Some(_) => {
            push_issue(&mut issues, "$.records", "must not be empty");
            &Vec::new()
        }
        None => {
            push_issue(&mut issues, "$.records", "must be a non-empty array");
            &Vec::new()
        }
    };

    let mut records = Vec::with_capacity(record_values.len());
    for (index, value) in record_values.iter().enumerate() {
        if let Some(record) = parse_record(value, index, &mut issues) {
            records.push(record);
        }
    }

    let mut record_ids = BTreeSet::new();
    for (index, record) in records.iter().enumerate() {
        if !record_ids.insert(record.record_id.clone()) {
            push_issue(
                &mut issues,
                format!("$.records[{index}].record_id"),
                format!("duplicate record ID {}", record.record_id),
            );
        }
    }

    let mut partition_counts = BTreeMap::<Partition, usize>::new();
    for record in &records {
        *partition_counts.entry(record.partition).or_default() += 1;
    }
    for partition in [Partition::Train, Partition::Validation, Partition::Test] {
        if partition_counts.get(&partition).copied().unwrap_or(0) == 0 {
            push_issue(
                &mut issues,
                "$.records",
                format!("partition {partition:?} must contain at least one record"),
            );
        }
    }

    // Adjacent windows and repeated physical events may not cross any split.
    let mut sequence_owners = BTreeMap::<String, Partition>::new();
    let mut event_owners = BTreeMap::<String, Partition>::new();
    for (index, record) in records.iter().enumerate() {
        for (key_name, key, owners) in [
            (
                "sequence_group_id",
                record.sequence_group_id.as_str(),
                &mut sequence_owners,
            ),
            ("event_id", record.event_id.as_str(), &mut event_owners),
        ] {
            if let Some(previous) = owners.get(key) {
                if *previous != record.partition {
                    push_issue(
                        &mut issues,
                        format!("$.records[{index}].{key_name}"),
                        format!("{key_name} {key} appears in more than one partition"),
                    );
                }
            } else {
                owners.insert(key.to_owned(), record.partition);
            }
        }
    }

    // Holdout axis is checked on the actual records, not inferred from the
    // fold name. No unknown/missing metadata is upgraded into independence.
    if let Some(axis) = axis {
        if axis != HoldoutAxis::None {
            let held = held_out_value.as_deref();
            let mut test_values = BTreeSet::new();
            for (index, record) in records.iter().enumerate() {
                let is_test = record.partition == Partition::Test;
                if axis == HoldoutAxis::SourceFailureDomain {
                    let includes_held = held.is_some_and(|held| {
                        record.failure_domain_ids.iter().any(|value| value == held)
                    });
                    if is_test && !includes_held {
                        push_issue(
                            &mut issues,
                            format!("$.records[{index}].failure_domain_ids"),
                            "test record does not declare the held-out failure domain",
                        );
                    }
                    if !is_test && includes_held {
                        push_issue(
                            &mut issues,
                            format!("$.records[{index}].failure_domain_ids"),
                            "training/validation record contains the held-out failure domain",
                        );
                    }
                } else if let Some(value) = record.axis_value(axis) {
                    if is_test {
                        test_values.insert(value.to_owned());
                        if Some(value) != held {
                            push_issue(
                                &mut issues,
                                format!("$.records[{index}]"),
                                "test record does not match the declared held-out value",
                            );
                        }
                    } else if Some(value) == held {
                        push_issue(
                            &mut issues,
                            format!("$.records[{index}]"),
                            "training/validation record contains the held-out value",
                        );
                    }
                }
            }
            if axis != HoldoutAxis::SourceFailureDomain
                && test_values.len() != 1
            {
                push_issue(
                    &mut issues,
                    "$.records",
                    "test partition must contain exactly one held-out-axis value",
                );
            }
        }
    }

    // For leave-one-failure-domain-out, explicitly check intersection as well
    // as the held-out ID. Other source failure domains may be shared only if
    // they are declared independent of the selected holdout axis; their graph
    // remains visible for a separate common-cause audit.
    if axis == Some(HoldoutAxis::SourceFailureDomain) {
        let test_domains: BTreeSet<&str> = records
            .iter()
            .filter(|record| record.partition == Partition::Test)
            .flat_map(|record| record.failure_domain_ids.iter().map(String::as_str))
            .collect();
        let development_domains: BTreeSet<&str> = records
            .iter()
            .filter(|record| record.partition != Partition::Test)
            .flat_map(|record| record.failure_domain_ids.iter().map(String::as_str))
            .collect();
        for shared in test_domains.intersection(&development_domains) {
            if Some(*shared) != held_out_value.as_deref() {
                push_issue(
                    &mut issues,
                    "$.records",
                    format!("non-held-out failure domain {shared} appears in development and test partitions"),
                );
            }
        }
    }

    issues.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.message.cmp(&b.message)));
    if !issues.is_empty() {
        return Err(issues);
    }

    let axis_text = axis_text.expect("axis field parsed after issue-free validation");
    let protocol = protocol.expect("protocol field parsed after issue-free validation");
    Ok(SplitValidationSummary {
        fold_id: fold_id.expect("fold ID parsed after issue-free validation"),
        protocol,
        holdout_axis: axis_text,
        held_out_value,
        record_count: records.len(),
        train_count: partition_counts.get(&Partition::Train).copied().unwrap_or(0),
        validation_count: partition_counts.get(&Partition::Validation).copied().unwrap_or(0),
        test_count: partition_counts.get(&Partition::Test).copied().unwrap_or(0),
        sequence_group_count: records
            .iter()
            .map(|record| record.sequence_group_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        event_group_count: records
            .iter()
            .map(|record| record.event_id.as_str())
            .collect::<BTreeSet<_>>()
            .len(),
        failure_domain_count: records
            .iter()
            .flat_map(|record| record.failure_domain_ids.iter().map(String::as_str))
            .collect::<BTreeSet<_>>()
            .len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn record(
        id: &str,
        split: &str,
        environment: &str,
        sequence: &str,
        event: &str,
    ) -> Value {
        json!({
            "record_id": id,
            "dataset_id": "dataset-a",
            "sequence_group_id": sequence,
            "event_id": event,
            "environment_id": environment,
            "device_id": "device-a",
            "firmware_id": "fw-1",
            "event_class": "multipath",
            "failure_domain_ids": [format!("{environment}-power"), "device-a-clock"],
            "split": split
        })
    }

    fn environment_manifest() -> Value {
        json!({
            "schema_version": "nav-generalization-split/v0.1",
            "fold_id": "site-b-held-out",
            "protocol": "leave_one_environment_out",
            "holdout_axis": "environment_id",
            "held_out_value": "site-b",
            "records": [
                record("tr-1", "train", "site-a", "seq-a1", "event-a1"),
                record("va-1", "validation", "site-a", "seq-a2", "event-a2"),
                record("te-1", "test", "site-b", "seq-b1", "event-b1"),
                record("te-2", "test", "site-b", "seq-b2", "event-b2")
            ]
        })
    }

    #[test]
    fn valid_leave_one_environment_out_manifest_passes() {
        let result = validate_split_manifest(&environment_manifest()).unwrap();
        assert_eq!(result.record_count, 4);
        assert_eq!(result.train_count, 1);
        assert_eq!(result.validation_count, 1);
        assert_eq!(result.test_count, 2);
        assert_eq!(result.held_out_value.as_deref(), Some("site-b"));
    }

    #[test]
    fn held_out_environment_in_training_fails_closed() {
        let mut manifest = environment_manifest();
        manifest["records"][0]["environment_id"] = json!("site-b");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("training/validation record contains the held-out value")
        }));
    }

    #[test]
    fn sequence_windows_cannot_cross_train_and_test() {
        let mut manifest = environment_manifest();
        manifest["records"][2]["sequence_group_id"] = json!("seq-a1");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("sequence_group_id")));
    }

    #[test]
    fn physical_event_identity_cannot_cross_partitions() {
        let mut manifest = environment_manifest();
        manifest["records"][3]["event_id"] = json!("event-a1");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("event_id")));
    }

    #[test]
    fn unknown_domain_metadata_does_not_become_independence() {
        let mut manifest = environment_manifest();
        manifest["records"][0]["device_id"] = json!("");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.path.ends_with(".device_id")));
    }

    #[test]
    fn leave_one_failure_domain_out_checks_cross_split_intersection() {
        let mut manifest = environment_manifest();
        manifest["protocol"] = json!("leave_one_failure_domain_out");
        manifest["holdout_axis"] = json!("source_failure_domain_id");
        manifest["held_out_value"] = json!("site-b-power");
        manifest["records"][2]["failure_domain_ids"] = json!(["site-b-power", "test-clock"]);
        manifest["records"][3]["failure_domain_ids"] = json!(["site-b-power", "other-test-domain"]);
        manifest["records"][0]["failure_domain_ids"] = json!(["site-a-power", "train-clock"]);
        manifest["records"][1]["failure_domain_ids"] = json!(["site-a-power", "train-clock"]);
        let summary = validate_split_manifest(&manifest).unwrap();
        assert_eq!(summary.held_out_value.as_deref(), Some("site-b-power"));

        manifest["records"][0]["failure_domain_ids"] = json!(["site-b-power", "train-clock"]);
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("training/validation record contains the held-out failure domain")
        }));
    }

    #[test]
    fn all_partitions_are_required() {
        let mut manifest = environment_manifest();
        manifest["records"].as_array_mut().unwrap().retain(|record| {
            record["split"] != json!("validation")
        });
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("validation")));
    }

    #[test]
    fn input_permutation_preserves_summary_and_ordered_errors() {
        let mut forward = environment_manifest();
        let mut reverse = environment_manifest();
        reverse["records"].as_array_mut().unwrap().reverse();
        assert_eq!(
            validate_split_manifest(&forward).unwrap(),
            validate_split_manifest(&reverse).unwrap()
        );

        forward["records"][0]["sequence_group_id"] = json!("seq-b1");
        reverse["records"][3]["sequence_group_id"] = json!("seq-a1");
        let forward_errors = validate_split_manifest(&forward).unwrap_err();
        let reverse_errors = validate_split_manifest(&reverse).unwrap_err();
        let forward_messages: Vec<String> =
            forward_errors.into_iter().map(|error| error.message).collect();
        let reverse_messages: Vec<String> =
            reverse_errors.into_iter().map(|error| error.message).collect();
        assert_eq!(forward_messages, reverse_messages);
    }

    #[test]
    fn test_records_must_match_exactly_one_holdout_value() {
        let mut manifest = environment_manifest();
        manifest["records"][3]["environment_id"] = json!("site-c");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| {
            error.message.contains("test record does not match the declared held-out value")
        }));
    }

    #[test]
    fn duplicate_record_ids_are_rejected() {
        let mut manifest = environment_manifest();
        manifest["records"][3]["record_id"] = json!("te-1");
        let errors = validate_split_manifest(&manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("duplicate record ID")));
    }

    #[test]
    fn grouped_in_domain_protocol_keeps_event_and_sequence_groups_separate() {
        let mut manifest = environment_manifest();
        manifest["protocol"] = json!("grouped_in_domain");
        manifest["holdout_axis"] = json!("none");
        manifest["held_out_value"] = Value::Null;
        manifest["records"][2]["environment_id"] = json!("site-a");
        manifest["records"][3]["environment_id"] = json!("site-a");
        let summary = validate_split_manifest(&manifest).unwrap();
        assert_eq!(summary.protocol, "grouped_in_domain");
    }
}
