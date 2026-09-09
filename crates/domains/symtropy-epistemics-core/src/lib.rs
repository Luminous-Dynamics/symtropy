// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit epistemic surfaces for civilization-scale simulation.
//!
//! This crate deliberately does **not** own world truth. It records what was
//! observed, what an actor currently believes, what an actor publicly asserted,
//! and what an institution currently records. Those states may disagree.
//!
//! No API in this crate promotes observation -> belief -> assertion ->
//! institutional record implicitly. Every transition is an explicit caller
//! action with inspectable basis references and causal-history provenance.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Inclusive confidence ceiling used by observation/belief records.
pub const MAX_CONFIDENCE: u16 = 10_000;

/// Stable proposition registered once and referenced by all epistemic surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpistemicStatement {
    /// Stable proposition identity.
    pub id: StableId,
    /// Thing, actor, institution, place, event, or other entity the proposition concerns.
    pub subject_id: StableId,
    /// Canonical proposition text for V0.
    ///
    /// This is data, not a world-truth claim. A later typed proposition schema may
    /// replace the V0 text surface without collapsing the authority classes below.
    pub proposition: String,
}

/// Typed reference to one existing epistemic record used as explicit basis.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EpistemicRef {
    /// Observer evidence.
    Observation(StableId),
    /// Historical or current belief record.
    Belief(StableId),
    /// Public assertion.
    Assertion(StableId),
    /// Institutional record.
    InstitutionalRecord(StableId),
}

/// One observer's evidence about a proposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationRecord {
    /// Stable observation identity.
    pub id: StableId,
    /// Person, sensor, document-ingestion process, instrument, or other source.
    pub observer_id: StableId,
    /// Registered proposition being observed/reported.
    pub statement_id: StableId,
    /// Confidence from 0 to 10,000.
    pub confidence: u16,
    /// Canonical tick at which this evidence was observed/received.
    pub observed_tick: u64,
    /// Optional tick after which the evidence should be treated as stale.
    pub stale_after_tick: Option<u64>,
    /// Causal-history event that introduced this observation.
    pub source_event_id: StableId,
}

impl ObservationRecord {
    /// Returns whether this observation is stale at `tick`.
    pub fn is_stale(&self, tick: u64) -> bool {
        self.stale_after_tick.is_some_and(|expiry| tick > expiry)
    }
}

/// One actor's explicit current-or-historical belief about a proposition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BeliefRecord {
    /// Stable belief-record identity. Historical belief records are never rewritten.
    pub id: StableId,
    /// Actor, institution, AI, or other epistemic holder.
    pub holder_id: StableId,
    /// Registered proposition believed.
    pub statement_id: StableId,
    /// Holder confidence from 0 to 10,000.
    pub confidence: u16,
    /// Canonical tick at which this belief record became current for the holder/proposition.
    pub recorded_tick: u64,
    /// Optional staleness horizon for the belief.
    pub stale_after_tick: Option<u64>,
    /// Explicit evidence/claims/records the holder used as basis.
    pub basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that recorded the belief transition.
    pub source_event_id: StableId,
}

impl BeliefRecord {
    /// Returns whether this belief is stale at `tick`.
    pub fn is_stale(&self, tick: u64) -> bool {
        self.stale_after_tick.is_some_and(|expiry| tick > expiry)
    }
}

/// One public assertion by a speaker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssertionRecord {
    /// Stable assertion identity.
    pub id: StableId,
    /// Speaker/issuing actor or institution.
    pub speaker_id: StableId,
    /// Registered proposition publicly asserted.
    pub statement_id: StableId,
    /// Optional confidence explicitly declared by the speaker.
    ///
    /// This is not inferred from the speaker's private belief.
    pub declared_confidence: Option<u16>,
    /// Canonical tick at which the assertion was made public.
    pub asserted_tick: u64,
    /// Explicit cited basis, if any.
    pub basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that records publication.
    pub source_event_id: StableId,
}

