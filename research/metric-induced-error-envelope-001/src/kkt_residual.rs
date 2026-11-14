// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Metric-aware residual estimates for a fixed linear constrained KKT system.
//!
//! For SPD M and full-row-rank J, define W = J M^-1 J^T and
//!
//!     [ M  J^T ] [a] = [f]
//!     [ J   0  ] [λ]   [b].
//!
//! A computed pair (a_hat, lambda_hat) has residuals
//!
//!     r_m = f - (M a_hat + J^T lambda_hat)
//!     r_c = b - J a_hat.
//!
//! The paired residual rho² = ||r_m||²_(M^-1) + ||r_c||²_(W^-1) gives
//! exact-arithmetic fixed-linear-system bounds:
//!
//!     ||a - a_hat||_M <= rho
//!     ||lambda - lambda_hat||_W <= sqrt(2) rho
//!     combined_error <= ((1 + sqrt(5))/2) rho.
//!
//! The f64 receipt below is a numerical evaluation of this theorem, not a
//! validated real-arithmetic enclosure. Nonlinear midpoint and contact claims
//! require separate residual-to-state and active-set qualifications.

use crate::{
    ConstraintMultiplierV1, ConstraintResidualV1, CoordinateFrame, Cotangent,
    DelassusMetricV1, MetricErrorV1, Tangent,
};

const SQRT_2: f64 = std::f64::consts::SQRT_2;
const GOLDEN_RATIO: f64 = (1.0 + 5.0_f64.sqrt()) * 0.5;

/// Explicitly narrow claim scope for the current residual certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KktResidualClaimScopeV1 {
    FixedLinearSpdMassFullRowRankConstraints,
}

/// Numerical evaluation of the exact-arithmetic linear KKT residual theorem.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstrainedKktResidualReceiptV1<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    pub force_balance_residual: Cotangent<Frame, N>,
    pub constraint_residual: ConstraintResidualV1<Space, K>,
    pub force_residual_dual_norm: f64,
    pub constraint_residual_dual_norm: f64,
    pub paired_residual_norm: f64,
    pub acceleration_error_bound: f64,
    pub multiplier_error_bound: f64,
    pub combined_state_error_bound: f64,
    pub claim_scope: KktResidualClaimScopeV1,
}

