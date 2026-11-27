// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Versioned, typed AERO decision-context envelope.
//!
//! This module derives a context commitment from exact claim, subject,
//! configuration, operating-envelope, time, source-scope, evidence-cut,
//! trust-policy, claim-specification, required-evidence and risk-profile
//! identities. It prevents callers from choosing an arbitrary shared digest
//! that is unrelated to the typed context they submit.
//!
//! The referenced external manifests and policy artifacts remain owner-supplied
//! inputs. This digest binds their identifiers; it does not authenticate their
//! bytes, issuers, signatures, physical truth or authority.

use serde::Serialize;
use sha2::{Digest, Sha256};

/// Versioned typed context for one engineering claim evaluation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecisionContextV0_1 {
    /// Stable engineering claim identity.
    pub claim_id: String,
    /// Exact physical article/subject identity.
    pub subject_ref: String,
    /// Exact intended configuration identity.
    pub configuration_ref: String,
    /// Exact operating-envelope identity.
    pub operating_envelope_ref: String,
    /// Earliest hard-bounded acquisition time, UTC Unix epoch milliseconds.
    pub acquisition_time_earliest_unix_ms: u64,
    /// Latest hard-bounded acquisition time, UTC Unix epoch milliseconds.
    pub acquisition_time_latest_unix_ms: u64,
    /// Selected AS-KNOWN evaluation snapshot, UTC Unix epoch milliseconds.
    pub as_known_at_unix_ms: u64,
    /// Digest of the declared source-universe/scope manifest.
    pub source_scope_manifest_sha256: String,
    /// Digest of owner-native evidence receipt/watermark cut.
    pub evidence_knowledge_cut_sha256: String,
    /// Digest of the pinned trust-policy snapshot.
    pub trust_policy_sha256: String,
    /// Digest of the normative claim specification and required obligations.
    pub claim_specification_sha256: String,
    /// Digest of the exact required-evidence reference manifest.
    pub required_evidence_manifest_sha256: String,
    /// Digest of the risk classification and consequence assessment.
    pub risk_classification_sha256: String,
    /// Digest of the model/applicability profile relevant to this claim.
    pub model_applicability_profile_sha256: String,
}

#[derive(Serialize)]
struct DecisionContextEnvelope<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    claim_id: &'a str,
    subject_ref: &'a str,
    configuration_ref: &'a str,
    operating_envelope_ref: &'a str,
    acquisition_time_earliest_unix_ms: u64,
    acquisition_time_latest_unix_ms: u64,
    as_known_at_unix_ms: u64,
    source_scope_manifest_sha256: &'a str,
    evidence_knowledge_cut_sha256: &'a str,
    trust_policy_sha256: &'a str,
    claim_specification_sha256: &'a str,
    required_evidence_manifest_sha256: &'a str,
    risk_classification_sha256: &'a str,
    model_applicability_profile_sha256: &'a str,
}

#[derive(Serialize)]
struct RequiredEvidenceManifestEnvelope<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    evidence_refs: Vec<&'a str>,
}

fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty()
        && value == value.trim()
        && value.bytes().all(|byte| byte.is_ascii_graphic())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

/// Hash a required-evidence manifest independent of caller order.
///
/// Duplicate references remain represented in the digest; the claim gate
/// separately rejects duplicate mandatory references. Empty manifests are
/// hashable but cannot produce a reviewable claim.
pub fn compute_required_evidence_manifest_sha256(
    evidence_refs: &[String],
) -> Result<String, String> {
    if evidence_refs.iter().any(|reference| !valid_identifier(reference)) {
        return Err("REQUIRED_EVIDENCE_REFERENCE_NONCANONICAL".to_owned());
    }
    let mut sorted_refs = evidence_refs.iter().map(String::as_str).collect::<Vec<_>>();
    sorted_refs.sort_unstable();
    let envelope = RequiredEvidenceManifestEnvelope {
        schema: "AeroRequiredEvidenceManifestV1",
        canonicalization_profile: "rust-serde-json-struct-order-v1",
        evidence_refs: sorted_refs,
    };
    let bytes = serde_json::to_vec(&envelope)
        .map_err(|_| "REQUIRED_EVIDENCE_MANIFEST_SERIALIZATION_FAILED".to_owned())?;
    Ok(sha256_hex(&bytes))
}

