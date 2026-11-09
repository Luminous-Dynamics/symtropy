// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-015: paired causal diagnostic for memory admission.
//!
//! Each case is evaluated under the same visible task state with eight
//! policies. "No memory" is the matched control. The selector receives only
//! the current observation and the candidate memory metadata/content; ground
//! truth is consulted only by the scorer. This finite synthetic diagnostic is
//! not a native-robot benchmark and makes no SOTA claim.
//!
//! rustc --edition=2024 --test tools/vision-memory-causal-admission.rs -o /tmp/vis-mem-admission-tests
//! /tmp/vis-mem-admission-tests
//! rustc --edition=2024 -O tools/vision-memory-causal-admission.rs -o /tmp/vis-mem-admission
//! /tmp/vis-mem-admission > vis-mem-admission.tsv

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActionKey {
    A,
    B,
    C,
}

impl ActionKey {
    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Provenance {
    Unverified,
    SourceVerified,
    TransferValidated,
}

impl Provenance {
    fn label(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::SourceVerified => "source_verified",
            Self::TransferValidated => "transfer_validated",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    RevisionBound,
    TaskInvariant,
}

impl Scope {
    fn label(self) -> &'static str {
        match self {
            Self::RevisionBound => "revision_bound",
            Self::TaskInvariant => "task_invariant_claim",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    task_key: u8,
    world_revision: u16,
    observed_safe_action: Option<ActionKey>,
    verification_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Memory {
    task_key: u8,
    source_revision: u16,
    action: ActionKey,
    provenance: Provenance,
    scope: Scope,
}

#[derive(Clone, Debug)]
struct Scenario {
    name: &'static str,
    observation: Observation,
    memory: Option<Memory>,
    // Evaluator-only truth. Never passed to choose_decision.
    actual_safe_action: ActionKey,
}

fn obs(
    task_key: u8,
    world_revision: u16,
    observed_safe_action: Option<ActionKey>,
    verification_available: bool,
) -> Observation {
    Observation {
        task_key,
        world_revision,
        observed_safe_action,
        verification_available,
    }
}

fn mem(
    task_key: u8,
    source_revision: u16,
    action: ActionKey,
    provenance: Provenance,
    scope: Scope,
) -> Memory {
    Memory { task_key, source_revision, action, provenance, scope }
}

fn scenarios() -> Vec<Scenario> {
    use ActionKey::{A, B};
    use Provenance::{SourceVerified as SV, TransferValidated as TV, Unverified as U};
    use Scope::{RevisionBound as RB, TaskInvariant as TI};

    vec![
        Scenario { name: "no_memory_visible_control", observation: obs(1, 2, Some(A), false), memory: None, actual_safe_action: A },
        Scenario { name: "no_memory_hidden_verifiable", observation: obs(1, 2, None, true), memory: None, actual_safe_action: A },
        Scenario { name: "no_memory_hidden_unverifiable", observation: obs(1, 2, None, false), memory: None, actual_safe_action: A },
        Scenario { name: "fresh_verified_hidden", observation: obs(1, 2, None, true), memory: Some(mem(1, 2, A, SV, RB)), actual_safe_action: A },
        Scenario { name: "fresh_verified_observed", observation: obs(1, 2, Some(A), false), memory: Some(mem(1, 2, A, SV, RB)), actual_safe_action: A },
        Scenario { name: "fresh_memory_contradicts_observation", observation: obs(1, 2, Some(A), false), memory: Some(mem(1, 2, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "stale_wrong_verifiable", observation: obs(1, 2, None, true), memory: Some(mem(1, 1, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "stale_wrong_unverifiable", observation: obs(1, 2, None, false), memory: Some(mem(1, 1, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "stale_valid_transfer_certified_no_verify", observation: obs(1, 2, None, false), memory: Some(mem(1, 1, A, TV, TI)), actual_safe_action: A },
        Scenario { name: "stale_invariant_claim_without_transfer_validation", observation: obs(1, 2, None, true), memory: Some(mem(1, 1, B, SV, TI)), actual_safe_action: A },
        Scenario { name: "irrelevant_tempting_wrong_verifiable", observation: obs(1, 2, None, true), memory: Some(mem(2, 2, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "irrelevant_coincidental_action_match", observation: obs(1, 2, None, false), memory: Some(mem(2, 2, A, SV, RB)), actual_safe_action: A },
        Scenario { name: "irrelevant_memory_with_direct_evidence", observation: obs(1, 2, Some(A), false), memory: Some(mem(2, 2, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "current_unverified_wrong_verifiable", observation: obs(1, 2, None, true), memory: Some(mem(1, 2, B, U, RB)), actual_safe_action: A },
        Scenario { name: "current_unverified_correct_unverifiable", observation: obs(1, 2, None, false), memory: Some(mem(1, 2, A, U, RB)), actual_safe_action: A },
        Scenario { name: "stale_memory_overruled_by_observation", observation: obs(1, 2, Some(A), false), memory: Some(mem(1, 1, B, SV, RB)), actual_safe_action: A },
        Scenario { name: "same_revision_wrong_unverified", observation: obs(1, 2, None, true), memory: Some(mem(1, 2, B, U, RB)), actual_safe_action: A },
        Scenario { name: "stale_valid_but_not_transfer_validated", observation: obs(1, 2, None, false), memory: Some(mem(1, 1, A, SV, TI)), actual_safe_action: A },
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Decision {
    UseMemory,
    UseObservation,
    Verify,
    Abstain,
}

impl Decision {
    fn label(self) -> &'static str {
        match self {
            Self::UseMemory => "use_memory",
            Self::UseObservation => "use_observation",
            Self::Verify => "verify",
            Self::Abstain => "abstain",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    NoMemory,
    AlwaysRecall,
    TaskMatchOnly,
    RevisionOnly,
    ObservationFirst,
    FreshVerifiedGate,
    ScopeClaimGate,
    TransferValidatedGate,
}

impl Policy {
    const ALL: [Self; 8] = [
        Self::NoMemory,
        Self::AlwaysRecall,
        Self::TaskMatchOnly,
        Self::RevisionOnly,
        Self::ObservationFirst,
        Self::FreshVerifiedGate,
        Self::ScopeClaimGate,
        Self::TransferValidatedGate,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::NoMemory => "no_memory",
            Self::AlwaysRecall => "always_recall",
            Self::TaskMatchOnly => "task_match_only",
            Self::RevisionOnly => "revision_only",
            Self::ObservationFirst => "observation_first",
            Self::FreshVerifiedGate => "fresh_verified_gate",
            Self::ScopeClaimGate => "scope_claim_gate",
            Self::TransferValidatedGate => "transfer_validated_gate",
        }
    }
}

fn fallback(observation: Observation) -> Decision {
    if observation.observed_safe_action.is_some() {
        Decision::UseObservation
    } else if observation.verification_available {
        Decision::Verify
    } else {
        Decision::Abstain
    }
}

// This is the complete policy input boundary. Evaluator-only scenario fields
// (including actual_safe_action) are structurally unavailable here.
fn choose_decision(
    policy: Policy,
    observation: Observation,
    memory: Option<Memory>,
) -> Decision {
    let task_match = memory.is_some_and(|m| m.task_key == observation.task_key);
    let revision_match = memory.is_some_and(|m| m.source_revision == observation.world_revision);

    match policy {
        Policy::NoMemory => fallback(observation),
        Policy::AlwaysRecall => {
            if memory.is_some() { Decision::UseMemory } else { fallback(observation) }
        }
        Policy::TaskMatchOnly => {
            if task_match { Decision::UseMemory } else { fallback(observation) }
        }
        // Deliberately weak control: revision recency alone, ignoring task and
        // current observations.
        Policy::RevisionOnly => {
            if revision_match { Decision::UseMemory } else { fallback(observation) }
        }
        Policy::ObservationFirst => {
            if observation.observed_safe_action.is_some() {
                Decision::UseObservation
            } else if task_match {
                Decision::UseMemory
            } else {
                fallback(observation)
            }
        }
        Policy::FreshVerifiedGate => {
            if observation.observed_safe_action.is_some() {
                Decision::UseObservation
            } else if task_match
                && revision_match
                && memory.is_some_and(|m| m.provenance != Provenance::Unverified)
            {
                Decision::UseMemory
            } else if observation.verification_available {
                Decision::Verify
            } else {
                Decision::Abstain
            }
        }
        // Ablation: trusts a "task invariant" scope declaration without
        // requiring transfer-specific validation. Kept to measure this failure
        // mode, not recommended as a production gate.
        Policy::ScopeClaimGate => {
            if observation.observed_safe_action.is_some() {
                Decision::UseObservation
            } else if task_match
                && memory.is_some_and(|m| {
                    m.provenance != Provenance::Unverified
                        && (m.source_revision == observation.world_revision
                            || m.scope == Scope::TaskInvariant)
                })
            {
                Decision::UseMemory
            } else if observation.verification_available {
                Decision::Verify
            } else {
                Decision::Abstain
            }
        }
        Policy::TransferValidatedGate => {
            if observation.observed_safe_action.is_some() {
                Decision::UseObservation
            } else if task_match
                && memory.is_some_and(|m| {
                    (m.source_revision == observation.world_revision
                        && m.provenance != Provenance::Unverified)
                        || (m.source_revision != observation.world_revision
                            && m.scope == Scope::TaskInvariant
                            && m.provenance == Provenance::TransferValidated)
                })
            {
                Decision::UseMemory
            } else if observation.verification_available {
                Decision::Verify
            } else {
                Decision::Abstain
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    chosen_action: Option<ActionKey>,
    success: bool,
    wrong_action: bool,
    verification_cost: u8,
    abstained: bool,
}

fn evaluate_decision(
    decision: Decision,
    observation: Observation,
    memory: Option<Memory>,
    actual_safe_action: ActionKey,
) -> Outcome {
    let (chosen_action, verification_cost) = match decision {
        Decision::UseMemory => (memory.map(|m| m.action), 0),
        Decision::UseObservation => (observation.observed_safe_action, 0),
        Decision::Verify if observation.verification_available => (Some(actual_safe_action), 1),
        Decision::Verify => (None, 1),
        Decision::Abstain => (None, 0),
    };
    Outcome {
        chosen_action,
        success: chosen_action == Some(actual_safe_action),
        wrong_action: chosen_action.is_some_and(|a| a != actual_safe_action),
        verification_cost,
        abstained: decision == Decision::Abstain,
    }
}

#[derive(Clone, Copy, Debug)]
struct RunResult {
    case_name: &'static str,
    policy: Policy,
    decision: Decision,
    observation: Observation,
    memory: Option<Memory>,
    actual_safe_action: ActionKey,
    outcome: Outcome,
    memory_used: bool,
    helpful_use: bool,
    harmful_use: bool,
    paired_lift: i8,
    avoidable_verification: bool,
    false_abstention: bool,
}

fn run_case(scenario: &Scenario, policy: Policy) -> RunResult {
    let decision = choose_decision(policy, scenario.observation, scenario.memory);
    let outcome = evaluate_decision(
        decision,
        scenario.observation,
        scenario.memory,
        scenario.actual_safe_action,
    );

    let baseline_decision = choose_decision(
        Policy::NoMemory,
        scenario.observation,
        scenario.memory,
    );
    let baseline = evaluate_decision(
        baseline_decision,
        scenario.observation,
        scenario.memory,
        scenario.actual_safe_action,
    );

    let memory_used = decision == Decision::UseMemory;
    let helpful_use = memory_used
        && outcome.success
        && (!baseline.success || outcome.verification_cost < baseline.verification_cost);
    let harmful_use = memory_used && !outcome.success && baseline.success;
    let paired_lift = (if outcome.success { 1i8 } else { 0i8 }) - (if baseline.success { 1i8 } else { 0i8 });

    let trusted_current_memory = scenario.memory.is_some_and(|m| {
        m.task_key == scenario.observation.task_key
            && m.source_revision == scenario.observation.world_revision
            && m.provenance != Provenance::Unverified
    });
    let validated_transfer_memory = scenario.memory.is_some_and(|m| {
        m.task_key == scenario.observation.task_key
            && m.source_revision != scenario.observation.world_revision
            && m.scope == Scope::TaskInvariant
            && m.provenance == Provenance::TransferValidated
    });
    let avoidable_verification = decision == Decision::Verify
        && (scenario.observation.observed_safe_action.is_some()
            || trusted_current_memory
            || validated_transfer_memory);
    let false_abstention = outcome.abstained
        && (scenario.observation.observed_safe_action.is_some()
            || scenario.observation.verification_available
            || trusted_current_memory
            || validated_transfer_memory);

    RunResult {
        case_name: scenario.name,
        policy,
        decision,
        observation: scenario.observation,
        memory: scenario.memory,
        actual_safe_action: scenario.actual_safe_action,
        outcome,
        memory_used,
        helpful_use,
        harmful_use,
        paired_lift,
        avoidable_verification,
        false_abstention,
    }
}

fn frac(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 }
}

fn push_record(out: &mut String, fields: &[String]) {
    out.push_str(&fields.join("\t"));
    out.push('\n');
}

fn flag(value: bool) -> String {
    u8::from(value).to_string()
}

fn opt_action(value: Option<ActionKey>) -> String {
    value.map(ActionKey::label).unwrap_or("none").to_owned()
}

fn report() -> String {
    let cases = scenarios();
    let mut rows = Vec::new();
    for scenario in &cases {
        for policy in Policy::ALL {
            rows.push(run_case(scenario, policy));
        }
    }

    let mut out = String::new();
    push_record(&mut out, &["META".into(), "format".into(), "VIS-MEM-015-v0.1".into()]);
    push_record(&mut out, &["META".into(), "scope".into(), "paired synthetic memory-admission diagnostic; not native benchmark performance".into()]);
    push_record(&mut out, &["META".into(), "policy_truth_boundary".into(), "actual_safe_action is scorer-only and absent from choose_decision inputs".into()]);
    push_record(&mut out, &[
        "CASE_HEADER".into(), "case".into(), "policy".into(), "goal_key".into(),
        "world_revision".into(), "observed_action".into(), "verification_available".into(),
        "memory_available".into(), "memory_task_match".into(), "memory_revision_match".into(),
        "memory_action".into(), "memory_provenance".into(), "memory_scope".into(),
        "actual_safe_action".into(), "decision".into(), "executed_action".into(),
        "success".into(), "wrong_action".into(), "memory_used".into(), "helpful_use".into(),
        "harmful_use".into(), "paired_lift_vs_no_memory".into(), "verification_cost".into(),
        "avoidable_verification".into(), "abstained".into(), "false_abstention".into(),
    ]);

    for r in &rows {
        let task_match = r.memory.is_some_and(|m| m.task_key == r.observation.task_key);
        let revision_match = r.memory.is_some_and(|m| m.source_revision == r.observation.world_revision);
        push_record(&mut out, &[
            "CASE".into(),
            r.case_name.into(),
            r.policy.label().into(),
            r.observation.task_key.to_string(),
            r.observation.world_revision.to_string(),
            opt_action(r.observation.observed_safe_action),
            flag(r.observation.verification_available),
            flag(r.memory.is_some()),
            flag(task_match),
            flag(revision_match),
            r.memory.map(|m| m.action.label()).unwrap_or("none").into(),
            r.memory.map(|m| m.provenance.label()).unwrap_or("none").into(),
            r.memory.map(|m| m.scope.label()).unwrap_or("none").into(),
            r.actual_safe_action.label().into(),
            r.decision.label().into(),
            opt_action(r.outcome.chosen_action),
            flag(r.outcome.success),
            flag(r.outcome.wrong_action),
            flag(r.memory_used),
            flag(r.helpful_use),
            flag(r.harmful_use),
            r.paired_lift.to_string(),
            r.outcome.verification_cost.to_string(),
            flag(r.avoidable_verification),
            flag(r.outcome.abstained),
            flag(r.false_abstention),
        ]);
    }

    push_record(&mut out, &[
        "SUMMARY_HEADER".into(), "policy".into(), "cases".into(), "success_rate".into(),
        "wrong_action_rate".into(), "memory_use_rate_when_available".into(), "helpful_use_rate_among_uses".into(),
        "harmful_use_rate_among_uses".into(), "mean_paired_lift_vs_no_memory".into(),
        "verification_rate".into(), "avoidable_verification_rate_among_verifications".into(),
        "abstention_rate".into(), "false_abstention_rate_among_abstentions".into(),
        "mean_verification_cost".into(),
    ]);

    for policy in Policy::ALL {
        let policy_rows: Vec<&RunResult> = rows.iter().filter(|r| r.policy == policy).collect();
        let available = policy_rows.iter().filter(|r| r.memory.is_some()).count();
        let used = policy_rows.iter().filter(|r| r.memory_used).count();
        let helpful = policy_rows.iter().filter(|r| r.helpful_use).count();
        let harmful = policy_rows.iter().filter(|r| r.harmful_use).count();
        let successful = policy_rows.iter().filter(|r| r.outcome.success).count();
        let wrong = policy_rows.iter().filter(|r| r.outcome.wrong_action).count();
        let verified: Vec<&RunResult> = policy_rows.iter().copied().filter(|r| r.decision == Decision::Verify).collect();
        let avoidable = verified.iter().filter(|r| r.avoidable_verification).count();
        let abstained: Vec<&RunResult> = policy_rows.iter().copied().filter(|r| r.outcome.abstained).collect();
        let false_abstain = abstained.iter().filter(|r| r.false_abstention).count();
        let lift_sum: i32 = policy_rows.iter().map(|r| i32::from(r.paired_lift)).sum();
        let verify_cost_sum: usize = policy_rows.iter().map(|r| usize::from(r.outcome.verification_cost)).sum();
        push_record(&mut out, &[
            "SUMMARY".into(),
            policy.label().into(),
            policy_rows.len().to_string(),
            format!("{:.3}", frac(successful, policy_rows.len())),
            format!("{:.3}", frac(wrong, policy_rows.len())),
            format!("{:.3}", frac(used, available)),
            format!("{:.3}", frac(helpful, used)),
            format!("{:.3}", frac(harmful, used)),
            format!("{:.3}", lift_sum as f64 / policy_rows.len().max(1) as f64),
            format!("{:.3}", frac(verified.len(), policy_rows.len())),
            format!("{:.3}", frac(avoidable, verified.len())),
            format!("{:.3}", frac(abstained.len(), policy_rows.len())),
            format!("{:.3}", frac(false_abstain, abstained.len())),
            format!("{:.3}", verify_cost_sum as f64 / policy_rows.len().max(1) as f64),
        ]);
    }
    out
}

fn main() {
    print!("{}", report());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(name: &str) -> Scenario {
        scenarios().into_iter().find(|s| s.name == name).unwrap()
    }

    #[test]
    fn verified_fresh_memory_can_replace_verification_without_changing_success() {
        let s = case("fresh_verified_hidden");
        let baseline = run_case(&s, Policy::NoMemory);
        let memory = run_case(&s, Policy::FreshVerifiedGate);
        assert!(baseline.outcome.success);
        assert!(memory.outcome.success);
        assert_eq!(baseline.outcome.verification_cost, 1);
        assert_eq!(memory.outcome.verification_cost, 0);
        assert!(memory.helpful_use);
        assert_eq!(memory.paired_lift, 0);
    }

    #[test]
    fn fresh_transfer_certificate_can_make_old_memory_useful_without_verification() {
        let s = case("stale_valid_transfer_certified_no_verify");
        let baseline = run_case(&s, Policy::NoMemory);
        let gated = run_case(&s, Policy::TransferValidatedGate);
        assert!(!baseline.outcome.success);
        assert!(gated.outcome.success);
        assert_eq!(gated.decision, Decision::UseMemory);
        assert_eq!(gated.paired_lift, 1);
        assert!(gated.helpful_use);
    }

    #[test]
    fn transfer_gate_rejects_unvalidated_invariance_claim_and_verifies() {
        let s = case("stale_invariant_claim_without_transfer_validation");
        let scope_claim = run_case(&s, Policy::ScopeClaimGate);
        let validated = run_case(&s, Policy::TransferValidatedGate);
        assert!(scope_claim.outcome.wrong_action);
        assert!(scope_claim.harmful_use);
        assert_eq!(validated.decision, Decision::Verify);
        assert!(validated.outcome.success);
        assert!(!validated.harmful_use);
    }

    #[test]
    fn current_observation_overrides_conflicting_or_irrelevant_memory() {
        for name in ["fresh_memory_contradicts_observation", "irrelevant_memory_with_direct_evidence", "stale_memory_overruled_by_observation"] {
            let s = case(name);
            let r = run_case(&s, Policy::TransferValidatedGate);
            assert_eq!(r.decision, Decision::UseObservation);
            assert!(r.outcome.success);
            assert!(!r.memory_used);
        }
    }

    #[test]
    fn task_match_gate_avoids_an_irrelevant_memory_but_always_recall_does_not() {
        let s = case("irrelevant_tempting_wrong_verifiable");
        let always = run_case(&s, Policy::AlwaysRecall);
        let matched = run_case(&s, Policy::TaskMatchOnly);
        assert!(always.outcome.wrong_action);
        assert_eq!(matched.decision, Decision::Verify);
        assert!(matched.outcome.success);
    }

    #[test]
    fn recency_only_is_not_enough_when_same_revision_memory_is_irrelevant() {
        let s = case("irrelevant_tempting_wrong_verifiable");
        assert_eq!(run_case(&s, Policy::RevisionOnly).decision, Decision::UseMemory);
        assert_eq!(run_case(&s, Policy::TransferValidatedGate).decision, Decision::Verify);
    }

    #[test]
    fn transfer_gate_abstains_instead_of_inventing_success_without_evidence_or_verification() {
        let s = case("stale_wrong_unverifiable");
        let r = run_case(&s, Policy::TransferValidatedGate);
        assert_eq!(r.decision, Decision::Abstain);
        assert!(!r.outcome.success);
        assert!(!r.false_abstention);
    }

    #[test]
    fn every_policy_respects_verification_availability() {
        for s in scenarios().iter().filter(|s| !s.observation.verification_available) {
            for policy in Policy::ALL {
                let r = run_case(s, policy);
                assert_ne!(r.decision, Decision::Verify, "{} / {}", s.name, policy.label());
            }
        }
    }

    #[test]
    fn policy_selector_cannot_read_evaluator_truth_or_scenario_labels() {
        let source = include_str!("vision-memory-causal-admission.rs");
        let start = source.find("fn choose_decision(").unwrap();
        let end = source[start..].find("\nfn evaluate_decision(").unwrap() + start;
        let selector = &source[start..end];
        assert!(!selector.contains("Scenario"));
        assert!(!selector.contains("actual_safe_action"));
        assert!(!selector.contains("success"));
        assert!(!selector.contains("case_name"));
    }

    #[test]
    fn report_is_deterministic_and_schema_counts_are_fixed() {
        let a = report();
        let b = report();
        assert_eq!(a, b);
        assert_eq!(a.lines().filter(|l| l.starts_with("CASE_HEADER\t")).count(), 1);
        assert_eq!(a.lines().filter(|l| l.starts_with("CASE\t")).count(), 18 * Policy::ALL.len());
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY_HEADER\t")).count(), 1);
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY\t")).count(), Policy::ALL.len());
        for line in a.lines() {
            if line.starts_with("CASE_HEADER\t") || line.starts_with("CASE\t") {
                assert_eq!(line.split('\t').count(), 26, "bad case schema: {line}");
            }
            if line.starts_with("SUMMARY_HEADER\t") || line.starts_with("SUMMARY\t") {
                assert_eq!(line.split('\t').count(), 14, "bad summary schema: {line}");
            }
        }
    }
}
