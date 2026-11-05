// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Exact small-council voting power and authority-topology audit.
//!
//! Nominal voting weight, effective coalition power, role concentration and
//! procedural independence are reported separately. No scalar democracy or
//! decentralization score is produced.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXACT_VOTERS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedCouncil {
    pub actor_ids: Vec<String>,
    pub weights: Vec<u64>,
    pub quota: u64,
}

impl WeightedCouncil {
    pub fn validate(&self) -> Result<u64, AuthorityError> {
        let count = self.actor_ids.len();
        if count < 2 {
            return Err(AuthorityError::TooFewVoters(count));
        }
        if count > MAX_EXACT_VOTERS {
            return Err(AuthorityError::TooManyVoters {
                actual: count,
                maximum: MAX_EXACT_VOTERS,
            });
        }
        if self.weights.len() != count {
            return Err(AuthorityError::WeightCountMismatch {
                expected: count,
                actual: self.weights.len(),
            });
        }
        let mut ids = BTreeSet::new();
        for id in &self.actor_ids {
            if id.trim().is_empty() {
                return Err(AuthorityError::EmptyActorId);
            }
            if !ids.insert(id) {
                return Err(AuthorityError::DuplicateActorId(id.clone()));
            }
        }
        let total = self.weights.iter().try_fold(0_u64, |sum, weight| {
            sum.checked_add(*weight)
                .ok_or(AuthorityError::WeightOverflow)
        })?;
        if total == 0 {
            return Err(AuthorityError::ZeroTotalWeight);
        }
        if self.quota == 0 || self.quota > total {
            return Err(AuthorityError::InvalidQuota {
                quota: self.quota,
                total,
            });
        }
        Ok(total)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActorPower {
    pub actor_id: String,
    pub nominal_weight: u64,
    pub nominal_weight_share: f64,
    pub raw_banzhaf_swings: u64,
    pub normalized_banzhaf: f64,
    pub shapley_shubik: f64,
    /// Normalized Banzhaf power divided by nominal weight share.
    pub banzhaf_power_to_weight_ratio: Option<f64>,
    pub veto_player: bool,
    pub dictator: bool,
    /// True when all other voters together still meet the quota.
    pub council_remains_winnable_after_removal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MinimalWinningCoalition {
    pub actor_ids: Vec<String>,
    pub total_weight: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VotingPowerReport {
    pub voter_count: usize,
    pub total_weight: u64,
    pub quota: u64,
    pub enumerated_coalition_count: u64,
    pub winning_coalition_count: u64,
    pub minimal_winning_coalition_count: usize,
    pub actors: Vec<ActorPower>,
    pub minimal_winning_coalitions: Vec<MinimalWinningCoalition>,
    pub nominal_weight_hhi: f64,
    pub normalized_banzhaf_hhi: f64,
    pub nominal_weight_gini: f64,
    pub normalized_banzhaf_gini: f64,
}

pub fn analyze_voting_power(
    council: &WeightedCouncil,
) -> Result<VotingPowerReport, AuthorityError> {
    let total_weight = council.validate()?;
    let voter_count = council.actor_ids.len();
    let coalition_count = 1_u64 << voter_count;
    let mut coalition_weights = vec![0_u64; coalition_count as usize];

    // Dynamic subset-sum table: one addition per coalition, then O(1) access
    // while enumerating swing voters and minimal winning coalitions.
    for mask in 1..coalition_count {
        let least_significant_bit = mask & mask.wrapping_neg();
        let actor_index = least_significant_bit.trailing_zeros() as usize;
        let previous_mask = mask ^ least_significant_bit;
        coalition_weights[mask as usize] =
            coalition_weights[previous_mask as usize] + council.weights[actor_index];
    }

    let mut raw_swings = vec![0_u64; voter_count];
    let mut shapley_shubik = vec![0.0_f64; voter_count];
    let mut veto_player = vec![true; voter_count];
    let mut winning_coalition_count = 0_u64;
    let mut minimal_winning_coalitions = Vec::new();
    let mut factorial = vec![1.0_f64; voter_count + 1];
    for index in 1..=voter_count {
        factorial[index] = factorial[index - 1] * index as f64;
    }

    for mask in 0..coalition_count {
        let weight = coalition_weights[mask as usize];
        if weight >= council.quota {
            winning_coalition_count += 1;
            let mut is_minimal = true;
            for (actor, is_veto_player) in veto_player.iter_mut().enumerate() {
                let bit = 1_u64 << actor;
                if mask & bit == 0 {
                    *is_veto_player = false;
                } else if weight - council.weights[actor] >= council.quota {
                    is_minimal = false;
                }
            }
            if is_minimal {
                let actor_ids = (0..voter_count)
                    .filter(|actor| mask & (1_u64 << actor) != 0)
                    .map(|actor| council.actor_ids[actor].clone())
                    .collect();
                minimal_winning_coalitions.push(MinimalWinningCoalition {
                    actor_ids,
                    total_weight: weight,
                });
            }
        } else {
            let coalition_size = mask.count_ones() as usize;
            for actor in 0..voter_count {
                let bit = 1_u64 << actor;
                if mask & bit != 0 {
                    continue;
                }
                if weight + council.weights[actor] >= council.quota {
                    raw_swings[actor] += 1;
                    shapley_shubik[actor] += factorial[coalition_size]
                        * factorial[voter_count - coalition_size - 1]
                        / factorial[voter_count];
                }
            }
        }
    }

    let total_swings = raw_swings.iter().sum::<u64>();
    if total_swings == 0 {
        return Err(AuthorityError::NoCriticalVoters);
    }

    let normalized_banzhaf = raw_swings
        .iter()
        .map(|swings| *swings as f64 / total_swings as f64)
        .collect::<Vec<_>>();
    let nominal_shares = council
        .weights
        .iter()
        .map(|weight| *weight as f64 / total_weight as f64)
        .collect::<Vec<_>>();

    let actors = (0..voter_count)
        .map(|index| {
            let nominal_weight_share = nominal_shares[index];
            let power = normalized_banzhaf[index];
            ActorPower {
                actor_id: council.actor_ids[index].clone(),
                nominal_weight: council.weights[index],
                nominal_weight_share,
                raw_banzhaf_swings: raw_swings[index],
                normalized_banzhaf: power,
                shapley_shubik: shapley_shubik[index],
                banzhaf_power_to_weight_ratio: if nominal_weight_share == 0.0 {
                    None
                } else {
                    Some(power / nominal_weight_share)
                },
                veto_player: veto_player[index],
                dictator: council.weights[index] >= council.quota
                    && total_weight - council.weights[index] < council.quota,
                council_remains_winnable_after_removal: total_weight - council.weights[index]
                    >= council.quota,
            }
        })
        .collect::<Vec<_>>();

    Ok(VotingPowerReport {
        voter_count,
        total_weight,
        quota: council.quota,
        enumerated_coalition_count: coalition_count,
        winning_coalition_count,
        minimal_winning_coalition_count: minimal_winning_coalitions.len(),
        actors,
        minimal_winning_coalitions,
        nominal_weight_hhi: hhi(&nominal_shares),
        normalized_banzhaf_hhi: hhi(&normalized_banzhaf),
        nominal_weight_gini: gini(&nominal_shares),
        normalized_banzhaf_gini: gini(&normalized_banzhaf),
    })
}

fn hhi(shares: &[f64]) -> f64 {
    shares.iter().map(|share| share * share).sum()
}

fn gini(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let total = sorted.iter().sum::<f64>();
    if total <= f64::EPSILON {
        return 0.0;
    }
    let weighted = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (index + 1) as f64 * value)
        .sum::<f64>();
    (2.0 * weighted / (sorted.len() as f64 * total)
        - (sorted.len() as f64 + 1.0) / sorted.len() as f64)
        .clamp(0.0, 1.0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuthorityRole {
    Proposer,
    AgendaController,
    Summarizer,
    PublicRecordEditor,
    EvidenceIssuer,
    EvidenceVerifier,
    Approver,
    Disburser,
    Auditor,
    OriginalAdjudicator,
    AppealAdjudicator,
    RuleAmender,
    EmergencyOverride,
}

impl AuthorityRole {
    pub const ALL: [Self; 13] = [
        Self::Proposer,
        Self::AgendaController,
        Self::Summarizer,
        Self::PublicRecordEditor,
        Self::EvidenceIssuer,
        Self::EvidenceVerifier,
        Self::Approver,
        Self::Disburser,
        Self::Auditor,
        Self::OriginalAdjudicator,
        Self::AppealAdjudicator,
        Self::RuleAmender,
        Self::EmergencyOverride,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Proposer => "proposer",
            Self::AgendaController => "agenda_controller",
            Self::Summarizer => "summarizer",
            Self::PublicRecordEditor => "public_record_editor",
            Self::EvidenceIssuer => "evidence_issuer",
            Self::EvidenceVerifier => "evidence_verifier",
            Self::Approver => "approver",
            Self::Disburser => "disburser",
            Self::Auditor => "auditor",
            Self::OriginalAdjudicator => "original_adjudicator",
            Self::AppealAdjudicator => "appeal_adjudicator",
            Self::RuleAmender => "rule_amender",
            Self::EmergencyOverride => "emergency_override",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityAssignment {
    pub actor_id: String,
    pub roles: Vec<AuthorityRole>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSeverity {
    Forbidden,
    Advisory,
}

impl RuleSeverity {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Forbidden => "forbidden",
            Self::Advisory => "advisory",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleCombinationRule {
    pub first: AuthorityRole,
    pub second: AuthorityRole,
    pub severity: RuleSeverity,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityPolicy {
    /// Roles whose absence makes the declared governance design incomplete.
    pub required_roles: Vec<AuthorityRole>,
    /// Pairwise conflicts are policy choices, not universal mathematical laws.
    pub role_combination_rules: Vec<RoleCombinationRule>,
}

impl AuthorityPolicy {
    pub fn public_governance_reference() -> Self {
        Self {
            required_roles: vec![
                AuthorityRole::AgendaController,
                AuthorityRole::Summarizer,
                AuthorityRole::PublicRecordEditor,
                AuthorityRole::EvidenceVerifier,
                AuthorityRole::Approver,
                AuthorityRole::Disburser,
                AuthorityRole::Auditor,
                AuthorityRole::OriginalAdjudicator,
                AuthorityRole::AppealAdjudicator,
            ],
            role_combination_rules: vec![
                RoleCombinationRule {
                    first: AuthorityRole::Approver,
                    second: AuthorityRole::Disburser,
                    severity: RuleSeverity::Forbidden,
                    rationale: "approval and disbursement require independent actors".into(),
                },
                RoleCombinationRule {
                    first: AuthorityRole::EvidenceIssuer,
                    second: AuthorityRole::EvidenceVerifier,
                    severity: RuleSeverity::Forbidden,
                    rationale: "an evidence issuer must not be its sole verifier".into(),
                },
                RoleCombinationRule {
                    first: AuthorityRole::OriginalAdjudicator,
                    second: AuthorityRole::AppealAdjudicator,
                    severity: RuleSeverity::Forbidden,
                    rationale: "appeal adjudication must be independent of the original decision"
                        .into(),
                },
                RoleCombinationRule {
                    first: AuthorityRole::AgendaController,
                    second: AuthorityRole::Summarizer,
                    severity: RuleSeverity::Advisory,
                    rationale: "agenda and summary control can jointly shape the public record"
                        .into(),
                },
                RoleCombinationRule {
                    first: AuthorityRole::AgendaController,
                    second: AuthorityRole::PublicRecordEditor,
                    severity: RuleSeverity::Advisory,
                    rationale:
                        "agenda and record-editing concentration warrants independent review".into(),
                },
                RoleCombinationRule {
                    first: AuthorityRole::RuleAmender,
                    second: AuthorityRole::Auditor,
                    severity: RuleSeverity::Advisory,
                    rationale: "rule amendment and audit concentration warrants independent review"
                        .into(),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleCoverage {
    pub role: AuthorityRole,
    pub holder_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleCombinationConflict {
    pub actor_id: String,
    pub first: AuthorityRole,
    pub second: AuthorityRole,
    pub severity: RuleSeverity,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityTopologyReport {
    pub actor_count: usize,
    pub role_coverage: Vec<RoleCoverage>,
    pub missing_required_roles: Vec<AuthorityRole>,
    pub single_holder_roles: Vec<AuthorityRole>,
    pub conflicts: Vec<RoleCombinationConflict>,
    pub forbidden_conflict_count: usize,
    pub advisory_conflict_count: usize,
    /// None means independence could not be evaluated from the supplied roles.
    pub appeal_independence: Option<bool>,
    pub compliant_with_declared_policy: bool,
}

pub fn audit_authority_topology(
    actor_ids: &[String],
    assignments: &[AuthorityAssignment],
    policy: &AuthorityPolicy,
) -> Result<AuthorityTopologyReport, AuthorityError> {
    if actor_ids.len() < 2 {
        return Err(AuthorityError::TooFewActors(actor_ids.len()));
    }
    let actor_set = actor_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if actor_set.len() != actor_ids.len() {
        return Err(AuthorityError::DuplicateActorId("actor list".into()));
    }
    if actor_ids.iter().any(|actor_id| actor_id.trim().is_empty()) {
        return Err(AuthorityError::EmptyActorId);
    }

    let mut roles_by_actor = BTreeMap::new();
    for assignment in assignments {
        if !actor_set.contains(assignment.actor_id.as_str()) {
            return Err(AuthorityError::UnknownAssignedActor(
                assignment.actor_id.clone(),
            ));
        }
        if roles_by_actor.contains_key(assignment.actor_id.as_str()) {
            return Err(AuthorityError::DuplicateAssignment(
                assignment.actor_id.clone(),
            ));
        }
        let mut roles = BTreeSet::new();
        for role in &assignment.roles {
            if !roles.insert(*role) {
                return Err(AuthorityError::DuplicateRole {
                    actor_id: assignment.actor_id.clone(),
                    role: *role,
                });
            }
        }
        roles_by_actor.insert(assignment.actor_id.as_str(), roles);
    }

    let role_coverage = AuthorityRole::ALL
        .iter()
        .map(|role| {
            let holder_ids = actor_ids
                .iter()
                .filter(|actor_id| {
                    roles_by_actor
                        .get(actor_id.as_str())
                        .is_some_and(|roles| roles.contains(role))
                })
                .cloned()
                .collect();
            RoleCoverage {
                role: *role,
                holder_ids,
            }
        })
        .collect::<Vec<_>>();

    let missing_required_roles = policy
        .required_roles
        .iter()
        .copied()
        .filter(|role| {
            role_coverage
                .iter()
                .find(|coverage| coverage.role == *role)
                .is_none_or(|coverage| coverage.holder_ids.is_empty())
        })
        .collect::<Vec<_>>();
    let single_holder_roles = role_coverage
        .iter()
        .filter(|coverage| coverage.holder_ids.len() == 1)
        .map(|coverage| coverage.role)
        .collect::<Vec<_>>();

    let mut conflicts = Vec::new();
    for rule in &policy.role_combination_rules {
        for (actor_id, roles) in &roles_by_actor {
            if roles.contains(&rule.first) && roles.contains(&rule.second) {
                conflicts.push(RoleCombinationConflict {
                    actor_id: (*actor_id).into(),
                    first: rule.first,
                    second: rule.second,
                    severity: rule.severity,
                    rationale: rule.rationale.clone(),
                });
            }
        }
    }

    let forbidden_conflict_count = conflicts
        .iter()
        .filter(|conflict| conflict.severity == RuleSeverity::Forbidden)
        .count();
    let advisory_conflict_count = conflicts
        .iter()
        .filter(|conflict| conflict.severity == RuleSeverity::Advisory)
        .count();

    let original = role_coverage
        .iter()
        .find(|coverage| coverage.role == AuthorityRole::OriginalAdjudicator)
        .map(|coverage| {
            coverage
                .holder_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        });
    let appeal = role_coverage
        .iter()
        .find(|coverage| coverage.role == AuthorityRole::AppealAdjudicator)
        .map(|coverage| {
            coverage
                .holder_ids
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        });
    let appeal_independence = match (original, appeal) {
        (Some(original), Some(appeal)) if !original.is_empty() && !appeal.is_empty() => {
            Some(original.is_disjoint(&appeal))
        }
        _ => None,
    };

    let compliant_with_declared_policy = missing_required_roles.is_empty()
        && forbidden_conflict_count == 0
        && appeal_independence != Some(false);

    Ok(AuthorityTopologyReport {
        actor_count: actor_ids.len(),
        role_coverage,
        missing_required_roles,
        single_holder_roles,
        conflicts,
        forbidden_conflict_count,
        advisory_conflict_count,
        appeal_independence,
        compliant_with_declared_policy,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalDecisionTrace {
    pub proposal_id: String,
    pub proposer_id: String,
    pub approver_id: Option<String>,
    pub disburser_id: Option<String>,
    pub evidence_issuer_ids: Vec<String>,
    pub evidence_verifier_ids: Vec<String>,
    pub original_adjudicator_id: Option<String>,
    pub appeal_adjudicator_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionConflictKind {
    ProposerApprovesOwnProposal,
    ProposerDisbursesOwnProposal,
    ApproverAlsoDisburses,
    EvidenceIssuerVerifiesOwnEvidence,
    MissingEvidenceVerifier,
    OriginalAdjudicatorHearsOwnAppeal,
    MissingAppealAdjudicator,
}

impl DecisionConflictKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ProposerApprovesOwnProposal => "proposer_approves_own_proposal",
            Self::ProposerDisbursesOwnProposal => "proposer_disburses_own_proposal",
            Self::ApproverAlsoDisburses => "approver_also_disburses",
            Self::EvidenceIssuerVerifiesOwnEvidence => "evidence_issuer_verifies_own_evidence",
            Self::MissingEvidenceVerifier => "missing_evidence_verifier",
            Self::OriginalAdjudicatorHearsOwnAppeal => "original_adjudicator_hears_own_appeal",
            Self::MissingAppealAdjudicator => "missing_appeal_adjudicator",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionTraceAudit {
    pub proposal_id: String,
    pub conflicts: Vec<DecisionConflictKind>,
    pub compliant_with_reference_separation: bool,
}

pub fn audit_decision_trace(
    trace: &ProposalDecisionTrace,
    known_actor_ids: &[String],
) -> Result<DecisionTraceAudit, AuthorityError> {
    if trace.proposal_id.trim().is_empty() {
        return Err(AuthorityError::EmptyProposalId);
    }
    let known = known_actor_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let ids = std::iter::once(trace.proposer_id.as_str())
        .chain(trace.approver_id.as_deref())
        .chain(trace.disburser_id.as_deref())
        .chain(trace.evidence_issuer_ids.iter().map(String::as_str))
        .chain(trace.evidence_verifier_ids.iter().map(String::as_str))
        .chain(trace.original_adjudicator_id.as_deref())
        .chain(trace.appeal_adjudicator_id.as_deref());
    for actor_id in ids {
        if !known.contains(actor_id) {
            return Err(AuthorityError::UnknownTraceActor(actor_id.into()));
        }
    }
    if trace.proposer_id.trim().is_empty() {
        return Err(AuthorityError::EmptyActorId);
    }

    let mut conflicts = Vec::new();
    if trace.approver_id.as_deref() == Some(trace.proposer_id.as_str()) {
        conflicts.push(DecisionConflictKind::ProposerApprovesOwnProposal);
    }
    if trace.disburser_id.as_deref() == Some(trace.proposer_id.as_str()) {
        conflicts.push(DecisionConflictKind::ProposerDisbursesOwnProposal);
    }
    if trace.approver_id.is_some() && trace.approver_id == trace.disburser_id {
        conflicts.push(DecisionConflictKind::ApproverAlsoDisburses);
    }
    if trace.evidence_verifier_ids.is_empty() {
        conflicts.push(DecisionConflictKind::MissingEvidenceVerifier);
    }
    if trace
        .evidence_issuer_ids
        .iter()
        .any(|issuer| trace.evidence_verifier_ids.contains(issuer))
    {
        conflicts.push(DecisionConflictKind::EvidenceIssuerVerifiesOwnEvidence);
    }
    match (
        trace.original_adjudicator_id.as_deref(),
        trace.appeal_adjudicator_id.as_deref(),
    ) {
        (Some(original), Some(appeal)) if original == appeal => {
            conflicts.push(DecisionConflictKind::OriginalAdjudicatorHearsOwnAppeal);
        }
        (_, None) => conflicts.push(DecisionConflictKind::MissingAppealAdjudicator),
        _ => {}
    }

    Ok(DecisionTraceAudit {
        proposal_id: trace.proposal_id.clone(),
        compliant_with_reference_separation: conflicts.is_empty(),
        conflicts,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    TooFewVoters(usize),
    TooManyVoters {
        actual: usize,
        maximum: usize,
    },
    WeightCountMismatch {
        expected: usize,
        actual: usize,
    },
    EmptyActorId,
    DuplicateActorId(String),
    WeightOverflow,
    ZeroTotalWeight,
    InvalidQuota {
        quota: u64,
        total: u64,
    },
    NoCriticalVoters,
    TooFewActors(usize),
    UnknownAssignedActor(String),
    DuplicateAssignment(String),
    DuplicateRole {
        actor_id: String,
        role: AuthorityRole,
    },
    EmptyProposalId,
    UnknownTraceActor(String),
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVoters(count) => write!(f, "at least two voters required, got {count}"),
            Self::TooManyVoters { actual, maximum } => {
                write!(f, "{actual} voters exceed the exact limit of {maximum}")
            }
            Self::WeightCountMismatch { expected, actual } => {
                write!(f, "expected {expected} weights, got {actual}")
            }
            Self::EmptyActorId => write!(f, "actor IDs must not be empty"),
            Self::DuplicateActorId(id) => write!(f, "duplicate actor ID: {id}"),
            Self::WeightOverflow => write!(f, "total voting weight overflowed u64"),
            Self::ZeroTotalWeight => write!(f, "total voting weight must be positive"),
            Self::InvalidQuota { quota, total } => {
                write!(
                    f,
                    "quota {quota} must be between 1 and total weight {total}"
                )
            }
            Self::NoCriticalVoters => {
                write!(
                    f,
                    "weighted voting game has no critical voters; inspect its quota"
                )
            }
            Self::TooFewActors(count) => {
                write!(f, "at least two governance actors required, got {count}")
            }
            Self::UnknownAssignedActor(id) => write!(f, "assignment references unknown actor {id}"),
            Self::DuplicateAssignment(id) => {
                write!(f, "actor {id} has more than one role assignment")
            }
            Self::DuplicateRole { actor_id, role } => {
                write!(f, "actor {actor_id} repeats role {}", role.name())
            }
            Self::EmptyProposalId => write!(f, "proposal ID must not be empty"),
            Self::UnknownTraceActor(id) => {
                write!(f, "decision trace references unknown actor {id}")
            }
        }
    }
}

impl std::error::Error for AuthorityError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn council(weights: &[u64], quota: u64) -> WeightedCouncil {
        WeightedCouncil {
            actor_ids: (0..weights.len())
                .map(|index| format!("actor_{index}"))
                .collect(),
            weights: weights.to_vec(),
            quota,
        }
    }

    #[test]
    fn exact_banzhaf_and_shapley_shubik_match_four_voter_hand_calculation() {
        let report = analyze_voting_power(&council(&[4, 3, 2, 1], 6)).unwrap();
        let expected = [5.0 / 12.0, 3.0 / 12.0, 3.0 / 12.0, 1.0 / 12.0];
        assert_eq!(report.enumerated_coalition_count, 16);
        assert_eq!(report.minimal_winning_coalition_count, 3);
        for (actor, expected) in report.actors.iter().zip(expected) {
            assert!((actor.normalized_banzhaf - expected).abs() < 1e-12);
            assert!((actor.shapley_shubik - expected).abs() < 1e-12);
        }
    }

    #[test]
    fn banzhaf_and_shapley_shubik_are_both_reported_when_they_disagree() {
        let report = analyze_voting_power(&council(&[40, 25, 20, 10, 5], 51)).unwrap();
        assert!((report.actors[0].normalized_banzhaf - 0.44).abs() < 1e-12);
        assert!((report.actors[0].shapley_shubik - 0.45).abs() < 1e-12);
        assert_ne!(
            report.actors[0].normalized_banzhaf,
            report.actors[0].shapley_shubik
        );
    }

    #[test]
    fn a_dictator_is_detected_even_when_nominal_weight_is_only_one_input() {
        let report = analyze_voting_power(&council(&[51, 13, 13, 13, 10], 51)).unwrap();
        assert!(report.actors[0].dictator);
        assert!(report.actors[0].veto_player);
        assert_eq!(report.actors[0].normalized_banzhaf, 1.0);
        assert!(
            report.actors[1..]
                .iter()
                .all(|actor| actor.normalized_banzhaf == 0.0)
        );
    }

    #[test]
    fn symmetric_council_has_equal_power_and_zero_gini() {
        let report = analyze_voting_power(&council(&[2, 2, 2, 2, 2], 6)).unwrap();
        assert!(
            report
                .actors
                .iter()
                .all(|actor| (actor.normalized_banzhaf - 0.2).abs() < 1e-12)
        );
        assert!(report.normalized_banzhaf_gini.abs() < 1e-12);
    }

    #[test]
    fn role_policy_detects_forbidden_and_advisory_concentrations() {
        let actors = ["alice", "bob", "carol", "dave"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let assignments = vec![
            AuthorityAssignment {
                actor_id: "alice".into(),
                roles: vec![
                    AuthorityRole::Approver,
                    AuthorityRole::Disburser,
                    AuthorityRole::AgendaController,
                    AuthorityRole::Summarizer,
                ],
            },
            AuthorityAssignment {
                actor_id: "bob".into(),
                roles: vec![
                    AuthorityRole::EvidenceIssuer,
                    AuthorityRole::EvidenceVerifier,
                    AuthorityRole::OriginalAdjudicator,
                ],
            },
            AuthorityAssignment {
                actor_id: "carol".into(),
                roles: vec![AuthorityRole::AppealAdjudicator, AuthorityRole::Auditor],
            },
            AuthorityAssignment {
                actor_id: "dave".into(),
                roles: vec![AuthorityRole::PublicRecordEditor],
            },
        ];
        let report = audit_authority_topology(
            &actors,
            &assignments,
            &AuthorityPolicy::public_governance_reference(),
        )
        .unwrap();
        assert_eq!(report.forbidden_conflict_count, 2);
        assert_eq!(report.advisory_conflict_count, 1);
        assert_eq!(report.appeal_independence, Some(true));
        assert!(!report.compliant_with_declared_policy);
    }

    #[test]
    fn independent_appeal_is_distinguished_from_a_repeat_adjudicator() {
        let actors = ["alice", "bob", "carol"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let assignments = vec![
            AuthorityAssignment {
                actor_id: "alice".into(),
                roles: vec![AuthorityRole::OriginalAdjudicator],
            },
            AuthorityAssignment {
                actor_id: "bob".into(),
                roles: vec![AuthorityRole::AppealAdjudicator],
            },
            AuthorityAssignment {
                actor_id: "carol".into(),
                roles: vec![],
            },
        ];
        let report = audit_authority_topology(
            &actors,
            &assignments,
            &AuthorityPolicy {
                required_roles: vec![],
                role_combination_rules: AuthorityPolicy::public_governance_reference()
                    .role_combination_rules,
            },
        )
        .unwrap();
        assert_eq!(report.appeal_independence, Some(true));
        assert!(report.compliant_with_declared_policy);

        let non_independent = vec![
            AuthorityAssignment {
                actor_id: "alice".into(),
                roles: vec![
                    AuthorityRole::OriginalAdjudicator,
                    AuthorityRole::AppealAdjudicator,
                ],
            },
            AuthorityAssignment {
                actor_id: "bob".into(),
                roles: vec![],
            },
            AuthorityAssignment {
                actor_id: "carol".into(),
                roles: vec![],
            },
        ];
        let report = audit_authority_topology(
            &actors,
            &non_independent,
            &AuthorityPolicy::public_governance_reference(),
        )
        .unwrap();
        assert_eq!(report.appeal_independence, Some(false));
    }

    #[test]
    fn proposal_trace_catches_self_approval_self_disbursement_and_evidence_capture() {
        let known = ["alice", "bob", "carol"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let trace = ProposalDecisionTrace {
            proposal_id: "proposal_001".into(),
            proposer_id: "alice".into(),
            approver_id: Some("alice".into()),
            disburser_id: Some("alice".into()),
            evidence_issuer_ids: vec!["bob".into()],
            evidence_verifier_ids: vec!["bob".into()],
            original_adjudicator_id: Some("carol".into()),
            appeal_adjudicator_id: Some("carol".into()),
        };
        let audit = audit_decision_trace(&trace, &known).unwrap();
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::ProposerApprovesOwnProposal)
        );
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::ProposerDisbursesOwnProposal)
        );
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::ApproverAlsoDisburses)
        );
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::EvidenceIssuerVerifiesOwnEvidence)
        );
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::OriginalAdjudicatorHearsOwnAppeal)
        );
        assert!(!audit.compliant_with_reference_separation);
    }

    #[test]
    fn missing_appeal_and_evidence_review_are_visible() {
        let known = ["alice", "bob"]
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        let trace = ProposalDecisionTrace {
            proposal_id: "proposal_002".into(),
            proposer_id: "alice".into(),
            approver_id: Some("bob".into()),
            disburser_id: None,
            evidence_issuer_ids: vec![],
            evidence_verifier_ids: vec![],
            original_adjudicator_id: Some("bob".into()),
            appeal_adjudicator_id: None,
        };
        let audit = audit_decision_trace(&trace, &known).unwrap();
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::MissingEvidenceVerifier)
        );
        assert!(
            audit
                .conflicts
                .contains(&DecisionConflictKind::MissingAppealAdjudicator)
        );
    }

    #[test]
    fn exact_enumeration_fails_closed_above_declared_limit() {
        let report = council(&[1; MAX_EXACT_VOTERS + 1], 2);
        assert!(matches!(
            report.validate(),
            Err(AuthorityError::TooManyVoters { .. })
        ));
    }
}
