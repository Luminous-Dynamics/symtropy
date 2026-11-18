// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Version-scoped taxonomic identity and overlapping living-system graphs.
//!
//! Taxon identifiers are scoped by authority, dataset/checklist and release.
//! Living systems can contain multiple taxa and overlapping relationships.
//! This module performs no external taxonomy resolution.

use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::fmt;

/// Identity key for one taxon concept in one dataset release.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TaxonConceptKey {
    pub authority: String,
    pub dataset_id: String,
    pub release: String,
    pub concept_id: String,
}

/// External taxonomic concept plus mutable name/rank annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonConceptRef {
    pub authority: String,
    pub dataset_id: String,
    pub release: String,
    pub concept_id: String,
    pub scientific_name: String,
    /// Provider-defined rank; not a closed enum.
    pub rank: String,
}

impl TaxonConceptRef {
    pub fn key(&self) -> TaxonConceptKey {
        TaxonConceptKey {
            authority: self.authority.clone(),
            dataset_id: self.dataset_id.clone(),
            release: self.release.clone(),
            concept_id: self.concept_id.clone(),
        }
    }

    pub fn has_same_concept_key(&self, other: &Self) -> bool {
        self.key() == other.key()
    }

    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.authority, "taxon.authority")?;
        require_non_empty(&self.dataset_id, "taxon.dataset_id")?;
        require_non_empty(&self.release, "taxon.release")?;
        require_non_empty(&self.concept_id, "taxon.concept_id")?;
        require_non_empty(&self.scientific_name, "taxon.scientific_name")?;
        require_non_empty(&self.rank, "taxon.rank")?;
        Ok(())
    }
}

/// Source-scoped life-stage term, distinct from the coarse simulator LifeStage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeStageConceptRef {
    pub vocabulary_authority: String,
    pub vocabulary_release: String,
    pub concept_id: String,
    pub label: String,
}

impl LifeStageConceptRef {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.vocabulary_authority, "life_stage.vocabulary_authority")?;
        require_non_empty(&self.vocabulary_release, "life_stage.vocabulary_release")?;
        require_non_empty(&self.concept_id, "life_stage.concept_id")?;
        require_non_empty(&self.label, "life_stage.label")?;
        Ok(())
    }
}

/// Versioned mapping-method identity; the source resolver owns its semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonMappingMethodRef {
    pub authority: String,
    pub release: String,
    pub method_id: String,
    pub label: String,
}

impl TaxonMappingMethodRef {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.authority, "mapping_method.authority")?;
        require_non_empty(&self.release, "mapping_method.release")?;
        require_non_empty(&self.method_id, "mapping_method.method_id")?;
        require_non_empty(&self.label, "mapping_method.label")?;
        Ok(())
    }
}

/// Directional semantic relation between two independently identified taxon concepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonMappingRelation {
    ExactMatch,
    Congruent,
    Includes,
    IncludedIn,
    Overlaps,
    RelatedTo,
    NoMatch,
    Unresolved,
}

impl TaxonMappingRelation {
    /// Return the correct relation when source and target are swapped.
    pub const fn inverse(self) -> Self {
        match self {
            Self::ExactMatch => Self::ExactMatch,
            Self::Congruent => Self::Congruent,
            Self::Includes => Self::IncludedIn,
            Self::IncludedIn => Self::Includes,
            Self::Overlaps => Self::Overlaps,
            Self::RelatedTo => Self::RelatedTo,
            Self::NoMatch => Self::NoMatch,
            Self::Unresolved => Self::Unresolved,
        }
    }

    const fn can_be_accepted_as_mapping(self) -> bool {
        !matches!(self, Self::NoMatch | Self::Unresolved)
    }
}

/// Review lifecycle. Acceptance does not change either source concept's identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaxonMappingStatus {
    Proposed,
    Reviewed,
    Accepted,
    Rejected,
    Deprecated,
}

/// Evidence-bearing assertion between two source-scoped taxon concepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxonConceptMapping {
    pub mapping_id: String,
    pub source: TaxonConceptKey,
    pub target: TaxonConceptKey,
    pub relation: TaxonMappingRelation,
    pub status: TaxonMappingStatus,
    pub method: TaxonMappingMethodRef,
    /// Opaque IDs owned by external provenance/evidence services.
    pub evidence_refs: Vec<String>,
}

