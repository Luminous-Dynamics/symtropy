// SPDX-License-Identifier: AGPL-3.0-or-later
//! Inertia-whitened, rank-revealing contact normal-row seed selection.
//!
//! Given an SPD generalized mass matrix M = L L^T and normal contact Jacobian
//! J_n, this crate forms S = J_n L^-T using triangular solves only. It then
//! proposes a deterministic basis for the row space of S using pivoted,
//! twice-reorthogonalized modified Gram-Schmidt.
//!
//! IMPORTANT: linear dependence in the inertial row space is not proof that a
//! unilateral contact inequality or friction-cone complementarity row is
//! redundant. This crate is a seed proposer and numerical diagnostic only.
//! Physical omission must be established by a model-specific certificate such
//! as the full-patch KKT oracle for its admitted convex frictionless QP.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

pub const SCHEMA_VERSION: &str = "inertial-contact-rank-oracle-v0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankErrorKind {
    EmptyCandidates,
    DimensionMismatch,
    InvalidIdentity,
    DuplicateContactId,
    NonFiniteInput,
    InvalidTolerance,
    AsymmetricMass,
    NonPositiveDefiniteMass,
    IllConditionedMass,
    NonFiniteEvaluation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RankError {
    pub kind: RankErrorKind,
    pub reason: String,
}

impl RankError {
    fn new(kind: RankErrorKind, reason: impl Into<String>) -> Self {
        Self {
            kind,
            reason: reason.into(),
        }
    }
}

impl Display for RankError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.reason)
    }
}

impl Error for RankError {}

