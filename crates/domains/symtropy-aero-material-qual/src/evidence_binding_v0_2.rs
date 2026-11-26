// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent specimen/configuration assertions and time-scoped metrology binding.
//!
//! This module checks the v0.2 synthetic evidence-binding contract. It does not
//! authenticate certificates cryptographically, prove that a physical
//! observation occurred, establish material allowables, qualify a process, or
//! approve an aircraft. A consistent binding is only a prerequisite for
//! evidence review by competent authorities.

use crate::effectivity_interval_v0_1::{
    EffectivityRelation, TimeInterval, compare_effectivity_intervals,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const PROFILE_ID: &str = "aero-evidence-binding-v0.2";
const CLAIM_CEILING: &str = "synthetic_binding_contract_only";
const INTERVAL_SEMANTICS: &str = "[valid_from_unix_ms, valid_until_unix_ms)";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BindingFinding {
    pub code: String,
    pub subject_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MeasurementDisposition {
    pub observation_id: String,
    pub status: String,
    pub reasons: Vec<String>,
    /// Known events retained as non-applicable to this subject-time acquisition.
    pub non_applicable_event_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BindingReport {
    pub evaluator_profile: String,
    pub case_id: Option<String>,
    pub structurally_complete: bool,
    pub specimen_identity_status: String,
    pub intended_configuration_status: String,
    pub metrology_binding_status: String,
    pub decision_status: String,
    pub physical_qualification_status: String,
    pub violations: Vec<BindingFinding>,
    pub blockers: Vec<BindingFinding>,
    pub measurement_dispositions: Vec<MeasurementDisposition>,
    pub capability_gaps: Vec<String>,
}

fn field_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn push(findings: &mut Vec<BindingFinding>, code: &str, subject: &str) {
    findings.push(BindingFinding {
        code: code.to_owned(),
        subject_id: subject.to_owned(),
    });
}

fn array_at<'a>(
    root: &'a Value,
    path: &str,
    violations: &mut Vec<BindingFinding>,
) -> Vec<&'a Value> {
    match root.pointer(path).and_then(Value::as_array) {
        Some(rows) => rows.iter().collect(),
        None => {
            push(violations, "MISSING_OR_INVALID_COLLECTION", path);
            Vec::new()
        }
    }
}

fn index_by_id<'a>(
    rows: &[&'a Value],
    kind: &str,
    violations: &mut Vec<BindingFinding>,
) -> BTreeMap<String, &'a Value> {
    let mut index = BTreeMap::new();
    let mut ambiguous_ids = BTreeSet::new();
    for (position, row) in rows.iter().enumerate() {
        let Some(id) = field_str(row, "id") else {
            push(
                violations,
                "MISSING_ENTITY_ID",
                &format!("{kind}[{position}]"),
            );
            continue;
        };
        if id.trim().is_empty() || id != id.trim() {
            push(violations, "NON_CANONICAL_ENTITY_ID", id);
            continue;
        }
        if ambiguous_ids.contains(id) {
            push(violations, "DUPLICATE_ENTITY_ID", id);
            continue;
        }
        if index.contains_key(id) {
            index.remove(id);
            ambiguous_ids.insert(id.to_owned());
            push(violations, "DUPLICATE_ENTITY_ID", id);
        } else {
            index.insert(id.to_owned(), *row);
        }
    }
    index
}

fn has_provenance(assertion: &Value) -> bool {
    [
        "source_system",
        "source_record_id",
        "assertion_method",
        "issuer_id",
        "provenance_ref",
    ]
    .iter()
    .all(|key| nonempty(field_str(assertion, key)))
}

fn field_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

/// Parse a nominal point and optional hard-bounds pair.
///
/// Existing v0.2 records remain exact-point observations when both bounds are
/// omitted. When either bound is present, both must be valid unsigned integers,
/// ordered, and contain the nominal point. Statistical intervals are not
/// promoted to hard bounds by this parser.
fn hard_time_interval(
    value: &Value,
    point_field: &str,
    earliest_field: &str,
    latest_field: &str,
) -> Result<TimeInterval, &'static str> {
    let point = field_u64(value, point_field).ok_or("TIME_POINT_MISSING_OR_INVALID")?;
    let earliest_present = value.get(earliest_field).is_some();
    let latest_present = value.get(latest_field).is_some();
    match (earliest_present, latest_present) {
        (false, false) => Ok(TimeInterval {
            earliest_unix_ms: point,
            latest_unix_ms: point,
        }),
        (true, true) => {
            let earliest =
                field_u64(value, earliest_field).ok_or("TIME_BOUNDS_VALUE_MISSING_OR_INVALID")?;
            let latest =
                field_u64(value, latest_field).ok_or("TIME_BOUNDS_VALUE_MISSING_OR_INVALID")?;
            if earliest > latest {
                return Err("TIME_BOUNDS_START_AFTER_END");
            }
            if point < earliest || point > latest {
                return Err("TIME_POINT_OUTSIDE_DECLARED_BOUNDS");
            }
            Ok(TimeInterval {
                earliest_unix_ms: earliest,
                latest_unix_ms: latest,
            })
        }
        _ => Err("TIME_BOUNDS_PAIR_INCOMPLETE"),
    }
}

fn sort_findings(findings: &mut [BindingFinding]) {
    findings.sort_by(|left, right| {
        (&left.code, &left.subject_id).cmp(&(&right.code, &right.subject_id))
    });
}

fn assertion_rows<'a>(
    rows: &[&'a Value],
    specimen_id: &str,
    relation_field: &str,
) -> Vec<&'a Value> {
    rows.iter()
        .copied()
        .filter(|row| field_str(row, relation_field) == Some(specimen_id))
        .collect()
}

fn check_independent_identity(
    specimen_id: &str,
    assertion_rows: &[&Value],
    expected: &[(&str, &str)],
    identity_label: &str,
    violations: &mut Vec<BindingFinding>,
    blockers: &mut Vec<BindingFinding>,
) -> bool {
    if assertion_rows.is_empty() {
        push(
            blockers,
            &format!("{identity_label}_ASSERTION_MISSING"),
            specimen_id,
        );
        return false;
    }
    if assertion_rows.len() != 1 {
        push(
            violations,
            &format!("{identity_label}_ASSERTION_NOT_UNIQUE"),
            specimen_id,
        );
        push(
            blockers,
            &format!("{identity_label}_IDENTITY_UNRESOLVED"),
            specimen_id,
        );
        return false;
    }

    let assertion = assertion_rows[0];
    let mut consistent = true;
    if !has_provenance(assertion) {
        consistent = false;
        push(
            violations,
            &format!("{identity_label}_PROVENANCE_INCOMPLETE"),
            specimen_id,
        );
        push(
            blockers,
            &format!("{identity_label}_IDENTITY_UNRESOLVED"),
            specimen_id,
        );
    }

    for (declared_field, expected_id) in expected {
        if field_str(assertion, declared_field) != Some(*expected_id) {
            push(
                violations,
                &format!("{identity_label}_DECLARED_IDENTITY_MISMATCH"),
                specimen_id,
            );
            push(
                blockers,
                &format!("{identity_label}_IDENTITY_CONFLICT"),
                specimen_id,
            );
            consistent = false;
        }
    }

    match field_str(assertion, "identity_status") {
        Some("verified") => {
            for (verified_field, expected_id) in expected.iter().map(|(declared, expected)| {
                let verified = declared.replacen("declared_", "verified_", 1);
                (verified, *expected)
            }) {
                if field_str(assertion, &verified_field) != Some(verified_field_value(expected_id))
                {
                    push(
                        violations,
                        &format!("{identity_label}_VERIFIED_IDENTITY_MISMATCH"),
                        specimen_id,
                    );
                    push(
                        blockers,
                        &format!("{identity_label}_IDENTITY_CONFLICT"),
                        specimen_id,
                    );
                    consistent = false;
                }
            }
        }
        Some("unresolved") => {
            push(
                blockers,
                &format!("{identity_label}_IDENTITY_UNRESOLVED"),
                specimen_id,
            );
            consistent = false;
        }
        Some("conflict") => {
            push(
                blockers,
                &format!("{identity_label}_IDENTITY_CONFLICT"),
                specimen_id,
            );
            consistent = false;
        }
        _ => {
            push(
                violations,
                &format!("{identity_label}_UNKNOWN_IDENTITY_STATUS"),
                specimen_id,
            );
            push(
                blockers,
                &format!("{identity_label}_IDENTITY_UNRESOLVED"),
                specimen_id,
            );
            consistent = false;
        }
    }

    consistent
}

