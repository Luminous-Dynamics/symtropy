//! Synthetic graph-coupled compartment systems with an independent spectral oracle.
//!
//! This module is a mathematical benchmark, not a physiological model.
//! Capacities, conductances, states and time are arbitrary quantities.
//!
//! For an undirected graph with node capacities C_i and edge conductances g_ij:
//!
//!     C_i * dx_i/dt = sum_j g_ij * (x_j - x_i)
//!
//! Hence C * dx/dt = -L * x, where L is the weighted graph Laplacian.
//! The total sum(C_i * x_i) is conserved and the C-weighted quadratic energy
//! 0.5 * sum(C_i * x_i^2) is non-increasing for the exact continuous dynamics.

use std::collections::HashSet;
use std::fmt;

/// One undirected exchange edge between two node indices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub left: usize,
    pub right: usize,
    pub conductance: f64,
}

/// Frozen graph and initial state for a synthetic network benchmark.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkScenario {
    pub name: &'static str,
    pub capacities: Vec<f64>,
    pub initial: Vec<f64>,
    pub edges: Vec<Edge>,
    pub horizon: f64,
}

impl NetworkScenario {
    pub fn validate(&self) -> Result<(), NetworkError> {
        let n = self.capacities.len();
        if n < 2 || self.initial.len() != n {
            return Err(NetworkError::InvalidNodeCount);
        }
        if !self.horizon.is_finite() || self.horizon <= 0.0 {
            return Err(NetworkError::InvalidHorizon);
        }

        for (index, capacity) in self.capacities.iter().enumerate() {
            if !capacity.is_finite() || *capacity <= 0.0 {
                return Err(NetworkError::InvalidCapacity { node: index });
            }
        }
        for (index, value) in self.initial.iter().enumerate() {
            if !value.is_finite() {
                return Err(NetworkError::NonFiniteInitialState { node: index });
            }
        }

        let mut edge_ids = HashSet::new();
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.left >= n || edge.right >= n {
                return Err(NetworkError::UnknownNode { edge: index });
            }
            if edge.left == edge.right {
                return Err(NetworkError::SelfEdge { edge: index });
            }
            if !edge.conductance.is_finite() || edge.conductance < 0.0 {
                return Err(NetworkError::InvalidConductance { edge: index });
            }
            let key = if edge.left < edge.right {
                (edge.left, edge.right)
            } else {
                (edge.right, edge.left)
            };
            if !edge_ids.insert(key) {
                return Err(NetworkError::DuplicateEdge {
                    left: key.0,
                    right: key.1,
                });
            }
        }
        Ok(())
    }

    pub fn conserved_total(&self) -> f64 {
        self.capacities
            .iter()
            .zip(self.initial.iter())
            .map(|(capacity, value)| capacity * value)
            .sum()
    }

    pub fn quadratic_energy(&self, state: &[f64]) -> f64 {
        0.5 * self
            .capacities
            .iter()
            .zip(state.iter())
            .map(|(capacity, value)| capacity * value * value)
            .sum::<f64>()
    }

    fn laplacian(&self) -> Vec<Vec<f64>> {
        let n = self.capacities.len();
        let mut laplacian = vec![vec![0.0; n]; n];
        for edge in &self.edges {
            let i = edge.left;
            let j = edge.right;
            let g = edge.conductance;
            laplacian[i][i] += g;
            laplacian[j][j] += g;
            laplacian[i][j] -= g;
            laplacian[j][i] -= g;
        }
        laplacian
    }

    /// Construct the independent spectral oracle for this frozen graph.
    pub fn spectral_oracle(&self) -> Result<SpectralOracle, NetworkError> {
        self.validate()?;
        let n = self.capacities.len();
        let laplacian = self.laplacian();
        let mut symmetric = vec![vec![0.0; n]; n];
        let sqrt_capacity: Vec<f64> = self.capacities.iter().map(|value| value.sqrt()).collect();

        // B = C^(-1/2) * L * C^(-1/2) is symmetric positive semidefinite.
        for i in 0..n {
            for j in 0..n {
                symmetric[i][j] = laplacian[i][j] / (sqrt_capacity[i] * sqrt_capacity[j]);
            }
        }

        let (mut eigenvalues, eigenvectors) = symmetric_eigen(symmetric)?;
        for value in &mut eigenvalues {
            if *value < -1.0e-10 {
                return Err(NetworkError::NonPositiveSemidefiniteLaplacian);
            }
            if *value < 0.0 {
                *value = 0.0;
            }
        }

        let transformed_initial: Vec<f64> = self
            .initial
            .iter()
            .zip(sqrt_capacity.iter())
            .map(|(value, scale)| value * scale)
            .collect();

        let n = self.capacities.len();
        let mut modal_coefficients = vec![0.0; n];
        for mode in 0..n {
            for node in 0..n {
                modal_coefficients[mode] += eigenvectors[node][mode] * transformed_initial[node];
            }
        }

        let max_eigenvalue = eigenvalues.iter().copied().fold(0.0_f64, f64::max);
        Ok(SpectralOracle {
            capacities: self.capacities.clone(),
            sqrt_capacity,
            eigenvalues,
            eigenvectors,
            modal_coefficients,
            max_eigenvalue,
        })
    }
}

