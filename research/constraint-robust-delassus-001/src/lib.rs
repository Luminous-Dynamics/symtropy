// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Coordinate-covariant uncertainty envelopes for dense Delassus operators.
//!
//! This is a bounded fixed-size reference. Frobenius norms provide upper
//! estimates for spectral norms, not formally verified interval enclosures.

#![forbid(unsafe_code)]

use core::marker::PhantomData;
use std::fmt;

pub type Vector<const N: usize> = [f64; N];
pub type Matrix<const N: usize> = [[f64; N]; N];
pub type Jacobian<const K: usize, const N: usize> = [[f64; N]; K];
pub type Square<const K: usize> = [[f64; K]; K];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CartesianFrame;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DenseFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RobustConstraintErrorV1 {
    ZeroDimension,
    NonFiniteInput,
    NonFiniteIntermediate,
    InvalidProfile,
    SingularCoordinateChange,
    AsymmetricMassMetric { row: usize, column: usize },
    AsymmetricMassPerturbation { row: usize, column: usize },
    NonPositiveDefiniteMassMetric { pivot: usize },
    NegativeConstraintMetric { pivot: usize },
}

impl fmt::Display for RobustConstraintErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for RobustConstraintErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustConstraintProfileV1 {
    pub relative_symmetry_tolerance: f64,
    pub relative_pivot_tolerance: f64,
    pub relative_roundoff_inflation: f64,
    pub minimum_delassus_eigenvalue: f64,
    pub maximum_admissible_rho: f64,
    pub maximum_informative_relative_bound: f64,
}

