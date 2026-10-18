//! Adaptive, probability-free measurement policy tree qualification.
//!
//! This is an independent finite-state oracle for conditional measurement plans.
//! It tests branching, stopping, and worst-case budgets; it does not estimate expected VoI.

use crate::sha256_hex;
use jsonschema::validator_for;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub const EMBEDDED_ADAPTIVE_POLICY_CORPUS: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_ADAPTIVE_POLICIES_V0_1.json"
);
pub const EMBEDDED_ADAPTIVE_POLICY_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_ADAPTIVE_POLICIES_V0_1.schema.json"
);

const EXPECTED_ADAPTIVE_CASE_COUNT: usize = 3;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AdaptivePolicyFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdaptivePolicySummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub corpus_sha256: String,
    pub schema_sha256: String,
    pub cases_seen: usize,
    pub cases_passed: usize,
    pub policies_seen: usize,
    pub policies_matched: usize,
    pub passed: bool,
    pub results: Vec<Value>,
    pub failures: Vec<AdaptivePolicyFailure>,
}

impl AdaptivePolicySummary {
    fn new(corpus_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            corpus_sha256: sha256_hex(corpus_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            cases_seen: 0,
            cases_passed: 0,
            policies_seen: 0,
            policies_matched: 0,
            passed: false,
            results: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(AdaptivePolicyFailure {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjectiveVector {
    service_hours: u64,
    energy_units: u64,
    repair_ticks: u64,
    cost_units: u64,
}

#[derive(Debug, Clone)]
struct StateResult {
    feasible: bool,
    objectives: ObjectiveVector,
}

#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    states: BTreeMap<String, StateResult>,
}

#[derive(Debug, Clone)]
struct MeasurementOutcome {
    id: String,
    states: Vec<String>,
}

#[derive(Debug, Clone)]
struct MeasurementAction {
    id: String,
    evidence_state: String,
    source_revision: String,
    cost_units: u64,
    duration_ticks: u64,
    cost_basis_id: String,
    duration_basis_id: String,
    resolves_unknown_ids: Vec<String>,
    outcomes: Vec<MeasurementOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecisionSet {
    remaining_critical_unknown_ids: Vec<String>,
    robust_feasible_candidate_ids: Vec<String>,
    pareto_candidate_ids: Vec<String>,
}

fn sorted(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values: Vec<String> = values.into_iter().collect();
    values.sort();
    values.dedup();
    values
}

fn strings(value: &Value) -> Option<Vec<String>> {
    value
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(ToOwned::to_owned))
        .collect()
}

fn parse_candidates(case: &Value, declared_states: &[String]) -> Option<Vec<Candidate>> {
    let mut candidates = Vec::new();
    let mut candidate_ids = BTreeSet::new();
    for candidate_value in case.get("candidates")?.as_array()? {
        let candidate_id = candidate_value.get("candidate_id")?.as_str()?.to_owned();
        if !candidate_ids.insert(candidate_id.clone()) {
            return None;
        }
        let mut state_results = BTreeMap::new();
        for result in candidate_value.get("state_results")?.as_array()? {
            let state_id = result.get("state_id")?.as_str()?.to_owned();
            if !declared_states.contains(&state_id) {
                return None;
            }
            let objective = result.get("objectives")?;
            let state_result = StateResult {
                feasible: result.get("feasible")?.as_bool()?,
                objectives: ObjectiveVector {
                    service_hours: objective.get("service_hours")?.as_u64()?,
                    energy_units: objective.get("energy_units")?.as_u64()?,
                    repair_ticks: objective.get("repair_ticks")?.as_u64()?,
                    cost_units: objective.get("cost_units")?.as_u64()?,
                },
            };
            if state_results.insert(state_id, state_result).is_some() {
                return None;
            }
        }
        let found: BTreeSet<&str> = state_results.keys().map(String::as_str).collect();
        let expected: BTreeSet<&str> = declared_states.iter().map(String::as_str).collect();
        if found != expected {
            return None;
        }
        candidates.push(Candidate {
            id: candidate_id,
            states: state_results,
        });
    }
    Some(candidates)
}

fn parse_actions(case: &Value) -> Option<BTreeMap<String, MeasurementAction>> {
    let mut actions = BTreeMap::new();
    for action_value in case.get("actions")?.as_array()? {
        let id = action_value.get("action_id")?.as_str()?.to_owned();
        let outcomes: Option<Vec<MeasurementOutcome>> = action_value
            .get("outcomes")?
            .as_array()?
            .iter()
            .map(|outcome| {
                Some(MeasurementOutcome {
                    id: outcome.get("outcome_id")?.as_str()?.to_owned(),
                    states: strings(outcome.get("compatible_state_ids")?)?,
                })
            })
            .collect();
        let action = MeasurementAction {
            id: id.clone(),
            evidence_state: action_value.get("evidence_state")?.as_str()?.to_owned(),
            source_revision: action_value.get("source_revision")?.as_str()?.to_owned(),
            cost_units: action_value.get("estimated_cost_units")?.as_u64()?,
            duration_ticks: action_value.get("estimated_duration_ticks")?.as_u64()?,
            cost_basis_id: action_value.get("cost_basis_id")?.as_str()?.to_owned(),
            duration_basis_id: action_value.get("duration_basis_id")?.as_str()?.to_owned(),
            resolves_unknown_ids: strings(action_value.get("resolves_unknown_ids")?)?,
            outcomes: outcomes?,
        };
        if actions.insert(id, action).is_some() {
            return None;
        }
    }
    Some(actions)
}

fn vector_for(candidate: &Candidate, states: &[String]) -> Option<ObjectiveVector> {
    let values: Vec<&StateResult> = states
        .iter()
        .map(|state| candidate.states.get(state))
        .collect::<Option<Vec<_>>>()?;
    Some(ObjectiveVector {
        service_hours: values.iter().map(|value| value.objectives.service_hours).min()?,
        energy_units: values.iter().map(|value| value.objectives.energy_units).max()?,
        repair_ticks: values.iter().map(|value| value.objectives.repair_ticks).max()?,
        cost_units: values.iter().map(|value| value.objectives.cost_units).max()?,
    })
}

fn dominates(left: ObjectiveVector, right: ObjectiveVector) -> bool {
    let no_worse = left.service_hours >= right.service_hours
        && left.energy_units <= right.energy_units
        && left.repair_ticks <= right.repair_ticks
        && left.cost_units <= right.cost_units;
    let strictly_better = left.service_hours > right.service_hours
        || left.energy_units < right.energy_units
        || left.repair_ticks < right.repair_ticks
        || left.cost_units < right.cost_units;
    no_worse && strictly_better
}

fn decision_set(
    candidates: &[Candidate],
    states: &[String],
    blockers: &[String],
) -> DecisionSet {
    let blockers = sorted(blockers.to_vec());
    if !blockers.is_empty() {
        return DecisionSet {
            remaining_critical_unknown_ids: blockers,
            robust_feasible_candidate_ids: Vec::new(),
            pareto_candidate_ids: Vec::new(),
        };
    }

    let feasible: Vec<&Candidate> = candidates
        .iter()
        .filter(|candidate| {
            states.iter().all(|state| {
                candidate
                    .states
                    .get(state)
                    .is_some_and(|result| result.feasible)
            })
        })
        .collect();
    let ids = sorted(feasible.iter().map(|candidate| candidate.id.clone()));
    let pareto = sorted(feasible.iter().filter_map(|candidate| {
        let right = vector_for(candidate, states)?;
        let beaten = feasible.iter().any(|other| {
            other.id != candidate.id
                && vector_for(other, states).is_some_and(|left| dominates(left, right))
        });
        (!beaten).then(|| candidate.id.clone())
    }));
    DecisionSet {
        remaining_critical_unknown_ids: blockers,
        robust_feasible_candidate_ids: ids,
        pareto_candidate_ids: pareto,
    }
}

fn decision_is_stable(candidates: &[Candidate], states: &[String], current: &DecisionSet) -> bool {
    if states.is_empty() || !current.remaining_critical_unknown_ids.is_empty() {
        return false;
    }
    states.iter().all(|state| {
        let one_state = decision_set(candidates, std::slice::from_ref(state), &[]);
        one_state.robust_feasible_candidate_ids == current.robust_feasible_candidate_ids
            && one_state.pareto_candidate_ids == current.pareto_candidate_ids
    })
}

fn action_rejection_codes(
    action: &MeasurementAction,
    scenario_revision: &str,
    universe: &[String],
    current_blockers: &[String],
) -> BTreeSet<String> {
    let mut codes = BTreeSet::new();
    if action.evidence_state != "valid" {
        codes.insert("POLICY_ACTION_EVIDENCE_NOT_VALID".to_owned());
    }
    if action.source_revision != scenario_revision {
        codes.insert("POLICY_ACTION_REVISION_MISMATCH".to_owned());
    }
    if action.cost_basis_id.trim().is_empty() || action.duration_basis_id.trim().is_empty() {
        codes.insert("POLICY_ACTION_COST_TIME_BASIS_MISSING".to_owned());
    }
    for unknown in &action.resolves_unknown_ids {
        if !current_blockers.contains(unknown) {
            codes.insert("POLICY_ACTION_UNKNOWN_NOT_PRESENT".to_owned());
        }
    }

    let mut memberships = BTreeMap::<String, String>::new();
    let mut outcome_ids = BTreeSet::new();
    for outcome in &action.outcomes {
        if outcome.id.trim().is_empty() || !outcome_ids.insert(outcome.id.clone()) {
            codes.insert("POLICY_ACTION_OUTCOME_ID_DUPLICATE".to_owned());
        }
        if outcome.states.is_empty() {
            codes.insert("POLICY_ACTION_OUTCOME_STATES_EMPTY".to_owned());
        }
        let mut within_outcome = BTreeSet::new();
        for state in &outcome.states {
            if !universe.contains(state) {
                codes.insert("POLICY_ACTION_STATE_UNDECLARED".to_owned());
            }
            if !within_outcome.insert(state.clone()) {
                codes.insert("POLICY_ACTION_STATE_DUPLICATE_WITHIN_OUTCOME".to_owned());
            }
            if memberships.insert(state.clone(), outcome.id.clone()).is_some() {
                codes.insert("POLICY_ACTION_PARTITION_OVERLAP".to_owned());
            }
        }
    }
    if universe.iter().any(|state| !memberships.contains_key(state)) {
        codes.insert("POLICY_ACTION_PARTITION_INCOMPLETE".to_owned());
    }
    codes
}

#[derive(Debug)]
struct PolicyWalker<'a> {
    revision: &'a str,
    universe: &'a [String],
    candidates: &'a [Candidate],
    actions: &'a BTreeMap<String, MeasurementAction>,
    nodes: BTreeMap<String, Value>,
    policy_id: String,
    max_depth: usize,
    max_actions: usize,
    max_cost_units: u64,
    max_duration_ticks: u64,
    errors: BTreeSet<String>,
    visited: BTreeSet<String>,
    active: BTreeSet<String>,
    leaves: Vec<Value>,
    worst_cost: u64,
    worst_duration: u64,
    worst_actions: usize,
    deepest_actions: usize,
}

impl<'a> PolicyWalker<'a> {
    fn new(
        revision: &'a str,
        universe: &'a [String],
        candidates: &'a [Candidate],
        actions: &'a BTreeMap<String, MeasurementAction>,
        policy: &Value,
    ) -> Self {
        let mut nodes = BTreeMap::new();
        let mut errors = BTreeSet::new();
        for node in policy["nodes"].as_array().into_iter().flatten() {
            let Some(id) = node.get("node_id").and_then(Value::as_str) else {
                errors.insert("POLICY_NODE_ID_MISSING".to_owned());
                continue;
            };
            if nodes.insert(id.to_owned(), node.clone()).is_some() {
                errors.insert("POLICY_NODE_ID_DUPLICATE".to_owned());
            }
        }
        Self {
            revision,
            universe,
            candidates,
            actions,
            nodes,
            policy_id: policy["policy_id"].as_str().unwrap_or("").to_owned(),
            max_depth: policy["max_depth"].as_u64().unwrap_or(0) as usize,
            max_actions: policy["max_actions"].as_u64().unwrap_or(0) as usize,
            max_cost_units: policy["max_cost_units"].as_u64().unwrap_or(0),
            max_duration_ticks: policy["max_duration_ticks"].as_u64().unwrap_or(0),
            errors,
            visited: BTreeSet::new(),
            active: BTreeSet::new(),
            leaves: Vec::new(),
            worst_cost: 0,
            worst_duration: 0,
            worst_actions: 0,
            deepest_actions: 0,
        }
    }

    fn visit(
        &mut self,
        node_id: &str,
        states: Vec<String>,
        blockers: Vec<String>,
        path_cost: u64,
        path_duration: u64,
        actions_used: usize,
    ) {
        if !self.errors.is_empty() {
            return;
        }
        if self.active.contains(node_id) {
            self.errors.insert("POLICY_CYCLE".to_owned());
            return;
        }
        if self.visited.contains(node_id) {
            self.errors.insert("POLICY_NODE_REUSED".to_owned());
            return;
        }
        let Some(node) = self.nodes.get(node_id).cloned() else {
            self.errors.insert("POLICY_NODE_UNDECLARED".to_owned());
            return;
        };
        self.visited.insert(node_id.to_owned());
        self.active.insert(node_id.to_owned());

        let node_type = node.get("node_type").and_then(Value::as_str).unwrap_or("");
        match node_type {
            "terminal" => {
                let terminal_action = node
                    .get("terminal_action")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let blockers = sorted(blockers);
                let decision = decision_set(self.candidates, &states, &blockers);
                match terminal_action {
                    "stop_decision_stable" => {
                        if !blockers.is_empty() {
                            self.errors
                                .insert("POLICY_STOP_WITH_CRITICAL_BLOCKER".to_owned());
                        } else if decision.robust_feasible_candidate_ids.is_empty() {
                            self.errors
                                .insert("POLICY_STOP_WITHOUT_ROBUST_CANDIDATE".to_owned());
                        } else if !decision_is_stable(self.candidates, &states, &decision) {
                            self.errors
                                .insert("POLICY_STOP_DECISION_NOT_STABLE".to_owned());
                        }
                    }
                    "reopen_design" => {
                        if !blockers.is_empty() {
                            self.errors
                                .insert("POLICY_REOPEN_WITH_CRITICAL_BLOCKER".to_owned());
                        } else if !decision.robust_feasible_candidate_ids.is_empty() {
                            self.errors
                                .insert("POLICY_REOPEN_WITH_ROBUST_CANDIDATE".to_owned());
                        }
                    }
                    "escalate_blocker" => {
                        if blockers.is_empty() {
                            self.errors
                                .insert("POLICY_ESCALATE_WITHOUT_BLOCKER".to_owned());
                        }
                    }
                    _ => {
                        self.errors
                            .insert("POLICY_TERMINAL_ACTION_UNKNOWN".to_owned());
                    }
                }
                self.leaves.push(json!({
                    "leaf_node_id": node_id,
                    "terminal_action": terminal_action,
                    "compatible_state_ids": sorted(states),
                    "remaining_critical_unknown_ids": decision.remaining_critical_unknown_ids,
                    "robust_feasible_candidate_ids": decision.robust_feasible_candidate_ids,
                    "pareto_candidate_ids": decision.pareto_candidate_ids,
                    "cumulative_cost_units": path_cost,
                    "cumulative_duration_ticks": path_duration
                }));
                self.worst_cost = self.worst_cost.max(path_cost);
                self.worst_duration = self.worst_duration.max(path_duration);
                self.worst_actions = self.worst_actions.max(actions_used);
            }
            "measure" => {
                let action_id = node.get("action_id").and_then(Value::as_str).unwrap_or("");
                let Some(action) = self.actions.get(action_id).cloned() else {
                    self.errors.insert("POLICY_ACTION_UNDECLARED".to_owned());
                    self.active.remove(node_id);
                    return;
                };
                let action_codes = action_rejection_codes(
                    &action,
                    self.revision,
                    self.universe,
                    &blockers,
                );
                self.errors.extend(action_codes);
                if !self.errors.is_empty() {
                    self.active.remove(node_id);
                    return;
                }

                let reachable: BTreeMap<String, Vec<String>> = action
                    .outcomes
                    .iter()
                    .filter_map(|outcome| {
                        let subset = sorted(
                            outcome
                                .states
                                .iter()
                                .filter(|state| states.contains(state))
                                .cloned(),
                        );
                        (!subset.is_empty()).then(|| (outcome.id.clone(), subset))
                    })
                    .collect();
                let Some(branches) = node.get("branches").and_then(Value::as_array) else {
                    self.errors.insert("POLICY_BRANCHES_MISSING".to_owned());
                    self.active.remove(node_id);
                    return;
                };
                let mut branch_map = BTreeMap::new();
                for branch in branches {
                    let branch_id = branch["outcome_id"].as_str().unwrap_or("").to_owned();
                    let next_id = branch["next_node_id"].as_str().unwrap_or("").to_owned();
                    if branch_map.insert(branch_id, next_id).is_some() {
                        self.errors.insert("POLICY_BRANCH_ID_DUPLICATE".to_owned());
                    }
                }
                for outcome_id in reachable.keys() {
                    if !branch_map.contains_key(outcome_id) {
                        self.errors.insert("POLICY_BRANCH_MISSING".to_owned());
                    }
                }
                for outcome_id in branch_map.keys() {
                    if !reachable.contains_key(outcome_id) {
                        self.errors
                            .insert("POLICY_BRANCH_UNREACHABLE_OUTCOME".to_owned());
                    }
                }
                if !self.errors.is_empty() {
                    self.active.remove(node_id);
                    return;
                }

                let blockers_after: Vec<String> = blockers
                    .into_iter()
                    .filter(|unknown| !action.resolves_unknown_ids.contains(unknown))
                    .collect();
                let Some(next_cost) = path_cost.checked_add(action.cost_units) else {
                    self.errors.insert("POLICY_COST_OVERFLOW".to_owned());
                    self.active.remove(node_id);
                    return;
                };
                let Some(next_duration) = path_duration.checked_add(action.duration_ticks) else {
                    self.errors.insert("POLICY_DURATION_OVERFLOW".to_owned());
                    self.active.remove(node_id);
                    return;
                };
                let next_actions = actions_used + 1;
                self.deepest_actions = self.deepest_actions.max(next_actions);
                if next_actions > self.max_actions {
                    self.errors
                        .insert("POLICY_ACTION_BUDGET_EXCEEDED".to_owned());
                }
                if next_actions > self.max_depth {
                    self.errors
                        .insert("POLICY_DEPTH_BUDGET_EXCEEDED".to_owned());
                }
                if next_cost > self.max_cost_units {
                    self.errors
                        .insert("POLICY_COST_BUDGET_EXCEEDED".to_owned());
                }
                if next_duration > self.max_duration_ticks {
                    self.errors
                        .insert("POLICY_DURATION_BUDGET_EXCEEDED".to_owned());
                }
                if !self.errors.is_empty() {
                    self.active.remove(node_id);
                    return;
                }

                for (outcome_id, subset) in reachable {
                    let next_node = branch_map.get(&outcome_id).cloned().unwrap_or_default();
                    self.visit(
                        &next_node,
                        subset,
                        blockers_after.clone(),
                        next_cost,
                        next_duration,
                        next_actions,
                    );
                    if !self.errors.is_empty() {
                        break;
                    }
                }
            }
            _ => {
                self.errors.insert("POLICY_NODE_TYPE_UNKNOWN".to_owned());
            }
        }
        self.active.remove(node_id);
    }

    fn finish(
        mut self,
        root_node_id: &str,
        initial_blockers: Vec<String>,
    ) -> Value {
        let initial_states = self.universe.to_vec();
        self.visit(
            root_node_id,
            initial_states,
            initial_blockers,
            0,
            0,
            0,
        );
        if self.visited.len() != self.nodes.len() {
            self.errors.insert("POLICY_UNREACHABLE_NODE".to_owned());
        }
        let rejection_codes: Vec<String> = self.errors.into_iter().collect();
        let rejected = !rejection_codes.is_empty();
        self.leaves.sort_by(|left, right| {
            left["leaf_node_id"]
                .as_str()
                .cmp(&right["leaf_node_id"].as_str())
        });
        json!({
            "policy_id": self.policy_id,
            "status": if rejected { "rejected" } else { "accepted" },
            "rejection_codes": rejection_codes,
            "worst_case_cost_units": if rejected { Value::Null } else { json!(self.worst_cost) },
            "worst_case_duration_ticks": if rejected {
                Value::Null
            } else {
                json!(self.worst_duration)
            },
            "worst_case_action_count": if rejected {
                Value::Null
            } else {
                json!(self.worst_actions)
            },
            "leaf_count": if rejected { Value::Null } else { json!(self.leaves.len()) },
            "leaves": if rejected { Vec::<Value>::new() } else { self.leaves }
        })
    }
}

fn evaluate_policy(
    case: &Value,
    policy: &Value,
    universe: &[String],
    candidates: &[Candidate],
    actions: &BTreeMap<String, MeasurementAction>,
) -> Value {
    let revision = case["state_revision"].as_str().unwrap_or("");
    let mut walker = PolicyWalker::new(revision, universe, candidates, actions, policy);
    let root = policy["root_node_id"].as_str().unwrap_or("");
    let blockers = strings(&case["unresolved_critical_unknown_ids"]).unwrap_or_default();
    walker.finish(root, blockers)
}

pub fn validate_adaptive_policy_bytes(
    corpus_bytes: &[u8],
    schema_bytes: &[u8],
) -> AdaptivePolicySummary {
    let mut summary = AdaptivePolicySummary::new(corpus_bytes, schema_bytes);
    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("ADAPTIVE_SCHEMA_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let corpus_value: Value = match serde_json::from_slice(corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("ADAPTIVE_CORPUS_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            summary.fail("ADAPTIVE_SCHEMA_COMPILE_FAILED", "$", error.to_string());
            return summary;
        }
    };
    let schema_errors: Vec<_> = validator.iter_errors(&corpus_value).collect();
    if !schema_errors.is_empty() {
        for error in schema_errors {
            summary.fail(
                "ADAPTIVE_SCHEMA_VALIDATION_FAILED",
                error.instance_path().to_string(),
                error.to_string(),
            );
        }
        return summary;
    }

    let Some(root) = corpus_value.as_object() else {
        summary.fail("ADAPTIVE_CORPUS_ROOT_INVALID", "$", "Expected an object.");
        return summary;
    };
    summary.benchmark_id = root
        .get("benchmark_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    summary.version = root.get("version").and_then(Value::as_str).map(ToOwned::to_owned);
    if summary.benchmark_id.as_deref() != Some("SPATIAL-RESILIENCE-ADAPTIVE-POLICIES")
        || summary.version.as_deref() != Some("0.1.0")
    {
        summary.fail(
            "ADAPTIVE_CORPUS_ID_OR_VERSION_MISMATCH",
            "$",
            "This verifier freezes adaptive policy corpus v0.1.0.",
        );
        return summary;
    }

    let Some(cases) = root.get("cases").and_then(Value::as_array) else {
        summary.fail("ADAPTIVE_CASES_MISSING", "$.cases", "Expected cases array.");
        return summary;
    };
    summary.cases_seen = cases.len();
    if cases.len() != EXPECTED_ADAPTIVE_CASE_COUNT {
        summary.fail(
            "ADAPTIVE_CASE_COUNT_MISMATCH",
            "$.cases",
            format!("Expected {EXPECTED_ADAPTIVE_CASE_COUNT} cases."),
        );
        return summary;
    }

    let mut case_ids = BTreeSet::new();
    for (case_index, case) in cases.iter().enumerate() {
        let path = format!("$.cases[{case_index}]");
        let case_id = case.get("id").and_then(Value::as_str).unwrap_or("");
        let expected_case_id = format!("SPA-{:03}", case_index + 1);
        if case_id != expected_case_id {
            summary.fail(
                "ADAPTIVE_CASE_ID_SEQUENCE",
                format!("{path}.id"),
                format!("Expected {expected_case_id:?}, got {case_id:?}."),
            );
        }
        if !case_ids.insert(case_id.to_owned()) {
            summary.fail(
                "ADAPTIVE_CASE_ID_DUPLICATE",
                format!("{path}.id"),
                format!("Case ID {case_id:?} is duplicated."),
            );
            continue;
        }

        let Some(state_ids) = strings(&case["state_ids"]) else {
            summary.fail(
                "ADAPTIVE_STATE_IDS_INVALID",
                format!("{path}.state_ids"),
                "State IDs must be a string array.",
            );
            continue;
        };
        let current_states = strings(&case["current_possible_state_ids"]).unwrap_or_default();
        let blockers = strings(&case["unresolved_critical_unknown_ids"]).unwrap_or_default();
        let mut semantic_valid = true;
        let declared: BTreeSet<&str> = state_ids.iter().map(String::as_str).collect();
        for state in &current_states {
            if !declared.contains(state.as_str()) {
                summary.fail(
                    "ADAPTIVE_CURRENT_STATE_UNDECLARED",
                    format!("{path}.current_possible_state_ids"),
                    format!("Current state {state:?} is undeclared."),
                );
                semantic_valid = false;
            }
        }
        if state_ids.is_empty() || current_states.is_empty() {
            summary.fail(
                "ADAPTIVE_STATE_SET_EMPTY",
                &path,
                "Declared and current state sets must be non-empty.",
            );
            semantic_valid = false;
        }
        if sorted(state_ids.clone()).len() != state_ids.len()
            || sorted(current_states.clone()).len() != current_states.len()
            || sorted(blockers.clone()).len() != blockers.len()
        {
            summary.fail(
                "ADAPTIVE_STATE_ID_DUPLICATE",
                &path,
                "State and blocker ID arrays must be unique.",
            );
            semantic_valid = false;
        }
        let Some(candidates) = parse_candidates(case, &state_ids) else {
            summary.fail(
                "ADAPTIVE_CANDIDATE_STATE_COVERAGE",
                format!("{path}.candidates"),
                "Candidate IDs must be unique and each candidate must define one result per state.",
            );
            continue;
        };
        let Some(actions) = parse_actions(case) else {
            summary.fail(
                "ADAPTIVE_ACTION_SCHEMA_INVALID",
                format!("{path}.actions"),
                "Action IDs must be unique and action fields must be complete.",
            );
            continue;
        };
        if !semantic_valid {
            continue;
        }

        let policy_values = case["policies"].as_array().expect("schema-validated policy array");
        let mut policy_ids = BTreeSet::new();
        let before_case_failures = summary.failures.len();
        for (policy_index, policy) in policy_values.iter().enumerate() {
            summary.policies_seen += 1;
            let policy_path = format!("{path}.policies[{policy_index}]");
            let policy_id = policy["policy_id"].as_str().unwrap_or("");
            if !policy_ids.insert(policy_id.to_owned()) {
                summary.fail(
                    "ADAPTIVE_POLICY_ID_DUPLICATE",
                    format!("{policy_path}.policy_id"),
                    format!("Policy ID {policy_id:?} is duplicated."),
                );
                continue;
            }
            let actual = evaluate_policy(case, policy, &current_states, &candidates, &actions);
            if actual != policy["expected"] {
                summary.fail(
                    "ADAPTIVE_EXPECTATION_MISMATCH",
                    &policy_path,
                    format!(
                        "Policy {:?}: expected {:?}, computed {:?}.",
                        policy_id, policy["expected"], actual
                    ),
                );
            } else {
                summary.policies_matched += 1;
            }
            summary.results.push(json!({
                "case_id": case_id,
                "policy": actual
            }));
        }
        if summary.failures.len() == before_case_failures {
            summary.cases_passed += 1;
        }
    }

    summary.failures.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.results.sort_by(|left, right| {
        (&left["case_id"], &left["policy"]["policy_id"])
            .cmp(&(&right["case_id"], &right["policy"]["policy_id"]))
    });
    summary.passed = summary.failures.is_empty()
        && summary.cases_passed == summary.cases_seen
        && summary.policies_matched == summary.policies_seen;
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_embedded() -> AdaptivePolicySummary {
        validate_adaptive_policy_bytes(
            EMBEDDED_ADAPTIVE_POLICY_CORPUS.as_bytes(),
            EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes(),
        )
    }

    fn policy<'a>(
        summary: &'a AdaptivePolicySummary,
        case_id: &str,
        policy_id: &str,
    ) -> &'a Value {
        summary
            .results
            .iter()
            .find(|item| {
                item["case_id"] == case_id && item["policy"]["policy_id"] == policy_id
            })
            .map(|item| &item["policy"])
            .expect("policy result exists")
    }

    #[test]
    fn embedded_adaptive_policy_tree_corpus_matches_known_answers() {
        let summary = run_embedded();
        assert!(
            summary.passed,
            "unexpected failures: {:?}",
            summary.failures
        );
        assert_eq!(summary.cases_seen, 3);
        assert_eq!(summary.cases_passed, 3);
        assert_eq!(summary.policies_seen, 5);
        assert_eq!(summary.policies_matched, 5);
        assert_eq!(summary.corpus_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
    }

    #[test]
    fn adaptive_plan_beats_universal_scan_on_worst_case_cost_and_duration() {
        let summary = run_embedded();
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        let fixed = policy(&summary, "SPA-001", "fixed-universal-scan");
        assert_eq!(adaptive["status"], "accepted");
        assert_eq!(fixed["status"], "accepted");
        assert_eq!(adaptive["worst_case_cost_units"], 5);
        assert_eq!(fixed["worst_case_cost_units"], 10);
        assert_eq!(adaptive["worst_case_duration_ticks"], 3);
        assert_eq!(fixed["worst_case_duration_ticks"], 4);
    }

    #[test]
    fn unresolved_critical_blocker_prevents_decision_stable_leaf() {
        let summary = run_embedded();
        let unsafe_policy = policy(&summary, "SPA-002", "unsafe-stop-before-fire-review");
        let escalation = policy(&summary, "SPA-002", "escalate-fire-review");
        assert_eq!(unsafe_policy["status"], "rejected");
        assert!(unsafe_policy["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_STOP_WITH_CRITICAL_BLOCKER"));
        assert_eq!(escalation["status"], "accepted");
        assert_eq!(escalation["leaves"][0]["terminal_action"], "escalate_blocker");
        assert_eq!(
            escalation["leaves"][0]["remaining_critical_unknown_ids"][0],
            "fire-separation-review"
        );
    }

    #[test]
    fn branch_cannot_reference_outcome_eliminated_by_parent() {
        let summary = run_embedded();
        let invalid = policy(&summary, "SPA-003", "unreachable-child-outcome");
        assert_eq!(invalid["status"], "rejected");
        assert!(invalid["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_BRANCH_UNREACHABLE_OUTCOME"));
    }

    #[test]
    fn probability_fields_are_rejected_by_the_strict_schema() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["policies"][0]["expected_probability"] = Value::from(0.8);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        assert!(!summary.passed);
        assert!(summary
            .failures
            .iter()
            .any(|failure| failure.code == "ADAPTIVE_SCHEMA_VALIDATION_FAILED"));
    }

    #[test]
    fn action_order_and_node_order_do_not_change_policy_results() {
        let original = run_embedded();
        assert!(original.passed, "original failures: {:?}", original.failures);
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        let case = &mut corpus["cases"][0];
        case["actions"].as_array_mut().unwrap().reverse();
        case["candidates"].as_array_mut().unwrap().reverse();
        for candidate in case["candidates"].as_array_mut().unwrap() {
            candidate["state_results"].as_array_mut().unwrap().reverse();
        }
        for action in case["actions"].as_array_mut().unwrap() {
            action["outcomes"].as_array_mut().unwrap().reverse();
        }
        for pol in case["policies"].as_array_mut().unwrap() {
            pol["nodes"].as_array_mut().unwrap().reverse();
            for node in pol["nodes"].as_array_mut().unwrap() {
                if let Some(branches) = node.get_mut("branches").and_then(Value::as_array_mut) {
                    branches.reverse();
                }
            }
        }
        let bytes = serde_json::to_vec(&corpus).expect("serialize reordered corpus");
        let reordered =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        assert!(
            reordered.passed,
            "reordered failures: {:?}",
            reordered.failures
        );
        assert_eq!(original.results, reordered.results);
    }

    #[test]
    fn cost_budget_is_enforced_per_path() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["policies"][0]["max_cost_units"] = Value::from(4);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_COST_BUDGET_EXCEEDED"));
    }

