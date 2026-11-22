//! Exact, evidence-referenced correlation of source-local release/acceptance IDs.
//!
//! Different systems may assign different local handoff IDs to the same
//! physical transfer. This module permits a canonical correlation only when
//! a receipt binds the exact release and acceptance event keys, canonical box,
//! parties, place, transport leg, quantity/unit and mapping profile. It never
//! fuzzy-matches by time, vessel, location or identifier similarity.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    temporal_order::{TimeEvidence, TemporalRelation, compare_time_evidence},
    Classification, CustodyEvent, EvaluationError, EvaluationReport, EventKind, HandoffField,
    evaluate,
};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SourceEventRef {
    pub source_id: String,
    pub source_event_id: String,
}

impl SourceEventRef {
    pub fn new(source_id: impl Into<String>, source_event_id: impl Into<String>) -> Self {
        Self {
            source_id: source_id.into(),
            source_event_id: source_event_id.into(),
        }
    }
}

/// An explicit correlation decision linking two source-local event identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandoffCorrelationReceipt {
    pub receipt_id: String,
    pub canonical_handoff_id: String,
    pub canonical_box_id: String,
    pub release_event: SourceEventRef,
    pub accept_event: SourceEventRef,
    pub release_local_handoff_id: String,
    pub accept_local_handoff_id: String,
    pub from_party: String,
    pub to_party: String,
    pub location_id: String,
    pub transport_leg_id: String,
    pub quantity: u32,
    pub unit: String,
    pub mapping_profile_id: String,
    pub mapping_profile_version: String,
    pub evidence_id: String,
}

/// Event-time evidence bound to one exact source event revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventTimeBinding {
    pub source_event: SourceEventRef,
    pub revision: u32,
    pub evidence: TimeEvidence,
}

