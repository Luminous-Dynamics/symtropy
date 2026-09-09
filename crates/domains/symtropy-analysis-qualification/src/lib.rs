// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Faceted qualification contracts for exact Symtropy analysis evidence.
//!
//! This crate evaluates whether one coherent exact analysis-evidence cut meets
//! one exact consumer qualification profile. It does not execute solvers,
//! authenticate signatures, emit Fabrication engineering facts, declare a
//! design safe, or confer certification/commissioning/payment authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};
use symtropy_analysis_contracts::{
    AnalysisEvidence, AnalysisEvidenceRef, AnalysisRequest, AnalysisRequestRef, ExactSemanticRef,
};
use symtropy_design::ContentDigest;
use symtropy_game_state::StableId;

pub const QUALIFICATION_PROFILE_SCHEMA_VERSION: u32 = 1;
pub const QUALIFICATION_CUT_SCHEMA_VERSION: u32 = 1;
const PROFILE_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.qualification-profile.v1\0";
const CUT_DIGEST_DOMAIN: &[u8] = b"symtropy.analysis.qualification-cut.v1\0";
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

stable_id_type!(QualificationProfileId);
stable_id_type!(QualificationCutId);

/// What a consumer accepts for one independently named qualification facet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacetAcceptance {
    /// The named qualification authority must explicitly establish the facet.
    EstablishedOnly,
    /// The facet may either be established or explicitly declared inapplicable.
    EstablishedOrNotApplicable,
}

/// Disposition asserted by the exact named qualification attestation.
///
/// This is not a global truth value. A profile determines which authority is
/// allowed to speak for each facet, and the overall decision is derived from
/// the complete coherent cut rather than caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacetDisposition {
    Established,
    Failed,
    Indeterminate,
    NotApplicable,
}

/// One required, independent qualification dimension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetRule {
    pub facet_id: StableId,
    pub required_authority_id: StableId,
    pub acceptance: FacetAcceptance,
}

impl FacetRule {
    pub fn new(
        facet_id: StableId,
        required_authority_id: StableId,
        acceptance: FacetAcceptance,
    ) -> Result<Self, QualificationError> {
        let rule = Self {
            facet_id,
            required_authority_id,
            acceptance,
        };
        rule.validate()?;
        Ok(rule)
    }

    fn validate(&self) -> Result<(), QualificationError> {
        validate_stable_id(&self.facet_id)?;
        validate_stable_id(&self.required_authority_id)
    }
}

/// Exact immutable consumer profile defining the complete set of facets needed
/// for one kind of downstream decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationProfile {
    schema_version: u32,
    id: QualificationProfileId,
    revision: u64,
    rules: Vec<FacetRule>,
}

