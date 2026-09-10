use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AlleleId, ChromosomeHaplotype, ChromosomeId, ChromosomeMap, ChromosomeMapDigest,
    ChromosomeMapId, ChromosomeRecombinationModel, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileDigest, EvolutionError, GeneticMapIntervalMicromorgans,
    HereditarySchema, HereditarySchemaDigest, HereditarySchemaId, ParentRole,
    PhasedHereditaryState, PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

pub const LINKED_GAMETE_VERSION: u32 = 1;
pub const LINKED_GAMETE_DERIVATION_VERSION: u32 = 1;

const LINKED_GAMETE_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:linked-gamete:v1\0";
const HAPLOTYPE_CONTENT_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:chromosome-haplotype-content:v1\0";
const LINKED_GAMETE_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-gamete-provenance:v1\0";
const ZERO_CROSSOVER_DRAW_DOMAIN: &[u8] =
    b"symtropy:evolution:linked-gamete:no-crossovers-independent-assortment:v1\0";

/// One haploid chromosome copy for every chromosome in the exact chromosome map.
///
/// A `LinkedGamete` is a state object, not proof of how that gamete was produced.
/// Derivation authority lives in `LinkedGameteDerivationProvenance`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedGamete {
    pub gamete_version: u32,
    pub schema_id: HereditarySchemaId,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_id: ChromosomeMapId,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub chromosomes: BTreeMap<ChromosomeId, ChromosomeHaplotype>,
}

impl LinkedGamete {
    pub fn new(
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        chromosomes: impl IntoIterator<Item = (ChromosomeId, ChromosomeHaplotype)>,
    ) -> Result<Self, EvolutionError> {
        chromosome_map.validate(schema)?;
        let mut by_id = BTreeMap::new();
        for (chromosome_id, haplotype) in chromosomes {
            if by_id.insert(chromosome_id.clone(), haplotype).is_some() {
                return Err(EvolutionError::DuplicateChromosomeIdentity {
                    chromosome: chromosome_id,
                });
            }
        }
        let value = Self {
            gamete_version: LINKED_GAMETE_VERSION,
            schema_id: schema.id.clone(),
            schema_digest: schema.canonical_digest()?,
            chromosome_map_id: chromosome_map.id.clone(),
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            chromosomes: by_id,
        };
        value.validate(schema, chromosome_map)?;
        Ok(value)
    }

