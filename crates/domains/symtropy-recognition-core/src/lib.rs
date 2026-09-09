// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-relative recognition for civilization-scale simulation.
//!
//! Recognition is always held by a particular recognizer about a particular
//! subject. It is not universal truth, legitimacy, ownership, effective control,
//! or an automatic mutation of the recognized subject.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_epistemics_core::EpistemicRef;
use symtropy_game_state::StableId;
use symtropy_succession_core::SuccessionLedger;

/// Subject about which one actor/institution takes a recognition position.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecognitionSubject {
    /// One exact succession/office claim.
    OfficeClaim(StableId),
    /// One institution as an institutional entity.
    Institution(StableId),
    /// One actor/person/entity.
    Actor(StableId),
    /// Extensible external record/object identity owned by another domain.
    External {
        namespace: String,
        id: StableId,
    },
}

/// Current position one recognizer records toward one subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RecognitionPosition {
    /// Recognizer explicitly recognizes/accepts the subject under its own rules.
    Recognizes,
    /// Recognizer explicitly rejects/non-recognizes the subject.
    Rejects,
    /// Recognizer records the subject as disputed/contested rather than resolved.
    Disputes,
    /// Recognizer has explicitly withdrawn its prior position without replacing it
    /// with recognition/rejection.
    Withdrawn,
}

/// Immutable revision of one recognizer's position about one subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecognitionRecord {
    /// Stable recognition-record identity.
    pub id: StableId,
    /// Actor, institution, authority, polity, organization, or other recognizer.
    pub recognizer_id: StableId,
    /// Exact subject of recognition.
    pub subject: RecognitionSubject,
    /// Position recorded by the recognizer.
    pub position: RecognitionPosition,
    /// Canonical tick at which this revision became the recognizer's current position.
    pub recorded_tick: u64,
    /// Optional last canonical tick at which this record is effective.
    pub valid_until_tick: Option<u64>,
    /// Optional epistemic basis cited by the recognizer.
    pub epistemic_basis: BTreeSet<EpistemicRef>,
    /// Causal-history event that introduced this recognition revision.
    pub source_event_id: StableId,
}

impl RecognitionRecord {
    /// Returns whether this record is temporally effective at `tick`.
    pub fn is_effective(&self, tick: u64) -> bool {
        self.position != RecognitionPosition::Withdrawn
            && tick >= self.recorded_tick
            && self
                .valid_until_tick
                .is_none_or(|until| tick <= until)
    }
}

/// Immutable recognition history plus current per-recognizer/subject projection.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RecognitionLedger {
    records: BTreeMap<StableId, RecognitionRecord>,
    current: BTreeMap<(StableId, RecognitionSubject), StableId>,
}

impl RecognitionLedger {
    /// Creates an empty recognition ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one immutable recognition revision.
    ///
    /// Office-claim subjects are checked against the supplied succession ledger.
    /// Other subject classes remain opaque identities owned by their respective
    /// domains and are not promoted into truth by this API.
    pub fn record(
        &mut self,
        succession: &SuccessionLedger,
        record: RecognitionRecord,
    ) -> Result<(), RecognitionError> {
        validate_subject(succession, &record.subject)?;
        validate_window(record.recorded_tick, record.valid_until_tick, &record.id)?;
        if self.records.contains_key(&record.id) {
            return Err(RecognitionError::DuplicateRecord(record.id));
        }

        let key = (record.recognizer_id.clone(), record.subject.clone());
        if let Some(existing_id) = self.current.get(&key) {
            let existing = self
                .records
                .get(existing_id)
                .expect("current recognition index references retained record");
            if record.recorded_tick < existing.recorded_tick {
                return Err(RecognitionError::StaleProjection {
                    recognizer_id: record.recognizer_id,
                    subject: record.subject,
                    existing_tick: existing.recorded_tick,
                    attempted_tick: record.recorded_tick,
                });
            }
        }

        let record_id = record.id.clone();
        self.records.insert(record_id.clone(), record);
        self.current.insert(key, record_id);
        Ok(())
    }

    /// Returns one immutable recognition revision by identity.
    pub fn record_by_id(&self, id: &StableId) -> Option<&RecognitionRecord> {
        self.records.get(id)
    }

    /// Returns the latest recorded position for one recognizer/subject pair,
    /// regardless of whether it has expired or is a withdrawal.
    pub fn current_record(
        &self,
        recognizer_id: &StableId,
        subject: &RecognitionSubject,
    ) -> Option<&RecognitionRecord> {
        let key = (recognizer_id.clone(), subject.clone());
        self.current.get(&key).and_then(|id| self.records.get(id))
    }

    /// Returns the currently effective recognition position at `tick`.
    pub fn effective_record(
        &self,
        recognizer_id: &StableId,
        subject: &RecognitionSubject,
        tick: u64,
    ) -> Option<&RecognitionRecord> {
        self.current_record(recognizer_id, subject)
            .filter(|record| record.is_effective(tick))
    }

