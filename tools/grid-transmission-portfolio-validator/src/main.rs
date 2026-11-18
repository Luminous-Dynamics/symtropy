// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Violation {
    code: String,
    path: String,
    message: String,
}

#[derive(Debug)]
struct Report {
    schema_version: String,
    portfolio_id: String,
    violations: Vec<Violation>,
}

impl Report {
    fn is_valid(&self) -> bool {
        self.violations.is_empty()
    }

    fn to_json(&self) -> Value {
        json!({
            "schema_version": self.schema_version,
            "portfolio_id": self.portfolio_id,
            "valid": self.is_valid(),
            "violations": self.violations.iter().map(|v| json!({
                "code": v.code,
                "path": v.path,
                "message": v.message,
            })).collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone)]
struct SegmentInfo {
    id: String,
    sequence: u64,
    candidates: HashMap<String, CandidateInfo>,
}

#[derive(Debug, Clone)]
struct CandidateInfo {
    id: String,
    segment_id: String,
    sequence: u64,
    carrier: String,
}

#[derive(Debug, Clone)]
struct TransitionInfo {
    id: String,
    input_carrier: String,
    output_carrier: String,
}

fn violation(
    violations: &mut Vec<Violation>,
    code: &str,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    violations.push(Violation {
        code: code.to_owned(),
        path: path.into(),
        message: message.into(),
    });
}

fn nonempty_string(value: Option<&Value>) -> Option<&str> {
    value.and_then(Value::as_str).filter(|s| !s.trim().is_empty())
}

fn array(value: Option<&Value>) -> Option<&Vec<Value>> {
    value.and_then(Value::as_array)
}

fn string_set(value: Option<&Value>) -> Vec<String> {
    array(value)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn push_unknown_evidence_reference(
    id: &str,
    path: &str,
    evidence: &BTreeMap<String, String>,
    violations: &mut Vec<Violation>,
) {
    if !evidence.contains_key(id) {
        violation(
            violations,
            "unknown_evidence_reference",
            path,
            format!("Evidence ID '{id}' is not declared in the portfolio evidence catalogue."),
        );
    }
}

fn validate_evidence_refs(
    ids: &[String],
    path: &str,
    evidence: &BTreeMap<String, String>,
    violations: &mut Vec<Violation>,
) {
    for (index, id) in ids.iter().enumerate() {
        push_unknown_evidence_reference(
            id,
            &format!("{path}[{index}]"),
            evidence,
            violations,
        );
    }
}

fn evidence_class_supports(state: &str, classification: &str) -> bool {
    match state {
        "unknown" => false,
        "analyst_scenario" => classification == "analyst_scenario",
        "modelled" => classification == "modelled",
        "published_literature" => classification == "published_literature",
        "manufacturer_declared" => classification == "manufacturer_declared",
        "measured" => matches!(
            classification,
            "measured" | "independently_verified" | "field_qualified"
        ),
        "independently_verified" => {
            matches!(classification, "independently_verified" | "field_qualified")
        }
        "field_qualified" => classification == "field_qualified",
        _ => false,
    }
}

fn validate_evidence_state(
    state: &str,
    ids: &[String],
    path: &str,
    evidence: &BTreeMap<String, String>,
    violations: &mut Vec<Violation>,
) {
    if state == "unknown" {
        if !ids.is_empty() {
            violation(
                violations,
                "unknown_state_has_evidence",
                format!("{path}.evidence_ids"),
                "Unknown claims must not carry evidence IDs that could be mistaken for support.",
            );
        }
        return;
    }

    if ids.is_empty() {
        violation(
            violations,
            "evidence_required_for_claim",
            format!("{path}.evidence_ids"),
            format!("Evidence state '{state}' requires at least one evidence record."),
        );
        return;
    }

    for (index, id) in ids.iter().enumerate() {
        let id_path = format!("{path}.evidence_ids[{index}]");
        match evidence.get(id) {
            None => push_unknown_evidence_reference(id, &id_path, evidence, violations),
            Some(classification) if !evidence_class_supports(state, classification) => {
                violation(
                    violations,
                    "evidence_class_escalation",
                    id_path,
                    format!(
                        "Claim state '{state}' is not supported by evidence class '{classification}'."
                    ),
                );
            }
            Some(_) => {}
        }
    }
}

fn target_carrier_for_service(kind: &str) -> Option<&'static str> {
    match kind {
        "net_ac" => Some("electricity_ac"),
        "regulated_dc" => Some("electricity_dc"),
        "useful_heat" => Some("thermal"),
        "hydrogen_at_delivery" => Some("hydrogen"),
        "ammonia_at_delivery" => Some("ammonia"),
        "propulsion_work" => Some("mechanical"),
        "other" => None,
        _ => None,
    }
}

fn unit_supports_service(kind: &str, unit: &str) -> bool {
    match kind {
        "net_ac" | "regulated_dc" => matches!(unit, "kWh" | "MWh"),
        "useful_heat" => matches!(unit, "kWh_th" | "MWh_th" | "MJ"),
        "hydrogen_at_delivery" => unit == "kg_H2",
        "ammonia_at_delivery" => unit == "kg_NH3",
        "propulsion_work" => matches!(unit, "kWh_mechanical" | "MJ"),
        "other" => true,
        _ => false,
    }
}

fn validate_portfolio(value: &Value) -> Report {
    let mut violations = Vec::new();
    let root = match value.as_object() {
        Some(root) => root,
        None => {
            violation(
                &mut violations,
                "root_not_object",
                "$",
                "Portfolio root must be a JSON object.",
            );
            return Report {
                schema_version: String::new(),
                portfolio_id: String::new(),
                violations,
            };
        }
    };

    let schema_version = nonempty_string(root.get("schema_version"))
        .unwrap_or_default()
        .to_owned();
    let portfolio_id = nonempty_string(root.get("portfolio_id"))
        .unwrap_or_default()
        .to_owned();

    if schema_version != "0.1.0" {
        violation(
            &mut violations,
            "unsupported_schema_version",
            "$.schema_version",
            format!("Expected schema version '0.1.0', found '{schema_version}'."),
        );
    }
    if portfolio_id.is_empty() {
        violation(
            &mut violations,
            "missing_portfolio_id",
            "$.portfolio_id",
            "Portfolio ID must be a non-empty string.",
        );
    }

    // Evidence catalogue: IDs are immutable within one portfolio snapshot.
    let mut evidence = BTreeMap::<String, String>::new();
    match array(root.get("evidence")) {
        Some(records) => {
            for (index, record) in records.iter().enumerate() {
                let path = format!("$.evidence[{index}]");
                let id = nonempty_string(record.get("evidence_id"));
                let classification = nonempty_string(record.get("classification"));
                let (Some(id), Some(classification)) = (id, classification) else {
                    violation(
                        &mut violations,
                        "malformed_evidence_record",
                        path,
                        "Evidence record requires non-empty evidence_id and classification.",
                    );
                    continue;
                };
                const CLASSES: &[&str] = &[
                    "analyst_scenario",
                    "modelled",
                    "published_literature",
                    "manufacturer_declared",
                    "measured",
                    "independently_verified",
                    "field_qualified",
                ];
                if !CLASSES.contains(&classification) {
                    violation(
                        &mut violations,
                        "invalid_evidence_class",
                        format!("{path}.classification"),
                        format!("Unsupported evidence classification '{classification}'."),
                    );
                }
                if evidence
                    .insert(id.to_owned(), classification.to_owned())
                    .is_some()
                {
                    violation(
                        &mut violations,
                        "duplicate_evidence_id",
                        format!("{path}.evidence_id"),
                        format!("Evidence ID '{id}' is not unique."),
                    );
                }
            }
        }
        None => violation(
            &mut violations,
            "missing_evidence_catalogue",
            "$.evidence",
            "Evidence catalogue must be an array.",
        ),
    }

    // Portfolio maturity is itself an evidence-bounded claim.
    if let Some(maturity) = root.get("maturity") {
        let state = nonempty_string(maturity.get("state")).unwrap_or("invalid");
        let maturity_evidence = nonempty_string(maturity.get("evidence_id"));
        if !matches!(
            state,
            "design_only" | "field_reconciled" | "independently_reviewed"
        ) {
            violation(
                &mut violations,
                "invalid_maturity_state",
                "$.maturity.state",
                format!("Unsupported maturity state '{state}'."),
            );
        }
        if let Some(id) = maturity_evidence {
            push_unknown_evidence_reference(
                id,
                "$.maturity.evidence_id",
                &evidence,
                &mut violations,
            );
            if let Some(classification) = evidence.get(id) {
                let adequate = match state {
                    "design_only" => true,
                    "field_reconciled" => classification == "field_qualified",
                    "independently_reviewed" => matches!(
                        classification.as_str(),
                        "independently_verified" | "field_qualified"
                    ),
                    _ => false,
                };
                if !adequate {
                    violation(
                        &mut violations,
                        "maturity_evidence_insufficient",
                        "$.maturity.evidence_id",
                        format!(
                            "Evidence class '{classification}' does not support maturity state '{state}'."
                        ),
                    );
                }
            }
        } else if state != "design_only" {
            violation(
                &mut violations,
                "maturity_evidence_required",
                "$.maturity.evidence_id",
                "Field-reconciled or independently-reviewed maturity requires explicit evidence.",
            );
        }
    } else {
        violation(
            &mut violations,
            "missing_maturity",
            "$.maturity",
            "Portfolio maturity and scope must be declared.",
        );
    }

    // Typed delivered service and chronological demand profile.
    let mut service_kind = String::new();
    let mut target_carrier: Option<&str> = None;
    if let Some(service) = root.get("service") {
        service_kind = nonempty_string(service.get("kind"))
            .unwrap_or_default()
            .to_owned();
        target_carrier = target_carrier_for_service(&service_kind);
        if target_carrier.is_none() && service_kind != "other" {
            violation(
                &mut violations,
                "unsupported_service_kind",
                "$.service.kind",
                format!("Unsupported service kind '{service_kind}'."),
            );
        }
        if nonempty_string(service.get("service_id")).is_none()
            || nonempty_string(service.get("boundary_id")).is_none()
        {
            violation(
                &mut violations,
                "service_identity_missing",
                "$.service",
                "Service ID and boundary ID must be non-empty.",
            );
        }

        match array(service.get("profile")) {
            Some(profile) if !profile.is_empty() => {
                let mut windows = Vec::<(u64, u64, usize)>::new();
                for (index, point) in profile.iter().enumerate() {
                    let path = format!("$.service.profile[{index}]");
                    let start = point.get("window_start_unix_s").and_then(Value::as_u64);
                    let end = point.get("window_end_unix_s").and_then(Value::as_u64);
                    match (start, end) {
                        (Some(start), Some(end)) if end > start => {
                            windows.push((start, end, index));
                        }
                        _ => violation(
                            &mut violations,
                            "invalid_service_window",
                            &path,
                            "Service window must have integer timestamps and end > start.",
                        ),
                    }
                    let amount = point.get("quantity_value").and_then(Value::as_f64);
                    if !matches!(amount, Some(v) if v.is_finite() && v > 0.0) {
                        violation(
                            &mut violations,
                            "invalid_service_quantity",
                            format!("{path}.quantity_value"),
                            "Service quantity must be finite and strictly positive.",
                        );
                    }
                    let unit = nonempty_string(point.get("quantity_unit")).unwrap_or_default();
                    if !unit_supports_service(&service_kind, unit) {
                        violation(
                            &mut violations,
                            "service_unit_incompatible",
                            format!("{path}.quantity_unit"),
                            format!(
                                "Unit '{unit}' is not a declared unit for service kind '{service_kind}'."
                            ),
                        );
                    }
                }
                windows.sort_by_key(|(start, _, index)| (*start, *index));
                for pair in windows.windows(2) {
                    if pair[1].0 < pair[0].1 {
                        violation(
                            &mut violations,
                            "overlapping_service_windows",
                            "$.service.profile",
                            format!(
                                "Profile windows {} and {} overlap.",
                                pair[0].2, pair[1].2
                            ),
                        );
                    }
                }
            }
            _ => violation(
                &mut violations,
                "empty_service_profile",
                "$.service.profile",
                "Service profile must contain at least one interval.",
            ),
        }
    } else {
        violation(
            &mut violations,
            "missing_service",
            "$.service",
            "A typed delivered-service contract is required.",
        );
    }

    // Route segments and candidate catalogue.
    let mut segments = Vec::<SegmentInfo>::new();
    let mut segment_ids = HashSet::<String>::new();
    let mut segment_sequences = HashSet::<u64>::new();
    let mut global_candidate_ids = HashSet::<String>::new();

    if let Some(route) = root.get("route") {
        if nonempty_string(route.get("route_id")).is_none() {
            violation(
                &mut violations,
                "missing_route_id",
                "$.route.route_id",
                "Route ID must be a non-empty string.",
            );
        }
        match array(route.get("segments")) {
            Some(items) if !items.is_empty() => {
                for (index, item) in items.iter().enumerate() {
                    let path = format!("$.route.segments[{index}]");
                    let id = nonempty_string(item.get("segment_id"))
                        .unwrap_or_default()
                        .to_owned();
                    let sequence = item.get("sequence").and_then(Value::as_u64);
                    if id.is_empty() {
                        violation(
                            &mut violations,
                            "missing_segment_id",
                            format!("{path}.segment_id"),
                            "Segment ID must be non-empty.",
                        );
                    } else if !segment_ids.insert(id.clone()) {
                        violation(
                            &mut violations,
                            "duplicate_segment_id",
                            format!("{path}.segment_id"),
                            format!("Segment ID '{id}' is duplicated."),
                        );
                    }
                    let sequence = match sequence {
                        Some(sequence) => {
                            if !segment_sequences.insert(sequence) {
                                violation(
                                    &mut violations,
                                    "duplicate_segment_sequence",
                                    format!("{path}.sequence"),
                                    format!("Segment sequence {sequence} is duplicated."),
                                );
                            }
                            sequence
                        }
                        None => {
                            violation(
                                &mut violations,
                                "invalid_segment_sequence",
                                format!("{path}.sequence"),
                                "Segment sequence must be a non-negative integer.",
                            );
                            index as u64
                        }
                    };

                    let mut candidates = HashMap::new();
                    match array(item.get("candidates")) {
                        Some(candidate_items) if !candidate_items.is_empty() => {
                            for (candidate_index, candidate) in candidate_items.iter().enumerate()
                            {
                                let candidate_path =
                                    format!("{path}.candidates[{candidate_index}]");
                                let candidate_id = nonempty_string(candidate.get("candidate_id"))
                                    .unwrap_or_default()
                                    .to_owned();
                                let carrier = nonempty_string(candidate.get("carrier"))
                                    .unwrap_or_default()
                                    .to_owned();
                                let state = nonempty_string(candidate.get("evidence_state"))
                                    .unwrap_or("invalid");
                                let evidence_ids =
                                    string_set(candidate.get("evidence_ids"));
                                validate_evidence_refs(
                                    &evidence_ids,
                                    &format!("{candidate_path}.evidence_ids"),
                                    &evidence,
                                    &mut violations,
                                );
                                validate_evidence_state(
                                    state,
                                    &evidence_ids,
                                    &candidate_path,
                                    &evidence,
                                    &mut violations,
                                );

                                if candidate_id.is_empty() {
                                    violation(
                                        &mut violations,
                                        "missing_candidate_id",
                                        format!("{candidate_path}.candidate_id"),
                                        "Candidate ID must be non-empty.",
                                    );
                                } else if !global_candidate_ids.insert(candidate_id.clone()) {
                                    violation(
                                        &mut violations,
                                        "duplicate_candidate_id",
                                        format!("{candidate_path}.candidate_id"),
                                        format!(
                                            "Candidate ID '{candidate_id}' must be unique across the route."
                                        ),
                                    );
                                }
                                if carrier.is_empty() {
                                    violation(
                                        &mut violations,
                                        "missing_candidate_carrier",
                                        format!("{candidate_path}.carrier"),
                                        "Candidate carrier must be non-empty.",
                                    );
                                }
                                if !candidate_id.is_empty() && !carrier.is_empty() {
                                    candidates.insert(
                                        candidate_id.clone(),
                                        CandidateInfo {
                                            id: candidate_id,
                                            segment_id: id.clone(),
                                            sequence,
                                            carrier,
                                        },
                                    );
                                }
                            }
                        }
                        _ => violation(
                            &mut violations,
                            "empty_candidate_catalogue",
                            format!("{path}.candidates"),
                            "Each route segment requires at least one candidate.",
                        ),
                    }
                    segments.push(SegmentInfo {
                        id,
                        sequence,
                        candidates,
                    });
                }
                segments.sort_by_key(|segment| segment.sequence);
                for (index, segment) in segments.iter().enumerate() {
                    if segment.sequence != index as u64 {
                        violation(
                            &mut violations,
                            "non_contiguous_segment_sequence",
                            "$.route.segments",
                            format!(
                                "Expected sequence {index}, found {} for segment '{}'.",
                                segment.sequence, segment.id
                            ),
                        );
                    }
                }
            }
            _ => violation(
                &mut violations,
                "empty_route_segments",
                "$.route.segments",
                "Route must contain at least one segment.",
            ),
        }
    } else {
        violation(
            &mut violations,
            "missing_route",
            "$.route",
            "Route definition is required.",
        );
    }

    // Each design is a complete route selection plus explicit carrier transitions.
    if let Some(route) = root.get("route") {
        match array(route.get("designs")) {
            Some(designs) if !designs.is_empty() => {
                let mut design_ids = HashSet::<String>::new();
                for (design_index, design) in designs.iter().enumerate() {
                    let path = format!("$.route.designs[{design_index}]");
                    let design_id = nonempty_string(design.get("design_id"))
                        .unwrap_or_default()
                        .to_owned();
                    if design_id.is_empty() {
                        violation(
                            &mut violations,
                            "missing_design_id",
                            format!("{path}.design_id"),
                            "Design ID must be non-empty.",
                        );
                    } else if !design_ids.insert(design_id.clone()) {
                        violation(
                            &mut violations,
                            "duplicate_design_id",
                            format!("{path}.design_id"),
                            format!("Design ID '{design_id}' is duplicated."),
                        );
                    }

                    let design_evidence = string_set(design.get("evidence_ids"));
                    validate_evidence_refs(
                        &design_evidence,
                        &format!("{path}.evidence_ids"),
                        &evidence,
                        &mut violations,
                    );

                    let mut selected = BTreeMap::<u64, CandidateInfo>::new();
                    let mut seen_choice_segments = HashSet::<String>::new();
                    let choices = array(design.get("segment_choices"));
                    match choices {
                        Some(choices) => {
                            if choices.len() != segments.len() {
                                violation(
                                    &mut violations,
                                    "incomplete_segment_selection",
                                    format!("{path}.segment_choices"),
                                    format!(
                                        "Design selects {} segments, but route contains {}.",
                                        choices.len(),
                                        segments.len()
                                    ),
                                );
                            }
                            let mut last_sequence: Option<u64> = None;
                            for (choice_index, choice) in choices.iter().enumerate() {
                                let choice_path =
                                    format!("{path}.segment_choices[{choice_index}]");
                                let segment_id = nonempty_string(choice.get("segment_id"))
                                    .unwrap_or_default()
                                    .to_owned();
                                let candidate_id = nonempty_string(choice.get("candidate_id"))
                                    .unwrap_or_default();
                                if !seen_choice_segments.insert(segment_id.clone()) {
                                    violation(
                                        &mut violations,
                                        "duplicate_segment_choice",
                                        format!("{choice_path}.segment_id"),
                                        format!("Segment '{segment_id}' is selected more than once."),
                                    );
                                }
                                match segments.iter().find(|s| s.id == segment_id) {
                                    None => violation(
                                        &mut violations,
                                        "unknown_segment_choice",
                                        format!("{choice_path}.segment_id"),
                                        format!("Segment '{segment_id}' is not declared in this route."),
                                    ),
                                    Some(segment) => {
                                        if let Some(previous) = last_sequence {
                                            if segment.sequence < previous {
                                                violation(
                                                    &mut violations,
                                                    "segment_choices_out_of_order",
                                                    &path,
                                                    "Segment choices must follow route sequence.",
                                                );
                                            }
                                        }
                                        last_sequence = Some(segment.sequence);
                                        match segment.candidates.get(candidate_id.unwrap_or_default())
                                        {
                                            None => violation(
                                                &mut violations,
                                                "candidate_not_in_selected_segment",
                                                format!("{choice_path}.candidate_id"),
                                                format!(
                                                    "Candidate '{}' is not declared for segment '{}'.",
                                                    candidate_id.unwrap_or_default(),
                                                    segment_id
                                                ),
                                            ),
                                            Some(candidate) => {
                                                selected.insert(segment.sequence, candidate.clone());
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        None => violation(
                            &mut violations,
                            "missing_segment_choices",
                            format!("{path}.segment_choices"),
                            "Design requires a segment_choices array.",
                        ),
                    }

                    let source_boundary = design.get("source_boundary");
                    let source_carrier = source_boundary
                        .and_then(|b| nonempty_string(b.get("carrier")));
                    let source_evidence = string_set(
                        source_boundary.and_then(|b| b.get("evidence_ids")),
                    );
                    validate_evidence_refs(
                        &source_evidence,
                        &format!("{path}.source_boundary.evidence_ids"),
                        &evidence,
                        &mut violations,
                    );
                    if let Some(first) = selected.get(&0) {
                        match source_carrier {
                            Some(carrier) if carrier == first.carrier => {}
                            Some(carrier) => violation(
                                &mut violations,
                                "source_carrier_mismatch",
                                format!("{path}.source_boundary.carrier"),
                                format!(
                                    "Source carrier '{carrier}' does not match first segment carrier '{}'.",
                                    first.carrier
                                ),
                            ),
                            None => violation(
                                &mut violations,
                                "missing_source_carrier",
                                format!("{path}.source_boundary.carrier"),
                                "Source boundary must declare the carrier entering the first segment.",
                            ),
                        }
                    }

                    let mut transitions = BTreeMap::<u64, Vec<TransitionInfo>>::new();
                    let transition_values = array(design.get("transitions"));
                    let mut transition_ids = HashSet::<String>::new();
                    if let Some(items) = transition_values {
                        let mut previous_sequence: Option<u64> = None;
                        for (transition_index, transition) in items.iter().enumerate() {
                            let transition_path =
                                format!("{path}.transitions[{transition_index}]");
                            let id = nonempty_string(transition.get("transition_id"))
                                .unwrap_or_default()
                                .to_owned();
                            if id.is_empty() {
                                violation(
                                    &mut violations,
                                    "missing_transition_id",
                                    format!("{transition_path}.transition_id"),
                                    "Transition ID must be non-empty.",
                                );
                            } else if !transition_ids.insert(id.clone()) {
                                violation(
                                    &mut violations,
                                    "duplicate_transition_id",
                                    format!("{transition_path}.transition_id"),
                                    format!("Transition ID '{id}' is duplicated."),
                                );
                            }
                            let sequence = transition
                                .get("after_segment_sequence")
                                .and_then(Value::as_u64);
                            let sequence = match sequence {
                                Some(sequence) => sequence,
                                None => {
                                    violation(
                                        &mut violations,
                                        "invalid_transition_sequence",
                                        format!("{transition_path}.after_segment_sequence"),
                                        "Transition sequence must be a non-negative integer.",
                                    );
                                    continue;
                                }
                            };
                            if previous_sequence.is_some_and(|previous| sequence < previous) {
                                violation(
                                    &mut violations,
                                    "transitions_out_of_order",
                                    format!("{path}.transitions"),
                                    "Transitions must be ordered by after_segment_sequence.",
                                );
                            }
                            previous_sequence = Some(sequence);
                            if !segments.iter().any(|s| s.sequence == sequence) {
                                violation(
                                    &mut violations,
                                    "transition_sequence_out_of_range",
                                    format!("{transition_path}.after_segment_sequence"),
                                    format!("No route segment has sequence {sequence}."),
                                );
                            }
                            let input = nonempty_string(transition.get("input_carrier"))
                                .unwrap_or_default()
                                .to_owned();
                            let output = nonempty_string(transition.get("output_carrier"))
                                .unwrap_or_default()
                                .to_owned();
                            let state = nonempty_string(transition.get("evidence_state"))
                                .unwrap_or("invalid");
                            let evidence_ids =
                                string_set(transition.get("evidence_ids"));
                            validate_evidence_refs(
                                &evidence_ids,
                                &format!("{transition_path}.evidence_ids"),
                                &evidence,
                                &mut violations,
                            );
                            validate_evidence_state(
                                state,
                                &evidence_ids,
                                &transition_path,
                                &evidence,
                                &mut violations,
                            );
                            transitions.entry(sequence).or_default().push(TransitionInfo {
                                id,
                                input_carrier: input,
                                output_carrier: output,
                            });
                        }
                    } else {
                        violation(
                            &mut violations,
                            "missing_transitions",
                            format!("{path}.transitions"),
                            "Design requires a transitions array (which may be empty).",
                        );
                    }

                    for (position, segment) in segments.iter().enumerate() {
                        let Some(start_candidate) = selected.get(&segment.sequence) else {
                            continue;
                        };
                        let target = if position + 1 < segments.len() {
                            selected
                                .get(&segments[position + 1].sequence)
                                .map(|candidate| candidate.carrier.as_str())
                        } else {
                            target_carrier
                        };
                        let Some(target) = target else {
                            continue;
                        };
                        let chain = transitions
                            .get(&segment.sequence)
                            .cloned()
                            .unwrap_or_default();
                        let mut current = start_candidate.carrier.as_str();
                        for transition in &chain {
                            if transition.input_carrier != current {
                                violation(
                                    &mut violations,
                                    "broken_carrier_transition_chain",
                                    format!("{path}.transitions"),
                                    format!(
                                        "Transition '{}' expects '{}', but path currently carries '{}'.",
                                        transition.id, transition.input_carrier, current
                                    ),
                                );
                            }
                            current = transition.output_carrier.as_str();
                        }
                        if current != target {
                            violation(
                                &mut violations,
                                "missing_or_incomplete_carrier_transition",
                                format!("{path}.transitions"),
                                format!(
                                    "After segment {}, carrier is '{}', but next segment/service requires '{}'.",
                                    segment.sequence, current, target
                                ),
                            );
                        }
                    }
                }
            }
            _ => violation(
                &mut violations,
                "empty_route_designs",
                "$.route.designs",
                "At least one complete route design is required.",
            ),
        }
    }

    violations.sort_by(|a, b| {
        (&a.code, &a.path, &a.message).cmp(&(&b.code, &b.path, &b.message))
    });

    Report {
        schema_version,
        portfolio_id,
        violations,
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!(
            "Usage: grid-transmission-portfolio-validator <portfolio.json> [--json]"
        );
        return ExitCode::from(2);
    };
    let json_output = args.any(|arg| arg == "--json");
    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("Could not read '{path}': {error}");
            return ExitCode::from(2);
        }
    };
    let value: Value = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Invalid JSON in '{path}': {error}");
            return ExitCode::from(2);
        }
    };

    let report = validate_portfolio(&value);
    if json_output {
        println!("{}", report.to_json());
    } else if report.is_valid() {
        println!(
            "VALID: portfolio '{}' passed v{} semantic route checks.",
            report.portfolio_id, report.schema_version
        );
    } else {
        println!(
            "INVALID: portfolio '{}' has {} semantic violation(s).",
            report.portfolio_id,
            report.violations.len()
        );
        for item in &report.violations {
            println!("- {} at {}: {}", item.code, item.path, item.message);
        }
    }

    if report.is_valid() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!(
        "../../../docs/research/grid-transmission-portfolio/data/synthetic-route-portfolio-v0.1.json"
    );

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("synthetic fixture is valid JSON")
    }

    fn has_code(report: &Report, code: &str) -> bool {
        report.violations.iter().any(|v| v.code == code)
    }

    #[test]
    fn accepts_complete_ac_and_hybrid_dc_routes() {
        let report = validate_portfolio(&fixture());
        assert!(report.is_valid(), "{:?}", report.violations);
    }

    #[test]
    fn rejects_missing_segment_selection() {
        let mut value = fixture();
        value["route"]["designs"][0]["segment_choices"]
            .as_array_mut()
            .expect("choices")
            .pop();
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "incomplete_segment_selection"));
    }

    #[test]
    fn rejects_candidate_from_another_segment() {
        let mut value = fixture();
        value["route"]["designs"][0]["segment_choices"][1]["candidate_id"] =
            json!("seg0-hvac-overhead");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "candidate_not_in_selected_segment"));
    }

    #[test]
    fn rejects_missing_dc_to_ac_terminal_conversion() {
        let mut value = fixture();
        value["route"]["designs"][1]["transitions"] = json!([]);
        let report = validate_portfolio(&value);
        assert!(has_code(
            &report,
            "missing_or_incomplete_carrier_transition"
        ));
    }

    #[test]
    fn rejects_broken_conversion_chain() {
        let mut value = fixture();
        value["route"]["designs"][1]["transitions"][0]["input_carrier"] =
            json!("electricity_ac");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "broken_carrier_transition_chain"));
    }

    #[test]
    fn rejects_claim_escalation_from_scenario_to_independent() {
        let mut value = fixture();
        value["route"]["segments"][0]["candidates"][0]["evidence_state"] =
            json!("independently_verified");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "evidence_class_escalation"));
    }

    #[test]
    fn rejects_unknown_claim_with_attached_evidence() {
        let mut value = fixture();
        value["route"]["segments"][0]["candidates"][0]["evidence_state"] =
            json!("unknown");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "unknown_state_has_evidence"));
    }

    #[test]
    fn rejects_electrical_service_with_hydrogen_mass_unit() {
        let mut value = fixture();
        value["service"]["profile"][0]["quantity_unit"] = json!("kg_H2");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "service_unit_incompatible"));
    }

    #[test]
    fn rejects_duplicate_segment_sequence() {
        let mut value = fixture();
        value["route"]["segments"][1]["sequence"] = json!(0);
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "duplicate_segment_sequence"));
    }

    #[test]
    fn rejects_unresolved_evidence_reference() {
        let mut value = fixture();
        value["route"]["designs"][0]["source_boundary"]["evidence_ids"] =
            json!(["missing-evidence"]);
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "unknown_evidence_reference"));
    }

    #[test]
    fn rejects_overlapping_service_windows() {
        let mut value = fixture();
        value["service"]["profile"].as_array_mut().unwrap().push(json!({
            "window_start_unix_s": 1760001000,
            "window_end_unix_s": 1760007200,
            "quantity_value": 1.0,
            "quantity_unit": "kWh"
        }));
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "overlapping_service_windows"));
    }

    #[test]
    fn rejects_non_field_evidence_for_field_reconciled_maturity() {
        let mut value = fixture();
        value["maturity"]["state"] = json!("field_reconciled");
        let report = validate_portfolio(&value);
        assert!(has_code(&report, "maturity_evidence_insufficient"));
    }

    #[test]
    fn report_order_is_deterministic() {
        let mut value = fixture();
        value["route"]["designs"][1]["transitions"] = json!([]);
        value["service"]["profile"][0]["quantity_unit"] = json!("kg_H2");
        let a = validate_portfolio(&value).to_json().to_string();
        let b = validate_portfolio(&value).to_json().to_string();
        assert_eq!(a, b);
    }
}
