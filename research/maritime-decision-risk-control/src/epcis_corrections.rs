// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Append-only EPCIS 2.0 error-declaration and correction reference semantics.
//!
//! EPCIS ErrorDeclaration does not mutate/delete the prior event. It records
//! that the prior assertions are erroneous from declarationTime, preserving
//! the prior event and the error declaration for subsequent queries.
//! correctiveEventIDs identify replacement event records; they are not
//! silently substituted for the original or treated as available before their
//! own repository recordTime.
//!
//! This module is a narrow typed compatibility fixture, not a full EPCIS
//! parser, JSON-LD implementation, conformance validator or physical witness.

use std::fmt::Write as FmtWrite;

const EVENT_DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const EVENT_DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpcisEventRecord {
    /// EPCIS eventID (a globally unique URI under the standard).
    pub event_id: String,
    /// Canonical EPC/object identifiers participating in the event.
    pub subject_ids: Vec<String>,
    /// EPCIS eventTime: when the source asserts the real-world event occurred.
    pub event_time_unix: i64,
    /// EPCIS eventTimeZoneOffset, retained independently from UTC event time.
    pub event_time_zone_offset: String,
    /// EPCIS recordTime: when the repository recorded the event.
    pub record_time_unix: i64,
    /// Source-native row/record identity, distinct from eventID.
    pub source_record_id: String,
    /// Digest of the event assertions excluding recordTime and ErrorDeclaration.
    pub assertion_payload_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpcisErrorDeclaration {
    /// Internal source-record identity for this ErrorDeclaration record.
    pub declaration_record_id: String,
    /// EPCIS eventID of the prior event being declared erroneous.
    /// This deliberately repeats the prior eventID, as permitted by EPCIS.
    pub event_id: String,
    pub subject_ids: Vec<String>,
    /// Must equal the prior event's eventTime.
    pub event_time_unix: i64,
    pub event_time_zone_offset: String,
    /// When the repository recorded this ErrorDeclaration.
    pub record_time_unix: i64,
    /// EPCIS ErrorDeclaration.declarationTime.
    pub declaration_time_unix: i64,
    pub reason: String,
    /// Zero or more eventIDs for events that correct the original assertion.
    pub corrective_event_ids: Vec<String>,
    /// Must match the prior event payload other than recordTime/ErrorDeclaration.
    pub assertion_payload_sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EpcisRecordValidationError {
    EmptyIdentity,
    InvalidEventIdUri,
    InvalidSubjectUri,
    EmptySubjectSet,
    DuplicateSubject,
    InvalidSha256,
    InvalidTimeZoneOffset,
    NegativeTimestamp,
    RecordTimeBeforeEventTime,
    DuplicateEventId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrectionAppendError {
    InvalidDeclarationIdentity,
    InvalidEventId,
    TargetEventUnknown,
    SubjectIdentityMismatch,
    EventTimeMismatch,
    TimeZoneOffsetMismatch,
    DeclarationTimeBeforeEvent,
    RecordTimeBeforeDeclaration,
    AssertionPayloadMismatch,
    DuplicateDeclarationRecord,
    DuplicateErrorDeclarationForEvent,
    DuplicateCorrectiveEventId,
    CorrectiveEventIdEqualsOriginal,
    InvalidCorrectiveEventId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventInterpretation {
    /// No error declaration was visible at this historical cutoff.
    AsReported,
    /// The original event is retained but must no longer be interpreted as true.
    DeclaredInError {
        declaration_record_id: String,
        declaration_time_unix: i64,
        reason: String,
        corrective_event_ids: Vec<String>,
        corrective_events_available: Vec<String>,
        corrective_events_not_yet_available: Vec<String>,
    },
}

impl EventInterpretation {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::AsReported => "AS_REPORTED",
            Self::DeclaredInError { .. } => "DECLARED_IN_ERROR",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventAsOfView {
    /// The original immutable event remains present even after error declaration.
    pub original_event: EpcisEventRecord,
    pub interpretation: EventInterpretation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EpcisCorrectionLedger {
    events: Vec<EpcisEventRecord>,
    declarations: Vec<EpcisErrorDeclaration>,
}

fn is_uri(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        && !rest.is_empty()
        && !value.chars().any(char::is_whitespace)
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_timezone_offset(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 6 || !matches!(bytes[0], b'+' | b'-') || bytes[3] != b':' {
        return false;
    }
    let digits = [bytes[1], bytes[2], bytes[4], bytes[5]];
    if !digits.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let hours = u16::from(bytes[1] - b'0') * 10 + u16::from(bytes[2] - b'0');
    let minutes = u16::from(bytes[4] - b'0') * 10 + u16::from(bytes[5] - b'0');
    hours <= 14 && minutes < 60 && (hours != 14 || minutes == 0)
}

impl EpcisEventRecord {
    pub fn validate(&self) -> Result<(), EpcisRecordValidationError> {
        if self.event_id.trim().is_empty() || self.source_record_id.trim().is_empty() {
            return Err(EpcisRecordValidationError::EmptyIdentity);
        }
        if !is_uri(&self.event_id) {
            return Err(EpcisRecordValidationError::InvalidEventIdUri);
        }
        if self.subject_ids.is_empty() {
            return Err(EpcisRecordValidationError::EmptySubjectSet);
        }
        let mut seen = std::collections::BTreeSet::new();
        for subject_id in &self.subject_ids {
            if !is_uri(subject_id) {
                return Err(EpcisRecordValidationError::InvalidSubjectUri);
            }
            if !seen.insert(subject_id) {
                return Err(EpcisRecordValidationError::DuplicateSubject);
            }
        }
        if !valid_sha256(&self.assertion_payload_sha256) {
            return Err(EpcisRecordValidationError::InvalidSha256);
        }
        if !valid_timezone_offset(&self.event_time_zone_offset) {
            return Err(EpcisRecordValidationError::InvalidTimeZoneOffset);
        }
        if self.event_time_unix < 0 || self.record_time_unix < 0 {
            return Err(EpcisRecordValidationError::NegativeTimestamp);
        }
        if self.record_time_unix < self.event_time_unix {
            return Err(EpcisRecordValidationError::RecordTimeBeforeEventTime);
        }
        Ok(())
    }
}

impl EpcisCorrectionLedger {
    pub fn events(&self) -> &[EpcisEventRecord] {
        &self.events
    }

    pub fn declarations(&self) -> &[EpcisErrorDeclaration] {
        &self.declarations
    }

    /// Appends an original or corrective event; eventID cannot be silently reused.
    pub fn append_event(
        &mut self,
        event: EpcisEventRecord,
    ) -> Result<(), EpcisRecordValidationError> {
        event.validate()?;
        if self
            .events
            .iter()
            .any(|existing| existing.event_id == event.event_id)
        {
            return Err(EpcisRecordValidationError::DuplicateEventId);
        }
        self.events.push(event);
        Ok(())
    }

    /// Appends an ErrorDeclaration without changing the original event.
    ///
    /// The repeated eventID is intentional and only occurs in this separate
    /// declaration object. All other prior-event assertions are bound by
    /// subject IDs, eventTime, time-zone offset and payload digest.
    pub fn declare_event_in_error(
        &mut self,
        declaration: EpcisErrorDeclaration,
    ) -> Result<(), CorrectionAppendError> {
        if declaration.declaration_record_id.trim().is_empty()
            || declaration.reason.trim().is_empty()
        {
            return Err(CorrectionAppendError::InvalidDeclarationIdentity);
        }
        if !is_uri(&declaration.event_id) {
            return Err(CorrectionAppendError::InvalidEventId);
        }
        if !valid_timezone_offset(&declaration.event_time_zone_offset)
            || declaration.event_time_unix < 0
            || declaration.declaration_time_unix < 0
            || declaration.record_time_unix < 0
        {
            return Err(CorrectionAppendError::InvalidDeclarationIdentity);
        }
        if !valid_sha256(&declaration.assertion_payload_sha256) {
            return Err(CorrectionAppendError::AssertionPayloadMismatch);
        }
        if declaration.record_time_unix < declaration.declaration_time_unix {
            return Err(CorrectionAppendError::RecordTimeBeforeDeclaration);
        }
        let Some(original) = self
            .events
            .iter()
            .find(|event| event.event_id == declaration.event_id)
        else {
            return Err(CorrectionAppendError::TargetEventUnknown);
        };
        if original.subject_ids != declaration.subject_ids {
            return Err(CorrectionAppendError::SubjectIdentityMismatch);
        }
        if original.event_time_unix != declaration.event_time_unix {
            return Err(CorrectionAppendError::EventTimeMismatch);
        }
        if original.event_time_zone_offset != declaration.event_time_zone_offset {
            return Err(CorrectionAppendError::TimeZoneOffsetMismatch);
        }
        if declaration.declaration_time_unix < original.event_time_unix {
            return Err(CorrectionAppendError::DeclarationTimeBeforeEvent);
        }
        if original.assertion_payload_sha256 != declaration.assertion_payload_sha256 {
            return Err(CorrectionAppendError::AssertionPayloadMismatch);
        }
        if self
            .declarations
            .iter()
            .any(|existing| existing.declaration_record_id == declaration.declaration_record_id)
        {
            return Err(CorrectionAppendError::DuplicateDeclarationRecord);
        }
        if self
            .declarations
            .iter()
            .any(|existing| existing.event_id == declaration.event_id)
        {
            return Err(CorrectionAppendError::DuplicateErrorDeclarationForEvent);
        }
        let mut corrective_ids = std::collections::BTreeSet::new();
        for corrective_event_id in &declaration.corrective_event_ids {
            if !is_uri(corrective_event_id) {
                return Err(CorrectionAppendError::InvalidCorrectiveEventId);
            }
            if corrective_event_id == &declaration.event_id {
                return Err(CorrectionAppendError::CorrectiveEventIdEqualsOriginal);
            }
            if !corrective_ids.insert(corrective_event_id) {
                return Err(CorrectionAppendError::DuplicateCorrectiveEventId);
            }
        }

        self.declarations.push(declaration);
        Ok(())
    }

    /// Returns the event interpretation supported by records available at cutoff.
    ///
    /// Before declaration.recordTime, the historical view remains AS_REPORTED.
    /// Once the ErrorDeclaration is available, the original is marked in error.
    /// Corrective events are listed only if their own recordTime is also in view.
    pub fn event_as_of(&self, event_id: &str, as_of_unix: i64) -> Option<EventAsOfView> {
        if as_of_unix < 0 {
            return None;
        }
        let original = self
            .events
            .iter()
            .find(|event| event.event_id == event_id && event.record_time_unix <= as_of_unix)?;
        let declaration = self.declarations.iter().find(|declaration| {
            declaration.event_id == event_id
                && declaration.declaration_time_unix <= as_of_unix
                && declaration.record_time_unix <= as_of_unix
        });

        let interpretation = match declaration {
            None => EventInterpretation::AsReported,
            Some(declaration) => {
                let mut available = Vec::new();
                let mut unavailable = Vec::new();
                for corrective_id in &declaration.corrective_event_ids {
                    if self.events.iter().any(|event| {
                        event.event_id == *corrective_id && event.record_time_unix <= as_of_unix
                    }) {
                        available.push(corrective_id.clone());
                    } else {
                        unavailable.push(corrective_id.clone());
                    }
                }
                EventInterpretation::DeclaredInError {
                    declaration_record_id: declaration.declaration_record_id.clone(),
                    declaration_time_unix: declaration.declaration_time_unix,
                    reason: declaration.reason.clone(),
                    corrective_event_ids: declaration.corrective_event_ids.clone(),
                    corrective_events_available: available,
                    corrective_events_not_yet_available: unavailable,
                }
            }
        };

        Some(EventAsOfView {
            original_event: original.clone(),
            interpretation,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReceiptCase {
    BeforeDeclaration,
    DeclarationNotYetRecorded,
    DeclaredCorrectionPending,
    CorrectiveEventAvailable,
    SubjectMismatchRejected,
    PayloadMismatchRejected,
}

impl ReceiptCase {
    const fn label(self) -> &'static str {
        match self {
            Self::BeforeDeclaration => "BEFORE_DECLARATION",
            Self::DeclarationNotYetRecorded => "DECLARATION_NOT_YET_RECORDED",
            Self::DeclaredCorrectionPending => "DECLARED_CORRECTION_PENDING",
            Self::CorrectiveEventAvailable => "CORRECTIVE_EVENT_AVAILABLE",
            Self::SubjectMismatchRejected => "SUBJECT_MISMATCH_REJECTED",
            Self::PayloadMismatchRejected => "PAYLOAD_MISMATCH_REJECTED",
        }
    }
}

fn original_event() -> EpcisEventRecord {
    EpcisEventRecord {
        event_id: "urn:uuid:11111111-1111-4111-8111-111111111111".to_owned(),
        subject_ids: vec!["urn:epc:id:sscc:0614141.1234567890".to_owned()],
        event_time_unix: 1_900_000_000,
        event_time_zone_offset: "+02:00".to_owned(),
        record_time_unix: 1_900_000_030,
        source_record_id: "DGT-GATE-RECORD-001".to_owned(),
        assertion_payload_sha256: EVENT_DIGEST_A.to_owned(),
    }
}

fn error_declaration() -> EpcisErrorDeclaration {
    EpcisErrorDeclaration {
        declaration_record_id: "urn:uuid:22222222-2222-4222-8222-222222222222".to_owned(),
        event_id: "urn:uuid:11111111-1111-4111-8111-111111111111".to_owned(),
        subject_ids: vec!["urn:epc:id:sscc:0614141.1234567890".to_owned()],
        event_time_unix: 1_900_000_000,
        event_time_zone_offset: "+02:00".to_owned(),
        record_time_unix: 1_900_000_400,
        declaration_time_unix: 1_900_000_300,
        reason: "SOURCE_CAPTURE_DUPLICATE_OR_INCORRECT".to_owned(),
        corrective_event_ids: vec!["urn:uuid:33333333-3333-4333-8333-333333333333".to_owned()],
        assertion_payload_sha256: EVENT_DIGEST_A.to_owned(),
    }
}

fn corrective_event() -> EpcisEventRecord {
    EpcisEventRecord {
        event_id: "urn:uuid:33333333-3333-4333-8333-333333333333".to_owned(),
        subject_ids: vec!["urn:epc:id:sscc:0614141.1234567890".to_owned()],
        event_time_unix: 1_900_000_000,
        event_time_zone_offset: "+02:00".to_owned(),
        record_time_unix: 1_900_000_500,
        source_record_id: "DGT-GATE-RECORD-001-CORRECTED".to_owned(),
        assertion_payload_sha256: EVENT_DIGEST_B.to_owned(),
    }
}

pub fn render_csv() -> String {
    let mut ledger = EpcisCorrectionLedger::default();
    ledger
        .append_event(original_event())
        .expect("valid original EPCIS event");
    let mut output = String::from(
        "case_id,as_of_unix,original_retained,interpretation,available_correctives,unavailable_correctives,claim_ceiling\n",
    );

    let before = ledger
        .event_as_of(
            "urn:uuid:11111111-1111-4111-8111-111111111111",
            1_900_000_200,
        )
        .expect("original event available");
    let _ = writeln!(
        output,
        "{},1900000200,true,{},{},0,SYNTHETIC_ONLY",
        ReceiptCase::BeforeDeclaration.label(),
        before.interpretation.label(),
        0
    );

    ledger
        .declare_event_in_error(error_declaration())
        .expect("valid error declaration");

    let not_recorded = ledger
        .event_as_of(
            "urn:uuid:11111111-1111-4111-8111-111111111111",
            1_900_000_350,
        )
        .expect("original event still visible before declaration recordTime");
    let _ = writeln!(
        output,
        "{},1900000350,true,{},{},0,SYNTHETIC_ONLY",
        ReceiptCase::DeclarationNotYetRecorded.label(),
        not_recorded.interpretation.label(),
        0
    );

    let pending = ledger
        .event_as_of(
            "urn:uuid:11111111-1111-4111-8111-111111111111",
            1_900_000_450,
        )
        .expect("error declaration is now visible");
    let (available, unavailable) = match &pending.interpretation {
        EventInterpretation::DeclaredInError {
            corrective_events_available,
            corrective_events_not_yet_available,
            ..
        } => (
            corrective_events_available.len(),
            corrective_events_not_yet_available.len(),
        ),
        EventInterpretation::AsReported => unreachable!("error declaration is recorded"),
    };
    let _ = writeln!(
        output,
        "{},1900000450,true,DECLARED_IN_ERROR,{available},{unavailable},SYNTHETIC_ONLY",
        ReceiptCase::DeclaredCorrectionPending.label()
    );

    ledger
        .append_event(corrective_event())
        .expect("valid corrective EPCIS event");
    let corrected = ledger
        .event_as_of(
            "urn:uuid:11111111-1111-4111-8111-111111111111",
            1_900_000_600,
        )
        .expect("original event retained after correction arrival");
    let (available, unavailable) = match &corrected.interpretation {
        EventInterpretation::DeclaredInError {
            corrective_events_available,
            corrective_events_not_yet_available,
            ..
        } => (
            corrective_events_available.len(),
            corrective_events_not_yet_available.len(),
        ),
        EventInterpretation::AsReported => unreachable!("error declaration remains present"),
    };
    let _ = writeln!(
        output,
        "{},1900000600,true,DECLARED_IN_ERROR,{available},{unavailable},SYNTHETIC_ONLY",
        ReceiptCase::CorrectiveEventAvailable.label()
    );

    let mut subject_mismatch = error_declaration();
    subject_mismatch.declaration_record_id =
        "urn:uuid:44444444-4444-4444-8444-444444444444".to_owned();
    subject_mismatch.subject_ids = vec!["urn:epc:id:sscc:0614141.9999999999".to_owned()];
    let _ = writeln!(
        output,
        "{},NA,true,REJECTED,0,0,SYNTHETIC_ONLY",
        ReceiptCase::SubjectMismatchRejected.label()
    );
    debug_assert_eq!(
        ledger.declare_event_in_error(subject_mismatch),
        Err(CorrectionAppendError::SubjectIdentityMismatch)
    );

    let mut payload_mismatch = error_declaration();
    payload_mismatch.declaration_record_id =
        "urn:uuid:55555555-5555-4555-8555-555555555555".to_owned();
    payload_mismatch.assertion_payload_sha256 = EVENT_DIGEST_B.to_owned();
    let _ = writeln!(
        output,
        "{},NA,true,REJECTED,0,0,SYNTHETIC_ONLY",
        ReceiptCase::PayloadMismatchRejected.label()
    );
    debug_assert_eq!(
        ledger.declare_event_in_error(payload_mismatch),
        Err(CorrectionAppendError::AssertionPayloadMismatch)
    );

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_time_record_time_and_timezone_offset_are_retained_separately() {
        let event = original_event();
        event.validate().expect("valid event");
        assert_eq!(event.event_time_unix, 1_900_000_000);
        assert_eq!(event.record_time_unix, 1_900_000_030);
        assert_eq!(event.event_time_zone_offset, "+02:00");
    }

    #[test]
    fn error_declaration_preserves_original_and_changes_only_later_interpretation() {
        let mut ledger = EpcisCorrectionLedger::default();
        let original = original_event();
        let event_id = original.event_id.clone();
        ledger
            .append_event(original.clone())
            .expect("append original");

        let before = ledger
            .event_as_of(&event_id, 1_900_000_200)
            .expect("original available");
        assert_eq!(before.interpretation, EventInterpretation::AsReported);

        ledger
            .declare_event_in_error(error_declaration())
            .expect("append ErrorDeclaration");
        let declaration_not_yet_recorded = ledger
            .event_as_of(&event_id, 1_900_000_350)
            .expect("the declaration is not in the repository yet");
        assert_eq!(
            declaration_not_yet_recorded.interpretation,
            EventInterpretation::AsReported
        );

        let after = ledger
            .event_as_of(&event_id, 1_900_000_450)
            .expect("declaration now visible");
        assert!(matches!(
            after.interpretation,
            EventInterpretation::DeclaredInError { .. }
        ));
        assert_eq!(after.original_event, original);
        assert_eq!(ledger.events().len(), 1);
        assert_eq!(ledger.declarations().len(), 1);
    }

    #[test]
    fn corrective_event_is_not_reported_available_before_its_own_record_time() {
        let mut ledger = EpcisCorrectionLedger::default();
        ledger.append_event(original_event()).expect("original");
        ledger
            .declare_event_in_error(error_declaration())
            .expect("error declaration");

        let pending = ledger
            .event_as_of(
                "urn:uuid:11111111-1111-4111-8111-111111111111",
                1_900_000_450,
            )
            .expect("declared event");
        assert!(matches!(
            pending.interpretation,
            EventInterpretation::DeclaredInError {
                corrective_events_available,
                corrective_events_not_yet_available,
                ..
            } if corrective_events_available.is_empty()
                && corrective_events_not_yet_available.len() == 1
        ));

        ledger
            .append_event(corrective_event())
            .expect("corrective event");
        let resolved = ledger
            .event_as_of(
                "urn:uuid:11111111-1111-4111-8111-111111111111",
                1_900_000_600,
            )
            .expect("corrective event now available");
        assert!(matches!(
            resolved.interpretation,
            EventInterpretation::DeclaredInError {
                corrective_events_available,
                corrective_events_not_yet_available,
                ..
            } if corrective_events_available.len() == 1
                && corrective_events_not_yet_available.is_empty()
        ));
        assert_eq!(
            resolved.original_event.event_id,
            "urn:uuid:11111111-1111-4111-8111-111111111111"
        );
    }

    #[test]
    fn error_declaration_requires_same_event_identity_subject_time_offset_and_payload() {
        let mut ledger = EpcisCorrectionLedger::default();
        ledger.append_event(original_event()).expect("original");

        let mut declaration = error_declaration();
        declaration.subject_ids[0] = "urn:epc:id:sscc:0614141.9999999999".to_owned();
        assert_eq!(
            ledger.declare_event_in_error(declaration),
            Err(CorrectionAppendError::SubjectIdentityMismatch)
        );

        let mut declaration = error_declaration();
        declaration.event_time_unix += 1;
        assert_eq!(
            ledger.declare_event_in_error(declaration),
            Err(CorrectionAppendError::EventTimeMismatch)
        );

        let mut declaration = error_declaration();
        declaration.event_time_zone_offset = "+01:00".to_owned();
        assert_eq!(
            ledger.declare_event_in_error(declaration),
            Err(CorrectionAppendError::TimeZoneOffsetMismatch)
        );

        let mut declaration = error_declaration();
        declaration.assertion_payload_sha256 = EVENT_DIGEST_B.to_owned();
        assert_eq!(
            ledger.declare_event_in_error(declaration),
            Err(CorrectionAppendError::AssertionPayloadMismatch)
        );
    }

    #[test]
    fn event_id_cannot_be_reused_by_a_second_normal_event() {
        let mut ledger = EpcisCorrectionLedger::default();
        ledger.append_event(original_event()).expect("original");
        assert_eq!(
            ledger.append_event(original_event()),
            Err(EpcisRecordValidationError::DuplicateEventId)
        );
    }

    #[test]
    fn invalid_uri_timezone_and_record_order_are_rejected() {
        let mut event = original_event();
        event.event_id = "not a URI".to_owned();
        assert_eq!(
            event.validate(),
            Err(EpcisRecordValidationError::InvalidEventIdUri)
        );

        let mut event = original_event();
        event.event_time_zone_offset = "+15:00".to_owned();
        assert_eq!(
            event.validate(),
            Err(EpcisRecordValidationError::InvalidTimeZoneOffset)
        );

        let mut event = original_event();
        event.record_time_unix = event.event_time_unix - 1;
        assert_eq!(
            event.validate(),
            Err(EpcisRecordValidationError::RecordTimeBeforeEventTime)
        );
    }

    #[test]
    fn receipt_is_deterministic_and_contains_all_correction_states() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 7);
        let header_width = first.lines().next().expect("header").split(',').count();
        assert_eq!(header_width, 7);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), header_width);
        }
        assert!(first.contains("AS_REPORTED"));
        assert!(first.contains("DECLARED_IN_ERROR"));
        assert!(first.contains("CORRECTIVE_EVENT_AVAILABLE"));
        assert!(first.contains("SUBJECT_MISMATCH_REJECTED"));
        assert!(first.contains("PAYLOAD_MISMATCH_REJECTED"));
    }
}
