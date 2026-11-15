// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit hyphal morphology graph primitives.
//!
//! This module is a topology/identity substrate only. It does not model fungal
//! physiology, growth kinetics, transport conductance, nutrient preference,
//! symbiosis, or biological parameter values. Those require separate,
//! evidence-qualified process models.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

/// Stable identity for a hyphal graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub u64);

/// Stable identity for a hyphal segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SegmentId(pub u64);

/// Stable identity for an applied topology event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EventId(pub u64);

/// Three-dimensional coordinate in millimetres.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Position3Mm {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Position3Mm {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }

    pub fn midpoint(self, other: Self) -> Self {
        Self {
            x: self.x * 0.5 + other.x * 0.5,
            y: self.y * 0.5 + other.y * 0.5,
            z: self.z * 0.5 + other.z * 0.5,
        }
    }

    /// Numerically stable Euclidean distance in millimetres.
    pub fn distance_to(self, other: Self) -> f64 {
        (self.x - other.x)
            .hypot(self.y - other.y)
            .hypot(self.z - other.z)
    }

    fn interpolate(self, other: Self, fraction: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * fraction,
            y: self.y + (other.y - self.y) * fraction,
            z: self.z + (other.z - self.z) * fraction,
        }
    }
}

/// Topological role only; these labels do not imply a particular fungal taxon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyphalNodeKind {
    Anchor,
    ActiveTip,
    BranchPoint,
    Junction,
    Fusion,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HyphalNode {
    pub id: NodeId,
    pub kind: HyphalNodeKind,
    pub position: Position3Mm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyphalSegmentState {
    Active,
    Inactive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HyphalSegment {
    pub id: SegmentId,
    pub from: NodeId,
    pub to: NodeId,
    pub length_mm: f64,
    /// None means diameter is not specified; it is not zero.
    pub diameter_mm: Option<f64>,
    pub state: HyphalSegmentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    Applied,
    AlreadyApplied,
}

/// Splits an existing segment at an interior attachment point and adds a branch.
/// The original segment is replaced by two segments whose lengths sum exactly
/// to the original declared length. The new branch length is geometric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BranchEvent {
    pub id: EventId,
    pub parent_segment: SegmentId,
    pub attachment_fraction: f64,
    pub branch_node: NodeId,
    pub branch_tip: NodeId,
    pub proximal_segment: SegmentId,
    pub distal_segment: SegmentId,
    pub branch_segment: SegmentId,
    pub branch_tip_position: Position3Mm,
}

/// Joins two distinct active tips using a new fusion node and two connecting
/// segments. Fusion is a topology operation; biological compatibility and
/// transport capability must be qualified elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FusionEvent {
    pub id: EventId,
    pub first_tip: NodeId,
    pub second_tip: NodeId,
    pub fusion_node: NodeId,
    pub first_segment: SegmentId,
    pub second_segment: SegmentId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppliedGraphEvent {
    Branch(BranchEvent),
    Fusion(FusionEvent),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
    NonFinitePosition,
    NonPositiveSegmentLength,
    InvalidDiameter,
    ZeroLengthGeometry,
    SegmentEndpointsEqual,
    DuplicateNode(NodeId),
    DuplicateSegment(SegmentId),
    MissingNode(NodeId),
    MissingSegment(SegmentId),
    InvalidAttachmentFraction,
    NotActiveTip(NodeId),
    SameFusionTip,
    EventIdConflict(EventId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePosition => write!(f, "node position must be finite"),
            Self::NonPositiveSegmentLength => write!(f, "segment length must be finite and positive"),
            Self::InvalidDiameter => write!(f, "diameter must be finite and positive when present"),
            Self::ZeroLengthGeometry => write!(f, "event would create zero-length geometry"),
            Self::SegmentEndpointsEqual => write!(f, "segment endpoints must be distinct"),
            Self::DuplicateNode(id) => write!(f, "node {:?} already exists", id),
            Self::DuplicateSegment(id) => write!(f, "segment {:?} already exists", id),
            Self::MissingNode(id) => write!(f, "node {:?} does not exist", id),
            Self::MissingSegment(id) => write!(f, "segment {:?} does not exist", id),
            Self::InvalidAttachmentFraction => write!(f, "branch attachment fraction must be inside (0, 1)"),
            Self::NotActiveTip(id) => write!(f, "node {:?} is not an active tip", id),
            Self::SameFusionTip => write!(f, "fusion requires two distinct tips"),
            Self::EventIdConflict(id) => write!(f, "event ID {:?} was reused with a different payload", id),
        }
    }
}

