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

    fn taxon(authority: &str, dataset: &str, release: &str, id: &str, name: &str) -> TaxonConceptRef {
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

    fn node(id: &str, level: LivingSystemLevel, members: Vec<LivingSystemMember>) -> LivingSystemNode {
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
        let gbif = taxon("GBIF Backbone", "gbif-backbone", "2023-08-28", "12345", "Bacterium example");
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
                member("host", taxon("NCBI", "taxonomy", "test-ncbi-snapshot", "NCBI:TEST-9606", "Homo sapiens")),
                member("microbe", bacterium()),
                member("fungus", taxon("Catalogue of Life", "7ddf754f-d193-4cc9-b351-99906754a03b", "COL26.10-XR", "col:EXAMPLE-FUNGUS", "Fungus example")),
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
                node("symbiont", LivingSystemLevel::Individual, vec![member("symbiont", bacterium())]),
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
