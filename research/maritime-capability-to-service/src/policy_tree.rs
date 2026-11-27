//! Multi-epoch non-anticipativity validation for finite scenario policy trees.
//!
//! This module validates a supplied policy tree. It does not optimize policies,
//! model scenario probabilities, authenticate evidence digests, or authorize actions.

use std::collections::{BTreeMap, BTreeSet};

use crate::kite::Gate;
use crate::kite_oracle::DecisionContext;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTreeAction {
    pub action_id: String,
    pub admissibility: Gate,
    pub hard_constraints_passed: bool,
    pub protected_floor_breach_unit_hours: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannedStage {
    pub decision_epoch: u64,
    pub decision_cutoff: u64,
    /// SHA-256 commitment to the complete canonical observable evidence state.
    /// The caller must include all decision-relevant payload/value digests,
    /// rights, authority, freshness and admissibility dispositions.
    pub observable_state_digest: String,
    pub available_actions: Vec<PolicyTreeAction>,
    pub selected_action_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioPolicyPath {
    pub scenario_id: String,
    pub context: DecisionContext,
    pub stages: Vec<PlannedStage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PolicyInformationSetKey {
    pub decision_epoch: u64,
    pub decision_cutoff: u64,
    pub observable_state_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyInformationSetSummary {
    pub key: PolicyInformationSetKey,
    pub scenario_ids: Vec<String>,
    pub selected_action_id: String,
    pub action_catalog: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyTreeReport {
    pub scenario_count: usize,
    pub stage_decision_count: usize,
    pub information_set_count: usize,
    pub shared_information_set_count: usize,
    pub maximum_scenarios_per_information_set: usize,
    pub information_sets: Vec<PolicyInformationSetSummary>,
}

#[derive(Clone, Debug)]
struct InformationSetAccumulator {
    scenario_ids: Vec<String>,
    selected_action_id: String,
    action_catalog: Vec<String>,
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Validate a supplied policy tree for temporal consistency, action feasibility,
/// and non-anticipativity at every epoch.
///
/// The information-set key is (decision_epoch, decision_cutoff,
/// observable_state_digest). Scenarios in the same set must have the same
/// action catalogue and select the same action. Later stages may diverge after
/// their committed observable evidence states diverge.
///
/// The digest is validated for encoding and used as a grouping commitment; its
/// canonical construction and authenticity are the responsibility of the
/// evidence-owning adapter.
pub fn validate_non_anticipative_policy_tree(
    paths: &[ScenarioPolicyPath],
) -> Result<PolicyTreeReport, String> {
    let Some(first_path) = paths.first() else {
        return Err("policy tree must contain at least one scenario path".to_owned());
    };

    validate_context(&first_path.context)?;
    if first_path.scenario_id.trim().is_empty() {
        return Err("scenario IDs must be non-empty".to_owned());
    }
    if first_path.stages.is_empty() {
        return Err(format!(
            "{}: scenario path must contain at least one decision stage",
            first_path.scenario_id
        ));
    }

    let expected_context = &first_path.context;
    let expected_schedule = stage_schedule(&first_path.stages, &first_path.scenario_id)?;

    let mut scenario_ids = BTreeSet::new();
    let mut information_sets: BTreeMap<
        PolicyInformationSetKey,
        InformationSetAccumulator,
    > = BTreeMap::new();
    let mut stage_decision_count = 0usize;

    for path in paths {
        if path.scenario_id.trim().is_empty() || !scenario_ids.insert(path.scenario_id.as_str()) {
            return Err("scenario IDs must be non-empty and unique".to_owned());
        }
        if path.context != *expected_context {
            return Err(format!(
                "{}: scenario context differs from the frozen service obligation",
                path.scenario_id
            ));
        }
        if path.stages.is_empty() {
            return Err(format!(
                "{}: scenario path must contain at least one decision stage",
                path.scenario_id
            ));
        }

        let schedule = stage_schedule(&path.stages, &path.scenario_id)?;
        if schedule != expected_schedule {
            return Err(format!(
                "{}: scenario decision epoch/cutoff schedule differs from the frozen tree",
                path.scenario_id
            ));
        }

        for stage in &path.stages {
            stage_decision_count = stage_decision_count
                .checked_add(1)
                .ok_or_else(|| "policy-tree stage decision count overflow".to_owned())?;

            if !is_sha256_digest(&stage.observable_state_digest) {
                return Err(format!(
                    "{} at epoch {}: observable-state digest must be 64 lowercase hexadecimal SHA-256 characters",
                    path.scenario_id, stage.decision_epoch
                ));
            }
            if stage.selected_action_id.trim().is_empty() {
                return Err(format!(
                    "{} at epoch {}: selected action ID must be non-empty",
                    path.scenario_id, stage.decision_epoch
                ));
            }
            if stage.available_actions.is_empty() {
                return Err(format!(
                    "{} at epoch {}: action catalogue must be non-empty",
                    path.scenario_id, stage.decision_epoch
                ));
            }

            let mut actions: BTreeMap<&str, &PolicyTreeAction> = BTreeMap::new();
            for action in &stage.available_actions {
                if action.action_id.trim().is_empty()
                    || actions.insert(action.action_id.as_str(), action).is_some()
                {
                    return Err(format!(
                        "{} at epoch {}: action IDs must be non-empty and unique",
                        path.scenario_id, stage.decision_epoch
                    ));
                }
            }

            let selected = actions.get(stage.selected_action_id.as_str()).ok_or_else(|| {
                format!(
                    "{} at epoch {}: selected action '{}' is absent from its action catalogue",
                    path.scenario_id, stage.decision_epoch, stage.selected_action_id
                )
            })?;

            if selected.admissibility != Gate::Yes
                || !selected.hard_constraints_passed
                || selected.protected_floor_breach_unit_hours > 0
            {
                return Err(format!(
                    "{} at epoch {}: selected action '{}' fails admissibility, hard constraints, or protected service floor",
                    path.scenario_id, stage.decision_epoch, stage.selected_action_id
                ));
            }

            let action_catalog: Vec<String> =
                actions.keys().map(|action_id| (*action_id).to_owned()).collect();
            let key = PolicyInformationSetKey {
                decision_epoch: stage.decision_epoch,
                decision_cutoff: stage.decision_cutoff,
                observable_state_digest: stage.observable_state_digest.clone(),
            };

            match information_sets.get_mut(&key) {
                Some(existing) => {
                    if existing.action_catalog != action_catalog {
                        return Err(format!(
                            "information set at epoch {} has inconsistent action catalogues",
                            stage.decision_epoch
                        ));
                    }
                    if existing.selected_action_id != stage.selected_action_id {
                        return Err(format!(
                            "non-anticipativity violation at epoch {}: indistinguishable scenarios selected '{}' and '{}'",
                            stage.decision_epoch,
                            existing.selected_action_id,
                            stage.selected_action_id
                        ));
                    }
                    existing.scenario_ids.push(path.scenario_id.clone());
                }
                None => {
                    information_sets.insert(
                        key,
                        InformationSetAccumulator {
                            scenario_ids: vec![path.scenario_id.clone()],
                            selected_action_id: stage.selected_action_id.clone(),
                            action_catalog,
                        },
                    );
                }
            }
        }
    }

    let mut shared_information_set_count = 0usize;
    let mut maximum_scenarios_per_information_set = 0usize;
    let mut summaries = Vec::with_capacity(information_sets.len());

    for (key, mut accumulator) in information_sets {
        accumulator.scenario_ids.sort();
        let scenario_count = accumulator.scenario_ids.len();
        if scenario_count > 1 {
            shared_information_set_count += 1;
        }
        maximum_scenarios_per_information_set =
            maximum_scenarios_per_information_set.max(scenario_count);

        summaries.push(PolicyInformationSetSummary {
            key,
            scenario_ids: accumulator.scenario_ids,
            selected_action_id: accumulator.selected_action_id,
            action_catalog: accumulator.action_catalog,
        });
    }

    Ok(PolicyTreeReport {
        scenario_count: paths.len(),
        stage_decision_count,
        information_set_count: summaries.len(),
        shared_information_set_count,
        maximum_scenarios_per_information_set,
        information_sets: summaries,
    })
}

fn validate_context(context: &DecisionContext) -> Result<(), String> {
    if context.instance_id.trim().is_empty()
        || context.instance_version.trim().is_empty()
        || context.service_id.trim().is_empty()
        || context.expected_units == 0
    {
        return Err("policy-tree context has empty identity or zero expected units".to_owned());
    }
    Ok(())
}

fn stage_schedule(
    stages: &[PlannedStage],
    scenario_id: &str,
) -> Result<Vec<(u64, u64)>, String> {
    let mut schedule = Vec::with_capacity(stages.len());
    let mut previous: Option<(u64, u64)> = None;

    for stage in stages {
        let current = (stage.decision_epoch, stage.decision_cutoff);
        if let Some(prior) = previous {
            if current.0 <= prior.0 {
                return Err(format!(
                    "{}: decision epochs must be strictly increasing",
                    scenario_id
                ));
            }
            if current.1 <= prior.1 {
                return Err(format!(
                    "{}: decision cutoffs must be strictly increasing",
                    scenario_id
                ));
            }
        }
        schedule.push(current);
        previous = Some(current);
    }

    Ok(schedule)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> DecisionContext {
        DecisionContext {
            instance_id: "R1-POLICY-TREE-TINY".to_owned(),
            instance_version: "v0.2".to_owned(),
            service_id: "ESSENTIAL-SERVICE".to_owned(),
            service_deadline: 300,
            expected_units: 10,
        }
    }

    fn action(action_id: &str) -> PolicyTreeAction {
        PolicyTreeAction {
            action_id: action_id.to_owned(),
            admissibility: Gate::Yes,
            hard_constraints_passed: true,
            protected_floor_breach_unit_hours: 0,
        }
    }

    fn stage(
        epoch: u64,
        cutoff: u64,
        digest: &str,
        selected: &str,
    ) -> PlannedStage {
        PlannedStage {
            decision_epoch: epoch,
            decision_cutoff: cutoff,
            observable_state_digest: digest.to_owned(),
            available_actions: vec![action("ABSTAIN"), action("LOCAL"), action("IMPORT")],
            selected_action_id: selected.to_owned(),
        }
    }

    fn path(id: &str, stages: Vec<PlannedStage>) -> ScenarioPolicyPath {
        ScenarioPolicyPath {
            scenario_id: id.to_owned(),
            context: context(),
            stages,
        }
    }

    #[test]
    fn same_initial_information_requires_same_action_but_later_divergence_allows_adaptation() {
        let paths = vec![
            path(
                "SCENARIO-A",
                vec![
                    stage(1, 100, &"a".repeat(64), "LOCAL"),
                    stage(2, 200, &"b".repeat(64), "LOCAL"),
                ],
            ),
            path(
                "SCENARIO-B",
                vec![
                    stage(1, 100, &"a".repeat(64), "LOCAL"),
                    stage(2, 200, &"c".repeat(64), "IMPORT"),
                ],
            ),
        ];

        let report = validate_non_anticipative_policy_tree(&paths).expect("valid policy tree");
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.stage_decision_count, 4);
        assert_eq!(report.information_set_count, 3);
        assert_eq!(report.shared_information_set_count, 1);
        assert_eq!(report.maximum_scenarios_per_information_set, 2);
    }

    #[test]
    fn identical_observations_with_different_actions_are_rejected() {
        let paths = vec![
            path("A", vec![stage(1, 100, &"a".repeat(64), "LOCAL")]),
            path("B", vec![stage(1, 100, &"a".repeat(64), "IMPORT")]),
        ];
        let error = validate_non_anticipative_policy_tree(&paths).unwrap_err();
        assert!(error.contains("non-anticipativity violation"));
    }

    #[test]
    fn identical_observations_with_different_action_catalogues_are_rejected() {
        let first = stage(1, 100, &"a".repeat(64), "LOCAL");
        let mut second = stage(1, 100, &"a".repeat(64), "LOCAL");
        second.available_actions.pop();

        let error =
            validate_non_anticipative_policy_tree(&[path("A", vec![first]), path("B", vec![second])])
                .unwrap_err();
        assert!(error.contains("inconsistent action catalogues"));
    }

    #[test]
    fn selected_action_must_exist_and_be_admissible() {
        let unavailable = stage(1, 100, &"a".repeat(64), "UNKNOWN");
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", vec![unavailable])]).is_err()
        );

        let mut blocked = stage(1, 100, &"a".repeat(64), "LOCAL");
        blocked.available_actions[1].admissibility = Gate::Indeterminate;
        let error =
            validate_non_anticipative_policy_tree(&[path("A", vec![blocked])]).unwrap_err();
        assert!(error.contains("fails admissibility"));
    }

    #[test]
    fn hard_constraint_and_protected_floor_failures_are_rejected() {
        let mut hard_failure = stage(1, 100, &"a".repeat(64), "LOCAL");
        hard_failure.available_actions[1].hard_constraints_passed = false;
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", vec![hard_failure])]).is_err()
        );

        let mut floor_failure = stage(1, 100, &"a".repeat(64), "LOCAL");
        floor_failure.available_actions[1].protected_floor_breach_unit_hours = 1;
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", vec![floor_failure])]).is_err()
        );
    }

    #[test]
    fn scenario_paths_must_share_the_same_epoch_and_cutoff_schedule() {
        let first = path(
            "A",
            vec![
                stage(1, 100, &"a".repeat(64), "LOCAL"),
                stage(2, 200, &"b".repeat(64), "LOCAL"),
            ],
        );
        let second = path("B", vec![stage(1, 100, &"a".repeat(64), "LOCAL")]);
        let error = validate_non_anticipative_policy_tree(&[first, second]).unwrap_err();
        assert!(error.contains("schedule differs"));
    }

    #[test]
    fn epochs_and_cutoffs_must_be_strictly_increasing() {
        let repeated_epoch = vec![
            stage(1, 100, &"a".repeat(64), "LOCAL"),
            stage(1, 200, &"b".repeat(64), "LOCAL"),
        ];
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", repeated_epoch)]).is_err()
        );

        let repeated_cutoff = vec![
            stage(1, 100, &"a".repeat(64), "LOCAL"),
            stage(2, 100, &"b".repeat(64), "LOCAL"),
        ];
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", repeated_cutoff)]).is_err()
        );
    }

    #[test]
    fn malformed_observable_state_digest_is_rejected() {
        let invalid = stage(1, 100, "not-a-sha256", "LOCAL");
        assert!(
            validate_non_anticipative_policy_tree(&[path("A", vec![invalid])]).is_err()
        );
    }

    #[test]
    fn report_is_invariant_to_scenario_input_order() {
        let first = path(
            "A",
            vec![
                stage(1, 100, &"a".repeat(64), "LOCAL"),
                stage(2, 200, &"b".repeat(64), "LOCAL"),
            ],
        );
        let second = path(
            "B",
            vec![
                stage(1, 100, &"a".repeat(64), "LOCAL"),
                stage(2, 200, &"c".repeat(64), "IMPORT"),
            ],
        );

        let forward =
            validate_non_anticipative_policy_tree(&[first.clone(), second.clone()]).unwrap();
        let reverse = validate_non_anticipative_policy_tree(&[second, first]).unwrap();
        assert_eq!(forward, reverse);
    }
}
