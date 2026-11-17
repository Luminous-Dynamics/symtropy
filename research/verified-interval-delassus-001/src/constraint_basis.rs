// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Outward-rounded covariance evidence for a change of constraint-equation basis.
//!
//! For r_new = T r_old, the represented constraint map transforms as
//! J_new = T J_old, the Delassus matrix as W_new = T W_old T^T, and its dual
//! multiplier as lambda_new = T^-T lambda_old. Generalized coordinates and
//! the mass metric are unchanged by this second, independent covariance axis.
//!
//! V0 accepts an exact binary64 point matrix T (each entry is an exact binary64
//! input) and interval-enclosed M, J, residual and multiplier data. All products
//! and solves after input admission use outward-rounded interval arithmetic.
//! It does not yet support uncertain interval-valued T.

use std::fmt;

use verified_interval_metric_envelope::{Interval, IntervalErrorV1};

use crate::delassus::{
    DerivedConstraintCoefficientsV1, IntervalMatrix, VerifiedDelassusErrorV1,
    VerifiedDelassusProfileV1, certify_delassus_coefficients,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintBasisErrorV1 {
    ZeroDimension,
    NonFiniteTransform,
    SingularTransform { pivot: usize },
    DelassusEnclosuresDisjoint { row: usize, column: usize },
    PairingEnclosuresDisjoint,
    GeneralizedForceEnclosuresDisjoint { coordinate: usize },
    CorrectionEnclosuresDisjoint { coordinate: usize },
    CorrectionNormEnclosuresDisjoint,
    CorrectionResidualNormIdentityNotEnclosed { transformed: bool },
    MultiplierNormEnclosuresDisjoint,
    ResidualNormEnclosuresDisjoint,
    ResidualEquationNotEnclosed { row: usize },
    MultiplierEquationNotEnclosed { index: usize },
    CertificateMismatch,
    Interval(IntervalErrorV1),
    Delassus(VerifiedDelassusErrorV1),
}

impl fmt::Display for ConstraintBasisErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConstraintBasisErrorV1 {}

impl From<IntervalErrorV1> for ConstraintBasisErrorV1 {
    fn from(value: IntervalErrorV1) -> Self {
        Self::Interval(value)
    }
}

impl From<VerifiedDelassusErrorV1> for ConstraintBasisErrorV1 {
    fn from(value: VerifiedDelassusErrorV1) -> Self {
        Self::Delassus(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintBasisStatusV1 {
    VerifiedCovariant,
}

/// Complete bounded evidence for a constraint-equation basis change.
#[derive(Debug, Clone, PartialEq)]
pub struct IntervalConstraintBasisCertificateV1<const N: usize, const K: usize> {
    transform: [[f64; K]; K],
    transformed_jacobian: IntervalMatrix<K, N>,
    delassus_from_congruence: IntervalMatrix<K, K>,
    delassus_common: IntervalMatrix<K, K>,
    transformed_residual: [Interval; K],
    transformed_multiplier: [Interval; K],
    generalized_force_original: [Interval; N],
    generalized_force_transformed: [Interval; N],
    minimum_kinetic_correction_original: [Interval; N],
    minimum_kinetic_correction_transformed: [Interval; N],
    dual_pairing_original: Interval,
    dual_pairing_transformed: Interval,
    multiplier_kinetic_norm_original: Interval,
    multiplier_kinetic_norm_transformed: Interval,
    residual_dual_norm_original: Interval,
    residual_dual_norm_transformed: Interval,
    correction_mass_norm_original: Interval,
    correction_mass_norm_transformed: Interval,
    original_coefficients: DerivedConstraintCoefficientsV1<N, K>,
    transformed_coefficients: DerivedConstraintCoefficientsV1<N, K>,
    status: ConstraintBasisStatusV1,
}

impl<const N: usize, const K: usize> IntervalConstraintBasisCertificateV1<N, K> {
    pub fn transform(&self) -> &[[f64; K]; K] {
        &self.transform
    }

    pub fn transformed_jacobian(&self) -> &IntervalMatrix<K, N> {
        &self.transformed_jacobian
    }

    pub fn delassus_from_congruence(&self) -> &IntervalMatrix<K, K> {
        &self.delassus_from_congruence
    }

    /// Intersection of the independently derived and congruence-derived W.
    pub fn delassus_common(&self) -> &IntervalMatrix<K, K> {
        &self.delassus_common
    }

    pub fn transformed_residual(&self) -> &[Interval; K] {
        &self.transformed_residual
    }

    pub fn transformed_multiplier(&self) -> &[Interval; K] {
        &self.transformed_multiplier
    }

    pub fn generalized_force_original(&self) -> &[Interval; N] {
        &self.generalized_force_original
    }

    pub fn generalized_force_transformed(&self) -> &[Interval; N] {
        &self.generalized_force_transformed
    }

    pub fn minimum_kinetic_correction_original(&self) -> &[Interval; N] {
        &self.minimum_kinetic_correction_original
    }

    pub fn minimum_kinetic_correction_transformed(&self) -> &[Interval; N] {
        &self.minimum_kinetic_correction_transformed
    }

    pub fn dual_pairing_original(&self) -> Interval {
        self.dual_pairing_original
    }

    pub fn dual_pairing_transformed(&self) -> Interval {
        self.dual_pairing_transformed
    }

    pub fn multiplier_kinetic_norm_original(&self) -> Interval {
        self.multiplier_kinetic_norm_original
    }

    pub fn multiplier_kinetic_norm_transformed(&self) -> Interval {
        self.multiplier_kinetic_norm_transformed
    }

    pub fn residual_dual_norm_original(&self) -> Interval {
        self.residual_dual_norm_original
    }

    pub fn residual_dual_norm_transformed(&self) -> Interval {
        self.residual_dual_norm_transformed
    }

    pub fn correction_mass_norm_original(&self) -> Interval {
        self.correction_mass_norm_original
    }

    pub fn correction_mass_norm_transformed(&self) -> Interval {
        self.correction_mass_norm_transformed
    }

    pub fn original_coefficients(&self) -> &DerivedConstraintCoefficientsV1<N, K> {
        &self.original_coefficients
    }

    pub fn transformed_coefficients(&self) -> &DerivedConstraintCoefficientsV1<N, K> {
        &self.transformed_coefficients
    }

    pub fn status(&self) -> ConstraintBasisStatusV1 {
        self.status
    }
}

fn interval_overlap(left: Interval, right: Interval) -> Option<Interval> {
    let lower = left.lower().max(right.lower());
    let upper = left.upper().min(right.upper());
    if lower <= upper {
        Interval::new(lower, upper).ok()
    } else {
        None
    }
}

fn interval_contains_interval(outer: Interval, inner: Interval) -> bool {
    outer.lower() <= inner.lower() && outer.upper() >= inner.upper()
}

fn require_vector_overlap<const N: usize>(
    left: &[Interval; N],
    right: &[Interval; N],
    error: impl Fn(usize) -> ConstraintBasisErrorV1,
) -> Result<(), ConstraintBasisErrorV1> {
    for index in 0..N {
        if interval_overlap(left[index], right[index]).is_none() {
            return Err(error(index));
        }
    }
    Ok(())
}

fn point_matrix_interval<const R: usize, const C: usize>(
    matrix: &[[f64; C]; R],
) -> Result<[[Interval; C]; R], ConstraintBasisErrorV1> {
    let mut result = [[Interval::zero(); C]; R];
    for (row, values) in matrix.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(ConstraintBasisErrorV1::NonFiniteTransform);
            }
            result[row][column] = Interval::point(*value)?;
        }
    }
    Ok(result)
}

fn transpose_interval<const R: usize, const C: usize>(
    matrix: &[[Interval; C]; R],
) -> [[Interval; R]; C] {
    let mut result = [[Interval::zero(); R]; C];
    for (row, values) in matrix.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            result[column][row] = *value;
        }
    }
    result
}

