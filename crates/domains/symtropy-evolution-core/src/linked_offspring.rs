use crate::{
    canonical::{fmt_hex, put_text, put_u32},
    ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileDigest, EvolutionError, HereditarySchema,
    HereditarySchemaDigest, LinkedGameteDerivation, LinkedGameteDerivationProvenanceDigest,
    LinkedGameteDigest, ParentRole, PhasedChromosomeState, PhasedHereditaryState,
    PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const DIPLOID_LINKED_OFFSPRING_DERIVATION_VERSION: u32 = 1;
const DIPLOID_LINKED_OFFSPRING_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:diploid-linked-offspring-provenance:v1\0";

/// Exact reference to one already-revalidated linked gamete contribution.
///
/// Parent role remains provenance. It is not encoded into the canonical child
/// homolog row ordering, which remains the C2 unlabeled-homolog representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedGameteContributionEvidence {
    pub role: ParentRole,
    pub source_phased_state_digest: PhasedHereditaryStateDigest,
    pub gamete_digest: LinkedGameteDigest,
    pub gamete_derivation_digest: LinkedGameteDerivationProvenanceDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiploidLinkedOffspringProvenance {
    derivation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    recombination_profile_digest: ChromosomeRecombinationProfileDigest,
    event_id: ReproductionEventId,
    parent_a: LinkedGameteContributionEvidence,
    parent_b: LinkedGameteContributionEvidence,
    child_digest: PhasedHereditaryStateDigest,
}

impl DiploidLinkedOffspringProvenance {
    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn parent_a(&self) -> &LinkedGameteContributionEvidence {
        &self.parent_a
    }

    pub fn parent_b(&self) -> &LinkedGameteContributionEvidence {
        &self.parent_b
    }

    pub fn child_digest(&self) -> PhasedHereditaryStateDigest {
        self.child_digest
    }

    pub fn canonical_digest(&self) -> DiploidLinkedOffspringProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(DIPLOID_LINKED_OFFSPRING_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.recombination_profile_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        put_contribution(&mut digest, &self.parent_a);
        put_contribution(&mut digest, &self.parent_b);
        digest.update(self.child_digest.as_bytes());
        DiploidLinkedOffspringProvenanceDigest(digest.finalize().into())
    }

    /// Revalidate restored child/parentage evidence by revalidating both linked
    /// gamete derivations and deterministically rebuilding the exact child.
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        profile: &ChromosomeRecombinationProfile,
        parent_a_source: &PhasedHereditaryState,
        parent_a: &LinkedGameteDerivation,
        parent_b_source: &PhasedHereditaryState,
        parent_b: &LinkedGameteDerivation,
        event: &ReproductionEventId,
        child: &PhasedHereditaryState,
    ) -> Result<(), EvolutionError> {
        if self.derivation_version != DIPLOID_LINKED_OFFSPRING_DERIVATION_VERSION {
            return Err(EvolutionError::LinkedOffspringDerivationMismatch);
        }
        if &self.event_id != event {
            return Err(EvolutionError::LinkedOffspringEventContextMismatch);
        }
        schema.validate()?;
        chromosome_map.validate(schema)?;
        profile.validate(schema, chromosome_map)?;
        if schema.ploidy != 2 {
            return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
        }

        if schema.canonical_digest()? != self.schema_digest
            || chromosome_map.canonical_digest(schema)? != self.chromosome_map_digest
            || profile.canonical_digest(schema, chromosome_map)? != self.recombination_profile_digest
        {
            return Err(EvolutionError::LinkedOffspringAuthorityMismatch);
        }

        parent_a.provenance.validate_current(
            schema,
            chromosome_map,
            parent_a_source,
            profile,
            event,
            ParentRole::ParentA,
            &parent_a.gamete,
        )?;
        parent_b.provenance.validate_current(
            schema,
            chromosome_map,
            parent_b_source,
            profile,
            event,
            ParentRole::ParentB,
            &parent_b.gamete,
        )?;

        let expected_parent_a = contribution_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a,
            ParentRole::ParentA,
        )?;
        let expected_parent_b = contribution_evidence(
            schema,
            chromosome_map,
            parent_b_source,
            parent_b,
            ParentRole::ParentB,
        )?;
        if self.parent_a != expected_parent_a || self.parent_b != expected_parent_b {
            return Err(EvolutionError::LinkedOffspringContributionMismatch);
        }

        child.validate(schema, chromosome_map)?;
        if child.canonical_digest(schema, chromosome_map)? != self.child_digest {
            return Err(EvolutionError::LinkedOffspringResultMismatch);
        }

        let recomputed = assemble_diploid_linked_offspring(
            schema,
            chromosome_map,
            profile,
            parent_a_source,
            parent_a,
            parent_b_source,
            parent_b,
            event,
        )?;
        if recomputed.child != *child || recomputed.provenance != *self {
            return Err(EvolutionError::LinkedOffspringDerivationMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiploidLinkedOffspringProvenanceDigest([u8; 32]);

impl DiploidLinkedOffspringProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DiploidLinkedOffspringProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiploidLinkedOffspringProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DiploidLinkedOffspringProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiploidLinkedOffspringDerivation {
    pub child: PhasedHereditaryState,
    pub provenance: DiploidLinkedOffspringProvenance,
}

/// Deterministically assemble one diploid phased child from exact validated
/// ParentA and ParentB linked gamete derivations.
///
/// This operation introduces no additional stochastic draw and performs no
/// mutation. Parent-role order remains in provenance only; each child chromosome
/// is routed through C2 whole-haplotype canonicalization.
pub fn assemble_diploid_linked_offspring(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    profile: &ChromosomeRecombinationProfile,
    parent_a_source: &PhasedHereditaryState,
    parent_a: &LinkedGameteDerivation,
    parent_b_source: &PhasedHereditaryState,
    parent_b: &LinkedGameteDerivation,
    event: &ReproductionEventId,
) -> Result<DiploidLinkedOffspringDerivation, EvolutionError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    profile.validate(schema, chromosome_map)?;
    if schema.ploidy != 2 {
        return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
    }

    parent_a.provenance.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        profile,
        event,
        ParentRole::ParentA,
        &parent_a.gamete,
    )?;
    parent_b.provenance.validate_current(
        schema,
        chromosome_map,
        parent_b_source,
        profile,
        event,
        ParentRole::ParentB,
        &parent_b.gamete,
    )?;

    let mut chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());
    for chromosome_id in chromosome_map.chromosomes.keys() {
        let from_a = parent_a
            .gamete
            .chromosomes
            .get(chromosome_id)
            .ok_or(EvolutionError::LinkedGameteChromosomeSetMismatch)?
            .clone();
        let from_b = parent_b
            .gamete
            .chromosomes
            .get(chromosome_id)
            .ok_or(EvolutionError::LinkedGameteChromosomeSetMismatch)?
            .clone();
        chromosomes.push(PhasedChromosomeState::new(
            chromosome_id.clone(),
            vec![from_a, from_b],
        ));
    }

    let child = PhasedHereditaryState::new(schema, chromosome_map, chromosomes)?;
    let provenance = DiploidLinkedOffspringProvenance {
        derivation_version: DIPLOID_LINKED_OFFSPRING_DERIVATION_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        recombination_profile_digest: profile.canonical_digest(schema, chromosome_map)?,
        event_id: event.clone(),
        parent_a: contribution_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a,
            ParentRole::ParentA,
        )?,
        parent_b: contribution_evidence(
            schema,
            chromosome_map,
            parent_b_source,
            parent_b,
            ParentRole::ParentB,
        )?,
        child_digest: child.canonical_digest(schema, chromosome_map)?,
    };

    Ok(DiploidLinkedOffspringDerivation { child, provenance })
}

fn contribution_evidence(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    derivation: &LinkedGameteDerivation,
    role: ParentRole,
) -> Result<LinkedGameteContributionEvidence, EvolutionError> {
    Ok(LinkedGameteContributionEvidence {
        role,
        source_phased_state_digest: source.canonical_digest(schema, chromosome_map)?,
        gamete_digest: derivation.gamete.canonical_digest(schema, chromosome_map)?,
        gamete_derivation_digest: derivation.provenance.canonical_digest(),
    })
}

fn put_contribution(digest: &mut Sha256, contribution: &LinkedGameteContributionEvidence) {
    digest.update([parent_role_tag(contribution.role)]);
    digest.update(contribution.source_phased_state_digest.as_bytes());
    digest.update(contribution.gamete_digest.as_bytes());
    digest.update(contribution.gamete_derivation_digest.as_bytes());
}

fn parent_role_tag(role: ParentRole) -> u8 {
    match role {
        ParentRole::ClonalParent => 0,
        ParentRole::ParentA => 1,
        ParentRole::ParentB => 2,
    }
}
