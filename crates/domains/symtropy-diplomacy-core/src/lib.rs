// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Clause-level diplomacy without a global relationship scalar.
//!
//! Treaties are explicit bundles of obligations/rights/claims. Ratification,
//! notice, performance assertions, disputes, sanctions, and withdrawal remain
//! distinct records. This crate does not decide truth, legitimacy, payment,
//! military obedience, or whether a disputed clause was actually breached.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Generic diplomatic party identity. Interpretation belongs to higher layers.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DiplomaticParty {
    pub id: StableId,
}

/// Generic evidence reference without importing every possible producing domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DiplomaticEvidenceRef {
    pub namespace: String,
    pub record_id: StableId,
}

/// Exact scope for rights, sanctions, guarantees, or other clauses.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct DiplomaticScope {
    pub jurisdiction_id: Option<StableId>,
    pub target_id: Option<StableId>,
    pub capability_namespace: Option<String>,
    pub capability_operation: Option<String>,
}

/// Structured clause kinds. Unknown future forms can use `Custom`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreatyClauseKind {
    /// One party grants another an access/use/transit/basing/etc. right.
    AccessRight {
        grantor_id: StableId,
        grantee_id: StableId,
        scope: DiplomaticScope,
    },
    /// One party guarantees a beneficiary or protected subject under explicit scope.
    Guarantee {
        guarantor_id: StableId,
        beneficiary_id: StableId,
        protected_subject_id: StableId,
        scope: DiplomaticScope,
    },
    /// Material/service obligation expressed without executing economy mutation.
    PerformanceObligation {
        obligor_id: StableId,
        beneficiary_id: StableId,
        performance_namespace: String,
        performance_operation: String,
        subject_id: Option<StableId>,
        quantity: Option<u64>,
        unit_id: Option<StableId>,
        due_tick: Option<u64>,
    },
    /// Explicit debt acknowledgement. Payment/settlement belongs elsewhere.
    Debt {
        debtor_id: StableId,
        creditor_id: StableId,
        principal: u64,
        unit_id: StableId,
        due_tick: Option<u64>,
    },
    /// Formal restriction imposed by one party against another.
    Sanction {
        imposer_id: StableId,
        target_party_id: StableId,
        scope: DiplomaticScope,
    },
    /// One party asserts a claim to territory, office, asset, route, or other subject.
    Claim {
        claimant_id: StableId,
        subject_id: StableId,
        claim_namespace: String,
        basis: Vec<DiplomaticEvidenceRef>,
    },
    /// Extensible scenario/content clause.
    Custom {
        namespace: String,
        kind: String,
        subject_ids: Vec<StableId>,
    },
}

/// One immutable treaty clause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyClause {
    pub id: StableId,
    pub kind: TreatyClauseKind,
    pub valid_from_tick: u64,
    pub valid_until_tick: Option<u64>,
    pub source_event_id: StableId,
}

impl TreatyClause {
    pub fn is_active_at(&self, tick: u64) -> bool {
        tick >= self.valid_from_tick
            && self
                .valid_until_tick
                .is_none_or(|until| tick <= until)
    }
}

/// Immutable treaty text/specification. Proposal is not ratification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatySpec {
    pub id: StableId,
    pub title: String,
    pub parties: BTreeSet<StableId>,
    pub required_ratifiers: BTreeSet<StableId>,
    pub clauses: Vec<TreatyClause>,
    pub proposed_tick: u64,
    pub source_event_id: StableId,
}

impl TreatySpec {
    pub fn validate(&self) -> Result<(), DiplomacyError> {
        if self.parties.len() < 2 {
            return Err(DiplomacyError::TooFewTreatyParties(self.id.clone()));
        }
        if self.clauses.is_empty() {
            return Err(DiplomacyError::EmptyTreaty(self.id.clone()));
        }
        if !self.required_ratifiers.is_subset(&self.parties) {
            return Err(DiplomacyError::RatifierOutsideParties(self.id.clone()));
        }
        let mut clause_ids = BTreeSet::new();
        for clause in &self.clauses {
            if !clause_ids.insert(clause.id.clone()) {
                return Err(DiplomacyError::DuplicateClause(clause.id.clone()));
            }
            if clause
                .valid_until_tick
                .is_some_and(|until| until < clause.valid_from_tick)
            {
                return Err(DiplomacyError::InvalidClauseWindow(clause.id.clone()));
            }
        }
        Ok(())
    }
}

