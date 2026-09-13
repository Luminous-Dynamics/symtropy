use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId, BiologicalSpeciesModel,
    BiologicalSpeciesModelDigest, EvolutionError, SpeciesModelContentDigest, SpeciesModelError,
    SpeciesModelValidityDomainDigest, ValidatedBiologicalSpeciesModel,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const SPECIES_CONCEPT_AUTHORITY_VERSION: u32 = 1;
pub const STRICT_BIOLOGICAL_SPECIES_CONCEPT_FAMILY_VERSION: u32 = 1;
const AUTHORITY_DOMAIN: &[u8] = b"symtropy:evolution:species-concept-authority:v1\0";
const CONTENT_DOMAIN: &[u8] = b"symtropy:evolution:species-concept-content:v1\0";
const VALIDITY_DOMAIN: &[u8] = b"symtropy:evolution:species-concept-validity-domain:v1\0";
const ADAPTER_RULE_DOMAIN: &[u8] = b"symtropy:evolution:species-concept-adapter-rule:v1\0";
const STRICT_BSC_ADAPTER_SPEC: &[u8] = b"strict biological species concept adapter v1: preserve exact BiologicalSpeciesModel V1 semantic content and validity/qualification identity; current species classification and historical transition interval are separate capabilities; require explicit qualified model applicability; require meaningful reproductive-isolation evidence; qualifier or validity-domain drift does not create a new concept family; source qualification is provenance and does not imply direct endorsement of this adapter; no outcome fields; no universal taxonomy claim";

macro_rules! local_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
                let value = value.into();
                validate_text($field, &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
            }
        }
    };
}

local_id!(SpeciesConceptFamilyId, "SpeciesConceptFamilyId");
local_id!(SpeciesConceptModelId, "SpeciesConceptModelId");

fn strict_biological_family_id() -> SpeciesConceptFamilyId {
    SpeciesConceptFamilyId::new("strict-biological-species").expect("static family ID is valid")
}