    #[test]
    fn stale_action_revision_blocks_the_policy() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][0]["source_revision"] =
            Value::String("stale-survey-revision".to_owned());
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_ACTION_REVISION_MISMATCH"));
    }

    #[test]
    fn incomplete_action_partition_is_not_accepted() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][1]["outcomes"][1]["compatible_state_ids"]
            .as_array_mut()
            .expect("state array")
            .retain(|state| state != "fail-bad");
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_ACTION_PARTITION_INCOMPLETE"));
    }

    #[test]
    fn cycles_are_rejected_even_when_every_node_has_a_declared_id() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["policies"][0]["nodes"][0]["branches"][0]["next_node_id"] =
            Value::String("root".to_owned());
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_CYCLE"));
    }

    #[test]
    fn measurement_depth_budget_is_enforced_on_the_longest_branch() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["policies"][0]["max_depth"] = Value::from(1);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_DEPTH_BUDGET_EXCEEDED"));
    }

    #[test]
    fn terminal_leaf_rejects_decision_changes_between_remaining_states() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["candidates"][0]["state_results"][0]["objectives"]["repair_ticks"] =
            Value::from(2);
        corpus["cases"][0]["candidates"][1]["state_results"][1]["objectives"]["energy_units"] =
            Value::from(12);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_STOP_DECISION_NOT_STABLE"));
    }

    #[test]
    fn missing_outcome_branch_is_rejected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_ADAPTIVE_POLICY_CORPUS).expect("valid corpus");
        corpus["cases"][0]["policies"][0]["nodes"][0]["branches"]
            .as_array_mut()
            .expect("branch list")
            .retain(|branch| branch["outcome_id"] != "access-fail");
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_adaptive_policy_bytes(&bytes, EMBEDDED_ADAPTIVE_POLICY_SCHEMA.as_bytes());
        let adaptive = policy(&summary, "SPA-001", "adaptive-clearance-first");
        assert_eq!(adaptive["status"], "rejected");
        assert!(adaptive["rejection_codes"]
            .as_array()
            .expect("rejection code list")
            .iter()
            .any(|code| code == "POLICY_BRANCH_MISSING"));
    }

}
