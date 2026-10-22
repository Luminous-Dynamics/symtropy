// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Deterministic hierarchical representation selection for planetary/system scenes.
//!
//! The host supplies screen-space error and asset residency. This module chooses
//! representation nodes to draw. It does not cull, load assets, apply transforms,
//! compute error bounds, or modify canonical body state.
//!
//! For replacement refinement, the host must mark children_cover_parent only if
//! the immediate child representations fully cover the parent. Missing content
//! is never silently interpreted as complete coverage.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Stable representation-node identity, distinct from semantic entity identity.
pub type RepresentationNodeId = u64;

/// Residency of one representation asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentAvailability {
    Ready,
    Loading,
    Unavailable,
}

/// How a parent combines with its descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyRefinement {
    /// Draw descendants instead of the parent when refinement succeeds.
    Replace,
    /// Draw parent plus descendants when refinement succeeds.
    Additive,
}

/// One immutable node in a body/proxy/tile/feature representation hierarchy.
///
/// Multiple nodes may share semantic_entity_id while representing that entity
/// at different scales. screen_error_px is supplied by the host; this function
/// validates finiteness, but cannot establish that the estimate is conservative.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchyNode {
    pub node_id: RepresentationNodeId,
    pub semantic_entity_id: u64,
    pub parent_id: Option<RepresentationNodeId>,
    pub children: Vec<RepresentationNodeId>,
    pub content: ContentAvailability,
    pub refinement: HierarchyRefinement,
    pub children_cover_parent: bool,
    pub screen_error_px: f32,
    /// Require finer content when a refinement is needed. A far-away selected
    /// body proxy need not be detail-pinned; use this for genuinely required detail.
    pub detail_pinned: bool,
    /// Request refinement even when the numerical error is below the policy threshold.
    pub force_refine: bool,
}

/// One pass's screen-space error budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HierarchyPolicy {
    pub max_screen_error_px: f32,
}

/// Why a parent is held instead of replaced by descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HierarchyHoldReason {
    WithinErrorBudget,
    ChildContentNotReady,
    ChildCoverageNotCertified,
}

/// A held parent and reason code for telemetry/diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldParent {
    pub node_id: RepresentationNodeId,
    pub reason: HierarchyHoldReason,
}

/// Representation chosen to draw this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedRepresentation {
    pub node_id: RepresentationNodeId,
    pub semantic_entity_id: u64,
}

/// Deterministic result and explicit streaming fallbacks.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchySelection {
    /// Chosen representations, sorted by stable node ID.
    pub render_nodes: Vec<SelectedRepresentation>,
    pub held_parent_nodes: Vec<HeldParent>,
    /// Parents whose requested refinement was satisfied, sorted by node ID.
    pub refined_parent_nodes: Vec<RepresentationNodeId>,
    /// Unavailable parents replaced by fully resident children that cover them.
    pub bypassed_unavailable_parent_nodes: Vec<RepresentationNodeId>,
}

/// Malformed hierarchy or unsatisfied representation constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchySelectionError {
    InvalidScreenErrorBudget,
    DuplicateRepresentationNodeId,
    DuplicateChildNode,
    MissingChildNode,
    ParentChildMismatch,
    OrphanedNode,
    HierarchyCycle,
    ReplacementChildrenDoNotCoverParent,
    NoRenderableRepresentation,
    ProtectedDetailUnavailable,
}

impl fmt::Display for HierarchySelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for HierarchySelectionError {}