#[derive(Clone, Debug, PartialEq)]
pub struct InertialRankInput {
    /// Digest of the complete generated candidate population.
    pub candidate_set_digest: String,
    /// Digest of the exact generalized mass-properties input.
    pub mass_properties_digest: String,
    /// Stable IDs in the same order as the Jacobian rows.
    pub contact_ids: Vec<String>,
    /// Symmetric generalized mass matrix M, in declared generalized coordinates.
    pub mass_matrix: Vec<Vec<f64>>,
    /// Normal contact Jacobian J_n: one row per contact candidate.
    pub normal_jacobian: Vec<Vec<f64>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankSelectionConfig {
    /// Relative tolerance for symmetry of the input mass matrix.
    pub mass_symmetry_relative: f64,
    /// Relative Cholesky pivot floor relative to max(abs(M_ij)).
    /// Smaller pivots are reported as an ill-conditioned mass metric.
    pub mass_cholesky_relative_pivot: f64,
    /// RRQR-style relative row-rank threshold: eta = rank_relative * first_pivot.
    pub rank_relative: f64,
}

impl Default for RankSelectionConfig {
    fn default() -> Self {
        Self {
            mass_symmetry_relative: 1.0e-12,
            mass_cholesky_relative_pivot: 1.0e-14,
            rank_relative: 1.0e-10,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RowDisposition {
    SelectedBasis,
    BelowInertialRankThreshold,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContactRowDiagnostic {
    pub input_index: usize,
    pub contact_id: String,
    pub inertial_row_norm: f64,
    pub final_projection_residual: f64,
    pub relative_projection_residual: Option<f64>,
    pub disposition: RowDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InertialRankReport {
    pub schema_version: String,
    pub candidate_set_digest: String,
    pub mass_properties_digest: String,
    pub candidate_count: usize,
    pub generalized_dof: usize,
    pub selected_rank: usize,
    /// Candidate IDs ordered by deterministic pivot selection.
    pub selected_basis_contact_ids: Vec<String>,
    /// Excluded candidate IDs in stable lexical order.
    pub excluded_contact_ids: Vec<String>,
    /// Pivot residual norms in selection order. These are QR-style pivots,
    /// not singular values and not a spectral condition estimate.
    pub pivot_residual_norms: Vec<f64>,
    pub first_pivot_norm: f64,
    pub rank_threshold: f64,
    pub maximum_projection_residual: f64,
    pub maximum_relative_projection_residual: f64,
    pub maximum_mass_symmetry_error: f64,
    pub mass_symmetry_tolerance: f64,
    pub minimum_cholesky_diagonal: f64,
    pub minimum_cholesky_diagonal_ratio: f64,
    /// Always false in this seed-proposer version. A separate contact-law
    /// certificate must decide whether omitted rows may be removed.
    pub physical_omission_certified: bool,
    pub row_diagnostics: Vec<ContactRowDiagnostic>,
    /// Every candidate row S_i = J_i L^-T, in the input candidate order.
    /// This allows an independent consumer to reconstruct the Delassus Gram matrix.
    pub whitened_contact_rows: Vec<Vec<f64>>,
    /// Orthonormal basis vectors in whitened generalized-velocity coordinates.
    pub whitened_row_basis: Vec<Vec<f64>>,
}

struct CholeskyDiagnostics {
    factor: Vec<Vec<f64>>,
    maximum_symmetry_error: f64,
    symmetry_tolerance: f64,
    minimum_diagonal: f64,
}

type ResidualCandidate = Option<(Vec<f64>, f64)>;

fn finite_matrix(matrix: &[Vec<f64>]) -> bool {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .all(|value| value.is_finite())
}

fn max_abs_matrix(matrix: &[Vec<f64>]) -> f64 {
    matrix
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
}

fn stable_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, RankError> {
    let mut value = 0.0_f64;
    for (left_value, right_value) in left.iter().zip(right) {
        value += left_value * right_value;
        if !value.is_finite() {
            return Err(RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "whitened row dot product is non-finite",
            ));
        }
    }
    Ok(value)
}

fn tolerances_valid(config: RankSelectionConfig) -> bool {
    [
        config.mass_symmetry_relative,
        config.mass_cholesky_relative_pivot,
        config.rank_relative,
    ]
    .iter()
    .all(|value| value.is_finite() && *value > 0.0 && *value < 1.0)
}

/// Validate M and compute its lower Cholesky factor without forming M^-1.
fn cholesky_mass(
    mass: &[Vec<f64>],
    config: RankSelectionConfig,
) -> Result<CholeskyDiagnostics, RankError> {
    let dof = mass.len();
    if dof == 0 || mass.iter().any(|row| row.len() != dof) {
        return Err(RankError::new(
            RankErrorKind::DimensionMismatch,
            "mass matrix must be non-empty and square",
        ));
    }
    if !finite_matrix(mass) {
        return Err(RankError::new(
            RankErrorKind::NonFiniteInput,
            "mass matrix entries must be finite",
        ));
    }

    let mass_scale = max_abs_matrix(mass);
    if mass_scale == 0.0 {
        return Err(RankError::new(
            RankErrorKind::NonPositiveDefiniteMass,
            "zero mass matrix is not positive definite",
        ));
    }
    let symmetry_tolerance = config.mass_symmetry_relative * mass_scale;
    let mut maximum_symmetry_error = 0.0_f64;
    let mut symmetric = vec![vec![0.0_f64; dof]; dof];

    for i in 0..dof {
        for j in 0..dof {
            let difference = (mass[i][j] - mass[j][i]).abs();
            if !difference.is_finite() {
                return Err(RankError::new(
                    RankErrorKind::NonFiniteEvaluation,
                    "mass symmetry difference overflowed",
                ));
            }
            maximum_symmetry_error = maximum_symmetry_error.max(difference);
            symmetric[i][j] = 0.5 * mass[i][j] + 0.5 * mass[j][i];
        }
    }
    if maximum_symmetry_error > symmetry_tolerance {
        return Err(RankError::new(
            RankErrorKind::AsymmetricMass,
            "mass matrix is not symmetric within the declared relative tolerance",
        ));
    }

    let pivot_floor = config.mass_cholesky_relative_pivot * mass_scale;
    let mut lower = vec![vec![0.0_f64; dof]; dof];
    let mut minimum_diagonal = f64::INFINITY;

    for i in 0..dof {
        for j in 0..=i {
            let mut residual = symmetric[i][j];
            for (left, right) in lower[i].iter().take(j).zip(lower[j].iter().take(j)) {
                residual -= left * right;
            }
            if !residual.is_finite() {
                return Err(RankError::new(
                    RankErrorKind::NonFiniteEvaluation,
                    "mass Cholesky factorization produced a non-finite residual",
                ));
            }

            if i == j {
                if residual <= 0.0 {
                    return Err(RankError::new(
                        RankErrorKind::NonPositiveDefiniteMass,
                        "mass matrix has a non-positive Cholesky pivot",
                    ));
                }
                if residual <= pivot_floor {
                    return Err(RankError::new(
                        RankErrorKind::IllConditionedMass,
                        "mass Cholesky pivot is below the declared relative floor",
                    ));
                }
                let diagonal = residual.sqrt();
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err(RankError::new(
                        RankErrorKind::NonFiniteEvaluation,
                        "mass Cholesky diagonal is invalid",
                    ));
                }
                lower[i][j] = diagonal;
                minimum_diagonal = minimum_diagonal.min(diagonal);
            } else {
                let value = residual / lower[j][j];
                if !value.is_finite() {
                    return Err(RankError::new(
                        RankErrorKind::NonFiniteEvaluation,
                        "mass Cholesky off-diagonal entry is non-finite",
                    ));
                }
                lower[i][j] = value;
            }
        }
    }

    let maximum_diagonal = (0..dof)
        .map(|index| lower[index][index])
        .fold(0.0_f64, f64::max);
    let ratio = minimum_diagonal / maximum_diagonal;
    if !ratio.is_finite() || ratio <= 0.0 {
        return Err(RankError::new(
            RankErrorKind::NonFiniteEvaluation,
            "mass Cholesky diagonal ratio is invalid",
        ));
    }

    Ok(CholeskyDiagnostics {
        factor: lower,
        maximum_symmetry_error,
        symmetry_tolerance,
        minimum_diagonal,
    })
}

/// Solve L x = rhs for a lower-triangular Cholesky factor L.
fn solve_lower(lower: &[Vec<f64>], rhs: &[f64]) -> Result<Vec<f64>, RankError> {
    let dof = lower.len();
    if rhs.len() != dof {
        return Err(RankError::new(
            RankErrorKind::DimensionMismatch,
            "triangular solve right-hand side has the wrong dimension",
        ));
    }
    let mut solution = vec![0.0_f64; dof];
    for i in 0..dof {
        let mut value = rhs[i];
        for j in 0..i {
            value -= lower[i][j] * solution[j];
        }
        value /= lower[i][i];
        if !value.is_finite() {
            return Err(RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "inertia-whitening triangular solve is non-finite",
            ));
        }
        solution[i] = value;
    }
    Ok(solution)
}

/// Compute the twice-reorthogonalized residual of a row against the current
/// orthonormal basis. This is deterministic modified Gram-Schmidt.
fn orthogonal_residual(row: &[f64], basis: &[Vec<f64>]) -> Result<Vec<f64>, RankError> {
    let mut residual = row.to_vec();
    for _ in 0..2 {
        for direction in basis {
            let coefficient = dot(&residual, direction)?;
            for (value, basis_value) in residual.iter_mut().zip(direction) {
                *value -= coefficient * basis_value;
                if !value.is_finite() {
                    return Err(RankError::new(
                        RankErrorKind::NonFiniteEvaluation,
                        "orthogonal projection produced a non-finite residual",
                    ));
                }
            }
        }
    }
    Ok(residual)
}

fn residual_candidates(
    whitened_rows: &[Vec<f64>],
    basis: &[Vec<f64>],
    selected: &[bool],
) -> Result<Vec<ResidualCandidate>, RankError> {
    let mut output = Vec::with_capacity(whitened_rows.len());
    for (index, row) in whitened_rows.iter().enumerate() {
        if selected[index] {
            output.push(None);
            continue;
        }
        let residual = orthogonal_residual(row, basis)?;
        let norm = stable_norm(&residual);
        if !norm.is_finite() {
            return Err(RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "inertial projection residual norm is non-finite",
            ));
        }
        output.push(Some((residual, norm)));
    }
    Ok(output)
}

/// Build a deterministic inertial row-space basis.
///
/// This is an RRQR-style pivoted modified-Gram-Schmidt reference, not a
/// strong-RRQR or Householder-QR implementation. It proposes a seed subset;
/// it never certifies that excluded unilateral contacts are physically safe
/// to remove.
pub fn select_inertial_rank_basis(
    input: &InertialRankInput,
    config: RankSelectionConfig,
) -> Result<InertialRankReport, RankError> {
    if !tolerances_valid(config) {
        return Err(RankError::new(
            RankErrorKind::InvalidTolerance,
            "all relative tolerances must be finite and strictly between zero and one",
        ));
    }
    if input.candidate_set_digest.trim().is_empty()
        || input.mass_properties_digest.trim().is_empty()
    {
        return Err(RankError::new(
            RankErrorKind::InvalidIdentity,
            "candidate-set and mass-properties digests must be non-empty",
        ));
    }

    let candidate_count = input.contact_ids.len();
    if candidate_count == 0 {
        return Err(RankError::new(
            RankErrorKind::EmptyCandidates,
            "at least one contact candidate is required",
        ));
    }
    if input.normal_jacobian.len() != candidate_count {
        return Err(RankError::new(
            RankErrorKind::DimensionMismatch,
            "normal Jacobian must have one row per contact ID",
        ));
    }

    let mut ids = BTreeSet::new();
    for id in &input.contact_ids {
        if id.trim().is_empty() {
            return Err(RankError::new(
                RankErrorKind::InvalidIdentity,
                "contact IDs must be non-empty",
            ));
        }
        if !ids.insert(id.as_str()) {
            return Err(RankError::new(
                RankErrorKind::DuplicateContactId,
                "contact IDs must be unique",
            ));
        }
    }

    let dof = input.mass_matrix.len();
    if dof == 0 || input.normal_jacobian.iter().any(|row| row.len() != dof) {
        return Err(RankError::new(
            RankErrorKind::DimensionMismatch,
            "each normal Jacobian row must match the non-zero mass-matrix dimension",
        ));
    }
    if !finite_matrix(&input.normal_jacobian) {
        return Err(RankError::new(
            RankErrorKind::NonFiniteInput,
            "normal Jacobian entries must be finite",
        ));
    }

    let CholeskyDiagnostics {
        factor: lower,
        maximum_symmetry_error: symmetry_error,
        symmetry_tolerance,
        minimum_diagonal,
    } = cholesky_mass(&input.mass_matrix, config)?;
    let maximum_diagonal = (0..dof)
        .map(|index| lower[index][index])
        .fold(0.0_f64, f64::max);
    let minimum_diagonal_ratio = minimum_diagonal / maximum_diagonal;

    // S = J_n L^-T. Transposing each row gives S_i^T = L^-1 J_i^T,
    // which is evaluated with a lower-triangular solve.
    let mut whitened_rows = Vec::with_capacity(candidate_count);
    for row in &input.normal_jacobian {
        whitened_rows.push(solve_lower(&lower, row)?);
    }

    let inertial_row_norms = whitened_rows
        .iter()
        .map(|row| stable_norm(row))
        .collect::<Vec<_>>();
    if !inertial_row_norms.iter().all(|value| value.is_finite()) {
        return Err(RankError::new(
            RankErrorKind::NonFiniteEvaluation,
            "inertial row norm is non-finite",
        ));
    }
    let first_pivot_norm = inertial_row_norms.iter().copied().fold(0.0_f64, f64::max);
    let rank_threshold = config.rank_relative * first_pivot_norm;
    if !rank_threshold.is_finite() {
        return Err(RankError::new(
            RankErrorKind::NonFiniteEvaluation,
            "relative rank threshold overflowed",
        ));
    }

    let mut selected = vec![false; candidate_count];
    let mut basis: Vec<Vec<f64>> = Vec::new();
    let mut pivot_norms = Vec::new();
    let mut selected_indices = Vec::new();

    loop {
        let residuals = residual_candidates(&whitened_rows, &basis, &selected)?;
        let mut best_index: Option<usize> = None;
        let mut best_norm = 0.0_f64;
        let mut best_residual: Option<Vec<f64>> = None;

        for (index, candidate) in residuals.iter().enumerate() {
            let Some((residual, norm)) = candidate else {
                continue;
            };
            let should_replace = match best_index {
                None => true,
                Some(current) => {
                    *norm > best_norm
                        || (*norm == best_norm
                            && input.contact_ids[index] < input.contact_ids[current])
                }
            };
            if should_replace {
                best_index = Some(index);
                best_norm = *norm;
                best_residual = Some(residual.clone());
            }
        }

        let Some(index) = best_index else {
            break;
        };
        if best_norm <= rank_threshold {
            break;
        }

        let mut direction = best_residual.ok_or_else(|| {
            RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "pivot residual unexpectedly disappeared",
            )
        })?;
        for value in &mut direction {
            *value /= best_norm;
        }
        if !direction.iter().all(|value| value.is_finite()) {
            return Err(RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "normalized inertial basis vector is non-finite",
            ));
        }

