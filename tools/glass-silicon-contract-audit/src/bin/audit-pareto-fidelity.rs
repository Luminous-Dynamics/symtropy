use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-pareto-fidelity-synthetic-v0-1";
const POLICY_CLASSES: [&str; 4] = [
    "confirmatory_priority",
    "screen_then_confirm",
    "screen_only",
    "random_confirmatory",
];
const CLAIM_CEILING: &str = "Synthetic constrained Pareto and fidelity-accounting mechanics only; no glass/silicon performance, policy superiority, or industrial benefit is validated.";

#[derive(Clone, Debug)]
struct Tariff {
    cost: f64,
    seconds: f64,
}

#[derive(Clone, Debug)]
enum Action {
    Evaluate { candidate_id: String, fidelity: String },
    Stop(String),
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn close(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= 1e-9
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
    let Some(object) = value.as_object() else {
        fail(report, "FAIL", "OBJECT_REQUIRED", &format!("{path} must be a JSON object"));
        return;
    };
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            fail(report, "FAIL", "UNKNOWN_PROPERTY", &format!("{path}.{key} is not permitted"));
        }
    }
}

fn required_number(value: &Value, field: &str, path: &str, positive: bool, report: &mut Value) -> Option<f64> {
    match value.get(field).and_then(Value::as_f64) {
        Some(number) if number.is_finite() && (if positive { number > 0.0 } else { number >= 0.0 }) => Some(number),
        _ => {
            fail(report, "FAIL", "NUMBER_INVALID", &format!("{path}.{field} has invalid numeric value"));
            None
        }
    }
}

fn required_str<'a>(value: &'a Value, field: &str, path: &str, report: &mut Value) -> Option<&'a str> {
    match value.get(field).and_then(Value::as_str).map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => Some(text),
        None => {
            fail(report, "FAIL", "STRING_REQUIRED", &format!("{path}.{field} must be a non-empty string"));
            None
        }
    }
}

fn read_tariffs(oracle: &Value, report: &mut Value) -> HashMap<String, Tariff> {
    let mut tariffs = HashMap::new();
    let Some(items) = oracle.get("action_cost_catalog").and_then(Value::as_array) else {
        fail(report, "FAIL", "TARIFF_CATALOG_REQUIRED", "oracle.action_cost_catalog must be an array");
        return tariffs;
    };
    for (index, item) in items.iter().enumerate() {
        let path = format!("oracle.action_cost_catalog[{index}]");
        only_keys(item, &["fidelity", "cost_units", "duration_seconds"], &path, report);
        let Some(fidelity) = required_str(item, "fidelity", &path, report) else { continue };
        let Some(cost) = required_number(item, "cost_units", &path, true, report) else { continue };
        let Some(seconds) = required_number(item, "duration_seconds", &path, true, report) else { continue };
        if !["screen", "confirmatory"].contains(&fidelity) {
            fail(report, "FAIL", "FIDELITY_UNREGISTERED", &format!("unregistered fidelity {fidelity}"));
        }
        if tariffs.insert(fidelity.to_owned(), Tariff { cost, seconds }).is_some() {
            fail(report, "FAIL", "FIDELITY_TARIFF_DUPLICATE", &format!("duplicate tariff for {fidelity}"));
        }
    }
    for required in ["screen", "confirmatory"] {
        if !tariffs.contains_key(required) {
            fail(report, "FAIL", "FIDELITY_TARIFF_MISSING", &format!("missing evaluator-owned tariff {required}"));
        }
    }
    tariffs
}

