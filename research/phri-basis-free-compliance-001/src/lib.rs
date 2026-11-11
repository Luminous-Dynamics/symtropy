#![forbid(unsafe_code)]

//! Basis-free 3x3 symmetric-matrix passivity constraint reference.
//!
//! V0 evaluates a candidate continuous-time task-space compliance parameter
//! rate against a matrix-coordinate passivity inequality and an Loewner
//! stiffness-rate barrier. It deliberately does not compute eigenvectors,
//! eigenbasis derivatives, an optimizer solution, a control law, or actuator
//! commands.

use core::fmt;

/// A real symmetric 3x3 matrix stored in six independent coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Symmetric3 {
    pub xx: f64,
    pub yy: f64,
    pub zz: f64,
    pub xy: f64,
    pub xz: f64,
    pub yz: f64,
}

impl Symmetric3 {
    pub const fn diagonal(xx: f64, yy: f64, zz: f64) -> Self {
        Self {
            xx,
            yy,
            zz,
            xy: 0.0,
            xz: 0.0,
            yz: 0.0,
        }
    }

    /// Evaluate xᵀ A x without expanding A into a dense matrix.
    pub fn quadratic_form(self, x: [f64; 3]) -> Result<f64, MatrixError> {
        if !x.iter().all(|value| value.is_finite()) {
            return Err(MatrixError::NonFiniteVector);
        }
        self.validate_finite()?;
        let result = self.xx * x[0] * x[0]
            + self.yy * x[1] * x[1]
            + self.zz * x[2] * x[2]
            + 2.0 * self.xy * x[0] * x[1]
            + 2.0 * self.xz * x[0] * x[2]
            + 2.0 * self.yz * x[1] * x[2];
        if !result.is_finite() {
            return Err(MatrixError::ArithmeticOverflow);
        }
        Ok(result)
    }

    /// Return self + scale * other, preserving exact symmetry by construction.
    pub fn add_scaled(self, other: Self, scale: f64) -> Result<Self, MatrixError> {
        self.validate_finite()?;
        other.validate_finite()?;
        if !scale.is_finite() {
            return Err(MatrixError::NonFiniteScale);
        }
        let result = Self {
            xx: self.xx + scale * other.xx,
            yy: self.yy + scale * other.yy,
            zz: self.zz + scale * other.zz,
            xy: self.xy + scale * other.xy,
            xz: self.xz + scale * other.xz,
            yz: self.yz + scale * other.yz,
        };
        result.validate_finite()?;
        Ok(result)
    }

    /// Check positive semidefiniteness using all principal minors of a
    /// symmetric 3x3 matrix. The matrix is scaled by its largest absolute
    /// entry before evaluating the minors to reduce overflow/underflow.
    ///
    /// tolerance is dimensionless and applies to the normalized principal
    /// minors. This is a numerical certificate, not a symbolic proof.
    pub fn is_positive_semidefinite(self, tolerance: f64) -> Result<bool, MatrixError> {
        self.validate_tolerance(tolerance)?;
        let Some(a) = self.normalized_entries()? else {
            return Ok(true);
        };
        let [xx, yy, zz, xy, xz, yz] = a;

        let one_by_one = [xx, yy, zz];
        let two_by_two = [
            xx * yy - xy * xy,
            xx * zz - xz * xz,
            yy * zz - yz * yz,
        ];
        let determinant = xx * yy * zz
            + 2.0 * xy * xz * yz
            - xx * yz * yz
            - yy * xz * xz
            - zz * xy * xy;

        if !one_by_one.iter().all(|value| value.is_finite())
            || !two_by_two.iter().all(|value| value.is_finite())
            || !determinant.is_finite()
        {
            return Err(MatrixError::ArithmeticOverflow);
        }

        Ok(one_by_one.iter().all(|value| *value >= -tolerance)
            && two_by_two.iter().all(|value| *value >= -tolerance)
            && determinant >= -tolerance)
    }

    /// Check positive definiteness using Sylvester's criterion on the leading
    /// principal minors. A tolerance-sized normalized margin is rejected.
    pub fn is_positive_definite(self, tolerance: f64) -> Result<bool, MatrixError> {
        self.validate_tolerance(tolerance)?;
        let Some(a) = self.normalized_entries()? else {
            return Ok(false);
        };
        let [xx, yy, zz, xy, xz, yz] = a;
        let leading_two = xx * yy - xy * xy;
        let determinant = xx * yy * zz
            + 2.0 * xy * xz * yz
            - xx * yz * yz
            - yy * xz * xz
            - zz * xy * xy;
        if !leading_two.is_finite() || !determinant.is_finite() {
            return Err(MatrixError::ArithmeticOverflow);
        }
        Ok(xx > tolerance && leading_two > tolerance && determinant > tolerance)
    }

    fn validate_tolerance(self, tolerance: f64) -> Result<(), MatrixError> {
        self.validate_finite()?;
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(MatrixError::InvalidTolerance);
        }
        Ok(())
    }

    fn validate_finite(self) -> Result<(), MatrixError> {
        if [
            self.xx, self.yy, self.zz, self.xy, self.xz, self.yz,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            Ok(())
        } else {
            Err(MatrixError::NonFiniteMatrix)
        }
    }

    fn normalized_entries(self) -> Result<Option<[f64; 6]>, MatrixError> {
        self.validate_finite()?;
        let scale = [
            self.xx.abs(),
            self.yy.abs(),
            self.zz.abs(),
            self.xy.abs(),
            self.xz.abs(),
            self.yz.abs(),
        ]
        .into_iter()
        .fold(0.0_f64, f64::max);
        if scale == 0.0 {
            return Ok(None);
        }
        let normalized = [
            self.xx / scale,
            self.yy / scale,
            self.zz / scale,
            self.xy / scale,
            self.xz / scale,
            self.yz / scale,
        ];
        if !normalized.iter().all(|value| value.is_finite()) {
            return Err(MatrixError::ArithmeticOverflow);
        }
        Ok(Some(normalized))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixError {
    NonFiniteMatrix,
    NonFiniteVector,
    NonFiniteScale,
    InvalidTolerance,
    ArithmeticOverflow,
}

impl fmt::Display for MatrixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for MatrixError {}

/// Input to the basis-free continuous-time compliance passivity evaluator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasisFreeComplianceRequest {
    /// Current positive scalar inertia scaling.
    pub epsilon: f64,
    pub epsilon_min: f64,
    pub epsilon_decay_rate: f64,
    /// Candidate inertia-scaling rate.
    pub epsilon_rate: f64,
    /// Configuration-dependent task-space inertia Λ(q).
    pub task_inertia: Symmetric3,
    /// Current symmetric positive-definite task-space stiffness K.
    pub stiffness: Symmetric3,
    /// Candidate symmetric stiffness-rate matrix K_dot.
    pub stiffness_rate: Symmetric3,
    /// Applied symmetric positive-semidefinite damping matrix D.
    pub damping: Symmetric3,
    pub stiffness_decay_rate: f64,
    pub task_error: [f64; 3],
    pub task_error_velocity: [f64; 3],
    /// Dimensionless tolerance for PSD/SPD matrix checks.
    pub matrix_tolerance: f64,
    /// Absolute passivity-inequality tolerance in watts.
    pub passivity_tolerance_w: f64,
}