    /// Returns every recognizer's current effective position about one subject.
    ///
    /// No consensus/legitimacy scalar is calculated. Contradictory results are
    /// intentionally preserved for higher-level political reasoning.
    pub fn effective_positions_for_subject<'a>(
        &'a self,
        subject: &'a RecognitionSubject,
        tick: u64,
    ) -> impl Iterator<Item = &'a RecognitionRecord> {
        self.current
            .iter()
            .filter(move |((_, current_subject), _)| current_subject == subject)
            .filter_map(|(_, record_id)| self.records.get(record_id))
            .filter(move |record| record.is_effective(tick))
    }
}

fn validate_subject(
    succession: &SuccessionLedger,
    subject: &RecognitionSubject,
) -> Result<(), RecognitionError> {
    match subject {
        RecognitionSubject::OfficeClaim(claim_id) => {
            if succession.claim(claim_id).is_some() {
                Ok(())
            } else {
                Err(RecognitionError::UnknownOfficeClaim(claim_id.clone()))
            }
        }
        RecognitionSubject::External { namespace, .. } if namespace.trim().is_empty() => {
            Err(RecognitionError::EmptyExternalNamespace)
        }
        RecognitionSubject::Institution(_)
        | RecognitionSubject::Actor(_)
        | RecognitionSubject::External { .. } => Ok(()),
    }
}

fn validate_window(
    recorded_tick: u64,
    valid_until_tick: Option<u64>,
    record_id: &StableId,
) -> Result<(), RecognitionError> {
    if valid_until_tick.is_some_and(|until| until < recorded_tick) {
        Err(RecognitionError::InvalidTemporalWindow {
            record_id: record_id.clone(),
            recorded_tick,
            valid_until_tick,
        })
    } else {
        Ok(())
    }
}

/// Structural failures in recognition records/projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionError {
    DuplicateRecord(StableId),
    UnknownOfficeClaim(StableId),
    EmptyExternalNamespace,
    InvalidTemporalWindow {
        record_id: StableId,
        recorded_tick: u64,
        valid_until_tick: Option<u64>,
    },
    StaleProjection {
        recognizer_id: StableId,
        subject: RecognitionSubject,
        existing_tick: u64,
        attempted_tick: u64,
    },
}

impl fmt::Display for RecognitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateRecord(id) => write!(formatter, "recognition record {id} already exists"),
            Self::UnknownOfficeClaim(id) => write!(formatter, "unknown office claim {id}"),
            Self::EmptyExternalNamespace => {
                formatter.write_str("external recognition subject namespace must be non-empty")
            }
            Self::InvalidTemporalWindow {
                record_id,
                recorded_tick,
                valid_until_tick,
            } => write!(
                formatter,
                "recognition record {record_id} has invalid window {recorded_tick}..={valid_until_tick:?}"
            ),
            Self::StaleProjection {
                recognizer_id,
                subject,
                existing_tick,
                attempted_tick,
            } => write!(
                formatter,
                "recognition projection for {recognizer_id} / {subject:?} moved backward from tick {existing_tick} to {attempted_tick}"
            ),
        }
    }
}

