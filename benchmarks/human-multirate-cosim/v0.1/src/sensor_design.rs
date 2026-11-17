//! Exact small-graph sensor-placement and observability benchmark.
//!
//! This module is a synthetic mathematical reference. It does not model human
//! physiology or physical sensor performance. It composes the graph spectral
//! oracle in `network` and keeps sensor selection separate from state estimation.
//!
//! For a frozen linear graph, observations at times t_k with selection operator
//! H and full channel covariance R contribute
//!
//!   W_o = sum_k Phi(t_k)^T H^T R^(-1) H Phi(t_k).
//!
//! The covariance is retained as a full matrix. Unknown, asymmetric, singular,
//! or non-positive-definite covariance is rejected; no diagonal fallback,
//! pseudoinverse, or hidden jitter is used.

use crate::network::{NetworkError, NetworkScenario, symmetric_eigen};
use std::collections::{HashMap, HashSet};
use std::fmt;

const MAX_EXACT_CANDIDATES: usize = 16;
const RANK_RELATIVE_TOLERANCE: f64 = 1.0e-10;
const SYMMETRY_TOLERANCE: f64 = 1.0e-12;

/// One candidate sensor measuring a single abstract graph-node state.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorCandidate {
    pub id: u32,
    pub node: usize,
    pub cost: f64,
    /// Sensors sharing a group are assumed to fail together in that
    /// explicitly enumerated common-cause scenario.
    pub common_cause_group: Option<u32>,
}

/// Frozen sensor-placement problem. Covariance rows/columns follow the
/// candidate vector order and are never silently reordered.
#[derive(Debug, Clone, PartialEq)]
pub struct SensorDesignScenario {
    pub name: &'static str,
    pub network: NetworkScenario,
    pub candidates: Vec<SensorCandidate>,
    pub noise_covariance: Vec<Vec<f64>>,
    /// Observation times relative to the initial network state.
    pub observation_times: Vec<f64>,
    pub sensor_budget: f64,
}

impl SensorDesignScenario {
    pub fn validate(&self) -> Result<(), SensorDesignError> {
        self.network.validate().map_err(SensorDesignError::Network)?;
        let count = self.candidates.len();
        if count == 0 || count > MAX_EXACT_CANDIDATES {
            return Err(SensorDesignError::InvalidCandidateCount);
        }
        if !self.sensor_budget.is_finite() || self.sensor_budget <= 0.0 {
            return Err(SensorDesignError::InvalidBudget);
        }
        if self.noise_covariance.len() != count
            || self.noise_covariance.iter().any(|row| row.len() != count)
        {
            return Err(SensorDesignError::CovarianceShapeMismatch);
        }
        if self.observation_times.is_empty() {
            return Err(SensorDesignError::MissingObservationTimes);
        }

        let mut ids = HashSet::new();
        for (index, sensor) in self.candidates.iter().enumerate() {
            if !ids.insert(sensor.id) {
                return Err(SensorDesignError::DuplicateSensorId(sensor.id));
            }
            if sensor.node >= self.network.capacities.len() {
                return Err(SensorDesignError::UnknownSensorNode { sensor: index });
            }
            if !sensor.cost.is_finite() || sensor.cost <= 0.0 {
                return Err(SensorDesignError::InvalidSensorCost { sensor: index });
            }
        }

        let mut previous_time = -1.0;
        for time in &self.observation_times {
            if !time.is_finite()
                || *time < 0.0
                || *time <= previous_time
                || *time > self.network.horizon
            {
                return Err(SensorDesignError::InvalidObservationTime);
            }
            previous_time = *time;
        }

        validate_spd(&self.noise_covariance)?;
        Ok(())
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }
}

/// Objective used to compare budget-feasible sensor subsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementCriterion {
    /// Maximize the smallest information eigenvalue. Rank-deficient designs
    /// have a zero minimum eigenvalue and remain visible in the results.
    EOptimal,
    /// Maximize log(det(W_o)); only full-rank designs are eligible.
    DOptimal,
    /// Minimize trace(inv(W_o)); only full-rank designs are eligible.
    AOptimal,
}

/// Information and observability metrics for one exact sensor subset.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementEvaluation {
    pub selected_sensor_ids: Vec<u32>,
    pub total_cost: f64,
    pub state_dimension: usize,
    pub observability_rank: usize,
    pub information_eigenvalues: Vec<f64>,
    pub minimum_information_eigenvalue: f64,
    pub maximum_information_eigenvalue: f64,
    pub condition_number: Option<f64>,
    pub log_determinant: Option<f64>,
    pub trace_inverse: Option<f64>,
}