/// Constraint evaluation result for one proposed rate pair.
///
/// This is a diagnostic receipt only. A false flag means the candidate must
/// not be used by a controller. A true flag is not an actuation authorization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasisFreeComplianceReceipt {
    pub epsilon_barrier_lhs: f64,
    pub epsilon_barrier_rhs: f64,
    pub epsilon_barrier_satisfied: bool,
    /// The Loewner barrier matrix K_dot + kappa_K * K.
    pub stiffness_barrier_matrix: Symmetric3,
    pub stiffness_barrier_positive_semidefinite: bool,
    /// 0.5*epsilon_dot*vᵀΛv + 0.5*eᵀK_dot e - vᵀDv.
    pub passivity_lhs_w: f64,
    pub damping_dissipation_w: f64,
    pub passivity_slack_w: f64,
    pub passivity_satisfied: bool,
    pub candidate_admissible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComplianceEvaluationError {
    InvalidInertiaScale,
    InvalidStiffnessDecayRate,
    InvalidPassivityTolerance,
    TaskInertiaNotPositiveDefinite,
    StiffnessNotPositiveDefinite,
    DampingNotPositiveSemidefinite,
    Matrix(MatrixError),
}

impl From<MatrixError> for ComplianceEvaluationError {
    fn from(value: MatrixError) -> Self {
        Self::Matrix(value)
    }
}

impl fmt::Display for ComplianceEvaluationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ComplianceEvaluationError {}

/// Evaluate the matrix-coordinate task-space passivity and parameter barriers.
///
/// The candidate inequality is:
///
/// 0.5 * epsilon_dot * vᵀ Lambda(q) v
///   + 0.5 * eᵀ K_dot e
///   - vᵀ D v <= 0
///
/// The parameter barriers are:
///
/// epsilon_dot >= -kappa_epsilon * (epsilon - epsilon_min)
/// K_dot + kappa_K * K >= 0 in the Loewner/PSD order.
///
/// The derivation assumes the same configuration-dependent task-space
/// manipulator dynamics and skew-symmetry compensation as the cited
/// passivity literature. It does not prove sampled-data passivity, null-space
/// passivity, force/pressure safety, or a complete robot-level guarantee.
pub fn evaluate_basis_free_compliance_rate(
    request: BasisFreeComplianceRequest,
) -> Result<BasisFreeComplianceReceipt, ComplianceEvaluationError> {
    if !request.epsilon.is_finite()
        || !request.epsilon_min.is_finite()
        || request.epsilon <= 0.0
        || request.epsilon_min <= 0.0
        || request.epsilon < request.epsilon_min
        || !request.epsilon_decay_rate.is_finite()
        || request.epsilon_decay_rate <= 0.0
        || !request.epsilon_rate.is_finite()
    {
        return Err(ComplianceEvaluationError::InvalidInertiaScale);
    }
    if !request.stiffness_decay_rate.is_finite()
        || request.stiffness_decay_rate <= 0.0
    {
        return Err(ComplianceEvaluationError::InvalidStiffnessDecayRate);
    }
    if !request.passivity_tolerance_w.is_finite()
        || request.passivity_tolerance_w < 0.0
    {
        return Err(ComplianceEvaluationError::InvalidPassivityTolerance);
    }

    if !request
        .task_inertia
        .is_positive_definite(request.matrix_tolerance)?
    {
        return Err(ComplianceEvaluationError::TaskInertiaNotPositiveDefinite);
    }
    if !request
        .stiffness
        .is_positive_definite(request.matrix_tolerance)?
    {
        return Err(ComplianceEvaluationError::StiffnessNotPositiveDefinite);
    }
    if !request
        .damping
        .is_positive_semidefinite(request.matrix_tolerance)?
    {
        return Err(ComplianceEvaluationError::DampingNotPositiveSemidefinite);
    }

    if !request.task_error.iter().all(|value| value.is_finite())
        || !request
            .task_error_velocity
            .iter()
            .all(|value| value.is_finite())
    {
        return Err(ComplianceEvaluationError::Matrix(MatrixError::NonFiniteVector));
    }

    let epsilon_barrier_lhs =
        request.epsilon_rate + request.epsilon_decay_rate * request.epsilon;
    let epsilon_barrier_rhs = request.epsilon_decay_rate * request.epsilon_min;
    let epsilon_barrier_satisfied = epsilon_barrier_lhs >= epsilon_barrier_rhs;

    let stiffness_barrier_matrix = request
        .stiffness_rate
        .add_scaled(request.stiffness, request.stiffness_decay_rate)?;
    let stiffness_barrier_positive_semidefinite =
        stiffness_barrier_matrix.is_positive_semidefinite(request.matrix_tolerance)?;

    let inertia_quadratic = request
        .task_inertia
        .quadratic_form(request.task_error_velocity)?;
    let stiffness_rate_quadratic = request
        .stiffness_rate
        .quadratic_form(request.task_error)?;
    let damping_dissipation_w = request
        .damping
        .quadratic_form(request.task_error_velocity)?;

    let passivity_lhs_w = 0.5 * request.epsilon_rate * inertia_quadratic
        + 0.5 * stiffness_rate_quadratic
        - damping_dissipation_w;
    if !epsilon_barrier_lhs.is_finite()
        || !epsilon_barrier_rhs.is_finite()
        || !damping_dissipation_w.is_finite()
        || !passivity_lhs_w.is_finite()
    {
        return Err(ComplianceEvaluationError::Matrix(MatrixError::ArithmeticOverflow));
    }

    let passivity_slack_w = -passivity_lhs_w;
    let passivity_satisfied = passivity_lhs_w <= request.passivity_tolerance_w;
    let candidate_admissible = epsilon_barrier_satisfied
        && stiffness_barrier_positive_semidefinite
        && passivity_satisfied;

    Ok(BasisFreeComplianceReceipt {
        epsilon_barrier_lhs,
        epsilon_barrier_rhs,
        epsilon_barrier_satisfied,
        stiffness_barrier_matrix,
        stiffness_barrier_positive_semidefinite,
        passivity_lhs_w,
        damping_dissipation_w,
        passivity_slack_w,
        passivity_satisfied,
        candidate_admissible,
    })
}


