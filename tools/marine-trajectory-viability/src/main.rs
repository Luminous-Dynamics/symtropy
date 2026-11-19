use chrono::{DateTime, FixedOffset};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-trajectory-viability/0.1.0";
const SCENARIO_CLASSES: [&str; 5] = [
    "adverse_forecast_error",
    "common_cause_generation_watermaker_outage",
    "main_generation_outage",
    "nominal",
    "watermaker_outage",
];
const COMPONENT_IDS: [&str; 4] = [
    "main_generator",
    "watermaker",
    "shore_power",
    "battery_inverter",
];
const NUMERIC_TOLERANCE: f64 = 1e-9;

#[derive(Clone, Copy)]
struct MetricSpec {
    metric: &'static str,
    unit: &'static str,
    operator: &'static str,
}

const METRICS: [MetricSpec; 11] = [
    MetricSpec { metric: "energy_reserve_kwh", unit: "kWh", operator: "gte" },
    MetricSpec { metric: "freshwater_usable_l", unit: "L", operator: "gte" },
    MetricSpec { metric: "food_reserve_days", unit: "day", operator: "gte" },
    MetricSpec { metric: "waste_capacity_margin_days", unit: "day", operator: "gte" },
    MetricSpec { metric: "critical_spares_coverage_days", unit: "day", operator: "gte" },
    MetricSpec { metric: "power_service_availability", unit: "fraction", operator: "gte" },
    MetricSpec { metric: "potable_water_service_availability", unit: "fraction", operator: "gte" },
    MetricSpec { metric: "sanitation_service_availability", unit: "fraction", operator: "gte" },
    MetricSpec { metric: "refrigeration_service_availability", unit: "fraction", operator: "gte" },
    MetricSpec { metric: "crew_workload_hours_rolling_24h", unit: "h", operator: "lte" },
    MetricSpec { metric: "comfort_breach_minutes_rolling_24h", unit: "min", operator: "lte" },
];

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

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
}

fn close(left: f64, right: f64, tolerance: f64) -> bool {
    (left - right).abs() <= tolerance.max(NUMERIC_TOLERANCE)
}

fn contract_canonical(contract: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = contract.as_array()?;
    if items.len() != METRICS.len() {
        errors.push("service contract must define exactly eleven trajectory axes".to_owned());
    }
    let mut rows = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let floor = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        let (Some(floor), Some(coverage)) = (floor, coverage) else {
            errors.push(format!("contract axis '{metric}' has a missing/non-finite floor or coverage"));
            continue;
        };
        if metric.is_empty() || !seen.insert(metric.to_owned()) {
            errors.push(format!("service contract has a missing or duplicated axis '{metric}'"));
            continue;
        }
        let Some(spec) = METRICS.iter().find(|spec| spec.metric == metric) else {
            errors.push(format!("service contract declares unknown axis '{metric}'"));
            continue;
        };
        if unit != spec.unit
            || operator != spec.operator
            || !(0.0..=1.0).contains(&coverage)
        {
            errors.push(format!("service contract axis '{metric}' has an invalid unit, operator or coverage threshold"));
        }
        rows.push((
            metric.to_owned(),
            format!("{metric}|{unit}|{operator}|{floor:.17}|{coverage:.17}"),
        ));
    }
    for spec in METRICS {
        if !seen.contains(spec.metric) {
            errors.push(format!("service contract is missing '{}'", spec.metric));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n"))
}

fn snapshot_digest(snapshot: &Value) -> Option<String> {
    let object = snapshot.as_object()?;
    let mut fields = object.iter()
        .filter(|(key, _)| key.as_str() != "mission_snapshot_sha256")
        .map(|(key, value)| {
            let value = value.as_str()?;
            Some((key.clone(), format!("{key}|{value}")))
        })
        .collect::<Option<Vec<_>>>()?;
    fields.sort_by(|left, right| left.0.cmp(&right.0));
    Some(sha256_hex(fields.into_iter()
        .map(|(_, field)| field)
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes()))
}

fn initial_state_digest(state: &Value, errors: &mut Vec<String>) -> Option<String> {
    let metrics = state.get("resource_metrics").and_then(Value::as_array)?;
    let mut rows = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in metrics {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let (Some(value), Some(lower), Some(upper)) = (
            number_at(item, "value"),
            number_at(item, "lower_bound"),
            number_at(item, "upper_bound"),
        ) else {
            errors.push(format!("initial resource axis '{metric}' has invalid numeric values"));
            continue;
        };
        if !seen.insert(metric.to_owned()) {
            errors.push(format!("initial state duplicates resource axis '{metric}'"));
        }
        rows.push((
            metric.to_owned(),
            format!("{metric}|{unit}|{value:.17}|{lower:.17}|{upper:.17}"),
        ));
    }
    if rows.len() != METRICS.len() {
        errors.push("initial state does not include every trajectory resource axis".to_owned());
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(sha256_hex(rows.into_iter()
        .map(|(_, row)| row)
        .collect::<Vec<_>>()
        .join("\n")
        .as_bytes()))
}

fn component<'a>(state: &'a Value, component_id: &str) -> Option<&'a Value> {
    state.get("components")?.as_array()?.iter()
        .find(|item| string_at(item, "component_id") == Some(component_id))
}

