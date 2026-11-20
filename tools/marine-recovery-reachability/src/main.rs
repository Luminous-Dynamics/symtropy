use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-recovery-reachability/0.1.0";
const METRICS: [(&str, &str, &str); 11] = [
    ("energy_reserve_kwh", "kWh", "gte"),
    ("freshwater_usable_l", "L", "gte"),
    ("food_reserve_days", "day", "gte"),
    ("waste_capacity_margin_days", "day", "gte"),
    ("critical_spares_coverage_days", "day", "gte"),
    ("power_service_availability", "fraction", "gte"),
    ("potable_water_service_availability", "fraction", "gte"),
    ("sanitation_service_availability", "fraction", "gte"),
    ("refrigeration_service_availability", "fraction", "gte"),
    ("crew_workload_hours_rolling_24h", "h", "lte"),
    ("comfort_breach_minutes_rolling_24h", "min", "lte"),
];
const NUMERIC_TOLERANCE: f64 = 1e-9;
const ACTION_CLASSES: [&str; 9] = [
    "load_shed",
    "divert_to_safe_haven",
    "shore_resupply",
    "repair",
    "controlled_hold",
    "human_handover",
    "change_sail_configuration",
    "switch_power_mode",
    "continue_route",
];

#[derive(Clone)]
struct ContractMetric {
    unit: String,
    operator: String,
    floor: f64,
    minimum_coverage: f64,
    max_abs_rate_per_hour: f64,
}

#[derive(Clone, Copy, Debug)]
struct Interval {
    value: f64,
    lower: f64,
    upper: f64,
    coverage: f64,
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64).filter(|number| number.is_finite())
}

fn valid_digest(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn canonical_digest(value: &Value, excluded_key: Option<&str>) -> Option<String> {
    let mut canonical = value.clone();
    if let Some(key) = excluded_key {
        canonical.as_object_mut()?.remove(key);
    }
    let bytes = serde_json::to_vec(&canonical).ok()?;
    Some(sha256_hex(&bytes))
}

fn parse_time(raw: Option<&str>) -> Option<DateTime<FixedOffset>> {
    raw.and_then(|value| DateTime::parse_from_rfc3339(value).ok())
}

fn lower_cone_minimum(a: f64, b: f64, rate: f64, duration: f64) -> f64 {
    if rate <= NUMERIC_TOLERANCE || duration <= NUMERIC_TOLERANCE {
        return a.min(b);
    }
    let intersection = (a - b + rate * duration) / (2.0 * rate);
    if (0.0..=duration).contains(&intersection) {
        (a + b - rate * duration) / 2.0
    } else if intersection < 0.0 {
        a.max(b - rate * duration)
    } else {
        (a - rate * duration).max(b)
    }
}

fn upper_cone_maximum(a: f64, b: f64, rate: f64, duration: f64) -> f64 {
    if rate <= NUMERIC_TOLERANCE || duration <= NUMERIC_TOLERANCE {
        return a.max(b);
    }
    let intersection = (b - a + rate * duration) / (2.0 * rate);
    if (0.0..=duration).contains(&intersection) {
        (a + b + rate * duration) / 2.0
    } else if intersection < 0.0 {
        a.min(b + rate * duration)
    } else {
        (a + rate * duration).min(b)
    }
}

fn parse_contract(value: &Value, errors: &mut Vec<String>) -> BTreeMap<String, ContractMetric> {
    let mut result = BTreeMap::<String, ContractMetric>::new();
    let Some(items) = value.as_array() else {
        errors.push("service_contract must be an array".to_owned());
        return result;
    };
    if items.len() != METRICS.len() {
        errors.push("service_contract must define exactly eleven resource/service axes".to_owned());
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let floor = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        let rate = number_at(item, "max_abs_rate_per_hour");
        if metric.is_empty() {
            errors.push("service contract contains an empty metric ID".to_owned());
            continue;
        }
        let Some((_, expected_unit, expected_operator)) =
            METRICS.iter().find(|(name, _, _)| *name == metric)
        else {
            errors.push(format!("service contract contains unknown metric '{metric}'"));
            continue;
        };
        if unit != *expected_unit || operator != *expected_operator {
            errors.push(format!("service contract metric '{metric}' has invalid unit/operator"));
        }
        let (Some(floor), Some(coverage), Some(rate)) = (floor, coverage, rate) else {
            errors.push(format!("service contract metric '{metric}' has missing/non-finite floor, coverage or rate"));
            continue;
        };
        if !(0.0..=1.0).contains(&coverage) || rate < 0.0 {
            errors.push(format!("service contract metric '{metric}' has invalid coverage/rate bound"));
            continue;
        }
        if string_at(item, "rate_bound_method").is_none_or(str::is_empty)
            || !["measured", "modeled", "derived", "synthetic"].contains(
                &string_at(item, "rate_bound_class").unwrap_or("")
            )
            || !valid_digest(string_at(item, "rate_bound_evidence_sha256"))
        {
            errors.push(format!("service contract metric '{metric}' lacks rate-bound method/class/evidence"));
        }
        let contract_metric = ContractMetric {
            unit: unit.to_owned(),
            operator: operator.to_owned(),
            floor,
            minimum_coverage: coverage,
            max_abs_rate_per_hour: rate,
        };
        if result.insert(metric.to_owned(), contract_metric).is_some() {
            errors.push(format!("service contract duplicates metric '{metric}'"));
        }
    }
    for (metric, _, _) in METRICS {
        if !result.contains_key(metric) {
            errors.push(format!("service contract is missing '{metric}'"));
        }
    }
    result
}

fn parse_state(value: &Value, contract: &BTreeMap<String, ContractMetric>, errors: &mut Vec<String>, label: &str) -> Option<BTreeMap<String, Interval>> {
    let items = value.as_array()?;
    if items.len() != METRICS.len() {
        errors.push(format!("{label} must contain exactly eleven service/resource intervals"));
    }
    let mut state = BTreeMap::<String, Interval>::new();
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let (Some(value), Some(lower), Some(upper), Some(coverage)) = (
            number_at(item, "value"),
            number_at(item, "lower_bound"),
            number_at(item, "upper_bound"),
            number_at(item, "coverage_fraction"),
        ) else {
            errors.push(format!("{label} metric '{metric}' has missing/non-finite interval data"));
            continue;
        };
        let Some(spec) = contract.get(metric) else {
            errors.push(format!("{label} has unknown metric '{metric}'"));
            continue;
        };
        if unit != spec.unit || !valid_digest(string_at(item, "evidence_sha256")) {
            errors.push(format!("{label} metric '{metric}' has invalid unit or evidence digest"));
        }
        if lower < 0.0 || lower > value || value > upper
            || !(0.0..=1.0).contains(&coverage)
            || coverage < spec.minimum_coverage
        {
            errors.push(format!("{label} metric '{metric}' has invalid/insufficient uncertainty coverage"));
        }
        if state.insert(metric.to_owned(), Interval { value, lower, upper, coverage }).is_some() {
            errors.push(format!("{label} duplicates metric '{metric}'"));
        }
    }
    for (metric, _, _) in METRICS {
        if !state.contains_key(metric) {
            errors.push(format!("{label} is missing metric '{metric}'"));
        }
    }
    Some(state)
}

