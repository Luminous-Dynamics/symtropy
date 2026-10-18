//! Decision-impact evaluation over finite synthetic state partitions.
//!
//! This computes conditional robust-feasible and Pareto sets without assigning
//! probabilities. It does not estimate expected Value of Information or real costs.

use crate::sha256_hex;
use jsonschema::validator_for;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

pub const EMBEDDED_DECISION_IMPACT_CORPUS: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.json"
);
pub const EMBEDDED_DECISION_IMPACT_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/SPATIAL_RESILIENCE_DECISION_IMPACT_V0_1.schema.json"
);

const EXPECTED_CASE_COUNT: usize = 8;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DecisionImpactFailure {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DecisionSet {
    pub remaining_critical_unknown_ids: Vec<String>,
    pub robust_feasible_candidate_ids: Vec<String>,
    pub pareto_candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OutcomeImpact {
    pub outcome_id: String,
    pub compatible_state_ids: Vec<String>,
    pub compatible_state_count: usize,
    pub remaining_critical_unknown_ids: Vec<String>,
    pub robust_feasible_candidate_ids: Vec<String>,
    pub pareto_candidate_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ActionImpact {
    pub action_id: String,
    pub action_type: String,
    pub evidence_state: String,
    pub source_revision: String,
    pub status: String,
    pub rejection_codes: Vec<String>,
    pub decision_relevant: bool,
    pub changes_critical_unknown_set: bool,
    pub changes_hard_feasible_set: bool,
    pub changes_pareto_set: bool,
    pub worst_case_remaining_state_count: Option<usize>,
    pub newly_robust_feasible_candidate_ids: Vec<String>,
    pub estimated_cost_units: u64,
    pub estimated_duration_ticks: u64,
    pub cost_basis_id: String,
    pub duration_basis_id: String,
    pub outcomes: Vec<OutcomeImpact>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DecisionCaseResult {
    pub case_id: String,
    pub baseline: DecisionSet,
    pub actions: Vec<ActionImpact>,
    pub recommended_action_order: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DecisionImpactSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub corpus_sha256: String,
    pub schema_sha256: String,
    pub cases_seen: usize,
    pub cases_passed: usize,
    pub passed: bool,
    pub results: Vec<DecisionCaseResult>,
    pub failures: Vec<DecisionImpactFailure>,
}

impl DecisionImpactSummary {
    fn new(corpus_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            corpus_sha256: sha256_hex(corpus_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            cases_seen: 0,
            cases_passed: 0,
            passed: false,
            results: Vec::new(),
            failures: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.failures.push(DecisionImpactFailure {
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
struct ObservationOutcome {
    id: String,
    states: Vec<String>,
}

#[derive(Debug, Clone)]
struct MeasurementAction {
    id: String,
    action_type: String,
    evidence_state: String,
    source_revision: String,
    cost_units: u64,
    duration_ticks: u64,
    cost_basis_id: String,
    duration_basis_id: String,
    resolves_unknown_ids: Vec<String>,
    outcomes: Vec<ObservationOutcome>,
}

#[derive(Debug, Clone)]
struct Scenario {
    id: String,
    revision: String,
    state_ids: Vec<String>,
    current_states: Vec<String>,
    unresolved_unknowns: Vec<String>,
    candidates: Vec<Candidate>,
    actions: Vec<MeasurementAction>,
}

fn sorted(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut values: Vec<String> = values.into_iter().collect();
    values.sort();
    values.dedup();
    values
}

fn strings(object: &serde_json::Map<String, Value>, key: &str) -> Option<Vec<String>> {
    object
        .get(key)?
        .as_array()?
        .iter()
        .map(|value| value.as_str().map(ToOwned::to_owned))
        .collect()
}

fn u64_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<u64> {
    object.get(key)?.as_u64()
}

fn string_field<'a>(object: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key)?.as_str()
}

fn parse_scenario(
    value: &Value,
    index: usize,
    summary: &mut DecisionImpactSummary,
) -> Option<Scenario> {
    let path = format!("$.cases[{index}]");
    let object = value.as_object()?;
    let id = string_field(object, "id")?.to_owned();
    let revision = string_field(object, "state_revision")?.to_owned();
    let state_ids = strings(object, "state_ids")?;
    let current_states = strings(object, "current_possible_state_ids")?;
    let unresolved_unknowns = strings(object, "unresolved_critical_unknown_ids")?;

    check_unique_ids(&state_ids, &format!("{path}.state_ids"), summary);
    check_unique_ids(
        &current_states,
        &format!("{path}.current_possible_state_ids"),
        summary,
    );
    check_unique_ids(
        &unresolved_unknowns,
        &format!("{path}.unresolved_critical_unknown_ids"),
        summary,
    );

    let declared_states: BTreeSet<&str> = state_ids.iter().map(String::as_str).collect();
    for state in &current_states {
        if !declared_states.contains(state.as_str()) {
            summary.fail(
                "DECISION_CURRENT_STATE_UNDECLARED",
                format!("{path}.current_possible_state_ids"),
                format!("Current state {state:?} is not declared in state_ids."),
            );
        }
    }
    if current_states.is_empty() {
        summary.fail(
            "DECISION_CURRENT_STATES_EMPTY",
            format!("{path}.current_possible_state_ids"),
            "At least one current possible state is required.",
        );
    }

    let mut candidate_ids = BTreeSet::new();
    let mut candidates = Vec::new();
    let Some(candidate_values) = object.get("candidates").and_then(Value::as_array) else {
        summary.fail(
            "DECISION_CANDIDATES_MISSING",
            format!("{path}.candidates"),
            "Expected candidate array.",
        );
        return None;
    };

    for (candidate_index, candidate_value) in candidate_values.iter().enumerate() {
        let candidate_path = format!("{path}.candidates[{candidate_index}]");
        let Some(candidate_object) = candidate_value.as_object() else {
            summary.fail(
                "DECISION_CANDIDATE_INVALID",
                &candidate_path,
                "Candidate must be a JSON object.",
            );
            return None;
        };
        let Some(candidate_id) = string_field(candidate_object, "candidate_id") else {
            summary.fail(
                "DECISION_CANDIDATE_ID_MISSING",
                format!("{candidate_path}.candidate_id"),
                "Candidate ID must be a string.",
            );
            return None;
        };
        if !candidate_ids.insert(candidate_id.to_owned()) {
            summary.fail(
                "DECISION_CANDIDATE_ID_DUPLICATE",
                format!("{candidate_path}.candidate_id"),
                format!("Candidate ID {candidate_id:?} is duplicated."),
            );
        }

        let Some(state_result_values) = candidate_object
            .get("state_results")
            .and_then(Value::as_array)
        else {
            summary.fail(
                "DECISION_STATE_RESULTS_MISSING",
                format!("{candidate_path}.state_results"),
                "Expected a state_results array.",
            );
            return None;
        };

        let mut state_results = BTreeMap::new();
        for (result_index, state_result_value) in state_result_values.iter().enumerate() {
            let result_path = format!("{candidate_path}.state_results[{result_index}]");
            let Some(result_object) = state_result_value.as_object() else {
                summary.fail(
                    "DECISION_STATE_RESULT_INVALID",
                    &result_path,
                    "State result must be an object.",
                );
                return None;
            };
            let Some(state_id) = string_field(result_object, "state_id") else {
                summary.fail(
                    "DECISION_STATE_RESULT_ID_MISSING",
                    format!("{result_path}.state_id"),
                    "State result needs a state ID.",
                );
                return None;
            };
            if !declared_states.contains(state_id) {
                summary.fail(
                    "DECISION_STATE_RESULT_UNDECLARED",
                    format!("{result_path}.state_id"),
                    format!("Candidate state result references undeclared state {state_id:?}."),
                );
            }
            let Some(feasible) = result_object.get("feasible").and_then(Value::as_bool) else {
                summary.fail(
                    "DECISION_STATE_FEASIBILITY_MISSING",
                    format!("{result_path}.feasible"),
                    "Feasibility must be explicit.",
                );
                return None;
            };
            let Some(objectives) = result_object.get("objectives").and_then(Value::as_object)
            else {
                summary.fail(
                    "DECISION_OBJECTIVES_MISSING",
                    format!("{result_path}.objectives"),
                    "Objective vector must be declared.",
                );
                return None;
            };
            let Some(service_hours) = u64_field(objectives, "service_hours") else {
                return None;
            };
            let Some(energy_units) = u64_field(objectives, "energy_units") else {
                return None;
            };
            let Some(repair_ticks) = u64_field(objectives, "repair_ticks") else {
                return None;
            };
            let Some(cost_units) = u64_field(objectives, "cost_units") else {
                return None;
            };
            let entry = StateResult {
                feasible,
                objectives: ObjectiveVector {
                    service_hours,
                    energy_units,
                    repair_ticks,
                    cost_units,
                },
            };
            if state_results.insert(state_id.to_owned(), entry).is_some() {
                summary.fail(
                    "DECISION_STATE_RESULT_DUPLICATE",
                    format!("{result_path}.state_id"),
                    format!(
                        "Candidate {:?} duplicates state result {state_id:?}.",
                        candidate_id
                    ),
                );
            }
        }

        let found_states: BTreeSet<&str> = state_results.keys().map(String::as_str).collect();
        if found_states != declared_states {
            summary.fail(
                "DECISION_STATE_RESULT_COVERAGE",
                format!("{candidate_path}.state_results"),
                "Each candidate must declare exactly one result for every scenario state.",
            );
        }

        candidates.push(Candidate {
            id: candidate_id.to_owned(),
            states: state_results,
        });
    }

    let Some(action_values) = object.get("actions").and_then(Value::as_array) else {
        summary.fail(
            "DECISION_ACTIONS_MISSING",
            format!("{path}.actions"),
            "Expected actions array.",
        );
        return None;
    };
    let mut action_ids = BTreeSet::new();
    let mut actions = Vec::new();

    for (action_index, action_value) in action_values.iter().enumerate() {
        let action_path = format!("{path}.actions[{action_index}]");
        let Some(action_object) = action_value.as_object() else {
            summary.fail(
                "DECISION_ACTION_INVALID",
                &action_path,
                "Action must be an object.",
            );
            return None;
        };
        let Some(action_id) = string_field(action_object, "action_id") else {
            return None;
        };
        if !action_ids.insert(action_id.to_owned()) {
            summary.fail(
                "DECISION_ACTION_ID_DUPLICATE",
                format!("{action_path}.action_id"),
                format!("Action ID {action_id:?} is duplicated."),
            );
        }

        let Some(action_type) = string_field(action_object, "action_type") else {
            return None;
        };
        let Some(evidence_state) = string_field(action_object, "evidence_state") else {
            return None;
        };
        let Some(source_revision) = string_field(action_object, "source_revision") else {
            return None;
        };
        let Some(cost_units) = u64_field(action_object, "estimated_cost_units") else {
            return None;
        };
        let Some(duration_ticks) = u64_field(action_object, "estimated_duration_ticks") else {
            return None;
        };
        let Some(cost_basis_id) = string_field(action_object, "cost_basis_id") else {
            return None;
        };
        let Some(duration_basis_id) = string_field(action_object, "duration_basis_id") else {
            return None;
        };
        let Some(resolves_unknown_ids) = strings(action_object, "resolves_unknown_ids") else {
            return None;
        };
        let Some(outcome_values) = action_object.get("outcomes").and_then(Value::as_array) else {
            return None;
        };

        let mut outcomes = Vec::new();
        let mut outcome_parse_failed = false;
        for (outcome_index, outcome_value) in outcome_values.iter().enumerate() {
            let outcome_path = format!("{action_path}.outcomes[{outcome_index}]");
            let Some(outcome_object) = outcome_value.as_object() else {
                summary.fail(
                    "DECISION_OUTCOME_INVALID",
                    &outcome_path,
                    "Outcome must be an object.",
                );
                outcome_parse_failed = true;
                continue;
            };
            let Some(outcome_id) = string_field(outcome_object, "outcome_id") else {
                summary.fail(
                    "DECISION_OUTCOME_ID_MISSING",
                    format!("{outcome_path}.outcome_id"),
                    "Outcome ID must be a string.",
                );
                outcome_parse_failed = true;
                continue;
            };
            let Some(compatible_state_ids) = strings(outcome_object, "compatible_state_ids") else {
                summary.fail(
                    "DECISION_OUTCOME_STATES_MISSING",
                    format!("{outcome_path}.compatible_state_ids"),
                    "Outcome state partition must be an array.",
                );
                outcome_parse_failed = true;
                continue;
            };
            outcomes.push(ObservationOutcome {
                id: outcome_id.to_owned(),
                states: compatible_state_ids,
            });
        }
        if outcome_parse_failed {
            continue;
        }

        actions.push(MeasurementAction {
            id: action_id.to_owned(),
            action_type: action_type.to_owned(),
            evidence_state: evidence_state.to_owned(),
            source_revision: source_revision.to_owned(),
            cost_units,
            duration_ticks,
            cost_basis_id: cost_basis_id.to_owned(),
            duration_basis_id: duration_basis_id.to_owned(),
            resolves_unknown_ids,
            outcomes,
        });
    }

    Some(Scenario {
        id,
        revision,
        state_ids,
        current_states,
        unresolved_unknowns,
        candidates,
        actions,
    })
}

fn check_unique_ids(values: &[String], path: &str, summary: &mut DecisionImpactSummary) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            summary.fail(
                "DECISION_ID_DUPLICATE",
                path,
                format!("Identifier {value:?} is duplicated."),
            );
        }
    }
}

fn objective_for_states(candidate: &Candidate, states: &[String]) -> Option<ObjectiveVector> {
    let results: Vec<&StateResult> = states
        .iter()
        .map(|state| candidate.states.get(state))
        .collect::<Option<Vec<_>>>()?;

    Some(ObjectiveVector {
        service_hours: results
            .iter()
            .map(|result| result.objectives.service_hours)
            .min()?,
        energy_units: results
            .iter()
            .map(|result| result.objectives.energy_units)
            .max()?,
        repair_ticks: results
            .iter()
            .map(|result| result.objectives.repair_ticks)
            .max()?,
        cost_units: results
            .iter()
            .map(|result| result.objectives.cost_units)
            .max()?,
    })
}

fn robust_feasible_candidates(
    candidates: &[Candidate],
    states: &[String],
    unresolved_unknowns: &[String],
) -> Vec<String> {
    if !unresolved_unknowns.is_empty() {
        return Vec::new();
    }

    sorted(candidates.iter().filter_map(|candidate| {
        let feasible = states.iter().all(|state| {
            candidate
                .states
                .get(state)
                .is_some_and(|state_result| state_result.feasible)
        });
        feasible.then(|| candidate.id.clone())
    }))
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

fn pareto_candidates(
    candidates: &[Candidate],
    states: &[String],
    unresolved_unknowns: &[String],
) -> Vec<String> {
    let feasible = robust_feasible_candidates(candidates, states, unresolved_unknowns);
    let candidate_by_id: BTreeMap<&str, &Candidate> = candidates
        .iter()
        .map(|candidate| (candidate.id.as_str(), candidate))
        .collect();
    let admitted: Vec<&Candidate> = feasible
        .iter()
        .filter_map(|id| candidate_by_id.get(id.as_str()).copied())
        .collect();

    sorted(admitted.iter().filter_map(|candidate| {
        let right = objective_for_states(candidate, states)?;
        let is_dominated = admitted.iter().any(|other| {
            other.id != candidate.id
                && objective_for_states(other, states).is_some_and(|left| dominates(left, right))
        });
        (!is_dominated).then(|| candidate.id.clone())
    }))
}

fn decision_set(
    candidates: &[Candidate],
    states: &[String],
    unresolved_unknowns: &[String],
) -> DecisionSet {
    DecisionSet {
        remaining_critical_unknown_ids: sorted(unresolved_unknowns.to_vec()),
        robust_feasible_candidate_ids: robust_feasible_candidates(
            candidates,
            states,
            unresolved_unknowns,
        ),
        pareto_candidate_ids: pareto_candidates(candidates, states, unresolved_unknowns),
    }
}

fn same_ids(left: &[String], right: &[String]) -> bool {
    sorted(left.to_vec()) == sorted(right.to_vec())
}

fn compare_id_field(
    expected: &Value,
    field: &str,
    actual: &[String],
    path: &str,
    summary: &mut DecisionImpactSummary,
) {
    let Some(values) = expected.get(field).and_then(Value::as_array) else {
        summary.fail(
            "DECISION_EXPECTED_FIELD_MISSING",
            format!("{path}.{field}"),
            "Expected identifier array is missing or malformed.",
        );
        return;
    };
    let expected_values: Option<Vec<String>> = values
        .iter()
        .map(|value| value.as_str().map(ToOwned::to_owned))
        .collect();
    if expected_values
        .as_ref()
        .is_none_or(|values| !same_ids(values, actual))
    {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.{field}"),
            format!("Expected {:?}, computed {:?}.", expected_values, actual),
        );
    }
}

fn compare_decision_set(
    expected: &Value,
    actual: &DecisionSet,
    path: &str,
    summary: &mut DecisionImpactSummary,
) {
    compare_id_field(
        expected,
        "remaining_critical_unknown_ids",
        &actual.remaining_critical_unknown_ids,
        path,
        summary,
    );
    compare_id_field(
        expected,
        "robust_feasible_candidate_ids",
        &actual.robust_feasible_candidate_ids,
        path,
        summary,
    );
    compare_id_field(
        expected,
        "pareto_candidate_ids",
        &actual.pareto_candidate_ids,
        path,
        summary,
    );
}

fn action_rejection_codes(scenario: &Scenario, action: &MeasurementAction) -> Vec<String> {
    let mut codes = BTreeSet::new();

    if action.evidence_state != "valid" {
        codes.insert("ACTION_EVIDENCE_NOT_VALID".to_owned());
    }
    if action.source_revision != scenario.revision {
        codes.insert("ACTION_REVISION_MISMATCH".to_owned());
    }

    let mut resolved = BTreeSet::new();
    for unknown in &action.resolves_unknown_ids {
        if !scenario.unresolved_unknowns.contains(unknown) {
            codes.insert("ACTION_UNKNOWN_NOT_DECLARED".to_owned());
        }
        if !resolved.insert(unknown) {
            codes.insert("ACTION_UNKNOWN_ID_DUPLICATE".to_owned());
        }
    }

    let declared_current: BTreeSet<&str> =
        scenario.current_states.iter().map(String::as_str).collect();
    let mut state_memberships = BTreeSet::new();
    let mut outcome_ids = BTreeSet::new();

    if action.outcomes.is_empty() {
        codes.insert("ACTION_OUTCOMES_EMPTY".to_owned());
    }
    for outcome in &action.outcomes {
        if !outcome_ids.insert(outcome.id.as_str()) {
            codes.insert("ACTION_OUTCOME_ID_DUPLICATE".to_owned());
        }
        if outcome.states.is_empty() {
            codes.insert("ACTION_OUTCOME_STATES_EMPTY".to_owned());
        }
        let mut local_memberships = BTreeSet::new();
        for state in &outcome.states {
            if !declared_current.contains(state.as_str()) {
                codes.insert("ACTION_STATE_UNDECLARED".to_owned());
            }
            if !local_memberships.insert(state.as_str()) {
                codes.insert("ACTION_STATE_DUPLICATE_WITHIN_OUTCOME".to_owned());
            }
            if !state_memberships.insert(state.as_str()) {
                codes.insert("ACTION_PARTITION_OVERLAP".to_owned());
            }
        }
    }

    if !declared_current
        .iter()
        .all(|state| state_memberships.contains(state))
    {
        codes.insert("ACTION_PARTITION_INCOMPLETE".to_owned());
    }

    codes.into_iter().collect()
}

fn evaluate_action(
    scenario: &Scenario,
    action: &MeasurementAction,
    baseline: &DecisionSet,
) -> ActionImpact {
    let rejection_codes = action_rejection_codes(scenario, action);
    if !rejection_codes.is_empty() {
        return ActionImpact {
            action_id: action.id.clone(),
            action_type: action.action_type.clone(),
            evidence_state: action.evidence_state.clone(),
            source_revision: action.source_revision.clone(),
            status: "reject_action".to_owned(),
            rejection_codes,
            decision_relevant: false,
            changes_critical_unknown_set: false,
            changes_hard_feasible_set: false,
            changes_pareto_set: false,
            worst_case_remaining_state_count: None,
            newly_robust_feasible_candidate_ids: Vec::new(),
            estimated_cost_units: action.cost_units,
            estimated_duration_ticks: action.duration_ticks,
            cost_basis_id: action.cost_basis_id.clone(),
            duration_basis_id: action.duration_basis_id.clone(),
            outcomes: Vec::new(),
        };
    }

    let remaining_unknowns = sorted(
        scenario
            .unresolved_unknowns
            .iter()
            .filter(|unknown| !action.resolves_unknown_ids.contains(unknown))
            .cloned(),
    );
    let changes_critical_unknown_set =
        remaining_unknowns != baseline.remaining_critical_unknown_ids;

    let mut outcome_results = Vec::new();
    let mut worst_case_state_count = 0usize;
    let mut candidate_union = BTreeSet::new();
    let mut changes_hard = false;
    let mut changes_pareto = false;

    let mut ordered_outcomes = action.outcomes.clone();
    ordered_outcomes.sort_by(|left, right| left.id.cmp(&right.id));

    for outcome in ordered_outcomes {
        let states = sorted(outcome.states);
        let set = decision_set(scenario.candidates.as_slice(), &states, &remaining_unknowns);
        worst_case_state_count = worst_case_state_count.max(states.len());
        if set.robust_feasible_candidate_ids != baseline.robust_feasible_candidate_ids {
            changes_hard = true;
        }
        if set.pareto_candidate_ids != baseline.pareto_candidate_ids {
            changes_pareto = true;
        }
        for candidate_id in &set.robust_feasible_candidate_ids {
            candidate_union.insert(candidate_id.clone());
        }
        outcome_results.push(OutcomeImpact {
            outcome_id: outcome.id,
            compatible_state_ids: states.clone(),
            compatible_state_count: states.len(),
            remaining_critical_unknown_ids: set.remaining_critical_unknown_ids,
            robust_feasible_candidate_ids: set.robust_feasible_candidate_ids,
            pareto_candidate_ids: set.pareto_candidate_ids,
        });
    }

    let newly_robust = candidate_union
        .into_iter()
        .filter(|candidate| !baseline.robust_feasible_candidate_ids.contains(candidate))
        .collect();

    ActionImpact {
        action_id: action.id.clone(),
        action_type: action.action_type.clone(),
        evidence_state: action.evidence_state.clone(),
        source_revision: action.source_revision.clone(),
        status: "evaluated".to_owned(),
        rejection_codes: Vec::new(),
        decision_relevant: changes_critical_unknown_set || changes_hard || changes_pareto,
        changes_critical_unknown_set,
        changes_hard_feasible_set: changes_hard,
        changes_pareto_set: changes_pareto,
        worst_case_remaining_state_count: Some(worst_case_state_count),
        newly_robust_feasible_candidate_ids: newly_robust,
        estimated_cost_units: action.cost_units,
        estimated_duration_ticks: action.duration_ticks,
        cost_basis_id: action.cost_basis_id.clone(),
        duration_basis_id: action.duration_basis_id.clone(),
        outcomes: outcome_results,
    }
}

fn compare_action_expected(
    expected: &Value,
    actual: &ActionImpact,
    path: &str,
    summary: &mut DecisionImpactSummary,
) {
    let Some(object) = expected.as_object() else {
        summary.fail(
            "DECISION_EXPECTED_ACTION_INVALID",
            path,
            "Expected action must be an object.",
        );
        return;
    };

    macro_rules! check_bool {
        ($field:literal, $actual:expr) => {
            if object.get($field).and_then(Value::as_bool) != Some($actual) {
                summary.fail(
                    "DECISION_EXPECTATION_MISMATCH",
                    format!("{path}.{}", $field),
                    format!("Expected {:?}, computed {:?}.", object.get($field), $actual),
                );
            }
        };
    }

    if string_field(object, "action_id") != Some(actual.action_id.as_str()) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.action_id"),
            "Action identity differs.",
        );
    }
    for (field, actual_value) in [
        ("action_type", actual.action_type.as_str()),
        ("evidence_state", actual.evidence_state.as_str()),
        ("source_revision", actual.source_revision.as_str()),
    ] {
        if string_field(object, field) != Some(actual_value) {
            summary.fail(
                "DECISION_EXPECTATION_MISMATCH",
                format!("{path}.{field}"),
                format!("Action provenance field {field:?} differs."),
            );
        }
    }
    if string_field(object, "status") != Some(actual.status.as_str()) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.status"),
            format!(
                "Expected {:?}, computed {:?}.",
                object.get("status"),
                actual.status
            ),
        );
    }

    check_bool!("decision_relevant", actual.decision_relevant);
    check_bool!(
        "changes_critical_unknown_set",
        actual.changes_critical_unknown_set
    );
    check_bool!(
        "changes_hard_feasible_set",
        actual.changes_hard_feasible_set
    );
    check_bool!("changes_pareto_set", actual.changes_pareto_set);

    if string_field(object, "cost_basis_id") != Some(actual.cost_basis_id.as_str()) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.cost_basis_id"),
            "Measurement cost basis differs from the frozen corpus.",
        );
    }
    if string_field(object, "duration_basis_id") != Some(actual.duration_basis_id.as_str()) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.duration_basis_id"),
            "Measurement duration basis differs from the frozen corpus.",
        );
    }
    if u64_field(object, "estimated_cost_units") != Some(actual.estimated_cost_units) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.estimated_cost_units"),
            "Measurement cost differs from the frozen corpus.",
        );
    }
    if u64_field(object, "estimated_duration_ticks") != Some(actual.estimated_duration_ticks) {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.estimated_duration_ticks"),
            "Measurement duration differs from the frozen corpus.",
        );
    }

    let expected_worst = match object.get("worst_case_remaining_state_count") {
        Some(Value::Null) => None,
        Some(value) => value
            .as_u64()
            .and_then(|number| usize::try_from(number).ok()),
        None => {
            summary.fail(
                "DECISION_EXPECTATION_MISSING",
                format!("{path}.worst_case_remaining_state_count"),
                "Expected field is required.",
            );
            None
        }
    };
    if expected_worst != actual.worst_case_remaining_state_count {
        summary.fail(
            "DECISION_EXPECTATION_MISMATCH",
            format!("{path}.worst_case_remaining_state_count"),
            format!(
                "Expected {:?}, computed {:?}.",
                expected_worst, actual.worst_case_remaining_state_count
            ),
        );
    }

    compare_id_field(
        expected,
        "newly_robust_feasible_candidate_ids",
        &actual.newly_robust_feasible_candidate_ids,
        path,
        summary,
    );

    let expected_outcomes = object.get("outcomes").and_then(Value::as_array);
    let Some(expected_outcomes) = expected_outcomes else {
        summary.fail(
            "DECISION_EXPECTED_OUTCOMES_MISSING",
            format!("{path}.outcomes"),
            "Expected an outcome array.",
        );
        return;
    };
    let expected_by_id: BTreeMap<&str, &Value> = expected_outcomes
        .iter()
        .filter_map(|outcome| Some((outcome.get("outcome_id")?.as_str()?, outcome)))
        .collect();
    if expected_by_id.len() != expected_outcomes.len()
        || expected_by_id.len() != actual.outcomes.len()
    {
        summary.fail(
            "DECISION_EXPECTED_OUTCOME_COVERAGE",
            format!("{path}.outcomes"),
            "Expected outcome identities do not match computed outcomes.",
        );
        return;
    }

    for outcome in &actual.outcomes {
        let Some(expected_outcome) = expected_by_id.get(outcome.outcome_id.as_str()) else {
            summary.fail(
                "DECISION_EXPECTED_OUTCOME_MISSING",
                format!("{path}.outcomes"),
                format!("No expected answer for outcome {:?}.", outcome.outcome_id),
            );
            continue;
        };
        compare_id_field(
            expected_outcome,
            "compatible_state_ids",
            &outcome.compatible_state_ids,
            &format!("{path}.outcomes[{}]", outcome.outcome_id),
            summary,
        );
        if expected_outcome
            .get("compatible_state_count")
            .and_then(Value::as_u64)
            != Some(outcome.compatible_state_count as u64)
        {
            summary.fail(
                "DECISION_EXPECTATION_MISMATCH",
                format!(
                    "{path}.outcomes[{}].compatible_state_count",
                    outcome.outcome_id
                ),
                "Outcome state count differs.",
            );
        }
        compare_decision_set(
            expected_outcome,
            &DecisionSet {
                remaining_critical_unknown_ids: outcome.remaining_critical_unknown_ids.clone(),
                robust_feasible_candidate_ids: outcome.robust_feasible_candidate_ids.clone(),
                pareto_candidate_ids: outcome.pareto_candidate_ids.clone(),
            },
            &format!("{path}.outcomes[{}]", outcome.outcome_id),
            summary,
        );
    }
}

