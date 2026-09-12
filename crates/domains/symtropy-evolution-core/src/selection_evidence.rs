use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, ChromosomeMap, ConsequenceError,
    EvidenceProtocolId, EvolutionError, EvolutionIndividualId, EvolutionaryContextRefDigest,
    ExplicitConsequenceLedgerDigest, ExplicitLinkedPopulationCensus,
    ExplicitLinkedPopulationCensusDigest, ExposureEvidenceSourceId, HereditarySchema,
    IndividualConsequenceObservationDigest, LinkedIndividualError, LinkedIndividualManifestDigest,
    LinkedIndividualSubject, PhenotypeEvidenceSourceId, PopulationId, ValidatedConsequenceLedger,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const PHENOTYPE_EVIDENCE_REF_VERSION: u32 = 1;
pub const EXPOSURE_EVIDENCE_REF_VERSION: u32 = 1;
pub const SELECTION_EVIDENCE_RECORD_VERSION: u32 = 1;
pub const EXPLICIT_SELECTION_EVIDENCE_LEDGER_VERSION: u32 = 1;

const PHENOTYPE_REF_DOMAIN: &[u8] = b"symtropy:evolution:phenotype-evidence-ref:v1\0";
const EXPOSURE_REF_DOMAIN: &[u8] = b"symtropy:evolution:exposure-evidence-ref:v1\0";
const SELECTION_EVIDENCE_LEDGER_DOMAIN: &[u8] =
    b"symtropy:evolution:explicit-selection-evidence-ledger:v1\0";

macro_rules! opaque_digest {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name([u8; 32]);

        impl $name {
            pub fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_hex(&self.0, f)?;
                write!(f, ")")
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_hex(&self.0, f)
            }
        }
    };
}

