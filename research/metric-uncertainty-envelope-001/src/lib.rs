// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Coordinate-covariant perturbation envelopes for SPD mass solves.
//!
//! The uncertainty bound is derived after Cholesky whitening. The Frobenius
//! norm is an upper bound for the spectral norm in exact arithmetic; the
//! computed value is still a numerical estimate, not a verified interval.

#![forbid(unsafe_code)]

use core::marker::PhantomData;
use std::fmt;

pub type Vector<const N: usize> = [f64; N];
pub type Matrix<const N: usize> = [[f64; N]; N];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CartesianFrame;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DenseFrame;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UncertaintyErrorV1 {
    ZeroDimension,
    NonFiniteInput,
    NonFiniteIntermediate,
    InvalidProfile,
    SingularCoordinateChange,
    AsymmetricMassMetric { row: usize, column: usize },
    AsymmetricMassPerturbation { row: usize, column: usize },
    NonPositiveDefiniteMassMetric { pivot: usize },
    NormNotPositive,
}

impl fmt::Display for UncertaintyErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for UncertaintyErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UncertaintyProfileV1 {
    pub relative_symmetry_tolerance: f64,
    pub relative_pivot_tolerance: f64,
    pub relative_frobenius_roundoff_inflation: f64,
    pub maximum_informative_rho: f64,
    pub maximum_admissible_rho: f64,
}

