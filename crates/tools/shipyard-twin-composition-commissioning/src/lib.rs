//! Synthetic composition and commissioning assurance for shipyard digital twins.
//!
//! This crate checks the composed system, not merely the independent health
//! status of its members. It is synthetic-only and does not commission physical
//! equipment or claim conformance to ISO/AWI work items.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionMode {
    Integrated,
    Unified,
    Federated,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticOnly,
    MeasuredPhysical,
    Simulation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TwinMember {
    pub member_id: String,
    pub owner_id: String,
    pub workpiece_id: String,
    pub revision: String,
    pub schema_version: String,
    pub units_signature: String,
    pub coordinate_frame: String,
    pub calibration_revision: String,
    pub clock_domain: String,
    pub authority_domain: String,
    pub allowed_capabilities: Vec<String>,
    pub source_sha256: [u8; 32],
    pub evidence_class: EvidenceClass,
    pub current: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceContract {
    pub from_member_id: String,
    pub to_member_id: String,
    pub source_revision: String,
    pub target_revision: String,
    pub source_schema_version: String,
    pub target_schema_version: String,
    pub schema_compatible: bool,
    pub units_compatible: bool,
    pub transform_receipt_sha256: [u8; 32],
    pub source_calibration_revision: String,
    pub target_calibration_revision: String,
    pub clock_mapping_receipt_sha256: [u8; 32],
    pub clock_offset_bound_ticks: u64,
    pub maximum_clock_offset_ticks: u64,
    pub update_period_ticks: u64,
    pub maximum_update_period_ticks: u64,
    pub latency_ticks: u64,
    pub maximum_latency_ticks: u64,
    pub jitter_ticks: u64,
    pub maximum_jitter_ticks: u64,
    pub fidelity_permille: u16,
    pub minimum_fidelity_permille: u16,
    pub unified_semantic_schema: Option<String>,
    pub authority_preserved: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologyEvidence {
    pub integrated_authority_domain: Option<String>,
    pub unified_semantic_schema: Option<String>,
    pub federated_authority_receipt_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissioningRequest {
    pub composition_id: String,
    pub composition_revision: String,
    pub mode: CompositionMode,
    pub members: Vec<TwinMember>,
    pub interfaces: Vec<InterfaceContract>,
    pub topology_evidence: TopologyEvidence,
    pub requested_capability: String,
    pub decision_at_tick: u64,
    pub receipt_ttl_ticks: u64,
    pub maximum_end_to_end_latency_ticks: u64,
    pub unknown_update_outcome: bool,
    pub live_operation_requested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureDomain {
    Member,
    Interface,
    Composition,
    Authority,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    EmptyIdentity,
    TooFewMembers,
    DuplicateMember,
    WorkpieceMismatch,
    MissingSourceDigest,
    PhysicalEvidenceNotAdmitted,
    StaleMember,
    RequestedCapabilityNotGranted,
    MissingInterface,
    UnknownInterfaceMember,
    SelfInterface,
    DuplicateInterface,
    MemberRevisionMismatch,
    SchemaIncompatible,
    UnitsIncompatible,
    CoordinateTransformMissing,
    CalibrationRevisionMismatch,
    ClockMappingMissing,
    ClockOffsetTooLarge,
    UpdatePeriodTooSlow,
    LatencyTooHigh,
    JitterTooHigh,
    FidelityTooLow,
    UnifiedSchemaMissing,
    IntegratedAuthorityDomainMismatch,
    FederatedAuthorityReceiptMissing,
    AuthorityNotPreserved,
    EndToEndLatencyTooHigh,
    UnknownUpdateOutcome,
    LiveOperationForbidden,
    InvalidReceiptTtl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissioningFailure {
    pub domain: FailureDomain,
    pub reason: FailureReason,
    pub member_id: Option<String>,
    pub interface_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberBinding {
    pub member_id: String,
    pub revision: String,
    pub source_sha256: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommissioningReceipt {
    pub composition_id: String,
    pub composition_revision: String,
    pub mode: CompositionMode,
    pub members: Vec<MemberBinding>,
    pub issued_at_tick: u64,
    pub expires_at_tick: u64,
    pub granted_capability: String,
    pub state: CompositionState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionState {
    Draft,
    Commissioning,
    Qualified,
    Degraded,
    Suspended,
    RequalificationRequired,
    Retired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleEvent {
    BeginCommissioning,
    CommissionSucceeded,
    CommissionFailed,
    MemberChanged,
    ReceiptExpired,
    UnknownOutcome,
    Reconciled,
    RepairCompleted,
    Retire,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptCheck {
    Valid,
    RequalificationRequired,
    Suspended,
}

fn empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn nonzero(digest: &[u8; 32]) -> bool {
    digest.iter().any(|byte| *byte != 0)
}

fn failure(
    domain: FailureDomain,
    reason: FailureReason,
    member_id: Option<&str>,
    interface_id: Option<&str>,
) -> CommissioningFailure {
    CommissioningFailure {
        domain,
        reason,
        member_id: member_id.map(str::to_owned),
        interface_id: interface_id.map(str::to_owned),
    }
}

fn interface_id(interface: &InterfaceContract) -> String {
    format!("{}->{}", interface.from_member_id, interface.to_member_id)
}

fn validate_topology(request: &CommissioningRequest) -> Result<(), CommissioningFailure> {
    match request.mode {
        CompositionMode::Integrated => {
            let Some(authority_domain) = request
                .topology_evidence
                .integrated_authority_domain
                .as_deref()
                .filter(|value| !empty(value))
            else {
                return Err(failure(
                    FailureDomain::Composition,
                    FailureReason::IntegratedAuthorityDomainMismatch,
                    None,
                    None,
                ));
            };
            if request
                .members
                .iter()
                .any(|member| member.authority_domain != authority_domain)
            {
                return Err(failure(
                    FailureDomain::Composition,
                    FailureReason::IntegratedAuthorityDomainMismatch,
                    None,
                    None,
                ));
            }
        }
        CompositionMode::Unified => {
            let Some(unified_schema) = request
                .topology_evidence
                .unified_semantic_schema
                .as_deref()
                .filter(|value| !empty(value))
            else {
                return Err(failure(
                    FailureDomain::Composition,
                    FailureReason::UnifiedSchemaMissing,
                    None,
                    None,
                ));
            };
            if request.interfaces.iter().any(|interface| {
                interface.unified_semantic_schema.as_deref() != Some(unified_schema)
            }) {
                return Err(failure(
                    FailureDomain::Interface,
                    FailureReason::UnifiedSchemaMissing,
                    None,
                    None,
                ));
            }
        }
        CompositionMode::Federated => {
            if !nonzero(
                &request
                    .topology_evidence
                    .federated_authority_receipt_sha256,
            ) || request
                .interfaces
                .iter()
                .any(|interface| !interface.authority_preserved)
            {
                return Err(failure(
                    FailureDomain::Authority,
                    FailureReason::FederatedAuthorityReceiptMissing,
                    None,
                    None,
                ));
            }
        }
    }
    Ok(())
}

/// Commission a synthetic composition only after every member and interface passes.
///
/// The returned receipt binds exact member revisions and source digests. It has a
/// bounded lifetime and never grants a capability not held by every member.
pub fn commission(
    request: &CommissioningRequest,
) -> Result<CommissioningReceipt, CommissioningFailure> {
    if empty(&request.composition_id)
        || empty(&request.composition_revision)
        || empty(&request.requested_capability)
    {
        return Err(failure(
            FailureDomain::Composition,
            FailureReason::EmptyIdentity,
            None,
            None,
        ));
    }
    if request.members.len() < 2 {
        return Err(failure(
            FailureDomain::Composition,
            FailureReason::TooFewMembers,
            None,
            None,
        ));
    }
    if request.interfaces.is_empty() {
        return Err(failure(
            FailureDomain::Composition,
            FailureReason::MissingInterface,
            None,
            None,
        ));
    }
    if request.receipt_ttl_ticks == 0 {
        return Err(failure(
            FailureDomain::Composition,
            FailureReason::InvalidReceiptTtl,
            None,
            None,
        ));
    }
    if request.unknown_update_outcome {
        return Err(failure(
            FailureDomain::Authority,
            FailureReason::UnknownUpdateOutcome,
            None,
            None,
        ));
    }
    if request.live_operation_requested {
        return Err(failure(
            FailureDomain::Authority,
            FailureReason::LiveOperationForbidden,
            None,
            None,
        ));
    }

    let mut member_by_id = HashMap::new();
    let mut member_ids = HashSet::new();
    let expected_workpiece = request.members[0].workpiece_id.as_str();

    for member in &request.members {
        if empty(&member.member_id)
            || empty(&member.owner_id)
            || empty(&member.workpiece_id)
            || empty(&member.revision)
            || empty(&member.schema_version)
            || empty(&member.units_signature)
            || empty(&member.coordinate_frame)
            || empty(&member.calibration_revision)
            || empty(&member.clock_domain)
            || empty(&member.authority_domain)
        {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::EmptyIdentity,
                Some(&member.member_id),
                None,
            ));
        }
        if !member_ids.insert(member.member_id.as_str()) {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::DuplicateMember,
                Some(&member.member_id),
                None,
            ));
        }
        if member.workpiece_id != expected_workpiece {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::WorkpieceMismatch,
                Some(&member.member_id),
                None,
            ));
        }
        if !nonzero(&member.source_sha256) {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::MissingSourceDigest,
                Some(&member.member_id),
                None,
            ));
        }
        if member.evidence_class != EvidenceClass::SyntheticOnly {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::PhysicalEvidenceNotAdmitted,
                Some(&member.member_id),
                None,
            ));
        }
        if !member.current {
            return Err(failure(
                FailureDomain::Member,
                FailureReason::StaleMember,
                Some(&member.member_id),
                None,
            ));
        }
        if !member
            .allowed_capabilities
            .iter()
            .any(|capability| capability == &request.requested_capability)
        {
            return Err(failure(
                FailureDomain::Authority,
                FailureReason::RequestedCapabilityNotGranted,
                Some(&member.member_id),
                None,
            ));
        }
        member_by_id.insert(member.member_id.as_str(), member);
    }

    let mut seen_interfaces = HashSet::new();
    let mut aggregate_latency_ticks = 0u64;
    for interface in &request.interfaces {
        let id = interface_id(interface);
        if interface.from_member_id == interface.to_member_id {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::SelfInterface,
                None,
                Some(&id),
            ));
        }
        if !seen_interfaces.insert(id.clone()) {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::DuplicateInterface,
                None,
                Some(&id),
            ));
        }
        let Some(from) = member_by_id.get(interface.from_member_id.as_str()) else {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::UnknownInterfaceMember,
                None,
                Some(&id),
            ));
        };
        let Some(to) = member_by_id.get(interface.to_member_id.as_str()) else {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::UnknownInterfaceMember,
                None,
                Some(&id),
            ));
        };
        if interface.source_revision != from.revision
            || interface.target_revision != to.revision
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::MemberRevisionMismatch,
                None,
                Some(&id),
            ));
        }
        if !interface.schema_compatible
            || interface.source_schema_version != from.schema_version
            || interface.target_schema_version != to.schema_version
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::SchemaIncompatible,
                None,
                Some(&id),
            ));
        }
        if !interface.units_compatible
            || from.units_signature != to.units_signature
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::UnitsIncompatible,
                None,
                Some(&id),
            ));
        }
        if from.coordinate_frame != to.coordinate_frame
            && (!nonzero(&interface.transform_receipt_sha256)
                || interface.source_calibration_revision != from.calibration_revision
                || interface.target_calibration_revision != to.calibration_revision)
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::CoordinateTransformMissing,
                None,
                Some(&id),
            ));
        }
        if interface.clock_offset_bound_ticks > interface.maximum_clock_offset_ticks
            || (from.clock_domain != to.clock_domain
                && !nonzero(&interface.clock_mapping_receipt_sha256))
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::ClockOffsetTooLarge,
                None,
                Some(&id),
            ));
        }
        if interface.update_period_ticks == 0
            || interface.update_period_ticks > interface.maximum_update_period_ticks
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::UpdatePeriodTooSlow,
                None,
                Some(&id),
            ));
        }
        if interface.latency_ticks > interface.maximum_latency_ticks {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::LatencyTooHigh,
                None,
                Some(&id),
            ));
        }
        if interface.jitter_ticks > interface.maximum_jitter_ticks {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::JitterTooHigh,
                None,
                Some(&id),
            ));
        }
        if interface.fidelity_permille < interface.minimum_fidelity_permille
            || interface.fidelity_permille > 1_000
        {
            return Err(failure(
                FailureDomain::Interface,
                FailureReason::FidelityTooLow,
                None,
                Some(&id),
            ));
        }
        if !interface.authority_preserved {
            return Err(failure(
                FailureDomain::Authority,
                FailureReason::AuthorityNotPreserved,
                None,
                Some(&id),
            ));
        }
        aggregate_latency_ticks = aggregate_latency_ticks
            .checked_add(interface.latency_ticks)
            .ok_or_else(|| {
                failure(
                    FailureDomain::Composition,
                    FailureReason::EndToEndLatencyTooHigh,
                    None,
                    None,
                )
            })?;
    }

    if aggregate_latency_ticks > request.maximum_end_to_end_latency_ticks {
        return Err(failure(
            FailureDomain::Composition,
            FailureReason::EndToEndLatencyTooHigh,
            None,
            None,
        ));
    }
    validate_topology(request)?;

    let expires_at_tick = request
        .decision_at_tick
        .checked_add(request.receipt_ttl_ticks)
        .ok_or_else(|| {
            failure(
                FailureDomain::Composition,
                FailureReason::InvalidReceiptTtl,
                None,
                None,
            )
        })?;
    let mut members: Vec<MemberBinding> = request
        .members
        .iter()
        .map(|member| MemberBinding {
            member_id: member.member_id.clone(),
            revision: member.revision.clone(),
            source_sha256: member.source_sha256,
        })
        .collect();
    members.sort_by(|left, right| left.member_id.cmp(&right.member_id));

    Ok(CommissioningReceipt {
        composition_id: request.composition_id.clone(),
        composition_revision: request.composition_revision.clone(),
        mode: request.mode,
        members,
        issued_at_tick: request.decision_at_tick,
        expires_at_tick,
        granted_capability: request.requested_capability.clone(),
        state: CompositionState::Qualified,
    })
}

