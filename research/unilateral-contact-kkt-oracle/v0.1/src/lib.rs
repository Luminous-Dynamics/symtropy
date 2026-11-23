// SPDX-License-Identifier: AGPL-3.0-or-later
//! Active-set and omitted-contact KKT oracle for a restricted frictionless
//! unilateral impulse problem.
//!
//! The declared problem is:
//!
//!     minimize 0.5 * lambda^T A lambda + b^T lambda
//!     subject to lambda >= 0
//!
//! A must be symmetric positive semidefinite. Under this condition, KKT
//! feasibility is sufficient for global optimality of the declared QP.
//!
//! This is a research oracle, not Symtropy's production TGS Soft solver. It
//! does not model tangential friction, changing contact geometry during the
//! solve, or general nonconvex complementarity dynamics.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "unilateral-contact-kkt-oracle-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QpErrorKind {
    EmptyProblem,
    DimensionMismatch,
    InvalidContactId,
    DuplicateContactId,
    NonFiniteInput,
    NonFiniteEvaluation,
    InvalidTolerance,
    AsymmetricMatrix,
    NotPositiveSemidefinite,
    InvalidRetainedSet,
    ActiveSetBudgetExceeded,
    NoRestrictedKktSolution,
    LinearSolveResidualExceeded,
    NoOmittedSeparationWitness,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QpError {
    pub kind: QpErrorKind,
    pub reason: String,
}