impl Error for RecognitionError {}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_civilization_core::{Institution, OfficeDefinition};
    use symtropy_succession_core::{ClaimGrounds, OfficeClaim};

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn succession_with_two_claims() -> SuccessionLedger {
        let mut institution = Institution::new(id("institution:helion"), "Helion Compact");
        institution
            .define_office(OfficeDefinition {
                id: id("office:chancellor"),
                name: "Chancellor".into(),
                tags: BTreeSet::new(),
            })
            .expect("office");

        let mut succession = SuccessionLedger::new();
        for (claim_id, claimant, namespace) in [
            ("claim:mara", "actor:mara", "charter"),
            ("claim:kael", "actor:kael", "fleet-acclamation"),
        ] {
            succession
                .submit_claim(
                    &institution,
                    OfficeClaim {
                        id: id(claim_id),
                        institution_id: id("institution:helion"),
                        office_id: id("office:chancellor"),
                        claimant_id: id(claimant),
                        grounds: ClaimGrounds::ExtraInstitutional {
                            namespace: namespace.into(),
                            basis: "declared basis".into(),
                        },
                        asserted_tick: 10,
                        valid_until_tick: None,
                        epistemic_basis: BTreeSet::new(),
                        source_event_id: id(&format!("event:{claim_id}")),
                    },
                )
                .expect("claim");
        }
        succession
    }

    #[test]
    fn different_recognizers_can_take_conflicting_positions() {
        let succession = succession_with_two_claims();
        let mut recognition = RecognitionLedger::new();

        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:fleet-kael"),
                    recognizer_id: id("institution:fleet-command"),
                    subject: RecognitionSubject::OfficeClaim(id("claim:kael")),
                    position: RecognitionPosition::Recognizes,
                    recorded_tick: 12,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:fleet-recognition"),
                },
            )
            .expect("fleet recognizes Kael");
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:senate-kael"),
                    recognizer_id: id("institution:senate"),
                    subject: RecognitionSubject::OfficeClaim(id("claim:kael")),
                    position: RecognitionPosition::Rejects,
                    recorded_tick: 13,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:senate-rejection"),
                },
            )
            .expect("senate rejects Kael");
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:senate-mara"),
                    recognizer_id: id("institution:senate"),
                    subject: RecognitionSubject::OfficeClaim(id("claim:mara")),
                    position: RecognitionPosition::Recognizes,
                    recorded_tick: 13,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:senate-recognition"),
                },
            )
            .expect("senate recognizes Mara");

        let kael_positions = recognition
            .effective_positions_for_subject(&RecognitionSubject::OfficeClaim(id("claim:kael")), 20)
            .map(|record| record.position)
            .collect::<BTreeSet<_>>();
        assert_eq!(
            kael_positions,
            BTreeSet::from([
                RecognitionPosition::Recognizes,
                RecognitionPosition::Rejects,
            ])
        );
        assert!(succession.claim(&id("claim:kael")).is_some());
        assert!(succession.claim(&id("claim:mara")).is_some());
    }

    #[test]
    fn later_position_replaces_current_without_erasing_history() {
        let succession = succession_with_two_claims();
        let subject = RecognitionSubject::OfficeClaim(id("claim:mara"));
        let mut recognition = RecognitionLedger::new();
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:early"),
                    recognizer_id: id("institution:trade-ministry"),
                    subject: subject.clone(),
                    position: RecognitionPosition::Recognizes,
                    recorded_tick: 20,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:early"),
                },
            )
            .expect("early");
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:late"),
                    recognizer_id: id("institution:trade-ministry"),
                    subject: subject.clone(),
                    position: RecognitionPosition::Rejects,
                    recorded_tick: 30,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:late"),
                },
            )
            .expect("late");

        assert_eq!(
            recognition
                .effective_record(&id("institution:trade-ministry"), &subject, 31)
                .expect("current")
                .position,
            RecognitionPosition::Rejects
        );
        assert!(recognition.record_by_id(&id("recognition:early")).is_some());
    }

    #[test]
    fn unknown_claim_cannot_be_recognized_as_if_it_exists() {
        let succession = succession_with_two_claims();
        let mut recognition = RecognitionLedger::new();
        let result = recognition.record(
            &succession,
            RecognitionRecord {
                id: id("recognition:unknown"),
                recognizer_id: id("institution:senate"),
                subject: RecognitionSubject::OfficeClaim(id("claim:missing")),
                position: RecognitionPosition::Recognizes,
                recorded_tick: 1,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:unknown"),
            },
        );
        assert!(matches!(
            result,
            Err(RecognitionError::UnknownOfficeClaim(_))
        ));
    }

    #[test]
    fn current_projection_cannot_move_backward() {
        let succession = succession_with_two_claims();
        let subject = RecognitionSubject::OfficeClaim(id("claim:mara"));
        let mut recognition = RecognitionLedger::new();
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:new"),
                    recognizer_id: id("institution:senate"),
                    subject: subject.clone(),
                    position: RecognitionPosition::Recognizes,
                    recorded_tick: 50,
                    valid_until_tick: None,
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:new"),
                },
            )
            .expect("new");
        let result = recognition.record(
            &succession,
            RecognitionRecord {
                id: id("recognition:old"),
                recognizer_id: id("institution:senate"),
                subject,
                position: RecognitionPosition::Rejects,
                recorded_tick: 49,
                valid_until_tick: None,
                epistemic_basis: BTreeSet::new(),
                source_event_id: id("event:old"),
            },
        );
        assert!(matches!(result, Err(RecognitionError::StaleProjection { .. })));
    }

    #[test]
    fn expiry_removes_effective_position_without_rewriting_record() {
        let succession = succession_with_two_claims();
        let subject = RecognitionSubject::OfficeClaim(id("claim:mara"));
        let mut recognition = RecognitionLedger::new();
        recognition
            .record(
                &succession,
                RecognitionRecord {
                    id: id("recognition:temporary"),
                    recognizer_id: id("institution:ally"),
                    subject: subject.clone(),
                    position: RecognitionPosition::Recognizes,
                    recorded_tick: 10,
                    valid_until_tick: Some(20),
                    epistemic_basis: BTreeSet::new(),
                    source_event_id: id("event:temporary"),
                },
            )
            .expect("temporary");
        assert!(
            recognition
                .effective_record(&id("institution:ally"), &subject, 20)
                .is_some()
        );
        assert!(
            recognition
                .effective_record(&id("institution:ally"), &subject, 21)
                .is_none()
        );
        assert!(recognition.record_by_id(&id("recognition:temporary")).is_some());
    }
}