/// Exact continuous-time solution from a symmetric spectral decomposition.
///
/// The eigensolver is numerical, so "exact" here means the closed-form matrix
/// exponential evaluated through an independently computed eigendecomposition,
/// not symbolic exact arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub struct SpectralOracle {
    capacities: Vec<f64>,
    sqrt_capacity: Vec<f64>,
    eigenvalues: Vec<f64>,
    eigenvectors: Vec<Vec<f64>>,
    modal_coefficients: Vec<f64>,
    max_eigenvalue: f64,
}

impl SpectralOracle {
    pub fn max_eigenvalue(&self) -> f64 {
        self.max_eigenvalue
    }

    pub fn eigenvalues(&self) -> &[f64] {
        &self.eigenvalues
    }

    pub fn state_at(&self, time: f64) -> Result<Vec<f64>, NetworkError> {
        if !time.is_finite() || time < 0.0 {
            return Err(NetworkError::InvalidOracleTime);
        }
        let n = self.capacities.len();
        let mut transformed = vec![0.0; n];

        for node in 0..n {
            for mode in 0..n {
                transformed[node] += self.eigenvectors[node][mode]
                    * self.modal_coefficients[mode]
                    * (-self.eigenvalues[mode] * time).exp();
            }
        }

        let state: Vec<f64> = transformed
            .iter()
            .zip(self.sqrt_capacity.iter())
            .map(|(value, scale)| value / scale)
            .collect();

        if state.iter().all(|value| value.is_finite()) {
            Ok(state)
        } else {
            Err(NetworkError::NonFiniteOracleState)
        }
    }
}

/// Deterministic integration/coupling candidates for network exchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMethod {
    /// Compute every edge flux from one old-state snapshot, then apply all updates.
    JacobiSharedFluxEuler,
    /// Apply one conservative equal-and-opposite transfer per edge in stored order.
    SerialForwardEdgeSweep,
    /// Apply one conservative equal-and-opposite transfer per edge in reverse order.
    SerialReverseEdgeSweep,
    /// Monolithic implicit midpoint reference for the frozen linear graph.
    ImplicitMidpointReference,
    /// Monolithic backward Euler reference for the frozen linear graph.
    BackwardEulerReference,
}

impl NetworkMethod {
    pub const ALL: [Self; 5] = [
        Self::JacobiSharedFluxEuler,
        Self::SerialForwardEdgeSweep,
        Self::SerialReverseEdgeSweep,
        Self::ImplicitMidpointReference,
        Self::BackwardEulerReference,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::JacobiSharedFluxEuler => "jacobi_shared_flux_euler",
            Self::SerialForwardEdgeSweep => "serial_forward_edge_sweep",
            Self::SerialReverseEdgeSweep => "serial_reverse_edge_sweep",
            Self::ImplicitMidpointReference => "implicit_midpoint_reference",
            Self::BackwardEulerReference => "backward_euler_reference",
        }
    }
}