/// Current-state semantics of an institution's recorded proposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InstitutionalRecordStatus {
    /// Institution currently records the proposition as its active record for the key.
    Active,
    /// Institution explicitly withdrew/voided the prior active proposition for the key.
    Withdrawn,
}

/// One immutable institutional record revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstitutionalRecord {
    /// Stable immutable record-revision identity.
    pub id: StableId,
    /// Institution maintaining the record.
    pub institution_id: StableId,
    /// Institution-local stable logical key such as `office:chancellor` or
    /// `incident:station-44:cause`.
    pub record_key: String,
    /// Proposition recorded by the institution.
    pub statement_id: StableId,
    /// Current-state semantics of this revision.
    pub status: InstitutionalRecordStatus,
    /// Canonical tick at which this revision became the current institutional record.
    pub recorded_tick: u64,
    /// Explicit basis used by the institution, if any.
    pub basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that records this revision.
    pub source_event_id: StableId,
}

/// Dependency-light epistemic registry and current-state projections.
///
/// Immutable observations, beliefs, assertions, and institutional record revisions
/// remain inspectable. Separate indexes identify the current belief and current
/// institutional record without deleting history.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct EpistemicLedger {
    statements: BTreeMap<StableId, EpistemicStatement>,
    observations: BTreeMap<StableId, ObservationRecord>,
    beliefs: BTreeMap<StableId, BeliefRecord>,
    current_beliefs: BTreeMap<(StableId, StableId), StableId>,
    assertions: BTreeMap<StableId, AssertionRecord>,
    institutional_records: BTreeMap<StableId, InstitutionalRecord>,
    current_institutional_records: BTreeMap<(StableId, String), StableId>,
}

impl EpistemicLedger {
    /// Creates an empty ledger. It has no world-truth storage by design.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers one proposition exactly once.
    pub fn register_statement(
        &mut self,
        statement: EpistemicStatement,
    ) -> Result<(), EpistemicError> {
        if statement.proposition.trim().is_empty() {
            return Err(EpistemicError::EmptyStatement(statement.id));
        }
        if self.statements.contains_key(&statement.id) {
            return Err(EpistemicError::DuplicateStatement(statement.id));
        }
        self.statements.insert(statement.id.clone(), statement);
        Ok(())
    }

    /// Returns a registered proposition.
    pub fn statement(&self, id: &StableId) -> Option<&EpistemicStatement> {
        self.statements.get(id)
    }

    /// Records immutable observer evidence.
    pub fn record_observation(
        &mut self,
        observation: ObservationRecord,
    ) -> Result<(), EpistemicError> {
        self.require_statement(&observation.statement_id)?;
        validate_confidence(observation.confidence)?;
        validate_stale_window(
            observation.observed_tick,
            observation.stale_after_tick,
            &observation.id,
        )?;
        if self.observations.contains_key(&observation.id) {
            return Err(EpistemicError::DuplicateObservation(observation.id));
        }
        self.observations.insert(observation.id.clone(), observation);
        Ok(())
    }

    /// Returns one immutable observation.
    pub fn observation(&self, id: &StableId) -> Option<&ObservationRecord> {
        self.observations.get(id)
    }

    /// Explicitly records one belief revision and makes it current for the exact
    /// `(holder, proposition)` pair.
    ///
    /// Recording an observation never calls this automatically.
    pub fn record_belief(&mut self, belief: BeliefRecord) -> Result<(), EpistemicError> {
        self.require_statement(&belief.statement_id)?;
        validate_confidence(belief.confidence)?;
        validate_stale_window(belief.recorded_tick, belief.stale_after_tick, &belief.id)?;
        if self.beliefs.contains_key(&belief.id) {
            return Err(EpistemicError::DuplicateBelief(belief.id));
        }
        let self_ref = EpistemicRef::Belief(belief.id.clone());
        self.validate_basis(&belief.basis, Some(&self_ref))?;

        let projection_key = (belief.holder_id.clone(), belief.statement_id.clone());
        if let Some(existing_id) = self.current_beliefs.get(&projection_key) {
            let existing = self
                .beliefs
                .get(existing_id)
                .expect("current belief index references retained belief");
            if belief.recorded_tick < existing.recorded_tick {
                return Err(EpistemicError::StaleBeliefProjection {
                    holder_id: belief.holder_id,
                    statement_id: belief.statement_id,
                    existing_tick: existing.recorded_tick,
                    attempted_tick: belief.recorded_tick,
                });
            }
        }

        let belief_id = belief.id.clone();
        self.beliefs.insert(belief_id.clone(), belief);
        self.current_beliefs.insert(projection_key, belief_id);
        Ok(())
    }

