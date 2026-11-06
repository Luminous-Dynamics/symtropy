use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-space-expansion-trial-synthetic-v0-1";
const REQUIRED_REGIMES: [&str; 4] = ["helpful", "neutral", "harmful", "misleading_prior"];
const REQUIRED_CLASSES: [&str; 4] = [
    "fixed_space",
    "random_expansion",
    "rule_based_expansion",
    "llm_expansion",
];
const CLAIM_CEILING: &str = "Synthetic campaign accounting and hidden-oracle scoring only; no materials, property, feasibility model, or industrial claim is validated.";

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn fail(report: &mut Value, level: &str, code: &str, detail: &str) {
    let current = report.get("status").and_then(Value::as_str).unwrap_or("PASS");
    let next = if current == "FAIL" || level == "FAIL" {
        "FAIL"
    } else if current == "UNRESOLVED" || level == "UNRESOLVED" {
        "UNRESOLVED"
    } else {
        "PASS"
    };
    report["status"] = json!(next);
    if let Some(findings) = report.get_mut("findings").and_then(Value::as_array_mut) {
        findings.push(json!({"code": code, "detail": detail}));
    }
}

fn only_keys(value: &Value, allowed: &[&str], path: &str, report: &mut Value) {
    let Some(map) = value.as_object() else {
        fail(report, "FAIL", "OBJECT_REQUIRED", &format!("{path} must be a JSON object"));
        return;
    };
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            fail(report, "FAIL", "UNKNOWN_PROPERTY", &format!("{path}.{key} is not permitted"));
        }
    }
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn is_sha256(value: Option<&str>) -> bool {
    value.is_some_and(|s| {
        s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

fn number(value: &Value, field: &str, report: &mut Value, positive: bool) -> Option<f64> {
    match value.get(field).and_then(Value::as_f64) {
        Some(n) if n.is_finite() && (if positive { n > 0.0 } else { n >= 0.0 }) => Some(n),
        _ => {
            let requirement = if positive { "finite and greater than zero" } else { "finite and non-negative" };
            fail(report, "FAIL", "NUMBER_INVALID", &format!("{field} must be {requirement}"));
            None
        }
    }
}

fn required_string(value: &Value, field: &str, path: &str, report: &mut Value) -> Option<String> {
    match value.get(field).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(s.to_owned()),
        None => {
            fail(report, "FAIL", "STRING_REQUIRED", &format!("{path}.{field} must be a non-empty string"));
            None
        }
    }
}

#[derive(Clone, Debug)]
struct Outcome {
    feasible: bool,
    utility: f64,
}

#[derive(Clone, Debug)]
struct Tariff {
    cost: f64,
    seconds: f64,
}

fn tariff_for(event: &Value, report: &mut Value) -> Option<String> {
    match event.get("event_type").and_then(Value::as_str) {
        Some("evaluate_candidate") => Some("evaluate_candidate".to_owned()),
        Some("expand_space") => match event.get("expansion_method").and_then(Value::as_str) {
            Some("random") => Some("expand_random".to_owned()),
            Some("rule_based") => Some("expand_rule_based".to_owned()),
            Some("llm") => Some("expand_llm".to_owned()),
            Some(other) => {
                fail(report, "FAIL", "EXPANSION_METHOD_INVALID", &format!("unsupported expansion method {other}"));
                None
            }
            None => {
                fail(report, "FAIL", "EXPANSION_METHOD_REQUIRED", "expand_space requires expansion_method");
                None
            }
        },
        Some(other) => {
            fail(report, "FAIL", "EVENT_TYPE_INVALID", &format!("unsupported event_type {other}"));
            None
        }
        None => {
            fail(report, "FAIL", "EVENT_TYPE_REQUIRED", "event_type is required");
            None
        }
    }
}

fn expected_method(policy_class: &str) -> Option<&'static str> {
    match policy_class {
        "random_expansion" => Some("random"),
        "rule_based_expansion" => Some("rule_based"),
        "llm_expansion" => Some("llm"),
        _ => None,
    }
}

fn audit(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let input_sha256 = sha256(public_raw);
    let oracle_sha256 = sha256(oracle_raw);
    let mut report = json!({
        "status": "PASS",
        "audit_kind": "synthetic_adaptive_search_space_campaign",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": input_sha256,
        "oracle_sha256": oracle_sha256,
        "environment_count": 0,
        "campaigns_checked": 0,
        "action_cost_catalog_checked": 0,
        "budget_cost_units": null,
        "budget_duration_seconds": null,
        "campaign_results": [],
        "environment_comparisons": [],
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });

    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut report, "FAIL", "PUBLIC_JSON_INVALID", &e.to_string());
            return report;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut report, "FAIL", "ORACLE_JSON_INVALID", &e.to_string());
            return report;
        }
    };

    only_keys(
        &public,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "budgets", "initial_candidate_family_ids", "campaigns"],
        "public",
        &mut report,
    );
    only_keys(
        &oracle,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "action_cost_catalog", "environments"],
        "oracle",
        &mut report,
    );
    if public.get("schema_version").and_then(Value::as_u64) != Some(1)
        || oracle.get("schema_version").and_then(Value::as_u64) != Some(1)
    {
        fail(&mut report, "FAIL", "SCHEMA_VERSION_INVALID", "public input and sealed oracle must use schema_version 1");
    }
    if public.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
    {
        fail(&mut report, "FAIL", "BENCHMARK_ID_MISMATCH", "public input and sealed oracle must identify the registered benchmark");
    }
    if public.get("evidence_class").and_then(Value::as_str) != Some("synthetic")
        || oracle.get("evidence_class").and_then(Value::as_str) != Some("synthetic_evaluator_only")
    {
        fail(&mut report, "FAIL", "EVIDENCE_CLASS_MISMATCH", "public input must be synthetic and oracle must be evaluator-only synthetic data");
    }

    let Some(budgets) = public.get("budgets") else {
        fail(&mut report, "FAIL", "BUDGETS_REQUIRED", "public.budgets is required");
        return report;
    };
    only_keys(budgets, &["max_cost_units", "max_duration_seconds"], "public.budgets", &mut report);
    let max_cost = number(budgets, "max_cost_units", &mut report, true);
    let max_seconds = number(budgets, "max_duration_seconds", &mut report, true);
    report["budget_cost_units"] = max_cost.map_or(Value::Null, |n| json!(n));
    report["budget_duration_seconds"] = max_seconds.map_or(Value::Null, |n| json!(n));

    let Some(initial_ids) = public.get("initial_candidate_family_ids").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "INITIAL_CANDIDATES_REQUIRED", "initial_candidate_family_ids must be a non-empty array");
        return report;
    };
    if initial_ids.is_empty() {
        fail(&mut report, "FAIL", "INITIAL_CANDIDATES_EMPTY", "the initial fixed space cannot be empty");
    }
    let mut initial = HashSet::new();
    for value in initial_ids {
        let Some(id) = value.as_str() else {
            fail(&mut report, "FAIL", "INITIAL_CANDIDATE_ID_INVALID", "initial candidate family IDs must be strings");
            continue;
        };
        if !valid_slug(id) {
            fail(&mut report, "FAIL", "INITIAL_CANDIDATE_ID_INVALID", &format!("invalid initial candidate family ID {id}"));
        }
        if !initial.insert(id.to_owned()) {
            fail(&mut report, "FAIL", "INITIAL_CANDIDATE_DUPLICATE", &format!("initial candidate family ID {id} is duplicated"));
        }
    }

    let Some(tariff_items) = oracle.get("action_cost_catalog").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "ACTION_COST_CATALOG_REQUIRED", "oracle.action_cost_catalog must be an array");
        return report;
    };
    let mut tariffs: HashMap<String, Tariff> = HashMap::new();
    for (i, item) in tariff_items.iter().enumerate() {
        only_keys(item, &["action_type", "cost_units", "duration_seconds"], &format!("oracle.action_cost_catalog[{i}]"), &mut report);
        let action_type = required_string(item, "action_type", &format!("oracle.action_cost_catalog[{i}]"), &mut report);
        let cost = number(item, "cost_units", &mut report, true);
        let seconds = number(item, "duration_seconds", &mut report, false);
        if let (Some(action_type), Some(cost), Some(seconds)) = (action_type, cost, seconds) {
            if !["evaluate_candidate", "expand_random", "expand_rule_based", "expand_llm"].contains(&action_type.as_str()) {
                fail(&mut report, "FAIL", "ACTION_TARIFF_UNKNOWN", &format!("unregistered action tariff {action_type}"));
            }
            if tariffs.insert(action_type.clone(), Tariff { cost, seconds }).is_some() {
                fail(&mut report, "FAIL", "ACTION_TARIFF_DUPLICATE", &format!("action tariff {action_type} is declared more than once"));
            }
        }
    }
    for required in ["evaluate_candidate", "expand_random", "expand_rule_based", "expand_llm"] {
        if !tariffs.contains_key(required) {
            fail(&mut report, "FAIL", "ACTION_TARIFF_MISSING", &format!("oracle is missing required tariff {required}"));
        }
    }
    report["action_cost_catalog_checked"] = json!(tariffs.len());

    let Some(env_items) = oracle.get("environments").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "ENVIRONMENTS_REQUIRED", "oracle.environments must be an array");
        return report;
    };
    let mut envs: BTreeMap<String, (String, HashMap<String, Outcome>)> = BTreeMap::new();
    let mut seen_regimes = BTreeSet::new();
    for (i, item) in env_items.iter().enumerate() {
        only_keys(item, &["environment_id", "regime", "outcomes"], &format!("oracle.environments[{i}]"), &mut report);
        let id = required_string(item, "environment_id", &format!("oracle.environments[{i}]"), &mut report);
        let regime = required_string(item, "regime", &format!("oracle.environments[{i}]"), &mut report);
        if let Some(regime) = regime.as_ref() {
            if !REQUIRED_REGIMES.contains(&regime.as_str()) {
                fail(&mut report, "FAIL", "REGIME_INVALID", &format!("unregistered environment regime {regime}"));
            }
            if !seen_regimes.insert(regime.clone()) {
                fail(&mut report, "FAIL", "REGIME_DUPLICATE", &format!("environment regime {regime} must occur exactly once"));
            }
        }
        let Some(outcome_items) = item.get("outcomes").and_then(Value::as_array) else {
            fail(&mut report, "FAIL", "OUTCOMES_REQUIRED", "each environment must provide an outcomes array");
            continue;
        };
        let mut outcomes = HashMap::new();
        for (j, row) in outcome_items.iter().enumerate() {
            only_keys(row, &["candidate_family_id", "feasible", "utility"], &format!("oracle.environments[{i}].outcomes[{j}]"), &mut report);
            let id = required_string(row, "candidate_family_id", &format!("oracle.environments[{i}].outcomes[{j}]"), &mut report);
            let feasible = row.get("feasible").and_then(Value::as_bool);
            let utility = row.get("utility").and_then(Value::as_f64);
            if feasible.is_none() || utility.is_none_or(|u| !u.is_finite()) {
                fail(&mut report, "FAIL", "OUTCOME_INVALID", "each outcome requires a boolean feasibility value and a finite utility");
                continue;
            }
            if let Some(id) = id {
                if outcomes.insert(id.clone(), Outcome { feasible: feasible.unwrap(), utility: utility.unwrap() }).is_some() {
                    fail(&mut report, "FAIL", "OUTCOME_DUPLICATE", &format!("candidate {id} has duplicate outcomes in one environment"));
                }
            }
        }
        if let (Some(id), Some(regime)) = (id, regime) {
            if !valid_slug(&id) {
                fail(&mut report, "FAIL", "ENVIRONMENT_ID_INVALID", &format!("invalid environment ID {id}"));
            }
            if envs.insert(id.clone(), (regime, outcomes)).is_some() {
                fail(&mut report, "FAIL", "ENVIRONMENT_DUPLICATE", &format!("environment ID {id} is duplicated"));
            }
        }
    }
    for required in REQUIRED_REGIMES {
        if !seen_regimes.contains(required) {
            fail(&mut report, "FAIL", "REGIME_MISSING", &format!("sealed oracle must include the {required} regime"));
        }
    }
    report["environment_count"] = json!(envs.len());

    let Some(campaign_items) = public.get("campaigns").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "CAMPAIGNS_REQUIRED", "public.campaigns must be an array");
        return report;
    };
    if campaign_items.is_empty() {
        fail(&mut report, "FAIL", "CAMPAIGNS_EMPTY", "at least one campaign is required");
        return report;
    }

    let mut campaign_ids = HashSet::new();
    let mut campaigns_by_env: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut results = Vec::new();

    for (campaign_index, campaign) in campaign_items.iter().enumerate() {
        only_keys(campaign, &["campaign_id", "environment_id", "policy_id", "policy_class", "events"], &format!("public.campaigns[{campaign_index}]"), &mut report);
        let campaign_id = required_string(campaign, "campaign_id", "campaign", &mut report).unwrap_or_else(|| format!("invalid-{campaign_index}"));
        let environment_id = required_string(campaign, "environment_id", &format!("campaign {campaign_id}"), &mut report);
        let policy_id = required_string(campaign, "policy_id", &format!("campaign {campaign_id}"), &mut report);
        let policy_class = required_string(campaign, "policy_class", &format!("campaign {campaign_id}"), &mut report).unwrap_or_default();
        if !campaign_ids.insert(campaign_id.clone()) {
            fail(&mut report, "FAIL", "CAMPAIGN_ID_DUPLICATE", &format!("campaign ID {campaign_id} is duplicated"));
        }
        if !REQUIRED_CLASSES.contains(&policy_class.as_str()) {
            fail(&mut report, "FAIL", "POLICY_CLASS_INVALID", &format!("campaign {campaign_id} has unregistered policy class {policy_class}"));
        }
        let Some(environment_id) = environment_id else { continue };
        let Some((regime, outcomes)) = envs.get(&environment_id) else {
            fail(&mut report, "UNRESOLVED", "ENVIRONMENT_OUTCOME_MISSING", &format!("campaign {campaign_id} references unknown sealed environment {environment_id}"));
            continue;
        };
        campaigns_by_env.entry(environment_id.clone()).or_default().insert(policy_class.clone());
        let Some(events) = campaign.get("events").and_then(Value::as_array) else {
            fail(&mut report, "FAIL", "EVENTS_REQUIRED", &format!("campaign {campaign_id} events must be an array"));
            continue;
        };
        if events.is_empty() {
            fail(&mut report, "FAIL", "EVENTS_EMPTY", &format!("campaign {campaign_id} has no events"));
            continue;
        }

        let mut visible = initial.clone();
        let mut evaluated = HashSet::new();
        let mut event_ids = HashSet::new();
        let mut cost_used = 0.0_f64;
        let mut seconds_used = 0.0_f64;
        let mut expansion_count = 0_usize;
        let mut evaluation_count = 0_usize;
        let mut feasible_hits = 0_usize;
        let mut false_feasible_predictions = 0_usize;
        let mut true_feasible_predictions = 0_usize;
        let mut best_utility: Option<f64> = None;
        let mut first_feasible_cost: Option<f64> = None;
        let mut trace_event_ids = Vec::new();

        for (event_index, event) in events.iter().enumerate() {
            let event_type = event.get("event_type").and_then(Value::as_str).unwrap_or("");
            let allowed: &[&str] = if event_type == "expand_space" {
                &["event_seq", "event_id", "event_type", "candidate_family_id", "expansion_method", "proposal_sha256"]
            } else if event_type == "evaluate_candidate" {
                &["event_seq", "event_id", "event_type", "candidate_family_id", "claimed_feasible"]
            } else {
                &["event_seq", "event_id", "event_type", "candidate_family_id"]
            };
            only_keys(event, allowed, &format!("campaign {campaign_id}.events[{event_index}]"), &mut report);
            if event.get("event_seq").and_then(Value::as_u64) != Some((event_index + 1) as u64) {
                fail(&mut report, "FAIL", "EVENT_SEQUENCE_INVALID", &format!("campaign {campaign_id} event sequence must be consecutive from 1"));
            }
            let event_id = required_string(event, "event_id", &format!("campaign {campaign_id} event"), &mut report).unwrap_or_else(|| format!("invalid-event-{event_index}"));
            if !event_ids.insert(event_id.clone()) {
                fail(&mut report, "FAIL", "EVENT_ID_DUPLICATE", &format!("campaign {campaign_id} event ID {event_id} is duplicated"));
            }
            trace_event_ids.push(event_id.clone());

            let family_id = required_string(event, "candidate_family_id", &format!("campaign {campaign_id} event {event_id}"), &mut report).unwrap_or_default();
            let Some(tariff_id) = tariff_for(event, &mut report) else { continue };
            let Some(tariff) = tariffs.get(&tariff_id) else {
                fail(&mut report, "FAIL", "ACTION_TARIFF_MISSING", &format!("missing evaluator-owned tariff {tariff_id}"));
                continue;
            };

            // Charge before interpreting a measurement result. Policy-supplied costs are not accepted.
            cost_used += tariff.cost;
            seconds_used += tariff.seconds;
            if let Some(cap) = max_cost {
                if cost_used > cap + 1e-9 {
                    fail(&mut report, "FAIL", "COST_BUDGET_OVERRUN", &format!("campaign {campaign_id} charged cost {cost_used} above cap {cap}"));
                }
            }
            if let Some(cap) = max_seconds {
                if seconds_used > cap + 1e-9 {
                    fail(&mut report, "FAIL", "TIME_BUDGET_OVERRUN", &format!("campaign {campaign_id} charged duration {seconds_used} above cap {cap}"));
                }
            }

            if event_type == "expand_space" {
                expansion_count += 1;
                let method = event.get("expansion_method").and_then(Value::as_str).unwrap_or("");
                let expected = expected_method(&policy_class);
                if expected != Some(method) {
                    fail(&mut report, "FAIL", "POLICY_EXPANSION_METHOD_MISMATCH", &format!("campaign {campaign_id} policy class {policy_class} cannot use expansion method {method}"));
                }
                if visible.contains(&family_id) {
                    fail(&mut report, "FAIL", "FAMILY_ALREADY_VISIBLE", &format!("campaign {campaign_id} tries to re-add visible candidate family {family_id}"));
                }
                if !is_sha256(event.get("proposal_sha256").and_then(Value::as_str)) {
                    fail(&mut report, "FAIL", "PROPOSAL_DIGEST_INVALID", &format!("campaign {campaign_id} expansion requires a 64-character proposal SHA-256"));
                }
                visible.insert(family_id);
            } else if event_type == "evaluate_candidate" {
                evaluation_count += 1;
                if !visible.contains(&family_id) {
                    fail(&mut report, "FAIL", "CANDIDATE_NOT_VISIBLE", &format!("campaign {campaign_id} evaluates {family_id} before it is in the visible search space"));
                }
                if !evaluated.insert(family_id.clone()) {
                    fail(&mut report, "FAIL", "CANDIDATE_EVALUATED_TWICE", &format!("campaign {campaign_id} evaluates candidate {family_id} more than once"));
                }
                let Some(claimed_feasible) = event.get("claimed_feasible").and_then(Value::as_bool) else {
                    fail(&mut report, "FAIL", "FEASIBILITY_PREDICTION_REQUIRED", &format!("campaign {campaign_id} evaluation must record the pre-observation claimed_feasible prediction"));
                    continue;
                };
                let Some(outcome) = outcomes.get(&family_id) else {
                    fail(&mut report, "UNRESOLVED", "SEALED_OUTCOME_MISSING", &format!("no sealed outcome for environment {environment_id}, candidate {family_id}; cannot count it as a negative or a success"));
                    continue;
                };
                if claimed_feasible && outcome.feasible {
                    true_feasible_predictions += 1;
                } else if claimed_feasible && !outcome.feasible {
                    false_feasible_predictions += 1;
                }
                if outcome.feasible {
                    feasible_hits += 1;
                    best_utility = Some(best_utility.map_or(outcome.utility, |old| old.max(outcome.utility)));
                    if first_feasible_cost.is_none() {
                        first_feasible_cost = Some(cost_used);
                    }
                }
            }
        }

        let actual_expansion_count = expansion_count;
        match expected_method(&policy_class) {
            None if actual_expansion_count != 0 => fail(&mut report, "FAIL", "FIXED_SPACE_EXPANDED", &format!("fixed-space campaign {campaign_id} must not expand the domain")),
            Some(_) if actual_expansion_count != 1 => fail(&mut report, "FAIL", "EXPANSION_COUNT_INVALID", &format!("expansion policy campaign {campaign_id} must contain exactly one registered expansion event in v0.1")),
            _ => {}
        }

        results.push(json!({
            "campaign_id": campaign_id,
            "environment_id": environment_id,
            "environment_regime": regime,
            "policy_id": policy_id,
            "policy_class": policy_class,
            "trace_event_ids": trace_event_ids,
            "events_checked": events.len(),
            "expansion_actions": expansion_count,
            "candidate_evaluations": evaluation_count,
            "verified_feasible_discoveries": feasible_hits,
            "true_feasible_predictions": true_feasible_predictions,
            "false_feasible_predictions": false_feasible_predictions,
            "best_verified_feasible_utility": best_utility,
            "first_feasible_cost_units": first_feasible_cost,
            "cost_units_charged": cost_used,
            "duration_seconds_charged": seconds_used,
            "claim_ceiling": "Synthetic evaluator score only."
        }));
    }

    for (environment_id, (regime, _)) in &envs {
        let classes = campaigns_by_env.get(environment_id).cloned().unwrap_or_default();
        for required in REQUIRED_CLASSES {
            if !classes.contains(required) {
                fail(&mut report, "FAIL", "POLICY_ARM_MISSING", &format!("environment {environment_id} ({regime}) is missing policy class {required}"));
            }
        }
        for class in &classes {
            let count = campaign_items.iter().filter(|campaign| {
                campaign.get("environment_id").and_then(Value::as_str) == Some(environment_id)
                    && campaign.get("policy_class").and_then(Value::as_str) == Some(class.as_str())
            }).count();
            if count != 1 {
                fail(&mut report, "FAIL", "POLICY_ARM_COUNT_INVALID", &format!("environment {environment_id} has {count} campaigns for policy class {class}; exactly one is required in this fixture protocol"));
            }
        }
    }

    // Summaries keep environments separate; do not average away the harmful or misleading-prior regimes.
    let mut comparisons = Vec::new();
    for regime in REQUIRED_REGIMES {
        let env_ids: Vec<_> = envs.iter().filter(|(_, (r, _))| r == regime).map(|(id, _)| id).collect();
        for env_id in env_ids {
            let mut rows: Vec<Value> = results.iter()
                .filter(|row| row.get("environment_id").and_then(Value::as_str) == Some(env_id.as_str()))
                .cloned()
                .collect();
            rows.sort_by(|a, b| {
                a.get("policy_class").and_then(Value::as_str).cmp(&b.get("policy_class").and_then(Value::as_str))
            });
            comparisons.push(json!({"environment_id": env_id, "regime": regime, "policy_results": rows}));
        }
    }
    report["campaigns_checked"] = json!(results.len());
    report["campaign_results"] = json!(results);
    report["environment_comparisons"] = json!(comparisons);
    report
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: audit-search-space-trial <public-campaign-input.json> <sealed-oracle.json>");
        return ExitCode::from(64);
    }
    let public = match read_bytes(&args[1]) {
        Ok(v) => v,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(2); }
    };
    let oracle = match read_bytes(&args[2]) {
        Ok(v) => v,
        Err(e) => { eprintln!("{e}"); return ExitCode::from(2); }
    };
    let report = audit(&public, &oracle);
    let status = report.get("status").and_then(Value::as_str).unwrap_or("FAIL");
    let code = match status {
        "PASS" => 0,
        "UNRESOLVED" => 3,
        _ => 2,
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/search-space-campaign-input.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/search-space-outcome-oracle.synthetic.json");

    fn run(public: &[u8], oracle: &[u8]) -> Value {
        audit(public, oracle)
    }

    fn find<'a>(report: &'a Value, env: &str, class: &str) -> &'a Value {
        report["campaign_results"].as_array().unwrap().iter().find(|r| {
            r["environment_id"] == env && r["policy_class"] == class
        }).expect("campaign result exists")
    }

    #[test]
    fn all_arms_fit_the_same_evaluator_owned_cost_budget() {
        let report = run(PUBLIC, ORACLE);
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["campaigns_checked"], 16);
        for campaign in report["campaign_results"].as_array().unwrap() {
            assert_eq!(campaign["cost_units_charged"], 5.0);
            assert!(campaign["duration_seconds_charged"].as_f64().unwrap() <= 25.0);
        }
    }

    #[test]
    fn helpful_expansion_can_win_without_being_assumed_to_win_everywhere() {
        let report = run(PUBLIC, ORACLE);
        assert_eq!(find(&report, "helpful", "fixed_space")["best_verified_feasible_utility"], 4.0);
        assert_eq!(find(&report, "helpful", "random_expansion")["best_verified_feasible_utility"], 6.0);
        assert_eq!(find(&report, "helpful", "rule_based_expansion")["best_verified_feasible_utility"], 8.0);
        assert_eq!(find(&report, "helpful", "llm_expansion")["best_verified_feasible_utility"], 10.0);
    }

    #[test]
    fn neutral_expansion_must_not_be_reported_as_a_gain() {
        let report = run(PUBLIC, ORACLE);
        for class in REQUIRED_CLASSES {
            assert_eq!(find(&report, "neutral", class)["best_verified_feasible_utility"], 6.0);
        }
    }

    #[test]
    fn harmful_expansion_can_lose_to_fixed_space() {
        let report = run(PUBLIC, ORACLE);
        assert_eq!(find(&report, "harmful", "fixed_space")["best_verified_feasible_utility"], 8.0);
        assert_eq!(find(&report, "harmful", "random_expansion")["best_verified_feasible_utility"], 7.0);
        assert_eq!(find(&report, "harmful", "rule_based_expansion")["best_verified_feasible_utility"], 7.0);
        assert_eq!(find(&report, "harmful", "llm_expansion")["best_verified_feasible_utility"], 6.0);
    }

    #[test]
    fn misleading_prior_produces_a_false_feasible_llm_prediction() {
        let report = run(PUBLIC, ORACLE);
        assert_eq!(find(&report, "misleading_prior", "llm_expansion")["best_verified_feasible_utility"], 6.0);
        assert_eq!(find(&report, "misleading_prior", "llm_expansion")["false_feasible_predictions"], 1);
    }

    #[test]
    fn policy_cost_understatement_is_impossible_because_costs_are_not_in_public_events() {
        let public: Value = serde_json::from_slice(PUBLIC).unwrap();
        let encoded = serde_json::to_vec(&public).unwrap();
        let report = run(&encoded, ORACLE);
        assert_eq!(report["status"], "PASS");
        assert!(report["campaign_results"][0].get("cost_units").is_none());
        assert_eq!(report["campaign_results"][0]["cost_units_charged"], 5.0);
    }

    #[test]
    fn hidden_outcomes_are_not_allowed_in_policy_visible_input() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        public["campaigns"][0]["events"][0]["outcome"] = json!({"feasible":true,"utility":1000000.0});
        let encoded = serde_json::to_vec(&public).unwrap();
        let report = run(&encoded, ORACLE);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "UNKNOWN_PROPERTY"));
    }

    #[test]
    fn expansion_must_precede_evaluation_of_the_new_family() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        let campaign = public["campaigns"].as_array_mut().unwrap().iter_mut().find(|c| c["campaign_id"] == "helpful-random-expansion").unwrap();
        campaign["events"].as_array_mut().unwrap().swap(0, 3);
        for (i, event) in campaign["events"].as_array_mut().unwrap().iter_mut().enumerate() {
            event["event_seq"] = json!(i + 1);
        }
        let encoded = serde_json::to_vec(&public).unwrap();
        let report = run(&encoded, ORACLE);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "CANDIDATE_NOT_VISIBLE"));
    }

    #[test]
    fn hidden_outcome_mutation_changes_score_but_not_public_trace() {
        let original = run(PUBLIC, ORACLE);
        let mut oracle: Value = serde_json::from_slice(ORACLE).unwrap();
        let helpful = oracle["environments"].as_array_mut().unwrap().iter_mut().find(|e| e["environment_id"] == "helpful").unwrap();
        let llm_outcome = helpful["outcomes"].as_array_mut().unwrap().iter_mut().find(|o| o["candidate_family_id"] == "llm-family").unwrap();
        llm_outcome["utility"] = json!(0.0);
        let encoded = serde_json::to_vec(&oracle).unwrap();
        let mutated = run(PUBLIC, &encoded);
        assert_eq!(original["input_sha256"], mutated["input_sha256"]);
        assert_eq!(
            find(&original, "helpful", "llm_expansion")["trace_event_ids"],
            find(&mutated, "helpful", "llm_expansion")["trace_event_ids"]
        );
        assert_ne!(
            find(&original, "helpful", "llm_expansion")["best_verified_feasible_utility"],
            find(&mutated, "helpful", "llm_expansion")["best_verified_feasible_utility"]
        );
    }

    #[test]
    fn budget_overrun_is_a_hard_failure() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        public["budgets"]["max_cost_units"] = json!(4.0);
        let encoded = serde_json::to_vec(&public).unwrap();
        let report = run(&encoded, ORACLE);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "COST_BUDGET_OVERRUN"));
    }

    #[test]
    fn missing_outcomes_are_unresolved_not_zero() {
        let mut oracle: Value = serde_json::from_slice(ORACLE).unwrap();
        let helpful = oracle["environments"].as_array_mut().unwrap().iter_mut().find(|e| e["environment_id"] == "helpful").unwrap();
        helpful["outcomes"].as_array_mut().unwrap().retain(|o| o["candidate_family_id"] != "llm-family");
        let encoded = serde_json::to_vec(&oracle).unwrap();
        let report = run(PUBLIC, &encoded);
        assert_eq!(report["status"], "UNRESOLVED");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "SEALED_OUTCOME_MISSING"));
    }

    #[test]
    fn missing_policy_arm_fails_instead_of_improving_the_scorecard_by_omission() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        public["campaigns"].as_array_mut().unwrap().retain(|c| c["campaign_id"] != "harmful-fixed-space");
        let encoded = serde_json::to_vec(&public).unwrap();
        let report = run(&encoded, ORACLE);
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "POLICY_ARM_MISSING"));
    }
}