impl Default for UncertaintyProfileV1 {
    fn default() -> Self {
        Self {
            relative_symmetry_tolerance: 128.0 * f64::EPSILON,
            relative_pivot_tolerance: 128.0 * f64::EPSILON,
            relative_frobenius_roundoff_inflation: 512.0 * f64::EPSILON,
            maximum_informative_rho: 0.1,
            maximum_admissible_rho: 0.9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvelopeStatusV1 {
    InformativeEstimate,
    WeakEstimate,
    OutsidePerturbationEnvelope,
    VacuousEstimate,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerturbationEnvelopeV1 {
    /// Frobenius upper estimate for ||L^-1 ΔM L^-T||_2.
    pub relative_mass_perturbation_rho: f64,
    /// ||ΔQ||_(M^-1), evaluated by Cholesky whitening.
    pub force_uncertainty_dual_norm: f64,
    /// ||a0||_M for the nominal solution.
    pub nominal_acceleration_mass_norm: f64,
    /// (||ΔQ||_(M^-1) + rho ||a0||_M) / (1-rho), if rho < 1.
    pub acceleration_error_bound_mass_norm: Option<f64>,
    pub status: EnvelopeStatusV1,
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
    profile: UncertaintyProfileV1,
    frame: PhantomData<Frame>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoordinateRebase<From, To, const N: usize> {
    /// q = S y.
    source_from_target: Matrix<N>,
    target_from_source: Matrix<N>,
    condition_inf_estimate: f64,
    frames: PhantomData<(From, To)>,
}

fn validate_profile(profile: UncertaintyProfileV1) -> Result<(), UncertaintyErrorV1> {
    if !profile.relative_symmetry_tolerance.is_finite()
        || profile.relative_symmetry_tolerance < 0.0
        || !profile.relative_pivot_tolerance.is_finite()
        || profile.relative_pivot_tolerance <= 0.0
        || !profile.relative_frobenius_roundoff_inflation.is_finite()
        || profile.relative_frobenius_roundoff_inflation < 0.0
        || !profile.maximum_informative_rho.is_finite()
        || profile.maximum_informative_rho <= 0.0
        || !profile.maximum_admissible_rho.is_finite()
        || profile.maximum_admissible_rho <= profile.maximum_informative_rho
        || profile.maximum_admissible_rho >= 1.0
    {
        return Err(UncertaintyErrorV1::InvalidProfile);
    }
    Ok(())
}

fn validate_vector<const N: usize>(vector: &Vector<N>) -> Result<(), UncertaintyErrorV1> {
    if N == 0 {
        return Err(UncertaintyErrorV1::ZeroDimension);
    }
    if vector.iter().any(|value| !value.is_finite()) {
        return Err(UncertaintyErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn validate_matrix<const N: usize>(matrix: &Matrix<N>) -> Result<(), UncertaintyErrorV1> {
    if N == 0 {
        return Err(UncertaintyErrorV1::ZeroDimension);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(UncertaintyErrorV1::NonFiniteInput);
    }
    Ok(())
}

fn dot<const N: usize>(left: &Vector<N>, right: &Vector<N>) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn norm_euclidean<const N: usize>(vector: &Vector<N>) -> Result<f64, UncertaintyErrorV1> {
    let norm = vector.iter().fold(0.0_f64, |acc, value| acc.hypot(*value));
    if norm.is_finite() {
        Ok(norm)
    } else {
        Err(UncertaintyErrorV1::NonFiniteIntermediate)
    }
}

fn matrix_inf<const N: usize>(matrix: &Matrix<N>) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
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
    profile: UncertaintyProfileV1,
) -> Result<Matrix<N>, UncertaintyErrorV1> {
    validate_matrix(matrix)?;
    validate_profile(profile)?;
    let scale = matrix_inf(matrix);
    if !scale.is_finite() {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    if scale == 0.0 {
        return Err(UncertaintyErrorV1::NonPositiveDefiniteMassMetric { pivot: 0 });
    }
    let symmetry_threshold = profile.relative_symmetry_tolerance * scale;
    if !symmetry_threshold.is_finite() {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    for row in 0..N {
        for column in (row + 1)..N {
            if (matrix[row][column] - matrix[column][row]).abs() > symmetry_threshold {
                return Err(UncertaintyErrorV1::AsymmetricMassMetric { row, column });
            }
        }
    }

    let pivot_threshold = profile.relative_pivot_tolerance * scale;
    if !pivot_threshold.is_finite() {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    let mut lower = [[0.0; N]; N];
    for row in 0..N {
        for column in 0..=row {
            let correction = (0..column)
                .map(|inner| lower[row][inner] * lower[column][inner])
                .sum::<f64>();
            let value = matrix[row][column] - correction;
            if !value.is_finite() {
                return Err(UncertaintyErrorV1::NonFiniteIntermediate);
            }
            if row == column {
                if value <= pivot_threshold {
                    return Err(UncertaintyErrorV1::NonPositiveDefiniteMassMetric { pivot: row });
                }
                lower[row][column] = value.sqrt();
            } else {
                lower[row][column] = value / lower[column][column];
            }
        }
    }
    if lower.iter().flatten().any(|value| !value.is_finite()) {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    Ok(lower)
}

/// Solve L x = rhs for lower-triangular L.
fn solve_lower<const N: usize>(
    lower: &Matrix<N>,
    rhs: &Vector<N>,
) -> Result<Vector<N>, UncertaintyErrorV1> {
    validate_vector(rhs)?;
    let mut solution = *rhs;
    for row in 0..N {
        for column in 0..row {
            solution[row] -= lower[row][column] * solution[column];
        }
        solution[row] /= lower[row][row];
        if !solution[row].is_finite() {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

/// Solve M x = rhs using M = L L^T.
fn solve_spd<const N: usize>(
    lower: &Matrix<N>,
    rhs: &Vector<N>,
) -> Result<Vector<N>, UncertaintyErrorV1> {
    let intermediate = solve_lower(lower, rhs)?;
    let transposed = transpose(lower);
    let mut solution = intermediate;
    for row in (0..N).rev() {
        for column in (row + 1)..N {
            solution[row] -= transposed[row][column] * solution[column];
        }
        solution[row] /= transposed[row][row];
        if !solution[row].is_finite() {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
}

impl<Frame, const N: usize> Tangent<Frame, N> {
    pub fn new(components: Vector<N>) -> Result<Self, UncertaintyErrorV1> {
        validate_vector(&components)?;
        Ok(Self { components, frame: PhantomData })
    }
    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

impl<Frame, const N: usize> Covector<Frame, N> {
    pub fn new(components: Vector<N>) -> Result<Self, UncertaintyErrorV1> {
        validate_vector(&components)?;
        Ok(Self { components, frame: PhantomData })
    }
    pub fn components(&self) -> &Vector<N> {
        &self.components
    }
}

impl<Frame, const N: usize> Metric<Frame, N> {
    pub fn new(
        matrix: Matrix<N>,
        profile: UncertaintyProfileV1,
    ) -> Result<Self, UncertaintyErrorV1> {
        let lower = cholesky(&matrix, profile)?;
        Ok(Self { matrix, lower, profile, frame: PhantomData })
    }

    pub fn matrix(&self) -> &Matrix<N> {
        &self.matrix
    }

    pub fn tangent_norm(&self, tangent: &Tangent<Frame, N>) -> Result<f64, UncertaintyErrorV1> {
        let applied = apply(&self.matrix, &tangent.components);
        let square = dot(&tangent.components, &applied);
        if !square.is_finite() || square < 0.0 {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        Ok(square.sqrt())
    }

    pub fn cotangent_norm(&self, covector: &Covector<Frame, N>) -> Result<f64, UncertaintyErrorV1> {
        let whitened = solve_lower(&self.lower, &covector.components)?;
        norm_euclidean(&whitened)
    }

    /// Return L^-1 alpha, whose Euclidean norm is the M^-1 dual norm.
    pub fn whiten_covector(&self, covector: &Covector<Frame, N>) -> Result<Vector<N>, UncertaintyErrorV1> {
        solve_lower(&self.lower, &covector.components)
    }

    pub fn solve(&self, rhs: &Covector<Frame, N>) -> Result<Tangent<Frame, N>, UncertaintyErrorV1> {
        Tangent::new(solve_spd(&self.lower, &rhs.components)?)
    }

    /// H = L^-1 ΔM L^-T. The result is symmetrized only after the
    /// input perturbation has passed its own relative symmetry check.
    pub fn whiten_mass_perturbation(
        &self,
        perturbation: &Matrix<N>,
    ) -> Result<Matrix<N>, UncertaintyErrorV1> {
        validate_matrix(perturbation)?;
        let scale = matrix_inf(perturbation);
        if !scale.is_finite() {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        let symmetry_threshold = self.profile.relative_symmetry_tolerance * scale;
        if !symmetry_threshold.is_finite() {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        for row in 0..N {
            for column in (row + 1)..N {
                if (perturbation[row][column] - perturbation[column][row]).abs()
                    > symmetry_threshold
                {
                    return Err(UncertaintyErrorV1::AsymmetricMassPerturbation { row, column });
                }
            }
        }

        // B = L^-1 ΔM, computed column by column.
        let mut left_whitened = [[0.0; N]; N];
        for column in 0..N {
            let rhs = std::array::from_fn(|row| perturbation[row][column]);
            let solved = solve_lower(&self.lower, &rhs)?;
            for row in 0..N {
                left_whitened[row][column] = solved[row];
            }
        }

        // H = B L^-T. For each row, transpose it and solve L z = B[row,:]^T.
        let mut whitened = [[0.0; N]; N];
        for row in 0..N {
            let rhs = std::array::from_fn(|column| left_whitened[row][column]);
            let solved = solve_lower(&self.lower, &rhs)?;
            for column in 0..N {
                whitened[row][column] = solved[column];
            }
        }
        if whitened.iter().flatten().any(|value| !value.is_finite()) {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }

        // The exact matrix is symmetric. Canonicalize only the floating-point
        // skew introduced by the two triangular-solve passes.
        for row in 0..N {
            for column in (row + 1)..N {
                let average = 0.5 * whitened[row][column] + 0.5 * whitened[column][row];
                whitened[row][column] = average;
                whitened[column][row] = average;
            }
        }
        Ok(whitened)
    }

    pub fn rebase<To>(
        &self,
        coordinate_map: &CoordinateRebase<Frame, To, N>,
    ) -> Result<Metric<To, N>, UncertaintyErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&coordinate_map.source_from_target), &self.matrix),
            &coordinate_map.source_from_target,
        );
        Metric::new(transformed, self.profile)
    }
}

impl<From, To, const N: usize> CoordinateRebase<From, To, N> {
    /// Construct S, mapping target tangent components into source coordinates.
    pub fn new(source_from_target: Matrix<N>) -> Result<Self, UncertaintyErrorV1> {
        validate_matrix(&source_from_target)?;
        let scale = matrix_inf(&source_from_target);
        if scale == 0.0 || !scale.is_finite() {
            return Err(UncertaintyErrorV1::SingularCoordinateChange);
        }
        let threshold = 64.0 * f64::EPSILON * scale;
        let mut left = source_from_target;
        let mut right = [[0.0; N]; N];
        for (index, row) in right.iter_mut().enumerate() {
            row[index] = 1.0;
        }
        for column in 0..N {
            let pivot_row = (column..N)
                .max_by(|a, b| left[*a][column].abs().total_cmp(&left[*b][column].abs()))
                .ok_or(UncertaintyErrorV1::ZeroDimension)?;
            let pivot = left[pivot_row][column].abs();
            if !pivot.is_finite() || pivot <= threshold {
                return Err(UncertaintyErrorV1::SingularCoordinateChange);
            }
            if pivot_row != column {
                left.swap(pivot_row, column);
                right.swap(pivot_row, column);
            }
            let divisor = left[column][column];
            for entry in 0..N {
                left[column][entry] /= divisor;
                right[column][entry] /= divisor;
            }
            for row in 0..N {
                if row == column {
                    continue;
                }
                let multiplier = left[row][column];
                for entry in 0..N {
                    left[row][entry] -= multiplier * left[column][entry];
                    right[row][entry] -= multiplier * right[column][entry];
                }
            }
        }
        if right.iter().flatten().any(|value| !value.is_finite()) {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        let condition_inf_estimate = matrix_inf(&source_from_target) * matrix_inf(&right);
        if !condition_inf_estimate.is_finite() {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        Ok(Self {
            source_from_target,
            target_from_source: right,
            condition_inf_estimate,
            frames: PhantomData,
        })
    }

    pub fn condition_inf_estimate(&self) -> f64 {
        self.condition_inf_estimate
    }

    pub fn tangent_to_target(
        &self,
        tangent: &Tangent<From, N>,
    ) -> Result<Tangent<To, N>, UncertaintyErrorV1> {
        Tangent::new(apply(&self.target_from_source, &tangent.components))
    }

    pub fn covector_to_target(
        &self,
        covector: &Covector<From, N>,
    ) -> Result<Covector<To, N>, UncertaintyErrorV1> {
        Covector::new(apply(&transpose(&self.source_from_target), &covector.components))
    }

    pub fn metric_to_target(
        &self,
        metric: &Metric<From, N>,
    ) -> Result<Metric<To, N>, UncertaintyErrorV1> {
        metric.rebase(self)
    }

    pub fn perturbation_to_target(
        &self,
        perturbation: &Matrix<N>,
    ) -> Result<Matrix<N>, UncertaintyErrorV1> {
        let transformed = multiply(
            &multiply(&transpose(&self.source_from_target), perturbation),
            &self.source_from_target,
        );
        if transformed.iter().flatten().any(|value| !value.is_finite()) {
            return Err(UncertaintyErrorV1::NonFiniteIntermediate);
        }
        Ok(transformed)
    }
}

pub fn estimate_acceleration_uncertainty<Frame, const N: usize>(
    metric: &Metric<Frame, N>,
    nominal_acceleration: &Tangent<Frame, N>,
    mass_perturbation: &Matrix<N>,
    force_perturbation: &Covector<Frame, N>,
) -> Result<PerturbationEnvelopeV1, UncertaintyErrorV1> {
    let whitened_mass_perturbation = metric.whiten_mass_perturbation(mass_perturbation)?;
    let mut frobenius = 0.0_f64;
    for value in whitened_mass_perturbation.iter().flatten() {
        frobenius = frobenius.hypot(*value);
    }
    if !frobenius.is_finite() {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    let rho = frobenius
        * (1.0 + metric.profile.relative_frobenius_roundoff_inflation);
    if !rho.is_finite() {
        return Err(UncertaintyErrorV1::NonFiniteIntermediate);
    }
    let force_uncertainty_dual_norm = metric.cotangent_norm(force_perturbation)?;
    let nominal_acceleration_mass_norm = metric.tangent_norm(nominal_acceleration)?;

    if rho >= 1.0 || rho > metric.profile.maximum_admissible_rho {
        return Ok(PerturbationEnvelopeV1 {
            relative_mass_perturbation_rho: rho,
            force_uncertainty_dual_norm,
            nominal_acceleration_mass_norm,
            acceleration_error_bound_mass_norm: None,
            status: EnvelopeStatusV1::OutsidePerturbationEnvelope,
        });
    }

    let numerator =
        force_uncertainty_dual_norm + rho * nominal_acceleration_mass_norm;
    let denominator = 1.0 - rho;
    let bound = numerator / denominator;
    if !bound.is_finite() {
        return Ok(PerturbationEnvelopeV1 {
            relative_mass_perturbation_rho: rho,
            force_uncertainty_dual_norm,
            nominal_acceleration_mass_norm,
            acceleration_error_bound_mass_norm: None,
            status: EnvelopeStatusV1::VacuousEstimate,
        });
    }
    let status = if rho <= metric.profile.maximum_informative_rho {
        EnvelopeStatusV1::InformativeEstimate
    } else {
        EnvelopeStatusV1::WeakEstimate
    };
    Ok(PerturbationEnvelopeV1 {
        relative_mass_perturbation_rho: rho,
        force_uncertainty_dual_norm,
        nominal_acceleration_mass_norm,
        acceleration_error_bound_mass_norm: Some(bound),
        status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (
        Metric<CartesianFrame, 3>,
        CoordinateRebase<CartesianFrame, DenseFrame, 3>,
        Tangent<CartesianFrame, 3>,
        Covector<CartesianFrame, 3>,
    ) {
        let profile = UncertaintyProfileV1::default();
        let metric = Metric::new(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile,
        )
        .unwrap();
        let rebase = CoordinateRebase::new([
            [1.0, 2.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 0.0, 1.0],
        ])
        .unwrap();
        let acceleration = Tangent::new([0.5, -2.0 / 3.0, 0.8]).unwrap();
        let force = Covector::new(apply(metric.matrix(), acceleration.components())).unwrap();
        (metric, rebase, acceleration, force)
    }

    fn diagonal_mass_perturbation() -> Matrix<3> {
        [[0.02, 0.0, 0.0], [0.0, -0.03, 0.0], [0.0, 0.0, 0.05]]
    }

    fn force_perturbation() -> Vector<3> {
        [0.01, -0.02, 0.03]
    }

    #[test]
    fn mass_and_force_uncertainty_bound_covers_perturbed_spd_solution() {
        let (metric, _, acceleration, _) = fixture();
        let delta_mass = diagonal_mass_perturbation();
        let delta_force = Covector::new(force_perturbation()).unwrap();
        let profile = UncertaintyProfileV1::default();
        let envelope = estimate_acceleration_uncertainty(
            &metric,
            &acceleration,
            &delta_mass,
            &delta_force,
        )
        .unwrap();
        assert!(matches!(
            envelope.status,
            EnvelopeStatusV1::InformativeEstimate | EnvelopeStatusV1::WeakEstimate
        ));
        let bound = envelope.acceleration_error_bound_mass_norm.unwrap();

        let perturbed_mass = std::array::from_fn(|row| {
            std::array::from_fn(|column| metric.matrix()[row][column] + delta_mass[row][column])
        });
        let perturbed_metric = Metric::new(perturbed_mass, profile).unwrap();
        let nominal_force = Covector::new(apply(metric.matrix(), acceleration.components())).unwrap();
        let true_force = Covector::new(std::array::from_fn(|i| {
            nominal_force.components()[i] + delta_force.components()[i]
        }))
        .unwrap();
        let true_acceleration = perturbed_metric.solve(&true_force).unwrap();
        let error = std::array::from_fn(|i| {
            true_acceleration.components()[i] - acceleration.components()[i]
        });
        let error_tangent = Tangent::new(error).unwrap();
        let observed = metric.tangent_norm(&error_tangent).unwrap();
        assert!(observed <= bound + 1.0e-12);
    }

    #[test]
    fn uncertainty_envelope_is_invariant_under_dense_rebasing() {
        let (metric_q, rebase, acceleration_q, force_q) = fixture();
        let delta_mass_q = diagonal_mass_perturbation();
        let delta_force_q = Covector::new(force_perturbation()).unwrap();
        let envelope_q = estimate_acceleration_uncertainty(
            &metric_q,
            &acceleration_q,
            &delta_mass_q,
            &delta_force_q,
        )
        .unwrap();

        let metric_y = rebase.metric_to_target(&metric_q).unwrap();
        let acceleration_y = rebase.tangent_to_target(&acceleration_q).unwrap();
        let delta_mass_y = rebase.perturbation_to_target(&delta_mass_q).unwrap();
        let delta_force_y = rebase.covector_to_target(&delta_force_q).unwrap();
        let envelope_y = estimate_acceleration_uncertainty(
            &metric_y,
            &acceleration_y,
            &delta_mass_y,
            &delta_force_y,
        )
        .unwrap();

        assert!(
            (envelope_q.relative_mass_perturbation_rho
                - envelope_y.relative_mass_perturbation_rho)
                .abs()
                < 2.0e-12
        );
        assert!(
            (envelope_q.force_uncertainty_dual_norm - envelope_y.force_uncertainty_dual_norm).abs()
                < 2.0e-12
        );
        assert!(
            (envelope_q.nominal_acceleration_mass_norm
                - envelope_y.nominal_acceleration_mass_norm)
                .abs()
                < 2.0e-12
        );
        assert!(
            (envelope_q.acceleration_error_bound_mass_norm.unwrap()
                - envelope_y.acceleration_error_bound_mass_norm.unwrap())
                .abs()
                < 3.0e-12
        );
    }

    #[test]
    fn force_only_uncertainty_reduces_to_exact_dual_norm_identity() {
        let (metric, _, acceleration, _) = fixture();
        let delta_force = Covector::new(force_perturbation()).unwrap();
        let zero_mass = [[0.0; 3]; 3];
        let envelope = estimate_acceleration_uncertainty(
            &metric,
            &acceleration,
            &zero_mass,
            &delta_force,
        )
        .unwrap();
        let exact_error = metric.solve(&delta_force).unwrap();
        let observed = metric.tangent_norm(&exact_error).unwrap();
        assert!((observed - envelope.force_uncertainty_dual_norm).abs() < 1.0e-12);
        assert!(
            (observed - envelope.acceleration_error_bound_mass_norm.unwrap()).abs() < 1.0e-12
        );
    }

    #[test]
    fn rho_at_or_above_one_abstains_without_a_finite_bound() {
        let (metric, _, acceleration, _) = fixture();
        let perturbation = [
            [2.0, 0.0, 0.0],
            [0.0, 3.0, 0.0],
            [0.0, 0.0, 5.0],
        ];
        let delta_force = Covector::new([0.0; 3]).unwrap();
        let envelope = estimate_acceleration_uncertainty(
            &metric,
            &acceleration,
            &perturbation,
            &delta_force,
        )
        .unwrap();
        assert!(envelope.relative_mass_perturbation_rho >= 1.0);
        assert_eq!(envelope.status, EnvelopeStatusV1::OutsidePerturbationEnvelope);
        assert!(envelope.acceleration_error_bound_mass_norm.is_none());
    }

    #[test]
    fn mass_perturbation_must_be_symmetric() {
        let (metric, _, acceleration, _) = fixture();
        let asymmetric = [[0.0, 0.1, 0.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]];
        let delta_force = Covector::new([0.0; 3]).unwrap();
        assert!(matches!(
            estimate_acceleration_uncertainty(
                &metric,
                &acceleration,
                &asymmetric,
                &delta_force
            ),
            Err(UncertaintyErrorV1::AsymmetricMassPerturbation { .. })
        ));
    }

    #[test]
    fn wrong_force_covector_transformation_changes_the_envelope() {
        let (metric_q, rebase, acceleration_q, _) = fixture();
        let delta_mass_q = diagonal_mass_perturbation();
        let delta_force_q = Covector::new(force_perturbation()).unwrap();
        let envelope_q = estimate_acceleration_uncertainty(
            &metric_q,
            &acceleration_q,
            &delta_mass_q,
            &delta_force_q,
        )
        .unwrap();

        let metric_y = rebase.metric_to_target(&metric_q).unwrap();
        let acceleration_y = rebase.tangent_to_target(&acceleration_q).unwrap();
        let delta_mass_y = rebase.perturbation_to_target(&delta_mass_q).unwrap();
        // Deliberately wrong: a covector is transformed as a tangent vector.
        let wrong_force_y = Covector::<DenseFrame, 3>::new(apply(
            &rebase.target_from_source,
            delta_force_q.components(),
        ))
        .unwrap();
        let wrong = estimate_acceleration_uncertainty(
            &metric_y,
            &acceleration_y,
            &delta_mass_y,
            &wrong_force_y,
        )
        .unwrap();
        assert!(
            (envelope_q.acceleration_error_bound_mass_norm.unwrap()
                - wrong.acceleration_error_bound_mass_norm.unwrap())
                .abs()
                > 1.0e-3
        );
    }

    #[test]
    fn nonfinite_and_overflow_inputs_fail_closed() {
        let (metric, _, acceleration, _) = fixture();
        let delta_force = Covector::new([f64::MAX, f64::MAX, f64::MAX]).unwrap();
        let perturbation = [
            [f64::MAX, 0.0, 0.0],
            [0.0, f64::MAX, 0.0],
            [0.0, 0.0, f64::MAX],
        ];
        assert_eq!(
            estimate_acceleration_uncertainty(
                &metric,
                &acceleration,
                &perturbation,
                &delta_force
            ),
            Err(UncertaintyErrorV1::NonFiniteIntermediate)
        );
    }
}
