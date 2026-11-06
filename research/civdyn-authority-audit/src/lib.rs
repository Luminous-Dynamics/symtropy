// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Formal authority and power audit for small weighted councils.
//!
//! This kernel reports nominal voting weight, exact Banzhaf swings,
//! Shapley-Shubik pivotality, minimal winning coalitions and role conflicts.
//! It does not reduce democratic legitimacy to one score.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_EXACT_COUNCIL_SIZE: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VotingActor {
    pub id: String,
    pub weight: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedCouncil {
    pub actors: Vec<VotingActor>,
    pub quota: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActorPower {
    pub actor_id: String,
    pub nominal_weight: u64,
    pub nominal_weight_share: f64,
    pub raw_banzhaf_swings: u128,
    pub normalized_banzhaf_power: f64,
    pub shapley_shubik_power: f64,
    pub power_to_weight_ratio: Option<f64>,
    pub veto_player: bool,
    pub dictator: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalWinningCoalition {
    pub actor_ids: Vec<String>,
    pub total_weight: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CouncilPowerReport {
    pub total_weight: u64,
    pub quota: u64,
    pub actors: Vec<ActorPower>,
    pub minimal_winning_coalitions: Vec<MinimalWinningCoalition>,
    pub nominal_weight_hhi: f64,
    pub normalized_banzhaf_hhi: f64,
    pub shapley_shubik_hhi: f64,
    pub nominal_weight_gini: f64,
    pub normalized_banzhaf_gini: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Capability {
    Propose,
    ControlAgenda,
    Summarize,
    EditPublicRecord,
    IssueEvidence,
    VerifyEvidence,
    Approve,
    Disburse,
    Audit,
    AdjudicateInitial,
    AdjudicateAppeal,
    ChangeRules,
}

impl Capability {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Propose => "propose",
            Self::ControlAgenda => "control_agenda",
            Self::Summarize => "summarize",
            Self::EditPublicRecord => "edit_public_record",
            Self::IssueEvidence => "issue_evidence",
            Self::VerifyEvidence => "verify_evidence",
            Self::Approve => "approve",
            Self::Disburse => "disburse",
            Self::Audit => "audit",
            Self::AdjudicateInitial => "adjudicate_initial",
            Self::AdjudicateAppeal => "adjudicate_appeal",
            Self::ChangeRules => "change_rules",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActorPermissions {
    pub actor_id: String,
    pub capabilities: BTreeSet<Capability>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparationRule {
    pub rule_id: String,
    pub first: Capability,
    pub second: Capability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceTopology {
    pub actors: Vec<ActorPermissions>,
    pub separation_rules: Vec<SeparationRule>,
    pub initial_adjudicator_ids: BTreeSet<String>,
    pub final_appeal_adjudicator_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleConflict {
    pub actor_id: String,
    pub rule_id: String,
    pub first: Capability,
    pub second: Capability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityConcentration {
    pub capability: Capability,
    pub holder_count: usize,
    pub single_point_of_control: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernanceTopologyReport {
    pub role_conflicts: Vec<RoleConflict>,
    pub capability_concentration: Vec<CapabilityConcentration>,
    pub appeal_channel_available: bool,
    pub appeal_adjudicator_independent: bool,
    /// Means only that the declared separation rules and appeal-independence
    /// requirement pass. It is not a democratic legitimacy judgement.
    pub declared_policy_satisfied: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorityError {
    TooFewActors(usize),
    CouncilTooLarge { actual: usize, maximum: usize },
    EmptyActorId,
    DuplicateActorId(String),
    ZeroVotingWeight(String),
    WeightOverflow,
    InvalidQuota { quota: u64, total_weight: u64 },
    EmptyPermissionActor,
    DuplicatePermissionActor(String),
    UnknownInitialAdjudicator(String),
    EmptySeparationRuleId,
    DuplicateSeparationRule(String),
    SameCapabilityInRule(String),
    InvalidCoalitionMask,
}

impl fmt::Display for AuthorityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewActors(n) => write!(f, "at least two voting actors required, got {n}"),
            Self::CouncilTooLarge { actual, maximum } => write!(f, "exact enumeration limited to {maximum} actors, got {actual}"),
            Self::EmptyActorId => write!(f, "actor ID must not be empty"),
            Self::DuplicateActorId(id) => write!(f, "duplicate actor ID: {id}"),
            Self::ZeroVotingWeight(id) => write!(f, "actor {id} has zero voting weight"),
            Self::WeightOverflow => write!(f, "total voting weight overflowed u64"),
            Self::InvalidQuota { quota, total_weight } => write!(f, "quota {quota} must be in 1..={total_weight}"),
            Self::EmptyPermissionActor => write!(f, "permission actor ID must not be empty"),
            Self::DuplicatePermissionActor(id) => write!(f, "duplicate permission record for actor {id}"),
            Self::UnknownInitialAdjudicator(id) => write!(f, "unknown initial adjudicator {id}"),
            Self::EmptySeparationRuleId => write!(f, "separation rule ID must not be empty"),
            Self::DuplicateSeparationRule(id) => write!(f, "duplicate separation rule ID: {id}"),
            Self::SameCapabilityInRule(id) => write!(f, "separation rule {id} compares a capability to itself"),
            Self::InvalidCoalitionMask => write!(f, "coalition mask is outside the council"),
        }
    }
}

impl std::error::Error for AuthorityError {}

impl WeightedCouncil {
    pub fn validate(&self) -> Result<u64, AuthorityError> {
        if self.actors.len() < 2 {
            return Err(AuthorityError::TooFewActors(self.actors.len()));
        }
        if self.actors.len() > MAX_EXACT_COUNCIL_SIZE {
            return Err(AuthorityError::CouncilTooLarge {
                actual: self.actors.len(),
                maximum: MAX_EXACT_COUNCIL_SIZE,
            });
        }
        let mut ids = BTreeSet::new();
        let mut total = 0_u64;
        for actor in &self.actors {
            if actor.id.trim().is_empty() {
                return Err(AuthorityError::EmptyActorId);
            }
            if !ids.insert(actor.id.as_str()) {
                return Err(AuthorityError::DuplicateActorId(actor.id.clone()));
            }
            if actor.weight == 0 {
                return Err(AuthorityError::ZeroVotingWeight(actor.id.clone()));
            }
            total = total.checked_add(actor.weight).ok_or(AuthorityError::WeightOverflow)?;
        }
        if self.quota == 0 || self.quota > total {
            return Err(AuthorityError::InvalidQuota {
                quota: self.quota,
                total_weight: total,
            });
        }
        Ok(total)
    }

    pub fn audit(&self) -> Result<CouncilPowerReport, AuthorityError> {
        let total_weight = self.validate()?;
        let actor_count = self.actors.len();
        let coalition_count = 1_u64 << actor_count;
        let mut raw_swings = vec![0_u128; actor_count];
        let mut shapley_pivotal_permutations = vec![0_u128; actor_count];
        let mut minimal_winning_coalitions = Vec::new();

        for mask in 0..coalition_count {
            let coalition_weight = self.coalition_weight(mask)?;
            let is_winning = coalition_weight >= self.quota;
            if is_winning {
                let mut minimal = true;
                let mut actor_ids = Vec::new();
                for (index, actor) in self.actors.iter().enumerate() {
                    if mask & (1_u64 << index) != 0 {
                        actor_ids.push(actor.id.clone());
                        if coalition_weight - actor.weight >= self.quota {
                            minimal = false;
                        }
                    }
                }
                if minimal {
                    minimal_winning_coalitions.push(MinimalWinningCoalition {
                        actor_ids,
                        total_weight: coalition_weight,
                    });
                }
            }

            for (index, actor) in self.actors.iter().enumerate() {
                let bit = 1_u64 << index;
                if mask & bit != 0 {
                    continue;
                }
                if !is_winning
                    && coalition_weight < self.quota
                    && coalition_weight + actor.weight >= self.quota
                {
                    raw_swings[index] += 1;
                    let predecessor_count = mask.count_ones() as usize;
                    let permutations = factorial(predecessor_count)
                        * factorial(actor_count - predecessor_count - 1);
                    shapley_pivotal_permutations[index] += permutations;
                }
            }
        }

        minimal_winning_coalitions.sort_by(|left, right| {
            left.actor_ids.len().cmp(&right.actor_ids.len())
                .then_with(|| left.actor_ids.cmp(&right.actor_ids))
        });

        let total_swings = raw_swings.iter().sum::<u128>();
        let permutation_count = factorial(actor_count);
        let mut actors = Vec::with_capacity(actor_count);
        let mut nominal_shares = Vec::with_capacity(actor_count);
        let mut banzhaf_shares = Vec::with_capacity(actor_count);
        let mut shapley_shares = Vec::with_capacity(actor_count);

        for (index, actor) in self.actors.iter().enumerate() {
            let nominal_share = actor.weight as f64 / total_weight as f64;
            let banzhaf_share = if total_swings == 0 {
                0.0
            } else {
                raw_swings[index] as f64 / total_swings as f64
            };
            let shapley_share =
                shapley_pivotal_permutations[index] as f64 / permutation_count as f64;
            let veto_player = total_weight - actor.weight < self.quota;
            let dictator = veto_player && actor.weight >= self.quota;
            nominal_shares.push(nominal_share);
            banzhaf_shares.push(banzhaf_share);
            shapley_shares.push(shapley_share);
            actors.push(ActorPower {
                actor_id: actor.id.clone(),
                nominal_weight: actor.weight,
                nominal_weight_share: nominal_share,
                raw_banzhaf_swings: raw_swings[index],
                normalized_banzhaf_power: banzhaf_share,
                shapley_shubik_power: shapley_share,
                power_to_weight_ratio: if nominal_share > 0.0 {
                    Some(banzhaf_share / nominal_share)
                } else {
                    None
                },
                veto_player,
                dictator,
            });
        }

        Ok(CouncilPowerReport {
            total_weight,
            quota: self.quota,
            actors,
            minimal_winning_coalitions,
            nominal_weight_hhi: hhi(&nominal_shares),
            normalized_banzhaf_hhi: hhi(&banzhaf_shares),
            shapley_shubik_hhi: hhi(&shapley_shares),
            nominal_weight_gini: gini(&nominal_shares),
            normalized_banzhaf_gini: gini(&banzhaf_shares),
        })
    }

    fn coalition_weight(&self, mask: u64) -> Result<u64, AuthorityError> {
        let mut total = 0_u64;
        for (index, actor) in self.actors.iter().enumerate() {
            if mask & (1_u64 << index) != 0 {
                total = total.checked_add(actor.weight).ok_or(AuthorityError::WeightOverflow)?;
            }
        }
        Ok(total)
    }
}

impl GovernanceTopology {
    pub fn audit(&self) -> Result<GovernanceTopologyReport, AuthorityError> {
        let mut actor_map = BTreeMap::new();
        for actor in &self.actors {
            if actor.actor_id.trim().is_empty() {
                return Err(AuthorityError::EmptyPermissionActor);
            }
            if actor_map.insert(actor.actor_id.as_str(), actor).is_some() {
                return Err(AuthorityError::DuplicatePermissionActor(actor.actor_id.clone()));
            }
        }

        for id in &self.initial_adjudicator_ids {
            if !actor_map.contains_key(id.as_str()) {
                return Err(AuthorityError::UnknownInitialAdjudicator(id.clone()));
            }
        }

        let mut rule_ids = BTreeSet::new();
        let mut role_conflicts = Vec::new();
        for rule in &self.separation_rules {
            if rule.rule_id.trim().is_empty() {
                return Err(AuthorityError::EmptySeparationRuleId);
            }
            if !rule_ids.insert(rule.rule_id.as_str()) {
                return Err(AuthorityError::DuplicateSeparationRule(rule.rule_id.clone()));
            }
            if rule.first == rule.second {
                return Err(AuthorityError::SameCapabilityInRule(rule.rule_id.clone()));
            }
            for actor in &self.actors {
                if actor.capabilities.contains(&rule.first)
                    && actor.capabilities.contains(&rule.second)
                {
                    role_conflicts.push(RoleConflict {
                        actor_id: actor.actor_id.clone(),
                        rule_id: rule.rule_id.clone(),
                        first: rule.first,
                        second: rule.second,
                    });
                }
            }
        }

        let capabilities = [
            Capability::Propose,
            Capability::ControlAgenda,
            Capability::Summarize,
            Capability::EditPublicRecord,
            Capability::IssueEvidence,
            Capability::VerifyEvidence,
            Capability::Approve,
            Capability::Disburse,
            Capability::Audit,
            Capability::AdjudicateInitial,
            Capability::AdjudicateAppeal,
            Capability::ChangeRules,
        ];
        let capability_concentration = capabilities
            .iter()
            .map(|capability| {
                let holder_count = self.actors
                    .iter()
                    .filter(|actor| actor.capabilities.contains(capability))
                    .count();
                CapabilityConcentration {
                    capability: *capability,
                    holder_count,
                    single_point_of_control: holder_count == 1,
                }
            })
            .collect::<Vec<_>>();

        let appeal_adjudicator = self.final_appeal_adjudicator_id.as_deref();
        let appeal_channel_available = appeal_adjudicator
            .and_then(|id| actor_map.get(id))
            .is_some_and(|actor| actor.capabilities.contains(&Capability::AdjudicateAppeal));
        let appeal_adjudicator_independent = appeal_channel_available
            && appeal_adjudicator.is_some_and(|id| !self.initial_adjudicator_ids.contains(id));

        let declared_policy_satisfied =
            role_conflicts.is_empty() && appeal_channel_available && appeal_adjudicator_independent;

        Ok(GovernanceTopologyReport {
            role_conflicts,
            capability_concentration,
            appeal_channel_available,
            appeal_adjudicator_independent,
            declared_policy_satisfied,
        })
    }
}

fn factorial(n: usize) -> u128 {
    (1..=n as u128).product()
}

fn hhi(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum()
}

fn gini(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let total = ordered.iter().sum::<f64>();
    if total <= f64::EPSILON {
        return 0.0;
    }
    let weighted = ordered
        .iter()
        .enumerate()
        .map(|(index, value)| (index + 1) as f64 * value)
        .sum::<f64>();
    (2.0 * weighted / (ordered.len() as f64 * total)
        - (ordered.len() as f64 + 1.0) / ordered.len() as f64)
        .clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(id: &str, weight: u64) -> VotingActor {
        VotingActor { id: id.into(), weight }
    }

    #[test]
    fn equal_three_member_majority_has_equal_exact_power_indices() {
        let report = WeightedCouncil {
            actors: vec![actor("a", 1), actor("b", 1), actor("c", 1)],
            quota: 2,
        }.audit().unwrap();

        assert_eq!(report.minimal_winning_coalitions.len(), 3);
        assert!(report.actors.iter().all(|actor| actor.raw_banzhaf_swings == 2));
        assert!(report.actors.iter().all(|actor| (actor.normalized_banzhaf_power - 1.0 / 3.0).abs() < 1e-12));
        assert!(report.actors.iter().all(|actor| (actor.shapley_shubik_power - 1.0 / 3.0).abs() < 1e-12));
    }

    #[test]
    fn weighted_voting_power_can_exceed_nominal_weight_concentration() {
        let report = WeightedCouncil {
            actors: vec![actor("large", 2), actor("small_a", 1), actor("small_b", 1)],
            quota: 3,
        }.audit().unwrap();

        let large = &report.actors[0];
        assert_eq!(large.raw_banzhaf_swings, 3);
        assert!((large.normalized_banzhaf_power - 0.6).abs() < 1e-12);
        assert!((large.shapley_shubik_power - 2.0 / 3.0).abs() < 1e-12);
        assert!(large.normalized_banzhaf_power > large.nominal_weight_share);
        assert_eq!(report.minimal_winning_coalitions.len(), 2);
    }

    #[test]
    fn quota_one_member_is_a_dictator_and_veto_player() {
        let report = WeightedCouncil {
            actors: vec![actor("majority", 51), actor("minority", 49)],
            quota: 51,
        }.audit().unwrap();
        assert!(report.actors[0].dictator);
        assert!(report.actors[0].veto_player);
        assert!(!report.actors[1].veto_player);
        assert_eq!(report.actors[0].normalized_banzhaf_power, 1.0);
        assert_eq!(report.actors[1].normalized_banzhaf_power, 0.0);
    }

    #[test]
    fn exact_enumeration_fails_closed_above_limit() {
        let council = WeightedCouncil {
            actors: (0..=MAX_EXACT_COUNCIL_SIZE).map(|index| actor(&format!("a{index}"), 1)).collect(),
            quota: 2,
        };
        assert!(matches!(council.validate(), Err(AuthorityError::CouncilTooLarge { .. })));
    }

    #[test]
    fn duplicate_ids_and_invalid_quota_fail_closed() {
        let duplicate = WeightedCouncil {
            actors: vec![actor("same", 1), actor("same", 1)],
            quota: 1,
        };
        assert!(matches!(duplicate.validate(), Err(AuthorityError::DuplicateActorId(_))));

        let bad_quota = WeightedCouncil {
            actors: vec![actor("a", 1), actor("b", 1)],
            quota: 3,
        };
        assert!(matches!(bad_quota.validate(), Err(AuthorityError::InvalidQuota { .. })));
    }

    #[test]
    fn role_conflicts_and_independent_appeal_are_reported_separately() {
        let topology = GovernanceTopology {
            actors: vec![
                ActorPermissions {
                    actor_id: "concentrated".into(),
                    capabilities: BTreeSet::from([
                        Capability::Propose,
                        Capability::Approve,
                        Capability::Disburse,
                        Capability::IssueEvidence,
                        Capability::VerifyEvidence,
                        Capability::ControlAgenda,
                        Capability::EditPublicRecord,
                        Capability::AdjudicateInitial,
                        Capability::AdjudicateAppeal,
                    ]),
                },
                ActorPermissions {
                    actor_id: "independent_appeal".into(),
                    capabilities: BTreeSet::from([Capability::AdjudicateAppeal]),
                },
            ],
            separation_rules: vec![
                SeparationRule { rule_id: "proposal_approval".into(), first: Capability::Propose, second: Capability::Approve },
                SeparationRule { rule_id: "approval_disbursement".into(), first: Capability::Approve, second: Capability::Disburse },
                SeparationRule { rule_id: "evidence_separation".into(), first: Capability::IssueEvidence, second: Capability::VerifyEvidence },
                SeparationRule { rule_id: "record_appeal_separation".into(), first: Capability::EditPublicRecord, second: Capability::AdjudicateInitial },
            ],
            initial_adjudicator_ids: BTreeSet::from(["concentrated".into()]),
            final_appeal_adjudicator_id: Some("independent_appeal".into()),
        };
        let report = topology.audit().unwrap();
        assert_eq!(report.role_conflicts.len(), 4);
        assert!(report.appeal_channel_available);
        assert!(report.appeal_adjudicator_independent);
        assert!(!report.declared_policy_satisfied);
        assert!(report.capability_concentration.iter().any(|item| item.single_point_of_control));
    }

    #[test]
    fn appeal_by_original_adjudicator_is_not_independent() {
        let topology = GovernanceTopology {
            actors: vec![ActorPermissions {
                actor_id: "same_actor".into(),
                capabilities: BTreeSet::from([Capability::AdjudicateInitial, Capability::AdjudicateAppeal]),
            }],
            separation_rules: vec![],
            initial_adjudicator_ids: BTreeSet::from(["same_actor".into()]),
            final_appeal_adjudicator_id: Some("same_actor".into()),
        };
        let report = topology.audit().unwrap();
        assert!(report.appeal_channel_available);
        assert!(!report.appeal_adjudicator_independent);
    }
}