fn validate_contract(public: &Value, oracle: &Value, report: &mut Value) {
    only_keys(
        public,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "budgets", "objectives", "hard_constraints", "reference_point", "random_seed", "candidates"],
        "public",
        report,
    );
    only_keys(
        oracle,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "action_cost_catalog", "environments"],
        "oracle",
        report,
    );

    if public.get("schema_version").and_then(Value::as_u64) != Some(1)
        || oracle.get("schema_version").and_then(Value::as_u64) != Some(1)
        || public.get("evidence_class").and_then(Value::as_str) != Some("synthetic")
        || oracle.get("evidence_class").and_then(Value::as_str) != Some("synthetic_evaluator_only")
    {
        fail(report, "FAIL", "CONTRACT_VERSION_INVALID", "schema version or evidence class is invalid");
    }
    if public.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
    {
        fail(report, "FAIL", "BENCHMARK_ID_MISMATCH", "public task and sealed oracle must use the registered benchmark ID");
    }
    if public.get("claim_ceiling").and_then(Value::as_str).is_none()
        || oracle.get("claim_ceiling").and_then(Value::as_str).is_none()
    {
        fail(report, "FAIL", "CLAIM_CEILING_MISSING", "both artifacts must declare a claim ceiling");
    }

    let budgets = public.get("budgets").unwrap_or(&Value::Null);
    only_keys(budgets, &["max_cost_units", "max_duration_seconds"], "public.budgets", report);
    let max_cost = required_number(budgets, "max_cost_units", "public.budgets", true, report);
    let max_seconds = required_number(budgets, "max_duration_seconds", "public.budgets", true, report);
    if max_cost.is_some_and(|value| !close(value, 5.5))
        || max_seconds.is_some_and(|value| !close(value, 9.0))
    {
        fail(report, "FAIL", "BUDGET_CONTRACT_CHANGED", "v0.1 uses the frozen 5.5 cost-unit and 9-second caps");
    }

    let objectives = public.get("objectives").and_then(Value::as_array);
    if let Some(objectives) = objectives {
        if objectives.len() != 2 {
            fail(report, "FAIL", "OBJECTIVE_COUNT_INVALID", "the v0.1 score uses exactly two preregistered objectives");
        }
        let expected = [("quality", "maximize", "normalized_quality"), ("energy", "minimize", "synthetic_energy_units")];
        for (index, item) in objectives.iter().enumerate() {
            only_keys(item, &["id", "direction", "unit"], &format!("public.objectives[{index}]"), report);
            let actual = (
                item.get("id").and_then(Value::as_str).unwrap_or(""),
                item.get("direction").and_then(Value::as_str).unwrap_or(""),
                item.get("unit").and_then(Value::as_str).unwrap_or(""),
            );
            if index >= expected.len() || actual != expected[index] {
                fail(report, "FAIL", "OBJECTIVE_CONTRACT_CHANGED", "objective identity, direction, ordering, and unit are frozen in v0.1");
            }
        }
    } else {
        fail(report, "FAIL", "OBJECTIVES_REQUIRED", "public.objectives must be an array");
    }

    let constraints = public.get("hard_constraints").and_then(Value::as_array);
    if !constraints.is_some_and(|items| items.len() == 1
        && items[0].get("id").and_then(Value::as_str) == Some("qualification")
        && items[0].get("qualifying_fidelity").and_then(Value::as_str) == Some("confirmatory")
        && items[0].get("rule").and_then(Value::as_str) == Some("must_pass"))
    {
        fail(report, "FAIL", "HARD_CONSTRAINT_CONTRACT_CHANGED", "qualification must be a hard gate established only by confirmatory fidelity");
    }

    let reference = public.get("reference_point").unwrap_or(&Value::Null);
    only_keys(reference, &["quality", "energy"], "public.reference_point", report);
    if !reference.get("quality").and_then(Value::as_f64).is_some_and(|value| close(value, 0.0))
        || !reference.get("energy").and_then(Value::as_f64).is_some_and(|value| close(value, 10.0))
    {
        fail(report, "FAIL", "PARETO_REFERENCE_POINT_CHANGED", "the v0.1 Pareto reference point is frozen at quality=0, energy=10");
    }

    if public.get("random_seed").and_then(Value::as_u64).is_none() {
        fail(report, "FAIL", "RANDOM_SEED_INVALID", "public.random_seed must be a non-negative integer");
    }
    let Some(candidates) = public.get("candidates").and_then(Value::as_array) else {
        fail(report, "FAIL", "CANDIDATES_REQUIRED", "public.candidates must be an array");
        return;
    };
    if candidates.len() < 3 {
        fail(report, "FAIL", "CANDIDATE_CATALOG_TOO_SMALL", "at least three candidates are required");
    }
    let mut ids = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = format!("public.candidates[{index}]");
        only_keys(candidate, &["id", "screen_priority", "features"], &path, report);
        let Some(id) = required_str(candidate, "id", &path, report) else { continue };
        if ids.insert(id.to_owned()) == false {
            fail(report, "FAIL", "CANDIDATE_ID_DUPLICATE", &format!("duplicate candidate ID {id}"));
        }
        if let Some(priority) = required_number(candidate, "screen_priority", &path, false, report) {
            if !(0.0..=1.0).contains(&priority) {
                fail(report, "FAIL", "SCREEN_PRIORITY_OUT_OF_RANGE", &format!("{path}.screen_priority must be in [0, 1]"));
            }
        }
        if !candidate.get("features").and_then(Value::as_array).is_some_and(|features| features.len() == 2 && features.iter().all(|f| f.as_f64().is_some_and(f64::is_finite))) {
            fail(report, "FAIL", "CANDIDATE_FEATURES_INVALID", &format!("{path}.features must contain exactly two finite numbers"));
        }
    }

    let _ = read_tariffs(oracle, report);
    let Some(environments) = oracle.get("environments").and_then(Value::as_array) else {
        fail(report, "FAIL", "ENVIRONMENTS_REQUIRED", "oracle.environments must be an array");
        return;
    };
    let mut environment_ids = HashSet::new();
    for (env_index, environment) in environments.iter().enumerate() {
        let env_path = format!("oracle.environments[{env_index}]");
        only_keys(environment, &["environment_id", "outcomes"], &env_path, report);
        let Some(env_id) = required_str(environment, "environment_id", &env_path, report) else { continue };
        if !environment_ids.insert(env_id.to_owned()) {
            fail(report, "FAIL", "ENVIRONMENT_DUPLICATE", &format!("duplicate environment {env_id}"));
        }
        if !["informative", "neutral", "misleading_low_fidelity"].contains(&env_id) {
            fail(report, "FAIL", "ENVIRONMENT_UNREGISTERED", &format!("unregistered environment {env_id}"));
        }
        let Some(outcomes) = environment.get("outcomes").and_then(Value::as_array) else {
            fail(report, "FAIL", "OUTCOMES_REQUIRED", &format!("{env_path}.outcomes must be an array"));
            continue;
        };
        let mut outcome_keys = HashSet::new();
        for (out_index, outcome) in outcomes.iter().enumerate() {
            let out_path = format!("{env_path}.outcomes[{out_index}]");
            only_keys(outcome, &["candidate_id", "fidelity", "objectives", "constraints"], &out_path, report);
            let candidate_id = required_str(outcome, "candidate_id", &out_path, report).unwrap_or("");
            let fidelity = required_str(outcome, "fidelity", &out_path, report).unwrap_or("");
            if !ids.contains(candidate_id) {
                fail(report, "FAIL", "OUTCOME_CANDIDATE_UNKNOWN", &format!("{out_path} references unknown candidate {candidate_id}"));
            }
            if !["screen", "confirmatory"].contains(&fidelity) {
                fail(report, "FAIL", "OUTCOME_FIDELITY_UNKNOWN", &format!("{out_path} has unknown fidelity {fidelity}"));
            }
            if !outcome_keys.insert((candidate_id.to_owned(), fidelity.to_owned())) {
                fail(report, "FAIL", "OUTCOME_DUPLICATE", &format!("duplicate outcome for {candidate_id}/{fidelity}"));
            }
            let objectives = outcome.get("objectives").unwrap_or(&Value::Null);
            only_keys(objectives, &["quality", "energy"], &format!("{out_path}.objectives"), report);
            let quality = objectives.get("quality").and_then(Value::as_f64);
            let energy = objectives.get("energy").and_then(Value::as_f64);
            if !quality.is_some_and(|value| value.is_finite() && (0.0..=1.0).contains(&value)) {
                fail(report, "FAIL", "OUTCOME_QUALITY_OUT_OF_RANGE", &format!("{out_path}.objectives.quality must be finite and in [0, 1]"));
            }
            if !energy.is_some_and(|value| value.is_finite() && value >= 0.0) {
                fail(report, "FAIL", "OUTCOME_ENERGY_INVALID", &format!("{out_path}.objectives.energy must be finite and non-negative"));
            }
            if fidelity == "screen" {
                if !outcome.get("constraints").is_some_and(Value::is_null) {
                    fail(report, "FAIL", "LOW_FIDELITY_CANNOT_QUALIFY", &format!("{out_path} low-fidelity result must not assert a hard-constraint result"));
                }
            } else {
                let constraints = outcome.get("constraints").unwrap_or(&Value::Null);
                only_keys(constraints, &["qualification"], &format!("{out_path}.constraints"), report);
                if constraints.get("qualification").and_then(Value::as_bool).is_none() {
                    fail(report, "FAIL", "CONFIRMATORY_CONSTRAINT_REQUIRED", &format!("{out_path} confirmatory result must report qualification"));
                }
            }
        }
        if outcome_keys.len() != candidates.len() * 2 {
            fail(report, "UNRESOLVED", "OUTCOME_COVERAGE_INCOMPLETE", &format!("{env_id} has {} candidate/fidelity outcomes; expected {}", outcome_keys.len(), candidates.len() * 2));
        }
    }
    if environment_ids.len() != 3 {
        fail(report, "FAIL", "ENVIRONMENT_COVERAGE_INVALID", "v0.1 requires informative, neutral, and misleading_low_fidelity regimes");
    }
}

fn tariffs_from_oracle(oracle: &Value) -> HashMap<String, Tariff> {
    oracle.get("action_cost_catalog").and_then(Value::as_array).into_iter().flatten().filter_map(|item| {
        Some((
            item.get("fidelity")?.as_str()?.to_owned(),
            Tariff {
                cost: item.get("cost_units")?.as_f64()?,
                seconds: item.get("duration_seconds")?.as_f64()?,
            },
        ))
    }).collect()
}

fn outcome_for<'a>(oracle: &'a Value, environment_id: &str, candidate_id: &str, fidelity: &str) -> Option<&'a Value> {
    oracle.get("environments")?.as_array()?.iter()
        .find(|environment| environment.get("environment_id").and_then(Value::as_str) == Some(environment_id))?
        .get("outcomes")?.as_array()?.iter()
        .find(|outcome| {
            outcome.get("candidate_id").and_then(Value::as_str) == Some(candidate_id)
                && outcome.get("fidelity").and_then(Value::as_str) == Some(fidelity)
        })
}

fn candidate_by_id<'a>(public: &'a Value, candidate_id: &str) -> Option<&'a Value> {
    public.get("candidates")?.as_array()?.iter()
        .find(|candidate| candidate.get("id").and_then(Value::as_str) == Some(candidate_id))
}

fn candidate_ids(public: &Value) -> Vec<String> {
    public.get("candidates").and_then(Value::as_array).into_iter().flatten()
        .filter_map(|candidate| candidate.get("id").and_then(Value::as_str).map(str::to_owned))
        .collect()
}

fn has_observation(observations: &[Value], candidate_id: &str, fidelity: &str) -> bool {
    observations.iter().any(|observation| {
        observation.get("candidate_id").and_then(Value::as_str) == Some(candidate_id)
            && observation.get("fidelity").and_then(Value::as_str) == Some(fidelity)
    })
}

fn screen_score(observation: &Value) -> f64 {
    let quality = observation.get("objectives").and_then(|v| v.get("quality")).and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY);
    let energy = observation.get("objectives").and_then(|v| v.get("energy")).and_then(Value::as_f64).unwrap_or(f64::INFINITY);
    quality - 0.02 * energy
}