fn multiply_interval<const R: usize, const K: usize, const C: usize>(
    left: &[[Interval; K]; R],
    right: &[[Interval; C]; K],
) -> Result<[[Interval; C]; R], ConstraintBasisErrorV1> {
    let mut result = [[Interval::zero(); C]; R];
    for row in 0..R {
        for column in 0..C {
            let mut sum = Interval::zero();
            for inner in 0..K {
                sum = sum.add(left[row][inner].mul(right[inner][column])?)?;
            }
            result[row][column] = sum;
        }
    }
    Ok(result)
}

fn multiply_interval_vector<const R: usize, const K: usize>(
    matrix: &[[Interval; K]; R],
    vector: &[Interval; K],
) -> Result<[Interval; R], ConstraintBasisErrorV1> {
    let mut result = [Interval::zero(); R];
    for row in 0..R {
        let mut sum = Interval::zero();
        for column in 0..K {
            sum = sum.add(matrix[row][column].mul(vector[column])?)?;
        }
        result[row] = sum;
    }
    Ok(result)
}

fn multiply_point_interval<const R: usize, const K: usize, const C: usize>(
    left: &[[f64; K]; R],
    right: &[[Interval; C]; K],
) -> Result<[[Interval; C]; R], ConstraintBasisErrorV1> {
    let left_interval = point_matrix_interval(left)?;
    multiply_interval(&left_interval, right)
}

