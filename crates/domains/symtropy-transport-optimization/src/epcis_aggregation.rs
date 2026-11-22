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
    /// Preserve the original EPCIS xsd:double lexeme exactly; do not round-trip through f64.
    /// None represents an unspecified/unknown quantity.
    pub quantity_lexeme: Option<String>,
    /// None is required when quantity_lexeme is None; when present this is a UOM code lexeme.
    pub unit_of_measure: Option<String>,
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
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn valid_positive_xsd_double(value: &str) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value
            .parse::<f64>()
            .is_ok_and(|number| number.is_finite() && number > 0.0)
}

/// A QuantityElement without a UOM denotes a count and therefore needs an
/// exact positive integer value. Work on the source lexeme so large integer
/// counts are not rounded through binary floating-point arithmetic.
fn valid_positive_integer_lexeme(value: &str) -> bool {
    if !valid_positive_xsd_double(value) {
        return false;
    }

    let (mantissa, exponent) = match value.find(['e', 'E']) {
        Some(index) => {
            let Ok(exponent) = value[index + 1..].parse::<i64>() else {
                return false;
            };
            (&value[..index], exponent)
        }
        None => (value, 0),
    };
    let mantissa = mantissa.strip_prefix('+').unwrap_or(mantissa);
    let (whole, fractional) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = format!("{whole}{fractional}");
    let decimal_places = fractional.len() as i64 - exponent;

    if decimal_places <= 0 {
        return true;
    }
    if decimal_places > digits.len() as i64 {
        return false;
    }

    digits[digits.len() - decimal_places as usize..]
        .bytes()
        .all(|digit| digit == b'0')
}

/// EPCIS uses the 2- or 3-character Common Code form for physical-measure
/// units. Code-list membership and status are qualified by the pinned
/// vocabulary profile, not guessed here.
fn valid_uom_lexeme(value: &str) -> bool {
    matches!(value.len(), 2 | 3)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
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
    for item in quantities {
        let quantity_is_valid = match (
            item.quantity_lexeme.as_deref(),
            item.unit_of_measure.as_deref(),
        ) {
            // EPCIS permits an unspecified quantity only when no UOM is given.
            (None, None) => true,
            (None, Some(_)) => false,
            // Without a UOM, the value denotes an integer count.
            (Some(quantity), None) => valid_positive_integer_lexeme(quantity),
            // With a UOM, the value denotes a positive physical measure.
            (Some(quantity), Some(uom)) => {
                valid_positive_xsd_double(quantity) && valid_uom_lexeme(uom)
            }
        };

        if !valid_identifier(&item.object_class_id) || !quantity_is_valid {
            return Err(AggregationSemanticError::InvalidQuantityElement(
                item.object_class_id.clone(),
            ));
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
        quantity_lexeme: &str,
        unit_of_measure: &str,
    ) -> QuantityElement {
        QuantityElement {
            object_class_id: object_class_id.to_string(),
            quantity_lexeme: Some(quantity_lexeme.to_string()),
            unit_of_measure: Some(unit_of_measure.to_string()),
        }
    }

    fn count_quantity(object_class_id: &str, quantity_lexeme: &str) -> QuantityElement {
        QuantityElement {
            object_class_id: object_class_id.to_string(),
            quantity_lexeme: Some(quantity_lexeme.to_string()),
            unit_of_measure: None,
        }
    }

    fn unknown_quantity(object_class_id: &str) -> QuantityElement {
        QuantityElement {
            object_class_id: object_class_id.to_string(),
            quantity_lexeme: None,
            unit_of_measure: None,
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
        input.child_quantity_list = vec![count_quantity("urn:epc:class:lgtin:milk", "12")];
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
                    vec![count_quantity("urn:epc:class:lgtin:milk", "12")]
                );
            }
            other => panic!("unexpected effect: {other:?}"),
        }
    }

    #[test]
    fn add_preserves_both_instance_and_class_level_lists() {
        let mut input = event(AggregationAction::Add);
        input.child_epcs = vec!["urn:epc:id:sscc:pallet-1".to_string()];
        input.child_quantity_list = vec![count_quantity("urn:epc:class:carton", "3")];
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
    fn quantity_lexeme_is_preserved_and_must_be_positive() {
        for invalid in ["0", "0.000", "-1", "NaN", "INF", "1e309", " 2 ", "."] {
            let mut input = event(AggregationAction::Add);
            input.child_quantity_list = vec![quantity("class-a", invalid, "KGM")];
            assert!(matches!(
                normalize_aggregation_event(&input),
                Err(AggregationSemanticError::InvalidQuantityElement(_))
            ));
        }

        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![quantity("class-a", "0.1250", "KGM")];
        let output = normalize_aggregation_event(&input).unwrap();
        assert!(matches!(
            output.effect,
            AggregationEffect::Add { child_quantity_list, .. }
                if child_quantity_list[0].quantity_lexeme.as_deref() == Some("0.1250")
        ));
    }

    #[test]
    fn quantity_list_preserves_unknown_counts_and_physical_measures() {
        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![
            unknown_quantity("class-unknown"),
            count_quantity("class-count", "9007199254740993"),
            quantity("class-weight", "0.1250", "KGM"),
            quantity("class-measured-exponent", "1.25e2", "KGM"),
        ];

        let output = normalize_aggregation_event(&input).unwrap();
        assert!(matches!(
            output.effect,
            AggregationEffect::Add { child_quantity_list, .. }
                if child_quantity_list == vec![
                    unknown_quantity("class-unknown"),
                    count_quantity("class-count", "9007199254740993"),
                    quantity("class-weight", "0.1250", "KGM"),
            quantity("class-measured-exponent", "1.25e2", "KGM"),
                ]
        ));
    }

    #[test]
    fn count_quantity_without_uom_must_be_a_positive_integer() {
        for invalid in ["0", "-1", "1.5", "1e-1", "NaN", "INF"] {
            let mut input = event(AggregationAction::Add);
            input.child_quantity_list = vec![count_quantity("class-count", invalid)];
            assert!(matches!(
                normalize_aggregation_event(&input),
                Err(AggregationSemanticError::InvalidQuantityElement(_))
            ));
        }

        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![count_quantity("class-count", "100e-2")];
        assert!(normalize_aggregation_event(&input).is_ok());
    }

    #[test]
    fn physical_measure_requires_quantity_and_two_or_three_character_uom() {
        let mut missing_quantity = event(AggregationAction::Add);
        missing_quantity.child_quantity_list = vec![QuantityElement {
            object_class_id: "class-a".to_string(),
            quantity_lexeme: None,
            unit_of_measure: Some("KGM".to_string()),
        }];
        assert!(matches!(
            normalize_aggregation_event(&missing_quantity),
            Err(AggregationSemanticError::InvalidQuantityElement(_))
        ));
    }

    #[test]
    fn repeated_class_quantity_entries_are_preserved_without_aggregation() {
        let mut input = event(AggregationAction::Add);
        input.child_quantity_list = vec![
            count_quantity("class-a", "2"),
            count_quantity("class-a", "3"),
        ];
        let output = normalize_aggregation_event(&input).unwrap();
        assert!(matches!(
            output.effect,
            AggregationEffect::Add { child_quantity_list, .. }
                if child_quantity_list == vec![count_quantity("class-a", "2"), count_quantity("class-a", "3")]
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
        for unit in ["", " KGM", "KGM ", "ea", "KGMX"] {
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
