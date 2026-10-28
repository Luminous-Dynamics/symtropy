use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

const BENCHMARK_ID: &str = "glass-space-expansion-live-loop-synthetic-v0-1";
const POLICY_CLASSES: [&str; 6] = [
    "fixed_space",
    "gp_ei_fixed_space",
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


fn candidate_features(candidate: &Value) -> Option<[f64; 2]> {
    let features = candidate.get("features")?.as_array()?;
    if features.len() != 2 {
        return None;
    }
    let x = features[0].as_f64()?;
    let y = features[1].as_f64()?;
    if !x.is_finite() || !y.is_finite() {
        return None;
    }
    Some([x, y])
}

fn rbf_kernel(a: [f64; 2], b: [f64; 2], lengthscale: f64) -> f64 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let distance_squared = dx * dx + dy * dy;
    (-0.5 * distance_squared / (lengthscale * lengthscale)).exp()
}

fn normal_pdf(z: f64) -> f64 {
    (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

// Abramowitz-Stegun-style approximation. Adequate for deterministic ranking in this synthetic finite-pool baseline.
fn normal_cdf(z: f64) -> f64 {
    let x = z.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    let poly = t * (0.319_381_530
        + t * (-0.356_563_782
            + t * (1.781_477_937
                + t * (-1.821_255_978 + t * 1.330_274_429))));
    let tail = normal_pdf(x) * poly;
    if z >= 0.0 { 1.0 - tail } else { tail }
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut residual = matrix[i][j];
            for k in 0..j {
                residual -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !residual.is_finite() || residual <= 1e-12 {
                    return None;
                }
                lower[i][j] = residual.sqrt();
            } else {
                if lower[j][j] <= 0.0 {
                    return None;
                }
                lower[i][j] = residual / lower[j][j];
            }
        }
    }
    Some(lower)
}

fn solve_from_cholesky(lower: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = lower.len();
    if n == 0 || rhs.len() != n {
        return None;
    }
    let mut forward = vec![0.0; n];
    for i in 0..n {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * forward[j];
        }
        if lower[i][i] <= 0.0 {
            return None;
        }
        forward[i] = value / lower[i][i];
    }

    let mut solution = vec![0.0; n];
    for i in (0..n).rev() {
        let mut value = forward[i];
        for j in (i + 1)..n {
            value -= lower[j][i] * solution[j];
        }
        solution[i] = value / lower[i][i];
    }
    solution.iter().all(|x| x.is_finite()).then_some(solution)
}

fn gp_ei_score_candidate(view: &Value, candidate_id: &str) -> Option<f64> {
    let candidates = view.get("visible_candidates")?.as_array()?;
    let train_observations = view.get("observations")?.as_array()?;
    if train_observations.is_empty() {
        return None;
    }
    let candidate_map: HashMap<&str, &Value> = candidates.iter()
        .filter_map(|candidate| Some((candidate.get("id")?.as_str()?, candidate)))
        .collect();

    let mut train_x = Vec::<[f64; 2]>::new();
    let mut train_y = Vec::<f64>::new();
    for observation in train_observations {
        let id = observation.get("candidate_family_id")?.as_str()?;
        let candidate = candidate_map.get(id)?;
        train_x.push(candidate_features(candidate)?);
        let utility = observation.get("utility")?.as_f64()?;
        if !utility.is_finite() {
            return None;
        }
        // This baseline is explicitly normalized for the frozen synthetic utility range [0, 10].
        train_y.push(utility / 10.0);
    }
    let target = *candidate_map.get(candidate_id)?;
    let target_x = candidate_features(target)?;

    // Frozen finite-pool GP-EI v0.1: unit RBF amplitude, lengthscale 0.85, observation-noise variance 0.05, xi 0.01.
    const LENGTHSCALE: f64 = 0.85;
    const NOISE_VARIANCE: f64 = 0.05;
    const XI: f64 = 0.01;
    let n = train_x.len();
    let mut kernel = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            kernel[i][j] = rbf_kernel(train_x[i], train_x[j], LENGTHSCALE);
        }
        kernel[i][i] += NOISE_VARIANCE;
    }
    let lower = cholesky_lower(&kernel)?;
    let alpha = solve_from_cholesky(&lower, &train_y)?;
    let k_star: Vec<f64> = train_x.iter().map(|x| rbf_kernel(*x, target_x, LENGTHSCALE)).collect();
    let mean: f64 = k_star.iter().zip(alpha.iter()).map(|(k, a)| k * a).sum();

    // var(f*) = k(x*,x*) - || L^{-1}k* ||^2, with unit prior variance.
    let mut forward = vec![0.0; n];
    for i in 0..n {
        let mut value = k_star[i];
        for j in 0..i {
            value -= lower[i][j] * forward[j];
        }
        forward[i] = value / lower[i][i];
    }
    let variance = (1.0 - forward.iter().map(|v| v * v).sum::<f64>()).max(0.0);
    let sigma = variance.sqrt();
    let best_y = train_y.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !mean.is_finite() || !sigma.is_finite() || !best_y.is_finite() {
        return None;
    }
    let improvement = mean - best_y - XI;
    if sigma <= 1e-12 {
        return Some(improvement.max(0.0));
    }
    let z = improvement / sigma;
    let ei = improvement * normal_cdf(z) + sigma * normal_pdf(z);
    ei.is_finite().then_some(ei.max(0.0))
}

fn gp_ei_next_candidate(view: &Value) -> Option<String> {
    let candidates = view.get("visible_candidates")?.as_array()?;
    candidates.iter()
        .filter(|candidate| candidate.get("evaluated").and_then(Value::as_bool) == Some(false))
        .filter_map(|candidate| {
            let id = candidate.get("id")?.as_str()?;
            let score = gp_ei_score_candidate(view, id)?;
            Some((id.to_owned(), score))
        })
        .max_by(|a, b| {
            a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0))
        })
        .map(|(id, _)| id)
}