pub fn strict_biological_species_concept_adapter_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(ADAPTER_RULE_DOMAIN);
    put_u64(&mut digest, STRICT_BSC_ADAPTER_SPEC.len() as u64);
    digest.update(STRICT_BSC_ADAPTER_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("strict-biological-species-concept-adapter-v1")
            .expect("static adapter method ID is valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesConceptCapability {
    CurrentSpeciesStatus,
    HistoricalTransitionInterval,
}

impl SpeciesConceptCapability {
    fn tag(self) -> u8 {
        match self {
            Self::CurrentSpeciesStatus => 0,
            Self::HistoricalTransitionInterval => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesConceptEvidenceRequirement {
    ExplicitQualifiedModelApplicability,
    CurrentCompleteReproductiveBarrier,
    PersistentOrRecontactLineageHistory,
    HistoricalTransitionTemporalEvidence,
}

impl SpeciesConceptEvidenceRequirement {
    fn tag(self) -> u8 {
        match self {
            Self::ExplicitQualifiedModelApplicability => 0,
            Self::CurrentCompleteReproductiveBarrier => 1,
            Self::PersistentOrRecontactLineageHistory => 2,
            Self::HistoricalTransitionTemporalEvidence => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SpeciesConceptDomainConstraint {
    ReproductiveIsolationMustBeBiologicallyMeaningful,
}

impl SpeciesConceptDomainConstraint {
    fn tag(self) -> u8 {
        match self {
            Self::ReproductiveIsolationMustBeBiologicallyMeaningful => 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptContentDigest([u8; 32]);

impl SpeciesConceptContentDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciesConceptContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesConceptContentDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciesConceptContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptValidityDomainDigest([u8; 32]);

impl SpeciesConceptValidityDomainDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciesConceptValidityDomainDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesConceptValidityDomainDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciesConceptValidityDomainDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptAuthorityDigest([u8; 32]);

impl SpeciesConceptAuthorityDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciesConceptAuthorityDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesConceptAuthorityDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciesConceptAuthorityDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SpeciesConceptIdentity {
    pub family_id: SpeciesConceptFamilyId,
    pub family_version: u32,
    pub content_digest: SpeciesConceptContentDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesConceptSourceBinding {
    StrictBiologicalSpeciesV1 {
        model: BiologicalSpeciesModel,
        model_digest: BiologicalSpeciesModelDigest,
    },
}

impl SpeciesConceptSourceBinding {
    fn tag(&self) -> u8 {
        match self {
            Self::StrictBiologicalSpeciesV1 { .. } => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptQualificationRef {
    /// Qualification provenance inherited from the concrete source model.
    /// This does not assert that the source qualifier directly endorsed the generic adapter.
    pub authority: AnalysisAuthorityRef,
    pub source_model_digest: BiologicalSpeciesModelDigest,
    pub concept_content_digest: SpeciesConceptContentDigest,
    pub validity_domain_digest: SpeciesConceptValidityDomainDigest,
}

impl SpeciesConceptQualificationRef {
    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.authority);
        digest.update(self.source_model_digest.as_bytes());
        digest.update(self.concept_content_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesConceptAuthority {
    authority_version: u32,
    pub family_id: SpeciesConceptFamilyId,
    pub family_version: u32,
    pub model_id: SpeciesConceptModelId,
    pub concept_content_digest: SpeciesConceptContentDigest,
    pub validity_domain_digest: SpeciesConceptValidityDomainDigest,
    pub capabilities: Vec<SpeciesConceptCapability>,
    pub evidence_requirements: Vec<SpeciesConceptEvidenceRequirement>,
    pub domain_constraints: Vec<SpeciesConceptDomainConstraint>,
    pub source: SpeciesConceptSourceBinding,
    pub qualification: SpeciesConceptQualificationRef,
    pub adapter_rule_authority: AnalysisAuthorityRef,
}

impl SpeciesConceptAuthority {
    pub fn from_strict_biological(
        model: &ValidatedBiologicalSpeciesModel<'_>,
    ) -> Result<Self, SpeciesConceptError> {
        let source = model.model();
        let family_id = strict_biological_family_id();
        let family_version = STRICT_BIOLOGICAL_SPECIES_CONCEPT_FAMILY_VERSION;
        let capabilities = strict_biological_capabilities();
        let evidence_requirements = strict_biological_requirements();
        let domain_constraints = strict_biological_domain_constraints();
        let concept_content_digest = strict_biological_concept_content_digest(
            source.model_content_digest,
            &capabilities,
            &evidence_requirements,
            &domain_constraints,
        );
        let validity_domain_digest = strict_biological_concept_validity_domain_digest(
            source.validity_domain.canonical_digest(),
        );
        let authority = Self {
            authority_version: SPECIES_CONCEPT_AUTHORITY_VERSION,
            family_id,
            family_version,
            model_id: SpeciesConceptModelId::new(source.model_id.as_str())?,
            concept_content_digest,
            validity_domain_digest,
            capabilities,
            evidence_requirements,
            domain_constraints,
            source: SpeciesConceptSourceBinding::StrictBiologicalSpeciesV1 {
                model: source.clone(),
                model_digest: model.model_digest(),
            },
            qualification: SpeciesConceptQualificationRef {
                authority: source.qualification.authority.clone(),
                source_model_digest: model.model_digest(),
                concept_content_digest,
                validity_domain_digest,
            },
            adapter_rule_authority: strict_biological_species_concept_adapter_rule_v1(),
        };
        authority.validate_local()?;
        Ok(authority)
    }

    pub fn conceptual_identity(&self) -> Result<SpeciesConceptIdentity, SpeciesConceptError> {
        self.validate_local()?;
        Ok(SpeciesConceptIdentity {
            family_id: self.family_id.clone(),
            family_version: self.family_version,
            content_digest: self.concept_content_digest,
        })
    }

    /// Convenience inspection only. Current authority is carried by
    /// `ValidatedSpeciesConceptAuthority`, not by this predicate on a restored value.
    pub fn supports(&self, capability: SpeciesConceptCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<SpeciesConceptAuthorityDigest, SpeciesConceptError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(AUTHORITY_DOMAIN);
        put_u32(&mut digest, self.authority_version);
        put_text(&mut digest, self.family_id.as_str());
        put_u32(&mut digest, self.family_version);
        put_text(&mut digest, self.model_id.as_str());
        digest.update(self.concept_content_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
        put_u64(&mut digest, self.capabilities.len() as u64);
        for capability in &self.capabilities {
            digest.update([capability.tag()]);
        }
        put_u64(&mut digest, self.evidence_requirements.len() as u64);
        for requirement in &self.evidence_requirements {
            digest.update([requirement.tag()]);
        }
        put_u64(&mut digest, self.domain_constraints.len() as u64);
        for constraint in &self.domain_constraints {
            digest.update([constraint.tag()]);
        }
        digest.update([self.source.tag()]);
        match &self.source {
            SpeciesConceptSourceBinding::StrictBiologicalSpeciesV1 {
                model,
                model_digest,
            } => {
                digest.update(model_digest.as_bytes());
                digest.update(model.canonical_digest()?.as_bytes());
            }
        }
        self.qualification.put(&mut digest);
        put_authority(&mut digest, &self.adapter_rule_authority);
        Ok(SpeciesConceptAuthorityDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SpeciesConceptError> {
        if self.authority_version != SPECIES_CONCEPT_AUTHORITY_VERSION {
            return Err(SpeciesConceptError::UnsupportedVersion(self.authority_version));
        }
        match &self.source {
            SpeciesConceptSourceBinding::StrictBiologicalSpeciesV1 {
                model,
                model_digest,
            } => {
                if model.canonical_digest()? != *model_digest {
                    return Err(SpeciesConceptError::SourceModelDigestMismatch);
                }
                if self.family_id != strict_biological_family_id()
                    || self.family_version != STRICT_BIOLOGICAL_SPECIES_CONCEPT_FAMILY_VERSION
                {
                    return Err(SpeciesConceptError::FamilyMismatch);
                }
                if self.model_id.as_str() != model.model_id.as_str() {
                    return Err(SpeciesConceptError::ModelIdMismatch);
                }

                let expected_capabilities = strict_biological_capabilities();
                if self.capabilities != expected_capabilities {
                    return Err(SpeciesConceptError::CapabilitySurfaceMismatch);
                }
                let expected_requirements = strict_biological_requirements();
                if self.evidence_requirements != expected_requirements {
                    return Err(SpeciesConceptError::EvidenceRequirementMismatch);
                }
                let expected_constraints = strict_biological_domain_constraints();
                if self.domain_constraints != expected_constraints {
                    return Err(SpeciesConceptError::DomainConstraintMismatch);
                }

                let expected_content = strict_biological_concept_content_digest(
                    model.model_content_digest,
                    &expected_capabilities,
                    &expected_requirements,
                    &expected_constraints,
                );
                if self.concept_content_digest != expected_content {
                    return Err(SpeciesConceptError::ConceptContentMismatch);
                }

                let expected_domain = strict_biological_concept_validity_domain_digest(
                    model.validity_domain.canonical_digest(),
                );
                if self.validity_domain_digest != expected_domain {
                    return Err(SpeciesConceptError::ValidityDomainMismatch);
                }

                if self.qualification.authority != model.qualification.authority
                    || self.qualification.source_model_digest != *model_digest
                    || self.qualification.concept_content_digest != expected_content
                    || self.qualification.validity_domain_digest != expected_domain
                {
                    return Err(SpeciesConceptError::QualificationBindingMismatch);
                }
                if self.adapter_rule_authority
                    != strict_biological_species_concept_adapter_rule_v1()
                {
                    return Err(SpeciesConceptError::AdapterRuleMismatch);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "validated species-concept authority should gate model-neutral species reasoning"]
pub struct ValidatedSpeciesConceptAuthority<'a> {
    authority: &'a SpeciesConceptAuthority,
    authority_digest: SpeciesConceptAuthorityDigest,
    conceptual_identity: SpeciesConceptIdentity,
}

impl<'a> ValidatedSpeciesConceptAuthority<'a> {
    pub fn validate_current_strict_biological(
        authority: &'a SpeciesConceptAuthority,
        model: &ValidatedBiologicalSpeciesModel<'_>,
    ) -> Result<Self, SpeciesConceptError> {
        authority.validate_local()?;
        let recomputed = SpeciesConceptAuthority::from_strict_biological(model)?;
        if recomputed != *authority {
            return Err(SpeciesConceptError::ReplayMismatch);
        }
        Ok(Self {
            authority,
            authority_digest: authority.canonical_digest()?,
            conceptual_identity: authority.conceptual_identity()?,
        })
    }

    pub fn authority(&self) -> &'a SpeciesConceptAuthority {
        self.authority
    }

    pub fn authority_digest(&self) -> SpeciesConceptAuthorityDigest {
        self.authority_digest
    }

    pub fn conceptual_identity(&self) -> &SpeciesConceptIdentity {
        &self.conceptual_identity
    }

    pub fn supports_capability(&self, capability: SpeciesConceptCapability) -> bool {
        self.authority.capabilities.contains(&capability)
    }

    pub fn require_capability(
        &self,
        capability: SpeciesConceptCapability,
    ) -> Result<(), SpeciesConceptError> {
        if self.supports_capability(capability) {
            Ok(())
        } else {
            Err(SpeciesConceptError::UnsupportedCapability(capability))
        }
    }
}

fn strict_biological_capabilities() -> Vec<SpeciesConceptCapability> {
    vec![
        SpeciesConceptCapability::CurrentSpeciesStatus,
        SpeciesConceptCapability::HistoricalTransitionInterval,
    ]
}

fn strict_biological_requirements() -> Vec<SpeciesConceptEvidenceRequirement> {
    vec![
        SpeciesConceptEvidenceRequirement::ExplicitQualifiedModelApplicability,
        SpeciesConceptEvidenceRequirement::CurrentCompleteReproductiveBarrier,
        SpeciesConceptEvidenceRequirement::PersistentOrRecontactLineageHistory,
        SpeciesConceptEvidenceRequirement::HistoricalTransitionTemporalEvidence,
    ]
}

fn strict_biological_domain_constraints() -> Vec<SpeciesConceptDomainConstraint> {
    vec![SpeciesConceptDomainConstraint::ReproductiveIsolationMustBeBiologicallyMeaningful]
}

fn strict_biological_concept_content_digest(
    source_content: SpeciesModelContentDigest,
    capabilities: &[SpeciesConceptCapability],
    requirements: &[SpeciesConceptEvidenceRequirement],
    constraints: &[SpeciesConceptDomainConstraint],
) -> SpeciesConceptContentDigest {
    let mut digest = Sha256::new();
    digest.update(CONTENT_DOMAIN);
    put_text(&mut digest, strict_biological_family_id().as_str());
    put_u32(
        &mut digest,
        STRICT_BIOLOGICAL_SPECIES_CONCEPT_FAMILY_VERSION,
    );
    digest.update(source_content.as_bytes());
    put_u64(&mut digest, STRICT_BSC_ADAPTER_SPEC.len() as u64);
    digest.update(STRICT_BSC_ADAPTER_SPEC);
    put_u64(&mut digest, capabilities.len() as u64);
    for capability in capabilities {
        digest.update([capability.tag()]);
    }
    put_u64(&mut digest, requirements.len() as u64);
    for requirement in requirements {
        digest.update([requirement.tag()]);
    }
    put_u64(&mut digest, constraints.len() as u64);
    for constraint in constraints {
        digest.update([constraint.tag()]);
    }
    SpeciesConceptContentDigest(digest.finalize().into())
}

fn strict_biological_concept_validity_domain_digest(
    source_domain: SpeciesModelValidityDomainDigest,
) -> SpeciesConceptValidityDomainDigest {
    let mut digest = Sha256::new();
    digest.update(VALIDITY_DOMAIN);
    put_text(&mut digest, strict_biological_family_id().as_str());
    put_u32(
        &mut digest,
        STRICT_BIOLOGICAL_SPECIES_CONCEPT_FAMILY_VERSION,
    );
    digest.update(source_domain.as_bytes());
    SpeciesConceptValidityDomainDigest(digest.finalize().into())
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum SpeciesConceptError {
    Evolution(EvolutionError),
    SourceModel(SpeciesModelError),
    UnsupportedVersion(u32),
    SourceModelDigestMismatch,
    FamilyMismatch,
    ModelIdMismatch,
    CapabilitySurfaceMismatch,
    EvidenceRequirementMismatch,
    DomainConstraintMismatch,
    ConceptContentMismatch,
    ValidityDomainMismatch,
    QualificationBindingMismatch,
    AdapterRuleMismatch,
    UnsupportedCapability(SpeciesConceptCapability),
    ReplayMismatch,
}

impl From<EvolutionError> for SpeciesConceptError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<SpeciesModelError> for SpeciesConceptError {
    fn from(value: SpeciesModelError) -> Self {
        Self::SourceModel(value)
    }
}

impl fmt::Display for SpeciesConceptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "species-concept identity error: {error}"),
            Self::SourceModel(error) => write!(f, "source species-model error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported species-concept authority version {version}")
            }
            Self::SourceModelDigestMismatch => {
                write!(f, "embedded source model does not match its persisted digest")
            }
            Self::FamilyMismatch => write!(
                f,
                "species-concept family/version does not match the embedded source model adapter"
            ),
            Self::ModelIdMismatch => write!(
                f,
                "model-neutral species-concept model ID does not match the source model ID"
            ),
            Self::CapabilitySurfaceMismatch => write!(
                f,
                "species-concept capability surface does not match the frozen source adapter"
            ),
            Self::EvidenceRequirementMismatch => write!(
                f,
                "species-concept evidence requirements do not match the frozen source adapter"
            ),
            Self::DomainConstraintMismatch => write!(
                f,
                "species-concept domain constraints do not match the frozen source adapter"
            ),
            Self::ConceptContentMismatch => write!(
                f,
                "species-concept semantic content digest does not match the source model semantics"
            ),
            Self::ValidityDomainMismatch => write!(
                f,
                "species-concept validity-domain digest does not match the source model domain"
            ),
            Self::QualificationBindingMismatch => write!(
                f,
                "species-concept source-qualification provenance does not match the source model"
            ),
            Self::AdapterRuleMismatch => write!(
                f,
                "species-concept adapter rule does not match the built-in strict-BSC adapter"
            ),
            Self::UnsupportedCapability(capability) => {
                write!(f, "species-concept authority does not support {capability:?}")
            }
            Self::ReplayMismatch => write!(
                f,
                "persisted species-concept authority does not replay against the current source model"
            ),
        }
    }
}

impl Error for SpeciesConceptError {}
