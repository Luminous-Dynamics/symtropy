use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-recovery-reachability/0.1.0";
const NUMERIC_TOLERANCE: f64 = 1e-9;
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
const SCENARIO_CLASSES: [&str; 7] = [
    "common_cause_power_water",
    "competing_paths",
    "dependency_unavailable",
    "generator_trip",
    "resource_depletion",
    "service_unverified",
    "watermaker_failure",
];

#[derive(Clone)]
struct MetricContract {
    unit: String,
    operator: String,
    floor: f64,
    minimum_coverage: f64,
}

#[derive(Clone)]
struct MetricInterval {
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

fn digest_valid(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical_digest(value: &Value, excluded_field: Option<&str>) -> Option<String> {
    let mut canonical = value.clone();
    if let Some(field) = excluded_field {
        canonical.as_object_mut()?.remove(field)?;
    }
    Some(sha256_hex(&serde_json::to_vec(&canonical).ok()?))
}

fn array_digest(value: &Value) -> Option<String> {
    Some(sha256_hex(&serde_json::to_vec(value).ok()?))
}

fn digest_field_matches(value: &Value, field: &str, excluded: Option<&str>, errors: &mut Vec<String>, label: &str) {
    match canonical_digest(value, excluded) {
        Some(calculated) if string_at(value, field) == Some(calculated.as_str()) => {}
        Some(_) => errors.push(format!("{label} {field} does not match its canonical payload")),
        None => errors.push(format!("{label} cannot be canonically serialized")),
    }
}

fn validate_contract(value: &Value, errors: &mut Vec<String>) -> BTreeMap<String, MetricContract> {
    let mut contract = BTreeMap::<String, MetricContract>::new();
    let Some(items) = value.as_array() else {
        errors.push("service_contract must be an array".to_owned());
        return contract;
    };
    if items.len() != METRICS.len() {
        errors.push("service_contract must define exactly eleven axes".to_owned());
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let floor = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        let Some((_, expected_unit, expected_operator)) = METRICS.iter().find(|spec| spec.0 == metric) else {
            errors.push(format!("service contract contains unknown metric '{metric}'"));
            continue;
        };
        let (Some(floor), Some(coverage)) = (floor, coverage) else {
            errors.push(format!("service contract '{metric}' has a missing/non-finite threshold"));
            continue;
        };
        if unit != *expected_unit || operator != *expected_operator || !(0.0..=1.0).contains(&coverage) {
            errors.push(format!("service contract '{metric}' has invalid unit, inequality or coverage"));
        }
        if contract.insert(metric.to_owned(), MetricContract {
            unit: unit.to_owned(), operator: operator.to_owned(), floor, minimum_coverage: coverage,
        }).is_some() {
            errors.push(format!("service contract duplicates metric '{metric}'"));
        }
    }
    for (metric, _, _) in METRICS {
        if !contract.contains_key(metric) {
            errors.push(format!("service contract is missing '{metric}'"));
        }
    }
    contract
}

fn parse_metrics(
    items: &Value,
    contract: &BTreeMap<String, MetricContract>,
    record_status: &str,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeMap<String, MetricInterval> {
    let mut parsed = BTreeMap::<String, MetricInterval>::new();
    let Some(items) = items.as_array() else {
        errors.push(format!("{label} resource_metrics must be an array"));
        return parsed;
    };
    if items.len() != METRICS.len() {
        errors.push(format!("{label} must contain exactly eleven resource metrics"));
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let value = number_at(item, "value");
        let lower = number_at(item, "lower_bound");
        let upper = number_at(item, "upper_bound");
        let coverage = number_at(item, "coverage_fraction");
        let class = string_at(item, "measurement_class").unwrap_or("");
        let method = string_at(item, "measurement_method").unwrap_or("");
        let Some(contract_metric) = contract.get(metric) else {
            errors.push(format!("{label} contains unknown metric '{metric}'"));
            continue;
        };
        let (Some(value), Some(lower), Some(upper), Some(coverage)) = (value, lower, upper, coverage) else {
            errors.push(format!("{label} metric '{metric}' has missing/non-finite measurements"));
            continue;
        };
        if unit != contract_metric.unit
            || lower > value
            || value > upper
            || lower < 0.0
            || !(0.0..=1.0).contains(&coverage)
            || !digest_valid(string_at(item, "evidence_sha256"))
            || method.is_empty()
            || !["measured", "modeled", "derived", "synthetic"].contains(&class)
            || (record_status == "synthetic" && class != "synthetic")
            || (record_status != "synthetic" && class == "synthetic")
        {
            errors.push(format!("{label} metric '{metric}' has invalid unit, bounds, coverage or evidence class"));
            continue;
        }
        if parsed.insert(metric.to_owned(), MetricInterval {
            lower, upper, coverage,
        }).is_some() {
            errors.push(format!("{label} duplicates metric '{metric}'"));
        }
    }
    for (metric, _, _) in METRICS {
        if !parsed.contains_key(metric) {
            errors.push(format!("{label} is missing metric '{metric}'"));
        }
    }
    parsed
}

fn conservative_margins(
    metrics: &BTreeMap<String, MetricInterval>,
    contract: &BTreeMap<String, MetricContract>,
    label: &str,
) -> (Vec<String>, BTreeMap<String, f64>) {
    let mut blockers = Vec::<String>::new();
    let mut margins = BTreeMap::<String, f64>::new();
    for (metric, interval) in metrics {
        if let Some(floor) = contract.get(metric) {
            let margin = match floor.operator.as_str() {
                "gte" => interval.lower - floor.floor,
                "lte" => floor.floor - interval.upper,
                _ => f64::NEG_INFINITY,
            };
            margins.insert(metric.clone(), margin);
            if interval.coverage + NUMERIC_TOLERANCE < floor.minimum_coverage {
                blockers.push(format!("{label} metric '{metric}' coverage {:.3} is below {:.3}", interval.coverage, floor.minimum_coverage));
            }
            if margin < -NUMERIC_TOLERANCE {
                blockers.push(format!("{label} metric '{metric}' conservative interval breaches {} {} {}", floor.operator, floor.floor, floor.unit));
            }
        }
    }
    (blockers, margins)
}

fn string_set(value: &Value) -> BTreeSet<String> {
    value.as_array().map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default()
}

fn contains_all(have: &BTreeSet<String>, required: &BTreeSet<String>) -> bool {
    required.is_subset(have)
}

fn validate_state_digests(scenario: &Value, errors: &mut Vec<String>) {
    if let Some(states) = scenario.get("states").and_then(Value::as_array) {
        let mut state_ids = BTreeSet::<String>::new();
        for state in states {
            let id = string_at(state, "state_id").unwrap_or("");
            if id.is_empty() || !state_ids.insert(id.to_owned()) {
                errors.push(format!("scenario state has missing/duplicate state_id '{id}'"));
            }
            digest_field_matches(state, "state_sha256", Some("state_sha256"), errors, &format!("state '{id}'"));
            if !digest_valid(string_at(state, "hard_gate_receipt_sha256"))
                || string_at(state, "hard_gate_receipt_ref").is_none_or(str::is_empty)
            {
                errors.push(format!("state '{id}' has no hard-gate evidence receipt"));
            }
        }
    }
    if let Some(actions) = scenario.get("actions").and_then(Value::as_array) {
        let mut action_ids = BTreeSet::<String>::new();
        for action in actions {
            let id = string_at(action, "action_id").unwrap_or("");
            if id.is_empty() || !action_ids.insert(id.to_owned()) {
                errors.push(format!("scenario action has missing/duplicate action_id '{id}'"));
            }
            digest_field_matches(action, "action_sha256", Some("action_sha256"), errors, &format!("action '{id}'"));
            if let Some(checkpoints) = action.get("checkpoints").and_then(Value::as_array) {
                let mut checkpoint_ids = BTreeSet::<String>::new();
                for checkpoint in checkpoints {
                    let checkpoint_id = string_at(checkpoint, "checkpoint_id").unwrap_or("");
                    if checkpoint_id.is_empty() || !checkpoint_ids.insert(checkpoint_id.to_owned()) {
                        errors.push(format!("action '{id}' has missing/duplicate checkpoint_id '{checkpoint_id}'"));
                    }
                    digest_field_matches(checkpoint, "checkpoint_sha256", Some("checkpoint_sha256"), errors, &format!("checkpoint '{checkpoint_id}'"));
                }
            }
        }
    }
    digest_field_matches(scenario, "scenario_sha256", Some("scenario_sha256"), errors, &format!("scenario '{}'", string_at(scenario, "scenario_id").unwrap_or("unknown")));
}

fn validate_action(
    action: &Value,
    scenario_class: &str,
    from: &Value,
    to: &Value,
    contract: &BTreeMap<String, MetricContract>,
    hard_gate_roster: &BTreeSet<String>,
    record_status: &str,
) -> Vec<String> {
    let id = string_at(action, "action_id").unwrap_or("unknown");
    let mut blockers = Vec::<String>::new();
    if !string_set(action.get("allowed_scenario_classes").unwrap_or(&Value::Null)).contains(scenario_class) {
        blockers.push(format!("action '{id}' is not admitted for scenario class '{scenario_class}'"));
    }
    let capabilities = string_set(from.get("capabilities").unwrap_or(&Value::Null));
    let required_capabilities = string_set(action.get("required_capabilities").unwrap_or(&Value::Null));
    for missing in required_capabilities.difference(&capabilities) {
        blockers.push(format!("action '{id}' requires unavailable capability '{missing}'"));
    }
    let available_components = from.get("components").and_then(Value::as_object)
        .map(|items| items.iter().filter(|(_, value)| value.as_str() == Some("available")).map(|(key, _)| key.clone()).collect::<BTreeSet<_>>())
        .unwrap_or_default();
    let required_components = string_set(action.get("required_components_available").unwrap_or(&Value::Null));
    for missing in required_components.difference(&available_components) {
        blockers.push(format!("action '{id}' requires unavailable component '{missing}'"));
    }
    let duration_min = number_at(action, "duration_min_hours");
    let duration_max = number_at(action, "duration_max_hours");
    match (duration_min, duration_max) {
        (Some(minimum), Some(maximum)) if minimum >= 0.0 && maximum > 0.0 && minimum <= maximum => {}
        _ => blockers.push(format!("action '{id}' has an invalid duration interval")),
    }
    for digest_field in ["action_sha256", "model_sha256", "evidence_sha256", "segment_envelope_receipt_sha256"] {
        if !digest_valid(string_at(action, digest_field)) {
            blockers.push(format!("action '{id}' has invalid {digest_field}"));
        }
    }
    for reference_field in ["evidence_ref", "segment_envelope_receipt_ref"] {
        if string_at(action, reference_field).is_none_or(str::is_empty) {
            blockers.push(format!("action '{id}' is missing {reference_field}"));
        }
    }

    let checks = action.get("hard_gate_checks").and_then(Value::as_array);
    let mut observed_gates = BTreeSet::<String>::new();
    if let Some(checks) = checks {
        for check in checks {
            let gate = string_at(check, "hard_gate_id").unwrap_or("");
            if gate.is_empty() || !observed_gates.insert(gate.to_owned()) {
                blockers.push(format!("action '{id}' has a missing/duplicate hard-gate check"));
            }
            if !digest_valid(string_at(check, "evidence_sha256")) || string_at(check, "evidence_ref").is_none_or(str::is_empty) {
                blockers.push(format!("action '{id}' hard-gate '{gate}' lacks evidence"));
            }
            if string_at(check, "status") != Some("pass") {
                blockers.push(format!("action '{id}' hard-gate '{gate}' is not PASS"));
            }
        }
    } else {
        blockers.push(format!("action '{id}' has no hard-gate checks"));
    }
    if &observed_gates != hard_gate_roster {
        blockers.push(format!("action '{id}' hard-gate check set does not match the frozen roster"));
    }

    let from_metrics = parse_metrics(
        from.get("resource_metrics").unwrap_or(&Value::Null), contract, record_status,
        &format!("action '{id}' start state"), &mut blockers,
    );
    let to_metrics = parse_metrics(
        to.get("resource_metrics").unwrap_or(&Value::Null), contract, record_status,
        &format!("action '{id}' successor state"), &mut blockers,
    );
    let deltas = action.get("resource_deltas").and_then(Value::as_array);
    let mut seen_deltas = BTreeSet::<String>::new();
    if deltas.is_none_or(|items| items.len() != METRICS.len()) {
        blockers.push(format!("action '{id}' must carry one resource delta for every service axis"));
    }
    if let Some(deltas) = deltas {
        for delta in deltas {
            let metric = string_at(delta, "metric").unwrap_or("");
            let unit = string_at(delta, "unit").unwrap_or("");
            let lower_delta = number_at(delta, "lower_delta");
            let upper_delta = number_at(delta, "upper_delta");
            if !seen_deltas.insert(metric.to_owned()) {
                blockers.push(format!("action '{id}' duplicates resource delta '{metric}'"));
                continue;
            }
            let (Some(lower_delta), Some(upper_delta)) = (lower_delta, upper_delta) else {
                blockers.push(format!("action '{id}' resource delta '{metric}' is missing/non-finite"));
                continue;
            };
            if lower_delta > upper_delta || contract.get(metric).is_none_or(|spec| spec.unit != unit) {
                blockers.push(format!("action '{id}' resource delta '{metric}' has invalid interval/unit"));
                continue;
            }
            if let (Some(start), Some(end)) = (from_metrics.get(metric), to_metrics.get(metric)) {
                let reachable_lower = start.lower + lower_delta;
                let reachable_upper = start.upper + upper_delta;
                if end.lower > reachable_lower + NUMERIC_TOLERANCE || end.upper < reachable_upper - NUMERIC_TOLERANCE {
                    blockers.push(format!("action '{id}' successor '{metric}' interval narrows the declared reachable interval"));
                }
            }
        }
    }
    for (metric, _, _) in METRICS {
        if !seen_deltas.contains(metric) {
            blockers.push(format!("action '{id}' is missing resource delta '{metric}'"));
        }
    }

    let from_components = from.get("components").and_then(Value::as_object);
    let to_components = to.get("components").and_then(Value::as_object);
    let mut actual_component_changes = BTreeSet::<String>::new();
    if let (Some(before), Some(after)) = (from_components, to_components) {
        let all_ids = before.keys().chain(after.keys()).cloned().collect::<BTreeSet<_>>();
        for component_id in all_ids {
            if before.get(&component_id) != after.get(&component_id) {
                actual_component_changes.insert(component_id);
            }
        }
    } else {
        blockers.push(format!("action '{id}' source/successor component registry is missing"));
    }

    let changes = action.get("component_changes").and_then(Value::as_array);
    let mut seen_changes = BTreeSet::<String>::new();
    if let Some(changes) = changes {
        for change in changes {
            let component_id = string_at(change, "component_id").unwrap_or("");
            if component_id.is_empty() || !seen_changes.insert(component_id.to_owned()) {
                blockers.push(format!("action '{id}' has missing/duplicate component change"));
                continue;
            }
            let before = from.get("components").and_then(|items| items.get(component_id)).and_then(Value::as_str);
            let after = to.get("components").and_then(|items| items.get(component_id)).and_then(Value::as_str);
            if before != string_at(change, "from_availability") || after != string_at(change, "to_availability") {
                blockers.push(format!("action '{id}' component change '{component_id}' does not match state transition"));
            }
            if !digest_valid(string_at(change, "evidence_sha256")) || string_at(change, "evidence_ref").is_none_or(str::is_empty) {
                blockers.push(format!("action '{id}' component change '{component_id}' lacks evidence"));
            }
        }
    }

    if actual_component_changes != seen_changes {
        blockers.push(format!("action '{id}' component_changes do not exactly cover every source/successor availability change"));
    }

    let checkpoints = action.get("checkpoints").and_then(Value::as_array);
    if checkpoints.is_none_or(Vec::is_empty) {
        blockers.push(format!("action '{id}' has no intermediate checkpoints"));
    } else if let Some(checkpoints) = checkpoints {
        let mut last_elapsed = 0.0;
        for checkpoint in checkpoints {
            let checkpoint_id = string_at(checkpoint, "checkpoint_id").unwrap_or("unknown");
            let elapsed = number_at(checkpoint, "elapsed_upper_hours");
            match elapsed {
                Some(value) if value > last_elapsed && duration_max.is_some_and(|maximum| value <= maximum + NUMERIC_TOLERANCE) => {
                    last_elapsed = value;
                }
                _ => blockers.push(format!("action '{id}' checkpoint '{checkpoint_id}' has non-monotonic/out-of-action elapsed time")),
            }
            if !digest_valid(string_at(checkpoint, "checkpoint_sha256"))
                || !digest_valid(string_at(checkpoint, "evidence_sha256"))
                || string_at(checkpoint, "evidence_ref").is_none_or(str::is_empty)
            {
                blockers.push(format!("action '{id}' checkpoint '{checkpoint_id}' lacks digest/evidence"));
            }
            let checkpoint_metrics = parse_metrics(
                checkpoint.get("resource_metrics").unwrap_or(&Value::Null), contract, record_status,
                &format!("action '{id}' checkpoint '{checkpoint_id}'"), &mut blockers,
            );
            let (checkpoint_blockers, _) = conservative_margins(
                &checkpoint_metrics, contract, &format!("action '{id}' checkpoint '{checkpoint_id}'"),
            );
            blockers.extend(checkpoint_blockers);
        }
    }

    let (state_blockers, _) = conservative_margins(&to_metrics, contract, &format!("action '{id}' successor"));
    blockers.extend(state_blockers);
    blockers
}

#[derive(Clone)]
struct FeasiblePath {
    terminal_id: String,
    terminal_class: String,
    action_ids: Vec<String>,
    total_duration_upper_hours: f64,
    minimum_margin_by_metric: BTreeMap<String, (f64, String)>,
}

fn update_minimum_margins(
    margins: &mut BTreeMap<String, (f64, String)>,
    metrics: &BTreeMap<String, MetricInterval>,
    contract: &BTreeMap<String, MetricContract>,
    when: &str,
) {
    let (_, current) = conservative_margins(metrics, contract, when);
    for (metric, margin) in current {
        match margins.get(&metric) {
            Some((existing, _)) if *existing <= margin => {}
            _ => { margins.insert(metric, (margin, when.to_owned())); }
        }
    }
}

struct SearchContext<'a> {
    scenario_id: &'a str,
    scenario_class: &'a str,
    contract: &'a BTreeMap<String, MetricContract>,
    hard_gate_roster: &'a BTreeSet<String>,
    record_status: &'a str,
    max_depth: usize,
    max_paths: usize,
    state_by_id: BTreeMap<String, &'a Value>,
    actions_by_from: BTreeMap<String, Vec<&'a Value>>,
    terminals_by_state: BTreeMap<String, Vec<&'a Value>>,
    feasible_paths: Vec<FeasiblePath>,
    rejected_actions: Vec<Value>,
    rejected_terminals: Vec<Value>,
    dead_ends: Vec<Value>,
    explored_edges: usize,
    fatal: Vec<String>,
}

impl<'a> SearchContext<'a> {
    fn terminal_blockers(terminal: &Value) -> Vec<String> {
        let id = string_at(terminal, "terminal_id").unwrap_or("unknown");
        let class = string_at(terminal, "terminal_class").unwrap_or("");
        let expected_kind = match class {
            "safe_haven" => Some("refuge"),
            "shore_resupply" => Some("shore_service"),
            "repair_restoration" => Some("repair_restoration"),
            "controlled_safe_hold" => Some("safe_hold"),
            _ => None,
        };
        let mut blockers = Vec::<String>::new();
        if expected_kind != Some(string_at(terminal, "qualification_kind").unwrap_or("")) {
            blockers.push(format!("terminal '{id}' has a qualification kind inconsistent with terminal class"));
        }
        for field in [
            "availability_receipt_sha256", "access_receipt_sha256", "service_receipt_sha256",
            "qualification_receipt_sha256", "authority_receipt_sha256",
        ] {
            if !digest_valid(string_at(terminal, field)) {
                blockers.push(format!("terminal '{id}' has invalid {field}"));
            }
        }
        for field in [
            "availability_receipt_ref", "access_receipt_ref", "service_receipt_ref",
            "qualification_receipt_ref", "authority_receipt_ref",
        ] {
            if string_at(terminal, field).is_none_or(str::is_empty) {
                blockers.push(format!("terminal '{id}' has missing {field}"));
            }
        }
        if string_at(terminal, "availability_status") != Some("verified")
            || string_at(terminal, "access_status") != Some("verified")
            || string_at(terminal, "service_status") != Some("verified")
        {
            blockers.push(format!("terminal '{id}' access, availability or service is not verified"));
        }
        blockers
    }

    #[allow(clippy::too_many_arguments)]
    fn walk(
        &mut self,
        current_state_id: &str,
        depth: usize,
        elapsed_upper: f64,
        action_ids: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
        margins: &mut BTreeMap<String, (f64, String)>,
    ) {
        if self.explored_edges > self.max_paths {
            self.fatal.push(format!("scenario '{}' exceeded max_paths_per_scenario", self.scenario_id));
            return;
        }
        let current_state = self.state_by_id.get(current_state_id).copied().unwrap_or(&Value::Null);
        let mut current_state_blockers = Vec::<String>::new();
        let current_metrics = parse_metrics(
            current_state.get("resource_metrics").unwrap_or(&Value::Null),
            self.contract,
            self.record_status,
            &format!("current state '{current_state_id}'"),
            &mut current_state_blockers,
        );
        let (floor_blockers, _) = conservative_margins(
            &current_metrics,
            self.contract,
            &format!("current state '{current_state_id}'"),
        );
        current_state_blockers.extend(floor_blockers);
        if string_at(current_state, "hard_gate_disposition") != Some("pass") {
            current_state_blockers.push(format!("current state '{current_state_id}' hard-gate disposition is not PASS"));
        }

        if let Some(terminals) = self.terminals_by_state.get(current_state_id).cloned() {
            for terminal in terminals {
                let mut blockers = Self::terminal_blockers(terminal);
                blockers.extend(current_state_blockers.clone());
                let terminal_id = string_at(terminal, "terminal_id").unwrap_or("unknown").to_owned();
                if blockers.is_empty() {
                    let terminal_class = string_at(terminal, "terminal_class").unwrap_or("").to_owned();
                    self.feasible_paths.push(FeasiblePath {
                        terminal_id, terminal_class, action_ids: action_ids.clone(),
                        total_duration_upper_hours: elapsed_upper,
                        minimum_margin_by_metric: margins.clone(),
                    });
                } else {
                    self.rejected_terminals.push(json!({
                        "terminal_id":terminal_id,
                        "action_ids":action_ids,
                        "blockers":blockers
                    }));
                }
            }
        }
        if depth >= self.max_depth {
            self.dead_ends.push(json!({"state_id":current_state_id,"action_ids":action_ids,"reason":"max_action_depth"}));
            return;
        }
        let outgoing = self.actions_by_from.get(current_state_id).cloned().unwrap_or_default();
        if outgoing.is_empty() {
            if !self.terminals_by_state.contains_key(current_state_id) {
                self.dead_ends.push(json!({"state_id":current_state_id,"action_ids":action_ids,"reason":"no_declared_outgoing_recovery_action"}));
            }
            return;
        }
        for action in outgoing {
            self.explored_edges += 1;
            let action_id = string_at(action, "action_id").unwrap_or("unknown").to_owned();
            let to_id = string_at(action, "to_state_id").unwrap_or("").to_owned();
            let from = self.state_by_id.get(current_state_id).copied().unwrap_or(&Value::Null);
            let to = self.state_by_id.get(&to_id).copied().unwrap_or(&Value::Null);
            let mut blockers = current_state_blockers.clone();
            blockers.extend(validate_action(
                action, self.scenario_class, from, to, self.contract,
                self.hard_gate_roster, self.record_status,
            ));
            if visited.contains(&to_id) {
                blockers.push(format!("action '{action_id}' creates a cycle/repeated state '{to_id}'"));
            }
            if !blockers.is_empty() {
                let mut tried = action_ids.clone();
                tried.push(action_id.clone());
                self.rejected_actions.push(json!({
                    "at_state_id":current_state_id,
                    "action_id":action_id,
                    "tried_action_ids":tried,
                    "blockers":blockers
                }));
                continue;
            }
            let duration = number_at(action, "duration_max_hours").unwrap_or(0.0);
            let new_elapsed = elapsed_upper + duration;
            let mut new_margins = margins.clone();
            let to_metrics = parse_metrics(
                to.get("resource_metrics").unwrap_or(&Value::Null), self.contract,
                self.record_status, &format!("successor state '{to_id}'"), &mut Vec::new(),
            );
            update_minimum_margins(&mut new_margins,&to_metrics,self.contract,&format!("state:{to_id}"));
            if let Some(checkpoints) = action.get("checkpoints").and_then(Value::as_array) {
                for checkpoint in checkpoints {
                    let metrics = parse_metrics(
                        checkpoint.get("resource_metrics").unwrap_or(&Value::Null),self.contract,
                        self.record_status,"path checkpoint",&mut Vec::new(),
                    );
                    let cp_id = string_at(checkpoint,"checkpoint_id").unwrap_or("unknown");
                    update_minimum_margins(&mut new_margins,&metrics,self.contract,&format!("checkpoint:{cp_id}"));
                }
            }
            visited.insert(to_id.clone());
            action_ids.push(action_id);
            self.walk(&to_id,depth+1,new_elapsed,action_ids,visited,&mut new_margins);
            action_ids.pop();
            visited.remove(&to_id);
            if !self.fatal.is_empty() {
                return;
            }
        }
    }
}

fn validate_scenario(
    scenario: &Value,
    scenario_class: &str,
    contract: &BTreeMap<String,MetricContract>,
    hard_gate_roster: &BTreeSet<String>,
    record_status: &str,
    snapshot: &Value,
) -> Value {
    let scenario_id = string_at(scenario,"scenario_id").unwrap_or("unknown");
    let mut errors = Vec::<String>::new();
    let states = scenario.get("states").and_then(Value::as_array);
    let actions = scenario.get("actions").and_then(Value::as_array);
    let terminals = scenario.get("terminals").and_then(Value::as_array);
    if states.is_none() || actions.is_none() || terminals.is_none() {
        return json!({"scenario_id":scenario_id,"disposition":"INVALID","findings":["states/actions/terminals arrays are required"]});
    }
    let states = states.unwrap();
    let actions = actions.unwrap();
    let terminals = terminals.unwrap();
    let mut state_by_id = BTreeMap::<String,&Value>::new();
    for state in states {
        let id = string_at(state,"state_id").unwrap_or("").to_owned();
        if state_by_id.insert(id.clone(),state).is_some() {
            errors.push(format!("scenario '{scenario_id}' duplicates state '{id}'"));
        }
        digest_field_matches(state,"state_sha256",Some("state_sha256"),&mut errors,&format!("state '{id}'"));
        let metrics = parse_metrics(
            state.get("resource_metrics").unwrap_or(&Value::Null),contract,record_status,
            &format!("state '{id}'"),&mut errors,
        );
        if id == string_at(scenario,"trigger_state_id").unwrap_or("") {
            let _ = metrics;
        }
        if string_at(state,"hard_gate_disposition").is_none() {
            errors.push(format!("state '{id}' is missing hard-gate disposition"));
        }
    }
    if !state_by_id.contains_key(string_at(scenario,"trigger_state_id").unwrap_or("")) {
        errors.push(format!("scenario '{scenario_id}' trigger_state_id is missing"));
    }
    if let Some(actions) = scenario.get("actions").and_then(Value::as_array) {
        let mut action_ids = BTreeSet::<String>::new();
        for action in actions {
            let id = string_at(action,"action_id").unwrap_or("").to_owned();
            if !action_ids.insert(id.clone()) {
                errors.push(format!("scenario '{scenario_id}' duplicates action '{id}'"));
            }
            digest_field_matches(action,"action_sha256",Some("action_sha256"),&mut errors,&format!("action '{id}'"));
            let from = string_at(action,"from_state_id").unwrap_or("");
            let to = string_at(action,"to_state_id").unwrap_or("");
            if !state_by_id.contains_key(from) || !state_by_id.contains_key(to) {
                errors.push(format!("action '{id}' references an undeclared from/to state"));
            }
            let _ = (from,to);
        }
    }
    let mut terminal_ids = BTreeSet::<String>::new();
    for terminal in terminals {
        let id = string_at(terminal,"terminal_id").unwrap_or("").to_owned();
        if !terminal_ids.insert(id.clone()) {
            errors.push(format!("scenario '{scenario_id}' duplicates terminal '{id}'"));
        }
        let state_id = string_at(terminal,"state_id").unwrap_or("");
        if !state_by_id.contains_key(state_id) {
            errors.push(format!("terminal '{id}' references undeclared state '{state_id}'"));
        }
        if string_at(terminal,"operating_envelope_sha256") != string_at(snapshot,"operating_envelope_sha256") {
            errors.push(format!("terminal '{id}' operating-envelope digest differs from mission snapshot"));
        }
    }

    let mut feasible_paths = Vec::<FeasiblePath>::new();
    let mut rejected_actions = Vec::<Value>::new();
    let mut rejected_terminals = Vec::<Value>::new();
    let mut dead_ends = Vec::<Value>::new();
    let mut fatal = Vec::<String>::new();
    let mut actions_by_from = BTreeMap::<String,Vec<&Value>>::new();
    for action in actions {
        actions_by_from.entry(string_at(action,"from_state_id").unwrap_or("").to_owned()).or_default().push(action);
    }
    let mut terminals_by_state = BTreeMap::<String,Vec<&Value>>::new();
    for terminal in terminals {
        terminals_by_state.entry(string_at(terminal,"state_id").unwrap_or("").to_owned()).or_default().push(terminal);
    }

    let trigger_id = string_at(scenario,"trigger_state_id").unwrap_or("");
    if let Some(trigger) = state_by_id.get(trigger_id) {
        let mut margins = BTreeMap::<String,(f64,String)>::new();
        let trigger_metrics = parse_metrics(
            trigger.get("resource_metrics").unwrap_or(&Value::Null),contract,record_status,
            &format!("trigger state '{trigger_id}'"),&mut errors,
        );
        update_minimum_margins(&mut margins,&trigger_metrics,contract,&format!("state:{trigger_id}"));
        let mut ctx = SearchContext {
            scenario_id,scenario_class,contract,hard_gate_roster,record_status,
            max_depth:snapshot.get("max_action_depth").and_then(Value::as_u64).unwrap_or(1) as usize,
            max_paths:snapshot.get("max_paths_per_scenario").and_then(Value::as_u64).unwrap_or(1) as usize,
            state_by_id,actions_by_from,terminals_by_state,
            feasible_paths:Vec::new(),rejected_actions:Vec::new(),rejected_terminals:Vec::new(),
            dead_ends:Vec::new(),explored_edges:0,fatal:Vec::new(),
        };
        let mut visited=BTreeSet::<String>::new();
        visited.insert(trigger_id.to_owned());
        ctx.walk(trigger_id,0,0.0,&mut Vec::new(),&mut visited,&mut margins);
        feasible_paths=ctx.feasible_paths;
        rejected_actions=ctx.rejected_actions;
        rejected_terminals=ctx.rejected_terminals;
        dead_ends=ctx.dead_ends;
        fatal=ctx.fatal;
        errors.extend(fatal);
    }
    feasible_paths.sort_by(|left,right| left.action_ids.cmp(&right.action_ids).then(left.terminal_id.cmp(&right.terminal_id)));
    let feasible_path_reports=feasible_paths.iter().map(|path| json!({
        "terminal_id":path.terminal_id,
        "terminal_class":path.terminal_class,
        "action_ids":path.action_ids,
        "total_duration_upper_hours":path.total_duration_upper_hours,
        "minimum_conservative_margin_by_metric":path.minimum_margin_by_metric.iter().map(|(metric,(margin,at))|json!({
            "metric":metric,"margin":margin,"at":at
        })).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    let scenario_disposition=if !errors.is_empty() {"INVALID"} else if feasible_paths.is_empty() {"INFEASIBLE/ABSTAIN"} else {"RECOVERY_REACHABLE"};
    json!({
        "scenario_id":scenario_id,
        "scenario_class":scenario_class,
        "disposition":scenario_disposition,
        "feasible_path_count":feasible_paths.len(),
        "feasible_paths":feasible_path_reports,
        "rejected_actions":rejected_actions,
        "rejected_terminals":rejected_terminals,
        "dead_ends":dead_ends,
        "findings":{"errors":errors}
    })
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    if string_at(value,"schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value,"scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value,"claim_ceiling") != Some("bounded_action_recovery_reachability_audit_only") {
        errors.push("claim_ceiling must remain bounded-action reachability audit only".to_owned());
    }
    let record_status=string_at(value,"record_status").unwrap_or("");
    if !["synthetic","shadow_replay","measured_campaign","published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing/unsupported".to_owned());
    }
    let contract_value=value.get("service_contract").unwrap_or(&Value::Null);
    let contract=validate_contract(contract_value,&mut errors);
    let snapshot=value.get("mission_snapshot").unwrap_or(&Value::Null);
    let contract_sha=array_digest(contract_value).unwrap_or_default();
    if string_at(snapshot,"service_contract_sha256") != Some(contract_sha.as_str()) {
        errors.push("service_contract_sha256 does not match canonical contract".to_owned());
    }
    for (array_name,snapshot_name) in [
        ("capability_registry","capability_registry_sha256"),
        ("component_registry","component_registry_sha256"),
        ("hard_gate_roster","hard_gate_roster_sha256"),
        ("scenario_roster","scenario_roster_sha256"),
    ] {
        let array=value.get(array_name).unwrap_or(&Value::Null);
        let digest=array_digest(array).unwrap_or_default();
        if string_at(snapshot,snapshot_name) != Some(digest.as_str()) {
            errors.push(format!("{snapshot_name} does not match {array_name}"));
        }
        if let Some(items)=array.as_array() {
            let strings=items.iter().filter_map(Value::as_str).collect::<Vec<_>>();
            let unique=strings.iter().copied().collect::<BTreeSet<_>>();
            if strings.len()!=items.len() || unique.len()!=items.len() {
                errors.push(format!("{array_name} has non-string or duplicate values"));
            }
        }
    }
    let scenarios=value.get("scenarios").and_then(Value::as_array);
    if scenarios.is_none_or(|items| items.len()!=SCENARIO_CLASSES.len()) {
        errors.push("exactly seven mandatory recovery scenarios are required".to_owned());
    }
    let roster=string_set(value.get("scenario_roster").unwrap_or(&Value::Null));
    let expected_roster=SCENARIO_CLASSES.iter().map(|item|item.to_string()).collect::<BTreeSet<_>>();
    if roster!=expected_roster {
        errors.push("scenario_roster does not match the mandatory recovery classes".to_owned());
    }

    let caps=string_set(value.get("capability_registry").unwrap_or(&Value::Null));
    let components=string_set(value.get("component_registry").unwrap_or(&Value::Null));
    let hard_gate_roster=string_set(value.get("hard_gate_roster").unwrap_or(&Value::Null));
    let mut reports=Vec::<Value>::new();
    let mut seen_classes=BTreeSet::<String>::new();
    let mut seen_scenarios=BTreeSet::<String>::new();
    if let Some(scenarios)=scenarios {
        for scenario in scenarios {
            let class=string_at(scenario,"scenario_class").unwrap_or("").to_owned();
            let id=string_at(scenario,"scenario_id").unwrap_or("").to_owned();
            if !seen_classes.insert(class.clone()) {
                errors.push(format!("duplicate recovery scenario class '{class}'"));
            }
            if !seen_scenarios.insert(id.clone()) {
                errors.push(format!("duplicate recovery scenario id '{id}'"));
            }
            if !expected_roster.contains(&class) {
                errors.push(format!("unknown recovery scenario class '{class}'"));
            }
            validate_state_digests(scenario,&mut errors);
            reports.push(validate_scenario(scenario,&class,&contract,&hard_gate_roster,record_status,snapshot));
        }
    }
    if seen_classes!=expected_roster {
        errors.push("scenario records do not exactly cover the seven mandatory recovery classes".to_owned());
    }
    if caps.is_empty() || components.is_empty() || hard_gate_roster.is_empty() {
        errors.push("capability, component and hard-gate registries must be non-empty".to_owned());
    }
    digest_field_matches(snapshot,"mission_snapshot_sha256",Some("mission_snapshot_sha256"),&mut errors,"mission snapshot");
    for field in [
        "craft_configuration_sha256","occupancy_sha256","service_contract_sha256",
        "capability_registry_sha256","component_registry_sha256","hard_gate_roster_sha256",
        "scenario_roster_sha256","operating_envelope_sha256","initial_resource_state_sha256",
    ] {
        if !digest_valid(string_at(snapshot,field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }
    let all_reachable=reports.iter().all(|report| report["disposition"]=="RECOVERY_REACHABLE");
    let any_invalid=reports.iter().any(|report| report["disposition"]=="INVALID");
    let global_disposition=if !errors.is_empty() || any_invalid {
        "INVALID"
    } else if all_reachable {
        if record_status=="synthetic" {"SYNTHETIC_ONLY"} else {"RECOVERY_AUDITED_READ_ONLY"}
    } else if record_status=="synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "RECOVERY_AUDITED_READ_ONLY"
    };
    let global_candidate=if all_reachable && errors.is_empty() && !any_invalid {
        "ALL_REQUIRED_SCENARIOS_HAVE_A_VERIFIED_RECOVERY_PATH"
    } else {
        "INFEASIBLE/ABSTAIN"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":global_disposition,
        "candidate_disposition":global_candidate,
        "input_sha256":sha256_hex(raw),
        "scenario_count":reports.len(),
        "scenario_reports":reports,
        "aggregate_scalar_score":"NOT_COMPUTED",
        "claim_ceiling":"bounded_finite_action_graph_reachability_not_continuous_viability_kernel_not_navigation_authority",
        "findings":{"errors":errors},
        "note":"This enumerates a finite graph of declared, evidence-bound action envelopes. It does not derive physical reachability, prove continuous dynamics, establish probabilities, guarantee a refuge or authorize an action. A successful path is only as credible as its qualified action, state, segment, terminal and authority evidence."
    })
}

fn run() -> Result<(),String> {
    let args=env::args().collect::<Vec<_>>();
    if args.len()!=3 || args[1]!="audit-recovery" {
        return Err("Usage: marine-recovery-reachability audit-recovery <fixture.json>".to_owned());
    }
    let raw=fs::read(&args[2]).map_err(|error|format!("{}: {error}",args[2]))?;
    let value:Value=serde_json::from_slice(&raw).map_err(|error|format!("{}: invalid JSON: {error}",args[2]))?;
    let report=validate(&raw,&value);
    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
    match string_at(&report,"disposition") {
        Some("SYNTHETIC_ONLY"|"RECOVERY_AUDITED_READ_ONLY")=>Ok(()),
        Some(disposition)=>Err(format!("recovery audit disposition: {disposition}")),
        None=>Err("recovery report has no disposition".to_owned())
    }
}

fn main()->ExitCode {
    match run() {
        Ok(())=>ExitCode::SUCCESS,
        Err(error)=>{eprintln!("{error}");ExitCode::from(2)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture()->Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-recovery-reachability-0001.json"))
            .expect("recovery fixture parses")
    }

    fn refresh(value:&mut Value) {
        let contract=value["service_contract"].clone();
        value["mission_snapshot"]["service_contract_sha256"]=json!(array_digest(&contract).unwrap());
        for (array_name,snapshot_name) in [
            ("capability_registry","capability_registry_sha256"),
            ("component_registry","component_registry_sha256"),
            ("hard_gate_roster","hard_gate_roster_sha256"),
            ("scenario_roster","scenario_roster_sha256"),
        ] {
            value["mission_snapshot"][snapshot_name]=json!(array_digest(&value[array_name]).unwrap());
        }
        if let Some(scenarios)=value["scenarios"].as_array_mut() {
            for scenario in scenarios {
                if let Some(states)=scenario["states"].as_array_mut() {
                    for state in states {
                        state["state_sha256"]=json!(canonical_digest(state,Some("state_sha256")).unwrap());
                    }
                }
                if let Some(actions)=scenario["actions"].as_array_mut() {
                    for action in actions {
                        if let Some(checkpoints)=action["checkpoints"].as_array_mut() {
                            for checkpoint in checkpoints {
                                checkpoint["checkpoint_sha256"]=json!(canonical_digest(checkpoint,Some("checkpoint_sha256")).unwrap());
                            }
                        }
                        action["action_sha256"]=json!(canonical_digest(action,Some("action_sha256")).unwrap());
                    }
                }
                scenario["scenario_sha256"]=json!(canonical_digest(scenario,Some("scenario_sha256")).unwrap());
            }
        }
        let initial=value["scenarios"][0]["states"][0]["resource_metrics"].clone();
        let initial_projection=initial.as_array().unwrap().iter().map(|item|json!({
            "metric":item["metric"],"unit":item["unit"],"value":item["value"],
            "lower_bound":item["lower_bound"],"upper_bound":item["upper_bound"]
        })).collect::<Vec<_>>();
        value["mission_snapshot"]["initial_resource_state_sha256"]=json!(array_digest(&json!(initial_projection)).unwrap());
        value["mission_snapshot"]["mission_snapshot_sha256"]=json!(canonical_digest(&value["mission_snapshot"],Some("mission_snapshot_sha256")).unwrap());
    }

    fn audit(value:&Value)->Value {
        let raw=serde_json::to_vec(value).unwrap();
        validate(&raw,value)
    }

    #[test]
    fn corpus_finds_refuge_repair_and_safe_hold_options() {
        let report=audit(&fixture());
        assert_eq!(report["disposition"],"SYNTHETIC_ONLY");
        let generator=&report["scenario_reports"][0];
        assert_eq!(generator["disposition"],"RECOVERY_REACHABLE");
        let classes=generator["feasible_paths"].as_array().unwrap().iter()
            .map(|path|path["terminal_class"].as_str().unwrap()).collect::<BTreeSet<_>>();
        assert!(classes.contains("safe_haven"));
        assert!(classes.contains("repair_restoration"));
        assert!(classes.contains("controlled_safe_hold"));
    }

    #[test]
    fn watermaker_failure_requires_verified_shore_resupply() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="watermaker_failure").unwrap();
        assert_eq!(scenario["disposition"],"RECOVERY_REACHABLE");
        assert_eq!(scenario["feasible_paths"][0]["terminal_class"],"shore_resupply");
        assert_eq!(scenario["feasible_paths"][0]["action_ids"].as_array().unwrap().len(),2);
    }

    #[test]
    fn common_cause_outage_with_no_viable_path_abstains() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="common_cause_power_water").unwrap();
        assert_eq!(scenario["disposition"],"INFEASIBLE/ABSTAIN");
        assert_eq!(scenario["feasible_path_count"],0);
    }

    #[test]
    fn all_options_that_breach_a_resource_floor_are_rejected() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="resource_depletion").unwrap();
        assert_eq!(scenario["disposition"],"INFEASIBLE/ABSTAIN");
        assert_eq!(scenario["feasible_path_count"],0);
        assert_eq!(scenario["rejected_actions"].as_array().unwrap().len(),2);
    }

    #[test]
    fn unavailable_repair_dependency_is_not_accepted() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="dependency_unavailable").unwrap();
        assert_eq!(scenario["feasible_path_count"],0);
        assert!(scenario["rejected_actions"][0]["blockers"].as_array().unwrap().iter()
            .any(|item|item.as_str().unwrap().contains("generator_repair_kit")));
    }

    #[test]
    fn reachable_port_with_unverified_service_is_not_a_terminal_success() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="service_unverified").unwrap();
        assert_eq!(scenario["feasible_path_count"],0);
        assert!(scenario["rejected_terminals"][0]["blockers"].as_array().unwrap().iter()
            .any(|item|item.as_str().unwrap().contains("not verified")));
    }

    #[test]
    fn slower_viable_route_is_retained_alongside_rejected_fast_route() {
        let report=audit(&fixture());
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="competing_paths").unwrap();
        assert_eq!(scenario["feasible_path_count"],1);
        assert_eq!(scenario["feasible_paths"][0]["action_ids"][0],"slow-but-viable");
        assert!(scenario["rejected_actions"].as_array().unwrap().iter()
            .any(|item|item["action_id"]=="fast-but-infeasible"));
        assert_eq!(scenario["feasible_paths"][0]["total_duration_upper_hours"],8);
    }

    #[test]
    fn action_hard_gate_roster_must_match_the_frozen_registry() {
        let mut value=fixture();
        value["scenarios"][0]["actions"][0]["hard_gate_checks"].as_array_mut().unwrap().pop();
        refresh(&mut value);
        let report=audit(&value);
        assert_eq!(report["scenario_reports"][0]["feasible_path_count"],0);
        assert!(report["scenario_reports"][0]["rejected_actions"][0]["blockers"].as_array().unwrap().iter()
            .any(|item|item.as_str().unwrap().contains("roster")));
    }

    #[test]
    fn successor_interval_cannot_narrow_the_reachable_uncertainty() {
        let mut value=fixture();
        value["scenarios"][0]["states"][1]["resource_metrics"][0]["lower_bound"]=json!(14);
        value["scenarios"][0]["states"][1]["resource_metrics"][0]["upper_bound"]=json!(14);
        refresh(&mut value);
        let report=audit(&value);
        assert_eq!(report["scenario_reports"][0]["feasible_path_count"],0);
        assert!(report["scenario_reports"][0]["rejected_actions"][0]["blockers"].as_array().unwrap().iter()
            .any(|item|item.as_str().unwrap().contains("narrows")));
    }

    #[test]
    fn scenario_digest_tampering_is_invalid() {
        let mut value=fixture();
        value["scenarios"][0]["states"][0]["capabilities"].as_array_mut().unwrap().pop();
        let report=audit(&value);
        assert_eq!(report["scenario_reports"][0]["disposition"],"INVALID");
    }

    #[test]
    fn unknown_hard_gate_result_blocks_a_recovery_action() {
        let mut value=fixture();
        value["scenarios"][0]["actions"][0]["hard_gate_checks"][0]["status"]=json!("unknown");
        refresh(&mut value);
        let report=audit(&value);
        assert_eq!(report["scenario_reports"][0]["feasible_path_count"],2);
        assert!(report["scenario_reports"][0]["rejected_actions"].as_array().unwrap().iter()
            .any(|item|item["action_id"]=="divert-to-refuge"));
    }

    #[test]
    fn repeated_state_cycle_is_rejected_without_looping() {
        let mut value=fixture();
        let s=&mut value["scenarios"][0];
        let mut cycle=s["actions"][0].clone();
        cycle["action_id"]=json!("cycle-back");
        cycle["from_state_id"]=s["states"][1]["state_id"].clone();
        cycle["to_state_id"]=s["states"][0]["state_id"].clone();
        cycle["allowed_scenario_classes"]=json!(["generator_trip"]);
        cycle["required_capabilities"]=json!([]);
        cycle["required_components_available"]=json!([]);
        cycle["duration_min_hours"]=json!(1);
        cycle["duration_max_hours"]=json!(1);
        cycle["resource_deltas"]=json!(METRICS.iter().map(|(metric,unit,_)|json!({
            "metric":metric,"unit":unit,"lower_delta":0,"upper_delta":0
        })).collect::<Vec<_>>());
        cycle["component_changes"]=json!([]);
        cycle["checkpoints"]=json!([{
            "checkpoint_id":"cycle-checkpoint","checkpoint_sha256":"0".repeat(64),
            "elapsed_upper_hours":1,"resource_metrics":s["states"][0]["resource_metrics"],
            "evidence_ref":"SYNTHETIC-CYCLE","evidence_sha256":"1".repeat(64)
        }]);
        cycle["hard_gate_checks"]=s["actions"][0]["hard_gate_checks"].clone();
        cycle["action_sha256"]=json!("2".repeat(64));
        s["actions"].as_array_mut().unwrap().push(cycle);
        refresh(&mut value);
        let report=audit(&value);
        assert!(report["scenario_reports"][0]["rejected_actions"].as_array().unwrap().iter()
            .any(|item|item["action_id"]=="cycle-back"));
    }
}
