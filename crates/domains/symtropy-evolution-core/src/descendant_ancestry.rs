use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    chromosome_stochastic::parent_role_tag,
    AncestryAuthorityError, AncestryCopyId, ChromosomeAncestryState, ChromosomeId,
    ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    DiploidLinkedOffspringDerivationV2, DiploidLinkedOffspringProvenanceV2Digest,
    EvolutionError, GameteAncestryDerivation, GameteAncestryDerivationProvenanceDigest,
    GameteAncestryError, HaplotypeAncestryClass, HereditarySchema, HereditarySchemaDigest,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescendantChromosomeCopy {
    pub chromosome_id: ChromosomeId,
    pub parent_role: ParentRole,
    pub child_copy_id: AncestryCopyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledAncestryInheritanceEdge {
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub parent_role: ParentRole,
    pub source_copy_id: AncestryCopyId,
    pub child_copy_id: AncestryCopyId,
}

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
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        offspring: &DiploidLinkedOffspringDerivationV2,
        child_ancestry: &PhasedAncestryState,
        event: &ReproductionEventId,
    ) -> Result<(), DescendantAncestryError> {
        offspring.child.validate(schema, chromosome_map)?;
        child_ancestry.validate_current(schema, chromosome_map, &offspring.child)?;
        if self.state_version != DESCENDANT_ANCESTRY_MATERIALIZATION_VERSION {
            return Err(DescendantAncestryError::UnsupportedMaterializationVersion(
                self.state_version,
            ));
        }
        if &self.reproduction_event_id != event {
            return Err(DescendantAncestryError::EventContextMismatch);
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.child_phased_state_digest
                != offspring.child.canonical_digest(schema, chromosome_map)?
            || self.child_ancestry_state_digest
                != child_ancestry.canonical_digest(schema, chromosome_map, &offspring.child)?
        {
            return Err(DescendantAncestryError::CurrentAuthorityMismatch);
        }

        let expected_copy_count = chromosome_map
            .chromosomes
            .len()
            .checked_mul(2)
            .ok_or(DescendantAncestryError::StructureMismatch)?;
        let expected_edge_count = chromosome_map
            .chromosomes
            .values()
            .try_fold(0_usize, |total, definition| {
                definition
                    .loci
                    .len()
                    .checked_mul(2)
                    .and_then(|count| total.checked_add(count))
                    .ok_or(DescendantAncestryError::StructureMismatch)
            })?;
        if self.descendant_copies.len() != expected_copy_count
            || self.edges.len() != expected_edge_count
        {
            return Err(DescendantAncestryError::StructureMismatch);
        }

        let mut copy_cursor = 0_usize;
        let mut edge_cursor = 0_usize;
        let mut descendant_ids = BTreeSet::new();
        for (chromosome_id, definition) in &chromosome_map.chromosomes {
            let copy_a = self
                .descendant_copies
                .get(copy_cursor)
                .ok_or(DescendantAncestryError::StructureMismatch)?;
            let copy_b = self
                .descendant_copies
                .get(copy_cursor + 1)
                .ok_or(DescendantAncestryError::StructureMismatch)?;
            copy_cursor += 2;
            if copy_a.chromosome_id != *chromosome_id
                || copy_b.chromosome_id != *chromosome_id
                || copy_a.parent_role != ParentRole::ParentA
                || copy_b.parent_role != ParentRole::ParentB
                || copy_a.child_copy_id == copy_b.child_copy_id
            {
                return Err(DescendantAncestryError::StructureMismatch);
            }
            if !descendant_ids.insert(copy_a.child_copy_id.clone())
                || !descendant_ids.insert(copy_b.child_copy_id.clone())
            {
                return Err(DescendantAncestryError::DescendantIdentityCollision);
            }

            for (role, child_copy_id) in [
                (ParentRole::ParentA, &copy_a.child_copy_id),
                (ParentRole::ParentB, &copy_b.child_copy_id),
            ] {
                for mapped_locus in &definition.loci {
                    let edge = self
                        .edges
                        .get(edge_cursor)
                        .ok_or(DescendantAncestryError::StructureMismatch)?;
                    edge_cursor += 1;
                    if edge.chromosome_id != *chromosome_id
                        || edge.locus_id != mapped_locus.locus_id
                        || edge.parent_role != role
                        || &edge.child_copy_id != child_copy_id
                        || edge.source_copy_id == edge.child_copy_id
                    {
                        return Err(DescendantAncestryError::StructureMismatch);
                    }
                }
            }
        }

        let sidecar_ids: BTreeSet<_> = child_ancestry
            .chromosomes
            .values()
            .flat_map(|chromosome| chromosome.classes.iter())
            .flat_map(|class| class.copy_ids.iter().cloned())
            .collect();
        if sidecar_ids != descendant_ids {
            return Err(DescendantAncestryError::StructureMismatch);
        }
        Ok(())
    }

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

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
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
        child_ancestry: &PhasedAncestryState,
        materialization: &DescendantAncestryMaterialization,
    ) -> Result<(), DescendantAncestryError> {
        if self.derivation_version != DESCENDANT_ANCESTRY_DERIVATION_VERSION {
            return Err(DescendantAncestryError::UnsupportedDerivationVersion(
                self.derivation_version,
            ));
        }
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
        materialization.validate_current(
            schema,
            chromosome_map,
            offspring,
            child_ancestry,
            event,
        )?;
        if self.offspring_provenance_digest != offspring.provenance.canonical_digest()
            || self.parent_a_gamete_ancestry_digest
                != parent_a_gamete_ancestry.provenance.canonical_digest()
            || self.parent_b_gamete_ancestry_digest
                != parent_b_gamete_ancestry.provenance.canonical_digest()
            || self.materialization_digest != materialization.canonical_digest()
        {
            return Err(DescendantAncestryError::CurrentAuthorityMismatch);
        }

        let recomputed = derive_descendant_ancestry(
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
            offspring,
            event,
        )?;
        if recomputed.child_ancestry != *child_ancestry
            || recomputed.materialization != *materialization
            || recomputed.provenance != *self
        {
            return Err(DescendantAncestryError::DerivationMismatch);
        }
        Ok(())
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
    materialization.validate_current(
        schema,
        chromosome_map,
        offspring,
        &child_ancestry,
        event,
    )?;
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
        write!(&mut text, "{byte:02x}")
            .map_err(|_| DescendantAncestryError::IdentityEncoding)?;
    }
    AncestryCopyId::new(text).map_err(DescendantAncestryError::Evolution)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescendantAncestryError {
    Evolution(EvolutionError),
    Ancestry(AncestryAuthorityError),
    GameteAncestry(GameteAncestryError),
    IdentityEncoding,
    UnsupportedMaterializationVersion(u32),
    UnsupportedDerivationVersion(u32),
    CurrentAuthorityMismatch,
    EventContextMismatch,
    StructureMismatch,
    ChromosomeSetMismatch,
    LocusSequenceMismatch { chromosome: ChromosomeId },
    DescendantIdentityCollision,
    DerivationMismatch,
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

impl From<GameteAncestryError> for DescendantAncestryError {
    fn from(value: GameteAncestryError) -> Self {
        Self::GameteAncestry(value)
    }
}

impl fmt::Display for DescendantAncestryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Ancestry(error) => write!(f, "ancestry authority error: {error}"),
            Self::GameteAncestry(error) => {
                write!(f, "gamete ancestry authority error: {error}")
            }
            Self::IdentityEncoding => {
                write!(f, "failed to encode deterministic descendant ancestry identity")
            }
            Self::UnsupportedMaterializationVersion(version) => {
                write!(f, "unsupported descendant ancestry materialization version {version}")
            }
            Self::UnsupportedDerivationVersion(version) => {
                write!(f, "unsupported descendant ancestry derivation version {version}")
            }
            Self::CurrentAuthorityMismatch => {
                write!(f, "descendant ancestry does not match exact current authority")
            }
            Self::EventContextMismatch => {
                write!(f, "descendant ancestry reproduction-event context mismatch")
            }
            Self::StructureMismatch => {
                write!(f, "descendant ancestry copy/edge structure is not canonical")
            }
            Self::ChromosomeSetMismatch => {
                write!(f, "descendant ancestry chromosome set mismatch")
            }
            Self::LocusSequenceMismatch { chromosome } => write!(
                f,
                "descendant ancestry locus sequence mismatch on chromosome {}",
                chromosome.as_str()
            ),
            Self::DescendantIdentityCollision => write!(
                f,
                "derived descendant ancestry-copy identity collides with another child or parental source identity"
            ),
            Self::DerivationMismatch => {
                write!(f, "descendant ancestry deterministic replay mismatch")
            }
        }
    }
}

impl Error for DescendantAncestryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Ancestry(error) => Some(error),
            Self::GameteAncestry(error) => Some(error),
            _ => None,
        }
    }
}
