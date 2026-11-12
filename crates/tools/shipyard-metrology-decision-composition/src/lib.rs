//! Point-in-time composition contract for scan, metrology and pass decisions.
//!
//! V0.1 is synthetic-only. It composes upstream evidence receipts; it does not
//! recalibrate a scanner, register point clouds, recompute profile geometry,
//! predict weld physics, evaluate NDT, qualify a WPS, or authorize execution.
//! SHA-256 values are supplied by receipt producers and checked for format and
//! exact cross-link equality. This crate does not implement a hashing primitive.

use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Identity {
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceKind {
    Calibration,
    ReferenceScan,
    ObservedScan,
    Registration,
    ScanQualification,
    GeometryComparison,
    NdtOutcome,
    RepairDisposition,
    LearningArtifact,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    Simulated,
    Measured,
    Inspected,
    Accepted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionPurpose {
    MediumNextPass,
    SlowQualityRequalification,
    ProductionAcceptance,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleMarker {
    pub event_id: String,
    pub content_sha256: String,
    pub effective_at_tick: u64,
    pub first_received_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceEnvelope {
    pub kind: EvidenceKind,
    pub evidence_id: String,
    pub content_sha256: String,
    pub identity: Identity,
    pub evidence_class: EvidenceClass,
    pub clock_id: String,
    pub measured_at_tick: u64,
    pub first_received_at_tick: u64,
    pub valid_from_tick: u64,
    pub expires_at_tick: u64,
    pub purpose: DecisionPurpose,
    pub authorized_for_medium_next_pass: bool,
    pub revocation: Option<LifecycleMarker>,
    pub supersession: Option<LifecycleMarker>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScanQualificationReceipt {
    pub receipt_sha256: String,
    pub identity: Identity,
    pub observed_scan_id: String,
    pub observed_scan_sha256: String,
    pub calibration_receipt_sha256: String,
    pub registration_receipt_sha256: String,
    pub coordinate_frame: String,
    pub policy_id: String,
    pub policy_revision: String,
    pub expanded_length_uncertainty_units: u64,
    pub maximum_expanded_length_uncertainty_units: u64,
    pub length_decision_tolerance_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeometryComparisonReceipt {
    pub output_sha256: String,
    pub identity: Identity,
    pub reference_scan_id: String,
    pub reference_scan_sha256: String,
    pub observed_scan_id: String,
    pub observed_scan_sha256: String,
    pub registration_receipt_sha256: String,
    pub coordinate_frame: String,
    pub length_unit: String,
    pub x_grid_sha256: String,
    pub coverage_permille: u16,
    pub minimum_coverage_permille: u16,
    pub maximum_unsupported_gap_units: u64,
    pub largest_unsupported_gap_units: u64,
    pub signed_net_area_units: i128,
    pub added_area_units: u128,
    pub removed_area_units: u128,
    pub area_uncertainty_units: u128,
    pub maximum_area_uncertainty_units: u128,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionInput {
    pub identity: Identity,
    pub decision_tick: u64,
    pub clock_id: String,
    pub purpose: DecisionPurpose,
    pub evidence_snapshot_sha256: String,
    pub candidate_set_sha256: String,
    pub planner_build_sha256: String,
    pub evidence: Vec<EvidenceEnvelope>,
    pub medium_feature_event_ids: Vec<String>,
    pub scan_qualification: ScanQualificationReceipt,
    pub geometry_comparison: GeometryComparisonReceipt,
    pub automatic_selection_requested: bool,
    pub live_execution_requested: bool,
    pub wps_change_requested: bool,
    pub accepted_as_built_write_requested: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    InvalidDigest,
    DuplicateEvidenceId,
    WrongDecisionPurpose,
    UnauthorizedActionRequested,
    MissingRequiredEvidence,
    DuplicateRequiredEvidence,
    EvidenceIdentityMismatch,
    EvidenceClassNotSynthetic,
    EvidenceClockMismatch,
    EvidenceNotAuthorizedForMediumLoop,
    EvidenceWrongPurpose,
    MeasurementFromFuture,
    EvidenceReceivedAfterCutoff,
    EvidenceNotYetValidAtCutoff,
    EvidenceExpiredAtCutoff,
    EvidenceRevokedAtCutoff,
    EvidenceSupersededAtCutoff,
    InvalidLifecycleMarker,
    EvidenceDependencyOrderInvalid,
    MediumFeatureSetMismatch,
    ForbiddenMediumEvidence,
    ScanQualificationReceiptMismatch,
    GeometryReceiptMismatch,
    ReferenceAndObservedScanMustDiffer,
    CoordinateFrameMismatch,
    CoordinateUnitMismatch,
    InvalidCoverage,
    UnsupportedGapAboveLimit,
    LengthUncertaintyAboveLimit,
    AreaUncertaintyAboveLimit,
    InvalidUncertaintyLimit,
    GeometryAreaArithmeticOverflow,
    GeometrySignedAreaMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct AcceptedEvidenceRef {
    pub kind: EvidenceKind,
    pub evidence_id: String,
    pub content_sha256: String,
    pub measured_at_tick: u64,
    pub first_received_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PendingLifecycleRef {
    pub evidence_id: String,
    pub content_sha256: String,
    pub effective_at_tick: u64,
    pub first_received_at_tick: u64,
    pub is_supersession: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComposedDecisionReceipt {
    pub identity: Identity,
    pub decision_tick: u64,
    pub clock_id: String,
    pub purpose: DecisionPurpose,
    pub accepted_evidence: Vec<AcceptedEvidenceRef>,
    pub pending_lifecycle_events_known_at_cutoff: Vec<PendingLifecycleRef>,
    pub evidence_snapshot_sha256: String,
    pub candidate_set_sha256: String,
    pub planner_build_sha256: String,
    pub scan_qualification_sha256: String,
    pub geometry_comparison_sha256: String,
    pub reference_scan_sha256: String,
    pub observed_scan_sha256: String,
    pub calibration_receipt_sha256: String,
    pub registration_receipt_sha256: String,
    pub expanded_length_uncertainty_units: u64,
    pub length_decision_tolerance_units: u64,
    pub signed_net_area_units: i128,
    pub added_area_units: u128,
    pub removed_area_units: u128,
    pub area_uncertainty_units: u128,
    pub claim_ceiling: &'static str,
    pub automatic_selection: bool,
    pub live_execution_authorized: bool,
    pub wps_change_authorized: bool,
    pub accepted_as_built_write_authorized: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionDecision {
    Composed(Box<ComposedDecisionReceipt>),
    Abstain(Vec<AbstainReason>),
}

fn empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn push_reason(reasons: &mut Vec<AbstainReason>, reason: AbstainReason) {
    if !reasons.contains(&reason) {
        reasons.push(reason);
    }
}

fn one(
    evidence: &[EvidenceEnvelope],
    kind: EvidenceKind,
) -> Result<&EvidenceEnvelope, AbstainReason> {
    let mut matches = evidence.iter().filter(|item| item.kind == kind);
    let first = matches
        .next()
        .ok_or(AbstainReason::MissingRequiredEvidence)?;
    if matches.next().is_some() {
        return Err(AbstainReason::DuplicateRequiredEvidence);
    }
    Ok(first)
}

fn validate_lifecycle(
    marker: &Option<LifecycleMarker>,
    decision_tick: u64,
    is_supersession: bool,
    pending: &mut Vec<PendingLifecycleRef>,
    reasons: &mut Vec<AbstainReason>,
) {
    let Some(marker) = marker else {
        return;
    };

    // A lifecycle event first received after cutoff is outside the historical
    // knowledge set. Ignore it before inspecting its claimed effective time.
    if marker.first_received_at_tick > decision_tick {
        return;
    }
    if empty(&marker.event_id) || !valid_sha256(&marker.content_sha256) {
        push_reason(reasons, AbstainReason::InvalidLifecycleMarker);
        return;
    }
    if marker.effective_at_tick <= decision_tick {
        push_reason(
            reasons,
            if is_supersession {
                AbstainReason::EvidenceSupersededAtCutoff
            } else {
                AbstainReason::EvidenceRevokedAtCutoff
            },
        );
    } else {
        pending.push(PendingLifecycleRef {
            evidence_id: marker.event_id.clone(),
            content_sha256: marker.content_sha256.clone(),
            effective_at_tick: marker.effective_at_tick,
            first_received_at_tick: marker.first_received_at_tick,
            is_supersession,
        });
    }
}

fn validate_envelope(
    item: &EvidenceEnvelope,
    input: &CompositionInput,
    pending: &mut Vec<PendingLifecycleRef>,
    reasons: &mut Vec<AbstainReason>,
) {
    if empty(&item.evidence_id) || empty(&item.clock_id) {
        push_reason(reasons, AbstainReason::EmptyIdentity);
    }
    if !valid_sha256(&item.content_sha256) {
        push_reason(reasons, AbstainReason::InvalidDigest);
    }
    if item.identity != input.identity {
        push_reason(reasons, AbstainReason::EvidenceIdentityMismatch);
    }
    if item.evidence_class != EvidenceClass::SyntheticContractOnly {
        push_reason(reasons, AbstainReason::EvidenceClassNotSynthetic);
    }
    if item.clock_id != input.clock_id {
        push_reason(reasons, AbstainReason::EvidenceClockMismatch);
    }
    if item.purpose != DecisionPurpose::MediumNextPass {
        push_reason(reasons, AbstainReason::EvidenceWrongPurpose);
    }
    if !item.authorized_for_medium_next_pass {
        push_reason(reasons, AbstainReason::EvidenceNotAuthorizedForMediumLoop);
    }
    if item.measured_at_tick > input.decision_tick {
        push_reason(reasons, AbstainReason::MeasurementFromFuture);
    }
    if item.first_received_at_tick > input.decision_tick {
        push_reason(reasons, AbstainReason::EvidenceReceivedAfterCutoff);
    }
    if item.first_received_at_tick < item.measured_at_tick {
        push_reason(reasons, AbstainReason::EvidenceDependencyOrderInvalid);
    }
    if item.valid_from_tick > input.decision_tick {
        push_reason(reasons, AbstainReason::EvidenceNotYetValidAtCutoff);
    }
    if input.decision_tick > item.expires_at_tick {
        push_reason(reasons, AbstainReason::EvidenceExpiredAtCutoff);
    }
    if item.expires_at_tick < item.valid_from_tick {
        push_reason(reasons, AbstainReason::EvidenceDependencyOrderInvalid);
    }
    validate_lifecycle(
        &item.revocation,
        input.decision_tick,
        false,
        pending,
        reasons,
    );
    validate_lifecycle(
        &item.supersession,
        input.decision_tick,
        true,
        pending,
        reasons,
    );
}

pub fn compose_medium_decision(input: &CompositionInput) -> CompositionDecision {
    let mut reasons = Vec::new();
    if [
        input.identity.workpiece_id.as_str(),
        input.identity.joint_revision.as_str(),
        input.identity.layer_id.as_str(),
        input.identity.predecessor_pass_id.as_str(),
        input.clock_id.as_str(),
    ]
    .into_iter()
    .any(empty)
    {
        push_reason(&mut reasons, AbstainReason::EmptyIdentity);
    }
    if !valid_sha256(&input.evidence_snapshot_sha256)
        || !valid_sha256(&input.candidate_set_sha256)
        || !valid_sha256(&input.planner_build_sha256)
    {
        push_reason(&mut reasons, AbstainReason::InvalidDigest);
    }
    if input.purpose != DecisionPurpose::MediumNextPass {
        push_reason(&mut reasons, AbstainReason::WrongDecisionPurpose);
    }
    if input.automatic_selection_requested
        || input.live_execution_requested
        || input.wps_change_requested
        || input.accepted_as_built_write_requested
    {
        push_reason(&mut reasons, AbstainReason::UnauthorizedActionRequested);
    }

    let mut ids = HashSet::new();
    for event in &input.evidence {
        if !ids.insert(event.evidence_id.as_str()) {
            push_reason(&mut reasons, AbstainReason::DuplicateEvidenceId);
        }
    }
    if !reasons.is_empty() {
        return CompositionDecision::Abstain(reasons);
    }

    let calibration = match one(&input.evidence, EvidenceKind::Calibration) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };
    let registration = match one(&input.evidence, EvidenceKind::Registration) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };
    let qualification_event = match one(&input.evidence, EvidenceKind::ScanQualification) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };
    let geometry_event = match one(&input.evidence, EvidenceKind::GeometryComparison) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };
    let reference_scan = match one(&input.evidence, EvidenceKind::ReferenceScan) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };
    let observed_scan = match one(&input.evidence, EvidenceKind::ObservedScan) {
        Ok(value) => value,
        Err(reason) => return CompositionDecision::Abstain(vec![reason]),
    };

    let required_events = [
        calibration,
        reference_scan,
        observed_scan,
        registration,
        qualification_event,
        geometry_event,
    ];
    let mut expected_feature_ids: Vec<String> = required_events
        .iter()
        .map(|event| event.evidence_id.clone())
        .collect();
    expected_feature_ids.sort();
    let mut actual_feature_ids = input.medium_feature_event_ids.clone();
    actual_feature_ids.sort();
    let has_duplicate_features = actual_feature_ids.windows(2).any(|pair| pair[0] == pair[1]);
    if has_duplicate_features || actual_feature_ids != expected_feature_ids {
        let forbidden = input.medium_feature_event_ids.iter().any(|id| {
            input.evidence.iter().any(|event| {
                event.evidence_id == *id
                    && matches!(
                        event.kind,
                        EvidenceKind::NdtOutcome
                            | EvidenceKind::RepairDisposition
                            | EvidenceKind::LearningArtifact
                    )
            })
        });
        push_reason(
            &mut reasons,
            if forbidden {
                AbstainReason::ForbiddenMediumEvidence
            } else {
                AbstainReason::MediumFeatureSetMismatch
            },
        );
    }

    let mut pending = Vec::new();
    for event in required_events {
        validate_envelope(event, input, &mut pending, &mut reasons);
    }

    let qualification = &input.scan_qualification;
    if qualification.identity != input.identity
        || qualification.observed_scan_id != observed_scan.evidence_id
        || qualification.observed_scan_sha256 != observed_scan.content_sha256
        || qualification.calibration_receipt_sha256 != calibration.content_sha256
        || qualification.registration_receipt_sha256 != registration.content_sha256
        || qualification.receipt_sha256 != qualification_event.content_sha256
        || empty(&qualification.policy_id)
        || empty(&qualification.policy_revision)
    {
        push_reason(
            &mut reasons,
            AbstainReason::ScanQualificationReceiptMismatch,
        );
    }
    if !valid_sha256(&qualification.receipt_sha256)
        || !valid_sha256(&qualification.observed_scan_sha256)
        || !valid_sha256(&qualification.calibration_receipt_sha256)
        || !valid_sha256(&qualification.registration_receipt_sha256)
    {
        push_reason(&mut reasons, AbstainReason::InvalidDigest);
    }
    if empty(&qualification.coordinate_frame)
        || qualification.coordinate_frame != input.geometry_comparison.coordinate_frame
    {
        push_reason(&mut reasons, AbstainReason::CoordinateFrameMismatch);
    }
    if qualification.maximum_expanded_length_uncertainty_units == 0
        || qualification.length_decision_tolerance_units == 0
    {
        push_reason(&mut reasons, AbstainReason::InvalidUncertaintyLimit);
    }
    if qualification.expanded_length_uncertainty_units
        > qualification.maximum_expanded_length_uncertainty_units
        || qualification.expanded_length_uncertainty_units
            > qualification.length_decision_tolerance_units
    {
        push_reason(&mut reasons, AbstainReason::LengthUncertaintyAboveLimit);
    }

    let geometry = &input.geometry_comparison;
    if geometry.identity != input.identity
        || geometry.output_sha256 != geometry_event.content_sha256
        || geometry.reference_scan_id != reference_scan.evidence_id
        || geometry.reference_scan_sha256 != reference_scan.content_sha256
        || geometry.observed_scan_id != observed_scan.evidence_id
        || geometry.observed_scan_sha256 != observed_scan.content_sha256
        || geometry.registration_receipt_sha256 != registration.content_sha256
        || !valid_sha256(&geometry.output_sha256)
        || !valid_sha256(&geometry.reference_scan_sha256)
        || !valid_sha256(&geometry.observed_scan_sha256)
        || !valid_sha256(&geometry.registration_receipt_sha256)
        || !valid_sha256(&geometry.x_grid_sha256)
    {
        push_reason(&mut reasons, AbstainReason::GeometryReceiptMismatch);
    }
    if geometry.reference_scan_id == geometry.observed_scan_id
        || geometry.reference_scan_sha256 == geometry.observed_scan_sha256
    {
        push_reason(
            &mut reasons,
            AbstainReason::ReferenceAndObservedScanMustDiffer,
        );
    }
    if empty(&geometry.coordinate_frame) {
        push_reason(&mut reasons, AbstainReason::EmptyIdentity);
    }
    if geometry.length_unit != "mm" {
        push_reason(&mut reasons, AbstainReason::CoordinateUnitMismatch);
    }
    if geometry.coverage_permille > 1_000
        || geometry.minimum_coverage_permille > 1_000
        || geometry.coverage_permille < geometry.minimum_coverage_permille
    {
        push_reason(&mut reasons, AbstainReason::InvalidCoverage);
    }
    if geometry.largest_unsupported_gap_units > geometry.maximum_unsupported_gap_units {
        push_reason(&mut reasons, AbstainReason::UnsupportedGapAboveLimit);
    }
    if geometry.maximum_area_uncertainty_units == 0 {
        push_reason(&mut reasons, AbstainReason::InvalidUncertaintyLimit);
    }
    if geometry.area_uncertainty_units > geometry.maximum_area_uncertainty_units {
        push_reason(&mut reasons, AbstainReason::AreaUncertaintyAboveLimit);
    }

    // Keep length and area uncertainty dimensions separate. Validate the
    // signed/added/removed area relation using checked integer conversion.
    let signed_added = i128::try_from(geometry.added_area_units);
    let signed_removed = i128::try_from(geometry.removed_area_units);
    let expected_net = match (signed_added, signed_removed) {
        (Ok(added), Ok(removed)) => added.checked_sub(removed),
        _ => None,
    };
    match expected_net {
        Some(value) if value == geometry.signed_net_area_units => {}
        None => push_reason(&mut reasons, AbstainReason::GeometryAreaArithmeticOverflow),
        Some(_) => push_reason(&mut reasons, AbstainReason::GeometrySignedAreaMismatch),
    }

    // Derived receipts must not predate the evidence they claim to derive from.
    if qualification_event.measured_at_tick < observed_scan.measured_at_tick
        || qualification_event.measured_at_tick < registration.measured_at_tick
        || qualification_event.measured_at_tick < calibration.measured_at_tick
        || qualification_event.first_received_at_tick < observed_scan.first_received_at_tick
        || qualification_event.first_received_at_tick < registration.first_received_at_tick
        || qualification_event.first_received_at_tick < calibration.first_received_at_tick
        || geometry_event.measured_at_tick < reference_scan.measured_at_tick
        || geometry_event.measured_at_tick < observed_scan.measured_at_tick
        || geometry_event.measured_at_tick < registration.measured_at_tick
        || geometry_event.first_received_at_tick < reference_scan.first_received_at_tick
        || geometry_event.first_received_at_tick < observed_scan.first_received_at_tick
        || geometry_event.first_received_at_tick < registration.first_received_at_tick
    {
        push_reason(&mut reasons, AbstainReason::EvidenceDependencyOrderInvalid);
    }

    if !reasons.is_empty() {
        return CompositionDecision::Abstain(reasons);
    }

    let mut accepted_evidence: Vec<AcceptedEvidenceRef> = required_events
        .iter()
        .map(|event| AcceptedEvidenceRef {
            kind: event.kind,
            evidence_id: event.evidence_id.clone(),
            content_sha256: event.content_sha256.clone(),
            measured_at_tick: event.measured_at_tick,
            first_received_at_tick: event.first_received_at_tick,
        })
        .collect();
    accepted_evidence.sort();
    pending.sort();
    pending.dedup();

    CompositionDecision::Composed(Box::new(ComposedDecisionReceipt {
        identity: input.identity.clone(),
        decision_tick: input.decision_tick,
        clock_id: input.clock_id.clone(),
        purpose: input.purpose,
        accepted_evidence,
        pending_lifecycle_events_known_at_cutoff: pending,
        evidence_snapshot_sha256: input.evidence_snapshot_sha256.clone(),
        candidate_set_sha256: input.candidate_set_sha256.clone(),
        planner_build_sha256: input.planner_build_sha256.clone(),
        scan_qualification_sha256: qualification.receipt_sha256.clone(),
        geometry_comparison_sha256: geometry.output_sha256.clone(),
        reference_scan_sha256: geometry.reference_scan_sha256.clone(),
        observed_scan_sha256: geometry.observed_scan_sha256.clone(),
        calibration_receipt_sha256: qualification.calibration_receipt_sha256.clone(),
        registration_receipt_sha256: geometry.registration_receipt_sha256.clone(),
        expanded_length_uncertainty_units: qualification.expanded_length_uncertainty_units,
        length_decision_tolerance_units: qualification.length_decision_tolerance_units,
        signed_net_area_units: geometry.signed_net_area_units,
        added_area_units: geometry.added_area_units,
        removed_area_units: geometry.removed_area_units,
        area_uncertainty_units: geometry.area_uncertainty_units,
        claim_ceiling: "SYNTHETIC_POINT_IN_TIME_METROLOGY_COMPOSITION_ONLY",
        automatic_selection: false,
        live_execution_authorized: false,
        wps_change_authorized: false,
        accepted_as_built_write_authorized: false,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn identity() -> Identity {
        Identity {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R3".to_owned(),
            layer_id: "LAYER-04".to_owned(),
            predecessor_pass_id: "PASS-03".to_owned(),
        }
    }

    fn envelope(
        kind: EvidenceKind,
        id: &str,
        hash: String,
        measured: u64,
        received: u64,
    ) -> EvidenceEnvelope {
        EvidenceEnvelope {
            kind,
            evidence_id: id.to_owned(),
            content_sha256: hash,
            identity: identity(),
            evidence_class: EvidenceClass::SyntheticContractOnly,
            clock_id: "SYNTH-CLOCK-01".to_owned(),
            measured_at_tick: measured,
            first_received_at_tick: received,
            valid_from_tick: 1,
            expires_at_tick: 200,
            purpose: DecisionPurpose::MediumNextPass,
            authorized_for_medium_next_pass: true,
            revocation: None,
            supersession: None,
        }
    }

    fn base_input() -> CompositionInput {
        let calibration_hash = digest('a');
        let reference_hash = digest('b');
        let observed_hash = digest('c');
        let registration_hash = digest('d');
        let qualification_hash = digest('e');
        let geometry_hash = digest('f');

        let calibration = envelope(
            EvidenceKind::Calibration,
            "CAL-01",
            calibration_hash.clone(),
            10,
            11,
        );
        let reference = envelope(
            EvidenceKind::ReferenceScan,
            "SCAN-REF",
            reference_hash.clone(),
            20,
            21,
        );
        let observed = envelope(
            EvidenceKind::ObservedScan,
            "SCAN-AFTER-PASS-03",
            observed_hash.clone(),
            30,
            31,
        );
        let registration = envelope(
            EvidenceKind::Registration,
            "REG-01",
            registration_hash.clone(),
            32,
            33,
        );
        let qualification = envelope(
            EvidenceKind::ScanQualification,
            "QUAL-01",
            qualification_hash.clone(),
            34,
            35,
        );
        let geometry = envelope(
            EvidenceKind::GeometryComparison,
            "GEOM-01",
            geometry_hash.clone(),
            36,
            37,
        );

        CompositionInput {
            identity: identity(),
            decision_tick: 40,
            clock_id: "SYNTH-CLOCK-01".to_owned(),
            purpose: DecisionPurpose::MediumNextPass,
            evidence_snapshot_sha256: digest('1'),
            candidate_set_sha256: digest('2'),
            planner_build_sha256: digest('3'),
            evidence: vec![
                calibration,
                reference,
                observed,
                registration,
                qualification,
                geometry,
            ],
            medium_feature_event_ids: vec![
                "CAL-01".to_owned(),
                "SCAN-REF".to_owned(),
                "SCAN-AFTER-PASS-03".to_owned(),
                "REG-01".to_owned(),
                "QUAL-01".to_owned(),
                "GEOM-01".to_owned(),
            ],
            scan_qualification: ScanQualificationReceipt {
                receipt_sha256: qualification_hash,
                identity: identity(),
                observed_scan_id: "SCAN-AFTER-PASS-03".to_owned(),
                observed_scan_sha256: observed_hash.clone(),
                calibration_receipt_sha256: calibration_hash,
                registration_receipt_sha256: registration_hash.clone(),
                coordinate_frame: "WELD-FRAME-01".to_owned(),
                policy_id: "SCAN-POLICY-01".to_owned(),
                policy_revision: "R2".to_owned(),
                expanded_length_uncertainty_units: 3,
                maximum_expanded_length_uncertainty_units: 5,
                length_decision_tolerance_units: 4,
            },
            geometry_comparison: GeometryComparisonReceipt {
                output_sha256: geometry_hash,
                identity: identity(),
                reference_scan_id: "SCAN-REF".to_owned(),
                reference_scan_sha256: reference_hash,
                observed_scan_id: "SCAN-AFTER-PASS-03".to_owned(),
                observed_scan_sha256: observed_hash,
                registration_receipt_sha256: registration_hash,
                coordinate_frame: "WELD-FRAME-01".to_owned(),
                length_unit: "mm".to_owned(),
                x_grid_sha256: digest('4'),
                coverage_permille: 995,
                minimum_coverage_permille: 980,
                maximum_unsupported_gap_units: 2,
                largest_unsupported_gap_units: 1,
                signed_net_area_units: 12,
                added_area_units: 15,
                removed_area_units: 3,
                area_uncertainty_units: 2,
                maximum_area_uncertainty_units: 5,
            },
            automatic_selection_requested: false,
            live_execution_requested: false,
            wps_change_requested: false,
            accepted_as_built_write_requested: false,
        }
    }

    fn reasons(input: &CompositionInput) -> Vec<AbstainReason> {
        match compose_medium_decision(input) {
            CompositionDecision::Abstain(reasons) => reasons,
            CompositionDecision::Composed(_) => panic!("expected composition abstention"),
        }
    }

    #[test]
    fn exact_synthetic_evidence_chain_composes() {
        let CompositionDecision::Composed(receipt) = compose_medium_decision(&base_input()) else {
            panic!("the frozen positive composition case should pass");
        };
        assert_eq!(receipt.accepted_evidence.len(), 6);
        assert_eq!(receipt.signed_net_area_units, 12);
        assert_eq!(receipt.expanded_length_uncertainty_units, 3);
        assert!(!receipt.automatic_selection);
        assert!(!receipt.live_execution_authorized);
        assert!(!receipt.wps_change_authorized);
        assert!(!receipt.accepted_as_built_write_authorized);
    }

    #[test]
    fn input_event_permutation_does_not_change_canonical_receipt() {
        let mut input = base_input();
        let CompositionDecision::Composed(original) = compose_medium_decision(&input) else {
            panic!("baseline must compose");
        };
        input.evidence.reverse();
        let CompositionDecision::Composed(permuted) = compose_medium_decision(&input) else {
            panic!("permuted baseline must compose");
        };
        assert_eq!(original, permuted);
    }

    #[test]
    fn workpiece_mismatch_abstains() {
        let mut input = base_input();
        input.evidence[0].identity.workpiece_id = "WID-OTHER".to_owned();
        assert!(reasons(&input).contains(&AbstainReason::EvidenceIdentityMismatch));
    }

    #[test]
    fn joint_revision_mismatch_abstains() {
        let mut input = base_input();
        input.geometry_comparison.identity.joint_revision = "JOINT-R4".to_owned();
        assert!(reasons(&input).contains(&AbstainReason::GeometryReceiptMismatch));
    }

    #[test]
    fn predecessor_pass_mismatch_abstains() {
        let mut input = base_input();
        input.scan_qualification.identity.predecessor_pass_id = "PASS-02".to_owned();
        assert!(reasons(&input).contains(&AbstainReason::ScanQualificationReceiptMismatch));
    }

    #[test]
    fn calibration_digest_mismatch_abstains() {
        let mut input = base_input();
        input.scan_qualification.calibration_receipt_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::ScanQualificationReceiptMismatch));
    }

    #[test]
    fn registration_digest_mismatch_abstains() {
        let mut input = base_input();
        input.geometry_comparison.registration_receipt_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::GeometryReceiptMismatch));
    }

    #[test]
    fn reference_scan_mismatch_abstains() {
        let mut input = base_input();
        input.geometry_comparison.reference_scan_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::GeometryReceiptMismatch));
    }

    #[test]
    fn observed_scan_mismatch_abstains() {
        let mut input = base_input();
        input.scan_qualification.observed_scan_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::ScanQualificationReceiptMismatch));
    }

    #[test]
    fn scan_qualification_output_digest_mismatch_abstains() {
        let mut input = base_input();
        input.scan_qualification.receipt_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::ScanQualificationReceiptMismatch));
    }

    #[test]
    fn geometry_output_digest_mismatch_abstains() {
        let mut input = base_input();
        input.geometry_comparison.output_sha256 = digest('9');
        assert!(reasons(&input).contains(&AbstainReason::GeometryReceiptMismatch));
    }

    #[test]
    fn insufficient_geometry_coverage_abstains() {
        let mut input = base_input();
        input.geometry_comparison.coverage_permille = 970;
        assert!(reasons(&input).contains(&AbstainReason::InvalidCoverage));
    }

    #[test]
    fn excessive_unsupported_gap_abstains() {
        let mut input = base_input();
        input.geometry_comparison.largest_unsupported_gap_units = 3;
        assert!(reasons(&input).contains(&AbstainReason::UnsupportedGapAboveLimit));
    }

    #[test]
    fn excessive_length_uncertainty_abstains() {
        let mut input = base_input();
        input.scan_qualification.expanded_length_uncertainty_units = 5;
        assert!(reasons(&input).contains(&AbstainReason::LengthUncertaintyAboveLimit));
    }

    #[test]
    fn excessive_area_uncertainty_abstains() {
        let mut input = base_input();
        input.geometry_comparison.area_uncertainty_units = 6;
        assert!(reasons(&input).contains(&AbstainReason::AreaUncertaintyAboveLimit));
    }

    #[test]
    fn length_and_area_uncertainty_are_checked_in_separate_dimensions() {
        let mut input = base_input();
        input.scan_qualification.expanded_length_uncertainty_units = 4;
        input.scan_qualification.length_decision_tolerance_units = 4;
        input.geometry_comparison.area_uncertainty_units = 5;
        input.geometry_comparison.maximum_area_uncertainty_units = 5;
        assert!(matches!(
            compose_medium_decision(&input),
            CompositionDecision::Composed(_)
        ));
    }

    #[test]
    fn ndt_outcome_cannot_be_used_as_a_medium_loop_feature() {
        let mut input = base_input();
        let mut ndt = envelope(EvidenceKind::NdtOutcome, "NDT-01", digest('7'), 38, 39);
        ndt.evidence_class = EvidenceClass::Inspected;
        ndt.purpose = DecisionPurpose::SlowQualityRequalification;
        ndt.authorized_for_medium_next_pass = false;
        input.evidence.push(ndt);
        input.medium_feature_event_ids.push("NDT-01".to_owned());
        assert!(reasons(&input).contains(&AbstainReason::ForbiddenMediumEvidence));
    }

    #[test]
    fn evidence_received_after_cutoff_cannot_backfill_historical_decision() {
        let mut input = base_input();
        input.evidence[2].first_received_at_tick = 41;
        assert!(reasons(&input).contains(&AbstainReason::EvidenceReceivedAfterCutoff));
    }

    #[test]
    fn future_measurement_abstains() {
        let mut input = base_input();
        input.evidence[2].measured_at_tick = 41;
        input.evidence[2].first_received_at_tick = 42;
        assert!(reasons(&input).contains(&AbstainReason::MeasurementFromFuture));
    }

    #[test]
    fn expiry_at_cutoff_is_enforced() {
        let mut input = base_input();
        input.evidence[0].expires_at_tick = 39;
        assert!(reasons(&input).contains(&AbstainReason::EvidenceExpiredAtCutoff));
    }

    #[test]
    fn revocation_known_before_cutoff_abstains() {
        let mut input = base_input();
        input.evidence[2].revocation = Some(LifecycleMarker {
            event_id: "REV-01".to_owned(),
            content_sha256: digest('8'),
            effective_at_tick: 35,
            first_received_at_tick: 36,
        });
        assert!(reasons(&input).contains(&AbstainReason::EvidenceRevokedAtCutoff));
    }

    #[test]
    fn late_revocation_does_not_rewrite_historical_cutoff() {
        let mut input = base_input();
        input.evidence[2].revocation = Some(LifecycleMarker {
            event_id: "REV-LATE".to_owned(),
            content_sha256: digest('8'),
            effective_at_tick: 35,
            first_received_at_tick: 41,
        });
        assert!(matches!(
            compose_medium_decision(&input),
            CompositionDecision::Composed(_)
        ));
    }

    #[test]
    fn supersession_known_before_cutoff_abstains() {
        let mut input = base_input();
        input.evidence[3].supersession = Some(LifecycleMarker {
            event_id: "SUPER-01".to_owned(),
            content_sha256: digest('8'),
            effective_at_tick: 39,
            first_received_at_tick: 39,
        });
        assert!(reasons(&input).contains(&AbstainReason::EvidenceSupersededAtCutoff));
    }

    #[test]
    fn late_supersession_does_not_rewrite_historical_cutoff() {
        let mut input = base_input();
        input.evidence[3].supersession = Some(LifecycleMarker {
            event_id: "SUPER-LATE".to_owned(),
            content_sha256: digest('8'),
            effective_at_tick: 39,
            first_received_at_tick: 42,
        });
        assert!(matches!(
            compose_medium_decision(&input),
            CompositionDecision::Composed(_)
        ));
    }

    #[test]
    fn duplicate_evidence_identity_abstains() {
        let mut input = base_input();
        input.evidence[1].evidence_id = input.evidence[0].evidence_id.clone();
        assert!(reasons(&input).contains(&AbstainReason::DuplicateEvidenceId));
    }

    #[test]
    fn missing_digest_abstains() {
        let mut input = base_input();
        input.evidence[2].content_sha256 = "missing".to_owned();
        assert!(reasons(&input).contains(&AbstainReason::InvalidDigest));
    }

    #[test]
    fn live_execution_or_acceptance_write_is_never_authorized() {
        let mut input = base_input();
        input.live_execution_requested = true;
        assert!(reasons(&input).contains(&AbstainReason::UnauthorizedActionRequested));
    }

    #[test]
    fn derived_receipt_cannot_predate_its_inputs() {
        let mut input = base_input();
        input.geometry_comparison.identity.layer_id = "LAYER-05".to_owned();
        input.evidence[5].measured_at_tick = 15;
        assert!(reasons(&input).contains(&AbstainReason::EvidenceDependencyOrderInvalid));
    }

    #[test]
    fn zero_uncertainty_limits_are_invalid() {
        let mut input = base_input();
        input.scan_qualification.length_decision_tolerance_units = 0;
        assert!(reasons(&input).contains(&AbstainReason::InvalidUncertaintyLimit));
    }

    #[test]
    fn known_but_not_yet_effective_revocation_is_preserved() {
        let mut input = base_input();
        input.evidence[2].revocation = Some(LifecycleMarker {
            event_id: "REV-FUTURE".to_owned(),
            content_sha256: digest('8'),
            effective_at_tick: 60,
            first_received_at_tick: 39,
        });
        let CompositionDecision::Composed(receipt) = compose_medium_decision(&input) else {
            panic!("future-effective revocation known at cutoff should be preserved");
        };
        assert_eq!(receipt.pending_lifecycle_events_known_at_cutoff.len(), 1);
        assert!(!receipt.pending_lifecycle_events_known_at_cutoff[0].is_supersession);
    }

    #[test]
    fn qualification_and_geometry_frame_mismatch_abstains() {
        let mut input = base_input();
        input.geometry_comparison.coordinate_frame = "OTHER-FRAME".to_owned();
        assert!(reasons(&input).contains(&AbstainReason::CoordinateFrameMismatch));
    }
}
