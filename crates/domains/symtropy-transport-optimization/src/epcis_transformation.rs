// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Loss-aware EPCIS 2.0.1 TransformationEvent semantics.
//!
//! A TransformationEvent is a many-to-many input/output relation. A shared
//! transformationID groups several events into one contribution scope; it
//! does not provide a per-input allocation matrix, group-closure proof, yield,
//! waste or material-balance result. QuantityElement semantics are shared with
//! the AggregationEvent reference module. This is a meaning-layer for an
//! already-parsed event, not an EPCIS wire-format parser or conformance claim.

use std::collections::BTreeMap;

use crate::epcis_aggregation::{
    AggregationSemanticError, QuantityElement, valid_epcis_offset, valid_identifier,
    validate_quantity_elements,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpcisTransformationEvent {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
    pub source_profile_id: String,
    pub source_profile_version: String,
    /// Exact transformationID string. Its URI syntax is delegated to the
    /// versioned wire-format/profile validator.
    pub transformation_id: Option<String>,
    /// Instance-level input EPCs. The standard treats the list as unordered.
    pub input_epcs: Vec<String>,
    /// Class-level input quantities, never expanded into instance IDs.
    pub input_quantity_list: Vec<QuantityElement>,
    /// Instance-level output EPCs. The standard treats the list as unordered.
    pub output_epcs: Vec<String>,
    /// Class-level output quantities, never expanded into instance IDs.
    pub output_quantity_list: Vec<QuantityElement>,
    /// Original source eventTime text; no UTC/TAI conversion is performed.
    pub event_time_lexeme: String,
    /// Original EPCIS eventTimeZoneOffset text.
    pub event_time_zone_offset: String,
    /// Repository bookkeeping time, distinct from physical eventTime.
    pub record_time_lexeme: Option<String>,
    /// Selected superclass context fields retained as opaque source values.
    pub biz_step: Option<String>,
    pub disposition: Option<String>,
    pub read_point_id: Option<String>,
    pub biz_location_id: Option<String>,
    /// Opaque source error-declaration content/reference, not interpreted here.
    pub error_declaration: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SourceEventRevisionRef {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformationSemanticError {
    EmptySourceIdentity,
    EmptySourceProfile,
    EmptyEventTime,
    InvalidEventTimeZoneOffset(String),
    EmptyRecordTime,
    InvalidOptionalContext(&'static str),
    InvalidTransformationId,
    InvalidInputIdentity(String),
    InvalidOutputIdentity(String),
    MissingInputs,
    MissingOutputs,
    MissingInputsAndOutputs,
    InvalidQuantityElement(String),
    ConflictingSourceEventRevision(SourceEventRevisionRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformationScope {
    /// With no transformationID, only this event's own inputs may contribute
    /// to this event's own outputs.
    EventLocal,
    /// Events with the identical exact transformationID share a many-to-many
    /// possible-contribution scope.
    SharedTransformation(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedTransformationEvent {
    source: SourceEventRevisionRef,
    source_profile_id: String,
    source_profile_version: String,
    transformation_id: Option<String>,
    input_epcs: Vec<String>,
    input_quantity_list: Vec<QuantityElement>,
    output_epcs: Vec<String>,
    output_quantity_list: Vec<QuantityElement>,
    event_time_lexeme: String,
    event_time_zone_offset: String,
    record_time_lexeme: Option<String>,
    biz_step: Option<String>,
    disposition: Option<String>,
    read_point_id: Option<String>,
    biz_location_id: Option<String>,
    error_declaration: Option<String>,
}

impl NormalizedTransformationEvent {
    pub fn source(&self) -> &SourceEventRevisionRef {
        &self.source
    }

    pub fn transformation_id(&self) -> Option<&str> {
        self.transformation_id.as_deref()
    }

    pub fn scope(&self) -> TransformationScope {
        match &self.transformation_id {
            Some(id) => TransformationScope::SharedTransformation(id.clone()),
            None => TransformationScope::EventLocal,
        }
    }

    pub fn input_epcs(&self) -> &[String] {
        &self.input_epcs
    }

    pub fn input_quantity_list(&self) -> &[QuantityElement] {
        &self.input_quantity_list
    }

    pub fn output_epcs(&self) -> &[String] {
        &self.output_epcs
    }

    pub fn output_quantity_list(&self) -> &[QuantityElement] {
        &self.output_quantity_list
    }

    pub fn event_time_lexeme(&self) -> &str {
        &self.event_time_lexeme
    }

    pub fn event_time_zone_offset(&self) -> &str {
        &self.event_time_zone_offset
    }

    pub fn record_time_lexeme(&self) -> Option<&str> {
        self.record_time_lexeme.as_deref()
    }

    pub fn source_profile(&self) -> (&str, &str) {
        (&self.source_profile_id, &self.source_profile_version)
    }

    pub fn business_context(&self) -> (Option<&str>, Option<&str>, Option<&str>, Option<&str>) {
        (
            self.biz_step.as_deref(),
            self.disposition.as_deref(),
            self.read_point_id.as_deref(),
            self.biz_location_id.as_deref(),
        )
    }

    pub fn error_declaration(&self) -> Option<&str> {
        self.error_declaration.as_deref()
    }
}

fn validate_epcs(values: &[String], input: bool) -> Result<(), TransformationSemanticError> {
    for value in values {
        if !valid_identifier(value) {
            return Err(if input {
                TransformationSemanticError::InvalidInputIdentity(value.clone())
            } else {
                TransformationSemanticError::InvalidOutputIdentity(value.clone())
            });
        }
    }
    Ok(())
}

fn validate_optional_context(
    value: Option<&String>,
    field: &'static str,
) -> Result<(), TransformationSemanticError> {
    if value.is_some_and(|value| !valid_identifier(value)) {
        return Err(TransformationSemanticError::InvalidOptionalContext(field));
    }
    Ok(())
}

/// Validate and preserve an already-parsed EPCIS 2.0.1 TransformationEvent.
///
/// Normative cardinality:
/// - without transformationID: at least one input AND one output;
/// - with transformationID: at least one input OR one output (or both).
///
/// Lists remain in their source order for faithful replay, but their order has
/// no contribution/causality meaning under EPCIS. Repeated values are
/// preserved; this layer does not silently deduplicate or aggregate them.
pub fn normalize_transformation_event(
    event: &EpcisTransformationEvent,
) -> Result<NormalizedTransformationEvent, TransformationSemanticError> {
    if !valid_identifier(&event.source_id) || !valid_identifier(&event.source_event_id) {
        return Err(TransformationSemanticError::EmptySourceIdentity);
    }
    if !valid_identifier(&event.source_profile_id)
        || !valid_identifier(&event.source_profile_version)
    {
        return Err(TransformationSemanticError::EmptySourceProfile);
    }
    if !valid_identifier(&event.event_time_lexeme) {
        return Err(TransformationSemanticError::EmptyEventTime);
    }
    if !valid_epcis_offset(&event.event_time_zone_offset) {
        return Err(TransformationSemanticError::InvalidEventTimeZoneOffset(
            event.event_time_zone_offset.clone(),
        ));
    }
    if event
        .record_time_lexeme
        .as_ref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return Err(TransformationSemanticError::EmptyRecordTime);
    }
    if event
        .transformation_id
        .as_ref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return Err(TransformationSemanticError::InvalidTransformationId);
    }

    for (value, field) in [
        (event.biz_step.as_ref(), "bizStep"),
        (event.disposition.as_ref(), "disposition"),
        (event.read_point_id.as_ref(), "readPoint.id"),
        (event.biz_location_id.as_ref(), "bizLocation.id"),
        (event.error_declaration.as_ref(), "errorDeclaration"),
    ] {
        validate_optional_context(value, field)?;
    }

    validate_epcs(&event.input_epcs, true)?;
    validate_epcs(&event.output_epcs, false)?;
    validate_quantity_elements(&event.input_quantity_list).map_err(map_quantity_error)?;
    validate_quantity_elements(&event.output_quantity_list).map_err(map_quantity_error)?;

    let has_inputs = !event.input_epcs.is_empty() || !event.input_quantity_list.is_empty();
    let has_outputs = !event.output_epcs.is_empty() || !event.output_quantity_list.is_empty();

    match event.transformation_id.as_ref() {
        None => {
            if !has_inputs {
                return Err(TransformationSemanticError::MissingInputs);
            }
            if !has_outputs {
                return Err(TransformationSemanticError::MissingOutputs);
            }
        }
        Some(_) if !has_inputs && !has_outputs => {
            return Err(TransformationSemanticError::MissingInputsAndOutputs);
        }
        Some(_) => {}
    }

    Ok(NormalizedTransformationEvent {
        source: SourceEventRevisionRef {
            source_id: event.source_id.clone(),
            source_event_id: event.source_event_id.clone(),
            revision: event.revision,
        },
        source_profile_id: event.source_profile_id.clone(),
        source_profile_version: event.source_profile_version.clone(),
        transformation_id: event.transformation_id.clone(),
        input_epcs: event.input_epcs.clone(),
        input_quantity_list: event.input_quantity_list.clone(),
        output_epcs: event.output_epcs.clone(),
        output_quantity_list: event.output_quantity_list.clone(),
        event_time_lexeme: event.event_time_lexeme.clone(),
        event_time_zone_offset: event.event_time_zone_offset.clone(),
        record_time_lexeme: event.record_time_lexeme.clone(),
        biz_step: event.biz_step.clone(),
        disposition: event.disposition.clone(),
        read_point_id: event.read_point_id.clone(),
        biz_location_id: event.biz_location_id.clone(),
        error_declaration: event.error_declaration.clone(),
    })
}

fn map_quantity_error(error: AggregationSemanticError) -> TransformationSemanticError {
    match error {
        AggregationSemanticError::InvalidQuantityElement(class_id) => {
            TransformationSemanticError::InvalidQuantityElement(class_id)
        }
        _ => TransformationSemanticError::InvalidQuantityElement(String::new()),
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TransformationGroupKey {
    EventLocal(SourceEventRevisionRef),
    SharedTransformation(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributedEpc {
    pub source_event: SourceEventRevisionRef,
    pub epc: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AttributedQuantity {
    pub source_event: SourceEventRevisionRef,
    pub quantity: QuantityElement,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationStatus {
    /// EPCIS establishes a possible many-to-many contribution scope but does
    /// not identify which individual input contributed to which output.
    UnallocatedManyToMany,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransformationClosure {
    /// A source event or a shared transformationID does not prove that the
    /// process is complete, all outputs are reported, or the group is closed.
    NotEstablished,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationGroup {
    pub key: TransformationGroupKey,
    pub member_events: Vec<SourceEventRevisionRef>,
    pub input_epcs: Vec<AttributedEpc>,
    pub input_quantities: Vec<AttributedQuantity>,
    pub output_epcs: Vec<AttributedEpc>,
    pub output_quantities: Vec<AttributedQuantity>,
    pub allocation: AllocationStatus,
    pub closure: TransformationClosure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransformationGroupingError {
    ConflictingSourceEventRevision(SourceEventRevisionRef),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationGroupingReport {
    pub input_event_rows: usize,
    pub unique_event_revisions: usize,
    pub exact_duplicate_replays: usize,
    pub groups: Vec<TransformationGroup>,
}

/// Group exact transformationID matches into a shared many-to-many relation.
///
/// An event without transformationID remains event-local. Exact duplicate
/// source-event revisions are idempotent; conflicting content at the same
/// source/event/revision identity rejects the whole grouping result.
pub fn build_transformation_groups(
    events: &[NormalizedTransformationEvent],
) -> Result<TransformationGroupingReport, TransformationGroupingError> {
    let mut by_source_revision: BTreeMap<SourceEventRevisionRef, NormalizedTransformationEvent> =
        BTreeMap::new();
    let mut exact_duplicate_replays = 0usize;

    for event in events {
        match by_source_revision.get(&event.source) {
            Some(previous) if previous == event => {
                exact_duplicate_replays += 1;
            }
            Some(_) => {
                return Err(TransformationGroupingError::ConflictingSourceEventRevision(
                    event.source.clone(),
                ));
            }
            None => {
                by_source_revision.insert(event.source.clone(), (*event).clone());
            }
        }
    }

    let mut members_by_group: BTreeMap<TransformationGroupKey, Vec<NormalizedTransformationEvent>> =
        BTreeMap::new();

    for event in by_source_revision.values() {
        let key = match &event.transformation_id {
            Some(id) => TransformationGroupKey::SharedTransformation(id.clone()),
            None => TransformationGroupKey::EventLocal(event.source.clone()),
        };
        members_by_group.entry(key).or_default().push(event.clone());
    }

    let mut groups = Vec::with_capacity(members_by_group.len());
    for (key, members) in members_by_group {
        let mut member_events = Vec::with_capacity(members.len());
        let mut input_epcs = Vec::new();
        let mut input_quantities = Vec::new();
        let mut output_epcs = Vec::new();
        let mut output_quantities = Vec::new();

        for event in members {
            member_events.push(event.source.clone());
            input_epcs.extend(event.input_epcs.iter().cloned().map(|epc| AttributedEpc {
                source_event: event.source.clone(),
                epc,
            }));
            input_quantities.extend(event.input_quantity_list.iter().cloned().map(|quantity| {
                AttributedQuantity {
                    source_event: event.source.clone(),
                    quantity,
                }
            }));
            output_epcs.extend(event.output_epcs.iter().cloned().map(|epc| AttributedEpc {
                source_event: event.source.clone(),
                epc,
            }));
            output_quantities.extend(event.output_quantity_list.iter().cloned().map(|quantity| {
                AttributedQuantity {
                    source_event: event.source.clone(),
                    quantity,
                }
            }));
        }

        groups.push(TransformationGroup {
            key,
            member_events,
            input_epcs,
            input_quantities,
            output_epcs,
            output_quantities,
            allocation: AllocationStatus::UnallocatedManyToMany,
            closure: TransformationClosure::NotEstablished,
        });
    }

    Ok(TransformationGroupingReport {
        input_event_rows: events.len(),
        unique_event_revisions: by_source_revision.len(),
        exact_duplicate_replays,
        groups,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::epcis_aggregation::QuantityElement;

    fn event() -> EpcisTransformationEvent {
        EpcisTransformationEvent {
            source_id: "WMS_A".to_string(),
            source_event_id: "urn:uuid:transform-001".to_string(),
            revision: 1,
            source_profile_id: "GS1_EPCIS".to_string(),
            source_profile_version: "2.0.1".to_string(),
            transformation_id: None,
            input_epcs: vec!["urn:epc:id:sgtin:input-1".to_string()],
            input_quantity_list: Vec::new(),
            output_epcs: vec!["urn:epc:id:sgtin:output-1".to_string()],
            output_quantity_list: Vec::new(),
            event_time_lexeme: "2026-11-22T10:00:00+02:00".to_string(),
            event_time_zone_offset: "+02:00".to_string(),
            record_time_lexeme: Some("2026-11-22T08:00:04Z".to_string()),
            biz_step: Some("commissioning".to_string()),
            disposition: Some("in_progress".to_string()),
            read_point_id: Some("urn:epc:id:sgln:12345.1.0".to_string()),
            biz_location_id: None,
            error_declaration: None,
        }
    }

    fn quantity(
        object_class_id: &str,
        quantity_lexeme: Option<&str>,
        unit_of_measure: Option<&str>,
    ) -> QuantityElement {
        QuantityElement {
            object_class_id: object_class_id.to_string(),
            quantity_lexeme: quantity_lexeme.map(str::to_string),
            unit_of_measure: unit_of_measure.map(str::to_string),
        }
    }

    fn normalized(
        source_event_id: &str,
        transformation_id: Option<&str>,
        inputs: &[&str],
        outputs: &[&str],
    ) -> NormalizedTransformationEvent {
        let mut input = event();
        input.source_event_id = source_event_id.to_string();
        input.transformation_id = transformation_id.map(str::to_string);
        input.input_epcs = inputs.iter().map(|value| (*value).to_string()).collect();
        input.output_epcs = outputs.iter().map(|value| (*value).to_string()).collect();
        normalize_transformation_event(&input).unwrap()
    }

    #[test]
    fn event_without_transformation_id_requires_inputs_and_outputs() {
        let mut input = event();
        assert!(normalize_transformation_event(&input).is_ok());

        input.output_epcs.clear();
        assert_eq!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::MissingOutputs)
        );

        input.output_epcs.push("output".to_string());
        input.input_epcs.clear();
        assert_eq!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::MissingInputs)
        );
    }

    #[test]
    fn event_with_transformation_id_may_add_inputs_only() {
        let mut input = event();
        input.transformation_id = Some("urn:example:transformation:batch-7".to_string());
        input.output_epcs.clear();
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(
            normalized.scope(),
            TransformationScope::SharedTransformation(
                "urn:example:transformation:batch-7".to_string()
            )
        );
        assert!(normalized.output_epcs().is_empty());
    }

    #[test]
    fn event_with_transformation_id_may_add_outputs_only() {
        let mut input = event();
        input.transformation_id = Some("urn:example:transformation:batch-7".to_string());
        input.input_epcs.clear();
        let normalized = normalize_transformation_event(&input).unwrap();
        assert!(normalized.input_epcs().is_empty());
        assert_eq!(normalized.output_epcs().len(), 1);
    }

    #[test]
    fn event_with_transformation_id_still_requires_one_side() {
        let mut input = event();
        input.transformation_id = Some("urn:example:transformation:batch-7".to_string());
        input.input_epcs.clear();
        input.output_epcs.clear();
        assert_eq!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::MissingInputsAndOutputs)
        );
    }

    #[test]
    fn inputs_from_one_event_can_share_group_scope_with_outputs_from_another() {
        let input_event = normalized(
            "urn:uuid:transform-input",
            Some("urn:example:transformation:batch-9"),
            &["input-a", "input-b"],
            &[],
        );
        let output_event = normalized(
            "urn:uuid:transform-output",
            Some("urn:example:transformation:batch-9"),
            &[],
            &["output-a", "output-b"],
        );

        let report = build_transformation_groups(&[output_event, input_event]).unwrap();
        assert_eq!(report.groups.len(), 1);
        let group = &report.groups[0];
        assert_eq!(group.input_epcs.len(), 2);
        assert_eq!(group.output_epcs.len(), 2);
        assert_eq!(group.member_events.len(), 2);
        assert_eq!(group.allocation, AllocationStatus::UnallocatedManyToMany);
        assert_eq!(group.closure, TransformationClosure::NotEstablished);
    }

    #[test]
    fn distinct_transformation_ids_never_share_a_group() {
        let first = normalized("event-a", Some("urn:example:batch-a"), &["in-a"], &[]);
        let second = normalized("event-b", Some("urn:example:batch-b"), &[], &["out-b"]);
        let report = build_transformation_groups(&[first, second]).unwrap();
        assert_eq!(report.groups.len(), 2);
    }

    #[test]
    fn events_without_transformation_id_remain_event_local() {
        let first = normalized("event-a", None, &["in-a"], &["out-a"]);
        let second = normalized("event-b", None, &["in-b"], &["out-b"]);
        let report = build_transformation_groups(&[second, first]).unwrap();
        assert_eq!(report.groups.len(), 2);
        assert!(
            report
                .groups
                .iter()
                .all(|group| matches!(group.key, TransformationGroupKey::EventLocal(_)))
        );
    }

    #[test]
    fn same_epc_on_both_sides_is_preserved_not_auto_rejected() {
        let event = normalized(
            "event-rework",
            None,
            &["urn:epc:id:sgtin:rework-1"],
            &["urn:epc:id:sgtin:rework-1"],
        );
        let report = build_transformation_groups(&[event]).unwrap();
        assert_eq!(
            report.groups[0].input_epcs[0].epc,
            report.groups[0].output_epcs[0].epc
        );
    }

    #[test]
    fn repeated_input_and_output_epcs_are_preserved_without_deduplication() {
        let mut input = event();
        input.input_epcs = vec!["input-1".to_string(), "input-1".to_string()];
        input.output_epcs = vec!["output-1".to_string(), "output-1".to_string()];
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(normalized.input_epcs().len(), 2);
        assert_eq!(normalized.output_epcs().len(), 2);
    }

    #[test]
    fn class_quantities_are_preserved_on_separate_input_and_output_sides() {
        let mut input = event();
        input.input_quantity_list = vec![quantity("class-input", Some("2.5"), Some("KGM"))];
        input.output_quantity_list = vec![quantity("class-output", Some("1.25"), Some("KGM"))];
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(
            normalized.input_quantity_list()[0]
                .quantity_lexeme
                .as_deref(),
            Some("2.5")
        );
        assert_eq!(
            normalized.output_quantity_list()[0]
                .quantity_lexeme
                .as_deref(),
            Some("1.25")
        );
    }

    #[test]
    fn unknown_class_quantity_is_not_converted_to_zero() {
        let mut input = event();
        input.input_quantity_list = vec![quantity("class-unknown", None, None)];
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(normalized.input_quantity_list()[0].quantity_lexeme, None);
    }

    #[test]
    fn repeated_class_quantity_entries_are_preserved() {
        let mut input = event();
        input.input_quantity_list = vec![
            quantity("class-a", Some("2"), None),
            quantity("class-a", Some("3"), None),
        ];
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(normalized.input_quantity_list().len(), 2);
    }

    #[test]
    fn invalid_fractional_count_without_uom_is_rejected() {
        let mut input = event();
        input.input_quantity_list = vec![quantity("class-count", Some("1.25"), None)];
        assert!(matches!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::InvalidQuantityElement(_))
        ));
    }

    #[test]
    fn event_time_and_record_time_remain_separate() {
        let normalized = normalize_transformation_event(&event()).unwrap();
        assert_eq!(normalized.event_time_lexeme(), "2026-11-22T10:00:00+02:00");
        assert_eq!(
            normalized.record_time_lexeme(),
            Some("2026-11-22T08:00:04Z")
        );
    }

    #[test]
    fn selected_business_context_and_error_declaration_are_retained() {
        let mut input = event();
        input.biz_location_id = Some("urn:epc:id:sgln:12345.2.0".to_string());
        input.error_declaration = Some("opaque-source-error-declaration".to_string());
        let normalized = normalize_transformation_event(&input).unwrap();
        assert_eq!(
            normalized.business_context(),
            (
                Some("commissioning"),
                Some("in_progress"),
                Some("urn:epc:id:sgln:12345.1.0"),
                Some("urn:epc:id:sgln:12345.2.0")
            )
        );
        assert_eq!(
            normalized.error_declaration(),
            Some("opaque-source-error-declaration")
        );
    }

    #[test]
    fn malformed_event_provenance_is_rejected() {
        let mut input = event();
        input.source_id.clear();
        assert_eq!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::EmptySourceIdentity)
        );

        let mut input = event();
        input.revision = 0;
        assert_eq!(
            normalize_transformation_event(&input)
                .unwrap()
                .source()
                .revision,
            0
        );

        let mut input = event();
        input.event_time_zone_offset = "+14:01".to_string();
        assert!(matches!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::InvalidEventTimeZoneOffset(_))
        ));
    }

    #[test]
    fn blank_transformation_id_is_rejected_without_repair() {
        let mut input = event();
        input.transformation_id = Some(" batch-9 ".to_string());
        assert_eq!(
            normalize_transformation_event(&input),
            Err(TransformationSemanticError::InvalidTransformationId)
        );
    }

    #[test]
    fn conflicting_same_revision_content_rejects_grouping() {
        let first = normalized(
            "same-event",
            Some("urn:example:batch"),
            &["input-a"],
            &["out-a"],
        );
        let second = normalized(
            "same-event",
            Some("urn:example:batch"),
            &["input-b"],
            &["out-a"],
        );
        assert!(matches!(
            build_transformation_groups(&[first, second]),
            Err(TransformationGroupingError::ConflictingSourceEventRevision(
                _
            ))
        ));
    }

    #[test]
    fn exact_duplicate_event_replay_is_idempotent() {
        let one = normalized(
            "same-event",
            Some("urn:example:batch"),
            &["input-a"],
            &["out-a"],
        );
        let report = build_transformation_groups(&[one.clone(), one]).unwrap();
        assert_eq!(report.input_event_rows, 2);
        assert_eq!(report.unique_event_revisions, 1);
        assert_eq!(report.exact_duplicate_replays, 1);
        assert_eq!(report.groups[0].input_epcs.len(), 1);
    }

    #[test]
    fn grouping_is_deterministic_under_event_arrival_permutation() {
        let a = normalized("event-a", Some("urn:example:batch"), &["input-a"], &[]);
        let b = normalized("event-b", Some("urn:example:batch"), &[], &["output-b"]);
        let forward = build_transformation_groups(&[a.clone(), b.clone()]).unwrap();
        let reverse = build_transformation_groups(&[b, a]).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn group_does_not_claim_mass_balance_or_completion() {
        let a = normalized("event-a", Some("urn:example:batch"), &["input-a"], &[]);
        let report = build_transformation_groups(&[a]).unwrap();
        assert_eq!(
            report.groups[0].closure,
            TransformationClosure::NotEstablished
        );
        assert_eq!(
            report.groups[0].allocation,
            AllocationStatus::UnallocatedManyToMany
        );
    }

    #[test]
    fn input_and_output_quantities_keep_source_event_attribution() {
        let mut input = event();
        input.input_quantity_list = vec![quantity("class-in", Some("4"), None)];
        input.output_quantity_list = vec![quantity("class-out", Some("3"), None)];
        let normalized = normalize_transformation_event(&input).unwrap();
        let report = build_transformation_groups(&[normalized]).unwrap();
        let group = &report.groups[0];
        assert_eq!(
            group.input_quantities[0].source_event.source_event_id,
            "urn:uuid:transform-001"
        );
        assert_eq!(
            group.output_quantities[0].source_event.source_event_id,
            "urn:uuid:transform-001"
        );
    }
}