/// Select representations for all root hierarchies under a hard error ceiling.
///
/// The hierarchy must be a forest. Input ordering does not affect the output.
/// Every chosen node retains its semantic entity ID even when node IDs change.
pub fn select_hierarchy(
    nodes: &[HierarchyNode],
    policy: HierarchyPolicy,
) -> Result<HierarchySelection, HierarchySelectionError> {
    if !policy.max_screen_error_px.is_finite() || policy.max_screen_error_px < 0.0 {
        return Err(HierarchySelectionError::InvalidScreenErrorBudget);
    }

    let mut index = BTreeMap::new();
    for node in nodes {
        if !node.screen_error_px.is_finite() || node.screen_error_px < 0.0 {
            return Err(HierarchySelectionError::InvalidScreenErrorBudget);
        }
        if index.insert(node.node_id, node).is_some() {
            return Err(HierarchySelectionError::DuplicateRepresentationNodeId);
        }
    }
    validate_structure(&index)?;

    let mut result = HierarchySelection {
        render_nodes: Vec::new(),
        held_parent_nodes: Vec::new(),
        refined_parent_nodes: Vec::new(),
        bypassed_unavailable_parent_nodes: Vec::new(),
    };

    let roots: Vec<_> = index
        .values()
        .filter(|node| node.parent_id.is_none())
        .map(|node| node.node_id)
        .collect();

    for root in roots {
        visit_node(root, &index, policy, &mut result)?;
    }

    result.render_nodes.sort_by_key(|choice| choice.node_id);
    result.held_parent_nodes.sort_by_key(|held| held.node_id);
    result.refined_parent_nodes.sort_unstable();
    result.bypassed_unavailable_parent_nodes.sort_unstable();
    Ok(result)
}

fn validate_structure(
    index: &BTreeMap<RepresentationNodeId, &HierarchyNode>,
) -> Result<(), HierarchySelectionError> {
    for node in index.values() {
        let mut child_set = BTreeSet::new();
        for child_id in &node.children {
            if !child_set.insert(*child_id) {
                return Err(HierarchySelectionError::DuplicateChildNode);
            }
            let child = index
                .get(child_id)
                .ok_or(HierarchySelectionError::MissingChildNode)?;
            if child.parent_id != Some(node.node_id) {
                return Err(HierarchySelectionError::ParentChildMismatch);
            }
        }
        if let Some(parent_id) = node.parent_id {
            let parent = index
                .get(&parent_id)
                .ok_or(HierarchySelectionError::OrphanedNode)?;
            if !parent.children.contains(&node.node_id) {
                return Err(HierarchySelectionError::ParentChildMismatch);
            }
        }
    }

    // Walk each parent chain once. Per-node fresh walks are quadratic on deep
    // hierarchies; completed nodes let later walks stop at a previously validated
    // prefix while still detecting cycles detached from every root.
    let mut completed = BTreeSet::new();
    for start_id in index.keys().copied() {
        let mut path = Vec::new();
        let mut active_path = BTreeSet::new();
        let mut current = Some(start_id);
        while let Some(node_id) = current {
            if completed.contains(&node_id) {
                break;
            }
            if !active_path.insert(node_id) {
                return Err(HierarchySelectionError::HierarchyCycle);
            }
            path.push(node_id);
            current = index
                .get(&node_id)
                .ok_or(HierarchySelectionError::OrphanedNode)?
                .parent_id;
        }
        completed.extend(path);
    }
    Ok(())
}

fn visit_node(
    node_id: RepresentationNodeId,
    index: &BTreeMap<RepresentationNodeId, &HierarchyNode>,
    policy: HierarchyPolicy,
    result: &mut HierarchySelection,
) -> Result<(), HierarchySelectionError> {
    let node = *index
        .get(&node_id)
        .ok_or(HierarchySelectionError::MissingChildNode)?;
    let has_children = !node.children.is_empty();
    let wants_refine =
        has_children && (node.force_refine || node.screen_error_px > policy.max_screen_error_px);

    if !wants_refine {
        return choose_current_or_bypass(node, index, policy, result);
    }

    if !node.children_cover_parent && node.refinement == HierarchyRefinement::Replace {
        if node.detail_pinned || node.force_refine {
            return Err(HierarchySelectionError::ReplacementChildrenDoNotCoverParent);
        }
        return hold_parent(node, HierarchyHoldReason::ChildCoverageNotCertified, result);
    }

    let children_ready = all_children_ready(node, index);
    if !children_ready {
        if node.detail_pinned || node.force_refine {
            return Err(HierarchySelectionError::ProtectedDetailUnavailable);
        }
        if node.refinement == HierarchyRefinement::Additive
            && node.content == ContentAvailability::Ready
        {
            result.render_nodes.push(selected(node));
            result.held_parent_nodes.push(HeldParent {
                node_id: node.node_id,
                reason: HierarchyHoldReason::ChildContentNotReady,
            });
            return Ok(());
        }
        if node.content == ContentAvailability::Ready {
            return hold_parent(node, HierarchyHoldReason::ChildContentNotReady, result);
        }
        return Err(HierarchySelectionError::NoRenderableRepresentation);
    }

    result.refined_parent_nodes.push(node.node_id);
    if node.refinement == HierarchyRefinement::Additive {
        if node.content == ContentAvailability::Ready {
            result.render_nodes.push(selected(node));
        } else if node.detail_pinned {
            return Err(HierarchySelectionError::ProtectedDetailUnavailable);
        } else if !node.children_cover_parent {
            return Err(HierarchySelectionError::NoRenderableRepresentation);
        }
    }
    for child_id in &node.children {
        visit_node(*child_id, index, policy, result)?;
    }
    Ok(())
}

