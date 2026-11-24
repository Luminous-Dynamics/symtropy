// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! DCSA event-classifier maturity adapter.
//!
//! DCSA eventClassifierCode distinguishes ESTimated, REQuested, PLanNed and
//! ACTual milestones. This adapter preserves that maturity in a typed result.
//! ACT means "the source classifies this event as actual"; it is not an
//! independent physical witness, capture-completeness proof or receiver receipt.
//!
//! This is a narrow, version-pinned mapping fixture, not a full DCSA T&T or
//! Port Call parser, JSON schema validator, API conformance implementation or
//! event-identity authority.

use std::fmt::Write as FmtWrite;

const EXPECTED_STANDARD_VERSION: &str = "DCSA-TT-2.2";
const FIXTURE_AS_OF: i64 = 2_000_000_000;
const FIXTURE_PUBLISHED: i64 = 1_900_000_000;
const FIXTURE_RETRIEVED: i64 = 1_900_000_010;
const FIXTURE_EVENT_TIME: i64 = 1_899_999_900;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcsaEventClassifier {
    Estimated,
    Requested,
    Planned,
    Actual,
}

impl DcsaEventClassifier {
    pub fn parse(code: &str) -> Result<Self, DcsaClassifierError> {
        match code {
            "EST" => Ok(Self::Estimated),
            "REQ" => Ok(Self::Requested),
            "PLN" => Ok(Self::Planned),
            "ACT" => Ok(Self::Actual),
            _ => Err(DcsaClassifierError::UnknownClassifier(code.to_owned())),
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::Estimated => "EST",
            Self::Requested => "REQ",
            Self::Planned => "PLN",
            Self::Actual => "ACT",
        }
    }