/// Every budget-feasible subset and the deterministic best subset under one
/// declared objective. No heuristic is used in the exact small-instance lane.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacementTournament {
    pub criterion: PlacementCriterion,
    pub evaluations: Vec<PlacementEvaluation>,
    pub best: Option<PlacementEvaluation>,
}

/// One declared common-cause sensor-group loss and the remaining information.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonCauseFailureEvaluation {
    pub failed_group: u32,
    pub remaining: PlacementEvaluation,
}

/// Nominal and enumerated common-cause robustness for one sensor subset.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustPlacementEvaluation {
    pub nominal: PlacementEvaluation,
    pub failures: Vec<CommonCauseFailureEvaluation>,
    pub worst_case_rank: usize,
    pub worst_case_minimum_information_eigenvalue: f64,
}

/// Exact subset tournament where the objective is worst-case E-optimality
/// over every common-cause group represented in the selected subset.
#[derive(Debug, Clone, PartialEq)]
pub struct RobustPlacementTournament {
    pub evaluations: Vec<RobustPlacementEvaluation>,
    pub best: Option<RobustPlacementEvaluation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorDesignError {
    Network(NetworkError),
    InvalidCandidateCount,
    InvalidBudget,
    CovarianceShapeMismatch,
    MissingObservationTimes,
    DuplicateSensorId(u32),
    UnknownSensorNode { sensor: usize },
    InvalidSensorCost { sensor: usize },
    InvalidObservationTime,
    NonFiniteCovariance,
    AsymmetricCovariance,
    CovarianceNotPositiveDefinite,
    EmptySelection,
    UnknownSensorId(u32),
    DuplicateSelectedSensor(u32),
    BudgetExceeded,
    InvalidInformationMatrix,
}

impl fmt::Display for SensorDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "network scenario invalid: {error}"),
            Self::InvalidCandidateCount => write!(
                f,
                "exact sensor subset search requires between one and sixteen candidates"
            ),
            Self::InvalidBudget => write!(f, "sensor budget must be finite and positive"),
            Self::CovarianceShapeMismatch => {
                write!(f, "noise covariance shape must match candidate count")
            }
            Self::MissingObservationTimes => write!(f, "at least one observation time is required"),
            Self::DuplicateSensorId(id) => write!(f, "duplicate sensor ID {id}"),
            Self::UnknownSensorNode { sensor } => {
                write!(f, "sensor candidate {sensor} references an unknown graph node")
            }
            Self::InvalidSensorCost { sensor } => {
                write!(f, "sensor candidate {sensor} has an invalid cost")
            }
            Self::InvalidObservationTime => {
                write!(f, "observation times must be finite, increasing and within the horizon")
            }
            Self::NonFiniteCovariance => {
                write!(f, "noise covariance contains a non-finite value")
            }
            Self::AsymmetricCovariance => {
                write!(f, "noise covariance must be symmetric within the declared tolerance")
            }
            Self::CovarianceNotPositiveDefinite => {
                write!(f, "noise covariance must be strictly positive definite")
            }
            Self::EmptySelection => write!(f, "at least one sensor must be selected"),
            Self::UnknownSensorId(id) => write!(f, "unknown selected sensor ID {id}"),
            Self::DuplicateSelectedSensor(id) => {
                write!(f, "sensor ID {id} was selected more than once")
            }
            Self::BudgetExceeded => write!(f, "selected sensors exceed the declared budget"),
            Self::InvalidInformationMatrix => {
                write!(f, "information matrix is not positive semidefinite within tolerance")
            }
        }
    }
}

impl std::error::Error for SensorDesignError {}