fn choose_action(policy: &str, view: &Value, expansion_decision_made: bool) -> Action {
    let expanded = view.get("expansion_used").and_then(Value::as_bool).unwrap_or(false);
    let observations = view.get("observations").and_then(Value::as_array).cloned().unwrap_or_default();

    // Static expansion is a deliberately non-adaptive control. It expands before observing any result.
    if policy == "static_prior_expansion" && !expanded {
        return Action::Expand("prior_guided".to_owned());
    }

    // Observation-conditioned arms begin with the same public screening candidate; the static-expansion control intentionally expands first.
    if observations.is_empty() {
        let candidates = view.get("visible_candidates").and_then(Value::as_array).cloned().unwrap_or_default();
        let rank_field = if expanded { "selector_score" } else { "screen_priority" };
        let best = candidates.into_iter().filter(|c| c.get("evaluated").and_then(Value::as_bool) == Some(false))
            .max_by(|a, b| {
                a.get(rank_field).and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&b.get(rank_field).and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
            });
        return best.and_then(|c| c.get("id").and_then(Value::as_str).map(|s| Action::Evaluate(s.to_owned())))
            .unwrap_or_else(|| Action::Stop("NO_UNMEASURED_VISIBLE_CANDIDATES".to_owned()));
    }

    // Fixed-space Gaussian-process expected improvement: rank only still-visible candidates
    // using the GP posterior computed from observations revealed by earlier actions.
    if policy == "gp_ei_fixed_space" {
        return gp_ei_next_candidate(view)
            .map(Action::Evaluate)
            .unwrap_or_else(|| Action::Stop("GP_EI_POSTERIOR_UNRESOLVED".to_owned()));
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
                "features": c.get("features").cloned().unwrap_or(Value::Null),
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
        if tariffs.insert(key.to_owned(), Tariff { cost, seconds }).is_some() {
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

fn run_one_with_executor<F>(catalog: &Value, oracle: &Value, environment_id: &str, policy: &str, executor: &mut F) -> Value
where
    F: FnMut(&str, &Value, bool) -> Result<Action, String>,
{
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
        "oracle_best_feasible_utility": null,
        "terminal_simple_regret": null,
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
    let oracle_best_feasible_utility = outcomes.values()
        .filter(|(feasible, _)| *feasible)
        .map(|(_, utility)| *utility)
        .reduce(f64::max);
    report["oracle_best_feasible_utility"] = oracle_best_feasible_utility.map_or(Value::Null, |v| json!(v));
    if oracle_best_feasible_utility.is_none() {
        fail(&mut report, "UNRESOLVED", "NO_FEASIBLE_ORACLE_REFERENCE", "sealed environment has no feasible reference outcome, so simple regret is undefined");
    }
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
        only_keys(family, &["id", "domain", "initially_visible", "screen_priority", "predicted_feasible", "selector_scores", "failure_fallback_score", "features"], &format!("catalog.candidate_families[{i}]"), &mut report);
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
        let action = match executor(policy, &view, expansion_decision_made) {
            Ok(action) => action,
            Err(error) => {
                fail(&mut report, "FAIL", "POLICY_EXECUTOR_FAILED", &error);
                terminal = "POLICY_EXECUTOR_FAILED".to_owned();
                break;
            }
        };
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
    let simple_regret = match (oracle_best_feasible_utility, best_utility) {
        (Some(optimum), Some(incumbent)) => Some((optimum - incumbent).max(0.0)),
        _ => None,
    };
    report["terminal_simple_regret"] = simple_regret.map_or(Value::Null, |v| json!(v));
    report["first_feasible_cost_units"] = first_feasible_cost.map_or(Value::Null, |v| json!(v));
    report["cost_units_charged"] = json!(cost_used);
    report["duration_seconds_charged"] = json!(seconds_used);
    report["terminal_disposition"] = json!(terminal);
    report
}

fn run_one(catalog: &Value, oracle: &Value, environment_id: &str, policy: &str) -> Value {
    let mut direct = |policy: &str, view: &Value, expansion_decision_made: bool| {
        Ok(choose_action(policy, view, expansion_decision_made))
    };
    run_one_with_executor(catalog, oracle, environment_id, policy, &mut direct)
}

fn run_all_with_executor<F>(public_raw: &[u8], oracle_raw: &[u8], executor: &mut F) -> Value
where
    F: FnMut(&str, &Value, bool) -> Result<Action, String>,
{
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
            let mut episode = run_one_with_executor(&public, &oracle, env_id, policy, executor);
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


fn build_replay_view(
    catalog: &Value,
    policy: &str,
    visible: &BTreeSet<String>,
    evaluated: &HashSet<String>,
    observations: &[Value],
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
                "features": c.get("features").cloned().unwrap_or(Value::Null),
                "screen_priority": c.get("screen_priority").cloned().unwrap_or(Value::Null),
                "predicted_feasible": c.get("predicted_feasible").cloned().unwrap_or(Value::Null),
                "selector_score": c.get("selector_scores").and_then(|v| v.get(policy)).cloned().unwrap_or(Value::Null),
                "failure_fallback_score": c.get("failure_fallback_score").cloned().unwrap_or(Value::Null),
                "is_expansion_candidate": c.get("initially_visible").and_then(Value::as_bool) == Some(false),
                "evaluated": evaluated.contains(id)
            })
        }).collect::<Vec<_>>();
    json!({
        "policy_id": policy,
        "visible_candidates": visible_candidates,
        "observations": observations,
        "expanded_candidate_ids": expanded_candidate_ids,
        "expansion_used": expansion_used,
        "available_expansion_methods": if expansion_used { Vec::<String>::new() } else { vec!["random".to_owned(), "rule_based".to_owned(), "prior_guided".to_owned()] },
        "public_thresholds": catalog.get("policy_contract").and_then(|v| v.get("thresholds")).cloned().unwrap_or_else(|| json!({})),
        "remaining_cost_units": remaining_cost,
        "remaining_duration_seconds": remaining_seconds
    })
}