fn deterministic_random_rank(seed: u64, candidate_id: &str) -> String {
    sha256(format!("pareto-fidelity-random-v1|{seed}|{candidate_id}").as_bytes())
}

fn choose_action(policy: &str, view: &Value) -> Action {
    let observations = view.get("observations").and_then(Value::as_array).cloned().unwrap_or_default();
    let candidates = view.get("visible_candidates").and_then(Value::as_array).cloned().unwrap_or_default();
    let available = view.get("available_fidelities").and_then(Value::as_array).cloned().unwrap_or_default();
    let has_fidelity = |fidelity: &str| available.iter().any(|value| value.as_str() == Some(fidelity));

    let ids = candidate_ids_from_view(&candidates);
    match policy {
        "confirmatory_priority" => {
            if !has_fidelity("confirmatory") {
                return Action::Stop("NO_AFFORDABLE_CONFIRMATORY_MEASUREMENT".to_owned());
            }
            let best = candidates.iter()
                .filter(|candidate| !has_observation(&observations, candidate.get("id").and_then(Value::as_str).unwrap_or(""), "confirmatory"))
                .max_by(|left, right| {
                    left.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(&right.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
                });
            best.and_then(|candidate| candidate.get("id").and_then(Value::as_str))
                .map(|candidate_id| Action::Evaluate { candidate_id: candidate_id.to_owned(), fidelity: "confirmatory".to_owned() })
                .unwrap_or_else(|| Action::Stop("NO_UNCONFIRMED_CANDIDATES".to_owned()))
        }
        "screen_then_confirm" => {
            let screen_count = observations.iter().filter(|item| item.get("fidelity").and_then(Value::as_str) == Some("screen")).count();
            if screen_count < 3 && has_fidelity("screen") {
                let candidate = candidates.iter()
                    .filter(|candidate| !has_observation(&observations, candidate.get("id").and_then(Value::as_str).unwrap_or(""), "screen"))
                    .max_by(|left, right| {
                        left.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                            .total_cmp(&right.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
                    });
                if let Some(candidate_id) = candidate.and_then(|candidate| candidate.get("id").and_then(Value::as_str)) {
                    return Action::Evaluate { candidate_id: candidate_id.to_owned(), fidelity: "screen".to_owned() };
                }
            }
            if !has_fidelity("confirmatory") {
                return Action::Stop("NO_AFFORDABLE_CONFIRMATORY_MEASUREMENT".to_owned());
            }
            let best_screened = observations.iter()
                .filter(|item| item.get("fidelity").and_then(Value::as_str) == Some("screen"))
                .filter(|item| !has_observation(&observations, item.get("candidate_id").and_then(Value::as_str).unwrap_or(""), "confirmatory"))
                .max_by(|left, right| {
                    screen_score(left).total_cmp(&screen_score(right))
                        .then_with(|| right.get("candidate_id").and_then(Value::as_str).unwrap_or("").cmp(left.get("candidate_id").and_then(Value::as_str).unwrap_or("")))
                });
            best_screened.and_then(|observation| observation.get("candidate_id").and_then(Value::as_str))
                .map(|candidate_id| Action::Evaluate { candidate_id: candidate_id.to_owned(), fidelity: "confirmatory".to_owned() })
                .unwrap_or_else(|| Action::Stop("NO_SCREENED_CANDIDATE_REMAINS".to_owned()))
        }
        "screen_only" => {
            if !has_fidelity("screen") {
                return Action::Stop("NO_AFFORDABLE_SCREEN".to_owned());
            }
            let best = candidates.iter()
                .filter(|candidate| !has_observation(&observations, candidate.get("id").and_then(Value::as_str).unwrap_or(""), "screen"))
                .max_by(|left, right| {
                    left.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY)
                        .total_cmp(&right.get("screen_priority").and_then(Value::as_f64).unwrap_or(f64::NEG_INFINITY))
                });
            best.and_then(|candidate| candidate.get("id").and_then(Value::as_str))
                .map(|candidate_id| Action::Evaluate { candidate_id: candidate_id.to_owned(), fidelity: "screen".to_owned() })
                .unwrap_or_else(|| Action::Stop("ALL_CANDIDATES_SCREENED".to_owned()))
        }
        "random_confirmatory" => {
            if !has_fidelity("confirmatory") {
                return Action::Stop("NO_AFFORDABLE_CONFIRMATORY_MEASUREMENT".to_owned());
            }
            let seed = view.get("random_seed").and_then(Value::as_u64).unwrap_or(0);
            ids.iter().filter(|candidate_id| !has_observation(&observations, candidate_id, "confirmatory"))
                .min_by_key(|candidate_id| deterministic_random_rank(seed, candidate_id))
                .map(|candidate_id| Action::Evaluate { candidate_id: candidate_id.to_owned(), fidelity: "confirmatory".to_owned() })
                .unwrap_or_else(|| Action::Stop("NO_UNCONFIRMED_CANDIDATES".to_owned()))
        }
        _ => Action::Stop("POLICY_UNREGISTERED".to_owned()),
    }
}

fn candidate_ids_from_view(candidates: &[Value]) -> Vec<String> {
    candidates.iter().filter_map(|candidate| candidate.get("id").and_then(Value::as_str).map(str::to_owned)).collect()
}

fn build_public_view(
    public: &Value,
    tariffs: &HashMap<String, Tariff>,
    observations: &[Value],
    cost_used: f64,
    seconds_used: f64,
) -> Value {
    let max_cost = public["budgets"]["max_cost_units"].as_f64().unwrap_or(0.0);
    let max_seconds = public["budgets"]["max_duration_seconds"].as_f64().unwrap_or(0.0);
    let available_fidelities = ["screen", "confirmatory"].into_iter().filter(|fidelity| {
        tariffs.get(*fidelity).is_some_and(|tariff| {
            cost_used + tariff.cost <= max_cost + 1e-9
                && seconds_used + tariff.seconds <= max_seconds + 1e-9
        })
    }).collect::<Vec<_>>();
    let visible_candidates = public["candidates"].as_array().cloned().unwrap_or_default().into_iter().map(|candidate| {
        let id = candidate.get("id").and_then(Value::as_str).unwrap_or("");
        json!({
            "id": id,
            "screen_priority": candidate.get("screen_priority").cloned().unwrap_or(Value::Null),
            "features": candidate.get("features").cloned().unwrap_or(Value::Null),
            "screened": has_observation(observations, id, "screen"),
            "confirmatory_measured": has_observation(observations, id, "confirmatory")
        })
    }).collect::<Vec<_>>();
    json!({
        "policy_contract_version": 1,
        "random_seed": public.get("random_seed").cloned().unwrap_or(json!(0)),
        "objectives": public.get("objectives").cloned().unwrap_or(Value::Null),
        "reference_point": public.get("reference_point").cloned().unwrap_or(Value::Null),
        "hard_constraints": public.get("hard_constraints").cloned().unwrap_or(Value::Null),
        "visible_candidates": visible_candidates,
        "observations": observations,
        "action_costs": {
            "screen": tariffs.get("screen").map(|t| json!({"cost_units":t.cost,"duration_seconds":t.seconds})).unwrap_or(Value::Null),
            "confirmatory": tariffs.get("confirmatory").map(|t| json!({"cost_units":t.cost,"duration_seconds":t.seconds})).unwrap_or(Value::Null)
        },
        "available_fidelities": available_fidelities,
        "remaining_cost_units": (max_cost - cost_used).max(0.0),
        "remaining_duration_seconds": (max_seconds - seconds_used).max(0.0)
    })
}

fn objective_pair(objectives: &Value) -> Option<(f64, f64)> {
    Some((objectives.get("quality")?.as_f64()?, objectives.get("energy")?.as_f64()?))
}

fn transformed(objectives: &Value) -> Option<(f64, f64)> {
    let (quality, energy) = objective_pair(objectives)?;
    // The public objectives are quality maximize and energy minimize. Hypervolume
    // uses the equivalent maximize-space coordinate (quality, -energy).
    Some((quality, -energy))
}

fn dominates(left: (f64, f64), right: (f64, f64)) -> bool {
    left.0 >= right.0 && left.1 >= right.1 && (left.0 > right.0 || left.1 > right.1)
}

fn pareto_ids(candidates: &[(String, Value)]) -> Vec<String> {
    let values = candidates.iter().filter_map(|(id, objectives)| transformed(objectives).map(|point| (id.clone(), point))).collect::<Vec<_>>();
    let mut out = values.iter().filter(|(id, point)| {
        !values.iter().any(|(other_id, other_point)| id != other_id && dominates(*other_point, *point))
    }).map(|(id, _)| id.clone()).collect::<Vec<_>>();
    out.sort();
    out
}

fn hypervolume(objectives: &[Value], reference: &Value) -> f64 {
    let Some(reference_quality) = reference.get("quality").and_then(Value::as_f64) else { return 0.0 };
    let Some(reference_energy) = reference.get("energy").and_then(Value::as_f64) else { return 0.0 };
    let ref_point = (reference_quality, -reference_energy);
    let mut points = objectives.iter().filter_map(transformed).filter(|point| {
        point.0 > ref_point.0 && point.1 > ref_point.1
    }).collect::<Vec<_>>();
    if points.is_empty() {
        return 0.0;
    }
    points.sort_by(|a,b| a.0.total_cmp(&b.0).then_with(|| b.1.total_cmp(&a.1)));
    let mut area = 0.0;
    let mut previous_x = ref_point.0;
    for index in 0..points.len() {
        let x = points[index].0;
        let max_y = points[index..].iter().map(|point| point.1).fold(f64::NEG_INFINITY, f64::max);
        area += (x - previous_x).max(0.0) * (max_y - ref_point.1).max(0.0);
        previous_x = x;
    }
    area
}

fn feasible_confirmatory_outcomes(public: &Value, oracle: &Value, environment_id: &str) -> Vec<(String, Value)> {
    let mut feasible = Vec::new();
    for candidate_id in candidate_ids(public) {
        if let Some(outcome) = outcome_for(oracle, environment_id, &candidate_id, "confirmatory") {
            if outcome.get("constraints").and_then(|v| v.get("qualification")).and_then(Value::as_bool) == Some(true) {
                if let Some(objectives) = outcome.get("objectives") {
                    feasible.push((candidate_id, objectives.clone()));
                }
            }
        }
    }
    feasible
}

fn run_episode(public: &Value, oracle: &Value, environment_id: &str, policy: &str, tariffs: &HashMap<String, Tariff>) -> Value {
    let mut report = json!({
        "status": "PASS",
        "environment_id": environment_id,
        "policy_class": policy,
        "events": [],
        "findings": [],
        "evaluations": 0,
        "screen_evaluations": 0,
        "confirmatory_evaluations": 0,
        "accepted_feasible_candidates": [],
        "confirmed_infeasible_candidates": [],
        "observed_pareto_candidate_ids": [],
        "true_pareto_candidate_ids": [],
        "observed_hypervolume": 0.0,
        "oracle_hypervolume": 0.0,
        "hypervolume_regret": 0.0,
        "pareto_recall": null,
        "cost_units_charged": 0.0,
        "duration_seconds_charged": 0.0,
        "claim_ceiling": CLAIM_CEILING
    });
    let max_cost = public["budgets"]["max_cost_units"].as_f64().unwrap_or(0.0);
    let max_seconds = public["budgets"]["max_duration_seconds"].as_f64().unwrap_or(0.0);
    let mut observations: Vec<Value> = Vec::new();
    let mut seen = HashSet::<(String, String)>::new();
    let mut cost_used = 0.0;
    let mut seconds_used = 0.0;
    let mut terminal = "POLICY_STOPPED".to_owned();

    for seq in 1..=16 {
        let view = build_public_view(public, tariffs, &observations, cost_used, seconds_used);
        let view_digest = sha256(&serde_json::to_vec(&view).unwrap_or_default());
        let action = choose_action(policy, &view);
        match action {
            Action::Stop(reason) => {
                terminal = reason;
                break;
            }
            Action::Evaluate { candidate_id, fidelity } => {
                if candidate_by_id(public, &candidate_id).is_none() {
                    fail(&mut report, "FAIL", "CANDIDATE_NOT_REGISTERED", &format!("policy requested unknown candidate {candidate_id}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                if !["screen", "confirmatory"].contains(&fidelity.as_str()) {
                    fail(&mut report, "FAIL", "FIDELITY_NOT_REGISTERED", &format!("policy requested unknown fidelity {fidelity}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                if !seen.insert((candidate_id.clone(), fidelity.clone())) {
                    fail(&mut report, "FAIL", "DUPLICATE_FIDELITY_EVALUATION", &format!("duplicate {fidelity} evaluation for {candidate_id}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }
                let Some(tariff) = tariffs.get(&fidelity) else {
                    fail(&mut report, "FAIL", "TARIFF_MISSING", &format!("no evaluator-owned tariff for {fidelity}"));
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                };
                if cost_used + tariff.cost > max_cost + 1e-9 || seconds_used + tariff.seconds > max_seconds + 1e-9 {
                    fail(&mut report, "FAIL", "ACTION_OVERRUN", "the requested fidelity would exceed the shared cost or duration cap");
                    terminal = "REJECTED_ACTION".to_owned();
                    break;
                }

                // Charge from the evaluator-owned tariff before revealing any outcome.
                cost_used += tariff.cost;
                seconds_used += tariff.seconds;
                let mut event = json!({
                    "event_seq": seq,
                    "candidate_id": candidate_id,
                    "fidelity": fidelity,
                    "decision_view_sha256": view_digest,
                    "cost_units_charged": tariff.cost,
                    "duration_seconds_charged": tariff.seconds,
                    "observed_result": null
                });
                let Some(outcome) = outcome_for(oracle, environment_id, &candidate_id, &fidelity) else {
                    report["events"].as_array_mut().unwrap().push(event);
                    fail(&mut report, "UNRESOLVED", "SEALED_OUTCOME_MISSING", &format!("missing outcome for {environment_id}/{candidate_id}/{fidelity}"));
                    terminal = "UNRESOLVED_OUTCOME".to_owned();
                    break;
                };
                let objectives = outcome.get("objectives").cloned().unwrap_or(Value::Null);
                let constraints = outcome.get("constraints").cloned().unwrap_or(Value::Null);
                if fidelity == "screen" && !constraints.is_null() {
                    report["events"].as_array_mut().unwrap().push(event);
                    fail(&mut report, "FAIL", "LOW_FIDELITY_CONSTRAINT_LEAK", "a screen observation cannot establish the hard qualification gate");
                    terminal = "REJECTED_OUTCOME".to_owned();
                    break;
                }
                event["observed_result"] = json!({
                    "objectives": objectives,
                    "constraint_results": if fidelity == "confirmatory" { constraints.clone() } else { Value::Null }
                });
                report["events"].as_array_mut().unwrap().push(event);
                observations.push(json!({
                    "candidate_id": candidate_id,
                    "fidelity": fidelity,
                    "objectives": objectives,
                    "constraint_results": if fidelity == "confirmatory" { constraints.clone() } else { Value::Null }
                }));
                if fidelity == "screen" {
                    report["screen_evaluations"] = json!(report["screen_evaluations"].as_u64().unwrap_or(0) + 1);
                } else {
                    report["confirmatory_evaluations"] = json!(report["confirmatory_evaluations"].as_u64().unwrap_or(0) + 1);
                }
                report["evaluations"] = json!(report["evaluations"].as_u64().unwrap_or(0) + 1);
                terminal = "BUDGET_OR_POLICY_PENDING".to_owned();
            }
        }
    }
    report["terminal_disposition"] = json!(terminal);
    report["cost_units_charged"] = json!(cost_used);
    report["duration_seconds_charged"] = json!(seconds_used);

    let accepted = observations.iter().filter(|observation| {
        observation.get("fidelity").and_then(Value::as_str) == Some("confirmatory")
            && observation.get("constraint_results").and_then(|v| v.get("qualification")).and_then(Value::as_bool) == Some(true)
    }).filter_map(|observation| {
        observation.get("candidate_id").and_then(Value::as_str).map(|id| (id.to_owned(), observation["objectives"].clone()))
    }).collect::<Vec<_>>();
    let infeasible = observations.iter().filter(|observation| {
        observation.get("fidelity").and_then(Value::as_str) == Some("confirmatory")
            && observation.get("constraint_results").and_then(|v| v.get("qualification")).and_then(Value::as_bool) == Some(false)
    }).filter_map(|observation| observation.get("candidate_id").and_then(Value::as_str).map(str::to_owned)).collect::<Vec<_>>();
    let observed_pareto = pareto_ids(&accepted);
    let observed_points = observed_pareto.iter().filter_map(|id| accepted.iter().find(|(candidate_id,_)| candidate_id == id).map(|(_,objectives)| objectives.clone())).collect::<Vec<_>>();
    let true_feasible = feasible_confirmatory_outcomes(public, oracle, environment_id);
    let true_pareto = pareto_ids(&true_feasible);
    let true_points = true_pareto.iter().filter_map(|id| true_feasible.iter().find(|(candidate_id,_)| candidate_id == id).map(|(_,objectives)| objectives.clone())).collect::<Vec<_>>();
    let observed_hv = hypervolume(&observed_points, &public["reference_point"]);
    let oracle_hv = hypervolume(&true_points, &public["reference_point"]);
    let observed_set: HashSet<String> = observed_pareto.iter().cloned().collect();
    let true_set: HashSet<String> = true_pareto.iter().cloned().collect();
    let recall = if true_set.is_empty() { None } else {
        Some(observed_set.intersection(&true_set).count() as f64 / true_set.len() as f64)
    };
    report["accepted_feasible_candidates"] = json!(accepted.iter().map(|(id,_)| id).collect::<Vec<_>>());
    report["confirmed_infeasible_candidates"] = json!(infeasible);
    report["observed_pareto_candidate_ids"] = json!(observed_pareto);
    report["true_pareto_candidate_ids"] = json!(true_pareto);
    report["observed_hypervolume"] = json!(observed_hv);
    report["oracle_hypervolume"] = json!(oracle_hv);
    report["hypervolume_regret"] = json!((oracle_hv - observed_hv).max(0.0));
    report["pareto_recall"] = recall.map_or(Value::Null, |value| json!(value));
    report
}

fn run_all(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let input_digest = sha256(public_raw);
    let oracle_digest = sha256(oracle_raw);
    let mut report = json!({
        "schema_version": 1,
        "status": "PASS",
        "audit_kind": "synthetic_constrained_pareto_fidelity_event_loop",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": input_digest,
        "oracle_sha256": oracle_digest,
        "environment_count": 0,
        "policy_count": POLICY_CLASSES.len(),
        "campaigns_checked": 0,
        "campaigns": [],
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });
    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut report, "FAIL", "PUBLIC_JSON_INVALID", &error.to_string());
            return report;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut report, "FAIL", "ORACLE_JSON_INVALID", &error.to_string());
            return report;
        }
    };
    validate_contract(&public, &oracle, &mut report);
    if report["status"] == "FAIL" {
        return report;
    }
    let tariffs = tariffs_from_oracle(&oracle);
    let environments = oracle.get("environments").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut campaigns = Vec::new();
    for environment in &environments {
        let environment_id = environment.get("environment_id").and_then(Value::as_str).unwrap_or("");
        for policy in POLICY_CLASSES {
            let campaign = run_episode(&public, &oracle, environment_id, policy, &tariffs);
            if campaign["status"] == "FAIL" {
                fail(&mut report, "FAIL", "CAMPAIGN_FAILED", &format!("{environment_id}/{policy} failed a protocol check"));
            } else if campaign["status"] == "UNRESOLVED" {
                fail(&mut report, "UNRESOLVED", "CAMPAIGN_UNRESOLVED", &format!("{environment_id}/{policy} could not be fully scored"));
            }
            campaigns.push(campaign);
        }
    }
    report["environment_count"] = json!(environments.len());
    report["campaigns_checked"] = json!(campaigns.len());
    report["campaigns"] = json!(campaigns);
    report
}

fn audit_replay(report_raw: &[u8], public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut replay = json!({
        "status": "PASS",
        "audit_kind": "synthetic_pareto_fidelity_replay",
        "benchmark_id": BENCHMARK_ID,
        "report_sha256": sha256(report_raw),
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "campaigns_checked": 0,
        "events_checked": 0,
        "cost_units_recomputed": 0.0,
        "findings": []
    });
    let report: Value = match serde_json::from_slice(report_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut replay, "FAIL", "REPORT_JSON_INVALID", &error.to_string());
            return replay;
        }
    };
    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut replay, "FAIL", "PUBLIC_JSON_INVALID", &error.to_string());
            return replay;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut replay, "FAIL", "ORACLE_JSON_INVALID", &error.to_string());
            return replay;
        }
    };

    if report.get("input_sha256").and_then(Value::as_str) != Some(sha256(public_raw).as_str()) {
        fail(&mut replay, "FAIL", "INPUT_DIGEST_MISMATCH", "report does not bind to the exact public input bytes");
    }
    if report.get("oracle_sha256").and_then(Value::as_str) != Some(sha256(oracle_raw).as_str()) {
        fail(&mut replay, "FAIL", "ORACLE_DIGEST_MISMATCH", "report does not bind to the exact sealed-oracle bytes");
    }
    if report.get("status").and_then(Value::as_str) != Some("PASS")
        || report.get("audit_kind").and_then(Value::as_str) != Some("synthetic_constrained_pareto_fidelity_event_loop")
        || report.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || report.get("schema_version").and_then(Value::as_u64) != Some(1)
    {
        fail(&mut replay, "FAIL", "REPORT_CONTRACT_INVALID", "campaign report status, schema version, benchmark ID, or audit kind is invalid");
    }

    let mut contract = json!({"status":"PASS","findings":[]});
    validate_contract(&public, &oracle, &mut contract);
    if contract.get("status").and_then(Value::as_str) != Some("PASS") {
        fail(&mut replay, "FAIL", "INPUT_CONTRACT_INVALID", "public task or sealed oracle does not satisfy its frozen contract");
        return replay;
    }
    let tariffs = tariffs_from_oracle(&oracle);
    let max_cost = public["budgets"]["max_cost_units"].as_f64().unwrap_or(0.0);
    let max_seconds = public["budgets"]["max_duration_seconds"].as_f64().unwrap_or(0.0);
    let environments = oracle.get("environments").and_then(Value::as_array).cloned().unwrap_or_default();
    let campaigns = match report.get("campaigns").and_then(Value::as_array) {
        Some(value) => value,
        None => {
            fail(&mut replay, "FAIL", "CAMPAIGNS_REQUIRED", "report.campaigns must be an array");
            return replay;
        }
    };
    let expected_count = environments.len() * POLICY_CLASSES.len();
    if campaigns.len() != expected_count
        || report.get("campaigns_checked").and_then(Value::as_u64) != Some(campaigns.len() as u64)
        || report.get("environment_count").and_then(Value::as_u64) != Some(environments.len() as u64)
        || report.get("policy_count").and_then(Value::as_u64) != Some(POLICY_CLASSES.len() as u64)
    {
        fail(&mut replay, "FAIL", "CAMPAIGN_COVERAGE_INVALID", &format!("expected {expected_count} environment/policy campaigns with matching summary counts"));
    }

    let mut seen_pairs = HashSet::<(String, String)>::new();
    let mut checked_events = 0usize;
    let mut total_cost = 0.0f64;
    for (campaign_index, campaign) in campaigns.iter().enumerate() {
        let path = format!("report.campaigns[{campaign_index}]");
        only_keys(
            campaign,
            &["status","environment_id","policy_class","events","findings","evaluations","screen_evaluations","confirmatory_evaluations","accepted_feasible_candidates","confirmed_infeasible_candidates","observed_pareto_candidate_ids","true_pareto_candidate_ids","observed_hypervolume","oracle_hypervolume","hypervolume_regret","pareto_recall","cost_units_charged","duration_seconds_charged","claim_ceiling","terminal_disposition"],
            &path,
            &mut replay
        );
        let environment_id = campaign.get("environment_id").and_then(Value::as_str).unwrap_or("");
        let policy = campaign.get("policy_class").and_then(Value::as_str).unwrap_or("");
        if campaign.get("status").and_then(Value::as_str) != Some("PASS") {
            fail(&mut replay, "FAIL", "CAMPAIGN_STATUS_INVALID", &format!("{path} is not a PASS campaign"));
        }
        if !environments.iter().any(|environment| environment.get("environment_id").and_then(Value::as_str) == Some(environment_id)) {
            fail(&mut replay, "FAIL", "CAMPAIGN_ENVIRONMENT_INVALID", &format!("{path} has unknown environment {environment_id}"));
        }
        if !POLICY_CLASSES.contains(&policy) {
            fail(&mut replay, "FAIL", "CAMPAIGN_POLICY_INVALID", &format!("{path} has unregistered policy {policy}"));
        }
        if !seen_pairs.insert((environment_id.to_owned(), policy.to_owned())) {
            fail(&mut replay, "FAIL", "CAMPAIGN_PAIR_DUPLICATE", &format!("duplicate campaign {environment_id}/{policy}"));
        }
        let events = match campaign.get("events").and_then(Value::as_array) {
            Some(value) => value,
            None => {
                fail(&mut replay, "FAIL", "EVENTS_REQUIRED", &format!("{path}.events must be an array"));
                continue;
            }
        };

        let mut observations = Vec::<Value>::new();
        let mut seen = HashSet::<(String, String)>::new();
        let mut cost_used = 0.0f64;
        let mut seconds_used = 0.0f64;
        let mut screen_count = 0usize;
        let mut confirmatory_count = 0usize;

        for (event_index, event) in events.iter().enumerate() {
            let event_path = format!("{path}.events[{event_index}]");
            only_keys(event, &["event_seq","candidate_id","fidelity","decision_view_sha256","cost_units_charged","duration_seconds_charged","observed_result"], &event_path, &mut replay);
            if event.get("event_seq").and_then(Value::as_u64) != Some((event_index + 1) as u64) {
                fail(&mut replay, "FAIL", "EVENT_SEQUENCE_INVALID", &format!("{event_path} event_seq must be consecutive"));
            }
            let candidate_id = event.get("candidate_id").and_then(Value::as_str).unwrap_or("");
            let fidelity = event.get("fidelity").and_then(Value::as_str).unwrap_or("");
            if candidate_by_id(&public, candidate_id).is_none() {
                fail(&mut replay, "FAIL", "CANDIDATE_NOT_REGISTERED", &format!("{event_path} references unknown candidate {candidate_id}"));
                continue;
            }
            if !["screen","confirmatory"].contains(&fidelity) {
                fail(&mut replay, "FAIL", "FIDELITY_NOT_REGISTERED", &format!("{event_path} references unregistered fidelity {fidelity}"));
                continue;
            }
            if !seen.insert((candidate_id.to_owned(), fidelity.to_owned())) {
                fail(&mut replay, "FAIL", "FIDELITY_REPEAT_INVALID", &format!("{event_path} repeats {candidate_id}/{fidelity}"));
            }

            let view = build_public_view(&public, &tariffs, &observations, cost_used, seconds_used);
            let expected_view_digest = sha256(&serde_json::to_vec(&view).unwrap_or_default());
            if event.get("decision_view_sha256").and_then(Value::as_str) != Some(expected_view_digest.as_str()) {
                fail(&mut replay, "FAIL", "DECISION_VIEW_DIGEST_MISMATCH", &format!("{event_path} does not bind to the reconstructed pre-action view"));
            }
            match choose_action(policy, &view) {
                Action::Evaluate { candidate_id: expected_id, fidelity: expected_fidelity } => {
                    if candidate_id != expected_id || fidelity != expected_fidelity {
                        fail(&mut replay, "FAIL", "POLICY_ACTION_MISMATCH", &format!("{event_path} differs from the registered policy action for the reconstructed view"));
                    }
                }
                Action::Stop(reason) => {
                    fail(&mut replay, "FAIL", "ACTION_AFTER_POLICY_STOP", &format!("{event_path} exists after the policy should stop: {reason}"));
                }
            }

            let Some(tariff) = tariffs.get(fidelity) else {
                fail(&mut replay, "FAIL", "TARIFF_MISSING", &format!("{event_path} has no registered tariff"));
                continue;
            };
            if cost_used + tariff.cost > max_cost + 1e-9 || seconds_used + tariff.seconds > max_seconds + 1e-9 {
                fail(&mut replay, "FAIL", "ACTION_OVERRUN", &format!("{event_path} exceeds the shared budget"));
            }
            if !event.get("cost_units_charged").and_then(Value::as_f64).is_some_and(|value| close(value, tariff.cost)) {
                fail(&mut replay, "FAIL", "ACTION_COST_MISMATCH", &format!("{event_path} cost does not match the evaluator-owned tariff"));
            }
            if !event.get("duration_seconds_charged").and_then(Value::as_f64).is_some_and(|value| close(value, tariff.seconds)) {
                fail(&mut replay, "FAIL", "ACTION_DURATION_MISMATCH", &format!("{event_path} duration does not match the evaluator-owned tariff"));
            }

            cost_used += tariff.cost;
            seconds_used += tariff.seconds;
            total_cost += tariff.cost;
            let Some(outcome) = outcome_for(&oracle, environment_id, candidate_id, fidelity) else {
                fail(&mut replay, "UNRESOLVED", "SEALED_OUTCOME_MISSING", &format!("no sealed outcome for {environment_id}/{candidate_id}/{fidelity}"));
                continue;
            };
            let expected_result = json!({
                "objectives": outcome.get("objectives").cloned().unwrap_or(Value::Null),
                "constraint_results": if fidelity == "confirmatory" {
                    outcome.get("constraints").cloned().unwrap_or(Value::Null)
                } else {
                    Value::Null
                }
            });
            if event.get("observed_result") != Some(&expected_result) {
                fail(&mut replay, "FAIL", "OBSERVATION_ORACLE_MISMATCH", &format!("{event_path} observed result does not match the sealed oracle"));
            }
            if fidelity == "screen" && !expected_result["constraint_results"].is_null() {
                fail(&mut replay, "FAIL", "LOW_FIDELITY_CONSTRAINT_LEAK", &format!("{event_path} screen cannot establish hard qualification"));
            }
            observations.push(json!({
                "candidate_id": candidate_id,
                "fidelity": fidelity,
                "objectives": expected_result["objectives"].clone(),
                "constraint_results": expected_result["constraint_results"].clone()
            }));
            if fidelity == "screen" {
                screen_count += 1;
            } else {
                confirmatory_count += 1;
            }
            checked_events += 1;
        }

        let final_view = build_public_view(&public, &tariffs, &observations, cost_used, seconds_used);
        match choose_action(policy, &final_view) {
            Action::Stop(reason) => {
                if campaign.get("terminal_disposition").and_then(Value::as_str) != Some(reason.as_str()) {
                    fail(&mut replay, "FAIL", "TERMINAL_DISPOSITION_MISMATCH", &format!("{path} terminal reason does not match the policy at the final public view"));
                }
            }
            Action::Evaluate { candidate_id, fidelity } => {
                fail(&mut replay, "FAIL", "TRANSCRIPT_TRUNCATED", &format!("{path} ends while the policy still requests {fidelity} for {candidate_id}"));
            }
        }

        let accepted = observations.iter().filter(|observation| {
            observation.get("fidelity").and_then(Value::as_str) == Some("confirmatory")
                && observation.get("constraint_results").and_then(|value| value.get("qualification")).and_then(Value::as_bool) == Some(true)
        }).filter_map(|observation| {
            observation.get("candidate_id").and_then(Value::as_str).map(|id| (id.to_owned(), observation["objectives"].clone()))
        }).collect::<Vec<_>>();
        let infeasible = observations.iter().filter(|observation| {
            observation.get("fidelity").and_then(Value::as_str) == Some("confirmatory")
                && observation.get("constraint_results").and_then(|value| value.get("qualification")).and_then(Value::as_bool) == Some(false)
        }).filter_map(|observation| observation.get("candidate_id").and_then(Value::as_str).map(str::to_owned)).collect::<Vec<_>>();
        let observed_front = pareto_ids(&accepted);
        let observed_points = observed_front.iter().filter_map(|id| accepted.iter().find(|(candidate_id,_)| candidate_id == id).map(|(_,objectives)| objectives.clone())).collect::<Vec<_>>();
        let true_feasible = feasible_confirmatory_outcomes(&public, &oracle, environment_id);
        let true_front = pareto_ids(&true_feasible);
        let true_points = true_front.iter().filter_map(|id| true_feasible.iter().find(|(candidate_id,_)| candidate_id == id).map(|(_,objectives)| objectives.clone())).collect::<Vec<_>>();
        let observed_hv = hypervolume(&observed_points, &public["reference_point"]);
        let oracle_hv = hypervolume(&true_points, &public["reference_point"]);
        let observed_set = observed_front.iter().cloned().collect::<HashSet<_>>();
        let true_set = true_front.iter().cloned().collect::<HashSet<_>>();
        let recall = if true_set.is_empty() { None } else { Some(observed_set.intersection(&true_set).count() as f64 / true_set.len() as f64) };

        if campaign.get("evaluations").and_then(Value::as_u64) != Some(observations.len() as u64)
            || campaign.get("screen_evaluations").and_then(Value::as_u64) != Some(screen_count as u64)
            || campaign.get("confirmatory_evaluations").and_then(Value::as_u64) != Some(confirmatory_count as u64)
        {
            fail(&mut replay, "FAIL", "EVALUATION_COUNT_MISMATCH", &format!("{path} evaluation counts do not replay"));
        }
        if !campaign.get("cost_units_charged").and_then(Value::as_f64).is_some_and(|value| close(value, cost_used))
            || !campaign.get("duration_seconds_charged").and_then(Value::as_f64).is_some_and(|value| close(value, seconds_used))
        {
            fail(&mut replay, "FAIL", "RESOURCE_TOTAL_MISMATCH", &format!("{path} total cost/duration do not replay"));
        }
        if campaign.get("accepted_feasible_candidates").cloned().unwrap_or(Value::Null) != json!(accepted.iter().map(|(id,_)| id.clone()).collect::<Vec<_>>())
            || campaign.get("confirmed_infeasible_candidates").cloned().unwrap_or(Value::Null) != json!(infeasible)
            || campaign.get("observed_pareto_candidate_ids").cloned().unwrap_or(Value::Null) != json!(observed_front)
            || campaign.get("true_pareto_candidate_ids").cloned().unwrap_or(Value::Null) != json!(true_front)
        {
            fail(&mut replay, "FAIL", "FEASIBILITY_OR_PARETO_MISMATCH", &format!("{path} accepted candidates or Pareto sets do not replay"));
        }
        if !campaign.get("observed_hypervolume").and_then(Value::as_f64).is_some_and(|value| close(value, observed_hv))
            || !campaign.get("oracle_hypervolume").and_then(Value::as_f64).is_some_and(|value| close(value, oracle_hv))
            || !campaign.get("hypervolume_regret").and_then(Value::as_f64).is_some_and(|value| close(value, (oracle_hv - observed_hv).max(0.0)))
        {
            fail(&mut replay, "FAIL", "HYPERVOLUME_MISMATCH", &format!("{path} hypervolume or regret does not replay"));
        }
        let recall_matches = match (campaign.get("pareto_recall"), recall) {
            (Some(Value::Null), None) => true,
            (Some(value), Some(expected)) => value.as_f64().is_some_and(|actual| close(actual, expected)),
            _ => false,
        };
        if !recall_matches {
            fail(&mut replay, "FAIL", "PARETO_RECALL_MISMATCH", &format!("{path} Pareto recall does not replay"));
        }

        if cost_used > max_cost + 1e-9 || seconds_used > max_seconds + 1e-9 {
            fail(&mut replay, "FAIL", "CAMPAIGN_BUDGET_OVERRUN", &format!("{path} cumulative resource use exceeds the cap"));
        }
    }

    for environment in &environments {
        let environment_id = environment.get("environment_id").and_then(Value::as_str).unwrap_or("");
        for policy in POLICY_CLASSES {
            if !seen_pairs.contains(&(environment_id.to_owned(), policy.to_owned())) {
                fail(&mut replay, "FAIL", "CAMPAIGN_PAIR_MISSING", &format!("missing campaign {environment_id}/{policy}"));
            }
        }
    }
    replay["campaigns_checked"] = json!(campaigns.len());
    replay["events_checked"] = json!(checked_events);
    replay["cost_units_recomputed"] = json!(total_cost);
    replay
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 5 && args[1] == "--replay" {
        let report_raw = match fs::read(&args[2]) {
            Ok(value) => value,
            Err(error) => { eprintln!("cannot read {}: {error}", args[2]); return ExitCode::from(2); }
        };
        let public_raw = match fs::read(&args[3]) {
            Ok(value) => value,
            Err(error) => { eprintln!("cannot read {}: {error}", args[3]); return ExitCode::from(2); }
        };
        let oracle_raw = match fs::read(&args[4]) {
            Ok(value) => value,
            Err(error) => { eprintln!("cannot read {}: {error}", args[4]); return ExitCode::from(2); }
        };
        let replay = audit_replay(&report_raw, &public_raw, &oracle_raw);
        let code = if replay["status"] == "PASS" { 0 } else { 2 };
        println!("{}", serde_json::to_string_pretty(&replay).expect("replay serializes"));
        return ExitCode::from(code);
    }
    if args.len() != 3 {
        eprintln!("Usage:\n  audit-pareto-fidelity <public-task.json> <sealed-oracle.json>\n  audit-pareto-fidelity --replay <report.json> <public-task.json> <sealed-oracle.json>");
        return ExitCode::from(64);
    }
    let public_raw = match fs::read(&args[1]) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read {}: {error}", args[1]); return ExitCode::from(2); }
    };
    let oracle_raw = match fs::read(&args[2]) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read {}: {error}", args[2]); return ExitCode::from(2); }
    };
    let report = run_all(&public_raw, &oracle_raw);
    let code = match report["status"].as_str().unwrap_or("FAIL") {
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

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/glass-pareto-fidelity-task.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/glass-pareto-fidelity-oracle.synthetic.json");

    fn values() -> (Value, Value) {
        (serde_json::from_slice(PUBLIC).unwrap(), serde_json::from_slice(ORACLE).unwrap())
    }

    fn run_env(public: &Value, oracle: &Value, environment: &str, policy: &str) -> Value {
        let tariffs = tariffs_from_oracle(oracle);
        run_episode(public, oracle, environment, policy, &tariffs)
    }

    #[test]
    fn known_answer_pareto_front_excludes_dominated_and_infeasible_candidates() {
        let (public, oracle) = values();
        let feasible = feasible_confirmatory_outcomes(&public, &oracle, "informative");
        assert_eq!(pareto_ids(&feasible), vec!["glass-a", "glass-b", "glass-c"]);
        assert!(!pareto_ids(&feasible).contains(&"glass-d".to_owned()));
        assert!(!pareto_ids(&feasible).contains(&"glass-e".to_owned()));
    }

    #[test]
    fn hypervolume_known_answer_respects_mixed_objective_directions() {
        let reference = json!({"quality":0.0,"energy":10.0});
        let points = vec![
            json!({"quality":0.90,"energy":8.0}),
            json!({"quality":0.82,"energy":5.0}),
            json!({"quality":0.70,"energy":3.0})
        ];
        assert!(close(hypervolume(&points, &reference), 5.66));
    }

    #[test]
    fn screen_results_never_certify_hard_qualification() {
        let (public, oracle) = values();
        let report = run_env(&public, &oracle, "informative", "screen_only");
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["confirmatory_evaluations"], 0);
        assert_eq!(report["accepted_feasible_candidates"].as_array().unwrap().len(), 0);
        assert_eq!(report["observed_pareto_candidate_ids"].as_array().unwrap().len(), 0);
        assert_eq!(report["observed_hypervolume"], 0.0);
    }

    #[test]
    fn helpful_screening_selects_two_feasible_pareto_candidates_under_the_same_cap() {
        let (public, oracle) = values();
        let screened = run_env(&public, &oracle, "informative", "screen_then_confirm");
        let direct = run_env(&public, &oracle, "informative", "confirmatory_priority");
        assert_eq!(screened["cost_units_charged"], 5.5);
        assert_eq!(direct["cost_units_charged"], 4.0);
        assert_eq!(screened["accepted_feasible_candidates"].as_array().unwrap().len(), 2);
        assert_eq!(screened["observed_pareto_candidate_ids"], json!(["glass-a","glass-b"]));
        assert!(screened["observed_hypervolume"].as_f64().unwrap() > direct["observed_hypervolume"].as_f64().unwrap());
    }

    #[test]
    fn misleading_low_fidelity_is_not_mistaken_for_feasibility_or_high_quality() {
        let (public, oracle) = values();
        let report = run_env(&public, &oracle, "misleading_low_fidelity", "screen_then_confirm");
        assert!(report["confirmed_infeasible_candidates"].as_array().unwrap().iter().any(|id| id == "glass-d"));
        assert!(!report["accepted_feasible_candidates"].as_array().unwrap().iter().any(|id| id == "glass-d"));
        assert_eq!(report["accepted_feasible_candidates"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn objective_directions_reference_point_and_units_cannot_be_changed_post_hoc() {
        let (mut public, oracle) = values();
        public["objectives"][1]["direction"] = json!("maximize");
        let changed_direction = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(changed_direction["status"], "FAIL");
        assert!(changed_direction["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "OBJECTIVE_CONTRACT_CHANGED"));

        let (mut public, oracle) = values();
        public["reference_point"]["energy"] = json!(100.0);
        let changed_reference = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(changed_reference["status"], "FAIL");
        assert!(changed_reference["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "PARETO_REFERENCE_POINT_CHANGED"));
    }

    #[test]
    fn objective_values_must_stay_inside_declared_physical_units_domain() {
        let (public, mut oracle) = values();
        let environment = oracle["environments"].as_array_mut().unwrap().iter_mut()
            .find(|environment| environment["environment_id"] == "informative").unwrap();
        environment["outcomes"].as_array_mut().unwrap().iter_mut()
            .find(|outcome| outcome["candidate_id"] == "glass-a" && outcome["fidelity"] == "confirmatory").unwrap()["objectives"]["quality"] = json!(1.5);
        let report = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "OUTCOME_QUALITY_OUT_OF_RANGE"));
    }

    #[test]
    fn low_fidelity_outcomes_cannot_carry_constraint_labels() {
        let (public, mut oracle) = values();
        let environment = oracle["environments"].as_array_mut().unwrap().iter_mut()
            .find(|environment| environment["environment_id"] == "informative").unwrap();
        environment["outcomes"].as_array_mut().unwrap().iter_mut()
            .find(|outcome| outcome["candidate_id"] == "glass-d" && outcome["fidelity"] == "screen").unwrap()["constraints"] = json!({"qualification":true});
        let report = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "LOW_FIDELITY_CANNOT_QUALIFY"));
    }

    #[test]
    fn costs_are_evaluator_owned_and_every_action_is_charged_before_reveal() {
        let (public, oracle) = values();
        let report = run_env(&public, &oracle, "informative", "screen_then_confirm");
        assert_eq!(report["events"][0]["fidelity"], "screen");
        assert_eq!(report["events"][0]["cost_units_charged"], 0.5);
        assert_eq!(report["events"][0]["observed_result"]["constraint_results"], Value::Null);
        assert_eq!(report["events"][3]["fidelity"], "confirmatory");
        assert_eq!(report["events"][3]["cost_units_charged"], 2.0);
        assert_eq!(report["cost_units_charged"], 5.5);
        assert!(report["events"].as_array().unwrap().iter().all(|event| event["decision_view_sha256"].as_str().is_some_and(|digest| digest.len() == 64)));
    }

    #[test]
    fn public_catalog_cannot_inject_hidden_material_outcomes() {
        let (mut public, oracle) = values();
        public["candidates"][0]["hidden_outcome"] = json!({"quality":1.0,"energy":0.0,"qualification":true});
        let report = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "FAIL");
        assert!(report["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "UNKNOWN_PROPERTY"));
    }

    #[test]
    fn missing_high_fidelity_outcome_is_unresolved_not_zero() {
        let (public, mut oracle) = values();
        let environment = oracle["environments"].as_array_mut().unwrap().iter_mut()
            .find(|environment| environment["environment_id"] == "informative").unwrap();
        environment["outcomes"].as_array_mut().unwrap().retain(|outcome| !(outcome["candidate_id"] == "glass-a" && outcome["fidelity"] == "confirmatory"));
        let report = run_all(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(report["status"], "UNRESOLVED");
        assert!(report["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "OUTCOME_COVERAGE_INCOMPLETE"));
    }

    #[test]
    fn independent_replay_rejects_a_forged_hypervolume() {
        let report = run_all(PUBLIC, ORACLE);
        let mut forged = report.clone();
        forged["campaigns"][0]["observed_hypervolume"] = json!(99999.0);
        let raw = serde_json::to_vec(&forged).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "TRANSCRIPT_REPLAY_MISMATCH"));
    }

    #[test]
    fn event_replay_rejects_a_legal_but_policy_inconsistent_action() {
        let report = run_all(PUBLIC, ORACLE);
        let mut forged = report.clone();
        forged["campaigns"][0]["events"][0]["candidate_id"] = json!("glass-a");
        let raw = serde_json::to_vec(&forged).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "POLICY_ACTION_MISMATCH"));
    }

    #[test]
    fn event_replay_rejects_underreported_fidelity_cost() {
        let report = run_all(PUBLIC, ORACLE);
        let mut forged = report.clone();
        forged["campaigns"][0]["events"][0]["cost_units_charged"] = json!(0.001);
        let raw = serde_json::to_vec(&forged).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "ACTION_COST_MISMATCH"));
    }

    #[test]
    fn event_replay_rejects_a_forged_pre_action_view_digest() {
        let report = run_all(PUBLIC, ORACLE);
        let mut forged = report.clone();
        forged["campaigns"][0]["events"][0]["decision_view_sha256"] = json!("0".repeat(64));
        let raw = serde_json::to_vec(&forged).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "FAIL");
        assert!(replay["findings"].as_array().unwrap().iter().any(|finding| finding["code"] == "DECISION_VIEW_DIGEST_MISMATCH"));
    }

    #[test]
    fn exact_inputs_replay_to_identical_campaigns_and_metrics() {
        let report = run_all(PUBLIC, ORACLE);
        let raw = serde_json::to_vec(&report).unwrap();
        let replay = audit_replay(&raw, PUBLIC, ORACLE);
        assert_eq!(replay["status"], "PASS");
        assert_eq!(replay["campaigns_checked"], 12);
        assert!(replay["events_checked"].as_u64().unwrap() > 0);
    }
}