fn metric<'a>(state: &'a Value, metric_id: &str) -> Option<&'a Value> {
    state.get("resource_metrics")?.as_array()?.iter()
        .find(|item| string_at(item, "metric") == Some(metric_id))
}

fn component_unavailable(from: &Value, to: &Value, component_id: &str) -> bool {
    [from, to].iter().any(|state| {
        component(state, component_id)
            .and_then(|item| string_at(item, "availability"))
            == Some("unavailable")
    })
}

fn check_component_roster(state: &Value, label: &str, errors: &mut Vec<String>) -> BTreeMap<String, String> {
    let mut statuses = BTreeMap::<String, String>::new();
    let Some(items) = state.get("components").and_then(Value::as_array) else {
        errors.push(format!("{label} has no component availability array"));
        return statuses;
    };
    if items.len() != COMPONENT_IDS.len() {
        errors.push(format!("{label} must report exactly four component states"));
    }
    for item in items {
        let id = string_at(item, "component_id").unwrap_or("");
        let availability = string_at(item, "availability").unwrap_or("");
        let event_id = string_at(item, "event_id").unwrap_or("");
        if !COMPONENT_IDS.contains(&id) {
            errors.push(format!("{label} contains unknown component '{id}'"));
            continue;
        }
        if !["available", "unavailable"].contains(&availability) {
            errors.push(format!("{label} component '{id}' has invalid availability"));
        }
        if availability == "available" && event_id != "none" {
            errors.push(format!("{label} available component '{id}' must use event_id 'none'"));
        }
        if availability == "unavailable" && event_id.is_empty() {
            errors.push(format!("{label} unavailable component '{id}' needs an event identity"));
        }
        if statuses.insert(id.to_owned(), availability.to_owned()).is_some() {
            errors.push(format!("{label} duplicates component '{id}'"));
        }
    }
    for id in COMPONENT_IDS {
        if !statuses.contains_key(id) {
            errors.push(format!("{label} is missing component '{id}'"));
        }
    }
    statuses
}

fn check_resource_metrics(
    state: &Value,
    scenario_id: &str,
    time_index: usize,
    contract: &Value,
    record_status: &str,
    errors: &mut Vec<String>,
    blockers: &mut Vec<String>,
    margins: &mut BTreeMap<String, (f64, String)>,
) -> BTreeMap<String, f64> {
    let label = format!("scenario '{scenario_id}' time_index {time_index}");
    let Some(items) = state.get("resource_metrics").and_then(Value::as_array) else {
        errors.push(format!("{label} has no resource metric array"));
        return BTreeMap::new();
    };
    if items.len() != METRICS.len() {
        errors.push(format!("{label} must include exactly eleven resource axes"));
    }
    let mut result = BTreeMap::<String, f64>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in items {
        let metric_id = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let value = number_at(item, "value");
        let lower = number_at(item, "lower_bound");
        let upper = number_at(item, "upper_bound");
        let coverage = number_at(item, "coverage_fraction");
        let measurement_class = string_at(item, "measurement_class").unwrap_or("");
        let measurement_method = string_at(item, "measurement_method").unwrap_or("");
        if !seen.insert(metric_id.to_owned()) {
            errors.push(format!("{label} duplicates metric '{metric_id}'"));
            continue;
        }
        let Some(spec) = METRICS.iter().find(|spec| spec.metric == metric_id) else {
            errors.push(format!("{label} contains unknown resource metric '{metric_id}'"));
            continue;
        };
        if unit != spec.unit || !valid_digest(string_at(item, "evidence_sha256")) {
            errors.push(format!("{label} metric '{metric_id}' has wrong unit or invalid evidence digest"));
        }
        let (Some(value), Some(lower), Some(upper), Some(coverage)) =
            (value, lower, upper, coverage) else {
                errors.push(format!("{label} metric '{metric_id}' has missing or non-finite data"));
                continue;
            };
        if lower < 0.0 || lower > value || value > upper
            || !(0.0..=1.0).contains(&coverage)
            || measurement_method.is_empty()
            || !["measured", "modeled", "derived", "synthetic"].contains(&measurement_class)
            || (record_status == "synthetic" && measurement_class != "synthetic")
            || (record_status != "synthetic" && measurement_class == "synthetic")
        {
            errors.push(format!("{label} metric '{metric_id}' has invalid uncertainty, coverage, method or evidence class"));
            continue;
        }
        result.insert(metric_id.to_owned(), value);
        let contract_item = contract.as_array()
            .and_then(|items| items.iter().find(|item| string_at(item, "metric") == Some(metric_id)));
        let Some(contract_item) = contract_item else {
            errors.push(format!("service contract missing '{metric_id}'"));
            continue;
        };
        let floor = number_at(contract_item, "floor_value").unwrap_or(f64::NAN);
        let min_coverage = number_at(contract_item, "minimum_coverage_fraction").unwrap_or(1.0);
        let operator = string_at(contract_item, "operator").unwrap_or("");
        let margin = match operator {
            "gte" => lower - floor,
            "lte" => floor - upper,
            _ => f64::NEG_INFINITY,
        };
        let timestamp = string_at(state, "timestamp").unwrap_or("unknown");
        match margins.get(metric_id) {
            Some((current, _)) if *current <= margin => {}
            _ => {
                margins.insert(metric_id.to_owned(), (margin, timestamp.to_owned()));
            }
        }
        if coverage < min_coverage {
            blockers.push(format!("{label} metric '{metric_id}' coverage {:.3} is below {:.3}", coverage, min_coverage));
        }
        if margin < -NUMERIC_TOLERANCE {
            blockers.push(format!("{label} metric '{metric_id}' conservative bound breaches floor {operator} {floor} {unit}"));
        }
    }
    for spec in METRICS {
        if !seen.contains(spec.metric) {
            errors.push(format!("{label} is missing resource metric '{}'", spec.metric));
        }
    }
    result
}

