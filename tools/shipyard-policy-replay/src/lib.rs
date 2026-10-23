//! Non-anticipativity audit for finite shipyard recovery-policy replay suites.
//!
//! This crate checks policy information discipline only. It is deliberately
//! independent of the schedule-feasibility and polygon-geometry verifiers.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

pub type Tick = u64;

/// A uniquely identified immutable event, at the time it became observable.
/// The event ID must identify the event's canonical content in the source data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvent {
    pub id: String,
    pub observed_at_tick: Tick,
}

/// A canonical identity for the action committed at a decision epoch.
///
/// The caller should supply a digest of that action's canonical encoding, not
/// a full future schedule whose unobserved branches may differ. This crate
/// checks equality only; it does not calculate or authenticate digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyAction {
    pub tick: Tick,
    pub action_key: String,
}

/// One scenario's event-observation stream and policy actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioTrace {
    pub scenario_id: String,
    pub events: Vec<ReplayEvent>,
    pub actions: Vec<PolicyAction>,
}

/// A suite uses the same declared decision epochs for each scenario.
/// Event-triggered systems should encode explicit no-op actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySuite {
    pub decision_ticks: Vec<Tick>,
    pub scenarios: Vec<ScenarioTrace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyReplayStatus {
    /// The supplied complete traces obey non-anticipativity.
    ///
    /// This is not a schedule-feasibility, optimality, or safety result.
    Consistent,
    /// Complete traces select different actions with identical observable
    /// histories at the same decision tick.
    AnticipativityViolation,
    /// The suite is empty or malformed, so no policy-consistency claim is made.
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyReplayDiagnostic {
    pub code: String,
    pub scenario_id: Option<String>,
    pub tick: Option<Tick>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyReplayReport {
    pub status: PolicyReplayStatus,
    pub scenario_count: usize,
    pub decision_count: usize,
    pub information_set_count: usize,
    /// Number of later traces compared with the first action for each shared
    /// information set. This is deterministic and is not a pairwise n² count.
    pub information_set_comparisons: usize,
    pub diagnostics: Vec<PolicyReplayDiagnostic>,
}

/// Audit structural completeness and the non-anticipativity condition.
///
/// At decision tick t, the observable history is the sorted sequence of
/// (event ID, observation tick) pairs for events observed at or before t. For
/// each (t, history), every trace must select the same action key.
///
/// The input must already be bound to its intended source revision. Shared
/// observations across scenarios must reuse canonical event IDs. This function
/// does not establish that event logs are truthful, action keys are
/// collision-resistant, or selected actions are feasible.
pub fn audit_policy_replay(suite: &ReplaySuite) -> PolicyReplayReport {
    let scenario_count = suite.scenarios.len();
    let decision_count = suite
        .scenarios
        .iter()
        .map(|scenario| scenario.actions.len())
        .sum();
    let mut diagnostics = Vec::new();

    if suite.scenarios.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_EMPTY_SCENARIOS",
            None,
            None,
            "the replay suite must contain at least one declared scenario",
        );
    }
    if suite.decision_ticks.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_EMPTY_DECISION_EPOCHS",
            None,
            None,
            "the replay suite must declare at least one decision epoch",
        );
    }
    if suite
        .decision_ticks
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_DECISION_EPOCH_ORDER",
            None,
            None,
            "decision ticks must be unique and strictly increasing",
        );
    }

    let expected_ticks: BTreeSet<Tick> = suite.decision_ticks.iter().copied().collect();
    let mut scenario_ids = BTreeSet::new();

    for scenario in &suite.scenarios {
        if scenario.scenario_id.trim().is_empty() {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_EMPTY_SCENARIO_ID",
                None,
                None,
                "scenario IDs must be non-empty",
            );
        } else if !scenario_ids.insert(scenario.scenario_id.as_str()) {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_DUPLICATE_SCENARIO_ID",
                Some(&scenario.scenario_id),
                None,
                "scenario IDs must be unique within the replay suite",
            );
        }

        let mut event_ids = BTreeSet::new();
        for event in &scenario.events {
            if event.id.trim().is_empty() {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_EMPTY_EVENT_ID",
                    Some(&scenario.scenario_id),
                    Some(event.observed_at_tick),
                    "event IDs must be non-empty",
                );
            } else if !event_ids.insert(event.id.as_str()) {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_DUPLICATE_EVENT_ID",
                    Some(&scenario.scenario_id),
                    Some(event.observed_at_tick),
                    "event IDs must be unique within a scenario trace",
                );
            }
        }

        if scenario
            .actions
            .windows(2)
            .any(|pair| pair[0].tick >= pair[1].tick)
        {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_ACTION_ORDER",
                Some(&scenario.scenario_id),
                None,
                "actions must be ordered by strictly increasing decision tick",
            );
        }

        let mut counts = BTreeMap::<Tick, usize>::new();
        for action in &scenario.actions {
            *counts.entry(action.tick).or_default() += 1;
            if !expected_ticks.contains(&action.tick) {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_UNEXPECTED_DECISION_TICK",
                    Some(&scenario.scenario_id),
                    Some(action.tick),
                    "an action uses a tick not declared by the replay suite",
                );
            }
            if action.action_key.trim().is_empty() {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_EMPTY_ACTION_KEY",
                    Some(&scenario.scenario_id),
                    Some(action.tick),
                    "action keys must be non-empty canonical identities",
                );
            }
        }

        for tick in &suite.decision_ticks {
            match counts.get(tick).copied().unwrap_or(0) {
                0 => push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_MISSING_DECISION",
                    Some(&scenario.scenario_id),
                    Some(*tick),
                    "exactly one explicit action is required at each declared decision tick",
                ),
                1 => {}
                count => push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_DUPLICATE_DECISION",
                    Some(&scenario.scenario_id),
                    Some(*tick),
                    &format!("expected one action at this tick, found {count}"),
                ),
            }
        }
    }

    if !diagnostics.is_empty() {
        return PolicyReplayReport {
            status: PolicyReplayStatus::Incomplete,
            scenario_count,
            decision_count,
            information_set_count: 0,
            information_set_comparisons: 0,
            diagnostics,
        };
    }

    // Sorting references ensures identical evidence produces identical reports
    // independent of the input scenario order.
    let mut ordered_scenarios: Vec<&ScenarioTrace> = suite.scenarios.iter().collect();
    ordered_scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    // The observation tick belongs in the key: observing the same event at
    // different times represents a different information history.
    type InformationSetKey = (Tick, Vec<(String, Tick)>);
    let mut selected_action_by_information_set =
        BTreeMap::<InformationSetKey, (String, String)>::new();
    let mut information_sets = BTreeSet::<InformationSetKey>::new();
    let mut information_set_comparisons = 0;

    for scenario in ordered_scenarios {
        let actions_by_tick: BTreeMap<Tick, &PolicyAction> = scenario
            .actions
            .iter()
            .map(|action| (action.tick, action))
            .collect();

        for tick in &suite.decision_ticks {
            let mut visible_history: Vec<(String, Tick)> = scenario
                .events
                .iter()
                .filter(|event| event.observed_at_tick <= *tick)
                .map(|event| (event.id.clone(), event.observed_at_tick))
                .collect();
            visible_history.sort();

            let key = (*tick, visible_history);
            information_sets.insert(key.clone());

            // Structural validation above proves exactly one action exists for
            // each declared tick in every trace.
            let action = actions_by_tick[tick];
            match selected_action_by_information_set.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert((action.action_key.clone(), scenario.scenario_id.clone()));
                }
                Entry::Occupied(entry) => {
                    let (prior_action_key, prior_scenario_id) = entry.get();
                    information_set_comparisons += 1;
                    if prior_action_key != &action.action_key {
                        push_diagnostic(
                            &mut diagnostics,
                            "NON_ANTICIPATIVE_ACTION_MISMATCH",
                            Some(&scenario.scenario_id),
                            Some(*tick),
                            &format!(
                                "scenario '{}' selects a different action than scenario '{}' despite an identical observable history",
                                scenario.scenario_id, prior_scenario_id
                            ),
                        );
                    }
                }
            }
        }
    }

    let status = if diagnostics.is_empty() {
        PolicyReplayStatus::Consistent
    } else {
        PolicyReplayStatus::AnticipativityViolation
    };

    PolicyReplayReport {
        status,
        scenario_count,
        decision_count,
        information_set_count: information_sets.len(),
        information_set_comparisons,
        diagnostics,
    }
}

