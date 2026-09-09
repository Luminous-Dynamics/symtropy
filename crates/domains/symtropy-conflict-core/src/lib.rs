// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Civilization-scale conflict records without a global war score.
//!
//! This crate owns declared conflict participation, stated aims, mobilization
//! commitments, ceasefire proposals, negotiated settlement terms, demobilization,
//! and reconstruction obligations. It does not simulate combat, determine
//! battlefield truth, transfer territory/assets, create casualties, or execute
//! economic payments merely because a term was agreed.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Generic reference to evidence/state owned by another domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConflictEvidenceRef {
    pub namespace: String,
    pub record_id: StableId,
}

/// One declared participant and its side/alignment identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictParticipant {
    pub party_id: StableId,
    /// Multiple parties may share a side; side identity has no engine-privileged meaning.
    pub side_id: StableId,
    pub joined_tick: u64,
    pub source_event_id: StableId,
}

/// Desired political result. These are demands/aims, not already-achieved state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictAimKind {
    Recognition {
        recognizer_id: StableId,
        subject_id: StableId,
    },
    AccessRight {
        grantor_id: StableId,
        grantee_id: StableId,
        scope_id: StableId,
    },
    /// Request that another authority transfer/recognize control or title.
    /// Recording the aim does not perform the transfer.
    TransferClaim {
        from_party_id: StableId,
        to_party_id: StableId,
        subject_id: StableId,
        relation_namespace: String,
    },
    Reparations {
        payer_id: StableId,
        beneficiary_id: StableId,
        amount: u64,
        unit_id: StableId,
    },
    Demilitarization {
        obligated_party_id: StableId,
        scope_id: StableId,
        capability_namespace: String,
    },
    Independence {
        claimant_id: StableId,
        from_party_id: StableId,
        subject_id: StableId,
    },
    TreatyChange {
        treaty_id: StableId,
        clause_id: Option<StableId>,
        requested_change: String,
    },
    Custom {
        namespace: String,
        kind: String,
        subject_ids: Vec<StableId>,
    },
}

/// Immutable conflict aim asserted by one participant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictAim {
    pub id: StableId,
    pub asserted_by_party_id: StableId,
    pub kind: ConflictAimKind,
    pub asserted_tick: u64,
    pub evidence: Vec<ConflictEvidenceRef>,
    pub source_event_id: StableId,
}

/// Initial conflict declaration/specification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConflictSpec {
    pub id: StableId,
    pub title: String,
    pub participants: Vec<ConflictParticipant>,
    pub aims: Vec<ConflictAim>,
    pub declared_tick: u64,
    /// Optional diplomatic/claim/incident record that contextualizes the declaration.
    pub trigger_refs: Vec<ConflictEvidenceRef>,
    pub source_event_id: StableId,
}

impl ConflictSpec {
    pub fn validate(&self) -> Result<(), ConflictError> {
        if self.participants.len() < 2 {
            return Err(ConflictError::TooFewParticipants(self.id.clone()));
        }
        let mut parties = BTreeSet::new();
        let mut sides = BTreeSet::new();
        for participant in &self.participants {
            if !parties.insert(participant.party_id.clone()) {
                return Err(ConflictError::DuplicateParticipant(
                    participant.party_id.clone(),
                ));
            }
            if participant.joined_tick < self.declared_tick {
                return Err(ConflictError::ParticipantJoinedBeforeDeclaration {
                    conflict_id: self.id.clone(),
                    party_id: participant.party_id.clone(),
                });
            }
            sides.insert(participant.side_id.clone());
        }
        if sides.len() < 2 {
            return Err(ConflictError::TooFewSides(self.id.clone()));
        }
        let mut aim_ids = BTreeSet::new();
        for aim in &self.aims {
            if !aim_ids.insert(aim.id.clone()) {
                return Err(ConflictError::DuplicateAim(aim.id.clone()));
            }
            if !parties.contains(&aim.asserted_by_party_id) {
                return Err(ConflictError::AimByNonParticipant {
                    conflict_id: self.id.clone(),
                    party_id: aim.asserted_by_party_id.clone(),
                });
            }
            if aim.asserted_tick < self.declared_tick {
                return Err(ConflictError::AimBeforeDeclaration(aim.id.clone()));
            }
        }
        Ok(())
    }