/// Evaluate one declared subset by building its full-covariance observability
/// information matrix. Sensor IDs are resolved against the frozen manifest.
pub fn evaluate_placement(
    scenario: &SensorDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<PlacementEvaluation, SensorDesignError> {
    scenario.validate()?;
    let indices = resolve_selected_indices(scenario, selected_sensor_ids)?;
    evaluate_indices(scenario, &indices)
}

/// Enumerate all non-empty, budget-feasible subsets and select the optimum
/// under the declared objective. Ties prefer higher rank, lower cost, then
/// lexicographically smaller sensor IDs.
pub fn enumerate_placements(
    scenario: &SensorDesignScenario,
    criterion: PlacementCriterion,
) -> Result<PlacementTournament, SensorDesignError> {
    scenario.validate()?;
    let count = scenario.candidates.len();
    let subset_count = 1_u64 << count;
    let mut evaluations = Vec::new();
    let mut best: Option<PlacementEvaluation> = None;

    for mask in 1..subset_count {
        let indices: Vec<usize> = (0..count)
            .filter(|index| (mask & (1_u64 << index)) != 0)
            .collect();
        let cost: f64 = indices
            .iter()
            .map(|index| scenario.candidates[*index].cost)
            .sum();
        if cost > scenario.sensor_budget + 1.0e-12 {
            continue;
        }

        let evaluation = evaluate_indices(scenario, &indices)?;
        if objective_value(&evaluation, criterion).is_none() {
            evaluations.push(evaluation);
            continue;
        }
        if best
            .as_ref()
            .is_none_or(|current| is_better(&evaluation, current, criterion))
        {
            best = Some(evaluation.clone());
        }
        evaluations.push(evaluation);
    }

    Ok(PlacementTournament {
        criterion,
        evaluations,
        best,
    })
}

/// Evaluate nominal observability and every single declared common-cause group
/// outage for the selected subset. Independent failures are not inferred here.
pub fn evaluate_common_cause_robustness(
    scenario: &SensorDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<RobustPlacementEvaluation, SensorDesignError> {
    scenario.validate()?;
    let indices = resolve_selected_indices(scenario, selected_sensor_ids)?;
    let nominal = evaluate_indices(scenario, &indices)?;

    let groups: HashSet<u32> = indices
        .iter()
        .filter_map(|index| scenario.candidates[*index].common_cause_group)
        .collect();
    let mut ordered_groups: Vec<u32> = groups.into_iter().collect();
    ordered_groups.sort_unstable();

    let mut failures = Vec::with_capacity(ordered_groups.len());
    let mut worst_case_rank = nominal.observability_rank;
    let mut worst_case_minimum_information_eigenvalue =
        nominal.minimum_information_eigenvalue;

    for group in ordered_groups {
        let remaining_indices: Vec<usize> = indices
            .iter()
            .copied()
            .filter(|index| scenario.candidates[*index].common_cause_group != Some(group))
            .collect();
        let remaining = evaluate_indices(scenario, &remaining_indices)?;
        worst_case_rank = worst_case_rank.min(remaining.observability_rank);
        worst_case_minimum_information_eigenvalue = worst_case_minimum_information_eigenvalue
            .min(remaining.minimum_information_eigenvalue);
        failures.push(CommonCauseFailureEvaluation {
            failed_group: group,
            remaining,
        });
    }

    Ok(RobustPlacementEvaluation {
        nominal,
        failures,
        worst_case_rank,
        worst_case_minimum_information_eigenvalue,
    })
}

/// Enumerate all budget-feasible subsets and maximize the minimum information
/// eigenvalue after any one selected common-cause group is lost.
pub fn enumerate_robust_placements(
    scenario: &SensorDesignScenario,
) -> Result<RobustPlacementTournament, SensorDesignError> {
    scenario.validate()?;
    let count = scenario.candidates.len();
    let subset_count = 1_u64 << count;
    let mut evaluations = Vec::new();
    let mut best: Option<RobustPlacementEvaluation> = None;

    for mask in 1..subset_count {
        let indices: Vec<usize> = (0..count)
            .filter(|index| (mask & (1_u64 << index)) != 0)
            .collect();
        let cost: f64 = indices
            .iter()
            .map(|index| scenario.candidates[*index].cost)
            .sum();
        if cost > scenario.sensor_budget + 1.0e-12 {
            continue;
        }
        let ids: Vec<u32> = indices
            .iter()
            .map(|index| scenario.candidates[*index].id)
            .collect();
        let evaluation = evaluate_common_cause_robustness(scenario, &ids)?;
        if best
            .as_ref()
            .is_none_or(|current| is_better_robust(&evaluation, current))
        {
            best = Some(evaluation.clone());
        }
        evaluations.push(evaluation);
    }

    Ok(RobustPlacementTournament { evaluations, best })
}

fn resolve_selected_indices(
    scenario: &SensorDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<Vec<usize>, SensorDesignError> {
    if selected_sensor_ids.is_empty() {
        return Err(SensorDesignError::EmptySelection);
    }
    let mut seen = HashSet::new();
    let lookup: HashMap<u32, usize> = scenario
        .candidates
        .iter()
        .enumerate()
        .map(|(index, sensor)| (sensor.id, index))
        .collect();
    let mut indices = Vec::with_capacity(selected_sensor_ids.len());
    for id in selected_sensor_ids {
        if !seen.insert(*id) {
            return Err(SensorDesignError::DuplicateSelectedSensor(*id));
        }
        let index = lookup
            .get(id)
            .copied()
            .ok_or(SensorDesignError::UnknownSensorId(*id))?;
        indices.push(index);
    }
    indices.sort_unstable();

    let cost: f64 = indices
        .iter()
        .map(|index| scenario.candidates[*index].cost)
        .sum();
    if cost > scenario.sensor_budget + 1.0e-12 {
        return Err(SensorDesignError::BudgetExceeded);
    }
    Ok(indices)
}

fn evaluate_indices(
    scenario: &SensorDesignScenario,
    indices: &[usize],
) -> Result<PlacementEvaluation, SensorDesignError> {
    let dimension = scenario.network.capacities.len();
    let selected_sensor_ids: Vec<u32> = indices
        .iter()
        .map(|index| scenario.candidates[*index].id)
        .collect();
    let total_cost: f64 = indices
        .iter()
        .map(|index| scenario.candidates[*index].cost)
        .sum();

    if indices.is_empty() {
        return Ok(PlacementEvaluation {
            selected_sensor_ids,
            total_cost,
            state_dimension: dimension,
            observability_rank: 0,
            information_eigenvalues: vec![0.0; dimension],
            minimum_information_eigenvalue: 0.0,
            maximum_information_eigenvalue: 0.0,
            condition_number: None,
            log_determinant: None,
            trace_inverse: None,
        });
    }

    let covariance = principal_submatrix(&scenario.noise_covariance, indices);
    let covariance_inverse = invert_spd(&covariance)?;
    let oracle = scenario
        .network
        .spectral_oracle()
        .map_err(SensorDesignError::Network)?;
    let mut information = vec![vec![0.0; dimension]; dimension];

    for time in &scenario.observation_times {
        let transition = oracle
            .transition_matrix(*time)
            .map_err(SensorDesignError::Network)?;
        let sensor_count = indices.len();
        let mut observed_transition = vec![vec![0.0; dimension]; sensor_count];

        for (row, sensor_index) in indices.iter().enumerate() {
            let node = scenario.candidates[*sensor_index].node;
            observed_transition[row].clone_from_slice(&transition[node]);
        }

        let mut weighted = vec![vec![0.0; dimension]; sensor_count];
        for row in 0..sensor_count {
            for column in 0..dimension {
                for inner in 0..sensor_count {
                    weighted[row][column] +=
                        covariance_inverse[row][inner] * observed_transition[inner][column];
                }
            }
        }

        for i in 0..dimension {
            for j in 0..dimension {
                for row in 0..sensor_count {
                    information[i][j] +=
                        observed_transition[row][i] * weighted[row][j];
                }
            }
        }
    }

    // Remove only roundoff-level asymmetry; do not alter rank by regularization.
    for i in 0..dimension {
        for j in (i + 1)..dimension {
            let mean = 0.5 * (information[i][j] + information[j][i]);
            information[i][j] = mean;
            information[j][i] = mean;
        }
    }

    let (mut eigenvalues, _) =
        symmetric_eigen(information).map_err(SensorDesignError::Network)?;
    for value in &mut eigenvalues {
        if *value < -1.0e-10 {
            return Err(SensorDesignError::InvalidInformationMatrix);
        }
        if *value < 0.0 {
            *value = 0.0;
        }
    }
    eigenvalues.sort_by(f64::total_cmp);

    let maximum = eigenvalues.last().copied().unwrap_or(0.0);
    let threshold = (maximum * RANK_RELATIVE_TOLERANCE).max(1.0e-14);
    let rank = eigenvalues.iter().filter(|value| **value > threshold).count();
    let minimum = eigenvalues.first().copied().unwrap_or(0.0);
    let full_rank = rank == dimension;

    let (condition_number, log_determinant, trace_inverse) = if full_rank {
        let condition = maximum / minimum;
        let logdet = eigenvalues.iter().map(|value| value.ln()).sum();
        let trace = eigenvalues.iter().map(|value| 1.0 / value).sum();
        (Some(condition), Some(logdet), Some(trace))
    } else {
        (None, None, None)
    };

    Ok(PlacementEvaluation {
        selected_sensor_ids,
        total_cost,
        state_dimension: dimension,
        observability_rank: rank,
        information_eigenvalues: eigenvalues,
        minimum_information_eigenvalue: minimum,
        maximum_information_eigenvalue: maximum,
        condition_number,
        log_determinant,
        trace_inverse,
    })
}

fn objective_value(
    evaluation: &PlacementEvaluation,
    criterion: PlacementCriterion,
) -> Option<f64> {
    match criterion {
        PlacementCriterion::EOptimal => Some(evaluation.minimum_information_eigenvalue),
        PlacementCriterion::DOptimal => evaluation.log_determinant,
        PlacementCriterion::AOptimal => evaluation.trace_inverse.map(|value| -value),
    }
}

fn is_better(
    candidate: &PlacementEvaluation,
    current: &PlacementEvaluation,
    criterion: PlacementCriterion,
) -> bool {
    let candidate_value = objective_value(candidate, criterion).unwrap_or(f64::NEG_INFINITY);
    let current_value = objective_value(current, criterion).unwrap_or(f64::NEG_INFINITY);
    if candidate_value > current_value + 1.0e-12 {
        return true;
    }
    if current_value > candidate_value + 1.0e-12 {
        return false;
    }
    if candidate.observability_rank != current.observability_rank {
        return candidate.observability_rank > current.observability_rank;
    }
    if (candidate.total_cost - current.total_cost).abs() > 1.0e-12 {
        return candidate.total_cost < current.total_cost;
    }
    candidate.selected_sensor_ids < current.selected_sensor_ids
}

fn is_better_robust(
    candidate: &RobustPlacementEvaluation,
    current: &RobustPlacementEvaluation,
) -> bool {
    if candidate.worst_case_minimum_information_eigenvalue
        > current.worst_case_minimum_information_eigenvalue + 1.0e-12
    {
        return true;
    }
    if current.worst_case_minimum_information_eigenvalue
        > candidate.worst_case_minimum_information_eigenvalue + 1.0e-12
    {
        return false;
    }
    if candidate.worst_case_rank != current.worst_case_rank {
        return candidate.worst_case_rank > current.worst_case_rank;
    }
    if (candidate.nominal.total_cost - current.nominal.total_cost).abs() > 1.0e-12 {
        return candidate.nominal.total_cost < current.nominal.total_cost;
    }
    candidate.nominal.selected_sensor_ids < current.nominal.selected_sensor_ids
}

pub(crate) fn principal_submatrix(matrix: &[Vec<f64>], indices: &[usize]) -> Vec<Vec<f64>> {
    indices
        .iter()
        .map(|row| indices.iter().map(|column| matrix[*row][*column]).collect())
        .collect()
}

pub(crate) fn validate_spd(matrix: &[Vec<f64>]) -> Result<(), SensorDesignError> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(SensorDesignError::NonFiniteCovariance);
    }
    let n = matrix.len();
    for i in 0..n {
        for j in 0..i {
            if (matrix[i][j] - matrix[j][i]).abs() > SYMMETRY_TOLERANCE {
                return Err(SensorDesignError::AsymmetricCovariance);
            }
        }
    }
    cholesky(matrix).map(|_| ())
}

fn cholesky(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, SensorDesignError> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut value = matrix[i][j];
            for k in 0..j {
                value -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err(SensorDesignError::CovarianceNotPositiveDefinite);
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

pub(crate) fn invert_spd(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, SensorDesignError> {
    let lower = cholesky(matrix)?;
    let n = matrix.len();
    let mut inverse = vec![vec![0.0; n]; n];

    for column in 0..n {
        let mut y = vec![0.0; n];
        for row in 0..n {
            let mut value = if row == column { 1.0 } else { 0.0 };
            for k in 0..row {
                value -= lower[row][k] * y[k];
            }
            y[row] = value / lower[row][row];
        }

        let mut x = vec![0.0; n];
        for row in (0..n).rev() {
            let mut value = y[row];
            for k in (row + 1)..n {
                value -= lower[k][row] * x[k];
            }
            x[row] = value / lower[row][row];
        }
        for row in 0..n {
            inverse[row][column] = x[row];
        }
    }

    Ok(inverse)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{Edge, NetworkScenario};

    fn scenario() -> SensorDesignScenario {
        SensorDesignScenario {
            name: "three_node_equal_chain_sensor_placement",
            network: NetworkScenario {
                name: "three_node_equal_chain",
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![1.0, 0.0, 0.2],
                edges: vec![
                    Edge {
                        left: 0,
                        right: 1,
                        conductance: 1.0,
                    },
                    Edge {
                        left: 1,
                        right: 2,
                        conductance: 1.0,
                    },
                ],
                horizon: 2.0,
            },
            candidates: vec![
                SensorCandidate {
                    id: 10,
                    node: 0,
                    cost: 1.0,
                    common_cause_group: Some(100),
                },
                SensorCandidate {
                    id: 20,
                    node: 1,
                    cost: 1.0,
                    common_cause_group: None,
                },
                SensorCandidate {
                    id: 30,
                    node: 2,
                    cost: 1.0,
                    common_cause_group: Some(100),
                },
            ],
            noise_covariance: vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            observation_times: vec![0.0, 0.2, 0.5, 1.0],
            sensor_budget: 2.0,
        }
    }

    #[test]
    fn spectral_transition_matches_the_state_oracle() {
        let scenario = scenario();
        let oracle = scenario.network.spectral_oracle().unwrap();
        let transition = oracle.transition_matrix(0.4).unwrap();
        let mapped: Vec<f64> = transition
            .iter()
            .map(|row| {
                row.iter()
                    .zip(scenario.network.initial.iter())
                    .map(|(a, b)| a * b)
                    .sum()
            })
            .collect();
        let expected = oracle.state_at(0.4).unwrap();
        for (actual, reference) in mapped.iter().zip(expected.iter()) {
            assert!((actual - reference).abs() < 1.0e-11);
        }
    }

    #[test]
    fn central_sensor_misses_the_chain_antisymmetric_mode() {
        let result = evaluate_placement(&scenario(), &[20]).unwrap();
        assert_eq!(result.observability_rank, 2);
        assert_eq!(result.state_dimension, 3);
        assert!(result.log_determinant.is_none());
        assert!(result.trace_inverse.is_none());
    }

    #[test]
    fn endpoint_sensor_observes_all_distinct_chain_modes() {
        let result = evaluate_placement(&scenario(), &[10]).unwrap();
        assert_eq!(result.observability_rank, 3);
        assert!(result.log_determinant.is_some());
        assert!(result.trace_inverse.is_some());
    }

    #[test]
    fn exact_e_optimal_subset_prefers_a_full_rank_sensor() {
        let mut problem = scenario();
        problem.sensor_budget = 1.0;
        let result = enumerate_placements(&problem, PlacementCriterion::EOptimal).unwrap();
        let best = result.best.unwrap();
        assert_eq!(best.observability_rank, 3);
        assert_eq!(best.total_cost, 1.0);
        assert!(best.selected_sensor_ids == vec![10] || best.selected_sensor_ids == vec![30]);
    }

    #[test]
    fn correlated_covariance_is_used_and_invalid_covariance_rejects() {
        let mut problem = scenario();
        problem.noise_covariance = vec![
            vec![1.0, 0.9, 0.0],
            vec![0.9, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let correlated = evaluate_placement(&problem, &[10, 20]).unwrap();

        problem.noise_covariance = vec![
            vec![1.0, 1.1, 0.0],
            vec![1.1, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        assert_eq!(
            evaluate_placement(&problem, &[10, 20]),
            Err(SensorDesignError::CovarianceNotPositiveDefinite)
        );
        assert_eq!(correlated.observability_rank, 3);
    }

    #[test]
    fn common_cause_outage_removes_every_sensor_in_the_group() {
        let result = evaluate_common_cause_robustness(&scenario(), &[10, 30]).unwrap();
        assert_eq!(result.failures.len(), 1);
        assert_eq!(result.failures[0].failed_group, 100);
        assert_eq!(result.failures[0].remaining.observability_rank, 0);
        assert_eq!(result.worst_case_rank, 0);
        assert_eq!(result.worst_case_minimum_information_eigenvalue, 0.0);
    }

    #[test]
    fn sensor_budget_and_duplicate_selection_fail_closed() {
        let mut problem = scenario();
        problem.sensor_budget = 1.0;
        assert_eq!(
            evaluate_placement(&problem, &[10, 30]),
            Err(SensorDesignError::BudgetExceeded)
        );
        assert_eq!(
            evaluate_placement(&scenario(), &[10, 10]),
            Err(SensorDesignError::DuplicateSelectedSensor(10))
        );
    }
}