/// Request for the fixed-size basis-free compliance-rate optimizer.
///
/// The objective is measured in the Cholesky-whitened stiffness-rate metric:
/// ||L^-1 (K_dot - K_dot_ref) L^-T||_F^2, where K = L L^T.
/// This is the congruence-invariant metric tr(K^-1 DeltaK K^-1 DeltaK).
///
/// V0 has one affine passivity constraint and two lower-rate barriers. It
/// intentionally has no application-specific upper-rate boxes; those change
/// the feasibility problem and must be added as an explicitly separate V1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasisFreeRateOptimizationRequest {
    pub epsilon: f64,
    pub epsilon_min: f64,
    pub epsilon_decay_rate: f64,
    pub epsilon_rate_reference: f64,
    pub task_inertia: Symmetric3,
    pub stiffness: Symmetric3,
    pub stiffness_rate_reference: Symmetric3,
    pub damping: Symmetric3,
    pub stiffness_decay_rate: f64,
    pub task_error: [f64; 3],
    pub task_error_velocity: [f64; 3],
    pub epsilon_rate_weight: f64,
    pub stiffness_rate_weight: f64,
    pub matrix_tolerance: f64,
    pub passivity_tolerance_w: f64,
}

/// Result of the basis-free rate optimization.
///
/// This is a numerical optimization receipt only. It does not authorize
/// actuation or establish sampled-data, null-space, or full-robot passivity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasisFreeRateOptimizationReceipt {
    pub epsilon_rate: f64,
    pub stiffness_rate: Symmetric3,
    pub dual_multiplier: f64,
    pub passivity_residual_w: f64,
    pub passivity_slack_w: f64,
    pub passivity_complementarity_residual: f64,
    pub epsilon_rate_lower_bound: f64,
    pub stiffness_barrier_matrix: Symmetric3,
    pub minimum_stiffness_barrier_eigenvalue: f64,
    pub dual_iterations: u8,
    pub passivity_constraint_active: bool,
    pub candidate_admissible: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BasisFreeOptimizerError {
    InvalidObjectiveWeight,
    InvalidReferenceRate,
    NegativeDampingDissipation,
    CholeskyFailure,
    SpectralDecompositionFailure,
    DualBracketFailure,
    Infeasible,
    ArithmeticOverflow,
    IndependentPostCheckFailed,
}

impl fmt::Display for BasisFreeOptimizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for BasisFreeOptimizerError {}

type Dense3 = [[f64; 3]; 3];

fn dense_from_symmetric(matrix: Symmetric3) -> Dense3 {
    [
        [matrix.xx, matrix.xy, matrix.xz],
        [matrix.xy, matrix.yy, matrix.yz],
        [matrix.xz, matrix.yz, matrix.zz],
    ]
}

fn symmetric_from_dense(matrix: Dense3) -> Result<Symmetric3, BasisFreeOptimizerError> {
    let result = Symmetric3 {
        xx: matrix[0][0],
        yy: matrix[1][1],
        zz: matrix[2][2],
        xy: 0.5 * (matrix[0][1] + matrix[1][0]),
        xz: 0.5 * (matrix[0][2] + matrix[2][0]),
        yz: 0.5 * (matrix[1][2] + matrix[2][1]),
    };
    if [
        result.xx, result.yy, result.zz, result.xy, result.xz, result.yz,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        Ok(result)
    } else {
        Err(BasisFreeOptimizerError::ArithmeticOverflow)
    }
}

#[allow(clippy::needless_range_loop)]
fn transpose_dense3(matrix: Dense3) -> Dense3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row][column] = matrix[column][row];
        }
    }
    result
}

#[allow(clippy::needless_range_loop)]
fn multiply_dense3(left: Dense3, right: Dense3) -> Dense3 {
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for inner in 0..3 {
                result[row][column] += left[row][inner] * right[inner][column];
            }
        }
    }
    result
}

fn dense_max_abs(matrix: Dense3) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

#[allow(clippy::needless_range_loop)]
fn cholesky_lower_3(matrix: Dense3) -> Result<Dense3, BasisFreeOptimizerError> {
    let mut lower = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..=row {
            let mut sum = matrix[row][column];
            for inner in 0..column {
                sum -= lower[row][inner] * lower[column][inner];
            }
            if row == column {
                if !sum.is_finite() || sum <= 0.0 {
                    return Err(BasisFreeOptimizerError::CholeskyFailure);
                }
                lower[row][column] = sum.sqrt();
            } else {
                let pivot = lower[column][column];
                let value = sum / pivot;
                if !value.is_finite() {
                    return Err(BasisFreeOptimizerError::CholeskyFailure);
                }
                lower[row][column] = value;
            }
        }
    }
    Ok(lower)
}

#[allow(clippy::needless_range_loop)]
fn inverse_lower_3(lower: Dense3) -> Result<Dense3, BasisFreeOptimizerError> {
    let mut inverse = [[0.0; 3]; 3];
    for column in 0..3 {
        for row in 0..3 {
            if row < column {
                continue;
            }
            let mut rhs = if row == column { 1.0 } else { 0.0 };
            for inner in 0..row {
                rhs -= lower[row][inner] * inverse[inner][column];
            }
            let value = rhs / lower[row][row];
            if !value.is_finite() {
                return Err(BasisFreeOptimizerError::CholeskyFailure);
            }
            inverse[row][column] = value;
        }
    }
    Ok(inverse)
}

