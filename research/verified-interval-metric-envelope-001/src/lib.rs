// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Outward-rounded interval arithmetic for a small SPD metric uncertainty bound.
//!
//! The claim is conditional on IEEE-754 binary64, round-to-nearest elementary
//! operations, no fast-math/reassociation, and this crate's pinned build target.

#![forbid(unsafe_code)]

use std::fmt;

pub type Vector<const N: usize> = [f64; N];
pub type Matrix<const N: usize> = [[f64; N]; N];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    lower: f64,
    upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntervalErrorV1 {
    NonFiniteEndpoint,
    ReversedEndpoints,
    NonFiniteOperation,
    DivisionByZeroInterval,
    NegativeSquareRoot,
}

impl fmt::Display for IntervalErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for IntervalErrorV1 {}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalErrorV1> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteEndpoint);
        }
        if lower > upper {
            return Err(IntervalErrorV1::ReversedEndpoints);
        }
        Ok(Self { lower, upper })
    }

    pub fn point(value: f64) -> Result<Self, IntervalErrorV1> {
        Self::new(value, value)
    }

    pub const fn zero() -> Self {
        Self { lower: 0.0, upper: 0.0 }
    }

    pub fn lower(self) -> f64 {
        self.lower
    }

    pub fn upper(self) -> f64 {
        self.upper
    }

    pub fn max_abs(self) -> f64 {
        self.lower.abs().max(self.upper.abs())
    }

    pub fn contains(self, value: f64) -> bool {
        self.lower <= value && value <= self.upper
    }

    pub fn add(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        Self::new(
            checked_finite((self.lower + rhs.lower).next_down())?,
            checked_finite((self.upper + rhs.upper).next_up())?,
        )
    }

    pub fn sub(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        Self::new(
            checked_finite((self.lower - rhs.upper).next_down())?,
            checked_finite((self.upper - rhs.lower).next_up())?,
        )
    }

    pub fn mul(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(IntervalErrorV1::NonFiniteOperation);
        }
        let lower = products
            .iter()
            .map(|value| value.next_down())
            .fold(f64::INFINITY, f64::min);
        let upper = products
            .iter()
            .map(|value| value.next_up())
            .fold(f64::NEG_INFINITY, f64::max);
        Self::new(lower, upper)
    }

    pub fn div(self, rhs: Self) -> Result<Self, IntervalErrorV1> {
        if rhs.lower <= 0.0 && rhs.upper >= 0.0 {
            return Err(IntervalErrorV1::DivisionByZeroInterval);
        }
        let quotients = [
            self.lower / rhs.lower,
            self.lower / rhs.upper,
            self.upper / rhs.lower,
            self.upper / rhs.upper,
        ];
        if quotients.iter().any(|value| !value.is_finite()) {
            return Err(IntervalErrorV1::NonFiniteOperation);
        }
        let lower = quotients
            .iter()
            .map(|value| value.next_down())
            .fold(f64::INFINITY, f64::min);
        let upper = quotients
            .iter()
            .map(|value| value.next_up())
            .fold(f64::NEG_INFINITY, f64::max);
        Self::new(lower, upper)
    }

    pub fn sqrt(self) -> Result<Self, IntervalErrorV1> {
        if self.lower < 0.0 {
            return Err(IntervalErrorV1::NegativeSquareRoot);
        }
        let lower = if self.lower == 0.0 {
            0.0
        } else {
            checked_finite(self.lower.sqrt().next_down())?
        };
        let upper = checked_finite(self.upper.sqrt().next_up())?;
        Self::new(lower, upper)
    }
}

fn checked_finite(value: f64) -> Result<f64, IntervalErrorV1> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(IntervalErrorV1::NonFiniteOperation)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedEnvelopeProfileV1 {
    pub maximum_rho: f64,
}

