//! Leakage-resistant event-level validation for pneumatic conveying state datasets.
//!
//! The crate admits sensor/event evidence and grouped split manifests only. It
//! does not train or score a classifier, infer physical capacity, recommend
//! operating limits, control machinery, or certify a conveying line.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub channel_count: usize,
    pub group_count: usize,
    pub train_group_count: usize,
    pub calibration_group_count: usize,
    pub test_group_count: usize,
    pub heldout_dimensions: Vec<String>,
    pub window_count: usize,
    pub event_count: usize,
    pub critical_event_count: usize,
    pub missing_channel_observation_count: usize,
    pub degraded_channel_observation_count: usize,
    pub windows_with_non_observed_channels: usize,
    pub observed_state_labels: Vec<String>,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn uint(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn dimension_key(group: &Value, dimension: &str) -> Option<&str> {
    match dimension {
        "SITE" => text(group, "site_id"),
        "CAMPAIGN" => text(group, "campaign_id"),
        "PATH_CONFIGURATION" => text(group, "path_configuration_ref"),
        "MACHINE_CONFIGURATION" => text(group, "machine_configuration_ref"),
        "MATERIAL_COHORT" => text(group, "material_cohort_ref"),
        "TEMPORAL_BLOCK" => text(group, "temporal_block_ref"),
        "PARENT_ROCK_BLOCK" => text(group, "parent_rock_block_ref"),
        _ => None,
    }
}

fn is_restriction_state(label: &str) -> bool {
    matches!(
        label,
        "PARTIAL_RESTRICTION" | "DEVELOPING_BLOCKAGE" | "COMPLETE_BLOCKAGE"
    )
}

fn label_method_compatible(label: &str, method: &str) -> bool {
    match label {
        "NOMINAL_STABLE" => method == "WITNESSED_NORMAL_OPERATION",
        "STARTUP_SHUTDOWN_TRANSIENT" => {
            matches!(method, "WITNESSED_PHYSICAL_INSPECTION" | "WITNESSED_NORMAL_OPERATION")
        }
        "PARTIAL_RESTRICTION" | "DEVELOPING_BLOCKAGE" | "COMPLETE_BLOCKAGE" => matches!(
            method,
            "SEEDED_RESTRICTION_TEST" | "WITNESSED_PHYSICAL_INSPECTION" | "LINE_INSPECTION"
        ),
        "BLOWER_VFD_FAULT" => {
            matches!(method, "VFD_FAULT_DIAGNOSTIC" | "WITNESSED_PHYSICAL_INSPECTION")
        }
        "PRESSURE_SENSOR_FAULT" | "AIRFLOW_SENSOR_FAULT" => {
            method == "CALIBRATION_DIAGNOSTIC"
        }
        "UNKNOWN_OUT_OF_DOMAIN" => {
            matches!(method, "WITNESSED_PHYSICAL_INSPECTION" | "LINE_INSPECTION")
        }
        "HIGH_SOLIDS_LOADING"
        | "STRATIFIED_FLOW"
        | "DUNE_DEPOSITION"
        | "WATER_ACCUMULATION"
        | "FILTER_CYCLONE_LOADING"
        | "AIR_LEAK_DISCONNECTION" => matches!(
            method,
            "WITNESSED_PHYSICAL_INSPECTION"
                | "LINE_INSPECTION"
                | "RECOVERED_MASS_RECONCILIATION"
        ),
        _ => false,
    }
}

fn channel_kind_is(channel: &Value, kinds: &[&str]) -> bool {
    text(channel, "kind").is_some_and(|kind| kinds.contains(&kind))
}

/// Validate all semantic and leakage gates in a frozen event-dataset manifest.
pub fn validate_case(case: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(
            &mut errors,
            "UNSUPPORTED_SCHEMA_VERSION",
            "$.schema_version",
            "schema_version must be exactly 1",
        );
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(
            &mut errors,
            "INVALID_EVIDENCE_CLASS",
            "$.evidence_class",
            "evidence_class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED",
        );
    }

    let Some(context) = case.get("campaign_context") else {
        push(
            &mut errors,
            "MISSING_CAMPAIGN_CONTEXT",
            "$.campaign_context",
            "campaign context is required",
        );
        return errors;
    };
    for key in [
        "campaign_id",
        "clock_sync_protocol_ref",
        "label_protocol_ref",
        "dataset_source_ref",
    ] {
        if text(context, key).is_none() {
            push(
                &mut errors,
                "MISSING_CAMPAIGN_PROVENANCE",
                &format!("$.campaign_context.{key}"),
                "non-empty campaign and protocol provenance is required",
            );
        }
    }

    let Some(channels) = case.get("channels").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_CHANNEL_REGISTRY",
            "$.channels",
            "channels must be an array",
        );
        return errors;
    };
    if channels.is_empty() {
        push(
            &mut errors,
            "EMPTY_CHANNEL_REGISTRY",
            "$.channels",
            "at least one sensor channel is required",
        );
    }

    let mut channel_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, channel) in channels.iter().enumerate() {
        let path = format!("$.channels[{index}]");
        let Some(id) = text(channel, "channel_id") else {
            push(
                &mut errors,
                "MISSING_CHANNEL_ID",
                &format!("{path}.channel_id"),
                "channel_id is required",
            );
            continue;
        };
        if channel_by_id.insert(id.to_owned(), channel).is_some() {
            push(
                &mut errors,
                "DUPLICATE_CHANNEL_ID",
                &format!("{path}.channel_id"),
                "channel identifiers must be unique",
            );
        }

        for key in [
            "unit",
            "instrument_ref",
            "calibration_ref",
            "physical_location_ref",
            "source_artifact_ref",
            "clock_domain_ref",
        ] {
            if text(channel, key).is_none() {
                push(
                    &mut errors,
                    "MISSING_CHANNEL_PROVENANCE",
                    &format!("{path}.{key}"),
                    "sensor unit, instrument, calibration, physical location, source and clock domain are required",
                );
            }
        }

        if text(channel, "kind").is_none() {
            push(
                &mut errors,
                "MISSING_CHANNEL_KIND",
                &format!("{path}.kind"),
                "typed sensor channel kind is required",
            );
        }
        match finite(channel, "sample_rate_hz") {
            Some(rate) if rate > 0.0 => {}
            _ => push(
                &mut errors,
                "INVALID_CHANNEL_SAMPLE_RATE",
                &format!("{path}.sample_rate_hz"),
                "sample rate must be finite and positive",
            ),
        }
    }

    let Some(groups) = case.get("groups").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_GROUPS",
            "$.groups",
            "groups must be an array",
        );
        return errors;
    };
    if groups.len() < 3 {
        push(
            &mut errors,
            "INSUFFICIENT_GROUPS",
            "$.groups",
            "at least one train, calibration and test group are required",
        );
    }

    let mut group_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, group) in groups.iter().enumerate() {
        let path = format!("$.groups[{index}]");
        let Some(id) = text(group, "group_id") else {
            push(
                &mut errors,
                "MISSING_GROUP_ID",
                &format!("{path}.group_id"),
                "group_id is required",
            );
            continue;
        };
        if group_by_id.insert(id.to_owned(), group).is_some() {
            push(
                &mut errors,
                "DUPLICATE_GROUP_ID",
                &format!("{path}.group_id"),
                "group identifiers must be unique",
            );
        }

        for key in [
            "site_id",
            "campaign_id",
            "path_configuration_ref",
            "machine_configuration_ref",
            "material_cohort_ref",
            "temporal_block_ref",
            "parent_rock_block_ref",
        ] {
            if text(group, key).is_none() {
                push(
                    &mut errors,
                    "MISSING_GROUP_DIMENSION",
                    &format!("{path}.{key}"),
                    "every group needs explicit site/campaign/path/machine/material/time/rock-block identity",
                );
            }
        }
    }

    let Some(split) = case.get("split") else {
        push(&mut errors, "MISSING_SPLIT", "$.split", "split declaration is required");
        return errors;
    };
    let Some(heldout) = split.get("heldout_dimensions").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_HELDOUT_DIMENSIONS",
            "$.split.heldout_dimensions",
            "heldout_dimensions must be an array",
        );
        return errors;
    };
    if heldout.is_empty() {
        push(
            &mut errors,
            "EMPTY_HELDOUT_DIMENSIONS",
            "$.split.heldout_dimensions",
            "at least one complete-group holdout dimension is required",
        );
    }

    let mut dimensions = Vec::new();
    let allowed_dimensions = [
        "SITE",
        "CAMPAIGN",
        "PATH_CONFIGURATION",
        "MACHINE_CONFIGURATION",
        "MATERIAL_COHORT",
        "TEMPORAL_BLOCK",
        "PARENT_ROCK_BLOCK",
    ];
    let mut seen_dimensions = BTreeSet::new();
    for (index, dimension) in heldout.iter().enumerate() {
        let Some(name) = dimension.as_str() else {
            push(
                &mut errors,
                "INVALID_HELDOUT_DIMENSION",
                &format!("$.split.heldout_dimensions[{index}]"),
                "held-out dimension must be a string",
            );
            continue;
        };
        if !allowed_dimensions.contains(&name) {
            push(
                &mut errors,
                "UNKNOWN_HELDOUT_DIMENSION",
                &format!("$.split.heldout_dimensions[{index}]"),
                "held-out dimension is outside the v0.1 group vocabulary",
            );
        }
        if !seen_dimensions.insert(name.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_HELDOUT_DIMENSION",
                &format!("$.split.heldout_dimensions[{index}]"),
                "held-out dimensions must be unique",
            );
        }
        dimensions.push(name.to_owned());
    }

    let partition_fields = [
        ("train", "train_group_ids"),
        ("calibration", "calibration_group_ids"),
        ("test", "test_group_ids"),
    ];
    let mut partition_sets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut assigned_groups: BTreeMap<String, String> = BTreeMap::new();

    for (partition, field) in partition_fields {
        let path = format!("$.split.{field}");
        let Some(ids) = split.get(field).and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_PARTITION_GROUPS",
                &path,
                "train, calibration and test group lists are required",
            );
            continue;
        };
        if ids.is_empty() {
            push(
                &mut errors,
                "EMPTY_PARTITION",
                &path,
                "train, calibration and test partitions must each be non-empty",
            );
        }

        let mut set = BTreeSet::new();
        for (index, id_value) in ids.iter().enumerate() {
            let Some(id) = id_value.as_str() else {
                push(
                    &mut errors,
                    "INVALID_PARTITION_GROUP_ID",
                    &format!("{path}[{index}]"),
                    "partition group IDs must be strings",
                );
                continue;
            };
            if !group_by_id.contains_key(id) {
                push(
                    &mut errors,
                    "UNKNOWN_PARTITION_GROUP",
                    &format!("{path}[{index}]"),
                    "partition refers to an undeclared group",
                );
            }
            if !set.insert(id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_PARTITION_GROUP",
                    &format!("{path}[{index}]"),
                    "a group may appear only once in each partition",
                );
            }
            if let Some(previous) = assigned_groups.insert(id.to_owned(), partition.to_owned()) {
                push(
                    &mut errors,
                    "GROUP_SPLIT_LEAKAGE",
                    &format!("{path}[{index}]"),
                    format!("group already assigned to {previous}; groups cannot cross partitions"),
                );
            }
        }
        partition_sets.insert(partition.to_owned(), set);
    }

    for group_id in group_by_id.keys() {
        if !assigned_groups.contains_key(group_id) {
            push(
                &mut errors,
                "UNASSIGNED_GROUP",
                "$.split",
                format!("declared group {group_id} must belong to exactly one partition"),
            );
        }
    }

    for dimension in &dimensions {
        let mut values_by_partition: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (partition, group_ids) in &partition_sets {
            let mut values = BTreeSet::new();
            for group_id in group_ids {
                if let Some(group) = group_by_id.get(group_id) {
                    if let Some(value) = dimension_key(group, dimension) {
                        values.insert(value.to_owned());
                    }
                }
            }
            values_by_partition.insert(partition.clone(), values);
        }

        let names = ["train", "calibration", "test"];
        for left_index in 0..names.len() {
            for right_index in (left_index + 1)..names.len() {
                let left = values_by_partition
                    .get(names[left_index])
                    .cloned()
                    .unwrap_or_default();
                let right = values_by_partition
                    .get(names[right_index])
                    .cloned()
                    .unwrap_or_default();
                if left.iter().any(|value| right.contains(value)) {
                    push(
                        &mut errors,
                        "HELDOUT_DIMENSION_LEAKAGE",
                        "$.split.heldout_dimensions",
                        format!(
                            "dimension {dimension} overlaps between {} and {} partitions",
                            names[left_index], names[right_index]
                        ),
                    );
                }
            }
        }
    }

    let Some(windows) = case.get("windows").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_WINDOWS",
            "$.windows",
            "windows must be an array",
        );
        return errors;
    };
    let mut window_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    let mut windows_by_group: BTreeMap<String, Vec<(u64, u64, String)>> = BTreeMap::new();

    for (index, window) in windows.iter().enumerate() {
        let path = format!("$.windows[{index}]");
        let Some(id) = text(window, "window_id") else {
            push(
                &mut errors,
                "MISSING_WINDOW_ID",
                &format!("{path}.window_id"),
                "window_id is required",
            );
            continue;
        };
        if window_by_id.insert(id.to_owned(), window).is_some() {
            push(
                &mut errors,
                "DUPLICATE_WINDOW_ID",
                &format!("{path}.window_id"),
                "window identifiers must be unique",
            );
        }

        let group_id = text(window, "group_id");
        if group_id.is_none_or(|group| !group_by_id.contains_key(group)) {
            push(
                &mut errors,
                "UNKNOWN_WINDOW_GROUP",
                &format!("{path}.group_id"),
                "window must reference a declared group",
            );
        }

        let start = uint(window, "start_ms");
        let end = uint(window, "end_ms");
        match (start, end) {
            (Some(start), Some(end)) if end > start => {
                if let Some(group_id) = group_id {
                    windows_by_group
                        .entry(group_id.to_owned())
                        .or_default()
                        .push((start, end, id.to_owned()));
                }
            }
            _ => push(
                &mut errors,
                "INVALID_WINDOW_BOUNDS",
                &path,
                "window requires non-negative start_ms and end_ms > start_ms",
            ),
        }

        match finite(window, "time_uncertainty_ms") {
            Some(u) if u >= 0.0 => {}
            _ => push(
                &mut errors,
                "INVALID_WINDOW_TIME_UNCERTAINTY",
                &format!("{path}.time_uncertainty_ms"),
                "time uncertainty must be finite and non-negative",
            ),
        }
        let sync_ref = text(window, "clock_sync_ref");
        if sync_ref.is_none() {
            push(
                &mut errors,
                "MISSING_WINDOW_CLOCK_SYNC",
                &format!("{path}.clock_sync_ref"),
                "synchronized clock reference is required",
            );
        }

        let Some(observations) = window.get("channel_observations").and_then(Value::as_array) else {
            push(
                &mut errors,
                "MISSING_CHANNEL_OBSERVATIONS",
                &format!("{path}.channel_observations"),
                "every window must declare channel observations",
            );
            continue;
        };

        let mut observed_ids = BTreeSet::new();
        for (observation_index, observation) in observations.iter().enumerate() {
            let observation_path = format!("{path}.channel_observations[{observation_index}]");
            let Some(channel_id) = text(observation, "channel_id") else {
                push(
                    &mut errors,
                    "MISSING_OBSERVATION_CHANNEL_ID",
                    &format!("{observation_path}.channel_id"),
                    "channel_id is required",
                );
                continue;
            };

            if !channel_by_id.contains_key(channel_id) {
                push(
                    &mut errors,
                    "UNKNOWN_OBSERVATION_CHANNEL",
                    &format!("{observation_path}.channel_id"),
                    "observation references an undeclared channel",
                );
            }
            if !observed_ids.insert(channel_id.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_CHANNEL_OBSERVATION",
                    &format!("{observation_path}.channel_id"),
                    "each registered channel must have exactly one window observation",
                );
            }

            if text(observation, "clock_sync_ref") != sync_ref {
                push(
                    &mut errors,
                    "CHANNEL_CLOCK_SYNC_MISMATCH",
                    &format!("{observation_path}.clock_sync_ref"),
                    "all channel observations in a window must bind to its common synchronized time reference",
                );
            }
            match finite(observation, "time_uncertainty_ms") {
                Some(u) if u >= 0.0 => {}
                _ => push(
                    &mut errors,
                    "INVALID_CHANNEL_TIME_UNCERTAINTY",
                    &format!("{observation_path}.time_uncertainty_ms"),
                    "channel time uncertainty must be finite and non-negative",
                ),
            }

            match text(observation, "availability") {
                Some("OBSERVED") => {
                    if text(observation, "source_ref").is_none()
                        || text(observation, "quality_ref").is_none()
                    {
                        push(
                            &mut errors,
                            "OBSERVED_CHANNEL_MISSING_PROVENANCE",
                            &observation_path,
                            "observed channel data require source and quality references",
                        );
                    }
                    if text(observation, "unavailable_reason").is_some() {
                        push(
                            &mut errors,
                            "OBSERVED_CHANNEL_HAS_UNAVAILABLE_REASON",
                            &observation_path,
                            "observed channel cannot carry an unavailable reason",
                        );
                    }
                }
                Some("DEGRADED") => {
                    if text(observation, "source_ref").is_none()
                        || text(observation, "quality_ref").is_none()
                        || text(observation, "unavailable_reason").is_none()
                    {
                        push(
                            &mut errors,
                            "DEGRADED_CHANNEL_INCOMPLETE",
                            &observation_path,
                            "degraded channel requires raw source, quality evidence and a degradation reason",
                        );
                    }
                }
                Some("MISSING") => {
                    if text(observation, "unavailable_reason").is_none() {
                        push(
                            &mut errors,
                            "MISSING_CHANNEL_REASON",
                            &observation_path,
                            "missing channel requires an explicit reason; it is never zero-filled",
                        );
                    }
                    if text(observation, "source_ref").is_some() {
                        push(
                            &mut errors,
                            "MISSING_CHANNEL_HAS_SOURCE",
                            &observation_path,
                            "missing channel cannot claim a raw observation source",
                        );
                    }
                }
                _ => push(
                    &mut errors,
                    "INVALID_CHANNEL_AVAILABILITY",
                    &format!("{observation_path}.availability"),
                    "availability must be OBSERVED, DEGRADED or MISSING",
                ),
            }
        }

        for channel_id in channel_by_id.keys() {
            if !observed_ids.contains(channel_id) {
                push(
                    &mut errors,
                    "UNDECLARED_CHANNEL_GAP",
                    &format!("{path}.channel_observations"),
                    format!("registered channel {channel_id} has no explicit observation state"),
                );
            }
        }
    }

    for (group_id, windows) in &mut windows_by_group {
        windows.sort_by_key(|(start, _, _)| *start);
        for pair in windows.windows(2) {
            if pair[1].0 < pair[0].1 {
                push(
                    &mut errors,
                    "OVERLAPPING_WINDOWS_WITHIN_GROUP",
                    "$.windows",
                    format!(
                        "windows {} and {} overlap in group {group_id}; overlapping windows can leak nearly identical signals",
                        pair[0].2, pair[1].2
                    ),
                );
            }
        }
    }

    let Some(events) = case.get("events").and_then(Value::as_array) else {
        push(
            &mut errors,
            "MISSING_EVENTS",
            "$.events",
            "events must be an array",
        );
        return errors;
    };
    if events.is_empty() {
        push(
            &mut errors,
            "EMPTY_EVENTS",
            "$.events",
            "at least one independently labelled event or stable interval is required",
        );
    }

    let mut event_ids = BTreeSet::new();
    for (index, event) in events.iter().enumerate() {
        let path = format!("$.events[{index}]");
        let Some(event_id) = text(event, "event_id") else {
            push(
                &mut errors,
                "MISSING_EVENT_ID",
                &format!("{path}.event_id"),
                "event_id is required",
            );
            continue;
        };
        if !event_ids.insert(event_id.to_owned()) {
            push(
                &mut errors,
                "DUPLICATE_EVENT_ID",
                &format!("{path}.event_id"),
                "event identifiers must be unique",
            );
        }

        let Some(window_id) = text(event, "window_id") else {
            push(
                &mut errors,
                "MISSING_EVENT_WINDOW",
                &format!("{path}.window_id"),
                "event must reference one observation window",
            );
            continue;
        };
        let Some(window) = window_by_id.get(window_id).copied() else {
            push(
                &mut errors,
                "UNKNOWN_EVENT_WINDOW",
                &format!("{path}.window_id"),
                "event references an undeclared observation window",
            );
            continue;
        };

        let labels = event.get("state_labels").and_then(Value::as_array);
        let Some(labels) = labels else {
            push(
                &mut errors,
                "MISSING_EVENT_LABELS",
                &format!("{path}.state_labels"),
                "state_labels must be a non-empty array",
            );
            continue;
        };
        if labels.is_empty() {
            push(
                &mut errors,
                "EMPTY_EVENT_LABELS",
                &format!("{path}.state_labels"),
                "event requires at least one state label",
            );
        }

        let method = text(event, "evidence_method");
        let mut unique_labels = BTreeSet::new();
        for (label_index, label_value) in labels.iter().enumerate() {
            let Some(label) = label_value.as_str() else {
                push(
                    &mut errors,
                    "INVALID_EVENT_LABEL",
                    &format!("{path}.state_labels[{label_index}]"),
                    "state label must be a string",
                );
                continue;
            };
            if !unique_labels.insert(label.to_owned()) {
                push(
                    &mut errors,
                    "DUPLICATE_EVENT_LABEL",
                    &format!("{path}.state_labels[{label_index}]"),
                    "state labels must be unique within one event",
                );
            }
            if !label_method_compatible(label, method.unwrap_or("")) {
                push(
                    &mut errors,
                    "EVENT_LABEL_EVIDENCE_METHOD_MISMATCH",
                    &format!("{path}.evidence_method"),
                    format!("evidence method is not an admissible independent authority for label {label}"),
                );
            }

            if label == "PRESSURE_SENSOR_FAULT" {
                let degraded_pressure = event_has_degraded_kind(
                    event,
                    window,
                    &channel_by_id,
                    &["PRESSURE_ABSOLUTE", "PRESSURE_DIFFERENTIAL", "FILTER_DIFFERENTIAL_PRESSURE"],
                );
                if !degraded_pressure {
                    push(
                        &mut errors,
                        "PRESSURE_FAULT_WITHOUT_DEGRADED_CHANNEL",
                        &path,
                        "pressure sensor fault label requires an explicit degraded/missing pressure-channel observation",
                    );
                }
            }
            if label == "AIRFLOW_SENSOR_FAULT" {
                let degraded_airflow =
                    event_has_degraded_kind(event, window, &channel_by_id, &["AIRFLOW"]);
                if !degraded_airflow {
                    push(
                        &mut errors,
                        "AIRFLOW_FAULT_WITHOUT_DEGRADED_CHANNEL",
                        &path,
                        "airflow sensor fault label requires an explicit degraded/missing airflow-channel observation",
                    );
                }
            }
        }

        if text(event, "evidence_role") != Some("INDEPENDENT_EVENT_EVIDENCE") {
            push(
                &mut errors,
                "NON_INDEPENDENT_EVENT_LABEL",
                &format!("{path}.evidence_role"),
                "model predictions, simulations and unverified operator reports cannot establish event ground truth",
            );
        }
        for key in ["evidence_ref", "adjudicator_ref", "clock_sync_ref"] {
            if text(event, key).is_none() {
                push(
                    &mut errors,
                    "MISSING_EVENT_PROVENANCE",
                    &format!("{path}.{key}"),
                    "independent evidence, adjudicator and synchronized clock references are required",
                );
            }
        }
        if text(event, "clock_sync_ref") != text(window, "clock_sync_ref") {
            push(
                &mut errors,
                "EVENT_CLOCK_SYNC_MISMATCH",
                &format!("{path}.clock_sync_ref"),
                "event timestamps must use the exact observation-window clock reference",
            );
        }
        match finite(event, "time_uncertainty_ms") {
            Some(u) if u >= 0.0 => {}
            _ => push(
                &mut errors,
                "INVALID_EVENT_TIME_UNCERTAINTY",
                &format!("{path}.time_uncertainty_ms"),
                "event time uncertainty must be finite and non-negative",
            ),
        }

        let window_start = uint(window, "start_ms");
        let window_end = uint(window, "end_ms");
        let onset = uint(event, "onset_ms");
        let end = uint(event, "end_ms");
        let deadline = uint(event, "detection_deadline_ms");
        match (window_start, window_end, onset, end, deadline) {
            (Some(ws), Some(we), Some(on), Some(en), Some(dl))
                if on >= ws && on <= en && en <= we && dl >= on && dl <= we => {}
            _ => push(
                &mut errors,
                "INVALID_EVENT_TIME_BOUNDS",
                &path,
                "event onset/end must lie within its window; detection deadline must be between onset and window end",
            ),
        }

        if let Some(clear_time) = event.get("clear_time_ms") {
            match (clear_time, end) {
                (Value::Null, _) => {}
                (Value::Number(value), Some(event_end)) => {
                    if value.as_u64().is_none_or(|clear| clear < event_end) {
                        push(
                            &mut errors,
                            "INVALID_EVENT_CLEAR_TIME",
                            &format!("{path}.clear_time_ms"),
                            "clear time must be null or an integer at/after event end",
                        );
                    }
                }
                _ => push(
                    &mut errors,
                    "INVALID_EVENT_CLEAR_TIME",
                    &format!("{path}.clear_time_ms"),
                    "clear time must be null or an integer timestamp",
                ),
            }
        } else {
            push(
                &mut errors,
                "MISSING_EVENT_CLEAR_TIME",
                &format!("{path}.clear_time_ms"),
                "use null for not-yet-cleared events; omit is not equivalent to unknown",
            );
        }
    }

    errors
}

