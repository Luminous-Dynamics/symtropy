// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact finite-design reference for network interference and exposure-map
//! misspecification. This module is a known-truth benchmark, not a general
//! causal estimator or a claim that one exposure mapping is universally valid.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use crate::network_adequacy::{NetworkAdequacyError, UndirectedNetwork};

const MAX_EXACT_DESIGN_NODES: usize = 16;
const OUTCOME_EQUALITY_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkInterferenceError {
    Network(String),
    ClusterMembershipLengthMismatch,
    ClusterIdsNotContiguous,
    ExactDesignTooLarge { node_count: usize, maximum: usize },
    InvalidMechanism,
    AssignmentLengthMismatch,
    InvalidAssignmentProbability,
    InvalidMappingConfiguration,
    NoNetworkCompletions,
    CompletionNodeCountMismatch,
    ObservedEdgeMissingFromCompletion,
    TargetNodeOutOfRange,
    InvalidTreatmentAssignment,
}

impl fmt::Display for NetworkInterferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for NetworkInterferenceError {}

impl From<NetworkAdequacyError> for NetworkInterferenceError {
    fn from(value: NetworkAdequacyError) -> Self {
        Self::Network(value.to_string())
    }
}

/// A simple undirected network with a fixed partition into non-empty clusters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusteredNetwork {
    graph: UndirectedNetwork,
    cluster_ids: Vec<usize>,
    cluster_members: Vec<Vec<usize>>,
}

impl ClusteredNetwork {
    pub fn new(
        graph: UndirectedNetwork,
        cluster_ids: Vec<usize>,
    ) -> Result<Self, NetworkInterferenceError> {
        if cluster_ids.len() != graph.node_count() {
            return Err(NetworkInterferenceError::ClusterMembershipLengthMismatch);
        }
        let cluster_count = cluster_ids
            .iter()
            .copied()
            .max()
            .map_or(0, |maximum| maximum.saturating_add(1));
        if cluster_count == 0 {
            return Err(NetworkInterferenceError::ClusterIdsNotContiguous);
        }

        let mut cluster_members = vec![Vec::new(); cluster_count];
        for (node, cluster) in cluster_ids.iter().copied().enumerate() {
            let Some(members) = cluster_members.get_mut(cluster) else {
                return Err(NetworkInterferenceError::ClusterIdsNotContiguous);
            };
            members.push(node);
        }
        if cluster_members.iter().any(Vec::is_empty) {
            return Err(NetworkInterferenceError::ClusterIdsNotContiguous);
        }

        Ok(Self {
            graph,
            cluster_ids,
            cluster_members,
        })
    }

    pub fn graph(&self) -> &UndirectedNetwork {
        &self.graph
    }

    pub fn cluster_ids(&self) -> &[usize] {
        &self.cluster_ids
    }

    pub fn cluster_count(&self) -> usize {
        self.cluster_members.len()
    }

