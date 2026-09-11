use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AlleleId, AncestryAuthorityError, AncestryCopyId, ChromosomeId, ChromosomeMap,
    ChromosomeMapDigest, ChromosomeRecombinationProfile, DescendantAncestryDerivation,
    DescendantAncestryDerivationProvenanceDigest, EvolutionError, EvolutionOperatorProfile,
    GameteAncestryDerivation, HereditarySchema, HereditarySchemaDigest,
    LinkedGameteDerivationEvidence, LinkedMutationError, LinkedMutationExecution,
    LinkedMutationExecutionDigest, LinkedMutationOutcome, LocusId, ModeledAncestryGraph,
    MutationOrigin, MutationOriginDigest, ParentRole, PhasedAncestryState,
    PhasedAncestryStateDigest, PhasedHereditaryState, PhasedHereditaryStateDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const MUTATION_LINEAGE_STATE_VERSION: u32 = 1;
pub const MUTATION_LINEAGE_TRANSITION_VERSION: u32 = 1;

const MUTATION_LINEAGE_STATE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:mutation-lineage-state:v1\0";
const MUTATION_LINEAGE_TRANSITION_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:mutation-lineage-transition:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLineageEntry {
    pub ancestry_copy_id: AncestryCopyId,
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub allele: AlleleId,
    pub active_origin: Option<MutationOriginDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLineageEvent {
    pub origin: MutationOrigin,
    pub previous_active_origin: Option<MutationOriginDigest>,
}

impl MutationLineageEvent {
    pub fn origin_digest(&self) -> MutationOriginDigest {
        self.origin.canonical_digest()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLineageState {
    state_version: u32,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    phased_state_digest: PhasedHereditaryStateDigest,
    ancestry_state_digest: PhasedAncestryStateDigest,
    pub entries: Vec<MutationLineageEntry>,
    pub history: Vec<MutationLineageEvent>,
}

impl MutationLineageState {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        ancestry_state: &PhasedAncestryState,
    ) -> Result<(), MutationLineageError> {
        phased_state.validate(schema, chromosome_map)?;
        ancestry_state.validate_current(schema, chromosome_map, phased_state)?;
        if self.state_version != MUTATION_LINEAGE_STATE_VERSION {
            return Err(MutationLineageError::UnsupportedStateVersion(
                self.state_version,
            ));
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
            || self.phased_state_digest != phased_state.canonical_digest(schema, chromosome_map)?
            || self.ancestry_state_digest
                != ancestry_state.canonical_digest(schema, chromosome_map, phased_state)?
        {
            return Err(MutationLineageError::CurrentAuthorityMismatch);
        }
        if self
            .entries
            .windows(2)
            .any(|window| entry_cmp(&window[0], &window[1]) != Ordering::Less)
        {
            return Err(MutationLineageError::NonCanonicalEntryOrder);
        }
        if self.history.windows(2).any(|window| {
            origin_key(&window[0].origin_digest()) >= origin_key(&window[1].origin_digest())
        }) {
            return Err(MutationLineageError::NonCanonicalHistoryOrder);
        }

        let expected = expected_entries(schema, chromosome_map, phased_state, ancestry_state)?;
        if self.entries.len() != expected.len() {
            return Err(MutationLineageError::EntryCoverageMismatch);
        }
        for entry in &self.entries {
            let key = entry_key(entry);
            let expected_allele = expected
                .get(&key)
                .ok_or(MutationLineageError::EntryCoverageMismatch)?;
            if expected_allele != &entry.allele {
                return Err(MutationLineageError::CurrentAlleleMismatch {
                    copy: entry.ancestry_copy_id.clone(),
                    locus: entry.locus_id.clone(),
                });
            }
        }

        let mut history_by_digest = BTreeMap::new();
        for event in &self.history {
            let digest = event.origin_digest();
            let key = origin_key(&digest);
            if history_by_digest.insert(key, event).is_some() {
                return Err(MutationLineageError::DuplicateHistoryOrigin(digest));
            }
            let definition = chromosome_map
                .chromosomes
                .get(event.origin.chromosome_id())
                .ok_or_else(|| MutationLineageError::HistoryChromosomeMismatch {
                    origin: digest,
                })?;
            if !definition
                .loci
                .iter()
                .any(|mapped| &mapped.locus_id == event.origin.locus_id())
            {
                return Err(MutationLineageError::HistoryLocusMismatch { origin: digest });
            }
        }

        for event in &self.history {
            if let Some(previous) = event.previous_active_origin {
                let previous_event = history_by_digest
                    .get(&origin_key(&previous))
                    .ok_or(MutationLineageError::MissingPreviousOrigin(previous))?;
                if previous_event.origin.chromosome_id() != event.origin.chromosome_id() {
                    return Err(MutationLineageError::HistoryChromosomeMismatch {
                        origin: event.origin_digest(),
                    });
                }
                if previous_event.origin.locus_id() != event.origin.locus_id() {
                    return Err(MutationLineageError::HistoryLocusMismatch {
                        origin: event.origin_digest(),
                    });
                }
            }
        }

        let mut reachable = BTreeSet::new();
        for entry in &self.entries {
            if let Some(active) = entry.active_origin {
                let active_event = history_by_digest
                    .get(&origin_key(&active))
                    .ok_or(MutationLineageError::MissingActiveOrigin(active))?;
                if active_event.origin.chromosome_id() != &entry.chromosome_id
                    || active_event.origin.locus_id() != &entry.locus_id
                    || active_event.origin.derived_allele() != &entry.allele
                {
                    return Err(MutationLineageError::ActiveOriginMismatch {
                        copy: entry.ancestry_copy_id.clone(),
                        locus: entry.locus_id.clone(),
                    });
                }
                collect_reachable(active, &history_by_digest, &mut reachable)?;
            }
        }
        if reachable.len() != self.history.len() {
            return Err(MutationLineageError::OrphanHistory);
        }
        Ok(())
    }

    pub fn validate_root_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        ancestry_state: &PhasedAncestryState,
    ) -> Result<(), MutationLineageError> {
        let recomputed = initialize_root_mutation_lineage(
            schema,
            chromosome_map,
            phased_state,
            ancestry_state,
        )?;
        if &recomputed != self {
            return Err(MutationLineageError::RootReplayMismatch);
        }
        Ok(())
    }

    pub fn entry(
        &self,
        ancestry_copy_id: &AncestryCopyId,
        locus_id: &LocusId,
    ) -> Option<&MutationLineageEntry> {
        self.entries.iter().find(|entry| {
            &entry.ancestry_copy_id == ancestry_copy_id && &entry.locus_id == locus_id
        })
    }

    pub fn history_event(
        &self,
        digest: MutationOriginDigest,
    ) -> Option<&MutationLineageEvent> {
        self.history
            .iter()
            .find(|event| event.origin_digest() == digest)
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        phased_state: &PhasedHereditaryState,
        ancestry_state: &PhasedAncestryState,
    ) -> Result<MutationLineageStateDigest, MutationLineageError> {
        self.validate_current(schema, chromosome_map, phased_state, ancestry_state)?;
        let mut digest = Sha256::new();
        digest.update(MUTATION_LINEAGE_STATE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.state_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.phased_state_digest.as_bytes());
        digest.update(self.ancestry_state_digest.as_bytes());
        put_u64(&mut digest, self.entries.len() as u64);
        for entry in &self.entries {
            put_text(&mut digest, entry.ancestry_copy_id.as_str());
            put_text(&mut digest, entry.chromosome_id.as_str());
            put_text(&mut digest, entry.locus_id.as_str());
            put_text(&mut digest, entry.allele.as_str());
            match entry.active_origin {
                None => digest.update([0]),
                Some(origin) => {
                    digest.update([1]);
                    digest.update(origin.as_bytes());
                }
            }
        }
        put_u64(&mut digest, self.history.len() as u64);
        for event in &self.history {
            digest.update(event.origin_digest().as_bytes());
            match event.previous_active_origin {
                None => digest.update([0]),
                Some(previous) => {
                    digest.update([1]);
                    digest.update(previous.as_bytes());
                }
            }
        }
        Ok(MutationLineageStateDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationLineageStateDigest([u8; 32]);

impl MutationLineageStateDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MutationLineageStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutationLineageStateDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for MutationLineageStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLineageTransitionProvenance {
    transition_version: u32,
    parent_a_lineage_digest: MutationLineageStateDigest,
    parent_b_lineage_digest: MutationLineageStateDigest,
    descendant_provenance_digest: DescendantAncestryDerivationProvenanceDigest,
    mutation_execution_digest: LinkedMutationExecutionDigest,
    result_lineage_digest: MutationLineageStateDigest,
}

impl MutationLineageTransitionProvenance {
    pub fn canonical_digest(&self) -> MutationLineageTransitionProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(MUTATION_LINEAGE_TRANSITION_DIGEST_DOMAIN);
        put_u32(&mut digest, self.transition_version);
        digest.update(self.parent_a_lineage_digest.as_bytes());
        digest.update(self.parent_b_lineage_digest.as_bytes());
        digest.update(self.descendant_provenance_digest.as_bytes());
        digest.update(self.mutation_execution_digest.as_bytes());
        digest.update(self.result_lineage_digest.as_bytes());
        MutationLineageTransitionProvenanceDigest(digest.finalize().into())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MutationLineageTransitionProvenanceDigest([u8; 32]);

impl MutationLineageTransitionProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for MutationLineageTransitionProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MutationLineageTransitionProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationLineageTransitionResult {
    pub state: MutationLineageState,
    pub provenance: MutationLineageTransitionProvenance,
}

impl MutationLineageTransitionResult {
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        operators: &EvolutionOperatorProfile,
        parent_a_source: &PhasedHereditaryState,
        parent_a_source_ancestry: &PhasedAncestryState,
        parent_a_lineage: &MutationLineageState,
        parent_a_profile: &ChromosomeRecombinationProfile,
        parent_a_gamete: &LinkedGameteDerivationEvidence,
        parent_a_gamete_ancestry: &GameteAncestryDerivation,
        parent_b_source: &PhasedHereditaryState,
        parent_b_source_ancestry: &PhasedAncestryState,
        parent_b_lineage: &MutationLineageState,
        parent_b_profile: &ChromosomeRecombinationProfile,
        parent_b_gamete: &LinkedGameteDerivationEvidence,
        parent_b_gamete_ancestry: &GameteAncestryDerivation,
        descendant: &DescendantAncestryDerivation,
        graph: &ModeledAncestryGraph,
        execution: &LinkedMutationExecution,
    ) -> Result<(), MutationLineageError> {
        let recomputed = derive_descendant_mutation_lineage(
            schema,
            chromosome_map,
            operators,
            parent_a_source,
            parent_a_source_ancestry,
            parent_a_lineage,
            parent_a_profile,
            parent_a_gamete,
            parent_a_gamete_ancestry,
            parent_b_source,
            parent_b_source_ancestry,
            parent_b_lineage,
            parent_b_profile,
            parent_b_gamete,
            parent_b_gamete_ancestry,
            descendant,
            graph,
            execution,
        )?;
        if recomputed != *self {
            return Err(MutationLineageError::TransitionReplayMismatch);
        }
        Ok(())
    }
}

pub fn initialize_root_mutation_lineage(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    phased_state: &PhasedHereditaryState,
    ancestry_state: &PhasedAncestryState,
) -> Result<MutationLineageState, MutationLineageError> {
    phased_state.validate(schema, chromosome_map)?;
    ancestry_state.validate_current(schema, chromosome_map, phased_state)?;
    let expected = expected_entries(schema, chromosome_map, phased_state, ancestry_state)?;
    let mut entries: Vec<_> = expected
        .into_iter()
        .map(
            |((ancestry_copy_id, chromosome_id, locus_id), allele)| MutationLineageEntry {
                ancestry_copy_id,
                chromosome_id,
                locus_id,
                allele,
                active_origin: None,
            },
        )
        .collect();
    entries.sort_by(entry_cmp);
    let state = MutationLineageState {
        state_version: MUTATION_LINEAGE_STATE_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        phased_state_digest: phased_state.canonical_digest(schema, chromosome_map)?,
        ancestry_state_digest: ancestry_state.canonical_digest(schema, chromosome_map, phased_state)?,
        entries,
        history: Vec::new(),
    };
    state.validate_current(schema, chromosome_map, phased_state, ancestry_state)?;
    Ok(state)
}

#[allow(clippy::too_many_arguments)]
pub fn derive_descendant_mutation_lineage(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    operators: &EvolutionOperatorProfile,
    parent_a_source: &PhasedHereditaryState,
    parent_a_source_ancestry: &PhasedAncestryState,
    parent_a_lineage: &MutationLineageState,
    parent_a_profile: &ChromosomeRecombinationProfile,
    parent_a_gamete: &LinkedGameteDerivationEvidence,
    parent_a_gamete_ancestry: &GameteAncestryDerivation,
    parent_b_source: &PhasedHereditaryState,
    parent_b_source_ancestry: &PhasedAncestryState,
    parent_b_lineage: &MutationLineageState,
    parent_b_profile: &ChromosomeRecombinationProfile,
    parent_b_gamete: &LinkedGameteDerivationEvidence,
    parent_b_gamete_ancestry: &GameteAncestryDerivation,
    descendant: &DescendantAncestryDerivation,
    graph: &ModeledAncestryGraph,
    execution: &LinkedMutationExecution,
) -> Result<MutationLineageTransitionResult, MutationLineageError> {
    parent_a_lineage.validate_current(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_source_ancestry,
    )?;
    parent_b_lineage.validate_current(
        schema,
        chromosome_map,
        parent_b_source,
        parent_b_source_ancestry,
    )?;
    execution.validate_current(
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

    let mut edge_by_child_locus = BTreeMap::new();
    for edge in &descendant.materialization.edges {
        let key = (edge.child_copy_id.clone(), edge.locus_id.clone());
        if edge_by_child_locus.insert(key, edge).is_some() {
            return Err(MutationLineageError::DuplicateInheritanceEdge);
        }
    }

    let mut history_by_digest: BTreeMap<[u8; 32], MutationLineageEvent> = BTreeMap::new();
    let mut entries = Vec::with_capacity(execution.opportunities.len());

    for opportunity in &execution.opportunities {
        let edge = edge_by_child_locus
            .get(&(opportunity.ancestry_copy_id.clone(), opportunity.locus_id.clone()))
            .ok_or_else(|| MutationLineageError::InheritanceEdgeMissing {
                child: opportunity.ancestry_copy_id.clone(),
                locus: opportunity.locus_id.clone(),
            })?;
        if edge.chromosome_id != opportunity.chromosome_id
            || edge.parent_role != opportunity.parent_role
        {
            return Err(MutationLineageError::InheritanceContextMismatch);
        }
        let parent_lineage = match edge.parent_role {
            ParentRole::ParentA => parent_a_lineage,
            ParentRole::ParentB => parent_b_lineage,
            ParentRole::ClonalParent => return Err(MutationLineageError::UnsupportedParentRole),
        };
        let parent_entry = parent_lineage
            .entry(&edge.source_copy_id, &edge.locus_id)
            .ok_or_else(|| MutationLineageError::ParentEntryMissing {
                copy: edge.source_copy_id.clone(),
                locus: edge.locus_id.clone(),
            })?;
        if parent_entry.chromosome_id != edge.chromosome_id
            || parent_entry.allele != opportunity.ancestral_allele
        {
            return Err(MutationLineageError::InheritedAlleleMismatch);
        }

        if let Some(active) = parent_entry.active_origin {
            copy_history_chain(parent_lineage, active, &mut history_by_digest)?;
        }

        let (active_origin, expected_allele) = match &opportunity.outcome {
            LinkedMutationOutcome::NoMutation { .. } => {
                (parent_entry.active_origin, parent_entry.allele.clone())
            }
            LinkedMutationOutcome::Substitution { origin, .. } => {
                if origin.ancestry_copy_id() != &opportunity.ancestry_copy_id
                    || origin.chromosome_id() != &opportunity.chromosome_id
                    || origin.locus_id() != &opportunity.locus_id
                    || origin.ancestral_allele() != &parent_entry.allele
                {
                    return Err(MutationLineageError::MutationOriginContextMismatch);
                }
                let digest = origin.canonical_digest();
                let event = MutationLineageEvent {
                    origin: origin.clone(),
                    previous_active_origin: parent_entry.active_origin,
                };
                insert_history_event(&mut history_by_digest, event)?;
                (Some(digest), origin.derived_allele().clone())
            }
        };

        let current_allele = allele_for_copy_locus(
            schema,
            chromosome_map,
            &execution.mutated_child,
            &execution.mutated_child_ancestry,
            &opportunity.ancestry_copy_id,
            &opportunity.chromosome_id,
            &opportunity.locus_id,
        )?;
        if current_allele != expected_allele {
            return Err(MutationLineageError::CurrentAlleleMismatch {
                copy: opportunity.ancestry_copy_id.clone(),
                locus: opportunity.locus_id.clone(),
            });
        }
        entries.push(MutationLineageEntry {
            ancestry_copy_id: opportunity.ancestry_copy_id.clone(),
            chromosome_id: opportunity.chromosome_id.clone(),
            locus_id: opportunity.locus_id.clone(),
            allele: current_allele,
            active_origin,
        });
    }

    entries.sort_by(entry_cmp);
    let history: Vec<_> = history_by_digest.into_values().collect();
    let state = MutationLineageState {
        state_version: MUTATION_LINEAGE_STATE_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        phased_state_digest: execution
            .mutated_child
            .canonical_digest(schema, chromosome_map)?,
        ancestry_state_digest: execution.mutated_child_ancestry.canonical_digest(
            schema,
            chromosome_map,
            &execution.mutated_child,
        )?,
        entries,
        history,
    };
    state.validate_current(
        schema,
        chromosome_map,
        &execution.mutated_child,
        &execution.mutated_child_ancestry,
    )?;

    let parent_a_lineage_digest = parent_a_lineage.canonical_digest(
        schema,
        chromosome_map,
        parent_a_source,
        parent_a_source_ancestry,
    )?;
    let parent_b_lineage_digest = parent_b_lineage.canonical_digest(
        schema,
        chromosome_map,
        parent_b_source,
        parent_b_source_ancestry,
    )?;
    let result_lineage_digest = state.canonical_digest(
        schema,
        chromosome_map,
        &execution.mutated_child,
        &execution.mutated_child_ancestry,
    )?;
    let provenance = MutationLineageTransitionProvenance {
        transition_version: MUTATION_LINEAGE_TRANSITION_VERSION,
        parent_a_lineage_digest,
        parent_b_lineage_digest,
        descendant_provenance_digest: descendant.provenance.canonical_digest(),
        mutation_execution_digest: execution.canonical_digest(),
        result_lineage_digest,
    };
    Ok(MutationLineageTransitionResult { state, provenance })
}

fn expected_entries(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    phased_state: &PhasedHereditaryState,
    ancestry_state: &PhasedAncestryState,
) -> Result<BTreeMap<(AncestryCopyId, ChromosomeId, LocusId), AlleleId>, MutationLineageError> {
    let mut expected = BTreeMap::new();
    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        let genetic = phased_state
            .chromosomes
            .get(chromosome_id)
            .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
        let ancestry = ancestry_state
            .chromosomes
            .get(chromosome_id)
            .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
        for class in &ancestry.classes {
            let haplotype = genetic
                .haplotypes
                .get(usize::from(class.representative_haplotype_slot))
                .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
            for copy_id in &class.copy_ids {
                for (index, mapped_locus) in definition.loci.iter().enumerate() {
                    let allele = haplotype
                        .alleles
                        .get(index)
                        .ok_or_else(|| MutationLineageError::LocusMissing(mapped_locus.locus_id.clone()))?
                        .clone();
                    let key = (
                        copy_id.clone(),
                        chromosome_id.clone(),
                        mapped_locus.locus_id.clone(),
                    );
                    if expected.insert(key, allele).is_some() {
                        return Err(MutationLineageError::EntryCoverageMismatch);
                    }
                }
            }
        }
    }
    Ok(expected)
}

fn allele_for_copy_locus(
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
    phased_state: &PhasedHereditaryState,
    ancestry_state: &PhasedAncestryState,
    copy_id: &AncestryCopyId,
    chromosome_id: &ChromosomeId,
    locus_id: &LocusId,
) -> Result<AlleleId, MutationLineageError> {
    phased_state.validate(schema, chromosome_map)?;
    ancestry_state.validate_current(schema, chromosome_map, phased_state)?;
    let definition = chromosome_map
        .chromosomes
        .get(chromosome_id)
        .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
    let locus_index = definition
        .loci
        .iter()
        .position(|mapped| &mapped.locus_id == locus_id)
        .ok_or_else(|| MutationLineageError::LocusMissing(locus_id.clone()))?;
    let ancestry = ancestry_state
        .chromosomes
        .get(chromosome_id)
        .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
    let class = ancestry
        .classes
        .iter()
        .find(|class| class.copy_ids.iter().any(|candidate| candidate == copy_id))
        .ok_or_else(|| MutationLineageError::CopyMissing(copy_id.clone()))?;
    let genetic = phased_state
        .chromosomes
        .get(chromosome_id)
        .ok_or_else(|| MutationLineageError::ChromosomeMissing(chromosome_id.clone()))?;
    genetic
        .haplotypes
        .get(usize::from(class.representative_haplotype_slot))
        .and_then(|haplotype| haplotype.alleles.get(locus_index))
        .cloned()
        .ok_or_else(|| MutationLineageError::LocusMissing(locus_id.clone()))
}

fn copy_history_chain(
    state: &MutationLineageState,
    active: MutationOriginDigest,
    target: &mut BTreeMap<[u8; 32], MutationLineageEvent>,
) -> Result<(), MutationLineageError> {
    let mut cursor = Some(active);
    let mut visited = BTreeSet::new();
    while let Some(digest) = cursor {
        let key = origin_key(&digest);
        if !visited.insert(key) {
            return Err(MutationLineageError::HistoryCycle(digest));
        }
        let event = state
            .history_event(digest)
            .ok_or(MutationLineageError::MissingActiveOrigin(digest))?
            .clone();
        cursor = event.previous_active_origin;
        insert_history_event(target, event)?;
    }
    Ok(())
}

fn insert_history_event(
    target: &mut BTreeMap<[u8; 32], MutationLineageEvent>,
    event: MutationLineageEvent,
) -> Result<(), MutationLineageError> {
    let digest = event.origin_digest();
    let key = origin_key(&digest);
    if let Some(existing) = target.get(&key) {
        if existing != &event {
            return Err(MutationLineageError::HistoryIdentityCollision(digest));
        }
        return Ok(());
    }
    target.insert(key, event);
    Ok(())
}

fn collect_reachable(
    active: MutationOriginDigest,
    history: &BTreeMap<[u8; 32], &MutationLineageEvent>,
    reachable: &mut BTreeSet<[u8; 32]>,
) -> Result<(), MutationLineageError> {
    let mut cursor = Some(active);
    let mut local = BTreeSet::new();
    while let Some(digest) = cursor {
        let key = origin_key(&digest);
        if !local.insert(key) {
            return Err(MutationLineageError::HistoryCycle(digest));
        }
        reachable.insert(key);
        let event = history
            .get(&key)
            .ok_or(MutationLineageError::MissingActiveOrigin(digest))?;
        cursor = event.previous_active_origin;
    }
    Ok(())
}

fn entry_key(entry: &MutationLineageEntry) -> (AncestryCopyId, ChromosomeId, LocusId) {
    (
        entry.ancestry_copy_id.clone(),
        entry.chromosome_id.clone(),
        entry.locus_id.clone(),
    )
}

fn entry_cmp(left: &MutationLineageEntry, right: &MutationLineageEntry) -> Ordering {
    entry_key(left).cmp(&entry_key(right))
}

fn origin_key(digest: &MutationOriginDigest) -> [u8; 32] {
    *digest.as_bytes()
}

#[derive(Debug)]
pub enum MutationLineageError {
    Evolution(EvolutionError),
    Ancestry(AncestryAuthorityError),
    LinkedMutation(LinkedMutationError),
    UnsupportedStateVersion(u32),
    CurrentAuthorityMismatch,
    RootReplayMismatch,
    TransitionReplayMismatch,
    NonCanonicalEntryOrder,
    NonCanonicalHistoryOrder,
    EntryCoverageMismatch,
    CurrentAlleleMismatch { copy: AncestryCopyId, locus: LocusId },
    ChromosomeMissing(ChromosomeId),
    LocusMissing(LocusId),
    CopyMissing(AncestryCopyId),
    DuplicateHistoryOrigin(MutationOriginDigest),
    HistoryIdentityCollision(MutationOriginDigest),
    MissingActiveOrigin(MutationOriginDigest),
    MissingPreviousOrigin(MutationOriginDigest),
    HistoryCycle(MutationOriginDigest),
    OrphanHistory,
    HistoryChromosomeMismatch { origin: MutationOriginDigest },
    HistoryLocusMismatch { origin: MutationOriginDigest },
    ActiveOriginMismatch { copy: AncestryCopyId, locus: LocusId },
    DuplicateInheritanceEdge,
    InheritanceEdgeMissing { child: AncestryCopyId, locus: LocusId },
    InheritanceContextMismatch,
    ParentEntryMissing { copy: AncestryCopyId, locus: LocusId },
    InheritedAlleleMismatch,
    MutationOriginContextMismatch,
    UnsupportedParentRole,
}

impl From<EvolutionError> for MutationLineageError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<AncestryAuthorityError> for MutationLineageError {
    fn from(value: AncestryAuthorityError) -> Self {
        Self::Ancestry(value)
    }
}

impl From<LinkedMutationError> for MutationLineageError {
    fn from(value: LinkedMutationError) -> Self {
        Self::LinkedMutation(value)
    }
}

impl fmt::Display for MutationLineageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Ancestry(error) => write!(f, "ancestry authority error: {error}"),
            Self::LinkedMutation(error) => write!(f, "linked mutation authority error: {error}"),
            Self::UnsupportedStateVersion(version) => {
                write!(f, "unsupported mutation-lineage state version {version}")
            }
            Self::CurrentAuthorityMismatch => {
                write!(f, "mutation-lineage state does not match exact current authority")
            }
            Self::RootReplayMismatch => write!(f, "restored root mutation-lineage state does not replay"),
            Self::TransitionReplayMismatch => write!(f, "restored mutation-lineage transition does not replay"),
            Self::NonCanonicalEntryOrder => write!(f, "mutation-lineage entries are not in canonical order"),
            Self::NonCanonicalHistoryOrder => write!(f, "mutation-lineage history is not in canonical order"),
            Self::EntryCoverageMismatch => write!(f, "mutation-lineage entry coverage does not match current ancestry"),
            Self::CurrentAlleleMismatch { copy, locus } => write!(
                f,
                "mutation-lineage allele mismatch for copy {} at locus {}",
                copy.as_str(),
                locus.as_str()
            ),
            Self::ChromosomeMissing(id) => write!(f, "mutation-lineage chromosome {} is missing", id.as_str()),
            Self::LocusMissing(id) => write!(f, "mutation-lineage locus {} is missing", id.as_str()),
            Self::CopyMissing(id) => write!(f, "mutation-lineage ancestry copy {} is missing", id.as_str()),
            Self::DuplicateHistoryOrigin(origin) => write!(f, "duplicate mutation history origin {origin}"),
            Self::HistoryIdentityCollision(origin) => write!(f, "mutation history identity collision for {origin}"),
            Self::MissingActiveOrigin(origin) => write!(f, "active mutation origin {origin} is absent from history"),
            Self::MissingPreviousOrigin(origin) => write!(f, "previous mutation origin {origin} is absent from history"),
            Self::HistoryCycle(origin) => write!(f, "mutation history cycle detected at {origin}"),
            Self::OrphanHistory => write!(f, "mutation history contains events unreachable from extant active origins"),
            Self::HistoryChromosomeMismatch { origin } => write!(f, "mutation history chromosome mismatch at {origin}"),
            Self::HistoryLocusMismatch { origin } => write!(f, "mutation history locus mismatch at {origin}"),
            Self::ActiveOriginMismatch { copy, locus } => write!(
                f,
                "active mutation origin does not explain copy {} locus {}",
                copy.as_str(),
                locus.as_str()
            ),
            Self::DuplicateInheritanceEdge => write!(f, "duplicate descendant inheritance edge"),
            Self::InheritanceEdgeMissing { child, locus } => write!(
                f,
                "descendant inheritance edge is missing for child {} locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::InheritanceContextMismatch => write!(f, "descendant inheritance context mismatch"),
            Self::ParentEntryMissing { copy, locus } => write!(
                f,
                "parent mutation-lineage entry is missing for copy {} locus {}",
                copy.as_str(),
                locus.as_str()
            ),
            Self::InheritedAlleleMismatch => write!(f, "inherited mutation-lineage allele does not match descendant ancestry"),
            Self::MutationOriginContextMismatch => write!(f, "realized mutation origin does not match inherited lineage context"),
            Self::UnsupportedParentRole => write!(f, "mutation-lineage transmission supports ParentA/ParentB descendants only"),
        }
    }
}

impl Error for MutationLineageError {}
