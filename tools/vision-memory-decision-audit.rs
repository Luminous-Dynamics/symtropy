// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! VIS-MEM-012: counterfactual decision-value audit and ambiguity handling.
//!
//! Every paired case uses the same current-observation digest in both latent
//! worlds. The policy sees only that digest, the request, and its memory bank.
//! Evaluator-only labels are used after the action is frozen.
//!
//! This is an independent synthetic audit, not a reproduction of CMA, STALE,
//! TANGLE, or a native robotics result.
//!
//! rustc --edition=2024 --test tools/vision-memory-decision-audit.rs -o /tmp/vis-mem-decision-tests
//! /tmp/vis-mem-decision-tests
//! rustc --edition=2024 -O tools/vision-memory-decision-audit.rs -o /tmp/vis-mem-decision-audit
//! /tmp/vis-mem-decision-audit > vis-mem-decision-audit.tsv

const TASK_ID: u8 = 1;
const IRRELEVANT_TASK_ID: u8 = 9;
const SAME_PRESENT_DIGEST: &str = "same-present-8b3f1c";
const CLAIM_LOGICAL_BYTES: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Origin {
    Left,
    Right,
}

impl Origin {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    Left,
    Right,
    Clarify,
}

impl Action {
    fn for_origin(origin: Origin) -> Self {
        match origin {
            Origin::Left => Self::Left,
            Origin::Right => Self::Right,
        }
    }