/// Separate numerical and physical-bookkeeping metrics for one network run.
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkScore {
    pub steps: usize,
    pub max_abs_state_error: f64,
    pub rms_state_error: f64,
    pub max_abs_total_residual: f64,
    pub max_quadratic_energy_increase: f64,
    pub min_state: f64,
    pub max_state: f64,
    pub max_spectral_stability_number: f64,
    pub positive_states: bool,
    pub final_state: Vec<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    InvalidNodeCount,
    InvalidHorizon,
    InvalidCapacity { node: usize },
    NonFiniteInitialState { node: usize },
    UnknownNode { edge: usize },
    SelfEdge { edge: usize },
    InvalidConductance { edge: usize },
    DuplicateEdge { left: usize, right: usize },
    InvalidOracleTime,
    NonFiniteOracleState,
    NonPositiveSemidefiniteLaplacian,
    SpectralEigensolverDidNotConverge,
    InvalidStep,
    StepDoesNotDivideHorizon,
    ExcessiveStepCount,
    SingularLinearSystem,
    NonFiniteState { step: usize },
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidNodeCount => write!(
                f,
                "network needs at least two nodes and matching initial states"
            ),
            Self::InvalidHorizon => write!(f, "network horizon must be finite and positive"),
            Self::InvalidCapacity { node } => {
                write!(f, "capacity at node {node} must be finite and positive")
            }
            Self::NonFiniteInitialState { node } => {
                write!(f, "initial state at node {node} must be finite")
            }
            Self::UnknownNode { edge } => write!(f, "edge {edge} references an unknown node"),
            Self::SelfEdge { edge } => write!(f, "edge {edge} cannot connect a node to itself"),
            Self::InvalidConductance { edge } => write!(
                f,
                "conductance on edge {edge} must be finite and nonnegative"
            ),
            Self::DuplicateEdge { left, right } => {
                write!(f, "duplicate undirected edge ({left}, {right})")
            }
            Self::InvalidOracleTime => write!(f, "oracle time must be finite and nonnegative"),
            Self::NonFiniteOracleState => write!(f, "spectral oracle produced a non-finite state"),
            Self::NonPositiveSemidefiniteLaplacian => write!(
                f,
                "weighted Laplacian is not positive semidefinite within tolerance"
            ),
            Self::SpectralEigensolverDidNotConverge => {
                write!(f, "symmetric Jacobi eigensolver did not converge")
            }
            Self::InvalidStep => write!(f, "communication step must be finite and positive"),
            Self::StepDoesNotDivideHorizon => {
                write!(f, "communication step must divide the horizon")
            }
            Self::ExcessiveStepCount => write!(f, "network step count exceeds the safety limit"),
            Self::SingularLinearSystem => {
                write!(f, "implicit network reference system is singular")
            }
            Self::NonFiniteState { step } => write!(f, "non-finite network state at step {step}"),
        }
    }
}

impl std::error::Error for NetworkError {}

