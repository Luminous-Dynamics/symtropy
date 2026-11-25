// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit DCSA ↔ GS1 EPCIS/CBV cross-standard field mapping.
//!
//! Source and target standards describe related but non-identical semantic
//! domains. This mapper therefore requires a version-pinned, reviewed mapping
//! rule for every transferred field, retains the source payload digest and
//! source-record identity, and labels contextual/lossy mappings as irreversible.
//!
//! In particular, DCSA eventClassifierCode (EST/REQ/PLN/ACT) describes
//! milestone maturity; it must never be silently mapped to EPCIS bizStep or
//! disposition, which describe business-step/disposition semantics.
//!
//! This is a mapping-contract reference implementation, not a full DCSA or
//! EPCIS parser, JSON-LD serializer, vocabulary registry or conformance suite.

use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;

const DCSA_TT_VERSION: &str = "DCSA-TT-2.2.0";
const EPCIS_VERSION: &str = "GS1-EPCIS-2.0.1";
const CBV_VERSION: &str = "GS1-CBV-2.0.0";
const FIXTURE_DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MAPPING_BUNDLE_DIGEST: &str =
    "c9d2b17e1ef041bb6787849eda725021502d9822ab4b6980f357152dd9016e96";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingKind {
    Exact,
    Contextual,
    Lossy,
}

impl MappingKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Exact => "EXACT",
            Self::Contextual => "CONTEXTUAL",
            Self::Lossy => "LOSSY",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingReversibility {
    Reversible,
    Irreversible,
}

