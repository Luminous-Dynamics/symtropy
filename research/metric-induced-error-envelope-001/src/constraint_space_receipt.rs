// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-aware rectangular constraint-map and Delassus evidence.
//!
//! This module composes three geometric objects:
//! - the mass metric M on generalized-velocity tangents;
//! - the constraint-rate metric C on constraint-space tangents;
//! - the Delassus cometric W = J M^-1 J^T from constraint covectors to
//!   constraint tangents.
//!
//! The rectangular tangent map is whitened as B = L_C^T J L_M^-T.
//! Consequently B B^T = L_C^T W L_C, so the squared singular spectrum of B
//! is the eigenvalue spectrum of the constraint-metric-whitened Delassus
//! operator. No explicit metric inverse or normal-equation SVD is formed.
//!
//! A positive-definite Delassus solve is exposed only when J is numerically
//! full row rank and the nonzero-spectrum condition is inside the profile.
//! Rank-deficient cases retain their rectangular receipt but do not receive
//! multiplier-solve authority; they belong to the quotient solver.

use core::marker::PhantomData;

use crate::constraint_metric::{
    ConstraintMultiplierV1, ConstraintResidualV1, DelassusMetricV1, rebase_constraint_jacobian,
};
use crate::operator::{OperatorConditionErrorV1, OperatorConditionProfileV1};
use crate::rectangular_operator::{
    RectangularOperatorConditionReceiptV1, RectangularOperatorStatusV1,
    RectangularTangentToTangentOperator,
};
use crate::{CoordinateFrame, Cotangent, Metric, MetricErrorV1, MetricProfileV1, Rebasis, Tangent};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintSpaceProfileV1 {
    pub operator: OperatorConditionProfileV1,
    pub maximum_matrix_entries: usize,
    pub maximum_work_units: usize,
}