/// Integrate one graph under a fixed communication step and score against
/// the spectral oracle. No participant state is committed if a step is invalid.
pub fn simulate_network(
    scenario: &NetworkScenario,
    method: NetworkMethod,
    communication_step: f64,
) -> Result<NetworkScore, NetworkError> {
    scenario.validate()?;
    if !communication_step.is_finite() || communication_step <= 0.0 {
        return Err(NetworkError::InvalidStep);
    }

    let raw_steps = scenario.horizon / communication_step;
    let rounded_steps = raw_steps.round();
    let tolerance = 1.0e-10 * raw_steps.abs().max(1.0);
    if !raw_steps.is_finite() || (raw_steps - rounded_steps).abs() > tolerance {
        return Err(NetworkError::StepDoesNotDivideHorizon);
    }
    if !(1.0..=10_000_000.0).contains(&rounded_steps) {
        return Err(NetworkError::ExcessiveStepCount);
    }

    let oracle = scenario.spectral_oracle()?;
    let laplacian = scenario.laplacian();
    let steps = rounded_steps as usize;
    let n = scenario.capacities.len();
    let mut state = scenario.initial.clone();
    let initial_total = scenario.conserved_total();
    let mut previous_energy = scenario.quadratic_energy(&state);

    let mut max_abs_state_error: f64 = 0.0;
    let mut sum_squared_error = 0.0;
    let mut error_samples = 0_usize;
    let mut max_abs_total_residual: f64 = 0.0;
    let mut max_quadratic_energy_increase: f64 = 0.0;
    let mut min_state = state.iter().copied().fold(f64::INFINITY, f64::min);
    let mut max_state = state.iter().copied().fold(f64::NEG_INFINITY, f64::max);

    for index in 0..steps {
        let next = match method {
            NetworkMethod::JacobiSharedFluxEuler => {
                let mut increments = vec![0.0; n];
                for edge in &scenario.edges {
                    let flux = edge.conductance
                        * (state[edge.left] - state[edge.right])
                        * communication_step;
                    increments[edge.left] -= flux / scenario.capacities[edge.left];
                    increments[edge.right] += flux / scenario.capacities[edge.right];
                }
                state
                    .iter()
                    .zip(increments.iter())
                    .map(|(value, delta)| value + delta)
                    .collect()
            }
            NetworkMethod::SerialForwardEdgeSweep => {
                let mut next = state.clone();
                for edge in &scenario.edges {
                    apply_edge_flux(scenario, &mut next, *edge, communication_step);
                }
                next
            }
            NetworkMethod::SerialReverseEdgeSweep => {
                let mut next = state.clone();
                for edge in scenario.edges.iter().rev() {
                    apply_edge_flux(scenario, &mut next, *edge, communication_step);
                }
                next
            }
            NetworkMethod::ImplicitMidpointReference => {
                let lhs = build_shifted_matrix(
                    &scenario.capacities,
                    &laplacian,
                    0.5 * communication_step,
                );
                let rhs_matrix = build_shifted_matrix(
                    &scenario.capacities,
                    &laplacian,
                    -0.5 * communication_step,
                );
                let rhs = matrix_vector_product(&rhs_matrix, &state);
                solve_dense(lhs, rhs)?
            }
            NetworkMethod::BackwardEulerReference => {
                let lhs =
                    build_shifted_matrix(&scenario.capacities, &laplacian, communication_step);
                let rhs: Vec<f64> = scenario
                    .capacities
                    .iter()
                    .zip(state.iter())
                    .map(|(capacity, value)| capacity * value)
                    .collect();
                solve_dense(lhs, rhs)?
            }
        };

        if next.len() != n || next.iter().any(|value| !value.is_finite()) {
            return Err(NetworkError::NonFiniteState { step: index + 1 });
        }
        state = next;

        let time = (index + 1) as f64 * communication_step;
        let expected = oracle.state_at(time)?;
        for (actual, reference) in state.iter().zip(expected.iter()) {
            let error = actual - reference;
            max_abs_state_error = max_abs_state_error.max(error.abs());
            sum_squared_error += error * error;
            error_samples += 1;
        }

        let total: f64 = scenario
            .capacities
            .iter()
            .zip(state.iter())
            .map(|(capacity, value)| capacity * value)
            .sum();
        max_abs_total_residual = max_abs_total_residual.max((total - initial_total).abs());

        let energy = scenario.quadratic_energy(&state);
        max_quadratic_energy_increase =
            max_quadratic_energy_increase.max((energy - previous_energy).max(0.0));
        previous_energy = energy;

        min_state = min_state.min(state.iter().copied().fold(f64::INFINITY, f64::min));
        max_state = max_state.max(state.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    }

    Ok(NetworkScore {
        steps,
        max_abs_state_error,
        rms_state_error: (sum_squared_error / error_samples as f64).sqrt(),
        max_abs_total_residual,
        max_quadratic_energy_increase,
        min_state,
        max_state,
        max_spectral_stability_number: oracle.max_eigenvalue() * communication_step,
        positive_states: min_state >= 0.0,
        final_state: state,
    })
}

fn apply_edge_flux(scenario: &NetworkScenario, state: &mut [f64], edge: Edge, step: f64) {
    let flux = edge.conductance * (state[edge.left] - state[edge.right]) * step;
    state[edge.left] -= flux / scenario.capacities[edge.left];
    state[edge.right] += flux / scenario.capacities[edge.right];
}

fn build_shifted_matrix(capacities: &[f64], laplacian: &[Vec<f64>], scale: f64) -> Vec<Vec<f64>> {
    let n = capacities.len();
    let mut matrix = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            matrix[i][j] = scale * laplacian[i][j];
        }
        matrix[i][i] += capacities[i];
    }
    matrix
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