fn audit_replay(report_raw: &[u8], public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let report_digest = sha256(report_raw);
    let public_digest = sha256(public_raw);
    let oracle_digest = sha256(oracle_raw);
    let mut audit = json!({
        "status": "PASS",
        "audit_kind": "independent_live_policy_transcript_replay",
        "benchmark_id": BENCHMARK_ID,
        "report_sha256": report_digest,
        "input_sha256": public_digest,
        "oracle_sha256": oracle_digest,
        "episodes_checked": 0,
        "events_checked": 0,
        "evaluations_replayed": 0,
        "expansions_replayed": 0,
        "cost_units_recomputed": 0.0,
        "duration_seconds_recomputed": 0.0,
        "findings": [],
        "claim_ceiling": "Transcript integrity and declared synthetic outcome replay only; no process isolation, material truth, or industrial result is established."
    });
    let report: Value = match serde_json::from_slice(report_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut audit, "FAIL", "REPORT_JSON_INVALID", &e.to_string());
            return audit;
        }
    };
    let catalog: Value = match serde_json::from_slice(public_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut audit, "FAIL", "PUBLIC_JSON_INVALID", &e.to_string());
            return audit;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(v) => v,
        Err(e) => {
            fail(&mut audit, "FAIL", "ORACLE_JSON_INVALID", &e.to_string());
            return audit;
        }
    };
    only_keys(&report, &["status","audit_kind","benchmark_id","input_sha256","oracle_sha256","policy_count","environment_count","episodes_checked","episodes","findings","claim_ceiling"], "run_report", &mut audit);
    if report.get("audit_kind").and_then(Value::as_str) != Some("synthetic_live_policy_event_loop")
        || report.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID) {
        fail(&mut audit, "FAIL", "RUN_REPORT_KIND_INVALID", "input report is not a live policy event-loop report for this benchmark");
    }
    if report.get("policy_count").and_then(Value::as_u64) != Some(POLICY_CLASSES.len() as u64) {
        fail(&mut audit, "FAIL", "POLICY_COUNT_MISMATCH", "run report's policy count does not match the registered policy list");
    }
    if report.get("input_sha256").and_then(Value::as_str) != Some(public_digest.as_str()) {
        fail(&mut audit, "FAIL", "PUBLIC_DIGEST_MISMATCH", "live-loop report does not bind to the exact public catalog bytes supplied to replay");
    }
    if report.get("oracle_sha256").and_then(Value::as_str) != Some(oracle_digest.as_str()) {
        fail(&mut audit, "FAIL", "ORACLE_DIGEST_MISMATCH", "live-loop report does not bind to the exact sealed oracle bytes supplied to replay");
    }
    if catalog.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID) {
        fail(&mut audit, "FAIL", "BENCHMARK_ID_MISMATCH", "public catalog and oracle benchmark IDs do not match the replay contract");
    }
    let max_cost = catalog.get("budgets").and_then(|v| v.get("max_cost_units")).and_then(Value::as_f64).unwrap_or(f64::NAN);
    let max_seconds = catalog.get("budgets").and_then(|v| v.get("max_duration_seconds")).and_then(Value::as_f64).unwrap_or(f64::NAN);
    if !max_cost.is_finite() || max_cost <= 0.0 || !max_seconds.is_finite() || max_seconds <= 0.0 {
        fail(&mut audit, "FAIL", "BUDGET_INVALID", "public budget caps are missing or invalid");
        return audit;
    }

    let families = catalog.get("candidate_families").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut family_by_id: HashMap<String, Value> = HashMap::new();
    for (i, family) in families.iter().enumerate() {
        let Some(id) = family.get("id").and_then(Value::as_str) else {
            fail(&mut audit, "FAIL", "FAMILY_ID_MISSING", &format!("catalog candidate {i} lacks an ID"));
            continue;
        };
        if family_by_id.insert(id.to_owned(), family.clone()).is_some() {
            fail(&mut audit, "FAIL", "FAMILY_ID_DUPLICATE", &format!("catalog candidate ID {id} is duplicated"));
        }
    }
    let tariffs = get_tariffs(&oracle, &mut audit);
    let Some(oracle_envs) = oracle.get("environments").and_then(Value::as_array) else {
        fail(&mut audit, "FAIL", "ENVIRONMENTS_REQUIRED", "sealed oracle has no environment array");
        return audit;
    };
    let Some(episodes) = report.get("episodes").and_then(Value::as_array) else {
        fail(&mut audit, "FAIL", "EPISODES_REQUIRED", "run report has no episodes array");
        return audit;
    };

    let mut episodes_checked = 0usize;
    let mut events_checked = 0usize;
    let mut policy_actions_checked = 0usize;
    let mut evals_replayed = 0usize;
    let mut expansions_replayed = 0usize;
    let mut total_cost = 0.0f64;
    let mut total_seconds = 0.0f64;
    let mut episode_ids = HashSet::new();

    for (episode_index, episode) in episodes.iter().enumerate() {
        let env_id = episode.get("environment_id").and_then(Value::as_str).unwrap_or("");
        let policy = episode.get("policy_class").and_then(Value::as_str).unwrap_or("");
        let campaign_id = episode.get("campaign_id").and_then(Value::as_str).unwrap_or("");
        if !valid_slug(env_id) || !allowed_policy(policy) || campaign_id.is_empty() {
            fail(&mut audit, "FAIL", "EPISODE_IDENTITY_INVALID", &format!("episode {episode_index} has invalid identity fields"));
            continue;
        }
        if !episode_ids.insert(campaign_id.to_owned()) {
            fail(&mut audit, "FAIL", "EPISODE_ID_DUPLICATE", &format!("duplicate campaign ID {campaign_id}"));
        }
        if episode.get("status").and_then(Value::as_str) == Some("FAIL") {
            fail(&mut audit, "FAIL", "SOURCE_EPISODE_FAILED", &format!("episode {campaign_id} is already marked FAIL"));
        }
        if episode.get("input_sha256").and_then(Value::as_str) != Some(public_digest.as_str())
            || episode.get("oracle_sha256").and_then(Value::as_str) != Some(oracle_digest.as_str()) {
            fail(&mut audit, "FAIL", "EPISODE_DIGEST_MISMATCH", &format!("episode {campaign_id} does not bind to the supplied inputs"));
        }
        let Some(env) = oracle_envs.iter().find(|e| e.get("environment_id").and_then(Value::as_str) == Some(env_id)) else {
            fail(&mut audit, "UNRESOLVED", "ENVIRONMENT_MISSING", &format!("sealed oracle has no environment {env_id}"));
            continue;
        };
        let outcome_rows = env.get("outcomes").and_then(Value::as_array).cloned().unwrap_or_default();
        let outcome_by_id: HashMap<String, (bool, f64)> = outcome_rows.iter().filter_map(|row| {
            let id = row.get("candidate_family_id")?.as_str()?.to_owned();
            let feasible = row.get("feasible")?.as_bool()?;
            let utility = row.get("utility")?.as_f64()?;
            utility.is_finite().then_some((id, (feasible, utility)))
        }).collect();
        let oracle_best_feasible_utility = outcome_by_id.values()
            .filter(|(feasible, _)| *feasible)
            .map(|(_, utility)| *utility)
            .reduce(f64::max);
        if !close(episode.get("oracle_best_feasible_utility").and_then(Value::as_f64), oracle_best_feasible_utility) {
            fail(&mut audit, "FAIL", "ORACLE_BEST_UTILITY_MISMATCH", &format!("episode {campaign_id} oracle optimum does not match sealed feasible outcomes"));
        }
        let mut visible: BTreeSet<String> = families.iter()
            .filter(|f| f.get("initially_visible").and_then(Value::as_bool) == Some(true))
            .filter_map(|f| f.get("id").and_then(Value::as_str).map(str::to_owned)).collect();
        let mut evaluated = HashSet::new();
        let mut observations = Vec::<Value>::new();
        let mut expanded_ids = Vec::<String>::new();
        let mut expansion_used = false;
        let mut expansion_decision_made = policy == "fixed_space" || policy == "static_prior_expansion";
        let mut cost_used = 0.0f64;
        let mut seconds_used = 0.0f64;
        let mut feasible_hits = 0usize;
        let mut false_feasible = 0usize;
        let mut best_utility: Option<f64> = None;
        let mut first_feasible_cost: Option<f64> = None;
        let Some(events) = episode.get("events").and_then(Value::as_array) else {
            fail(&mut audit, "FAIL", "EVENTS_REQUIRED", &format!("episode {campaign_id} has no event array"));
            continue;
        };
        if events.is_empty() {
            fail(&mut audit, "FAIL", "EVENTS_EMPTY", &format!("episode {campaign_id} has no events"));
        }

        for (event_index, event) in events.iter().enumerate() {
            let event_type = event.get("action_type").and_then(Value::as_str).unwrap_or("");
            let allowed_keys: &[&str] = match event_type {
                "evaluate_candidate" => &["event_seq","action_type","candidate_family_id","decision_view_sha256","cost_units_charged","duration_seconds_charged","predicted_feasible_before_observation","observed_outcome"],
                "expand_space" => &["event_seq","action_type","expansion_method","candidate_family_id","proposal_sha256","decision_view_sha256","cost_units_charged","duration_seconds_charged","observed_outcome"],
                _ => {
                    fail(&mut audit, "FAIL", "EVENT_TYPE_INVALID", &format!("episode {campaign_id} has invalid action type {event_type}"));
                    continue;
                }
            };
            only_keys(event, allowed_keys, &format!("episode {campaign_id}.events[{event_index}]"), &mut audit);
            if event.get("event_seq").and_then(Value::as_u64) != Some((event_index + 1) as u64) {
                fail(&mut audit, "FAIL", "EVENT_SEQUENCE_INVALID", &format!("episode {campaign_id} event sequence is not consecutive"));
            }
            let Some(candidate_id) = event.get("candidate_family_id").and_then(Value::as_str) else {
                fail(&mut audit, "FAIL", "EVENT_CANDIDATE_ID_MISSING", &format!("episode {campaign_id} event {event_index} lacks a candidate ID"));
                continue;
            };

            let view = build_replay_view(
                &catalog, policy, &visible, &evaluated, &observations, &expanded_ids,
                expansion_used, (max_cost - cost_used).max(0.0), (max_seconds - seconds_used).max(0.0)
            );
            let expected_view_digest = sha256(&serde_json::to_vec(&view).unwrap_or_default());
            if event.get("decision_view_sha256").and_then(Value::as_str) != Some(expected_view_digest.as_str()) {
                fail(&mut audit, "FAIL", "DECISION_VIEW_DIGEST_MISMATCH", &format!("episode {campaign_id} event {event_index} view digest is not reproducible from the prior public state"));
            }

            let consideration_due = !expansion_decision_made
                && observations.len() == 1
                && method_for_policy(policy).is_some();
            let expected_action = choose_action(policy, &view, expansion_decision_made);
            if consideration_due {
                expansion_decision_made = true;
            }
            let action_matches = match &expected_action {
                Action::Evaluate(expected_id) => event_type == "evaluate_candidate" && candidate_id == expected_id,
                Action::Expand(expected_method) => {
                    event_type == "expand_space"
                        && event.get("expansion_method").and_then(Value::as_str) == Some(expected_method.as_str())
                }
                Action::Stop(reason) => {
                    fail(&mut audit, "FAIL", "POLICY_ACTION_AFTER_STOP", &format!("episode {campaign_id} event {event_index} occurs after deterministic policy would stop: {reason}"));
                    false
                }
            };
            policy_actions_checked += 1;
            if !action_matches {
                fail(&mut audit, "FAIL", "POLICY_ACTION_MISMATCH", &format!("episode {campaign_id} event {event_index} does not match the registered deterministic policy action for the reconstructed public view"));
            }

            let (tariff_key, method) = if event_type == "evaluate_candidate" {
                ("evaluate_candidate", None)
            } else {
                let Some(m) = event.get("expansion_method").and_then(Value::as_str) else {
                    fail(&mut audit, "FAIL", "EXPANSION_METHOD_REQUIRED", &format!("episode {campaign_id} expansion has no method"));
                    continue;
                };
                let Some(key) = tariff_key_for_method(m) else {
                    fail(&mut audit, "FAIL", "EXPANSION_METHOD_INVALID", &format!("episode {campaign_id} has unknown expansion method {m}"));
                    continue;
                };
                (key, Some(m))
            };
            let Some(tariff) = tariffs.get(tariff_key) else {
                fail(&mut audit, "FAIL", "TARIFF_MISSING", &format!("episode {campaign_id} missing tariff {tariff_key}"));
                continue;
            };
            let declared_cost = event.get("cost_units_charged").and_then(Value::as_f64).unwrap_or(f64::NAN);
            let declared_seconds = event.get("duration_seconds_charged").and_then(Value::as_f64).unwrap_or(f64::NAN);
            if (declared_cost - tariff.cost).abs() > 1e-9 || (declared_seconds - tariff.seconds).abs() > 1e-9 {
                fail(&mut audit, "FAIL", "ACTION_CHARGE_MISMATCH", &format!("episode {campaign_id} event {event_index} does not match evaluator-owned tariff"));
            }
            cost_used += tariff.cost;
            seconds_used += tariff.seconds;
            if cost_used > max_cost + 1e-9 || seconds_used > max_seconds + 1e-9 {
                fail(&mut audit, "FAIL", "RESOURCE_BUDGET_OVERRUN", &format!("episode {campaign_id} exceeds the registered cost or duration cap"));
            }
            if event.get("observed_outcome").is_some_and(|v| !v.is_null()) && event_type == "expand_space" {
                fail(&mut audit, "FAIL", "EXPANSION_REVEALED_OUTCOME", &format!("expansion event in {campaign_id} reveals a sealed outcome"));
            }

            if event_type == "expand_space" {
                expansions_replayed += 1;
                if expansion_used {
                    fail(&mut audit, "FAIL", "MULTIPLE_EXPANSIONS", &format!("episode {campaign_id} has more than one expansion"));
                }
                if method_for_policy(policy) != method {
                    fail(&mut audit, "FAIL", "EXPANSION_POLICY_MISMATCH", &format!("episode {campaign_id} policy {policy} did not declare method {method:?}"));
                }
                let Some(option) = catalog.get("expansion_options").and_then(Value::as_array).into_iter().flatten()
                    .find(|o| o.get("method").and_then(Value::as_str) == method) else {
                    fail(&mut audit, "UNRESOLVED", "EXPANSION_OPTION_MISSING", &format!("episode {campaign_id} has no registered method option"));
                    continue;
                };
                if option.get("candidate_family_id").and_then(Value::as_str) != Some(candidate_id)
                    || event.get("proposal_sha256").and_then(Value::as_str) != option.get("proposal_sha256").and_then(Value::as_str) {
                    fail(&mut audit, "FAIL", "EXPANSION_RECEIPT_MISMATCH", &format!("episode {campaign_id} expansion ID/digest differs from the registered proposal"));
                }
                if visible.contains(candidate_id) || !family_by_id.contains_key(candidate_id) {
                    fail(&mut audit, "FAIL", "EXPANSION_VISIBILITY_INVALID", &format!("episode {campaign_id} expands an already visible or unregistered family {candidate_id}"));
                }
                if !option.get("evidence_refs").and_then(Value::as_array).is_some_and(|a| !a.is_empty()) {
                    fail(&mut audit, "UNRESOLVED", "EXPANSION_EVIDENCE_MISSING", &format!("episode {campaign_id} expansion option has no evidence references"));
                }
                visible.insert(candidate_id.to_owned());
                expanded_ids.push(candidate_id.to_owned());
                expansion_used = true;
            } else {
                evals_replayed += 1;
                if !visible.contains(candidate_id) {
                    fail(&mut audit, "FAIL", "CANDIDATE_NOT_VISIBLE", &format!("episode {campaign_id} evaluates {candidate_id} before it was revealed"));
                }
                if !evaluated.insert(candidate_id.to_owned()) {
                    fail(&mut audit, "FAIL", "CANDIDATE_DUPLICATE_EVALUATION", &format!("episode {campaign_id} evaluates {candidate_id} more than once"));
                }
                let Some(family) = family_by_id.get(candidate_id) else {
                    fail(&mut audit, "FAIL", "CANDIDATE_UNREGISTERED", &format!("episode {campaign_id} evaluates unknown family {candidate_id}"));
                    continue;
                };
                if event.get("predicted_feasible_before_observation") != family.get("predicted_feasible") {
                    fail(&mut audit, "FAIL", "PREDICTION_RECEIPT_MISMATCH", &format!("episode {campaign_id} predicted feasibility differs from public catalog"));
                }
                let Some((feasible, utility)) = outcome_by_id.get(candidate_id).copied() else {
                    if event.get("observed_outcome").is_some_and(|v| !v.is_null()) {
                        fail(&mut audit, "FAIL", "MISSING_OUTCOME_DISCLOSED", &format!("episode {campaign_id} exposes a result absent from sealed oracle"));
                    } else {
                        fail(&mut audit, "UNRESOLVED", "SEALED_OUTCOME_MISSING", &format!("episode {campaign_id} has no oracle outcome for {candidate_id}"));
                    }
                    continue;
                };
                let expected = json!({"feasible":feasible,"utility":utility});
                if event.get("observed_outcome") != Some(&expected) {
                    fail(&mut audit, "FAIL", "OBSERVATION_ORACLE_MISMATCH", &format!("episode {campaign_id} observed result differs from sealed oracle"));
                }
                if family.get("predicted_feasible").and_then(Value::as_bool) == Some(true) && !feasible {
                    false_feasible += 1;
                }
                observations.push(json!({"candidate_family_id":candidate_id,"feasible":feasible,"utility":utility}));
                if feasible {
                    feasible_hits += 1;
                    best_utility = Some(best_utility.map_or(utility, |v| v.max(utility)));
                    if first_feasible_cost.is_none() {
                        first_feasible_cost = Some(cost_used);
                    }
                }
            }
            events_checked += 1;
        }

        if !close(episode.get("cost_units_charged").and_then(Value::as_f64), Some(cost_used)) {
            fail(&mut audit, "FAIL", "TOTAL_COST_MISMATCH", &format!("episode {campaign_id} total cost does not replay"));
        }
        if !close(episode.get("duration_seconds_charged").and_then(Value::as_f64), Some(seconds_used)) {
            fail(&mut audit, "FAIL", "TOTAL_DURATION_MISMATCH", &format!("episode {campaign_id} total duration does not replay"));
        }
        if episode.get("evaluations").and_then(Value::as_u64) != Some(observations.len() as u64) {
            fail(&mut audit, "FAIL", "EVALUATION_COUNT_MISMATCH", &format!("episode {campaign_id} evaluation count does not replay"));
        }
        if episode.get("expansions").and_then(Value::as_u64) != Some(expanded_ids.len() as u64) {
            fail(&mut audit, "FAIL", "EXPANSION_COUNT_MISMATCH", &format!("episode {campaign_id} expansion count does not replay"));
        }
        if episode.get("verified_feasible_discoveries").and_then(Value::as_u64) != Some(feasible_hits as u64) {
            fail(&mut audit, "FAIL", "FEASIBLE_HIT_COUNT_MISMATCH", &format!("episode {campaign_id} feasible-hit count does not replay"));
        }
        if episode.get("false_feasible_predictions").and_then(Value::as_u64) != Some(false_feasible as u64) {
            fail(&mut audit, "FAIL", "FALSE_FEASIBLE_COUNT_MISMATCH", &format!("episode {campaign_id} false-feasible count does not replay"));
        }
        if !close(episode.get("best_verified_feasible_utility").and_then(Value::as_f64), best_utility) {
            fail(&mut audit, "FAIL", "BEST_UTILITY_MISMATCH", &format!("episode {campaign_id} best verified feasible utility does not replay"));
        }
        let expected_simple_regret = match (oracle_best_feasible_utility, best_utility) {
            (Some(optimum), Some(incumbent)) => Some((optimum - incumbent).max(0.0)),
            _ => None,
        };
        if !close(episode.get("terminal_simple_regret").and_then(Value::as_f64), expected_simple_regret) {
            fail(&mut audit, "FAIL", "SIMPLE_REGRET_MISMATCH", &format!("episode {campaign_id} terminal simple regret does not replay"));
        }
        if !close(episode.get("first_feasible_cost_units").and_then(Value::as_f64), first_feasible_cost) {
            fail(&mut audit, "FAIL", "FIRST_FEASIBLE_COST_MISMATCH", &format!("episode {campaign_id} first feasible cost does not replay"));
        }
        total_cost += cost_used;
        total_seconds += seconds_used;
        episodes_checked += 1;
    }

    if report.get("episodes_checked").and_then(Value::as_u64) != Some(episodes.len() as u64) {
        fail(&mut audit, "FAIL", "EPISODE_COUNT_MISMATCH", "reported episode count does not match episode array length");
    }
    let expected_episode_count = oracle_envs.len() * POLICY_CLASSES.len();
    if episodes.len() != expected_episode_count {
        fail(&mut audit, "FAIL", "POLICY_ENVIRONMENT_COVERAGE_MISMATCH", &format!("expected {expected_episode_count} environment/policy pairs but replay received {}", episodes.len()));
    }
    let mut pairs = HashSet::new();
    for episode in episodes {
        let env = episode.get("environment_id").and_then(Value::as_str).unwrap_or("");
        let policy = episode.get("policy_class").and_then(Value::as_str).unwrap_or("");
        if !pairs.insert((env.to_owned(), policy.to_owned())) {
            fail(&mut audit, "FAIL", "POLICY_ENVIRONMENT_PAIR_DUPLICATE", &format!("duplicate episode for {env}/{policy}"));
        }
    }
    for env in oracle_envs {
        let env_id = env.get("environment_id").and_then(Value::as_str).unwrap_or("");
        for policy in POLICY_CLASSES {
            if !pairs.contains(&(env_id.to_owned(), policy.to_owned())) {
                fail(&mut audit, "FAIL", "POLICY_ENVIRONMENT_PAIR_MISSING", &format!("missing episode for {env_id}/{policy}"));
            }
        }
    }
    audit["episodes_checked"] = json!(episodes_checked);
    audit["events_checked"] = json!(events_checked);
    audit["policy_actions_checked"] = json!(policy_actions_checked);
    audit["evaluations_replayed"] = json!(evals_replayed);
    audit["expansions_replayed"] = json!(expansions_replayed);
    audit["cost_units_recomputed"] = json!(total_cost);
    audit["duration_seconds_recomputed"] = json!(total_seconds);
    audit
}

