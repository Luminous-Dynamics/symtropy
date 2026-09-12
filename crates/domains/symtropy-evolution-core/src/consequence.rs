use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, ChromosomeMap, ConsequenceObservationId,
    ConsequenceWindowId, EvolutionError, EvolutionIndividualId, EvolutionaryContextId,
    ExplicitLinkedPopulationCensus, ExplicitLinkedPopulationCensusDigest, HereditarySchema,
    LinkedIndividualError, LinkedIndividualManifestDigest, LinkedIndividualSubject, PopulationId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const EVOLUTIONARY_CONTEXT_REF_VERSION: u32 = 1;
pub const INDIVIDUAL_CONSEQUENCE_OBSERVATION_VERSION: u32 = 1;
pub const EXPLICIT_CONSEQUENCE_LEDGER_VERSION: u32 = 1;

const CONTEXT_REF_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:evolutionary-context-ref:v1\0";
const CONSEQUENCE_OBSERVATION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:individual-consequence-observation:v1\0";
const CONSEQUENCE_LEDGER_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:explicit-consequence-ledger:v1\0";

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvolutionaryContextContentDigest([u8; 32]);

impl EvolutionaryContextContentDigest {
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for EvolutionaryContextContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvolutionaryContextContentDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for EvolutionaryContextContentDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionaryContextRef {
    context_version: u32,
    pub context_id: EvolutionaryContextId,
    pub revision: u64,
    pub content_digest: EvolutionaryContextContentDigest,
    pub window_id: ConsequenceWindowId,
}

impl EvolutionaryContextRef {
    pub fn new(
        context_id: EvolutionaryContextId,
        revision: u64,
        content_digest: EvolutionaryContextContentDigest,
        window_id: ConsequenceWindowId,
    ) -> Self {
        Self {
            context_version: EVOLUTIONARY_CONTEXT_REF_VERSION,
            context_id,
            revision,
            content_digest,
            window_id,
        }
    }