    pub fn party_ids(&self) -> BTreeSet<StableId> {
        self.participants
            .iter()
            .map(|participant| participant.party_id.clone())
            .collect()
    }
}

/// A participant's explicit mobilization commitment.
///
/// Resource/formation identities remain owned by their producing domains; this
/// record does not prove availability, physical deployment, or obedience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobilizationCommitment {
    pub id: StableId,
    pub conflict_id: StableId,
    pub party_id: StableId,
    pub commitment_namespace: String,
    pub subject_id: StableId,
    pub amount: Option<u64>,
    pub unit_id: Option<StableId>,
    pub committed_tick: u64,
    pub evidence: Vec<ConflictEvidenceRef>,
    pub source_event_id: StableId,
}

/// Ceasefire terms are constraints agreed for a bounded period, not peace itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeasefireProposal {
    pub id: StableId,
    pub conflict_id: StableId,
    pub proposed_by_party_id: StableId,
    pub required_acceptors: BTreeSet<StableId>,
    pub effective_not_before_tick: u64,
    pub expires_tick: Option<u64>,
    pub terms: Vec<SettlementTerm>,
    pub proposed_tick: u64,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CeasefireAcceptance {
    pub id: StableId,
    pub proposal_id: StableId,
    pub party_id: StableId,
    pub accepted_tick: u64,
    pub source_event_id: StableId,
}