/// Dense Gaussian elimination with partial pivoting, used only for tiny
/// synthetic oracle/reference systems.
fn solve_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Result<Vec<f64>, NetworkError> {
    let n = rhs.len();
    for pivot in 0..n {
        let mut best = pivot;
        for row in (pivot + 1)..n {
            if matrix[row][pivot].abs() > matrix[best][pivot].abs() {
                best = row;
            }
        }
        if matrix[best][pivot].abs() <= 1.0e-14 {
            return Err(NetworkError::SingularLinearSystem);
        }
        matrix.swap(pivot, best);
        rhs.swap(pivot, best);

        for row in (pivot + 1)..n {
            let factor = matrix[row][pivot] / matrix[pivot][pivot];
            matrix[row][pivot] = 0.0;
            for column in (pivot + 1)..n {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }

    let mut result = vec![0.0; n];
    for row in (0..n).rev() {
        let tail: f64 = ((row + 1)..n)
            .map(|column| matrix[row][column] * result[column])
            .sum();
        result[row] = (rhs[row] - tail) / matrix[row][row];
    }
    if result.iter().all(|value| value.is_finite()) {
        Ok(result)
    } else {
        Err(NetworkError::SingularLinearSystem)
    }
}

/// Jacobi diagonalization for a real symmetric matrix.
/// Eigenvectors are returned by columns of the matrix.
fn symmetric_eigen(mut matrix: Vec<Vec<f64>>) -> Result<(Vec<f64>, Vec<Vec<f64>>), NetworkError> {
    let n = matrix.len();
    let mut vectors = vec![vec![0.0; n]; n];
    for i in 0..n {
        vectors[i][i] = 1.0;
    }

    let max_iterations = 100_usize.saturating_mul(n).saturating_mul(n).max(1);
    let mut converged = false;

    for _ in 0..max_iterations {
        let mut p = 0;
        let mut q = 0;
        let mut max_off_diagonal = 0.0_f64;
        let mut scale = 1.0_f64;

        for i in 0..n {
            scale = scale.max(matrix[i][i].abs());
            for j in (i + 1)..n {
                if matrix[i][j].abs() > max_off_diagonal {
                    max_off_diagonal = matrix[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }

        if max_off_diagonal <= 1.0e-13 * scale {
            converged = true;
            break;
        }

        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;

        matrix[p][p] = app - t * apq;
        matrix[q][q] = aqq + t * apq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for k in 0..n {
            if k != p && k != q {
                let akp = matrix[k][p];
                let akq = matrix[k][q];
                let new_kp = cosine * akp - sine * akq;
                let new_kq = sine * akp + cosine * akq;
                matrix[k][p] = new_kp;
                matrix[p][k] = new_kp;
                matrix[k][q] = new_kq;
                matrix[q][k] = new_kq;
            }

            let vip = vectors[k][p];
            let viq = vectors[k][q];
            vectors[k][p] = cosine * vip - sine * viq;
            vectors[k][q] = sine * vip + cosine * viq;
        }
    }

    if !converged {
        return Err(NetworkError::SpectralEigensolverDidNotConverge);
    }

    let eigenvalues = (0..n).map(|i| matrix[i][i]).collect();
    Ok((eigenvalues, vectors))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chain() -> NetworkScenario {
        NetworkScenario {
            name: "three_node_heterogeneous_chain",
            capacities: vec![1.0, 2.0, 4.0],
            initial: vec![1.0, 0.2, 0.0],
            edges: vec![
                Edge {
                    left: 0,
                    right: 1,
                    conductance: 0.8,
                },
                Edge {
                    left: 1,
                    right: 2,
                    conductance: 0.2,
                },
            ],
            horizon: 4.0,
        }
    }

    #[test]
    fn spectral_oracle_matches_initial_state_and_conserves_weighted_total() {
        let scenario = chain();
        let oracle = scenario.spectral_oracle().unwrap();
        let initial = oracle.state_at(0.0).unwrap();
        for (actual, expected) in initial.iter().zip(scenario.initial.iter()) {
            assert!((actual - expected).abs() < 1.0e-11);
        }

        let total = scenario.conserved_total();
        for time in [0.0, 0.1, 0.5, 1.0, 2.0, 4.0, 20.0] {
            let state = oracle.state_at(time).unwrap();
            let candidate: f64 = scenario
                .capacities
                .iter()
                .zip(state.iter())
                .map(|(capacity, value)| capacity * value)
                .sum();
            assert!((candidate - total).abs() < 1.0e-10);
            assert!(state.iter().all(|value| *value >= -1.0e-12));
        }
    }

    #[test]
    fn exact_quadratic_energy_is_non_increasing() {
        let scenario = chain();
        let oracle = scenario.spectral_oracle().unwrap();
        let mut previous = scenario.quadratic_energy(&scenario.initial);
        for index in 1..=100 {
            let state = oracle
                .state_at(index as f64 * scenario.horizon / 100.0)
                .unwrap();
            let energy = scenario.quadratic_energy(&state);
            assert!(energy <= previous + 1.0e-11);
            previous = energy;
        }
    }

    #[test]
    fn jacobi_shared_flux_preserves_total_at_a_stable_step() {
        let scenario = chain();
        let score =
            simulate_network(&scenario, NetworkMethod::JacobiSharedFluxEuler, 0.01).unwrap();
        assert!(score.max_abs_total_residual < 1.0e-11);
        assert!(score.max_quadratic_energy_increase < 1.0e-11);
        assert!(score.positive_states);
    }

    #[test]
    fn sequential_edge_order_changes_trajectory_but_preserves_total() {
        let scenario = chain();
        let forward =
            simulate_network(&scenario, NetworkMethod::SerialForwardEdgeSweep, 0.5).unwrap();
        let reverse =
            simulate_network(&scenario, NetworkMethod::SerialReverseEdgeSweep, 0.5).unwrap();
        assert!(forward.max_abs_total_residual < 1.0e-11);
        assert!(reverse.max_abs_total_residual < 1.0e-11);
        let difference: f64 = forward
            .final_state
            .iter()
            .zip(reverse.final_state.iter())
            .map(|(a, b)| (a - b).abs())
            .sum();
        assert!(difference > 1.0e-8);
    }

    #[test]
    fn implicit_references_preserve_total_and_positive_state_on_large_step() {
        let scenario = chain();
        let backward =
            simulate_network(&scenario, NetworkMethod::BackwardEulerReference, 4.0).unwrap();
        assert!(backward.max_abs_total_residual < 1.0e-10);
        assert!(backward.positive_states);
        assert!(backward.min_state >= -1.0e-12);

        let midpoint =
            simulate_network(&scenario, NetworkMethod::ImplicitMidpointReference, 4.0).unwrap();
        assert!(midpoint.max_abs_total_residual < 1.0e-10);
    }

    #[test]
    fn duplicate_edges_are_rejected_before_simulation() {
        let mut scenario = chain();
        scenario.edges.push(scenario.edges[0]);
        assert_eq!(
            scenario.validate(),
            Err(NetworkError::DuplicateEdge { left: 0, right: 1 })
        );
    }

    #[test]
    fn disconnected_components_have_multiple_zero_modes() {
        let scenario = NetworkScenario {
            name: "two_disconnected_pairs",
            capacities: vec![1.0, 2.0, 3.0, 4.0],
            initial: vec![1.0, 0.0, 0.5, 0.25],
            edges: vec![
                Edge {
                    left: 0,
                    right: 1,
                    conductance: 1.0,
                },
                Edge {
                    left: 2,
                    right: 3,
                    conductance: 0.5,
                },
            ],
            horizon: 2.0,
        };
        let oracle = scenario.spectral_oracle().unwrap();
        let zero_modes = oracle
            .eigenvalues()
            .iter()
            .filter(|value| value.abs() < 1.0e-10)
            .count();
        assert_eq!(zero_modes, 2);
    }

    #[test]
    fn invalid_step_and_non_dividing_horizon_reject() {
        let scenario = chain();
        assert_eq!(
            simulate_network(&scenario, NetworkMethod::BackwardEulerReference, f64::NAN),
            Err(NetworkError::InvalidStep)
        );
        assert_eq!(
            simulate_network(&scenario, NetworkMethod::BackwardEulerReference, 0.3),
            Err(NetworkError::StepDoesNotDivideHorizon)
        );
    }
}
