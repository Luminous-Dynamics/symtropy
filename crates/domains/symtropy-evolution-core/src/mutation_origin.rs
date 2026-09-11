use crate::{
    assemble_diploid_linked_offspring_from_evidence,
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::parent_role_tag,
    AlleleId, AncestryCopyId, AncestryGeneration, AncestryGraphError, ChromosomeId,
    ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    DescendantAncestryDerivation, DescendantAncestryDerivationProvenanceDigest,
    DescendantAncestryError, EvolutionError, EvolutionOperatorProfile,
    EvolutionOperatorProfileDigest, GameteAncestryDerivation, HereditarySchema,
    HereditarySchemaDigest, LinkedGameteDerivationEvidence,
    LinkedGameteDerivationEvidenceDigest, LocusId, ModeledAncestryGraph, ParentRole,
    PhasedAncestryState, PhasedHereditaryState, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const MUTATION_ORIGIN_VERSION: u32 = 1;
const MUTATION_ORIGIN_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:mutation-origin:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationOriginTiming {
    AtDescendantBirthV1,
}

impl MutationOriginTiming {
    fn tag(self) -> u8 {
        match self {
            Self::AtDescendantBirthV1 => 0,
        }
    }
}

/// Exact evidence that one modeled-locus allele substitution is declared to
/// originate on one persistent descendant chromosome copy at birth.
///
/// V1 is an origin/provenance authority only. It deliberately does not claim
/// that the mutation was sampled by the operator RNG, does not apply the
/// derived allele to genomic state, and contains no fitness/selection fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationOrigin {
    origin_version: u32,
    timing: MutationOriginTiming,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    operator_digest: EvolutionOperatorProfileDigest,
    descendant_provenance_digest: DescendantAncestryDerivationProvenanceDigest,
    parent_gamete_evidence_digest: LinkedGameteDerivationEvidenceDigest,
    reproduction_event_id: ReproductionEventId,
    ancestry_generation: AncestryGeneration,
    ancestry_copy_id: AncestryCopyId,
    chromosome_id: ChromosomeId,
    locus_id: LocusId,
    parent_role: ParentRole,
    ancestral_allele: AlleleId,
    derived_allele: AlleleId,
}

impl MutationOrigin {
    pub fn timing(&self) -> MutationOriginTiming {
        self.timing
    }

    pub fn ancestry_copy_id(&self) -> &AncestryCopyId {
        &self.ancestry_copy_id
    }

    pub fn chromosome_id(&self) -> &ChromosomeId {
        &self.chromosome_id
    }

    pub fn locus_id(&self) -> &LocusId {
        &self.locus_id
    }

    pub fn parent_role(&self) -> ParentRole {
        self.parent_role
    }

    pub fn ancestral_allele(&self) -> &AlleleId {
        &self.ancestral_allele
    }

    pub fn derived_allele(&self) -> &AlleleId {
        &self.derived_allele
    }

    pub fn ancestry_generation(&self) -> AncestryGeneration {
        self.ancestry_generation
    }

    pub fn reproduction_event_id(&self) -> &ReproductionEventId {
        &self.reproduction_event_id
    }

