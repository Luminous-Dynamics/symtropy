// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Semantic checks for the synthetic Navigation Observability Transition corpus.
//!
//! This validator is intentionally narrower than a JSON Schema implementation.
//! It checks cross-record invariants that are easy to miss when validating each
//! JSON object independently. JSON Schema validation remains a separate gate.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub mod schema_validation;
pub mod dataset_admission;
pub mod generalization_split;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub path: String,
    pub message: String,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationSummary {
    pub case_count: usize,
    pub source_count: usize,
    pub phase_count: usize,
    pub observation_count: usize,
    pub transition_count: usize,
    pub fault_count: usize,
    pub evaluator_truth_state_count: usize,
    pub positive_control_count: usize,
    pub negative_control_count: usize,
}

fn issue(issues: &mut Vec<ValidationIssue>, path: impl Into<String>, message: impl Into<String>) {
    issues.push(ValidationIssue {
        path: path.into(),
        message: message.into(),
    });
}

fn object<'a>(
    value: &'a Value,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a Map<String, Value>> {
    match value.as_object() {
        Some(map) => Some(map),
        None => {
            issue(issues, path, "must be an object");
            None
        }
    }
}

fn array<'a>(
    value: &'a Value,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a Vec<Value>> {
    match value.as_array() {
        Some(values) => Some(values),
        None => {
            issue(issues, path, "must be an array");
            None
        }
    }
}

fn value<'a>(
    object_value: &'a Value,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a Value> {
    let map = object(object_value, path, issues)?;
    match map.get(key) {
        Some(found) => Some(found),
        None => {
            issue(issues, path, format!("missing required property {key}"));
            None
        }
    }
}

fn string<'a>(
    object_value: &'a Value,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<&'a str> {
    let field_path = format!("{path}.{key}");
    match value(object_value, key, path, issues).and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Some(text),
        Some(_) => {
            issue(issues, field_path, "must not be empty");
            None
        }
        None => {
            if !issues.iter().any(|item| item.path == field_path) {
                issue(issues, field_path, "must be a string");
            }
            None
        }
    }
}

fn unsigned(
    object_value: &Value,
    key: &str,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Option<u64> {
    let field_path = format!("{path}.{key}");
    match value(object_value, key, path, issues).and_then(Value::as_u64) {
        Some(number) => Some(number),
        None => {
            if !issues.iter().any(|item| item.path == field_path) {
                issue(issues, field_path, "must be a non-negative integer");
            }
            None
        }
    }
}

fn strings(
    value: &Value,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) -> Vec<String> {
    let Some(values) = array(value, path, issues) else {
        return Vec::new();
    };
    values
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item.as_str() {
            Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
            _ => {
                issue(issues, format!("{path}[{index}]"), "must be a non-empty string");
                None
            }
        })
        .collect()
}

fn unique_strings(values: &[String], path: &str, issues: &mut Vec<ValidationIssue>) {
    let mut seen = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        if !seen.insert(item.as_str()) {
            issue(
                issues,
                format!("{path}[{index}]"),
                format!("duplicate identifier {item}"),
            );
        }
    }
}

fn allowed_state(state: &str) -> bool {
    matches!(
        state,
        "NORMAL"
            | "DEGRADED"
            | "INTEGRITY_UNCERTAIN"
            | "UNAVAILABLE"
            | "RECOVERING"
            | "INDETERMINATE"
    )
}

fn finite_number(value: &Value) -> bool {
    value.as_f64().is_some_and(f64::is_finite)
}

fn vector_len(
    value: &Value,
    expected: usize,
    path: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(values) = array(value, path, issues) else {
        return;
    };
    if values.len() != expected {
        issue(
            issues,
            path,
            format!("must contain exactly {expected} numeric components"),
        );
        return;
    }
    for (index, component) in values.iter().enumerate() {
        if !finite_number(component) {
            issue(
                issues,
                format!("{path}[{index}]"),
                "component must be a finite number",
            );
        }
    }
}

