//! Exact, evidence-referenced correlation of source-local release/acceptance IDs.
//!
//! Different systems may assign different local handoff IDs to the same
//! physical transfer. This module permits a canonical correlation only when
//! a receipt binds the exact release and acceptance event keys, canonical box,
//! parties, place, transport leg, quantity/unit and mapping profile. It never
//! fuzzy-matches by time, vessel, location or identifier similarity.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    CargoEvent, Classification, EntityKind, EvaluationError, EvaluationReport, EventKind,
    HandoffField, evaluate,
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
    events: &[CargoEvent],
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
    events: &[CargoEvent],
    receipts: &[HandoffCorrelationReceipt],
    as_of: i64,
) -> Result<Vec<CargoEvent>, CorrelationError> {
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
        if release.entity_kind != EntityKind::ContainerBox
            || accept.entity_kind != EntityKind::ContainerBox
            || release.local_entity_id != accept.local_entity_id
            || release.local_entity_id != receipt.canonical_box_id
        {
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

/// Reconcile source-local handoff IDs only through exact correlation receipts,
/// then run the existing fail-closed custody evaluator.
pub fn evaluate_with_handoff_correlations(
    expectations: &[super::CustodyExpectation],
    events: &[CargoEvent],
    receipts: &[HandoffCorrelationReceipt],
    as_of: i64,
) -> Result<EvaluationReport, CorrelationError> {
    let canonicalized = apply_handoff_correlations(events, receipts, as_of)?;
    evaluate(expectations, &canonicalized, as_of).map_err(CorrelationError::Evaluation)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ChainFailure;
    use crate::parse_events_tsv;
    use crate::parse_expectations_tsv;

    fn fixtures() -> (Vec<super::CustodyExpectation>, Vec<CargoEvent>) {
        (
            parse_expectations_tsv(include_str!("../data/expectations.tsv")).unwrap(),
            parse_events_tsv(include_str!("../data/events.tsv")).unwrap(),
        )
    }

    fn receipt(release: &CargoEvent, accept: &CargoEvent) -> HandoffCorrelationReceipt {
        HandoffCorrelationReceipt {
            receipt_id: "CORR-001".to_string(),
            canonical_handoff_id: "CANON-HANDOFF-001".to_string(),
            canonical_box_id: release.local_entity_id.clone(),
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
            evaluate_with_handoff_correlations(&expectations, &events, &[correlation], 200)
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
        let (_, mut events) = fixtures();
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
    fn vessel_visit_event_cannot_be_correlated_as_a_container_box_handoff() {
        let (_, mut events) = fixtures();
        let mut release = events
            .iter()
            .find(|event| event.source_event_id == "B1-R1")
            .unwrap()
            .clone();
        release.entity_kind = EntityKind::VesselVisit;
        for event in &mut events {
            if event.source_event_id == "B1-R1" {
                event.entity_kind = EntityKind::VesselVisit;
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
        let mut conflict = release.clone();
        conflict.local_entity_id = "BOX-SUBSTITUTED".to_string();
        conflict.revision = 2;
        conflict.published_at = 100;
        events.push(conflict);

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
