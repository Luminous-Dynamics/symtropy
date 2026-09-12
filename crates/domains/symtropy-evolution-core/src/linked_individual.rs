use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64}, AncestryAuthorityError, AncestryCopyId,
    ChromosomeMap, ChromosomeMapDigest, EvolutionError, EvolutionIndividualId,
    HereditarySchema, HereditarySchemaDigest, MutationFateError, MutationFateSubject,
    MutationLineageError, MutationLineageState, MutationLineageStateDigest, PhasedAncestryState,
    PhasedAncestryStateDigest, PhasedHereditaryState, PhasedHereditaryStateDigest, PopulationId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};

pub const LINKED_INDIVIDUAL_MANIFEST_VERSION: u32 = 1;
pub const EXPLICIT_LINKED_POPULATION_CENSUS_VERSION: u32 = 1;

const LINKED_INDIVIDUAL_MANIFEST_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-individual-manifest:v1\0";
const EXPLICIT_LINKED_POPULATION_CENSUS_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:explicit-linked-population-census:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedIndividualManifest {
    manifest_version: u32,
    pub individual_id: EvolutionIndividualId,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    phased_state_digest: PhasedHereditaryStateDigest,
    ancestry_state_digest: PhasedAncestryStateDigest,
    mutation_lineage_digest: MutationLineageStateDigest,
}