impl QpError {
    fn new(kind: QpErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for QpError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for QpError {}

#[derive(Clone, Debug, PartialEq)]
pub struct UnilateralContactQp {
    pub contact_ids: Vec<String>,
    /// Symmetric Delassus/contact-response matrix A, in candidate ID order.
    pub delassus: Vec<Vec<f64>>,
    /// Free normal relative velocity plus fixed declared bias, in m/s.
    pub linear_term: Vec<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumericTolerances {
    pub matrix_relative: f64,
    pub pivot_relative: f64,
    pub solve_relative: f64,
    pub impulse_absolute: f64,
    pub gradient_absolute: f64,
    pub complementarity_absolute: f64,
}

impl Default for NumericTolerances {
    fn default() -> Self {
        Self {
            matrix_relative: 1.0e-10,
            pivot_relative: 1.0e-12,
            solve_relative: 1.0e-10,
            impulse_absolute: 1.0e-10,
            gradient_absolute: 1.0e-9,
            complementarity_absolute: 1.0e-9,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveSetConfig {
    pub maximum_active_sets: u64,
    pub tolerances: NumericTolerances,
}

impl Default for ActiveSetConfig {
    fn default() -> Self {
        Self {
            maximum_active_sets: 1_048_576,
            tolerances: NumericTolerances::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveConfig {
    pub maximum_refinement_rounds: u64,
    pub maximum_active_sets_per_round: u64,
    pub tolerances: NumericTolerances,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            maximum_refinement_rounds: 32,
            maximum_active_sets_per_round: 1_048_576,
            tolerances: NumericTolerances::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QpValidationReport {
    pub dimension: usize,
    pub maximum_symmetry_error: f64,
    pub minimum_ldlt_pivot: f64,
    pub matrix_tolerance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct KktCertificate {
    pub globally_optimal_within_declared_qp: bool,
    /// True only when the full KKT vector also has zero impulse on every omitted contact.
    pub retained_subset_is_globally_optimal: bool,
    pub maximum_primal_violation: f64,
    pub maximum_dual_violation: f64,
    pub maximum_complementarity_residual: f64,
    pub maximum_omitted_impulse: f64,
    pub minimum_omitted_gradient: Option<f64>,
    pub worst_omitted_contact_id: Option<String>,
    pub gradient: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactImpulse {
    pub contact_id: String,
    pub normal_impulse: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RestrictedSolution {
    /// Full-size impulse vector; omitted contacts are exactly zero.
    pub impulses: Vec<ContactImpulse>,
    pub objective_value: f64,
    pub active_set_subsets_evaluated: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AdaptiveStatus {
    FullPatchKktCertified,
    RefinementBudgetExhausted,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RefinementRound {
    pub round_index: u64,
    pub retained_contact_ids: Vec<String>,
    pub objective_value: f64,
    pub maximum_primal_violation: f64,
    pub maximum_dual_violation: f64,
    pub maximum_complementarity_residual: f64,
    pub minimum_omitted_gradient: Option<f64>,
    pub worst_omitted_contact_id: Option<String>,
    pub added_contact_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptiveReport {
    pub status: AdaptiveStatus,
    pub qp_validation: QpValidationReport,
    pub final_impulses: Vec<ContactImpulse>,
    pub final_certificate: KktCertificate,
    pub refinement_rounds: Vec<RefinementRound>,
    pub active_set_subsets_evaluated: u64,
}

#[derive(Clone, Debug)]
struct ValidatedQp {
    ids: Vec<String>,
    matrix: Vec<Vec<f64>>,
    linear: Vec<f64>,
    report: QpValidationReport,
}

#[derive(Clone, Debug)]
struct CandidateSolution {
    impulses: Vec<f64>,
    objective: f64,
    active_ids: Vec<String>,
}

fn tolerances_valid(t: NumericTolerances) -> bool {
    [
        t.matrix_relative,
        t.pivot_relative,
        t.solve_relative,
        t.impulse_absolute,
        t.gradient_absolute,
        t.complementarity_absolute,
    ]
    .iter()
    .all(|value| value.is_finite() && *value > 0.0)
}

fn max_abs_matrix(matrix: &[Vec<f64>]) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

fn validate_qp(
    qp: &UnilateralContactQp,
    tolerances: NumericTolerances,
) -> Result<ValidatedQp, QpError> {
    if !tolerances_valid(tolerances) {
        return Err(QpError::new(
            QpErrorKind::InvalidTolerance,
            "all numerical tolerances must be finite and strictly positive",
        ));
    }

    let dimension = qp.contact_ids.len();
    if dimension == 0 {
        return Err(QpError::new(
            QpErrorKind::EmptyProblem,
            "at least one full-patch candidate contact is required",
        ));
    }
    if qp.delassus.len() != dimension || qp.linear_term.len() != dimension {
        return Err(QpError::new(
            QpErrorKind::DimensionMismatch,
            "contact IDs, Delassus rows and linear-term length must match",
        ));
    }

    let mut ids = BTreeSet::new();
    for id in &qp.contact_ids {
        if id.trim().is_empty() {
            return Err(QpError::new(
                QpErrorKind::InvalidContactId,
                "contact IDs must be non-empty",
            ));
        }
        if !ids.insert(id.as_str()) {
            return Err(QpError::new(
                QpErrorKind::DuplicateContactId,
                "contact IDs must be unique",
            ));
        }
    }

    for row in &qp.delassus {
        if row.len() != dimension {
            return Err(QpError::new(
                QpErrorKind::DimensionMismatch,
                "Delassus matrix must be square",
            ));
        }
        if !row.iter().all(|value| value.is_finite()) {
            return Err(QpError::new(
                QpErrorKind::NonFiniteInput,
                "Delassus matrix values must be finite",
            ));
        }
    }
    if !qp.linear_term.iter().all(|value| value.is_finite()) {
        return Err(QpError::new(
            QpErrorKind::NonFiniteInput,
            "linear-term values must be finite",
        ));
    }

    let matrix_scale = max_abs_matrix(&qp.delassus).max(f64::MIN_POSITIVE);
    let matrix_tolerance = tolerances.matrix_relative * matrix_scale;
    let mut maximum_symmetry_error = 0.0_f64;
    let mut symmetric = vec![vec![0.0_f64; dimension]; dimension];
    for i in 0..dimension {
        for j in 0..dimension {
            let difference = (qp.delassus[i][j] - qp.delassus[j][i]).abs();
            maximum_symmetry_error = maximum_symmetry_error.max(difference);
            symmetric[i][j] = 0.5 * (qp.delassus[i][j] + qp.delassus[j][i]);
        }
    }
    if maximum_symmetry_error > matrix_tolerance {
        return Err(QpError::new(
            QpErrorKind::AsymmetricMatrix,
            "Delassus matrix is not symmetric within the declared relative tolerance",
        ));
    }

    let (minimum_ldlt_pivot, psd_tolerance) =
        validate_positive_semidefinite(&symmetric, tolerances.matrix_relative)?;
    Ok(ValidatedQp {
        ids: qp.contact_ids.clone(),
        matrix: symmetric,
        linear: qp.linear_term.clone(),
        report: QpValidationReport {
            dimension,
            maximum_symmetry_error,
            minimum_ldlt_pivot,
            matrix_tolerance: psd_tolerance,
        },
    })
}

/// Deterministic LDL^T check for symmetric positive semidefiniteness.
///
/// A near-zero pivot is accepted only if residual entries below that pivot are
/// also near zero. This supports common rank-deficient PSD contact matrices.
fn validate_positive_semidefinite(
    matrix: &[Vec<f64>],
    relative_tolerance: f64,
) -> Result<(f64, f64), QpError> {
    let dimension = matrix.len();
    let scale = max_abs_matrix(matrix).max(f64::MIN_POSITIVE);
    let tolerance = relative_tolerance * scale;
    let mut lower = vec![vec![0.0_f64; dimension]; dimension];
    let mut diagonal = vec![0.0_f64; dimension];
    let mut minimum_pivot = f64::INFINITY;

    for i in 0..dimension {
        lower[i][i] = 1.0;
        for j in 0..i {
            let mut residual = matrix[i][j];
            for k in 0..j {
                residual -= lower[i][k] * lower[j][k] * diagonal[k];
            }
            if !residual.is_finite() {
                return Err(QpError::new(
                    QpErrorKind::NonFiniteEvaluation,
                    "LDL factorization produced a non-finite residual",
                ));
            }
            if diagonal[j].abs() <= tolerance {
                if residual.abs() > tolerance {
                    return Err(QpError::new(
                        QpErrorKind::NotPositiveSemidefinite,
                        "zero LDL pivot has a non-zero residual column entry",
                    ));
                }
                lower[i][j] = 0.0;
            } else {
                lower[i][j] = residual / diagonal[j];
            }
        }

        let mut pivot = matrix[i][i];
        for k in 0..i {
            pivot -= lower[i][k] * lower[i][k] * diagonal[k];
        }
        if !pivot.is_finite() {
            return Err(QpError::new(
                QpErrorKind::NonFiniteEvaluation,
                "LDL factorization produced a non-finite pivot",
            ));
        }
        minimum_pivot = minimum_pivot.min(pivot);
        if pivot < -tolerance {
            return Err(QpError::new(
                QpErrorKind::NotPositiveSemidefinite,
                "Delassus matrix has a negative LDL pivot",
            ));
        }
        diagonal[i] = if pivot.abs() <= tolerance { 0.0 } else { pivot };
    }

    Ok((minimum_pivot, tolerance))
}

fn objective(matrix: &[Vec<f64>], linear: &[f64], impulses: &[f64]) -> f64 {
    let mut quadratic = 0.0_f64;
    let mut affine = 0.0_f64;
    for i in 0..impulses.len() {
        affine += linear[i] * impulses[i];
        for j in 0..impulses.len() {
            quadratic += impulses[i] * matrix[i][j] * impulses[j];
        }
    }
    0.5 * quadratic + affine
}

fn gradient(matrix: &[Vec<f64>], linear: &[f64], impulses: &[f64]) -> Vec<f64> {
    (0..impulses.len())
        .map(|i| {
            linear[i]
                + matrix[i]
                    .iter()
                    .zip(impulses)
                    .map(|(coefficient, impulse)| coefficient * impulse)
                    .sum::<f64>()
        })
        .collect()
}

fn solve_dense(
    mut matrix: Vec<Vec<f64>>,
    mut rhs: Vec<f64>,
    pivot_relative: f64,
    solve_relative: f64,
) -> Result<Option<Vec<f64>>, QpError> {
    let dimension = rhs.len();
    if dimension == 0 {
        return Ok(Some(Vec::new()));
    }

    let original_matrix = matrix.clone();
    let original_rhs = rhs.clone();
    let matrix_scale = max_abs_matrix(&matrix).max(f64::MIN_POSITIVE);
    let pivot_tolerance = pivot_relative * matrix_scale;

    for column in 0..dimension {
        let mut pivot_row = column;
        let mut pivot_value = matrix[column][column].abs();
        for row in (column + 1)..dimension {
            let candidate = matrix[row][column].abs();
            if candidate > pivot_value {
                pivot_value = candidate;
                pivot_row = row;
            }
        }
        if pivot_value <= pivot_tolerance {
            return Ok(None);
        }
        if pivot_row != column {
            matrix.swap(pivot_row, column);
            rhs.swap(pivot_row, column);
        }

        let pivot = matrix[column][column];
        for row in (column + 1)..dimension {
            let factor = matrix[row][column] / pivot;
            matrix[row][column] = 0.0;
            for j in (column + 1)..dimension {
                matrix[row][j] -= factor * matrix[column][j];
            }
            rhs[row] -= factor * rhs[column];
        }
    }

    let mut solution = vec![0.0_f64; dimension];
    for row in (0..dimension).rev() {
        let mut value = rhs[row];
        for j in (row + 1)..dimension {
            value -= matrix[row][j] * solution[j];
        }
        solution[row] = value / matrix[row][row];
        if !solution[row].is_finite() {
            return Ok(None);
        }
    }

    let solution_norm: f64 = solution.iter().map(|value| value.abs()).sum();
    let rhs_scale = original_rhs
        .iter()
        .fold(f64::MIN_POSITIVE, |maximum, value| maximum.max(value.abs()));
    let residual_scale = rhs_scale
        .max(matrix_scale * solution_norm)
        .max(f64::MIN_POSITIVE);
    let mut maximum_residual = 0.0_f64;
    for i in 0..dimension {
        let predicted = original_matrix[i]
            .iter()
            .zip(&solution)
            .map(|(coefficient, value)| coefficient * value)
            .sum::<f64>();
        if !predicted.is_finite() {
            return Err(QpError::new(
                QpErrorKind::NonFiniteEvaluation,
                "linear-system residual evaluation is non-finite",
            ));
        }
        maximum_residual = maximum_residual.max((predicted - original_rhs[i]).abs());
    }
    if maximum_residual > solve_relative * residual_scale {
        return Err(QpError::new(
            QpErrorKind::LinearSolveResidualExceeded,
            "active-set linear solve residual exceeds the declared relative tolerance",
        ));
    }
    Ok(Some(solution))
}

fn restricted_kkt_passes(
    matrix: &[Vec<f64>],
    linear: &[f64],
    impulses: &[f64],
    retained: &[usize],
    tolerances: NumericTolerances,
) -> bool {
    let values = gradient(matrix, linear, impulses);
    if !values.iter().all(|value| value.is_finite())
        || !impulses.iter().all(|value| value.is_finite())
    {
        return false;
    }
    retained.iter().all(|index| {
        impulses[*index] >= -tolerances.impulse_absolute
            && values[*index] >= -tolerances.gradient_absolute
            && (impulses[*index] * values[*index]).abs() <= tolerances.complementarity_absolute
    })
}

fn candidate_precedes(left: &CandidateSolution, right: &CandidateSolution) -> bool {
    left.objective
        .total_cmp(&right.objective)
        .then_with(|| left.active_ids.len().cmp(&right.active_ids.len()))
        .then_with(|| left.active_ids.cmp(&right.active_ids))
        .is_lt()
}

/// Solve the restricted non-negative QP by deterministic active-set enumeration.
///
/// The result is full-size and padded with zero impulses for omitted contacts.
/// Enumeration fails closed when the number of active subsets exceeds the budget.
pub fn solve_restricted_qp(
    qp: &UnilateralContactQp,
    retained_contact_ids: &[String],
    config: ActiveSetConfig,
) -> Result<RestrictedSolution, QpError> {
    let validated = validate_qp(qp, config.tolerances)?;
    solve_restricted_validated(&validated, retained_contact_ids, config)
}

fn solve_restricted_validated(
    qp: &ValidatedQp,
    retained_contact_ids: &[String],
    config: ActiveSetConfig,
) -> Result<RestrictedSolution, QpError> {
    if retained_contact_ids.len() >= 64 {
        return Err(QpError::new(
            QpErrorKind::ActiveSetBudgetExceeded,
            "active-set enumeration supports at most 63 retained contacts",
        ));
    }

    let mut retained = Vec::with_capacity(retained_contact_ids.len());
    let mut seen = BTreeSet::new();
    for id in retained_contact_ids {
        if !seen.insert(id.as_str()) {
            return Err(QpError::new(
                QpErrorKind::InvalidRetainedSet,
                "retained contact IDs must be unique",
            ));
        }
        let Some(index) = qp.ids.iter().position(|candidate| candidate == id) else {
            return Err(QpError::new(
                QpErrorKind::InvalidRetainedSet,
                "retained contact ID is not in the full candidate set",
            ));
        };
        retained.push(index);
    }
    retained.sort_by(|left, right| qp.ids[*left].cmp(&qp.ids[*right]));

    let subset_count = 1_u64.checked_shl(retained.len() as u32).ok_or_else(|| {
        QpError::new(
            QpErrorKind::ActiveSetBudgetExceeded,
            "active-set subset count overflow",
        )
    })?;
    if subset_count > config.maximum_active_sets {
        return Err(QpError::new(
            QpErrorKind::ActiveSetBudgetExceeded,
            "active-set subset count exceeds the configured budget",
        ));
    }

    let dimension = qp.ids.len();
    let mut best: Option<CandidateSolution> = None;
    for mask in 0..subset_count {
        let active = retained
            .iter()
            .enumerate()
            .filter_map(|(bit, index)| {
                if mask & (1_u64 << bit) != 0 {
                    Some(*index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        let mut active_matrix = vec![vec![0.0_f64; active.len()]; active.len()];
        let mut active_rhs = vec![0.0_f64; active.len()];
        for (row, original_row) in active.iter().enumerate() {
            active_rhs[row] = -qp.linear[*original_row];
            for (column, original_column) in active.iter().enumerate() {
                active_matrix[row][column] = qp.matrix[*original_row][*original_column];
            }
        }

        let active_solution = match solve_dense(
            active_matrix,
            active_rhs,
            config.tolerances.pivot_relative,
            config.tolerances.solve_relative,
        )? {
            Some(solution) => solution,
            None => continue,
        };

        let mut impulses = vec![0.0_f64; dimension];
        let mut invalid = false;
        for (position, index) in active.iter().enumerate() {
            let value = active_solution[position];
            if value < -config.tolerances.impulse_absolute {
                invalid = true;
                break;
            }
            impulses[*index] = value.max(0.0);
        }
        if invalid
            || !restricted_kkt_passes(
                &qp.matrix,
                &qp.linear,
                &impulses,
                &retained,
                config.tolerances,
            )
        {
            continue;
        }

        let active_ids = retained
            .iter()
            .filter_map(|index| {
                if impulses[*index] > config.tolerances.impulse_absolute {
                    Some(qp.ids[*index].clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let objective_value = objective(&qp.matrix, &qp.linear, &impulses);
        if !objective_value.is_finite() {
            return Err(QpError::new(
                QpErrorKind::NonFiniteEvaluation,
                "restricted QP objective is non-finite",
            ));
        }
        let candidate = CandidateSolution {
            objective: objective_value,
            impulses,
            active_ids,
        };
        if best
            .as_ref()
            .is_none_or(|incumbent| candidate_precedes(&candidate, incumbent))
        {
            best = Some(candidate);
        }
    }

    let Some(best) = best else {
        return Err(QpError::new(
            QpErrorKind::NoRestrictedKktSolution,
            "active-set enumeration found no restricted KKT solution",
        ));
    };

    Ok(RestrictedSolution {
        impulses: qp
            .ids
            .iter()
            .zip(best.impulses)
            .map(|(contact_id, normal_impulse)| ContactImpulse {
                contact_id: contact_id.clone(),
                normal_impulse,
            })
            .collect(),
        objective_value: best.objective,
        active_set_subsets_evaluated: subset_count,
    })
}

/// Verify full-patch KKT conditions for a restricted impulse vector.
///
/// The global-optimality flag is true only when A was validated as symmetric
/// PSD and every full-patch primal, dual and complementarity residual is within
/// its declared tolerance.
pub fn verify_full_patch_kkt(
    qp: &UnilateralContactQp,
    impulses: &[ContactImpulse],
    retained_contact_ids: &[String],
    tolerances: NumericTolerances,
) -> Result<KktCertificate, QpError> {
    let validated = validate_qp(qp, tolerances)?;
    verify_validated_kkt(&validated, impulses, retained_contact_ids, tolerances)
}

fn verify_validated_kkt(
    qp: &ValidatedQp,
    impulses: &[ContactImpulse],
    retained_contact_ids: &[String],
    tolerances: NumericTolerances,
) -> Result<KktCertificate, QpError> {
    if impulses.len() != qp.ids.len() {
        return Err(QpError::new(
            QpErrorKind::DimensionMismatch,
            "full-patch impulse vector must contain one value per candidate contact",
        ));
    }

    let mut retained = BTreeSet::new();
    for id in retained_contact_ids {
        if !retained.insert(id.as_str()) || !qp.ids.iter().any(|candidate| candidate == id) {
            return Err(QpError::new(
                QpErrorKind::InvalidRetainedSet,
                "retained IDs must be unique members of the full candidate set",
            ));
        }
    }

    let mut lambda = vec![0.0_f64; qp.ids.len()];
    for (position, impulse) in impulses.iter().enumerate() {
        if impulse.contact_id != qp.ids[position] || !impulse.normal_impulse.is_finite() {
            return Err(QpError::new(
                QpErrorKind::DimensionMismatch,
                "impulse IDs must match full candidate order and values must be finite",
            ));
        }
        lambda[position] = impulse.normal_impulse;
    }

    let dual = gradient(&qp.matrix, &qp.linear, &lambda);
    if !dual.iter().all(|value| value.is_finite()) {
        return Err(QpError::new(
            QpErrorKind::NonFiniteEvaluation,
            "full-patch KKT gradient is non-finite",
        ));
    }
    let mut primal_violation = 0.0_f64;
    let mut dual_violation = 0.0_f64;
    let mut complementarity = 0.0_f64;
    let mut maximum_omitted_impulse = 0.0_f64;
    let mut minimum_omitted: Option<f64> = None;
    let mut worst_omitted: Option<String> = None;

    for i in 0..qp.ids.len() {
        primal_violation = primal_violation.max((-lambda[i]).max(0.0));
        dual_violation = dual_violation.max((-dual[i]).max(0.0));
        complementarity = complementarity.max((lambda[i] * dual[i]).abs());

        if !retained.contains(qp.ids[i].as_str()) {
            maximum_omitted_impulse = maximum_omitted_impulse.max(lambda[i].abs());
            let is_worse = minimum_omitted.is_none_or(|minimum| {
                dual[i] < minimum
                    || (dual[i] == minimum
                        && worst_omitted
                            .as_ref()
                            .is_none_or(|current_id| qp.ids[i] < *current_id))
            });
            if is_worse {
                minimum_omitted = Some(dual[i]);
                worst_omitted = Some(qp.ids[i].clone());
            }
        }
    }

    let full_kkt_passes = primal_violation <= tolerances.impulse_absolute
        && dual_violation <= tolerances.gradient_absolute
        && complementarity <= tolerances.complementarity_absolute;
    let retained_subset_passes =
        full_kkt_passes && maximum_omitted_impulse <= tolerances.impulse_absolute;

    Ok(KktCertificate {
        globally_optimal_within_declared_qp: full_kkt_passes,
        retained_subset_is_globally_optimal: retained_subset_passes,
        maximum_primal_violation: primal_violation,
        maximum_dual_violation: dual_violation,
        maximum_complementarity_residual: complementarity,
        maximum_omitted_impulse,
        minimum_omitted_gradient: minimum_omitted,
        worst_omitted_contact_id: worst_omitted,
        gradient: dual,
    })
}

/// Solve a seed subset, add the most violated omitted contact, and repeat.
///
/// This is globally sufficient only for the declared convex frictionless
/// unilateral QP because every pass validates A as PSD and checks all
/// full-patch KKT conditions.
pub fn adaptive_refine(
    qp: &UnilateralContactQp,
    initial_retained_contact_ids: &[String],
    config: AdaptiveConfig,
) -> Result<AdaptiveReport, QpError> {
    let validated = validate_qp(qp, config.tolerances)?;
    let mut retained = initial_retained_contact_ids.to_vec();
    let mut rounds = Vec::new();
    let mut total_subsets = 0_u64;

    for round_index in 0..=config.maximum_refinement_rounds {
        let restricted = solve_restricted_validated(
            &validated,
            &retained,
            ActiveSetConfig {
                maximum_active_sets: config.maximum_active_sets_per_round,
                tolerances: config.tolerances,
            },
        )?;
        total_subsets = total_subsets
            .checked_add(restricted.active_set_subsets_evaluated)
            .ok_or_else(|| {
                QpError::new(
                    QpErrorKind::ActiveSetBudgetExceeded,
                    "cumulative active-set count overflow",
                )
            })?;

        let certificate = verify_validated_kkt(
            &validated,
            &restricted.impulses,
            &retained,
            config.tolerances,
        )?;
        let retained_sorted = retained
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect::<Vec<_>>();

        if certificate.retained_subset_is_globally_optimal {
            rounds.push(RefinementRound {
                round_index,
                retained_contact_ids: retained_sorted,
                objective_value: restricted.objective_value,
                maximum_primal_violation: certificate.maximum_primal_violation,
                maximum_dual_violation: certificate.maximum_dual_violation,
                maximum_complementarity_residual: certificate.maximum_complementarity_residual,
                minimum_omitted_gradient: certificate.minimum_omitted_gradient,
                worst_omitted_contact_id: certificate.worst_omitted_contact_id.clone(),
                added_contact_id: None,
            });
            return Ok(AdaptiveReport {
                status: AdaptiveStatus::FullPatchKktCertified,
                qp_validation: validated.report,
                final_impulses: restricted.impulses,
                final_certificate: certificate,
                refinement_rounds: rounds,
                active_set_subsets_evaluated: total_subsets,
            });
        }

        let worst = certificate
            .worst_omitted_contact_id
            .as_ref()
            .zip(certificate.minimum_omitted_gradient)
            .filter(|(_, value)| *value < -config.tolerances.gradient_absolute)
            .map(|(id, _)| id.clone());

        let Some(add_id) = worst else {
            return Err(QpError::new(
                QpErrorKind::NoOmittedSeparationWitness,
                "restricted residuals failed but no omitted contact has a negative-gradient separation witness",
            ));
        };

        if round_index < config.maximum_refinement_rounds {
            rounds.push(RefinementRound {
                round_index,
                retained_contact_ids: retained_sorted,
                objective_value: restricted.objective_value,
                maximum_primal_violation: certificate.maximum_primal_violation,
                maximum_dual_violation: certificate.maximum_dual_violation,
                maximum_complementarity_residual: certificate.maximum_complementarity_residual,
                minimum_omitted_gradient: certificate.minimum_omitted_gradient,
                worst_omitted_contact_id: certificate.worst_omitted_contact_id.clone(),
                added_contact_id: Some(add_id.clone()),
            });
            retained.push(add_id);
            continue;
        }

        rounds.push(RefinementRound {
            round_index,
            retained_contact_ids: retained_sorted,
            objective_value: restricted.objective_value,
            maximum_primal_violation: certificate.maximum_primal_violation,
            maximum_dual_violation: certificate.maximum_dual_violation,
            maximum_complementarity_residual: certificate.maximum_complementarity_residual,
            minimum_omitted_gradient: certificate.minimum_omitted_gradient,
            worst_omitted_contact_id: certificate.worst_omitted_contact_id.clone(),
            added_contact_id: None,
        });
        return Ok(AdaptiveReport {
            status: AdaptiveStatus::RefinementBudgetExhausted,
            qp_validation: validated.report,
            final_impulses: restricted.impulses,
            final_certificate: certificate,
            refinement_rounds: rounds,
            active_set_subsets_evaluated: total_subsets,
        });
    }

    Err(QpError::new(
        QpErrorKind::NoOmittedSeparationWitness,
        "adaptive refinement terminated without a final report",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qp(ids: &[&str], matrix: Vec<Vec<f64>>, linear: Vec<f64>) -> UnilateralContactQp {
        UnilateralContactQp {
            contact_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            delassus: matrix,
            linear_term: linear,
        }
    }

    fn strings(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn config() -> AdaptiveConfig {
        AdaptiveConfig {
            maximum_refinement_rounds: 8,
            maximum_active_sets_per_round: 1024,
            tolerances: NumericTolerances::default(),
        }
    }

    #[test]
    fn single_closing_contact_has_positive_impulse_and_zero_gradient() {
        let problem = qp(&["c1"], vec![vec![2.0]], vec![-4.0]);
        let report = adaptive_refine(&problem, &strings(&["c1"]), config()).expect("KKT pass");
        assert_eq!(report.status, AdaptiveStatus::FullPatchKktCertified);
        assert!((report.final_impulses[0].normal_impulse - 2.0).abs() < 1.0e-12);
        assert!(report.final_certificate.globally_optimal_within_declared_qp);
        assert!(report.final_certificate.maximum_dual_violation < 1.0e-12);
    }

    #[test]
    fn separating_contact_uses_zero_impulse_and_positive_gradient() {
        let problem = qp(&["c1"], vec![vec![2.0]], vec![1.0]);
        let report = adaptive_refine(&problem, &[], config()).expect("zero impulse KKT pass");
        assert_eq!(report.final_impulses[0].normal_impulse, 0.0);
        assert_eq!(report.final_certificate.gradient[0], 1.0);
        assert!(report.final_certificate.globally_optimal_within_declared_qp);
    }

    #[test]
    fn adaptive_separator_adds_only_the_violated_omitted_contact() {
        let problem = qp(
            &["c1", "c2", "c3"],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            vec![-2.0, -1.0, 0.5],
        );
        let report = adaptive_refine(&problem, &strings(&["c1"]), config())
            .expect("adaptive KKT certificate");
        assert_eq!(report.status, AdaptiveStatus::FullPatchKktCertified);
        assert_eq!(report.refinement_rounds.len(), 2);
        assert_eq!(
            report.refinement_rounds[0].added_contact_id.as_deref(),
            Some("c2")
        );
        assert_eq!(
            report
                .final_impulses
                .iter()
                .map(|item| item.normal_impulse)
                .collect::<Vec<_>>(),
            vec![2.0, 1.0, 0.0]
        );
        assert!(report.final_certificate.globally_optimal_within_declared_qp);
    }

    #[test]
    fn coupled_delassus_matrix_uses_omitted_gradient_as_witness() {
        let problem = qp(
            &["left", "right"],
            vec![vec![2.0, -1.0], vec![-1.0, 2.0]],
            vec![-1.0, -1.0],
        );
        let report =
            adaptive_refine(&problem, &strings(&["left"]), config()).expect("coupled contact KKT");
        assert_eq!(report.status, AdaptiveStatus::FullPatchKktCertified);
        assert_eq!(
            report.refinement_rounds[0].added_contact_id.as_deref(),
            Some("right")
        );
        assert!((report.final_impulses[0].normal_impulse - 1.0).abs() < 1.0e-12);
        assert!((report.final_impulses[1].normal_impulse - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn redundant_rank_deficient_psd_rows_can_be_certified_without_retaining_both() {
        let problem = qp(
            &["a", "b"],
            vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let report =
            adaptive_refine(&problem, &strings(&["a"]), config()).expect("redundant contact KKT");
        assert_eq!(report.status, AdaptiveStatus::FullPatchKktCertified);
        assert_eq!(report.final_impulses[0].normal_impulse, 1.0);
        assert_eq!(report.final_impulses[1].normal_impulse, 0.0);
        assert_eq!(report.final_certificate.gradient, vec![0.0, 0.0]);
        assert!(report.final_certificate.retained_subset_is_globally_optimal);
    }

    #[test]
    fn verifier_distinguishes_full_kkt_from_reduced_subset_support() {
        let problem = qp(
            &["kept", "omitted"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let full_solution = vec![
            ContactImpulse {
                contact_id: "kept".to_owned(),
                normal_impulse: 1.0,
            },
            ContactImpulse {
                contact_id: "omitted".to_owned(),
                normal_impulse: 1.0,
            },
        ];
        let certificate = verify_full_patch_kkt(
            &problem,
            &full_solution,
            &strings(&["kept"]),
            NumericTolerances::default(),
        )
        .expect("full KKT verification");
        assert!(certificate.globally_optimal_within_declared_qp);
        assert!(!certificate.retained_subset_is_globally_optimal);
        assert_eq!(certificate.maximum_omitted_impulse, 1.0);
    }

    #[test]
    fn indefinite_delassus_matrix_is_rejected() {
        let problem = qp(
            &["a", "b"],
            vec![vec![1.0, 2.0], vec![2.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let error = adaptive_refine(&problem, &strings(&["a"]), config())
            .expect_err("indefinite matrix must fail");
        assert_eq!(error.kind, QpErrorKind::NotPositiveSemidefinite);
    }

    #[test]
    fn asymmetric_matrix_is_rejected() {
        let problem = qp(
            &["a", "b"],
            vec![vec![1.0, 0.2], vec![0.4, 1.0]],
            vec![-1.0, -1.0],
        );
        let error =
            adaptive_refine(&problem, &strings(&["a"]), config()).expect_err("asymmetry must fail");
        assert_eq!(error.kind, QpErrorKind::AsymmetricMatrix);
    }

    #[test]
    fn wrong_dimensions_and_duplicate_ids_are_rejected() {
        let wrong = qp(&["a", "b"], vec![vec![1.0]], vec![-1.0, -1.0]);
        assert_eq!(
            adaptive_refine(&wrong, &strings(&["a"]), config())
                .expect_err("dimension mismatch")
                .kind,
            QpErrorKind::DimensionMismatch
        );

        let duplicate = qp(
            &["a", "a"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![-1.0, -1.0],
        );
        assert_eq!(
            adaptive_refine(&duplicate, &strings(&["a"]), config())
                .expect_err("duplicate IDs")
                .kind,
            QpErrorKind::DuplicateContactId
        );
    }

    #[test]
    fn input_order_permutation_preserves_id_keyed_solution() {
        let first = qp(
            &["a", "b", "c"],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            vec![-2.0, -1.0, 0.5],
        );
        let permuted = qp(
            &["c", "a", "b"],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            vec![0.5, -2.0, -1.0],
        );
        let first_report =
            adaptive_refine(&first, &strings(&["a"]), config()).expect("first report");
        let permuted_report =
            adaptive_refine(&permuted, &strings(&["a"]), config()).expect("permuted report");
        let first_map = first_report
            .final_impulses
            .into_iter()
            .map(|item| (item.contact_id, item.normal_impulse))
            .collect::<std::collections::BTreeMap<_, _>>();
        let permuted_map = permuted_report
            .final_impulses
            .into_iter()
            .map(|item| (item.contact_id, item.normal_impulse))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(first_map, permuted_map);
    }

    #[test]
    fn refinement_budget_exhaustion_is_typed_and_does_not_pass() {
        let problem = qp(
            &["a", "b"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let mut cfg = config();
        cfg.maximum_refinement_rounds = 0;
        let report = adaptive_refine(&problem, &strings(&["a"]), cfg).expect("bounded run");
        assert_eq!(report.status, AdaptiveStatus::RefinementBudgetExhausted);
        assert!(!report.final_certificate.globally_optimal_within_declared_qp);
    }

    #[test]
    fn active_set_budget_exhaustion_fails_closed() {
        let problem = qp(
            &["a", "b"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let error = solve_restricted_qp(
            &problem,
            &strings(&["a", "b"]),
            ActiveSetConfig {
                maximum_active_sets: 2,
                tolerances: NumericTolerances::default(),
            },
        )
        .expect_err("subset budget");
        assert_eq!(error.kind, QpErrorKind::ActiveSetBudgetExceeded);
    }

    #[test]
    fn stable_id_breaks_equal_omitted_gradient_ties() {
        let problem = qp(
            &["z", "a"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![-1.0, -1.0],
        );
        let report = adaptive_refine(&problem, &[], config()).expect("tie-break");
        assert_eq!(
            report.refinement_rounds[0].added_contact_id.as_deref(),
            Some("a")
        );
    }

    #[test]
    fn invalid_tolerances_are_rejected() {
        let problem = qp(&["a"], vec![vec![1.0]], vec![-1.0]);
        let mut cfg = config();
        cfg.tolerances.gradient_absolute = f64::NAN;
        assert_eq!(
            adaptive_refine(&problem, &[], cfg)
                .expect_err("NaN tolerance")
                .kind,
            QpErrorKind::InvalidTolerance
        );
    }
}
