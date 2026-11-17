//! HoloFrac's first executable slice: typed directed spatial-graph retrieval.
//!
//! The authoritative graph remains exact. HDC is an associative candidate index;
//! it is never used as geometry, integrity, authorization, or engineering truth.
//!
//! The continuous view in this benchmark is an f32 materialization of the same
//! deterministic bipolar vector. It measures storage/scoring trade-offs only;
//! it is not a separate continuous encoder family.

use std::collections::HashSet;
use std::time::Instant;

use symtropy_hdc_core::{
    BipolarBundle, BipolarHV, ContinuousHV, EncoderSpec, HdcError, ItemMemory, StableHash64,
};

pub const GRAPH_SCHEMA_VERSION: u32 = 1;
pub const ENCODER_RECIPE: &str = "holofrac-directed-typed-triple-v2:subject+subject-scale+predicate+object+object-scale:majority-bundle";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SpatialScale {
    Site,
    Building,
    Storey,
    Zone,
    Room,
    Assembly,
    Component,
    Material,
}

impl SpatialScale {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Site => "site",
            Self::Building => "building",
            Self::Storey => "storey",
            Self::Zone => "zone",
            Self::Room => "room",
            Self::Assembly => "assembly",
            Self::Component => "component",
            Self::Material => "material",
        }
    }

    const fn hierarchy_rank(self) -> u8 {
        match self {
            Self::Site => 0,
            Self::Building => 1,
            Self::Storey => 2,
            Self::Zone => 3,
            Self::Room => 4,
            Self::Assembly => 5,
            Self::Component => 6,
            Self::Material => 7,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RelationKind {
    Contains,
    AdjacentTo,
    AccessibleFrom,
    Supports,
    InterfacesWith,
    VentilatesTo,
    Supplies,
    DrainsTo,
    RequiresAccessTo,
}

impl RelationKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::AdjacentTo => "adjacent_to",
            Self::AccessibleFrom => "accessible_from",
            Self::Supports => "supports",
            Self::InterfacesWith => "interfaces_with",
            Self::VentilatesTo => "ventilates_to",
            Self::Supplies => "supplies",
            Self::DrainsTo => "drains_to",
            Self::RequiresAccessTo => "requires_access_to",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpatialEntity {
    pub id: String,
    pub scale: SpatialScale,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpatialFact {
    pub subject: String,
    pub relation: RelationKind,
    pub object: String,
}

impl SpatialFact {
    pub fn reversed(&self) -> Self {
        Self {
            subject: self.object.clone(),
            relation: self.relation,
            object: self.subject.clone(),
        }
    }

    pub fn canonical_key(&self) -> String {
        format!(
            "{}\u{1f}{}\u{1f}{}",
            self.subject,
            self.relation.as_str(),
            self.object
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphError(pub &'static str);

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for GraphError {}

#[derive(Debug)]
pub enum HoloFracError {
    Graph(GraphError),
    Hdc(HdcError),
    StaleIndex,
    EncoderMismatch,
}

impl std::fmt::Display for HoloFracError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Graph(error) => write!(f, "spatial graph error: {error}"),
            Self::Hdc(error) => write!(f, "HDC encoding error: {error}"),
            Self::StaleIndex => f.write_str("spatial index source snapshot does not match graph"),
            Self::EncoderMismatch => {
                f.write_str("query encoder fingerprint does not match index encoder")
            }
        }
    }
}

impl std::error::Error for HoloFracError {}

impl From<GraphError> for HoloFracError {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}

impl From<HdcError> for HoloFracError {
    fn from(value: HdcError) -> Self {
        Self::Hdc(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpatialGraph {
    entities: Vec<SpatialEntity>,
    facts: Vec<SpatialFact>,
}

impl SpatialGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_entity(&mut self, entity: SpatialEntity) -> Result<(), GraphError> {
        if entity.id.trim().is_empty() || entity.id.chars().any(char::is_control) {
            return Err(GraphError(
                "entity ID must be non-empty and contain no control characters",
            ));
        }
        if self
            .entities
            .iter()
            .any(|existing| existing.id == entity.id)
        {
            return Err(GraphError("duplicate entity ID"));
        }
        self.entities.push(entity);
        Ok(())
    }

    pub fn add_fact(&mut self, fact: SpatialFact) -> Result<(), GraphError> {
        if fact.subject.trim().is_empty() || fact.object.trim().is_empty() {
            return Err(GraphError("fact endpoints must not be empty"));
        }
        if fact.subject == fact.object {
            return Err(GraphError("self-relations are not allowed in this fixture"));
        }
        if self.facts.contains(&fact) {
            return Err(GraphError("duplicate directed fact"));
        }
        self.facts.push(fact);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), GraphError> {
        let mut ids = HashSet::with_capacity(self.entities.len());
        for entity in &self.entities {
            if entity.id.trim().is_empty()
                || entity.id.chars().any(char::is_control)
                || !ids.insert(entity.id.as_str())
            {
                return Err(GraphError(
                    "empty, control-containing or duplicate entity ID",
                ));
            }
        }

        let mut facts = HashSet::with_capacity(self.facts.len());
        for fact in &self.facts {
            if fact.subject == fact.object {
                return Err(GraphError("self-relations are not allowed in this fixture"));
            }
            if !ids.contains(fact.subject.as_str()) || !ids.contains(fact.object.as_str()) {
                return Err(GraphError("fact references an unknown endpoint"));
            }
            if !facts.insert(fact) {
                return Err(GraphError("duplicate directed fact"));
            }

            if fact.relation == RelationKind::Contains {
                let subject_scale = self.entity_scale(&fact.subject)?;
                let object_scale = self.entity_scale(&fact.object)?;
                if subject_scale.hierarchy_rank() >= object_scale.hierarchy_rank() {
                    return Err(GraphError(
                        "contains must point from a broader scale to a narrower scale",
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn entities(&self) -> &[SpatialEntity] {
        &self.entities
    }

    pub fn facts(&self) -> &[SpatialFact] {
        &self.facts
    }

    pub fn entity_scale(&self, id: &str) -> Result<SpatialScale, GraphError> {
        self.entities
            .iter()
            .find(|entity| entity.id == id)
            .map(|entity| entity.scale)
            .ok_or(GraphError(
                "entity scale lookup references an unknown endpoint",
            ))
    }

    /// Exact typed-graph oracle. This is authoritative for benchmark labels.
    pub fn contains_fact(&self, query: &SpatialFact) -> bool {
        self.facts.contains(query)
    }

    /// Deterministic, sorted canonical representation for an exact metadata index.
    pub fn canonical_facts(&self) -> Vec<String> {
        let mut keys: Vec<_> = self.facts.iter().map(SpatialFact::canonical_key).collect();
        keys.sort();
        keys
    }

    /// Canonical exact source snapshot. Insertion order is not semantically
    /// significant; IDs, scales and directed facts are.
    fn canonical_snapshot(&self) -> Self {
        let mut entities = self.entities.clone();
        entities.sort_by(|left, right| left.id.cmp(&right.id));
        let mut facts = self.facts.clone();
        facts.sort_by_key(SpatialFact::canonical_key);
        Self { entities, facts }
    }

    /// Non-security digest for regression identity only. Use SHA-256/BLAKE3
    /// in a separate evidence pipeline when cryptographic integrity is needed.
    pub fn digest_non_security(&self) -> Result<u64, GraphError> {
        self.validate()?;
        Ok(self.digest_validated())
    }

    /// Hash canonical source content after the caller has validated the graph.
    fn digest_validated(&self) -> u64 {
        let mut hash = StableHash64::new();
        hash.write_u32(GRAPH_SCHEMA_VERSION);

        let mut entities = self.entities.clone();
        entities.sort_by(|a, b| a.id.cmp(&b.id));
        hash.write_u64(entities.len() as u64);
        for entity in entities {
            hash_bytes(&mut hash, entity.id.as_bytes());
            hash_bytes(&mut hash, entity.scale.as_str().as_bytes());
        }

        let mut facts = self.facts.clone();
        facts.sort_by_key(SpatialFact::canonical_key);
        hash.write_u64(facts.len() as u64);
        for fact in facts {
            hash_bytes(&mut hash, fact.subject.as_bytes());
            hash_bytes(&mut hash, fact.relation.as_str().as_bytes());
            hash_bytes(&mut hash, fact.object.as_bytes());
        }
        hash.finish()
    }
}

fn hash_bytes(hash: &mut StableHash64, bytes: &[u8]) {
    hash.write_u64(bytes.len() as u64);
    hash.write(bytes);
}

#[derive(Clone, Debug)]
pub struct EncodedFact {
    pub bipolar: BipolarHV,
    pub continuous_f32_view: ContinuousHV,
}

#[derive(Clone, Debug)]
pub struct HoloFracEncoder {
    memory: ItemMemory,
    spec: EncoderSpec,
    fingerprint: u64,
}

impl HoloFracEncoder {
    pub fn new(spec: EncoderSpec) -> Result<Self, HdcError> {
        let memory = ItemMemory::new(spec.clone())?;
        let mut hash = StableHash64::new();
        hash.write_u64(spec.fingerprint());
        hash.write_u32(GRAPH_SCHEMA_VERSION);
        hash_bytes(&mut hash, ENCODER_RECIPE.as_bytes());
        Ok(Self {
            memory,
            spec,
            fingerprint: hash.finish(),
        })
    }

    pub fn spec(&self) -> &EncoderSpec {
        &self.spec
    }

    /// Encoder-contract fingerprint; non-cryptographic and never an integrity seal.
    pub fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// Encode a typed, directed triple. Entity scale participates in the code.
    ///
    /// Binding is the existing commutative bipolar HDC operation. Subject,
    /// subject-scale, predicate, object and object-scale have distinct roles.
    /// Direction is represented by role assignment. This is not GHRR or
    /// non-commutative matrix binding.
    pub fn encode_fact(
        &self,
        graph: &SpatialGraph,
        fact: &SpatialFact,
    ) -> Result<EncodedFact, HoloFracError> {
        let subject_scale = graph.entity_scale(&fact.subject)?;
        let object_scale = graph.entity_scale(&fact.object)?;

        let subject = self
            .memory
            .bind_role_value("subject", "entity", &fact.subject);
        let subject_scale =
            self.memory
                .bind_role_value("subject_scale", "spatial_scale", subject_scale.as_str());
        let predicate =
            self.memory
                .bind_role_value("predicate", "relation", fact.relation.as_str());
        let object = self
            .memory
            .bind_role_value("object", "entity", &fact.object);
        let object_scale =
            self.memory
                .bind_role_value("object_scale", "spatial_scale", object_scale.as_str());

        let mut bundle = BipolarBundle::new(self.spec.dimension);
        bundle.add(&subject)?;
        bundle.add(&subject_scale)?;
        bundle.add(&predicate)?;
        bundle.add(&object)?;
        bundle.add(&object_scale)?;
        let bipolar = bundle.finish(&self.memory.tie_breaker("spatial-fact-v2"))?;
        let continuous_f32_view = bipolar.to_continuous();

        Ok(EncodedFact {
            bipolar,
            continuous_f32_view,
        })
    }
}

#[derive(Clone, Debug)]
struct IndexedFact {
    fact: SpatialFact,
    encoded: EncodedFact,
}

/// A prebuilt index. Candidate vectors and canonical keys are computed once;
/// query latency must not include repeated index reconstruction.
#[derive(Clone, Debug)]
pub struct SpatialIndex {
    source_graph_digest_non_security: u64,
    source_graph_snapshot: SpatialGraph,
    encoder_spec: EncoderSpec,
    encoder_fingerprint_non_security: u64,
    canonical_keys: Vec<String>,
    facts: Vec<IndexedFact>,
}

impl SpatialIndex {
    pub fn build(
        graph: &SpatialGraph,
        encoder: &HoloFracEncoder,
    ) -> Result<(Self, f64), HoloFracError> {
        let started = Instant::now();
        graph.validate()?;
        let source_graph_digest_non_security = graph.digest_validated();
        let source_graph_snapshot = graph.canonical_snapshot();
        let encoder_spec = encoder.spec().clone();
        let encoder_fingerprint_non_security = encoder.fingerprint();

        let mut facts = Vec::with_capacity(graph.facts.len());
        for fact in &graph.facts {
            facts.push(IndexedFact {
                fact: fact.clone(),
                encoded: encoder.encode_fact(graph, fact)?,
            });
        }

        let canonical_keys = graph.canonical_facts();

        let index = Self {
            source_graph_digest_non_security,
            source_graph_snapshot,
            encoder_spec,
            encoder_fingerprint_non_security,
            canonical_keys,
            facts,
        };
        Ok((index, started.elapsed().as_secs_f64() * 1_000_000.0))
    }

    pub fn len(&self) -> usize {
        self.facts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Exposes the non-security encoder fingerprint for run receipts.
    pub fn encoder_fingerprint_non_security(&self) -> u64 {
        self.encoder_fingerprint_non_security
    }

    pub fn validate_source(&self, graph: &SpatialGraph) -> Result<(), HoloFracError> {
        graph.validate()?;
        if graph.canonical_snapshot() != self.source_graph_snapshot {
            return Err(HoloFracError::StaleIndex);
        }
        Ok(())
    }

    pub fn validate_encoder(&self, encoder: &HoloFracEncoder) -> Result<(), HoloFracError> {
        if encoder.spec() != &self.encoder_spec {
            return Err(HoloFracError::EncoderMismatch);
        }
        Ok(())
    }

    /// Safe public query path. Source and encoder freshness are checked before retrieval.
    pub fn rank_query(
        &self,
        graph: &SpatialGraph,
        encoder: &HoloFracEncoder,
        query: &SpatialFact,
    ) -> Result<QueryOutcome, HoloFracError> {
        self.validate_source(graph)?;
        self.validate_encoder(encoder)?;
        self.rank_query_prevalidated(graph, encoder, query)
    }

    /// Internal hot path used only after validating the source graph once for
    /// the frozen benchmark batch. The digest check is outside query timing.
    fn rank_query_prevalidated(
        &self,
        graph: &SpatialGraph,
        encoder: &HoloFracEncoder,
        query: &SpatialFact,
    ) -> Result<QueryOutcome, HoloFracError> {
        let encode_started = Instant::now();
        let query_encoded = encoder.encode_fact(graph, query)?;
        let query_encode_micros = encode_started.elapsed().as_secs_f64() * 1_000_000.0;

        let bipolar_started = Instant::now();
        let mut bipolar_ranked = Vec::with_capacity(self.facts.len());
        for indexed in &self.facts {
            let score = query_encoded.bipolar.similarity(&indexed.encoded.bipolar)?;
            bipolar_ranked.push((score, indexed.fact.clone()));
        }
        bipolar_ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.canonical_key().cmp(&right.1.canonical_key()))
        });
        let bipolar_search_micros = bipolar_started.elapsed().as_secs_f64() * 1_000_000.0;

        let f32_started = Instant::now();
        let mut f32_ranked = Vec::with_capacity(self.facts.len());
        for indexed in &self.facts {
            let score = query_encoded
                .continuous_f32_view
                .similarity(&indexed.encoded.continuous_f32_view)?;
            f32_ranked.push((score, indexed.fact.clone()));
        }
        f32_ranked.sort_by(|left, right| {
            right
                .0
                .total_cmp(&left.0)
                .then_with(|| left.1.canonical_key().cmp(&right.1.canonical_key()))
        });
        let f32_search_micros = f32_started.elapsed().as_secs_f64() * 1_000_000.0;

        let oracle_started = Instant::now();
        let oracle_match = graph.contains_fact(query);
        let exact_oracle_micros = oracle_started.elapsed().as_secs_f64() * 1_000_000.0;

        let metadata_started = Instant::now();
        let exact_metadata_match = self
            .canonical_keys
            .binary_search(&query.canonical_key())
            .is_ok();
        let metadata_lookup_micros = metadata_started.elapsed().as_secs_f64() * 1_000_000.0;

        let (top1_similarity, hdc_top1) = bipolar_ranked
            .first()
            .map(|(score, fact)| (*score, Some(fact.clone())))
            .unwrap_or((0.0, None));
        let (f32_top1_similarity, f32_top1) = f32_ranked
            .first()
            .map(|(score, fact)| (*score, Some(fact.clone())))
            .unwrap_or((0.0, None));

        // Exact-match threshold is intentionally strict. Partial-context
        // retrieval needs its own calibrated acceptance and abstention study.
        let hdc_exact_accept = hdc_top1.is_some() && top1_similarity >= 1.0 - f32::EPSILON;
        let f32_exact_accept = f32_top1.is_some() && f32_top1_similarity >= 1.0 - f32::EPSILON;

        Ok(QueryOutcome {
            query: query.clone(),
            oracle_match,
            exact_metadata_match,
            hdc_top1,
            top1_similarity,
            hdc_exact_accept,
            f32_top1,
            f32_top1_similarity,
            f32_exact_accept,
            query_encode_micros,
            bipolar_search_micros,
            f32_search_micros,
            exact_oracle_micros,
            metadata_lookup_micros,
        })
    }
}

#[derive(Clone, Debug)]
pub struct QueryOutcome {
    pub query: SpatialFact,
    pub oracle_match: bool,
    pub exact_metadata_match: bool,
    pub hdc_top1: Option<SpatialFact>,
    pub top1_similarity: f32,
    pub hdc_exact_accept: bool,
    pub f32_top1: Option<SpatialFact>,
    pub f32_top1_similarity: f32,
    pub f32_exact_accept: bool,
    pub query_encode_micros: f64,
    pub bipolar_search_micros: f64,
    pub f32_search_micros: f64,
    pub exact_oracle_micros: f64,
    pub metadata_lookup_micros: f64,
}

#[derive(Clone, Debug)]
pub struct BenchmarkReport {
    pub dimension: usize,
    pub graph_digest_non_security: u64,
    pub encoder_fingerprint_non_security: u64,
    pub fact_count: usize,
    pub positive_queries: usize,
    pub positive_top1_hits: usize,
    pub f32_positive_top1_hits: usize,
    pub reversed_queries: usize,
    pub reversed_false_accepts: usize,
    pub f32_reversed_false_accepts: usize,
    pub index_build_micros: f64,
    pub mean_query_encode_micros: f64,
    pub mean_bipolar_search_micros: f64,
    pub mean_f32_search_micros: f64,
    pub mean_bipolar_end_to_end_micros: f64,
    pub mean_f32_end_to_end_micros: f64,
    pub mean_exact_oracle_micros: f64,
    pub mean_metadata_lookup_micros: f64,
    /// Payload estimate for a bipolar-only vector index.
    pub bipolar_index_bytes_estimate: usize,
    /// Payload estimate for an f32-only vector index.
    pub f32_index_bytes_estimate: usize,
    /// Both representations are retained simultaneously by this research index.
    pub dual_view_index_bytes_estimate: usize,
    pub outcomes: Vec<QueryOutcome>,
}

pub fn run_benchmark(
    graph: &SpatialGraph,
    dimension: usize,
    seed: u64,
) -> Result<BenchmarkReport, HoloFracError> {
    if graph.facts.is_empty() {
        return Err(HoloFracError::Graph(GraphError(
            "benchmark graph must contain at least one fact",
        )));
    }

    let spec = EncoderSpec {
        schema_version: 2,
        dimension,
        seed,
        scalar_levels: 257,
    };
    let encoder = HoloFracEncoder::new(spec)?;
    let (index, index_build_micros) = SpatialIndex::build(graph, &encoder)?;
    let graph_digest_non_security = index.source_graph_digest_non_security;

    let mut queries = Vec::with_capacity(graph.facts.len() * 2);
    for fact in &graph.facts {
        queries.push(fact.clone());
        queries.push(fact.reversed());
    }

    // Check the immutable source and encoder snapshots once, outside per-query timers.
    index.validate_source(graph)?;
    index.validate_encoder(&encoder)?;

    // One warm-up query reduces first-call effects in this deliberately small fixture.
    if let Some(first) = queries.first() {
        let _ = index.rank_query_prevalidated(graph, &encoder, first)?;
    }

    let mut outcomes = Vec::with_capacity(queries.len());
    for query in &queries {
        outcomes.push(index.rank_query_prevalidated(graph, &encoder, query)?);
    }

    let positive: Vec<_> = outcomes.iter().filter(|row| row.oracle_match).collect();
    let reversed: Vec<_> = outcomes.iter().filter(|row| !row.oracle_match).collect();
    let positive_top1_hits = positive
        .iter()
        .filter(|row| row.hdc_top1.as_ref() == Some(&row.query))
        .count();
    let f32_positive_top1_hits = positive
        .iter()
        .filter(|row| row.f32_top1.as_ref() == Some(&row.query))
        .count();
    let reversed_false_accepts = reversed.iter().filter(|row| row.hdc_exact_accept).count();
    let f32_reversed_false_accepts = reversed.iter().filter(|row| row.f32_exact_accept).count();

    let query_count = outcomes.len().max(1) as f64;
    let mean_of =
        |select: fn(&QueryOutcome) -> f64| outcomes.iter().map(select).sum::<f64>() / query_count;
    let mean_query_encode_micros = mean_of(|row| row.query_encode_micros);
    let mean_bipolar_search_micros = mean_of(|row| row.bipolar_search_micros);
    let mean_f32_search_micros = mean_of(|row| row.f32_search_micros);
    let mean_exact_oracle_micros = mean_of(|row| row.exact_oracle_micros);
    let mean_metadata_lookup_micros = mean_of(|row| row.metadata_lookup_micros);

    Ok(BenchmarkReport {
        dimension,
        graph_digest_non_security,
        encoder_fingerprint_non_security: index.encoder_fingerprint_non_security(),
        fact_count: graph.facts.len(),
        positive_queries: positive.len(),
        positive_top1_hits,
        f32_positive_top1_hits,
        reversed_queries: reversed.len(),
        reversed_false_accepts,
        f32_reversed_false_accepts,
        index_build_micros,
        mean_query_encode_micros,
        mean_bipolar_search_micros,
        mean_f32_search_micros,
        mean_bipolar_end_to_end_micros: mean_query_encode_micros + mean_bipolar_search_micros,
        mean_f32_end_to_end_micros: mean_query_encode_micros + mean_f32_search_micros,
        mean_exact_oracle_micros,
        mean_metadata_lookup_micros,
        bipolar_index_bytes_estimate: graph.facts.len() * dimension * std::mem::size_of::<i8>(),
        f32_index_bytes_estimate: graph.facts.len() * dimension * std::mem::size_of::<f32>(),
        dual_view_index_bytes_estimate: graph.facts.len()
            * dimension
            * (std::mem::size_of::<i8>() + std::mem::size_of::<f32>()),
        outcomes,
    })
}

/// Small deterministic fixture that crosses architectural scales and services.
pub fn reference_architecture_graph() -> Result<SpatialGraph, GraphError> {
    let mut graph = SpatialGraph::new();
    for (id, scale) in [
        ("site", SpatialScale::Site),
        ("house", SpatialScale::Building),
        ("ground", SpatialScale::Storey),
        ("living-zone", SpatialScale::Zone),
        ("kitchen", SpatialScale::Room),
        ("living", SpatialScale::Room),
        ("bathroom", SpatialScale::Room),
        ("corridor", SpatialScale::Room),
        ("water-main", SpatialScale::Component),
        ("extract-vent", SpatialScale::Component),
    ] {
        graph.add_entity(SpatialEntity {
            id: id.to_owned(),
            scale,
        })?;
    }

    for (subject, relation, object) in [
        ("site", RelationKind::Contains, "house"),
        ("house", RelationKind::Contains, "ground"),
        ("ground", RelationKind::Contains, "living-zone"),
        ("living-zone", RelationKind::Contains, "kitchen"),
        ("living-zone", RelationKind::Contains, "living"),
        ("living-zone", RelationKind::Contains, "bathroom"),
        ("living", RelationKind::AccessibleFrom, "corridor"),
        ("kitchen", RelationKind::AccessibleFrom, "corridor"),
        ("water-main", RelationKind::Supplies, "kitchen"),
        ("water-main", RelationKind::Supplies, "bathroom"),
        ("bathroom", RelationKind::VentilatesTo, "extract-vent"),
    ] {
        graph.add_fact(SpatialFact {
            subject: subject.to_owned(),
            relation,
            object: object.to_owned(),
        })?;
    }

    graph.validate()?;
    Ok(graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(dimension: usize, seed: u64) -> EncoderSpec {
        EncoderSpec {
            schema_version: 2,
            dimension,
            seed,
            scalar_levels: 257,
        }
    }

    #[test]
    fn reference_graph_is_valid_and_deterministic() {
        let a = reference_architecture_graph().unwrap();
        let b = reference_architecture_graph().unwrap();
        assert_eq!(
            a.digest_non_security().unwrap(),
            b.digest_non_security().unwrap()
        );
        assert_eq!(a.canonical_facts(), b.canonical_facts());
    }

    #[test]
    fn graph_rejects_duplicate_ids_and_unknown_endpoints() {
        let mut graph = SpatialGraph::new();
        graph
            .add_entity(SpatialEntity {
                id: "room-a".into(),
                scale: SpatialScale::Room,
            })
            .unwrap();
        assert!(
            graph
                .add_entity(SpatialEntity {
                    id: "room-a".into(),
                    scale: SpatialScale::Room,
                })
                .is_err()
        );
        graph
            .add_fact(SpatialFact {
                subject: "room-a".into(),
                relation: RelationKind::AccessibleFrom,
                object: "missing".into(),
            })
            .unwrap();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn control_characters_in_entity_ids_are_rejected() {
        let mut graph = SpatialGraph::new();
        assert!(
            graph
                .add_entity(SpatialEntity {
                    id: format!("room{}ambiguous", char::from(31_u8)),
                    scale: SpatialScale::Room,
                })
                .is_err()
        );
    }

    #[test]
    fn duplicate_facts_and_self_relations_are_rejected() {
        let mut graph = SpatialGraph::new();
        let fact = SpatialFact {
            subject: "a".into(),
            relation: RelationKind::Supports,
            object: "b".into(),
        };
        graph.add_fact(fact.clone()).unwrap();
        assert!(graph.add_fact(fact).is_err());
        assert!(
            graph
                .add_fact(SpatialFact {
                    subject: "a".into(),
                    relation: RelationKind::Supports,
                    object: "a".into(),
                })
                .is_err()
        );
    }

    #[test]
    fn contains_must_point_from_broader_to_narrower_scale() {
        let mut graph = SpatialGraph::new();
        graph
            .add_entity(SpatialEntity {
                id: "room".into(),
                scale: SpatialScale::Room,
            })
            .unwrap();
        graph
            .add_entity(SpatialEntity {
                id: "site".into(),
                scale: SpatialScale::Site,
            })
            .unwrap();
        graph
            .add_fact(SpatialFact {
                subject: "room".into(),
                relation: RelationKind::Contains,
                object: "site".into(),
            })
            .unwrap();
        assert!(graph.validate().is_err());
    }

    #[test]
    fn role_encoding_preserves_direction_as_a_role_assignment() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = HoloFracEncoder::new(spec(4_096, 7)).unwrap();
        let forward = SpatialFact {
            subject: "water-main".into(),
            relation: RelationKind::Supplies,
            object: "kitchen".into(),
        };
        let reverse = forward.reversed();
        let encoded_forward = encoder.encode_fact(&graph, &forward).unwrap();
        let encoded_reverse = encoder.encode_fact(&graph, &reverse).unwrap();
        assert_ne!(encoded_forward.bipolar, encoded_reverse.bipolar);
        assert!(
            encoded_forward
                .bipolar
                .similarity(&encoded_reverse.bipolar)
                .unwrap()
                < 1.0
        );
    }

    #[test]
    fn entity_scales_participate_in_encoding() {
        let make_graph = |second_scale| {
            let mut graph = SpatialGraph::new();
            graph
                .add_entity(SpatialEntity {
                    id: "a".into(),
                    scale: SpatialScale::Room,
                })
                .unwrap();
            graph
                .add_entity(SpatialEntity {
                    id: "b".into(),
                    scale: second_scale,
                })
                .unwrap();
            graph
                .add_fact(SpatialFact {
                    subject: "a".into(),
                    relation: RelationKind::AdjacentTo,
                    object: "b".into(),
                })
                .unwrap();
            graph
        };
        let a = make_graph(SpatialScale::Room);
        let b = make_graph(SpatialScale::Zone);
        let fact = a.facts()[0].clone();
        let encoder = HoloFracEncoder::new(spec(4_096, 13)).unwrap();
        assert_ne!(
            encoder.encode_fact(&a, &fact).unwrap().bipolar,
            encoder.encode_fact(&b, &fact).unwrap().bipolar
        );
    }

    #[test]
    fn encoding_is_deterministic_and_fingerprint_is_configuration_bound() {
        let graph = reference_architecture_graph().unwrap();
        let fact = graph.facts()[0].clone();
        let a = HoloFracEncoder::new(spec(4_096, 9)).unwrap();
        let b = HoloFracEncoder::new(spec(4_096, 9)).unwrap();
        let changed_seed = HoloFracEncoder::new(spec(4_096, 10)).unwrap();
        let changed_dimension = HoloFracEncoder::new(spec(8_192, 9)).unwrap();
        let mut changed_schema_spec = spec(4_096, 9);
        changed_schema_spec.schema_version += 1;
        let changed_schema = HoloFracEncoder::new(changed_schema_spec).unwrap();

        assert_eq!(
            a.encode_fact(&graph, &fact).unwrap().bipolar,
            b.encode_fact(&graph, &fact).unwrap().bipolar
        );
        assert_ne!(a.fingerprint(), changed_seed.fingerprint());
        assert_ne!(a.fingerprint(), changed_dimension.fingerprint());
        assert_ne!(a.fingerprint(), changed_schema.fingerprint());
    }

    #[test]
    fn exact_graph_oracle_does_not_confuse_reversed_edges() {
        let graph = reference_architecture_graph().unwrap();
        let forward = SpatialFact {
            subject: "water-main".into(),
            relation: RelationKind::Supplies,
            object: "kitchen".into(),
        };
        assert!(graph.contains_fact(&forward));
        assert!(!graph.contains_fact(&forward.reversed()));
    }

    #[test]
    fn index_is_built_once_and_stale_source_state_is_rejected() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = HoloFracEncoder::new(spec(2_048, 3)).unwrap();
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        assert_eq!(index.len(), graph.facts().len());

        let mut changed = graph.clone();
        changed.entities[8].scale = SpatialScale::Assembly;
        assert!(matches!(
            index.rank_query(&changed, &encoder, &graph.facts()[0]),
            Err(HoloFracError::StaleIndex)
        ));
    }

    #[test]
    fn index_rejects_mismatched_encoder_configuration() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = HoloFracEncoder::new(spec(2_048, 3)).unwrap();
        let wrong_encoder = HoloFracEncoder::new(spec(2_048, 4)).unwrap();
        let (index, _) = SpatialIndex::build(&graph, &encoder).unwrap();
        assert!(matches!(
            index.rank_query(&graph, &wrong_encoder, &graph.facts()[0]),
            Err(HoloFracError::EncoderMismatch)
        ));
    }

    #[test]
    fn graph_digest_and_snapshot_ignore_insertion_order() {
        let original = reference_architecture_graph().unwrap();
        let mut reordered = SpatialGraph::new();
        for entity in original.entities().iter().rev() {
            reordered.add_entity(entity.clone()).unwrap();
        }
        for fact in original.facts().iter().rev() {
            reordered.add_fact(fact.clone()).unwrap();
        }
        assert_eq!(
            original.digest_non_security().unwrap(),
            reordered.digest_non_security().unwrap()
        );
        assert_eq!(
            original.canonical_snapshot(),
            reordered.canonical_snapshot()
        );
        let encoder = HoloFracEncoder::new(spec(2_048, 17)).unwrap();
        let (index, _) = SpatialIndex::build(&original, &encoder).unwrap();
        assert!(index.validate_source(&reordered).is_ok());
    }

    #[test]
    fn hdc_retrieval_separates_reference_facts_from_reversed_controls() {
        let graph = reference_architecture_graph().unwrap();
        let report = run_benchmark(&graph, 8_192, 11).unwrap();
        assert_eq!(report.positive_top1_hits, report.positive_queries);
        assert_eq!(report.f32_positive_top1_hits, report.positive_queries);
        assert_eq!(report.reversed_false_accepts, 0);
        assert_eq!(report.f32_reversed_false_accepts, 0);
    }

    #[test]
    fn continuous_view_is_explicitly_the_same_code_with_larger_storage() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = HoloFracEncoder::new(spec(2_048, 5)).unwrap();
        let fact = graph.facts()[0].clone();
        let encoded = encoder.encode_fact(&graph, &fact).unwrap();
        let score_bipolar = encoded.bipolar.similarity(&encoded.bipolar).unwrap();
        let score_continuous = encoded
            .continuous_f32_view
            .similarity(&encoded.continuous_f32_view)
            .unwrap();
        assert!((score_bipolar - score_continuous).abs() < 1.0e-6);
        assert_eq!(encoded.continuous_f32_view.values.len(), 2_048);
    }

    #[test]
    fn invalid_zero_dimension_fails_closed() {
        assert!(HoloFracEncoder::new(spec(0, 1)).is_err());
    }

    #[test]
    fn source_graph_digest_is_not_the_encoder_fingerprint() {
        let graph = reference_architecture_graph().unwrap();
        let encoder = HoloFracEncoder::new(spec(2_048, 5)).unwrap();
        assert_ne!(graph.digest_non_security().unwrap(), encoder.fingerprint());
    }
}
