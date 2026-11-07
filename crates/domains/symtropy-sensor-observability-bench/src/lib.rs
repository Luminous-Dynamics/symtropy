//! Pairwise fault diagnosability for a declared excavation/conveying sensor layout.
//!
//! The evaluator counts independently grouped measurement principles, not raw
//! channel count. It only treats calibrated, in-domain, independently measured
//! response intervals as physical signatures. It never trains a classifier,
//! selects a safe installation, or turns synthetic signatures into field truth.
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
pub struct PairIdentifiability {
    pub pair_id: String,
    pub state_a_id: String,
    pub state_b_id: String,
    pub critical: bool,
    pub required_independent_principles: usize,
    pub observed_independent_principles: usize,
    pub discriminating_channel_ids: Vec<String>,
    pub independence_group_refs: Vec<String>,
    pub status: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EvaluationReport {
    pub case_id: String,
    pub evidence_disposition: String,
    pub channel_count: usize,
    pub independence_group_count: usize,
    pub state_count: usize,
    pub required_pair_count: usize,
    pub identifiable_pair_count: usize,
    pub not_identifiable_pair_count: usize,
    pub insufficient_evidence_pair_count: usize,
    pub critical_unidentifiable_pair_count: usize,
    pub pair_results: Vec<PairIdentifiability>,
    pub pooled_fault_accuracy: String,
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
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn state_ids_match(parent: &Value, context: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();
    if text(context, "parent_case_id") != text(parent, "case_id") {
        push(&mut errors, "PARENT_CASE_MISMATCH", "$.parent_context.parent_case_id", "observability case must bind to the exact supplied #3876 parent case");
    }
    let intervals = parent.get("intervals").and_then(Value::as_array);
    let interval_id = text(context, "parent_interval_id");
    let matches: Vec<&Value> = intervals
        .into_iter()
        .flatten()
        .filter(|interval| text(interval, "interval_id") == interval_id)
        .collect();
    if matches.len() != 1 {
        push(&mut errors, "PARENT_INTERVAL_NOT_UNIQUE", "$.parent_context.parent_interval_id", "parent interval must resolve to exactly one interval");
    } else if text(context, "trial_id") != text(matches[0], "trial_id") {
        push(&mut errors, "PARENT_TRIAL_MISMATCH", "$.parent_context.trial_id", "trial identity must match exact parent interval");
    }
    if let Some(profile) = parent.get("profile") {
        for key in [
            "material_cohort_ref",
            "machine_configuration_ref",
            "transport_configuration_ref",
        ] {
            if text(context, key) != text(profile, key) {
                push(&mut errors, "PARENT_PROFILE_MISMATCH", &format!("$.parent_context.{key}"), "observability context must match exact parent material/machine/transport profile");
            }
        }
    } else {
        push(&mut errors, "PARENT_PROFILE_MISSING", "$.parent.profile", "parent profile is required");
    }
    errors
}

fn intervals_disjoint(a_lower: f64, a_upper: f64, b_lower: f64, b_upper: f64) -> bool {
    a_upper < b_lower || b_upper < a_lower
}

fn valid_state_id(id: &str) -> bool {
    matches!(
        id,
        "NORMAL_STABLE"
            | "STARTUP_SHUTDOWN_TRANSIENT"
            | "HIGH_SOLIDS_LOADING"
            | "STRATIFIED_FLOW"
            | "DUNE_DEPOSITION"
            | "PARTIAL_RESTRICTION"
            | "DEVELOPING_BLOCKAGE"
            | "COMPLETE_BLOCKAGE"
            | "WATER_ACCUMULATION"
            | "FILTER_CYCLONE_LOADING"
            | "AIR_LEAK_DISCONNECTION"
            | "BLOWER_VFD_FAULT"
            | "PRESSURE_SENSOR_FAULT"
            | "AIRFLOW_SENSOR_FAULT"
            | "CURRENT_SENSOR_FAULT"
            | "UNKNOWN_OUT_OF_DOMAIN"
    )
}

/// Validate sensor layout, state list, pair set and complete response matrix.
pub fn validate_case(case: &Value, parent: &Value) -> Vec<Violation> {
    let mut errors = state_ids_match(parent, &case["parent_context"]);

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() || text(case, "measurement_system_case_id").is_none() {
        push(&mut errors, "MISSING_CASE_OR_MEASUREMENT_SYSTEM_ID", "$", "case and measurement-system reference IDs are required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class must be SYNTHETIC_CONFORMANCE_FIXTURE or MEASURED");
    }

    let Some(profile) = case.get("layout_profile") else {
        push(&mut errors, "MISSING_LAYOUT_PROFILE", "$.layout_profile", "versioned layout profile and pair-set identity are required");
        return errors;
    };
    for key in ["layout_profile_ref", "required_pair_set_ref"] {
        if text(profile, key).is_none() {
            push(&mut errors, "MISSING_LAYOUT_PROFILE_FIELD", &format!("$.layout_profile.{key}"), "layout profile and required-pair-set references are required");
        }
    }
    if profile
        .get("minimum_independent_principles_for_critical_pairs")
        .and_then(Value::as_u64)
        .is_none_or(|n| n == 0)
    {
        push(&mut errors, "INVALID_DEFAULT_INDEPENDENCE_THRESHOLD", "$.layout_profile.minimum_independent_principles_for_critical_pairs", "critical pair default must require at least one independent measurement principle");
    }

    let Some(channels) = case.get("channels").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_CHANNELS", "$.channels", "channels must be an array");
        return errors;
    };
    if channels.is_empty() {
        push(&mut errors, "EMPTY_CHANNELS", "$.channels", "at least one selected sensor channel is required");
    }
    let mut channel_map: BTreeMap<String, &Value> = BTreeMap::new();
    let mut independence_groups = BTreeSet::new();
    for (index, channel) in channels.iter().enumerate() {
        let path = format!("$.channels[{index}]");
        let Some(id) = text(channel, "channel_id") else {
            push(&mut errors, "MISSING_CHANNEL_ID", &format!("{path}.channel_id"), "channel_id is required");
            continue;
        };
        if channel_map.insert(id.to_owned(), channel).is_some() {
            push(&mut errors, "DUPLICATE_CHANNEL_ID", &format!("{path}.channel_id"), "channel IDs must be unique");
        }
        for key in [
            "measurand",
            "technique",
            "unit",
            "physical_location_ref",
            "calibration_ref",
            "independence_group_ref",
            "dependency_domain_ref",
            "source_artifact_ref",
        ] {
            if text(channel, key).is_none() {
                push(&mut errors, "INCOMPLETE_CHANNEL_IDENTITY", &format!("{path}.{key}"), "channel measurand, technique, unit, location, calibration, independent-principle group, dependency domain and source are required");
            }
        }
        if let Some(group) = text(channel, "independence_group_ref") {
            independence_groups.insert(group.to_owned());
        }
        let domain = channel.get("domain");
        if domain.is_none()
            || text(domain.unwrap_or(&Value::Null), "material_cohort_ref").is_none()
            || text(domain.unwrap_or(&Value::Null), "path_configuration_ref").is_none()
            || text(domain.unwrap_or(&Value::Null), "geometry_profile_ref").is_none()
        {
            push(&mut errors, "INCOMPLETE_CHANNEL_DOMAIN", &format!("{path}.domain"), "channel calibration domain must declare material cohort, path and geometry");
        } else {
            let domain = domain.unwrap();
            if text(domain, "material_cohort_ref") != text(&case["parent_context"], "material_cohort_ref")
                || text(domain, "path_configuration_ref") != text(&case["parent_context"], "transport_configuration_ref")
            {
                push(&mut errors, "CHANNEL_PARENT_DOMAIN_MISMATCH", &format!("{path}.domain"), "channel material/path domain must match the exact parent cohort and transport configuration");
            }
        }
    }

    let Some(states) = case.get("states").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_STATES", "$.states", "states must be an array");
        return errors;
    };
    let mut state_map: BTreeMap<String, &Value> = BTreeMap::new();
    for (index, state) in states.iter().enumerate() {
        let path = format!("$.states[{index}]");
        let Some(id) = text(state, "state_id") else {
            push(&mut errors, "MISSING_STATE_ID", &format!("{path}.state_id"), "state_id is required");
            continue;
        };
        if !valid_state_id(id) {
            push(&mut errors, "UNKNOWN_STATE_ID", &format!("{path}.state_id"), "state is outside the frozen v0.1 taxonomy");
        }
        if state_map.insert(id.to_owned(), state).is_some() {
            push(&mut errors, "DUPLICATE_STATE_ID", &format!("{path}.state_id"), "state IDs must be unique");
        }
        if !matches!(
            text(state, "state_class"),
            Some("PROCESS_STATE" | "SENSOR_FAULT" | "OUT_OF_DOMAIN")
        ) {
            push(&mut errors, "INVALID_STATE_CLASS", &format!("{path}.state_class"), "state class must distinguish process, sensor fault and OOD");
        }
        let faulted = state.get("faulted_channel_ids").and_then(Value::as_array);
        if faulted.is_none() {
            push(&mut errors, "MISSING_FAULTED_CHANNEL_LIST", &format!("{path}.faulted_channel_ids"), "every state must explicitly declare faulted_channel_ids, including an empty list for process states");
        }
        let faulted_ids: Vec<&str> = faulted.into_iter().flatten().filter_map(Value::as_str).collect();
        if faulted_ids.iter().collect::<BTreeSet<_>>().len() != faulted_ids.len() {
            push(&mut errors, "DUPLICATE_FAULTED_CHANNEL_ID", &format!("{path}.faulted_channel_ids"), "faulted channel IDs must be unique");
        }
        if text(state, "state_class") == Some("SENSOR_FAULT") && faulted_ids.is_empty() {
            push(&mut errors, "SENSOR_FAULT_WITHOUT_CHANNEL", &format!("{path}.faulted_channel_ids"), "sensor-fault states must identify at least one affected channel");
        }
        if text(state, "state_class") != Some("SENSOR_FAULT") && !faulted_ids.is_empty() {
            push(&mut errors, "NON_SENSOR_STATE_HAS_FAULTED_CHANNEL", &format!("{path}.faulted_channel_ids"), "only SENSOR_FAULT states may declare faulted sensor channels");
        }
        for faulted_id in faulted_ids {
            if !channel_map.contains_key(faulted_id) {
                push(&mut errors, "UNKNOWN_FAULTED_CHANNEL", &format!("{path}.faulted_channel_ids"), format!("faulted channel {faulted_id} is not in the selected layout"));
            }
        }
    }

