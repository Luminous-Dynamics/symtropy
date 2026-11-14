// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Rank-aware quotient geometry for redundant linear holonomic constraints.
//!
//! W = J M^-1 J^T may be positive semidefinite when constraint rows are
//! redundant. Multipliers are equivalence classes modulo ker(J^T); the
//! generalized-force correction and minimum-kinetic correction are unique.
//!
//! Numerical rank and residual compatibility are evaluated on the doubly
//! whitened operator B = L_H^T J L_M^-T. A one-sided Jacobi SVD avoids
//! forming normal equations for the quotient solve. Under generalized and
//! constraint-coordinate changes, B changes by orthogonal factors, so its
//! singular values are the invariant rank coordinates.

use super::{ConstraintMultiplierV1, ConstraintResidualV1};
use crate::{CoordinateFrame, Cotangent, Metric, MetricErrorV1, MetricProfileV1, Tangent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintQuotientProfileV1 {
    pub relative_rank_tolerance: f64,
    /// Multiplicative uncertainty band around the singular-value rank threshold; must exceed one.
    pub rank_uncertainty_factor: f64,
    pub relative_compatibility_tolerance: f64,
    /// Relative consistency threshold for reconstructed physical equations.
    pub relative_reduced_pivot_tolerance: f64,
}

impl Default for ConstraintQuotientProfileV1 {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-10,
            rank_uncertainty_factor: 10.0,
            relative_compatibility_tolerance: 1.0e-10,
            relative_reduced_pivot_tolerance: 128.0 * f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintResidualStatusV1 {
    Compatible,
    Incompatible,
    Inconclusive,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricAdmissionSummaryV1 {
    pub matrix_inf_norm: f64,
    pub minimum_cholesky_pivot: f64,
    pub maximum_relative_symmetry_defect: f64,
}

impl From<crate::MetricAdmissionReceiptV1> for MetricAdmissionSummaryV1 {
    fn from(value: crate::MetricAdmissionReceiptV1) -> Self {
        Self {
            matrix_inf_norm: value.matrix_inf_norm,
            minimum_cholesky_pivot: value.minimum_cholesky_pivot,
            maximum_relative_symmetry_defect: value.maximum_relative_symmetry_defect,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintQuotientAdmissionReceiptV1 {
    pub numerical_rank: usize,
    pub multiplier_nullity: usize,
    pub rank_resolved: bool,
    /// Relative singular value inside the declared uncertainty band, if any.
    pub unresolved_relative_pivot: Option<f64>,
    /// Orthogonality defect of the retained left singular vectors.
    pub maximum_residual_basis_orthogonality_defect: f64,
    pub minimum_resolved_singular_value: Option<f64>,
    pub jacobi_svd_converged: bool,
    pub jacobi_sweeps: usize,
    pub residual_metric: MetricAdmissionSummaryV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintQuotientErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteJacobian,
    NonFiniteIntermediate,
    NegativeQuotientQuadraticForm,
    Metric(MetricErrorV1),
}

impl From<MetricErrorV1> for ConstraintQuotientErrorV1 {
    fn from(value: MetricErrorV1) -> Self {
        Self::Metric(value)
    }
}

impl core::fmt::Display for ConstraintQuotientErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConstraintQuotientErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintQuotientSolveReceiptV1<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    status: ConstraintResidualStatusV1,
    numerical_rank: usize,
    multiplier_nullity: usize,
    compatibility_relative_defect: f64,
    reduced_solve_relative_defect: f64,
    multiplier_representative: Option<ConstraintMultiplierV1<Space, K>>,
    residual_dual_norm: Option<f64>,
    minimum_kinetic_correction: Option<Tangent<Frame, N>>,
    minimum_kinetic_correction_norm: Option<f64>,
}

impl<Frame: CoordinateFrame, Space: CoordinateFrame, const N: usize, const K: usize>
    ConstraintQuotientSolveReceiptV1<Frame, Space, N, K>
{
    pub fn status(&self) -> ConstraintResidualStatusV1 {
        self.status
    }
    pub fn numerical_rank(&self) -> usize {
        self.numerical_rank
    }
    pub fn multiplier_nullity(&self) -> usize {
        self.multiplier_nullity
    }
    pub fn compatibility_relative_defect(&self) -> f64 {
        self.compatibility_relative_defect
    }
    pub fn reduced_solve_relative_defect(&self) -> f64 {
        self.reduced_solve_relative_defect
    }
    pub fn multiplier_representative(&self) -> Option<&ConstraintMultiplierV1<Space, K>> {
        self.multiplier_representative.as_ref()
    }
    pub fn residual_dual_norm(&self) -> Option<f64> {
        self.residual_dual_norm
    }
    pub fn minimum_kinetic_correction(&self) -> Option<&Tangent<Frame, N>> {
        self.minimum_kinetic_correction.as_ref()
    }
    pub fn minimum_kinetic_correction_norm(&self) -> Option<f64> {
        self.minimum_kinetic_correction_norm
    }
}

fn validate_profile(profile: ConstraintQuotientProfileV1) -> Result<(), ConstraintQuotientErrorV1> {
    if !profile.relative_rank_tolerance.is_finite()
        || profile.relative_rank_tolerance <= 0.0
        || profile.relative_rank_tolerance >= 1.0
        || !profile.rank_uncertainty_factor.is_finite()
        || profile.rank_uncertainty_factor <= 1.0
        || profile.rank_uncertainty_factor > 1.0e6
        || !profile.relative_compatibility_tolerance.is_finite()
        || profile.relative_compatibility_tolerance <= 0.0
        || profile.relative_compatibility_tolerance >= 1.0
        || !profile.relative_reduced_pivot_tolerance.is_finite()
        || profile.relative_reduced_pivot_tolerance <= 0.0
        || profile.relative_reduced_pivot_tolerance >= 1.0
    {
        return Err(ConstraintQuotientErrorV1::InvalidProfile);
    }
    Ok(())
}

fn h_norm<Space: CoordinateFrame, const K: usize>(
    metric: &Metric<Space, K>,
    vector: &[f64; K],
) -> Result<f64, ConstraintQuotientErrorV1> {
    let whitened = metric.whiten_tangent(&Tangent::<Space, K>::new(*vector)?)?;
    let norm = crate::vector_euclidean(&whitened);
    if norm.is_finite() {
        Ok(norm)
    } else {
        Err(ConstraintQuotientErrorV1::NonFiniteIntermediate)
    }
}

fn mat_vec<const K: usize>(
    matrix: &[[f64; K]; K],
    vector: &[f64; K],
) -> Result<[f64; K], ConstraintQuotientErrorV1> {
    let mut result = [0.0; K];
    for row in 0..K {
        for column in 0..K {
            result[row] += matrix[row][column] * vector[column];
        }
        if !result[row].is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(result)
}

#[derive(Debug, Clone, Copy)]
struct WhitenedConstraintSvdV1<const N: usize, const K: usize> {
    left: [[f64; N]; K],
    right: [[f64; N]; N],
    singular_values: [f64; N],
    rank: usize,
    rank_resolved: bool,
    unresolved_relative_singular: Option<f64>,
    orthogonality_defect: f64,
    converged: bool,
    sweeps: usize,
}

fn one_sided_jacobi_svd<const N: usize, const K: usize>(
    input: &[[f64; N]; K],
) -> Result<([[f64; N]; K], [[f64; N]; N], [f64; N], bool, usize), ConstraintQuotientErrorV1> {
    let mut work = *input;
    let mut right = [[0.0; N]; N];
    for index in 0..N {
        right[index][index] = 1.0;
    }

    // The cyclic one-sided Jacobi method orthogonalizes columns directly.
    // Unlike eigendecomposition of B^T B or B B^T, it does not square the
    // condition number before revealing small singular values.
    let relative_jacobi_tolerance = 16.0 * f64::EPSILON;
    let maximum_sweeps = N.saturating_mul(N).saturating_mul(8).max(32);
    let mut converged = false;
    let mut sweeps = 0usize;

    for sweep in 0..maximum_sweeps {
        let mut rotated = false;
        for left_column in 0..N {
            for right_column in (left_column + 1)..N {
                let mut alpha = 0.0_f64;
                let mut beta = 0.0_f64;
                let mut gamma = 0.0_f64;
                for row in 0..K {
                    let left_value = work[row][left_column];
                    let right_value = work[row][right_column];
                    alpha += left_value * left_value;
                    beta += right_value * right_value;
                    gamma += left_value * right_value;
                }
                if !alpha.is_finite() || !beta.is_finite() || !gamma.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                if alpha == 0.0 || beta == 0.0 {
                    continue;
                }
                let cross_scale = alpha.sqrt() * beta.sqrt();
                if !cross_scale.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                if gamma.abs() <= relative_jacobi_tolerance * cross_scale {
                    continue;
                }

                let zeta = (beta - alpha) / (2.0 * gamma);
                if !zeta.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                let tangent = if zeta == 0.0 {
                    1.0
                } else {
                    zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt())
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;
                if !cosine.is_finite() || !sine.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }

                for row in 0..K {
                    let left_value = work[row][left_column];
                    let right_value = work[row][right_column];
                    work[row][left_column] = cosine * left_value - sine * right_value;
                    work[row][right_column] = sine * left_value + cosine * right_value;
                }
                for row in 0..N {
                    let left_value = right[row][left_column];
                    let right_value = right[row][right_column];
                    right[row][left_column] = cosine * left_value - sine * right_value;
                    right[row][right_column] = sine * left_value + cosine * right_value;
                }
                rotated = true;
            }
        }

        sweeps = sweep + 1;
        if !rotated {
            converged = true;
            break;
        }
    }

    let mut singular_values = [0.0; N];
    for column in 0..N {
        let values: [f64; K] = std::array::from_fn(|row| work[row][column]);
        singular_values[column] = crate::vector_euclidean(&values);
        if !singular_values[column].is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }
    }

    // Deterministic descending singular-value order; tie breaks retain the
    // prior column order for stable replay.
    for position in 0..N {
        let mut selected = position;
        for candidate in (position + 1)..N {
            if singular_values[candidate] > singular_values[selected] {
                selected = candidate;
            }
        }
        if selected != position {
            singular_values.swap(position, selected);
            for row in 0..K {
                work[row].swap(position, selected);
            }
            for row in 0..N {
                right[row].swap(position, selected);
            }
        }
    }

    let mut left = [[0.0; N]; K];
    for column in 0..N {
        let singular_value = singular_values[column];
        if singular_value > 0.0 {
            for row in 0..K {
                left[row][column] = work[row][column] / singular_value;
                if !left[row][column].is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
            }
        }
    }

    Ok((left, right, singular_values, converged, sweeps))
}

fn build_whitened_constraint_svd<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    mass: &Metric<Frame, N>,
    jacobian: &[[f64; N]; K],
    residual_metric: &Metric<Space, K>,
    profile: ConstraintQuotientProfileV1,
) -> Result<WhitenedConstraintSvdV1<N, K>, ConstraintQuotientErrorV1> {
    // A = L_H^T J L_M^-T. First whiten every constraint covector by M,
    // then whiten each resulting residual-space column by H.
    let mut mass_whitened_rows = [[0.0; N]; K];
    for row in 0..K {
        let covector = Cotangent::<Frame, N>::new(jacobian[row])?;
        mass_whitened_rows[row] = mass.whiten_cotangent(&covector)?;
    }

    let mut whitened_jacobian = [[0.0; N]; K];
    for column in 0..N {
        let residual_column =
            std::array::from_fn(|row| mass_whitened_rows[row][column]);
        let tangent = Tangent::<Space, K>::new(residual_column)?;
        let whitened_column = residual_metric.whiten_tangent(&tangent)?;
        for row in 0..K {
            whitened_jacobian[row][column] = whitened_column[row];
        }
    }

    let (left, right, singular_values, converged, sweeps) =
        one_sided_jacobi_svd(&whitened_jacobian)?;
    let maximum_singular_value = singular_values[0];
    let mut rank = 0usize;
    let mut rank_resolved = converged;
    let mut unresolved_relative_singular = None;

    if maximum_singular_value > 0.0 {
        let threshold = profile.relative_rank_tolerance * maximum_singular_value;
        let lower_threshold = threshold / profile.rank_uncertainty_factor;
        let upper_threshold = threshold * profile.rank_uncertainty_factor;
        if !threshold.is_finite() || !lower_threshold.is_finite() || !upper_threshold.is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }
        for singular_value in singular_values {
            if singular_value <= lower_threshold {
                break;
            }
            if singular_value < upper_threshold {
                rank_resolved = false;
                unresolved_relative_singular = Some(singular_value / maximum_singular_value);
                break;
            }
            rank += 1;
        }
    }

    let mut orthogonality_defect = 0.0_f64;
    for left_axis in 0..rank {
        for right_axis in 0..rank {
            let left_column: [f64; K] =
                std::array::from_fn(|row| left[row][left_axis]);
            let right_column: [f64; K] =
                std::array::from_fn(|row| left[row][right_axis]);
            let actual = crate::dot(&left_column, &right_column);
            let expected = if left_axis == right_axis { 1.0 } else { 0.0 };
            orthogonality_defect = orthogonality_defect.max((actual - expected).abs());
        }
    }

    Ok(WhitenedConstraintSvdV1 {
        left,
        right,
        singular_values,
        rank,
        rank_resolved,
        unresolved_relative_singular,
        orthogonality_defect,
        converged,
        sweeps,
    })
}

/// Rank-aware Delassus quotient for W = J M^-1 J^T.
///
/// The solve uses the SVD of the doubly whitened constraint operator. The
/// original Delassus matrix is retained for multiplier-fibre diagnostics and
/// independent equation reconstruction, but is not used as a normal equation
/// for the minimum-kinetic correction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RankDeficientDelassusQuotientV1<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    mass: Metric<Frame, N>,
    jacobian: [[f64; N]; K],
    delassus: [[f64; K]; K],
    residual_metric: Metric<Space, K>,
    left_singular_vectors: [[f64; N]; K],
    right_singular_vectors: [[f64; N]; N],
    singular_values: [f64; N],
    rank: usize,
    rank_resolved: bool,
    profile: ConstraintQuotientProfileV1,
    receipt: ConstraintQuotientAdmissionReceiptV1,
}

impl<Frame: CoordinateFrame, Space: CoordinateFrame, const N: usize, const K: usize>
    RankDeficientDelassusQuotientV1<Frame, Space, N, K>
{
    pub fn new(
        mass: &Metric<Frame, N>,
        jacobian: [[f64; N]; K],
        residual_metric: &Metric<Space, K>,
        profile: ConstraintQuotientProfileV1,
    ) -> Result<Self, ConstraintQuotientErrorV1> {
        validate_profile(profile)?;
        if N == 0 || K == 0 {
            return Err(ConstraintQuotientErrorV1::ZeroDimension);
        }
        if jacobian.iter().flatten().any(|value| !value.is_finite()) {
            return Err(ConstraintQuotientErrorV1::NonFiniteJacobian);
        }

        // W = J M^-1 J^T, formed from whitened covectors without explicitly
        // forming M^-1. This remains an independent operator for verification.
        let mut mass_whitened_rows = [[0.0; N]; K];
        for row in 0..K {
            let covector = Cotangent::<Frame, N>::new(jacobian[row])?;
            mass_whitened_rows[row] = mass.whiten_cotangent(&covector)?;
        }
        let mut delassus = [[0.0; K]; K];
        for row in 0..K {
            for column in row..K {
                let value = crate::dot(&mass_whitened_rows[row], &mass_whitened_rows[column]);
                if !value.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                delassus[row][column] = value;
                delassus[column][row] = value;
            }
        }

        let svd = build_whitened_constraint_svd(mass, &jacobian, residual_metric, profile)?;
        let minimum_resolved_singular_value = if svd.rank > 0 {
            Some(svd.singular_values[svd.rank - 1])
        } else {
            None
        };

        Ok(Self {
            mass: *mass,
            jacobian,
            delassus,
            residual_metric: *residual_metric,
            left_singular_vectors: svd.left,
            right_singular_vectors: svd.right,
            singular_values: svd.singular_values,
            rank: svd.rank,
            rank_resolved: svd.rank_resolved,
            profile,
            receipt: ConstraintQuotientAdmissionReceiptV1 {
                numerical_rank: svd.rank,
                multiplier_nullity: K - svd.rank,
                rank_resolved: svd.rank_resolved,
                unresolved_relative_pivot: svd.unresolved_relative_singular,
                maximum_residual_basis_orthogonality_defect: svd.orthogonality_defect,
                minimum_resolved_singular_value,
                jacobi_svd_converged: svd.converged,
                jacobi_sweeps: svd.sweeps,
                residual_metric: residual_metric.admission_receipt().into(),
            },
        })
    }

    pub fn rank(&self) -> usize {
        self.rank
    }
    pub fn nullity(&self) -> usize {
        K - self.rank
    }
    pub fn receipt(&self) -> ConstraintQuotientAdmissionReceiptV1 {
        self.receipt
    }

    /// Quotient norm sqrt(lambda^T W lambda), invariant under lambda -> lambda+z
    /// whenever z is in ker(J^T).
    pub fn multiplier_quotient_norm(
        &self,
        multiplier: &ConstraintMultiplierV1<Space, K>,
    ) -> Result<f64, ConstraintQuotientErrorV1> {
        let mapped = mat_vec(&self.delassus, multiplier.components())?;
        let squared = crate::dot(multiplier.components(), &mapped);
        let scale =
            crate::vector_euclidean(multiplier.components()) * crate::vector_euclidean(&mapped);
        if !squared.is_finite()
            || !scale.is_finite()
            || squared < -self.profile.relative_rank_tolerance * scale
        {
            return Err(ConstraintQuotientErrorV1::NegativeQuotientQuadraticForm);
        }
        Ok(squared.max(0.0).sqrt())
    }

    /// Solve W lambda = r on the quotient using the SVD of
    /// B = L_H^T J L_M^-T. The minimum-kinetic correction is obtained directly
    /// from B^+; the multiplier representative is reconstructed in whitened
    /// dual coordinates and checked against the original Delassus operator.
    pub fn solve_residual(
        &self,
        residual: &ConstraintResidualV1<Space, K>,
    ) -> Result<ConstraintQuotientSolveReceiptV1<Frame, Space, N, K>, ConstraintQuotientErrorV1>
    {
        let residual_components = *residual.components();
        let whitened_residual = self
            .residual_metric
            .whiten_tangent(&Tangent::<Space, K>::new(residual_components)?)?;
        let residual_norm = crate::vector_euclidean(&whitened_residual);

        let mut coefficients = [0.0; N];
        let mut projected = [0.0; K];
        for axis in 0..self.rank {
            for row in 0..K {
                coefficients[axis] +=
                    self.left_singular_vectors[row][axis] * whitened_residual[row];
            }
            for row in 0..K {
                projected[row] += self.left_singular_vectors[row][axis] * coefficients[axis];
            }
        }
        let whitened_defect: [f64; K] =
            std::array::from_fn(|row| whitened_residual[row] - projected[row]);
        let defect_norm = crate::vector_euclidean(&whitened_defect);
        let compatibility_relative_defect = if residual_norm == 0.0 {
            defect_norm
        } else {
            defect_norm / residual_norm
        };
        if !compatibility_relative_defect.is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }

        if !self.rank_resolved {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                f64::INFINITY,
            ));
        }

        let tolerance = self.profile.relative_compatibility_tolerance;
        if compatibility_relative_defect > 10.0 * tolerance {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Incompatible,
                compatibility_relative_defect,
                f64::INFINITY,
            ));
        }
        if compatibility_relative_defect > tolerance {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                f64::INFINITY,
            ));
        }

        // z = V Sigma^-1 U^T b is the minimum-Euclidean-norm solution
        // of B z = b, hence delta-v = L_M^-T z is the minimum-kinetic
        // physical correction.
        let mut whitened_correction = [0.0; N];
        let mut whitened_multiplier = [0.0; K];
        for axis in 0..self.rank {
            let singular_value = self.singular_values[axis];
            if !singular_value.is_finite() || singular_value <= 0.0 {
                return Ok(self.empty_receipt(
                    ConstraintResidualStatusV1::Inconclusive,
                    compatibility_relative_defect,
                    f64::INFINITY,
                ));
            }
            let inverse = 1.0 / singular_value;
            let inverse_squared = inverse * inverse;
            if !inverse.is_finite() || !inverse_squared.is_finite() {
                return Ok(self.empty_receipt(
                    ConstraintResidualStatusV1::Inconclusive,
                    compatibility_relative_defect,
                    f64::INFINITY,
                ));
            }
            let correction_coefficient = coefficients[axis] * inverse;
            let multiplier_coefficient = coefficients[axis] * inverse_squared;
            if !correction_coefficient.is_finite() || !multiplier_coefficient.is_finite() {
                return Ok(self.empty_receipt(
                    ConstraintResidualStatusV1::Inconclusive,
                    compatibility_relative_defect,
                    f64::INFINITY,
                ));
            }
            for dof in 0..N {
                whitened_correction[dof] +=
                    self.right_singular_vectors[dof][axis] * correction_coefficient;
            }
            for row in 0..K {
                whitened_multiplier[row] +=
                    self.left_singular_vectors[row][axis] * multiplier_coefficient;
            }
        }
        if whitened_correction.iter().any(|value| !value.is_finite())
            || whitened_multiplier.iter().any(|value| !value.is_finite())
        {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                f64::INFINITY,
            ));
        }

        let lambda = self.residual_metric.unwhiten_cotangent(&whitened_multiplier)?;
        let multiplier = ConstraintMultiplierV1::<Space, K>::new(*lambda.components())?;
        let dual_pairing = crate::dot(lambda.components(), &residual_components);
        if !dual_pairing.is_finite()
            || dual_pairing < -tolerance * residual_norm.max(1.0)
        {
            return Err(ConstraintQuotientErrorV1::NegativeQuotientQuadraticForm);
        }
        let residual_dual_norm = dual_pairing.max(0.0).sqrt();

        let mut generalized_force = [0.0; N];
        for row in 0..K {
            for dof in 0..N {
                generalized_force[dof] +=
                    self.jacobian[row][dof] * multiplier.components()[row];
            }
        }
        let correction = self.mass.raise(&Cotangent::<Frame, N>::new(generalized_force)?)?;
        let kinetic_norm = self.mass.tangent_norm(&correction)?;
        let recovered_whitened_correction = self.mass.whiten_tangent(&correction)?;
        let correction_defect: [f64; N] = std::array::from_fn(|dof| {
            recovered_whitened_correction[dof] - whitened_correction[dof]
        });
        let correction_relative_defect = crate::vector_euclidean(&correction_defect)
            / crate::vector_euclidean(&whitened_correction).max(1.0);

        let mapped = mat_vec(&self.delassus, multiplier.components())?;
        let equation_defect: [f64; K] =
            std::array::from_fn(|row| mapped[row] - residual_components[row]);
        let equation_relative_defect =
            h_norm(&self.residual_metric, &equation_defect)? / residual_norm.max(1.0);

        let mut reconstructed = [0.0; K];
        for row in 0..K {
            for dof in 0..N {
                reconstructed[row] += self.jacobian[row][dof] * correction.components()[dof];
            }
        }
        let reconstruction_defect: [f64; K] =
            std::array::from_fn(|row| reconstructed[row] - residual_components[row]);
        let reconstruction_relative_defect =
            h_norm(&self.residual_metric, &reconstruction_defect)? / residual_norm.max(1.0);

        let norm_identity_defect = (residual_dual_norm - kinetic_norm).abs()
            / residual_dual_norm.max(kinetic_norm).max(1.0);
        let reduced_solve_relative_defect = correction_relative_defect
            .max(equation_relative_defect)
            .max(reconstruction_relative_defect)
            .max(norm_identity_defect);
        if !reduced_solve_relative_defect.is_finite()
            || reduced_solve_relative_defect > tolerance
        {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                reduced_solve_relative_defect,
            ));
        }

        Ok(ConstraintQuotientSolveReceiptV1 {
            status: ConstraintResidualStatusV1::Compatible,
            numerical_rank: self.rank,
            multiplier_nullity: K - self.rank,
            compatibility_relative_defect,
            reduced_solve_relative_defect,
            multiplier_representative: Some(multiplier),
            residual_dual_norm: Some(residual_dual_norm),
            minimum_kinetic_correction: Some(correction),
            minimum_kinetic_correction_norm: Some(kinetic_norm),
        })
    }

    fn empty_receipt(
        &self,
        status: ConstraintResidualStatusV1,
        compatibility_relative_defect: f64,
        reduced_solve_relative_defect: f64,
    ) -> ConstraintQuotientSolveReceiptV1<Frame, Space, N, K> {
        ConstraintQuotientSolveReceiptV1 {
            status,
            numerical_rank: self.rank,
            multiplier_nullity: K - self.rank,
            compatibility_relative_defect,
            reduced_solve_relative_defect,
            multiplier_representative: None,
            residual_dual_norm: None,
            minimum_kinetic_correction: None,
            minimum_kinetic_correction_norm: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct Cartesian;
    impl CoordinateFrame for Cartesian {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct OldConstraintSpace;
    impl CoordinateFrame for OldConstraintSpace {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct NewConstraintSpace;
    impl CoordinateFrame for NewConstraintSpace {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct DenseFrame;
    impl CoordinateFrame for DenseFrame {}

    fn metric_profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }
    fn quotient_profile() -> ConstraintQuotientProfileV1 {
        ConstraintQuotientProfileV1 {
            relative_rank_tolerance: 1.0e-10,
            rank_uncertainty_factor: 10.0,
            relative_compatibility_tolerance: 1.0e-9,
            relative_reduced_pivot_tolerance: 1.0e-12,
        }
    }
    fn mass() -> Metric<Cartesian, 3> {
        Metric::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap()
    }
    fn residual_metric<Space: CoordinateFrame>() -> Metric<Space, 3> {
        Metric::admit(
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap()
    }
    fn redundant_jacobian() -> [[f64; 3]; 3] {
        [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 0.0, 1.0]]
    }
    fn compatible_residual() -> [f64; 3] {
        let velocity = [0.1, -0.2, 0.3];
        let jacobian = redundant_jacobian();
        std::array::from_fn(|row| {
            (0..3)
                .map(|column| jacobian[row][column] * velocity[column])
                .sum()
        })
    }
    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "{left} != {right}"
        );
    }

    #[test]
    fn duplicated_rows_create_multiplier_quotient_but_unique_physical_correction() {
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            redundant_jacobian(),
            &residual_metric(),
            quotient_profile(),
        )
        .unwrap();
        assert_eq!(quotient.rank(), 2);
        assert_eq!(quotient.nullity(), 1);

        let residual =
            ConstraintResidualV1::<OldConstraintSpace, 3>::new(compatible_residual()).unwrap();
        let solved = quotient.solve_residual(&residual).unwrap();
        assert_eq!(solved.status(), ConstraintResidualStatusV1::Compatible);
        let correction = solved.minimum_kinetic_correction().unwrap();
        close(
            quotient.mass.tangent_norm(correction).unwrap(),
            solved.residual_dual_norm().unwrap(),
            2.0e-9,
        );
        for row in 0..3 {
            let reconstructed: f64 = (0..3)
                .map(|column| redundant_jacobian()[row][column] * correction.components()[column])
                .sum();
            close(reconstructed, residual.components()[row], 2.0e-9);
        }
    }

    #[test]
    fn null_multiplier_shifts_preserve_quotient_norm_and_generalized_force() {
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            redundant_jacobian(),
            &residual_metric(),
            quotient_profile(),
        )
        .unwrap();
        let lambda =
            ConstraintMultiplierV1::<OldConstraintSpace, 3>::new([0.5, -0.75, 1.25]).unwrap();
        let shifted =
            ConstraintMultiplierV1::<OldConstraintSpace, 3>::new([3.5, -0.75, -1.75]).unwrap();
        close(
            quotient.multiplier_quotient_norm(&lambda).unwrap(),
            quotient.multiplier_quotient_norm(&shifted).unwrap(),
            2.0e-12,
        );
        for dof in 0..3 {
            let original: f64 = (0..3)
                .map(|row| redundant_jacobian()[row][dof] * lambda.components()[row])
                .sum();
            let shifted_force: f64 = (0..3)
                .map(|row| redundant_jacobian()[row][dof] * shifted.components()[row])
                .sum();
            close(original, shifted_force, 2.0e-13);
        }
    }

    #[test]
    fn incompatible_residual_is_never_silently_projected() {
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            redundant_jacobian(),
            &residual_metric(),
            quotient_profile(),
        )
        .unwrap();
        let residual =
            ConstraintResidualV1::<OldConstraintSpace, 3>::new([0.4, -0.7, 0.41]).unwrap();
        let solved = quotient.solve_residual(&residual).unwrap();
        assert_eq!(solved.status(), ConstraintResidualStatusV1::Incompatible);
        assert!(solved.minimum_kinetic_correction().is_none());
        assert!(solved.residual_dual_norm().is_none());
    }

    #[test]
    fn constraint_equation_rebasis_preserves_quotient_norms_and_correction() {
        let mass = mass();
        let h_old = residual_metric::<OldConstraintSpace>();
        let j_old = redundant_jacobian();
        let residual_old = compatible_residual();
        let q_old = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass,
            j_old,
            &h_old,
            quotient_profile(),
        )
        .unwrap();
        let solved_old = q_old
            .solve_residual(
                &ConstraintResidualV1::<OldConstraintSpace, 3>::new(residual_old).unwrap(),
            )
            .unwrap();
        assert_eq!(solved_old.status(), ConstraintResidualStatusV1::Compatible);

        // r_new = T r_old. Rebasis stores S=T^-1 because r_old = S r_new.
        let s = [[1.0, -2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let rebasis =
            crate::Rebasis::<OldConstraintSpace, NewConstraintSpace, 3>::new(s, metric_profile())
                .unwrap();
        let h_new = rebasis.metric_to_to(&h_old, metric_profile()).unwrap();
        let mut j_new = [[0.0; 3]; 3];
        for column in 0..3 {
            j_new[0][column] = j_old[0][column] + 2.0 * j_old[1][column];
            j_new[1][column] = j_old[1][column];
            j_new[2][column] = j_old[2][column];
        }
        let residual_new = [
            residual_old[0] + 2.0 * residual_old[1],
            residual_old[1],
            residual_old[2],
        ];
        let q_new = RankDeficientDelassusQuotientV1::<Cartesian, NewConstraintSpace, 3, 3>::new(
            &mass,
            j_new,
            &h_new,
            quotient_profile(),
        )
        .unwrap();
        let solved_new = q_new
            .solve_residual(
                &ConstraintResidualV1::<NewConstraintSpace, 3>::new(residual_new).unwrap(),
            )
            .unwrap();
        assert_eq!(solved_new.status(), ConstraintResidualStatusV1::Compatible);
        close(
            solved_old.residual_dual_norm().unwrap(),
            solved_new.residual_dual_norm().unwrap(),
            2.0e-9,
        );
        for dof in 0..3 {
            close(
                solved_old
                    .minimum_kinetic_correction()
                    .unwrap()
                    .components()[dof],
                solved_new
                    .minimum_kinetic_correction()
                    .unwrap()
                    .components()[dof],
                2.0e-9,
            );
        }
    }

    #[test]
    fn exact_rational_delassus_fixture_matches_independent_oracle() {
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            redundant_jacobian(),
            &residual_metric(),
            quotient_profile(),
        )
        .unwrap();
        let expected = [
            [7.0 / 10.0, -1.0 / 5.0, 7.0 / 10.0],
            [-1.0 / 5.0, 23.0 / 15.0, -1.0 / 5.0],
            [7.0 / 10.0, -1.0 / 5.0, 7.0 / 10.0],
        ];
        for row in 0..3 {
            for column in 0..3 {
                close(
                    quotient.matrix()[row][column],
                    expected[row][column],
                    2.0e-13,
                );
            }
        }

        // Exact rational solution class: (71/155, -123/310, 0) plus
        // arbitrary multiples of the null multiplier (1, 0, -1).
        let exact_representative = ConstraintMultiplierV1::<OldConstraintSpace, 3>::new([
            71.0 / 155.0,
            -123.0 / 310.0,
            0.0,
        ])
        .unwrap();
        let residual =
            ConstraintResidualV1::<OldConstraintSpace, 3>::new([0.4, -0.7, 0.4]).unwrap();
        let dual_pairing = crate::dot(exact_representative.components(), residual.components());
        close(dual_pairing, 1429.0 / 3100.0, 2.0e-13);

        let solved = quotient.solve_residual(&residual).unwrap();
        assert_eq!(solved.status(), ConstraintResidualStatusV1::Compatible);
        close(
            solved.residual_dual_norm().unwrap().powi(2),
            1429.0 / 3100.0,
            2.0e-9,
        );
    }

    #[test]
    fn multiplier_quotient_norm_is_invariant_under_constraint_basis_change() {
        let h_old = residual_metric::<OldConstraintSpace>();
        let q_old = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            redundant_jacobian(),
            &h_old,
            quotient_profile(),
        )
        .unwrap();
        let s = [[1.0, -2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let rebasis =
            Rebasis::<OldConstraintSpace, NewConstraintSpace, 3>::new(s, metric_profile()).unwrap();
        let h_new = rebasis.metric_to_to(&h_old, metric_profile()).unwrap();
        let j_old = redundant_jacobian();
        let mut j_new = [[0.0; 3]; 3];
        for column in 0..3 {
            j_new[0][column] = j_old[0][column] + 2.0 * j_old[1][column];
            j_new[1][column] = j_old[1][column];
            j_new[2][column] = j_old[2][column];
        }
        let q_new = RankDeficientDelassusQuotientV1::<Cartesian, NewConstraintSpace, 3, 3>::new(
            &mass(),
            j_new,
            &h_new,
            quotient_profile(),
        )
        .unwrap();

        let lambda_old =
            ConstraintMultiplierV1::<OldConstraintSpace, 3>::new([0.5, -0.75, 1.25]).unwrap();
        let lambda_covector =
            Cotangent::<OldConstraintSpace, 3>::new(*lambda_old.components()).unwrap();
        let lambda_new_components = rebasis.cotangent_to_to(&lambda_covector).unwrap();
        let lambda_new = ConstraintMultiplierV1::<NewConstraintSpace, 3>::new(
            *lambda_new_components.components(),
        )
        .unwrap();
        close(
            q_old.multiplier_quotient_norm(&lambda_old).unwrap(),
            q_new.multiplier_quotient_norm(&lambda_new).unwrap(),
            2.0e-11,
        );
    }

    #[test]
    fn generalized_and_constraint_rebases_commute_for_redundant_constraints() {
        let mass_q = mass();
        let h_old = residual_metric::<OldConstraintSpace>();
        let j_q = redundant_jacobian();
        let residual_q = compatible_residual();
        let quotient_q =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
                &mass_q,
                j_q,
                &h_old,
                quotient_profile(),
            )
            .unwrap();
        let solved_q = quotient_q
            .solve_residual(
                &ConstraintResidualV1::<OldConstraintSpace, 3>::new(residual_q).unwrap(),
            )
            .unwrap();
        assert_eq!(solved_q.status(), ConstraintResidualStatusV1::Compatible);

        // Independent generalized-coordinate rebase q = S y.
        let generalized_rebasis = Rebasis::<Cartesian, DenseFrame, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let mass_y = generalized_rebasis
            .metric_to_to(&mass_q, metric_profile())
            .unwrap();
        let j_y = crate::constraint_metric::rebase_constraint_jacobian(&j_q, &generalized_rebasis)
            .unwrap();

        // Independent constraint-equation rebase r_new = T r_old.
        let constraint_s = [[1.0, -2.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let constraint_rebasis = Rebasis::<OldConstraintSpace, NewConstraintSpace, 3>::new(
            constraint_s,
            metric_profile(),
        )
        .unwrap();
        let h_new = constraint_rebasis
            .metric_to_to(&h_old, metric_profile())
            .unwrap();
        let mut j_y_new = [[0.0; 3]; 3];
        for column in 0..3 {
            j_y_new[0][column] = j_y[0][column] + 2.0 * j_y[1][column];
            j_y_new[1][column] = j_y[1][column];
            j_y_new[2][column] = j_y[2][column];
        }
        let residual_new = [
            residual_q[0] + 2.0 * residual_q[1],
            residual_q[1],
            residual_q[2],
        ];

        let quotient_y_new =
            RankDeficientDelassusQuotientV1::<DenseFrame, NewConstraintSpace, 3, 3>::new(
                &mass_y,
                j_y_new,
                &h_new,
                quotient_profile(),
            )
            .unwrap();
        let solved_y_new = quotient_y_new
            .solve_residual(
                &ConstraintResidualV1::<NewConstraintSpace, 3>::new(residual_new).unwrap(),
            )
            .unwrap();
        assert_eq!(
            solved_y_new.status(),
            ConstraintResidualStatusV1::Compatible
        );
        assert_eq!(solved_y_new.numerical_rank(), solved_q.numerical_rank());
        close(
            solved_y_new.residual_dual_norm().unwrap(),
            solved_q.residual_dual_norm().unwrap(),
            3.0e-9,
        );

        let correction_y_expected = generalized_rebasis
            .tangent_to_to(solved_q.minimum_kinetic_correction().unwrap())
            .unwrap();
        for dof in 0..3 {
            close(
                solved_y_new
                    .minimum_kinetic_correction()
                    .unwrap()
                    .components()[dof],
                correction_y_expected.components()[dof],
                3.0e-9,
            );
        }
    }

    #[test]
    fn rank_and_compatibility_ladder_is_invariant_under_metric_aware_row_shear() {
        let profile = ConstraintQuotientProfileV1 {
            relative_rank_tolerance: 1.0e-10,
            rank_uncertainty_factor: 10.0,
            relative_compatibility_tolerance: 1.0e-10,
            relative_reduced_pivot_tolerance: 1.0e-12,
        };
        let t = [[1.0, 8.0, 0.0], [0.0, 1.0, 4.0], [0.0, 0.0, 1.0]];
        let s = [[1.0, -8.0, 32.0], [0.0, 1.0, -4.0], [0.0, 0.0, 1.0]];
        let row_rebasis =
            Rebasis::<OldConstraintSpace, NewConstraintSpace, 3>::new(s, metric_profile())
                .unwrap();
        let h_old = residual_metric::<OldConstraintSpace>();
        let h_new = row_rebasis.metric_to_to(&h_old, metric_profile()).unwrap();
        let x = [0.1, -0.2, 0.3];

        for epsilon in [1.0e-8, 1.0e-9, 1.0e-10, 1.0e-11, 1.0e-12, 0.0] {
            let j_old = [
                [1.0, 0.0, 1.0],
                [0.0, 2.0, -1.0],
                [1.0, epsilon, 1.0],
            ];
            let j_new: [[f64; 3]; 3] = std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    (0..3).map(|inner| t[row][inner] * j_old[inner][column]).sum()
                })
            });
            let residual_old = std::array::from_fn(|row| {
                (0..3).map(|column| j_old[row][column] * x[column]).sum()
            });
            let residual_new = row_rebasis
                .tangent_to_to(
                    &Tangent::<OldConstraintSpace, 3>::new(residual_old).unwrap(),
                )
                .unwrap();

            let old_quotient = RankDeficientDelassusQuotientV1::<
                Cartesian,
                OldConstraintSpace,
                3,
                3,
            >::new(&mass(), j_old, &h_old, profile)
            .unwrap();
            let new_quotient = RankDeficientDelassusQuotientV1::<
                Cartesian,
                NewConstraintSpace,
                3,
                3,
            >::new(&mass(), j_new, &h_new, profile)
            .unwrap();
            let old_result = old_quotient
                .solve_residual(&ConstraintResidualV1::<OldConstraintSpace, 3>::new(residual_old).unwrap())
                .unwrap();
            let new_result = new_quotient
                .solve_residual(
                    &ConstraintResidualV1::<NewConstraintSpace, 3>::new(
                        *residual_new.components(),
                    )
                    .unwrap(),
                )
                .unwrap();

            assert_eq!(old_result.status(), new_result.status(), "epsilon={epsilon:e}");
            assert_eq!(
                old_result.numerical_rank(),
                new_result.numerical_rank(),
                "epsilon={epsilon:e}"
            );
            assert_eq!(
                old_result.multiplier_nullity(),
                new_result.multiplier_nullity(),
                "epsilon={epsilon:e}"
            );
            close(
                old_result.compatibility_relative_defect(),
                new_result.compatibility_relative_defect(),
                2.0e-8,
            );
            if let (Some(old_norm), Some(new_norm)) = (
                old_result.residual_dual_norm(),
                new_result.residual_dual_norm(),
            ) {
                close(old_norm, new_norm, 2.0e-8);
                let old_correction = old_result.minimum_kinetic_correction().unwrap();
                let new_correction = new_result.minimum_kinetic_correction().unwrap();
                for dof in 0..3 {
                    close(
                        old_correction.components()[dof],
                        new_correction.components()[dof],
                        2.0e-8,
                    );
                }
            } else {
                assert_eq!(old_result.status(), ConstraintResidualStatusV1::Inconclusive);
            }
        }

        // For an exactly duplicated first/third row, z=(1,0,-1) lies in
        // ker(J^T). Adding it to a realizable residual makes the residual
        // incompatible; the same verdict must survive the row-basis change.
        let j_old = redundant_jacobian();
        let base = compatible_residual();
        let incompatible_old = std::array::from_fn(|row| {
            base[row] + 0.1 * [1.0, 0.0, -1.0][row]
        });
        let incompatible_new = row_rebasis
            .tangent_to_to(
                &Tangent::<OldConstraintSpace, 3>::new(incompatible_old).unwrap(),
            )
            .unwrap();
        let old_quotient = RankDeficientDelassusQuotientV1::<
            Cartesian,
            OldConstraintSpace,
            3,
            3,
        >::new(&mass(), j_old, &h_old, profile)
        .unwrap();
        let j_new: [[f64; 3]; 3] = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                (0..3).map(|inner| t[row][inner] * j_old[inner][column]).sum()
            })
        });
        let new_quotient = RankDeficientDelassusQuotientV1::<
            Cartesian,
            NewConstraintSpace,
            3,
            3,
        >::new(&mass(), j_new, &h_new, profile)
        .unwrap();
        let old_result = old_quotient
            .solve_residual(
                &ConstraintResidualV1::<OldConstraintSpace, 3>::new(incompatible_old).unwrap(),
            )
            .unwrap();
        let new_result = new_quotient
            .solve_residual(
                &ConstraintResidualV1::<NewConstraintSpace, 3>::new(
                    *incompatible_new.components(),
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(old_result.status(), ConstraintResidualStatusV1::Incompatible);
        assert_eq!(new_result.status(), ConstraintResidualStatusV1::Incompatible);
        assert!(old_result.minimum_kinetic_correction().is_none());
        assert!(new_result.minimum_kinetic_correction().is_none());
    }

    #[test]
    fn rank_threshold_uncertainty_band_forces_inconclusive_without_correction() {
        let profile = ConstraintQuotientProfileV1 {
            relative_rank_tolerance: 1.0e-8,
            rank_uncertainty_factor: 10.0,
            relative_compatibility_tolerance: 1.0e-9,
            relative_reduced_pivot_tolerance: 1.0e-12,
        };
        let near_dependent = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 5.0e-8, 1.0]];
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            near_dependent,
            &residual_metric(),
            profile,
        )
        .unwrap();
        let receipt = quotient.receipt();
        assert_eq!(receipt.numerical_rank, 2);
        assert!(!receipt.rank_resolved);
        assert!(receipt.unresolved_relative_pivot.is_some());

        let residual =
            ConstraintResidualV1::<OldConstraintSpace, 3>::new(std::array::from_fn(|row| {
                (0..3)
                    .map(|column| near_dependent[row][column] * [0.1, -0.2, 0.3][column])
                    .sum()
            }))
            .unwrap();
        let outcome = quotient.solve_residual(&residual).unwrap();
        assert_eq!(outcome.status(), ConstraintResidualStatusV1::Inconclusive);
        assert!(outcome.minimum_kinetic_correction().is_none());
        assert!(outcome.multiplier_representative().is_none());
    }

    #[test]
    fn well_separated_small_independent_row_is_admitted_as_full_rank() {
        let profile = ConstraintQuotientProfileV1 {
            relative_rank_tolerance: 1.0e-8,
            rank_uncertainty_factor: 10.0,
            relative_compatibility_tolerance: 1.0e-9,
            relative_reduced_pivot_tolerance: 1.0e-12,
        };
        let full_rank = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0], [1.0, 1.0e-4, 1.0]];
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
            &mass(),
            full_rank,
            &residual_metric(),
            profile,
        )
        .unwrap();
        assert_eq!(quotient.rank(), 3);
        assert!(quotient.receipt().rank_resolved);
        assert_eq!(quotient.nullity(), 0);
    }

    #[test]
    fn rank_zero_jacobian_accepts_only_zero_residual() {
        let zero_j = [[0.0; 3]; 2];
        let h = Metric::<OldConstraintSpace, 2>::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile())
            .unwrap();
        let quotient = RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 2>::new(
            &mass(),
            zero_j,
            &h,
            quotient_profile(),
        )
        .unwrap();
        assert_eq!(quotient.rank(), 0);
        let zero = ConstraintResidualV1::<OldConstraintSpace, 2>::new([0.0, 0.0]).unwrap();
        assert_eq!(
            quotient.solve_residual(&zero).unwrap().status(),
            ConstraintResidualStatusV1::Compatible
        );
        let nonzero = ConstraintResidualV1::<OldConstraintSpace, 2>::new([0.0, 1.0]).unwrap();
        assert_eq!(
            quotient.solve_residual(&nonzero).unwrap().status(),
            ConstraintResidualStatusV1::Incompatible
        );
    }
}
