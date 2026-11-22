pub mod temporal_order;

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventKind {
    Open,
    Release,
    Accept,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Classification {
    Planned,
    Actual,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceKind {
    ProviderEvent,
    IndependentObservation,
    ReceiverReceipt,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustodyExpectation {
    pub box_id: String,
    pub initial_custodian: String,
    pub expected_receiver: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CustodyEvent {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
    pub published_at: i64,
    pub event_time: i64,
    pub box_id: String,
    pub handoff_id: String,
    pub kind: EventKind,
    pub from_party: String,
    pub to_party: String,
    pub location_id: String,
    pub transport_leg_id: String,
    pub quantity: u32,
    pub unit: String,
    pub classification: Classification,
    pub evidence_kind: EvidenceKind,
    pub evidence_id: String,
    pub voided: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum HandoffField {
    HandoffId,
    FromParty,
    ToParty,
    Location,
    TransportLeg,
    Quantity,
    Unit,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ChainFailure {
    MissingOpening,
    DuplicateOpening,
    InvalidOpening,
    WrongInitialCustodian,
    WrongCustodian,
    OverlappingRelease,
    UnmatchedAcceptance,
    HandoffFieldMismatch(HandoffField),
    AcceptanceNotAfterRelease,
    AmbiguousEventTime,
    PendingHandoff,
    HandoffIdReused,
    InvalidQuantity,
    InvalidHandoff,
    MissingEvidenceReference,
    FinalReceiverWithoutReceipt,
    OpeningNotIndependent,
    NotAtExpectedReceiver,
    UnknownBox,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    ConflictingEventRevision {
        source_id: String,
        source_event_id: String,
        revision: u32,
    },
    DuplicateExpectation(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChainStatus {
    Complete,
    Incomplete,
    Rejected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChainOutcome {
    pub status: ChainStatus,
    pub failure: Option<ChainFailure>,
    pub receiver_receipt_time: Option<i64>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvaluationReport {
    pub input_event_rows: usize,
    pub visible_event_rows: usize,
    pub future_event_rows: usize,
    pub superseded_event_rows: usize,
    pub replayed_event_rows: usize,
    pub voided_event_rows: usize,
    pub planned_event_rows: usize,
    pub actual_event_rows: usize,
    pub complete_box_ids: BTreeSet<String>,
    pub incomplete_box_ids: BTreeSet<String>,
    pub rejected_box_ids: BTreeSet<String>,
    pub outcomes: BTreeMap<String, ChainOutcome>,
    pub failure_counts: BTreeMap<ChainFailure, usize>,
    pub unknown_box_events: usize,
}

impl EvaluationReport {
    pub fn complete_chains(&self) -> usize {
        self.complete_box_ids.len()
    }
}

fn require_non_empty(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("empty required field: {field}"))
    } else {
        Ok(())
    }
}

fn parse_number<T: std::str::FromStr>(value: &str, field: &str) -> Result<T, String> {
    value
        .parse::<T>()
        .map_err(|_| format!("invalid {field}: {value}"))
}

fn parse_event_kind(value: &str) -> Result<EventKind, String> {
    match value {
        "OPEN" => Ok(EventKind::Open),
        "RELEASE" => Ok(EventKind::Release),
        "ACCEPT" => Ok(EventKind::Accept),
        _ => Err(format!("unknown event kind: {value}")),
    }
}

fn parse_classification(value: &str) -> Result<Classification, String> {
    match value {
        "PLANNED" => Ok(Classification::Planned),
        "ACTUAL" => Ok(Classification::Actual),
        _ => Err(format!("unknown classification: {value}")),
    }
}

fn parse_evidence_kind(value: &str) -> Result<EvidenceKind, String> {
    match value {
        "PROVIDER_EVENT" => Ok(EvidenceKind::ProviderEvent),
        "INDEPENDENT_OBSERVATION" => Ok(EvidenceKind::IndependentObservation),
        "RECEIVER_RECEIPT" => Ok(EvidenceKind::ReceiverReceipt),
        _ => Err(format!("unknown evidence kind: {value}")),
    }
}

pub fn parse_expectations_tsv(input: &str) -> Result<Vec<CustodyExpectation>, String> {
    const HEADER: &str = "box_id\tinitial_custodian\texpected_receiver";
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) {
        return Err("expectation header mismatch".to_string());
    }

    let mut rows = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 3 {
            return Err(format!(
                "expectation row {} has {} fields, expected 3",
                index + 2,
                fields.len()
            ));
        }
        for (value, name) in [
            (fields[0], "box_id"),
            (fields[1], "initial_custodian"),
            (fields[2], "expected_receiver"),
        ] {
            require_non_empty(value, name)?;
        }
        if fields[1] == fields[2] {
            return Err(format!(
                "expectation row {} has identical custodians",
                index + 2
            ));
        }
        rows.push(CustodyExpectation {
            box_id: fields[0].to_string(),
            initial_custodian: fields[1].to_string(),
            expected_receiver: fields[2].to_string(),
        });
    }
    Ok(rows)
}

pub fn parse_events_tsv(input: &str) -> Result<Vec<CustodyEvent>, String> {
    const HEADER: &str = "source_id\tsource_event_id\trevision\tpublished_at\tevent_time\tbox_id\thandoff_id\tkind\tfrom_party\tto_party\tlocation_id\ttransport_leg_id\tquantity\tunit\tclassification\tevidence_kind\tevidence_id\tvoided";
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) {
        return Err("event header mismatch".to_string());
    }

    let mut rows = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 18 {
            return Err(format!(
                "event row {} has {} fields, expected 18",
                index + 2,
                fields.len()
            ));
        }
        for (value, name) in [
            (fields[0], "source_id"),
            (fields[1], "source_event_id"),
            (fields[5], "box_id"),
            (fields[6], "handoff_id"),
            (fields[8], "from_party"),
            (fields[9], "to_party"),
            (fields[10], "location_id"),
            (fields[11], "transport_leg_id"),
            (fields[13], "unit"),
            (fields[16], "evidence_id"),
        ] {
            require_non_empty(value, name)?;
        }
        let revision: u32 = parse_number(fields[2], "revision")?;
        if revision == 0 {
            return Err(format!("event row {} has revision zero", index + 2));
        }
        let quantity: u32 = parse_number(fields[12], "quantity")?;
        let voided = match fields[17] {
            "true" => true,
            "false" => false,
            other => return Err(format!("invalid voided flag: {other}")),
        };
        rows.push(CustodyEvent {
            source_id: fields[0].to_string(),
            source_event_id: fields[1].to_string(),
            revision,
            published_at: parse_number(fields[3], "published_at")?,
            event_time: parse_number(fields[4], "event_time")?,
            box_id: fields[5].to_string(),
            handoff_id: fields[6].to_string(),
            kind: parse_event_kind(fields[7])?,
            from_party: fields[8].to_string(),
            to_party: fields[9].to_string(),
            location_id: fields[10].to_string(),
            transport_leg_id: fields[11].to_string(),
            quantity,
            unit: fields[13].to_string(),
            classification: parse_classification(fields[14])?,
            evidence_kind: parse_evidence_kind(fields[15])?,
            evidence_id: fields[16].to_string(),
            voided,
        });
    }
    Ok(rows)
}

fn record_failure(
    report: &mut EvaluationReport,
    box_id: &str,
    failure: ChainFailure,
    status: ChainStatus,
) {
    *report.failure_counts.entry(failure.clone()).or_default() += 1;
    match status {
        ChainStatus::Complete => {
            report.complete_box_ids.insert(box_id.to_string());
        }
        ChainStatus::Incomplete => {
            report.incomplete_box_ids.insert(box_id.to_string());
        }
        ChainStatus::Rejected => {
            report.rejected_box_ids.insert(box_id.to_string());
        }
    }
    report.outcomes.insert(
        box_id.to_string(),
        ChainOutcome {
            status,
            failure: Some(failure),
            receiver_receipt_time: None,
        },
    );
}

fn validate_open(event: &CustodyEvent, expectation: &CustodyExpectation) -> Option<ChainFailure> {
    if event.handoff_id != "OPEN"
        || event.from_party != "-"
        || event.transport_leg_id != "-"
        || event.quantity != 1
        || event.unit != "BOX"
    {
        return Some(ChainFailure::InvalidOpening);
    }
    if event.to_party != expectation.initial_custodian {
        return Some(ChainFailure::WrongInitialCustodian);
    }
    if event.evidence_id.trim().is_empty() || event.location_id.trim().is_empty() {
        return Some(ChainFailure::MissingEvidenceReference);
    }
    if event.evidence_kind != EvidenceKind::IndependentObservation {
        return Some(ChainFailure::OpeningNotIndependent);
    }
    None
}

fn compare_handoff(release: &CustodyEvent, accept: &CustodyEvent) -> Option<ChainFailure> {
    if release.handoff_id != accept.handoff_id {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::HandoffId));
    }
    if release.from_party != accept.from_party {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::FromParty));
    }
    if release.to_party != accept.to_party {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::ToParty));
    }
    if release.location_id != accept.location_id {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::Location));
    }
    if release.transport_leg_id != accept.transport_leg_id {
        return Some(ChainFailure::HandoffFieldMismatch(
            HandoffField::TransportLeg,
        ));
    }
    if release.quantity != accept.quantity {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::Quantity));
    }
    if release.unit != accept.unit {
        return Some(ChainFailure::HandoffFieldMismatch(HandoffField::Unit));
    }
    None
}