impl LinkedIndividualManifest {
    pub fn new(
        individual_id: EvolutionIndividualId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        ancestry_state: &PhasedAncestryState,
        lineage_state: &MutationLineageState,
    ) -> Result<Self, LinkedIndividualError> {
        phased_state.validate(schema, chromosome_map)?;
        ancestry_state.validate_current(schema, chromosome_map, phased_state)?;
        lineage_state.validate_current(schema, chromosome_map, phased_state, ancestry_state)?;

        let manifest = Self {
            manifest_version: LINKED_INDIVIDUAL_MANIFEST_VERSION,
            individual_id,
            schema_digest: schema.canonical_digest()?,
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            phased_state_digest: phased_state.canonical_digest(schema, chromosome_map)?,
            ancestry_state_digest: ancestry_state.canonical_digest(
                schema,
                chromosome_map,
                phased_state,
            )?,
            mutation_lineage_digest: lineage_state.canonical_digest(
                schema,
                chromosome_map,
                phased_state,
                ancestry_state,
            )?,
        };
        manifest.validate_current(
            schema,
            chromosome_map,
            phased_state,
            ancestry_state,
            lineage_state,
        )?;
        Ok(manifest)
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        ancestry_state: &PhasedAncestryState,
        lineage_state: &MutationLineageState,
    ) -> Result<(), LinkedIndividualError> {
        if self.manifest_version != LINKED_INDIVIDUAL_MANIFEST_VERSION {
            return Err(LinkedIndividualError::UnsupportedManifestVersion(
                self.manifest_version,
            ));
        }
        phased_state.validate(schema, chromosome_map)?;
        ancestry_state.validate_current(schema, chromosome_map, phased_state)?;
        lineage_state.validate_current(schema, chromosome_map, phased_state, ancestry_state)?;

        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.phased_state_digest != phased_state.canonical_digest(schema, chromosome_map)?
            || self.ancestry_state_digest
                != ancestry_state.canonical_digest(schema, chromosome_map, phased_state)?
            || self.mutation_lineage_digest
                != lineage_state.canonical_digest(
                    schema,
                    chromosome_map,
                    phased_state,
                    ancestry_state,
                )?
        {
            return Err(LinkedIndividualError::ManifestAuthorityMismatch);
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> LinkedIndividualManifestDigest {
        let mut digest = Sha256::new();
        digest.update(LINKED_INDIVIDUAL_MANIFEST_DIGEST_DOMAIN);
        put_u32(&mut digest, self.manifest_version);
        put_text(&mut digest, self.individual_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.phased_state_digest.as_bytes());
        digest.update(self.ancestry_state_digest.as_bytes());
        digest.update(self.mutation_lineage_digest.as_bytes());
        LinkedIndividualManifestDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedIndividualManifestDigest([u8; 32]);

impl LinkedIndividualManifestDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedIndividualManifestDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedIndividualManifestDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedIndividualManifestDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinkedIndividualSubject<'a> {
    pub manifest: &'a LinkedIndividualManifest,
    pub phased_state: &'a PhasedHereditaryState,
    pub ancestry_state: &'a PhasedAncestryState,
    pub lineage_state: &'a MutationLineageState,
}

impl<'a> LinkedIndividualSubject<'a> {
    pub fn new(
        manifest: &'a LinkedIndividualManifest,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &'a PhasedHereditaryState,
        ancestry_state: &'a PhasedAncestryState,
        lineage_state: &'a MutationLineageState,
    ) -> Result<Self, LinkedIndividualError> {
        manifest.validate_current(
            schema,
            chromosome_map,
            phased_state,
            ancestry_state,
            lineage_state,
        )?;
        Ok(Self {
            manifest,
            phased_state,
            ancestry_state,
            lineage_state,
        })
    }

    pub fn as_mutation_fate_subject(&self) -> Result<MutationFateSubject<'a>, LinkedIndividualError> {
        Ok(MutationFateSubject::new(
            self.phased_state,
            self.ancestry_state,
            self.lineage_state,
            1,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitLinkedPopulationMember {
    pub individual_id: EvolutionIndividualId,
    pub manifest_digest: LinkedIndividualManifestDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExplicitLinkedPopulationCensus {
    census_version: u32,
    pub population_id: PopulationId,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    pub members: Vec<ExplicitLinkedPopulationMember>,
}

impl ExplicitLinkedPopulationCensus {
    pub fn capture(
        population_id: PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        subjects: &[LinkedIndividualSubject<'_>],
    ) -> Result<Self, LinkedIndividualError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        let members = validated_members(schema, chromosome_map, subjects)?;
        let census = Self {
            census_version: EXPLICIT_LINKED_POPULATION_CENSUS_VERSION,
            population_id,
            schema_digest: schema.canonical_digest()?,
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            members,
        };
        census.validate_local()?;
        Ok(census)
    }

    pub fn validate_current(
        &self,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        subjects: &[LinkedIndividualSubject<'_>],
    ) -> Result<(), LinkedIndividualError> {
        self.validate_local()?;
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if &self.population_id != expected_population_id {
            return Err(LinkedIndividualError::PopulationContextMismatch);
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(LinkedIndividualError::CensusAuthorityMismatch);
        }
        let current = validated_members(schema, chromosome_map, subjects)?;
        if current != self.members {
            return Err(LinkedIndividualError::CensusMemberMismatch);
        }
        Ok(())
    }

    pub fn census_size(&self) -> u64 {
        self.members.len() as u64
    }

    pub fn mutation_fate_subjects<'a>(
        &self,
        expected_population_id: &PopulationId,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        subjects: &'a [LinkedIndividualSubject<'a>],
    ) -> Result<Vec<MutationFateSubject<'a>>, LinkedIndividualError> {
        self.validate_current(expected_population_id, schema, chromosome_map, subjects)?;
        subjects
            .iter()
            .map(LinkedIndividualSubject::as_mutation_fate_subject)
            .collect()
    }

    pub fn canonical_digest(&self) -> Result<ExplicitLinkedPopulationCensusDigest, LinkedIndividualError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(EXPLICIT_LINKED_POPULATION_CENSUS_DIGEST_DOMAIN);
        put_u32(&mut digest, self.census_version);
        put_text(&mut digest, self.population_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_u64(&mut digest, self.members.len() as u64);
        for member in &self.members {
            put_text(&mut digest, member.individual_id.as_str());
            digest.update(member.manifest_digest.as_bytes());
        }
        Ok(ExplicitLinkedPopulationCensusDigest(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), LinkedIndividualError> {
        if self.census_version != EXPLICIT_LINKED_POPULATION_CENSUS_VERSION {
            return Err(LinkedIndividualError::UnsupportedCensusVersion(
                self.census_version,
            ));
        }
        if self.members.is_empty() {
            return Err(LinkedIndividualError::EmptyCensus);
        }
        if self
            .members
            .windows(2)
            .any(|window| window[0].individual_id >= window[1].individual_id)
        {
            return Err(LinkedIndividualError::NonCanonicalMemberOrder);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExplicitLinkedPopulationCensusDigest([u8; 32]);

impl ExplicitLinkedPopulationCensusDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ExplicitLinkedPopulationCensusDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ExplicitLinkedPopulationCensusDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ExplicitLinkedPopulationCensusDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

fn validated_members(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    subjects: &[LinkedIndividualSubject<'_>],
) -> Result<Vec<ExplicitLinkedPopulationMember>, LinkedIndividualError> {
    if subjects.is_empty() {
        return Err(LinkedIndividualError::EmptyCensus);
    }

    let mut members = BTreeMap::new();
    let mut ancestry_owner: BTreeMap<AncestryCopyId, EvolutionIndividualId> = BTreeMap::new();

    for subject in subjects {
        subject.manifest.validate_current(
            schema,
            chromosome_map,
            subject.phased_state,
            subject.ancestry_state,
            subject.lineage_state,
        )?;
        let individual_id = subject.manifest.individual_id.clone();
        let member = ExplicitLinkedPopulationMember {
            individual_id: individual_id.clone(),
            manifest_digest: subject.manifest.canonical_digest(),
        };
        if members.insert(individual_id.clone(), member).is_some() {
            return Err(LinkedIndividualError::DuplicateIndividualId(individual_id));
        }

        for chromosome in subject.ancestry_state.chromosomes.values() {
            for class in &chromosome.classes {
                for copy_id in &class.copy_ids {
                    if let Some(existing) = ancestry_owner.insert(copy_id.clone(), individual_id.clone()) {
                        return Err(LinkedIndividualError::DuplicateAncestryCopyOwnership {
                            copy_id: copy_id.clone(),
                            first: existing,
                            second: individual_id.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(members.into_values().collect())
}

#[derive(Debug)]
pub enum LinkedIndividualError {
    Evolution(EvolutionError),
    Ancestry(AncestryAuthorityError),
    MutationLineage(MutationLineageError),
    MutationFate(MutationFateError),
    UnsupportedManifestVersion(u32),
    UnsupportedCensusVersion(u32),
    ManifestAuthorityMismatch,
    PopulationContextMismatch,
    CensusAuthorityMismatch,
    CensusMemberMismatch,
    EmptyCensus,
    NonCanonicalMemberOrder,
    DuplicateIndividualId(EvolutionIndividualId),
    DuplicateAncestryCopyOwnership {
        copy_id: AncestryCopyId,
        first: EvolutionIndividualId,
        second: EvolutionIndividualId,
    },
}

impl fmt::Display for LinkedIndividualError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority failed: {error}"),
            Self::Ancestry(error) => write!(f, "ancestry authority failed: {error}"),
            Self::MutationLineage(error) => write!(f, "mutation-lineage authority failed: {error}"),
            Self::MutationFate(error) => write!(f, "mutation-fate authority failed: {error}"),
            Self::UnsupportedManifestVersion(version) => {
                write!(f, "unsupported linked-individual manifest version {version}")
            }
            Self::UnsupportedCensusVersion(version) => {
                write!(f, "unsupported explicit linked census version {version}")
            }
            Self::ManifestAuthorityMismatch => write!(f, "linked-individual manifest authority mismatch"),
            Self::PopulationContextMismatch => write!(f, "explicit linked census population context mismatch"),
            Self::CensusAuthorityMismatch => write!(f, "explicit linked census authority mismatch"),
            Self::CensusMemberMismatch => write!(f, "explicit linked census member set mismatch"),
            Self::EmptyCensus => write!(f, "explicit linked census may not be empty"),
            Self::NonCanonicalMemberOrder => write!(f, "explicit linked census member order is noncanonical"),
            Self::DuplicateIndividualId(id) => write!(f, "duplicate individual identity {}", id.as_str()),
            Self::DuplicateAncestryCopyOwnership { copy_id, first, second } => write!(
                f,
                "ancestry copy {} is owned by both {} and {}",
                copy_id.as_str(),
                first.as_str(),
                second.as_str()
            ),
        }
    }
}

impl Error for LinkedIndividualError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Ancestry(error) => Some(error),
            Self::MutationLineage(error) => Some(error),
            Self::MutationFate(error) => Some(error),
            _ => None,
        }
    }
}

impl From<EvolutionError> for LinkedIndividualError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<AncestryAuthorityError> for LinkedIndividualError {
    fn from(value: AncestryAuthorityError) -> Self {
        Self::Ancestry(value)
    }
}

impl From<MutationLineageError> for LinkedIndividualError {
    fn from(value: MutationLineageError) -> Self {
        Self::MutationLineage(value)
    }
}

impl From<MutationFateError> for LinkedIndividualError {
    fn from(value: MutationFateError) -> Self {
        Self::MutationFate(value)
    }
}
