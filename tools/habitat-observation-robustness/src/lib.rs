// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const REPORT_SCHEMA: &str = "luminous.habitat_observation_report.v0.2";
pub const CLAIM_CEILING: &str = "synthetic_observation_robustness_not_site_safety";
const FIXTURE_SHA256: &str = "20da58685f63743f79e4aef94e8df528e7495bffb2103f0ccaa2471e2bf173f0";
const REQUIRED_MODES: [&str; 6] = [
    "valid",
    "false_negative",
    "false_positive",
    "stale",
    "inconclusive",
    "contradictory",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fixture {
    pub schema_version: String,
    pub fixture_id: String,
    pub status: String,
    pub synthetic_only: bool,
    pub claim_ceiling: String,
    pub intended_use: String,
    pub hazard_names: Vec<String>,
    pub conditions: Vec<SiteCondition>,
    pub observation_modes: Vec<String>,
    pub candidates: Vec<Candidate>,
    pub actions: Vec<ObservationAction>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SiteCondition {
    pub id: String,
    pub hazards: BTreeMap<String, bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub cost_units: u64,
    pub blocked_if_any_hazard_present: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationAction {
    pub id: String,
    pub measured_hazard: String,
    pub cost_units: u64,
    pub description: String,
    pub outcomes: Vec<ObservationOutcome>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ObservationOutcome {
    pub id: String,
    pub classification: String,
    pub state_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeReport {
    pub outcome_id: String,
    pub classification: String,
    pub compatible_state_count: usize,
    pub robust_feasible_candidate_ids: Vec<String>,
    pub selected_candidate_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionReport {
    pub action_id: String,
    pub measured_hazard: String,
    pub action_cost_units: u64,
    pub outcome_count: usize,
    pub outcomes: Vec<OutcomeReport>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenchmarkReport {
    pub schema_version: String,
    pub fixture_id: String,
    pub input_sha256: String,
    pub verification_status: String,
    pub claim_ceiling: String,
    pub condition_count: usize,
    pub observation_mode_count: usize,
    pub composite_state_count: usize,
    pub candidate_count: usize,
    pub action_count: usize,
    pub adversarial_mode_state_action_count: usize,
    pub false_negative_control_count: usize,
    pub unresolved_outcome_count: usize,
    pub false_safe_control_count: usize,
    pub actions: Vec<ActionReport>,
}

#[derive(Clone, Debug)]
struct CompositeState {
    id: String,
    condition_id: String,
    observation_mode: String,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn composite_state_id(condition_id: &str, mode: &str) -> String {
    format!("{condition_id}::{mode}")
}

fn composite_states(fixture: &Fixture) -> Vec<CompositeState> {
    let mut states = Vec::new();
    for condition in &fixture.conditions {
        for mode in &fixture.observation_modes {
            states.push(CompositeState {
                id: composite_state_id(&condition.id, mode),
                condition_id: condition.id.clone(),
                observation_mode: mode.clone(),
            });
        }
    }
    states
}

fn expected_classification(mode: &str, hazard_present: bool) -> &'static str {
    match mode {
        "stale" | "inconclusive" | "contradictory" => "unresolved",
        "false_negative" => "negative",
        "false_positive" => "positive",
        "valid" if hazard_present => "positive",
        "valid" => "negative",
        _ => "invalid",
    }
}

fn candidate_is_feasible(candidate: &Candidate, condition: &SiteCondition) -> bool {
    candidate.blocked_if_any_hazard_present.iter().all(|hazard| {
        condition.hazards.get(hazard).copied() == Some(false)
    })
}

/// Verifies exact fixture bytes before interpreting the synthetic observation model.
pub fn verify_fixture_bytes(bytes: &[u8]) -> Result<BenchmarkReport, Vec<String>> {
    let digest = sha256_hex(bytes);
    if digest != FIXTURE_SHA256 {
        return Err(vec![format!(
            "frozen observation fixture digest mismatch: expected {FIXTURE_SHA256}, got {digest}"
        )]);
    }
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| vec![format!("observation fixture rejected: {error}")])?;
    evaluate_fixture(&fixture, digest)
}

fn evaluate_fixture(
    fixture: &Fixture,
    input_sha256: String,
) -> Result<BenchmarkReport, Vec<String>> {
    let errors = validate_fixture(fixture);
    if !errors.is_empty() {
        return Err(errors);
    }

    let states = composite_states(fixture);
    let condition_by_id: BTreeMap<&str, &SiteCondition> = fixture
        .conditions
        .iter()
        .map(|condition| (condition.id.as_str(), condition))
        .collect();
    let mut action_reports = Vec::new();
    let mut false_safe_control_count = 0usize;
    let mut unresolved_outcome_count = 0usize;
    let mut false_negative_control_count = 0usize;

    for action in &fixture.actions {
        let mut outcome_reports = Vec::new();
        for outcome in &action.outcomes {
            let compatible_states: Vec<&CompositeState> = outcome
                .state_ids
                .iter()
                .filter_map(|id| states.iter().find(|state| state.id == *id))
                .collect();
            let mut robust_feasible: Vec<&Candidate> = fixture
                .candidates
                .iter()
                .filter(|candidate| {
                    compatible_states.iter().all(|state| {
                        condition_by_id
                            .get(state.condition_id.as_str())
                            .is_some_and(|condition| candidate_is_feasible(candidate, condition))
                    })
                })
                .collect();
            robust_feasible.sort_by(|left, right| {
                left.cost_units
                    .cmp(&right.cost_units)
                    .then_with(|| left.id.cmp(&right.id))
            });
            let selected = robust_feasible.first().copied();

            if outcome.classification == "unresolved" {
                unresolved_outcome_count += 1;
            }
            for state in &compatible_states {
                if state.observation_mode == "false_negative"
                    && condition_by_id
                        .get(state.condition_id.as_str())
                        .and_then(|condition| condition.hazards.get(&action.measured_hazard))
                        .copied()
                        == Some(true)
                {
                    false_negative_control_count += 1;
                }
                if let Some(candidate) = selected {
                    if let Some(condition) = condition_by_id.get(state.condition_id.as_str()) {
                        if !candidate_is_feasible(candidate, condition) {
                            false_safe_control_count += 1;
                        }
                    }
                }
            }

            outcome_reports.push(OutcomeReport {
                outcome_id: outcome.id.clone(),
                classification: outcome.classification.clone(),
                compatible_state_count: compatible_states.len(),
                robust_feasible_candidate_ids: robust_feasible
                    .iter()
                    .map(|candidate| candidate.id.clone())
                    .collect(),
                selected_candidate_id: selected.map(|candidate| candidate.id.clone()),
            });
        }
        outcome_reports.sort_by(|left, right| {
            left.classification
                .cmp(&right.classification)
                .then_with(|| left.outcome_id.cmp(&right.outcome_id))
        });
        action_reports.push(ActionReport {
            action_id: action.id.clone(),
            measured_hazard: action.measured_hazard.clone(),
            action_cost_units: action.cost_units,
            outcome_count: outcome_reports.len(),
            outcomes: outcome_reports,
        });
    }
    action_reports.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let adversarial_mode_count = fixture
        .observation_modes
        .iter()
        .filter(|mode| mode.as_str() != "valid")
        .count();
    Ok(BenchmarkReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        verification_status: "SYNTHETIC_OBSERVATION_SEMANTICS_ONLY_NOT_PHYSICAL_VALIDATION"
            .to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        condition_count: fixture.conditions.len(),
        observation_mode_count: fixture.observation_modes.len(),
        composite_state_count: states.len(),
        candidate_count: fixture.candidates.len(),
        action_count: fixture.actions.len(),
        adversarial_mode_state_action_count: adversarial_mode_count
            * fixture.conditions.len()
            * fixture.actions.len(),
        false_negative_control_count,
        unresolved_outcome_count,
        false_safe_control_count,
        actions: action_reports,
    })
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();
    if fixture.schema_version != "luminous.habitat_observation_fixture.v0.2" {
        errors.push(format!("unsupported schema_version {:?}", fixture.schema_version));
    }
    if fixture.fixture_id.trim().is_empty() {
        errors.push("fixture_id must not be blank".to_owned());
    }
    if fixture.status != "proposed_synthetic_observation_robustness_benchmark" {
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
    if fixture.conditions.len() != 4 {
        errors.push(format!("v0.2 requires 4 site conditions, got {}", fixture.conditions.len()));
    }
    if fixture.candidates.len() != 3 {
        errors.push(format!("v0.2 requires 3 candidates, got {}", fixture.candidates.len()));
    }
    if fixture.actions.len() != 2 {
        errors.push(format!("v0.2 requires 2 observation actions, got {}", fixture.actions.len()));
    }

    let hazard_names: BTreeSet<&str> = fixture.hazard_names.iter().map(String::as_str).collect();
    if hazard_names.len() != fixture.hazard_names.len() || hazard_names.is_empty() {
        errors.push("hazard_names must be non-empty and unique".to_owned());
    }
    let observed_modes: BTreeSet<&str> =
        fixture.observation_modes.iter().map(String::as_str).collect();
    let required_modes: BTreeSet<&str> = REQUIRED_MODES.into_iter().collect();
    if observed_modes != required_modes {
        errors.push(format!(
            "observation_modes must contain exactly the v0.2 adversarial modes {:?}",
            required_modes
        ));
    }

    let mut condition_ids = BTreeSet::new();
    for condition in &fixture.conditions {
        if condition.id.trim().is_empty() {
            errors.push("condition ID must not be blank".to_owned());
        }
        if !condition_ids.insert(condition.id.as_str()) {
            errors.push(format!("duplicate condition ID {:?}", condition.id));
        }
        let keys: BTreeSet<&str> = condition.hazards.keys().map(String::as_str).collect();
        if keys != hazard_names {
            errors.push(format!("{} hazard keys do not match hazard_names", condition.id));
        }
    }

    let mut candidate_ids = BTreeSet::new();
    for candidate in &fixture.candidates {
        if candidate.id.trim().is_empty() {
            errors.push("candidate ID must not be blank".to_owned());
        }
        if !candidate_ids.insert(candidate.id.as_str()) {
            errors.push(format!("duplicate candidate ID {:?}", candidate.id));
        }
        if candidate.cost_units == 0 {
            errors.push(format!("{} cost_units must be positive", candidate.id));
        }
        let mut blocked = BTreeSet::new();
        for hazard in &candidate.blocked_if_any_hazard_present {
            if !hazard_names.contains(hazard.as_str()) {
                errors.push(format!("{} references undeclared hazard {:?}", candidate.id, hazard));
            }
            if !blocked.insert(hazard.as_str()) {
                errors.push(format!("{} repeats blocked hazard {:?}", candidate.id, hazard));
            }
        }
    }
    for condition in &fixture.conditions {
        if fixture.candidates.iter().all(|candidate| {
            !candidate_is_feasible(candidate, condition)
        }) {
            errors.push(format!("no candidate is feasible in condition {:?}", condition.id));
        }
    }

    let states = composite_states(fixture);
    let state_ids: BTreeSet<&str> = states.iter().map(|state| state.id.as_str()).collect();
    let condition_by_id: BTreeMap<&str, &SiteCondition> = fixture
        .conditions
        .iter()
        .map(|condition| (condition.id.as_str(), condition))
        .collect();
    let mut action_ids = BTreeSet::new();
    for action in &fixture.actions {
        if action.id.trim().is_empty() {
            errors.push("action ID must not be blank".to_owned());
        }
        if !action_ids.insert(action.id.as_str()) {
            errors.push(format!("duplicate action ID {:?}", action.id));
        }
        if !hazard_names.contains(action.measured_hazard.as_str()) {
            errors.push(format!(
                "{} measures undeclared hazard {:?}",
                action.id, action.measured_hazard
            ));
        }
        if action.cost_units == 0 {
            errors.push(format!("{} cost_units must be positive", action.id));
        }
        if action.description.trim().is_empty() {
            errors.push(format!("{} description must not be blank", action.id));
        }

        let mut outcome_ids = BTreeSet::new();
        let mut classifications = BTreeSet::new();
        let mut assignments: BTreeMap<&str, (&str, &str)> = BTreeMap::new();
        for outcome in &action.outcomes {
            if outcome.id.trim().is_empty() {
                errors.push(format!("{} has a blank outcome ID", action.id));
            }
            if !outcome_ids.insert(outcome.id.as_str()) {
                errors.push(format!("{} has duplicate outcome ID {:?}", action.id, outcome.id));
            }
            if !["positive", "negative", "unresolved"].contains(&outcome.classification.as_str()) {
                errors.push(format!(
                    "{} outcome {} has unsupported classification {:?}",
                    action.id, outcome.id, outcome.classification
                ));
            }
            if !classifications.insert(outcome.classification.as_str()) {
                errors.push(format!(
                    "{} repeats classification {:?}",
                    action.id, outcome.classification
                ));
            }
            if outcome.state_ids.is_empty() {
                errors.push(format!("{} outcome {} is empty", action.id, outcome.id));
            }
            let mut local_ids = BTreeSet::new();
            for state_id in &outcome.state_ids {
                if !state_ids.contains(state_id.as_str()) {
                    errors.push(format!(
                        "{} outcome {} references undeclared state {:?}",
                        action.id, outcome.id, state_id
                    ));
                    continue;
                }
                if !local_ids.insert(state_id.as_str()) {
                    errors.push(format!(
                        "{} outcome {} repeats state {:?}",
                        action.id, outcome.id, state_id
                    ));
                }
                if assignments
                    .insert(
                        state_id.as_str(),
                        (outcome.classification.as_str(), outcome.id.as_str()),
                    )
                    .is_some()
                {
                    errors.push(format!(
                        "{} assigns state {:?} to multiple outcomes",
                        action.id, state_id
                    ));
                }
            }
        }
        let expected_classifications: BTreeSet<&str> =
            ["positive", "negative", "unresolved"].into_iter().collect();
        if classifications != expected_classifications {
            errors.push(format!(
                "{} must include positive, negative and unresolved outcomes",
                action.id
            ));
        }
        for state in &states {
            if !assignments.contains_key(state.id.as_str()) {
                errors.push(format!("{} omits composite state {:?}", action.id, state.id));
                continue;
            }
            let Some((actual_classification, _)) =
                assignments.get(state.id.as_str()).copied()
            else {
                continue;
            };
            let expected = condition_by_id
                .get(state.condition_id.as_str())
                .and_then(|condition| condition.hazards.get(&action.measured_hazard))
                .map(|hazard| expected_classification(&state.observation_mode, *hazard))
                .unwrap_or("invalid");
            if actual_classification != expected {
                errors.push(format!(
                    "{} maps state {:?} to {:?}, expected {:?} for mode {:?}",
                    action.id, state.id, actual_classification, expected, state.observation_mode
                ));
            }
        }
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const FIXTURE_BYTES: &[u8] = include_bytes!(
        "../../../docs/research/fixtures/HABITAT_OBSERVATION_ROBUSTNESS_V0_2.json"
    );

    #[test]
    fn frozen_fixture_has_expected_adversarial_census_and_zero_false_safe_controls() {
        let report = verify_fixture_bytes(FIXTURE_BYTES).expect("frozen fixture should verify");
        assert_eq!(report.input_sha256, FIXTURE_SHA256);
        assert_eq!(report.condition_count, 4);
        assert_eq!(report.observation_mode_count, 6);
        assert_eq!(report.composite_state_count, 24);
        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.action_count, 2);
        assert_eq!(report.adversarial_mode_state_action_count, 40);
        assert_eq!(report.false_negative_control_count, 4);
        assert_eq!(report.unresolved_outcome_count, 2);
        assert_eq!(report.false_safe_control_count, 0);
        for action in report.actions {
            assert_eq!(action.outcome_count, 3);
            for outcome in action.outcomes {
                assert_eq!(outcome.selected_candidate_id.as_deref(), Some("inland_fallback"));
                assert_eq!(
                    outcome.robust_feasible_candidate_ids,
                    vec!["inland_fallback".to_owned()]
                );
            }
        }
    }

    #[test]
    fn exact_fixture_byte_mutation_is_rejected() {
        let mut changed = FIXTURE_BYTES.to_vec();
        changed.push(b' ');
        let errors = verify_fixture_bytes(&changed).expect_err("changed bytes must be rejected");
        assert!(errors.iter().any(|error| error.contains("digest mismatch")));
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        value["unreviewed_safety_override"] = serde_json::Value::Bool(true);
        let parsed: Result<Fixture, _> = serde_json::from_value(value);
        assert!(parsed.is_err());
    }

    #[test]
    fn false_negative_cannot_be_treated_as_clear_evidence() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        let action = fixture.actions.iter().find(|action| action.id == "A-FLOOD-SURVEY").unwrap();
        let false_negative_state = "S-CANYON-FLOOD::false_negative";
        let outcome = action.outcomes.iter().find(|outcome| {
            outcome.state_ids.iter().any(|id| id == false_negative_state)
        }).unwrap();
        assert_eq!(outcome.classification, "negative");
        let report = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        let action_report = report.actions.iter().find(|item| item.action_id == action.id).unwrap();
        let outcome_report = action_report
            .outcomes
            .iter()
            .find(|item| item.outcome_id == outcome.id)
            .unwrap();
        assert_eq!(outcome_report.selected_candidate_id.as_deref(), Some("inland_fallback"));
    }

    #[test]
    fn stale_inconclusive_and_contradictory_results_remain_unresolved() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        for action in &fixture.actions {
            for mode in ["stale", "inconclusive", "contradictory"] {
                for condition in &fixture.conditions {
                    let id = composite_state_id(&condition.id, mode);
                    let outcome = action.outcomes.iter().find(|outcome| {
                        outcome.state_ids.iter().any(|state_id| state_id == &id)
                    }).unwrap();
                    assert_eq!(outcome.classification, "unresolved");
                }
            }
        }
    }

    #[test]
    fn incomplete_or_overlapping_partitions_are_rejected() {
        let mut fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        fixture.actions[0].outcomes[0].state_ids.pop();
        assert!(!validate_fixture(&fixture).is_empty());

        let mut fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        let state = fixture.actions[0].outcomes[0].state_ids[0].clone();
        fixture.actions[0].outcomes[1].state_ids.push(state);
        assert!(!validate_fixture(&fixture).is_empty());
    }

    #[test]
    fn independent_exhaustive_oracle_agrees_on_robust_candidate_sets() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        let report = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        // Deliberately direct test oracle: enumerate conditions for each raw outcome and
        // check every blocked hazard without calling candidate_is_feasible.
        for action in &fixture.actions {
            let action_report = report.actions.iter().find(|r| r.action_id == action.id).unwrap();
            for outcome in &action.outcomes {
                let mut expected = Vec::new();
                for candidate in &fixture.candidates {
                    let mut safe_for_every_state = true;
                    for condition in &fixture.conditions {
                        let matched = outcome.state_ids.iter().any(|id| {
                            id.starts_with(&format!("{}::", condition.id))
                        });
                        if !matched {
                            continue;
                        }
                        for hazard in &candidate.blocked_if_any_hazard_present {
                            if condition.hazards.get(hazard).copied().unwrap_or(true) {
                                safe_for_every_state = false;
                            }
                        }
                    }
                    if safe_for_every_state {
                        expected.push((candidate.cost_units, candidate.id.clone()));
                    }
                }
                expected.sort();
                let expected_ids: Vec<String> = expected.into_iter().map(|(_, id)| id).collect();
                let actual = action_report.outcomes.iter()
                    .find(|item| item.outcome_id == outcome.id).unwrap();
                assert_eq!(actual.robust_feasible_candidate_ids, expected_ids);
            }
        }
    }

    #[test]
    fn irrelevant_input_order_does_not_change_report() {
        let fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        let baseline = evaluate_fixture(&fixture, FIXTURE_SHA256.to_owned()).unwrap();
        let mut reordered = fixture.clone();
        reordered.conditions.reverse();
        reordered.observation_modes.reverse();
        reordered.actions.reverse();
        reordered.candidates.reverse();
        for action in &mut reordered.actions {
            action.outcomes.reverse();
            for outcome in &mut action.outcomes {
                outcome.state_ids.reverse();
            }
        }
        let reordered_report = evaluate_fixture(&reordered, FIXTURE_SHA256.to_owned()).unwrap();
        assert_eq!(baseline, reordered_report);
    }

    #[test]
    fn invalid_mode_semantics_fail_closed() {
        let mut fixture: Fixture = serde_json::from_slice(FIXTURE_BYTES).unwrap();
        let action = fixture
            .actions
            .iter_mut()
            .find(|action| action.id == "A-FLOOD-SURVEY")
            .unwrap();
        let outcome_index = action.outcomes.iter().position(|outcome| {
            outcome.state_ids.iter().any(|id| id == "S-CANYON-FLOOD::false_negative")
        }).unwrap();
        action.outcomes[outcome_index].classification = "positive".to_owned();
        assert!(!validate_fixture(&fixture).is_empty());
    }
}
