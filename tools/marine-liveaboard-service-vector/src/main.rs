use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const ENGINE_VERSION: &str = "marine-liveaboard-service-vector/0.1.0";
const EXPECTED_CONTRACT_SHA256: &str =
    "2a6506cda1d96b4c6406ec84904fdf07bc2ca4d2b0a9bbfa0360fb7e9700a919";

#[derive(Clone, Copy)]
struct MetricSpec {
    metric: &'static str,
    unit: &'static str,
    operator: &'static str,
    floor: f64,
}

const METRICS: [MetricSpec; 10] = [
    MetricSpec { metric: "arrival_delay_hours", unit: "h", operator: "lte", floor: 5.0 },
    MetricSpec { metric: "energy_reserve_hours_at_arrival", unit: "h", operator: "gte", floor: 12.0 },
    MetricSpec { metric: "freshwater_reserve_days_at_arrival", unit: "day", operator: "gte", floor: 3.0 },
    MetricSpec { metric: "food_reserve_days_at_arrival", unit: "day", operator: "gte", floor: 7.0 },
    MetricSpec { metric: "waste_capacity_margin_days", unit: "day", operator: "gte", floor: 1.0 },
    MetricSpec { metric: "critical_spares_coverage_days", unit: "day", operator: "gte", floor: 3.0 },
    MetricSpec { metric: "essential_service_outage_minutes", unit: "min", operator: "lte", floor: 30.0 },
    MetricSpec { metric: "comfort_breach_minutes", unit: "min", operator: "lte", floor: 60.0 },
    MetricSpec { metric: "crew_workload_hours_per_day", unit: "h/day", operator: "lte", floor: 8.0 },
    MetricSpec { metric: "purchased_energy_kwh_per_berth_day", unit: "kWh/berth-day", operator: "lte", floor: 6.0 },
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

fn contract_canonical(value: &Value, errors: &mut Vec<String>) -> Option<String> {
    let items = value.as_array()?;
    let mut rows = Vec::<(String, String)>::new();
    let mut seen = BTreeSet::<String>::new();
    for item in items {
        let metric = string_at(item, "metric").unwrap_or("");
        let unit = string_at(item, "unit").unwrap_or("");
        let operator = string_at(item, "operator").unwrap_or("");
        let floor = number_at(item, "floor_value");
        let coverage = number_at(item, "minimum_coverage_fraction");
        let Some(floor) = floor else {
            errors.push(format!("service contract '{metric}' has a missing/non-finite floor"));
            continue;
        };
        let Some(coverage) = coverage else {
            errors.push(format!("service contract '{metric}' has missing coverage requirement"));
            continue;
        };
        if metric.is_empty() || unit.is_empty() || !seen.insert(metric.to_owned()) {
            errors.push("service floor contract has a missing or duplicated metric".to_owned());
            continue;
        }
        rows.push((
            metric.to_owned(),
            format!("{metric}|{unit}|{operator}|{floor:.17}|{coverage:.17}"),
        ));
    }
    if rows.len() != METRICS.len() {
        errors.push("service floor contract must define all ten required axes exactly once".to_owned());
    }
    for spec in METRICS {
        let Some(item) = items.iter().find(|item| string_at(item, "metric") == Some(spec.metric)) else {
            errors.push(format!("service floor contract is missing '{}'", spec.metric));
            continue;
        };
        if string_at(item, "unit") != Some(spec.unit)
            || string_at(item, "operator") != Some(spec.operator)
            || number_at(item, "floor_value") != Some(spec.floor)
        {
            errors.push(format!("service floor '{}' must use unit '{}', {} {}, and the canonical definition", spec.metric, spec.unit, spec.operator, spec.floor));
        }
        if number_at(item, "minimum_coverage_fraction").is_none_or(|coverage| !(0.0..=1.0).contains(&coverage)) {
            errors.push(format!("service floor '{}' has invalid coverage fraction", spec.metric));
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
    Some(sha256_hex(&fields.into_iter().map(|(_, field)| field).collect::<Vec<_>>().join("\n")))
}

#[derive(Clone)]
struct MetricValue {
    value: f64,
    unit: String,
}

#[derive(Clone)]
struct PolicyReport {
    policy_id: String,
    role: String,
    operating_envelope: String,
    feasible: bool,
    metrics: BTreeMap<String, MetricValue>,
    relaxation_count: usize,
}

fn policy_report(
    policy: &Value,
    contract: &Value,
    base_operating_envelope: &str,
    errors: &mut Vec<String>,
    blockers: &mut Vec<String>,
) -> Option<PolicyReport> {
    let policy_id = string_at(policy, "policy_id").unwrap_or("").to_owned();
    let role = string_at(policy, "role").unwrap_or("").to_owned();
    if policy_id.is_empty() {
        errors.push("policy_id is required".to_owned());
    }
    if !digest_valid(string_at(policy, "mission_snapshot_sha256"))
        || !digest_valid(string_at(policy, "decision_output_sha256"))
        || !digest_valid(string_at(policy, "operating_envelope_sha256"))
    {
        errors.push(format!("policy '{policy_id}' has invalid snapshot, output or operating-envelope digest"));
    }
    let expected_forecast_use = match role.as_str() {
        "weather_blind_reference" => Some(false),
        "weather_informed_resource_blind" | "integrated_candidate" | "authorized_soft_relaxation" => Some(true),
        _ => None,
    };
    if expected_forecast_use.is_none() {
        errors.push(format!("policy '{policy_id}' has unsupported policy role"));
    } else if policy.get("forecast_use").and_then(Value::as_bool) != expected_forecast_use {
        errors.push(format!("policy '{policy_id}' forecast_use does not match its declared role"));
    }

    let gate_disposition = string_at(policy, "hard_gate_disposition").unwrap_or("");
    let gate_violations = policy.get("hard_gate_violations").and_then(Value::as_u64);
    if gate_violations.is_none() {
        errors.push(format!("policy '{policy_id}' requires a non-negative integer hard_gate_violations count"));
    }
    if gate_disposition == "pass" && gate_violations != Some(0) {
        errors.push(format!("policy '{policy_id}' says hard gates pass but reports violations"));
    }
    if ["fail", "infeasible", "abstain"].contains(&gate_disposition) && gate_violations == Some(0) {
        errors.push(format!("policy '{policy_id}' has a non-pass gate disposition without any recorded violation"));
    }

    let relaxations = policy.get("soft_constraint_relaxations").and_then(Value::as_array);
    let relaxation_count = relaxations.map_or(0, Vec::len);
    let policy_envelope = string_at(policy, "operating_envelope_sha256").unwrap_or("");
    if let Some(relaxations) = relaxations {
        for (index, relaxation) in relaxations.iter().enumerate() {
            let constraint_id = string_at(relaxation, "constraint_id").unwrap_or("");
            let service_floor_relaxation = METRICS.iter().any(|spec| spec.metric == constraint_id);
            if string_at(relaxation, "constraint_class") != Some("soft")
                || service_floor_relaxation
                || constraint_id.is_empty()
                || string_at(relaxation, "unit").is_none_or(str::is_empty)
                || number_at(relaxation, "prior_limit").is_none()
                || number_at(relaxation, "new_limit").is_none()
                || string_at(relaxation, "authority_receipt_ref").is_none_or(str::is_empty)
                || !digest_valid(string_at(relaxation, "authority_receipt_sha256"))
                || !digest_valid(string_at(relaxation, "new_operating_envelope_sha256"))
            {
                errors.push(format!("policy '{policy_id}' relaxation[{index}] lacks explicit soft classification, limits, authority receipt or new-envelope digest"));
                continue;
            }
            if string_at(relaxation, "new_operating_envelope_sha256") != Some(policy_envelope) {
                errors.push(format!("policy '{policy_id}' relaxation[{index}] does not match the policy operating-envelope digest"));
            }
            if policy_envelope == base_operating_envelope {
                errors.push(format!("policy '{policy_id}' changes a limit without creating a distinct operating envelope"));
            }
        }
    }
    if relaxation_count == 0 && policy_envelope != base_operating_envelope {
        errors.push(format!("policy '{policy_id}' changes the operating envelope without a declared relaxation"));
    }

    let results = policy.get("resource_metrics").and_then(Value::as_array);
    if results.is_none_or(|items| items.len() != METRICS.len()) {
        errors.push(format!("policy '{policy_id}' must report exactly the ten resource/service axes"));
    }
    let mut values = BTreeMap::<String, MetricValue>::new();
    let mut feasible = gate_disposition == "pass" && gate_violations == Some(0);
    if let Some(results) = results {
        for item in results {
            let metric = string_at(item, "metric").unwrap_or("");
            let unit = string_at(item, "unit").unwrap_or("");
            let Some(spec) = METRICS.iter().find(|spec| spec.metric == metric) else {
                errors.push(format!("policy '{policy_id}' reports unknown resource axis '{metric}'"));
                continue;
            };
            let value = number_at(item, "value");
            let coverage = number_at(item, "coverage_fraction");
            if unit != spec.unit || !digest_valid(string_at(item, "evidence_sha256")) {
                errors.push(format!("policy '{policy_id}' axis '{metric}' has an incorrect unit or missing evidence digest"));
            }
            if value.is_none() || coverage.is_none_or(|coverage| !(0.0..=1.0).contains(&coverage)) {
                errors.push(format!("policy '{policy_id}' axis '{metric}' has invalid value or measurement coverage"));
                continue;
            }
            if values.insert(metric.to_owned(), MetricValue { value: value.unwrap(), unit: unit.to_owned() }).is_some() {
                errors.push(format!("policy '{policy_id}' duplicates resource axis '{metric}'"));
            }
            let contract_item = contract.as_array().and_then(|items| items.iter().find(|item| string_at(item, "metric") == Some(metric)));
            if let Some(contract_item) = contract_item {
                let minimum_coverage = number_at(contract_item, "minimum_coverage_fraction").unwrap_or(1.0);
                if coverage.unwrap() < minimum_coverage {
                    feasible = false;
                    blockers.push(format!("policy '{policy_id}' axis '{metric}' coverage {:.3} is below required {:.3}", coverage.unwrap(), minimum_coverage));
                }
                let floor = number_at(contract_item, "floor_value").unwrap_or(f64::NAN);
                let operator = string_at(contract_item, "operator").unwrap_or("");
                let passes = match operator {
                    "gte" => value.unwrap() >= floor,
                    "lte" => value.unwrap() <= floor,
                    _ => false,
                };
                if !passes {
                    feasible = false;
                    blockers.push(format!("policy '{policy_id}' axis '{metric}' breaches its non-compensatory floor"));
                }
            }
        }
    }
    for spec in METRICS {
        if !values.contains_key(spec.metric) {
            errors.push(format!("policy '{policy_id}' is missing resource axis '{}'", spec.metric));
        }
    }
    if gate_disposition != "pass" || gate_violations != Some(0) {
        feasible = false;
        if role == "integrated_candidate" {
            blockers.push(format!("candidate policy '{policy_id}' has non-passing hard-gate disposition"));
        }
    }
    if relaxation_count > 0 && role != "authorized_soft_relaxation" {
        feasible = false;
        if role == "integrated_candidate" {
            blockers.push(format!("candidate policy '{policy_id}' changes a soft constraint and must be evaluated as a distinct authorized policy arm"));
        } else {
            errors.push(format!("policy '{policy_id}' contains a relaxation but is not the authorized_soft_relaxation role"));
        }
    }
    if role == "authorized_soft_relaxation" && relaxation_count == 0 {
        errors.push(format!("authorized soft-relaxation policy '{policy_id}' must include at least one authority-bound relaxation"));
    }
    Some(PolicyReport {
        policy_id,
        role,
        operating_envelope: policy_envelope.to_owned(),
        feasible,
        metrics: values,
        relaxation_count,
    })
}

fn dominates(left: &PolicyReport, right: &PolicyReport) -> bool {
    let mut strict = false;
    for spec in METRICS {
        let (Some(a), Some(b)) = (left.metrics.get(spec.metric), right.metrics.get(spec.metric)) else {
            return false;
        };
        let no_worse = match spec.operator {
            "gte" => a.value >= b.value,
            "lte" => a.value <= b.value,
            _ => false,
        };
        let better = match spec.operator {
            "gte" => a.value > b.value,
            "lte" => a.value < b.value,
            _ => false,
        };
        if !no_worse {
            return false;
        }
        strict |= better;
    }
    strict
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
    if string_at(value, "claim_ceiling") != Some("non_compensatory_service_vector_comparison_only") {
        errors.push("claim_ceiling must remain non-compensatory".to_owned());
    }
    let status = string_at(value, "record_status").unwrap_or("");
    if !["synthetic", "shadow_replay", "measured_campaign", "published_reproduction"].contains(&status) {
        errors.push("record_status is missing or unsupported".to_owned());
    }

    let contract = value.get("service_floor_contract").unwrap_or(&Value::Null);
    let canonical_contract = contract_canonical(contract, &mut errors);
    if let Some(canonical) = canonical_contract {
        let calculated = sha256_hex(&canonical);
        if calculated != EXPECTED_CONTRACT_SHA256
            || string_at(value.get("mission_snapshot").unwrap_or(&Value::Null), "service_floor_contract_sha256")
                != Some(calculated.as_str())
        {
            errors.push("service_floor_contract_sha256 does not match the canonical service floor contract".to_owned());
        }
    }
    let snapshot = value.get("mission_snapshot").unwrap_or(&Value::Null);
    let snapshot_hash = snapshot_digest(snapshot);
    if snapshot_hash.is_none()
        || snapshot_hash.as_deref() != string_at(snapshot, "mission_snapshot_sha256")
        || !digest_valid(snapshot_hash.as_deref())
    {
        errors.push("mission_snapshot_sha256 does not match canonical sorted mission snapshot fields".to_owned());
    }
    for field in [
        "craft_configuration_sha256", "occupancy_sha256", "initial_resource_state_sha256",
        "mission_plan_sha256", "forecast_information_set_sha256", "hard_gate_roster_sha256",
        "operating_envelope_sha256", "service_floor_contract_sha256",
    ] {
        if !digest_valid(string_at(snapshot, field)) {
            errors.push(format!("mission snapshot has invalid {field}"));
        }
    }
    let base_envelope = string_at(snapshot, "operating_envelope_sha256").unwrap_or("");
    let policies = value.get("policies").and_then(Value::as_array);
    if policies.is_none_or(|items| items.len() < 3) {
        errors.push("at least three policy arms are required".to_owned());
    }
    let mut policy_reports = BTreeMap::<String, PolicyReport>::new();
    let mut roles = BTreeSet::<String>::new();
    if let Some(policies) = policies {
        for policy in policies {
            let policy_id = string_at(policy, "policy_id").unwrap_or("").to_owned();
            if !roles.insert(string_at(policy, "role").unwrap_or("").to_owned()) {
                errors.push("policy role is duplicated".to_owned());
            }
            if string_at(policy, "mission_snapshot_sha256") != string_at(snapshot, "mission_snapshot_sha256") {
                errors.push(format!("policy '{policy_id}' does not share the exact frozen mission snapshot"));
            }
            if let Some(report) = policy_report(policy, contract, base_envelope, &mut errors, &mut blockers) {
                if policy_reports.insert(report.policy_id.clone(), report).is_some() {
                    errors.push(format!("policy ID '{policy_id}' is duplicated"));
                }
            }
        }
    }
    for required_role in ["weather_blind_reference", "weather_informed_resource_blind", "integrated_candidate"] {
        if !roles.contains(required_role) {
            errors.push(format!("required policy arm '{required_role}' is missing"));
        }
    }

    let comparison = value.get("comparison").unwrap_or(&Value::Null);
    let baseline_id = string_at(comparison, "baseline_policy_id").unwrap_or("");
    let candidate_id = string_at(comparison, "candidate_policy_id").unwrap_or("");
    let baseline = policy_reports.get(baseline_id);
    let candidate = policy_reports.get(candidate_id);
    if baseline.is_none() || candidate.is_none() {
        errors.push("comparison baseline/candidate IDs must reference declared policies".to_owned());
    }
    let mut axis_deltas = Vec::<Value>::new();
    let mut candidate_dominates = false;
    let mut baseline_dominates = false;
    let mut incomparable = false;
    if let (Some(baseline), Some(candidate)) = (baseline, candidate) {
        candidate_dominates = dominates(candidate, baseline);
        baseline_dominates = dominates(baseline, candidate);
        incomparable = !candidate_dominates && !baseline_dominates;
        for spec in METRICS {
            if let (Some(base), Some(cand)) = (baseline.metrics.get(spec.metric), candidate.metrics.get(spec.metric)) {
                let delta = cand.value - base.value;
                let candidate_better = match spec.operator {
                    "gte" => delta > 0.0,
                    "lte" => delta < 0.0,
                    _ => false,
                };
                axis_deltas.push(json!({
                    "metric":spec.metric,
                    "unit":spec.unit,
                    "baseline_value":base.value,
                    "candidate_value":cand.value,
                    "candidate_minus_baseline":delta,
                    "candidate_direction":"candidate_better_if_delta_has_operator_direction",
                    "candidate_better_on_axis":candidate_better
                }));
            }
        }
        if !candidate.feasible {
            blockers.push(format!("integrated candidate '{candidate_id}' fails at least one service floor, coverage floor or hard gate"));
        }
    }
    let pareto = json!({
        "baseline_policy_id":baseline_id,
        "candidate_policy_id":candidate_id,
        "candidate_dominates_baseline":candidate_dominates,
        "baseline_dominates_candidate":baseline_dominates,
        "incomparable":incomparable,
        "axis_deltas":axis_deltas,
        "aggregate_scalar_score":"NOT_COMPUTED"
    });

    let report = json!({
        "engine_version":ENGINE_VERSION,
        "scorecard_id":string_at(value,"scorecard_id"),
        "disposition":if !errors.is_empty() {"INVALID"} else if !blockers.is_empty() {"BLOCKED"} else if status == "synthetic" {"SYNTHETIC_ONLY"} else {"SERVICE_VECTOR_AUDITED_READ_ONLY"},
        "input_sha256":sha256_hex(std::str::from_utf8(raw).unwrap_or("")),
        "policy_count":policy_reports.len(),
        "policies":policy_reports.values().map(|policy| json!({
            "policy_id":policy.policy_id,
            "role":policy.role,
            "operating_envelope_sha256":policy.operating_envelope,
            "feasible":policy.feasible,
            "relaxation_count":policy.relaxation_count
        })).collect::<Vec<_>>(),
        "pareto_comparison":pareto,
        "findings":{"errors":errors,"blockers":blockers},
        "claim_ceiling":"per_axis_floor_and_pareto_audit_only_no_aggregate_utility_no_operational_authorization"
    });
    report
}

fn run() -> Result<(), String> {
    let arguments = env::args().collect::<Vec<_>>();
    if arguments.len() != 3 || arguments[1] != "audit-service-vector" {
        return Err("Usage: marine-liveaboard-service-vector audit-service-vector <fixture.json>".to_owned());
    }
    let raw = fs::read(&arguments[2]).map_err(|error| format!("{}: {error}", arguments[2]))?;
    let value: Value = serde_json::from_slice(&raw)
        .map_err(|error| format!("{}: invalid JSON: {error}", arguments[2]))?;
    let report = validate(&raw, &value);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|error| error.to_string())?);
    match string_at(&report, "disposition") {
        Some("SYNTHETIC_ONLY" | "SERVICE_VECTOR_AUDITED_READ_ONLY") => Ok(()),
        Some(disposition) => Err(format!("service vector disposition: {disposition}")),
        None => Err("service vector report has no disposition".to_owned()),
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
        serde_json::from_str(include_str!("../fixtures/synthetic-liveaboard-service-vector-0001.json"))
            .expect("synthetic service vector fixture parses")
    }

    fn audit(value: &Value) -> Value {
        let raw = serde_json::to_vec(value).expect("fixture serializes");
        validate(&raw, value)
    }

    #[test]
    fn synthetic_vector_is_incomparable_without_scalarization() {
        let report = audit(&fixture());
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["policy_count"], 3);
        assert_eq!(report["pareto_comparison"]["incomparable"], true);
        assert_eq!(report["pareto_comparison"]["candidate_dominates_baseline"], false);
        assert_eq!(report["pareto_comparison"]["aggregate_scalar_score"], "NOT_COMPUTED");
    }

    #[test]
    fn candidate_below_freshwater_floor_is_blocked() {
        let mut value = fixture();
        value["policies"][2]["resource_metrics"][2]["value"] = json!(2.0);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn candidate_hard_gate_violation_is_blocked() {
        let mut value = fixture();
        value["policies"][2]["hard_gate_violations"] = json!(1);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn policy_mission_snapshot_must_match_exactly() {
        let mut value = fixture();
        value["policies"][2]["mission_snapshot_sha256"] = json!("f".repeat(64));
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn missing_resource_axis_is_invalid() {
        let mut value = fixture();
        value["policies"][2]["resource_metrics"].as_array_mut().unwrap().pop();
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn insufficient_measurement_coverage_blocks_candidate() {
        let mut value = fixture();
        value["policies"][2]["resource_metrics"][2]["coverage_fraction"] = json!(0.5);
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn unauthorized_or_incomplete_relaxation_is_invalid() {
        let mut value = fixture();
        value["policies"][2]["soft_constraint_relaxations"] = json!([{
            "constraint_class":"hard",
            "constraint_id":"freshwater-reserve",
            "prior_limit":3.0,
            "new_limit":2.0,
            "unit":"day",
            "authority_receipt_ref":"",
            "authority_receipt_sha256":"bad",
            "new_operating_envelope_sha256":"bad"
        }]);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn relaxation_on_integrated_candidate_requires_a_distinct_policy_arm() {
        let mut value = fixture();
        value["policies"][2]["soft_constraint_relaxations"] = json!([{
            "constraint_class":"soft",
            "constraint_id":"optional-comfort-preference",
            "prior_limit":45.0,
            "new_limit":60.0,
            "unit":"min",
            "authority_receipt_ref":"SYNTHETIC-AUTHORITY-RECEIPT",
            "authority_receipt_sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "new_operating_envelope_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }]);
        value["policies"][2]["operating_envelope_sha256"] =
            json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(audit(&value)["disposition"], "BLOCKED");
    }

    #[test]
    fn explicitly_authorized_soft_relaxation_is_a_separate_policy_arm() {
        let mut value = fixture();
        let mut policy = value["policies"][2].clone();
        policy["policy_id"] = json!("authorized-soft-relaxation-arm");
        policy["role"] = json!("authorized_soft_relaxation");
        policy["decision_output_sha256"] = json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        policy["operating_envelope_sha256"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        policy["soft_constraint_relaxations"] = json!([{
            "constraint_class":"soft",
            "constraint_id":"optional-comfort-preference",
            "prior_limit":45.0,
            "new_limit":60.0,
            "unit":"min",
            "authority_receipt_ref":"SYNTHETIC-AUTHORITY-RECEIPT",
            "authority_receipt_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "new_operating_envelope_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }]);
        value["policies"].as_array_mut().unwrap().push(policy);
        let report = audit(&value);
        assert_eq!(report["disposition"], "SYNTHETIC_ONLY");
        assert_eq!(report["policy_count"], 4);
        assert_eq!(report["policies"].as_array().unwrap().iter()
            .find(|policy| policy["policy_id"] == "authorized-soft-relaxation-arm").unwrap()["operating_envelope_sha256"],
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    }

    #[test]
    fn authorized_soft_relaxation_cannot_modify_a_service_floor() {
        let mut value = fixture();
        let mut policy = value["policies"][2].clone();
        policy["policy_id"] = json!("authorized-soft-relaxation-arm");
        policy["role"] = json!("authorized_soft_relaxation");
        policy["decision_output_sha256"] = json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        policy["operating_envelope_sha256"] = json!("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        policy["soft_constraint_relaxations"] = json!([{
            "constraint_class":"soft",
            "constraint_id":"freshwater_reserve_days_at_arrival",
            "prior_limit":3.0,
            "new_limit":2.0,
            "unit":"day",
            "authority_receipt_ref":"SYNTHETIC-AUTHORITY-RECEIPT",
            "authority_receipt_sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "new_operating_envelope_sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }]);
        value["policies"].as_array_mut().unwrap().push(policy);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }

    #[test]
    fn weather_blind_arm_must_not_consume_forecast() {
        let mut value = fixture();
        value["policies"][0]["forecast_use"] = json!(true);
        assert_eq!(audit(&value)["disposition"], "INVALID");
    }
}