opaque_digest!(EvidenceContentDigest);
opaque_digest!(EvidenceProtocolContentDigest);
opaque_digest!(ObservationSupportDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhenotypeEvidenceRef {
    evidence_version: u32,
    pub individual_id: EvolutionIndividualId,
    pub source_id: PhenotypeEvidenceSourceId,
    pub revision: u64,
    pub content_digest: EvidenceContentDigest,
    pub protocol_id: EvidenceProtocolId,
    pub protocol_content_digest: EvidenceProtocolContentDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl PhenotypeEvidenceRef {
    pub fn new(
        individual_id: EvolutionIndividualId,
        source_id: PhenotypeEvidenceSourceId,
        revision: u64,
        content_digest: EvidenceContentDigest,
        protocol_id: EvidenceProtocolId,
        protocol_content_digest: EvidenceProtocolContentDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Self {
        Self {
            evidence_version: PHENOTYPE_EVIDENCE_REF_VERSION,
            individual_id,
            source_id,
            revision,
            content_digest,
            protocol_id,
            protocol_content_digest,
            context_digest,
        }
    }

    fn validate_for(
        &self,
        individual_id: &EvolutionIndividualId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionEvidenceError> {
        if self.evidence_version != PHENOTYPE_EVIDENCE_REF_VERSION {
            return Err(SelectionEvidenceError::UnsupportedPhenotypeEvidenceVersion(
                self.evidence_version,
            ));
        }
        if &self.individual_id != individual_id {
            return Err(SelectionEvidenceError::EvidenceIndividualMismatch);
        }
        if self.context_digest != context_digest {
            return Err(SelectionEvidenceError::EvidenceContextMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        digest.update(PHENOTYPE_REF_DOMAIN);
        put_u32(digest, self.evidence_version);
        put_text(digest, self.individual_id.as_str());
        put_text(digest, self.source_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        put_text(digest, self.protocol_id.as_str());
        digest.update(self.protocol_content_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExposureEvidenceRef {
    evidence_version: u32,
    pub individual_id: EvolutionIndividualId,
    pub source_id: ExposureEvidenceSourceId,
    pub revision: u64,
    pub content_digest: EvidenceContentDigest,
    pub protocol_id: EvidenceProtocolId,
    pub protocol_content_digest: EvidenceProtocolContentDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl ExposureEvidenceRef {
    pub fn new(
        individual_id: EvolutionIndividualId,
        source_id: ExposureEvidenceSourceId,
        revision: u64,
        content_digest: EvidenceContentDigest,
        protocol_id: EvidenceProtocolId,
        protocol_content_digest: EvidenceProtocolContentDigest,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Self {
        Self {
            evidence_version: EXPOSURE_EVIDENCE_REF_VERSION,
            individual_id,
            source_id,
            revision,
            content_digest,
            protocol_id,
            protocol_content_digest,
            context_digest,
        }
    }

    fn validate_for(
        &self,
        individual_id: &EvolutionIndividualId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionEvidenceError> {
        if self.evidence_version != EXPOSURE_EVIDENCE_REF_VERSION {
            return Err(SelectionEvidenceError::UnsupportedExposureEvidenceVersion(
                self.evidence_version,
            ));
        }
        if &self.individual_id != individual_id {
            return Err(SelectionEvidenceError::EvidenceIndividualMismatch);
        }
        if self.context_digest != context_digest {
            return Err(SelectionEvidenceError::EvidenceContextMismatch);
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        digest.update(EXPOSURE_REF_DOMAIN);
        put_u32(digest, self.evidence_version);
        put_text(digest, self.individual_id.as_str());
        put_text(digest, self.source_id.as_str());
        put_u64(digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        put_text(digest, self.protocol_id.as_str());
        digest.update(self.protocol_content_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhenotypeEvidenceStatus {
    Unavailable,
    PartialWindow {
        evidence: PhenotypeEvidenceRef,
        support_digest: ObservationSupportDigest,
    },
    CompleteWindow(PhenotypeEvidenceRef),
}

impl PhenotypeEvidenceStatus {
    fn validate_for(
        &self,
        individual_id: &EvolutionIndividualId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionEvidenceError> {
        match self {
            Self::Unavailable => Ok(()),
            Self::PartialWindow { evidence, .. } | Self::CompleteWindow(evidence) => {
                evidence.validate_for(individual_id, context_digest)
            }
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Unavailable => digest.update([0]),
            Self::PartialWindow {
                evidence,
                support_digest,
            } => {
                digest.update([1]);
                evidence.update_digest(digest);
                digest.update(support_digest.as_bytes());
            }
            Self::CompleteWindow(evidence) => {
                digest.update([2]);
                evidence.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExposureEvidenceStatus {
    Unavailable,
    PartialWindow {
        evidence: ExposureEvidenceRef,
        support_digest: ObservationSupportDigest,
    },
    CompleteWindow(ExposureEvidenceRef),
}

impl ExposureEvidenceStatus {
    fn validate_for(
        &self,
        individual_id: &EvolutionIndividualId,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionEvidenceError> {
        match self {
            Self::Unavailable => Ok(()),
            Self::PartialWindow { evidence, .. } | Self::CompleteWindow(evidence) => {
                evidence.validate_for(individual_id, context_digest)
            }
        }
    }

    fn update_digest(&self, digest: &mut Sha256) {
        match self {
            Self::Unavailable => digest.update([0]),
            Self::PartialWindow {
                evidence,
                support_digest,
            } => {
                digest.update([1]);
                evidence.update_digest(digest);
                digest.update(support_digest.as_bytes());
            }
            Self::CompleteWindow(evidence) => {
                digest.update([2]);
                evidence.update_digest(digest);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualSelectionEvidenceInput {
    pub individual_id: EvolutionIndividualId,
    pub phenotype: PhenotypeEvidenceStatus,
    pub exposure: ExposureEvidenceStatus,
}

impl IndividualSelectionEvidenceInput {
    pub fn new(
        individual_id: EvolutionIndividualId,
        phenotype: PhenotypeEvidenceStatus,
        exposure: ExposureEvidenceStatus,
    ) -> Self {
        Self {
            individual_id,
            phenotype,
            exposure,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionEvidenceRecord {
    record_version: u32,
    pub individual_id: EvolutionIndividualId,
    individual_manifest_digest: LinkedIndividualManifestDigest,
    consequence_observation_digest: IndividualConsequenceObservationDigest,
    pub phenotype: PhenotypeEvidenceStatus,
    pub exposure: ExposureEvidenceStatus,
}

impl SelectionEvidenceRecord {
    fn validate_local(
        &self,
        context_digest: EvolutionaryContextRefDigest,
    ) -> Result<(), SelectionEvidenceError> {
        if self.record_version != SELECTION_EVIDENCE_RECORD_VERSION {
            return Err(SelectionEvidenceError::UnsupportedRecordVersion(
                self.record_version,
            ));
        }
        self.phenotype
            .validate_for(&self.individual_id, context_digest)?;
        self.exposure
            .validate_for(&self.individual_id, context_digest)?;
        Ok(())
    }

    pub fn individual_manifest_digest(&self) -> LinkedIndividualManifestDigest {
        self.individual_manifest_digest
    }

    pub fn consequence_observation_digest(&self) -> IndividualConsequenceObservationDigest {
        self.consequence_observation_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitSelectionEvidenceLedger {
    ledger_version: u32,
    consequence_ledger_digest: ExplicitConsequenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
    pub records: Vec<SelectionEvidenceRecord>,
}

impl ExplicitSelectionEvidenceLedger {
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        inputs: impl IntoIterator<Item = IndividualSelectionEvidenceInput>,
    ) -> Result<Self, SelectionEvidenceError> {
        census.validate_current(
            validated_consequences.population_id(),
            schema,
            chromosome_map,
            subjects,
        )?;
        let census_digest = census.canonical_digest()?;
        if census_digest != validated_consequences.census_digest() {
            return Err(SelectionEvidenceError::CensusAuthorityMismatch);
        }

        let context_digest = validated_consequences.context_digest();
        let mut by_individual = BTreeMap::new();
        for input in inputs {
            input
                .phenotype
                .validate_for(&input.individual_id, context_digest)?;
            input
                .exposure
                .validate_for(&input.individual_id, context_digest)?;
            if by_individual
                .insert(input.individual_id.clone(), input)
                .is_some()
            {
                return Err(SelectionEvidenceError::DuplicateIndividualEvidence);
            }
        }

        let consequence_observations = &validated_consequences.ledger().observations;
        if by_individual.len() != consequence_observations.len() {
            return Err(SelectionEvidenceError::IncompleteConsequenceCoverage);
        }

        let subjects_by_individual: BTreeMap<_, _> = subjects
            .iter()
            .map(|subject| (subject.manifest.individual_id.clone(), *subject))
            .collect();

        let mut records = Vec::with_capacity(consequence_observations.len());
        for observation in consequence_observations {
            let input = by_individual
                .remove(&observation.individual_id)
                .ok_or(SelectionEvidenceError::IncompleteConsequenceCoverage)?;
            let subject = subjects_by_individual
                .get(&observation.individual_id)
                .ok_or_else(|| {
                    SelectionEvidenceError::IndividualNotInCurrentSubjects(
                        observation.individual_id.clone(),
                    )
                })?;
            records.push(SelectionEvidenceRecord {
                record_version: SELECTION_EVIDENCE_RECORD_VERSION,
                individual_id: observation.individual_id.clone(),
                individual_manifest_digest: subject.manifest.canonical_digest(),
                consequence_observation_digest: observation.canonical_digest()?,
                phenotype: input.phenotype,
                exposure: input.exposure,
            });
        }
        if !by_individual.is_empty() {
            return Err(SelectionEvidenceError::ExtraIndividualEvidence);
        }

        let ledger = Self {
            ledger_version: EXPLICIT_SELECTION_EVIDENCE_LEDGER_VERSION,
            consequence_ledger_digest: validated_consequences.ledger_digest(),
            population_id: validated_consequences.population_id().clone(),
            census_digest,
            context_digest,
            records,
        };
        ledger.validate_local()?;
        Ok(ledger)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
    ) -> Result<(), SelectionEvidenceError> {
        self.validate_local()?;
        let inputs = self.records.iter().map(|record| {
            IndividualSelectionEvidenceInput::new(
                record.individual_id.clone(),
                record.phenotype.clone(),
                record.exposure.clone(),
            )
        });
        let recomputed = Self::capture(
            validated_consequences,
            schema,
            chromosome_map,
            census,
            subjects,
            inputs,
        )?;
        if recomputed != *self {
            return Err(SelectionEvidenceError::LedgerReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<ExplicitSelectionEvidenceLedgerDigest, SelectionEvidenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(SELECTION_EVIDENCE_LEDGER_DOMAIN);
        put_u32(&mut digest, self.ledger_version);
        digest.update(self.consequence_ledger_digest.as_bytes());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records {
            put_u32(&mut digest, record.record_version);
            put_text(&mut digest, record.individual_id.as_str());
            digest.update(record.individual_manifest_digest.as_bytes());
            digest.update(record.consequence_observation_digest.as_bytes());
            record.phenotype.update_digest(&mut digest);
            record.exposure.update_digest(&mut digest);
        }
        Ok(ExplicitSelectionEvidenceLedgerDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), SelectionEvidenceError> {
        if self.ledger_version != EXPLICIT_SELECTION_EVIDENCE_LEDGER_VERSION {
            return Err(SelectionEvidenceError::UnsupportedLedgerVersion(
                self.ledger_version,
            ));
        }
        if self.records.is_empty() {
            return Err(SelectionEvidenceError::IncompleteConsequenceCoverage);
        }
        if self
            .records
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(SelectionEvidenceError::NonCanonicalRecordOrder);
        }
        for record in &self.records {
            record.validate_local(self.context_digest)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExplicitSelectionEvidenceLedgerDigest([u8; 32]);

impl ExplicitSelectionEvidenceLedgerDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ExplicitSelectionEvidenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExplicitSelectionEvidenceLedgerDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ExplicitSelectionEvidenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug)]
#[must_use = "validated selection evidence should be consumed by downstream analysis APIs"]
pub struct ValidatedSelectionEvidenceLedger<'a> {
    ledger: &'a ExplicitSelectionEvidenceLedger,
    ledger_digest: ExplicitSelectionEvidenceLedgerDigest,
    consequence_ledger_digest: ExplicitConsequenceLedgerDigest,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
}

impl<'a> ValidatedSelectionEvidenceLedger<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        ledger: &'a ExplicitSelectionEvidenceLedger,
        validated_consequences: &ValidatedConsequenceLedger<'_>,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
    ) -> Result<Self, SelectionEvidenceError> {
        ledger.validate_current(
            validated_consequences,
            schema,
            chromosome_map,
            census,
            subjects,
        )?;
        Ok(Self {
            ledger,
            ledger_digest: ledger.canonical_digest()?,
            consequence_ledger_digest: validated_consequences.ledger_digest(),
            population_id: validated_consequences.population_id().clone(),
            census_digest: validated_consequences.census_digest(),
            context_digest: validated_consequences.context_digest(),
        })
    }

    pub fn ledger(&self) -> &'a ExplicitSelectionEvidenceLedger {
        self.ledger
    }

    pub fn ledger_digest(&self) -> ExplicitSelectionEvidenceLedgerDigest {
        self.ledger_digest
    }

    pub fn consequence_ledger_digest(&self) -> ExplicitConsequenceLedgerDigest {
        self.consequence_ledger_digest
    }

    pub fn population_id(&self) -> &PopulationId {
        &self.population_id
    }

    pub fn census_digest(&self) -> ExplicitLinkedPopulationCensusDigest {
        self.census_digest
    }

    pub fn context_digest(&self) -> EvolutionaryContextRefDigest {
        self.context_digest
    }
}

#[derive(Debug)]
pub enum SelectionEvidenceError {
    Evolution(EvolutionError),
    Individual(LinkedIndividualError),
    Consequence(ConsequenceError),
    UnsupportedPhenotypeEvidenceVersion(u32),
    UnsupportedExposureEvidenceVersion(u32),
    UnsupportedRecordVersion(u32),
    UnsupportedLedgerVersion(u32),
    EvidenceIndividualMismatch,
    EvidenceContextMismatch,
    CensusAuthorityMismatch,
    DuplicateIndividualEvidence,
    IncompleteConsequenceCoverage,
    ExtraIndividualEvidence,
    IndividualNotInCurrentSubjects(EvolutionIndividualId),
    NonCanonicalRecordOrder,
    LedgerReplayMismatch,
}

impl fmt::Display for SelectionEvidenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority failed: {error}"),
            Self::Individual(error) => write!(f, "individual authority failed: {error}"),
            Self::Consequence(error) => write!(f, "consequence authority failed: {error}"),
            Self::UnsupportedPhenotypeEvidenceVersion(version) => {
                write!(f, "unsupported phenotype-evidence version {version}")
            }
            Self::UnsupportedExposureEvidenceVersion(version) => {
                write!(f, "unsupported exposure-evidence version {version}")
            }
            Self::UnsupportedRecordVersion(version) => {
                write!(f, "unsupported selection-evidence record version {version}")
            }
            Self::UnsupportedLedgerVersion(version) => {
                write!(f, "unsupported selection-evidence ledger version {version}")
            }
            Self::EvidenceIndividualMismatch => {
                write!(f, "external evidence is bound to a different individual")
            }
            Self::EvidenceContextMismatch => {
                write!(f, "external evidence is bound to a different context/window")
            }
            Self::CensusAuthorityMismatch => {
                write!(f, "selection evidence census differs from validated consequence authority")
            }
            Self::DuplicateIndividualEvidence => {
                write!(f, "multiple selection-evidence inputs supplied for one individual")
            }
            Self::IncompleteConsequenceCoverage => {
                write!(f, "selection evidence does not preserve the complete consequence denominator")
            }
            Self::ExtraIndividualEvidence => {
                write!(f, "selection evidence contains an individual outside the consequence ledger")
            }
            Self::IndividualNotInCurrentSubjects(id) => write!(
                f,
                "selection-evidence individual {} is absent from current linked subjects",
                id.as_str()
            ),
            Self::NonCanonicalRecordOrder => {
                write!(f, "selection-evidence record order is noncanonical")
            }
            Self::LedgerReplayMismatch => write!(f, "selection-evidence ledger replay mismatch"),
        }
    }
}

impl Error for SelectionEvidenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Individual(error) => Some(error),
            Self::Consequence(error) => Some(error),
            _ => None,
        }
    }
}

impl From<EvolutionError> for SelectionEvidenceError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<LinkedIndividualError> for SelectionEvidenceError {
    fn from(value: LinkedIndividualError) -> Self {
        Self::Individual(value)
    }
}

impl From<ConsequenceError> for SelectionEvidenceError {
    fn from(value: ConsequenceError) -> Self {
        Self::Consequence(value)
    }
}