fn check_energy_transition(
    transition: &Value,
    from: &Value,
    to: &Value,
    scenario_id: &str,
    transition_index: usize,
    errors: &mut Vec<String>,
) -> f64 {
    let label = format!("scenario '{scenario_id}' transition {transition_index} energy");
    let Some(flow) = transition.get("energy") else {
        errors.push(format!("{label} ledger is missing"));
        return f64::INFINITY;
    };
    let keys = ["start_kwh","generated_kwh","shore_charged_kwh","house_load_kwh","propulsion_load_kwh","conversion_loss_kwh","end_kwh","tolerance_kwh"];
    let values = keys.iter().map(|key| number_at(flow,key)).collect::<Option<Vec<_>>>();
    let Some(v) = values else {
        errors.push(format!("{label} has missing/non-finite fields"));
        return f64::INFINITY;
    };
    if v.iter().any(|number| *number < 0.0) || !valid_digest(string_at(flow,"source_sha256")) {
        errors.push(format!("{label} has negative flow or invalid evidence digest"));
    }
    let residual = v[0] + v[1] + v[2] - v[3] - v[4] - v[5] - v[6];
    if residual.abs() > v[7].max(NUMERIC_TOLERANCE) {
        errors.push(format!("{label} conservation residual {residual:.6} exceeds tolerance {:.6}",v[7]));
    }
    let from_state = metric(from,"energy_reserve_kwh").and_then(|item| number_at(item,"value"));
    let to_state = metric(to,"energy_reserve_kwh").and_then(|item| number_at(item,"value"));
    if from_state.is_none_or(|state| !close(state,v[0],v[7]))
        || to_state.is_none_or(|state| !close(state,v[6],v[7]))
    {
        errors.push(format!("{label} start/end do not reconcile to time-step resource states"));
    }
    if component_unavailable(from,to,"main_generator") && v[1] > NUMERIC_TOLERANCE {
        errors.push(format!("{label} credits generation while main_generator is unavailable"));
    }
    if (component_unavailable(from,to,"shore_power")
        || component_unavailable(from,to,"battery_inverter"))
        && v[2] > NUMERIC_TOLERANCE
    {
        errors.push(format!("{label} credits shore charging while shore power or battery inverter is unavailable"));
    }
    residual.abs()
}