    pub fn validate(&self) -> Result<(), ConsequenceError> {
        if self.context_version != EVOLUTIONARY_CONTEXT_REF_VERSION {
            return Err(ConsequenceError::UnsupportedContextVersion(
                self.context_version,
            ));
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<EvolutionaryContextRefDigest, ConsequenceError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(CONTEXT_REF_DIGEST_DOMAIN);
        put_u32(&mut digest, self.context_version);
        put_text(&mut digest, self.context_id.as_str());
        put_u64(&mut digest, self.revision);
        digest.update(self.content_digest.as_bytes());
        put_text(&mut digest, self.window_id.as_str());
        Ok(EvolutionaryContextRefDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvolutionaryContextRefDigest([u8; 32]);

impl EvolutionaryContextRefDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for EvolutionaryContextRefDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EvolutionaryContextRefDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for EvolutionaryContextRefDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViabilityConsequence {
    SurvivedWindow,
    DiedDuringWindow,
}

impl ViabilityConsequence {
    fn tag(self) -> u8 {
        match self {
            Self::SurvivedWindow => 0,
            Self::DiedDuringWindow => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductiveEventConsequence {
    pub opportunities_observed: u64,
    pub realized_events: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantProductionConsequence {
    pub produced: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantRecruitmentConsequence {
    pub recruited: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualConsequences {
    pub viability: Option<ViabilityConsequence>,
    pub reproductive_events: Option<ReproductiveEventConsequence>,
    pub descendant_production: Option<DescendantProductionConsequence>,
    pub descendant_recruitment: Option<DescendantRecruitmentConsequence>,
}

impl IndividualConsequences {
    pub fn validate(&self) -> Result<(), ConsequenceError> {
        if self.viability.is_none()
            && self.reproductive_events.is_none()
            && self.descendant_production.is_none()
            && self.descendant_recruitment.is_none()
        {
            return Err(ConsequenceError::NoObservedChannels);
        }
        if let Some(events) = &self.reproductive_events {
            if events.realized_events > events.opportunities_observed {
                return Err(ConsequenceError::ReproductiveEventsExceedOpportunities);
            }
        }
        if let (Some(production), Some(recruitment)) =
            (&self.descendant_production, &self.descendant_recruitment)
        {
            if recruitment.recruited > production.produced {
                return Err(ConsequenceError::RecruitmentExceedsProduction);
            }
        }
        Ok(())
    }

    fn update_digest(&self, digest: &mut Sha256) {
        match self.viability {
            None => digest.update([0]),
            Some(value) => digest.update([1, value.tag()]),
        }
        match &self.reproductive_events {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                put_u64(digest, value.opportunities_observed);
                put_u64(digest, value.realized_events);
            }
        }
        match &self.descendant_production {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                put_u64(digest, value.produced);
            }
        }
        match &self.descendant_recruitment {
            None => digest.update([0]),
            Some(value) => {
                digest.update([1]);
                put_u64(digest, value.recruited);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndividualConsequenceObservation {
    observation_version: u32,
    pub observation_id: ConsequenceObservationId,
    pub individual_id: EvolutionIndividualId,
    population_id: PopulationId,
    individual_manifest_digest: LinkedIndividualManifestDigest,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
    pub consequences: IndividualConsequences,
}

impl IndividualConsequenceObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn declare(
        observation_id: ConsequenceObservationId,
        individual_id: EvolutionIndividualId,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        context: &EvolutionaryContextRef,
        consequences: IndividualConsequences,
    ) -> Result<Self, ConsequenceError> {
        context.validate()?;
        consequences.validate()?;
        census.validate_current(expected_population_id, schema, chromosome_map, subjects)?;
        let subject = find_subject(subjects, &individual_id)
            .ok_or_else(|| ConsequenceError::IndividualNotInCensus(individual_id.clone()))?;
        let member = census
            .members
            .iter()
            .find(|member| member.individual_id == individual_id)
            .ok_or_else(|| ConsequenceError::IndividualNotInCensus(individual_id.clone()))?;
        let manifest_digest = subject.manifest.canonical_digest();
        if member.manifest_digest != manifest_digest {
            return Err(ConsequenceError::IndividualManifestMismatch);
        }

        let observation = Self {
            observation_version: INDIVIDUAL_CONSEQUENCE_OBSERVATION_VERSION,
            observation_id,
            individual_id,
            population_id: expected_population_id.clone(),
            individual_manifest_digest: manifest_digest,
            census_digest: census.canonical_digest()?,
            context_digest: context.canonical_digest()?,
            consequences,
        };
        observation.validate_local()?;
        Ok(observation)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        context: &EvolutionaryContextRef,
    ) -> Result<(), ConsequenceError> {
        self.validate_local()?;
        let recomputed = Self::declare(
            self.observation_id.clone(),
            self.individual_id.clone(),
            expected_population_id,
            schema,
            chromosome_map,
            census,
            subjects,
            context,
            self.consequences.clone(),
        )?;
        if recomputed != *self {
            return Err(ConsequenceError::ObservationReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<IndividualConsequenceObservationDigest, ConsequenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(CONSEQUENCE_OBSERVATION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.observation_version);
        put_text(&mut digest, self.observation_id.as_str());
        put_text(&mut digest, self.individual_id.as_str());
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.individual_manifest_digest.as_bytes());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        self.consequences.update_digest(&mut digest);
        Ok(IndividualConsequenceObservationDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ConsequenceError> {
        if self.observation_version != INDIVIDUAL_CONSEQUENCE_OBSERVATION_VERSION {
            return Err(ConsequenceError::UnsupportedObservationVersion(
                self.observation_version,
            ));
        }
        self.consequences.validate()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IndividualConsequenceObservationDigest([u8; 32]);

impl IndividualConsequenceObservationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for IndividualConsequenceObservationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IndividualConsequenceObservationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for IndividualConsequenceObservationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitConsequenceLedger {
    ledger_version: u32,
    population_id: PopulationId,
    census_digest: ExplicitLinkedPopulationCensusDigest,
    context_digest: EvolutionaryContextRefDigest,
    pub observations: Vec<IndividualConsequenceObservation>,
}

impl ExplicitConsequenceLedger {
    #[allow(clippy::too_many_arguments)]
    pub fn capture(
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        context: &EvolutionaryContextRef,
        observations: impl IntoIterator<Item = IndividualConsequenceObservation>,
    ) -> Result<Self, ConsequenceError> {
        context.validate()?;
        census.validate_current(expected_population_id, schema, chromosome_map, subjects)?;
        let census_digest = census.canonical_digest()?;
        let context_digest = context.canonical_digest()?;

        let mut by_individual = BTreeMap::new();
        let mut observation_ids = BTreeMap::new();
        for observation in observations {
            observation.validate_current(
                expected_population_id,
                schema,
                chromosome_map,
                census,
                subjects,
                context,
            )?;
            if observation.census_digest != census_digest
                || observation.context_digest != context_digest
            {
                return Err(ConsequenceError::ObservationAuthorityMismatch);
            }
            if observation_ids
                .insert(observation.observation_id.clone(), ())
                .is_some()
            {
                return Err(ConsequenceError::DuplicateObservationId(
                    observation.observation_id.clone(),
                ));
            }
            if by_individual
                .insert(observation.individual_id.clone(), observation)
                .is_some()
            {
                return Err(ConsequenceError::DuplicateIndividualObservation);
            }
        }

        if by_individual.len() != census.members.len()
            || census
                .members
                .iter()
                .any(|member| !by_individual.contains_key(&member.individual_id))
        {
            return Err(ConsequenceError::IncompleteCensusCoverage);
        }

        let ledger = Self {
            ledger_version: EXPLICIT_CONSEQUENCE_LEDGER_VERSION,
            population_id: expected_population_id.clone(),
            census_digest,
            context_digest,
            observations: by_individual.into_values().collect(),
        };
        ledger.validate_local()?;
        Ok(ledger)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        census: &ExplicitLinkedPopulationCensus,
        subjects: &[LinkedIndividualSubject<'_>],
        context: &EvolutionaryContextRef,
    ) -> Result<(), ConsequenceError> {
        self.validate_local()?;
        let recomputed = Self::capture(
            expected_population_id,
            schema,
            chromosome_map,
            census,
            subjects,
            context,
            self.observations.clone(),
        )?;
        if recomputed != *self {
            return Err(ConsequenceError::LedgerReplayMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<ExplicitConsequenceLedgerDigest, ConsequenceError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(CONSEQUENCE_LEDGER_DIGEST_DOMAIN);
        put_u32(&mut digest, self.ledger_version);
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.census_digest.as_bytes());
        digest.update(self.context_digest.as_bytes());
        put_u64(&mut digest, self.observations.len() as u64);
        for observation in &self.observations {
            digest.update(observation.canonical_digest()?.as_bytes());
        }
        Ok(ExplicitConsequenceLedgerDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), ConsequenceError> {
        if self.ledger_version != EXPLICIT_CONSEQUENCE_LEDGER_VERSION {
            return Err(ConsequenceError::UnsupportedLedgerVersion(
                self.ledger_version,
            ));
        }
        if self.observations.is_empty() {
            return Err(ConsequenceError::IncompleteCensusCoverage);
        }
        if self
            .observations
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(ConsequenceError::NonCanonicalObservationOrder);
        }
        let mut ids = BTreeMap::new();
        for observation in &self.observations {
            observation.validate_local()?;
            if observation.population_id != self.population_id
                || observation.census_digest != self.census_digest
                || observation.context_digest != self.context_digest
            {
                return Err(ConsequenceError::ObservationAuthorityMismatch);
            }
            if ids
                .insert(observation.observation_id.clone(), ())
                .is_some()
            {
                return Err(ConsequenceError::DuplicateObservationId(
                    observation.observation_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExplicitConsequenceLedgerDigest([u8; 32]);

impl ExplicitConsequenceLedgerDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ExplicitConsequenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExplicitConsequenceLedgerDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ExplicitConsequenceLedgerDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

fn find_subject<'a>(
    subjects: &[LinkedIndividualSubject<'a>],
    individual_id: &EvolutionIndividualId,
) -> Option<LinkedIndividualSubject<'a>> {
    subjects
        .iter()
        .copied()
        .find(|subject| &subject.manifest.individual_id == individual_id)
}

#[derive(Debug)]
pub enum ConsequenceError {
    Evolution(EvolutionError),
    Individual(LinkedIndividualError),
    UnsupportedContextVersion(u32),
    UnsupportedObservationVersion(u32),
    UnsupportedLedgerVersion(u32),
    NoObservedChannels,
    ReproductiveEventsExceedOpportunities,
    RecruitmentExceedsProduction,
    IndividualNotInCensus(EvolutionIndividualId),
    IndividualManifestMismatch,
    ObservationAuthorityMismatch,
    ObservationReplayMismatch,
    DuplicateObservationId(ConsequenceObservationId),
    DuplicateIndividualObservation,
    IncompleteCensusCoverage,
    NonCanonicalObservationOrder,
    LedgerReplayMismatch,
}

impl fmt::Display for ConsequenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority failed: {error}"),
            Self::Individual(error) => write!(f, "individual authority failed: {error}"),
            Self::UnsupportedContextVersion(version) => {
                write!(f, "unsupported evolutionary-context version {version}")
            }
            Self::UnsupportedObservationVersion(version) => {
                write!(f, "unsupported consequence-observation version {version}")
            }
            Self::UnsupportedLedgerVersion(version) => {
                write!(f, "unsupported consequence-ledger version {version}")
            }
            Self::NoObservedChannels => write!(f, "consequence observation contains no supplied channels"),
            Self::ReproductiveEventsExceedOpportunities => write!(f, "realized reproductive events exceed observed opportunities"),
            Self::RecruitmentExceedsProduction => write!(f, "recruited descendants exceed produced descendants"),
            Self::IndividualNotInCensus(id) => write!(f, "individual {} is not in the explicit census", id.as_str()),
            Self::IndividualManifestMismatch => write!(f, "consequence individual manifest does not match census membership"),
            Self::ObservationAuthorityMismatch => write!(f, "consequence observation authority mismatch"),
            Self::ObservationReplayMismatch => write!(f, "consequence observation replay mismatch"),
            Self::DuplicateObservationId(id) => write!(f, "duplicate consequence observation id {}", id.as_str()),
            Self::DuplicateIndividualObservation => write!(f, "multiple consequence observations supplied for one individual"),
            Self::IncompleteCensusCoverage => write!(f, "consequence ledger does not cover the exact explicit census once"),
            Self::NonCanonicalObservationOrder => write!(f, "consequence ledger observation order is noncanonical"),
            Self::LedgerReplayMismatch => write!(f, "consequence ledger replay mismatch"),
        }
    }
}

impl Error for ConsequenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Individual(error) => Some(error),
            _ => None,
        }
    }
}

impl From<EvolutionError> for ConsequenceError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<LinkedIndividualError> for ConsequenceError {
    fn from(value: LinkedIndividualError) -> Self {
        Self::Individual(value)
    }
}