fn transform_vector<const K: usize>(
    transform: &[[f64; K]; K],
    vector: &[Interval; K],
) -> Result<[Interval; K], ConstraintBasisErrorV1> {
    let mut result = [Interval::zero(); K];
    for row in 0..K {
        let mut sum = Interval::zero();
        for column in 0..K {
            sum = sum.add(Interval::point(transform[row][column])?.mul(vector[column])?)?;
        }
        result[row] = sum;
    }
    Ok(result)
}

/// Solve the point-defined system A x = b using outward interval elimination.
///
/// A pivot is admitted only when its interval excludes zero. The matrix is
/// not inverted and no singular transform is regularized.
fn solve_point_system<const K: usize>(
    matrix: &[[f64; K]; K],
    rhs: &[Interval; K],
) -> Result<[Interval; K], ConstraintBasisErrorV1> {
    if K == 0 {
        return Err(ConstraintBasisErrorV1::ZeroDimension);
    }
    let mut left = point_matrix_interval(matrix)?;
    let mut right = *rhs;

    for column in 0..K {
        let mut selected = None;
        let mut selected_margin = 0.0_f64;
        for row in column..K {
            let pivot = left[row][column];
            let margin = if pivot.lower() > 0.0 {
                pivot.lower()
            } else if pivot.upper() < 0.0 {
                -pivot.upper()
            } else {
                0.0
            };
            if margin > selected_margin {
                selected = Some(row);
                selected_margin = margin;
            }
        }
        let pivot_row =
            selected.ok_or(ConstraintBasisErrorV1::SingularTransform { pivot: column })?;
        left.swap(column, pivot_row);
        right.swap(column, pivot_row);

        let pivot = left[column][column];
        for entry in column..K {
            left[column][entry] = left[column][entry].div(pivot)?;
        }
        right[column] = right[column].div(pivot)?;

        for row in 0..K {
            if row == column {
                continue;
            }
            let factor = left[row][column];
            for entry in column..K {
                left[row][entry] = left[row][entry].sub(factor.mul(left[column][entry])?)?;
            }
            right[row] = right[row].sub(factor.mul(right[column])?)?;
        }
    }
    Ok(right)
}

fn pairing<const K: usize>(
    covector: &[Interval; K],
    vector: &[Interval; K],
) -> Result<Interval, ConstraintBasisErrorV1> {
    let mut result = Interval::zero();
    for index in 0..K {
        result = result.add(covector[index].mul(vector[index])?)?;
    }
    Ok(result)
}

fn generalized_force<const N: usize, const K: usize>(
    jacobian: &IntervalMatrix<K, N>,
    multiplier: &[Interval; K],
) -> Result<[Interval; N], ConstraintBasisErrorV1> {
    let mut force = [Interval::zero(); N];
    for coordinate in 0..N {
        let mut sum = Interval::zero();
        for constraint in 0..K {
            sum = sum.add(jacobian[constraint][coordinate].mul(multiplier[constraint])?)?;
        }
        force[coordinate] = sum;
    }
    Ok(force)
}