    pub fn canonical_digest(&self) -> MutationOriginDigest {
        let mut digest = Sha256::new();
        digest.update(MUTATION_ORIGIN_DIGEST_DOMAIN);
        put_u32(&mut digest, self.origin_version);
        digest.update([self.timing.tag()]);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.operator_digest.as_bytes());
        digest.update(self.descendant_provenance_digest.as_bytes());
        digest.update(self.parent_gamete_evidence_digest.as_bytes());
        put_text(&mut digest, self.reproduction_event_id.as_str());
        put_u64(&mut digest, self.ancestry_generation.get());
        put_text(&mut digest, self.ancestry_copy_id.as_str());
        put_text(&mut digest, self.chromosome_id.as_str());
        put_text(&mut digest, self.locus_id.as_str());
        digest.update([parent_role_tag(self.parent_role)]);
        put_text(&mut digest, self.ancestral_allele.as_str());
        put_text(&mut digest, self.derived_allele.as_str());
        MutationOriginDigest(digest.finalize().into())
    }

    /// Restore-time authority must be re-earned from the exact current
    /// reproduction, ancestry, graph-node, and operator authorities.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        operators: &EvolutionOperatorProfile,
        parent_a_source: &PhasedHereditaryState,
        parent_a_source_ancestry: &PhasedAncestryState,
        parent_a_profile: &ChromosomeRecombinationProfile,
        parent_a_gamete: &LinkedGameteDerivationEvidence,
        parent_a_gamete_ancestry: &GameteAncestryDerivation,
        parent_b_source: &PhasedHereditaryState,
        parent_b_source_ancestry: &PhasedAncestryState,
        parent_b_profile: &ChromosomeRecombinationProfile,
        parent_b_gamete: &LinkedGameteDerivationEvidence,
        parent_b_gamete_ancestry: &GameteAncestryDerivation,
        descendant: &DescendantAncestryDerivation,
        graph: &ModeledAncestryGraph,
    ) -> Result<(), MutationOriginError> {
        if self.origin_version != MUTATION_ORIGIN_VERSION
            || self.timing != MutationOriginTiming::AtDescendantBirthV1
        {
            return Err(MutationOriginError::UnsupportedVersion(self.origin_version));
        }
        let recomputed = declare_at_birth_mutation_origin(
            schema,
            chromosome_map,
            operators,
            parent_a_source,
            parent_a_source_ancestry,
            parent_a_profile,
            parent_a_gamete,
            parent_a_gamete_ancestry,
            parent_b_source,
            parent_b_source_ancestry,
            parent_b_profile,
            parent_b_gamete,
            parent_b_gamete_ancestry,
            descendant,
            graph,
            &self.ancestry_copy_id,
            &self.locus_id,
            &self.derived_allele,
        )?;
        if recomputed != *self {
            return Err(MutationOriginError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationOriginDigest([u8; 32]);

impl MutationOriginDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MutationOriginDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutationOriginDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for MutationOriginDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn declare_at_birth_mutation_origin(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    operators: &EvolutionOperatorProfile,
    parent_a_source: &PhasedHereditaryState,
    parent_a_source_ancestry: &PhasedAncestryState,
    parent_a_profile: &ChromosomeRecombinationProfile,
    parent_a_gamete: &LinkedGameteDerivationEvidence,
    parent_a_gamete_ancestry: &GameteAncestryDerivation,
    parent_b_source: &PhasedHereditaryState,
    parent_b_source_ancestry: &PhasedAncestryState,
    parent_b_profile: &ChromosomeRecombinationProfile,
    parent_b_gamete: &LinkedGameteDerivationEvidence,
    parent_b_gamete_ancestry: &GameteAncestryDerivation,
    descendant: &DescendantAncestryDerivation,
    graph: &ModeledAncestryGraph,
    ancestry_copy_id: &AncestryCopyId,
    locus_id: &LocusId,
    derived_allele: &AlleleId,
) -> Result<MutationOrigin, MutationOriginError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    operators.validate()?;
    graph.validate_current(schema, chromosome_map)?;

    let event = &descendant.materialization.reproduction_event_id;
    let offspring = assemble_diploid_linked_offspring_from_evidence(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_profile,
        parent_a_gamete,
        parent_b_source,
        parent_b_profile,
        parent_b_gamete,
        event,
    )?;
    descendant.provenance.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_source_ancestry,
        parent_a_profile,
        parent_a_gamete,
        parent_a_gamete_ancestry,
        parent_b_source,
        parent_b_source_ancestry,
        parent_b_profile,
        parent_b_gamete,
        parent_b_gamete_ancestry,
        &offspring,
        event,
        &descendant.child_ancestry,
        &descendant.materialization,
    )?;

    let copy = descendant
        .materialization
        .descendant_copies
        .iter()
        .find(|copy| &copy.child_copy_id == ancestry_copy_id)
        .ok_or_else(|| MutationOriginError::DescendantCopyMissing(ancestry_copy_id.clone()))?;

    let node = graph
        .nodes
        .get(ancestry_copy_id)
        .ok_or_else(|| MutationOriginError::GraphNodeMissing(ancestry_copy_id.clone()))?;
    if node.birth_event.as_ref() != Some(event) {
        return Err(MutationOriginError::BirthEventMismatch);
    }
    if node.chromosome_id != copy.chromosome_id {
        return Err(MutationOriginError::ChromosomeMismatch);
    }

    let definition = chromosome_map
        .chromosomes
        .get(&copy.chromosome_id)
        .ok_or_else(|| MutationOriginError::ChromosomeMismatch)?;
    let locus_index = definition
        .loci
        .iter()
        .position(|mapped| &mapped.locus_id == locus_id)
        .ok_or_else(|| MutationOriginError::LocusNotOnChromosome(locus_id.clone()))?;
    let locus = schema
        .loci
        .get(locus_id)
        .ok_or_else(|| MutationOriginError::LocusNotOnChromosome(locus_id.clone()))?;
    if !locus.allowed_alleles.contains(derived_allele) {
        return Err(MutationOriginError::DerivedAlleleNotAllowed(derived_allele.clone()));
    }

    let parent_gamete = match copy.parent_role {
        ParentRole::ParentA => parent_a_gamete,
        ParentRole::ParentB => parent_b_gamete,
        ParentRole::ClonalParent => return Err(MutationOriginError::UnsupportedParentRole),
    };
    let gamete_chromosome = parent_gamete
        .gamete()
        .chromosomes
        .get(&copy.chromosome_id)
        .ok_or(MutationOriginError::ChromosomeMismatch)?;
    let ancestral_allele = gamete_chromosome
        .alleles
        .get(locus_index)
        .ok_or_else(|| MutationOriginError::LocusNotOnChromosome(locus_id.clone()))?
        .clone();
    if ancestral_allele == *derived_allele {
        return Err(MutationOriginError::NoAllelicChange);
    }

    Ok(MutationOrigin {
        origin_version: MUTATION_ORIGIN_VERSION,
        timing: MutationOriginTiming::AtDescendantBirthV1,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        operator_digest: operators.canonical_digest()?,
        descendant_provenance_digest: descendant.provenance.canonical_digest(),
        parent_gamete_evidence_digest: parent_gamete.canonical_digest(),
        reproduction_event_id: event.clone(),
        ancestry_generation: node.generation,
        ancestry_copy_id: ancestry_copy_id.clone(),
        chromosome_id: copy.chromosome_id.clone(),
        locus_id: locus_id.clone(),
        parent_role: copy.parent_role,
        ancestral_allele,
        derived_allele: derived_allele.clone(),
    })
}

#[derive(Debug)]
pub enum MutationOriginError {
    Evolution(EvolutionError),
    Descendant(DescendantAncestryError),
    Graph(AncestryGraphError),
    UnsupportedVersion(u32),
    DescendantCopyMissing(AncestryCopyId),
    GraphNodeMissing(AncestryCopyId),
    BirthEventMismatch,
    ChromosomeMismatch,
    LocusNotOnChromosome(LocusId),
    DerivedAlleleNotAllowed(AlleleId),
    NoAllelicChange,
    UnsupportedParentRole,
    ReplayMismatch,
}

impl From<EvolutionError> for MutationOriginError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}
impl From<DescendantAncestryError> for MutationOriginError {
    fn from(value: DescendantAncestryError) -> Self {
        Self::Descendant(value)
    }
}
impl From<AncestryGraphError> for MutationOriginError {
    fn from(value: AncestryGraphError) -> Self {
        Self::Graph(value)
    }
}

