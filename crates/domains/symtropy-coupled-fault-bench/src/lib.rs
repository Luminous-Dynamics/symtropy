//! Higher-order fault combination and additive-null interaction screening.
//!
//! Combined-state responses must be recorded independently. The additive
//! differential envelope is a screening null model, not a physical law.
//! Deviation from it is not, by itself, proof of a physical interaction.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const TOL: f64 = 1.0e-9;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InteractionResult {
    pub combination_id: String,
    pub channel_id: String,
    pub status: String,
    pub null_lower: Option<f64>,
    pub null_upper: Option<f64>,
    pub combined_lower: Option<f64>,
    pub combined_upper: Option<f64>,
    pub active_fault_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombinationPairResult {
    pub pair_id: String,
    pub combination_a_id: String,
    pub combination_b_id: String,
    pub required_independent_principles: usize,
    pub observed_independent_principles: usize,
    pub discriminating_channel_ids: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub fault_count: usize,
    pub combination_count: usize,
    pub response_count: usize,
    pub interaction_results: Vec<InteractionResult>,
    pub deviation_observed_count: usize,
    pub consistent_with_null_count: usize,
    pub insufficient_interaction_evidence_count: usize,
    pub combination_pair_results: Vec<CombinationPairResult>,
    pub limitations: Vec<String>,
}

fn push(errors: &mut Vec<Violation>, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(Violation {
        code: code.to_owned(),
        path: path.to_owned(),
        detail: detail.into(),
    });
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|n| n.is_finite())
}

fn same_domain(a: &Value, b: &Value) -> bool {
    ["material_cohort_ref", "path_configuration_ref", "geometry_profile_ref"]
        .iter()
        .all(|key| text(a, key).is_some() && text(a, key) == text(b, key))
}

fn response_role_admissible(response: &Value, evidence_class: &str) -> bool {
    match (evidence_class, text(response, "evidence_role")) {
        ("SYNTHETIC_CONFORMANCE_FIXTURE", Some("SYNTHETIC_CONFORMANCE_FIXTURE")) => true,
        ("MEASURED", Some("INDEPENDENT_MEASUREMENT")) => true,
        _ => false,
    }
}

fn expected_faulted_unavailability(
    combination: &Value,
    channel_id: &str,
    response: &Value,
) -> bool {
    combination.get("faulted_channel_ids")
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(channel_id)))
        && matches!(text(response, "availability"), Some("DEGRADED" | "MISSING"))
}

