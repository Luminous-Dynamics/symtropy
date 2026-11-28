// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-to-context binding for a typed AERO engineering claim specification.
//!
//! This is a narrow admission adapter. It binds exact source payload bytes to
//! the typed claim object, its v0.1 semantic commitment, and the existing
//! decision context. It does not derive the complete assurance obligation
//! vector, authenticate owner receipts, evaluate producer authority, or grant
//! engineering / physical-work authority.

use super::claim_specification_v0_1::{
    ClaimSpecificationDispositionV0_1, ClaimSpecificationV0_1,
};
use super::decision_context_v0_1::DecisionContextV0_1;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

/// Result of binding the exact specification payload and typed semantics to a
/// decision context. A successful result means only that these identities and
/// intervals agree under the declared v0.1 contracts.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimSpecificationBindingDispositionV0_1 {
    #[serde(rename = "BOUND")]
    Bound,
    #[serde(rename = "INCOMPLETE")]
    Incomplete,
    #[serde(rename = "REJECTED")]
    Rejected,
    #[serde(rename = "UNSUPPORTED_PROFILE")]
    UnsupportedProfile,
}

impl ClaimSpecificationBindingDispositionV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bound => "BOUND",
            Self::Incomplete => "INCOMPLETE",
            Self::Rejected => "REJECTED",
            Self::UnsupportedProfile => "UNSUPPORTED_PROFILE",
        }
    }
}

/// Exact source inputs for claim-specification admission.
///
/// `expected_raw_payload_sha256` must originate from the source/owner
/// commitment that the caller intends to verify. This function recomputes the
/// bytes' digest but does not authenticate that commitment or its issuer.
pub struct ClaimSpecificationBindingInputV0_1<'a> {
    pub raw_payload: &'a [u8],
    pub expected_raw_payload_sha256: &'a str,
    pub specification: &'a ClaimSpecificationV0_1,
    pub decision_context: &'a DecisionContextV0_1,
}

/// Fully disclosed commitments and deterministic failure reasons.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClaimSpecificationBindingAssessmentV0_1 {
    pub disposition: ClaimSpecificationBindingDispositionV0_1,
    pub raw_payload_sha256: String,
    pub semantic_sha256: Option<String>,
    pub decision_context_sha256: Option<String>,
    pub reason_codes: Vec<String>,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub execution_authority_granted: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn assessment(
    disposition: ClaimSpecificationBindingDispositionV0_1,
    raw_payload_sha256: String,
    semantic_sha256: Option<String>,
    decision_context_sha256: Option<String>,
    reasons: BTreeSet<String>,
) -> ClaimSpecificationBindingAssessmentV0_1 {
    ClaimSpecificationBindingAssessmentV0_1 {
        disposition,
        raw_payload_sha256,
        semantic_sha256,
        decision_context_sha256,
        reason_codes: reasons.into_iter().collect(),
        physical_truth_established: false,
        engineering_acceptance_granted: false,
        execution_authority_granted: false,
    }
}

