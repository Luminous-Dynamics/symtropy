use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::parent_role_tag,
    AncestryAuthorityError, AncestryCopyId, ChromosomeAncestryState, ChromosomeId,
    ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    DiploidLinkedOffspringDerivationV2, DiploidLinkedOffspringProvenanceV2Digest,
    EvolutionError, GameteAncestryDerivation, GameteAncestryDerivationProvenanceDigest,
    HaplotypeAncestryClass, HereditarySchema, HereditarySchemaDigest,
    LinkedGameteDerivationEvidence, LocusId, ParentRole, PhasedAncestryState,
    PhasedAncestryStateDigest, PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    error::Error,
    fmt,
    fmt::Write as _,
};

pub const DESCENDANT_ANCESTRY_MATERIALIZATION_VERSION: u32 = 1;
pub const DESCENDANT_ANCESTRY_DERIVATION_VERSION: u32 = 1;

const DESCENDANT_COPY_ID_DOMAIN: &[u8] =
    b"symtropy:evolution:descendant-ancestry-copy:v1\0";
const DESCENDANT_ANCESTRY_MATERIALIZATION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:descendant-ancestry-materialization:v1\0";
const DESCENDANT_ANCESTRY_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:descendant-ancestry-provenance:v1\0";

/// One newly materialized persistent ancestry-copy identity in the child.
///
/// `parent_role` identifies the contribution process that created this child
/// chromosome copy. It does not label a canonical C2 child haplotype row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantChromosomeCopy {
    pub chromosome_id: ChromosomeId,
    pub parent_role: ParentRole,
    pub child_copy_id: AncestryCopyId,
}

/// Exact ancestry transmission at one modeled hereditary locus.
///
/// The edge links a parental persistent ancestry copy to the newly materialized
/// child ancestry copy. It makes no claim about unmodeled sequence between loci.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledAncestryInheritanceEdge {
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub parent_role: ParentRole,
    pub source_copy_id: AncestryCopyId,
    pub child_copy_id: AncestryCopyId,
}

/// Process-independent result of one validated descendant-copy materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantAncestryMaterialization {
    pub state_version: u32,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub reproduction_event_id: ReproductionEventId,
    pub child_phased_state_digest: PhasedHereditaryStateDigest,
    pub child_ancestry_state_digest: PhasedAncestryStateDigest,
    pub descendant_copies: Vec<DescendantChromosomeCopy>,
    pub edges: Vec<ModeledAncestryInheritanceEdge>,
}