fn event_has_degraded_kind(
    _event: &Value,
    window: &Value,
    channel_by_id: &BTreeMap<String, &Value>,
    kinds: &[&str],
) -> bool {
    let observations = window
        .get("channel_observations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten();
    observations.into_iter().any(|observation| {
        let Some(channel_id) = text(observation, "channel_id") else {
            return false;
        };
        let Some(channel) = channel_by_id.get(channel_id) else {
            return false;
        };
        matches!(text(observation, "availability"), Some("DEGRADED" | "MISSING"))
            && channel_kind_is(channel, kinds)
    })
}

/// Evaluate a validated manifest's coverage and event composition.
/// This is not a model score or a physical-performance result.
pub fn evaluate_case(case: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case);
    if !errors.is_empty() {
        return Err(errors);
    }

    let split = &case["split"];
    let windows = case["windows"].as_array().expect("validated windows");
    let events = case["events"].as_array().expect("validated events");
    let channels = case["channels"].as_array().expect("validated channels");
    let groups = case["groups"].as_array().expect("validated groups");

    let mut missing = 0usize;
    let mut degraded = 0usize;
    let mut windows_with_non_observed = 0usize;
    for window in windows {
        let mut has_non_observed = false;
        for observation in window["channel_observations"]
            .as_array()
            .expect("validated observations")
        {
            match text(observation, "availability").expect("validated availability") {
                "MISSING" => {
                    missing += 1;
                    has_non_observed = true;
                }
                "DEGRADED" => {
                    degraded += 1;
                    has_non_observed = true;
                }
                _ => {}
            }
        }
        if has_non_observed {
            windows_with_non_observed += 1;
        }
    }

    let mut observed_labels = BTreeSet::new();
    let mut critical_events = 0usize;
    for event in events {
        if event.get("critical").and_then(Value::as_bool) == Some(true) {
            critical_events += 1;
        }
        if let Some(labels) = event.get("state_labels").and_then(Value::as_array) {
            for label in labels {
                if let Some(label) = label.as_str() {
                    observed_labels.insert(label.to_owned());
                }
            }
        }
    }

    let dimensions = split["heldout_dimensions"]
        .as_array()
        .expect("validated heldout dimensions")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("validated case id").to_owned(),
        channel_count: channels.len(),
        group_count: groups.len(),
        train_group_count: split["train_group_ids"].as_array().expect("train groups").len(),
        calibration_group_count: split["calibration_group_ids"]
            .as_array()
            .expect("calibration groups")
            .len(),
        test_group_count: split["test_group_ids"].as_array().expect("test groups").len(),
        heldout_dimensions: dimensions,
        window_count: windows.len(),
        event_count: events.len(),
        critical_event_count: critical_events,
        missing_channel_observation_count: missing,
        degraded_channel_observation_count: degraded,
        windows_with_non_observed_channels: windows_with_non_observed,
        observed_state_labels: observed_labels.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-conveyance-state-case.v0_1.json"
        ))
        .expect("fixture parses")
    }

    fn has_code(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn first_window_mut(case: &mut Value) -> &mut Value {
        &mut case["windows"][0]
    }

    fn test_window_mut(case: &mut Value) -> &mut Value {
        &mut case["windows"][2]
    }

    fn first_event_mut(case: &mut Value) -> &mut Value {
        &mut case["events"][0]
    }

    #[test]
    fn synthetic_event_manifest_is_valid() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn structural_report_counts_channels_partitions_and_events() {
        let report = evaluate_case(&fixture()).expect("fixture evaluates");
        assert_eq!(report.channel_count, 4);
        assert_eq!(report.group_count, 3);
        assert_eq!(report.train_group_count, 1);
        assert_eq!(report.calibration_group_count, 1);
        assert_eq!(report.test_group_count, 1);
        assert_eq!(report.window_count, 3);
        assert_eq!(report.event_count, 4);
        assert_eq!(report.critical_event_count, 1);
        assert_eq!(report.degraded_channel_observation_count, 1);
        assert_eq!(report.missing_channel_observation_count, 0);
        assert_eq!(report.windows_with_non_observed_channels, 1);
        assert!(report.observed_state_labels.contains(&"DEVELOPING_BLOCKAGE".to_owned()));
        assert!(report.observed_state_labels.contains(&"PRESSURE_SENSOR_FAULT".to_owned()));
    }

    #[test]
    fn split_groups_are_disjoint() {
        let mut case = fixture();
        case["split"]["test_group_ids"] = case["split"]["train_group_ids"].clone();
        assert!(has_code(
            &validate_case(&case),
            "GROUP_SPLIT_LEAKAGE"
        ));
    }

    #[test]
    fn declared_holdout_dimension_must_be_disjoint() {
        let mut case = fixture();
        case["groups"][2]["path_configuration_ref"] =
            case["groups"][0]["path_configuration_ref"].clone();
        assert!(has_code(
            &validate_case(&case),
            "HELDOUT_DIMENSION_LEAKAGE"
        ));
    }

    #[test]
    fn every_declared_group_must_be_assigned_once() {
        let mut case = fixture();
        case["groups"].as_array_mut().unwrap().push(serde_json::json!({
            "group_id":"unassigned","site_id":"s","campaign_id":"c","path_configuration_ref":"p",
            "machine_configuration_ref":"m","material_cohort_ref":"r","temporal_block_ref":"t","parent_rock_block_ref":"b"
        }));
        assert!(has_code(&validate_case(&case), "UNASSIGNED_GROUP"));
    }

    #[test]
    fn split_cannot_reference_unknown_group() {
        let mut case = fixture();
        case["split"]["test_group_ids"] = serde_json::json!(["not-declared"]);
        assert!(has_code(&validate_case(&case), "UNKNOWN_PARTITION_GROUP"));
    }

    #[test]
    fn duplicate_group_ids_are_rejected() {
        let mut case = fixture();
        let first = case["groups"][0].clone();
        case["groups"].as_array_mut().unwrap().push(first);
        assert!(has_code(&validate_case(&case), "DUPLICATE_GROUP_ID"));
    }

    #[test]
    fn duplicate_channel_ids_are_rejected() {
        let mut case = fixture();
        let first = case["channels"][0].clone();
        case["channels"].as_array_mut().unwrap().push(first);
        assert!(has_code(&validate_case(&case), "DUPLICATE_CHANNEL_ID"));
    }

    #[test]
    fn non_positive_sample_rate_is_rejected() {
        let mut case = fixture();
        case["channels"][0]["sample_rate_hz"] = serde_json::json!(0.0);
        assert!(has_code(&validate_case(&case), "INVALID_CHANNEL_SAMPLE_RATE"));
    }

    #[test]
    fn group_windows_may_not_overlap() {
        let mut case = fixture();
        let duplicate = case["windows"][0].clone();
        case["windows"].as_array_mut().unwrap().push(duplicate);
        assert!(has_code(&validate_case(&case), "OVERLAPPING_WINDOWS_WITHIN_GROUP"));
    }

    #[test]
    fn unknown_window_group_is_rejected() {
        let mut case = fixture();
        first_window_mut(&mut case)["group_id"] = serde_json::json!("no-group");
        assert!(has_code(&validate_case(&case), "UNKNOWN_WINDOW_GROUP"));
    }

    #[test]
    fn missing_channel_state_cannot_be_omitted() {
        let mut case = fixture();
        first_window_mut(&mut case)["channel_observations"].as_array_mut().unwrap().pop();
        assert!(has_code(&validate_case(&case), "UNDECLARED_CHANNEL_GAP"));
    }

    #[test]
    fn duplicate_channel_observation_is_rejected() {
        let mut case = fixture();
        let duplicate = case["windows"][0]["channel_observations"][0].clone();
        first_window_mut(&mut case)["channel_observations"].as_array_mut().unwrap().push(duplicate);
        assert!(has_code(&validate_case(&case), "DUPLICATE_CHANNEL_OBSERVATION"));
    }

    #[test]
    fn undeclared_channel_observation_is_rejected() {
        let mut case = fixture();
        case["windows"][0]["channel_observations"][0]["channel_id"] =
            serde_json::json!("unknown-channel");
        assert!(has_code(&validate_case(&case), "UNKNOWN_OBSERVATION_CHANNEL"));
    }

    #[test]
    fn observed_channel_requires_source_and_quality() {
        let mut case = fixture();
        case["windows"][0]["channel_observations"][0]
            .as_object_mut().unwrap().remove("source_ref");
        assert!(has_code(
            &validate_case(&case),
            "OBSERVED_CHANNEL_MISSING_PROVENANCE"
        ));
    }

    #[test]
    fn missing_channel_requires_reason_and_cannot_claim_source() {
        let mut case = fixture();
        let observation = &mut case["windows"][0]["channel_observations"][0];
        observation["availability"] = serde_json::json!("MISSING");
        observation.as_object_mut().unwrap().remove("unavailable_reason");
        assert!(has_code(&validate_case(&case), "MISSING_CHANNEL_REASON"));
        assert!(has_code(&validate_case(&case), "MISSING_CHANNEL_HAS_SOURCE"));
    }

    #[test]
    fn degraded_channel_requires_reason_quality_and_source() {
        let mut case = fixture();
        let observation = &mut case["windows"][0]["channel_observations"][0];
        observation["availability"] = serde_json::json!("DEGRADED");
        observation.as_object_mut().unwrap().remove("unavailable_reason");
        assert!(has_code(&validate_case(&case), "DEGRADED_CHANNEL_INCOMPLETE"));
    }

    #[test]
    fn channel_clock_sync_must_match_window() {
        let mut case = fixture();
        case["windows"][0]["channel_observations"][0]["clock_sync_ref"] =
            serde_json::json!("other-clock");
        assert!(has_code(&validate_case(&case), "CHANNEL_CLOCK_SYNC_MISMATCH"));
    }

    #[test]
    fn event_must_reference_existing_window() {
        let mut case = fixture();
        first_event_mut(&mut case)["window_id"] = serde_json::json!("missing-window");
        assert!(has_code(&validate_case(&case), "UNKNOWN_EVENT_WINDOW"));
    }

    #[test]
    fn event_time_bounds_must_fit_window() {
        let mut case = fixture();
        first_event_mut(&mut case)["onset_ms"] = serde_json::json!(0);
        assert!(has_code(&validate_case(&case), "INVALID_EVENT_TIME_BOUNDS"));
    }

    #[test]
    fn detection_deadline_cannot_precede_onset() {
        let mut case = fixture();
        first_event_mut(&mut case)["detection_deadline_ms"] = serde_json::json!(500);
        assert!(has_code(&validate_case(&case), "INVALID_EVENT_TIME_BOUNDS"));
    }

    #[test]
    fn clear_time_cannot_precede_event_end() {
        let mut case = fixture();
        first_event_mut(&mut case)["clear_time_ms"] = serde_json::json!(8000);
        assert!(has_code(&validate_case(&case), "INVALID_EVENT_CLEAR_TIME"));
    }

    #[test]
    fn event_model_prediction_cannot_be_ground_truth() {
        let mut case = fixture();
        first_event_mut(&mut case)["evidence_role"] = serde_json::json!("MODEL_ESTIMATE");
        assert!(has_code(&validate_case(&case), "NON_INDEPENDENT_EVENT_LABEL"));
    }

    #[test]
    fn normal_stable_requires_witnessed_normal_authority() {
        let mut case = fixture();
        first_event_mut(&mut case)["evidence_method"] =
            serde_json::json!("WITNESSED_PHYSICAL_INSPECTION");
        assert!(has_code(
            &validate_case(&case),
            "EVENT_LABEL_EVIDENCE_METHOD_MISMATCH"
        ));
    }

    #[test]
    fn blockage_requires_an_independent_physical_method() {
        let mut case = fixture();
        case["events"][2]["evidence_method"] =
            serde_json::json!("CALIBRATION_DIAGNOSTIC");
        assert!(has_code(
            &validate_case(&case),
            "EVENT_LABEL_EVIDENCE_METHOD_MISMATCH"
        ));
    }

    #[test]
    fn pressure_sensor_fault_requires_degraded_pressure_channel() {
        let mut case = fixture();
        for observation in case["windows"][2]["channel_observations"]
            .as_array_mut().unwrap()
        {
            if observation["channel_id"] == "dp-line" {
                observation["availability"] = serde_json::json!("OBSERVED");
                observation.as_object_mut().unwrap().remove("unavailable_reason");
            }
        }
        assert!(has_code(
            &validate_case(&case),
            "PRESSURE_FAULT_WITHOUT_DEGRADED_CHANNEL"
        ));
    }

    #[test]
    fn pressure_fault_requires_diagnostic_label_method() {
        let mut case = fixture();
        case["events"][3]["evidence_method"] =
            serde_json::json!("WITNESSED_PHYSICAL_INSPECTION");
        assert!(has_code(
            &validate_case(&case),
            "EVENT_LABEL_EVIDENCE_METHOD_MISMATCH"
        ));
    }

    #[test]
    fn single_and_multi_label_events_are_allowed() {
        let mut case = fixture();
        case["events"][2]["state_labels"] =
            serde_json::json!(["DEVELOPING_BLOCKAGE"]);
        assert!(validate_case(&case).is_empty());
    }

    #[test]
    fn event_clock_must_match_window_clock() {
        let mut case = fixture();
        first_event_mut(&mut case)["clock_sync_ref"] =
            serde_json::json!("other-clock");
        assert!(has_code(&validate_case(&case), "EVENT_CLOCK_SYNC_MISMATCH"));
    }

    #[test]
    fn explicit_missing_channel_is_reported_not_zero_filled() {
        let mut case = fixture();
        let obs = &mut case["windows"][2]["channel_observations"][0];
        obs["availability"] = serde_json::json!("MISSING");
        obs["unavailable_reason"] = serde_json::json!("sensor unplugged");
        obs.as_object_mut().unwrap().remove("source_ref");
        obs.as_object_mut().unwrap().remove("quality_ref");
        case["events"].as_array_mut().unwrap().retain(|event| {
            event["event_id"] != "event-sensor-fault-001"
        });
        let report = evaluate_case(&case).expect("missing channel is explicit");
        assert_eq!(report.missing_channel_observation_count, 1);
    }

    #[test]
    fn replay_is_deterministic() {
        let case = fixture();
        assert_eq!(
            evaluate_case(&case).expect("first"),
            evaluate_case(&case).expect("second")
        );
    }
}
