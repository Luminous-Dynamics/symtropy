// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent specimen/configuration assertions and time-scoped metrology binding.
//!
//! This module checks the v0.2 synthetic evidence-binding contract. It does not
//! authenticate certificates cryptographically, prove that a physical
//! observation occurred, establish material allowables, qualify a process, or
//! approve an aircraft. A consistent binding is only a prerequisite for
//! evidence review by competent authorities.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const PROFILE_ID: &str = "aero-evidence-binding-v0.2";
const CLAIM_CEILING: &str = "synthetic_binding_contract_only";
const INTERVAL_SEMANTICS: &str = "[valid_from_utc, valid_until_utc)";

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
        if index.insert(id.to_owned(), *row).is_some() {
            push(violations, "DUPLICATE_ENTITY_ID", id);
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

fn valid_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return false;
    }

    let digits = |start: usize, end: usize| -> Option<u32> {
        let slice = bytes.get(start..end)?;
        if !slice.iter().all(u8::is_ascii_digit) {
            return None;
        }
        std::str::from_utf8(slice).ok()?.parse().ok()
    };

    let year = match digits(0, 4) {
        Some(value) if value >= 1970 => value,
        _ => return false,
    };
    let month = match digits(5, 7) {
        Some(value @ 1..=12) => value,
        _ => return false,
    };
    let day = match digits(8, 10) {
        Some(value) => value,
        None => return false,
    };
    let hour = match digits(11, 13) {
        Some(value @ 0..=23) => value,
        _ => return false,
    };
    let minute = match digits(14, 16) {
        Some(value @ 0..=59) => value,
        _ => return false,
    };
    let second = match digits(17, 19) {
        Some(value @ 0..=59) => value,
        _ => return false,
    };

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };

    day >= 1
        && day <= days_in_month
        && hour <= 23
        && minute <= 59
        && second <= 59
}

fn sort_findings(findings: &mut [BindingFinding]) {
    findings.sort_by(|left, right| {
        (&left.code, &left.subject_id).cmp(&(&right.code, &right.subject_id))
    });
}