fn run_all(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut direct = |policy: &str, view: &Value, expansion_decision_made: bool| {
        Ok(choose_action(policy, view, expansion_decision_made))
    };
    run_all_with_executor(public_raw, oracle_raw, &mut direct)
}

fn action_to_value(action: &Action) -> Value {
    match action {
        Action::Evaluate(candidate_id) => json!({
            "action_type": "evaluate_candidate",
            "candidate_family_id": candidate_id
        }),
        Action::Expand(method) => json!({
            "action_type": "expand_space",
            "expansion_method": method
        }),
        Action::Stop(reason) => json!({
            "action_type": "stop",
            "reason": reason
        })
    }
}

fn action_from_value(value: &Value) -> Result<Action, String> {
    let Some(action_type) = value.get("action_type").and_then(Value::as_str) else {
        return Err("policy response requires action_type".to_owned());
    };
    let allowed: &[&str] = match action_type {
        "evaluate_candidate" => &["action_type", "candidate_family_id"],
        "expand_space" => &["action_type", "expansion_method"],
        "stop" => &["action_type", "reason"],
        other => return Err(format!("unrecognized policy action_type {other}")),
    };
    let Some(object) = value.as_object() else {
        return Err("policy response must be an object".to_owned());
    };
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("policy response contains undeclared field {key}"));
    }
    match action_type {
        "evaluate_candidate" => {
            let id = value.get("candidate_family_id").and_then(Value::as_str)
                .ok_or_else(|| "evaluate_candidate requires candidate_family_id".to_owned())?;
            if !valid_slug(id) {
                return Err("candidate_family_id must be a lowercase slug".to_owned());
            }
            Ok(Action::Evaluate(id.to_owned()))
        }
        "expand_space" => {
            let method = value.get("expansion_method").and_then(Value::as_str)
                .ok_or_else(|| "expand_space requires expansion_method".to_owned())?;
            if !["random", "rule_based", "prior_guided"].contains(&method) {
                return Err(format!("unsupported expansion_method {method}"));
            }
            Ok(Action::Expand(method.to_owned()))
        }
        "stop" => {
            let reason = value.get("reason").and_then(Value::as_str)
                .filter(|reason| !reason.trim().is_empty())
                .ok_or_else(|| "stop requires a non-empty reason".to_owned())?;
            Ok(Action::Stop(reason.to_owned()))
        }
        _ => unreachable!()
    }
}