fn evaluate_box(
    expectation: &CustodyExpectation,
    events: &[&CustodyEvent],
    report: &mut EvaluationReport,
) {
    let mut actual: Vec<&CustodyEvent> = events
        .iter()
        .copied()
        .filter(|event| event.classification == Classification::Actual)
        .collect();
    actual.sort_by_key(|event| event.event_time);

    if actual.is_empty() {
        record_failure(
            report,
            &expectation.box_id,
            ChainFailure::MissingOpening,
            ChainStatus::Incomplete,
        );
        return;
    }

    if actual
        .windows(2)
        .any(|pair| pair[0].event_time == pair[1].event_time)
    {
        record_failure(
            report,
            &expectation.box_id,
            ChainFailure::AmbiguousEventTime,
            ChainStatus::Rejected,
        );
        return;
    }

    if actual[0].kind != EventKind::Open {
        record_failure(
            report,
            &expectation.box_id,
            ChainFailure::MissingOpening,
            ChainStatus::Incomplete,
        );
        return;
    }

    let mut custodian: Option<String> = None;
    let mut pending: Option<&CustodyEvent> = None;
    let mut seen_handoffs = BTreeSet::new();
    let mut receiver_receipt_time = None;

    for (index, event_ref) in actual.iter().enumerate() {
        let event = *event_ref;
        if event.kind != EventKind::Accept && (event.quantity != 1 || event.unit != "BOX") {
            record_failure(
                report,
                &expectation.box_id,
                ChainFailure::InvalidQuantity,
                ChainStatus::Rejected,
            );
            return;
        }
        if event.evidence_id.trim().is_empty() {
            record_failure(
                report,
                &expectation.box_id,
                ChainFailure::MissingEvidenceReference,
                ChainStatus::Rejected,
            );
            return;
        }

        match event.kind {
            EventKind::Open => {
                if index != 0 || custodian.is_some() {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::DuplicateOpening,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                if let Some(failure) = validate_open(event, expectation) {
                    record_failure(report, &expectation.box_id, failure, ChainStatus::Rejected);
                    return;
                }
                custodian = Some(event.to_party.clone());
            }
            EventKind::Release => {
                if custodian.as_deref() != Some(event.from_party.as_str()) {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::WrongCustodian,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                if pending.is_some() {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::OverlappingRelease,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                if event.handoff_id == "OPEN" || !seen_handoffs.insert(event.handoff_id.clone()) {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::HandoffIdReused,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                if event.to_party == event.from_party
                    || event.to_party == "-"
                    || event.location_id == "-"
                    || event.transport_leg_id == "-"
                {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::InvalidHandoff,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                receiver_receipt_time = None;
                pending = Some(event);
            }
            EventKind::Accept => {
                let Some(release) = pending else {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::UnmatchedAcceptance,
                        ChainStatus::Rejected,
                    );
                    return;
                };
                if event.event_time <= release.event_time {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::AcceptanceNotAfterRelease,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                if let Some(failure) = compare_handoff(release, event) {
                    record_failure(report, &expectation.box_id, failure, ChainStatus::Rejected);
                    return;
                }
                if custodian.as_deref() != Some(release.from_party.as_str()) {
                    record_failure(
                        report,
                        &expectation.box_id,
                        ChainFailure::WrongCustodian,
                        ChainStatus::Rejected,
                    );
                    return;
                }
                custodian = Some(event.to_party.clone());
                pending = None;
                receiver_receipt_time = if event.to_party == expectation.expected_receiver
                    && event.evidence_kind == EvidenceKind::ReceiverReceipt
                {
                    Some(event.event_time)
                } else {
                    None
                };
            }
        }
    }

    if pending.is_some() {
        record_failure(
            report,
            &expectation.box_id,
            ChainFailure::PendingHandoff,
            ChainStatus::Incomplete,
        );
        return;
    }
    if custodian.as_deref() == Some(expectation.expected_receiver.as_str()) {
        if let Some(received_at) = receiver_receipt_time {
            report.complete_box_ids.insert(expectation.box_id.clone());
            report.outcomes.insert(
                expectation.box_id.clone(),
                ChainOutcome {
                    status: ChainStatus::Complete,
                    failure: None,
                    receiver_receipt_time: Some(received_at),
                },
            );
        } else {
            record_failure(
                report,
                &expectation.box_id,
                ChainFailure::FinalReceiverWithoutReceipt,
                ChainStatus::Rejected,
            );
        }
    } else {
        record_failure(
            report,
            &expectation.box_id,
            ChainFailure::NotAtExpectedReceiver,
            ChainStatus::Incomplete,
        );
    }
}

pub fn evaluate(
    expectations: &[CustodyExpectation],
    events: &[CustodyEvent],
    as_of: i64,
) -> Result<EvaluationReport, EvaluationError> {
    let mut report = EvaluationReport {
        input_event_rows: events.len(),
        ..EvaluationReport::default()
    };

    let mut expected_by_box = BTreeMap::new();
    for expectation in expectations {
        if expected_by_box
            .insert(expectation.box_id.as_str(), expectation)
            .is_some()
        {
            return Err(EvaluationError::DuplicateExpectation(
                expectation.box_id.clone(),
            ));
        }
    }

    let mut by_event_key: BTreeMap<(&str, &str), Vec<&CustodyEvent>> = BTreeMap::new();
    for event in events {
        if event.published_at <= as_of {
            report.visible_event_rows += 1;
            by_event_key
                .entry((&event.source_id, &event.source_event_id))
                .or_default()
                .push(event);
        } else {
            report.future_event_rows += 1;
        }
    }

    let mut effective = Vec::new();
    for ((source_id, source_event_id), revisions) in by_event_key {
        let latest_revision = revisions
            .iter()
            .map(|event| event.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&CustodyEvent> = revisions
            .iter()
            .copied()
            .filter(|event| event.revision == latest_revision)
            .collect();
        let first = latest[0];
        if latest.iter().any(|candidate| *candidate != first) {
            return Err(EvaluationError::ConflictingEventRevision {
                source_id: source_id.to_string(),
                source_event_id: source_event_id.to_string(),
                revision: latest_revision,
            });
        }
        report.replayed_event_rows += latest.len().saturating_sub(1);
        report.superseded_event_rows += revisions.len().saturating_sub(latest.len());
        if first.voided {
            report.voided_event_rows += 1;
        } else {
            effective.push(first);
        }
    }

    let mut by_box: BTreeMap<&str, Vec<&CustodyEvent>> = BTreeMap::new();
    for event in effective {
        match event.classification {
            Classification::Planned => report.planned_event_rows += 1,
            Classification::Actual => report.actual_event_rows += 1,
        }
        by_box.entry(&event.box_id).or_default().push(event);
    }

    for (box_id, events_for_unknown) in &by_box {
        if !expected_by_box.contains_key(*box_id) {
            report.unknown_box_events += events_for_unknown.len();
            record_failure(
                &mut report,
                box_id,
                ChainFailure::UnknownBox,
                ChainStatus::Rejected,
            );
        }
    }

    for (box_id, expectation) in expected_by_box {
        let box_events = by_box.get(box_id).map(Vec::as_slice).unwrap_or(&[]);
        evaluate_box(expectation, box_events, &mut report);
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Vec<CustodyExpectation>, Vec<CustodyEvent>) {
        let expectations =
            parse_expectations_tsv(include_str!("../data/expectations.tsv")).unwrap();
        let events = parse_events_tsv(include_str!("../data/events.tsv")).unwrap();
        (expectations, events)
    }

    fn event_mut<'a>(
        events: &'a mut [CustodyEvent],
        source_event_id: &str,
    ) -> &'a mut CustodyEvent {
        events
            .iter_mut()
            .find(|event| event.source_event_id == source_event_id)
            .unwrap()
    }

    #[test]
    fn frozen_fixture_has_one_complete_and_one_pending_box() {
        let (expectations, events) = fixtures();
        let report = evaluate(&expectations, &events, 200).unwrap();

        assert_eq!(report.input_event_rows, 11);
        assert_eq!(report.visible_event_rows, 10);
        assert_eq!(report.future_event_rows, 1);
        assert_eq!(report.planned_event_rows, 1);
        assert_eq!(report.actual_event_rows, 9);
        assert_eq!(report.complete_chains(), 1);
        assert!(report.complete_box_ids.contains("BOX-0001"));
        assert!(report.incomplete_box_ids.contains("BOX-0002"));
        assert_eq!(
            report.failure_counts.get(&ChainFailure::PendingHandoff),
            Some(&1)
        );
    }

    #[test]
    fn reverse_ingestion_order_produces_identical_reconstruction() {
        let (expectations, events) = fixtures();
        let forward = evaluate(&expectations, &events, 200).unwrap();
        let mut reversed_events = events;
        reversed_events.reverse();
        let reversed = evaluate(&expectations, &reversed_events, 200).unwrap();
        assert_eq!(forward, reversed);
    }

    #[test]
    fn exact_replay_is_idempotent() {
        let (expectations, mut events) = fixtures();
        events.push(events[0].clone());
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.complete_chains(), 1);
        assert_eq!(report.replayed_event_rows, 1);
    }

    #[test]
    fn wrong_custodian_cannot_release_the_box() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-R1").from_party = "UNRELATED".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert!(report.rejected_box_ids.contains("BOX-0001"));
        assert_eq!(
            report.failure_counts.get(&ChainFailure::WrongCustodian),
            Some(&1)
        );
    }

    #[test]
    fn acceptance_must_match_the_location() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A1").location_id = "OTHER_LOCATION".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::HandoffFieldMismatch(HandoffField::Location)),
            Some(&1)
        );
        assert_eq!(report.complete_chains(), 0);
    }

    #[test]
    fn acceptance_must_match_the_transport_leg() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A2").transport_leg_id = "WRONG_LEG".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::HandoffFieldMismatch(
                    HandoffField::TransportLeg
                )),
            Some(&1)
        );
    }

    #[test]
    fn missing_acceptance_remains_pending_not_delivered() {
        let (expectations, mut events) = fixtures();
        events.retain(|event| event.source_event_id != "B1-A3");
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.complete_chains(), 0);
        assert_eq!(
            report.failure_counts.get(&ChainFailure::PendingHandoff),
            Some(&2)
        );
    }

    #[test]
    fn overlapping_release_is_rejected() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A2").event_time = 45;
        let mut extra = event_mut(&mut events, "B1-R2").clone();
        extra.source_event_id = "B1-R-OVERLAP".to_string();
        extra.handoff_id = "H-OVERLAP".to_string();
        extra.event_time = 35;
        extra.from_party = "COASTAL_FEEDER".to_string();
        extra.to_party = "OTHER_CARRIER".to_string();
        events.push(extra);
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report.failure_counts.get(&ChainFailure::OverlappingRelease),
            Some(&1)
        );
    }

    #[test]
    fn acceptance_before_release_is_unmatched() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A1").event_time = 15;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::UnmatchedAcceptance),
            Some(&1)
        );
    }

    #[test]
    fn equal_event_times_are_not_arbitrarily_ordered() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A1").event_time = 20;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report.failure_counts.get(&ChainFailure::AmbiguousEventTime),
            Some(&1)
        );
    }

    #[test]
    fn planned_acceptance_does_not_close_a_real_release() {
        let (expectations, events) = fixtures();
        let only_planned = events
            .into_iter()
            .filter(|event| event.box_id == "BOX-0002")
            .collect::<Vec<_>>();
        let report = evaluate(&expectations, &only_planned, 200).unwrap();
        assert!(report.incomplete_box_ids.contains("BOX-0002"));
        assert_eq!(report.complete_chains(), 0);
    }

    #[test]
    fn provider_only_final_acceptance_is_not_receiver_receipt() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A3").evidence_kind = EvidenceKind::ProviderEvent;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::FinalReceiverWithoutReceipt),
            Some(&1)
        );
        assert_eq!(report.complete_chains(), 0);
    }

    #[test]
    fn receiver_receipt_is_invalidated_by_later_release() {
        let (expectations, mut events) = fixtures();
        let mut later_release = event_mut(&mut events, "B1-R3").clone();
        later_release.source_event_id = "B1-R4".to_string();
        later_release.event_time = 50;
        later_release.published_at = 50;
        later_release.handoff_id = "H4".to_string();
        later_release.from_party = "INLAND_RECEIVER".to_string();
        later_release.to_party = "NEXT_CARRIER".to_string();
        later_release.location_id = "GAUTENG_DEPOT".to_string();
        later_release.transport_leg_id = "LAST_MILE_02".to_string();
        events.push(later_release);

        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.complete_chains(), 0);
        assert_eq!(
            report.failure_counts.get(&ChainFailure::PendingHandoff),
            Some(&2)
        );
    }

    #[test]
    fn opening_custody_requires_independent_observation() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-OPEN").evidence_kind = EvidenceKind::ProviderEvent;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::OpeningNotIndependent),
            Some(&1)
        );
        assert_eq!(report.complete_chains(), 0);
    }

    #[test]
    fn wrong_final_receiver_does_not_complete_the_chain() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-R3").to_party = "WRONG_RECEIVER".to_string();
        event_mut(&mut events, "B1-A3").to_party = "WRONG_RECEIVER".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::NotAtExpectedReceiver),
            Some(&1)
        );
    }

    #[test]
    fn reused_handoff_id_is_rejected() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-R2").handoff_id = "H1".to_string();
        event_mut(&mut events, "B1-A2").handoff_id = "H1".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report.failure_counts.get(&ChainFailure::HandoffIdReused),
            Some(&1)
        );
    }

    #[test]
    fn voided_latest_release_leaves_acceptance_unmatched() {
        let (expectations, mut events) = fixtures();
        let mut void_revision = event_mut(&mut events, "B1-R1").clone();
        void_revision.revision = 2;
        void_revision.published_at = 100;
        void_revision.voided = true;
        events.push(void_revision);
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.voided_event_rows, 1);
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::UnmatchedAcceptance),
            Some(&1)
        );
    }

    #[test]
    fn same_revision_conflict_is_rejected() {
        let (expectations, mut events) = fixtures();
        let mut conflict = events[0].clone();
        conflict.to_party = "SOMEONE_ELSE".to_string();
        events.push(conflict);
        assert_eq!(
            evaluate(&expectations, &events, 200),
            Err(EvaluationError::ConflictingEventRevision {
                source_id: "SRC_DGT".to_string(),
                source_event_id: "B1-OPEN".to_string(),
                revision: 1,
            })
        );
    }

    #[test]
    fn future_published_acceptance_is_not_visible() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A3").published_at = 250;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.future_event_rows, 2);
        assert_eq!(report.complete_chains(), 0);
    }

    #[test]
    fn wrong_quantity_is_rejected() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A1").quantity = 2;
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::HandoffFieldMismatch(HandoffField::Quantity)),
            Some(&1)
        );
    }

    #[test]
    fn acceptance_unit_must_match_release() {
        let (expectations, mut events) = fixtures();
        event_mut(&mut events, "B1-A1").unit = "PALLET".to_string();
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::HandoffFieldMismatch(HandoffField::Unit)),
            Some(&1)
        );
    }

    #[test]
    fn duplicate_opening_is_rejected() {
        let (expectations, mut events) = fixtures();
        let mut extra = events[0].clone();
        extra.source_event_id = "B1-OPEN-2".to_string();
        extra.event_time = 2;
        events.push(extra);
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(
            report.failure_counts.get(&ChainFailure::DuplicateOpening),
            Some(&1)
        );
    }

    #[test]
    fn unknown_box_identity_is_not_repaired_by_guessing() {
        let (expectations, mut events) = fixtures();
        let mut extra = events[0].clone();
        extra.source_event_id = "BOX-UNKNOWN-OPEN".to_string();
        extra.box_id = "BOX-UNKNOWN".to_string();
        events.push(extra);
        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.unknown_box_events, 1);
        assert!(report.rejected_box_ids.contains("BOX-UNKNOWN"));
    }

    #[test]
    fn parser_rejects_unknown_schema_and_missing_evidence() {
        assert!(parse_expectations_tsv("box\tinitial\nA\tB").is_err());
        assert!(parse_events_tsv("source_id\tsource_event_id\nx\ty").is_err());
        let mut invalid = include_str!("../data/events.tsv").to_string();
        invalid = invalid.replace(
            "RECEIVER_RECEIPT\tR-007\tfalse",
            "RECEIVER_RECEIPT\t\tfalse",
        );
        assert!(parse_events_tsv(&invalid).is_err());
    }
}
