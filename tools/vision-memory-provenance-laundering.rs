// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-018: provenance laundering and taint containment.
//!
//! Dependency-free synthetic security-contract diagnostic. It tests whether
//! a memory whose root, ancestry, or revocation status is not admissible can
//! influence a later action after being summarized. Evaluator-only attack
//! labels and correct actions are never passed to choose_decision.
//!
//! This is not a native robotics benchmark or a cryptographic implementation.
//!
//! rustc --edition=2024 --test tools/vision-memory-provenance-laundering.rs -o /tmp/vis-mem-provenance-tests
//! /tmp/vis-mem-provenance-tests
//! rustc --edition=2024 -O tools/vision-memory-provenance-laundering.rs -o /tmp/vis-mem-provenance
//! /tmp/vis-mem-provenance > vis-mem-provenance.tsv

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    A,
    B,
    C,
}

impl Action {
    fn label(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
            Self::C => "C",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrustClaim {
    Trusted,
    Untrusted,
}

impl TrustClaim {
    fn label(self) -> &'static str {
        match self {
            Self::Trusted => "trusted_claim",
            Self::Untrusted => "untrusted_claim",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Observation {
    task_key: u8,
    world_revision: u16,
    observed_action: Option<Action>,
    verification_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Memory {
    task_key: u8,
    action: Action,
    trust_claim: TrustClaim,
    // These flags stand for outcomes of an independent verification and
    // lineage index. They are not parsed from free-form memory prose.
    root_authenticated: bool,
    lineage_complete: bool,
    ancestors_authenticated: bool,
    inherited_taint: bool,
    source_revoked: bool,
    ancestor_revoked: bool,
    lineage_depth: u8,
    source_revision: u16,
}

#[derive(Clone, Debug)]
struct Scenario {
    name: &'static str,
    observation: Observation,
    memory: Option<Memory>,
    // Evaluator-only; never passed to choose_decision.
    attack_label: AttackLabel,
    actual_safe_action: Action,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AttackLabel {
    Clean,
    TrustSpoof,
    TaintedSummary,
    MissingParent,
    RevokedSource,
    RevokedAncestor,
    DirectContradiction,
    UnverifiableUnknown,
}

impl AttackLabel {
    fn label(self) -> &'static str {
        match self {
            Self::Clean => "clean",
            Self::TrustSpoof => "trust_spoof",
            Self::TaintedSummary => "tainted_summary",
            Self::MissingParent => "missing_parent",
            Self::RevokedSource => "revoked_source",
            Self::RevokedAncestor => "revoked_ancestor",
            Self::DirectContradiction => "direct_contradiction",
            Self::UnverifiableUnknown => "unverifiable_unknown",
        }
    }

    fn is_poisoned(self) -> bool {
        matches!(
            self,
            Self::TrustSpoof
                | Self::TaintedSummary
                | Self::MissingParent
                | Self::RevokedSource
                | Self::RevokedAncestor
        )
    }
}

fn no_observation(task_key: u8, world_revision: u16, verification_available: bool) -> Observation {
    Observation {
        task_key,
        world_revision,
        observed_action: None,
        verification_available,
    }
}

fn memory(
    task_key: u8,
    action: Action,
    trust_claim: TrustClaim,
    root_authenticated: bool,
    lineage_complete: bool,
    ancestors_authenticated: bool,
    inherited_taint: bool,
    source_revoked: bool,
    ancestor_revoked: bool,
    lineage_depth: u8,
    source_revision: u16,
) -> Memory {
    Memory {
        task_key,
        action,
        trust_claim,
        root_authenticated,
        lineage_complete,
        ancestors_authenticated,
        inherited_taint,
        source_revoked,
        ancestor_revoked,
        lineage_depth,
        source_revision,
    }
}

fn scenarios() -> Vec<Scenario> {
    use Action::{A, B};
    use AttackLabel::{
        Clean, DirectContradiction, MissingParent, RevokedAncestor, RevokedSource,
        TaintedSummary, TrustSpoof, UnverifiableUnknown,
    };
    use TrustClaim::{Trusted, Untrusted};

    vec![
        Scenario {
            name: "clean_authenticated_summary",
            observation: no_observation(1, 4, false),
            memory: Some(memory(1, A, Trusted, true, true, true, false, false, false, 2, 4)),
            attack_label: Clean,
            actual_safe_action: A,
        },
        Scenario {
            name: "claimed_trust_forged_root",
            observation: no_observation(1, 4, true),
            memory: Some(memory(1, B, Trusted, false, true, true, false, false, false, 1, 4)),
            attack_label: TrustSpoof,
            actual_safe_action: A,
        },
        Scenario {
            name: "tainted_fact_laundered_by_summary",
            observation: no_observation(1, 4, true),
            memory: Some(memory(1, B, Trusted, true, true, true, true, false, false, 3, 4)),
            attack_label: TaintedSummary,
            actual_safe_action: A,
        },
        Scenario {
            name: "summary_missing_parent_link",
            observation: no_observation(1, 4, true),
            memory: Some(memory(1, B, Trusted, true, false, false, false, false, false, 2, 4)),
            attack_label: MissingParent,
            actual_safe_action: A,
        },
        Scenario {
            name: "revoked_root_after_payload_copy",
            observation: no_observation(1, 4, true),
            memory: Some(memory(1, B, Trusted, true, true, true, false, true, false, 3, 3)),
            attack_label: RevokedSource,
            actual_safe_action: A,
        },
        Scenario {
            name: "revoked_ancestor_but_clean_child_claim",
            observation: no_observation(1, 4, true),
            memory: Some(memory(1, B, Trusted, true, true, true, false, false, true, 4, 4)),
            attack_label: RevokedAncestor,
            actual_safe_action: A,
        },
        Scenario {
            name: "authenticated_live_observation_contradicts_memory",
            observation: Observation {
                task_key: 1,
                world_revision: 4,
                observed_action: Some(A),
                verification_available: false,
            },
            memory: Some(memory(1, B, Trusted, true, true, true, false, false, false, 2, 2)),
            attack_label: DirectContradiction,
            actual_safe_action: A,
        },
        Scenario {
            name: "no_lineage_no_verification",
            observation: no_observation(1, 4, false),
            memory: Some(memory(1, B, Untrusted, true, false, false, false, false, false, 0, 4)),
            attack_label: UnverifiableUnknown,
            actual_safe_action: A,
        },
        Scenario {
            name: "valid_untrusted_claim_but_authenticated_root",
            observation: no_observation(1, 4, false),
            memory: Some(memory(1, A, Untrusted, true, true, true, false, false, false, 1, 4)),
            attack_label: Clean,
            actual_safe_action: A,
        },
        Scenario {
            name: "valid_task_mismatch_control",
            observation: no_observation(1, 4, true),
            memory: Some(memory(2, B, Trusted, true, true, true, false, false, false, 1, 4)),
            attack_label: UnverifiableUnknown,
            actual_safe_action: A,
        },
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
    AlwaysUse,
    LabelOnly,
    RootAuthenticationOnly,
    TaintPropagating,
    LineageRevocationAware,
}

impl Policy {
    const ALL: [Self; 5] = [
        Self::AlwaysUse,
        Self::LabelOnly,
        Self::RootAuthenticationOnly,
        Self::TaintPropagating,
        Self::LineageRevocationAware,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::AlwaysUse => "always_use",
            Self::LabelOnly => "label_only",
            Self::RootAuthenticationOnly => "root_auth_only",
            Self::TaintPropagating => "taint_propagating",
            Self::LineageRevocationAware => "lineage_revocation_aware",
        }
    }
}

fn fallback(observation: Observation) -> Decision {
    if observation.observed_action.is_some() {
        Decision::UseObservation
    } else if observation.verification_available {
        Decision::Verify
    } else {
        Decision::Abstain
    }

// This is the complete policy input boundary. Evaluator-only attack labels and
// actual_safe_action are not part of this function's input signature.
fn choose_decision(
    policy: Policy,
    observation: Observation,
    candidate: Option<Memory>,
) -> Decision {
    if policy != Policy::AlwaysUse && observation.observed_action.is_some() {
        return Decision::UseObservation;
    }

    let Some(memory) = candidate else {
        return fallback(observation);
    };

    match policy {
        Policy::AlwaysUse => Decision::UseMemory,
        Policy::LabelOnly => {
            if memory.trust_claim == TrustClaim::Trusted {
                Decision::UseMemory
            } else {
                fallback(observation)
            }
        }
        Policy::RootAuthenticationOnly => {
            if memory.root_authenticated {
                Decision::UseMemory
            } else {
                fallback(observation)
            }
        }
        Policy::TaintPropagating => {
            if memory.task_key == observation.task_key
                && memory.root_authenticated
                && memory.lineage_complete
                && memory.ancestors_authenticated
                && !memory.inherited_taint
                && !memory.source_revoked
            {
                Decision::UseMemory
            } else {
                fallback(observation)
            }
        }
        Policy::LineageRevocationAware => {
            if memory.task_key == observation.task_key
                && memory.root_authenticated
                && memory.lineage_complete
                && memory.ancestors_authenticated
                && !memory.inherited_taint
                && !memory.source_revoked
                && !memory.ancestor_revoked
                && memory.source_revision >= observation.world_revision
            {
                Decision::UseMemory
            } else {
                fallback(observation)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Outcome {
    chosen_action: Option<Action>,
    success: bool,
    wrong_action: bool,
    verification_cost: u8,
    abstained: bool,
}

fn score_decision(
    decision: Decision,
    observation: Observation,
    candidate: Option<Memory>,
    actual_safe_action: Action,
) -> Outcome {
    let (chosen_action, verification_cost) = match decision {
        Decision::UseMemory => (candidate.map(|memory| memory.action), 0),
        Decision::UseObservation => (observation.observed_action, 0),
        Decision::Verify if observation.verification_available => (Some(actual_safe_action), 1),
        Decision::Verify => (None, 1),
        Decision::Abstain => (None, 0),
    };
    Outcome {
        chosen_action,
        success: chosen_action == Some(actual_safe_action),
        wrong_action: chosen_action.is_some_and(|action| action != actual_safe_action),
        verification_cost,
        abstained: decision == Decision::Abstain,
    }
}

#[derive(Clone, Copy, Debug)]
struct ResultRow {
    scenario: &'static str,
    policy: Policy,
    observation: Observation,
    candidate: Option<Memory>,
    attack_label: AttackLabel,
    decision: Decision,
    outcome: Outcome,
    memory_used: bool,
    poison_use: bool,
    poison_contained: bool,
    false_quarantine: bool,
    revoked_use: bool,
    missing_lineage_use: bool,
}

fn run_case(scenario: &Scenario, policy: Policy) -> ResultRow {
    let decision = choose_decision(policy, scenario.observation, scenario.memory);
    let outcome = score_decision(decision, scenario.observation, scenario.memory, scenario.actual_safe_action);
    let memory_used = decision == Decision::UseMemory;
    let poison_use = memory_used && scenario.attack_label.is_poisoned();
    let poison_contained = scenario.attack_label.is_poisoned() && !memory_used;
    let legitimate_candidate = scenario.attack_label == AttackLabel::Clean
        && scenario.memory.is_some_and(|memory| {
            memory.root_authenticated
                && memory.lineage_complete
                && memory.ancestors_authenticated
                && !memory.inherited_taint
                && !memory.source_revoked
                && !memory.ancestor_revoked
                && memory.task_key == scenario.observation.task_key
        });
    let false_quarantine = legitimate_candidate && !memory_used && scenario.observation.observed_action.is_none();
    let revoked_use = memory_used && scenario.memory.is_some_and(|memory| {
        memory.source_revoked || memory.ancestor_revoked
    });
    let missing_lineage_use = memory_used && scenario.memory.is_some_and(|memory| {
        !memory.lineage_complete || !memory.ancestors_authenticated
    });

    ResultRow {
        scenario: scenario.name,
        policy,
        observation: scenario.observation,
        candidate: scenario.memory,
        attack_label: scenario.attack_label,
        decision,
        outcome,
        memory_used,
        poison_use,
        poison_contained,
        false_quarantine,
        revoked_use,
        missing_lineage_use,
    }
}

fn flag(value: bool) -> String {
    if value { "1".to_owned() } else { "0".to_owned() }
}

fn frac(n: usize, d: usize) -> f64 {
    if d == 0 { 0.0 } else { n as f64 / d as f64 }
}

fn push(out: &mut String, values: &[String]) {
    out.push_str(&values.join("\t"));
    out.push('\n');
}

fn opt_action(value: Option<Action>) -> String {
    value.map(Action::label).unwrap_or("none").to_owned()
}

fn report() -> String {
    let cases = scenarios();
    let mut results = Vec::new();
    for scenario in &cases {
        for policy in Policy::ALL {
            results.push(run_case(scenario, policy));
        }
    }

    let mut out = String::new();
    push(&mut out, &["META".into(), "format".into(), "VIS-MEM-018-v0.1".into()]);
    push(&mut out, &["META".into(), "scope".into(), "synthetic provenance laundering diagnostic; not cryptographic or native-robot validation".into()]);
    push(&mut out, &["META".into(), "policy_truth_boundary".into(), "attack labels and actual correct actions remain evaluator-only".into()]);
    push(&mut out, &[
        "CASE_HEADER".into(), "case".into(), "policy".into(), "attack_label_evaluator_only".into(),
        "task_key".into(), "world_revision".into(), "observed_action".into(), "verification_available".into(),
        "memory_available".into(), "memory_task_key".into(), "candidate_action".into(),
        "trust_claim".into(), "root_authenticated".into(), "lineage_complete".into(),
        "ancestors_authenticated".into(), "inherited_taint".into(), "source_revoked".into(),
        "ancestor_revoked".into(), "lineage_depth".into(), "source_revision".into(),
        "decision".into(), "executed_action".into(), "success".into(), "wrong_action".into(),
        "memory_used".into(), "poison_use".into(), "poison_contained".into(),
        "false_quarantine".into(), "revoked_memory_use".into(), "missing_lineage_use".into(),
        "verification_cost".into(), "abstained".into(),
    ]);

    for r in &results {
        let m = r.candidate;
        push(&mut out, &[
            "CASE".into(), r.scenario.into(), r.policy.label().into(), r.attack_label.label().into(),
            r.observation.task_key.to_string(), r.observation.world_revision.to_string(),
            opt_action(r.observation.observed_action), flag(r.observation.verification_available),
            flag(m.is_some()), m.map(|v| v.task_key.to_string()).unwrap_or_else(|| "NA".into()),
            m.map(|v| v.action.label()).unwrap_or("none").into(),
            m.map(|v| v.trust_claim.label()).unwrap_or("none").into(),
            flag(m.is_some_and(|v| v.root_authenticated)),
            flag(m.is_some_and(|v| v.lineage_complete)),
            flag(m.is_some_and(|v| v.ancestors_authenticated)),
            flag(m.is_some_and(|v| v.inherited_taint)),
            flag(m.is_some_and(|v| v.source_revoked)),
            flag(m.is_some_and(|v| v.ancestor_revoked)),
            m.map(|v| v.lineage_depth.to_string()).unwrap_or_else(|| "NA".into()),
            m.map(|v| v.source_revision.to_string()).unwrap_or_else(|| "NA".into()),
            r.decision.label().into(), opt_action(r.outcome.chosen_action),
            flag(r.outcome.success), flag(r.outcome.wrong_action), flag(r.memory_used),
            flag(r.poison_use), flag(r.poison_contained), flag(r.false_quarantine),
            flag(r.revoked_use), flag(r.missing_lineage_use),
            r.outcome.verification_cost.to_string(), flag(r.outcome.abstained),
        ]);
    }

    push(&mut out, &[
        "SUMMARY_HEADER".into(), "policy".into(), "cases".into(), "success_rate".into(),
        "wrong_action_rate".into(), "poisoned_memory_use_rate".into(),
        "poison_containment_rate".into(), "false_quarantine_rate_on_clean_candidates".into(),
        "revoked_memory_use_rate".into(), "missing_lineage_memory_use_rate".into(),
        "verification_rate".into(), "abstention_rate".into(), "mean_verification_cost".into(),
    ]);

    for policy in Policy::ALL {
        let rows: Vec<&ResultRow> = results.iter().filter(|row| row.policy == policy).collect();
        let poisoned: Vec<&ResultRow> = rows.iter().copied()
            .filter(|row| row.attack_label.is_poisoned()).collect();
        let clean_candidates: Vec<&ResultRow> = rows.iter().copied()
            .filter(|row| row.attack_label == AttackLabel::Clean
                && row.candidate.is_some_and(|m| m.root_authenticated
                    && m.lineage_complete && m.ancestors_authenticated
                    && !m.inherited_taint && !m.source_revoked && !m.ancestor_revoked
                    && m.task_key == row.observation.task_key)
                && row.observation.observed_action.is_none()).collect();
        let verification_count = rows.iter().filter(|row| row.decision == Decision::Verify).count();
        let abstentions = rows.iter().filter(|row| row.outcome.abstained).count();
        push(&mut out, &[
            "SUMMARY".into(), policy.label().into(), rows.len().to_string(),
            format!("{:.3}", frac(rows.iter().filter(|r| r.outcome.success).count(), rows.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.outcome.wrong_action).count(), rows.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.poison_use).count(), poisoned.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.poison_contained).count(), poisoned.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.false_quarantine).count(), clean_candidates.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.revoked_use).count(), poisoned.len())),
            format!("{:.3}", frac(rows.iter().filter(|r| r.missing_lineage_use).count(), poisoned.len())),
            format!("{:.3}", frac(verification_count, rows.len())),
            format!("{:.3}", frac(abstentions, rows.len())),
            format!("{:.3}", rows.iter().map(|r| f64::from(r.outcome.verification_cost)).sum::<f64>() / rows.len().max(1) as f64),
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

    fn scenario(name: &str) -> Scenario {
        scenarios().into_iter().find(|case| case.name == name).unwrap()
    }

    fn run(policy: Policy, name: &str) -> ResultRow {
        run_case(&scenario(name), policy)
    }

    #[test]
    fn label_only_policy_accepts_a_forged_trust_label() {
        assert_eq!(run(Policy::LabelOnly, "claimed_trust_forged_root").decision, Decision::UseMemory);
        assert!(run(Policy::LineageRevocationAware, "claimed_trust_forged_root").poison_contained);
    }

    #[test]
    fn root_authentication_without_transitive_taint_check_accepts_laundered_summary() {
        assert!(run(Policy::RootAuthenticationOnly, "tainted_fact_laundered_by_summary").poison_use);
        assert!(run(Policy::TaintPropagating, "tainted_fact_laundered_by_summary").poison_contained);
    }

    #[test]
    fn taint_survives_summary_and_missing_parent_is_quarantined() {
        assert_eq!(run(Policy::TaintPropagating, "tainted_fact_laundered_by_summary").decision, Decision::Verify);
        assert_eq!(run(Policy::TaintPropagating, "summary_missing_parent_link").decision, Decision::Verify);
    }

    #[test]
    fn revocation_is_checked_on_ancestor_chain_not_just_leaf() {
        assert!(run(Policy::TaintPropagating, "revoked_ancestor_but_clean_child_claim").memory_used);
        assert!(run(Policy::LineageRevocationAware, "revoked_ancestor_but_clean_child_claim").poison_contained);
        assert!(run(Policy::LineageRevocationAware, "revoked_root_after_payload_copy").poison_contained);
    }

    #[test]
    fn live_observation_overrides_old_or_contradictory_memory() {
        let r = run(Policy::LineageRevocationAware, "authenticated_live_observation_contradicts_memory");
        assert_eq!(r.decision, Decision::UseObservation);
        assert!(r.outcome.success);
        assert!(!r.memory_used);
    }

    #[test]
    fn valid_authenticated_derivative_is_not_quarantined() {
        let r = run(Policy::LineageRevocationAware, "clean_authenticated_summary");
        assert!(r.memory_used);
        assert!(r.outcome.success);
        assert!(!r.false_quarantine);
    }

    #[test]
    fn no_lineage_and_no_verification_causes_abstention_not_fabricated_success() {
        let r = run(Policy::LineageRevocationAware, "no_lineage_no_verification");
        assert_eq!(r.decision, Decision::Abstain);
        assert!(!r.outcome.success);
    }

    #[test]
    fn policy_selector_cannot_read_evaluator_truth() {
        let src = include_str!("vision-memory-provenance-laundering.rs");
        let start = src.find("fn choose_decision(").unwrap();
        let end = src[start..].find("\n#[derive(Clone, Copy, Debug)]\nstruct Outcome").unwrap() + start;
        let selector = &src[start..end];
        assert!(!selector.contains("Scenario"));
        assert!(!selector.contains("attack_label"));
        assert!(!selector.contains("actual_safe_action"));
        assert!(!selector.contains("is_poisoned"));
    }

    #[test]
    fn deterministic_report_has_complete_case_and_summary_schemas() {
        let a = report();
        assert_eq!(a, report());
        assert_eq!(a.lines().filter(|l| l.starts_with("CASE\t")).count(), 10 * Policy::ALL.len());
        assert_eq!(a.lines().filter(|l| l.starts_with("SUMMARY\t")).count(), Policy::ALL.len());
        for line in a.lines() {
            if line.starts_with("CASE_HEADER\t") || line.starts_with("CASE\t") {
                assert_eq!(line.split('\t').count(), 32, "case schema: {line}");
            }
            if line.starts_with("SUMMARY_HEADER\t") || line.starts_with("SUMMARY\t") {
                assert_eq!(line.split('\t').count(), 13, "summary schema: {line}");
            }
        }
    }
}