impl TaxonConceptMapping {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.mapping_id, "mapping.mapping_id")?;
        validate_taxon_key(&self.source, true)?;
        validate_taxon_key(&self.target, false)?;
        if self.source == self.target {
            return Err(LivingSystemIdentityError::SelfTaxonMapping);
        }
        self.method.validate()?;

        let mut evidence_ids = HashSet::new();
        for evidence_ref in &self.evidence_refs {
            require_non_empty(evidence_ref, "mapping.evidence_ref")?;
            if !evidence_ids.insert(evidence_ref.as_str()) {
                return Err(LivingSystemIdentityError::DuplicateEvidenceReference(
                    evidence_ref.clone(),
                ));
            }
        }
        if self.status != TaxonMappingStatus::Proposed && self.evidence_refs.is_empty() {
            return Err(LivingSystemIdentityError::MissingMappingEvidence);
        }
        if self.status == TaxonMappingStatus::Accepted
            && !self.relation.can_be_accepted_as_mapping()
        {
            return Err(LivingSystemIdentityError::InvalidAcceptedMappingRelation);
        }
        Ok(())
    }
}

/// Validate a set of crosswalk assertions. Multiple mappings from one source
/// are allowed for release splits, overlaps or competing provider assertions.
pub fn validate_taxon_mappings(
    mappings: &[TaxonConceptMapping],
) -> Result<(), LivingSystemIdentityError> {
    let mut mapping_ids = HashSet::new();
    for mapping in mappings {
        mapping.validate()?;
        if !mapping_ids.insert(mapping.mapping_id.as_str()) {
            return Err(LivingSystemIdentityError::DuplicateTaxonMappingId(
                mapping.mapping_id.clone(),
            ));
        }
    }
    Ok(())
}

fn validate_taxon_key(
    key: &TaxonConceptKey,
    source: bool,
) -> Result<(), LivingSystemIdentityError> {
    let prefix = if source { "mapping.source" } else { "mapping.target" };
    require_non_empty(
        &key.authority,
        if source { "mapping.source.authority" } else { "mapping.target.authority" },
    )?;
    require_non_empty(
        &key.dataset_id,
        if source { "mapping.source.dataset_id" } else { "mapping.target.dataset_id" },
    )?;
    require_non_empty(
        &key.release,
        if source { "mapping.source.release" } else { "mapping.target.release" },
    )?;
    require_non_empty(
        &key.concept_id,
        if source { "mapping.source.concept_id" } else { "mapping.target.concept_id" },
    )?;
    let _ = prefix;
    Ok(())
}

/// Coarse organizational scale, not a taxonomic rank or kingdom.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LivingSystemLevel {
    Unspecified,
    Individual,
    Population,
    Community,
    Consortium,
    Holobiont,
    Colony,
    Biofilm,
    Ecosystem,
}

/// One biological member represented in a living-system node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivingSystemMember {
    pub member_id: String,
    pub taxon: TaxonConceptRef,
    pub life_stage: Option<LifeStageConceptRef>,
    pub local_variant_id: Option<String>,
}

impl LivingSystemMember {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.member_id, "member.member_id")?;
        self.taxon.validate()?;
        if let Some(stage) = &self.life_stage {
            stage.validate()?;
        }
        if let Some(variant) = &self.local_variant_id {
            require_non_empty(variant, "member.local_variant_id")?;
        }
        Ok(())
    }
}

/// A living system may contain one or many taxon concepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivingSystemNode {
    pub node_id: String,
    pub level: LivingSystemLevel,
    pub members: Vec<LivingSystemMember>,
}

impl LivingSystemNode {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.node_id, "node.node_id")?;
        if self.members.is_empty() {
            return Err(LivingSystemIdentityError::NodeHasNoMembers);
        }
        let mut member_ids = HashSet::new();
        for member in &self.members {
            member.validate()?;
            if !member_ids.insert(member.member_id.as_str()) {
                return Err(LivingSystemIdentityError::DuplicateMemberId(
                    member.member_id.clone(),
                ));
            }
        }
        Ok(())
    }
}

