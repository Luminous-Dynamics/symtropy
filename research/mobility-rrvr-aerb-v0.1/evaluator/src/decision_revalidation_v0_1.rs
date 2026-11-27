// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! AERB decision-to-effect revalidation reference reducer.
//!
//! This module preserves an immutable historical decision cut while deciding
//! whether its evidence support is still applicable at a later pre-effect cut.
//! It consumes already-admitted change-coverage and dependency-impact receipts;
//! it does not implement an event store, dependency graph, signature verifier,
//! action-ticket service, dispatch path, or evidence-invalidation engine.
//!
//! A CurrentForCommit result is a prerequisite observation only. It is not an
//! actuation permit. The independent mobility action authority remains the only
//! owner allowed to admit a physical effect.

use std::collections::BTreeSet;

const SHA256_HEX_LENGTH: usize = 64;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionCut {
    /// Monotonic event/effective-time frontier in the declared clock profile.
    pub event_frontier_sequence: u64,
    /// Monotonic knowledge/ingest frontier.
    pub ingest_frontier_sequence: u64,
    /// Qualified mapping of event time into the declared sequence basis.
    pub clock_profile_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionDependency {
    pub dependency_id: String,
    pub admitted_revision_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionSupportReceipt {
    pub decision_id: String,
    pub decision_support_receipt_sha256: String,
    pub decision_snapshot_sha256: String,
    pub possible_world_support_sha256: String,
    pub counterfactual_policy_sha256: String,
    pub action_ticket_sha256: String,
    pub dependency_closure_sha256: String,
    pub change_stream_profile_sha256: String,
    pub dependency_closure_complete: bool,
    pub valid_through_event_sequence: u64,
    pub decision_cut: DecisionCut,
    pub dependencies: Vec<DecisionDependency>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeCoverageReceipt {
    /// The covered interval is (from, through], in ingest sequence space.
    pub from_ingest_frontier_exclusive: u64,
    pub through_ingest_frontier_inclusive: u64,
    pub coverage_sha256: String,
    pub profile_sha256: String,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeKind {
    Updated,
    Revoked,
    Superseded,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionChange {
    pub event_id: String,
    pub event_sha256: String,
    pub impact_receipt_sha256: String,
    pub effective_event_sequence: u64,
    pub ingested_sequence: u64,
    pub kind: ChangeKind,
    /// Complete, upstream-adjudicated projection onto decision dependencies.
    /// An empty list means proven unrelated only when impact_assessment_complete
    /// is true and the dependency closure is independently complete.
    pub affected_dependency_ids: Vec<String>,
    pub resulting_revision_sha256: String,
    pub impact_assessment_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecomputeReason {
    ActionTicketChanged,
    ClockProfileChanged,
    ValidityHorizonExceeded,
    DependencyChanged {
        event_id: String,
        dependency_id: String,
        kind: ChangeKind,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum IndeterminateReason {
    DependencyClosureIncomplete,
    ChangeCoverageIncomplete,
    ChangeCoverageRangeMismatch,
    DependencyImpactIncomplete { event_id: String },
    UnknownChangeKind { event_id: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InvalidReason {
    EmptyDecisionId,
    DigestInvalid(&'static str),
    EmptyDependencySet,
    EmptyDependencyId,
    DuplicateDependencyId(String),
    DuplicateEventId(String),
    EmptyEventId,
    EmptyAffectedDependencyId,
    DuplicateAffectedDependencyId(String),
    ChangeOutsideCoveredIngestRange { event_id: String },
    CutRegressed,
    CoverageRangeReversed,
    ValidityHorizonPrecedesDecisionCut,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecisionRevalidation {
    /// The old decision remains applicable at this exact cut. This is not a permit.
    CurrentForCommit {
        decision_id: String,
        decision_support_receipt_sha256: String,
        action_ticket_sha256: String,
        coverage_sha256: String,
        current_cut: DecisionCut,
    },
    /// A known relevant change or applicability boundary requires recomputation.
    RecomputeRequired {
        reasons: Vec<RecomputeReason>,
    },
    /// The reducer cannot establish completeness/currentness. Effect stays blocked.
    Indeterminate {
        reasons: Vec<IndeterminateReason>,
    },
    /// The submitted contract is malformed or internally inconsistent.
    Invalid {
        reason: InvalidReason,
    },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_cut(cut: &DecisionCut) -> Result<(), InvalidReason> {
    if !valid_sha256(&cut.clock_profile_sha256) {
        return Err(InvalidReason::DigestInvalid("clock_profile_sha256"));
    }
    Ok(())
}

fn validate_receipt(receipt: &DecisionSupportReceipt) -> Result<(), InvalidReason> {
    if !nonempty(&receipt.decision_id) {
        return Err(InvalidReason::EmptyDecisionId);
    }
    for (name, digest) in [
        (
            "decision_support_receipt_sha256",
            &receipt.decision_support_receipt_sha256,
        ),
        ("decision_snapshot_sha256", &receipt.decision_snapshot_sha256),
        (
            "possible_world_support_sha256",
            &receipt.possible_world_support_sha256,
        ),
        (
            "counterfactual_policy_sha256",
            &receipt.counterfactual_policy_sha256,
        ),
        ("action_ticket_sha256", &receipt.action_ticket_sha256),
        (
            "dependency_closure_sha256",
            &receipt.dependency_closure_sha256,
        ),
        (
            "change_stream_profile_sha256",
            &receipt.change_stream_profile_sha256,
        ),
    ] {
        if !valid_sha256(digest) {
            return Err(InvalidReason::DigestInvalid(name));
        }
    }
    validate_cut(&receipt.decision_cut)?;
    if receipt.valid_through_event_sequence < receipt.decision_cut.event_frontier_sequence {
        return Err(InvalidReason::ValidityHorizonPrecedesDecisionCut);
    }
    if receipt.dependencies.is_empty() {
        return Err(InvalidReason::EmptyDependencySet);
    }
    let mut dependency_ids = BTreeSet::new();
    for dependency in &receipt.dependencies {
        if !nonempty(&dependency.dependency_id) {
            return Err(InvalidReason::EmptyDependencyId);
        }
        if !valid_sha256(&dependency.admitted_revision_sha256) {
            return Err(InvalidReason::DigestInvalid(
                "dependency.admitted_revision_sha256",
            ));
        }
        if !dependency_ids.insert(dependency.dependency_id.as_str()) {
            return Err(InvalidReason::DuplicateDependencyId(
                dependency.dependency_id.clone(),
            ));
        }
    }
    Ok(())
}

/// Revalidate one immutable decision-support receipt at a later pre-effect cut.
///
/// The caller must first validate the signatures/provenance and semantics of the
/// upstream change-coverage and dependency-impact receipts. This reducer checks
/// their declared profiles, exact ingest interval, identities and completeness
/// flags, but does not authenticate their issuers or traverse dependency graphs.
pub fn revalidate_before_effect(
    receipt: &DecisionSupportReceipt,
    coverage: &ChangeCoverageReceipt,
    changes: &[DecisionChange],
    current_cut: &DecisionCut,
    current_action_ticket_sha256: &str,
) -> DecisionRevalidation {
    if let Err(reason) = validate_receipt(receipt) {
        return DecisionRevalidation::Invalid { reason };
    }
    if let Err(reason) = validate_cut(current_cut) {
        return DecisionRevalidation::Invalid { reason };
    }
    if !valid_sha256(current_action_ticket_sha256) {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::DigestInvalid("current_action_ticket_sha256"),
        };
    }
    if !valid_sha256(&coverage.coverage_sha256) {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::DigestInvalid("coverage_sha256"),
        };
    }
    if !valid_sha256(&coverage.profile_sha256) {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::DigestInvalid("coverage.profile_sha256"),
        };
    }
    if coverage.through_ingest_frontier_inclusive
        < coverage.from_ingest_frontier_exclusive
    {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::CoverageRangeReversed,
        };
    }
    if current_cut.event_frontier_sequence < receipt.decision_cut.event_frontier_sequence
        || current_cut.ingest_frontier_sequence < receipt.decision_cut.ingest_frontier_sequence
    {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::CutRegressed,
        };
    }

    if coverage.profile_sha256 != receipt.change_stream_profile_sha256 {
        return DecisionRevalidation::Invalid {
            reason: InvalidReason::DigestInvalid("change_stream_profile_mismatch"),
        };
    }

    let mut indeterminate = Vec::new();
    if !receipt.dependency_closure_complete {
        indeterminate.push(IndeterminateReason::DependencyClosureIncomplete);
    }
    if !coverage.complete {
        indeterminate.push(IndeterminateReason::ChangeCoverageIncomplete);
    }
    if coverage.from_ingest_frontier_exclusive
        != receipt.decision_cut.ingest_frontier_sequence
        || coverage.through_ingest_frontier_inclusive
            != current_cut.ingest_frontier_sequence
    {
        indeterminate.push(IndeterminateReason::ChangeCoverageRangeMismatch);
    }
    if !indeterminate.is_empty() {
        indeterminate.sort();
        indeterminate.dedup();
        return DecisionRevalidation::Indeterminate {
            reasons: indeterminate,
        };
    }

    let decision_dependencies: BTreeSet<&str> = receipt
        .dependencies
        .iter()
        .map(|dependency| dependency.dependency_id.as_str())
        .collect();
    let mut event_ids = BTreeSet::new();
    let mut recompute = Vec::new();

    for change in changes {
        if !nonempty(&change.event_id) {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::EmptyEventId,
            };
        }
        if !event_ids.insert(change.event_id.as_str()) {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::DuplicateEventId(change.event_id.clone()),
            };
        }
        if !valid_sha256(&change.event_sha256) {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::DigestInvalid("change.event_sha256"),
            };
        }
        if !valid_sha256(&change.impact_receipt_sha256) {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::DigestInvalid("change.impact_receipt_sha256"),
            };
        }
        if !valid_sha256(&change.resulting_revision_sha256) {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::DigestInvalid("change.resulting_revision_sha256"),
            };
        }
        if change.ingested_sequence <= coverage.from_ingest_frontier_exclusive
            || change.ingested_sequence > coverage.through_ingest_frontier_inclusive
        {
            return DecisionRevalidation::Invalid {
                reason: InvalidReason::ChangeOutsideCoveredIngestRange {
                    event_id: change.event_id.clone(),
                },
            };
        }

        let mut affected_ids = BTreeSet::new();
        for dependency_id in &change.affected_dependency_ids {
            if !nonempty(dependency_id) {
                return DecisionRevalidation::Invalid {
                    reason: InvalidReason::EmptyAffectedDependencyId,
                };
            }
            if !affected_ids.insert(dependency_id.as_str()) {
                return DecisionRevalidation::Invalid {
                    reason: InvalidReason::DuplicateAffectedDependencyId(
                        dependency_id.clone(),
                    ),
                };
            }
        }

        // Future-effective changes are not active at this exact commit cut.
        // Late arrivals with old effective time remain visible because their
        // ingest sequence lies inside the covered post-decision knowledge range.
        if change.effective_event_sequence > current_cut.event_frontier_sequence {
            continue;
        }
        if !change.impact_assessment_complete {
            indeterminate.push(IndeterminateReason::DependencyImpactIncomplete {
                event_id: change.event_id.clone(),
            });
            continue;
        }
        if change.kind == ChangeKind::Unknown {
            indeterminate.push(IndeterminateReason::UnknownChangeKind {
                event_id: change.event_id.clone(),
            });
            continue;
        }

        for dependency_id in affected_ids {
            if decision_dependencies.contains(dependency_id) {
                recompute.push(RecomputeReason::DependencyChanged {
                    event_id: change.event_id.clone(),
                    dependency_id: dependency_id.to_owned(),
                    kind: change.kind,
                });
            }
        }
    }

    // Unknown impact anywhere in the covered interval prevents a positive
    // CurrentForCommit conclusion, even when another known change is relevant.
    if !indeterminate.is_empty() {
        indeterminate.sort();
        indeterminate.dedup();
        return DecisionRevalidation::Indeterminate {
            reasons: indeterminate,
        };
    }

    if current_action_ticket_sha256 != receipt.action_ticket_sha256 {
        recompute.push(RecomputeReason::ActionTicketChanged);
    }
    if current_cut.clock_profile_sha256 != receipt.decision_cut.clock_profile_sha256 {
        recompute.push(RecomputeReason::ClockProfileChanged);
    }
    if current_cut.event_frontier_sequence > receipt.valid_through_event_sequence {
        recompute.push(RecomputeReason::ValidityHorizonExceeded);
    }

    if !recompute.is_empty() {
        recompute.sort();
        recompute.dedup();
        return DecisionRevalidation::RecomputeRequired { reasons: recompute };
    }

    DecisionRevalidation::CurrentForCommit {
        decision_id: receipt.decision_id.clone(),
        decision_support_receipt_sha256: receipt.decision_support_receipt_sha256.clone(),
        action_ticket_sha256: current_action_ticket_sha256.to_owned(),
        coverage_sha256: coverage.coverage_sha256.clone(),
        current_cut: current_cut.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        character.to_string().repeat(SHA256_HEX_LENGTH)
    }

    fn cut(event_frontier_sequence: u64, ingest_frontier_sequence: u64) -> DecisionCut {
        DecisionCut {
            event_frontier_sequence,
            ingest_frontier_sequence,
            clock_profile_sha256: digest('c'),
        }
    }

    fn receipt() -> DecisionSupportReceipt {
        DecisionSupportReceipt {
            decision_id: "decision-7".to_owned(),
            decision_support_receipt_sha256: digest('0'),
            decision_snapshot_sha256: digest('a'),
            possible_world_support_sha256: digest('b'),
            counterfactual_policy_sha256: digest('d'),
            action_ticket_sha256: digest('e'),
            dependency_closure_sha256: digest('f'),
            change_stream_profile_sha256: digest('1'),
            dependency_closure_complete: true,
            valid_through_event_sequence: 25,
            decision_cut: cut(20, 30),
            dependencies: vec![
                DecisionDependency {
                    dependency_id: "endpoint-profile".to_owned(),
                    admitted_revision_sha256: digest('2'),
                },
                DecisionDependency {
                    dependency_id: "effect-observation".to_owned(),
                    admitted_revision_sha256: digest('3'),
                },
            ],
        }
    }

    fn coverage() -> ChangeCoverageReceipt {
        ChangeCoverageReceipt {
            from_ingest_frontier_exclusive: 30,
            through_ingest_frontier_inclusive: 31,
            coverage_sha256: digest('4'),
            profile_sha256: digest('1'),
            complete: true,
        }
    }

    fn change(
        event_id: &str,
        effective: u64,
        ingested: u64,
        kind: ChangeKind,
        affected: &[&str],
    ) -> DecisionChange {
        DecisionChange {
            event_id: event_id.to_owned(),
            event_sha256: digest('5'),
            impact_receipt_sha256: digest('7'),
            effective_event_sequence: effective,
            ingested_sequence: ingested,
            kind,
            affected_dependency_ids: affected.iter().map(|id| (*id).to_owned()).collect(),
            resulting_revision_sha256: digest('6'),
            impact_assessment_complete: true,
        }
    }

    #[test]
    fn complete_unchanged_dependencies_remain_current_at_commit_cut() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::CurrentForCommit { .. }
        ));
        assert_eq!(receipt.decision_cut, cut(20, 30));
    }

    #[test]
    fn relevant_dependency_update_requires_recomputation() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[change(
                "late-pose-calibration",
                21,
                31,
                ChangeKind::Updated,
                &["effect-observation"],
            )],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::RecomputeRequired { .. }
        ));
    }

    #[test]
    fn revoked_dependency_requires_recomputation() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[change(
                "source-revocation",
                21,
                31,
                ChangeKind::Revoked,
                &["endpoint-profile"],
            )],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::RecomputeRequired { .. }
        ));
    }

    #[test]
    fn complete_unrelated_change_does_not_invalidate_decision() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[change(
                "unrelated-ui-profile-change",
                21,
                31,
                ChangeKind::Updated,
                &["operator-dashboard-theme"],
            )],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::CurrentForCommit { .. }
        ));
    }

    #[test]
    fn incomplete_change_stream_coverage_is_indeterminate() {
        let receipt = receipt();
        let mut coverage = coverage();
        coverage.complete = false;
        let result = revalidate_before_effect(
            &receipt,
            &coverage,
            &[],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::Indeterminate { .. }
        ));
    }

    #[test]
    fn incomplete_dependency_closure_is_indeterminate() {
        let mut receipt = receipt();
        receipt.dependency_closure_complete = false;
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::Indeterminate { .. }
        ));
    }

    #[test]
    fn late_arrival_with_predecision_effective_time_forces_recomputation() {
        let receipt = receipt();
        let late = change(
            "late-arriving-effect",
            19,
            31,
            ChangeKind::Updated,
            &["effect-observation"],
        );
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[late],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::RecomputeRequired { .. }
        ));
    }

    #[test]
    fn changed_clock_profile_requires_recomputation() {
        let receipt = receipt();
        let mut current = cut(21, 31);
        current.clock_profile_sha256 = digest('9');
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[],
            &current,
            &receipt.action_ticket_sha256,
        );
        assert_eq!(
            result,
            DecisionRevalidation::RecomputeRequired {
                reasons: vec![RecomputeReason::ClockProfileChanged],
            }
        );
    }

    #[test]
    fn changed_action_ticket_requires_recomputation() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[],
            &cut(21, 31),
            &digest('8'),
        );
        assert_eq!(
            result,
            DecisionRevalidation::RecomputeRequired {
                reasons: vec![RecomputeReason::ActionTicketChanged],
            }
        );
    }

    #[test]
    fn expired_decision_horizon_requires_recomputation() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[],
            &cut(26, 31),
            &receipt.action_ticket_sha256,
        );
        assert_eq!(
            result,
            DecisionRevalidation::RecomputeRequired {
                reasons: vec![RecomputeReason::ValidityHorizonExceeded],
            }
        );
    }

    #[test]
    fn incomplete_dependency_impact_prevents_current_conclusion() {
        let receipt = receipt();
        let mut event = change("impact-gap", 21, 31, ChangeKind::Updated, &[]);
        event.impact_assessment_complete = false;
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[event],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::Indeterminate { .. }
        ));
    }

    #[test]
    fn unknown_change_kind_prevents_current_conclusion() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[change("unknown-change", 21, 31, ChangeKind::Unknown, &[])],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::Indeterminate { .. }
        ));
    }

    #[test]
    fn change_coverage_must_exactly_span_the_new_knowledge_frontier() {
        let receipt = receipt();
        let mut coverage = coverage();
        coverage.from_ingest_frontier_exclusive = 29;
        let result = revalidate_before_effect(
            &receipt,
            &coverage,
            &[],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::Indeterminate { .. }
        ));
    }

    #[test]
    fn aba_revision_return_does_not_restore_historical_decision() {
        let receipt = receipt();
        let mut first = change(
            "profile-a-to-b",
            21,
            31,
            ChangeKind::Updated,
            &["endpoint-profile"],
        );
        first.resulting_revision_sha256 = digest('7');
        let mut second = change(
            "profile-b-to-a",
            22,
            32,
            ChangeKind::Updated,
            &["endpoint-profile"],
        );
        second.resulting_revision_sha256 = receipt.dependencies[0].admitted_revision_sha256.clone();
        let mut coverage = coverage();
        coverage.through_ingest_frontier_inclusive = 32;
        let result = revalidate_before_effect(
            &receipt,
            &coverage,
            &[first, second],
            &cut(22, 32),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::RecomputeRequired { .. }
        ));
        assert_eq!(receipt.decision_cut, cut(20, 30));
    }

    #[test]
    fn duplicate_event_identity_is_invalid() {
        let receipt = receipt();
        let event = change("duplicate", 21, 31, ChangeKind::Updated, &[]);
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[event.clone(), event],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert_eq!(
            result,
            DecisionRevalidation::Invalid {
                reason: InvalidReason::DuplicateEventId("duplicate".to_owned()),
            }
        );
    }

    #[test]
    fn future_effective_change_is_not_applied_at_an_earlier_commit_cut() {
        let receipt = receipt();
        let result = revalidate_before_effect(
            &receipt,
            &coverage(),
            &[change(
                "future-revocation",
                24,
                31,
                ChangeKind::Revoked,
                &["endpoint-profile"],
            )],
            &cut(21, 31),
            &receipt.action_ticket_sha256,
        );
        assert!(matches!(
            result,
            DecisionRevalidation::CurrentForCommit { .. }
        ));
    }
}