    fn origin(self) -> Option<Origin> {
        match self {
            Self::Left => Some(Origin::Left),
            Self::Right => Some(Origin::Right),
            Self::Clarify => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Clarify => "clarify",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Claim {
    task_id: u8,
    origin: Origin,
    authority: u8,
    source_time: u16,
    arrival_time: u16,
    source_id: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct MemoryBank {
    claims: Vec<Claim>,
}

impl MemoryBank {
    fn new(claims: Vec<Claim>) -> Self {
        Self { claims }
    }

    fn logical_bytes(&self) -> usize {
        self.claims.len() * CLAIM_LOGICAL_BYTES
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PolicyRequest {
    task_id: u8,
    current_digest: &'static str,
    prompt_premise: Option<Origin>,
}

#[derive(Clone, Debug)]
struct FixturePair {
    name: &'static str,
    left_target: Option<Origin>,
    right_target: Option<Origin>,
    left_request: PolicyRequest,
    right_request: PolicyRequest,
    left_bank: MemoryBank,
    right_bank: MemoryBank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    MemoryBlindLeft,
    GlobalLatestArrival,
    RelevantLatestArrival,
    RelevantLatestSourceTime,
    AuthorityWeighted,
    CautiousAuthority,
    PromptPremiseFollower,
    AlwaysClarify,
}

impl Policy {
    const ALL: [Self; 8] = [
        Self::MemoryBlindLeft,
        Self::GlobalLatestArrival,
        Self::RelevantLatestArrival,
        Self::RelevantLatestSourceTime,
        Self::AuthorityWeighted,
        Self::CautiousAuthority,
        Self::PromptPremiseFollower,
        Self::AlwaysClarify,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::MemoryBlindLeft => "memory_blind_left",
            Self::GlobalLatestArrival => "global_latest_arrival",
            Self::RelevantLatestArrival => "relevant_latest_arrival",
            Self::RelevantLatestSourceTime => "relevant_latest_source_time",
            Self::AuthorityWeighted => "authority_weighted",
            Self::CautiousAuthority => "cautious_authority",
            Self::PromptPremiseFollower => "prompt_premise_follower",
            Self::AlwaysClarify => "always_clarify",
        }
    }
}

#[derive(Clone, Debug)]
struct PairResult {
    fixture: &'static str,
    policy: Policy,
    left_target: Option<Origin>,
    right_target: Option<Origin>,
    left_action: Action,
    right_action: Action,
    matched_left_utility: f64,
    matched_right_utility: f64,
    crossed_left_to_right: Option<f64>,
    crossed_right_to_left: Option<f64>,
    pair_reliable: bool,
    logical_memory_bytes: usize,
}

fn claim(
    task_id: u8,
    origin: Origin,
    authority: u8,
    source_time: u16,
    arrival_time: u16,
    source_id: u8,
) -> Claim {
    Claim { task_id, origin, authority, source_time, arrival_time, source_id }
}

fn request(prompt_premise: Option<Origin>) -> PolicyRequest {
    PolicyRequest {
        task_id: TASK_ID,
        current_digest: SAME_PRESENT_DIGEST,
        prompt_premise,
    }
}

fn paired_claim_banks(
    name: &'static str,
    build: impl Fn(Origin) -> Vec<Claim>,
    premise_is_opposite: bool,
    targets_are_ambiguous: bool,
) -> FixturePair {
    let left_truth = if targets_are_ambiguous { None } else { Some(Origin::Left) };
    let right_truth = if targets_are_ambiguous { None } else { Some(Origin::Right) };
    let left_bank = MemoryBank::new(build(Origin::Left));
    let right_bank = if targets_are_ambiguous {
        left_bank.clone()
    } else {
        MemoryBank::new(build(Origin::Right))
    };
    FixturePair {
        name,
        left_target: left_truth,
        right_target: right_truth,
        left_request: request(if premise_is_opposite { Some(Origin::Right) } else { None }),
        right_request: request(if premise_is_opposite { Some(Origin::Left) } else { None }),
        left_bank,
        right_bank,
    }
}

fn fixtures() -> Vec<FixturePair> {
    let mut all = Vec::new();

    all.push(paired_claim_banks(
        "clean_relevant",
        |truth| vec![claim(TASK_ID, truth, 90, 10, 10, 1)],
        false,
        false,
    ));

    // The old revision arrives last. Arrival-order recency should fail;
    // source-time revision handling should keep the actual current revision.
    all.push(paired_claim_banks(
        "stale_late_arrival",
        |truth| vec![
            claim(TASK_ID, truth, 90, 10, 10, 1),
            claim(TASK_ID, truth.opposite(), 90, 2, 20, 1),
        ],
        false,
        false,
    ));

    // A later, low-authority source contradicts an established high-authority
    // source. A pure source-time recency policy will pick the weak claim.
    all.push(paired_claim_banks(
        "low_authority_conflict",
        |truth| vec![
            claim(TASK_ID, truth, 90, 10, 10, 1),
            claim(TASK_ID, truth.opposite(), 10, 11, 11, 2),
        ],
        false,
        false,
    ));

    // Same task-relevant record, then an unrelated task's record arriving
    // later. Global recency is intentionally included as a contamination
    // negative control; relevance-conditioned policies should be invariant.
    all.push(paired_claim_banks(
        "irrelevant_injection",
        |truth| vec![
            claim(TASK_ID, truth, 90, 10, 10, 1),
            claim(IRRELEVANT_TASK_ID, truth.opposite(), 90, 99, 99, 9),
        ],
        false,
        false,
    ));

    // Equal-authority independent sources conflict with no additional
    // adjudicating evidence. The evaluator target is underdetermined: clarify.
    all.push(paired_claim_banks(
        "irreducible_conflict",
        |_truth| vec![
            claim(TASK_ID, Origin::Left, 60, 10, 10, 1),
            claim(TASK_ID, Origin::Right, 60, 10, 10, 2),
        ],
        false,
        true,
    ));

    // The request includes a false presupposition. The memories remain the
    // only evidence for the origin; the request must not overwrite them.
    all.push(paired_claim_banks(
        "false_prompt_premise",
        |truth| vec![claim(TASK_ID, truth, 90, 10, 10, 1)],
        true,
        false,
    ));

    all
}

fn latest_claim<'a>(
    bank: &'a MemoryBank,
    task_id: Option<u8>,
    by_arrival: bool,
) -> Option<&'a Claim> {
    bank.claims
        .iter()
        .filter(|c| task_id.is_none_or(|id| c.task_id == id))
        .max_by_key(|c| {
            if by_arrival {
                (c.arrival_time, c.source_time, c.source_id)
            } else {
                (c.source_time, c.arrival_time, c.source_id)
            }
        })
}

fn latest_source_time_decision(bank: &MemoryBank, task_id: u8) -> Action {
    let relevant: Vec<&Claim> = bank.claims.iter().filter(|c| c.task_id == task_id).collect();
    let Some(max_source_time) = relevant.iter().map(|c| c.source_time).max() else {
        return Action::Clarify;
    };
    let latest: Vec<&Claim> = relevant
        .into_iter()
        .filter(|c| c.source_time == max_source_time)
        .collect();
    let has_left = latest.iter().any(|c| c.origin == Origin::Left);
    let has_right = latest.iter().any(|c| c.origin == Origin::Right);
    match (has_left, has_right) {
        (true, false) => Action::Left,
        (false, true) => Action::Right,
        _ => Action::Clarify,
    }
}

fn authority_scores(bank: &MemoryBank, task_id: u8) -> (u32, u32) {
    let mut left_score = 0u32;
    let mut right_score = 0u32;
    for c in bank.claims.iter().filter(|c| c.task_id == task_id) {
        let superseded = bank.claims.iter().any(|newer| {
            newer.task_id == c.task_id
                && newer.source_id == c.source_id
                && newer.source_time > c.source_time
        });
        if superseded {
            continue;
        }
        match c.origin {
            Origin::Left => left_score += u32::from(c.authority),
            Origin::Right => right_score += u32::from(c.authority),
        }
    }
    (left_score, right_score)
}

fn authority_decision(bank: &MemoryBank, task_id: u8, minimum_margin: u32) -> Action {
    let (left_score, right_score) = authority_scores(bank, task_id);
    if left_score == 0 && right_score == 0 {
        return Action::Clarify;
    }
    if left_score > right_score && left_score - right_score >= minimum_margin {
        Action::Left
    } else if right_score > left_score && right_score - left_score >= minimum_margin {
        Action::Right
    } else {
        Action::Clarify
    }
}

fn decide(policy: Policy, req: &PolicyRequest, bank: &MemoryBank) -> Action {
    match policy {
        Policy::MemoryBlindLeft => Action::Left,
        Policy::GlobalLatestArrival => latest_claim(bank, None, true)
            .map(|c| Action::for_origin(c.origin))
            .unwrap_or(Action::Clarify),
        Policy::RelevantLatestArrival => latest_claim(bank, Some(req.task_id), true)
            .map(|c| Action::for_origin(c.origin))
            .unwrap_or(Action::Clarify),
        Policy::RelevantLatestSourceTime => latest_source_time_decision(bank, req.task_id),
        Policy::AuthorityWeighted => authority_decision(bank, req.task_id, 1),
        Policy::CautiousAuthority => authority_decision(bank, req.task_id, 60),
        Policy::PromptPremiseFollower => req.prompt_premise
            .map(Action::for_origin)
            .unwrap_or_else(|| latest_source_time_decision(bank, req.task_id)),
        Policy::AlwaysClarify => Action::Clarify,
    }
}

fn utility(action: Action, target: Option<Origin>) -> f64 {
    match target {
        Some(origin) => match action.origin() {
            Some(chosen) if chosen == origin => 1.0,
            Some(_) => -1.0,
            None => -0.2,
        },
        None => match action {
            Action::Clarify => 0.2,
            Action::Left | Action::Right => -1.0,
        },
    }
}

fn is_warranted(action: Action, target: Option<Origin>) -> bool {
    match target {
        Some(origin) => action.origin() == Some(origin),
        None => action == Action::Clarify,
    }
}

fn evaluate_pair(pair: &FixturePair, policy: Policy) -> PairResult {
    assert_eq!(
        pair.left_request.current_digest,
        pair.right_request.current_digest,
        "paired histories must reconverge to exactly the same present"
    );
    let left_action = decide(policy, &pair.left_request, &pair.left_bank);
    let right_action = decide(policy, &pair.right_request, &pair.right_bank);
    let matched_left_utility = utility(left_action, pair.left_target);
    let matched_right_utility = utility(right_action, pair.right_target);
    let both_worlds_resolved = pair.left_target.is_some() && pair.right_target.is_some();
    let crossed_left_to_right = if both_worlds_resolved {
        Some(utility(left_action, pair.right_target))
    } else {
        None
    };
    let crossed_right_to_left = if both_worlds_resolved {
        Some(utility(right_action, pair.left_target))
    } else {
        None
    };
    let pair_reliable = is_warranted(left_action, pair.left_target)
        && is_warranted(right_action, pair.right_target);
    let logical_memory_bytes =
        (pair.left_bank.logical_bytes() + pair.right_bank.logical_bytes()) / 2;

    PairResult {
        fixture: pair.name,
        policy,
        left_target: pair.left_target,
        right_target: pair.right_target,
        left_action,
        right_action,
        matched_left_utility,
        matched_right_utility,
        crossed_left_to_right,
        crossed_right_to_left,
        pair_reliable,
        logical_memory_bytes,
    }
}

fn fraction(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 { 0.0 } else { numerator as f64 / denominator as f64 }
}

fn action_on(results: &[PairResult], policy: Policy, fixture: &str, left_direction: bool) -> Option<Action> {
    let result = results.iter().find(|r| r.policy == policy && r.fixture == fixture)?;
    Some(if left_direction { result.left_action } else { result.right_action })
}

fn format_report() -> String {
    let cases = fixtures();
    let mut output = String::new();
    output.push_str("META\tformat\tVIS-MEM-012-v0.1\n");
    output.push_str("META\tclaim_logical_bytes\t16 (fixed logical accounting; not measured RAM)\n");
    output.push_str("META\tutility\tcorrect=1.0 wrong=-1.0 clarify_resolvable=-0.2 clarify_ambiguous=0.2\n");
    output.push_str("META\tevaluator_labels\tkept outside policy request\n");
    output.push_str("PAIR_HEADER\tfixture\tpolicy\tdigest_left\tdigest_right\ttruth_left\ttruth_right\taction_left\taction_right\taction_sensitivity\tpair_reliable\tmatched_utility_left\tmatched_utility_right\tcrossed_utility_left_to_right\tcrossed_utility_right_to_left\tlogical_memory_bytes\n");

    let mut results = Vec::new();
    for case in &cases {
        for policy in Policy::ALL {
            let r = evaluate_pair(case, policy);
            let target_left = r.left_target.map(Origin::label).unwrap_or("underdetermined");
            let target_right = r.right_target.map(Origin::label).unwrap_or("underdetermined");
            let cross_left = r.crossed_left_to_right.map(|v| format!("{v:.3}")).unwrap_or_else(|| "NA".to_owned());
            let cross_right = r.crossed_right_to_left.map(|v| format!("{v:.3}")).unwrap_or_else(|| "NA".to_owned());
            output.push_str(&format!(
                "PAIR\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.3}\t{:.3}\t{}\t{}\t{}\n",
                r.fixture,
                r.policy.name(),
                case.left_request.current_digest,
                case.right_request.current_digest,
                target_left,
                target_right,
                r.left_action.label(),
                r.right_action.label(),
                u8::from(r.left_action != r.right_action),
                u8::from(r.pair_reliable),
                r.matched_left_utility,
                r.matched_right_utility,
                cross_left,
                cross_right,
                r.logical_memory_bytes
            ));
            results.push(r);
        }
    }

    output.push_str("SUMMARY_HEADER\tpolicy\tpaired_cases\taction_sensitivity\twarranted_choice_rate\twrong_action_rate\tunnecessary_clarification_rate\tpair_reliability\tmean_matched_utility\tmean_crossed_utility\tirrelevant_history_invariance\tstale_resistance\tfalse_premise_resistance\tambiguous_clarification\tmean_logical_memory_bytes\n");
    for policy in Policy::ALL {
        let rows: Vec<&PairResult> = results.iter().filter(|r| r.policy == policy).collect();
        let resolved: Vec<&PairResult> = rows.iter().copied()
            .filter(|r| r.left_target.is_some() && r.right_target.is_some()).collect();
        let decisions_count = resolved.len() * 2;
        let sensitive_pairs = resolved.iter().filter(|r| r.left_action != r.right_action).count();
        let warranted_count = resolved.iter().map(|r| {
            usize::from(is_warranted(r.left_action, r.left_target))
                + usize::from(is_warranted(r.right_action, r.right_target))
        }).sum::<usize>();
        let clarify_resolved = resolved.iter().map(|r| {
            usize::from(r.left_action == Action::Clarify)
                + usize::from(r.right_action == Action::Clarify)
        }).sum::<usize>();
        let wrong_count = rows.iter().map(|r| {
            usize::from(matches!(r.left_target, Some(t) if r.left_action.origin() != Some(t)))
                + usize::from(matches!(r.right_target, Some(t) if r.right_action.origin() != Some(t)))
                + usize::from(r.left_target.is_none() && r.left_action != Action::Clarify)
                + usize::from(r.right_target.is_none() && r.right_action != Action::Clarify)
        }).sum::<usize>();
        let pair_reliable = rows.iter().filter(|r| r.pair_reliable).count();
        let matched_total: f64 = rows.iter().map(|r| r.matched_left_utility + r.matched_right_utility).sum();
        let crossed_values: Vec<f64> = rows.iter()
            .flat_map(|r| [r.crossed_left_to_right, r.crossed_right_to_left])
            .flatten()
            .collect();
        let mean_crossed = crossed_values.iter().sum::<f64>() / crossed_values.len().max(1) as f64;

        let irrelevant_invariance = {
            let mut same = 0;
            let mut total = 0;
            for left_direction in [true, false] {
                if let (Some(a), Some(b)) = (
                    action_on(&results, policy, "clean_relevant", left_direction),
                    action_on(&results, policy, "irrelevant_injection", left_direction),
                ) {
                    same += usize::from(a == b);
                    total += 1;
                }
            }
            fraction(same, total)
        };
        let stale_resistance = results.iter()
            .find(|r| r.policy == policy && r.fixture == "stale_late_arrival")
            .map(|r| {
                let correct = usize::from(r.left_target.and_then(|t| r.left_action.origin().map(|a| a == t)).unwrap_or(false))
                    + usize::from(r.right_target.and_then(|t| r.right_action.origin().map(|a| a == t)).unwrap_or(false));
                fraction(correct, 2)
            })
            .unwrap_or(0.0);
        let false_premise_resistance = {
            let mut same = 0;
            let mut total = 0;
            for left_direction in [true, false] {
                if let (Some(a), Some(b)) = (
                    action_on(&results, policy, "clean_relevant", left_direction),
                    action_on(&results, policy, "false_prompt_premise", left_direction),
                ) {
                    same += usize::from(a == b);
                    total += 1;
                }
            }
            fraction(same, total)
        };
        let ambiguous_clarification = results.iter()
            .find(|r| r.policy == policy && r.fixture == "irreducible_conflict")
            .map(|r| u8::from(r.left_action == Action::Clarify && r.right_action == Action::Clarify))
            .unwrap_or(0);
        let mean_memory = rows.iter().map(|r| r.logical_memory_bytes as f64).sum::<f64>() / rows.len().max(1) as f64;

        output.push_str(&format!(
            "SUMMARY\t{}\t{}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{:.3}\t{}\t{:.3}\n",
            policy.name(),
            rows.len(),
            fraction(sensitive_pairs, resolved.len()),
            fraction(warranted_count, decisions_count),
            fraction(wrong_count, rows.len() * 2),
            fraction(clarify_resolved, decisions_count),
            fraction(pair_reliable, rows.len()),
            matched_total / (rows.len() * 2).max(1) as f64,
            mean_crossed,
            irrelevant_invariance,
            stale_resistance,
            false_premise_resistance,
            ambiguous_clarification,
            mean_memory
        ));
    }

    output
}

fn main() {
    print!("{}", format_report());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn result_for(policy: Policy, fixture: &str) -> PairResult {
        let case = fixtures().into_iter().find(|c| c.name == fixture).unwrap();
        evaluate_pair(&case, policy)
    }

    #[test]
    fn all_pairs_have_exactly_the_same_current_observation_digest() {
        for case in fixtures() {
            assert_eq!(case.left_request.current_digest, case.right_request.current_digest);
            assert_eq!(case.left_request.current_digest, SAME_PRESENT_DIGEST);
        }
    }

    #[test]
    fn reliable_memory_warrants_opposite_actions_at_the_same_present() {
        let r = result_for(Policy::AuthorityWeighted, "clean_relevant");
        assert_eq!(r.left_action, Action::Left);
        assert_eq!(r.right_action, Action::Right);
        assert!(r.pair_reliable);
        assert_eq!(r.crossed_left_to_right, Some(-1.0));
        assert_eq!(r.crossed_right_to_left, Some(-1.0));
    }

    #[test]
    fn latest_arrival_is_vulnerable_to_late_stale_revision() {
        let r = result_for(Policy::RelevantLatestArrival, "stale_late_arrival");
        assert_eq!(r.left_action, Action::Right);
        assert_eq!(r.right_action, Action::Left);
        let safe = result_for(Policy::RelevantLatestSourceTime, "stale_late_arrival");
        assert_eq!(safe.left_action, Action::Left);
        assert_eq!(safe.right_action, Action::Right);
    }

    #[test]
    fn low_authority_newer_claim_does_not_overrule_trusted_evidence() {
        let newest = result_for(Policy::RelevantLatestSourceTime, "low_authority_conflict");
        assert_eq!(newest.left_action, Action::Right);
        assert_eq!(newest.right_action, Action::Left);
        let weighted = result_for(Policy::AuthorityWeighted, "low_authority_conflict");
        assert_eq!(weighted.left_action, Action::Left);
        assert_eq!(weighted.right_action, Action::Right);
    }

    #[test]
    fn irrelevant_late_history_changes_global_but_not_scoped_recall() {
        let global = result_for(Policy::GlobalLatestArrival, "irrelevant_injection");
        assert_eq!(global.left_action, Action::Right);
        assert_eq!(global.right_action, Action::Left);
        let scoped = result_for(Policy::AuthorityWeighted, "irrelevant_injection");
        assert_eq!(scoped.left_action, Action::Left);
        assert_eq!(scoped.right_action, Action::Right);
    }

    #[test]
    fn irreducible_equal_authority_conflict_calls_for_clarification() {
        let weighted = result_for(Policy::AuthorityWeighted, "irreducible_conflict");
        assert_eq!(weighted.left_action, Action::Clarify);
        assert_eq!(weighted.right_action, Action::Clarify);
        assert_eq!(utility(Action::Clarify, None), 0.2);
        assert_eq!(utility(Action::Left, None), -1.0);
    }

    #[test]
    fn false_prompt_premise_does_not_override_reliable_history() {
        let follower = result_for(Policy::PromptPremiseFollower, "false_prompt_premise");
        assert_eq!(follower.left_action, Action::Right);
        assert_eq!(follower.right_action, Action::Left);
        let robust = result_for(Policy::AuthorityWeighted, "false_prompt_premise");
        assert_eq!(robust.left_action, Action::Left);
        assert_eq!(robust.right_action, Action::Right);
    }

    #[test]
    fn always_clarify_is_safe_but_pays_resolvable_cost() {
        let ambiguous = result_for(Policy::AlwaysClarify, "irreducible_conflict");
        assert!(ambiguous.pair_reliable);
        assert_eq!(ambiguous.matched_left_utility, 0.2);
        let clean = result_for(Policy::AlwaysClarify, "clean_relevant");
        assert_eq!(clean.matched_left_utility, -0.2);
    }

    #[test]
    fn finite_suite_shape_is_fixed() {
        assert_eq!(fixtures().len(), 6);
        assert_eq!(Policy::ALL.len(), 8);
        assert_eq!(fixtures().len() * Policy::ALL.len(), 48);
    }

    #[test]
    fn authority_resolver_copes_with_stale_revision_and_ambiguity() {
        assert_eq!(result_for(Policy::AuthorityWeighted, "stale_late_arrival").left_action, Action::Left);
        assert_eq!(result_for(Policy::AuthorityWeighted, "irreducible_conflict").left_action, Action::Clarify);
    }

    #[test]
    fn deterministic_report_contains_expected_pair_and_summary_rows() {
        let a = format_report();
        let b = format_report();
        assert_eq!(a, b);
        assert_eq!(a.lines().filter(|line| line.starts_with("PAIR\t")).count(), 48);
        assert_eq!(a.lines().filter(|line| line.starts_with("SUMMARY\t")).count(), 8);
    }

    #[test]
    fn logical_memory_bytes_are_not_zero_for_stored_claims() {
        let r = result_for(Policy::AuthorityWeighted, "clean_relevant");
        assert_eq!(r.logical_memory_bytes, CLAIM_LOGICAL_BYTES);
    }

    #[test]
    fn crossed_world_scoring_is_not_reported_for_irreducible_conflict() {
        let r = result_for(Policy::AuthorityWeighted, "irreducible_conflict");
        assert_eq!(r.crossed_left_to_right, None);
        assert_eq!(r.crossed_right_to_left, None);
    }
}