    pub const fn maturity(self) -> DcsaEventMaturity {
        match self {
            Self::Estimated => DcsaEventMaturity::EstimatedOnly,
            Self::Requested => DcsaEventMaturity::RequestedOnly,
            Self::Planned => DcsaEventMaturity::PlannedOnly,
            Self::Actual => DcsaEventMaturity::SourceReportedActualOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcsaEventMaturity {
    EstimatedOnly,
    RequestedOnly,
    PlannedOnly,
    SourceReportedActualOnly,
}

impl DcsaEventMaturity {
    pub const fn label(self) -> &'static str {
        match self {
            Self::EstimatedOnly => "ESTIMATED_ONLY",
            Self::RequestedOnly => "REQUESTED_ONLY",
            Self::PlannedOnly => "PLANNED_ONLY",
            Self::SourceReportedActualOnly => "SOURCE_REPORTED_ACTUAL_ONLY",
        }
    }

    /// No DCSA classifier code is an independent physical witness.
    pub const fn independent_physical_observation(self) -> bool {
        false
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DcsaClassifierError {
    UnknownClassifier(String),
    WrongStandardVersion,
    EmptyIdentity,
    InvalidUri,
    InvalidTimestamp,
    EventTimeUnknown,
    InvalidTimezoneOffset,
    RecordTimeBeforeEventTime,
    PublicationTimeUnknown,
    PublicationAfterRetrieval,
    EvidenceUnavailableAtAsOf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcsaSourceEvent {
    /// Exact DCSA release pinned by the adapter. Do not silently accept drift.
    pub standard_version: String,
    /// DCSA eventID URI.
    pub event_id: String,
    /// Canonical EPC/container, shipment, vessel-call or other subject URI.
    pub subject_id: String,
    /// Source-native message/record identity.
    pub source_record_id: String,
    /// eventClassifierCode: EST, REQ, PLN or ACT.
    pub classifier_code: String,
    /// DCSA eventDateTime / event time in UTC, when supplied by this event family.
    pub event_time_unix: Option<i64>,
    /// Original local offset retained separately from UTC.
    pub event_time_zone_offset: String,
    /// When the consuming repository received/recorded this message.
    pub record_time_unix: i64,
    /// When the publisher made this source record available.
    pub published_at_unix: Option<i64>,
    /// When the evidence service actually retrieved this record.
    pub retrieved_at_unix: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DcsaMaturityReceipt {
    pub standard_version: String,
    pub event_id: String,
    pub subject_id: String,
    pub source_record_id: String,
    pub classifier: DcsaEventClassifier,
    pub maturity: DcsaEventMaturity,
    pub event_time_unix: Option<i64>,
    pub event_time_zone_offset: String,
    pub record_time_unix: i64,
    pub published_at_unix: i64,
    pub retrieved_at_unix: i64,
    /// Always false: source event maturity does not establish independent truth.
    pub independent_physical_observation: bool,
    pub claim_ceiling: &'static str,
}

impl DcsaSourceEvent {
    pub fn admit_as_of(
        &self,
        as_of_unix: i64,
    ) -> Result<DcsaMaturityReceipt, DcsaClassifierError> {
        if self.standard_version != EXPECTED_STANDARD_VERSION {
            return Err(DcsaClassifierError::WrongStandardVersion);
        }
        if self.event_id.trim().is_empty()
            || self.subject_id.trim().is_empty()
            || self.source_record_id.trim().is_empty()
        {
            return Err(DcsaClassifierError::EmptyIdentity);
        }
        if !is_uri(&self.event_id) || !is_uri(&self.subject_id) {
            return Err(DcsaClassifierError::InvalidUri);
        }
        let classifier = DcsaEventClassifier::parse(&self.classifier_code)?;
        if !valid_timezone_offset(&self.event_time_zone_offset) {
            return Err(DcsaClassifierError::InvalidTimezoneOffset);
        }
        if as_of_unix < 0
            || self.record_time_unix < 0
            || self.retrieved_at_unix < 0
            || self.published_at_unix.is_some_and(|time| time < 0)
            || self.event_time_unix.is_some_and(|time| time < 0)
        {
            return Err(DcsaClassifierError::InvalidTimestamp);
        }
        let Some(published_at) = self.published_at_unix else {
            return Err(DcsaClassifierError::PublicationTimeUnknown);
        };
        if published_at > self.retrieved_at_unix {
            return Err(DcsaClassifierError::PublicationAfterRetrieval);
        }
        if self.record_time_unix > as_of_unix
            || self.retrieved_at_unix > as_of_unix
            || published_at > as_of_unix
        {
            return Err(DcsaClassifierError::EvidenceUnavailableAtAsOf);
        }
        if classifier == DcsaEventClassifier::Actual && self.event_time_unix.is_none() {
            return Err(DcsaClassifierError::EventTimeUnknown);
        }
        if classifier == DcsaEventClassifier::Actual
            && self
                .event_time_unix
                .is_some_and(|event_time| self.record_time_unix < event_time)
        {
            return Err(DcsaClassifierError::RecordTimeBeforeEventTime);
        }

        Ok(DcsaMaturityReceipt {
            standard_version: self.standard_version.clone(),
            event_id: self.event_id.clone(),
            subject_id: self.subject_id.clone(),
            source_record_id: self.source_record_id.clone(),
            classifier,
            maturity: classifier.maturity(),
            event_time_unix: self.event_time_unix,
            event_time_zone_offset: self.event_time_zone_offset.clone(),
            record_time_unix: self.record_time_unix,
            published_at_unix: published_at,
            retrieved_at_unix: self.retrieved_at_unix,
            independent_physical_observation: false,
            claim_ceiling: classifier.maturity().label(),
        })
    }
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

fn fixture_event(code: &str) -> DcsaSourceEvent {
    DcsaSourceEvent {
        standard_version: EXPECTED_STANDARD_VERSION.to_owned(),
        event_id: format!("urn:uuid:{code}-1111-4111-8111-111111111111"),
        subject_id: "urn:epc:id:sscc:0614141.1234567890".to_owned(),
        source_record_id: format!("dgt-source-record-{code}"),
        classifier_code: code.to_owned(),
        event_time_unix: Some(FIXTURE_EVENT_TIME),
        event_time_zone_offset: "+02:00".to_owned(),
        record_time_unix: FIXTURE_RETRIEVED,
        published_at_unix: Some(FIXTURE_PUBLISHED),
        retrieved_at_unix: FIXTURE_RETRIEVED,
    }
}

#[derive(Clone, Copy)]
struct BenchmarkCase {
    id: &'static str,
    code: &'static str,
    expected: &'static str,
    published: Option<i64>,
    retrieved: i64,
    record_time: i64,
    event_time: Option<i64>,
    version: &'static str,
}

fn benchmark_cases() -> [BenchmarkCase; 11] {
    [
        BenchmarkCase {
            id: "estimated_remains_estimated",
            code: "EST",
            expected: "ESTIMATED_ONLY",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "requested_remains_requested",
            code: "REQ",
            expected: "REQUESTED_ONLY",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "planned_remains_planned",
            code: "PLN",
            expected: "PLANNED_ONLY",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "actual_remains_source_reported_actual",
            code: "ACT",
            expected: "SOURCE_REPORTED_ACTUAL_ONLY",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "unknown_classifier_rejected",
            code: "XYZ",
            expected: "UNKNOWN_CLASSIFIER",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "wrong_standard_version_rejected",
            code: "ACT",
            expected: "WRONG_STANDARD_VERSION",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: "DCSA-TT-2.1",
        },
        BenchmarkCase {
            id: "unknown_publication_time_rejected",
            code: "ACT",
            expected: "PUBLICATION_TIME_UNKNOWN",
            published: None,
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "future_retrieval_rejected",
            code: "ACT",
            expected: "EVIDENCE_UNAVAILABLE_AT_AS_OF",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_AS_OF + 1,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "actual_without_event_time_rejected",
            code: "ACT",
            expected: "EVENT_TIME_UNKNOWN",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: None,
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "planned_future_event_time_is_admissible",
            code: "PLN",
            expected: "PLANNED_ONLY",
            published: Some(FIXTURE_PUBLISHED),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_AS_OF + 10),
            version: EXPECTED_STANDARD_VERSION,
        },
        BenchmarkCase {
            id: "publication_after_retrieval_rejected",
            code: "ACT",
            expected: "PUBLICATION_AFTER_RETRIEVAL",
            published: Some(FIXTURE_RETRIEVAL + 10),
            retrieved: FIXTURE_RETRIEVED,
            record_time: FIXTURE_RETRIEVED,
            event_time: Some(FIXTURE_EVENT_TIME),
            version: EXPECTED_STANDARD_VERSION,
        },
    ]
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,classifier_code,expected_maturity,observed_maturity,independent_physical_observation,claim_ceiling,status\n",
    );
    for case in benchmark_cases() {
        let mut event = fixture_event(case.code);
        event.standard_version = case.version.to_owned();
        event.published_at_unix = case.published;
        event.retrieved_at_unix = case.retrieved;
        event.record_time_unix = case.record_time;
        event.event_time_unix = case.event_time;

        let result = event.admit_as_of(FIXTURE_AS_OF);
        match result {
            Ok(receipt) => {
                let observed = receipt.maturity.label();
                assert_eq!(observed, case.expected, "DCSA case {}", case.id);
                assert!(!receipt.independent_physical_observation);
                let _ = writeln!(
                    output,
                    "{},{},{},{},false,{},ADMITTED",
                    case.id,
                    case.code,
                    case.expected,
                    observed,
                    receipt.claim_ceiling
                );
            }
            Err(error) => {
                let observed = match error {
                    DcsaClassifierError::UnknownClassifier(_) => "UNKNOWN_CLASSIFIER",
                    DcsaClassifierError::WrongStandardVersion => "WRONG_STANDARD_VERSION",
                    DcsaClassifierError::EmptyIdentity => "EMPTY_IDENTITY",
                    DcsaClassifierError::InvalidUri => "INVALID_URI",
                    DcsaClassifierError::InvalidTimestamp => "INVALID_TIMESTAMP",
                    DcsaClassifierError::EventTimeUnknown => "EVENT_TIME_UNKNOWN",
                    DcsaClassifierError::InvalidTimezoneOffset => "INVALID_TIMEZONE_OFFSET",
                    DcsaClassifierError::RecordTimeBeforeEventTime => {
                        "RECORD_TIME_BEFORE_EVENT_TIME"
                    }
                    DcsaClassifierError::PublicationTimeUnknown => "PUBLICATION_TIME_UNKNOWN",
                    DcsaClassifierError::PublicationAfterRetrieval => "PUBLICATION_AFTER_RETRIEVAL",
                    DcsaClassifierError::EvidenceUnavailableAtAsOf => "EVIDENCE_UNAVAILABLE_AT_AS_OF",
                };
                assert_eq!(observed, case.expected, "DCSA case {}", case.id);
                let _ = writeln!(
                    output,
                    "{},{},{},{},false,NONE,BLOCKED_{observed}",
                    case.id, case.code, case.expected, observed
                );
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_four_classifier_codes_preserve_distinct_maturity() {
        assert_eq!(
            DcsaEventClassifier::parse("EST").expect("EST").maturity(),
            DcsaEventMaturity::EstimatedOnly
        );
        assert_eq!(
            DcsaEventClassifier::parse("REQ").expect("REQ").maturity(),
            DcsaEventMaturity::RequestedOnly
        );
        assert_eq!(
            DcsaEventClassifier::parse("PLN").expect("PLN").maturity(),
            DcsaEventMaturity::PlannedOnly
        );
        assert_eq!(
            DcsaEventClassifier::parse("ACT").expect("ACT").maturity(),
            DcsaEventMaturity::SourceReportedActualOnly
        );
    }

    #[test]
    fn actual_is_never_promoted_to_independent_physical_observation() {
        let receipt = fixture_event("ACT")
            .admit_as_of(FIXTURE_AS_OF)
            .expect("valid source-reported ACT event");
        assert_eq!(receipt.maturity, DcsaEventMaturity::SourceReportedActualOnly);
        assert!(!receipt.independent_physical_observation);
        assert!(!receipt.maturity.independent_physical_observation());
    }

    #[test]
    fn planned_estimated_and_requested_events_never_map_to_actual() {
        for code in ["EST", "REQ", "PLN"] {
            let receipt = fixture_event(code)
                .admit_as_of(FIXTURE_AS_OF)
                .expect("valid non-actual DCSA event");
            assert_ne!(receipt.maturity, DcsaEventMaturity::SourceReportedActualOnly);
        }
    }

    #[test]
    fn unknown_classifier_and_standard_version_drift_are_rejected() {
        let mut event = fixture_event("XYZ");
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::UnknownClassifier("XYZ".to_owned()))
        );
        event = fixture_event("ACT");
        event.standard_version = "DCSA-TT-2.1".to_owned();
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::WrongStandardVersion)
        );
    }

    #[test]
    fn future_planned_time_is_not_mistaken_for_future_physical_actual() {
        let mut event = fixture_event("PLN");
        event.event_time_unix = Some(FIXTURE_AS_OF + 10);
        let receipt = event
            .admit_as_of(FIXTURE_AS_OF)
            .expect("planned event can have a future event time");
        assert_eq!(receipt.maturity, DcsaEventMaturity::PlannedOnly);
    }

    #[test]
    fn publication_after_retrieval_is_rejected_as_invalid_envelope() {
        let mut event = fixture_event("ACT");
        event.published_at_unix = Some(FIXTURE_RETRIEVED + 10);
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::PublicationAfterRetrieval)
        );
    }

    #[test]
    fn historical_asof_requires_publication_recording_and_retrieval() {
        let mut event = fixture_event("ACT");
        event.published_at_unix = None;
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::PublicationTimeUnknown)
        );
        event = fixture_event("ACT");
        event.retrieved_at_unix = FIXTURE_AS_OF + 1;
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::EvidenceUnavailableAtAsOf)
        );
        event = fixture_event("ACT");
        event.record_time_unix = FIXTURE_AS_OF + 1;
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::EvidenceUnavailableAtAsOf)
        );
    }

    #[test]
    fn actual_requires_event_time_and_all_events_retain_source_identity() {
        let mut event = fixture_event("ACT");
        event.event_time_unix = None;
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::EventTimeUnknown)
        );
        event = fixture_event("PLN");
        event.source_record_id.clear();
        assert_eq!(
            event.admit_as_of(FIXTURE_AS_OF),
            Err(DcsaClassifierError::EmptyIdentity)
        );
    }

    #[test]
    fn event_time_record_time_and_timezone_remain_distinct() {
        let receipt = fixture_event("ACT")
            .admit_as_of(FIXTURE_AS_OF)
            .expect("valid receipt");
        assert_eq!(receipt.event_time_unix, Some(FIXTURE_EVENT_TIME));
        assert_eq!(receipt.record_time_unix, FIXTURE_RETRIEVED);
        assert_eq!(receipt.event_time_zone_offset, "+02:00");
    }

    #[test]
    fn benchmark_receipt_is_deterministic_and_all_known_answers_match() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 12);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 7);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("ESTIMATED_ONLY"));
        assert!(first.contains("REQUESTED_ONLY"));
        assert!(first.contains("PLANNED_ONLY"));
        assert!(first.contains("SOURCE_REPORTED_ACTUAL_ONLY"));
        assert!(first.contains("independent_physical_observation"));
    }
}
