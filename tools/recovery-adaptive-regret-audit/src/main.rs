use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-adaptive-regret-corpus-v0.1.json");
const PV_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-adaptive-regret-pv-corpus-v0.1.json");

#[derive(Debug, Clone)]
struct ActionOption {
    cost_cents: i64,
    gate_status: String,
    reason_code: String,
}

#[derive(Debug, Clone)]
struct Scenario {
    id: String,
    initial_observation: String,
    post_inspection_observation: Option<String>,
    actions: BTreeMap<String, ActionOption>,
    best_action: String,
    best_cost_cents: i64,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct ScenarioOutcome {
    scenario_id: String,
    initial_observation: String,
    inspected: bool,
    selected_action: Option<String>,
    best_feasible_action: String,
    best_feasible_cost_cents: i64,
    inspection_cost_cents: i64,
    operation_cost_cents: Option<i64>,
    total_scenario_cost_cents: Option<i64>,
    regret_cents: Option<i64>,
    outcome: String,
    reason_code: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct PolicyResult {
    policy_id: String,
    feasible_across_scenario_set: bool,
    ineligible_scenario_ids: Vec<String>,
    total_unweighted_scenario_cost_cents: Option<i64>,
    worst_case_regret_cents: Option<i64>,
    total_unweighted_regret_cents: Option<i64>,
    outcomes: Vec<ScenarioOutcome>,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    profile_id: String,
    case_id: String,
    corpus_sha256: String,
    source_commit: Option<String>,
    scenario_count: usize,
    policy_count: usize,
    operation_action_count: usize,
    feasible_policy_count: usize,
    policy_results: Vec<PolicyResult>,
    frontier: Vec<String>,
    expected_frontier_matches: bool,
    aggregation_semantics: String,
    all_plan_only: bool,
    claim_ceiling: String,
    errors: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join("")
}

fn nonempty_string<'a>(value: Option<&'a Value>) -> Option<&'a str> {
    value.and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn is_hex_sha(value: &str, digits: usize) -> bool {
    value.len() == digits && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_scenarios(
    root: &Value,
    profile_id: &str,
    errors: &mut Vec<String>,
) -> (Vec<Scenario>, BTreeSet<String>, BTreeSet<String>) {
    let mut scenarios = Vec::new();
    let mut scenario_ids = BTreeSet::new();
    let mut observation_classes = BTreeSet::new();
    let mut operation_names = BTreeSet::new();
    let entries = match root.get("scenarios").and_then(Value::as_array) {
        Some(entries) if !entries.is_empty() => entries,
        _ => {
            errors.push("SCENARIOS_MISSING_OR_EMPTY".to_string());
            return (scenarios, observation_classes, operation_names);
        }
    };

    match root.get("scenario_count").and_then(Value::as_u64) {
        Some(declared) if declared as usize == entries.len() => {}
        Some(declared) => errors.push(format!(
            "SCENARIO_COUNT_MISMATCH: declared {declared}, actual {}",
            entries.len()
        )),
        None => errors.push("SCENARIO_COUNT_MISSING".to_string()),
    }

    for (index, entry) in entries.iter().enumerate() {
        let label = format!("SCENARIO_{}", index + 1);
        let id = match nonempty_string(entry.get("id")) {
            Some(id) => id.to_string(),
            None => {
                errors.push(format!("{label}_ID_MISSING"));
                continue;
            }
        };
        if !scenario_ids.insert(id.clone()) {
            errors.push(format!("{id}: DUPLICATE_SCENARIO_ID"));
        }
        if nonempty_string(entry.get("domain_profile_id")) != Some(profile_id) {
            errors.push(format!("{id}: DOMAIN_PROFILE_MISMATCH"));
        }
        let initial = match nonempty_string(entry.get("initial_observation")) {
            Some(value) => value.to_string(),
            None => {
                errors.push(format!("{id}: INITIAL_OBSERVATION_MISSING"));
                continue;
            }
        };
        observation_classes.insert(initial.clone());
        let post = entry
            .get("post_inspection_observation")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);

        let mut actions = BTreeMap::new();
        match entry.get("action_options").and_then(Value::as_object) {
            Some(options) if !options.is_empty() => {
                for (action_id, option) in options {
                    if action_id.trim().is_empty() {
                        errors.push(format!("{id}: ACTION_ID_EMPTY"));
                        continue;
                    }
                    operation_names.insert(action_id.clone());
                    let cost = option.get("cost_cents").and_then(Value::as_i64);
                    let gate_status = nonempty_string(option.get("gate_status"));
                    let reason_code = nonempty_string(option.get("reason_code"));
                    if cost.map(|value| value < 0).unwrap_or(true) {
                        errors.push(format!("{id}: ACTION_COST_INVALID: {action_id}"));
                    }
                    if !matches!(gate_status, Some("PASS" | "BLOCK" | "UNKNOWN")) {
                        errors.push(format!("{id}: ACTION_GATE_STATUS_INVALID: {action_id}"));
                    }
                    if reason_code.is_none() {
                        errors.push(format!("{id}: ACTION_REASON_CODE_MISSING: {action_id}"));
                    }
                    if let (Some(cost_cents), Some(gate_status), Some(reason_code)) =
                        (cost, gate_status, reason_code)
                    {
                        actions.insert(action_id.clone(), ActionOption {
                            cost_cents,
                            gate_status: gate_status.to_string(),
                            reason_code: reason_code.to_string(),
                        });
                    }
                }
            }
            _ => errors.push(format!("{id}: ACTION_OPTIONS_MISSING_OR_EMPTY")),
        }

        let best = actions.iter()
            .filter(|(_, option)| option.gate_status == "PASS")
            .min_by(|(id_a, a), (id_b, b)| a.cost_cents.cmp(&b.cost_cents).then_with(|| id_a.cmp(id_b)));

        let (best_action, best_cost_cents) = match best {
            Some((action, option)) => (action.clone(), option.cost_cents),
            None => {
                errors.push(format!("{id}: NO_PASSING_ACTION_FOR_SCENARIO"));
                continue;
            }
        };
        scenarios.push(Scenario {
            id,
            initial_observation: initial,
            post_inspection_observation: post,
            actions,
            best_action,
            best_cost_cents,
        });
    }

    if scenario_ids.len() != entries.len() {
        errors.push("SCENARIO_ID_COVERAGE_MISMATCH".to_string());
    }
    (scenarios, observation_classes, operation_names)
}

fn parse_policy_decisions(
    value: Option<&Value>,
    required_keys: &BTreeSet<String>,
    allowed_actions: &BTreeSet<String>,
    label: &str,
    errors: &mut Vec<String>,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();
    let object = match value.and_then(Value::as_object) {
        Some(object) => object,
        None => {
            errors.push(format!("{label}_MISSING_OR_NOT_OBJECT"));
            return result;
        }
    };
    let keys: BTreeSet<String> = object.keys().cloned().collect();
    for key in required_keys.difference(&keys) {
        errors.push(format!("{label}_MISSING_OBSERVATION_RULE: {key}"));
    }
    for key in keys.difference(required_keys) {
        errors.push(format!("{label}_UNKNOWN_OBSERVATION_RULE: {key}"));
    }
    for (observation, action_value) in object {
        match nonempty_string(Some(action_value)) {
            Some(action) => {
                if action != "inspect" && !allowed_actions.contains(action) {
                    errors.push(format!("{label}_UNKNOWN_ACTION: {observation} -> {action}"));
                }
                result.insert(observation.clone(), action.to_string());
            }
            None => errors.push(format!("{label}_EMPTY_ACTION: {observation}")),
        }
    }
    result
}

fn add_checked(a: i64, b: i64, label: &str, errors: &mut Vec<String>) -> Option<i64> {
    match a.checked_add(b) {
        Some(value) => Some(value),
        None => {
            errors.push(format!("{label}_ARITHMETIC_OVERFLOW"));
            None
        }
    }
}

fn evaluate_policy(
    policy: &Value,
    scenarios: &[Scenario],
    observation_classes: &BTreeSet<String>,
    operation_names: &BTreeSet<String>,
    profile_id: &str,
    errors: &mut Vec<String>,
) -> Option<PolicyResult> {
    let policy_id = nonempty_string(policy.get("id"))?.to_string();
    if nonempty_string(policy.get("domain_profile_id")) != Some(profile_id) {
        errors.push(format!("{policy_id}: POLICY_PROFILE_MISMATCH"));
    }
    if policy.get("scenario_overrides").is_some() {
        errors.push(format!("{policy_id}: SCENARIO_SPECIFIC_OVERRIDE_FORBIDDEN"));
    }

    let initial = parse_policy_decisions(
        policy.get("initial_decisions"),
        observation_classes,
        operation_names,
        &format!("{policy_id}: INITIAL_DECISIONS"),
        errors,
    );
    let inspection_cost = policy.get("inspection_cost_cents").and_then(Value::as_i64);
    if inspection_cost.map(|value| value < 0).unwrap_or(true) {
        errors.push(format!("{policy_id}: INSPECTION_COST_INVALID"));
    }
    let inspection_cost = inspection_cost.unwrap_or(0);
    let any_inspection = initial.values().any(|action| action == "inspect");
    if !any_inspection && inspection_cost != 0 {
        errors.push(format!("{policy_id}: UNUSED_INSPECTION_COST_NONZERO"));
    }

    let relevant_post_observations: BTreeSet<String> = scenarios.iter()
        .filter(|scenario| initial.get(&scenario.initial_observation).map(String::as_str) == Some("inspect"))
        .filter_map(|scenario| scenario.post_inspection_observation.clone())
        .collect();
    let post_decisions = if any_inspection {
        parse_policy_decisions(
            policy.get("post_inspection_decisions"),
            &relevant_post_observations,
            operation_names,
            &format!("{policy_id}: POST_INSPECTION_DECISIONS"),
            errors,
        )
    } else {
        match policy.get("post_inspection_decisions").and_then(Value::as_object) {
            Some(object) if object.is_empty() => {}
            None => errors.push(format!("{policy_id}: POST_INSPECTION_DECISIONS_MISSING_OR_NOT_OBJECT")),
            Some(_) => errors.push(format!("{policy_id}: POST_INSPECTION_RULES_WITHOUT_INSPECTION")),
        }
        BTreeMap::new()
    };

    let mut outcomes = Vec::new();
    let mut ineligible_ids = Vec::new();
    let mut total_cost = Some(0i64);
    let mut total_regret = Some(0i64);
    let mut worst_regret: Option<i64> = Some(0);

    for scenario in scenarios {
        let initial_action = match initial.get(&scenario.initial_observation) {
            Some(action) => action,
            None => {
                errors.push(format!("{policy_id}: NO_DECISION_FOR_SCENARIO: {}", scenario.id));
                ineligible_ids.push(scenario.id.clone());
                total_cost = None;
                total_regret = None;
                worst_regret = None;
                outcomes.push(ScenarioOutcome {
                    scenario_id: scenario.id.clone(),
                    initial_observation: scenario.initial_observation.clone(),
                    inspected: false,
                    selected_action: None,
                    best_feasible_action: scenario.best_action.clone(),
                    best_feasible_cost_cents: scenario.best_cost_cents,
                    inspection_cost_cents: 0,
                    operation_cost_cents: None,
                    total_scenario_cost_cents: None,
                    regret_cents: None,
                    outcome: "INELIGIBLE".to_string(),
                    reason_code: Some("OBSERVATION_RULE_MISSING".to_string()),
                });
                continue;
            }
        };

        let (inspected, selected_action, incurred_inspection_cost) = if initial_action == "inspect" {
            match scenario.post_inspection_observation.as_ref() {
                Some(post_observation) => match post_decisions.get(post_observation) {
                    Some(action) => (true, action.clone(), inspection_cost),
                    None => {
                        errors.push(format!("{policy_id}: POST_INSPECTION_BRANCH_MISSING: {}", scenario.id));
                        ineligible_ids.push(scenario.id.clone());
                        total_cost = None;
                        total_regret = None;
                        worst_regret = None;
                        outcomes.push(ScenarioOutcome {
                            scenario_id: scenario.id.clone(),
                            initial_observation: scenario.initial_observation.clone(),
                            inspected: true,
                            selected_action: None,
                            best_feasible_action: scenario.best_action.clone(),
                            best_feasible_cost_cents: scenario.best_cost_cents,
                            inspection_cost_cents: inspection_cost,
                            operation_cost_cents: None,
                            total_scenario_cost_cents: None,
                            regret_cents: None,
                            outcome: "INVALID_POLICY".to_string(),
                            reason_code: Some("POST_INSPECTION_BRANCH_MISSING".to_string()),
                        });
                        continue;
                    }
                },
                None => {
                    errors.push(format!("{policy_id}: INSPECTION_RESULT_UNAVAILABLE: {}", scenario.id));
                    ineligible_ids.push(scenario.id.clone());
                    total_cost = None;
                    total_regret = None;
                    worst_regret = None;
                    outcomes.push(ScenarioOutcome {
                        scenario_id: scenario.id.clone(),
                        initial_observation: scenario.initial_observation.clone(),
                        inspected: true,
                        selected_action: None,
                        best_feasible_action: scenario.best_action.clone(),
                        best_feasible_cost_cents: scenario.best_cost_cents,
                        inspection_cost_cents: inspection_cost,
                        operation_cost_cents: None,
                        total_scenario_cost_cents: None,
                        regret_cents: None,
                        outcome: "INVALID_POLICY".to_string(),
                        reason_code: Some("INSPECTION_RESULT_UNAVAILABLE".to_string()),
                    });
                    continue;
                }
            }
        } else {
            (false, initial_action.clone(), 0)
        };

        match scenario.actions.get(&selected_action) {
            None => {
                errors.push(format!("{policy_id}: ACTION_NOT_AVAILABLE_IN_SCENARIO: {} -> {selected_action}", scenario.id));
                ineligible_ids.push(scenario.id.clone());
                total_cost = None;
                total_regret = None;
                worst_regret = None;
                outcomes.push(ScenarioOutcome {
                    scenario_id: scenario.id.clone(),
                    initial_observation: scenario.initial_observation.clone(),
                    inspected,
                    selected_action: Some(selected_action),
                    best_feasible_action: scenario.best_action.clone(),
                    best_feasible_cost_cents: scenario.best_cost_cents,
                    inspection_cost_cents: incurred_inspection_cost,
                    operation_cost_cents: None,
                    total_scenario_cost_cents: None,
                    regret_cents: None,
                    outcome: "INELIGIBLE".to_string(),
                    reason_code: Some("ACTION_NOT_AVAILABLE_IN_SCENARIO".to_string()),
                });
            }
            Some(option) if option.gate_status != "PASS" => {
                ineligible_ids.push(scenario.id.clone());
                total_cost = None;
                total_regret = None;
                worst_regret = None;
                outcomes.push(ScenarioOutcome {
                    scenario_id: scenario.id.clone(),
                    initial_observation: scenario.initial_observation.clone(),
                    inspected,
                    selected_action: Some(selected_action),
                    best_feasible_action: scenario.best_action.clone(),
                    best_feasible_cost_cents: scenario.best_cost_cents,
                    inspection_cost_cents: incurred_inspection_cost,
                    operation_cost_cents: Some(option.cost_cents),
                    total_scenario_cost_cents: None,
                    regret_cents: None,
                    outcome: format!("INELIGIBLE_{}", option.gate_status),
                    reason_code: Some(option.reason_code.clone()),
                });
            }
            Some(option) => {
                let maybe_total = add_checked(
                    option.cost_cents,
                    incurred_inspection_cost,
                    &format!("{policy_id}:{}", scenario.id),
                    errors,
                );
                let maybe_regret = maybe_total.and_then(|cost| {
                    cost.checked_sub(scenario.best_cost_cents).or_else(|| {
                        errors.push(format!("{policy_id}:{}: REGRET_ARITHMETIC_UNDERFLOW", scenario.id));
                        None
                    })
                });
                if maybe_regret.map(|value| value < 0).unwrap_or(false) {
                    errors.push(format!("{policy_id}:{}: NEGATIVE_REGRET", scenario.id));
                }
                if let Some(cost) = maybe_total {
                    if let Some(total) = total_cost {
                        match total.checked_add(cost) {
                            Some(next) => total_cost = Some(next),
                            None => {
                                total_cost = None;
                                errors.push(format!("{policy_id}: TOTAL_COST_OVERFLOW"));
                            }
                        }
                    }
                } else {
                    total_cost = None;
                }
                if let Some(regret) = maybe_regret {
                    if let Some(total) = total_regret {
                        match total.checked_add(regret) {
                            Some(next) => total_regret = Some(next),
                            None => {
                                total_regret = None;
                                errors.push(format!("{policy_id}: TOTAL_REGRET_OVERFLOW"));
                            }
                        }
                    }
                    worst_regret = worst_regret.map(|worst| worst.max(regret));
                } else {
                    total_regret = None;
                    worst_regret = None;
                }
                outcomes.push(ScenarioOutcome {
                    scenario_id: scenario.id.clone(),
                    initial_observation: scenario.initial_observation.clone(),
                    inspected,
                    selected_action: Some(selected_action),
                    best_feasible_action: scenario.best_action.clone(),
                    best_feasible_cost_cents: scenario.best_cost_cents,
                    inspection_cost_cents: incurred_inspection_cost,
                    operation_cost_cents: Some(option.cost_cents),
                    total_scenario_cost_cents: maybe_total,
                    regret_cents: maybe_regret,
                    outcome: "FEASIBLE".to_string(),
                    reason_code: None,
                });
            }
        }
    }

    ineligible_ids.sort();
    ineligible_ids.dedup();
    let feasible = ineligible_ids.is_empty() && total_cost.is_some() && worst_regret.is_some();
    let result = PolicyResult {
        policy_id: policy_id.clone(),
        feasible_across_scenario_set: feasible,
        ineligible_scenario_ids: ineligible_ids.clone(),
        total_unweighted_scenario_cost_cents: if feasible { total_cost } else { None },
        worst_case_regret_cents: if feasible { worst_regret } else { None },
        total_unweighted_regret_cents: if feasible { total_regret } else { None },
        outcomes,
    };

    if policy.get("expected_feasible").and_then(Value::as_bool) != Some(feasible) {
        errors.push(format!("{policy_id}: EXPECTED_FEASIBILITY_MISMATCH"));
    }
    let expected_ineligible: Vec<String> = match policy.get("expected_ineligible_scenario_ids").and_then(Value::as_array) {
        Some(ids) => {
            let mut strings = Vec::new();
            for id in ids {
                match nonempty_string(Some(id)) {
                    Some(id) => strings.push(id.to_string()),
                    None => errors.push(format!("{policy_id}: EXPECTED_INELIGIBLE_ID_INVALID")),
                }
            }
            strings.sort();
            strings
        }
        None => {
            errors.push(format!("{policy_id}: EXPECTED_INELIGIBLE_IDS_MISSING"));
            Vec::new()
        }
    };
    if expected_ineligible != result.ineligible_scenario_ids {
        errors.push(format!("{policy_id}: EXPECTED_INELIGIBLE_SET_MISMATCH"));
    }

    let compare_expected_metric = |field: &str, actual: Option<i64>, errors: &mut Vec<String>| {
        let expected = policy.get(field).and_then(Value::as_i64);
        let expected_is_null = policy.get(field).map(Value::is_null).unwrap_or(false);
        match (actual, expected, expected_is_null) {
            (Some(actual), Some(expected), _) if actual == expected => {}
            (None, None, true) => {}
            _ => errors.push(format!("{policy_id}: EXPECTED_METRIC_MISMATCH: {field}")),
        }
    };
    compare_expected_metric("expected_total_scenario_cost_cents", result.total_unweighted_scenario_cost_cents, errors);
    compare_expected_metric("expected_worst_regret_cents", result.worst_case_regret_cents, errors);
    compare_expected_metric("expected_total_regret_cents", result.total_unweighted_regret_cents, errors);

    Some(result)
}

fn dominates(a: &PolicyResult, b: &PolicyResult) -> bool {
    if !a.feasible_across_scenario_set || !b.feasible_across_scenario_set {
        return false;
    }
    let (a_cost, a_regret, b_cost, b_regret) = match (
        a.total_unweighted_scenario_cost_cents,
        a.worst_case_regret_cents,
        b.total_unweighted_scenario_cost_cents,
        b.worst_case_regret_cents,
    ) {
        (Some(ac), Some(ar), Some(bc), Some(br)) => (ac, ar, bc, br),
        _ => return false,
    };
    a_cost <= b_cost && a_regret <= b_regret && (a_cost < b_cost || a_regret < b_regret)
}

fn compute_frontier(results: &[PolicyResult]) -> Vec<String> {
    let mut frontier: Vec<String> = results.iter()
        .filter(|candidate| candidate.feasible_across_scenario_set)
        .filter(|candidate| !results.iter().any(|other| dominates(other, candidate)))
        .map(|result| result.policy_id.clone())
        .collect();
    frontier.sort();
    frontier
}

fn make_receipt(
    raw: &[u8],
    source_commit: Option<String>,
    parsed: Option<Value>,
    parse_error: Option<String>,
) -> AuditReceipt {
    let had_parse_error = parse_error.is_some();
    let mut errors = Vec::new();
    let mut schema_version = "unavailable".to_string();
    let mut algorithm_id = "unavailable".to_string();
    let mut profile_id = "unavailable".to_string();
    let mut case_id = "unavailable".to_string();
    let mut scenario_count = 0usize;
    let mut policy_count = 0usize;
    let mut operation_action_count = 0usize;
    let mut all_plan_only = false;
    let mut policy_results = Vec::new();
    let mut frontier = Vec::new();
    let mut expected_frontier_matches = false;

    if let Some(error) = parse_error {
        errors.push(format!("CORPUS_JSON_INVALID: {error}"));
    }
    if let Some(commit) = source_commit.as_deref() {
        if !is_hex_sha(commit, 40) {
            errors.push("SOURCE_COMMIT_INVALID".to_string());
        }
    }

    if let Some(root) = parsed.as_ref() {
        schema_version = nonempty_string(root.get("schema_version")).unwrap_or("unavailable").to_string();
        algorithm_id = nonempty_string(root.get("algorithm_id")).unwrap_or("unavailable").to_string();
        profile_id = nonempty_string(root.get("decision_profile_id")).unwrap_or("unavailable").to_string();
        case_id = nonempty_string(root.get("case_id")).unwrap_or("unavailable").to_string();

        if schema_version != "adaptive-recovery-regret-v0.1" {
            errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
        }
        if algorithm_id != "scenario-observation-regret-frontier-v0.1" {
            errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
        }
        if nonempty_string(root.get("status")) != Some("synthetic-logic-fixture") {
            errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
        }
        let claim = root.get("claim_ceiling");
        all_plan_only = claim.and_then(|v| v.get("expected_actuation")).and_then(Value::as_bool) == Some(false);
        if !all_plan_only {
            errors.push("ACTUATION_CEILING_INVALID".to_string());
        }
        if claim.and_then(|v| v.get("real_world_measurements")).and_then(Value::as_bool) != Some(false) {
            errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
        }
        if claim.and_then(|v| v.get("field_accuracy_claimed")).and_then(Value::as_bool) != Some(false) {
            errors.push("FIELD_ACCURACY_MUST_NOT_BE_CLAIMED".to_string());
        }
        if nonempty_string(claim.and_then(|v| v.get("execution_authority"))) != Some("NO_ACTUATION_AUTHORIZED") {
            errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
        }
        if root.get("scenario_semantics").and_then(|v| v.get("probabilities_supplied")).and_then(Value::as_bool) != Some(false) {
            errors.push("SCENARIO_PROBABILITIES_MUST_NOT_BE_FABRICATED".to_string());
        }
        if nonempty_string(root.get("scenario_semantics").and_then(|v| v.get("aggregation")))
            != Some("unweighted scenario sum; not an expected-value estimate or probability distribution")
        {
            errors.push("SCENARIO_AGGREGATION_SEMANTICS_INVALID".to_string());
        }

        let (scenarios, observation_classes, operation_names) = parse_scenarios(root, &profile_id, &mut errors);
        scenario_count = scenarios.len();
        operation_action_count = operation_names.len();
        let policies: Vec<&Value> = match root.get("policies").and_then(Value::as_array) {
            Some(policies) if !policies.is_empty() => policies.iter().collect(),
            _ => {
                errors.push("POLICIES_MISSING_OR_EMPTY".to_string());
                Vec::new()
            }
        };
        policy_count = policies.len();
        match root.get("policy_count").and_then(Value::as_u64) {
            Some(declared) if declared as usize == policy_count => {}
            Some(declared) => errors.push(format!("POLICY_COUNT_MISMATCH: declared {declared}, actual {policy_count}")),
            None => errors.push("POLICY_COUNT_MISSING".to_string()),
        }

        let mut policy_ids = BTreeSet::new();
        for (index, policy) in policies.iter().enumerate() {
            let label = format!("POLICY_{}", index + 1);
            match nonempty_string(policy.get("id")) {
                Some(id) => if !policy_ids.insert(id.to_string()) {
                    errors.push(format!("{id}: DUPLICATE_POLICY_ID"));
                },
                None => {
                    errors.push(format!("{label}_ID_MISSING"));
                    continue;
                }
            }
            if let Some(result) = evaluate_policy(policy, &scenarios, &observation_classes, &operation_names, &profile_id, &mut errors) {
                policy_results.push(result);
            } else {
                errors.push(format!("{label}_EVALUATION_FAILED"));
            }
        }

        if policy_ids.len() != policy_count {
            errors.push("POLICY_ID_COVERAGE_MISMATCH".to_string());
        }
        policy_results.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));
        frontier = compute_frontier(&policy_results);
        let expected_frontier_value = root.get("expected_frontier").and_then(Value::as_array);
        let mut expected_frontier = Vec::new();
        match expected_frontier_value {
            Some(values) => {
                for value in values {
                    match nonempty_string(Some(value)) {
                        Some(id) => expected_frontier.push(id.to_string()),
                        None => errors.push("EXPECTED_FRONTIER_ENTRY_INVALID".to_string()),
                    }
                }
                expected_frontier.sort();
            }
            None => errors.push("EXPECTED_FRONTIER_MISSING".to_string()),
        }
        expected_frontier_matches = frontier == expected_frontier;
        if !expected_frontier_matches {
            errors.push("EXPECTED_FRONTIER_MISMATCH".to_string());
        }
        for id in &expected_frontier {
            if !policy_ids.contains(id) {
                errors.push(format!("EXPECTED_FRONTIER_UNKNOWN_POLICY: {id}"));
            }
        }

