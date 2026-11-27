// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit semantic binding between an engineering claim and an owner model metric.
//!
//! This adapter prevents an otherwise valid model report from being substituted
//! for a different measurand, unit, aggregation, population, predicate, uncertainty
//! rule or validation stratum. It binds owner receipt references and historical
//! receiver time, but it does not authenticate signatures, calculate VVUQ,
//! determine scientific equivalence, establish physical truth or grant authority.

use crate::claim_specification_v0_1::{
    AggregationV0_1, ClaimSpecificationDispositionV0_1, ClaimSpecificationV0_1,
    ComparisonOperatorV0_1, PopulationQuantifierV0_1, RationalV0_1,
    UnitCodeV0_1, UncertaintyTreatmentV0_1,
};
use crate::decision_context_v0_1::{
    DecisionContextV0_1, compute_required_evidence_manifest_sha256,
};
use crate::model_credibility_cut_v0_1::{
    MetricStratumStatus, ModelCredibilityCutV0_1,
};
use crate::owner_evidence_cut_v0_1::{
    OwnerDependencyCoverage, OwnerEvidenceCutV0_1, OwnerEvidenceFreshness,
    OwnerEvidenceLifecycle, OwnerEvidenceRecordV0_1,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CLAIM_METRIC_BINDING_SCHEMA_V0_1: &str = "AeroClaimMetricSemanticBindingV0_1";

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

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimMetricStratumBindingV0_1 {
    /// Stable stratum identity in the normative claim specification.
    pub claim_stratum_id: String,
    /// Exact stratum reference used by the model owner's metric report.
    pub model_stratum_ref: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ClaimMetricBindingDispositionV0_1 {
    SemanticsBound,
    Incomplete,
    Unverified,
    NotKnownAtSnapshot,
    OutOfScope,
    Rejected,
    UnsupportedProfile,
}

impl ClaimMetricBindingDispositionV0_1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SemanticsBound => "SEMANTICS_BOUND",
            Self::Incomplete => "INCOMPLETE",
            Self::Unverified => "UNVERIFIED",
            Self::NotKnownAtSnapshot => "NOT_KNOWN_AT_SNAPSHOT",
            Self::OutOfScope => "OUT_OF_SCOPE",
            Self::Rejected => "REJECTED",
            Self::UnsupportedProfile => "UNSUPPORTED_PROFILE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClaimMetricBindingAssessmentV0_1 {
    pub disposition: ClaimMetricBindingDispositionV0_1,
    pub reason_codes: Vec<String>,
    pub claim_specification_semantic_sha256: String,
    pub decision_context_sha256: String,
    pub model_credibility_sha256: String,
    pub owner_evidence_cut_sha256: String,
    /// This module binds receipt identities but never authenticates their signatures.
    pub owner_authentication_performed: bool,
    pub physical_truth_established: bool,
    pub engineering_acceptance_granted: bool,
    pub execution_authority_granted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimMetricSemanticBindingV0_1 {
    pub schema: String,
    pub binding_ref: String,
    pub claim_id: String,

    /// SHA-256 of the exact source bytes supplied to the specification parser.
    pub claim_specification_payload_sha256: String,
    /// Versioned typed commitment computed from ClaimSpecificationV0_1.
    pub claim_specification_semantic_sha256: String,
    /// Exact decision-context commitment for this evaluation.
    pub decision_context_sha256: String,
    /// Exact model-credibility cut commitment that contains the metric reports.
    pub model_credibility_sha256: String,
    /// Exact owner-native evidence cut containing all receipt identities below.
    pub owner_evidence_cut_sha256: String,
    /// Exact references included in DecisionContextV0_1.required_evidence_manifest_sha256.
    pub required_evidence_refs: Vec<String>,

    /// Owner-native model metric identity. Never inferred from a display name.
    pub model_metric_id: String,
    /// Explicit mapping from the owner metric to the normative claim measurand.
    pub measurand_id: String,
    pub metric_definition_ref: String,
    pub metric_definition_sha256: String,
    pub semantic_mapping_ref: String,
    pub semantic_mapping_basis_sha256: String,

    pub unit_code: UnitCodeV0_1,
    pub aggregation: AggregationV0_1,

    pub population_ref: String,
    pub sampling_unit_ref: String,
    pub population_manifest_sha256: String,
    pub population_quantifier: PopulationQuantifierV0_1,
    pub population_confidence_level_ppm: Option<u32>,

    pub predicate_operator: ComparisonOperatorV0_1,
    pub predicate_threshold: RationalV0_1,
    pub predicate_unit_code: UnitCodeV0_1,

    pub uncertainty_treatment: UncertaintyTreatmentV0_1,
    pub uncertainty_method_ref: String,
    pub uncertainty_profile_sha256: String,
    pub uncertainty_confidence_level_ppm: Option<u32>,

    /// One-to-one mapping from every claim-required stratum to model stratum IDs.
    pub required_strata: Vec<ClaimMetricStratumBindingV0_1>,

    /// Exact owner-native records. Each role is independently addressable.
    pub owner_namespace: String,
    pub metric_semantics_evidence_ref: String,
    pub metric_semantics_record_sha256: String,
    pub model_report_evidence_ref: String,
    pub model_report_record_sha256: String,
    pub owner_attestation_evidence_ref: String,
    pub owner_attestation_record_sha256: String,
    pub producer_authority_receipt_ref: String,
    pub producer_authority_receipt_sha256: String,
    pub semantic_validation_receipt_ref: String,
    pub semantic_validation_receipt_sha256: String,

    /// Receiver-observed interval for this owner binding, not issuer-asserted time.
    pub receiver_first_seen_earliest_unix_ms: u64,
    pub receiver_first_seen_latest_unix_ms: u64,
}

#[derive(Serialize)]
struct ClaimMetricBindingEnvelope<'a> {
    schema: &'static str,
    canonicalization_profile: &'static str,
    binding: &'a ClaimMetricSemanticBindingV0_1,
}

impl ClaimMetricSemanticBindingV0_1 {
    /// Canonical typed commitment to the complete semantic binding.
    ///
    /// This is a versioned Rust/Serde struct-order profile, not RFC 8785/JCS.
    pub fn compute_sha256(&self) -> Result<String, Vec<String>> {
        let mut errors = Vec::new();
        if self.schema != CLAIM_METRIC_BINDING_SCHEMA_V0_1 {
            errors.push("CLAIM_METRIC_BINDING_SCHEMA_UNSUPPORTED".to_owned());
        }
        if !valid_identifier(&self.binding_ref) || !valid_identifier(&self.claim_id) {
            errors.push("CLAIM_METRIC_BINDING_IDENTITY_INVALID".to_owned());
        }
        for value in [
            self.claim_specification_payload_sha256.as_str(),
            self.claim_specification_semantic_sha256.as_str(),
            self.decision_context_sha256.as_str(),
            self.model_credibility_sha256.as_str(),
            self.owner_evidence_cut_sha256.as_str(),
            self.metric_definition_sha256.as_str(),
            self.semantic_mapping_basis_sha256.as_str(),
            self.population_manifest_sha256.as_str(),
            self.uncertainty_profile_sha256.as_str(),
            self.metric_semantics_record_sha256.as_str(),
            self.model_report_record_sha256.as_str(),
            self.owner_attestation_record_sha256.as_str(),
            self.producer_authority_receipt_sha256.as_str(),
            self.semantic_validation_receipt_sha256.as_str(),
        ] {
            if !valid_sha256(value) {
                errors.push("CLAIM_METRIC_BINDING_DIGEST_INVALID".to_owned());
                break;
            }
        }
        let identifiers = [
            self.model_metric_id.as_str(),
            self.measurand_id.as_str(),
            self.metric_definition_ref.as_str(),
            self.semantic_mapping_ref.as_str(),
            self.population_ref.as_str(),
            self.sampling_unit_ref.as_str(),
            self.uncertainty_method_ref.as_str(),
            self.owner_namespace.as_str(),
            self.metric_semantics_evidence_ref.as_str(),
            self.model_report_evidence_ref.as_str(),
            self.owner_attestation_evidence_ref.as_str(),
            self.producer_authority_receipt_ref.as_str(),
            self.semantic_validation_receipt_ref.as_str(),
        ];
        if identifiers.iter().any(|value| !valid_identifier(value)) {
            errors.push("CLAIM_METRIC_BINDING_REFERENCE_INVALID".to_owned());
        }
        if !self.predicate_threshold.is_canonical() {
            errors.push("CLAIM_METRIC_BINDING_THRESHOLD_NONCANONICAL".to_owned());
        }
        if self.required_strata.is_empty() {
            errors.push("CLAIM_METRIC_BINDING_REQUIRED_STRATA_EMPTY".to_owned());
        }
        let mut claim_strata = BTreeSet::new();
        let mut model_strata = BTreeSet::new();
        for item in &self.required_strata {
            if !valid_identifier(&item.claim_stratum_id)
                || !valid_identifier(&item.model_stratum_ref)
            {
                errors.push("CLAIM_METRIC_BINDING_STRATUM_REFERENCE_INVALID".to_owned());
            }
            if !claim_strata.insert(item.claim_stratum_id.as_str())
                || !model_strata.insert(item.model_stratum_ref.as_str())
            {
                errors.push("CLAIM_METRIC_BINDING_STRATUM_MAPPING_NOT_ONE_TO_ONE".to_owned());
            }
        }
        if self.receiver_first_seen_earliest_unix_ms
            > self.receiver_first_seen_latest_unix_ms
        {
            errors.push("CLAIM_METRIC_BINDING_RECEIVER_INTERVAL_REVERSED".to_owned());
        }
        errors.sort();
        errors.dedup();
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut required_refs = BTreeSet::new();
        for evidence_ref in &self.required_evidence_refs {
            if !valid_identifier(evidence_ref) {
                errors.push("CLAIM_METRIC_BINDING_REQUIRED_EVIDENCE_REFERENCE_INVALID".to_owned());
            }
            if !required_refs.insert(evidence_ref.as_str()) {
                errors.push("CLAIM_METRIC_BINDING_REQUIRED_EVIDENCE_REFERENCE_DUPLICATE".to_owned());
            }
        }
        if self.required_evidence_refs.is_empty() {
            errors.push("CLAIM_METRIC_BINDING_REQUIRED_EVIDENCE_MANIFEST_EMPTY".to_owned());
        }
        errors.sort();
        errors.dedup();
        if !errors.is_empty() {
            return Err(errors);
        }
        let mut normalized = self.clone();
        normalized.required_evidence_refs.sort();
        normalized.required_strata.sort_by(|left, right| {
            (&left.claim_stratum_id, &left.model_stratum_ref)
                .cmp(&(&right.claim_stratum_id, &right.model_stratum_ref))
        });
        let envelope = ClaimMetricBindingEnvelope {
            schema: "AeroClaimMetricBindingCommitmentV0_1",
            canonicalization_profile: "rust-serde-json-struct-order-v1",
            binding: &normalized,
        };
        let bytes = serde_json::to_vec(&envelope)
            .map_err(|_| vec!["CLAIM_METRIC_BINDING_SERIALIZATION_FAILED".to_owned()])?;
        Ok(sha256_hex(&bytes))
    }

    /// Compare claim semantics, model report semantics, owner records and historical cutoffs.
    ///
    /// A SEMANTICS_BOUND result means the typed proposition is explicitly mapped
    /// to the exact model metric and reports. It does not establish that the
    /// mapping is scientifically adequate or that owner signatures are authentic.
    pub fn assess(
        &self,
        specification: &ClaimSpecificationV0_1,
        raw_specification_payload_sha256: &str,
        decision_context: &DecisionContextV0_1,
        model_cut: &ModelCredibilityCutV0_1,
        owner_cut: &OwnerEvidenceCutV0_1,
    ) -> ClaimMetricBindingAssessmentV0_1 {
        let mut reasons = BTreeSet::new();
        let mut rejected = false;
        let mut incomplete = false;
        let mut unverified = false;
        let mut out_of_scope = false;
        let mut not_known_at_snapshot = false;
        let mut unsupported = false;

        match self.compute_sha256() {
            Ok(_) => {}
            Err(errors) => {
                rejected = true;
                reasons.extend(errors);
            }
        }

        let specification_assessment = specification.assess();
        match specification_assessment.disposition {
            ClaimSpecificationDispositionV0_1::FormallyWellSpecified => {}
            ClaimSpecificationDispositionV0_1::Incomplete => {
                incomplete = true;
                reasons.extend(specification_assessment.reason_codes);
            }
            ClaimSpecificationDispositionV0_1::Rejected => {
                rejected = true;
                reasons.extend(specification_assessment.reason_codes);
            }
            ClaimSpecificationDispositionV0_1::UnsupportedProfile => {
                unsupported = true;
                reasons.extend(specification_assessment.reason_codes);
            }
        }

        if self.schema != CLAIM_METRIC_BINDING_SCHEMA_V0_1 {
            unsupported = true;
            reasons.insert("CLAIM_METRIC_BINDING_SCHEMA_UNSUPPORTED".to_owned());
        }
        if !valid_identifier(&self.binding_ref) || !valid_identifier(&self.claim_id) {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_IDENTITY_INVALID".to_owned());
        }

        let semantic_digest = match specification.compute_semantic_sha256() {
            Ok(value) => value,
            Err(errors) => {
                reasons.extend(errors);
                String::new()
            }
        };
        if !valid_sha256(raw_specification_payload_sha256) {
            rejected = true;
            reasons.insert("CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_INVALID".to_owned());
        } else if self.claim_specification_payload_sha256 != raw_specification_payload_sha256 {
            rejected = true;
            reasons.insert("CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_MISMATCH".to_owned());
        }
        if !semantic_digest.is_empty()
            && self.claim_specification_semantic_sha256 != semantic_digest
        {
            rejected = true;
            reasons.insert("CLAIM_SPECIFICATION_SEMANTIC_DIGEST_MISMATCH".to_owned());
        }

        let context_digest = match decision_context.compute_sha256() {
            Ok(value) => value,
            Err(errors) => {
                rejected = true;
                reasons.extend(errors);
                String::new()
            }
        };
        if !context_digest.is_empty() && self.decision_context_sha256 != context_digest {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_DECISION_CONTEXT_MISMATCH".to_owned());
        }
        if decision_context.claim_specification_sha256 != semantic_digest {
            rejected = true;
            reasons.insert("DECISION_CONTEXT_CLAIM_SPECIFICATION_SEMANTIC_DIGEST_MISMATCH".to_owned());
        }

        match compute_required_evidence_manifest_sha256(&self.required_evidence_refs) {
            Ok(manifest_digest)
                if manifest_digest == decision_context.required_evidence_manifest_sha256 => {}
            Ok(_) => {
                rejected = true;
                reasons.insert("CLAIM_METRIC_BINDING_REQUIRED_EVIDENCE_MANIFEST_MISMATCH".to_owned());
            }
            Err(reason) => {
                rejected = true;
                reasons.insert(reason);
            }
        }

        for (code, actual, expected) in [
            ("CLAIM_ID", self.claim_id.as_str(), specification.claim_id.as_str()),
            ("CLAIM_ID_CONTEXT", decision_context.claim_id.as_str(), specification.claim_id.as_str()),
            ("SUBJECT_CONTEXT", decision_context.subject_ref.as_str(), specification.subject_ref.as_str()),
            ("CONFIGURATION_CONTEXT", decision_context.configuration_ref.as_str(), specification.configuration_ref.as_str()),
            ("OPERATING_ENVELOPE_CONTEXT", decision_context.operating_envelope_ref.as_str(), specification.operating_envelope_ref.as_str()),
            ("MODEL_CLAIM_ID", model_cut.claim_id.as_str(), specification.claim_id.as_str()),
            ("MODEL_CONFIGURATION", model_cut.configuration_ref.as_str(), specification.configuration_ref.as_str()),
            ("MODEL_OPERATING_ENVELOPE", model_cut.operating_envelope_ref.as_str(), specification.operating_envelope_ref.as_str()),
            ("OWNER_PHYSICAL_SUBJECT", owner_cut.physical_unit_ref.as_str(), specification.subject_ref.as_str()),
            ("OWNER_CONFIGURATION", owner_cut.configuration_ref.as_str(), specification.configuration_ref.as_str()),
            ("OWNER_NAMESPACE", self.owner_namespace.as_str(), owner_cut.owner_namespace.as_str()),
        ] {
            if actual != expected {
                rejected = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{code}_MISMATCH"));
            }
        }

        if specification.validity.effective_from_unix_ms
            > decision_context.acquisition_time_earliest_unix_ms
            || decision_context.acquisition_time_latest_unix_ms
                >= specification.validity.effective_until_unix_ms
        {
            out_of_scope = true;
            reasons.insert("CLAIM_SPECIFICATION_NOT_EFFECTIVE_FOR_ACQUISITION_INTERVAL".to_owned());
        }

        if self.receiver_first_seen_earliest_unix_ms
            > self.receiver_first_seen_latest_unix_ms
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_RECEIVER_INTERVAL_REVERSED".to_owned());
        } else if self.receiver_first_seen_latest_unix_ms
            > decision_context.as_known_at_unix_ms
        {
            not_known_at_snapshot = true;
            reasons.insert("CLAIM_METRIC_BINDING_NOT_KNOWN_AT_SNAPSHOT".to_owned());
        }
        if self.receiver_first_seen_latest_unix_ms
            > owner_cut.receiver_receipt_watermark_unix_ms
        {
            unverified = true;
            reasons.insert("CLAIM_METRIC_BINDING_AFTER_OWNER_RECEIPT_WATERMARK".to_owned());
        }
        if owner_cut.receiver_receipt_watermark_unix_ms
            > decision_context.as_known_at_unix_ms
        {
            rejected = true;
            reasons.insert("OWNER_RECEIPT_WATERMARK_AFTER_DECISION_SNAPSHOT".to_owned());
        }
        if owner_cut.trust_policy_sha256 != decision_context.trust_policy_sha256 {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_OWNER_TRUST_POLICY_MISMATCH".to_owned());
        }

        let model_digest = match model_cut.compute_sha256() {
            Ok(value) => value,
            Err(errors) => {
                rejected = true;
                reasons.extend(errors);
                String::new()
            }
        };
        if !model_digest.is_empty() && self.model_credibility_sha256 != model_digest {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_CREDIBILITY_DIGEST_MISMATCH".to_owned());
        }
        if decision_context.model_applicability_profile_sha256
            != model_cut.applicability_profile_sha256
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_APPLICABILITY_MISMATCH".to_owned());
        }
        if decision_context.risk_classification_sha256 != model_cut.risk_classification_sha256 {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_RISK_CLASSIFICATION_MISMATCH".to_owned());
        }

        let owner_digest = match owner_cut.compute_sha256() {
            Ok(value) => value,
            Err(errors) => {
                rejected = true;
                reasons.extend(errors);
                String::new()
            }
        };
        if !owner_digest.is_empty() && self.owner_evidence_cut_sha256 != owner_digest {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_OWNER_CUT_DIGEST_MISMATCH".to_owned());
        }

        for (code, value) in [
            ("METRIC_DEFINITION", self.metric_definition_sha256.as_str()),
            ("MAPPING_BASIS", self.semantic_mapping_basis_sha256.as_str()),
            ("POPULATION_MANIFEST", self.population_manifest_sha256.as_str()),
            ("UNCERTAINTY_PROFILE", self.uncertainty_profile_sha256.as_str()),
        ] {
            if !valid_sha256(value) {
                rejected = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{code}_DIGEST_INVALID"));
            }
        }
        for (code, value) in [
            ("MODEL_METRIC_ID", self.model_metric_id.as_str()),
            ("MEASURAND_ID", self.measurand_id.as_str()),
            ("METRIC_DEFINITION_REF", self.metric_definition_ref.as_str()),
            ("SEMANTIC_MAPPING_REF", self.semantic_mapping_ref.as_str()),
            ("POPULATION_REF", self.population_ref.as_str()),
            ("SAMPLING_UNIT_REF", self.sampling_unit_ref.as_str()),
            ("UNCERTAINTY_METHOD_REF", self.uncertainty_method_ref.as_str()),
        ] {
            if value.trim().is_empty() {
                incomplete = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{code}_MISSING"));
            } else if !valid_identifier(value) {
                rejected = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{code}_NONCANONICAL"));
            }
        }

        if self.measurand_id != specification.measurand.measurand_id {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MEASURAND_MISMATCH".to_owned());
        }
        if self.unit_code != specification.measurand.unit_code
            || self.predicate_unit_code != specification.predicate.unit_code
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_UNIT_MISMATCH".to_owned());
        }
        if self.aggregation != specification.measurand.aggregation {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_AGGREGATION_MISMATCH".to_owned());
        }
        if self.population_ref != specification.population.population_ref
            || self.sampling_unit_ref != specification.population.sampling_unit_ref
            || self.population_manifest_sha256 != specification.population.population_manifest_sha256
            || self.population_quantifier != specification.population.quantifier
            || self.population_confidence_level_ppm != specification.population.confidence_level_ppm
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_POPULATION_SEMANTICS_MISMATCH".to_owned());
        }
        if self.predicate_operator != specification.predicate.operator
            || self.predicate_threshold != specification.predicate.threshold
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_PREDICATE_MISMATCH".to_owned());
        }
        if !self.predicate_threshold.is_canonical() {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_PREDICATE_NONCANONICAL".to_owned());
        }
        if self.uncertainty_treatment != specification.uncertainty.treatment
            || self.uncertainty_method_ref != specification.uncertainty.method_ref
            || self.uncertainty_confidence_level_ppm
                != specification.uncertainty.confidence_level_ppm
        {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_UNCERTAINTY_SEMANTICS_MISMATCH".to_owned());
        }

        if owner_cut.validation_errors().is_empty() {
            if !owner_cut.artifact_payloads_verified {
                unverified = true;
                reasons.insert("CLAIM_METRIC_BINDING_OWNER_PAYLOADS_UNVERIFIED".to_owned());
            }
        } else {
            rejected = true;
            reasons.extend(owner_cut.validation_errors());
        }

        let mut owner_records = BTreeMap::new();
        for record in &owner_cut.evidence_records {
            owner_records.insert(record.evidence_ref.as_str(), record);
        }
        let required_owner_records = [
            ("METRIC_SEMANTICS", self.metric_semantics_evidence_ref.as_str(), self.metric_semantics_record_sha256.as_str()),
            ("MODEL_REPORT", self.model_report_evidence_ref.as_str(), self.model_report_record_sha256.as_str()),
            ("OWNER_ATTESTATION", self.owner_attestation_evidence_ref.as_str(), self.owner_attestation_record_sha256.as_str()),
            ("PRODUCER_AUTHORITY_RECEIPT", self.producer_authority_receipt_ref.as_str(), self.producer_authority_receipt_sha256.as_str()),
            ("SEMANTIC_VALIDATION_RECEIPT", self.semantic_validation_receipt_ref.as_str(), self.semantic_validation_receipt_sha256.as_str()),
        ];
        let required_refs = self.required_evidence_refs.iter().map(String::as_str).collect::<BTreeSet<_>>();
        for (_, evidence_ref, _) in required_owner_records {
            if !required_refs.contains(evidence_ref) {
                rejected = true;
                reasons.insert("CLAIM_METRIC_BINDING_OWNER_RECEIPT_NOT_IN_REQUIRED_MANIFEST".to_owned());
            }
        }
        let mut distinct_refs = BTreeSet::new();
        for (role, evidence_ref, expected_record_sha256) in required_owner_records {
            if !valid_identifier(evidence_ref) || !valid_sha256(expected_record_sha256) {
                incomplete = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_REFERENCE_OR_DIGEST_MISSING"));
                continue;
            }
            if !distinct_refs.insert(evidence_ref) {
                rejected = true;
                reasons.insert("CLAIM_METRIC_BINDING_OWNER_RECORD_ROLE_REUSED".to_owned());
            }
            let Some(record) = owner_records.get(evidence_ref) else {
                unverified = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_OWNER_RECORD_MISSING"));
                continue;
            };
            if record.record_sha256 != expected_record_sha256 {
                rejected = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_OWNER_RECORD_DIGEST_MISMATCH"));
            }
            if record.subject_ref != specification.subject_ref
                || record.configuration_ref != specification.configuration_ref
            {
                rejected = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_OWNER_CONTEXT_MISMATCH"));
            }
            match record.lifecycle {
                OwnerEvidenceLifecycle::Active => {}
                OwnerEvidenceLifecycle::Superseded
                | OwnerEvidenceLifecycle::Disputed
                | OwnerEvidenceLifecycle::Revoked
                | OwnerEvidenceLifecycle::Withdrawn => {
                    rejected = true;
                    reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_OWNER_LIFECYCLE_NOT_ACTIVE"));
                }
            }
            match record.freshness {
                OwnerEvidenceFreshness::Fresh => {}
                OwnerEvidenceFreshness::StaleDirect | OwnerEvidenceFreshness::StaleTransitive => {
                    unverified = true;
                    reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_OWNER_EVIDENCE_STALE"));
                }
            }
            if record.dependency_coverage == OwnerDependencyCoverage::Incomplete {
                incomplete = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_DEPENDENCY_COVERAGE_INCOMPLETE"));
            }
            if !record.unresolved_inputs.is_empty() {
                incomplete = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_UNRESOLVED_INPUTS"));
            }
            if record.requires_reverification || !record.eligible_under_declared_coverage {
                unverified = true;
                reasons.insert(format!("CLAIM_METRIC_BINDING_{role}_REVERIFICATION_REQUIRED"));
            }
        }

        if self.model_report_evidence_ref != model_cut.evidence_ref {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_REPORT_REFERENCE_MISMATCH".to_owned());
        }

        let specification_strata = specification
            .population
            .required_strata
            .iter()
            .map(|item| item.stratum_id.as_str())
            .collect::<BTreeSet<_>>();
        let mut mapped_claim_strata = BTreeSet::new();
        let mut mapped_model_strata = BTreeSet::new();
        for item in &self.required_strata {
            if !valid_identifier(&item.claim_stratum_id)
                || !valid_identifier(&item.model_stratum_ref)
            {
                rejected = true;
                reasons.insert("CLAIM_METRIC_BINDING_STRATUM_REFERENCE_INVALID".to_owned());
            }
            if !mapped_claim_strata.insert(item.claim_stratum_id.as_str())
                || !mapped_model_strata.insert(item.model_stratum_ref.as_str())
            {
                rejected = true;
                reasons.insert("CLAIM_METRIC_BINDING_STRATUM_MAPPING_NOT_ONE_TO_ONE".to_owned());
            }
        }
        if mapped_claim_strata != specification_strata {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_REQUIRED_STRATUM_COVERAGE_MISMATCH".to_owned());
        }

        let expected_model_pairs = self
            .required_strata
            .iter()
            .map(|item| (self.model_metric_id.as_str(), item.model_stratum_ref.as_str()))
            .collect::<BTreeSet<_>>();
        let actual_required_pairs = model_cut
            .required_metric_strata
            .iter()
            .filter(|item| item.metric_id == self.model_metric_id)
            .map(|item| (item.metric_id.as_str(), item.stratum_ref.as_str()))
            .collect::<BTreeSet<_>>();
        if actual_required_pairs != expected_model_pairs {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_REQUIRED_STRATA_MISMATCH".to_owned());
        }

        let reports = model_cut
            .metric_stratum_reports
            .iter()
            .filter(|item| item.metric_id == self.model_metric_id)
            .map(|item| (item.stratum_ref.as_str(), item))
            .collect::<BTreeMap<_, _>>();
        let reported_strata = reports.keys().copied().collect::<BTreeSet<_>>();
        if reported_strata != mapped_model_strata {
            rejected = true;
            reasons.insert("CLAIM_METRIC_BINDING_MODEL_REPORT_SET_MISMATCH".to_owned());
        }
        for stratum_ref in mapped_model_strata {
            let Some(report) = reports.get(stratum_ref) else {
                incomplete = true;
                reasons.insert(format!(
                    "CLAIM_METRIC_BINDING_MODEL_REPORT_MISSING:{}",
                    stratum_ref
                ));
                continue;
            };
            if report.population_manifest_sha256 != self.population_manifest_sha256 {
                rejected = true;
                reasons.insert(format!(
                    "CLAIM_METRIC_BINDING_REPORT_POPULATION_MISMATCH:{}",
                    stratum_ref
                ));
            }
            match report.status {
                MetricStratumStatus::WithinDeclaredProfile => {}
                MetricStratumStatus::OutsideDeclaredProfile => {
                    rejected = true;
                    reasons.insert(format!(
                        "CLAIM_METRIC_BINDING_METRIC_OUTSIDE_PROFILE:{}",
                        stratum_ref
                    ));
                }
                MetricStratumStatus::Indeterminate => {
                    incomplete = true;
                    reasons.insert(format!(
                        "CLAIM_METRIC_BINDING_METRIC_INDETERMINATE:{}",
                        stratum_ref
                    ));
                }
                MetricStratumStatus::Unverified => {
                    unverified = true;
                    reasons.insert(format!(
                        "CLAIM_METRIC_BINDING_METRIC_UNVERIFIED:{}",
                        stratum_ref
                    ));
                }
            }
        }

        let disposition = if unsupported {
            ClaimMetricBindingDispositionV0_1::UnsupportedProfile
        } else if rejected {
            ClaimMetricBindingDispositionV0_1::Rejected
        } else if out_of_scope {
            ClaimMetricBindingDispositionV0_1::OutOfScope
        } else if not_known_at_snapshot {
            ClaimMetricBindingDispositionV0_1::NotKnownAtSnapshot
        } else if incomplete {
            ClaimMetricBindingDispositionV0_1::Incomplete
        } else if unverified {
            ClaimMetricBindingDispositionV0_1::Unverified
        } else if reasons.is_empty() {
            ClaimMetricBindingDispositionV0_1::SemanticsBound
        } else {
            ClaimMetricBindingDispositionV0_1::Rejected
        };

        ClaimMetricBindingAssessmentV0_1 {
            disposition,
            reason_codes: reasons.into_iter().collect(),
            claim_specification_semantic_sha256: semantic_digest,
            decision_context_sha256: context_digest,
            model_credibility_sha256: model_digest,
            owner_evidence_cut_sha256: owner_digest,
            owner_authentication_performed: false,
            physical_truth_established: false,
            engineering_acceptance_granted: false,
            execution_authority_granted: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claim_specification_v0_1::{
        ClaimSpecificationV0_1, ValidityIntervalV0_1,
    };
    use crate::decision_context_v0_1::compute_required_evidence_manifest_sha256;
    use crate::model_credibility_cut_v0_1::{
        MetricStratumReportV0_1, ModelCredibilityCutV0_1,
    };
    use crate::owner_evidence_cut_v0_1::OwnerEvidenceCutV0_1;

    fn sha(text: &str) -> String {
        sha256_hex(text.as_bytes())
    }

    fn owner_record(
        evidence_ref: &str,
        subject_ref: &str,
        configuration_ref: &str,
        digest: &str,
    ) -> OwnerEvidenceRecordV0_1 {
        OwnerEvidenceRecordV0_1 {
            evidence_ref: evidence_ref.to_owned(),
            record_sha256: digest.to_owned(),
            subject_ref: subject_ref.to_owned(),
            configuration_ref: configuration_ref.to_owned(),
            lifecycle: OwnerEvidenceLifecycle::Active,
            predecessor_ref: None,
            freshness: OwnerEvidenceFreshness::Fresh,
            dependency_coverage: OwnerDependencyCoverage::Complete,
            unresolved_inputs: Vec::new(),
            requires_reverification: false,
            eligible_under_declared_coverage: true,
        }
    }

    struct Fixture {
        specification: ClaimSpecificationV0_1,
        payload_sha256: String,
        context: DecisionContextV0_1,
        model_cut: ModelCredibilityCutV0_1,
        owner_cut: OwnerEvidenceCutV0_1,
        binding: ClaimMetricSemanticBindingV0_1,
    }

    fn fixture() -> Fixture {
        let mut specification: ClaimSpecificationV0_1 = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_claim_specification_v0_1.json"
        ))
        .expect("claim spec fixture");
        specification.claim_id = "claim-wing-panel-001".to_owned();
        specification.subject_ref = "part:wing-panel-001".to_owned();
        specification.configuration_ref = "configuration:wing-r17".to_owned();
        specification.operating_envelope_ref = "envelope:transport-normal".to_owned();
        specification.validity = ValidityIntervalV0_1 {
            effective_from_unix_ms: 1_795_514_399_900,
            effective_until_unix_ms: 1_798_668_800_000,
        };
        let payload_bytes = serde_json::to_vec(&specification).expect("serialize spec fixture");
        let payload_sha256 = sha256_hex(&payload_bytes);
        let semantic_sha256 = specification
            .compute_semantic_sha256()
            .expect("semantic spec fixture");

        let mut context = DecisionContextV0_1 {
            claim_id: specification.claim_id.clone(),
            subject_ref: specification.subject_ref.clone(),
            configuration_ref: specification.configuration_ref.clone(),
            operating_envelope_ref: specification.operating_envelope_ref.clone(),
            acquisition_time_earliest_unix_ms: 1_795_514_399_900,
            acquisition_time_latest_unix_ms: 1_795_514_400_100,
            as_known_at_unix_ms: 1_795_516_800_000,
            source_scope_manifest_sha256: "a".repeat(64),
            evidence_knowledge_cut_sha256: "0".repeat(64),
            trust_policy_sha256: "c".repeat(64),
            claim_specification_sha256: semantic_sha256.clone(),
            required_evidence_manifest_sha256: "0".repeat(64),
            risk_classification_sha256: "b".repeat(64),
            model_applicability_profile_sha256: "a".repeat(64),
        };

        let mut model_cut: ModelCredibilityCutV0_1 = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_model_credibility_cut_v0_1.json"
        ))
        .expect("model credibility cut fixture");
        model_cut.required_metric_strata = vec![
            crate::model_credibility_cut_v0_1::RequiredMetricStratumV0_1 {
                metric_id: "maximum_principal_stress".to_owned(),
                stratum_ref: "environment:temperature-ambient".to_owned(),
            },
            crate::model_credibility_cut_v0_1::RequiredMetricStratumV0_1 {
                metric_id: "maximum_principal_stress".to_owned(),
                stratum_ref: "load_case:limit-load".to_owned(),
            },
        ];
        context.model_applicability_profile_sha256 =
            model_cut.applicability_profile_sha256.clone();
        context.risk_classification_sha256 = model_cut.risk_classification_sha256.clone();
        model_cut.metric_stratum_reports = model_cut
            .required_metric_strata
            .iter()
            .map(|required| MetricStratumReportV0_1 {
                metric_id: required.metric_id.clone(),
                stratum_ref: required.stratum_ref.clone(),
                population_manifest_sha256: specification
                    .population
                    .population_manifest_sha256
                    .clone(),
                report_sha256: sha(&format!("report:{}:{}", required.metric_id, required.stratum_ref)),
                uncertainty_report_sha256: sha(&format!("uncertainty:{}:{}", required.metric_id, required.stratum_ref)),
                sample_count: 100,
                adverse_count: 0,
                status: MetricStratumStatus::WithinDeclaredProfile,
            })
            .collect();

        let mut owner_cut: OwnerEvidenceCutV0_1 = serde_json::from_str(include_str!(
            "../../../../docs/research/aviation/examples/aero_owner_evidence_cut_v0_1.json"
        ))
        .expect("owner evidence cut fixture");
        let role_refs = [
            "receipt:metric-semantics",
            model_cut.evidence_ref.as_str(),
            "receipt:metric-owner-attestation",
            "receipt:metric-authority-verification",
            "receipt:metric-semantic-validation",
        ];
        let mut records = owner_cut.evidence_records.clone();
        for (index, evidence_ref) in role_refs.iter().enumerate() {
            if records.iter().any(|record| record.evidence_ref == *evidence_ref) {
                continue;
            }
            records.push(owner_record(
                evidence_ref,
                &specification.subject_ref,
                &specification.configuration_ref,
                &format!("{:064x}", index + 20),
            ));
        }
        owner_cut.evidence_records = records;
        let model_record_sha256 = owner_cut
            .evidence_records
            .iter()
            .find(|record| record.evidence_ref == model_cut.evidence_ref)
            .map(|record| record.record_sha256.clone())
            .expect("model report owner record");
        let semantics_record_sha256 = owner_cut
            .evidence_records
            .iter()
            .find(|record| record.evidence_ref == "receipt:metric-semantics")
            .map(|record| record.record_sha256.clone())
            .expect("metric semantics owner record");
        let attestation_record_sha256 = owner_cut
            .evidence_records
            .iter()
            .find(|record| record.evidence_ref == "receipt:metric-owner-attestation")
            .map(|record| record.record_sha256.clone())
            .expect("owner attestation record");
        let authority_record_sha256 = owner_cut
            .evidence_records
            .iter()
            .find(|record| record.evidence_ref == "receipt:metric-authority-verification")
            .map(|record| record.record_sha256.clone())
            .expect("authority record");
        let semantic_validation_record_sha256 = owner_cut
            .evidence_records
            .iter()
            .find(|record| record.evidence_ref == "receipt:metric-semantic-validation")
            .map(|record| record.record_sha256.clone())
            .expect("semantic validation record");

        context.evidence_knowledge_cut_sha256 =
            owner_cut.compute_sha256().expect("owner cut fixture digest");
        context.required_evidence_manifest_sha256 =
            compute_required_evidence_manifest_sha256(&vec![
                "receipt:inspection-01".to_owned(),
                "receipt:calibration-02".to_owned(),
            ])
            .expect("required evidence manifest");
        let model_sha256 = model_cut.compute_sha256().expect("model cut digest");
        let owner_sha256 = owner_cut.compute_sha256().expect("owner cut digest");

        let required_evidence_refs = vec![
            "receipt:inspection-01".to_owned(),
            "receipt:calibration-02".to_owned(),
            "receipt:metric-semantics".to_owned(),
            model_cut.evidence_ref.clone(),
            "receipt:metric-owner-attestation".to_owned(),
            "receipt:metric-authority-verification".to_owned(),
            "receipt:metric-semantic-validation".to_owned(),
        ];
        context.required_evidence_manifest_sha256 =
            compute_required_evidence_manifest_sha256(&required_evidence_refs)
                .expect("expanded required evidence manifest");
        let context_sha256 = context.compute_sha256().expect("expanded context digest");
        let binding = ClaimMetricSemanticBindingV0_1 {
            schema: CLAIM_METRIC_BINDING_SCHEMA_V0_1.to_owned(),
            binding_ref: "aero:metric-binding:wing-panel-strength-001".to_owned(),
            claim_id: specification.claim_id.clone(),
            claim_specification_payload_sha256: payload_sha256.clone(),
            claim_specification_semantic_sha256: semantic_sha256,
            decision_context_sha256: context_sha256,
            model_credibility_sha256: model_sha256,
            owner_evidence_cut_sha256: owner_sha256,
            model_metric_id: "maximum_principal_stress".to_owned(),
            measurand_id: specification.measurand.measurand_id.clone(),
            metric_definition_ref: "aeroweift:metric:maximum-principal-stress".to_owned(),
            metric_definition_sha256: sha("maximum-principal-stress-definition"),
            semantic_mapping_ref: "aeroweift:mapping:stress-to-maximum-principal-stress".to_owned(),
            semantic_mapping_basis_sha256: sha("independently-reviewed-mapping-basis"),
            unit_code: specification.measurand.unit_code,
            aggregation: specification.measurand.aggregation,
            population_ref: specification.population.population_ref.clone(),
            sampling_unit_ref: specification.population.sampling_unit_ref.clone(),
            population_manifest_sha256: specification.population.population_manifest_sha256.clone(),
            population_quantifier: specification.population.quantifier,
            population_confidence_level_ppm: specification.population.confidence_level_ppm,
            predicate_operator: specification.predicate.operator,
            predicate_threshold: specification.predicate.threshold,
            predicate_unit_code: specification.predicate.unit_code,
            uncertainty_treatment: specification.uncertainty.treatment,
            uncertainty_method_ref: specification.uncertainty.method_ref.clone(),
            uncertainty_profile_sha256: sha("uncertainty-profile"),
            uncertainty_confidence_level_ppm: specification.uncertainty.confidence_level_ppm,
            required_strata: vec![
                ClaimMetricStratumBindingV0_1 {
                    claim_stratum_id: "stratum:temperature-ambient".to_owned(),
                    model_stratum_ref: "environment:temperature-ambient".to_owned(),
                },
                ClaimMetricStratumBindingV0_1 {
                    claim_stratum_id: "stratum:limit-load".to_owned(),
                    model_stratum_ref: "load_case:limit-load".to_owned(),
                },
            ],
            owner_namespace: owner_cut.owner_namespace.clone(),
            metric_semantics_evidence_ref: "receipt:metric-semantics".to_owned(),
            metric_semantics_record_sha256: semantics_record_sha256,
            model_report_evidence_ref: model_cut.evidence_ref.clone(),
            model_report_record_sha256: model_record_sha256,
            owner_attestation_evidence_ref: "receipt:metric-owner-attestation".to_owned(),
            owner_attestation_record_sha256: attestation_record_sha256,
            producer_authority_receipt_ref: "receipt:metric-authority-verification".to_owned(),
            producer_authority_receipt_sha256: authority_record_sha256,
            semantic_validation_receipt_ref: "receipt:metric-semantic-validation".to_owned(),
            semantic_validation_receipt_sha256: semantic_validation_record_sha256,
            receiver_first_seen_earliest_unix_ms: 1_795_516_500_000,
            receiver_first_seen_latest_unix_ms: 1_795_516_600_000,
        };
        Fixture {
            specification,
            payload_sha256,
            context,
            model_cut,
            owner_cut,
            binding,
        }
    }

    fn assess(fixture: &Fixture) -> ClaimMetricBindingAssessmentV0_1 {
        fixture.binding.assess(
            &fixture.specification,
            &fixture.payload_sha256,
            &fixture.context,
            &fixture.model_cut,
            &fixture.owner_cut,
        )
    }

    #[test]
    fn exact_claim_metric_semantics_are_bound_without_granting_authority() {
        let fixture = fixture();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::SemanticsBound);
        assert!(!result.owner_authentication_performed);
        assert!(!result.physical_truth_established);
        assert!(!result.engineering_acceptance_granted);
        assert!(!result.execution_authority_granted);
    }

    #[test]
    fn wrong_model_metric_is_rejected_even_when_model_and_configuration_match() {
        let mut fixture = fixture();
        fixture.binding.model_metric_id = "maximum_displacement".to_owned();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Rejected);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_MODEL_REQUIRED_STRATA_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn unit_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.unit_code = UnitCodeV0_1::Pascal;
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_UNIT_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn aggregation_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.aggregation = AggregationV0_1::Mean;
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_AGGREGATION_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn population_manifest_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.population_manifest_sha256 = sha("different-population");
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_POPULATION_SEMANTICS_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn population_quantifier_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.population_quantifier = PopulationQuantifierV0_1::EveryRequiredUnit;
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_POPULATION_SEMANTICS_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn predicate_threshold_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.predicate_threshold.numerator += 1;
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_PREDICATE_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn uncertainty_method_substitution_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.uncertainty_method_ref = "method:unrelated".to_owned();
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_UNCERTAINTY_SEMANTICS_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn required_stratum_mapping_must_be_complete_and_one_to_one() {
        let mut missing = fixture();
        missing.binding.required_strata.pop();
        let result = assess(&missing);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_REQUIRED_STRATUM_COVERAGE_MISMATCH".to_owned()
        ));

        let mut duplicate = fixture();
        duplicate.binding.required_strata[1].model_stratum_ref =
            duplicate.binding.required_strata[0].model_stratum_ref.clone();
        let result = assess(&duplicate);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_STRATUM_MAPPING_NOT_ONE_TO_ONE".to_owned()
        ));
    }

    #[test]
    fn model_report_population_must_match_the_claim_population() {
        let mut fixture = fixture();
        fixture.model_cut.metric_stratum_reports[0].population_manifest_sha256 =
            sha("another-population");
        fixture.binding.model_credibility_sha256 =
            fixture.model_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert!(result.reason_codes.iter().any(|reason| {
            reason.starts_with("CLAIM_METRIC_BINDING_REPORT_POPULATION_MISMATCH:")
        }));
    }

    #[test]
    fn missing_model_stratum_report_is_incomplete() {
        let mut fixture = fixture();
        fixture.model_cut.metric_stratum_reports.pop();
        fixture.binding.model_credibility_sha256 =
            fixture.model_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Incomplete);
    }

    #[test]
    fn outside_profile_metric_stratum_rejects_binding() {
        let mut fixture = fixture();
        fixture.model_cut.metric_stratum_reports[0].status =
            MetricStratumStatus::OutsideDeclaredProfile;
        fixture.binding.model_credibility_sha256 =
            fixture.model_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Rejected);
    }

    #[test]
    fn indeterminate_metric_stratum_remains_incomplete() {
        let mut fixture = fixture();
        fixture.model_cut.metric_stratum_reports[0].status =
            MetricStratumStatus::Indeterminate;
        fixture.binding.model_credibility_sha256 =
            fixture.model_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Incomplete);
    }

    #[test]
    fn unverified_metric_stratum_remains_unverified() {
        let mut fixture = fixture();
        fixture.model_cut.metric_stratum_reports[0].status =
            MetricStratumStatus::Unverified;
        fixture.binding.model_credibility_sha256 =
            fixture.model_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Unverified);
    }

    #[test]
    fn stale_owner_semantics_receipt_cannot_be_used() {
        let mut fixture = fixture();
        let record = fixture.owner_cut.evidence_records.iter_mut()
            .find(|record| record.evidence_ref == fixture.binding.metric_semantics_evidence_ref)
            .unwrap();
        record.freshness = OwnerEvidenceFreshness::StaleDirect;
        fixture.binding.owner_evidence_cut_sha256 =
            fixture.owner_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Unverified);
    }

    #[test]
    fn unavailable_payload_verification_remains_unverified() {
        let mut fixture = fixture();
        fixture.owner_cut.artifact_payloads_verified = false;
        fixture.binding.owner_evidence_cut_sha256 =
            fixture.owner_cut.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::Unverified);
    }

    #[test]
    fn late_receiver_first_seen_is_excluded_from_historical_snapshot() {
        let mut fixture = fixture();
        fixture.binding.receiver_first_seen_latest_unix_ms =
            fixture.context.as_known_at_unix_ms + 1;
        let result = assess(&fixture);
        assert_eq!(
            result.disposition,
            ClaimMetricBindingDispositionV0_1::NotKnownAtSnapshot
        );
    }

    #[test]
    fn specification_effectivity_must_cover_the_entire_acquisition_interval() {
        let mut fixture = fixture();
        fixture.specification.validity.effective_until_unix_ms =
            fixture.context.acquisition_time_latest_unix_ms;
        let payload = serde_json::to_vec(&fixture.specification).unwrap();
        fixture.payload_sha256 = sha256_hex(&payload);
        let semantic = fixture.specification.compute_semantic_sha256().unwrap();
        fixture.binding.claim_specification_payload_sha256 = fixture.payload_sha256.clone();
        fixture.binding.claim_specification_semantic_sha256 = semantic.clone();
        fixture.context.claim_specification_sha256 = semantic;
        fixture.binding.decision_context_sha256 =
            fixture.context.compute_sha256().unwrap();
        let result = assess(&fixture);
        assert_eq!(result.disposition, ClaimMetricBindingDispositionV0_1::OutOfScope);
    }

    #[test]
    fn raw_and_semantic_specification_digests_are_not_interchangeable() {
        let mut fixture = fixture();
        fixture.binding.claim_specification_payload_sha256 =
            fixture.binding.claim_specification_semantic_sha256.clone();
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_SPECIFICATION_RAW_PAYLOAD_DIGEST_MISMATCH".to_owned()
        ));
    }

    #[test]
    fn owner_receipt_role_reuse_is_rejected() {
        let mut fixture = fixture();
        fixture.binding.producer_authority_receipt_ref =
            fixture.binding.owner_attestation_evidence_ref.clone();
        let result = assess(&fixture);
        assert!(result.reason_codes.contains(
            &"CLAIM_METRIC_BINDING_OWNER_RECORD_ROLE_REUSED".to_owned()
        ));
    }

    #[test]
    fn binding_digest_is_stable_and_sensitive_to_semantic_changes() {
        let fixture = fixture();
        let original = fixture.binding.compute_sha256().unwrap();
        let mut reversed = fixture.binding.clone();
        reversed.required_strata.reverse();
        assert_eq!(original, reversed.compute_sha256().unwrap());
        let mut changed = fixture.binding.clone();
        changed.predicate_threshold.numerator += 1;
        assert_ne!(original, changed.compute_sha256().unwrap());
    }
}
