use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::{
        parent_role_tag, semantic_initial_haplotype_slot, semantic_marker_parity_draw_ppm,
    },
    haldane_odd_parity_probability_ppm, ChromosomeHaplotype, ChromosomeId, ChromosomeMap,
    ChromosomeMapDigest, ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileDigest, EvolutionError, HereditarySchema,
    HereditarySchemaDigest, LinkedGamete, LinkedGameteDigest, LocusId, ParentRole,
    PhasedHereditaryState, PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

pub const MARKER_MARGINAL_GAMETE_DERIVATION_VERSION: u32 = 1;
const MARKER_MARGINAL_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:marker-marginal-gamete-provenance:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrossoverParity {
    Even,
    Odd,
}

impl CrossoverParity {
    fn tag(self) -> u8 {
        match self {
            Self::Even => 0,
            Self::Odd => 1,
        }
    }
}

/// Exact modeled-marker evidence for one adjacent genetic-map interval.
///
/// `Odd` means only that the hidden Poisson crossover count in this interval was
/// sampled as odd. It does not establish one crossover or any breakpoint coordinate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjacentMarkerParityEvidence {
    pub chromosome_id: ChromosomeId,
    pub left_locus_id: LocusId,
    pub right_locus_id: LocusId,
    pub distance_micromorgans: u64,
    pub odd_probability_ppm: u32,
    pub opportunity_draw_ppm: u32,
    pub parity: CrossoverParity,
    pub source_haplotype_slot_after: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerMarginalChromosomeEvidence {
    pub chromosome_id: ChromosomeId,
    pub initial_source_haplotype_slot: u8,
    pub adjacent_intervals: Vec<AdjacentMarkerParityEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerMarginalGameteDerivationProvenance {
    derivation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    recombination_profile_digest: ChromosomeRecombinationProfileDigest,
    source_phased_state_digest: PhasedHereditaryStateDigest,
    event_id: ReproductionEventId,
    parent_role: ParentRole,
    gamete_digest: LinkedGameteDigest,
    chromosomes: BTreeMap<ChromosomeId, MarkerMarginalChromosomeEvidence>,
}

impl MarkerMarginalGameteDerivationProvenance {
    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn parent_role(&self) -> ParentRole {
        self.parent_role
    }

    pub fn gamete_digest(&self) -> LinkedGameteDigest {
        self.gamete_digest
    }

    pub fn chromosomes(&self) -> &BTreeMap<ChromosomeId, MarkerMarginalChromosomeEvidence> {
        &self.chromosomes
    }

    pub fn canonical_digest(&self) -> MarkerMarginalGameteDerivationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(MARKER_MARGINAL_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.recombination_profile_digest.as_bytes());
        digest.update(self.source_phased_state_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        digest.update([parent_role_tag(self.parent_role)]);
        digest.update(self.gamete_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, chromosome) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_text(&mut digest, chromosome.chromosome_id.as_str());
            digest.update([chromosome.initial_source_haplotype_slot]);
            put_u64(&mut digest, chromosome.adjacent_intervals.len() as u64);
            for interval in &chromosome.adjacent_intervals {
                put_text(&mut digest, interval.chromosome_id.as_str());
                put_text(&mut digest, interval.left_locus_id.as_str());
                put_text(&mut digest, interval.right_locus_id.as_str());
                put_u64(&mut digest, interval.distance_micromorgans);
                put_u32(&mut digest, interval.odd_probability_ppm);
                put_u32(&mut digest, interval.opportunity_draw_ppm);
                digest.update([interval.parity.tag()]);
                digest.update([interval.source_haplotype_slot_after]);
            }
        }
        MarkerMarginalGameteDerivationProvenanceDigest(digest.finalize().into())
    }

    /// Restored provenance is only data until the exact marker-marginal process is
    /// re-executed against externally expected event/role and current authorities.
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        source: &PhasedHereditaryState,
        profile: &ChromosomeRecombinationProfile,
        event: &ReproductionEventId,
        parent_role: ParentRole,
        gamete: &LinkedGamete,
    ) -> Result<(), EvolutionError> {
        if self.derivation_version != MARKER_MARGINAL_GAMETE_DERIVATION_VERSION {
            return Err(EvolutionError::LinkedGameteDerivationMismatch);
        }
        validate_sexual_parent_role(parent_role)?;
        if &self.event_id != event || self.parent_role != parent_role {
            return Err(EvolutionError::LinkedGameteEventContextMismatch);
        }
        if schema.canonical_digest()? != self.schema_digest
            || chromosome_map.canonical_digest(schema)? != self.chromosome_map_digest
            || profile.canonical_digest(schema, chromosome_map)? != self.recombination_profile_digest
            || source.canonical_digest(schema, chromosome_map)? != self.source_phased_state_digest
        {
            return Err(EvolutionError::LinkedGameteSourceAuthorityMismatch);
        }
        gamete.validate(schema, chromosome_map)?;
        if gamete.canonical_digest(schema, chromosome_map)? != self.gamete_digest {
            return Err(EvolutionError::LinkedGameteResultMismatch);
        }

        let recomputed = derive_marker_marginal_poisson_linked_gamete(
            schema,
            chromosome_map,
            source,
            profile,
            event,
            parent_role,
        )?;
        if recomputed.gamete != *gamete || recomputed.provenance != *self {
            return Err(EvolutionError::LinkedGameteDerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MarkerMarginalGameteDerivationProvenanceDigest([u8; 32]);

impl MarkerMarginalGameteDerivationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MarkerMarginalGameteDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MarkerMarginalGameteDerivationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for MarkerMarginalGameteDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkerMarginalGameteDerivation {
    pub gamete: LinkedGamete,
    pub provenance: MarkerMarginalGameteDerivationProvenance,
}

pub fn derive_marker_marginal_poisson_linked_gamete(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    profile: &ChromosomeRecombinationProfile,
    event: &ReproductionEventId,
    parent_role: ParentRole,
) -> Result<MarkerMarginalGameteDerivation, EvolutionError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    source.validate(schema, chromosome_map)?;
    profile.validate(schema, chromosome_map)?;
    if schema.ploidy != 2 {
        return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
    }
    if profile.model != ChromosomeRecombinationModel::PoissonCrossoversNoInterferenceV1 {
        return Err(EvolutionError::LinkedGameteRecombinationModelMismatch);
    }
    validate_sexual_parent_role(parent_role)?;

    let mut gamete_chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());
    let mut chromosome_evidence = BTreeMap::new();

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        let source_chromosome = source
            .chromosomes
            .get(chromosome_id)
            .ok_or(EvolutionError::PhasedChromosomeSetMismatch)?;
        let haplotype_zero = source_chromosome
            .haplotypes
            .first()
            .ok_or(EvolutionError::LinkedGameteSourceAuthorityMismatch)?;
        let haplotype_one = source_chromosome
            .haplotypes
            .get(1)
            .ok_or(EvolutionError::LinkedGameteSourceAuthorityMismatch)?;
        let identical_haplotypes = haplotype_zero == haplotype_one;
        let mut source_slot = if identical_haplotypes {
            0
        } else {
            semantic_initial_haplotype_slot(event, parent_role, chromosome_id)
        };
        let initial_source_haplotype_slot = source_slot as u8;

        let first_allele = source_chromosome
            .haplotypes
            .get(source_slot)
            .and_then(|haplotype| haplotype.alleles.first())
            .ok_or(EvolutionError::LinkedGameteSourceAuthorityMismatch)?
            .clone();
        let mut alleles = vec![first_allele];
        let mut adjacent_intervals = Vec::with_capacity(definition.loci.len().saturating_sub(1));

        for locus_pair in definition.loci.windows(2) {
            let left = &locus_pair[0];
            let right = &locus_pair[1];
            let distance_micromorgans = right
                .position
                .get()
                .checked_sub(left.position.get())
                .ok_or(EvolutionError::SamplingInvariantViolation)?;
            let odd_probability_ppm =
                haldane_odd_parity_probability_ppm(distance_micromorgans);
            let opportunity_draw_ppm = semantic_marker_parity_draw_ppm(
                event,
                parent_role,
                chromosome_id,
                &left.locus_id,
                &right.locus_id,
            )?;
            let parity = if opportunity_draw_ppm < odd_probability_ppm {
                CrossoverParity::Odd
            } else {
                CrossoverParity::Even
            };
            if parity == CrossoverParity::Odd && !identical_haplotypes {
                source_slot ^= 1;
            }
            if identical_haplotypes {
                source_slot = 0;
            }

            let allele = source_chromosome
                .haplotypes
                .get(source_slot)
                .and_then(|haplotype| haplotype.alleles.get(alleles.len()))
                .ok_or(EvolutionError::LinkedGameteSourceAuthorityMismatch)?
                .clone();
            alleles.push(allele);
            adjacent_intervals.push(AdjacentMarkerParityEvidence {
                chromosome_id: chromosome_id.clone(),
                left_locus_id: left.locus_id.clone(),
                right_locus_id: right.locus_id.clone(),
                distance_micromorgans,
                odd_probability_ppm,
                opportunity_draw_ppm,
                parity,
                source_haplotype_slot_after: source_slot as u8,
            });
        }

        gamete_chromosomes.push((chromosome_id.clone(), ChromosomeHaplotype::new(alleles)));
        chromosome_evidence.insert(
            chromosome_id.clone(),
            MarkerMarginalChromosomeEvidence {
                chromosome_id: chromosome_id.clone(),
                initial_source_haplotype_slot,
                adjacent_intervals,
            },
        );
    }

    let gamete = LinkedGamete::new(schema, chromosome_map, gamete_chromosomes)?;
    let provenance = MarkerMarginalGameteDerivationProvenance {
        derivation_version: MARKER_MARGINAL_GAMETE_DERIVATION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        recombination_profile_digest: profile.canonical_digest(schema, chromosome_map)?,
        source_phased_state_digest: source.canonical_digest(schema, chromosome_map)?,
        event_id: event.clone(),
        parent_role,
        gamete_digest: gamete.canonical_digest(schema, chromosome_map)?,
        chromosomes: chromosome_evidence,
    };

    Ok(MarkerMarginalGameteDerivation { gamete, provenance })
}

fn validate_sexual_parent_role(parent_role: ParentRole) -> Result<(), EvolutionError> {
    match parent_role {
        ParentRole::ParentA | ParentRole::ParentB => Ok(()),
        ParentRole::ClonalParent => Err(EvolutionError::LinkedGameteRequiresSexualParentRole),
    }
}