/// Evaluate the force and constraint residuals of a candidate fixed-linear KKT
/// solution, then compute the metric-paired residual and theorem bounds.
///
/// The Delassus object owns the mass metric and Jacobian used here, preventing
/// a caller from accidentally combining residuals from unrelated M/J inputs.
pub fn evaluate_constrained_kkt_residual<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    delassus: &DelassusMetricV1<Frame, Space, N, K>,
    force_rhs: &Cotangent<Frame, N>,
    constraint_rhs: &ConstraintResidualV1<Space, K>,
    acceleration_candidate: &Tangent<Frame, N>,
    multiplier_candidate: &ConstraintMultiplierV1<Space, K>,
) -> Result<ConstrainedKktResidualReceiptV1<Frame, Space, N, K>, MetricErrorV1> {
    // r_m = f - M a_hat - J^T lambda_hat.
    let inertial_covector = delassus.mass_metric().lower(acceleration_candidate)?;
    let mut force_residual = *force_rhs.components();
    for (dof, residual_component) in force_residual.iter_mut().enumerate() {
        *residual_component -= inertial_covector.components()[dof];
        for (constraint, row) in delassus.jacobian().iter().enumerate() {
            *residual_component -= row[dof] * multiplier_candidate.components()[constraint];
        }
        if !residual_component.is_finite() {
            return Err(MetricErrorV1::NonFiniteIntermediate);
        }
    }
    let force_balance_residual = Cotangent::<Frame, N>::new(force_residual)?;

    // r_c = b - J a_hat.
    let achieved_constraint = delassus.apply_jacobian(acceleration_candidate)?;
    let constraint_residual = ConstraintResidualV1::new(
        std::array::from_fn(|index| {
            constraint_rhs.components()[index] - achieved_constraint.components()[index]
        }),
    )?;

    let force_residual_dual_norm = delassus
        .mass_metric()
        .cotangent_norm(&force_balance_residual)?;
    let constraint_residual_dual_norm = delassus.residual_norm(&constraint_residual)?;
    let paired_residual_norm =
        force_residual_dual_norm.hypot(constraint_residual_dual_norm);
    if !paired_residual_norm.is_finite() {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    let acceleration_error_bound = paired_residual_norm;
    let multiplier_error_bound = SQRT_2 * paired_residual_norm;
    let combined_state_error_bound = GOLDEN_RATIO * paired_residual_norm;
    if !acceleration_error_bound.is_finite()
        || !multiplier_error_bound.is_finite()
        || !combined_state_error_bound.is_finite()
    {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    Ok(ConstrainedKktResidualReceiptV1 {
        force_balance_residual,
        constraint_residual,
        force_residual_dual_norm,
        constraint_residual_dual_norm,
        paired_residual_norm,
        acceleration_error_bound,
        multiplier_error_bound,
        combined_state_error_bound,
        claim_scope: KktResidualClaimScopeV1::FixedLinearSpdMassFullRowRankConstraints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CartesianQ, ConstraintSpace, DenseY, Metric, MetricProfileV1, Rebasis,
        rebase_constraint_jacobian,
    };

    fn profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.16e}, right={right:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn diagonal_kkt_fixture_covers_independent_solution_errors() {
        let mass = Metric::<CartesianQ, 2>::admit(
            [[2.0, 0.0], [0.0, 3.0]],
            profile(),
        )
        .unwrap();
        let jacobian = [[1.0, 1.0]];
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 2, 1>::from_mass_jacobian(
            &mass, jacobian, profile(),
        )
        .unwrap();

        let acceleration_exact = Tangent::new([0.5, -0.25]).unwrap();
        let multiplier_exact = ConstraintMultiplierV1::new([0.75]).unwrap();
        let force_rhs = Cotangent::new([1.75, 0.0]).unwrap();
        let constraint_rhs = ConstraintResidualV1::new([0.25]).unwrap();

        let acceleration_candidate = Tangent::new([0.49, -0.24]).unwrap();
        let multiplier_candidate = ConstraintMultiplierV1::new([0.74]).unwrap();
        let receipt = evaluate_constrained_kkt_residual(
            &delassus,
            &force_rhs,
            &constraint_rhs,
            &acceleration_candidate,
            &multiplier_candidate,
        )
        .unwrap();

        let acceleration_error = Tangent::new([
            acceleration_exact.components()[0] - acceleration_candidate.components()[0],
            acceleration_exact.components()[1] - acceleration_candidate.components()[1],
        ])
        .unwrap();
        let multiplier_error =
            ConstraintMultiplierV1::new([multiplier_exact.components()[0]
                - multiplier_candidate.components()[0]])
            .unwrap();
        let acceleration_error_norm = mass.tangent_norm(&acceleration_error).unwrap();
        let multiplier_error_norm = delassus.multiplier_norm(&multiplier_error).unwrap();
        let combined_error_norm = acceleration_error_norm.hypot(multiplier_error_norm);

        assert!(
            acceleration_error_norm <= receipt.acceleration_error_bound + 2.0e-13
        );
        assert!(multiplier_error_norm <= receipt.multiplier_error_bound + 2.0e-13);
        assert!(combined_error_norm <= receipt.combined_state_error_bound + 2.0e-13);
        assert_eq!(
            receipt.claim_scope,
            KktResidualClaimScopeV1::FixedLinearSpdMassFullRowRankConstraints
        );
    }

    #[test]
    fn combined_state_bound_is_sharp_at_the_golden_ratio_constant() {
        // N=K=1, M=J=1. The reduced 2x2 KKT block attains the operator
        // norm phi for x=1, h=-phi in the normalized range coordinates.
        let mass = Metric::<CartesianQ, 1>::admit([[1.0]], profile()).unwrap();
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 1, 1>::from_mass_jacobian(
            &mass,
            [[1.0]],
            profile(),
        )
        .unwrap();
        let phi = GOLDEN_RATIO;
        let receipt = evaluate_constrained_kkt_residual(
            &delassus,
            &Cotangent::new([1.0]).unwrap(),
            &ConstraintResidualV1::new([-phi]).unwrap(),
            &Tangent::new([0.0]).unwrap(),
            &ConstraintMultiplierV1::new([0.0]).unwrap(),
        )
        .unwrap();

        let exact_acceleration_error = phi;
        let exact_multiplier_error = 1.0 + phi;
        let exact_combined_error =
            exact_acceleration_error.hypot(exact_multiplier_error);
        close(receipt.paired_residual_norm, 1.0_f64.hypot(phi), 2.0e-15);
        close(
            receipt.combined_state_error_bound,
            exact_combined_error,
            3.0e-15,
        );
        close(
            receipt.combined_state_error_bound / receipt.paired_residual_norm,
            phi,
            2.0e-15,
        );
    }

    #[test]
    fn acceleration_bound_is_sharp_for_a_constraint_nullspace_residual() {
        let mass =
            Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], profile()).unwrap();
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 2, 1>::from_mass_jacobian(
            &mass,
            [[1.0, 0.0]],
            profile(),
        )
        .unwrap();
        let receipt = evaluate_constrained_kkt_residual(
            &delassus,
            &Cotangent::new([0.0, 1.0]).unwrap(),
            &ConstraintResidualV1::new([0.0]).unwrap(),
            &Tangent::new([0.0, 0.0]).unwrap(),
            &ConstraintMultiplierV1::new([0.0]).unwrap(),
        )
        .unwrap();
        close(receipt.force_residual_dual_norm, 1.0, 0.0);
        close(receipt.constraint_residual_dual_norm, 0.0, 0.0);
        close(receipt.acceleration_error_bound, 1.0, 0.0);
    }

    #[test]
    fn paired_residual_and_bounds_are_invariant_under_dense_coordinate_rebasing() {
        let mass_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap();
        let basis = [
            [1.0, 2.0, -1.0],
            [0.0, 1.0, 3.0],
            [0.0, 0.0, 1.0],
        ];
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(basis, profile()).unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();
        let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q, jacobian_q, profile(),
        )
        .unwrap();
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y, jacobian_y, profile(),
        )
        .unwrap();

        let acceleration_q = Tangent::new([0.3, -0.7, 0.2]).unwrap();
        let acceleration_y = rebasis.tangent_to_to(&acceleration_q).unwrap();
        let force_q = Cotangent::new([1.4, -0.8, 2.2]).unwrap();
        let force_y = rebasis.cotangent_to_to(&force_q).unwrap();
        let multiplier = ConstraintMultiplierV1::new([0.4, -0.2]).unwrap();
        let target = ConstraintResidualV1::new([0.1, -0.3]).unwrap();

        let receipt_q = evaluate_constrained_kkt_residual(
            &delassus_q,
            &force_q,
            &target,
            &acceleration_q,
            &multiplier,
        )
        .unwrap();
        let receipt_y = evaluate_constrained_kkt_residual(
            &delassus_y,
            &force_y,
            &target,
            &acceleration_y,
            &multiplier,
        )
        .unwrap();

        close(
            receipt_q.force_residual_dual_norm,
            receipt_y.force_residual_dual_norm,
            2.0e-12,
        );
        close(
            receipt_q.constraint_residual_dual_norm,
            receipt_y.constraint_residual_dual_norm,
            2.0e-12,
        );
        close(
            receipt_q.paired_residual_norm,
            receipt_y.paired_residual_norm,
            2.0e-12,
        );
        close(
            receipt_q.combined_state_error_bound,
            receipt_y.combined_state_error_bound,
            2.0e-12,
        );
    }

    #[test]
    fn redundant_constraints_cannot_produce_a_finite_kkt_certificate() {
        let mass =
            Metric::<CartesianQ, 3>::admit([[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], profile())
                .unwrap();
        let redundant = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0]];
        let result = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass, redundant, profile(),
        );
        assert!(matches!(
            result,
            Err(MetricErrorV1::NotPositiveDefinite { .. })
        ));
    }

    #[test]
    fn wrong_force_variance_is_detected_after_dense_rebasing() {
        let mass_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap();
        let basis = [[1.0, 5.0, 0.0], [0.0, 1.0, 2.0], [0.0, 0.0, 1.0]];
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(basis, profile()).unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y, jacobian_y, profile(),
        )
        .unwrap();
        let force_q = Cotangent::new([2.0, -1.0, 3.0]).unwrap();
        let correct_force_y = rebasis.cotangent_to_to(&force_q).unwrap();
        let wrong_force_y = Cotangent::new(*force_q.components()).unwrap();
        let acceleration_y = Tangent::new([0.5, -0.25, 0.75]).unwrap();
        let multiplier = ConstraintMultiplierV1::new([0.2, -0.4]).unwrap();
        let target = ConstraintResidualV1::new([0.1, 0.3]).unwrap();

        let correct = evaluate_constrained_kkt_residual(
            &delassus_y,
            &correct_force_y,
            &target,
            &acceleration_y,
            &multiplier,
        )
        .unwrap();
        let wrong = evaluate_constrained_kkt_residual(
            &delassus_y,
            &wrong_force_y,
            &target,
            &acceleration_y,
            &multiplier,
        )
        .unwrap();
        assert!(
            (correct.paired_residual_norm - wrong.paired_residual_norm).abs() > 1.0e-3
        );
    }
}