    /// Returns one immutable historical/current belief revision.
    pub fn belief(&self, id: &StableId) -> Option<&BeliefRecord> {
        self.beliefs.get(id)
    }

    /// Returns the current belief for one exact holder/proposition pair.
    pub fn current_belief(
        &self,
        holder_id: &StableId,
        statement_id: &StableId,
    ) -> Option<&BeliefRecord> {
        let key = (holder_id.clone(), statement_id.clone());
        self.current_beliefs
            .get(&key)
            .and_then(|id| self.beliefs.get(id))
    }

    /// Records one immutable public assertion.
    ///
    /// The asserted proposition may differ from the speaker's private beliefs;
    /// this API intentionally performs no implicit belief equivalence check.
    pub fn publish_assertion(
        &mut self,
        assertion: AssertionRecord,
    ) -> Result<(), EpistemicError> {
        self.require_statement(&assertion.statement_id)?;
        if let Some(confidence) = assertion.declared_confidence {
            validate_confidence(confidence)?;
        }
        if self.assertions.contains_key(&assertion.id) {
            return Err(EpistemicError::DuplicateAssertion(assertion.id));
        }
        let self_ref = EpistemicRef::Assertion(assertion.id.clone());
        self.validate_basis(&assertion.basis, Some(&self_ref))?;
        self.assertions.insert(assertion.id.clone(), assertion);
        Ok(())
    }

    /// Returns one immutable public assertion.
    pub fn assertion(&self, id: &StableId) -> Option<&AssertionRecord> {
        self.assertions.get(id)
    }

    /// Records one immutable institutional record revision and makes it current
    /// for the exact `(institution, record_key)` pair.
    ///
    /// Institutional record is not treated as institutional belief, public truth,
    /// legal authority, or effective control.
    pub fn record_institutional(
        &mut self,
        record: InstitutionalRecord,
    ) -> Result<(), EpistemicError> {
        self.require_statement(&record.statement_id)?;
        if record.record_key.trim().is_empty() {
            return Err(EpistemicError::EmptyInstitutionalRecordKey(record.id));
        }
        if self.institutional_records.contains_key(&record.id) {
            return Err(EpistemicError::DuplicateInstitutionalRecord(record.id));
        }
        let self_ref = EpistemicRef::InstitutionalRecord(record.id.clone());
        self.validate_basis(&record.basis, Some(&self_ref))?;

        let projection_key = (record.institution_id.clone(), record.record_key.clone());
        if let Some(existing_id) = self.current_institutional_records.get(&projection_key) {
            let existing = self
                .institutional_records
                .get(existing_id)
                .expect("current institutional index references retained record");
            if record.recorded_tick < existing.recorded_tick {
                return Err(EpistemicError::StaleInstitutionalProjection {
                    institution_id: record.institution_id,
                    record_key: record.record_key,
                    existing_tick: existing.recorded_tick,
                    attempted_tick: record.recorded_tick,
                });
            }
        }

        let record_id = record.id.clone();
        self.institutional_records.insert(record_id.clone(), record);
        self.current_institutional_records
            .insert(projection_key, record_id);
        Ok(())
    }

    /// Returns one immutable institutional record revision.
    pub fn institutional_record(&self, id: &StableId) -> Option<&InstitutionalRecord> {
        self.institutional_records.get(id)
    }