fn floor_margin(spec: &ContractMetric, interval: Interval) -> f64 {
    match spec.operator.as_str() {
        "gte" => interval.lower - spec.floor,
        "lte" => spec.floor - interval.upper,
        _ => f64::NEG_INFINITY,
    }
}

fn state_floor_margins(
    state: &BTreeMap<String, Interval>,
    contract: &BTreeMap<String, ContractMetric>,
) -> BTreeMap<String, f64> {
    let mut margins = BTreeMap::new();
    for (metric, interval) in state {
        if let Some(spec) = contract.get(metric) {
            margins.insert(metric.clone(), floor_margin(spec, *interval));
        }
    }
    margins
}

fn apply_action(
    action: &Value,
    state: &BTreeMap<String, Interval>,
    scenario: &Value,
    contract: &BTreeMap<String, ContractMetric>,
    errors: &mut Vec<String>,
) -> Result<(BTreeMap<String, Interval>, BTreeMap<String, f64>), Vec<String>> {
    let action_id = string_at(action, "action_id").unwrap_or("unknown");
    let mut reasons = Vec::<String>::new();

    let action_class = string_at(action, "action_class").unwrap_or("");
    if !ACTION_CLASSES.contains(&action_class) {
        reasons.push("unknown_action_class".to_owned());
    }
    if ["shore_resupply", "controlled_hold", "human_handover"].contains(&action_class) {
        if string_at(action, "authority_receipt_ref").is_none_or(str::is_empty)
            || !valid_digest(string_at(action, "authority_receipt_sha256"))
        {
            reasons.push("action_requires_authority_receipt".to_owned());
        }
    }
    let trigger = string_at(scenario, "trigger_class").unwrap_or("");
    let allowed_triggers = action.get("trigger_classes").and_then(Value::as_array);
    if !allowed_triggers.is_some_and(|items| items.iter().any(|item| item.as_str() == Some(trigger))) {
        reasons.push("trigger_not_admitted_for_action".to_owned());
    }

    let available_components = scenario.get("available_components").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if let Some(required) = action.get("requires_components").and_then(Value::as_array) {
        for component in required.iter().filter_map(Value::as_str) {
            if !available_components.contains(component) {
                reasons.push(format!("required_component_unavailable:{component}"));
            }
        }
    } else {
        reasons.push("missing_component_preconditions".to_owned());
    }

    let available_capabilities = scenario.get("available_capabilities").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    if let Some(required) = action.get("requires_capabilities").and_then(Value::as_array) {
        for capability in required.iter().filter_map(Value::as_str) {
            if !available_capabilities.contains(capability) {
                reasons.push(format!("required_capability_unavailable:{capability}"));
            }
        }
    } else {
        reasons.push("missing_capability_preconditions".to_owned());
    }

    let duration_lower = number_at(action, "duration_hours_lower").unwrap_or(f64::NAN);
    let duration_upper = number_at(action, "duration_hours_upper").unwrap_or(f64::NAN);
    if !duration_lower.is_finite() || !duration_upper.is_finite()
        || duration_lower <= 0.0 || duration_upper < duration_lower
    {
        reasons.push("invalid_action_duration_interval".to_owned());
    }

    let required_gates = scenario.get("required_hard_gate_ids").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let checks = action.get("hard_gate_checks").and_then(Value::as_array);
    let mut seen_gates = BTreeSet::<String>::new();
    if let Some(checks) = checks {
        for check in checks {
            let gate = string_at(check, "gate_id").unwrap_or("");
            if gate.is_empty() || !seen_gates.insert(gate.to_owned()) {
                errors.push(format!("action '{action_id}' has missing/duplicate hard-gate check"));
                continue;
            }
            if !valid_digest(string_at(check, "evidence_sha256")) {
                errors.push(format!("action '{action_id}' hard gate '{gate}' lacks evidence digest"));
            }
            if string_at(check, "status") != Some("pass") {
                reasons.push(format!("hard_gate_not_passed:{gate}"));
            }
        }
    } else {
        reasons.push("missing_hard_gate_checks".to_owned());
    }
    if seen_gates != required_gates {
        reasons.push("hard_gate_roster_incomplete_or_extra".to_owned());
    }

    let deltas = action.get("metric_deltas").and_then(Value::as_array);
    let mut next_state = BTreeMap::<String, Interval>::new();
    let mut margins = BTreeMap::<String, f64>::new();
    if deltas.is_none_or(|items| items.len() != METRICS.len()) {
        reasons.push("action_metric_delta_roster_incomplete".to_owned());
    } else if let Some(deltas) = deltas {
        let mut seen = BTreeSet::<String>::new();
        for delta in deltas {
            let metric = string_at(delta, "metric").unwrap_or("");
            let unit = string_at(delta, "unit").unwrap_or("");
            let (Some(lower_delta), Some(upper_delta)) =
                (number_at(delta, "lower_delta"), number_at(delta, "upper_delta"))
            else {
                reasons.push(format!("invalid_action_delta:{metric}"));
                continue;
            };
            if !seen.insert(metric.to_owned()) {
                reasons.push(format!("duplicate_action_delta:{metric}"));
                continue;
            }
            let Some(spec) = contract.get(metric) else {
                reasons.push(format!("unknown_action_metric:{metric}"));
                continue;
            };
            if unit != spec.unit || lower_delta > upper_delta
                || !valid_digest(string_at(delta, "evidence_sha256"))
            {
                reasons.push(format!("invalid_action_delta_contract:{metric}"));
                continue;
            }
            let Some(previous) = state.get(metric) else {
                reasons.push(format!("missing_pre_action_state:{metric}"));
                continue;
            };
            let next = Interval {
                value: previous.value + (lower_delta + upper_delta) / 2.0,
                lower: previous.lower + lower_delta,
                upper: previous.upper + upper_delta,
                coverage: previous.coverage,
            };
            if next.lower < 0.0 || next.lower > next.value || next.value > next.upper {
                reasons.push(format!("invalid_successor_interval:{metric}"));
                continue;
            }
            let rate = spec.max_abs_rate_per_hour;
            let minimum_duration = number_at(action, "duration_hours_lower").unwrap_or(f64::NAN).max(0.0);
            let maximum_duration = duration_upper.max(0.0);
            if (next.lower - previous.lower).abs() > rate * minimum_duration + NUMERIC_TOLERANCE
                || (next.upper - previous.upper).abs() > rate * minimum_duration + NUMERIC_TOLERANCE
            {
                reasons.push(format!("endpoint_change_exceeds_declared_rate_bound_at_minimum_duration:{metric}"));
                continue;
            }
            let envelope_margin = match spec.operator.as_str() {
                "gte" => lower_cone_minimum(previous.lower, next.lower, rate, maximum_duration) - spec.floor,
                "lte" => spec.floor - upper_cone_maximum(previous.upper, next.upper, rate, maximum_duration),
                _ => f64::NEG_INFINITY,
            };
            let end_margin = floor_margin(spec, next);
            let margin = envelope_margin.min(end_margin);
            margins.insert(metric.to_owned(), margin);
            if margin < -NUMERIC_TOLERANCE {
                reasons.push(format!("service_floor_breached_during_action:{metric}"));
            }
            next_state.insert(metric.to_owned(), next);
        }
        for (metric, _, _) in METRICS {
            if !seen.contains(metric) {
                reasons.push(format!("missing_action_delta:{metric}"));
            }
        }
    }

    if !reasons.is_empty() {
        return Err(reasons);
    }
    Ok((next_state, margins))
}

