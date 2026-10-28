use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-space-expansion-live-loop-synthetic-v0-1";
const POLICY_CLASSES: [&str; 5] = [
    "fixed_space",
    "random_expansion",
    "rule_based_expansion",
    "prior_guided_expansion",
    "static_prior_expansion",
];
const CLAIM_CEILING: &str = "Synthetic live event-loop mechanics only; no material model, physical feasibility, state-of-the-art result, or industrial benefit is validated.";

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Evaluate(String),
    Expand(String),
    Stop(String),
}

#[derive(Clone, Debug)]
struct Tariff {
    action_type: String,
    cost: f64,
    seconds: f64,
}

#[derive(Clone, Debug)]
struct Observation {
    candidate_id: String,
    feasible: bool,
    utility: f64,
}

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

fn valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.as_bytes()[0].is_ascii_lowercase()
        && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

fn req_string<'a>(v: &'a Value, field: &str, report: &mut Value, path: &str) -> Option<&'a str> {
    match v.get(field).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(s),
        None => {
            fail(report, "FAIL", "STRING_REQUIRED", &format!("{path}.{field} must be a non-empty string"));
            None
        }
    }
}

fn finite_number(v: &Value, field: &str, report: &mut Value, path: &str, positive: bool) -> Option<f64> {
    match v.get(field).and_then(Value::as_f64) {
        Some(n) if n.is_finite() && (if positive { n > 0.0 } else { n >= 0.0 }) => Some(n),
        _ => {
            fail(report, "FAIL", "NUMBER_INVALID", &format!("{path}.{field} must be finite and {}", if positive { "greater than zero" } else { "non-negative" }));
            None
        }
    }
}

fn allowed_policy(policy: &str) -> bool {
    POLICY_CLASSES.contains(&policy)
}

fn tariff_key_for_method(method: &str) -> Option<&'static str> {
    match method {
        "random" => Some("expand_random"),
        "rule_based" => Some("expand_rule_based"),
        "prior_guided" => Some("expand_prior_guided"),
        _ => None,
    }
}

fn method_for_policy(policy: &str) -> Option<&'static str> {
    match policy {
        "random_expansion" => Some("random"),
        "rule_based_expansion" => Some("rule_based"),
        "prior_guided_expansion" | "static_prior_expansion" => Some("prior_guided"),
        _ => None,
    }
}

fn expansion_threshold(catalog: &Value, policy: &str) -> Option<f64> {
    catalog.get("policy_contract")?
        .get("thresholds")?
        .get(policy)?
        .as_f64()
}

fn choose_action(policy: &str, view: &Value, expansion_decision_made: bool) -> Action {
    let expanded = view.get("expansion_used").and_then(Value::as_bool).unwrap_or(false);
    let observations = view.get("observations").and_then(Value::as_array).cloned().unwrap_or_default();

    // Static expansion is a deliberately non-adaptive control. It expands before observing any result.
    if policy == "static_prior_expansion" && !expanded {
        return Action::Expand("prior_guided".to_owned());
    }

    // Every arm begins with the same public screening candidate. The outcome is not known yet.
    if observations.is_empty() {
        let candidates = view.get("visible_candidates").and_then(Value::as_array).cloned().unwrap_or_default();
        let best = candidates.into_iter().filter(|c| c.get("evaluated").and_then(Value::as_bool) == Some(false))
            .max_by(|a, b| {
                a.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&b.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
            });
        return best.and_then(|c| c.get("id").and_then(Value::as_str).map(|s| Action::Evaluate(s.to_owned())))
            .unwrap_or_else(|| Action::Stop("NO_UNMEASURED_VISIBLE_CANDIDATES".to_owned()));
    }

    // Adaptive outer-loop policies decide whether to expand only after the first screening result.
    if !expansion_decision_made {
        if let (Some(method), Some(threshold)) = (method_for_policy(policy), expansion_threshold_from_view(view, policy)) {
            let first = &observations[0];
            let value = first.get("utility").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY);
            let feasible = first.get("feasible").and_then(Value::as_bool).unwrap_or(false);
            if !feasible || value < threshold {
                return Action::Expand(method.to_owned());
            }
        }
    }

    let last = observations.last().cloned().unwrap_or(Value::Null);
    let last_id = last.get("candidate_family_id").and_then(Value::as_str).unwrap_or("");
    let last_feasible = last.get("feasible").and_then(Value::as_bool).unwrap_or(false);
    let expanded_ids: HashSet<String> = view.get("expanded_candidate_ids").and_then(Value::as_array)
        .into_iter().flatten().filter_map(Value::as_str).map(str::to_owned).collect();
    let recovery = expanded_ids.contains(last_id) && !last_feasible;

    let candidates = view.get("visible_candidates").and_then(Value::as_array).cloned().unwrap_or_default();
    let best = candidates.into_iter().filter(|c| c.get("evaluated").and_then(Value::as_bool) == Some(false))
        .max_by(|a, b| {
            let field = if recovery { "failure_fallback_score" } else { "selector_score" };
            a.get(field).and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&b.get(field).and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
        });
    best.and_then(|c| c.get("id").and_then(Value::as_str).map(|s| Action::Evaluate(s.to_owned())))
        .unwrap_or_else(|| Action::Stop("NO_UNMEASURED_VISIBLE_CANDIDATES".to_owned()))
}

