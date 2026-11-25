// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Conservative reference checker for RRVR temporal closure.
//!
//! This crate does not query, fence, cancel, drain, or actuate hardware. It
//! evaluates a supplied graph/evidence envelope and returns Closed only when
//! either the fenced-and-drained proof path or the fully bounded-expiry path
//! is structurally complete. A closure result is not physical NoEffect and is
//! not recovery authority.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const CONTRACT_VERSION: &str = "0.1";
pub const MAX_NODES: usize = 64;
pub const MAX_EDGES: usize = 256;

type EdgePathKey<'a> = (&'a str, &'a str);
type EdgeIndex<'a> = BTreeMap<EdgePathKey<'a>, usize>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeEvidence {
    pub node_id: String,
    pub is_ingress: bool,
    pub is_terminal: bool,
    /// This node rejects commands carrying the old authority epoch.
    pub rejects_old_epoch: bool,
    /// Terminal nodes must enforce this at the actual actuation transition.
    pub endpoint_fence_enforced: bool,
    /// Fenced-and-drained evidence for this node.
    pub sequence_generation_id: String,
    pub accepted_sequence_generation_id: String,
    pub retired_sequence_generation_id: String,
    pub last_accepted_sequence: u64,
    pub last_retired_sequence: u64,
    pub outstanding_commands: u32,
    pub retention_bound_ms: Option<u64>,
    /// Expiry must be enforced when the message is received at this node.
    pub receiver_enforces_expiry: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EdgeEvidence {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub in_flight_messages: u32,
    /// Proves that this edge cannot enqueue a new old-epoch message after
    /// the source's fence/drain receipt.
    pub no_future_old_epoch_enqueue: bool,
    pub transit_bound_ms: Option<u64>,
    /// Forwarding preserves original command identity and age; it cannot
    /// renew expiry by republishing.
    pub preserves_original_age: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalClosureEnvelope {
    pub profile_id: String,
    pub graph_revision: String,
    pub old_authority_epoch: u64,
    pub current_authority_epoch: u64,
    pub nodes: Vec<NodeEvidence>,
    pub edges: Vec<EdgeEvidence>,
    pub inventory_complete: bool,
    pub clock_profile_valid: bool,
    /// Conservative path-wide aggregate of all source/receiver clock uncertainty.
    pub clock_uncertainty_bound_ms: Option<u64>,
    /// Conservative path-wide scheduling/endpoint execution allowance.
    pub scheduling_jitter_bound_ms: Option<u64>,
    /// Monotonic elapsed time after the ingress fence, never wall-clock time.
    /// Used only by bounded expiry, never by fenced-and-drained.
    pub elapsed_since_ingress_fence_ms: u64,
    /// A late command, profile change, clock invalidation, epoch rollback,
    /// or other revocation invalidates an earlier closure candidate.
    pub invalidated: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProofPath {
    FencedAndDrained,
    BoundedExpiry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    MissingProfileIdentity,
    EmptyGraph,
    TooManyNodes,
    TooManyEdges,
    IncompleteInventory,
    InvalidEpochTransition,
    InvalidatedEvidence,
    DuplicateNode,
    DuplicateEdge,
    DuplicateEdgePath,
    MissingIngress,
    MultipleIngresses,
    MissingTerminal,
    UnknownEdgeEndpoint,
    SelfLoop,
    Cycle,
    UnreachableNode,
    TerminalMismatch,
    MissingOldEpochFence,
    EndpointFenceMissing,
    UnaccountedNodeSequence,
    SequenceGenerationMismatch,
    OutstandingNodeCommands,
    InFlightEdgeMessages,
    FutureEnqueuePossible,
    UnboundedTransit,
    UnboundedRetention,
    OriginalAgeNotPreserved,
    ReceiverExpiryNotEnforced,
    InvalidClockProfile,
    MissingClockUncertainty,
    MissingSchedulingBound,
    HorizonOverflow,
    HorizonNotElapsed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClosureDecision {
    Closed {
        path: ProofPath,
        horizon_ms: Option<u64>,
        qualified_nodes: usize,
        qualified_edges: usize,
    },
    Hold(HoldReason),
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn graph_indexes(
    envelope: &TemporalClosureEnvelope,
) -> Result<(usize, Vec<Vec<usize>>), HoldReason> {
    if !nonempty(&envelope.profile_id) || !nonempty(&envelope.graph_revision) {
        return Err(HoldReason::MissingProfileIdentity);
    }
    if envelope.nodes.is_empty() {
        return Err(HoldReason::EmptyGraph);
    }
    if envelope.nodes.len() > MAX_NODES {
        return Err(HoldReason::TooManyNodes);
    }
    if envelope.edges.len() > MAX_EDGES {
        return Err(HoldReason::TooManyEdges);
    }
    if !envelope.inventory_complete {
        return Err(HoldReason::IncompleteInventory);
    }
    if envelope.current_authority_epoch <= envelope.old_authority_epoch {
        return Err(HoldReason::InvalidEpochTransition);
    }
    if envelope.invalidated {
        return Err(HoldReason::InvalidatedEvidence);
    }

    let mut ids = BTreeMap::new();
    for (index, node) in envelope.nodes.iter().enumerate() {
        if !nonempty(&node.node_id) || ids.insert(node.node_id.clone(), index).is_some() {
            return Err(HoldReason::DuplicateNode);
        }
    }

    let ingress: Vec<usize> = envelope
        .nodes
        .iter()
        .enumerate()
        .filter_map(|(i, node)| node.is_ingress.then_some(i))
        .collect();
    if ingress.is_empty() {
        return Err(HoldReason::MissingIngress);
    }
    if ingress.len() != 1 {
        return Err(HoldReason::MultipleIngresses);
    }
    if !envelope.nodes.iter().any(|node| node.is_terminal) {
        return Err(HoldReason::MissingTerminal);
    }

    let mut edge_ids = BTreeSet::new();
    let mut edge_paths = BTreeSet::new();
    let mut adjacency = vec![Vec::new(); envelope.nodes.len()];
    let mut indegree = vec![0_usize; envelope.nodes.len()];
    let mut outdegree = vec![0_usize; envelope.nodes.len()];
    for edge in &envelope.edges {
        if !nonempty(&edge.edge_id) || !edge_ids.insert(edge.edge_id.clone()) {
            return Err(HoldReason::DuplicateEdge);
        }
        if !edge_paths.insert((edge.from_node_id.clone(), edge.to_node_id.clone())) {
            return Err(HoldReason::DuplicateEdgePath);
        }
        let Some(&from) = ids.get(&edge.from_node_id) else {
            return Err(HoldReason::UnknownEdgeEndpoint);
        };
        let Some(&to) = ids.get(&edge.to_node_id) else {
            return Err(HoldReason::UnknownEdgeEndpoint);
        };
        if from == to {
            return Err(HoldReason::SelfLoop);
        }
        adjacency[from].push(to);
        indegree[to] += 1;
        outdegree[from] += 1;
    }

    // Kahn's algorithm rejects forwarding cycles. A cyclic profile needs a
    // separate termination/fencing proof and is not silently treated as a DAG.
    let mut queue = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            queue.push_back(index);
        }
    }
    let mut processed = 0_usize;
    let mut remaining = indegree.clone();
    while let Some(node) = queue.pop_front() {
        processed += 1;
        for &next in &adjacency[node] {
            remaining[next] -= 1;
            if remaining[next] == 0 {
                queue.push_back(next);
            }
        }
    }
    if processed != envelope.nodes.len() {
        return Err(HoldReason::Cycle);
    }

    for (index, node) in envelope.nodes.iter().enumerate() {
        if node.is_ingress && indegree[index] != 0 {
            return Err(HoldReason::TerminalMismatch);
        }
        if node.is_terminal && outdegree[index] != 0 {
            return Err(HoldReason::TerminalMismatch);
        }
        if !node.is_terminal && outdegree[index] == 0 {
            return Err(HoldReason::TerminalMismatch);
        }
    }

    // Every declared node must be reachable from the single ingress.
    let mut seen = BTreeSet::new();
    let mut pending = vec![ingress[0]];
    while let Some(node) = pending.pop() {
        if seen.insert(node) {
            pending.extend(adjacency[node].iter().copied());
        }
    }
    if seen.len() != envelope.nodes.len() {
        return Err(HoldReason::UnreachableNode);
    }

    Ok((ingress[0], adjacency))
}

fn fenced_and_drained(envelope: &TemporalClosureEnvelope) -> Result<(), HoldReason> {
    for node in &envelope.nodes {
        if !node.rejects_old_epoch {
            return Err(HoldReason::MissingOldEpochFence);
        }
        if node.is_terminal && !node.endpoint_fence_enforced {
            return Err(HoldReason::EndpointFenceMissing);
        }
        if !nonempty(&node.sequence_generation_id)
            || node.accepted_sequence_generation_id != node.sequence_generation_id
            || node.retired_sequence_generation_id != node.sequence_generation_id
        {
            return Err(HoldReason::SequenceGenerationMismatch);
        }
        if node.last_accepted_sequence != node.last_retired_sequence {
            return Err(HoldReason::UnaccountedNodeSequence);
        }
        if node.outstanding_commands != 0 {
            return Err(HoldReason::OutstandingNodeCommands);
        }
    }
    for edge in &envelope.edges {
        if edge.in_flight_messages != 0 {
            return Err(HoldReason::InFlightEdgeMessages);
        }
        if !edge.no_future_old_epoch_enqueue {
            return Err(HoldReason::FutureEnqueuePossible);
        }
    }
    Ok(())
}

fn checked_add(total: u64, value: u64) -> Result<u64, HoldReason> {
    total.checked_add(value).ok_or(HoldReason::HorizonOverflow)
}

fn bounded_expiry_horizon(
    envelope: &TemporalClosureEnvelope,
    ingress_index: usize,
    adjacency: &[Vec<usize>],
) -> Result<u64, HoldReason> {
    if envelope.nodes.iter().any(|node| !node.rejects_old_epoch) {
        return Err(HoldReason::MissingOldEpochFence);
    }
    if !envelope.clock_profile_valid {
        return Err(HoldReason::InvalidClockProfile);
    }
    let clock_uncertainty = envelope
        .clock_uncertainty_bound_ms
        .ok_or(HoldReason::MissingClockUncertainty)?;
    let scheduling_jitter = envelope
        .scheduling_jitter_bound_ms
        .ok_or(HoldReason::MissingSchedulingBound)?;

    let edge_index: EdgeIndex<'_> = envelope
        .edges
        .iter()
        .enumerate()
        .map(|(index, edge)| ((edge.from_node_id.as_str(), edge.to_node_id.as_str()), index))
        .collect();

    fn longest_path(
        index: usize,
        envelope: &TemporalClosureEnvelope,
        adjacency: &[Vec<usize>],
        edge_index: &EdgeIndex<'_>,
        memo: &mut [Option<u64>],
    ) -> Result<u64, HoldReason> {
        if let Some(value) = memo[index] {
            return Ok(value);
        }
        let node = &envelope.nodes[index];
        if !node.receiver_enforces_expiry {
            return Err(HoldReason::ReceiverExpiryNotEnforced);
        }
        let retention = node
            .retention_bound_ms
            .ok_or(HoldReason::UnboundedRetention)?;
        let mut longest_child = 0_u64;
        for &next in &adjacency[index] {
            let next_id = envelope.nodes[next].node_id.as_str();
            let edge = edge_index
                .get(&(node.node_id.as_str(), next_id))
                .map(|edge_position| &envelope.edges[*edge_position])
                .ok_or(HoldReason::UnknownEdgeEndpoint)?;
            if !edge.preserves_original_age {
                return Err(HoldReason::OriginalAgeNotPreserved);
            }
            let transit = edge.transit_bound_ms.ok_or(HoldReason::UnboundedTransit)?;
            let child = longest_path(next, envelope, adjacency, edge_index, memo)?;
            longest_child = longest_child.max(checked_add(transit, child)?);
        }
        let value = checked_add(retention, longest_child)?;
        memo[index] = Some(value);
        Ok(value)
    }

    let mut memo = vec![None; envelope.nodes.len()];
    let path_horizon = longest_path(
        ingress_index,
        envelope,
        adjacency,
        &edge_index,
        &mut memo,
    )?;
    checked_add(checked_add(path_horizon, clock_uncertainty)?, scheduling_jitter)
}

pub fn evaluate(envelope: &TemporalClosureEnvelope) -> ClosureDecision {
    let (ingress_index, adjacency) = match graph_indexes(envelope) {
        Ok(value) => value,
        Err(reason) => return ClosureDecision::Hold(reason),
    };

    if fenced_and_drained(envelope).is_ok() {
        return ClosureDecision::Closed {
            path: ProofPath::FencedAndDrained,
            horizon_ms: None,
            qualified_nodes: envelope.nodes.len(),
            qualified_edges: envelope.edges.len(),
        };
    }

    match bounded_expiry_horizon(envelope, ingress_index, &adjacency) {
        Ok(horizon) if envelope.elapsed_since_ingress_fence_ms >= horizon => {
            ClosureDecision::Closed {
                path: ProofPath::BoundedExpiry,
                horizon_ms: Some(horizon),
                qualified_nodes: envelope.nodes.len(),
                qualified_edges: envelope.edges.len(),
            }
        }
        Ok(_) => ClosureDecision::Hold(HoldReason::HorizonNotElapsed),
        Err(reason) => ClosureDecision::Hold(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_envelope() -> TemporalClosureEnvelope {
        let nodes = vec![
            NodeEvidence {
                node_id: "controller".into(),
                is_ingress: true,
                is_terminal: false,
                rejects_old_epoch: true,
                endpoint_fence_enforced: false,
                sequence_generation_id: "generation-1".into(),
                accepted_sequence_generation_id: "generation-1".into(),
                retired_sequence_generation_id: "generation-1".into(),
                last_accepted_sequence: 12,
                last_retired_sequence: 12,
                outstanding_commands: 0,
                retention_bound_ms: Some(2),
                receiver_enforces_expiry: true,
            },
            NodeEvidence {
                node_id: "driver".into(),
                is_ingress: false,
                is_terminal: false,
                rejects_old_epoch: true,
                endpoint_fence_enforced: false,
                sequence_generation_id: "generation-1".into(),
                accepted_sequence_generation_id: "generation-1".into(),
                retired_sequence_generation_id: "generation-1".into(),
                last_accepted_sequence: 12,
                last_retired_sequence: 12,
                outstanding_commands: 0,
                retention_bound_ms: Some(3),
                receiver_enforces_expiry: true,
            },
            NodeEvidence {
                node_id: "actuator".into(),
                is_ingress: false,
                is_terminal: true,
                rejects_old_epoch: true,
                endpoint_fence_enforced: true,
                sequence_generation_id: "generation-1".into(),
                accepted_sequence_generation_id: "generation-1".into(),
                retired_sequence_generation_id: "generation-1".into(),
                last_accepted_sequence: 12,
                last_retired_sequence: 12,
                outstanding_commands: 0,
                retention_bound_ms: Some(0),
                receiver_enforces_expiry: true,
            },
        ];
        let edges = vec![
            EdgeEvidence {
                edge_id: "controller-driver".into(),
                from_node_id: "controller".into(),
                to_node_id: "driver".into(),
                in_flight_messages: 0,
                no_future_old_epoch_enqueue: true,
                transit_bound_ms: Some(5),
                preserves_original_age: true,
            },
            EdgeEvidence {
                edge_id: "driver-actuator".into(),
                from_node_id: "driver".into(),
                to_node_id: "actuator".into(),
                in_flight_messages: 0,
                no_future_old_epoch_enqueue: true,
                transit_bound_ms: Some(7),
                preserves_original_age: true,
            },
        ];
        TemporalClosureEnvelope {
            profile_id: "test-profile".into(),
            graph_revision: "graph-v1".into(),
            old_authority_epoch: 4,
            current_authority_epoch: 5,
            nodes,
            edges,
            inventory_complete: true,
            clock_profile_valid: true,
            clock_uncertainty_bound_ms: Some(4),
            scheduling_jitter_bound_ms: Some(2),
            elapsed_since_ingress_fence_ms: 23,
            invalidated: false,
        }
    }

    #[test]
    fn fenced_and_drained_closes_without_waiting_for_horizon() {
        let mut envelope = base_envelope();
        envelope.elapsed_since_ingress_fence_ms = 0;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Closed {
                path: ProofPath::FencedAndDrained,
                horizon_ms: None,
                qualified_nodes: 3,
                qualified_edges: 2,
            }
        );
    }

    #[test]
    fn bounded_expiry_requires_complete_finite_path_and_elapsed_horizon() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        let decision = evaluate(&envelope);
        assert_eq!(
            decision,
            ClosureDecision::Closed {
                path: ProofPath::BoundedExpiry,
                horizon_ms: Some(23),
                qualified_nodes: 3,
                qualified_edges: 2,
            }
        );
    }

    #[test]
    fn unbounded_edge_never_closes_by_waiting() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.edges[0].transit_bound_ms = None;
        envelope.elapsed_since_ingress_fence_ms = u64::MAX;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::UnboundedTransit)
        );
    }

    #[test]
    fn quiet_time_does_not_replace_receiver_enforced_expiry() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.nodes[1].receiver_enforces_expiry = false;
        envelope.elapsed_since_ingress_fence_ms = u64::MAX;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::ReceiverExpiryNotEnforced)
        );
    }

    #[test]
    fn forwarding_must_preserve_original_message_age() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.edges[0].preserves_original_age = false;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::OriginalAgeNotPreserved)
        );
    }

    #[test]
    fn sequence_gap_prevents_fenced_and_drained_closure() {
        let mut envelope = base_envelope();
        envelope.nodes[1].last_retired_sequence = 11;
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Closed {
                path: ProofPath::BoundedExpiry,
                horizon_ms: Some(23),
                qualified_nodes: 3,
                qualified_edges: 2,
            }
        );
        envelope.edges[0].transit_bound_ms = None;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::UnboundedTransit)
        );
    }

    #[test]
    fn bounded_expiry_holds_until_the_full_path_horizon_elapses() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.elapsed_since_ingress_fence_ms = 22;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::HorizonNotElapsed)
        );
    }

    #[test]
    fn arithmetic_overflow_cannot_produce_a_finite_closure_horizon() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.clock_uncertainty_bound_ms = Some(u64::MAX);
        envelope.scheduling_jitter_bound_ms = Some(1);
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::HorizonOverflow)
        );
    }

    #[test]
    fn invalid_clock_profile_cannot_support_expiry() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.clock_profile_valid = false;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::InvalidClockProfile)
        );
    }

    #[test]
    fn incomplete_inventory_and_invalidated_evidence_hold() {
        let mut envelope = base_envelope();
        envelope.inventory_complete = false;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::IncompleteInventory)
        );
        envelope.inventory_complete = true;
        envelope.invalidated = true;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::InvalidatedEvidence)
        );
    }

    #[test]
    fn cycle_is_not_treated_as_a_finite_path() {
        let mut envelope = base_envelope();
        envelope.edges.push(EdgeEvidence {
            edge_id: "actuator-controller".into(),
            from_node_id: "actuator".into(),
            to_node_id: "controller".into(),
            in_flight_messages: 0,
            no_future_old_epoch_enqueue: true,
            transit_bound_ms: Some(1),
            preserves_original_age: true,
        });
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::Cycle)
        );
    }

    #[test]
    fn bounded_expiry_requires_old_epoch_rejection_at_every_holder() {
        let mut envelope = base_envelope();
        envelope.edges[0].in_flight_messages = 1;
        envelope.edges[0].no_future_old_epoch_enqueue = false;
        envelope.nodes[1].rejects_old_epoch = false;
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::MissingOldEpochFence)
        );
    }

    #[test]
    fn accepted_and_retired_sequences_must_share_the_inventory_generation() {
        let mut envelope = base_envelope();
        envelope.nodes[1].retired_sequence_generation_id = "generation-0".into();
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::SequenceGenerationMismatch)
        );
    }

    #[test]
    fn duplicate_parallel_edge_identity_is_rejected() {
        let mut envelope = base_envelope();
        let mut duplicate = envelope.edges[0].clone();
        duplicate.edge_id = "controller-driver-duplicate".into();
        envelope.edges.push(duplicate);
        assert_eq!(
            evaluate(&envelope),
            ClosureDecision::Hold(HoldReason::DuplicateEdgePath)
        );
    }

    #[test]
    fn closure_does_not_expose_or_infer_no_effect() {
        let decision = evaluate(&base_envelope());
        assert!(matches!(
            decision,
            ClosureDecision::Closed {
                path: ProofPath::FencedAndDrained,
                ..
            }
        ));
    }
}
