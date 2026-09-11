use crate::{
    assemble_diploid_linked_offspring_from_evidence,
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::parent_role_tag,
    declare_at_birth_mutation_origin, AlleleId, AncestryCopyId, AncestryGraphError,
    ChromosomeId, ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    DescendantAncestryDerivation, DescendantAncestryDerivationProvenanceDigest,
    DescendantAncestryError, EvolutionError, EvolutionOperatorProfile,
    EvolutionOperatorProfileDigest, GameteAncestryDerivation, HereditarySchema,
    HereditarySchemaDigest, LinkedGameteDerivationEvidence, LocusId, ModeledAncestryGraph,
    MutationOrigin, MutationOriginError, ParentRole, PhasedAncestryState,
    PhasedChromosomeState, PhasedHereditaryState, PhasedHereditaryStateDigest,
    ReproductionEventId, PROBABILITY_SCALE_PPM,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const LINKED_MUTATION_EXECUTION_VERSION: u32 = 1;
const LINKED_MUTATION_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-mutation-semantic-draw:v1\0";
const LINKED_MUTATION_OPPORTUNITY_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-mutation-opportunity:v1\0";
const LINKED_MUTATION_EXECUTION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-mutation-execution:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoMutationReason {
    RateMiss,
    NoAlternativeAllele,
}

impl NoMutationReason {
    fn tag(self) -> u8 {
        match self {
            Self::RateMiss => 0,
            Self::NoAlternativeAllele => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkedMutationOutcome {
    NoMutation { reason: NoMutationReason },
    Substitution {
        alternate_draw_index: u64,
        origin: MutationOrigin,
    },
}

/// One exact mutation opportunity on one persistent descendant chromosome copy.
///
/// `occurrence_draw_ppm` is retained even for no-mutation outcomes so later
/// calibration can audit the full opportunity census instead of observing only
/// successful substitutions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedMutationOpportunity {
    pub ancestry_copy_id: AncestryCopyId,
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub parent_role: ParentRole,
    pub ancestral_allele: AlleleId,
    pub occurrence_draw_ppm: u64,
    pub outcome: LinkedMutationOutcome,
}

impl LinkedMutationOpportunity {
    pub fn canonical_digest(&self) -> LinkedMutationOpportunityDigest {
        let mut digest = Sha256::new();
        digest.update(LINKED_MUTATION_OPPORTUNITY_DIGEST_DOMAIN);
        put_text(&mut digest, self.ancestry_copy_id.as_str());
        put_text(&mut digest, self.chromosome_id.as_str());
        put_text(&mut digest, self.locus_id.as_str());
        digest.update([parent_role_tag(self.parent_role)]);
        put_text(&mut digest, self.ancestral_allele.as_str());
        put_u64(&mut digest, self.occurrence_draw_ppm);
        match &self.outcome {
            LinkedMutationOutcome::NoMutation { reason } => {
                digest.update([0, reason.tag()]);
            }
            LinkedMutationOutcome::Substitution {
                alternate_draw_index,
                origin,
            } => {
                digest.update([1]);
                put_u64(&mut digest, *alternate_draw_index);
                digest.update(origin.canonical_digest().as_bytes());
            }
        }
        LinkedMutationOpportunityDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedMutationOpportunityDigest([u8; 32]);

impl LinkedMutationOpportunityDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedMutationOpportunityDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedMutationOpportunityDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedMutationOpportunityDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

/// Exact stochastic mutation execution for one linked diploid descendant.
///
/// V1 contains no phenotype, fitness, or selection authority. It binds the
/// unmutated descendant ancestry, the exact operator profile, every mutation
/// opportunity (including no-ops), and the resulting canonical phased child.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedMutationExecution {
    execution_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    operator_digest: EvolutionOperatorProfileDigest,
    descendant_provenance_digest: DescendantAncestryDerivationProvenanceDigest,
    reproduction_event_id: ReproductionEventId,
    unmutated_child_digest: PhasedHereditaryStateDigest,
    mutated_child_digest: PhasedHereditaryStateDigest,
    pub opportunities: Vec<LinkedMutationOpportunity>,
    pub mutated_child: PhasedHereditaryState,
}

impl LinkedMutationExecution {
    pub fn reproduction_event_id(&self) -> &ReproductionEventId {
        &self.reproduction_event_id
    }

    pub fn unmutated_child_digest(&self) -> PhasedHereditaryStateDigest {
        self.unmutated_child_digest
    }

    pub fn mutated_child_digest(&self) -> PhasedHereditaryStateDigest {
        self.mutated_child_digest
    }

    pub fn canonical_digest(&self) -> LinkedMutationExecutionDigest {
        let mut digest = Sha256::new();
        digest.update(LINKED_MUTATION_EXECUTION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.execution_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.operator_digest.as_bytes());
        digest.update(self.descendant_provenance_digest.as_bytes());
        put_text(&mut digest, self.reproduction_event_id.as_str());
        digest.update(self.unmutated_child_digest.as_bytes());
        digest.update(self.mutated_child_digest.as_bytes());
        put_u64(&mut digest, self.opportunities.len() as u64);
        for opportunity in &self.opportunities {
            digest.update(opportunity.canonical_digest().as_bytes());
        }
        LinkedMutationExecutionDigest(digest.finalize().into())
    }

    /// Restore-time authority is earned only by exact deterministic replay from
    /// current linked-gamete, ancestry, graph, and operator authorities.
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
    ) -> Result<(), LinkedMutationError> {
        if self.execution_version != LINKED_MUTATION_EXECUTION_VERSION {
            return Err(LinkedMutationError::UnsupportedVersion(
                self.execution_version,
            ));
        }
        let replayed = execute_linked_mutations(
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
        )?;
        if replayed != *self {
            return Err(LinkedMutationError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedMutationExecutionDigest([u8; 32]);

impl LinkedMutationExecutionDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedMutationExecutionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedMutationExecutionDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedMutationExecutionDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[allow(clippy::too_many_arguments)]
pub fn execute_linked_mutations(
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
) -> Result<LinkedMutationExecution, LinkedMutationError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    operators.validate()?;
    graph.validate_current(schema, chromosome_map)?;

    let event = &descendant.materialization.reproduction_event_id;
    let unmutated = assemble_diploid_linked_offspring_from_evidence(
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
        &unmutated,
        event,
        &descendant.child_ancestry,
        &descendant.materialization,
    )?;

    let mut opportunities = Vec::new();
    let mut mutated_chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        let mut from_a = parent_a_gamete
            .gamete()
            .chromosomes
            .get(chromosome_id)
            .ok_or_else(|| LinkedMutationError::ChromosomeMissing(chromosome_id.clone()))?
            .clone();
        let mut from_b = parent_b_gamete
            .gamete()
            .chromosomes
            .get(chromosome_id)
            .ok_or_else(|| LinkedMutationError::ChromosomeMissing(chromosome_id.clone()))?
            .clone();

        let copy_a = descendant
            .materialization
            .descendant_copies
            .iter()
            .find(|copy| {
                copy.chromosome_id == *chromosome_id && copy.parent_role == ParentRole::ParentA
            })
            .ok_or_else(|| LinkedMutationError::DescendantCopyMissing {
                chromosome: chromosome_id.clone(),
                role: ParentRole::ParentA,
            })?;
        let copy_b = descendant
            .materialization
            .descendant_copies
            .iter()
            .find(|copy| {
                copy.chromosome_id == *chromosome_id && copy.parent_role == ParentRole::ParentB
            })
            .ok_or_else(|| LinkedMutationError::DescendantCopyMissing {
                chromosome: chromosome_id.clone(),
                role: ParentRole::ParentB,
            })?;

        execute_copy_opportunities(
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
            event,
            chromosome_id,
            definition,
            copy_a.child_copy_id.clone(),
            ParentRole::ParentA,
            &mut from_a.alleles,
            &mut opportunities,
        )?;
        execute_copy_opportunities(
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
            event,
            chromosome_id,
            definition,
            copy_b.child_copy_id.clone(),
            ParentRole::ParentB,
            &mut from_b.alleles,
            &mut opportunities,
        )?;

        mutated_chromosomes.push(PhasedChromosomeState::new(
            chromosome_id.clone(),
            vec![from_a, from_b],
        ));
    }

    let mutated_child = PhasedHereditaryState::new(schema, chromosome_map, mutated_chromosomes)?;
    let value = LinkedMutationExecution {
        execution_version: LINKED_MUTATION_EXECUTION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        operator_digest: operators.canonical_digest()?,
        descendant_provenance_digest: descendant.provenance.canonical_digest(),
        reproduction_event_id: event.clone(),
        unmutated_child_digest: unmutated.child.canonical_digest(schema, chromosome_map)?,
        mutated_child_digest: mutated_child.canonical_digest(schema, chromosome_map)?,
        opportunities,
        mutated_child,
    };
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
fn execute_copy_opportunities(
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
    event: &ReproductionEventId,
    chromosome_id: &ChromosomeId,
    definition: &crate::ChromosomeDefinition,
    ancestry_copy_id: AncestryCopyId,
    parent_role: ParentRole,
    alleles: &mut [AlleleId],
    opportunities: &mut Vec<LinkedMutationOpportunity>,
) -> Result<(), LinkedMutationError> {
    if alleles.len() != definition.loci.len() {
        return Err(LinkedMutationError::LocusSequenceMismatch {
            chromosome: chromosome_id.clone(),
        });
    }

    for (locus_index, mapped_locus) in definition.loci.iter().enumerate() {
        let ancestral_allele = alleles[locus_index].clone();
        let occurrence_draw_ppm = linked_draw_below(
            event,
            operators,
            &ancestry_copy_id,
            chromosome_id,
            &mapped_locus.locus_id,
            "occurs",
            u64::from(PROBABILITY_SCALE_PPM),
        )?;
        let locus = schema
            .loci
            .get(&mapped_locus.locus_id)
            .ok_or_else(|| EvolutionError::MissingLocus(mapped_locus.locus_id.clone()))?;

        if occurrence_draw_ppm >= u64::from(operators.mutation.per_copy_rate_ppm) {
            opportunities.push(LinkedMutationOpportunity {
                ancestry_copy_id: ancestry_copy_id.clone(),
                chromosome_id: chromosome_id.clone(),
                locus_id: mapped_locus.locus_id.clone(),
                parent_role,
                ancestral_allele,
                occurrence_draw_ppm,
                outcome: LinkedMutationOutcome::NoMutation {
                    reason: NoMutationReason::RateMiss,
                },
            });
            continue;
        }

        let alternatives: Vec<_> = locus
            .allowed_alleles
            .iter()
            .filter(|candidate| *candidate != &ancestral_allele)
            .cloned()
            .collect();
        if alternatives.is_empty() {
            opportunities.push(LinkedMutationOpportunity {
                ancestry_copy_id: ancestry_copy_id.clone(),
                chromosome_id: chromosome_id.clone(),
                locus_id: mapped_locus.locus_id.clone(),
                parent_role,
                ancestral_allele,
                occurrence_draw_ppm,
                outcome: LinkedMutationOutcome::NoMutation {
                    reason: NoMutationReason::NoAlternativeAllele,
                },
            });
            continue;
        }

        let alternate_draw_index = linked_draw_below(
            event,
            operators,
            &ancestry_copy_id,
            chromosome_id,
            &mapped_locus.locus_id,
            "alternate",
            alternatives.len() as u64,
        )?;
        let derived_allele = alternatives[alternate_draw_index as usize].clone();
        let origin = declare_at_birth_mutation_origin(
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
            &ancestry_copy_id,
            &mapped_locus.locus_id,
            &derived_allele,
        )?;
        alleles[locus_index] = derived_allele;
        opportunities.push(LinkedMutationOpportunity {
            ancestry_copy_id: ancestry_copy_id.clone(),
            chromosome_id: chromosome_id.clone(),
            locus_id: mapped_locus.locus_id.clone(),
            parent_role,
            ancestral_allele,
            occurrence_draw_ppm,
            outcome: LinkedMutationOutcome::Substitution {
                alternate_draw_index,
                origin,
            },
        });
    }
    Ok(())
}

fn linked_draw_below(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    ancestry_copy_id: &AncestryCopyId,
    chromosome_id: &ChromosomeId,
    locus_id: &LocusId,
    purpose: &str,
    upper: u64,
) -> Result<u64, LinkedMutationError> {
    if upper == 0 {
        return Err(LinkedMutationError::InvalidDrawUpperBound);
    }
    if upper == 1 {
        return Ok(0);
    }

    let zone = u64::MAX - (u64::MAX % upper);
    let mut attempt = 0_u64;
    loop {
        let value = linked_semantic_draw_u64(
            event,
            operators,
            ancestry_copy_id,
            chromosome_id,
            locus_id,
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

fn linked_semantic_draw_u64(
    event: &ReproductionEventId,
    operators: &EvolutionOperatorProfile,
    ancestry_copy_id: &AncestryCopyId,
    chromosome_id: &ChromosomeId,
    locus_id: &LocusId,
    purpose: &str,
    attempt: u64,
) -> Result<u64, LinkedMutationError> {
    operators.validate()?;
    let mut digest = Sha256::new();
    digest.update(LINKED_MUTATION_DRAW_DOMAIN);
    put_text(&mut digest, event.as_str());
    put_text(&mut digest, operators.profile_id.as_str());
    put_text(&mut digest, &operators.version);
    operators.mutation.put_randomness_identity(&mut digest);
    put_text(&mut digest, ancestry_copy_id.as_str());
    put_text(&mut digest, chromosome_id.as_str());
    put_text(&mut digest, locus_id.as_str());
    put_text(&mut digest, purpose);
    put_u64(&mut digest, attempt);
    let bytes: [u8; 32] = digest.finalize().into();
    Ok(u64::from_le_bytes(
        bytes[..8]
            .try_into()
            .expect("SHA-256 output has an 8-byte prefix"),
    ))
}

#[derive(Debug)]
pub enum LinkedMutationError {
    Evolution(EvolutionError),
    Descendant(DescendantAncestryError),
    Graph(AncestryGraphError),
    Origin(MutationOriginError),
    UnsupportedVersion(u32),
    InvalidDrawUpperBound,
    ChromosomeMissing(ChromosomeId),
    DescendantCopyMissing {
        chromosome: ChromosomeId,
        role: ParentRole,
    },
    LocusSequenceMismatch { chromosome: ChromosomeId },
    ReplayMismatch,
}

impl From<EvolutionError> for LinkedMutationError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<DescendantAncestryError> for LinkedMutationError {
    fn from(value: DescendantAncestryError) -> Self {
        Self::Descendant(value)
    }
}

impl From<AncestryGraphError> for LinkedMutationError {
    fn from(value: AncestryGraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<MutationOriginError> for LinkedMutationError {
    fn from(value: MutationOriginError) -> Self {
        Self::Origin(value)
    }
}

impl fmt::Display for LinkedMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Descendant(error) => write!(f, "descendant ancestry authority error: {error}"),
            Self::Graph(error) => write!(f, "ancestry graph authority error: {error}"),
            Self::Origin(error) => write!(f, "mutation-origin authority error: {error}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported linked-mutation execution version {version}")
            }
            Self::InvalidDrawUpperBound => write!(f, "linked mutation draw upper bound is zero"),
            Self::ChromosomeMissing(id) => {
                write!(f, "linked mutation chromosome {} is missing", id.as_str())
            }
            Self::DescendantCopyMissing { chromosome, role } => write!(
                f,
                "persistent descendant copy is missing for chromosome {} and role {role:?}",
                chromosome.as_str()
            ),
            Self::LocusSequenceMismatch { chromosome } => write!(
                f,
                "linked mutation locus sequence mismatch on chromosome {}",
                chromosome.as_str()
            ),
            Self::ReplayMismatch => write!(
                f,
                "restored linked-mutation execution does not deterministically replay"
            ),
        }
    }
}

impl Error for LinkedMutationError {}
