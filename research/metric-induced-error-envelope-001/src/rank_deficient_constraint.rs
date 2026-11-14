// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Quotient geometry for redundant linear holonomic constraints.
//!
//! W = J M^-1 J^T is positive semidefinite when constraint rows are redundant.
//! Multipliers are equivalent modulo ker(J^T); physical generalized force and
//! minimum-kinetic correction are unique even when a multiplier vector is not.
//!
//! Floating-point rank and residual compatibility are measured in an explicit
//! SPD residual-space metric H. Under r_new = T r_old, callers must provide
//! H_new = T^-T H_old T^-1. This prevents row units from changing acceptance.

use core::marker::PhantomData;

use crate::{
    CoordinateFrame, Cotangent, Metric, MetricErrorV1, MetricProfileV1, Tangent,
};
use super::{ConstraintMultiplierV1, ConstraintResidualV1};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintQuotientProfileV1 {
    pub relative_rank_tolerance: f64,
    pub relative_compatibility_tolerance: f64,
    pub relative_reduced_pivot_tolerance: f64,
}

impl Default for ConstraintQuotientProfileV1 {
    fn default() -> Self {
        Self {
            relative_rank_tolerance: 1.0e-10,
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
    pub maximum_residual_basis_orthogonality_defect: f64,
    pub minimum_reduced_cholesky_pivot: Option<f64>,
    pub residual_metric: MetricAdmissionSummaryV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstraintQuotientErrorV1 {
    ZeroDimension,
    InvalidProfile,
    NonFiniteJacobian,
    NonFiniteIntermediate,
    ReducedOperatorNotPositiveDefinite { pivot: usize },
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

fn validate_profile(
    profile: ConstraintQuotientProfileV1,
) -> Result<(), ConstraintQuotientErrorV1> {
    if !profile.relative_rank_tolerance.is_finite()
        || profile.relative_rank_tolerance <= 0.0
        || profile.relative_rank_tolerance >= 1.0
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

fn h_inner<Space: CoordinateFrame, const K: usize>(
    metric: &Metric<Space, K>,
    left: &[f64; K],
    right: &[f64; K],
) -> Result<f64, ConstraintQuotientErrorV1> {
    let mut value = 0.0;
    for row in 0..K {
        for column in 0..K {
            value += left[row] * metric.matrix()[row][column] * right[column];
        }
    }
    if value.is_finite() {
        Ok(value)
    } else {
        Err(ConstraintQuotientErrorV1::NonFiniteIntermediate)
    }
}

fn h_norm<Space: CoordinateFrame, const K: usize>(
    metric: &Metric<Space, K>,
    vector: &[f64; K],
) -> Result<f64, ConstraintQuotientErrorV1> {
    let squared = h_inner(metric, vector, vector)?;
    if squared < 0.0 {
        return Err(ConstraintQuotientErrorV1::NegativeQuotientQuadraticForm);
    }
    Ok(squared.sqrt())
}

fn build_h_orthonormal_range_basis<
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    jacobian: &[[f64; N]; K],
    residual_metric: &Metric<Space, K>,
    relative_rank_tolerance: f64,
) -> Result<([[f64; K]; K], usize, f64), ConstraintQuotientErrorV1> {
    // Columns of J span range(J)=range(W). Reveal that subspace using H,
    // so the rank profile is meaningful under a matching residual-metric
    // transformation H_new=T^-T H_old T^-1.
    let mut columns = [[0.0; K]; N];
    let mut scale = 0.0_f64;
    for dof in 0..N {
        for row in 0..K {
            columns[dof][row] = jacobian[row][dof];
        }
        scale = scale.max(h_norm(residual_metric, &columns[dof])?);
    }
    if !scale.is_finite() {
        return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
    }
    let threshold = relative_rank_tolerance * scale;
    if !threshold.is_finite() {
        return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
    }

    let mut basis = [[0.0; K]; K]; // basis[coordinate][axis]
    let mut rank = 0usize;
    for pivot_index in 0..N.min(K) {
        let mut selected = pivot_index;
        let mut selected_norm = h_norm(residual_metric, &columns[selected])?;
        for (candidate, column) in columns.iter().enumerate().skip(pivot_index + 1) {
            let candidate_norm = h_norm(residual_metric, column)?;
            if candidate_norm > selected_norm {
                selected = candidate;
                selected_norm = candidate_norm;
            }
        }
        columns.swap(pivot_index, selected);
        if selected_norm <= threshold {
            break;
        }

        let pivot = h_norm(residual_metric, &columns[pivot_index])?;
        if pivot <= threshold || !pivot.is_finite() {
            break;
        }
        for coordinate in 0..K {
            basis[coordinate][rank] = columns[pivot_index][coordinate] / pivot;
        }

        let basis_column = std::array::from_fn(|coordinate| basis[coordinate][rank]);
        for candidate in (pivot_index + 1)..N {
            // Two reorthogonalization passes.
            for _ in 0..2 {
                let projection = h_inner(residual_metric, &basis_column, &columns[candidate])?;
                for coordinate in 0..K {
                    columns[candidate][coordinate] -= projection * basis_column[coordinate];
                }
            }
        }
        rank += 1;
    }

    let mut maximum_defect = 0.0_f64;
    for left_axis in 0..rank {
        for right_axis in 0..rank {
            let left = std::array::from_fn(|coordinate| basis[coordinate][left_axis]);
            let right = std::array::from_fn(|coordinate| basis[coordinate][right_axis]);
            let actual = h_inner(residual_metric, &left, &right)?;
            let expected = if left_axis == right_axis { 1.0 } else { 0.0 };
            maximum_defect = maximum_defect.max((actual - expected).abs());
        }
    }
    Ok((basis, rank, maximum_defect))
}

fn cholesky_reduced<const K: usize>(
    matrix: &[[f64; K]; K],
    rank: usize,
    relative_pivot_tolerance: f64,
) -> Result<([[f64; K]; K], Option<f64>), ConstraintQuotientErrorV1> {
    if rank == 0 {
        return Ok(([[0.0; K]; K], None));
    }
    let scale = (0..rank)
        .map(|index| matrix[index][index].abs())
        .fold(0.0_f64, f64::max);
    let threshold = relative_pivot_tolerance * scale;
    if !scale.is_finite() || scale == 0.0 || !threshold.is_finite() {
        return Err(ConstraintQuotientErrorV1::ReducedOperatorNotPositiveDefinite {
            pivot: 0,
        });
    }

    let mut lower = [[0.0; K]; K];
    let mut minimum_pivot = f64::INFINITY;
    for row in 0..rank {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for inner in 0..column {
                value -= lower[row][inner] * lower[column][inner];
            }
            if !value.is_finite() {
                return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
            }
            if row == column {
                if value <= threshold {
                    return Err(ConstraintQuotientErrorV1::ReducedOperatorNotPositiveDefinite {
                        pivot: row,
                    });
                }
                lower[row][column] = value.sqrt();
                minimum_pivot = minimum_pivot.min(value);
            } else {
                lower[row][column] = value / lower[column][column];
            }
            if !lower[row][column].is_finite() {
                return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
            }
        }
    }
    Ok((lower, Some(minimum_pivot)))
}

fn solve_reduced<const K: usize>(
    lower: &[[f64; K]; K],
    rank: usize,
    rhs: &[f64; K],
) -> Result<[f64; K], ConstraintQuotientErrorV1> {
    let mut intermediate = [0.0; K];
    for row in 0..rank {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * intermediate[column];
        }
        intermediate[row] = value / lower[row][row];
    }

    let mut solution = [0.0; K];
    for row in (0..rank).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..rank {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }
    }
    Ok(solution)
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

/// Rank-aware Delassus quotient for W = J M^-1 J^T.
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
    range_basis: [[f64; K]; K],
    dual_range_basis: [[f64; K]; K],
    reduced_cholesky: [[f64; K]; K],
    rank: usize,
    profile: ConstraintQuotientProfileV1,
    receipt: ConstraintQuotientAdmissionReceiptV1,
    _spaces: PhantomData<(Frame, Space)>,
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

        // W = J M^-1 J^T using Cholesky-whitened constraint covectors.
        let mut whitened = [[0.0; N]; K];
        for row in 0..K {
            let covector = Cotangent::<Frame, N>::new(jacobian[row])?;
            whitened[row] = mass.whiten_cotangent(&covector)?;
        }
        let mut delassus = [[0.0; K]; K];
        for row in 0..K {
            for column in row..K {
                let value = crate::dot(&whitened[row], &whitened[column]);
                if !value.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                delassus[row][column] = value;
                delassus[column][row] = value;
            }
        }

        let (range_basis, rank, orthogonality_defect) =
            build_h_orthonormal_range_basis(
                &jacobian,
                residual_metric,
                profile.relative_rank_tolerance,
            )?;

        // H U is the dual basis. In these coordinates the reduced quotient
        // operator K_r = U^T H W H U is positive definite on range(W).
        let mut dual_range_basis = [[0.0; K]; K];
        for axis in 0..rank {
            for row in 0..K {
                for column in 0..K {
                    dual_range_basis[row][axis] +=
                        residual_metric.matrix()[row][column] * range_basis[column][axis];
                }
                if !dual_range_basis[row][axis].is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
            }
        }

        let mut reduced = [[0.0; K]; K];
        for left_axis in 0..rank {
            let left = std::array::from_fn(|coordinate| dual_range_basis[coordinate][left_axis]);
            for right_axis in left_axis..rank {
                let right = std::array::from_fn(|coordinate| dual_range_basis[coordinate][right_axis]);
                let mapped = mat_vec(&delassus, &right)?;
                let value = crate::dot(&left, &mapped);
                if !value.is_finite() {
                    return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
                }
                reduced[left_axis][right_axis] = value;
                reduced[right_axis][left_axis] = value;
            }
        }

        let (reduced_cholesky, minimum_pivot) = cholesky_reduced(
            &reduced,
            rank,
            profile.relative_reduced_pivot_tolerance,
        )?;

        Ok(Self {
            mass: *mass,
            jacobian,
            delassus,
            residual_metric: *residual_metric,
            range_basis,
            dual_range_basis,
            reduced_cholesky,
            rank,
            profile,
            receipt: ConstraintQuotientAdmissionReceiptV1 {
                numerical_rank: rank,
                multiplier_nullity: K - rank,
                maximum_residual_basis_orthogonality_defect: orthogonality_defect,
                minimum_reduced_cholesky_pivot: minimum_pivot,
                residual_metric: residual_metric.admission_receipt().into(),
            },
            _spaces: PhantomData,
        })
    }

