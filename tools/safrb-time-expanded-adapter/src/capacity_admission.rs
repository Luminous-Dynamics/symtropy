#![forbid(unsafe_code)]

//! Structural admission of externally pre-admitted, time-bounded capacity records.
//!
//! Windows are half-open. Every required gate must be covered exactly once over
//! [0, horizon + 1). Gaps, overlaps, stale/revoked records and graph-gate
//! alias splitting are rejected. This module does not verify signatures,
//! issuer authority, source-byte truth, calibration or physical asset condition.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CAPACITY_ADMISSION_SCHEMA: &str = "safrb-evidence-bound-capacity-schedule-v0.1";
pub const CAPACITY_SCHEDULE_REPORT_SCHEMA: &str = "safrb-admitted-capacity-schedule-report-v0.1";
pub const CAPACITY_ADMISSION_CLAIM_CEILING: &str = "Structural validation of externally pre-admitted, time-bounded capacity records only. This module does not verify source-byte authenticity, signatures, issuer authority, calibration, physical asset condition or scenario-set completeness. Same-role cross-subject aliases are rejected as unsupported; cross-role shared-asset coupling and shared-gate compilation are not modeled.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityRole {
    Storage,
    Handling,
    ServiceIssue,
    Route,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityEvidenceClass {
    ConservativeOperationalBound,
    Demonstrated,
    Nameplate,
    Hypothetical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapacityAdmissionStatus {
    Admitted,
    Stale,
    Revoked,
    Unknown,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacitySubject {
    /// Model gate identity, e.g. node:yard-7 or transfer:route-12.
    pub subject_id: String,
    pub role: CapacityRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityEvidenceRecord {
    pub evidence_id: String,
    pub scenario_id: String,
    pub subject_id: String,
    pub role: CapacityRole,
    /// Canonical physical identity supplied by the upstream resource registry.
    pub canonical_resource_ref: String,
    pub effective_from_period: u64,
    /// Exclusive end period.
    pub effective_until_period_exclusive: u64,
    /// The record must remain fresh through its entire effective window.
    pub fresh_until_period_exclusive: u64,
    pub capacity: u64,
    pub unit_ref: String,
    pub quantity_boundary_ref: String,
    pub timebase_ref: String,
    pub evidence_class: CapacityEvidenceClass,
    /// Externally supplied admission result; not authenticated here.
    pub admission_status: CapacityAdmissionStatus,
    pub issuer_ref: String,
    pub authority_ref: String,
    pub source_bytes_sha256: String,
    pub admission_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityScheduleInput {
    pub schema_version: String,
    pub case_id: String,
    pub scenario_id: String,
    pub timebase_ref: String,
    pub unit_ref: String,
    pub quantity_boundary_ref: String,
    /// Last modeled period, inclusive.
    pub horizon_last_period: u64,
    pub required_subjects: Vec<CapacitySubject>,
    pub records: Vec<CapacityEvidenceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdmittedCapacityWindow {
    pub evidence_id: String,
    pub scenario_id: String,
    pub subject_id: String,
    pub role: CapacityRole,
    pub canonical_resource_ref: String,
    pub effective_from_period: u64,
    pub effective_until_period_exclusive: u64,
    pub fresh_until_period_exclusive: u64,
    pub capacity: u64,
    pub unit_ref: String,
    pub quantity_boundary_ref: String,
    pub timebase_ref: String,
    pub evidence_class: CapacityEvidenceClass,
    pub admission_status: CapacityAdmissionStatus,
    pub issuer_ref: String,
    pub authority_ref: String,
    pub source_bytes_sha256: String,
    pub admission_receipt_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapacityScheduleReport {
    pub schema_version: String,
    pub case_id: String,
    pub scenario_id: String,
    /// Binds all sorted subjects and complete sorted source records.
    pub schedule_sha256: String,
    pub windows: Vec<AdmittedCapacityWindow>,
    pub claim_ceiling: String,
}

fn nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
}

fn length_prefix(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let length = u64::try_from(value.len())
        .map_err(|_| "capacity schedule digest field exceeds u64 length".to_string())?;
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn canonical_input_bytes(input: &CapacityScheduleInput) -> Result<Vec<u8>, String> {
    let mut canonical = input.clone();
    canonical.required_subjects.sort();
    canonical.records.sort_by(|a, b| {
        (
            a.role,
            a.subject_id.as_str(),
            a.effective_from_period,
            a.effective_until_period_exclusive,
            a.evidence_id.as_str(),
        )
            .cmp(&(
                b.role,
                b.subject_id.as_str(),
                b.effective_from_period,
                b.effective_until_period_exclusive,
                b.evidence_id.as_str(),
            ))
    });
    serde_json::to_vec(&canonical)
        .map_err(|error| format!("could not encode canonical capacity schedule: {error}"))
}

fn schedule_digest(input: &CapacityScheduleInput) -> Result<String, String> {
    let mut bytes = b"LUMINOUS-SAFRB-CAPACITY-SCHEDULE-V1\0".to_vec();
    length_prefix(&mut bytes, &canonical_input_bytes(input)?)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn validate_input_header(input: &CapacityScheduleInput) -> Result<u64, String> {
    if input.schema_version != CAPACITY_ADMISSION_SCHEMA {
        return Err("unsupported capacity admission schema".to_string());
    }
    nonempty(&input.case_id, "case_id")?;
    nonempty(&input.scenario_id, "scenario_id")?;
    nonempty(&input.timebase_ref, "timebase_ref")?;
    nonempty(&input.unit_ref, "unit_ref")?;
    nonempty(&input.quantity_boundary_ref, "quantity_boundary_ref")?;
    if input.required_subjects.is_empty() {
        return Err("capacity schedule must declare required subjects".to_string());
    }
    if input.records.is_empty() {
        return Err("capacity schedule must contain evidence records".to_string());
    }
    input
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "capacity schedule horizon end overflowed u64".to_string())
}

fn validate_record(
    input: &CapacityScheduleInput,
    record: &CapacityEvidenceRecord,
    horizon_end: u64,
) -> Result<(), String> {
    for (value, label) in [
        (record.evidence_id.as_str(), "evidence_id"),
        (record.scenario_id.as_str(), "record scenario_id"),
        (record.subject_id.as_str(), "record subject_id"),
        (
            record.canonical_resource_ref.as_str(),
            "canonical_resource_ref",
        ),
        (record.issuer_ref.as_str(), "issuer_ref"),
        (record.authority_ref.as_str(), "authority_ref"),
    ] {
        nonempty(value, label)?;
    }
    if record.scenario_id != input.scenario_id {
        return Err(format!(
            "record {} belongs to another scenario",
            record.evidence_id
        ));
    }
    if record.unit_ref != input.unit_ref
        || record.quantity_boundary_ref != input.quantity_boundary_ref
        || record.timebase_ref != input.timebase_ref
    {
        return Err(format!(
            "record {} has unit, boundary or timebase mismatch",
            record.evidence_id
        ));
    }
    if record.effective_from_period >= record.effective_until_period_exclusive {
        return Err(format!(
            "record {} has empty or reversed half-open interval",
            record.evidence_id
        ));
    }
    if record.effective_until_period_exclusive > horizon_end {
        return Err(format!(
            "record {} extends beyond the declared horizon",
            record.evidence_id
        ));
    }
    if record.fresh_until_period_exclusive < record.effective_until_period_exclusive {
        return Err(format!(
            "record {} becomes stale before its effective window ends",
            record.evidence_id
        ));
    }
    if record.evidence_class != CapacityEvidenceClass::ConservativeOperationalBound {
        return Err(format!(
            "record {} is not a conservative operational capacity bound",
            record.evidence_id
        ));
    }
    if record.admission_status != CapacityAdmissionStatus::Admitted {
        return Err(format!(
            "record {} is not externally marked admitted ({:?})",
            record.evidence_id, record.admission_status
        ));
    }
    if !valid_sha256(&record.source_bytes_sha256) {
        return Err(format!(
            "record {} has invalid source-bytes digest",
            record.evidence_id
        ));
    }
    if !valid_sha256(&record.admission_receipt_sha256) {
        return Err(format!(
            "record {} has invalid admission-receipt digest",
            record.evidence_id
        ));
    }
    Ok(())
}

fn to_window(record: &CapacityEvidenceRecord) -> AdmittedCapacityWindow {
    AdmittedCapacityWindow {
        evidence_id: record.evidence_id.clone(),
        scenario_id: record.scenario_id.clone(),
        subject_id: record.subject_id.clone(),
        role: record.role,
        canonical_resource_ref: record.canonical_resource_ref.clone(),
        effective_from_period: record.effective_from_period,
        effective_until_period_exclusive: record.effective_until_period_exclusive,
        fresh_until_period_exclusive: record.fresh_until_period_exclusive,
        capacity: record.capacity,
        unit_ref: record.unit_ref.clone(),
        quantity_boundary_ref: record.quantity_boundary_ref.clone(),
        timebase_ref: record.timebase_ref.clone(),
        evidence_class: record.evidence_class,
        admission_status: record.admission_status,
        issuer_ref: record.issuer_ref.clone(),
        authority_ref: record.authority_ref.clone(),
        source_bytes_sha256: record.source_bytes_sha256.clone(),
        admission_receipt_sha256: record.admission_receipt_sha256.clone(),
    }
}

/// Validate externally pre-admitted capacity records and produce a canonical
/// schedule. No capacity is interpolated across gaps.
pub fn validate_capacity_schedule(
    input: &CapacityScheduleInput,
) -> Result<CapacityScheduleReport, String> {
    let horizon_end = validate_input_header(input)?;
    let mut subjects = BTreeSet::new();
    let mut evidence_ids = BTreeSet::new();
    let mut grouped = BTreeMap::<(CapacityRole, String), Vec<&CapacityEvidenceRecord>>::new();
    let mut resource_to_subject = BTreeMap::<(CapacityRole, String), String>::new();

    for subject in &input.required_subjects {
        nonempty(&subject.subject_id, "required subject_id")?;
        if !subjects.insert((subject.role, subject.subject_id.clone())) {
            return Err(format!("duplicate required subject {}", subject.subject_id));
        }
    }

    for record in &input.records {
        validate_record(input, record, horizon_end)?;
        if !evidence_ids.insert(record.evidence_id.as_str()) {
            return Err(format!("duplicate evidence ID {}", record.evidence_id));
        }
        let key = (record.role, record.subject_id.clone());
        if !subjects.contains(&key) {
            return Err(format!(
                "record {} targets undeclared subject",
                record.evidence_id
            ));
        }
        let resource_key = (record.role, record.canonical_resource_ref.clone());
        if let Some(existing) = resource_to_subject.get(&resource_key) {
            if existing != &record.subject_id {
                return Err(format!(
                    "unsupported shared-resource alias: resource {} maps to multiple {:?} subjects",
                    record.canonical_resource_ref, record.role
                ));
            }
        } else {
            resource_to_subject.insert(resource_key, record.subject_id.clone());
        }
        grouped.entry(key).or_default().push(record);
    }

    if grouped.len() != subjects.len() {
        return Err("one or more required capacity subjects have no evidence records".to_string());
    }

    let mut windows = Vec::with_capacity(input.records.len());
    for subject in &input.required_subjects {
        let key = (subject.role, subject.subject_id.clone());
        let mut records = grouped
            .remove(&key)
            .ok_or_else(|| format!("missing records for subject {}", subject.subject_id))?;
        records.sort_by(|a, b| {
            (
                a.effective_from_period,
                a.effective_until_period_exclusive,
                a.evidence_id.as_str(),
            )
                .cmp(&(
                    b.effective_from_period,
                    b.effective_until_period_exclusive,
                    b.evidence_id.as_str(),
                ))
        });

        let mut cursor = 0_u64;
        let mut canonical_resource: Option<&str> = None;
        for record in records {
            if let Some(existing) = canonical_resource {
                if existing != record.canonical_resource_ref.as_str() {
                    return Err(format!(
                        "subject {} changes canonical physical identity across periods",
                        subject.subject_id
                    ));
                }
            } else {
                canonical_resource = Some(record.canonical_resource_ref.as_str());
            }

            if record.effective_from_period < cursor {
                return Err(format!(
                    "overlapping capacity windows for subject {} at period {}",
                    subject.subject_id, record.effective_from_period
                ));
            }
            if record.effective_from_period > cursor {
                return Err(format!(
                    "capacity evidence gap for subject {}: expected {}, found {}",
                    subject.subject_id, cursor, record.effective_from_period
                ));
            }
            cursor = record.effective_until_period_exclusive;
            windows.push(to_window(record));
        }
        if cursor != horizon_end {
            return Err(format!(
                "capacity coverage for subject {} ends at {}, expected {}",
                subject.subject_id, cursor, horizon_end
            ));
        }
    }

    windows.sort_by(|a, b| {
        (
            a.role,
            a.subject_id.as_str(),
            a.effective_from_period,
            a.evidence_id.as_str(),
        )
            .cmp(&(
                b.role,
                b.subject_id.as_str(),
                b.effective_from_period,
                b.evidence_id.as_str(),
            ))
    });

    Ok(CapacityScheduleReport {
        schema_version: CAPACITY_SCHEDULE_REPORT_SCHEMA.to_string(),
        case_id: input.case_id.clone(),
        scenario_id: input.scenario_id.clone(),
        schedule_sha256: schedule_digest(input)?,
        windows,
        claim_ceiling: CAPACITY_ADMISSION_CLAIM_CEILING.to_string(),
    })
}

fn verifier_schedule_digest(input: &CapacityScheduleInput) -> Result<String, String> {
    let mut canonical = input.clone();
    canonical.required_subjects.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
    });
    canonical.records.sort_by(|left, right| {
        left.role
            .cmp(&right.role)
            .then_with(|| left.subject_id.cmp(&right.subject_id))
            .then_with(|| left.effective_from_period.cmp(&right.effective_from_period))
            .then_with(|| {
                left.effective_until_period_exclusive
                    .cmp(&right.effective_until_period_exclusive)
            })
            .then_with(|| left.evidence_id.cmp(&right.evidence_id))
    });
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| format!("capacity verifier could not canonicalize input: {error}"))?;
    let mut bytes = b"LUMINOUS-SAFRB-CAPACITY-SCHEDULE-V1\\0".to_vec();
    length_prefix(&mut bytes, &encoded)?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn verifier_to_window(record: &CapacityEvidenceRecord) -> AdmittedCapacityWindow {
    AdmittedCapacityWindow {
        evidence_id: record.evidence_id.clone(),
        scenario_id: record.scenario_id.clone(),
        subject_id: record.subject_id.clone(),
        role: record.role,
        canonical_resource_ref: record.canonical_resource_ref.clone(),
        effective_from_period: record.effective_from_period,
        effective_until_period_exclusive: record.effective_until_period_exclusive,
        fresh_until_period_exclusive: record.fresh_until_period_exclusive,
        capacity: record.capacity,
        unit_ref: record.unit_ref.clone(),
        quantity_boundary_ref: record.quantity_boundary_ref.clone(),
        timebase_ref: record.timebase_ref.clone(),
        evidence_class: record.evidence_class,
        admission_status: record.admission_status,
        issuer_ref: record.issuer_ref.clone(),
        authority_ref: record.authority_ref.clone(),
        source_bytes_sha256: record.source_bytes_sha256.clone(),
        admission_receipt_sha256: record.admission_receipt_sha256.clone(),
    }
}

/// Independently verify report coverage and window fields without calling the
/// producer validator.
pub fn verify_capacity_schedule(
    input: &CapacityScheduleInput,
    report: &CapacityScheduleReport,
) -> Result<(), String> {
    let horizon_end = input
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "capacity verifier horizon end overflowed".to_string())?;
    if input.schema_version != CAPACITY_ADMISSION_SCHEMA
        || report.schema_version != CAPACITY_SCHEDULE_REPORT_SCHEMA
    {
        return Err("capacity verifier encountered unsupported schema version".to_string());
    }
    for (value, label) in [
        (input.case_id.as_str(), "case_id"),
        (input.scenario_id.as_str(), "scenario_id"),
        (input.timebase_ref.as_str(), "timebase_ref"),
        (input.unit_ref.as_str(), "unit_ref"),
        (
            input.quantity_boundary_ref.as_str(),
            "quantity_boundary_ref",
        ),
    ] {
        nonempty(value, label)?;
    }
    if report.case_id != input.case_id || report.scenario_id != input.scenario_id {
        return Err("capacity report case/scenario identity mismatch".to_string());
    }
    if report.claim_ceiling != CAPACITY_ADMISSION_CLAIM_CEILING {
        return Err("capacity report claim ceiling mismatch".to_string());
    }
    if report.schedule_sha256 != verifier_schedule_digest(input)? {
        return Err("capacity schedule input digest mismatch".to_string());
    }
    if input.required_subjects.is_empty() || input.records.is_empty() {
        return Err("capacity verifier found empty subject or record set".to_string());
    }

    let mut subjects = BTreeSet::new();
    for subject in &input.required_subjects {
        nonempty(&subject.subject_id, "required subject_id")?;
        if !subjects.insert((subject.role, subject.subject_id.clone())) {
            return Err("capacity verifier found duplicate required subject".to_string());
        }
    }

    let mut evidence_ids = BTreeSet::new();
    let mut records_by_subject =
        BTreeMap::<(CapacityRole, String), Vec<&CapacityEvidenceRecord>>::new();
    let mut resource_to_subject = BTreeMap::<(CapacityRole, &str), &str>::new();
    for record in &input.records {
        if record.evidence_id.trim().is_empty()
            || !evidence_ids.insert(record.evidence_id.as_str())
            || record.scenario_id != input.scenario_id
            || record.subject_id.trim().is_empty()
            || record.canonical_resource_ref.trim().is_empty()
            || record.issuer_ref.trim().is_empty()
            || record.authority_ref.trim().is_empty()
            || record.unit_ref != input.unit_ref
            || record.quantity_boundary_ref != input.quantity_boundary_ref
            || record.timebase_ref != input.timebase_ref
            || record.effective_from_period >= record.effective_until_period_exclusive
            || record.effective_until_period_exclusive > horizon_end
            || record.fresh_until_period_exclusive < record.effective_until_period_exclusive
            || record.evidence_class != CapacityEvidenceClass::ConservativeOperationalBound
            || record.admission_status != CapacityAdmissionStatus::Admitted
            || !valid_sha256(&record.source_bytes_sha256)
            || !valid_sha256(&record.admission_receipt_sha256)
        {
            return Err(format!(
                "capacity verifier rejected record {}",
                record.evidence_id
            ));
        }
        if !subjects.contains(&(record.role, record.subject_id.clone())) {
            return Err("capacity verifier found undeclared subject".to_string());
        }
        let physical_key = (record.role, record.canonical_resource_ref.as_str());
        if let Some(existing) = resource_to_subject.get(&physical_key) {
            if *existing != record.subject_id.as_str() {
                return Err("capacity verifier found unsupported shared-resource alias".to_string());
            }
        } else {
            resource_to_subject.insert(physical_key, record.subject_id.as_str());
        }
        records_by_subject
            .entry((record.role, record.subject_id.clone()))
            .or_default()
            .push(record);
    }
    if records_by_subject.len() != subjects.len() {
        return Err("capacity verifier found incomplete required-subject set".to_string());
    }

    let mut expected_windows = Vec::with_capacity(input.records.len());
    for (role, subject_id) in &subjects {
        let mut records = records_by_subject
            .remove(&(*role, subject_id.clone()))
            .ok_or_else(|| format!("capacity verifier missing subject {subject_id}"))?;
        records.sort_by(|a, b| {
            (
                a.effective_from_period,
                a.effective_until_period_exclusive,
                a.evidence_id.as_str(),
            )
                .cmp(&(
                    b.effective_from_period,
                    b.effective_until_period_exclusive,
                    b.evidence_id.as_str(),
                ))
        });

        let mut cursor = 0_u64;
        let mut physical_ref: Option<&str> = None;
        for record in records {
            if let Some(previous) = physical_ref {
                if previous != record.canonical_resource_ref.as_str() {
                    return Err("capacity verifier found changing physical identity".to_string());
                }
            } else {
                physical_ref = Some(record.canonical_resource_ref.as_str());
            }
            if record.effective_from_period != cursor {
                return Err(format!(
                    "capacity verifier found overlap or gap for subject {subject_id} at period {cursor}"
                ));
            }
            cursor = record.effective_until_period_exclusive;
            expected_windows.push(AdmittedCapacityWindow {
                evidence_id: record.evidence_id.clone(),
                scenario_id: record.scenario_id.clone(),
                subject_id: record.subject_id.clone(),
                role: record.role,
                canonical_resource_ref: record.canonical_resource_ref.clone(),
                effective_from_period: record.effective_from_period,
                effective_until_period_exclusive: record.effective_until_period_exclusive,
                fresh_until_period_exclusive: record.fresh_until_period_exclusive,
                capacity: record.capacity,
                unit_ref: record.unit_ref.clone(),
                quantity_boundary_ref: record.quantity_boundary_ref.clone(),
                timebase_ref: record.timebase_ref.clone(),
                evidence_class: record.evidence_class,
                admission_status: record.admission_status,
                issuer_ref: record.issuer_ref.clone(),
                authority_ref: record.authority_ref.clone(),
                source_bytes_sha256: record.source_bytes_sha256.clone(),
                admission_receipt_sha256: record.admission_receipt_sha256.clone(),
            });
        }
        if cursor != horizon_end {
            return Err(format!(
                "capacity verifier found incomplete coverage for {subject_id}"
            ));
        }
    }

    expected_windows.sort_by(|a, b| {
        (
            a.role,
            a.subject_id.as_str(),
            a.effective_from_period,
            a.evidence_id.as_str(),
        )
            .cmp(&(
                b.role,
                b.subject_id.as_str(),
                b.effective_from_period,
                b.evidence_id.as_str(),
            ))
    });
    if report.windows != expected_windows {
        return Err("capacity schedule windows do not match source records".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(tag: u64) -> String {
        format!("sha256:{tag:064x}")
    }

    fn record(
        evidence_id: &str,
        from: u64,
        until: u64,
        capacity: u64,
        source: u64,
        receipt: u64,
    ) -> CapacityEvidenceRecord {
        CapacityEvidenceRecord {
            evidence_id: evidence_id.to_string(),
            scenario_id: "scenario-a".to_string(),
            subject_id: "node:yard-a".to_string(),
            role: CapacityRole::Handling,
            canonical_resource_ref: "physical:pump-a".to_string(),
            effective_from_period: from,
            effective_until_period_exclusive: until,
            fresh_until_period_exclusive: until,
            capacity,
            unit_ref: "litre".to_string(),
            quantity_boundary_ref: "deliverable-at-gate".to_string(),
            timebase_ref: "synthetic:daily-periods".to_string(),
            evidence_class: CapacityEvidenceClass::ConservativeOperationalBound,
            admission_status: CapacityAdmissionStatus::Admitted,
            issuer_ref: "issuer:synthetic".to_string(),
            authority_ref: "authority:synthetic".to_string(),
            source_bytes_sha256: sha(source),
            admission_receipt_sha256: sha(receipt),
        }
    }

    fn input(records: Vec<CapacityEvidenceRecord>) -> CapacityScheduleInput {
        CapacityScheduleInput {
            schema_version: CAPACITY_ADMISSION_SCHEMA.to_string(),
            case_id: "synthetic-capacity-window-case".to_string(),
            scenario_id: "scenario-a".to_string(),
            timebase_ref: "synthetic:daily-periods".to_string(),
            unit_ref: "litre".to_string(),
            quantity_boundary_ref: "deliverable-at-gate".to_string(),
            horizon_last_period: 2,
            required_subjects: vec![CapacitySubject {
                subject_id: "node:yard-a".to_string(),
                role: CapacityRole::Handling,
            }],
            records,
        }
    }

    #[test]
    fn adjacent_half_open_windows_cover_the_horizon_exactly() {
        let model = input(vec![
            record("window-0", 0, 1, 100, 1, 11),
            record("window-1", 1, 3, 80, 2, 12),
        ]);
        let report = validate_capacity_schedule(&model).unwrap();
        assert_eq!(report.windows.len(), 2);
        assert_eq!(report.windows[0].capacity, 100);
        assert_eq!(
            report.windows[0].evidence_class,
            CapacityEvidenceClass::ConservativeOperationalBound
        );
        assert_eq!(
            report.windows[0].admission_status,
            CapacityAdmissionStatus::Admitted
        );
        assert_eq!(report.windows[1].effective_from_period, 1);
        verify_capacity_schedule(&model, &report).unwrap();
    }

    #[test]
    fn overlapping_windows_are_rejected_even_when_capacity_values_match() {
        let model = input(vec![
            record("window-0", 0, 2, 100, 1, 11),
            record("window-1", 1, 3, 100, 2, 12),
        ]);
        assert!(
            validate_capacity_schedule(&model)
                .unwrap_err()
                .contains("overlapping")
        );
    }

    #[test]
    fn gaps_are_rejected_without_interpolation() {
        let model = input(vec![
            record("window-0", 0, 1, 100, 1, 11),
            record("window-1", 2, 3, 80, 2, 12),
        ]);
        assert!(
            validate_capacity_schedule(&model)
                .unwrap_err()
                .contains("gap")
        );
    }

    #[test]
    fn record_that_expires_before_its_window_ends_is_rejected() {
        let mut expired = record("expired", 0, 3, 100, 1, 11);
        expired.fresh_until_period_exclusive = 2;
        assert!(
            validate_capacity_schedule(&input(vec![expired]))
                .unwrap_err()
                .contains("stale")
        );
    }

    #[test]
    fn stale_revoked_unknown_and_conflicted_records_are_rejected() {
        for status in [
            CapacityAdmissionStatus::Stale,
            CapacityAdmissionStatus::Revoked,
            CapacityAdmissionStatus::Unknown,
            CapacityAdmissionStatus::Conflicted,
        ] {
            let mut candidate = record("not-admitted", 0, 3, 100, 1, 11);
            candidate.admission_status = status;
            assert!(
                validate_capacity_schedule(&input(vec![candidate]))
                    .unwrap_err()
                    .contains("not externally marked admitted")
            );
        }
    }

    #[test]
    fn nameplate_demonstrated_and_hypothetical_classes_are_not_conservative_bounds() {
        for class in [
            CapacityEvidenceClass::Demonstrated,
            CapacityEvidenceClass::Nameplate,
            CapacityEvidenceClass::Hypothetical,
        ] {
            let mut candidate = record("wrong-class", 0, 3, 100, 1, 11);
            candidate.evidence_class = class;
            assert!(
                validate_capacity_schedule(&input(vec![candidate]))
                    .unwrap_err()
                    .contains("not a conservative operational capacity bound")
            );
        }
    }

    #[test]
    fn unit_boundary_timebase_and_scenario_mismatches_are_rejected() {
        let mut wrong_unit = record("wrong-unit", 0, 3, 100, 1, 11);
        wrong_unit.unit_ref = "gallon".to_string();
        assert!(
            validate_capacity_schedule(&input(vec![wrong_unit]))
                .unwrap_err()
                .contains("unit, boundary or timebase mismatch")
        );

        let mut wrong_boundary = record("wrong-boundary", 0, 3, 100, 1, 11);
        wrong_boundary.quantity_boundary_ref = "different-boundary".to_string();
        assert!(
            validate_capacity_schedule(&input(vec![wrong_boundary]))
                .unwrap_err()
                .contains("unit, boundary or timebase mismatch")
        );

        let mut wrong_scenario = record("wrong-scenario", 0, 3, 100, 1, 11);
        wrong_scenario.scenario_id = "scenario-b".to_string();
        assert!(
            validate_capacity_schedule(&input(vec![wrong_scenario]))
                .unwrap_err()
                .contains("different scenario")
        );
    }

    #[test]
    fn one_canonical_physical_resource_cannot_be_split_across_gate_subjects() {
        let mut model = input(vec![record("window-a", 0, 3, 100, 1, 11)]);
        model.required_subjects.push(CapacitySubject {
            subject_id: "node:yard-b".to_string(),
            role: CapacityRole::Handling,
        });
        let mut alias = record("window-b", 0, 3, 100, 2, 12);
        alias.subject_id = "node:yard-b".to_string();
        let spliced = CapacityScheduleInput {
            records: vec![model.records[0].clone(), alias],
            ..model
        };
        assert!(
            validate_capacity_schedule(&spliced)
                .unwrap_err()
                .contains("unsupported shared-resource alias")
        );
    }

    #[test]
    fn subject_cannot_change_physical_identity_across_windows() {
        let mut second = record("window-1", 1, 3, 80, 2, 12);
        second.canonical_resource_ref = "physical:pump-b".to_string();
        let model = input(vec![record("window-0", 0, 1, 100, 1, 11), second]);
        assert!(
            validate_capacity_schedule(&model)
                .unwrap_err()
                .contains("changes canonical physical identity")
        );
    }

    #[test]
    fn verifier_rejects_record_mutation_with_old_receipt_digest() {
        let mut model = input(vec![record("window", 0, 3, 100, 1, 11)]);
        let report = validate_capacity_schedule(&model).unwrap();
        model.records[0].capacity = 99;
        assert!(
            verify_capacity_schedule(&model, &report)
                .unwrap_err()
                .contains("input digest mismatch")
        );
    }

    #[test]
    fn verifier_rejects_tampered_window_capacity() {
        let model = input(vec![record("window", 0, 3, 100, 1, 11)]);
        let mut report = validate_capacity_schedule(&model).unwrap();
        report.windows[0].capacity = 99;
        assert!(
            verify_capacity_schedule(&model, &report)
                .unwrap_err()
                .contains("windows do not match")
        );
    }

    #[test]
    fn input_order_does_not_change_digest_but_capacity_mutation_does() {
        let a = record("window-a", 0, 1, 100, 1, 11);
        let b = record("window-b", 1, 3, 80, 2, 12);
        let forward = input(vec![a.clone(), b.clone()]);
        let reverse = input(vec![b, a]);
        assert_eq!(
            schedule_digest(&forward).unwrap(),
            schedule_digest(&reverse).unwrap()
        );

        let mut changed = forward.clone();
        changed.records[0].capacity += 1;
        assert_ne!(
            schedule_digest(&forward).unwrap(),
            schedule_digest(&changed).unwrap()
        );
    }

    fn next_small(state: &mut u64, bound: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) % bound
    }

    #[test]
    fn seeded_contiguous_window_schedules_match_exact_coverage_oracle() {
        for seed in 0..200_u64 {
            let mut state = 0x05AF_B202_6D15_CA11_u64 ^ seed;
            let horizon_last = 1 + next_small(&mut state, 6);
            let horizon_end = horizon_last + 1;
            let mut cursor = 0_u64;
            let mut records = Vec::new();
            let mut index = 0_u64;
            while cursor < horizon_end {
                let remaining = horizon_end - cursor;
                let length = 1 + next_small(&mut state, remaining);
                records.push(record(
                    &format!("seed-{seed}-window-{index}"),
                    cursor,
                    cursor + length,
                    next_small(&mut state, 500),
                    seed.wrapping_add(index).wrapping_add(1),
                    seed.wrapping_add(index).wrapping_add(101),
                ));
                cursor += length;
                index += 1;
            }
            let mut model = input(records);
            model.horizon_last_period = horizon_last;
            let report = validate_capacity_schedule(&model).unwrap();
            assert_eq!(report.windows.first().unwrap().effective_from_period, 0);
            assert_eq!(
                report
                    .windows
                    .last()
                    .unwrap()
                    .effective_until_period_exclusive,
                horizon_end
            );
            assert_eq!(report.windows.len(), model.records.len());
            verify_capacity_schedule(&model, &report).unwrap();
        }
    }

    #[test]
    fn strict_json_round_trip_unknown_fields_and_schema_documents_are_checked() {
        let model = input(vec![
            record("window-0", 0, 1, 100, 1, 11),
            record("window-1", 1, 3, 80, 2, 12),
        ]);
        let bytes = serde_json::to_vec(&model).unwrap();
        let decoded: CapacityScheduleInput = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, model);
        let report = validate_capacity_schedule(&decoded).unwrap();
        let report_bytes = serde_json::to_vec(&report).unwrap();
        let decoded_report: CapacityScheduleReport = serde_json::from_slice(&report_bytes).unwrap();
        assert_eq!(decoded_report, report);
        verify_capacity_schedule(&decoded, &decoded_report).unwrap();

        let mut unknown = serde_json::to_value(&model).unwrap();
        unknown["unexpected"] = serde_json::json!(true);
        assert!(serde_json::from_value::<CapacityScheduleInput>(unknown).is_err());

        let input_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/capacity-schedule-input-v0.1.schema.json"
        ))
        .unwrap();
        let report_schema: serde_json::Value = serde_json::from_str(include_str!(
            "../schemas/capacity-schedule-report-v0.1.schema.json"
        ))
        .unwrap();
        assert_eq!(
            input_schema["properties"]["schema_version"]["const"],
            CAPACITY_ADMISSION_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["schema_version"]["const"],
            CAPACITY_SCHEDULE_REPORT_SCHEMA
        );
        assert_eq!(
            report_schema["properties"]["claim_ceiling"]["const"],
            CAPACITY_ADMISSION_CLAIM_CEILING
        );
        assert_eq!(input_schema["additionalProperties"], false);
        assert_eq!(report_schema["additionalProperties"], false);
    }
}
