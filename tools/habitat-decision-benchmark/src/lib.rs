// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.habitat_decision_benchmark.report.v0.1";
pub const CLAIM_CEILING: &str =
    "synthetic_decision_impact_only_not_real_site_safety";
const FIXTURE_SHA256: &str = "4c30be2c0e10c5a292a2dd2016860807a40caee2970253fda498b4022f87b8a9";
const RANKING_POLICY: &str = concat!(
    "maximize_minimax_regret_reduction_per_cost_unit_",
    "then_minimize_post_regret_then_cost_then_duration_then_id"
);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecisionFixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub status: String,
    pub synthetic_only: bool,
    pub claim_ceiling: String,
    pub intended_use: String,
    pub state_space: Vec<SiteState>,
    pub candidates: Vec<SiteCandidate>,
    pub actions: Vec<SurveyAction>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteState {
    pub id: String,
    pub hazards: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteCandidate {
    pub id: String,
    pub cost_units: u64,
    pub blocked_if_any_hazard_present: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SurveyAction {
    pub id: String,
    pub cost_units: u64,
    pub duration_hours: u32,
    pub description: String,
    pub outcomes: Vec<SurveyOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SurveyOutcome {
    pub id: String,
    pub state_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeDecision {
    pub outcome_id: String,
    pub possible_state_ids: Vec<String>,
    pub selected_candidate_id: String,
    pub selected_candidate_cost_units: u64,
    pub worst_case_regret_units: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionScore {
    pub action_id: String,
    pub action_cost_units: u64,
    pub duration_hours: u32,
    pub outcome_count: usize,
    pub max_residual_state_count: usize,
    pub outcomes_changing_baseline_choice: usize,
    pub worst_case_regret_after_units: u64,
    pub worst_case_regret_reduction_units: u64,
    pub regret_reduction_per_cost_unit_milli: u64,
    pub worst_case_total_cost_units: u64,
    pub outcome_decisions: Vec<OutcomeDecision>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub verification_status: String,
    pub claim_ceiling: String,
    pub ranking_policy: String,
    pub state_count: usize,
    pub candidate_count: usize,
    pub action_count: usize,
    pub baseline_robust_candidate_id: String,
    pub baseline_worst_case_regret_units: u64,
    pub ranked_actions: Vec<ActionScore>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The exact frozen bytes are checked before the fixture is interpreted.
pub fn verify_fixture_bytes(bytes: &[u8]) -> Result<BenchmarkReport, Vec<String>> {
    let digest = sha256_hex(bytes);
    if digest != FIXTURE_SHA256 {
        return Err(vec![format!(
            "frozen decision fixture digest mismatch: expected {FIXTURE_SHA256}, got {digest}"
        )]);
    }
    let fixture: DecisionFixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("decision fixture rejected: {error}")])?;
    evaluate_fixture(&fixture, digest)
}

/// Exposed for adversarial tests and independent experiments. Production CLI uses
/// verify_fixture_bytes, which also pins the exact frozen source bytes.
fn evaluate_fixture(
    fixture: &DecisionFixture,
    input_sha256: String,
) -> Result<BenchmarkReport, Vec<String>> {
    let errors = validate_fixture(fixture);
    if !errors.is_empty() {
        return Err(errors);
    }

    let all_state_ids: Vec<String> = fixture
        .state_space
        .iter()
        .map(|state| state.id.clone())
        .collect();
    let baseline = choose_robust_candidate(fixture, &all_state_ids)
        .expect("validation guarantees a robust-feasible baseline");
    let baseline_regret =
        worst_regret_for_selected_candidate(fixture, baseline, &all_state_ids)
            .expect("validated finite-state candidate costs cannot overflow regret");
    let mut ranked_actions = fixture
        .actions
        .iter()
        .map(|action| evaluate_action(fixture, action, baseline, baseline_regret))
        .collect::<Result<Vec<_>, _>>()?;
    ranked_actions.sort_by(compare_action_scores);

    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        verification_status: "SYNTHETIC_REFERENCE_EVALUATION_NOT_PHYSICAL_VALIDATION".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        ranking_policy: RANKING_POLICY.to_owned(),
        state_count: fixture.state_space.len(),
        candidate_count: fixture.candidates.len(),
        action_count: fixture.actions.len(),
        baseline_robust_candidate_id: baseline.id.clone(),
        baseline_worst_case_regret_units: baseline_regret,
        ranked_actions,
    })
}

fn validate_fixture(fixture: &DecisionFixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "luminous.habitat_decision_fixture.v0.1" {
        errors.push(format!("unsupported schema_version {:?}", fixture.schema_version));
    }
    if fixture.fixture_id.trim().is_empty() {
        errors.push("fixture_id must not be blank".to_owned());
    }
    if fixture.status != "proposed_synthetic_decision_benchmark" {
        errors.push(format!("unsupported status {:?}", fixture.status));
    }
    if !fixture.synthetic_only {
        errors.push("synthetic_only must remain true".to_owned());
    }
    if fixture.claim_ceiling != CLAIM_CEILING {
        errors.push(format!("claim ceiling must be {CLAIM_CEILING:?}"));
    }
    if fixture.intended_use.trim().is_empty() {
        errors.push("intended_use must not be blank".to_owned());
    }
    if fixture.state_space.len() != 4 {
        errors.push(format!("v0.1 requires 4 states, got {}", fixture.state_space.len()));
    }
    if fixture.candidates.len() != 3 {
        errors.push(format!("v0.1 requires 3 candidates, got {}", fixture.candidates.len()));
    }
    if fixture.actions.len() != 3 {
        errors.push(format!("v0.1 requires 3 actions, got {}", fixture.actions.len()));
    }

    let mut state_ids = BTreeSet::new();
    for state in &fixture.state_space {
        if state.id.trim().is_empty() {
            errors.push("state ID must not be blank".to_owned());
        }
        if !state_ids.insert(state.id.clone()) {
            errors.push(format!("duplicate state ID {:?}", state.id));
        }
        if state.hazards.is_empty() {
            errors.push(format!("{} must declare at least one hazard variable", state.id));
        }
    }
    let hazard_names: BTreeSet<String> = fixture
        .state_space
        .first()
        .map(|state| state.hazards.keys().cloned().collect())
        .unwrap_or_default();
    for state in &fixture.state_space {
        let current: BTreeSet<String> = state.hazards.keys().cloned().collect();
        if current != hazard_names {
            errors.push(format!("{} hazard keys differ from the declared state schema", state.id));
        }
    }
    if hazard_names.is_empty() {
        errors.push("state space must declare at least one hazard variable".to_owned());
    }

    let mut candidate_ids = BTreeSet::new();
    for candidate in &fixture.candidates {
        if candidate.id.trim().is_empty() {
            errors.push("candidate ID must not be blank".to_owned());
        }
        if !candidate_ids.insert(candidate.id.clone()) {
            errors.push(format!("duplicate candidate ID {:?}", candidate.id));
        }
        let mut blocked = BTreeSet::new();
        for hazard in &candidate.blocked_if_any_hazard_present {
            if !hazard_names.contains(hazard) {
                errors.push(format!("{} references undeclared hazard {:?}", candidate.id, hazard));
            }
            if !blocked.insert(hazard) {
                errors.push(format!("{} repeats blocked hazard {:?}", candidate.id, hazard));
            }
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &fixture.actions {
        if action.id.trim().is_empty() {
            errors.push("action ID must not be blank".to_owned());
        }
        if !action_ids.insert(action.id.clone()) {
            errors.push(format!("duplicate action ID {:?}", action.id));
        }
        if action.cost_units == 0 {
            errors.push(format!("{} action cost_units must be positive for ranking", action.id));
        }
        if action.duration_hours == 0 {
            errors.push(format!("{} duration_hours must be positive", action.id));
        }
        if action.description.trim().is_empty() {
            errors.push(format!("{} description must not be blank", action.id));
        }
        if action.outcomes.is_empty() {
            errors.push(format!("{} must declare outcomes", action.id));
        }
        let mut outcome_ids = BTreeSet::new();
        let mut state_assignments: BTreeMap<String, usize> = BTreeMap::new();
        for outcome in &action.outcomes {
            if outcome.id.trim().is_empty() {
                errors.push(format!("{} has blank outcome ID", action.id));
            }
            if !outcome_ids.insert(outcome.id.clone()) {
                errors.push(format!("{} has duplicate outcome ID {:?}", action.id, outcome.id));
            }
            if outcome.state_ids.is_empty() {
                errors.push(format!(
                    "{} outcome {} must cover at least one state",
                    action.id, outcome.id
                ));
            }
            let mut local_ids = BTreeSet::new();
            for state_id in &outcome.state_ids {
                if !state_ids.contains(state_id) {
                    errors.push(format!(
                        "{} outcome {} references undeclared state {:?}",
                        action.id, outcome.id, state_id
                    ));
                }
                if !local_ids.insert(state_id) {
                    errors.push(format!(
                        "{} outcome {} repeats state {:?}",
                        action.id, outcome.id, state_id
                    ));
                }
                *state_assignments.entry(state_id.clone()).or_insert(0) += 1;
            }
        }
        for state_id in &state_ids {
            match state_assignments.get(state_id).copied().unwrap_or(0) {
                0 => errors.push(format!("{} outcomes omit state {:?}", action.id, state_id)),
                1 => {}
                count => errors.push(format!(
                    "{} outcomes assign state {:?} {count} times",
                    action.id, state_id
                )),
            }
        }
        for assigned_id in state_assignments.keys() {
            if !state_ids.contains(assigned_id) {
                errors.push(format!(
                    "{} contains assignment for unknown state {:?}",
                    action.id, assigned_id
                ));
            }
        }
    }

    // Every state needs a feasible alternative and the initial uncertainty set needs
    // one robust-feasible baseline. Reject rather than panic if either is absent.
    for state in &fixture.state_space {
        if fixture.candidates.iter().all(|candidate| !candidate_is_feasible(candidate, state)) {
            errors.push(format!("no candidate is feasible in declared state {:?}", state.id));
        }
    }
    let all_state_ids: Vec<String> = fixture
        .state_space
        .iter()
        .map(|state| state.id.clone())
        .collect();
    if !all_state_ids.is_empty() && choose_robust_candidate(fixture, &all_state_ids).is_none() {
        errors.push("no robust-feasible baseline candidate spans the full state space".to_owned());
    }
    errors
}

fn state_by_id<'a>(fixture: &'a DecisionFixture, id: &str) -> Option<&'a SiteState> {
    fixture
        .state_space
        .iter()
        .find(|state| state.id == id)
}

fn candidate_is_feasible(candidate: &SiteCandidate, state: &SiteState) -> bool {
    candidate.blocked_if_any_hazard_present.iter().all(|hazard| {
        state.hazards.get(hazard).copied() == Some(false)
    })
}

fn choose_robust_candidate<'a>(
    fixture: &'a DecisionFixture,
    possible_state_ids: &[String],
) -> Option<&'a SiteCandidate> {
    fixture
        .candidates
        .iter()
        .filter(|candidate| {
            possible_state_ids.iter().all(|state_id| {
                state_by_id(fixture, state_id)
                    .is_some_and(|state| candidate_is_feasible(candidate, state))
            })
        })
        .min_by(|a, b| a.cost_units.cmp(&b.cost_units).then_with(|| a.id.cmp(&b.id)))
}

