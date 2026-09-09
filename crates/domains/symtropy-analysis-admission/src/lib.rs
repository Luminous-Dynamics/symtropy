// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admission boundary between faceted analysis qualification and downstream authority.
//!
//! A1 can derive that a coherent set of qualification assertions satisfies a
//! consumer profile. A2 adds a second boundary: every required facet assertion
//! must also resolve through one exact, sealed verification registry before the
//! qualification may become an admitted downstream input.
//!
//! The bootstrap registry is not itself a cryptographic verifier. It is the
//! dependency-light authority surface into which Xenia, signed registries,
//! laboratory PKI, certification systems, or other verifier implementations can
//! deposit exact verification receipts. Constructing bootstrap input is not
//! proof; runtime consumers must select and retain one exact sealed registry.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_analysis_contracts::{
    AnalysisEvidence, AnalysisEvidenceRef, AnalysisRequest, ExactSemanticRef,
};
use symtropy_analysis_qualification::{
    AnalysisQualificationCut, FacetAssessment, QualificationOutcome, QualificationProfile,
    QualificationProfileRef,
};
use symtropy_design::ContentDigest;
use symtropy_game_state::StableId;

pub const ADMISSION_POLICY_SCHEMA_VERSION: u32 = 1;
pub const VERIFICATION_REGISTRY_SCHEMA_VERSION: u32 = 1;
pub const ADMISSION_RECEIPT_SCHEMA_VERSION: u32 = 1;

const POLICY_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.admission-policy.v1\0";
const REGISTRY_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.verification-registry.v1\0";
const RECEIPT_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.admission-receipt.v1\0";
const SHA256_ALGORITHM_ID: &str = "sha256";

macro_rules! stable_id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(StableId);

        impl $name {
            pub const fn new(id: StableId) -> Self {
                Self(id)
            }

            pub const fn stable_id(&self) -> &StableId {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

stable_id_type!(AdmissionPolicyId);
stable_id_type!(VerificationRegistryId);
stable_id_type!(VerificationRecordId);
stable_id_type!(AdmissionReceiptId);

/// Exact verifier/issuer requirements for one A1 qualification facet.
///
/// `issuer_profile` pins the exact semantics/configuration of the authority
/// allowed to issue the A1 assertion. `verifier_profile` separately pins the
/// exact semantics/configuration used to authenticate that assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetAdmissionRule {
    pub facet_id: StableId,
    pub issuer_profile: ExactSemanticRef,
    pub verifier_profile: ExactSemanticRef,
}

impl FacetAdmissionRule {
    pub fn new(
        facet_id: StableId,
        issuer_profile: ExactSemanticRef,
        verifier_profile: ExactSemanticRef,
    ) -> Result<Self, AdmissionError> {
        let rule = Self {
            facet_id,
            issuer_profile,
            verifier_profile,
        };
        rule.validate()?;
        Ok(rule)
    }

    fn validate(&self) -> Result<(), AdmissionError> {
        validate_stable_id(&self.facet_id)?;
        self.issuer_profile
            .validate()
            .map_err(AdmissionError::Analysis)?;
        self.verifier_profile
            .validate()
            .map_err(AdmissionError::Analysis)
    }
}

/// Exact consumer admission policy layered over one exact A1 profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationAdmissionPolicy {
    schema_version: u32,
    id: AdmissionPolicyId,
    revision: u64,
    qualification_profile: QualificationProfileRef,
    facet_rules: Vec<FacetAdmissionRule>,
}

impl QualificationAdmissionPolicy {
    pub fn new(
        id: AdmissionPolicyId,
        revision: u64,
        profile: &QualificationProfile,
        mut facet_rules: Vec<FacetAdmissionRule>,
    ) -> Result<Self, AdmissionError> {
        facet_rules.sort_by(|left, right| left.facet_id.cmp(&right.facet_id));
        let policy = Self {
            schema_version: ADMISSION_POLICY_SCHEMA_VERSION,
            id,
            revision,
            qualification_profile: profile.exact_ref()?,
            facet_rules,
        };
        policy.validate_against(profile)?;
        Ok(policy)
    }

    pub fn id(&self) -> &AdmissionPolicyId {
        &self.id
    }

    pub fn facet_rules(&self) -> &[FacetAdmissionRule] {
        &self.facet_rules
    }

    pub fn exact_ref(&self) -> Result<AdmissionPolicyRef, AdmissionError> {
        Ok(AdmissionPolicyRef {
            id: self.id.clone(),
            revision: self.revision,
            content_digest: self.content_digest()?,
        })
    }

    pub fn content_digest(&self) -> Result<ContentDigest, AdmissionError> {
        self.validate_structure()?;
        sha256_digest(&self.canonical_preimage()?)
    }

