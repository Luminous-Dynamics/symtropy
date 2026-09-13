use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, IsolationStudyUnitId,
    RealizedGeneFlowObservation, ReproductiveContactStudy, ReproductiveContactStudyDigest,
    ReproductiveIsolationDesign, ReproductiveIsolationDesignDigest, ReproductiveOpportunityOutcome,
    ValidatedReproductiveContactStudy, ValidatedReproductiveIsolationDesign,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const REPRODUCTIVE_ISOLATION_EVIDENCE_VERSION: u32 = 1;
const DOMAIN: &[u8] = b"symtropy:evolution:reproductive-isolation-evidence:v1\0";

#[derive(Debug)]
pub struct IsolationStudyEvidenceInput<'a, 'b> {
    pub unit_id: IsolationStudyUnitId,
    pub study: &'a ValidatedReproductiveContactStudy<'b>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsolationStudyRecord {
    pub unit_id: IsolationStudyUnitId,
    pub contact_study: ReproductiveContactStudy,
    pub contact_study_digest: ReproductiveContactStudyDigest,
}

impl IsolationStudyRecord {
    fn put(&self, digest: &mut Sha256) {
        put_text(digest, self.unit_id.as_str());
        digest.update(self.contact_study_digest.as_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReproductiveBarrierProfile {
    pub study_count: u64,
    pub total_opportunities: u64,
    pub studies_with_observed_contact: u64,
    pub barrier_supporting_studies: u64,
    pub observed_contact_opportunities: u64,
    pub no_contact_observations: u64,
    pub pairing_barriers: u64,
    pub mating_barriers: u64,
    pub conception_barriers: u64,
    pub hybrid_viability_barriers: u64,
    pub hybrid_fertility_barriers: u64,
    pub viable_fertile_hybrids: u64,
    pub viable_hybrids_fertility_unknown: u64,
    pub realized_gene_flow_observations: u64,
    pub unavailable_reproductive_observations: u64,
    pub unavailable_gene_flow_observations: u64,
}

impl ReproductiveBarrierProfile {
    fn put(self, digest: &mut Sha256) {
        for value in [
            self.study_count,
            self.total_opportunities,
            self.studies_with_observed_contact,
            self.barrier_supporting_studies,
            self.observed_contact_opportunities,
            self.no_contact_observations,
            self.pairing_barriers,
            self.mating_barriers,
            self.conception_barriers,
            self.hybrid_viability_barriers,
            self.hybrid_fertility_barriers,
            self.viable_fertile_hybrids,
            self.viable_hybrids_fertility_unknown,
            self.realized_gene_flow_observations,
            self.unavailable_reproductive_observations,
            self.unavailable_gene_flow_observations,
        ] {
            put_u64(digest, value);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductiveIsolationStatus {
    Supported,
    NotSupported,
    Contradicted,
    InsufficientEvidence,
}

impl ReproductiveIsolationStatus {
    fn tag(self) -> u8 {
        match self {
            Self::Supported => 0,
            Self::NotSupported => 1,
            Self::Contradicted => 2,
            Self::InsufficientEvidence => 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveIsolationEvidence {
    evidence_version: u32,
    design: ReproductiveIsolationDesign,
    design_digest: ReproductiveIsolationDesignDigest,
    pub studies: Vec<IsolationStudyRecord>,
    pub barrier_profile: ReproductiveBarrierProfile,
    pub status: ReproductiveIsolationStatus,
}

impl ReproductiveIsolationEvidence {
    pub fn capture<'a, 'b>(
        design: &ValidatedReproductiveIsolationDesign<'_>,
        studies: impl IntoIterator<Item = IsolationStudyEvidenceInput<'a, 'b>>,
    ) -> Result<Self, ReproductiveIsolationEvidenceError> {
        let mut by_id = BTreeMap::new();
        for input in studies {
            if by_id.insert(input.unit_id.clone(), input).is_some() {
                return Err(ReproductiveIsolationEvidenceError::DuplicateStudyUnit);
            }
        }
        if by_id.len() != design.design().studies.len() {
            return Err(ReproductiveIsolationEvidenceError::IncompleteStudyCoverage);
        }

        let mut records = Vec::with_capacity(by_id.len());
        let mut seen_studies = Vec::new();
        for declaration in &design.design().studies {
            let input = by_id
                .remove(&declaration.unit_id)
                .ok_or(ReproductiveIsolationEvidenceError::MissingStudyUnit)?;
            if input.study.design_digest() != declaration.contact_design_digest {
                return Err(ReproductiveIsolationEvidenceError::ContactDesignMismatch);
            }
            let study_digest = input.study.study_digest();
            if seen_studies.iter().any(|seen| *seen == study_digest) {
                return Err(ReproductiveIsolationEvidenceError::DuplicateContactStudyDigest);
            }
            seen_studies.push(study_digest);
            records.push(IsolationStudyRecord {
                unit_id: declaration.unit_id.clone(),
                contact_study: input.study.study().clone(),
                contact_study_digest: study_digest,
            });
        }
        if !by_id.is_empty() {
            return Err(ReproductiveIsolationEvidenceError::UnexpectedStudyUnit);
        }

        let barrier_profile = derive_barrier_profile(&records)?;
        let status = derive_status(design.design(), barrier_profile);
        let evidence = Self {
            evidence_version: REPRODUCTIVE_ISOLATION_EVIDENCE_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            studies: records,
            barrier_profile,
            status,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn design(&self) -> &ReproductiveIsolationDesign {
        &self.design
    }

    pub fn design_digest(&self) -> ReproductiveIsolationDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ReproductiveIsolationEvidenceDigest, ReproductiveIsolationEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.studies.len() as u64);
        for study in &self.studies {
            study.put(&mut digest);
        }
        self.barrier_profile.put(&mut digest);
        digest.update([self.status.tag()]);
        Ok(ReproductiveIsolationEvidenceDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ReproductiveIsolationEvidenceError> {
        if self.evidence_version != REPRODUCTIVE_ISOLATION_EVIDENCE_VERSION {
            return Err(ReproductiveIsolationEvidenceError::UnsupportedVersion(
                self.evidence_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(ReproductiveIsolationEvidenceError::DesignBindingMismatch);
        }
        if self.studies.len() != self.design.studies.len() {
            return Err(ReproductiveIsolationEvidenceError::IncompleteStudyCoverage);
        }

        let mut seen_studies = Vec::new();
        for (declaration, record) in self.design.studies.iter().zip(&self.studies) {
            if declaration.unit_id != record.unit_id {
                return Err(ReproductiveIsolationEvidenceError::NonCanonicalStudyOrder);
            }
            if record.contact_study.design_digest() != declaration.contact_design_digest {
                return Err(ReproductiveIsolationEvidenceError::ContactDesignMismatch);
            }
            let digest = record.contact_study.canonical_digest()?;
            if digest != record.contact_study_digest {
                return Err(ReproductiveIsolationEvidenceError::ContactStudyDigestMismatch);
            }
            if seen_studies.iter().any(|seen| *seen == digest) {
                return Err(ReproductiveIsolationEvidenceError::DuplicateContactStudyDigest);
            }
            seen_studies.push(digest);
        }

        let profile = derive_barrier_profile(&self.studies)?;
        if profile != self.barrier_profile {
            return Err(ReproductiveIsolationEvidenceError::BarrierProfileInvariant);
        }
        if derive_status(&self.design, profile) != self.status {
            return Err(ReproductiveIsolationEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductiveIsolationEvidenceDigest([u8; 32]);

impl ReproductiveIsolationEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ReproductiveIsolationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReproductiveIsolationEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ReproductiveIsolationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated reproductive-isolation evidence should gate any later species-model claim"]
pub struct ValidatedReproductiveIsolationEvidence<'a> {
    evidence: &'a ReproductiveIsolationEvidence,
    evidence_digest: ReproductiveIsolationEvidenceDigest,
    design_digest: ReproductiveIsolationDesignDigest,
}

impl<'a> ValidatedReproductiveIsolationEvidence<'a> {
    pub fn validate_current<'b, 'c>(
        evidence: &'a ReproductiveIsolationEvidence,
        design: &ValidatedReproductiveIsolationDesign<'_>,
        studies: impl IntoIterator<Item = IsolationStudyEvidenceInput<'b, 'c>>,
    ) -> Result<Self, ReproductiveIsolationEvidenceError> {
        evidence.validate_local()?;
        let recomputed = ReproductiveIsolationEvidence::capture(design, studies)?;
        if recomputed != *evidence {
            return Err(ReproductiveIsolationEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn evidence(&self) -> &'a ReproductiveIsolationEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> ReproductiveIsolationEvidenceDigest {
        self.evidence_digest
    }

    pub fn design_digest(&self) -> ReproductiveIsolationDesignDigest {
        self.design_digest
    }
}

fn derive_barrier_profile(
    studies: &[IsolationStudyRecord],
) -> Result<ReproductiveBarrierProfile, ReproductiveIsolationEvidenceError> {
    let mut profile = ReproductiveBarrierProfile {
        study_count: u64::try_from(studies.len())
            .map_err(|_| ReproductiveIsolationEvidenceError::ArithmeticOverflow)?,
        ..ReproductiveBarrierProfile::default()
    };

    for study in studies {
        let mut study_observed_contact = false;
        let mut study_barrier = false;
        let mut study_disqualifying = false;

        for record in &study.contact_study.records {
            profile.total_opportunities = profile
                .total_opportunities
                .checked_add(1)
                .ok_or(ReproductiveIsolationEvidenceError::ArithmeticOverflow)?;

            match &record.outcome {
                ReproductiveOpportunityOutcome::NoContact { .. } => {
                    profile.no_contact_observations = add_one(profile.no_contact_observations)?;
                }
                ReproductiveOpportunityOutcome::ContactNoPairing { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.pairing_barriers = add_one(profile.pairing_barriers)?;
                    study_observed_contact = true;
                    study_barrier = true;
                }
                ReproductiveOpportunityOutcome::PairedNoMating { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.mating_barriers = add_one(profile.mating_barriers)?;
                    study_observed_contact = true;
                    study_barrier = true;
                }
                ReproductiveOpportunityOutcome::MatingNoConception { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.conception_barriers = add_one(profile.conception_barriers)?;
                    study_observed_contact = true;
                    study_barrier = true;
                }
                ReproductiveOpportunityOutcome::ConceptionNoViableOffspring { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.hybrid_viability_barriers =
                        add_one(profile.hybrid_viability_barriers)?;
                    study_observed_contact = true;
                    study_barrier = true;
                }
                ReproductiveOpportunityOutcome::ViableInfertileOffspring { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.hybrid_fertility_barriers =
                        add_one(profile.hybrid_fertility_barriers)?;
                    study_observed_contact = true;
                    study_barrier = true;
                }
                ReproductiveOpportunityOutcome::ViableFertileOffspring { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.viable_fertile_hybrids = add_one(profile.viable_fertile_hybrids)?;
                    study_observed_contact = true;
                    study_disqualifying = true;
                }
                ReproductiveOpportunityOutcome::ViableOffspringFertilityUnknown { .. } => {
                    profile.observed_contact_opportunities =
                        add_one(profile.observed_contact_opportunities)?;
                    profile.viable_hybrids_fertility_unknown =
                        add_one(profile.viable_hybrids_fertility_unknown)?;
                    study_observed_contact = true;
                    study_disqualifying = true;
                }
                ReproductiveOpportunityOutcome::Unavailable { .. } => {
                    profile.unavailable_reproductive_observations =
                        add_one(profile.unavailable_reproductive_observations)?;
                    study_disqualifying = true;
                }
            }

            match &record.realized_gene_flow {
                RealizedGeneFlowObservation::Realized { .. } => {
                    profile.realized_gene_flow_observations =
                        add_one(profile.realized_gene_flow_observations)?;
                    study_disqualifying = true;
                }
                RealizedGeneFlowObservation::Unavailable { .. } => {
                    profile.unavailable_gene_flow_observations =
                        add_one(profile.unavailable_gene_flow_observations)?;
                    study_disqualifying = true;
                }
                RealizedGeneFlowObservation::NoneObserved { .. } => {}
            }
        }

        if study_observed_contact {
            profile.studies_with_observed_contact =
                add_one(profile.studies_with_observed_contact)?;
        }
        if study_barrier && !study_disqualifying {
            profile.barrier_supporting_studies =
                add_one(profile.barrier_supporting_studies)?;
        }
    }
    Ok(profile)
}

fn derive_status(
    design: &ReproductiveIsolationDesign,
    profile: ReproductiveBarrierProfile,
) -> ReproductiveIsolationStatus {
    if profile.realized_gene_flow_observations > 0 || profile.viable_fertile_hybrids > 0 {
        return ReproductiveIsolationStatus::Contradicted;
    }
    if profile.unavailable_reproductive_observations > 0
        || profile.unavailable_gene_flow_observations > 0
        || profile.viable_hybrids_fertility_unknown > 0
    {
        return ReproductiveIsolationStatus::InsufficientEvidence;
    }
    if profile.observed_contact_opportunities < design.minimum_observed_contact_opportunities {
        return ReproductiveIsolationStatus::InsufficientEvidence;
    }
    if profile.barrier_supporting_studies >= design.minimum_barrier_supporting_studies {
        ReproductiveIsolationStatus::Supported
    } else {
        ReproductiveIsolationStatus::NotSupported
    }
}

fn add_one(value: u64) -> Result<u64, ReproductiveIsolationEvidenceError> {
    value
        .checked_add(1)
        .ok_or(ReproductiveIsolationEvidenceError::ArithmeticOverflow)
}

#[derive(Debug)]
pub enum ReproductiveIsolationEvidenceError {
    Design(crate::ReproductiveIsolationDesignError),
    ContactEvidence(crate::ReproductiveContactEvidenceError),
    UnsupportedVersion(u32),
    DuplicateStudyUnit,
    MissingStudyUnit,
    UnexpectedStudyUnit,
    IncompleteStudyCoverage,
    ContactDesignMismatch,
    DuplicateContactStudyDigest,
    ContactStudyDigestMismatch,
    NonCanonicalStudyOrder,
    DesignBindingMismatch,
    BarrierProfileInvariant,
    StatusInvariant,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::ReproductiveIsolationDesignError> for ReproductiveIsolationEvidenceError {
    fn from(value: crate::ReproductiveIsolationDesignError) -> Self {
        Self::Design(value)
    }
}

impl From<crate::ReproductiveContactEvidenceError> for ReproductiveIsolationEvidenceError {
    fn from(value: crate::ReproductiveContactEvidenceError) -> Self {
        Self::ContactEvidence(value)
    }
}

impl fmt::Display for ReproductiveIsolationEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "reproductive-isolation design error: {error}"),
            Self::ContactEvidence(error) => write!(f, "reproductive-contact evidence error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported reproductive-isolation evidence version {version}")
            }
            Self::DuplicateStudyUnit => write!(f, "isolation study unit supplied more than once"),
            Self::MissingStudyUnit => write!(f, "a preregistered isolation study is missing"),
            Self::UnexpectedStudyUnit => write!(f, "an undeclared isolation study was supplied"),
            Self::IncompleteStudyCoverage => write!(
                f,
                "reproductive-isolation evidence requires every preregistered contact study"
            ),
            Self::ContactDesignMismatch => write!(
                f,
                "current 09A study does not match its preregistered contact-study design"
            ),
            Self::DuplicateContactStudyDigest => write!(
                f,
                "one 09A contact study cannot masquerade as multiple independent isolation studies"
            ),
            Self::ContactStudyDigestMismatch => write!(
                f,
                "persisted 09A study snapshot does not match its recorded canonical digest"
            ),
            Self::NonCanonicalStudyOrder => {
                write!(f, "reproductive-isolation studies are not in canonical unit order")
            }
            Self::DesignBindingMismatch => {
                write!(f, "reproductive-isolation evidence binds a different design")
            }
            Self::BarrierProfileInvariant => write!(
                f,
                "persisted reproductive-barrier profile does not recompute from complete 09A studies"
            ),
            Self::StatusInvariant => write!(
                f,
                "persisted reproductive-isolation status does not recompute from the barrier profile"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted reproductive-isolation evidence does not replay against current 09A studies"
            ),
            Self::ArithmeticOverflow => {
                write!(f, "reproductive-isolation evidence arithmetic overflowed")
            }
        }
    }
}

impl Error for ReproductiveIsolationEvidenceError {}