fn check_freshwater_transition(
    transition: &Value,
    from: &Value,
    to: &Value,
    scenario_id: &str,
    transition_index: usize,
    errors: &mut Vec<String>,
) -> f64 {
    let label = format!("scenario '{scenario_id}' transition {transition_index} freshwater");
    let Some(flow) = transition.get("freshwater") else {
        errors.push(format!("{label} ledger is missing"));
        return f64::INFINITY;
    };
    let keys = ["start_l","produced_l","replenished_l","consumed_l","authorized_discharge_l","end_l","tolerance_l"];
    let values = keys.iter().map(|key| number_at(flow,key)).collect::<Option<Vec<_>>>();
    let Some(v) = values else {
        errors.push(format!("{label} has missing/non-finite fields"));
        return f64::INFINITY;
    };
    if v.iter().any(|number| *number < 0.0)
        || !string_at(flow,"replenishment_authority_ref").is_some_and(|reference| !reference.is_empty())
        || !valid_digest(string_at(flow,"replenishment_evidence_sha256"))
    {
        errors.push(format!("{label} has negative flow or missing authority/evidence identity"));
    }
    if v[4] > NUMERIC_TOLERANCE {
        errors.push(format!("{label} authorized_discharge_l is unsupported in this profile; discharge needs a separately qualified authority contract"));
    }
    if v[2] > NUMERIC_TOLERANCE
        && string_at(flow,"replenishment_authority_ref").is_none_or(|reference| reference == "NO_REPLENISHMENT")
    {
        errors.push(format!("{label} credits replenishment without an authority receipt"));
    }
    let residual = v[0] + v[1] + v[2] - v[3] - v[4] - v[5];
    if residual.abs() > v[6].max(NUMERIC_TOLERANCE) {
        errors.push(format!("{label} conservation residual {residual:.6} exceeds tolerance {:.6}",v[6]));
    }
    let from_state = metric(from,"freshwater_usable_l").and_then(|item| number_at(item,"value"));
    let to_state = metric(to,"freshwater_usable_l").and_then(|item| number_at(item,"value"));
    if from_state.is_none_or(|state| !close(state,v[0],v[6]))
        || to_state.is_none_or(|state| !close(state,v[5],v[6]))
    {
        errors.push(format!("{label} start/end do not reconcile to time-step resource states"));
    }
    if component_unavailable(from,to,"watermaker") && v[1] > NUMERIC_TOLERANCE {
        errors.push(format!("{label} credits water production while watermaker is unavailable"));
    }
    residual.abs()
}