impl MappingReversibility {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Reversible => "REVERSIBLE",
            Self::Irreversible => "IRREVERSIBLE",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrosswalkError {
    EmptyRuleIdentity,
    MissingMappingEvidence,
    MissingLossinessReason,
    ExactMappingMustBeReversible,
    NonExactMappingMustBeIrreversible,
    DscaClassifierCannotMapToBusinessStep,
    DscaClassifierCannotMapToDisposition,
    SourceStandardVersionMismatch,
    TargetStandardVersionMismatch,
    SourceMessageIdentityMissing,
    SourceSubjectIdentityMissing,
    SourcePayloadReferenceMissing,
    SourcePayloadReferenceMismatch,
    InvalidSourcePayloadDigest,
    MappingRuleNotApproved,
    ExternalApprovalRecordMissing,
    SyntheticRuleUsedForNonSyntheticSource,
    SourceEventFamilyMismatch,
    TargetVocabularyVersionMismatch,
    InvalidMappingBundleDigest,
    MappingBundleReferenceMissing,
    MappingBundleReferenceMismatch,
    ExactMappingValueChanged,
    InvalidTargetVocabularyValue,
    SourceFieldMissing,
    SourceValueMismatch,
}

impl CrosswalkError {
    pub const fn label(self) -> &'static str {
        match self {
            Self::EmptyRuleIdentity => "EMPTY_RULE_IDENTITY",
            Self::MissingMappingEvidence => "MAPPING_EVIDENCE_MISSING",
            Self::MissingLossinessReason => "LOSSINESS_REASON_MISSING",
            Self::ExactMappingMustBeReversible => "EXACT_MAPPING_NOT_REVERSIBLE",
            Self::NonExactMappingMustBeIrreversible => "NONEXACT_MAPPING_CLAIMS_REVERSIBILITY",
            Self::DscaClassifierCannotMapToBusinessStep => "DCSA_CLASSIFIER_CANNOT_MAP_TO_BIZSTEP",
            Self::DscaClassifierCannotMapToDisposition => {
                "DCSA_CLASSIFIER_CANNOT_MAP_TO_DISPOSITION"
            }
            Self::SourceStandardVersionMismatch => "SOURCE_STANDARD_VERSION_MISMATCH",
            Self::TargetStandardVersionMismatch => "TARGET_STANDARD_VERSION_MISMATCH",
            Self::SourceMessageIdentityMissing => "SOURCE_MESSAGE_IDENTITY_MISSING",
            Self::SourceSubjectIdentityMissing => "SOURCE_SUBJECT_IDENTITY_MISSING",
            Self::SourcePayloadReferenceMissing => "SOURCE_PAYLOAD_REFERENCE_MISSING",
            Self::SourcePayloadReferenceMismatch => "SOURCE_PAYLOAD_REFERENCE_MISMATCH",
            Self::InvalidSourcePayloadDigest => "INVALID_SOURCE_PAYLOAD_DIGEST",
            Self::MappingRuleNotApproved => "MAPPING_RULE_NOT_APPROVED",
            Self::ExternalApprovalRecordMissing => "EXTERNAL_APPROVAL_RECORD_MISSING",
            Self::SyntheticRuleUsedForNonSyntheticSource => {
                "SYNTHETIC_RULE_USED_FOR_NON_SYNTHETIC_SOURCE"
            }
            Self::SourceEventFamilyMismatch => "SOURCE_EVENT_FAMILY_MISMATCH",
            Self::TargetVocabularyVersionMismatch => "TARGET_VOCABULARY_VERSION_MISMATCH",
            Self::InvalidMappingBundleDigest => "INVALID_MAPPING_BUNDLE_DIGEST",
            Self::MappingBundleReferenceMissing => "MAPPING_BUNDLE_REFERENCE_MISSING",
            Self::MappingBundleReferenceMismatch => "MAPPING_BUNDLE_REFERENCE_MISMATCH",
            Self::ExactMappingValueChanged => "EXACT_MAPPING_VALUE_CHANGED",
            Self::InvalidTargetVocabularyValue => "INVALID_TARGET_VOCABULARY_VALUE",
            Self::SourceFieldMissing => "SOURCE_FIELD_MISSING",
            Self::SourceValueMismatch => "SOURCE_VALUE_MISMATCH",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMessage {
    pub standard_version: String,
    /// DCSA event family, e.g. EquipmentEvent or ShipmentEvent.
    pub event_family: String,
    pub source_record_id: String,
    pub source_event_id: String,
    /// Canonical container, vessel-call or shipment identity.
    pub subject_id: String,
    /// Synthetic fixtures are never eligible to support live-source claims.
    pub is_synthetic: bool,
    /// SHA-256 of the original, unmodified source message bytes.
    pub raw_payload_sha256: String,
    /// Immutable object-store or evidence-ledger reference. This module carries
    /// the reference and digest; it does not itself retain the source bytes.
    pub raw_payload_ref: String,
    /// Parsed source fields, never a replacement for the original message.
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingReviewStatus {
    SyntheticFixtureOnly,
    ExternallyApproved,
    Draft,
    Rejected,
}

impl MappingReviewStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::SyntheticFixtureOnly => "SYNTHETIC_FIXTURE_ONLY",
            Self::ExternallyApproved => "EXTERNALLY_APPROVED",
            Self::Draft => "DRAFT",
            Self::Rejected => "REJECTED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingRule {
    pub rule_id: String,
    /// Immutable mapping bundle identity and revision, not an implicit latest rule.
    pub mapping_bundle_id: String,
    pub mapping_bundle_revision: String,
    pub mapping_bundle_sha256: String,
    pub mapping_bundle_ref: String,
    pub source_standard_version: String,
    pub source_event_family: String,
    pub source_field: String,
    pub source_value: String,
    pub target_standard_version: String,
    /// Present only for fields governed by the GS1 Core Business Vocabulary.
    pub target_vocabulary_version: Option<String>,
    pub target_field: String,
    pub target_value: String,
    pub kind: MappingKind,
    pub reversibility: MappingReversibility,
    pub review_status: MappingReviewStatus,
    pub reviewer_ref: String,
    /// Required for externally approved rules; a reference only, not self-validating proof.
    pub approval_record_ref: Option<String>,
    pub mapping_evidence_ref: String,
    /// Required for contextual and lossy mappings; describes the semantic gap.
    pub lossiness_reason: Option<String>,
}

impl MappingRule {
    pub fn validate(&self) -> Result<(), CrosswalkError> {
        if self.rule_id.trim().is_empty()
            || self.source_standard_version.trim().is_empty()
            || self.source_event_family.trim().is_empty()
            || self.source_field.trim().is_empty()
            || self.source_value.trim().is_empty()
            || self.target_standard_version.trim().is_empty()
            || self.target_field.trim().is_empty()
            || self.target_value.trim().is_empty()
        {
            return Err(CrosswalkError::EmptyRuleIdentity);
        }
        if self.mapping_bundle_id.trim().is_empty()
            || self.mapping_bundle_revision.trim().is_empty()
            || self.reviewer_ref.trim().is_empty()
        {
            return Err(CrosswalkError::EmptyRuleIdentity);
        }
        if !valid_sha256(&self.mapping_bundle_sha256) {
            return Err(CrosswalkError::InvalidMappingBundleDigest);
        }
        if self.mapping_bundle_ref.trim().is_empty() {
            return Err(CrosswalkError::MappingBundleReferenceMissing);
        }
        let expected_bundle_ref =
            format!("urn:mapping-bundle:sha256:{}", self.mapping_bundle_sha256);
        if self.mapping_bundle_ref != expected_bundle_ref {
            return Err(CrosswalkError::MappingBundleReferenceMismatch);
        }
        match self.review_status {
            MappingReviewStatus::SyntheticFixtureOnly | MappingReviewStatus::ExternallyApproved => {}
            MappingReviewStatus::Draft | MappingReviewStatus::Rejected => {
                return Err(CrosswalkError::MappingRuleNotApproved);
            }
        }
        if self.review_status == MappingReviewStatus::ExternallyApproved
            && self
                .approval_record_ref
                .as_deref()
                .is_none_or(str::is_empty)
        {
            return Err(CrosswalkError::ExternalApprovalRecordMissing);
        }
        if self.mapping_evidence_ref.trim().is_empty() {
            return Err(CrosswalkError::MissingMappingEvidence);
        }

        if self.source_standard_version.starts_with("DCSA-")
            && self.source_field == "eventClassifierCode"
        {
            if self.target_field == "bizStep" {
                return Err(CrosswalkError::DscaClassifierCannotMapToBusinessStep);
            }
            if self.target_field == "disposition" {
                return Err(CrosswalkError::DscaClassifierCannotMapToDisposition);
            }
        }

        match (self.kind, self.reversibility) {
            (MappingKind::Exact, MappingReversibility::Reversible) => {
                if self.source_value != self.target_value {
                    return Err(CrosswalkError::ExactMappingValueChanged);
                }
            }
            (MappingKind::Exact, MappingReversibility::Irreversible) => {
                return Err(CrosswalkError::ExactMappingMustBeReversible);
            }
            (MappingKind::Contextual | MappingKind::Lossy, MappingReversibility::Reversible) => {
                return Err(CrosswalkError::NonExactMappingMustBeIrreversible);
            }
            (MappingKind::Contextual | MappingKind::Lossy, MappingReversibility::Irreversible) => {
                if self.lossiness_reason.as_deref().is_none_or(str::is_empty) {
                    return Err(CrosswalkError::MissingLossinessReason);
                }
            }
        }
        if self.target_field == "bizStep" || self.target_field == "disposition" {
            if self.target_vocabulary_version.as_deref() != Some(CBV_VERSION) {
                return Err(CrosswalkError::TargetVocabularyVersionMismatch);
            }
        } else if self.target_vocabulary_version.is_some() {
            return Err(CrosswalkError::TargetVocabularyVersionMismatch);
        }
        if self.target_field == "bizStep" && !is_supported_cbv_biz_step(&self.target_value) {
            return Err(CrosswalkError::InvalidTargetVocabularyValue);
        }
        if self.target_field == "disposition" && !is_supported_cbv_disposition(&self.target_value) {
            return Err(CrosswalkError::InvalidTargetVocabularyValue);
        }
        Ok(())
    }
}

fn is_supported_cbv_biz_step(value: &str) -> bool {
    let code = value
        .strip_prefix("urn:epcglobal:cbv:bizstep:")
        .or_else(|| value.strip_prefix("https://ref.gs1.org/cbv/BizStep-"));
    matches!(
        code,
        Some(
            "accepting"
                | "arriving"
                | "departing"
                | "loading"
                | "receiving"
                | "shipping"
                | "storing"
                | "transporting"
                | "unloading"
        )
    )
}

fn is_supported_cbv_disposition(value: &str) -> bool {
    let code = value
        .strip_prefix("urn:epcglobal:cbv:disp:")
        .or_else(|| value.strip_prefix("https://ref.gs1.org/cbv/Disp-"));
    matches!(
        code,
        Some(
            "available"
                | "damaged"
                | "destroyed"
                | "in_progress"
                | "in_transit"
                | "returned"
                | "stolen"
                | "unavailable"
        )
    )
}

fn is_uri(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once(':') else {
        return false;
    };
    !scheme.is_empty()
        && scheme.as_bytes()[0].is_ascii_alphabetic()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
        && !rest.is_empty()
        && !value.chars().any(char::is_whitespace)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrosswalkReceipt {
    pub rule_id: String,
    pub mapping_bundle_id: String,
    pub mapping_bundle_revision: String,
    pub mapping_bundle_sha256: String,
    pub mapping_bundle_ref: String,
    pub review_status: MappingReviewStatus,
    pub reviewer_ref: String,
    pub approval_record_ref: Option<String>,
    pub source_standard_version: String,
    pub source_event_family: String,
    pub source_record_id: String,
    pub source_event_id: String,
    pub source_subject_id: String,
    pub source_payload_sha256: String,
    pub source_payload_ref: String,
    pub source_field: String,
    pub source_value: String,
    pub target_standard_version: String,
    pub target_vocabulary_version: Option<String>,
    pub target_field: String,
    pub target_value: String,
    pub kind: MappingKind,
    pub reversibility: MappingReversibility,
    pub mapping_evidence_ref: String,
    pub lossiness_reason: Option<String>,
    /// Digest and object-store reference are carried; bytes remain in source storage.
    pub source_payload_digest_carried: bool,
    pub source_payload_ref_carried: bool,
}

pub fn map_field(
    source: &SourceMessage,
    target_standard_version: &str,
    rule: &MappingRule,
) -> Result<CrosswalkReceipt, CrosswalkError> {
    rule.validate()?;

    if source.standard_version != rule.source_standard_version {
        return Err(CrosswalkError::SourceStandardVersionMismatch);
    }
    if target_standard_version != rule.target_standard_version {
        return Err(CrosswalkError::TargetStandardVersionMismatch);
    }
    if source.event_family != rule.source_event_family {
        return Err(CrosswalkError::SourceEventFamilyMismatch);
    }
    if rule.review_status == MappingReviewStatus::SyntheticFixtureOnly && !source.is_synthetic {
        return Err(CrosswalkError::SyntheticRuleUsedForNonSyntheticSource);
    }
    if source.source_record_id.trim().is_empty() || source.source_event_id.trim().is_empty() {
        return Err(CrosswalkError::SourceMessageIdentityMissing);
    }
    if source.subject_id.trim().is_empty() || !is_uri(&source.subject_id) {
        return Err(CrosswalkError::SourceSubjectIdentityMissing);
    }
    if source.raw_payload_ref.trim().is_empty() {
        return Err(CrosswalkError::SourcePayloadReferenceMissing);
    }
    if !valid_sha256(&source.raw_payload_sha256) {
        return Err(CrosswalkError::InvalidSourcePayloadDigest);
    }
    let expected_payload_ref = format!("urn:source-artifact:sha256:{}", source.raw_payload_sha256);
    if source.raw_payload_ref != expected_payload_ref {
        return Err(CrosswalkError::SourcePayloadReferenceMismatch);
    }

    let source_value = source
        .fields
        .get(&rule.source_field)
        .ok_or(CrosswalkError::SourceFieldMissing)?;
    if source_value != &rule.source_value {
        return Err(CrosswalkError::SourceValueMismatch);
    }

    Ok(CrosswalkReceipt {
        rule_id: rule.rule_id.clone(),
        mapping_bundle_id: rule.mapping_bundle_id.clone(),
        mapping_bundle_revision: rule.mapping_bundle_revision.clone(),
        mapping_bundle_sha256: rule.mapping_bundle_sha256.clone(),
        mapping_bundle_ref: rule.mapping_bundle_ref.clone(),
        review_status: rule.review_status,
        reviewer_ref: rule.reviewer_ref.clone(),
        approval_record_ref: rule.approval_record_ref.clone(),
        source_standard_version: source.standard_version.clone(),
        source_event_family: source.event_family.clone(),
        source_record_id: source.source_record_id.clone(),
        source_event_id: source.source_event_id.clone(),
        source_subject_id: source.subject_id.clone(),
        source_payload_sha256: source.raw_payload_sha256.clone(),
        source_payload_ref: source.raw_payload_ref.clone(),
        source_field: rule.source_field.clone(),
        source_value: source_value.clone(),
        target_standard_version: rule.target_standard_version.clone(),
        target_vocabulary_version: rule.target_vocabulary_version.clone(),
        target_field: rule.target_field.clone(),
        target_value: rule.target_value.clone(),
        kind: rule.kind,
        reversibility: rule.reversibility,
        mapping_evidence_ref: rule.mapping_evidence_ref.clone(),
        lossiness_reason: rule.lossiness_reason.clone(),
        source_payload_digest_carried: true,
        source_payload_ref_carried: true,
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn source_fixture() -> SourceMessage {
    let mut fields = BTreeMap::new();
    fields.insert("eventClassifierCode".to_owned(), "ACT".to_owned());
    fields.insert("equipmentEventTypeCode".to_owned(), "LOAD".to_owned());
    SourceMessage {
        standard_version: DCSA_TT_VERSION.to_owned(),
        event_family: "EquipmentEvent".to_owned(),
        source_record_id: "DCSA-SOURCE-RECORD-001".to_owned(),
        source_event_id: "urn:uuid:11111111-1111-4111-8111-111111111111".to_owned(),
        subject_id: "urn:epc:id:sscc:0614141.1234567890".to_owned(),
        is_synthetic: true,
        raw_payload_sha256: FIXTURE_DIGEST.to_owned(),
        raw_payload_ref: format!("urn:source-artifact:sha256:{FIXTURE_DIGEST}"),
        fields,
    }
}

fn rule(
    source_field: &str,
    source_value: &str,
    target_field: &str,
    target_value: &str,
    kind: MappingKind,
    reversibility: MappingReversibility,
    lossiness_reason: Option<&str>,
) -> MappingRule {
    MappingRule {
        rule_id: format!("crosswalk-{source_field}-to-{target_field}-v1"),
        mapping_bundle_id: "maritime-dcsa-epcis-crosswalk".to_owned(),
        mapping_bundle_revision: "v1.0.0".to_owned(),
        mapping_bundle_sha256: MAPPING_BUNDLE_DIGEST.to_owned(),
        mapping_bundle_ref: format!("urn:mapping-bundle:sha256:{MAPPING_BUNDLE_DIGEST}"),
        source_standard_version: DCSA_TT_VERSION.to_owned(),
        source_event_family: "EquipmentEvent".to_owned(),
        source_field: source_field.to_owned(),
        source_value: source_value.to_owned(),
        target_standard_version: EPCIS_VERSION.to_owned(),
        target_vocabulary_version: if target_field == "bizStep" || target_field == "disposition" {
            Some(CBV_VERSION.to_owned())
        } else {
            None
        },
        target_field: target_field.to_owned(),
        target_value: target_value.to_owned(),
        kind,
        reversibility,
        review_status: MappingReviewStatus::SyntheticFixtureOnly,
        reviewer_ref: "synthetic-reviewer:cross-standard-mapping-v1".to_owned(),
        approval_record_ref: None,
        mapping_evidence_ref: "synthetic-cross-standard-review-v1".to_owned(),
        lossiness_reason: lossiness_reason.map(str::to_owned),
    }
}

#[derive(Clone, Copy)]
struct Case {
    id: &'static str,
    expected: &'static str,
    target_field: &'static str,
    kind: MappingKind,
    reversibility: MappingReversibility,
}

fn cases() -> [Case; 25] {
    [
        Case {
            id: "classifier_to_extension_field_exact",
            expected: "EXACT",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "event_type_to_bizstep_contextual",
            expected: "CONTEXTUAL",
            target_field: "bizStep",
            kind: MappingKind::Contextual,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "classifier_to_bizstep_rejected",
            expected: "DCSA_CLASSIFIER_CANNOT_MAP_TO_BIZSTEP",
            target_field: "bizStep",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "classifier_to_disposition_rejected",
            expected: "DCSA_CLASSIFIER_CANNOT_MAP_TO_DISPOSITION",
            target_field: "disposition",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "nonexact_mapping_cannot_claim_reversible",
            expected: "NONEXACT_MAPPING_CLAIMS_REVERSIBILITY",
            target_field: "bizStep",
            kind: MappingKind::Contextual,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "lossy_mapping_with_explicit_reason",
            expected: "LOSSY",
            target_field: "bizStep",
            kind: MappingKind::Lossy,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "lossy_mapping_without_reason_rejected",
            expected: "LOSSINESS_REASON_MISSING",
            target_field: "bizStep",
            kind: MappingKind::Lossy,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "source_version_drift_rejected",
            expected: "SOURCE_STANDARD_VERSION_MISMATCH",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_payload_digest_invalid",
            expected: "INVALID_SOURCE_PAYLOAD_DIGEST",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_value_mismatch_rejected",
            expected: "SOURCE_VALUE_MISMATCH",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "exact_mapping_value_changed_rejected",
            expected: "EXACT_MAPPING_VALUE_CHANGED",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "draft_mapping_rule_rejected",
            expected: "MAPPING_RULE_NOT_APPROVED",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "unsupported_cbv_bizstep_rejected",
            expected: "INVALID_TARGET_VOCABULARY_VALUE",
            target_field: "bizStep",
            kind: MappingKind::Contextual,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "mapping_bundle_digest_invalid",
            expected: "INVALID_MAPPING_BUNDLE_DIGEST",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_subject_identity_missing",
            expected: "SOURCE_SUBJECT_IDENTITY_MISSING",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_payload_reference_missing",
            expected: "SOURCE_PAYLOAD_REFERENCE_MISSING",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_payload_reference_mismatch",
            expected: "SOURCE_PAYLOAD_REFERENCE_MISMATCH",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "mapping_bundle_reference_missing",
            expected: "MAPPING_BUNDLE_REFERENCE_MISSING",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "mapping_bundle_reference_mismatch",
            expected: "MAPPING_BUNDLE_REFERENCE_MISMATCH",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "synthetic_rule_cannot_map_non_synthetic_source",
            expected: "SYNTHETIC_RULE_USED_FOR_NON_SYNTHETIC_SOURCE",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "source_event_family_mismatch_rejected",
            expected: "SOURCE_EVENT_FAMILY_MISMATCH",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "cbv_version_mismatch_rejected",
            expected: "TARGET_VOCABULARY_VERSION_MISMATCH",
            target_field: "bizStep",
            kind: MappingKind::Contextual,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "cbv_version_missing_rejected",
            expected: "TARGET_VOCABULARY_VERSION_MISMATCH",
            target_field: "bizStep",
            kind: MappingKind::Contextual,
            reversibility: MappingReversibility::Irreversible,
        },
        Case {
            id: "rejected_mapping_rule_rejected",
            expected: "MAPPING_RULE_NOT_APPROVED",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
        Case {
            id: "external_approval_record_missing_rejected",
            expected: "EXTERNAL_APPROVAL_RECORD_MISSING",
            target_field: "sourceMaturity",
            kind: MappingKind::Exact,
            reversibility: MappingReversibility::Reversible,
        },
    ]
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,expected_mapping_kind,observed_status,source_event_family,target_field,target_standard_version,target_vocabulary_version,mapping_bundle_revision,mapping_bundle_sha256,review_status,source_payload_digest_carried,reversibility,claim_ceiling\n",
    );
    for case in cases() {
        let mut source = source_fixture();
        let mut mapping_rule = match case.id {
            "event_type_to_bizstep_contextual" => rule(
                "equipmentEventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                Some("DCSA LOAD event type and EPCIS loading bizStep require reviewed context"),
            ),
            "classifier_to_bizstep_rejected" | "classifier_to_disposition_rejected" => rule(
                "eventClassifierCode",
                "ACT",
                case.target_field,
                "urn:epcglobal:cbv:disp:in_progress",
                case.kind,
                case.reversibility,
                None,
            ),
            "nonexact_mapping_cannot_claim_reversible" => rule(
                "equipmentEventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                Some("contextual mapping"),
            ),
            "lossy_mapping_with_explicit_reason" => rule(
                "equipmentEventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                Some("source-specific subreason omitted by target vocabulary"),
            ),
            "lossy_mapping_without_reason_rejected" => rule(
                "equipmentEventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                None,
            ),
            "exact_mapping_value_changed_rejected" => rule(
                "eventClassifierCode",
                "ACT",
                case.target_field,
                "actual",
                case.kind,
                case.reversibility,
                None,
            ),
            "unsupported_cbv_bizstep_rejected" => rule(
                "equipmentEventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:not_a_bizstep",
                case.kind,
                case.reversibility,
                Some("synthetic unsupported vocabulary value"),
            ),
            _ => rule(
                "eventClassifierCode",
                "ACT",
                case.target_field,
                "ACT",
                case.kind,
                case.reversibility,
                None,
            ),
        };
        match case.id {
            "source_version_drift_rejected" => source.standard_version = "DCSA-TT-2.1".to_owned(),
            "source_payload_digest_invalid" => {
                source.raw_payload_sha256 = "not-a-digest".to_owned()
            }
            "source_value_mismatch_rejected" => {
                source
                    .fields
                    .insert("eventClassifierCode".to_owned(), "PLN".to_owned());
            }
            "draft_mapping_rule_rejected" => {
                mapping_rule.review_status = MappingReviewStatus::Draft;
            }
            "mapping_bundle_digest_invalid" => {
                mapping_rule.mapping_bundle_sha256 = "not-a-digest".to_owned();
            }
            "mapping_bundle_reference_missing" => mapping_rule.mapping_bundle_ref.clear(),
            "mapping_bundle_reference_mismatch" => {
                mapping_rule.mapping_bundle_ref =
                    "urn:mapping-bundle:sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".to_owned();
            }
            "synthetic_rule_cannot_map_non_synthetic_source" => {
                source.is_synthetic = false;
            }
            "source_event_family_mismatch_rejected" => {
                source.event_family = "ShipmentEvent".to_owned();
            }
            "cbv_version_mismatch_rejected" => {
                mapping_rule.target_vocabulary_version = Some("GS1-CBV-1.0.0".to_owned());
            }
            "cbv_version_missing_rejected" => {
                mapping_rule.target_vocabulary_version = None;
            }
            "rejected_mapping_rule_rejected" => {
                mapping_rule.review_status = MappingReviewStatus::Rejected;
            }
            "external_approval_record_missing_rejected" => {
                mapping_rule.review_status = MappingReviewStatus::ExternallyApproved;
                mapping_rule.approval_record_ref = None;
            }
            "source_subject_identity_missing" => source.subject_id.clear(),
            "source_payload_reference_missing" => source.raw_payload_ref.clear(),
            "source_payload_reference_mismatch" => {
                source.raw_payload_ref =
                    "urn:source-artifact:sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned();
            }
            _ => {}
        }

        let result = map_field(&source, EPCIS_VERSION, &mapping_rule);
        let (observed, digest_carried, reversibility) = match result {
            Ok(receipt) => (
                receipt.kind.label(),
                receipt.source_payload_digest_carried,
                receipt.reversibility.label(),
            ),
            Err(error) => (error.label(), false, "NONE"),
        };
        assert_eq!(observed, case.expected, "crosswalk case {}", case.id);
        let _ = writeln!(
            output,
            "{},{},{},{},{},{},{},{},{},{},{},{},SYNTHETIC_ONLY",
            case.id,
            case.expected,
            observed,
            source.event_family,
            case.target_field,
            mapping_rule.target_standard_version,
            mapping_rule.target_vocabulary_version.as_deref().unwrap_or("NONE"),
            mapping_rule.mapping_bundle_revision,
            mapping_rule.mapping_bundle_sha256,
            mapping_rule.review_status.label(),
            digest_carried,
            reversibility
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_mapping_preserves_source_payload_and_record_identity() {
        let source = source_fixture();
        let mapped = map_field(
            &source,
            EPCIS_VERSION,
            &rule(
                "eventClassifierCode",
                "ACT",
                "sourceMaturity",
                "ACT",
                MappingKind::Exact,
                MappingReversibility::Reversible,
                None,
            ),
        )
        .expect("exact reviewed mapping");
        assert_eq!(mapped.source_record_id, source.source_record_id);
        assert_eq!(mapped.source_event_id, source.source_event_id);
        assert_eq!(mapped.source_payload_sha256, source.raw_payload_sha256);
        assert_eq!(mapped.source_value, "ACT");
        assert_eq!(mapped.target_value, "ACT");
        assert!(mapped.source_payload_digest_carried);
        assert_eq!(mapped.reversibility, MappingReversibility::Reversible);
    }

    #[test]
    fn classifier_maturity_cannot_be_converted_to_bizstep_or_disposition() {
        for target in ["bizStep", "disposition"] {
            let mapping_rule = rule(
                "eventClassifierCode",
                "ACT",
                target,
                "urn:epcglobal:cbv:disp:in_progress",
                MappingKind::Exact,
                MappingReversibility::Reversible,
                None,
            );
            let expected = if target == "bizStep" {
                CrosswalkError::DscaClassifierCannotMapToBusinessStep
            } else {
                CrosswalkError::DscaClassifierCannotMapToDisposition
            };
            assert_eq!(mapping_rule.validate(), Err(expected));
        }
    }

    #[test]
    fn classifier_firewall_covers_other_dsca_releases() {
        let mut future_release = rule(
            "eventClassifierCode",
            "ACT",
            "bizStep",
            "urn:epcglobal:cbv:bizstep:loading",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        future_release.source_standard_version = "DCSA-PC-2.0.0".to_owned();
        assert_eq!(
            future_release.validate(),
            Err(CrosswalkError::DscaClassifierCannotMapToBusinessStep)
        );
    }

    #[test]
    fn contextual_mapping_is_explicitly_irreversible() {
        let receipt = map_field(
            &source_fixture(),
            EPCIS_VERSION,
            &rule(
                "equipmentEventTypeCode",
                "LOAD",
                "bizStep",
                "urn:epcglobal:cbv:bizstep:loading",
                MappingKind::Contextual,
                MappingReversibility::Irreversible,
                Some("source event and target business step require contextual review"),
            ),
        )
        .expect("contextual mapping with lossiness recorded");
        assert_eq!(receipt.kind, MappingKind::Contextual);
        assert_eq!(receipt.reversibility, MappingReversibility::Irreversible);
        assert!(receipt.lossiness_reason.is_some());
    }

    #[test]
    fn exact_mapping_cannot_be_marked_irreversible_without_reasoning() {
        assert_eq!(
            rule(
                "eventClassifierCode",
                "ACT",
                "sourceMaturity",
                "ACT",
                MappingKind::Exact,
                MappingReversibility::Irreversible,
                None,
            )
            .validate(),
            Err(CrosswalkError::ExactMappingMustBeReversible)
        );
    }

    #[test]
    fn a_lossy_mapping_requires_a_reason_and_cannot_claim_reversibility() {
        let missing_reason = rule(
            "equipmentEventTypeCode",
            "LOAD",
            "bizStep",
            "urn:epcglobal:cbv:bizstep:loading",
            MappingKind::Lossy,
            MappingReversibility::Irreversible,
            None,
        );
        assert_eq!(
            missing_reason.validate(),
            Err(CrosswalkError::MissingLossinessReason)
        );

        let false_reversibility = rule(
            "equipmentEventTypeCode",
            "LOAD",
            "bizStep",
            "urn:epcglobal:cbv:bizstep:loading",
            MappingKind::Lossy,
            MappingReversibility::Reversible,
            Some("source context omitted"),
        );
        assert_eq!(
            false_reversibility.validate(),
            Err(CrosswalkError::NonExactMappingMustBeIrreversible)
        );
    }

    #[test]
    fn mapping_rule_requires_a_review_evidence_reference() {
        let mut candidate = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        candidate.mapping_evidence_ref.clear();
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::MissingMappingEvidence)
        );
    }

    #[test]
    fn mapping_bundle_reference_must_match_its_digest() {
        let mut candidate = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        candidate.mapping_bundle_ref.clear();
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::MappingBundleReferenceMissing)
        );
        candidate.mapping_bundle_ref =
            "urn:mapping-bundle:sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                .to_owned();
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::MappingBundleReferenceMismatch)
        );
    }

    #[test]
    fn source_payload_reference_must_match_raw_digest() {
        let mut source = source_fixture();
        source.raw_payload_ref.clear();
        let exact = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SourcePayloadReferenceMissing)
        );
        source.raw_payload_ref =
            "urn:source-artifact:sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                .to_owned();
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SourcePayloadReferenceMismatch)
        );
    }