fn best_candidate_cost_for_state(fixture: &DecisionFixture, state: &SiteState) -> Option<u64> {
    fixture
        .candidates
        .iter()
        .filter(|candidate| candidate_is_feasible(candidate, state))
        .map(|candidate| candidate.cost_units)
        .min()
}

fn worst_regret_for_selected_candidate(
    fixture: &DecisionFixture,
    candidate: &SiteCandidate,
    possible_state_ids: &[String],
) -> Option<u64> {
    let mut worst: Option<u64> = None;
    for state_id in possible_state_ids {
        let state = state_by_id(fixture, state_id)?;
        let state_best = best_candidate_cost_for_state(fixture, state)?;
        let regret = candidate.cost_units.checked_sub(state_best)?;
        worst = Some(worst.unwrap_or(0).max(regret));
    }
    worst
}

fn evaluate_action(
    fixture: &DecisionFixture,
    action: &SurveyAction,
    baseline: &SiteCandidate,
    baseline_regret: u64,
) -> Result<ActionScore, Vec<String>> {
    let mut outcome_decisions = Vec::new();
    let mut worst_case_regret_after = 0u64;
    let mut max_residual_state_count = 0usize;
    let mut outcomes_changing_baseline_choice = 0usize;
    let mut max_candidate_cost = 0u64;

    let mut outcomes = action.outcomes.clone();
    outcomes.sort_by(|a, b| a.id.cmp(&b.id));
    for outcome in outcomes {
        let mut possible_state_ids = outcome.state_ids.clone();
        possible_state_ids.sort();
        let selected = choose_robust_candidate(fixture, &possible_state_ids).ok_or_else(|| {
            vec![format!("{} outcome {} has no robust-feasible candidate", action.id, outcome.id)]
        })?;
        let regret = worst_regret_for_selected_candidate(fixture, selected, &possible_state_ids)
            .ok_or_else(|| {
                vec![format!(
                    "{} outcome {} has no finite regret result",
                    action.id, outcome.id
                )]
            })?;
        worst_case_regret_after = worst_case_regret_after.max(regret);
        max_residual_state_count = max_residual_state_count.max(possible_state_ids.len());
        max_candidate_cost = max_candidate_cost.max(selected.cost_units);
        if selected.id != baseline.id {
            outcomes_changing_baseline_choice += 1;
        }
        outcome_decisions.push(OutcomeDecision {
            outcome_id: outcome.id,
            possible_state_ids,
            selected_candidate_id: selected.id.clone(),
            selected_candidate_cost_units: selected.cost_units,
            worst_case_regret_units: regret,
        });
    }

    let regret_reduction = baseline_regret.saturating_sub(worst_case_regret_after);
    let rate_milli = ((regret_reduction as u128 * 1000) / action.cost_units as u128) as u64;
    let total_cost = action.cost_units.checked_add(max_candidate_cost).ok_or_else(|| {
        vec![format!("{} total-cost arithmetic overflow", action.id)]
    })?;

    Ok(ActionScore {
        action_id: action.id.clone(),
        action_cost_units: action.cost_units,
        duration_hours: action.duration_hours,
        outcome_count: outcome_decisions.len(),
        max_residual_state_count,
        outcomes_changing_baseline_choice,
        worst_case_regret_after_units: worst_case_regret_after,
        worst_case_regret_reduction_units: regret_reduction,
        regret_reduction_per_cost_unit_milli: rate_milli,
        worst_case_total_cost_units: total_cost,
        outcome_decisions,
    })
}