/// Validate the exact parent observability identities and all combined-state
/// fault-set/response references.
pub fn validate_case(case: &Value, observability: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    let evidence_class = text(case, "evidence_class").unwrap_or("");
    if !matches!(evidence_class, "SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED") {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class must be synthetic conformance or measured");
    }
    if text(case, "observability_case_id") != text(observability, "case_id") {
        push(&mut errors, "OBSERVABILITY_CASE_ID_MISMATCH", "$.observability_case_id", "case must bind to the exact supplied singleton observability matrix");
    }
    if evidence_class == "MEASURED"
        && text(observability, "evidence_class") != Some("MEASURED")
    {
        push(&mut errors, "MEASURED_CASE_HAS_SYNTHETIC_SINGLETON_PARENT", "$.observability_case_id", "measured interaction claims require measured singleton and baseline parent evidence");
    }

    let Some(parent_context) = case.get("parent_context") else {
        push(&mut errors, "MISSING_PARENT_CONTEXT", "$.parent_context", "exact #4005 parent identity is required");
        return errors;
    };
    let Some(parent_observability_context) = observability.get("parent_context") else {
        push(&mut errors, "PARENT_OBSERVABILITY_CONTEXT_MISSING", "$.observability.parent_context", "singleton observability parent context is required");
        return errors;
    };
    for key in [
        "parent_case_id",
        "parent_interval_id",
        "trial_id",
        "material_cohort_ref",
        "machine_configuration_ref",
        "transport_configuration_ref",
    ] {
        if text(parent_context, key).is_none()
            || text(parent_context, key) != text(parent_observability_context, key)
        {
            push(&mut errors, "PARENT_CONTEXT_MISMATCH", &format!("$.parent_context.{key}"), "combined-fault case must bind to exact singleton parent context");
        }
    }

    if text(case, "baseline_state_id").is_none_or(|id| {
        !observability["states"].as_array().into_iter().flatten().any(|state| text(state, "state_id") == Some(id))
    }) {
        push(&mut errors, "UNKNOWN_BASELINE_STATE", "$.baseline_state_id", "baseline state must exist in the singleton observability case");
    }

    let Some(null_model) = case.get("null_model") else {
        push(&mut errors, "MISSING_NULL_MODEL", "$.null_model", "explicit additive differential null model is required");
        return errors;
    };
    if text(null_model, "model_kind") != Some("ADDITIVE_DIFFERENTIAL_SCREENING_ONLY") {
        push(&mut errors, "UNSUPPORTED_NULL_MODEL", "$.null_model.model_kind", "v0.1 only supports an explicitly labelled additive differential screening null");
    }
    for key in ["equation_ref", "assumption_ref", "dependence_note_ref", "source_ref"] {
        if text(null_model, key).is_none() {
            push(&mut errors, "MISSING_NULL_MODEL_PROVENANCE", &format!("$.null_model.{key}"), "null-model equation, assumption, dependence note and source are required");
        }
    }

    let Some(channels) = observability.get("channels").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_PARENT_CHANNELS", "$.observability.channels", "parent channels are required");
        return errors;
    };
    let channel_map: BTreeMap<&str, &Value> = channels.iter()
        .filter_map(|channel| text(channel, "channel_id").map(|id| (id, channel)))
        .collect();
    let state_map: BTreeMap<&str, &Value> = observability["states"].as_array().into_iter().flatten()
        .filter_map(|state| text(state, "state_id").map(|id| (id, state)))
        .collect();
    let singleton_responses: BTreeMap<(&str, &str), &Value> = observability["responses"].as_array().into_iter().flatten()
        .filter_map(|response| {
            Some((
                (text(response, "state_id")?, text(response, "channel_id")?),
                response,
            ))
        })
        .collect();

    let Some(faults) = case.get("fault_catalog").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_FAULT_CATALOG", "$.fault_catalog", "fault catalog must be an array");
        return errors;
    };
    if faults.len() < 2 {
        push(&mut errors, "FAULT_CATALOG_TOO_SMALL", "$.fault_catalog", "at least two fault hypotheses are required");
    }
    let mut fault_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, fault) in faults.iter().enumerate() {
        let path = format!("$.fault_catalog[{index}]");
        let Some(id) = text(fault, "fault_id") else {
            push(&mut errors, "MISSING_FAULT_ID", &format!("{path}.fault_id"), "fault_id is required");
            continue;
        };
        if fault_map.insert(id.to_owned(), fault).is_some() {
            push(&mut errors, "DUPLICATE_FAULT_ID", &format!("{path}.fault_id"), "fault IDs must be unique");
        }
        let singleton_state_id = text(fault, "singleton_state_id");
        if singleton_state_id.is_none_or(|state_id| !state_map.contains_key(state_id)) {
            push(&mut errors, "UNKNOWN_SINGLETON_FAULT_STATE", &format!("{path}.singleton_state_id"), "fault must map to a declared singleton state in #4005");
        }
        if !matches!(text(fault, "fault_layer"), Some("PROCESS" | "SENSOR" | "COMMON_CAUSE")) {
            push(&mut errors, "INVALID_FAULT_LAYER", &format!("{path}.fault_layer"), "fault layer must be PROCESS, SENSOR or COMMON_CAUSE");
        }
        if text(fault, "causal_group_ref").is_none() || text(fault, "source_ref").is_none() {
            push(&mut errors, "MISSING_FAULT_PROVENANCE", &path, "causal group and source references are required");
        }
        let affected: Vec<&str> = fault.get("affected_channel_ids").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if text(fault, "fault_layer") == Some("SENSOR") && affected.is_empty() {
            push(&mut errors, "SENSOR_FAULT_WITHOUT_AFFECTED_CHANNEL", &format!("{path}.affected_channel_ids"), "sensor faults must identify affected channels");
        }
        for channel_id in affected {
            if !channel_map.contains_key(channel_id) {
                push(&mut errors, "UNKNOWN_FAULT_AFFECTED_CHANNEL", &format!("{path}.affected_channel_ids"), "affected channel must exist in parent observability layout");
            }
        }
    }

    let Some(combinations) = case.get("combined_states").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_COMBINED_STATES", "$.combined_states", "combined states must be an array");
        return errors;
    };
    if combinations.is_empty() {
        push(&mut errors, "EMPTY_COMBINED_STATES", "$.combined_states", "at least one combined fault state is required");
    }
    let mut combination_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, combination) in combinations.iter().enumerate() {
        let path = format!("$.combined_states[{index}]");
        let Some(id) = text(combination, "combination_id") else {
            push(&mut errors, "MISSING_COMBINATION_ID", &format!("{path}.combination_id"), "combination_id is required");
            continue;
        };
        if combination_map.insert(id.to_owned(), combination).is_some() {
            push(&mut errors, "DUPLICATE_COMBINATION_ID", &format!("{path}.combination_id"), "combination IDs must be unique");
        }
        let active: Vec<&str> = combination.get("active_fault_ids").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if active.len() < 2 {
            push(&mut errors, "COMBINATION_REQUIRES_MULTIPLE_FAULTS", &format!("{path}.active_fault_ids"), "combined state must contain at least two faults");
        }
        if active.iter().collect::<BTreeSet<_>>().len() != active.len() {
            push(&mut errors, "DUPLICATE_ACTIVE_FAULT_ID", &format!("{path}.active_fault_ids"), "active fault IDs must be unique");
        }
        let mut exclusive_groups = BTreeSet::new();
        let mut derived_faulted_channels = BTreeSet::new();
        for fault_id in &active {
            let Some(fault) = fault_map.get(*fault_id).copied() else {
                push(&mut errors, "UNKNOWN_ACTIVE_FAULT_ID", &format!("{path}.active_fault_ids"), format!("unknown active fault {fault_id}"));
                continue;
            };
            if let Some(group) = text(fault, "mutually_exclusive_group_ref") {
                if !exclusive_groups.insert(group.to_owned()) {
                    push(&mut errors, "CONTRADICTORY_ACTIVE_FAULTS", &format!("{path}.active_fault_ids"), format!("multiple active faults share mutually-exclusive group {group}"));
                }
            }
            for channel_id in fault.get("affected_channel_ids").and_then(Value::as_array).into_iter().flatten() {
                if let Some(channel_id) = channel_id.as_str() {
                    derived_faulted_channels.insert(channel_id.to_owned());
                }
            }
        }
        let declared_faulted: BTreeSet<String> = combination.get("faulted_channel_ids").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).map(str::to_owned).collect();
        if declared_faulted != derived_faulted_channels {
            push(&mut errors, "FAULTED_CHANNEL_SET_MISMATCH", &format!("{path}.faulted_channel_ids"), "combined-state faulted channel list must equal the union of the active fault definitions");
        }
        let onset_relation = text(combination, "onset_relation").unwrap_or("");
        let onset_order: Vec<&str> = combination.get("onset_order").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if onset_relation == "KNOWN_ORDER" {
            if onset_order.len() != active.len()
                || onset_order.iter().collect::<BTreeSet<_>>() != active.iter().collect::<BTreeSet<_>>()
            {
                push(&mut errors, "INVALID_FAULT_ONSET_ORDER", &format!("{path}.onset_order"), "known onset order must contain every active fault exactly once");
            }
        } else if !onset_order.is_empty() {
            push(&mut errors, "UNEXPECTED_FAULT_ONSET_ORDER", &format!("{path}.onset_order"), "onset order must be empty when onset relation is simultaneous or unknown");
        }
        if onset_relation == "KNOWN_ORDER" && text(combination, "combination_type") == Some("CO_OCCURRENCE") {
            // Co-occurrence is compatible with a recorded order; it simply
            // means the events overlap, not that one causes the other.
        }
        if !matches!(text(combination, "combination_type"), Some("CO_OCCURRENCE" | "CAUSAL_CHAIN" | "COMMON_CAUSE")) {
            push(&mut errors, "INVALID_COMBINATION_TYPE", &format!("{path}.combination_type"), "combination type must distinguish co-occurrence, causal chain and common cause");
        }
        if text(combination, "combination_type") == Some("CAUSAL_CHAIN")
            && onset_relation != "KNOWN_ORDER"
        {
            push(&mut errors, "CAUSAL_CHAIN_REQUIRES_KNOWN_ORDER", &path, "causal chains require an explicit known fault onset order");
        }
        if !matches!(onset_relation, "SIMULTANEOUS" | "KNOWN_ORDER" | "UNKNOWN_ORDER") {
            push(&mut errors, "INVALID_ONSET_RELATION", &format!("{path}.onset_relation"), "onset relation must be explicit");
        }
        if text(combination, "evidence_class") != Some(evidence_class) {
            push(&mut errors, "COMBINATION_EVIDENCE_CLASS_MISMATCH", &format!("{path}.evidence_class"), "combination evidence class must match case evidence class");
        }
        for key in ["event_window_ref", "adjudication_ref"] {
            if text(combination, key).is_none() {
                push(&mut errors, "MISSING_COMBINATION_PROVENANCE", &format!("{path}.{key}"), "event window and adjudication are required");
            }
        }
    }

    let Some(responses) = case.get("responses").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_COMBINED_RESPONSES", "$.responses", "combined response matrix is required");
        return errors;
    };
    let mut response_map: BTreeMap<(String, String), &Value> = BTreeMap::new();
    let mut response_sources = BTreeSet::new();
    for (index, response) in responses.iter().enumerate() {
        let path = format!("$.responses[{index}]");
        let combination_id = text(response, "combination_id");
        let channel_id = text(response, "channel_id");
        let (Some(combination_id), Some(channel_id)) = (combination_id, channel_id) else {
            push(&mut errors, "MISSING_COMBINED_RESPONSE_IDENTITY", &path, "response must identify a combination and channel");
            continue;
        };
        let Some(combination) = combination_map.get(combination_id).copied() else {
            push(&mut errors, "UNKNOWN_RESPONSE_COMBINATION", &format!("{path}.combination_id"), "response combination is not declared");
            continue;
        };
        let Some(channel) = channel_map.get(channel_id).copied() else {
            push(&mut errors, "UNKNOWN_RESPONSE_CHANNEL", &format!("{path}.channel_id"), "response channel is not in parent observability layout");
            continue;
        };
        if response_map.insert((combination_id.to_owned(), channel_id.to_owned()), response).is_some() {
            push(&mut errors, "DUPLICATE_COMBINED_RESPONSE", &path, "one combined response per combination/channel is permitted");
        }
        if text(response, "unit") != text(channel, "unit") {
            push(&mut errors, "COMBINED_RESPONSE_UNIT_MISMATCH", &format!("{path}.unit"), "combined response unit must match the singleton channel unit");
        }
        if !same_domain(response.get("domain").unwrap_or(&Value::Null), channel.get("domain").unwrap_or(&Value::Null)) {
            push(&mut errors, "COMBINED_RESPONSE_DOMAIN_MISMATCH", &format!("{path}.domain"), "combined response domain must match exact singleton channel material/path/geometry domain");
        }
        if text(response, "measurement_window_ref") != text(combination, "event_window_ref") {
            push(&mut errors, "COMBINED_RESPONSE_WINDOW_MISMATCH", &format!("{path}.measurement_window_ref"), "all combined response channels must use the combination's exact synchronized event window");
        }
        for key in ["source_ref", "adjudication_ref"] {
            if text(response, key).is_none() {
                push(&mut errors, "MISSING_COMBINED_RESPONSE_PROVENANCE", &format!("{path}.{key}"), "combined response requires source and adjudication");
            }
        }
        if let Some(source) = text(response, "source_ref") {
            if !response_sources.insert(source.to_owned()) {
                push(&mut errors, "DUPLICATE_COMBINED_RESPONSE_SOURCE", &format!("{path}.source_ref"), "every independently collected response requires a unique source reference");
            }
        }
        match text(response, "availability") {
            Some("VALID") => {
                match (finite(response, "value_lower"), finite(response, "value_upper")) {
                    (Some(lower), Some(upper)) if upper >= lower => {}
                    _ => push(&mut errors, "INVALID_COMBINED_RESPONSE_INTERVAL", &path, "VALID combined response requires finite ordered interval bounds"),
                }
                if !matches!(
                    text(response, "evidence_role"),
                    Some("INDEPENDENT_MEASUREMENT" | "SYNTHETIC_CONFORMANCE_FIXTURE")
                ) {
                    push(&mut errors, "NONMEASUREMENT_COMBINED_RESPONSE", &format!("{path}.evidence_role"), "model/operator output cannot establish a combined-state physical response");
                }
            }
            Some("DEGRADED" | "MISSING" | "UNKNOWN") => {
                if !matches!(response.get("value_lower"), Some(Value::Null))
                    || !matches!(response.get("value_upper"), Some(Value::Null))
                {
                    push(&mut errors, "UNAVAILABLE_COMBINED_RESPONSE_HAS_VALUES", &path, "unavailable combined response cannot carry numeric values");
                }
            }
            _ => push(&mut errors, "INVALID_COMBINED_RESPONSE_AVAILABILITY", &format!("{path}.availability"), "availability must be VALID, DEGRADED, MISSING or UNKNOWN"),
        }
        let declared_faulted: BTreeSet<&str> = combination.get("faulted_channel_ids").and_then(Value::as_array).into_iter().flatten()
            .filter_map(Value::as_str).collect();
        if declared_faulted.contains(channel_id)
            && !matches!(text(response, "availability"), Some("DEGRADED" | "MISSING"))
        {
            push(&mut errors, "FAULTED_CHANNEL_REPORTED_HEALTHY", &path, "a channel affected by an active sensor fault cannot report a healthy numeric response");
        }
        if evidence_class == "MEASURED" {
            if text(response, "evidence_role") != Some("INDEPENDENT_MEASUREMENT") {
                push(&mut errors, "MEASURED_CASE_HAS_NONMEASURED_RESPONSE", &path, "measured combined-fault cases require independent measurements, not model/synthetic/operator responses");
            }
        }
    }

    for combination_id in combination_map.keys() {
        for channel_id in channel_map.keys() {
            if !response_map.contains_key(&(combination_id.clone(), channel_id.to_string())) {
                push(&mut errors, "INCOMPLETE_COMBINED_RESPONSE_MATRIX", "$.responses", format!("missing response for {combination_id} and {channel_id}"));
            }
        }
    }

    if evidence_class == "MEASURED" {
        for fault in faults {
            let singleton_id = text(fault, "singleton_state_id").unwrap_or("");
            for channel_id in channel_map.keys() {
                match singleton_responses.get(&(singleton_id, channel_id)) {
                    Some(response) if text(response, "evidence_role") == Some("INDEPENDENT_MEASUREMENT") => {}
                    _ => push(&mut errors, "MEASURED_CASE_HAS_UNQUALIFIED_SINGLETON_PARENT", "$.fault_catalog", format!("measured null model requires independent singleton response evidence for {singleton_id}/{channel_id}")),
                }
            }
        }
    }

    let Some(pairs) = case.get("required_combination_pairs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_REQUIRED_COMBINATION_PAIRS", "$.required_combination_pairs", "required combination pairs must be an array");
        return errors;
    };
    let mut pair_ids = BTreeSet::new();
    for (index, pair) in pairs.iter().enumerate() {
        let path = format!("$.required_combination_pairs[{index}]");
        let Some(pair_id) = text(pair, "pair_id") else {
            push(&mut errors, "MISSING_COMBINATION_PAIR_ID", &format!("{path}.pair_id"), "pair_id is required");
            continue;
        };
        if !pair_ids.insert(pair_id.to_owned()) {
            push(&mut errors, "DUPLICATE_COMBINATION_PAIR_ID", &format!("{path}.pair_id"), "pair IDs must be unique");
        }
        let a = text(pair, "combination_a_id");
        let b = text(pair, "combination_b_id");
        if a.is_none() || b.is_none() || a == b {
            push(&mut errors, "INVALID_COMBINATION_PAIR", &path, "pair must refer to two different combined states");
        }
        for combination_id in [a, b].into_iter().flatten() {
            if !combination_map.contains_key(combination_id) {
                push(&mut errors, "UNKNOWN_COMBINATION_PAIR_MEMBER", &path, "pair references an undeclared combined state");
            }
        }
        if pair.get("minimum_independent_principles").and_then(Value::as_u64).is_none_or(|n| n == 0) {
            push(&mut errors, "INVALID_COMBINATION_PAIR_THRESHOLD", &format!("{path}.minimum_independent_principles"), "pair requires a positive independent-principle threshold");
        }
    }

    errors
}

