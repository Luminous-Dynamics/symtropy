// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-013: context-conditioned failure memory and replay-gated repair.
//!
//! Independent synthetic action/precondition diagnostic. Evaluator safety
//! labels are used only after policy decisions are frozen; policies see only
//! task/object/context and recorded, source-versioned outcomes.
//!
//! This is not a reproduction of BetaMem, FAER, or LIBERO-Recovery.
//!
//! rustc --edition=2024 --test tools/vision-memory-failure-repair.rs -o /tmp/vis-mem-failure-tests
//! /tmp/vis-mem-failure-tests
//! rustc --edition=2024 -O tools/vision-memory-failure-repair.rs -o /tmp/vis-mem-failure
//! /tmp/vis-mem-failure > vis-mem-failure.tsv

const TASK_A: u8 = 1;
const TASK_B: u8 = 2;
const OBJECT_A: u8 = 11;
const OBJECT_B: u8 = 12;
const CLAIM_LOGICAL_BYTES: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Context {
    Blocked,
    Clear,
    Occluded,
    Visible,
    UnknownCause,
}

impl Context {
    fn label(self) -> &'static str {
        match self {
            Self::Blocked => "blocked",
            Self::Clear => "clear",
            Self::Occluded => "occluded",
            Self::Visible => "visible",
            Self::UnknownCause => "unknown_cause",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Primary,
    Alternate,
    Inspect,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Alternate => "alternate",
            Self::Inspect => "inspect",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Outcome {
    Failed,
    Succeeded,
}

impl Outcome {
    fn label(self) -> &'static str {
        match self {
            Self::Failed => "failed",
            Self::Succeeded => "succeeded",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct OutcomeRecord {
    task_id: u8,
    object_id: u8,
    action: Action,
    context: Context,
    outcome: Outcome,
    source_revision: u64,
    arrival_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Memory {
    records: Vec<OutcomeRecord>,
}

impl Memory {
    fn new(records: Vec<OutcomeRecord>) -> Self {
        Self { records }
    }

    fn logical_bytes(&self) -> usize {
        self.records.len() * CLAIM_LOGICAL_BYTES
    }

    fn has_any_primary_failure(&self) -> bool {
        self.records.iter().any(|r| {
            r.action == Action::Primary && r.outcome == Outcome::Failed
        })
    }

    fn has_matching_primary_failure(&self, scenario: &Scenario) -> bool {
        self.records.iter().any(|r| {
            r.task_id == scenario.task_id
                && r.object_id == scenario.object_id
                && r.action == Action::Primary
                && r.context == scenario.current_context
                && r.outcome == Outcome::Failed
        })
    }

    fn latest_matching_primary_outcome(&self, scenario: &Scenario) -> Option<&OutcomeRecord> {
        self.records
            .iter()
            .filter(|r| {
                r.task_id == scenario.task_id
                    && r.object_id == scenario.object_id
                    && r.action == Action::Primary
                    && r.context == scenario.current_context
                    && r.source_revision <= scenario.current_revision
            })
            .max_by_key(|r| (r.source_revision, r.arrival_sequence))
    }
}

#[derive(Clone, Debug)]
struct Scenario {
    name: &'static str,
    task_id: u8,
    object_id: u8,
    current_context: Context,
    current_revision: u64,
    memory: Memory,
    evaluator_safe_action: Action,
}

fn record(
    task_id: u8,
    object_id: u8,
    action: Action,
    context: Context,
    outcome: Outcome,
    source_revision: u64,
    arrival_sequence: u64,
) -> OutcomeRecord {
    OutcomeRecord {
        task_id,
        object_id,
        action,
        context,
        outcome,
        source_revision,
        arrival_sequence,
    }
}

fn failure(task_id: u8, object_id: u8, context: Context) -> OutcomeRecord {
    record(task_id, object_id, Action::Primary, context, Outcome::Failed, 1, 1)
}

fn scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "persistent_blocker_same_context",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::Blocked,
            current_revision: 1,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Blocked)]),
            evaluator_safe_action: Action::Alternate,
        },
        Scenario {
            name: "blocker_resolved",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::Clear,
            current_revision: 2,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Blocked)]),
            evaluator_safe_action: Action::Primary,
        },
        Scenario {
            name: "similar_but_distinct_object",
            task_id: TASK_A,
            object_id: OBJECT_B,
            current_context: Context::Blocked,
            current_revision: 2,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Blocked)]),
            evaluator_safe_action: Action::Primary,
        },
        Scenario {
            name: "unrelated_task_same_action",
            task_id: TASK_B,
            object_id: OBJECT_A,
            current_context: Context::Blocked,
            current_revision: 2,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Blocked)]),
            evaluator_safe_action: Action::Primary,
        },
        Scenario {
            name: "transient_occlusion_resolved",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::Visible,
            current_revision: 2,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Occluded)]),
            evaluator_safe_action: Action::Primary,
        },
        Scenario {
            name: "changed_cause_unknown",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::UnknownCause,
            current_revision: 2,
            memory: Memory::new(vec![failure(TASK_A, OBJECT_A, Context::Blocked)]),
            evaluator_safe_action: Action::Inspect,
        },
        Scenario {
            name: "stale_failure_superseded_by_success",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::Blocked,
            current_revision: 2,
            memory: Memory::new(vec![
                failure(TASK_A, OBJECT_A, Context::Blocked),
                record(
                    TASK_A,
                    OBJECT_A,
                    Action::Primary,
                    Context::Blocked,
                    Outcome::Succeeded,
                    2,
                    2,
                ),
            ]),
            evaluator_safe_action: Action::Primary,
        },
        Scenario {
            name: "nominal_no_failure_history",
            task_id: TASK_A,
            object_id: OBJECT_A,
            current_context: Context::Clear,
            current_revision: 1,
            memory: Memory::new(Vec::new()),
            evaluator_safe_action: Action::Primary,
        },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    AlwaysRetry,
    GlobalTaboo,
    ContextKeyedSuppression,
    ConditionalRepair,
    ReplayGatedRepair,
}