fn request_policy_subprocess(
    executable: &Path,
    policy: &str,
    public_view: &Value,
    expansion_decision_made: bool,
) -> Result<Action, String> {
    let request = json!({
        "schema_version": 1,
        "policy_class": policy,
        "public_view": public_view,
        "expansion_decision_made": expansion_decision_made
    });
    let request_bytes = serde_json::to_vec(&request)
        .map_err(|e| format!("cannot serialize policy request: {e}"))?;
    let mut child = Command::new(executable)
        .arg("--policy-child")
        .env_clear()
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start policy subprocess: {e}"))?;
    {
        let stdin = child.stdin.as_mut()
            .ok_or_else(|| "policy subprocess stdin was not created".to_owned())?;
        stdin.write_all(&request_bytes)
            .map_err(|e| format!("cannot write policy request: {e}"))?;
    }
    let output = child.wait_with_output()
        .map_err(|e| format!("cannot collect policy subprocess output: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "policy subprocess exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }
    let response: Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("policy subprocess did not return one valid JSON action: {e}"))?;
    action_from_value(&response)
}

fn run_all_subprocess(public_raw: &[u8], oracle_raw: &[u8], executable: &Path) -> Value {
    let mut isolated_adapter = |policy: &str, view: &Value, expansion_decision_made: bool| {
        request_policy_subprocess(executable, policy, view, expansion_decision_made)
    };
    run_all_with_executor(public_raw, oracle_raw, &mut isolated_adapter)
}

