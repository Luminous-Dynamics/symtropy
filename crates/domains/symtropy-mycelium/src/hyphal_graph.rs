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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ActiveGraphCriticality {
    /// Active segments whose removal increases the number of connected components.
    pub bridge_segments: Vec<SegmentId>,
    /// Nodes whose removal increases the number of connected components.
    pub articulation_nodes: Vec<NodeId>,
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

/// Joins one active tip to an interior point of an existing active segment.
/// The target segment is split into proximal/distal pieces and a connector is
/// created. This matches the topology operation used by the published BMX
/// reference; it does not decide whether biological fusion is admissible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipSegmentFusionEvent {
    pub id: EventId,
    pub tip: NodeId,
    pub target_segment: SegmentId,
    pub attachment_fraction: f64,
    pub fusion_node: NodeId,
    pub proximal_segment: SegmentId,
    pub distal_segment: SegmentId,
    pub connecting_segment: SegmentId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppliedGraphEvent {
    Branch(BranchEvent),
    Fusion(FusionEvent),
    TipSegmentFusion(TipSegmentFusionEvent),
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
    RetiredSegmentIdentity(SegmentId),
    MissingNode(NodeId),
    MissingSegment(SegmentId),
    InvalidAttachmentFraction,
    NotActiveTip(NodeId),
    InactiveTargetSegment(SegmentId),
    InactiveParentSegment(SegmentId),
    SameFusionTip,
    EventIdConflict(EventId),
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePosition => write!(f, "node position must be finite"),
            Self::NonPositiveSegmentLength => {
                write!(f, "segment length must be finite and positive")
            }
            Self::InvalidDiameter => write!(f, "diameter must be finite and positive when present"),
            Self::ZeroLengthGeometry => write!(f, "event would create zero-length geometry"),
            Self::SegmentEndpointsEqual => write!(f, "segment endpoints must be distinct"),
            Self::DuplicateNode(id) => write!(f, "node {:?} already exists", id),
            Self::DuplicateSegment(id) => write!(f, "segment {:?} already exists", id),
            Self::RetiredSegmentIdentity(id) => {
                write!(f, "segment identity {:?} was previously retired", id)
            }
            Self::MissingNode(id) => write!(f, "node {:?} does not exist", id),
            Self::MissingSegment(id) => write!(f, "segment {:?} does not exist", id),
            Self::InvalidAttachmentFraction => {
                write!(f, "branch attachment fraction must be inside (0, 1)")
            }
            Self::NotActiveTip(id) => write!(f, "node {:?} is not an active tip", id),
            Self::InactiveTargetSegment(id) => {
                write!(f, "target segment {:?} is inactive", id)
            }
            Self::InactiveParentSegment(id) => {
                write!(f, "parent segment {:?} is inactive", id)
            }
            Self::SameFusionTip => write!(f, "fusion requires two distinct tips"),
            Self::EventIdConflict(id) => {
                write!(f, "event ID {:?} was reused with a different payload", id)
            }
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
    retired_segments: BTreeSet<SegmentId>,
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

    /// Segment IDs remain retired after removal; identity is never recycled.
    pub fn retired_segment_ids(&self) -> impl Iterator<Item = SegmentId> + '_ {
        self.retired_segments.iter().copied()
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
        self.segments
            .values()
            .map(|segment| segment.length_mm)
            .sum()
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
        if self.retired_segments.contains(&segment.id) {
            return Err(GraphError::RetiredSegmentIdentity(segment.id));
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
        if parent.state != HyphalSegmentState::Active {
            return Err(GraphError::InactiveParentSegment(event.parent_segment));
        }
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
        next.retired_segments.insert(event.parent_segment);
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
        next.applied_events
            .insert(event.id, AppliedGraphEvent::Branch(event));
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
        next.nodes
            .get_mut(&event.first_tip)
            .expect("validated first tip")
            .kind = HyphalNodeKind::Junction;
        next.nodes
            .get_mut(&event.second_tip)
            .expect("validated second tip")
            .kind = HyphalNodeKind::Junction;
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
        next.applied_events
            .insert(event.id, AppliedGraphEvent::Fusion(event));
        *self = next;
        Ok(ApplyOutcome::Applied)
    }

    /// Fuse one active tip to an interior point of an active segment.
    ///
    /// This is a topology transaction only. The caller supplies the attachment
    /// fraction after applying its own biological eligibility / stochastic
    /// fusion rule. The existing segment length is conserved across its split.
    pub fn apply_tip_segment_fusion(
        &mut self,
        event: TipSegmentFusionEvent,
    ) -> Result<ApplyOutcome, GraphError> {
        if let Some(applied) = self.applied_events.get(&event.id) {
            return if *applied == AppliedGraphEvent::TipSegmentFusion(event) {
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

        let tip = self
            .nodes
            .get(&event.tip)
            .cloned()
            .ok_or(GraphError::MissingNode(event.tip))?;
        if tip.kind != HyphalNodeKind::ActiveTip {
            return Err(GraphError::NotActiveTip(event.tip));
        }
        let target = self
            .segments
            .get(&event.target_segment)
            .cloned()
            .ok_or(GraphError::MissingSegment(event.target_segment))?;
        if target.state != HyphalSegmentState::Active {
            return Err(GraphError::InactiveTargetSegment(event.target_segment));
        }

        let from = self
            .nodes
            .get(&target.from)
            .ok_or(GraphError::MissingNode(target.from))?;
        let to = self
            .nodes
            .get(&target.to)
            .ok_or(GraphError::MissingNode(target.to))?;
        let fusion_position = from
            .position
            .interpolate(to.position, event.attachment_fraction);
        if !fusion_position.is_finite() {
            return Err(GraphError::NonFinitePosition);
        }
        let connecting_length = tip.position.distance_to(fusion_position);
        if !connecting_length.is_finite() || connecting_length <= 0.0 {
            return Err(GraphError::ZeroLengthGeometry);
        }

        let mut next = self.clone();
        next.segments.remove(&event.target_segment);
        next.retired_segments.insert(event.target_segment);
        next.nodes
            .get_mut(&event.tip)
            .expect("validated active tip")
            .kind = HyphalNodeKind::Junction;
        next.add_node(HyphalNode {
            id: event.fusion_node,
            kind: HyphalNodeKind::Fusion,
            position: fusion_position,
        })?;
        next.add_segment(HyphalSegment {
            id: event.proximal_segment,
            from: target.from,
            to: event.fusion_node,
            length_mm: target.length_mm * event.attachment_fraction,
            diameter_mm: target.diameter_mm,
            state: target.state,
        })?;
        next.add_segment(HyphalSegment {
            id: event.distal_segment,
            from: event.fusion_node,
            to: target.to,
            length_mm: target.length_mm * (1.0 - event.attachment_fraction),
            diameter_mm: target.diameter_mm,
            state: target.state,
        })?;
        next.add_segment(HyphalSegment {
            id: event.connecting_segment,
            from: event.tip,
            to: event.fusion_node,
            length_mm: connecting_length,
            diameter_mm: None,
            state: HyphalSegmentState::Active,
        })?;
        next.applied_events
            .insert(event.id, AppliedGraphEvent::TipSegmentFusion(event));
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

    /// Return deterministic active-segment connected components.
    ///
    /// Inactive segments remain part of structural history and are included by
    /// `connected_components`, but they do not establish active connectivity.
    pub fn active_connected_components(&self) -> Vec<Vec<NodeId>> {
        let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = self
            .nodes
            .keys()
            .copied()
            .map(|id| (id, Vec::new()))
            .collect();
        for segment in self.segments.values() {
            if segment.state != HyphalSegmentState::Active {
                continue;
            }
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

    /// Find active-segment bridges and articulation nodes with iterative DFS.
    ///
    /// Ordered map/set lookups add logarithmic overhead to the Tarjan traversal;
    /// measure scaling on representative large graphs before claiming a runtime
    /// bound. This is structural criticality only: it does not model edge
    /// capacity, measured cytoplasmic flow or resource delivery. Parallel
    /// segments remain distinct, so one of two parallel connections is not
    /// incorrectly classified as a bridge.
    pub fn active_graph_criticality(&self) -> ActiveGraphCriticality {
        let mut adjacency: BTreeMap<NodeId, Vec<(NodeId, SegmentId)>> = self
            .nodes
            .keys()
            .copied()
            .map(|node| (node, Vec::new()))
            .collect();
        for segment in self.segments.values() {
            if segment.state != HyphalSegmentState::Active {
                continue;
            }
            adjacency
                .get_mut(&segment.from)
                .expect("validated segment source exists")
                .push((segment.to, segment.id));
            adjacency
                .get_mut(&segment.to)
                .expect("validated segment target exists")
                .push((segment.from, segment.id));
        }

        let mut discovery = BTreeMap::<NodeId, usize>::new();
        let mut low = BTreeMap::<NodeId, usize>::new();
        let mut parent = BTreeMap::<NodeId, (NodeId, SegmentId)>::new();
        let mut child_count = BTreeMap::<NodeId, usize>::new();
        let mut bridges = BTreeSet::<SegmentId>::new();
        let mut articulation = BTreeSet::<NodeId>::new();
        let mut clock = 0_usize;

        // Iterative DFS avoids recursion-depth failures on long fungal chains.
        for root in self.nodes.keys().copied() {
            if discovery.contains_key(&root) {
                continue;
            }
            clock += 1;
            discovery.insert(root, clock);
            low.insert(root, clock);
            child_count.insert(root, 0);
            let mut stack = vec![(root, 0_usize)];

            while let Some((current, next_neighbor_index)) = stack.last().copied() {
                let neighbors = adjacency
                    .get(&current)
                    .expect("all graph nodes have adjacency entries");
                if next_neighbor_index < neighbors.len() {
                    stack.last_mut().expect("DFS stack is non-empty").1 += 1;
                    let (neighbor, edge_id) = neighbors[next_neighbor_index];

                    if parent
                        .get(&current)
                        .is_some_and(|(_, parent_edge)| *parent_edge == edge_id)
                    {
                        continue;
                    }

                    match discovery.entry(neighbor) {
                        std::collections::btree_map::Entry::Vacant(entry) => {
                            parent.insert(neighbor, (current, edge_id));
                            *child_count.entry(current).or_default() += 1;
                            clock += 1;
                            entry.insert(clock);
                            low.insert(neighbor, clock);
                            child_count.insert(neighbor, 0);
                            stack.push((neighbor, 0));
                        }
                        std::collections::btree_map::Entry::Occupied(entry) => {
                            let neighbor_discovery = *entry.get();
                            let current_low = low
                                .get_mut(&current)
                                .expect("visited node has low-link value");
                            *current_low = (*current_low).min(neighbor_discovery);
                        }
                    }
                } else {
                    stack.pop();
                    if let Some((parent_node, parent_edge)) = parent.get(&current).copied() {
                        let current_low =
                            *low.get(&current).expect("visited node has low-link value");
                        let parent_discovery = *discovery
                            .get(&parent_node)
                            .expect("parent node has discovery time");
                        let parent_low = low
                            .get_mut(&parent_node)
                            .expect("parent node has low-link value");
                        *parent_low = (*parent_low).min(current_low);

                        if current_low > parent_discovery {
                            bridges.insert(parent_edge);
                        }
                        if parent.contains_key(&parent_node) && current_low >= parent_discovery {
                            articulation.insert(parent_node);
                        }
                    } else if child_count.get(&current).copied().unwrap_or_default() > 1 {
                        articulation.insert(current);
                    }
                }
            }
        }

        ActiveGraphCriticality {
            bridge_segments: bridges.into_iter().collect(),
            articulation_nodes: articulation.into_iter().collect(),
        }
    }

    /// Whether an active-segment path connects two nodes.
    pub fn has_active_path(&self, from: NodeId, to: NodeId) -> bool {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return false;
        }
        self.active_connected_components().iter().any(|component| {
            component.binary_search(&from).is_ok() && component.binary_search(&to).is_ok()
        })
    }

    pub fn has_path(&self, from: NodeId, to: NodeId) -> bool {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return false;
        }
        self.connected_components().iter().any(|component| {
            component.binary_search(&from).is_ok() && component.binary_search(&to).is_ok()
        })
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
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 10.0, 0.0, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 10.0)).unwrap();
        graph
    }

    #[test]
    fn linear_graph_has_one_component_and_exact_length() {
        let graph = linear_graph();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.segment_count(), 1);
        assert_eq!(
            graph.connected_components(),
            vec![vec![NodeId(1), NodeId(2)]]
        );
        assert!(graph.has_path(NodeId(1), NodeId(2)));
        assert_eq!(graph.total_length_mm(), 10.0);
    }

    #[test]
    fn branch_splits_parent_and_preserves_declared_parent_length() {
        let mut graph = linear_graph();
        let outcome = graph
            .apply_branch(BranchEvent {
                id: EventId(10),
                parent_segment: SegmentId(1),
                attachment_fraction: 0.5,
                branch_node: NodeId(3),
                branch_tip: NodeId(4),
                proximal_segment: SegmentId(2),
                distal_segment: SegmentId(3),
                branch_segment: SegmentId(4),
                branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
            })
            .unwrap();

        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(graph.segment_count(), 3);
        assert_eq!(graph.total_length_mm(), 15.0);
        assert_eq!(graph.connected_components().len(), 1);
        let branch_degree = graph
            .segments()
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
        assert_eq!(
            graph.apply_branch(event).unwrap(),
            ApplyOutcome::AlreadyApplied
        );
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
    fn inactive_parent_rejects_branch_without_mutation() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 10.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(1),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 10.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Inactive,
            })
            .unwrap();
        let before = graph.clone();
        assert_eq!(
            graph.apply_branch(BranchEvent {
                id: EventId(42),
                parent_segment: SegmentId(1),
                attachment_fraction: 0.5,
                branch_node: NodeId(3),
                branch_tip: NodeId(4),
                proximal_segment: SegmentId(2),
                distal_segment: SegmentId(3),
                branch_segment: SegmentId(4),
                branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
            }),
            Err(GraphError::InactiveParentSegment(SegmentId(1)))
        );
        assert_eq!(graph, before);
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
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 5.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(3, HyphalNodeKind::Anchor, 0.0, 10.0, 0.0))
            .unwrap();
        graph
            .add_node(node(4, HyphalNodeKind::ActiveTip, 5.0, 10.0, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 5.0)).unwrap();
        graph.add_segment(segment(2, 3, 4, 5.0)).unwrap();
        assert_eq!(graph.connected_components().len(), 2);
        assert!(!graph.has_path(NodeId(1), NodeId(4)));
    }

    #[test]
    fn fusion_joins_components_and_deactivates_old_tips() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 5.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(3, HyphalNodeKind::Anchor, 10.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(4, HyphalNodeKind::ActiveTip, 5.0, 0.1, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 5.0)).unwrap();
        graph.add_segment(segment(2, 3, 4, 5.0)).unwrap();
        assert_eq!(graph.connected_components().len(), 2);

        let outcome = graph
            .apply_fusion(FusionEvent {
                id: EventId(20),
                first_tip: NodeId(2),
                second_tip: NodeId(4),
                fusion_node: NodeId(5),
                first_segment: SegmentId(3),
                second_segment: SegmentId(4),
            })
            .unwrap();

        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(graph.connected_components().len(), 1);
        assert!(graph.has_path(NodeId(1), NodeId(3)));
        assert_eq!(
            graph.node(NodeId(2)).unwrap().kind,
            HyphalNodeKind::Junction
        );
        assert_eq!(
            graph.node(NodeId(4)).unwrap().kind,
            HyphalNodeKind::Junction
        );
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
    fn tip_fuses_to_interior_segment_and_splits_target() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::Anchor, 10.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(3, HyphalNodeKind::ActiveTip, 5.0, 3.0, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 10.0)).unwrap();

        let outcome = graph
            .apply_tip_segment_fusion(TipSegmentFusionEvent {
                id: EventId(30),
                tip: NodeId(3),
                target_segment: SegmentId(1),
                attachment_fraction: 0.5,
                fusion_node: NodeId(4),
                proximal_segment: SegmentId(2),
                distal_segment: SegmentId(3),
                connecting_segment: SegmentId(4),
            })
            .unwrap();

        assert_eq!(outcome, ApplyOutcome::Applied);
        assert_eq!(graph.segment_count(), 3);
        assert_eq!(graph.total_length_mm(), 13.0);
        assert_eq!(graph.connected_components().len(), 1);
        assert_eq!(
            graph.node(NodeId(3)).unwrap().kind,
            HyphalNodeKind::Junction
        );
        assert_eq!(graph.node(NodeId(4)).unwrap().kind, HyphalNodeKind::Fusion);
        assert_eq!(graph.segment(SegmentId(2)).unwrap().length_mm, 5.0);
        assert_eq!(graph.segment(SegmentId(3)).unwrap().length_mm, 5.0);
        assert_eq!(graph.segment(SegmentId(4)).unwrap().diameter_mm, None);
    }

    #[test]
    fn tip_segment_fusion_replay_is_idempotent_and_conflicts_are_rejected() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::Anchor, 10.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(3, HyphalNodeKind::ActiveTip, 5.0, 3.0, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 10.0)).unwrap();
        let event = TipSegmentFusionEvent {
            id: EventId(31),
            tip: NodeId(3),
            target_segment: SegmentId(1),
            attachment_fraction: 0.5,
            fusion_node: NodeId(4),
            proximal_segment: SegmentId(2),
            distal_segment: SegmentId(3),
            connecting_segment: SegmentId(4),
        };

        assert_eq!(
            graph.apply_tip_segment_fusion(event).unwrap(),
            ApplyOutcome::Applied
        );
        let snapshot = graph.clone();
        assert_eq!(
            graph.apply_tip_segment_fusion(event).unwrap(),
            ApplyOutcome::AlreadyApplied
        );
        assert_eq!(graph, snapshot);

        let conflicting = TipSegmentFusionEvent {
            attachment_fraction: 0.4,
            ..event
        };
        assert_eq!(
            graph.apply_tip_segment_fusion(conflicting),
            Err(GraphError::EventIdConflict(EventId(31)))
        );
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn inactive_target_rejects_tip_segment_fusion_without_mutation() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::Anchor, 10.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(3, HyphalNodeKind::ActiveTip, 5.0, 3.0, 0.0))
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(1),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 10.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Inactive,
            })
            .unwrap();
        let snapshot = graph.clone();

        assert_eq!(
            graph.apply_tip_segment_fusion(TipSegmentFusionEvent {
                id: EventId(32),
                tip: NodeId(3),
                target_segment: SegmentId(1),
                attachment_fraction: 0.5,
                fusion_node: NodeId(4),
                proximal_segment: SegmentId(2),
                distal_segment: SegmentId(3),
                connecting_segment: SegmentId(4),
            }),
            Err(GraphError::InactiveTargetSegment(SegmentId(1)))
        );
        assert_eq!(graph, snapshot);
    }

    #[test]
    fn retired_segment_identity_cannot_be_reused_after_branch() {
        let mut graph = linear_graph();
        graph
            .apply_branch(BranchEvent {
                id: EventId(40),
                parent_segment: SegmentId(1),
                attachment_fraction: 0.5,
                branch_node: NodeId(3),
                branch_tip: NodeId(4),
                proximal_segment: SegmentId(2),
                distal_segment: SegmentId(3),
                branch_segment: SegmentId(4),
                branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
            })
            .unwrap();
        assert!(graph.retired_segment_ids().any(|id| id == SegmentId(1)));
        let before = graph.clone();
        assert_eq!(
            graph.add_segment(segment(1, 1, 3, 5.0)),
            Err(GraphError::RetiredSegmentIdentity(SegmentId(1)))
        );
        assert_eq!(graph, before);
    }

    #[test]
    fn branch_rejects_reusing_parent_segment_identity_atomically() {
        let mut graph = linear_graph();
        let before = graph.clone();
        let result = graph.apply_branch(BranchEvent {
            id: EventId(41),
            parent_segment: SegmentId(1),
            attachment_fraction: 0.5,
            branch_node: NodeId(3),
            branch_tip: NodeId(4),
            proximal_segment: SegmentId(1),
            distal_segment: SegmentId(3),
            branch_segment: SegmentId(4),
            branch_tip_position: Position3Mm::new(5.0, 5.0, 0.0),
        });
        assert_eq!(
            result,
            Err(GraphError::RetiredSegmentIdentity(SegmentId(1)))
        );
        assert_eq!(graph, before);
    }

    #[test]
    fn inactive_segments_are_not_active_paths() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 5.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(1),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 5.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Inactive,
            })
            .unwrap();
        assert!(graph.has_path(NodeId(1), NodeId(2)));
        assert!(!graph.has_active_path(NodeId(1), NodeId(2)));
    }

    fn oracle_active_component_count(
        node_count: usize,
        edges: &[(u64, usize, usize)],
        removed_edge: Option<u64>,
        removed_node: Option<usize>,
    ) -> usize {
        let mut adjacency = vec![Vec::<usize>::new(); node_count];
        for (edge_id, from, to) in edges {
            if Some(*edge_id) == removed_edge
                || Some(*from) == removed_node
                || Some(*to) == removed_node
            {
                continue;
            }
            adjacency[*from].push(*to);
            adjacency[*to].push(*from);
        }

        let mut visited = vec![false; node_count];
        let mut components = 0;
        for start in 0..node_count {
            if Some(start) == removed_node || visited[start] {
                continue;
            }
            components += 1;
            visited[start] = true;
            let mut queue = VecDeque::from([start]);
            while let Some(current) = queue.pop_front() {
                for neighbor in &adjacency[current] {
                    if !visited[*neighbor] {
                        visited[*neighbor] = true;
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
        components
    }

    #[test]
    fn active_graph_criticality_matches_removal_oracle_for_all_small_simple_graphs() {
        // Exhaust every simple undirected graph through five vertices.
        // Bridges and articulation nodes are independently defined by whether
        // removing that identity increases the number of connected components.
        for node_count in 1..=5 {
            let mut possible_edges = Vec::new();
            for from in 0..node_count {
                for to in (from + 1)..node_count {
                    possible_edges.push((possible_edges.len() as u64 + 1, from, to));
                }
            }
            let graph_count = 1_usize << possible_edges.len();

            for mask in 0..graph_count {
                let selected: Vec<_> = possible_edges
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| (mask >> index) & 1 == 1)
                    .map(|(_, edge)| *edge)
                    .collect();

                let mut graph = HyphalGraph::new();
                for index in 0..node_count {
                    let node_id = index as u64 + 1;
                    graph
                        .add_node(node(
                            node_id,
                            HyphalNodeKind::Anchor,
                            index as f64,
                            0.0,
                            0.0,
                        ))
                        .unwrap();
                }
                for (edge_id, from, to) in &selected {
                    graph
                        .add_segment(segment(*edge_id, *from as u64 + 1, *to as u64 + 1, 1.0))
                        .unwrap();
                }

                let baseline = oracle_active_component_count(node_count, &selected, None, None);
                let expected_bridges: Vec<_> = selected
                    .iter()
                    .filter(|(edge_id, _, _)| {
                        oracle_active_component_count(node_count, &selected, Some(*edge_id), None)
                            > baseline
                    })
                    .map(|(edge_id, _, _)| SegmentId(*edge_id))
                    .collect();
                let expected_articulation: Vec<_> = (0..node_count)
                    .filter(|node_index| {
                        oracle_active_component_count(
                            node_count,
                            &selected,
                            None,
                            Some(*node_index),
                        ) > baseline
                    })
                    .map(|node_index| NodeId(node_index as u64 + 1))
                    .collect();

                assert_eq!(
                    graph.active_graph_criticality().bridge_segments,
                    expected_bridges,
                    "bridge mismatch for node_count={node_count}, mask={mask}"
                );
                assert_eq!(
                    graph.active_graph_criticality().articulation_nodes,
                    expected_articulation,
                    "articulation mismatch for node_count={node_count}, mask={mask}"
                );
            }
        }
    }

    #[test]
    fn active_graph_criticality_finds_chain_bridges_and_articulation() {
        let mut graph = HyphalGraph::new();
        for id in 1..=3 {
            graph
                .add_node(node(id, HyphalNodeKind::Anchor, id as f64, 0.0, 0.0))
                .unwrap();
        }
        graph.add_segment(segment(1, 1, 2, 1.0)).unwrap();
        graph.add_segment(segment(2, 2, 3, 1.0)).unwrap();

        let criticality = graph.active_graph_criticality();
        assert_eq!(
            criticality.bridge_segments,
            vec![SegmentId(1), SegmentId(2)]
        );
        assert_eq!(criticality.articulation_nodes, vec![NodeId(2)]);
    }

    #[test]
    fn active_graph_criticality_finds_no_cut_edges_in_cycle() {
        let mut graph = HyphalGraph::new();
        for (id, x, y) in [(1, 0.0, 0.0), (2, 1.0, 0.0), (3, 0.5, 1.0)] {
            graph
                .add_node(node(id, HyphalNodeKind::Anchor, x, y, 0.0))
                .unwrap();
        }
        graph.add_segment(segment(1, 1, 2, 1.0)).unwrap();
        graph.add_segment(segment(2, 2, 3, 1.0)).unwrap();
        graph.add_segment(segment(3, 3, 1, 1.0)).unwrap();

        let criticality = graph.active_graph_criticality();
        assert!(criticality.bridge_segments.is_empty());
        assert!(criticality.articulation_nodes.is_empty());
    }

    #[test]
    fn parallel_active_segments_are_not_individually_bridges() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 1.0, 0.0, 0.0))
            .unwrap();
        graph.add_segment(segment(1, 1, 2, 1.0)).unwrap();
        graph.add_segment(segment(2, 1, 2, 1.0)).unwrap();

        let criticality = graph.active_graph_criticality();
        assert!(criticality.bridge_segments.is_empty());
        assert!(criticality.articulation_nodes.is_empty());
    }

    #[test]
    fn active_graph_criticality_handles_a_long_chain_iteratively() {
        let mut graph = HyphalGraph::new();
        const NODE_COUNT: u64 = 4_096;
        for id in 1..=NODE_COUNT {
            graph
                .add_node(node(id, HyphalNodeKind::Anchor, id as f64, 0.0, 0.0))
                .unwrap();
        }
        for id in 1..NODE_COUNT {
            graph.add_segment(segment(id, id, id + 1, 1.0)).unwrap();
        }

        let criticality = graph.active_graph_criticality();
        assert_eq!(criticality.bridge_segments.len(), (NODE_COUNT - 1) as usize);
        assert_eq!(
            criticality.articulation_nodes.len(),
            (NODE_COUNT - 2) as usize
        );
    }

    #[test]
    fn active_graph_criticality_is_independent_of_insertion_order() {
        let build = |reverse: bool| {
            let mut graph = HyphalGraph::new();
            let mut nodes = vec![
                node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0),
                node(2, HyphalNodeKind::Anchor, 1.0, 0.0, 0.0),
                node(3, HyphalNodeKind::Anchor, 2.0, 0.0, 0.0),
                node(4, HyphalNodeKind::Anchor, 3.0, 0.0, 0.0),
            ];
            let mut segments = vec![
                segment(1, 1, 2, 1.0),
                segment(2, 2, 3, 1.0),
                segment(3, 3, 1, 1.0),
                segment(4, 3, 4, 1.0),
            ];
            if reverse {
                nodes.reverse();
                segments.reverse();
            }
            for item in nodes {
                graph.add_node(item).unwrap();
            }
            for item in segments {
                graph.add_segment(item).unwrap();
            }
            graph
        };

        assert_eq!(
            build(false).active_graph_criticality(),
            build(true).active_graph_criticality()
        );
    }

    #[test]
    fn inactive_segments_do_not_create_active_bridges() {
        let mut graph = HyphalGraph::new();
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 1.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_segment(HyphalSegment {
                id: SegmentId(1),
                from: NodeId(1),
                to: NodeId(2),
                length_mm: 1.0,
                diameter_mm: Some(0.02),
                state: HyphalSegmentState::Inactive,
            })
            .unwrap();

        let criticality = graph.active_graph_criticality();
        assert!(criticality.bridge_segments.is_empty());
        assert!(criticality.articulation_nodes.is_empty());
    }

    #[test]
    fn rejects_non_finite_and_zero_length_geometry() {
        let mut graph = HyphalGraph::new();
        assert_eq!(
            graph.add_node(node(1, HyphalNodeKind::Anchor, f64::NAN, 0.0, 0.0)),
            Err(GraphError::NonFinitePosition)
        );
        graph
            .add_node(node(1, HyphalNodeKind::Anchor, 0.0, 0.0, 0.0))
            .unwrap();
        graph
            .add_node(node(2, HyphalNodeKind::ActiveTip, 0.0, 0.0, 0.0))
            .unwrap();
        assert_eq!(
            graph.add_segment(segment(1, 1, 2, f64::INFINITY)),
            Err(GraphError::NonPositiveSegmentLength)
        );
    }
}