fn push_diagnostic(
    diagnostics: &mut Vec<PolicyReplayDiagnostic>,
    code: &str,
    scenario_id: Option<&str>,
    tick: Option<Tick>,
    detail: &str,
) {
    diagnostics.push(PolicyReplayDiagnostic {
        code: code.to_owned(),
        scenario_id: scenario_id.map(str::to_owned),
        tick,
        detail: detail.to_owned(),
    });
}

#[cfg(test)]
mod tests {
    use super::{
        audit_policy_replay, PolicyAction, PolicyReplayStatus, ReplayEvent, ReplaySuite,
        ScenarioTrace,
    };

    fn event(id: &str, observed_at_tick: u64) -> ReplayEvent {
        ReplayEvent {
            id: id.to_owned(),
            observed_at_tick,
        }
    }

    fn action(tick: u64, action_key: &str) -> PolicyAction {
        PolicyAction {
            tick,
            action_key: action_key.to_owned(),
        }
    }

    fn scenario(
        scenario_id: &str,
        events: Vec<ReplayEvent>,
        actions: Vec<PolicyAction>,
    ) -> ScenarioTrace {
        ScenarioTrace {
            scenario_id: scenario_id.to_owned(),
            events,
            actions,
        }
    }

    #[test]
    fn adaptive_policy_may_branch_only_after_disturbance_is_observed() {
        let suite = ReplaySuite {
            decision_ticks: vec![0, 2, 5],
            scenarios: vec![
                scenario(
                    "vehicle-a-fails",
                    vec![event("shared-start", 0), event("vehicle-a-fault", 4)],
                    vec![action(0, "initial-plan"), action(2, "hold"), action(5, "reroute-a")],
                ),
                scenario(
                    "vehicle-b-fails",
                    vec![event("shared-start", 0), event("vehicle-b-fault", 4)],
                    vec![action(0, "initial-plan"), action(2, "hold"), action(5, "reroute-b")],
                ),
            ],
        };

        let report = audit_policy_replay(&suite);
        assert_eq!(report.status, PolicyReplayStatus::Consistent);
        assert_eq!(report.scenario_count, 2);
        assert_eq!(report.decision_count, 6);
        assert_eq!(report.information_set_comparisons, 2);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn rejects_clairvoyant_action_before_branch_is_observed() {
        let suite = ReplaySuite {
            decision_ticks: vec![0, 2, 5],
            scenarios: vec![
                scenario(
                    "vehicle-a-fails",
                    vec![event("shared-start", 0), event("vehicle-a-fault", 4)],
                    vec![action(0, "initial-plan"), action(2, "reroute-a"), action(5, "reroute-a")],
                ),
                scenario(
                    "vehicle-b-fails",
                    vec![event("shared-start", 0), event("vehicle-b-fault", 4)],
                    vec![action(0, "initial-plan"), action(2, "reroute-b"), action(5, "reroute-b")],
                ),
            ],
        };

        let report = audit_policy_replay(&suite);
        assert_eq!(report.status, PolicyReplayStatus::AnticipativityViolation);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "NON_ANTICIPATIVE_ACTION_MISMATCH"
                && diagnostic.tick == Some(2)
        }));
    }

    #[test]
    fn event_observed_at_decision_tick_is_visible_to_that_decision() {
        let suite = ReplaySuite {
            decision_ticks: vec![4],
            scenarios: vec![
                scenario(
                    "vehicle-a-fails",
                    vec![event("vehicle-a-fault", 4)],
                    vec![action(4, "reroute-a")],
                ),
                scenario(
                    "vehicle-b-fails",
                    vec![event("vehicle-b-fault", 4)],
                    vec![action(4, "reroute-b")],
                ),
            ],
        };

        assert_eq!(
            audit_policy_replay(&suite).status,
            PolicyReplayStatus::Consistent
        );
    }

    #[test]
    fn observation_time_is_part_of_the_information_history() {
        let suite = ReplaySuite {
            decision_ticks: vec![2],
            scenarios: vec![
                scenario(
                    "early-observation",
                    vec![event("vehicle-fault", 0)],
                    vec![action(2, "act-after-early-warning")],
                ),
                scenario(
                    "late-observation",
                    vec![event("vehicle-fault", 1)],
                    vec![action(2, "act-after-late-warning")],
                ),
            ],
        };

        assert_eq!(
            audit_policy_replay(&suite).status,
            PolicyReplayStatus::Consistent
        );
    }

    #[test]
    fn missing_action_is_incomplete_not_a_pass() {
        let suite = ReplaySuite {
            decision_ticks: vec![0, 1],
            scenarios: vec![scenario("case-a", vec![], vec![action(0, "a")])],
        };

        let report = audit_policy_replay(&suite);
        assert_eq!(report.status, PolicyReplayStatus::Incomplete);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "INCOMPLETE_MISSING_DECISION"
                && diagnostic.tick == Some(1)
        }));
    }

    #[test]
    fn duplicate_event_identity_is_incomplete() {
        let suite = ReplaySuite {
            decision_ticks: vec![0],
            scenarios: vec![scenario(
                "case-a",
                vec![event("same-id", 0), event("same-id", 0)],
                vec![action(0, "a")],
            )],
        };

        let report = audit_policy_replay(&suite);
        assert_eq!(report.status, PolicyReplayStatus::Incomplete);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "INCOMPLETE_DUPLICATE_EVENT_ID"));
    }

    #[test]
    fn empty_scenario_or_epoch_set_is_incomplete() {
        let empty_scenarios = ReplaySuite {
            decision_ticks: vec![0],
            scenarios: vec![],
        };
        let empty_epochs = ReplaySuite {
            decision_ticks: vec![],
            scenarios: vec![scenario("case-a", vec![], vec![])],
        };

        assert_eq!(
            audit_policy_replay(&empty_scenarios).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&empty_epochs).status,
            PolicyReplayStatus::Incomplete
        );
    }

    #[test]
    fn report_is_deterministic_under_scenario_permutation() {
        let a = scenario(
            "a",
            vec![event("shared", 0), event("only-a", 3)],
            vec![action(0, "same"), action(4, "a")],
        );
        let b = scenario(
            "b",
            vec![event("shared", 0), event("only-b", 3)],
            vec![action(0, "same"), action(4, "b")],
        );

        let forward = ReplaySuite {
            decision_ticks: vec![0, 4],
            scenarios: vec![a.clone(), b.clone()],
        };
        let reverse = ReplaySuite {
            decision_ticks: vec![0, 4],
            scenarios: vec![b, a],
        };

        assert_eq!(audit_policy_replay(&forward), audit_policy_replay(&reverse));
    }
}