fn run_policy_child() -> ExitCode {
    let mut raw = Vec::new();
    if let Err(error) = std::io::stdin().read_to_end(&mut raw) {
        eprintln!("cannot read policy request: {error}");
        return ExitCode::from(2);
    }
    let request: Value = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid policy request JSON: {error}");
            return ExitCode::from(2);
        }
    };
    let Some(object) = request.as_object() else {
        eprintln!("policy request must be an object");
        return ExitCode::from(2);
    };
    let allowed = ["schema_version", "policy_class", "public_view", "expansion_decision_made"];
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        eprintln!("policy request contains undeclared field {key}");
        return ExitCode::from(2);
    }
    if request.get("schema_version").and_then(Value::as_u64) != Some(1) {
        eprintln!("policy request schema_version must equal 1");
        return ExitCode::from(2);
    }
    let Some(policy) = request.get("policy_class").and_then(Value::as_str) else {
        eprintln!("policy request needs policy_class");
        return ExitCode::from(2);
    };
    if !allowed_policy(policy) {
        eprintln!("policy request contains unregistered policy_class {policy}");
        return ExitCode::from(2);
    }
    let Some(view) = request.get("public_view") else {
        eprintln!("policy request needs public_view");
        return ExitCode::from(2);
    };
    let Some(expansion_decision_made) = request.get("expansion_decision_made").and_then(Value::as_bool) else {
        eprintln!("policy request needs boolean expansion_decision_made");
        return ExitCode::from(2);
    };
    let action = choose_action(policy, view, expansion_decision_made);
    println!("{}", serde_json::to_string(&action_to_value(&action)).expect("action serializes"));
    ExitCode::SUCCESS
}