    /// Returns the current record revision for one institution-local key.
    pub fn current_institutional_record(
        &self,
        institution_id: &StableId,
        record_key: &str,
    ) -> Option<&InstitutionalRecord> {
        let key = (institution_id.clone(), record_key.to_owned());
        self.current_institutional_records
            .get(&key)
            .and_then(|id| self.institutional_records.get(id))
    }

    fn require_statement(&self, id: &StableId) -> Result<(), EpistemicError> {
        if self.statements.contains_key(id) {
            Ok(())
        } else {
            Err(EpistemicError::UnknownStatement(id.clone()))
        }
    }

    fn validate_basis(
        &self,
        basis: &BTreeSet<EpistemicRef>,
        forbidden_self: Option<&EpistemicRef>,
    ) -> Result<(), EpistemicError> {
        for reference in basis {
            if forbidden_self.is_some_and(|self_ref| self_ref == reference) {
                return Err(EpistemicError::SelfBasis(reference.clone()));
            }
            if !self.contains_ref(reference) {
                return Err(EpistemicError::UnknownBasis(reference.clone()));
            }
        }
        Ok(())
    }

    fn contains_ref(&self, reference: &EpistemicRef) -> bool {
        match reference {
            EpistemicRef::Observation(id) => self.observations.contains_key(id),
            EpistemicRef::Belief(id) => self.beliefs.contains_key(id),
            EpistemicRef::Assertion(id) => self.assertions.contains_key(id),
            EpistemicRef::InstitutionalRecord(id) => self.institutional_records.contains_key(id),
        }
    }
}

fn validate_confidence(confidence: u16) -> Result<(), EpistemicError> {
    if confidence <= MAX_CONFIDENCE {
        Ok(())
    } else {
        Err(EpistemicError::InvalidConfidence(confidence))
    }
}

fn validate_stale_window(
    recorded_tick: u64,
    stale_after_tick: Option<u64>,
    record_id: &StableId,
) -> Result<(), EpistemicError> {
    if stale_after_tick.is_some_and(|expiry| expiry < recorded_tick) {
        Err(EpistemicError::InvalidStalenessWindow {
            record_id: record_id.clone(),
            recorded_tick,
            stale_after_tick,
        })
    } else {
        Ok(())
    }
}

/// Structural errors for epistemic records/projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpistemicError {
    EmptyStatement(StableId),
    DuplicateStatement(StableId),
    UnknownStatement(StableId),
    DuplicateObservation(StableId),
    DuplicateBelief(StableId),
    DuplicateAssertion(StableId),
    DuplicateInstitutionalRecord(StableId),
    InvalidConfidence(u16),
    InvalidStalenessWindow {
        record_id: StableId,
        recorded_tick: u64,
        stale_after_tick: Option<u64>,
    },
    UnknownBasis(EpistemicRef),
    SelfBasis(EpistemicRef),
    StaleBeliefProjection {
        holder_id: StableId,
        statement_id: StableId,
        existing_tick: u64,
        attempted_tick: u64,
    },
    EmptyInstitutionalRecordKey(StableId),
    StaleInstitutionalProjection {
        institution_id: StableId,
        record_key: String,
        existing_tick: u64,
        attempted_tick: u64,
    },
}