// Thresholds are copied into the policy's public view; no evaluator regime or outcome map is passed.
fn expansion_threshold_from_view(view: &Value, policy: &str) -> Option<f64> {
    view.get("public_thresholds")?.get(policy)?.as_f64()
}

fn build_public_view(
    catalog: &Value,
    policy: &str,
    visible: &BTreeSet<String>,
    evaluated: &HashSet<String>,
    observations: &[Observation],
    expanded_candidate_ids: &[String],
    expansion_used: bool,
    remaining_cost: f64,
    remaining_seconds: f64,
) -> Value {
    let visible_candidates = catalog.get("candidate_families").and_then(Value::as_array).into_iter().flatten()
        .filter(|c| c.get("id").and_then(Value::as_str).is_some_and(|id| visible.contains(id)))
        .map(|c| {
            let id = c.get("id").and_then(Value::as_str).unwrap_or("");
            json!({
                "id": id,
                "domain": c.get("domain").cloned().unwrap_or(Value::Null),
                "screen_priority": c.get("screen_priority").cloned().unwrap_or(Value::Null),
                "predicted_feasible": c.get("predicted_feasible").cloned().unwrap_or(Value::Null),
                "selector_score": c.get("selector_scores").and_then(|v| v.get(policy)).cloned().unwrap_or(Value::Null),
                "failure_fallback_score": c.get("failure_fallback_score").cloned().unwrap_or(Value::Null),
                "is_expansion_candidate": c.get("initially_visible").and_then(Value::as_bool) == Some(false),
                "evaluated": evaluated.contains(id)
            })
        })
        .collect::<Vec<_>>();
    let obs = observations.iter().map(|o| json!({
        "candidate_family_id": o.candidate_id,
        "feasible": o.feasible,
        "utility": o.utility
    })).collect::<Vec<_>>();
    json!({
        "policy_id": policy,
        "visible_candidates": visible_candidates,
        "observations": obs,
        "expanded_candidate_ids": expanded_candidate_ids,
        "expansion_used": expansion_used,
        "available_expansion_methods": if expansion_used { Vec::<String>::new() } else { vec!["random".to_owned(), "rule_based".to_owned(), "prior_guided".to_owned()] },
        "public_thresholds": catalog.get("policy_contract").and_then(|v| v.get("thresholds")).cloned().unwrap_or_else(|| json!({})),
        "remaining_cost_units": remaining_cost,
        "remaining_duration_seconds": remaining_seconds
    })
}

fn get_tariffs(oracle: &Value, report: &mut Value) -> HashMap<String, Tariff> {
    let mut tariffs = HashMap::new();
    let Some(items) = oracle.get("action_cost_catalog").and_then(Value::as_array) else {
        fail(report, "FAIL", "TARIFF_CATALOG_REQUIRED", "oracle.action_cost_catalog must be an array");
        return tariffs;
    };
    for (i, item) in items.iter().enumerate() {
        only_keys(item, &["action_type", "cost_units", "duration_seconds"], &format!("oracle.action_cost_catalog[{i}]"), report);
        let Some(key) = req_string(item, "action_type", report, &format!("oracle.action_cost_catalog[{i}]")) else { continue };
        let Some(cost) = finite_number(item, "cost_units", report, &format!("oracle.action_cost_catalog[{i}]"), true) else { continue };
        let Some(seconds) = finite_number(item, "duration_seconds", report, &format!("oracle.action_cost_catalog[{i}]"), false) else { continue };
        if tariffs.insert(key.to_owned(), Tariff { action_type: key.to_owned(), cost, seconds }).is_some() {
            fail(report, "FAIL", "TARIFF_DUPLICATE", &format!("duplicate action tariff {key}"));
        }
    }
    for key in ["evaluate_candidate", "expand_random", "expand_rule_based", "expand_prior_guided"] {
        if !tariffs.contains_key(key) {
            fail(report, "FAIL", "TARIFF_MISSING", &format!("missing evaluator-owned tariff {key}"));
        }
    }
    tariffs
}

