// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for canonical plant structural-topology invariants.
//!
//! This intentionally freezes graph semantics independently of the future plant
//! product API. Canonical plants persist biological structure, not render meshes.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ElementId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OrganKind {
    Root,
    Stem,
    Branch,
    Bud,
    Leaf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StructuralNode {
    id: ElementId,
    parent: Option<ElementId>,
    kind: OrganKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TopologyError {
    MissingElement(ElementId),
    CannotSeverRoot,
    IdExhausted,
    BrokenParent {
        child: ElementId,
        parent: ElementId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PlantGraph {
    nodes: BTreeMap<ElementId, StructuralNode>,
    next_id: u64,
}

impl PlantGraph {
    fn new() -> Self {
        let root = StructuralNode {
            id: ElementId(0),
            parent: None,
            kind: OrganKind::Root,
        };
        Self {
            nodes: BTreeMap::from([(root.id, root)]),
            next_id: 1,
        }
    }

    fn root(&self) -> ElementId {
        ElementId(0)
    }

    fn append_child(
        &mut self,
        parent: ElementId,
        kind: OrganKind,
    ) -> Result<ElementId, TopologyError> {
        if !self.nodes.contains_key(&parent) {
            return Err(TopologyError::MissingElement(parent));
        }

        let id = ElementId(self.next_id);
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or(TopologyError::IdExhausted)?;
        self.nodes.insert(
            id,
            StructuralNode {
                id,
                parent: Some(parent),
                kind,
            },
        );
        Ok(id)
    }

    fn descendants_including(&self, target: ElementId) -> Result<BTreeSet<ElementId>, TopologyError> {
        if !self.nodes.contains_key(&target) {
            return Err(TopologyError::MissingElement(target));
        }

        let mut selected = BTreeSet::from([target]);
        loop {
            let before = selected.len();
            for node in self.nodes.values() {
                if node.parent.is_some_and(|parent| selected.contains(&parent)) {
                    selected.insert(node.id);
                }
            }
            if selected.len() == before {
                return Ok(selected);
            }
        }
    }

    fn sever_subtree(&mut self, target: ElementId) -> Result<BTreeSet<ElementId>, TopologyError> {
        if target == self.root() {
            return Err(TopologyError::CannotSeverRoot);
        }

        let removed = self.descendants_including(target)?;
        for id in &removed {
            self.nodes.remove(id);
        }
        self.validate()?;
        Ok(removed)
    }

    fn validate(&self) -> Result<(), TopologyError> {
        for node in self.nodes.values() {
            if let Some(parent) = node.parent {
                if !self.nodes.contains_key(&parent) {
                    return Err(TopologyError::BrokenParent {
                        child: node.id,
                        parent,
                    });
                }
            }
        }
        Ok(())
    }
}

#[test]
fn severing_internal_branch_removes_exact_canonical_subtree() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let branch_a = graph.append_child(stem, OrganKind::Branch).unwrap();
    let branch_b = graph.append_child(stem, OrganKind::Branch).unwrap();
    let bud = graph.append_child(branch_a, OrganKind::Bud).unwrap();
    let leaf = graph.append_child(bud, OrganKind::Leaf).unwrap();

    let removed = graph.sever_subtree(branch_a).unwrap();

    assert_eq!(removed, BTreeSet::from([branch_a, bud, leaf]));
    assert!(graph.nodes.contains_key(&graph.root()));
    assert!(graph.nodes.contains_key(&stem));
    assert!(graph.nodes.contains_key(&branch_b));
    assert!(!graph.nodes.contains_key(&branch_a));
    assert!(!graph.nodes.contains_key(&bud));
    assert!(!graph.nodes.contains_key(&leaf));
    assert_eq!(graph.validate(), Ok(()));
}

#[test]
fn severed_element_ids_are_never_reused_for_regrowth() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let old_branch = graph.append_child(stem, OrganKind::Branch).unwrap();
    let old_bud = graph.append_child(old_branch, OrganKind::Bud).unwrap();

    graph.sever_subtree(old_branch).unwrap();
    let replacement = graph.append_child(stem, OrganKind::Branch).unwrap();

    assert!(replacement > old_branch);
    assert!(replacement > old_bud);
    assert_eq!(
        graph.sever_subtree(old_branch),
        Err(TopologyError::MissingElement(old_branch))
    );
}

#[test]
fn stale_parent_reference_cannot_attach_new_growth() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let branch = graph.append_child(stem, OrganKind::Branch).unwrap();
    graph.sever_subtree(branch).unwrap();

    assert_eq!(
        graph.append_child(branch, OrganKind::Bud),
        Err(TopologyError::MissingElement(branch))
    );
    assert_eq!(graph.validate(), Ok(()));
}

#[test]
fn root_cannot_be_removed_by_ordinary_breakage() {
    let mut graph = PlantGraph::new();
    let snapshot = graph.clone();

    assert_eq!(
        graph.sever_subtree(graph.root()),
        Err(TopologyError::CannotSeverRoot)
    );
    assert_eq!(graph, snapshot);
}

#[test]
fn render_rebuild_equivalent_clone_preserves_canonical_topology() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let branch = graph.append_child(stem, OrganKind::Branch).unwrap();
    let _leaf = graph.append_child(branch, OrganKind::Leaf).unwrap();

    let before_render_teardown = graph.clone();
    let rebuilt_presentation_source = graph.clone();

    assert_eq!(rebuilt_presentation_source, before_render_teardown);
    assert_eq!(graph, before_render_teardown);
}

#[test]
fn topology_validation_detects_orphaned_live_structure() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let branch = graph.append_child(stem, OrganKind::Branch).unwrap();

    graph.nodes.remove(&stem);

    assert_eq!(
        graph.validate(),
        Err(TopologyError::BrokenParent {
            child: branch,
            parent: stem,
        })
    );
}

#[test]
fn independent_sibling_growth_does_not_change_existing_ids_or_parentage() {
    let mut graph = PlantGraph::new();
    let stem = graph.append_child(graph.root(), OrganKind::Stem).unwrap();
    let branch_a = graph.append_child(stem, OrganKind::Branch).unwrap();
    let snapshot_a = graph.nodes[&branch_a];

    let branch_b = graph.append_child(stem, OrganKind::Branch).unwrap();
    let leaf_b = graph.append_child(branch_b, OrganKind::Leaf).unwrap();

    assert_eq!(graph.nodes[&branch_a], snapshot_a);
    assert_eq!(graph.nodes[&branch_b].parent, Some(stem));
    assert_eq!(graph.nodes[&leaf_b].parent, Some(branch_b));
    assert_eq!(graph.validate(), Ok(()));
}
