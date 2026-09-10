use crate::{
    canonical::{fmt_hex, put_text, put_u32},
    ChromosomeMap, ChromosomeMapDigest, ChromosomeRecombinationProfile,
    ChromosomeRecombinationProfileDigest, EvolutionError, HereditarySchema,
    HereditarySchemaDigest, LinkedGameteDerivationEvidence,
    LinkedGameteDerivationEvidenceDigest, LinkedGameteDigest, ParentRole, PhasedChromosomeState,
    PhasedHereditaryState, PhasedHereditaryStateDigest, ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

pub const DIPLOID_LINKED_OFFSPRING_DERIVATION_V2_VERSION: u32 = 2;
const DIPLOID_LINKED_OFFSPRING_PROVENANCE_V2_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:diploid-linked-offspring-provenance:v2\0";

/// Exact contribution evidence for one parent-specific linked-gamete process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkedGameteContributionEvidenceV2 {
    pub role: ParentRole,
    pub source_phased_state_digest: PhasedHereditaryStateDigest,
    pub recombination_profile_digest: ChromosomeRecombinationProfileDigest,
    pub gamete_digest: LinkedGameteDigest,
    pub derivation_evidence_digest: LinkedGameteDerivationEvidenceDigest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiploidLinkedOffspringProvenanceV2 {
    derivation_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    event_id: ReproductionEventId,
    parent_a: LinkedGameteContributionEvidenceV2,
    parent_b: LinkedGameteContributionEvidenceV2,
    child_digest: PhasedHereditaryStateDigest,
}

impl DiploidLinkedOffspringProvenanceV2 {
    pub fn event_id(&self) -> &ReproductionEventId {
        &self.event_id
    }

    pub fn parent_a(&self) -> &LinkedGameteContributionEvidenceV2 {
        &self.parent_a
    }

    pub fn parent_b(&self) -> &LinkedGameteContributionEvidenceV2 {
        &self.parent_b
    }

    pub fn child_digest(&self) -> PhasedHereditaryStateDigest {
        self.child_digest
    }

    pub fn canonical_digest(&self) -> DiploidLinkedOffspringProvenanceV2Digest {
        let mut digest = Sha256::new();
        digest.update(DIPLOID_LINKED_OFFSPRING_PROVENANCE_V2_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_text(&mut digest, self.event_id.as_str());
        put_contribution(&mut digest, &self.parent_a);
        put_contribution(&mut digest, &self.parent_b);
        digest.update(self.child_digest.as_bytes());
        DiploidLinkedOffspringProvenanceV2Digest(digest.finalize().into())
    }

    /// Restore-time/current validation. Each contribution replays through its
    /// exact process-specific derivation evidence and its own recombination profile.
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        parent_a_source: &PhasedHereditaryState,
        parent_a_profile: &ChromosomeRecombinationProfile,
        parent_a: &LinkedGameteDerivationEvidence,
        parent_b_source: &PhasedHereditaryState,
        parent_b_profile: &ChromosomeRecombinationProfile,
        parent_b: &LinkedGameteDerivationEvidence,
        event: &ReproductionEventId,
        child: &PhasedHereditaryState,
    ) -> Result<(), EvolutionError> {
        if self.derivation_version != DIPLOID_LINKED_OFFSPRING_DERIVATION_V2_VERSION {
            return Err(EvolutionError::LinkedOffspringDerivationMismatch);
        }
        if &self.event_id != event {
            return Err(EvolutionError::LinkedOffspringEventContextMismatch);
        }
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if schema.ploidy != 2 {
            return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
        }
        if schema.canonical_digest()? != self.schema_digest
            || chromosome_map.canonical_digest(schema)? != self.chromosome_map_digest
        {
            return Err(EvolutionError::LinkedOffspringAuthorityMismatch);
        }

        parent_a.validate_current(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a_profile,
            event,
            ParentRole::ParentA,
        )?;
        parent_b.validate_current(
            schema,
            chromosome_map,
            parent_b_source,
            parent_b_profile,
            event,
            ParentRole::ParentB,
        )?;

        let expected_parent_a = contribution_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a_profile,
            parent_a,
            ParentRole::ParentA,
        )?;
        let expected_parent_b = contribution_evidence(
            schema,
            chromosome_map,
            parent_b_source,
            parent_b_profile,
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

        let recomputed = assemble_diploid_linked_offspring_from_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a_profile,
            parent_a,
            parent_b_source,
            parent_b_profile,
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
pub struct DiploidLinkedOffspringProvenanceV2Digest([u8; 32]);

impl DiploidLinkedOffspringProvenanceV2Digest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for DiploidLinkedOffspringProvenanceV2Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DiploidLinkedOffspringProvenanceV2Digest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for DiploidLinkedOffspringProvenanceV2Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiploidLinkedOffspringDerivationV2 {
    pub child: PhasedHereditaryState,
    pub provenance: DiploidLinkedOffspringProvenanceV2,
}

/// Assemble one canonical diploid phased child from two independently validated
/// linked-gamete derivation evidence values.
///
/// ParentA and ParentB may use different exact recombination profiles. Fertilization
/// itself adds no random draw and no mutation; all meiotic stochasticity belongs to
/// the parent-specific gamete derivations.
pub fn assemble_diploid_linked_offspring_from_evidence(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    parent_a_source: &PhasedHereditaryState,
    parent_a_profile: &ChromosomeRecombinationProfile,
    parent_a: &LinkedGameteDerivationEvidence,
    parent_b_source: &PhasedHereditaryState,
    parent_b_profile: &ChromosomeRecombinationProfile,
    parent_b: &LinkedGameteDerivationEvidence,
    event: &ReproductionEventId,
) -> Result<DiploidLinkedOffspringDerivationV2, EvolutionError> {
    schema.validate()?;
    chromosome_map.validate(schema)?;
    if schema.ploidy != 2 {
        return Err(EvolutionError::ModeRequiresDiploid(schema.ploidy));
    }

    parent_a.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_profile,
        event,
        ParentRole::ParentA,
    )?;
    parent_b.validate_current(
        schema,
        chromosome_map,
        parent_b_source,
        parent_b_profile,
        event,
        ParentRole::ParentB,
    )?;

    let mut chromosomes = Vec::with_capacity(chromosome_map.chromosomes.len());
    for chromosome_id in chromosome_map.chromosomes.keys() {
        let from_a = parent_a
            .gamete()
            .chromosomes
            .get(chromosome_id)
            .ok_or(EvolutionError::LinkedGameteChromosomeSetMismatch)?
            .clone();
        let from_b = parent_b
            .gamete()
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
    let provenance = DiploidLinkedOffspringProvenanceV2 {
        derivation_version: DIPLOID_LINKED_OFFSPRING_DERIVATION_V2_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        event_id: event.clone(),
        parent_a: contribution_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a_profile,
            parent_a,
            ParentRole::ParentA,
        )?,
        parent_b: contribution_evidence(
            schema,
            chromosome_map,
            parent_b_source,
            parent_b_profile,
            parent_b,
            ParentRole::ParentB,
        )?,
        child_digest: child.canonical_digest(schema, chromosome_map)?,
    };

    Ok(DiploidLinkedOffspringDerivationV2 { child, provenance })
}

fn contribution_evidence(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    source: &PhasedHereditaryState,
    profile: &ChromosomeRecombinationProfile,
    derivation: &LinkedGameteDerivationEvidence,
    role: ParentRole,
) -> Result<LinkedGameteContributionEvidenceV2, EvolutionError> {
    Ok(LinkedGameteContributionEvidenceV2 {
        role,
        source_phased_state_digest: source.canonical_digest(schema, chromosome_map)?,
        recombination_profile_digest: profile.canonical_digest(schema, chromosome_map)?,
        gamete_digest: derivation.gamete().canonical_digest(schema, chromosome_map)?,
        derivation_evidence_digest: derivation.canonical_digest(),
    })
}

fn put_contribution(digest: &mut Sha256, contribution: &LinkedGameteContributionEvidenceV2) {
    digest.update([parent_role_tag(contribution.role)]);
    digest.update(contribution.source_phased_state_digest.as_bytes());
    digest.update(contribution.recombination_profile_digest.as_bytes());
    digest.update(contribution.gamete_digest.as_bytes());
    digest.update(contribution.derivation_evidence_digest.as_bytes());
}

fn parent_role_tag(role: ParentRole) -> u8 {
    match role {
        ParentRole::ClonalParent => 0,
        ParentRole::ParentA => 1,
        ParentRole::ParentB => 2,
    }
}
