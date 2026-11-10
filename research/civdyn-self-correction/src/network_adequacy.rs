// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic graph summaries for source-pinned CIVDYN network adequacy.
//!
//! Version 1 deliberately supports only undirected, unweighted, simple graphs.
//! It is a benchmark adapter, not a general-purpose graph library. Network
//! summaries are converted into an ordered multivariate statistic so that
//! equal degree sequences cannot conceal different topology.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use crate::joint_adequacy::JointComponentValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkAdequacyError {
    ZeroNodes,
    SelfLoop { node: usize },
    NodeOutOfRange { node: usize, node_count: usize },
    DuplicateUndirectedEdge { left: usize, right: usize },
    NoReachablePairsForPathStatistic,
}

impl fmt::Display for NetworkAdequacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroNodes => write!(formatter, "network must contain at least one node"),
            Self::SelfLoop { node } => write!(formatter, "self-loop at node {node} is not allowed"),
            Self::NodeOutOfRange { node, node_count } => write!(
                formatter,
                "node {node} is outside the valid range 0..{node_count}"
            ),
            Self::DuplicateUndirectedEdge { left, right } => {
                write!(formatter, "duplicate undirected edge ({left}, {right})")
            }
            Self::NoReachablePairsForPathStatistic => {
                write!(
                    formatter,
                    "mean finite shortest path is undefined without reachable pairs"
                )
            }
        }
    }
}

impl std::error::Error for NetworkAdequacyError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndirectedNetwork {
    node_count: usize,
    canonical_edges: Vec<(usize, usize)>,
    adjacency: Vec<BTreeSet<usize>>,
}

impl UndirectedNetwork {
    pub fn new(
        node_count: usize,
        edges: impl IntoIterator<Item = (usize, usize)>,
    ) -> Result<Self, NetworkAdequacyError> {
        if node_count == 0 {
            return Err(NetworkAdequacyError::ZeroNodes);
        }

        let mut canonical_edges = Vec::new();
        for (left, right) in edges {
            if left >= node_count {
                return Err(NetworkAdequacyError::NodeOutOfRange {
                    node: left,
                    node_count,
                });
            }
            if right >= node_count {
                return Err(NetworkAdequacyError::NodeOutOfRange {
                    node: right,
                    node_count,
                });
            }
            if left == right {
                return Err(NetworkAdequacyError::SelfLoop { node: left });
            }
            canonical_edges.push((left.min(right), left.max(right)));
        }

        canonical_edges.sort_unstable();
        for pair in canonical_edges.windows(2) {
            if pair[0] == pair[1] {
                return Err(NetworkAdequacyError::DuplicateUndirectedEdge {
                    left: pair[0].0,
                    right: pair[0].1,
                });
            }
        }

        let mut adjacency = vec![BTreeSet::new(); node_count];
        for &(left, right) in &canonical_edges {
            adjacency[left].insert(right);
            adjacency[right].insert(left);
        }

        Ok(Self {
            node_count,
            canonical_edges,
            adjacency,
        })
    }

    pub fn node_count(&self) -> usize {
        self.node_count
    }

    pub fn edge_count(&self) -> usize {
        self.canonical_edges.len()
    }

    /// Stable endpoint-ordered representation suitable for canonical hashing.
    pub fn canonical_edges(&self) -> &[(usize, usize)] {
        &self.canonical_edges
    }

