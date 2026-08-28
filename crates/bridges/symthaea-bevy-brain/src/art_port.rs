// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed artistic port for Bevy-hosted Symthaea brains.
//!
//! This is deliberately proposal-first. It does not mutate Bevy scenes, spawn
//! meshes, move cameras, or paint pixels. It gives an entity carrying
//! `CognitiveBrain` a typed place to receive artistic scene observations and
//! emit revision-bound proposals without collapsing artistic semantics into the
//! generic motor vector.
//!
//! The vocabulary mirrors `symthaea-art-world` schema v1 in the private/full
//! Symthaea workspace. The standalone Symtropy repository keeps this small
//! compatibility surface local until the real crate is publishable/re-wired.

use bevy::prelude::*;

pub const ART_WORLD_SCHEMA_V1: &str = "symthaea.art-world.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtAuthorityMode {
    Observe,
    Propose,
    Author,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtOperation {
    ImportArtifact,
    CreateForm,
    TransformForm,
    RemoveForm,
    JoinForms,
    SeparateForms,
    ApplyMaterial,
    AlterSurface,
    PlaceLight,
    MoveCamera,
    CreateStroke,
    EraseStroke,
    Deform,
    Repeat,
    InterruptPattern,
    Reveal,
    Occlude,
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Reflect)]
pub enum ArtParameterValue {
    Float(f64),
    Integer(i64),
    Bool(bool),
    Text(String),
    Vec2([f64; 2]),
    Vec3([f64; 3]),
    ColorRgba([f32; 4]),
}

/// Read-only semantic observation of one exact committed Bevy art-world state.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct ArtPerceptionFrame {
    pub schema: String,
    pub world_id: String,
    pub revision_id: String,
    pub revision_sequence: u64,
    pub content_hash: String,
    pub selected_entities: Vec<String>,
    pub scene_summary: Vec<String>,
    pub render_digest: Option<String>,
}