    #[test]
    fn source_subject_identity_is_mandatory_for_every_mapped_field() {
        let mut source = source_fixture();
        source.subject_id.clear();
        let exact = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SourceSubjectIdentityMissing)
        );
    }

    #[test]
    fn mapping_bundle_digest_and_exact_value_are_verified() {
        let mut candidate = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        candidate.mapping_bundle_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::InvalidMappingBundleDigest)
        );
        candidate.mapping_bundle_sha256 = MAPPING_BUNDLE_DIGEST.to_owned();
        candidate.target_value = "actual".to_owned();
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::ExactMappingValueChanged)
        );
    }

    #[test]
    fn source_event_family_is_part_of_the_mapping_key() {
        let mut source = source_fixture();
        source.event_family = "ShipmentEvent".to_owned();
        let exact = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SourceEventFamilyMismatch)
        );
    }

    #[test]
    fn fixture_only_mapping_cannot_be_reused_for_live_source_messages() {
        let mut source = source_fixture();
        source.is_synthetic = false;
        let exact = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SyntheticRuleUsedForNonSyntheticSource)
        );
    }

    #[test]
    fn external_mapping_requires_an_external_approval_record_reference() {
        let source = {
            let mut source = source_fixture();
            source.is_synthetic = false;
            source
        };
        let mut external_rule = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        external_rule.review_status = MappingReviewStatus::ExternallyApproved;
        external_rule.approval_record_ref = None;
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &external_rule),
            Err(CrosswalkError::ExternalApprovalRecordMissing)
        );

        external_rule.approval_record_ref =
            Some("urn:review-record:synthetic-example".to_owned());
        assert!(
            map_field(&source, EPCIS_VERSION, &external_rule).is_ok(),
            "the adapter checks that an approval reference is declared; governance must resolve and verify it"
        );
    }

    #[test]
    fn cbv_target_version_is_pinned_separately_from_epcis_version() {
        let mut candidate = rule(
            "equipmentEventTypeCode",
            "LOAD",
            "bizStep",
            "urn:epcglobal:cbv:bizstep:loading",
            MappingKind::Contextual,
            MappingReversibility::Irreversible,
            Some("contextual equipment-event translation"),
        );
        candidate.target_vocabulary_version = Some("GS1-CBV-1.0.0".to_owned());
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::TargetVocabularyVersionMismatch)
        );
        candidate.target_vocabulary_version = None;
        assert_eq!(
            candidate.validate(),
            Err(CrosswalkError::TargetVocabularyVersionMismatch)
        );
    }

    #[test]
    fn source_and_target_versions_are_pinned() {
        let source = source_fixture();
        let exact = rule(
            "eventClassifierCode",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, "GS1-EPCIS-2.0.1", &exact),
            Err(CrosswalkError::TargetStandardVersionMismatch)
        );
        let mut drifted = source;
        drifted.standard_version = "DCSA-TT-2.1".to_owned();
        assert_eq!(
            map_field(&drifted, EPCIS_VERSION, &exact),
            Err(CrosswalkError::SourceStandardVersionMismatch)
        );
    }

    #[test]
    fn source_field_and_value_must_match_exactly() {
        let source = source_fixture();
        let missing = rule(
            "missingField",
            "ACT",
            "sourceMaturity",
            "ACT",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &missing),
            Err(CrosswalkError::SourceFieldMissing)
        );
        let mismatch = rule(
            "eventClassifierCode",
            "PLN",
            "sourceMaturity",
            "PLN",
            MappingKind::Exact,
            MappingReversibility::Reversible,
            None,
        );
        assert_eq!(
            map_field(&source, EPCIS_VERSION, &mismatch),
            Err(CrosswalkError::SourceValueMismatch)
        );
    }

    #[test]
    fn receipt_is_deterministic_and_contains_every_mapping_boundary() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 26);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 13);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("EXACT"));
        assert!(first.contains("CONTEXTUAL"));
        assert!(first.contains("DCSA_CLASSIFIER_CANNOT_MAP_TO_BIZSTEP"));
        assert!(first.contains("DCSA_CLASSIFIER_CANNOT_MAP_TO_DISPOSITION"));
        assert!(first.contains("MAPPING_RULE_NOT_APPROVED"));
        assert!(first.contains("INVALID_TARGET_VOCABULARY_VALUE"));
        assert!(first.contains("SOURCE_PAYLOAD_REFERENCE_MISMATCH"));
        assert!(first.contains("MAPPING_BUNDLE_REFERENCE_MISSING"));
        assert!(first.contains("MAPPING_BUNDLE_REFERENCE_MISMATCH"));
        assert!(first.contains("SYNTHETIC_RULE_USED_FOR_NON_SYNTHETIC_SOURCE"));
        assert!(first.contains("SOURCE_EVENT_FAMILY_MISMATCH"));
        assert!(first.contains("TARGET_VOCABULARY_VERSION_MISMATCH"));
        assert!(first.contains("EXTERNAL_APPROVAL_RECORD_MISSING"));
        assert!(first.contains("GS1-EPCIS-2.0.1"));
        assert!(first.contains("GS1-CBV-2.0.0"));
        assert!(first.contains("source_payload_digest_carried"));
    }
}