fn solve_lower<const K: usize>(
    lower: &IntervalMatrix<K, K>,
    rhs: &[Interval; K],
) -> Result<[Interval; K], ConstraintBasisErrorV1> {
    let mut solution = [Interval::zero(); K];
    for row in 0..K {
        let mut value = rhs[row];
        for column in 0..row {
            value = value.sub(lower[row][column].mul(solution[column])?)?;
        }
        solution[row] = value.div(lower[row][row])?;
    }
    Ok(solution)
}

fn solve_upper_transpose<const K: usize>(
    lower: &IntervalMatrix<K, K>,
    rhs: &[Interval; K],
) -> Result<[Interval; K], ConstraintBasisErrorV1> {
    let mut solution = [Interval::zero(); K];
    for reverse in 0..K {
        let row = K - 1 - reverse;
        let mut value = rhs[row];
        for column in (row + 1)..K {
            value = value.sub(lower[column][row].mul(solution[column])?)?;
        }
        solution[row] = value.div(lower[row][row])?;
    }
    Ok(solution)
}

fn cholesky_interval<const K: usize>(
    matrix: &IntervalMatrix<K, K>,
) -> Result<IntervalMatrix<K, K>, ConstraintBasisErrorV1> {
    let mut lower = [[Interval::zero(); K]; K];
    for row in 0..K {
        for column in 0..=row {
            let mut correction = Interval::zero();
            for (left, right) in lower[row].iter().zip(lower[column].iter()).take(column) {
                correction = correction.add(left.mul(*right)?)?;
            }
            let pivot = matrix[row][column].sub(correction)?;
            if row == column {
                if pivot.lower() <= 0.0 {
                    return Err(ConstraintBasisErrorV1::Delassus(
                        VerifiedDelassusErrorV1::InconclusiveDelassusSpd {
                            pivot: row,
                            diagonal_lower: pivot.lower(),
                        },
                    ));
                }
                lower[row][column] = pivot.sqrt()?;
            } else {
                lower[row][column] = pivot.div(lower[column][column])?;
            }
        }
    }
    Ok(lower)
}

fn solve_spd<const K: usize>(
    matrix: &IntervalMatrix<K, K>,
    rhs: &[Interval; K],
) -> Result<[Interval; K], ConstraintBasisErrorV1> {
    let lower = cholesky_interval(matrix)?;
    let intermediate = solve_lower(&lower, rhs)?;
    solve_upper_transpose(&lower, &intermediate)
}

fn quadratic_form<const K: usize>(
    matrix: &IntervalMatrix<K, K>,
    vector: &[Interval; K],
) -> Result<Interval, ConstraintBasisErrorV1> {
    let mut sum = Interval::zero();
    for row in 0..K {
        for column in 0..K {
            sum = sum.add(vector[row].mul(matrix[row][column])?.mul(vector[column])?)?;
        }
    }
    if sum.upper() < 0.0 {
        return Err(ConstraintBasisErrorV1::Delassus(
            VerifiedDelassusErrorV1::NonFiniteBound,
        ));
    }
    Interval::new(sum.lower().max(0.0), sum.upper()).map_err(Into::into)
}

fn quadratic_norm<const K: usize>(
    matrix: &IntervalMatrix<K, K>,
    vector: &[Interval; K],
) -> Result<Interval, ConstraintBasisErrorV1> {
    Ok(quadratic_form(matrix, vector)?.sqrt()?)
}

fn mass_quadratic_form<const N: usize>(
    matrix: &IntervalMatrix<N, N>,
    vector: &[Interval; N],
) -> Result<Interval, ConstraintBasisErrorV1> {
    let mut sum = Interval::zero();
    for row in 0..N {
        for column in 0..N {
            sum = sum.add(vector[row].mul(matrix[row][column])?.mul(vector[column])?)?;
        }
    }
    if sum.upper() < 0.0 {
        return Err(ConstraintBasisErrorV1::Delassus(
            VerifiedDelassusErrorV1::NonFiniteBound,
        ));
    }
    Interval::new(sum.lower().max(0.0), sum.upper()).map_err(Into::into)
}