impl Default for VerifiedEnvelopeProfileV1 {
    fn default() -> Self {
        Self { maximum_rho: 0.9 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerifiedEnvelopeErrorV1 {
    InvalidProfile,
    ZeroDimension,
    NonFiniteInput,
    NonSymmetricMass,
    NonSymmetricMassPerturbation,
    InconclusiveSpd { pivot: usize, lower_bound: f64 },
    Interval(IntervalErrorV1),
}

impl fmt::Display for VerifiedEnvelopeErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for VerifiedEnvelopeErrorV1 {}

impl From<IntervalErrorV1> for VerifiedEnvelopeErrorV1 {
    fn from(value: IntervalErrorV1) -> Self {
        Self::Interval(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifiedEnvelopeStatusV1 {
    VerifiedEnclosure,
    OutsidePerturbationEnvelope,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerifiedMetricEnvelopeV1 {
    pub rho_upper: f64,
    pub force_dual_norm_upper: f64,
    pub nominal_acceleration_mass_norm_upper: f64,
    pub acceleration_error_bound_upper: Option<f64>,
    pub minimum_cholesky_pivot_lower: f64,
    pub status: VerifiedEnvelopeStatusV1,
}

fn validate_profile(profile: VerifiedEnvelopeProfileV1) -> Result<(), VerifiedEnvelopeErrorV1> {
    if !profile.maximum_rho.is_finite()
        || profile.maximum_rho <= 0.0
        || profile.maximum_rho >= 1.0
    {
        return Err(VerifiedEnvelopeErrorV1::InvalidProfile);
    }
    Ok(())
}

fn validate_vector<const N: usize>(vector: &Vector<N>) -> Result<(), VerifiedEnvelopeErrorV1> {
    if N == 0 {
        return Err(VerifiedEnvelopeErrorV1::ZeroDimension);
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(VerifiedEnvelopeErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn validate_matrix<const N: usize>(matrix: &Matrix<N>) -> Result<(), VerifiedEnvelopeErrorV1> {
    if N == 0 {
        return Err(VerifiedEnvelopeErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(VerifiedEnvelopeErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn validate_symmetric_matrix<const N: usize>(
    matrix: &Matrix<N>,
) -> Result<(), VerifiedEnvelopeErrorV1> {
    validate_matrix(matrix)?;
    for row in 0..N {
        for column in 0..N {
            if matrix[row][column] != matrix[column][row] {
                return Err(VerifiedEnvelopeErrorV1::NonSymmetricMass);
            }
        }
    }
    Ok(())
}

fn interval_zero_matrix<const N: usize>() -> [[Interval; N]; N] {
    [[Interval::zero(); N]; N]
}

fn interval_dot<const N: usize>(
    left: &[Interval; N],
    right: &[Interval; N],
) -> Result<Interval, IntervalErrorV1> {
    let mut sum = Interval::zero();
    for index in 0..N {
        sum = sum.add(left[index].mul(right[index])?)?;
    }
    Ok(sum)
}

fn cholesky_interval<const N: usize>(
    matrix: &Matrix<N>,
) -> Result<([[Interval; N]; N], f64), VerifiedEnvelopeErrorV1> {
    validate_symmetric_matrix(matrix)?;
    let mut lower = interval_zero_matrix::<N>();
    let mut minimum_pivot = f64::INFINITY;
    for row in 0..N {
        for column in 0..=row {
            let left: [Interval; N] = std::array::from_fn(|index| {
                if index < column {
                    lower[row][index]
                } else {
                    Interval::zero()
                }
            });
            let right: [Interval; N] = std::array::from_fn(|index| {
                if index < column {
                    lower[column][index]
                } else {
                    Interval::zero()
                }
            });
            let correction = interval_dot(&left, &right)?;
            let pivot = Interval::point(matrix[row][column])?.sub(correction)?;
            if row == column {
                if pivot.lower() <= 0.0 {
                    return Err(VerifiedEnvelopeErrorV1::InconclusiveSpd {
                        pivot: row,
                        lower_bound: pivot.lower(),
                    });
                }
                minimum_pivot = minimum_pivot.min(pivot.lower());
                lower[row][column] = pivot.sqrt()?;
            } else {
                let value = pivot.div(lower[column][column])?;
                lower[row][column] = value;
            }
        }
    }
    Ok((lower, minimum_pivot))
}

fn solve_lower_interval<const N: usize>(
    lower: &[[Interval; N]; N],
    rhs: &[Interval; N],
) -> Result<[Interval; N], IntervalErrorV1> {
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

fn whiten_mass_perturbation<const N: usize>(
    lower: &[[Interval; N]; N],
    perturbation: &Matrix<N>,
) -> Result<[[Interval; N]; N], VerifiedEnvelopeErrorV1> {
    validate_matrix(perturbation)?;
    for row in 0..N {
        for column in 0..N {
            if perturbation[row][column] != perturbation[column][row] {
                return Err(VerifiedEnvelopeErrorV1::NonSymmetricMassPerturbation);
            }
        }
    }
    let mut left_whitened = interval_zero_matrix::<N>();
    for column in 0..N {
        let mut rhs = [Interval::zero(); N];
        for row in 0..N {
            rhs[row] = Interval::point(perturbation[row][column])?;
        }
        let solved = solve_lower_interval(lower, &rhs)?;
        for row in 0..N {
            left_whitened[row][column] = solved[row];
        }
    }
    let mut result = interval_zero_matrix::<N>();
    for row in 0..N {
        let rhs: [Interval; N] =
            std::array::from_fn(|column| left_whitened[row][column]);
        let solved = solve_lower_interval(lower, &rhs)?;
        for column in 0..N {
            result[row][column] = solved[column];
        }
    }
    Ok(result)
}

fn upper_norm<const N: usize>(vector: &[Interval; N]) -> Result<f64, IntervalErrorV1> {
    let mut squared_upper = 0.0;
    for value in vector {
        let magnitude = value.max_abs();
        let square_upper = Interval::point(magnitude)?.mul(Interval::point(magnitude)?)?.upper();
        squared_upper = Interval::point(squared_upper)?
            .add(Interval::point(square_upper)?)?
            .upper();
    }
    if squared_upper == 0.0 {
        Ok(0.0)
    } else {
        checked_finite(squared_upper.sqrt().next_up())
    }
}

fn upper_frobenius<const N: usize>(
    matrix: &[[Interval; N]; N],
) -> Result<f64, IntervalErrorV1> {
    let mut squared_upper = 0.0;
    for value in matrix.iter().flatten() {
        let magnitude = value.max_abs();
        let square_upper = Interval::point(magnitude)?.mul(Interval::point(magnitude)?)?.upper();
        squared_upper = Interval::point(squared_upper)?
            .add(Interval::point(square_upper)?)?
            .upper();
    }
    if squared_upper == 0.0 {
        Ok(0.0)
    } else {
        checked_finite(squared_upper.sqrt().next_up())
    }
}

fn upper_add(left: f64, right: f64) -> Result<f64, IntervalErrorV1> {
    Ok(Interval::point(left)?.add(Interval::point(right)?)?.upper())
}
fn upper_mul(left: f64, right: f64) -> Result<f64, IntervalErrorV1> {
    Ok(Interval::point(left)?.mul(Interval::point(right)?)?.upper())
}
fn lower_sub(left: f64, right: f64) -> Result<f64, IntervalErrorV1> {
    Ok(Interval::point(left)?.sub(Interval::point(right)?)?.lower())
}
fn upper_div(left: f64, right: f64) -> Result<f64, IntervalErrorV1> {
    Ok(Interval::point(left)?.div(Interval::point(right)?)?.upper())
}

pub fn certify_metric_uncertainty<const N: usize>(
    mass: &Matrix<N>,
    mass_perturbation: &Matrix<N>,
    force_perturbation: &Vector<N>,
    nominal_acceleration: &Vector<N>,
    profile: VerifiedEnvelopeProfileV1,
) -> Result<VerifiedMetricEnvelopeV1, VerifiedEnvelopeErrorV1> {
    validate_profile(profile)?;
    validate_matrix(mass)?;
    validate_matrix(mass_perturbation)?;
    validate_vector(force_perturbation)?;
    validate_vector(nominal_acceleration)?;

    let (lower, minimum_cholesky_pivot_lower) = cholesky_interval(mass)?;
    let whitened_mass = whiten_mass_perturbation(&lower, mass_perturbation)?;

    let rho_upper = upper_frobenius(&whitened_mass)?;
    let mut force_rhs = [Interval::zero(); N];
    for index in 0..N {
        force_rhs[index] = Interval::point(force_perturbation[index])?;
    }
    let whitened_force = solve_lower_interval(&lower, &force_rhs)?;
    let force_dual_norm_upper = upper_norm(&whitened_force)?;

    // ||a0||_M = ||L^T a0||_2.
    let mut whitened_acceleration = [Interval::zero(); N];
    for column in 0..N {
        let mut sum = Interval::zero();
        for row in 0..N {
            let factor = lower[row][column];
            let acceleration = Interval::point(nominal_acceleration[row])?;
            sum = sum.add(factor.mul(acceleration)?)?;
        }
        whitened_acceleration[column] = sum;
    }
    let nominal_acceleration_mass_norm_upper = upper_norm(&whitened_acceleration)?;

    if rho_upper >= 1.0 || rho_upper > profile.maximum_rho {
        return Ok(VerifiedMetricEnvelopeV1 {
            rho_upper,
            force_dual_norm_upper,
            nominal_acceleration_mass_norm_upper,
            acceleration_error_bound_upper: None,
            minimum_cholesky_pivot_lower,
            status: VerifiedEnvelopeStatusV1::OutsidePerturbationEnvelope,
        });
    }

    let numerator = upper_add(
        force_dual_norm_upper,
        upper_mul(rho_upper, nominal_acceleration_mass_norm_upper)?,
    )?;
    let denominator_lower = lower_sub(1.0, rho_upper)?;
    if denominator_lower <= 0.0 {
        return Ok(VerifiedMetricEnvelopeV1 {
            rho_upper,
            force_dual_norm_upper,
            nominal_acceleration_mass_norm_upper,
            acceleration_error_bound_upper: None,
            minimum_cholesky_pivot_lower,
            status: VerifiedEnvelopeStatusV1::OutsidePerturbationEnvelope,
        });
    }
    let acceleration_error_bound_upper = upper_div(numerator, denominator_lower)?;

    Ok(VerifiedMetricEnvelopeV1 {
        rho_upper,
        force_dual_norm_upper,
        nominal_acceleration_mass_norm_upper,
        acceleration_error_bound_upper: Some(acceleration_error_bound_upper),
        minimum_cholesky_pivot_lower,
        status: VerifiedEnvelopeStatusV1::VerifiedEnclosure,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_add_and_multiply_enclose_exact_binary_operations() {
        let a = Interval::point(0.1).unwrap();
        let b = Interval::point(0.2).unwrap();
        let sum = a.add(b).unwrap();
        let product = a.mul(b).unwrap();
        let expected_sum = 0.1_f64 as f64 + 0.2_f64 as f64;
        let expected_product = 0.1_f64 as f64 * 0.2_f64 as f64;
        assert!(sum.contains(expected_sum));
        assert!(product.contains(expected_product));
        assert!(sum.lower() <= sum.upper());
        assert!(product.lower() <= product.upper());
    }

    #[test]
    fn division_by_zero_interval_is_rejected() {
        let numerator = Interval::point(1.0).unwrap();
        let denominator = Interval::new(-0.1, 0.1).unwrap();
        assert_eq!(
            numerator.div(denominator),
            Err(IntervalErrorV1::DivisionByZeroInterval)
        );
    }

    #[test]
    fn interval_cholesky_and_metric_bound_cover_diagonal_perturbed_solution() {
        let mass = [[2.0, 0.0], [0.0, 3.0]];
        let delta_mass = [[0.02, 0.0], [0.0, -0.03]];
        let delta_force = [0.01, -0.02];
        let nominal_acceleration = [0.5, -0.2];
        let envelope = certify_metric_uncertainty(
            &mass,
            &delta_mass,
            &delta_force,
            &nominal_acceleration,
            VerifiedEnvelopeProfileV1::default(),
        )
        .unwrap();

        assert_eq!(envelope.status, VerifiedEnvelopeStatusV1::VerifiedEnclosure);
        let bound = envelope.acceleration_error_bound_upper.unwrap();
        assert!(envelope.minimum_cholesky_pivot_lower > 0.0);

        let true_acceleration = [
            (mass[0][0] * nominal_acceleration[0] + delta_force[0])
                / (mass[0][0] + delta_mass[0][0]),
            (mass[1][1] * nominal_acceleration[1] + delta_force[1])
                / (mass[1][1] + delta_mass[1][1]),
        ];
        let error = [
            true_acceleration[0] - nominal_acceleration[0],
            true_acceleration[1] - nominal_acceleration[1],
        ];
        let observed_mass_error =
            (mass[0][0] * error[0] * error[0] + mass[1][1] * error[1] * error[1]).sqrt();
        assert!(observed_mass_error <= bound);
    }

    fn transpose2(matrix: &Matrix<2>) -> Matrix<2> {
        [[matrix[0][0], matrix[1][0]], [matrix[0][1], matrix[1][1]]]
    }

    fn multiply2(left: &Matrix<2>, right: &Matrix<2>) -> Matrix<2> {
        [
            [
                left[0][0] * right[0][0] + left[0][1] * right[1][0],
                left[0][0] * right[0][1] + left[0][1] * right[1][1],
            ],
            [
                left[1][0] * right[0][0] + left[1][1] * right[1][0],
                left[1][0] * right[0][1] + left[1][1] * right[1][1],
            ],
        ]
    }

    fn apply2(matrix: &Matrix<2>, vector: &Vector<2>) -> Vector<2> {
        [
            matrix[0][0] * vector[0] + matrix[0][1] * vector[1],
            matrix[1][0] * vector[0] + matrix[1][1] * vector[1],
        ]
    }

    fn solve2(matrix: &Matrix<2>, rhs: &Vector<2>) -> Vector<2> {
        let determinant = matrix[0][0] * matrix[1][1] - matrix[0][1] * matrix[1][0];
        [
            (rhs[0] * matrix[1][1] - matrix[0][1] * rhs[1]) / determinant,
            (matrix[0][0] * rhs[1] - rhs[0] * matrix[1][0]) / determinant,
        ]
    }

    fn mass_norm2(metric: &Matrix<2>, vector: &Vector<2>) -> f64 {
        let applied = apply2(metric, vector);
        (vector[0] * applied[0] + vector[1] * applied[1]).sqrt()
    }

    #[test]
    fn dense_coordinate_rebasis_produces_a_valid_independent_enclosure() {
        let profile = VerifiedEnvelopeProfileV1::default();
        let mass_q = [[2.0, 0.0], [0.0, 3.0]];
        let delta_mass_q = [[0.02, 0.0], [0.0, -0.03]];
        let delta_force_q = [0.01, -0.02];
        let acceleration_q = [0.5, -0.2];

        let q_certificate = certify_metric_uncertainty(
            &mass_q,
            &delta_mass_q,
            &delta_force_q,
            &acceleration_q,
            profile,
        )
        .unwrap();
        let q_force = apply2(&mass_q, &acceleration_q);
        let q_true_force = [q_force[0] + delta_force_q[0], q_force[1] + delta_force_q[1]];
        let q_true_mass = [
            [mass_q[0][0] + delta_mass_q[0][0], 0.0],
            [0.0, mass_q[1][1] + delta_mass_q[1][1]],
        ];
        let q_true_acceleration = solve2(&q_true_mass, &q_true_force);
        let q_error = [
            q_true_acceleration[0] - acceleration_q[0],
            q_true_acceleration[1] - acceleration_q[1],
        ];
        assert!(
            mass_norm2(&mass_q, &q_error)
                <= q_certificate.acceleration_error_bound_upper.unwrap()
        );

        // q = S y, with a dense non-orthogonal, determinant-one basis map.
        let s = [[1.0, 2.0], [0.0, 1.0]];
        let st = transpose2(&s);
        let s_inverse = [[1.0, -2.0], [0.0, 1.0]];
        let mass_y = multiply2(&multiply2(&st, &mass_q), &s);
        let delta_mass_y = multiply2(&multiply2(&st, &delta_mass_q), &s);
        let acceleration_y = apply2(&s_inverse, &acceleration_q);
        let delta_force_y = apply2(&st, &delta_force_q);
        let y_certificate = certify_metric_uncertainty(
            &mass_y,
            &delta_mass_y,
            &delta_force_y,
            &acceleration_y,
            profile,
        )
        .unwrap();

        let y_force = apply2(&mass_y, &acceleration_y);
        let y_true_force = [y_force[0] + delta_force_y[0], y_force[1] + delta_force_y[1]];
        let y_true_mass = [
            [mass_y[0][0] + delta_mass_y[0][0], mass_y[0][1] + delta_mass_y[0][1]],
            [mass_y[1][0] + delta_mass_y[1][0], mass_y[1][1] + delta_mass_y[1][1]],
        ];
        let y_true_acceleration = solve2(&y_true_mass, &y_true_force);
        let y_error = [
            y_true_acceleration[0] - acceleration_y[0],
            y_true_acceleration[1] - acceleration_y[1],
        ];
        assert!(
            mass_norm2(&mass_y, &y_error)
                <= y_certificate.acceleration_error_bound_upper.unwrap()
        );
        assert_eq!(q_certificate.status, VerifiedEnvelopeStatusV1::VerifiedEnclosure);
        assert_eq!(y_certificate.status, VerifiedEnvelopeStatusV1::VerifiedEnclosure);
    }

    #[test]
    fn interval_cholesky_abstains_when_spd_cannot_be_proved() {
        let singular = [[1.0, 2.0], [2.0, 4.0]];
        assert!(matches!(
            certify_metric_uncertainty(
                &singular,
                &[[0.0, 0.0], [0.0, 0.0]],
                &[0.0, 0.0],
                &[1.0, 1.0],
                VerifiedEnvelopeProfileV1::default()
            ),
            Err(VerifiedEnvelopeErrorV1::InconclusiveSpd { .. })
        ));
    }

    #[test]
    fn rho_upper_at_or_above_one_emits_no_finite_bound() {
        let mass = [[1.0, 0.0], [0.0, 1.0]];
        let delta_mass = [[2.0, 0.0], [0.0, 2.0]];
        let envelope = certify_metric_uncertainty(
            &mass,
            &delta_mass,
            &[0.0, 0.0],
            &[0.5, -0.5],
            VerifiedEnvelopeProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            envelope.status,
            VerifiedEnvelopeStatusV1::OutsidePerturbationEnvelope
        );
        assert!(envelope.acceleration_error_bound_upper.is_none());
    }

    #[test]
    fn asymmetric_mass_and_perturbation_are_rejected() {
        assert_eq!(
            certify_metric_uncertainty(
                &[[1.0, 0.2], [0.0, 1.0]],
                &[[0.0, 0.0], [0.0, 0.0]],
                &[0.0, 0.0],
                &[1.0, 1.0],
                VerifiedEnvelopeProfileV1::default()
            ),
            Err(VerifiedEnvelopeErrorV1::NonSymmetricMass)
        );
        assert_eq!(
            certify_metric_uncertainty(
                &[[1.0, 0.0], [0.0, 1.0]],
                &[[0.0, 0.1], [0.0, 0.0]],
                &[0.0, 0.0],
                &[1.0, 1.0],
                VerifiedEnvelopeProfileV1::default()
            ),
            Err(VerifiedEnvelopeErrorV1::NonSymmetricMass)
        );
    }

    #[test]
    fn nonfinite_and_overflow_operations_fail_closed() {
        let a = Interval::point(f64::MAX).unwrap();
        let b = Interval::point(f64::MAX).unwrap();
        assert_eq!(a.mul(b), Err(IntervalErrorV1::NonFiniteOperation));
        assert_eq!(
            certify_metric_uncertainty(
                &[[1.0, 0.0], [0.0, 1.0]],
                &[[0.0, 0.0], [0.0, 0.0]],
                &[f64::MAX, f64::MAX],
                &[1.0, 1.0],
                VerifiedEnvelopeProfileV1::default()
            ),
            Err(VerifiedEnvelopeErrorV1::Interval(
                IntervalErrorV1::NonFiniteOperation
            ))
        );
    }
}