/// Immutable party ratification/approval record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyRatification {
    pub id: StableId,
    pub treaty_id: StableId,
    pub ratifier_id: StableId,
    pub ratified_tick: u64,
    pub source_event_id: StableId,
}

/// Explicit withdrawal/termination record. It does not delete treaty history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreatyTermination {
    pub id: StableId,
    pub treaty_id: StableId,
    pub terminating_party_id: StableId,
    pub effective_tick: u64,
    pub reason: String,
    pub source_event_id: StableId,
}

/// Formal diplomatic notice. A communication message id may be attached, but
/// notice authorship does not imply simulated delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiplomaticNotice {
    pub id: StableId,
    pub sender_id: StableId,
    pub recipient_id: StableId,
    pub notice_namespace: String,
    pub subject_id: Option<StableId>,
    pub authored_tick: u64,
    pub communication_message_id: Option<StableId>,
    pub source_event_id: StableId,
}

/// A party's position on whether one clause is being/was satisfied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClausePosition {
    Satisfied,
    Breached,
    Disputed,
    Waived,
    Unknown,
}

/// Immutable clause-performance assertion. Different parties may disagree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClauseAssessment {
    pub id: StableId,
    pub treaty_id: StableId,
    pub clause_id: StableId,
    pub assessor_id: StableId,
    pub position: ClausePosition,
    pub assessed_tick: u64,
    pub evidence: Vec<DiplomaticEvidenceRef>,
    pub source_event_id: StableId,
}

/// Derived treaty lifecycle state. No global faction relation score exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TreatyState {
    Proposed,
    Active,
    Terminated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DiplomacyWorld {
    treaties: BTreeMap<StableId, TreatySpec>,
    ratifications: BTreeMap<StableId, TreatyRatification>,
    ratifications_by_treaty: BTreeMap<StableId, BTreeSet<StableId>>,
    terminations: BTreeMap<StableId, TreatyTermination>,
    termination_by_treaty: BTreeMap<StableId, StableId>,
    notices: BTreeMap<StableId, DiplomaticNotice>,
    assessments: BTreeMap<StableId, ClauseAssessment>,
}

impl DiplomacyWorld {
    pub fn propose_treaty(&mut self, treaty: TreatySpec) -> Result<(), DiplomacyError> {
        if self.treaties.contains_key(&treaty.id) {
            return Err(DiplomacyError::DuplicateTreaty(treaty.id));
        }
        treaty.validate()?;
        self.treaties.insert(treaty.id.clone(), treaty);
        Ok(())
    }

    pub fn ratify(&mut self, ratification: TreatyRatification) -> Result<(), DiplomacyError> {
        if self.ratifications.contains_key(&ratification.id) {
            return Err(DiplomacyError::DuplicateRatification(ratification.id));
        }
        let treaty = self
            .treaties
            .get(&ratification.treaty_id)
            .ok_or_else(|| DiplomacyError::UnknownTreaty(ratification.treaty_id.clone()))?;
        if !treaty.parties.contains(&ratification.ratifier_id) {
            return Err(DiplomacyError::RatifierOutsideParties(
                ratification.treaty_id,
            ));
        }
        if ratification.ratified_tick < treaty.proposed_tick {
            return Err(DiplomacyError::RatifiedBeforeProposal {
                treaty_id: treaty.id.clone(),
                ratified_tick: ratification.ratified_tick,
                proposed_tick: treaty.proposed_tick,
            });
        }
        let ratifiers = self
            .ratifications_by_treaty
            .entry(treaty.id.clone())
            .or_default();
        if !ratifiers.insert(ratification.ratifier_id.clone()) {
            return Err(DiplomacyError::PartyAlreadyRatified {
                treaty_id: treaty.id.clone(),
                ratifier_id: ratification.ratifier_id,
            });
        }
        self.ratifications
            .insert(ratification.id.clone(), ratification);
        Ok(())
    }