fn mass_quadratic_norm<const N: usize>(
    matrix: &IntervalMatrix<N, N>,
    vector: &[Interval; N],
) -> Result<Interval, ConstraintBasisErrorV1> {
    Ok(mass_quadratic_form(matrix, vector)?.sqrt()?)
}

fn multiply_b_vector<const N: usize, const K: usize>(
    b: &IntervalMatrix<N, K>,
    vector: &[Interval; K],
) -> Result<[Interval; N], ConstraintBasisErrorV1> {
    let mut result = [Interval::zero(); N];
    for row in 0..N {
        let mut sum = Interval::zero();
        for column in 0..K {
            sum = sum.add(b[row][column].mul(vector[column])?)?;
        }
        result[row] = sum;
    }
    Ok(result)
}

fn intersect_delassus<const K: usize>(
    congruence: &IntervalMatrix<K, K>,
    derived: &IntervalMatrix<K, K>,
) -> Result<IntervalMatrix<K, K>, ConstraintBasisErrorV1> {
    let mut common = [[Interval::zero(); K]; K];
    for row in 0..K {
        for column in row..K {
            let value = interval_overlap(congruence[row][column], congruence[column][row])
                .and_then(|value| interval_overlap(value, derived[row][column]))
                .and_then(|value| interval_overlap(value, derived[column][row]))
                .ok_or(ConstraintBasisErrorV1::DelassusEnclosuresDisjoint { row, column })?;
            common[row][column] = value;
            common[column][row] = value;
        }
    }
    Ok(common)
}

