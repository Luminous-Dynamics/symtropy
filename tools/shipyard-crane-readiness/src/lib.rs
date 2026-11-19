//! Synthetic-first evidence verifier for bridge/gantry crane operational readiness.
//!
//! This crate consumes a lift-preflight receipt and independently checks
//! inspection, protective-device, crane-duty, personnel, briefing and exclusion
//! zone evidence. It is not a permit system, crane controller or safety authority.

use std::collections::{BTreeMap, BTreeSet};

pub const CONTRACT_VERSION: &str = "shipyard-crane-readiness-v0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceClass {
    CraneInspection,
    SupportingStructureInspection,
    DutyMonitoring,
    LimiterVerification,
    PersonnelCredential,
    SafeWorkBriefing,
    BriefingAcknowledgement,
    ExclusionZonePlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceStatus {
    Current,
    Restricted,
    Revoked,
    Faulted,
    Overdue,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PersonnelRole {
    Operator,
    Slinger,
    Signaller,
    Supervisor,
    AppointedPerson,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceState {
    Operational,
    Faulted,
    Bypassed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExclusionZoneState {
    Enforced,
    NotEnforced,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiftPreflightDisposition {
    ReadyForHumanReview,
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_class: EvidenceClass,
    pub record_id: String,
    pub owner_id: String,
    pub document_revision: String,
    pub subject_id: String,
    pub subject_revision: String,
    pub sha256: String,
    pub clock_domain_id: String,
    pub valid_from_tick: u64,
    pub valid_until_tick: u64,
    pub status: EvidenceStatus,
    /// Set only for personnel credentials; the person reference remains pseudonymous.
    pub personnel_role: Option<PersonnelRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredEvidence {
    pub evidence_class: EvidenceClass,
    pub subject_id: String,
    pub subject_revision: String,
    pub personnel_role: Option<PersonnelRole>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequiredDevice {
    pub device_id: String,
    pub device_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEvidence {
    pub device_id: String,
    pub device_revision: String,
    pub state: DeviceState,
    pub evidence_record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleAssignment {
    pub role: PersonnelRole,
    /// Pseudonymous subject reference; no name/email/identity data is required.
    pub person_reference: String,
    pub credential_record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefingAcknowledgement {
    pub role: PersonnelRole,
    pub person_reference: String,
    pub method_statement_id: String,
    pub method_statement_revision: String,
    pub acknowledged_at_tick: u64,
    pub evidence_record_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiftPreflightReceiptRef {
    pub receipt_id: String,
    pub input_sha256: String,
    pub report_sha256: String,
    pub profile_version: String,
    pub evaluator_revision: String,
    pub disposition: LiftPreflightDisposition,
    pub invalidated: bool,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub site_epoch_id: String,
    pub clock_domain_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub issued_at_tick: u64,
    pub valid_until_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessPlan {
    pub contract_version: String,
    pub readiness_id: String,
    pub crane_id: String,
    pub crane_configuration_revision: String,
    pub supporting_structure_id: String,
    pub supporting_structure_revision: String,
    pub site_epoch_id: String,
    pub clock_domain_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub method_statement_id: String,
    pub method_statement_revision: String,
    pub exclusion_zone_plan_revision: String,
    pub lift_preflight: LiftPreflightReceiptRef,
    pub required_evidence: Vec<RequiredEvidence>,
    pub evidence_records: Vec<EvidenceRecord>,
    pub required_devices: Vec<RequiredDevice>,
    pub device_evidence: Vec<DeviceEvidence>,
    pub required_roles: Vec<PersonnelRole>,
    pub role_assignments: Vec<RoleAssignment>,
    pub briefing_acknowledgements: Vec<BriefingAcknowledgement>,
    pub exclusion_zone_state: ExclusionZoneState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// Supplied readiness evidence is internally consistent; human/site review remains mandatory.
    ReadyForHumanReview,
    /// A source-owned disqualifying operating state is explicit.
    Rejected,
    /// A required record is missing, stale, ambiguous, or outside the declared profile.
    Indeterminate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticSeverity {
    Rejected,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: DiagnosticSeverity,
    pub subject_id: Option<String>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReport {
    pub contract_version: String,
    pub disposition: Disposition,
    pub diagnostics: Vec<Diagnostic>,
    pub verified_evidence_count: usize,
    pub required_evidence_count: usize,
    pub verified_device_count: usize,
    pub required_device_count: usize,
    pub verified_role_count: usize,
    pub required_role_count: usize,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn push(
    diagnostics: &mut Vec<Diagnostic>,
    code: &'static str,
    severity: DiagnosticSeverity,
    subject: Option<&str>,
    detail: impl Into<String>,
) {
    diagnostics.push(Diagnostic {
        code,
        severity,
        subject_id: subject.map(str::to_owned),
        detail: detail.into(),
    });
}

fn disposition_for(diagnostics: &[Diagnostic]) -> Disposition {
    if diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Indeterminate) {
        Disposition::Indeterminate
    } else if diagnostics.iter().any(|d| d.severity == DiagnosticSeverity::Rejected) {
        Disposition::Rejected
    } else {
        Disposition::ReadyForHumanReview
    }
}

fn validate_context(plan: &ReadinessPlan, diagnostics: &mut Vec<Diagnostic>) {
    if plan.contract_version != CONTRACT_VERSION {
        push(
            diagnostics,
            "READINESS_CONTRACT_VERSION_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.readiness_id),
            "input contract version does not match this evaluator",
        );
    }
    for (field, value) in [
        ("readiness_id", plan.readiness_id.as_str()),
        ("crane_id", plan.crane_id.as_str()),
        ("crane_configuration_revision", plan.crane_configuration_revision.as_str()),
        ("supporting_structure_id", plan.supporting_structure_id.as_str()),
        ("supporting_structure_revision", plan.supporting_structure_revision.as_str()),
        ("site_epoch_id", plan.site_epoch_id.as_str()),
        ("clock_domain_id", plan.clock_domain_id.as_str()),
        ("method_statement_id", plan.method_statement_id.as_str()),
        ("method_statement_revision", plan.method_statement_revision.as_str()),
        ("exclusion_zone_plan_revision", plan.exclusion_zone_plan_revision.as_str()),
    ] {
        if !nonblank(value) {
            push(
                diagnostics,
                "READINESS_CONTEXT_FIELD_EMPTY",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.readiness_id),
                format!("{field} must be nonblank"),
            );
        }
    }
    if plan.end_tick <= plan.start_tick {
        push(
            diagnostics,
            "READINESS_WINDOW_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.readiness_id),
            "readiness operation window must be a non-empty half-open interval",
        );
    }
    if plan.required_evidence.is_empty() {
        push(
            diagnostics,
            "REQUIRED_EVIDENCE_SET_EMPTY",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.readiness_id),
            "readiness profile must declare required source evidence",
        );
    }
    if plan.required_devices.is_empty() {
        push(
            diagnostics,
            "REQUIRED_DEVICE_SET_EMPTY",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.crane_id),
            "readiness profile must declare required protective/indicating devices",
        );
    }
    if plan.required_roles.is_empty() {
        push(
            diagnostics,
            "REQUIRED_ROLE_SET_EMPTY",
            DiagnosticSeverity::Indeterminate,
            Some(&plan.readiness_id),
            "readiness profile must declare the roles required for the operation",
        );
    }
}

fn validate_lift_receipt(plan: &ReadinessPlan, diagnostics: &mut Vec<Diagnostic>) {
    let receipt = &plan.lift_preflight;
    if !nonblank(&receipt.receipt_id)
        || !is_sha256(&receipt.input_sha256)
        || !is_sha256(&receipt.report_sha256)
        || !nonblank(&receipt.profile_version)
        || !nonblank(&receipt.evaluator_revision)
    {
        push(
            diagnostics,
            "LIFT_PREFLIGHT_RECEIPT_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&receipt.receipt_id),
            "receipt identity, input/report digests, profile and evaluator revision must be complete",
        );
    }
    if receipt.disposition != LiftPreflightDisposition::ReadyForHumanReview {
        let (severity, detail) = match receipt.disposition {
            LiftPreflightDisposition::ReadyForHumanReview => unreachable!(),
            LiftPreflightDisposition::Rejected => (
                DiagnosticSeverity::Rejected,
                "underlying lift preflight is rejected",
            ),
            LiftPreflightDisposition::Indeterminate => (
                DiagnosticSeverity::Indeterminate,
                "underlying lift preflight is indeterminate",
            ),
        };
        push(
            diagnostics,
            "LIFT_PREFLIGHT_NOT_READY",
            severity,
            Some(&receipt.receipt_id),
            detail,
        );
    }
    if receipt.invalidated {
        push(
            diagnostics,
            "LIFT_PREFLIGHT_RECEIPT_INVALIDATED",
            DiagnosticSeverity::Indeterminate,
            Some(&receipt.receipt_id),
            "underlying lift-preflight dependency changed; a new receipt is required",
        );
    }
    if receipt.crane_id != plan.crane_id
        || receipt.crane_configuration_revision != plan.crane_configuration_revision
        || receipt.site_epoch_id != plan.site_epoch_id
        || receipt.clock_domain_id != plan.clock_domain_id
    {
        push(
            diagnostics,
            "LIFT_PREFLIGHT_CONTEXT_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&receipt.receipt_id),
            "lift receipt must bind this crane/configuration/site epoch/clock domain",
        );
    }
    if receipt.start_tick != plan.start_tick
        || receipt.end_tick != plan.end_tick
        || receipt.issued_at_tick > plan.start_tick
        || receipt.valid_until_tick < plan.end_tick
    {
        push(
            diagnostics,
            "LIFT_PREFLIGHT_WINDOW_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&receipt.receipt_id),
            "lift receipt must be issued before the operation and cover the exact operation window",
        );
    }
}

fn index_evidence<'a>(
    plan: &'a ReadinessPlan,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<&'a str, &'a EvidenceRecord> {
    let mut records = BTreeMap::new();
    for record in &plan.evidence_records {
        if !nonblank(&record.record_id) {
            push(
                diagnostics,
                "EVIDENCE_RECORD_ID_EMPTY",
                DiagnosticSeverity::Indeterminate,
                None,
                "evidence record ID must be nonblank",
            );
            continue;
        }
        if let Some(existing) = records.get(record.record_id.as_str()) {
            if *existing != record {
                push(
                    diagnostics,
                    "EVIDENCE_RECORD_ID_CONFLICT",
                    DiagnosticSeverity::Indeterminate,
                    Some(&record.record_id),
                    "one evidence record ID was reused with conflicting metadata",
                );
            }
        } else {
            records.insert(record.record_id.as_str(), record);
        }
    }
    records
}

fn validate_record(
    plan: &ReadinessPlan,
    record: &EvidenceRecord,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut valid = true;
    if !nonblank(&record.owner_id)
        || !nonblank(&record.document_revision)
        || !nonblank(&record.subject_id)
        || !nonblank(&record.subject_revision)
        || !is_sha256(&record.sha256)
    {
        valid = false;
        push(
            diagnostics,
            "EVIDENCE_RECORD_METADATA_INVALID",
            DiagnosticSeverity::Indeterminate,
            Some(&record.record_id),
            "owner, document revision, subject/revision and SHA-256 lexical shape are required",
        );
    }
    if record.clock_domain_id != plan.clock_domain_id {
        valid = false;
        push(
            diagnostics,
            "EVIDENCE_CLOCK_DOMAIN_MISMATCH",
            DiagnosticSeverity::Indeterminate,
            Some(&record.record_id),
            "evidence validity ticks must use the readiness plan clock domain",
        );
    }
    if record.valid_from_tick >= record.valid_until_tick
        || record.valid_from_tick > plan.start_tick
        || record.valid_until_tick < plan.end_tick
    {
        valid = false;
        push(
            diagnostics,
            "EVIDENCE_NOT_VALID_FOR_OPERATION",
            DiagnosticSeverity::Indeterminate,
            Some(&record.record_id),
            "evidence validity must cover the complete half-open operation window",
        );
    }
    match record.status {
        EvidenceStatus::Current => {}
        EvidenceStatus::Restricted => {
            valid = false;
            push(
                diagnostics,
                "EVIDENCE_RESTRICTED",
                DiagnosticSeverity::Rejected,
                Some(&record.record_id),
                "source record carries an operating restriction",
            );
        }
        EvidenceStatus::Revoked => {
            valid = false;
            push(
                diagnostics,
                "EVIDENCE_REVOKED",
                DiagnosticSeverity::Rejected,
                Some(&record.record_id),
                "source record is revoked",
            );
        }
        EvidenceStatus::Faulted => {
            valid = false;
            push(
                diagnostics,
                "EVIDENCE_FAULTED",
                DiagnosticSeverity::Rejected,
                Some(&record.record_id),
                "source record reports a fault",
            );
        }
        EvidenceStatus::Overdue => {
            valid = false;
            push(
                diagnostics,
                "EVIDENCE_OVERDUE",
                DiagnosticSeverity::Rejected,
                Some(&record.record_id),
                "source-owned inspection/qualification state is overdue",
            );
        }
        EvidenceStatus::Unknown => {
            valid = false;
            push(
                diagnostics,
                "EVIDENCE_STATUS_UNKNOWN",
                DiagnosticSeverity::Indeterminate,
                Some(&record.record_id),
                "source status is unknown",
            );
        }
    }
    valid
}

fn validate_required_evidence(
    plan: &ReadinessPlan,
    records: &BTreeMap<&str, &EvidenceRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> usize {
    let mut verified = 0usize;
    let mut seen = BTreeSet::new();
    for required in &plan.required_evidence {
        if !seen.insert((
            required.evidence_class,
            required.subject_id.as_str(),
            required.subject_revision.as_str(),
            required.personnel_role,
        )) {
            push(
                diagnostics,
                "REQUIRED_EVIDENCE_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(&required.subject_id),
                "required evidence subject/class was declared more than once",
            );
            continue;
        }
        let matches: Vec<&EvidenceRecord> = records
            .values()
            .copied()
            .filter(|record| {
                record.evidence_class == required.evidence_class
                    && record.subject_id == required.subject_id
                    && record.subject_revision == required.subject_revision
                    && record.personnel_role == required.personnel_role
            })
            .collect();
        match matches.as_slice() {
            [record] => {
                if validate_record(plan, record, diagnostics) {
                    verified += 1;
                }
            }
            [] => push(
                diagnostics,
                "REQUIRED_EVIDENCE_MISSING",
                DiagnosticSeverity::Indeterminate,
                Some(&required.subject_id),
                format!(
                    "missing {:?} evidence for exact subject/revision",
                    required.evidence_class
                ),
            ),
            _ => push(
                diagnostics,
                "REQUIRED_EVIDENCE_AMBIGUOUS",
                DiagnosticSeverity::Indeterminate,
                Some(&required.subject_id),
                format!(
                    "multiple {:?} evidence records match the same required subject/revision",
                    required.evidence_class
                ),
            ),
        }
    }
    verified
}

fn validate_devices(
    plan: &ReadinessPlan,
    records: &BTreeMap<&str, &EvidenceRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> usize {
    let mut verified = 0usize;
    let mut seen = BTreeSet::new();
    for required in &plan.required_devices {
        if !seen.insert((required.device_id.as_str(), required.device_revision.as_str())) {
            push(
                diagnostics,
                "REQUIRED_DEVICE_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(&required.device_id),
                "required device identity/revision is duplicated",
            );
            continue;
        }
        let matches: Vec<&DeviceEvidence> = plan
            .device_evidence
            .iter()
            .filter(|item| {
                item.device_id == required.device_id
                    && item.device_revision == required.device_revision
            })
            .collect();
        if matches.len() != 1 {
            push(
                diagnostics,
                "REQUIRED_DEVICE_STATE_MISSING_OR_AMBIGUOUS",
                DiagnosticSeverity::Indeterminate,
                Some(&required.device_id),
                "exactly one current status record is required for each protective/indicating device",
            );
            continue;
        }
        let device = matches[0];
        let evidence = records.get(device.evidence_record_id.as_str()).copied();
        let evidence_valid = evidence.is_some_and(|record| {
            record.evidence_class == EvidenceClass::LimiterVerification
                && record.subject_id == device.device_id
                && record.subject_revision == device.device_revision
                && validate_record(plan, record, diagnostics)
        });
        if !evidence_valid {
            if evidence.is_none() {
                push(
                    diagnostics,
                    "DEVICE_VERIFICATION_EVIDENCE_MISSING",
                    DiagnosticSeverity::Indeterminate,
                    Some(&device.device_id),
                    "device state must bind a current limiter-verification evidence record",
                );
            } else if let Some(record) = evidence {
                if record.evidence_class != EvidenceClass::LimiterVerification
                    || record.subject_id != device.device_id
                    || record.subject_revision != device.device_revision
                {
                    push(
                        diagnostics,
                        "DEVICE_VERIFICATION_SUBJECT_MISMATCH",
                        DiagnosticSeverity::Indeterminate,
                        Some(&device.device_id),
                        "device verification must bind the exact device and revision",
                    );
                }
            }
        }
        match device.state {
            DeviceState::Operational if evidence_valid => verified += 1,
            DeviceState::Operational => {}
            DeviceState::Faulted => push(
                diagnostics,
                "REQUIRED_DEVICE_FAULTED",
                DiagnosticSeverity::Rejected,
                Some(&device.device_id),
                "required protective/indicating device is faulted",
            ),
            DeviceState::Bypassed => push(
                diagnostics,
                "REQUIRED_DEVICE_BYPASSED",
                DiagnosticSeverity::Rejected,
                Some(&device.device_id),
                "required protective/indicating device is bypassed",
            ),
            DeviceState::Unknown => push(
                diagnostics,
                "REQUIRED_DEVICE_STATE_UNKNOWN",
                DiagnosticSeverity::Indeterminate,
                Some(&device.device_id),
                "required protective/indicating device state is unknown",
            ),
        }
    }
    verified
}

fn validate_roles(
    plan: &ReadinessPlan,
    records: &BTreeMap<&str, &EvidenceRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> usize {
    let mut verified = 0usize;
    let mut required = BTreeSet::new();
    for role in &plan.required_roles {
        if !required.insert(*role) {
            push(
                diagnostics,
                "REQUIRED_ROLE_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.readiness_id),
                "required role is listed more than once",
            );
            continue;
        }
        let assignments: Vec<&RoleAssignment> = plan
            .role_assignments
            .iter()
            .filter(|item| item.role == *role)
            .collect();
        if assignments.len() != 1 {
            push(
                diagnostics,
                "ROLE_ASSIGNMENT_MISSING_OR_AMBIGUOUS",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.readiness_id),
                format!("exactly one pseudonymous assignment is required for role {role:?}"),
            );
            continue;
        }
        let assignment = assignments[0];
        if !nonblank(&assignment.person_reference) {
            push(
                diagnostics,
                "ROLE_PERSON_REFERENCE_EMPTY",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.readiness_id),
                "person reference must be nonblank and pseudonymous",
            );
            continue;
        }
        match records.get(assignment.credential_record_id.as_str()).copied() {
            Some(record)
                if record.evidence_class == EvidenceClass::PersonnelCredential
                    && record.subject_id == assignment.person_reference
                    && record.personnel_role == Some(role) =>
            {
                if validate_record(plan, record, diagnostics) {
                    verified += 1;
                }
            }
            Some(_) => push(
                diagnostics,
                "ROLE_CREDENTIAL_SUBJECT_OR_SCOPE_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(&assignment.person_reference),
                "credential must bind the pseudonymous person, exact role and operation scope",
            ),
            None => push(
                diagnostics,
                "ROLE_CREDENTIAL_MISSING",
                DiagnosticSeverity::Indeterminate,
                Some(&assignment.person_reference),
                "required personnel credential record is missing",
            ),
        }
    }
    verified
}

fn validate_briefing(
    plan: &ReadinessPlan,
    records: &BTreeMap<&str, &EvidenceRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let briefing_records: Vec<&EvidenceRecord> = records
        .values()
        .copied()
        .filter(|record| {
            record.evidence_class == EvidenceClass::SafeWorkBriefing
                && record.subject_id == plan.method_statement_id
                && record.subject_revision == plan.method_statement_revision
                && record.personnel_role.is_none()
        })
        .collect();
    let briefing_valid = match briefing_records.as_slice() {
        [record] => validate_record(plan, record, diagnostics),
        [] => {
            push(
                diagnostics,
                "SAFE_WORK_BRIEFING_RECORD_MISSING",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.method_statement_id),
                "briefing evidence for the exact method statement revision is missing",
            );
            false
        }
        _ => {
            push(
                diagnostics,
                "SAFE_WORK_BRIEFING_RECORD_AMBIGUOUS",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.method_statement_id),
                "multiple briefing records match the exact method statement revision",
            );
            false
        }
    };

    let mut complete = briefing_valid;
    let mut seen_acknowledgement_ids = BTreeSet::new();
    for acknowledgement in &plan.briefing_acknowledgements {
        if !seen_acknowledgement_ids.insert(acknowledgement.evidence_record_id.as_str()) {
            push(
                diagnostics,
                "BRIEFING_ACK_RECORD_DUPLICATE",
                DiagnosticSeverity::Indeterminate,
                Some(&acknowledgement.evidence_record_id),
                "one acknowledgement evidence record was reused more than once",
            );
            complete = false;
        }
    }

    for role in &plan.required_roles {
        let assigned: Vec<&RoleAssignment> = plan
            .role_assignments
            .iter()
            .filter(|assignment| assignment.role == *role)
            .collect();
        let person_reference = match assigned.as_slice() {
            [assignment] => assignment.person_reference.as_str(),
            _ => {
                complete = false;
                continue;
            }
        };
        let acknowledgements: Vec<&BriefingAcknowledgement> = plan
            .briefing_acknowledgements
            .iter()
            .filter(|item| item.role == *role)
            .collect();
        let acknowledgement = match acknowledgements.as_slice() {
            [item] => *item,
            [] => {
                complete = false;
                push(
                    diagnostics,
                    "BRIEFING_ACK_MISSING",
                    DiagnosticSeverity::Indeterminate,
                    Some(&plan.method_statement_id),
                    format!("required role {role:?} has not acknowledged the briefing"),
                );
                continue;
            }
            _ => {
                complete = false;
                push(
                    diagnostics,
                    "BRIEFING_ACK_AMBIGUOUS",
                    DiagnosticSeverity::Indeterminate,
                    Some(&plan.method_statement_id),
                    format!("multiple briefing acknowledgements exist for role {role:?}"),
                );
                continue;
            }
        };

        if acknowledgement.person_reference != person_reference
            || acknowledgement.method_statement_id != plan.method_statement_id
            || acknowledgement.method_statement_revision != plan.method_statement_revision
            || acknowledgement.acknowledged_at_tick > plan.start_tick
        {
            complete = false;
            push(
                diagnostics,
                "BRIEFING_ACK_SUBJECT_OR_TIME_MISMATCH",
                DiagnosticSeverity::Indeterminate,
                Some(&acknowledgement.evidence_record_id),
                "acknowledgement must bind the assigned person, exact method-statement revision and occur no later than operation start",
            );
            continue;
        }

        match records.get(acknowledgement.evidence_record_id.as_str()).copied() {
            Some(record)
                if record.evidence_class == EvidenceClass::BriefingAcknowledgement
                    && record.subject_id == acknowledgement.person_reference
                    && record.subject_revision == plan.method_statement_revision
                    && record.personnel_role == Some(*role) =>
            {
                if !validate_record(plan, record, diagnostics) {
                    complete = false;
                }
            }
            Some(_) => {
                complete = false;
                push(
                    diagnostics,
                    "BRIEFING_ACK_EVIDENCE_MISMATCH",
                    DiagnosticSeverity::Indeterminate,
                    Some(&acknowledgement.evidence_record_id),
                    "acknowledgement evidence must bind the exact person, role and method-statement revision",
                );
            }
            None => {
                complete = false;
                push(
                    diagnostics,
                    "BRIEFING_ACK_EVIDENCE_MISSING",
                    DiagnosticSeverity::Indeterminate,
                    Some(&acknowledgement.evidence_record_id),
                    "source evidence for this role acknowledgement is missing",
                );
            }
        }
    }
    complete
}

fn validate_exclusion_zone(
    plan: &ReadinessPlan,
    records: &BTreeMap<&str, &EvidenceRecord>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let record = records.values().copied().find(|record| {
        record.evidence_class == EvidenceClass::ExclusionZonePlan
            && record.subject_id == plan.site_epoch_id
            && record.subject_revision == plan.exclusion_zone_plan_revision
    });
    let evidence_valid = match record {
        Some(record) => validate_record(plan, record, diagnostics),
        None => {
            push(
                diagnostics,
                "EXCLUSION_ZONE_EVIDENCE_MISSING",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.site_epoch_id),
                "site-epoch-bound exclusion-zone evidence is required",
            );
            false
        }
    };
    match plan.exclusion_zone_state {
        ExclusionZoneState::Enforced if evidence_valid => true,
        ExclusionZoneState::Enforced => false,
        ExclusionZoneState::NotEnforced => {
            push(
                diagnostics,
                "EXCLUSION_ZONE_NOT_ENFORCED",
                DiagnosticSeverity::Rejected,
                Some(&plan.site_epoch_id),
                "required exclusion-zone controls are declared not enforced",
            );
            false
        }
        ExclusionZoneState::Unknown => {
            push(
                diagnostics,
                "EXCLUSION_ZONE_STATE_UNKNOWN",
                DiagnosticSeverity::Indeterminate,
                Some(&plan.site_epoch_id),
                "exclusion-zone enforcement state is unknown",
            );
            false
        }
    }
}


/// Reasons why previously evaluated readiness evidence must be re-evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessInvalidationReason {
    EvidenceStatusChanged,
    EvidenceDigestChanged,
    DeviceStateChanged,
    SubjectRevisionChanged,
    ExclusionZoneStateChanged,
    LiftPreflightChanged,
    OperationWindowChanged,
    ClockDomainChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadinessInvalidation {
    pub reason: ReadinessInvalidationReason,
    pub dependency_id: String,
    pub previous_value: String,
    pub current_value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessReplayDisposition {
    Current,
    Invalidated,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessReplayReport {
    pub disposition: ReadinessReplayDisposition,
    pub invalidations: Vec<ReadinessInvalidation>,
    pub applied_source_count: usize,
    pub stale_event_count: usize,
    pub duplicate_event_count: usize,
    pub conflicted_source_count: usize,
}

/// Source-owned change event. Sequence is monotonic within the named source_id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessEvent {
    EvidenceStatusChanged {
        record_id: String,
        new_status: EvidenceStatus,
    },
    EvidenceDigestChanged {
        record_id: String,
        new_sha256: String,
    },
    DeviceStateChanged {
        device_id: String,
        device_revision: String,
        new_state: DeviceState,
    },
    SubjectRevisionChanged {
        evidence_class: EvidenceClass,
        subject_id: String,
        new_revision: String,
    },
    ExclusionZoneStateChanged {
        site_epoch_id: String,
        new_state: ExclusionZoneState,
    },
    LiftPreflightChanged {
        receipt_id: String,
        input_sha256: String,
        report_sha256: String,
        disposition: LiftPreflightDisposition,
        invalidated: bool,
    },
    OperationWindowChanged {
        readiness_id: String,
        new_start_tick: u64,
        new_end_tick: u64,
    },
    ClockDomainChanged {
        readiness_id: String,
        new_clock_domain_id: String,
    },
}

impl ReadinessEvent {
    /// Canonical source identity: the event cannot claim a different source from
    /// the dependency it changes.
    pub fn expected_source_id(&self) -> String {
        match self {
            Self::EvidenceStatusChanged { record_id, .. }
            | Self::EvidenceDigestChanged { record_id, .. } => record_id.clone(),
            Self::DeviceStateChanged { device_id, .. } => device_id.clone(),
            Self::SubjectRevisionChanged { evidence_class, subject_id, .. } => {
                format!("subject::{evidence_class:?}::{subject_id}")
            }
            Self::ExclusionZoneStateChanged { site_epoch_id, .. } => {
                format!("exclusion-zone::{site_epoch_id}")
            }
            Self::LiftPreflightChanged { receipt_id, .. } => receipt_id.clone(),
            Self::OperationWindowChanged { readiness_id, .. } => {
                format!("operation-window::{readiness_id}")
            }
            Self::ClockDomainChanged { readiness_id, .. } => {
                format!("clock-domain::{readiness_id}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ReadinessEventEnvelope {
    pub event_id: String,
    pub source_id: String,
    pub source_sequence: u64,
    pub event: ReadinessEvent,
}

/// Dependency snapshot for deterministic batch replay. Newer source-sequence
/// events supersede older events from the same source, regardless of arrival
/// order. Same-sequence conflicting payloads fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadinessDependencies {
    plan: ReadinessPlan,
}

impl ReadinessDependencies {
    pub fn from_plan(plan: &ReadinessPlan) -> Self {
        Self { plan: plan.clone() }
    }

    pub fn invalidation_for(&self, event: &ReadinessEvent) -> Option<ReadinessInvalidation> {
        let plan = &self.plan;
        match event {
            ReadinessEvent::EvidenceStatusChanged { record_id, new_status } => {
                let record = plan.evidence_records.iter().find(|item| item.record_id == *record_id)?;
                (record.status != *new_status).then(|| ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::EvidenceStatusChanged,
                    dependency_id: record_id.clone(),
                    previous_value: format!("{:?}", record.status),
                    current_value: format!("{:?}", new_status),
                })
            }
            ReadinessEvent::EvidenceDigestChanged { record_id, new_sha256 } => {
                let record = plan.evidence_records.iter().find(|item| item.record_id == *record_id)?;
                (!record.sha256.eq_ignore_ascii_case(new_sha256)).then(|| ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::EvidenceDigestChanged,
                    dependency_id: record_id.clone(),
                    previous_value: record.sha256.to_ascii_lowercase(),
                    current_value: new_sha256.to_ascii_lowercase(),
                })
            }
            ReadinessEvent::DeviceStateChanged { device_id, device_revision, new_state } => {
                let device = plan.device_evidence.iter().find(|item| {
                    item.device_id == *device_id && item.device_revision == *device_revision
                })?;
                (device.state != *new_state).then(|| ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::DeviceStateChanged,
                    dependency_id: device_id.clone(),
                    previous_value: format!("{:?}", device.state),
                    current_value: format!("{:?}", new_state),
                })
            }
            ReadinessEvent::SubjectRevisionChanged { evidence_class, subject_id, new_revision } => {
                let previous = plan.evidence_records.iter().find(|item| {
                    item.evidence_class == *evidence_class && item.subject_id == *subject_id
                })?.subject_revision.clone();
                (previous != *new_revision).then(|| ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::SubjectRevisionChanged,
                    dependency_id: subject_id.clone(),
                    previous_value: previous,
                    current_value: new_revision.clone(),
                })
            }
            ReadinessEvent::ExclusionZoneStateChanged { site_epoch_id, new_state } => {
                if *site_epoch_id != plan.site_epoch_id || plan.exclusion_zone_state == *new_state {
                    return None;
                }
                Some(ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::ExclusionZoneStateChanged,
                    dependency_id: site_epoch_id.clone(),
                    previous_value: format!("{:?}", plan.exclusion_zone_state),
                    current_value: format!("{:?}", new_state),
                })
            }
            ReadinessEvent::LiftPreflightChanged {
                receipt_id,
                input_sha256,
                report_sha256,
                disposition,
                invalidated,
            } => {
                let receipt = &plan.lift_preflight;
                if receipt.receipt_id != *receipt_id
                    || (receipt.input_sha256.eq_ignore_ascii_case(input_sha256)
                        && receipt.report_sha256.eq_ignore_ascii_case(report_sha256)
                        && receipt.disposition == *disposition
                        && receipt.invalidated == *invalidated)
                {
                    return None;
                }
                Some(ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::LiftPreflightChanged,
                    dependency_id: receipt_id.clone(),
                    previous_value: format!(
                        "{}|{}|{:?}|{}",
                        receipt.input_sha256,
                        receipt.report_sha256,
                        receipt.disposition,
                        receipt.invalidated
                    ),
                    current_value: format!(
                        "{}|{}|{:?}|{}",
                        input_sha256,
                        report_sha256,
                        disposition,
                        invalidated
                    ),
                })
            }
            ReadinessEvent::OperationWindowChanged { readiness_id, new_start_tick, new_end_tick } => {
                if *readiness_id != plan.readiness_id
                    || (plan.start_tick == *new_start_tick && plan.end_tick == *new_end_tick)
                {
                    return None;
                }
                Some(ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::OperationWindowChanged,
                    dependency_id: readiness_id.clone(),
                    previous_value: format!("[{}, {})", plan.start_tick, plan.end_tick),
                    current_value: format!("[{}, {})", new_start_tick, new_end_tick),
                })
            }
            ReadinessEvent::ClockDomainChanged { readiness_id, new_clock_domain_id } => {
                if *readiness_id != plan.readiness_id || plan.clock_domain_id == *new_clock_domain_id {
                    return None;
                }
                Some(ReadinessInvalidation {
                    reason: ReadinessInvalidationReason::ClockDomainChanged,
                    dependency_id: readiness_id.clone(),
                    previous_value: plan.clock_domain_id.clone(),
                    current_value: new_clock_domain_id.clone(),
                })
            }
        }
    }

    /// Replays a batch independent of arrival order. For each source, the highest
    /// sequence is authoritative. Exact duplicate events are idempotent. Conflicting
    /// event IDs or distinct payloads at the same highest sequence are indeterminate.
    pub fn replay(&self, events: &[ReadinessEventEnvelope]) -> ReadinessReplayReport {
        let mut event_ids: BTreeMap<&str, &ReadinessEventEnvelope> = BTreeMap::new();
        let mut source_events: BTreeMap<&str, BTreeMap<u64, BTreeSet<ReadinessEvent>>> =
            BTreeMap::new();
        let mut conflicts = BTreeSet::new();
        let mut duplicate_event_count = 0usize;
        let mut unique_event_count = 0usize;

        for envelope in events {
            if !nonblank(&envelope.event_id)
                || !nonblank(&envelope.source_id)
                || envelope.source_sequence == 0
                || envelope.source_id != envelope.event.expected_source_id()
            {
                conflicts.insert(envelope.source_id.clone());
                continue;
            }
            if let Some(existing) = event_ids.get(envelope.event_id.as_str()) {
                if *existing == envelope {
                    duplicate_event_count += 1;
                    continue;
                }
                conflicts.insert(envelope.source_id.clone());
                continue;
            }
            event_ids.insert(envelope.event_id.as_str(), envelope);
            source_events
                .entry(envelope.source_id.as_str())
                .or_default()
                .entry(envelope.source_sequence)
                .or_default()
                .insert(envelope.event.clone());
            unique_event_count += 1;
        }

        let mut invalidations = BTreeSet::new();
        let mut stale_event_count = 0usize;
        let mut applied_source_count = 0usize;
        for (source_id, sequence_map) in &source_events {
            let Some((latest_sequence, latest_payloads)) = sequence_map.iter().next_back() else {
                continue;
            };
            stale_event_count += sequence_map
                .iter()
                .filter(|(sequence, _)| *sequence < latest_sequence)
                .map(|(_, payloads)| payloads.len())
                .sum::<usize>();
            if latest_payloads.len() != 1 {
                conflicts.insert((*source_id).to_owned());
                continue;
            }
            let Some(event) = latest_payloads.iter().next() else {
                continue;
            };
            applied_source_count += 1;
            if let Some(invalidation) = self.invalidation_for(event) {
                invalidations.insert(invalidation);
            }
        }
        let _ = unique_event_count;

        let invalidations: Vec<_> = invalidations.into_iter().collect();
        let disposition = if !conflicts.is_empty() {
            ReadinessReplayDisposition::Indeterminate
        } else if !invalidations.is_empty() {
            ReadinessReplayDisposition::Invalidated
        } else {
            ReadinessReplayDisposition::Current
        };
        ReadinessReplayReport {
            disposition,
            invalidations,
            applied_source_count,
            stale_event_count,
            duplicate_event_count,
            conflicted_source_count: conflicts.len(),
        }
    }
}

pub fn evaluate(plan: &ReadinessPlan) -> ReadinessReport {
    let mut diagnostics = Vec::new();
    validate_context(plan, &mut diagnostics);
    validate_lift_receipt(plan, &mut diagnostics);
    let records = index_evidence(plan, &mut diagnostics);

    let verified_evidence_count =
        validate_required_evidence(plan, &records, &mut diagnostics);
    let verified_device_count = validate_devices(plan, &records, &mut diagnostics);
    let verified_role_count = validate_roles(plan, &records, &mut diagnostics);
    validate_briefing(plan, &records, &mut diagnostics);
    validate_exclusion_zone(plan, &records, &mut diagnostics);

    diagnostics.sort();
    ReadinessReport {
        contract_version: CONTRACT_VERSION.to_owned(),
        disposition: disposition_for(&diagnostics),
        diagnostics,
        verified_evidence_count,
        required_evidence_count: plan.required_evidence.len(),
        verified_device_count,
        required_device_count: plan.required_devices.len(),
        verified_role_count,
        required_role_count: plan.required_roles.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        class: EvidenceClass,
        id: &str,
        subject: &str,
        revision: &str,
        role: Option<PersonnelRole>,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_class: class,
            record_id: id.to_owned(),
            owner_id: format!("owner::{id}"),
            document_revision: "doc-r1".to_owned(),
            subject_id: subject.to_owned(),
            subject_revision: revision.to_owned(),
            sha256: "a".repeat(64),
            clock_domain_id: "yard-clock-01".to_owned(),
            valid_from_tick: 0,
            valid_until_tick: 1_000,
            status: EvidenceStatus::Current,
            personnel_role: role,
        }
    }

    fn baseline() -> ReadinessPlan {
        let required_roles = vec![
            PersonnelRole::Operator,
            PersonnelRole::Slinger,
            PersonnelRole::Signaller,
            PersonnelRole::Supervisor,
            PersonnelRole::AppointedPerson,
        ];
        let mut evidence_records = vec![
            record(EvidenceClass::CraneInspection, "crane-insp-1", "gantry-1", "crane-r4", None),
            record(EvidenceClass::SupportingStructureInspection, "structure-insp-1", "runway-1", "runway-r2", None),
            record(EvidenceClass::DutyMonitoring, "duty-1", "gantry-1", "duty-r3", None),
            record(EvidenceClass::LimiterVerification, "limiter-1", "load-limiter-1", "device-r2", None),
            record(EvidenceClass::LimiterVerification, "limiter-2", "hoist-limit-1", "device-r1", None),
            record(EvidenceClass::SafeWorkBriefing, "briefing-1", "method-1", "method-r5", None),
            record(EvidenceClass::ExclusionZonePlan, "exclusion-1", "yard-epoch-7", "zone-r3", None),
        ];
        for (index, role) in required_roles.iter().enumerate() {
            let id = format!("credential-{index}");
            let person = format!("person-ref-{index}");
            evidence_records.push(record(
                EvidenceClass::PersonnelCredential,
                &id,
                &person,
                "credential-r1",
                Some(*role),
            ));
            evidence_records.push(record(
                EvidenceClass::BriefingAcknowledgement,
                &format!("ack-{index}"),
                &person,
                "method-r5",
                Some(*role),
            ));
        }
        ReadinessPlan {
            contract_version: CONTRACT_VERSION.to_owned(),
            readiness_id: "readiness-001".to_owned(),
            crane_id: "gantry-1".to_owned(),
            crane_configuration_revision: "crane-r4".to_owned(),
            supporting_structure_id: "runway-1".to_owned(),
            supporting_structure_revision: "runway-r2".to_owned(),
            site_epoch_id: "yard-epoch-7".to_owned(),
            clock_domain_id: "yard-clock-01".to_owned(),
            start_tick: 100,
            end_tick: 200,
            method_statement_id: "method-1".to_owned(),
            method_statement_revision: "method-r5".to_owned(),
            exclusion_zone_plan_revision: "zone-r3".to_owned(),
            lift_preflight: LiftPreflightReceiptRef {
                receipt_id: "lift-receipt-001".to_owned(),
                input_sha256: "b".repeat(64),
                report_sha256: "c".repeat(64),
                profile_version: "shipyard-lift-envelope-v0.1".to_owned(),
                evaluator_revision: "symtropy-lift-eval-r1".to_owned(),
                disposition: LiftPreflightDisposition::ReadyForHumanReview,
                invalidated: false,
                crane_id: "gantry-1".to_owned(),
                crane_configuration_revision: "crane-r4".to_owned(),
                site_epoch_id: "yard-epoch-7".to_owned(),
                clock_domain_id: "yard-clock-01".to_owned(),
                start_tick: 100,
                end_tick: 200,
                issued_at_tick: 50,
                valid_until_tick: 200,
            },
            required_evidence: vec![
                RequiredEvidence {
                    evidence_class: EvidenceClass::CraneInspection,
                    subject_id: "gantry-1".to_owned(),
                    subject_revision: "crane-r4".to_owned(),
                    personnel_role: None,
                },
                RequiredEvidence {
                    evidence_class: EvidenceClass::SupportingStructureInspection,
                    subject_id: "runway-1".to_owned(),
                    subject_revision: "runway-r2".to_owned(),
                    personnel_role: None,
                },
                RequiredEvidence {
                    evidence_class: EvidenceClass::DutyMonitoring,
                    subject_id: "gantry-1".to_owned(),
                    subject_revision: "duty-r3".to_owned(),
                    personnel_role: None,
                },
                RequiredEvidence {
                    evidence_class: EvidenceClass::SafeWorkBriefing,
                    subject_id: "method-1".to_owned(),
                    subject_revision: "method-r5".to_owned(),
                    personnel_role: None,
                },
                RequiredEvidence {
                    evidence_class: EvidenceClass::ExclusionZonePlan,
                    subject_id: "yard-epoch-7".to_owned(),
                    subject_revision: "zone-r3".to_owned(),
                    personnel_role: None,
                },
            ],
            evidence_records: std::mem::take(&mut evidence_records),
            required_devices: vec![
                RequiredDevice {
                    device_id: "load-limiter-1".to_owned(),
                    device_revision: "device-r2".to_owned(),
                },
                RequiredDevice {
                    device_id: "hoist-limit-1".to_owned(),
                    device_revision: "device-r1".to_owned(),
                },
            ],
            device_evidence: vec![
                DeviceEvidence {
                    device_id: "load-limiter-1".to_owned(),
                    device_revision: "device-r2".to_owned(),
                    state: DeviceState::Operational,
                    evidence_record_id: "limiter-1".to_owned(),
                },
                DeviceEvidence {
                    device_id: "hoist-limit-1".to_owned(),
                    device_revision: "device-r1".to_owned(),
                    state: DeviceState::Operational,
                    evidence_record_id: "limiter-2".to_owned(),
                },
            ],
            required_roles: required_roles.clone(),
            role_assignments: required_roles
                .iter()
                .enumerate()
                .map(|(index, role)| RoleAssignment {
                    role: *role,
                    person_reference: format!("person-ref-{index}"),
                    credential_record_id: format!("credential-{index}"),
                })
                .collect(),
            briefing_acknowledgements: required_roles
                .iter()
                .enumerate()
                .map(|(index, role)| BriefingAcknowledgement {
                    role: *role,
                    person_reference: format!("person-ref-{index}"),
                    method_statement_id: "method-1".to_owned(),
                    method_statement_revision: "method-r5".to_owned(),
                    acknowledged_at_tick: 80,
                    evidence_record_id: format!("ack-{index}"),
                })
                .collect(),
            exclusion_zone_state: ExclusionZoneState::Enforced,
        }
    }

    fn has_code(report: &ReadinessReport, code: &str) -> bool {
        report.diagnostics.iter().any(|item| item.code == code)
    }

    #[test]
    fn current_source_status_change_invalidates_readiness() {
        let plan = baseline();
        let dependencies = ReadinessDependencies::from_plan(&plan);
        let event = ReadinessEventEnvelope {
            event_id: "event-001".to_owned(),
            source_id: "crane-insp-1".to_owned(),
            source_sequence: 2,
            event: ReadinessEvent::EvidenceStatusChanged {
                record_id: "crane-insp-1".to_owned(),
                new_status: EvidenceStatus::Revoked,
            },
        };
        let report = dependencies.replay(&[event]);
        assert_eq!(report.disposition, ReadinessReplayDisposition::Invalidated);
        assert_eq!(report.invalidations.len(), 1);
        assert_eq!(
            report.invalidations[0].reason,
            ReadinessInvalidationReason::EvidenceStatusChanged
        );
    }

    #[test]
    fn no_op_and_unrelated_events_do_not_invalidate_readiness() {
        let plan = baseline();
        let dependencies = ReadinessDependencies::from_plan(&plan);
        let events = [
            ReadinessEventEnvelope {
                event_id: "event-noop".to_owned(),
                source_id: "crane-insp-1".to_owned(),
                source_sequence: 2,
                event: ReadinessEvent::EvidenceStatusChanged {
                    record_id: "crane-insp-1".to_owned(),
                    new_status: EvidenceStatus::Current,
                },
            },
            ReadinessEventEnvelope {
                event_id: "event-unrelated".to_owned(),
                source_id: "unrelated-record".to_owned(),
                source_sequence: 9,
                event: ReadinessEvent::EvidenceStatusChanged {
                    record_id: "unrelated-record".to_owned(),
                    new_status: EvidenceStatus::Revoked,
                },
            },
        ];
        let report = dependencies.replay(&events);
        assert_eq!(report.disposition, ReadinessReplayDisposition::Current);
        assert!(report.invalidations.is_empty());
    }

    #[test]
    fn newer_source_event_supersedes_stale_event_and_is_arrival_order_independent() {
        let plan = baseline();
        let dependencies = ReadinessDependencies::from_plan(&plan);
        let older = ReadinessEventEnvelope {
            event_id: "event-older".to_owned(),
            source_id: "crane-insp-1".to_owned(),
            source_sequence: 1,
            event: ReadinessEvent::EvidenceStatusChanged {
                record_id: "crane-insp-1".to_owned(),
                new_status: EvidenceStatus::Revoked,
            },
        };
        let newer = ReadinessEventEnvelope {
            event_id: "event-newer".to_owned(),
            source_id: "crane-insp-1".to_owned(),
            source_sequence: 2,
            event: ReadinessEvent::EvidenceStatusChanged {
                record_id: "crane-insp-1".to_owned(),
                new_status: EvidenceStatus::Current,
            },
        };
        let forward = dependencies.replay(&[older.clone(), newer.clone()]);
        let reverse = dependencies.replay(&[newer, older]);
        assert_eq!(forward, reverse);
        assert_eq!(forward.disposition, ReadinessReplayDisposition::Current);
        assert!(forward.invalidations.is_empty());
        assert_eq!(forward.stale_event_count, 1);
    }

    #[test]
    fn latest_fault_event_invalidates_regardless_of_arrival_order() {
        let plan = baseline();
        let dependencies = ReadinessDependencies::from_plan(&plan);
        let older = ReadinessEventEnvelope {
            event_id: "device-recovered".to_owned(),
            source_id: "load-limiter-1".to_owned(),
            source_sequence: 4,
            event: ReadinessEvent::DeviceStateChanged {
                device_id: "load-limiter-1".to_owned(),
                device_revision: "device-r2".to_owned(),
                new_state: DeviceState::Operational,
            },
        };
        let newer = ReadinessEventEnvelope {
            event_id: "device-faulted".to_owned(),
            source_id: "load-limiter-1".to_owned(),
            source_sequence: 5,
            event: ReadinessEvent::DeviceStateChanged {
                device_id: "load-limiter-1".to_owned(),
                device_revision: "device-r2".to_owned(),
                new_state: DeviceState::Faulted,
            },
        };
        let forward = dependencies.replay(&[older.clone(), newer.clone()]);
        let reverse = dependencies.replay(&[newer, older]);
        assert_eq!(forward, reverse);
        assert_eq!(forward.disposition, ReadinessReplayDisposition::Invalidated);
        assert_eq!(
            forward.invalidations[0].reason,
            ReadinessInvalidationReason::DeviceStateChanged
        );
    }

    #[test]
    fn conflicting_payloads_at_the_same_source_sequence_are_indeterminate() {
        let dependencies = ReadinessDependencies::from_plan(&baseline());
        let events = [
            ReadinessEventEnvelope {
                event_id: "event-a".to_owned(),
                source_id: "crane-insp-1".to_owned(),
                source_sequence: 8,
                event: ReadinessEvent::EvidenceStatusChanged {
                    record_id: "crane-insp-1".to_owned(),
                    new_status: EvidenceStatus::Revoked,
                },
            },
            ReadinessEventEnvelope {
                event_id: "event-b".to_owned(),
                source_id: "crane-insp-1".to_owned(),
                source_sequence: 8,
                event: ReadinessEvent::EvidenceStatusChanged {
                    record_id: "crane-insp-1".to_owned(),
                    new_status: EvidenceStatus::Current,
                },
            },
        ];
        let report = dependencies.replay(&events);
        assert_eq!(report.disposition, ReadinessReplayDisposition::Indeterminate);
        assert_eq!(report.conflicted_source_count, 1);
    }

    #[test]
    fn exact_duplicate_event_is_idempotent_and_counted_once() {
        let dependencies = ReadinessDependencies::from_plan(&baseline());
        let event = ReadinessEventEnvelope {
            event_id: "event-duplicate".to_owned(),
            source_id: "crane-insp-1".to_owned(),
            source_sequence: 2,
            event: ReadinessEvent::EvidenceStatusChanged {
                record_id: "crane-insp-1".to_owned(),
                new_status: EvidenceStatus::Current,
            },
        };
        let report = dependencies.replay(&[event.clone(), event]);
        assert_eq!(report.disposition, ReadinessReplayDisposition::Current);
        assert_eq!(report.duplicate_event_count, 1);
        assert_eq!(report.applied_source_count, 1);
    }

    #[test]
    fn event_source_identity_mismatch_is_indeterminate() {
        let dependencies = ReadinessDependencies::from_plan(&baseline());
        let event = ReadinessEventEnvelope {
            event_id: "event-spoofed".to_owned(),
            source_id: "different-record".to_owned(),
            source_sequence: 2,
            event: ReadinessEvent::EvidenceStatusChanged {
                record_id: "crane-insp-1".to_owned(),
                new_status: EvidenceStatus::Revoked,
            },
        };
        let report = dependencies.replay(&[event]);
        assert_eq!(report.disposition, ReadinessReplayDisposition::Indeterminate);
        assert_eq!(report.conflicted_source_count, 1);
    }

    #[test]
    fn frozen_conformance_fixture_maps_all_vectors_to_rust_tests() {
        const FIXTURE: &str = include_str!("../fixtures/conformance-vectors-v0.1.json");
        let mappings: [(&str, &str); 15] = [
            ("CRR-VERSION-001", stringify!(unsupported_readiness_contract_version_is_indeterminate)),
            ("CRR-BASE-001", stringify!(complete_current_evidence_is_ready_for_human_review_only)),
            ("CRR-INSP-001", stringify!(missing_crane_inspection_is_indeterminate)),
            ("CRR-INSP-002", stringify!(revoked_crane_inspection_is_rejected)),
            ("CRR-DEVICE-001", stringify!(bypassed_required_limiter_is_rejected)),
            ("CRR-DUTY-001", stringify!(stale_duty_monitoring_evidence_is_indeterminate)),
            ("CRR-PEOPLE-001", stringify!(missing_required_personnel_role_is_indeterminate)),
            ("CRR-BRIEF-001", stringify!(briefing_without_all_required_role_acknowledgements_is_indeterminate)),
            ("CRR-BRIEF-002", stringify!(acknowledgement_for_another_method_statement_revision_is_indeterminate)),
            ("CRR-INSP-003", stringify!(duplicate_matching_inspection_records_are_ambiguous)),
            ("CRR-ZONE-001", stringify!(unenforced_exclusion_zone_is_rejected)),
            ("CRR-PREFLIGHT-001", stringify!(invalidated_or_wrong_crane_lift_receipt_is_indeterminate)),
            ("CRR-META-001", stringify!(evidence_input_permutation_preserves_canonical_report_value)),
            ("CRR-DEVICE-002", stringify!(unknown_limiter_state_is_indeterminate_not_a_pass)),
            ("CRR-EVID-001", stringify!(evidence_record_id_conflict_is_indeterminate)),
        ];
        assert_eq!(FIXTURE.matches(r#""id": "CRR-"#).count(), 15);
        for (vector_id, test_name) in mappings {
            assert!(
                FIXTURE.contains(&format!(r#""id": "{vector_id}""#)),
                "missing conformance vector {vector_id}"
            );
            assert!(
                FIXTURE.contains(&format!(r#""test_name": "{test_name}""#)),
                "missing test mapping {test_name}"
            );
        }
    }

    #[test]
    fn unsupported_readiness_contract_version_is_indeterminate() {
        let mut plan = baseline();
        plan.contract_version = "shipyard-crane-readiness-v0.0".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "READINESS_CONTRACT_VERSION_MISMATCH"));
    }

    #[test]
    fn complete_current_evidence_is_ready_for_human_review_only() {
        let report = evaluate(&baseline());
        assert_eq!(report.disposition, Disposition::ReadyForHumanReview);
        assert_eq!(report.verified_device_count, 2);
        assert_eq!(report.verified_role_count, 5);
    }

    #[test]
    fn missing_crane_inspection_is_indeterminate() {
        let mut plan = baseline();
        plan.evidence_records.retain(|item| item.record_id != "crane-insp-1");
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "REQUIRED_EVIDENCE_MISSING"));
    }

    #[test]
    fn revoked_crane_inspection_is_rejected() {
        let mut plan = baseline();
        plan.evidence_records
            .iter_mut()
            .find(|item| item.record_id == "crane-insp-1")
            .unwrap()
            .status = EvidenceStatus::Revoked;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "EVIDENCE_REVOKED"));
    }

    #[test]
    fn bypassed_required_limiter_is_rejected() {
        let mut plan = baseline();
        plan.device_evidence[0].state = DeviceState::Bypassed;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "REQUIRED_DEVICE_BYPASSED"));
    }

    #[test]
    fn stale_duty_monitoring_evidence_is_indeterminate() {
        let mut plan = baseline();
        plan.evidence_records
            .iter_mut()
            .find(|item| item.record_id == "duty-1")
            .unwrap()
            .valid_until_tick = 199;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_NOT_VALID_FOR_OPERATION"));
    }

    #[test]
    fn missing_required_personnel_role_is_indeterminate() {
        let mut plan = baseline();
        plan.role_assignments
            .retain(|item| item.role != PersonnelRole::Signaller);
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "ROLE_ASSIGNMENT_MISSING_OR_AMBIGUOUS"));
    }

    #[test]
    fn briefing_without_all_required_role_acknowledgements_is_indeterminate() {
        let mut plan = baseline();
        plan.briefing_acknowledgements
            .retain(|acknowledgement| acknowledgement.role != PersonnelRole::Slinger);
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "BRIEFING_ACK_MISSING"));
    }

    #[test]
    fn acknowledgement_for_another_method_statement_revision_is_indeterminate() {
        let mut plan = baseline();
        plan.briefing_acknowledgements[0].method_statement_revision = "method-r4".to_owned();
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "BRIEFING_ACK_SUBJECT_OR_TIME_MISMATCH"));
    }

    #[test]
    fn duplicate_matching_inspection_records_are_ambiguous() {
        let mut plan = baseline();
        let duplicate = plan
            .evidence_records
            .iter()
            .find(|record| record.record_id == "crane-insp-1")
            .unwrap()
            .clone();
        let mut second = duplicate;
        second.record_id = "crane-insp-duplicate".to_owned();
        plan.evidence_records.push(second);
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "REQUIRED_EVIDENCE_AMBIGUOUS"));
    }

    #[test]
    fn unenforced_exclusion_zone_is_rejected() {
        let mut plan = baseline();
        plan.exclusion_zone_state = ExclusionZoneState::NotEnforced;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Rejected);
        assert!(has_code(&report, "EXCLUSION_ZONE_NOT_ENFORCED"));
    }

    #[test]
    fn invalidated_or_wrong_crane_lift_receipt_is_indeterminate() {
        let mut plan = baseline();
        plan.lift_preflight.invalidated = true;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "LIFT_PREFLIGHT_RECEIPT_INVALIDATED"));

        plan.lift_preflight.invalidated = false;
        plan.lift_preflight.crane_configuration_revision = "crane-r3".to_owned();
        let mismatch = evaluate(&plan);
        assert_eq!(mismatch.disposition, Disposition::Indeterminate);
        assert!(has_code(&mismatch, "LIFT_PREFLIGHT_CONTEXT_MISMATCH"));
    }

    #[test]
    fn evidence_input_permutation_preserves_canonical_report_value() {
        let first = baseline();
        let mut second = baseline();
        second.evidence_records.reverse();
        second.role_assignments.reverse();
        second.device_evidence.reverse();
        second.briefing_acknowledgements.reverse();
        assert_eq!(evaluate(&first), evaluate(&second));
    }

    #[test]
    fn unknown_limiter_state_is_indeterminate_not_a_pass() {
        let mut plan = baseline();
        plan.device_evidence[0].state = DeviceState::Unknown;
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "REQUIRED_DEVICE_STATE_UNKNOWN"));
    }

    #[test]
    fn evidence_record_id_conflict_is_indeterminate() {
        let mut plan = baseline();
        let mut conflicting = plan.evidence_records[0].clone();
        conflicting.sha256 = "d".repeat(64);
        plan.evidence_records.push(conflicting);
        let report = evaluate(&plan);
        assert_eq!(report.disposition, Disposition::Indeterminate);
        assert!(has_code(&report, "EVIDENCE_RECORD_ID_CONFLICT"));
    }
}