impl std::error::Error for GraphError {}

/// Deterministic, ordered topology container.
///
/// IDs are supplied by the caller so they can be bound to source events and
/// stable replay records. This type has no random number generator and does
/// not create biological parameters.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HyphalGraph {
    nodes: BTreeMap<NodeId, HyphalNode>,
    segments: BTreeMap<SegmentId, HyphalSegment>,
    applied_events: BTreeMap<EventId, AppliedGraphEvent>,
}

impl HyphalGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn nodes(&self) -> impl Iterator<Item = &HyphalNode> {
        self.nodes.values()
    }

    pub fn segments(&self) -> impl Iterator<Item = &HyphalSegment> {
        self.segments.values()
    }

    pub fn node(&self, id: NodeId) -> Option<&HyphalNode> {
        self.nodes.get(&id)
    }

    pub fn segment(&self, id: SegmentId) -> Option<&HyphalSegment> {
        self.segments.get(&id)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }

    pub fn contains_event(&self, id: EventId) -> bool {
        self.applied_events.contains_key(&id)
    }

    pub fn total_length_mm(&self) -> f64 {
        self.segments.values().map(|segment| segment.length_mm).sum()
    }

    pub fn add_node(&mut self, node: HyphalNode) -> Result<(), GraphError> {
        if !node.position.is_finite() {
            return Err(GraphError::NonFinitePosition);
        }
        if self.nodes.contains_key(&node.id) {
            return Err(GraphError::DuplicateNode(node.id));
        }
        self.nodes.insert(node.id, node);
        Ok(())
    }

    pub fn add_segment(&mut self, segment: HyphalSegment) -> Result<(), GraphError> {
        validate_segment(&segment)?;
        if segment.from == segment.to {
            return Err(GraphError::SegmentEndpointsEqual);
        }
        if !self.nodes.contains_key(&segment.from) {
            return Err(GraphError::MissingNode(segment.from));
        }
        if !self.nodes.contains_key(&segment.to) {
            return Err(GraphError::MissingNode(segment.to));
        }
        if self.segments.contains_key(&segment.id) {
            return Err(GraphError::DuplicateSegment(segment.id));
        }
        self.segments.insert(segment.id, segment);
        Ok(())
    }

    /// Apply a branch transactionally. A repeated event ID is idempotent.
    pub fn apply_branch(&mut self, event: BranchEvent) -> Result<ApplyOutcome, GraphError> {
        if let Some(applied) = self.applied_events.get(&event.id) {
            return if *applied == AppliedGraphEvent::Branch(event) {
                Ok(ApplyOutcome::AlreadyApplied)
            } else {
                Err(GraphError::EventIdConflict(event.id))
            };
        }
        if !event.attachment_fraction.is_finite()
            || event.attachment_fraction <= 0.0
            || event.attachment_fraction >= 1.0
        {
            return Err(GraphError::InvalidAttachmentFraction);
        }
        if !event.branch_tip_position.is_finite() {
            return Err(GraphError::NonFinitePosition);
        }

        let parent = self
            .segments
            .get(&event.parent_segment)
            .cloned()
            .ok_or(GraphError::MissingSegment(event.parent_segment))?;
        let from = self
            .nodes
            .get(&parent.from)
            .ok_or(GraphError::MissingNode(parent.from))?;
        let to = self
            .nodes
            .get(&parent.to)
            .ok_or(GraphError::MissingNode(parent.to))?;
        let branch_position = from
            .position
            .interpolate(to.position, event.attachment_fraction);
        if !branch_position.is_finite() {
            return Err(GraphError::NonFinitePosition);
        }
        let branch_length = branch_position.distance_to(event.branch_tip_position);
        if !branch_length.is_finite() || branch_length <= 0.0 {
            return Err(GraphError::ZeroLengthGeometry);
        }

        let mut next = self.clone();
        next.segments.remove(&event.parent_segment);
        next.add_node(HyphalNode {
            id: event.branch_node,
            kind: HyphalNodeKind::BranchPoint,
            position: branch_position,
        })?;
        next.add_node(HyphalNode {
            id: event.branch_tip,
            kind: HyphalNodeKind::ActiveTip,
            position: event.branch_tip_position,
        })?;
        next.add_segment(HyphalSegment {
            id: event.proximal_segment,
            from: parent.from,
            to: event.branch_node,
            length_mm: parent.length_mm * event.attachment_fraction,
            diameter_mm: parent.diameter_mm,
            state: parent.state,
        })?;
        next.add_segment(HyphalSegment {
            id: event.distal_segment,
            from: event.branch_node,
            to: parent.to,
            length_mm: parent.length_mm * (1.0 - event.attachment_fraction),
            diameter_mm: parent.diameter_mm,
            state: parent.state,
        })?;
        next.add_segment(HyphalSegment {
            id: event.branch_segment,
            from: event.branch_node,
            to: event.branch_tip,
            length_mm: branch_length,
            diameter_mm: parent.diameter_mm,
            state: HyphalSegmentState::Active,
        })?;
        next.applied_events.insert(event.id, AppliedGraphEvent::Branch(event));
        *self = next;
        Ok(ApplyOutcome::Applied)
    }

    /// Fuse two active tips transactionally. The new connecting segment
    /// diameters remain unknown rather than being inferred from either parent.
    pub fn apply_fusion(&mut self, event: FusionEvent) -> Result<ApplyOutcome, GraphError> {
        if let Some(applied) = self.applied_events.get(&event.id) {
            return if *applied == AppliedGraphEvent::Fusion(event) {
                Ok(ApplyOutcome::AlreadyApplied)
            } else {
                Err(GraphError::EventIdConflict(event.id))
            };
        }
        if event.first_tip == event.second_tip {
            return Err(GraphError::SameFusionTip);
        }
        let first = self
            .nodes
            .get(&event.first_tip)
            .cloned()
            .ok_or(GraphError::MissingNode(event.first_tip))?;
        let second = self
            .nodes
            .get(&event.second_tip)
            .cloned()
            .ok_or(GraphError::MissingNode(event.second_tip))?;
        if first.kind != HyphalNodeKind::ActiveTip {
            return Err(GraphError::NotActiveTip(event.first_tip));
        }
        if second.kind != HyphalNodeKind::ActiveTip {
            return Err(GraphError::NotActiveTip(event.second_tip));
        }

        let fusion_position = first.position.midpoint(second.position);
        if !fusion_position.is_finite() {
            return Err(GraphError::NonFinitePosition);
        }
        let first_length = first.position.distance_to(fusion_position);
        let second_length = second.position.distance_to(fusion_position);
        if !first_length.is_finite()
            || !second_length.is_finite()
            || first_length <= 0.0
            || second_length <= 0.0
        {
            return Err(GraphError::ZeroLengthGeometry);
        }

        let mut next = self.clone();
        next.nodes.get_mut(&event.first_tip).expect("validated first tip").kind =
            HyphalNodeKind::Junction;
        next.nodes.get_mut(&event.second_tip).expect("validated second tip").kind =
            HyphalNodeKind::Junction;
        next.add_node(HyphalNode {
            id: event.fusion_node,
            kind: HyphalNodeKind::Fusion,
            position: fusion_position,
        })?;
        next.add_segment(HyphalSegment {
            id: event.first_segment,
            from: event.first_tip,
            to: event.fusion_node,
            length_mm: first_length,
            diameter_mm: None,
            state: HyphalSegmentState::Active,
        })?;
        next.add_segment(HyphalSegment {
            id: event.second_segment,
            from: event.second_tip,
            to: event.fusion_node,
            length_mm: second_length,
            diameter_mm: None,
            state: HyphalSegmentState::Active,
        })?;
        next.applied_events.insert(event.id, AppliedGraphEvent::Fusion(event));
        *self = next;
        Ok(ApplyOutcome::Applied)
    }

    /// Return deterministic connected components, each sorted by NodeId.
    pub fn connected_components(&self) -> Vec<Vec<NodeId>> {
        let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = self
            .nodes
            .keys()
            .copied()
            .map(|id| (id, Vec::new()))
            .collect();
        for segment in self.segments.values() {
            if let Some(neighbors) = adjacency.get_mut(&segment.from) {
                neighbors.push(segment.to);
            }
            if let Some(neighbors) = adjacency.get_mut(&segment.to) {
                neighbors.push(segment.from);
            }
        }

        let mut visited = BTreeSet::new();
        let mut components = Vec::new();
        for start in self.nodes.keys().copied() {
            if visited.contains(&start) {
                continue;
            }
            let mut queue = VecDeque::from([start]);
            let mut component = Vec::new();
            visited.insert(start);
            while let Some(current) = queue.pop_front() {
                component.push(current);
                if let Some(neighbors) = adjacency.get(&current) {
                    for neighbor in neighbors {
                        if visited.insert(*neighbor) {
                            queue.push_back(*neighbor);
                        }
                    }
                }
            }
            component.sort_unstable();
            components.push(component);
        }
        components
    }

    pub fn has_path(&self, from: NodeId, to: NodeId) -> bool {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return false;
        }
        self.connected_components()
            .iter()
            .any(|component| component.binary_search(&from).is_ok()
                && component.binary_search(&to).is_ok())
    }
}