fn terminal_status(
    terminal: &Value,
    scenario_time: Option<DateTime<FixedOffset>>,
    elapsed_hours: f64,
    intervention_deadline_hours: f64,
) -> Result<f64, String> {
    let terminal_id = string_at(terminal, "terminal_id").unwrap_or("unknown");
    if string_at(terminal, "receipt_sha256") != canonical_digest(terminal, Some("receipt_sha256")).as_deref() {
        return Err(format!("terminal_receipt_digest_mismatch:{terminal_id}"));
    }
    for field in [
        "destination_location_sha256",
        "access_evidence_sha256",
        "service_availability_sha256",
        "terminal_evidence_sha256",
        "authority_receipt_sha256",
    ] {
        if !valid_digest(string_at(terminal, field)) {
            return Err(format!("terminal_missing_evidence:{terminal_id}:{field}"));
        }
    }
    if string_at(terminal, "authority_receipt_ref").is_none_or(str::is_empty) {
        return Err(format!("terminal_missing_authority_receipt:{terminal_id}"));
    }
    if terminal.get("available").and_then(Value::as_bool) != Some(true)
        || string_at(terminal, "access_status") != Some("verified")
    {
        return Err(format!("terminal_not_available_or_access_unverified:{terminal_id}"));
    }
    let objective = string_at(terminal, "objective_class").unwrap_or("");
    let service_status = string_at(terminal, "service_status").unwrap_or("");
    let service_ok = match objective {
        "shore_resupply" | "repair_restoration" => service_status == "verified",
        "safe_haven" | "controlled_safe_hold" => {
            service_status == "not_required" || service_status == "verified"
        }
        "human_handover" => service_status == "verified",
        _ => false,
    };
    if !service_ok {
        return Err(format!("terminal_service_not_verified:{terminal_id}"));
    }
    let human_response_upper_hours = if objective == "human_handover" {
        let acknowledged = terminal.get("human_acknowledged").and_then(Value::as_bool) == Some(true);
        let role = string_at(terminal, "human_authority_role").unwrap_or("");
        let communications = valid_digest(string_at(terminal, "communications_evidence_sha256"));
        let response = number_at(terminal, "human_response_upper_hours").unwrap_or(f64::NAN);
        if !acknowledged || role.is_empty() || role == "not_applicable" || !communications
            || !response.is_finite() || response < 0.0
        {
            return Err(format!("human_handover_acknowledgement_or_comms_unverified:{terminal_id}"));
        }
        if elapsed_hours + response > intervention_deadline_hours + NUMERIC_TOLERANCE {
            return Err(format!("human_handover_response_after_intervention_deadline:{terminal_id}"));
        }
        response
    } else {
        0.0
    };
    let deadline = parse_time(string_at(terminal, "arrival_deadline_at"))
        .ok_or_else(|| format!("terminal_deadline_invalid:{terminal_id}"))?;
    let valid_until = parse_time(string_at(terminal, "receipt_valid_until"))
        .ok_or_else(|| format!("terminal_receipt_expiry_invalid:{terminal_id}"))?;
    let Some(scenario_time) = scenario_time else {
        return Err(format!("scenario_decision_time_invalid:{terminal_id}"));
    };
    let arrival = scenario_time + Duration::seconds((elapsed_hours * 3600.0).ceil() as i64);
    if arrival > deadline {
        return Err(format!("terminal_arrival_after_deadline:{terminal_id}"));
    }
    if arrival > valid_until {
        return Err(format!("terminal_receipt_expired_before_arrival:{terminal_id}"));
    }
    Ok(human_response_upper_hours)
}