fn choose_current_or_bypass(
    node: &HierarchyNode,
    index: &BTreeMap<RepresentationNodeId, &HierarchyNode>,
    policy: HierarchyPolicy,
    result: &mut HierarchySelection,
) -> Result<(), HierarchySelectionError> {
    if node.content == ContentAvailability::Ready {
        result.render_nodes.push(selected(node));
        if !node.children.is_empty() {
            result.held_parent_nodes.push(HeldParent {
                node_id: node.node_id,
                reason: HierarchyHoldReason::WithinErrorBudget,
            });
        }
        return Ok(());
    }

    if node.detail_pinned {
        return Err(HierarchySelectionError::ProtectedDetailUnavailable);
    }
    if node.children.is_empty() || !node.children_cover_parent || !all_children_ready(node, index) {
        return Err(HierarchySelectionError::NoRenderableRepresentation);
    }

    result.bypassed_unavailable_parent_nodes.push(node.node_id);
    for child_id in &node.children {
        visit_node(*child_id, index, policy, result)?;
    }
    Ok(())
}

fn hold_parent(
    node: &HierarchyNode,
    reason: HierarchyHoldReason,
    result: &mut HierarchySelection,
) -> Result<(), HierarchySelectionError> {
    if node.content != ContentAvailability::Ready {
        if node.detail_pinned || node.force_refine {
            return Err(HierarchySelectionError::ProtectedDetailUnavailable);
        }
        return Err(HierarchySelectionError::NoRenderableRepresentation);
    }
    result.render_nodes.push(selected(node));
    result.held_parent_nodes.push(HeldParent {
        node_id: node.node_id,
        reason,
    });
    Ok(())
}

fn selected(node: &HierarchyNode) -> SelectedRepresentation {
    SelectedRepresentation {
        node_id: node.node_id,
        semantic_entity_id: node.semantic_entity_id,
    }
}