fn close(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(a), Some(b)) => a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-9,
        (None, None) => true,
        _ => false,
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 2 && args[1] == "--policy-child" {
        return run_policy_child();
    }
    if args.len() == 5 && args[1] == "--replay" {
        let report_raw = match fs::read(&args[2]) {
            Ok(bytes) => bytes,
            Err(e) => { eprintln!("cannot read {}: {e}", args[2]); return ExitCode::from(2); }
        };
        let public_raw = match fs::read(&args[3]) {
            Ok(bytes) => bytes,
            Err(e) => { eprintln!("cannot read {}: {e}", args[3]); return ExitCode::from(2); }
        };
        let oracle_raw = match fs::read(&args[4]) {
            Ok(bytes) => bytes,
            Err(e) => { eprintln!("cannot read {}: {e}", args[4]); return ExitCode::from(2); }
        };
        let replay = audit_replay(&report_raw, &public_raw, &oracle_raw);
        let status = replay.get("status").and_then(Value::as_str).unwrap_or("FAIL");
        let code = match status { "PASS" => 0, "UNRESOLVED" => 3, _ => 2 };
        println!("{}", serde_json::to_string_pretty(&replay).expect("replay report serializes"));
        return ExitCode::from(code);
    }
    if args.len() != 3 {
        eprintln!("Usage:\n  run-search-space-loop <public-policy-catalog.json> <sealed-outcome-oracle.json>\n  run-search-space-loop --replay <loop-report.json> <public-policy-catalog.json> <sealed-outcome-oracle.json>");
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
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("cannot resolve policy host executable: {error}");
            return ExitCode::from(2);
        }
    };
    let report = run_all_subprocess(&public_raw, &oracle_raw, &executable);
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
        assert_eq!(report["episodes_checked"], 24);
        assert_eq!(report["environment_count"], 4);
        for e in report["episodes"].as_array().unwrap() {
            assert_eq!(e["cost_units_charged"], 5.0, "{}", e["campaign_id"]);
            assert!(e["duration_seconds_charged"].as_f64().unwrap() <= 25.0, "{}", e["campaign_id"]);
        }
    }

    #[test]
    fn gp_ei_fixed_space_uses_revealed_utilities_and_never_expands() {
        let (public, oracle) = values();
        for environment in ["helpful", "neutral", "harmful", "misleading_prior"] {
            let report = run_env(&public, &oracle, environment, "gp_ei_fixed_space");
            assert_eq!(report["status"], "PASS");
            assert_eq!(report["expansions"], 0);
            assert_eq!(report["evaluations"], 5);
            assert_eq!(report["cost_units_charged"], 5.0);
            assert_eq!(report["duration_seconds_charged"], 25.0);
            assert!(report["events"].as_array().unwrap().iter().all(|event| event["action_type"] == "evaluate_candidate"));
        }
    }

    #[test]
    fn gp_ei_candidate_ranking_is_deterministic_and_only_returns_unmeasured_candidates() {
        let (public, _oracle) = values();
        let initial: BTreeSet<String> = public["candidate_families"].as_array().unwrap().iter()
            .filter(|c| c["initially_visible"] == true)
            .filter_map(|c| c["id"].as_str().map(str::to_owned)).collect();
        let observations = vec![Observation { candidate_id: "glass-a".to_owned(), feasible: true, utility: 4.0 }];
        let evaluated = HashSet::from(["glass-a".to_owned()]);
        let view = build_public_view(&public, "gp_ei_fixed_space", &initial, &evaluated, &observations, &[], false, 4.0, 20.0);
        let first = gp_ei_next_candidate(&view).unwrap();
        let second = gp_ei_next_candidate(&view).unwrap();
        assert_eq!(first, second);
        assert_ne!(first, "glass-a");
        assert!(initial.contains(&first));
        assert!(gp_ei_score_candidate(&view, &first).unwrap().is_finite());
    }

    #[test]
    fn gp_ei_cholesky_solver_rejects_non_positive_definite_matrix() {
        let singular = vec![vec![1.0, 2.0], vec![2.0, 1.0]];
        assert!(cholesky_lower(&singular).is_none());
        let valid = vec![vec![1.1, 0.4], vec![0.4, 1.1]];
        let lower = cholesky_lower(&valid).unwrap();
        let solution = solve_from_cholesky(&lower, &[1.0, 0.0]).unwrap();
        assert!(solution.iter().all(|x| x.is_finite()));
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
        let static_report = run_env(&public, &oracle, "harmful", "static_prior_expansion");
        let adaptive = run_env(&public, &oracle, "harmful", "prior_guided_expansion");
        assert_eq!(static_report["events"][0]["action_type"], "expand_space");
        assert_eq!(adaptive["events"][0]["action_type"], "evaluate_candidate");
        assert_eq!(static_report["expansions"], 1);
        assert_eq!(adaptive["expansions"], 0);
        assert_eq!(static_report["best_verified_feasible_utility"], 6.0);
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
        assert_eq!(report["best_verified_feasible_utility"], 4.0);
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

    #[test]
    fn independent_replay_recomputes_the_full_live_loop_receipt() {
        let report = run_all(PUBLIC, ORACLE);
        let report_raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&report_raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "PASS");
        assert_eq!(replay["episodes_checked"], 24);
        assert!(replay["events_checked"].as_u64().unwrap() >= 90);
        assert_eq!(replay["cost_units_recomputed"], 120.0);
    }

    #[test]
    fn independent_replay_rejects_a_missing_policy_environment_pair() {
        let mut report = run_all(PUBLIC, ORACLE);
        report["episodes"].as_array_mut().unwrap().retain(|e| {
            !(e["environment_id"] == "harmful" && e["policy_class"] == "gp_ei_fixed_space")
        });
        report["episodes_checked"] = json!(report["episodes"].as_array().unwrap().len());
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "POLICY_ENVIRONMENT_PAIR_MISSING"));
    }

    #[test]
    fn simple_regret_measures_missed_feasible_utility_without_entering_policy_state() {
        let (public, oracle) = values();
        let fixed = run_env(&public, &oracle, "helpful", "fixed_space");
        let expanded = run_env(&public, &oracle, "helpful", "prior_guided_expansion");
        assert_eq!(fixed["oracle_best_feasible_utility"], 10.0);
        assert_eq!(fixed["best_verified_feasible_utility"], 4.0);
        assert_eq!(fixed["terminal_simple_regret"], 6.0);
        assert_eq!(expanded["terminal_simple_regret"], 0.0);
        let initial: BTreeSet<String> = public["candidate_families"].as_array().unwrap().iter()
            .filter(|c| c["initially_visible"] == true)
            .filter_map(|c| c["id"].as_str().map(str::to_owned)).collect();
        let view = build_public_view(&public, "fixed_space", &initial, &HashSet::new(), &[], &[], false, 5.0, 25.0);
        let serialized = serde_json::to_string(&view).unwrap();
        assert!(!serialized.contains("oracle_best_feasible_utility"));
        assert!(!serialized.contains("terminal_simple_regret"));
    }

    #[test]
    fn independent_replay_rejects_forged_simple_regret() {
        let mut report = run_all(PUBLIC, ORACLE);
        report["episodes"].as_array_mut().unwrap()[0]["terminal_simple_regret"] = json!(0.0);
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "SIMPLE_REGRET_MISMATCH"));
    }

    #[test]
    fn independent_replay_rejects_a_legal_but_policy_inconsistent_action() {
        let mut report = run_all(PUBLIC, ORACLE);
        let first_event = &mut report["episodes"].as_array_mut().unwrap()[0]["events"].as_array_mut().unwrap()[0];
        assert_eq!(first_event["action_type"], "evaluate_candidate");
        first_event["candidate_family_id"] = json!("glass-b");
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "POLICY_ACTION_MISMATCH"));
    }

    #[test]
    fn independent_replay_rejects_forged_observation_values() {
        let (public, oracle) = values();
        let mut report = run_all(PUBLIC, ORACLE);
        let event = report["episodes"].as_array_mut().unwrap()[0]["events"].as_array_mut().unwrap()
            .iter_mut().find(|e| e["action_type"] == "evaluate_candidate").unwrap();
        event["observed_outcome"]["utility"] = json!(9999.0);
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, &serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "OBSERVATION_ORACLE_MISMATCH"));
    }

    #[test]
    fn independent_replay_rejects_underreported_action_cost() {
        let report = run_all(PUBLIC, ORACLE);
        let mut report = report;
        report["episodes"].as_array_mut().unwrap()[0]["events"][0]["cost_units_charged"] = json!(0.01);
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "ACTION_CHARGE_MISMATCH"));
    }

    #[test]
    fn independent_replay_rejects_stale_view_digest() {
        let mut report = run_all(PUBLIC, ORACLE);
        report["episodes"].as_array_mut().unwrap()[0]["events"][0]["decision_view_sha256"] = json!("0".repeat(64));
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "DECISION_VIEW_DIGEST_MISMATCH"));
    }

    #[test]
    fn independent_replay_refuses_relabelled_oracle_bytes() {
        let report = run_all(PUBLIC, ORACLE);
        let raw = serde_json::to_vec(&report).unwrap();
        let mut changed: Value = serde_json::from_slice(ORACLE).unwrap();
        changed["environments"][0]["outcomes"][0]["utility"] = json!(1234.0);
        let changed_raw = serde_json::to_vec(&changed).unwrap();
        let replay = audit_replay(&raw, PUBLIC, &changed_raw);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|f| f["code"] == "ORACLE_DIGEST_MISMATCH"));
    }

}