    pub fn summarize(&self) -> NetworkSummary {
        let mut degree_sequence: Vec<usize> = self.adjacency.iter().map(BTreeSet::len).collect();
        degree_sequence.sort_unstable_by(|left, right| right.cmp(left));
        let mut degree_counts = BTreeMap::<usize, usize>::new();
        for &degree in &degree_sequence {
            *degree_counts.entry(degree).or_default() += 1;
        }
        let degree_histogram: Vec<(usize, usize)> = degree_counts.into_iter().collect();

        let mean_degree = degree_sequence.iter().sum::<usize>() as f64 / self.node_count as f64;
        let degree_variance = degree_sequence
            .iter()
            .map(|degree| {
                let delta = *degree as f64 - mean_degree;
                delta * delta
            })
            .sum::<f64>()
            / self.node_count as f64;

        let (component_sizes, reachable_unordered_pairs, distance_sum) =
            self.component_and_distance_summary();
        let component_count = component_sizes.len();
        let largest_component_share =
            component_sizes.iter().copied().max().unwrap_or(0) as f64 / self.node_count as f64;
        let mean_finite_shortest_path = (reachable_unordered_pairs > 0)
            .then(|| distance_sum as f64 / reachable_unordered_pairs as f64);

        let connected_triplets: u128 = self
            .adjacency
            .iter()
            .map(|neighbors| {
                let degree = neighbors.len() as u128;
                degree.saturating_mul(degree.saturating_sub(1)) / 2
            })
            .sum();
        let triangle_count = self.triangle_count();
        let global_transitivity = if connected_triplets == 0 {
            0.0
        } else {
            (3_u128.saturating_mul(triangle_count)) as f64 / connected_triplets as f64
        };

        NetworkSummary {
            node_count: self.node_count,
            edge_count: self.canonical_edges.len(),
            degree_sequence,
            degree_histogram,
            mean_degree,
            degree_variance,
            component_count,
            largest_component_share,
            global_transitivity,
            reachable_unordered_pairs,
            mean_finite_shortest_path,
        }
    }

    fn component_and_distance_summary(&self) -> (Vec<usize>, usize, u128) {
        let mut visited = vec![false; self.node_count];
        let mut component_sizes = Vec::new();

        for start in 0..self.node_count {
            if visited[start] {
                continue;
            }
            visited[start] = true;
            let mut queue = VecDeque::from([start]);
            let mut size = 0_usize;
            while let Some(node) = queue.pop_front() {
                size += 1;
                for &neighbor in &self.adjacency[node] {
                    if !visited[neighbor] {
                        visited[neighbor] = true;
                        queue.push_back(neighbor);
                    }
                }
            }
            component_sizes.push(size);
        }

        let mut reachable_pairs = 0_usize;
        let mut distance_sum = 0_u128;
        for source in 0..self.node_count {
            let mut distances = vec![usize::MAX; self.node_count];
            distances[source] = 0;
            let mut queue = VecDeque::from([source]);
            while let Some(node) = queue.pop_front() {
                let next_distance = distances[node] + 1;
                for &neighbor in &self.adjacency[node] {
                    if distances[neighbor] == usize::MAX {
                        distances[neighbor] = next_distance;
                        queue.push_back(neighbor);
                    }
                }
            }
            for distance in distances.iter().skip(source + 1) {
                if *distance != usize::MAX {
                    reachable_pairs = reachable_pairs.saturating_add(1);
                    distance_sum = distance_sum.saturating_add(*distance as u128);
                }
            }
        }

        (component_sizes, reachable_pairs, distance_sum)
    }