/// A handoff correlation with release/acceptance time proof attached.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedHandoffCorrelationReceipt {
    pub correlation: HandoffCorrelationReceipt,
    pub release_time: EventTimeBinding,
    pub accept_time: EventTimeBinding,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemporalCorrelationError {
    ReleaseTimeBindingMismatch,
    AcceptTimeBindingMismatch,
    ReleaseRevisionMismatch,
    AcceptRevisionMismatch,
    EventTimeEvidenceIdMismatch,
    OrderNotProven(TemporalRelation),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QualifiedCorrelationError {
    Correlation(CorrelationError),
    Temporal(TemporalCorrelationError),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CorrelationError {
    EmptyField(&'static str),
    DuplicateReceiptId(String),
    DuplicateCanonicalHandoffId(String),
    MissingReferencedEvent(SourceEventRef),
    ConflictingEventRevision(SourceEventRef),
    ReferencedEventNotEffective(SourceEventRef),
    ReferencedEventNotActual(SourceEventRef),
    ReferencedEventWrongKind(SourceEventRef),
    ReferencedBoxMismatch,
    LocalHandoffIdMismatch(SourceEventRef),
    HandoffFieldMismatch(HandoffField),
    HandoffTimeNotIncreasing,
    DuplicateReleaseMapping(SourceEventRef),
    DuplicateAcceptanceMapping(SourceEventRef),
    Evaluation(EvaluationError),
}

fn require_non_empty(value: &str, name: &'static str) -> Result<(), CorrelationError> {
    if value.trim().is_empty() || value.trim() != value {
        return Err(CorrelationError::EmptyField(name));
    }
    Ok(())
}

fn effective_event_index(
    events: &[CustodyEvent],
    reference: &SourceEventRef,
    as_of: i64,
) -> Result<usize, CorrelationError> {
    let matching: Vec<usize> = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event.source_id == reference.source_id
                && event.source_event_id == reference.source_event_id
                && event.published_at <= as_of
        })
        .map(|(index, _)| index)
        .collect();

    if matching.is_empty() {
        return Err(CorrelationError::MissingReferencedEvent(reference.clone()));
    }

    let latest_revision = matching
        .iter()
        .map(|index| events[*index].revision)
        .max()
        .unwrap_or_default();
    let latest: Vec<usize> = matching
        .into_iter()
        .filter(|index| events[*index].revision == latest_revision)
        .collect();
    let first = latest[0];

    if latest.iter().any(|index| events[*index] != events[first]) {
        return Err(CorrelationError::ConflictingEventRevision(
            reference.clone(),
        ));
    }

    let event = &events[first];
    if event.voided {
        return Err(CorrelationError::ReferencedEventNotEffective(
            reference.clone(),
        ));
    }
    if event.classification != Classification::Actual {
        return Err(CorrelationError::ReferencedEventNotActual(
            reference.clone(),
        ));
    }

    Ok(first)
}

fn require_equal_field(
    receipt_value: &str,
    release_value: &str,
    accept_value: &str,
    field: HandoffField,
) -> Result<(), CorrelationError> {
    if receipt_value != release_value || receipt_value != accept_value {
        return Err(CorrelationError::HandoffFieldMismatch(field));
    }
    Ok(())
}

/// Validate receipts and return a cloned event tape in which only the exact
/// referenced release/acceptance event pair receives its canonical handoff
/// identifier. Raw source-local identifiers remain unchanged in the receipt.
pub fn apply_handoff_correlations(
    events: &[CustodyEvent],
    receipts: &[HandoffCorrelationReceipt],
    as_of: i64,
) -> Result<Vec<CustodyEvent>, CorrelationError> {
    let mut receipt_ids = BTreeSet::new();
    let mut canonical_ids = BTreeSet::new();
    let mut mapped_releases = BTreeSet::new();
    let mut mapped_acceptances = BTreeSet::new();
    let mut rewrites: BTreeMap<usize, String> = BTreeMap::new();

    for receipt in receipts {
        for (value, name) in [
            (receipt.receipt_id.as_str(), "receipt_id"),
            (
                receipt.canonical_handoff_id.as_str(),
                "canonical_handoff_id",
            ),
            (receipt.canonical_box_id.as_str(), "canonical_box_id"),
            (
                receipt.release_local_handoff_id.as_str(),
                "release_local_handoff_id",
            ),
            (
                receipt.accept_local_handoff_id.as_str(),
                "accept_local_handoff_id",
            ),
            (receipt.from_party.as_str(), "from_party"),
            (receipt.to_party.as_str(), "to_party"),
            (receipt.location_id.as_str(), "location_id"),
            (receipt.transport_leg_id.as_str(), "transport_leg_id"),
            (receipt.unit.as_str(), "unit"),
            (receipt.mapping_profile_id.as_str(), "mapping_profile_id"),
            (
                receipt.mapping_profile_version.as_str(),
                "mapping_profile_version",
            ),
            (receipt.evidence_id.as_str(), "evidence_id"),
            (
                receipt.release_event.source_id.as_str(),
                "release_source_id",
            ),
            (
                receipt.release_event.source_event_id.as_str(),
                "release_source_event_id",
            ),
            (receipt.accept_event.source_id.as_str(), "accept_source_id"),
            (
                receipt.accept_event.source_event_id.as_str(),
                "accept_source_event_id",
            ),
        ] {
            require_non_empty(value, name)?;
        }

        if !receipt_ids.insert(receipt.receipt_id.clone()) {
            return Err(CorrelationError::DuplicateReceiptId(
                receipt.receipt_id.clone(),
            ));
        }
        if !canonical_ids.insert((
            receipt.canonical_box_id.clone(),
            receipt.canonical_handoff_id.clone(),
        )) {
            return Err(CorrelationError::DuplicateCanonicalHandoffId(
                receipt.canonical_handoff_id.clone(),
            ));
        }
        if !mapped_releases.insert(receipt.release_event.clone()) {
            return Err(CorrelationError::DuplicateReleaseMapping(
                receipt.release_event.clone(),
            ));
        }
        if !mapped_acceptances.insert(receipt.accept_event.clone()) {
            return Err(CorrelationError::DuplicateAcceptanceMapping(
                receipt.accept_event.clone(),
            ));
        }

        let release_index = effective_event_index(events, &receipt.release_event, as_of)?;
        let accept_index = effective_event_index(events, &receipt.accept_event, as_of)?;
        let release = &events[release_index];
        let accept = &events[accept_index];

        if release.kind != EventKind::Release {
            return Err(CorrelationError::ReferencedEventWrongKind(
                receipt.release_event.clone(),
            ));
        }
        if accept.kind != EventKind::Accept {
            return Err(CorrelationError::ReferencedEventWrongKind(
                receipt.accept_event.clone(),
            ));
        }
        if release.event_time >= accept.event_time {
            return Err(CorrelationError::HandoffTimeNotIncreasing);
        }
        if release.box_id != accept.box_id || release.box_id != receipt.canonical_box_id {
            return Err(CorrelationError::ReferencedBoxMismatch);
        }
        if release.handoff_id != receipt.release_local_handoff_id {
            return Err(CorrelationError::LocalHandoffIdMismatch(
                receipt.release_event.clone(),
            ));
        }
        if accept.handoff_id != receipt.accept_local_handoff_id {
            return Err(CorrelationError::LocalHandoffIdMismatch(
                receipt.accept_event.clone(),
            ));
        }

        require_equal_field(
            &receipt.from_party,
            &release.from_party,
            &accept.from_party,
            HandoffField::FromParty,
        )?;
        require_equal_field(
            &receipt.to_party,
            &release.to_party,
            &accept.to_party,
            HandoffField::ToParty,
        )?;
        require_equal_field(
            &receipt.location_id,
            &release.location_id,
            &accept.location_id,
            HandoffField::Location,
        )?;
        require_equal_field(
            &receipt.transport_leg_id,
            &release.transport_leg_id,
            &accept.transport_leg_id,
            HandoffField::TransportLeg,
        )?;
        if release.quantity != receipt.quantity || accept.quantity != receipt.quantity {
            return Err(CorrelationError::HandoffFieldMismatch(
                HandoffField::Quantity,
            ));
        }
        require_equal_field(
            &receipt.unit,
            &release.unit,
            &accept.unit,
            HandoffField::Unit,
        )?;

        rewrites.insert(release_index, receipt.canonical_handoff_id.clone());
        rewrites.insert(accept_index, receipt.canonical_handoff_id.clone());
    }

    let mut canonicalized = events.to_vec();
    for (index, canonical_handoff_id) in rewrites {
        canonicalized[index].handoff_id = canonical_handoff_id;
    }
    Ok(canonicalized)
}

/// Stable, length-prefixed event key for binding temporal evidence to a
/// source event revision without delimiter ambiguity.
pub fn event_time_evidence_id(source_event: &SourceEventRef, revision: u32) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        source_event.source_id.len(),
        source_event.source_id,
        source_event.source_event_id.len(),
        source_event.source_event_id,
        revision
    )
}