impl DescendantAncestryMaterialization {
    pub fn canonical_digest(&self) -> DescendantAncestryMaterializationDigest {
        let mut digest = Sha256::new();
        digest.update(DESCENDANT_ANCESTRY_MATERIALIZATION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_text(&mut digest, self.reproduction_event_id.as_str());
        digest.update(self.child_phased_state_digest.as_bytes());
        digest.update(self.child_ancestry_state_digest.as_bytes());
        put_u64(&mut digest, self.descendant_copies.len() as u64);
        for copy in &self.descendant_copies {
            put_text(&mut digest, copy.chromosome_id.as_str());
            digest.update([parent_role_tag(copy.parent_role)]);
            put_text(&mut digest, copy.child_copy_id.as_str());
        }
        put_u64(&mut digest, self.edges.len() as u64);
        for edge in &self.edges {
            put_text(&mut digest, edge.chromosome_id.as_str());
            put_text(&mut digest, edge.locus_id.as_str());
            digest.update([parent_role_tag(edge.parent_role)]);
            put_text(&mut digest, edge.source_copy_id.as_str());
            put_text(&mut digest, edge.child_copy_id.as_str());
        }
        DescendantAncestryMaterializationDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DescendantAncestryMaterializationDigest([u8; 32]);

impl DescendantAncestryMaterializationDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DescendantAncestryMaterializationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DescendantAncestryMaterializationDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DescendantAncestryMaterializationDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantAncestryDerivationProvenance {
    derivation_version: u32,
    offspring_provenance_digest: DiploidLinkedOffspringProvenanceV2Digest,
    parent_a_gamete_ancestry_digest: GameteAncestryDerivationProvenanceDigest,
    parent_b_gamete_ancestry_digest: GameteAncestryDerivationProvenanceDigest,
    materialization_digest: DescendantAncestryMaterializationDigest,
}

impl DescendantAncestryDerivationProvenance {
    pub fn offspring_provenance_digest(&self) -> DiploidLinkedOffspringProvenanceV2Digest {
        self.offspring_provenance_digest
    }

    pub fn materialization_digest(&self) -> DescendantAncestryMaterializationDigest {
        self.materialization_digest
    }

    pub fn canonical_digest(&self) -> DescendantAncestryDerivationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(DESCENDANT_ANCESTRY_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.offspring_provenance_digest.as_bytes());
        digest.update(self.parent_a_gamete_ancestry_digest.as_bytes());
        digest.update(self.parent_b_gamete_ancestry_digest.as_bytes());
        digest.update(self.materialization_digest.as_bytes());
        DescendantAncestryDerivationProvenanceDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DescendantAncestryDerivationProvenanceDigest([u8; 32]);

impl DescendantAncestryDerivationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DescendantAncestryDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DescendantAncestryDerivationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DescendantAncestryDerivationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantAncestryDerivation {
    pub child_ancestry: PhasedAncestryState,
    pub materialization: DescendantAncestryMaterialization,
    pub provenance: DescendantAncestryDerivationProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn derive_descendant_ancestry(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    parent_a_source: &crate::PhasedHereditaryState,
    parent_a_source_ancestry: &PhasedAncestryState,
    parent_a_profile: &ChromosomeRecombinationProfile,
    parent_a_gamete: &LinkedGameteDerivationEvidence,
    parent_a_gamete_ancestry: &GameteAncestryDerivation,
    parent_b_source: &crate::PhasedHereditaryState,
    parent_b_source_ancestry: &PhasedAncestryState,
    parent_b_profile: &ChromosomeRecombinationProfile,
    parent_b_gamete: &LinkedGameteDerivationEvidence,
    parent_b_gamete_ancestry: &GameteAncestryDerivation,
    offspring: &DiploidLinkedOffspringDerivationV2,
    event: &ReproductionEventId,
) -> Result<DescendantAncestryDerivation, DescendantAncestryError> {
    offspring.provenance.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_profile,
        parent_a_gamete,
        parent_b_source,
        parent_b_profile,
        parent_b_gamete,
        event,
        &offspring.child,
    )?;
    parent_a_gamete_ancestry.provenance.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_source_ancestry,
        parent_a_profile,
        parent_a_gamete,
        event,
        ParentRole::ParentA,
        &parent_a_gamete_ancestry.ancestry,
    )?;
    parent_b_gamete_ancestry.provenance.validate_current(
        schema,
        chromosome_map,
        parent_b_source,
        parent_b_source_ancestry,
        parent_b_profile,
        parent_b_gamete,
        event,
        ParentRole::ParentB,
        &parent_b_gamete_ancestry.ancestry,
    )?;

    let mut descendant_copies = Vec::with_capacity(chromosome_map.chromosomes.len() * 2);
    let mut child_chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());
    let mut edges = Vec::new();
    let mut child_ids = BTreeSet::new();
    let mut parent_source_ids = BTreeSet::new();

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        let from_a = parent_a_gamete
            .gamete()
            .chromosomes
            .get(chromosome_id)
            .ok_or(DescendantAncestryError::ChromosomeSetMismatch)?;
        let from_b = parent_b_gamete
            .gamete()
            .chromosomes
            .get(chromosome_id)
            .ok_or(DescendantAncestryError::ChromosomeSetMismatch)?;
        let ancestry_a = parent_a_gamete_ancestry
            .ancestry
            .chromosomes
            .get(chromosome_id)
            .ok_or(DescendantAncestryError::ChromosomeSetMismatch)?;
        let ancestry_b = parent_b_gamete_ancestry
            .ancestry
            .chromosomes
            .get(chromosome_id)
            .ok_or(DescendantAncestryError::ChromosomeSetMismatch)?;
        if ancestry_a.loci.len() != definition.loci.len()
            || ancestry_b.loci.len() != definition.loci.len()
        {
            return Err(DescendantAncestryError::LocusSequenceMismatch {
                chromosome: chromosome_id.clone(),
            });
        }

        let child_a = derived_child_copy_id(
            event,
            ParentRole::ParentA,
            chromosome_id,
            parent_a_gamete_ancestry.provenance.canonical_digest(),
        )?;
        let child_b = derived_child_copy_id(
            event,
            ParentRole::ParentB,
            chromosome_id,
            parent_b_gamete_ancestry.provenance.canonical_digest(),
        )?;
        if !child_ids.insert(child_a.clone()) || !child_ids.insert(child_b.clone()) {
            return Err(DescendantAncestryError::DescendantIdentityCollision);
        }

        descendant_copies.push(DescendantChromosomeCopy {
            chromosome_id: chromosome_id.clone(),
            parent_role: ParentRole::ParentA,
            child_copy_id: child_a.clone(),
        });
        descendant_copies.push(DescendantChromosomeCopy {
            chromosome_id: chromosome_id.clone(),
            parent_role: ParentRole::ParentB,
            child_copy_id: child_b.clone(),
        });

        let classes = if from_a == from_b {
            vec![HaplotypeAncestryClass::new(
                0,
                vec![child_a.clone(), child_b.clone()],
            )?]
        } else if from_a < from_b {
            vec![
                HaplotypeAncestryClass::new(0, vec![child_a.clone()])?,
                HaplotypeAncestryClass::new(1, vec![child_b.clone()])?,
            ]
        } else {
            vec![
                HaplotypeAncestryClass::new(0, vec![child_b.clone()])?,
                HaplotypeAncestryClass::new(1, vec![child_a.clone()])?,
            ]
        };
        child_chromosomes.push(ChromosomeAncestryState::new(
            chromosome_id.clone(),
            classes,
        )?);

        append_edges(
            &mut edges,
            &mut parent_source_ids,
            chromosome_id,
            definition,
            ParentRole::ParentA,
            ancestry_a,
            &child_a,
        )?;
        append_edges(
            &mut edges,
            &mut parent_source_ids,
            chromosome_id,
            definition,
            ParentRole::ParentB,
            ancestry_b,
            &child_b,
        )?;
    }

    if child_ids.iter().any(|id| parent_source_ids.contains(id)) {
        return Err(DescendantAncestryError::DescendantIdentityCollision);
    }

    let child_ancestry =
        PhasedAncestryState::new(schema, chromosome_map, &offspring.child, child_chromosomes)?;
    let materialization = DescendantAncestryMaterialization {
        state_version: DESCENDANT_ANCESTRY_MATERIALIZATION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        reproduction_event_id: event.clone(),
        child_phased_state_digest: offspring.child.canonical_digest(schema, chromosome_map)?,
        child_ancestry_state_digest: child_ancestry
            .canonical_digest(schema, chromosome_map, &offspring.child)?,
        descendant_copies,
        edges,
    };
    let provenance = DescendantAncestryDerivationProvenance {
        derivation_version: DESCENDANT_ANCESTRY_DERIVATION_VERSION,
        offspring_provenance_digest: offspring.provenance.canonical_digest(),
        parent_a_gamete_ancestry_digest: parent_a_gamete_ancestry
            .provenance
            .canonical_digest(),
        parent_b_gamete_ancestry_digest: parent_b_gamete_ancestry
            .provenance
            .canonical_digest(),
        materialization_digest: materialization.canonical_digest(),
    };

    Ok(DescendantAncestryDerivation {
        child_ancestry,
        materialization,
        provenance,
    })
}

fn append_edges(
    edges: &mut Vec<ModeledAncestryInheritanceEdge>,
    parent_source_ids: &mut BTreeSet<AncestryCopyId>,
    chromosome_id: &ChromosomeId,
    definition: &crate::ChromosomeDefinition,
    parent_role: ParentRole,
    ancestry: &crate::ChromosomeGameteAncestry,
    child_copy_id: &AncestryCopyId,
) -> Result<(), DescendantAncestryError> {
    for (origin, mapped_locus) in ancestry.loci.iter().zip(&definition.loci) {
        if origin.locus_id != mapped_locus.locus_id {
            return Err(DescendantAncestryError::LocusSequenceMismatch {
                chromosome: chromosome_id.clone(),
            });
        }
        parent_source_ids.insert(origin.source_copy_id.clone());
        edges.push(ModeledAncestryInheritanceEdge {
            chromosome_id: chromosome_id.clone(),
            locus_id: origin.locus_id.clone(),
            parent_role,
            source_copy_id: origin.source_copy_id.clone(),
            child_copy_id: child_copy_id.clone(),
        });
    }
    Ok(())
}

fn derived_child_copy_id(
    event: &ReproductionEventId,
    parent_role: ParentRole,
    chromosome_id: &ChromosomeId,
    parent_ancestry_digest: GameteAncestryDerivationProvenanceDigest,
) -> Result<AncestryCopyId, DescendantAncestryError> {
    let mut digest = Sha256::new();
    digest.update(DESCENDANT_COPY_ID_DOMAIN);
    put_text(&mut digest, event.as_str());
    digest.update([parent_role_tag(parent_role)]);
    put_text(&mut digest, chromosome_id.as_str());
    digest.update(parent_ancestry_digest.as_bytes());
    let bytes: [u8; 32] = digest.finalize().into();
    let mut text = String::from("derived-v1:");
    for byte in bytes {
        write!(&mut text, "{byte:02x}").map_err(|_| DescendantAncestryError::IdentityEncoding)?;
    }
    AncestryCopyId::new(text).map_err(DescendantAncestryError::Evolution)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescendantAncestryError {
    Evolution(EvolutionError),
    Ancestry(AncestryAuthorityError),
    IdentityEncoding,
    ChromosomeSetMismatch,
    LocusSequenceMismatch { chromosome: ChromosomeId },
    DescendantIdentityCollision,
}

impl From<EvolutionError> for DescendantAncestryError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<AncestryAuthorityError> for DescendantAncestryError {
    fn from(value: AncestryAuthorityError) -> Self {
        Self::Ancestry(value)
    }
}

impl fmt::Display for DescendantAncestryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Ancestry(error) => write!(f, "ancestry authority error: {error}"),
            Self::IdentityEncoding => write!(f, "failed to encode deterministic descendant ancestry identity"),
            Self::ChromosomeSetMismatch => write!(f, "descendant ancestry chromosome set mismatch"),
            Self::LocusSequenceMismatch { chromosome } => write!(
                f,
                "descendant ancestry locus sequence mismatch on chromosome {}",
                chromosome.as_str()
            ),
            Self::DescendantIdentityCollision => write!(
                f,
                "derived descendant ancestry-copy identity collides with another child or parental source identity"
            ),
        }
    }
}

impl Error for DescendantAncestryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Ancestry(error) => Some(error),
            _ => None,
        }
    }
}