    pub fn terminate(&mut self, termination: TreatyTermination) -> Result<(), DiplomacyError> {
        if self.terminations.contains_key(&termination.id) {
            return Err(DiplomacyError::DuplicateTermination(termination.id));
        }
        let treaty = self
            .treaties
            .get(&termination.treaty_id)
            .ok_or_else(|| DiplomacyError::UnknownTreaty(termination.treaty_id.clone()))?;
        if !treaty.parties.contains(&termination.terminating_party_id) {
            return Err(DiplomacyError::TerminatorOutsideParties(
                termination.terminating_party_id,
            ));
        }
        if self
            .termination_by_treaty
            .contains_key(&termination.treaty_id)
        {
            return Err(DiplomacyError::TreatyAlreadyTerminated(
                termination.treaty_id,
            ));
        }
        self.termination_by_treaty
            .insert(termination.treaty_id.clone(), termination.id.clone());
        self.terminations
            .insert(termination.id.clone(), termination);
        Ok(())
    }

    pub fn record_notice(&mut self, notice: DiplomaticNotice) -> Result<(), DiplomacyError> {
        if self.notices.contains_key(&notice.id) {
            return Err(DiplomacyError::DuplicateNotice(notice.id));
        }
        if notice.sender_id == notice.recipient_id {
            return Err(DiplomacyError::SameNoticeParty(notice.id));
        }
        self.notices.insert(notice.id.clone(), notice);
        Ok(())
    }

    pub fn assess_clause(&mut self, assessment: ClauseAssessment) -> Result<(), DiplomacyError> {
        if self.assessments.contains_key(&assessment.id) {
            return Err(DiplomacyError::DuplicateAssessment(assessment.id));
        }
        let treaty = self
            .treaties
            .get(&assessment.treaty_id)
            .ok_or_else(|| DiplomacyError::UnknownTreaty(assessment.treaty_id.clone()))?;
        if !treaty
            .clauses
            .iter()
            .any(|clause| clause.id == assessment.clause_id)
        {
            return Err(DiplomacyError::UnknownClause {
                treaty_id: assessment.treaty_id,
                clause_id: assessment.clause_id,
            });
        }
        if !treaty.parties.contains(&assessment.assessor_id) {
            return Err(DiplomacyError::AssessorOutsideParties(
                assessment.assessor_id,
            ));
        }
        self.assessments
            .insert(assessment.id.clone(), assessment);
        Ok(())
    }

    pub fn treaty_state(&self, treaty_id: &StableId) -> Option<TreatyState> {
        let treaty = self.treaties.get(treaty_id)?;
        if self.termination_by_treaty.contains_key(treaty_id) {
            return Some(TreatyState::Terminated);
        }
        let ratified = self
            .ratifications_by_treaty
            .get(treaty_id)
            .cloned()
            .unwrap_or_default();
        if treaty.required_ratifiers.is_subset(&ratified) {
            Some(TreatyState::Active)
        } else {
            Some(TreatyState::Proposed)
        }
    }

