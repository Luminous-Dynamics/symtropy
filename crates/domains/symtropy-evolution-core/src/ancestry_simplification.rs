use crate::{
    ancestry_pruning::AncestryReachabilityPruningProvenanceDigest,
    canonical::{fmt_hex, put_text, put_u32, put_u64},
    prune_ancestry_reachability, AncestryCopyId, AncestryGraphError, AncestryGraphNode,
    AncestryPruningError, AncestryRetentionSet, ChromosomeId, ChromosomeMap,
    ChromosomeMapDigest, EvolutionError, HereditarySchema, HereditarySchemaDigest, LocusId,
    ModeledAncestryGraph, ModeledAncestryGraphDigest,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};

pub const SIMPLIFIED_ANCESTRY_GRAPH_VERSION: u32 = 1;
pub const ANCESTRY_SIMPLIFICATION_DERIVATION_VERSION: u32 = 1;

const SIMPLIFIED_GRAPH_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:simplified-ancestry-graph:v1\0";
const SIMPLIFICATION_PROVENANCE_DIGEST_DOMAIN: &[u8] =
    b"symtropy:evolution:ancestry-simplification-provenance:v1\0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AncestrySimplificationProfile {
    FocalProtectedRootsAndBranchesV1,
}

impl AncestrySimplificationProfile {
    fn tag(self) -> u8 {
        match self {
            Self::FocalProtectedRootsAndBranchesV1 => 0,
        }
    }
}

/// A compressed ancestry-path edge at one exact modeled locus.
///
/// This is deliberately not an `AncestryGraphEdge`. It does not claim direct
/// reproduction. It means only that `source_copy_id` is the nearest retained
/// ancestor of `child_copy_id` at this modeled locus under the declared
/// simplification profile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SimplifiedAncestryEdge {
    pub chromosome_id: ChromosomeId,
    pub locus_id: LocusId,
    pub source_copy_id: AncestryCopyId,
    pub child_copy_id: AncestryCopyId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SimplifiedAncestryGraph {
    pub graph_version: u32,
    pub schema_digest: HereditarySchemaDigest,
    pub chromosome_map_digest: ChromosomeMapDigest,
    pub profile: AncestrySimplificationProfile,
    pub retention: AncestryRetentionSet,
    pub nodes: BTreeMap<AncestryCopyId, AncestryGraphNode>,
    pub edges: Vec<SimplifiedAncestryEdge>,
}

