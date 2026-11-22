// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Loss-aware semantic decoding for EPCIS 2.0.1 AggregationEvent actions.
//!
//! This module translates the meaning of one already-parsed source event. It
//! is not an EPCIS JSON/XML parser or a conformance claim. In particular,
//! OBSERVE is retained as an observation (not a membership mutation),
//! childQuantityList stays class-level quantity evidence (not fabricated
//! instance identities), and DELETE with no child list is an explicit
//! delete-all operation for the named parent.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AggregationAction {
    Add,
    Observe,
    Delete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantityElement {
    pub object_class_id: String,
    /// Preserve the original EPCIS decimal lexeme exactly; do not use f64.
    pub quantity_decimal: String,
    pub unit_of_measure: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpcisAggregationEvent {
    /// Local source identity supplied by the versioned source adapter.
    pub source_id: String,
    /// Stable source-event key supplied by the versioned source adapter.
    pub source_event_id: String,
    pub source_profile_id: String,
    pub source_profile_version: String,
    pub action: AggregationAction,
    /// Optional only for OBSERVE under the EPCIS AggregationEvent contract.
    pub parent_id: Option<String>,
    /// Instance-level child identities (childEPCs).
    pub child_epcs: Vec<String>,
    /// Class-level quantity facts (childQuantityList); never expanded to IDs.
    pub child_quantity_list: Vec<QuantityElement>,
    /// Preserve source lexemes; timestamp parsing/normalization is upstream.
    pub event_time_lexeme: String,
    /// Preserve and validate the EPCIS local offset as a separate field.
    pub event_time_zone_offset: String,
    /// Repository bookkeeping time, not physical event time.
    pub record_time_lexeme: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregationEffect {
    Add {
        parent_id: String,
        child_epcs: Vec<String>,
        child_quantity_list: Vec<QuantityElement>,
    },
    Observe {
        parent_id: Option<String>,
        observed_child_epcs: Vec<String>,
        observed_child_quantities: Vec<QuantityElement>,
        may_be_incomplete: bool,
    },
    Delete {
        parent_id: String,
        child_epcs: Vec<String>,
        child_quantity_list: Vec<QuantityElement>,
    },
    DeleteAll {
        parent_id: String,
    },
}

impl AggregationEffect {
    /// OBSERVE never changes the active membership graph.
    pub fn changes_membership(&self) -> bool {
        !matches!(self, Self::Observe { .. })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedAggregationEvent {
    pub source_id: String,
    pub source_event_id: String,
    pub source_profile_id: String,
    pub source_profile_version: String,
    /// Original eventTime text. It is not replaced by recordTime.
    pub event_time_lexeme: String,
    /// Original EPCIS eventTimeZoneOffset text.
    pub event_time_zone_offset: String,
    /// Original repository recordTime text, if present.
    pub record_time_lexeme: Option<String>,
    pub effect: AggregationEffect,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AggregationSemanticError {
    EmptySourceIdentity,
    EmptySourceProfile,
    EmptyEventTime,
    InvalidEventTimeZoneOffset(String),
    EmptyRecordTime,
    InvalidParentIdentity,
    MissingParent,
    MissingChildren,
    InvalidChildIdentity(String),
    DuplicateChildIdentity(String),
    InvalidQuantityElement(String),
    DuplicateQuantityElement {
        object_class_id: String,
        unit_of_measure: String,
    },
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn valid_positive_decimal(value: &str) -> bool {
    if value.is_empty() || value.trim() != value || value.starts_with('-') {
        return false;
    }

    let value = value.strip_prefix('+').unwrap_or(value);
    if value.is_empty() {
        return false;
    }

    let mut dot_seen = false;
    let mut digit_seen = false;
    let mut nonzero_seen = false;

    for byte in value.bytes() {
        match byte {
            b'0'..=b'9' => {
                digit_seen = true;
                nonzero_seen |= byte != b'0';
            }
            b'.' if !dot_seen => dot_seen = true,
            _ => return false,
        }
    }

    digit_seen && nonzero_seen
}

/// EPCIS 2.0.1 eventTimeZoneOffset lexical form: +/-HH:MM, bounded by 14:00.
fn valid_epcis_offset(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 6
        || !matches!(bytes[0], b'+' | b'-')
        || bytes[3] != b':'
        || !bytes[1].is_ascii_digit()
        || !bytes[2].is_ascii_digit()
        || !bytes[4].is_ascii_digit()
        || !bytes[5].is_ascii_digit()
    {
        return false;
    }

    let hours = u16::from(bytes[1] - b'0') * 10 + u16::from(bytes[2] - b'0');
    let minutes = u16::from(bytes[4] - b'0') * 10 + u16::from(bytes[5] - b'0');
    minutes <= 59 && (hours < 14 || (hours == 14 && minutes == 0))
}

fn validate_child_epcs(child_epcs: &[String]) -> Result<(), AggregationSemanticError> {
    let mut seen = BTreeSet::new();
    for child in child_epcs {
        if !valid_identifier(child) {
            return Err(AggregationSemanticError::InvalidChildIdentity(
                child.clone(),
            ));
        }
        if !seen.insert(child.as_str()) {
            return Err(AggregationSemanticError::DuplicateChildIdentity(
                child.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_quantities(quantities: &[QuantityElement]) -> Result<(), AggregationSemanticError> {
    let mut seen = BTreeSet::new();
    for item in quantities {
        if !valid_identifier(&item.object_class_id)
            || !valid_positive_decimal(&item.quantity_decimal)
            || !valid_identifier(&item.unit_of_measure)
        {
            return Err(AggregationSemanticError::InvalidQuantityElement(
                item.object_class_id.clone(),
            ));
        }

        let key = (item.object_class_id.as_str(), item.unit_of_measure.as_str());
        if !seen.insert(key) {
            return Err(AggregationSemanticError::DuplicateQuantityElement {
                object_class_id: item.object_class_id.clone(),
                unit_of_measure: item.unit_of_measure.clone(),
            });
        }
    }
    Ok(())
}

/// Convert an already-parsed EPCIS AggregationEvent into a loss-aware meaning
/// object. This function performs no graph mutation and no timestamp parsing.
pub fn normalize_aggregation_event(
    event: &EpcisAggregationEvent,
) -> Result<NormalizedAggregationEvent, AggregationSemanticError> {
    if !valid_identifier(&event.source_id) || !valid_identifier(&event.source_event_id) {
        return Err(AggregationSemanticError::EmptySourceIdentity);
    }
    if !valid_identifier(&event.source_profile_id)
        || !valid_identifier(&event.source_profile_version)
    {
        return Err(AggregationSemanticError::EmptySourceProfile);
    }
    if !valid_identifier(&event.event_time_lexeme) {
        return Err(AggregationSemanticError::EmptyEventTime);
    }
    if !valid_epcis_offset(&event.event_time_zone_offset) {
        return Err(AggregationSemanticError::InvalidEventTimeZoneOffset(
            event.event_time_zone_offset.clone(),
        ));
    }
    if event
        .record_time_lexeme
        .as_ref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return Err(AggregationSemanticError::EmptyRecordTime);
    }
    if event
        .parent_id
        .as_ref()
        .is_some_and(|value| !valid_identifier(value))
    {
        return Err(AggregationSemanticError::InvalidParentIdentity);
    }

    validate_child_epcs(&event.child_epcs)?;
    validate_quantities(&event.child_quantity_list)?;

    let has_children = !event.child_epcs.is_empty() || !event.child_quantity_list.is_empty();

    let effect = match event.action {
        AggregationAction::Add => {
            let parent_id = event
                .parent_id
                .as_ref()
                .ok_or(AggregationSemanticError::MissingParent)?;
            if !has_children {
                return Err(AggregationSemanticError::MissingChildren);
            }
            AggregationEffect::Add {
                parent_id: parent_id.clone(),
                child_epcs: event.child_epcs.clone(),
                child_quantity_list: event.child_quantity_list.clone(),
            }
        }
        AggregationAction::Observe => {
            if !has_children {
                return Err(AggregationSemanticError::MissingChildren);
            }
            AggregationEffect::Observe {
                parent_id: event.parent_id.clone(),
                observed_child_epcs: event.child_epcs.clone(),
                observed_child_quantities: event.child_quantity_list.clone(),
                may_be_incomplete: true,
            }
        }
        AggregationAction::Delete => {
            let parent_id = event
                .parent_id
                .as_ref()
                .ok_or(AggregationSemanticError::MissingParent)?;
            if has_children {
                AggregationEffect::Delete {
                    parent_id: parent_id.clone(),
                    child_epcs: event.child_epcs.clone(),
                    child_quantity_list: event.child_quantity_list.clone(),
                }
            } else {
                AggregationEffect::DeleteAll {
                    parent_id: parent_id.clone(),
                }
            }
        }
    };

    Ok(NormalizedAggregationEvent {
        source_id: event.source_id.clone(),
        source_event_id: event.source_event_id.clone(),
        source_profile_id: event.source_profile_id.clone(),
        source_profile_version: event.source_profile_version.clone(),
        event_time_lexeme: event.event_time_lexeme.clone(),
        event_time_zone_offset: event.event_time_zone_offset.clone(),
        record_time_lexeme: event.record_time_lexeme.clone(),
        effect,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(action: AggregationAction) -> EpcisAggregationEvent {
        EpcisAggregationEvent {
            source_id: "WMS_A".to_string(),
            source_event_id: "urn:uuid:aggregation-001".to_string(),
            source_profile_id: "GS1_EPCIS".to_string(),
            source_profile_version: "2.0.1".to_string(),
            action,
            parent_id: Some("urn:epc:id:sscc:123".to_string()),
            child_epcs: Vec::new(),
            child_quantity_list: Vec::new(),
            event_time_lexeme: "2026-11-22T10:00:00+02:00".to_string(),
            event_time_zone_offset: "+02:00".to_string(),
            record_time_lexeme: Some("2026-11-22T08:00:04Z".to_string()),
        }
    }

    fn quantity(
        object_class_id: &str,
        quantity_decimal: &str,
        unit_of_measure: &str,
    ) -> QuantityElement {
        QuantityElement {
            object_class_id: object_class_id.to_string(),
            quantity_decimal: quantity_decimal.to_string(),
            unit_of_measure: unit_of_measure.to_string(),
        }
    }

    #[test]
    fn add_preserves_instance_child_identities() {
        let mut input = event(AggregationAction::Add);
        input.child_epcs = vec!["urn:epc:id:sgtin:1".to_string()];
        let output = normalize_aggregation_event(&input).unwrap();

        assert_eq!(
            output.effect,
            AggregationEffect::Add {
                parent_id: "urn:epc:id:sscc:123".to_string(),
                child_epcs: vec!["urn:epc:id:sgtin:1".to_string()],
                child_quantity_list: Vec::new(),
            }
        );
        assert!(output.effect.changes_membership());
    }

    #[test]
    fn add_preserves_class_quantity_without_fabricating_instance_ids() {
        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![quantity("urn:epc:class:lgtin:milk", "12.50", "EA")];
        let output = normalize_aggregation_event(&input).unwrap();

        match output.effect {
            AggregationEffect::Add {
                child_epcs,
                child_quantity_list,
                ..
            } => {
                assert!(child_epcs.is_empty());
                assert_eq!(
                    child_quantity_list,
                    vec![quantity("urn:epc:class:lgtin:milk", "12.50", "EA")]
                );
            }
            other => panic!("unexpected effect: {other:?}"),
        }
    }

    #[test]
    fn add_preserves_both_instance_and_class_level_lists() {
        let mut input = event(AggregationAction::Add);
        input.child_epcs = vec!["urn:epc:id:sscc:pallet-1".to_string()];
        input.child_quantity_list = vec![quantity("urn:epc:class:carton", "3", "EA")];
        let output = normalize_aggregation_event(&input).unwrap();

        assert!(matches!(
            output.effect,
            AggregationEffect::Add {
                child_epcs,
                child_quantity_list,
                ..
            } if child_epcs.len() == 1 && child_quantity_list.len() == 1
        ));
    }

    #[test]
    fn observe_is_an_incomplete_observation_not_a_membership_mutation() {
        let mut input = event(AggregationAction::Observe);
        input.child_epcs = vec!["urn:epc:id:sscc:observed-pallet".to_string()];
        let output = normalize_aggregation_event(&input).unwrap();

        assert!(matches!(
            &output.effect,
            AggregationEffect::Observe {
                may_be_incomplete: true,
                ..
            }
        ));
        assert!(!output.effect.changes_membership());
    }

    #[test]
    fn observe_may_omit_parent_identity() {
        let mut input = event(AggregationAction::Observe);
        input.parent_id = None;
        input.child_epcs = vec!["urn:epc:id:sscc:observed-pallet".to_string()];

        assert!(matches!(
            normalize_aggregation_event(&input).unwrap().effect,
            AggregationEffect::Observe {
                parent_id: None,
                ..
            }
        ));
    }

    #[test]
    fn observe_does_not_infer_that_unlisted_children_were_deleted() {
        let mut input = event(AggregationAction::Observe);
        input.child_epcs = vec!["urn:epc:id:sscc:one-observed-child".to_string()];
        let output = normalize_aggregation_event(&input).unwrap();

        assert!(!output.effect.changes_membership());
        assert!(matches!(
            output.effect,
            AggregationEffect::Observe {
                observed_child_epcs,
                may_be_incomplete: true,
                ..
            } if observed_child_epcs.len() == 1
        ));
    }

    #[test]
    fn delete_with_named_children_is_child_specific() {
        let mut input = event(AggregationAction::Delete);
        input.child_epcs = vec!["urn:epc:id:sscc:pallet-1".to_string()];
        let output = normalize_aggregation_event(&input).unwrap();

        assert_eq!(
            output.effect,
            AggregationEffect::Delete {
                parent_id: "urn:epc:id:sscc:123".to_string(),
                child_epcs: vec!["urn:epc:id:sscc:pallet-1".to_string()],
                child_quantity_list: Vec::new(),
            }
        );
    }

    #[test]
    fn delete_with_empty_child_lists_means_delete_all_children() {
        let input = event(AggregationAction::Delete);
        assert_eq!(
            normalize_aggregation_event(&input).unwrap().effect,
            AggregationEffect::DeleteAll {
                parent_id: "urn:epc:id:sscc:123".to_string()
            }
        );
    }

    #[test]
    fn observe_with_class_quantity_only_remains_non_mutating() {
        let mut input = event(AggregationAction::Observe);
        input.parent_id = None;
        input.child_quantity_list = vec![quantity("class-a", "4.25", "KGM")];

        let output = normalize_aggregation_event(&input).unwrap();
        assert!(!output.effect.changes_membership());
        assert!(matches!(
            output.effect,
            AggregationEffect::Observe {
                parent_id: None,
                observed_child_epcs,
                observed_child_quantities,
                may_be_incomplete: true,
            } if observed_child_epcs.is_empty()
                && observed_child_quantities == vec![quantity("class-a", "4.25", "KGM")]
        ));
    }

    #[test]
    fn delete_with_class_quantity_is_subset_delete_not_delete_all() {
        let mut input = event(AggregationAction::Delete);
        input.child_quantity_list = vec![quantity("class-a", "4.25", "KGM")];

        assert_eq!(
            normalize_aggregation_event(&input).unwrap().effect,
            AggregationEffect::Delete {
                parent_id: "urn:epc:id:sscc:123".to_string(),
                child_epcs: Vec::new(),
                child_quantity_list: vec![quantity("class-a", "4.25", "KGM")],
            }
        );
    }

    #[test]
    fn empty_add_is_rejected() {
        assert_eq!(
            normalize_aggregation_event(&event(AggregationAction::Add)),
            Err(AggregationSemanticError::MissingChildren)
        );
    }

    #[test]
    fn empty_observe_is_rejected() {
        assert_eq!(
            normalize_aggregation_event(&event(AggregationAction::Observe)),
            Err(AggregationSemanticError::MissingChildren)
        );
    }

    #[test]
    fn add_and_delete_require_parent_identity() {
        let mut add = event(AggregationAction::Add);
        add.parent_id = None;
        add.child_epcs = vec!["child".to_string()];
        assert_eq!(
            normalize_aggregation_event(&add),
            Err(AggregationSemanticError::MissingParent)
        );

        let mut delete = event(AggregationAction::Delete);
        delete.parent_id = None;
        assert_eq!(
            normalize_aggregation_event(&delete),
            Err(AggregationSemanticError::MissingParent)
        );
    }

    #[test]
    fn duplicate_child_epcs_are_rejected_not_deduplicated() {
        let mut input = event(AggregationAction::Add);
        input.child_epcs = vec!["child-1".to_string(), "child-1".to_string()];
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::DuplicateChildIdentity(
                "child-1".to_string()
            ))
        );
    }

    #[test]
    fn quantity_decimal_is_exact_and_must_be_positive() {
        for invalid in ["0", "0.000", "-1", "1e3", " 2 ", "."] {
            let mut input = event(AggregationAction::Add);
            input.child_quantity_list = vec![quantity("class-a", invalid, "EA")];
            assert!(matches!(
                normalize_aggregation_event(&input),
                Err(AggregationSemanticError::InvalidQuantityElement(_))
            ));
        }

        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![quantity("class-a", "0.1250", "EA")];
        let output = normalize_aggregation_event(&input).unwrap();
        assert!(matches!(
            output.effect,
            AggregationEffect::Add { child_quantity_list, .. }
                if child_quantity_list[0].quantity_decimal == "0.1250"
        ));
    }

    #[test]
    fn duplicate_class_quantity_key_is_rejected() {
        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![
            quantity("class-a", "2", "EA"),
            quantity("class-a", "3", "EA"),
        ];
        assert!(matches!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::DuplicateQuantityElement { .. })
        ));
    }

    #[test]
    fn event_time_and_record_time_remain_separate_opaque_lexemes() {
        let input = event(AggregationAction::Delete);
        let output = normalize_aggregation_event(&input).unwrap();

        assert_eq!(output.event_time_lexeme, "2026-11-22T10:00:00+02:00");
        assert_eq!(
            output.record_time_lexeme.as_deref(),
            Some("2026-11-22T08:00:04Z")
        );
    }

    #[test]
    fn invalid_epcis_offset_is_rejected() {
        for invalid in ["UTC", "+14:01", "+15:00", "+2:00", "02:00", "+00:99"] {
            let mut input = event(AggregationAction::Delete);
            input.event_time_zone_offset = invalid.to_string();
            assert!(matches!(
                normalize_aggregation_event(&input),
                Err(AggregationSemanticError::InvalidEventTimeZoneOffset(_))
            ));
        }
    }

    #[test]
    fn invalid_source_and_repository_provenance_is_rejected() {
        let mut input = event(AggregationAction::Delete);
        input.source_id.clear();
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::EmptySourceIdentity)
        );

        let mut input = event(AggregationAction::Delete);
        input.source_event_id = " padded-event-key ".to_string();
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::EmptySourceIdentity)
        );

        let mut input = event(AggregationAction::Delete);
        input.source_profile_version.clear();
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::EmptySourceProfile)
        );

        let mut input = event(AggregationAction::Delete);
        input.record_time_lexeme = Some(" ".to_string());
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::EmptyRecordTime)
        );
    }

    #[test]
    fn epcis_offset_accepts_both_inclusive_fourteen_hour_boundaries() {
        for offset in ["+14:00", "-14:00", "+00:00", "-00:00"] {
            let mut input = event(AggregationAction::Delete);
            input.event_time_zone_offset = offset.to_string();
            assert!(normalize_aggregation_event(&input).is_ok(), "{offset}");
        }
    }

    #[test]
    fn blank_or_padded_unit_of_measure_is_not_normalized() {
        for unit in ["", " EA", "EA "] {
            let mut input = event(AggregationAction::Add);
            input.child_quantity_list = vec![quantity("class-a", "2", unit)];
            assert!(matches!(
                normalize_aggregation_event(&input),
                Err(AggregationSemanticError::InvalidQuantityElement(_))
            ));
        }
    }

    #[test]
    fn raw_identifiers_are_not_trimmed_or_repaired() {
        let mut input = event(AggregationAction::Add);
        input.child_epcs = vec![" child ".to_string()];
        assert_eq!(
            normalize_aggregation_event(&input),
            Err(AggregationSemanticError::InvalidChildIdentity(
                " child ".to_string()
            ))
        );
    }
}