    pub fn cluster_members(&self) -> &[Vec<usize>] {
        &self.cluster_members
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TreatmentAssignment {
    pub treated: Vec<bool>,
    /// Exact finite-design probability, not a posterior probability over worlds.
    pub probability: f64,
}

/// Enumerate a two-stage randomized saturation design exactly.
///
/// Stage 1 chooses each cluster's treated count uniformly from 0..=cluster size.
/// Stage 2 chooses uniformly among all subsets having that treated count.
/// Every binary assignment is retained with its design probability.
pub fn enumerate_two_stage_saturation_design(
    network: &ClusteredNetwork,
) -> Result<Vec<TreatmentAssignment>, NetworkInterferenceError> {
    let node_count = network.node_count();
    if node_count > MAX_EXACT_DESIGN_NODES {
        return Err(NetworkInterferenceError::ExactDesignTooLarge {
            node_count,
            maximum: MAX_EXACT_DESIGN_NODES,
        });
    }

    let mut cluster_options = Vec::with_capacity(network.cluster_count());
    for members in network.cluster_members() {
        let cluster_size = members.len();
        let mut options = Vec::new();
        for treated_count in 0..=cluster_size {
            let combinations = choose_nodes(members, treated_count);
            if combinations.is_empty() {
                return Err(NetworkInterferenceError::InvalidMechanism);
            }
            let probability = 1.0
                / (cluster_size + 1) as f64
                / combinations.len() as f64;
            for combination in combinations {
                options.push((combination, probability));
            }
        }
        cluster_options.push(options);
    }

    let mut assignments = Vec::new();
    let mut treated = vec![false; node_count];
    enumerate_cluster_products(
        &cluster_options,
        0,
        &mut treated,
        1.0,
        &mut assignments,
    );

    let probability_mass: f64 = assignments
        .iter()
        .map(|assignment| assignment.probability)
        .sum();
    if assignments.is_empty()
        || !probability_mass.is_finite()
        || (probability_mass - 1.0).abs() > 1.0e-12
    {
        return Err(NetworkInterferenceError::InvalidAssignmentProbability);
    }
    Ok(assignments)
}

fn choose_nodes(nodes: &[usize], count: usize) -> Vec<Vec<usize>> {
    fn visit(
        nodes: &[usize],
        count: usize,
        start: usize,
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == count {
            output.push(current.clone());
            return;
        }
        let remaining = count - current.len();
        if nodes.len().saturating_sub(start) < remaining {
            return;
        }
        for index in start..=nodes.len() - remaining {
            current.push(nodes[index]);
            visit(nodes, count, index + 1, current, output);
            current.pop();
        }
    }

    let mut output = Vec::new();
    visit(nodes, count, 0, &mut Vec::new(), &mut output);
    output
}

fn enumerate_cluster_products(
    cluster_options: &[Vec<(Vec<usize>, f64)>],
    cluster_index: usize,
    treated: &mut [bool],
    probability: f64,
    output: &mut Vec<TreatmentAssignment>,
) {
    if cluster_index == cluster_options.len() {
        output.push(TreatmentAssignment {
            treated: treated.to_vec(),
            probability,
        });
        return;
    }

    for (selected, conditional_probability) in &cluster_options[cluster_index] {
        let members: Vec<usize> = cluster_options[cluster_index]
            .iter()
            .flat_map(|(nodes, _)| nodes.iter().copied())
            .collect();
        for node in members {
            treated[node] = false;
        }
        for node in selected {
            treated[*node] = true;
        }
        enumerate_cluster_products(
            cluster_options,
            cluster_index + 1,
            treated,
            probability * conditional_probability,
            output,
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct InterferenceMechanism {
    pub baseline_outcomes: Vec<f64>,
    pub direct_effect: f64,
    pub within_cluster_spillover_per_treated_neighbor: f64,
    pub between_cluster_spillover_per_treated_neighbor: f64,
}

impl InterferenceMechanism {
    pub fn new(
        baseline_outcomes: Vec<f64>,
        direct_effect: f64,
        within_cluster_spillover_per_treated_neighbor: f64,
        between_cluster_spillover_per_treated_neighbor: f64,
    ) -> Result<Self, NetworkInterferenceError> {
        if baseline_outcomes.is_empty()
            || baseline_outcomes.iter().any(|value| !value.is_finite())
            || !direct_effect.is_finite()
            || !within_cluster_spillover_per_treated_neighbor.is_finite()
            || !between_cluster_spillover_per_treated_neighbor.is_finite()
        {
            return Err(NetworkInterferenceError::InvalidMechanism);
        }
        Ok(Self {
            baseline_outcomes,
            direct_effect,
            within_cluster_spillover_per_treated_neighbor,
            between_cluster_spillover_per_treated_neighbor,
        })
    }

    pub fn evaluate(
        &self,
        network: &ClusteredNetwork,
        assignment: &[bool],
    ) -> Result<Vec<f64>, NetworkInterferenceError> {
        if assignment.len() != network.node_count()
            || self.baseline_outcomes.len() != network.node_count()
        {
            return Err(NetworkInterferenceError::AssignmentLengthMismatch);
        }

        let adjacency = adjacency(network.graph());
        let mut outcomes = Vec::with_capacity(network.node_count());
        for node in 0..network.node_count() {
            let mut within_treated = 0_usize;
            let mut between_treated = 0_usize;
            for neighbor in &adjacency[node] {
                if assignment[*neighbor] {
                    if network.cluster_ids[node] == network.cluster_ids[*neighbor] {
                        within_treated += 1;
                    } else {
                        between_treated += 1;
                    }
                }
            }

            let own_treatment = if assignment[node] { 1.0 } else { 0.0 };
            let outcome = self.baseline_outcomes[node]
                + self.direct_effect * own_treatment
                + self.within_cluster_spillover_per_treated_neighbor
                    * within_treated as f64
                + self.between_cluster_spillover_per_treated_neighbor
                    * between_treated as f64;
            if !outcome.is_finite() {
                return Err(NetworkInterferenceError::InvalidMechanism);
            }
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    pub fn known_effects(&self) -> CausalEffectTruth {
        CausalEffectTruth {
            direct_effect: self.direct_effect,
            within_cluster_spillover_per_treated_neighbor: self
                .within_cluster_spillover_per_treated_neighbor,
            between_cluster_spillover_per_treated_neighbor: self
                .between_cluster_spillover_per_treated_neighbor,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CausalEffectTruth {
    pub direct_effect: f64,
    pub within_cluster_spillover_per_treated_neighbor: f64,
    pub between_cluster_spillover_per_treated_neighbor: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExposureMapping {
    FullNetwork,
    WithinClusterOnly,
    WeightedNeighborMean {
        within_weight: u32,
        between_weight: u32,
    },
    Threshold {
        minimum_treated_neighbors: usize,
    },
    TruncatedCanonicalNeighbors {
        maximum_neighbors: usize,
    },
}

impl ExposureMapping {
    pub fn id(&self) -> &'static str {
        match self {
            Self::FullNetwork => "full_network_within_between_counts",
            Self::WithinClusterOnly => "within_cluster_only",
            Self::WeightedNeighborMean { .. } => "weighted_neighbor_mean",
            Self::Threshold { .. } => "treated_neighbor_threshold",
            Self::TruncatedCanonicalNeighbors { .. } => "truncated_canonical_neighbors",
        }
    }

    fn validate(&self) -> Result<(), NetworkInterferenceError> {
        match self {
            Self::WeightedNeighborMean {
                within_weight,
                between_weight,
            } if *within_weight == 0 && *between_weight == 0 => {
                Err(NetworkInterferenceError::InvalidMappingConfiguration)
            }
            Self::Threshold {
                minimum_treated_neighbors,
            } if *minimum_treated_neighbors == 0 => {
                Err(NetworkInterferenceError::InvalidMappingConfiguration)
            }
            Self::TruncatedCanonicalNeighbors { maximum_neighbors } if *maximum_neighbors == 0 => {
                Err(NetworkInterferenceError::InvalidMappingConfiguration)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct ExposureKey {
    own_treated: bool,
    numerator: u64,
    denominator: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExposureMappingSummary {
    pub mapping_id: &'static str,
    pub exposure_cell_count: usize,
    pub nonconstant_exposure_cell_count: usize,
    pub maximum_within_cell_outcome_range: f64,
    pub mean_within_cell_outcome_range: f64,
    /// True only when every assignment pair sharing a unit-specific mapped
    /// exposure also has the same known-truth potential outcome.
    pub mapping_is_sufficient_for_fixture: bool,
}

pub fn assess_exposure_mapping(
    network: &ClusteredNetwork,
    assignments: &[TreatmentAssignment],
    mechanism: &InterferenceMechanism,
    mapping: &ExposureMapping,
) -> Result<ExposureMappingSummary, NetworkInterferenceError> {
    mapping.validate()?;
    validate_assignments(assignments, network.node_count())?;

    let adjacency = adjacency(network.graph());
    let mut cells = BTreeMap::<(usize, ExposureKey), (f64, f64)>::new();
    for assignment in assignments {
        let outcomes = mechanism.evaluate(network, &assignment.treated)?;
        for node in 0..network.node_count() {
            let key = exposure_key(network, &adjacency, &assignment.treated, node, mapping);
            let outcome = outcomes[node];
            cells
                .entry((node, key))
                .and_modify(|range| {
                    range.0 = range.0.min(outcome);
                    range.1 = range.1.max(outcome);
                })
                .or_insert((outcome, outcome));
        }
    }

    let mut nonconstant_cells = 0_usize;
    let mut maximum_range = 0.0_f64;
    let mut range_sum = 0.0_f64;
    for (minimum, maximum) in cells.values() {
        let range = maximum - minimum;
        if range > OUTCOME_EQUALITY_TOLERANCE {
            nonconstant_cells += 1;
        }
        maximum_range = maximum_range.max(range);
        range_sum += range;
    }
    let cell_count = cells.len();
    let mean_range = if cell_count == 0 {
        0.0
    } else {
        range_sum / cell_count as f64
    };

    Ok(ExposureMappingSummary {
        mapping_id: mapping.id(),
        exposure_cell_count: cell_count,
        nonconstant_exposure_cell_count: nonconstant_cells,
        maximum_within_cell_outcome_range: maximum_range,
        mean_within_cell_outcome_range: mean_range,
        mapping_is_sufficient_for_fixture: nonconstant_cells == 0,
    })
}

fn exposure_key(
    network: &ClusteredNetwork,
    adjacency: &[Vec<usize>],
    assignment: &[bool],
    node: usize,
    mapping: &ExposureMapping,
) -> ExposureKey {
    let mut within_treated = 0_usize;
    let mut between_treated = 0_usize;
    for neighbor in &adjacency[node] {
        if assignment[*neighbor] {
            if network.cluster_ids[node] == network.cluster_ids[*neighbor] {
                within_treated += 1;
            } else {
                between_treated += 1;
            }
        }
    }

    let (numerator, denominator) = match mapping {
        ExposureMapping::FullNetwork => (within_treated as u64, between_treated as u64),
        ExposureMapping::WithinClusterOnly => (within_treated as u64, 0),
        ExposureMapping::WeightedNeighborMean {
            within_weight,
            between_weight,
        } => {
            let numerator = within_treated as u64 * u64::from(*within_weight)
                + between_treated as u64 * u64::from(*between_weight);
            let denominator = adjacency[node]
                .iter()
                .map(|neighbor| {
                    if network.cluster_ids[node] == network.cluster_ids[*neighbor] {
                        u64::from(*within_weight)
                    } else {
                        u64::from(*between_weight)
                    }
                })
                .sum::<u64>();
            if denominator == 0 {
                (0, 1)
            } else {
                let divisor = greatest_common_divisor(numerator, denominator);
                (numerator / divisor, denominator / divisor)
            }
        }
        ExposureMapping::Threshold {
            minimum_treated_neighbors,
        } => (
            u64::from(within_treated + between_treated >= *minimum_treated_neighbors),
            0,
        ),
        ExposureMapping::TruncatedCanonicalNeighbors { maximum_neighbors } => {
            let count = adjacency[node]
                .iter()
                .take(*maximum_neighbors)
                .filter(|neighbor| assignment[**neighbor])
                .count();
            (count as u64, 0)
        }
    };

    ExposureKey {
        own_treated: assignment[node],
        numerator,
        denominator,
    }
}

fn greatest_common_divisor(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn adjacency(network: &UndirectedNetwork) -> Vec<Vec<usize>> {
    let mut neighbors = vec![Vec::new(); network.node_count()];
    for &(left, right) in network.canonical_edges() {
        neighbors[left].push(right);
        neighbors[right].push(left);
    }
    for list in &mut neighbors {
        list.sort_unstable();
    }
    neighbors
}

fn validate_assignments(
    assignments: &[TreatmentAssignment],
    node_count: usize,
) -> Result<(), NetworkInterferenceError> {
    if assignments.is_empty()
        || assignments.iter().any(|assignment| {
            assignment.treated.len() != node_count
                || !assignment.probability.is_finite()
                || assignment.probability <= 0.0
        })
    {
        return Err(NetworkInterferenceError::InvalidAssignmentProbability);
    }
    let probability_mass: f64 = assignments
        .iter()
        .map(|assignment| assignment.probability)
        .sum();
    if !probability_mass.is_finite() || (probability_mass - 1.0).abs() > 1.0e-12 {
        return Err(NetworkInterferenceError::InvalidAssignmentProbability);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContrastSupport {
    pub direct_supported_units: usize,
    pub within_cluster_spillover_supported_units: usize,
    pub between_cluster_spillover_supported_units: usize,
    pub total_units: usize,
}

pub fn assess_contrast_support(
    network: &ClusteredNetwork,
    assignments: &[TreatmentAssignment],
) -> Result<ContrastSupport, NetworkInterferenceError> {
    validate_assignments(assignments, network.node_count())?;
    let zero = vec![false; network.node_count()];
    if !has_assignment(assignments, &zero) {
        return Err(NetworkInterferenceError::InvalidAssignmentProbability);
    }
    let adjacency = adjacency(network.graph());
    let mut direct_supported = 0_usize;
    let mut within_supported = 0_usize;
    let mut between_supported = 0_usize;

    for node in 0..network.node_count() {
        let mut direct_toggle = zero.clone();
        direct_toggle[node] = true;
        if has_assignment(assignments, &direct_toggle) {
            direct_supported += 1;
        }

        let within_neighbor = adjacency[node]
            .iter()
            .copied()
            .find(|neighbor| network.cluster_ids[*neighbor] == network.cluster_ids[node]);
        if let Some(neighbor) = within_neighbor {
            let mut toggle = zero.clone();
            toggle[neighbor] = true;
            if has_assignment(assignments, &toggle) {
                within_supported += 1;
            }
        }

        let between_neighbor = adjacency[node]
            .iter()
            .copied()
            .find(|neighbor| network.cluster_ids[*neighbor] != network.cluster_ids[node]);
        if let Some(neighbor) = between_neighbor {
            let mut toggle = zero.clone();
            toggle[neighbor] = true;
            if has_assignment(assignments, &toggle) {
                between_supported += 1;
            }
        }
    }

    Ok(ContrastSupport {
        direct_supported_units: direct_supported,
        within_cluster_spillover_supported_units: within_supported,
        between_cluster_spillover_supported_units: between_supported,
        total_units: network.node_count(),
    })
}

fn has_assignment(assignments: &[TreatmentAssignment], target: &[bool]) -> bool {
    assignments
        .iter()
        .any(|assignment| assignment.treated == target && assignment.probability > 0.0)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartialIdentificationBounds {
    pub target_unit: usize,
    pub completion_count: usize,
    pub lower: f64,
    pub upper: f64,
    pub point_identified: bool,
}

/// Preserve a target-unit policy-effect identified set across declared network
/// completions. Completion counts are not converted into probabilities.
pub fn target_effect_bounds_over_completions(
    observed_partial_graph: &UndirectedNetwork,
    completions: &[UndirectedNetwork],
    cluster_ids: &[usize],
    mechanism: &InterferenceMechanism,
    policy_assignment: &[bool],
    target_unit: usize,
) -> Result<PartialIdentificationBounds, NetworkInterferenceError> {
    if completions.is_empty() {
        return Err(NetworkInterferenceError::NoNetworkCompletions);
    }
    if target_unit >= observed_partial_graph.node_count() {
        return Err(NetworkInterferenceError::TargetNodeOutOfRange);
    }
    if policy_assignment.len() != observed_partial_graph.node_count() {
        return Err(NetworkInterferenceError::AssignmentLengthMismatch);
    }

    let observed_edges: std::collections::BTreeSet<(usize, usize)> = observed_partial_graph
        .canonical_edges()
        .iter()
        .copied()
        .collect();
    let control_assignment = vec![false; observed_partial_graph.node_count()];
    let mut effects = Vec::with_capacity(completions.len());

    for completion in completions {
        if completion.node_count() != observed_partial_graph.node_count() {
            return Err(NetworkInterferenceError::CompletionNodeCountMismatch);
        }
        if !observed_edges
            .iter()
            .all(|edge| completion.canonical_edges().binary_search(edge).is_ok())
        {
            return Err(NetworkInterferenceError::ObservedEdgeMissingFromCompletion);
        }
        let clustered = ClusteredNetwork::new(completion.clone(), cluster_ids.to_vec())?;
        let policy_outcomes = mechanism.evaluate(&clustered, policy_assignment)?;
        let control_outcomes = mechanism.evaluate(&clustered, &control_assignment)?;
        effects.push(policy_outcomes[target_unit] - control_outcomes[target_unit]);
    }

    let lower = effects.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = effects.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !lower.is_finite() || !upper.is_finite() || lower > upper {
        return Err(NetworkInterferenceError::InvalidMechanism);
    }

    Ok(PartialIdentificationBounds {
        target_unit,
        completion_count: completions.len(),
        lower,
        upper,
        point_identified: (upper - lower).abs() <= OUTCOME_EQUALITY_TOLERANCE,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete_network() -> ClusteredNetwork {
        let graph =
            UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 2), (1, 3)]).unwrap();
        ClusteredNetwork::new(graph, vec![0, 0, 1, 1]).unwrap()
    }

    fn mechanism(between_effect: f64) -> InterferenceMechanism {
        InterferenceMechanism::new(vec![0.0; 4], 0.4, 0.2, between_effect).unwrap()
    }

    fn assignments(network: &ClusteredNetwork) -> Vec<TreatmentAssignment> {
        enumerate_two_stage_saturation_design(network).unwrap()
    }

    fn mappings() -> Vec<ExposureMapping> {
        vec![
            ExposureMapping::FullNetwork,
            ExposureMapping::WithinClusterOnly,
            ExposureMapping::WeightedNeighborMean {
                within_weight: 1,
                between_weight: 1,
            },
            ExposureMapping::Threshold {
                minimum_treated_neighbors: 1,
            },
            ExposureMapping::TruncatedCanonicalNeighbors {
                maximum_neighbors: 1,
            },
        ]
    }

    #[test]
    fn two_stage_design_enumerates_all_assignments_and_probability_mass() {
        let network = complete_network();
        let draws = assignments(&network);
        assert_eq!(draws.len(), 16);
        let mass: f64 = draws.iter().map(|draw| draw.probability).sum();
        assert!((mass - 1.0).abs() < 1.0e-12);
        let distinct: std::collections::BTreeSet<Vec<bool>> =
            draws.iter().map(|draw| draw.treated.clone()).collect();
        assert_eq!(distinct.len(), 16);
    }

    #[test]
    fn full_mapping_is_sufficient_but_misspecified_maps_are_not() {
        let network = complete_network();
        let draws = assignments(&network);
        let truth = mechanism(0.6);

        let summaries: Vec<ExposureMappingSummary> = mappings()
            .iter()
            .map(|mapping| assess_exposure_mapping(&network, &draws, &truth, mapping).unwrap())
            .collect();

        assert!(summaries[0].mapping_is_sufficient_for_fixture);
        assert_eq!(summaries[0].maximum_within_cell_outcome_range, 0.0);
        for summary in summaries.iter().skip(1) {
            assert!(!summary.mapping_is_sufficient_for_fixture);
            assert!(summary.maximum_within_cell_outcome_range > 0.0);
        }
        assert!(summaries[1].maximum_within_cell_outcome_range >= 0.6);
    }

    #[test]
    fn omitted_between_cluster_effect_does_not_create_false_mapping_failure() {
        let network = complete_network();
        let draws = assignments(&network);
        let summary = assess_exposure_mapping(
            &network,
            &draws,
            &mechanism(0.0),
            &ExposureMapping::WithinClusterOnly,
        )
        .unwrap();
        assert!(summary.mapping_is_sufficient_for_fixture);
        assert_eq!(summary.maximum_within_cell_outcome_range, 0.0);
    }

    #[test]
    fn direct_within_and_between_contrasts_have_design_support() {
        let network = complete_network();
        let support = assess_contrast_support(&network, &assignments(&network)).unwrap();
        assert_eq!(support.total_units, 4);
        assert_eq!(support.direct_supported_units, 4);
        assert_eq!(support.within_cluster_spillover_supported_units, 4);
        assert_eq!(support.between_cluster_spillover_supported_units, 4);
        let truth = mechanism(0.6).known_effects();
        assert!((truth.direct_effect - 0.4).abs() < 1.0e-12);
        assert!((truth.within_cluster_spillover_per_treated_neighbor - 0.2).abs() < 1.0e-12);
        assert!((truth.between_cluster_spillover_per_treated_neighbor - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn no_cross_cluster_edges_means_no_between_cluster_contrast_support() {
        let graph = UndirectedNetwork::new(4, [(0, 1), (2, 3)]).unwrap();
        let network = ClusteredNetwork::new(graph, vec![0, 0, 1, 1]).unwrap();
        let support = assess_contrast_support(&network, &assignments(&network)).unwrap();
        assert_eq!(support.direct_supported_units, 4);
        assert_eq!(support.within_cluster_spillover_supported_units, 4);
        assert_eq!(support.between_cluster_spillover_supported_units, 0);
    }

    #[test]
    fn partial_graph_completions_return_an_interval_not_a_point_claim() {
        let observed = UndirectedNetwork::new(4, [(0, 1), (2, 3)]).unwrap();
        let completion_a =
            UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 2), (1, 3)]).unwrap();
        let completion_b =
            UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 3), (1, 2)]).unwrap();
        let policy = [true, false, false, false];

        let bounds = target_effect_bounds_over_completions(
            &observed,
            &[completion_a, completion_b],
            &[0, 0, 1, 1],
            &mechanism(0.6),
            &policy,
            3,
        )
        .unwrap();

        assert_eq!(bounds.completion_count, 2);
        assert!(!bounds.point_identified);
        assert!((bounds.lower - 0.0).abs() < 1.0e-12);
        assert!((bounds.upper - 0.6).abs() < 1.0e-12);
    }

    #[test]
    fn completion_must_preserve_every_observed_edge() {
        let observed = UndirectedNetwork::new(4, [(0, 1), (2, 3)]).unwrap();
        let invalid = UndirectedNetwork::new(4, [(0, 1), (0, 2)]).unwrap();
        let error = target_effect_bounds_over_completions(
            &observed,
            &[invalid],
            &[0, 0, 1, 1],
            &mechanism(0.6),
            &[true, false, false, false],
            3,
        )
        .unwrap_err();
        assert_eq!(error, NetworkInterferenceError::ObservedEdgeMissingFromCompletion);
    }

    #[test]
    fn assignment_and_network_contracts_fail_closed() {
        let graph = UndirectedNetwork::new(4, [(0, 1), (2, 3)]).unwrap();
        assert_eq!(
            ClusteredNetwork::new(graph.clone(), vec![0, 0, 1]),
            Err(NetworkInterferenceError::ClusterMembershipLengthMismatch)
        );
        assert_eq!(
            ClusteredNetwork::new(graph, vec![0, 0, 2, 2]),
            Err(NetworkInterferenceError::ClusterIdsNotContiguous)
        );
        assert_eq!(
            InterferenceMechanism::new(vec![0.0; 4], f64::NAN, 0.2, 0.6),
            Err(NetworkInterferenceError::InvalidMechanism)
        );
        let network = complete_network();
        assert_eq!(
            assess_exposure_mapping(
                &network,
                &assignments(&network),
                &mechanism(0.6),
                &ExposureMapping::Threshold {
                    minimum_treated_neighbors: 0
                }
            ),
            Err(NetworkInterferenceError::InvalidMappingConfiguration)
        );
        assert_eq!(
            assess_exposure_mapping(
                &network,
                &assignments(&network),
                &mechanism(0.6),
                &ExposureMapping::WeightedNeighborMean {
                    within_weight: 0,
                    between_weight: 0
                }
            ),
            Err(NetworkInterferenceError::InvalidMappingConfiguration)
        );
    }

    #[test]
    fn edge_ordering_does_not_change_mapping_results() {
        let left = ClusteredNetwork::new(
            UndirectedNetwork::new(4, [(0, 1), (2, 3), (0, 2), (1, 3)]).unwrap(),
            vec![0, 0, 1, 1],
        )
        .unwrap();
        let right = ClusteredNetwork::new(
            UndirectedNetwork::new(4, [(3, 1), (2, 0), (3, 2), (1, 0)]).unwrap(),
            vec![0, 0, 1, 1],
        )
        .unwrap();
        let left_draws = assignments(&left);
        let right_draws = assignments(&right);
        for mapping in mappings() {
            let first = assess_exposure_mapping(&left, &left_draws, &mechanism(0.6), &mapping)
                .unwrap();
            let second = assess_exposure_mapping(&right, &right_draws, &mechanism(0.6), &mapping)
                .unwrap();
            assert_eq!(first, second);
        }
    }

    #[test]
    fn exact_design_size_is_bounded() {
        let graph = UndirectedNetwork::new(17, []).unwrap();
        let network = ClusteredNetwork::new(graph, vec![0; 17]).unwrap();
        assert_eq!(
            enumerate_two_stage_saturation_design(&network),
            Err(NetworkInterferenceError::ExactDesignTooLarge {
                node_count: 17,
                maximum: MAX_EXACT_DESIGN_NODES
            })
        );
    }
}