    pub fn validate(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<(), EvolutionError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if self.gamete_version != LINKED_GAMETE_VERSION {
            return Err(EvolutionError::UnsupportedLinkedGamete(self.gamete_version));
        }
        if self.schema_id != schema.id || self.schema_digest != schema.canonical_digest()? {
            return Err(EvolutionError::HereditarySchemaAuthorityMismatch);
        }
        if self.chromosome_map_id != chromosome_map.id
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(EvolutionError::LinkedGameteChromosomeMapAuthorityMismatch);
        }
        if self.chromosomes.len() != chromosome_map.chromosomes.len() {
            return Err(EvolutionError::LinkedGameteChromosomeSetMismatch);
        }

        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let haplotype = self
                .chromosomes
                .get(chromosome_id)
                .ok_or(EvolutionError::LinkedGameteChromosomeSetMismatch)?;
            if haplotype.alleles.len() != definition.loci.len() {
                return Err(EvolutionError::LinkedGameteHaplotypeLocusCountMismatch {
                    chromosome: chromosome_id.clone(),
                    expected: definition.loci.len(),
                    observed: haplotype.alleles.len(),
                });
            }
            for (mapped_locus, allele) in definition.loci.iter().zip(&haplotype.alleles) {
                let locus = schema
                    .loci
                    .get(&mapped_locus.locus_id)
                    .ok_or_else(|| EvolutionError::MissingLocus(mapped_locus.locus_id.clone()))?;
                if !locus.allowed_alleles.contains(allele) {
                    return Err(EvolutionError::UnknownAllele {
                        locus: mapped_locus.locus_id.clone(),
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
        chromosome_map: &ChromosomeMap,
    ) -> Result<LinkedGameteDigest, EvolutionError> {
        self.validate(schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(LINKED_GAMETE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.gamete_version);
        put_text(&mut digest, self.schema_id.as_str());
        digest.update(self.schema_digest.as_bytes());
        put_text(&mut digest, self.chromosome_map_id.as_str());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_u64(&mut digest, self.chromosomes.len() as u64);
        for (chromosome_id, haplotype) in &self.chromosomes {
            put_text(&mut digest, chromosome_id.as_str());
            put_u64(&mut digest, haplotype.alleles.len() as u64);
            for allele in &haplotype.alleles {
                put_text(&mut digest, allele.as_str());
            }
        }
        Ok(LinkedGameteDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedGameteDigest([u8; 32]);

impl LinkedGameteDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedGameteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedGameteDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedGameteDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

/// Digest of one chromosome haplotype's allele content under exact schema/map authority.
///
/// This is content identity, not persistent chromosome ancestry identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChromosomeHaplotypeDigest([u8; 32]);

impl ChromosomeHaplotypeDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ChromosomeHaplotypeDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ChromosomeHaplotypeDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ChromosomeHaplotypeDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

/// Whole process-domain inheritance from one local canonical source haplotype slot.
///
/// `source_haplotype_slot` is meaningful only together with the exact source
/// `PhasedHereditaryStateDigest` bound by the enclosing provenance. It is not a
/// persistent homolog/ancestry ID. The content digest prevents the slot from
/// being the sole description of what was inherited.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WholeChromosomeInheritanceSegment {
    pub chromosome_id: ChromosomeId,
    pub interval: GeneticMapIntervalMicromorgans,
    pub source_haplotype_slot: u8,
    pub source_haplotype_digest: ChromosomeHaplotypeDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedGameteDerivationProvenance {
    derivation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    recombination_profile_digest: ChromosomeRecombinationProfileDigest,
    source_phased_state_digest: PhasedHereditaryStateDigest,
    event_id: ReproductionEventId,
    parent_role: ParentRole,
    gamete_digest: LinkedGameteDigest,
    segments: BTreeMap<ChromosomeId, WholeChromosomeInheritanceSegment>,
}

impl LinkedGameteDerivationProvenance {
    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn parent_role(&self) -> ParentRole {
        self.parent_role
    }

    pub fn source_phased_state_digest(&self) -> PhasedHereditaryStateDigest {
        self.source_phased_state_digest
    }

    pub fn gamete_digest(&self) -> LinkedGameteDigest {
        self.gamete_digest
    }

    pub fn segments(&self) -> &BTreeMap<ChromosomeId, WholeChromosomeInheritanceSegment> {
        &self.segments
    }

    pub fn canonical_digest(&self) -> LinkedGameteDerivationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(LINKED_GAMETE_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.recombination_profile_digest.as_bytes());
        digest.update(self.source_phased_state_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        digest.update([parent_role_tag(self.parent_role)]);
        digest.update(self.gamete_digest.as_bytes());
        put_u64(&mut digest, self.segments.len() as u64);
        for (chromosome_id, segment) in &self.segments {
            put_text(&mut digest, chromosome_id.as_str());
            put_text(&mut digest, segment.chromosome_id.as_str());
            put_u64(&mut digest, segment.interval.start.get());
            put_u64(&mut digest, segment.interval.end.get());
            digest.update([segment.source_haplotype_slot]);
            digest.update(segment.source_haplotype_digest.as_bytes());
        }
        LinkedGameteDerivationProvenanceDigest(digest.finalize().into())
    }

    /// Revalidate restored derivation evidence against exact external event
    /// context and by deterministic re-execution from current authorities.
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
        if self.derivation_version != LINKED_GAMETE_DERIVATION_VERSION {
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

        let derived = derive_zero_crossover_linked_gamete(
            schema,
            chromosome_map,
            source,
            profile,
            event,
            parent_role,
        )?;
        if derived.gamete != *gamete || derived.provenance.segments != self.segments {
            return Err(EvolutionError::LinkedGameteDerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LinkedGameteDerivationProvenanceDigest([u8; 32]);

impl LinkedGameteDerivationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for LinkedGameteDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LinkedGameteDerivationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for LinkedGameteDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedGameteDerivation {
    pub gamete: LinkedGamete,
    pub provenance: LinkedGameteDerivationProvenance,
}

pub fn derive_zero_crossover_linked_gamete(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    profile: &ChromosomeRecombinationProfile,
    event: &ReproductionEventId,
    parent_role: ParentRole,
) -> Result<LinkedGameteDerivation, EvolutionError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    source.validate(schema, chromosome_map)?;
    profile.validate(schema, chromosome_map)?;

    if schema.ploidy != 2 {
        return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
    }
    if profile.model != ChromosomeRecombinationModel::NoCrossoversIndependentAssortmentV1 {
        return Err(EvolutionError::LinkedGameteRecombinationModelMismatch);
    }
    validate_sexual_parent_role(parent_role)?;

    let schema_digest = schema.canonical_digest()?;
    let map_digest = chromosome_map.canonical_digest(schema)?;
    let mut gamete_chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());
    let mut segments = BTreeMap::new();

    for chromosome_id in chromosome_map.chromosomes.keys() {
        let source_chromosome = source
            .chromosomes
            .get(chromosome_id)
            .ok_or(EvolutionError::PhasedChromosomeSetMismatch)?;
        let slot = semantic_haplotype_slot(event, parent_role, chromosome_id);
        let haplotype = source_chromosome
            .haplotypes
            .get(slot)
            .ok_or(EvolutionError::LinkedGameteSourceAuthorityMismatch)?
            .clone();
        let domain = profile
            .domains
            .get(chromosome_id)
            .ok_or(EvolutionError::OperatorAuthorityMismatch)?;
        let segment = WholeChromosomeInheritanceSegment {
            chromosome_id: chromosome_id.clone(),
            interval: domain.interval,
            source_haplotype_slot: slot as u8,
            source_haplotype_digest: chromosome_haplotype_digest(
                schema_digest,
                map_digest,
                chromosome_id,
                &haplotype,
            ),
        };
        gamete_chromosomes.push((chromosome_id.clone(), haplotype));
        segments.insert(chromosome_id.clone(), segment);
    }

    let gamete = LinkedGamete::new(schema, chromosome_map, gamete_chromosomes)?;
    let provenance = LinkedGameteDerivationProvenance {
        derivation_version: LINKED_GAMETE_DERIVATION_VERSION,
        schema_digest,
        chromosome_map_digest: map_digest,
        recombination_profile_digest: profile.canonical_digest(schema, chromosome_map)?,
        source_phased_state_digest: source.canonical_digest(schema, chromosome_map)?,
        event_id: event.clone(),
        parent_role,
        gamete_digest: gamete.canonical_digest(schema, chromosome_map)?,
        segments,
    };

    Ok(LinkedGameteDerivation { gamete, provenance })
}

fn validate_sexual_parent_role(parent_role: ParentRole) -> Result<(), EvolutionError> {
    match parent_role {
        ParentRole::ParentA | ParentRole::ParentB => Ok(()),
        ParentRole::ClonalParent => Err(EvolutionError::LinkedGameteRequiresSexualParentRole),
    }
}

fn parent_role_tag(parent_role: ParentRole) -> u8 {
    match parent_role {
        ParentRole::ClonalParent => 0,
        ParentRole::ParentA => 1,
        ParentRole::ParentB => 2,
    }
}

fn semantic_haplotype_slot(
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
) -> usize {
    let mut digest = Sha256::new();
    digest.update(ZERO_CROSSOVER_DRAW_DOMAIN);
    put_text(&mut digest, event.as_str());
    digest.update([parent_role_tag(parent_role)]);
    put_text(&mut digest, chromosome_id.as_str());
    let bytes: [u8; 32] = digest.finalize().into();
    usize::from(bytes[0] & 1)
}

fn chromosome_haplotype_digest(
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    chromosome_id: &ChromosomeId,
    haplotype: &ChromosomeHaplotype,
) -> ChromosomeHaplotypeDigest {
    let mut digest = Sha256::new();
    digest.update(HAPLOTYPE_CONTENT_DIGEST_DOMAIN);
    digest.update(schema_digest.as_bytes());
    digest.update(chromosome_map_digest.as_bytes());
    put_text(&mut digest, chromosome_id.as_str());
    put_u64(&mut digest, haplotype.alleles.len() as u64);
    for allele in &haplotype.alleles {
        put_text(&mut digest, allele.as_str());
    }
    ChromosomeHaplotypeDigest(digest.finalize().into())
}