fn action_digest_valid(action: &Value) -> bool {
    valid_digest(string_at(action, "action_sha256"))
        && string_at(action, "action_sha256")
            == canonical_digest(action, Some("action_sha256")).as_deref()
        && valid_digest(string_at(action, "action_evidence_sha256"))
}

fn explore_paths(
    scenario: &Value,
    contract: &BTreeMap<String, ContractMetric>,
    actions: &[Value],
    terminals: &[Value],
    node: &str,
    state: BTreeMap<String, Interval>,
    elapsed_lower: f64,
    elapsed_upper: f64,
    path: &mut Vec<String>,
    visited_nodes: &mut BTreeSet<String>,
    accumulated_margins: &mut BTreeMap<String, f64>,
    depth_limit: usize,
    deadline_hours: f64,
    max_paths: usize,
    max_expanded_edges: usize,
    expanded_edges: &mut usize,
    feasible_paths: &mut Vec<Value>,
    rejected_edges: &mut Vec<Value>,
    errors: &mut Vec<String>,
    search_truncated: &mut bool,
) {
    let mut outgoing = actions.iter()
        .filter(|action| string_at(action, "from_node") == Some(node))
        .collect::<Vec<_>>();
    outgoing.sort_by_key(|action| string_at(action, "action_id").unwrap_or(""));
    if path.len() >= depth_limit && !outgoing.is_empty() {
        *search_truncated = true;
        return;
    }
    if feasible_paths.len() >= max_paths && !outgoing.is_empty() {
        *search_truncated = true;
        return;
    }
    for action in outgoing {
        if *expanded_edges >= max_expanded_edges {
            *search_truncated = true;
            break;
        }
        *expanded_edges += 1;
        if feasible_paths.len() >= max_paths {
            *search_truncated = true;
            break;
        }
        let action_id = string_at(action, "action_id").unwrap_or("unknown").to_owned();
        if !action_digest_valid(action) {
            errors.push(format!("action '{action_id}' digest/evidence invalid"));
            rejected_edges.push(json!({"action_id":action_id,"reasons":["action_digest_invalid"]}));
            continue;
        }
        let action_duration_lower = number_at(action, "duration_hours_lower").unwrap_or(f64::NAN);
        let action_duration_upper = number_at(action, "duration_hours_upper").unwrap_or(f64::NAN);
        let total_lower = elapsed_lower + action_duration_lower;
        let total_upper = elapsed_upper + action_duration_upper;
        if !total_upper.is_finite() || total_upper > deadline_hours + NUMERIC_TOLERANCE {
            rejected_edges.push(json!({
                "action_id":action_id,
                "reasons":["intervention_deadline_exceeded"],
                "elapsed_upper_hours":total_upper,
                "intervention_deadline_hours":deadline_hours
            }));
            continue;
        }
        let result = apply_action(action, &state, scenario, contract, errors);
        let (next_state, action_margins) = match result {
            Ok(result) => result,
            Err(reasons) => {
                rejected_edges.push(json!({"action_id":action_id,"reasons":reasons}));
                continue;
            }
        };
        if next_state.is_empty() {
            rejected_edges.push(json!({"action_id":action_id,"reasons":["empty_successor_state"]}));
            continue;
        }
        let next_node = string_at(action, "to_node").unwrap_or("");
        if next_node.is_empty() || visited_nodes.contains(next_node) {
            rejected_edges.push(json!({"action_id":action_id,"reasons":["empty_or_revisited_node"]}));
            continue;
        }
        path.push(action_id.clone());
        let previous_margins = accumulated_margins.clone();
        for (metric, margin) in action_margins {
            accumulated_margins.entry(metric)
                .and_modify(|current| *current = current.min(margin))
                .or_insert(margin);
        }

        let node_terminals = terminals.iter()
            .filter(|terminal| string_at(terminal, "node_id") == Some(next_node))
            .collect::<Vec<_>>();
        for terminal in node_terminals {
            let terminal_id = string_at(terminal, "terminal_id").unwrap_or("unknown").to_owned();
            let terminal_objective = string_at(terminal, "objective_class").unwrap_or("");
            if (terminal_objective == "human_handover") != (string_at(action, "action_class") == Some("human_handover")) {
                rejected_edges.push(json!({
                    "action_id":action_id,
                    "terminal_id":terminal_id,
                    "reasons":["human_handover_action_terminal_mismatch"]
                }));
                continue;
            }
            match terminal_status(
                terminal,
                parse_time(string_at(scenario, "decision_time")),
                total_upper,
                deadline_hours
            ) {
                Ok(human_response_upper_hours) => feasible_paths.push(json!({
                    "action_path":path,
                    "terminal_id":terminal_id,
                    "objective_class":terminal_objective,
                    "duration_hours_lower":total_lower,
                    "duration_hours_upper":total_upper,
                    "human_response_upper_hours":human_response_upper_hours,
                    "remaining_intervention_margin_hours":(deadline_hours-total_upper-human_response_upper_hours).max(0.0),
                    "minimum_inter_sample_margin_by_metric":accumulated_margins
                })),
                Err(reason) => rejected_edges.push(json!({
                    "action_id":action_id,
                    "terminal_id":terminal_id,
                    "reasons":[reason]
                })),
            }
        }

        visited_nodes.insert(next_node.to_owned());
        explore_paths(
            scenario, contract, actions, terminals, next_node, next_state,
            total_lower, total_upper, path, visited_nodes, accumulated_margins,
            depth_limit, deadline_hours, max_paths, max_expanded_edges, expanded_edges,
            feasible_paths, rejected_edges, errors, search_truncated
        );
        visited_nodes.remove(next_node);
        *accumulated_margins = previous_margins;
        path.pop();
    }
}