impl QualificationProfile {
    pub fn new(
        id: QualificationProfileId,
        revision: u64,
        mut rules: Vec<FacetRule>,
    ) -> Result<Self, QualificationError> {
        rules.sort_by(|left, right| left.facet_id.cmp(&right.facet_id));
        let profile = Self {
            schema_version: QUALIFICATION_PROFILE_SCHEMA_VERSION,
            id,
            revision,
            rules,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn id(&self) -> &QualificationProfileId {
        &self.id
    }

    pub fn rules(&self) -> &[FacetRule] {
        &self.rules
    }

    pub fn exact_ref(&self) -> Result<QualificationProfileRef, QualificationError> {
        Ok(QualificationProfileRef {
            id: self.id.clone(),
            revision: self.revision,
            content_digest: self.content_digest()?,
        })
    }

    pub fn content_digest(&self) -> Result<ContentDigest, QualificationError> {
        self.validate()?;
        sha256_digest(&self.canonical_preimage()?)
    }

    pub fn validate(&self) -> Result<(), QualificationError> {
        if self.schema_version != QUALIFICATION_PROFILE_SCHEMA_VERSION {
            return Err(QualificationError::UnsupportedProfileSchema(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        if self.rules.is_empty() {
            return Err(QualificationError::FacetRulesRequired);
        }
        let mut previous: Option<&StableId> = None;
        for rule in &self.rules {
            rule.validate()?;
            if let Some(previous) = previous {
                if previous >= &rule.facet_id {
                    return if previous == &rule.facet_id {
                        Err(QualificationError::DuplicateFacet(rule.facet_id.clone()))
                    } else {
                        Err(QualificationError::NonCanonicalOrder("profile.rules"))
                    };
                }
            }
            previous = Some(&rule.facet_id);
        }
        Ok(())
    }

    fn rule(&self, facet_id: &StableId) -> Option<&FacetRule> {
        self.rules
            .binary_search_by(|rule| rule.facet_id.cmp(facet_id))
            .ok()
            .map(|index| &self.rules[index])
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, QualificationError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PROFILE_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        bytes.extend_from_slice(&self.revision.to_le_bytes());
        encode_len(&mut bytes, self.rules.len())?;
        for rule in &self.rules {
            encode_stable_id(&mut bytes, &rule.facet_id);
            encode_stable_id(&mut bytes, &rule.required_authority_id);
            bytes.push(match rule.acceptance {
                FacetAcceptance::EstablishedOnly => 0,
                FacetAcceptance::EstablishedOrNotApplicable => 1,
            });
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct QualificationProfileRef {
    pub id: QualificationProfileId,
    pub revision: u64,
    pub content_digest: ContentDigest,
}

impl QualificationProfileRef {
    pub fn validate(&self) -> Result<(), QualificationError> {
        validate_stable_id(self.id.stable_id())?;
        self.content_digest
            .validate()
            .map_err(QualificationError::Design)
    }
}

/// Exact assessment for one facet of one exact A0 evidence record.
///
/// The attestation is opaque here: transport/signature/authentication remains
/// an external authority concern. The qualification profile pins which
/// `authority_id` is acceptable for this facet, and this record pins the exact
/// A0 evidence subject so an assessment cannot be silently reused elsewhere.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetAssessment {
    pub facet_id: StableId,
    pub subject_evidence: AnalysisEvidenceRef,
    pub attestation: ExactSemanticRef,
    pub disposition: FacetDisposition,
    supporting_evidence: Vec<ExactSemanticRef>,
}

impl FacetAssessment {
    pub fn new(
        facet_id: StableId,
        subject_evidence: AnalysisEvidenceRef,
        attestation: ExactSemanticRef,
        disposition: FacetDisposition,
        mut supporting_evidence: Vec<ExactSemanticRef>,
    ) -> Result<Self, QualificationError> {
        supporting_evidence.sort();
        let assessment = Self {
            facet_id,
            subject_evidence,
            attestation,
            disposition,
            supporting_evidence,
        };
        assessment.validate()?;
        Ok(assessment)
    }

    pub fn supporting_evidence(&self) -> &[ExactSemanticRef] {
        &self.supporting_evidence
    }

    fn validate(&self) -> Result<(), QualificationError> {
        validate_stable_id(&self.facet_id)?;
        validate_analysis_evidence_ref(&self.subject_evidence)?;
        self.attestation.validate().map_err(QualificationError::Analysis)?;
        validate_exact_refs("assessment.supporting_evidence", &self.supporting_evidence)
    }
}

/// Overall result derived from an exact A0 execution record, exact profile and
/// exact set of facet assessments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualificationOutcome {
    /// A0 itself says the execution cannot enter requested-result qualification.
    StructurallyIneligible,
    /// At least one required facet explicitly failed the profile.
    Failed,
    /// No required facet failed, but one or more are absent/indeterminate or
    /// otherwise not accepted by the profile.
    Indeterminate,
    /// Every required facet is present, from the required authority, and its
    /// disposition is accepted by the profile.
    Established,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FacetDecision {
    pub facet_id: StableId,
    pub disposition: Option<FacetDisposition>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QualificationDecision {
    pub overall: QualificationOutcome,
    pub facets: Vec<FacetDecision>,
}

/// One coherent immutable evidence cut for a consequential analysis
/// qualification decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisQualificationCut {
    schema_version: u32,
    id: QualificationCutId,
    request: AnalysisRequestRef,
    evidence: AnalysisEvidenceRef,
    profile: QualificationProfileRef,
    evaluation_context: ExactSemanticRef,
    assessments: Vec<FacetAssessment>,
}

impl AnalysisQualificationCut {
    pub fn new(
        id: QualificationCutId,
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
        evaluation_context: ExactSemanticRef,
        mut assessments: Vec<FacetAssessment>,
    ) -> Result<Self, QualificationError> {
        assessments.sort_by(|left, right| left.facet_id.cmp(&right.facet_id));
        let cut = Self {
            schema_version: QUALIFICATION_CUT_SCHEMA_VERSION,
            id,
            request: request.exact_ref().map_err(QualificationError::Analysis)?,
            evidence: evidence
                .exact_ref(request)
                .map_err(QualificationError::Analysis)?,
            profile: profile.exact_ref()?,
            evaluation_context,
            assessments,
        };
        cut.validate_against(request, evidence, profile)?;
        Ok(cut)
    }

    pub fn evidence(&self) -> &AnalysisEvidenceRef {
        &self.evidence
    }

    pub fn profile(&self) -> &QualificationProfileRef {
        &self.profile
    }

    pub fn assessments(&self) -> &[FacetAssessment] {
        &self.assessments
    }

    pub fn decision(
        &self,
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
    ) -> Result<QualificationDecision, QualificationError> {
        self.validate_against(request, evidence, profile)?;

        let mut facets = Vec::with_capacity(profile.rules().len());
        let mut failed = false;
        let mut indeterminate = false;

        for rule in profile.rules() {
            let assessment = self
                .assessments
                .binary_search_by(|assessment| assessment.facet_id.cmp(&rule.facet_id))
                .ok()
                .map(|index| &self.assessments[index]);

            let (disposition, accepted) = match assessment {
                None => {
                    indeterminate = true;
                    (None, false)
                }
                Some(assessment) => {
                    let accepted = match (rule.acceptance, assessment.disposition) {
                        (_, FacetDisposition::Failed) => {
                            failed = true;
                            false
                        }
                        (_, FacetDisposition::Indeterminate) => {
                            indeterminate = true;
                            false
                        }
                        (FacetAcceptance::EstablishedOnly, FacetDisposition::Established)
                        | (
                            FacetAcceptance::EstablishedOrNotApplicable,
                            FacetDisposition::Established,
                        )
                        | (
                            FacetAcceptance::EstablishedOrNotApplicable,
                            FacetDisposition::NotApplicable,
                        ) => true,
                        (FacetAcceptance::EstablishedOnly, FacetDisposition::NotApplicable) => {
                            failed = true;
                            false
                        }
                    };
                    (Some(assessment.disposition), accepted)
                }
            };

            facets.push(FacetDecision {
                facet_id: rule.facet_id.clone(),
                disposition,
                accepted,
            });
        }

        let overall = if !evidence.may_enter_result_qualification() {
            QualificationOutcome::StructurallyIneligible
        } else if failed {
            QualificationOutcome::Failed
        } else if indeterminate {
            QualificationOutcome::Indeterminate
        } else {
            QualificationOutcome::Established
        };

        Ok(QualificationDecision { overall, facets })
    }

    pub fn content_digest(
        &self,
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
    ) -> Result<ContentDigest, QualificationError> {
        self.validate_against(request, evidence, profile)?;
        sha256_digest(&self.canonical_preimage()?)
    }

    pub fn validate_against(
        &self,
        request: &AnalysisRequest,
        evidence: &AnalysisEvidence,
        profile: &QualificationProfile,
    ) -> Result<(), QualificationError> {
        if self.schema_version != QUALIFICATION_CUT_SCHEMA_VERSION {
            return Err(QualificationError::UnsupportedCutSchema(
                self.schema_version,
            ));
        }
        validate_stable_id(self.id.stable_id())?;
        self.evaluation_context
            .validate()
            .map_err(QualificationError::Analysis)?;
        profile.validate()?;

        let expected_request = request.exact_ref().map_err(QualificationError::Analysis)?;
        if self.request != expected_request {
            return Err(QualificationError::RequestMismatch);
        }
        let expected_evidence = evidence
            .exact_ref(request)
            .map_err(QualificationError::Analysis)?;
        if self.evidence != expected_evidence {
            return Err(QualificationError::EvidenceMismatch);
        }
        if self.profile != profile.exact_ref()? {
            return Err(QualificationError::ProfileMismatch);
        }

        let mut previous: Option<&StableId> = None;
        for assessment in &self.assessments {
            assessment.validate()?;
            if assessment.subject_evidence != self.evidence {
                return Err(QualificationError::AssessmentSubjectMismatch(
                    assessment.facet_id.clone(),
                ));
            }
            let Some(rule) = profile.rule(&assessment.facet_id) else {
                return Err(QualificationError::UnknownFacet(
                    assessment.facet_id.clone(),
                ));
            };
            if assessment.attestation.authority_id != rule.required_authority_id {
                return Err(QualificationError::WrongFacetAuthority {
                    facet_id: assessment.facet_id.clone(),
                    expected: rule.required_authority_id.clone(),
                    actual: assessment.attestation.authority_id.clone(),
                });
            }
            if let Some(previous) = previous {
                if previous >= &assessment.facet_id {
                    return if previous == &assessment.facet_id {
                        Err(QualificationError::DuplicateAssessment(
                            assessment.facet_id.clone(),
                        ))
                    } else {
                        Err(QualificationError::NonCanonicalOrder(
                            "cut.assessments",
                        ))
                    };
                }
            }
            previous = Some(&assessment.facet_id);
        }
        Ok(())
    }

    fn canonical_preimage(&self) -> Result<Vec<u8>, QualificationError> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(CUT_DIGEST_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        encode_stable_id(&mut bytes, self.id.stable_id());
        encode_analysis_request_ref(&mut bytes, &self.request)?;
        encode_analysis_evidence_ref(&mut bytes, &self.evidence)?;
        encode_stable_id(&mut bytes, self.profile.id.stable_id());
        bytes.extend_from_slice(&self.profile.revision.to_le_bytes());
        encode_digest(&mut bytes, &self.profile.content_digest)?;
        encode_exact_ref(&mut bytes, &self.evaluation_context)?;
        encode_len(&mut bytes, self.assessments.len())?;
        for assessment in &self.assessments {
            encode_stable_id(&mut bytes, &assessment.facet_id);
            encode_analysis_evidence_ref(&mut bytes, &assessment.subject_evidence)?;
            encode_exact_ref(&mut bytes, &assessment.attestation)?;
            bytes.push(match assessment.disposition {
                FacetDisposition::Established => 0,
                FacetDisposition::Failed => 1,
                FacetDisposition::Indeterminate => 2,
                FacetDisposition::NotApplicable => 3,
            });
            encode_len(&mut bytes, assessment.supporting_evidence.len())?;
            for supporting in &assessment.supporting_evidence {
                encode_exact_ref(&mut bytes, supporting)?;
            }
        }
        Ok(bytes)
    }
}

fn validate_analysis_evidence_ref(
    reference: &AnalysisEvidenceRef,
) -> Result<(), QualificationError> {
    validate_stable_id(reference.id.stable_id())?;
    reference
        .content_digest
        .validate()
        .map_err(QualificationError::Design)
}

fn validate_exact_refs(
    field: &'static str,
    refs: &[ExactSemanticRef],
) -> Result<(), QualificationError> {
    let mut previous: Option<&ExactSemanticRef> = None;
    for reference in refs {
        reference.validate().map_err(QualificationError::Analysis)?;
        if let Some(previous) = previous {
            if previous >= reference {
                return if previous == reference {
                    Err(QualificationError::DuplicateSupportingEvidence(
                        reference.clone(),
                    ))
                } else {
                    Err(QualificationError::NonCanonicalOrder(field))
                };
            }
        }
        previous = Some(reference);
    }
    Ok(())
}

fn encode_analysis_request_ref(
    bytes: &mut Vec<u8>,
    reference: &AnalysisRequestRef,
) -> Result<(), QualificationError> {
    validate_stable_id(reference.id.stable_id())?;
    encode_stable_id(bytes, reference.id.stable_id());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_analysis_evidence_ref(
    bytes: &mut Vec<u8>,
    reference: &AnalysisEvidenceRef,
) -> Result<(), QualificationError> {
    validate_analysis_evidence_ref(reference)?;
    encode_stable_id(bytes, reference.id.stable_id());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_exact_ref(
    bytes: &mut Vec<u8>,
    reference: &ExactSemanticRef,
) -> Result<(), QualificationError> {
    reference.validate().map_err(QualificationError::Analysis)?;
    encode_stable_id(bytes, &reference.authority_id);
    encode_stable_id(bytes, &reference.subject_id);
    bytes.extend_from_slice(&reference.revision.to_le_bytes());
    encode_digest(bytes, &reference.content_digest)
}

fn encode_digest(
    bytes: &mut Vec<u8>,
    digest: &ContentDigest,
) -> Result<(), QualificationError> {
    digest.validate().map_err(QualificationError::Design)?;
    encode_stable_id(bytes, &digest.algorithm);
    encode_string(bytes, &digest.value)
}

fn sha256_digest(bytes: &[u8]) -> Result<ContentDigest, QualificationError> {
    ContentDigest::new(
        StableId::parse(SHA256_ALGORITHM_ID)
            .expect("sha256 is a valid stable identifier literal"),
        hex(&Sha256::digest(bytes)),
    )
    .map_err(QualificationError::Design)
}

fn encode_stable_id(bytes: &mut Vec<u8>, id: &StableId) {
    encode_string(bytes, id.as_str()).expect("StableId length is bounded well below u64");
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), QualificationError> {
    encode_len(bytes, value.len())?;
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_len(bytes: &mut Vec<u8>, len: usize) -> Result<(), QualificationError> {
    let len = u64::try_from(len).map_err(|_| QualificationError::LengthOverflow)?;
    bytes.extend_from_slice(&len.to_le_bytes());
    Ok(())
}

fn validate_stable_id(id: &StableId) -> Result<(), QualificationError> {
    StableId::parse(id.as_str())
        .map(|_| ())
        .map_err(|_| QualificationError::InvalidStableId(id.as_str().to_string()))
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
pub enum QualificationError {
    InvalidStableId(String),
    UnsupportedProfileSchema(u32),
    UnsupportedCutSchema(u32),
    FacetRulesRequired,
    DuplicateFacet(StableId),
    DuplicateAssessment(StableId),
    DuplicateSupportingEvidence(ExactSemanticRef),
    UnknownFacet(StableId),
    WrongFacetAuthority {
        facet_id: StableId,
        expected: StableId,
        actual: StableId,
    },
    AssessmentSubjectMismatch(StableId),
    RequestMismatch,
    EvidenceMismatch,
    ProfileMismatch,
    NonCanonicalOrder(&'static str),
    LengthOverflow,
    Analysis(symtropy_analysis_contracts::AnalysisError),
    Design(symtropy_design::DesignError),
}

impl fmt::Display for QualificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStableId(id) => write!(formatter, "invalid stable identifier: {id}"),
            Self::UnsupportedProfileSchema(version) => {
                write!(formatter, "unsupported qualification profile schema {version}")
            }
            Self::UnsupportedCutSchema(version) => {
                write!(formatter, "unsupported qualification cut schema {version}")
            }
            Self::FacetRulesRequired => write!(formatter, "qualification profile requires at least one facet rule"),
            Self::DuplicateFacet(id) => write!(formatter, "duplicate qualification facet {id}"),
            Self::DuplicateAssessment(id) => write!(formatter, "duplicate assessment for qualification facet {id}"),
            Self::DuplicateSupportingEvidence(reference) => {
                write!(formatter, "duplicate supporting qualification evidence {reference:?}")
            }
            Self::UnknownFacet(id) => write!(formatter, "assessment references unknown qualification facet {id}"),
            Self::WrongFacetAuthority { facet_id, expected, actual } => write!(
                formatter,
                "qualification facet {facet_id} requires authority {expected}, got {actual}"
            ),
            Self::AssessmentSubjectMismatch(id) => write!(
                formatter,
                "qualification facet {id} does not bind the exact analysis evidence in this cut"
            ),
            Self::RequestMismatch => write!(formatter, "qualification cut does not bind the supplied exact analysis request"),
            Self::EvidenceMismatch => write!(formatter, "qualification cut does not bind the supplied exact analysis evidence"),
            Self::ProfileMismatch => write!(formatter, "qualification cut does not bind the supplied exact qualification profile"),
            Self::NonCanonicalOrder(field) => write!(formatter, "qualification field {field} is not canonically ordered"),
            Self::LengthOverflow => write!(formatter, "canonical qualification length exceeds u64"),
            Self::Analysis(error) => error.fmt(formatter),
            Self::Design(error) => error.fmt(formatter),
        }
    }
}

impl Error for QualificationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
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
    use symtropy_design::{DesignId, DesignRevisionRef};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn digest(value: &str) -> ContentDigest {
        ContentDigest::new(id("blake3"), value).unwrap()
    }

    fn exact(authority: &str, subject: &str, content: &str) -> ExactSemanticRef {
        ExactSemanticRef::new(id(authority), id(subject), 1, digest(content)).unwrap()
    }

    fn request() -> AnalysisRequest {
        let subject = DesignRevisionRef::new(
            DesignId::new(id("design:bracket")),
            1,
            digest("design-a"),
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
            subject,
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

    fn evidence(request: &AnalysisRequest, converged: bool, binary: &str) -> AnalysisEvidence {
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
            vec![AnalysisObservation::new(
                AnalysisObservationId::new(id("observation:residual")),
                ObservationClass::Diagnostic,
                id("diagnostic:residual"),
                id("dimension:dimensionless"),
                None,
                AnalysisValue::Measurement {
                    lower: 1,
                    upper: 1,
                    resolution: 1,
                },
            )
            .unwrap()]
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
                    id("facet:model-applicability"),
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
        assessments: Vec<FacetAssessment>,
    ) -> Result<AnalysisQualificationCut, QualificationError> {
        AnalysisQualificationCut::new(
            QualificationCutId::new(id("qualification-cut:bracket")),
            request,
            evidence,
            profile,
            exact("authority:clock", "evaluation-context:1", "context"),
            assessments,
        )
    }

    #[test]
    fn complete_accepted_facets_establish_qualification() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let cut = cut(
            &request,
            &evidence,
            &profile,
            vec![
                assessment(
                    &request,
                    &evidence,
                    "facet:numerical",
                    "authority:numerical-qualification",
                    FacetDisposition::Established,
                ),
                assessment(
                    &request,
                    &evidence,
                    "facet:model-applicability",
                    "authority:model-qualification",
                    FacetDisposition::Established,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            cut.decision(&request, &evidence, &profile).unwrap().overall,
            QualificationOutcome::Established
        );
    }

    #[test]
    fn missing_facet_is_indeterminate_not_established() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let cut = cut(
            &request,
            &evidence,
            &profile,
            vec![assessment(
                &request,
                &evidence,
                "facet:numerical",
                "authority:numerical-qualification",
                FacetDisposition::Established,
            )],
        )
        .unwrap();
        assert_eq!(
            cut.decision(&request, &evidence, &profile).unwrap().overall,
            QualificationOutcome::Indeterminate
        );
    }

    #[test]
    fn explicit_failed_facet_fails_overall_qualification() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let cut = cut(
            &request,
            &evidence,
            &profile,
            vec![
                assessment(
                    &request,
                    &evidence,
                    "facet:numerical",
                    "authority:numerical-qualification",
                    FacetDisposition::Failed,
                ),
                assessment(
                    &request,
                    &evidence,
                    "facet:model-applicability",
                    "authority:model-qualification",
                    FacetDisposition::Established,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            cut.decision(&request, &evidence, &profile).unwrap().overall,
            QualificationOutcome::Failed
        );
    }

    #[test]
    fn structurally_ineligible_a0_evidence_cannot_be_established() {
        let request = request();
        let evidence = evidence(&request, false, "solver-a");
        let profile = profile();
        let cut = cut(
            &request,
            &evidence,
            &profile,
            vec![
                assessment(
                    &request,
                    &evidence,
                    "facet:numerical",
                    "authority:numerical-qualification",
                    FacetDisposition::Established,
                ),
                assessment(
                    &request,
                    &evidence,
                    "facet:model-applicability",
                    "authority:model-qualification",
                    FacetDisposition::Established,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            cut.decision(&request, &evidence, &profile).unwrap().overall,
            QualificationOutcome::StructurallyIneligible
        );
    }

    #[test]
    fn wrong_authority_cannot_satisfy_named_facet() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let result = cut(
            &request,
            &evidence,
            &profile,
            vec![assessment(
                &request,
                &evidence,
                "facet:numerical",
                "authority:unrelated",
                FacetDisposition::Established,
            )],
        );
        assert!(matches!(
            result,
            Err(QualificationError::WrongFacetAuthority { .. })
        ));
    }

    #[test]
    fn established_only_rule_rejects_not_applicable() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let cut = cut(
            &request,
            &evidence,
            &profile,
            vec![
                assessment(
                    &request,
                    &evidence,
                    "facet:numerical",
                    "authority:numerical-qualification",
                    FacetDisposition::NotApplicable,
                ),
                assessment(
                    &request,
                    &evidence,
                    "facet:model-applicability",
                    "authority:model-qualification",
                    FacetDisposition::Established,
                ),
            ],
        )
        .unwrap();
        assert_eq!(
            cut.decision(&request, &evidence, &profile).unwrap().overall,
            QualificationOutcome::Failed
        );
    }

    #[test]
    fn exact_solver_evidence_substitution_changes_cut_identity() {
        let request = request();
        let left_evidence = evidence(&request, true, "solver-a");
        let right_evidence = evidence(&request, true, "solver-b");
        let profile = profile();

        let left = cut(&request, &left_evidence, &profile, Vec::new()).unwrap();
        let right = cut(&request, &right_evidence, &profile, Vec::new()).unwrap();

        assert_ne!(
            left.content_digest(&request, &left_evidence, &profile)
                .unwrap(),
            right
                .content_digest(&request, &right_evidence, &profile)
                .unwrap()
        );
    }

    #[test]
    fn caller_insertion_order_does_not_change_cut_identity() {
        let request = request();
        let evidence = evidence(&request, true, "solver-a");
        let profile = profile();
        let numerical = assessment(
            &request,
            &evidence,
            "facet:numerical",
            "authority:numerical-qualification",
            FacetDisposition::Established,
        );
        let applicability = assessment(
            &request,
            &evidence,
            "facet:model-applicability",
            "authority:model-qualification",
            FacetDisposition::Established,
        );
        let left = cut(
            &request,
            &evidence,
            &profile,
            vec![numerical.clone(), applicability.clone()],
        )
        .unwrap();
        let right = cut(
            &request,
            &evidence,
            &profile,
            vec![applicability, numerical],
        )
        .unwrap();
        assert_eq!(left, right);
        assert_eq!(
            left.content_digest(&request, &evidence, &profile).unwrap(),
            right.content_digest(&request, &evidence, &profile).unwrap()
        );
    }
}