fn check_case(
    case: &Value,
    case_index: usize,
    issues: &mut Vec<ValidationIssue>,
    totals: &mut ValidationSummary,
    case_ids: &mut BTreeSet<String>,
) {
    let path = format!("$.cases[{case_index}]");
    if string(case, "schema_version", &path, issues) != Some("nav-transition-case/v0.1") {
        issue(issues, format!("{path}.schema_version"), "unsupported case schema version");
    }
    if string(case, "evidence_class", &path, issues) != Some("synthetic") {
        issue(issues, format!("{path}.evidence_class"), "v0.1 cases must be synthetic");
    }
    if string(case, "claim_ceiling", &path, issues)
        != Some("synthetic_transition_contract_only")
    {
        issue(
            issues,
            format!("{path}.claim_ceiling"),
            "unexpected claim ceiling",
        );
    }

    let case_id = string(case, "case_id", &path, issues).unwrap_or("<invalid>").to_owned();
    if !case_ids.insert(case_id.clone()) {
        issue(issues, format!("{path}.case_id"), "duplicate case ID");
    }

    let domain_path = value(case, "domain_path", &path, issues)
        .map(|v| strings(v, &format!("{path}.domain_path"), issues))
        .unwrap_or_default();
    if domain_path.len() < 2 {
        issue(
            issues,
            format!("{path}.domain_path"),
            "must declare at least two environment/domain entries",
        );
    }

    // Sources and failure-domain declarations.
    let mut source_modalities = BTreeMap::<String, String>::new();
    if let Some(source_values) = value(case, "source_registry", &path, issues)
        .and_then(|v| array(v, &format!("{path}.source_registry"), issues))
    {
        if source_values.is_empty() {
            issue(issues, format!("{path}.source_registry"), "must not be empty");
        }
        for (index, source) in source_values.iter().enumerate() {
            let source_path = format!("{path}.source_registry[{index}]");
            let source_id = string(source, "source_id", &source_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            let modality = string(source, "modality", &source_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            let _configuration = string(source, "configuration_ref", &source_path, issues);
            let _clock = string(source, "clock_domain_ref", &source_path, issues);
            let status = string(source, "qualification_status", &source_path, issues);
            if !matches!(
                status,
                Some("synthetic_fixture" | "candidate_unqualified" | "stale" | "invalid")
            ) {
                issue(
                    issues,
                    format!("{source_path}.qualification_status"),
                    "unsupported source qualification status",
                );
            }
            let failure_domains = value(source, "failure_domains", &source_path, issues)
                .map(|v| strings(v, &format!("{source_path}.failure_domains"), issues))
                .unwrap_or_default();
            if failure_domains.is_empty() {
                issue(
                    issues,
                    format!("{source_path}.failure_domains"),
                    "must declare at least one failure domain",
                );
            }
            unique_strings(
                &failure_domains,
                &format!("{source_path}.failure_domains"),
                issues,
            );
            if source_modalities
                .insert(source_id.clone(), modality)
                .is_some()
            {
                issue(
                    issues,
                    format!("{source_path}.source_id"),
                    format!("duplicate source ID {source_id}"),
                );
            }
            totals.source_count += 1;
        }
    }

    // Frame and clock identities are assembled from phase profiles. Repeated
    // IDs may recur only with exactly the same declarations.
    let mut frame_bodies = BTreeMap::<String, String>::new();
    let mut frame_definitions = BTreeMap::<String, Value>::new();
    let mut time_definitions = BTreeMap::<String, Value>::new();
    let mut phase_states = BTreeMap::<String, String>::new();
    let mut phase_ids = BTreeSet::<String>::new();

    if let Some(phases) = value(case, "phases", &path, issues)
        .and_then(|v| array(v, &format!("{path}.phases"), issues))
    {
        if phases.len() < 2 {
            issue(issues, format!("{path}.phases"), "must contain at least two phases");
        }
        for (index, phase) in phases.iter().enumerate() {
            let phase_path = format!("{path}.phases[{index}]");
            let phase_id = string(phase, "phase_id", &phase_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            if !phase_ids.insert(phase_id.clone()) {
                issue(
                    issues,
                    format!("{phase_path}.phase_id"),
                    format!("duplicate phase ID {phase_id}"),
                );
            }
            let environment = string(phase, "environment", &phase_path, issues)
                .unwrap_or("<invalid>");
            if !domain_path.iter().any(|entry| entry == environment) {
                issue(
                    issues,
                    format!("{phase_path}.environment"),
                    "phase environment is not present in domain_path",
                );
            }
            let body_id = string(phase, "body_id", &phase_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            let expected_state = string(phase, "expected_service_state", &phase_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            if !allowed_state(&expected_state) {
                issue(
                    issues,
                    format!("{phase_path}.expected_service_state"),
                    "unsupported navigation service state",
                );
            }
            phase_states.insert(phase_id, expected_state);

            let frame = value(phase, "frame", &phase_path, issues);
            if let Some(frame) = frame {
                let frame_id = string(frame, "frame_id", &format!("{phase_path}.frame"), issues)
                    .unwrap_or("<invalid>")
                    .to_owned();
                let frame_body = string(frame, "body_id", &format!("{phase_path}.frame"), issues)
                    .unwrap_or("<invalid>")
                    .to_owned();
                let _frame_kind =
                    string(frame, "frame_kind", &format!("{phase_path}.frame"), issues);
                let _origin =
                    string(frame, "origin_ref", &format!("{phase_path}.frame"), issues);
                let _epoch = string(frame, "epoch_ref", &format!("{phase_path}.frame"), issues);
                if frame_body != body_id {
                    issue(
                        issues,
                        format!("{phase_path}.frame.body_id"),
                        "frame body identity does not match phase body identity",
                    );
                }
                if let Some(existing) = frame_bodies.get(&frame_id) {
                    if existing != &frame_body {
                        issue(
                            issues,
                            format!("{phase_path}.frame.frame_id"),
                            "the same frame ID is declared for different bodies",
                        );
                    }
                } else {
                    frame_bodies.insert(frame_id.clone(), frame_body);
                }
                if let Some(existing) = frame_definitions.get(&frame_id) {
                    if existing != frame {
                        issue(
                            issues,
                            format!("{phase_path}.frame"),
                            "repeated frame ID has conflicting frame metadata",
                        );
                    }
                } else {
                    frame_definitions.insert(frame_id, frame.clone());
                }
            }

            let time_basis = value(phase, "time_basis", &phase_path, issues);
            if let Some(time_basis) = time_basis {
                let time_id = string(
                    time_basis,
                    "time_basis_id",
                    &format!("{phase_path}.time_basis"),
                    issues,
                )
                .unwrap_or("<invalid>")
                .to_owned();
                let _kind = string(
                    time_basis,
                    "kind",
                    &format!("{phase_path}.time_basis"),
                    issues,
                );
                let _clock_or_scale = string(
                    time_basis,
                    "clock_or_scale_id",
                    &format!("{phase_path}.time_basis"),
                    issues,
                );
                let _epoch = string(
                    time_basis,
                    "epoch_ref",
                    &format!("{phase_path}.time_basis"),
                    issues,
                );
                if let Some(uncertainty) = value(
                    time_basis,
                    "uncertainty_s",
                    &format!("{phase_path}.time_basis"),
                    issues,
                ) {
                    if !finite_number(uncertainty) || uncertainty.as_f64().unwrap_or(-1.0) < 0.0
                    {
                        issue(
                            issues,
                            format!("{phase_path}.time_basis.uncertainty_s"),
                            "must be finite and non-negative",
                        );
                    }
                }
                if let Some(existing) = time_definitions.get(&time_id) {
                    if existing != time_basis {
                        issue(
                            issues,
                            format!("{phase_path}.time_basis"),
                            "repeated time-basis ID has conflicting clock/scale metadata",
                        );
                    }
                } else {
                    time_definitions.insert(time_id, time_basis.clone());
                }
            }

            let measurements = value(phase, "measurement_classes", &phase_path, issues)
                .map(|v| strings(v, &format!("{phase_path}.measurement_classes"), issues))
                .unwrap_or_default();
            for modality in measurements {
                if !source_modalities.values().any(|candidate| candidate == &modality) {
                    issue(
                        issues,
                        format!("{phase_path}.measurement_classes"),
                        format!("declared modality {modality} has no registered source"),
                    );
                }
            }
            let _communications =
                string(phase, "communications_state", &phase_path, issues);
            totals.phase_count += 1;
        }
    }

    // The controller stream may contain only declared observations/statuses.
    let mut observation_ids = BTreeSet::new();
    let mut prior_offset = None;
    if let Some(observations) = value(case, "controller_stream", &path, issues)
        .and_then(|v| array(v, &format!("{path}.controller_stream"), issues))
    {
        if observations.is_empty() {
            issue(
                issues,
                format!("{path}.controller_stream"),
                "must contain at least one controller-visible record",
            );
        }
        for (index, observation) in observations.iter().enumerate() {
            let observation_path = format!("{path}.controller_stream[{index}]");
            let observation_id = string(observation, "observation_id", &observation_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            if !observation_ids.insert(observation_id.clone()) {
                issue(
                    issues,
                    format!("{observation_path}.observation_id"),
                    format!("duplicate observation ID {observation_id}"),
                );
            }
            let offset = unsigned(observation, "offset_ms", &observation_path, issues);
            if let Some(offset) = offset {
                if prior_offset.is_some_and(|previous| offset < previous) {
                    issue(
                        issues,
                        format!("{observation_path}.offset_ms"),
                        "controller stream offsets must be nondecreasing",
                    );
                }
                prior_offset = Some(offset);
            }
            let source_id = string(observation, "source_id", &observation_path, issues)
                .unwrap_or("<invalid>");
            let source_modality = source_modalities.get(source_id);
            if source_modality.is_none() {
                issue(
                    issues,
                    format!("{observation_path}.source_id"),
                    format!("source {source_id} is not declared in source_registry"),
                );
            }
            if let Some(modality) = observation.get("modality").and_then(Value::as_str) {
                if source_modality.is_some_and(|registered| registered != modality) {
                    issue(
                        issues,
                        format!("{observation_path}.modality"),
                        "observation modality conflicts with registered source modality",
                    );
                }
            }
            let frame_id = string(observation, "frame_id", &observation_path, issues)
                .unwrap_or("<invalid>");
            if !frame_bodies.contains_key(frame_id) {
                issue(
                    issues,
                    format!("{observation_path}.frame_id"),
                    format!("frame {frame_id} is not declared by any phase"),
                );
            }
            let time_id = string(observation, "time_basis_id", &observation_path, issues)
                .unwrap_or("<invalid>");
            if !time_definitions.contains_key(time_id) {
                issue(
                    issues,
                    format!("{observation_path}.time_basis_id"),
                    format!("time basis {time_id} is not declared by any phase"),
                );
            }
            let _kind = string(observation, "event_kind", &observation_path, issues);
            let _class = string(observation, "observation_class", &observation_path, issues);
            if let Some(components) = observation.get("components") {
                if !components.is_null() {
                    if let Some(values) = array(
                        components,
                        &format!("{observation_path}.components"),
                        issues,
                    ) {
                        for (component_index, component) in values.iter().enumerate() {
                            if !finite_number(component) {
                                issue(
                                    issues,
                                    format!(
                                        "{observation_path}.components[{component_index}]"
                                    ),
                                    "component must be a finite number",
                                );
                            }
                        }
                    }
                }
            }
            for hidden_key in [
                "evaluator_only",
                "truth_states",
                "event_labels",
                "ground_truth",
                "oracle_state",
            ] {
                if observation.get(hidden_key).is_some() {
                    issue(
                        issues,
                        format!("{observation_path}.{hidden_key}"),
                        "evaluator-only truth or labels must not appear in controller_stream",
                    );
                }
            }
            totals.observation_count += 1;
        }
    }

    // Transitions must refer to known phases and agree with the destination
    // phase's declared service state.
    let mut transition_ids = BTreeSet::new();
    let mut transition_offset = None;
    if let Some(transitions) = value(case, "transitions", &path, issues)
        .and_then(|v| array(v, &format!("{path}.transitions"), issues))
    {
        if transitions.is_empty() {
            issue(issues, format!("{path}.transitions"), "must not be empty");
        }
        for (index, transition) in transitions.iter().enumerate() {
            let transition_path = format!("{path}.transitions[{index}]");
            let transition_id = string(transition, "transition_id", &transition_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            if !transition_ids.insert(transition_id.clone()) {
                issue(
                    issues,
                    format!("{transition_path}.transition_id"),
                    "duplicate transition ID",
                );
            }
            let from_phase = string(transition, "from_phase", &transition_path, issues)
                .unwrap_or("<invalid>");
            let to_phase = string(transition, "to_phase", &transition_path, issues)
                .unwrap_or("<invalid>");
            if !phase_ids.contains(from_phase) || !phase_ids.contains(to_phase) {
                issue(
                    issues,
                    &transition_path,
                    "transition references an unknown phase",
                );
            }
            let expected = string(transition, "expected_state", &transition_path, issues)
                .unwrap_or("<invalid>");
            if !allowed_state(expected) {
                issue(
                    issues,
                    format!("{transition_path}.expected_state"),
                    "unsupported navigation service state",
                );
            }
            if let Some(destination_state) = phase_states.get(to_phase) {
                if destination_state != expected {
                    issue(
                        issues,
                        format!("{transition_path}.expected_state"),
                        "transition state does not match destination phase service state",
                    );
                }
            }
            if let Some(offset) = unsigned(transition, "offset_ms", &transition_path, issues) {
                if transition_offset.is_some_and(|previous| offset < previous) {
                    issue(
                        issues,
                        format!("{transition_path}.offset_ms"),
                        "transition offsets must be nondecreasing",
                    );
                }
                transition_offset = Some(offset);
            }
            totals.transition_count += 1;
        }
    }

    // Fault schedules and source references.
    let mut fault_ids = BTreeSet::new();
    if let Some(faults) = value(case, "faults", &path, issues)
        .and_then(|v| array(v, &format!("{path}.faults"), issues))
    {
        for (index, fault) in faults.iter().enumerate() {
            let fault_path = format!("{path}.faults[{index}]");
            let fault_id = string(fault, "fault_id", &fault_path, issues)
                .unwrap_or("<invalid>")
                .to_owned();
            if !fault_ids.insert(fault_id) {
                issue(
                    issues,
                    format!("{fault_path}.fault_id"),
                    "duplicate fault ID",
                );
            }
            let source_ids = value(fault, "source_ids", &fault_path, issues)
                .map(|v| strings(v, &format!("{fault_path}.source_ids"), issues))
                .unwrap_or_default();
            if source_ids.is_empty() {
                issue(
                    issues,
                    format!("{fault_path}.source_ids"),
                    "must identify at least one affected source",
                );
            }
            for source_id in source_ids {
                if !source_modalities.contains_key(&source_id) {
                    issue(
                        issues,
                        format!("{fault_path}.source_ids"),
                        format!("fault references undeclared source {source_id}"),
                    );
                }
            }
            let start = unsigned(fault, "start_offset_ms", &fault_path, issues);
            let end = unsigned(fault, "end_offset_ms", &fault_path, issues);
            if let (Some(start), Some(end)) = (start, end) {
                if end < start {
                    issue(
                        issues,
                        &fault_path,
                        "fault end_offset_ms precedes start_offset_ms",
                    );
                }
            }
            let _kind = string(fault, "fault_kind", &fault_path, issues);
            let _effect =
                string(fault, "controller_visible_effect", &fault_path, issues);
            totals.fault_count += 1;
        }
    }

    // Evaluator-only truth and labels must remain separate and refer to the
    // declared phase/frame/fault universe.
    if let Some(evaluator) = value(case, "evaluator_only", &path, issues) {
        let reference_class = string(evaluator, "reference_class", &format!("{path}.evaluator_only"), issues);
        if !matches!(
            reference_class,
            Some("synthetic_oracle" | "independent_metrology" | "measured_reference")
        ) {
            issue(
                issues,
                format!("{path}.evaluator_only.reference_class"),
                "unsupported evaluator reference class",
            );
        }
        let reference_sources = value(
            evaluator,
            "reference_sources",
            &format!("{path}.evaluator_only"),
            issues,
        )
        .map(|v| strings(v, &format!("{path}.evaluator_only.reference_sources"), issues))
        .unwrap_or_default();
        if reference_sources.is_empty() {
            issue(
                issues,
                format!("{path}.evaluator_only.reference_sources"),
                "must identify at least one evaluator reference source",
            );
        }
        let mut truth_ids = BTreeSet::new();
        let mut truth_offset = None;
        if let Some(truths) = value(
            evaluator,
            "truth_states",
            &format!("{path}.evaluator_only"),
            issues,
        )
        .and_then(|v| array(v, &format!("{path}.evaluator_only.truth_states"), issues))
        {
            if truths.len() < 2 {
                issue(
                    issues,
                    format!("{path}.evaluator_only.truth_states"),
                    "must contain at least two truth states",
                );
            }
            for (index, truth) in truths.iter().enumerate() {
                let truth_path = format!("{path}.evaluator_only.truth_states[{index}]");
                let truth_id = string(truth, "truth_id", &truth_path, issues)
                    .unwrap_or("<invalid>")
                    .to_owned();
                if !truth_ids.insert(truth_id) {
                    issue(
                        issues,
                        format!("{truth_path}.truth_id"),
                        "duplicate evaluator truth ID",
                    );
                }
                let truth_frame = string(truth, "frame_id", &truth_path, issues)
                    .unwrap_or("<invalid>");
                let truth_body = string(truth, "body_id", &truth_path, issues)
                    .unwrap_or("<invalid>");
                match frame_bodies.get(truth_frame) {
                    Some(expected_body) if expected_body == truth_body => {}
                    Some(_) => issue(
                        issues,
                        format!("{truth_path}.body_id"),
                        "truth body does not match declared truth frame body",
                    ),
                    None => issue(
                        issues,
                        format!("{truth_path}.frame_id"),
                        "truth references an undeclared frame",
                    ),
                }
                vector_len(
                    value(truth, "position_m", &truth_path, issues)
                        .unwrap_or(&Value::Null),
                    3,
                    &format!("{truth_path}.position_m"),
                    issues,
                );
                vector_len(
                    value(truth, "velocity_mps", &truth_path, issues)
                        .unwrap_or(&Value::Null),
                    3,
                    &format!("{truth_path}.velocity_mps"),
                    issues,
                );
                if let Some(attitude) = truth.get("attitude_quaternion_xyzw") {
                    vector_len(
                        attitude,
                        4,
                        &format!("{truth_path}.attitude_quaternion_xyzw"),
                        issues,
                    );
                }
                if let Some(offset) = unsigned(truth, "offset_ms", &truth_path, issues) {
                    if truth_offset.is_some_and(|previous| offset < previous) {
                        issue(
                            issues,
                            format!("{truth_path}.offset_ms"),
                            "evaluator truth offsets must be nondecreasing",
                        );
                    }
                    truth_offset = Some(offset);
                }
                totals.evaluator_truth_state_count += 1;
            }
        }

        let mut labelled_faults = BTreeSet::new();
        if let Some(labels) = value(
            evaluator,
            "event_labels",
            &format!("{path}.evaluator_only"),
            issues,
        )
        .and_then(|v| array(v, &format!("{path}.evaluator_only.event_labels"), issues))
        {
            for (index, label) in labels.iter().enumerate() {
                let label_path = format!("{path}.evaluator_only.event_labels[{index}]");
                let fault_id = string(label, "fault_id", &label_path, issues)
                    .unwrap_or("<invalid>")
                    .to_owned();
                if !fault_ids.contains(&fault_id) {
                    issue(
                        issues,
                        format!("{label_path}.fault_id"),
                        format!("event label references unknown fault {fault_id}"),
                    );
                }
                if !labelled_faults.insert(fault_id) {
                    issue(
                        issues,
                        format!("{label_path}.fault_id"),
                        "duplicate evaluator event label for fault",
                    );
                }
                let _kind = string(label, "true_fault_kind", &label_path, issues);
                let _onset = unsigned(label, "onset_offset_ms", &label_path, issues);
                if label.get("cause_attribution").and_then(Value::as_str)
                    != Some("not_adjudicated")
                {
                    issue(
                        issues,
                        format!("{label_path}.cause_attribution"),
                        "v0.1 does not adjudicate cause or actor attribution",
                    );
                }
            }
        }
    }

    // Expected dispositions are ordered evaluator expectations, never
    // controller inputs. Require deterministic nondecreasing offsets.
    let mut disposition_offset = None;
    let mut dispositions = BTreeSet::new();
    if let Some(expected) = value(case, "expected_dispositions", &path, issues)
        .and_then(|v| array(v, &format!("{path}.expected_dispositions"), issues))
    {
        if expected.is_empty() {
            issue(
                issues,
                format!("{path}.expected_dispositions"),
                "must contain at least one expected disposition",
            );
        }
        for (index, disposition) in expected.iter().enumerate() {
            let disposition_path = format!("{path}.expected_dispositions[{index}]");
            let state = string(disposition, "expected_state", &disposition_path, issues)
                .unwrap_or("<invalid>");
            if !allowed_state(state) {
                issue(
                    issues,
                    format!("{disposition_path}.expected_state"),
                    "unsupported navigation service state",
                );
            }
            let condition =
                string(disposition, "minimum_evidence_condition", &disposition_path, issues);
            let reason = string(disposition, "reason_code", &disposition_path, issues);
            if condition.is_none() || reason.is_none() {
                // The field-level errors above are sufficient.
            }
            if let Some(offset) =
                unsigned(disposition, "at_offset_ms", &disposition_path, issues)
            {
                if disposition_offset.is_some_and(|previous| offset < previous) {
                    issue(
                        issues,
                        format!("{disposition_path}.at_offset_ms"),
                        "expected disposition offsets must be nondecreasing",
                    );
                }
                disposition_offset = Some(offset);
                if !dispositions.insert((offset, state.to_owned())) {
                    issue(
                        issues,
                        &disposition_path,
                        "duplicate state expectation at the same offset",
                    );
                }
            }
        }
    }

    // These counters are useful in the CLI and benchmark receipt.
    if case_id.contains("positive") {
        totals.positive_control_count += 1;
    }
    if case_id.contains("negative") {
        totals.negative_control_count += 1;
    }
}

pub fn validate_corpus(
    fixture: &Value,
    manifest: &Value,
) -> Result<ValidationSummary, Vec<ValidationIssue>> {
    let mut issues = Vec::new();
    let mut summary = ValidationSummary {
        case_count: 0,
        source_count: 0,
        phase_count: 0,
        observation_count: 0,
        transition_count: 0,
        fault_count: 0,
        evaluator_truth_state_count: 0,
        positive_control_count: 0,
        negative_control_count: 0,
    };

    if string(fixture, "schema_version", "$", &mut issues)
        != Some("nav-transition-corpus/v0.1")
    {
        issue(&mut issues, "$.schema_version", "unsupported corpus schema version");
    }
    if string(fixture, "benchmark_id", "$", &mut issues) != Some("nav-transition-001") {
        issue(&mut issues, "$.benchmark_id", "unexpected benchmark identity");
    }
    if string(fixture, "claim_ceiling", "$", &mut issues)
        != Some("synthetic_transition_contract_only")
    {
        issue(&mut issues, "$.claim_ceiling", "unexpected corpus claim ceiling");
    }

    let cases = value(fixture, "cases", "$", &mut issues)
        .and_then(|v| array(v, "$.cases", &mut issues))
        .cloned()
        .unwrap_or_default();
    summary.case_count = cases.len();

    let fixture_count = unsigned(fixture, "case_count", "$", &mut issues);
    if fixture_count != Some(cases.len() as u64) {
        issue(&mut issues, "$.case_count", "does not match cases array length");
    }

    if string(manifest, "schema_version", "$manifest", &mut issues)
        != Some("nav-transition-benchmark/v0.1")
    {
        issue(
            &mut issues,
            "$manifest.schema_version",
            "unsupported manifest schema version",
        );
    }
    if string(manifest, "benchmark_id", "$manifest", &mut issues)
        != Some("nav-transition-001")
    {
        issue(
            &mut issues,
            "$manifest.benchmark_id",
            "manifest benchmark identity does not match fixture",
        );
    }
    let manifest_count = unsigned(manifest, "case_count", "$manifest", &mut issues);
    if manifest_count != Some(cases.len() as u64) {
        issue(
            &mut issues,
            "$manifest.case_count",
            "does not match fixture cases array length",
        );
    }
    let manifest_case_ids = value(manifest, "case_ids", "$manifest", &mut issues)
        .map(|v| strings(v, "$manifest.case_ids", &mut issues))
        .unwrap_or_default();
    unique_strings(&manifest_case_ids, "$manifest.case_ids", &mut issues);

    let mut case_ids = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        check_case(case, index, &mut issues, &mut summary, &mut case_ids);
    }

    let expected_ids = case_ids.iter().cloned().collect::<Vec<_>>();
    let mut actual_ids = manifest_case_ids;
    actual_ids.sort();
    if actual_ids != expected_ids {
        issue(
            &mut issues,
            "$manifest.case_ids",
            "manifest case IDs do not exactly match fixture case IDs",
        );
    }

    if !issues.is_empty() {
        return Err(issues);
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/fixtures/nav_transition_cases.v0.1.json"
        ))
        .expect("fixture JSON parses")
    }

    fn parse_manifest() -> Value {
        serde_json::from_str(include_str!(
            "../../../docs/research/navigation-transition-v0.1/manifest.v0.1.json"
        ))
        .expect("manifest JSON parses")
    }

    #[test]
    fn committed_corpus_passes_semantic_cross_reference_checks() {
        let summary = validate_corpus(&parse_fixture(), &parse_manifest()).unwrap();
        assert_eq!(summary.case_count, 14);
        assert!(summary.source_count > 0);
        assert!(summary.observation_count > 0);
        assert_eq!(summary.positive_control_count, 2);
        assert_eq!(summary.negative_control_count, 3);
    }

    #[test]
    fn undeclared_controller_source_is_rejected() {
        let mut fixture = parse_fixture();
        fixture["cases"][0]["controller_stream"][0]["source_id"] =
            Value::String("src.not.registered".to_owned());
        let errors = validate_corpus(&fixture, &parse_manifest()).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("not declared")));
    }

    #[test]
    fn body_frame_mismatch_is_rejected() {
        let mut fixture = parse_fixture();
        fixture["cases"][0]["phases"][0]["frame"]["body_id"] =
            Value::String("mars".to_owned());
        let errors = validate_corpus(&fixture, &parse_manifest()).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("phase body identity")));
    }

    #[test]
    fn evaluator_truth_cannot_leak_into_controller_stream() {
        let mut fixture = parse_fixture();
        fixture["cases"][0]["controller_stream"][0]["truth_states"] =
            Value::Array(Vec::new());
        let errors = validate_corpus(&fixture, &parse_manifest()).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("evaluator-only")));
    }

    #[test]
    fn mismatched_manifest_case_set_is_rejected() {
        let fixture = parse_fixture();
        let mut manifest = parse_manifest();
        manifest["case_ids"][0] = Value::String("nav.unknown.case".to_owned());
        let errors = validate_corpus(&fixture, &manifest).unwrap_err();
        assert!(errors.iter().any(|error| error.message.contains("exactly match")));
    }

    #[test]
    fn shared_source_count_is_not_used_as_independence_proof() {
        let fixture = parse_fixture();
        let case = &fixture["cases"];
        let common_mode = case
            .as_array()
            .unwrap()
            .iter()
            .find(|item| {
                item["case_id"] == "nav.common-mode-false-independence.negative.v0.1"
            })
            .unwrap();
        let ids = common_mode["source_registry"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source["source_id"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), 3);
        let shared_a = &common_mode["source_registry"][0]["failure_domains"];
        let shared_b = &common_mode["source_registry"][1]["failure_domains"];
        assert!(shared_a.as_array().unwrap().iter().any(|domain| {
            shared_b.as_array().unwrap().contains(domain)
        }));
    }
}
pub mod mission_risk_contract;