fn compare_action_scores(a: &ActionScore, b: &ActionScore) -> Ordering {
    // Compare ratios by cross multiplication instead of rounded floating-point values.
    let left = a.worst_case_regret_reduction_units as u128 * b.action_cost_units as u128;
    let right = b.worst_case_regret_reduction_units as u128 * a.action_cost_units as u128;
    right.cmp(&left)
        .then_with(|| a.worst_case_regret_after_units.cmp(&b.worst_case_regret_after_units))
        .then_with(|| a.action_cost_units.cmp(&b.action_cost_units))
        .then_with(|| a.duration_hours.cmp(&b.duration_hours))
        .then_with(|| a.action_id.cmp(&b.action_id))
}

fn fixture_sha256(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn pinned_fixture_bytes() -> &'static [u8] {
        include_bytes!("../../../docs/research/fixtures/HABITAT_DECISION_IMPACT_V0_1.json")
    }

    fn fixture_value() -> Value {
        serde_json::from_slice(pinned_fixture_bytes()).unwrap()
    }

    fn parse_value(value: Value) -> DecisionFixture {
        serde_json::from_value(value).unwrap()
    }

    // Independent, intentionally brute-force reference oracle. It enumerates all
    // candidate/state combinations instead of calling the production selector.
    fn oracle_robust_candidate<'a>(
        fixture: &'a DecisionFixture,
        possible_states: &[String],
    ) -> Option<&'a SiteCandidate> {
        let mut feasible = Vec::new();
        for candidate in &fixture.candidates {
            let mut passes_every_state = true;
            for state_id in possible_states {
                let state = fixture
            .state_space
            .iter()
            .find(|entry| entry.id == *state_id)?;
                for hazard in &candidate.blocked_if_any_hazard_present {
                    if state.hazards.get(hazard).copied().unwrap_or(true) {
                        passes_every_state = false;
                    }
                }
            }
            if passes_every_state {
                feasible.push(candidate);
            }
        }
        feasible.sort_by(|left, right| {
            left.cost_units
                .cmp(&right.cost_units)
                .then_with(|| left.id.cmp(&right.id))
        });
        feasible.into_iter().next()
    }

    fn oracle_state_best_cost(fixture: &DecisionFixture, state_id: &str) -> Option<u64> {
        let state = fixture
            .state_space
            .iter()
            .find(|entry| entry.id == state_id)?;
        let mut feasible_costs = Vec::new();
        for candidate in &fixture.candidates {
            let mut candidate_ok = true;
            for hazard in &candidate.blocked_if_any_hazard_present {
                match state.hazards.get(hazard) {
                    Some(false) => {}
                    Some(true) | None => candidate_ok = false,
                }
            }
            if candidate_ok {
                feasible_costs.push(candidate.cost_units);
            }
        }
        feasible_costs.into_iter().min()
    }

    fn oracle_action_regret(
        fixture: &DecisionFixture,
        action: &SurveyAction,
    ) -> Option<(u64, Vec<(String, String)>)> {
        let mut worst = 0u64;
        let mut choices = Vec::new();
        for outcome in &action.outcomes {
            let chosen = oracle_robust_candidate(fixture, &outcome.state_ids)?;
            choices.push((outcome.id.clone(), chosen.id.clone()));
            for state_id in &outcome.state_ids {
                let best = oracle_state_best_cost(fixture, state_id)?;
                let regret = chosen.cost_units.checked_sub(best)?;
                worst = worst.max(regret);
            }
        }
        choices.sort();
        Some((worst, choices))
    }

    #[test]
    fn verifies_exact_fixture_bytes_and_digest() {
        let report = verify_fixture_bytes(pinned_fixture_bytes()).unwrap();
        assert_eq!(report.input_sha256, FIXTURE_SHA256);
        assert_eq!(report.state_count, 4);
        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.action_count, 3);
        assert_eq!(report.baseline_robust_candidate_id, "inland_fallback");
        assert_eq!(report.baseline_worst_case_regret_units, 120);
    }

    #[test]
    fn ranked_actions_match_frozen_synthetic_expected_values() {
        let report = verify_fixture_bytes(pinned_fixture_bytes()).unwrap();
        let ranked: Vec<&str> = report
            .ranked_actions
            .iter()
            .map(|action| action.action_id.as_str())
            .collect();
        assert_eq!(
            ranked,
            vec!["A-COMBINED-SURVEY", "A-FLOOD-SURVEY", "A-GEOTECH-SURVEY"]
        );

        let combined = &report.ranked_actions[0];
        assert_eq!(combined.worst_case_regret_after_units, 0);
        assert_eq!(combined.worst_case_regret_reduction_units, 120);
        assert_eq!(combined.regret_reduction_per_cost_unit_milli, 4800);
        assert_eq!(combined.max_residual_state_count, 1);
        assert_eq!(combined.worst_case_total_cost_units, 245);

        let flood = &report.ranked_actions[1];
        assert_eq!(flood.worst_case_regret_after_units, 105);
        assert_eq!(flood.worst_case_regret_reduction_units, 15);
        assert_eq!(flood.regret_reduction_per_cost_unit_milli, 3000);
        assert_eq!(flood.max_residual_state_count, 2);
        assert_eq!(flood.worst_case_total_cost_units, 225);

        let geo = &report.ranked_actions[2];
        assert_eq!(geo.worst_case_regret_after_units, 120);
        assert_eq!(geo.worst_case_regret_reduction_units, 0);
        assert_eq!(geo.regret_reduction_per_cost_unit_milli, 0);
        assert_eq!(geo.worst_case_total_cost_units, 228);
    }

    #[test]
    fn brute_force_reference_matches_every_action_and_outcome() {
        let fixture: DecisionFixture = serde_json::from_slice(pinned_fixture_bytes()).unwrap();
        let report = evaluate_fixture(&fixture, fixture_sha256(pinned_fixture_bytes())).unwrap();
        let baseline_ids: Vec<String> = fixture
            .state_space
            .iter()
            .map(|state| state.id.clone())
            .collect();
        let baseline = oracle_robust_candidate(&fixture, &baseline_ids).unwrap();
        let base_regret = fixture
            .state_space
            .iter()
            .map(|state| {
                let best = oracle_state_best_cost(&fixture, &state.id).unwrap();
                baseline.cost_units - best
            })
            .max()
            .unwrap();
        assert_eq!(base_regret, report.baseline_worst_case_regret_units);

        for production in &report.ranked_actions {
            let action = fixture
                .actions
                .iter()
                .find(|action| action.id == production.action_id)
                .unwrap();
            let (oracle_regret, oracle_choices) = oracle_action_regret(&fixture, action).unwrap();
            assert_eq!(production.worst_case_regret_after_units, oracle_regret);
            let production_choices: Vec<(String, String)> = production
                .outcome_decisions
                .iter()
                .map(|outcome| {
                    (
                        outcome.outcome_id.clone(),
                        outcome.selected_candidate_id.clone(),
                    )
                })
                .collect();
            assert_eq!(production_choices, oracle_choices);
        }
    }

    #[test]
    fn rejects_modified_bytes_even_if_json_still_parses() {
        let mut bytes = pinned_fixture_bytes().to_vec();
        bytes.push(b' ');
        let errors = verify_fixture_bytes(&bytes).unwrap_err();
        assert!(errors[0].contains("digest mismatch"));
    }

    #[test]
    fn rejects_undeclared_hazards() {
        let mut value = fixture_value();
        value["candidates"][0]["blocked_if_any_hazard_present"]
            .as_array_mut()
            .unwrap()
            .push(json!("imaginary_hazard"));
        let fixture = parse_value(value);
        let errors = validate_fixture(&fixture);
        assert!(errors.iter().any(|error| error.contains("undeclared hazard")));
    }

    #[test]
    fn rejects_missing_state_from_an_action_partition() {
        let mut value = fixture_value();
        value["actions"][0]["outcomes"][0]["state_ids"]
            .as_array_mut()
            .unwrap()
            .pop();
        let errors = validate_fixture(&parse_value(value));
        assert!(errors.iter().any(|error| error.contains("outcomes omit state")));
    }

    #[test]
    fn rejects_undeclared_state_in_action_partition() {
        let mut value = fixture_value();
        value["actions"][0]["outcomes"][0]["state_ids"][0] = json!("missing-state");
        let errors = validate_fixture(&parse_value(value));
        assert!(errors.iter().any(|error| error.contains("references undeclared state")));
        assert!(errors.iter().any(|error| error.contains("outcomes omit state")));
    }

    #[test]
    fn rejects_overlapping_action_outcomes() {
        let mut value = fixture_value();
        value["actions"][0]["outcomes"][1]["state_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("clear"));
        let errors = validate_fixture(&parse_value(value));
        assert!(errors
            .iter()
            .any(|error| error.contains("assign state \"clear\" 2 times")));
    }

    #[test]
    fn rejects_duplicate_ids_and_unknown_fields() {
        let mut value = fixture_value();
        value["candidates"][1]["id"] = json!("canyon_residence");
        let errors = validate_fixture(&parse_value(value)).into_iter().collect::<Vec<_>>();
        assert!(errors.iter().any(|error| error.contains("duplicate candidate ID")));

        let mut unknown = fixture_value();
        unknown["unreviewed_field"] = json!(true);
        let error = serde_json::from_value::<DecisionFixture>(unknown).unwrap_err().to_string();
        assert!(error.contains("unknown field"));
    }

    #[test]
    fn fails_if_any_state_has_no_feasible_candidate() {
        let mut value = fixture_value();
        value["candidates"][2]["blocked_if_any_hazard_present"] = json!([
            "canyon_flood", "ridge_rock_unstable"
        ]);
        let errors = validate_fixture(&parse_value(value));
        assert!(errors.iter().any(|error| error.contains("no candidate is feasible")));
    }

    #[test]
    fn permutation_of_states_candidates_actions_and_outcomes_preserves_decision() {
        let fixture: DecisionFixture = serde_json::from_slice(pinned_fixture_bytes()).unwrap();
        let original = evaluate_fixture(&fixture, "same-test-digest".to_owned()).unwrap();
        let mut permuted = fixture.clone();
        permuted.state_space.reverse();
        permuted.candidates.reverse();
        permuted.actions.reverse();
        for action in &mut permuted.actions {
            action.outcomes.reverse();
            for outcome in &mut action.outcomes {
                outcome.state_ids.reverse();
            }
        }
        let mut altered = evaluate_fixture(&permuted, "different-test-digest".to_owned()).unwrap();
        altered.input_sha256 = original.input_sha256.clone();
        assert_eq!(altered, original);
    }

    #[test]
    fn standard_sha256_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