impl fmt::Display for EpistemicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStatement(id) => write!(formatter, "epistemic statement {id} is empty"),
            Self::DuplicateStatement(id) => write!(formatter, "statement {id} already exists"),
            Self::UnknownStatement(id) => write!(formatter, "unknown statement {id}"),
            Self::DuplicateObservation(id) => write!(formatter, "observation {id} already exists"),
            Self::DuplicateBelief(id) => write!(formatter, "belief {id} already exists"),
            Self::DuplicateAssertion(id) => write!(formatter, "assertion {id} already exists"),
            Self::DuplicateInstitutionalRecord(id) => {
                write!(formatter, "institutional record {id} already exists")
            }
            Self::InvalidConfidence(value) => {
                write!(formatter, "confidence {value} exceeds {MAX_CONFIDENCE}")
            }
            Self::InvalidStalenessWindow {
                record_id,
                recorded_tick,
                stale_after_tick,
            } => write!(
                formatter,
                "record {record_id} has invalid staleness window {recorded_tick}..{stale_after_tick:?}"
            ),
            Self::UnknownBasis(reference) => write!(formatter, "unknown epistemic basis {reference:?}"),
            Self::SelfBasis(reference) => write!(formatter, "record cannot cite itself as basis: {reference:?}"),
            Self::StaleBeliefProjection {
                holder_id,
                statement_id,
                existing_tick,
                attempted_tick,
            } => write!(
                formatter,
                "belief projection for holder {holder_id} / statement {statement_id} moved backward from tick {existing_tick} to {attempted_tick}"
            ),
            Self::EmptyInstitutionalRecordKey(id) => {
                write!(formatter, "institutional record {id} has an empty logical key")
            }
            Self::StaleInstitutionalProjection {
                institution_id,
                record_key,
                existing_tick,
                attempted_tick,
            } => write!(
                formatter,
                "institutional record {institution_id}/{record_key} moved backward from tick {existing_tick} to {attempted_tick}"
            ),
        }
    }
}