fn validate_segment(segment: &HyphalSegment) -> Result<(), GraphError> {
    if !segment.length_mm.is_finite() || segment.length_mm <= 0.0 {
        return Err(GraphError::NonPositiveSegmentLength);
    }
    if let Some(diameter) = segment.diameter_mm
        && (!diameter.is_finite() || diameter <= 0.0)
    {
        return Err(GraphError::InvalidDiameter);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: u64, kind: HyphalNodeKind, x: f64, y: f64, z: f64) -> HyphalNode {
        HyphalNode {
            id: NodeId(id),
            kind,
            position: Position3Mm::new(x, y, z),
        }
    }

    fn segment(id: u64, from: u64, to: u64, length_mm: f64) -> HyphalSegment {
        HyphalSegment {
            id: SegmentId(id),
            from: NodeId(from),
            to: NodeId(to),
            length_mm,
            diameter_mm: Some(0.02),
            state: HyphalSegmentState::Active,
        }
    }

    fn linear_graph() -> HyphalGraph {
        let mut graph = HyphalGraph::new();
        graph.add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(2, HyphalNodeKind::ActiveTip, 10.0, 0.0, 0.0)).unwrap();
        graph.add_segment(segment(1, 1, 2, 10.0)).unwrap();
        graph
    }

    #[test]
    fn linear_graph_has_one_component_and_exact_length() {
        let graph = linear_graph();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.segment_count(), 1);
        assert_eq!(graph.connected_components(), vec![vec![NodeId(1), NodeId(2)]]);
        assert!(graph.has_path(NodeId(1), NodeId(2)));
        assert_eq!(graph.total_length_mm(), 10.0);
    }

    #[test]
    fn branch_splits_parent_and_preserves_declared_parent_length() {
        let mut graph = linear_graph();
        let outcome = graph.apply_branch(BranchEvent {
            id: EventId(10),
            parent_segment: SegmentId(1),
            attachment_fraction: 0.5,
            branch_node: NodeId(3),
            branch_tip: NodeId(4),
            proximal_segment: SegmentId(2),
            distal_segment: SegmentId(3),
            branch_segment: SegmentId(4),
            branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
        }).unwrap();

        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(graph.segment_count(), 3);
        assert_eq!(graph.total_length_mm(), 15.0);
        assert_eq!(graph.connected_components().len(), 1);
        let branch_degree = graph.segments()
            .filter(|edge| edge.from == NodeId(3) || edge.to == NodeId(3))
            .count();
        assert_eq!(branch_degree, 3);
    }

    #[test]
    fn repeated_branch_event_is_idempotent() {
        let mut graph = linear_graph();
        let event = BranchEvent {
            id: EventId(11),
            parent_segment: SegmentId(1),
            attachment_fraction: 0.5,
            branch_node: NodeId(3),
            branch_tip: NodeId(4),
            proximal_segment: SegmentId(2),
            distal_segment: SegmentId(3),
            branch_segment: SegmentId(4),
            branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
        };
        assert_eq!(graph.apply_branch(event).unwrap(), ApplyOutcome::Applied);
        let snapshot = graph.clone();
        assert_eq!(graph.apply_branch(event).unwrap(), ApplyOutcome::AlreadyApplied);
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn reusing_event_id_with_a_different_payload_is_rejected() {
        let mut graph = linear_graph();
        let original = BranchEvent {
            id: EventId(13),
            parent_segment: SegmentId(1),
            attachment_fraction: 0.5,
            branch_node: NodeId(3),
            branch_tip: NodeId(4),
            proximal_segment: SegmentId(2),
            distal_segment: SegmentId(3),
            branch_segment: SegmentId(4),
            branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
        };
        graph.apply_branch(original).unwrap();
        let snapshot = graph.clone();
        let conflicting = BranchEvent {
            branch_tip_position: Position3Mm::new(5.0, 6.0, 0.0),
            ..original
        };
        assert_eq!(
            graph.apply_branch(conflicting),
            Err(GraphError::EventIdConflict(EventId(13)))
        );
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn invalid_branch_fraction_leaves_graph_unchanged() {
        let mut graph = linear_graph();
        let snapshot = graph.clone();
        let result = graph.apply_branch(BranchEvent {
            id: EventId(12),
            parent_segment: SegmentId(1),
            attachment_fraction: 1.0,
            branch_node: NodeId(3),
            branch_tip: NodeId(4),
            proximal_segment: SegmentId(2),
            distal_segment: SegmentId(3),
            branch_segment: SegmentId(4),
            branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
        });
        assert_eq!(result, Err(GraphError::InvalidAttachmentFraction));
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn disconnected_tips_have_no_path() {
        let mut graph = HyphalGraph::new();
        graph.add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(2, HyphalNodeKind::ActiveTip, 5.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(3, HyphalNodeKind::Anchor, 0.0, 10.0, 0.0)).unwrap();
        graph.add_node(node(4, HyphalNodeKind::ActiveTip, 5.0, 10.0, 0.0)).unwrap();
        graph.add_segment(segment(1, 1, 2, 5.0)).unwrap();
        graph.add_segment(segment(2, 3, 4, 5.0)).unwrap();
        assert_eq!(graph.connected_components().len(), 2);
        assert!(!graph.has_path(NodeId(1), NodeId(4)));
    }

    #[test]
    fn fusion_joins_components_and_deactivates_old_tips() {
        let mut graph = HyphalGraph::new();
        graph.add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(2, HyphalNodeKind::ActiveTip, 5.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(3, HyphalNodeKind::Anchor, 10.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(4, HyphalNodeKind::ActiveTip, 5.0, 0.1, 0.0)).unwrap();
        graph.add_segment(segment(1, 1, 2, 5.0)).unwrap();
        graph.add_segment(segment(2, 3, 4, 5.0)).unwrap();
        assert_eq!(graph.connected_components().len(), 2);

        let outcome = graph.apply_fusion(FusionEvent {
            id: EventId(20),
            first_tip: NodeId(2),
            second_tip: NodeId(4),
            fusion_node: NodeId(5),
            first_segment: SegmentId(3),
            second_segment: SegmentId(4),
        }).unwrap();

        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(graph.connected_components().len(), 1);
        assert!(graph.has_path(NodeId(1), NodeId(3)));
        assert_eq!(graph.node(NodeId(2)).unwrap().kind, HyphalNodeKind::Junction);
        assert_eq!(graph.node(NodeId(4)).unwrap().kind, HyphalNodeKind::Junction);
        assert_eq!(graph.node(NodeId(5)).unwrap().kind, HyphalNodeKind::Fusion);
    }

    #[test]
    fn fusion_requires_active_tips_and_is_transactional() {
        let mut graph = linear_graph();
        let snapshot = graph.clone();
        let result = graph.apply_fusion(FusionEvent {
            id: EventId(21),
            first_tip: NodeId(1),
            second_tip: NodeId(2),
            fusion_node: NodeId(3),
            first_segment: SegmentId(2),
            second_segment: SegmentId(3),
        });
        assert_eq!(result, Err(GraphError::NotActiveTip(NodeId(1))));
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn rejects_non_finite_and_zero_length_geometry() {
        let mut graph = HyphalGraph::new();
        assert_eq!(
            graph.add_node(node(1, HyphalNodeKind::Anchor, f64::NAN, 0.0, 0.0)),
            Err(GraphError::NonFinitePosition)
        );
        graph.add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0)).unwrap();
        graph.add_node(node(2, HyphalNodeKind::ActiveTip, 0.0, 0.0, 0.0)).unwrap();
        assert_eq!(
            graph.add_segment(segment(1, 1, 2, f64::INFINITY)),
            Err(GraphError::NonPositiveSegmentLength)
        );
    }
}
