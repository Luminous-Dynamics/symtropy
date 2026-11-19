use chrono::{DateTime, Duration, FixedOffset};
use serde_json::{json, Value};
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

fn digest_valid(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect::<String>()
}

fn canonical_value_digest(value: &Value) -> Option<String> {
    let bytes = serde_json::to_vec(value).ok()?;
    Some(sha256_hex(&bytes))
}

fn parse_time(value: Option<&str>) -> Option<DateTime<FixedOffset>> {
    value.and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
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
    Some(sha256_hex(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n").as_bytes()))
}

fn contract_digest(contract: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = contract.as_array()?;
    if items.len() != METRICS.len() {
        errors.push("service contract must declare exactly eleven resource axes".to_owned());
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
            errors.push(format!("service floor '{metric}' has invalid floor or coverage"));
            continue;
        };
        let Some(spec) = METRICS.iter().find(|spec| spec.metric == metric) else {
            errors.push(format!("service contract contains unknown metric '{metric}'"));
            continue;
        };
        if !seen.insert(metric.to_owned()) || unit != spec.unit || operator != spec.operator
            || !(0.0..=1.0).contains(&coverage)
        {
            errors.push(format!("service contract metric '{metric}' is duplicated or has invalid unit/operator/coverage"));
            continue;
        }
        rows.push((metric.to_owned(), format!("{metric}|{unit}|{operator}|{floor:.17}|{coverage:.17}")));
    }
    for spec in METRICS {
        if !seen.contains(spec.metric) {
            errors.push(format!("service contract is missing '{}'", spec.metric));
        }
    }
    rows.sort_by(|left, right| left.0.cmp(&right.0));
    Some(sha256_hex(rows.into_iter().map(|(_, row)| row).collect::<Vec<_>>().join("\n").as_bytes()))
}

fn roster_digest(roster: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = roster.as_array()?;
    let mut classes = Vec::<String>::new();
    let mut seen = BTreeSet::<String>::new();
    if items.len() != SCENARIO_CLASSES.len() {
        errors.push("scenario roster must contain exactly five required classes".to_owned());
    }
    for item in items {
        let class = string_at(item, "scenario_class").unwrap_or("");
        if !SCENARIO_CLASSES.contains(&class) || !seen.insert(class.to_owned())
            || !digest_valid(string_at(item, "scenario_sha256"))
            || !digest_valid(string_at(item, "disturbance_sha256"))
        {
            errors.push(format!("scenario roster entry '{class}' is unknown, duplicated or lacks valid digests"));
        }
        classes.push(class.to_owned());
    }
    classes.sort();
    if classes.iter().map(String::as_str).collect::<Vec<_>>().as_slice() != SCENARIO_CLASSES.as_slice() {
        errors.push("scenario roster does not exactly match the mandatory five-class set".to_owned());
    }
    Some(sha256_hex(classes.join("\n").as_bytes()))
}

#[derive(Clone)]
struct ActionReport {
    action_id: String,
    role: String,
    priority: u64,
    action_sha256: String,
    permit_expires_at: String,
    accepted: bool,
    blockers: Vec<String>,
}

fn evaluate_action(
    action: &Value,
    decision_time: DateTime<FixedOffset>,
    state_sha: &str,
    runtime_model: &str,
    runtime_domain: &str,
    permit_ttl_ms: i64,
    contract: &Value,
    roster_classes: &BTreeSet<String>,
    record_status: &str,
    errors: &mut Vec<String>,
) -> Option<ActionReport> {
    let action_id = string_at(action, "action_id").unwrap_or("").to_owned();
    let role = string_at(action, "role").unwrap_or("").to_owned();
    let priority = action.get("priority_rank").and_then(Value::as_u64).unwrap_or(0);
    let mut blockers = Vec::<String>::new();
    if action_id.is_empty() || priority == 0 || !["primary", "fallback"].contains(&role.as_str()) {
        errors.push("action candidate has missing identity, priority or role".to_owned());
    }
    let action_value = action.get("action").unwrap_or(&Value::Null);
    let calculated_action_sha = canonical_value_digest(action_value).unwrap_or_default();
    if !valid_digest(string_at(action, "action_sha256"))
        || string_at(action, "action_sha256") != Some(calculated_action_sha.as_str())
    {
        errors.push(format!("action '{action_id}' action_sha256 does not bind its exact proposed control values"));
    }
    if string_at(action, "source_current_state_sha256") != Some(state_sha) {
        errors.push(format!("action '{action_id}' was proposed against a different current state"));
    }
    if string_at(action, "model_sha256") != Some(runtime_model)
        || string_at(action, "operating_domain_sha256") != Some(runtime_domain)
    {
        errors.push(format!("action '{action_id}' model/operating-domain identity does not match the runtime contract"));
    }
    let expiry = parse_time(string_at(action, "permit_expires_at"));
    match expiry {
        Some(expiry) if expiry > decision_time && expiry <= decision_time + Duration::milliseconds(permit_ttl_ms) => {}
        _ => errors.push(format!("action '{action_id}' permit expiry is invalid or exceeds the configured TTL")),
    }

    let successors = action.get("successors").and_then(Value::as_array);
    if successors.is_none_or(|items| items.len() != roster_classes.len()) {
        errors.push(format!("action '{action_id}' must provide exactly one successor for every required scenario"));
        return None;
    }
    let mut seen = BTreeSet::<String>::new();
    for successor in successors.unwrap() {
        let scenario_class = string_at(successor, "scenario_class").unwrap_or("");
        if !roster_classes.contains(scenario_class) || !seen.insert(scenario_class.to_owned()) {
            errors.push(format!("action '{action_id}' has unknown or duplicated successor scenario '{scenario_class}'"));
            continue;
        }
        let successor_body = {
            let mut body = successor.clone();
            body.as_object_mut().and_then(|object| object.remove("successor_state_sha256"));
            body
        };
        let calculated_successor_sha = canonical_value_digest(&successor_body).unwrap_or_default();
        if !valid_digest(string_at(successor, "successor_state_sha256"))
            || string_at(successor, "successor_state_sha256") != Some(calculated_successor_sha.as_str())
        {
            errors.push(format!("action '{action_id}' successor for '{scenario_class}' has a stale state digest"));
        }

        let metrics = successor.get("resource_metrics").and_then(Value::as_array);
        if metrics.is_none_or(|items| items.len() != METRICS.len()) {
            errors.push(format!("action '{action_id}' successor '{scenario_class}' must report all eleven axes"));
            continue;
        }
        let mut seen_metrics = BTreeSet::<String>::new();
        for item in metrics.unwrap() {
            let metric_id = string_at(item, "metric").unwrap_or("");
            let unit = string_at(item, "unit").unwrap_or("");
            let Some(spec) = METRICS.iter().find(|spec| spec.metric == metric_id) else {
                errors.push(format!("action '{action_id}' successor '{scenario_class}' has unknown metric '{metric_id}'"));
                continue;
            };
            if !seen_metrics.insert(metric_id.to_owned()) {
                errors.push(format!("action '{action_id}' successor '{scenario_class}' duplicates metric '{metric_id}'"));
                continue;
            }
            let (Some(value), Some(lower), Some(upper), Some(coverage)) = (
                number_at(item,"value"), number_at(item,"lower_bound"),
                number_at(item,"upper_bound"), number_at(item,"coverage_fraction")
            ) else {
                errors.push(format!("action '{action_id}' successor '{scenario_class}' metric '{metric_id}' is incomplete"));
                continue;
            };
            let evidence_class = string_at(item, "measurement_class").unwrap_or("");
            if unit != spec.unit || lower < 0.0 || lower > value || value > upper
                || !(0.0..=1.0).contains(&coverage)
                || string_at(item,"measurement_method").is_none_or(str::is_empty)
                || !valid_digest(string_at(item,"evidence_sha256"))
                || !["measured","modeled","derived","synthetic"].contains(&evidence_class)
                || (record_status == "synthetic" && evidence_class != "synthetic")
                || (record_status != "synthetic" && evidence_class == "synthetic")
            {
                errors.push(format!("action '{action_id}' successor '{scenario_class}' metric '{metric_id}' has invalid unit, bound, evidence or measurement class"));
                continue;
            }
            let contract_item = contract.as_array().and_then(|items| items.iter().find(|entry| string_at(entry,"metric")==Some(metric_id)));
            let Some(contract_item) = contract_item else {
                errors.push(format!("contract is missing metric '{metric_id}'"));
                continue;
            };
            let floor = number_at(contract_item,"floor_value").unwrap_or(f64::NAN);
            let min_coverage = number_at(contract_item,"minimum_coverage_fraction").unwrap_or(1.0);
            let operator = string_at(contract_item,"operator").unwrap_or("");
            let margin = match operator { "gte" => lower-floor, "lte" => floor-upper, _ => f64::NEG_INFINITY };
            if coverage < min_coverage {
                blockers.push(format!("{scenario_class}/{metric_id}: coverage {coverage:.3} below {min_coverage:.3}"));
            }
            if margin < -1e-9 {
                blockers.push(format!("{scenario_class}/{metric_id}: conservative margin {margin:.3} {unit} breaches {operator} {floor}"));
            }
        }
        for spec in METRICS {
            if !seen_metrics.contains(spec.metric) {
                errors.push(format!("action '{action_id}' successor '{scenario_class}' missing '{}'",spec.metric));
            }
        }
    }
    if seen != *roster_classes {
        errors.push(format!("action '{action_id}' successor set does not exactly cover the scenario roster"));
    }
    Some(ActionReport {
        action_id,
        role,
        priority,
        action_sha256: calculated_action_sha,
        permit_expires_at: string_at(action,"permit_expires_at").unwrap_or("").to_owned(),
        accepted: blockers.is_empty(),
        blockers,
    })
}

fn validate(raw: &[u8], value: &Value) -> Value {
    let mut errors = Vec::<String>::new();
    let mut global_blockers = Vec::<String>::new();
    if string_at(value,"schema_version") != Some("0.1.0") {
        errors.push("schema_version must be 0.1.0".to_owned());
    }
    if string_at(value,"decision_id").is_none_or(str::is_empty) {
        errors.push("decision_id is required".to_owned());
    }
    if string_at(value,"claim_ceiling") != Some("one_step_model_output_action_gate_only") {
        errors.push("claim_ceiling must remain one-step and model-output scoped".to_owned());
    }
    let record_status = string_at(value,"record_status").unwrap_or("");
    if !["synthetic","shadow_replay","measured_campaign","published_reproduction"].contains(&record_status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }
    let decision_time = parse_time(string_at(value,"decision_time"));
    if decision_time.is_none() {
        errors.push("decision_time must be RFC3339".to_owned());
    }
    let runtime = value.get("runtime_contract").unwrap_or(&Value::Null);
    for field in ["transition_model_sha256","operating_domain_sha256","shield_version_sha256"] {
        if !valid_digest(string_at(runtime,field)) {
            errors.push(format!("runtime contract has invalid {field}"));
        }
    }
    let max_age = runtime.get("max_state_age_ms").and_then(Value::as_i64).filter(|v| *v > 0);
    let ttl = runtime.get("permit_ttl_ms").and_then(Value::as_i64).filter(|v| *v > 0);
    if max_age.is_none() || ttl.is_none() {
        errors.push("runtime contract requires positive maximum state age and permit TTL".to_owned());
    }

    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    let snapshot_hash = snapshot_digest(snapshot);
    if snapshot_hash.as_deref() != string_at(snapshot,"mission_snapshot_sha256")
        || !digest_valid(snapshot_hash.as_deref())
    {
        errors.push("mission_snapshot_sha256 does not bind the canonical mission snapshot".to_owned());
    }
    for field in [
        "craft_configuration_sha256","occupancy_sha256","initial_resource_state_sha256",
        "mission_plan_sha256","service_contract_sha256","scenario_roster_sha256",
        "time_grid_sha256","forecast_information_set_sha256","hard_gate_roster_sha256",
        "operating_envelope_sha256",
    ] {
        if !digest_valid(string_at(snapshot,field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }
    if string_at(runtime,"operating_domain_sha256") != string_at(snapshot,"operating_envelope_sha256") {
        errors.push("runtime operating-domain digest differs from frozen mission operating envelope".to_owned());
    }

    let contract = value.get("service_contract").unwrap_or(&Value::Null);
    let calculated_contract = contract_digest(contract,&mut errors);
    if calculated_contract.as_deref() != string_at(snapshot,"service_contract_sha256") {
        errors.push("service contract digest does not match mission snapshot".to_owned());
    }
    let roster = value.get("scenario_roster").unwrap_or(&Value::Null);
    let calculated_roster = roster_digest(roster,&mut errors);
    if calculated_roster.as_deref() != string_at(snapshot,"scenario_roster_sha256") {
        errors.push("scenario roster digest does not match mission snapshot".to_owned());
    }
    let roster_classes = roster.as_array().map(|items| items.iter()
        .filter_map(|item| string_at(item,"scenario_class").map(str::to_owned))
        .collect::<BTreeSet<_>>()).unwrap_or_default();

    let current_state = value.get("current_state").unwrap_or(&Value::Null);
    let state_sequence = current_state.get("sequence").and_then(Value::as_u64);
    let state_sha = string_at(current_state,"state_sha256").unwrap_or("");
    if state_sequence.is_none() || !valid_digest(Some(state_sha))
        || !valid_digest(string_at(current_state,"telemetry_evidence_sha256"))
    {
        errors.push("current state requires sequence, state SHA and telemetry evidence SHA".to_owned());
    }
    let mut state_stale = false;
    match (
        decision_time.clone(),
        parse_time(string_at(current_state,"timestamp")),
        max_age,
    ) {
        (Some(now),Some(timestamp),Some(max_age_ms)) => {
            let age = now.signed_duration_since(timestamp).num_milliseconds();
            if age < 0 || age > max_age_ms {
                state_stale = true;
                global_blockers.push(format!("current state age {age}ms is outside allowed 0..={max_age_ms}ms"));
            }
        }
        _ => errors.push("current state timestamp must be RFC3339".to_owned()),
    }

    let actions = value.get("action_candidates").and_then(Value::as_array);
    if actions.is_none_or(Vec::is_empty) {
        errors.push("at least one action candidate is required".to_owned());
    }
    let mut reports = Vec::<ActionReport>::new();
    let mut ids = BTreeSet::<String>::new();
    let mut ranks = BTreeSet::<u64>::new();
    let mut primary_ids = BTreeSet::<String>::new();
    if let (Some(actions),Some(now),Some(ttl_ms)) = (actions,decision_time,ttl) {
        let mut ordered = actions.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|action| action.get("priority_rank").and_then(Value::as_u64).unwrap_or(u64::MAX));
        for action in &ordered {
            let id = string_at(action,"action_id").unwrap_or("").to_owned();
            let role = string_at(action,"role").unwrap_or("");
            let priority = action.get("priority_rank").and_then(Value::as_u64).unwrap_or(0);
            if id.is_empty() || !ids.insert(id.clone()) {
                errors.push(format!("action candidate ID '{id}' is missing or duplicated"));
            }
            if priority == 0 || !ranks.insert(priority) {
                errors.push(format!("action '{id}' has zero or duplicated priority rank"));
            }
            if role == "primary" {
                primary_ids.insert(id);
            }
            if let Some(report) = evaluate_action(
                action,now,state_sha,
                string_at(runtime,"transition_model_sha256").unwrap_or(""),
                string_at(runtime,"operating_domain_sha256").unwrap_or(""),
                ttl_ms,contract,&roster_classes,record_status,&mut errors
            ) {
                reports.push(report);
            }
        }
    }
    if primary_ids.len() != 1 {
        errors.push("exactly one primary action proposal is required".to_owned());
    }
    if let Some(actions) = actions {
        if let Some(primary_id) = primary_ids.iter().next() {
            for action in actions {
                let role = string_at(action,"role").unwrap_or("");
                if role == "fallback" && string_at(action,"fallback_for_action_id") != Some(primary_id.as_str()) {
                    errors.push(format!("fallback action '{}' must identify the primary action it protects",string_at(action,"action_id").unwrap_or("unknown")));
                }
                if role == "primary" && action.get("fallback_for_action_id").is_some_and(|v| !v.is_null()) {
                    errors.push("primary action must not declare fallback_for_action_id".to_owned());
                }
            }
        }
    }

    reports.sort_by_key(|report| report.priority);
    let selected = if !state_stale && errors.is_empty() && global_blockers.is_empty() {
        reports.iter().find(|report| report.accepted)
    } else {
        None
    };
    let action_reports = reports.iter().map(|report| json!({
        "action_id":report.action_id,
        "role":report.role,
        "priority_rank":report.priority,
        "action_sha256":report.action_sha256,
        "accepted":report.accepted,
        "decision":if report.accepted {"PERMIT_CANDIDATE"} else {"REJECT"},
        "blocking_findings":report.blockers
    })).collect::<Vec<_>>();

    let mut permit = Value::Null;
    if let Some(selected) = selected {
        let body = json!({
            "decision_id":string_at(value,"decision_id"),
            "permit_nonce":string_at(value,"decision_id"),
            "action_id":selected.action_id,
            "action_sha256":selected.action_sha256,
            "source_state_sequence":state_sequence,
            "source_state_sha256":state_sha,
            "transition_model_sha256":string_at(runtime,"transition_model_sha256"),
            "operating_domain_sha256":string_at(runtime,"operating_domain_sha256"),
            "service_contract_sha256":string_at(snapshot,"service_contract_sha256"),
            "scenario_roster_sha256":string_at(snapshot,"scenario_roster_sha256"),
            "expires_at":selected.permit_expires_at,
            "single_use_required":true,
            "actuation_performed":false
        });
        let receipt_sha = canonical_value_digest(&body).unwrap_or_default();
        permit = json!({"receipt":body,"receipt_sha256":receipt_sha});
    }

    let decision = if !errors.is_empty() {
        "INVALID"
    } else if state_stale || !global_blockers.is_empty() || selected.is_none() {
        "ABSTAIN"
    } else {
        "PERMIT"
    };
    let disposition = if !errors.is_empty() {
        "INVALID"
    } else if record_status == "synthetic" {
        "SYNTHETIC_ONLY"
    } else {
        "SHIELD_AUDITED_READ_ONLY"
    };
    json!({
        "engine_version":ENGINE_VERSION,
        "decision_id":string_at(value,"decision_id"),
        "disposition":disposition,
        "decision":decision,
        "selected_action_id":selected.map(|action| action.action_id.as_str()),
        "action_reports":action_reports,
        "permit":permit,
        "actuation_performed":false,
        "executor_responsibility":"persist_and_consume permit_nonce exactly once; reconcile unknown execution outcome before any new action",
        "findings":{"errors":errors,"global_blockers":global_blockers},
        "claim_ceiling":"one_step_model_output_gate_not_closed_loop_safety_proof_not_navigation_authorization",
        "note":"This read-only gate checks declared model-predicted one-step successor envelopes. It does not validate the transition model or prove recursive feasibility, inter-sample safety, controller stability, probability calibration or physical vessel safety. No actuator interface is exposed."
    })
}

fn run() -> Result<(),String> {
    let args=env::args().collect::<Vec<_>>();
    if args.len()!=3 || args[1]!="audit-action-set" {
        return Err("Usage: marine-runtime-action-shield audit-action-set <proposal.json>".to_owned());
    }
    let raw=fs::read(&args[2]).map_err(|error|format!("{}: {error}",args[2]))?;
    let value:Value=serde_json::from_slice(&raw).map_err(|error|format!("{}: invalid JSON: {error}",args[2]))?;
    let report=validate(&raw,&value);
    println!("{}",serde_json::to_string_pretty(&report).map_err(|error|error.to_string())?);
    match string_at(&report,"disposition") {
        Some("SYNTHETIC_ONLY" | "SHIELD_AUDITED_READ_ONLY" | "INVALID")=>Ok(()),
        Some(disposition)=>Err(format!("action shield disposition: {disposition}")),
        None=>Err("action shield report has no disposition".to_owned())
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
        serde_json::from_str(include_str!("../fixtures/synthetic-primary-rejected-fallback-permitted-0001.json"))
            .expect("synthetic action shield fixture parses")
    }

    fn audit(value:&Value)->Value {
        let raw=serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw,value)
    }

    #[test]
    fn rejects_unsafe_primary_and_permits_first_safe_fallback() {
        let report=audit(&fixture());
        assert_eq!(report["disposition"],"SYNTHETIC_ONLY");
        assert_eq!(report["decision"],"PERMIT");
        assert_eq!(report["selected_action_id"],"preserve-essential-reserves");
        assert_eq!(report["action_reports"][0]["decision"],"REJECT");
        assert_eq!(report["action_reports"][1]["decision"],"PERMIT_CANDIDATE");
        assert_eq!(report["actuation_performed"],false);
    }

    #[test]
    fn primary_is_preferred_when_all_scenarios_pass() {
        let mut value=fixture();
        for successor in value["action_candidates"][0]["successors"].as_array_mut().unwrap() {
            for metric in successor["resource_metrics"].as_array_mut().unwrap() {
                if metric["metric"]=="energy_reserve_kwh" {
                    metric["value"]=json!(8.0);metric["lower_bound"]=json!(7.9);metric["upper_bound"]=json!(8.1);
                }
                if metric["metric"]=="freshwater_usable_l" {
                    metric["value"]=json!(110.0);metric["lower_bound"]=json!(109.9);metric["upper_bound"]=json!(110.1);
                }
            }
            let mut body=successor.clone();
            body.as_object_mut().unwrap().remove("successor_state_sha256");
            successor["successor_state_sha256"]=json!(canonical_value_digest(&body).unwrap());
        }
        let action=value["action_candidates"][0]["action"].clone();
        value["action_candidates"][0]["action_sha256"]=json!(canonical_value_digest(&action).unwrap());
        assert_eq!(audit(&value)["selected_action_id"],"maintain-cruise");
    }

    #[test]
    fn no_safe_candidate_returns_abstain() {
        let mut value=fixture();
        for successor in value["action_candidates"][1]["successors"].as_array_mut().unwrap() {
            for metric in successor["resource_metrics"].as_array_mut().unwrap() {
                if metric["metric"]=="freshwater_usable_l" {
                    metric["value"]=json!(50.0);metric["lower_bound"]=json!(49.9);metric["upper_bound"]=json!(50.1);
                }
            }
            let mut body=successor.clone();
            body.as_object_mut().unwrap().remove("successor_state_sha256");
            successor["successor_state_sha256"]=json!(canonical_value_digest(&body).unwrap());
        }
        assert_eq!(audit(&value)["decision"],"ABSTAIN");
        assert_eq!(audit(&value)["permit"],Value::Null);
    }

    #[test]
    fn stale_telemetry_state_forces_abstention() {
        let mut value=fixture();
        value["current_state"]["timestamp"]=json!("2026-03-01T11:00:00Z");
        assert_eq!(audit(&value)["decision"],"ABSTAIN");
    }

    #[test]
    fn replayed_or_expired_action_permit_deadline_is_invalid() {
        let mut value=fixture();
        value["action_candidates"][0]["permit_expires_at"]=json!("2026-03-01T12:00:05Z");
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn action_digest_must_bind_exact_control_values() {
        let mut value=fixture();
        value["action_candidates"][1]["action"]["propulsion_load_kw"]=json!(1.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn mission_contract_digest_must_match_runtime_snapshot() {
        let mut value=fixture();
        value["service_contract"][0]["floor_value"]=json!(3.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn successor_evidence_must_use_the_exact_required_scenario_class() {
        let mut value=fixture();
        value["action_candidates"][1]["successors"][0]["scenario_class"]=json!("unregistered-scenario");
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn candidate_model_must_match_runtime_model() {
        let mut value=fixture();
        value["action_candidates"][1]["model_sha256"]=json!("a".repeat(64));
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn current_state_digest_must_match_candidate_source_state() {
        let mut value=fixture();
        value["action_candidates"][1]["source_current_state_sha256"]=json!("a".repeat(64));
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn omitted_disturbance_successor_is_invalid_not_implicitly_safe() {
        let mut value=fixture();
        value["action_candidates"][1]["successors"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn successor_digest_must_bind_the_predicted_resource_envelope() {
        let mut value=fixture();
        value["action_candidates"][1]["successors"][0]["resource_metrics"][0]["value"]=json!(999.0);
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }

    #[test]
    fn fallback_must_name_the_primary_it_protects() {
        let mut value=fixture();
        value["action_candidates"][1]["fallback_for_action_id"]=json!("unknown-primary");
        assert_eq!(audit(&value)["disposition"],"INVALID");
    }
}
