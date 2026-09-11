use crate::{
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    AncestryCopyId, AncestryGraphEdge, AncestryGraphError, ChromosomeMap,
    ChromosomeMapDigest, HereditarySchema, HereditarySchemaDigest, ModeledAncestryGraph,
    ModeledAncestryGraphDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const ANCESTRY_RETENTION_SET_VERSION: u32 = 1;
pub const ANCESTRY_REACHABILITY_PRUNING_VERSION: u32 = 1;

const RETENTION_SET_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:ancestry-retention-set:v1\0";
const PRUNING_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:ancestry-reachability-pruning:v1\0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryRetentionSet {
    pub retention_version: u32,
    pub focal_copy_ids: Vec<AncestryCopyId>,
    pub protected_copy_ids: Vec<AncestryCopyId>,
}

impl AncestryRetentionSet {
    pub fn new(
        mut focal_copy_ids: Vec<AncestryCopyId>,
        mut protected_copy_ids: Vec<AncestryCopyId>,
    ) -> Result<Self, AncestryPruningError> {
        if focal_copy_ids.is_empty() {
            return Err(AncestryPruningError::EmptyFocalSet);
        }
        focal_copy_ids.sort();
        protected_copy_ids.sort();
        if focal_copy_ids.windows(2).any(|w| w[0] == w[1])
            || protected_copy_ids.windows(2).any(|w| w[0] == w[1])
        {
            return Err(AncestryPruningError::DuplicateRetentionIdentity);
        }
        if focal_copy_ids
            .iter()
            .any(|id| protected_copy_ids.binary_search(id).is_ok())
        {
            return Err(AncestryPruningError::FocalProtectedOverlap);
        }
        Ok(Self {
            retention_version: ANCESTRY_RETENTION_SET_VERSION,
            focal_copy_ids,
            protected_copy_ids,
        })
    }

    pub fn validate_current(
        &self,
        graph: &ModeledAncestryGraph,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<(), AncestryPruningError> {
        graph.validate_current(schema, chromosome_map)?;
        if self.retention_version != ANCESTRY_RETENTION_SET_VERSION {
            return Err(AncestryPruningError::UnsupportedRetentionVersion(
                self.retention_version,
            ));
        }
        if self.focal_copy_ids.is_empty() {
            return Err(AncestryPruningError::EmptyFocalSet);
        }
        if self.focal_copy_ids.windows(2).any(|w| w[0] >= w[1])
            || self.protected_copy_ids.windows(2).any(|w| w[0] >= w[1])
        {
            return Err(AncestryPruningError::NonCanonicalRetentionOrder);
        }
        if self
            .focal_copy_ids
            .iter()
            .any(|id| self.protected_copy_ids.binary_search(id).is_ok())
        {
            return Err(AncestryPruningError::FocalProtectedOverlap);
        }
        for id in self
            .focal_copy_ids
            .iter()
            .chain(self.protected_copy_ids.iter())
        {
            if !graph.nodes.contains_key(id) {
                return Err(AncestryPruningError::MissingRetainedCopy(id.clone()));
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        graph: &ModeledAncestryGraph,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<AncestryRetentionSetDigest, AncestryPruningError> {
        self.validate_current(graph, schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(RETENTION_SET_DIGEST_DOMAIN);
        put_u32(&mut digest, self.retention_version);
        put_u64(&mut digest, self.focal_copy_ids.len() as u64);
        for id in &self.focal_copy_ids {
            put_text(&mut digest, id.as_str());
        }
        put_u64(&mut digest, self.protected_copy_ids.len() as u64);
        for id in &self.protected_copy_ids {
            put_text(&mut digest, id.as_str());
        }
        Ok(AncestryRetentionSetDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AncestryRetentionSetDigest([u8; 32]);

impl AncestryRetentionSetDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AncestryRetentionSetDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AncestryRetentionSetDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryReachabilityPruningProvenance {
    pruning_version: u32,
    source_graph_digest: ModeledAncestryGraphDigest,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    retention_set_digest: AncestryRetentionSetDigest,
    retained_node_ids: Vec<AncestryCopyId>,
    result_graph_digest: ModeledAncestryGraphDigest,
}

impl AncestryReachabilityPruningProvenance {
    pub fn retained_node_ids(&self) -> &[AncestryCopyId] {
        &self.retained_node_ids
    }

    pub fn canonical_digest(&self) -> AncestryReachabilityPruningProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(PRUNING_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.pruning_version);
        digest.update(self.source_graph_digest.as_bytes());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.retention_set_digest.as_bytes());
        put_u64(&mut digest, self.retained_node_ids.len() as u64);
        for id in &self.retained_node_ids {
            put_text(&mut digest, id.as_str());
        }
        digest.update(self.result_graph_digest.as_bytes());
        AncestryReachabilityPruningProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        source_graph: &ModeledAncestryGraph,
        retention: &AncestryRetentionSet,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        result_graph: &ModeledAncestryGraph,
    ) -> Result<(), AncestryPruningError> {
        if self.pruning_version != ANCESTRY_REACHABILITY_PRUNING_VERSION {
            return Err(AncestryPruningError::UnsupportedPruningVersion(
                self.pruning_version,
            ));
        }
        let recomputed = prune_ancestry_reachability(
            source_graph,
            retention,
            schema,
            chromosome_map,
        )?;
        if recomputed.graph != *result_graph || recomputed.provenance != *self {
            return Err(AncestryPruningError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AncestryReachabilityPruningProvenanceDigest([u8; 32]);

impl AncestryReachabilityPruningProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AncestryReachabilityPruningProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AncestryReachabilityPruningProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestryReachabilityPruningResult {
    pub graph: ModeledAncestryGraph,
    pub provenance: AncestryReachabilityPruningProvenance,
}

pub fn prune_ancestry_reachability(
    source_graph: &ModeledAncestryGraph,
    retention: &AncestryRetentionSet,
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
) -> Result<AncestryReachabilityPruningResult, AncestryPruningError> {
    source_graph.validate_current(schema, chromosome_map)?;
    retention.validate_current(source_graph, schema, chromosome_map)?;
    let source_digest = source_graph.canonical_digest(schema, chromosome_map)?;
    let retention_digest = retention.canonical_digest(source_graph, schema, chromosome_map)?;

    let mut incoming_by_child: BTreeMap<AncestryCopyId, Vec<AncestryGraphEdge>> =
        BTreeMap::new();
    for edge in &source_graph.edges {
        incoming_by_child
            .entry(edge.child_copy_id.clone())
            .or_default()
            .push(edge.clone());
    }

    let mut work: Vec<AncestryCopyId> = retention
        .focal_copy_ids
        .iter()
        .chain(retention.protected_copy_ids.iter())
        .cloned()
        .collect();
    let mut retained_nodes = BTreeSet::new();
    let mut retained_edges = BTreeSet::new();

    while let Some(copy_id) = work.pop() {
        if !retained_nodes.insert(copy_id.clone()) {
            continue;
        }
        let node = source_graph
            .nodes
            .get(&copy_id)
            .ok_or_else(|| AncestryPruningError::MissingRetainedCopy(copy_id.clone()))?;
        if node.birth_event.is_none() {
            continue;
        }
        let incoming = incoming_by_child
            .get(&copy_id)
            .ok_or_else(|| AncestryPruningError::IncompleteSourceGraph(copy_id.clone()))?;
        for edge in incoming {
            retained_edges.insert(edge.clone());
            work.push(edge.source_copy_id.clone());
        }
    }

    let nodes = retained_nodes
        .iter()
        .map(|id| {
            source_graph
                .nodes
                .get(id)
                .cloned()
                .map(|node| (id.clone(), node))
                .ok_or_else(|| AncestryPruningError::MissingRetainedCopy(id.clone()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let edges: Vec<_> = retained_edges.into_iter().collect();
    let graph = ModeledAncestryGraph {
        graph_version: source_graph.graph_version,
        schema_digest: source_graph.schema_digest,
        chromosome_map_digest: source_graph.chromosome_map_digest,
        nodes,
        edges,
    };
    graph.validate_current(schema, chromosome_map)?;
    let result_digest = graph.canonical_digest(schema, chromosome_map)?;
    let retained_node_ids: Vec<_> = retained_nodes.into_iter().collect();
    let provenance = AncestryReachabilityPruningProvenance {
        pruning_version: ANCESTRY_REACHABILITY_PRUNING_VERSION,
        source_graph_digest: source_digest,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        retention_set_digest: retention_digest,
        retained_node_ids,
        result_graph_digest: result_digest,
    };
    Ok(AncestryReachabilityPruningResult { graph, provenance })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AncestryPruningError {
    Graph(AncestryGraphError),
    UnsupportedRetentionVersion(u32),
    UnsupportedPruningVersion(u32),
    EmptyFocalSet,
    DuplicateRetentionIdentity,
    NonCanonicalRetentionOrder,
    FocalProtectedOverlap,
    MissingRetainedCopy(AncestryCopyId),
    IncompleteSourceGraph(AncestryCopyId),
    ReplayMismatch,
}

impl From<AncestryGraphError> for AncestryPruningError {
    fn from(value: AncestryGraphError) -> Self {
        Self::Graph(value)
    }
}

impl fmt::Display for AncestryPruningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "ancestry graph authority error: {error}"),
            Self::UnsupportedRetentionVersion(version) => {
                write!(f, "unsupported ancestry retention-set version {version}")
            }
            Self::UnsupportedPruningVersion(version) => {
                write!(f, "unsupported ancestry reachability-pruning version {version}")
            }
            Self::EmptyFocalSet => write!(f, "ancestry pruning requires at least one focal copy"),
            Self::DuplicateRetentionIdentity => {
                write!(f, "ancestry retention set contains a duplicate copy identity")
            }
            Self::NonCanonicalRetentionOrder => {
                write!(f, "restored ancestry retention set is not in strict canonical order")
            }
            Self::FocalProtectedOverlap => {
                write!(f, "one ancestry copy cannot be both focal and protected in V1")
            }
            Self::MissingRetainedCopy(id) => write!(
                f,
                "retained ancestry copy {} is absent from the exact source graph",
                id.as_str()
            ),
            Self::IncompleteSourceGraph(id) => write!(
                f,
                "non-root ancestry copy {} has no complete incoming parentage in source graph",
                id.as_str()
            ),
            Self::ReplayMismatch => {
                write!(f, "ancestry reachability-pruning deterministic replay mismatch")
            }
        }
    }
}

impl Error for AncestryPruningError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Graph(error) => Some(error),
            _ => None,
        }
    }
}
