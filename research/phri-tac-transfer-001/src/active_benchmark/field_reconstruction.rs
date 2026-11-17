//! Synthetic mechanics-kernel inverse reconstruction for distributed tactile fields.
//!
//! This module solves a small nonnegative inverse problem:
//!
//! ```text
//! y = K x + noise
//! ```
//!
//! where y is the sensor-native, per-taxel response after the upstream
//! calibration/observability gates, K is a declared mechanical coupling
//! operator, and x is a latent contact-field intensity. Optional graph
//! regularization is restricted to explicitly supplied physical adjacency
//! edges; callers must omit edges across module seams or mechanical
//! discontinuities.
//!
//! This is a synthetic research primitive. It does not establish physical
//! pressure units, contact mechanics, calibration, a force/moment balance,
//! uncertainty coverage, or a qualified measurement. Missing native channels
//! are never labelled as direct observations. A latent value inferred from
//! neighboring channels remains model-derived.

#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldCellSupport {
    /// A native sensor channel at this taxel location was admitted to the
    /// inverse problem. The reconstructed value is still model-conditioned.
    NativeChannelAvailable,
    /// No native channel at this location was admitted, but another admitted
    /// channel has nonzero sensitivity to this field cell through K.
    InferredThroughMechanicalCoupling,
    /// No admitted channel observes this field cell under the supplied K.
    /// The inverse solver must not invent a numerical value for it.
    NoObservableSupport,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldCellEstimate {
    pub taxel_index: usize,
    pub latent_contact_intensity: Option<f64>,
    pub support: FieldCellSupport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpatialEdge {
    pub left_taxel: usize,
    pub right_taxel: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldReconstructionProblem {
    /// Sensor-native calibrated response, one value per taxel channel.
    /// Upstream reference and observability qualification is the caller's job.
    pub native_response: Vec<f64>,
    /// Row-major n × n mechanical coupling operator.
    /// K[row, column] is the response in channel row to unit field intensity
    /// at field cell column.
    pub coupling_matrix: Vec<f64>,
    /// False excludes a row (for example, saturated, dead, or invalid channel).
    pub admitted_native_channels: Vec<bool>,
    /// As-built mechanical adjacency only. Do not connect across seams or
    /// mechanically isolated regions.
    pub spatial_edges: Vec<SpatialEdge>,
    /// Nonnegative graph-Laplacian penalty. Zero disables spatial smoothing.
    pub smoothness_penalty: f64,
    /// Stable digest/reference for the supplied forward model.
    pub forward_model_digest: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldReconstructionConfig {
    pub maximum_iterations: usize,
    pub convergence_tolerance: f64,
    pub sensitivity_epsilon: f64,
}

impl Default for FieldReconstructionConfig {
    fn default() -> Self {
        Self {
            maximum_iterations: 20_000,
            convergence_tolerance: 1.0e-10,
            sensitivity_epsilon: 1.0e-12,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldReconstructionReport {
    pub taxel_count: usize,
    pub admitted_native_channel_count: usize,
    pub inferred_cell_count: usize,
    pub unsupported_cell_count: usize,
    pub estimates: Vec<FieldCellEstimate>,
    /// Predicted native response from the fitted field; entries for
    /// non-admitted channels are still forward predictions, not observations.
    pub predicted_native_response: Vec<f64>,
    pub residual_rms_native_units: f64,
    pub regularized_objective: f64,
    pub iterations: usize,
    pub converged: bool,
    pub forward_model_digest: String,
    pub claim_class: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
struct QuadraticProblem {
    hessian: Vec<f64>,
    rhs: Vec<f64>,
    taxel_count: usize,
    lipschitz_upper_bound: f64,
}

fn validate_problem(
    problem: &FieldReconstructionProblem,
    config: &FieldReconstructionConfig,
) -> Result<usize, String> {
    let n = problem.native_response.len();
    let matrix_len = n
        .checked_mul(n)
        .ok_or_else(|| "coupling matrix dimensions overflow".to_owned())?;
    if n == 0
        || problem.coupling_matrix.len() != matrix_len
        || problem.admitted_native_channels.len() != n
    {
        return Err(
            "field problem requires a nonempty square coupling matrix and matching channel vectors"
                .to_owned(),
        );
    }
    if problem.forward_model_digest.trim().is_empty() {
        return Err("forward-model digest must not be empty".to_owned());
    }
    if problem
        .native_response
        .iter()
        .any(|value| !value.is_finite())
        || problem
            .coupling_matrix
            .iter()
            .any(|value| !value.is_finite())
    {
        return Err("native response and coupling matrix must be finite".to_owned());
    }
    if !problem.smoothness_penalty.is_finite() || problem.smoothness_penalty < 0.0 {
        return Err("smoothness penalty must be finite and nonnegative".to_owned());
    }
    if config.maximum_iterations == 0
        || !config.convergence_tolerance.is_finite()
        || config.convergence_tolerance <= 0.0
        || !config.sensitivity_epsilon.is_finite()
        || config.sensitivity_epsilon < 0.0
    {
        return Err("solver configuration requires positive iteration/tolerance and nonnegative finite sensitivity epsilon".to_owned());
    }
    if !problem
        .admitted_native_channels
        .iter()
        .any(|admitted| *admitted)
    {
        return Err(
            "field reconstruction requires at least one admitted native channel".to_owned(),
        );
    }

    let mut seen_edges = std::collections::BTreeSet::new();
    for edge in &problem.spatial_edges {
        if edge.left_taxel >= n || edge.right_taxel >= n || edge.left_taxel == edge.right_taxel {
            return Err("spatial edge contains an invalid taxel index".to_owned());
        }
        let normalized = if edge.left_taxel < edge.right_taxel {
            (edge.left_taxel, edge.right_taxel)
        } else {
            (edge.right_taxel, edge.left_taxel)
        };
        if !seen_edges.insert(normalized) {
            return Err("spatial graph contains duplicate undirected edges".to_owned());
        }
    }

    Ok(n)
}

fn build_quadratic(
    problem: &FieldReconstructionProblem,
    n: usize,
) -> Result<QuadraticProblem, String> {
    // H = Kᵀ M K + λ L; b = Kᵀ M y, where M removes unadmitted rows.
    // The objective is 1/2 xᵀ H x - bᵀ x + constant.
    let mut hessian = vec![0.0; n * n];
    let mut rhs = vec![0.0; n];

    for row in 0..n {
        if !problem.admitted_native_channels[row] {
            continue;
        }
        for left in 0..n {
            let k_left = problem.coupling_matrix[row * n + left];
            rhs[left] += k_left * problem.native_response[row];
            for right in 0..n {
                hessian[left * n + right] += k_left * problem.coupling_matrix[row * n + right];
            }
        }
    }

    // Add λ DᵀD for each declared undirected physical adjacency edge.
    for edge in &problem.spatial_edges {
        let left = edge.left_taxel;
        let right = edge.right_taxel;
        let lambda = problem.smoothness_penalty;
        hessian[left * n + left] += lambda;
        hessian[right * n + right] += lambda;
        hessian[left * n + right] -= lambda;
        hessian[right * n + left] -= lambda;
    }

    if hessian.iter().any(|value| !value.is_finite()) || rhs.iter().any(|value| !value.is_finite())
    {
        return Err("normal-equation construction produced non-finite values".to_owned());
    }

    // H is symmetric positive semidefinite. Its maximum absolute row sum is
    // a conservative upper bound on the spectral norm and therefore a safe
    // Lipschitz bound for the quadratic gradient.
    let lipschitz_upper_bound = (0..n)
        .map(|row| {
            hessian[row * n..(row + 1) * n]
                .iter()
                .map(|value| value.abs())
                .sum::<f64>()
        })
        .fold(0.0_f64, f64::max);

    if !lipschitz_upper_bound.is_finite() || lipschitz_upper_bound <= f64::EPSILON {
        return Err(
            "admitted channels and forward model provide no invertible field information"
                .to_owned(),
        );
    }

    Ok(QuadraticProblem {
        hessian,
        rhs,
        taxel_count: n,
        lipschitz_upper_bound,
    })
}

fn gradient(problem: &QuadraticProblem, field: &[f64]) -> Vec<f64> {
    let n = problem.taxel_count;
    (0..n)
        .map(|row| {
            let row_start = row * n;
            problem.hessian[row_start..row_start + n]
                .iter()
                .zip(field)
                .map(|(coefficient, value)| coefficient * value)
                .sum::<f64>()
                - problem.rhs[row]
        })
        .collect()
}

fn objective(problem: &FieldReconstructionProblem, field: &[f64], n: usize) -> f64 {
    let mut residual_sum = 0.0;
    for row in 0..n {
        if !problem.admitted_native_channels[row] {
            continue;
        }
        let prediction = (0..n)
            .map(|column| problem.coupling_matrix[row * n + column] * field[column])
            .sum::<f64>();
        let residual = prediction - problem.native_response[row];
        residual_sum += residual * residual;
    }

    let smoothness_sum = problem
        .spatial_edges
        .iter()
        .map(|edge| {
            let difference = field[edge.left_taxel] - field[edge.right_taxel];
            difference * difference
        })
        .sum::<f64>();

    0.5 * residual_sum + 0.5 * problem.smoothness_penalty * smoothness_sum
}

fn predicted_response(coupling_matrix: &[f64], field: &[f64], n: usize) -> Vec<f64> {
    (0..n)
        .map(|row| {
            coupling_matrix[row * n..(row + 1) * n]
                .iter()
                .zip(field)
                .map(|(coefficient, value)| coefficient * value)
                .sum()
        })
        .collect()
}

fn cell_support(
    problem: &FieldReconstructionProblem,
    n: usize,
    column: usize,
    sensitivity_epsilon: f64,
) -> FieldCellSupport {
    if problem.admitted_native_channels[column] {
        return FieldCellSupport::NativeChannelAvailable;
    }

    let has_admitted_sensitivity = (0..n).any(|row| {
        problem.admitted_native_channels[row]
            && problem.coupling_matrix[row * n + column].abs() > sensitivity_epsilon
    });

    if has_admitted_sensitivity {
        FieldCellSupport::InferredThroughMechanicalCoupling
    } else {
        FieldCellSupport::NoObservableSupport
    }
}

/// Reconstruct a nonnegative latent contact-field intensity using projected
/// gradient descent on a convex quadratic objective.
///
/// This first version uses a fixed, caller-supplied mechanical coupling
/// operator and optional physical-adjacency Laplacian regularization. It
/// explicitly marks cells with no admitted native sensitivity as unsupported.
/// The result is never a qualified force/pressure measurement.
pub fn reconstruct_contact_field(
    problem: &FieldReconstructionProblem,
    config: &FieldReconstructionConfig,
) -> Result<FieldReconstructionReport, String> {
    let n = validate_problem(problem, config)?;
    let quadratic = build_quadratic(problem, n)?;
    let step_size = 1.0 / quadratic.lipschitz_upper_bound;

    // Deterministic zero initialization is conservative for a nonnegative
    // contact-intensity field. Projection prevents negative pressure-like
    // artifacts from the inverse solver.
    let mut field = vec![0.0; n];
    let mut converged = false;
    let mut iterations = 0;

    for iteration in 0..config.maximum_iterations {
        let gradient = gradient(&quadratic, &field);
        let mut maximum_delta = 0.0_f64;
        let mut maximum_field = 0.0_f64;

        for index in 0..n {
            let updated = (field[index] - step_size * gradient[index]).max(0.0);
            maximum_delta = maximum_delta.max((updated - field[index]).abs());
            maximum_field = maximum_field.max(updated.abs());
            field[index] = updated;
        }

        iterations = iteration + 1;
        if maximum_delta <= config.convergence_tolerance * (1.0 + maximum_field) {
            converged = true;
            break;
        }
    }

    if field.iter().any(|value| !value.is_finite() || *value < 0.0) {
        return Err("projected solver produced a non-finite or negative field value".to_owned());
    }

    let predicted_native_response = predicted_response(&problem.coupling_matrix, &field, n);
    let mut residual_sum = 0.0;
    let mut admitted_count = 0;
    for (row, predicted) in predicted_native_response.iter().enumerate() {
        if problem.admitted_native_channels[row] {
            let residual = predicted - problem.native_response[row];
            residual_sum += residual * residual;
            admitted_count += 1;
        }
    }

    let mut inferred_cell_count = 0;
    let mut unsupported_cell_count = 0;
    let estimates: Vec<_> = (0..n)
        .map(|taxel_index| {
            let support = cell_support(problem, n, taxel_index, config.sensitivity_epsilon);
            let latent_contact_intensity = match support {
                FieldCellSupport::NativeChannelAvailable => Some(field[taxel_index]),
                FieldCellSupport::InferredThroughMechanicalCoupling => {
                    inferred_cell_count += 1;
                    Some(field[taxel_index])
                }
                FieldCellSupport::NoObservableSupport => {
                    unsupported_cell_count += 1;
                    None
                }
            };
            FieldCellEstimate {
                taxel_index,
                latent_contact_intensity,
                support,
            }
        })
        .collect();

    Ok(FieldReconstructionReport {
        taxel_count: n,
        admitted_native_channel_count: admitted_count,
        inferred_cell_count,
        unsupported_cell_count,
        estimates,
        predicted_native_response,
        residual_rms_native_units: (residual_sum / admitted_count as f64).sqrt(),
        regularized_objective: objective(problem, &field, n),
        iterations,
        converged,
        forward_model_digest: problem.forward_model_digest.clone(),
        claim_class: "synthetic_mechanics_inverse_model_only",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_problem(response: Vec<f64>) -> FieldReconstructionProblem {
        let n = response.len();
        let mut matrix = vec![0.0; n * n];
        for index in 0..n {
            matrix[index * n + index] = 1.0;
        }
        FieldReconstructionProblem {
            native_response: response,
            coupling_matrix: matrix,
            admitted_native_channels: vec![true; n],
            spatial_edges: Vec::new(),
            smoothness_penalty: 0.0,
            forward_model_digest: "sha256:synthetic-identity-kernel".to_owned(),
        }
    }

    #[test]
    fn identity_kernel_recovers_a_known_nonnegative_field() {
        let expected = vec![0.0, 0.25, 1.0, 0.5, 0.0];
        let report = reconstruct_contact_field(
            &identity_problem(expected.clone()),
            &FieldReconstructionConfig::default(),
        )
        .expect("identity inverse");
        assert!(report.converged);
        assert_eq!(report.admitted_native_channel_count, 5);
        assert!(
            report
                .estimates
                .iter()
                .all(|cell| { cell.support == FieldCellSupport::NativeChannelAvailable })
        );
        for (estimate, truth) in report.estimates.iter().zip(expected) {
            let value = estimate.latent_contact_intensity.expect("supported cell");
            assert!((value - truth).abs() < 1.0e-7);
        }
        assert!(report.residual_rms_native_units < 1.0e-8);
    }

    #[test]
    fn coupled_kernel_recovers_a_known_field_with_nonzero_neighbor_response() {
        // Symmetric, strictly diagonally dominant tridiagonal kernel.
        let n = 4;
        let matrix = vec![
            0.8, 0.2, 0.0, 0.0, 0.2, 0.6, 0.2, 0.0, 0.0, 0.2, 0.6, 0.2, 0.0, 0.0, 0.2, 0.8,
        ];
        let truth = vec![1.0, 0.0, 0.75, 0.25];
        let response = (0..n)
            .map(|row| {
                (0..n)
                    .map(|column| matrix[row * n + column] * truth[column])
                    .sum()
            })
            .collect();
        let mut problem = identity_problem(response);
        problem.coupling_matrix = matrix;
        let report = reconstruct_contact_field(&problem, &FieldReconstructionConfig::default())
            .expect("coupled inverse");
        assert!(report.converged);
        for (estimate, expected) in report.estimates.iter().zip(truth) {
            assert!(
                (estimate.latent_contact_intensity.expect("observed support") - expected).abs()
                    < 1.0e-6
            );
        }
    }

    #[test]
    fn projection_prevents_negative_contact_intensity() {
        let problem = identity_problem(vec![-0.5, 0.2, -1.0]);
        let report = reconstruct_contact_field(&problem, &FieldReconstructionConfig::default())
            .expect("nonnegative projection");
        assert!(report.converged);
        assert_eq!(
            report
                .estimates
                .iter()
                .map(|cell| cell.latent_contact_intensity.expect("supported"))
                .collect::<Vec<_>>(),
            vec![0.0, 0.2, 0.0]
        );
    }

    #[test]
    fn missing_sensor_channel_is_not_labelled_as_a_direct_measurement() {
        let mut problem = identity_problem(vec![0.2, 0.4, 0.6]);
        problem.admitted_native_channels[1] = false;
        problem.coupling_matrix = vec![0.8, 0.2, 0.0, 0.2, 0.6, 0.2, 0.0, 0.2, 0.8];
        let report = reconstruct_contact_field(&problem, &FieldReconstructionConfig::default())
            .expect("partial channels");
        assert_eq!(
            report.estimates[1].support,
            FieldCellSupport::InferredThroughMechanicalCoupling
        );
        assert_eq!(report.inferred_cell_count, 1);
        assert_eq!(report.unsupported_cell_count, 0);
    }

    #[test]
    fn unsupported_field_cell_returns_none_not_a_neighbor_fabricated_value() {
        let mut problem = identity_problem(vec![0.4, 0.0, 0.0]);
        problem.admitted_native_channels = vec![true, false, false];
        problem.coupling_matrix = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let report = reconstruct_contact_field(&problem, &FieldReconstructionConfig::default())
            .expect("partially observable field");
        assert_eq!(
            report.estimates[1].support,
            FieldCellSupport::NoObservableSupport
        );
        assert_eq!(
            report.estimates[2].support,
            FieldCellSupport::NoObservableSupport
        );
        assert_eq!(report.estimates[1].latent_contact_intensity, None);
        assert_eq!(report.estimates[2].latent_contact_intensity, None);
    }

    #[test]
    fn graph_regularization_uses_only_declared_physical_edges() {
        let mut problem = identity_problem(vec![1.0, 0.0, 1.0, 0.0]);
        problem.spatial_edges = vec![
            SpatialEdge {
                left_taxel: 0,
                right_taxel: 1,
            },
            SpatialEdge {
                left_taxel: 2,
                right_taxel: 3,
            },
        ];
        problem.smoothness_penalty = 0.1;
        let report = reconstruct_contact_field(&problem, &FieldReconstructionConfig::default())
            .expect("graph regularized inverse");
        assert!(report.converged);
        // There is no edge between taxels 1 and 2: the seam is not smoothed.
        let left_seam_value = report.estimates[1].latent_contact_intensity.unwrap();
        let right_seam_value = report.estimates[2].latent_contact_intensity.unwrap();
        assert!(right_seam_value > left_seam_value);
    }

    #[test]
    fn no_admitted_channels_is_rejected() {
        let mut problem = identity_problem(vec![0.1, 0.2]);
        problem.admitted_native_channels = vec![false, false];
        assert!(
            reconstruct_contact_field(&problem, &FieldReconstructionConfig::default()).is_err()
        );
    }

    #[test]
    fn invalid_matrix_graph_and_solver_parameters_fail_closed() {
        let mut problem = identity_problem(vec![0.1, 0.2]);
        problem.coupling_matrix.pop();
        assert!(
            reconstruct_contact_field(&problem, &FieldReconstructionConfig::default()).is_err()
        );

        let mut problem = identity_problem(vec![0.1, 0.2]);
        problem.spatial_edges.push(SpatialEdge {
            left_taxel: 0,
            right_taxel: 0,
        });
        assert!(
            reconstruct_contact_field(&problem, &FieldReconstructionConfig::default()).is_err()
        );

        let mut problem = identity_problem(vec![0.1, 0.2]);
        problem.spatial_edges = vec![
            SpatialEdge {
                left_taxel: 0,
                right_taxel: 1,
            };
            2
        ];
        assert!(
            reconstruct_contact_field(&problem, &FieldReconstructionConfig::default()).is_err()
        );

        let problem = identity_problem(vec![0.1, 0.2]);
        let config = FieldReconstructionConfig {
            maximum_iterations: 0,
            ..FieldReconstructionConfig::default()
        };
        assert!(reconstruct_contact_field(&problem, &config).is_err());
    }
}
