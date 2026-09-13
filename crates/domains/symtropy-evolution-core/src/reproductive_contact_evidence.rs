use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AnalysisAuthorityRef,
    ReproductionEventId, ReproductionProvenanceDigest, ReproductiveContactStudyDesign,
    ReproductiveContactStudyDesignDigest, ReproductiveOpportunityId,
    ValidatedReproductiveContactStudyDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const REPRODUCTIVE_CONTACT_STUDY_VERSION: u32 = 1;
const STUDY_DOMAIN: &[u8] = b"symtropy:evolution:reproductive-contact-study:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveObservationStage {
    Contact, Pairing, Mating, Conception, OffspringViability, OffspringFertility,
}
impl ReproductiveObservationStage {
    fn tag(self) -> u8 {
        match self {
            Self::Contact => 0, Self::Pairing => 1, Self::Mating => 2,
            Self::Conception => 3, Self::OffspringViability => 4, Self::OffspringFertility => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveOpportunityOutcome {
    NoContact { evidence: AnalysisAuthorityRef },
    ContactNoPairing { evidence: AnalysisAuthorityRef },
    PairedNoMating { evidence: AnalysisAuthorityRef },
    MatingNoConception { evidence: AnalysisAuthorityRef },
    ConceptionNoViableOffspring { evidence: AnalysisAuthorityRef },
    ViableOffspringFertilityUnknown {
        event_id: ReproductionEventId,
        reproduction_provenance_digest: ReproductionProvenanceDigest,
        evidence: AnalysisAuthorityRef,
    },
    ViableInfertileOffspring {
        event_id: ReproductionEventId,
        reproduction_provenance_digest: ReproductionProvenanceDigest,
        evidence: AnalysisAuthorityRef,
    },
    ViableFertileOffspring {
        event_id: ReproductionEventId,
        reproduction_provenance_digest: ReproductionProvenanceDigest,
        evidence: AnalysisAuthorityRef,
    },
    Unavailable { stage: ReproductiveObservationStage, reason: AnalysisAuthorityRef },
}

impl ReproductiveOpportunityOutcome {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::NoContact { evidence } => { digest.update([0]); update_authority_digest(digest, evidence); }
            Self::ContactNoPairing { evidence } => { digest.update([1]); update_authority_digest(digest, evidence); }
            Self::PairedNoMating { evidence } => { digest.update([2]); update_authority_digest(digest, evidence); }
            Self::MatingNoConception { evidence } => { digest.update([3]); update_authority_digest(digest, evidence); }
            Self::ConceptionNoViableOffspring { evidence } => { digest.update([4]); update_authority_digest(digest, evidence); }
            Self::ViableOffspringFertilityUnknown { event_id, reproduction_provenance_digest, evidence } => {
                digest.update([5]); put_text(digest, event_id.as_str());
                digest.update(reproduction_provenance_digest.as_bytes()); update_authority_digest(digest, evidence);
            }
            Self::ViableInfertileOffspring { event_id, reproduction_provenance_digest, evidence } => {
                digest.update([6]); put_text(digest, event_id.as_str());
                digest.update(reproduction_provenance_digest.as_bytes()); update_authority_digest(digest, evidence);
            }
            Self::ViableFertileOffspring { event_id, reproduction_provenance_digest, evidence } => {
                digest.update([7]); put_text(digest, event_id.as_str());
                digest.update(reproduction_provenance_digest.as_bytes()); update_authority_digest(digest, evidence);
            }
            Self::Unavailable { stage, reason } => {
                digest.update([8, stage.tag()]); update_authority_digest(digest, reason);
            }
        }
    }

    fn is_unavailable(&self) -> bool { matches!(self, Self::Unavailable { .. }) }
    fn is_no_contact(&self) -> bool { matches!(self, Self::NoContact { .. }) }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RealizedGeneFlowObservation {
    NoneObserved {
        materialization_authority: AnalysisAuthorityRef,
        evidence: AnalysisAuthorityRef,
    },
    Realized {
        materialization_authority: AnalysisAuthorityRef,
        ancestry_evidence: AnalysisAuthorityRef,
    },
    Unavailable { reason: AnalysisAuthorityRef },
}

impl RealizedGeneFlowObservation {
    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::NoneObserved { materialization_authority, evidence } => {
                digest.update([0]); update_authority_digest(digest, materialization_authority);
                update_authority_digest(digest, evidence);
            }
            Self::Realized { materialization_authority, ancestry_evidence } => {
                digest.update([1]); update_authority_digest(digest, materialization_authority);
                update_authority_digest(digest, ancestry_evidence);
            }
            Self::Unavailable { reason } => { digest.update([2]); update_authority_digest(digest, reason); }
        }
    }
    fn is_realized(&self) -> bool { matches!(self, Self::Realized { .. }) }
    fn is_unavailable(&self) -> bool { matches!(self, Self::Unavailable { .. }) }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveOpportunityEvidenceInput {
    pub opportunity_id: ReproductiveOpportunityId,
    pub outcome: ReproductiveOpportunityOutcome,
    pub realized_gene_flow: RealizedGeneFlowObservation,
    pub demography_accounting_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveContactRecord {
    pub opportunity_id: ReproductiveOpportunityId,
    pub outcome: ReproductiveOpportunityOutcome,
    pub realized_gene_flow: RealizedGeneFlowObservation,
    pub demography_accounting_authority: AnalysisAuthorityRef,
}
impl ReproductiveContactRecord {
    fn from_input(input: ReproductiveOpportunityEvidenceInput) -> Self {
        Self {
            opportunity_id: input.opportunity_id,
            outcome: input.outcome,
            realized_gene_flow: input.realized_gene_flow,
            demography_accounting_authority: input.demography_accounting_authority,
        }
    }
    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.opportunity_id.as_str());
        self.outcome.update_digest(digest);
        self.realized_gene_flow.update_digest(digest);
        update_authority_digest(digest, &self.demography_accounting_authority);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveContactStudyStatus {
    CrossLineageGeneFlowObserved,
    ViableFertileHybridObserved,
    ViableInfertileHybridObserved,
    CrossLineageReproductionObserved,
    NoRealizedGeneFlowWithObservedOpportunity,
    NoContactOpportunity,
    InsufficientEvidence,
}
impl ReproductiveContactStudyStatus {
    fn tag(self) -> u8 {
        match self {
            Self::CrossLineageGeneFlowObserved => 0,
            Self::ViableFertileHybridObserved => 1,
            Self::ViableInfertileHybridObserved => 2,
            Self::CrossLineageReproductionObserved => 3,
            Self::NoRealizedGeneFlowWithObservedOpportunity => 4,
            Self::NoContactOpportunity => 5,
            Self::InsufficientEvidence => 6,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveContactStudy {
    study_version: u32,
    design: ReproductiveContactStudyDesign,
    design_digest: ReproductiveContactStudyDesignDigest,
    pub records: Vec<ReproductiveContactRecord>,
    pub status: ReproductiveContactStudyStatus,
}

impl ReproductiveContactStudy {
    pub fn capture(
        design: &ValidatedReproductiveContactStudyDesign<'_>,
        inputs: impl IntoIterator<Item = ReproductiveOpportunityEvidenceInput>,
    ) -> Result<Self, ReproductiveContactEvidenceError> {
        let mut by_id = BTreeMap::new();
        for input in inputs {
            let id = input.opportunity_id.clone();
            if by_id.insert(id.clone(), input).is_some() {
                return Err(ReproductiveContactEvidenceError::DuplicateOpportunity(id));
            }
        }
        if by_id.len() != design.design().opportunities.len() {
            return Err(ReproductiveContactEvidenceError::IncompleteOpportunityCoverage);
        }
        let mut records = Vec::with_capacity(design.design().opportunities.len());
        for declaration in &design.design().opportunities {
            let input = by_id.remove(&declaration.opportunity_id)
                .ok_or_else(|| ReproductiveContactEvidenceError::MissingOpportunity(declaration.opportunity_id.clone()))?;
            validate_input_against_design(design.design(), &input)?;
            records.push(ReproductiveContactRecord::from_input(input));
        }
        if !by_id.is_empty() { return Err(ReproductiveContactEvidenceError::UnexpectedOpportunity); }
        let status = derive_status(&records);
        let study = Self {
            study_version: REPRODUCTIVE_CONTACT_STUDY_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            records,
            status,
        };
        study.validate_local()?;
        Ok(study)
    }

    pub fn design(&self) -> &ReproductiveContactStudyDesign { &self.design }
    pub fn design_digest(&self) -> ReproductiveContactStudyDesignDigest { self.design_digest }

    pub fn canonical_digest(&self) -> Result<ReproductiveContactStudyDigest, ReproductiveContactEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(STUDY_DOMAIN);
        put_u32(&mut digest, self.study_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records { record.update_digest(&mut digest); }
        digest.update([self.status.tag()]);
        Ok(ReproductiveContactStudyDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ReproductiveContactEvidenceError> {
        if self.study_version != REPRODUCTIVE_CONTACT_STUDY_VERSION {
            return Err(ReproductiveContactEvidenceError::UnsupportedVersion(self.study_version));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(ReproductiveContactEvidenceError::DesignBindingMismatch);
        }
        if self.records.len() != self.design.opportunities.len() {
            return Err(ReproductiveContactEvidenceError::IncompleteOpportunityCoverage);
        }
        for (declaration, record) in self.design.opportunities.iter().zip(&self.records) {
            if declaration.opportunity_id != record.opportunity_id {
                return Err(ReproductiveContactEvidenceError::NonCanonicalOpportunityOrder);
            }
            validate_record_against_design(&self.design, record)?;
        }
        if derive_status(&self.records) != self.status {
            return Err(ReproductiveContactEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductiveContactStudyDigest([u8; 32]);
impl ReproductiveContactStudyDigest { pub fn as_bytes(&self) -> &[u8; 32] { &self.0 } }
impl fmt::Debug for ReproductiveContactStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReproductiveContactStudyDigest(")?; fmt_hex(&self.0, f)?; write!(f, ")")
    }
}
impl fmt::Display for ReproductiveContactStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { fmt_hex(&self.0, f) }
}

#[derive(Debug)]
#[must_use = "validated contact study should gate reproductive-isolation evidence"]
pub struct ValidatedReproductiveContactStudy<'a> {
    study: &'a ReproductiveContactStudy,
    study_digest: ReproductiveContactStudyDigest,
    design_digest: ReproductiveContactStudyDesignDigest,
}
impl<'a> ValidatedReproductiveContactStudy<'a> {
    pub fn validate_current(
        study: &'a ReproductiveContactStudy,
        design: &ValidatedReproductiveContactStudyDesign<'_>,
        inputs: impl IntoIterator<Item = ReproductiveOpportunityEvidenceInput>,
    ) -> Result<Self, ReproductiveContactEvidenceError> {
        study.validate_local()?;
        let recomputed = ReproductiveContactStudy::capture(design, inputs)?;
        if recomputed != *study { return Err(ReproductiveContactEvidenceError::ReplayMismatch); }
        Ok(Self { study, study_digest: study.canonical_digest()?, design_digest: design.design_digest() })
    }
    pub fn study(&self) -> &'a ReproductiveContactStudy { self.study }
    pub fn study_digest(&self) -> ReproductiveContactStudyDigest { self.study_digest }
    pub fn design_digest(&self) -> ReproductiveContactStudyDesignDigest { self.design_digest }
}

fn validate_input_against_design(
    design: &ReproductiveContactStudyDesign,
    input: &ReproductiveOpportunityEvidenceInput,
) -> Result<(), ReproductiveContactEvidenceError> {
    let record = ReproductiveContactRecord::from_input(input.clone());
    validate_record_against_design(design, &record)
}

fn validate_record_against_design(
    design: &ReproductiveContactStudyDesign,
    record: &ReproductiveContactRecord,
) -> Result<(), ReproductiveContactEvidenceError> {
    if record.demography_accounting_authority != design.demography_accounting_authority {
        return Err(ReproductiveContactEvidenceError::DemographyAuthorityMismatch(record.opportunity_id.clone()));
    }
    match &record.realized_gene_flow {
        RealizedGeneFlowObservation::NoneObserved { materialization_authority, .. }
        | RealizedGeneFlowObservation::Realized { materialization_authority, .. }
            if materialization_authority == &design.gene_flow_materialization_authority => {}
        RealizedGeneFlowObservation::Unavailable { .. } => {}
        _ => return Err(ReproductiveContactEvidenceError::GeneFlowAuthorityMismatch(record.opportunity_id.clone())),
    }
    Ok(())
}

fn derive_status(records: &[ReproductiveContactRecord]) -> ReproductiveContactStudyStatus {
    if records.iter().any(|record| record.realized_gene_flow.is_realized()) {
        return ReproductiveContactStudyStatus::CrossLineageGeneFlowObserved;
    }
    if records.iter().any(|record| matches!(record.outcome, ReproductiveOpportunityOutcome::ViableFertileOffspring { .. })) {
        return ReproductiveContactStudyStatus::ViableFertileHybridObserved;
    }
    if records.iter().any(|record| matches!(record.outcome, ReproductiveOpportunityOutcome::ViableInfertileOffspring { .. })) {
        return ReproductiveContactStudyStatus::ViableInfertileHybridObserved;
    }
    if records.iter().any(|record| matches!(record.outcome, ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown { .. })) {
        return ReproductiveContactStudyStatus::CrossLineageReproductionObserved;
    }
    if records.iter().any(|record| record.outcome.is_unavailable() || record.realized_gene_flow.is_unavailable()) {
        return ReproductiveContactStudyStatus::InsufficientEvidence;
    }
    if records.iter().all(|record| record.outcome.is_no_contact()) {
        return ReproductiveContactStudyStatus::NoContactOpportunity;
    }
    ReproductiveContactStudyStatus::NoRealizedGeneFlowWithObservedOpportunity
}

fn update_authority_digest(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str()); put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum ReproductiveContactEvidenceError {
    Design(crate::ReproductiveContactDesignError), UnsupportedVersion(u32),
    DuplicateOpportunity(ReproductiveOpportunityId), MissingOpportunity(ReproductiveOpportunityId),
    UnexpectedOpportunity, IncompleteOpportunityCoverage, NonCanonicalOpportunityOrder,
    DemographyAuthorityMismatch(ReproductiveOpportunityId), GeneFlowAuthorityMismatch(ReproductiveOpportunityId),
    DesignBindingMismatch, StatusInvariant, ReplayMismatch,
}
impl From<crate::ReproductiveContactDesignError> for ReproductiveContactEvidenceError {
    fn from(value: crate::ReproductiveContactDesignError) -> Self { Self::Design(value) }
}
impl fmt::Display for ReproductiveContactEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "reproductive-contact design error: {error}"),
            Self::UnsupportedVersion(v) => write!(f, "unsupported reproductive-contact study version {v}"),
            Self::DuplicateOpportunity(id) => write!(f, "opportunity {} supplied more than once", id.as_str()),
            Self::MissingOpportunity(id) => write!(f, "preregistered opportunity {} is missing", id.as_str()),
            Self::UnexpectedOpportunity => write!(f, "an undeclared reproductive opportunity was supplied"),
            Self::IncompleteOpportunityCoverage => write!(f, "contact study requires exactly one outcome for every preregistered opportunity"),
            Self::NonCanonicalOpportunityOrder => write!(f, "contact records are not in canonical preregistered order"),
            Self::DemographyAuthorityMismatch(id) => write!(f, "opportunity {} changes the preregistered demography authority", id.as_str()),
            Self::GeneFlowAuthorityMismatch(id) => write!(f, "opportunity {} changes the preregistered gene-flow materialization authority", id.as_str()),
            Self::DesignBindingMismatch => write!(f, "contact study binds a different preregistered design"),
            Self::StatusInvariant => write!(f, "persisted contact-study status does not recompute from the complete opportunity ledger"),
            Self::ReplayMismatch => write!(f, "persisted contact study does not replay against current authorities"),
        }
    }
}
impl Error for ReproductiveContactEvidenceError {}
