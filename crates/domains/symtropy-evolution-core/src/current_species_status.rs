use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AnalysisAuthorityRef, BiologicalSpeciesModel, BiologicalSpeciesModelDigest,
    CurrentSpeciesClassificationDesign, CurrentSpeciesClassificationDesignDigest,
    LineageDivergenceHistory, LineageDivergenceHistoryDigest, LineageDivergenceHistoryStatus,
    ReproductiveIsolationEvidence, ReproductiveIsolationEvidenceDigest, ReproductiveIsolationStatus,
    SpeciesModelValidityDomainDigest, ValidatedBiologicalSpeciesModel,
    ValidatedCurrentSpeciesClassificationDesign, ValidatedLineageDivergenceHistory,
    ValidatedReproductiveIsolationEvidence,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const CURRENT_SPECIES_STATUS_EVIDENCE_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:current-species-status-evidence:v1\0";

#[derive(Debug, Clone)]
pub enum SpeciesModelApplicabilityInput {
    InsideValidityDomain { evidence: AnalysisAuthorityRef },
    OutsideValidityDomain { evidence: AnalysisAuthorityRef },
    Unavailable { reason: AnalysisAuthorityRef },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeciesModelApplicabilityDisposition {
    InsideValidityDomain,
    OutsideValidityDomain,
    Unavailable,
}

impl SpeciesModelApplicabilityDisposition {
    fn tag(&self) -> u8 {
        match self {
            Self::InsideValidityDomain => 0,
            Self::OutsideValidityDomain => 1,
            Self::Unavailable => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeciesModelApplicabilityEvidence {
    pub lineage_a: AnalysisAuthorityRef,
    pub lineage_b: AnalysisAuthorityRef,
    pub species_model_digest: BiologicalSpeciesModelDigest,
    pub validity_domain_digest: SpeciesModelValidityDomainDigest,
    pub protocol: AnalysisAuthorityRef,
    pub disposition: SpeciesModelApplicabilityDisposition,
    pub evidence: AnalysisAuthorityRef,
}

impl SpeciesModelApplicabilityEvidence {
    fn materialize(
        design: &CurrentSpeciesClassificationDesign,
        input: SpeciesModelApplicabilityInput,
    ) -> Self {
        let (disposition, evidence) = match input {
            SpeciesModelApplicabilityInput::InsideValidityDomain { evidence } => {
                (SpeciesModelApplicabilityDisposition::InsideValidityDomain, evidence)
            }
            SpeciesModelApplicabilityInput::OutsideValidityDomain { evidence } => {
                (SpeciesModelApplicabilityDisposition::OutsideValidityDomain, evidence)
            }
            SpeciesModelApplicabilityInput::Unavailable { reason } => {
                (SpeciesModelApplicabilityDisposition::Unavailable, reason)
            }
        };
        Self {
            lineage_a: design.lineage_a.clone(),
            lineage_b: design.lineage_b.clone(),
            species_model_digest: design.species_model_digest,
            validity_domain_digest: design.validity_domain_digest,
            protocol: design.applicability_protocol.clone(),
            disposition,
            evidence,
        }
    }

    fn validate_against(
        &self,
        design: &CurrentSpeciesClassificationDesign,
    ) -> Result<(), CurrentSpeciesStatusError> {
        if self.lineage_a != design.lineage_a || self.lineage_b != design.lineage_b {
            return Err(CurrentSpeciesStatusError::ApplicabilityLineageMismatch);
        }
        if self.species_model_digest != design.species_model_digest
            || self.validity_domain_digest != design.validity_domain_digest
        {
            return Err(CurrentSpeciesStatusError::ApplicabilityModelMismatch);
        }
        if self.protocol != design.applicability_protocol {
            return Err(CurrentSpeciesStatusError::ApplicabilityProtocolMismatch);
        }
        Ok(())
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.lineage_a);
        put_authority(digest, &self.lineage_b);
        digest.update(self.species_model_digest.as_bytes());
        digest.update(self.validity_domain_digest.as_bytes());
        put_authority(digest, &self.protocol);
        digest.update([self.disposition.tag()]);
        put_authority(digest, &self.evidence);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CurrentSpeciesStatus {
    SupportedUnderModel,
    NotSupportedUnderModel,
    ContradictedUnderModel,
    InsufficientEvidence,
    OutsideModelValidityDomain,
}

impl CurrentSpeciesStatus {
    fn tag(self) -> u8 {
        match self {
            Self::SupportedUnderModel => 0,
            Self::NotSupportedUnderModel => 1,
            Self::ContradictedUnderModel => 2,
            Self::InsufficientEvidence => 3,
            Self::OutsideModelValidityDomain => 4,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrentSpeciesStatusEvidence {
    evidence_version: u32,
    design: CurrentSpeciesClassificationDesign,
    design_digest: CurrentSpeciesClassificationDesignDigest,
    lineage_history: LineageDivergenceHistory,
    lineage_history_digest: LineageDivergenceHistoryDigest,
    reproductive_isolation: ReproductiveIsolationEvidence,
    reproductive_isolation_digest: ReproductiveIsolationEvidenceDigest,
    species_model: BiologicalSpeciesModel,
    species_model_digest: BiologicalSpeciesModelDigest,
    pub applicability: SpeciesModelApplicabilityEvidence,
    pub status: CurrentSpeciesStatus,
}

impl CurrentSpeciesStatusEvidence {
    pub fn evaluate(
        design: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationEvidence<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        applicability: SpeciesModelApplicabilityInput,
    ) -> Result<Self, CurrentSpeciesStatusError> {
        let frozen = design.design();
        if lineage_history.design_digest() != frozen.lineage_history_design_digest {
            return Err(CurrentSpeciesStatusError::LineageHistoryDesignMismatch);
        }
        if reproductive_isolation.design_digest() != frozen.reproductive_isolation_design_digest {
            return Err(CurrentSpeciesStatusError::IsolationDesignMismatch);
        }
        if species_model.model_digest() != frozen.species_model_digest {
            return Err(CurrentSpeciesStatusError::SpeciesModelMismatch);
        }

        let history_snapshot = lineage_history.history().clone();
        let isolation_snapshot = reproductive_isolation.evidence().clone();
        let model_snapshot = species_model.model().clone();
        let applicability = SpeciesModelApplicabilityEvidence::materialize(frozen, applicability);
        let status = derive_status(
            &applicability,
            history_snapshot.status,
            isolation_snapshot.status,
        );
        let evidence = Self {
            evidence_version: CURRENT_SPECIES_STATUS_EVIDENCE_VERSION,
            design: frozen.clone(),
            design_digest: design.design_digest(),
            lineage_history_digest: lineage_history.history_digest(),
            lineage_history: history_snapshot,
            reproductive_isolation_digest: reproductive_isolation.evidence_digest(),
            reproductive_isolation: isolation_snapshot,
            species_model_digest: species_model.model_digest(),
            species_model: model_snapshot,
            applicability,
            status,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn design(&self) -> &CurrentSpeciesClassificationDesign {
        &self.design
    }

    pub fn design_digest(&self) -> CurrentSpeciesClassificationDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CurrentSpeciesStatusEvidenceDigest, CurrentSpeciesStatusError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        digest.update(self.lineage_history_digest.as_bytes());
        digest.update(self.reproductive_isolation_digest.as_bytes());
        digest.update(self.species_model_digest.as_bytes());
        self.applicability.put(&mut digest);
        digest.update([self.status.tag()]);
        Ok(CurrentSpeciesStatusEvidenceDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), CurrentSpeciesStatusError> {
        if self.evidence_version != CURRENT_SPECIES_STATUS_EVIDENCE_VERSION {
            return Err(CurrentSpeciesStatusError::UnsupportedVersion(
                self.evidence_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(CurrentSpeciesStatusError::DesignBindingMismatch);
        }
        if self.lineage_history.canonical_digest()? != self.lineage_history_digest
            || self.lineage_history.design_digest() != self.design.lineage_history_design_digest
        {
            return Err(CurrentSpeciesStatusError::LineageHistoryBindingMismatch);
        }
        if self.reproductive_isolation.canonical_digest()? != self.reproductive_isolation_digest
            || self.reproductive_isolation.design_digest()
                != self.design.reproductive_isolation_design_digest
        {
            return Err(CurrentSpeciesStatusError::IsolationBindingMismatch);
        }
        if self.species_model.canonical_digest()? != self.species_model_digest
            || self.species_model_digest != self.design.species_model_digest
            || self.species_model.model_content_digest != self.design.species_model_content_digest
            || self.species_model.validity_domain.canonical_digest()
                != self.design.validity_domain_digest
        {
            return Err(CurrentSpeciesStatusError::SpeciesModelBindingMismatch);
        }
        if self.lineage_history.design().lineage_a != self.design.lineage_a
            || self.lineage_history.design().lineage_b != self.design.lineage_b
            || self.reproductive_isolation.design().lineage_a != self.design.lineage_a
            || self.reproductive_isolation.design().lineage_b != self.design.lineage_b
        {
            return Err(CurrentSpeciesStatusError::TargetLineageBindingMismatch);
        }
        self.applicability.validate_against(&self.design)?;
        let expected = derive_status(
            &self.applicability,
            self.lineage_history.status,
            self.reproductive_isolation.status,
        );
        if expected != self.status {
            return Err(CurrentSpeciesStatusError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CurrentSpeciesStatusEvidenceDigest([u8; 32]);

impl CurrentSpeciesStatusEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for CurrentSpeciesStatusEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CurrentSpeciesStatusEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for CurrentSpeciesStatusEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated current species status is model-bound current evidence, not a speciation event"]
pub struct ValidatedCurrentSpeciesStatus<'a> {
    evidence: &'a CurrentSpeciesStatusEvidence,
    evidence_digest: CurrentSpeciesStatusEvidenceDigest,
    design_digest: CurrentSpeciesClassificationDesignDigest,
}

impl<'a> ValidatedCurrentSpeciesStatus<'a> {
    pub fn validate_current(
        evidence: &'a CurrentSpeciesStatusEvidence,
        design: &ValidatedCurrentSpeciesClassificationDesign<'_>,
        lineage_history: &ValidatedLineageDivergenceHistory<'_>,
        reproductive_isolation: &ValidatedReproductiveIsolationEvidence<'_>,
        species_model: &ValidatedBiologicalSpeciesModel<'_>,
        applicability: SpeciesModelApplicabilityInput,
    ) -> Result<Self, CurrentSpeciesStatusError> {
        evidence.validate_local()?;
        let recomputed = CurrentSpeciesStatusEvidence::evaluate(
            design,
            lineage_history,
            reproductive_isolation,
            species_model,
            applicability,
        )?;
        if recomputed != *evidence {
            return Err(CurrentSpeciesStatusError::ReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn evidence(&self) -> &'a CurrentSpeciesStatusEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> CurrentSpeciesStatusEvidenceDigest {
        self.evidence_digest
    }

    pub fn design_digest(&self) -> CurrentSpeciesClassificationDesignDigest {
        self.design_digest
    }
}

fn derive_status(
    applicability: &SpeciesModelApplicabilityEvidence,
    history: LineageDivergenceHistoryStatus,
    isolation: ReproductiveIsolationStatus,
) -> CurrentSpeciesStatus {
    match applicability.disposition {
        SpeciesModelApplicabilityDisposition::OutsideValidityDomain => {
            return CurrentSpeciesStatus::OutsideModelValidityDomain;
        }
        SpeciesModelApplicabilityDisposition::Unavailable => {
            return CurrentSpeciesStatus::InsufficientEvidence;
        }
        SpeciesModelApplicabilityDisposition::InsideValidityDomain => {}
    }

    if matches!(
        history,
        LineageDivergenceHistoryStatus::LineageFusionObserved
            | LineageDivergenceHistoryStatus::NotPersistent
    ) || isolation == ReproductiveIsolationStatus::Contradicted
    {
        return CurrentSpeciesStatus::ContradictedUnderModel;
    }
    if history == LineageDivergenceHistoryStatus::InsufficientEvidence
        || isolation == ReproductiveIsolationStatus::InsufficientEvidence
    {
        return CurrentSpeciesStatus::InsufficientEvidence;
    }
    if isolation == ReproductiveIsolationStatus::NotSupported {
        return CurrentSpeciesStatus::NotSupportedUnderModel;
    }
    if isolation == ReproductiveIsolationStatus::Supported
        && matches!(
            history,
            LineageDivergenceHistoryStatus::PersistentDivergenceObserved
                | LineageDivergenceHistoryStatus::DivergenceWithRecontact
        )
    {
        return CurrentSpeciesStatus::SupportedUnderModel;
    }
    CurrentSpeciesStatus::NotSupportedUnderModel
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum CurrentSpeciesStatusError {
    Design(crate::CurrentSpeciesClassificationDesignError),
    LineageHistory(crate::LineageDivergenceHistoryError),
    Isolation(crate::ReproductiveIsolationEvidenceError),
    SpeciesModel(crate::SpeciesModelError),
    UnsupportedVersion(u32),
    LineageHistoryDesignMismatch,
    IsolationDesignMismatch,
    SpeciesModelMismatch,
    DesignBindingMismatch,
    LineageHistoryBindingMismatch,
    IsolationBindingMismatch,
    SpeciesModelBindingMismatch,
    TargetLineageBindingMismatch,
    ApplicabilityLineageMismatch,
    ApplicabilityModelMismatch,
    ApplicabilityProtocolMismatch,
    StatusInvariant,
    ReplayMismatch,
}

impl From<crate::CurrentSpeciesClassificationDesignError> for CurrentSpeciesStatusError {
    fn from(value: crate::CurrentSpeciesClassificationDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<crate::LineageDivergenceHistoryError> for CurrentSpeciesStatusError {
    fn from(value: crate::LineageDivergenceHistoryError) -> Self {
        Self::LineageHistory(value)
    }
}

impl From<crate::ReproductiveIsolationEvidenceError> for CurrentSpeciesStatusError {
    fn from(value: crate::ReproductiveIsolationEvidenceError) -> Self {
        Self::Isolation(value)
    }
}

impl From<crate::SpeciesModelError> for CurrentSpeciesStatusError {
    fn from(value: crate::SpeciesModelError) -> Self {
        Self::SpeciesModel(value)
    }
}

impl fmt::Display for CurrentSpeciesStatusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "current species-classification design error: {error}"),
            Self::LineageHistory(error) => write!(f, "lineage-history error: {error}"),
            Self::Isolation(error) => write!(f, "reproductive-isolation error: {error}"),
            Self::SpeciesModel(error) => write!(f, "species-model error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported current species-status evidence version {version}")
            }
            Self::LineageHistoryDesignMismatch => {
                write!(f, "current SEL-10A history does not match the preregistered design")
            }
            Self::IsolationDesignMismatch => {
                write!(f, "current SEL-09B evidence does not match the preregistered design")
            }
            Self::SpeciesModelMismatch => {
                write!(f, "current SEL-10B model does not match the preregistered model")
            }
            Self::DesignBindingMismatch => {
                write!(f, "persisted species-status evidence binds a different design")
            }
            Self::LineageHistoryBindingMismatch => {
                write!(f, "persisted lineage-history snapshot/digest does not match the frozen design")
            }
            Self::IsolationBindingMismatch => {
                write!(f, "persisted reproductive-isolation snapshot/digest does not match the frozen design")
            }
            Self::SpeciesModelBindingMismatch => {
                write!(f, "persisted species-model snapshot/digest does not match the frozen design")
            }
            Self::TargetLineageBindingMismatch => {
                write!(f, "upstream current evidence does not bind the preregistered ordered lineage pair")
            }
            Self::ApplicabilityLineageMismatch => {
                write!(f, "model-applicability evidence binds a different lineage pair")
            }
            Self::ApplicabilityModelMismatch => {
                write!(f, "model-applicability evidence binds a different model/domain")
            }
            Self::ApplicabilityProtocolMismatch => {
                write!(f, "model-applicability evidence does not bind the preregistered protocol")
            }
            Self::StatusInvariant => {
                write!(f, "persisted current species status does not recompute from current evidence/model")
            }
            Self::ReplayMismatch => write!(
                f,
                "persisted current species-status evidence does not replay against current authorities"
            ),
        }
    }
}

impl Error for CurrentSpeciesStatusError {}