    fn triangle_count(&self) -> u128 {
        let mut triangles = 0_u128;
        for left in 0..self.node_count {
            for &middle in self.adjacency[left].range((left + 1)..) {
                let right_neighbors = &self.adjacency[middle];
                for &right in self.adjacency[left].range((middle + 1)..) {
                    if right_neighbors.contains(&right) {
                        triangles = triangles.saturating_add(1);
                    }
                }
            }
        }
        triangles
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NetworkSummary {
    pub node_count: usize,
    pub edge_count: usize,
    /// Degree sequence in descending order, independent of node-label ordering.
    pub degree_sequence: Vec<usize>,
    /// Sorted (degree, node_count) pairs.
    pub degree_histogram: Vec<(usize, usize)>,
    pub mean_degree: f64,
    /// Population variance of node degree.
    pub degree_variance: f64,
    pub component_count: usize,
    pub largest_component_share: f64,
    /// Three times triangle count divided by connected triplets; 0 when no
    /// connected triplets exist.
    pub global_transitivity: f64,
    /// Number of reachable unordered node pairs (unreachable pairs excluded).
    pub reachable_unordered_pairs: usize,
    /// Mean shortest-path distance over reachable unordered pairs only.
    /// None means there are no reachable distinct-node pairs.
    pub mean_finite_shortest_path: Option<f64>,
}

impl NetworkSummary {
    /// The ordered vector is a semantic contract shared by observations and
    /// predictive draws. Units and reference scales are declared separately
    /// in JointStatisticSpec.
    pub fn joint_component_values(&self) -> Result<Vec<JointComponentValue>, NetworkAdequacyError> {
        let mean_path = self
            .mean_finite_shortest_path
            .ok_or(NetworkAdequacyError::NoReachablePairsForPathStatistic)?;
        Ok(vec![
            JointComponentValue {
                component_id: "mean-degree".to_string(),
                measurement_unit: "edges_per_node".to_string(),
                value: self.mean_degree,
            },
            JointComponentValue {
                component_id: "degree-variance".to_string(),
                measurement_unit: "degree_squared".to_string(),
                value: self.degree_variance,
            },
            JointComponentValue {
                component_id: "global-transitivity".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.global_transitivity,
            },
            JointComponentValue {
                component_id: "component-count".to_string(),
                measurement_unit: "components".to_string(),
                value: self.component_count as f64,
            },
            JointComponentValue {
                component_id: "largest-component-share".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.largest_component_share,
            },
            JointComponentValue {
                component_id: "mean-finite-shortest-path".to_string(),
                measurement_unit: "hops".to_string(),
                value: mean_path,
            },
            JointComponentValue {
                component_id: "reachable-unordered-pairs".to_string(),
                measurement_unit: "pairs".to_string(),
                value: self.reachable_unordered_pairs as f64,
            },
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TemporalNetworkError {
    TooFewSnapshots,
    TickCountMismatch,
    TicksNotStrictlyIncreasing,
    SnapshotNodeCountMismatch {
        expected: usize,
        actual: usize,
    },
    NonFiniteTemporalSummary,
}

impl fmt::Display for TemporalNetworkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TemporalNetworkError {}

/// A time-ordered series of static network snapshots with stable node IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalNetworkSequence {
    ticks: Vec<u64>,
    snapshots: Vec<UndirectedNetwork>,
}

impl TemporalNetworkSequence {
    pub fn new(
        ticks: Vec<u64>,
        snapshots: Vec<UndirectedNetwork>,
    ) -> Result<Self, TemporalNetworkError> {
        if snapshots.len() < 2 {
            return Err(TemporalNetworkError::TooFewSnapshots);
        }
        if ticks.len() != snapshots.len() {
            return Err(TemporalNetworkError::TickCountMismatch);
        }
        if ticks.windows(2).any(|pair| pair[1] <= pair[0]) {
            return Err(TemporalNetworkError::TicksNotStrictlyIncreasing);
        }
        let expected_nodes = snapshots[0].node_count();
        if let Some(snapshot) = snapshots
            .iter()
            .find(|snapshot| snapshot.node_count() != expected_nodes)
        {
            return Err(TemporalNetworkError::SnapshotNodeCountMismatch {
                expected: expected_nodes,
                actual: snapshot.node_count(),
            });
        }
        Ok(Self { ticks, snapshots })
    }

    pub fn ticks(&self) -> &[u64] {
        &self.ticks
    }

    pub fn snapshots(&self) -> &[UndirectedNetwork] {
        &self.snapshots
    }

    pub fn summarize(&self) -> Result<TemporalNetworkSummary, TemporalNetworkError> {
        let snapshot_summaries: Vec<NetworkSummary> =
            self.snapshots.iter().map(UndirectedNetwork::summarize).collect();
        let component_counts: Vec<f64> = snapshot_summaries
            .iter()
            .map(|summary| summary.component_count as f64)
            .collect();
        let mean_degrees: Vec<f64> = snapshot_summaries
            .iter()
            .map(|summary| summary.mean_degree)
            .collect();
        let transitivity: Vec<f64> = snapshot_summaries
            .iter()
            .map(|summary| summary.global_transitivity)
            .collect();
        let x: Vec<f64> = self
            .ticks
            .iter()
            .map(|tick| (*tick - self.ticks[0]) as f64)
            .collect();

        let mean_component_count = arithmetic_mean(&component_counts);
        let mean_degree = arithmetic_mean(&mean_degrees);
        let mean_global_transitivity = arithmetic_mean(&transitivity);
        let component_count_trend_per_tick = linear_trend(&x, &component_counts);
        let mean_degree_trend_per_tick = linear_trend(&x, &mean_degrees);
        let transitivity_trend_per_tick = linear_trend(&x, &transitivity);

        let maximum_possible_edges = (self.snapshots[0].node_count() as f64)
            * (self.snapshots[0].node_count().saturating_sub(1) as f64)
            / 2.0;
        let mut edge_jaccards = Vec::with_capacity(self.snapshots.len() - 1);
        let mut edge_addition_rates = Vec::with_capacity(self.snapshots.len() - 1);
        let mut edge_removal_rates = Vec::with_capacity(self.snapshots.len() - 1);

        for pair in self.snapshots.windows(2) {
            let previous: BTreeSet<(usize, usize)> =
                pair[0].canonical_edges().iter().copied().collect();
            let current: BTreeSet<(usize, usize)> =
                pair[1].canonical_edges().iter().copied().collect();
            let intersection = previous.intersection(&current).count() as f64;
            let union = previous.union(&current).count() as f64;
            // Two empty edge sets are identical: Jaccard = 1, turnover = 0.
            let jaccard = if union == 0.0 { 1.0 } else { intersection / union };
            let additions = current.difference(&previous).count() as f64;
            let removals = previous.difference(&current).count() as f64;
            let denominator = maximum_possible_edges.max(1.0);
            edge_jaccards.push(jaccard);
            edge_addition_rates.push(additions / denominator);
            edge_removal_rates.push(removals / denominator);
        }

        let mean_adjacent_edge_jaccard = arithmetic_mean(&edge_jaccards);
        let mean_adjacent_edge_turnover = 1.0 - mean_adjacent_edge_jaccard;
        let mean_edge_addition_rate = arithmetic_mean(&edge_addition_rates);
        let mean_edge_removal_rate = arithmetic_mean(&edge_removal_rates);

        let values = [
            mean_component_count,
            component_count_trend_per_tick,
            mean_degree,
            mean_degree_trend_per_tick,
            mean_global_transitivity,
            transitivity_trend_per_tick,
            mean_adjacent_edge_jaccard,
            mean_adjacent_edge_turnover,
            mean_edge_addition_rate,
            mean_edge_removal_rate,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(TemporalNetworkError::NonFiniteTemporalSummary);
        }

        Ok(TemporalNetworkSummary {
            snapshot_count: self.snapshots.len(),
            node_count: self.snapshots[0].node_count(),
            start_tick: self.ticks[0],
            end_tick: *self.ticks.last().expect("at least two snapshots"),
            mean_component_count,
            component_count_trend_per_tick,
            mean_degree,
            mean_degree_trend_per_tick,
            mean_global_transitivity,
            transitivity_trend_per_tick,
            mean_adjacent_edge_jaccard,
            mean_adjacent_edge_turnover,
            mean_edge_addition_rate,
            mean_edge_removal_rate,
        })
    }
}

fn arithmetic_mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// Ordinary least-squares slope against actual tick offsets, not snapshot index.
fn linear_trend(x: &[f64], y: &[f64]) -> f64 {
    let mean_x = arithmetic_mean(x);
    let mean_y = arithmetic_mean(y);
    let covariance = x
        .iter()
        .zip(y)
        .map(|(x_value, y_value)| (x_value - mean_x) * (y_value - mean_y))
        .sum::<f64>();
    let variance = x
        .iter()
        .map(|x_value| (x_value - mean_x).powi(2))
        .sum::<f64>();
    covariance / variance
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporalNetworkSummary {
    pub snapshot_count: usize,
    pub node_count: usize,
    pub start_tick: u64,
    pub end_tick: u64,
    pub mean_component_count: f64,
    pub component_count_trend_per_tick: f64,
    pub mean_degree: f64,
    pub mean_degree_trend_per_tick: f64,
    pub mean_global_transitivity: f64,
    pub transitivity_trend_per_tick: f64,
    pub mean_adjacent_edge_jaccard: f64,
    pub mean_adjacent_edge_turnover: f64,
    /// Added edges per possible undirected dyad per adjacent transition.
    pub mean_edge_addition_rate: f64,
    /// Removed edges per possible undirected dyad per adjacent transition.
    pub mean_edge_removal_rate: f64,
}

impl TemporalNetworkSummary {
    /// Stable ten-component schema for joint energy/variogram scoring.
    pub fn joint_component_values(&self) -> Vec<JointComponentValue> {
        vec![
            JointComponentValue {
                component_id: "mean-component-count".to_string(),
                measurement_unit: "components".to_string(),
                value: self.mean_component_count,
            },
            JointComponentValue {
                component_id: "component-count-trend".to_string(),
                measurement_unit: "components_per_tick".to_string(),
                value: self.component_count_trend_per_tick,
            },
            JointComponentValue {
                component_id: "mean-degree".to_string(),
                measurement_unit: "edges_per_node".to_string(),
                value: self.mean_degree,
            },
            JointComponentValue {
                component_id: "mean-degree-trend".to_string(),
                measurement_unit: "edges_per_node_per_tick".to_string(),
                value: self.mean_degree_trend_per_tick,
            },
            JointComponentValue {
                component_id: "mean-transitivity".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.mean_global_transitivity,
            },
            JointComponentValue {
                component_id: "transitivity-trend".to_string(),
                measurement_unit: "ratio_per_tick".to_string(),
                value: self.transitivity_trend_per_tick,
            },
            JointComponentValue {
                component_id: "mean-edge-jaccard".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.mean_adjacent_edge_jaccard,
            },
            JointComponentValue {
                component_id: "mean-edge-turnover".to_string(),
                measurement_unit: "ratio".to_string(),
                value: self.mean_adjacent_edge_turnover,
            },
            JointComponentValue {
                component_id: "mean-edge-addition-rate".to_string(),
                measurement_unit: "dyad_fraction_per_transition".to_string(),
                value: self.mean_edge_addition_rate,
            },
            JointComponentValue {
                component_id: "mean-edge-removal-rate".to_string(),
                measurement_unit: "dyad_fraction_per_transition".to_string(),
                value: self.mean_edge_removal_rate,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joint_adequacy::{
        JointComponentSpec, JointMetricConfig, JointObservation, JointPredictiveDraw,
        JointStatisticSpec, assess_joint_predictive_draws,
    };
    use crate::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

    fn cycle_six() -> UndirectedNetwork {
        UndirectedNetwork::new(6, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)])
            .expect("valid C6")
    }

    fn two_triangles() -> UndirectedNetwork {
        UndirectedNetwork::new(6, [(0, 1), (1, 2), (2, 0), (3, 4), (4, 5), (5, 3)])
            .expect("valid 2K3")
    }

    #[test]
    fn same_degree_sequence_different_topology_has_exact_known_answers() {
        let cycle = cycle_six().summarize();
        let triangles = two_triangles().summarize();

        assert_eq!(cycle.degree_sequence, vec![2; 6]);
        assert_eq!(triangles.degree_sequence, vec![2; 6]);
        assert_eq!(cycle.degree_histogram, triangles.degree_histogram);
        assert_eq!(cycle.mean_degree, 2.0);
        assert_eq!(triangles.mean_degree, 2.0);
        assert_eq!(cycle.degree_variance, 0.0);
        assert_eq!(triangles.degree_variance, 0.0);

        assert_eq!(cycle.component_count, 1);
        assert_eq!(triangles.component_count, 2);
        assert_eq!(cycle.largest_component_share, 1.0);
        assert_eq!(triangles.largest_component_share, 0.5);
        assert_eq!(cycle.global_transitivity, 0.0);
        assert_eq!(triangles.global_transitivity, 1.0);
        assert_eq!(cycle.reachable_unordered_pairs, 15);
        assert_eq!(triangles.reachable_unordered_pairs, 6);
        assert!((cycle.mean_finite_shortest_path.expect("C6 has paths") - 1.8).abs() < 1e-12);
        assert!(
            (triangles
                .mean_finite_shortest_path
                .expect("triangles have paths")
                - 1.0)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn edge_order_and_endpoint_orientation_do_not_change_canonical_graph() {
        let first = UndirectedNetwork::new(6, [(0, 1), (1, 2), (2, 3), (3, 4), (4, 5), (5, 0)])
            .expect("valid graph");
        let reordered = UndirectedNetwork::new(6, [(5, 4), (3, 2), (1, 0), (0, 5), (4, 3), (2, 1)])
            .expect("same undirected edges");
        assert_eq!(first.canonical_edges(), reordered.canonical_edges());
        assert_eq!(first.summarize(), reordered.summarize());
    }

    #[test]
    fn rejects_zero_nodes_self_loops_out_of_range_and_duplicate_edges() {
        assert_eq!(
            UndirectedNetwork::new(0, []).unwrap_err(),
            NetworkAdequacyError::ZeroNodes
        );
        assert_eq!(
            UndirectedNetwork::new(3, [(1, 1)]).unwrap_err(),
            NetworkAdequacyError::SelfLoop { node: 1 }
        );
        assert_eq!(
            UndirectedNetwork::new(3, [(0, 3)]).unwrap_err(),
            NetworkAdequacyError::NodeOutOfRange {
                node: 3,
                node_count: 3
            }
        );
        assert_eq!(
            UndirectedNetwork::new(3, [(0, 1), (1, 0)]).unwrap_err(),
            NetworkAdequacyError::DuplicateUndirectedEdge { left: 0, right: 1 }
        );
    }

    #[test]
    fn isolated_nodes_and_disconnected_distance_denominator_are_explicit() {
        let graph = UndirectedNetwork::new(4, [(0, 1)]).expect("valid disconnected graph");
        let summary = graph.summarize();
        assert_eq!(summary.component_count, 3);
        assert_eq!(summary.largest_component_share, 0.5);
        assert_eq!(summary.reachable_unordered_pairs, 1);
        assert_eq!(summary.mean_finite_shortest_path, Some(1.0));
        assert!(summary.joint_component_values().is_ok());

        let isolates = UndirectedNetwork::new(3, []).expect("valid isolate-only graph");
        assert_eq!(isolates.summarize().reachable_unordered_pairs, 0);
        assert_eq!(
            isolates.summarize().joint_component_values().unwrap_err(),
            NetworkAdequacyError::NoReachablePairsForPathStatistic
        );
    }

    fn joint_schema() -> JointStatisticSpec {
        let components = [
            ("mean-degree", "edges_per_node", 2.0, 1.0),
            ("degree-variance", "degree_squared", 0.0, 1.0),
            ("global-transitivity", "ratio", 0.0, 1.0),
            ("component-count", "components", 1.0, 1.0),
            ("largest-component-share", "ratio", 1.0, 1.0),
            ("mean-finite-shortest-path", "hops", 1.8, 1.0),
            ("reachable-unordered-pairs", "pairs", 15.0, 15.0),
        ]
        .into_iter()
        .map(|(id, unit, center, scale)| JointComponentSpec {
            id: id.to_string(),
            measurement_unit: unit.to_string(),
            reference_center: center,
            reference_scale: scale,
        })
        .collect();

        JointStatisticSpec {
            id: "held-out-network-topology".to_string(),
            source_family: "synthetic-network-known-answer".to_string(),
            source_digest: "sha256-network-source".to_string(),
            population: "six-node-network-fixture".to_string(),
            context: "same-degree-opposite-topology".to_string(),
            independent_unit: IndependentUnitKind::Network,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: 20,
            components,
        }
    }

    fn joint_manifest() -> SplitManifest {
        SplitManifest {
            id: "network-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Network,
            calibration_data_digest: "sha256-network-calibration".to_string(),
            manifest_digest: "sha256-network-manifest".to_string(),
            calibration_units: vec!["network-calibration".to_string()],
            validation_units: vec!["network-held-out".to_string()],
            challenge_units: Vec::new(),
        }
    }

    fn assess_candidate(
        predicted: &UndirectedNetwork,
        model_digest: &str,
    ) -> crate::joint_adequacy::JointAdequacyResult {
        let schema = joint_schema();
        let manifest = joint_manifest();
        let observation = JointObservation {
            statistic_id: schema.id.clone(),
            independent_unit_id: "network-held-out".to_string(),
            sample_size: 6,
            source_digest: schema.source_digest.clone(),
            observation_digest: "sha256-network-observation".to_string(),
            split_manifest_digest: manifest.manifest_digest.clone(),
            components: cycle_six()
                .summarize()
                .joint_component_values()
                .expect("observed C6 has path pairs"),
        };
        let predicted_components = predicted
            .summarize()
            .joint_component_values()
            .expect("candidate graph has path pairs");
        let draws: Vec<JointPredictiveDraw> = (0..20)
            .map(|draw_index| JointPredictiveDraw {
                statistic_id: schema.id.clone(),
                independent_unit_id: "network-held-out".to_string(),
                sample_size: 6,
                draw_index,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-network-draws-{model_digest}"),
                calibration_data_digest: Some(manifest.calibration_data_digest.clone()),
                split_manifest_digest: manifest.manifest_digest.clone(),
                components: predicted_components.clone(),
            })
            .collect();
        assess_joint_predictive_draws(
            &schema,
            &manifest,
            SplitRole::Validation,
            &observation,
            &draws,
            &JointMetricConfig {
                energy_beta: 1.0,
                variogram_power: 1.0,
                pair_weights: vec![1.0; 21],
            },
        )
        .expect("network topology assessment must qualify the frozen fixture")
    }

    fn three_disjoint_edges() -> UndirectedNetwork {
        UndirectedNetwork::new(6, [(0, 1), (2, 3), (4, 5)]).expect("valid matching graph")
    }

    fn temporal_path(reversed: bool) -> TemporalNetworkSequence {
        let mut snapshots = vec![cycle_six(), two_triangles(), three_disjoint_edges()];
        if reversed {
            snapshots.reverse();
        }
        TemporalNetworkSequence::new(vec![10, 20, 30], snapshots).expect("valid temporal path")
    }

    #[test]
    fn reversed_temporal_path_preserves_snapshot_marginals_but_reverses_trend() {
        let forward = temporal_path(false).summarize().expect("forward summary");
        let reversed = temporal_path(true).summarize().expect("reversed summary");

        assert_eq!(forward.snapshot_count, 3);
        assert_eq!(forward.mean_component_count, 2.0);
        assert_eq!(reversed.mean_component_count, 2.0);
        assert!((forward.component_count_trend_per_tick - 0.1).abs() < 1e-12);
        assert!((reversed.component_count_trend_per_tick + 0.1).abs() < 1e-12);
        assert!((forward.mean_degree - (5.0 / 3.0)).abs() < 1e-12);
        assert!((reversed.mean_degree - (5.0 / 3.0)).abs() < 1e-12);
        assert!((forward.mean_degree_trend_per_tick + 0.05).abs() < 1e-12);
        assert!((reversed.mean_degree_trend_per_tick - 0.05).abs() < 1e-12);
        assert_eq!(
            forward.mean_adjacent_edge_jaccard,
            reversed.mean_adjacent_edge_jaccard
        );
        assert_eq!(
            forward.mean_adjacent_edge_turnover,
            reversed.mean_adjacent_edge_turnover
        );
        assert_ne!(
            forward.joint_component_values(),
            reversed.joint_component_values()
        );
    }

    #[test]
    fn temporal_sequence_rejects_invalid_tick_and_node_invariants() {
        assert_eq!(
            TemporalNetworkSequence::new(vec![1], vec![cycle_six()]).unwrap_err(),
            TemporalNetworkError::TooFewSnapshots
        );
        assert_eq!(
            TemporalNetworkSequence::new(vec![1, 2], vec![cycle_six()]).unwrap_err(),
            TemporalNetworkError::TickCountMismatch
        );
        assert_eq!(
            TemporalNetworkSequence::new(vec![2, 1], vec![cycle_six(), two_triangles()])
                .unwrap_err(),
            TemporalNetworkError::TicksNotStrictlyIncreasing
        );
        assert_eq!(
            TemporalNetworkSequence::new(vec![1, 2], vec![cycle_six(), UndirectedNetwork::new(5, [(0, 1)]).unwrap()])
                .unwrap_err(),
            TemporalNetworkError::SnapshotNodeCountMismatch {
                expected: 6,
                actual: 5,
            }
        );
    }

    #[test]
    fn temporal_joint_adequacy_detects_reversed_order_with_same_snapshot_marginals() {
        let statistic = JointStatisticSpec {
            id: "temporal-network-path".to_string(),
            source_family: "synthetic-temporal-network-known-answer".to_string(),
            source_digest: "sha256-temporal-network-source".to_string(),
            population: "six-node-temporal-network".to_string(),
            context: "same-snapshot-multiset-reversed-order".to_string(),
            independent_unit: IndependentUnitKind::Network,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: 20,
            components: [
                ("mean-component-count", "components"),
                ("component-count-trend", "components_per_tick"),
                ("mean-degree", "edges_per_node"),
                ("mean-degree-trend", "edges_per_node_per_tick"),
                ("mean-transitivity", "ratio"),
                ("transitivity-trend", "ratio_per_tick"),
                ("mean-edge-jaccard", "ratio"),
                ("mean-edge-turnover", "ratio"),
                ("mean-edge-addition-rate", "dyad_fraction_per_transition"),
                ("mean-edge-removal-rate", "dyad_fraction_per_transition"),
            ]
            .into_iter()
            .map(|(id, unit)| JointComponentSpec {
                id: id.to_string(),
                measurement_unit: unit.to_string(),
                reference_center: 0.0,
                reference_scale: 1.0,
            })
            .collect(),
        };
        let manifest = SplitManifest {
            id: "temporal-network-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Network,
            calibration_data_digest: "sha256-temporal-network-calibration".to_string(),
            manifest_digest: "sha256-temporal-network-manifest".to_string(),
            calibration_units: vec!["network-calibration".to_string()],
            validation_units: vec!["network-held-out".to_string()],
            challenge_units: Vec::new(),
        };
        let observed = temporal_path(false).summarize().expect("observed path");
        let observation = JointObservation {
            statistic_id: statistic.id.clone(),
            independent_unit_id: "network-held-out".to_string(),
            sample_size: 3,
            source_digest: statistic.source_digest.clone(),
            observation_digest: "sha256-temporal-network-observation".to_string(),
            split_manifest_digest: manifest.manifest_digest.clone(),
            components: observed.joint_component_values(),
        };

        let correct_values = observed.joint_component_values();
        let reversed_values = temporal_path(true)
            .summarize()
            .expect("reversed path")
            .joint_component_values();
        let make_draws = |components: Vec<JointComponentValue>, model: &str| {
            (0..20)
                .map(|draw_index| JointPredictiveDraw {
                    statistic_id: statistic.id.clone(),
                    independent_unit_id: "network-held-out".to_string(),
                    sample_size: 3,
                    draw_index,
                    model_digest: model.to_string(),
                    draw_set_digest: format!("sha256-temporal-draws-{model}"),
                    calibration_data_digest: Some(manifest.calibration_data_digest.clone()),
                    split_manifest_digest: manifest.manifest_digest.clone(),
                    components: components.clone(),
                })
                .collect::<Vec<_>>()
        };
        let config = JointMetricConfig {
            energy_beta: 1.0,
            variogram_power: 1.0,
            pair_weights: vec![1.0; 45],
        };
        let correct = assess_joint_predictive_draws(
            &statistic,
            &manifest,
            SplitRole::Validation,
            &observation,
            &make_draws(correct_values, "sha256-temporal-model-correct"),
            &config,
        )
        .expect("correct temporal model");
        let reversed = assess_joint_predictive_draws(
            &statistic,
            &manifest,
            SplitRole::Validation,
            &observation,
            &make_draws(reversed_values, "sha256-temporal-model-reversed"),
            &config,
        )
        .expect("reversed temporal model");

        assert_eq!(correct.energy_score, 0.0);
        assert_eq!(correct.variogram_score, 0.0);
        assert!(reversed.energy_score > 0.0);
        assert!(reversed.variogram_score > 0.0);
        assert_eq!(correct.predictive_draw_count, 20);
        assert_eq!(reversed.predictive_draw_count, 20);
    }

    #[test]
    fn held_out_joint_adequacy_detects_wrong_topology_with_identical_degrees() {
        let correct = assess_candidate(&cycle_six(), "sha256-network-model-correct");
        let wrong = assess_candidate(&two_triangles(), "sha256-network-model-wrong");
        assert_eq!(correct.energy_score, 0.0);
        assert_eq!(correct.variogram_score, 0.0);
        assert!(wrong.energy_score > 0.0);
        assert!(wrong.variogram_score > 0.0);
        assert_eq!(correct.predictive_draw_count, 20);
        assert_eq!(wrong.predictive_draw_count, 20);

        // Degree-only observations would be identical and cannot expose the
        // topology error; the full relational vector does.
        let cycle = cycle_six().summarize();
        let triangles = two_triangles().summarize();
        assert_eq!(cycle.degree_sequence, triangles.degree_sequence);
        assert_eq!(cycle.mean_degree, triangles.mean_degree);
        assert_eq!(cycle.degree_variance, triangles.degree_variance);
        assert_ne!(cycle.global_transitivity, triangles.global_transitivity);
    }
}
