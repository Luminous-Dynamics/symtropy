//! Exact small-set Pareto sensor placement under fault-pair observability constraints.
//!
//! The search enumerates finite candidate subsets and evaluates the selected
//! channels against the existing sensor-observability response matrix. It
//! returns a Pareto set, not a weighted winner. Costs and response evidence
//! retain their source/evidence class; synthetic inputs remain synthetic.
#![forbid(unsafe_code)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_EXACT_SEARCH_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Violation {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairScenarioResult {
    pub pair_id: String,
    pub status: String,
    pub observed_independent_principles: usize,
    pub required_independent_principles: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioCoverage {
    pub scenario_id: String,
    pub identifiable_pairs: usize,
    pub total_pairs: usize,
    pub identifiable_critical_pairs: usize,
    pub total_critical_pairs: usize,
    pub pair_results: Vec<PairScenarioResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParetoLayout {
    pub candidate_ids: Vec<String>,
    pub channel_ids: Vec<String>,
    pub lifecycle_cost_units: f64,
    pub cost_unit: String,
    pub channel_count: usize,
    pub maintenance_hours_per_year: f64,
    pub planned_unavailability_hours_per_year: f64,
    pub worst_case_identifiable_pairs: usize,
    pub total_required_pairs: usize,
    pub feasible: bool,
    pub scenarios: Vec<ScenarioCoverage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SearchReport {
    pub case_id: String,
    pub observability_case_id: String,
    pub evidence_disposition: String,
    pub candidate_count: usize,
    pub exact_subsets_examined: u64,
    pub subsets_over_budget: u64,
    pub subsets_over_channel_limit: u64,
    pub feasible_layout_count: usize,
    pub pareto_layout_count: usize,
    pub pareto_layouts: Vec<ParetoLayout>,
    pub search_method: String,
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
    value.get(key).and_then(Value::as_f64).filter(|v| v.is_finite())
}

fn known_fault_unavailability(
    observation_case: &Value,
    state_id: &str,
    channel_id: &str,
    response: &Value,
) -> bool {
    let state = observation_case["states"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|state| text(state, "state_id") == Some(state_id));
    state
        .and_then(|state| state.get("faulted_channel_ids"))
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(channel_id)))
        && text(response, "availability") != Some("VALID")
}