impl Default for ConstraintSpaceProfileV1 {
    fn default() -> Self {
        Self {
            operator: OperatorConditionProfileV1::default(),
            maximum_matrix_entries: 16_384,
            maximum_work_units: 100_000_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSpaceReceiptErrorV1 {
    Metric(MetricErrorV1),
    Operator(OperatorConditionErrorV1),
    ArithmeticOverflow,
    MatrixEntryBudgetExceeded,
    WorkBudgetExceeded,
    FullRowRankAuthorityUnavailable,
}

impl From<MetricErrorV1> for ConstraintSpaceReceiptErrorV1 {
    fn from(value: MetricErrorV1) -> Self {
        Self::Metric(value)
    }
}

impl From<OperatorConditionErrorV1> for ConstraintSpaceReceiptErrorV1 {
    fn from(value: OperatorConditionErrorV1) -> Self {
        Self::Operator(value)
    }
}

impl core::fmt::Display for ConstraintSpaceReceiptErrorV1 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ConstraintSpaceReceiptErrorV1 {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintMapRankStatusV1 {
    FullRowRank,
    FullColumnRankNotSurjective,
    RankDeficient,
    OutsideConditionEnvelope,
}

/// A physical constraint-space receipt for J: T_D -> T_C.
///
/// The receipt retains the exact mass/constraint metrics and Jacobian used to
/// build both the metric-aware rectangular spectrum and the Delassus operator.
/// The Delassus operator is absent for rank-deficient or out-of-envelope maps.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintSpaceReceiptV1<
    Domain: CoordinateFrame,
    ConstraintSpace: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    mass_metric: Metric<Domain, N>,
    constraint_metric: Metric<ConstraintSpace, K>,
    jacobian: [[f64; N]; K],
    operator_receipt: RectangularOperatorConditionReceiptV1,
    rank_status: ConstraintMapRankStatusV1,
    singular_values: Vec<f64>,
    /// Eigenvalue estimates of L_C^T W L_C = B B^T. Explicit trailing zeros
    /// are appended for codomain nullity, making this vector length K.
    delassus_whitened_eigenvalue_estimates: Vec<f64>,
    delassus: Option<DelassusMetricV1<Domain, ConstraintSpace, N, K>>,
    _spaces: PhantomData<(Domain, ConstraintSpace)>,
}

fn preflight<const N: usize, const K: usize>(
    profile: ConstraintSpaceProfileV1,
) -> Result<(), ConstraintSpaceReceiptErrorV1> {
    if N == 0 || K == 0 {
        return Err(ConstraintSpaceReceiptErrorV1::Operator(
            OperatorConditionErrorV1::ZeroDimension,
        ));
    }
    if profile.maximum_matrix_entries == 0 || profile.maximum_work_units == 0 {
        return Err(ConstraintSpaceReceiptErrorV1::WorkBudgetExceeded);
    }
    let rectangular_entries = K
        .checked_mul(N)
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    let constraint_operator_entries = K
        .checked_mul(K)
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    let matrix_entries = rectangular_entries
        .checked_add(constraint_operator_entries)
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    if matrix_entries > profile.maximum_matrix_entries {
        return Err(ConstraintSpaceReceiptErrorV1::MatrixEntryBudgetExceeded);
    }

    // One-sided Jacobi rotates each pair of domain columns per sweep.
    // Include K^2*N for the Delassus Gram construction in the upper envelope.
    let pairs = N
        .checked_mul(N.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    let jacobi_work = pairs
        .checked_mul(K)
        .and_then(|value| value.checked_mul(profile.operator.maximum_jacobi_sweeps))
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    let delassus_work = K
        .checked_mul(K)
        .and_then(|value| value.checked_mul(N))
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    let required = jacobi_work
        .checked_add(delassus_work)
        .ok_or(ConstraintSpaceReceiptErrorV1::ArithmeticOverflow)?;
    if required > profile.maximum_work_units {
        return Err(ConstraintSpaceReceiptErrorV1::WorkBudgetExceeded);
    }
    Ok(())
}

impl<Domain: CoordinateFrame, ConstraintSpace: CoordinateFrame, const N: usize, const K: usize>
    ConstraintSpaceReceiptV1<Domain, ConstraintSpace, N, K>
{
    /// Qualify a rectangular constraint map under independent domain and
    /// constraint-rate metrics.
    pub fn new(
        mass_metric: Metric<Domain, N>,
        constraint_metric: Metric<ConstraintSpace, K>,
        jacobian: [[f64; N]; K],
        metric_profile: MetricProfileV1,
        profile: ConstraintSpaceProfileV1,
    ) -> Result<Self, ConstraintSpaceReceiptErrorV1> {
        preflight::<N, K>(profile)?;
        let rectangular =
            RectangularTangentToTangentOperator::<Domain, ConstraintSpace, K, N>::new(jacobian)?;
        let operator_receipt =
            rectangular.assess(&mass_metric, &constraint_metric, profile.operator)?;
        let singular_values = operator_receipt.singular_values.clone();

        let mut delassus_whitened_eigenvalue_estimates = Vec::with_capacity(K);
        for singular_value in &singular_values {
            let squared = singular_value * singular_value;
            if !squared.is_finite() {
                return Err(ConstraintSpaceReceiptErrorV1::Operator(
                    OperatorConditionErrorV1::NonFiniteIntermediate,
                ));
            }
            delassus_whitened_eigenvalue_estimates.push(squared);
        }
        delassus_whitened_eigenvalue_estimates.extend(std::iter::repeat_n(
            0.0,
            K.saturating_sub(singular_values.len()),
        ));

        let rank_status =
            if operator_receipt.status == RectangularOperatorStatusV1::OutsideConditionEnvelope {
                ConstraintMapRankStatusV1::OutsideConditionEnvelope
            } else if operator_receipt.numerical_rank == K {
                ConstraintMapRankStatusV1::FullRowRank
            } else if operator_receipt.numerical_rank == N && K > N {
                ConstraintMapRankStatusV1::FullColumnRankNotSurjective
            } else {
                ConstraintMapRankStatusV1::RankDeficient
            };

        // W is SPD only for a full-row-rank constraint map. Do not regularize
        // a singular Delassus matrix to manufacture a multiplier inverse.
        let delassus = if rank_status == ConstraintMapRankStatusV1::FullRowRank {
            Some(DelassusMetricV1::from_mass_jacobian(
                &mass_metric,
                jacobian,
                metric_profile,
            )?)
        } else {
            None
        };

        Ok(Self {
            mass_metric,
            constraint_metric,
            jacobian,
            operator_receipt,
            rank_status,
            singular_values,
            delassus_whitened_eigenvalue_estimates,
            delassus,
            _spaces: PhantomData,
        })
    }

    pub fn mass_metric(&self) -> &Metric<Domain, N> {
        &self.mass_metric
    }

    pub fn constraint_metric(&self) -> &Metric<ConstraintSpace, K> {
        &self.constraint_metric
    }

    pub fn jacobian(&self) -> &[[f64; N]; K] {
        &self.jacobian
    }

    pub fn operator_receipt(&self) -> &RectangularOperatorConditionReceiptV1 {
        &self.operator_receipt
    }

    pub fn rank_status(&self) -> ConstraintMapRankStatusV1 {
        self.rank_status
    }

    pub fn singular_values(&self) -> &[f64] {
        &self.singular_values
    }

    pub fn delassus_whitened_eigenvalue_estimates(&self) -> &[f64] {
        &self.delassus_whitened_eigenvalue_estimates
    }

    pub fn delassus(&self) -> Option<&DelassusMetricV1<Domain, ConstraintSpace, N, K>> {
        self.delassus.as_ref()
    }

    /// Apply J to a generalized-velocity tangent, returning constraint-rate
    /// tangent components.
    pub fn apply_jacobian(
        &self,
        velocity: &Tangent<Domain, N>,
    ) -> Result<Tangent<ConstraintSpace, K>, MetricErrorV1> {
        let mut rate = [0.0; K];
        for (row_index, row) in self.jacobian.iter().enumerate() {
            let value = row
                .iter()
                .zip(velocity.components().iter())
                .map(|(jacobian_value, velocity_value)| jacobian_value * velocity_value)
                .sum::<f64>();
            if !value.is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
            rate[row_index] = value;
        }
        Tangent::new(rate)
    }

    /// Constraint-rate tangent norm measured with C.
    pub fn constraint_rate_norm(
        &self,
        rate: &Tangent<ConstraintSpace, K>,
    ) -> Result<f64, MetricErrorV1> {
        self.constraint_metric.tangent_norm(rate)
    }

    /// Constraint-force covector norm measured with C^-1.
    pub fn constraint_covector_dual_norm(
        &self,
        covector: &Cotangent<ConstraintSpace, K>,
    ) -> Result<f64, MetricErrorV1> {
        self.constraint_metric.cotangent_norm(covector)
    }

    /// Delassus multiplier norm sqrt(lambda^T W lambda), equal to the
    /// mass-metric norm of the induced generalized-velocity correction.
    pub fn multiplier_kinetic_norm(
        &self,
        multiplier: &Cotangent<ConstraintSpace, K>,
    ) -> Result<f64, ConstraintSpaceReceiptErrorV1> {
        let delassus = self
            .delassus
            .as_ref()
            .ok_or(ConstraintSpaceReceiptErrorV1::FullRowRankAuthorityUnavailable)?;
        let typed = ConstraintMultiplierV1::<ConstraintSpace, K>::new(*multiplier.components())?;
        Ok(delassus.multiplier_norm(&typed)?)
    }

    /// Solve W lambda = r on the full-row-rank path and return both the
    /// multiplier covector and its minimum-kinetic-norm generalized correction.
    pub fn solve_minimum_kinetic_correction(
        &self,
        residual: &Tangent<ConstraintSpace, K>,
    ) -> Result<(Cotangent<ConstraintSpace, K>, Tangent<Domain, N>), ConstraintSpaceReceiptErrorV1>
    {
        let delassus = self
            .delassus
            .as_ref()
            .ok_or(ConstraintSpaceReceiptErrorV1::FullRowRankAuthorityUnavailable)?;
        let residual_typed =
            ConstraintResidualV1::<ConstraintSpace, K>::new(*residual.components())?;
        let multiplier = delassus.solve_residual(&residual_typed)?;
        let multiplier_covector = Cotangent::<ConstraintSpace, K>::new(*multiplier.components())?;
        let correction = delassus.induced_velocity_correction(&multiplier)?;
        Ok((multiplier_covector, correction))
    }

    /// Residual dual norm induced by the Delassus cometric W^-1.
    pub fn residual_delassus_dual_norm(
        &self,
        residual: &Tangent<ConstraintSpace, K>,
    ) -> Result<f64, ConstraintSpaceReceiptErrorV1> {
        let delassus = self
            .delassus
            .as_ref()
            .ok_or(ConstraintSpaceReceiptErrorV1::FullRowRankAuthorityUnavailable)?;
        let residual_typed =
            ConstraintResidualV1::<ConstraintSpace, K>::new(*residual.components())?;
        Ok(delassus.residual_norm(&residual_typed)?)
    }
}

/// Rebase a rectangular Jacobian under independent domain and constraint-rate
/// coordinate changes. For q = S y and c = T d:
/// J_d = T^-1 J_q S, M_y = S^T M_q S, and C_d = T^T C_c T.
pub fn rebase_constraint_jacobian_both<
    DomainFrom: CoordinateFrame,
    DomainTo: CoordinateFrame,
    ConstraintFrom: CoordinateFrame,
    ConstraintTo: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    jacobian_from: &[[f64; N]; K],
    domain_rebasis: &Rebasis<DomainFrom, DomainTo, N>,
    constraint_rebasis: &Rebasis<ConstraintFrom, ConstraintTo, K>,
) -> Result<[[f64; N]; K], MetricErrorV1> {
    let domain_rebased = rebase_constraint_jacobian(jacobian_from, domain_rebasis)?;
    let mut result = [[0.0; N]; K];
    for column in 0..N {
        let constraint_vector = Tangent::<ConstraintFrom, K>::new(std::array::from_fn(|row| {
            domain_rebased[row][column]
        }))?;
        let rebased = constraint_rebasis.tangent_to_to(&constraint_vector)?;
        for (row, value) in result.iter_mut().zip(rebased.components().iter()) {
            row[column] = *value;
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct CartesianQ;
    impl CoordinateFrame for CartesianQ {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct DenseY;
    impl CoordinateFrame for DenseY {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct ConstraintSpace;
    impl CoordinateFrame for ConstraintSpace {}

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct DenseConstraint;
    impl CoordinateFrame for DenseConstraint {}

    fn metric_profile() -> MetricProfileV1 {
        MetricProfileV1 {
            relative_pivot_tolerance: 1.0e-13,
            relative_symmetry_tolerance: 1.0e-12,
        }
    }

    fn constraint_profile() -> ConstraintSpaceProfileV1 {
        ConstraintSpaceProfileV1 {
            operator: OperatorConditionProfileV1 {
                relative_jacobi_tolerance: 1.0e-13,
                relative_rank_tolerance: 1.0e-12,
                maximum_condition_estimate: 1.0e10,
                maximum_jacobi_sweeps: 64,
            },
            maximum_matrix_entries: 128,
            maximum_work_units: 10_000,
        }
    }

    fn mass_q() -> Metric<CartesianQ, 3> {
        Metric::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            metric_profile(),
        )
        .unwrap()
    }

    fn constraint_metric() -> Metric<ConstraintSpace, 2> {
        Metric::admit([[2.0, 0.25], [0.25, 1.5]], metric_profile()).unwrap()
    }

    fn jacobian_q() -> [[f64; 3]; 2] {
        [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]]
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "{left:.17e} != {right:.17e} tolerance={tolerance:.3e}"
        );
    }

    fn matrix_close<const R: usize, const C: usize>(
        left: &[[f64; C]; R],
        right: &[[f64; C]; R],
        tolerance: f64,
    ) {
        for (left_row, right_row) in left.iter().zip(right.iter()) {
            for (left_value, right_value) in left_row.iter().zip(right_row.iter()) {
                close(*left_value, *right_value, tolerance);
            }
        }
    }

    fn transpose<const N: usize>(matrix: &[[f64; N]; N]) -> [[f64; N]; N] {
        let mut result = [[0.0; N]; N];
        for row in 0..N {
            for column in 0..N {
                result[row][column] = matrix[column][row];
            }
        }
        result
    }

    fn matmul<const R: usize, const K: usize, const C: usize>(
        left: &[[f64; K]; R],
        right: &[[f64; C]; K],
    ) -> [[f64; C]; R] {
        let mut result = [[0.0; C]; R];
        for row in 0..R {
            for column in 0..C {
                for inner in 0..K {
                    result[row][column] += left[row][inner] * right[inner][column];
                }
            }
        }
        result
    }

    fn inverse_from_rebasis<F: CoordinateFrame, T: CoordinateFrame, const K: usize>(
        rebasis: &Rebasis<F, T, K>,
    ) -> [[f64; K]; K] {
        let mut inverse = [[0.0; K]; K];
        for column in 0..K {
            let mut unit = [0.0; K];
            unit[column] = 1.0;
            let vector = Tangent::<F, K>::new(unit).unwrap();
            let transformed = rebasis.tangent_to_to(&vector).unwrap();
            for row in 0..K {
                inverse[row][column] = transformed.components()[row];
            }
        }
        inverse
    }

    #[test]
    fn simultaneous_domain_and_constraint_rebases_preserve_spectrum_and_delassus() {
        let mass_q = mass_q();
        let constraint_metric_c = constraint_metric();
        let jacobian_q = jacobian_q();
        let receipt_q = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass_q,
            constraint_metric_c,
            jacobian_q,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        assert_eq!(
            receipt_q.rank_status(),
            ConstraintMapRankStatusV1::FullRowRank
        );
        assert_eq!(receipt_q.operator_receipt().numerical_rank, 2);
        assert!(receipt_q.delassus().is_some());
        assert_eq!(receipt_q.delassus_whitened_eigenvalue_estimates().len(), 2);

        let domain_rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let constraint_rebasis = Rebasis::<ConstraintSpace, DenseConstraint, 2>::new(
            [[1.0, 2.0], [0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let mass_y = domain_rebasis
            .metric_to_to(&mass_q, metric_profile())
            .unwrap();
        let constraint_metric_d = constraint_rebasis
            .metric_to_to(&constraint_metric_c, metric_profile())
            .unwrap();
        let jacobian_d =
            rebase_constraint_jacobian_both(&jacobian_q, &domain_rebasis, &constraint_rebasis)
                .unwrap();
        let receipt_d = ConstraintSpaceReceiptV1::<DenseY, DenseConstraint, 3, 2>::new(
            mass_y,
            constraint_metric_d,
            jacobian_d,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        assert_eq!(
            receipt_d.rank_status(),
            ConstraintMapRankStatusV1::FullRowRank
        );
        assert_eq!(receipt_d.operator_receipt().numerical_rank, 2);
        for (left, right) in receipt_q
            .singular_values()
            .iter()
            .zip(receipt_d.singular_values().iter())
        {
            close(*left, *right, 2.0e-10);
        }
        for (left, right) in receipt_q
            .delassus_whitened_eigenvalue_estimates()
            .iter()
            .zip(receipt_d.delassus_whitened_eigenvalue_estimates().iter())
        {
            close(*left, *right, 5.0e-10);
        }

        let delassus_q = receipt_q.delassus().unwrap();
        let delassus_d = receipt_d.delassus().unwrap();
        let inverse_t = inverse_from_rebasis(&constraint_rebasis);
        let expected_w_d = matmul(
            &matmul(&inverse_t, delassus_q.matrix()),
            &transpose(&inverse_t),
        );
        matrix_close(delassus_d.matrix(), &expected_w_d, 5.0e-10);

        let velocity_y = Tangent::<DenseY, 3>::new([0.25, -0.5, 1.0]).unwrap();
        let velocity_q = domain_rebasis.tangent_to_from(&velocity_y).unwrap();
        let rate_q = receipt_q.apply_jacobian(&velocity_q).unwrap();
        let rate_d = receipt_d.apply_jacobian(&velocity_y).unwrap();
        let expected_rate_d = constraint_rebasis.tangent_to_to(&rate_q).unwrap();
        for (actual, expected) in rate_d
            .components()
            .iter()
            .zip(expected_rate_d.components().iter())
        {
            close(*actual, *expected, 2.0e-12);
        }
        close(
            receipt_q.constraint_rate_norm(&rate_q).unwrap(),
            receipt_d.constraint_rate_norm(&rate_d).unwrap(),
            2.0e-10,
        );

        let multiplier_q = Cotangent::<ConstraintSpace, 2>::new([0.75, -0.2]).unwrap();
        let multiplier_d = constraint_rebasis.cotangent_to_to(&multiplier_q).unwrap();
        close(
            receipt_q
                .constraint_covector_dual_norm(&multiplier_q)
                .unwrap(),
            receipt_d
                .constraint_covector_dual_norm(&multiplier_d)
                .unwrap(),
            2.0e-10,
        );
        close(
            crate::pairing(&multiplier_q, &rate_q),
            crate::pairing(&multiplier_d, &rate_d),
            2.0e-12,
        );

        let multiplier_q_typed =
            ConstraintMultiplierV1::<ConstraintSpace, 2>::new(*multiplier_q.components()).unwrap();
        let multiplier_d_typed =
            ConstraintMultiplierV1::<DenseConstraint, 2>::new(*multiplier_d.components()).unwrap();
        let correction_q = delassus_q
            .induced_velocity_correction(&multiplier_q_typed)
            .unwrap();
        let correction_d = delassus_d
            .induced_velocity_correction(&multiplier_d_typed)
            .unwrap();
        let correction_q_from_y = domain_rebasis.tangent_to_from(&correction_d).unwrap();
        for (actual, expected) in correction_q
            .components()
            .iter()
            .zip(correction_q_from_y.components().iter())
        {
            close(*actual, *expected, 2.0e-10);
        }
        let reconstructed_rate = receipt_q.apply_jacobian(&correction_q).unwrap();
        let predicted_rate = delassus_q.apply_multiplier(&multiplier_q_typed).unwrap();
        for (actual, expected) in reconstructed_rate
            .components()
            .iter()
            .zip(predicted_rate.components().iter())
        {
            close(*actual, *expected, 2.0e-12);
        }

        // Solve a compatible constraint residual independently in both bases.
        let target_rate_q = Tangent::<ConstraintSpace, 2>::new([0.4, -0.2]).unwrap();
        let target_rate_d = constraint_rebasis.tangent_to_to(&target_rate_q).unwrap();
        let (lambda_solved_q, correction_solved_q) = receipt_q
            .solve_minimum_kinetic_correction(&target_rate_q)
            .unwrap();
        let (lambda_solved_d, correction_solved_d) = receipt_d
            .solve_minimum_kinetic_correction(&target_rate_d)
            .unwrap();
        let expected_lambda_d = constraint_rebasis
            .cotangent_to_to(&lambda_solved_q)
            .unwrap();
        for (actual, expected) in lambda_solved_d
            .components()
            .iter()
            .zip(expected_lambda_d.components().iter())
        {
            close(*actual, *expected, 2.0e-10);
        }
        let correction_solved_q_from_y = domain_rebasis
            .tangent_to_from(&correction_solved_d)
            .unwrap();
        for (actual, expected) in correction_solved_q
            .components()
            .iter()
            .zip(correction_solved_q_from_y.components().iter())
        {
            close(*actual, *expected, 2.0e-10);
        }
        close(
            receipt_q
                .residual_delassus_dual_norm(&target_rate_q)
                .unwrap(),
            receipt_d
                .residual_delassus_dual_norm(&target_rate_d)
                .unwrap(),
            2.0e-10,
        );
        close(
            receipt_q
                .mass_metric
                .tangent_norm(&correction_solved_q)
                .unwrap(),
            receipt_q
                .residual_delassus_dual_norm(&target_rate_q)
                .unwrap(),
            2.0e-10,
        );
    }

    #[test]
    fn rectangular_dimensions_and_rank_deficiency_do_not_gain_false_delassus_authority() {
        let mass_2 =
            Metric::<CartesianQ, 2>::admit([[2.0, 0.0], [0.0, 3.0]], metric_profile()).unwrap();
        let constraint_3 = Metric::<ConstraintSpace, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 4.0]],
            metric_profile(),
        )
        .unwrap();
        let tall = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 2, 3>::new(
            mass_2,
            constraint_3,
            [[1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        assert_eq!(tall.operator_receipt().numerical_rank, 2);
        assert_eq!(tall.operator_receipt().domain_nullity, 0);
        assert_eq!(tall.operator_receipt().codomain_nullity, 1);
        assert_eq!(
            tall.rank_status(),
            ConstraintMapRankStatusV1::FullColumnRankNotSurjective
        );
        assert!(tall.delassus().is_none());
        assert!(matches!(
            tall.solve_minimum_kinetic_correction(
                &Tangent::<ConstraintSpace, 3>::new([1.0, 2.0, 3.0]).unwrap()
            ),
            Err(ConstraintSpaceReceiptErrorV1::FullRowRankAuthorityUnavailable)
        ));

        let redundant = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass_q(),
            constraint_metric(),
            [[1.0, 0.0, 1.0], [2.0, 0.0, 2.0]],
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        assert_eq!(redundant.operator_receipt().numerical_rank, 1);
        assert_eq!(redundant.operator_receipt().codomain_nullity, 1);
        assert_eq!(
            redundant.rank_status(),
            ConstraintMapRankStatusV1::RankDeficient
        );
        assert!(redundant.delassus().is_none());
    }

    #[test]
    fn distinct_constraint_metrics_change_the_physical_spectrum() {
        let mass = mass_q();
        let jacobian = jacobian_q();
        let first = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass,
            Metric::admit([[1.0, 0.0], [0.0, 1.0]], metric_profile()).unwrap(),
            jacobian,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        let second = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass,
            Metric::admit([[4.0, 0.5], [0.5, 2.0]], metric_profile()).unwrap(),
            jacobian,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        assert!((first.singular_values()[0] - second.singular_values()[0]).abs() > 1.0e-3);
    }

    #[test]
    fn domain_only_coordinate_rebasis_preserves_the_constraint_spectrum() {
        let mass_q = mass_q();
        let constraint_metric_c = constraint_metric();
        let jacobian_q = jacobian_q();
        let baseline = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass_q,
            constraint_metric_c,
            jacobian_q,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();
        let domain_rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            metric_profile(),
        )
        .unwrap();
        let mass_y = domain_rebasis
            .metric_to_to(&mass_q, metric_profile())
            .unwrap();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &domain_rebasis).unwrap();
        let rebased = ConstraintSpaceReceiptV1::<DenseY, ConstraintSpace, 3, 2>::new(
            mass_y,
            constraint_metric_c,
            jacobian_y,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();

        for (expected, actual) in baseline
            .singular_values()
            .iter()
            .zip(rebased.singular_values().iter())
        {
            close(*expected, *actual, 2.0e-10);
        }
    }

    #[test]
    fn missing_constraint_metric_rebasis_is_detected() {
        let mass = mass_q();
        let constraint_metric_c = constraint_metric();
        let jacobian_q = jacobian_q();
        let baseline = ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
            mass,
            constraint_metric_c,
            jacobian_q,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();

        // Re-describe the constraint equations by r_new = T r_old and
        // J_new = T J_old, but deliberately keep C's old component matrix.
        // The correct transformed metric is C_new = T^-T C_old T^-1.
        let jacobian_new = [
            std::array::from_fn(|dof| jacobian_q[0][dof] + 2.0 * jacobian_q[1][dof]),
            jacobian_q[1],
        ];
        let missing_metric_transform =
            Metric::<DenseConstraint, 2>::admit(*constraint_metric_c.matrix(), metric_profile())
                .unwrap();
        let wrong = ConstraintSpaceReceiptV1::<CartesianQ, DenseConstraint, 3, 2>::new(
            mass,
            missing_metric_transform,
            jacobian_new,
            metric_profile(),
            constraint_profile(),
        )
        .unwrap();

        assert!(
            (baseline.singular_values()[0] - wrong.singular_values()[0]).abs() > 1.0e-3,
            "omitting C_new = T^-T C_old T^-1 must change the physical constraint spectrum"
        );
    }

    #[test]
    fn zero_dimensions_and_budget_exhaustion_fail_before_qualification() {
        let zero =
            RectangularTangentToTangentOperator::<CartesianQ, ConstraintSpace, 0, 3>::new([]);
        assert!(matches!(zero, Err(OperatorConditionErrorV1::ZeroDimension)));

        let mut limited = constraint_profile();
        limited.maximum_matrix_entries = 2;
        assert!(matches!(
            ConstraintSpaceReceiptV1::<CartesianQ, ConstraintSpace, 3, 2>::new(
                mass_q(),
                constraint_metric(),
                jacobian_q(),
                metric_profile(),
                limited,
            ),
            Err(ConstraintSpaceReceiptErrorV1::MatrixEntryBudgetExceeded)
        ));
    }
}