    pub fn assessments_for_clause<'a>(
        &'a self,
        treaty_id: &'a StableId,
        clause_id: &'a StableId,
    ) -> impl Iterator<Item = &'a ClauseAssessment> {
        self.assessments.values().filter(move |assessment| {
            assessment.treaty_id == *treaty_id && assessment.clause_id == *clause_id
        })
    }

    pub fn treaty(&self, treaty_id: &StableId) -> Option<&TreatySpec> {
        self.treaties.get(treaty_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiplomacyError {
    DuplicateTreaty(StableId),
    TooFewTreatyParties(StableId),
    EmptyTreaty(StableId),
    RatifierOutsideParties(StableId),
    DuplicateClause(StableId),
    InvalidClauseWindow(StableId),
    DuplicateRatification(StableId),
    UnknownTreaty(StableId),
    RatifiedBeforeProposal {
        treaty_id: StableId,
        ratified_tick: u64,
        proposed_tick: u64,
    },
    PartyAlreadyRatified {
        treaty_id: StableId,
        ratifier_id: StableId,
    },
    DuplicateTermination(StableId),
    TerminatorOutsideParties(StableId),
    TreatyAlreadyTerminated(StableId),
    DuplicateNotice(StableId),
    SameNoticeParty(StableId),
    DuplicateAssessment(StableId),
    UnknownClause {
        treaty_id: StableId,
        clause_id: StableId,
    },
    AssessorOutsideParties(StableId),
}

impl fmt::Display for DiplomacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateTreaty(id) => write!(f, "treaty {id} already exists"),
            Self::TooFewTreatyParties(id) => write!(f, "treaty {id} needs at least two parties"),
            Self::EmptyTreaty(id) => write!(f, "treaty {id} has no clauses"),
            Self::RatifierOutsideParties(id) => write!(f, "treaty {id} references a ratifier outside its parties"),
            Self::DuplicateClause(id) => write!(f, "treaty clause {id} is duplicated"),
            Self::InvalidClauseWindow(id) => write!(f, "treaty clause {id} has an invalid validity window"),
            Self::DuplicateRatification(id) => write!(f, "ratification {id} already exists"),
            Self::UnknownTreaty(id) => write!(f, "unknown treaty {id}"),
            Self::RatifiedBeforeProposal { treaty_id, .. } => write!(f, "treaty {treaty_id} was ratified before proposal"),
            Self::PartyAlreadyRatified { treaty_id, ratifier_id } => write!(f, "party {ratifier_id} already ratified treaty {treaty_id}"),
            Self::DuplicateTermination(id) => write!(f, "termination {id} already exists"),
            Self::TerminatorOutsideParties(id) => write!(f, "terminating party {id} is not a treaty party"),
            Self::TreatyAlreadyTerminated(id) => write!(f, "treaty {id} is already terminated"),
            Self::DuplicateNotice(id) => write!(f, "notice {id} already exists"),
            Self::SameNoticeParty(id) => write!(f, "notice {id} has the same sender and recipient"),
            Self::DuplicateAssessment(id) => write!(f, "clause assessment {id} already exists"),
            Self::UnknownClause { treaty_id, clause_id } => write!(f, "treaty {treaty_id} has no clause {clause_id}"),
            Self::AssessorOutsideParties(id) => write!(f, "assessor {id} is not a treaty party"),
        }
    }
}

