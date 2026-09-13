use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    error::validate_text,
    AnalysisAuthorityRef, EvolutionError,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const BIOLOGICAL_SPECIES_MODEL_VERSION: u32 = 1;
const MODEL_DOMAIN: &[u8] = b"symtropy:evolution:biological-species-model:v1\0";
const CONTENT_DOMAIN: &[u8] = b"symtropy:evolution:biological-species-model-content:v1\0";
const VALIDITY_DOMAIN: &[u8] = b"symtropy:evolution:species-model-validity-domain:v1\0";
const MODEL_SPEC: &[u8] = b"strict biological-species model v1: current complete reproductive-isolation evidence is required; persistent lineage-divergence history is required; historical recontact without current complete-barrier contradiction may remain compatible; current viable fertile hybridization or realized hereditary gene flow contradicts the complete-isolation lane; lineage fusion or explicit loss of lineage persistence contradicts distinct current species status; geographic separation alone is never sufficient; incomplete lineage sorting alone does not decide status; ecology is not required by this narrow v1 lane; missing or conflicting required evidence yields insufficient evidence; model is current-status only and does not establish a historical speciation event";

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

local_id!(SpeciesModelId, "SpeciesModelId");
local_id!(SpeciesModelValidityDomainId, "SpeciesModelValidityDomainId");

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciesModelContentDigest([u8; 32]);

impl SpeciesModelContentDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciesModelContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesModelContentDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciesModelContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

