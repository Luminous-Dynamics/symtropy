use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EntityKind {
    ContainerBox,
    VesselVisit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingStatus {
    Active,
    Revoked,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityMapping {
    pub mapping_id: String,
    pub entity_kind: EntityKind,
    pub source_id: String,
    pub local_id: String,
    pub canonical_id: String,
    pub valid_from: i64,
    pub valid_until: Option<i64>,
    pub revision: u32,
    pub published_at: i64,
    pub status: MappingStatus,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Milestone {
    DischargedFromVessel,
    LoadedToFeeder,
    DischargedFromFeeder,
    LoadedToRail,
    ArrivedInland,
    DeliveredToReceiver,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EventClassification {
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
pub struct CargoEvent {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
    pub published_at: i64,
    pub event_time: i64,
    pub entity_kind: EntityKind,
    pub local_entity_id: String,
    pub milestone: Milestone,
    pub classification: EventClassification,
    pub evidence_kind: EvidenceKind,
    pub evidence_id: String,
    pub voided: bool,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum IdentityError {
    Unmatched,
    OutOfWindow,
    Revoked,
    Ambiguous,
    ConflictingMappingRevision(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationError {
    ConflictingEventRevision {
        source_id: String,
        source_event_id: String,
        revision: u32,
    },
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
    pub resolved_actual_event_rows: usize,
    pub unresolved_actual_event_rows: usize,
    pub unqualified_delivery_claims: usize,
    pub receiver_receipt_delivery_events: usize,
    pub receipt_backed_box_ids: BTreeSet<String>,
    pub unresolved_reasons: BTreeMap<IdentityError, usize>,
}

impl EvaluationReport {
    pub fn unique_receipt_backed_boxes(&self) -> usize {
        self.receipt_backed_box_ids.len()
    }
}

fn parse_entity_kind(value: &str) -> Result<EntityKind, String> {
    match value {
        "CONTAINER_BOX" => Ok(EntityKind::ContainerBox),
        "VESSEL_VISIT" => Ok(EntityKind::VesselVisit),
        _ => Err(format!("unknown entity_kind: {value}")),
    }
}

fn parse_mapping_status(value: &str) -> Result<MappingStatus, String> {
    match value {
        "ACTIVE" => Ok(MappingStatus::Active),
        "REVOKED" => Ok(MappingStatus::Revoked),
        _ => Err(format!("unknown mapping status: {value}")),
    }
}

fn parse_milestone(value: &str) -> Result<Milestone, String> {
    match value {
        "DISCHARGED_FROM_VESSEL" => Ok(Milestone::DischargedFromVessel),
        "LOADED_TO_FEEDER" => Ok(Milestone::LoadedToFeeder),
        "DISCHARGED_FROM_FEEDER" => Ok(Milestone::DischargedFromFeeder),
        "LOADED_TO_RAIL" => Ok(Milestone::LoadedToRail),
        "ARRIVED_INLAND" => Ok(Milestone::ArrivedInland),
        "DELIVERED_TO_RECEIVER" => Ok(Milestone::DeliveredToReceiver),
        _ => Err(format!("unknown milestone: {value}")),
    }
}

fn parse_classification(value: &str) -> Result<EventClassification, String> {
    match value {
        "PLANNED" => Ok(EventClassification::Planned),
        "ACTUAL" => Ok(EventClassification::Actual),
        _ => Err(format!("unknown event classification: {value}")),
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

pub fn parse_crosswalk_tsv(input: &str) -> Result<Vec<IdentityMapping>, String> {
    const HEADER: &str = "mapping_id\tentity_kind\tsource_id\tlocal_id\tcanonical_id\tvalid_from\tvalid_until\trevision\tpublished_at\tstatus\tevidence_id";
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) {
        return Err("crosswalk header mismatch".to_string());
    }

    let mut mappings = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 11 {
            return Err(format!("crosswalk row {} has {} fields, expected 11", index + 2, fields.len()));
        }
        for (field_index, field_name) in [
            (0, "mapping_id"),
            (2, "source_id"),
            (3, "local_id"),
            (4, "canonical_id"),
            (10, "evidence_id"),
        ] {
            require_non_empty(fields[field_index], field_name)?;
        }
        let valid_from: i64 = parse_number(fields[5], "valid_from")?;
        let valid_until = if fields[6] == "*" {
            None
        } else {
            Some(parse_number(fields[6], "valid_until")?)
        };
        if valid_until.is_some_and(|end| end <= valid_from) {
            return Err(format!("crosswalk row {} has a non-positive validity window", index + 2));
        }
        let revision: u32 = parse_number(fields[7], "mapping revision")?;
        if revision == 0 {
            return Err(format!("crosswalk row {} has revision zero", index + 2));
        }
        mappings.push(IdentityMapping {
            mapping_id: fields[0].to_string(),
            entity_kind: parse_entity_kind(fields[1])?,
            source_id: fields[2].to_string(),
            local_id: fields[3].to_string(),
            canonical_id: fields[4].to_string(),
            valid_from,
            valid_until,
            revision,
            published_at: parse_number(fields[8], "mapping publication time")?,
            status: parse_mapping_status(fields[9])?,
            evidence_id: fields[10].to_string(),
        });
    }
    Ok(mappings)
}

pub fn parse_events_tsv(input: &str) -> Result<Vec<CargoEvent>, String> {
    const HEADER: &str = "source_id\tsource_event_id\trevision\tpublished_at\tevent_time\tentity_kind\tlocal_entity_id\tmilestone\tclassification\tevidence_kind\tevidence_id\tvoided";
    let mut lines = input.lines();
    if lines.next() != Some(HEADER) {
        return Err("event header mismatch".to_string());
    }

    let mut events = Vec::new();
    for (index, raw_line) in lines.enumerate() {
        let line = raw_line.trim_end_matches('\r');
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != 12 {
            return Err(format!("event row {} has {} fields, expected 12", index + 2, fields.len()));
        }
        for (field_index, field_name) in [
            (0, "source_id"),
            (1, "source_event_id"),
            (6, "local_entity_id"),
            (10, "evidence_id"),
        ] {
            require_non_empty(fields[field_index], field_name)?;
        }
        let revision: u32 = parse_number(fields[2], "event revision")?;
        if revision == 0 {
            return Err(format!("event row {} has revision zero", index + 2));
        }
        let voided = match fields[11] {
            "true" => true,
            "false" => false,
            other => return Err(format!("invalid voided flag: {other}")),
        };
        events.push(CargoEvent {
            source_id: fields[0].to_string(),
            source_event_id: fields[1].to_string(),
            revision,
            published_at: parse_number(fields[3], "event publication time")?,
            event_time: parse_number(fields[4], "event time")?,
            entity_kind: parse_entity_kind(fields[5])?,
            local_entity_id: fields[6].to_string(),
            milestone: parse_milestone(fields[7])?,
            classification: parse_classification(fields[8])?,
            evidence_kind: parse_evidence_kind(fields[9])?,
            evidence_id: fields[10].to_string(),
            voided,
        });
    }
    Ok(events)
}

fn effective_mappings<'a>(
    mappings: &'a [IdentityMapping],
    as_of: i64,
) -> Result<Vec<&'a IdentityMapping>, IdentityError> {
    let mut by_mapping_id: BTreeMap<&str, Vec<&IdentityMapping>> = BTreeMap::new();
    for mapping in mappings.iter().filter(|mapping| mapping.published_at <= as_of) {
        by_mapping_id.entry(&mapping.mapping_id).or_default().push(mapping);
    }

    let mut effective = Vec::new();
    for (mapping_id, revisions) in by_mapping_id {
        let latest_revision = revisions
            .iter()
            .map(|mapping| mapping.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&IdentityMapping> = revisions
            .into_iter()
            .filter(|mapping| mapping.revision == latest_revision)
            .collect();
        let first = latest[0];
        if latest.iter().any(|candidate| *candidate != first) {
            return Err(IdentityError::ConflictingMappingRevision(mapping_id.to_string()));
        }
        effective.push(first);
    }
    Ok(effective)
}

pub fn resolve_identity(
    mappings: &[IdentityMapping],
    entity_kind: EntityKind,
    source_id: &str,
    local_id: &str,
    event_time: i64,
    as_of: i64,
) -> Result<String, IdentityError> {
    let effective = effective_mappings(mappings, as_of)?;
    let aliases: Vec<&IdentityMapping> = effective
        .into_iter()
        .filter(|mapping| {
            mapping.entity_kind == entity_kind
                && mapping.source_id == source_id
                && mapping.local_id == local_id
        })
        .collect();

    if aliases.is_empty() {
        return Err(IdentityError::Unmatched);
    }

    let in_window: Vec<&IdentityMapping> = aliases
        .into_iter()
        .filter(|mapping| {
            mapping.valid_from <= event_time
                && mapping.valid_until.is_none_or(|valid_until| event_time < valid_until)
        })
        .collect();

    if in_window.is_empty() {
        return Err(IdentityError::OutOfWindow);
    }
    if in_window.iter().any(|mapping| mapping.status == MappingStatus::Revoked) {
        return Err(IdentityError::Revoked);
    }

    let canonical_ids: BTreeSet<String> = in_window
        .into_iter()
        .filter(|mapping| mapping.status == MappingStatus::Active)
        .map(|mapping| mapping.canonical_id.clone())
        .collect();

    if canonical_ids.is_empty() {
        return Err(IdentityError::Revoked);
    }
    if canonical_ids.len() > 1 {
        return Err(IdentityError::Ambiguous);
    }
    Ok(canonical_ids.into_iter().next().unwrap_or_default())
}

pub fn evaluate(
    mappings: &[IdentityMapping],
    events: &[CargoEvent],
    as_of: i64,
) -> Result<EvaluationReport, EvaluationError> {
    let mut report = EvaluationReport {
        input_event_rows: events.len(),
        ..EvaluationReport::default()
    };

    let mut by_event_key: BTreeMap<(&str, &str), Vec<&CargoEvent>> = BTreeMap::new();
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

    let mut effective_events = Vec::new();
    for ((source_id, source_event_id), revisions) in by_event_key {
        let latest_revision = revisions
            .iter()
            .map(|event| event.revision)
            .max()
            .unwrap_or_default();
        let latest: Vec<&CargoEvent> = revisions
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
            effective_events.push(first);
        }
    }

    for event in effective_events {
        match event.classification {
            EventClassification::Planned => {
                report.planned_event_rows += 1;
                continue;
            }
            EventClassification::Actual => report.actual_event_rows += 1,
        }

        match resolve_identity(
            mappings,
            event.entity_kind,
            &event.source_id,
            &event.local_entity_id,
            event.event_time,
            as_of,
        ) {
            Ok(canonical_id) => {
                report.resolved_actual_event_rows += 1;
                if event.milestone == Milestone::DeliveredToReceiver {
                    if event.entity_kind == EntityKind::ContainerBox
                        && event.evidence_kind == EvidenceKind::ReceiverReceipt
                    {
                        report.receiver_receipt_delivery_events += 1;
                        report.receipt_backed_box_ids.insert(canonical_id);
                    } else {
                        report.unqualified_delivery_claims += 1;
                    }
                }
            }
            Err(reason) => {
                report.unresolved_actual_event_rows += 1;
                *report.unresolved_reasons.entry(reason).or_default() += 1;
            }
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Vec<IdentityMapping>, Vec<CargoEvent>) {
        let mappings = parse_crosswalk_tsv(include_str!("../data/crosswalk.tsv")).unwrap();
        let events = parse_events_tsv(include_str!("../data/events.tsv")).unwrap();
        (mappings, events)
    }

    #[test]
    fn frozen_fixture_matches_independent_golden_counts() {
        let (mappings, events) = fixtures();
        let report = evaluate(&mappings, &events, 200).unwrap();

        assert_eq!(report.input_event_rows, 11);
        assert_eq!(report.visible_event_rows, 10);
        assert_eq!(report.future_event_rows, 1);
        assert_eq!(report.superseded_event_rows, 0);
        assert_eq!(report.replayed_event_rows, 1);
        assert_eq!(report.planned_event_rows, 1);
        assert_eq!(report.actual_event_rows, 8);
        assert_eq!(report.resolved_actual_event_rows, 7);
        assert_eq!(report.unresolved_actual_event_rows, 1);
        assert_eq!(report.unqualified_delivery_claims, 1);
        assert_eq!(report.receiver_receipt_delivery_events, 1);
        assert_eq!(report.unique_receipt_backed_boxes(), 1);
        assert!(report.receipt_backed_box_ids.contains("BOX-0001"));
        assert_eq!(
            report.unresolved_reasons.get(&IdentityError::Unmatched),
            Some(&1)
        );
    }

    #[test]
    fn two_source_aliases_for_one_box_count_as_one_delivery() {
        let (mappings, mut events) = fixtures();
        let mut second_receipt = events
            .iter()
            .find(|event| event.source_event_id == "RC-E-006")
            .unwrap()
            .clone();
        second_receipt.source_id = "SRC_FEEDER".to_string();
        second_receipt.source_event_id = "FEED-E-011".to_string();
        second_receipt.local_entity_id = "FEED-BOX-A".to_string();
        events.push(second_receipt);

        let report = evaluate(&mappings, &events, 200).unwrap();
        assert_eq!(report.receiver_receipt_delivery_events, 2);
        assert_eq!(report.unique_receipt_backed_boxes(), 1);
    }

    #[test]
    fn planned_delivery_is_not_a_realized_delivery() {
        let (mappings, events) = fixtures();
        let planned = events
            .into_iter()
            .filter(|event| event.source_event_id == "RC-E-007")
            .collect::<Vec<_>>();
        let report = evaluate(&mappings, &planned, 200).unwrap();

        assert_eq!(report.planned_event_rows, 1);
        assert_eq!(report.actual_event_rows, 0);
        assert_eq!(report.unique_receipt_backed_boxes(), 0);
    }

    #[test]
    fn provider_delivery_claim_is_not_a_receiver_receipt() {
        let (mappings, events) = fixtures();
        let provider_claim = events
            .into_iter()
            .filter(|event| event.source_event_id == "DGT-E-008")
            .collect::<Vec<_>>();
        let report = evaluate(&mappings, &provider_claim, 200).unwrap();

        assert_eq!(report.unqualified_delivery_claims, 1);
        assert_eq!(report.unique_receipt_backed_boxes(), 0);
    }

    #[test]
    fn unresolved_identity_cannot_be_counted_as_delivered() {
        let (mappings, events) = fixtures();
        let unknown = events
            .into_iter()
            .filter(|event| event.source_event_id == "RC-E-009")
            .collect::<Vec<_>>();
        let report = evaluate(&mappings, &unknown, 200).unwrap();

        assert_eq!(report.unresolved_actual_event_rows, 1);
        assert_eq!(report.unique_receipt_backed_boxes(), 0);
    }

    #[test]
    fn time_bounded_alias_reuse_resolves_by_event_time() {
        let (mut mappings, _) = fixtures();
        mappings.push(IdentityMapping {
            mapping_id: "M-REUSE-A".to_string(),
            entity_kind: EntityKind::ContainerBox,
            source_id: "SRC_TEST".to_string(),
            local_id: "REUSED-ID".to_string(),
            canonical_id: "BOX-OLD".to_string(),
            valid_from: 0,
            valid_until: Some(50),
            revision: 1,
            published_at: 10,
            status: MappingStatus::Active,
            evidence_id: "EV-OLD".to_string(),
        });
        mappings.push(IdentityMapping {
            mapping_id: "M-REUSE-B".to_string(),
            entity_kind: EntityKind::ContainerBox,
            source_id: "SRC_TEST".to_string(),
            local_id: "REUSED-ID".to_string(),
            canonical_id: "BOX-NEW".to_string(),
            valid_from: 50,
            valid_until: None,
            revision: 1,
            published_at: 10,
            status: MappingStatus::Active,
            evidence_id: "EV-NEW".to_string(),
        });

        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_TEST", "REUSED-ID", 20, 200),
            Ok("BOX-OLD".to_string())
        );
        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_TEST", "REUSED-ID", 60, 200),
            Ok("BOX-NEW".to_string())
        );
    }

    #[test]
    fn overlapping_aliases_to_different_boxes_are_ambiguous() {
        let (mut mappings, _) = fixtures();
        mappings.push(IdentityMapping {
            mapping_id: "M-AMBIGUOUS".to_string(),
            entity_kind: EntityKind::ContainerBox,
            source_id: "SRC_DGT".to_string(),
            local_id: "DGT-BOX-A".to_string(),
            canonical_id: "BOX-WRONG".to_string(),
            valid_from: 0,
            valid_until: None,
            revision: 1,
            published_at: 10,
            status: MappingStatus::Active,
            evidence_id: "EV-AMBIGUOUS".to_string(),
        });

        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_DGT", "DGT-BOX-A", 6, 200),
            Err(IdentityError::Ambiguous)
        );
    }

    #[test]
    fn revoked_mapping_blocks_resolution() {
        let (mut mappings, _) = fixtures();
        let mut revoked = mappings
            .iter()
            .find(|mapping| mapping.mapping_id == "M-001")
            .unwrap()
            .clone();
        revoked.revision = 2;
        revoked.published_at = 50;
        revoked.status = MappingStatus::Revoked;
        revoked.evidence_id = "EV-REVOKE".to_string();
        mappings.push(revoked);

        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_DGT", "DGT-BOX-A", 6, 200),
            Err(IdentityError::Revoked)
        );
        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_DGT", "DGT-BOX-A", 6, 40),
            Ok("BOX-0001".to_string())
        );
    }

    #[test]
    fn same_mapping_revision_with_conflicting_target_is_rejected() {
        let (mut mappings, _) = fixtures();
        let mut conflict = mappings[0].clone();
        conflict.canonical_id = "BOX-CONFLICT".to_string();
        mappings.push(conflict);

        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_DGT", "DGT-BOX-A", 6, 200),
            Err(IdentityError::ConflictingMappingRevision("M-001".to_string()))
        );
    }

    #[test]
    fn alias_outside_its_validity_window_is_not_resolved() {
        let (mut mappings, _) = fixtures();
        mappings.push(IdentityMapping {
            mapping_id: "M-EXPIRED".to_string(),
            entity_kind: EntityKind::ContainerBox,
            source_id: "SRC_TEST".to_string(),
            local_id: "EXPIRED-ID".to_string(),
            canonical_id: "BOX-EXPIRED".to_string(),
            valid_from: 0,
            valid_until: Some(10),
            revision: 1,
            published_at: 5,
            status: MappingStatus::Active,
            evidence_id: "EV-EXPIRED".to_string(),
        });

        assert_eq!(
            resolve_identity(&mappings, EntityKind::ContainerBox, "SRC_TEST", "EXPIRED-ID", 10, 200),
            Err(IdentityError::OutOfWindow)
        );
    }

    #[test]
    fn conflicting_reuse_of_a_source_event_id_is_rejected() {
        let (mappings, mut events) = fixtures();
        let mut conflict = events[0].clone();
        conflict.milestone = Milestone::LoadedToFeeder;
        events.push(conflict);

        assert_eq!(
            evaluate(&mappings, &events, 200),
            Err(EvaluationError::ConflictingEventRevision {
                source_id: "SRC_DGT".to_string(),
                source_event_id: "DGT-E-001".to_string(),
                revision: 1,
            })
        );
    }

    #[test]
    fn latest_visible_void_revision_supersedes_prior_event() {
        let (mappings, mut events) = fixtures();
        let mut void_revision = events
            .iter()
            .find(|event| event.source_event_id == "DGT-E-008")
            .unwrap()
            .clone();
        void_revision.revision = 2;
        void_revision.published_at = 40;
        void_revision.voided = true;
        events.push(void_revision);

        let report = evaluate(&mappings, &events, 200).unwrap();
        assert_eq!(report.superseded_event_rows, 1);
        assert_eq!(report.voided_event_rows, 1);
        assert_eq!(report.unqualified_delivery_claims, 0);
    }

    #[test]
    fn source_aliases_do_not_cross_entity_kinds() {
        let (mappings, _) = fixtures();
        assert_eq!(
            resolve_identity(&mappings, EntityKind::VesselVisit, "SRC_DGT", "DGT-BOX-A", 6, 200),
            Err(IdentityError::Unmatched)
        );
    }

    #[test]
    fn invalid_windows_and_missing_evidence_references_are_rejected() {
        let bad_window = include_str!("../data/crosswalk.tsv")
            .replace("DGT-BOX-A\tBOX-0001\t0\t*", "DGT-BOX-A\tBOX-0001\t20\t10");
        assert!(parse_crosswalk_tsv(&bad_window).is_err());

        let missing_evidence = include_str!("../data/events.tsv")
            .replace("RECEIVER_RECEIPT\tRECEIPT-EV-006\tfalse", "RECEIVER_RECEIPT\t\tfalse");
        assert!(parse_events_tsv(&missing_evidence).is_err());
    }

    #[test]
    fn tsv_contract_rejects_unknown_schema() {
        assert!(parse_crosswalk_tsv("mapping_id\tunknown\nrow\tvalue").is_err());
        assert!(parse_events_tsv("source_id\tsource_event_id\nrow\tvalue").is_err());
    }
}