fn run_one(catalog: &Value, oracle: &Value, environment_id: &str, policy: &str) -> Value {
    let mut report = json!({
        "status": "PASS",
        "campaign_id": format!("{environment_id}-{policy}"),
        "environment_id": environment_id,
        "policy_class": policy,
        "policy_id": format!("synthetic-live-loop-{policy}-v1"),
        "input_sha256": null,
        "oracle_sha256": null,
        "events": [],
        "evaluations": 0,
        "expansions": 0,
        "verified_feasible_discoveries": 0,
        "false_feasible_predictions": 0,
        "best_verified_feasible_utility": null,
        "first_feasible_cost_units": null,
        "cost_units_charged": 0.0,
        "duration_seconds_charged": 0.0,
        "terminal_disposition": "NOT_STARTED",
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });
    if !allowed_policy(policy) {
        fail(&mut report, "FAIL", "POLICY_CLASS_INVALID", &format!("unregistered policy class {policy}"));
        report["terminal_disposition"] = json!("REJECTED");
        return report;
    }
    if catalog.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID) {
        fail(&mut report, "FAIL", "BENCHMARK_ID_MISMATCH", "catalog and oracle must use the live-loop benchmark ID");
        report["terminal_disposition"] = json!("REJECTED");
        return report;
    }
    let Some(envs) = oracle.get("environments").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "ENVIRONMENTS_REQUIRED", "sealed oracle has no environment array");
        report["terminal_disposition"] = json!("REJECTED");
        return report;
    };
    let Some(env) = envs.iter().find(|e| e.get("environment_id").and_then(Value::as_str) == Some(environment_id)) else {
        fail(&mut report, "UNRESOLVED", "ENVIRONMENT_MISSING", &format!("sealed oracle has no environment {environment_id}"));
        report["terminal_disposition"] = json!("UNRESOLVED");
        return report;
    };
    let Some(outcome_rows) = env.get("outcomes").and_then(Value::as_array) else {
        fail(&mut report, "UNRESOLVED", "OUTCOMES_MISSING", "sealed environment has no outcome array");
        report["terminal_disposition"] = json!("UNRESOLVED");
        return report;
    };
    let outcomes: HashMap<String, (bool, f64)> = outcome_rows.iter().filter_map(|row| {
        let id = row.get("candidate_family_id")?.as_str()?.to_owned();
        let feasible = row.get("feasible")?.as_bool()?;
        let utility = row.get("utility")?.as_f64()?;
        utility.is_finite().then_some((id, (feasible, utility)))
    }).collect();
    let tariffs = get_tariffs(oracle, &mut report);
    let max_cost = catalog.get("budgets").and_then(|v| v.get("max_cost_units")).and_then(Value::as_f64).unwrap_or(0.0);
    let max_seconds = catalog.get("budgets").and_then(|v| v.get("max_duration_seconds")).and_then(Value::as_f64).unwrap_or(0.0);
    if !max_cost.is_finite() || max_cost <= 0.0 || !max_seconds.is_finite() || max_seconds <= 0.0 {
        fail(&mut report, "FAIL", "BUDGET_INVALID", "public cost and duration caps must be finite and positive");
        report["terminal_disposition"] = json!("REJECTED");
        return report;
    }

    let family_rows = catalog.get("candidate_families").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut family_by_id: HashMap<String, Value> = HashMap::new();
    for (i, family) in family_rows.iter().enumerate() {
        only_keys(family, &["id", "domain", "initially_visible", "screen_priority", "predicted_feasible", "selector_scores", "failure_fallback_score"], &format!("catalog.candidate_families[{i}]"), &mut report);
        let Some(id) = family.get("id").and_then(Value::as_str) else {
            fail(&mut report, "FAIL", "FAMILY_ID_REQUIRED", "each public family requires a string ID");
            continue;
        };
        if !valid_slug(id) {
            fail(&mut report, "FAIL", "FAMILY_ID_INVALID", &format!("invalid candidate family ID {id}"));
        }
        if family_by_id.insert(id.to_owned(), family.clone()).is_some() {
            fail(&mut report, "FAIL", "FAMILY_ID_DUPLICATE", &format!("duplicate candidate family ID {id}"));
        }
    }
    let Some(initial_ids) = catalog.get("candidate_families").and_then(Value::as_array) else {
        fail(&mut report, "FAIL", "CANDIDATE_CATALOG_REQUIRED", "candidate_families must be an array");
        report["terminal_disposition"] = json!("REJECTED");
        return report;
    };
    let mut visible: BTreeSet<String> = initial_ids.iter().filter(|c| c.get("initially_visible").and_then(Value::as_bool) == Some(true))
        .filter_map(|c| c.get("id").and_then(Value::as_str).map(str::to_owned)).collect();
    let mut evaluated = HashSet::new();
    let mut expanded_candidate_ids = Vec::new();
    let mut observations = Vec::<Observation>::new();
    let mut expansion_used = false;
    let mut expansion_decision_made = policy == "fixed_space" || policy == "static_prior_expansion";
    let mut cost_used = 0.0_f64;
    let mut seconds_used = 0.0_f64;
    let mut feasible_hits = 0_usize;
    let mut false_feasible = 0_usize;
    let mut best_utility: Option<f64> = None;
    let mut first_feasible_cost: Option<f64> = None;
    let mut terminal = "NO_ACTION".to_owned();

    for seq in 1..=16 {
        let remaining_cost = (max_cost - cost_used).max(0.0);
        let remaining_seconds = (max_seconds - seconds_used).max(0.0);
        if remaining_cost < tariffs.get("evaluate_candidate").map(|t| t.cost).unwrap_or(f64::INFINITY)
            || remaining_seconds < tariffs.get("evaluate_candidate").map(|t| t.seconds).unwrap_or(f64::INFINITY) {
            terminal = "BUDGET_EXHAUSTED".to_owned();
            break;
        }
        let view = build_public_view(catalog, policy, &visible, &evaluated, &observations,
            &expanded_candidate_ids, expansion_used, remaining_cost, remaining_seconds);
        let consideration_due = !expansion_decision_made
            && observations.len() == 1
            && method_for_policy(policy).is_some();
        let action = choose_action(policy, &view, expansion_decision_made);
        if consideration_due {
            expansion_decision_made = true;
        }
        let view_bytes = serde_json::to_vec(&view).unwrap_or_default();
        let view_digest = sha256(&view_bytes);

        match action {
            Action::Stop(reason) => {
                terminal = reason;
                break;
            }
            Action::Evaluate(candidate_id) => {
                if !visible.contains(&candidate_id) {
                    fail(&mut report, "FAIL", "CANDIDATE_NOT_VISIBLE", &format!("policy attempted to evaluate hidden candidate {candidate_id}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                if !evaluated.insert(candidate_id.clone()) {
                    fail(&mut report, "FAIL", "CANDIDATE_DUPLICATE_EVALUATION", &format!("policy attempted to re-evaluate {candidate_id}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                let Some(family) = family_by_id.get(&candidate_id) else {
                    fail(&mut report, "FAIL", "FAMILY_DEFINITION_MISSING", &format!("public family definition missing for {candidate_id}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                let Some(tariff) = tariffs.get("evaluate_candidate") else {
                    fail(&mut report, "FAIL", "TARIFF_MISSING", "missing evaluation tariff");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                if cost_used + tariff.cost > max_cost + 1e-9 || seconds_used + tariff.seconds > max_seconds + 1e-9 {
                    fail(&mut report, "FAIL", "ACTION_WOULD_OVERRUN_BUDGET", "candidate evaluation would exceed the shared budget");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }

                // Charge first. Only after the charge does the evaluator reveal the selected outcome.
                cost_used += tariff.cost;
                seconds_used += tariff.seconds;
                let mut event = json!({
                    "event_seq": seq,
                    "action_type": "evaluate_candidate",
                    "candidate_family_id": candidate_id,
                    "decision_view_sha256": view_digest,
                    "cost_units_charged": tariff.cost,
                    "duration_seconds_charged": tariff.seconds,
                    "predicted_feasible_before_observation": family.get("predicted_feasible").cloned().unwrap_or(Value::Null),
                    "observed_outcome": null
                });
                let Some((feasible, utility)) = outcomes.get(&candidate_id).copied() else {
                    report["events"].as_array_mut().unwrap().push(event);
                    fail(&mut report, "UNRESOLVED", "SEALED_OUTCOME_MISSING", &format!("missing outcome for {environment_id}/{candidate_id}; it is not converted to zero"));
                    terminal = "UNRESOLVED_OUTCOME".to_owned();
                    break;
                };
                let prediction = family.get("predicted_feasible").and_then(Value::as_bool);
                if prediction == Some(true) && !feasible {
                    false_feasible += 1;
                }
                if feasible {
                    feasible_hits += 1;
                    best_utility = Some(best_utility.map_or(utility, |old| old.max(utility)));
                    if first_feasible_cost.is_none() {
                        first_feasible_cost = Some(cost_used);
                    }
                }
                event["observed_outcome"] = json!({"feasible": feasible, "utility": utility});
                report["events"].as_array_mut().unwrap().push(event);
                observations.push(Observation { candidate_id, feasible, utility });
                terminal = "BUDGET_OR_CANDIDATES_PENDING".to_owned();
            }
            Action::Expand(method) => {
                if expansion_used {
                    fail(&mut report, "FAIL", "REPEATED_EXPANSION", "v0.1 permits at most one expansion per episode");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                if method_for_policy(policy) != Some(method.as_str()) {
                    fail(&mut report, "FAIL", "EXPANSION_METHOD_POLICY_MISMATCH", &format!("policy {policy} may not use expansion method {method}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                let Some(tariff_key) = tariff_key_for_method(&method) else {
                    fail(&mut report, "FAIL", "EXPANSION_METHOD_INVALID", &format!("unsupported expansion method {method}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                let Some(tariff) = tariffs.get(tariff_key) else {
                    fail(&mut report, "FAIL", "TARIFF_MISSING", &format!("missing expansion tariff {tariff_key}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                if cost_used + tariff.cost > max_cost + 1e-9 || seconds_used + tariff.seconds > max_seconds + 1e-9 {
                    fail(&mut report, "FAIL", "ACTION_WOULD_OVERRUN_BUDGET", "space expansion would exceed the shared budget");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                let Some(option) = catalog.get("expansion_options").and_then(Value::as_array).into_iter().flatten()
                    .find(|o| o.get("method").and_then(Value::as_str) == Some(method.as_str())) else {
                    fail(&mut report, "UNRESOLVED", "EXPANSION_OPTION_MISSING", &format!("no public proposal option for method {method}"));
                    terminal = "UNRESOLVED_EXPANSION".to_owned();
                    break;
                };
                let Some(candidate_id) = option.get("candidate_family_id").and_then(Value::as_str) else {
                    fail(&mut report, "FAIL", "EXPANSION_CANDIDATE_MISSING", "expansion option lacks candidate_family_id");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                if !option.get("proposal_sha256").and_then(Value::as_str).is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())) {
                    fail(&mut report, "FAIL", "PROPOSAL_DIGEST_INVALID", "expansion proposal requires a SHA-256 digest");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                if !option.get("evidence_refs").and_then(Value::as_array).is_some_and(|a| !a.is_empty()) {
                    fail(&mut report, "UNRESOLVED", "PROPOSAL_EVIDENCE_MISSING", "expansion proposal has no evidence references");
                    terminal = "UNRESOLVED_EXPANSION".to_owned();
                    break;
                }
                if visible.contains(candidate_id) || !family_by_id.contains_key(candidate_id) {
                    fail(&mut report, "FAIL", "EXPANSION_CANDIDATE_INVALID", "expansion target must be a registered, not-yet-visible public family");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }

                // Charge generation/expansion before the proposed family is revealed to the policy.
                cost_used += tariff.cost;
                seconds_used += tariff.seconds;
                visible.insert(candidate_id.to_owned());
                expanded_candidate_ids.push(candidate_id.to_owned());
                expansion_used = true;
                report["events"].as_array_mut().unwrap().push(json!({
                    "event_seq": seq,
                    "action_type": "expand_space",
                    "expansion_method": method,
                    "candidate_family_id": candidate_id,
                    "proposal_sha256": option.get("proposal_sha256").cloned().unwrap_or(Value::Null),
                    "decision_view_sha256": view_digest,
                    "cost_units_charged": tariff.cost,
                    "duration_seconds_charged": tariff.seconds,
                    "observed_outcome": null
                }));
                terminal = "BUDGET_OR_CANDIDATES_PENDING".to_owned();
            }
        }
    }

    // If every visible candidate was evaluated, the loop may finish on STOP instead of a budget error.
    if terminal == "BUDGET_OR_CANDIDATES_PENDING" && report.get("status").and_then(Value::as_str) == Some("PASS") {
        terminal = "COMPLETED".to_owned();
    }
    report["evaluations"] = json!(observations.len());
    report["expansions"] = json!(expanded_candidate_ids.len());
    report["verified_feasible_discoveries"] = json!(feasible_hits);
    report["false_feasible_predictions"] = json!(false_feasible);
    report["best_verified_feasible_utility"] = best_utility.map_or(Value::Null, |v| json!(v));
    report["first_feasible_cost_units"] = first_feasible_cost.map_or(Value::Null, |v| json!(v));
    report["cost_units_charged"] = json!(cost_used);
    report["duration_seconds_charged"] = json!(seconds_used);
    report["terminal_disposition"] = json!(terminal);
    report
}

fn run_all(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let input_digest = sha256(public_raw);
    let oracle_digest = sha256(oracle_raw);
    let mut top = json!({
        "status": "PASS",
        "audit_kind": "synthetic_live_policy_event_loop",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": input_digest,
        "oracle_sha256": oracle_digest,
        "policy_count": POLICY_CLASSES.len(),
        "environment_count": 0,
        "episodes_checked": 0,
        "episodes": [],
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });
    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut top, "FAIL", "PUBLIC_JSON_INVALID", &e.to_string());
            return top;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut top, "FAIL", "ORACLE_JSON_INVALID", &e.to_string());
            return top;
        }
    };
    only_keys(&public, &["schema_version","benchmark_id","evidence_class","claim_ceiling","budgets","policy_contract","candidate_families","expansion_options"], "public", &mut top);
    only_keys(&oracle, &["schema_version","benchmark_id","evidence_class","claim_ceiling","action_cost_catalog","environments"], "oracle", &mut top);
    if public.get("schema_version").and_then(Value::as_u64) != Some(1)
        || oracle.get("schema_version").and_then(Value::as_u64) != Some(1)
        || public.get("evidence_class").and_then(Value::as_str) != Some("synthetic")
        || oracle.get("evidence_class").and_then(Value::as_str) != Some("synthetic_evaluator_only") {
        fail(&mut top, "FAIL", "INPUT_CONTRACT_INVALID", "schema_version or evidence_class is invalid");
        return top;
    }
    if public.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID) {
        fail(&mut top, "FAIL", "BENCHMARK_ID_MISMATCH", "public catalog and sealed oracle must use the live-loop benchmark ID");
        return top;
    }
    if public.get("claim_ceiling").and_then(Value::as_str).is_none()
        || oracle.get("claim_ceiling").and_then(Value::as_str).is_none() {
        fail(&mut top, "FAIL", "CLAIM_CEILING_MISSING", "both inputs must declare a claim ceiling");
    }
    let Some(environments) = oracle.get("environments").and_then(Value::as_array) else {
        fail(&mut top, "FAIL", "ENVIRONMENTS_REQUIRED", "sealed oracle has no environments array");
        return top;
    };
    let mut episodes = Vec::new();
    for env in environments {
        let Some(env_id) = env.get("environment_id").and_then(Value::as_str) else {
            fail(&mut top, "FAIL", "ENVIRONMENT_ID_MISSING", "every evaluator environment needs an ID");
            continue;
        };
        for policy in POLICY_CLASSES {
            let mut episode = run_one(&public, &oracle, env_id, policy);
            if let Some(o) = episode.as_object_mut() {
                o.insert("input_sha256".to_owned(), json!(input_digest));
                o.insert("oracle_sha256".to_owned(), json!(oracle_digest));
            }
            if episode.get("status").and_then(Value::as_str) == Some("FAIL") {
                fail(&mut top, "FAIL", "EPISODE_FAILED", &format!("episode {env_id}/{policy} failed protocol checks"));
            } else if episode.get("status").and_then(Value::as_str) == Some("UNRESOLVED") {
                fail(&mut top, "UNRESOLVED", "EPISODE_UNRESOLVED", &format!("episode {env_id}/{policy} could not be fully scored"));
            }
            episodes.push(episode);
        }
    }
    top["environment_count"] = json!(environments.len());
    top["episodes_checked"] = json!(episodes.len());
    top["episodes"] = json!(episodes);
    top
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: run-search-space-loop <public-policy-catalog.json> <sealed-outcome-oracle.json>");
        return ExitCode::from(64);
    }
    let public_raw = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(e) => { eprintln!("cannot read {}: {e}", args[1]); return ExitCode::from(2); }
    };
    let oracle_raw = match fs::read(&args[2]) {
        Ok(bytes) => bytes,
        Err(e) => { eprintln!("cannot read {}: {e}", args[2]); return ExitCode::from(2); }
    };
    let report = run_all(&public_raw, &oracle_raw);
    let status = report.get("status").and_then(Value::as_str).unwrap_or("FAIL");
    let code = match status { "PASS" => 0, "UNRESOLVED" => 3, _ => 2 };
    println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/search-space-live-policy-catalog.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/search-space-live-outcome-oracle.synthetic.json");

    fn values() -> (Value, Value) {
        (serde_json::from_slice(PUBLIC).unwrap(), serde_json::from_slice(ORACLE).unwrap())
    }
    fn run_env(public: &Value, oracle: &Value, env: &str, policy: &str) -> Value {
        run_one(public, oracle, env, policy)
    }
    fn episode<'a>(all: &'a Value, env: &str, policy: &str) -> &'a Value {
        all["episodes"].as_array().unwrap().iter().find(|e| e["environment_id"] == env && e["policy_class"] == policy).unwrap()
    }
    fn action_signature(events: &[Value]) -> Vec<(String, String)> {
        events.iter().map(|e| (
            e["action_type"].as_str().unwrap_or("").to_owned(),
            if e["action_type"] == "expand_space" {
                e["expansion_method"].as_str().unwrap_or("").to_owned()
            } else {
                e["candidate_family_id"].as_str().unwrap_or("").to_owned()
            }
        )).collect()
    }

    #[test]
    fn live_loop_runs_all_policy_regime_pairs_with_budget_parity() {
        let report = run_all(PUBLIC, ORACLE);
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["episodes_checked"], 20);
        assert_eq!(report["environment_count"], 4);
        for e in report["episodes"].as_array().unwrap() {
            assert_eq!(e["cost_units_charged"], 5.0, "{}", e["campaign_id"]);
            assert!(e["duration_seconds_charged"].as_f64().unwrap() <= 25.0, "{}", e["campaign_id"]);
        }
    }

    #[test]
    fn adaptive_expansion_is_triggered_by_revealed_screen_result() {
        let (public, oracle) = values();
        let helpful = run_env(&public, &oracle, "helpful", "random_expansion");
        let neutral = run_env(&public, &oracle, "neutral", "random_expansion");
        assert_eq!(helpful["expansions"], 1);
        assert_eq!(neutral["expansions"], 0);
        assert_eq!(helpful["events"][0]["candidate_family_id"], "glass-a");
        assert_eq!(helpful["events"][1]["action_type"], "expand_space");
        assert_eq!(neutral["events"][0]["candidate_family_id"], "glass-a");
        assert_eq!(neutral["events"][1]["action_type"], "evaluate_candidate");
    }

    #[test]
    fn static_expansion_is_separated_from_observation_conditioned_expansion() {
        let (public, oracle) = values();
        let static = run_env(&public, &oracle, "harmful", "static_prior_expansion");
        let adaptive = run_env(&public, &oracle, "harmful", "prior_guided_expansion");
        assert_eq!(static["events"][0]["action_type"], "expand_space");
        assert_eq!(adaptive["events"][0]["action_type"], "evaluate_candidate");
        assert_eq!(static["expansions"], 1);
        assert_eq!(adaptive["expansions"], 0);
        assert_eq!(static["best_verified_feasible_utility"], 6.0);
        assert_eq!(adaptive["best_verified_feasible_utility"], 8.0);
    }

    #[test]
    fn prior_guided_policy_surfaces_misleading_prior_false_feasible() {
        let (public, oracle) = values();
        let report = run_env(&public, &oracle, "misleading_prior", "prior_guided_expansion");
        assert_eq!(report["expansions"], 1);
        assert_eq!(report["false_feasible_predictions"], 1);
        assert!(report["events"].as_array().unwrap().iter().any(|e| {
            e["candidate_family_id"] == "llm-family" && e["observed_outcome"]["feasible"] == false
        }));
    }

    #[test]
    fn hidden_outcome_change_cannot_change_actions_before_that_outcome_is_revealed() {
        let (public, oracle) = values();
        let original = run_env(&public, &oracle, "helpful", "random_expansion");
        let mut changed = oracle.clone();
        let env = changed["environments"].as_array_mut().unwrap().iter_mut().find(|e| e["environment_id"] == "helpful").unwrap();
        let row = env["outcomes"].as_array_mut().unwrap().iter_mut().find(|r| r["candidate_family_id"] == "random-family").unwrap();
        row["feasible"] = json!(false);
        row["utility"] = json!(0.0);
        let mutated = run_env(&public, &changed, "helpful", "random_expansion");
        let orig_events = original["events"].as_array().unwrap();
        let mutated_events = mutated["events"].as_array().unwrap();
        assert_eq!(action_signature(&orig_events[..3]), action_signature(&mutated_events[..3]));
        assert_eq!(orig_events[2]["decision_view_sha256"], mutated_events[2]["decision_view_sha256"]);
        assert_ne!(orig_events[2]["observed_outcome"], mutated_events[2]["observed_outcome"]);
        assert_eq!(orig_events[3]["candidate_family_id"], "glass-b");
        assert_eq!(mutated_events[3]["candidate_family_id"], "glass-c");
    }

    #[test]
    fn policy_view_does_not_contain_evaluator_environment_or_unseen_outcomes() {
        let (public, _oracle) = values();
        let initial: BTreeSet<String> = public["candidate_families"].as_array().unwrap().iter()
            .filter(|c| c["initially_visible"] == true)
            .filter_map(|c| c["id"].as_str().map(str::to_owned)).collect();
        let view = build_public_view(&public, "random_expansion", &initial, &HashSet::new(), &[], &[], false, 5.0, 25.0);
        let text = serde_json::to_string(&view).unwrap();
        assert!(!text.contains("helpful"));
        assert!(!text.contains("harmful"));
        assert!(!text.contains("outcomes"));
        assert!(!text.contains("utility"));
    }

    #[test]
    fn hidden_outcomes_cannot_be_injected_into_public_catalog() {
        let (mut public, oracle) = values();
        public["candidate_families"][0]["hidden_outcome"] = json!({"feasible":true,"utility":999999.0});
        let report = run_all(serde_json::to_string(&public).unwrap().as_bytes(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|f| f["code"] == "EPISODE_FAILED"));
    }

    #[test]
    fn candidate_cannot_be_evaluated_before_expansion_reveals_it() {
        let (public, oracle) = values();
        let mut visible = BTreeSet::new();
        visible.insert("glass-a".to_owned());
        let view = json!({
            "observations": [{"candidate_family_id":"glass-a","feasible":true,"utility":4.0}],
            "visible_candidates": [{"id":"glass-a","evaluated":true}],
            "expansion_used": false,
            "expanded_candidate_ids": [],
            "public_thresholds": {"random_expansion": 5.5},
            "remaining_cost_units": 4.0,"remaining_duration_seconds":20.0
        });
        let action = choose_action("random_expansion", &view, true);
        assert_eq!(action, Action::Stop("NO_UNMEASURED_VISIBLE_CANDIDATES".to_owned()));
        let report = run_env(&public, &oracle, "helpful", "random_expansion");
        assert_eq!(report["events"][1]["action_type"], "expand_space");
        assert_eq!(report["events"][2]["candidate_family_id"], "random-family");
    }

    #[test]
    fn evaluator_owned_costs_are_charged_before_outcome_exposure() {
        let (public, oracle) = values();
        let report = run_env(&public, &oracle, "helpful", "prior_guided_expansion");
        assert_eq!(report["events"][0]["cost_units_charged"], 1.0);
        assert_eq!(report["events"][0]["observed_outcome"]["utility"], 4.0);
        assert_eq!(report["events"][1]["cost_units_charged"], 3.0);
        assert_eq!(report["events"][1]["observed_outcome"], Value::Null);
        assert_eq!(report["events"][2]["cost_units_charged"], 1.0);
        assert_eq!(report["cost_units_charged"], 5.0);
    }

    #[test]
    fn missing_selected_outcome_is_unresolved_not_zero() {
        let (public, mut oracle) = values();
        let env = oracle["environments"].as_array_mut().unwrap().iter_mut().find(|e| e["environment_id"] == "helpful").unwrap();
        env["outcomes"].as_array_mut().unwrap().retain(|r| r["candidate_family_id"] != "llm-family");
        let report = run_env(&public, &oracle, "helpful", "prior_guided_expansion");
        assert_eq!(report["status"], "UNRESOLVED");
        assert_eq!(report["terminal_disposition"], "UNRESOLVED_OUTCOME");
        assert!(report["best_verified_feasible_utility"].is_null());
    }

    #[test]
    fn duplicate_candidate_ids_fail_closed() {
        let (mut public, oracle) = values();
        let first = public["candidate_families"][0].clone();
        public["candidate_families"].as_array_mut().unwrap().push(first);
        let report = run_all(serde_json::to_string(&public).unwrap().as_bytes(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "FAIL");
        assert!(report["episodes"].as_array().unwrap().iter().any(|e| e["status"] == "FAIL"));
    }

    #[test]
    fn current_loop_is_deterministic_for_exact_public_and_oracle_bytes() {
        let first = run_all(PUBLIC, ORACLE);
        let second = run_all(PUBLIC, ORACLE);
        assert_eq!(first, second);
    }
}