/// Negotiated outcome term. Every consequential term requires another authority
/// to execute/verify it; this crate records the agreement only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementTermKind {
    Recognize {
        recognizer_id: StableId,
        subject_id: StableId,
    },
    TransferRequested {
        from_party_id: StableId,
        to_party_id: StableId,
        subject_id: StableId,
        relation_namespace: String,
    },
    PaymentRequested {
        payer_id: StableId,
        beneficiary_id: StableId,
        amount: u64,
        unit_id: StableId,
    },
    AccessRightRequested {
        grantor_id: StableId,
        grantee_id: StableId,
        scope_id: StableId,
    },
    Demobilize {
        party_id: StableId,
        scope_id: Option<StableId>,
    },
    ReconstructionObligation {
        obligor_id: StableId,
        beneficiary_id: StableId,
        project_namespace: String,
        subject_id: StableId,
        quantity: Option<u64>,
        unit_id: Option<StableId>,
        due_tick: Option<u64>,
    },
    Custom {
        namespace: String,
        kind: String,
        subject_ids: Vec<StableId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementTerm {
    pub id: StableId,
    pub kind: SettlementTermKind,
    pub evidence: Vec<ConflictEvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementProposal {
    pub id: StableId,
    pub conflict_id: StableId,
    pub proposed_by_party_id: StableId,
    pub required_acceptors: BTreeSet<StableId>,
    pub terms: Vec<SettlementTerm>,
    pub proposed_tick: u64,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementAcceptance {
    pub id: StableId,
    pub proposal_id: StableId,
    pub party_id: StableId,
    pub accepted_tick: u64,
    pub source_event_id: StableId,
}

/// Explicit demobilization record following a ceasefire/settlement or unilateral decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DemobilizationRecord {
    pub id: StableId,
    pub conflict_id: StableId,
    pub party_id: StableId,
    pub subject_id: StableId,
    pub demobilized_tick: u64,
    pub basis_ref: Option<ConflictEvidenceRef>,
    pub source_event_id: StableId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictState {
    Active,
    Ceasefire,
    Settled,
}

/// Headless political-conflict state. Combat/world/economy authorities remain external.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ConflictWorld {
    conflicts: BTreeMap<StableId, ConflictSpec>,
    mobilizations: BTreeMap<StableId, MobilizationCommitment>,
    ceasefires: BTreeMap<StableId, CeasefireProposal>,
    ceasefire_acceptances: BTreeMap<StableId, CeasefireAcceptance>,
    ceasefire_acceptors: BTreeMap<StableId, BTreeSet<StableId>>,
    settlements: BTreeMap<StableId, SettlementProposal>,
    settlement_acceptances: BTreeMap<StableId, SettlementAcceptance>,
    settlement_acceptors: BTreeMap<StableId, BTreeSet<StableId>>,
    settled_by_conflict: BTreeMap<StableId, StableId>,
    demobilizations: BTreeMap<StableId, DemobilizationRecord>,
}

impl ConflictWorld {
    pub fn declare_conflict(&mut self, conflict: ConflictSpec) -> Result<(), ConflictError> {
        if self.conflicts.contains_key(&conflict.id) {
            return Err(ConflictError::DuplicateConflict(conflict.id));
        }
        conflict.validate()?;
        self.conflicts.insert(conflict.id.clone(), conflict);
        Ok(())
    }

    pub fn mobilize(&mut self, commitment: MobilizationCommitment) -> Result<(), ConflictError> {
        if self.mobilizations.contains_key(&commitment.id) {
            return Err(ConflictError::DuplicateMobilization(commitment.id));
        }
        let conflict = self
            .conflicts
            .get(&commitment.conflict_id)
            .ok_or_else(|| ConflictError::UnknownConflict(commitment.conflict_id.clone()))?;
        if !conflict.party_ids().contains(&commitment.party_id) {
            return Err(ConflictError::NonParticipant {
                conflict_id: conflict.id.clone(),
                party_id: commitment.party_id,
            });
        }
        if commitment.committed_tick < conflict.declared_tick {
            return Err(ConflictError::MobilizedBeforeDeclaration(commitment.id));
        }
        self.mobilizations.insert(commitment.id.clone(), commitment);
        Ok(())
    }

    pub fn propose_ceasefire(&mut self, proposal: CeasefireProposal) -> Result<(), ConflictError> {
        if self.ceasefires.contains_key(&proposal.id) {
            return Err(ConflictError::DuplicateCeasefire(proposal.id));
        }
        let conflict = self
            .conflicts
            .get(&proposal.conflict_id)
            .ok_or_else(|| ConflictError::UnknownConflict(proposal.conflict_id.clone()))?;
        validate_required_parties(
            conflict,
            &proposal.proposed_by_party_id,
            &proposal.required_acceptors,
        )?;
        if proposal.effective_not_before_tick < proposal.proposed_tick {
            return Err(ConflictError::CeasefireEffectiveBeforeProposal(proposal.id));
        }
        if proposal
            .expires_tick
            .is_some_and(|expiry| expiry < proposal.effective_not_before_tick)
        {
            return Err(ConflictError::InvalidCeasefireWindow(proposal.id));
        }
        validate_terms_unique(&proposal.terms)?;
        self.ceasefires.insert(proposal.id.clone(), proposal);
        Ok(())
    }

    pub fn accept_ceasefire(&mut self, acceptance: CeasefireAcceptance) -> Result<(), ConflictError> {
        if self.ceasefire_acceptances.contains_key(&acceptance.id) {
            return Err(ConflictError::DuplicateCeasefireAcceptance(acceptance.id));
        }
        let proposal = self
            .ceasefires
            .get(&acceptance.proposal_id)
            .ok_or_else(|| ConflictError::UnknownCeasefire(acceptance.proposal_id.clone()))?;
        if !proposal.required_acceptors.contains(&acceptance.party_id) {
            return Err(ConflictError::UnauthorizedAcceptance {
                proposal_id: proposal.id.clone(),
                party_id: acceptance.party_id,
            });
        }
        if acceptance.accepted_tick < proposal.proposed_tick {
            return Err(ConflictError::AcceptanceBeforeProposal(acceptance.id));
        }
        let acceptors = self
            .ceasefire_acceptors
            .entry(proposal.id.clone())
            .or_default();
        if !acceptors.insert(acceptance.party_id.clone()) {
            return Err(ConflictError::PartyAlreadyAccepted {
                proposal_id: proposal.id.clone(),
                party_id: acceptance.party_id,
            });
        }
        self.ceasefire_acceptances
            .insert(acceptance.id.clone(), acceptance);
        Ok(())
    }

    pub fn propose_settlement(&mut self, proposal: SettlementProposal) -> Result<(), ConflictError> {
        if self.settlements.contains_key(&proposal.id) {
            return Err(ConflictError::DuplicateSettlement(proposal.id));
        }
        let conflict = self
            .conflicts
            .get(&proposal.conflict_id)
            .ok_or_else(|| ConflictError::UnknownConflict(proposal.conflict_id.clone()))?;
        validate_required_parties(
            conflict,
            &proposal.proposed_by_party_id,
            &proposal.required_acceptors,
        )?;
        if proposal.terms.is_empty() {
            return Err(ConflictError::EmptySettlement(proposal.id));
        }
        validate_terms_unique(&proposal.terms)?;
        self.settlements.insert(proposal.id.clone(), proposal);
        Ok(())
    }

    pub fn accept_settlement(
        &mut self,
        acceptance: SettlementAcceptance,
    ) -> Result<(), ConflictError> {
        if self.settlement_acceptances.contains_key(&acceptance.id) {
            return Err(ConflictError::DuplicateSettlementAcceptance(acceptance.id));
        }
        let proposal = self
            .settlements
            .get(&acceptance.proposal_id)
            .ok_or_else(|| ConflictError::UnknownSettlement(acceptance.proposal_id.clone()))?;
        if self.settled_by_conflict.contains_key(&proposal.conflict_id) {
            return Err(ConflictError::ConflictAlreadySettled(
                proposal.conflict_id.clone(),
            ));
        }
        if !proposal.required_acceptors.contains(&acceptance.party_id) {
            return Err(ConflictError::UnauthorizedAcceptance {
                proposal_id: proposal.id.clone(),
                party_id: acceptance.party_id,
            });
        }
        if acceptance.accepted_tick < proposal.proposed_tick {
            return Err(ConflictError::AcceptanceBeforeProposal(acceptance.id));
        }
        let acceptors = self
            .settlement_acceptors
            .entry(proposal.id.clone())
            .or_default();
        if !acceptors.insert(acceptance.party_id.clone()) {
            return Err(ConflictError::PartyAlreadyAccepted {
                proposal_id: proposal.id.clone(),
                party_id: acceptance.party_id,
            });
        }
        if proposal.required_acceptors.is_subset(acceptors) {
            self.settled_by_conflict
                .insert(proposal.conflict_id.clone(), proposal.id.clone());
        }
        self.settlement_acceptances
            .insert(acceptance.id.clone(), acceptance);
        Ok(())
    }

    pub fn record_demobilization(
        &mut self,
        record: DemobilizationRecord,
    ) -> Result<(), ConflictError> {
        if self.demobilizations.contains_key(&record.id) {
            return Err(ConflictError::DuplicateDemobilization(record.id));
        }
        let conflict = self
            .conflicts
            .get(&record.conflict_id)
            .ok_or_else(|| ConflictError::UnknownConflict(record.conflict_id.clone()))?;
        if !conflict.party_ids().contains(&record.party_id) {
            return Err(ConflictError::NonParticipant {
                conflict_id: conflict.id.clone(),
                party_id: record.party_id,
            });
        }
        self.demobilizations.insert(record.id.clone(), record);
        Ok(())
    }

    pub fn conflict_state(&self, conflict_id: &StableId, tick: u64) -> Option<ConflictState> {
        self.conflicts.get(conflict_id)?;
        if self.settled_by_conflict.contains_key(conflict_id) {
            return Some(ConflictState::Settled);
        }
        let ceasefire_active = self.ceasefires.values().any(|proposal| {
            if proposal.conflict_id != *conflict_id || tick < proposal.effective_not_before_tick {
                return false;
            }
            if proposal.expires_tick.is_some_and(|expiry| tick > expiry) {
                return false;
            }
            let acceptors = self
                .ceasefire_acceptors
                .get(&proposal.id)
                .cloned()
                .unwrap_or_default();
            proposal.required_acceptors.is_subset(&acceptors)
        });
        if ceasefire_active {
            Some(ConflictState::Ceasefire)
        } else {
            Some(ConflictState::Active)
        }
    }

    pub fn settlement(&self, proposal_id: &StableId) -> Option<&SettlementProposal> {
        self.settlements.get(proposal_id)
    }

    /// Returns agreed terms once required parties have accepted. These terms are
    /// agreements only; callers must use domain-specific authorities to execute them.
    pub fn agreed_settlement_terms(
        &self,
        conflict_id: &StableId,
    ) -> Option<impl Iterator<Item = &SettlementTerm>> {
        let proposal_id = self.settled_by_conflict.get(conflict_id)?;
        let proposal = self.settlements.get(proposal_id)?;
        Some(proposal.terms.iter())
    }
}

fn validate_required_parties(
    conflict: &ConflictSpec,
    proposer: &StableId,
    required_acceptors: &BTreeSet<StableId>,
) -> Result<(), ConflictError> {
    let parties = conflict.party_ids();
    if !parties.contains(proposer) {
        return Err(ConflictError::NonParticipant {
            conflict_id: conflict.id.clone(),
            party_id: proposer.clone(),
        });
    }
    if required_acceptors.is_empty() || !required_acceptors.is_subset(&parties) {
        return Err(ConflictError::InvalidRequiredAcceptors(conflict.id.clone()));
    }
    Ok(())
}

fn validate_terms_unique(terms: &[SettlementTerm]) -> Result<(), ConflictError> {
    let mut ids = BTreeSet::new();
    for term in terms {
        if !ids.insert(term.id.clone()) {
            return Err(ConflictError::DuplicateSettlementTerm(term.id.clone()));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictError {
    DuplicateConflict(StableId),
    TooFewParticipants(StableId),
    TooFewSides(StableId),
    DuplicateParticipant(StableId),
    ParticipantJoinedBeforeDeclaration { conflict_id: StableId, party_id: StableId },
    DuplicateAim(StableId),
    AimByNonParticipant { conflict_id: StableId, party_id: StableId },
    AimBeforeDeclaration(StableId),
    UnknownConflict(StableId),
    NonParticipant { conflict_id: StableId, party_id: StableId },
    DuplicateMobilization(StableId),
    MobilizedBeforeDeclaration(StableId),
    DuplicateCeasefire(StableId),
    InvalidRequiredAcceptors(StableId),
    CeasefireEffectiveBeforeProposal(StableId),
    InvalidCeasefireWindow(StableId),
    DuplicateSettlementTerm(StableId),
    DuplicateCeasefireAcceptance(StableId),
    UnknownCeasefire(StableId),
    UnauthorizedAcceptance { proposal_id: StableId, party_id: StableId },
    AcceptanceBeforeProposal(StableId),
    PartyAlreadyAccepted { proposal_id: StableId, party_id: StableId },
    DuplicateSettlement(StableId),
    EmptySettlement(StableId),
    DuplicateSettlementAcceptance(StableId),
    UnknownSettlement(StableId),
    ConflictAlreadySettled(StableId),
    DuplicateDemobilization(StableId),
}

impl fmt::Display for ConflictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateConflict(id) => write!(f, "conflict {id} already exists"),
            Self::TooFewParticipants(id) => write!(f, "conflict {id} has fewer than two participants"),
            Self::TooFewSides(id) => write!(f, "conflict {id} has fewer than two sides"),
            Self::DuplicateParticipant(id) => write!(f, "participant {id} is duplicated"),
            Self::ParticipantJoinedBeforeDeclaration { conflict_id, party_id } => write!(f, "party {party_id} joins conflict {conflict_id} before declaration"),
            Self::DuplicateAim(id) => write!(f, "conflict aim {id} is duplicated"),
            Self::AimByNonParticipant { conflict_id, party_id } => write!(f, "non-participant {party_id} asserts an aim in conflict {conflict_id}"),
            Self::AimBeforeDeclaration(id) => write!(f, "conflict aim {id} predates the conflict declaration"),
            Self::UnknownConflict(id) => write!(f, "unknown conflict {id}"),
            Self::NonParticipant { conflict_id, party_id } => write!(f, "party {party_id} is not a participant in conflict {conflict_id}"),
            Self::DuplicateMobilization(id) => write!(f, "mobilization {id} already exists"),
            Self::MobilizedBeforeDeclaration(id) => write!(f, "mobilization {id} predates the conflict declaration"),
            Self::DuplicateCeasefire(id) => write!(f, "ceasefire proposal {id} already exists"),
            Self::InvalidRequiredAcceptors(id) => write!(f, "conflict {id} proposal has invalid required acceptors"),
            Self::CeasefireEffectiveBeforeProposal(id) => write!(f, "ceasefire {id} becomes effective before proposal"),
            Self::InvalidCeasefireWindow(id) => write!(f, "ceasefire {id} has an invalid window"),
            Self::DuplicateSettlementTerm(id) => write!(f, "settlement term {id} is duplicated"),
            Self::DuplicateCeasefireAcceptance(id) => write!(f, "ceasefire acceptance {id} already exists"),
            Self::UnknownCeasefire(id) => write!(f, "unknown ceasefire proposal {id}"),
            Self::UnauthorizedAcceptance { proposal_id, party_id } => write!(f, "party {party_id} may not accept proposal {proposal_id}"),
            Self::AcceptanceBeforeProposal(id) => write!(f, "acceptance {id} predates its proposal"),
            Self::PartyAlreadyAccepted { proposal_id, party_id } => write!(f, "party {party_id} already accepted proposal {proposal_id}"),
            Self::DuplicateSettlement(id) => write!(f, "settlement proposal {id} already exists"),
            Self::EmptySettlement(id) => write!(f, "settlement proposal {id} has no terms"),
            Self::DuplicateSettlementAcceptance(id) => write!(f, "settlement acceptance {id} already exists"),
            Self::UnknownSettlement(id) => write!(f, "unknown settlement proposal {id}"),
            Self::ConflictAlreadySettled(id) => write!(f, "conflict {id} already has an accepted settlement"),
            Self::DuplicateDemobilization(id) => write!(f, "demobilization {id} already exists"),
        }
    }
}

impl Error for ConflictError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn conflict() -> ConflictSpec {
        ConflictSpec {
            id: id("conflict:hub-crisis"),
            title: "Hub Crisis".into(),
            participants: vec![
                ConflictParticipant {
                    party_id: id("polity:aster"),
                    side_id: id("side:aster"),
                    joined_tick: 100,
                    source_event_id: id("event:aster-joins"),
                },
                ConflictParticipant {
                    party_id: id("polity:vesper"),
                    side_id: id("side:vesper"),
                    joined_tick: 100,
                    source_event_id: id("event:vesper-joins"),
                },
            ],
            aims: vec![ConflictAim {
                id: id("aim:hub-access"),
                asserted_by_party_id: id("polity:aster"),
                kind: ConflictAimKind::TreatyChange {
                    treaty_id: id("treaty:hub-access"),
                    clause_id: Some(id("clause:access")),
                    requested_change: "restore docking access".into(),
                },
                asserted_tick: 100,
                evidence: vec![ConflictEvidenceRef {
                    namespace: "diplomacy-clause-assessment".into(),
                    record_id: id("assessment:access-breach"),
                }],
                source_event_id: id("event:aim"),
            }],
            declared_tick: 100,
            trigger_refs: vec![ConflictEvidenceRef {
                namespace: "diplomatic-notice".into(),
                record_id: id("notice:access-suspended"),
            }],
            source_event_id: id("event:conflict-declared"),
        }
    }

    fn settlement_terms() -> Vec<SettlementTerm> {
        vec![
            SettlementTerm {
                id: id("term:access"),
                kind: SettlementTermKind::AccessRightRequested {
                    grantor_id: id("polity:vesper"),
                    grantee_id: id("polity:aster"),
                    scope_id: id("loc:hub"),
                },
                evidence: Vec::new(),
            },
            SettlementTerm {
                id: id("term:rebuild"),
                kind: SettlementTermKind::ReconstructionObligation {
                    obligor_id: id("polity:aster"),
                    beneficiary_id: id("polity:vesper"),
                    project_namespace: "station-repair".into(),
                    subject_id: id("station:hub-east"),
                    quantity: Some(500),
                    unit_id: Some(id("unit:work-package")),
                    due_tick: Some(900),
                },
                evidence: Vec::new(),
            },
        ]
    }

    #[test]
    fn ceasefire_requires_all_declared_acceptors() {
        let mut world = ConflictWorld::default();
        world.declare_conflict(conflict()).expect("conflict");
        world
            .propose_ceasefire(CeasefireProposal {
                id: id("ceasefire:1"),
                conflict_id: id("conflict:hub-crisis"),
                proposed_by_party_id: id("polity:aster"),
                required_acceptors: BTreeSet::from([id("polity:aster"), id("polity:vesper")]),
                effective_not_before_tick: 210,
                expires_tick: Some(400),
                terms: Vec::new(),
                proposed_tick: 200,
                source_event_id: id("event:ceasefire"),
            })
            .expect("proposal");
        world
            .accept_ceasefire(CeasefireAcceptance {
                id: id("accept:aster"),
                proposal_id: id("ceasefire:1"),
                party_id: id("polity:aster"),
                accepted_tick: 201,
                source_event_id: id("event:accept-aster"),
            })
            .expect("aster accepts");
        assert_eq!(
            world.conflict_state(&id("conflict:hub-crisis"), 220),
            Some(ConflictState::Active)
        );
        world
            .accept_ceasefire(CeasefireAcceptance {
                id: id("accept:vesper"),
                proposal_id: id("ceasefire:1"),
                party_id: id("polity:vesper"),
                accepted_tick: 205,
                source_event_id: id("event:accept-vesper"),
            })
            .expect("vesper accepts");
        assert_eq!(
            world.conflict_state(&id("conflict:hub-crisis"), 220),
            Some(ConflictState::Ceasefire)
        );
    }

    #[test]
    fn accepted_settlement_ends_conflict_but_does_not_execute_terms() {
        let mut world = ConflictWorld::default();
        world.declare_conflict(conflict()).expect("conflict");
        world
            .propose_settlement(SettlementProposal {
                id: id("settlement:hub"),
                conflict_id: id("conflict:hub-crisis"),
                proposed_by_party_id: id("polity:vesper"),
                required_acceptors: BTreeSet::from([id("polity:aster"), id("polity:vesper")]),
                terms: settlement_terms(),
                proposed_tick: 500,
                source_event_id: id("event:settlement"),
            })
            .expect("settlement proposal");
        for (party, suffix, tick) in [
            ("polity:aster", "aster", 501),
            ("polity:vesper", "vesper", 502),
        ] {
            world
                .accept_settlement(SettlementAcceptance {
                    id: id(&format!("settlement-accept:{suffix}")),
                    proposal_id: id("settlement:hub"),
                    party_id: id(party),
                    accepted_tick: tick,
                    source_event_id: id(&format!("event:settlement-accept:{suffix}")),
                })
                .expect("accept settlement");
        }
        assert_eq!(
            world.conflict_state(&id("conflict:hub-crisis"), 600),
            Some(ConflictState::Settled)
        );
        let terms: Vec<_> = world
            .agreed_settlement_terms(&id("conflict:hub-crisis"))
            .expect("agreed terms")
            .collect();
        assert_eq!(terms.len(), 2);
        // The agreement contains an access request and reconstruction obligation;
        // this crate exposes no API that mutates an asset, treaty, inventory, or project.
    }

    #[test]
    fn mobilization_is_commitment_not_proof_of_deployment() {
        let mut world = ConflictWorld::default();
        world.declare_conflict(conflict()).expect("conflict");
        world
            .mobilize(MobilizationCommitment {
                id: id("mobilization:relief-fleet"),
                conflict_id: id("conflict:hub-crisis"),
                party_id: id("polity:aster"),
                commitment_namespace: "formation".into(),
                subject_id: id("formation:relief-fleet"),
                amount: None,
                unit_id: None,
                committed_tick: 110,
                evidence: vec![ConflictEvidenceRef {
                    namespace: "institution-order".into(),
                    record_id: id("order:mobilize-relief-fleet"),
                }],
                source_event_id: id("event:mobilization"),
            })
            .expect("mobilization");
        assert_eq!(world.conflict_state(&id("conflict:hub-crisis"), 120), Some(ConflictState::Active));
    }
}