impl fmt::Display for MutationOriginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Descendant(error) => write!(f, "descendant ancestry authority error: {error}"),
            Self::Graph(error) => write!(f, "ancestry graph authority error: {error}"),
            Self::UnsupportedVersion(version) => write!(f, "unsupported mutation-origin version {version}"),
            Self::DescendantCopyMissing(id) => write!(f, "descendant ancestry copy {id:?} is absent"),
            Self::GraphNodeMissing(id) => write!(f, "ancestry graph node {id:?} is absent"),
            Self::BirthEventMismatch => write!(f, "mutation origin does not match the ancestry-copy birth event"),
            Self::ChromosomeMismatch => write!(f, "mutation origin chromosome authority mismatch"),
            Self::LocusNotOnChromosome(id) => write!(f, "locus {id:?} is not modeled on the descendant chromosome"),
            Self::DerivedAlleleNotAllowed(id) => write!(f, "derived allele {id:?} is not allowed by the hereditary schema"),
            Self::NoAllelicChange => write!(f, "mutation origin requires ancestral and derived alleles to differ"),
            Self::UnsupportedParentRole => write!(f, "MUT-05A supports linked ParentA/ParentB descendants only"),
            Self::ReplayMismatch => write!(f, "restored mutation-origin evidence does not deterministically replay"),
        }
    }
}

impl Error for MutationOriginError {}