fn validate_observability_parent(case: &Value, observability: &Value) -> Vec<Violation> {
    let mut errors = Vec::new();
    if text(case, "observability_case_id") != text(observability, "case_id") {
        push(
            &mut errors,
            "OBSERVABILITY_CASE_ID_MISMATCH",
            "$.observability_case_id",
            "placement case must reference the exact supplied observability response matrix",
        );
    }
    if text(observability, "case_id").is_none() {
        push(&mut errors, "OBSERVABILITY_CASE_ID_MISSING", "$.observability.case_id", "observability case ID is required");
    }
    let channels = observability.get("channels").and_then(Value::as_array);
    let states = observability.get("states").and_then(Value::as_array);
    let pairs = observability.get("required_state_pairs").and_then(Value::as_array);
    let responses = observability.get("responses").and_then(Value::as_array);
    let (Some(channels), Some(states), Some(pairs), Some(responses)) = (channels, states, pairs, responses) else {
        push(&mut errors, "OBSERVABILITY_CASE_INCOMPLETE", "$.observability", "channels, states, required pairs and response matrix are all required");
        return errors;
    };
    let mut channel_ids = BTreeSet::new();
    let mut channel_map = BTreeMap::new();
    for channel in channels {
        if let Some(id) = text(channel, "channel_id") {
            if !channel_ids.insert(id.to_owned()) {
                push(&mut errors, "OBSERVABILITY_DUPLICATE_CHANNEL", "$.observability.channels", "channel IDs must be unique");
            }
            channel_map.insert(id.to_owned(), channel);
        } else {
            push(&mut errors, "OBSERVABILITY_CHANNEL_ID_MISSING", "$.observability.channels", "channel IDs are required");
        }
    }
    let mut state_ids = BTreeSet::new();
    for state in states {
        if let Some(id) = text(state, "state_id") {
            if !state_ids.insert(id.to_owned()) {
                push(&mut errors, "OBSERVABILITY_DUPLICATE_STATE", "$.observability.states", "state IDs must be unique");
            }
        } else {
            push(&mut errors, "OBSERVABILITY_STATE_ID_MISSING", "$.observability.states", "state IDs are required");
        }
    }
    let mut response_keys = BTreeSet::new();
    for response in responses {
        let state_id = text(response, "state_id");
        let channel_id = text(response, "channel_id");
        let (Some(state_id), Some(channel_id)) = (state_id, channel_id) else {
            push(&mut errors, "OBSERVABILITY_RESPONSE_IDENTITY_MISSING", "$.observability.responses", "every response must identify a state and channel");
            continue;
        };
        if !state_ids.contains(state_id) || !channel_map.contains_key(channel_id) {
            push(&mut errors, "OBSERVABILITY_RESPONSE_REFERENCE_UNKNOWN", "$.observability.responses", "response references a state or channel outside the declared matrix");
        }
        if !response_keys.insert((state_id.to_owned(), channel_id.to_owned())) {
            push(&mut errors, "OBSERVABILITY_DUPLICATE_RESPONSE", "$.observability.responses", "each state/channel cell must appear once");
        }
        if let Some(channel) = channel_map.get(channel_id) {
            if text(response, "unit") != text(channel, "unit") {
                push(&mut errors, "OBSERVABILITY_RESPONSE_UNIT_MISMATCH", "$.observability.responses", "response unit must match its channel");
            }
            if !["material_cohort_ref", "path_configuration_ref", "geometry_profile_ref"].iter().all(|key| {
                text(&response["domain"], key).is_some()
                    && text(&response["domain"], key) == text(&channel["domain"], key)
            }) {
                push(&mut errors, "OBSERVABILITY_RESPONSE_DOMAIN_MISMATCH", "$.observability.responses", "response domain must match its channel");
            }
        }
    }
    for state_id in &state_ids {
        for channel_id in &channel_ids {
            if !response_keys.contains(&(state_id.clone(), channel_id.clone())) {
                push(&mut errors, "OBSERVABILITY_RESPONSE_MATRIX_INCOMPLETE", "$.observability.responses", "every state/channel response cell is required");
            }
        }
    }
    for pair in pairs {
        if text(pair, "state_a_id").is_none_or(|id| !state_ids.contains(id))
            || text(pair, "state_b_id").is_none_or(|id| !state_ids.contains(id))
            || pair.get("minimum_independent_principles").and_then(Value::as_u64).is_none_or(|n| n == 0)
        {
            push(&mut errors, "OBSERVABILITY_REQUIRED_PAIR_INVALID", "$.observability.required_state_pairs", "required pairs must reference declared states and positive independence thresholds");
        }
    }
    errors
}

