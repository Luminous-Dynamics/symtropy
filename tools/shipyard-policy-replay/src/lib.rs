//! Non-anticipativity audit for finite shipyard recovery-policy replay suites.
//!
//! The source event logs and candidate actions are separate inputs: a candidate
//! cannot rewrite the observation history against which its actions are judged.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};

pub type Tick = u64;

/// A source observation, including when it became available to the policy.
///
/// `id` identifies the underlying event; `observation_key` identifies the
/// complete policy-visible payload in canonical form (ideally a digest of the
/// canonical payload). The key must change if visible contents change, and it
/// should be the same for identical shared observations across scenario
/// branches. This crate checks non-emptiness/equality, not digest authenticity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationVisibility {
    /// The event payload was available to the policy by its observation tick.
    PolicyVisible,
    /// The event belongs to evaluator/source truth but was not provided to the policy.
    EvaluatorOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvent {
    pub id: String,
    pub observed_at_tick: Tick,
    pub observation_key: String,
    pub visibility: ObservationVisibility,
}

/// Immutable source-owned event observations for one declared scenario.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioEventLog {
    pub scenario_id: String,
    pub events: Vec<ReplayEvent>,
}

/// A canonical identity for the action committed at one decision epoch.
///
/// The key should identify the current action only, not a full future schedule
/// whose as-yet-unobserved branches may differ. This crate checks key equality;
/// it does not calculate or authenticate action digests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyAction {
    pub tick: Tick,
    pub action_key: String,
}

/// Candidate-supplied actions for one scenario. No event history is accepted
/// from this object; the auditor joins it to the source-owned ScenarioEventLog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioTrace {
    pub scenario_id: String,
    pub actions: Vec<PolicyAction>,
}

/// The benchmark manifest inventory, source observations, and candidate traces.
///
/// `expected_scenario_ids` must be loaded independently from the immutable
/// benchmark/source manifest. This function checks list coverage but does not
/// authenticate the manifest or any file digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaySuite {
    pub expected_scenario_ids: Vec<String>,
    pub decision_ticks: Vec<Tick>,
    pub source_scenarios: Vec<ScenarioEventLog>,
    pub candidate_traces: Vec<ScenarioTrace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyReplayStatus {
    /// Complete candidate traces obey non-anticipativity for source histories.
    ///
    /// This is not a schedule-feasibility, optimality, or safety result.
    Consistent,
    /// Complete traces choose different actions with identical source-visible
    /// histories at the same decision tick.
    AnticipativityViolation,
    /// Source coverage, candidate coverage, or trace structure is malformed.
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
    pub source_scenario_count: usize,
    pub candidate_trace_count: usize,
    pub decision_count: usize,
    pub information_set_count: usize,
    /// Number of later traces compared against the first action in each
    /// shared information set. Deterministic; not a pairwise n-squared count.
    pub information_set_comparisons: usize,
    pub diagnostics: Vec<PolicyReplayDiagnostic>,
}