/// External relationship vocabulary term for domain-specific interactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationshipConceptRef {
    pub vocabulary_authority: String,
    pub vocabulary_release: String,
    pub concept_id: String,
    pub label: String,
}

impl RelationshipConceptRef {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.vocabulary_authority, "relation.vocabulary_authority")?;
        require_non_empty(&self.vocabulary_release, "relation.vocabulary_release")?;
        require_non_empty(&self.concept_id, "relation.concept_id")?;
        require_non_empty(&self.label, "relation.label")?;
        Ok(())
    }
}

/// Directed relation between system nodes. Contains is the only acyclic
/// relation; ecological and exchange relations may overlap and form cycles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivingSystemRelation {
    Contains,
    HostAssociatedWith,
    SymbioticWith,
    TrophicInteractionWith,
    InteractsWith,
    ExchangesMatterWith,
    ExchangesEnergyWith,
    ExchangesChemicalSignalWith,
    Custom(RelationshipConceptRef),
}

impl LivingSystemRelation {
    fn is_containment(&self) -> bool {
        matches!(self, Self::Contains)
    }

    fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        if let Self::Custom(reference) = self {
            reference.validate()?;
        }
        Ok(())
    }
}

/// Directed multigraph edge between two living-system nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LivingSystemEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub relation: LivingSystemRelation,
}

impl LivingSystemEdge {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        require_non_empty(&self.edge_id, "edge.edge_id")?;
        require_non_empty(&self.from_node_id, "edge.from_node_id")?;
        require_non_empty(&self.to_node_id, "edge.to_node_id")?;
        self.relation.validate()
    }
}

/// Nested living-system graph. Multiple parents and overlapping relationships
/// are allowed; only the Contains subgraph must be acyclic.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LivingSystemGraph {
    pub nodes: Vec<LivingSystemNode>,
    pub edges: Vec<LivingSystemEdge>,
}

impl LivingSystemGraph {
    pub fn validate(&self) -> Result<(), LivingSystemIdentityError> {
        if self.nodes.is_empty() {
            return Err(LivingSystemIdentityError::EmptyGraph);
        }

        let mut node_indices = HashMap::with_capacity(self.nodes.len());
        for (index, node) in self.nodes.iter().enumerate() {
            node.validate()?;
            if node_indices.insert(node.node_id.as_str(), index).is_some() {
                return Err(LivingSystemIdentityError::DuplicateNodeId(
                    node.node_id.clone(),
                ));
            }
        }

        let mut edge_ids = HashSet::new();
        let mut adjacency = vec![Vec::new(); self.nodes.len()];
        let mut indegree = vec![0_usize; self.nodes.len()];

        for edge in &self.edges {
            edge.validate()?;
            if !edge_ids.insert(edge.edge_id.as_str()) {
                return Err(LivingSystemIdentityError::DuplicateEdgeId(
                    edge.edge_id.clone(),
                ));
            }
            let from = *node_indices.get(edge.from_node_id.as_str()).ok_or_else(|| {
                LivingSystemIdentityError::UnknownNodeReference(edge.from_node_id.clone())
            })?;
            let to = *node_indices.get(edge.to_node_id.as_str()).ok_or_else(|| {
                LivingSystemIdentityError::UnknownNodeReference(edge.to_node_id.clone())
            })?;
            if edge.relation.is_containment() {
                adjacency[from].push(to);
                indegree[to] += 1;
            }
        }

        // Kahn's algorithm avoids recursion for large containment graphs.
        let mut ready = VecDeque::new();
        for (index, degree) in indegree.iter().enumerate() {
            if *degree == 0 {
                ready.push_back(index);
            }
        }
        let mut visited = 0_usize;
        while let Some(index) = ready.pop_front() {
            visited += 1;
            for &child in &adjacency[index] {
                indegree[child] -= 1;
                if indegree[child] == 0 {
                    ready.push_back(child);
                }
            }
        }
        if visited != self.nodes.len() {
            return Err(LivingSystemIdentityError::ContainmentCycle);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LivingSystemIdentityError {
    EmptyField(&'static str),
    EmptyGraph,
    NodeHasNoMembers,
    DuplicateNodeId(String),
    DuplicateMemberId(String),
    DuplicateEdgeId(String),
    UnknownNodeReference(String),
    ContainmentCycle,
    SelfTaxonMapping,
    MissingMappingEvidence,
    InvalidAcceptedMappingRelation,
    DuplicateEvidenceReference(String),
    DuplicateTaxonMappingId(String),
}

impl fmt::Display for LivingSystemIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "{field} must be non-empty"),
            Self::EmptyGraph => write!(formatter, "living-system graph must contain a node"),
            Self::NodeHasNoMembers => {
                write!(formatter, "living-system node must contain at least one member")
            }
            Self::DuplicateNodeId(id) => write!(formatter, "duplicate node ID: {id}"),
            Self::DuplicateMemberId(id) => write!(formatter, "duplicate member ID: {id}"),
            Self::DuplicateEdgeId(id) => write!(formatter, "duplicate edge ID: {id}"),
            Self::UnknownNodeReference(id) => {
                write!(formatter, "edge references unknown node: {id}")
            }
            Self::ContainmentCycle => {
                write!(formatter, "containment relationships must be acyclic")
            }
            Self::SelfTaxonMapping => {
                write!(formatter, "a taxon concept cannot be crosswalked to itself")
            }
            Self::MissingMappingEvidence => {
                write!(formatter, "reviewed taxon mappings require evidence references")
            }
            Self::InvalidAcceptedMappingRelation => {
                write!(formatter, "NoMatch and Unresolved cannot be accepted mappings")
            }
            Self::DuplicateEvidenceReference(id) => {
                write!(formatter, "duplicate mapping evidence reference: {id}")
            }
            Self::DuplicateTaxonMappingId(id) => {
                write!(formatter, "duplicate taxon mapping ID: {id}")
            }
        }
    }
}