/// A receipt must be requalified if membership, revision or source digests change.
pub fn check_receipt(
    request: &CommissioningRequest,
    receipt: &CommissioningReceipt,
    now_tick: u64,
) -> ReceiptCheck {
    if receipt.state != CompositionState::Qualified
        || receipt.composition_id != request.composition_id
        || receipt.composition_revision != request.composition_revision
        || receipt.mode != request.mode
        || receipt.granted_capability != request.requested_capability
        || now_tick < receipt.issued_at_tick
        || now_tick >= receipt.expires_at_tick
    {
        return ReceiptCheck::RequalificationRequired;
    }
    let mut current: Vec<MemberBinding> = request
        .members
        .iter()
        .map(|member| MemberBinding {
            member_id: member.member_id.clone(),
            revision: member.revision.clone(),
            source_sha256: member.source_sha256,
        })
        .collect();
    current.sort_by(|left, right| left.member_id.cmp(&right.member_id));
    if current != receipt.members {
        return ReceiptCheck::RequalificationRequired;
    }
    if request.unknown_update_outcome {
        return ReceiptCheck::Suspended;
    }
    ReceiptCheck::Valid
}

/// Deterministic composition lifecycle with fail-closed unknown-outcome handling.
pub fn transition_state(
    current: CompositionState,
    event: LifecycleEvent,
) -> CompositionState {
    use CompositionState::*;
    use LifecycleEvent::*;
    if current == Retired {
        return Retired;
    }
    if event == Retire {
        return Retired;
    }
    if event == UnknownOutcome {
        return Suspended;
    }
    match (current, event) {
        (Draft, BeginCommissioning) => Commissioning,
        (Commissioning, CommissionSucceeded) => Qualified,
        (Commissioning, CommissionFailed) => Degraded,
        (Qualified, MemberChanged | ReceiptExpired) => RequalificationRequired,
        (Suspended, Reconciled) => RequalificationRequired,
        (Degraded, RepairCompleted) => Commissioning,
        (RequalificationRequired, BeginCommissioning) => Commissioning,
        (Suspended, BeginCommissioning) => Commissioning,
        (state, _) => state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> [u8; 32] {
        [byte; 32]
    }

    fn member(id: &str, owner: &str, clock: &str) -> TwinMember {
        TwinMember {
            member_id: id.to_owned(),
            owner_id: owner.to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            revision: "R1".to_owned(),
            schema_version: "1.0".to_owned(),
            units_signature: "SI-MM-NS".to_owned(),
            coordinate_frame: "FRAME-A".to_owned(),
            calibration_revision: "CAL-R1".to_owned(),
            clock_domain: clock.to_owned(),
            authority_domain: "YARD-OBSERVE".to_owned(),
            allowed_capabilities: vec!["observe".to_owned()],
            source_sha256: digest(if id == "equipment" { 0x11 } else { 0x22 }),
            evidence_class: EvidenceClass::SyntheticOnly,
            current: true,
        }
    }

    fn interface(from: &str, to: &str, clock_offset: u64) -> InterfaceContract {
        InterfaceContract {
            from_member_id: from.to_owned(),
            to_member_id: to.to_owned(),
            source_revision: "R1".to_owned(),
            target_revision: "R1".to_owned(),
            source_schema_version: "1.0".to_owned(),
            target_schema_version: "1.0".to_owned(),
            schema_compatible: true,
            units_compatible: true,
            transform_receipt_sha256: [0; 32],
            source_calibration_revision: "CAL-R1".to_owned(),
            target_calibration_revision: "CAL-R1".to_owned(),
            clock_mapping_receipt_sha256: if clock_offset == 0 { [0; 32] } else { digest(0x33) },
            clock_offset_bound_ticks: clock_offset,
            maximum_clock_offset_ticks: 5,
            update_period_ticks: 10,
            maximum_update_period_ticks: 20,
            latency_ticks: 5,
            maximum_latency_ticks: 10,
            jitter_ticks: 1,
            maximum_jitter_ticks: 3,
            fidelity_permille: 995,
            minimum_fidelity_permille: 980,
            unified_semantic_schema: Some("YARD-CANONICAL-1".to_owned()),
            authority_preserved: true,
        }
    }

    fn base_request(mode: CompositionMode) -> CommissioningRequest {
        CommissioningRequest {
            composition_id: "WELD-TWIN-COMPOSITION-01".to_owned(),
            composition_revision: "C1".to_owned(),
            mode,
            members: vec![
                member("equipment", "OWNER-A", "CLOCK-A"),
                member("workpiece", "OWNER-B", "CLOCK-A"),
            ],
            interfaces: vec![interface("equipment", "workpiece", 0)],
            topology_evidence: TopologyEvidence {
                integrated_authority_domain: Some("YARD-OBSERVE".to_owned()),
                unified_semantic_schema: Some("YARD-CANONICAL-1".to_owned()),
                federated_authority_receipt_sha256: digest(0x44),
            },
            requested_capability: "observe".to_owned(),
            decision_at_tick: 100,
            receipt_ttl_ticks: 50,
            maximum_end_to_end_latency_ticks: 10,
            unknown_update_outcome: false,
            live_operation_requested: false,
        }
    }

    #[test]
    fn integrated_composition_commissions() {
        let result = commission(&base_request(CompositionMode::Integrated));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().state, CompositionState::Qualified);
    }

    #[test]
    fn unified_composition_commissions() {
        let result = commission(&base_request(CompositionMode::Unified));
        assert!(result.is_ok());
    }

    #[test]
    fn federated_composition_commissions_with_authority_receipt() {
        let result = commission(&base_request(CompositionMode::Federated));
        assert!(result.is_ok());
    }

    #[test]
    fn member_revision_is_bound_into_receipt() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        assert_eq!(receipt.members[0].revision, "R1");
        assert_eq!(receipt.members[0].source_sha256, digest(0x11));
    }

    #[test]
    fn receipt_is_valid_until_expiry() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        assert_eq!(check_receipt(&request, &receipt, 149), ReceiptCheck::Valid);
    }

    #[test]
    fn receipt_expires_and_requires_requalification() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        assert_eq!(
            check_receipt(&request, &receipt, 150),
            ReceiptCheck::RequalificationRequired
        );
    }

    #[test]
    fn member_revision_change_invalidates_composition_receipt() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        let mut changed = request.clone();
        changed.members[0].revision = "R2".to_owned();
        assert_eq!(
            check_receipt(&changed, &receipt, 120),
            ReceiptCheck::RequalificationRequired
        );
    }

    #[test]
    fn source_digest_change_invalidates_composition_receipt() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        let mut changed = request.clone();
        changed.members[0].source_sha256 = digest(0x77);
        assert_eq!(
            check_receipt(&changed, &receipt, 120),
            ReceiptCheck::RequalificationRequired
        );
    }

    #[test]
    fn different_workpiece_identity_is_rejected() {
        let mut request = base_request(CompositionMode::Integrated);
        request.members[1].workpiece_id = "WID-SYNTH-02".to_owned();
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::WorkpieceMismatch
        );
    }

    #[test]
    fn unknown_update_outcome_suspends_composition() {
        let mut request = base_request(CompositionMode::Integrated);
        request.unknown_update_outcome = true;
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::UnknownUpdateOutcome
        );
    }

    #[test]
    fn unknown_outcome_invalidates_existing_receipt_until_reconciliation() {
        let request = base_request(CompositionMode::Integrated);
        let receipt = commission(&request).expect("base should commission");
        let mut unknown = request.clone();
        unknown.unknown_update_outcome = true;
        assert_eq!(check_receipt(&unknown, &receipt, 120), ReceiptCheck::Suspended);
    }

    #[test]
    fn excessive_end_to_end_latency_rejects_locally_green_interfaces() {
        let mut request = base_request(CompositionMode::Integrated);
        request.interfaces.push(interface("workpiece", "equipment", 0));
        request.maximum_end_to_end_latency_ticks = 9;
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::EndToEndLatencyTooHigh
        );
    }

    #[test]
    fn federated_authority_receipt_is_required() {
        let mut request = base_request(CompositionMode::Federated);
        request.topology_evidence.federated_authority_receipt_sha256 = [0; 32];
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::FederatedAuthorityReceiptMissing
        );
    }

    #[test]
    fn unified_semantic_schema_must_match_each_interface() {
        let mut request = base_request(CompositionMode::Unified);
        request.interfaces[0].unified_semantic_schema = Some("OTHER".to_owned());
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::UnifiedSchemaMissing
        );
    }

    #[test]
    fn live_operation_is_never_authorized() {
        let mut request = base_request(CompositionMode::Integrated);
        request.live_operation_requested = true;
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::LiveOperationForbidden
        );
    }

    #[test]
    fn unsupported_requested_capability_is_rejected() {
        let mut request = base_request(CompositionMode::Integrated);
        request.requested_capability = "execute_weld".to_owned();
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::RequestedCapabilityNotGranted
        );
    }

    #[test]
    fn interface_clock_mapping_required_for_different_clock_domains() {
        let mut request = base_request(CompositionMode::Federated);
        request.members[1].clock_domain = "CLOCK-B".to_owned();
        request.interfaces[0].clock_mapping_receipt_sha256 = [0; 32];
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::ClockOffsetTooLarge
        );
    }

    #[test]
    fn interface_fidelity_is_a_separate_commissioning_dimension() {
        let mut request = base_request(CompositionMode::Integrated);
        request.interfaces[0].fidelity_permille = 900;
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::FidelityTooLow
        );
    }

    #[test]
    fn missing_coordinate_transform_rejects_different_frames() {
        let mut request = base_request(CompositionMode::Integrated);
        request.members[1].coordinate_frame = "FRAME-B".to_owned();
        assert_eq!(
            commission(&request).unwrap_err().reason,
            FailureReason::CoordinateTransformMissing
        );
    }

    #[test]
    fn lifecycle_unknown_outcome_suspends_qualified_composition() {
        assert_eq!(
            transition_state(CompositionState::Qualified, LifecycleEvent::UnknownOutcome),
            CompositionState::Suspended
        );
    }

    #[test]
    fn member_change_requires_requalification() {
        assert_eq!(
            transition_state(CompositionState::Qualified, LifecycleEvent::MemberChanged),
            CompositionState::RequalificationRequired
        );
    }

    #[test]
    fn retired_composition_cannot_be_reactivated() {
        assert_eq!(
            transition_state(CompositionState::Retired, LifecycleEvent::BeginCommissioning),
            CompositionState::Retired
        );
    }
}
