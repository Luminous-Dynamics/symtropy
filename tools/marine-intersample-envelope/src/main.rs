use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-intersample-envelope/0.1.0";
const SCENARIO_CLASSES: [&str; 5] = [
    "adverse_forecast_error",
    "common_cause_generation_watermaker_outage",
    "main_generation_outage",
    "nominal",
    "watermaker_outage",
];
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

#[derive(Clone)]
struct ContractMetric {
    metric: String,
    unit: String,
    operator: String,
    floor: f64,
    minimum_coverage: f64,
    max_abs_rate_per_hour: f64,
    rate_bound_method: String,
    rate_bound_class: String,
    rate_bound_evidence_sha256: String,
}

#[derive(Clone)]
struct SampleMetric {
    value: f64,
    lower: f64,
    upper: f64,
    coverage: f64,
    unit: String,
    evidence_sha256: String,
    measurement_class: String,
    measurement_method: String,
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
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
}

fn canonical_json_digest(value: &Value, omitted_field: Option<&str>) -> Option<String> {
    let mut canonical = value.clone();
    if let Some(field) = omitted_field {
        canonical.as_object_mut()?.remove(field);
    }
    let bytes = serde_json::to_vec(&canonical).ok()?;
    Some(sha256_hex(&bytes))
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

fn duration_hours(start: &DateTime<FixedOffset>, end: &DateTime<FixedOffset>) -> Option<f64> {
    let millis = end.signed_duration_since(start.clone()).num_milliseconds();
    if millis <= 0 {
        None
    } else {
        Some(millis as f64 / 3_600_000.0)
    }
}

fn contract_canonical(contract: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = contract.as_array()?;
    if items.len() != METRICS.len() {
        errors.push("service contract must define exactly eleven axes".to_owned());
    }
    let mut rows = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let (Some(floor), Some(coverage), Some(rate)) = (
            number_at(item, "floor_value"),
            number_at(item, "minimum_coverage_fraction"),
            number_at(item, "max_abs_rate_per_hour"),
        ) else {
            errors.push(format!("contract axis '{metric}' has a missing or non-finite floor/rate"));
            continue;
        };
        let method = string_at(item, "rate_bound_method").unwrap_or("");
        let class = string_at(item, "rate_bound_class").unwrap_or("");
        let rate_digest = string_at(item, "rate_bound_evidence_sha256").unwrap_or("");
        if metric.is_empty() || !seen.insert(metric.to_owned()) {
            errors.push(format!("service contract has a missing or duplicated axis '{metric}'"));
            continue;
        }
        let Some((_, expected_unit, expected_operator)) =
            METRICS.iter().find(|(name, _, _)| *name == metric)
        else {
            errors.push(format!("service contract contains unknown metric '{metric}'"));
            continue;
        };
        if unit != *expected_unit
            || operator != *expected_operator
            || !(0.0..=1.0).contains(&coverage)
            || rate < 0.0
            || method.is_empty()
            || !["measured", "modeled", "derived", "synthetic"].contains(&class)
            || !digest_valid(Some(rate_digest))
        {
            errors.push(format!("contract axis '{metric}' has invalid units, operator, coverage or rate-bound evidence"));
        }
        rows.push((
            metric.to_owned(),
            format!(
                "{metric}|{unit}|{operator}|{floor:.17}|{coverage:.17}|{rate:.17}|{method}|{class}|{rate_digest}"
            ),
        ));
    }
    for (metric, _, _) in METRICS {
        if !seen.contains(metric) {
            errors.push(format!("service contract is missing '{metric}'"));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n"))
}

fn contract_map(contract: &Value) -> BTreeMap<String, ContractMetric> {
    let mut result = BTreeMap::new();
    if let Some(items) = contract.as_array() {
        for item in items {
            let (Some(metric), Some(unit), Some(operator), Some(floor), Some(minimum_coverage),
                Some(max_abs_rate_per_hour), Some(rate_bound_method), Some(rate_bound_class),
                Some(rate_bound_evidence_sha256)) = (
                string_at(item, "metric"), string_at(item, "unit"), string_at(item, "operator"),
                number_at(item, "floor_value"), number_at(item, "minimum_coverage_fraction"),
                number_at(item, "max_abs_rate_per_hour"), string_at(item, "rate_bound_method"),
                string_at(item, "rate_bound_class"), string_at(item, "rate_bound_evidence_sha256"),
            ) else {
                continue;
            };
            result.insert(metric.to_owned(), ContractMetric {
                metric: metric.to_owned(),
                unit: unit.to_owned(),
                operator: operator.to_owned(),
                floor,
                minimum_coverage,
                max_abs_rate_per_hour,
                rate_bound_method: rate_bound_method.to_owned(),
                rate_bound_class: rate_bound_class.to_owned(),
                rate_bound_evidence_sha256: rate_bound_evidence_sha256.to_owned(),
            });
        }
    }
    result
}

fn sample_metrics(
    sample: &Value,
    scenario_id: &str,
    time_index: usize,
    record_status: &str,
    contract: &BTreeMap<String, ContractMetric>,
    errors: &mut Vec<String>,
    blockers: &mut Vec<String>,
) -> BTreeMap<String, SampleMetric> {
    let label = format!("scenario '{scenario_id}' sample {time_index}");
    let mut result = BTreeMap::new();
    let Some(items) = sample.get("resource_metrics").and_then(Value::as_array) else {
        errors.push(format!("{label} has no resource_metrics array"));
        return result;
    };
    if items.len() != METRICS.len() {
        errors.push(format!("{label} must contain exactly eleven resource axes"));
    }
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let Some(spec) = contract.get(metric) else {
            errors.push(format!("{label} reports unknown or uncontracted metric '{metric}'"));
            continue;
        };
        let (Some(value), Some(lower), Some(upper), Some(coverage)) = (
            number_at(item, "value"),
            number_at(item, "lower_bound"),
            number_at(item, "upper_bound"),
            number_at(item, "coverage_fraction"),
        ) else {
            errors.push(format!("{label} metric '{metric}' has missing/non-finite estimates"));
            continue;
        };
        let measurement_class = string_at(item, "measurement_class").unwrap_or("");
        let measurement_method = string_at(item, "measurement_method").unwrap_or("");
        let evidence_sha256 = string_at(item, "evidence_sha256").unwrap_or("");
        if unit != spec.unit || !digest_valid(Some(evidence_sha256)) {
            errors.push(format!("{label} metric '{metric}' has an invalid unit or evidence digest"));
        }
        if lower < 0.0
            || lower > value
            || value > upper
            || !(0.0..=1.0).contains(&coverage)
            || measurement_method.is_empty()
            || !["measured", "modeled", "derived", "synthetic"].contains(&measurement_class)
            || (record_status == "synthetic" && measurement_class != "synthetic")
            || (record_status != "synthetic" && measurement_class == "synthetic")
        {
            errors.push(format!("{label} metric '{metric}' has invalid bounds, coverage, method or measurement class"));
            continue;
        }
        if coverage < spec.minimum_coverage {
            blockers.push(format!(
                "{label} metric '{metric}' coverage {:.3} is below the contract minimum {:.3}",
                coverage, spec.minimum_coverage
            ));
        }
        if result.insert(metric.to_owned(), SampleMetric {
            value,
            lower,
            upper,
            coverage,
            unit: unit.to_owned(),
            evidence_sha256: evidence_sha256.to_owned(),
            measurement_class: measurement_class.to_owned(),
            measurement_method: measurement_method.to_owned(),
        }).is_some() {
            errors.push(format!("{label} duplicates metric '{metric}'"));
        }
    }
    for (metric, _, _) in METRICS {
        if !result.contains_key(metric) {
            errors.push(format!("{label} is missing metric '{metric}'"));
        }
    }
    result
}

fn initial_state_digest(metrics: &BTreeMap<String, SampleMetric>) -> String {
    let canonical = metrics.iter().map(|(metric, item)| format!(
        "{metric}|{}|{:.17}|{:.17}|{:.17}",
        item.unit, item.value, item.lower, item.upper
    )).collect::<Vec<_>>().join("\n");
    sha256_hex(canonical.as_bytes())
}

fn endpoint_gap(start: &SampleMetric, end: &SampleMetric) -> f64 {
    (start.lower - end.upper).max(end.lower - start.upper).max(0.0)
}

// The minimum of max(a - L*t, b - L*(dt-t)) over t in [0,dt].
fn exact_lower_tube_min(a: f64, b: f64, rate: f64, dt: f64) -> f64 {
    let span = rate * dt;
    if a > b + span {
        a - span
    } else if b > a + span {
        b - span
    } else {
        (a + b - span) / 2.0
    }
}

// The maximum of min(a + L*t, b + L*(dt-t)) over t in [0,dt].
fn exact_upper_tube_max(a: f64, b: f64, rate: f64, dt: f64) -> f64 {
    let span = rate * dt;
    if a > b + span {
        b + span
    } else if b > a + span {
        a + span
    } else {
        (a + b + span) / 2.0
    }
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
    if string_at(value, "claim_ceiling") != Some("declared_rate_bounded_inter_sample_envelope_audit_only") {
        errors.push("claim_ceiling must preserve the declared-rate inter-sample-only boundary".to_owned());
    }
    let record_status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let contract = value.get("service_contract").unwrap_or(&Value::Null);
    let contract_canonical = contract_canonical(contract, &mut errors);
    let contract_by_metric = contract_map(contract);
    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    if let Some(canonical) = contract_canonical {
        let calculated = sha256_hex(canonical.as_bytes());
        if string_at(snapshot, "service_contract_sha256") != Some(calculated.as_str()) {
            errors.push("service_contract_sha256 does not match the canonical rate/floor contract".to_owned());
        }
    }
    for spec in contract_by_metric.values() {
        if record_status == "synthetic" && spec.rate_bound_class != "synthetic" {
            errors.push(format!("synthetic record cannot promote rate-bound class '{}' for metric '{}'", spec.rate_bound_class, spec.metric));
        }
        if record_status != "synthetic" && spec.rate_bound_class == "synthetic" {
            errors.push(format!("non-synthetic record cannot use a synthetic rate bound for metric '{}'", spec.metric));
        }
        if spec.rate_bound_method.is_empty() || !digest_valid(Some(&spec.rate_bound_evidence_sha256)) {
            errors.push(format!("metric '{}' rate bound lacks method/evidence identity", spec.metric));
        }
    }

    let time_grid = value.get("time_grid").and_then(Value::as_array);
    let mut times = Vec::<String>::new();
    if let Some(items) = time_grid {
        for item in items {
            if let Some(timestamp) = item.as_str() {
                times.push(timestamp.to_owned());
            } else {
                errors.push("time_grid entries must be timestamp strings".to_owned());
            }
        }
    } else {
        errors.push("time_grid must be an array".to_owned());
    }
    if times.len() < 3 {
        errors.push("time_grid must contain at least three points".to_owned());
    }
    let parsed_times = times.iter().map(|time| DateTime::parse_from_rfc3339(time).ok()).collect::<Vec<_>>();
    if parsed_times.iter().any(Option::is_none) {
        errors.push("time_grid contains an invalid RFC3339 timestamp".to_owned());
    } else {
        for pair in parsed_times.windows(2) {
            if pair[0] >= pair[1] {
                errors.push("time_grid must be strictly increasing".to_owned());
                break;
            }
        }
    }
    let time_digest = sha256_hex(times.join("\n").as_bytes());
    if string_at(value, "time_grid_sha256") != Some(time_digest.as_str())
        || string_at(snapshot, "time_grid_sha256") != Some(time_digest.as_str())
    {
        errors.push("time_grid_sha256 does not match the ordered timestamps".to_owned());
    }

    let roster = value.get("scenario_roster").and_then(Value::as_array);
    let mut roster_items = roster.map(|items| items.iter().filter_map(Value::as_str).map(str::to_owned).collect::<Vec<_>>()).unwrap_or_default();
    if roster.is_none() {
        errors.push("scenario_roster must be an array".to_owned());
    }
    roster_items.sort();
    if roster_items.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario_roster must contain all five required classes exactly once".to_owned());
    }
    let roster_digest = sha256_hex(roster_items.join("\n").as_bytes());
    if string_at(snapshot, "scenario_roster_sha256") != Some(roster_digest.as_str()) {
        errors.push("scenario_roster_sha256 does not match the canonical roster".to_owned());
    }

    let snapshot_digest = canonical_json_digest(snapshot, Some("mission_snapshot_sha256"));
    if snapshot_digest.as_deref() != string_at(snapshot, "mission_snapshot_sha256")
        || !digest_valid(snapshot_digest.as_deref())
    {
        errors.push("mission_snapshot_sha256 does not match the canonical snapshot".to_owned());
    }
    for field in [
        "craft_configuration_sha256", "occupancy_sha256", "initial_resource_state_sha256",
        "service_contract_sha256", "time_grid_sha256", "scenario_roster_sha256",
        "model_registry_sha256", "operating_envelope_sha256",
    ] {
        if !digest_valid(string_at(snapshot, field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }

    let scenarios = value.get("scenarios").and_then(Value::as_array);
    if scenarios.is_none_or(|items| items.len() != SCENARIO_CLASSES.len()) {
        errors.push("exactly five required scenario records are required".to_owned());
    }
    let mut seen_classes = BTreeSet::<String>::new();
    let mut seen_ids = BTreeSet::<String>::new();
    let mut initial_state: Option<BTreeMap<String, (String, f64, f64, f64)>> = None;
    let mut scenario_reports = Vec::<Value>::new();
    let mut all_envelopes = 0_usize;

    if let Some(scenarios) = scenarios {
        for scenario in scenarios {
            let scenario_id = string_at(scenario, "scenario_id").unwrap_or("unknown");
            let scenario_class = string_at(scenario, "scenario_class").unwrap_or("");
            if !seen_classes.insert(scenario_class.to_owned()) {
                errors.push(format!("scenario '{scenario_id}' duplicates class '{scenario_class}'"));
            }
            if !seen_ids.insert(scenario_id.to_owned()) {
                errors.push(format!("scenario_id '{scenario_id}' is duplicated"));
            }
            if !SCENARIO_CLASSES.contains(&scenario_class) {
                errors.push(format!("scenario '{scenario_id}' has an unknown class"));
            }
            if scenario.get("event_manifest_complete").and_then(Value::as_bool) != Some(true) {
                errors.push(format!("scenario '{scenario_id}' does not assert a complete event manifest"));
            }
            let events = scenario.get("events").and_then(Value::as_array);
            if events.is_none() {
                errors.push(format!("scenario '{scenario_id}' requires an events array"));
            }
            if let Some(events) = events {
                let actual = scenario.get("event_manifest_sha256").and_then(Value::as_str);
                let calculated = serde_json::to_vec(events).ok().map(|bytes| sha256_hex(&bytes));
                if calculated.as_deref() != actual {
                    errors.push(format!("scenario '{scenario_id}' event_manifest_sha256 does not bind its events"));
                }
                for event in events {
                    let timestamp = string_at(event, "timestamp").unwrap_or("");
                    if !times.iter().any(|grid_time| grid_time == timestamp) {
                        errors.push(format!("scenario '{scenario_id}' event at '{timestamp}' is not an explicit time-grid boundary"));
                    }
                    let event_class = string_at(event, "event_class").unwrap_or("");
                    let component_id = string_at(event, "component_id").unwrap_or("");
                    if string_at(event, "event_id").is_none_or(str::is_empty)
                        || !["outage", "repair", "weather_disturbance", "operator_action", "shore_service"].contains(&event_class)
                        || ![
                            "environment", "main_generator", "watermaker", "shore_power",
                            "battery_inverter", "potable_water_system", "waste_service"
                        ].contains(&component_id)
                        || parse_time(Some(timestamp)).is_none()
                    {
                        errors.push(format!("scenario '{scenario_id}' contains an event with invalid identity, class, component or timestamp"));
                    }
                }
            }
            match canonical_json_digest(scenario, Some("scenario_sha256")) {
                Some(calculated) if string_at(scenario, "scenario_sha256") == Some(calculated.as_str()) => {}
                Some(_) => errors.push(format!("scenario '{scenario_id}' scenario_sha256 does not bind the complete sample/event payload")),
                None => errors.push(format!("scenario '{scenario_id}' cannot be canonically serialized")),
            }
            let event_items = events.cloned().unwrap_or_default();
            let has_event = |event_class: &str, component_id: &str| {
                event_items.iter().any(|event|
                    string_at(event, "event_class") == Some(event_class)
                    && string_at(event, "component_id") == Some(component_id)
                )
            };
            match scenario_class {
                "nominal" => {
                    if !event_items.is_empty() {
                        errors.push(format!("nominal scenario '{scenario_id}' must not include disturbance events"));
                    }
                }
                "adverse_forecast_error" => {
                    if !has_event("weather_disturbance", "environment") {
                        errors.push(format!("adverse scenario '{scenario_id}' requires an explicit weather disturbance event"));
                    }
                }
                "main_generation_outage" => {
                    if !has_event("outage", "main_generator") {
                        errors.push(format!("generation-outage scenario '{scenario_id}' requires a main_generator outage event"));
                    }
                }
                "watermaker_outage" => {
                    if !has_event("outage", "watermaker") {
                        errors.push(format!("watermaker-outage scenario '{scenario_id}' requires a watermaker outage event"));
                    }
                }
                "common_cause_generation_watermaker_outage" => {
                    let generator = event_items.iter().find(|event|
                        string_at(event, "event_class") == Some("outage")
                        && string_at(event, "component_id") == Some("main_generator")
                    );
                    let watermaker = event_items.iter().find(|event|
                        string_at(event, "event_class") == Some("outage")
                        && string_at(event, "component_id") == Some("watermaker")
                    );
                    let shared = generator.zip(watermaker).is_some_and(|(generator, watermaker)|
                        string_at(generator, "event_id") == string_at(watermaker, "event_id")
                        && string_at(generator, "timestamp") == string_at(watermaker, "timestamp")
                    );
                    if !shared {
                        errors.push(format!("common-cause scenario '{scenario_id}' requires one shared outage event identity/time for generation and watermaker"));
                    }
                }
                _ => {}
            }

            let Some(samples) = scenario.get("samples").and_then(Value::as_array) else {
                errors.push(format!("scenario '{scenario_id}' has no samples array"));
                continue;
            };
            if samples.len() != times.len() {
                errors.push(format!("scenario '{scenario_id}' must contain exactly one sample per time-grid point"));
            }
            let mut sample_maps = Vec::<BTreeMap<String, SampleMetric>>::new();
            for (index, sample) in samples.iter().enumerate() {
                if sample.get("time_index").and_then(Value::as_u64) != Some(index as u64) {
                    errors.push(format!("scenario '{scenario_id}' has missing or misordered time_index {index}"));
                }
                if times.get(index).map(String::as_str) != string_at(sample, "timestamp") {
                    errors.push(format!("scenario '{scenario_id}' timestamp at sample {index} does not match the shared time grid"));
                }
                let metrics = sample_metrics(
                    sample, scenario_id, index, record_status,
                    &contract_by_metric, &mut errors, &mut blockers
                );
                if index == 0 {
                    let current = metrics.iter().map(|(metric, data)| (
                        metric.clone(),
                        (data.unit.clone(), data.value, data.lower, data.upper)
                    )).collect::<BTreeMap<_, _>>();
                    if let Some(reference) = &initial_state {
                        if reference != &current {
                            errors.push(format!("scenario '{scenario_id}' differs from the frozen initial resource state"));
                        }
                    } else {
                        initial_state = Some(current);
                    }
                }
                sample_maps.push(metrics);
            }

            let mut interval_reports = Vec::<Value>::new();
            let mut worst_by_metric = BTreeMap::<String, (f64, String)>::new();
            let mut scenario_first_breach: Option<Value> = None;
            if sample_maps.len() >= 2 && parsed_times.iter().all(Option::is_some) {
                for index in 0..sample_maps.len().min(parsed_times.len()).saturating_sub(1) {
                    let start_time = parsed_times[index].as_ref().unwrap();
                    let end_time = parsed_times[index + 1].as_ref().unwrap();
                    let Some(dt) = duration_hours(start_time, end_time) else {
                        errors.push(format!("scenario '{scenario_id}' interval {index} has non-positive duration"));
                        continue;
                    };
                    let mut interval_metrics = Vec::<Value>::new();
                    for (metric_id, contract_metric) in &contract_by_metric {
                        let (Some(start), Some(end)) = (
                            sample_maps[index].get(metric_id),
                            sample_maps[index + 1].get(metric_id),
                        ) else {
                            continue;
                        };
                        let required_gap = endpoint_gap(start, end);
                        let allowable_change = contract_metric.max_abs_rate_per_hour * dt;
                        if required_gap > allowable_change + NUMERIC_TOLERANCE {
                            errors.push(format!(
                                "scenario '{scenario_id}' metric '{metric_id}' endpoint uncertainty sets are incompatible with the declared rate ceiling over interval {index}"
                            ));
                        }
                        let (tube_bound, margin, bound_kind) = if contract_metric.operator == "gte" {
                            let bound = exact_lower_tube_min(
                                start.lower, end.lower,
                                contract_metric.max_abs_rate_per_hour, dt
                            );
                            (bound, bound - contract_metric.floor, "lower")
                        } else {
                            let bound = exact_upper_tube_max(
                                start.upper, end.upper,
                                contract_metric.max_abs_rate_per_hour, dt
                            );
                            (bound, contract_metric.floor - bound, "upper")
                        };
                        let interval_label = format!("{} -> {}", times[index], times[index + 1]);
                        match worst_by_metric.get(metric_id) {
                            Some((current, _)) if *current <= margin => {}
                            _ => {
                                worst_by_metric.insert(metric_id.clone(), (margin, interval_label.clone()));
                            }
                        }
                        let finding = json!({
                            "scenario_id":scenario_id,
                            "scenario_class":scenario_class,
                            "metric":metric_id,
                            "unit":contract_metric.unit,
                            "interval_index":index,
                            "interval_start":times[index],
                            "interval_end":times[index + 1],
                            "duration_hours":dt,
                            "bound_kind":bound_kind,
                            "inter_sample_conservative_bound":tube_bound,
                            "floor_operator":contract_metric.operator,
                            "floor_value":contract_metric.floor,
                            "conservative_margin":margin,
                            "rate_bound_per_hour":contract_metric.max_abs_rate_per_hour,
                            "rate_bound_method":contract_metric.rate_bound_method,
                            "rate_bound_evidence_sha256":contract_metric.rate_bound_evidence_sha256,
                            "endpoint_measurement_coverage_min":start.coverage.min(end.coverage),
                            "start_measurement_class":start.measurement_class,
                            "end_measurement_class":end.measurement_class,
                            "start_measurement_method":start.measurement_method,
                            "end_measurement_method":end.measurement_method,
                            "endpoint_evidence_sha256":[start.evidence_sha256.clone(),end.evidence_sha256.clone()]
                        });
                        if margin < -NUMERIC_TOLERANCE && scenario_first_breach.is_none() {
                            scenario_first_breach = Some(finding.clone());
                        }
                        interval_metrics.push(finding);
                        all_envelopes += 1;
                    }
                    interval_reports.push(json!({
                        "interval_index":index,
                        "start":times[index],
                        "end":times[index + 1],
                        "duration_hours":dt,
                        "metric_envelopes":interval_metrics
                    }));
                }
            }
            let scenario_blocked = scenario_first_breach.is_some()
                || blockers.iter().any(|finding| finding.contains(&format!("scenario '{scenario_id}'")));
            scenario_reports.push(json!({
                "scenario_id":scenario_id,
                "scenario_class":scenario_class,
                "inter_sample_feasible":!scenario_blocked,
                "first_inter_sample_breach":scenario_first_breach,
                "worst_margin_by_metric":worst_by_metric.iter().map(|(metric,(margin,interval))| json!({
                    "metric":metric,"worst_conservative_margin":margin,"worst_interval":interval
                })).collect::<Vec<_>>(),
                "interval_reports":interval_reports
            }));
        }
    }
    let actual_classes = seen_classes.iter().cloned().collect::<Vec<_>>();
    if actual_classes.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario records do not cover the exact mandatory roster".to_owned());
    }
    if let Some(initial) = &initial_state {
        let calculated = initial_state_digest(initial);
        if string_at(snapshot, "initial_resource_state_sha256") != Some(calculated.as_str()) {
            errors.push("initial_resource_state_sha256 does not match canonical first-sample resource values and bounds".to_owned());
        }
    } else {
        errors.push("no canonical initial resource state was available".to_owned());
    }

    let invalid = !errors.is_empty();
    let blocked = !blockers.is_empty()
        || scenario_reports.iter().any(|report| report["inter_sample_feasible"] == false);
    let disposition = if invalid {
        "INVALID"
    } else if blocked {
        "BLOCKED"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "INTER_SAMPLE_AUDITED_READ_ONLY"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "candidate_disposition":if invalid || blocked {"INFEASIBLE/ABSTAIN"} else {"DECLARED_RATE_BOUNDED_ENVELOPE_FEASIBLE"},
        "input_sha256":sha256_hex(raw),
        "scenario_count":scenario_reports.len(),
        "scenario_class_coverage":actual_classes,
        "time_point_count":times.len(),
        "interval_count_per_complete_scenario":times.len().saturating_sub(1),
        "inter_sample_envelope_count":all_envelopes,
        "scenario_reports":scenario_reports,
        "findings":{"errors":errors,"blockers":blockers},
        "aggregate_scalar_score":"NOT_COMPUTED",
        "claim_ceiling":"declared_rate_bounded_inter_sample_envelope_not_physical_rate_bound_validation_not_viability_kernel_not_operational_authorization",
        "assumptions":[
            "Each declared maximum absolute rate is valid over the exact registered operating envelope.",
            "The continuous-state Lipschitz assumption applies between each adjacent pair of samples.",
            "Known discrete mode changes and outage/repair events occur only at explicit time-grid boundaries.",
            "Endpoint measurement intervals contain the true endpoint values."
        ]
    })
}