/// Require proof-grade strict ordering for the exact release/acceptance
/// revisions before applying the base correlation receipt.
///
/// Estimated, overlapping, touching and incomparable time evidence all
/// refuse the correlation. The referenced clock calibration remains an
/// upstream claim carried by TimeEvidence; this function does not authenticate
/// or independently audit it.
pub fn apply_qualified_handoff_correlations(
    events: &[CustodyEvent],
    receipts: &[QualifiedHandoffCorrelationReceipt],
    as_of: i64,
) -> Result<Vec<CustodyEvent>, QualifiedCorrelationError> {
    let mut plain_receipts = Vec::with_capacity(receipts.len());

    for receipt in receipts {
        if receipt.release_time.source_event != receipt.correlation.release_event {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::ReleaseTimeBindingMismatch,
            ));
        }
        if receipt.accept_time.source_event != receipt.correlation.accept_event {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::AcceptTimeBindingMismatch,
            ));
        }
        if receipt.release_time.revision == 0 {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::ReleaseRevisionMismatch,
            ));
        }
        if receipt.accept_time.revision == 0 {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::AcceptRevisionMismatch,
            ));
        }
        if receipt.release_time.evidence.event_id()
            != event_time_evidence_id(
                &receipt.release_time.source_event,
                receipt.release_time.revision,
            )
            || receipt.accept_time.evidence.event_id()
                != event_time_evidence_id(
                    &receipt.accept_time.source_event,
                    receipt.accept_time.revision,
                )
        {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::EventTimeEvidenceIdMismatch,
            ));
        }

        let release_index = effective_event_index(
            events,
            &receipt.release_time.source_event,
            as_of,
        )
        .map_err(QualifiedCorrelationError::Correlation)?;
        let accept_index = effective_event_index(
            events,
            &receipt.accept_time.source_event,
            as_of,
        )
        .map_err(QualifiedCorrelationError::Correlation)?;

        if events[release_index].revision != receipt.release_time.revision {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::ReleaseRevisionMismatch,
            ));
        }
        if events[accept_index].revision != receipt.accept_time.revision {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::AcceptRevisionMismatch,
            ));
        }

        let relation = compare_time_evidence(
            &receipt.release_time.evidence,
            &receipt.accept_time.evidence,
        );
        if relation != TemporalRelation::ProvenBefore {
            return Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::OrderNotProven(relation),
            ));
        }

        plain_receipts.push(receipt.correlation.clone());
    }

    apply_handoff_correlations(events, &plain_receipts, as_of)
        .map_err(QualifiedCorrelationError::Correlation)
}

