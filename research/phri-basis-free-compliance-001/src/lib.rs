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
}
