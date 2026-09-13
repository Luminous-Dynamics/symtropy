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
const DOMAIN: &[u8] = b"symtropy:evolution:reproductive-contact-study:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveObservationStage {
    Contact,
    Pairing,
    Mating,
    Conception,
    OffspringViability,
    OffspringFertility,
}

impl ReproductiveObservationStage {
    fn tag(self) -> u8 {
        match self {
            Self::Contact => 0,
            Self::Pairing => 1,
            Self::Mating => 2,
            Self::Conception => 3,
            Self::OffspringViability => 4,
            Self::OffspringFertility => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveStageEvidence {
    pub protocol: AnalysisAuthorityRef,
    pub evidence: AnalysisAuthorityRef,
}

impl ReproductiveStageEvidence {
    pub fn new(protocol: AnalysisAuthorityRef, evidence: AnalysisAuthorityRef) -> Self {
        Self { protocol, evidence }
    }

    fn put(&self, digest: &mut Sha256) {
        put_authority(digest, &self.protocol);
        put_authority(digest, &self.evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservedOffspringEvidence {
    pub event_id: ReproductionEventId,
    pub reproduction_provenance_digest: ReproductionProvenanceDigest,
    pub parentage_authority: AnalysisAuthorityRef,
    pub parentage_evidence: AnalysisAuthorityRef,
}

impl ObservedOffspringEvidence {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.event_id.as_str());
        digest.update(self.reproduction_provenance_digest.as_bytes());
        put_authority(digest, &self.parentage_authority);
        put_authority(digest, &self.parentage_evidence);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveOpportunityOutcome {
    NoContact {
        contact: ReproductiveStageEvidence,
    },
    ContactNoPairing {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
    },
    PairedNoMating {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
    },
    MatingNoConception {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
        conception: ReproductiveStageEvidence,
    },
    ConceptionNoViableOffspring {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
        conception: ReproductiveStageEvidence,
        viability: ReproductiveStageEvidence,
    },
    ViableOffspringFertilityUnknown {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
        conception: ReproductiveStageEvidence,
        viability: ReproductiveStageEvidence,
        offspring: ObservedOffspringEvidence,
    },
    ViableInfertileOffspring {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
        conception: ReproductiveStageEvidence,
        viability: ReproductiveStageEvidence,
        offspring: ObservedOffspringEvidence,
        fertility: ReproductiveStageEvidence,
    },
    ViableFertileOffspring {
        contact: ReproductiveStageEvidence,
        pairing: ReproductiveStageEvidence,
        mating: ReproductiveStageEvidence,
        conception: ReproductiveStageEvidence,
        viability: ReproductiveStageEvidence,
        offspring: ObservedOffspringEvidence,
        fertility: ReproductiveStageEvidence,
    },
    Unavailable {
        stage: ReproductiveObservationStage,
        missing_data_authority: AnalysisAuthorityRef,
        reason: AnalysisAuthorityRef,
    },
}

impl ReproductiveOpportunityOutcome {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::NoContact { contact } => {
                digest.update([0]);
                contact.put(digest);
            }
            Self::ContactNoPairing { contact, pairing } => {
                digest.update([1]);
                contact.put(digest);
                pairing.put(digest);
            }
            Self::PairedNoMating {
                contact,
                pairing,
                mating,
            } => {
                digest.update([2]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
            }
            Self::MatingNoConception {
                contact,
                pairing,
                mating,
                conception,
            } => {
                digest.update([3]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
                conception.put(digest);
            }
            Self::ConceptionNoViableOffspring {
                contact,
                pairing,
                mating,
                conception,
                viability,
            } => {
                digest.update([4]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
                conception.put(digest);
                viability.put(digest);
            }
            Self::ViableOffspringFertilityUnknown {
                contact,
                pairing,
                mating,
                conception,
                viability,
                offspring,
            } => {
                digest.update([5]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
                conception.put(digest);
                viability.put(digest);
                offspring.put(digest);
            }
            Self::ViableInfertileOffspring {
                contact,
                pairing,
                mating,
                conception,
                viability,
                offspring,
                fertility,
            } => {
                digest.update([6]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
                conception.put(digest);
                viability.put(digest);
                offspring.put(digest);
                fertility.put(digest);
            }
            Self::ViableFertileOffspring {
                contact,
                pairing,
                mating,
                conception,
                viability,
                offspring,
                fertility,
            } => {
                digest.update([7]);
                contact.put(digest);
                pairing.put(digest);
                mating.put(digest);
                conception.put(digest);
                viability.put(digest);
                offspring.put(digest);
                fertility.put(digest);
            }
            Self::Unavailable {
                stage,
                missing_data_authority,
                reason,
            } => {
                digest.update([8, stage.tag()]);
                put_authority(digest, missing_data_authority);
                put_authority(digest, reason);
            }
        }
    }

    fn unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }

    fn no_contact(&self) -> bool {
        matches!(self, Self::NoContact { .. })
    }

    fn offspring(&self) -> Option<&ObservedOffspringEvidence> {
        match self {
            Self::ViableOffspringFertilityUnknown { offspring, .. }
            | Self::ViableInfertileOffspring { offspring, .. }
            | Self::ViableFertileOffspring { offspring, .. } => Some(offspring),
            _ => None,
        }
    }

    fn missing_authority(&self) -> Option<&AnalysisAuthorityRef> {
        match self {
            Self::Unavailable {
                missing_data_authority,
                ..
            } => Some(missing_data_authority),
            _ => None,
        }
    }
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
    Unavailable {
        missing_data_authority: AnalysisAuthorityRef,
        reason: AnalysisAuthorityRef,
    },
}

impl RealizedGeneFlowObservation {
    fn put(&self, digest: &mut Sha256) {
        match self {
            Self::NoneObserved {
                materialization_authority,
                evidence,
            } => {
                digest.update([0]);
                put_authority(digest, materialization_authority);
                put_authority(digest, evidence);
            }
            Self::Realized {
                materialization_authority,
                ancestry_evidence,
            } => {
                digest.update([1]);
                put_authority(digest, materialization_authority);
                put_authority(digest, ancestry_evidence);
            }
            Self::Unavailable {
                missing_data_authority,
                reason,
            } => {
                digest.update([2]);
                put_authority(digest, missing_data_authority);
                put_authority(digest, reason);
            }
        }
    }

    fn realized(&self) -> bool {
        matches!(self, Self::Realized { .. })
    }

    fn unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }
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

    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.opportunity_id.as_str());
        self.outcome.put(digest);
        self.realized_gene_flow.put(digest);
        put_authority(digest, &self.demography_accounting_authority);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveContactStudyStatus {
    CrossLineageGeneFlowObserved,
    ViableFertileHybridObserved,
    ViableInfertileHybridObserved,
    CrossLineageReproductionObserved,
    NoRealizedGeneFlowWithObservedOpportunity,
    NoContactObserved,
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
            Self::NoContactObserved => 5,
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

        let mut records = Vec::with_capacity(by_id.len());
        for declaration in &design.design().opportunities {
            let input = by_id
                .remove(&declaration.opportunity_id)
                .ok_or_else(|| {
                    ReproductiveContactEvidenceError::MissingOpportunity(
                        declaration.opportunity_id.clone(),
                    )
                })?;
            let record = ReproductiveContactRecord::from_input(input);
            validate_record(design.design(), &record)?;
            records.push(record);
        }
        if !by_id.is_empty() {
            return Err(ReproductiveContactEvidenceError::UnexpectedOpportunity);
        }

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

    pub fn design(&self) -> &ReproductiveContactStudyDesign {
        &self.design
    }

    pub fn design_digest(&self) -> ReproductiveContactStudyDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ReproductiveContactStudyDigest, ReproductiveContactEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.study_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records {
            record.put(&mut digest);
        }
        digest.update([self.status.tag()]);
        Ok(ReproductiveContactStudyDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ReproductiveContactEvidenceError> {
        if self.study_version != REPRODUCTIVE_CONTACT_STUDY_VERSION {
            return Err(ReproductiveContactEvidenceError::UnsupportedVersion(
                self.study_version,
            ));
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
            validate_record(&self.design, record)?;
        }
        if derive_status(&self.records) != self.status {
            return Err(ReproductiveContactEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductiveContactStudyDigest([u8; 32]);

impl ReproductiveContactStudyDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ReproductiveContactStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReproductiveContactStudyDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ReproductiveContactStudyDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
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
        if recomputed != *study {
            return Err(ReproductiveContactEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            study,
            study_digest: study.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn study(&self) -> &'a ReproductiveContactStudy {
        self.study
    }

    pub fn study_digest(&self) -> ReproductiveContactStudyDigest {
        self.study_digest
    }

    pub fn design_digest(&self) -> ReproductiveContactStudyDesignDigest {
        self.design_digest
    }
}

fn validate_stage(
    design: &ReproductiveContactStudyDesign,
    stage: ReproductiveObservationStage,
    evidence: &ReproductiveStageEvidence,
    opportunity_id: &ReproductiveOpportunityId,
) -> Result<(), ReproductiveContactEvidenceError> {
    let expected = match stage {
        ReproductiveObservationStage::Contact => &design.contact_protocol,
        ReproductiveObservationStage::Pairing => &design.pairing_protocol,
        ReproductiveObservationStage::Mating => &design.mating_protocol,
        ReproductiveObservationStage::Conception => &design.conception_protocol,
        ReproductiveObservationStage::OffspringViability => &design.offspring_viability_protocol,
        ReproductiveObservationStage::OffspringFertility => &design.offspring_fertility_protocol,
    };
    if &evidence.protocol != expected {
        return Err(ReproductiveContactEvidenceError::StageProtocolMismatch {
            opportunity_id: opportunity_id.clone(),
            stage,
        });
    }
    Ok(())
}

fn validate_outcome(
    design: &ReproductiveContactStudyDesign,
    opportunity_id: &ReproductiveOpportunityId,
    outcome: &ReproductiveOpportunityOutcome,
) -> Result<(), ReproductiveContactEvidenceError> {
    let contact = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::Contact,
            evidence,
            opportunity_id,
        )
    };
    let pairing = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::Pairing,
            evidence,
            opportunity_id,
        )
    };
    let mating = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::Mating,
            evidence,
            opportunity_id,
        )
    };
    let conception = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::Conception,
            evidence,
            opportunity_id,
        )
    };
    let viability = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::OffspringViability,
            evidence,
            opportunity_id,
        )
    };
    let fertility = |evidence: &ReproductiveStageEvidence| {
        validate_stage(
            design,
            ReproductiveObservationStage::OffspringFertility,
            evidence,
            opportunity_id,
        )
    };

    match outcome {
        ReproductiveOpportunityOutcome::NoContact { contact: contact_evidence } => {
            contact(contact_evidence)?;
        }
        ReproductiveOpportunityOutcome::ContactNoPairing {
            contact: contact_evidence,
            pairing: pairing_evidence,
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
        }
        ReproductiveOpportunityOutcome::PairedNoMating {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
            mating(mating_evidence)?;
        }
        ReproductiveOpportunityOutcome::MatingNoConception {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
            conception: conception_evidence,
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
            mating(mating_evidence)?;
            conception(conception_evidence)?;
        }
        ReproductiveOpportunityOutcome::ConceptionNoViableOffspring {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
            conception: conception_evidence,
            viability: viability_evidence,
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
            mating(mating_evidence)?;
            conception(conception_evidence)?;
            viability(viability_evidence)?;
        }
        ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
            conception: conception_evidence,
            viability: viability_evidence,
            ..
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
            mating(mating_evidence)?;
            conception(conception_evidence)?;
            viability(viability_evidence)?;
        }
        ReproductiveOpportunityOutcome::ViableInfertileOffspring {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
            conception: conception_evidence,
            viability: viability_evidence,
            fertility: fertility_evidence,
            ..
        }
        | ReproductiveOpportunityOutcome::ViableFertileOffspring {
            contact: contact_evidence,
            pairing: pairing_evidence,
            mating: mating_evidence,
            conception: conception_evidence,
            viability: viability_evidence,
            fertility: fertility_evidence,
            ..
        } => {
            contact(contact_evidence)?;
            pairing(pairing_evidence)?;
            mating(mating_evidence)?;
            conception(conception_evidence)?;
            viability(viability_evidence)?;
            fertility(fertility_evidence)?;
        }
        ReproductiveOpportunityOutcome::Unavailable { .. } => {}
    }
    Ok(())
}

fn validate_record(
    design: &ReproductiveContactStudyDesign,
    record: &ReproductiveContactRecord,
) -> Result<(), ReproductiveContactEvidenceError> {
    if record.demography_accounting_authority != design.demography_accounting_authority {
        return Err(ReproductiveContactEvidenceError::DemographyAuthorityMismatch(
            record.opportunity_id.clone(),
        ));
    }
    validate_outcome(design, &record.opportunity_id, &record.outcome)?;
    if let Some(parentage) = record.outcome.offspring().map(|offspring| &offspring.parentage_authority)
    {
        if parentage != &design.parentage_authority {
            return Err(ReproductiveContactEvidenceError::ParentageAuthorityMismatch(
                record.opportunity_id.clone(),
            ));
        }
    }
    if let Some(missing) = record.outcome.missing_authority() {
        if missing != &design.missing_data_authority {
            return Err(ReproductiveContactEvidenceError::MissingDataAuthorityMismatch(
                record.opportunity_id.clone(),
            ));
        }
    }
    match &record.realized_gene_flow {
        RealizedGeneFlowObservation::NoneObserved {
            materialization_authority,
            ..
        }
        | RealizedGeneFlowObservation::Realized {
            materialization_authority,
            ..
        } if materialization_authority == &design.gene_flow_materialization_authority => {}
        RealizedGeneFlowObservation::Unavailable {
            missing_data_authority,
            ..
        } if missing_data_authority == &design.missing_data_authority => {}
        RealizedGeneFlowObservation::Unavailable { .. } => {
            return Err(ReproductiveContactEvidenceError::MissingDataAuthorityMismatch(
                record.opportunity_id.clone(),
            ));
        }
        _ => {
            return Err(ReproductiveContactEvidenceError::GeneFlowAuthorityMismatch(
                record.opportunity_id.clone(),
            ));
        }
    }
    Ok(())
}

fn derive_status(records: &[ReproductiveContactRecord]) -> ReproductiveContactStudyStatus {
    if records.iter().any(|record| record.realized_gene_flow.realized()) {
        return ReproductiveContactStudyStatus::CrossLineageGeneFlowObserved;
    }
    if records.iter().any(|record| {
        matches!(
            &record.outcome,
            ReproductiveOpportunityOutcome::ViableFertileOffspring { .. }
        )
    }) {
        return ReproductiveContactStudyStatus::ViableFertileHybridObserved;
    }
    if records.iter().any(|record| {
        matches!(
            &record.outcome,
            ReproductiveOpportunityOutcome::ViableInfertileOffspring { .. }
        )
    }) {
        return ReproductiveContactStudyStatus::ViableInfertileHybridObserved;
    }
    if records.iter().any(|record| {
        matches!(
            &record.outcome,
            ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown { .. }
        )
    }) {
        return ReproductiveContactStudyStatus::CrossLineageReproductionObserved;
    }
    if records
        .iter()
        .any(|record| record.outcome.unavailable() || record.realized_gene_flow.unavailable())
    {
        return ReproductiveContactStudyStatus::InsufficientEvidence;
    }
    if records.iter().all(|record| record.outcome.no_contact()) {
        return ReproductiveContactStudyStatus::NoContactObserved;
    }
    ReproductiveContactStudyStatus::NoRealizedGeneFlowWithObservedOpportunity
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

#[derive(Debug)]
pub enum ReproductiveContactEvidenceError {
    Design(crate::ReproductiveContactDesignError),
    UnsupportedVersion(u32),
    DuplicateOpportunity(ReproductiveOpportunityId),
    MissingOpportunity(ReproductiveOpportunityId),
    UnexpectedOpportunity,
    IncompleteOpportunityCoverage,
    NonCanonicalOpportunityOrder,
    StageProtocolMismatch {
        opportunity_id: ReproductiveOpportunityId,
        stage: ReproductiveObservationStage,
    },
    DemographyAuthorityMismatch(ReproductiveOpportunityId),
    ParentageAuthorityMismatch(ReproductiveOpportunityId),
    GeneFlowAuthorityMismatch(ReproductiveOpportunityId),
    MissingDataAuthorityMismatch(ReproductiveOpportunityId),
    DesignBindingMismatch,
    StatusInvariant,
    ReplayMismatch,
}

impl From<crate::ReproductiveContactDesignError> for ReproductiveContactEvidenceError {
    fn from(value: crate::ReproductiveContactDesignError) -> Self {
        Self::Design(value)
    }
}

impl fmt::Display for ReproductiveContactEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "reproductive-contact design error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported reproductive-contact study version {version}")
            }
            Self::DuplicateOpportunity(id) => {
                write!(f, "opportunity {} supplied more than once", id.as_str())
            }
            Self::MissingOpportunity(id) => {
                write!(f, "preregistered opportunity {} is missing", id.as_str())
            }
            Self::UnexpectedOpportunity => write!(f, "an undeclared reproductive opportunity was supplied"),
            Self::IncompleteOpportunityCoverage => write!(
                f,
                "contact study requires exactly one outcome for every preregistered opportunity"
            ),
            Self::NonCanonicalOpportunityOrder => {
                write!(f, "contact records are not in canonical preregistered order")
            }
            Self::StageProtocolMismatch {
                opportunity_id,
                stage,
            } => write!(
                f,
                "opportunity {} uses a non-preregistered {:?} protocol",
                opportunity_id.as_str(),
                stage
            ),
            Self::DemographyAuthorityMismatch(id) => write!(
                f,
                "opportunity {} changes the preregistered demography authority",
                id.as_str()
            ),
            Self::ParentageAuthorityMismatch(id) => write!(
                f,
                "opportunity {} changes the preregistered parentage authority",
                id.as_str()
            ),
            Self::GeneFlowAuthorityMismatch(id) => write!(
                f,
                "opportunity {} changes the preregistered gene-flow materialization authority",
                id.as_str()
            ),
            Self::MissingDataAuthorityMismatch(id) => write!(
                f,
                "opportunity {} changes the preregistered missing-data authority",
                id.as_str()
            ),
            Self::DesignBindingMismatch => {
                write!(f, "contact study binds a different preregistered design")
            }
            Self::StatusInvariant => write!(
                f,
                "persisted contact-study status does not recompute from the complete opportunity ledger"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted contact study does not replay against current authorities"
            ),
        }
    }
}

impl Error for ReproductiveContactEvidenceError {}