/// P0 synthetic evaluator. Event-time ticks are dimensionless fixture values;
/// this path is not a proof-grade cross-source chronology qualification.
pub fn evaluate_with_synthetic_handoff_correlations(
    expectations: &[crate::CustodyExpectation],
    events: &[CustodyEvent],
    receipts: &[HandoffCorrelationReceipt],
    as_of: i64,
) -> Result<EvaluationReport, CorrelationError> {
    let canonicalized = apply_handoff_correlations(events, receipts, as_of)?;
    evaluate(expectations, &canonicalized, as_of).map_err(CorrelationError::Evaluation)
}

/// P1 evaluator. Each exact release/acceptance event pair must carry
/// proof-grade interval evidence establishing strict release-before-acceptance.
pub fn evaluate_with_handoff_correlations(
    expectations: &[crate::CustodyExpectation],
    events: &[CustodyEvent],
    receipts: &[QualifiedHandoffCorrelationReceipt],
    as_of: i64,
) -> Result<EvaluationReport, QualifiedCorrelationError> {
    let canonicalized = apply_qualified_handoff_correlations(events, receipts, as_of)?;
    evaluate(expectations, &canonicalized, as_of)
        .map_err(CorrelationError::Evaluation)
        .map_err(QualifiedCorrelationError::Correlation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChainFailure;
    use crate::parse_events_tsv;
    use crate::parse_expectations_tsv;

    fn fixtures() -> (Vec<crate::CustodyExpectation>, Vec<CustodyEvent>) {
        (
            parse_expectations_tsv(include_str!("../data/expectations.tsv")).unwrap(),
            parse_events_tsv(include_str!("../data/events.tsv")).unwrap(),
        )
    }

    fn receipt(release: &CustodyEvent, accept: &CustodyEvent) -> HandoffCorrelationReceipt {
        HandoffCorrelationReceipt {
            receipt_id: "CORR-001".to_string(),
            canonical_handoff_id: "CANON-HANDOFF-001".to_string(),
            canonical_box_id: release.box_id.clone(),
            release_event: SourceEventRef::new(
                release.source_id.clone(),
                release.source_event_id.clone(),
            ),
            accept_event: SourceEventRef::new(
                accept.source_id.clone(),
                accept.source_event_id.clone(),
            ),
            release_local_handoff_id: release.handoff_id.clone(),
            accept_local_handoff_id: accept.handoff_id.clone(),
            from_party: release.from_party.clone(),
            to_party: release.to_party.clone(),
            location_id: release.location_id.clone(),
            transport_leg_id: release.transport_leg_id.clone(),
            quantity: release.quantity,
            unit: release.unit.clone(),
            mapping_profile_id: "DGT_FEEDER_HANDOFF".to_string(),
            mapping_profile_version: "1.0".to_string(),
            evidence_id: "OPAQUE-CORRELATION-EVIDENCE".to_string(),
        }
    }

    fn qualified_time_binding(
        event_ref: &SourceEventRef,
        revision: u32,
        timestamp: i64,
        error_ticks: u64,
    ) -> EventTimeBinding {
        let profile = super::super::temporal_order::ClockBoundProfile::new(
            super::super::temporal_order::ClockBoundProfileSpec {
                clock_id: format!("CLOCK-{}", event_ref.source_id),
                profile_id: "CLOCK_PROFILE_V1".to_string(),
                time_basis_id: "UTC_NS_TEST_V1".to_string(),
                calibration_evidence_id: "CALIBRATION-OPAQUE".to_string(),
                synchronization_method: "SYNTHETIC_REFERENCE".to_string(),
                sync_state: super::super::temporal_order::ClockSyncState::Locked,
                calibrated_at: -10_000,
                valid_until: 10_000,
                max_offset_error_ticks: Some(error_ticks),
                max_holdover_drift_ticks: Some(0),
                max_capture_path_ticks: Some(0),
                max_rounding_error_ticks: Some(0),
            },
        )
        .unwrap();
        let evidence_id = event_time_evidence_id(event_ref, revision);
        EventTimeBinding {
            source_event: event_ref.clone(),
            revision,
            evidence: profile.qualify_event(evidence_id, timestamp).unwrap(),
        }
    }

    fn qualified_receipt(
        release: &CustodyEvent,
        accept: &CustodyEvent,
        release_time: i64,
        accept_time: i64,
        error_ticks: u64,
    ) -> QualifiedHandoffCorrelationReceipt {
        let base = receipt(release, accept);
        let release_ref = base.release_event.clone();
        let accept_ref = base.accept_event.clone();
        QualifiedHandoffCorrelationReceipt {
            correlation: base,
            release_time: qualified_time_binding(
                &release_ref,
                release.revision,
                release_time,
                error_ticks,
            ),
            accept_time: qualified_time_binding(
                &accept_ref,
                accept.revision,
                accept_time,
                error_ticks,
            ),
        }
    }

    #[test]
    fn qualified_correlation_requires_proven_release_before_acceptance() {
        let (_, mut source_events) = fixtures();
        let release = source_events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let mut accept = source_events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        accept.handoff_id = "FEEDER-LOCAL-77".to_string();
        for event in &mut source_events {
            if event.source_event_id == "B1-A1" {
                event.handoff_id = "FEEDER-LOCAL-77".to_string();
            }
        }

        let qualified = qualified_receipt(&release, &accept, 100, 110, 2);
        let canonicalized =
            apply_qualified_handoff_correlations(&source_events, &[qualified], 200).unwrap();

        assert!(canonicalized.iter().any(|event| {
            event.source_event_id == "B1-R1" && event.handoff_id == "CANON-HANDOFF-001"
        }));
        assert!(canonicalized.iter().any(|event| {
            event.source_event_id == "B1-A1" && event.handoff_id == "CANON-HANDOFF-001"
        }));
    }

    #[test]
    fn proof_grade_evaluator_counts_only_after_time_and_correlation_qualification() {
        let (expectations, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let qualified = qualified_receipt(&release, &accept, 100, 110, 2);

        let report =
            evaluate_with_handoff_correlations(&expectations, &events, &[qualified], 200)
                .unwrap();
        assert_eq!(report.complete_chains(), 1);
        assert_eq!(report.incomplete_box_ids.len(), 1);
    }

    #[test]
    fn overlapping_or_touching_intervals_cannot_qualify_a_handoff() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let qualified = qualified_receipt(&release, &accept, 100, 110, 5);

        assert_eq!(
            apply_qualified_handoff_correlations(&events, &[qualified], 200),
            Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::OrderNotProven(TemporalRelation::UnresolvedOverlap)
            ))
        );
    }

    #[test]
    fn reversed_time_enclosures_cannot_qualify_a_handoff() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let qualified = qualified_receipt(&release, &accept, 120, 100, 2);

        assert_eq!(
            apply_qualified_handoff_correlations(&events, &[qualified], 200),
            Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::OrderNotProven(TemporalRelation::ProvenAfter)
            ))
        );
    }

    #[test]
    fn estimate_only_clock_bounds_do_not_establish_handoff_order() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let mut qualified = qualified_receipt(&release, &accept, 100, 120, 0);
        qualified.release_time.evidence = TimeEvidence::new(
            qualified.release_time.evidence.event_id().to_string(),
            qualified.release_time.evidence.time_basis_id().to_string(),
            qualified.release_time.evidence.interval(),
            super::super::temporal_order::TimeBoundKind::EstimateOnly,
        )
        .unwrap();

        assert_eq!(
            apply_qualified_handoff_correlations(&events, &[qualified], 200),
            Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::OrderNotProven(TemporalRelation::EstimatedBefore)
            ))
        );
    }

    #[test]
    fn temporal_binding_must_match_exact_source_event_revision() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let mut qualified = qualified_receipt(&release, &accept, 100, 120, 0);
        qualified.release_time.revision = 2;

        assert_eq!(
            apply_qualified_handoff_correlations(&events, &[qualified], 200),
            Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::ReleaseRevisionMismatch
            ))
        );
    }

    #[test]
    fn time_evidence_cannot_be_replayed_for_another_source_event() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let mut qualified = qualified_receipt(&release, &accept, 100, 120, 0);
        qualified.accept_time.evidence = qualified.release_time.evidence.clone();

        assert_eq!(
            apply_qualified_handoff_correlations(&events, &[qualified], 200),
            Err(QualifiedCorrelationError::Temporal(
                TemporalCorrelationError::EventTimeEvidenceIdMismatch
            ))
        );
    }

    #[test]
    fn exact_receipt_correlates_different_source_local_handoff_ids() {
        let (expectations, mut events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let mut accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        accept.handoff_id = "FEEDER-LOCAL-77".to_string();
        for event in &mut events {
            if event.source_event_id == "B1-A1" {
                event.handoff_id = "FEEDER-LOCAL-77".to_string();
            }
        }

        let correlation = receipt(&release, &accept);
        let report =
            evaluate_with_synthetic_handoff_correlations(&expectations, &events, &[correlation], 200)
                .unwrap();

        assert_eq!(report.complete_chains(), 1);
        assert!(report.complete_box_ids.contains("BOX-0001"));
    }

    #[test]
    fn mismatched_local_ids_without_receipt_are_not_fuzzy_matched() {
        let (expectations, mut events) = fixtures();
        for event in &mut events {
            if event.source_event_id == "B1-A1" {
                event.handoff_id = "FEEDER-LOCAL-77".to_string();
            }
        }

        let report = evaluate(&expectations, &events, 200).unwrap();
        assert_eq!(report.complete_chains(), 0);
        assert_eq!(
            report
                .failure_counts
                .get(&ChainFailure::HandoffFieldMismatch(HandoffField::HandoffId)),
            Some(&1)
        );
    }

    #[test]
    fn synthetic_correlation_rejects_non_increasing_source_event_times() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let correlation = receipt(&release, &accept);
        let mut reversed = events.clone();
        for event in &mut reversed {
            if event.source_event_id == "B1-A1" {
                event.event_time = 20;
            }
        }

        assert_eq!(
            apply_handoff_correlations(&reversed, &[correlation], 200),
            Err(CorrelationError::HandoffTimeNotIncreasing)
        );
    }

    #[test]
    fn receipt_cannot_repair_wrong_location() {
        let (expectations, mut events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let mut accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        accept.location_id = "WRONG_LOCATION".to_string();
        for event in &mut events {
            if event.source_event_id == "B1-A1" {
                event.location_id = "WRONG_LOCATION".to_string();
            }
        }
        let correlation = receipt(&release, &accept);

        assert_eq!(
            evaluate_with_handoff_correlations(&expectations, &events, &[correlation], 200),
            Err(CorrelationError::HandoffFieldMismatch(
                HandoffField::Location
            ))
        );
    }

    #[test]
    fn receipt_must_reference_the_exact_release_event_key() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let mut correlation = receipt(&release, &accept);
        correlation.release_event.source_event_id = "OTHER-RELEASE".to_string();

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::MissingReferencedEvent(
                SourceEventRef::new("SRC_DGT", "OTHER-RELEASE")
            ))
        );
    }

    #[test]
    fn latest_voided_release_cannot_be_used_by_correlation_receipt() {
        let (_, mut events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let correlation = receipt(&release, &accept);
        let mut void_revision = release.clone();
        void_revision.revision = 2;
        void_revision.published_at = 100;
        void_revision.voided = true;
        events.push(void_revision);

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::ReferencedEventNotEffective(
                SourceEventRef::new("SRC_DGT", "B1-R1")
            ))
        );
    }

    #[test]
    fn planned_acceptance_cannot_be_correlated_as_a_physical_handoff() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B2-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B2-A1-PLANNED")
            .unwrap()
            .clone();
        let correlation = receipt(&release, &accept);

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::ReferencedEventNotActual(
                SourceEventRef::new("SRC_FEEDER", "B2-A1-PLANNED")
            ))
        );
    }

    #[test]
    fn duplicate_release_mapping_is_rejected() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let first = receipt(&release, &accept);
        let mut second = first.clone();
        second.receipt_id = "CORR-002".to_string();
        second.canonical_handoff_id = "CANON-HANDOFF-002".to_string();

        assert_eq!(
            apply_handoff_correlations(&events, &[first, second], 200),
            Err(CorrelationError::DuplicateReleaseMapping(
                SourceEventRef::new("SRC_DGT", "B1-R1")
            ))
        );
    }

    #[test]
    fn event_for_different_canonical_box_cannot_be_correlated() {
        let (_, mut events) = fixtures();
        let mut release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        release.box_id = "OTHER-BOX".to_string();
        for event in &mut events {
            if event.source_event_id == "B1-R1" {
                event.box_id = "OTHER-BOX".to_string();
            }
        }
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let correlation = receipt(&release, &accept);

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::ReferencedBoxMismatch)
        );
    }

    #[test]
    fn conflicting_latest_event_revision_cannot_be_correlated() {
        let (_, mut events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let correlation = receipt(&release, &accept);
        let mut first_revision = release.clone();
        first_revision.revision = 2;
        first_revision.published_at = 100;
        let mut conflicting_revision = first_revision.clone();
        conflicting_revision.box_id = "BOX-SUBSTITUTED".to_string();
        events.push(first_revision);
        events.push(conflicting_revision);

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::ConflictingEventRevision(
                SourceEventRef::new("SRC_DGT", "B1-R1")
            ))
        );
    }

    #[test]
    fn empty_evidence_or_mapping_profile_is_rejected() {
        let (_, events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        let mut correlation = receipt(&release, &accept);
        correlation.evidence_id.clear();

        assert_eq!(
            apply_handoff_correlations(&events, &[correlation], 200),
            Err(CorrelationError::EmptyField("evidence_id"))
        );
    }

    #[test]
    fn correlation_does_not_mutate_original_source_events() {
        let (_, mut events) = fixtures();
        let release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        let mut accept = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .clone();
        for event in &mut events {
            if event.source_event_id == "B1-A1" {
                event.handoff_id = "FEEDER-LOCAL-77".to_string();
                accept.handoff_id = "FEEDER-LOCAL-77".to_string();
            }
        }
        let correlation = receipt(&release, &accept);
        let original_release_id = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .handoff_id
            .clone();
        let original_accept_id = events
            .iter()
            .find(|event| event.source_event_id == "B1-A1")
            .unwrap()
            .handoff_id
            .clone();

        let canonicalized = apply_handoff_correlations(&events, &[correlation], 200).unwrap();
        assert_eq!(
            events
                .iter()
                .find(|event| event.source_event_id == "B1-R1")
                .unwrap()
                .handoff_id,
            original_release_id
        );
        assert_eq!(
            events
                .iter()
                .find(|event| event.source_event_id == "B1-A1")
                .unwrap()
                .handoff_id,
            original_accept_id
        );
        assert_eq!(
            canonicalized
                .iter()
                .find(|event| event.source_event_id == "B1-A1")
                .unwrap()
                .handoff_id,
            "CANON-HANDOFF-001"
        );
    }
}
