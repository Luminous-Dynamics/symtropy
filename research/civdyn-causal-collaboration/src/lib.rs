// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Evidence-qualified causal collaboration metrics for multi-agent action traces.
//!
//! This crate scores a supplied causal-action graph. It does not generate causal
//! edges, simulate an environment, or treat a model's proposed edge as ground
//! truth. Verified edges define a lower bound; candidate edges define an upper
//! bound on Causal Collaboration Effectiveness (CCE).

use core::fmt;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const MAX_AGENTS: usize = 1_000;
pub const MAX_ACTIONS: usize = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionKind {
    EnvironmentAction,
    Communication,
    ResourceTransfer,
    Checkpoint,
    Other,
}

impl ActionKind {
    pub const fn is_communication(self) -> bool {
        matches!(self, Self::Communication)
    }

    pub const fn is_resource_transfer(self) -> bool {
        matches!(self, Self::ResourceTransfer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRecord {
    pub agent_id: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionRecord {
    pub action_id: String,
    pub agent_id: String,
    /// Global causal order; must be unique and strictly increase along edges.
    pub sequence: u64,
    /// Domain round, which may contain several sequential actions.
    pub round: u32,
    pub kind: ActionKind,
    pub fingerprint: Option<String>,
    pub resource_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalEvidenceKind {
    EnvironmentStateTransition,
    ResourceLineageReceipt,
    AcknowledgedCommunication,
    DeterministicPrecondition,
    HumanAdjudication,
    ModelProposed,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalEdgeStatus {
    Verified,
    Candidate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalEdge {
    pub cause_action_id: String,
    pub effect_action_id: String,
    pub evidence_ref: String,
    pub evidence_kind: CausalEvidenceKind,
    pub status: CausalEdgeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollaborationTrace {
    pub agents: Vec<AgentRecord>,
    pub actions: Vec<ActionRecord>,
    pub causal_edges: Vec<CausalEdge>,
    /// Action IDs that directly achieved one or more final task success criteria.
    pub success_action_ids: Vec<String>,
    pub task_succeeded: bool,
    pub partial_success_count: u32,
    pub partial_success_total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollaborationError {
    InvalidAgentCount(usize),
    TooManyActions(usize),
    EmptyAgentId,
    EmptyAgentRole {
        agent_id: String,
    },
    DuplicateAgentId(String),
    EmptyActionId {
        index: usize,
    },
    DuplicateActionId(String),
    UnknownActionAgent {
        action_id: String,
        agent_id: String,
    },
    DuplicateSequence(u64),
    EmptyFingerprint {
        action_id: String,
    },
    EmptyResourceReference {
        action_id: String,
    },
    NonMonotonicRound {
        earlier_action: String,
        later_action: String,
    },
    InvalidSuccessActionSet,
    DuplicateSuccessAction(String),
    UnknownSuccessAction(String),
    PartialSuccessOutOfRange {
        count: u32,
        total: u32,
    },
    EmptyCausalEvidenceReference,
    UnknownCausalAction {
        action_id: String,
    },
    SelfCausalEdge(String),
    NonForwardCausalEdge {
        cause: String,
        effect: String,
    },
    DuplicateCausalEdge {
        cause: String,
        effect: String,
    },
    ModelProposalMarkedVerified {
        cause: String,
        effect: String,
    },
}

impl fmt::Display for CollaborationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAgentCount(count) => {
                write!(f, "agent count must be in 1..={MAX_AGENTS}, got {count}")
            }
            Self::TooManyActions(count) => {
                write!(f, "action count must not exceed {MAX_ACTIONS}, got {count}")
            }
            Self::EmptyAgentId => write!(f, "agent ID must not be empty"),
            Self::EmptyAgentRole { agent_id } => {
                write!(f, "role must not be empty for agent {agent_id}")
            }
            Self::DuplicateAgentId(id) => write!(f, "duplicate agent ID: {id}"),
            Self::EmptyActionId { index } => {
                write!(f, "action at index {index} has an empty ID")
            }
            Self::DuplicateActionId(id) => write!(f, "duplicate action ID: {id}"),
            Self::UnknownActionAgent {
                action_id,
                agent_id,
            } => {
                write!(f, "action {action_id} refers to unknown agent {agent_id}")
            }
            Self::DuplicateSequence(sequence) => {
                write!(f, "global action sequence is duplicated: {sequence}")
            }
            Self::EmptyFingerprint { action_id } => {
                write!(f, "action {action_id} has an empty fingerprint")
            }
            Self::EmptyResourceReference { action_id } => {
                write!(f, "action {action_id} has an empty resource reference")
            }
            Self::NonMonotonicRound {
                earlier_action,
                later_action,
            } => write!(
                f,
                "round order decreases from action {earlier_action} to {later_action}"
            ),
            Self::InvalidSuccessActionSet => {
                write!(
                    f,
                    "successful tasks need success actions; failed tasks must not list them"
                )
            }
            Self::DuplicateSuccessAction(id) => {
                write!(f, "success action ID is duplicated: {id}")
            }
            Self::UnknownSuccessAction(id) => {
                write!(f, "success action ID is unknown: {id}")
            }
            Self::PartialSuccessOutOfRange { count, total } => {
                write!(
                    f,
                    "partial success count {count} exceeds declared total {total}"
                )
            }
            Self::EmptyCausalEvidenceReference => {
                write!(f, "causal edge evidence reference must not be empty")
            }
            Self::UnknownCausalAction { action_id } => {
                write!(f, "causal edge refers to unknown action {action_id}")
            }
            Self::SelfCausalEdge(id) => write!(f, "action {id} cannot cause itself"),
            Self::NonForwardCausalEdge { cause, effect } => {
                write!(
                    f,
                    "causal edge {cause} -> {effect} is not strictly forward in sequence"
                )
            }
            Self::DuplicateCausalEdge { cause, effect } => {
                write!(f, "causal edge {cause} -> {effect} is duplicated")
            }
            Self::ModelProposalMarkedVerified { cause, effect } => write!(
                f,
                "model-proposed edge {cause} -> {effect} cannot be marked verified without independent adjudication"
            ),
        }
    }
}

impl std::error::Error for CollaborationError {}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentContribution {
    pub agent_id: String,
    pub role: String,
    pub action_count: usize,
    pub verified_contributing_actions: usize,
    pub candidate_inclusive_contributing_actions: usize,
    /// Causal contribution per action by this agent; None means the agent acted zero times.
    pub pac_verified_lower: Option<f64>,
    pub pac_candidate_upper: Option<f64>,
    /// Share of all contributing actions; None means no actions contribute.
    pub useful_effort_share_lower: Option<f64>,
    pub useful_effort_share_upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CollaborationMetrics {
    pub task_succeeded: bool,
    pub action_count: usize,
    pub agent_count: usize,
    pub round_count: u32,
    pub partial_success_ratio: Option<f64>,
    pub verified_contributing_actions: usize,
    pub candidate_inclusive_contributing_actions: usize,
    pub cce_verified_lower: f64,
    pub cce_candidate_upper: f64,
    pub verified_cross_agent_causal_edges: usize,
    pub candidate_inclusive_cross_agent_causal_edges: usize,
    pub verified_cross_agent_contributing_actions: usize,
    pub candidate_inclusive_cross_agent_contributing_actions: usize,
    pub message_action_count: usize,
    pub verified_contributing_messages: usize,
    pub candidate_inclusive_contributing_messages: usize,
    pub message_effectiveness_verified_lower: Option<f64>,
    pub message_effectiveness_candidate_upper: Option<f64>,
    pub resource_transfer_action_count: usize,
    /// Number of action instances beyond the first for repeated fingerprints.
    pub duplicate_action_count: usize,
    pub duplicate_fingerprint_groups: usize,
    pub per_agent: Vec<AgentContribution>,
}

impl CollaborationTrace {
    pub fn validate(&self) -> Result<(), CollaborationError> {
        if self.agents.is_empty() || self.agents.len() > MAX_AGENTS {
            return Err(CollaborationError::InvalidAgentCount(self.agents.len()));
        }
        if self.actions.len() > MAX_ACTIONS {
            return Err(CollaborationError::TooManyActions(self.actions.len()));
        }

        let mut agent_ids = BTreeSet::new();
        for agent in &self.agents {
            if agent.agent_id.trim().is_empty() {
                return Err(CollaborationError::EmptyAgentId);
            }
            if agent.role.trim().is_empty() {
                return Err(CollaborationError::EmptyAgentRole {
                    agent_id: agent.agent_id.clone(),
                });
            }
            if !agent_ids.insert(agent.agent_id.as_str()) {
                return Err(CollaborationError::DuplicateAgentId(agent.agent_id.clone()));
            }
        }

        let mut action_ids = BTreeMap::new();
        let mut sequences = BTreeSet::new();
        for (index, action) in self.actions.iter().enumerate() {
            if action.action_id.trim().is_empty() {
                return Err(CollaborationError::EmptyActionId { index });
            }
            if action_ids
                .insert(action.action_id.as_str(), action)
                .is_some()
            {
                return Err(CollaborationError::DuplicateActionId(
                    action.action_id.clone(),
                ));
            }
            if !agent_ids.contains(action.agent_id.as_str()) {
                return Err(CollaborationError::UnknownActionAgent {
                    action_id: action.action_id.clone(),
                    agent_id: action.agent_id.clone(),
                });
            }
            if !sequences.insert(action.sequence) {
                return Err(CollaborationError::DuplicateSequence(action.sequence));
            }
            if action
                .fingerprint
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(CollaborationError::EmptyFingerprint {
                    action_id: action.action_id.clone(),
                });
            }
            if action
                .resource_ref
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            {
                return Err(CollaborationError::EmptyResourceReference {
                    action_id: action.action_id.clone(),
                });
            }
        }

        let mut ordered_actions = self.actions.iter().collect::<Vec<_>>();
        ordered_actions.sort_by_key(|action| action.sequence);
        for pair in ordered_actions.windows(2) {
            if pair[1].round < pair[0].round {
                return Err(CollaborationError::NonMonotonicRound {
                    earlier_action: pair[0].action_id.clone(),
                    later_action: pair[1].action_id.clone(),
                });
            }
        }

        if self.task_succeeded == self.success_action_ids.is_empty() {
            return Err(CollaborationError::InvalidSuccessActionSet);
        }
        let mut success_ids = BTreeSet::new();
        for id in &self.success_action_ids {
            if !success_ids.insert(id.as_str()) {
                return Err(CollaborationError::DuplicateSuccessAction(id.clone()));
            }
            if !action_ids.contains_key(id.as_str()) {
                return Err(CollaborationError::UnknownSuccessAction(id.clone()));
            }
        }
        if self.partial_success_count > self.partial_success_total {
            return Err(CollaborationError::PartialSuccessOutOfRange {
                count: self.partial_success_count,
                total: self.partial_success_total,
            });
        }
        if self.partial_success_total == 0 && self.partial_success_count != 0 {
            return Err(CollaborationError::PartialSuccessOutOfRange {
                count: self.partial_success_count,
                total: self.partial_success_total,
            });
        }

        let mut edge_pairs = BTreeSet::new();
        for edge in &self.causal_edges {
            if edge.evidence_ref.trim().is_empty() {
                return Err(CollaborationError::EmptyCausalEvidenceReference);
            }
            let cause = action_ids
                .get(edge.cause_action_id.as_str())
                .ok_or_else(|| CollaborationError::UnknownCausalAction {
                    action_id: edge.cause_action_id.clone(),
                })?;
            let effect = action_ids
                .get(edge.effect_action_id.as_str())
                .ok_or_else(|| CollaborationError::UnknownCausalAction {
                    action_id: edge.effect_action_id.clone(),
                })?;
            if edge.cause_action_id == edge.effect_action_id {
                return Err(CollaborationError::SelfCausalEdge(
                    edge.cause_action_id.clone(),
                ));
            }
            if cause.sequence >= effect.sequence {
                return Err(CollaborationError::NonForwardCausalEdge {
                    cause: edge.cause_action_id.clone(),
                    effect: edge.effect_action_id.clone(),
                });
            }
            if !edge_pairs.insert((
                edge.cause_action_id.as_str(),
                edge.effect_action_id.as_str(),
            )) {
                return Err(CollaborationError::DuplicateCausalEdge {
                    cause: edge.cause_action_id.clone(),
                    effect: edge.effect_action_id.clone(),
                });
            }
            if edge.evidence_kind == CausalEvidenceKind::ModelProposed
                && edge.status == CausalEdgeStatus::Verified
            {
                return Err(CollaborationError::ModelProposalMarkedVerified {
                    cause: edge.cause_action_id.clone(),
                    effect: edge.effect_action_id.clone(),
                });
            }
        }

        Ok(())
    }

    pub fn score(&self) -> Result<CollaborationMetrics, CollaborationError> {
        self.validate()?;

        let verified = self.contributing_actions(false);
        let candidate_inclusive = self.contributing_actions(true);
        let action_count = self.actions.len();
        let verified_count = verified.len();
        let candidate_count = candidate_inclusive.len();

        let mut agent_actions: HashMap<&str, usize> = HashMap::new();
        let mut agent_verified: HashMap<&str, usize> = HashMap::new();
        let mut agent_candidate: HashMap<&str, usize> = HashMap::new();
        let action_agent: HashMap<&str, &str> = self
            .actions
            .iter()
            .map(|action| (action.action_id.as_str(), action.agent_id.as_str()))
            .collect();

        let mut message_action_count = 0_usize;
        let mut verified_contributing_messages = 0_usize;
        let mut candidate_inclusive_contributing_messages = 0_usize;
        let mut resource_transfer_action_count = 0_usize;
        let mut fingerprints: BTreeMap<&str, usize> = BTreeMap::new();

        for action in &self.actions {
            *agent_actions.entry(action.agent_id.as_str()).or_default() += 1;
            if verified.contains(&action.action_id) {
                *agent_verified.entry(action.agent_id.as_str()).or_default() += 1;
            }
            if candidate_inclusive.contains(&action.action_id) {
                *agent_candidate.entry(action.agent_id.as_str()).or_default() += 1;
            }
            if action.kind.is_communication() {
                message_action_count += 1;
                verified_contributing_messages += if verified.contains(&action.action_id) {
                    1
                } else {
                    0
                };
                candidate_inclusive_contributing_messages +=
                    if candidate_inclusive.contains(&action.action_id) {
                        1
                    } else {
                        0
                    };
            }
            if action.kind.is_resource_transfer() {
                resource_transfer_action_count += 1;
            }
            if let Some(fingerprint) = &action.fingerprint {
                *fingerprints.entry(fingerprint.as_str()).or_default() += 1;
            }
        }

        let mut verified_cross_agent_causal_edges = 0_usize;
        let mut candidate_inclusive_cross_agent_causal_edges = 0_usize;
        let mut verified_cross_agent_contributing_actions = BTreeSet::new();
        let mut candidate_inclusive_cross_agent_contributing_actions = BTreeSet::new();

        for edge in &self.causal_edges {
            let cause_agent = action_agent[edge.cause_action_id.as_str()];
            let effect_agent = action_agent[edge.effect_action_id.as_str()];
            if cause_agent == effect_agent
                || !candidate_inclusive.contains(&edge.cause_action_id)
                || !candidate_inclusive.contains(&edge.effect_action_id)
            {
                continue;
            }
            candidate_inclusive_cross_agent_causal_edges += 1;
            candidate_inclusive_cross_agent_contributing_actions
                .insert(edge.cause_action_id.clone());
            candidate_inclusive_cross_agent_contributing_actions
                .insert(edge.effect_action_id.clone());

            if edge.status == CausalEdgeStatus::Verified
                && verified.contains(&edge.cause_action_id)
                && verified.contains(&edge.effect_action_id)
            {
                verified_cross_agent_causal_edges += 1;
                verified_cross_agent_contributing_actions.insert(edge.cause_action_id.clone());
                verified_cross_agent_contributing_actions.insert(edge.effect_action_id.clone());
            }
        }

        let duplicate_action_count = fingerprints
            .values()
            .map(|count| count.saturating_sub(1))
            .sum::<usize>();
        let duplicate_fingerprint_groups =
            fingerprints.values().filter(|count| **count > 1).count();

        let per_agent = self
            .agents
            .iter()
            .map(|agent| {
                let action_count = *agent_actions.get(agent.agent_id.as_str()).unwrap_or(&0);
                let verified_contributing_actions =
                    *agent_verified.get(agent.agent_id.as_str()).unwrap_or(&0);
                let candidate_inclusive_contributing_actions =
                    *agent_candidate.get(agent.agent_id.as_str()).unwrap_or(&0);
                AgentContribution {
                    agent_id: agent.agent_id.clone(),
                    role: agent.role.clone(),
                    action_count,
                    verified_contributing_actions,
                    candidate_inclusive_contributing_actions,
                    pac_verified_lower: ratio(verified_contributing_actions, action_count),
                    pac_candidate_upper: ratio(
                        candidate_inclusive_contributing_actions,
                        action_count,
                    ),
                    useful_effort_share_lower: ratio(verified_contributing_actions, verified_count),
                    useful_effort_share_upper: ratio(
                        candidate_inclusive_contributing_actions,
                        candidate_count,
                    ),
                }
            })
            .collect();

        let (cce_verified_lower, cce_candidate_upper) = if self.task_succeeded {
            (
                ratio(verified_count, action_count).unwrap_or(0.0),
                ratio(candidate_count, action_count).unwrap_or(0.0),
            )
        } else {
            (0.0, 0.0)
        };

        Ok(CollaborationMetrics {
            task_succeeded: self.task_succeeded,
            action_count,
            agent_count: self.agents.len(),
            round_count: self
                .actions
                .iter()
                .map(|action| action.round)
                .max()
                .unwrap_or(0),
            partial_success_ratio: ratio(
                self.partial_success_count as usize,
                self.partial_success_total as usize,
            ),
            verified_contributing_actions: verified_count,
            candidate_inclusive_contributing_actions: candidate_count,
            cce_verified_lower,
            cce_candidate_upper,
            verified_cross_agent_causal_edges,
            candidate_inclusive_cross_agent_causal_edges,
            verified_cross_agent_contributing_actions: verified_cross_agent_contributing_actions
                .len(),
            candidate_inclusive_cross_agent_contributing_actions:
                candidate_inclusive_cross_agent_contributing_actions.len(),
            message_action_count,
            verified_contributing_messages,
            candidate_inclusive_contributing_messages,
            message_effectiveness_verified_lower: ratio(
                verified_contributing_messages,
                message_action_count,
            ),
            message_effectiveness_candidate_upper: ratio(
                candidate_inclusive_contributing_messages,
                message_action_count,
            ),
            resource_transfer_action_count,
            duplicate_action_count,
            duplicate_fingerprint_groups,
            per_agent,
        })
    }

    fn contributing_actions(&self, include_candidate_edges: bool) -> BTreeSet<String> {
        if !self.task_succeeded {
            return BTreeSet::new();
        }
        let mut contributing = BTreeSet::new();
        let mut frontier = self.success_action_ids.clone();
        while let Some(action_id) = frontier.pop() {
            if !contributing.insert(action_id.clone()) {
                continue;
            }
            for edge in &self.causal_edges {
                let included = edge.status == CausalEdgeStatus::Verified || include_candidate_edges;
                if included && edge.effect_action_id == action_id {
                    frontier.push(edge.cause_action_id.clone());
                }
            }
        }
        contributing
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> CollaborationTrace {
        CollaborationTrace {
            agents: vec![
                AgentRecord {
                    agent_id: "miner".into(),
                    role: "extract resources".into(),
                },
                AgentRecord {
                    agent_id: "courier".into(),
                    role: "transport resources".into(),
                },
                AgentRecord {
                    agent_id: "smith".into(),
                    role: "craft equipment".into(),
                },
                AgentRecord {
                    agent_id: "wizard".into(),
                    role: "provide instructions".into(),
                },
                AgentRecord {
                    agent_id: "idle".into(),
                    role: "standby".into(),
                },
            ],
            actions: vec![
                ActionRecord {
                    action_id: "ore".into(),
                    agent_id: "miner".into(),
                    sequence: 1,
                    round: 1,
                    kind: ActionKind::EnvironmentAction,
                    fingerprint: Some("mine-iron".into()),
                    resource_ref: Some("ore-1".into()),
                },
                ActionRecord {
                    action_id: "message".into(),
                    agent_id: "miner".into(),
                    sequence: 2,
                    round: 1,
                    kind: ActionKind::Communication,
                    fingerprint: None,
                    resource_ref: None,
                },
                ActionRecord {
                    action_id: "wood".into(),
                    agent_id: "courier".into(),
                    sequence: 3,
                    round: 2,
                    kind: ActionKind::EnvironmentAction,
                    fingerprint: Some("gather-wood".into()),
                    resource_ref: Some("wood-1".into()),
                },
                ActionRecord {
                    action_id: "transfer".into(),
                    agent_id: "courier".into(),
                    sequence: 4,
                    round: 2,
                    kind: ActionKind::ResourceTransfer,
                    fingerprint: Some("transfer-ore-1".into()),
                    resource_ref: Some("ore-1".into()),
                },
                ActionRecord {
                    action_id: "craft".into(),
                    agent_id: "smith".into(),
                    sequence: 5,
                    round: 3,
                    kind: ActionKind::EnvironmentAction,
                    fingerprint: Some("craft-vanguard".into()),
                    resource_ref: None,
                },
                ActionRecord {
                    action_id: "hint".into(),
                    agent_id: "wizard".into(),
                    sequence: 6,
                    round: 3,
                    kind: ActionKind::Communication,
                    fingerprint: None,
                    resource_ref: None,
                },
                ActionRecord {
                    action_id: "duplicate-craft".into(),
                    agent_id: "smith".into(),
                    sequence: 7,
                    round: 4,
                    kind: ActionKind::EnvironmentAction,
                    fingerprint: Some("craft-vanguard".into()),
                    resource_ref: None,
                },
                ActionRecord {
                    action_id: "deliver".into(),
                    agent_id: "smith".into(),
                    sequence: 8,
                    round: 5,
                    kind: ActionKind::Checkpoint,
                    fingerprint: Some("deliver-vanguard".into()),
                    resource_ref: None,
                },
            ],
            causal_edges: vec![
                CausalEdge {
                    cause_action_id: "ore".into(),
                    effect_action_id: "transfer".into(),
                    evidence_ref: "ledger/ore-1".into(),
                    evidence_kind: CausalEvidenceKind::ResourceLineageReceipt,
                    status: CausalEdgeStatus::Verified,
                },
                CausalEdge {
                    cause_action_id: "message".into(),
                    effect_action_id: "transfer".into(),
                    evidence_ref: "chat/ack-44".into(),
                    evidence_kind: CausalEvidenceKind::AcknowledgedCommunication,
                    status: CausalEdgeStatus::Candidate,
                },
                CausalEdge {
                    cause_action_id: "wood".into(),
                    effect_action_id: "craft".into(),
                    evidence_ref: "ledger/wood-1".into(),
                    evidence_kind: CausalEvidenceKind::ResourceLineageReceipt,
                    status: CausalEdgeStatus::Verified,
                },
                CausalEdge {
                    cause_action_id: "transfer".into(),
                    effect_action_id: "craft".into(),
                    evidence_ref: "inventory/receipt-7".into(),
                    evidence_kind: CausalEvidenceKind::EnvironmentStateTransition,
                    status: CausalEdgeStatus::Verified,
                },
                CausalEdge {
                    cause_action_id: "craft".into(),
                    effect_action_id: "deliver".into(),
                    evidence_ref: "task/verifier-1".into(),
                    evidence_kind: CausalEvidenceKind::DeterministicPrecondition,
                    status: CausalEdgeStatus::Verified,
                },
                CausalEdge {
                    cause_action_id: "hint".into(),
                    effect_action_id: "craft".into(),
                    evidence_ref: "model-judge/proposal-2".into(),
                    evidence_kind: CausalEvidenceKind::ModelProposed,
                    status: CausalEdgeStatus::Candidate,
                },
            ],
            success_action_ids: vec!["deliver".into()],
            task_succeeded: true,
            partial_success_count: 4,
            partial_success_total: 4,
        }
    }

    #[test]
    fn cce_lower_and_upper_bounds_are_hand_computable() {
        let metrics = fixture().score().unwrap();
        assert_eq!(metrics.action_count, 8);
        assert_eq!(metrics.verified_contributing_actions, 5);
        assert_eq!(metrics.candidate_inclusive_contributing_actions, 7);
        assert!((metrics.cce_verified_lower - 5.0 / 8.0).abs() < 1e-12);
        assert!((metrics.cce_candidate_upper - 7.0 / 8.0).abs() < 1e-12);
    }

    #[test]
    fn per_agent_pac_and_idle_agent_are_reported() {
        let metrics = fixture().score().unwrap();
        let miner = metrics
            .per_agent
            .iter()
            .find(|agent| agent.agent_id == "miner")
            .unwrap();
        let smith = metrics
            .per_agent
            .iter()
            .find(|agent| agent.agent_id == "smith")
            .unwrap();
        let wizard = metrics
            .per_agent
            .iter()
            .find(|agent| agent.agent_id == "wizard")
            .unwrap();
        let idle = metrics
            .per_agent
            .iter()
            .find(|agent| agent.agent_id == "idle")
            .unwrap();

        assert_eq!(miner.pac_verified_lower, Some(0.5));
        assert_eq!(miner.pac_candidate_upper, Some(1.0));
        assert_eq!(smith.action_count, 3);
        assert!((smith.pac_verified_lower.unwrap() - 2.0 / 3.0).abs() < 1e-12);
        assert_eq!(wizard.pac_verified_lower, Some(0.0));
        assert_eq!(wizard.pac_candidate_upper, Some(1.0));
        assert_eq!(idle.pac_verified_lower, None);
    }

    #[test]
    fn message_volume_is_separate_from_message_effectiveness() {
        let metrics = fixture().score().unwrap();
        assert_eq!(metrics.message_action_count, 2);
        assert_eq!(metrics.verified_contributing_messages, 0);
        assert_eq!(metrics.candidate_inclusive_contributing_messages, 2);
        assert_eq!(metrics.message_effectiveness_verified_lower, Some(0.0));
        assert_eq!(metrics.message_effectiveness_candidate_upper, Some(1.0));
    }

    #[test]
    fn resource_lineage_and_cross_agent_dependencies_are_counted() {
        let metrics = fixture().score().unwrap();
        assert_eq!(metrics.resource_transfer_action_count, 1);
        assert_eq!(metrics.verified_cross_agent_causal_edges, 3);
        assert_eq!(metrics.candidate_inclusive_cross_agent_causal_edges, 5);
        assert_eq!(metrics.verified_cross_agent_contributing_actions, 4);
        assert_eq!(
            metrics.candidate_inclusive_cross_agent_contributing_actions,
            6
        );
    }

    #[test]
    fn duplicate_action_fingerprints_are_explicit() {
        let metrics = fixture().score().unwrap();
        assert_eq!(metrics.duplicate_action_count, 1);
        assert_eq!(metrics.duplicate_fingerprint_groups, 1);
    }

    #[test]
    fn failed_task_has_zero_cce_but_retains_partial_success() {
        let mut trace = fixture();
        trace.task_succeeded = false;
        trace.success_action_ids.clear();
        trace.partial_success_count = 2;
        trace.partial_success_total = 4;
        let metrics = trace.score().unwrap();
        assert_eq!(metrics.cce_verified_lower, 0.0);
        assert_eq!(metrics.cce_candidate_upper, 0.0);
        assert_eq!(metrics.partial_success_ratio, Some(0.5));
        assert_eq!(metrics.verified_contributing_actions, 0);
    }

    #[test]
    fn candidate_edge_changes_upper_bound_but_not_verified_lower_bound() {
        let mut trace = fixture();
        let before = trace.score().unwrap();
        trace
            .causal_edges
            .retain(|edge| edge.cause_action_id != "message");
        let after = trace.score().unwrap();
        assert_eq!(before.cce_verified_lower, after.cce_verified_lower);
        assert!(after.cce_candidate_upper < before.cce_candidate_upper);
    }

    #[test]
    fn model_proposed_edge_cannot_be_marked_verified() {
        let mut trace = fixture();
        trace.causal_edges[1].status = CausalEdgeStatus::Verified;
        assert!(matches!(
            trace.validate(),
            Err(CollaborationError::ModelProposalMarkedVerified { .. })
        ));
    }

    #[test]
    fn unknown_agents_and_edges_fail_closed() {
        let mut trace = fixture();
        trace.actions[0].agent_id = "unknown".into();
        assert!(matches!(
            trace.validate(),
            Err(CollaborationError::UnknownActionAgent { .. })
        ));

        let mut bad_edge = fixture();
        bad_edge.causal_edges[0].effect_action_id = "missing".into();
        assert!(matches!(
            bad_edge.validate(),
            Err(CollaborationError::UnknownCausalAction { .. })
        ));
    }

    #[test]
    fn backwards_edges_duplicate_edges_and_self_edges_fail_closed() {
        let mut backwards = fixture();
        backwards.causal_edges[0].cause_action_id = "deliver".into();
        assert!(matches!(
            backwards.validate(),
            Err(CollaborationError::NonForwardCausalEdge { .. })
        ));

        let mut duplicate = fixture();
        duplicate
            .causal_edges
            .push(duplicate.causal_edges[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(CollaborationError::DuplicateCausalEdge { .. })
        ));

        let mut self_edge = fixture();
        self_edge.causal_edges[0].effect_action_id = "ore".into();
        assert!(matches!(
            self_edge.validate(),
            Err(CollaborationError::SelfCausalEdge(_))
        ));
    }

    #[test]
    fn duplicate_global_sequences_and_decreasing_rounds_fail_closed() {
        let mut duplicate_sequence = fixture();
        duplicate_sequence.actions[1].sequence = duplicate_sequence.actions[0].sequence;
        assert!(matches!(
            duplicate_sequence.validate(),
            Err(CollaborationError::DuplicateSequence(_))
        ));

        let mut decreasing_round = fixture();
        decreasing_round.actions[2].round = 0;
        assert!(matches!(
            decreasing_round.validate(),
            Err(CollaborationError::NonMonotonicRound { .. })
        ));
    }

    #[test]
    fn agent_useful_effort_shares_sum_to_one_when_success_has_contributors() {
        let metrics = fixture().score().unwrap();
        let lower_sum = metrics
            .per_agent
            .iter()
            .filter_map(|agent| agent.useful_effort_share_lower)
            .sum::<f64>();
        let upper_sum = metrics
            .per_agent
            .iter()
            .filter_map(|agent| agent.useful_effort_share_upper)
            .sum::<f64>();
        assert!((lower_sum - 1.0).abs() < 1e-12);
        assert!((upper_sum - 1.0).abs() < 1e-12);
        assert!(metrics.cce_verified_lower <= metrics.cce_candidate_upper);
    }

    #[test]
    fn success_failure_and_partial_success_contract_is_explicit() {
        let mut inconsistent = fixture();
        inconsistent.success_action_ids.clear();
        assert_eq!(
            inconsistent.validate(),
            Err(CollaborationError::InvalidSuccessActionSet)
        );

        let mut invalid_partial = fixture();
        invalid_partial.partial_success_count = 5;
        invalid_partial.partial_success_total = 4;
        assert!(matches!(
            invalid_partial.validate(),
            Err(CollaborationError::PartialSuccessOutOfRange { .. })
        ));
    }
}