impl DecisionContextV0_1 {
    /// Return all deterministic contract violations without silently repairing input.
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();
        for (field, value) in [
            ("CLAIM_ID", self.claim_id.as_str()),
            ("SUBJECT_REF", self.subject_ref.as_str()),
            ("CONFIGURATION_REF", self.configuration_ref.as_str()),
            (
                "OPERATING_ENVELOPE_REF",
                self.operating_envelope_ref.as_str(),
            ),
        ] {
            if !valid_identifier(value) {
                errors.push(format!("DECISION_CONTEXT_{field}_INVALID"));
            }
        }
        if self.acquisition_time_earliest_unix_ms > self.acquisition_time_latest_unix_ms {
            errors.push("DECISION_CONTEXT_ACQUISITION_INTERVAL_REVERSED".to_owned());
        }
        if self.acquisition_time_latest_unix_ms > self.as_known_at_unix_ms {
            errors.push("DECISION_CONTEXT_ACQUISITION_AFTER_AS_KNOWN".to_owned());
        }
        for (field, value) in [
            ("SOURCE_SCOPE", self.source_scope_manifest_sha256.as_str()),
            (
                "EVIDENCE_KNOWLEDGE_CUT",
                self.evidence_knowledge_cut_sha256.as_str(),
            ),
            ("TRUST_POLICY", self.trust_policy_sha256.as_str()),
            (
                "CLAIM_SPECIFICATION",
                self.claim_specification_sha256.as_str(),
            ),
            (
                "REQUIRED_EVIDENCE_MANIFEST",
                self.required_evidence_manifest_sha256.as_str(),
            ),
            (
                "RISK_CLASSIFICATION",
                self.risk_classification_sha256.as_str(),
            ),
            (
                "MODEL_APPLICABILITY_PROFILE",
                self.model_applicability_profile_sha256.as_str(),
            ),
        ] {
            if !valid_sha256(value) {
                errors.push(format!("DECISION_CONTEXT_{field}_DIGEST_INVALID"));
            }
        }
        errors
    }

    /// Compute SHA-256 over the validated, versioned field-ordered JSON envelope.
    ///
    /// This is a versioned Rust/Serde serialization profile, not a general
    /// RFC 8785 canonical JSON implementation. Independent implementations
    /// must reproduce this profile exactly or agree a future cross-language
    /// canonicalization standard before exchanging these digests.
    pub fn compute_sha256(&self) -> Result<String, Vec<String>> {
        let errors = self.validation_errors();
        if !errors.is_empty() {
            return Err(errors);
        }
        let envelope = DecisionContextEnvelope {
            schema: "AeroDecisionContextV0_1",
            canonicalization_profile: "rust-serde-json-struct-order-v1",
            claim_id: &self.claim_id,
            subject_ref: &self.subject_ref,
            configuration_ref: &self.configuration_ref,
            operating_envelope_ref: &self.operating_envelope_ref,
            acquisition_time_earliest_unix_ms: self.acquisition_time_earliest_unix_ms,
            acquisition_time_latest_unix_ms: self.acquisition_time_latest_unix_ms,
            as_known_at_unix_ms: self.as_known_at_unix_ms,
            source_scope_manifest_sha256: &self.source_scope_manifest_sha256,
            evidence_knowledge_cut_sha256: &self.evidence_knowledge_cut_sha256,
            trust_policy_sha256: &self.trust_policy_sha256,
            claim_specification_sha256: &self.claim_specification_sha256,
            required_evidence_manifest_sha256: &self.required_evidence_manifest_sha256,
            risk_classification_sha256: &self.risk_classification_sha256,
            model_applicability_profile_sha256: &self.model_applicability_profile_sha256,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| vec!["DECISION_CONTEXT_SERIALIZATION_FAILED".to_owned()])?;
        Ok(sha256_hex(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::{DecisionContextV0_1, compute_required_evidence_manifest_sha256};

    fn context(refs: &[String]) -> DecisionContextV0_1 {
        DecisionContextV0_1 {
            claim_id: "claim-wing-panel-001".to_owned(),
            subject_ref: "part:wing-panel-001".to_owned(),
            configuration_ref: "configuration:wing-r17".to_owned(),
            operating_envelope_ref: "envelope:transport-normal".to_owned(),
            acquisition_time_earliest_unix_ms: 1_795_514_399_900,
            acquisition_time_latest_unix_ms: 1_795_514_400_100,
            as_known_at_unix_ms: 1_795_516_800_000,
            source_scope_manifest_sha256: "a".repeat(64),
            evidence_knowledge_cut_sha256: "b".repeat(64),
            trust_policy_sha256: "c".repeat(64),
            claim_specification_sha256: "d".repeat(64),
            required_evidence_manifest_sha256: compute_required_evidence_manifest_sha256(refs)
                .expect("manifest serializes"),
            risk_classification_sha256: "e".repeat(64),
            model_applicability_profile_sha256: "f".repeat(64),
        }
    }

    #[test]
    fn rust_digest_matches_the_frozen_cross_language_context_vector() {
        let refs = vec![
            "receipt:inspection-01".to_owned(),
            "receipt:calibration-02".to_owned(),
        ];
        let context = context(&refs);
        assert_eq!(
            context.required_evidence_manifest_sha256,
            "17656ee262a7455dfc652ba667379f8696160c7a567a559a5d8fb7bbced45032"
        );
        assert_eq!(
            context.compute_sha256().expect("valid context"),
            "b413084d0f543cc90a15e14cdff3fa92e6cf934b9e57773182017dfa2001768e"
        );
    }

    #[test]
    fn identical_typed_contexts_have_identical_commitments() {
        let refs = vec!["receipt:inspection-01".to_owned()];
        assert_eq!(
            context(&refs).compute_sha256(),
            context(&refs).compute_sha256()
        );
    }

    #[test]
    fn required_evidence_manifest_is_order_invariant_and_duplicate_sensitive() {
        let a = vec!["receipt:a".to_owned(), "receipt:b".to_owned()];
        let b = vec!["receipt:b".to_owned(), "receipt:a".to_owned()];
        let duplicate = vec![
            "receipt:a".to_owned(),
            "receipt:a".to_owned(),
            "receipt:b".to_owned(),
        ];
        assert_eq!(
            compute_required_evidence_manifest_sha256(&a),
            compute_required_evidence_manifest_sha256(&b)
        );
        assert_ne!(
            compute_required_evidence_manifest_sha256(&a),
            compute_required_evidence_manifest_sha256(&duplicate)
        );
    }

    #[test]
    fn every_external_context_dimension_changes_the_commitment() {
        let refs = vec!["receipt:inspection-01".to_owned()];
        let original = context(&refs);
        let baseline = original.compute_sha256().expect("valid context");
        let mut variants = Vec::new();

        let mut changed = original.clone();
        changed.claim_id.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.subject_ref.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.configuration_ref.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.operating_envelope_ref.push_str("-changed");
        variants.push(changed);
        let mut changed = original.clone();
        changed.acquisition_time_earliest_unix_ms -= 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.acquisition_time_latest_unix_ms += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.as_known_at_unix_ms += 1;
        variants.push(changed);
        let mut changed = original.clone();
        changed.source_scope_manifest_sha256 = "1".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.evidence_knowledge_cut_sha256 = "2".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.trust_policy_sha256 = "3".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.claim_specification_sha256 = "4".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.required_evidence_manifest_sha256 = "5".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.risk_classification_sha256 = "6".repeat(64);
        variants.push(changed);
        let mut changed = original.clone();
        changed.model_applicability_profile_sha256 = "7".repeat(64);
        variants.push(changed);

        for variant in variants {
            assert_ne!(variant.compute_sha256().expect("valid context"), baseline);
        }
    }

    #[test]
    fn reversed_or_future_acquisition_interval_is_rejected() {
        let refs = vec!["receipt:inspection-01".to_owned()];
        let mut reversed = context(&refs);
        reversed.acquisition_time_earliest_unix_ms = reversed.acquisition_time_latest_unix_ms + 1;
        assert!(
            reversed
                .validation_errors()
                .contains(&"DECISION_CONTEXT_ACQUISITION_INTERVAL_REVERSED".to_owned())
        );

        let mut future = context(&refs);
        future.as_known_at_unix_ms = future.acquisition_time_latest_unix_ms - 1;
        assert!(
            future
                .validation_errors()
                .contains(&"DECISION_CONTEXT_ACQUISITION_AFTER_AS_KNOWN".to_owned())
        );
    }

    #[test]
    fn context_and_manifest_identifiers_reject_spaces_controls_and_non_ascii() {
        let refs = vec!["receipt:inspection-01".to_owned()];
        let mut context = context(&refs);
        context.subject_ref = "part:wing panel".to_owned();
        assert!(
            context
                .validation_errors()
                .contains(&"DECISION_CONTEXT_SUBJECT_REF_INVALID".to_owned())
        );

        let with_control = vec!["receipt:inspection\n01".to_owned()];
        assert_eq!(
            compute_required_evidence_manifest_sha256(&with_control),
            Err("REQUIRED_EVIDENCE_REFERENCE_NONCANONICAL".to_owned())
        );
        let with_non_ascii = vec!["receipt:inspection-é".to_owned()];
        assert_eq!(
            compute_required_evidence_manifest_sha256(&with_non_ascii),
            Err("REQUIRED_EVIDENCE_REFERENCE_NONCANONICAL".to_owned())
        );
    }

    #[test]
    fn invalid_external_digest_or_identity_is_rejected() {
        let refs = vec!["receipt:inspection-01".to_owned()];
        let mut invalid = context(&refs);
        invalid.trust_policy_sha256 = "ABCDEF".to_owned();
        invalid.subject_ref = " part:wing-panel-001".to_owned();
        let errors = invalid.validation_errors();
        assert!(errors.contains(&"DECISION_CONTEXT_TRUST_POLICY_DIGEST_INVALID".to_owned()));
        assert!(errors.contains(&"DECISION_CONTEXT_SUBJECT_REF_INVALID".to_owned()));
        assert!(invalid.compute_sha256().is_err());
    }

    #[test]
    fn empty_required_manifest_is_deterministic_but_does_not_create_reviewable_evidence() {
        let empty: Vec<String> = Vec::new();
        let commitment =
            compute_required_evidence_manifest_sha256(&empty).expect("empty manifest serializes");
        assert_eq!(commitment.len(), 64);
        let valid_context = context(&empty);
        assert!(valid_context.compute_sha256().is_ok());
    }
}