#[allow(clippy::needless_range_loop)]
fn dense_vector_product(matrix: Dense3, vector: [f64; 3]) -> [f64; 3] {
    let mut result = [0.0; 3];
    for row in 0..3 {
        for column in 0..3 {
            result[row] += matrix[row][column] * vector[column];
        }
    }
    result
}

fn quadratic_dense3(matrix: Dense3, vector: [f64; 3]) -> f64 {
    let product = dense_vector_product(matrix, vector);
    vector[0] * product[0] + vector[1] * product[1] + vector[2] * product[2]
}

/// Symmetric Jacobi eigensolver for fixed 3x3 matrices.
///
/// The input is scaled before rotations. Eigenvectors are used only locally
/// for a pointwise matrix function; no eigenvector identity is retained or
/// differentiated across samples. Repeated eigenspaces therefore reconstruct
/// the same matrix-valued spectral projection.
#[allow(clippy::needless_range_loop)]
fn symmetric_eigen_3(
    input: Dense3,
) -> Result<([f64; 3], Dense3), BasisFreeOptimizerError> {
    let scale = dense_max_abs(input);
    if !scale.is_finite() {
        return Err(BasisFreeOptimizerError::SpectralDecompositionFailure);
    }
    if scale == 0.0 {
        return Ok(([0.0; 3], [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]));
    }

    let mut matrix = input;
    for value in matrix.iter_mut().flatten() {
        *value /= scale;
    }
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut converged = false;

    for _ in 0..64 {
        let pairs = [(0_usize, 1_usize), (0, 2), (1, 2)];
        let mut pivot = (0_usize, 1_usize);
        let mut maximum = matrix[0][1].abs();
        for (row, column) in pairs.into_iter().skip(1) {
            let candidate = matrix[row][column].abs();
            if candidate > maximum {
                maximum = candidate;
                pivot = (row, column);
            }
        }
        if maximum <= 32.0 * f64::EPSILON {
            converged = true;
            break;
        }

        let (p, q) = pivot;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let angle = 0.5 * (2.0 * apq).atan2(aqq - app);
        let cosine = angle.cos();
        let sine = angle.sin();

        for index in 0..3 {
            if index == p || index == q {
                continue;
            }
            let aip = matrix[index][p];
            let aiq = matrix[index][q];
            let new_ip = cosine * aip - sine * aiq;
            let new_iq = sine * aip + cosine * aiq;
            matrix[index][p] = new_ip;
            matrix[p][index] = new_ip;
            matrix[index][q] = new_iq;
            matrix[q][index] = new_iq;
        }

        matrix[p][p] = cosine * cosine * app
            - 2.0 * cosine * sine * apq
            + sine * sine * aqq;
        matrix[q][q] = sine * sine * app
            + 2.0 * cosine * sine * apq
            + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;

        for row in 0..3 {
            let vip = vectors[row][p];
            let viq = vectors[row][q];
            vectors[row][p] = cosine * vip - sine * viq;
            vectors[row][q] = sine * vip + cosine * viq;
        }
    }

    if !converged
        && matrix[0][1].abs().max(matrix[0][2].abs()).max(matrix[1][2].abs())
            > 128.0 * f64::EPSILON
    {
        return Err(BasisFreeOptimizerError::SpectralDecompositionFailure);
    }

    let eigenvalues = [
        matrix[0][0] * scale,
        matrix[1][1] * scale,
        matrix[2][2] * scale,
    ];
    if !eigenvalues.iter().all(|value| value.is_finite())
        || !vectors.iter().flatten().all(|value| value.is_finite())
    {
        return Err(BasisFreeOptimizerError::SpectralDecompositionFailure);
    }
    Ok((eigenvalues, vectors))
}

#[allow(clippy::needless_range_loop)]
fn project_symmetric_floor_3(
    input: Dense3,
    lower_eigenvalue: f64,
) -> Result<Dense3, BasisFreeOptimizerError> {
    if !lower_eigenvalue.is_finite() {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }
    let (eigenvalues, vectors) = symmetric_eigen_3(input)?;
    let projected = [
        eigenvalues[0].max(lower_eigenvalue),
        eigenvalues[1].max(lower_eigenvalue),
        eigenvalues[2].max(lower_eigenvalue),
    ];
    let mut result = [[0.0; 3]; 3];
    for row in 0..3 {
        for column in 0..3 {
            for axis in 0..3 {
                result[row][column] +=
                    vectors[row][axis] * projected[axis] * vectors[column][axis];
            }
        }
    }
    if !result.iter().flatten().all(|value| value.is_finite()) {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }
    Ok(result)
}

#[derive(Clone, Copy)]
struct DualCandidate {
    epsilon_rate: f64,
    whitened_stiffness_rate: Dense3,
    passivity_residual_w: f64,
}