fn response_interval(response: &Value) -> Option<(f64, f64)> {
    if text(response, "availability") != Some("VALID") {
        return None;
    }
    Some((finite(response, "value_lower")?, finite(response, "value_upper")?))
}

/// Calculate additive-differential null envelopes using conservative interval
/// arithmetic. The combined response is never synthesized from singleton data.
pub fn evaluate_case(case: &Value, observability: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case, observability);
    if !errors.is_empty() {
        return Err(errors);
    }

    let evidence_class = text(case, "evidence_class").unwrap();
    let channels = observability["channels"].as_array().unwrap();
    let channel_map: BTreeMap<&str, &Value> = channels.iter()
        .map(|channel| (text(channel, "channel_id").unwrap(), channel))
        .collect();
    let fault_map: BTreeMap<&str, &Value> = case["fault_catalog"].as_array().unwrap().iter()
        .map(|fault| (text(fault, "fault_id").unwrap(), fault))
        .collect();
    let singleton_responses: BTreeMap<(&str, &str), &Value> = observability["responses"].as_array().unwrap().iter()
        .map(|response| (
            (text(response, "state_id").unwrap(), text(response, "channel_id").unwrap()),
            response
        )).collect();
    let combination_map: BTreeMap<&str, &Value> = case["combined_states"].as_array().unwrap().iter()
        .map(|combination| (text(combination, "combination_id").unwrap(), combination))
        .collect();
    let combined_responses: BTreeMap<(&str, &str), &Value> = case["responses"].as_array().unwrap().iter()
        .map(|response| (
            (text(response, "combination_id").unwrap(), text(response, "channel_id").unwrap()),
            response
        )).collect();
    let baseline_state_id = text(case, "baseline_state_id").unwrap();
    let mut interaction_results = Vec::new();

    for combination in case["combined_states"].as_array().unwrap() {
        let combination_id = text(combination, "combination_id").unwrap();
        let active_fault_ids: Vec<&str> = combination["active_fault_ids"].as_array().unwrap()
            .iter().map(|id| id.as_str().unwrap()).collect();
        for (channel_id, channel) in &channel_map {
            let combined = combined_responses[&(combination_id, *channel_id)];
            let combined_interval = response_interval(combined);
            let mut baseline_and_singletons = Vec::new();
            let baseline = singleton_responses[&(baseline_state_id, *channel_id)];
            baseline_and_singletons.push(baseline);
            for fault_id in &active_fault_ids {
                let fault = fault_map[*fault_id];
                let singleton_state_id = text(fault, "singleton_state_id").unwrap();
                baseline_and_singletons.push(singleton_responses[&(singleton_state_id, *channel_id)]);
            }

            let mut null_lower = 0.0;
            let mut null_upper = 0.0;
            let mut null_available = true;
            let mut singleton_intervals = Vec::new();
            for response in &baseline_and_singletons {
                if text(response, "unit") != text(channel, "unit")
                    || !same_domain(response.get("domain").unwrap_or(&Value::Null), channel.get("domain").unwrap_or(&Value::Null))
                    || !response_role_admissible(response, text(observability, "evidence_class").unwrap_or(""))
                {
                    null_available = false;
                    break;
                }
                match response_interval(response) {
                    Some(interval) => singleton_intervals.push(interval),
                    None => {
                        null_available = false;
                        break;
                    }
                }
            }
            if null_available {
                let baseline_interval = singleton_intervals[0];
                let mut sum_singleton_lower = 0.0;
                let mut sum_singleton_upper = 0.0;
                for interval in singleton_intervals.iter().skip(1) {
                    sum_singleton_lower += interval.0;
                    sum_singleton_upper += interval.1;
                }
                let fault_count_minus_one = (active_fault_ids.len() - 1) as f64;
                null_lower = sum_singleton_lower - fault_count_minus_one * baseline_interval.1;
                null_upper = sum_singleton_upper - fault_count_minus_one * baseline_interval.0;
                if !null_lower.is_finite() || !null_upper.is_finite() || null_upper < null_lower {
                    null_available = false;
                }
            }

            let nonnegative_measurand = matches!(
                text(channel, "measurand"),
                Some("DRY_SOLID_MASS_FLOW" | "AIRFLOW" | "PARTICLE_VELOCITY" | "PARTICLE_CONCENTRATION")
            );
            let null_outside_physical_domain = null_available
                && nonnegative_measurand
                && null_upper < 0.0;

            let status = match (null_available, combined_interval) {
                (false, _) | (true, None) => "INSUFFICIENT_EVIDENCE",
                (true, Some(_)) if null_outside_physical_domain => "INSUFFICIENT_EVIDENCE",
                (true, Some((combined_lower, combined_upper))) => {
                    let overlaps = combined_lower <= null_upper && null_lower <= combined_upper;
                    if overlaps { "CONSISTENT_WITH_DECLARED_NULL_MODEL" } else { "DEVIATION_OBSERVED" }
                }
            };
            interaction_results.push(InteractionResult {
                combination_id: combination_id.to_owned(),
                channel_id: (*channel_id).to_owned(),
                status: status.to_owned(),
                null_lower: if null_available { Some(null_lower) } else { None },
                null_upper: if null_available { Some(null_upper) } else { None },
                combined_lower: combined_interval.map(|v| v.0),
                combined_upper: combined_interval.map(|v| v.1),
                active_fault_count: active_fault_ids.len(),
            });
        }
    }

    let mut combination_pair_results = Vec::new();
    for pair in case["required_combination_pairs"].as_array().unwrap() {
        let pair_id = text(pair, "pair_id").unwrap();
        let combination_a_id = text(pair, "combination_a_id").unwrap();
        let combination_b_id = text(pair, "combination_b_id").unwrap();
        let combination_a = combination_map[combination_a_id];
        let combination_b = combination_map[combination_b_id];
        let mut groups = BTreeSet::new();
        let mut discriminating_channels = Vec::new();
        let mut unresolved = false;
        for (channel_id, channel) in &channel_map {
            let a = combined_responses[&(combination_a_id, *channel_id)];
            let b = combined_responses[&(combination_b_id, *channel_id)];
            if !response_role_admissible(a, evidence_class) || !response_role_admissible(b, evidence_class)
                || text(a, "availability") == Some("UNKNOWN")
                || text(b, "availability") == Some("UNKNOWN")
            {
                unresolved = true;
                continue;
            }
            let ia = response_interval(a);
            let ib = response_interval(b);
            let (Some(ia), Some(ib)) = (ia, ib) else {
                let known_a = expected_faulted_unavailability(combination_a, channel_id, a);
                let known_b = expected_faulted_unavailability(combination_b, channel_id, b);
                if (text(a, "availability") != Some("VALID") && !known_a)
                    || (text(b, "availability") != Some("VALID") && !known_b)
                {
                    unresolved = true;
                }
                continue;
            };
            if ia.1 < ib.0 || ib.1 < ia.0 {
                if let Some(group) = text(channel, "independence_group_ref") {
                    groups.insert(group.to_owned());
                    discriminating_channels.push((*channel_id).to_owned());
                }
            }
        }
        discriminating_channels.sort();
        let minimum = pair["minimum_independent_principles"].as_u64().unwrap() as usize;
        let status = if groups.len() >= minimum {
            "IDENTIFIABLE"
        } else if unresolved {
            "INSUFFICIENT_EVIDENCE"
        } else {
            "NOT_IDENTIFIABLE"
        };
        combination_pair_results.push(CombinationPairResult {
            pair_id: pair_id.to_owned(),
            combination_a_id: combination_a_id.to_owned(),
            combination_b_id: combination_b_id.to_owned(),
            required_independent_principles: minimum,
            observed_independent_principles: groups.len(),
            discriminating_channel_ids: discriminating_channels,
            status: status.to_owned(),
        });
    }

    let deviation_observed_count = interaction_results.iter().filter(|r| r.status == "DEVIATION_OBSERVED").count();
    let consistent_with_null_count = interaction_results.iter().filter(|r| r.status == "CONSISTENT_WITH_DECLARED_NULL_MODEL").count();
    let insufficient_interaction_evidence_count = interaction_results.iter().filter(|r| r.status == "INSUFFICIENT_EVIDENCE").count();

    Ok(EvaluationReport {
        case_id: text(case, "case_id").unwrap().to_owned(),
        evidence_disposition: if evidence_class == "SYNTHETIC_CONFORMANCE_FIXTURE" { "SYNTHETIC_ONLY" } else { "MEASURED_EVIDENCE_REVIEW_REQUIRED" }.to_owned(),
        fault_count: faults_len(case),
        combination_count: combination_map.len(),
        response_count: case["responses"].as_array().unwrap().len(),
        interaction_results,
        deviation_observed_count,
        consistent_with_null_count,
        insufficient_interaction_evidence_count,
        combination_pair_results,
        limitations: vec![
            "Additive differential model is a null hypothesis, not a physical law".to_owned(),
            "A deviation may reflect non-additive physics, calibration, sampling, domain or timing mismatch".to_owned(),
            "Synthetic response intervals never establish physical interaction".to_owned(),
            "Interval arithmetic is conservative but does not estimate covariance or prove statistical independence".to_owned(),
            "Measured interaction conclusions require independently qualified baseline, singleton and combined-state measurements".to_owned(),
        ],
    })
}

