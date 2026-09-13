use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AdaptationReplicationDesign,
    AdaptationReplicationDesignDigest, HeritableResponseStudyDesignDigest,
    HeritableResponseStudyDigest, HeritableResponseStudyStatus, ReplicationUnitId,
    ValidatedAdaptationReplicationDesign, ValidatedHeritableResponseStudy,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const ADAPTATION_EVIDENCE_VERSION: u32 = 1;
const EVIDENCE_DOMAIN: &[u8] = b"symtropy:evolution:adaptation-evidence:v1\0";

#[derive(Debug)]
pub struct ReplicationStudyInput<'a, 'b> {
    pub unit_id: ReplicationUnitId,
    pub study: &'a ValidatedHeritableResponseStudy<'b>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplicationResponseRecord {
    pub unit_id: ReplicationUnitId,
    pub study_design_digest: HeritableResponseStudyDesignDigest,
    pub study_digest: HeritableResponseStudyDigest,
    pub study_status: HeritableResponseStudyStatus,
}

impl ReplicationResponseRecord {
    fn update_digest(&self, digest: &mut Sha256) {
        put_text(digest, self.unit_id.as_str());
        digest.update(self.study_design_digest.as_bytes());
        digest.update(self.study_digest.as_bytes());
        digest.update([study_status_tag(self.study_status)]);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdaptationEvidenceStatus {
    Supported,
    NotSupported,
    Contradicted,
    InsufficientEvidence,
}

impl AdaptationEvidenceStatus {
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
pub struct AdaptationEvidence {
    evidence_version: u32,
    design: AdaptationReplicationDesign,
    design_digest: AdaptationReplicationDesignDigest,
    pub records: Vec<ReplicationResponseRecord>,
    pub status: AdaptationEvidenceStatus,
}

impl AdaptationEvidence {
    pub fn capture<'a, 'b>(
        design: &ValidatedAdaptationReplicationDesign<'_>,
        studies: impl IntoIterator<Item = ReplicationStudyInput<'a, 'b>>,
    ) -> Result<Self, AdaptationEvidenceError> {
        let mut by_id = BTreeMap::new();
        for input in studies {
            if by_id.insert(input.unit_id.clone(), input).is_some() {
                return Err(AdaptationEvidenceError::DuplicateReplicationUnit);
            }
        }
        if by_id.len() != design.design().units.len() {
            return Err(AdaptationEvidenceError::IncompleteReplicationCoverage);
        }

        let mut records = Vec::with_capacity(design.design().units.len());
        let mut seen_studies = Vec::new();
        for declaration in &design.design().units {
            let input = by_id
                .remove(&declaration.unit_id)
                .ok_or(AdaptationEvidenceError::MissingReplicationUnit)?;
            if input.study.design_digest() != declaration.study_design_digest {
                return Err(AdaptationEvidenceError::StudyDesignMismatch);
            }
            let study_digest = input.study.study_digest();
            if seen_studies.iter().any(|seen| *seen == study_digest) {
                return Err(AdaptationEvidenceError::DuplicateStudyDigest);
            }
            seen_studies.push(study_digest);
            records.push(ReplicationResponseRecord {
                unit_id: declaration.unit_id.clone(),
                study_design_digest: input.study.design_digest(),
                study_digest,
                study_status: input.study.study().status,
            });
        }
        if !by_id.is_empty() {
            return Err(AdaptationEvidenceError::UnexpectedReplicationUnit);
        }

        let status = derive_adaptation_status(design.design(), &records)?;
        let evidence = Self {
            evidence_version: ADAPTATION_EVIDENCE_VERSION,
            design: design.design().clone(),
            design_digest: design.design_digest(),
            records,
            status,
        };
        evidence.validate_local()?;
        Ok(evidence)
    }

    pub fn design(&self) -> &AdaptationReplicationDesign {
        &self.design
    }

    pub fn design_digest(&self) -> AdaptationReplicationDesignDigest {
        self.design_digest
    }

    pub fn canonical_digest(&self) -> Result<AdaptationEvidenceDigest, AdaptationEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(EVIDENCE_DOMAIN);
        put_u32(&mut digest, self.evidence_version);
        digest.update(self.design_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records {
            record.update_digest(&mut digest);
        }
        digest.update([self.status.tag()]);
        Ok(AdaptationEvidenceDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), AdaptationEvidenceError> {
        if self.evidence_version != ADAPTATION_EVIDENCE_VERSION {
            return Err(AdaptationEvidenceError::UnsupportedVersion(
                self.evidence_version,
            ));
        }
        if self.design.canonical_digest()? != self.design_digest {
            return Err(AdaptationEvidenceError::DesignBindingMismatch);
        }
        if self.records.len() != self.design.units.len() {
            return Err(AdaptationEvidenceError::IncompleteReplicationCoverage);
        }
        let mut seen_studies = Vec::new();
        for (declaration, record) in self.design.units.iter().zip(&self.records) {
            if declaration.unit_id != record.unit_id {
                return Err(AdaptationEvidenceError::NonCanonicalReplicationOrder);
            }
            if declaration.study_design_digest != record.study_design_digest {
                return Err(AdaptationEvidenceError::StudyDesignMismatch);
            }
            if seen_studies.iter().any(|seen| *seen == record.study_digest) {
                return Err(AdaptationEvidenceError::DuplicateStudyDigest);
            }
            seen_studies.push(record.study_digest);
        }
        if derive_adaptation_status(&self.design, &self.records)? != self.status {
            return Err(AdaptationEvidenceError::StatusInvariant);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AdaptationEvidenceDigest([u8; 32]);

impl AdaptationEvidenceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AdaptationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AdaptationEvidenceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for AdaptationEvidenceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated adaptation evidence should gate any downstream evolutionary claim"]
pub struct ValidatedAdaptationEvidence<'a> {
    evidence: &'a AdaptationEvidence,
    evidence_digest: AdaptationEvidenceDigest,
    design_digest: AdaptationReplicationDesignDigest,
}

impl<'a> ValidatedAdaptationEvidence<'a> {
    pub fn validate_current<'b, 'c>(
        evidence: &'a AdaptationEvidence,
        design: &ValidatedAdaptationReplicationDesign<'_>,
        studies: impl IntoIterator<Item = ReplicationStudyInput<'b, 'c>>,
    ) -> Result<Self, AdaptationEvidenceError> {
        evidence.validate_local()?;
        let recomputed = AdaptationEvidence::capture(design, studies)?;
        if recomputed != *evidence {
            return Err(AdaptationEvidenceError::ReplayMismatch);
        }
        Ok(Self {
            evidence,
            evidence_digest: evidence.canonical_digest()?,
            design_digest: design.design_digest(),
        })
    }

    pub fn evidence(&self) -> &'a AdaptationEvidence {
        self.evidence
    }

    pub fn evidence_digest(&self) -> AdaptationEvidenceDigest {
        self.evidence_digest
    }

    pub fn design_digest(&self) -> AdaptationReplicationDesignDigest {
        self.design_digest
    }
}

fn derive_adaptation_status(
    design: &AdaptationReplicationDesign,
    records: &[ReplicationResponseRecord],
) -> Result<AdaptationEvidenceStatus, AdaptationEvidenceError> {
    if records.len() != design.units.len() {
        return Err(AdaptationEvidenceError::IncompleteReplicationCoverage);
    }
    if records
        .iter()
        .any(|record| record.study_status == HeritableResponseStudyStatus::ReversedResponse)
    {
        return Ok(AdaptationEvidenceStatus::Contradicted);
    }
    if records
        .iter()
        .any(|record| record.study_status == HeritableResponseStudyStatus::InsufficientEvidence)
    {
        return Ok(AdaptationEvidenceStatus::InsufficientEvidence);
    }
    let supported = records
        .iter()
        .filter(|record| {
            record.study_status == HeritableResponseStudyStatus::DirectionalResponseObserved
        })
        .count();
    let supported = u64::try_from(supported).map_err(|_| AdaptationEvidenceError::ArithmeticOverflow)?;
    if supported >= design.minimum_supported_replicates {
        Ok(AdaptationEvidenceStatus::Supported)
    } else {
        Ok(AdaptationEvidenceStatus::NotSupported)
    }
}

fn study_status_tag(status: HeritableResponseStudyStatus) -> u8 {
    match status {
        HeritableResponseStudyStatus::DirectionalResponseObserved => 0,
        HeritableResponseStudyStatus::NullResponse => 1,
        HeritableResponseStudyStatus::ReversedResponse => 2,
        HeritableResponseStudyStatus::HereditaryOnlyMismatch => 3,
        HeritableResponseStudyStatus::TraitOnlyMismatch => 4,
        HeritableResponseStudyStatus::InsufficientEvidence => 5,
    }
}

#[derive(Debug)]
pub enum AdaptationEvidenceError {
    Design(crate::AdaptationReplicationDesignError),
    UnsupportedVersion(u32),
    DuplicateReplicationUnit,
    MissingReplicationUnit,
    UnexpectedReplicationUnit,
    IncompleteReplicationCoverage,
    StudyDesignMismatch,
    DuplicateStudyDigest,
    NonCanonicalReplicationOrder,
    DesignBindingMismatch,
    StatusInvariant,
    ReplayMismatch,
    ArithmeticOverflow,
}

impl From<crate::AdaptationReplicationDesignError> for AdaptationEvidenceError {
    fn from(value: crate::AdaptationReplicationDesignError) -> Self {
        Self::Design(value)
    }
}

impl fmt::Display for AdaptationEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Design(error) => write!(f, "adaptation-replication design error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported adaptation-evidence version {version}")
            }
            Self::DuplicateReplicationUnit => write!(f, "replication unit supplied more than once"),
            Self::MissingReplicationUnit => write!(f, "a preregistered replication unit is missing"),
            Self::UnexpectedReplicationUnit => write!(f, "an undeclared replication unit was supplied"),
            Self::IncompleteReplicationCoverage => write!(
                f,
                "adaptation evidence requires one record for every preregistered replication unit"
            ),
            Self::StudyDesignMismatch => write!(
                f,
                "current B1 study does not match the preregistered B1 study-design digest"
            ),
            Self::DuplicateStudyDigest => write!(
                f,
                "one B1 study result cannot masquerade as multiple independent replications"
            ),
            Self::NonCanonicalReplicationOrder => {
                write!(f, "replication evidence is not in canonical preregistered unit order")
            }
            Self::DesignBindingMismatch => {
                write!(f, "adaptation evidence binds a different replication design")
            }
            Self::StatusInvariant => write!(
                f,
                "persisted adaptation status does not recompute from complete replication evidence"
            ),
            Self::ReplayMismatch => write!(
                f,
                "persisted adaptation evidence does not replay against current B1 studies"
            ),
            Self::ArithmeticOverflow => write!(f, "adaptation-evidence arithmetic overflowed"),
        }
    }
}

impl Error for AdaptationEvidenceError {}