impl Error for EpistemicError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn statement(id_value: &str, text: &str) -> EpistemicStatement {
        EpistemicStatement {
            id: id(id_value),
            subject_id: id("subject:station-44"),
            proposition: text.into(),
        }
    }

    #[test]
    fn observation_does_not_create_belief_implicitly() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:corrosion", "corrosion caused the failure"))
            .expect("register statement");
        ledger
            .record_observation(ObservationRecord {
                id: id("observation:inspection"),
                observer_id: id("sensor:inspection-drone"),
                statement_id: id("statement:corrosion"),
                confidence: 8_600,
                observed_tick: 10,
                stale_after_tick: Some(100),
                source_event_id: id("event:inspection"),
            })
            .expect("record observation");

        assert!(ledger.observation(&id("observation:inspection")).is_some());
        assert!(
            ledger
                .current_belief(&id("actor:minister"), &id("statement:corrosion"))
                .is_none()
        );
    }

    #[test]
    fn private_belief_and_public_assertion_may_disagree() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:corrosion", "corrosion caused the failure"))
            .expect("register corrosion");
        ledger
            .register_statement(statement("statement:sabotage", "enemy sabotage caused the failure"))
            .expect("register sabotage");
        ledger
            .record_observation(ObservationRecord {
                id: id("observation:inspection"),
                observer_id: id("sensor:inspection-drone"),
                statement_id: id("statement:corrosion"),
                confidence: 9_100,
                observed_tick: 10,
                stale_after_tick: None,
                source_event_id: id("event:inspection"),
            })
            .expect("observation");
        ledger
            .record_belief(BeliefRecord {
                id: id("belief:minister-corrosion"),
                holder_id: id("actor:minister"),
                statement_id: id("statement:corrosion"),
                confidence: 8_500,
                recorded_tick: 12,
                stale_after_tick: None,
                basis: BTreeSet::from([EpistemicRef::Observation(id("observation:inspection"))]),
                source_event_id: id("event:belief-update"),
            })
            .expect("belief");
        ledger
            .publish_assertion(AssertionRecord {
                id: id("assertion:minister-sabotage"),
                speaker_id: id("actor:minister"),
                statement_id: id("statement:sabotage"),
                declared_confidence: Some(10_000),
                asserted_tick: 13,
                basis: BTreeSet::from([EpistemicRef::Belief(id("belief:minister-corrosion"))]),
                source_event_id: id("event:press-conference"),
            })
            .expect("assertion");

        assert_eq!(
            ledger
                .current_belief(&id("actor:minister"), &id("statement:corrosion"))
                .expect("current belief")
                .statement_id,
            id("statement:corrosion")
        );
        assert_eq!(
            ledger
                .assertion(&id("assertion:minister-sabotage"))
                .expect("public assertion")
                .statement_id,
            id("statement:sabotage")
        );
    }

    #[test]
    fn institutional_record_is_an_independent_projection() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:pending", "failure cause remains under investigation"))
            .expect("statement");
        ledger
            .record_institutional(InstitutionalRecord {
                id: id("record:commission-1"),
                institution_id: id("institution:commission"),
                record_key: "incident:station-44:cause".into(),
                statement_id: id("statement:pending"),
                status: InstitutionalRecordStatus::Active,
                recorded_tick: 20,
                basis: BTreeSet::new(),
                source_event_id: id("event:commission-record"),
            })
            .expect("institutional record");

        let current = ledger
            .current_institutional_record(
                &id("institution:commission"),
                "incident:station-44:cause",
            )
            .expect("current record");
        assert_eq!(current.statement_id, id("statement:pending"));
        assert!(ledger.assertion(&id("record:commission-1")).is_none());
    }

    #[test]
    fn unknown_basis_fails_closed() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:test", "test proposition"))
            .expect("statement");
        let result = ledger.record_belief(BeliefRecord {
            id: id("belief:test"),
            holder_id: id("actor:test"),
            statement_id: id("statement:test"),
            confidence: 5_000,
            recorded_tick: 1,
            stale_after_tick: None,
            basis: BTreeSet::from([EpistemicRef::Observation(id("observation:missing"))]),
            source_event_id: id("event:test"),
        });
        assert!(matches!(result, Err(EpistemicError::UnknownBasis(_))));
    }

    #[test]
    fn belief_current_projection_cannot_move_backward() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:test", "test proposition"))
            .expect("statement");
        ledger
            .record_belief(BeliefRecord {
                id: id("belief:newer"),
                holder_id: id("actor:test"),
                statement_id: id("statement:test"),
                confidence: 6_000,
                recorded_tick: 20,
                stale_after_tick: None,
                basis: BTreeSet::new(),
                source_event_id: id("event:newer"),
            })
            .expect("newer belief");
        let result = ledger.record_belief(BeliefRecord {
            id: id("belief:older"),
            holder_id: id("actor:test"),
            statement_id: id("statement:test"),
            confidence: 7_000,
            recorded_tick: 19,
            stale_after_tick: None,
            basis: BTreeSet::new(),
            source_event_id: id("event:older"),
        });
        assert!(matches!(
            result,
            Err(EpistemicError::StaleBeliefProjection { .. })
        ));
    }

    #[test]
    fn institutional_projection_retains_old_revisions() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:first", "claimant A is recorded chancellor"))
            .expect("first");
        ledger
            .register_statement(statement("statement:second", "claimant B is recorded chancellor"))
            .expect("second");
        for (record_id, statement_id, tick) in [
            ("record:first", "statement:first", 10),
            ("record:second", "statement:second", 11),
        ] {
            ledger
                .record_institutional(InstitutionalRecord {
                    id: id(record_id),
                    institution_id: id("institution:senate"),
                    record_key: "office:chancellor".into(),
                    statement_id: id(statement_id),
                    status: InstitutionalRecordStatus::Active,
                    recorded_tick: tick,
                    basis: BTreeSet::new(),
                    source_event_id: id(&format!("event:{tick}")),
                })
                .expect("record revision");
        }

        assert!(ledger.institutional_record(&id("record:first")).is_some());
        assert_eq!(
            ledger
                .current_institutional_record(&id("institution:senate"), "office:chancellor")
                .expect("current record")
                .id,
            id("record:second")
        );
    }

    #[test]
    fn confidence_above_maximum_rejects() {
        let mut ledger = EpistemicLedger::new();
        ledger
            .register_statement(statement("statement:test", "test proposition"))
            .expect("statement");
        let result = ledger.record_observation(ObservationRecord {
            id: id("observation:test"),
            observer_id: id("sensor:test"),
            statement_id: id("statement:test"),
            confidence: 10_001,
            observed_tick: 1,
            stale_after_tick: None,
            source_event_id: id("event:test"),
        });
        assert_eq!(result, Err(EpistemicError::InvalidConfidence(10_001)));
    }
}