impl ArtPerceptionFrame {
    pub fn new(
        world_id: impl Into<String>,
        revision_id: impl Into<String>,
        revision_sequence: u64,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            schema: ART_WORLD_SCHEMA_V1.to_string(),
            world_id: world_id.into(),
            revision_id: revision_id.into(),
            revision_sequence,
            content_hash: content_hash.into(),
            selected_entities: Vec::new(),
            scene_summary: Vec::new(),
            render_digest: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum ArtProposalState {
    Proposed,
    Accepted,
    Rejected,
    Expired,
    Applied,
}

/// Semantic artistic intervention conceived against one exact scene revision.
#[derive(Debug, Clone, PartialEq, Reflect)]
pub struct ArtActionProposal {
    pub proposal_id: String,
    pub action_id: String,
    pub parent_revision: String,
    pub operation: ArtOperation,
    pub targets: Vec<String>,
    pub parameters: Vec<(String, ArtParameterValue)>,
    pub rationale: Option<String>,
    pub predicted_consequences: Vec<String>,
    pub state: ArtProposalState,
    pub decision_actor: Option<String>,
    pub decision_reason: Option<String>,
}

impl ArtActionProposal {
    pub fn new(
        proposal_id: impl Into<String>,
        action_id: impl Into<String>,
        parent_revision: impl Into<String>,
        operation: ArtOperation,
    ) -> Self {
        Self {
            proposal_id: proposal_id.into(),
            action_id: action_id.into(),
            parent_revision: parent_revision.into(),
            operation,
            targets: Vec::new(),
            parameters: Vec::new(),
            rationale: None,
            predicted_consequences: Vec::new(),
            state: ArtProposalState::Proposed,
            decision_actor: None,
            decision_reason: None,
        }
    }

    pub fn abstain(
        proposal_id: impl Into<String>,
        action_id: impl Into<String>,
        parent_revision: impl Into<String>,
    ) -> Self {
        Self::new(
            proposal_id,
            action_id,
            parent_revision,
            ArtOperation::Abstain,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub enum ArtPortEventKind {
    Observed,
    ProposalCreated,
    ProposalAccepted,
    ProposalRejected,
    ProposalExpired,
    ProposalApplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Reflect)]
pub struct ArtPortEvent {
    pub sequence: u64,
    pub kind: ArtPortEventKind,
    pub revision_id: String,
    pub proposal_id: Option<String>,
    pub actor: Option<String>,
}

/// Attach beside `CognitiveBrain` on an entity that participates in an art studio.
///
/// `ArtPort` is intentionally inert with respect to scene mutation. A later host
/// adapter drains accepted proposals and turns them into Bevy-native reversible
/// branches/commands. Keeping that responsibility separate makes the authority
/// boundary visible and testable.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct ArtPort {
    pub authority: ArtAuthorityMode,
    pub perception: Option<ArtPerceptionFrame>,
    pub proposals: Vec<ArtActionProposal>,
    pub events: Vec<ArtPortEvent>,
    next_event_sequence: u64,
}

impl ArtPort {
    pub fn new(authority: ArtAuthorityMode) -> Self {
        Self {
            authority,
            perception: None,
            proposals: Vec::new(),
            events: Vec::new(),
            next_event_sequence: 0,
        }
    }

    pub fn observe(&mut self, frame: ArtPerceptionFrame) -> Result<(), ArtPortError> {
        if frame.schema != ART_WORLD_SCHEMA_V1 {
            return Err(ArtPortError::SchemaMismatch(frame.schema));
        }
        let revision = frame.revision_id.clone();
        self.perception = Some(frame);
        self.push_event(ArtPortEventKind::Observed, revision, None, None);
        Ok(())
    }

    pub fn propose(&mut self, proposal: ArtActionProposal) -> Result<(), ArtPortError> {
        if self.authority == ArtAuthorityMode::Observe {
            return Err(ArtPortError::ProposalNotPermitted);
        }
        if proposal.state != ArtProposalState::Proposed {
            return Err(ArtPortError::InvalidProposalState);
        }
        let current = self
            .perception
            .as_ref()
            .ok_or(ArtPortError::NoPerceptionFrame)?;
        if proposal.parent_revision != current.revision_id {
            return Err(ArtPortError::StaleRevision {
                proposal: proposal.parent_revision,
                current: current.revision_id.clone(),
            });
        }
        if self
            .proposals
            .iter()
            .any(|existing| existing.proposal_id == proposal.proposal_id)
        {
            return Err(ArtPortError::DuplicateProposal(proposal.proposal_id));
        }
        let proposal_id = proposal.proposal_id.clone();
        let revision = proposal.parent_revision.clone();
        self.proposals.push(proposal);
        self.push_event(
            ArtPortEventKind::ProposalCreated,
            revision,
            Some(proposal_id),
            None,
        );
        Ok(())
    }

    pub fn accept(
        &mut self,
        proposal_id: &str,
        actor: impl Into<String>,
    ) -> Result<(), ArtPortError> {
        let actor = actor.into();
        let (revision, id) = {
            let proposal = self.find_proposed_mut(proposal_id)?;
            proposal.state = ArtProposalState::Accepted;
            proposal.decision_actor = Some(actor.clone());
            (proposal.parent_revision.clone(), proposal.proposal_id.clone())
        };
        self.push_event(
            ArtPortEventKind::ProposalAccepted,
            revision,
            Some(id),
            Some(actor),
        );
        Ok(())
    }

    pub fn reject(
        &mut self,
        proposal_id: &str,
        actor: impl Into<String>,
        reason: impl Into<String>,
    ) -> Result<(), ArtPortError> {
        let actor = actor.into();
        let reason = reason.into();
        let (revision, id) = {
            let proposal = self.find_proposed_mut(proposal_id)?;
            proposal.state = ArtProposalState::Rejected;
            proposal.decision_actor = Some(actor.clone());
            proposal.decision_reason = Some(reason);
            (proposal.parent_revision.clone(), proposal.proposal_id.clone())
        };
        self.push_event(
            ArtPortEventKind::ProposalRejected,
            revision,
            Some(id),
            Some(actor),
        );
        Ok(())
    }

    /// Accepted proposals waiting for a host adapter to apply them.
    pub fn accepted(&self) -> impl Iterator<Item = &ArtActionProposal> {
        self.proposals
            .iter()
            .filter(|proposal| proposal.state == ArtProposalState::Accepted)
    }

    /// Mark an accepted proposal applied after the host has produced a new
    /// committed scene revision. The host adapter, not this port, owns the
    /// actual mutation.
    pub fn mark_applied(&mut self, proposal_id: &str) -> Result<(), ArtPortError> {
        let (revision, id) = {
            let proposal = self
                .proposals
                .iter_mut()
                .find(|proposal| proposal.proposal_id == proposal_id)
                .ok_or_else(|| ArtPortError::UnknownProposal(proposal_id.to_string()))?;
            if proposal.state != ArtProposalState::Accepted {
                return Err(ArtPortError::InvalidProposalState);
            }
            proposal.state = ArtProposalState::Applied;
            (proposal.parent_revision.clone(), proposal.proposal_id.clone())
        };
        self.push_event(
            ArtPortEventKind::ProposalApplied,
            revision,
            Some(id),
            None,
        );
        Ok(())
    }

    /// `Author` enables an adapter to use a separately configured autonomous
    /// commit policy. `ArtPort` itself still never mutates the scene.
    pub fn allows_autonomous_commit(&self) -> bool {
        self.authority == ArtAuthorityMode::Author
    }

    fn find_proposed_mut(
        &mut self,
        proposal_id: &str,
    ) -> Result<&mut ArtActionProposal, ArtPortError> {
        let proposal = self
            .proposals
            .iter_mut()
            .find(|proposal| proposal.proposal_id == proposal_id)
            .ok_or_else(|| ArtPortError::UnknownProposal(proposal_id.to_string()))?;
        if proposal.state != ArtProposalState::Proposed {
            return Err(ArtPortError::InvalidProposalState);
        }
        Ok(proposal)
    }

    fn push_event(
        &mut self,
        kind: ArtPortEventKind,
        revision_id: String,
        proposal_id: Option<String>,
        actor: Option<String>,
    ) {
        let sequence = self.next_event_sequence;
        self.next_event_sequence = self.next_event_sequence.saturating_add(1);
        self.events.push(ArtPortEvent {
            sequence,
            kind,
            revision_id,
            proposal_id,
            actor,
        });
    }
}

impl Default for ArtPort {
    fn default() -> Self {
        Self::new(ArtAuthorityMode::Observe)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtPortError {
    SchemaMismatch(String),
    ProposalNotPermitted,
    NoPerceptionFrame,
    StaleRevision { proposal: String, current: String },
    DuplicateProposal(String),
    UnknownProposal(String),
    InvalidProposalState,
}

impl std::fmt::Display for ArtPortError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SchemaMismatch(schema) => write!(f, "unsupported art-world schema: {schema}"),
            Self::ProposalNotPermitted => write!(f, "observe-only art port cannot propose"),
            Self::NoPerceptionFrame => write!(f, "no art perception frame has been observed"),
            Self::StaleRevision { proposal, current } => write!(
                f,
                "proposal targets stale revision {proposal}; current revision is {current}"
            ),
            Self::DuplicateProposal(id) => write!(f, "duplicate proposal id: {id}"),
            Self::UnknownProposal(id) => write!(f, "unknown proposal id: {id}"),
            Self::InvalidProposalState => write!(f, "invalid proposal state transition"),
        }
    }
}

impl std::error::Error for ArtPortError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(revision: &str) -> ArtPerceptionFrame {
        ArtPerceptionFrame::new("studio", revision, 1, format!("hash-{revision}"))
    }

    #[test]
    fn observe_mode_is_read_only() {
        let mut port = ArtPort::new(ArtAuthorityMode::Observe);
        port.observe(frame("r1")).unwrap();
        let proposal = ArtActionProposal::new("p1", "a1", "r1", ArtOperation::Deform);
        assert_eq!(port.propose(proposal), Err(ArtPortError::ProposalNotPermitted));
    }

    #[test]
    fn proposal_is_revision_bound() {
        let mut port = ArtPort::new(ArtAuthorityMode::Propose);
        port.observe(frame("r2")).unwrap();
        let proposal = ArtActionProposal::new("p1", "a1", "r1", ArtOperation::Deform);
        assert_eq!(
            port.propose(proposal),
            Err(ArtPortError::StaleRevision {
                proposal: "r1".into(),
                current: "r2".into(),
            })
        );
    }

    #[test]
    fn accepted_proposal_remains_separate_from_scene_mutation() {
        let mut port = ArtPort::new(ArtAuthorityMode::Propose);
        port.observe(frame("r1")).unwrap();
        port.propose(ArtActionProposal::new(
            "p1",
            "a1",
            "r1",
            ArtOperation::MoveCamera,
        ))
        .unwrap();
        port.accept("p1", "collaborator").unwrap();
        assert_eq!(port.accepted().count(), 1);
        assert_eq!(port.perception.as_ref().unwrap().revision_id, "r1");
    }

    #[test]
    fn abstention_is_representable() {
        let proposal = ArtActionProposal::abstain("p1", "a1", "r1");
        assert_eq!(proposal.operation, ArtOperation::Abstain);
    }

    #[test]
    fn author_mode_is_explicit() {
        assert!(ArtPort::new(ArtAuthorityMode::Author).allows_autonomous_commit());
        assert!(!ArtPort::new(ArtAuthorityMode::Propose).allows_autonomous_commit());
    }
}