        basis.push(direction);
        selected[index] = true;
        selected_indices.push(index);
        pivot_norms.push(best_norm);
    }

    let final_residuals = residual_candidates(&whitened_rows, &basis, &selected)?;
    let mut row_diagnostics = Vec::with_capacity(candidate_count);
    let mut maximum_projection_residual = 0.0_f64;
    let mut maximum_relative_projection_residual = 0.0_f64;
    let mut excluded_contact_ids = Vec::new();

    for (index, (candidate_residual, whitened_row)) in
        final_residuals.iter().zip(&whitened_rows).enumerate()
    {
        let residual_norm = match candidate_residual {
            Some((_, norm)) => *norm,
            None => stable_norm(&orthogonal_residual(whitened_row, &basis)?),
        };
        if !residual_norm.is_finite() {
            return Err(RankError::new(
                RankErrorKind::NonFiniteEvaluation,
                "final row projection residual is non-finite",
            ));
        }
        maximum_projection_residual = maximum_projection_residual.max(residual_norm);
        let relative_residual = if first_pivot_norm > 0.0 {
            Some(residual_norm / first_pivot_norm)
        } else {
            Some(0.0)
        };
        if let Some(value) = relative_residual {
            if !value.is_finite() {
                return Err(RankError::new(
                    RankErrorKind::NonFiniteEvaluation,
                    "relative row projection residual is non-finite",
                ));
            }
            maximum_relative_projection_residual = maximum_relative_projection_residual.max(value);
        }

        let disposition = if selected[index] {
            RowDisposition::SelectedBasis
        } else {
            excluded_contact_ids.push(input.contact_ids[index].clone());
            RowDisposition::BelowInertialRankThreshold
        };
        row_diagnostics.push(ContactRowDiagnostic {
            input_index: index,
            contact_id: input.contact_ids[index].clone(),
            inertial_row_norm: inertial_row_norms[index],
            final_projection_residual: residual_norm,
            relative_projection_residual: relative_residual,
            disposition,
        });
    }
    row_diagnostics.sort_by(|left, right| left.contact_id.cmp(&right.contact_id));
    excluded_contact_ids.sort();