    let Some(required_pairs) = case.get("required_state_pairs").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_REQUIRED_STATE_PAIRS", "$.required_state_pairs", "required_state_pairs must be an array");
        return errors;
    };
    let mut pair_ids = BTreeSet::new();
    for (index, pair) in required_pairs.iter().enumerate() {
        let path = format!("$.required_state_pairs[{index}]");
        let Some(id) = text(pair, "pair_id") else {
            push(&mut errors, "MISSING_PAIR_ID", &format!("{path}.pair_id"), "pair_id is required");
            continue;
        };
        if !pair_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_PAIR_ID", &format!("{path}.pair_id"), "pair IDs must be unique");
        }
        let a = text(pair, "state_a_id");
        let b = text(pair, "state_b_id");
        if a.is_none() || b.is_none() || a == b {
            push(&mut errors, "INVALID_STATE_PAIR", &path, "pair must name two different states");
        }
        for state_id in [a, b].into_iter().flatten() {
            if !state_map.contains_key(state_id) {
                push(&mut errors, "UNKNOWN_PAIR_STATE", &path, format!("required pair references undeclared state {state_id}"));
            }
        }
        let pair_minimum = pair.get("minimum_independent_principles").and_then(Value::as_u64);
        if pair_minimum.is_none_or(|n| n == 0) {
            push(&mut errors, "INVALID_PAIR_INDEPENDENCE_THRESHOLD", &format!("{path}.minimum_independent_principles"), "each pair must explicitly require at least one independent principle");
        }
        if pair.get("critical").and_then(Value::as_bool) == Some(true) {
            let profile_minimum = profile
                .get("minimum_independent_principles_for_critical_pairs")
                .and_then(Value::as_u64)
                .unwrap_or(1);
            if pair_minimum.is_some_and(|n| n < profile_minimum) {
                push(&mut errors, "CRITICAL_PAIR_BELOW_LAYOUT_THRESHOLD", &format!("{path}.minimum_independent_principles"), "critical pair threshold cannot be lower than the versioned layout profile default");
            }
        }
    }

    let Some(responses) = case.get("responses").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_RESPONSES", "$.responses", "responses must be an array");
        return errors;
    };
    let mut response_map: BTreeMap<(String, String), &Value> = BTreeMap::new();
    let mut response_source_refs = BTreeSet::new();
    let mut state_window_refs: BTreeMap<String, String> = BTreeMap::new();
    for (index, response) in responses.iter().enumerate() {
        let path = format!("$.responses[{index}]");
        let state_id = text(response, "state_id");
        let channel_id = text(response, "channel_id");
        let (Some(state_id), Some(channel_id)) = (state_id, channel_id) else {
            push(&mut errors, "MISSING_RESPONSE_IDENTITY", &path, "every response must identify one state and one channel");
            continue;
        };
        if !state_map.contains_key(state_id) {
            push(&mut errors, "UNKNOWN_RESPONSE_STATE", &format!("{path}.state_id"), "response state is not declared");
        }
        let Some(channel) = channel_map.get(channel_id).copied() else {
            push(&mut errors, "UNKNOWN_RESPONSE_CHANNEL", &format!("{path}.channel_id"), "response channel is not declared");
            continue;
        };
        if response_map.insert((state_id.to_owned(), channel_id.to_owned()), response).is_some() {
            push(&mut errors, "DUPLICATE_STATE_CHANNEL_RESPONSE", &path, "only one response record is allowed for each state/channel pair");
        }
        if text(response, "unit") != text(channel, "unit") {
            push(&mut errors, "RESPONSE_UNIT_MISMATCH", &format!("{path}.unit"), "response unit must match channel unit");
        }
        if !same_domain(
            response.get("domain").unwrap_or(&Value::Null),
            channel.get("domain").unwrap_or(&Value::Null),
        ) {
            push(&mut errors, "RESPONSE_DOMAIN_MISMATCH", &format!("{path}.domain"), "response evidence domain must match exact channel calibration domain");
        }
        for key in ["measurement_window_ref", "source_ref", "adjudication_ref"] {
            if text(response, key).is_none() {
                push(&mut errors, "MISSING_RESPONSE_PROVENANCE", &format!("{path}.{key}"), "measurement window, raw source and adjudication references are required");
            }
        }
        if let Some(source) = text(response, "source_ref") {
            if !response_source_refs.insert(source.to_owned()) {
                push(&mut errors, "DUPLICATE_RESPONSE_SOURCE", &format!("{path}.source_ref"), "each state/channel response requires its own traceable source artifact; reuse needs an explicit paired/derived evidence model");
            }
        }
        if let Some(window) = text(response, "measurement_window_ref") {
            if let Some(previous) = state_window_refs.insert(state_id.to_owned(), window.to_owned()) {
                if previous != window {
                    push(&mut errors, "STATE_RESPONSE_WINDOW_MISMATCH", &format!("{path}.measurement_window_ref"), "all channel response records for one state must share the same declared synchronized measurement window");
                }
            }
        }
        let state_record = state_map.get(state_id).copied();
        let faulted_ids: BTreeSet<&str> = state_record
            .and_then(|state| state.get("faulted_channel_ids"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        if faulted_ids.contains(channel_id)
            && !matches!(text(response, "availability"), Some("DEGRADED" | "MISSING"))
        {
            push(&mut errors, "FAULTED_CHANNEL_REPORTED_HEALTHY", &path, "a channel declared faulted in this state must be DEGRADED or MISSING, never VALID");
        }
        match text(response, "evidence_role") {
            Some("INDEPENDENT_MEASUREMENT" | "SYNTHETIC_CONFORMANCE_FIXTURE") => {}
            Some("MODEL_ESTIMATE" | "OPERATOR_REPORTED") => {}
            _ => push(&mut errors, "INVALID_RESPONSE_EVIDENCE_ROLE", &format!("{path}.evidence_role"), "response evidence role is outside v0.1 vocabulary"),
        }
        match text(response, "availability") {
            Some("VALID") => {
                match (finite(response, "value_lower"), finite(response, "value_upper")) {
                    (Some(lower), Some(upper)) if upper >= lower => {}
                    _ => push(&mut errors, "INVALID_RESPONSE_INTERVAL", &path, "VALID response requires finite ordered lower/upper interval"),
                }
                if !matches!(
                    text(response, "evidence_role"),
                    Some("INDEPENDENT_MEASUREMENT" | "SYNTHETIC_CONFORMANCE_FIXTURE")
                ) {
                    push(&mut errors, "NON_PHYSICAL_RESPONSE_EVIDENCE", &format!("{path}.evidence_role"), "model/operator claims cannot establish a measured response envelope");
                }
            }
            Some("DEGRADED" | "MISSING" | "UNKNOWN") => {
                if !matches!(response.get("value_lower"), Some(Value::Null))
                    || !matches!(response.get("value_upper"), Some(Value::Null))
                {
                    push(&mut errors, "UNAVAILABLE_RESPONSE_HAS_VALUES", &path, "degraded/missing/unknown channel response must not carry numeric values");
                }
            }
            _ => push(&mut errors, "INVALID_RESPONSE_AVAILABILITY", &format!("{path}.availability"), "availability must be VALID, DEGRADED, MISSING or UNKNOWN"),
        }
    }

    for state_id in state_map.keys() {
        for channel_id in channel_map.keys() {
            if !response_map.contains_key(&(state_id.clone(), channel_id.clone())) {
                push(&mut errors, "INCOMPLETE_RESPONSE_MATRIX", "$.responses", format!("missing response for state {state_id} and channel {channel_id}"));
            }
        }
    }

    errors
}

fn same_domain(a: &Value, b: &Value) -> bool {
    ["material_cohort_ref", "path_configuration_ref", "geometry_profile_ref"]
        .iter()
        .all(|key| text(a, key).is_some() && text(a, key) == text(b, key))
}

/// Evaluate fault-pair identifiability using only non-overlapping intervals
/// from valid and independently evidenced channels. Channels sharing one
/// independence group contribute at most one independent principle.
pub fn evaluate_case(case: &Value, parent: &Value) -> Result<EvaluationReport, Vec<Violation>> {
    let errors = validate_case(case, parent);
    if !errors.is_empty() {
        return Err(errors);
    }

    let channels = case["channels"].as_array().expect("validated channels");
    let states = case["states"].as_array().expect("validated states");
    let required_pairs = case["required_state_pairs"].as_array().expect("validated pairs");
    let responses = case["responses"].as_array().expect("validated responses");
    let channel_map: BTreeMap<&str, &Value> = channels
        .iter()
        .map(|channel| (text(channel, "channel_id").expect("channel ID"), channel))
        .collect();
    let response_map: BTreeMap<(&str, &str), &Value> = responses
        .iter()
        .map(|response| {
            (
                (
                    text(response, "state_id").expect("state ID"),
                    text(response, "channel_id").expect("channel ID"),
                ),
                response,
            )
        })
        .collect();

    let mut pair_results = Vec::with_capacity(required_pairs.len());
    for pair in required_pairs {
        let state_a = text(pair, "state_a_id").expect("state A");
        let state_b = text(pair, "state_b_id").expect("state B");
        let mut groups = BTreeSet::new();
        let mut discriminating_channels = Vec::new();
        let mut unresolved_evidence = false;

        for (channel_id, channel) in &channel_map {
            let a = response_map[&(state_a, channel_id)];
            let b = response_map[&(state_b, channel_id)];
            let admissible_role = |response: &Value| {
                match text(response, "evidence_role") {
                    Some("INDEPENDENT_MEASUREMENT") => {
                        text(case, "evidence_class") == Some("MEASURED")
                    }
                    Some("SYNTHETIC_CONFORMANCE_FIXTURE") => {
                        text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE")
                    }
                    _ => false,
                }
            };

            if text(a, "availability") == Some("UNKNOWN")
                || text(b, "availability") == Some("UNKNOWN")
                || !admissible_role(a)
                || !admissible_role(b)
            {
                unresolved_evidence = true;
                continue;
            }
            if text(a, "availability") != Some("VALID")
                || text(b, "availability") != Some("VALID")
            {
                continue;
            }
            let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
                finite(a, "value_lower"),
                finite(a, "value_upper"),
                finite(b, "value_lower"),
                finite(b, "value_upper"),
            ) else {
                unresolved_evidence = true;
                continue;
            };
            if intervals_disjoint(a_lower, a_upper, b_lower, b_upper) {
                if let Some(group) = text(channel, "independence_group_ref") {
                    groups.insert(group.to_owned());
                    discriminating_channels.push((*channel_id).to_owned());
                }
            }
        }

        discriminating_channels.sort();
        let minimum = pair["minimum_independent_principles"]
            .as_u64()
            .expect("validated minimum") as usize;
        let status = if groups.len() >= minimum {
            "IDENTIFIABLE"
        } else if unresolved_evidence {
            "INSUFFICIENT_EVIDENCE"
        } else {
            "NOT_IDENTIFIABLE"
        };

        pair_results.push(PairIdentifiability {
            pair_id: text(pair, "pair_id").expect("pair ID").to_owned(),
            state_a_id: state_a.to_owned(),
            state_b_id: state_b.to_owned(),
            critical: pair["critical"].as_bool().expect("critical flag"),
            required_independent_principles: minimum,
            observed_independent_principles: groups.len(),
            discriminating_channel_ids: discriminating_channels,
            independence_group_refs: groups.into_iter().collect(),
            status: status.to_owned(),
        });
    }

    let identifiable = pair_results.iter().filter(|p| p.status == "IDENTIFIABLE").count();
    let not_identifiable = pair_results.iter().filter(|p| p.status == "NOT_IDENTIFIABLE").count();
    let insufficient = pair_results.iter().filter(|p| p.status == "INSUFFICIENT_EVIDENCE").count();
    let critical_unidentifiable = pair_results.iter().filter(|p| p.critical && p.status != "IDENTIFIABLE").count();
    let groups: BTreeSet<&str> = channels
        .iter()
        .filter_map(|channel| text(channel, "independence_group_ref"))
        .collect();

    Ok(EvaluationReport {
        case_id: text(case, "case_id").expect("case ID").to_owned(),
        evidence_disposition: if text(case, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE") {
            "SYNTHETIC_ONLY"
        } else {
            "MEASURED_RESPONSE_MATRIX_REVIEW_REQUIRED"
        }.to_owned(),
        channel_count: channels.len(),
        independence_group_count: groups.len(),
        state_count: states.len(),
        required_pair_count: pair_results.len(),
        identifiable_pair_count: identifiable,
        not_identifiable_pair_count: not_identifiable,
        insufficient_evidence_pair_count: insufficient,
        critical_unidentifiable_pair_count: critical_unidentifiable,
        pair_results,
        pooled_fault_accuracy: "NOT_COMPUTED_PAIRWISE_IDENTIFIABILITY_ONLY".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parent() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-fragment-transport-case.v0_1.json"
        )).expect("parent fixture parses")
    }

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-sensor-observability-case.v0_1.json"
        )).expect("observability fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|e| e.code == code)
    }

    fn pair<'a>(report: &'a EvaluationReport, id: &str) -> &'a PairIdentifiability {
        report.pair_results.iter().find(|p| p.pair_id == id).unwrap()
    }

    fn response_mut(case: &mut Value, state: &str, channel: &str) -> &mut Value {
        case["responses"].as_array_mut().unwrap().iter_mut()
            .find(|r| r["state_id"] == state && r["channel_id"] == channel).unwrap()
    }

    fn channel_mut(case: &mut Value, channel: &str) -> &mut Value {
        case["channels"].as_array_mut().unwrap().iter_mut()
            .find(|c| c["channel_id"] == channel).unwrap()
    }

    #[test]
    fn synthetic_layout_manifest_is_valid() {
        assert!(validate_case(&fixture(), &parent()).is_empty());
    }

    #[test]
    fn structural_report_counts_states_channels_and_required_pairs() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(report.channel_count, 7);
        assert_eq!(report.independence_group_count, 6);
        assert_eq!(report.state_count, 8);
        assert_eq!(report.required_pair_count, 8);
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
        assert_eq!(report.pooled_fault_accuracy, "NOT_COMPUTED_PAIRWISE_IDENTIFIABILITY_ONLY");
    }

    #[test]
    fn normal_vs_blockage_is_identifiable_with_multiple_principles() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        let result = pair(&report, "normal-vs-blockage");
        assert_eq!(result.status, "IDENTIFIABLE");
        assert!(result.observed_independent_principles >= 2);
    }

    #[test]
    fn two_pressure_channels_count_as_one_principle() {
        let mut case = fixture();
        let pressure_group = case["channels"][0]["independence_group_ref"].clone();
        channel_mut(&mut case, "dp-filter")["independence_group_ref"] = pressure_group;
        let report = evaluate_case(&case, &parent()).unwrap();
        let result = pair(&report, "normal-vs-filter-loading");
        assert!(result.observed_independent_principles < result.discriminating_channel_ids.len());
    }

    #[test]
    fn blockage_and_filter_loading_are_pairwise_identifiable() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(pair(&report, "blockage-vs-filter-loading").status, "IDENTIFIABLE");
    }

    #[test]
    fn blower_fault_is_identifiable_from_normal() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(pair(&report, "normal-vs-blower-fault").status, "IDENTIFIABLE");
    }

    #[test]
    fn pressure_sensor_fault_uses_independent_self_check() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        let result = pair(&report, "normal-vs-pressure-sensor-fault");
        assert_eq!(result.status, "IDENTIFIABLE");
        assert!(result.discriminating_channel_ids.contains(&"sensor-self-check".to_owned()));
        assert!(!result.discriminating_channel_ids.contains(&"dp-line".to_owned()));
    }

    #[test]
    fn airflow_sensor_fault_uses_independent_self_check() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        let result = pair(&report, "normal-vs-airflow-sensor-fault");
        assert_eq!(result.status, "IDENTIFIABLE");
        assert!(!result.discriminating_channel_ids.contains(&"airflow".to_owned()));
    }

    #[test]
    fn unknown_out_of_domain_state_is_insufficient_evidence() {
        let report = evaluate_case(&fixture(), &parent()).unwrap();
        assert_eq!(pair(&report, "normal-vs-ood").status, "INSUFFICIENT_EVIDENCE");
    }

    #[test]
    fn overlapping_intervals_do_not_discriminate() {
        let mut case = fixture();
        response_mut(&mut case, "DEVELOPING_BLOCKAGE", "dp-line")["value_lower"] =
            serde_json::json!(11.0);
        response_mut(&mut case, "DEVELOPING_BLOCKAGE", "dp-line")["value_upper"] =
            serde_json::json!(13.0);
        let report = evaluate_case(&case, &parent()).unwrap();
        let result = pair(&report, "normal-vs-blockage");
        assert!(!result.discriminating_channel_ids.contains(&"dp-line".to_owned()));
    }

    #[test]
    fn single_independence_group_cannot_satisfy_two_principle_gate() {
        let mut case = fixture();
        let pair_entry = case["required_state_pairs"].as_array_mut().unwrap()
            .iter_mut().find(|p| p["pair_id"] == "normal-vs-blockage").unwrap();
        pair_entry["minimum_independent_principles"] = serde_json::json!(6);
        let report = evaluate_case(&case, &parent()).unwrap();
        assert_ne!(pair(&report, "normal-vs-blockage").status, "IDENTIFIABLE");
    }

    #[test]
    fn model_estimates_cannot_create_physical_identifiability() {
        let mut case = fixture();
        response_mut(&mut case, "DEVELOPING_BLOCKAGE", "ecc-flow")["evidence_role"] =
            serde_json::json!("MODEL_ESTIMATE");
        let report = evaluate_case(&case, &parent()).unwrap();
        let result = pair(&report, "normal-vs-blockage");
        assert!(!result.discriminating_channel_ids.contains(&"ecc-flow".to_owned()));
    }

    #[test]
    fn missing_response_matrix_cell_is_rejected() {
        let mut case = fixture();
        case["responses"].as_array_mut().unwrap().pop();
        assert!(has(&validate_case(&case, &parent()), "INCOMPLETE_RESPONSE_MATRIX"));
    }

    #[test]
    fn duplicate_response_cell_is_rejected() {
        let mut case = fixture();
        let duplicate = case["responses"][0].clone();
        case["responses"].as_array_mut().unwrap().push(duplicate);
        assert!(has(&validate_case(&case, &parent()), "DUPLICATE_STATE_CHANNEL_RESPONSE"));
    }

    #[test]
    fn response_unit_must_match_channel() {
        let mut case = fixture();
        response_mut(&mut case, "NORMAL_STABLE", "dp-line")["unit"] =
            serde_json::json!("A");
        assert!(has(&validate_case(&case, &parent()), "RESPONSE_UNIT_MISMATCH"));
    }

    #[test]
    fn unknown_channel_in_response_is_rejected() {
        let mut case = fixture();
        case["responses"][0]["channel_id"] = serde_json::json!("not-a-channel");
        assert!(has(&validate_case(&case, &parent()), "UNKNOWN_RESPONSE_CHANNEL"));
    }

    #[test]
    fn parent_path_and_material_identity_must_match() {
        let mut case = fixture();
        case["channels"][0]["domain"]["material_cohort_ref"] =
            serde_json::json!("other-rock");
        assert!(has(&validate_case(&case, &parent()), "CHANNEL_PARENT_DOMAIN_MISMATCH"));
    }

    #[test]
    fn degraded_response_cannot_carry_a_numeric_interval() {
        let mut case = fixture();
        response_mut(&mut case, "PRESSURE_SENSOR_FAULT", "dp-line")["value_lower"] =
            serde_json::json!(10.0);
        assert!(has(&validate_case(&case, &parent()), "UNAVAILABLE_RESPONSE_HAS_VALUES"));
    }

    #[test]
    fn state_pair_cannot_compare_a_state_to_itself() {
        let mut case = fixture();
        let state_a = case["required_state_pairs"][0]["state_a_id"].clone();
        case["required_state_pairs"][0]["state_b_id"] = state_a;
        assert!(has(&validate_case(&case, &parent()), "INVALID_STATE_PAIR"));
    }

    #[test]
    fn unknown_state_taxonomy_is_rejected() {
        let mut case = fixture();
        case["states"][0]["state_id"] = serde_json::json!("MAGIC_FAULT");
        assert!(has(&validate_case(&case, &parent()), "UNKNOWN_STATE_ID"));
    }

    #[test]
    fn declared_faulted_channel_cannot_remain_valid() {
        let mut case = fixture();
        response_mut(&mut case, "PRESSURE_SENSOR_FAULT", "dp-line")["availability"] =
            serde_json::json!("VALID");
        response_mut(&mut case, "PRESSURE_SENSOR_FAULT", "dp-line")["value_lower"] =
            serde_json::json!(10.0);
        response_mut(&mut case, "PRESSURE_SENSOR_FAULT", "dp-line")["value_upper"] =
            serde_json::json!(12.0);
        assert!(has(
            &validate_case(&case, &parent()),
            "FAULTED_CHANNEL_REPORTED_HEALTHY"
        ));
    }

    #[test]
    fn sensor_fault_must_name_at_least_one_faulted_channel() {
        let mut case = fixture();
        case["states"][4]["faulted_channel_ids"] = serde_json::json!([]);
        assert!(has(
            &validate_case(&case, &parent()),
            "SENSOR_FAULT_WITHOUT_CHANNEL"
        ));
    }

    #[test]
    fn critical_pair_cannot_lower_layout_minimum() {
        let mut case = fixture();
        case["required_state_pairs"][0]["minimum_independent_principles"] =
            serde_json::json!(1);
        assert!(has(
            &validate_case(&case, &parent()),
            "CRITICAL_PAIR_BELOW_LAYOUT_THRESHOLD"
        ));
    }

    #[test]
    fn channels_for_one_state_must_share_a_measurement_window() {
        let mut case = fixture();
        response_mut(&mut case, "NORMAL_STABLE", "airflow")["measurement_window_ref"] =
            serde_json::json!("window:misaligned");
        assert!(has(
            &validate_case(&case, &parent()),
            "STATE_RESPONSE_WINDOW_MISMATCH"
        ));
    }

    #[test]
    fn deterministic_replay() {
        let case = fixture();
        assert_eq!(
            evaluate_case(&case, &parent()).unwrap(),
            evaluate_case(&case, &parent()).unwrap()
        );
    }
}