    pub fn validate_against(
        &self,
        profile: &QualificationProfile,
    ) -> Result<(), AdmissionError> {
        self.validate_structure()?;
        let profile_ref = profile.exact_ref()?;
        if self.qualification_profile != profile_ref {
            return Err(AdmissionError::QualificationProfileMismatch);
        }
        if self.facet_rules.len() != profile.rules().len() {
            return Err(AdmissionError::IncompleteFacetPolicy {
                required: profile.rules().len(),
                actual: self.facet_rules.len(),
            });
        }
        for profile_rule in profile.rules() {
            let Some(rule) = self.rule(&profile_rule.facet_id) else {
                return Err(AdmissionError::MissingFacetPolicy(
                    profile_rule.facet_id.clone(),
                ));
            };
            if rule.issuer_profile.authority_id != profile_rule.required_authority_id {
                return Err(AdmissionError::IssuerAuthorityMismatch {
                    facet_id: profile_rule.facet_id.clone(),
                    expected: profile_rule.required_authority_id.clone(),
                    actual: rule.issuer_profile.authority_id.clone(),
                });
            }
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), AdmissionError> {
        if self.schema_version != ADMISSION_POLICY_SCHEMA_VERSION {
            return Err(AdmissionError::UnsupportedAdmissionPolicySchema(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        self.qualification_profile.validate()?;
        if self.facet_rules.is_empty() {
            return Err(AdmissionError::FacetPolicyRequired);
        }
        let mut previous: Option<&StableId> = None;
        for rule in &self.facet_rules {
            rule.validate()?;
            if let Some(previous) = previous {
                if previous >= &rule.facet_id {
                    return if previous == &rule.facet_id {
                        Err(AdmissionError::DuplicateFacetPolicy(rule.facet_id.clone()))
                    } else {
                        Err(AdmissionError::NonCanonicalOrder("policy.facet_rules"))
                    };
                }
            }
            previous = Some(&rule.facet_id);
        }
        Ok(())
    }

    fn rule(&self, facet_id: &StableId) -> Option<&FacetAdmissionRule> {
        self.facet_rules
            .binary_search_by(|rule| rule.facet_id.cmp(facet_id))
            .ok()
            .map(|index| &self.facet_rules[index])
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, AdmissionError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(POLICY_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        encode_qualification_profile_ref(&mut bytes, &self.qualification_profile)?;
        encode_len(&mut bytes, self.facet_rules.len())?;
        for rule in &self.facet_rules {
            encode_stable_id(&mut bytes, &rule.facet_id);
            encode_exact_ref(&mut bytes, &rule.issuer_profile)?;
            encode_exact_ref(&mut bytes, &rule.verifier_profile)?;
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AdmissionPolicyRef {
    pub id: AdmissionPolicyId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

/// Settlement state of one externally produced verification receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Revoked,
    Superseded,
}

/// Exact bootstrap input representing authentication performed by another
/// authority. Runtime code cannot infer this record from an A1 assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetVerificationInput {
    pub id: VerificationRecordId,
    pub facet_id: StableId,
    pub qualification_cut_digest: ContentDigest,
    pub subject_evidence: AnalysisEvidenceRef,
    pub assessment_attestation: ExactSemanticRef,
    pub issuer_profile: ExactSemanticRef,
    pub verifier_profile: ExactSemanticRef,
    pub verification_receipt: ExactSemanticRef,
    pub status: VerificationStatus,
}

impl FacetVerificationInput {
    fn validate(&self) -> Result<(), AdmissionError> {
        validate_stable_id(self.id.stable_id())?;
        validate_stable_id(&self.facet_id)?;
        validate_digest(&self.qualification_cut_digest)?;
        validate_evidence_ref(&self.subject_evidence)?;
        self.assessment_attestation
            .validate()
            .map_err(AdmissionError::Analysis)?;
        self.issuer_profile
            .validate()
            .map_err(AdmissionError::Analysis)?;
        self.verifier_profile
            .validate()
            .map_err(AdmissionError::Analysis)?;
        self.verification_receipt
            .validate()
            .map_err(AdmissionError::Analysis)
    }
}

/// Exact sealed registry of externally verified facet assertions.
///
/// The registry is intentionally immutable after sealing. A changed verifier,
/// receipt, revocation status, or policy produces a new exact registry identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRegistry {
    schema_version: u32,
    id: VerificationRegistryId,
    revision: u64,
    policy: AdmissionPolicyRef,
    records: Vec<FacetVerificationInput>,
}

impl VerificationRegistry {
    pub fn seal(
        id: VerificationRegistryId,
        revision: u64,
        policy: &QualificationAdmissionPolicy,
        mut records: Vec<FacetVerificationInput>,
    ) -> Result<Self, AdmissionError> {
        records.sort_by(compare_record_key);
        let registry = Self {
            schema_version: VERIFICATION_REGISTRY_SCHEMA_VERSION,
            id,
            revision,
            policy: policy.exact_ref()?,
            records,
        };
        registry.validate_against(policy)?;
        Ok(registry)
    }

    pub fn records(&self) -> &[FacetVerificationInput] {
        &self.records
    }

    pub fn exact_ref(
        &self,
        policy: &QualificationAdmissionPolicy,
    ) -> Result<VerificationRegistryRef, AdmissionError> {
        Ok(VerificationRegistryRef {
            id: self.id.clone(),
            revision: self.revision,
            content_digest: self.content_digest(policy)?,
        })
    }

    pub fn content_digest(
        &self,
        policy: &QualificationAdmissionPolicy,
    ) -> Result<ContentDigest, AdmissionError> {
        self.validate_against(policy)?;
        sha256_digest(&self.canonical_preimage()?)
    }

    pub fn validate_against(
        &self,
        policy: &QualificationAdmissionPolicy,
    ) -> Result<(), AdmissionError> {
        if self.schema_version != VERIFICATION_REGISTRY_SCHEMA_VERSION {
            return Err(AdmissionError::UnsupportedVerificationRegistrySchema(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        if self.policy != policy.exact_ref()? {
            return Err(AdmissionError::AdmissionPolicyMismatch);
        }

        let mut previous: Option<&FacetVerificationInput> = None;
        for record in &self.records {
            record.validate()?;
            let Some(rule) = policy.rule(&record.facet_id) else {
                return Err(AdmissionError::UnknownFacet(record.facet_id.clone()));
            };
            if record.issuer_profile != rule.issuer_profile {
                return Err(AdmissionError::IssuerProfileMismatch(
                    record.facet_id.clone(),
                ));
            }
            if record.verifier_profile != rule.verifier_profile {
                return Err(AdmissionError::VerifierProfileMismatch(
                    record.facet_id.clone(),
                ));
            }
            if let Some(previous) = previous {
                match compare_record_key(previous, record) {
                    std::cmp::Ordering::Greater => {
                        return Err(AdmissionError::NonCanonicalOrder("registry.records"));
                    }
                    std::cmp::Ordering::Equal => {
                        return Err(AdmissionError::DuplicateVerificationRecord {
                            cut_digest: record.qualification_cut_digest.clone(),
                            facet_id: record.facet_id.clone(),
                        });
                    }
                    std::cmp::Ordering::Less => {}
                }
            }
            previous = Some(record);
        }
        Ok(())
    }

    fn record_for(
        &self,
        cut_digest: &ContentDigest,
        facet_id: &StableId,
    ) -> Option<&FacetVerificationInput> {
        self.records.iter().find(|record| {
            &record.qualification_cut_digest == cut_digest && &record.facet_id == facet_id
        })
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, AdmissionError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(REGISTRY_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        encode_admission_policy_ref(&mut bytes, &self.policy)?;
        encode_len(&mut bytes, self.records.len())?;
        for record in &self.records {
            encode_stable_id(&mut bytes, record.id.stable_id());
            encode_stable_id(&mut bytes, &record.facet_id);
            encode_digest(&mut bytes, &record.qualification_cut_digest)?;
            encode_evidence_ref(&mut bytes, &record.subject_evidence)?;
            encode_exact_ref(&mut bytes, &record.assessment_attestation)?;
            encode_exact_ref(&mut bytes, &record.issuer_profile)?;
            encode_exact_ref(&mut bytes, &record.verifier_profile)?;
            encode_exact_ref(&mut bytes, &record.verification_receipt)?;
            bytes.push(match record.status {
                VerificationStatus::Verified => 0,
                VerificationStatus::Revoked => 1,
                VerificationStatus::Superseded => 2,
            });
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct VerificationRegistryRef {
    pub id: VerificationRegistryId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

/// Downstream-admissible qualification capsule.
///
/// Fields are private and there is no free constructor. The only V0 production
/// path is `admit_qualification`, which recomputes A1 and resolves every facet
/// through the exact current sealed verification registry.
///
/// Deliberately **not** `Deserialize`: after persistence a consumer must replay
/// admission from the exact request/evidence/profile/cut/policy/registry (or a
/// future independently authenticated receipt boundary). Bytes shaped like a
/// prior verdict do not regain authority merely by being restored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdmittedAnalysisQualification {
    schema_version: u32,
    id: AdmissionReceiptId,
    policy: AdmissionPolicyRef,
    verification_registry: VerificationRegistryRef,
    qualification_profile: QualificationProfileRef,
    qualification_cut_digest: ContentDigest,
    subject_evidence: AnalysisEvidenceRef,
    facet_verification_receipts: Vec<ExactSemanticRef>,
    content_digest: ContentDigest,
}

impl AdmittedAnalysisQualification {
    pub fn id(&self) -> &AdmissionReceiptId {
        &self.id
    }

    pub fn subject_evidence(&self) -> &AnalysisEvidenceRef {
        &self.subject_evidence
    }

    pub fn qualification_profile(&self) -> &QualificationProfileRef {
        &self.qualification_profile
    }

    pub fn content_digest(&self) -> &ContentDigest {
        &self.content_digest
    }

    pub fn validate_current(
        &self,
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
        cut: &AnalysisQualificationCut,
        policy: &QualificationAdmissionPolicy,
        registry: &VerificationRegistry,
    ) -> Result<(), AdmissionError> {
        let reproduced = build_admission(
            self.id.clone(),
            request,
            evidence,
            profile,
            cut,
            policy,
            registry,
        )?;
        if &reproduced == self {
            Ok(())
        } else {
            Err(AdmissionError::AdmissionReceiptStale)
        }
    }
}

/// Recompute A1 and admit it only after every required facet has one exact
/// current external verification record.
pub fn admit_qualification(
    id: AdmissionReceiptId,
    request: &AnalysisRequest,
    evidence: &AnalysisEvidence,
    profile: &QualificationProfile,
    cut: &AnalysisQualificationCut,
    policy: &QualificationAdmissionPolicy,
    registry: &VerificationRegistry,
) -> Result<AdmittedAnalysisQualification, AdmissionError> {
    build_admission(id, request, evidence, profile, cut, policy, registry)
}

fn build_admission(
    id: AdmissionReceiptId,
    request: &AnalysisRequest,
    evidence: &AnalysisEvidence,
    profile: &QualificationProfile,
    cut: &AnalysisQualificationCut,
    policy: &QualificationAdmissionPolicy,
    registry: &VerificationRegistry,
) -> Result<AdmittedAnalysisQualification, AdmissionError> {
    validate_stable_id(id.stable_id())?;
    policy.validate_against(profile)?;
    registry.validate_against(policy)?;

    let decision = cut.decision(request, evidence, profile)?;
    if decision.overall != QualificationOutcome::Established {
        return Err(AdmissionError::QualificationNotEstablished(decision.overall));
    }

    let cut_digest = cut.content_digest(request, evidence, profile)?;
    let evidence_ref = evidence.exact_ref(request).map_err(AdmissionError::Analysis)?;
    let mut verification_receipts = Vec::with_capacity(profile.rules().len());

    for profile_rule in profile.rules() {
        let assessment = find_assessment(cut.assessments(), &profile_rule.facet_id)
            .ok_or_else(|| AdmissionError::MissingAssessment(profile_rule.facet_id.clone()))?;
        let admission_rule = policy
            .rule(&profile_rule.facet_id)
            .ok_or_else(|| AdmissionError::MissingFacetPolicy(profile_rule.facet_id.clone()))?;
        let verification = registry
            .record_for(&cut_digest, &profile_rule.facet_id)
            .ok_or_else(|| AdmissionError::VerificationRequired(profile_rule.facet_id.clone()))?;

        validate_verification_matches(
            assessment,
            admission_rule,
            verification,
            &cut_digest,
            &evidence_ref,
        )?;
        if verification.status != VerificationStatus::Verified {
            return Err(AdmissionError::VerificationNotCurrent {
                facet_id: profile_rule.facet_id.clone(),
                status: verification.status,
            });
        }
        verification_receipts.push(verification.verification_receipt.clone());
    }

    let policy_ref = policy.exact_ref()?;
    let registry_ref = registry.exact_ref(policy)?;
    let profile_ref = profile.exact_ref()?;

    let mut receipt = AdmittedAnalysisQualification {
        schema_version: ADMISSION_RECEIPT_SCHEMA_VERSION,
        id,
        policy: policy_ref,
        verification_registry: registry_ref,
        qualification_profile: profile_ref,
        qualification_cut_digest: cut_digest,
        subject_evidence: evidence_ref,
        facet_verification_receipts: verification_receipts,
        content_digest: placeholder_digest()?,
    };
    receipt.content_digest = admission_receipt_digest(&receipt)?;
    Ok(receipt)
}

fn validate_verification_matches(
    assessment: &FacetAssessment,
    rule: &FacetAdmissionRule,
    verification: &FacetVerificationInput,
    cut_digest: &ContentDigest,
    evidence_ref: &AnalysisEvidenceRef,
) -> Result<(), AdmissionError> {
    if verification.facet_id != assessment.facet_id {
        return Err(AdmissionError::VerificationFacetMismatch(
            assessment.facet_id.clone(),
        ));
    }
    if &verification.qualification_cut_digest != cut_digest {
        return Err(AdmissionError::VerificationCutMismatch(
            assessment.facet_id.clone(),
        ));
    }
    if &verification.subject_evidence != evidence_ref
        || verification.subject_evidence != assessment.subject_evidence
    {
        return Err(AdmissionError::VerificationSubjectMismatch(
            assessment.facet_id.clone(),
        ));
    }
    if verification.assessment_attestation != assessment.attestation {
        return Err(AdmissionError::AssessmentAttestationMismatch(
            assessment.facet_id.clone(),
        ));
    }
    if verification.issuer_profile != rule.issuer_profile {
        return Err(AdmissionError::IssuerProfileMismatch(
            assessment.facet_id.clone(),
        ));
    }
    if verification.verifier_profile != rule.verifier_profile {
        return Err(AdmissionError::VerifierProfileMismatch(
            assessment.facet_id.clone(),
        ));
    }
    Ok(())
}

fn find_assessment<'a>(
    assessments: &'a [FacetAssessment],
    facet_id: &StableId,
) -> Option<&'a FacetAssessment> {
    assessments
        .binary_search_by(|assessment| assessment.facet_id.cmp(facet_id))
        .ok()
        .map(|index| &assessments[index])
}

fn admission_receipt_digest(
    receipt: &AdmittedAnalysisQualification,
) -> Result<ContentDigest, AdmissionError> {
    if receipt.schema_version != ADMISSION_RECEIPT_SCHEMA_VERSION {
        return Err(AdmissionError::UnsupportedAdmissionReceiptSchema(
            receipt.schema_version,
        ));
    }
    let mut bytes = Vec::new();
    bytes.extend_from_slice(RECEIPT_DIGEST_DOMAIN);
    bytes.extend_from_slice(&receipt.schema_version.to_le_bytes());
    encode_stable_id(&mut bytes, receipt.id.stable_id());
    encode_admission_policy_ref(&mut bytes, &receipt.policy)?;
    encode_verification_registry_ref(&mut bytes, &receipt.verification_registry)?;
    encode_qualification_profile_ref(&mut bytes, &receipt.qualification_profile)?;
    encode_digest(&mut bytes, &receipt.qualification_cut_digest)?;
    encode_evidence_ref(&mut bytes, &receipt.subject_evidence)?;
    encode_len(&mut bytes, receipt.facet_verification_receipts.len())?;
    for verification_receipt in &receipt.facet_verification_receipts {
        encode_exact_ref(&mut bytes, verification_receipt)?;
    }
    sha256_digest(&bytes)
}

fn compare_record_key(
    left: &FacetVerificationInput,
    right: &FacetVerificationInput,
) -> std::cmp::Ordering {
    (&left.qualification_cut_digest, &left.facet_id)
        .cmp(&(&right.qualification_cut_digest, &right.facet_id))
}

fn encode_admission_policy_ref(
    bytes: &mut Vec<u8>,
    reference: &AdmissionPolicyRef,
) -> Result<(), AdmissionError> {
    validate_stable_id(reference.id.stable_id())?;
    encode_stable_id(bytes, reference.id.stable_id());
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_verification_registry_ref(
    bytes: &mut Vec<u8>,
    reference: &VerificationRegistryRef,
) -> Result<(), AdmissionError> {
    validate_stable_id(reference.id.stable_id())?;
    encode_stable_id(bytes, reference.id.stable_id());
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_qualification_profile_ref(
    bytes: &mut Vec<u8>,
    reference: &QualificationProfileRef,
) -> Result<(), AdmissionError> {
    reference.validate()?;
    encode_stable_id(bytes, reference.id.stable_id());
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn validate_evidence_ref(reference: &AnalysisEvidenceRef) -> Result<(), AdmissionError> {
    validate_stable_id(reference.id.stable_id())?;
    validate_digest(&reference.content_digest)
}

fn encode_evidence_ref(
    bytes: &mut Vec<u8>,
    reference: &AnalysisEvidenceRef,
) -> Result<(), AdmissionError> {
    validate_evidence_ref(reference)?;
    encode_stable_id(bytes, reference.id.stable_id());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_exact_ref(
    bytes: &mut Vec<u8>,
    reference: &ExactSemanticRef,
) -> Result<(), AdmissionError> {
    reference.validate().map_err(AdmissionError::Analysis)?;
    encode_stable_id(bytes, &reference.authority_id);
    encode_stable_id(bytes, &reference.subject_id);
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn validate_digest(digest: &ContentDigest) -> Result<(), AdmissionError> {
    digest.validate().map_err(AdmissionError::Design)
}

fn encode_digest(bytes: &mut Vec<u8>, digest: &ContentDigest) -> Result<(), AdmissionError> {
    validate_digest(digest)?;
    encode_stable_id(bytes, &digest.algorithm);
    encode_string(bytes, &digest.value)
}

fn sha256_digest(bytes: &[u8]) -> Result<ContentDigest, AdmissionError> {
    ContentDigest::new(
        StableId::parse(SHA256_ALGORITHM_ID)
            .expect("sha256 is a valid stable identifier literal"),
        hex(&Sha256::digest(bytes)),
    )
    .map_err(AdmissionError::Design)
}

fn placeholder_digest() -> Result<ContentDigest, AdmissionError> {
    ContentDigest::new(
        StableId::parse(SHA256_ALGORITHM_ID)
            .expect("sha256 is a valid stable identifier literal"),
        "0",
    )
    .map_err(AdmissionError::Design)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) {
    encode_string(bytes, id.as_str()).expect("StableId length is bounded well below u64");
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), AdmissionError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), AdmissionError> {
    let len = u64::try_from(len).map_err(|_| AdmissionError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), AdmissionError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| AdmissionError::InvalidStableId(id.as_str().to_string()))
}

fn hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug)]
pub enum AdmissionError {
    InvalidStableId(String),
    UnsupportedAdmissionPolicySchema(u32),
    UnsupportedVerificationRegistrySchema(u32),
    UnsupportedAdmissionReceiptSchema(u32),
    FacetPolicyRequired,
    DuplicateFacetPolicy(StableId),
    IncompleteFacetPolicy { required: usize, actual: usize },
    MissingFacetPolicy(StableId),
    IssuerAuthorityMismatch {
        facet_id: StableId,
        expected: StableId,
        actual: StableId,
    },
    QualificationProfileMismatch,
    AdmissionPolicyMismatch,
    UnknownFacet(StableId),
    IssuerProfileMismatch(StableId),
    VerifierProfileMismatch(StableId),
    DuplicateVerificationRecord {
        cut_digest: ContentDigest,
        facet_id: StableId,
    },
    QualificationNotEstablished(QualificationOutcome),
    MissingAssessment(StableId),
    VerificationRequired(StableId),
    VerificationNotCurrent {
        facet_id: StableId,
        status: VerificationStatus,
    },
    VerificationFacetMismatch(StableId),
    VerificationCutMismatch(StableId),
    VerificationSubjectMismatch(StableId),
    AssessmentAttestationMismatch(StableId),
    AdmissionReceiptStale,
    NonCanonicalOrder(&'static str),
    LengthOverflow,
    Qualification(symtropy_analysis_qualification::QualificationError),
    Analysis(symtropy_analysis_contracts::AnalysisError),
    Design(symtropy_design::DesignError),
}

impl From<symtropy_analysis_qualification::QualificationError> for AdmissionError {
    fn from(value: symtropy_analysis_qualification::QualificationError) -> Self {
        Self::Qualification(value)
    }
}

impl fmt::Display for AdmissionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::UnsupportedAdmissionPolicySchema(version) => {
                write!(formatter, "unsupported admission policy schema {version}")
            }
            Self::UnsupportedVerificationRegistrySchema(version) => {
                write!(formatter, "unsupported verification registry schema {version}")
            }
            Self::UnsupportedAdmissionReceiptSchema(version) => {
                write!(formatter, "unsupported admission receipt schema {version}")
            }
            Self::FacetPolicyRequired => write!(formatter, "admission policy requires at least one facet rule"),
            Self::DuplicateFacetPolicy(id) => write!(formatter, "duplicate admission facet policy {id}"),
            Self::IncompleteFacetPolicy { required, actual } => write!(
                formatter,
                "admission policy has {actual} facet rules but exact qualification profile requires {required}"
            ),
            Self::MissingFacetPolicy(id) => write!(formatter, "missing admission policy for facet {id}"),
            Self::IssuerAuthorityMismatch { facet_id, expected, actual } => write!(
                formatter,
                "facet {facet_id} requires A1 authority {expected}, admission issuer profile names {actual}"
            ),
            Self::QualificationProfileMismatch => write!(formatter, "admission policy does not bind the supplied exact A1 profile"),
            Self::AdmissionPolicyMismatch => write!(formatter, "verification registry does not bind the supplied exact admission policy"),
            Self::UnknownFacet(id) => write!(formatter, "verification record references unknown facet {id}"),
            Self::IssuerProfileMismatch(id) => write!(formatter, "verification record changed exact issuer profile for facet {id}"),
            Self::VerifierProfileMismatch(id) => write!(formatter, "verification record changed exact verifier profile for facet {id}"),
            Self::DuplicateVerificationRecord { cut_digest, facet_id } => write!(
                formatter,
                "duplicate verification record for cut {}:{} facet {facet_id}",
                cut_digest.algorithm,
                cut_digest.value
            ),
            Self::QualificationNotEstablished(outcome) => write!(formatter, "A1 qualification is not established: {outcome:?}"),
            Self::MissingAssessment(id) => write!(formatter, "established A1 cut unexpectedly lacks facet assessment {id}"),
            Self::VerificationRequired(id) => write!(formatter, "facet {id} has no exact admitted verification record"),
            Self::VerificationNotCurrent { facet_id, status } => write!(
                formatter,
                "facet {facet_id} verification status is {status:?}, not Verified"
            ),
            Self::VerificationFacetMismatch(id) => write!(formatter, "verification record facet does not match A1 facet {id}"),
            Self::VerificationCutMismatch(id) => write!(formatter, "verification record does not bind exact A1 cut for facet {id}"),
            Self::VerificationSubjectMismatch(id) => write!(formatter, "verification record does not bind exact A0 evidence for facet {id}"),
            Self::AssessmentAttestationMismatch(id) => write!(formatter, "verification record does not bind exact A1 attestation for facet {id}"),
            Self::AdmissionReceiptStale => write!(formatter, "admitted qualification no longer matches current exact inputs"),
            Self::NonCanonicalOrder(field) => write!(formatter, "admission field {field} is not canonically ordered"),
            Self::LengthOverflow => write!(formatter, "canonical admission length exceeds u64"),
            Self::Qualification(error) => error.fmt(formatter),
            Self::Analysis(error) => error.fmt(formatter),
            Self::Design(error) => error.fmt(formatter),
        }
    }
}

impl Error for AdmissionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Qualification(error) => Some(error),
            Self::Analysis(error) => Some(error),
            Self::Design(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_analysis_contracts::{
        AnalysisEvidenceId, AnalysisObservation, AnalysisObservationId, AnalysisProfileId,
        AnalysisProfileRef, AnalysisRequestId, AnalysisValue, NumericalDisposition,
        ObservableRequest, ObservationClass, RunDisposition, SolverIdentity,
    };
    use symtropy_analysis_qualification::{
        FacetAcceptance, FacetAssessment, FacetDisposition, FacetRule, QualificationCutId,
        QualificationProfileId,
    };
    use symtropy_design::{DesignId, DesignRevisionRef};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn digest(value: &str) -> ContentDigest {
        ContentDigest::new(id("blake3"), value).unwrap()
    }

    fn exact(authority: &str, subject: &str, value: &str) -> ExactSemanticRef {
        ExactSemanticRef::new(id(authority), id(subject), 1, digest(value)).unwrap()
    }

    fn request() -> AnalysisRequest {
        let design = DesignRevisionRef::new(
            DesignId::new(id("design:bracket")),
            1,
            digest("design"),
        )
        .unwrap();
        let profile = AnalysisProfileRef::new(
            AnalysisProfileId::new(id("analysis-profile:static")),
            1,
            digest("analysis-profile"),
        )
        .unwrap();
        AnalysisRequest::new(
            AnalysisRequestId::new(id("analysis-request:bracket")),
            design,
            profile,
            Vec::new(),
            vec![ObservableRequest::new(
                id("observable:deflection"),
                id("dimension:length"),
                Some(id("unit:um")),
            )
            .unwrap()],
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    fn evidence(request: &AnalysisRequest, binary: &str, converged: bool) -> AnalysisEvidence {
        let observations = if converged {
            vec![AnalysisObservation::new(
                AnalysisObservationId::new(id("observation:deflection")),
                ObservationClass::RequestedResult,
                id("observable:deflection"),
                id("dimension:length"),
                Some(id("unit:um")),
                AnalysisValue::Measurement {
                    lower: 390,
                    upper: 430,
                    resolution: 10,
                },
            )
            .unwrap()]
        } else {
            Vec::new()
        };
        AnalysisEvidence::new(
            AnalysisEvidenceId::new(id("analysis-evidence:bracket")),
            request,
            SolverIdentity::new(
                id("provider:solver-lab"),
                id("solver:fem"),
                "1.0.0",
                digest(binary),
            )
            .unwrap(),
            exact("authority:nix", "environment:solver", "environment"),
            exact("authority:model", "model:elastic", "model"),
            exact("authority:applicability", "domain:bracket", "applicability"),
            RunDisposition::Completed,
            if converged {
                NumericalDisposition::Converged
            } else {
                NumericalDisposition::NotConverged
            },
            Vec::new(),
            observations,
        )
        .unwrap()
    }

    fn profile() -> QualificationProfile {
        QualificationProfile::new(
            QualificationProfileId::new(id("qualification-profile:prototype")),
            1,
            vec![
                FacetRule::new(
                    id("facet:model"),
                    id("authority:model-qualification"),
                    FacetAcceptance::EstablishedOnly,
                )
                .unwrap(),
                FacetRule::new(
                    id("facet:numerical"),
                    id("authority:numerical-qualification"),
                    FacetAcceptance::EstablishedOnly,
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn assessment(
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        facet: &str,
        authority: &str,
        disposition: FacetDisposition,
    ) -> FacetAssessment {
        FacetAssessment::new(
            id(facet),
            evidence.exact_ref(request).unwrap(),
            exact(authority, &format!("attestation:{facet}"), facet),
            disposition,
            Vec::new(),
        )
        .unwrap()
    }

    fn cut(
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
        disposition: FacetDisposition,
    ) -> AnalysisQualificationCut {
        AnalysisQualificationCut::new(
            QualificationCutId::new(id("qualification-cut:bracket")),
            request,
            evidence,
            profile,
            exact("authority:clock", "evaluation-context:1", "context"),
            vec![
                assessment(
                    request,
                    evidence,
                    "facet:model",
                    "authority:model-qualification",
                    disposition,
                ),
                assessment(
                    request,
                    evidence,
                    "facet:numerical",
                    "authority:numerical-qualification",
                    disposition,
                ),
            ],
        )
        .unwrap()
    }

    fn admission_policy(profile: &QualificationProfile) -> QualificationAdmissionPolicy {
        QualificationAdmissionPolicy::new(
            AdmissionPolicyId::new(id("admission-policy:prototype")),
            1,
            profile,
            vec![
                FacetAdmissionRule::new(
                    id("facet:model"),
                    exact(
                        "authority:model-qualification",
                        "issuer-profile:model-v1",
                        "issuer-model",
                    ),
                    exact(
                        "authority:xenia",
                        "verifier-profile:model-v1",
                        "verifier-model",
                    ),
                )
                .unwrap(),
                FacetAdmissionRule::new(
                    id("facet:numerical"),
                    exact(
                        "authority:numerical-qualification",
                        "issuer-profile:numerical-v1",
                        "issuer-numerical",
                    ),
                    exact(
                        "authority:xenia",
                        "verifier-profile:numerical-v1",
                        "verifier-numerical",
                    ),
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn verification_records(
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
        cut: &AnalysisQualificationCut,
        policy: &QualificationAdmissionPolicy,
        status: VerificationStatus,
    ) -> Vec<FacetVerificationInput> {
        let cut_digest = cut.content_digest(request, evidence, profile).unwrap();
        let evidence_ref = evidence.exact_ref(request).unwrap();
        cut.assessments()
            .iter()
            .map(|assessment| {
                let rule = policy.rule(&assessment.facet_id).unwrap();
                FacetVerificationInput {
                    id: VerificationRecordId::new(
                        id(&format!("verification:{}", assessment.facet_id.as_str())),
                    ),
                    facet_id: assessment.facet_id.clone(),
                    qualification_cut_digest: cut_digest.clone(),
                    subject_evidence: evidence_ref.clone(),
                    assessment_attestation: assessment.attestation.clone(),
                    issuer_profile: rule.issuer_profile.clone(),
                    verifier_profile: rule.verifier_profile.clone(),
                    verification_receipt: exact(
                        "authority:xenia",
                        &format!("receipt:{}", assessment.facet_id.as_str()),
                        "verified",
                    ),
                    status,
                }
            })
            .collect()
    }

    #[test]
    fn raw_established_a1_cut_is_not_admitted_without_verification() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:empty")),
            1,
            &policy,
            Vec::new(),
        )
        .unwrap();
        let result = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            &registry,
        );
        assert!(matches!(result, Err(AdmissionError::VerificationRequired(_))));
    }

    #[test]
    fn every_exact_verified_facet_allows_admission() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let records = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        );
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:qualified")),
            1,
            &policy,
            records,
        )
        .unwrap();
        let admission = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            &registry,
        )
        .unwrap();
        admission
            .validate_current(&request, &evidence, &profile, &cut, &policy, &registry)
            .unwrap();
    }

    #[test]
    fn revoked_verification_cannot_admit_established_assertion() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let records = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Revoked,
        );
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:revoked")),
            1,
            &policy,
            records,
        )
        .unwrap();
        let result = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            &registry,
        );
        assert!(matches!(
            result,
            Err(AdmissionError::VerificationNotCurrent { .. })
        ));
    }

    #[test]
    fn failed_a1_qualification_cannot_be_admitted_even_with_verified_records() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Failed);
        let policy = admission_policy(&profile);
        let records = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        );
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:failed-cut")),
            1,
            &policy,
            records,
        )
        .unwrap();
        let result = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            &registry,
        );
        assert!(matches!(
            result,
            Err(AdmissionError::QualificationNotEstablished(
                QualificationOutcome::Failed
            ))
        ));
    }

    #[test]
    fn verifier_profile_drift_is_rejected_when_registry_is_sealed() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let mut records = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        );
        records[0].verifier_profile = exact(
            "authority:xenia",
            "verifier-profile:changed",
            "changed",
        );
        let result = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:bad-verifier")),
            1,
            &policy,
            records,
        );
        assert!(matches!(
            result,
            Err(AdmissionError::VerifierProfileMismatch(_))
        ));
    }

    #[test]
    fn verification_from_another_cut_cannot_satisfy_current_cut() {
        let request = request();
        let evidence = evidence(&request, "solver-a", true);
        let profile = profile();
        let cut = cut(&request, &evidence, &profile, FacetDisposition::Established);
        let policy = admission_policy(&profile);
        let mut records = verification_records(
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            VerificationStatus::Verified,
        );
        records[0].qualification_cut_digest = digest("different-cut");
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:other-cut")),
            1,
            &policy,
            records,
        )
        .unwrap();
        let result = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &evidence,
            &profile,
            &cut,
            &policy,
            &registry,
        );
        assert!(matches!(result, Err(AdmissionError::VerificationRequired(_))));
    }

    #[test]
    fn changing_solver_evidence_invalidates_old_admission() {
        let request = request();
        let old_evidence = evidence(&request, "solver-a", true);
        let new_evidence = evidence(&request, "solver-b", true);
        let profile = profile();
        let old_cut = cut(
            &request,
            &old_evidence,
            &profile,
            FacetDisposition::Established,
        );
        let policy = admission_policy(&profile);
        let records = verification_records(
            &request,
            &old_evidence,
            &profile,
            &old_cut,
            &policy,
            VerificationStatus::Verified,
        );
        let registry = VerificationRegistry::seal(
            VerificationRegistryId::new(id("verification-registry:old")),
            1,
            &policy,
            records,
        )
        .unwrap();
        let admission = admit_qualification(
            AdmissionReceiptId::new(id("admission:bracket")),
            &request,
            &old_evidence,
            &profile,
            &old_cut,
            &policy,
            &registry,
        )
        .unwrap();
        let new_cut = cut(
            &request,
            &new_evidence,
            &profile,
            FacetDisposition::Established,
        );
        let result = admission.validate_current(
            &request,
            &new_evidence,
            &profile,
            &new_cut,
            &policy,
            &registry,
        );
        assert!(result.is_err());
    }
}