fn check_waste_transition(
    transition: &Value,
    from: &Value,
    to: &Value,
    scenario_id: &str,
    transition_index: usize,
    errors: &mut Vec<String>,
) -> f64 {
    let label = format!("scenario '{scenario_id}' transition {transition_index} waste");
    let Some(flow) = transition.get("waste") else {
        errors.push(format!("{label} ledger is missing"));
        return f64::INFINITY;
    };
    let keys = ["start_margin_days","authorized_emptying_days","generated_days","end_margin_days","tolerance_days"];
    let values = keys.iter().map(|key| number_at(flow,key)).collect::<Option<Vec<_>>>();
    let Some(v) = values else {
        errors.push(format!("{label} has missing/non-finite fields"));
        return f64::INFINITY;
    };
    if v.iter().any(|number| *number < 0.0)
        || !string_at(flow,"service_authority_ref").is_some_and(|reference| !reference.is_empty())
        || !valid_digest(string_at(flow,"service_evidence_sha256"))
    {
        errors.push(format!("{label} has negative flow or missing service authority/evidence"));
    }
    if v[1] > NUMERIC_TOLERANCE && string_at(flow,"service_authority_ref") == Some("NO_EMPTYING") {
        errors.push(format!("{label} credits waste emptying without service authority"));
    }
    let residual = v[0] + v[1] - v[2] - v[3];
    if residual.abs() > v[4].max(NUMERIC_TOLERANCE) {
        errors.push(format!("{label} conservation residual {residual:.6} exceeds tolerance {:.6}",v[4]));
    }
    let from_state = metric(from,"waste_capacity_margin_days").and_then(|item| number_at(item,"value"));
    let to_state = metric(to,"waste_capacity_margin_days").and_then(|item| number_at(item,"value"));
    if from_state.is_none_or(|state| !close(state,v[0],v[4]))
        || to_state.is_none_or(|state| !close(state,v[3],v[4]))
    {
        errors.push(format!("{label} start/end do not reconcile to time-step resource states"));
    }
    residual.abs()
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut blockers = Vec::<String>::new();
    if string_at(value,"schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value,"scorecard_id").is_none_or(str::is_empty) {
        errors.push("scorecard_id is required".to_owned());
    }
    if string_at(value,"claim_ceiling") != Some("finite_horizon_sampled_trajectory_audit_only") {
        errors.push("claim_ceiling must preserve the sampled-trajectory-only boundary".to_owned());
    }
    let record_status = string_at(value,"record_status").unwrap_or("");
    if !["synthetic","shadow_replay","measured_campaign","published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let contract = value.get("service_contract").unwrap_or(&Value::Null);
    let canonical_contract = contract_canonical(contract,&mut errors);
    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    if let Some(canonical) = canonical_contract {
        let calculated = sha256_hex(canonical.as_bytes());
        if string_at(snapshot,"service_contract_sha256") != Some(calculated.as_str()) {
            errors.push("service_contract_sha256 does not match canonical trajectory contract".to_owned());
        }
    }

    let roster = value.get("scenario_roster").and_then(Value::as_array);
    let mut roster_classes = Vec::<String>::new();
    if let Some(items) = roster {
        roster_classes.extend(items.iter().filter_map(Value::as_str).map(str::to_owned));
    } else {
        errors.push("scenario_roster must be an array".to_owned());
    }
    roster_classes.sort();
    if roster_classes.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario_roster must contain every required disturbance class exactly once".to_owned());
    }
    let roster_canonical = roster_classes.join("\n");
    let roster_digest = sha256_hex(roster_canonical.as_bytes());
    if string_at(snapshot,"scenario_roster_sha256") != Some(roster_digest.as_str()) {
        errors.push("scenario_roster_sha256 does not match the canonical scenario roster".to_owned());
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
        errors.push("time_grid must include at least three ordered time points".to_owned());
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
    if string_at(value,"time_grid_sha256") != Some(time_digest.as_str())
        || string_at(snapshot,"time_grid_sha256") != Some(time_digest.as_str())
    {
        errors.push("time_grid_sha256 does not match the ordered time grid".to_owned());
    }

    let snapshot_hash = snapshot_digest(snapshot);
    if snapshot_hash.is_none()
        || snapshot_hash.as_deref() != string_at(snapshot,"mission_snapshot_sha256")
        || !valid_digest(snapshot_hash.as_deref())
    {
        errors.push("mission_snapshot_sha256 does not match canonical snapshot fields".to_owned());
    }
    for field in [
        "craft_configuration_sha256","occupancy_sha256","initial_resource_state_sha256",
        "mission_plan_sha256","service_contract_sha256","scenario_roster_sha256",
        "time_grid_sha256","forecast_information_set_sha256","hard_gate_roster_sha256",
        "operating_envelope_sha256",
    ] {
        if !valid_digest(string_at(snapshot,field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }

    let scenarios = value.get("scenarios").and_then(Value::as_array);
    if scenarios.is_none_or(|items| items.len() != SCENARIO_CLASSES.len()) {
        errors.push("exactly five required scenario records are required".to_owned());
    }
    let mut seen_classes = BTreeSet::<String>::new();
    let mut seen_ids = BTreeSet::<String>::new();
    let mut scenario_reports = Vec::<Value>::new();
    let mut initial_state_reference: Option<BTreeMap<String,(String,f64,f64,f64)>> = None;
    let mut maximum_balance_residuals = BTreeMap::<String,f64>::new();
    let mut initial_digest_observed: Option<String> = None;

    if let Some(scenarios) = scenarios {
        for scenario in scenarios {
            let scenario_id = string_at(scenario,"scenario_id").unwrap_or("unknown");
            let scenario_class = string_at(scenario,"scenario_class").unwrap_or("");
            if !seen_classes.insert(scenario_class.to_owned()) {
                errors.push(format!("scenario '{scenario_id}' duplicates class '{scenario_class}'"));
            }
            if !seen_ids.insert(scenario_id.to_owned()) {
                errors.push(format!("scenario_id '{scenario_id}' is duplicated"));
            }
            if !SCENARIO_CLASSES.contains(&scenario_class) {
                errors.push(format!("scenario '{scenario_id}' has unknown scenario class '{scenario_class}'"));
            }
            for field in ["scenario_sha256","disturbance_sha256","model_sha256"] {
                if !valid_digest(string_at(scenario,field)) {
                    errors.push(format!("scenario '{scenario_id}' has invalid {field}"));
                }
            }

            let states = scenario.get("states").and_then(Value::as_array);
            if states.is_none_or(|items| items.len() != times.len()) {
                errors.push(format!("scenario '{scenario_id}' must have exactly one state per time-grid point"));
            }
            let mut state_components = Vec::<BTreeMap<String,String>>::new();
            let mut margins = BTreeMap::<String,(f64,String)>::new();
            let mut scenario_breaches = Vec::<Value>::new();
            let mut first_breach: Option<Value> = None;
            if let Some(states) = states {
                for (index,state) in states.iter().enumerate() {
                    let timestamp = string_at(state,"timestamp").unwrap_or("unknown");
                    if state.get("time_index").and_then(Value::as_u64) != Some(index as u64) {
                        errors.push(format!("scenario '{scenario_id}' has missing or misordered time_index {index}"));
                    }
                    if times.get(index).map(String::as_str) != Some(timestamp) {
                        errors.push(format!("scenario '{scenario_id}' timestamp at index {index} does not match time_grid"));
                    }
                    let components = check_component_roster(state,&format!("scenario '{scenario_id}' time_index {index}"),&mut errors);
                    state_components.push(components);
                    let metrics = check_resource_metrics(
                        state,scenario_id,index,contract,record_status,
                        &mut errors,&mut blockers,&mut margins
                    );
                    if index == 0 {
                        let digest = initial_state_digest(state,&mut errors);
                        if let Some(digest) = digest {
                            if initial_digest_observed.is_none() {
                                initial_digest_observed = Some(digest.clone());
                            } else if initial_digest_observed.as_deref() != Some(digest.as_str()) {
                                errors.push("initial resource-state digest differs between scenarios".to_owned());
                            }
                        }
                        let mut initial = BTreeMap::<String,(String,f64,f64,f64)>::new();
                        if let Some(items) = state.get("resource_metrics").and_then(Value::as_array) {
                            for item in items {
                                if let (Some(id),Some(unit),Some(v),Some(lo),Some(hi)) = (
                                    string_at(item,"metric"),string_at(item,"unit"),
                                    number_at(item,"value"),number_at(item,"lower_bound"),number_at(item,"upper_bound")
                                ) {
                                    initial.insert(id.to_owned(),(unit.to_owned(),v,lo,hi));
                                }
                            }
                        }
                        if let Some(reference) = &initial_state_reference {
                            if reference != &initial {
                                errors.push(format!("scenario '{scenario_id}' does not share the frozen initial resource state"));
                            }
                        } else {
                            initial_state_reference = Some(initial);
                        }
                    }
                    if let Some(contract_items) = contract.as_array() {
                        for spec in METRICS {
                            let item = metric(state,spec.metric);
                            let contract_item = contract_items.iter().find(|item| string_at(item,"metric") == Some(spec.metric));
                            if let (Some(item),Some(contract_item)) = (item,contract_item) {
                                let value = number_at(item,"value").unwrap_or(f64::NAN);
                                let lower = number_at(item,"lower_bound").unwrap_or(f64::NAN);
                                let upper = number_at(item,"upper_bound").unwrap_or(f64::NAN);
                                let floor = number_at(contract_item,"floor_value").unwrap_or(f64::NAN);
                                let coverage = number_at(item,"coverage_fraction").unwrap_or(0.0);
                                let minimum_coverage = number_at(contract_item,"minimum_coverage_fraction").unwrap_or(1.0);
                                let operator = string_at(contract_item,"operator").unwrap_or("");
                                let margin = if operator == "gte" {lower-floor} else {floor-upper};
                                if margin < -NUMERIC_TOLERANCE || coverage < minimum_coverage {
                                    let finding = json!({
                                        "scenario_id":scenario_id,
                                        "scenario_class":scenario_class,
                                        "time_index":index,
                                        "timestamp":timestamp,
                                        "metric":spec.metric,
                                        "unit":spec.unit,
                                        "value":value,
                                        "lower_bound":lower,
                                        "upper_bound":upper,
                                        "floor_operator":operator,
                                        "floor_value":floor,
                                        "conservative_margin":margin,
                                        "coverage_fraction":coverage,
                                        "minimum_coverage_fraction":minimum_coverage
                                    });
                                    if first_breach.is_none() {
                                        first_breach = Some(finding.clone());
                                    }
                                    scenario_breaches.push(finding);
                                }
                            }
                        }
                    }
                    let _ = metrics;
                }
            }

            match scenario_class {
                "nominal" | "adverse_forecast_error" => {
                    if state_components.iter().any(|states|
                        states.get("main_generator").map(String::as_str) != Some("available")
                        || states.get("watermaker").map(String::as_str) != Some("available"))
                    {
                        errors.push(format!("scenario '{scenario_id}' must keep main_generator and watermaker available"));
                    }
                }
                "main_generation_outage" => {
                    if state_components.iter().any(|states|
                        states.get("main_generator").map(String::as_str) != Some("unavailable")
                        || states.get("watermaker").map(String::as_str) != Some("available"))
                    {
                        errors.push(format!("scenario '{scenario_id}' must disable only the main generator for the full horizon"));
                    }
                }
                "watermaker_outage" => {
                    if state_components.iter().any(|states|
                        states.get("watermaker").map(String::as_str) != Some("unavailable")
                        || states.get("main_generator").map(String::as_str) != Some("available"))
                    {
                        errors.push(format!("scenario '{scenario_id}' must disable only the watermaker for the full horizon"));
                    }
                }
                "common_cause_generation_watermaker_outage" => {
                    let event_ids = scenario.get("states").and_then(Value::as_array).map(|states| {
                        states.iter().map(|state| {
                            let generator_event = component(state,"main_generator").and_then(|item| string_at(item,"event_id")).unwrap_or("");
                            let watermaker_event = component(state,"watermaker").and_then(|item| string_at(item,"event_id")).unwrap_or("");
                            if component(state,"main_generator").and_then(|item| string_at(item,"availability")) != Some("unavailable")
                                || component(state,"watermaker").and_then(|item| string_at(item,"availability")) != Some("unavailable")
                                || generator_event.is_empty()
                                || generator_event != watermaker_event
                            {
                                errors.push(format!("scenario '{scenario_id}' common-cause event must disable generation and watermaker with one shared event ID at every time step"));
                            }
                            generator_event.to_owned()
                        }).collect::<BTreeSet<_>>()
                    }).unwrap_or_default();
                    if event_ids.len() != 1 {
                        errors.push(format!("scenario '{scenario_id}' common-cause event identity changes over the horizon"));
                    }
                }
                _ => {}
            }

            let transitions = scenario.get("transitions").and_then(Value::as_array);
            if transitions.is_none_or(|items| items.len() + 1 != times.len()) {
                errors.push(format!("scenario '{scenario_id}' must contain exactly one ledger per adjacent time pair"));
            }
            let mut residuals = BTreeMap::<String,f64>::new();
            if let (Some(transitions),Some(states)) = (
                transitions,
                scenario.get("states").and_then(Value::as_array)
            ) {
                for (index,transition) in transitions.iter().enumerate() {
                    if transition.get("from_time_index").and_then(Value::as_u64) != Some(index as u64)
                        || transition.get("to_time_index").and_then(Value::as_u64) != Some(index as u64 + 1)
                    {
                        errors.push(format!("scenario '{scenario_id}' transition {index} does not connect adjacent ordered time points"));
                    }
                    if let (Some(from),Some(to)) = (states.get(index),states.get(index+1)) {
                        let energy_residual = check_energy_transition(transition,from,to,scenario_id,index,&mut errors);
                        let water_residual = check_freshwater_transition(transition,from,to,scenario_id,index,&mut errors);
                        let waste_residual = check_waste_transition(transition,from,to,scenario_id,index,&mut errors);
                        residuals.insert("energy_kwh".to_owned(),energy_residual.max(*residuals.get("energy_kwh").unwrap_or(&0.0)));
                        residuals.insert("freshwater_l".to_owned(),water_residual.max(*residuals.get("freshwater_l").unwrap_or(&0.0)));
                        residuals.insert("waste_margin_days".to_owned(),waste_residual.max(*residuals.get("waste_margin_days").unwrap_or(&0.0)));
                    }
                }
            }
            for (key,residual) in &residuals {
                let current = maximum_balance_residuals.get(key).copied().unwrap_or(0.0);
                maximum_balance_residuals.insert(key.clone(),current.max(*residual));
            }
            let viable = scenario_breaches.is_empty()
                && !errors.iter().any(|error| error.contains(scenario_id));
            scenario_reports.push(json!({
                "scenario_id":scenario_id,
                "scenario_class":scenario_class,
                "viable":viable,
                "first_breach":first_breach,
                "breach_count":scenario_breaches.len(),
                "minimum_conservative_margin_by_resource":margins.iter().map(|(metric,(margin,timestamp))| json!({
                    "metric":metric,"minimum_margin":margin,"time_of_minimum_margin":timestamp
                })).collect::<Vec<_>>(),
                "maximum_balance_residuals":residuals
            }));
        }
    }
    let actual_classes = seen_classes.iter().cloned().collect::<Vec<_>>();
    if actual_classes.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario records do not exactly cover the mandatory scenario classes".to_owned());
    }
    if initial_digest_observed.as_deref() != string_at(snapshot,"initial_resource_state_sha256") {
        errors.push("initial_resource_state_sha256 does not match canonical first-step resource state".to_owned());
    }

    let blocked = !blockers.is_empty();
    let invalid = !errors.is_empty();
    let disposition = if invalid {
        "INVALID"
    } else if blocked {
        "BLOCKED"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "TRAJECTORY_AUDITED_READ_ONLY"
    };
    let input_sha = sha256_hex(raw);
    json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":disposition,
        "candidate_disposition":if invalid || blocked {"INFEASIBLE/ABSTAIN"} else {"SAMPLED_TRAJECTORY_FEASIBLE"},
        "input_sha256":input_sha,
        "scenario_count":scenario_reports.len(),
        "scenario_class_coverage":actual_classes,
        "time_step_count":times.len(),
        "scenario_reports":scenario_reports,
        "maximum_balance_residuals_across_scenarios":maximum_balance_residuals,
        "findings":{"errors":errors,"blockers":blockers},
        "aggregate_scalar_score":"NOT_COMPUTED",
        "claim_ceiling":"finite_horizon_sampled_trajectory_audit_not_mathematical_viability_kernel_not_operational_authorization",
        "note":"Every declared scenario/time point is checked. This is a sampled finite-horizon audit; it does not prove inter-sample safety, recursive feasibility, physical model validity, probability calibration or a viability kernel."
    })
}

