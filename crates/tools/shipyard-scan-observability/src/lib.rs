//! Frame-equivariant observability analysis for point-to-plane registration.
//!
//! This kernel measures which rigid-body twist directions are constrained by
//! the supplied geometry. The point-displacement metric is not a covariance,
//! and the generalized spectrum is not a substitute for sensor calibration,
//! systematic-bias analysis, or a complete metrology uncertainty budget.

pub const DOF: usize = 6;
pub type Matrix6 = [[f64; DOF]; DOF];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Correspondence {
    /// Source point expressed in the declared registration reference frame.
    pub point: [f64; 3],
    /// Unit target-surface normal in the same frame.
    pub normal: [f64; 3],
    /// Non-negative statistical/geometric correspondence weight.
    pub weight: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegistrationInput {
    pub case_id: String,
    pub frame_id: String,
    pub correspondences: Vec<Correspondence>,
    /// Reported registration residual; deliberately not used to infer observability.
    pub reported_residual_rmse: f64,
    /// Dimensionless threshold for declaring generalized eigen-directions weak.
    pub weak_threshold: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservabilityClass {
    FullyObservable,
    PartiallyObservable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureReason {
    EmptyIdentity,
    EmptyCorrespondenceSet,
    NonFinitePoint,
    NonFiniteNormal,
    NonUnitNormal,
    NonPositiveOrNonFiniteWeight,
    InvalidResidual,
    InvalidWeakThreshold,
    NonFiniteMatrix,
    MatrixNotSymmetric,
    MetricNotPositiveDefinite,
    EigenSolverDidNotConverge,
    GeneralizedEigenvalueOutOfRange,
    InvalidDecisionIdentity,
    InvalidDecisionGradient,
    ZeroDecisionGradient,
    InvalidDecisionThreshold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecisionObservabilityClass {
    WithinWeakSensitivityLimit,
    ExceedsWeakSensitivityLimit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DecisionObservabilityAssessment {
    pub decision_id: String,
    /// Fraction of the dual-metric gradient norm lying in the weak subspace.
    /// This value is invariant under consistent SE(3) coordinate changes.
    pub normalized_weak_sensitivity: f64,
    pub maximum_accepted_weak_sensitivity: f64,
    pub class: DecisionObservabilityClass,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservabilityReport {
    pub case_id: String,
    pub frame_id: String,
    pub class: ObservabilityClass,
    pub eigenvalues: [f64; DOF],
    /// Number of generalized eigenvalues at or below weak_threshold.
    pub weak_direction_count: usize,
    /// Generalized eigenvectors in original twist coordinates, one vector per
    /// entry. These are a basis for the weak subspace, not six axis flags.
    pub weak_subspace_basis: Vec<[f64; DOF]>,
    /// Point-to-plane information matrix H. Its values depend on coordinates.
    pub information_matrix: Matrix6,
    /// Point-displacement metric M. This is a twist-space metric, not covariance.
    pub displacement_metric: Matrix6,
    pub reported_residual_rmse: f64,
    pub weak_threshold: f64,
    pub metric_semantics: &'static str,
    pub claim_ceiling: &'static str,
}

const NORMAL_TOLERANCE: f64 = 1.0e-6;
const METRIC_PIVOT_TOLERANCE: f64 = 1.0e-12;
const EIGEN_TOLERANCE: f64 = 1.0e-12;
const SPECTRUM_TOLERANCE: f64 = 1.0e-8;

pub fn analyze_registration(
    input: &RegistrationInput,
) -> Result<ObservabilityReport, FailureReason> {
    if input.case_id.trim().is_empty() || input.frame_id.trim().is_empty() {
        return Err(FailureReason::EmptyIdentity);
    }
    if input.correspondences.is_empty() {
        return Err(FailureReason::EmptyCorrespondenceSet);
    }
    if !input.reported_residual_rmse.is_finite() || input.reported_residual_rmse < 0.0 {
        return Err(FailureReason::InvalidResidual);
    }
    if !input.weak_threshold.is_finite()
        || input.weak_threshold <= 0.0
        || input.weak_threshold >= 1.0
    {
        return Err(FailureReason::InvalidWeakThreshold);
    }

    let mut information = zero_matrix();
    let mut metric = zero_matrix();
    for correspondence in &input.correspondences {
        if !all_finite(&correspondence.point) {
            return Err(FailureReason::NonFinitePoint);
        }
        if !all_finite(&correspondence.normal) {
            return Err(FailureReason::NonFiniteNormal);
        }
        if !correspondence.weight.is_finite() || correspondence.weight <= 0.0 {
            return Err(FailureReason::NonPositiveOrNonFiniteWeight);
        }
        let normal_norm = dot3(correspondence.normal, correspondence.normal).sqrt();
        if (normal_norm - 1.0).abs() > NORMAL_TOLERANCE {
            return Err(FailureReason::NonUnitNormal);
        }

        let jacobian = point_displacement_jacobian(correspondence.point);
        let mut normal_row = [0.0; DOF];
        for column in 0..DOF {
            for axis in 0..3 {
                normal_row[column] += correspondence.normal[axis] * jacobian[axis][column];
            }
        }

        for row in 0..DOF {
            for column in 0..DOF {
                information[row][column] +=
                    correspondence.weight * normal_row[row] * normal_row[column];
                for axis in 0..3 {
                    metric[row][column] += correspondence.weight
                        * jacobian[axis][row]
                        * jacobian[axis][column];
                }
            }
        }
    }

    analyze_matrices(
        &input.case_id,
        &input.frame_id,
        information,
        metric,
        input.reported_residual_rmse,
        input.weak_threshold,
    )
}

/// Analyze H and M directly. This is public so a caller can verify congruence
/// metamorphisms and independent matrix construction implementations.
pub fn analyze_matrices(
    case_id: &str,
    frame_id: &str,
    information: Matrix6,
    metric: Matrix6,
    reported_residual_rmse: f64,
    weak_threshold: f64,
) -> Result<ObservabilityReport, FailureReason> {
    if case_id.trim().is_empty() || frame_id.trim().is_empty() {
        return Err(FailureReason::EmptyIdentity);
    }
    if !reported_residual_rmse.is_finite() || reported_residual_rmse < 0.0 {
        return Err(FailureReason::InvalidResidual);
    }
    if !weak_threshold.is_finite() || weak_threshold <= 0.0 || weak_threshold >= 1.0 {
        return Err(FailureReason::InvalidWeakThreshold);
    }
    if !matrix_finite(&information) || !matrix_finite(&metric) {
        return Err(FailureReason::NonFiniteMatrix);
    }
    if !matrix_symmetric(&information, 1.0e-9) || !matrix_symmetric(&metric, 1.0e-9) {
        return Err(FailureReason::MatrixNotSymmetric);
    }

    // Diagonal equilibration makes the factorization less sensitive to the
    // rotational/translational coordinate scale (e.g. metres versus mm).
    let mut diagonal_scale = [0.0; DOF];
    for i in 0..DOF {
        if metric[i][i] <= 0.0 {
            return Err(FailureReason::MetricNotPositiveDefinite);
        }
        diagonal_scale[i] = 1.0 / metric[i][i].sqrt();
    }
    let scaled_h = diagonal_congruence(&information, diagonal_scale);
    let scaled_m = diagonal_congruence(&metric, diagonal_scale);
    let lower = cholesky(scaled_m).ok_or(FailureReason::MetricNotPositiveDefinite)?;
    let inverse_lower = inverse_lower_triangular(lower)
        .ok_or(FailureReason::MetricNotPositiveDefinite)?;
    let inverse_lower_t = transpose(inverse_lower);
    let whitened = multiply(
        multiply(inverse_lower, scaled_h),
        inverse_lower_t,
    );
    if !matrix_finite(&whitened) {
        return Err(FailureReason::NonFiniteMatrix);
    }

    let (mut values, eigenvectors) =
        jacobi_eigen_symmetric(whitened).ok_or(FailureReason::EigenSolverDidNotConverge)?;
    let mut order = [0usize, 1, 2, 3, 4, 5];
    order.sort_by(|a, b| values[*a].total_cmp(&values[*b]));
    let mut sorted_values = [0.0; DOF];
    let mut weak_basis = Vec::new();

    for (sorted_index, source_index) in order.iter().copied().enumerate() {
        let mut value = values[source_index];
        if value < -SPECTRUM_TOLERANCE || value > 1.0 + SPECTRUM_TOLERANCE {
            return Err(FailureReason::GeneralizedEigenvalueOutOfRange);
        }
        if value < 0.0 {
            value = 0.0;
        } else if value > 1.0 {
            value = 1.0;
        }
        sorted_values[sorted_index] = value;

        if value <= weak_threshold {
            // If C = L^-1 H L^-T and M_eq = L L^T, the original
            // generalized eigenvector is v = D L^-T z.
            let mut vector = [0.0; DOF];
            for row in 0..DOF {
                let mut component = 0.0;
                for column in 0..DOF {
                    component += inverse_lower_t[row][column]
                        * eigenvectors[column][source_index];
                }
                vector[row] = diagonal_scale[row] * component;
            }
            weak_basis.push(vector);
        }
    }
    values = sorted_values;
    let weak_direction_count = weak_basis.len();
    let class = if weak_direction_count == 0 {
        ObservabilityClass::FullyObservable
    } else {
        ObservabilityClass::PartiallyObservable
    };

    Ok(ObservabilityReport {
        case_id: case_id.to_owned(),
        frame_id: frame_id.to_owned(),
        class,
        eigenvalues: values,
        weak_direction_count,
        weak_subspace_basis: weak_basis,
        information_matrix: information,
        displacement_metric: metric,
        reported_residual_rmse,
        weak_threshold,
        metric_semantics: "POINT_DISPLACEMENT_METRIC_NOT_COVARIANCE",
        claim_ceiling: "SYNTHETIC_REGISTRATION_OBSERVABILITY_ONLY",
    })
}

/// Assess whether the weak registration subspace materially affects one
/// downstream scalar decision. The supplied gradient is a covector with
/// respect to the declared twist coordinates. The result is local and
/// first-order; it is not a bound on nonlinear geometry/model discrepancy.
///
/// If x_old = B x_new, callers must transform the decision gradient as
/// g_new = B^T g_old. The normalized score is invariant under that mapping.
pub fn assess_decision_observability(
    report: &ObservabilityReport,
    decision_id: &str,
    decision_gradient: [f64; DOF],
    maximum_accepted_weak_sensitivity: f64,
) -> Result<DecisionObservabilityAssessment, FailureReason> {
    if decision_id.trim().is_empty() {
        return Err(FailureReason::InvalidDecisionIdentity);
    }
    if !decision_gradient.iter().all(|value| value.is_finite()) {
        return Err(FailureReason::InvalidDecisionGradient);
    }
    if !maximum_accepted_weak_sensitivity.is_finite()
        || !(0.0..=1.0).contains(&maximum_accepted_weak_sensitivity)
    {
        return Err(FailureReason::InvalidDecisionThreshold);
    }
    let dual_norm_squared =
        metric_dual_norm_squared(report.displacement_metric, decision_gradient)
            .ok_or(FailureReason::MetricNotPositiveDefinite)?;
    if !dual_norm_squared.is_finite() || dual_norm_squared <= 1.0e-20 {
        return Err(FailureReason::ZeroDecisionGradient);
    }

    let mut weak_component_squared = 0.0;
    for vector in &report.weak_subspace_basis {
        let component = dot6(decision_gradient, *vector);
        weak_component_squared += component * component;
    }
    let ratio_squared = weak_component_squared / dual_norm_squared;
    if !ratio_squared.is_finite() || ratio_squared < -1.0e-10 || ratio_squared > 1.0 + 1.0e-7 {
        return Err(FailureReason::InvalidDecisionGradient);
    }
    let normalized_weak_sensitivity = ratio_squared.clamp(0.0, 1.0).sqrt();
    let class = if normalized_weak_sensitivity <= maximum_accepted_weak_sensitivity {
        DecisionObservabilityClass::WithinWeakSensitivityLimit
    } else {
        DecisionObservabilityClass::ExceedsWeakSensitivityLimit
    };
    Ok(DecisionObservabilityAssessment {
        decision_id: decision_id.to_owned(),
        normalized_weak_sensitivity,
        maximum_accepted_weak_sensitivity,
        class,
        claim_ceiling: "SYNTHETIC_TASK_SPECIFIC_OBSERVABILITY_ONLY",
    })
}

fn metric_dual_norm_squared(metric: Matrix6, gradient: [f64; DOF]) -> Option<f64> {
    let mut diagonal_scale = [0.0; DOF];
    for i in 0..DOF {
        if !metric[i][i].is_finite() || metric[i][i] <= 0.0 {
            return None;
        }
        diagonal_scale[i] = 1.0 / metric[i][i].sqrt();
    }
    let scaled_metric = diagonal_congruence(&metric, diagonal_scale);
    let lower = cholesky(scaled_metric)?;
    let scaled_gradient: [f64; DOF] =
        std::array::from_fn(|i| diagonal_scale[i] * gradient[i]);
    let mut forward = [0.0; DOF];
    for row in 0..DOF {
        let mut value = scaled_gradient[row];
        for column in 0..row {
            value -= lower[row][column] * forward[column];
        }
        forward[row] = value / lower[row][row];
    }
    let mut solution = [0.0; DOF];
    for row in (0..DOF).rev() {
        let mut value = forward[row];
        for column in row + 1..DOF {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
    }
    let energy = dot6(scaled_gradient, solution);
    if energy.is_finite() && energy > 0.0 {
        Some(energy)
    } else {
        None
    }
}

/// For twist ordering [omega_x, omega_y, omega_z, v_x, v_y, v_z],
/// delta_p = omega x p + v = -[p]_x omega + v.
pub fn point_displacement_jacobian(point: [f64; 3]) -> [[f64; DOF]; 3] {
    [
        [0.0, point[2], -point[1], 1.0, 0.0, 0.0],
        [-point[2], 0.0, point[0], 0.0, 1.0, 0.0],
        [point[1], -point[0], 0.0, 0.0, 0.0, 1.0],
    ]
}

pub fn zero_matrix() -> Matrix6 {
    [[0.0; DOF]; DOF]
}

pub fn identity_matrix() -> Matrix6 {
    let mut result = zero_matrix();
    for i in 0..DOF {
        result[i][i] = 1.0;
    }
    result
}

pub fn transpose(matrix: Matrix6) -> Matrix6 {
    let mut result = zero_matrix();
    for row in 0..DOF {
        for column in 0..DOF {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

pub fn multiply(left: Matrix6, right: Matrix6) -> Matrix6 {
    let mut result = zero_matrix();
    for row in 0..DOF {
        for column in 0..DOF {
            for k in 0..DOF {
                result[row][column] += left[row][k] * right[k][column];
            }
        }
    }
    result
}

pub fn congruence(matrix: Matrix6, transform: Matrix6) -> Matrix6 {
    multiply(multiply(transpose(transform), matrix), transform)
}

/// Adjoint for a rigid frame transform under [rotation; translation] twist
/// ordering: [[R, 0], [[d]_x R, R]]. The caller defines whether this maps
/// new-frame twist coordinates to old-frame coordinates or vice versa.
pub fn se3_adjoint(rotation: [[f64; 3]; 3], translation: [f64; 3]) -> Matrix6 {
    let mut result = zero_matrix();
    let skew = [
        [0.0, -translation[2], translation[1]],
        [translation[2], 0.0, -translation[0]],
        [-translation[1], translation[0], 0.0],
    ];
    let mut skew_r = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for k in 0..3 {
                skew_r[row][column] += skew[row][k] * rotation[k][column];
            }
        }
    }
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = rotation[row][column];
            result[row + 3][column] = skew_r[row][column];
            result[row + 3][column + 3] = rotation[row][column];
        }
    }
    result
}

pub fn inverse_matrix(matrix: Matrix6) -> Option<Matrix6> {
    let mut a = matrix;
    let mut inverse = identity_matrix();
    for pivot in 0..DOF {
        let mut best = pivot;
        for row in pivot + 1..DOF {
            if a[row][pivot].abs() > a[best][pivot].abs() {
                best = row;
            }
        }
        if a[best][pivot].abs() < 1.0e-14 {
            return None;
        }
        if best != pivot {
            a.swap(best, pivot);
            inverse.swap(best, pivot);
        }
        let scale = a[pivot][pivot];
        for column in 0..DOF {
            a[pivot][column] /= scale;
            inverse[pivot][column] /= scale;
        }
        for row in 0..DOF {
            if row == pivot {
                continue;
            }
            let factor = a[row][pivot];
            for column in 0..DOF {
                a[row][column] -= factor * a[pivot][column];
                inverse[row][column] -= factor * inverse[pivot][column];
            }
        }
    }
    Some(inverse)
}

pub fn transformed_basis(
    basis: &[[f64; DOF]],
    coordinate_map: Matrix6,
) -> Option<Vec<[f64; DOF]>> {
    let inverse = inverse_matrix(coordinate_map)?;
    Some(
        basis
            .iter()
            .map(|vector| {
                let mut transformed = [0.0; DOF];
                for row in 0..DOF {
                    for column in 0..DOF {
                        transformed[row] += inverse[row][column] * vector[column];
                    }
                }
                transformed
            })
            .collect(),
    )
}

pub fn subspace_projector(basis: &[[f64; DOF]]) -> Matrix6 {
    let mut orthonormal: Vec<[f64; DOF]> = Vec::new();
    for candidate in basis {
        let mut vector = *candidate;
        for existing in &orthonormal {
            let projection = dot6(vector, *existing);
            for i in 0..DOF {
                vector[i] -= projection * existing[i];
            }
        }
        let norm = dot6(vector, vector).sqrt();
        if norm > 1.0e-10 {
            for item in &mut vector {
                *item /= norm;
            }
            orthonormal.push(vector);
        }
    }
    let mut projector = zero_matrix();
    for vector in orthonormal {
        for row in 0..DOF {
            for column in 0..DOF {
                projector[row][column] += vector[row] * vector[column];
            }
        }
    }
    projector
}

pub fn max_abs_difference(left: Matrix6, right: Matrix6) -> f64 {
    let mut maximum: f64 = 0.0;
    for row in 0..DOF {
        for column in 0..DOF {
            maximum = maximum.max((left[row][column] - right[row][column]).abs());
        }
    }
    maximum
}

fn diagonal_congruence(matrix: &Matrix6, scale: [f64; DOF]) -> Matrix6 {
    let mut result = zero_matrix();
    for row in 0..DOF {
        for column in 0..DOF {
            result[row][column] = scale[row] * matrix[row][column] * scale[column];
        }
    }
    result
}

fn cholesky(matrix: Matrix6) -> Option<Matrix6> {
    let mut lower = zero_matrix();
    for row in 0..DOF {
        for column in 0..=row {
            let mut sum = matrix[row][column];
            for k in 0..column {
                sum -= lower[row][k] * lower[column][k];
            }
            if row == column {
                if !sum.is_finite() || sum <= METRIC_PIVOT_TOLERANCE {
                    return None;
                }
                lower[row][column] = sum.sqrt();
            } else {
                lower[row][column] = sum / lower[column][column];
            }
        }
    }
    Some(lower)
}

fn inverse_lower_triangular(lower: Matrix6) -> Option<Matrix6> {
    let mut inverse = zero_matrix();
    for column in 0..DOF {
        for row in 0..DOF {
            let mut rhs = if row == column { 1.0 } else { 0.0 };
            for k in 0..row {
                rhs -= lower[row][k] * inverse[k][column];
            }
            if lower[row][row].abs() < 1.0e-15 {
                return None;
            }
            inverse[row][column] = rhs / lower[row][row];
        }
    }
    Some(inverse)
}

fn jacobi_eigen_symmetric(mut matrix: Matrix6) -> Option<([f64; DOF], Matrix6)> {
    let mut vectors = identity_matrix();
    let mut converged = false;
    for _ in 0..2_000 {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut largest: f64 = 0.0;
        let mut diagonal_scale: f64 = 1.0;
        for row in 0..DOF {
            diagonal_scale = diagonal_scale.max(matrix[row][row].abs());
            for column in row + 1..DOF {
                let value = matrix[row][column].abs();
                if value > largest {
                    largest = value;
                    p = row;
                    q = column;
                }
            }
        }
        if largest <= EIGEN_TOLERANCE * diagonal_scale {
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
        for k in 0..DOF {
            if k == p || k == q {
                continue;
            }
            let mkp = matrix[k][p];
            let mkq = matrix[k][q];
            matrix[k][p] = cosine * mkp - sine * mkq;
            matrix[p][k] = matrix[k][p];
            matrix[k][q] = sine * mkp + cosine * mkq;
            matrix[q][k] = matrix[k][q];
        }
        for row in 0..DOF {
            let vkp = vectors[row][p];
            let vkq = vectors[row][q];
            vectors[row][p] = cosine * vkp - sine * vkq;
            vectors[row][q] = sine * vkp + cosine * vkq;
        }
    }
    if !converged {
        return None;
    }
    let mut eigenvalues = [0.0; DOF];
    for i in 0..DOF {
        eigenvalues[i] = matrix[i][i];
    }
    Some((eigenvalues, vectors))
}

fn matrix_finite(matrix: &Matrix6) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

fn matrix_symmetric(matrix: &Matrix6, tolerance: f64) -> bool {
    for row in 0..DOF {
        for column in row + 1..DOF {
            if (matrix[row][column] - matrix[column][row]).abs() > tolerance {
                return false;
            }
        }
    }
    true
}

fn dot3(left: [f64; 3], right: [f64; 3]) -> f64 {
    left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
}

fn dot6(left: [f64; DOF], right: [f64; DOF]) -> f64 {
    let mut result = 0.0;
    for i in 0..DOF {
        result += left[i] * right[i];
    }
    result
}

fn all_finite(values: &[f64; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

pub mod fixtures {
    use super::{Correspondence, DOF, Matrix6, RegistrationInput};

    pub fn plane() -> RegistrationInput {
        let mut correspondences = Vec::new();
        for x in [-1.0, 0.0, 1.0] {
            for y in [-1.0, 0.0, 1.0] {
                correspondences.push(Correspondence {
                    point: [x, y, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    weight: 1.0,
                });
            }
        }
        input("plane", correspondences, 0.0)
    }

    pub fn cylinder() -> RegistrationInput {
        let mut correspondences = Vec::new();
        for z in [-1.0, 0.0, 1.0] {
            for angle in 0..24 {
                let theta = std::f64::consts::TAU * angle as f64 / 24.0;
                let normal = [theta.cos(), theta.sin(), 0.0];
                correspondences.push(Correspondence {
                    point: [normal[0], normal[1], z],
                    normal,
                    weight: 1.0,
                });
            }
        }
        input("cylinder", correspondences, 0.0)
    }

    pub fn three_face_corner() -> RegistrationInput {
        let mut correspondences = Vec::new();
        let levels = [-1.0, 0.0, 1.0];
        for a in levels {
            for b in levels {
                correspondences.push(Correspondence {
                    point: [0.0, a, b],
                    normal: [1.0, 0.0, 0.0],
                    weight: 1.0,
                });
                correspondences.push(Correspondence {
                    point: [a, 0.0, b],
                    normal: [0.0, 1.0, 0.0],
                    weight: 1.0,
                });
                correspondences.push(Correspondence {
                    point: [a, b, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    weight: 1.0,
                });
            }
        }
        input("three-face-corner", correspondences, 0.0)
    }

    pub fn two_intersecting_planes() -> RegistrationInput {
        let mut correspondences = Vec::new();
        for a in [-1.0, 0.0, 1.0] {
            for b in [-1.0, 0.0, 1.0] {
                correspondences.push(Correspondence {
                    point: [0.0, a, b],
                    normal: [1.0, 0.0, 0.0],
                    weight: 1.0,
                });
                correspondences.push(Correspondence {
                    point: [a, b, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    weight: 1.0,
                });
            }
        }
        input("two-intersecting-planes", correspondences, 0.0)
    }

    pub fn collinear() -> RegistrationInput {
        let mut correspondences = Vec::new();
        for x in [-2.0, -1.0, 0.0, 1.0, 2.0] {
            correspondences.push(Correspondence {
                point: [x, 0.0, 0.0],
                normal: [0.0, 1.0, 0.0],
                weight: 1.0,
            });
        }
        input("collinear", correspondences, 0.0)
    }

    pub fn input(case_id: &str, correspondences: Vec<Correspondence>, residual: f64) -> RegistrationInput {
        RegistrationInput {
            case_id: case_id.to_owned(),
            frame_id: "synthetic-frame".to_owned(),
            correspondences,
            reported_residual_rmse: residual,
            weak_threshold: 1.0e-8,
        }
    }

    pub fn diagonal_transform(values: [f64; DOF]) -> Matrix6 {
        let mut matrix = [[0.0; DOF]; DOF];
        for i in 0..DOF {
            matrix[i][i] = values[i];
        }
        matrix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_frame_equivariance(
        input: RegistrationInput,
        rotation: [[f64; 3]; 3],
        translation: [f64; 3],
    ) {
        let original = analyze_registration(&input).expect("base observability");
        let transform = se3_adjoint(rotation, translation);
        let changed = analyze_matrices(
            "frame-metamorphic",
            "transformed-frame",
            congruence(original.information_matrix, transform),
            congruence(original.displacement_metric, transform),
            input.reported_residual_rmse,
            input.weak_threshold,
        )
        .expect("transformed observability");
        assert_spectrum_close(original.eigenvalues, changed.eigenvalues, 1.0e-8);
        assert_eq!(original.weak_direction_count, changed.weak_direction_count);
        let expected = transformed_basis(&original.weak_subspace_basis, transform)
            .expect("invertible rigid adjoint");
        assert!(
            max_abs_difference(
                subspace_projector(&expected),
                subspace_projector(&changed.weak_subspace_basis)
            ) < 1.0e-7
        );
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} ≈ {right} within {tolerance}"
        );
    }

    fn assert_spectrum_close(left: [f64; DOF], right: [f64; DOF], tolerance: f64) {
        for i in 0..DOF {
            close(left[i], right[i], tolerance);
        }
    }

    #[test]
    fn three_nonparallel_faces_observe_all_six_twist_directions() {
        let report = analyze_registration(&fixtures::three_face_corner())
            .expect("three-face corner should have a positive-definite metric");
        assert_eq!(report.class, ObservabilityClass::FullyObservable);
        assert_eq!(report.weak_direction_count, 0);
        assert!(report.eigenvalues.iter().all(|value| *value > 1.0e-8));
    }

    #[test]
    fn single_plane_has_three_weak_directions() {
        let report =
            analyze_registration(&fixtures::plane()).expect("planar metric should be SPD");
        assert_eq!(report.class, ObservabilityClass::PartiallyObservable);
        assert_eq!(report.weak_direction_count, 3);
        assert!(report.eigenvalues[..3].iter().all(|value| *value < 1.0e-8));
    }

    #[test]
    fn cylindrical_geometry_has_two_axial_roll_weak_directions() {
        let report =
            analyze_registration(&fixtures::cylinder()).expect("cylinder metric should be SPD");
        assert_eq!(report.weak_direction_count, 2);
        assert!(report.eigenvalues[..2].iter().all(|value| *value < 1.0e-8));
    }

    #[test]
    fn two_intersecting_planes_leave_one_weak_direction() {
        let report = analyze_registration(&fixtures::two_intersecting_planes())
            .expect("two-plane metric should be SPD");
        assert_eq!(report.weak_direction_count, 1);
        assert!(report.eigenvalues[0] < 1.0e-8);
    }

    #[test]
    fn transforming_actual_points_and_normals_rebuilds_equivariant_matrices() {
        let original_input = fixtures::plane();
        let original = analyze_registration(&original_input).expect("source-frame plane");
        let angle = 0.42_f64;
        let rotation = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let translation = [1.2, -0.7, 0.5];
        let adjoint = se3_adjoint(rotation, translation);
        let old_twist_from_new = inverse_matrix(adjoint).expect("invertible adjoint");
        let mut transformed_input = original_input.clone();
        transformed_input.frame_id = "physically-transformed-frame".to_owned();
        for correspondence in &mut transformed_input.correspondences {
            let point = correspondence.point;
            let normal = correspondence.normal;
            for row in 0..3 {
                correspondence.point[row] = translation[row];
                correspondence.normal[row] = 0.0;
                for column in 0..3 {
                    correspondence.point[row] += rotation[row][column] * point[column];
                    correspondence.normal[row] += rotation[row][column] * normal[column];
                }
            }
        }
        let transformed = analyze_registration(&transformed_input)
            .expect("transformed point/normal correspondences");
        let expected_h = congruence(original.information_matrix, old_twist_from_new);
        let expected_m = congruence(original.displacement_metric, old_twist_from_new);
        assert!(max_abs_difference(expected_h, transformed.information_matrix) < 1.0e-8);
        assert!(max_abs_difference(expected_m, transformed.displacement_metric) < 1.0e-8);
        assert_spectrum_close(original.eigenvalues, transformed.eigenvalues, 1.0e-8);
        let expected_basis = transformed_basis(
            &original.weak_subspace_basis,
            old_twist_from_new,
        )
        .expect("new-frame twist basis");
        assert!(
            max_abs_difference(
                subspace_projector(&expected_basis),
                subspace_projector(&transformed.weak_subspace_basis)
            ) < 1.0e-7
        );
    }

    #[test]
    fn pure_origin_translation_preserves_spectrum_and_weak_subspace() {
        assert_frame_equivariance(
            fixtures::plane(),
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            [2.3, -1.1, 0.7],
        );
    }

    #[test]
    fn pure_body_rotation_preserves_spectrum_and_weak_subspace() {
        let angle = -0.83_f64;
        let rotation = [
            [1.0, 0.0, 0.0],
            [0.0, angle.cos(), -angle.sin()],
            [0.0, angle.sin(), angle.cos()],
        ];
        assert_frame_equivariance(fixtures::plane(), rotation, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn collinear_correspondences_cannot_define_a_full_six_dof_metric() {
        assert_eq!(
            analyze_registration(&fixtures::collinear()).unwrap_err(),
            FailureReason::MetricNotPositiveDefinite
        );
    }

    #[test]
    fn high_residual_well_conditioned_geometry_is_not_degeneracy() {
        let mut input = fixtures::three_face_corner();
        input.reported_residual_rmse = 12.0;
        let report = analyze_registration(&input).expect("finite high residual metadata");
        assert_eq!(report.class, ObservabilityClass::FullyObservable);
        close(report.reported_residual_rmse, 12.0, 0.0);
    }

    #[test]
    fn low_residual_degenerate_geometry_still_abstains_from_full_observability() {
        let input = fixtures::plane();
        let report = analyze_registration(&input).expect("planar correspondences valid");
        assert_eq!(report.reported_residual_rmse, 0.0);
        assert_eq!(report.class, ObservabilityClass::PartiallyObservable);
        assert_eq!(report.weak_direction_count, 3);
    }

    #[test]
    fn frame_congruence_preserves_generalized_spectrum_and_weak_subspace() {
        let original = analyze_registration(&fixtures::plane()).expect("plane report");
        let angle = 0.71_f64;
        let rotation = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let transform = se3_adjoint(rotation, [1.7, -0.4, 0.9]);
        let changed_h = congruence(original.information_matrix, transform);
        let changed_m = congruence(original.displacement_metric, transform);
        let changed = analyze_matrices(
            "plane-frame-changed",
            "shifted-body-frame",
            changed_h,
            changed_m,
            0.0,
            1.0e-8,
        )
        .expect("congruence-transformed system");
        assert_spectrum_close(original.eigenvalues, changed.eigenvalues, 1.0e-8);
        assert_eq!(original.weak_direction_count, changed.weak_direction_count);

        let expected_basis = transformed_basis(&original.weak_subspace_basis, transform)
            .expect("invertible SE(3) adjoint");
        let expected_projector = subspace_projector(&expected_basis);
        let observed_projector = subspace_projector(&changed.weak_subspace_basis);
        assert!(max_abs_difference(expected_projector, observed_projector) < 1.0e-7);
    }

    #[test]
    fn consistent_translation_unit_change_preserves_spectrum() {
        let original = analyze_registration(&fixtures::three_face_corner()).expect("corner");
        let unit_map = fixtures::diagonal_transform([1.0, 1.0, 1.0, 1000.0, 1000.0, 1000.0]);
        let changed = analyze_matrices(
            "unit-changed",
            "millimetre-twist-coordinates",
            congruence(original.information_matrix, unit_map),
            congruence(original.displacement_metric, unit_map),
            0.0,
            1.0e-8,
        )
        .expect("unit-rescaled congruence");
        assert_spectrum_close(original.eigenvalues, changed.eigenvalues, 1.0e-8);
    }

    #[test]
    fn uniform_scene_scale_congruence_preserves_spectrum() {
        let original = analyze_registration(&fixtures::cylinder()).expect("cylinder");
        let scale_map = fixtures::diagonal_transform([1000.0, 1000.0, 1000.0, 1.0, 1.0, 1.0]);
        let changed = analyze_matrices(
            "scaled-scene",
            "scaled-scene-frame",
            congruence(original.information_matrix, scale_map),
            congruence(original.displacement_metric, scale_map),
            0.0,
            1.0e-8,
        )
        .expect("scene-scale congruence");
        assert_spectrum_close(original.eigenvalues, changed.eigenvalues, 1.0e-8);
    }

    #[test]
    fn multiplying_all_correspondence_weights_does_not_change_spectrum() {
        let mut input = fixtures::three_face_corner();
        let baseline = analyze_registration(&input).expect("baseline");
        for correspondence in &mut input.correspondences {
            correspondence.weight *= 71.0;
        }
        let changed = analyze_registration(&input).expect("uniformly reweighted");
        assert_spectrum_close(baseline.eigenvalues, changed.eigenvalues, 1.0e-8);
    }

    #[test]
    fn task_gradient_in_a_weak_plane_direction_is_blocked() {
        let report = analyze_registration(&fixtures::plane()).expect("plane report");
        let assessment = assess_decision_observability(
            &report,
            "in-plane-bead-offset",
            [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            0.05,
        )
        .expect("valid task gradient");
        assert_eq!(
            assessment.class,
            DecisionObservabilityClass::ExceedsWeakSensitivityLimit
        );
        close(assessment.normalized_weak_sensitivity, 1.0, 1.0e-8);
    }

    #[test]
    fn task_gradient_in_a_plane_normal_direction_can_be_observable() {
        let report = analyze_registration(&fixtures::plane()).expect("plane report");
        let assessment = assess_decision_observability(
            &report,
            "normal-surface-offset",
            [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            0.05,
        )
        .expect("valid task gradient");
        assert_eq!(
            assessment.class,
            DecisionObservabilityClass::WithinWeakSensitivityLimit
        );
        close(assessment.normalized_weak_sensitivity, 0.0, 1.0e-8);
    }

    #[test]
    fn task_observability_score_is_invariant_under_frame_change() {
        let input = fixtures::plane();
        let report = analyze_registration(&input).expect("source plane");
        let gradient = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let before = assess_decision_observability(
            &report,
            "in-plane-offset",
            gradient,
            0.05,
        )
        .expect("source task assessment");

        let angle = 0.61_f64;
        let rotation = [
            [angle.cos(), -angle.sin(), 0.0],
            [angle.sin(), angle.cos(), 0.0],
            [0.0, 0.0, 1.0],
        ];
        let adjoint = se3_adjoint(rotation, [0.8, -0.3, 1.1]);
        let old_twist_from_new = inverse_matrix(adjoint).expect("adjoint inverse");
        let changed = analyze_matrices(
            "plane-transformed",
            "other-frame",
            congruence(report.information_matrix, old_twist_from_new),
            congruence(report.displacement_metric, old_twist_from_new),
            report.reported_residual_rmse,
            report.weak_threshold,
        )
        .expect("transformed plane");
        let gradient_new = std::array::from_fn(|row| {
            (0..DOF)
                .map(|column| old_twist_from_new[column][row] * gradient[column])
                .sum()
        });
        let after = assess_decision_observability(
            &changed,
            "in-plane-offset",
            gradient_new,
            0.05,
        )
        .expect("transformed task assessment");
        close(
            before.normalized_weak_sensitivity,
            after.normalized_weak_sensitivity,
            1.0e-8,
        );
        assert_eq!(before.class, after.class);
    }

    #[test]
    fn zero_decision_gradient_is_rejected() {
        let report = analyze_registration(&fixtures::plane()).expect("plane report");
        assert_eq!(
            assess_decision_observability(&report, "constant", [0.0; DOF], 0.1)
                .unwrap_err(),
            FailureReason::ZeroDecisionGradient
        );
    }

    #[test]
    fn invalid_task_observability_threshold_is_rejected() {
        let report = analyze_registration(&fixtures::plane()).expect("plane report");
        assert_eq!(
            assess_decision_observability(
                &report,
                "offset",
                [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
                1.1
            )
            .unwrap_err(),
            FailureReason::InvalidDecisionThreshold
        );
    }

    #[test]
    fn invalid_normal_is_rejected() {
        let mut input = fixtures::plane();
        input.correspondences[0].normal = [0.0, 0.0, 2.0];
        assert_eq!(
            analyze_registration(&input).unwrap_err(),
            FailureReason::NonUnitNormal
        );
    }

    #[test]
    fn zero_weight_is_rejected() {
        let mut input = fixtures::plane();
        input.correspondences[0].weight = 0.0;
        assert_eq!(
            analyze_registration(&input).unwrap_err(),
            FailureReason::NonPositiveOrNonFiniteWeight
        );
    }

    #[test]
    fn nonfinite_point_is_rejected() {
        let mut input = fixtures::plane();
        input.correspondences[0].point[1] = f64::NAN;
        assert_eq!(
            analyze_registration(&input).unwrap_err(),
            FailureReason::NonFinitePoint
        );
    }

    #[test]
    fn asymmetric_information_matrix_is_rejected() {
        let base = analyze_registration(&fixtures::plane()).expect("plane");
        let mut information = base.information_matrix;
        information[0][1] += 0.25;
        assert_eq!(
            analyze_matrices(
                "asymmetric",
                "test-frame",
                information,
                base.displacement_metric,
                0.0,
                1.0e-8,
            )
            .unwrap_err(),
            FailureReason::MatrixNotSymmetric
        );
    }

    #[test]
    fn empty_correspondence_set_is_rejected() {
        let mut input = fixtures::plane();
        input.correspondences.clear();
        assert_eq!(
            analyze_registration(&input).unwrap_err(),
            FailureReason::EmptyCorrespondenceSet
        );
    }

    #[test]
    fn invalid_weak_threshold_is_rejected() {
        let mut input = fixtures::plane();
        input.weak_threshold = 0.0;
        assert_eq!(
            analyze_registration(&input).unwrap_err(),
            FailureReason::InvalidWeakThreshold
        );
    }

    #[test]
    fn metric_report_is_never_labelled_covariance() {
        let report = analyze_registration(&fixtures::plane()).expect("plane");
        assert_eq!(
            report.metric_semantics,
            "POINT_DISPLACEMENT_METRIC_NOT_COVARIANCE"
        );
        assert_eq!(
            report.claim_ceiling,
            "SYNTHETIC_REGISTRATION_OBSERVABILITY_ONLY"
        );
    }
}
