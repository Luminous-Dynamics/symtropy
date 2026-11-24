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

const DCSA_TT_VERSION: &str = "DCSA-TT-2.2";
const EPCIS_VERSION: &str = "GS1-EPCIS-2.0.0";
const FIXTURE_DIGEST: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
    InvalidSourcePayloadDigest,
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
            Self::DscaClassifierCannotMapToBusinessStep => {
                "DCSA_CLASSIFIER_CANNOT_MAP_TO_BIZSTEP"
            }
            Self::DscaClassifierCannotMapToDisposition => {
                "DCSA_CLASSIFIER_CANNOT_MAP_TO_DISPOSITION"
            }
            Self::SourceStandardVersionMismatch => "SOURCE_STANDARD_VERSION_MISMATCH",
            Self::TargetStandardVersionMismatch => "TARGET_STANDARD_VERSION_MISMATCH",
            Self::SourceMessageIdentityMissing => "SOURCE_MESSAGE_IDENTITY_MISSING",
            Self::InvalidSourcePayloadDigest => "INVALID_SOURCE_PAYLOAD_DIGEST",
            Self::SourceFieldMissing => "SOURCE_FIELD_MISSING",
            Self::SourceValueMismatch => "SOURCE_VALUE_MISMATCH",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceMessage {
    pub standard_version: String,
    pub source_record_id: String,
    pub source_event_id: String,
    /// SHA-256 of the original, unmodified source message bytes.
    pub raw_payload_sha256: String,
    /// Already parsed source fields. Raw bytes remain authoritative and are
    /// identified by raw_payload_sha256; this map is not a replacement payload.
    pub fields: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingRule {
    pub rule_id: String,
    pub source_standard_version: String,
    pub source_field: String,
    pub source_value: String,
    pub target_standard_version: String,
    pub target_field: String,
    pub target_value: String,
    pub kind: MappingKind,
    pub reversibility: MappingReversibility,
    pub mapping_evidence_ref: String,
    /// Required for contextual and lossy mappings; describes the semantic gap.
    pub lossiness_reason: Option<String>,
}

impl MappingRule {
    pub fn validate(&self) -> Result<(), CrosswalkError> {
        if self.rule_id.trim().is_empty()
            || self.source_standard_version.trim().is_empty()
            || self.source_field.trim().is_empty()
            || self.source_value.trim().is_empty()
            || self.target_standard_version.trim().is_empty()
            || self.target_field.trim().is_empty()
            || self.target_value.trim().is_empty()
        {
            return Err(CrosswalkError::EmptyRuleIdentity);
        }
        if self.mapping_evidence_ref.trim().is_empty() {
            return Err(CrosswalkError::MissingMappingEvidence);
        }

        if self.source_standard_version == DCSA_TT_VERSION
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
            (MappingKind::Exact, MappingReversibility::Reversible) => {}
            (MappingKind::Exact, MappingReversibility::Irreversible) => {
                return Err(CrosswalkError::ExactMappingMustBeReversible);
            }
            (MappingKind::Contextual | MappingKind::Lossy, MappingReversibility::Reversible) => {
                return Err(CrosswalkError::NonExactMappingMustBeIrreversible);
            }
            (MappingKind::Contextual | MappingKind::Lossy, MappingReversibility::Irreversible) => {
                if self
                    .lossiness_reason
                    .as_deref()
                    .is_none_or(str::is_empty)
                {
                    return Err(CrosswalkError::MissingLossinessReason);
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrosswalkReceipt {
    pub rule_id: String,
    pub source_standard_version: String,
    pub source_record_id: String,
    pub source_event_id: String,
    pub source_payload_sha256: String,
    pub source_field: String,
    pub source_value: String,
    pub target_standard_version: String,
    pub target_field: String,
    pub target_value: String,
    pub kind: MappingKind,
    pub reversibility: MappingReversibility,
    pub mapping_evidence_ref: String,
    pub lossiness_reason: Option<String>,
    /// The original source payload is always retained by reference, not rewritten.
    pub source_payload_preserved: bool,
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
    if source.source_record_id.trim().is_empty() || source.source_event_id.trim().is_empty() {
        return Err(CrosswalkError::SourceMessageIdentityMissing);
    }
    if !valid_sha256(&source.raw_payload_sha256) {
        return Err(CrosswalkError::InvalidSourcePayloadDigest);
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
        source_standard_version: source.standard_version.clone(),
        source_record_id: source.source_record_id.clone(),
        source_event_id: source.source_event_id.clone(),
        source_payload_sha256: source.raw_payload_sha256.clone(),
        source_field: rule.source_field.clone(),
        source_value: source_value.clone(),
        target_standard_version: rule.target_standard_version.clone(),
        target_field: rule.target_field.clone(),
        target_value: rule.target_value.clone(),
        kind: rule.kind,
        reversibility: rule.reversibility,
        mapping_evidence_ref: rule.mapping_evidence_ref.clone(),
        lossiness_reason: rule.lossiness_reason.clone(),
        source_payload_preserved: true,
    })
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn source_fixture() -> SourceMessage {
    let mut fields = BTreeMap::new();
    fields.insert("eventClassifierCode".to_owned(), "ACT".to_owned());
    fields.insert("eventTypeCode".to_owned(), "LOAD".to_owned());
    SourceMessage {
        standard_version: DCSA_TT_VERSION.to_owned(),
        source_record_id: "DCSA-SOURCE-RECORD-001".to_owned(),
        source_event_id: "urn:uuid:11111111-1111-4111-8111-111111111111".to_owned(),
        raw_payload_sha256: FIXTURE_DIGEST.to_owned(),
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
        source_standard_version: DCSA_TT_VERSION.to_owned(),
        source_field: source_field.to_owned(),
        source_value: source_value.to_owned(),
        target_standard_version: EPCIS_VERSION.to_owned(),
        target_field: target_field.to_owned(),
        target_value: target_value.to_owned(),
        kind,
        reversibility,
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

fn cases() -> [Case; 10] {
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
    ]
}

pub fn render_csv() -> String {
    let mut output = String::from(
        "case_id,expected_mapping_kind,observed_status,target_field,source_payload_preserved,reversibility,claim_ceiling\n",
    );
    for case in cases() {
        let mut source = source_fixture();
        let mapping_rule = match case.id {
            "event_type_to_bizstep_contextual" => rule(
                "eventTypeCode",
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
                "eventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                Some("contextual mapping"),
            ),
            "lossy_mapping_with_explicit_reason" => rule(
                "eventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                Some("source-specific subreason omitted by target vocabulary"),
            ),
            "lossy_mapping_without_reason_rejected" => rule(
                "eventTypeCode",
                "LOAD",
                case.target_field,
                "urn:epcglobal:cbv:bizstep:loading",
                case.kind,
                case.reversibility,
                None,
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
            "source_payload_digest_invalid" => source.raw_payload_sha256 = "not-a-digest".to_owned(),
            "source_value_mismatch_rejected" => {
                source.fields.insert("eventClassifierCode".to_owned(), "PLN".to_owned());
            }
            _ => {}
        }

        let result = map_field(&source, EPCIS_VERSION, &mapping_rule);
        let (observed, preserved, reversibility) = match result {
            Ok(receipt) => (
                receipt.kind.label(),
                receipt.source_payload_preserved,
                receipt.reversibility.label(),
            ),
            Err(error) => (error.label(), true, "NONE"),
        };
        assert_eq!(observed, case.expected, "crosswalk case {}", case.id);
        let _ = writeln!(
            output,
            "{},{},{},{},{},{},SYNTHETIC_ONLY",
            case.id,
            case.expected,
            observed,
            case.target_field,
            preserved,
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
        assert!(mapped.source_payload_preserved);
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
    fn contextual_mapping_is_explicitly_irreversible() {
        let receipt = map_field(
            &source_fixture(),
            EPCIS_VERSION,
            &rule(
                "eventTypeCode",
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
            "eventTypeCode",
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
            "eventTypeCode",
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
        assert_eq!(first.lines().count(), 11);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 7);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("EXACT"));
        assert!(first.contains("CONTEXTUAL"));
        assert!(first.contains("DCSA_CLASSIFIER_CANNOT_MAP_TO_BIZSTEP"));
        assert!(first.contains("DCSA_CLASSIFIER_CANNOT_MAP_TO_DISPOSITION"));
        assert!(first.contains("source_payload_preserved"));
    }
}