    pub fn matrix(&self) -> &[[f64; K]; K] {
        &self.delassus
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
        let scale = crate::vector_euclidean(multiplier.components())
            * crate::vector_euclidean(&mapped);
        if !squared.is_finite() || squared < -self.profile.relative_rank_tolerance * scale {
            return Err(ConstraintQuotientErrorV1::NegativeQuotientQuadraticForm);
        }
        Ok(squared.max(0.0).sqrt())
    }

    /// Solve W lambda = r on the quotient. Incompatible residuals are returned
    /// explicitly; the method never silently projects and reports success.
    pub fn solve_residual(
        &self,
        residual: &ConstraintResidualV1<Space, K>,
    ) -> Result<ConstraintQuotientSolveReceiptV1<Frame, Space, N, K>, ConstraintQuotientErrorV1>
    {
        let residual_components = *residual.components();
        let residual_norm = h_norm(&self.residual_metric, &residual_components)?;

        // H-orthogonal projection onto range(W)=range(J).
        let mut coefficients = [0.0; K];
        for axis in 0..self.rank {
            let basis_column = std::array::from_fn(|coordinate| self.range_basis[coordinate][axis]);
            coefficients[axis] =
                h_inner(&self.residual_metric, &basis_column, &residual_components)?;
        }
        let mut projected = [0.0; K];
        for row in 0..K {
            for axis in 0..self.rank {
                projected[row] += self.range_basis[row][axis] * coefficients[axis];
            }
        }
        let defect = std::array::from_fn(|row| residual_components[row] - projected[row]);
        let defect_norm = h_norm(&self.residual_metric, &defect)?;
        let compatibility_relative_defect = if residual_norm == 0.0 {
            defect_norm
        } else {
            defect_norm / residual_norm
        };
        if !compatibility_relative_defect.is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
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

        // Solve K_r mu = U^T H r, where K_r = U^T H W H U.
        // Choose lambda = H U mu. Other solutions differ by ker(W).
        let mu = solve_reduced(&self.reduced_cholesky, self.rank, &coefficients)?;
        let mut multiplier_components = [0.0; K];
        for row in 0..K {
            for axis in 0..self.rank {
                multiplier_components[row] += self.dual_range_basis[row][axis] * mu[axis];
            }
        }
        let multiplier = ConstraintMultiplierV1::<Space, K>::new(multiplier_components)?;

        let mapped = mat_vec(&self.delassus, &multiplier_components)?;
        let equation_defect = std::array::from_fn(|row| mapped[row] - projected[row]);
        let equation_defect_norm = h_norm(&self.residual_metric, &equation_defect)?;
        let reduced_solve_relative_defect = if residual_norm == 0.0 {
            equation_defect_norm
        } else {
            equation_defect_norm / residual_norm
        };
        if !reduced_solve_relative_defect.is_finite() {
            return Err(ConstraintQuotientErrorV1::NonFiniteIntermediate);
        }
        if reduced_solve_relative_defect > tolerance {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                reduced_solve_relative_defect,
            ));
        }

        let dual_pairing = crate::dot(&multiplier_components, &residual_components);
        if !dual_pairing.is_finite()
            || dual_pairing < -tolerance * residual_norm.max(1.0)
        {
            return Err(ConstraintQuotientErrorV1::NegativeQuotientQuadraticForm);
        }
        let residual_dual_norm = dual_pairing.max(0.0).sqrt();

        let mut generalized_force = [0.0; N];
        for row in 0..K {
            for dof in 0..N {
                generalized_force[dof] += self.jacobian[row][dof] * multiplier_components[row];
            }
        }
        let correction = self.mass.raise(&Cotangent::<Frame, N>::new(generalized_force)?)?;
        let kinetic_norm = self.mass.tangent_norm(&correction)?;

        let mut reconstructed = [0.0; K];
        for row in 0..K {
            for dof in 0..N {
                reconstructed[row] += self.jacobian[row][dof] * correction.components()[dof];
            }
        }
        let reconstruction_defect =
            std::array::from_fn(|row| reconstructed[row] - residual_components[row]);
        let reconstruction_norm = h_norm(&self.residual_metric, &reconstruction_defect)?;
        let reconstruction_relative_defect = if residual_norm == 0.0 {
            reconstruction_norm
        } else {
            reconstruction_norm / residual_norm
        };
        if reconstruction_relative_defect > tolerance {
            return Ok(self.empty_receipt(
                ConstraintResidualStatusV1::Inconclusive,
                compatibility_relative_defect,
                reconstruction_relative_defect,
            ));
        }

        Ok(ConstraintQuotientSolveReceiptV1 {
            status: ConstraintResidualStatusV1::Compatible,
            numerical_rank: self.rank,
            multiplier_nullity: K - self.rank,
            compatibility_relative_defect,
            reduced_solve_relative_defect: reduced_solve_relative_defect
                .max(reconstruction_relative_defect),
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

    fn metric_profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }
    fn quotient_profile() -> ConstraintQuotientProfileV1 {
        ConstraintQuotientProfileV1 {
            relative_rank_tolerance: 1.0e-10,
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
            (0..3).map(|column| jacobian[row][column] * velocity[column]).sum()
        })
    }
    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!((left - right).abs() <= tolerance * scale, "{left} != {right}");
    }

    #[test]
    fn duplicated_rows_create_multiplier_quotient_but_unique_physical_correction() {
        let quotient =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
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
        let quotient =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
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
        let quotient =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
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
        let q_old =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 3>::new(
                &mass, j_old, &h_old, quotient_profile(),
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
        let q_new =
            RankDeficientDelassusQuotientV1::<Cartesian, NewConstraintSpace, 3, 3>::new(
                &mass, j_new, &h_new, quotient_profile(),
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
                solved_old.minimum_kinetic_correction().unwrap().components()[dof],
                solved_new.minimum_kinetic_correction().unwrap().components()[dof],
                2.0e-9,
            );
        }
    }

    #[test]
    fn rank_zero_jacobian_accepts_only_zero_residual() {
        let zero_j = [[0.0; 3]; 2];
        let h = Metric::<OldConstraintSpace, 2>::admit(
            [[1.0, 0.0], [0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let quotient =
            RankDeficientDelassusQuotientV1::<Cartesian, OldConstraintSpace, 3, 2>::new(
                &mass(), zero_j, &h, quotient_profile(),
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