/// Compile interval coefficient, dual-pairing, generalized-force and
/// minimum-kinetic-correction evidence under r_new = T r_old.
pub fn certify_interval_constraint_basis_covariance<const N: usize, const K: usize>(
    mass: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
    transform: &[[f64; K]; K],
    residual_old: &[Interval; K],
    multiplier_old: &[Interval; K],
    profile: VerifiedDelassusProfileV1,
) -> Result<IntervalConstraintBasisCertificateV1<N, K>, ConstraintBasisErrorV1> {
    if N == 0 || K == 0 {
        return Err(ConstraintBasisErrorV1::ZeroDimension);
    }

    let original_coefficients = certify_delassus_coefficients(mass, jacobian, profile)?;

    // Prove the constraint-equation basis transform is nonsingular before
    // qualifying the transformed Jacobian or Delassus operator.
    let transform_transpose_point: [[f64; K]; K] =
        std::array::from_fn(|row| std::array::from_fn(|column| transform[column][row]));
    let transformed_multiplier = solve_point_system(&transform_transpose_point, multiplier_old)?;
    let transform_transpose_interval = point_matrix_interval(&transform_transpose_point)?;
    let multiplier_reconstructed =
        multiply_interval_vector(&transform_transpose_interval, &transformed_multiplier)?;
    for index in 0..K {
        if !interval_contains_interval(multiplier_reconstructed[index], multiplier_old[index]) {
            return Err(ConstraintBasisErrorV1::MultiplierEquationNotEnclosed { index });
        }
    }

    let transform_interval = point_matrix_interval(transform)?;
    let transform_transpose = transpose_interval(&transform_interval);
    let transformed_residual = transform_vector(transform, residual_old)?;

    let transformed_jacobian = multiply_point_interval(transform, jacobian)?;
    let transformed_coefficients =
        certify_delassus_coefficients(mass, &transformed_jacobian, profile)?;

    let left_product = multiply_interval(
        &transform_interval,
        original_coefficients.delassus_intervals(),
    )?;
    let delassus_from_congruence = multiply_interval(&left_product, &transform_transpose)?;
    let delassus_common = intersect_delassus(
        &delassus_from_congruence,
        transformed_coefficients.delassus_intervals(),
    )?;

    let dual_pairing_original = pairing(multiplier_old, residual_old)?;
    let dual_pairing_transformed = pairing(&transformed_multiplier, &transformed_residual)?;
    if interval_overlap(dual_pairing_original, dual_pairing_transformed).is_none() {
        return Err(ConstraintBasisErrorV1::PairingEnclosuresDisjoint);
    }

    let generalized_force_original = generalized_force(jacobian, multiplier_old)?;
    let generalized_force_transformed =
        generalized_force(&transformed_jacobian, &transformed_multiplier)?;
    require_vector_overlap(
        &generalized_force_original,
        &generalized_force_transformed,
        |coordinate| ConstraintBasisErrorV1::GeneralizedForceEnclosuresDisjoint { coordinate },
    )?;

    let residual_solution_original =
        solve_spd(original_coefficients.delassus_intervals(), residual_old)?;
    let residual_solution_transformed = solve_spd(&delassus_common, &transformed_residual)?;
    let minimum_kinetic_correction_original = multiply_b_vector(
        original_coefficients.inverse_mass_jacobian_transpose(),
        &residual_solution_original,
    )?;
    let minimum_kinetic_correction_transformed = multiply_b_vector(
        transformed_coefficients.inverse_mass_jacobian_transpose(),
        &residual_solution_transformed,
    )?;
    require_vector_overlap(
        &minimum_kinetic_correction_original,
        &minimum_kinetic_correction_transformed,
        |coordinate| ConstraintBasisErrorV1::CorrectionEnclosuresDisjoint { coordinate },
    )?;

    let correction_mass_norm_original =
        mass_quadratic_norm(mass, &minimum_kinetic_correction_original)?;
    let correction_mass_norm_transformed =
        mass_quadratic_norm(mass, &minimum_kinetic_correction_transformed)?;
    if interval_overlap(
        correction_mass_norm_original,
        correction_mass_norm_transformed,
    )
    .is_none()
    {
        return Err(ConstraintBasisErrorV1::CorrectionNormEnclosuresDisjoint);
    }

    let multiplier_kinetic_norm_original =
        quadratic_norm(original_coefficients.delassus_intervals(), multiplier_old)?;
    let multiplier_kinetic_norm_transformed =
        quadratic_norm(&delassus_common, &transformed_multiplier)?;
    if interval_overlap(
        multiplier_kinetic_norm_original,
        multiplier_kinetic_norm_transformed,
    )
    .is_none()
    {
        return Err(ConstraintBasisErrorV1::MultiplierNormEnclosuresDisjoint);
    }

    let residual_dual_norm_original = quadratic_norm(
        original_coefficients.delassus_intervals(),
        &residual_solution_original,
    )?;
    let residual_dual_norm_transformed =
        quadratic_norm(&delassus_common, &residual_solution_transformed)?;
    if interval_overlap(residual_dual_norm_original, residual_dual_norm_transformed).is_none() {
        return Err(ConstraintBasisErrorV1::ResidualNormEnclosuresDisjoint);
    }
    if interval_overlap(correction_mass_norm_original, residual_dual_norm_original).is_none() {
        return Err(
            ConstraintBasisErrorV1::CorrectionResidualNormIdentityNotEnclosed {
                transformed: false,
            },
        );
    }
    if interval_overlap(
        correction_mass_norm_transformed,
        residual_dual_norm_transformed,
    )
    .is_none()
    {
        return Err(ConstraintBasisErrorV1::CorrectionResidualNormIdentityNotEnclosed { transformed: true });
    }

    // Check J delta_v = r in both constraint representations.
    let residual_reconstructed_original =
        multiply_interval_vector(jacobian, &minimum_kinetic_correction_original)?;
    let residual_reconstructed_transformed = multiply_interval_vector(
        &transformed_jacobian,
        &minimum_kinetic_correction_transformed,
    )?;
    for row in 0..K {
        if !interval_contains_interval(residual_reconstructed_original[row], residual_old[row])
            || !interval_contains_interval(
                residual_reconstructed_transformed[row],
                transformed_residual[row],
            )
        {
            return Err(ConstraintBasisErrorV1::ResidualEquationNotEnclosed { row });
        }
    }

    Ok(IntervalConstraintBasisCertificateV1 {
        transform: *transform,
        transformed_jacobian,
        delassus_from_congruence,
        delassus_common,
        transformed_residual,
        transformed_multiplier,
        generalized_force_original,
        generalized_force_transformed,
        minimum_kinetic_correction_original,
        minimum_kinetic_correction_transformed,
        dual_pairing_original,
        dual_pairing_transformed,
        multiplier_kinetic_norm_original,
        multiplier_kinetic_norm_transformed,
        residual_dual_norm_original,
        residual_dual_norm_transformed,
        correction_mass_norm_original,
        correction_mass_norm_transformed,
        original_coefficients,
        transformed_coefficients,
        status: ConstraintBasisStatusV1::VerifiedCovariant,
    })
}