impl Policy {
    const ALL: [Self; 5] = [
        Self::AlwaysRetry,
        Self::GlobalTaboo,
        Self::ContextKeyedSuppression,
        Self::ConditionalRepair,
        Self::ReplayGatedRepair,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::AlwaysRetry => "always_retry",
            Self::GlobalTaboo => "global_taboo",
            Self::ContextKeyedSuppression => "context_keyed_suppression",
            Self::ConditionalRepair => "conditional_repair",
            Self::ReplayGatedRepair => "replay_gated_repair",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuleScope {
    GlobalAction,
    ExactTaskObjectContext,
}

impl RuleScope {
    fn label(self) -> &'static str {
        match self {
            Self::GlobalAction => "global_action",
            Self::ExactTaskObjectContext => "exact_task_object_context",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RepairCandidate {
    scope: RuleScope,
    recovery_action: Action,
}

#[derive(Clone, Debug)]
struct PolicyResult {
    scenario: &'static str,
    policy: Policy,
    chosen_action: Action,
    safe_action: Action,
    success: bool,
    repeated_failure: bool,
    false_suppression: bool,
    collateral_suppression: bool,
    nominal_retained: bool,
    logical_memory_bytes: usize,
    rule_count: usize,
}

fn default_action(context: Context) -> Action {
    match context {
        Context::UnknownCause => Action::Inspect,
        Context::Blocked => Action::Primary,
        Context::Clear | Context::Occluded | Context::Visible => Action::Primary,
    }
}

fn candidate_action(candidate: RepairCandidate, scenario: &Scenario) -> Action {
    match candidate.scope {
        RuleScope::GlobalAction => {
            if scenario.memory.has_any_primary_failure() {
                candidate.recovery_action
            } else {
                default_action(scenario.current_context)
            }
        }
        RuleScope::ExactTaskObjectContext => {
            if let Some(latest) = scenario.memory.latest_matching_primary_outcome(scenario) {
                if latest.outcome == Outcome::Failed {
                    candidate.recovery_action
                } else {
                    default_action(scenario.current_context)
                }
            } else {
                default_action(scenario.current_context)
            }
        }
    }
}

fn evaluate_candidate(candidate: RepairCandidate, cases: &[Scenario]) -> (bool, usize, usize) {
    let mut regressions = 0usize;
    let mut same_context_recoveries = 0usize;
    for scenario in cases {
        let action = candidate_action(candidate, scenario);
        if action != scenario.evaluator_safe_action {
            regressions += 1;
        }
        if scenario.current_context == Context::Blocked
            && scenario.evaluator_safe_action == Action::Alternate
            && action == Action::Alternate
        {
            same_context_recoveries += 1;
        }
    }
    let has_recovery_control = cases.iter().any(|s| {
        s.current_context == Context::Blocked && s.evaluator_safe_action == Action::Alternate
    });
    (regressions == 0 && has_recovery_control && same_context_recoveries > 0, regressions, same_context_recoveries)
}

fn active_failure(memory: &Memory, scenario: &Scenario) -> bool {
    memory.latest_matching_primary_outcome(scenario)
        .is_some_and(|latest| latest.outcome == Outcome::Failed)
}

fn decide(policy: Policy, scenario: &Scenario, replay_gate_passes: bool) -> Action {
    match policy {
        Policy::AlwaysRetry => default_action(scenario.current_context),
        Policy::GlobalTaboo => {
            if scenario.memory.has_any_primary_failure() {
                Action::Alternate
            } else {
                default_action(scenario.current_context)
            }
        }
        Policy::ContextKeyedSuppression => {
            if scenario.memory.has_matching_primary_failure(scenario) {
                Action::Alternate
            } else {
                default_action(scenario.current_context)
            }
        }
        Policy::ConditionalRepair => {
            if active_failure(&scenario.memory, scenario) {
                Action::Alternate
            } else {
                default_action(scenario.current_context)
            }
        }
        Policy::ReplayGatedRepair => {
            if replay_gate_passes && active_failure(&scenario.memory, scenario) {
                Action::Alternate
            } else {
                default_action(scenario.current_context)
            }
        }
    }
}

fn evaluate(policy: Policy, scenario: &Scenario, replay_gate_passes: bool) -> PolicyResult {
    let chosen_action = decide(policy, scenario, replay_gate_passes);
    let success = chosen_action == scenario.evaluator_safe_action;
    let repeated_failure = scenario.current_context == Context::Blocked
        && scenario.evaluator_safe_action == Action::Alternate
        && chosen_action == Action::Primary;
    let false_suppression = chosen_action == Action::Alternate
        && scenario.evaluator_safe_action != Action::Alternate;
    let collateral_suppression = false_suppression
        && (scenario.task_id != TASK_A || scenario.object_id != OBJECT_A);
    let nominal_retained = scenario.name == "nominal_no_failure_history"
        && chosen_action == Action::Primary;
    PolicyResult {
        scenario: scenario.name,
        policy,
        chosen_action,
        safe_action: scenario.evaluator_safe_action,
        success,
        repeated_failure,
        false_suppression,
        collateral_suppression,
        nominal_retained,
        logical_memory_bytes: scenario.memory.logical_bytes(),
        rule_count: usize::from(active_failure(&scenario.memory, scenario)),
    }
}

fn format_report() -> String {
    let cases = scenarios();
    let unsafe_candidate = RepairCandidate {
        scope: RuleScope::GlobalAction,
        recovery_action: Action::Alternate,
    };
    let scoped_candidate = RepairCandidate {
        scope: RuleScope::ExactTaskObjectContext,
        recovery_action: Action::Alternate,
    };
    let (unsafe_promoted, unsafe_regressions, _) = evaluate_candidate(unsafe_candidate, &cases);
    let (scoped_promoted, scoped_regressions, _) = evaluate_candidate(scoped_candidate, &cases);

    let mut out = String::new();
    out.push_str("META\tformat\tVIS-MEM-013-v0.1\n");
    out.push_str("META\tscope\tindependent synthetic failure-memory diagnostic; not native robotics success\n");
    out.push_str(&format!(
        "GATE\t{}\t{}\t{}\n",
        unsafe_candidate.scope.label(),
        u8::from(unsafe_promoted),
        unsafe_regressions
    ));
    out.push_str(&format!(
        "GATE\t{}\t{}\t{}\n",
        scoped_candidate.scope.label(),
        u8::from(scoped_promoted),
        scoped_regressions
    ));
    out.push_str("CASE_HEADER\tcase\tpolicy\tcontext\tchosen_action\tevaluator_safe_action\tsuccess\trepeated_failure\tfalse_suppression\tcollateral_suppression\tnominal_retained\tlogical_memory_bytes\trule_count\n");

    let gate_passes = scoped_promoted;
    let mut results = Vec::new();
    for scenario in &cases {
        for policy in Policy::ALL {
            let result = evaluate(policy, scenario, gate_passes);
            out.push_str(&format!(
                "CASE\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                result.scenario,
                result.policy.label(),
                scenario.current_context.label(),
                result.chosen_action.label(),
                result.safe_action.label(),
                u8::from(result.success),
                u8::from(result.repeated_failure),
                u8::from(result.false_suppression),
                u8::from(result.collateral_suppression),
                u8::from(result.nominal_retained),
                result.logical_memory_bytes,
                result.rule_count
            ));
            results.push(result);
        }
    }

    out.push_str("SUMMARY_HEADER\tpolicy\tcases\tsuccess_rate\trepeat_failure_rate\tfalse_suppression_rate\tcollateral_suppression_rate\tnominal_retention\tmean_logical_memory_bytes\n");
    for policy in Policy::ALL {
        let rows: Vec<&PolicyResult> = results.iter().filter(|r| r.policy == policy).collect();
        let count = rows.len().max(1) as f64;
        let success = rows.iter().filter(|r| r.success).count() as f64 / count;
        let repeat = rows.iter().filter(|r| r.repeated_failure).count() as f64 / count;
        let suppress = rows.iter().filter(|r| r.false_suppression).count() as f64 / count;
        let collateral = rows.iter().filter(|r| r.collateral_suppression).count() as f64 / count;
        let nominal = rows.iter().filter(|r| r.nominal_retained).count() as f64;
        let bytes = rows.iter().map(|r| r.logical_memory_bytes as f64).sum::<f64>() / count;
        out.push_str(&format!(
            "SUMMARY\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\n",
            policy.label(),
            rows.len(),
            success,
            repeat,
            suppress,
            collateral,
            nominal,
            bytes
        ));
    }
    out
}

fn main() {
    print!("{}", format_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scenario(name: &str) -> Scenario {
        scenarios().into_iter().find(|s| s.name == name).unwrap()
    }

    fn action(policy: Policy, name: &str) -> Action {
        let cases = scenarios();
        let (_, _, _) = evaluate_candidate(
            RepairCandidate { scope: RuleScope::ExactTaskObjectContext, recovery_action: Action::Alternate },
            &cases,
        );
        evaluate(policy, &scenario(name), true).chosen_action
    }

    #[test]
    fn global_taboo_reduces_same_context_repeat_but_is_overgeneralized() {
        assert_eq!(action(Policy::GlobalTaboo, "persistent_blocker_same_context"), Action::Alternate);
        assert_eq!(action(Policy::GlobalTaboo, "blocker_resolved"), Action::Alternate);
        assert_eq!(action(Policy::GlobalTaboo, "unrelated_task_same_action"), Action::Alternate);
    }

    #[test]
    fn exact_context_repair_recovers_without_global_action_ban() {
        assert_eq!(action(Policy::ConditionalRepair, "persistent_blocker_same_context"), Action::Alternate);
        assert_eq!(action(Policy::ConditionalRepair, "blocker_resolved"), Action::Primary);
        assert_eq!(action(Policy::ConditionalRepair, "similar_but_distinct_object"), Action::Primary);
        assert_eq!(action(Policy::ConditionalRepair, "unrelated_task_same_action"), Action::Primary);
    }

    #[test]
    fn resolved_occlusion_allows_primary_again() {
        assert_eq!(action(Policy::ConditionalRepair, "transient_occlusion_resolved"), Action::Primary);
    }

    #[test]
    fn unknown_cause_requests_inspection_not_old_recovery() {
        assert_eq!(action(Policy::ConditionalRepair, "changed_cause_unknown"), Action::Inspect);
    }

    #[test]
    fn later_success_supersedes_the_prior_failure_rule() {
        assert_eq!(action(Policy::ContextKeyedSuppression, "stale_failure_superseded_by_success"), Action::Alternate);
        assert_eq!(action(Policy::ConditionalRepair, "stale_failure_superseded_by_success"), Action::Primary);
    }

    #[test]
    fn replay_gate_rejects_global_taboo_and_accepts_scoped_repair() {
        let cases = scenarios();
        let (ok_global, regressions_global, _) = evaluate_candidate(
            RepairCandidate { scope: RuleScope::GlobalAction, recovery_action: Action::Alternate },
            &cases,
        );
        let (ok_scoped, regressions_scoped, _) = evaluate_candidate(
            RepairCandidate { scope: RuleScope::ExactTaskObjectContext, recovery_action: Action::Alternate },
            &cases,
        );
        assert!(!ok_global);
        assert!(regressions_global > 0);
        assert!(ok_scoped);
        assert_eq!(regressions_scoped, 0);
    }

    #[test]
    fn replay_gated_policy_only_uses_accepted_repair() {
        assert_eq!(action(Policy::ReplayGatedRepair, "persistent_blocker_same_context"), Action::Alternate);
        assert_eq!(action(Policy::ReplayGatedRepair, "blocker_resolved"), Action::Primary);
    }

    #[test]
    fn always_retry_is_a_recurrence_negative_control() {
        let r = evaluate(Policy::AlwaysRetry, &scenario("persistent_blocker_same_context"), true);
        assert!(!r.success);
        assert!(r.repeated_failure);
    }

    #[test]
    fn global_ban_is_detected_as_collateral_suppression() {
        let r = evaluate(Policy::GlobalTaboo, &scenario("similar_but_distinct_object"), true);
        assert!(r.false_suppression);
        assert!(r.collateral_suppression);
    }

    #[test]
    fn successful_nominal_action_is_retained_when_no_failure_exists() {
        let r = evaluate(Policy::ConditionalRepair, &scenario("nominal_no_failure_history"), true);
        assert!(r.success);
        assert!(r.nominal_retained);
    }

    #[test]
    fn deterministic_report_contains_all_fixture_policy_pairs() {
        let a = format_report();
        let b = format_report();
        assert_eq!(a, b);
        assert_eq!(a.lines().filter(|l| l.starts_with("CASE\t")).count(), 40);
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY\t")).count(), 5);
        assert_eq!(a.lines().filter(|l| l.starts_with("GATE\t")).count(), 2);
    }

    #[test]
    fn logical_memory_accounting_is_explicitly_nonzero_for_failure_memory() {
        let r = evaluate(Policy::ConditionalRepair, &scenario("persistent_blocker_same_context"), true);
        assert_eq!(r.logical_memory_bytes, CLAIM_LOGICAL_BYTES);
    }

    #[test]
    fn evaluator_safe_action_is_not_directly_accessed_by_decider() {
        let text = include_str!("vision-memory-failure-repair.rs");
        let decider_start = text.find("fn decide(").unwrap();
        let decider_end = text[decider_start..].find("\nfn evaluate(").unwrap() + decider_start;
        let decider = &text[decider_start..decider_end];
        assert!(!decider.contains("evaluator_safe_action"));
        assert!(!decider.contains("evaluator"));
    }
}