fn evaluate_scenario(scenario: &Scenario) -> DecisionCaseResult {
    let baseline = decision_set(
        &scenario.candidates,
        &scenario.current_states,
        &scenario.unresolved_unknowns,
    );
    let mut actions: Vec<ActionImpact> = scenario
        .actions
        .iter()
        .map(|action| evaluate_action(scenario, action, &baseline))
        .collect();
    actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let mut recommended: Vec<&ActionImpact> = actions
        .iter()
        .filter(|action| action.status == "evaluated" && action.decision_relevant)
        .collect();
    recommended.sort_by(|left, right| {
        let left_resolved = scenario
            .actions
            .iter()
            .find(|action| action.id == left.action_id)
            .map_or(0, |action| action.resolves_unknown_ids.len());
        let right_resolved = scenario
            .actions
            .iter()
            .find(|action| action.id == right.action_id)
            .map_or(0, |action| action.resolves_unknown_ids.len());

        right_resolved
            .cmp(&left_resolved)
            .then_with(|| {
                right
                    .changes_hard_feasible_set
                    .cmp(&left.changes_hard_feasible_set)
            })
            .then_with(|| right.changes_pareto_set.cmp(&left.changes_pareto_set))
            .then_with(|| {
                left.worst_case_remaining_state_count
                    .cmp(&right.worst_case_remaining_state_count)
            })
            .then_with(|| left.estimated_cost_units.cmp(&right.estimated_cost_units))
            .then_with(|| {
                left.estimated_duration_ticks
                    .cmp(&right.estimated_duration_ticks)
            })
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    let recommended_action_order = recommended
        .into_iter()
        .map(|action| action.action_id.clone())
        .collect();

    DecisionCaseResult {
        case_id: scenario.id.clone(),
        baseline,
        actions,
        recommended_action_order,
    }
}

fn compare_case_expected(
    case_value: &Value,
    actual: &DecisionCaseResult,
    path: &str,
    summary: &mut DecisionImpactSummary,
) {
    let Some(object) = case_value.as_object() else {
        return;
    };
    if let Some(expected) = object.get("expected_baseline") {
        compare_decision_set(
            expected,
            &actual.baseline,
            &format!("{path}.expected_baseline"),
            summary,
        );
    }

    let Some(expected_actions) = object.get("expected_actions").and_then(Value::as_array) else {
        summary.fail(
            "DECISION_EXPECTED_ACTIONS_MISSING",
            format!("{path}.expected_actions"),
            "Expected action results are required.",
        );
        return;
    };
    let expected_by_id: BTreeMap<&str, &Value> = expected_actions
        .iter()
        .filter_map(|action| Some((action.get("action_id")?.as_str()?, action)))
        .collect();
    if expected_by_id.len() != expected_actions.len()
        || expected_by_id.len() != actual.actions.len()
    {
        summary.fail(
            "DECISION_EXPECTED_ACTION_COVERAGE",
            format!("{path}.expected_actions"),
            "Expected action identities do not match the frozen action set.",
        );
        return;
    }

    for action in &actual.actions {
        let Some(expected_action) = expected_by_id.get(action.action_id.as_str()) else {
            summary.fail(
                "DECISION_EXPECTED_ACTION_MISSING",
                format!("{path}.expected_actions"),
                format!("No expected result for action {:?}.", action.action_id),
            );
            continue;
        };
        compare_action_expected(
            expected_action,
            action,
            &format!("{path}.expected_actions[{}]", action.action_id),
            summary,
        );
    }

    let expected_order = strings(object, "expected_recommended_action_order").unwrap_or_default();
    if expected_order != actual.recommended_action_order {
        summary.fail(
            "DECISION_RECOMMENDATION_ORDER_MISMATCH",
            format!("{path}.expected_recommended_action_order"),
            format!(
                "Expected {:?}, computed {:?}.",
                expected_order, actual.recommended_action_order
            ),
        );
    }
}

pub fn validate_decision_impact_bytes(
    corpus_bytes: &[u8],
    schema_bytes: &[u8],
) -> DecisionImpactSummary {
    let mut summary = DecisionImpactSummary::new(corpus_bytes, schema_bytes);
    let schema_value: Value = match serde_json::from_slice(schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("DECISION_SCHEMA_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let corpus_value: Value = match serde_json::from_slice(corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("DECISION_CORPUS_JSON_INVALID", "$", error.to_string());
            return summary;
        }
    };
    let validator = match validator_for(&schema_value) {
        Ok(validator) => validator,
        Err(error) => {
            summary.fail("DECISION_SCHEMA_COMPILE_FAILED", "$", error.to_string());
            return summary;
        }
    };
    let schema_errors: Vec<_> = validator.iter_errors(&corpus_value).collect();
    if !schema_errors.is_empty() {
        for error in schema_errors {
            summary.fail(
                "DECISION_SCHEMA_VALIDATION_FAILED",
                error.instance_path().to_string(),
                error.to_string(),
            );
        }
        return summary;
    }
    let Some(root) = corpus_value.as_object() else {
        summary.fail("DECISION_CORPUS_ROOT_INVALID", "$", "Expected an object.");
        return summary;
    };
    summary.benchmark_id = string_field(root, "benchmark_id").map(ToOwned::to_owned);
    summary.version = string_field(root, "version").map(ToOwned::to_owned);
    if summary.benchmark_id.as_deref() != Some("SPATIAL-RESILIENCE-DECISION-IMPACT")
        || summary.version.as_deref() != Some("0.1.0")
    {
        summary.fail(
            "DECISION_CORPUS_ID_OR_VERSION_MISMATCH",
            "$",
            "This verifier freezes decision-impact corpus version 0.1.0.",
        );
        return summary;
    }

    let Some(cases) = root.get("cases").and_then(Value::as_array) else {
        summary.fail(
            "DECISION_CASES_MISSING",
            "$.cases",
            "Expected a case array.",
        );
        return summary;
    };
    summary.cases_seen = cases.len();
    if cases.len() != EXPECTED_CASE_COUNT {
        summary.fail(
            "DECISION_CASE_COUNT_MISMATCH",
            "$.cases",
            format!("Expected exactly {EXPECTED_CASE_COUNT} cases."),
        );
        return summary;
    }

    let mut scenario_ids = BTreeSet::new();
    for (index, case_value) in cases.iter().enumerate() {
        let path = format!("$.cases[{index}]");
        let Some(case_object) = case_value.as_object() else {
            summary.fail("DECISION_CASE_INVALID", &path, "Case must be an object.");
            continue;
        };
        let case_id = string_field(case_object, "id").unwrap_or("");
        let expected_id = format!("SPR-V-{:03}", index + 1);
        if case_id != expected_id {
            summary.fail(
                "DECISION_CASE_ID_SEQUENCE",
                format!("{path}.id"),
                format!("Expected {expected_id:?}, got {case_id:?}."),
            );
        }
        if !scenario_ids.insert(case_id.to_owned()) {
            summary.fail(
                "DECISION_CASE_ID_DUPLICATE",
                format!("{path}.id"),
                format!("Case ID {case_id:?} is duplicated."),
            );
            continue;
        }

        let before = summary.failures.len();
        let Some(scenario) = parse_scenario(case_value, index, &mut summary) else {
            continue;
        };
        let result = evaluate_scenario(&scenario);
        compare_case_expected(case_value, &result, &path, &mut summary);
        if summary.failures.len() == before {
            summary.cases_passed += 1;
        }
        summary.results.push(result);
    }

    summary.failures.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary
        .results
        .sort_by(|left, right| left.case_id.cmp(&right.case_id));
    summary.passed = summary.failures.is_empty() && summary.cases_passed == summary.cases_seen;
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_code(summary: &DecisionImpactSummary, code: &str) -> bool {
        summary.failures.iter().any(|failure| failure.code == code)
    }

    #[test]
    fn embedded_decision_impact_corpus_matches_known_answers() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        assert!(
            summary.passed,
            "unexpected failures: {:?}",
            summary.failures
        );
        assert_eq!(summary.cases_seen, 8);
        assert_eq!(summary.cases_passed, 8);
        assert_eq!(summary.corpus_sha256.len(), 64);
        assert_eq!(summary.schema_sha256.len(), 64);
    }

    #[test]
    fn inexpensive_decision_changing_survey_outranks_expensive_scan() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-001")
            .unwrap();
        assert_eq!(
            case.recommended_action_order,
            vec!["targeted-clearance-survey", "full-3d-scan"]
        );
    }

    #[test]
    fn measurement_without_decision_impact_is_not_misrepresented_as_useful() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-002")
            .unwrap();
        assert!(case.actions.iter().all(|action| !action.decision_relevant));
        assert!(case.recommended_action_order.is_empty());
    }

    #[test]
    fn unresolved_critical_unknown_blocks_robust_feasibility_after_partial_survey() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-005")
            .unwrap();
        let wall_only = case
            .actions
            .iter()
            .find(|action| action.action_id == "wall-only-survey")
            .unwrap();
        let combined = case
            .actions
            .iter()
            .find(|action| action.action_id == "wall-survey-plus-qualified-fire-review")
            .unwrap();
        assert!(
            wall_only
                .outcomes
                .iter()
                .all(|outcome| outcome.robust_feasible_candidate_ids.is_empty())
        );
        assert!(
            combined
                .outcomes
                .iter()
                .any(|outcome| !outcome.robust_feasible_candidate_ids.is_empty())
        );
    }

    #[test]
    fn pareto_change_is_reported_even_when_hard_feasibility_is_unchanged() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-007")
            .unwrap();
        let action = &case.actions[0];
        assert!(!action.changes_hard_feasible_set);
        assert!(action.changes_pareto_set);
    }

    #[test]
    fn invalid_action_partitions_are_rejected_with_stable_reasons() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let incomplete = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-003")
            .unwrap();
        let overlapping = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-004")
            .unwrap();
        assert!(
            incomplete.actions[0]
                .rejection_codes
                .contains(&"ACTION_PARTITION_INCOMPLETE".to_owned())
        );
        assert!(
            overlapping.actions[0]
                .rejection_codes
                .contains(&"ACTION_PARTITION_OVERLAP".to_owned())
        );
    }

    #[test]
    fn stale_sources_and_mis_scoped_unknown_resolution_are_rejected() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let stale = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-006")
            .unwrap();
        let mis_scoped = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-008")
            .unwrap();
        assert!(
            stale.actions[0]
                .rejection_codes
                .contains(&"ACTION_EVIDENCE_NOT_VALID".to_owned())
        );
        assert!(
            mis_scoped.actions[0]
                .rejection_codes
                .contains(&"ACTION_UNKNOWN_NOT_DECLARED".to_owned())
        );
    }

    #[test]
    fn mutation_of_measurement_cost_or_basis_is_detected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][0]["estimated_cost_units"] = Value::from(0);
        corpus["cases"][0]["actions"][0]["cost_basis_id"] =
            Value::String("unreviewed-cost-source-v1".to_owned());
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        assert!(has_code(&summary, "DECISION_EXPECTATION_MISMATCH"));
    }

    #[test]
    fn outcome_with_same_count_but_wrong_state_membership_is_rejected() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][0]["expected_actions"][0]["outcomes"][0]["compatible_state_ids"] =
            serde_json::json!(["width-ok"]);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        assert!(has_code(&summary, "DECISION_EXPECTATION_MISMATCH"));
    }

    #[test]
    fn action_receipt_preserves_type_evidence_revision_and_state_membership() {
        let summary = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-001")
            .unwrap();
        let action = case
            .actions
            .iter()
            .find(|item| item.action_id == "targeted-clearance-survey")
            .unwrap();
        assert_eq!(action.action_type, "survey");
        assert_eq!(action.evidence_state, "valid");
        assert_eq!(action.source_revision, "access-survey-v1");
        assert_eq!(action.cost_basis_id, "synthetic-cost-basis-v1");
        assert_eq!(action.duration_basis_id, "synthetic-duration-basis-v1");
        let fail_outcome = action
            .outcomes
            .iter()
            .find(|item| item.outcome_id == "clearance-fail")
            .unwrap();
        assert_eq!(fail_outcome.compatible_state_ids, vec!["width-fail"]);
    }

    #[test]
    fn undeclared_state_in_partition_is_rejected_semantically() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][0]["outcomes"][0]["compatible_state_ids"] =
            serde_json::json!(["undeclared-state"]);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        let case = summary
            .results
            .iter()
            .find(|result| result.case_id == "SPR-V-001")
            .expect("decision case should still return diagnostic results");
        let action = case
            .actions
            .iter()
            .find(|item| item.action_id == "targeted-clearance-survey")
            .expect("targeted survey result");
        assert!(
            action
                .rejection_codes
                .contains(&"ACTION_STATE_UNDECLARED".to_owned())
        );
        assert!(
            action
                .rejection_codes
                .contains(&"ACTION_PARTITION_INCOMPLETE".to_owned())
        );
    }

    #[test]
    fn duplicate_state_inside_outcome_is_rejected_by_schema() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][0]["outcomes"][0]["compatible_state_ids"] =
            serde_json::json!(["width-ok", "width-ok"]);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        assert!(has_code(&summary, "DECISION_SCHEMA_VALIDATION_FAILED"));
    }

    #[test]
    fn unsupported_probability_fields_are_rejected_by_schema() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][0]["actions"][0]["probability"] = Value::from(0.8);
        let bytes = serde_json::to_vec(&corpus).expect("serialize mutation");
        let summary =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        assert!(has_code(&summary, "DECISION_SCHEMA_VALIDATION_FAILED"));
    }

    fn reference_robust(
        candidates: &[Candidate],
        states: &[String],
        unresolved: &[String],
    ) -> Vec<String> {
        let mut answer = Vec::new();
        if unresolved.is_empty() {
            for candidate in candidates {
                let mut accepted = true;
                for state in states {
                    match candidate.states.get(state) {
                        Some(result) if result.feasible => {}
                        _ => {
                            accepted = false;
                            break;
                        }
                    }
                }
                if accepted {
                    answer.push(candidate.id.clone());
                }
            }
        }
        answer.sort();
        answer
    }

    fn reference_objectives(candidate: &Candidate, states: &[String]) -> Option<ObjectiveVector> {
        let mut service_hours = u64::MAX;
        let mut energy_units = 0;
        let mut repair_ticks = 0;
        let mut cost_units = 0;
        for state in states {
            let current = candidate.states.get(state)?.objectives;
            service_hours = service_hours.min(current.service_hours);
            energy_units = energy_units.max(current.energy_units);
            repair_ticks = repair_ticks.max(current.repair_ticks);
            cost_units = cost_units.max(current.cost_units);
        }
        (!states.is_empty()).then_some(ObjectiveVector {
            service_hours,
            energy_units,
            repair_ticks,
            cost_units,
        })
    }

    fn reference_dominates(left: ObjectiveVector, right: ObjectiveVector) -> bool {
        let all_no_worse = left.service_hours >= right.service_hours
            && left.energy_units <= right.energy_units
            && left.repair_ticks <= right.repair_ticks
            && left.cost_units <= right.cost_units;
        let any_better = left.service_hours > right.service_hours
            || left.energy_units < right.energy_units
            || left.repair_ticks < right.repair_ticks
            || left.cost_units < right.cost_units;
        all_no_worse && any_better
    }

    fn reference_pareto(
        candidates: &[Candidate],
        states: &[String],
        unresolved: &[String],
    ) -> Vec<String> {
        let feasible = reference_robust(candidates, states, unresolved);
        let mut answer = Vec::new();
        for candidate_id in &feasible {
            let candidate = candidates
                .iter()
                .find(|item| item.id == *candidate_id)
                .expect("feasible reference candidate must exist");
            let current = reference_objectives(candidate, states)
                .expect("candidate must have state objectives");
            let mut dominated = false;
            for other_id in &feasible {
                if other_id == candidate_id {
                    continue;
                }
                let other = candidates
                    .iter()
                    .find(|item| item.id == *other_id)
                    .expect("dominator candidate must exist");
                let other_vector = reference_objectives(other, states)
                    .expect("other candidate must have state objectives");
                if reference_dominates(other_vector, current) {
                    dominated = true;
                    break;
                }
            }
            if !dominated {
                answer.push(candidate_id.clone());
            }
        }
        answer.sort();
        answer
    }

    #[test]
    fn resolving_one_of_two_critical_unknowns_is_relevant_without_premature_feasibility() {
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        corpus["cases"][4]["unresolved_critical_unknown_ids"] =
            serde_json::json!(["fire-separation-review", "structural-load-review"]);
        let bytes = serde_json::to_vec(&corpus).expect("serialize augmented unknowns");
        let mut summary =
            DecisionImpactSummary::new(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        let scenario =
            parse_scenario(&corpus["cases"][4], 4, &mut summary).expect("scenario should parse");
        let result = evaluate_scenario(&scenario);
        let combined = result
            .actions
            .iter()
            .find(|item| item.action_id == "wall-survey-plus-qualified-fire-review")
            .unwrap();
        assert!(combined.changes_critical_unknown_set);
        assert!(combined.decision_relevant);
        assert!(!combined.changes_hard_feasible_set);
        assert!(!combined.changes_pareto_set);
        assert!(
            combined
                .outcomes
                .iter()
                .all(|outcome| outcome.robust_feasible_candidate_ids.is_empty())
        );
        assert!(combined.outcomes.iter().all(
            |outcome| outcome.remaining_critical_unknown_ids == vec!["structural-load-review"]
        ));
    }

    #[test]
    fn exhaustive_reference_matches_100_worlds_and_over_10000_candidate_state_results() {
        let mut candidate_state_evaluations = 0usize;
        for world in 0..100usize {
            let state_count = 2 + (world % 4);
            let states: Vec<String> = (0..state_count)
                .map(|index| format!("state-{index}"))
                .collect();
            let mut candidates = Vec::with_capacity(100);

            for candidate_index in 0..100usize {
                let mut state_results = BTreeMap::new();
                for (state_index, state_id) in states.iter().enumerate() {
                    let seed = world * 17 + candidate_index * 7 + state_index * 11;
                    let objectives = ObjectiveVector {
                        service_hours: ((seed * 3 + 5) % 19) as u64,
                        energy_units: ((seed * 5 + 2) % 17) as u64,
                        repair_ticks: ((seed * 7 + 3) % 13) as u64,
                        cost_units: ((seed * 11 + 1) % 23) as u64,
                    };
                    state_results.insert(
                        state_id.clone(),
                        StateResult {
                            feasible: seed % 13 != 0,
                            objectives,
                        },
                    );
                }
                candidates.push(Candidate {
                    id: format!("candidate-{candidate_index:03}"),
                    states: state_results,
                });
            }

            candidate_state_evaluations += candidates.len() * states.len();
            let mut partitions = vec![states.clone()];
            for excluded_index in 0..states.len() {
                partitions.push(
                    states
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| *index != excluded_index)
                        .map(|(_, state)| state.clone())
                        .collect(),
                );
            }

            for (partition_index, partition) in partitions.iter().enumerate() {
                let unresolved = if (world + partition_index) % 19 == 0 {
                    vec!["critical-unknown".to_owned()]
                } else {
                    Vec::new()
                };
                assert_eq!(
                    robust_feasible_candidates(&candidates, partition, &unresolved),
                    reference_robust(&candidates, partition, &unresolved),
                    "robust feasible mismatch in world {world}, partition {partition_index}"
                );
                assert_eq!(
                    pareto_candidates(&candidates, partition, &unresolved),
                    reference_pareto(&candidates, partition, &unresolved),
                    "Pareto mismatch in world {world}, partition {partition_index}"
                );
            }
        }
        assert!(
            candidate_state_evaluations >= 10_000,
            "generated test must cover at least 10,000 candidate/state results"
        );
    }

    #[test]
    fn input_order_changes_do_not_change_decision_outputs() {
        let original = validate_decision_impact_bytes(
            EMBEDDED_DECISION_IMPACT_CORPUS.as_bytes(),
            EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes(),
        );
        let mut corpus: Value =
            serde_json::from_str(EMBEDDED_DECISION_IMPACT_CORPUS).expect("valid corpus");
        let first = &mut corpus["cases"][0];
        first["state_ids"].as_array_mut().unwrap().reverse();
        first["current_possible_state_ids"]
            .as_array_mut()
            .unwrap()
            .reverse();
        first["candidates"].as_array_mut().unwrap().reverse();
        for candidate in first["candidates"].as_array_mut().unwrap() {
            candidate["state_results"].as_array_mut().unwrap().reverse();
        }
        first["actions"].as_array_mut().unwrap().reverse();
        for action in first["actions"].as_array_mut().unwrap() {
            action["outcomes"].as_array_mut().unwrap().reverse();
            for outcome in action["outcomes"].as_array_mut().unwrap() {
                outcome["compatible_state_ids"]
                    .as_array_mut()
                    .unwrap()
                    .reverse();
            }
        }
        let bytes = serde_json::to_vec(&corpus).expect("serialize reordered corpus");
        let reordered =
            validate_decision_impact_bytes(&bytes, EMBEDDED_DECISION_IMPACT_SCHEMA.as_bytes());
        assert!(
            original.passed,
            "original failures: {:?}",
            original.failures
        );
        assert!(
            reordered.passed,
            "reordered failures: {:?}",
            reordered.failures
        );
        assert_eq!(original.results, reordered.results);
    }
}