        AuditReceipt {
            result: if errors.is_empty() { "pass" } else { "fail" }.to_string(),
            schema_version,
            algorithm_id,
            profile_id,
            case_id,
            corpus_sha256: sha256_hex(raw),
            source_commit,
            scenario_count,
            policy_count,
            operation_action_count,
            feasible_policy_count: policy_results.iter().filter(|policy| policy.feasible_across_scenario_set).count(),
            policy_results,
            frontier,
            expected_frontier_matches,
            aggregation_semantics: "unweighted synthetic scenario sums; not a probability, expected value or field forecast".to_string(),
            all_plan_only,
            claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(),
            errors,
        }
    } else {
        if !had_parse_error {
            errors.push("CORPUS_VALUE_MISSING".to_string());
        }
        AuditReceipt {
            result: "fail".to_string(),
            schema_version,
            algorithm_id,
            profile_id,
            case_id,
            corpus_sha256: sha256_hex(raw),
            source_commit,
            scenario_count,
            policy_count,
            operation_action_count,
            feasible_policy_count: 0,
            policy_results,
            frontier,
            expected_frontier_matches,
            aggregation_semantics: "unweighted synthetic scenario sums; not a probability, expected value or field forecast".to_string(),
            all_plan_only,
            claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(),
            errors,
        }
    }
}

