// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic heredity and population-genetics primitives for Symtropy.
//!
//! EVO-03 deliberately owns hereditary/population semantics only. It does not
//! own ecology, fitness truth, morphology, rendering, persistent organism
//! identity, speciation, cognition, or civilization.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const EVOLUTION_SCHEMA_VERSION: u32 = 1;
pub const PROBABILITY_SCALE_PPM: u32 = 1_000_000;

const SCHEMA_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:heredity-schema:v1\0";
const STATE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:hereditary-state:v1\0";
const OPERATOR_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:operator-profile:v1\0";
const POPULATION_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:population-genetics:v1\0";
const REPRODUCTION_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:reproduction-provenance:v1\0";
const RNG_DOMAIN: &[u8] = b"symtropy:evolution:semantic-draw:v1\0";
const MUTATION_RNG_DOMAIN: &[u8] = b"mutation:v1\0";
const RECOMBINATION_RNG_DOMAIN: &[u8] = b"recombination:v1\0";

macro_rules! semantic_id {
    ($name:ident) => {
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, EvolutionError> {
                let value = value.into();
                validate_text(stringify!($name), &value)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

macro_rules! digest_display {
    ($name:ident) => {
        impl $name {
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                for byte in self.0 {
                    write!(f, "{byte:02x}")?;
                }
                Ok(())
            }
        }
    };
}

semantic_id!(HereditarySchemaId);
semantic_id!(LocusId);
semantic_id!(AlleleId);
semantic_id!(PopulationId);
semantic_id!(ReproductionEventId);
semantic_id!(OperatorProfileId);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocusDefinition {
    pub id: LocusId,
    pub allowed_alleles: BTreeSet<AlleleId>,
}

impl LocusDefinition {
    pub fn new(
        id: LocusId,
        allowed_alleles: impl IntoIterator<Item = AlleleId>,
    ) -> Result<Self, EvolutionError> {
        let value = Self {
            id,
            allowed_alleles: allowed_alleles.into_iter().collect(),
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        if self.allowed_alleles.is_empty() {
            return Err(EvolutionError::NoAlleles {
                locus: self.id.clone(),
            });
        }
        Ok(())
    }
}

/// Versioned finite hereditary schema.
///
/// V0 intentionally models named loci rather than pretending to implement a
/// universal molecular genome. `ploidy` is explicit; clonal reproduction may
/// use any supported ploidy, while V0 biparental recombination is diploid only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HereditarySchema {
    pub schema_version: u32,
    pub id: HereditarySchemaId,
    pub ploidy: u8,
    pub loci: BTreeMap<LocusId, LocusDefinition>,
}

impl HereditarySchema {
    pub fn new(
        id: HereditarySchemaId,
        ploidy: u8,
        loci: impl IntoIterator<Item = LocusDefinition>,
    ) -> Result<Self, EvolutionError> {
        let mut by_id = BTreeMap::new();
        for locus in loci {
            if by_id.insert(locus.id.clone(), locus).is_some() {
                return Err(EvolutionError::DuplicateLocus);
            }
        }
        let value = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            id,
            ploidy,
            loci: by_id,
        };
        value.validate()?;
        Ok(value)
    }

    pub fn validate(&self) -> Result<(), EvolutionError> {
        if self.schema_version != EVOLUTION_SCHEMA_VERSION {
            return Err(EvolutionError::UnsupportedSchema(self.schema_version));
        }
        if !(1..=16).contains(&self.ploidy) {
            return Err(EvolutionError::UnsupportedPloidy(self.ploidy));
        }
        if self.loci.is_empty() {
            return Err(EvolutionError::NoLoci);
        }
        for (key, locus) in &self.loci {
            if key != &locus.id {
                return Err(EvolutionError::LocusKeyMismatch {
                    key: key.clone(),
                    value: locus.id.clone(),
                });
            }
            locus.validate()?;
        }
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<HereditarySchemaDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(SCHEMA_DIGEST_DOMAIN);
        put_u32(&mut digest, self.schema_version);
        put_text(&mut digest, self.id.as_str());
        digest.update([self.ploidy]);
        put_u64(&mut digest, self.loci.len() as u64);
        for (locus_id, locus) in &self.loci {
            put_text(&mut digest, locus_id.as_str());
            put_u64(&mut digest, locus.allowed_alleles.len() as u64);
            for allele in &locus.allowed_alleles {
                put_text(&mut digest, allele.as_str());
            }
        }
        Ok(HereditarySchemaDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HereditarySchemaDigest([u8; 32]);

digest_display!(HereditarySchemaDigest);

/// Exact hereditary content for one biological individual/propagule.
///
/// This is not persistent organism identity and does not itself prove parentage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HereditaryState {
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    /// Exact allele copies per locus. The vector length must equal schema ploidy.
    pub copies: BTreeMap<LocusId, Vec<AlleleId>>,
}

impl HereditaryState {
    pub fn new(
        schema: &HereditarySchema,
        copies: BTreeMap<LocusId, Vec<AlleleId>>,
    ) -> Result<Self, EvolutionError> {
        let value = Self {
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            copies,
        };
        value.validate(schema)?;
        Ok(value)
    }

    pub fn validate(&self, schema: &HereditarySchema) -> Result<(), EvolutionError> {
        schema.validate()?;
        if self.schema_id != schema.id {
            return Err(EvolutionError::HereditarySchemaMismatch);
        }
        if self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.copies.len() != schema.loci.len() {
            return Err(EvolutionError::LocusSetMismatch);
        }
        for (locus_id, locus) in &schema.loci {
            let copies = self
                .copies
                .get(locus_id)
                .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
            if copies.len() != usize::from(schema.ploidy) {
                return Err(EvolutionError::CopyCountMismatch {
                    locus: locus_id.clone(),
                    expected: schema.ploidy,
                    observed: copies.len(),
                });
            }
            for allele in copies {
                if !locus.allowed_alleles.contains(allele) {
                    return Err(EvolutionError::UnknownAllele {
                        locus: locus_id.clone(),
                        allele: allele.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<HereditaryStateDigest, EvolutionError> {
        self.validate(schema)?;
        let mut digest = Sha256::new();
        digest.update(STATE_DIGEST_DOMAIN);
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_u64(&mut digest, self.copies.len() as u64);
        for (locus, copies) in &self.copies {
            put_text(&mut digest, locus.as_str());
            put_u64(&mut digest, copies.len() as u64);
            for allele in copies {
                put_text(&mut digest, allele.as_str());
            }
        }
        Ok(HereditaryStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HereditaryStateDigest([u8; 32]);

digest_display!(HereditaryStateDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationProfile {
    pub model_id: String,
    pub version: String,
    pub per_copy_rate_ppm: u32,
}

impl MutationProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("mutation.model_id", &self.model_id)?;
        validate_text("mutation.version", &self.version)?;
        if self.per_copy_rate_ppm > PROBABILITY_SCALE_PPM {
            return Err(EvolutionError::ProbabilityOutOfRange {
                observed_ppm: self.per_copy_rate_ppm,
            });
        }
        Ok(())
    }

    fn put_randomness_identity(&self, digest: &mut Sha256) {
        digest.update(MUTATION_RNG_DOMAIN);
        put_text(digest, &self.model_id);
        put_text(digest, &self.version);
        // The numeric rate is intentionally not part of the random variate
        // identity. It is the threshold applied to the same keyed draw, so a
        // rate sweep does not secretly reroll mutation opportunities.
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecombinationMode {
    /// Each locus chooses parental allele copies from an independent keyed draw.
    /// Chromosome linkage/crossover is intentionally deferred from V0.
    IndependentLoci,
}

impl RecombinationMode {
    fn tag(self) -> u8 {
        match self {
            Self::IndependentLoci => 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecombinationProfile {
    pub model_id: String,
    pub version: String,
    pub mode: RecombinationMode,
}

impl RecombinationProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("recombination.model_id", &self.model_id)?;
        validate_text("recombination.version", &self.version)?;
        Ok(())
    }

    fn put_randomness_identity(&self, digest: &mut Sha256) {
        digest.update(RECOMBINATION_RNG_DOMAIN);
        put_text(digest, &self.model_id);
        put_text(digest, &self.version);
        digest.update([self.mode.tag()]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvolutionOperatorProfile {
    pub profile_id: OperatorProfileId,
    pub version: String,
    pub mutation: MutationProfile,
    pub recombination: RecombinationProfile,
}

impl EvolutionOperatorProfile {
    pub fn validate(&self) -> Result<(), EvolutionError> {
        validate_text("operator.version", &self.version)?;
        self.mutation.validate()?;
        self.recombination.validate()?;
        Ok(())
    }

    pub fn canonical_digest(&self) -> Result<EvolutionOperatorProfileDigest, EvolutionError> {
        self.validate()?;
        let mut digest = Sha256::new();
        digest.update(OPERATOR_DIGEST_DOMAIN);
        put_text(&mut digest, self.profile_id.as_str());
        put_text(&mut digest, &self.version);
        put_text(&mut digest, &self.mutation.model_id);
        put_text(&mut digest, &self.mutation.version);
        put_u32(&mut digest, self.mutation.per_copy_rate_ppm);
        put_text(&mut digest, &self.recombination.model_id);
        put_text(&mut digest, &self.recombination.version);
        digest.update([self.recombination.mode.tag()]);
        Ok(EvolutionOperatorProfileDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvolutionOperatorProfileDigest([u8; 32]);

digest_display!(EvolutionOperatorProfileDigest);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReproductionMode {
    Clonal,
    /// V0 reference sexual model: one independently sampled allele per locus
    /// from each of two diploid parents.
    BiparentalDiploidIndependentLoci,
}

impl ReproductionMode {
    fn tag(self) -> u8 {
        match self {
            Self::Clonal => 0,
            Self::BiparentalDiploidIndependentLoci => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParentRole {
    ClonalParent,
    ParentA,
    ParentB,
}

impl ParentRole {
    fn tag(self) -> u8 {
        match self {
            Self::ClonalParent => 0,
            Self::ParentA => 1,
            Self::ParentB => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParentHereditaryRef {
    pub role: ParentRole,
    pub hereditary_digest: HereditaryStateDigest,
}

/// Immutable derivation evidence for one child hereditary state.
///
/// Deserializing this record is not by itself current authority. Consumers that
/// need parentage authority must call `validate_current` against the exact
/// current schema, parents, operators, and child content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproductionProvenance {
    schema_digest: HereditarySchemaDigest,
    event_id: ReproductionEventId,
    mode: ReproductionMode,
    operator_digest: EvolutionOperatorProfileDigest,
    parents: Vec<ParentHereditaryRef>,
    child_digest: HereditaryStateDigest,
}

impl ReproductionProvenance {
    pub fn schema_digest(&self) -> HereditarySchemaDigest {
        self.schema_digest
    }

    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn mode(&self) -> ReproductionMode {
        self.mode
    }

    pub fn operator_digest(&self) -> EvolutionOperatorProfileDigest {
        self.operator_digest
    }

    pub fn parents(&self) -> &[ParentHereditaryRef] {
        &self.parents
    }

    pub fn child_digest(&self) -> HereditaryStateDigest {
        self.child_digest
    }

    pub fn canonical_digest(&self) -> ReproductionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(REPRODUCTION_DIGEST_DOMAIN);
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        digest.update([self.mode.tag()]);
        digest.update(self.operator_digest.as_bytes());
        put_u64(&mut digest, self.parents.len() as u64);
        for parent in &self.parents {
            digest.update([parent.role.tag()]);
            digest.update(parent.hereditary_digest.as_bytes());
        }
        digest.update(self.child_digest.as_bytes());
        ReproductionProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        parents: &[&HereditaryState],
        operators: &EvolutionOperatorProfile,
        child: &HereditaryState,
    ) -> Result<(), EvolutionError> {
        let schema_digest = schema.canonical_digest()?;
        if schema_digest != self.schema_digest {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        let operator_digest = operators.canonical_digest()?;
        if operator_digest != self.operator_digest {
            return Err(EvolutionError::OperatorAuthorityMismatch);
        }
        for parent in parents {
            parent.validate(schema)?;
        }
        child.validate(schema)?;

        let expected_parents = parent_refs(schema, parents, self.mode)?;
        if expected_parents != self.parents {
            return Err(EvolutionError::ParentageMismatch);
        }

        let child_digest = child.canonical_digest(schema)?;
        if child_digest != self.child_digest {
            return Err(EvolutionError::ChildDigestMismatch);
        }

        let recomputed = derive_child_content(
            schema,
            parents,
            &self.event_id,
            operators,
            self.mode,
        )?;
        if recomputed != *child {
            return Err(EvolutionError::ChildDerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReproductionProvenanceDigest([u8; 32]);

digest_display!(ReproductionProvenanceDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OffspringDerivation {
    pub child: HereditaryState,
    pub provenance: ReproductionProvenance,
}

/// Derive offspring hereditary content and a revalidatable provenance record
/// from exact parent state, reproduction-event identity, and versioned operators.
pub fn derive_offspring(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    mode: ReproductionMode,
) -> Result<OffspringDerivation, EvolutionError> {
    schema.validate()?;
    operators.validate()?;
    for parent in parents {
        parent.validate(schema)?;
    }

    let child = derive_child_content(schema, parents, event, operators, mode)?;
    let schema_digest = schema.canonical_digest()?;
    let operator_digest = operators.canonical_digest()?;
    let parents = parent_refs(schema, parents, mode)?;
    let child_digest = child.canonical_digest(schema)?;
    let provenance = ReproductionProvenance {
        schema_digest,
        event_id: event.clone(),
        mode,
        operator_digest,
        parents,
        child_digest,
    };
    provenance.validate_current(schema, &parent_refs_as_states_not_available(), operators, &child).err();

    // `validate_current` requires the original parent-state slice and therefore
    // is performed by consumers on restored provenance. Construction above has
    // already validated those exact parents before the record is minted.
    Ok(OffspringDerivation { child, provenance })
}

fn parent_refs_as_states_not_available<'a>() -> Vec<&'a HereditaryState> {
    // This helper is never used to establish authority. It exists only so the
    // construction path above cannot accidentally appear to self-authorize via
    // a validator that requires caller-held parent objects.
    Vec::new()
}

fn parent_refs(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    mode: ReproductionMode,
) -> Result<Vec<ParentHereditaryRef>, EvolutionError> {
    match mode {
        ReproductionMode::Clonal => {
            if parents.len() != 1 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 1,
                    observed: parents.len(),
                });
            }
            Ok(vec![ParentHereditaryRef {
                role: ParentRole::ClonalParent,
                hereditary_digest: parents[0].canonical_digest(schema)?,
            }])
        }
        ReproductionMode::BiparentalDiploidIndependentLoci => {
            if parents.len() != 2 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 2,
                    observed: parents.len(),
                });
            }
            Ok(vec![
                ParentHereditaryRef {
                    role: ParentRole::ParentA,
                    hereditary_digest: parents[0].canonical_digest(schema)?,
                },
                ParentHereditaryRef {
                    role: ParentRole::ParentB,
                    hereditary_digest: parents[1].canonical_digest(schema)?,
                },
            ])
        }
    }
}

fn derive_child_content(
    schema: &HereditarySchema,
    parents: &[&HereditaryState],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    mode: ReproductionMode,
) -> Result<HereditaryState, EvolutionError> {
    let mut child = BTreeMap::new();

    match mode {
        ReproductionMode::Clonal => {
            if parents.len() != 1 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 1,
                    observed: parents.len(),
                });
            }
            for locus_id in schema.loci.keys() {
                let source = parents[0]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus");
                let mut copies = source.clone();
                mutate_copies(schema, locus_id, &mut copies, event, operators)?;
                child.insert(locus_id.clone(), copies);
            }
        }
        ReproductionMode::BiparentalDiploidIndependentLoci => {
            if parents.len() != 2 {
                return Err(EvolutionError::ParentCountMismatch {
                    expected: 2,
                    observed: parents.len(),
                });
            }
            if schema.ploidy != 2 {
                return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
            }

            for locus_id in schema.loci.keys() {
                let parent_a = parents[0]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus");
                let parent_b = parents[1]
                    .copies
                    .get(locus_id)
                    .expect("validated parent has every schema locus");

                let a_index = draw_below(
                    event,
                    operators,
                    locus_id,
                    0,
                    DrawKind::Recombination,
                    "parent-a-copy",
                    parent_a.len() as u64,
                )? as usize;
                let b_index = draw_below(
                    event,
                    operators,
                    locus_id,
                    1,
                    DrawKind::Recombination,
                    "parent-b-copy",
                    parent_b.len() as u64,
                )? as usize;

                let mut copies = vec![parent_a[a_index].clone(), parent_b[b_index].clone()];
                mutate_copies(schema, locus_id, &mut copies, event, operators)?;
                child.insert(locus_id.clone(), copies);
            }
        }
    }

    HereditaryState::new(schema, child)
}

fn mutate_copies(
    schema: &HereditarySchema,
    locus_id: &LocusId,
    copies: &mut [AlleleId],
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
) -> Result<(), EvolutionError> {
    let locus = schema
        .loci
        .get(locus_id)
        .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;

    for (copy_index, allele) in copies.iter_mut().enumerate() {
        let mutate = draw_below(
            event,
            operators,
            locus_id,
            copy_index as u64,
            DrawKind::Mutation,
            "occurs",
            u64::from(PROBABILITY_SCALE_PPM),
        )? < u64::from(operators.mutation.per_copy_rate_ppm);

        if !mutate || locus.allowed_alleles.len() <= 1 {
            continue;
        }

        let alternatives: Vec<_> = locus
            .allowed_alleles
            .iter()
            .filter(|candidate| *candidate != allele)
            .cloned()
            .collect();
        let choice = draw_below(
            event,
            operators,
            locus_id,
            copy_index as u64,
            DrawKind::Mutation,
            "alternate",
            alternatives.len() as u64,
        )? as usize;
        *allele = alternatives[choice].clone();
    }

    Ok(())
}

/// Aggregate allele-copy state. This intentionally contains no historical
/// individual genomes; it is a population summary, not reconstructed ancestry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PopulationGeneticState {
    pub schema_version: u32,
    pub population_id: PopulationId,
    pub hereditary_schema_id: HereditarySchemaId,
    pub hereditary_schema_digest: HereditarySchemaDigest,
    pub census_individuals: u64,
    pub allele_copy_counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>>,
}

impl PopulationGeneticState {
    pub fn from_individuals(
        population_id: PopulationId,
        schema: &HereditarySchema,
        individuals: &[HereditaryState],
    ) -> Result<Self, EvolutionError> {
        schema.validate()?;
        if individuals.is_empty() {
            return Err(EvolutionError::EmptyPopulation);
        }

        let mut counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>> = schema
            .loci
            .keys()
            .cloned()
            .map(|locus| (locus, BTreeMap::new()))
            .collect();

        for individual in individuals {
            individual.validate(schema)?;
            for (locus, copies) in &individual.copies {
                let locus_counts = counts
                    .get_mut(locus)
                    .expect("validated state uses schema loci only");
                for allele in copies {
                    let count = locus_counts.entry(allele.clone()).or_insert(0);
                    *count = count
                        .checked_add(1)
                        .ok_or(EvolutionError::CountOverflow)?;
                }
            }
        }

        let census_individuals = u64::try_from(individuals.len())
            .map_err(|_| EvolutionError::CountOverflow)?;
        let state = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            population_id,
            hereditary_schema_id: schema.id.clone(),
            hereditary_schema_digest: schema.canonical_digest()?,
            census_individuals,
            allele_copy_counts: counts,
        };
        state.validate(schema)?;
        Ok(state)
    }

    /// Construct aggregate state directly from declared allele-copy counts.
    /// This does not materialize or claim historical individual genotypes.
    pub fn from_counts(
        population_id: PopulationId,
        schema: &HereditarySchema,
        census_individuals: u64,
        allele_copy_counts: BTreeMap<LocusId, BTreeMap<AlleleId, u64>>,
    ) -> Result<Self, EvolutionError> {
        let state = Self {
            schema_version: EVOLUTION_SCHEMA_VERSION,
            population_id,
            hereditary_schema_id: schema.id.clone(),
            hereditary_schema_digest: schema.canonical_digest()?,
            census_individuals,
            allele_copy_counts,
        };
        state.validate(schema)?;
        Ok(state)
    }

    pub fn validate(&self, schema: &HereditarySchema) -> Result<(), EvolutionError> {
        schema.validate()?;
        if self.schema_version != EVOLUTION_SCHEMA_VERSION {
            return Err(EvolutionError::UnsupportedSchema(self.schema_version));
        }
        if self.hereditary_schema_id != schema.id {
            return Err(EvolutionError::HereditarySchemaMismatch);
        }
        if self.hereditary_schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.census_individuals == 0 {
            return Err(EvolutionError::EmptyPopulation);
        }
        if self.allele_copy_counts.len() != schema.loci.len() {
            return Err(EvolutionError::LocusSetMismatch);
        }

        let expected_per_locus = self
            .census_individuals
            .checked_mul(u64::from(schema.ploidy))
            .ok_or(EvolutionError::CountOverflow)?;

        for (locus_id, locus) in &schema.loci {
            let counts = self
                .allele_copy_counts
                .get(locus_id)
                .ok_or_else(|| EvolutionError::MissingLocus(locus_id.clone()))?;
            let mut total = 0_u64;
            for (allele, count) in counts {
                if !locus.allowed_alleles.contains(allele) {
                    return Err(EvolutionError::UnknownAllele {
                        locus: locus_id.clone(),
                        allele: allele.clone(),
                    });
                }
                total = total
                    .checked_add(*count)
                    .ok_or(EvolutionError::CountOverflow)?;
            }
            if total != expected_per_locus {
                return Err(EvolutionError::PopulationCopyTotalMismatch {
                    locus: locus_id.clone(),
                    expected: expected_per_locus,
                    observed: total,
                });
            }
        }
        Ok(())
    }

    pub fn allele_frequency_ppm(
        &self,
        schema: &HereditarySchema,
        locus: &LocusId,
        allele: &AlleleId,
    ) -> Result<u32, EvolutionError> {
        self.validate(schema)?;
        let locus_definition = schema
            .loci
            .get(locus)
            .ok_or_else(|| EvolutionError::MissingLocus(locus.clone()))?;
        if !locus_definition.allowed_alleles.contains(allele) {
            return Err(EvolutionError::UnknownAllele {
                locus: locus.clone(),
                allele: allele.clone(),
            });
        }
        let total = self
            .census_individuals
            .checked_mul(u64::from(schema.ploidy))
            .ok_or(EvolutionError::CountOverflow)?;
        let count = self
            .allele_copy_counts
            .get(locus)
            .and_then(|counts| counts.get(allele))
            .copied()
            .unwrap_or(0);
        let ppm = count
            .checked_mul(u64::from(PROBABILITY_SCALE_PPM))
            .ok_or(EvolutionError::CountOverflow)?
            / total;
        Ok(ppm as u32)
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
    ) -> Result<PopulationGeneticStateDigest, EvolutionError> {
        self.validate(schema)?;
        let mut digest = Sha256::new();
        digest.update(POPULATION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.schema_version);
        put_text(&mut digest, self.population_id.as_str());
        put_text(&mut digest, self.hereditary_schema_id.as_str());
        digest.update(self.hereditary_schema_digest.as_bytes());
        put_u64(&mut digest, self.census_individuals);
        put_u64(&mut digest, self.allele_copy_counts.len() as u64);
        for (locus, counts) in &self.allele_copy_counts {
            put_text(&mut digest, locus.as_str());
            put_u64(&mut digest, counts.len() as u64);
            for (allele, count) in counts {
                put_text(&mut digest, allele.as_str());
                put_u64(&mut digest, *count);
            }
        }
        Ok(PopulationGeneticStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PopulationGeneticStateDigest([u8; 32]);

digest_display!(PopulationGeneticStateDigest);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvolutionError {
    UnsupportedSchema(u32),
    EmptyText { field: &'static str },
    UnsupportedPloidy(u8),
    ModeRequiresDiploid(u8),
    NoLoci,
    DuplicateLocus,
    NoAlleles { locus: LocusId },
    LocusKeyMismatch { key: LocusId, value: LocusId },
    HereditarySchemaMismatch,
    HereditarySchemaAuthorityMismatch,
    OperatorAuthorityMismatch,
    LocusSetMismatch,
    MissingLocus(LocusId),
    CopyCountMismatch {
        locus: LocusId,
        expected: u8,
        observed: usize,
    },
    UnknownAllele { locus: LocusId, allele: AlleleId },
    ParentCountMismatch { expected: usize, observed: usize },
    ParentageMismatch,
    ChildDigestMismatch,
    ChildDerivationMismatch,
    ProbabilityOutOfRange { observed_ppm: u32 },
    EmptyPopulation,
    PopulationCopyTotalMismatch {
        locus: LocusId,
        expected: u64,
        observed: u64,
    },
    CountOverflow,
    InvalidDrawUpperBound,
}

impl fmt::Display for EvolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema(version) => write!(f, "unsupported evolution schema {version}"),
            Self::EmptyText { field } => write!(f, "{field} must not be empty"),
            Self::UnsupportedPloidy(ploidy) => write!(f, "unsupported ploidy {ploidy}"),
            Self::ModeRequiresDiploid(ploidy) => {
                write!(f, "biparental V0 mode requires diploid schema, observed {ploidy}")
            }
            Self::NoLoci => write!(f, "hereditary schema must contain at least one locus"),
            Self::DuplicateLocus => write!(f, "duplicate locus identity"),
            Self::NoAlleles { locus } => write!(f, "locus {} has no allowed alleles", locus.as_str()),
            Self::LocusKeyMismatch { key, value } => write!(
                f,
                "locus map key {} does not match value {}",
                key.as_str(),
                value.as_str()
            ),
            Self::HereditarySchemaMismatch => write!(f, "hereditary schema id mismatch"),
            Self::HereditarySchemaAuthorityMismatch => {
                write!(f, "exact hereditary schema authority mismatch")
            }
            Self::OperatorAuthorityMismatch => write!(f, "exact evolution operator authority mismatch"),
            Self::LocusSetMismatch => write!(f, "hereditary locus set mismatch"),
            Self::MissingLocus(locus) => write!(f, "missing locus {}", locus.as_str()),
            Self::CopyCountMismatch {
                locus,
                expected,
                observed,
            } => write!(
                f,
                "locus {} expected {expected} copies, observed {observed}",
                locus.as_str()
            ),
            Self::UnknownAllele { locus, allele } => write!(
                f,
                "allele {} is not allowed at locus {}",
                allele.as_str(),
                locus.as_str()
            ),
            Self::ParentCountMismatch { expected, observed } => {
                write!(f, "expected {expected} parent(s), observed {observed}")
            }
            Self::ParentageMismatch => write!(f, "reproduction parentage/provenance mismatch"),
            Self::ChildDigestMismatch => write!(f, "reproduction child digest mismatch"),
            Self::ChildDerivationMismatch => write!(f, "reproduction child derivation mismatch"),
            Self::ProbabilityOutOfRange { observed_ppm } => write!(
                f,
                "probability {observed_ppm} ppm exceeds {PROBABILITY_SCALE_PPM} ppm"
            ),
            Self::EmptyPopulation => write!(f, "population must contain at least one individual"),
            Self::PopulationCopyTotalMismatch {
                locus,
                expected,
                observed,
            } => write!(
                f,
                "locus {} expected {expected} allele copies, observed {observed}",
                locus.as_str()
            ),
            Self::CountOverflow => write!(f, "population/genetic count overflow"),
            Self::InvalidDrawUpperBound => write!(f, "semantic draw upper bound must be positive"),
        }
    }
}

impl Error for EvolutionError {}

#[derive(Debug, Clone, Copy)]
enum DrawKind {
    Mutation,
    Recombination,
}

fn validate_text(field: &'static str, value: &str) -> Result<(), EvolutionError> {
    if value.trim().is_empty() {
        Err(EvolutionError::EmptyText { field })
    } else {
        Ok(())
    }
}

fn draw_below(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    locus: &LocusId,
    copy_index: u64,
    kind: DrawKind,
    purpose: &str,
    upper: u64,
) -> Result<u64, EvolutionError> {
    if upper == 0 {
        return Err(EvolutionError::InvalidDrawUpperBound);
    }
    if upper == 1 {
        return Ok(0);
    }

    let zone = u64::MAX - (u64::MAX % upper);
    let mut attempt = 0_u64;
    loop {
        let value = semantic_draw_u64(
            event,
            operators,
            locus,
            copy_index,
            kind,
            purpose,
            attempt,
        )?;
        if value < zone {
            return Ok(value % upper);
        }
        attempt = attempt
            .checked_add(1)
            .ok_or(EvolutionError::CountOverflow)?;
    }
}

fn semantic_draw_u64(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    locus: &LocusId,
    copy_index: u64,
    kind: DrawKind,
    purpose: &str,
    attempt: u64,
) -> Result<u64, EvolutionError> {
    operators.validate()?;
    let mut digest = Sha256::new();
    digest.update(RNG_DOMAIN);
    put_text(&mut digest, event.as_str());
    put_text(&mut digest, operators.profile_id.as_str());
    put_text(&mut digest, &operators.version);
    match kind {
        DrawKind::Mutation => operators.mutation.put_randomness_identity(&mut digest),
        DrawKind::Recombination => operators.recombination.put_randomness_identity(&mut digest),
    }
    put_text(&mut digest, locus.as_str());
    put_u64(&mut digest, copy_index);
    put_text(&mut digest, purpose);
    put_u64(&mut digest, attempt);
    let bytes: [u8; 32] = digest.finalize().into();
    Ok(u64::from_le_bytes(
        bytes[..8]
            .try_into()
            .expect("SHA-256 output has an 8-byte prefix"),
    ))
}

fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allele(id: &str) -> AlleleId {
        AlleleId::new(id).expect("valid allele id")
    }

    fn locus(id: &str, alleles: &[&str]) -> LocusDefinition {
        LocusDefinition::new(
            LocusId::new(id).expect("valid locus id"),
            alleles.iter().map(|id| allele(id)),
        )
        .expect("valid locus")
    }

    fn schema_with_id(id: &str, locus_defs: Vec<LocusDefinition>) -> HereditarySchema {
        HereditarySchema::new(
            HereditarySchemaId::new(id).expect("valid schema id"),
            2,
            locus_defs,
        )
        .expect("valid schema")
    }

    fn schema(locus_defs: Vec<LocusDefinition>) -> HereditarySchema {
        schema_with_id("reference-diploid", locus_defs)
    }

    fn state(schema: &HereditarySchema, values: &[(&str, &[&str])]) -> HereditaryState {
        let copies = values
            .iter()
            .map(|(locus_id, alleles)| {
                (
                    LocusId::new(*locus_id).expect("valid locus id"),
                    alleles.iter().map(|id| allele(id)).collect(),
                )
            })
            .collect();
        HereditaryState::new(schema, copies).expect("valid hereditary state")
    }

    fn operators(rate_ppm: u32, version: &str) -> EvolutionOperatorProfile {
        EvolutionOperatorProfile {
            profile_id: OperatorProfileId::new("reference-independent-loci")
                .expect("valid operator id"),
            version: version.to_owned(),
            mutation: MutationProfile {
                model_id: "point-substitution".to_owned(),
                version: "v1".to_owned(),
                per_copy_rate_ppm: rate_ppm,
            },
            recombination: RecombinationProfile {
                model_id: "independent-loci".to_owned(),
                version: "v1".to_owned(),
                mode: RecombinationMode::IndependentLoci,
            },
        }
    }

    #[test]
    fn clonal_replay_is_exact_and_provenance_revalidates() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let parent = state(&schema, &[("pigment", &["dark", "light"])]);
        let event = ReproductionEventId::new("birth-42").expect("valid event");
        let operators = operators(25_000, "v1");

        let first = derive_offspring(
            &schema,
            &[&parent],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .expect("offspring");
        let second = derive_offspring(
            &schema,
            &[&parent],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .expect("offspring");

        assert_eq!(first, second);
        first
            .provenance
            .validate_current(&schema, &[&parent], &operators, &first.child)
            .expect("current provenance");
        assert_eq!(
            first.provenance.canonical_digest(),
            second.provenance.canonical_digest()
        );
    }

    #[test]
    fn same_schema_id_different_content_stales_old_state() {
        let schema_a = schema_with_id(
            "stable-schema-name",
            vec![locus("pigment", &["dark", "light"])],
        );
        let schema_b = schema_with_id(
            "stable-schema-name",
            vec![locus("pigment", &["dark", "light", "red"])],
        );
        let hereditary = state(&schema_a, &[("pigment", &["dark", "light"])]);

        assert_eq!(
            hereditary.validate(&schema_b),
            Err(EvolutionError::HereditarySchemaAuthorityMismatch)
        );
    }

    #[test]
    fn same_operator_labels_different_content_change_exact_authority() {
        let a = operators(1_000, "v1");
        let b = operators(2_000, "v1");
        assert_ne!(
            a.canonical_digest().expect("digest"),
            b.canonical_digest().expect("digest")
        );
    }

    #[test]
    fn mutation_rate_sweep_does_not_reroll_mutation_variate() {
        let event = ReproductionEventId::new("rate-sweep").unwrap();
        let locus = LocusId::new("pigment").unwrap();
        let low = operators(1_000, "v1");
        let high = operators(900_000, "v1");
        let a = semantic_draw_u64(
            &event,
            &low,
            &locus,
            0,
            DrawKind::Mutation,
            "occurs",
            0,
        )
        .unwrap();
        let b = semantic_draw_u64(
            &event,
            &high,
            &locus,
            0,
            DrawKind::Mutation,
            "occurs",
            0,
        )
        .unwrap();
        assert_eq!(a, b);
        assert_ne!(
            low.canonical_digest().unwrap(),
            high.canonical_digest().unwrap()
        );
    }

    #[test]
    fn unrelated_locus_does_not_shift_existing_locus_draw() {
        let schema_a = schema(vec![locus("pigment", &["dark", "light"])]);
        let schema_ab = schema(vec![
            locus("pigment", &["dark", "light"]),
            locus("enzyme", &["slow", "fast"]),
        ]);
        assert_ne!(
            schema_a.canonical_digest().expect("digest"),
            schema_ab.canonical_digest().expect("digest")
        );

        let a1 = state(&schema_a, &[("pigment", &["dark", "light"])]);
        let a2 = state(&schema_a, &[("pigment", &["light", "light"])]);
        let ab1 = state(
            &schema_ab,
            &[
                ("pigment", &["dark", "light"]),
                ("enzyme", &["slow", "fast"]),
            ],
        );
        let ab2 = state(
            &schema_ab,
            &[
                ("pigment", &["light", "light"]),
                ("enzyme", &["fast", "fast"]),
            ],
        );
        let event = ReproductionEventId::new("birth-keyed-loci").expect("valid event");
        let operators = operators(0, "v1");

        let child_a = derive_offspring(
            &schema_a,
            &[&a1, &a2],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .expect("offspring");
        let child_ab = derive_offspring(
            &schema_ab,
            &[&ab1, &ab2],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .expect("offspring");

        let pigment = LocusId::new("pigment").unwrap();
        assert_eq!(
            child_a.child.copies.get(&pigment),
            child_ab.child.copies.get(&pigment)
        );
    }

    #[test]
    fn parent_role_swap_changes_provenance() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let first_parent = state(&schema, &[("pigment", &["dark", "dark"])]);
        let second_parent = state(&schema, &[("pigment", &["light", "light"])]);
        let event = ReproductionEventId::new("role-sensitive-birth").unwrap();
        let operators = operators(0, "v1");

        let ab = derive_offspring(
            &schema,
            &[&first_parent, &second_parent],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();
        let ba = derive_offspring(
            &schema,
            &[&second_parent, &first_parent],
            &event,
            &operators,
            ReproductionMode::BiparentalDiploidIndependentLoci,
        )
        .unwrap();

        assert_ne!(
            ab.provenance.canonical_digest(),
            ba.provenance.canonical_digest()
        );
    }

    #[test]
    fn changed_parent_content_invalidates_provenance() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let original = state(&schema, &[("pigment", &["dark", "light"])]);
        let replacement = state(&schema, &[("pigment", &["light", "light"])]);
        let event = ReproductionEventId::new("parentage-check").unwrap();
        let operators = operators(0, "v1");
        let derivation = derive_offspring(
            &schema,
            &[&original],
            &event,
            &operators,
            ReproductionMode::Clonal,
        )
        .unwrap();

        assert_eq!(
            derivation.provenance.validate_current(
                &schema,
                &[&replacement],
                &operators,
                &derivation.child,
            ),
            Err(EvolutionError::ParentageMismatch)
        );
    }

    #[test]
    fn aggregate_population_counts_are_exact() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let first = state(&schema, &[("pigment", &["dark", "light"])]);
        let second = state(&schema, &[("pigment", &["dark", "dark"])]);
        let population = PopulationGeneticState::from_individuals(
            PopulationId::new("island-a").expect("valid population"),
            &schema,
            &[first, second],
        )
        .expect("population");

        let locus = LocusId::new("pigment").unwrap();
        assert_eq!(
            population
                .allele_frequency_ppm(&schema, &locus, &allele("dark"))
                .unwrap(),
            750_000
        );
        assert_eq!(
            population
                .allele_frequency_ppm(&schema, &locus, &allele("light"))
                .unwrap(),
            250_000
        );
    }

    #[test]
    fn aggregate_counts_cannot_invent_or_drop_allele_copies() {
        let schema = schema(vec![locus("pigment", &["dark", "light"])]);
        let mut counts = BTreeMap::new();
        counts.insert(
            LocusId::new("pigment").unwrap(),
            BTreeMap::from([(allele("dark"), 2), (allele("light"), 1)]),
        );

        let result = PopulationGeneticState::from_counts(
            PopulationId::new("island-a").unwrap(),
            &schema,
            2,
            counts,
        );
        assert!(matches!(
            result,
            Err(EvolutionError::PopulationCopyTotalMismatch { .. })
        ));
    }

    #[test]
    fn invalid_mutation_probability_fails_closed() {
        let result = operators(PROBABILITY_SCALE_PPM + 1, "v1").validate();
        assert!(matches!(
            result,
            Err(EvolutionError::ProbabilityOutOfRange { .. })
        ));
    }
}