/// Replay a certificate from its authoritative M, J, T, residual and multiplier
/// inputs. Any mutation of the receipt or any mismatch in the source data fails.
pub fn verify_interval_constraint_basis_certificate<const N: usize, const K: usize>(
    mass: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
    transform: &[[f64; K]; K],
    residual_old: &[Interval; K],
    multiplier_old: &[Interval; K],
    profile: VerifiedDelassusProfileV1,
    receipt: &IntervalConstraintBasisCertificateV1<N, K>,
) -> Result<(), ConstraintBasisErrorV1> {
    let replay = certify_interval_constraint_basis_covariance(
        mass,
        jacobian,
        transform,
        residual_old,
        multiplier_old,
        profile,
    )?;
    if replay == *receipt {
        Ok(())
    } else {
        Err(ConstraintBasisErrorV1::CertificateMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> VerifiedDelassusProfileV1 {
        VerifiedDelassusProfileV1::default()
    }

    fn fixture() -> (IntervalMatrix<3, 3>, IntervalMatrix<2, 3>) {
        let mass = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]];
        let jacobian = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        (
            crate::point_interval_matrix(&mass).unwrap(),
            crate::point_interval_jacobian(&jacobian).unwrap(),
        )
    }

    #[test]
    fn upper_triangular_constraint_shear_preserves_all_physical_quantities() {
        let (mass, jacobian) = fixture();
        let transform = [[1.0, 2.0], [0.0, 1.0]];
        let residual = [
            Interval::point(0.25).unwrap(),
            Interval::point(-0.4).unwrap(),
        ];
        let multiplier = [
            Interval::point(0.75).unwrap(),
            Interval::point(-1.25).unwrap(),
        ];
        let certificate = certify_interval_constraint_basis_covariance(
            &mass,
            &jacobian,
            &transform,
            &residual,
            &multiplier,
            profile(),
        )
        .unwrap();

        assert_eq!(
            certificate.status(),
            ConstraintBasisStatusV1::VerifiedCovariant
        );
        assert!(
            interval_overlap(
                certificate.dual_pairing_original(),
                certificate.dual_pairing_transformed()
            )
            .is_some()
        );
        assert!(
            interval_overlap(
                certificate.correction_mass_norm_original(),
                certificate.correction_mass_norm_transformed()
            )
            .is_some()
        );
        assert!(
            interval_overlap(
                certificate.multiplier_kinetic_norm_original(),
                certificate.multiplier_kinetic_norm_transformed()
            )
            .is_some()
        );
        assert!(
            interval_overlap(
                certificate.residual_dual_norm_original(),
                certificate.residual_dual_norm_transformed()
            )
            .is_some()
        );
        verify_interval_constraint_basis_certificate(
            &mass,
            &jacobian,
            &transform,
            &residual,
            &multiplier,
            profile(),
            &certificate,
        )
        .unwrap();
    }

    #[test]
    fn row_scaling_is_admitted_and_uses_inverse_transpose_multiplier() {
        let (mass, jacobian) = fixture();
        let transform = [[2.0, 0.0], [0.0, 0.5]];
        let residual = [
            Interval::new(0.24, 0.26).unwrap(),
            Interval::new(-0.41, -0.39).unwrap(),
        ];
        let multiplier = [
            Interval::new(0.74, 0.76).unwrap(),
            Interval::new(-1.26, -1.24).unwrap(),
        ];
        let certificate = certify_interval_constraint_basis_covariance(
            &mass,
            &jacobian,
            &transform,
            &residual,
            &multiplier,
            profile(),
        )
        .unwrap();
        assert!(certificate.transformed_multiplier()[0].contains(0.375));
        assert!(certificate.transformed_multiplier()[1].contains(-2.5));
        assert!(certificate.transformed_residual()[0].contains(0.5));
        assert!(certificate.transformed_residual()[1].contains(-0.2));
    }

    #[test]
    fn singular_constraint_transform_is_rejected() {
        let (mass, jacobian) = fixture();
        let transform = [[1.0, 2.0], [2.0, 4.0]];
        let residual = [
            Interval::point(0.25).unwrap(),
            Interval::point(-0.4).unwrap(),
        ];
        let multiplier = [
            Interval::point(0.75).unwrap(),
            Interval::point(-1.25).unwrap(),
        ];
        assert!(matches!(
            certify_interval_constraint_basis_covariance(
                &mass,
                &jacobian,
                &transform,
                &residual,
                &multiplier,
                profile(),
            ),
            Err(ConstraintBasisErrorV1::SingularTransform { .. })
        ));
    }

    #[test]
    fn forged_receipt_and_stale_problem_inputs_are_rejected() {
        let (mass, jacobian) = fixture();
        let transform = [[1.0, 2.0], [0.0, 1.0]];
        let residual = [
            Interval::point(0.25).unwrap(),
            Interval::point(-0.4).unwrap(),
        ];
        let multiplier = [
            Interval::point(0.75).unwrap(),
            Interval::point(-1.25).unwrap(),
        ];
        let mut certificate = certify_interval_constraint_basis_covariance(
            &mass,
            &jacobian,
            &transform,
            &residual,
            &multiplier,
            profile(),
        )
        .unwrap();

        certificate.transformed_residual[0] = Interval::point(123.0).unwrap();
        assert_eq!(
            verify_interval_constraint_basis_certificate(
                &mass,
                &jacobian,
                &transform,
                &residual,
                &multiplier,
                profile(),
                &certificate,
            ),
            Err(ConstraintBasisErrorV1::CertificateMismatch)
        );

        let mut changed_jacobian = jacobian;
        changed_jacobian[0][0] = Interval::point(1.5).unwrap();
        assert!(
            verify_interval_constraint_basis_certificate(
                &mass,
                &changed_jacobian,
                &transform,
                &residual,
                &multiplier,
                profile(),
                &certificate,
            )
            .is_err()
        );
    }

    #[test]
    fn wrong_transpose_and_wrong_multiplier_variance_are_detected() {
        let (mass, jacobian) = fixture();
        let transform = [[1.0, 2.0], [0.0, 1.0]];
        let residual = [
            Interval::point(0.25).unwrap(),
            Interval::point(-0.4).unwrap(),
        ];
        let multiplier = [
            Interval::point(0.75).unwrap(),
            Interval::point(-1.25).unwrap(),
        ];
        let certificate = certify_interval_constraint_basis_covariance(
            &mass,
            &jacobian,
            &transform,
            &residual,
            &multiplier,
            profile(),
        )
        .unwrap();

        // Wrong force mapping uses T^T J, not T J.
        let transform_t = [[1.0, 0.0], [2.0, 1.0]];
        let wrong_j = multiply_point_interval(&transform_t, &jacobian).unwrap();
        assert_ne!(wrong_j, certificate.transformed_jacobian);

        // Wrong multiplier mapping uses T^-1 lambda instead of T^-T lambda.
        let wrong_multiplier = [
            Interval::point(multiplier[0].lower())
                .unwrap()
                .add(Interval::point(-2.0).unwrap().mul(multiplier[1]).unwrap())
                .unwrap(),
            multiplier[1],
        ];
        assert_ne!(wrong_multiplier, *certificate.transformed_multiplier());
    }
}