/// Validate placement candidates and scenario identities against the exact
/// sensor-observability case. This does not re-qualify sensor measurements.
pub fn validate_case(case: &Value, observability: &Value) -> Vec<Violation> {
    let mut errors = validate_observability_parent(case, observability);

    if case.get("schema_version").and_then(Value::as_u64) != Some(1) {
        push(&mut errors, "UNSUPPORTED_SCHEMA_VERSION", "$.schema_version", "schema_version must be exactly 1");
    }
    if text(case, "case_id").is_none() {
        push(&mut errors, "MISSING_CASE_ID", "$.case_id", "case_id is required");
    }
    if !matches!(
        text(case, "evidence_class"),
        Some("SYNTHETIC_CONFORMANCE_FIXTURE" | "MEASURED_COSTS_AND_RESPONSES")
    ) {
        push(&mut errors, "INVALID_EVIDENCE_CLASS", "$.evidence_class", "evidence class is outside v0.1 vocabulary");
    }

    let Some(profile) = case.get("placement_profile") else {
        push(&mut errors, "MISSING_PLACEMENT_PROFILE", "$.placement_profile", "placement profile is required");
        return errors;
    };
    for key in ["profile_ref", "cost_unit"] {
        if text(profile, key).is_none() {
            push(&mut errors, "MISSING_PLACEMENT_PROFILE_FIELD", &format!("$.placement_profile.{key}"), "profile reference and cost unit are required");
        }
    }
    let search_limit = profile.get("exact_search_limit").and_then(Value::as_u64);
    if search_limit.is_none_or(|n| n == 0 || n as usize > MAX_EXACT_SEARCH_LIMIT) {
        push(&mut errors, "INVALID_EXACT_SEARCH_LIMIT", "$.placement_profile.exact_search_limit", "exact search limit must be between 1 and 20");
    }
    if profile.get("maximum_selected_channels").and_then(Value::as_u64).is_none_or(|n| n == 0) {
        push(&mut errors, "INVALID_MAXIMUM_CHANNELS", "$.placement_profile.maximum_selected_channels", "maximum selected channels must be positive");
    }
    for key in ["require_all_critical_pairs_nominal", "require_all_critical_pairs_under_outages"] {
        if profile.get(key).and_then(Value::as_bool).is_none() {
            push(&mut errors, "MISSING_CRITICAL_PAIR_REQUIREMENT", &format!("$.placement_profile.{key}"), "critical-pair nominal/outage requirements must be explicit booleans");
        }
    }
    match profile.get("max_lifecycle_cost_units") {
        Some(Value::Null) => {}
        Some(value) if value.as_f64().is_some_and(|n| n.is_finite() && n >= 0.0) => {}
        _ => push(&mut errors, "INVALID_LIFECYCLE_BUDGET", "$.placement_profile.max_lifecycle_cost_units", "lifecycle cost budget must be null or finite/non-negative"),
    }

    let Some(observability_channels) = observability.get("channels").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_OBSERVABILITY_CHANNELS", "$.observability.channels", "exact observability case must contain channel declarations");
        return errors;
    };
    let declared_channel_ids: BTreeSet<&str> = observability_channels
        .iter()
        .filter_map(|channel| text(channel, "channel_id"))
        .collect();
    let declared_groups: BTreeSet<&str> = observability_channels
        .iter()
        .filter_map(|channel| text(channel, "independence_group_ref"))
        .collect();

    let Some(candidates) = case.get("candidates").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_CANDIDATES", "$.candidates", "candidates must be an array");
        return errors;
    };
    if candidates.is_empty() {
        push(&mut errors, "EMPTY_CANDIDATES", "$.candidates", "at least one candidate sensor is required");
    }
    if candidates.len() > MAX_EXACT_SEARCH_LIMIT {
        push(&mut errors, "SEARCH_SPACE_LIMIT", "$.candidates", "exact v0.1 enumeration supports at most 20 candidate sensors");
    }

    let mut candidate_ids = BTreeSet::new();
    let mut candidate_channels = BTreeSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = format!("$.candidates[{index}]");
        let id = text(candidate, "candidate_id");
        let channel_id = text(candidate, "channel_id");
        if id.is_none() || channel_id.is_none() {
            push(&mut errors, "MISSING_CANDIDATE_IDENTITY", &path, "candidate_id and channel_id are required");
            continue;
        }
        if !candidate_ids.insert(id.unwrap().to_owned()) {
            push(&mut errors, "DUPLICATE_CANDIDATE_ID", &format!("{path}.candidate_id"), "candidate IDs must be unique");
        }
        if !candidate_channels.insert(channel_id.unwrap().to_owned()) {
            push(&mut errors, "DUPLICATE_CANDIDATE_CHANNEL", &format!("{path}.channel_id"), "only one candidate record may map to a given observability channel in v0.1");
        }
        if !declared_channel_ids.contains(channel_id.unwrap()) {
            push(&mut errors, "UNKNOWN_CANDIDATE_CHANNEL", &format!("{path}.channel_id"), "candidate channel must exist in the referenced observability case");
        }
        match finite(candidate, "lifecycle_cost_units") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_CANDIDATE_COST", &format!("{path}.lifecycle_cost_units"), "candidate lifecycle cost must be finite and non-negative"),
        }
        match finite(candidate, "maintenance_hours_per_year") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_MAINTENANCE_BURDEN", &format!("{path}.maintenance_hours_per_year"), "maintenance burden must be finite and non-negative"),
        }
        match finite(candidate, "planned_unavailability_hours_per_year") {
            Some(v) if v >= 0.0 => {}
            _ => push(&mut errors, "INVALID_PLANNED_UNAVAILABILITY", &format!("{path}.planned_unavailability_hours_per_year"), "planned unavailability must be finite and non-negative"),
        }
        if text(candidate, "cost_unit") != text(profile, "cost_unit") {
            push(&mut errors, "COST_UNIT_MISMATCH", &format!("{path}.cost_unit"), "all lifecycle cost records must use the exact placement-profile cost unit");
        }
        if text(candidate, "cost_source_ref").is_none() {
            push(&mut errors, "MISSING_COST_SOURCE", &format!("{path}.cost_source_ref"), "lifecycle cost requires source provenance");
        }
        if !matches!(
            text(candidate, "source_class"),
            Some("SYNTHETIC_COST_FIXTURE" | "VENDOR_QUOTE" | "FIELD_MAINTENANCE_LOG" | "ENGINEERING_ESTIMATE")
        ) {
            push(&mut errors, "INVALID_COST_SOURCE_CLASS", &format!("{path}.source_class"), "cost source class is outside v0.1 vocabulary");
        }
    }

    let Some(scenarios) = case.get("outage_scenarios").and_then(Value::as_array) else {
        push(&mut errors, "MISSING_OUTAGE_SCENARIOS", "$.outage_scenarios", "outage_scenarios must be an array");
        return errors;
    };
    let mut scenario_ids = BTreeSet::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let path = format!("$.outage_scenarios[{index}]");
        let Some(id) = text(scenario, "scenario_id") else {
            push(&mut errors, "MISSING_OUTAGE_SCENARIO_ID", &format!("{path}.scenario_id"), "scenario_id is required");
            continue;
        };
        if !scenario_ids.insert(id.to_owned()) {
            push(&mut errors, "DUPLICATE_OUTAGE_SCENARIO_ID", &format!("{path}.scenario_id"), "scenario IDs must be unique");
        }
        for (field, declared) in [
            ("failed_channel_ids", &declared_channel_ids),
        ] {
            for channel in scenario.get(field).and_then(Value::as_array).into_iter().flatten() {
                if channel.as_str().is_none_or(|value| !declared.contains(value)) {
                    push(&mut errors, "UNKNOWN_OUTAGE_CHANNEL", &format!("{path}.{field}"), "outage scenario channel must exist in the observability case");
                }
            }
        }
        for group in scenario.get("failed_independence_group_refs").and_then(Value::as_array).into_iter().flatten() {
            if group.as_str().is_none_or(|value| !declared_groups.contains(value)) {
                push(&mut errors, "UNKNOWN_OUTAGE_GROUP", &format!("{path}.failed_independence_group_refs"), "outage scenario independence group must exist in the observability case");
            }
        }
    }

    errors
}

