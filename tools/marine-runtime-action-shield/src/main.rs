use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-runtime-action-shield/0.1.0";
const SCENARIO_CLASSES: [&str; 5] = [
    "adverse_forecast_error",
    "common_cause_generation_watermaker_outage",
    "main_generation_outage",
    "nominal",
    "watermaker_outage",
];
const METRIC_IDS: [&str; 11] = [
    "energy_reserve_kwh",
    "freshwater_usable_l",
    "food_reserve_days",
    "waste_capacity_margin_days",
    "critical_spares_coverage_days",
    "power_service_availability",
    "potable_water_service_availability",
    "sanitation_service_availability",
    "refrigeration_service_availability",
    "crew_workload_hours_rolling_24h",
    "comfort_breach_minutes_rolling_24h",
];

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

fn canonical_json(value: &Value) -> Result<Vec<u8>, String> {
    fn write(value: &Value, output: &mut String) -> Result<(), String> {
        match value {
            Value::Null => output.push_str("null"),
            Value::Bool(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::Number(value) => output.push_str(&value.to_string()),
            Value::String(value) => output.push_str(
                &serde_json::to_string(value).map_err(|error| error.to_string())?
            ),
            Value::Array(values) => {
                output.push('[');
                for (index, item) in values.iter().enumerate() {
                    if index > 0 { output.push(','); }
                    write(item, output)?;
                }
                output.push(']');
            }
            Value::Object(values) => {
                output.push('{');
                let mut keys = values.keys().collect::<Vec<_>>();
                keys.sort();
                for (index, key) in keys.iter().enumerate() {
                    if index > 0 { output.push(','); }
                    output.push_str(&serde_json::to_string(*key).map_err(|error| error.to_string())?);
                    output.push(':');
                    write(&values[*key], output)?;
                }
                output.push('}');
            }
        }
        Ok(())
    }
    let mut output = String::new();
    write(value, &mut output)?;
    Ok(output.into_bytes())
}

fn canonical_digest(value: &Value, excluded_key: Option<&str>) -> Option<String> {
    let mut copy = value.clone();
    if let (Some(key), Some(object)) = (excluded_key, copy.as_object_mut()) {
        object.remove(key);
    }
    canonical_json(&copy).ok().map(|bytes| sha256_hex(&bytes))
}

fn hash_newline_list(values: &[String]) -> String {
    sha256_hex(values.join("\n").as_bytes())
}

#[derive(Clone, Copy)]
struct MetricContract {
    unit: String,
    operator: String,
    floor: f64,
    min_coverage: f64,
}

fn parse_contract(value: &Value, errors: &mut Vec<String>) -> BTreeMap<String, MetricContract> {
    let mut result = BTreeMap::<String, MetricContract>::new();
    let Some(items) = value.as_array() else {
        errors.push("service_contract must be an array".to_owned());
        return result;
    };
    if items.len() != METRIC_IDS.len() {
        errors.push("service_contract must contain exactly eleven resource/service axes".to_owned());
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let floor = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        if !METRIC_IDS.contains(&metric) || unit.is_empty()
            || !["gte", "lte"].contains(&operator)
            || floor.is_none()
            || coverage.is_none_or(|v| !(0.0..=1.0).contains(&v))
        {
            errors.push(format!("service contract has an unknown, incomplete or invalid axis '{metric}'"));
            continue;
        }
        if result.insert(metric.to_owned(), MetricContract {
            unit: unit.to_owned(),
            operator: operator.to_owned(),
            floor: floor.unwrap(),
            min_coverage: coverage.unwrap(),
        }).is_some() {
            errors.push(format!("service contract duplicates axis '{metric}'"));
        }
    }
    for metric in METRIC_IDS {
        if !result.contains_key(metric) {
            errors.push(format!("service contract is missing required axis '{metric}'"));
        }
    }
    result
}

#[derive(Clone)]
struct MetricInterval {
    value: f64,
    lower: f64,
    upper: f64,
    coverage: f64,
}

fn read_metric_intervals(
    value: &Value,
    contract: &BTreeMap<String, MetricContract>,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeMap<String, MetricInterval> {
    let mut result = BTreeMap::<String, MetricInterval>::new();
    let Some(items) = value.as_array() else {
        errors.push(format!("{label} resource_metrics must be an array"));
        return result;
    };
    if items.len() != METRIC_IDS.len() {
        errors.push(format!("{label} must report exactly eleven resource/service axes"));
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let (Some(value), Some(lower), Some(upper), Some(coverage)) = (
            number_at(item, "value"),
            number_at(item, "lower_bound"),
            number_at(item, "upper_bound"),
            number_at(item, "coverage_fraction"),
        ) else {
            errors.push(format!("{label} metric '{metric}' has missing or non-finite interval data"));
            continue;
        };
        let Some(spec) = contract.get(metric) else {
            errors.push(format!("{label} contains unknown resource axis '{metric}'"));
            continue;
        };
        if unit != spec.unit || lower > value || value > upper
            || !(0.0..=1.0).contains(&coverage)
            || !digest_valid(string_at(item, "evidence_sha256"))
        {
            errors.push(format!("{label} metric '{metric}' has invalid units, bounds, coverage or evidence digest"));
            continue;
        }
        if result.insert(metric.to_owned(), MetricInterval { value, lower, upper, coverage }).is_some() {
            errors.push(format!("{label} duplicates resource axis '{metric}'"));
        }
    }
    for metric in METRIC_IDS {
        if !result.contains_key(metric) {
            errors.push(format!("{label} is missing required axis '{metric}'"));
        }
    }
    result
}

fn metric_floor_pass(
    metric: &str,
    interval: &MetricInterval,
    contract: &MetricContract,
) -> Result<(), String> {
    if interval.coverage < contract.min_coverage {
        return Err(format!("{metric}: coverage {:.3} below {:.3}", interval.coverage, contract.min_coverage));
    }
    let margin = match contract.operator.as_str() {
        "gte" => interval.lower - contract.floor,
        "lte" => contract.floor - interval.upper,
        _ => return Err(format!("{metric}: unsupported floor operator")),
    };
    if margin < -1e-9 {
        return Err(format!(
            "{metric}: conservative margin {margin:.6} violates {} {:.6} {}",
            contract.operator, contract.floor, contract.unit
        ));
    }
    Ok(())
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

fn check_metric_floor_set(
    value: &Value,
    contract: &BTreeMap<String, MetricContract>,
    label: &str,
    blockers: &mut Vec<String>,
    errors: &mut Vec<String>,
) {
    let intervals = read_metric_intervals(value, contract, label, errors);
    for (metric, interval) in intervals {
        if let Some(spec) = contract.get(&metric) {
            if let Err(reason) = metric_floor_pass(&metric, &interval, spec) {
                blockers.push(format!("{label}: {reason}"));
            }
        }
    }
}

fn validate_bundle(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut global_blockers = Vec::<String>::new();
    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value, "claim_ceiling") != Some("one_step_runtime_decision_receipt_only") {
        errors.push("claim_ceiling must remain one_step_runtime_decision_receipt_only".to_owned());
    }
    let record_status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let contract_value = value.get("service_contract").unwrap_or(&Value::Null);
    let contract = parse_contract(contract_value, &mut errors);
    match canonical_digest(contract_value, None) {
        Some(calculated) if string_at(value.get("mission_snapshot").unwrap_or(&Value::Null), "service_contract_sha256") == Some(calculated.as_str()) => {}
        _ => errors.push("service_contract_sha256 does not bind the exact service contract".to_owned()),
    }

    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    match canonical_digest(snapshot, Some("mission_snapshot_sha256")) {
        Some(calculated) if string_at(snapshot, "mission_snapshot_sha256") == Some(calculated.as_str()) => {}
        _ => errors.push("mission_snapshot_sha256 does not bind the canonical mission snapshot".to_owned()),
    }
    let hard_gates = value.get("hard_gate_roster").and_then(Value::as_array);
    let mut hard_gate_ids = Vec::<String>::new();
    if let Some(items) = hard_gates {
        hard_gate_ids.extend(items.iter().filter_map(Value::as_str).map(str::to_owned));
    } else {
        errors.push("hard_gate_roster must be an array".to_owned());
    }
    if hard_gate_ids.is_empty() || hard_gate_ids.iter().collect::<BTreeSet<_>>().len() != hard_gate_ids.len() {
        errors.push("hard_gate_roster must contain non-empty unique IDs".to_owned());
    }
    if string_at(snapshot, "hard_gate_roster_sha256") != Some(hash_newline_list(&hard_gate_ids).as_str()) {
        errors.push("hard_gate_roster_sha256 does not match the declared ordered hard-gate roster".to_owned());
    }

    let scenario_roster = value.get("scenario_roster").and_then(Value::as_array);
    let mut scenario_ids = Vec::<String>::new();
    if let Some(items) = scenario_roster {
        scenario_ids.extend(items.iter().filter_map(Value::as_str).map(str::to_owned));
    } else {
        errors.push("scenario_roster must be an array".to_owned());
    }
    if scenario_ids.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario_roster must contain the exact five required disturbance classes in canonical order".to_owned());
    }
    if string_at(snapshot, "scenario_roster_sha256") != Some(hash_newline_list(&scenario_ids).as_str()) {
        errors.push("scenario_roster_sha256 does not match the declared scenario roster".to_owned());
    }

    let disturbance_roster = value.get("disturbance_roster").unwrap_or(&Value::Null);
    let mut disturbance_by_class = BTreeMap::<String, String>::new();
    if let Some(items) = disturbance_roster.as_array() {
        for item in items {
            let class = string_at(item, "scenario_class").unwrap_or("");
            let digest = string_at(item, "disturbance_sha256").unwrap_or("");
            if !SCENARIO_CLASSES.contains(&class) || !digest_valid(Some(digest))
                || string_at(item, "disturbance_id").is_none_or(str::is_empty)
                || disturbance_by_class.insert(class.to_owned(), digest.to_owned()).is_some()
            {
                errors.push("disturbance_roster has an unknown/duplicate class or missing ID/digest".to_owned());
            }
        }
    } else {
        errors.push("disturbance_roster must be an array".to_owned());
    }
    match canonical_digest(disturbance_roster, None) {
        Some(calculated) if string_at(snapshot, "disturbance_roster_sha256") == Some(calculated.as_str()) => {}
        _ => errors.push("disturbance_roster_sha256 does not bind the declared disturbance evidence".to_owned()),
    }
    if disturbance_by_class.len() != SCENARIO_CLASSES.len() {
        errors.push("disturbance_roster must cover all five required classes".to_owned());
    }

    for (field, key) in [
        ("craft_configuration_sha256", "craft_configuration_sha256"),
        ("actuator_configuration_sha256", "actuator_configuration_sha256"),
        ("model_sha256", "model_sha256"),
        ("operating_domain_sha256", "operating_domain_sha256"),
    ] {
        if !digest_valid(string_at(snapshot, field)) {
            errors.push(format!("mission snapshot has invalid {key}"));
        }
    }

    let state = value.get("current_state").unwrap_or(&Value::Null);
    match canonical_digest(state, Some("state_sha256")) {
        Some(calculated) if string_at(state, "state_sha256") == Some(calculated.as_str()) => {}
        _ => errors.push("state_sha256 does not bind the canonical current state".to_owned()),
    }
    for field in [
        "craft_configuration_sha256", "actuator_configuration_sha256",
        "model_sha256", "operating_domain_sha256",
    ] {
        if string_at(state, field) != string_at(snapshot, field) {
            errors.push(format!("current state {field} differs from the frozen mission snapshot"));
        }
    }
    let state_sequence = state.get("state_sequence").and_then(Value::as_u64);
    if state_sequence.is_none_or(|sequence| sequence == 0) {
        errors.push("current state requires a positive monotonic state_sequence".to_owned());
    }
    let decision = value.get("decision").unwrap_or(&Value::Null);
    let decision_time = parse_time(string_at(decision, "decision_time"));
    let observed_at = parse_time(string_at(state, "observed_at"));
    let valid_until = parse_time(string_at(state, "valid_until"));
    let mut state_fresh = true;
    match (decision_time, observed_at, valid_until) {
        (Some(now), Some(observed), Some(expiry)) => {
            if observed > now || expiry <= now {
                state_fresh = false;
                global_blockers.push("current state is future-dated or expired at the decision time".to_owned());
            } else {
                let age = (now - observed).num_seconds();
                let maximum_age = decision.get("max_state_age_seconds").and_then(Value::as_i64).unwrap_or(0);
                if maximum_age <= 0 || age > maximum_age {
                    state_fresh = false;
                    global_blockers.push(format!("current state age {age}s exceeds the maximum permitted freshness"));
                }
            }
        }
        _ => {
            state_fresh = false;
            errors.push("decision/current-state times must be valid RFC3339 timestamps".to_owned());
        }
    }
    let nonce = string_at(decision, "permit_nonce_sha256").unwrap_or("");
    if !digest_valid(Some(nonce)) {
        errors.push("permit_nonce_sha256 must be a SHA-256 digest".to_owned());
    }
    let consumed = value.get("permit_registry").and_then(|registry| registry.get("consumed_nonce_sha256")).and_then(Value::as_array);
    if consumed.is_none() {
        errors.push("permit_registry.consumed_nonce_sha256 must be an array".to_owned());
    } else if consumed.unwrap().iter().any(|item| item.as_str() == Some(nonce)) {
        state_fresh = false;
        global_blockers.push("permit nonce has already been consumed; replay is denied".to_owned());
    }
    if string_at(value.get("authority_scope").unwrap_or(&Value::Null), "scope") != Some("runtime_decision_receipt_only")
        || string_at(value.get("authority_scope").unwrap_or(&Value::Null), "authority_ref").is_none_or(str::is_empty)
        || !digest_valid(string_at(value.get("authority_scope").unwrap_or(&Value::Null), "authority_sha256"))
    {
        errors.push("authority_scope must be a valid decision-receipt-only authority record".to_owned());
    }

    let current_metrics = read_metric_intervals(
        state.get("resource_metrics").unwrap_or(&Value::Null), &contract, "current_state", &mut errors
    );
    for (metric, interval) in &current_metrics {
        if let Some(spec) = contract.get(metric) {
            if let Err(reason) = metric_floor_pass(metric, interval, spec) {
                state_fresh = false;
                global_blockers.push(format!("current state is outside the admitted service envelope: {reason}"));
            }
        }
    }
    let capabilities = state.get("available_capabilities").and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<BTreeSet<_>>())
        .unwrap_or_default();

    let candidates = value.get("candidates").and_then(Value::as_array);
    if candidates.is_none_or(Vec::is_empty) {
        errors.push("at least one action candidate is required".to_owned());
    }
    let mut proposed_count = 0_usize;
    let mut fallback_priorities = BTreeSet::<u64>::new();
    let mut candidate_ids = BTreeSet::<String>::new();
    let mut candidate_reports = BTreeMap::<String, (bool, Vec<String>, String, usize)>::new();

    if let Some(candidates) = candidates {
        for candidate in candidates {
            let candidate_id = string_at(candidate, "candidate_id").unwrap_or("").to_owned();
            let role = string_at(candidate, "role").unwrap_or("");
            if candidate_id.is_empty() || !candidate_ids.insert(candidate_id.clone()) {
                errors.push("candidate IDs must be non-empty and unique".to_owned());
                continue;
            }
            let mut reasons = Vec::<String>::new();
            let mut candidate_invalid = false;

            if !digest_valid(string_at(candidate, "action_digest_sha256"))
                || canonical_digest(candidate, Some("action_digest_sha256")).as_deref()
                    != string_at(candidate, "action_digest_sha256")
            {
                candidate_invalid = true;
                reasons.push("action digest does not match the canonical candidate payload".to_owned());
            }
            for field in [
                "transition_relation_sha256", "action_evidence_sha256",
                "precondition_state_sha256", "craft_configuration_sha256",
                "actuator_configuration_sha256", "model_sha256", "operating_domain_sha256",
            ] {
                if !digest_valid(string_at(candidate, field)) {
                    candidate_invalid = true;
                    reasons.push(format!("{field} is missing or invalid"));
                }
            }
            if candidate.get("precondition_state_sequence").and_then(Value::as_u64) != state_sequence
                || string_at(candidate, "precondition_state_sha256") != string_at(state, "state_sha256")
            {
                reasons.push("candidate precondition does not match the current state sequence and digest".to_owned());
            }
            for field in [
                "craft_configuration_sha256", "actuator_configuration_sha256",
                "model_sha256", "operating_domain_sha256",
            ] {
                if string_at(candidate, field) != string_at(snapshot, field) {
                    reasons.push(format!("candidate {field} does not match mission scope"));
                }
            }
            let required_capabilities = candidate.get("required_capabilities").and_then(Value::as_array)
                .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>())
                .unwrap_or_default();
            for capability in required_capabilities {
                if !capabilities.contains(capability) {
                    reasons.push(format!("required capability '{capability}' is unavailable"));
                }
            }
            let duration = number_at(candidate, "duration_seconds");
            if duration.is_none_or(|seconds| seconds <= 0.0) {
                candidate_invalid = true;
                reasons.push("candidate duration_seconds must be positive and finite".to_owned());
            }

            let successor_list = candidate.get("scenario_successors").and_then(Value::as_array);
            if successor_list.is_none_or(|items| items.len() != SCENARIO_CLASSES.len()) {
                candidate_invalid = true;
                reasons.push("candidate must declare exactly five one-step disturbance successors".to_owned());
            }
            let mut successor_by_class = BTreeMap::<String, &Value>::new();
            if let Some(successors) = successor_list {
                for successor in successors {
                    let class = string_at(successor, "scenario_class").unwrap_or("");
                    if !SCENARIO_CLASSES.contains(&class)
                        || successor_by_class.insert(class.to_owned(), successor).is_some()
                    {
                        candidate_invalid = true;
                        reasons.push("candidate contains an unknown or duplicated disturbance successor".to_owned());
                        continue;
                    }
                    let expected_disturbance = disturbance_by_class.get(class).map(String::as_str);
                    if expected_disturbance != string_at(successor, "disturbance_sha256") {
                        candidate_invalid = true;
                        reasons.push(format!("successor '{class}' does not bind the frozen disturbance evidence"));
                    }
                    if !digest_valid(string_at(successor, "successor_state_sha256"))
                        || canonical_digest(successor, Some("successor_state_sha256")).as_deref()
                            != string_at(successor, "successor_state_sha256")
                    {
                        candidate_invalid = true;
                        reasons.push(format!("successor '{class}' digest does not bind its canonical one-step state"));
                    }
                    let label = format!("candidate '{candidate_id}' successor '{class}'");
                    let metrics = read_metric_intervals(
                        successor.get("resource_metrics").unwrap_or(&Value::Null), &contract, &label, &mut errors
                    );
                    for (metric, interval) in metrics {
                        if let Some(spec) = contract.get(&metric) {
                            if let Err(reason) = metric_floor_pass(&metric, &interval, spec) {
                                reasons.push(format!("{label}: {reason}"));
                            }
                        }
                    }

                    let checks = successor.get("hard_gate_checks").and_then(Value::as_array);
                    let mut seen_gates = BTreeSet::<String>::new();
                    if checks.is_none_or(|items| items.len() != hard_gate_ids.len()) {
                        reasons.push(format!("{label}: hard-gate check set is incomplete"));
                    }
                    if let Some(checks) = checks {
                        for check in checks {
                            let gate_id = string_at(check, "gate_id").unwrap_or("");
                            let status = string_at(check, "status").unwrap_or("");
                            if !hard_gate_ids.iter().any(|id| id == gate_id)
                                || !seen_gates.insert(gate_id.to_owned())
                                || !digest_valid(string_at(check, "evidence_sha256"))
                            {
                                candidate_invalid = true;
                                reasons.push(format!("{label}: invalid hard-gate identity/evidence"));
                                continue;
                            }
                            if status != "pass" {
                                reasons.push(format!("{label}: hard gate '{gate_id}' is {status}, not pass"));
                            }
                        }
                    }
                    if seen_gates.len() != hard_gate_ids.len() {
                        reasons.push(format!("{label}: not every mandatory hard gate is independently checked"));
                    }

                    let recovery = successor.get("recoverability").unwrap_or(&Value::Null);
                    let recovery_status = string_at(recovery, "status").unwrap_or("");
                    let margin = number_at(recovery, "minimum_intervention_margin_seconds");
                    let required_margin = number_at(recovery, "required_intervention_margin_seconds");
                    if !digest_valid(string_at(recovery, "evidence_sha256")) {
                        candidate_invalid = true;
                        reasons.push(format!("{label}: recoverability evidence digest is missing"));
                    }
                    if recovery_status != "reachable" || margin.is_none() || required_margin.is_none()
                        || margin.unwrap_or(f64::NEG_INFINITY) < required_margin.unwrap_or(f64::INFINITY)
                    {
                        reasons.push(format!("{label}: no demonstrated recovery margin before the intervention deadline"));
                    }
                }
            }
            if successor_by_class.len() != SCENARIO_CLASSES.len() {
                candidate_invalid = true;
                reasons.push("candidate does not cover the exact five-class scenario roster".to_owned());
            }

            let safe = !candidate_invalid && reasons.is_empty();
            if role == "proposed" {
                proposed_count += 1;
                if candidate.get("fallback_priority").and_then(Value::as_u64) != Some(0) {
                    errors.push("proposed action must use fallback_priority 0".to_owned());
                }
            } else if role == "fallback" {
                match candidate.get("fallback_priority").and_then(Value::as_u64) {
                    Some(priority) if priority > 0 && fallback_priorities.insert(priority) => {}
                    _ => errors.push("fallback priorities must be positive, unique integers".to_owned()),
                }
            } else {
                errors.push(format!("candidate '{candidate_id}' has unsupported role"));
            }
            let action = string_at(candidate, "action_id").unwrap_or("").to_owned();
            candidate_reports.insert(candidate_id, (safe, reasons, action, candidate.get("fallback_priority").and_then(Value::as_u64).unwrap_or(u64::MAX) as usize));
        }
    }
    if proposed_count != 1 {
        errors.push("exactly one proposed action is required".to_owned());
    }
    for expected in 1..=fallback_priorities.len() as u64 {
        if !fallback_priorities.contains(&expected) {
            errors.push("fallback priorities must be contiguous starting at one".to_owned());
            break;
        }
    }

    let mut selected: Option<(String, String, String)> = None;
    if state_fresh && errors.is_empty() {
        if let Some(candidates) = candidates {
            if let Some(proposed) = candidates.iter().find(|item| string_at(item, "role") == Some("proposed")) {
                let id = string_at(proposed, "candidate_id").unwrap_or("");
                if candidate_reports.get(id).is_some_and(|report| report.0) {
                    selected = Some((id.to_owned(), string_at(proposed, "action_id").unwrap_or("").to_owned(), "proposed".to_owned()));
                }
            }
            if selected.is_none() {
                let mut fallbacks = candidates.iter()
                    .filter(|item| string_at(item, "role") == Some("fallback"))
                    .collect::<Vec<_>>();
                fallbacks.sort_by_key(|item| item.get("fallback_priority").and_then(Value::as_u64).unwrap_or(u64::MAX));
                for fallback in fallbacks {
                    let id = string_at(fallback, "candidate_id").unwrap_or("");
                    if candidate_reports.get(id).is_some_and(|report| report.0) {
                        selected = Some((id.to_owned(), string_at(fallback, "action_id").unwrap_or("").to_owned(), "fallback".to_owned()));
                        break;
                    }
                }
            }
        }
    }

    let runtime_decision = if !errors.is_empty() {
        "INVALID_INPUT"
    } else if !state_fresh {
        "ABSTAIN_STALE_OR_REPLAYED_STATE"
    } else if selected.as_ref().is_some_and(|selection| selection.2 == "proposed") {
        "PERMIT_PROPOSED_ACTION"
    } else if selected.is_some() {
        "PERMIT_FALLBACK_ACTION"
    } else {
        "ABSTAIN_NO_SAFE_ACTION"
    };
    let permit_expiry = match (decision_time, valid_until) {
        (Some(now), Some(state_expiry)) if selected.is_some() => {
            let ttl = decision.get("permit_ttl_seconds").and_then(Value::as_i64).unwrap_or(0);
            if ttl <= 0 {
                global_blockers.push("permit TTL must be positive".to_owned());
                None
            } else {
                let ttl_expiry = now + Duration::seconds(ttl);
                Some(if ttl_expiry < state_expiry { ttl_expiry.to_rfc3339() } else { state_expiry.to_rfc3339() })
            }
        }
        _ => None,
    };
    let decision_body = json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "runtime_decision":runtime_decision,
        "selected_candidate_id":selected.as_ref().map(|item| item.0.as_str()),
        "selected_action_id":selected.as_ref().map(|item| item.1.as_str()),
        "selected_role":selected.as_ref().map(|item| item.2.as_str()),
        "state_sequence":state_sequence,
        "state_sha256":string_at(state,"state_sha256"),
        "action_digest_sha256":selected.as_ref().and_then(|selected| {
            candidates.and_then(|items| items.iter().find(|item| string_at(item,"candidate_id") == Some(selected.0.as_str())))
                .and_then(|item| string_at(item,"action_digest_sha256"))
        }),
        "mission_snapshot_sha256":string_at(snapshot,"mission_snapshot_sha256"),
        "permit_nonce_sha256":nonce,
        "permit_expires_at":permit_expiry,
        "authority_scope":"runtime_decision_receipt_only",
        "executor_authorization":"NONE",
        "single_use_nonce_status":"UNCONSUMED_AT_EVALUATION_EXTERNAL_ATOMIC_CONSUME_REQUIRED",
        "findings":{"global_blockers":global_blockers,"candidate_evaluations":candidate_reports.iter().map(|(id,(safe,reasons,action,priority))| json!({
            "candidate_id":id,"action_id":action,"priority":priority,"safe":safe,"reasons":reasons
        })).collect::<Vec<_>>()},
        "claim_ceiling":"read_only_next_action_admission_receipt_not_actuator_command"
    });
    let receipt_digest = canonical_digest(&decision_body, None).unwrap_or_else(|| sha256_hex(raw));
    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if runtime_decision.starts_with("ABSTAIN") {
        "ABSTAIN"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "ACTION_ADMISSION_AUDITED_READ_ONLY"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "runtime_decision":runtime_decision,
        "selected_candidate_id":selected.as_ref().map(|item| item.0.as_str()),
        "selected_action_id":selected.as_ref().map(|item| item.1.as_str()),
        "selected_role":selected.as_ref().map(|item| item.2.as_str()),
        "permit_nonce_sha256":nonce,
        "permit_expires_at":permit_expiry,
        "state_sequence":state_sequence,
        "state_sha256":string_at(state,"state_sha256"),
        "mission_snapshot_sha256":string_at(snapshot,"mission_snapshot_sha256"),
        "decision_receipt_sha256":receipt_digest,
        "executor_authorization":"NONE",
        "single_use_nonce_status":"UNCONSUMED_AT_EVALUATION_EXTERNAL_ATOMIC_CONSUME_REQUIRED",
        "candidate_evaluations":candidate_reports.iter().map(|(id,(safe,reasons,action,priority))| json!({
            "candidate_id":id,"action_id":action,"priority":priority,"safe":safe,"reasons":reasons
        })).collect::<Vec<_>>(),
        "findings":{"errors":errors,"blockers":global_blockers},
        "claim_ceiling":"one_step_robust_action_admission_receipt_not_actuator_authorization_not_recursive_viability_proof",
        "note":"The evaluator is read-only. A digest is not a signature or authentication. Single-use consumption must be enforced atomically by a separate executor/permit ledger. No actuator command is generated or issued."
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 || args[1] != "audit-action" {
        return Err("Usage: marine-runtime-action-shield audit-action <bundle.json>".to_owned());
    }
    let raw = fs::read(&args[2]).map_err(|error| format!("{}: {error}", args[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", args[2]))?;
    let report = validate_bundle(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "ACTION_ADMISSION_AUDITED_READ_ONLY" | "ABSTAIN") => Ok(()),
        Some(disposition) => Err(format!("runtime action shield disposition: {disposition}")),
        None => Err("runtime action shield report has no disposition".to_owned()),
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

    fn fixture(name: &str) -> Value {
        let raw = match name {
            "proposed" => include_str!("../fixtures/synthetic-proposed-action-permit-0001.json"),
            "fallback" => include_str!("../fixtures/synthetic-fallback-action-permit-0001.json"),
            "none" => include_str!("../fixtures/synthetic-no-safe-action-abstain-0001.json"),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("runtime shield fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate_bundle(&raw, value)
    }

    #[test]
    fn robust_proposed_action_is_admitted_across_all_five_disturbances() {
        let report = audit(&fixture("proposed"));
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["runtime_decision"], "PERMIT_PROPOSED_ACTION");
        assert_eq!(report["selected_candidate_id"], "candidate-continue-course");
        assert_eq!(report["executor_authorization"], "NONE");
    }

    #[test]
    fn unsafe_proposal_selects_first_safe_predeclared_fallback() {
        let report = audit(&fixture("fallback"));
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["runtime_decision"], "PERMIT_FALLBACK_ACTION");
        assert_eq!(report["selected_candidate_id"], "fallback-reduce-speed");
        assert_eq!(report["selected_role"], "fallback");
    }

    #[test]
    fn no_safe_proposal_or_fallback_abstains() {
        let report = audit(&fixture("none"));
        assert_eq!(report["disposition"], "ABSTAIN");
        assert_eq!(report["runtime_decision"], "ABSTAIN_NO_SAFE_ACTION");
        assert_eq!(report["selected_candidate_id"], Value::Null);
    }

    #[test]
    fn stale_current_state_forces_abstention() {
        let mut value = fixture("proposed");
        value["decision"]["decision_time"] = json!("2026-11-20T05:01:00Z");
        assert_eq!(audit(&value)["runtime_decision"], "ABSTAIN_STALE_OR_REPLAYED_STATE");
    }

    #[test]
    fn consumed_nonce_is_rejected_before_any_action_is_selected() {
        let mut value = fixture("proposed");
        let nonce = value["decision"]["permit_nonce_sha256"].clone();
        value["permit_registry"]["consumed_nonce_sha256"] = json!([nonce]);
        assert_eq!(audit(&value)["runtime_decision"], "ABSTAIN_STALE_OR_REPLAYED_STATE");
    }

    #[test]
    fn tampered_action_digest_invalidates_the_bundle() {
        let mut value = fixture("proposed");
        value["candidates"][0]["action_id"] = json!("different-action");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn wrong_model_or_operating_domain_blocks_that_candidate() {
        let mut value = fixture("proposed");
        value["candidates"][0]["model_sha256"] = json!("f".repeat(64));
        value["candidates"][0]["action_digest_sha256"] = json!("");
        let digest = canonical_digest(&value["candidates"][0], Some("action_digest_sha256")).unwrap();
        value["candidates"][0]["action_digest_sha256"] = json!(digest);
        let report = audit(&value);
        assert_eq!(report["runtime_decision"], "PERMIT_FALLBACK_ACTION");
        assert_eq!(report["selected_candidate_id"], "fallback-reduce-speed");
    }

    #[test]
    fn all_fallbacks_are_evaluated_in_explicit_priority_order() {
        let mut value = fixture("fallback");
        value["candidates"][1]["scenario_successors"][0]["recoverability"]["status"] = json!("unknown");
        value["candidates"][1]["scenario_successors"][0]["successor_state_sha256"] = json!("");
        let successor_digest = canonical_digest(&value["candidates"][1]["scenario_successors"][0], Some("successor_state_sha256")).unwrap();
        value["candidates"][1]["scenario_successors"][0]["successor_state_sha256"] = json!(successor_digest);
        let action_digest = canonical_digest(&value["candidates"][1], Some("action_digest_sha256")).unwrap();
        value["candidates"][1]["action_digest_sha256"] = json!(action_digest);
        assert_eq!(audit(&value)["selected_candidate_id"], "fallback-divert-refuge");
    }

    #[test]
    fn recoverability_margin_must_exceed_human_intervention_budget() {
        let mut value = fixture("proposed");
        for successor in value["candidates"][0]["scenario_successors"].as_array_mut().unwrap() {
            successor["recoverability"]["minimum_intervention_margin_seconds"] = json!(100);
            successor["successor_state_sha256"] = json!("");
            let digest = canonical_digest(successor, Some("successor_state_sha256")).unwrap();
            successor["successor_state_sha256"] = json!(digest);
        }
        value["candidates"][0]["action_digest_sha256"] = json!("");
        let digest = canonical_digest(&value["candidates"][0], Some("action_digest_sha256")).unwrap();
        value["candidates"][0]["action_digest_sha256"] = json!(digest);
        assert_eq!(audit(&value)["runtime_decision"], "PERMIT_FALLBACK_ACTION");
    }

    #[test]
    fn required_capability_must_be_available_in_current_state() {
        let mut value = fixture("proposed");
        value["candidates"][0]["required_capabilities"] = json!(["unavailable-thruster"]);
        value["candidates"][0]["action_digest_sha256"] = json!("");
        let digest = canonical_digest(&value["candidates"][0], Some("action_digest_sha256")).unwrap();
        value["candidates"][0]["action_digest_sha256"] = json!(digest);
        assert_eq!(audit(&value)["runtime_decision"], "PERMIT_FALLBACK_ACTION");
    }

    #[test]
    fn missing_disturbance_successor_invalidates_candidate_evidence() {
        let mut value = fixture("proposed");
        value["candidates"][0]["scenario_successors"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn hard_gate_failure_cannot_be_offset_by_resource_surplus() {
        let mut value = fixture("proposed");
        value["candidates"][0]["scenario_successors"][1]["hard_gate_checks"][0]["status"] = json!("fail");
        value["candidates"][0]["scenario_successors"][1]["successor_state_sha256"] = json!("");
        let sd = canonical_digest(&value["candidates"][0]["scenario_successors"][1], Some("successor_state_sha256")).unwrap();
        value["candidates"][0]["scenario_successors"][1]["successor_state_sha256"] = json!(sd);
        value["candidates"][0]["action_digest_sha256"] = json!("");
        let ad = canonical_digest(&value["candidates"][0], Some("action_digest_sha256")).unwrap();
        value["candidates"][0]["action_digest_sha256"] = json!(ad);
        assert_eq!(audit(&value)["runtime_decision"], "PERMIT_FALLBACK_ACTION");
    }

    #[test]
    fn current_state_service_floor_breach_blocks_all_actions() {
        let mut value = fixture("proposed");
        value["current_state"]["resource_metrics"][1]["lower_bound"] = json!(70.0);
        value["current_state"]["state_sha256"] = json!("");
        let sd = canonical_digest(&value["current_state"], Some("state_sha256")).unwrap();
        value["current_state"]["state_sha256"] = json!(sd);
        for candidate in value["candidates"].as_array_mut().unwrap() {
            candidate["precondition_state_sha256"] = json!(sd);
            candidate["action_digest_sha256"] = json!("");
            let ad = canonical_digest(candidate, Some("action_digest_sha256")).unwrap();
            candidate["action_digest_sha256"] = json!(ad);
        }
        assert_eq!(audit(&value)["runtime_decision"], "ABSTAIN_STALE_OR_REPLAYED_STATE");
    }
}