impl Error for LivingSystemIdentityError {}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), LivingSystemIdentityError> {
    if value.trim().is_empty() {
        return Err(LivingSystemIdentityError::EmptyField(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn taxon(
        authority: &str,
        dataset: &str,
        release: &str,
        id: &str,
        name: &str,
    ) -> TaxonConceptRef {
        TaxonConceptRef {
            authority: authority.to_owned(),
            dataset_id: dataset.to_owned(),
            release: release.to_owned(),
            concept_id: id.to_owned(),
            scientific_name: name.to_owned(),
            rank: "species".to_owned(),
        }
    }

    fn member(id: &str, taxon: TaxonConceptRef) -> LivingSystemMember {
        LivingSystemMember {
            member_id: id.to_owned(),
            taxon,
            life_stage: None,
            local_variant_id: None,
        }
    }

    fn node(
        id: &str,
        level: LivingSystemLevel,
        members: Vec<LivingSystemMember>,
    ) -> LivingSystemNode {
        LivingSystemNode { node_id: id.to_owned(), level, members }
    }

    fn edge(id: &str, from: &str, to: &str, relation: LivingSystemRelation) -> LivingSystemEdge {
        LivingSystemEdge {
            edge_id: id.to_owned(),
            from_node_id: from.to_owned(),
            to_node_id: to.to_owned(),
            relation,
        }
    }

    fn bacterium() -> TaxonConceptRef {
        taxon(
            "Catalogue of Life",
            "7ddf754f-d193-4cc9-b351-99906754a03b",
            "COL26.10-XR",
            "col:EXAMPLE-BACTERIUM",
            "Bacterium example",
        )
    }

    #[test]
    fn taxon_identity_is_scoped_by_authority_dataset_release_and_concept() {
        let col = bacterium();
        let gbif = taxon(
            "GBIF Backbone",
            "gbif-backbone",
            "2023-08-28",
            "12345",
            "Bacterium example",
        );
        let older_col = taxon(
            "Catalogue of Life",
            "7ddf754f-d193-4cc9-b351-99906754a03b",
            "test-earlier-snapshot",
            "col:EXAMPLE-BACTERIUM",
            "Bacterium example",
        );
        assert_ne!(col.key(), gbif.key());
        assert_ne!(col.key(), older_col.key());
        assert!(col.has_same_concept_key(&col.clone()));
    }

    #[test]
    fn preferred_name_changes_do_not_change_source_scoped_concept_key() {
        let original = bacterium();
        let renamed = taxon(
            "Catalogue of Life",
            "7ddf754f-d193-4cc9-b351-99906754a03b",
            "COL26.10-XR",
            "col:EXAMPLE-BACTERIUM",
            "Bacterium example revised",
        );
        assert!(original.has_same_concept_key(&renamed));
        assert_ne!(original, renamed);
    }

    #[test]
    fn life_stage_vocabularies_are_independently_versioned() {
        let plant = LifeStageConceptRef {
            vocabulary_authority: "Plant Ontology".to_owned(),
            vocabulary_release: "test-plant-stage-release".to_owned(),
            concept_id: "PO:TEST-SEEDLING".to_owned(),
            label: "seedling".to_owned(),
        };
        let fungus = LifeStageConceptRef {
            vocabulary_authority: "Fungal Growth Ontology".to_owned(),
            vocabulary_release: "1.0".to_owned(),
            concept_id: "FGO:TEST-MYCELIUM".to_owned(),
            label: "conidiating mycelium".to_owned(),
        };
        assert!(plant.validate().is_ok());
        assert!(fungus.validate().is_ok());
        assert_ne!(plant.vocabulary_authority, fungus.vocabulary_authority);
    }

    #[test]
    fn one_holobiont_node_can_contain_multiple_taxa() {
        let holobiont = node(
            "holobiont-1",
            LivingSystemLevel::Holobiont,
            vec![
                member(
                    "host",
                    taxon(
                        "NCBI",
                        "taxonomy",
                        "test-ncbi-snapshot",
                        "NCBI:TEST-9606",
                        "Homo sapiens",
                    ),
                ),
                member("microbe", bacterium()),
                member(
                    "fungus",
                    taxon(
                        "Catalogue of Life",
                        "7ddf754f-d193-4cc9-b351-99906754a03b",
                        "COL26.10-XR",
                        "col:EXAMPLE-FUNGUS",
                        "Fungus example",
                    ),
                ),
            ],
        );
        assert!(holobiont.validate().is_ok());
        assert_eq!(holobiont.members.len(), 3);
    }

    #[test]
    fn containment_chain_is_accepted() {
        let graph = LivingSystemGraph {
            nodes: vec![
                node("ecosystem", LivingSystemLevel::Ecosystem, vec![member("e", bacterium())]),
                node("community", LivingSystemLevel::Community, vec![member("c", bacterium())]),
                node("population", LivingSystemLevel::Population, vec![member("p", bacterium())]),
            ],
            edges: vec![
                edge("ec", "ecosystem", "community", LivingSystemRelation::Contains),
                edge("cp", "community", "population", LivingSystemRelation::Contains),
            ],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn containment_cycle_is_rejected() {
        let graph = LivingSystemGraph {
            nodes: vec![
                node("a", LivingSystemLevel::Community, vec![member("a-member", bacterium())]),
                node("b", LivingSystemLevel::Community, vec![member("b-member", bacterium())]),
            ],
            edges: vec![
                edge("ab", "a", "b", LivingSystemRelation::Contains),
                edge("ba", "b", "a", LivingSystemRelation::Contains),
            ],
        };
        assert_eq!(graph.validate(), Err(LivingSystemIdentityError::ContainmentCycle));
    }

    #[test]
    fn overlapping_symbiosis_and_trophic_links_may_form_cycles() {
        let graph = LivingSystemGraph {
            nodes: vec![
                node("host", LivingSystemLevel::Individual, vec![member("host", bacterium())]),
                node(
                    "symbiont",
                    LivingSystemLevel::Individual,
                    vec![member("symbiont", bacterium())],
                ),
            ],
            edges: vec![
                edge("sym-a", "host", "symbiont", LivingSystemRelation::SymbioticWith),
                edge("sym-b", "symbiont", "host", LivingSystemRelation::SymbioticWith),
                edge("exchange", "host", "symbiont", LivingSystemRelation::ExchangesMatterWith),
                edge("trophic", "symbiont", "host", LivingSystemRelation::TrophicInteractionWith),
            ],
        };
        assert!(graph.validate().is_ok());
    }

    #[test]
    fn missing_taxon_release_or_concept_id_fails_closed() {
        let mut missing_release = bacterium();
        missing_release.release.clear();
        assert_eq!(
            missing_release.validate(),
            Err(LivingSystemIdentityError::EmptyField("taxon.release"))
        );
        let mut missing_id = bacterium();
        missing_id.concept_id.clear();
        assert_eq!(
            missing_id.validate(),
            Err(LivingSystemIdentityError::EmptyField("taxon.concept_id"))
        );
    }

    #[test]
    fn duplicate_local_ids_and_unknown_edges_fail_closed() {
        let duplicate_nodes = LivingSystemGraph {
            nodes: vec![
                node("same", LivingSystemLevel::Individual, vec![member("one", bacterium())]),
                node("same", LivingSystemLevel::Individual, vec![member("two", bacterium())]),
            ],
            edges: vec![],
        };
        assert_eq!(
            duplicate_nodes.validate(),
            Err(LivingSystemIdentityError::DuplicateNodeId("same".to_owned()))
        );
        let unknown_edge = LivingSystemGraph {
            nodes: vec![
                node("known", LivingSystemLevel::Individual, vec![member("one", bacterium())]),
            ],
            edges: vec![
                edge("unknown", "known", "missing", LivingSystemRelation::InteractsWith),
            ],
        };
        assert_eq!(
            unknown_edge.validate(),
            Err(LivingSystemIdentityError::UnknownNodeReference("missing".to_owned()))
        );
    }

    #[test]
    fn empty_graph_duplicate_member_ids_and_duplicate_edge_ids_fail_closed() {
        assert_eq!(
            LivingSystemGraph::default().validate(),
            Err(LivingSystemIdentityError::EmptyGraph)
        );

        let duplicate_members = LivingSystemGraph {
            nodes: vec![node(
                "community",
                LivingSystemLevel::Community,
                vec![
                    member("same-member", bacterium()),
                    member("same-member", bacterium()),
                ],
            )],
            edges: vec![],
        };
        assert_eq!(
            duplicate_members.validate(),
            Err(LivingSystemIdentityError::DuplicateMemberId(
                "same-member".to_owned()
            ))
        );

        let duplicate_edges = LivingSystemGraph {
            nodes: vec![
                node(
                    "a",
                    LivingSystemLevel::Individual,
                    vec![member("a", bacterium())],
                ),
                node(
                    "b",
                    LivingSystemLevel::Individual,
                    vec![member("b", bacterium())],
                ),
            ],
            edges: vec![
                edge("same-edge", "a", "b", LivingSystemRelation::InteractsWith),
                edge("same-edge", "b", "a", LivingSystemRelation::InteractsWith),
            ],
        };
        assert_eq!(
            duplicate_edges.validate(),
            Err(LivingSystemIdentityError::DuplicateEdgeId(
                "same-edge".to_owned()
            ))
        );
    }

    #[test]
    fn custom_relationships_require_a_versioned_vocabulary_reference() {
        let custom = RelationshipConceptRef {
            vocabulary_authority: "Example Interaction Ontology".to_owned(),
            vocabulary_release: "1.0".to_owned(),
            concept_id: "EIO:EXAMPLE-INTERACTION".to_owned(),
            label: "resource-mediated association".to_owned(),
        };
        let graph = LivingSystemGraph {
            nodes: vec![
                node(
                    "a",
                    LivingSystemLevel::Individual,
                    vec![member("a", bacterium())],
                ),
                node(
                    "b",
                    LivingSystemLevel::Individual,
                    vec![member("b", bacterium())],
                ),
            ],
            edges: vec![edge(
                "custom-link",
                "a",
                "b",
                LivingSystemRelation::Custom(custom),
            )],
        };
        assert!(graph.validate().is_ok());

        let invalid = RelationshipConceptRef {
            vocabulary_authority: "Example Interaction Ontology".to_owned(),
            vocabulary_release: String::new(),
            concept_id: "EIO:EXAMPLE-INTERACTION".to_owned(),
            label: "resource-mediated association".to_owned(),
        };
        assert_eq!(
            LivingSystemRelation::Custom(invalid).validate(),
            Err(LivingSystemIdentityError::EmptyField(
                "relation.vocabulary_release"
            ))
        );
    }

    #[test]
    fn invalid_life_stage_vocabulary_fails_closed() {
        let invalid = LifeStageConceptRef {
            vocabulary_authority: "Plant Ontology".to_owned(),
            vocabulary_release: String::new(),
            concept_id: "PO:TEST-SEEDLING".to_owned(),
            label: "seedling".to_owned(),
        };
        assert_eq!(
            invalid.validate(),
            Err(LivingSystemIdentityError::EmptyField(
                "life_stage.vocabulary_release"
            ))
        );
    }

    fn mapping(source: TaxonConceptRef, target: TaxonConceptRef) -> TaxonConceptMapping {
        TaxonConceptMapping {
            mapping_id: "mapping-1".to_owned(),
            source: source.key(),
            target: target.key(),
            relation: TaxonMappingRelation::ExactMatch,
            status: TaxonMappingStatus::Proposed,
            method: TaxonMappingMethodRef {
                authority: "Example Resolver".to_owned(),
                release: "1.0".to_owned(),
                method_id: "exact-curated-crosswalk".to_owned(),
                label: "curated exact concept comparison".to_owned(),
            },
            evidence_refs: Vec::new(),
        }
    }

    #[test]
    fn exact_name_identity_and_crosswalk_relation_are_not_implicit() {
        let source = taxon("Provider A", "dataset-a", "2026-01", "A:1", "Example organism");
        let target = taxon("Provider B", "dataset-b", "2026-01", "B:1", "Example organism");
        assert_ne!(source.key(), target.key());
        let assertion = mapping(source, target);
        assert_eq!(assertion.status, TaxonMappingStatus::Proposed);
        assert_eq!(assertion.relation, TaxonMappingRelation::ExactMatch);
        assert!(assertion.validate().is_ok());
    }

    #[test]
    fn exact_match_and_congruent_are_distinct_mapping_relations() {
        assert_ne!(
            TaxonMappingRelation::ExactMatch,
            TaxonMappingRelation::Congruent
        );
    }

    #[test]
    fn directional_inclusion_relations_have_explicit_inverses() {
        assert_eq!(
            TaxonMappingRelation::Includes.inverse(),
            TaxonMappingRelation::IncludedIn
        );
        assert_eq!(
            TaxonMappingRelation::IncludedIn.inverse(),
            TaxonMappingRelation::Includes
        );
        assert_eq!(
            TaxonMappingRelation::Overlaps.inverse(),
            TaxonMappingRelation::Overlaps
        );
    }

    #[test]
    fn no_match_and_unresolved_cannot_be_accepted() {
        let source = taxon("A", "dataset-a", "1", "A:1", "One");
        let target = taxon("B", "dataset-b", "1", "B:1", "Two");
        for relation in [TaxonMappingRelation::NoMatch, TaxonMappingRelation::Unresolved] {
            let mut assertion = mapping(source.clone(), target.clone());
            assertion.relation = relation;
            assertion.status = TaxonMappingStatus::Accepted;
            assertion.evidence_refs = vec!["receipt:review-1".to_owned()];
            assert_eq!(
                assertion.validate(),
                Err(LivingSystemIdentityError::InvalidAcceptedMappingRelation)
            );
        }
    }

    #[test]
    fn reviewed_or_accepted_mapping_requires_opaque_evidence_reference() {
        let source = taxon("A", "dataset-a", "1", "A:1", "One");
        let target = taxon("B", "dataset-b", "1", "B:1", "Two");
        let mut assertion = mapping(source, target);
        assertion.status = TaxonMappingStatus::Reviewed;
        assert_eq!(
            assertion.validate(),
            Err(LivingSystemIdentityError::MissingMappingEvidence)
        );
        assertion.status = TaxonMappingStatus::Accepted;
        assertion.relation = TaxonMappingRelation::Congruent;
        assertion.evidence_refs = vec!["sol-atlas:crosswalk-review:42".to_owned()];
        assert!(assertion.validate().is_ok());
    }

    #[test]
    fn unresolved_mapping_can_be_preserved_as_proposed_without_false_resolution() {
        let source = taxon("A", "dataset-a", "1", "A:1", "One");
        let target = taxon("B", "dataset-b", "1", "B:1", "One");
        let mut assertion = mapping(source, target);
        assertion.relation = TaxonMappingRelation::Unresolved;
        assert!(assertion.validate().is_ok());
    }

    #[test]
    fn one_to_many_release_split_and_conflicting_mappings_are_preserved() {
        let source = taxon("A", "dataset-a", "2025", "A:old", "Species alpha");
        let target_one = taxon("A", "dataset-a", "2026", "A:new-1", "Species alpha");
        let target_two = taxon("A", "dataset-a", "2026", "A:new-2", "Species alpha beta");
        let mut first = mapping(source.clone(), target_one);
        first.mapping_id = "split-1".to_owned();
        first.relation = TaxonMappingRelation::Includes;
        let mut second = mapping(source, target_two);
        second.mapping_id = "split-2".to_owned();
        second.relation = TaxonMappingRelation::Overlaps;
        assert!(validate_taxon_mappings(&[first, second]).is_ok());
    }

    #[test]
    fn duplicate_mapping_and_evidence_ids_fail_closed() {
        let source = taxon("A", "dataset-a", "1", "A:1", "One");
        let target = taxon("B", "dataset-b", "1", "B:1", "Two");
        let first = mapping(source.clone(), target.clone());
        let second = first.clone();
        assert_eq!(
            validate_taxon_mappings(&[first, second]),
            Err(LivingSystemIdentityError::DuplicateTaxonMappingId(
                "mapping-1".to_owned()
            ))
        );

        let mut duplicate_evidence = mapping(source, target);
        duplicate_evidence.status = TaxonMappingStatus::Reviewed;
        duplicate_evidence.evidence_refs =
            vec!["receipt:1".to_owned(), "receipt:1".to_owned()];
        assert_eq!(
            duplicate_evidence.validate(),
            Err(LivingSystemIdentityError::DuplicateEvidenceReference(
                "receipt:1".to_owned()
            ))
        );
    }

    #[test]
    fn self_mapping_is_rejected() {
        let source = taxon("A", "dataset-a", "1", "A:1", "One");
        let assertion = mapping(source.clone(), source);
        assert_eq!(
            assertion.validate(),
            Err(LivingSystemIdentityError::SelfTaxonMapping)
        );
    }

    #[test]
    fn input_permutation_does_not_change_graph_validity() {
        let graph = LivingSystemGraph {
            nodes: vec![
                node("ecosystem", LivingSystemLevel::Ecosystem, vec![member("e", bacterium())]),
                node("community", LivingSystemLevel::Community, vec![member("c", bacterium())]),
                node("population", LivingSystemLevel::Population, vec![member("p", bacterium())]),
            ],
            edges: vec![
                edge("ec", "ecosystem", "community", LivingSystemRelation::Contains),
                edge("cp", "community", "population", LivingSystemRelation::Contains),
            ],
        };
        let mut permuted = graph.clone();
        permuted.nodes.reverse();
        permuted.edges.reverse();
        assert_eq!(graph.validate(), permuted.validate());
    }

    #[test]
    fn parallel_relationships_between_same_nodes_are_allowed() {
        let graph = LivingSystemGraph {
            nodes: vec![
                node("a", LivingSystemLevel::Individual, vec![member("a", bacterium())]),
                node("b", LivingSystemLevel::Individual, vec![member("b", bacterium())]),
            ],
            edges: vec![
                edge("symbiosis", "a", "b", LivingSystemRelation::SymbioticWith),
                edge("chemical", "a", "b", LivingSystemRelation::ExchangesChemicalSignalWith),
                edge("matter", "a", "b", LivingSystemRelation::ExchangesMatterWith),
            ],
        };
        assert!(graph.validate().is_ok());
    }
}
