use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-temporal-viability/0.1.0";
const REQUIRED_METRICS: [(&str, &str); 10] = [
    ("arrival_delay_hours", "h"),
    ("energy_reserve_hours", "h"),
    ("freshwater_reserve_days", "day"),
    ("food_reserve_days", "day"),
    ("waste_capacity_margin_days", "day"),
    ("critical_spares_coverage_days", "day"),
    ("essential_service_outage_minutes", "min"),
    ("comfort_breach_minutes", "min"),
    ("crew_workload_hours_per_day", "h/day"),
    ("purchased_energy_kwh_per_berth_day", "kWh/berth-day"),
];
const INVENTORY_METRICS: [&str; 5] = [
    "energy_reserve_hours",
    "freshwater_reserve_days",
    "food_reserve_days",
    "waste_capacity_margin_days",
    "critical_spares_coverage_days",
];
const CUMULATIVE_METRICS: [&str; 2] = [
    "essential_service_outage_minutes",
    "comfort_breach_minutes",
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

fn sha256_hex(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

#[derive(Clone, Copy)]
struct Floor {
    unit: String,
    operator: String,
    value: f64,
    minimum_coverage: f64,
}

fn canonical_contract(contract: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = contract.as_array()?;
    let mut rows = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let value = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        let (Some(value), Some(coverage)) = (value, coverage) else {
            errors.push(format!("service floor '{metric}' has missing/non-finite threshold or coverage"));
            continue;
        };
        if metric.is_empty()
            || unit.is_empty()
            || !seen.insert(metric.to_owned())
            || !["gte", "lte"].contains(&operator)
            || !(0.0..=1.0).contains(&coverage)
        {
            errors.push(format!("service floor '{metric}' has invalid or duplicate identity/semantics"));
            continue;
        }
        rows.push((
            metric.to_owned(),
            format!("{metric}|{unit}|{operator}|{value:.17}|{coverage:.17}"),
        ));
    }
    if rows.len() != REQUIRED_METRICS.len() {
        errors.push("service-floor contract must define all ten required trajectory metrics exactly once".to_owned());
    }
    for (metric, unit) in REQUIRED_METRICS {
        if !items.iter().any(|item| {
            string_at(item, "metric") == Some(metric)
                && string_at(item, "unit") == Some(unit)
        }) {
            errors.push(format!("service-floor contract is missing '{metric}' with unit '{unit}'"));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n"))
}

fn snapshot_digest(snapshot: &Value) -> Option<String> {
    let object = snapshot.as_object()?;
    let mut rows = object.iter()
        .filter(|(key, _)| key.as_str() != "mission_snapshot_sha256")
        .map(|(key, value)| {
            let value = value.as_str()?;
            Some((key.clone(), format!("{key}|{value}")))
        })
        .collect::<Option<Vec<_>>>()?;
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(sha256_hex(&rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n")))
}

fn scenario_member_set_digest(members: &BTreeMap<String, String>) -> String {
    let canonical = members.iter()
        .map(|(member_id, digest)| format!("{member_id}:{digest}"))
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&canonical)
}

fn floor_map(contract: &Value, errors: &mut Vec<String>) -> BTreeMap<String, Floor> {
    let mut floors = BTreeMap::<String, Floor>::new();
    let Some(items) = contract.as_array() else {
        errors.push("service_floor_contract must be an array".to_owned());
        return floors;
    };
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("").to_owned();
        let unit = string_at(item, "unit").unwrap_or("").to_owned();
        let operator = string_at(item, "operator").unwrap_or("").to_owned();
        let value = number_at(item, "floor_value");
        let minimum_coverage = number_at(item, "minimum_coverage_fraction");
        let (Some(value), Some(minimum_coverage)) = (value, minimum_coverage) else {
            continue;
        };
        if !REQUIRED_METRICS.contains(&(metric.as_str(), unit.as_str()))
            || !["gte", "lte"].contains(&operator.as_str())
            || !(0.0..=1.0).contains(&minimum_coverage)
        {
            errors.push(format!("service floor '{metric}' has unknown metric/unit or invalid operator/coverage"));
            continue;
        }
        if floors.insert(metric.clone(), Floor { unit, operator, value, minimum_coverage }).is_some() {
            errors.push(format!("service floor '{metric}' is duplicated"));
        }
    }
    floors
}

fn check_gate(
    record: &Value,
    context: &str,
    errors: &mut Vec<String>,
    blockers: &mut Vec<String>,
) {
    let status = string_at(record, "hard_gate_disposition").unwrap_or("");
    let violations = record.get("hard_gate_violations").and_then(Value::as_u64);
    if !["pass", "fail", "infeasible", "abstain"].contains(&status)
        || violations.is_none()
        || string_at(record, "hard_gate_evidence_ref").is_none_or(str::is_empty)
        || !digest_valid(string_at(record, "hard_gate_evidence_sha256"))
    {
        errors.push(format!("{context} has missing/invalid hard-gate disposition or receipt"));
        return;
    }
    if status != "pass" || violations != Some(0) {
        blockers.push(format!("{context} has hard-gate disposition '{status}' with {} violation(s)", violations.unwrap_or(0)));
    }
}

fn metric_values(
    record: &Value,
    floors: &BTreeMap<String, Floor>,
    expected_class: &str,
    context: &str,
    require_estimate: bool,
    errors: &mut Vec<String>,
) -> BTreeMap<String, (f64, f64, f64)> {
    let mut result = BTreeMap::<String, (f64, f64, f64)>::new();
    let Some(items) = record.get("metrics").and_then(Value::as_array) else {
        errors.push(format!("{context} has no metrics array"));
        return result;
    };
    if items.len() != floors.len() {
        errors.push(format!("{context} must contain all {} service axes", floors.len()));
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("").to_owned();
        let Some(floor) = floors.get(&metric) else {
            errors.push(format!("{context} contains unknown service metric '{metric}'"));
            continue;
        };
        if string_at(item, "unit") != Some(floor.unit.as_str())
            || !digest_valid(string_at(item, "evidence_sha256"))
        {
            errors.push(format!("{context} metric '{metric}' has wrong unit or invalid evidence digest"));
            continue;
        }
        if string_at(item, "measurement_class") != Some(expected_class)
            || string_at(item, "measurement_method").is_none_or(str::is_empty)
        {
            errors.push(format!("{context} metric '{metric}' has an invalid measurement class or method"));
            continue;
        }
        let coverage = number_at(item, "coverage_fraction");
        if coverage.is_none_or(|coverage| !(0.0..=1.0).contains(&coverage)) {
            errors.push(format!("{context} metric '{metric}' has invalid coverage"));
            continue;
        }
        if coverage.unwrap() < floor.minimum_coverage {
            // Coverage is a conservative viability blocker, not a missing JSON field.
            result.insert(metric, (f64::NAN, f64::NAN, f64::NAN));
            continue;
        }
        let lower = number_at(item, "lower_bound");
        let upper = number_at(item, "upper_bound");
        let estimate = number_at(item, "value");
        let Some((lower, upper)) = lower.zip(upper) else {
            errors.push(format!("{context} metric '{metric}' has non-finite bounds"));
            continue;
        };
        if lower < 0.0 || lower > upper {
            errors.push(format!("{context} metric '{metric}' has invalid conservative bounds"));
            continue;
        }
        if require_estimate {
            let Some(estimate) = estimate else {
                errors.push(format!("{context} point metric '{metric}' has no finite value"));
                continue;
            };
            if lower > estimate || estimate > upper {
                errors.push(format!("{context} metric '{metric}' interval does not contain its point estimate"));
                continue;
            }
            result.insert(metric, (estimate, lower, upper));
        } else {
            if estimate.is_some() {
                errors.push(format!("{context} segment metric '{metric}' must express an interval envelope, not a point estimate"));
                continue;
            }
            result.insert(metric, (f64::NAN, lower, upper));
        }
    }
    for metric in floors.keys() {
        if !result.contains_key(metric) {
            errors.push(format!("{context} is missing metric '{metric}'"));
        }
    }
    result
}

fn apply_floors(
    metrics: &BTreeMap<String, (f64, f64, f64)>,
    floors: &BTreeMap<String, Floor>,
    context: &str,
    blockers: &mut Vec<String>,
) {
    for (metric, floor) in floors {
        let Some((_, lower, upper)) = metrics.get(metric) else {
            continue;
        };
        if !lower.is_finite() || !upper.is_finite() {
            blockers.push(format!("{context} metric '{metric}' has insufficient measurement coverage"));
            continue;
        }
        let passes = match floor.operator.as_str() {
            "gte" => *lower >= floor.value,
            "lte" => *upper <= floor.value,
            _ => false,
        };
        if !passes {
            blockers.push(format!("{context} metric '{metric}' breaches floor {} {} using conservative interval [{lower}, {upper}]",
                floor.operator, floor.value));
        }
    }
}

fn inventory_event_unit(metric: &str) -> Option<&'static str> {
    match metric {
        "energy_reserve_hours" => Some("h"),
        "freshwater_reserve_days" | "food_reserve_days" | "waste_capacity_margin_days" | "critical_spares_coverage_days" => Some("day"),
        _ => None,
    }
}

fn validate_path(
    path: &Value,
    policy_id: &str,
    member_digests: &BTreeMap<String, String>,
    floors: &BTreeMap<String, Floor>,
    mission_start: DateTime<FixedOffset>,
    mission_end: DateTime<FixedOffset>,
    max_gap_hours: f64,
    record_status: &str,
    expected_time_grid: &mut Option<Vec<String>>,
    errors: &mut Vec<String>,
    blockers: &mut Vec<String>,
) -> bool {
    let member_id = string_at(path, "scenario_member_id").unwrap_or("");
    let context = format!("policy '{policy_id}' scenario '{member_id}'");
    let expected_member_digest = member_digests.get(member_id).map(String::as_str);
    if member_id.is_empty()
        || expected_member_digest != string_at(path, "source_member_sha256")
        || !digest_valid(string_at(path, "path_result_sha256"))
    {
        errors.push(format!("{context} is not bound to a declared scenario member/path digest"));
    }

    let events = path.get("events").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut event_ids = BTreeSet::<String>::new();
    let mut parsed_events = Vec::<(DateTime<FixedOffset>, String, f64, String)>::new();
    for event in &events {
        let event_id = string_at(event, "event_id").unwrap_or("").to_owned();
        let event_time = parse_time(string_at(event, "event_time"));
        let metric = string_at(event, "resource_metric").unwrap_or("").to_owned();
        let delta = number_at(event, "delta");
        let unit = string_at(event, "unit").unwrap_or("").to_owned();
        let event_type = string_at(event, "event_type").unwrap_or("");
        if event_id.is_empty()
            || !event_ids.insert(event_id.clone())
            || event_time.is_none()
            || !floors.contains_key(&metric)
            || !INVENTORY_METRICS.contains(&metric.as_str())
            || delta.is_none()
            || unit != inventory_event_unit(&metric).unwrap_or("")
            || string_at(event, "evidence_ref").is_none_or(str::is_empty)
            || !digest_valid(string_at(event, "evidence_sha256"))
            || !matches!(
                (metric.as_str(), event_type),
                ("energy_reserve_hours", "energy_recharge" | "energy_generation" | "energy_consumption" | "energy_loss")
                    | ("freshwater_reserve_days", "freshwater_replenishment" | "freshwater_consumption" | "freshwater_leakage")
                    | ("food_reserve_days", "food_replenishment" | "food_consumption" | "food_spoilage")
                    | ("waste_capacity_margin_days", "waste_service" | "waste_generation")
                    | ("critical_spares_coverage_days", "spares_restock" | "spares_consumed" | "repair_recovery")
            )
        {
            errors.push(format!("{context} has invalid resource event '{event_id}'"));
            continue;
        }
        let time = event_time.unwrap();
        let delta_value = delta.unwrap();
        let event_is_positive = ["energy_recharge", "energy_generation", "freshwater_replenishment",
            "food_replenishment", "waste_service", "spares_restock", "repair_recovery"]
            .contains(&event_type);
        let event_is_negative = ["energy_consumption", "energy_loss", "freshwater_consumption",
            "freshwater_leakage", "food_consumption", "food_spoilage", "waste_generation",
            "spares_consumed"].contains(&event_type);
        if (event_is_positive && delta_value <= 0.0) || (event_is_negative && delta_value >= 0.0) {
            errors.push(format!("{context} resource event '{event_id}' has a delta sign inconsistent with its event type"));
        }
        if time <= mission_start || time > mission_end {
            errors.push(format!("{context} resource event '{event_id}' lies outside the mission interval"));
        } else {
            parsed_events.push((time, metric, delta_value, unit));
        }
    }

    let timepoints = path.get("timepoints").and_then(Value::as_array);
    let segments = path.get("segments").and_then(Value::as_array);
    let (Some(timepoints), Some(segments)) = (timepoints, segments) else {
        errors.push(format!("{context} requires timepoints and segments"));
        return false;
    };
    if timepoints.len() < 3 || segments.len() + 1 != timepoints.len() {
        errors.push(format!("{context} needs at least three timepoints and exactly one segment between adjacent points"));
        return false;
    }

    let mut point_times = Vec::<DateTime<FixedOffset>>::new();
    let mut point_time_strings = Vec::<String>::new();
    let mut point_metric_values = Vec::<BTreeMap<String, (f64, f64, f64)>>::new();
    let mut route_progress = Vec::<f64>::new();

    for (index, point) in timepoints.iter().enumerate() {
        let point_context = format!("{context} point[{index}]");
        check_gate(point, &point_context, errors, blockers);
        let time = parse_time(string_at(point, "at"));
        if time.is_none() {
            errors.push(format!("{point_context} timestamp is not RFC3339"));
            continue;
        }
        let time = time.unwrap();
        point_times.push(time);
        point_time_strings.push(string_at(point, "at").unwrap_or("").to_owned());
        if index == 0 && time != mission_start {
            errors.push(format!("{point_context} does not start at mission_window.start"));
        }
        if index + 1 == timepoints.len() && time != mission_end {
            errors.push(format!("{point_context} does not end at mission_window.end"));
        }
        if let Some(previous) = point_times.get(index.wrapping_sub(1)) {
            if time <= *previous {
                errors.push(format!("{point_context} timestamp is not strictly increasing"));
            }
            let gap = time.signed_duration_since(*previous).num_milliseconds() as f64 / 3_600_000.0;
            if gap > max_gap_hours {
                errors.push(format!("{point_context} sample gap {gap:.3}h exceeds maximum {max_gap_hours:.3}h"));
            }
        }
        let elapsed = number_at(point, "elapsed_hours");
        let expected_elapsed = time.signed_duration_since(mission_start).num_milliseconds() as f64 / 3_600_000.0;
        if elapsed.is_none_or(|value| (value - expected_elapsed).abs() > 1e-6) {
            errors.push(format!("{point_context} elapsed_hours disagrees with UTC timestamp"));
        }
        let progress = number_at(point, "route_progress_fraction");
        if progress.is_none_or(|progress| !(0.0..=1.0).contains(&progress)) {
            errors.push(format!("{point_context} has invalid route progress"));
            route_progress.push(f64::NAN);
        } else {
            route_progress.push(progress.unwrap());
        }
        let class = if record_status == "synthetic" { "synthetic" } else { "measured" };
        let metrics = metric_values(point, floors, class, &point_context, true, errors);
        if let Some(items) = point.get("metrics").and_then(Value::as_array) {
            for item in items {
                if parse_time(string_at(item, "measured_at")) != Some(time) {
                    errors.push(format!("{point_context} metric '{}' measurement timestamp does not match the trajectory point",
                        string_at(item, "metric").unwrap_or("unknown")));
                }
            }
        }
        apply_floors(&metrics, floors, &point_context, blockers);
        point_metric_values.push(metrics);
    }

    if point_times.len() != timepoints.len() {
        errors.push(format!("{context} contains invalid or missing timestamps"));
        return false;
    }
    if route_progress.first().is_none_or(|value| value.abs() > 1e-9)
        || route_progress.last().is_none_or(|value| (value - 1.0).abs() > 1e-9)
        || route_progress.windows(2).any(|window| window[1] < window[0])
    {
        errors.push(format!("{context} route progress must be monotone from 0.0 to 1.0"));
    }
    let grid = point_time_strings.clone();
    if let Some(expected) = expected_time_grid.as_ref() {
        if *expected != grid {
            errors.push(format!("{context} does not use the exact shared scenario time grid"));
        }
    } else {
        *expected_time_grid = Some(grid);
    }

    for metric in CUMULATIVE_METRICS {
        for pair in point_metric_values.windows(2) {
            let previous = pair[0].get(metric).map(|tuple| tuple.0);
            let current = pair[1].get(metric).map(|tuple| tuple.0);
            if let (Some(previous), Some(current)) = (previous, current) {
                if current + 1e-9 < previous {
                    errors.push(format!("{context} cumulative metric '{metric}' decreases; recovery cannot erase prior outage/exposure"));
                }
            }
        }
    }

    for (index, segment) in segments.iter().enumerate() {
        let segment_context = format!("{context} segment[{index}]");
        check_gate(segment, &segment_context, errors, blockers);
        let from = string_at(segment, "from");
        let to = string_at(segment, "to");
        if from != point_time_strings.get(index).map(String::as_str)
            || to != point_time_strings.get(index + 1).map(String::as_str)
        {
            errors.push(format!("{segment_context} does not span its exact adjacent timepoints"));
        }
        let class = if record_status == "synthetic" { "synthetic" } else { "measured" };
        let metrics = metric_values(segment, floors, class, &segment_context, false, errors);
        apply_floors(&metrics, floors, &segment_context, blockers);
        let start_values = point_metric_values.get(index);
        let end_values = point_metric_values.get(index + 1);
        for inventory_metric in INVENTORY_METRICS {
            let Some((start, _, _)) = start_values.and_then(|values| values.get(inventory_metric)) else {
                continue;
            };
            let Some((end, _, _)) = end_values.and_then(|values| values.get(inventory_metric)) else {
                continue;
            };
            let delta = end - start;
            let event_delta = parsed_events.iter()
                .filter(|(event_time, metric, _, _)| {
                    metric == inventory_metric
                        && *event_time > point_times[index]
                        && *event_time <= point_times[index + 1]
                })
                .map(|(_, _, value, _)| value)
                .sum::<f64>();
            if (delta - event_delta).abs() > 1e-6 {
                errors.push(format!("{segment_context} inventory change for '{inventory_metric}' is {delta:.6} but evidence events sum to {event_delta:.6}"));
            }
        }
    }
    blockers.iter().all(|blocker| !blocker.starts_with(&context))
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut blockers = Vec::<String>::new();
    if string_at(value, "schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value, "scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value, "claim_ceiling") != Some("trajectory_viability_audit_only") {
        errors.push("claim_ceiling must be trajectory_viability_audit_only".to_owned());
    }
    let status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let window = value.get("mission_window").unwrap_or(&Value::Null);
    let start = parse_time(string_at(window, "start"));
    let end = parse_time(string_at(window, "end"));
    let max_gap = number_at(window, "max_sample_gap_hours");
    let (Some(mission_start), Some(mission_end), Some(max_gap_hours)) = (start, end, max_gap) else {
        errors.push("mission window requires RFC3339 start/end and positive max_sample_gap_hours".to_owned());
        return json!({"disposition":"INVALID","findings":{"errors":errors,"blockers":blockers}});
    };
    if mission_end <= mission_start || max_gap_hours <= 0.0 {
        errors.push("mission window end must follow start and max sample gap must be positive".to_owned());
    }

    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    let snapshot_hash = snapshot_digest(snapshot);
    if snapshot_hash.as_deref() != string_at(snapshot, "mission_snapshot_sha256")
        || !digest_valid(snapshot_hash.as_deref())
    {
        errors.push("mission_snapshot_sha256 does not match canonical mission snapshot fields".to_owned());
    }
    for field in [
        "craft_configuration_sha256", "occupancy_sha256", "initial_resource_state_sha256",
        "mission_plan_sha256", "forecast_information_set_sha256", "hard_gate_roster_sha256",
        "operating_envelope_sha256", "service_floor_contract_sha256", "scenario_member_set_sha256",
    ] {
        if !digest_valid(string_at(snapshot, field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }

    let contract = value.get("service_floor_contract").unwrap_or(&Value::Null);
    if let Some(canonical) = canonical_contract(contract, &mut errors) {
        if string_at(snapshot, "service_floor_contract_sha256") != Some(sha256_hex(&canonical).as_str()) {
            errors.push("service_floor_contract_sha256 does not bind the canonical service floor contract".to_owned());
        }
    }
    let floors = floor_map(contract, &mut errors);

    let mut member_digests = BTreeMap::<String, String>::new();
    if let Some(members) = value.get("scenario_members").and_then(Value::as_array) {
        for member in members {
            let id = string_at(member, "scenario_member_id").unwrap_or("").to_owned();
            let digest = string_at(member, "source_member_sha256").unwrap_or("").to_owned();
            if id.is_empty() || !digest_valid(Some(&digest)) || member_digests.insert(id.clone(), digest).is_some() {
                errors.push(format!("scenario member '{id}' has a missing/duplicate ID or invalid digest"));
            }
        }
    } else {
        errors.push("scenario_members must be an array".to_owned());
    }
    if member_digests.len() < 2 {
        errors.push("at least two declared scenario members are required".to_owned());
    }
    if string_at(snapshot, "scenario_member_set_sha256") != Some(scenario_member_set_digest(&member_digests).as_str()) {
        errors.push("scenario_member_set_sha256 does not bind the canonical sorted scenario member IDs/digests".to_owned());
    }

    let policies = value.get("policies").and_then(Value::as_array);
    if policies.is_none_or(|items| items.len() < 3) {
        errors.push("at least three matched policy arms are required".to_owned());
    }
    let mut roles = BTreeSet::<String>::new();
    let mut policy_ids = BTreeSet::<String>::new();
    let mut candidate_viable = true;
    let mut policy_reports = Vec::<Value>::new();
    let mut arrival_only_trap_count = 0_usize;
    let mut mission_time_grid: Option<Vec<String>> = None;
    if let Some(policies) = policies {
        for policy in policies {
            let policy_id = string_at(policy, "policy_id").unwrap_or("").to_owned();
            let role = string_at(policy, "role").unwrap_or("").to_owned();
            if policy_id.is_empty() || !policy_ids.insert(policy_id.clone()) || !roles.insert(role.clone()) {
                errors.push(format!("policy '{policy_id}' has a missing/duplicate ID or duplicate role"));
            }
            let expected_forecast = match role.as_str() {
                "weather_blind_reference" => Some(false),
                "weather_informed_resource_blind" | "integrated_candidate" => Some(true),
                _ => None,
            };
            if expected_forecast.is_none()
                || policy.get("forecast_use").and_then(Value::as_bool) != expected_forecast
            {
                errors.push(format!("policy '{policy_id}' forecast-use flag disagrees with its role"));
            }
            if string_at(policy, "mission_snapshot_sha256") != string_at(snapshot, "mission_snapshot_sha256") {
                errors.push(format!("policy '{policy_id}' does not share exact frozen mission snapshot"));
            }
            let paths = policy.get("paths").and_then(Value::as_array);
            if paths.is_none_or(|items| items.len() != member_digests.len()) {
                errors.push(format!("policy '{policy_id}' must contain one trajectory per declared scenario member"));
                continue;
            }
            let mut path_members = BTreeSet::<String>::new();
            let mut policy_viable = true;
            for path in paths {
                let member_id = string_at(path, "scenario_member_id").unwrap_or("").to_owned();
                if !path_members.insert(member_id.clone()) {
                    errors.push(format!("policy '{policy_id}' duplicates scenario member '{member_id}'"));
                }
                let path_viable = validate_path(
                    path, &policy_id, &member_digests, &floors,
                    mission_start, mission_end, max_gap_hours, status,
                    &mut mission_time_grid, &mut errors, &mut blockers,
                );
                policy_viable &= path_viable;
                let times = path.get("timepoints").and_then(Value::as_array).cloned().unwrap_or_default();
                let arrival_passes = times.last().is_some_and(|point| {
                    let metrics = metric_values(
                        point, &floors,
                        if status == "synthetic" { "synthetic" } else { "measured" },
                        "arrival-only diagnostic", true, &mut Vec::new(),
                    );
                    floors.iter().all(|(metric, floor)| {
                        metrics.get(metric).is_some_and(|(_, lower, upper)| {
                            if floor.operator == "gte" { *lower >= floor.value } else { *upper <= floor.value }
                        })
                    })
                });
                if arrival_passes && !path_viable {
                    arrival_only_trap_count += 1;
                }
            }
            if path_members != member_digests.keys().cloned().collect::<BTreeSet<_>>() {
                errors.push(format!("policy '{policy_id}' does not preserve the exact declared scenario member set"));
            }
            if role == "integrated_candidate" && !policy_viable {
                candidate_viable = false;
            }
            policy_reports.push(json!({
                "policy_id":policy_id,
                "role":role,
                "trajectory_count":path_members.len(),
                "viable_across_all_scenarios":policy_viable
            }));
        }
    }
    for required in ["weather_blind_reference", "weather_informed_resource_blind", "integrated_candidate"] {
        if !roles.contains(required) {
            errors.push(format!("required policy role '{required}' is missing"));
        }
    }

    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if !candidate_viable {
        "BLOCKED"
    } else if status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "TEMPORAL_VIABILITY_AUDITED_READ_ONLY"
    };
    let input_digest = Sha256::digest(raw).iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "input_sha256":input_digest,
        "policy_count":policy_reports.len(),
        "scenario_member_count":member_digests.len(),
        "arrival_only_false_feasible_path_count":arrival_only_trap_count,
        "candidate_viable_across_all_scenarios":candidate_viable,
        "policies":policy_reports,
        "findings":{"errors":errors,"blockers":blockers},
        "claim_ceiling":"time_indexed_service_floor_and_hard_gate_audit_only_not_navigation_authority"
    })
}

fn run() -> Result<(), String> {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 || arguments[1] != "audit-trajectory" {
        return Err("Usage: marine-temporal-viability audit-trajectory <fixture.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "TEMPORAL_VIABILITY_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("trajectory audit disposition: {disposition}")),
        None => Err("trajectory audit report has no disposition".to_owned()),
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
        serde_json::from_str(include_str!("../fixtures/synthetic-temporal-viability-good-0001.json"))
            .expect("positive synthetic temporal fixture parses")
    }

    fn arrival_trap() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-arrival-only-trap-0001.json"))
            .expect("arrival-only adversarial fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    #[test]
    fn positive_temporal_fixture_is_synthetic_only() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["policy_count"], 3);
        assert_eq!(report["scenario_member_count"], 2);
        assert_eq!(report["candidate_viable_across_all_scenarios"], true);
        assert_eq!(report["arrival_only_false_feasible_path_count"], 0);
    }

    #[test]
    fn arrival_only_trap_is_blocked_after_later_replenishment() {
        let value = arrival_trap();
        let candidate_path = &value["policies"][2]["paths"][0];
        assert_eq!(candidate_path["timepoints"][2]["metrics"][2]["value"], 5.0);
        assert_eq!(candidate_path["timepoints"][1]["metrics"][2]["value"], 2.0);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn arrival_only_pass_does_not_hide_inter_sample_breach() {
        let mut value = fixture();
        value["policies"][2]["paths"][0]["segments"][0]["metrics"][2]["lower_bound"] = json!(2.0);
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        assert_eq!(report["arrival_only_false_feasible_path_count"], 1);
    }

    #[test]
    fn cumulative_service_outage_cannot_decrease_after_recovery() {
        let mut value = fixture();
        let metrics = value["policies"][2]["paths"][0]["timepoints"][2]["metrics"].as_array_mut().unwrap();
        let outage = metrics.iter_mut().find(|metric| metric["metric"] == "essential_service_outage_minutes").unwrap();
        outage["value"] = json!(1.0);
        outage["lower_bound"] = json!(0.9);
        outage["upper_bound"] = json!(1.1);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn workload_peak_inside_a_day_is_not_hidden_by_daily_average() {
        let mut value = fixture();
        let metrics = value["policies"][2]["paths"][0]["timepoints"][1]["metrics"].as_array_mut().unwrap();
        let workload = metrics.iter_mut().find(|metric| metric["metric"] == "crew_workload_hours_per_day").unwrap();
        workload["value"] = json!(10.0);
        workload["lower_bound"] = json!(9.9);
        workload["upper_bound"] = json!(10.1);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn stale_sample_gap_is_invalid() {
        let mut value = fixture();
        value["mission_window"]["max_sample_gap_hours"] = json!(3.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn inventory_replenishment_without_evidence_event_is_invalid() {
        let mut value = arrival_trap();
        value["policies"][2]["paths"][0]["events"] = json!([]);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn resource_event_sign_must_match_event_type() {
        let mut value = arrival_trap();
        value["policies"][2]["paths"][0]["events"][0]["delta"] = json!(3.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn inventory_event_delta_must_reconcile_with_state_change() {
        let mut value = arrival_trap();
        value["policies"][2]["paths"][0]["events"][0]["delta"] = json!(1.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn scenario_member_identity_cannot_change_within_path() {
        let mut value = fixture();
        value["policies"][2]["paths"][0]["source_member_sha256"] = json!("f".repeat(64));
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn weather_blind_policy_cannot_consume_forecast() {
        let mut value = fixture();
        value["policies"][0]["forecast_use"] = json!(true);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn segment_must_cover_exact_adjacent_timepoints() {
        let mut value = fixture();
        value["policies"][2]["paths"][0]["segments"][0]["to"] = json!("2025-03-01T07:00:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn hard_gate_failure_blocks_candidate() {
        let mut value = fixture();
        value["policies"][2]["paths"][0]["timepoints"][1]["hard_gate_disposition"] = json!("fail");
        value["policies"][2]["paths"][0]["timepoints"][1]["hard_gate_violations"] = json!(1);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn scenario_time_grid_must_match_across_member_paths() {
        let mut value = fixture();
        value["policies"][2]["paths"][1]["timepoints"][1]["at"] = json!("2025-03-01T07:00:00Z");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