fn scenario_time_or_earlier(scenario: &Value, valid_until: DateTime<FixedOffset>) -> bool {
    parse_time(string_at(scenario, "decision_time")).is_some_and(|decision| decision <= valid_until)
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut scenario_reports = Vec::<Value>::new();
    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value, "claim_ceiling") != Some("bounded_action_recovery_graph_audit_only") {
        errors.push("claim_ceiling must preserve bounded-action/read-only scope".to_owned());
    }
    let status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let contract = value.get("service_contract").unwrap_or(&Value::Null);
    let contract_map = parse_contract(contract, &mut errors);
    if string_at(value.get("mission_snapshot").unwrap_or(&Value::Null), "service_contract_sha256")
        != canonical_digest(contract, None).as_deref()
    {
        errors.push("service_contract_sha256 does not bind the exact service/rate contract".to_owned());
    }

    let trigger_roster = value.get("trigger_roster").and_then(Value::as_array);
    let mut triggers = trigger_roster.map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
    triggers.sort();
    if triggers.is_empty() || triggers.windows(2).any(|pair| pair[0] == pair[1]) {
        errors.push("trigger_roster must be non-empty and unique".to_owned());
    }
    if string_at(value.get("mission_snapshot").unwrap_or(&Value::Null), "trigger_roster_sha256")
        != canonical_digest(&json!(triggers), None).as_deref()
    {
        errors.push("trigger_roster_sha256 does not match canonical trigger roster".to_owned());
    }

    let gates = value.get("hard_gate_roster").and_then(Value::as_array);
    let mut hard_gates = gates.map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
    hard_gates.sort();
    if hard_gates.is_empty() || hard_gates.windows(2).any(|pair| pair[0] == pair[1]) {
        errors.push("hard_gate_roster must be non-empty and unique".to_owned());
    }
    if string_at(value.get("mission_snapshot").unwrap_or(&Value::Null), "hard_gate_roster_sha256")
        != canonical_digest(&json!(hard_gates), None).as_deref()
    {
        errors.push("hard_gate_roster_sha256 does not match canonical hard-gate roster".to_owned());
    }

    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    let capability_items = value.get("capability_roster").and_then(Value::as_array);
    let mut capability_records = capability_items.cloned().unwrap_or_default();
    capability_records.sort_by(|left, right| {
        string_at(left, "capability_id").cmp(&string_at(right, "capability_id"))
    });
    let mut capability_registry = BTreeMap::<String, (String, String, DateTime<FixedOffset>)>::new();
    if capability_records.is_empty() {
        errors.push("capability_roster must be non-empty".to_owned());
    }
    for capability in &capability_records {
        let id = string_at(capability, "capability_id").unwrap_or("");
        let qualification = string_at(capability, "qualification_status").unwrap_or("");
        let scope = string_at(capability, "qualification_scope_sha256").unwrap_or("");
        let evidence = string_at(capability, "qualification_evidence_sha256").unwrap_or("");
        let valid_until = parse_time(string_at(capability, "valid_until"));
        if id.is_empty() || !["qualified", "expired", "suspended"].contains(&qualification)
            || !valid_digest(Some(scope)) || !valid_digest(Some(evidence)) || valid_until.is_none()
        {
            errors.push(format!("capability '{id}' has invalid qualification identity or evidence"));
            continue;
        }
        if capability_registry.insert(id.to_owned(), (qualification.to_owned(), scope.to_owned(), valid_until.unwrap())).is_some() {
            errors.push(format!("capability '{id}' is duplicated"));
        }
    }
    if string_at(snapshot, "capability_roster_sha256")
        != canonical_digest(&json!(capability_records), None).as_deref()
    {
        errors.push("capability_roster_sha256 does not bind the sorted capability qualification roster".to_owned());
    }
    let operating_scope = string_at(snapshot, "operating_envelope_sha256").unwrap_or("");
    if capability_registry.values().any(|(_, scope, _)| scope != operating_scope) {
        errors.push("capability qualification scope differs from the mission operating envelope".to_owned());
    }
    if string_at(snapshot, "mission_snapshot_sha256") != canonical_digest(snapshot, Some("mission_snapshot_sha256")).as_deref() {
        errors.push("mission_snapshot_sha256 does not match canonical snapshot payload".to_owned());
    }
    let mission_start = parse_time(string_at(snapshot, "mission_window_start"));
    let mission_end = parse_time(string_at(snapshot, "mission_window_end"));
    if mission_start.is_none() || mission_end.is_none() || mission_start >= mission_end {
        errors.push("mission window is invalid or unordered".to_owned());
    }
    for field in ["craft_configuration_sha256", "occupancy_sha256", "operating_envelope_sha256", "capability_roster_sha256"] {
        if !valid_digest(string_at(snapshot, field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }

    let limits = value.get("graph_limits").unwrap_or(&Value::Null);
    let max_paths = limits.get("max_paths_per_scenario").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max_depth = limits.get("max_action_depth").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max_action_count = limits.get("max_action_count_per_scenario").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max_expanded_edges = limits.get("max_expanded_edges_per_scenario").and_then(Value::as_u64).unwrap_or(0) as usize;
    if max_paths == 0 || max_paths > 4096 || max_depth == 0 || max_depth > 32
        || max_action_count == 0 || max_action_count > 512
        || max_expanded_edges == 0 || max_expanded_edges > 16384
    {
        errors.push("graph limits must be within 1..4096 paths, 1..32 depth, 1..512 actions and 1..16384 expansions".to_owned());
    }

    let scenarios = value.get("scenarios").and_then(Value::as_array);
    if scenarios.is_none_or(Vec::is_empty) {
        errors.push("scenarios must be a non-empty array".to_owned());
    }
    let mut scenario_ids = BTreeSet::<String>::new();
    let mut scenario_triggers = BTreeSet::<String>::new();
    let mut scenario_hash_rows = Vec::<String>::new();
    if let Some(scenarios) = scenarios {
        for scenario in scenarios {
            let scenario_id = string_at(scenario, "scenario_id").unwrap_or("unknown");
            if !scenario_ids.insert(scenario_id.to_owned()) {
                errors.push(format!("scenario '{scenario_id}' is duplicated"));
            }
            let trigger = string_at(scenario, "trigger_class").unwrap_or("");
            if !triggers.iter().any(|item| item == trigger) {
                errors.push(format!("scenario '{scenario_id}' uses an unregistered trigger class"));
            }
            if !scenario_triggers.insert(trigger.to_owned()) {
                errors.push(format!("trigger class '{trigger}' appears in more than one recovery scenario"));
            }
            let expected_outcome = string_at(scenario, "expected_outcome").unwrap_or("");
            if !["recoverable", "abstain"].contains(&expected_outcome) {
                errors.push(format!("scenario '{scenario_id}' has invalid expected_outcome"));
            }
            if string_at(scenario, "scenario_class") != Some(trigger) {
                errors.push(format!("scenario '{scenario_id}' scenario_class/trigger_class mismatch"));
            }
            let expected_scenario_digest = canonical_digest(scenario, Some("scenario_sha256"));
            if expected_scenario_digest.as_deref() != string_at(scenario, "scenario_sha256") {
                errors.push(format!("scenario '{scenario_id}' scenario_sha256 does not bind the full scenario graph"));
            }
            if valid_digest(string_at(scenario, "scenario_sha256")) {
                scenario_hash_rows.push(format!("{scenario_id}|{}\n", string_at(scenario, "scenario_sha256").unwrap()));
            }
            let state_digest = canonical_digest(scenario.get("trigger_state").unwrap_or(&Value::Null), None);
            if state_digest.as_deref() != string_at(scenario, "trigger_state_sha256") {
                errors.push(format!("scenario '{scenario_id}' trigger-state digest mismatch"));
            }
            let state = parse_state(
                scenario.get("trigger_state").unwrap_or(&Value::Null),
                &contract_map, &mut errors, &format!("scenario '{scenario_id}' trigger state")
            ).unwrap_or_default();
            for (metric, interval) in &state {
                if let Some(spec) = contract_map.get(metric) {
                    if floor_margin(spec, *interval) < -NUMERIC_TOLERANCE {
                        errors.push(format!("scenario '{scenario_id}' trigger state already violates floor '{metric}'"));
                    }
                }
            }

            let scenario_time = parse_time(string_at(scenario, "decision_time"));
            if scenario_time.is_none() || mission_start.zip(mission_end).is_none_or(|(start,end)| scenario_time.unwrap() < start || scenario_time.unwrap() > end) {
                errors.push(format!("scenario '{scenario_id}' decision_time is outside the mission window"));
            }
            let deadline = number_at(scenario, "intervention_deadline_hours").unwrap_or(f64::NAN);
            let depth_limit = scenario.get("max_action_depth").and_then(Value::as_u64).unwrap_or(0) as usize;
            if !deadline.is_finite() || deadline <= 0.0 || depth_limit == 0 || depth_limit > max_depth {
                errors.push(format!("scenario '{scenario_id}' has invalid intervention deadline or action depth"));
            }
            let required_gates = scenario.get("required_hard_gate_ids").and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<BTreeSet<_>>())
                .unwrap_or_default();
            if required_gates != hard_gates.iter().cloned().collect::<BTreeSet<_>>() {
                errors.push(format!("scenario '{scenario_id}' hard-gate roster differs from mission registry"));
            }

            let actions = scenario.get("actions").and_then(Value::as_array).cloned().unwrap_or_default();
            let terminals = scenario.get("terminals").and_then(Value::as_array).cloned().unwrap_or_default();
            if actions.len() > max_action_count {
                errors.push(format!("scenario '{scenario_id}' exceeds max_action_count_per_scenario"));
            }
            let available_caps = scenario.get("available_capabilities").and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>())
                .unwrap_or_default();
            if available_caps.len() != scenario.get("available_capabilities").and_then(Value::as_array).map_or(0, Vec::len) {
                errors.push(format!("scenario '{scenario_id}' has missing/duplicate capability IDs"));
            }
            for capability_id in &available_caps {
                match capability_registry.get(*capability_id) {
                    Some((qualification, scope, valid_until))
                        if qualification == "qualified"
                            && scope == operating_scope
                            && scenario_time_or_earlier(scenario, *valid_until) => {}
                    Some(_) => errors.push(format!("scenario '{scenario_id}' declares capability '{capability_id}' without a current qualification for this operating envelope")),
                    None => errors.push(format!("scenario '{scenario_id}' declares unregistered capability '{capability_id}'")),
                }
            }
            let mut action_ids = BTreeSet::<String>::new();
            for action in &actions {
                let action_id = string_at(action, "action_id").unwrap_or("");
                if action_id.is_empty() || !action_ids.insert(action_id.to_owned()) {
                    errors.push(format!("scenario '{scenario_id}' has a missing/duplicate action ID"));
                }
                if !action_digest_valid(action) {
                    errors.push(format!("scenario '{scenario_id}' action '{action_id}' digest/evidence invalid"));
                }
                if !ACTION_CLASSES.contains(&string_at(action, "action_class").unwrap_or("")) {
                    errors.push(format!("scenario '{scenario_id}' action '{action_id}' has unknown action_class"));
                }
                if let Some(required_caps) = action.get("requires_capabilities").and_then(Value::as_array) {
                    for capability in required_caps.iter().filter_map(Value::as_str) {
                        if !available_caps.contains(capability) {
                            errors.push(format!("scenario '{scenario_id}' action '{action_id}' requires a capability not available in the trigger state: '{capability}'"));
                        }
                    }
                }
                if !valid_digest(string_at(action, "action_evidence_sha256")) {
                    errors.push(format!("scenario '{scenario_id}' action '{action_id}' evidence digest invalid"));
                }
                if action.get("hard_gate_checks").and_then(Value::as_array).is_none_or(Vec::is_empty) {
                    errors.push(format!("scenario '{scenario_id}' action '{action_id}' lacks hard-gate evidence"));
                }
            }
            let mut terminal_ids = BTreeSet::<String>::new();
            for terminal in &terminals {
                let terminal_id = string_at(terminal, "terminal_id").unwrap_or("");
                if terminal_id.is_empty() || !terminal_ids.insert(terminal_id.to_owned()) {
                    errors.push(format!("scenario '{scenario_id}' has a missing/duplicate terminal ID"));
                }
                if string_at(terminal, "receipt_sha256") != canonical_digest(terminal, Some("receipt_sha256")).as_deref() {
                    errors.push(format!("scenario '{scenario_id}' terminal '{terminal_id}' receipt digest invalid"));
                }
            }

            let mut feasible_paths = Vec::<Value>::new();
            let mut rejected_edges = Vec::<Value>::new();
            let mut search_truncated = false;
            let mut expanded_edges = 0_usize;
            let mut path = Vec::<String>::new();
            let mut visited_nodes = BTreeSet::<String>::new();
            visited_nodes.insert("start".to_owned());
            let mut margins = state_floor_margins(&state, &contract_map);
            explore_paths(
                scenario, &contract_map, &actions, &terminals, "start", state,
                0.0, 0.0, &mut path, &mut visited_nodes, &mut margins,
                depth_limit.min(max_depth), deadline, max_paths, max_expanded_edges,
                &mut expanded_edges, &mut feasible_paths, &mut rejected_edges, &mut errors, &mut search_truncated
            );

            let actual_recoverable = !feasible_paths.is_empty();
            let outcome = if actual_recoverable {
                "RECOVERABLE"
            } else if search_truncated {
                "SEARCH_INCOMPLETE/ABSTAIN"
            } else {
                "INFEASIBLE/ABSTAIN"
            };
            let expected = string_at(scenario, "expected_outcome").unwrap_or("");
            if (expected == "recoverable") != actual_recoverable && !search_truncated {
                errors.push(format!("scenario '{scenario_id}' expected_outcome '{expected}' does not match enumerated recovery graph"));
            }
            scenario_reports.push(json!({
                "scenario_id":scenario_id,
                "trigger_class":trigger,
                "outcome":outcome,
                "enumeration_complete":!search_truncated,
                "feasible_path_count":feasible_paths.len(),
                "expanded_edge_count":expanded_edges,
                "feasible_paths":feasible_paths,
                "rejected_edges":rejected_edges,
                "claim_ceiling":"bounded_action_graph_only"
            }));
        }
    }
    let declared_triggers = triggers.iter().cloned().collect::<BTreeSet<_>>();
    if scenario_triggers != declared_triggers {
        errors.push("scenario trigger coverage must match the registered trigger roster exactly once".to_owned());
    }
    scenario_hash_rows.sort();
    let scenario_set_digest = sha256_hex(scenario_hash_rows.join("").as_bytes());
    if string_at(snapshot, "scenario_set_sha256") != Some(scenario_set_digest.as_str()) {
        errors.push("scenario_set_sha256 does not bind the complete sorted scenario set".to_owned());
    }

    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "RECOVERY_REACHABILITY_AUDITED_READ_ONLY"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "input_sha256":sha256_hex(raw),
        "scenario_count":scenario_reports.len(),
        "recoverable_scenario_count":scenario_reports.iter().filter(|report| report["outcome"]=="RECOVERABLE").count(),
        "abstaining_scenario_count":scenario_reports.iter().filter(|report| report["outcome"]!="RECOVERABLE").count(),
        "incomplete_scenario_count":scenario_reports.iter().filter(|report| report["enumeration_complete"]==false).count(),
        "scenario_reports":scenario_reports,
        "findings":{"errors":errors},
        "aggregate_scalar_score":"NOT_COMPUTED",
        "claim_ceiling":"bounded_action_reachability_audit_not_continuous_viability_kernel_not_navigation_or_actuation_authority",
        "note":"All bounded action paths and rejected edges are reported. A terminal is successful only with verified access, required service availability, an unexpired authority/evidence receipt and arrival within the intervention deadline. This is a finite graph audit, not a proof of continuous-system viability."
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 || args[1] != "audit-recovery-graph" {
        return Err("Usage: marine-recovery-reachability audit-recovery-graph <fixture.json>".to_owned());
    }
    let raw = fs::read(&args[2]).map_err(|error| format!("{}: {error}", args[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", args[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "RECOVERY_REACHABILITY_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("recovery graph disposition: {disposition}")),
        None => Err("recovery graph report has no disposition".to_owned()),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-recovery-graph-0001.json"))
            .expect("synthetic recovery graph parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    fn rebind_scenario(value: &mut Value, scenario_index: usize) {
        let action_count = value["scenarios"][scenario_index]["actions"].as_array().unwrap().len();
        for action_index in 0..action_count {
            let digest = canonical_digest(
                &value["scenarios"][scenario_index]["actions"][action_index],
                Some("action_sha256")
            ).expect("action canonicalizes");
            value["scenarios"][scenario_index]["actions"][action_index]["action_sha256"] = json!(digest);
        }
        let terminal_count = value["scenarios"][scenario_index]["terminals"].as_array().unwrap().len();
        for terminal_index in 0..terminal_count {
            let digest = canonical_digest(
                &value["scenarios"][scenario_index]["terminals"][terminal_index],
                Some("receipt_sha256")
            ).expect("terminal canonicalizes");
            value["scenarios"][scenario_index]["terminals"][terminal_index]["receipt_sha256"] = json!(digest);
        }
        let scenario_digest = canonical_digest(
            &value["scenarios"][scenario_index],
            Some("scenario_sha256")
        ).expect("scenario canonicalizes");
        value["scenarios"][scenario_index]["scenario_sha256"] = json!(scenario_digest);
        let mut rows = value["scenarios"].as_array().unwrap().iter()
            .map(|scenario| format!(
                "{}|{}\n",
                string_at(scenario, "scenario_id").unwrap(),
                string_at(scenario, "scenario_sha256").unwrap()
            ))
            .collect::<Vec<_>>();
        rows.sort();
        value["mission_snapshot"]["scenario_set_sha256"] =
            json!(sha256_hex(rows.join("").as_bytes()));
        value["mission_snapshot"]["mission_snapshot_sha256"] = json!(
            canonical_digest(&value["mission_snapshot"], Some("mission_snapshot_sha256"))
                .expect("mission snapshot canonicalizes")
        );
    }

    #[test]
    fn all_synthetic_expected_recovery_outcomes_are_reproduced() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["scenario_count"], 5);
        assert_eq!(report["recoverable_scenario_count"], 2);
        assert_eq!(report["abstaining_scenario_count"], 3);
        assert_eq!(report["aggregate_scalar_score"], "NOT_COMPUTED");
    }

    #[test]
    fn generator_trip_retains_safe_haven_path_and_rejected_alternatives() {
        let report = audit(&fixture());
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap();
        assert_eq!(scenario["outcome"], "RECOVERABLE");
        assert_eq!(scenario["feasible_path_count"], 1);
        let ids = scenario["feasible_paths"][0]["action_path"].as_array().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["action_id"] == "fast-divert"));
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["action_id"] == "repair-generator"));
    }

    #[test]
    fn watermaker_failure_requires_verified_shore_resupply_terminal() {
        let report = audit(&fixture());
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "watermaker-trip").unwrap();
        assert_eq!(scenario["outcome"], "RECOVERABLE");
        assert_eq!(scenario["feasible_paths"][0]["objective_class"], "shore_resupply");
    }

    #[test]
    fn common_cause_outage_cannot_be_recovered_by_floor_breaching_route() {
        let report = audit(&fixture());
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "common-cause-outage").unwrap();
        assert_eq!(scenario["outcome"], "INFEASIBLE/ABSTAIN");
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["action_id"] == "divert-common-cause"));
    }

    #[test]
    fn a_route_after_the_intervention_deadline_is_not_a_recovery() {
        let report = audit(&fixture());
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "deadline-exhausted").unwrap();
        assert_eq!(scenario["feasible_path_count"], 0);
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["reasons"][0] == "intervention_deadline_exceeded"));
    }

    #[test]
    fn unverified_destination_service_is_not_a_successful_terminal() {
        let report = audit(&fixture());
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "unverified-port-service").unwrap();
        assert_eq!(scenario["outcome"], "INFEASIBLE/ABSTAIN");
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["reasons"][0].as_str().unwrap().contains("terminal_service_not_verified")));
    }

    #[test]
    fn changing_an_action_without_rehashing_invalidates_graph() {
        let mut value = fixture();
        value["scenarios"][0]["actions"][0]["metric_deltas"][0]["lower_delta"] = json!(-10);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn changing_a_terminal_receipt_without_rehashing_invalidates_graph() {
        let mut value = fixture();
        value["scenarios"][0]["terminals"][0]["available"] = json!(false);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn required_hard_gate_roster_must_match_every_action() {
        let mut value = fixture();
        value["scenarios"][0]["actions"][0]["hard_gate_checks"].as_array_mut().unwrap().pop();
        value["scenarios"][0]["expected_outcome"] = json!("abstain");
        rebind_scenario(&mut value, 0);
        let report = audit(&value);
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap();
        assert_eq!(scenario["outcome"], "INFEASIBLE/ABSTAIN");
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["reasons"].as_array().unwrap().iter()
                .any(|reason| reason == "hard_gate_roster_incomplete_or_extra")));
    }

    #[test]
    fn action_requiring_failed_component_is_not_traversed() {
        let mut value = fixture();
        value["scenarios"][0]["actions"][1]["requires_components"] = json!(["main_generator"]);
        value["scenarios"][0]["expected_outcome"] = json!("abstain");
        rebind_scenario(&mut value, 0);
        let report = audit(&value);
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap();
        assert_eq!(scenario["outcome"], "INFEASIBLE/ABSTAIN");
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["reasons"].as_array().unwrap().iter()
                .any(|reason| reason == "required_component_unavailable:main_generator")));
    }

    #[test]
    fn action_rate_bound_violation_cannot_be_hidden_by_safe_terminal() {
        let mut value = fixture();
        value["scenarios"][0]["actions"][0]["metric_deltas"][0]["lower_delta"] = json!(-20);
        value["scenarios"][0]["actions"][0]["metric_deltas"][0]["upper_delta"] = json!(-20);
        value["scenarios"][0]["expected_outcome"] = json!("abstain");
        rebind_scenario(&mut value, 0);
        let report = audit(&value);
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap();
        assert_eq!(scenario["outcome"], "INFEASIBLE/ABSTAIN");
        assert!(scenario["rejected_edges"].as_array().unwrap().iter()
            .any(|edge| edge["action_id"] == "shed-load" && edge["reasons"].as_array().unwrap().iter()
                .any(|reason| reason.as_str().unwrap().starts_with("endpoint_change_exceeds_declared_rate_bound"))));
    }

    #[test]
    fn path_enumeration_cap_is_reported_as_incomplete_not_exhaustive() {
        let mut value = fixture();
        value["graph_limits"]["max_paths_per_scenario"] = json!(1);
        let report = audit(&value);
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap();
        assert_eq!(scenario["outcome"], "RECOVERABLE");
        assert_eq!(scenario["enumeration_complete"], false);
        assert_eq!(report["incomplete_scenario_count"], 1);
    }

    #[test]
    fn malformed_decision_timestamp_is_invalid_without_panicking() {
        let mut value = fixture();
        value["scenarios"][0]["decision_time"] = json!("not-a-time");
        let report = audit(&value);
        assert_eq!(report["disposition"], "INVALID");
        assert_ne!(report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_id"] == "generator-trip").unwrap()["outcome"], "RECOVERABLE");
    }

    #[test]
    fn trigger_roster_tampering_invalidates_the_mission_snapshot() {
        let mut value = fixture();
        value["trigger_roster"][0] = json!("unregistered-fault");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