fn run() -> Result<(),String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 || args[1] != "audit-trajectory" {
        return Err("Usage: marine-trajectory-viability audit-trajectory <fixture.json>".to_owned());
    }
    let raw = fs::read(&args[2]).map_err(|error|format!("{}: {error}",args[2]))?;
    let value: Value = serde_json::from_slice(&raw).map_err(|error|format!("{}: invalid JSON: {error}",args[2]))?;
    let report = validate(&raw,&value);
    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
    match string_at(&report,"disposition") {
        Some("SYNTHETIC_ONLY" | "TRAJECTORY_AUDITED_READ_ONLY" | "BLOCKED") => Ok(()),
        Some(disposition) => Err(format!("trajectory audit disposition: {disposition}")),
        None => Err("trajectory report has no disposition".to_owned())
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
        serde_json::from_str(include_str!("../fixtures/synthetic-trajectory-pass-0001.json"))
            .expect("synthetic trajectory fixture parses")
    }

    fn breach_fixture() -> Value {
        serde_json::from_str(include_str!("../fixtures/synthetic-arrival-safe-midvoyage-breach-0001.json"))
            .expect("synthetic mid-voyage breach fixture parses")
    }

    fn audit(value:&Value) -> Value {
        let raw=serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw,value)
    }

    #[test]
    fn all_required_synthetic_scenarios_pass_the_sampled_horizon() {
        let report=audit(&fixture());
        assert_eq!(report["disposition"],"SYNTHETIC_ONLY");
        assert_eq!(report["candidate_disposition"],"SAMPLED_TRAJECTORY_FEASIBLE");
        assert_eq!(report["scenario_count"],5);
        assert_eq!(report["time_step_count"],3);
        assert_eq!(report["aggregate_scalar_score"],"NOT_COMPUTED");
    }

    #[test]
    fn arrival_safe_plan_with_intermediate_water_floor_breach_is_blocked() {
        let report=audit(&breach_fixture());
        assert_eq!(report["disposition"],"BLOCKED");
        assert_eq!(report["candidate_disposition"],"INFEASIBLE/ABSTAIN");
        let scenario=report["scenario_reports"].as_array().unwrap().iter()
            .find(|item|item["scenario_class"]=="adverse_forecast_error").unwrap();
        assert_eq!(scenario["viable"],false);
        assert_eq!(scenario["first_breach"]["time_index"],1);
        assert_eq!(scenario["first_breach"]["metric"],"freshwater_usable_l");
    }

    #[test]
    fn missing_required_disturbance_scenario_is_invalid() {
        let mut value=fixture();
        value["scenarios"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn generator_output_is_zero_during_generation_outage() {
        let mut value=fixture();
        let scenario=value["scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|item|item["scenario_class"]=="main_generation_outage").unwrap();
        scenario["transitions"][0]["energy"]["generated_kwh"]=json!(1.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn water_production_is_zero_during_watermaker_outage() {
        let mut value=fixture();
        let scenario=value["scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|item|item["scenario_class"]=="watermaker_outage").unwrap();
        scenario["transitions"][0]["freshwater"]["produced_l"]=json!(10.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn common_cause_failure_uses_one_shared_event_for_both_components() {
        let mut value=fixture();
        let scenario=value["scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|item|item["scenario_class"]=="common_cause_generation_watermaker_outage").unwrap();
        scenario["states"][1]["components"][1]["event_id"]=json!("different-event-id");
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn initial_resource_snapshot_must_match_across_scenarios() {
        let mut value=fixture();
        let scenario=value["scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|item|item["scenario_class"]=="watermaker_outage").unwrap();
        scenario["states"][0]["resource_metrics"][1]["value"]=json!(150.0);
        scenario["states"][0]["resource_metrics"][1]["lower_bound"]=json!(149.9);
        scenario["states"][0]["resource_metrics"][1]["upper_bound"]=json!(150.1);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn time_grid_cannot_skip_or_reorder_a_state_index() {
        let mut value=fixture();
        value["scenarios"][0]["states"][1]["time_index"]=json!(2);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn energy_conservation_residual_is_checked_independently() {
        let mut value=fixture();
        value["scenarios"][0]["transitions"][0]["energy"]["end_kwh"]=json!(19.0);
        value["scenarios"][0]["states"][1]["resource_metrics"][0]["value"]=json!(19.0);
        value["scenarios"][0]["states"][1]["resource_metrics"][0]["lower_bound"]=json!(18.9);
        value["scenarios"][0]["states"][1]["resource_metrics"][0]["upper_bound"]=json!(19.1);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn conservative_bound_fails_even_when_point_estimate_passes() {
        let mut value=fixture();
        value["scenarios"][1]["states"][1]["resource_metrics"][1]["value"]=json!(81.0);
        value["scenarios"][1]["states"][1]["resource_metrics"][1]["lower_bound"]=json!(79.0);
        value["scenarios"][1]["states"][1]["resource_metrics"][1]["upper_bound"]=json!(82.0);
        value["scenarios"][1]["transitions"][0]["freshwater"]["consumed_l"]=json!(79.0);
        value["scenarios"][1]["transitions"][0]["freshwater"]["end_l"]=json!(81.0);
        value["scenarios"][1]["transitions"][1]["freshwater"]["start_l"]=json!(81.0);
        value["scenarios"][1]["transitions"][1]["freshwater"]["replenished_l"]=json!(29.0);
        assert_eq!(audit(&value)["disposition"],"BLOCKED");
    }

    #[test]
    fn common_cause_scenario_requires_both_outages() {
        let mut value=fixture();
        let scenario=value["scenarios"].as_array_mut().unwrap().iter_mut()
            .find(|item|item["scenario_class"]=="common_cause_generation_watermaker_outage").unwrap();
        scenario["states"][0]["components"][1]["availability"]=json!("available");
        scenario["states"][0]["components"][1]["event_id"]=json!("none");
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }
}