fn all_children_ready(
    node: &HierarchyNode,
    index: &BTreeMap<RepresentationNodeId, &HierarchyNode>,
) -> bool {
    node.children.iter().all(|child_id| {
        index
            .get(child_id)
            .is_some_and(|child| child.content == ContentAvailability::Ready)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const POLICY: HierarchyPolicy = HierarchyPolicy {
        max_screen_error_px: 2.0,
    };

    fn node(
        node_id: u64,
        parent_id: Option<u64>,
        children: Vec<u64>,
        screen_error_px: f32,
    ) -> HierarchyNode {
        HierarchyNode {
            node_id,
            semantic_entity_id: 42,
            parent_id,
            children,
            content: ContentAvailability::Ready,
            refinement: HierarchyRefinement::Replace,
            children_cover_parent: true,
            screen_error_px,
            detail_pinned: false,
            force_refine: false,
        }
    }

    fn rendered(result: &HierarchySelection) -> Vec<u64> {
        result
            .render_nodes
            .iter()
            .map(|choice| choice.node_id)
            .collect()
    }

    #[test]
    fn within_error_budget_keeps_parent_representation() {
        let parent = node(1, None, vec![2, 3], 1.0);
        let children = [node(2, Some(1), vec![], 0.5), node(3, Some(1), vec![], 0.5)];
        let result = select_hierarchy(&[parent, children[0].clone(), children[1].clone()], POLICY)
            .expect("valid hierarchy");
        assert_eq!(rendered(&result), vec![1]);
        assert_eq!(
            result.held_parent_nodes,
            vec![HeldParent {
                node_id: 1,
                reason: HierarchyHoldReason::WithinErrorBudget,
            }]
        );
        assert!(result.refined_parent_nodes.is_empty());
    }

    #[test]
    fn above_error_budget_replaces_parent_with_all_ready_children() {
        let parent = node(1, None, vec![3, 2], 10.0);
        let child_a = node(2, Some(1), vec![], 0.5);
        let child_b = node(3, Some(1), vec![], 0.5);
        let result =
            select_hierarchy(&[parent, child_b, child_a], POLICY).expect("valid hierarchy");
        assert_eq!(rendered(&result), vec![2, 3]);
        assert_eq!(result.refined_parent_nodes, vec![1]);
    }

    #[test]
    fn incomplete_replacement_children_hold_ready_parent() {
        let parent = node(1, None, vec![2, 3], 10.0);
        let child_a = node(2, Some(1), vec![], 0.5);
        let mut child_b = node(3, Some(1), vec![], 0.5);
        child_b.content = ContentAvailability::Loading;
        let result =
            select_hierarchy(&[parent, child_a, child_b], POLICY).expect("parent fallback");
        assert_eq!(rendered(&result), vec![1]);
        assert_eq!(
            result.held_parent_nodes[0].reason,
            HierarchyHoldReason::ChildContentNotReady
        );
        assert!(result.refined_parent_nodes.is_empty());
    }

    #[test]
    fn pinned_detail_fails_when_required_children_are_not_ready() {
        let mut parent = node(1, None, vec![2, 3], 10.0);
        parent.detail_pinned = true;
        let child_a = node(2, Some(1), vec![], 0.5);
        let mut child_b = node(3, Some(1), vec![], 0.5);
        child_b.content = ContentAvailability::Unavailable;
        assert_eq!(
            select_hierarchy(&[parent, child_a, child_b], POLICY),
            Err(HierarchySelectionError::ProtectedDetailUnavailable)
        );
    }

    #[test]
    fn replacement_without_certified_coverage_is_rejected_for_forced_refinement() {
        let mut parent = node(1, None, vec![2], 10.0);
        parent.children_cover_parent = false;
        parent.force_refine = true;
        let child = node(2, Some(1), vec![], 0.1);
        assert_eq!(
            select_hierarchy(&[parent, child], POLICY),
            Err(HierarchySelectionError::ReplacementChildrenDoNotCoverParent)
        );
    }

    #[test]
    fn additive_refinement_keeps_parent_and_children() {
        let mut parent = node(1, None, vec![2], 10.0);
        parent.refinement = HierarchyRefinement::Additive;
        let child = node(2, Some(1), vec![], 0.5);
        let result = select_hierarchy(&[parent, child], POLICY).expect("additive hierarchy");
        assert_eq!(rendered(&result), vec![1, 2]);
    }

    #[test]
    fn force_refine_crosses_threshold_even_when_error_is_small() {
        let mut parent = node(1, None, vec![2], 0.5);
        parent.force_refine = true;
        let child = node(2, Some(1), vec![], 0.1);
        let result = select_hierarchy(&[parent, child], POLICY).expect("forced refinement");
        assert_eq!(rendered(&result), vec![2]);
    }

    #[test]
    fn unavailable_additive_parent_requires_certified_child_coverage() {
        let mut parent = node(1, None, vec![2], 10.0);
        parent.refinement = HierarchyRefinement::Additive;
        parent.children_cover_parent = false;
        parent.content = ContentAvailability::Unavailable;
        let child = node(2, Some(1), vec![], 0.5);
        assert_eq!(
            select_hierarchy(&[parent, child], POLICY),
            Err(HierarchySelectionError::NoRenderableRepresentation)
        );
    }

    #[test]
    fn unavailable_parent_can_be_bypassed_only_by_ready_covering_children() {
        let mut parent = node(1, None, vec![2, 3], 0.1);
        parent.content = ContentAvailability::Loading;
        let child_a = node(2, Some(1), vec![], 0.5);
        let child_b = node(3, Some(1), vec![], 0.5);
        let result = select_hierarchy(&[parent, child_a, child_b], POLICY).expect("child coverage");
        assert_eq!(rendered(&result), vec![2, 3]);
        assert_eq!(result.bypassed_unavailable_parent_nodes, vec![1]);
    }

    #[test]
    fn rejects_invalid_errors_and_malformed_parent_child_links() {
        let invalid = node(1, None, vec![], f32::NAN);
        assert_eq!(
            select_hierarchy(&[invalid], POLICY),
            Err(HierarchySelectionError::InvalidScreenErrorBudget)
        );
        let parent = node(1, None, vec![2], 3.0);
        let child = node(2, Some(9), vec![], 0.5);
        assert_eq!(
            select_hierarchy(&[parent, child], POLICY),
            Err(HierarchySelectionError::ParentChildMismatch)
        );
    }

    #[test]
    fn rejects_cycles_and_duplicate_child_ids() {
        let a = node(1, Some(2), vec![2], 3.0);
        let b = node(2, Some(1), vec![1], 3.0);
        assert_eq!(
            select_hierarchy(&[a, b], POLICY),
            Err(HierarchySelectionError::HierarchyCycle)
        );
        let parent = node(1, None, vec![2, 2], 3.0);
        let child = node(2, Some(1), vec![], 0.5);
        assert_eq!(
            select_hierarchy(&[parent, child], POLICY),
            Err(HierarchySelectionError::DuplicateChildNode)
        );
    }

    #[test]
    fn deep_hierarchy_validation_reuses_completed_parent_chains() {
        // A long chain is the worst case for repeated per-node ancestor walks.
        // Validate structure directly to isolate hierarchy checking from recursive
        // representation selection and rendering behavior.
        let depth = 4_096_u64;
        let nodes = (0..depth)
            .map(|node_id| {
                let parent_id = (node_id > 0).then_some(node_id - 1);
                let children = if node_id + 1 < depth {
                    vec![node_id + 1]
                } else {
                    Vec::new()
                };
                node(node_id, parent_id, children, 0.5)
            })
            .collect::<Vec<_>>();
        let index = nodes
            .iter()
            .map(|node| (node.node_id, node))
            .collect::<BTreeMap<_, _>>();

        assert_eq!(validate_structure(&index), Ok(()));
    }

    #[test]
    fn selection_is_deterministic_under_input_permutation() {
        let parent = node(1, None, vec![2, 3], 10.0);
        let child_a = node(2, Some(1), vec![], 0.5);
        let child_b = node(3, Some(1), vec![], 0.5);
        let forward = select_hierarchy(&[parent.clone(), child_a.clone(), child_b.clone()], POLICY)
            .expect("forward order");
        let reverse = select_hierarchy(&[child_b, parent, child_a], POLICY).expect("reverse order");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn semantic_identity_survives_coarse_to_refined_representation_switches() {
        // Use a non-default identity so a regression that accidentally writes the
        // fixture's default semantic ID (42) cannot satisfy this assertion.
        let mut parent = node(1, None, vec![2, 3], 1.0);
        parent.semantic_entity_id = 9_001;
        let mut child_a = node(2, Some(1), vec![], 0.5);
        child_a.semantic_entity_id = 9_001;
        let mut child_b = node(3, Some(1), vec![], 0.5);
        child_b.semantic_entity_id = 9_001;

        let coarse = select_hierarchy(
            &[parent.clone(), child_a.clone(), child_b.clone()],
            POLICY,
        )
        .expect("coarse representation");
        assert_eq!(rendered(&coarse), vec![1]);
        assert_eq!(
            coarse.render_nodes[0].semantic_entity_id,
            9_001,
            "coarse representation must retain its semantic entity identity"
        );

        parent.screen_error_px = 10.0;
        let refined = select_hierarchy(&[parent, child_a, child_b], POLICY)
            .expect("refined representation");
        assert_eq!(rendered(&refined), vec![2, 3]);
        assert!(
            refined
                .render_nodes
                .iter()
                .all(|node| node.semantic_entity_id == 9_001),
            "representation changes must not change semantic entity identity"
        );
    }

    #[test]
    fn rejects_bad_budgets_and_missing_fallback_content() {
        assert_eq!(
            select_hierarchy(
                &[],
                HierarchyPolicy {
                    max_screen_error_px: f32::INFINITY
                }
            ),
            Err(HierarchySelectionError::InvalidScreenErrorBudget)
        );
        let mut lone = node(1, None, vec![], 1.0);
        lone.content = ContentAvailability::Loading;
        assert_eq!(
            select_hierarchy(&[lone], POLICY),
            Err(HierarchySelectionError::NoRenderableRepresentation)
        );
    }
}
