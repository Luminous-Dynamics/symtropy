// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source-versioned South African RPAS/UAS applicability and evidence gate.
//!
//! This module evaluates whether a declared evidence package is structurally
//! complete for external operator/regulator review. It is not a legal opinion,
//! cryptographic verifier, SACAA approval, flight authorization or dispatcher.
//! A source URL or a caller's "verified" label is not authenticated here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ZA_UAS_GATE_SCHEMA_VERSION: u16 = 1;
pub const ZA_UAS_GATE_BENCHMARK_ID: &str = "ZA-UAS-OPS-GATE-001";
pub const MAX_OPERATIONS_PER_CORPUS: usize = 64;
pub const MAX_EVIDENCE_ITEMS_PER_OPERATION: usize = 32;
const SHA256_PREFIX: &str = "sha256:";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityClass {
    Commercial,
    Corporate,
    NonProfit,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AircraftControlMode {
    RemotelyPiloted,
    Autonomous,
    NotManagedInRealTime,
    OtherOrUnclassified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AircraftClass {
    Class1A,
    Class1B,
    Class1Other,
    Class2,
    OutsidePart101Class1Or2,
    Unknown,
}

impl AircraftClass {
    fn within_part101_class_1_or_2(self) -> bool {
        matches!(
            self,
            Self::Class1A | Self::Class1B | Self::Class1Other | Self::Class2
        )
    }

    fn private_operation_class(self) -> bool {
        matches!(self, Self::Class1A | Self::Class1B)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingVisibility {
    RestrictedVisualLineOfSight,
    VisualLineOfSight,
    ExtendedVisualLineOfSight,
    BeyondVisualLineOfSight,
    BeyondRadioLineOfSight,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    UasOperatingCertificate,
    AirServiceLicence,
    RemotePilotLicence,
    RemoteOperatorLetterOfApproval,
    CertificateOfRegistration,
    DirectorApprovedPayloadReleaseInOperationsManual,
    DirectorApprovedBvlosInOperationsManual,
    SaCats101BvlosEvidence,
    DirectorApprovedMultiUasOperation,
    ApprovedSingleRpsCommandInputArrangement,
    DirectorApprovedControlledAirspaceOperation,
}

impl ApprovalRequirement {
    fn requires_operations_manual_binding(self) -> bool {
        matches!(
            self,
            Self::DirectorApprovedPayloadReleaseInOperationsManual
                | Self::DirectorApprovedBvlosInOperationsManual
                | Self::DirectorApprovedMultiUasOperation
                | Self::ApprovedSingleRpsCommandInputArrangement
                | Self::DirectorApprovedControlledAirspaceOperation
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    CallerVerifiedCurrent,
    Missing,
    Unknown,
    Stale,
    Expired,
    Revoked,
    Mismatched,
    OutOfScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalEvidence {
    pub requirement: ApprovalRequirement,
    /// This is an upstream adjudication, not a signature verified by this module.
    pub state: EvidenceState,
    pub evidence_record_id: String,
    pub source_reference: String,
    pub source_version: String,
    pub aircraft_serial_digest: String,
    pub operator_identity_digest: String,
    pub operation_scope_digest: String,
    /// Required for Director/Operations-Manual scoped approvals.
    pub operations_manual_revision_digest: Option<String>,
    pub valid_from_unix_ms: u64,
    pub valid_until_unix_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZaUasOperationProfile {
    pub operation_id: String,
    pub activity_class: ActivityClass,
    pub aircraft_control_mode: AircraftControlMode,
    pub aircraft_class: AircraftClass,
    pub operating_visibility: OperatingVisibility,
    pub payload_release_or_delivery: bool,
    pub dangerous_goods_payload: bool,
    pub multi_uas_operation: bool,
    pub controlled_airspace: bool,
    pub aircraft_serial_digest: String,
    pub operator_identity_digest: String,
    pub operation_scope_digest: String,
    pub operations_manual_revision_digest: Option<String>,
    pub assessment_time_unix_ms: u64,
    pub approval_evidence: Vec<ApprovalEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZaUasEligibilityCorpus {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub source_snapshot_id: String,
    pub source_snapshot_as_of: String,
    pub claim_ceiling: String,
    pub operations: Vec<ZaUasOperationProfile>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateStatus {
    Blocked,
    Indeterminate,
    EligibleForExternalOperatorReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequiredEvidenceResult {
    pub requirement: ApprovalRequirement,
    pub state: EvidenceState,
    pub evidence_record_id: Option<String>,
    pub source_reference: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZaUasOperationResult {
    pub operation_id: String,
    pub candidate_status: CandidateStatus,
    pub required_evidence: Vec<RequiredEvidenceResult>,
    pub blocked_reasons: Vec<String>,
    pub indeterminate_reasons: Vec<String>,
    pub source_snapshot_id: String,
    pub claim_ceiling: String,
    pub flight_authorized: bool,
    pub dispatch_authorized: bool,
    pub dispatch_commands_emitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZaUasEligibilityReport {
    pub schema_version: u16,
    pub benchmark_id: String,
    pub source_snapshot_id: String,
    pub source_snapshot_as_of: String,
    pub input_digest_sha256: String,
    pub operation_count: usize,
    pub blocked_count: usize,
    pub indeterminate_count: usize,
    pub eligible_for_external_operator_review_count: usize,
    pub results: Vec<ZaUasOperationResult>,
    pub flight_authorization_granted: bool,
    pub dispatch_commands_emitted: bool,
    pub claim_ceiling: String,
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix(SHA256_PREFIX) else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn canonical_corpus_digest(corpus: &ZaUasEligibilityCorpus) -> Result<String, String> {
    let mut canonical = corpus.clone();
    canonical.operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    for operation in &mut canonical.operations {
        operation.approval_evidence.sort_by(|left, right| {
            left.requirement.cmp(&right.requirement)
        });
    }
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("cannot serialize canonical ZA UAS corpus: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_corpus(corpus: &ZaUasEligibilityCorpus) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if corpus.schema_version != ZA_UAS_GATE_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            ZA_UAS_GATE_SCHEMA_VERSION, corpus.schema_version
        ));
    }
    if corpus.benchmark_id != ZA_UAS_GATE_BENCHMARK_ID {
        errors.push("benchmark_id: unsupported benchmark identifier".into());
    }
    if corpus.source_snapshot_id.trim().is_empty()
        || corpus.source_snapshot_as_of.trim().is_empty()
        || corpus.claim_ceiling.trim().is_empty()
    {
        errors.push("source snapshot identity/date and claim ceiling are required".into());
    }
    if corpus.operations.is_empty() || corpus.operations.len() > MAX_OPERATIONS_PER_CORPUS {
        errors.push(format!(
            "operations: expected 1..={MAX_OPERATIONS_PER_CORPUS} entries"
        ));
    }

    let mut operation_ids = BTreeSet::new();
    for (index, operation) in corpus.operations.iter().enumerate() {
        if operation.operation_id.trim().is_empty()
            || !operation_ids.insert(operation.operation_id.as_str())
        {
            errors.push(format!("operations[{index}].operation_id: nonempty unique ID required"));
        }
        if !valid_sha256(&operation.aircraft_serial_digest)
            || !valid_sha256(&operation.operator_identity_digest)
            || !valid_sha256(&operation.operation_scope_digest)
        {
            errors.push(format!(
                "operations[{index}]: aircraft, operator and operation-scope SHA-256 digests are required"
            ));
        }
        if operation.operations_manual_revision_digest.as_deref()
            .is_some_and(|digest| !valid_sha256(digest))
        {
            errors.push(format!(
                "operations[{index}].operations_manual_revision_digest: malformed SHA-256 digest"
            ));
        }
        if operation.approval_evidence.len() > MAX_EVIDENCE_ITEMS_PER_OPERATION {
            errors.push(format!(
                "operations[{index}].approval_evidence: exceeds {MAX_EVIDENCE_ITEMS_PER_OPERATION} items"
            ));
        }

        let mut requirements = BTreeSet::new();
        for (evidence_index, evidence) in operation.approval_evidence.iter().enumerate() {
            if !requirements.insert(evidence.requirement) {
                errors.push(format!(
                    "operations[{index}].approval_evidence[{evidence_index}]: duplicate requirement"
                ));
            }
            if evidence.evidence_record_id.trim().is_empty()
                || evidence.source_reference.trim().is_empty()
                || evidence.source_version.trim().is_empty()
            {
                errors.push(format!(
                    "operations[{index}].approval_evidence[{evidence_index}]: evidence ID, source reference and source version are required"
                ));
            }
            if !valid_sha256(&evidence.aircraft_serial_digest)
                || !valid_sha256(&evidence.operator_identity_digest)
                || !valid_sha256(&evidence.operation_scope_digest)
            {
                errors.push(format!(
                    "operations[{index}].approval_evidence[{evidence_index}]: subject and scope digests are required"
                ));
            }
            if evidence.operations_manual_revision_digest.as_deref()
                .is_some_and(|digest| !valid_sha256(digest))
            {
                errors.push(format!(
                    "operations[{index}].approval_evidence[{evidence_index}]: malformed Operations Manual digest"
                ));
            }
            if evidence.valid_until_unix_ms <= evidence.valid_from_unix_ms {
                errors.push(format!(
                    "operations[{index}].approval_evidence[{evidence_index}]: validity interval must be nonempty"
                ));
            }
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn required_requirements(operation: &ZaUasOperationProfile) -> BTreeSet<ApprovalRequirement> {
    let mut required = BTreeSet::new();
    match operation.activity_class {
        ActivityClass::Commercial => {
            required.insert(ApprovalRequirement::UasOperatingCertificate);
            required.insert(ApprovalRequirement::AirServiceLicence);
            required.insert(ApprovalRequirement::RemotePilotLicence);
            required.insert(ApprovalRequirement::RemoteOperatorLetterOfApproval);
            required.insert(ApprovalRequirement::CertificateOfRegistration);
        }
        ActivityClass::Corporate | ActivityClass::NonProfit => {
            required.insert(ApprovalRequirement::UasOperatingCertificate);
            required.insert(ApprovalRequirement::RemotePilotLicence);
            required.insert(ApprovalRequirement::RemoteOperatorLetterOfApproval);
            required.insert(ApprovalRequirement::CertificateOfRegistration);
        }
        ActivityClass::Private => {
            // Private-operation exceptions are narrow. R-VLOS/Class 1A/1B
            // conditions and the object-release gate are checked separately.
        }
    }
    if operation.payload_release_or_delivery {
        required.insert(ApprovalRequirement::DirectorApprovedPayloadReleaseInOperationsManual);
    }
    if operation.operating_visibility == OperatingVisibility::BeyondVisualLineOfSight {
        required.insert(ApprovalRequirement::DirectorApprovedBvlosInOperationsManual);
        required.insert(ApprovalRequirement::SaCats101BvlosEvidence);
    }
    if operation.multi_uas_operation {
        required.insert(ApprovalRequirement::DirectorApprovedMultiUasOperation);
        required.insert(ApprovalRequirement::ApprovedSingleRpsCommandInputArrangement);
    }
    if operation.controlled_airspace {
        required.insert(ApprovalRequirement::DirectorApprovedControlledAirspaceOperation);
    }
    required
}

fn evaluate_operation(
    operation: &ZaUasOperationProfile,
    source_snapshot_id: &str,
) -> ZaUasOperationResult {
    let mut blocked = Vec::new();
    let mut indeterminate = Vec::new();
    match operation.aircraft_control_mode {
        AircraftControlMode::Autonomous => blocked.push(
            "autonomous_aircraft_applicability_is_unsupported_pending_written_competent_authority_determination".into(),
        ),
        AircraftControlMode::NotManagedInRealTime | AircraftControlMode::OtherOrUnclassified => {
            blocked.push("aircraft_is_not_a_supported_remotely_piloted_class_1_or_class_2_case".into())
        }
        AircraftControlMode::RemotelyPiloted => {}
    }

    match operation.aircraft_class {
        AircraftClass::Unknown => {
            indeterminate.push("aircraft_class_is_unknown".into());
        }
        AircraftClass::OutsidePart101Class1Or2 => {
            blocked.push("aircraft_class_is_outside_part101_class_1_or_2_v0_1_scope".into());
        }
        class if !class.within_part101_class_1_or_2() => {
            blocked.push("aircraft_class_is_not_part101_class_1_or_class_2".into());
        }
        _ => {}
    }

    if operation.operating_visibility == OperatingVisibility::Unknown {
        indeterminate.push("operating_visibility_classification_is_unknown".into());
    }
    if operation.operating_visibility == OperatingVisibility::BeyondRadioLineOfSight {
        blocked.push("brlos_is_unsupported_by_the_current_v0_1_south_african_source_snapshot".into());
    }

    if operation.activity_class == ActivityClass::Private {
        if !operation.aircraft_class.private_operation_class() {
            blocked.push("private_operations_are_limited_to_class_1a_or_1b".into());
        }
        if operation.operating_visibility != OperatingVisibility::RestrictedVisualLineOfSight {
            blocked.push("private_operations_are_limited_to_restricted_visual_line_of_sight".into());
        }
        if operation.payload_release_or_delivery {
            blocked.push(
                "private_operation_does_not_satisfy_the_uasoc_and_director_approved_object_release_gate".into(),
            );
        }
        if operation.dangerous_goods_payload {
            blocked.push("private_operations_must_not_carry_dangerous_goods".into());
        }
        if operation.multi_uas_operation {
            blocked.push("private_operation_multi_uas_is_outside_the_v0_1_qualified_scope".into());
        }
        if operation.operating_visibility == OperatingVisibility::BeyondVisualLineOfSight {
            blocked.push("private_bvlos_is_not_supported_by_the_v0_1_private_operation_profile".into());
        }
    } else if operation.dangerous_goods_payload {
        // Part 92 and dangerous-goods applicability is intentionally not
        // implemented by this Part 101 applicability screen.
        indeterminate.push("dangerous_goods_requires_a_separate_part_92_applicability_gate".into());
    }

    let required = required_requirements(operation);
    let evidence_by_requirement: BTreeMap<ApprovalRequirement, &ApprovalEvidence> =
        operation.approval_evidence.iter()
            .map(|evidence| (evidence.requirement, evidence))
            .collect();
    let mut required_evidence = Vec::new();

    for requirement in required {
        let Some(evidence) = evidence_by_requirement.get(&requirement).copied() else {
            required_evidence.push(RequiredEvidenceResult {
                requirement,
                state: EvidenceState::Missing,
                evidence_record_id: None,
                source_reference: None,
                reason: "required approval evidence is absent".into(),
            });
            indeterminate.push(format!("missing_required_evidence:{requirement:?}"));
            continue;
        };

        let mut reason = String::new();
        let mut state = evidence.state;
        if state == EvidenceState::CallerVerifiedCurrent {
            if evidence.aircraft_serial_digest != operation.aircraft_serial_digest
                || evidence.operator_identity_digest != operation.operator_identity_digest
                || evidence.operation_scope_digest != operation.operation_scope_digest
            {
                state = EvidenceState::Mismatched;
                reason = "evidence subject/operator/operation scope does not match".into();
            } else if evidence.valid_from_unix_ms > operation.assessment_time_unix_ms {
                state = EvidenceState::Unknown;
                reason = "evidence is not yet effective at assessment time".into();
            } else if evidence.valid_until_unix_ms <= operation.assessment_time_unix_ms {
                state = EvidenceState::Expired;
                reason = "evidence validity ended before assessment time".into();
            } else if requirement.requires_operations_manual_binding()
                && evidence.operations_manual_revision_digest.as_deref()
                    != operation.operations_manual_revision_digest.as_deref()
            {
                state = EvidenceState::OutOfScope;
                reason = "approval evidence does not bind the declared Operations Manual revision".into();
            } else if requirement.requires_operations_manual_binding()
                && operation.operations_manual_revision_digest.is_none()
            {
                state = EvidenceState::Missing;
                reason = "operation does not declare the Operations Manual revision".into();
            } else {
                reason = "caller asserts current verified evidence; authenticity is not checked here".into();
            }
        }
        match state {
            EvidenceState::CallerVerifiedCurrent => {}
            EvidenceState::Revoked | EvidenceState::Expired | EvidenceState::Mismatched
            | EvidenceState::OutOfScope => {
                blocked.push(format!("approval_not_usable:{requirement:?}:{state:?}"));
            }
            EvidenceState::Missing | EvidenceState::Unknown | EvidenceState::Stale => {
                indeterminate.push(format!("approval_not_current:{requirement:?}:{state:?}"));
            }
        }
        required_evidence.push(RequiredEvidenceResult {
            requirement,
            state,
            evidence_record_id: Some(evidence.evidence_record_id.clone()),
            source_reference: Some(evidence.source_reference.clone()),
            reason,
        });
    }

    blocked.sort();
    blocked.dedup();
    indeterminate.sort();
    indeterminate.dedup();
    let candidate_status = if !blocked.is_empty() {
        CandidateStatus::Blocked
    } else if !indeterminate.is_empty() {
        CandidateStatus::Indeterminate
    } else {
        CandidateStatus::EligibleForExternalOperatorReview
    };

    ZaUasOperationResult {
        operation_id: operation.operation_id.clone(),
        candidate_status,
        required_evidence,
        blocked_reasons: blocked,
        indeterminate_reasons: indeterminate,
        source_snapshot_id: source_snapshot_id.into(),
        claim_ceiling: "Structural South African Part 101 applicability/evidence-package screen only; caller-adjudicated evidence; not legal advice, SACAA approval, airspace clearance, flight authorization or dispatch".into(),
        flight_authorized: false,
        dispatch_authorized: false,
        dispatch_commands_emitted: false,
    }
}

/// Evaluate a source-versioned set of candidate South African UAS operations.
/// A passing result is only eligible for external operator/regulator review.
pub fn evaluate_za_uas_corpus(
    corpus: &ZaUasEligibilityCorpus,
) -> Result<ZaUasEligibilityReport, Vec<String>> {
    validate_corpus(corpus)?;
    let input_digest_sha256 =
        canonical_corpus_digest(corpus).map_err(|error| vec![error])?;
    let mut operations = corpus.operations.clone();
    operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
    let results: Vec<ZaUasOperationResult> = operations
        .iter()
        .map(|operation| evaluate_operation(operation, &corpus.source_snapshot_id))
        .collect();
    let blocked_count = results.iter()
        .filter(|result| result.candidate_status == CandidateStatus::Blocked)
        .count();
    let indeterminate_count = results.iter()
        .filter(|result| result.candidate_status == CandidateStatus::Indeterminate)
        .count();
    let eligible_count = results.iter()
        .filter(|result| result.candidate_status == CandidateStatus::EligibleForExternalOperatorReview)
        .count();

    Ok(ZaUasEligibilityReport {
        schema_version: ZA_UAS_GATE_SCHEMA_VERSION,
        benchmark_id: ZA_UAS_GATE_BENCHMARK_ID.into(),
        source_snapshot_id: corpus.source_snapshot_id.clone(),
        source_snapshot_as_of: corpus.source_snapshot_as_of.clone(),
        input_digest_sha256,
        operation_count: results.len(),
        blocked_count,
        indeterminate_count,
        eligible_for_external_operator_review_count: eligible_count,
        results,
        flight_authorization_granted: false,
        dispatch_commands_emitted: false,
        claim_ceiling: "Synthetic/caller-supplied applicability and evidence gate only. EligibleForExternalOperatorReview is not permission to operate; all real approvals and their authenticity/scope must be verified by the accountable operator and competent authority.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AIRCRAFT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const OPERATOR: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const SCOPE: &str = "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const MANUAL: &str = "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn evidence(requirement: ApprovalRequirement) -> ApprovalEvidence {
        ApprovalEvidence {
            requirement,
            state: EvidenceState::CallerVerifiedCurrent,
            evidence_record_id: format!("evidence-{requirement:?}"),
            source_reference: format!("synthetic://approval/{requirement:?}"),
            source_version: "synthetic-rule-profile-2026-11-20".into(),
            aircraft_serial_digest: AIRCRAFT.into(),
            operator_identity_digest: OPERATOR.into(),
            operation_scope_digest: SCOPE.into(),
            operations_manual_revision_digest: if requirement.requires_operations_manual_binding() {
                Some(MANUAL.into())
            } else {
                None
            },
            valid_from_unix_ms: 1_000,
            valid_until_unix_ms: 9_999,
        }
    }

    fn base_operation() -> ZaUasOperationProfile {
        ZaUasOperationProfile {
            operation_id: "commercial-rvlos-delivery".into(),
            activity_class: ActivityClass::Commercial,
            aircraft_control_mode: AircraftControlMode::RemotelyPiloted,
            aircraft_class: AircraftClass::Class2,
            operating_visibility: OperatingVisibility::RestrictedVisualLineOfSight,
            payload_release_or_delivery: true,
            dangerous_goods_payload: false,
            multi_uas_operation: false,
            controlled_airspace: false,
            aircraft_serial_digest: AIRCRAFT.into(),
            operator_identity_digest: OPERATOR.into(),
            operation_scope_digest: SCOPE.into(),
            operations_manual_revision_digest: Some(MANUAL.into()),
            assessment_time_unix_ms: 5_000,
            approval_evidence: vec![
                evidence(ApprovalRequirement::UasOperatingCertificate),
                evidence(ApprovalRequirement::AirServiceLicence),
                evidence(ApprovalRequirement::RemotePilotLicence),
                evidence(ApprovalRequirement::RemoteOperatorLetterOfApproval),
                evidence(ApprovalRequirement::CertificateOfRegistration),
                evidence(ApprovalRequirement::DirectorApprovedPayloadReleaseInOperationsManual),
            ],
        }
    }

    fn corpus(operations: Vec<ZaUasOperationProfile>) -> ZaUasEligibilityCorpus {
        ZaUasEligibilityCorpus {
            schema_version: 1,
            benchmark_id: ZA_UAS_GATE_BENCHMARK_ID.into(),
            source_snapshot_id: "za-sacaa-public-source-snapshot-2026-11-20".into(),
            source_snapshot_as_of: "2026-11-20".into(),
            claim_ceiling: "synthetic fixture".into(),
            operations,
        }
    }

    #[test]
    fn complete_commercial_delivery_is_only_eligible_for_external_review() {
        let report = evaluate_za_uas_corpus(&corpus(vec![base_operation()]))
            .expect("valid corpus");
        assert_eq!(
            report.results[0].candidate_status,
            CandidateStatus::EligibleForExternalOperatorReview
        );
        assert!(!report.flight_authorization_granted);
        assert!(!report.results[0].flight_authorized);
        assert!(!report.results[0].dispatch_authorized);
        assert!(!report.dispatch_commands_emitted);
    }

    #[test]
    fn commercial_operation_requires_asl_uasoc_rla_rpl_and_registration() {
        let mut operation = base_operation();
        operation.approval_evidence.retain(|item| {
            item.requirement != ApprovalRequirement::AirServiceLicence
        });
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
        assert!(report.results[0].indeterminate_reasons.iter().any(|reason| {
            reason.contains("AirServiceLicence")
        }));
    }

    #[test]
    fn corporate_and_nonprofit_do_not_inherit_commercial_asl_requirement() {
        for activity in [ActivityClass::Corporate, ActivityClass::NonProfit] {
            let mut operation = base_operation();
            operation.activity_class = activity;
            operation.approval_evidence.retain(|item| {
                item.requirement != ApprovalRequirement::AirServiceLicence
            });
            let report = evaluate_za_uas_corpus(&corpus(vec![operation]))
                .expect("valid corpus");
            assert_eq!(
                report.results[0].candidate_status,
                CandidateStatus::EligibleForExternalOperatorReview
            );
        }
    }

    #[test]
    fn autonomous_aircraft_is_blocked_not_treated_as_automatically_exempt() {
        let mut operation = base_operation();
        operation.aircraft_control_mode = AircraftControlMode::Autonomous;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);
        assert!(report.results[0].blocked_reasons.iter().any(|reason| {
            reason.contains("autonomous_aircraft_applicability_is_unsupported")
        }));
    }

    #[test]
    fn brlos_is_blocked_under_the_current_v0_1_source_snapshot() {
        let mut operation = base_operation();
        operation.operating_visibility = OperatingVisibility::BeyondRadioLineOfSight;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);
    }

    #[test]
    fn bvlos_requires_director_approval_and_sa_cats_101_evidence() {
        let mut operation = base_operation();
        operation.operating_visibility = OperatingVisibility::BeyondVisualLineOfSight;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation.clone()]))
            .expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
        operation.approval_evidence.push(evidence(
            ApprovalRequirement::DirectorApprovedBvlosInOperationsManual,
        ));
        operation.approval_evidence.push(evidence(
            ApprovalRequirement::SaCats101BvlosEvidence,
        ));
        let report = evaluate_za_uas_corpus(&corpus(vec![operation]))
            .expect("complete BVLOS evidence package");
        assert_eq!(
            report.results[0].candidate_status,
            CandidateStatus::EligibleForExternalOperatorReview
        );
    }

    #[test]
    fn multi_uas_requires_director_approval_and_single_rps_command_input_arrangement() {
        let mut operation = base_operation();
        operation.multi_uas_operation = true;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation.clone()]))
            .expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
        operation.approval_evidence.push(evidence(
            ApprovalRequirement::DirectorApprovedMultiUasOperation,
        ));
        operation.approval_evidence.push(evidence(
            ApprovalRequirement::ApprovedSingleRpsCommandInputArrangement,
        ));
        let report = evaluate_za_uas_corpus(&corpus(vec![operation]))
            .expect("complete multi-UAS evidence package");
        assert_eq!(
            report.results[0].candidate_status,
            CandidateStatus::EligibleForExternalOperatorReview
        );
    }

    #[test]
    fn payload_release_requires_director_approved_operations_manual_scope() {
        let mut operation = base_operation();
        operation.approval_evidence.retain(|item| {
            item.requirement
                != ApprovalRequirement::DirectorApprovedPayloadReleaseInOperationsManual
        });
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
    }

    #[test]
    fn private_delivery_is_blocked_even_when_other_fields_look_complete() {
        let mut operation = base_operation();
        operation.activity_class = ActivityClass::Private;
        operation.aircraft_class = AircraftClass::Class1A;
        operation.operating_visibility = OperatingVisibility::RestrictedVisualLineOfSight;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);
        assert!(report.results[0].blocked_reasons.iter().any(|reason| {
            reason.contains("private_operation_does_not_satisfy")
        }));
    }

    #[test]
    fn private_non_delivery_requires_class_1a_or_1b_and_rvlos() {
        let mut operation = base_operation();
        operation.activity_class = ActivityClass::Private;
        operation.aircraft_class = AircraftClass::Class1A;
        operation.operating_visibility = OperatingVisibility::RestrictedVisualLineOfSight;
        operation.payload_release_or_delivery = false;
        operation.dangerous_goods_payload = false;
        operation.approval_evidence.clear();
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("private R-VLOS");
        assert_eq!(
            report.results[0].candidate_status,
            CandidateStatus::EligibleForExternalOperatorReview
        );
    }

    #[test]
    fn expired_revoked_or_mismatched_evidence_never_passes() {
        for state in [
            EvidenceState::Expired,
            EvidenceState::Revoked,
            EvidenceState::Mismatched,
            EvidenceState::OutOfScope,
        ] {
            let mut operation = base_operation();
            operation.approval_evidence[0].state = state;
            let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
            assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);
        }
    }

    #[test]
    fn evidence_subject_scope_and_manual_revision_must_match() {
        let mut operation = base_operation();
        operation.approval_evidence[0].aircraft_serial_digest = OPERATOR.into();
        let report = evaluate_za_uas_corpus(&corpus(vec![operation.clone()]))
            .expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);

        let mut operation = base_operation();
        operation.approval_evidence[5].operations_manual_revision_digest = Some(SCOPE.into());
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Blocked);
    }

    #[test]
    fn stale_or_unknown_evidence_is_indeterminate_and_not_a_pass() {
        let mut operation = base_operation();
        operation.approval_evidence[0].state = EvidenceState::Stale;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
    }

    #[test]
    fn source_reference_alone_is_not_evidence() {
        let mut operation = base_operation();
        operation.approval_evidence[0].state = EvidenceState::Unknown;
        let report = evaluate_za_uas_corpus(&corpus(vec![operation])).expect("valid corpus");
        assert_eq!(report.results[0].candidate_status, CandidateStatus::Indeterminate);
    }

    #[test]
    fn input_order_permutations_produce_identical_report() {
        let first = corpus(vec![base_operation()]);
        let a = evaluate_za_uas_corpus(&first).expect("first report");
        let mut second = first.clone();
        second.operations[0].approval_evidence.reverse();
        let b = evaluate_za_uas_corpus(&second).expect("permuted report");
        assert_eq!(a, b);
    }

    #[test]
    fn rejects_duplicate_approval_requirement() {
        let mut operation = base_operation();
        operation.approval_evidence.push(operation.approval_evidence[0].clone());
        assert!(evaluate_za_uas_corpus(&corpus(vec![operation])).is_err());
    }

    #[test]
    fn rejects_malformed_scope_digest_and_missing_source_reference() {
        let mut operation = base_operation();
        operation.operation_scope_digest = "not-a-digest".into();
        assert!(evaluate_za_uas_corpus(&corpus(vec![operation])).is_err());
        let mut operation = base_operation();
        operation.approval_evidence[0].source_reference.clear();
        assert!(evaluate_za_uas_corpus(&corpus(vec![operation])).is_err());
    }
}