/// Bind source bytes -> typed claim specification -> decision context.
///
/// The claim-specification validity interval is interpreted as half-open
/// `[effective_from_unix_ms, effective_until_unix_ms)`. The acquisition
/// interval is closed at both ends; therefore the complete acquisition interval
/// must satisfy `earliest >= effective_from` and `latest < effective_until`.
///
/// In the existing DecisionContextV0_1 contract,
/// `claim_specification_sha256` is interpreted as the typed semantic
/// commitment produced by `ClaimSpecificationV0_1::compute_semantic_sha256`.
/// The raw payload digest is returned separately and must not be substituted
/// for that semantic digest. The required-evidence manifest already has its
/// own distinct context field.
///
/// This adapter is additive and does not replace `evaluate_claim_gate`.
pub fn bind_claim_specification_v0_1(
    input: ClaimSpecificationBindingInputV0_1<'_>,
) -> ClaimSpecificationBindingAssessmentV0_1 {
    let raw_payload_sha256 = sha256_hex(input.raw_payload);
    let mut reasons = BTreeSet::new();

    if !valid_sha256(input.expected_raw_payload_sha256) {
        reasons.insert("CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_INVALID".to_owned());
        return assessment(
            ClaimSpecificationBindingDispositionV0_1::Rejected,
            raw_payload_sha256,
            None,
            None,
            reasons,
        );
    }
    if raw_payload_sha256 != input.expected_raw_payload_sha256 {
        reasons.insert("CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_MISMATCH".to_owned());
        return assessment(
            ClaimSpecificationBindingDispositionV0_1::Rejected,
            raw_payload_sha256,
            None,
            None,
            reasons,
        );
    }

    let parsed: ClaimSpecificationV0_1 = match serde_json::from_slice(input.raw_payload) {
        Ok(value) => value,
        Err(_) => {
            reasons.insert("CLAIM_SPECIFICATION_RAW_PAYLOAD_PARSE_FAILED".to_owned());
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::Rejected,
                raw_payload_sha256,
                None,
                None,
                reasons,
            );
        }
    };
    if parsed != *input.specification {
        reasons.insert("CLAIM_SPECIFICATION_TYPED_OBJECT_PAYLOAD_MISMATCH".to_owned());
        return assessment(
            ClaimSpecificationBindingDispositionV0_1::Rejected,
            raw_payload_sha256,
            None,
            None,
            reasons,
        );
    }

    let specification_assessment = input.specification.assess();
    reasons.extend(specification_assessment.reason_codes.iter().cloned());
    match specification_assessment.disposition {
        ClaimSpecificationDispositionV0_1::Incomplete => {
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::Incomplete,
                raw_payload_sha256,
                None,
                None,
                reasons,
            );
        }
        ClaimSpecificationDispositionV0_1::Rejected => {
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::Rejected,
                raw_payload_sha256,
                None,
                None,
                reasons,
            );
        }
        ClaimSpecificationDispositionV0_1::UnsupportedProfile => {
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::UnsupportedProfile,
                raw_payload_sha256,
                None,
                None,
                reasons,
            );
        }
        ClaimSpecificationDispositionV0_1::FormallyWellSpecified => {}
    }

    let semantic_sha256 = match input.specification.compute_semantic_sha256() {
        Ok(value) => value,
        Err(errors) => {
            reasons.extend(errors);
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::Rejected,
                raw_payload_sha256,
                None,
                None,
                reasons,
            );
        }
    };

    let context_errors = input.decision_context.validation_errors();
    if !context_errors.is_empty() {
        reasons.extend(context_errors);
        return assessment(
            ClaimSpecificationBindingDispositionV0_1::Rejected,
            raw_payload_sha256,
            Some(semantic_sha256),
            None,
            reasons,
        );
    }
    let decision_context_sha256 = match input.decision_context.compute_sha256() {
        Ok(value) => value,
        Err(errors) => {
            reasons.extend(errors);
            return assessment(
                ClaimSpecificationBindingDispositionV0_1::Rejected,
                raw_payload_sha256,
                Some(semantic_sha256),
                None,
                reasons,
            );
        }
    };

    if input.specification.claim_id != input.decision_context.claim_id {
        reasons.insert("CLAIM_SPECIFICATION_CONTEXT_CLAIM_ID_MISMATCH".to_owned());
    }
    if input.specification.subject_ref != input.decision_context.subject_ref {
        reasons.insert("CLAIM_SPECIFICATION_CONTEXT_SUBJECT_MISMATCH".to_owned());
    }
    if input.specification.configuration_ref != input.decision_context.configuration_ref {
        reasons.insert("CLAIM_SPECIFICATION_CONTEXT_CONFIGURATION_MISMATCH".to_owned());
    }
    if input.specification.operating_envelope_ref
        != input.decision_context.operating_envelope_ref
    {
        reasons.insert("CLAIM_SPECIFICATION_CONTEXT_OPERATING_ENVELOPE_MISMATCH".to_owned());
    }
    if input.decision_context.claim_specification_sha256 != semantic_sha256 {
        reasons.insert("CLAIM_SPECIFICATION_CONTEXT_SEMANTIC_DIGEST_MISMATCH".to_owned());
    }

    let acquisition_is_within_specification = input
        .decision_context
        .acquisition_time_earliest_unix_ms
        >= input.specification.validity.effective_from_unix_ms
        && input
            .decision_context
            .acquisition_time_latest_unix_ms
            < input.specification.validity.effective_until_unix_ms;
    if !acquisition_is_within_specification {
        reasons.insert("CLAIM_SPECIFICATION_VALIDITY_DOES_NOT_COVER_ACQUISITION".to_owned());
    }

    let disposition = if reasons.is_empty() {
        ClaimSpecificationBindingDispositionV0_1::Bound
    } else {
        ClaimSpecificationBindingDispositionV0_1::Rejected
    };
    assessment(
        disposition,
        raw_payload_sha256,
        Some(semantic_sha256),
        Some(decision_context_sha256),
        reasons,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision_context_v0_1::DecisionContextV0_1;

    const SPECIFICATION_FIXTURE: &str = include_str!(
        "../../../../docs/research/aviation/examples/aero_claim_specification_v0_1.json"
    );

    fn fixture() -> (Vec<u8>, ClaimSpecificationV0_1, DecisionContextV0_1) {
        let raw_payload = SPECIFICATION_FIXTURE.as_bytes().to_vec();
        let specification: ClaimSpecificationV0_1 =
            serde_json::from_slice(&raw_payload).expect("claim-specification fixture parses");
        let semantic_sha256 = specification
            .compute_semantic_sha256()
            .expect("fixture has a valid semantic commitment");
        let decision_context = DecisionContextV0_1 {
            claim_id: specification.claim_id.clone(),
            subject_ref: specification.subject_ref.clone(),
            configuration_ref: specification.configuration_ref.clone(),
            operating_envelope_ref: specification.operating_envelope_ref.clone(),
            acquisition_time_earliest_unix_ms: specification.validity.effective_from_unix_ms,
            acquisition_time_latest_unix_ms: specification.validity.effective_from_unix_ms + 1,
            as_known_at_unix_ms: specification.validity.effective_from_unix_ms + 100,
            source_scope_manifest_sha256: "a".repeat(64),
            evidence_knowledge_cut_sha256: "b".repeat(64),
            trust_policy_sha256: "c".repeat(64),
            claim_specification_sha256: semantic_sha256,
            required_evidence_manifest_sha256: "d".repeat(64),
            risk_classification_sha256: "e".repeat(64),
            model_applicability_profile_sha256: "f".repeat(64),
        };
        (raw_payload, specification, decision_context)
    }

    fn raw_digest(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    #[test]
    fn binds_raw_payload_typed_semantics_and_decision_context() {
        let (raw_payload, specification, decision_context) = fixture();
        let expected_raw_payload_sha256 = raw_digest(&raw_payload);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });

        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Bound
        );
        assert_eq!(result.raw_payload_sha256, expected_raw_payload_sha256);
        assert_eq!(
            result.semantic_sha256.as_deref(),
            Some(decision_context.claim_specification_sha256.as_str())
        );
        assert!(result.decision_context_sha256.is_some());
        assert!(result.reason_codes.is_empty());
        assert!(!result.physical_truth_established);
        assert!(!result.engineering_acceptance_granted);
        assert!(!result.execution_authority_granted);
    }

    #[test]
    fn rejects_raw_payload_digest_substitution() {
        let (raw_payload, specification, decision_context) = fixture();
        let wrong_digest = "0".repeat(64);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &wrong_digest,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Rejected
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_MISMATCH"
        }));
    }

    #[test]
    fn rejects_payload_that_deserializes_to_another_typed_specification() {
        let (raw_payload, specification, decision_context) = fixture();
        let substituted = String::from_utf8(raw_payload)
            .expect("fixture UTF-8")
            .replace(
                "part:wing-panel-synthetic-001",
                "part:wing-panel-substituted-999",
            );
        let expected_raw_payload_sha256 = raw_digest(substituted.as_bytes());
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: substituted.as_bytes(),
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Rejected
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "CLAIM_SPECIFICATION_TYPED_OBJECT_PAYLOAD_MISMATCH"
        }));
    }

    #[test]
    fn rejects_context_subject_substitution() {
        let (raw_payload, specification, mut decision_context) = fixture();
        decision_context.subject_ref = "part:another-physical-article".to_owned();
        let expected_raw_payload_sha256 = raw_digest(&raw_payload);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Rejected
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "CLAIM_SPECIFICATION_CONTEXT_SUBJECT_MISMATCH"
        }));
    }

    #[test]
    fn rejects_context_semantic_commitment_substitution() {
        let (raw_payload, specification, mut decision_context) = fixture();
        decision_context.claim_specification_sha256 = "0".repeat(64);
        let expected_raw_payload_sha256 = raw_digest(&raw_payload);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Rejected
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "CLAIM_SPECIFICATION_CONTEXT_SEMANTIC_DIGEST_MISMATCH"
        }));
    }

    #[test]
    fn rejects_acquisition_interval_touching_exclusive_specification_end() {
        let (raw_payload, specification, mut decision_context) = fixture();
        decision_context.acquisition_time_latest_unix_ms =
            specification.validity.effective_until_unix_ms;
        decision_context.as_known_at_unix_ms =
            specification.validity.effective_until_unix_ms + 1;
        let expected_raw_payload_sha256 = raw_digest(&raw_payload);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Rejected
        );
        assert!(result.reason_codes.iter().any(|code| {
            code == "CLAIM_SPECIFICATION_VALIDITY_DOES_NOT_COVER_ACQUISITION"
        }));
    }

    #[test]
    fn incomplete_specification_is_not_promoted_by_valid_source_bytes() {
        let (_, mut specification, decision_context) = fixture();
        specification.subject_ref.clear();
        let raw_payload =
            serde_json::to_vec(&specification).expect("incomplete typed spec serializes");
        let expected_raw_payload_sha256 = raw_digest(&raw_payload);
        let result = bind_claim_specification_v0_1(ClaimSpecificationBindingInputV0_1 {
            raw_payload: &raw_payload,
            expected_raw_payload_sha256: &expected_raw_payload_sha256,
            specification: &specification,
            decision_context: &decision_context,
        });
        assert_eq!(
            result.disposition,
            ClaimSpecificationBindingDispositionV0_1::Incomplete
        );
        assert!(!result.physical_truth_established);
    }
}