    Ok(InertialRankReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        candidate_set_digest: input.candidate_set_digest.clone(),
        mass_properties_digest: input.mass_properties_digest.clone(),
        candidate_count,
        generalized_dof: dof,
        selected_rank: selected_indices.len(),
        selected_basis_contact_ids: selected_indices
            .iter()
            .map(|index| input.contact_ids[*index].clone())
            .collect(),
        excluded_contact_ids,
        pivot_residual_norms: pivot_norms,
        first_pivot_norm,
        rank_threshold,
        maximum_projection_residual,
        maximum_relative_projection_residual,
        maximum_mass_symmetry_error: symmetry_error,
        mass_symmetry_tolerance: symmetry_tolerance,
        minimum_cholesky_diagonal: minimum_diagonal,
        minimum_cholesky_diagonal_ratio: minimum_diagonal_ratio,
        physical_omission_certified: false,
        row_diagnostics,
        whitened_contact_rows: whitened_rows,
        whitened_row_basis: basis,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(ids: &[&str], mass: Vec<Vec<f64>>, jacobian: Vec<Vec<f64>>) -> InertialRankInput {
        InertialRankInput {
            candidate_set_digest: "candidate-set-fixture-001".to_owned(),
            mass_properties_digest: "mass-fixture-001".to_owned(),
            contact_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            mass_matrix: mass,
            normal_jacobian: jacobian,
        }
    }

    fn config() -> RankSelectionConfig {
        RankSelectionConfig::default()
    }

    fn multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let rows = left.len();
        let inner = right.len();
        let columns = right.first().map_or(0, Vec::len);
        let mut output = vec![vec![0.0; columns]; rows];
        for i in 0..rows {
            for k in 0..inner {
                for j in 0..columns {
                    output[i][j] += left[i][k] * right[k][j];
                }
            }
        }
        output
    }

    fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
        let rows = matrix.len();
        let columns = matrix.first().map_or(0, Vec::len);
        let mut output = vec![vec![0.0; rows]; columns];
        for (i, row) in matrix.iter().enumerate() {
            for (j, value) in row.iter().enumerate() {
                output[j][i] = *value;
            }
        }
        output
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "expected {left} ~= {right} within {tolerance}"
        );
    }

    #[test]
    fn inertial_whitening_selects_independent_rows_not_geometric_labels() {
        let problem = input(
            &[
                "base-contact",
                "tip-contact",
                "side-contact",
                "duplicate-base",
            ],
            vec![vec![4.0, 0.0], vec![0.0, 1.0]],
            vec![
                vec![2.0, 0.0],
                vec![4.0, 0.0],
                vec![0.0, 1.0],
                vec![-2.0, 0.0],
            ],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("valid rank basis");
        assert_eq!(report.selected_rank, 2);
        assert_eq!(
            report.selected_basis_contact_ids,
            vec!["tip-contact".to_owned(), "side-contact".to_owned()]
        );
        assert_eq!(
            report.excluded_contact_ids,
            vec!["base-contact".to_owned(), "duplicate-base".to_owned()]
        );
        assert!(report.maximum_projection_residual < 1.0e-12);
        assert!(!report.physical_omission_certified);
        close(report.first_pivot_norm, 2.0, 1.0e-12);
    }

    #[test]
    fn stable_contact_id_breaks_equal_pivot_ties() {
        let problem = input(
            &["z-contact", "a-contact"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![vec![1.0, 0.0], vec![-1.0, 0.0]],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("tie fixture");
        assert_eq!(report.selected_rank, 1);
        assert_eq!(report.selected_basis_contact_ids, vec!["a-contact"]);
        assert_eq!(report.excluded_contact_ids, vec!["z-contact"]);
    }

    #[test]
    fn negative_multiple_is_row_dependent_but_not_claimed_physically_redundant() {
        let problem = input(
            &["positive", "negative"],
            vec![vec![1.0]],
            vec![vec![2.0], vec![-4.0]],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("signed dependence");
        assert_eq!(report.selected_rank, 1);
        assert_eq!(report.excluded_contact_ids, vec!["positive"]);
        assert!(!report.physical_omission_certified);
    }

    #[test]
    fn near_dependent_rows_follow_the_declared_rank_threshold() {
        let problem = input(
            &["axis", "near-axis"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![vec![1.0, 0.0], vec![1.0, 1.0e-12]],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("near dependent");
        assert_eq!(report.selected_rank, 1);

        let stricter = RankSelectionConfig {
            rank_relative: 1.0e-14,
            ..config()
        };
        let report = select_inertial_rank_basis(&problem, stricter).expect("lower rank threshold");
        assert_eq!(report.selected_rank, 2);
    }

    #[test]
    fn zero_jacobian_rows_produce_empty_basis_not_a_false_rank() {
        let problem = input(
            &["zero-a", "zero-b"],
            vec![vec![2.0, 0.0], vec![0.0, 3.0]],
            vec![vec![0.0, 0.0], vec![0.0, 0.0]],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("zero rows");
        assert_eq!(report.selected_rank, 0);
        assert!(report.selected_basis_contact_ids.is_empty());
        assert_eq!(report.excluded_contact_ids.len(), 2);
        assert_eq!(report.maximum_projection_residual, 0.0);
    }

    #[test]
    fn input_order_permutation_preserves_selected_ids_and_pivots() {
        let first = input(
            &["a", "b", "c"],
            vec![vec![3.0, 0.5], vec![0.5, 2.0]],
            vec![vec![1.0, 0.0], vec![0.0, 2.0], vec![1.0, 2.0]],
        );
        let permuted = input(
            &["c", "a", "b"],
            vec![vec![3.0, 0.5], vec![0.5, 2.0]],
            vec![vec![1.0, 2.0], vec![1.0, 0.0], vec![0.0, 2.0]],
        );
        let a = select_inertial_rank_basis(&first, config()).expect("first");
        let b = select_inertial_rank_basis(&permuted, config()).expect("permuted");
        assert_eq!(a.selected_basis_contact_ids, b.selected_basis_contact_ids);
        assert_eq!(a.excluded_contact_ids, b.excluded_contact_ids);
        assert_eq!(a.pivot_residual_norms.len(), b.pivot_residual_norms.len());
        for (left, right) in a.pivot_residual_norms.iter().zip(&b.pivot_residual_norms) {
            close(*left, *right, 1.0e-12);
        }
    }

    #[test]
    fn whitened_candidate_rows_reconstruct_the_delassus_gram_matrix() {
        let mass = vec![vec![4.0, 0.5], vec![0.5, 2.0]];
        let jacobian = vec![vec![1.0, 0.0], vec![0.0, 2.0], vec![1.0, 1.0]];
        let report = select_inertial_rank_basis(&input(&["a", "b", "c"], mass, jacobian), config())
            .expect("rank report");
        assert_eq!(report.whitened_contact_rows.len(), 3);

        let reference_rows = [[1.0, 0.0], [0.0, 2.0], [1.0, 1.0]];
        let determinant = 4.0 * 2.0 - 0.5 * 0.5;
        for (whitened_i, row_i) in report
            .whitened_contact_rows
            .iter()
            .zip(reference_rows.iter())
        {
            for (whitened_j, row_j) in report
                .whitened_contact_rows
                .iter()
                .zip(reference_rows.iter())
            {
                let gram = dot(whitened_i, whitened_j).expect("finite whitened gram");
                let solved = [
                    (2.0 * row_j[0] - 0.5 * row_j[1]) / determinant,
                    (-0.5 * row_j[0] + 4.0 * row_j[1]) / determinant,
                ];
                let direct = row_i[0] * solved[0] + row_i[1] * solved[1];
                close(gram, direct, 1.0e-12);
            }
        }
    }

    #[test]
    fn dense_generalized_coordinate_rebasing_preserves_rank_and_pivots() {
        let mass = vec![vec![4.0, 0.5], vec![0.5, 2.0]];
        let jacobian = vec![
            vec![1.0, 0.0],
            vec![2.0, 1.0],
            vec![-1.0, 2.0],
            vec![1.0, 1.0],
        ];
        let transform = vec![vec![1.2, 0.3], vec![-0.4, 0.9]];

        let transformed_mass = multiply(&multiply(&transpose(&transform), &mass), &transform);
        let transformed_jacobian = multiply(&jacobian, &transform);

        let original =
            select_inertial_rank_basis(&input(&["a", "b", "c", "d"], mass, jacobian), config())
                .expect("original coordinates");
        let rebased = select_inertial_rank_basis(
            &input(
                &["a", "b", "c", "d"],
                transformed_mass,
                transformed_jacobian,
            ),
            config(),
        )
        .expect("rebased coordinates");

        assert_eq!(original.selected_rank, rebased.selected_rank);
        assert_eq!(
            original.selected_basis_contact_ids,
            rebased.selected_basis_contact_ids
        );
        for (left, right) in original
            .pivot_residual_norms
            .iter()
            .zip(&rebased.pivot_residual_norms)
        {
            close(*left, *right, 1.0e-10);
        }
        close(
            original.maximum_projection_residual,
            rebased.maximum_projection_residual,
            1.0e-10,
        );
    }

    #[test]
    fn asymmetric_mass_matrix_is_rejected() {
        let problem = input(
            &["a"],
            vec![vec![1.0, 0.2], vec![0.1, 1.0]],
            vec![vec![1.0, 0.0]],
        );
        let error = select_inertial_rank_basis(&problem, config()).expect_err("asymmetric mass");
        assert_eq!(error.kind, RankErrorKind::AsymmetricMass);
    }

    #[test]
    fn singular_and_indefinite_mass_matrices_are_rejected() {
        let singular = input(
            &["a"],
            vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            vec![vec![1.0, 0.0]],
        );
        assert_eq!(
            select_inertial_rank_basis(&singular, config())
                .expect_err("singular mass")
                .kind,
            RankErrorKind::NonPositiveDefiniteMass
        );

        let indefinite = input(
            &["a"],
            vec![vec![1.0, 2.0], vec![2.0, 1.0]],
            vec![vec![1.0, 0.0]],
        );
        assert_eq!(
            select_inertial_rank_basis(&indefinite, config())
                .expect_err("indefinite mass")
                .kind,
            RankErrorKind::NonPositiveDefiniteMass
        );
    }

    #[test]
    fn mass_pivot_floor_rejects_extreme_conditioning() {
        let problem = input(
            &["a"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0e-16]],
            vec![vec![1.0, 1.0]],
        );
        assert_eq!(
            select_inertial_rank_basis(&problem, config())
                .expect_err("below mass pivot floor")
                .kind,
            RankErrorKind::IllConditionedMass
        );
    }

    #[test]
    fn mismatched_dimensions_and_empty_id_are_rejected() {
        let bad_rows = input(
            &["a", "b"],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            vec![vec![1.0, 0.0]],
        );
        assert_eq!(
            select_inertial_rank_basis(&bad_rows, config())
                .expect_err("row count")
                .kind,
            RankErrorKind::DimensionMismatch
        );

        let bad_id = input(&[""], vec![vec![1.0]], vec![vec![1.0]]);
        assert_eq!(
            select_inertial_rank_basis(&bad_id, config())
                .expect_err("empty id")
                .kind,
            RankErrorKind::InvalidIdentity
        );
    }

    #[test]
    fn non_finite_mass_or_jacobian_is_rejected() {
        let mass = input(&["a"], vec![vec![f64::NAN]], vec![vec![1.0]]);
        assert_eq!(
            select_inertial_rank_basis(&mass, config())
                .expect_err("non-finite mass")
                .kind,
            RankErrorKind::NonFiniteInput
        );

        let jacobian = input(&["a"], vec![vec![1.0]], vec![vec![f64::INFINITY]]);
        assert_eq!(
            select_inertial_rank_basis(&jacobian, config())
                .expect_err("non-finite Jacobian")
                .kind,
            RankErrorKind::NonFiniteInput
        );
    }

    #[test]
    fn invalid_tolerances_and_missing_digests_are_rejected() {
        let problem = input(&["a"], vec![vec![1.0]], vec![vec![1.0]]);
        let invalid = RankSelectionConfig {
            rank_relative: 0.0,
            ..config()
        };
        assert_eq!(
            select_inertial_rank_basis(&problem, invalid)
                .expect_err("invalid rank tolerance")
                .kind,
            RankErrorKind::InvalidTolerance
        );

        let mut missing_digest = problem;
        missing_digest.mass_properties_digest.clear();
        assert_eq!(
            select_inertial_rank_basis(&missing_digest, config())
                .expect_err("missing digest")
                .kind,
            RankErrorKind::InvalidIdentity
        );
    }

    #[test]
    fn repeated_orthogonalization_controls_projection_loss() {
        let problem = input(
            &["x", "y", "z", "near"],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
            ],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 0.0],
                vec![0.0, 0.0, 1.0],
                vec![1.0, 1.0, 1.0 + 1.0e-13],
            ],
        );
        let report = select_inertial_rank_basis(&problem, config()).expect("reorthogonalization");
        assert_eq!(report.selected_rank, 3);
        assert!(report.maximum_projection_residual < 1.0e-12);
    }
}
