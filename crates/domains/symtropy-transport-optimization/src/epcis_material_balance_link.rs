// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Loss-aware handoff from an EPCIS TransformationEvent group to the existing
//! material-accounted-routes MassBalance receipt contract.
//!
//! This module does not calculate mass, yield, conversion, residue or process
//! closure. It validates exact references and status semantics from the
//! independent owner receipt. The upstream material-accounting owner retains
//! all physics, quantity/unit, uncertainty and process-boundary calculations.
//!
//! A successful result means only that the MassBalance gate receipt is
//! correctly bound and reports EXACT_BALANCED. It is not a full route
//! disposition, proof of physical transformation, product acceptance, or
//! release authority. Receipt digests are shape/reference checked here, not
//! cryptographically authenticated.

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransformationSourceEventRef {
    pub source_id: String,
    pub source_event_id: String,
    pub revision: u32,
    /// Lowercase SHA-256 shape of the immutable source event payload.
    pub source_payload_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransformationGroupEvidence {
    /// Stable group reference consumed by the external material-accounting case.
    pub group_ref: String,
    /// Present for a shared EPCIS transformationID group.
    /// Absent only for an event-local transformation with exactly one event.
    pub transformation_id: Option<String>,
    /// Effective, non-void source event revisions selected at the same as-of
    /// cutoff. This is not a claim that the physical process is complete.
    pub effective_source_events: Vec<TransformationSourceEventRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialBalanceCaseBinding {
    /// Exact case identity in the material-accounted-routes Stage 0 manifest.
    pub case_id: String,
    pub manifest_ref: String,
    /// Must refer to this exact EPCIS transformation group.
    pub group_ref: String,
    pub transformation_id: Option<String>,
    /// Exact process identity used by the owner mass-balance gate.
    pub process_ref: String,
    /// Exact input lot set / scope used by the owner mass-balance gate.
    pub input_lot_set_ref: String,
    /// Exact declared output scope. This adapter does not infer or expand it.
    pub output_ref: String,
    /// Expected immutable owner evidence digest for this exact process + lot set.
    pub expected_receipt_source_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialBalanceOwnerReceipt {
    pub case_id: String,
    pub manifest_ref: String,
    pub gate: String,
    pub receipt_ref: String,
    pub subject_ref: String,
    pub scope_ref: String,
    pub source_digest: String,
    pub status: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MaterialBalanceLinkDisposition {
    /// Only the MassBalance gate passed; other route gates remain unevaluated.
    MassBalanceGatePass,
    Indeterminate,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerDigestQualification {
    /// The digest was not evaluated because the receipt or its bindings failed first.
    NotEvaluated,
    /// A well-formed SHA-256 string matched the expected reference.
    /// This is not a signature check or proof that the bytes were retrieved.
    ShapeAndReferenceMatchedOnly,
    /// A well-formed digest did not match the exact digest pinned by the case.
    ShapeValidReferenceMismatch,
    Malformed,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MaterialBalanceLinkReason {
    MissingTransformationGroupRef,
    InvalidTransformationId,
    EmptyEffectiveEventSet,
    EventLocalGroupHasMultipleEvents,
    DuplicateSourceEventRevision,
    ConflictingSourceEventRevision,
    InvalidSourcePayloadDigest,
    InvalidSourceEventIdentity,
    MissingCaseBinding(&'static str),
    TransformationGroupBindingMismatch,
    TransformationIdBindingMismatch,
    InvalidExpectedReceiptDigest,
    MissingOwnerReceipt,
    MultipleOwnerReceipts,
    WrongOwnerGate,
    OwnerCaseBindingMismatch,
    OwnerManifestBindingMismatch,
    OwnerSubjectBindingMismatch,
    OwnerScopeBindingMismatch,
    OwnerReceiptDigestMissing,
    OwnerReceiptDigestMalformed,
    OwnerReceiptDigestMismatch,
    OwnerStatusIndeterminate,
    OwnerStatusBlocked,
    UnsupportedOwnerStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MaterialBalanceLinkDecision {
    pub disposition: MaterialBalanceLinkDisposition,
    pub reason: Option<MaterialBalanceLinkReason>,
    pub owner_receipt_ref: Option<String>,
    pub source_event_count: usize,
    pub digest_qualification: OwnerDigestQualification,
    /// Explicitly prevents this gate-only result being read as route acceptance.
    pub claim_ceiling: &'static str,
}

impl MaterialBalanceLinkDecision {
    fn new(
        disposition: MaterialBalanceLinkDisposition,
        reason: Option<MaterialBalanceLinkReason>,
        owner_receipt_ref: Option<String>,
        source_event_count: usize,
        digest_qualification: OwnerDigestQualification,
    ) -> Self {
        Self {
            disposition,
            reason,
            owner_receipt_ref,
            source_event_count,
            digest_qualification,
            claim_ceiling: "MASS_BALANCE_GATE_ONLY_NO_PHYSICAL_OR_RELEASE_AUTHORITY",
        }
    }

    fn blocked(
        reason: MaterialBalanceLinkReason,
        owner_receipt_ref: Option<String>,
        source_event_count: usize,
    ) -> Self {
        Self::blocked_with_digest(
            reason,
            owner_receipt_ref,
            source_event_count,
            OwnerDigestQualification::NotEvaluated,
        )
    }

    fn blocked_with_digest(
        reason: MaterialBalanceLinkReason,
        owner_receipt_ref: Option<String>,
        source_event_count: usize,
        digest_qualification: OwnerDigestQualification,
    ) -> Self {
        Self::new(
            MaterialBalanceLinkDisposition::Blocked,
            Some(reason),
            owner_receipt_ref,
            source_event_count,
            digest_qualification,
        )
    }

    fn indeterminate(
        reason: MaterialBalanceLinkReason,
        owner_receipt_ref: Option<String>,
        source_event_count: usize,
        digest_qualification: OwnerDigestQualification,
    ) -> Self {
        Self::new(
            MaterialBalanceLinkDisposition::Indeterminate,
            Some(reason),
            owner_receipt_ref,
            source_event_count,
            digest_qualification,
        )
    }
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_group(group: &TransformationGroupEvidence) -> Option<MaterialBalanceLinkReason> {
    if !valid_text(&group.group_ref) {
        return Some(MaterialBalanceLinkReason::MissingTransformationGroupRef);
    }
    if group
        .transformation_id
        .as_ref()
        .is_some_and(|id| !valid_text(id))
    {
        return Some(MaterialBalanceLinkReason::InvalidTransformationId);
    }
    if group.effective_source_events.is_empty() {
        return Some(MaterialBalanceLinkReason::EmptyEffectiveEventSet);
    }
    if group.transformation_id.is_none() && group.effective_source_events.len() != 1 {
        return Some(MaterialBalanceLinkReason::EventLocalGroupHasMultipleEvents);
    }

    let mut event_revisions = std::collections::BTreeMap::new();
    for event in &group.effective_source_events {
        if !valid_text(&event.source_id) || !valid_text(&event.source_event_id) {
            return Some(MaterialBalanceLinkReason::InvalidSourceEventIdentity);
        }
        if event.revision == 0 {
            return Some(MaterialBalanceLinkReason::InvalidSourceEventIdentity);
        }
        if !valid_sha256(&event.source_payload_sha256) {
            return Some(MaterialBalanceLinkReason::InvalidSourcePayloadDigest);
        }
        let key = (event.source_id.as_str(), event.source_event_id.as_str());
        match event_revisions.insert(key, event.revision) {
            Some(previous_revision) if previous_revision == event.revision => {
                return Some(MaterialBalanceLinkReason::DuplicateSourceEventRevision);
            }
            Some(_) => {
                return Some(MaterialBalanceLinkReason::ConflictingSourceEventRevision);
            }
            None => {}
        }
    }
    None
}

fn validate_case_binding(
    group: &TransformationGroupEvidence,
    case: &MaterialBalanceCaseBinding,
) -> Option<MaterialBalanceLinkReason> {
    for (value, field) in [
        (case.case_id.as_str(), "case_id"),
        (case.manifest_ref.as_str(), "manifest_ref"),
        (case.group_ref.as_str(), "group_ref"),
        (case.process_ref.as_str(), "process_ref"),
        (case.input_lot_set_ref.as_str(), "input_lot_set_ref"),
        (case.output_ref.as_str(), "output_ref"),
    ] {
        if !valid_text(value) {
            return Some(MaterialBalanceLinkReason::MissingCaseBinding(field));
        }
    }
    if case.group_ref != group.group_ref {
        return Some(MaterialBalanceLinkReason::TransformationGroupBindingMismatch);
    }
    if case.transformation_id != group.transformation_id {
        return Some(MaterialBalanceLinkReason::TransformationIdBindingMismatch);
    }
    if !valid_sha256(&case.expected_receipt_source_digest) {
        return Some(MaterialBalanceLinkReason::InvalidExpectedReceiptDigest);
    }
    None
}

/// Link one normalized EPCIS transformation group to exactly one independent
/// material-accounting MassBalance gate receipt.
///
/// The owner contract corresponds to MATERIAL-ACCOUNTED-ROUTES-001A #5879.
/// The accepted owner status is EXACT_BALANCED. POSSIBLE_WITH_UNCERTAINTY,
/// UNKNOWN and MISSING abstain; EXACT_UNBALANCED, IMPOSSIBLE_WITHIN_BOUNDS,
/// INVALID and unknown status strings block. This function does not reproduce
/// the owner's mass-balance arithmetic or decide the other seven route gates.
pub fn qualify_transformation_mass_balance_link(
    group: &TransformationGroupEvidence,
    case: &MaterialBalanceCaseBinding,
    receipts: &[MaterialBalanceOwnerReceipt],
) -> MaterialBalanceLinkDecision {
    let source_event_count = group.effective_source_events.len();
    if let Some(reason) = validate_group(group) {
        return MaterialBalanceLinkDecision::blocked(reason, None, source_event_count);
    }
    if let Some(reason) = validate_case_binding(group, case) {
        return MaterialBalanceLinkDecision::blocked(reason, None, source_event_count);
    }

    match receipts {
        [] => {
            return MaterialBalanceLinkDecision::indeterminate(
                MaterialBalanceLinkReason::MissingOwnerReceipt,
                None,
                source_event_count,
                OwnerDigestQualification::NotEvaluated,
            );
        }
        [receipt] => {
            if !valid_text(&receipt.receipt_ref)
                || !valid_text(&receipt.case_id)
                || !valid_text(&receipt.manifest_ref)
                || !valid_text(&receipt.gate)
                || !valid_text(&receipt.subject_ref)
                || !valid_text(&receipt.scope_ref)
                || !valid_text(&receipt.status)
            {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::OwnerCaseBindingMismatch,
                    Some(receipt.receipt_ref.clone()),
                    source_event_count,
                );
            }
            let receipt_ref = Some(receipt.receipt_ref.clone());

            if receipt.gate != "MASS_BALANCE" {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::WrongOwnerGate,
                    receipt_ref,
                    source_event_count,
                );
            }
            if receipt.case_id != case.case_id {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::OwnerCaseBindingMismatch,
                    receipt_ref,
                    source_event_count,
                );
            }
            if receipt.manifest_ref != case.manifest_ref {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::OwnerManifestBindingMismatch,
                    receipt_ref,
                    source_event_count,
                );
            }
            if receipt.subject_ref != case.process_ref {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::OwnerSubjectBindingMismatch,
                    receipt_ref,
                    source_event_count,
                );
            }
            if receipt.scope_ref != case.input_lot_set_ref {
                return MaterialBalanceLinkDecision::blocked(
                    MaterialBalanceLinkReason::OwnerScopeBindingMismatch,
                    receipt_ref,
                    source_event_count,
                );
            }
            if receipt.source_digest == "-" {
                return MaterialBalanceLinkDecision::indeterminate(
                    MaterialBalanceLinkReason::OwnerReceiptDigestMissing,
                    receipt_ref,
                    source_event_count,
                    OwnerDigestQualification::Missing,
                );
            }
            if !valid_sha256(&receipt.source_digest) {
                return MaterialBalanceLinkDecision::blocked_with_digest(
                    MaterialBalanceLinkReason::OwnerReceiptDigestMalformed,
                    receipt_ref,
                    source_event_count,
                    OwnerDigestQualification::Malformed,
                );
            }
            if receipt.source_digest != case.expected_receipt_source_digest {
                return MaterialBalanceLinkDecision::blocked_with_digest(
                    MaterialBalanceLinkReason::OwnerReceiptDigestMismatch,
                    receipt_ref,
                    source_event_count,
                    OwnerDigestQualification::ShapeValidReferenceMismatch,
                );
            }

            let digest_qualification = OwnerDigestQualification::ShapeAndReferenceMatchedOnly;
            match receipt.status.as_str() {
                "EXACT_BALANCED" => MaterialBalanceLinkDecision::new(
                    MaterialBalanceLinkDisposition::MassBalanceGatePass,
                    None,
                    receipt_ref,
                    source_event_count,
                    digest_qualification,
                ),
                "POSSIBLE_WITH_UNCERTAINTY" | "UNKNOWN" | "MISSING" => {
                    MaterialBalanceLinkDecision::indeterminate(
                        MaterialBalanceLinkReason::OwnerStatusIndeterminate,
                        receipt_ref,
                        source_event_count,
                        digest_qualification,
                    )
                }
                "EXACT_UNBALANCED" | "IMPOSSIBLE_WITHIN_BOUNDS" | "INVALID" => {
                    MaterialBalanceLinkDecision::blocked_with_digest(
                        MaterialBalanceLinkReason::OwnerStatusBlocked,
                        receipt_ref,
                        source_event_count,
                        digest_qualification,
                    )
                }
                _ => MaterialBalanceLinkDecision::blocked_with_digest(
                    MaterialBalanceLinkReason::UnsupportedOwnerStatus,
                    receipt_ref,
                    source_event_count,
                    digest_qualification,
                ),
            }
        }
        _ => MaterialBalanceLinkDecision::blocked(
            MaterialBalanceLinkReason::MultipleOwnerReceipts,
            None,
            source_event_count,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group() -> TransformationGroupEvidence {
        TransformationGroupEvidence {
            group_ref: "urn:example:transformation:batch-42".to_string(),
            transformation_id: Some("urn:example:transformation:batch-42".to_string()),
            effective_source_events: vec![
                TransformationSourceEventRef {
                    source_id: "WMS_A".to_string(),
                    source_event_id: "urn:uuid:input".to_string(),
                    revision: 2,
                    source_payload_sha256: "a".repeat(64),
                },
                TransformationSourceEventRef {
                    source_id: "WMS_B".to_string(),
                    source_event_id: "urn:uuid:output".to_string(),
                    revision: 1,
                    source_payload_sha256: "b".repeat(64),
                },
            ],
        }
    }

    fn case() -> MaterialBalanceCaseBinding {
        MaterialBalanceCaseBinding {
            case_id: "CASE-42".to_string(),
            manifest_ref: "urn:example:route-manifest:42".to_string(),
            group_ref: "urn:example:transformation:batch-42".to_string(),
            transformation_id: Some("urn:example:transformation:batch-42".to_string()),
            process_ref: "urn:example:process:42".to_string(),
            input_lot_set_ref: "urn:example:lot-set:42".to_string(),
            output_ref: "urn:example:output-set:42".to_string(),
            expected_receipt_source_digest: "c".repeat(64),
        }
    }

    fn receipt(status: &str) -> MaterialBalanceOwnerReceipt {
        MaterialBalanceOwnerReceipt {
            case_id: "CASE-42".to_string(),
            manifest_ref: "urn:example:route-manifest:42".to_string(),
            gate: "MASS_BALANCE".to_string(),
            receipt_ref: "urn:example:mass-balance-receipt:42".to_string(),
            subject_ref: "urn:example:process:42".to_string(),
            scope_ref: "urn:example:lot-set:42".to_string(),
            source_digest: "c".repeat(64),
            status: status.to_string(),
        }
    }

    fn evaluate(status: &str) -> MaterialBalanceLinkDecision {
        qualify_transformation_mass_balance_link(&group(), &case(), &[receipt(status)])
    }

    #[test]
    fn exact_balanced_owner_receipt_passes_only_the_mass_balance_gate() {
        let decision = evaluate("EXACT_BALANCED");
        assert_eq!(
            decision.disposition,
            MaterialBalanceLinkDisposition::MassBalanceGatePass
        );
        assert_eq!(decision.reason, None);
        assert_eq!(decision.source_event_count, 2);
        assert_eq!(
            decision.digest_qualification,
            OwnerDigestQualification::ShapeAndReferenceMatchedOnly
        );
        assert_eq!(
            decision.claim_ceiling,
            "MASS_BALANCE_GATE_ONLY_NO_PHYSICAL_OR_RELEASE_AUTHORITY"
        );
    }

    #[test]
    fn possible_balance_with_uncertainty_remains_indeterminate() {
        assert_eq!(
            evaluate("POSSIBLE_WITH_UNCERTAINTY").disposition,
            MaterialBalanceLinkDisposition::Indeterminate
        );
    }

    #[test]
    fn exact_unbalanced_and_impossible_statuses_block() {
        for status in ["EXACT_UNBALANCED", "IMPOSSIBLE_WITHIN_BOUNDS", "INVALID"] {
            assert_eq!(
                evaluate(status).disposition,
                MaterialBalanceLinkDisposition::Blocked
            );
        }
    }

    #[test]
    fn missing_owner_receipt_is_unknown_not_zero_or_balanced() {
        let decision = qualify_transformation_mass_balance_link(&group(), &case(), &[]);
        assert_eq!(
            decision.disposition,
            MaterialBalanceLinkDisposition::Indeterminate
        );
        assert_eq!(
            decision.reason,
            Some(MaterialBalanceLinkReason::MissingOwnerReceipt)
        );
    }

    #[test]
    fn more_than_one_owner_mass_balance_receipt_is_blocked() {
        let receipt = receipt("EXACT_BALANCED");
        let decision = qualify_transformation_mass_balance_link(
            &group(),
            &case(),
            &[receipt.clone(), receipt],
        );
        assert_eq!(
            decision.reason,
            Some(MaterialBalanceLinkReason::MultipleOwnerReceipts)
        );
    }

    #[test]
    fn wrong_gate_case_manifest_process_or_lot_scope_is_blocked() {
        let mut wrong_gate = receipt("EXACT_BALANCED");
        wrong_gate.gate = "UTILITY".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[wrong_gate]).reason,
            Some(MaterialBalanceLinkReason::WrongOwnerGate)
        );

        let mut wrong_case = receipt("EXACT_BALANCED");
        wrong_case.case_id = "OTHER".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[wrong_case]).reason,
            Some(MaterialBalanceLinkReason::OwnerCaseBindingMismatch)
        );

        let mut wrong_process = receipt("EXACT_BALANCED");
        wrong_process.subject_ref = "other-process".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[wrong_process]).reason,
            Some(MaterialBalanceLinkReason::OwnerSubjectBindingMismatch)
        );

        let mut wrong_lots = receipt("EXACT_BALANCED");
        wrong_lots.scope_ref = "other-lot-set".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[wrong_lots]).reason,
            Some(MaterialBalanceLinkReason::OwnerScopeBindingMismatch)
        );
    }

    #[test]
    fn source_group_identity_and_transformation_id_must_match_owner_case() {
        let mut case = case();
        case.group_ref = "wrong-group".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case, &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::TransformationGroupBindingMismatch)
        );

        let mut case = case();
        case.transformation_id = Some("wrong-id".to_string());
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case, &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::TransformationIdBindingMismatch)
        );
    }

    #[test]
    fn missing_or_malformed_receipt_digest_never_passes() {
        let mut missing = receipt("EXACT_BALANCED");
        missing.source_digest = "-".to_string();
        let decision = qualify_transformation_mass_balance_link(&group(), &case(), &[missing]);
        assert_eq!(
            decision.disposition,
            MaterialBalanceLinkDisposition::Indeterminate
        );
        assert_eq!(
            decision.digest_qualification,
            OwnerDigestQualification::Missing
        );

        let mut malformed = receipt("EXACT_BALANCED");
        malformed.source_digest = "not-a-digest".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[malformed]).reason,
            Some(MaterialBalanceLinkReason::OwnerReceiptDigestMalformed)
        );
    }

    #[test]
    fn digest_must_match_the_exact_owner_receipt_reference() {
        let mut mismatched = receipt("EXACT_BALANCED");
        mismatched.source_digest = "d".repeat(64);
        assert_eq!(
            qualify_transformation_mass_balance_link(&group(), &case(), &[mismatched]).reason,
            Some(MaterialBalanceLinkReason::OwnerReceiptDigestMismatch)
        );
    }

    #[test]
    fn transformation_event_source_payload_digests_and_revisions_are_required() {
        let mut bad = group();
        bad.effective_source_events[0].source_payload_sha256 = "bad".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&bad, &case(), &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::InvalidSourcePayloadDigest)
        );

        let mut bad = group();
        bad.effective_source_events[0].revision = 0;
        assert_eq!(
            qualify_transformation_mass_balance_link(&bad, &case(), &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::InvalidSourceEventIdentity)
        );
    }

    #[test]
    fn one_event_local_transformation_is_valid_but_multi_event_local_group_is_not() {
        let mut event_local = group();
        event_local.transformation_id = None;
        event_local.group_ref = "event-local:WMS_A:urn:uuid:input:2".to_string();
        event_local.effective_source_events.truncate(1);
        let mut case = case();
        case.transformation_id = None;
        case.group_ref = event_local.group_ref.clone();
        assert_eq!(
            qualify_transformation_mass_balance_link(
                &event_local,
                &case,
                &[receipt("EXACT_BALANCED")]
            )
            .disposition,
            MaterialBalanceLinkDisposition::MassBalanceGatePass
        );

        let mut invalid = group();
        invalid.transformation_id = None;
        assert_eq!(
            qualify_transformation_mass_balance_link(
                &invalid,
                &case(),
                &[receipt("EXACT_BALANCED")]
            )
            .reason,
            Some(MaterialBalanceLinkReason::EventLocalGroupHasMultipleEvents)
        );
    }

    #[test]
    fn same_source_event_with_different_revisions_is_a_conflict() {
        let mut group = group();
        group.effective_source_events[1].source_id =
            group.effective_source_events[0].source_id.clone();
        group.effective_source_events[1].source_event_id =
            group.effective_source_events[0].source_event_id.clone();
        group.effective_source_events[1].revision = 3;
        assert_eq!(
            qualify_transformation_mass_balance_link(&group, &case(), &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::ConflictingSourceEventRevision)
        );
    }

    #[test]
    fn blocked_owner_status_retains_well_formed_digest_qualification() {
        let decision = evaluate("EXACT_UNBALANCED");
        assert_eq!(
            decision.digest_qualification,
            OwnerDigestQualification::ShapeAndReferenceMatchedOnly
        );
    }

    #[test]
    fn digest_shape_and_reference_are_not_claimed_as_authentication() {
        let decision = evaluate("EXACT_BALANCED");
        assert_eq!(
            decision.digest_qualification,
            OwnerDigestQualification::ShapeAndReferenceMatchedOnly
        );
        assert!(decision.claim_ceiling.contains("NO_PHYSICAL"));
    }

    #[test]
    fn duplicate_event_keys_in_group_are_rejected() {
        let mut group = group();
        group.effective_source_events[1].source_id = "WMS_A".to_string();
        group.effective_source_events[1].source_event_id = "urn:uuid:input".to_string();
        assert_eq!(
            qualify_transformation_mass_balance_link(&group, &case(), &[receipt("EXACT_BALANCED")])
                .reason,
            Some(MaterialBalanceLinkReason::DuplicateSourceEventRevision)
        );
    }

    #[test]
    fn unknown_and_missing_owner_statuses_remain_indeterminate() {
        for status in ["UNKNOWN", "MISSING"] {
            assert_eq!(
                evaluate(status).disposition,
                MaterialBalanceLinkDisposition::Indeterminate
            );
        }
    }

    #[test]
    fn unsupported_owner_status_blocks_instead_of_defaulting_to_pass() {
        assert_eq!(
            evaluate("WHATEVER").disposition,
            MaterialBalanceLinkDisposition::Blocked
        );
    }
}