fn assertion_rows<'a>(
    rows: &'a [&'a Value],
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
            for (verified_field, expected_id) in expected
                .iter()
                .map(|(declared, expected)| {
                    let verified = declared.replacen("declared_", "verified_", 1);
                    (verified, *expected)
                })
            {
                if field_str(assertion, &verified_field) != Some(verified_field_value(expected_id)) {
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
        push(&mut violations, "CLAIM_CEILING_OR_PROFILE_VIOLATION", "case");
    }

    let evaluated_at = field_str(case, "evaluated_at_utc");
    if !evaluated_at.is_some_and(valid_utc_timestamp) {
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
    let specimen_assertions =
        array_at(case, "/specimen_source_assertions", &mut violations);
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
        ("intended_configuration_assertions", configuration_assertions.as_slice()),
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
    let event_index = index_by_id(&events, "calibration_events", &mut violations);

    let mut source_record_ids = BTreeSet::new();
    let mut source_record_replay = false;
    for assertion in &specimen_assertions {
        if let Some(source_id) = field_str(assertion, "source_record_id") {
            if !source_record_ids.insert(source_id.to_owned()) {
                push(
                    &mut violations,
                    "SPECIMEN_SOURCE_RECORD_REPLAY",
                    source_id,
                );
                push(
                    &mut blockers,
                    "SPECIMEN_SOURCE_IDENTITY_UNRESOLVED",
                    source_id,
                );
                source_record_replay = true;
            }
        }
    }

    let mut specimen_identity_ok = !source_record_replay;
    for specimen in &specimens {
        let specimen_id = field_str(specimen, "id")
            .unwrap_or("specimen")
            .to_owned();
        let Some(panel_id) = field_str(specimen, "panel_id") else {
            push(&mut violations, "SPECIMEN_PANEL_REFERENCE_MISSING", &specimen_id);
            push(&mut blockers, "SPECIMEN_ANCESTRY_UNRESOLVED", &specimen_id);
            specimen_identity_ok = false;
            continue;
        };
        let Some(panel) = panel_index.get(panel_id) else {
            push(&mut violations, "SPECIMEN_PANEL_REFERENCE_UNKNOWN", &specimen_id);
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

        let assertions =
            assertion_rows(&specimen_assertions, &specimen_id, "specimen_id");
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

        let assertions = assertion_rows(
            &configuration_assertions,
            specimen_id,
            "specimen_id",
        );
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
        for time_field in ["effective_at_utc", "recorded_at_utc"] {
            if !field_str(event, time_field).is_some_and(valid_utc_timestamp) {
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
        match field_str(event, "event_type") {
            Some("corrected" | "superseded") => {
                let target_id = field_str(event, "corrects_event_id");
                match target_id.and_then(|target| event_index.get(target).map(|row| (target, *row))) {
                    Some((target, target_event))
                        if field_str(target_event, "certificate_id") == Some(certificate_id)
                            && target != event_id
                            && field_str(target_event, "recorded_at_utc")
                                .zip(field_str(event, "recorded_at_utc"))
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
                push(
                    &mut violations,
                    "UNKNOWN_CALIBRATION_EVENT_TYPE",
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

    for observation in &observations {
        let observation_id = field_str(observation, "id").unwrap_or("observation");
        let mut reasons = BTreeSet::new();
        let acquired_at = field_str(observation, "acquired_at_utc");

        if field_str(observation, "time_basis") != Some("UTC") {
            reasons.insert("TIME_BASIS_NOT_NORMALIZED_TO_UTC".to_owned());
        }
        if !acquired_at.is_some_and(valid_utc_timestamp) {
            reasons.insert("ACQUISITION_TIMESTAMP_INVALID_OR_AMBIGUOUS".to_owned());
        } else if evaluated_at.is_some_and(valid_utc_timestamp)
            && acquired_at.is_some_and(|time| time > evaluated_at.unwrap())
        {
            reasons.insert("ACQUISITION_AFTER_EVALUATION_TIME".to_owned());
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
            for required_field in ["instrument_id", "method_revision_id", "calibration_lab", "certificate_artifact_ref"] {
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

            let valid_from = field_str(certificate, "valid_from_utc");
            let valid_until = field_str(certificate, "valid_until_utc");
            if !valid_from.is_some_and(valid_utc_timestamp)
                || !valid_until.is_some_and(valid_utc_timestamp)
                || valid_from >= valid_until
            {
                reasons.insert("CALIBRATION_INTERVAL_INVALID".to_owned());
            } else if let Some(acquired) = acquired_at.filter(|time| valid_utc_timestamp(time)) {
                if acquired < valid_from.unwrap() || acquired >= valid_until.unwrap() {
                    reasons.insert("CALIBRATION_OUTSIDE_VALIDITY_INTERVAL".to_owned());
                }
            }

            for event in &events {
                if field_str(event, "certificate_id") != Some(certificate_id) {
                    continue;
                }
                let Some(effective_at) = field_str(event, "effective_at_utc") else {
                    continue;
                };
                let Some(recorded_at) = field_str(event, "recorded_at_utc") else {
                    continue;
                };
                if !valid_utc_timestamp(effective_at)
                    || !valid_utc_timestamp(recorded_at)
                    || !evaluated_at.is_some_and(valid_utc_timestamp)
                    || recorded_at > evaluated_at.unwrap()
                    || effective_at > evaluated_at.unwrap()
                {
                    continue;
                }

                match field_str(event, "event_type") {
                    Some("revoked") => {
                        let code = if acquired_at.is_some_and(|acquired| {
                            valid_utc_timestamp(acquired) && acquired < effective_at
                        }) && event.get("retroactive").and_then(Value::as_bool) != Some(true)
                        {
                            "CALIBRATION_REVOKED_AFTER_ACQUISITION_PENDING_DISPOSITION"
                        } else {
                            "CALIBRATION_REVOKED_PENDING_DISPOSITION"
                        };
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
            "CERTIFICATE_AUTHENTICITY_IS_AN_IMPORTED_ASSERTION_NOT_CRYPTOGRAPHICALLY_VERIFIED".to_owned(),
            "SOURCE_RECORD_PROVENANCE_IS_NOT_PROOF_OF_PHYSICAL_IDENTITY".to_owned(),
            "PHYSICAL_MEASUREMENT_TRUTH_AND_LAB_METHOD_VALIDATION_REMAIN_OUTSIDE_THIS_EVALUATOR".to_owned(),
            "MYCELIX_RETAINS_OWNERSHIP_OF_ARTIFACT_DIGEST_AND_BYTE_LENGTH_INTEGRITY".to_owned(),
            "NO_MATERIAL_ALLOWABLE_PROCESS_CAPABILITY_PART_ACCEPTANCE_OR_AIRWORTHINESS_CLAIM".to_owned(),
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
        assert_eq!(report.specimen_identity_status, "INDEPENDENT_IDENTITY_CONSISTENT");
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
        assert!(has_violation(&report, "SPECIMEN_SOURCE_DECLARED_IDENTITY_MISMATCH"));
        assert!(has_blocker(&report, "SPECIMEN_SOURCE_IDENTITY_CONFLICT"));
    }

    #[test]
    fn hierarchy_mutation_is_detected_against_independent_source_assertion() {
        let mut case = fixture();
        case["hierarchy"]["material_lots"].as_array_mut().unwrap().push(serde_json::json!({"id":"LOT-B"}));
        case["hierarchy"]["process_batches"][0]["material_lot_id"] =
            Value::String("LOT-B".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(!report.structurally_complete);
        assert!(has_violation(&report, "SPECIMEN_SOURCE_DECLARED_IDENTITY_MISMATCH"));
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
        case["calibration_certificates"][0]["valid_from_utc"] =
            Value::String("2026-11-24T10:00:00Z".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(!has_blocker(&report, "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"));
    }

    #[test]
    fn calibration_end_boundary_is_exclusive() {
        let mut case = fixture();
        case["calibration_certificates"][0]["valid_until_utc"] =
            Value::String("2026-11-24T10:00:00Z".to_owned());
        let report = evaluate_binding_case(&case);
        assert!(has_blocker(
            &report,
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));
    }

    #[test]
    fn measurements_before_or_after_the_interval_are_blocked() {
        let mut before = fixture();
        before["observations"][0]["acquired_at_utc"] =
            Value::String("2026-11-24T08:59:59Z".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&before),
            "CALIBRATION_OUTSIDE_VALIDITY_INTERVAL"
        ));

        let mut after = fixture();
        after["observations"][0]["acquired_at_utc"] =
            Value::String("2026-11-24T11:00:00Z".to_owned());
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
            "TIME_BASIS_NOT_NORMALIZED_TO_UTC"
        ));

        let mut ambiguous = fixture();
        ambiguous["observations"][0]["acquired_at_utc"] =
            Value::String("2026-11-24 10:00".to_owned());
        assert!(has_blocker(
            &evaluate_binding_case(&ambiguous),
            "ACQUISITION_TIMESTAMP_INVALID_OR_AMBIGUOUS"
        ));
    }

    #[test]
    fn revocation_after_acquisition_is_preserved_and_blocks_positive_binding() {
        let mut case = fixture();
        case["evaluated_at_utc"] = Value::String("2026-11-24T10:40:00Z".to_owned());
        case["calibration_events"] = serde_json::json!([{
            "id": "EVENT-REVOKE-1",
            "certificate_id": "CAL-A",
            "event_type": "revoked",
            "effective_at_utc": "2026-11-24T10:30:00Z",
            "recorded_at_utc": "2026-11-24T10:35:00Z",
            "retroactive": false
        }]);
        let report = evaluate_binding_case(&case);
        assert!(has_blocker(
            &report,
            "CALIBRATION_REVOKED_AFTER_ACQUISITION_PENDING_DISPOSITION"
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
            "effective_at_utc": "2026-11-24T10:30:00Z",
            "recorded_at_utc": "2026-11-24T10:35:00Z",
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
            "effective_at_utc": "2026-11-24T10:30:00Z",
            "recorded_at_utc": "2026-11-24T10:35:00Z",
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
            "effective_at_utc": "2026-11-24T10:15:00Z",
            "recorded_at_utc": "2026-11-24T10:16:00Z",
            "retroactive": false
        });
        let correction_a = serde_json::json!({
            "id": "EVENT-CORRECT-A",
            "certificate_id": "CAL-A",
            "event_type": "corrected",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_utc": "2026-11-24T10:20:00Z",
            "recorded_at_utc": "2026-11-24T10:21:00Z",
            "retroactive": true
        });
        let correction_b = serde_json::json!({
            "id": "EVENT-CORRECT-B",
            "certificate_id": "CAL-A",
            "event_type": "superseded",
            "corrects_event_id": "EVENT-ORIGINAL",
            "effective_at_utc": "2026-11-24T10:22:00Z",
            "recorded_at_utc": "2026-11-24T10:23:00Z",
            "retroactive": true
        });
        contradictory["calibration_events"] = serde_json::json!([original, correction_a, correction_b]);
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
            oracle
                .get("oracle_review_status")
                .and_then(Value::as_str),
            Some("NOT_INDEPENDENTLY_REVIEWED")
        );
        let cases = oracle
            .get("cases")
            .and_then(Value::as_array)
            .expect("v0.2 mutation cases");
        assert_eq!(cases.len(), 16);

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
    fn record_order_permutations_produce_the_same_report() {
        let case = fixture();
        let baseline = evaluate_binding_case(&case);

        let mut permuted = case.clone();
        permuted["hierarchy"]["material_lots"].as_array_mut().unwrap().reverse();
        permuted["hierarchy"]["process_batches"].as_array_mut().unwrap().reverse();
        permuted["hierarchy"]["panels"].as_array_mut().unwrap().reverse();
        permuted["hierarchy"]["specimens"].as_array_mut().unwrap().reverse();
        permuted["specimen_source_assertions"].as_array_mut().unwrap().reverse();
        permuted["intended_configuration_assertions"].as_array_mut().unwrap().reverse();
        permuted["calibration_certificates"].as_array_mut().unwrap().reverse();
        permuted["observations"].as_array_mut().unwrap().reverse();

        assert_eq!(baseline, evaluate_binding_case(&permuted));
    }
}