impl SimplifiedAncestryGraph {
    pub fn validate_current(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<(), AncestrySimplificationError> {
        schema.validate()?;
        chromosome_map.validate(schema)?;
        if self.graph_version != SIMPLIFIED_ANCESTRY_GRAPH_VERSION {
            return Err(AncestrySimplificationError::UnsupportedGraphVersion(
                self.graph_version,
            ));
        }
        if self.schema_digest != schema.canonical_digest()?
            || self.chromosome_map_digest != chromosome_map.canonical_digest(schema)?
        {
            return Err(AncestrySimplificationError::CurrentAuthorityMismatch);
        }
        validate_retention_local(&self.retention)?;
        if self.nodes.is_empty() {
            return Err(AncestrySimplificationError::EmptyGraph);
        }

        for selected in self
            .retention
            .focal_copy_ids
            .iter()
            .chain(self.retention.protected_copy_ids.iter())
        {
            if !self.nodes.contains_key(selected) {
                return Err(AncestrySimplificationError::MissingRetainedCopy(
                    selected.clone(),
                ));
            }
        }

        for (key, node) in &self.nodes {
            if key != &node.copy_id {
                return Err(AncestrySimplificationError::NodeKeyMismatch);
            }
            if !chromosome_map.chromosomes.contains_key(&node.chromosome_id) {
                return Err(AncestrySimplificationError::UnknownChromosome(
                    node.chromosome_id.clone(),
                ));
            }
        }
        if self.edges.windows(2).any(|window| window[0] >= window[1]) {
            return Err(AncestrySimplificationError::NonCanonicalEdgeOrder);
        }

        let mut incoming = BTreeSet::new();
        let mut outgoing = BTreeSet::new();
        for edge in &self.edges {
            let source = self.nodes.get(&edge.source_copy_id).ok_or_else(|| {
                AncestrySimplificationError::MissingEndpoint(edge.source_copy_id.clone())
            })?;
            let child = self.nodes.get(&edge.child_copy_id).ok_or_else(|| {
                AncestrySimplificationError::MissingEndpoint(edge.child_copy_id.clone())
            })?;
            if source.chromosome_id != edge.chromosome_id
                || child.chromosome_id != edge.chromosome_id
            {
                return Err(AncestrySimplificationError::CrossChromosomeEdge);
            }
            let definition = chromosome_map
                .chromosomes
                .get(&edge.chromosome_id)
                .ok_or_else(|| {
                    AncestrySimplificationError::UnknownChromosome(edge.chromosome_id.clone())
                })?;
            if !definition
                .loci
                .iter()
                .any(|mapped| mapped.locus_id == edge.locus_id)
            {
                return Err(AncestrySimplificationError::UnknownLocus {
                    chromosome: edge.chromosome_id.clone(),
                    locus: edge.locus_id.clone(),
                });
            }
            if source.generation >= child.generation {
                return Err(AncestrySimplificationError::GenerationOrderViolation);
            }
            if !incoming.insert((edge.child_copy_id.clone(), edge.locus_id.clone())) {
                return Err(AncestrySimplificationError::DuplicateIncomingLocus {
                    child: edge.child_copy_id.clone(),
                    locus: edge.locus_id.clone(),
                });
            }
            outgoing.insert((edge.source_copy_id.clone(), edge.locus_id.clone()));
        }

        // Any non-root copy that participates as an ancestor at a locus must
        // remain connected upward at that same locus. A node may still be
        // globally retained for another locus without participating here.
        for (copy_id, locus_id) in &outgoing {
            let node = &self.nodes[copy_id];
            if node.birth_event.is_some()
                && !incoming.contains(&(copy_id.clone(), locus_id.clone()))
            {
                return Err(
                    AncestrySimplificationError::IncompleteParticipatingParentage {
                        child: copy_id.clone(),
                        locus: locus_id.clone(),
                    },
                );
            }
        }

        // Every selected descendant must preserve ancestry at every modeled
        // locus on its chromosome.
        for selected in self
            .retention
            .focal_copy_ids
            .iter()
            .chain(self.retention.protected_copy_ids.iter())
        {
            let node = &self.nodes[selected];
            if node.birth_event.is_none() {
                continue;
            }
            let definition = &chromosome_map.chromosomes[&node.chromosome_id];
            for mapped in &definition.loci {
                if !incoming.contains(&(selected.clone(), mapped.locus_id.clone())) {
                    return Err(AncestrySimplificationError::IncompleteSelectedParentage {
                        child: selected.clone(),
                        locus: mapped.locus_id.clone(),
                    });
                }
            }
        }

        for root in self.nodes.values().filter(|node| node.birth_event.is_none()) {
            if self
                .edges
                .iter()
                .any(|edge| edge.child_copy_id == root.copy_id)
            {
                return Err(AncestrySimplificationError::RootHasParents(
                    root.copy_id.clone(),
                ));
            }
        }
        Ok(())
    }

    pub fn canonical_digest(
        &self,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
    ) -> Result<SimplifiedAncestryGraphDigest, AncestrySimplificationError> {
        self.validate_current(schema, chromosome_map)?;
        let mut digest = Sha256::new();
        digest.update(SIMPLIFIED_GRAPH_DIGEST_DOMAIN);
        put_u32(&mut digest, self.graph_version);
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update([self.profile.tag()]);
        put_retention(&mut digest, &self.retention);
        put_u64(&mut digest, self.nodes.len() as u64);
        for (id, node) in &self.nodes {
            put_text(&mut digest, id.as_str());
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
        Ok(SimplifiedAncestryGraphDigest(digest.finalize().into()))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SimplifiedAncestryGraphDigest([u8; 32]);

impl SimplifiedAncestryGraphDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for SimplifiedAncestryGraphDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SimplifiedAncestryGraphDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestrySimplificationProvenance {
    derivation_version: u32,
    source_graph_digest: ModeledAncestryGraphDigest,
    schema_digest: HereditarySchemaDigest,
    chromosome_map_digest: ChromosomeMapDigest,
    pruning_provenance_digest: AncestryReachabilityPruningProvenanceDigest,
    profile: AncestrySimplificationProfile,
    retained_node_ids: Vec<AncestryCopyId>,
    result_graph_digest: SimplifiedAncestryGraphDigest,
}

impl AncestrySimplificationProvenance {
    pub fn retained_node_ids(&self) -> &[AncestryCopyId] {
        &self.retained_node_ids
    }

    pub fn canonical_digest(&self) -> AncestrySimplificationProvenanceDigest {
        let mut digest = Sha256::new();
        digest.update(SIMPLIFICATION_PROVENANCE_DIGEST_DOMAIN);
        put_u32(&mut digest, self.derivation_version);
        digest.update(self.source_graph_digest.as_bytes());
        digest.update(self.schema_digest.as_bytes());
        digest.update(self.chromosome_map_digest.as_bytes());
        digest.update(self.pruning_provenance_digest.as_bytes());
        digest.update([self.profile.tag()]);
        put_u64(&mut digest, self.retained_node_ids.len() as u64);
        for id in &self.retained_node_ids {
            put_text(&mut digest, id.as_str());
        }
        digest.update(self.result_graph_digest.as_bytes());
        AncestrySimplificationProvenanceDigest(digest.finalize().into())
    }

    pub fn validate_current(
        &self,
        source_graph: &ModeledAncestryGraph,
        retention: &AncestryRetentionSet,
        profile: AncestrySimplificationProfile,
        schema: &HereditarySchema,
        chromosome_map: &ChromosomeMap,
        result_graph: &SimplifiedAncestryGraph,
    ) -> Result<(), AncestrySimplificationError> {
        if self.derivation_version != ANCESTRY_SIMPLIFICATION_DERIVATION_VERSION {
            return Err(AncestrySimplificationError::UnsupportedDerivationVersion(
                self.derivation_version,
            ));
        }
        let recomputed = simplify_modeled_ancestry(
            source_graph,
            retention,
            profile,
            schema,
            chromosome_map,
        )?;
        if recomputed.graph != *result_graph || recomputed.provenance != *self {
            return Err(AncestrySimplificationError::ReplayMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AncestrySimplificationProvenanceDigest([u8; 32]);

impl AncestrySimplificationProvenanceDigest {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for AncestrySimplificationProvenanceDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AncestrySimplificationProvenanceDigest(")?;
        fmt_hex(&self.0, f)?;
        write!(f, ")")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AncestrySimplificationResult {
    pub graph: SimplifiedAncestryGraph,
    pub provenance: AncestrySimplificationProvenance,
}

pub fn simplify_modeled_ancestry(
    source_graph: &ModeledAncestryGraph,
    retention: &AncestryRetentionSet,
    profile: AncestrySimplificationProfile,
    schema: &HereditarySchema,
    chromosome_map: &ChromosomeMap,
) -> Result<AncestrySimplificationResult, AncestrySimplificationError> {
    source_graph.validate_current(schema, chromosome_map)?;
    retention.validate_current(source_graph, schema, chromosome_map)?;
    let pruned = prune_ancestry_reachability(source_graph, retention, schema, chromosome_map)?;
    let pruned_graph = &pruned.graph;

    let selected: BTreeSet<_> = retention
        .focal_copy_ids
        .iter()
        .chain(retention.protected_copy_ids.iter())
        .cloned()
        .collect();
    let mut globally_retained = BTreeSet::new();
    let mut simplified_edges = BTreeSet::new();

    for (chromosome_id, definition) in &chromosome_map.chromosomes {
        for mapped_locus in &definition.loci {
            let locus_id = &mapped_locus.locus_id;
            let mut incoming: BTreeMap<AncestryCopyId, AncestryCopyId> = BTreeMap::new();
            for edge in &pruned_graph.edges {
                if edge.chromosome_id == *chromosome_id && edge.locus_id == *locus_id {
                    incoming.insert(edge.child_copy_id.clone(), edge.source_copy_id.clone());
                }
            }

            // D2A is whole-copy conservative. D2B1 traces only the ancestry
            // actually reachable from selected copies at this exact locus.
            let mut work: Vec<_> = selected
                .iter()
                .filter(|id| {
                    pruned_graph
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
                if let Some(source) = incoming.get(&child) {
                    active_edges.push((source.clone(), child.clone()));
                    work.push(source.clone());
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
                let node = &pruned_graph.nodes[id];
                if selected.contains(id)
                    || node.birth_event.is_none()
                    || outdegree.get(id).copied().unwrap_or(0) != 1
                {
                    retained_locus.insert(id.clone());
                }
            }
            globally_retained.extend(retained_locus.iter().cloned());

            for child in &retained_locus {
                let node = &pruned_graph.nodes[child];
                if node.birth_event.is_none() {
                    continue;
                }
                let mut cursor = child.clone();
                let source = loop {
                    let parent = incoming.get(&cursor).ok_or_else(|| {
                        AncestrySimplificationError::MissingActiveParent {
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

    // Selection authority itself is persistent even if a future chromosome map
    // contains a chromosome with no modeled loci.
    globally_retained.extend(selected.iter().cloned());
    let nodes = globally_retained
        .iter()
        .map(|id| {
            pruned_graph
                .nodes
                .get(id)
                .cloned()
                .map(|node| (id.clone(), node))
                .ok_or_else(|| AncestrySimplificationError::MissingRetainedCopy(id.clone()))
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let edges: Vec<_> = simplified_edges.into_iter().collect();
    let graph = SimplifiedAncestryGraph {
        graph_version: SIMPLIFIED_ANCESTRY_GRAPH_VERSION,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        profile,
        retention: retention.clone(),
        nodes,
        edges,
    };
    graph.validate_current(schema, chromosome_map)?;

    let result_graph_digest = graph.canonical_digest(schema, chromosome_map)?;
    let provenance = AncestrySimplificationProvenance {
        derivation_version: ANCESTRY_SIMPLIFICATION_DERIVATION_VERSION,
        source_graph_digest: source_graph.canonical_digest(schema, chromosome_map)?,
        schema_digest: schema.canonical_digest()?,
        chromosome_map_digest: chromosome_map.canonical_digest(schema)?,
        pruning_provenance_digest: pruned.provenance.canonical_digest(),
        profile,
        retained_node_ids: globally_retained.into_iter().collect(),
        result_graph_digest,
    };
    Ok(AncestrySimplificationResult { graph, provenance })
}

fn validate_retention_local(
    retention: &AncestryRetentionSet,
) -> Result<(), AncestrySimplificationError> {
    if retention.retention_version != crate::ANCESTRY_RETENTION_SET_VERSION {
        return Err(AncestrySimplificationError::InvalidRetentionAuthority);
    }
    if retention.focal_copy_ids.is_empty()
        || retention.focal_copy_ids.windows(2).any(|w| w[0] >= w[1])
        || retention.protected_copy_ids.windows(2).any(|w| w[0] >= w[1])
        || retention
            .focal_copy_ids
            .iter()
            .any(|id| retention.protected_copy_ids.binary_search(id).is_ok())
    {
        return Err(AncestrySimplificationError::InvalidRetentionAuthority);
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AncestrySimplificationError {
    Graph(AncestryGraphError),
    Pruning(AncestryPruningError),
    Evolution(EvolutionError),
    UnsupportedGraphVersion(u32),
    UnsupportedDerivationVersion(u32),
    CurrentAuthorityMismatch,
    EmptyGraph,
    InvalidRetentionAuthority,
    MissingRetainedCopy(AncestryCopyId),
    NodeKeyMismatch,
    UnknownChromosome(ChromosomeId),
    UnknownLocus {
        chromosome: ChromosomeId,
        locus: LocusId,
    },
    NonCanonicalEdgeOrder,
    MissingEndpoint(AncestryCopyId),
    CrossChromosomeEdge,
    GenerationOrderViolation,
    DuplicateIncomingLocus {
        child: AncestryCopyId,
        locus: LocusId,
    },
    IncompleteParticipatingParentage {
        child: AncestryCopyId,
        locus: LocusId,
    },
    IncompleteSelectedParentage {
        child: AncestryCopyId,
        locus: LocusId,
    },
    RootHasParents(AncestryCopyId),
    MissingActiveParent {
        child: AncestryCopyId,
        locus: LocusId,
    },
    ReplayMismatch,
}

impl From<AncestryGraphError> for AncestrySimplificationError {
    fn from(value: AncestryGraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<AncestryPruningError> for AncestrySimplificationError {
    fn from(value: AncestryPruningError) -> Self {
        Self::Pruning(value)
    }
}

impl From<EvolutionError> for AncestrySimplificationError {
    fn from(value: EvolutionError) -> Self {
        Self::Evolution(value)
    }
}

impl fmt::Display for AncestrySimplificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "ancestry graph authority error: {error}"),
            Self::Pruning(error) => write!(f, "ancestry pruning authority error: {error}"),
            Self::Evolution(error) => write!(f, "evolution authority error: {error}"),
            Self::UnsupportedGraphVersion(version) => {
                write!(f, "unsupported simplified ancestry graph version {version}")
            }
            Self::UnsupportedDerivationVersion(version) => write!(
                f,
                "unsupported ancestry simplification derivation version {version}"
            ),
            Self::CurrentAuthorityMismatch => write!(
                f,
                "simplified ancestry graph does not match current schema/map authority"
            ),
            Self::EmptyGraph => write!(f, "simplified ancestry graph is empty"),
            Self::InvalidRetentionAuthority => write!(
                f,
                "simplified ancestry graph carries invalid focal/protected authority"
            ),
            Self::MissingRetainedCopy(id) => {
                write!(f, "retained ancestry copy {} is absent", id.as_str())
            }
            Self::NodeKeyMismatch => write!(f, "simplified ancestry graph node key mismatch"),
            Self::UnknownChromosome(id) => write!(
                f,
                "unknown simplified ancestry chromosome {}",
                id.as_str()
            ),
            Self::UnknownLocus { chromosome, locus } => write!(
                f,
                "locus {} does not belong to simplified ancestry chromosome {}",
                locus.as_str(),
                chromosome.as_str()
            ),
            Self::NonCanonicalEdgeOrder => {
                write!(f, "simplified ancestry edges are not in strict canonical order")
            }
            Self::MissingEndpoint(id) => write!(
                f,
                "simplified ancestry edge references missing copy {}",
                id.as_str()
            ),
            Self::CrossChromosomeEdge => {
                write!(f, "simplified ancestry edge crosses chromosomes")
            }
            Self::GenerationOrderViolation => {
                write!(f, "simplified ancestry edge violates generation order")
            }
            Self::DuplicateIncomingLocus { child, locus } => write!(
                f,
                "simplified child {} has duplicate incoming ancestry at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::IncompleteParticipatingParentage { child, locus } => write!(
                f,
                "participating simplified ancestor {} is disconnected from its own ancestry at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::IncompleteSelectedParentage { child, locus } => write!(
                f,
                "selected copy {} lacks simplified ancestry at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::RootHasParents(id) => write!(
                f,
                "simplified root {} unexpectedly has incoming ancestry",
                id.as_str()
            ),
            Self::MissingActiveParent { child, locus } => write!(
                f,
                "cannot trace retained ancestry parent for {} at locus {}",
                child.as_str(),
                locus.as_str()
            ),
            Self::ReplayMismatch => {
                write!(f, "ancestry simplification deterministic replay mismatch")
            }
        }
    }
}

impl Error for AncestrySimplificationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Graph(error) => Some(error),
            Self::Pruning(error) => Some(error),
            Self::Evolution(error) => Some(error),
            _ => None,
        }
    }
}