/// Audit scenario inventory, source event histories, candidate decision traces,
/// and the non-anticipativity condition.
///
/// At decision tick t, visible history is the sorted list of source-owned
/// (event ID, observation tick) pairs whose observation tick is <= t. For each
/// identical (t, visible history), every candidate trace must select the same
/// current action key.
///
/// The source inventory and logs must be bound by the caller to the intended
/// immutable revision. This function cannot establish event truth, manifest
/// authenticity, action-key collision resistance, or action feasibility.
pub fn audit_policy_replay(suite: &ReplaySuite) -> PolicyReplayReport {
    let source_scenario_count = suite.source_scenarios.len();
    let candidate_trace_count = suite.candidate_traces.len();
    let decision_count = suite
        .candidate_traces
        .iter()
        .map(|trace| trace.actions.len())
        .sum();
    let mut diagnostics = Vec::new();

    if suite.expected_scenario_ids.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_EMPTY_EXPECTED_SCENARIOS",
            None,
            None,
            "the source-defined expected scenario inventory must be non-empty",
        );
    }

    let mut expected_ids = BTreeSet::new();
    for expected_id in &suite.expected_scenario_ids {
        if expected_id.trim().is_empty() {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_EMPTY_EXPECTED_SCENARIO_ID",
                None,
                None,
                "expected scenario IDs must be non-empty",
            );
        } else if !expected_ids.insert(expected_id.as_str()) {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_DUPLICATE_EXPECTED_SCENARIO_ID",
                Some(expected_id.as_str()),
                None,
                "the source-defined expected scenario inventory must not duplicate IDs",
            );
        }
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

    if suite.source_scenarios.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_EMPTY_SOURCE_SCENARIOS",
            None,
            None,
            "source-owned event logs must cover the declared scenario inventory",
        );
    }
    if suite.candidate_traces.is_empty() {
        push_diagnostic(
            &mut diagnostics,
            "INCOMPLETE_EMPTY_CANDIDATE_TRACES",
            None,
            None,
            "at least one candidate trace is required",
        );
    }

    let expected_ticks: BTreeSet<Tick> = suite.decision_ticks.iter().copied().collect();
    let mut source_ids = BTreeSet::new();

    // Validate trusted source event logs independently of candidate output.
    for source in &suite.source_scenarios {
        if source.scenario_id.trim().is_empty() {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_EMPTY_SOURCE_SCENARIO_ID",
                None,
                None,
                "source scenario IDs must be non-empty",
            );
        } else if !source_ids.insert(source.scenario_id.as_str()) {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_DUPLICATE_SOURCE_SCENARIO_ID",
                Some(source.scenario_id.as_str()),
                None,
                "source event logs must have unique scenario IDs",
            );
        }

        let mut event_ids = BTreeSet::new();
        for event in &source.events {
            if event.id.trim().is_empty() {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_EMPTY_EVENT_ID",
                    Some(source.scenario_id.as_str()),
                    Some(event.observed_at_tick),
                    "source event IDs must be non-empty",
                );
            } else if !event_ids.insert(event.id.as_str()) {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_DUPLICATE_EVENT_ID",
                    Some(source.scenario_id.as_str()),
                    Some(event.observed_at_tick),
                    "source event IDs must be unique within a scenario log",
                );
            }
            if event.observation_key.trim().is_empty() {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_EMPTY_OBSERVATION_KEY",
                    Some(source.scenario_id.as_str()),
                    Some(event.observed_at_tick),
                    "source observation keys must identify non-empty canonical visible content",
                );
            }
        }
    }

    // Validate candidate traces independently; they provide no event history.
    let mut candidate_ids = BTreeSet::new();
    for trace in &suite.candidate_traces {
        if trace.scenario_id.trim().is_empty() {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_EMPTY_CANDIDATE_SCENARIO_ID",
                None,
                None,
                "candidate trace scenario IDs must be non-empty",
            );
        } else if !candidate_ids.insert(trace.scenario_id.as_str()) {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_DUPLICATE_CANDIDATE_SCENARIO_ID",
                Some(trace.scenario_id.as_str()),
                None,
                "candidate traces must have unique scenario IDs",
            );
        }

        if trace
            .actions
            .windows(2)
            .any(|pair| pair[0].tick >= pair[1].tick)
        {
            push_diagnostic(
                &mut diagnostics,
                "INCOMPLETE_ACTION_ORDER",
                Some(trace.scenario_id.as_str()),
                None,
                "candidate actions must be ordered by strictly increasing decision tick",
            );
        }

        let mut counts = BTreeMap::<Tick, usize>::new();
        for action in &trace.actions {
            *counts.entry(action.tick).or_default() += 1;
            if !expected_ticks.contains(&action.tick) {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_UNEXPECTED_DECISION_TICK",
                    Some(trace.scenario_id.as_str()),
                    Some(action.tick),
                    "a candidate action uses a tick not declared by the replay suite",
                );
            }
            if action.action_key.trim().is_empty() {
                push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_EMPTY_ACTION_KEY",
                    Some(trace.scenario_id.as_str()),
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
                    Some(trace.scenario_id.as_str()),
                    Some(*tick),
                    "exactly one explicit action is required at each declared decision tick",
                ),
                1 => {}
                count => push_diagnostic(
                    &mut diagnostics,
                    "INCOMPLETE_DUPLICATE_DECISION",
                    Some(trace.scenario_id.as_str()),
                    Some(*tick),
                    &format!("expected one action at this tick, found {count}"),
                ),
            }
        }
    }

    validate_inventory_coverage(
        &mut diagnostics,
        &expected_ids,
        &source_ids,
        "INCOMPLETE_MISSING_SOURCE_SCENARIO",
        "INCOMPLETE_UNEXPECTED_SOURCE_SCENARIO",
        "a declared scenario has no source-owned event log",
        "a source event log is not in the declared scenario inventory",
    );
    validate_inventory_coverage(
        &mut diagnostics,
        &expected_ids,
        &candidate_ids,
        "INCOMPLETE_MISSING_DECLARED_SCENARIO",
        "INCOMPLETE_UNEXPECTED_SCENARIO_TRACE",
        "a declared scenario has no candidate replay trace",
        "a candidate replay trace is not in the declared scenario inventory",
    );

    if !diagnostics.is_empty() {
        return PolicyReplayReport {
            status: PolicyReplayStatus::Incomplete,
            source_scenario_count,
            candidate_trace_count,
            decision_count,
            information_set_count: 0,
            information_set_comparisons: 0,
            diagnostics,
        };
    }

    let source_by_id: BTreeMap<&str, &ScenarioEventLog> = suite
        .source_scenarios
        .iter()
        .map(|source| (source.scenario_id.as_str(), source))
        .collect();

    // Sort candidate references so diagnostic ordering is stable if the input
    // candidate list is permuted.
    let mut ordered_traces: Vec<&ScenarioTrace> = suite.candidate_traces.iter().collect();
    ordered_traces.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    type InformationSetKey = (Tick, Vec<(String, Tick, String)>);
    let mut selected_action_by_information_set =
        BTreeMap::<InformationSetKey, (String, String)>::new();
    let mut information_sets = BTreeSet::<InformationSetKey>::new();
    let mut information_set_comparisons = 0;

    for trace in ordered_traces {
        // Crucially, this event history is taken from the independent source
        // map, never from candidate-supplied data.
        let source = source_by_id[trace.scenario_id.as_str()];
        let actions_by_tick: BTreeMap<Tick, &PolicyAction> = trace
            .actions
            .iter()
            .map(|action| (action.tick, action))
            .collect();

        for tick in &suite.decision_ticks {
            let mut visible_history: Vec<(String, Tick, String)> = source
                .events
                .iter()
                .filter(|event| {
                    event.visibility == ObservationVisibility::PolicyVisible
                        && event.observed_at_tick <= *tick
                })
                .map(|event| (
                    event.id.clone(),
                    event.observed_at_tick,
                    event.observation_key.clone(),
                ))
                .collect();
            visible_history.sort();

            let key = (*tick, visible_history);
            information_sets.insert(key.clone());
            let action = actions_by_tick[tick];

            match selected_action_by_information_set.entry(key) {
                Entry::Vacant(entry) => {
                    entry.insert((action.action_key.clone(), trace.scenario_id.clone()));
                }
                Entry::Occupied(entry) => {
                    let (prior_action_key, prior_scenario_id) = entry.get();
                    information_set_comparisons += 1;
                    if prior_action_key != &action.action_key {
                        push_diagnostic(
                            &mut diagnostics,
                            "NON_ANTICIPATIVE_ACTION_MISMATCH",
                            Some(trace.scenario_id.as_str()),
                            Some(*tick),
                            &format!(
                                "scenario '{}' selects a different action than scenario '{}' \
                                 despite an identical source-visible history",
                                trace.scenario_id, prior_scenario_id
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
        source_scenario_count,
        candidate_trace_count,
        decision_count,
        information_set_count: information_sets.len(),
        information_set_comparisons,
        diagnostics,
    }
}

fn validate_inventory_coverage(
    diagnostics: &mut Vec<PolicyReplayDiagnostic>,
    expected_ids: &BTreeSet<&str>,
    actual_ids: &BTreeSet<&str>,
    missing_code: &str,
    unexpected_code: &str,
    missing_detail: &str,
    unexpected_detail: &str,
) {
    for expected_id in expected_ids {
        if !actual_ids.contains(expected_id) {
            push_diagnostic(
                diagnostics,
                missing_code,
                Some(*expected_id),
                None,
                missing_detail,
            );
        }
    }
    for actual_id in actual_ids {
        if !expected_ids.contains(actual_id) {
            push_diagnostic(
                diagnostics,
                unexpected_code,
                Some(*actual_id),
                None,
                unexpected_detail,
            );
        }
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
        audit_policy_replay, ObservationVisibility, PolicyAction, PolicyReplayStatus, ReplayEvent,
        ReplaySuite, ScenarioEventLog, ScenarioTrace, Tick,
    };

    #[derive(Clone)]
    struct ScenarioFixture {
        scenario_id: String,
        events: Vec<ReplayEvent>,
        actions: Vec<PolicyAction>,
    }

    impl ScenarioFixture {
        fn source(&self) -> ScenarioEventLog {
            ScenarioEventLog {
                scenario_id: self.scenario_id.clone(),
                events: self.events.clone(),
            }
        }

        fn candidate(&self) -> ScenarioTrace {
            ScenarioTrace {
                scenario_id: self.scenario_id.clone(),
                actions: self.actions.clone(),
            }
        }
    }

    fn event(id: &str, observed_at_tick: u64) -> ReplayEvent {
        ReplayEvent {
            id: id.to_owned(),
            observed_at_tick,
            observation_key: format!("observation:{id}"),
            visibility: ObservationVisibility::PolicyVisible,
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
    ) -> ScenarioFixture {
        ScenarioFixture {
            scenario_id: scenario_id.to_owned(),
            events,
            actions,
        }
    }

    fn suite(
        expected_scenario_ids: &[&str],
        decision_ticks: Vec<Tick>,
        fixtures: Vec<ScenarioFixture>,
    ) -> ReplaySuite {
        ReplaySuite {
            expected_scenario_ids: expected_scenario_ids
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
            decision_ticks,
            source_scenarios: fixtures.iter().map(ScenarioFixture::source).collect(),
            candidate_traces: fixtures.iter().map(ScenarioFixture::candidate).collect(),
        }
    }

    #[test]
    fn adaptive_policy_may_branch_only_after_disturbance_is_observed() {
        let replay = suite(
            &["vehicle-a-fails", "vehicle-b-fails"],
            vec![0, 2, 5],
            vec![
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
        );

        let report = audit_policy_replay(&replay);
        assert_eq!(report.status, PolicyReplayStatus::Consistent);
        assert_eq!(report.source_scenario_count, 2);
        assert_eq!(report.candidate_trace_count, 2);
        assert_eq!(report.decision_count, 6);
        assert_eq!(report.information_set_comparisons, 2);
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn rejects_clairvoyant_action_before_branch_is_observed() {
        let replay = suite(
            &["vehicle-a-fails", "vehicle-b-fails"],
            vec![0, 2, 5],
            vec![
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
        );

        let report = audit_policy_replay(&replay);
        assert_eq!(report.status, PolicyReplayStatus::AnticipativityViolation);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "NON_ANTICIPATIVE_ACTION_MISMATCH"
                && diagnostic.tick == Some(2)
        }));
    }

    #[test]
    fn visible_payload_differences_create_distinct_information_sets() {
        let mut event_a = event("shared-fault-id", 0);
        event_a.observation_key = "severity-low".to_owned();
        let mut event_b = event("shared-fault-id", 0);
        event_b.observation_key = "severity-high".to_owned();

        let replay = suite(
            &["low-severity", "high-severity"],
            vec![0],
            vec![
                scenario(
                    "low-severity",
                    vec![event_a],
                    vec![action(0, "small-reroute")],
                ),
                scenario(
                    "high-severity",
                    vec![event_b],
                    vec![action(0, "large-reroute")],
                ),
            ],
        );

        assert_eq!(
            audit_policy_replay(&replay).status,
            PolicyReplayStatus::Consistent
        );
    }

    #[test]
    fn evaluator_only_payload_cannot_change_action_for_same_visible_history() {
        let mut hidden_a = event("future-outcome", 0);
        hidden_a.observation_key = "hidden-outcome-a".to_owned();
        hidden_a.visibility = ObservationVisibility::EvaluatorOnly;
        let mut hidden_b = event("future-outcome", 0);
        hidden_b.observation_key = "hidden-outcome-b".to_owned();
        hidden_b.visibility = ObservationVisibility::EvaluatorOnly;

        let replay = suite(
            &["branch-a", "branch-b"],
            vec![0],
            vec![
                scenario(
                    "branch-a",
                    vec![event("common-start", 0), hidden_a],
                    vec![action(0, "act-for-future-a")],
                ),
                scenario(
                    "branch-b",
                    vec![event("common-start", 0), hidden_b],
                    vec![action(0, "act-for-future-b")],
                ),
            ],
        );

        let report = audit_policy_replay(&replay);
        assert_eq!(report.status, PolicyReplayStatus::AnticipativityViolation);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "NON_ANTICIPATIVE_ACTION_MISMATCH"
                && diagnostic.tick == Some(0)
        }));
    }

    #[test]
    fn event_observed_at_decision_tick_is_visible_to_that_decision() {
        let replay = suite(
            &["vehicle-a-fails", "vehicle-b-fails"],
            vec![4],
            vec![
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
        );

        assert_eq!(
            audit_policy_replay(&replay).status,
            PolicyReplayStatus::Consistent
        );
    }

    #[test]
    fn observation_time_is_part_of_the_information_history() {
        let replay = suite(
            &["early-observation", "late-observation"],
            vec![2],
            vec![
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
        );

        assert_eq!(
            audit_policy_replay(&replay).status,
            PolicyReplayStatus::Consistent
        );
    }

    #[test]
    fn exhaustive_two_branch_reveal_matrix_matches_information_sets() {
        let decision_ticks = vec![0_u64, 1, 2];

        // Tick 3 means the branch event is not visible during this replay.
        for reveal_a in 0_u64..=3 {
            for reveal_b in 0_u64..=3 {
                for divergence_tick in 0_u64..=3 {
                    let make_actions = |branch_key: &str| {
                        decision_ticks
                            .iter()
                            .map(|tick| {
                                let key = if *tick < divergence_tick {
                                    "shared-action"
                                } else {
                                    branch_key
                                };
                                action(*tick, key)
                            })
                            .collect()
                    };

                    let replay = suite(
                        &["branch-a", "branch-b"],
                        decision_ticks.clone(),
                        vec![
                            scenario(
                                "branch-a",
                                vec![event("common-start", 0), event("fault-a", reveal_a)],
                                make_actions("action-a"),
                            ),
                            scenario(
                                "branch-b",
                                vec![event("common-start", 0), event("fault-b", reveal_b)],
                                make_actions("action-b"),
                            ),
                        ],
                    );

                    let should_reject = divergence_tick < reveal_a.min(reveal_b);
                    let expected = if should_reject {
                        PolicyReplayStatus::AnticipativityViolation
                    } else {
                        PolicyReplayStatus::Consistent
                    };
                    assert_eq!(
                        audit_policy_replay(&replay).status,
                        expected,
                        "reveal_a={reveal_a}, reveal_b={reveal_b}, divergence={divergence_tick}"
                    );
                }
            }
        }
    }

    #[test]
    fn missing_action_is_incomplete_not_a_pass() {
        let replay = suite(
            &["case-a"],
            vec![0, 1],
            vec![scenario("case-a", vec![], vec![action(0, "a")])],
        );

        let report = audit_policy_replay(&replay);
        assert_eq!(report.status, PolicyReplayStatus::Incomplete);
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "INCOMPLETE_MISSING_DECISION"
                && diagnostic.tick == Some(1)
        }));
    }

    #[test]
    fn duplicate_event_identity_is_incomplete() {
        let replay = suite(
            &["case-a"],
            vec![0],
            vec![scenario(
                "case-a",
                vec![event("same-id", 0), event("same-id", 0)],
                vec![action(0, "a")],
            )],
        );

        let report = audit_policy_replay(&replay);
        assert_eq!(report.status, PolicyReplayStatus::Incomplete);
        assert!(report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "INCOMPLETE_DUPLICATE_EVENT_ID"));
    }

    #[test]
    fn declared_scenario_inventory_rejects_missing_and_unexpected_source_and_candidate_traces() {
        let a = scenario("a", vec![], vec![action(0, "x")]);
        let missing_candidate = ReplaySuite {
            expected_scenario_ids: vec!["a".to_owned(), "b".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![a.source(), ScenarioEventLog {
                scenario_id: "b".to_owned(),
                events: vec![],
            }],
            candidate_traces: vec![a.candidate()],
        };
        let unexpected_candidate = ReplaySuite {
            expected_scenario_ids: vec!["a".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![a.source()],
            candidate_traces: vec![
                a.candidate(),
                ScenarioTrace {
                    scenario_id: "b".to_owned(),
                    actions: vec![action(0, "x")],
                },
            ],
        };
        let missing_source = ReplaySuite {
            expected_scenario_ids: vec!["a".to_owned(), "b".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![a.source()],
            candidate_traces: vec![
                a.candidate(),
                ScenarioTrace {
                    scenario_id: "b".to_owned(),
                    actions: vec![action(0, "x")],
                },
            ],
        };
        let unexpected_source = ReplaySuite {
            expected_scenario_ids: vec!["a".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![
                a.source(),
                ScenarioEventLog {
                    scenario_id: "b".to_owned(),
                    events: vec![],
                },
            ],
            candidate_traces: vec![a.candidate()],
        };

        assert!(audit_policy_replay(&missing_candidate)
            .diagnostics
            .iter()
            .any(|d| d.code == "INCOMPLETE_MISSING_DECLARED_SCENARIO"));
        assert!(audit_policy_replay(&unexpected_candidate)
            .diagnostics
            .iter()
            .any(|d| d.code == "INCOMPLETE_UNEXPECTED_SCENARIO_TRACE"));
        assert!(audit_policy_replay(&missing_source)
            .diagnostics
            .iter()
            .any(|d| d.code == "INCOMPLETE_MISSING_SOURCE_SCENARIO"));
        assert!(audit_policy_replay(&unexpected_source)
            .diagnostics
            .iter()
            .any(|d| d.code == "INCOMPLETE_UNEXPECTED_SOURCE_SCENARIO"));
        assert_eq!(
            audit_policy_replay(&missing_candidate).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&unexpected_candidate).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&missing_source).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&unexpected_source).status,
            PolicyReplayStatus::Incomplete
        );
    }

    #[test]
    fn empty_source_candidate_or_epoch_set_is_incomplete() {
        let no_source = ReplaySuite {
            expected_scenario_ids: vec!["case-a".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![],
            candidate_traces: vec![ScenarioTrace {
                scenario_id: "case-a".to_owned(),
                actions: vec![action(0, "a")],
            }],
        };
        let no_candidate = ReplaySuite {
            expected_scenario_ids: vec!["case-a".to_owned()],
            decision_ticks: vec![0],
            source_scenarios: vec![ScenarioEventLog {
                scenario_id: "case-a".to_owned(),
                events: vec![],
            }],
            candidate_traces: vec![],
        };
        let no_epochs = suite(
            &["case-a"],
            vec![],
            vec![scenario("case-a", vec![], vec![])],
        );

        assert_eq!(
            audit_policy_replay(&no_source).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&no_candidate).status,
            PolicyReplayStatus::Incomplete
        );
        assert_eq!(
            audit_policy_replay(&no_epochs).status,
            PolicyReplayStatus::Incomplete
        );
    }

    #[test]
    fn report_is_deterministic_under_candidate_trace_permutation() {
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

        let forward = suite(&["a", "b"], vec![0, 4], vec![a.clone(), b.clone()]);
        let reverse = suite(&["a", "b"], vec![0, 4], vec![b, a]);

        assert_eq!(
            audit_policy_replay(&forward),
            audit_policy_replay(&reverse)
        );
    }
}