pub fn strict_biological_species_model_content_digest_v1() -> SpeciesModelContentDigest {
    let mut digest = Sha256::new();
    digest.update(CONTENT_DOMAIN);
    put_u64(&mut digest, MODEL_SPEC.len() as u64);
    digest.update(MODEL_SPEC);
    SpeciesModelContentDigest(digest.finalize().into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveIsolationModelRequirement {
    CurrentCompleteBarrierSupported,
}

impl ReproductiveIsolationModelRequirement {
    fn tag(self) -> u8 {
        match self {
            Self::CurrentCompleteBarrierSupported => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageDivergenceModelRequirement {
    PersistentOrRecontactWithoutFusion,
}

impl LineageDivergenceModelRequirement {
    fn tag(self) -> u8 {
        match self {
            Self::PersistentOrRecontactWithoutFusion => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentGeneFlowPolicy {
    ContradictsCompleteIsolation,
}

impl CurrentGeneFlowPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::ContradictsCompleteIsolation => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistoricalRecontactPolicy {
    CompatibleWhenCurrentCompleteBarrierSupported,
}

impl HistoricalRecontactPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::CompatibleWhenCurrentCompleteBarrierSupported => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineageFusionSpeciesPolicy {
    ContradictsDistinctCurrentSpecies,
}

impl LineageFusionSpeciesPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::ContradictsDistinctCurrentSpecies => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GeographicIsolationPolicy {
    NeverSufficientAlone,
}

impl GeographicIsolationPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::NeverSufficientAlone => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IncompleteLineageSortingPolicy {
    DoesNotDecideStatusAlone,
}

impl IncompleteLineageSortingPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::DoesNotDecideStatusAlone => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EcologySpeciesEvidencePolicy {
    NotRequiredInStrictV1,
}

impl EcologySpeciesEvidencePolicy {
    fn tag(self) -> u8 {
        match self {
            Self::NotRequiredInStrictV1 => 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesModelMissingConflictPolicy {
    InsufficientEvidence,
}

impl SpeciesModelMissingConflictPolicy {
    fn tag(self) -> u8 {
        match self {
            Self::InsufficientEvidence => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesModelValidityDomainRef {
    pub domain_id: SpeciesModelValidityDomainId,
    pub authority: AnalysisAuthorityRef,
}

impl SpeciesModelValidityDomainRef {
    pub fn new(domain_id: SpeciesModelValidityDomainId, authority: AnalysisAuthorityRef) -> Self {
        Self {
            domain_id,
            authority,
        }
    }

    pub fn canonical_digest(&self) -> SpeciesModelValidityDomainDigest {
        let mut digest = Sha256::new();
        digest.update(VALIDITY_DOMAIN);
        put_text(&mut digest, self.domain_id.as_str());
        put_authority(&mut digest, &self.authority);
        SpeciesModelValidityDomainDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciesModelValidityDomainDigest([u8; 32]);

impl SpeciesModelValidityDomainDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SpeciesModelValidityDomainDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SpeciesModelValidityDomainDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for SpeciesModelValidityDomainDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesModelQualificationRef {
    pub authority: AnalysisAuthorityRef,
    pub model_content_digest: SpeciesModelContentDigest,
    pub validity_domain_digest: SpeciesModelValidityDomainDigest,
}

impl SpeciesModelQualificationRef {
    fn bind(
        authority: AnalysisAuthorityRef,
        model_content_digest: SpeciesModelContentDigest,
        validity_domain_digest: SpeciesModelValidityDomainDigest,
    ) -> Self {
        Self {
            authority,
            model_content_digest,
            validity_domain_digest,
        }
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.authority);
        digest.update(self.model_content_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BiologicalSpeciesModel {
    model_version: u32,
    pub model_id: SpeciesModelId,
    pub model_content_digest: SpeciesModelContentDigest,
    pub validity_domain: SpeciesModelValidityDomainRef,
    pub qualification: SpeciesModelQualificationRef,
    pub reproductive_isolation: ReproductiveIsolationModelRequirement,
    pub lineage_divergence: LineageDivergenceModelRequirement,
    pub current_gene_flow: CurrentGeneFlowPolicy,
    pub historical_recontact: HistoricalRecontactPolicy,
    pub lineage_fusion: LineageFusionSpeciesPolicy,
    pub geographic_isolation: GeographicIsolationPolicy,
    pub incomplete_lineage_sorting: IncompleteLineageSortingPolicy,
    pub ecology: EcologySpeciesEvidencePolicy,
    pub missing_or_conflicting_evidence: SpeciesModelMissingConflictPolicy,
}

impl BiologicalSpeciesModel {
    pub fn qualify(
        model_id: SpeciesModelId,
        validity_domain: SpeciesModelValidityDomainRef,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, SpeciesModelError> {
        let model_content_digest = strict_biological_species_model_content_digest_v1();
        let validity_domain_digest = validity_domain.canonical_digest();
        let model = Self {
            model_version: BIOLOGICAL_SPECIES_MODEL_VERSION,
            model_id,
            model_content_digest,
            qualification: SpeciesModelQualificationRef::bind(
                qualification_authority,
                model_content_digest,
                validity_domain_digest,
            ),
            validity_domain,
            reproductive_isolation:
                ReproductiveIsolationModelRequirement::CurrentCompleteBarrierSupported,
            lineage_divergence:
                LineageDivergenceModelRequirement::PersistentOrRecontactWithoutFusion,
            current_gene_flow: CurrentGeneFlowPolicy::ContradictsCompleteIsolation,
            historical_recontact:
                HistoricalRecontactPolicy::CompatibleWhenCurrentCompleteBarrierSupported,
            lineage_fusion: LineageFusionSpeciesPolicy::ContradictsDistinctCurrentSpecies,
            geographic_isolation: GeographicIsolationPolicy::NeverSufficientAlone,
            incomplete_lineage_sorting:
                IncompleteLineageSortingPolicy::DoesNotDecideStatusAlone,
            ecology: EcologySpeciesEvidencePolicy::NotRequiredInStrictV1,
            missing_or_conflicting_evidence: SpeciesModelMissingConflictPolicy::InsufficientEvidence,
        };
        model.validate_local()?;
        Ok(model)
    }

    pub fn canonical_digest(&self) -> Result<BiologicalSpeciesModelDigest, SpeciesModelError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(MODEL_DOMAIN);
        put_u32(&mut digest, self.model_version);
        put_text(&mut digest, self.model_id.as_str());
        digest.update(self.model_content_digest.as_bytes());
        digest.update(self.validity_domain.canonical_digest().as_bytes());
        self.qualification.put(&mut digest);
        digest.update([self.reproductive_isolation.tag()]);
        digest.update([self.lineage_divergence.tag()]);
        digest.update([self.current_gene_flow.tag()]);
        digest.update([self.historical_recontact.tag()]);
        digest.update([self.lineage_fusion.tag()]);
        digest.update([self.geographic_isolation.tag()]);
        digest.update([self.incomplete_lineage_sorting.tag()]);
        digest.update([self.ecology.tag()]);
        digest.update([self.missing_or_conflicting_evidence.tag()]);
        Ok(BiologicalSpeciesModelDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SpeciesModelError> {
        if self.model_version != BIOLOGICAL_SPECIES_MODEL_VERSION {
            return Err(SpeciesModelError::UnsupportedVersion(self.model_version));
        }
        let expected_content = strict_biological_species_model_content_digest_v1();
        if self.model_content_digest != expected_content {
            return Err(SpeciesModelError::ModelContentMismatch);
        }
        let expected_domain = self.validity_domain.canonical_digest();
        if self.qualification.model_content_digest != expected_content
            || self.qualification.validity_domain_digest != expected_domain
        {
            return Err(SpeciesModelError::QualificationBindingMismatch);
        }
        if self.reproductive_isolation
            != ReproductiveIsolationModelRequirement::CurrentCompleteBarrierSupported
            || self.lineage_divergence
                != LineageDivergenceModelRequirement::PersistentOrRecontactWithoutFusion
            || self.current_gene_flow != CurrentGeneFlowPolicy::ContradictsCompleteIsolation
            || self.historical_recontact
                != HistoricalRecontactPolicy::CompatibleWhenCurrentCompleteBarrierSupported
            || self.lineage_fusion
                != LineageFusionSpeciesPolicy::ContradictsDistinctCurrentSpecies
            || self.geographic_isolation != GeographicIsolationPolicy::NeverSufficientAlone
            || self.incomplete_lineage_sorting
                != IncompleteLineageSortingPolicy::DoesNotDecideStatusAlone
            || self.ecology != EcologySpeciesEvidencePolicy::NotRequiredInStrictV1
            || self.missing_or_conflicting_evidence
                != SpeciesModelMissingConflictPolicy::InsufficientEvidence
        {
            return Err(SpeciesModelError::PolicyMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BiologicalSpeciesModelDigest([u8; 32]);

impl BiologicalSpeciesModelDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for BiologicalSpeciesModelDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BiologicalSpeciesModelDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for BiologicalSpeciesModelDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated species model should gate later model-bound species classification"]
pub struct ValidatedBiologicalSpeciesModel<'a> {
    model: &'a BiologicalSpeciesModel,
    model_digest: BiologicalSpeciesModelDigest,
}

impl<'a> ValidatedBiologicalSpeciesModel<'a> {
    pub fn validate_current(
        model: &'a BiologicalSpeciesModel,
        validity_domain: SpeciesModelValidityDomainRef,
        qualification_authority: AnalysisAuthorityRef,
    ) -> Result<Self, SpeciesModelError> {
        model.validate_local()?;
        let recomputed = BiologicalSpeciesModel::qualify(
            model.model_id.clone(),
            validity_domain,
            qualification_authority,
        )?;
        if recomputed != *model {
            return Err(SpeciesModelError::ReplayMismatch);
        }
        Ok(Self {
            model,
            model_digest: model.canonical_digest()?,
        })
    }

    pub fn model(&self) -> &'a BiologicalSpeciesModel {
        self.model
    }

    pub fn model_digest(&self) -> BiologicalSpeciesModelDigest {
        self.model_digest
    }
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum SpeciesModelError {
    UnsupportedVersion(u32),
    ModelContentMismatch,
    QualificationBindingMismatch,
    PolicyMismatch,
    ReplayMismatch,
}

impl fmt::Display for SpeciesModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported biological-species model version {version}")
            }
            Self::ModelContentMismatch => write!(
                f,
                "persisted species model does not bind the built-in strict biological-species V1 content"
            ),
            Self::QualificationBindingMismatch => write!(
                f,
                "species-model qualification does not bind the exact model content and validity domain"
            ),
            Self::PolicyMismatch => write!(
                f,
                "persisted species-model policy does not match strict biological-species V1"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted species model does not replay against current domain/qualification authorities"
            ),
        }
    }
}

impl Error for SpeciesModelError {}