fn run() -> Result<(), String> {
    let mut path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-commit" => source_commit = Some(args.next().ok_or_else(|| "--source-commit requires a value".to_string())?),
            "--help" | "-h" => {
                println!("Usage: recovery-adaptive-regret-audit <corpus.json> [--source-commit <40-hex-sha>]");
                return Ok(());
            }
            _ if arg.starts_with('-') => return Err(format!("unknown argument: {arg}")),
            _ if path.is_none() => path = Some(arg),
            _ => return Err("provide exactly one corpus path".to_string()),
        }
    }
    let path = path.ok_or_else(|| "missing corpus path; use --help for usage".to_string())?;
    let bytes = fs::read(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let receipt = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => make_receipt(&bytes, source_commit, Some(value), None),
        Err(error) => make_receipt(&bytes, source_commit, None, Some(error.to_string())),
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| format!("receipt serialization failed: {error}"))?);
    if receipt.result != "pass" {
        return Err("adaptive scenario-regret audit failed".to_string());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("recovery-adaptive-regret-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn embedded() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("forest adaptive corpus must parse")
    }

    fn embedded_pv() -> Value {
        serde_json::from_str(PV_CORPUS).expect("PV adaptive corpus must parse")
    }

    fn audit(value: Value) -> AuditReceipt {
        let bytes = serde_json::to_vec(&value).expect("test input serializes");
        make_receipt(&bytes, None, Some(value), None)
    }

    fn independent_oracle(value: &Value) -> (Vec<(String, bool, Option<i64>, Option<i64>)>, Vec<String>) {
        let scenarios = value["scenarios"].as_array().unwrap();
        let mut scenario_best = BTreeMap::new();
        for scenario in scenarios {
            let mut candidates: Vec<(String, i64)> = scenario["action_options"].as_object().unwrap().iter()
                .filter_map(|(id, item)| {
                    (item["gate_status"].as_str() == Some("PASS"))
                        .then(|| (id.clone(), item["cost_cents"].as_i64().unwrap()))
                })
                .collect();
            candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
            scenario_best.insert(scenario["id"].as_str().unwrap().to_string(), candidates[0].clone());
        }
        let mut results = Vec::new();
        for policy in value["policies"].as_array().unwrap() {
            let id = policy["id"].as_str().unwrap().to_string();
            let initial = policy["initial_decisions"].as_object().unwrap();
            let post = policy["post_inspection_decisions"].as_object().unwrap();
            let mut costs = Vec::new();
            let mut regrets = Vec::new();
            let mut feasible = true;
            for scenario in scenarios {
                let obs = scenario["initial_observation"].as_str().unwrap();
                let mut action = initial[obs].as_str().unwrap();
                let mut inspection_cost = 0i64;
                if action == "inspect" {
                    let post_obs = scenario["post_inspection_observation"].as_str().unwrap();
                    action = post[post_obs].as_str().unwrap();
                    inspection_cost = policy["inspection_cost_cents"].as_i64().unwrap();
                }
                let option = &scenario["action_options"][action];
                if option["gate_status"].as_str() != Some("PASS") {
                    feasible = false;
                    break;
                }
                let cost = option["cost_cents"].as_i64().unwrap() + inspection_cost;
                let best = scenario_best[scenario["id"].as_str().unwrap()].1;
                costs.push(cost);
                regrets.push(cost - best);
            }
            let total = feasible.then(|| costs.iter().sum());
            let worst = feasible.then(|| *regrets.iter().max().unwrap());
            results.push((id, feasible, total, worst));
        }
        let mut frontier: Vec<String> = results.iter().filter(|x| x.1).filter(|candidate| {
            !results.iter().any(|other| other.1 && other.0 != candidate.0
                && other.2.unwrap() <= candidate.2.unwrap()
                && other.3.unwrap() <= candidate.3.unwrap()
                && (other.2.unwrap() < candidate.2.unwrap() || other.3.unwrap() < candidate.3.unwrap()))
        }).map(|x| x.0.clone()).collect();
        frontier.sort();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        (results, frontier)
    }

    #[test]
    fn frozen_corpus_matches_expected_frontier_and_receipt_contract() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("a".repeat(40)), Some(embedded()), None);
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.scenario_count, 5);
        assert_eq!(receipt.policy_count, 6);
        assert_eq!(receipt.operation_action_count, 4);
        assert_eq!(receipt.feasible_policy_count, 3);
        assert_eq!(receipt.frontier, vec!["inspect_then_adapt".to_string(), "visible_context_adaptive".to_string()]);
        assert!(receipt.all_plan_only);
        assert_eq!(receipt.claim_ceiling, "NO_ACTUATION_AUTHORIZED");
        assert_eq!(receipt.corpus_sha256.len(), 64);
    }

    #[test]
    fn second_profile_pv_corpus_transfers_without_reusing_forest_thresholds() {
        let receipt = make_receipt(PV_CORPUS.as_bytes(), Some("c".repeat(40)), Some(embedded_pv()), None);
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.profile_id, "photovoltaic_second_life_synthetic_v0.1");
        assert_eq!(receipt.scenario_count, 3);
        assert_eq!(receipt.policy_count, 3);
        assert_eq!(receipt.operation_action_count, 3);
        assert_eq!(receipt.feasible_policy_count, 2);
        assert_eq!(receipt.frontier, vec!["inspect_then_route".to_string(), "recycle_only".to_string()]);
        let inspection = receipt.policy_results.iter().find(|p| p.policy_id == "inspect_then_route").unwrap();
        assert_eq!(inspection.total_unweighted_scenario_cost_cents, Some(550));
        assert_eq!(inspection.worst_case_regret_cents, Some(60));
    }

    #[test]
    fn pv_profile_also_matches_independent_scenario_oracle() {
        let corpus = embedded_pv();
        let (independent, frontier) = independent_oracle(&corpus);
        let receipt = audit(corpus);
        let actual: Vec<(String, bool, Option<i64>, Option<i64>)> = receipt.policy_results.iter().map(|r|
            (r.policy_id.clone(), r.feasible_across_scenario_set, r.total_unweighted_scenario_cost_cents, r.worst_case_regret_cents)
        ).collect();
        assert_eq!(actual, independent);
        assert_eq!(receipt.frontier, frontier);
    }

    #[test]
    fn scenario_results_match_independent_brute_force_oracle() {
        let corpus = embedded();
        let (independent, frontier) = independent_oracle(&corpus);
        let receipt = audit(corpus);
        let actual: Vec<(String, bool, Option<i64>, Option<i64>)> = receipt.policy_results.iter().map(|r|
            (r.policy_id.clone(), r.feasible_across_scenario_set, r.total_unweighted_scenario_cost_cents, r.worst_case_regret_cents)
        ).collect();
        assert_eq!(actual, independent);
        assert_eq!(receipt.frontier, frontier);
    }

    #[test]
    fn observation_adaptive_policy_beats_retain_only_baseline_in_metrics() {
        let receipt = audit(embedded());
        let adaptive = receipt.policy_results.iter().find(|p| p.policy_id == "visible_context_adaptive").unwrap();
        let retain = receipt.policy_results.iter().find(|p| p.policy_id == "retain_all").unwrap();
        assert_eq!(adaptive.total_unweighted_scenario_cost_cents, Some(900));
        assert_eq!(adaptive.worst_case_regret_cents, Some(70));
        assert_eq!(retain.total_unweighted_scenario_cost_cents, Some(1500));
        assert_eq!(retain.worst_case_regret_cents, Some(200));
    }

    #[test]
    fn inspection_policy_pays_once_and_reduces_worst_regret() {
        let receipt = audit(embedded());
        let inspected = receipt.policy_results.iter().find(|p| p.policy_id == "inspect_then_adapt").unwrap();
        assert_eq!(inspected.total_unweighted_scenario_cost_cents, Some(910));
        assert_eq!(inspected.worst_case_regret_cents, Some(40));
        let low = inspected.outcomes.iter().find(|o| o.scenario_id == "S01_ACCESS_CLEAR_LOW_HANDLING").unwrap();
        let high = inspected.outcomes.iter().find(|o| o.scenario_id == "S02_ACCESS_CLEAR_HIGH_HANDLING").unwrap();
        assert_eq!(low.inspection_cost_cents, 40);
        assert_eq!(high.inspection_cost_cents, 40);
        assert_eq!(low.selected_action.as_deref(), Some("ground"));
        assert_eq!(high.selected_action.as_deref(), Some("cable"));
    }

    #[test]
    fn fixed_ground_policy_fails_closed_on_wet_ground_and_habitat_gate() {
        let receipt = audit(embedded());
        let policy = receipt.policy_results.iter().find(|p| p.policy_id == "fixed_ground").unwrap();
        assert!(!policy.feasible_across_scenario_set);
        assert_eq!(policy.ineligible_scenario_ids, vec![
            "S04_WET_GROUND_CABLE_ONLY".to_string(),
            "S05_HABITAT_RETENTION_REQUIRED".to_string()
        ]);
        assert_eq!(policy.total_unweighted_scenario_cost_cents, None);
    }

    #[test]
    fn unknown_gate_is_not_accepted_as_passing_action() {
        let mut corpus = embedded();
        corpus["scenarios"][0]["action_options"]["ground"]["gate_status"] = json!("UNKNOWN");
        corpus["scenarios"][0]["action_options"]["ground"]["reason_code"] = json!("TEST_UNKNOWN");
        let receipt = audit(corpus);
        assert!(!receipt.policy_results.iter().find(|p| p.policy_id == "visible_context_adaptive").unwrap().feasible_across_scenario_set);
    }

    #[test]
    fn scenario_specific_override_is_rejected_as_anticipative() {
        let mut corpus = embedded();
        corpus["policies"][4]["scenario_overrides"] = json!({
            "S01_ACCESS_CLEAR_LOW_HANDLING":"ground",
            "S02_ACCESS_CLEAR_HIGH_HANDLING":"cable"
        });
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SCENARIO_SPECIFIC_OVERRIDE_FORBIDDEN")));
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let mut corpus = embedded();
        corpus["scenarios"][1]["id"] = corpus["scenarios"][0]["id"].clone();
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("DUPLICATE_SCENARIO_ID")));
    }

    #[test]
    fn duplicate_policy_ids_are_rejected() {
        let mut corpus = embedded();
        corpus["policies"][1]["id"] = corpus["policies"][0]["id"].clone();
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("DUPLICATE_POLICY_ID")));
    }

    #[test]
    fn unknown_initial_observation_rule_is_rejected() {
        let mut corpus = embedded();
        corpus["policies"][4]["initial_decisions"]["invisible_future_state"] = json!("ground");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("UNKNOWN_OBSERVATION_RULE")));
    }

    #[test]
    fn missing_post_inspection_branch_is_rejected() {
        let mut corpus = embedded();
        corpus["policies"][5]["post_inspection_decisions"].as_object_mut().unwrap().remove("handling_high");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("POST_INSPECTION_DECISIONS_MISSING_OBSERVATION_RULE")));
    }

    #[test]
    fn post_inspection_rules_without_inspection_are_rejected() {
        let mut corpus = embedded();
        corpus["policies"][3]["post_inspection_decisions"] = json!({"handling_low":"ground"});
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("POST_INSPECTION_RULES_WITHOUT_INSPECTION")));
    }

    #[test]
    fn negative_inspection_cost_is_rejected() {
        let mut corpus = embedded();
        corpus["policies"][5]["inspection_cost_cents"] = json!(-1);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("INSPECTION_COST_INVALID")));
    }

    #[test]
    fn action_cost_must_be_nonnegative_integer() {
        let mut corpus = embedded();
        corpus["scenarios"][0]["action_options"]["ground"]["cost_cents"] = json!(-10);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("ACTION_COST_INVALID")));
    }

    #[test]
    fn wrong_profile_is_rejected() {
        let mut corpus = embedded();
        corpus["policies"][0]["domain_profile_id"] = json!("other-profile");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("POLICY_PROFILE_MISMATCH")));
    }

    #[test]
    fn wrong_expected_frontier_is_rejected() {
        let mut corpus = embedded();
        corpus["expected_frontier"] = json!(["retain_all"]);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("EXPECTED_FRONTIER_MISMATCH")));
    }

    #[test]
    fn input_policy_order_does_not_change_frontier() {
        let mut corpus = embedded();
        corpus["policies"].as_array_mut().unwrap().reverse();
        let receipt = audit(corpus);
        assert_eq!(receipt.frontier, vec!["inspect_then_adapt".to_string(), "visible_context_adaptive".to_string()]);
    }

    #[test]
    fn scenario_order_permutation_preserves_policy_metrics_and_frontier() {
        let baseline = audit(embedded());
        let mut corpus = embedded();
        corpus["scenarios"].as_array_mut().unwrap().reverse();
        let permuted = audit(corpus);
        assert_eq!(baseline.frontier, permuted.frontier);
        assert_eq!(baseline.feasible_policy_count, permuted.feasible_policy_count);
        for (a, b) in baseline.policy_results.iter().zip(permuted.policy_results.iter()) {
            assert_eq!(a.policy_id, b.policy_id);
            assert_eq!(a.feasible_across_scenario_set, b.feasible_across_scenario_set);
            assert_eq!(a.total_unweighted_scenario_cost_cents, b.total_unweighted_scenario_cost_cents);
            assert_eq!(a.worst_case_regret_cents, b.worst_case_regret_cents);
            assert_eq!(a.total_unweighted_regret_cents, b.total_unweighted_regret_cents);
        }
    }

    #[test]
    fn inspection_does_not_clear_an_independent_operation_gate() {
        let mut corpus = embedded();
        corpus["scenarios"][1]["action_options"]["cable"]["gate_status"] = json!("BLOCK");
        corpus["scenarios"][1]["action_options"]["cable"]["reason_code"] = json!("INDEPENDENT_TEST_BLOCK");
        let receipt = audit(corpus);
        let policy = receipt.policy_results.iter().find(|p| p.policy_id == "inspect_then_adapt").unwrap();
        assert!(!policy.feasible_across_scenario_set);
        let outcome = policy.outcomes.iter().find(|o| o.scenario_id == "S02_ACCESS_CLEAR_HIGH_HANDLING").unwrap();
        assert_eq!(outcome.outcome, "INELIGIBLE_BLOCK");
        assert_eq!(outcome.reason_code.as_deref(), Some("INDEPENDENT_TEST_BLOCK"));
    }

    #[test]
    fn scenario_probabilities_are_not_silently_admitted() {
        let mut corpus = embedded();
        corpus["scenario_semantics"]["probabilities_supplied"] = json!(true);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("SCENARIO_PROBABILITIES_MUST_NOT_BE_FABRICATED")));
    }

    #[test]
    fn bad_source_commit_is_rejected() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("not-a-commit".to_string()), Some(embedded()), None);
        assert!(receipt.errors.iter().any(|e| e.contains("SOURCE_COMMIT_INVALID")));
    }

    #[test]
    fn no_policy_can_change_action_between_indistinguishable_states_without_inspection() {
        let mut corpus = embedded();
        // Try to encode hidden-scenario-dependent choices in a side channel;
        // the policy schema forbids scenario IDs as a decision key.
        corpus["policies"][4]["initial_decisions"] = json!({
            "S01_ACCESS_CLEAR_LOW_HANDLING":"ground",
            "S02_ACCESS_CLEAR_HIGH_HANDLING":"cable",
            "remote_rough":"cable",
            "cable_only":"cable",
            "habitat_sensitive":"retain"
        });
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|e| e.contains("MISSING_OBSERVATION_RULE")));
        assert!(receipt.errors.iter().any(|e| e.contains("UNKNOWN_OBSERVATION_RULE")));
    }
}