fn faults_len(case: &Value) -> usize {
    case["fault_catalog"].as_array().map_or(0, Vec::len)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observability() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-sensor-observability-case.v0_1.json"
        )).expect("parent fixture parses")
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-coupled-fault-case.v0_1.json"
        )).expect("coupled fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    fn interaction<'a>(report: &'a EvaluationReport, combination: &str, channel: &str) -> &'a InteractionResult {
        report.interaction_results.iter()
            .find(|r| r.combination_id == combination && r.channel_id == channel).unwrap()
    }

    #[test]
    fn synthetic_coupled_fault_manifest_is_valid() {
        assert!(validate_case(&fixture(), &observability()).is_empty());
    }

    #[test]
    fn structural_report_counts_faults_combinations_and_response_cells() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        assert_eq!(report.fault_count, 5);
        assert_eq!(report.combination_count, 3);
        assert_eq!(report.response_count, 21);
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
    }

    #[test]
    fn blockage_plus_filter_has_a_screening_deviation_on_line_pressure() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        let result = interaction(&report, "combo-blockage-filter", "dp-line");
        assert_eq!(result.status, "DEVIATION_OBSERVED");
        assert_eq!(result.null_lower, Some(30.0));
        assert_eq!(result.null_upper, Some(41.0));
    }

    #[test]
    fn overlapping_dp_filter_intervals_are_consistent_with_additive_null() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        let result = interaction(&report, "combo-blockage-filter", "dp-filter");
        assert_eq!(result.status, "CONSISTENT_WITH_DECLARED_NULL_MODEL");
        assert_eq!(result.null_lower, Some(13.0));
        assert_eq!(result.null_upper, Some(20.0));
    }

    #[test]
    fn null_model_outside_nonnegative_measurand_domain_is_not_called_an_interaction() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        let result = interaction(&report, "combo-blockage-blower", "gravimetric-flow");
        assert!(result.null_upper.unwrap() < 0.0);
        assert_eq!(result.status, "INSUFFICIENT_EVIDENCE");
    }

    #[test]
    fn pressure_sensor_fault_channel_is_not_given_a_numeric_null_or_interaction_claim() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        let result = interaction(&report, "combo-blockage-pressure-sensor", "dp-line");
        assert_eq!(result.status, "INSUFFICIENT_EVIDENCE");
        assert_eq!(result.null_lower, None);
        assert_eq!(result.combined_lower, None);
    }

    #[test]
    fn pairwise_combination_results_use_independent_principle_groups() {
        let report = evaluate_case(&fixture(), &observability()).unwrap();
        assert_eq!(report.combination_pair_results.len(), 3);
        assert!(report.combination_pair_results.iter().all(|r| r.observed_independent_principles <= r.discriminating_channel_ids.len()));
    }

    #[test]
    fn duplicate_active_fault_id_is_rejected() {
        let mut case = fixture();
        case["combined_states"][0]["active_fault_ids"] = serde_json::json!([
            "fault-developing-blockage","fault-developing-blockage"
        ]);
        assert!(has(&validate_case(&case, &observability()), "DUPLICATE_ACTIVE_FAULT_ID"));
    }

    #[test]
    fn unknown_active_fault_id_is_rejected() {
        let mut case = fixture();
        case["combined_states"][0]["active_fault_ids"][1] = serde_json::json!("unknown-fault");
        assert!(has(&validate_case(&case, &observability()), "UNKNOWN_ACTIVE_FAULT_ID"));
    }

    #[test]
    fn mutually_exclusive_faults_cannot_be_combined() {
        let mut case = fixture();
        case["fault_catalog"][0]["mutually_exclusive_group_ref"] = serde_json::json!("exclusive:pair-x");
        case["fault_catalog"][1]["mutually_exclusive_group_ref"] = serde_json::json!("exclusive:pair-x");
        assert!(has(&validate_case(&case, &observability()), "CONTRADICTORY_ACTIVE_FAULTS"));
    }

    #[test]
    fn causal_chain_requires_known_onset_order() {
        let mut case = fixture();
        case["combined_states"][0]["combination_type"] = serde_json::json!("CAUSAL_CHAIN");
        assert!(has(
            &validate_case(&case, &observability()),
            "CAUSAL_CHAIN_REQUIRES_KNOWN_ORDER"
        ));
    }

    #[test]
    fn known_onset_order_must_cover_each_active_fault_once() {
        let mut case = fixture();
        case["combined_states"][1]["onset_order"] = serde_json::json!(["fault-blower-vfd"]);
        assert!(has(&validate_case(&case, &observability()), "INVALID_FAULT_ONSET_ORDER"));
    }

    #[test]
    fn sensor_fault_channel_must_be_unavailable_in_combined_state() {
        let mut case = fixture();
        let response = case["responses"].as_array_mut().unwrap().iter_mut()
            .find(|r| r["combination_id"] == "combo-blockage-pressure-sensor" && r["channel_id"] == "dp-line").unwrap();
        response["availability"] = serde_json::json!("VALID");
        response["value_lower"] = serde_json::json!(10.0);
        response["value_upper"] = serde_json::json!(12.0);
        assert!(has(&validate_case(&case, &observability()), "FAULTED_CHANNEL_REPORTED_HEALTHY"));
    }

    #[test]
    fn missing_combined_response_cell_is_rejected() {
        let mut case = fixture();
        case["responses"].as_array_mut().unwrap().pop();
        assert!(has(&validate_case(&case, &observability()), "INCOMPLETE_COMBINED_RESPONSE_MATRIX"));
    }

    #[test]
    fn duplicate_combined_response_cell_is_rejected() {
        let mut case = fixture();
        let duplicate = case["responses"][0].clone();
        case["responses"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_case(&case, &observability()), "DUPLICATE_COMBINED_RESPONSE"));
    }

    #[test]
    fn combined_response_unit_must_match_parent_channel() {
        let mut case = fixture();
        case["responses"][0]["unit"] = serde_json::json!("A");
        assert!(has(&validate_case(&case, &observability()), "COMBINED_RESPONSE_UNIT_MISMATCH"));
    }

    #[test]
    fn combined_response_domain_must_match_parent_channel() {
        let mut case = fixture();
        case["responses"][0]["domain"]["material_cohort_ref"] = serde_json::json!("different-rock");
        assert!(has(&validate_case(&case, &observability()), "COMBINED_RESPONSE_DOMAIN_MISMATCH"));
    }

    #[test]
    fn combined_response_window_must_match_combination_event_window() {
        let mut case = fixture();
        case["responses"][0]["measurement_window_ref"] = serde_json::json!("other-window");
        assert!(has(&validate_case(&case, &observability()), "COMBINED_RESPONSE_WINDOW_MISMATCH"));
    }

    #[test]
    fn simulation_only_parent_cannot_be_promoted_to_measured_interaction() {
        let mut case = fixture();
        case["evidence_class"] = serde_json::json!("MEASURED");
        case["combined_states"][0]["evidence_class"] = serde_json::json!("MEASURED");
        assert!(has(&validate_case(&case, &observability()), "MEASURED_CASE_HAS_SYNTHETIC_SINGLETON_PARENT"));
    }

    #[test]
    fn model_estimate_cannot_establish_combined_state_response() {
        let mut case = fixture();
        case["responses"][0]["evidence_role"] = serde_json::json!("MODEL_ESTIMATE");
        assert!(has(&validate_case(&case, &observability()), "NONMEASUREMENT_COMBINED_RESPONSE"));
    }

    #[test]
    fn combination_faulted_channel_set_must_match_fault_catalog() {
        let mut case = fixture();
        case["combined_states"][2]["faulted_channel_ids"] = serde_json::json!([]);
        assert!(has(&validate_case(&case, &observability()), "FAULTED_CHANNEL_SET_MISMATCH"));
    }

    #[test]
    fn deterministic_replay() {
        let case = fixture();
        let parent = observability();
        assert_eq!(
            evaluate_case(&case, &parent).unwrap(),
            evaluate_case(&case, &parent).unwrap()
        );
    }
}