impl Default for RobustConstraintProfileV1 {
    fn default() -> Self {
        Self {
            relative_symmetry_tolerance: 128.0 * f64::EPSILON,
            relative_pivot_tolerance: 128.0 * f64::EPSILON,
            relative_roundoff_inflation: 1024.0 * f64::EPSILON,
            minimum_delassus_eigenvalue: 1.0e-10,
            maximum_admissible_rho: 0.9,
            maximum_informative_relative_bound: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustConstraintStatusV1 {
    AdmittedByNumericalEnvelope,
    WeaklyAdmittedByNumericalEnvelope,
    OutsideMassPerturbationEnvelope,
    ConstraintRankInconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RobustDelassusEnvelopeV1<const K: usize> {
    pub rho_mass_perturbation: f64,
    pub delta_jacobian_perturbation: f64,
    pub whitened_jacobian_frobenius: f64,
    pub delassus_frobenius: f64,
    pub nominal_minimum_eigenvalue: f64,
    pub delassus_perturbation_bound: f64,
    pub robust_minimum_eigenvalue_lower_bound: f64,
    pub inverse_delassus_sensitivity_bound: Option<f64>,
    pub status: RobustConstraintStatusV1,
    pub nominal_delassus: Square<K>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tangent<Frame, const N: usize> {
    components: Vector<N>,
    frame: PhantomData<Frame>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Covector<Frame, const N: usize> {
    components: Vector<N>,
    frame: PhantomData<Frame>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Metric<Frame, const N: usize> {
    matrix: Matrix<N>,
    lower: Matrix<N>,
    profile: RobustConstraintProfileV1,
    frame: PhantomData<Frame>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateRebase<From, To, const N: usize> {
    /// q = S y; S maps target tangent components to source components.
    source_from_target: Matrix<N>,
    target_from_source: Matrix<N>,
    condition_inf_estimate: f64,
    frames: PhantomData<(From, To)>,
}

fn validate_profile(profile: RobustConstraintProfileV1) -> Result<(), RobustConstraintErrorV1> {
    if !profile.relative_symmetry_tolerance.is_finite()
        || profile.relative_symmetry_tolerance < 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.relative_roundoff_inflation.is_finite()
        || profile.relative_roundoff_inflation < 0.0
        || !profile.minimum_delassus_eigenvalue.is_finite()
        || profile.minimum_delassus_eigenvalue <= 0.0
        || !profile.maximum_admissible_rho.is_finite()
        || profile.maximum_admissible_rho <= 0.0
        || profile.maximum_admissible_rho >= 1.0
        || !profile.maximum_informative_relative_bound.is_finite()
        || profile.maximum_informative_relative_bound <= 0.0
    {
        return Err(RobustConstraintErrorV1::InvalidProfile);
    }
    Ok(())
}

fn validate_vector<const N: usize>(vector: &Vector<N>) -> Result<(), RobustConstraintErrorV1> {
    if N == 0 {
        return Err(RobustConstraintErrorV1::ZeroDimension);
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(RobustConstraintErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn validate_matrix<const N: usize>(matrix: &Matrix<N>) -> Result<(), RobustConstraintErrorV1> {
    if N == 0 {
        return Err(RobustConstraintErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(RobustConstraintErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn frobenius<const R: usize, const C: usize>(matrix: &[[f64; C]; R]) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |accumulator, value| accumulator.hypot(*value))
}

fn dot<const N: usize>(left: &Vector<N>, right: &Vector<N>) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn transpose<const N: usize>(matrix: &Matrix<N>) -> Matrix<N> {
    std::array::from_fn(|row| std::array::from_fn(|column| matrix[column][row]))
}

fn multiply<const N: usize>(left: &Matrix<N>, right: &Matrix<N>) -> Matrix<N> {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..N)
                .map(|inner| left[row][inner] * right[inner][column])
                .sum()
        })
    })
}

fn apply<const N: usize>(matrix: &Matrix<N>, vector: &Vector<N>) -> Vector<N> {
    std::array::from_fn(|row| {
        (0..N)
            .map(|column| matrix[row][column] * vector[column])
            .sum()
    })
}

fn cholesky<const N: usize>(
    matrix: &Matrix<N>,
    profile: RobustConstraintProfileV1,
) -> Result<Matrix<N>, RobustConstraintErrorV1> {
    validate_matrix(matrix)?;
    validate_profile(profile)?;
    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    if scale == 0.0 {
        return Err(RobustConstraintErrorV1::NonPositiveDefiniteMassMetric { pivot: 0 });
    }
    let symmetry_threshold = profile.relative_symmetry_tolerance * scale;
    let pivot_threshold = profile.relative_pivot_tolerance * scale;
    if !symmetry_threshold.is_finite() || !pivot_threshold.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }

    let mut effective = *matrix;
    for row in 0..N {
        for column in (row + 1)..N {
            if (matrix[row][column] - matrix[column][row]).abs() > symmetry_threshold {
                return Err(RobustConstraintErrorV1::AsymmetricMassMetric { row, column });
            }
            let average = 0.5 * matrix[row][column] + 0.5 * matrix[column][row];
            effective[row][column] = average;
            effective[column][row] = average;
        }
    }

    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let correction = (0..column)
                .map(|inner| lower[row][inner] * lower[column][inner])
                .sum::<f64>();
            let value = effective[row][column] - correction;
            if !value.is_finite() {
                return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
            }
            if row == column {
                if value <= pivot_threshold {
                    return Err(RobustConstraintErrorV1::NonPositiveDefiniteMassMetric { pivot: row });
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    if lower.iter().flatten().any(|value| !value.is_finite()) {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    Ok(lower)
}

fn solve_lower<const N: usize>(
    lower: &Matrix<N>,
    rhs: &Vector<N>,
) -> Result<Vector<N>, RobustConstraintErrorV1> {
    validate_vector(rhs)?;
    let mut solution = *rhs;
    for row in 0..N {
        for column in 0..row {
            solution[row] -= lower[row][column] * solution[column];
        }
        solution[row] /= lower[row][row];
        if !solution[row].is_finite() {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

fn inverse<const N: usize>(
    matrix: &Matrix<N>,
    profile: RobustConstraintProfileV1,
) -> Result<Matrix<N>, RobustConstraintErrorV1> {
    validate_matrix(matrix)?;
    let scale = matrix_inf(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(RobustConstraintErrorV1::SingularCoordinateChange);
    }
    let threshold = profile.relative_pivot_tolerance * scale;
    if !threshold.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    let mut left = *matrix;
    let mut right = std::array::from_fn(|row| {
        std::array::from_fn(|column| if row == column { 1.0 } else { 0.0 })
    });
    for pivot_column in 0..N {
        let pivot_row = (pivot_column..N)
            .max_by(|a, b| left[*a][pivot_column].abs().total_cmp(&left[*b][pivot_column].abs()))
            .ok_or(RobustConstraintErrorV1::ZeroDimension)?;
        let pivot = left[pivot_row][pivot_column].abs();
        if !pivot.is_finite() || pivot <= threshold {
            return Err(RobustConstraintErrorV1::SingularCoordinateChange);
        }
        if pivot_row != pivot_column {
            left.swap(pivot_row, pivot_column);
            right.swap(pivot_row, pivot_column);
        }
        let divisor = left[pivot_column][pivot_column];
        for column in 0..N {
            left[pivot_column][column] /= divisor;
            right[pivot_column][column] /= divisor;
        }
        for row in 0..N {
            if row == pivot_column {
                continue;
            }
            let multiplier = left[row][pivot_column];
            for column in 0..N {
                left[row][column] -= multiplier * left[pivot_column][column];
                right[row][column] -= multiplier * right[pivot_column][column];
            }
        }
    }
    if right.iter().flatten().any(|value| !value.is_finite()) {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    Ok(right)
}

fn symmetric_min_eigenvalue<const K: usize>(
    matrix: &Square<K>,
) -> Result<f64, RobustConstraintErrorV1> {
    if K == 0 {
        return Err(RobustConstraintErrorV1::ZeroDimension);
    }
    let mut a = *matrix;
    if a.iter().flatten().any(|value| !value.is_finite()) {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    let scale = matrix_inf(&a);
    if !scale.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    if K == 1 {
        return Ok(a[0][0]);
    }

    let tolerance = 64.0 * f64::EPSILON * scale.max(1.0);
    let max_iterations = 64 * K * K;
    for _ in 0..max_iterations {
        let mut p = 0usize;
        let mut q = 1usize;
        let mut largest = 0.0_f64;
        for row in 0..K {
            for column in (row + 1)..K {
                let candidate = a[row][column].abs();
                if candidate > largest {
                    largest = candidate;
                    p = row;
                    q = column;
                }
            }
        }
        if largest <= tolerance {
            break;
        }

        let apq = a[p][q];
        let tau = (a[q][q] - a[p][p]) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let app = a[p][p];
        let aqq = a[q][q];
        a[p][p] = app - t * apq;
        a[q][q] = aqq + t * apq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for index in 0..K {
            if index == p || index == q {
                continue;
            }
            let aip = a[index][p];
            let aiq = a[index][q];
            let new_ip = c * aip - s * aiq;
            let new_iq = s * aip + c * aiq;
            a[index][p] = new_ip;
            a[p][index] = new_ip;
            a[index][q] = new_iq;
            a[q][index] = new_iq;
        }
    }

    let minimum = (0..K).map(|index| a[index][index]).fold(f64::INFINITY, f64::min);
    if !minimum.is_finite() {
        Err(RobustConstraintErrorV1::NonFiniteIntermediate)
    } else {
        Ok(minimum)
    }
}

impl<Frame, const N: usize> Tangent<Frame, N> {
    pub fn new(components: Vector<N>) -> Result<Self, RobustConstraintErrorV1> {
        if N == 0 {
            return Err(RobustConstraintErrorV1::ZeroDimension);
        }
        if components.iter().any(|value| !value.is_finite()) {
            return Err(RobustConstraintErrorV1::NonFiniteInput);
        }
        Ok(Self { components, frame: PhantomData })
    }
    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

impl<Frame, const N: usize> Covector<Frame, N> {
    pub fn new(components: Vector<N>) -> Result<Self, RobustConstraintErrorV1> {
        if N == 0 {
            return Err(RobustConstraintErrorV1::ZeroDimension);
        }
        if components.iter().any(|value| !value.is_finite()) {
            return Err(RobustConstraintErrorV1::NonFiniteInput);
        }
        Ok(Self { components, frame: PhantomData })
    }
    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

impl<Frame, const N: usize> Metric<Frame, N> {
    pub fn new(matrix: Matrix<N>, profile: RobustConstraintProfileV1) -> Result<Self, RobustConstraintErrorV1> {
        let lower = cholesky(&matrix, profile)?;
        let mut effective = matrix;
        for row in 0..N {
            for column in (row + 1)..N {
                let average = 0.5 * matrix[row][column] + 0.5 * matrix[column][row];
                effective[row][column] = average;
                effective[column][row] = average;
            }
        }
        Ok(Self { matrix: effective, lower, profile, frame: PhantomData })
    }
    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }
    pub fn lower(&self) -> &Matrix<N> {
        &self.lower
    }
    pub fn whiten_covector(&self, covector: &Covector<Frame, N>) -> Result<Vector<N>, RobustConstraintErrorV1> {
        solve_lower(&self.lower, &covector.components)
    }
    pub fn solve(&self, covector: &Covector<Frame, N>) -> Result<Tangent<Frame, N>, RobustConstraintErrorV1> {
        let intermediate = solve_lower(&self.lower, &covector.components)?;
        let transposed = transpose(&self.lower);
        let mut solution = intermediate;
        for row in (0..N).rev() {
            for column in (row + 1)..N {
                solution[row] -= transposed[row][column] * solution[column];
            }
            solution[row] /= transposed[row][row];
            if !solution[row].is_finite() {
                return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
            }
        }
        Tangent::new(solution)
    }
    pub fn tangent_norm(&self, tangent: &Tangent<Frame, N>) -> Result<f64, RobustConstraintErrorV1> {
        let applied = apply(&self.matrix, &tangent.components);
        let squared = dot(&tangent.components, &applied);
        if !squared.is_finite() || squared < 0.0 {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        Ok(squared.sqrt())
    }
}

impl<From, To, const N: usize> CoordinateRebase<From, To, N> {
    pub fn new(
        source_from_target: Matrix<N>,
        profile: RobustConstraintProfileV1,
    ) -> Result<Self, RobustConstraintErrorV1> {
        validate_profile(profile)?;
        let target_from_source = inverse(&source_from_target, profile)?;
        let condition_inf_estimate = matrix_inf(&source_from_target) * matrix_inf(&target_from_source);
        if !condition_inf_estimate.is_finite() {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        Ok(Self { source_from_target, target_from_source, condition_inf_estimate, frames: PhantomData })
    }
    pub fn condition_inf_estimate(&self) -> f64 {
        self.condition_inf_estimate
    }
    pub fn metric_to_target(&self, metric: &Metric<From, N>) -> Result<Metric<To, N>, RobustConstraintErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&self.source_from_target), &metric.matrix),
            &self.source_from_target,
        );
        Metric::new(transformed, metric.profile)
    }
    pub fn jacobian_to_target<const K: usize>(
        &self,
        jacobian: &Jacobian<K, N>,
    ) -> Result<Jacobian<K, N>, RobustConstraintErrorV1> {
        let transformed = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                (0..N)
                    .map(|inner| jacobian[row][inner] * self.source_from_target[inner][column])
                    .sum()
            })
        });
        if transformed.iter().flatten().any(|value| !value.is_finite()) {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        Ok(transformed)
    }
    pub fn mass_perturbation_to_target(
        &self,
        perturbation: &Matrix<N>,
    ) -> Result<Matrix<N>, RobustConstraintErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&self.source_from_target), perturbation),
            &self.source_from_target,
        );
        if transformed.iter().flatten().any(|value| !value.is_finite()) {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        Ok(transformed)
    }
    pub fn jacobian_perturbation_to_target<const K: usize>(
        &self,
        perturbation: &Jacobian<K, N>,
    ) -> Result<Jacobian<K, N>, RobustConstraintErrorV1> {
        self.jacobian_to_target(perturbation)
    }
}

pub fn build_delassus<const K: usize, Frame, const N: usize>(
    metric: &Metric<Frame, N>,
    jacobian: &Jacobian<K, N>,
) -> Result<(Square<K>, [[f64; N]; K]), RobustConstraintErrorV1> {
    if K == 0 || N == 0 {
        return Err(RobustConstraintErrorV1::ZeroDimension);
    }
    let mut whitened = [[0.0; N]; K];
    for row in 0..K {
        let covector = Covector::<Frame, N>::new(jacobian[row])?;
        whitened[row] = metric.whiten_covector(&covector)?;
    }
    let mut delassus = [[0.0; K]; K];
    for row in 0..K {
        for column in row..K {
            let value = dot(&whitened[row], &whitened[column]);
            if !value.is_finite() {
                return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
            }
            delassus[row][column] = value;
            delassus[column][row] = value;
        }
    }
    Ok((delassus, whitened))
}

pub fn estimate_robust_delassus<const K: usize, Frame, const N: usize>(
    metric: &Metric<Frame, N>,
    jacobian: &Jacobian<K, N>,
    mass_perturbation: &Matrix<N>,
    jacobian_perturbation: &Jacobian<K, N>,
) -> Result<RobustDelassusEnvelopeV1<K>, RobustConstraintErrorV1> {
    let profile = metric.profile;
    let (delassus, whitened_jacobian) = build_delassus(metric, jacobian)?;
    let whitened_mass = metric.whiten_mass_perturbation(mass_perturbation)?;
    let whitened_delta_jacobian = whiten_jacobian(metric, jacobian_perturbation)?;

    let inflation = 1.0 + profile.relative_roundoff_inflation;
    let rho = frobenius(&whitened_mass) * inflation;
    let delta = frobenius(&whitened_delta_jacobian) * inflation;
    let y_norm = frobenius(&whitened_jacobian) * inflation;
    let w_norm = frobenius(&delassus);
    if !rho.is_finite() || !delta.is_finite() || !y_norm.is_finite() || !w_norm.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }

    let nominal_minimum_eigenvalue = symmetric_min_eigenvalue(&delassus)?;
    if rho >= 1.0 || rho > profile.maximum_admissible_rho {
        return Ok(RobustDelassusEnvelopeV1 {
            rho_mass_perturbation: rho,
            delta_jacobian_perturbation: delta,
            whitened_jacobian_frobenius: y_norm,
            delassus_frobenius: w_norm,
            nominal_minimum_eigenvalue,
            delassus_perturbation_bound: f64::INFINITY,
            robust_minimum_eigenvalue_lower_bound: f64::NEG_INFINITY,
            inverse_delassus_sensitivity_bound: None,
            status: RobustConstraintStatusV1::OutsideMassPerturbationEnvelope,
            nominal_delassus: delassus,
        });
    }

    let epsilon = (rho * y_norm * y_norm + 2.0 * delta * y_norm + delta * delta) / (1.0 - rho);
    if !epsilon.is_finite() {
        return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
    }
    let lower = nominal_minimum_eigenvalue - epsilon;
    let inverse_sensitivity = if epsilon < nominal_minimum_eigenvalue && nominal_minimum_eigenvalue > 0.0 {
        Some(epsilon / (nominal_minimum_eigenvalue * (nominal_minimum_eigenvalue - epsilon)))
    } else {
        None
    };
    let status = if lower <= profile.minimum_delassus_eigenvalue {
        RobustConstraintStatusV1::ConstraintRankInconclusive
    } else if epsilon / nominal_minimum_eigenvalue > profile.maximum_informative_relative_bound {
        RobustConstraintStatusV1::WeaklyAdmittedByNumericalEnvelope
    } else {
        RobustConstraintStatusV1::AdmittedByNumericalEnvelope
    };

    Ok(RobustDelassusEnvelopeV1 {
        rho_mass_perturbation: rho,
        delta_jacobian_perturbation: delta,
        whitened_jacobian_frobenius: y_norm,
        delassus_frobenius: w_norm,
        nominal_minimum_eigenvalue,
        delassus_perturbation_bound: epsilon,
        robust_minimum_eigenvalue_lower_bound: lower,
        inverse_delassus_sensitivity_bound: inverse_sensitivity,
        status,
        nominal_delassus: delassus,
    })
}

impl<Frame, const N: usize> Metric<Frame, N> {
    pub fn whiten_mass_perturbation(
        &self,
        perturbation: &Matrix<N>,
    ) -> Result<Matrix<N>, RobustConstraintErrorV1> {
        validate_matrix(perturbation)?;
        let scale = matrix_inf(perturbation);
        let threshold = self.profile.relative_symmetry_tolerance * scale;
        if !scale.is_finite() || !threshold.is_finite() {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        let mut effective = *perturbation;
        for row in 0..N {
            for column in (row + 1)..N {
                if (perturbation[row][column] - perturbation[column][row]).abs() > threshold {
                    return Err(RobustConstraintErrorV1::AsymmetricMassPerturbation { row, column });
                }
                let average = 0.5 * perturbation[row][column] + 0.5 * perturbation[column][row];
                effective[row][column] = average;
                effective[column][row] = average;
            }
        }

        let mut left_whitened = [[0.0; N]; N];
        for column in 0..N {
            let rhs = std::array::from_fn(|row| effective[row][column]);
            let solved = solve_lower(&self.lower, &rhs)?;
            for row in 0..N {
                left_whitened[row][column] = solved[row];
            }
        }
        let mut result = [[0.0; N]; N];
        for row in 0..N {
            let rhs = std::array::from_fn(|column| left_whitened[row][column]);
            let solved = solve_lower(&self.lower, &rhs)?;
            for column in 0..N {
                result[row][column] = solved[column];
            }
        }
        if result.iter().flatten().any(|value| !value.is_finite()) {
            return Err(RobustConstraintErrorV1::NonFiniteIntermediate);
        }
        Ok(result)
    }
}

fn whiten_jacobian<const K: usize, Frame, const N: usize>(
    metric: &Metric<Frame, N>,
    jacobian: &Jacobian<K, N>,
) -> Result<[[f64; N]; K], RobustConstraintErrorV1> {
    if K == 0 {
        return Err(RobustConstraintErrorV1::ZeroDimension);
    }
    let mut result = [[0.0; N]; K];
    for row in 0..K {
        let covector = Covector::<Frame, N>::new(jacobian[row])?;
        result[row] = metric.whiten_covector(&covector)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> RobustConstraintProfileV1 {
        RobustConstraintProfileV1::default()
    }

    fn fixture() -> (
        Metric<CartesianFrame, 3>,
        CoordinateRebase<CartesianFrame, DenseFrame, 3>,
        Jacobian<2, 3>,
        Matrix<3>,
        Jacobian<2, 3>,
    ) {
        let metric = Metric::new(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap();
        let rebase = CoordinateRebase::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 1.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let jacobian = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let mass_perturbation = [[0.01, 0.002, 0.0], [0.002, -0.015, 0.001], [0.0, 0.001, 0.02]];
        let jacobian_perturbation = [[0.001, -0.002, 0.001], [0.0, 0.002, -0.001]];
        (metric, rebase, jacobian, mass_perturbation, jacobian_perturbation)
    }

    #[test]
    fn robust_delassus_envelope_covers_explicit_perturbed_operator() {
        let (metric, _, jacobian, delta_mass, delta_jacobian) = fixture();
        let envelope = estimate_robust_delassus(&metric, &jacobian, &delta_mass, &delta_jacobian).unwrap();
        assert!(matches!(
            envelope.status,
            RobustConstraintStatusV1::AdmittedByNumericalEnvelope
                | RobustConstraintStatusV1::WeaklyAdmittedByNumericalEnvelope
        ));

        let perturbed_mass = std::array::from_fn(|row| {
            std::array::from_fn(|column| metric.matrix()[row][column] + delta_mass[row][column])
        });
        let perturbed_metric = Metric::new(perturbed_mass, profile()).unwrap();
        let perturbed_jacobian = std::array::from_fn(|row| {
            std::array::from_fn(|column| jacobian[row][column] + delta_jacobian[row][column])
        });
        let (perturbed_w, _) = build_delassus(&perturbed_metric, &perturbed_jacobian).unwrap();

        let difference = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                perturbed_w[row][column] - envelope.nominal_delassus[row][column]
            })
        });
        assert!(frobenius(&difference) <= envelope.delassus_perturbation_bound + 1.0e-12);
    }

    #[test]
    fn robust_delassus_envelope_is_invariant_under_dense_rebasing() {
        let (metric_q, rebase, jacobian_q, delta_mass_q, delta_jacobian_q) = fixture();
        let envelope_q =
            estimate_robust_delassus(&metric_q, &jacobian_q, &delta_mass_q, &delta_jacobian_q)
                .unwrap();

        let metric_y = rebase.metric_to_target(&metric_q).unwrap();
        let jacobian_y = rebase.jacobian_to_target(&jacobian_q).unwrap();
        let delta_mass_y = rebase.mass_perturbation_to_target(&delta_mass_q).unwrap();
        let delta_jacobian_y = rebase
            .jacobian_perturbation_to_target(&delta_jacobian_q)
            .unwrap();
        let envelope_y =
            estimate_robust_delassus(&metric_y, &jacobian_y, &delta_mass_y, &delta_jacobian_y)
                .unwrap();

        assert!(
            (envelope_q.rho_mass_perturbation - envelope_y.rho_mass_perturbation).abs()
                < 2.0e-12
        );
        assert!(
            (envelope_q.delta_jacobian_perturbation - envelope_y.delta_jacobian_perturbation)
                .abs()
                < 2.0e-12
        );
        assert!(
            (envelope_q.delassus_perturbation_bound - envelope_y.delassus_perturbation_bound).abs()
                < 2.0e-11
        );
        assert!(
            (envelope_q.robust_minimum_eigenvalue_lower_bound
                - envelope_y.robust_minimum_eigenvalue_lower_bound)
                .abs()
                < 2.0e-11
        );
    }

    #[test]
    fn near_dependent_constraints_abstain_when_rank_envelope_crosses_floor() {
        let metric = Metric::<CartesianFrame, 3>::new(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let jacobian = [[1.0, 0.0, 0.0], [1.0, 1.0e-4, 0.0]];
        let delta_jacobian = [[0.0, 0.0, 0.0], [0.0, -1.0e-4, 0.0]];
        let envelope = estimate_robust_delassus(
            &metric,
            &jacobian,
            &[[0.0; 3]; 3],
            &delta_jacobian,
        )
        .unwrap();
        assert_eq!(envelope.status, RobustConstraintStatusV1::ConstraintRankInconclusive);
        assert!(envelope.robust_minimum_eigenvalue_lower_bound <= profile().minimum_delassus_eigenvalue);
    }

    #[test]
    fn rho_at_or_above_one_abstains() {
        let metric = Metric::<CartesianFrame, 3>::new(
            [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
            profile(),
        )
        .unwrap();
        let jacobian = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let envelope = estimate_robust_delassus(
            &metric,
            &jacobian,
            &[[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0, 0.0, 3.0]],
            &[[0.0; 3]; 2],
        )
        .unwrap();
        assert_eq!(
            envelope.status,
            RobustConstraintStatusV1::OutsideMassPerturbationEnvelope
        );
        assert!(envelope.inverse_delassus_sensitivity_bound.is_none());
    }

    #[test]
    fn asymmetric_mass_perturbation_is_rejected() {
        let metric = Metric::<CartesianFrame, 2>::new([[1.0, 0.0], [0.0, 1.0]], profile()).unwrap();
        let jacobian = [[1.0, 0.0]];
        assert!(matches!(
            estimate_robust_delassus(
                &metric,
                &jacobian,
                &[[0.0, 0.1], [0.0, 0.0]],
                &[[0.0, 0.0]]
            ),
            Err(RobustConstraintErrorV1::AsymmetricMassPerturbation { .. })
        ));
    }

    #[test]
    fn wrong_jacobian_rebasis_law_changes_envelope() {
        let (metric_q, rebase, jacobian_q, delta_mass_q, delta_jacobian_q) = fixture();
        let nominal = estimate_robust_delassus(&metric_q, &jacobian_q, &delta_mass_q, &delta_jacobian_q).unwrap();

        let metric_y = rebase.metric_to_target(&metric_q).unwrap();
        let wrong_jacobian = jacobian_q; // Deliberately omitted J_y = J_q S.
        let wrong_delta_jacobian = delta_jacobian_q;
        let wrong = estimate_robust_delassus(
            &metric_y,
            &wrong_jacobian,
            &rebase.mass_perturbation_to_target(&delta_mass_q).unwrap(),
            &wrong_delta_jacobian,
        ).unwrap();
        assert!(
            (nominal.delassus_perturbation_bound - wrong.delassus_perturbation_bound).abs() > 1.0e-3
        );
    }
}
