// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Outward-rounded coefficient provenance for constrained mechanics.
//!
//! From interval enclosures of symmetric SPD M and J, derive
//! B = M^-1 J^T and W = J M^-1 J^T using interval triangular solves.
//! Independently derive W = Y Y^T with Y = J L^-T and M = L L^T.
//! Intersect both outward enclosures, verify M B - J^T contains zero, and
//! admit only when interval Cholesky proves W strictly positive definite.

use std::fmt;

use verified_interval_metric_envelope::{Interval, IntervalErrorV1};

pub type IntervalMatrix<const R: usize, const C: usize> = [[Interval; C]; R];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedDelassusProfileV1 {
    pub minimum_mass_factor_diagonal_lower: f64,
    pub minimum_delassus_factor_diagonal_lower: f64,
}

impl Default for VerifiedDelassusProfileV1 {
    fn default() -> Self {
        Self {
            minimum_mass_factor_diagonal_lower: 0.0,
            minimum_delassus_factor_diagonal_lower: 0.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifiedDelassusErrorV1 {
    InvalidProfile,
    ZeroDimension,
    ConstraintRowsExceedDimension,
    NonSymmetricMassIntervals,
    InconclusiveMassSpd { pivot: usize, diagonal_lower: f64 },
    InconclusiveDelassusSpd { pivot: usize, diagonal_lower: f64 },
    IndependentDerivationsDisjoint { row: usize, column: usize },
    CoefficientResidualNotEnclosed {
        row: usize,
        column: usize,
        residual_lower: f64,
        residual_upper: f64,
    },
    NonFiniteBound,
    Interval(IntervalErrorV1),
}

impl fmt::Display for VerifiedDelassusErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for VerifiedDelassusErrorV1 {}
impl From<IntervalErrorV1> for VerifiedDelassusErrorV1 {
    fn from(value: IntervalErrorV1) -> Self {
        Self::Interval(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedDelassusStatusV1 {
    VerifiedSpdFullRowRank,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DerivedConstraintCoefficientsV1<const N: usize, const K: usize> {
    mass_intervals: IntervalMatrix<N, N>,
    jacobian_intervals: IntervalMatrix<K, N>,
    profile: VerifiedDelassusProfileV1,
    mass_cholesky: IntervalMatrix<N, N>,
    inverse_mass_jacobian_transpose: IntervalMatrix<N, K>,
    whitened_jacobian: IntervalMatrix<K, N>,
    delassus: IntervalMatrix<K, K>,
    mass_factor_diagonal_lower: f64,
    delassus_factor_diagonal_lower: f64,
    delassus_minimum_eigenvalue_lower: f64,
    delassus_operator_norm_upper: f64,
    delassus_inverse_operator_norm_upper: f64,
    status: VerifiedDelassusStatusV1,
}

impl<const N: usize, const K: usize> DerivedConstraintCoefficientsV1<N, K> {
    pub fn mass_intervals(&self) -> &IntervalMatrix<N, N> { &self.mass_intervals }
    pub fn jacobian_intervals(&self) -> &IntervalMatrix<K, N> { &self.jacobian_intervals }
    pub fn profile(&self) -> VerifiedDelassusProfileV1 { self.profile }
    pub fn mass_cholesky_intervals(&self) -> &IntervalMatrix<N, N> { &self.mass_cholesky }

    /// Outward enclosure of B = M^-1 J^T.
    pub fn inverse_mass_jacobian_transpose(&self) -> &IntervalMatrix<N, K> {
        &self.inverse_mass_jacobian_transpose
    }
    /// Outward enclosure of Y = J L^-T, where M = L L^T.
    pub fn whitened_jacobian(&self) -> &IntervalMatrix<K, N> { &self.whitened_jacobian }
    /// Outward enclosure of W = J M^-1 J^T.
    pub fn delassus_intervals(&self) -> &IntervalMatrix<K, K> { &self.delassus }
    pub fn mass_factor_diagonal_lower(&self) -> f64 { self.mass_factor_diagonal_lower }
    pub fn delassus_factor_diagonal_lower(&self) -> f64 { self.delassus_factor_diagonal_lower }
    pub fn delassus_minimum_eigenvalue_lower(&self) -> f64 {
        self.delassus_minimum_eigenvalue_lower
    }
    pub fn delassus_operator_norm_upper(&self) -> f64 { self.delassus_operator_norm_upper }
    pub fn delassus_inverse_operator_norm_upper(&self) -> f64 {
        self.delassus_inverse_operator_norm_upper
    }
    pub fn status(&self) -> VerifiedDelassusStatusV1 { self.status }

    /// Bound sqrt(r^T W^-1 r), the minimum kinetic norm required for J delta_v = r.
    pub fn residual_dual_norm_upper(
        &self,
        residual: &[Interval; K],
    ) -> Result<f64, VerifiedDelassusErrorV1> {
        let residual_norm = upper_l2_norm(residual)?;
        let denominator_lower = Interval::point(self.delassus_minimum_eigenvalue_lower)?
            .sqrt()?.lower();
        if denominator_lower <= 0.0 {
            return Err(VerifiedDelassusErrorV1::NonFiniteBound);
        }
        let bound = Interval::point(residual_norm)?
            .div(Interval::point(denominator_lower)?)?.upper();
        finite_bound(bound)
    }

    /// Bound sqrt(lambda^T W lambda) by sqrt(||W||_2) ||lambda||_2.
    pub fn multiplier_kinetic_norm_upper(
        &self,
        multiplier: &[Interval; K],
    ) -> Result<f64, VerifiedDelassusErrorV1> {
        let multiplier_norm = upper_l2_norm(multiplier)?;
        let operator_scale = Interval::point(self.delassus_operator_norm_upper)?
            .sqrt()?.upper();
        let bound = Interval::point(multiplier_norm)?
            .mul(Interval::point(operator_scale)?)?.upper();
        finite_bound(bound)
    }
}

fn validate_profile(p: VerifiedDelassusProfileV1) -> Result<(), VerifiedDelassusErrorV1> {
    if !p.minimum_mass_factor_diagonal_lower.is_finite()
        || p.minimum_mass_factor_diagonal_lower < 0.0
        || !p.minimum_delassus_factor_diagonal_lower.is_finite()
        || p.minimum_delassus_factor_diagonal_lower < 0.0
    {
        return Err(VerifiedDelassusErrorV1::InvalidProfile);
    }
    Ok(())
}

fn finite_bound(value: f64) -> Result<f64, VerifiedDelassusErrorV1> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(VerifiedDelassusErrorV1::NonFiniteBound)
    }
}

pub fn point_interval_matrix<const N: usize>(
    matrix: &[[f64; N]; N],
) -> Result<IntervalMatrix<N, N>, VerifiedDelassusErrorV1> {
    let mut result = [[Interval::zero(); N]; N];
    for row in 0..N {
        for column in 0..N {
            result[row][column] = Interval::point(matrix[row][column])?;
        }
    }
    Ok(result)
}

pub fn point_interval_jacobian<const K: usize, const N: usize>(
    jacobian: &[[f64; N]; K],
) -> Result<IntervalMatrix<K, N>, VerifiedDelassusErrorV1> {
    let mut result = [[Interval::zero(); N]; K];
    for row in 0..K {
        for column in 0..N {
            result[row][column] = Interval::point(jacobian[row][column])?;
        }
    }
    Ok(result)
}

fn validate_mass<const N: usize>(
    mass: &IntervalMatrix<N, N>,
) -> Result<(), VerifiedDelassusErrorV1> {
    if N == 0 {
        return Err(VerifiedDelassusErrorV1::ZeroDimension);
    }
    for row in 0..N {
        for column in 0..N {
            if mass[row][column] != mass[column][row] {
                return Err(VerifiedDelassusErrorV1::NonSymmetricMassIntervals);
            }
        }
    }
    Ok(())
}

fn spd_failure(
    mass_factor: bool,
    pivot: usize,
    lower: f64,
) -> VerifiedDelassusErrorV1 {
    if mass_factor {
        VerifiedDelassusErrorV1::InconclusiveMassSpd {
            pivot,
            diagonal_lower: lower,
        }
    } else {
        VerifiedDelassusErrorV1::InconclusiveDelassusSpd {
            pivot,
            diagonal_lower: lower,
        }
    }
}

/// Interval Cholesky. Every exact SPD matrix in the input interval family is
/// represented by a factor enclosed by this recurrence, provided each pivot
/// interval is proved strictly positive.
fn cholesky_interval<const N: usize>(
    matrix: &IntervalMatrix<N, N>,
    minimum_diagonal_lower: f64,
    mass_factor: bool,
) -> Result<(IntervalMatrix<N, N>, f64), VerifiedDelassusErrorV1> {
    let mut lower = [[Interval::zero(); N]; N];
    let mut minimum_seen = f64::INFINITY;

    for row in 0..N {
        for column in 0..=row {
            let mut correction = Interval::zero();
            for inner in 0..column {
                correction = correction.add(lower[row][inner].mul(lower[column][inner])?)?;
            }
            let pivot = matrix[row][column].sub(correction)?;
            if row == column {
                if pivot.lower() <= 0.0 {
                    return Err(spd_failure(mass_factor, row, pivot.lower()));
                }
                let diagonal = pivot.sqrt()?;
                if diagonal.lower() <= minimum_diagonal_lower {
                    return Err(spd_failure(mass_factor, row, diagonal.lower()));
                }
                minimum_seen = minimum_seen.min(diagonal.lower());
                lower[row][column] = diagonal;
            } else {
                lower[row][column] = pivot.div(lower[column][column])?;
            }
        }
    }
    Ok((lower, minimum_seen))
}

fn solve_lower<const N: usize>(
    lower: &IntervalMatrix<N, N>,
    rhs: &[Interval; N],
) -> Result<[Interval; N], VerifiedDelassusErrorV1> {
    let mut solution = [Interval::zero(); N];
    for row in 0..N {
        let mut value = rhs[row];
        for column in 0..row {
            value = value.sub(lower[row][column].mul(solution[column])?)?;
        }
        solution[row] = value.div(lower[row][row])?;
    }
    Ok(solution)
}

fn solve_upper_transpose<const N: usize>(
    lower: &IntervalMatrix<N, N>,
    rhs: &[Interval; N],
) -> Result<[Interval; N], VerifiedDelassusErrorV1> {
    let mut solution = [Interval::zero(); N];
    for reverse in 0..N {
        let row = N - 1 - reverse;
        let mut value = rhs[row];
        for column in (row + 1)..N {
            value = value.sub(lower[column][row].mul(solution[column])?)?;
        }
        solution[row] = value.div(lower[row][row])?;
    }
    Ok(solution)
}

fn interval_mass_solve<const N: usize>(
    lower: &IntervalMatrix<N, N>,
    rhs: &[Interval; N],
) -> Result<[Interval; N], VerifiedDelassusErrorV1> {
    let intermediate = solve_lower(lower, rhs)?;
    solve_upper_transpose(lower, &intermediate)
}

fn derive_b<const N: usize, const K: usize>(
    lower: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
) -> Result<IntervalMatrix<N, K>, VerifiedDelassusErrorV1> {
    let mut b = [[Interval::zero(); K]; N];
    for constraint in 0..K {
        let rhs: [Interval; N] =
            std::array::from_fn(|coordinate| jacobian[constraint][coordinate]);
        let solution = interval_mass_solve(lower, &rhs)?;
        for coordinate in 0..N {
            b[coordinate][constraint] = solution[coordinate];
        }
    }
    Ok(b)
}

fn derive_y<const N: usize, const K: usize>(
    lower: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
) -> Result<IntervalMatrix<K, N>, VerifiedDelassusErrorV1> {
    let mut y = [[Interval::zero(); N]; K];
    for constraint in 0..K {
        let rhs: [Interval; N] =
            std::array::from_fn(|coordinate| jacobian[constraint][coordinate]);
        y[constraint] = solve_lower(lower, &rhs)?;
    }
    Ok(y)
}

fn direct_w<const N: usize, const K: usize>(
    jacobian: &IntervalMatrix<K, N>,
    b: &IntervalMatrix<N, K>,
) -> Result<IntervalMatrix<K, K>, VerifiedDelassusErrorV1> {
    let mut w = [[Interval::zero(); K]; K];
    for row in 0..K {
        for column in 0..K {
            let mut sum = Interval::zero();
            for inner in 0..N {
                sum = sum.add(jacobian[row][inner].mul(b[inner][column])?)?;
            }
            w[row][column] = sum;
        }
    }
    Ok(w)
}

fn gram_w<const N: usize, const K: usize>(
    y: &IntervalMatrix<K, N>,
) -> Result<IntervalMatrix<K, K>, VerifiedDelassusErrorV1> {
    let mut w = [[Interval::zero(); K]; K];
    for row in 0..K {
        for column in 0..K {
            let mut sum = Interval::zero();
            for inner in 0..N {
                sum = sum.add(y[row][inner].mul(y[column][inner])?)?;
            }
            w[row][column] = sum;
        }
    }
    Ok(w)
}

fn intersect(left: Interval, right: Interval) -> Option<Interval> {
    let lower = left.lower().max(right.lower());
    let upper = left.upper().min(right.upper());
    if lower <= upper {
        Interval::new(lower, upper).ok()
    } else {
        None
    }
}

fn intersect_w<const K: usize>(
    direct: &IntervalMatrix<K, K>,
    gram: &IntervalMatrix<K, K>,
) -> Result<IntervalMatrix<K, K>, VerifiedDelassusErrorV1> {
    let mut w = [[Interval::zero(); K]; K];
    for row in 0..K {
        for column in row..K {
            let common = intersect(direct[row][column], gram[row][column])
                .and_then(|value| intersect(value, direct[column][row]))
                .and_then(|value| intersect(value, gram[column][row]))
                .ok_or(VerifiedDelassusErrorV1::IndependentDerivationsDisjoint {
                    row,
                    column,
                })?;
            w[row][column] = common;
            w[column][row] = common;
        }
    }
    Ok(w)
}

fn upper_l2<const N: usize>(
    values: &[Interval; N],
) -> Result<f64, VerifiedDelassusErrorV1> {
    let mut sum = Interval::zero();
    for value in values {
        let magnitude = Interval::point(value.max_abs())?;
        sum = sum.add(magnitude.mul(magnitude)?)?;
    }
    finite_bound(sum.sqrt()?.upper())
}

fn upper_frobenius<const R: usize, const C: usize>(
    matrix: &[[Interval; C]; R],
) -> Result<f64, VerifiedDelassusErrorV1> {
    let mut sum = Interval::zero();
    for value in matrix.iter().flatten() {
        let magnitude = Interval::point(value.max_abs())?;
        sum = sum.add(magnitude.mul(magnitude)?)?;
    }
    finite_bound(sum.sqrt()?.upper())
}

fn inverse_factor_frobenius_upper<const K: usize>(
    lower: &IntervalMatrix<K, K>,
) -> Result<f64, VerifiedDelassusErrorV1> {
    let mut inverse = [[Interval::zero(); K]; K];
    for column in 0..K {
        let mut rhs = [Interval::zero(); K];
        rhs[column] = Interval::point(1.0)?;
        let solution = solve_lower(lower, &rhs)?;
        for row in 0..K {
            inverse[row][column] = solution[row];
        }
    }
    upper_frobenius(&inverse)
}

fn certify_w<const K: usize>(
    w: &IntervalMatrix<K, K>,
    profile: VerifiedDelassusProfileV1,
) -> Result<(f64, f64, f64, f64), VerifiedDelassusErrorV1> {
    let (lower, factor_diagonal_lower) = cholesky_interval(
        w,
        profile.minimum_delassus_factor_diagonal_lower,
        false,
    )?;
    let inverse_factor_norm_upper = inverse_factor_frobenius_upper(&lower)?;
    let inverse_operator_norm_upper = Interval::point(inverse_factor_norm_upper)?
        .mul(Interval::point(inverse_factor_norm_upper)?)?
        .upper();
    let eigenvalue_lower = Interval::point(1.0)?
        .div(Interval::point(inverse_operator_norm_upper)?)?
        .lower();
    let operator_norm_upper = upper_frobenius(w)?;
    finite_bound(eigenvalue_lower)?;
    finite_bound(operator_norm_upper)?;
    finite_bound(inverse_operator_norm_upper)?;
    if eigenvalue_lower <= 0.0 {
        return Err(VerifiedDelassusErrorV1::InconclusiveDelassusSpd {
            pivot: K - 1,
            diagonal_lower: eigenvalue_lower,
        });
    }
    Ok((
        factor_diagonal_lower,
        eigenvalue_lower,
        operator_norm_upper,
        inverse_operator_norm_upper,
    ))
}

fn verify_mass_residual<const N: usize, const K: usize>(
    mass: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
    b: &IntervalMatrix<N, K>,
) -> Result<(), VerifiedDelassusErrorV1> {
    for row in 0..N {
        for column in 0..K {
            let mut computed = Interval::zero();
            for inner in 0..N {
                computed = computed.add(mass[row][inner].mul(b[inner][column])?)?;
            }
            let residual = computed.sub(jacobian[column][row])?;
            if !residual.contains(0.0) {
                return Err(VerifiedDelassusErrorV1::CoefficientResidualNotEnclosed {
                    row,
                    column,
                    residual_lower: residual.lower(),
                    residual_upper: residual.upper(),
                });
            }
        }
    }
    Ok(())
}

/// Derive B and W from interval-enclosed M and J and prove full-row-rank admission.
/// Caller-supplied B or W is intentionally not accepted.
pub fn certify_delassus_coefficients<const N: usize, const K: usize>(
    mass: &IntervalMatrix<N, N>,
    jacobian: &IntervalMatrix<K, N>,
    profile: VerifiedDelassusProfileV1,
) -> Result<DerivedConstraintCoefficientsV1<N, K>, VerifiedDelassusErrorV1> {
    validate_profile(profile)?;
    if N == 0 || K == 0 {
        return Err(VerifiedDelassusErrorV1::ZeroDimension);
    }
    if K > N {
        return Err(VerifiedDelassusErrorV1::ConstraintRowsExceedDimension);
    }
    validate_mass(mass)?;

    let (mass_cholesky, mass_factor_diagonal_lower) = cholesky_interval(
        mass,
        profile.minimum_mass_factor_diagonal_lower,
        true,
    )?;
    let b = derive_b(&mass_cholesky, jacobian)?;
    let y = derive_y(&mass_cholesky, jacobian)?;
    verify_mass_residual(mass, jacobian, &b)?;
    let direct = direct_w(jacobian, &b)?;
    let gram = gram_w(&y)?;
    let w = intersect_w(&direct, &gram)?;
    let (
        delassus_factor_diagonal_lower,
        delassus_minimum_eigenvalue_lower,
        delassus_operator_norm_upper,
        delassus_inverse_operator_norm_upper,
    ) = certify_w(&w, profile)?;

    Ok(DerivedConstraintCoefficientsV1 {
        mass_intervals: *mass,
        jacobian_intervals: *jacobian,
        profile,
        mass_cholesky,
        inverse_mass_jacobian_transpose: b,
        whitened_jacobian: y,
        delassus: w,
        mass_factor_diagonal_lower,
        delassus_factor_diagonal_lower,
        delassus_minimum_eigenvalue_lower,
        delassus_operator_norm_upper,
        delassus_inverse_operator_norm_upper,
        status: VerifiedDelassusStatusV1::VerifiedSpdFullRowRank,
    })
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
            point_interval_matrix(&mass).unwrap(),
            point_interval_jacobian(&jacobian).unwrap(),
        )
    }

    fn point_multiply<const R: usize, const K: usize, const C: usize>(
        left: &[[f64; K]; R],
        right: &[[f64; C]; K],
    ) -> [[f64; C]; R] {
        std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                (0..K).map(|inner| left[row][inner] * right[inner][column]).sum()
            })
        })
    }

    fn point_transpose<const R: usize, const C: usize>(
        matrix: &[[f64; C]; R],
    ) -> [[f64; R]; C] {
        std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
    }

    #[test]
    fn derives_b_and_w_with_direct_and_whitened_formulations() {
        let (mass, jacobian) = fixture();
        let certificate = certify_delassus_coefficients(&mass, &jacobian, profile()).unwrap();
        assert_eq!(
            certificate.status(),
            VerifiedDelassusStatusV1::VerifiedSpdFullRowRank
        );
        let expected_b = [[0.5, 0.0], [0.0, 2.0 / 3.0], [0.2, -0.2]];
        for row in 0..3 {
            for column in 0..2 {
                assert!(certificate.inverse_mass_jacobian_transpose()[row][column]
                    .contains(expected_b[row][column]));
            }
        }
        let expected_w = [[0.7, -0.2], [-0.2, 23.0 / 15.0]];
        for row in 0..2 {
            for column in 0..2 {
                assert!(certificate.delassus_intervals()[row][column]
                    .contains(expected_w[row][column]));
            }
        }
        assert!(certificate.delassus_minimum_eigenvalue_lower() > 0.0);
        assert!(certificate.delassus_operator_norm_upper() > 0.0);
        assert!(certificate.delassus_inverse_operator_norm_upper() > 0.0);
    }

    #[test]
    fn uncertain_mass_and_jacobian_enclosures_cover_corner_samples() {
        let mass = [
            [Interval::new(1.999, 2.001).unwrap(), Interval::zero(), Interval::zero()],
            [Interval::zero(), Interval::new(2.999, 3.001).unwrap(), Interval::zero()],
            [Interval::zero(), Interval::zero(), Interval::new(4.999, 5.001).unwrap()],
        ];
        let jacobian = [
            [Interval::new(0.999, 1.001).unwrap(), Interval::zero(), Interval::new(0.999, 1.001).unwrap()],
            [Interval::zero(), Interval::new(1.999, 2.001).unwrap(), Interval::new(-1.001, -0.999).unwrap()],
        ];
        let c = certify_delassus_coefficients(&mass, &jacobian, profile()).unwrap();
        for (m0, m1, m2, j00, j02, j11, j12) in [
            (1.999, 2.999, 4.999, 0.999, 1.001, 1.999, -1.001),
            (2.001, 3.001, 5.001, 1.001, 0.999, 2.001, -0.999),
            (2.000, 3.000, 5.000, 1.000, 1.000, 2.000, -1.000),
        ] {
            let w00 = j00 * j00 / m0 + j02 * j02 / m2;
            let w01 = j02 * j12 / m2;
            let w11 = j11 * j11 / m1 + j12 * j12 / m2;
            assert!(c.delassus_intervals()[0][0].contains(w00));
            assert!(c.delassus_intervals()[0][1].contains(w01));
            assert!(c.delassus_intervals()[1][1].contains(w11));
        }
    }

    #[test]
    fn dense_coordinate_rebasis_keeps_delassus_enclosures_overlapping() {
        let (mass_q, jacobian_q) = fixture();
        let mass_point = [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]];
        let jacobian_point = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let s = [[1.0, 2.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]];
        let mass_y = point_multiply(&point_multiply(&point_transpose(&s), &mass_point), &s);
        let jacobian_y = point_multiply(&jacobian_point, &s);
        let q = certify_delassus_coefficients(&mass_q, &jacobian_q, profile()).unwrap();
        let y = certify_delassus_coefficients(
            &point_interval_matrix(&mass_y).unwrap(),
            &point_interval_jacobian(&jacobian_y).unwrap(),
            profile(),
        )
        .unwrap();
        for row in 0..2 {
            for column in 0..2 {
                assert!(intersect(
                    q.delassus_intervals()[row][column],
                    y.delassus_intervals()[row][column]
                ).is_some());
            }
        }
    }

    #[test]
    fn dependent_constraint_rows_are_inconclusive_not_regularized() {
        let mass = point_interval_matrix(&[
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 5.0],
        ])
        .unwrap();
        let j = point_interval_jacobian(&[[1.0, 0.0, 1.0], [2.0, 0.0, 2.0]]).unwrap();
        assert!(matches!(
            certify_delassus_coefficients(&mass, &j, profile()),
            Err(VerifiedDelassusErrorV1::InconclusiveDelassusSpd { .. })
        ));
    }

    #[test]
    fn non_spd_mass_is_inconclusive() {
        let mass = point_interval_matrix(&[[1.0, 2.0], [2.0, 1.0]]).unwrap();
        let j = point_interval_jacobian(&[[1.0, 0.0]]).unwrap();
        assert!(matches!(
            certify_delassus_coefficients(&mass, &j, profile()),
            Err(VerifiedDelassusErrorV1::InconclusiveMassSpd { .. })
        ));
    }

    #[test]
    fn residual_and_multiplier_physical_norm_bounds_are_finite() {
        let (mass, jacobian) = fixture();
        let c = certify_delassus_coefficients(&mass, &jacobian, profile()).unwrap();
        let residual = [
            Interval::new(0.249, 0.251).unwrap(),
            Interval::new(-0.401, -0.399).unwrap(),
        ];
        let multiplier = [
            Interval::new(0.749, 0.751).unwrap(),
            Interval::new(-1.251, -1.249).unwrap(),
        ];
        assert!(c.residual_dual_norm_upper(&residual).unwrap().is_finite());
        assert!(c.multiplier_kinetic_norm_upper(&multiplier).unwrap().is_finite());
    }
}