impl Error for DiplomacyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id")
    }

    fn treaty() -> TreatySpec {
        TreatySpec {
            id: id("treaty:hub-access"),
            title: "Hub Access and Supply Compact".into(),
            parties: BTreeSet::from([id("polity:aster"), id("polity:vesper")]),
            required_ratifiers: BTreeSet::from([id("polity:aster"), id("polity:vesper")]),
            clauses: vec![
                TreatyClause {
                    id: id("clause:access"),
                    kind: TreatyClauseKind::AccessRight {
                        grantor_id: id("polity:vesper"),
                        grantee_id: id("polity:aster"),
                        scope: DiplomaticScope {
                            jurisdiction_id: Some(id("loc:hub")),
                            target_id: None,
                            capability_namespace: Some("transit".into()),
                            capability_operation: Some("dock".into()),
                        },
                    },
                    valid_from_tick: 100,
                    valid_until_tick: None,
                    source_event_id: id("event:access-clause"),
                },
                TreatyClause {
                    id: id("clause:food"),
                    kind: TreatyClauseKind::PerformanceObligation {
                        obligor_id: id("polity:aster"),
                        beneficiary_id: id("polity:vesper"),
                        performance_namespace: "resource-delivery".into(),
                        performance_operation: "deliver".into(),
                        subject_id: Some(id("resource:food")),
                        quantity: Some(12_000),
                        unit_id: Some(id("unit:kg")),
                        due_tick: Some(500),
                    },
                    valid_from_tick: 100,
                    valid_until_tick: None,
                    source_event_id: id("event:food-clause"),
                },
            ],
            proposed_tick: 90,
            source_event_id: id("event:treaty-proposal"),
        }
    }

    #[test]
    fn proposal_is_not_active_until_required_parties_ratify() {
        let mut world = DiplomacyWorld::default();
        world.propose_treaty(treaty()).expect("proposal");
        assert_eq!(world.treaty_state(&id("treaty:hub-access")), Some(TreatyState::Proposed));
        world
            .ratify(TreatyRatification {
                id: id("ratify:aster"),
                treaty_id: id("treaty:hub-access"),
                ratifier_id: id("polity:aster"),
                ratified_tick: 95,
                source_event_id: id("event:ratify-aster"),
            })
            .expect("ratify aster");
        assert_eq!(world.treaty_state(&id("treaty:hub-access")), Some(TreatyState::Proposed));
        world
            .ratify(TreatyRatification {
                id: id("ratify:vesper"),
                treaty_id: id("treaty:hub-access"),
                ratifier_id: id("polity:vesper"),
                ratified_tick: 96,
                source_event_id: id("event:ratify-vesper"),
            })
            .expect("ratify vesper");
        assert_eq!(world.treaty_state(&id("treaty:hub-access")), Some(TreatyState::Active));
    }

    #[test]
    fn parties_can_disagree_about_breach_without_collapsing_to_relation_score() {
        let mut world = DiplomacyWorld::default();
        world.propose_treaty(treaty()).expect("proposal");
        world
            .assess_clause(ClauseAssessment {
                id: id("assessment:aster"),
                treaty_id: id("treaty:hub-access"),
                clause_id: id("clause:food"),
                assessor_id: id("polity:aster"),
                position: ClausePosition::Satisfied,
                assessed_tick: 510,
                evidence: vec![DiplomaticEvidenceRef {
                    namespace: "economy-shipment-arrival".into(),
                    record_id: id("arrival:food"),
                }],
                source_event_id: id("event:assessment-aster"),
            })
            .expect("aster assessment");
        world
            .assess_clause(ClauseAssessment {
                id: id("assessment:vesper"),
                treaty_id: id("treaty:hub-access"),
                clause_id: id("clause:food"),
                assessor_id: id("polity:vesper"),
                position: ClausePosition::Breached,
                assessed_tick: 511,
                evidence: vec![DiplomaticEvidenceRef {
                    namespace: "inspection".into(),
                    record_id: id("inspection:spoiled"),
                }],
                source_event_id: id("event:assessment-vesper"),
            })
            .expect("vesper assessment");
        let positions: BTreeSet<_> = world
            .assessments_for_clause(&id("treaty:hub-access"), &id("clause:food"))
            .map(|assessment| assessment.position)
            .collect();
        assert!(positions.contains(&ClausePosition::Satisfied));
        assert!(positions.contains(&ClausePosition::Breached));
    }

    #[test]
    fn notice_message_reference_does_not_imply_delivery() {
        let mut world = DiplomacyWorld::default();
        world
            .record_notice(DiplomaticNotice {
                id: id("notice:sanctions"),
                sender_id: id("polity:aster"),
                recipient_id: id("polity:vesper"),
                notice_namespace: "sanction-warning".into(),
                subject_id: Some(id("treaty:hub-access")),
                authored_tick: 700,
                communication_message_id: Some(id("message:sanction-warning")),
                source_event_id: id("event:notice"),
            })
            .expect("notice");
        // Diplomacy records the authored notice only. COMMS separately owns whether
        // `message:sanction-warning` ever arrives.
        assert_eq!(world.notices.len(), 1);
    }
}