fn run() -> Result<(), String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 || args[1] != "audit-inter-sample" {
        return Err("Usage: marine-intersample-envelope audit-inter-sample <bundle.json>".to_owned());
    }
    let raw = fs::read(&args[2]).map_err(|error| format!("{}: {error}", args[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", args[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "INTER_SAMPLE_AUDITED_READ_ONLY" | "BLOCKED") => Ok(()),
        Some(disposition) => Err(format!("inter-sample disposition: {disposition}")),
        None => Err("inter-sample report has no disposition".to_owned())
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
        serde_json::from_str(include_str!("../fixtures/synthetic-intersample-safe-0001.json"))
            .expect("safe fixture parses")
    }

    fn hidden_dip_fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-intersample-hidden-dip-0001.json"))
            .expect("hidden-dip fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    fn refresh_scenario_digest(scenario: &mut Value) {
        let events = scenario["events"].clone();
        scenario["event_manifest_sha256"] =
            json!(sha256_hex(&serde_json::to_vec(&events).expect("events serialize")));
        scenario["scenario_sha256"] = json!("");
        let digest = canonical_json_digest(scenario, Some("scenario_sha256"))
            .expect("scenario canonicalizes");
        scenario["scenario_sha256"] = json!(digest);
    }

    fn refresh_initial_and_snapshot(value: &mut Value) {
        let metrics = value["scenarios"][0]["samples"][0]["resource_metrics"]
            .as_array().expect("initial metrics array");
        let mut rows = metrics.iter().map(|metric| format!(
            "{}|{}|{:.17}|{:.17}|{:.17}",
            string_at(metric, "metric").expect("metric name"),
            string_at(metric, "unit").expect("metric unit"),
            number_at(metric, "value").expect("metric value"),
            number_at(metric, "lower_bound").expect("lower bound"),
            number_at(metric, "upper_bound").expect("upper bound")
        )).collect::<Vec<_>>();
        rows.sort();
        value["mission_snapshot"]["initial_resource_state_sha256"] =
            json!(sha256_hex(rows.join("\n").as_bytes()));
        value["mission_snapshot"]["mission_snapshot_sha256"] = json!("");
        let digest = canonical_json_digest(
            &value["mission_snapshot"], Some("mission_snapshot_sha256")
        ).expect("snapshot canonicalizes");
        value["mission_snapshot"]["mission_snapshot_sha256"] = json!(digest);
    }

    #[test]
    fn exact_two_cone_lower_and_upper_bounds_match_hand_calculation() {
        assert!((exact_lower_tube_min(90.0, 90.0, 4.0, 6.0) - 78.0).abs() < 1e-9);
        assert!((exact_upper_tube_max(11.0, 11.0, 0.5, 6.0) - 12.5).abs() < 1e-9);
        assert!((exact_lower_tube_min(100.0, 0.0, 1.0, 1.0) - 99.0).abs() < 1e-9);
        assert!((exact_upper_tube_max(100.0, 0.0, 1.0, 1.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn all_five_synthetic_scenarios_pass_the_declared_rate_envelope() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["candidate_disposition"], "DECLARED_RATE_BOUNDED_ENVELOPE_FEASIBLE");
        assert_eq!(report["scenario_count"], 5);
        assert_eq!(report["inter_sample_envelope_count"], 110);
        assert_eq!(report["aggregate_scalar_score"], "NOT_COMPUTED");
    }

    #[test]
    fn both_endpoints_can_pass_while_hidden_inter_sample_dip_is_blocked() {
        let value = hidden_dip_fixture();
        for scenario in value["scenarios"].as_array().unwrap() {
            for sample in scenario["samples"].as_array().unwrap() {
                let freshwater = sample["resource_metrics"].as_array().unwrap().iter()
                    .find(|metric| metric["metric"] == "freshwater_usable_l").unwrap();
                assert!(freshwater["lower_bound"].as_f64().unwrap() >= 80.0);
            }
        }
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        assert_eq!(report["candidate_disposition"], "INFEASIBLE/ABSTAIN");
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_class"] == "adverse_forecast_error").unwrap();
        let breach = scenario["first_inter_sample_breach"].as_object().unwrap();
        assert_eq!(breach["metric"], "freshwater_usable_l");
        assert_eq!(breach["inter_sample_conservative_bound"], 53.9);
        assert!(report["scenario_reports"].as_array().unwrap().iter()
            .filter(|item| item["inter_sample_feasible"] == false)
            .all(|item| item["scenario_class"] == "adverse_forecast_error"));
    }

    #[test]
    fn symmetric_upper_exposure_tube_can_breach_between_passing_endpoints() {
        let mut value = fixture();
        for scenario in value["scenarios"].as_array_mut().unwrap() {
            for sample in scenario["samples"].as_array_mut().unwrap() {
                let metric = sample["resource_metrics"].as_array_mut().unwrap().iter_mut()
                    .find(|metric| metric["metric"] == "crew_workload_hours_rolling_24h").unwrap();
                metric["value"] = json!(11.8);
                metric["lower_bound"] = json!(11.7);
                metric["upper_bound"] = json!(11.9);
            }
            refresh_scenario_digest(scenario);
        }
        refresh_initial_and_snapshot(&mut value);
        let report = audit(&value);
        assert_eq!(report["disposition"], "BLOCKED");
        let scenario = report["scenario_reports"].as_array().unwrap().iter()
            .find(|item| item["scenario_class"] == "nominal").unwrap();
        let breach = scenario["first_inter_sample_breach"].as_object().unwrap();
        assert_eq!(breach["metric"], "crew_workload_hours_rolling_24h");
        assert!(breach["inter_sample_conservative_bound"].as_f64().unwrap() > 12.0);
    }

    #[test]
    fn insufficient_endpoint_measurement_coverage_blocks() {
        let mut value = fixture();
        value["scenarios"][0]["samples"][1]["resource_metrics"][1]["coverage_fraction"] = json!(0.5);
        refresh_scenario_digest(&mut value["scenarios"][0]);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn missing_required_scenario_is_invalid() {
        let mut value = fixture();
        value["scenarios"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn missing_time_sample_is_invalid() {
        let mut value = fixture();
        value["scenarios"][0]["samples"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn event_between_sample_boundaries_is_invalid() {
        let mut value = fixture();
        value["scenarios"][0]["events"][0]["timestamp"] = json!("2026-04-01T03:00:00Z");
        let events = value["scenarios"][0]["events"].clone();
        value["scenarios"][0]["event_manifest_sha256"] = json!(sha256_hex(&serde_json::to_vec(&events).unwrap()));
        let mut scenario = value["scenarios"][0].clone();
        scenario["scenario_sha256"] = json!("");
        value["scenarios"][0]["scenario_sha256"] = json!(canonical_json_digest(&scenario, Some("scenario_sha256")).unwrap());
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn rate_bound_evidence_digest_is_required() {
        let mut value = fixture();
        value["service_contract"][1]["rate_bound_evidence_sha256"] = json!("not-a-digest");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn canonical_contract_digest_detects_rate_bound_tampering() {
        let mut value = fixture();
        value["service_contract"][1]["max_abs_rate_per_hour"] = json!(0.1);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn scenario_digest_detects_sample_tampering() {
        let mut value = fixture();
        value["scenarios"][0]["samples"][1]["resource_metrics"][0]["value"] = json!(25.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn initial_resource_digest_detects_starting_reserve_tampering() {
        let mut value = fixture();
        value["scenarios"][0]["samples"][0]["resource_metrics"][0]["value"] = json!(25.0);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn mission_snapshot_digest_detects_operating_envelope_tampering() {
        let mut value = fixture();
        value["mission_snapshot"]["operating_envelope_sha256"] = json!("f".repeat(64));
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn rate_bound_cannot_be_promoted_from_synthetic_to_measured() {
        let mut value = fixture();
        value["record_status"] = json!("measured_campaign");
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn endpoint_sets_must_be_compatible_with_the_declared_rate() {
        let mut value = fixture();
        value["scenarios"][0]["samples"][1]["resource_metrics"][0]["value"] = json!(1.0);
        value["scenarios"][0]["samples"][1]["resource_metrics"][0]["lower_bound"] = json!(0.9);
        value["scenarios"][0]["samples"][1]["resource_metrics"][0]["upper_bound"] = json!(1.1);
        refresh_scenario_digest(&mut value["scenarios"][0]);
        assert_eq!(audit(&value)["findings"]["errors"].as_array().unwrap().iter()
            .any(|error| error.as_str().unwrap_or("").contains("endpoint uncertainty sets are incompatible")), true);
    }

    #[test]
    fn each_disturbance_scenario_requires_its_declared_event_type() {
        let mut value = fixture();
        value["scenarios"][2]["events"] = json!([]);
        value["scenarios"][2]["event_manifest_sha256"] = json!(sha256_hex(&serde_json::to_vec(&value["scenarios"][2]["events"]).unwrap()));
        value["scenarios"][2]["scenario_sha256"] = json!("");
        let digest = canonical_json_digest(&value["scenarios"][2], Some("scenario_sha256")).unwrap();
        value["scenarios"][2]["scenario_sha256"] = json!(digest);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn unsupported_event_class_or_component_is_invalid() {
        let mut value = fixture();
        value["scenarios"][0]["events"][0]["component_id"] = json!("unknown-sensor");
        value["scenarios"][0]["event_manifest_sha256"] = json!(sha256_hex(&serde_json::to_vec(&value["scenarios"][0]["events"]).unwrap()));
        value["scenarios"][0]["scenario_sha256"] = json!("");
        let digest = canonical_json_digest(&value["scenarios"][0], Some("scenario_sha256")).unwrap();
        value["scenarios"][0]["scenario_sha256"] = json!(digest);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn common_cause_requires_shared_event_identity_and_timestamp() {
        let mut value = fixture();
        value["scenarios"][1]["events"][1]["event_id"] = json!("independent-watermaker-event");
        value["scenarios"][1]["event_manifest_sha256"] = json!(sha256_hex(&serde_json::to_vec(&value["scenarios"][1]["events"]).unwrap()));
        value["scenarios"][1]["scenario_sha256"] = json!("");
        let digest = canonical_json_digest(&value["scenarios"][1], Some("scenario_sha256")).unwrap();
        value["scenarios"][1]["scenario_sha256"] = json!(digest);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn closed_form_bounds_are_conservative_against_dense_segment_sampling() {
        for a_i in 0..=8 {
            for b_i in 0..=8 {
                for rate_i in 0..=5 {
                    let a = a_i as f64;
                    let b = b_i as f64;
                    let rate = rate_i as f64 * 0.2;
                    let dt = 3.0;
                    let lower = exact_lower_tube_min(a, b, rate, dt);
                    let upper = exact_upper_tube_max(a, b, rate, dt);
                    for step in 0..=10_000 {
                        let t = dt * step as f64 / 10_000.0;
                        let lower_cone = (a - rate * t).max(b - rate * (dt - t));
                        let upper_cone = (a + rate * t).min(b + rate * (dt - t));
                        assert!(lower <= lower_cone + 1e-9);
                        assert!(upper + 1e-9 >= upper_cone);
                    }
                }
            }
        }
    }

    #[test]
    fn event_manifest_completeness_is_mandatory() {
        let mut value = fixture();
        value["scenarios"][0]["event_manifest_complete"] = json!(false);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn zero_rate_bound_yields_constant_segment_envelope() {
        assert_eq!(exact_lower_tube_min(10.0, 10.0, 0.0, 6.0), 10.0);
        assert_eq!(exact_upper_tube_max(10.0, 10.0, 0.0, 6.0), 10.0);
    }
}