#[allow(clippy::too_many_arguments, clippy::needless_range_loop)]
fn evaluate_dual_candidate(
    multiplier: f64,
    epsilon_reference: f64,
    epsilon_lower: f64,
    epsilon_weight: f64,
    stiffness_reference: Dense3,
    stiffness_weight: f64,
    z: [f64; 3],
    stiffness_lower_eigenvalue: f64,
    a: f64,
    dissipation: f64,
) -> Result<DualCandidate, BasisFreeOptimizerError> {
    let epsilon_shift = multiplier * a / epsilon_weight;
    let epsilon_rate = (epsilon_reference - epsilon_shift).max(epsilon_lower);
    let rank_one_scale = multiplier / (2.0 * stiffness_weight);
    if !epsilon_shift.is_finite() || !epsilon_rate.is_finite() || !rank_one_scale.is_finite() {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    let mut shifted = stiffness_reference;
    for row in 0..3 {
        for column in 0..3 {
            shifted[row][column] -= rank_one_scale * z[row] * z[column];
        }
    }
    if !shifted.iter().flatten().all(|value| value.is_finite()) {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }
    let whitened_stiffness_rate =
        project_symmetric_floor_3(shifted, stiffness_lower_eigenvalue)?;
    let passivity_residual_w = a * epsilon_rate
        + 0.5 * quadratic_dense3(whitened_stiffness_rate, z)
        - dissipation;
    if !passivity_residual_w.is_finite() {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    Ok(DualCandidate {
        epsilon_rate,
        whitened_stiffness_rate,
        passivity_residual_w,
    })
}

/// Solve the basis-free scalar-inertia/full-symmetric-stiffness rate problem.
///
/// The continuous-time problem has one affine passivity half-space, a scalar
/// lower-rate barrier, and a Loewner PSD barrier. For this V0 constraint set,
/// feasibility is guaranteed for valid SPD Lambda/K, PSD D, positive decay
/// rates, and epsilon >= epsilon_min. Additional upper-rate/actuator bounds
/// are deliberately not silently added here.
///
/// A fixed-size Jacobi eigensolver is used only for the pointwise PSD-cone
/// projection. No generalized eigenvectors are selected, tracked, aligned,
/// or differentiated over time.
pub fn optimize_basis_free_compliance_rate(
    request: BasisFreeRateOptimizationRequest,
) -> Result<BasisFreeRateOptimizationReceipt, BasisFreeOptimizerError> {
    if !request.epsilon_rate_weight.is_finite()
        || request.epsilon_rate_weight <= 0.0
        || !request.stiffness_rate_weight.is_finite()
        || request.stiffness_rate_weight <= 0.0
    {
        return Err(BasisFreeOptimizerError::InvalidObjectiveWeight);
    }
    if !request.epsilon_rate_reference.is_finite()
        || [
            request.stiffness_rate_reference.xx,
            request.stiffness_rate_reference.yy,
            request.stiffness_rate_reference.zz,
            request.stiffness_rate_reference.xy,
            request.stiffness_rate_reference.xz,
            request.stiffness_rate_reference.yz,
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(BasisFreeOptimizerError::InvalidReferenceRate);
    }

    let epsilon_lower =
        -request.epsilon_decay_rate * (request.epsilon - request.epsilon_min);
    // Reuse the evaluator at the exact lower-rate boundary to validate all
    // physical matrices, scalar state, vectors, and tolerances before solving.
    let validation = evaluate_basis_free_compliance_rate(BasisFreeComplianceRequest {
        epsilon: request.epsilon,
        epsilon_min: request.epsilon_min,
        epsilon_decay_rate: request.epsilon_decay_rate,
        epsilon_rate: epsilon_lower,
        task_inertia: request.task_inertia,
        stiffness: request.stiffness,
        stiffness_rate: Symmetric3 {
            xx: -request.stiffness_decay_rate * request.stiffness.xx,
            yy: -request.stiffness_decay_rate * request.stiffness.yy,
            zz: -request.stiffness_decay_rate * request.stiffness.zz,
            xy: -request.stiffness_decay_rate * request.stiffness.xy,
            xz: -request.stiffness_decay_rate * request.stiffness.xz,
            yz: -request.stiffness_decay_rate * request.stiffness.yz,
        },
        damping: request.damping,
        stiffness_decay_rate: request.stiffness_decay_rate,
        task_error: request.task_error,
        task_error_velocity: request.task_error_velocity,
        matrix_tolerance: request.matrix_tolerance,
        passivity_tolerance_w: request.passivity_tolerance_w,
    })
    .map_err(|_| BasisFreeOptimizerError::IndependentPostCheckFailed)?;
    if !validation.candidate_admissible {
        return Err(BasisFreeOptimizerError::Infeasible);
    }
    let inertia_quadratic = request
        .task_inertia
        .quadratic_form(request.task_error_velocity)
        .map_err(|_| BasisFreeOptimizerError::ArithmeticOverflow)?;
    let dissipation = request
        .damping
        .quadratic_form(request.task_error_velocity)
        .map_err(|_| BasisFreeOptimizerError::ArithmeticOverflow)?;
    if dissipation < 0.0 {
        return Err(BasisFreeOptimizerError::NegativeDampingDissipation);
    }
    let a = 0.5 * inertia_quadratic;
    if !a.is_finite() || a < 0.0 || !dissipation.is_finite() {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    let stiffness_dense = dense_from_symmetric(request.stiffness);
    let lower = cholesky_lower_3(stiffness_dense)?;
    let inverse_lower = inverse_lower_3(lower)?;
    let inverse_lower_transpose = transpose_dense3(inverse_lower);
    let stiffness_reference = multiply_dense3(
        multiply_dense3(inverse_lower, dense_from_symmetric(request.stiffness_rate_reference)),
        inverse_lower_transpose,
    );
    if !stiffness_reference.iter().flatten().all(|value| value.is_finite()) {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    let z = dense_vector_product(transpose_dense3(lower), request.task_error);
    if !z.iter().all(|value| value.is_finite()) {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    let stiffness_lower_eigenvalue = -request.stiffness_decay_rate;
    let mut multiplier = 0.0_f64;
    let mut iterations = 0_u8;
    let mut candidate = evaluate_dual_candidate(
        multiplier,
        request.epsilon_rate_reference,
        epsilon_lower,
        request.epsilon_rate_weight,
        stiffness_reference,
        request.stiffness_rate_weight,
        z,
        stiffness_lower_eigenvalue,
        a,
        dissipation,
    )?;

    if candidate.passivity_residual_w > 0.0 {
        let z_norm_squared = z[0] * z[0] + z[1] * z[1] + z[2] * z[2];
        let minimum_residual = a * epsilon_lower
            - 0.5 * request.stiffness_decay_rate * z_norm_squared
            - dissipation;
        if !minimum_residual.is_finite() {
            return Err(BasisFreeOptimizerError::ArithmeticOverflow);
        }
        if minimum_residual > request.passivity_tolerance_w {
            return Err(BasisFreeOptimizerError::Infeasible);
        }

        let mut lower_multiplier = 0.0_f64;
        let mut upper_multiplier = 1.0_f64;
        let mut bracketed = false;
        for _ in 0..1024 {
            let upper_candidate = evaluate_dual_candidate(
                upper_multiplier,
                request.epsilon_rate_reference,
                epsilon_lower,
                request.epsilon_rate_weight,
                stiffness_reference,
                request.stiffness_rate_weight,
                z,
                stiffness_lower_eigenvalue,
                a,
                dissipation,
            )?;
            if upper_candidate.passivity_residual_w <= 0.0 {
                bracketed = true;
                break;
            }
            upper_multiplier *= 2.0;
            if !upper_multiplier.is_finite() {
                break;
            }
        }
        if !bracketed {
            return Err(BasisFreeOptimizerError::DualBracketFailure);
        }

        for index in 0..80 {
            let midpoint = lower_multiplier + 0.5 * (upper_multiplier - lower_multiplier);
            if midpoint == lower_multiplier || midpoint == upper_multiplier {
                break;
            }
            let midpoint_candidate = evaluate_dual_candidate(
                midpoint,
                request.epsilon_rate_reference,
                epsilon_lower,
                request.epsilon_rate_weight,
                stiffness_reference,
                request.stiffness_rate_weight,
                z,
                stiffness_lower_eigenvalue,
                a,
                dissipation,
            )?;
            if midpoint_candidate.passivity_residual_w > 0.0 {
                lower_multiplier = midpoint;
            } else {
                upper_multiplier = midpoint;
            }
            iterations = (index + 1) as u8;
        }

        multiplier = upper_multiplier;
        candidate = evaluate_dual_candidate(
            multiplier,
            request.epsilon_rate_reference,
            epsilon_lower,
            request.epsilon_rate_weight,
            stiffness_reference,
            request.stiffness_rate_weight,
            z,
            stiffness_lower_eigenvalue,
            a,
            dissipation,
        )?;
    }

    // Recover the optimizer result in the original task coordinates.
    let stiffness_rate_dense = multiply_dense3(
        multiply_dense3(lower, candidate.whitened_stiffness_rate),
        transpose_dense3(lower),
    );
    let stiffness_rate = symmetric_from_dense(stiffness_rate_dense)?;
    let independent = evaluate_basis_free_compliance_rate(BasisFreeComplianceRequest {
        epsilon: request.epsilon,
        epsilon_min: request.epsilon_min,
        epsilon_decay_rate: request.epsilon_decay_rate,
        epsilon_rate: candidate.epsilon_rate,
        task_inertia: request.task_inertia,
        stiffness: request.stiffness,
        stiffness_rate,
        damping: request.damping,
        stiffness_decay_rate: request.stiffness_decay_rate,
        task_error: request.task_error,
        task_error_velocity: request.task_error_velocity,
        matrix_tolerance: request.matrix_tolerance,
        passivity_tolerance_w: request.passivity_tolerance_w,
    })
    .map_err(|_| BasisFreeOptimizerError::IndependentPostCheckFailed)?;
    if !independent.candidate_admissible {
        return Err(BasisFreeOptimizerError::IndependentPostCheckFailed);
    }

    let (barrier_eigenvalues, _) =
        symmetric_eigen_3(dense_from_symmetric(independent.stiffness_barrier_matrix))?;
    let minimum_stiffness_barrier_eigenvalue = barrier_eigenvalues
        .into_iter()
        .fold(f64::INFINITY, f64::min);
    let barrier_scale = dense_max_abs(dense_from_symmetric(independent.stiffness_barrier_matrix));
    if minimum_stiffness_barrier_eigenvalue
        < -request.matrix_tolerance * barrier_scale
    {
        return Err(BasisFreeOptimizerError::IndependentPostCheckFailed);
    }
    let passivity_complementarity_residual =
        (multiplier * independent.passivity_lhs_w).abs();
    if !minimum_stiffness_barrier_eigenvalue.is_finite()
        || !passivity_complementarity_residual.is_finite()
    {
        return Err(BasisFreeOptimizerError::ArithmeticOverflow);
    }

    Ok(BasisFreeRateOptimizationReceipt {
        epsilon_rate: candidate.epsilon_rate,
        stiffness_rate,
        dual_multiplier: multiplier,
        passivity_residual_w: independent.passivity_lhs_w,
        passivity_slack_w: independent.passivity_slack_w,
        passivity_complementarity_residual,
        epsilon_rate_lower_bound: epsilon_lower,
        stiffness_barrier_matrix: independent.stiffness_barrier_matrix,
        minimum_stiffness_barrier_eigenvalue,
        dual_iterations: iterations,
        passivity_constraint_active: multiplier > 0.0,
        candidate_admissible: independent.candidate_admissible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagonal_request() -> BasisFreeComplianceRequest {
        BasisFreeComplianceRequest {
            epsilon: 1.0,
            epsilon_min: 0.5,
            epsilon_decay_rate: 1.0,
            epsilon_rate: -0.25,
            task_inertia: Symmetric3::diagonal(2.0, 3.0, 4.0),
            stiffness: Symmetric3::diagonal(100.0, 200.0, 300.0),
            stiffness_rate: Symmetric3::diagonal(-50.0, -60.0, -70.0),
            damping: Symmetric3::diagonal(10.0, 20.0, 30.0),
            stiffness_decay_rate: 1.0,
            task_error: [0.2, 0.3, 0.4],
            task_error_velocity: [1.0, 2.0, 3.0],
            matrix_tolerance: 1.0e-10,
            passivity_tolerance_w: 1.0e-10,
        }
    }

    #[test]
    fn symmetric_matrix_psd_checker_accepts_zero_and_positive_semidefinite_cases() {
        assert!(Symmetric3::diagonal(0.0, 0.0, 0.0)
            .is_positive_semidefinite(1.0e-12)
            .unwrap());
        assert!(Symmetric3::diagonal(1.0, 0.0, 2.0)
            .is_positive_semidefinite(1.0e-12)
            .unwrap());
        assert!(!Symmetric3::diagonal(1.0, -1.0, 2.0)
            .is_positive_semidefinite(1.0e-12)
            .unwrap());
    }

    #[test]
    fn symmetric_matrix_spd_checker_uses_sylvester_leading_minors() {
        assert!(Symmetric3::diagonal(1.0, 2.0, 3.0)
            .is_positive_definite(1.0e-12)
            .unwrap());
        assert!(!Symmetric3::diagonal(1.0, 0.0, 3.0)
            .is_positive_definite(1.0e-12)
            .unwrap());
        assert!(!Symmetric3::diagonal(-1.0, 2.0, 3.0)
            .is_positive_definite(1.0e-12)
            .unwrap());
    }

    #[test]
    fn repeated_eigenvalues_need_no_basis_or_eigenvector_tracking() {
        let repeated = Symmetric3::diagonal(5.0, 5.0, 2.0);
        assert!(repeated.is_positive_definite(1.0e-12).unwrap());

        let request = BasisFreeComplianceRequest {
            stiffness: repeated,
            stiffness_rate: Symmetric3::diagonal(-1.0, -1.0, -0.5),
            ..diagonal_request()
        };
        let receipt = evaluate_basis_free_compliance_rate(request).unwrap();
        assert!(receipt.candidate_admissible);
    }

    #[test]
    fn off_diagonal_stiffness_rate_is_evaluated_in_original_matrix_coordinates() {
        let mut request = diagonal_request();
        request.stiffness_rate = Symmetric3 {
            xx: -10.0,
            yy: -20.0,
            zz: -30.0,
            xy: 5.0,
            xz: 0.0,
            yz: 2.0,
        };
        let receipt = evaluate_basis_free_compliance_rate(request).unwrap();

        let expected = -10.0 * 0.04 - 20.0 * 0.09 - 30.0 * 0.16
            + 2.0 * 5.0 * 0.2 * 0.3
            + 2.0 * 2.0 * 0.3 * 0.4;
        assert!((receipt.passivity_lhs_w
            - (0.5 * -0.25 * 50.0 + 0.5 * expected - 360.0))
            .abs()
            < 1.0e-10);
    }

    #[test]
    fn basis_free_evaluator_accepts_a_passive_diagonal_candidate() {
        let receipt = evaluate_basis_free_compliance_rate(diagonal_request()).unwrap();
        assert!(receipt.epsilon_barrier_satisfied);
        assert!(receipt.stiffness_barrier_positive_semidefinite);
        assert!(receipt.passivity_satisfied);
        assert!(receipt.candidate_admissible);
        assert!(receipt.passivity_slack_w > 0.0);
    }

    #[test]
    fn stiffness_rate_must_preserve_the_loewner_decay_barrier() {
        let mut request = diagonal_request();
        request.stiffness_rate = Symmetric3::diagonal(-500.0, -60.0, -70.0);
        let receipt = evaluate_basis_free_compliance_rate(request).unwrap();

        assert!(!receipt.stiffness_barrier_positive_semidefinite);
        assert!(!receipt.candidate_admissible);
    }

    #[test]
    fn inertia_scale_rate_must_preserve_its_decay_barrier() {
        let mut request = diagonal_request();
        request.epsilon_rate = -0.75;
        let receipt = evaluate_basis_free_compliance_rate(request).unwrap();

        assert!(!receipt.epsilon_barrier_satisfied);
        assert!(!receipt.candidate_admissible);
    }

    #[test]
    fn invalid_task_inertia_stiffness_and_damping_are_rejected() {
        let mut request = diagonal_request();
        request.task_inertia = Symmetric3::diagonal(1.0, 0.0, 1.0);
        assert_eq!(
            evaluate_basis_free_compliance_rate(request),
            Err(ComplianceEvaluationError::TaskInertiaNotPositiveDefinite)
        );

        request = diagonal_request();
        request.stiffness = Symmetric3::diagonal(1.0, -1.0, 1.0);
        assert_eq!(
            evaluate_basis_free_compliance_rate(request),
            Err(ComplianceEvaluationError::StiffnessNotPositiveDefinite)
        );

        request = diagonal_request();
        request.damping = Symmetric3::diagonal(1.0, -1.0, 1.0);
        assert_eq!(
            evaluate_basis_free_compliance_rate(request),
            Err(ComplianceEvaluationError::DampingNotPositiveSemidefinite)
        );
    }

    #[test]
    fn candidate_evaluator_rejects_nonfinite_state_and_invalid_tolerances() {
        let mut request = diagonal_request();
        request.task_error[0] = f64::NAN;
        assert_eq!(
            evaluate_basis_free_compliance_rate(request),
            Err(ComplianceEvaluationError::Matrix(MatrixError::NonFiniteVector))
        );

        request = diagonal_request();
        request.matrix_tolerance = -1.0;
        assert_eq!(
            evaluate_basis_free_compliance_rate(request),
            Err(ComplianceEvaluationError::Matrix(MatrixError::InvalidTolerance))
        );
    }

    fn optimizer_request() -> BasisFreeRateOptimizationRequest {
        BasisFreeRateOptimizationRequest {
            epsilon: 1.0,
            epsilon_min: 0.0,
            epsilon_decay_rate: 1.0,
            epsilon_rate_reference: 1.0,
            task_inertia: Symmetric3::diagonal(1.0, 1.0, 1.0),
            stiffness: Symmetric3::diagonal(1.0, 1.0, 1.0),
            stiffness_rate_reference: Symmetric3::diagonal(2.0, 0.0, 0.0),
            damping: Symmetric3::diagonal(0.0, 0.0, 0.0),
            stiffness_decay_rate: 1.0,
            task_error: [1.0, 0.0, 0.0],
            task_error_velocity: [1.0, 0.0, 0.0],
            epsilon_rate_weight: 1.0,
            stiffness_rate_weight: 1.0,
            matrix_tolerance: 1.0e-10,
            passivity_tolerance_w: 1.0e-9,
        }
    }

    #[test]
    fn optimizer_matches_hand_solved_coupled_inertia_and_stiffness_projection() {
        let result = optimize_basis_free_compliance_rate(optimizer_request()).unwrap();
        assert!((result.epsilon_rate + 0.5).abs() < 1.0e-8);
        assert!((result.stiffness_rate.xx - 0.5).abs() < 1.0e-8);
        assert!(result.passivity_residual_w.abs() < 1.0e-8);
        assert!(result.minimum_stiffness_barrier_eigenvalue >= 0.999_999);
        assert!(result.candidate_admissible);
        assert!(result.passivity_constraint_active);
    }

    #[test]
    fn optimizer_projects_stiffness_when_velocity_is_zero() {
        let mut request = optimizer_request();
        request.task_error_velocity = [0.0; 3];
        request.epsilon_rate_reference = 0.3;
        request.stiffness_rate_reference = Symmetric3::diagonal(2.0, 0.0, 0.0);
        let result = optimize_basis_free_compliance_rate(request).unwrap();
        assert!((result.epsilon_rate - 0.3).abs() < 1.0e-10);
        assert!(result.stiffness_rate.xx.abs() < 1.0e-8);
        assert!(result.passivity_residual_w <= request.passivity_tolerance_w);
    }

    #[test]
    fn optimizer_projects_inertia_when_task_error_is_zero() {
        let mut request = optimizer_request();
        request.task_error = [0.0; 3];
        request.epsilon_rate_reference = 0.5;
        request.stiffness_rate_reference = Symmetric3::diagonal(0.0, 0.0, 0.0);
        let result = optimize_basis_free_compliance_rate(request).unwrap();
        assert!(result.epsilon_rate.abs() < 1.0e-8);
        assert!(result.stiffness_rate.xx.abs() < 1.0e-10);
        assert!(result.passivity_residual_w <= request.passivity_tolerance_w);
    }

    #[test]
    fn optimizer_handles_zero_state_and_repeated_projected_eigenvalues() {
        let mut request = optimizer_request();
        request.task_error = [0.0; 3];
        request.task_error_velocity = [0.0; 3];
        request.epsilon_rate_reference = -0.25;
        request.stiffness_rate_reference = Symmetric3::diagonal(-2.0, -2.0, 0.0);
        let result = optimize_basis_free_compliance_rate(request).unwrap();
        assert!((result.epsilon_rate + 0.25).abs() < 1.0e-10);
        assert!((result.stiffness_rate.xx + 1.0).abs() < 1.0e-8);
        assert!((result.stiffness_rate.yy + 1.0).abs() < 1.0e-8);
        assert!(result.minimum_stiffness_barrier_eigenvalue >= -1.0e-8);
        assert_eq!(result.dual_multiplier, 0.0);
    }

    #[test]
    fn optimizer_handles_epsilon_minimum_boundary_without_unbounded_dual_search() {
        let mut request = optimizer_request();
        request.epsilon = 0.5;
        request.epsilon_min = 0.5;
        request.task_error = [0.0; 3];
        request.epsilon_rate_reference = 0.75;
        request.stiffness_rate_reference = Symmetric3::diagonal(-0.3, -0.2, 0.0);
        let result = optimize_basis_free_compliance_rate(request).unwrap();
        assert!(result.epsilon_rate.abs() < 1.0e-8);
        assert!((result.stiffness_rate.xx + 0.3).abs() < 1.0e-8);
        assert!((result.stiffness_rate.yy + 0.2).abs() < 1.0e-8);
        assert!(result.passivity_residual_w <= request.passivity_tolerance_w);
    }

    #[test]
    fn optimizer_preserves_full_off_diagonal_matrix_structure() {
        let mut request = optimizer_request();
        request.stiffness = Symmetric3 {
            xx: 4.0, yy: 3.0, zz: 2.0, xy: 0.5, xz: 0.2, yz: -0.1,
        };
        request.stiffness_rate_reference = Symmetric3 {
            xx: 1.0, yy: -0.2, zz: 0.4, xy: 0.7, xz: -0.3, yz: 0.25,
        };
        request.task_error = [0.5, -0.25, 0.75];
        request.task_error_velocity = [0.2, 0.6, -0.1];
        let result = optimize_basis_free_compliance_rate(request).unwrap();
        assert!(result.candidate_admissible);
        assert!(result.passivity_residual_w <= request.passivity_tolerance_w);
        assert!(result.stiffness_barrier_matrix
            .is_positive_semidefinite(request.matrix_tolerance).unwrap());
        assert!(result.stiffness_rate.xy.is_finite());
        assert!(result.stiffness_rate.xz.is_finite());
        assert!(result.stiffness_rate.yz.is_finite());
    }


    #[test]
    fn optimizer_is_equivariant_under_orthogonal_task_coordinate_rotation() {
        let mut base_request = optimizer_request();
        base_request.epsilon = 1.0;
        base_request.epsilon_min = 0.0;
        let base = optimize_basis_free_compliance_rate(base_request).unwrap();

        let angle: f64 = 0.37;
        let cosine = angle.cos();
        let sine = angle.sin();
        let rotated_error = [cosine, sine, 0.0];
        let rotated_reference = Symmetric3 {
            xx: 2.0 * cosine * cosine,
            yy: 2.0 * sine * sine,
            zz: 0.0,
            xy: 2.0 * cosine * sine,
            xz: 0.0,
            yz: 0.0,
        };
        let mut rotated_request = optimizer_request();
        rotated_request.task_error = rotated_error;
        rotated_request.task_error_velocity = rotated_error;
        rotated_request.stiffness_rate_reference = rotated_reference;
        let rotated = optimize_basis_free_compliance_rate(rotated_request).unwrap();

        assert!((base.epsilon_rate - rotated.epsilon_rate).abs() < 1.0e-8);
        assert!((rotated.stiffness_rate.xx - 0.5 * cosine * cosine).abs() < 1.0e-8);
        assert!((rotated.stiffness_rate.yy - 0.5 * sine * sine).abs() < 1.0e-8);
        assert!((rotated.stiffness_rate.xy - 0.5 * cosine * sine).abs() < 1.0e-8);
        assert!(rotated.passivity_residual_w <= rotated_request.passivity_tolerance_w);
    }

    #[test]
    fn optimizer_rejects_nonpositive_objective_weights_and_bad_references() {
        let mut request = optimizer_request();
        request.epsilon_rate_weight = 0.0;
        assert_eq!(
            optimize_basis_free_compliance_rate(request),
            Err(BasisFreeOptimizerError::InvalidObjectiveWeight)
        );
        request = optimizer_request();
        request.stiffness_rate_reference.xy = f64::NAN;
        assert_eq!(
            optimize_basis_free_compliance_rate(request),
            Err(BasisFreeOptimizerError::InvalidReferenceRate)
        );
    }


}
