use crate::{
    assemble_diploid_linked_offspring_from_evidence,
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AncestryCopyId, AncestryGeneration, AncestryGraphNode, AncestryRetentionSet,
    AncestrySimplificationError, ChromosomeMap, ChromosomeMapDigest,
    ChromosomeRecombinationProfile, DescendantAncestryDerivation,
    DescendantAncestryDerivationProvenanceDigest, DescendantAncestryError, EvolutionError,
    GameteAncestryDerivation, HereditarySchema, HereditarySchemaDigest,
    LinkedGameteDerivationEvidence, PhasedAncestryState, PhasedHereditaryState,
    SimplifiedAncestryEdge, SimplifiedAncestryGraph, SimplifiedAncestryGraphDigest,
    SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const SIMPLIFIED_ANCESTRY_APPEND_VERSION: u32 = 1;
pub const SIMPLIFIED_ANCESTRY_RESIMPLIFICATION_VERSION: u32 = 1;

const APPEND_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:simplified-ancestry-append:v1\0";
const RESIMPLIFICATION_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:simplified-ancestry-resimplification:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimplifiedAncestryAppendProvenance {
    append_version: u32,
    source_graph_digest: SimplifiedAncestryGraphDigest,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    descendant_provenance_digest: DescendantAncestryDerivationProvenanceDigest,
    child_generation: AncestryGeneration,
    next_retention: AncestryRetentionSet,
    appended_node_ids: Vec<AncestryCopyId>,
    appended_edges: Vec<SimplifiedAncestryEdge>,
    result_graph_digest: SimplifiedAncestryGraphDigest,
}

impl SimplifiedAncestryAppendProvenance {
    pub fn appended_node_ids(&self) -> &[AncestryCopyId] {
        &self.appended_node_ids
    }

    pub fn appended_edges(&self) -> &[SimplifiedAncestryEdge] {
        &self.appended_edges
    }

    pub fn canonical_digest(&self) -> SimplifiedAncestryAppendProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(APPEND_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.append_version);
        digest.update(self.source_graph_digest.as_bytes());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.descendant_provenance_digest.as_bytes());
        put_u64(&mut digest, self.child_generation.get());
        put_retention(&mut digest, &self.next_retention);
        put_u64(&mut digest, self.appended_node_ids.len() as u64);
        for id in &self.appended_node_ids {
            put_text(&mut digest, id.as_str());
        }
        put_u64(&mut digest, self.appended_edges.len() as u64);
        for edge in &self.appended_edges {
            put_edge(&mut digest, edge);
        }
        digest.update(self.result_graph_digest.as_bytes());
        SimplifiedAncestryAppendProvenanceDigest(digest.finalize().into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        source_graph: &SimplifiedAncestryGraph,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
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
        child_generation: AncestryGeneration,
        next_retention: &AncestryRetentionSet,
        result_graph: &SimplifiedAncestryGraph,
    ) -> Result<(), SimplifiedAncestryForwardError> {
        if self.append_version != SIMPLIFIED_ANCESTRY_APPEND_VERSION {
            return Err(SimplifiedAncestryForwardError::UnsupportedAppendVersion(
                self.append_version,
            ));
        }
        let recomputed = append_descendant_ancestry_to_simplified_graph(
            source_graph,
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
            descendant,
            child_generation,
            next_retention,
        )?;
        if recomputed.graph != *result_graph || recomputed.provenance != *self {
            return Err(SimplifiedAncestryForwardError::AppendReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimplifiedAncestryAppendProvenanceDigest([u8; 32]);

impl SimplifiedAncestryAppendProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SimplifiedAncestryAppendProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimplifiedAncestryAppendProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimplifiedAncestryAppendResult {
    pub graph: SimplifiedAncestryGraph,
    pub provenance: SimplifiedAncestryAppendProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn append_descendant_ancestry_to_simplified_graph(
    source_graph: &SimplifiedAncestryGraph,
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
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
    child_generation: AncestryGeneration,
    next_retention: &AncestryRetentionSet,
) -> Result<SimplifiedAncestryAppendResult, SimplifiedAncestryForwardError> {
    source_graph.validate_current(schema, chromosome_map)?;
    validate_retention_local(next_retention)?;

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

    let source_digest = source_graph.canonical_digest(schema, chromosome_map)?;
    let mut graph = source_graph.clone();
    let mut appended_node_ids = Vec::new();
    let mut appended_edges = Vec::new();

    for copy in &descendant.materialization.descendant_copies {
        if graph.nodes.contains_key(&copy.child_copy_id) {
            return Err(SimplifiedAncestryForwardError::ChildAlreadyExists(
                copy.child_copy_id.clone(),
            ));
        }
        graph.nodes.insert(
            copy.child_copy_id.clone(),
            AncestryGraphNode {
                copy_id: copy.child_copy_id.clone(),
                chromosome_id: copy.chromosome_id.clone(),
                generation: child_generation,
                birth_event: Some(event.clone()),
            },
        );
        appended_node_ids.push(copy.child_copy_id.clone());
    }
    appended_node_ids.sort();

    for edge in &descendant.materialization.edges {
        let source = source_graph
            .nodes
            .get(&edge.source_copy_id)
            .ok_or_else(|| SimplifiedAncestryForwardError::SourceNodeMissing(
                edge.source_copy_id.clone(),
            ))?;
        if source.generation >= child_generation {
            return Err(SimplifiedAncestryForwardError::GenerationOrderViolation {
                source: source.copy_id.clone(),
                child: edge.child_copy_id.clone(),
            });
        }
        let simplified_edge = SimplifiedAncestryEdge {
            chromosome_id: edge.chromosome_id.clone(),
            locus_id: edge.locus_id.clone(),
            source_copy_id: edge.source_copy_id.clone(),
            child_copy_id: edge.child_copy_id.clone(),
        };
        graph.edges.push(simplified_edge.clone());
        appended_edges.push(simplified_edge);
    }
    graph.edges.sort();
    appended_edges.sort();
    graph.retention = next_retention.clone();
    graph.validate_current(schema, chromosome_map)?;

    let result_graph_digest = graph.canonical_digest(schema, chromosome_map)?;
    let provenance = SimplifiedAncestryAppendProvenance {
        append_version: SIMPLIFIED_ANCESTRY_APPEND_VERSION,
        source_graph_digest: source_digest,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        descendant_provenance_digest: descendant.provenance.canonical_digest(),
        child_generation,
        next_retention: next_retention.clone(),
        appended_node_ids,
        appended_edges,
        result_graph_digest,
    };
    Ok(SimplifiedAncestryAppendResult { graph, provenance })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimplifiedAncestryResimplificationProvenance {
    resimplification_version: u32,
    source_graph_digest: SimplifiedAncestryGraphDigest,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    next_retention: AncestryRetentionSet,
    retained_node_ids: Vec<AncestryCopyId>,
    result_graph_digest: SimplifiedAncestryGraphDigest,
}

impl SimplifiedAncestryResimplificationProvenance {
    pub fn retained_node_ids(&self) -> &[AncestryCopyId] {
        &self.retained_node_ids
    }

    pub fn canonical_digest(&self) -> SimplifiedAncestryResimplificationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(RESIMPLIFICATION_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.resimplification_version);
        digest.update(self.source_graph_digest.as_bytes());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_retention(&mut digest, &self.next_retention);
        put_u64(&mut digest, self.retained_node_ids.len() as u64);
        for id in &self.retained_node_ids {
            put_text(&mut digest, id.as_str());
        }
        digest.update(self.result_graph_digest.as_bytes());
        SimplifiedAncestryResimplificationProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        source_graph: &SimplifiedAncestryGraph,
        next_retention: &AncestryRetentionSet,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        result_graph: &SimplifiedAncestryGraph,
    ) -> Result<(), SimplifiedAncestryForwardError> {
        if self.resimplification_version != SIMPLIFIED_ANCESTRY_RESIMPLIFICATION_VERSION {
            return Err(
                SimplifiedAncestryForwardError::UnsupportedResimplificationVersion(
                    self.resimplification_version,
                ),
            );
        }
        let recomputed = resimplify_simplified_ancestry(
            source_graph,
            next_retention,
            schema,
            chromosome_map,
        )?;
        if recomputed.graph != *result_graph || recomputed.provenance != *self {
            return Err(SimplifiedAncestryForwardError::ResimplificationReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimplifiedAncestryResimplificationProvenanceDigest([u8; 32]);

impl SimplifiedAncestryResimplificationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SimplifiedAncestryResimplificationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimplifiedAncestryResimplificationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimplifiedAncestryResimplificationResult {
    pub graph: SimplifiedAncestryGraph,
    pub provenance: SimplifiedAncestryResimplificationProvenance,
}

pub fn resimplify_simplified_ancestry(
    source_graph: &SimplifiedAncestryGraph,
    next_retention: &AncestryRetentionSet,
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
) -> Result<SimplifiedAncestryResimplificationResult, SimplifiedAncestryForwardError> {
    source_graph.validate_current(schema, chromosome_map)?;
    validate_retention_local(next_retention)?;
    for id in next_retention
        .focal_copy_ids
        .iter()
        .chain(next_retention.protected_copy_ids.iter())
    {
        if !source_graph.nodes.contains_key(id) {
            return Err(SimplifiedAncestryForwardError::MissingRetainedCopy(
                id.clone(),
            ));
        }
    }

    let selected: BTreeSet<_> = next_retention
        .focal_copy_ids
        .iter()
        .chain(next_retention.protected_copy_ids.iter())
        .cloned()
        .collect();
    let mut globally_retained = BTreeSet::new();
    let mut simplified_edges = BTreeSet::new();

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        for mapped_locus in &definition.loci {
            let locus_id = &mapped_locus.locus_id;
            let incoming: BTreeMap<AncestryCopyId, AncestryCopyId> = source_graph
                .edges
                .iter()
                .filter(|edge| {
                    edge.chromosome_id == *chromosome_id && edge.locus_id == *locus_id
                })
                .map(|edge| (edge.child_copy_id.clone(), edge.source_copy_id.clone()))
                .collect();

            let mut work: Vec<_> = selected
                .iter()
                .filter(|id| {
                    source_graph
                        .nodes
                        .get(*id)
                        .is_some_and(|node| node.chromosome_id == *chromosome_id)
                })
                .cloned()
                .collect();
            let mut active_nodes = BTreeSet::new();
            let mut active_edges = Vec::new();
            while let Some(child) = work.pop() {
                if !active_nodes.insert(child.clone()) {
                    continue;
                }
                if let Some(parent) = incoming.get(&child) {
                    active_edges.push((parent.clone(), child.clone()));
                    work.push(parent.clone());
                }
            }
            if active_nodes.is_empty() {
                continue;
            }

            let mut outdegree: BTreeMap<AncestryCopyId, usize> = BTreeMap::new();
            for (source, _) in &active_edges {
                *outdegree.entry(source.clone()).or_default() += 1;
            }
            let mut retained_locus = BTreeSet::new();
            for id in &active_nodes {
                let node = &source_graph.nodes[id];
                if selected.contains(id)
                    || node.birth_event.is_none()
                    || outdegree.get(id).copied().unwrap_or(0) != 1
                {
                    retained_locus.insert(id.clone());
                }
            }
            globally_retained.extend(retained_locus.iter().cloned());

            for child in &retained_locus {
                let node = &source_graph.nodes[child];
                if node.birth_event.is_none() {
                    continue;
                }
                let mut cursor = child.clone();
                let source = loop {
                    let parent = incoming.get(&cursor).ok_or_else(|| {
                        SimplifiedAncestryForwardError::MissingActiveParent {
                            child: child.clone(),
                            locus: locus_id.clone(),
                        }
                    })?;
                    if retained_locus.contains(parent) {
                        break parent.clone();
                    }
                    cursor = parent.clone();
                };
                simplified_edges.insert(SimplifiedAncestryEdge {
                    chromosome_id: chromosome_id.clone(),
                    locus_id: locus_id.clone(),
                    source_copy_id: source,
                    child_copy_id: child.clone(),
                });
            }
        }
    }

    globally_retained.extend(selected.iter().cloned());
    let nodes = globally_retained
        .iter()
        .map(|id| {
            source_graph
                .nodes
                .get(id)
                .cloned()
                .map(|node| (id.clone(), node))
                .ok_or_else(|| SimplifiedAncestryForwardError::MissingRetainedCopy(id.clone()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let edges = simplified_edges.into_iter().collect();
    let graph = SimplifiedAncestryGraph {
        graph_version: SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
        schema_digest: source_graph.schema_digest,
        chromosome_map_digest: source_graph.chromosome_map_digest,
        profile: source_graph.profile,
        retention: next_retention.clone(),
        nodes,
        edges,
    };
    graph.validate_current(schema, chromosome_map)?;

    let provenance = SimplifiedAncestryResimplificationProvenance {
        resimplification_version: SIMPLIFIED_ANCESTRY_RESIMPLIFICATION_VERSION,
        source_graph_digest: source_graph.canonical_digest(schema, chromosome_map)?,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        next_retention: next_retention.clone(),
        retained_node_ids: globally_retained.into_iter().collect(),
        result_graph_digest: graph.canonical_digest(schema, chromosome_map)?,
    };
    Ok(SimplifiedAncestryResimplificationResult { graph, provenance })
}

fn validate_retention_local(
    retention: &AncestryRetentionSet,
) -> Result<(), SimplifiedAncestryForwardError> {
    if retention.retention_version != crate::ANCESTRY_RETENTION_SET_VERSION
        || retention.focal_copy_ids.is_empty()
        || retention.focal_copy_ids.windows(2).any(|w| w[0] >= w[1])
        || retention.protected_copy_ids.windows(2).any(|w| w[0] >= w[1])
        || retention
            .focal_copy_ids
            .iter()
            .any(|id| retention.protected_copy_ids.binary_search(id).is_ok())
    {
        return Err(SimplifiedAncestryForwardError::InvalidRetentionAuthority);
    }
    Ok(())
}

fn put_retention(digest: &mut Sha256, retention: &AncestryRetentionSet) {
    put_u32(digest, retention.retention_version);
    put_u64(digest, retention.focal_copy_ids.len() as u64);
    for id in &retention.focal_copy_ids {
        put_text(digest, id.as_str());
    }
    put_u64(digest, retention.protected_copy_ids.len() as u64);
    for id in &retention.protected_copy_ids {
        put_text(digest, id.as_str());
    }
}

fn put_edge(digest: &mut Sha256, edge: &SimplifiedAncestryEdge) {
    put_text(digest, edge.chromosome_id.as_str());
    put_text(digest, edge.locus_id.as_str());
    put_text(digest, edge.source_copy_id.as_str());
    put_text(digest, edge.child_copy_id.as_str());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SimplifiedAncestryForwardError {
    Simplification(AncestrySimplificationError),
    Descendant(DescendantAncestryError),
    Evolution(EvolutionError),
    UnsupportedAppendVersion(u32),
    UnsupportedResimplificationVersion(u32),
    InvalidRetentionAuthority,
    MissingRetainedCopy(AncestryCopyId),
    ChildAlreadyExists(AncestryCopyId),
    SourceNodeMissing(AncestryCopyId),
    GenerationOrderViolation {
        source: AncestryCopyId,
        child: AncestryCopyId,
    },
    MissingActiveParent {
        child: AncestryCopyId,
        locus: crate::LocusId,
    },
    AppendReplayMismatch,
    ResimplificationReplayMismatch,
}

impl From<AncestrySimplificationError> for SimplifiedAncestryForwardError {
    fn from(value: AncestrySimplificationError) -> Self {
        Self::Simplification(value)
    }
}

impl From<DescendantAncestryError> for SimplifiedAncestryForwardError {
    fn from(value: DescendantAncestryError) -> Self {
        Self::Descendant(value)
    }
}

impl From<EvolutionError> for SimplifiedAncestryForwardError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl fmt::Display for SimplifiedAncestryForwardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Simplification(error) => {
                write!(f, "simplified ancestry authority error: {error}")
            }
            Self::Descendant(error) => {
                write!(f, "descendant ancestry authority error: {error}")
            }
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::UnsupportedAppendVersion(version) => write!(
                f,
                "unsupported simplified ancestry append version {version}"
            ),
            Self::UnsupportedResimplificationVersion(version) => write!(
                f,
                "unsupported simplified ancestry resimplification version {version}"
            ),
            Self::InvalidRetentionAuthority => {
                write!(f, "invalid next simplified-ancestry retention authority")
            }
            Self::MissingRetainedCopy(id) => write!(
                f,
                "retained ancestry copy {} is absent from simplified graph",
                id.as_str()
            ),
            Self::ChildAlreadyExists(id) => write!(
                f,
                "descendant ancestry copy {} already exists in simplified graph",
                id.as_str()
            ),
            Self::SourceNodeMissing(id) => write!(
                f,
                "descendant source ancestry copy {} is absent from simplified graph",
                id.as_str()
            ),
            Self::GenerationOrderViolation { source, child } => write!(
                f,
                "simplified ancestry source {} is not earlier than child {}",
                source.as_str(),
                child.as_str()
            ),
            Self::MissingActiveParent { child, locus } => write!(
                f,
                "cannot trace compressed ancestry parent for {} at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::AppendReplayMismatch => {
                write!(f, "simplified ancestry append deterministic replay mismatch")
            }
            Self::ResimplificationReplayMismatch => write!(
                f,
                "simplified ancestry resimplification deterministic replay mismatch"
            ),
        }
    }
}

impl Error for SimplifiedAncestryForwardError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Simplification(error) => Some(error),
            Self::Descendant(error) => Some(error),
            Self::Evolution(error) => Some(error),
            _ => None,
        }
    }
}