fn pair_status(
    observability: &Value,
    pair: &Value,
    selected_channels: &BTreeSet<String>,
    failed_channels: &BTreeSet<String>,
    failed_groups: &BTreeSet<String>,
) -> PairScenarioResult {
    let channels = observability["channels"].as_array().expect("validated observability channels");
    let states: BTreeMap<&str, &Value> = observability["states"]
        .as_array().expect("states")
        .iter()
        .map(|state| (text(state, "state_id").expect("state id"), state))
        .collect();
    let responses: BTreeMap<(&str, &str), &Value> = observability["responses"]
        .as_array().expect("responses")
        .iter()
        .map(|response| (
            (
                text(response, "state_id").expect("response state"),
                text(response, "channel_id").expect("response channel"),
            ),
            response,
        ))
        .collect();

    let state_a = text(pair, "state_a_id").expect("state A");
    let state_b = text(pair, "state_b_id").expect("state B");
    let mut groups = BTreeSet::new();
    let mut unresolved = false;

    for channel in channels {
        let channel_id = text(channel, "channel_id").expect("channel ID");
        if !selected_channels.contains(channel_id) || failed_channels.contains(channel_id) {
            continue;
        }
        let group = text(channel, "independence_group_ref").expect("independence group");
        if failed_groups.contains(group) {
            continue;
        }
        let a = responses[&(state_a, channel_id)];
        let b = responses[&(state_b, channel_id)];

        for response in [a, b] {
            match text(response, "evidence_role") {
                Some("INDEPENDENT_MEASUREMENT") if text(observability, "evidence_class") == Some("MEASURED") => {}
                Some("SYNTHETIC_CONFORMANCE_FIXTURE") if text(observability, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE") => {}
                _ => unresolved = true,
            }
            if text(response, "availability") == Some("UNKNOWN") {
                unresolved = true;
            }
        }

        if text(a, "availability") != Some("VALID")
            || text(b, "availability") != Some("VALID")
        {
            let known_fault = known_fault_unavailability(observability, state_a, channel_id, a)
                || known_fault_unavailability(observability, state_b, channel_id, b);
            if !known_fault {
                unresolved = true;
            }
            continue;
        }
        let (Some(a_lower), Some(a_upper), Some(b_lower), Some(b_upper)) = (
            finite(a, "value_lower"),
            finite(a, "value_upper"),
            finite(b, "value_lower"),
            finite(b, "value_upper"),
        ) else {
            unresolved = true;
            continue;
        };
        if a_upper < b_lower || b_upper < a_lower {
            groups.insert(group.to_owned());
        }
    }

    let required = pair["minimum_independent_principles"].as_u64().unwrap_or(1) as usize;
    let status = if groups.len() >= required {
        "IDENTIFIABLE"
    } else if unresolved {
        "INSUFFICIENT_EVIDENCE"
    } else {
        "NOT_IDENTIFIABLE"
    };
    PairScenarioResult {
        pair_id: text(pair, "pair_id").unwrap_or("unknown").to_owned(),
        status: status.to_owned(),
        observed_independent_principles: groups.len(),
        required_independent_principles: required,
    }
}

fn evaluate_scenario(
    observability: &Value,
    pairs: &[Value],
    selected_channels: &BTreeSet<String>,
    scenario_id: &str,
    failed_channels: &BTreeSet<String>,
    failed_groups: &BTreeSet<String>,
) -> ScenarioCoverage {
    let pair_results: Vec<PairScenarioResult> = pairs
        .iter()
        .map(|pair| pair_status(observability, pair, selected_channels, failed_channels, failed_groups))
        .collect();
    let identifiable_pairs = pair_results.iter().filter(|p| p.status == "IDENTIFIABLE").count();
    let total_critical_pairs = pairs.iter().filter(|p| p["critical"].as_bool() == Some(true)).count();
    let identifiable_critical_pairs = pairs.iter().zip(pair_results.iter())
        .filter(|(pair, result)| pair["critical"].as_bool() == Some(true) && result.status == "IDENTIFIABLE")
        .count();
    ScenarioCoverage {
        scenario_id: scenario_id.to_owned(),
        identifiable_pairs,
        total_pairs: pairs.len(),
        identifiable_critical_pairs,
        total_critical_pairs,
        pair_results,
    }
}

fn dominates(a: &ParetoLayout, b: &ParetoLayout) -> bool {
    let no_worse = a.lifecycle_cost_units <= b.lifecycle_cost_units
        && a.channel_count <= b.channel_count
        && a.maintenance_hours_per_year <= b.maintenance_hours_per_year
        && a.planned_unavailability_hours_per_year <= b.planned_unavailability_hours_per_year
        && a.worst_case_identifiable_pairs >= b.worst_case_identifiable_pairs;
    let strictly_better = a.lifecycle_cost_units < b.lifecycle_cost_units
        || a.channel_count < b.channel_count
        || a.maintenance_hours_per_year < b.maintenance_hours_per_year
        || a.planned_unavailability_hours_per_year < b.planned_unavailability_hours_per_year
        || a.worst_case_identifiable_pairs > b.worst_case_identifiable_pairs;
    no_worse && strictly_better
}

/// Enumerate all non-empty sensor subsets and return the exact Pareto set.
/// The search is deliberately capped at 20 candidates to keep v0.1 exact.
pub fn optimize_layout(
    observability: &Value,
    placement: &Value,
) -> Result<SearchReport, Vec<Violation>> {
    let errors = validate_case(placement, observability);
    if !errors.is_empty() {
        return Err(errors);
    }

    let candidates = placement["candidates"].as_array().expect("validated candidates");
    let profile = &placement["placement_profile"];
    let pairs = observability["required_state_pairs"].as_array().expect("validated pairs");
    let channels: BTreeMap<&str, &Value> = observability["channels"]
        .as_array().expect("validated channels")
        .iter()
        .map(|channel| (text(channel, "channel_id").expect("channel ID"), channel))
        .collect();
    let budget = profile.get("max_lifecycle_cost_units").and_then(Value::as_f64);
    let max_channels = profile["maximum_selected_channels"].as_u64().unwrap() as usize;
    let require_nominal = profile["require_all_critical_pairs_nominal"].as_bool().unwrap_or(false);
    let require_outages = profile["require_all_critical_pairs_under_outages"].as_bool().unwrap_or(false);
    let cost_unit = text(profile, "cost_unit").expect("cost unit").to_owned();
    let candidate_count = candidates.len();
    let subset_limit = 1u64 << candidate_count;

    let mut exact_subsets_examined = 0u64;
    let mut subsets_over_budget = 0u64;
    let mut subsets_over_channel_limit = 0u64;
    let mut feasible_layouts = Vec::new();

    for mask in 1u64..subset_limit {
        exact_subsets_examined += 1;
        let selected: Vec<&Value> = candidates.iter().enumerate()
            .filter_map(|(index, candidate)| if mask & (1u64 << index) != 0 { Some(candidate) } else { None })
            .collect();
        if selected.len() > max_channels {
            subsets_over_channel_limit += 1;
            continue;
        }
        let lifecycle_cost: f64 = selected.iter().map(|candidate| finite(candidate, "lifecycle_cost_units").unwrap()).sum();
        if budget.is_some_and(|limit| lifecycle_cost > limit) {
            subsets_over_budget += 1;
            continue;
        }

        let mut candidate_ids: Vec<String> = selected.iter().map(|c| text(c, "candidate_id").unwrap().to_owned()).collect();
        let mut channel_ids: Vec<String> = selected.iter().map(|c| text(c, "channel_id").unwrap().to_owned()).collect();
        candidate_ids.sort();
        channel_ids.sort();
        let selected_channels: BTreeSet<String> = channel_ids.iter().cloned().collect();
        let maintenance_hours: f64 = selected.iter().map(|c| finite(c, "maintenance_hours_per_year").unwrap()).sum();
        let planned_unavailability: f64 = selected.iter().map(|c| finite(c, "planned_unavailability_hours_per_year").unwrap()).sum();

        let mut scenarios = Vec::new();
        scenarios.push(evaluate_scenario(
            observability, pairs, &selected_channels, "NOMINAL",
            &BTreeSet::new(), &BTreeSet::new()
        ));
        for scenario in placement["outage_scenarios"].as_array().unwrap() {
            let failed_channels: BTreeSet<String> = scenario["failed_channel_ids"].as_array().unwrap()
                .iter().filter_map(Value::as_str).map(str::to_owned).collect();
            let failed_groups: BTreeSet<String> = scenario["failed_independence_group_refs"].as_array().unwrap()
                .iter().filter_map(Value::as_str).map(str::to_owned).collect();
            scenarios.push(evaluate_scenario(
                observability, pairs, &selected_channels,
                text(scenario, "scenario_id").unwrap(),
                &failed_channels, &failed_groups
            ));
        }

        let critical_ok_nominal = scenarios.first().is_some_and(|s| s.identifiable_critical_pairs == s.total_critical_pairs);
        let critical_ok_outages = scenarios.iter().skip(1).all(|s| s.identifiable_critical_pairs == s.total_critical_pairs);
        let feasible = (!require_nominal || critical_ok_nominal) && (!require_outages || critical_ok_outages);
        if !feasible {
            continue;
        }
        let worst_case_identifiable_pairs = scenarios.iter().map(|s| s.identifiable_pairs).min().unwrap_or(0);
        feasible_layouts.push(ParetoLayout {
            candidate_ids,
            channel_ids,
            lifecycle_cost_units: lifecycle_cost,
            cost_unit: cost_unit.clone(),
            channel_count: selected.len(),
            maintenance_hours_per_year: maintenance_hours,
            planned_unavailability_hours_per_year: planned_unavailability,
            worst_case_identifiable_pairs,
            total_required_pairs: pairs.len(),
            feasible: true,
            scenarios,
        });
    }

    let mut pareto = Vec::new();
    for (index, candidate) in feasible_layouts.iter().enumerate() {
        let dominated = feasible_layouts.iter().enumerate().any(|(other_index, other)| {
            index != other_index && dominates(other, candidate)
        });
        if !dominated {
            pareto.push(candidate.clone());
        }
    }
    pareto.sort_by(|a, b| {
        a.lifecycle_cost_units.total_cmp(&b.lifecycle_cost_units)
            .then(a.channel_count.cmp(&b.channel_count))
            .then(a.maintenance_hours_per_year.total_cmp(&b.maintenance_hours_per_year))
            .then(a.planned_unavailability_hours_per_year.total_cmp(&b.planned_unavailability_hours_per_year))
            .then(a.channel_ids.cmp(&b.channel_ids))
    });

    Ok(SearchReport {
        case_id: text(placement, "case_id").unwrap().to_owned(),
        observability_case_id: text(observability, "case_id").unwrap().to_owned(),
        evidence_disposition: if text(placement, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE")
            || text(observability, "evidence_class") == Some("SYNTHETIC_CONFORMANCE_FIXTURE")
        {
            "SYNTHETIC_ONLY"
        } else {
            "MEASURED_COSTS_AND_RESPONSES_REVIEW_REQUIRED"
        }.to_owned(),
        candidate_count,
        exact_subsets_examined,
        subsets_over_budget,
        subsets_over_channel_limit,
        feasible_layout_count: feasible_layouts.len(),
        pareto_layout_count: pareto.len(),
        pareto_layouts: pareto,
        search_method: "EXACT_EXHAUSTIVE_SUBSET_ENUMERATION".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observability() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-sensor-observability-case.v0_1.json"
        )).expect("observability fixture parses")
    }

    fn placement() -> Value {
        serde_json::from_str(include_str!(
            "../../../../docs/research/rock-extraction-portfolio/fixtures/synthetic-sensor-placement-case.v0_1.json"
        )).expect("placement fixture parses")
    }

    fn has(errors: &[Violation], code: &str) -> bool {
        errors.iter().any(|error| error.code == code)
    }

    #[test]
    fn synthetic_placement_profile_is_valid() {
        assert!(validate_case(&placement(), &observability()).is_empty());
    }

    #[test]
    fn exact_search_examines_all_nonempty_subsets() {
        let report = optimize_layout(&observability(), &placement()).unwrap();
        assert_eq!(report.exact_subsets_examined, 127);
        assert_eq!(report.candidate_count, 7);
        assert_eq!(report.search_method, "EXACT_EXHAUSTIVE_SUBSET_ENUMERATION");
    }

    #[test]
    fn every_returned_layout_is_feasible_under_all_declared_scenarios() {
        let report = optimize_layout(&observability(), &placement()).unwrap();
        assert!(!report.pareto_layouts.is_empty());
        assert!(report.pareto_layouts.iter().all(|layout| {
            layout.feasible && layout.scenarios.iter().all(|s| s.identifiable_critical_pairs == s.total_critical_pairs)
        }));
    }

    #[test]
    fn pareto_layouts_respect_cost_budget_and_channel_limit() {
        let case = placement();
        let report = optimize_layout(&observability(), &case).unwrap();
        let budget = case["placement_profile"]["max_lifecycle_cost_units"].as_f64().unwrap();
        let maximum = case["placement_profile"]["maximum_selected_channels"].as_u64().unwrap() as usize;
        assert!(report.pareto_layouts.iter().all(|layout| layout.lifecycle_cost_units <= budget && layout.channel_count <= maximum));
    }

    #[test]
    fn returned_layouts_are_pairwise_nondominated() {
        let report = optimize_layout(&observability(), &placement()).unwrap();
        for i in 0..report.pareto_layouts.len() {
            for j in 0..report.pareto_layouts.len() {
                if i != j {
                    assert!(!dominates(&report.pareto_layouts[i], &report.pareto_layouts[j]));
                }
            }
        }
    }

    #[test]
    fn synthetic_costs_never_become_currency_claims() {
        let report = optimize_layout(&observability(), &placement()).unwrap();
        assert_eq!(report.evidence_disposition, "SYNTHETIC_ONLY");
        assert!(report.pareto_layouts.iter().all(|layout| layout.cost_unit == "SYNTHETIC_COST_POINTS"));
    }

    #[test]
    fn exact_search_rejects_more_than_twenty_candidates() {
        let mut case = placement();
        let candidate = case["candidates"][0].clone();
        for index in 0..14 {
            let mut copy = candidate.clone();
            copy["candidate_id"] = serde_json::json!(format!("extra-{index}"));
            copy["channel_id"] = serde_json::json!("dp-line");
            case["candidates"].as_array_mut().unwrap().push(copy);
        }
        assert!(has(&validate_case(&case, &observability()), "SEARCH_SPACE_LIMIT"));
    }

    #[test]
    fn duplicate_candidate_channel_is_rejected() {
        let mut case = placement();
        let mut copy = case["candidates"][0].clone();
        copy["candidate_id"] = serde_json::json!("duplicate-channel-candidate");
        case["candidates"].as_array_mut().unwrap().push(copy);
        assert!(has(&validate_case(&case, &observability()), "DUPLICATE_CANDIDATE_CHANNEL"));
    }

    #[test]
    fn lifecycle_cost_unit_must_match_profile() {
        let mut case = placement();
        case["candidates"][0]["cost_unit"] = serde_json::json!("USD");
        assert!(has(&validate_case(&case, &observability()), "COST_UNIT_MISMATCH"));
    }

    #[test]
    fn unknown_candidate_channel_is_rejected() {
        let mut case = placement();
        case["candidates"][0]["channel_id"] = serde_json::json!("not-a-channel");
        assert!(has(&validate_case(&case, &observability()), "UNKNOWN_CANDIDATE_CHANNEL"));
    }

    #[test]
    fn unknown_outage_group_is_rejected() {
        let mut case = placement();
        case["outage_scenarios"][0]["failed_independence_group_refs"] =
            serde_json::json!(["not-a-principle"]);
        assert!(has(&validate_case(&case, &observability()), "UNKNOWN_OUTAGE_GROUP"));
    }

    #[test]
    fn incomplete_observability_matrix_is_rejected() {
        let mut obs = observability();
        obs["responses"].as_array_mut().unwrap().pop();
        assert!(has(
            &validate_case(&placement(), &obs),
            "OBSERVABILITY_RESPONSE_MATRIX_INCOMPLETE"
        ));
    }

    #[test]
    fn wrong_observability_case_id_is_rejected() {
        let mut case = placement();
        case["observability_case_id"] = serde_json::json!("other-case");
        assert!(has(&validate_case(&case, &observability()), "OBSERVABILITY_CASE_ID_MISMATCH"));
    }

    #[test]
    fn negative_lifecycle_cost_is_rejected() {
        let mut case = placement();
        case["candidates"][0]["lifecycle_cost_units"] = serde_json::json!(-1.0);
        assert!(has(&validate_case(&case, &observability()), "INVALID_CANDIDATE_COST"));
    }

    #[test]
    fn over_budget_subsets_are_counted_and_excluded() {
        let mut case = placement();
        case["placement_profile"]["max_lifecycle_cost_units"] = serde_json::json!(1.0);
        let report = optimize_layout(&observability(), &case).unwrap();
        assert_eq!(report.feasible_layout_count, 0);
        assert!(report.subsets_over_budget > 0);
        assert!(report.pareto_layouts.is_empty());
    }

    #[test]
    fn pressure_group_outage_is_evaluated_separately_from_nominal() {
        let report = optimize_layout(&observability(), &placement()).unwrap();
        let layout = report.pareto_layouts.first().unwrap();
        assert!(layout.scenarios.iter().any(|s| s.scenario_id == "pressure-principle-unavailable"));
        assert!(layout.scenarios.iter().any(|s| s.scenario_id == "airflow-principle-unavailable"));
    }

    #[test]
    fn deterministic_output_under_candidate_permutation() {
        let mut case = placement();
        case["candidates"].as_array_mut().unwrap().reverse();
        assert_eq!(
            optimize_layout(&observability(), &placement()).unwrap(),
            optimize_layout(&observability(), &case).unwrap()
        );
    }

    #[test]
    fn maximum_selected_channels_is_enforced() {
        let mut case = placement();
        case["placement_profile"]["maximum_selected_channels"] = serde_json::json!(2);
        let report = optimize_layout(&observability(), &case).unwrap();
        assert!(report.pareto_layouts.iter().all(|layout| layout.channel_count <= 2));
    }

    #[test]
    fn critical_pairs_must_be_identifiable_in_every_required_outage_scenario() {
        let mut case = placement();
        case["outage_scenarios"].as_array_mut().unwrap().push(serde_json::json!({
            "scenario_id":"self-check-unavailable",
            "failed_channel_ids":["sensor-self-check"],
            "failed_independence_group_refs":[]
        }));
        let report = optimize_layout(&observability(), &case).unwrap();
        assert!(report.pareto_layouts.is_empty());
    }

    #[test]
    fn deterministic_replay() {
        let obs = observability();
        let place = placement();
        assert_eq!(
            optimize_layout(&obs, &place).unwrap(),
            optimize_layout(&obs, &place).unwrap()
        );
    }
}