fn verified_field_value(expected_id: &str) -> &str {
    expected_id
}

pub fn evaluate_binding_case(case: &Value) -> BindingReport {
    let mut violations = Vec::new();
    let mut blockers = Vec::new();
    let mut dispositions = Vec::new();

    let case_id = field_str(case, "case_id").map(str::to_owned);
    if field_str(case, "profile_id") != Some(PROFILE_ID)
        || case.get("synthetic_only").and_then(Value::as_bool) != Some(true)
        || field_str(case, "claim_ceiling") != Some(CLAIM_CEILING)
        || field_str(case, "validity_interval_semantics") != Some(INTERVAL_SEMANTICS)
    {
        push(
            &mut violations,
            "CLAIM_CEILING_OR_PROFILE_VIOLATION",
            "case",
        );
    }

    let evaluated_at = field_u64(case, "evaluated_at_unix_ms");
    if evaluated_at.is_none() {
        push(&mut violations, "INVALID_EVALUATION_TIMESTAMP", "case");
        push(&mut blockers, "EVALUATION_TIME_UNRESOLVED", "case");
    }

    let lots = array_at(case, "/hierarchy/material_lots", &mut violations);
    let batches = array_at(case, "/hierarchy/process_batches", &mut violations);
    let panels = array_at(case, "/hierarchy/panels", &mut violations);
    let specimens = array_at(case, "/hierarchy/specimens", &mut violations);
    let campaigns = array_at(case, "/hierarchy/test_campaigns", &mut violations);
    let configurations = array_at(case, "/hierarchy/configurations", &mut violations);
    let observations = array_at(case, "/observations", &mut violations);
    let specimen_assertions = array_at(case, "/specimen_source_assertions", &mut violations);
    let configuration_assertions =
        array_at(case, "/intended_configuration_assertions", &mut violations);
    let certificates = array_at(case, "/calibration_certificates", &mut violations);
    let events = array_at(case, "/calibration_events", &mut violations);

    for (kind, rows) in [
        ("material_lots", lots.as_slice()),
        ("process_batches", batches.as_slice()),
        ("panels", panels.as_slice()),
        ("specimens", specimens.as_slice()),
        ("test_campaigns", campaigns.as_slice()),
        ("configurations", configurations.as_slice()),
        ("observations", observations.as_slice()),
        ("specimen_source_assertions", specimen_assertions.as_slice()),
        (
            "intended_configuration_assertions",
            configuration_assertions.as_slice(),
        ),
        ("calibration_certificates", certificates.as_slice()),
    ] {
        if rows.is_empty() {
            push(&mut violations, "EMPTY_REQUIRED_COLLECTION", kind);
        }
    }

    let lot_index = index_by_id(&lots, "material_lots", &mut violations);
    let batch_index = index_by_id(&batches, "process_batches", &mut violations);
    let panel_index = index_by_id(&panels, "panels", &mut violations);
    let specimen_index = index_by_id(&specimens, "specimens", &mut violations);
    let campaign_index = index_by_id(&campaigns, "test_campaigns", &mut violations);
    let configuration_index = index_by_id(&configurations, "configurations", &mut violations);
    let certificate_index = index_by_id(&certificates, "calibration_certificates", &mut violations);
    let mut calibration_evidence_ids = BTreeSet::new();
    for certificate in &certificates {
        let certificate_id = field_str(certificate, "id").unwrap_or("certificate");
        match field_str(certificate, "calibration_evidence_id") {
            Some(evidence_id) if nonempty(Some(evidence_id)) => {
                if !calibration_evidence_ids.insert(evidence_id.to_owned()) {
                    push(
                        &mut violations,
                        "CALIBRATION_EVIDENCE_ID_REPLAY",
                        evidence_id,
                    );
                    push(
                        &mut blockers,
                        "CALIBRATION_CERTIFICATE_EVIDENCE_UNBOUND",
                        certificate_id,
                    );
                }
            }
            _ => {
                push(
                    &mut violations,
                    "CALIBRATION_EVIDENCE_ID_MISSING",
                    certificate_id,
                );
                push(
                    &mut blockers,
                    "CALIBRATION_CERTIFICATE_EVIDENCE_UNBOUND",
                    certificate_id,
                );
            }
        }
    }
    let _observation_index = index_by_id(&observations, "observations", &mut violations);
    let _source_assertion_index = index_by_id(
        &specimen_assertions,
        "specimen_source_assertions",
        &mut violations,
    );
    let _configuration_assertion_index = index_by_id(
        &configuration_assertions,
        "intended_configuration_assertions",
        &mut violations,
    );
    let event_index = index_by_id(&events, "calibration_events", &mut violations);

    for assertion in &specimen_assertions {
        let specimen_id = field_str(assertion, "specimen_id").unwrap_or("specimen");
        if !specimen_index.contains_key(specimen_id) {
            push(
                &mut violations,
                "SPECIMEN_SOURCE_ASSERTION_SPECIMEN_UNKNOWN",
                specimen_id,
            );
            push(
                &mut blockers,
                "SPECIMEN_SOURCE_IDENTITY_UNRESOLVED",
                specimen_id,
            );
        }
    }
    for assertion in &configuration_assertions {
        let specimen_id = field_str(assertion, "specimen_id").unwrap_or("specimen");
        if !specimen_index.contains_key(specimen_id) {
            push(
                &mut violations,
                "CONFIGURATION_ASSERTION_SPECIMEN_UNKNOWN",
                specimen_id,
            );
            push(
                &mut blockers,
                "INTENDED_CONFIGURATION_UNRESOLVED",
                specimen_id,
            );
        }
    }

    let mut specimen_item_ids = BTreeSet::new();
    let mut specimen_source_identity_issue = false;
    for assertion in &specimen_assertions {
        let specimen_id = field_str(assertion, "specimen_id").unwrap_or("specimen");
        if !nonempty(field_str(assertion, "sampling_event_id")) {
            push(
                &mut violations,
                "SPECIMEN_SAMPLING_EVENT_MISSING",
                specimen_id,
            );
            push(
                &mut blockers,
                "SPECIMEN_SOURCE_IDENTITY_UNRESOLVED",
                specimen_id,
            );
            specimen_source_identity_issue = true;
        }

        let sampling_event_id = field_str(assertion, "sampling_event_id").unwrap_or("UNRESOLVED");
        match field_str(assertion, "specimen_item_id") {
            Some(item_id) if nonempty(Some(item_id)) => {
                let item_key = (sampling_event_id.to_owned(), item_id.to_owned());
                if !specimen_item_ids.insert(item_key) {
                    push(
                        &mut violations,
                        "SPECIMEN_ITEM_ID_REPLAY",
                        &format!("{sampling_event_id}:{item_id}"),
                    );
                    push(
                        &mut blockers,
                        "SPECIMEN_SOURCE_IDENTITY_UNRESOLVED",
                        specimen_id,
                    );
                    specimen_source_identity_issue = true;
                }
            }
            _ => {
                push(&mut violations, "SPECIMEN_ITEM_ID_MISSING", specimen_id);
                push(
                    &mut blockers,
                    "SPECIMEN_SOURCE_IDENTITY_UNRESOLVED",
                    specimen_id,
                );
                specimen_source_identity_issue = true;
            }
        }
    }

    let mut specimen_identity_ok = !specimen_source_identity_issue;
    for specimen in &specimens {
        let specimen_id = field_str(specimen, "id").unwrap_or("specimen").to_owned();
        let Some(panel_id) = field_str(specimen, "panel_id") else {
            push(
                &mut violations,
                "SPECIMEN_PANEL_REFERENCE_MISSING",
                &specimen_id,
            );
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        let Some(panel) = panel_index.get(panel_id) else {
            push(
                &mut violations,
                "SPECIMEN_PANEL_REFERENCE_UNKNOWN",
                &specimen_id,
            );
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        let Some(batch_id) = field_str(panel, "process_batch_id") else {
            push(&mut violations, "PANEL_BATCH_REFERENCE_MISSING", panel_id);
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        let Some(batch) = batch_index.get(batch_id) else {
            push(&mut violations, "PANEL_BATCH_REFERENCE_UNKNOWN", panel_id);
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        let Some(lot_id) = field_str(batch, "material_lot_id") else {
            push(&mut violations, "BATCH_LOT_REFERENCE_MISSING", batch_id);
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        if !lot_index.contains_key(lot_id) {
            push(&mut violations, "BATCH_LOT_REFERENCE_UNKNOWN", batch_id);
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        }

        let assertions = assertion_rows(&specimen_assertions, &specimen_id, "specimen_id");
        let expected = [
            ("declared_material_lot_id", lot_id),
            ("declared_process_batch_id", batch_id),
            ("declared_panel_id", panel_id),
        ];
        let valid = check_independent_identity(
            &specimen_id,
            &assertions,
            &expected,
            "SPECIMEN_SOURCE",
            &mut violations,
            &mut blockers,
        );
        specimen_identity_ok &= valid;
    }

    let mut configuration_identity_ok = true;
    for specimen in &specimens {
        let specimen_id = field_str(specimen, "id").unwrap_or("specimen");
        let Some(configuration_id) = field_str(specimen, "configuration_id") else {
            push(
                &mut violations,
                "SPECIMEN_CONFIGURATION_REFERENCE_MISSING",
                specimen_id,
            );
            push(
                &mut blockers,
                "INTENDED_CONFIGURATION_UNRESOLVED",
                specimen_id,
            );
            configuration_identity_ok = false;
            continue;
        };
        if !configuration_index.contains_key(configuration_id) {
            push(
                &mut violations,
                "SPECIMEN_CONFIGURATION_REFERENCE_UNKNOWN",
                specimen_id,
            );
            push(
                &mut blockers,
                "INTENDED_CONFIGURATION_UNRESOLVED",
                specimen_id,
            );
            configuration_identity_ok = false;
            continue;
        }

        let assertions = assertion_rows(&configuration_assertions, specimen_id, "specimen_id");
        let expected = [("declared_intended_configuration_id", configuration_id)];
        let valid = check_independent_identity(
            specimen_id,
            &assertions,
            &expected,
            "INTENDED_CONFIGURATION",
            &mut violations,
            &mut blockers,
        );
        configuration_identity_ok &= valid;
    }

    let mut event_ids = BTreeSet::new();
    let mut corrected_event_targets = BTreeMap::<String, String>::new();
    for event in &events {
        let event_id = field_str(event, "id").unwrap_or("event");
        if !event_ids.insert(event_id.to_owned()) {
            push(&mut violations, "CALIBRATION_EVENT_REPLAY", event_id);
            push(
                &mut blockers,
                "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                event_id,
            );
        }
        let certificate_id = field_str(event, "certificate_id").unwrap_or("certificate");
        if !certificate_index.contains_key(certificate_id) {
            push(
                &mut violations,
                "CALIBRATION_EVENT_CERTIFICATE_UNKNOWN",
                event_id,
            );
            push(
                &mut blockers,
                "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                event_id,
            );
        }
        for time_field in ["effective_at_unix_ms", "recorded_at_unix_ms"] {
            if field_u64(event, time_field).is_none() {
                push(
                    &mut violations,
                    "INVALID_CALIBRATION_EVENT_TIMESTAMP",
                    event_id,
                );
                push(
                    &mut blockers,
                    "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                    event_id,
                );
            }
        }
        if let Err(reason) = hard_time_interval(
            event,
            "effective_at_unix_ms",
            "effective_time_earliest_unix_ms",
            "effective_time_latest_unix_ms",
        ) {
            push(&mut violations, reason, event_id);
            push(
                &mut blockers,
                "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                event_id,
            );
        }
        match field_str(event, "event_type") {
            Some("corrected" | "superseded") => {
                let target_id = field_str(event, "corrects_event_id");
                match target_id.and_then(|target| event_index.get(target).map(|row| (target, *row)))
                {
                    Some((target, target_event))
                        if field_str(target_event, "certificate_id") == Some(certificate_id)
                            && target != event_id
                            && field_u64(target_event, "recorded_at_unix_ms")
                                .zip(field_u64(event, "recorded_at_unix_ms"))
                                .is_some_and(|(prior, current)| prior < current) =>
                    {
                        if corrected_event_targets
                            .insert(target.to_owned(), event_id.to_owned())
                            .is_some()
                        {
                            push(
                                &mut violations,
                                "CONTRADICTORY_CALIBRATION_CORRECTIONS",
                                target,
                            );
                            push(
                                &mut blockers,
                                "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                                target,
                            );
                        }
                    }
                    _ => {
                        push(
                            &mut violations,
                            "CALIBRATION_CORRECTION_TARGET_INVALID",
                            event_id,
                        );
                        push(
                            &mut blockers,
                            "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                            event_id,
                        );
                    }
                }
            }
            Some("revoked" | "suspended") => {}
            _ => {
                push(&mut violations, "UNKNOWN_CALIBRATION_EVENT_TYPE", event_id);
                push(
                    &mut blockers,
                    "CALIBRATION_EVENT_HISTORY_UNRESOLVED",
                    event_id,
                );
            }
        }
    }

    for observation in &observations {
        let observation_id = field_str(observation, "id").unwrap_or("observation");
        let mut reasons = BTreeSet::new();
        let mut non_applicable_event_ids = BTreeSet::new();
        let acquisition_interval = match hard_time_interval(
            observation,
            "acquired_at_unix_ms",
            "acquisition_time_earliest_unix_ms",
            "acquisition_time_latest_unix_ms",
        ) {
            Ok(interval) => Some(interval),
            Err("TIME_POINT_MISSING_OR_INVALID") => {
                reasons.insert("ACQUISITION_TIMESTAMP_MISSING_OR_INVALID".to_owned());
                None
            }
            Err(reason) => {
                reasons.insert(reason.to_owned());
                None
            }
        };

        if field_str(observation, "time_basis") != Some("UTC_UNIX_EPOCH_MS") {
            reasons.insert("TIME_BASIS_NOT_NORMALIZED_TO_UNIX_MS".to_owned());
        }
        if let (Some(interval), Some(evaluated)) = (acquisition_interval, evaluated_at) {
            if interval.earliest_unix_ms > evaluated {
                reasons.insert("ACQUISITION_AFTER_EVALUATION_TIME".to_owned());
            } else if interval.latest_unix_ms > evaluated {
                reasons.insert(
                    "ACQUISITION_TIME_BOUNDARY_AMBIGUOUS_AT_EVALUATION_SNAPSHOT".to_owned(),
                );
            }
        }

        let specimen_id = field_str(observation, "specimen_id").unwrap_or("specimen");
        if !specimen_index.contains_key(specimen_id) {
            reasons.insert("OBSERVATION_SPECIMEN_UNKNOWN".to_owned());
        }

        let campaign_id = field_str(observation, "campaign_id").unwrap_or("campaign");
        let campaign = campaign_index.get(campaign_id);
        if campaign.is_none() {
            reasons.insert("CAMPAIGN_IDENTITY_UNKNOWN".to_owned());
        }

        let certificate_id = campaign
            .and_then(|row| field_str(row, "calibration_id"))
            .unwrap_or("certificate");
        let certificate = certificate_index.get(certificate_id);
        if certificate.is_none() {
            reasons.insert("CALIBRATION_CERTIFICATE_MISSING".to_owned());
        }

        if let (Some(campaign), Some(certificate)) = (campaign, certificate) {
            for required_field in [
                "instrument_id",
                "method_revision_id",
                "calibration_lab",
                "certificate_artifact_ref",
            ] {
                if !nonempty(field_str(certificate, required_field)) {
                    reasons.insert("CALIBRATION_CERTIFICATE_METADATA_INCOMPLETE".to_owned());
                }
            }
            if field_str(campaign, "instrument_id") != field_str(certificate, "instrument_id") {
                reasons.insert("CALIBRATION_INSTRUMENT_MISMATCH".to_owned());
            }
            if field_str(campaign, "protocol_revision_id")
                != field_str(certificate, "method_revision_id")
            {
                reasons.insert("CALIBRATION_METHOD_REVISION_MISMATCH".to_owned());
            }
            match field_str(certificate, "authenticity_status") {
                Some("authenticated") => {}
                Some("unauthenticated") => {
                    reasons.insert("CALIBRATION_CERTIFICATE_UNAUTHENTICATED".to_owned());
                }
                _ => {
                    reasons.insert("CALIBRATION_CERTIFICATE_AUTHENTICITY_UNKNOWN".to_owned());
                }
            }

            let valid_from = field_u64(certificate, "valid_from_unix_ms");
            let valid_until = field_u64(certificate, "valid_until_unix_ms");
            match (valid_from, valid_until, acquisition_interval) {
                (Some(start), Some(end), Some(acquired))
                    if start < end
                        && acquired.earliest_unix_ms >= start
                        && acquired.latest_unix_ms < end => {}
                (Some(start), Some(end), Some(acquired)) if start < end => {
                    if acquired.latest_unix_ms < start || acquired.earliest_unix_ms >= end {
                        reasons.insert("CALIBRATION_OUTSIDE_VALIDITY_INTERVAL".to_owned());
                    } else {
                        reasons.insert("CALIBRATION_VALIDITY_BOUNDARY_AMBIGUOUS".to_owned());
                    }
                }
                (Some(start), Some(end), None) if start < end => {
                    reasons.insert("CALIBRATION_ACQUISITION_TIME_UNRESOLVED".to_owned());
                }
                _ => {
                    reasons.insert("CALIBRATION_INTERVAL_INVALID".to_owned());
                }
            }

            for event in &events {
                if field_str(event, "certificate_id") != Some(certificate_id) {
                    continue;
                }
                let event_id = field_str(event, "id").unwrap_or("event");
                let Some(recorded_at) = field_u64(event, "recorded_at_unix_ms") else {
                    continue;
                };
                let Some(evaluated) = evaluated_at else {
                    continue;
                };
                // Recorded time remains a source assertion and is not promoted
                // to receiver-first-seen evidence by this evaluator.
                if recorded_at > evaluated {
                    continue;
                }
                let event_interval = match hard_time_interval(
                    event,
                    "effective_at_unix_ms",
                    "effective_time_earliest_unix_ms",
                    "effective_time_latest_unix_ms",
                ) {
                    Ok(interval) => interval,
                    Err(_) => {
                        reasons.insert("CALIBRATION_EVENT_EFFECTIVITY_BOUNDS_INVALID".to_owned());
                        continue;
                    }
                };
                let snapshot = TimeInterval {
                    earliest_unix_ms: evaluated,
                    latest_unix_ms: evaluated,
                };
                match compare_effectivity_intervals(event_interval, snapshot).relation {
                    EffectivityRelation::DefinitelyNotYetEffective => {
                        non_applicable_event_ids.insert(event_id.to_owned());
                        continue;
                    }
                    EffectivityRelation::BoundaryAmbiguous => {
                        reasons.insert(
                            "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_SNAPSHOT"
                                .to_owned(),
                        );
                        continue;
                    }
                    EffectivityRelation::InvalidInterval => {
                        reasons.insert("CALIBRATION_EVENT_EFFECTIVITY_BOUNDS_INVALID".to_owned());
                        continue;
                    }
                    EffectivityRelation::DefinitelyEffective => {}
                }

                let acquisition = match acquisition_interval {
                    Some(interval) => interval,
                    None => {
                        reasons.insert("CALIBRATION_EVENT_ACQUISITION_TIME_UNRESOLVED".to_owned());
                        continue;
                    }
                };
                match compare_effectivity_intervals(event_interval, acquisition).relation {
                    EffectivityRelation::DefinitelyNotYetEffective => {
                        if event.get("retroactive").and_then(Value::as_bool) == Some(true) {
                            reasons.insert(
                                "CALIBRATION_RETROACTIVE_FLAG_CONTRADICTS_EFFECTIVITY".to_owned(),
                            );
                        } else {
                            non_applicable_event_ids.insert(event_id.to_owned());
                        }
                        continue;
                    }
                    EffectivityRelation::BoundaryAmbiguous => {
                        reasons.insert(
                            "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_ACQUISITION"
                                .to_owned(),
                        );
                        continue;
                    }
                    EffectivityRelation::InvalidInterval => {
                        reasons.insert("CALIBRATION_EVENT_EFFECTIVITY_BOUNDS_INVALID".to_owned());
                        continue;
                    }
                    EffectivityRelation::DefinitelyEffective => {}
                }

                match field_str(event, "event_type") {
                    Some("revoked") => {
                        let code = "CALIBRATION_REVOKED_PENDING_DISPOSITION";
                        reasons.insert(code.to_owned());
                    }
                    Some("suspended") => {
                        reasons.insert("CALIBRATION_SUSPENDED_PENDING_DISPOSITION".to_owned());
                    }
                    Some("corrected" | "superseded") => {
                        reasons.insert("CALIBRATION_CHANGE_PENDING_DISPOSITION".to_owned());
                    }
                    _ => {}
                }
            }
        }

        let status = if reasons.is_empty() {
            "METROLOGY_BINDING_CONSISTENT_ONLY"
        } else {
            "BLOCKED"
        };
        let reasons = reasons.into_iter().collect::<Vec<_>>();
        for reason in &reasons {
            push(&mut blockers, reason, observation_id);
        }
        dispositions.push(MeasurementDisposition {
            observation_id: observation_id.to_owned(),
            status: status.to_owned(),
            reasons,
            non_applicable_event_ids: non_applicable_event_ids.into_iter().collect(),
        });
    }

    dispositions.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    sort_findings(&mut violations);
    sort_findings(&mut blockers);

    let structurally_complete = violations.is_empty()
        && specimen_identity_ok
        && configuration_identity_ok
        && blockers.is_empty();
    let specimen_identity_status = if specimen_identity_ok {
        "INDEPENDENT_IDENTITY_CONSISTENT"
    } else {
        "UNKNOWN_OR_CONFLICT"
    };
    let intended_configuration_status = if configuration_identity_ok {
        "INTENDED_CONFIGURATION_CONSISTENT"
    } else {
        "UNKNOWN_OR_CONFLICT"
    };
    let metrology_binding_status = if blockers.is_empty() && violations.is_empty() {
        "METROLOGY_BINDING_CONSISTENT_ONLY"
    } else {
        "BLOCKED_OR_INDETERMINATE"
    };

    BindingReport {
        evaluator_profile: "aero-evidence-binding-evaluator-v0.2".to_owned(),
        case_id,
        structurally_complete,
        specimen_identity_status: specimen_identity_status.to_owned(),
        intended_configuration_status: intended_configuration_status.to_owned(),
        metrology_binding_status: metrology_binding_status.to_owned(),
        decision_status: "NOT_ELIGIBLE_SYNTHETIC_ONLY".to_owned(),
        physical_qualification_status: "OUT_OF_SCOPE_NOT_CLAIMED".to_owned(),
        violations,
        blockers,
        measurement_dispositions: dispositions,
        capability_gaps: vec![
            "CERTIFICATE_AUTHENTICITY_IS_AN_IMPORTED_ASSERTION_NOT_CRYPTOGRAPHICALLY_VERIFIED"
                .to_owned(),
            "SOURCE_RECORD_PROVENANCE_IS_NOT_PROOF_OF_PHYSICAL_IDENTITY".to_owned(),
            "PHYSICAL_MEASUREMENT_TRUTH_AND_LAB_METHOD_VALIDATION_REMAIN_OUTSIDE_THIS_EVALUATOR"
                .to_owned(),
            "MYCELIX_RETAINS_OWNERSHIP_OF_ARTIFACT_DIGEST_AND_BYTE_LENGTH_INTEGRITY".to_owned(),
            "NO_MATERIAL_ALLOWABLE_PROCESS_CAPABILITY_PART_ACCEPTANCE_OR_AIRWORTHINESS_CLAIM"
                .to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate_binding_case;
    use serde_json::Value;

    const FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/synthetic_aero_evidence_binding_case_v0_2.json"
    );
    const MUTATION_ORACLE: &str = include_str!(
        "../../../../docs/research/aviation/examples/adversarial_evidence_binding_mutations_v0_2.json"
    );

    fn fixture() -> Value {
        serde_json::from_str(FIXTURE).expect("v0.2 synthetic fixture parses")
    }

    fn has_blocker(report: &super::BindingReport, code: &str) -> bool {
        report.blockers.iter().any(|finding| finding.code == code)
    }

    fn has_violation(report: &super::BindingReport, code: &str) -> bool {
        report.violations.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn fixture_is_consistent_but_never_qualified() {
        let report = evaluate_binding_case(&fixture());
        assert!(report.structurally_complete, "{report:?}");
        assert_eq!(
            report.specimen_identity_status,
            "INDEPENDENT_IDENTITY_CONSISTENT"
        );
        assert_eq!(
            report.intended_configuration_status,
            "INTENDED_CONFIGURATION_CONSISTENT"
        );
        assert_eq!(
            report.metrology_binding_status,
            "METROLOGY_BINDING_CONSISTENT_ONLY"
        );
        assert_eq!(report.decision_status, "NOT_ELIGIBLE_SYNTHETIC_ONLY");
        assert_eq!(
            report.physical_qualification_status,
            "OUT_OF_SCOPE_NOT_CLAIMED"
        );
    }

    #[test]
    fn wrong_declared_lot_is_not_hidden_by_a_valid_hierarchy() {
        let mut case = fixture();
        case["specimen_source_assertions"][0]["declared_material_lot_id"] =
            Value::String("LOT-WRONG".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_violation(
            &report,
            "SPECIMEN_SOURCE_DECLARED_IDENTITY_MISMATCH"
        ));
        assert!(has_blocker(&report, "SPECIMEN_SOURCE_IDENTITY_CONFLICT"));
    }

    #[test]
    fn hierarchy_mutation_is_detected_against_independent_source_assertion() {
        let mut case = fixture();
        case["hierarchy"]["material_lots"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"id":"LOT-B"}));
        case["hierarchy"]["process_batches"][0]["material_lot_id"] =
            Value::String("LOT-B".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_violation(
            &report,
            "SPECIMEN_SOURCE_DECLARED_IDENTITY_MISMATCH"
        ));
    }

    #[test]
    fn intended_configuration_mismatch_abstains() {
        let mut case = fixture();
        case["intended_configuration_assertions"][0]["declared_intended_configuration_id"] =
            Value::String("CONFIG-B".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_violation(
            &report,
            "INTENDED_CONFIGURATION_DECLARED_IDENTITY_MISMATCH"
        ));
    }

    #[test]
    fn calibration_start_boundary_is_inclusive() {
        let mut case = fixture();
        case["calibration_certificates"][0]["valid_from_unix_ms"] =
            Value::from(1_795_514_400_000u64);
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(
            &report,
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
    }

    #[test]
    fn calibration_end_boundary_is_exclusive() {
        let mut case = fixture();
        case["calibration_certificates"][0]["valid_until_unix_ms"] =
            Value::from(1_795_514_400_000u64);
        let report = evaluate_binding_case(&case);
        assert!(has_blocker(
            &report,
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
    }

    #[test]
    fn final_millisecond_before_exclusive_end_is_still_valid() {
        let mut case = fixture();
        case["evaluated_at_unix_ms"] = Value::from(1_795_518_000_000u64);
        case["observations"][0]["acquired_at_unix_ms"] = Value::from(1_795_517_999_999u64);
        assert!(!has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
    }

    #[test]
    fn acquisition_after_evaluation_time_is_blocked() {
        let mut case = fixture();
        case["observations"][0]["acquired_at_unix_ms"] = Value::from(1_795_516_200_001u64);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "ACQUISITION_AFTER_EVALUATION_TIME"
        ));
    }

    #[test]
    fn measurements_before_or_after_the_interval_are_blocked() {
        let mut before = fixture();
        before["observations"][0]["acquired_at_unix_ms"] = Value::from(1_795_510_799_999u64);
        assert!(has_blocker(
            &evaluate_binding_case(&before),
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));

        let mut after = fixture();
        after["observations"][0]["acquired_at_unix_ms"] = Value::from(1_795_518_000_000u64);
        assert!(has_blocker(
            &evaluate_binding_case(&after),
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
    }

    #[test]
    fn instrument_and_method_revision_must_match() {
        let mut instrument = fixture();
        instrument["calibration_certificates"][0]["instrument_id"] =
            Value::String("INSTRUMENT-OTHER".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&instrument),
            "CALIBRATION_INSTRUMENT_MISMATCH"
        ));

        let mut method = fixture();
        method["calibration_certificates"][0]["method_revision_id"] =
            Value::String("METHOD-OTHER".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&method),
            "CALIBRATION_METHOD_REVISION_MISMATCH"
        ));
    }

    #[test]
    fn missing_and_unauthenticated_certificates_do_not_pass() {
        let mut missing = fixture();
        missing["hierarchy"]["test_campaigns"][0]["calibration_id"] =
            Value::String("CAL-MISSING".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&missing),
            "CALIBRATION_CERTIFICATE_MISSING"
        ));

        let mut unauthenticated = fixture();
        unauthenticated["calibration_certificates"][0]["authenticity_status"] =
            Value::String("unauthenticated".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&unauthenticated),
            "CALIBRATION_CERTIFICATE_UNAUTHENTICATED"
        ));
    }

    #[test]
    fn unknown_timezone_and_ambiguous_timestamp_abstain() {
        let mut timezone = fixture();
        timezone["observations"][0]["time_basis"] = Value::String("LOCAL_UNKNOWN".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&timezone),
            "TIME_BASIS_NOT_NORMALIZED_TO_UNIX_MS"
        ));

        let mut ambiguous = fixture();
        ambiguous["observations"][0]["acquired_at_unix_ms"] = Value::from(-1);
        assert!(has_blocker(
            &evaluate_binding_case(&ambiguous),
            "ACQUISITION_TIMESTAMP_MISSING_OR_INVALID"
        ));
    }

    #[test]
    fn revocation_definitely_after_acquisition_does_not_rewrite_historical_binding() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-AFTER",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired + 1_000,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(
            &report,
            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
        ));
        assert_eq!(
            report.measurement_dispositions[0].status,
            "METROLOGY_BINDING_CONSISTENT_ONLY"
        );
        assert_eq!(
            report.measurement_dispositions[0].non_applicable_event_ids,
            vec!["EVENT-REVOKE-AFTER".to_owned()]
        );
    }

    #[test]
    fn acquisition_hard_bounds_inside_calibration_validity_are_admitted_for_binding() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["observations"][0]["acquisition_time_earliest_unix_ms"] = Value::from(acquired - 50);
        case["observations"][0]["acquisition_time_latest_unix_ms"] = Value::from(acquired + 50);
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(
            &report,
            "CALIBRATION_VALIDITY_BOUNDARY_AMBIGUOUS"
        ));
        assert!(!has_blocker(
            &report,
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
        assert_eq!(
            report.measurement_dispositions[0].status,
            "METROLOGY_BINDING_CONSISTENT_ONLY"
        );
    }

    #[test]
    fn acquisition_hard_bounds_crossing_calibration_start_abstain() {
        let mut case = fixture();
        let start = 1_795_510_800_000u64;
        case["observations"][0]["acquired_at_unix_ms"] = Value::from(start);
        case["observations"][0]["acquisition_time_earliest_unix_ms"] = Value::from(start - 1);
        case["observations"][0]["acquisition_time_latest_unix_ms"] = Value::from(start + 1);
        let report = evaluate_binding_case(&case);
        assert!(has_blocker(
            &report,
            "CALIBRATION_VALIDITY_BOUNDARY_AMBIGUOUS"
        ));
    }

    #[test]
    fn acquisition_interval_crossing_evaluation_snapshot_abstains() {
        let mut case = fixture();
        let evaluated = 1_795_516_800_000u64;
        case["evaluated_at_unix_ms"] = Value::from(evaluated);
        case["observations"][0]["acquired_at_unix_ms"] = Value::from(evaluated);
        case["observations"][0]["acquisition_time_earliest_unix_ms"] = Value::from(evaluated - 1);
        case["observations"][0]["acquisition_time_latest_unix_ms"] = Value::from(evaluated + 1);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "ACQUISITION_TIME_BOUNDARY_AMBIGUOUS_AT_EVALUATION_SNAPSHOT"
        ));
    }

    #[test]
    fn event_effective_hard_bounds_before_acquisition_apply_revocation() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-BEFORE",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired - 1_000,
            "effective_time_earliest_unix_ms": acquired - 1_100,
            "effective_time_latest_unix_ms": acquired - 900,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
        ));
    }

    #[test]
    fn event_effective_hard_bounds_overlapping_acquisition_abstain() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-OVERLAP",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired,
            "effective_time_earliest_unix_ms": acquired - 10,
            "effective_time_latest_unix_ms": acquired + 10,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_ACQUISITION"
        ));
    }

    #[test]
    fn known_future_effective_event_is_retained_but_not_applied_at_snapshot() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 1_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-FUTURE-KNOWN",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired + 2_000,
            "recorded_at_unix_ms": acquired + 500,
            "retroactive": false
        }]);
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(
            &report,
            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
        ));
        assert_eq!(
            report.measurement_dispositions[0].non_applicable_event_ids,
            vec!["EVENT-FUTURE-KNOWN".to_owned()]
        );
    }

    #[test]
    fn event_recorded_after_snapshot_does_not_leak_into_as_known_report() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 1_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-RECORDED-LATE",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired - 1_000,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(
            &report,
            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
        ));
        assert!(
            report.measurement_dispositions[0]
                .non_applicable_event_ids
                .is_empty()
        );
    }

    #[test]
    fn event_effective_hard_bounds_overlapping_snapshot_abstain() {
        let mut case = fixture();
        let evaluated = 1_795_516_800_000u64;
        case["evaluated_at_unix_ms"] = Value::from(evaluated);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-SNAPSHOT-OVERLAP",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": evaluated,
            "effective_time_earliest_unix_ms": evaluated - 10,
            "effective_time_latest_unix_ms": evaluated + 10,
            "recorded_at_unix_ms": evaluated - 1,
            "retroactive": false
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_SNAPSHOT"
        ));
    }

    #[test]
    fn retroactive_flag_cannot_override_a_definitely_later_effective_interval() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-RETROACTIVE-CONTRADICTION",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired + 1_000,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": true
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_RETROACTIVE_FLAG_CONTRADICTS_EFFECTIVITY"
        ));
    }

    #[test]
    fn event_effective_exactly_at_acquisition_is_inclusive() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-EXACT",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
        ));
    }

    #[test]
    fn incomplete_event_effective_bounds_fail_closed() {
        let mut case = fixture();
        let acquired = 1_795_514_400_000u64;
        case["evaluated_at_unix_ms"] = Value::from(acquired + 10_000);
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-INCOMPLETE",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": acquired - 1_000,
            "effective_time_earliest_unix_ms": acquired - 1_100,
            "recorded_at_unix_ms": acquired + 2_000,
            "retroactive": false
        }]);
        assert!(has_blocker(
            &evaluate_binding_case(&case),
            "TIME_BOUNDS_PAIR_INCOMPLETE"
        ));
    }

    #[test]
    fn exhaustive_small_event_and_acquisition_bounds_match_interval_oracle() {
        let base = 1_795_514_400_000u64;
        for event_earliest in 0..=3 {
            for event_latest in event_earliest..=3 {
                for acquisition_earliest in 0..=3 {
                    for acquisition_latest in acquisition_earliest..=3 {
                        let mut case = fixture();
                        let acquisition_point =
                            base + (acquisition_earliest + acquisition_latest) / 2;
                        let event_point = base + (event_earliest + event_latest) / 2;
                        case["evaluated_at_unix_ms"] = Value::from(base + 10);
                        case["observations"][0]["acquired_at_unix_ms"] =
                            Value::from(acquisition_point);
                        case["observations"][0]["acquisition_time_earliest_unix_ms"] =
                            Value::from(base + acquisition_earliest);
                        case["observations"][0]["acquisition_time_latest_unix_ms"] =
                            Value::from(base + acquisition_latest);
                        case["calibration_events"] = serde_json::json!([{
                            "id": "EVENT-EXHAUSTIVE",
                            "certificate_id": "CAL-A",
                            "event_type": "revoked",
                            "effective_at_unix_ms": event_point,
                            "effective_time_earliest_unix_ms": base + event_earliest,
                            "effective_time_latest_unix_ms": base + event_latest,
                            "recorded_at_unix_ms": base + 5,
                            "retroactive": false
                        }]);

                        let report = evaluate_binding_case(&case);
                        let definitely_effective = event_latest <= acquisition_earliest;
                        let definitely_not_yet_effective = event_earliest > acquisition_latest;
                        if definitely_effective {
                            assert!(has_blocker(
                                &report,
                                "CALIBRATION_REVOKED_PENDING_DISPOSITION"
                            ));
                            assert!(!has_blocker(
                                &report,
                                "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_ACQUISITION"
                            ));
                        } else if definitely_not_yet_effective {
                            assert!(!has_blocker(
                                &report,
                                "CALIBRATION_REVOKED_PENDING_DISPOSITION"
                            ));
                            assert_eq!(
                                report.measurement_dispositions[0].non_applicable_event_ids,
                                vec!["EVENT-EXHAUSTIVE".to_owned()]
                            );
                        } else {
                            assert!(has_blocker(
                                &report,
                                "CALIBRATION_EVENT_EFFECTIVITY_BOUNDARY_AMBIGUOUS_AT_ACQUISITION"
                            ));
                            assert!(!has_blocker(
                                &report,
                                "CALIBRATION_REVOKED_PENDING_DISPOSITION"
                            ));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn incomplete_or_malformed_time_bounds_fail_closed() {
        let mut incomplete = fixture();
        incomplete["observations"][0]["acquisition_time_earliest_unix_ms"] =
            Value::from(1_795_514_399_999u64);
        assert!(has_blocker(
            &evaluate_binding_case(&incomplete),
            "TIME_BOUNDS_PAIR_INCOMPLETE"
        ));

        let mut reversed = fixture();
        reversed["observations"][0]["acquisition_time_earliest_unix_ms"] =
            Value::from(1_795_514_400_001u64);
        reversed["observations"][0]["acquisition_time_latest_unix_ms"] =
            Value::from(1_795_514_400_000u64);
        assert!(has_blocker(
            &evaluate_binding_case(&reversed),
            "TIME_BOUNDS_START_AFTER_END"
        ));

        let mut outside = fixture();
        outside["observations"][0]["acquisition_time_earliest_unix_ms"] =
            Value::from(1_795_514_400_001u64);
        outside["observations"][0]["acquisition_time_latest_unix_ms"] =
            Value::from(1_795_514_400_010u64);
        assert!(has_blocker(
            &evaluate_binding_case(&outside),
            "TIME_POINT_OUTSIDE_DECLARED_BOUNDS"
        ));
    }

    #[test]
    fn duplicate_certificate_and_replayed_event_ids_are_rejected() {
        let mut duplicate_certificate = fixture();
        let duplicate = duplicate_certificate["calibration_certificates"][0].clone();
        duplicate_certificate["calibration_certificates"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(has_violation(
            &evaluate_binding_case(&duplicate_certificate),
            "DUPLICATE_ENTITY_ID"
        ));

        let mut replayed_event = fixture();
        let event = serde_json::json!({
            "id": "EVENT-1",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "effective_at_unix_ms": 1_795_516_200_000u64,
            "recorded_at_unix_ms": 1_795_516_500_000u64,
            "retroactive": true
        });
        replayed_event["calibration_events"] = serde_json::json!([event.clone(), event]);
        assert!(has_violation(
            &evaluate_binding_case(&replayed_event),
            "CALIBRATION_EVENT_REPLAY"
        ));
    }

    #[test]
    fn correction_events_require_a_unique_prior_event_target() {
        let mut missing_target = fixture();
        missing_target["calibration_events"] = serde_json::json!([{
            "id": "EVENT-CORRECT-1",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "corrects_event_id": "EVENT-NOT-FOUND",
            "effective_at_unix_ms": 1_795_516_200_000u64,
            "recorded_at_unix_ms": 1_795_516_500_000u64,
            "retroactive": true
        }]);
        assert!(has_violation(
            &evaluate_binding_case(&missing_target),
            "CALIBRATION_CORRECTION_TARGET_INVALID"
        ));

        let mut contradictory = fixture();
        let original = serde_json::json!({
            "id": "EVENT-ORIGINAL",
            "certificate_id": "CAL-A",
            "event_type": "suspended",
            "effective_at_unix_ms": 1_795_515_300_000u64,
            "recorded_at_unix_ms": 1_795_515_360_000u64,
            "retroactive": false
        });
        let correction_a = serde_json::json!({
            "id": "EVENT-CORRECT-A",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_unix_ms": 1_795_515_600_000u64,
            "recorded_at_unix_ms": 1_795_515_660_000u64,
            "retroactive": true
        });
        let correction_b = serde_json::json!({
            "id": "EVENT-CORRECT-B",
            "certificate_id": "CAL-A",
            "event_type": "superseded",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_unix_ms": 1_795_515_720_000u64,
            "recorded_at_unix_ms": 1_795_515_780_000u64,
            "retroactive": true
        });
        contradictory["calibration_events"] =
            serde_json::json!([original, correction_a, correction_b]);
        assert!(has_violation(
            &evaluate_binding_case(&contradictory),
            "CONTRADICTORY_CALIBRATION_CORRECTIONS"
        ));
    }

    #[test]
    fn frozen_v0_2_mutation_dispositions_match_exact_expected_sets() {
        fn apply_one(case: &mut Value, operation: &str, pointer: &str, value: &Value) {
            match operation {
                "set" => {
                    let target = case
                        .pointer_mut(pointer)
                        .unwrap_or_else(|| panic!("mutation pointer exists: {pointer}"));
                    *target = value.clone();
                }
                "append" => {
                    let target = case
                        .pointer_mut(pointer)
                        .and_then(Value::as_array_mut)
                        .unwrap_or_else(|| panic!("append pointer is an array: {pointer}"));
                    target.push(value.clone());
                }
                other => panic!("unsupported mutation operation: {other}"),
            }
        }

        let oracle: Value =
            serde_json::from_str(MUTATION_ORACLE).expect("v0.2 mutation oracle parses");
        assert_eq!(
            oracle.get("synthetic_only").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            oracle.get("oracle_review_status").and_then(Value::as_str),
            Some("NOT_INDEPENDENTLY_REVIEWED")
        );
        let cases = oracle
            .get("cases")
            .and_then(Value::as_array)
            .expect("v0.2 mutation cases");
        assert_eq!(cases.len(), 31);

        for mutation in cases {
            let case_id = mutation
                .get("case_id")
                .and_then(Value::as_str)
                .expect("mutation case ID");
            let mut case = fixture();

            match mutation.get("operation").and_then(Value::as_str) {
                Some("set" | "append") => {
                    apply_one(
                        &mut case,
                        mutation["operation"].as_str().expect("operation"),
                        mutation["pointer"].as_str().expect("pointer"),
                        mutation.get("value").expect("mutation value"),
                    );
                }
                Some("composite") => {
                    let steps = mutation["mutations"]
                        .as_array()
                        .expect("composite mutation steps");
                    for step in steps {
                        apply_one(
                            &mut case,
                            step["operation"].as_str().expect("step operation"),
                            step["pointer"].as_str().expect("step pointer"),
                            step.get("value").expect("step value"),
                        );
                    }
                }
                other => panic!("{case_id}: unsupported operation {other:?}"),
            }

            let report = evaluate_binding_case(&case);
            assert_eq!(
                report.structurally_complete,
                mutation["expected_structurally_complete"]
                    .as_bool()
                    .expect("expected structural status"),
                "{case_id}: structural disposition"
            );

            let expected_violations = mutation["expected_violation_codes"]
                .as_array()
                .expect("expected violations")
                .iter()
                .map(|code| code.as_str().expect("violation code").to_owned())
                .collect::<std::collections::BTreeSet<_>>();
            let actual_violations = report
                .violations
                .iter()
                .map(|finding| finding.code.clone())
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                actual_violations, expected_violations,
                "{case_id}: exact violation set"
            );

            let expected_blockers = mutation["expected_blocker_codes"]
                .as_array()
                .expect("expected blockers")
                .iter()
                .map(|code| code.as_str().expect("blocker code").to_owned())
                .collect::<std::collections::BTreeSet<_>>();
            let actual_blockers = report
                .blockers
                .iter()
                .map(|finding| finding.code.clone())
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(
                actual_blockers, expected_blockers,
                "{case_id}: exact blocker set"
            );
        }
    }

    #[test]
    fn calibration_event_permutations_preserve_the_same_report() {
        let mut case = fixture();
        let original = serde_json::json!({
            "id": "EVENT-ORIGINAL",
            "certificate_id": "CAL-A",
            "event_type": "suspended",
            "effective_at_unix_ms": 1_795_515_300_000u64,
            "recorded_at_unix_ms": 1_795_515_360_000u64,
            "retroactive": false
        });
        let correction_a = serde_json::json!({
            "id": "EVENT-CORRECT-A",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_unix_ms": 1_795_515_600_000u64,
            "recorded_at_unix_ms": 1_795_515_660_000u64,
            "retroactive": true
        });
        let correction_b = serde_json::json!({
            "id": "EVENT-CORRECT-B",
            "certificate_id": "CAL-A",
            "event_type": "superseded",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_unix_ms": 1_795_515_720_000u64,
            "recorded_at_unix_ms": 1_795_515_780_000u64,
            "retroactive": true
        });
        case["calibration_events"] = serde_json::json!([original, correction_a, correction_b]);

        let baseline = evaluate_binding_case(&case);
        assert!(has_violation(
            &baseline,
            "CONTRADICTORY_CALIBRATION_CORRECTIONS"
        ));

        let mut permuted = case;
        permuted["calibration_events"]
            .as_array_mut()
            .expect("calibration events")
            .reverse();
        assert_eq!(baseline, evaluate_binding_case(&permuted));
    }

    #[test]
    fn conflicting_duplicate_event_ids_are_ambiguous_and_permutation_invariant() {
        let mut case = fixture();
        let duplicate_a = serde_json::json!({
            "id": "EVENT-DUPLICATE",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_unix_ms": 1_795_515_300_000u64,
            "recorded_at_unix_ms": 1_795_515_360_000u64,
            "retroactive": false
        });
        let duplicate_b = serde_json::json!({
            "id": "EVENT-DUPLICATE",
            "certificate_id": "CAL-B",
            "event_type": "suspended",
            "effective_at_unix_ms": 1_795_515_330_000u64,
            "recorded_at_unix_ms": 1_795_515_390_000u64,
            "retroactive": false
        });
        let correction = serde_json::json!({
            "id": "EVENT-CORRECTION",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "corrects_event_id": "EVENT-DUPLICATE",
            "effective_at_unix_ms": 1_795_515_420_000u64,
            "recorded_at_unix_ms": 1_795_515_450_000u64,
            "retroactive": true
        });
        case["calibration_events"] = serde_json::json!([duplicate_a, duplicate_b, correction]);

        let baseline = evaluate_binding_case(&case);
        assert!(has_violation(&baseline, "CALIBRATION_EVENT_REPLAY"));
        assert!(has_violation(
            &baseline,
            "CALIBRATION_EVENT_CERTIFICATE_UNKNOWN"
        ));
        assert!(has_violation(
            &baseline,
            "CALIBRATION_CORRECTION_TARGET_INVALID"
        ));
        assert!(has_blocker(
            &baseline,
            "CALIBRATION_EVENT_HISTORY_UNRESOLVED"
        ));

        let mut permuted = case;
        permuted["calibration_events"]
            .as_array_mut()
            .expect("calibration events")
            .reverse();
        assert_eq!(baseline, evaluate_binding_case(&permuted));
    }

    #[test]
    fn record_order_permutations_produce_the_same_report() {
        let case = fixture();
        let baseline = evaluate_binding_case(&case);

        let mut permuted = case.clone();
        permuted["hierarchy"]["material_lots"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["hierarchy"]["process_batches"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["hierarchy"]["panels"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["hierarchy"]["specimens"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["specimen_source_assertions"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["intended_configuration_assertions"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["calibration_certificates"]
            .as_array_mut()
            .unwrap()
            .reverse();
        permuted["observations"].as_array_mut().unwrap().reverse();
        permuted["calibration_events"]
            .as_array_mut()
            .unwrap()
            .reverse();

        assert_eq!(baseline, evaluate_binding_case(&permuted));
    }
}
