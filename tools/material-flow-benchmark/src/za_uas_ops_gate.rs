// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! South African Part 101 applicability/evidence gate for delivery-UAS scenarios.
//!
//! This is a source-snapshot-bound contract evaluator. It never grants flight
//! authority, authenticates an approval document, or emits dispatch commands.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ZA_UAS_GATE_SCHEMA_VERSION: u16 = 1;
pub const ZA_UAS_SOURCE_SNAPSHOT_ID: &str = "ZA-SACAA-PART101-2026-11-20";
pub const ZA_UAS_MAX_CASES: usize = 64;
pub const ZA_UAS_MAX_SOURCE_REFERENCES: usize = 32;

const CLAIM_CEILING: &str = "Source-snapshot-bound South African RPAS/UAS applicability and evidence-gate test only; not legal advice, an SACAA determination, approval authenticity verification, airspace clearance, flight authorization, dispatch, airworthiness or evidence of safe operation.";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    AuthorityRecord,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    FreshInScope,
    Unknown,
    Stale,
    Missing,
    Expired,
    Revoked,
    OutOfScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityClass {
    Commercial,
    Corporate,
    NonProfit,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisualFlightMode {
    RestrictedVlos,
    Vlos,
    Evlos,
    Bvlos,
    Brlos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UasClass {
    Class1A,
    Class1B,
    Class2,
    Class3OrHigher,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateState {
    Pass,
    SyntheticOnly,
    Blocked,
    Indeterminate,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityStatus {
    EligibleForExternalOperatorReview,
    PassesSyntheticGateOnly,
    Blocked,
    Indeterminate,
    UnsupportedBySnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalEvidence {
    pub evidence_class: EvidenceClass,
    pub status: EvidenceStatus,
    pub record_id: String,
    pub authority: String,
    pub subject_id: String,
    pub operation_scope_digest_sha256: String,
    pub source_reference: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorBinding {
    pub operator_id: String,
    pub activity_class: ActivityClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AircraftBinding {
    pub aircraft_id: String,
    pub configuration_id: String,
    pub uas_class: UasClass,
    /// Part 101's applicability page lists autonomous unmanned aircraft as excluded.
    /// That exclusion is not treated as permission by this Part 101 gate.
    pub autonomous_control: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PilotBinding {
    pub pilot_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationConditions {
    pub operation_id: String,
    pub operation_scope_digest_sha256: String,
    pub visual_flight_mode: VisualFlightMode,
    /// Whether a current, approved command-and-control radio link is available.
    /// BVLOS may still be RLOS; BRLOS is a separate unsupported state in v0.1.
    pub rlos_command_link_available: bool,
    pub rlos_command_link_evidence_reference: String,
    pub payload_release_or_delivery: bool,
    pub multi_uas_operation: bool,
    pub public_road_takeoff_or_landing: bool,
    pub controlled_airspace: bool,
    pub restricted_or_prohibited_airspace: bool,
    pub within_ten_km_of_aerodrome: bool,
    pub property_owner_permission_required: bool,
    pub dangerous_goods_payload: bool,
    pub near_manned_aircraft: bool,
    pub over_or_within_fifty_m_of_public_road: bool,
    pub over_public_road: bool,
    pub rps_id: String,
    pub operations_manual_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalSet {
    pub air_service_licence: Option<ApprovalEvidence>,
    pub uas_operator_certificate: Option<ApprovalEvidence>,
    pub aircraft_letter_of_approval: Option<ApprovalEvidence>,
    pub remote_pilot_licence: Option<ApprovalEvidence>,
    pub aircraft_certificate_of_registration: Option<ApprovalEvidence>,
    pub operations_manual_scope: Option<ApprovalEvidence>,
    pub director_payload_release_approval: Option<ApprovalEvidence>,
    pub director_bvlos_approval: Option<ApprovalEvidence>,
    pub sa_cats_101_bvlos_compliance: Option<ApprovalEvidence>,
    pub sa_cats_101_c2_compliance: Option<ApprovalEvidence>,
    pub director_class_3_applicability: Option<ApprovalEvidence>,
    pub director_multi_uas_approval: Option<ApprovalEvidence>,
    pub director_rps_topology_approval: Option<ApprovalEvidence>,
    pub director_public_road_approval: Option<ApprovalEvidence>,
    pub local_authority_public_road_approval: Option<ApprovalEvidence>,
    pub airspace_approval: Option<ApprovalEvidence>,
    pub property_owner_permission: Option<ApprovalEvidence>,
    pub dangerous_goods_director_approval: Option<ApprovalEvidence>,
    pub part_92_dangerous_goods_compliance: Option<ApprovalEvidence>,
    pub director_public_road_overflight_approval: Option<ApprovalEvidence>,
    pub public_road_closure_evidence: Option<ApprovalEvidence>,
    pub public_road_safety_controls: Option<ApprovalEvidence>,
    /// The operation manual's declared multi-UAS control method.
    pub multi_uas_uses_command_inputs_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UasOperationCase {
    pub case_id: String,
    pub assessment_time_utc: String,
    pub operator: OperatorBinding,
    pub aircraft: AircraftBinding,
    pub pilot: PilotBinding,
    pub operation: OperationConditions,
    pub approvals: ApprovalSet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UasEligibilityBatch {
    pub schema_version: u16,
    pub source_snapshot_id: String,
    pub source_references: Vec<String>,
    pub cases: Vec<UasOperationCase>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GateFinding {
    pub gate_id: String,
    pub state: GateState,
    pub requirement: String,
    pub reason: String,
    pub evidence_record_id: Option<String>,
    pub evidence_source_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UasCaseReport {
    pub case_id: String,
    pub eligibility_status: EligibilityStatus,
    pub eligible_for_external_operator_review: bool,
    pub flight_authorized: bool,
    pub dispatch_command_emitted: bool,
    pub gate_count: usize,
    pub passed_gate_count: usize,
    pub synthetic_only_gate_count: usize,
    pub blocked_gate_count: usize,
    pub indeterminate_gate_count: usize,
    pub unsupported_gate_count: usize,
    pub findings: Vec<GateFinding>,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UasEligibilityReport {
    pub schema_version: u16,
    pub source_snapshot_id: String,
    pub input_digest_sha256: String,
    pub case_count: usize,
    pub cases: Vec<UasCaseReport>,
    pub flight_authority_granted: bool,
    pub dispatch_commands_emitted: bool,
    pub claim_ceiling: &'static str,
}

fn identifier_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'-' | b'_' | b'.' | b':')
        })
}

fn digest_valid(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_batch(batch: &UasEligibilityBatch) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    if batch.schema_version != ZA_UAS_GATE_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version: expected {}, got {}",
            ZA_UAS_GATE_SCHEMA_VERSION, batch.schema_version
        ));
    }
    if batch.source_snapshot_id != ZA_UAS_SOURCE_SNAPSHOT_ID {
        errors.push("source_snapshot_id: unsupported South African regulatory snapshot".into());
    }
    if batch.source_references.is_empty()
        || batch.source_references.len() > ZA_UAS_MAX_SOURCE_REFERENCES
        || batch.source_references.iter().any(|reference| reference.trim().is_empty())
    {
        errors.push("source_references: expected 1..=32 non-empty source references".into());
    }
    if batch.cases.is_empty() || batch.cases.len() > ZA_UAS_MAX_CASES {
        errors.push(format!("cases: expected 1..={ZA_UAS_MAX_CASES}"));
    }
    let mut case_ids = BTreeSet::new();
    for (index, case) in batch.cases.iter().enumerate() {
        if !identifier_valid(&case.case_id) || !case_ids.insert(case.case_id.as_str()) {
            errors.push(format!("cases[{index}].case_id: unique opaque identity required"));
        }
        if case.operation.rlos_command_link_evidence_reference.trim().is_empty() {
            errors.push(format!("cases[{index}].operation.rlos_command_link_evidence_reference: required"));
        }
        for (field, value) in [
            ("operator.operator_id", case.operator.operator_id.as_str()),
            ("aircraft.aircraft_id", case.aircraft.aircraft_id.as_str()),
            ("aircraft.configuration_id", case.aircraft.configuration_id.as_str()),
            ("pilot.pilot_id", case.pilot.pilot_id.as_str()),
            ("operation.operation_id", case.operation.operation_id.as_str()),
            ("operation.rps_id", case.operation.rps_id.as_str()),
            ("operation.operations_manual_revision", case.operation.operations_manual_revision.as_str()),
        ] {
            if !identifier_valid(value) {
                errors.push(format!("cases[{index}].{field}: invalid opaque identifier"));
            }
        }
        if !digest_valid(&case.operation.operation_scope_digest_sha256) {
            errors.push(format!("cases[{index}].operation.operation_scope_digest_sha256: invalid SHA-256 digest"));
        }
        if case.operation.over_public_road && !case.operation.over_or_within_fifty_m_of_public_road {
        errors.push(format!("cases[{index}].operation.over_public_road: implies road proximity condition"));
    }
    if case.assessment_time_utc.trim().is_empty() {
            errors.push(format!("cases[{index}].assessment_time_utc: required"));
        }
        if case.operation.payload_release_or_delivery
            && case.operator.activity_class == ActivityClass::Private
        {
            // This is a known private-operation prohibition in SACAA's published
            // private-use guidance, not an assumption that a private exception
            // can be used for commercial delivery.
        }
    }
    if errors.is_empty() { Ok(()) } else { Err(errors) }
}

fn evidence_gate(
    gate_id: &str,
    requirement: &str,
    evidence: Option<&ApprovalEvidence>,
    expected_subject_id: &str,
    expected_scope_digest: &str,
) -> GateFinding {
    let Some(evidence) = evidence else {
        return GateFinding {
            gate_id: gate_id.into(),
            state: GateState::Indeterminate,
            requirement: requirement.into(),
            reason: "required approval evidence is absent".into(),
            evidence_record_id: None,
            evidence_source_reference: None,
        };
    };

    let mut reason = String::new();
    let mut state = match evidence.status {
        EvidenceStatus::FreshInScope => GateState::Pass,
        EvidenceStatus::Unknown | EvidenceStatus::Stale | EvidenceStatus::Missing => {
            reason = "approval evidence is unknown, stale or missing".into();
            GateState::Indeterminate
        }
        EvidenceStatus::Expired | EvidenceStatus::Revoked | EvidenceStatus::OutOfScope => {
            reason = "approval is expired, revoked or outside the declared scope".into();
            GateState::Blocked
        }
    };

    if evidence.record_id.trim().is_empty()
        || evidence.authority.trim().is_empty()
        || evidence.source_reference.trim().is_empty()
    {
        state = GateState::Indeterminate;
        reason = "approval receipt lacks record, issuer or source reference".into();
    } else if evidence.subject_id != expected_subject_id {
        state = GateState::Blocked;
        reason = "approval subject does not match the declared operator, aircraft, pilot or RPS".into();
    } else if evidence.operation_scope_digest_sha256 != expected_scope_digest {
        state = GateState::Blocked;
        reason = "approval scope digest does not match this operation configuration".into();
    } else if state == GateState::Pass && evidence.evidence_class == EvidenceClass::Synthetic {
        state = GateState::SyntheticOnly;
        reason = "synthetic approval record exercises the gate but is not an authority record".into();
    } else if state == GateState::Pass && evidence.evidence_class == EvidenceClass::Unknown {
        state = GateState::Indeterminate;
        reason = "approval evidence class is unknown".into();
    } else if state == GateState::Pass {
        reason = "current in-scope authority record declared; authenticity is not cryptographically verified here".into();
    }

    GateFinding {
        gate_id: gate_id.into(),
        state,
        requirement: requirement.into(),
        reason,
        evidence_record_id: Some(evidence.record_id.clone()),
        evidence_source_reference: Some(evidence.source_reference.clone()),
    }
}

fn boolean_gate(
    gate_id: &str,
    requirement: &str,
    value: Option<bool>,
) -> GateFinding {
    let (state, reason) = match value {
        Some(true) => (GateState::Pass, "declared condition is satisfied".into()),
        Some(false) => (GateState::Blocked, "declared condition is not satisfied".into()),
        None => (GateState::Indeterminate, "required condition is unknown".into()),
    };
    GateFinding {
        gate_id: gate_id.into(),
        state,
        requirement: requirement.into(),
        reason,
        evidence_record_id: None,
        evidence_source_reference: None,
    }
}

fn boolean_gate_with_reference(
    gate_id: &str,
    requirement: &str,
    value: Option<bool>,
    source_reference: &str,
) -> GateFinding {
    let mut finding = boolean_gate(gate_id, requirement, value);
    finding.evidence_source_reference = Some(source_reference.to_string());
    finding
}

fn evaluate_case(case: &UasOperationCase) -> UasCaseReport {
    let mut findings = Vec::new();
    let scope = &case.operation.operation_scope_digest_sha256;
    let operator_id = &case.operator.operator_id;
    let aircraft_id = &case.aircraft.aircraft_id;
    let pilot_id = &case.pilot.pilot_id;
    let rps_id = &case.operation.rps_id;
    let approvals = &case.approvals;
    let activity = case.operator.activity_class;

    // Base approvals from SACAA's published Part 101 operator matrix.
    if activity == ActivityClass::Commercial {
        findings.push(evidence_gate(
            "air_service_licence",
            "Commercial activity: current Air Service Licence for this operator",
            approvals.air_service_licence.as_ref(),
            operator_id,
            scope,
        ));
    }
    if activity != ActivityClass::Private {
        findings.push(evidence_gate(
            "uas_operator_certificate",
            "UASOC held by the operating entity",
            approvals.uas_operator_certificate.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "aircraft_letter_of_approval",
            "RLA covering this aircraft/configuration and operation",
            approvals.aircraft_letter_of_approval.as_ref(),
            aircraft_id,
            scope,
        ));
        findings.push(evidence_gate(
            "remote_pilot_licence",
            "RPL for the responsible remote pilot",
            approvals.remote_pilot_licence.as_ref(),
            pilot_id,
            scope,
        ));
        findings.push(evidence_gate(
            "aircraft_certificate_of_registration",
            "Certificate of Registration for the exact aircraft",
            approvals.aircraft_certificate_of_registration.as_ref(),
            aircraft_id,
            scope,
        ));
        findings.push(evidence_gate(
            "operations_manual_scope",
            "Operations Manual revision covering this operation and aircraft",
            approvals.operations_manual_scope.as_ref(),
            operator_id,
            scope,
        ));
    } else {
        if case.operation.visual_flight_mode != VisualFlightMode::RestrictedVlos {
            findings.push(boolean_gate(
                "private_r_vlos_only",
                "Private operation limited to R-VLOS under the current private-use profile",
                Some(false),
            ));
        } else {
            findings.push(boolean_gate(
                "private_r_vlos_only",
                "Private operation limited to R-VLOS under the current private-use profile",
                Some(true),
            ));
        }
        findings.push(boolean_gate(
            "private_class_1a_or_1b",
            "Private operation restricted to Class 1A or Class 1B",
            Some(matches!(case.aircraft.uas_class, UasClass::Class1A | UasClass::Class1B)),
        ));
        if case.operation.payload_release_or_delivery {
            findings.push(boolean_gate(
                "private_payload_release_prohibited",
                "Private-use profile does not permit payload release/delivery",
                Some(false),
            ));
        }
        if case.operation.dangerous_goods_payload {
            findings.push(boolean_gate(
                "private_dangerous_goods_prohibited",
                "Private-use profile does not permit dangerous goods",
                Some(false),
            ));
        }
    }

    // Part 101 applies to RPAS under its scope; autonomous aircraft are
    // explicitly outside this profile and cannot pass by virtue of that exclusion.
    if case.aircraft.autonomous_control {
        findings.push(GateFinding {
            gate_id: "autonomous_aircraft_applicability".into(),
            state: GateState::Unsupported,
            requirement: "Written competent-authority determination of the applicable regime for autonomous UAS".into(),
            reason: "autonomous unmanned aircraft are excluded from the listed Part 101 applicability; exclusion is not an operating permission".into(),
            evidence_record_id: None,
            evidence_source_reference: None,
        });
    }

    // Current SACAA guidance distinguishes visual mode from the radio link.
    findings.push(boolean_gate_with_reference(
        "radio_line_of_sight_command_link",
        "Current command-and-control radio link is available; RLOS is distinct from VLOS",
        Some(case.operation.rlos_command_link_available),
        &case.operation.rlos_command_link_evidence_reference,
    ));
    if case.operation.visual_flight_mode == VisualFlightMode::Brlos {
        findings.push(GateFinding {
            gate_id: "beyond_radio_line_of_sight".into(),
            state: GateState::Unsupported,
            requirement: "BRLOS operating framework under a future/express SACAA approval".into(),
            reason: "SACAA's current published UAS page states operations are limited to RLOS and reserves BRLOS for future use".into(),
            evidence_record_id: None,
            evidence_source_reference: None,
        });
    }
    if activity != ActivityClass::Private {
        findings.push(evidence_gate(
            "sa_cats_101_c2_compliance",
            "C2 operational compliance with SA-CATS 101",
            approvals.sa_cats_101_c2_compliance.as_ref(),
            aircraft_id,
            scope,
        ));
    }

    if case.aircraft.uas_class == UasClass::Unknown {
        findings.push(GateFinding {
            gate_id: "aircraft_classification".into(),
            state: GateState::Indeterminate,
            requirement: "UAS class determined using the current SACAA mass/impact-energy/height/flight-rule classification".into(),
            reason: "aircraft classification is unknown".into(),
            evidence_record_id: None,
            evidence_source_reference: None,
        });
    }

    if case.aircraft.uas_class == UasClass::Class3OrHigher {
        findings.push(evidence_gate(
            "director_class_3_applicability",
            "Director determination that this Part 101 profile applies to this Class 3+ aircraft",
            approvals.director_class_3_applicability.as_ref(),
            aircraft_id,
            scope,
        ));
    }

    if case.operation.visual_flight_mode == VisualFlightMode::Bvlos {
        findings.push(evidence_gate(
            "director_bvlos_approval",
            "BVLOS approval in the UASOC Operations Manual by the Director",
            approvals.director_bvlos_approval.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "sa_cats_101_bvlos_compliance",
            "Evidence of compliance with the applicable SA-CATS 101 BVLOS conditions",
            approvals.sa_cats_101_bvlos_compliance.as_ref(),
            operator_id,
            scope,
        ));
    }

    if case.operation.payload_release_or_delivery {
        findings.push(evidence_gate(
            "director_payload_release_approval",
            "Payload release/delivery explicitly approved by the Director in the UASOC Operations Manual",
            approvals.director_payload_release_approval.as_ref(),
            operator_id,
            scope,
        ));
    }

    if case.operation.multi_uas_operation {
        findings.push(evidence_gate(
            "director_multi_uas_approval",
            "Multi-UAS operation approved by the Director for this UASOC holder",
            approvals.director_multi_uas_approval.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "director_rps_topology_approval",
            "RPS control arrangement approved by the Director",
            approvals.director_rps_topology_approval.as_ref(),
            rps_id,
            scope,
        ));
        findings.push(boolean_gate(
            "multi_uas_command_input_only",
            "Multi-UAS operation uses command inputs only as declared in the approved control arrangement",
            approvals.multi_uas_uses_command_inputs_only,
        ));
    }

    if case.operation.over_or_within_fifty_m_of_public_road {
        findings.push(evidence_gate(
            "director_public_road_overflight_approval",
            "Director approval for operation over, along or within 50 m of a public road",
            approvals.director_public_road_overflight_approval.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "public_road_safety_controls",
            "Evidence of reasonable care for road users and pedestrians in loss-of-control scenarios",
            approvals.public_road_safety_controls.as_ref(),
            operator_id,
            scope,
        ));
        if case.operation.over_public_road {
            findings.push(evidence_gate(
                "public_road_closure_evidence",
                "Evidence that the public road is closed for operations over the road",
                approvals.public_road_closure_evidence.as_ref(),
                operator_id,
                scope,
            ));
        }
    }

    if case.operation.public_road_takeoff_or_landing {
        findings.push(evidence_gate(
            "director_public_road_approval",
            "Director/Operations Manual approval for public-road takeoff or landing",
            approvals.director_public_road_approval.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "local_authority_public_road_approval",
            "Relevant local-authority approval for public-road takeoff or landing",
            approvals.local_authority_public_road_approval.as_ref(),
            operator_id,
            scope,
        ));
    }

    if case.operation.controlled_airspace
        || case.operation.restricted_or_prohibited_airspace
        || case.operation.within_ten_km_of_aerodrome
        || case.operation.near_manned_aircraft
    {
        findings.push(evidence_gate(
            "airspace_approval",
            "Applicable SACAA airspace/site approval for the declared route and location",
            approvals.airspace_approval.as_ref(),
            operator_id,
            scope,
        ));
    }

    if case.operation.property_owner_permission_required {
        findings.push(evidence_gate(
            "property_owner_permission",
            "Property-owner permission for the declared takeoff/landing/operation area",
            approvals.property_owner_permission.as_ref(),
            operator_id,
            scope,
        ));
    }

    if case.operation.dangerous_goods_payload {
        findings.push(evidence_gate(
            "dangerous_goods_director_approval",
            "Dangerous-goods approval in the UASOC Operations Manual",
            approvals.dangerous_goods_director_approval.as_ref(),
            operator_id,
            scope,
        ));
        findings.push(evidence_gate(
            "part_92_dangerous_goods_compliance",
            "Part 92 dangerous-goods applicability/compliance evidence",
            approvals.part_92_dangerous_goods_compliance.as_ref(),
            operator_id,
            scope,
        ));
    }

    findings.sort_by(|left, right| left.gate_id.cmp(&right.gate_id));
    let passed_gate_count = findings.iter().filter(|item| item.state == GateState::Pass).count();
    let synthetic_only_gate_count = findings.iter().filter(|item| item.state == GateState::SyntheticOnly).count();
    let blocked_gate_count = findings.iter().filter(|item| item.state == GateState::Blocked).count();
    let indeterminate_gate_count = findings.iter().filter(|item| item.state == GateState::Indeterminate).count();
    let unsupported_gate_count = findings.iter().filter(|item| item.state == GateState::Unsupported).count();

    let eligibility_status = if blocked_gate_count > 0 {
        EligibilityStatus::Blocked
    } else if unsupported_gate_count > 0 {
        EligibilityStatus::UnsupportedBySnapshot
    } else if indeterminate_gate_count > 0 {
        EligibilityStatus::Indeterminate
    } else if synthetic_only_gate_count > 0 {
        EligibilityStatus::PassesSyntheticGateOnly
    } else {
        EligibilityStatus::EligibleForExternalOperatorReview
    };

    UasCaseReport {
        case_id: case.case_id.clone(),
        eligibility_status,
        eligible_for_external_operator_review: eligibility_status
            == EligibilityStatus::EligibleForExternalOperatorReview,
        flight_authorized: false,
        dispatch_command_emitted: false,
        gate_count: findings.len(),
        passed_gate_count,
        synthetic_only_gate_count,
        blocked_gate_count,
        indeterminate_gate_count,
        unsupported_gate_count,
        findings,
        claim_ceiling: CLAIM_CEILING,
    }
}

fn canonical_batch(batch: &UasEligibilityBatch) -> UasEligibilityBatch {
    let mut canonical = batch.clone();
    canonical.source_references.sort();
    canonical.cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));
    canonical
}

pub fn evaluate_za_uas_eligibility(
    batch: &UasEligibilityBatch,
) -> Result<UasEligibilityReport, Vec<String>> {
    validate_batch(batch)?;
    let canonical = canonical_batch(batch);
    let encoded = serde_json::to_vec(&canonical)
        .map_err(|error| vec![format!("cannot encode canonical UAS gate input: {error}")])?;
    let digest = format!("{:x}", Sha256::digest(encoded));
    let cases: Vec<UasCaseReport> = canonical.cases.iter().map(evaluate_case).collect();
    Ok(UasEligibilityReport {
        schema_version: ZA_UAS_GATE_SCHEMA_VERSION,
        source_snapshot_id: ZA_UAS_SOURCE_SNAPSHOT_ID.into(),
        input_digest_sha256: digest,
        case_count: cases.len(),
        cases,
        flight_authority_granted: false,
        dispatch_commands_emitted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(subject: &str, scope: &str, name: &str) -> ApprovalEvidence {
        ApprovalEvidence {
            evidence_class: EvidenceClass::Synthetic,
            status: EvidenceStatus::FreshInScope,
            record_id: format!("synthetic-record-{name}"),
            authority: "synthetic-authority".into(),
            subject_id: subject.into(),
            operation_scope_digest_sha256: scope.into(),
            source_reference: format!("synthetic://approvals/{name}"),
        }
    }

    fn commercial_case() -> UasOperationCase {
        let operator = "operator-01";
        let aircraft = "aircraft-01";
        let pilot = "pilot-01";
        let rps = "rps-01";
        let scope = format!("sha256:{}", "a".repeat(64));
        UasOperationCase {
            case_id: "commercial-r-vlos-delivery".into(),
            assessment_time_utc: "2026-11-20T05:00:00Z".into(),
            operator: OperatorBinding {
                operator_id: operator.into(),
                activity_class: ActivityClass::Commercial,
            },
            aircraft: AircraftBinding {
                aircraft_id: aircraft.into(),
                configuration_id: "delivery-uav-v1".into(),
                uas_class: UasClass::Class1B,
                autonomous_control: false,
            },
            pilot: PilotBinding { pilot_id: pilot.into() },
            operation: OperationConditions {
                operation_id: "parcel-route-a".into(),
                operation_scope_digest_sha256: scope.clone(),
                visual_flight_mode: VisualFlightMode::RestrictedVlos,
                rlos_command_link_available: true,
                rlos_command_link_evidence_reference: "synthetic://c2-link/parcel-route-a".into(),
                payload_release_or_delivery: true,
                multi_uas_operation: false,
                public_road_takeoff_or_landing: false,
                controlled_airspace: false,
                restricted_or_prohibited_airspace: false,
                within_ten_km_of_aerodrome: false,
                property_owner_permission_required: true,
                dangerous_goods_payload: false,
                near_manned_aircraft: false,
                over_or_within_fifty_m_of_public_road: false,
                over_public_road: false,
                rps_id: rps.into(),
                operations_manual_revision: "om-rev-03".into(),
            },
            approvals: ApprovalSet {
                air_service_licence: Some(evidence(operator, &scope, "asl")),
                uas_operator_certificate: Some(evidence(operator, &scope, "uasoc")),
                aircraft_letter_of_approval: Some(evidence(aircraft, &scope, "rla")),
                remote_pilot_licence: Some(evidence(pilot, &scope, "rpl")),
                aircraft_certificate_of_registration: Some(evidence(aircraft, &scope, "cofr")),
                operations_manual_scope: Some(evidence(operator, &scope, "operations-manual")),
                director_payload_release_approval: Some(evidence(operator, &scope, "payload-release")),
                director_bvlos_approval: None,
                sa_cats_101_bvlos_compliance: None,
                director_multi_uas_approval: None,
                director_rps_topology_approval: None,
                director_public_road_approval: None,
                local_authority_public_road_approval: None,
                airspace_approval: None,
                property_owner_permission: Some(evidence(operator, &scope, "property")),
                dangerous_goods_director_approval: None,
                part_92_dangerous_goods_compliance: None,
                sa_cats_101_c2_compliance: Some(evidence(operator, &scope, "c2-compliance")),
                director_class_3_applicability: None,
                director_public_road_overflight_approval: None,
                public_road_closure_evidence: None,
                public_road_safety_controls: None,
                multi_uas_uses_command_inputs_only: None,
            },
        }
    }

    fn batch(cases: Vec<UasOperationCase>) -> UasEligibilityBatch {
        UasEligibilityBatch {
            schema_version: ZA_UAS_GATE_SCHEMA_VERSION,
            source_snapshot_id: ZA_UAS_SOURCE_SNAPSHOT_ID.into(),
            source_references: vec![
                "https://www.caa.co.za/industry-information/uas/".into(),
                "https://www.gov.za/sites/default/files/gcis_document/202303/48228rg11556gon3170.pdf".into(),
            ],
            cases,
        }
    }

    #[test]
    fn commercial_delivery_with_synthetic_receipts_passes_only_synthetic_gate() {
        let report = evaluate_za_uas_eligibility(&batch(vec![commercial_case()])).expect("valid input");
        let row = &report.cases[0];
        assert_eq!(row.eligibility_status, EligibilityStatus::PassesSyntheticGateOnly);
        assert!(!row.eligible_for_external_operator_review);
        assert!(!row.flight_authorized);
        assert!(!report.flight_authority_granted);
        assert!(!report.dispatch_commands_emitted);
    }

    #[test]
    fn missing_commercial_air_service_licence_is_indeterminate() {
        let mut case = commercial_case();
        case.approvals.air_service_licence = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "air_service_licence"
            && gate.state == GateState::Indeterminate));
    }

    #[test]
    fn corporate_activity_does_not_require_commercial_air_service_licence() {
        let mut case = commercial_case();
        case.operator.activity_class = ActivityClass::Corporate;
        case.approvals.air_service_licence = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert!(!report.cases[0].findings.iter().any(|gate| gate.gate_id == "air_service_licence"));
    }

    #[test]
    fn revoked_uasoc_blocks_the_operation() {
        let mut case = commercial_case();
        case.approvals.uas_operator_certificate.as_mut().unwrap().status = EvidenceStatus::Revoked;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn wrong_aircraft_registration_subject_blocks_the_operation() {
        let mut case = commercial_case();
        case.approvals.aircraft_certificate_of_registration.as_mut().unwrap().subject_id = "other-aircraft".into();
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn wrong_operation_scope_digest_blocks_the_operation() {
        let mut case = commercial_case();
        case.approvals.director_payload_release_approval.as_mut().unwrap().operation_scope_digest_sha256 =
            format!("sha256:{}", "b".repeat(64));
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn delivery_without_payload_release_approval_is_indeterminate() {
        let mut case = commercial_case();
        case.approvals.director_payload_release_approval = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn bvlos_requires_director_approval_and_sa_cats_evidence() {
        let mut case = commercial_case();
        case.operation.visual_flight_mode = VisualFlightMode::Bvlos;
        case.approvals.director_bvlos_approval = None;
        case.approvals.sa_cats_101_bvlos_compliance = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "director_bvlos_approval"));
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "sa_cats_101_bvlos_compliance"));
    }

    #[test]
    fn brlos_is_unsupported_under_current_source_snapshot() {
        let mut case = commercial_case();
        case.operation.visual_flight_mode = VisualFlightMode::Brlos;
        case.operation.rlos_command_link_available = true;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::UnsupportedBySnapshot);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "beyond_radio_line_of_sight"
            && gate.state == GateState::Unsupported));
    }

    #[test]
    fn autonomous_control_is_unsupported_not_automatically_exempt() {
        let mut case = commercial_case();
        case.aircraft.autonomous_control = true;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::UnsupportedBySnapshot);
    }

    #[test]
    fn multi_uas_requires_director_and_rps_controls() {
        let mut case = commercial_case();
        case.operation.multi_uas_operation = true;
        case.approvals.director_multi_uas_approval = None;
        case.approvals.director_rps_topology_approval = None;
        case.approvals.multi_uas_uses_command_inputs_only = Some(false);
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "multi_uas_command_input_only"
            && gate.state == GateState::Blocked));
    }

    #[test]
    fn public_road_takeoff_landing_requires_both_approvals() {
        let mut case = commercial_case();
        case.operation.public_road_takeoff_or_landing = true;
        case.approvals.director_public_road_approval = None;
        case.approvals.local_authority_public_road_approval = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn road_proximity_requires_director_scope_and_safety_controls() {
        let mut case = commercial_case();
        case.operation.over_or_within_fifty_m_of_public_road = true;
        case.approvals.director_public_road_overflight_approval = None;
        case.approvals.public_road_safety_controls = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "director_public_road_overflight_approval"));
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "public_road_safety_controls"));
    }

    #[test]
    fn overflight_of_public_road_also_requires_road_closure_evidence() {
        let mut case = commercial_case();
        case.operation.over_or_within_fifty_m_of_public_road = true;
        case.operation.over_public_road = true;
        case.approvals.director_public_road_overflight_approval = Some(evidence(
            &case.operator.operator_id,
            &case.operation.operation_scope_digest_sha256,
            "road-director",
        ));
        case.approvals.public_road_safety_controls = Some(evidence(
            &case.operator.operator_id,
            &case.operation.operation_scope_digest_sha256,
            "road-safety",
        ));
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "public_road_closure_evidence"));
    }

    #[test]
    fn class_three_requires_a_director_applicability_determination_not_an_rmt_flight_gate() {
        let mut case = commercial_case();
        case.aircraft.uas_class = UasClass::Class3OrHigher;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
        assert!(report.cases[0].findings.iter().any(|gate| gate.gate_id == "director_class_3_applicability"));
        assert!(!report.cases[0].findings.iter().any(|gate| gate.gate_id.contains("maintenance_technician")));
    }

    #[test]
    fn unknown_aircraft_classification_is_indeterminate() {
        let mut case = commercial_case();
        case.aircraft.uas_class = UasClass::Unknown;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn missing_current_c2_compliance_prevents_review_eligibility() {
        let mut case = commercial_case();
        case.approvals.sa_cats_101_c2_compliance = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn near_manned_aircraft_requires_airspace_scope_evidence() {
        let mut case = commercial_case();
        case.operation.near_manned_aircraft = true;
        case.approvals.airspace_approval = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn restricted_airspace_requires_route_scoped_airspace_approval() {
        let mut case = commercial_case();
        case.operation.restricted_or_prohibited_airspace = true;
        case.approvals.airspace_approval = None;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn stale_approval_is_indeterminate_not_a_pass() {
        let mut case = commercial_case();
        case.approvals.air_service_licence.as_mut().unwrap().status = EvidenceStatus::Stale;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Indeterminate);
    }

    #[test]
    fn expired_approval_blocks_the_operation() {
        let mut case = commercial_case();
        case.approvals.air_service_licence.as_mut().unwrap().status = EvidenceStatus::Expired;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn private_operation_is_limited_to_r_vlos_and_class_1a_or_1b() {
        let mut case = commercial_case();
        case.operator.activity_class = ActivityClass::Private;
        case.operation.visual_flight_mode = VisualFlightMode::Vlos;
        case.approvals.air_service_licence = None;
        case.approvals.uas_operator_certificate = None;
        case.approvals.aircraft_letter_of_approval = None;
        case.approvals.remote_pilot_licence = None;
        case.approvals.aircraft_certificate_of_registration = None;
        case.approvals.operations_manual_scope = None;
        case.operation.payload_release_or_delivery = false;
        case.operation.dangerous_goods_payload = false;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn private_parcel_delivery_is_blocked_not_misclassified_as_commercial_exemption() {
        let mut case = commercial_case();
        case.operator.activity_class = ActivityClass::Private;
        let report = evaluate_za_uas_eligibility(&batch(vec![case])).expect("valid envelope");
        assert_eq!(report.cases[0].eligibility_status, EligibilityStatus::Blocked);
    }

    #[test]
    fn deterministic_output_ignores_case_and_source_reference_order() {
        let first_case = commercial_case();
        let mut second_case = commercial_case();
        second_case.case_id = "commercial-case-02".into();
        let first = evaluate_za_uas_eligibility(&batch(vec![first_case.clone(), second_case.clone()]))
            .expect("first batch");
        let mut reordered = batch(vec![second_case, first_case]);
        reordered.source_references.reverse();
        let second = evaluate_za_uas_eligibility(&reordered).expect("reordered batch");
        assert_eq!(first, second);
    }

    #[test]
    fn rejects_stale_source_snapshot_version() {
        let mut input = batch(vec![commercial_case()]);
        input.source_snapshot_id = "ZA-SACAA-PART101-2025-01-01".into();
        assert!(validate_batch(&input).is_err());
    }

    #[test]
    fn rejects_duplicate_case_ids() {
        let case = commercial_case();
        let mut duplicate = commercial_case();
        duplicate.case_id = case.case_id.clone();
        assert!(validate_batch(&batch(vec![case, duplicate])).is_err());
    }

    #[test]
    fn every_case_keeps_flight_authority_and_dispatch_false() {
        let report = evaluate_za_uas_eligibility(&batch(vec![commercial_case()])).expect("batch");
        assert!(!report.flight_authority_granted);
        assert!(!report.dispatch_commands_emitted);
        assert!(!report.cases[0].flight_authorized);
        assert!(!report.cases[0].dispatch_command_emitted);
    }
}
