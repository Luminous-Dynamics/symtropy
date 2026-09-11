use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AncestryCopyId, ChromosomeId, ChromosomeMap, ChromosomeMapDigest,
    ChromosomeRecombinationProfile, DescendantAncestryDerivation,
    DescendantAncestryDerivationProvenanceDigest, DescendantAncestryError, EvolutionError,
    GameteAncestryDerivation, HereditarySchema, HereditarySchemaDigest,
    LinkedGameteDerivationEvidence, LocusId, PhasedAncestryState, PhasedHereditaryState,
    ReproductionEventId,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const MODELED_ANCESTRY_GRAPH_VERSION: u32 = 1;
pub const ANCESTRY_GRAPH_APPEND_VERSION: u32 = 1;

const GRAPH_DIGEST_DOMAIN: &[u8] = b"symtropy:evolution:modeled-ancestry-graph:v1\0";
const APPEND_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:ancestry-graph-append-provenance:v1\0";

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct AncestryGeneration(u64);

impl AncestryGeneration {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryGraphNode {
    pub copy_id: AncestryCopyId,
    pub chromosome_id: ChromosomeId,
    pub generation: AncestryGeneration,
    pub birth_event: Option<ReproductionEventId>,
}

impl AncestryGraphNode {
    pub fn root(
        copy_id: AncestryCopyId,
        chromosome_id: ChromosomeId,
        generation: AncestryGeneration,
    ) -> Self {
        Self {
            copy_id,
            chromosome_id,
            generation,
            birth_event: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AncestryGraphEdge {
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub source_copy_id: AncestryCopyId,
    pub child_copy_id: AncestryCopyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModeledAncestryGraph {
    pub graph_version: u32,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub nodes: BTreeMap<AncestryCopyId, AncestryGraphNode>,
    pub edges: Vec<AncestryGraphEdge>,
}

impl ModeledAncestryGraph {
    pub fn new_roots(
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        roots: impl IntoIterator<Item = AncestryGraphNode>,
    ) -> Result<Self, AncestryGraphError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        let mut nodes = BTreeMap::new();
        for root in roots {
            if root.birth_event.is_some() {
                return Err(AncestryGraphError::RootHasBirthEvent(root.copy_id));
            }
            let id = root.copy_id.clone();
            if nodes.insert(id.clone(), root).is_some() {
                return Err(AncestryGraphError::DuplicateNode(id));
            }
        }
        if nodes.is_empty() {
            return Err(AncestryGraphError::EmptyGraph);
        }
        let graph = Self {
            graph_version: MODELED_ANCESTRY_GRAPH_VERSION,
            schema_digest: schema.canonical_digest()?,
            chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
            nodes,
            edges: Vec::new(),
        };
        graph.validate_current(schema, chromosome_map)?;
        Ok(graph)
    }

    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<(), AncestryGraphError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if self.graph_version != MODELED_ANCESTRY_GRAPH_VERSION {
            return Err(AncestryGraphError::UnsupportedGraphVersion(
                self.graph_version,
            ));
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(AncestryGraphError::CurrentAuthorityMismatch);
        }
        if self.nodes.is_empty() {
            return Err(AncestryGraphError::EmptyGraph);
        }

        for (key, node) in &self.nodes {
            if key != &node.copy_id {
                return Err(AncestryGraphError::NodeKeyMismatch {
                    key: key.clone(),
                    value: node.copy_id.clone(),
                });
            }
            if !chromosome_map.chromosomes.contains_key(&node.chromosome_id) {
                return Err(AncestryGraphError::UnknownChromosome(
                    node.chromosome_id.clone(),
                ));
            }
        }

        if self.edges.windows(2).any(|window| window[0] >= window[1]) {
            return Err(AncestryGraphError::NonCanonicalEdgeOrder);
        }

        let mut incoming: BTreeMap<(AncestryCopyId, LocusId), usize> = BTreeMap::new();
        let mut incoming_count: BTreeMap<AncestryCopyId, usize> = BTreeMap::new();
        for edge in &self.edges {
            let source = self
                .nodes
                .get(&edge.source_copy_id)
                .ok_or_else(|| AncestryGraphError::MissingEndpoint(edge.source_copy_id.clone()))?;
            let child = self
                .nodes
                .get(&edge.child_copy_id)
                .ok_or_else(|| AncestryGraphError::MissingEndpoint(edge.child_copy_id.clone()))?;
            if source.chromosome_id != edge.chromosome_id
                || child.chromosome_id != edge.chromosome_id
            {
                return Err(AncestryGraphError::CrossChromosomeEdge);
            }
            let definition = chromosome_map
                .chromosomes
                .get(&edge.chromosome_id)
                .ok_or_else(|| AncestryGraphError::UnknownChromosome(edge.chromosome_id.clone()))?;
            if !definition
                .loci
                .iter()
                .any(|mapped| mapped.locus_id == edge.locus_id)
            {
                return Err(AncestryGraphError::UnknownLocus {
                    chromosome: edge.chromosome_id.clone(),
                    locus: edge.locus_id.clone(),
                });
            }
            if source.generation >= child.generation {
                return Err(AncestryGraphError::GenerationOrderViolation {
                    source: source.copy_id.clone(),
                    child: child.copy_id.clone(),
                });
            }
            let key = (edge.child_copy_id.clone(), edge.locus_id.clone());
            let count = incoming.entry(key).or_default();
            *count += 1;
            if *count > 1 {
                return Err(AncestryGraphError::DuplicateIncomingLocus {
                    child: edge.child_copy_id.clone(),
                    locus: edge.locus_id.clone(),
                });
            }
            *incoming_count.entry(edge.child_copy_id.clone()).or_default() += 1;
        }

        for node in self.nodes.values() {
            let observed = incoming_count.get(&node.copy_id).copied().unwrap_or(0);
            match &node.birth_event {
                None if observed != 0 => {
                    return Err(AncestryGraphError::RootHasParents(node.copy_id.clone()))
                }
                None => {}
                Some(_) => {
                    let expected = chromosome_map
                        .chromosomes
                        .get(&node.chromosome_id)
                        .ok_or_else(|| {
                            AncestryGraphError::UnknownChromosome(node.chromosome_id.clone())
                        })?
                        .loci
                        .len();
                    if observed != expected {
                        return Err(AncestryGraphError::IncompleteDescendantParentage {
                            child: node.copy_id.clone(),
                            expected,
                            observed,
                        });
                    }
                }
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<ModeledAncestryGraphDigest, AncestryGraphError> {
        self.validate_current(schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(GRAPH_DIGEST_DOMAIN);
        put_u32(&mut digest, self.graph_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        put_u64(&mut digest, self.nodes.len() as u64);
        for (copy_id, node) in &self.nodes {
            put_text(&mut digest, copy_id.as_str());
            put_text(&mut digest, node.copy_id.as_str());
            put_text(&mut digest, node.chromosome_id.as_str());
            put_u64(&mut digest, node.generation.get());
            match &node.birth_event {
                None => digest.update([0]),
                Some(event) => {
                    digest.update([1]);
                    put_text(&mut digest, event.as_str());
                }
            }
        }
        put_u64(&mut digest, self.edges.len() as u64);
        for edge in &self.edges {
            put_text(&mut digest, edge.chromosome_id.as_str());
            put_text(&mut digest, edge.locus_id.as_str());
            put_text(&mut digest, edge.source_copy_id.as_str());
            put_text(&mut digest, edge.child_copy_id.as_str());
        }
        Ok(ModeledAncestryGraphDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModeledAncestryGraphDigest([u8; 32]);

impl ModeledAncestryGraphDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for ModeledAncestryGraphDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ModeledAncestryGraphDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

impl fmt::Display for ModeledAncestryGraphDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_hex(&self.0, f)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryGraphAppendProvenance {
    append_version: u32,
    source_graph_digest: ModeledAncestryGraphDigest,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    descendant_provenance_digest: DescendantAncestryDerivationProvenanceDigest,
    child_generation: AncestryGeneration,
    appended_node_ids: Vec<AncestryCopyId>,
    appended_edges: Vec<AncestryGraphEdge>,
    result_graph_digest: ModeledAncestryGraphDigest,
}

impl AncestryGraphAppendProvenance {
    pub fn canonical_digest(&self) -> AncestryGraphAppendProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(APPEND_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.append_version);
        digest.update(self.source_graph_digest.as_bytes());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.descendant_provenance_digest.as_bytes());
        put_u64(&mut digest, self.child_generation.get());
        put_u64(&mut digest, self.appended_node_ids.len() as u64);
        for id in &self.appended_node_ids {
            put_text(&mut digest, id.as_str());
        }
        put_u64(&mut digest, self.appended_edges.len() as u64);
        for edge in &self.appended_edges {
            put_text(&mut digest, edge.chromosome_id.as_str());
            put_text(&mut digest, edge.locus_id.as_str());
            put_text(&mut digest, edge.source_copy_id.as_str());
            put_text(&mut digest, edge.child_copy_id.as_str());
        }
        digest.update(self.result_graph_digest.as_bytes());
        AncestryGraphAppendProvenanceDigest(digest.finalize().into())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        source_graph: &ModeledAncestryGraph,
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
        result_graph: &ModeledAncestryGraph,
    ) -> Result<(), AncestryGraphError> {
        if self.append_version != ANCESTRY_GRAPH_APPEND_VERSION {
            return Err(AncestryGraphError::UnsupportedAppendVersion(
                self.append_version,
            ));
        }
        let recomputed = append_descendant_ancestry_to_graph(
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
        )?;
        if recomputed.graph != *result_graph || recomputed.provenance != *self {
            return Err(AncestryGraphError::AppendReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AncestryGraphAppendProvenanceDigest([u8; 32]);

impl AncestryGraphAppendProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AncestryGraphAppendProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AncestryGraphAppendProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryGraphAppendResult {
    pub graph: ModeledAncestryGraph,
    pub provenance: AncestryGraphAppendProvenance,
}

#[allow(clippy::too_many_arguments)]
pub fn append_descendant_ancestry_to_graph(
    source_graph: &ModeledAncestryGraph,
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
) -> Result<AncestryGraphAppendResult, AncestryGraphError> {
    source_graph.validate_current(schema, chromosome_map)?;
    let event = &descendant.materialization.reproduction_event_id;
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
        &crate::assemble_diploid_linked_offspring_from_evidence(
            schema,
            chromosome_map,
            parent_a_source,
            parent_a_profile,
            parent_a_gamete,
            parent_b_source,
            parent_b_profile,
            parent_b_gamete,
            event,
        )?,
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
            return Err(AncestryGraphError::ChildAlreadyExists(
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
        let source = graph
            .nodes
            .get(&edge.source_copy_id)
            .ok_or_else(|| AncestryGraphError::SourceNodeMissing(edge.source_copy_id.clone()))?;
        if source.generation >= child_generation {
            return Err(AncestryGraphError::GenerationOrderViolation {
                source: source.copy_id.clone(),
                child: edge.child_copy_id.clone(),
            });
        }
        let graph_edge = AncestryGraphEdge {
            chromosome_id: edge.chromosome_id.clone(),
            locus_id: edge.locus_id.clone(),
            source_copy_id: edge.source_copy_id.clone(),
            child_copy_id: edge.child_copy_id.clone(),
        };
        graph.edges.push(graph_edge.clone());
        appended_edges.push(graph_edge);
    }
    graph.edges.sort();
    appended_edges.sort();
    graph.validate_current(schema, chromosome_map)?;
    let result_digest = graph.canonical_digest(schema, chromosome_map)?;
    let provenance = AncestryGraphAppendProvenance {
        append_version: ANCESTRY_GRAPH_APPEND_VERSION,
        source_graph_digest: source_digest,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        descendant_provenance_digest: descendant.provenance.canonical_digest(),
        child_generation,
        appended_node_ids,
        appended_edges,
        result_graph_digest: result_digest,
    };

    Ok(AncestryGraphAppendResult { graph, provenance })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AncestryGraphError {
    Evolution(EvolutionError),
    Descendant(DescendantAncestryError),
    UnsupportedGraphVersion(u32),
    UnsupportedAppendVersion(u32),
    CurrentAuthorityMismatch,
    EmptyGraph,
    RootHasBirthEvent(AncestryCopyId),
    DuplicateNode(AncestryCopyId),
    NodeKeyMismatch {
        key: AncestryCopyId,
        value: AncestryCopyId,
    },
    UnknownChromosome(ChromosomeId),
    UnknownLocus {
        chromosome: ChromosomeId,
        locus: LocusId,
    },
    NonCanonicalEdgeOrder,
    MissingEndpoint(AncestryCopyId),
    CrossChromosomeEdge,
    GenerationOrderViolation {
        source: AncestryCopyId,
        child: AncestryCopyId,
    },
    DuplicateIncomingLocus {
        child: AncestryCopyId,
        locus: LocusId,
    },
    RootHasParents(AncestryCopyId),
    IncompleteDescendantParentage {
        child: AncestryCopyId,
        expected: usize,
        observed: usize,
    },
    ChildAlreadyExists(AncestryCopyId),
    SourceNodeMissing(AncestryCopyId),
    AppendReplayMismatch,
}

impl From<EvolutionError> for AncestryGraphError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl From<DescendantAncestryError> for AncestryGraphError {
    fn from(value: DescendantAncestryError) -> Self {
        Self::Descendant(value)
    }
}

impl fmt::Display for AncestryGraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::Descendant(error) => write!(f, "descendant ancestry authority error: {error}"),
            Self::UnsupportedGraphVersion(version) => {
                write!(f, "unsupported modeled ancestry graph version {version}")
            }
            Self::UnsupportedAppendVersion(version) => {
                write!(f, "unsupported ancestry graph append version {version}")
            }
            Self::CurrentAuthorityMismatch => {
                write!(f, "ancestry graph does not match exact current schema/map authority")
            }
            Self::EmptyGraph => write!(f, "ancestry graph requires at least one root copy"),
            Self::RootHasBirthEvent(id) => write!(
                f,
                "ancestry root {} must not claim a reproduction birth event",
                id.as_str()
            ),
            Self::DuplicateNode(id) => write!(f, "duplicate ancestry graph node {}", id.as_str()),
            Self::NodeKeyMismatch { key, value } => write!(
                f,
                "ancestry graph key {} does not match embedded copy {}",
                key.as_str(),
                value.as_str()
            ),
            Self::UnknownChromosome(chromosome) => {
                write!(f, "unknown ancestry graph chromosome {}", chromosome.as_str())
            }
            Self::UnknownLocus { chromosome, locus } => write!(
                f,
                "locus {} does not belong to ancestry chromosome {}",
                locus.as_str(),
                chromosome.as_str()
            ),
            Self::NonCanonicalEdgeOrder => {
                write!(f, "ancestry graph edges are not in strict canonical order")
            }
            Self::MissingEndpoint(id) => {
                write!(f, "ancestry graph edge references missing copy {}", id.as_str())
            }
            Self::CrossChromosomeEdge => write!(f, "ancestry graph edge crosses chromosomes"),
            Self::GenerationOrderViolation { source, child } => write!(
                f,
                "ancestry source {} is not earlier than child {}",
                source.as_str(),
                child.as_str()
            ),
            Self::DuplicateIncomingLocus { child, locus } => write!(
                f,
                "child {} has more than one incoming ancestry source at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::RootHasParents(id) => {
                write!(f, "root ancestry copy {} unexpectedly has parent edges", id.as_str())
            }
            Self::IncompleteDescendantParentage {
                child,
                expected,
                observed,
            } => write!(
                f,
                "descendant ancestry copy {} expected {} modeled-locus parent edges, observed {}",
                child.as_str(),
                expected,
                observed
            ),
            Self::ChildAlreadyExists(id) => write!(
                f,
                "descendant ancestry copy {} already exists in source graph",
                id.as_str()
            ),
            Self::SourceNodeMissing(id) => write!(
                f,
                "descendant edge source copy {} is absent from source graph",
                id.as_str()
            ),
            Self::AppendReplayMismatch => {
                write!(f, "ancestry graph append deterministic replay mismatch")
            }
        }
    }
}

impl Error for AncestryGraphError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Evolution(error) => Some(error),
            Self::Descendant(error) => Some(error),
            _ => None,
        }
    }
}
