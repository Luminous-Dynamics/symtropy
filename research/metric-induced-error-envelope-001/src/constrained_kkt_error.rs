// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! A-posteriori residual certificates for fixed linear constrained KKT systems.
//!
//! The admitted system is
//!
//!     [ M  J^T ] [a]      [f]
//!     [ J   0  ] [λ]  =   [b],
//!
//! where M is SPD and J has full row rank. The retained Delassus metric
//! W = J M^-1 J^T supplies the dual norm for the constraint residual.
//!
//! Under the paired norms, whitening reduces the KKT operator to
//!
//!     [ I  B ] [u] = [g],    B^T B = I.
//!     [B^T 0 ] [v]   [h]
//!
//! The exact-arithmetic error constants are 1 for acceleration, sqrt(2) for
//! the multiplier, and (1 + sqrt(5))/2 for the combined state. Floating-point
//! evaluation of this receipt is numerical; it is not an interval proof.

use crate::{
    ConstraintMultiplierV1, ConstraintResidualV1, CoordinateFrame, Cotangent, DelassusMetricV1,
    MetricErrorV1, Tangent,
};

/// Sharp operator constants for the fixed, full-row-rank linear KKT system.
pub const KKT_ACCELERATION_ERROR_CONSTANT_V1: f64 = 1.0;
pub const KKT_MULTIPLIER_ERROR_CONSTANT_V1: f64 = std::f64::consts::SQRT_2;
pub const KKT_COMBINED_STATE_ERROR_CONSTANT_V1: f64 = 1.618_033_988_749_895;

/// Independently reconstructed residuals and metric-aware state-error estimates.
///
/// The acceleration error is measured in the mass metric M, the multiplier
/// error in the Delassus metric W, and the residuals in their dual metrics.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstrainedKktResidualReceiptV1<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    pub mass_equilibrium_residual: Cotangent<Frame, N>,
    pub constraint_residual: ConstraintResidualV1<Space, K>,
    pub mass_residual_dual_norm: f64,
    pub constraint_residual_dual_norm: f64,
    pub paired_residual_norm: f64,
    pub acceleration_error_bound: f64,
    pub multiplier_error_bound: f64,
    pub combined_state_error_bound: f64,
}

/// Reconstruct the KKT residuals of a candidate solution and evaluate the
/// exact-arithmetic residual-to-error inequalities for the admitted operator.
///
/// force_rhs is a force covector in the generalized-coordinate frame.
/// constraint_rhs is the right-hand side in the constraint-equation space.
/// The Delassus metric retains the mass metric and Jacobian used to form W.
///
/// This function certifies only the fixed linear KKT operator represented by
/// delassus. It does not bound nonlinear integration or model-discretization
/// error. The returned f64 values are numerical estimates of the exact bounds,
/// not formally rounded-out interval enclosures.
pub fn certify_linear_constrained_solution<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    delassus: &DelassusMetricV1<Frame, Space, N, K>,
    force_rhs: &Cotangent<Frame, N>,
    constraint_rhs: &ConstraintResidualV1<Space, K>,
    computed_acceleration: &Tangent<Frame, N>,
    computed_multiplier: &ConstraintMultiplierV1<Space, K>,
) -> Result<ConstrainedKktResidualReceiptV1<Frame, Space, N, K>, MetricErrorV1> {
    // First KKT row: r_m = f - (M a_hat + J^T lambda_hat).
    let mass_times_acceleration = delassus.mass_metric().lower(computed_acceleration)?;
    let mut generalized_constraint_force = [0.0; N];
    for (constraint_index, jacobian_row) in delassus.jacobian().iter().enumerate() {
        let multiplier = computed_multiplier.components()[constraint_index];
        for (dof, force_component) in generalized_constraint_force.iter_mut().enumerate() {
            *force_component += jacobian_row[dof] * multiplier;
            if !force_component.is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
        }
    }

    let mass_residual_components: [f64; N] = std::array::from_fn(|dof| {
        force_rhs.components()[dof]
            - mass_times_acceleration.components()[dof]
            - generalized_constraint_force[dof]
    });
    if mass_residual_components
        .iter()
        .any(|component| !component.is_finite())
    {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    let mass_equilibrium_residual = Cotangent::new(mass_residual_components)?;

    // Second KKT row: r_c = b - J a_hat.
    let achieved_constraint = delassus.apply_jacobian(computed_acceleration)?;
    let constraint_residual_components: [f64; K] = std::array::from_fn(|index| {
        constraint_rhs.components()[index] - achieved_constraint.components()[index]
    });
    if constraint_residual_components
        .iter()
        .any(|component| !component.is_finite())
    {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }
    let constraint_residual = ConstraintResidualV1::new(constraint_residual_components)?;

    // rho^2 = ||r_m||_(M^-1)^2 + ||r_c||_(W^-1)^2.
    let mass_residual_dual_norm = delassus
        .mass_metric()
        .cotangent_norm(&mass_equilibrium_residual)?;
    let constraint_residual_dual_norm = delassus.residual_norm(&constraint_residual)?;
    let paired_residual_norm = mass_residual_dual_norm.hypot(constraint_residual_dual_norm);

    let acceleration_error_bound = KKT_ACCELERATION_ERROR_CONSTANT_V1 * paired_residual_norm;
    let multiplier_error_bound = KKT_MULTIPLIER_ERROR_CONSTANT_V1 * paired_residual_norm;
    let combined_state_error_bound = KKT_COMBINED_STATE_ERROR_CONSTANT_V1 * paired_residual_norm;

    if !paired_residual_norm.is_finite()
        || !acceleration_error_bound.is_finite()
        || !multiplier_error_bound.is_finite()
        || !combined_state_error_bound.is_finite()
    {
        return Err(MetricErrorV1::NonFiniteIntermediate);
    }

    Ok(ConstrainedKktResidualReceiptV1 {
        mass_equilibrium_residual,
        constraint_residual,
        mass_residual_dual_norm,
        constraint_residual_dual_norm,
        paired_residual_norm,
        acceleration_error_bound,
        multiplier_error_bound,
        combined_state_error_bound,
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
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "{left:.17e} != {right:.17e}; tolerance={tolerance:.3e}"
        );
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct RebasedConstraintSpace;
    impl CoordinateFrame for RebasedConstraintSpace {}

    fn diagonal_fixture() -> (
        Metric<CartesianQ, 2>,
        DelassusMetricV1<CartesianQ, ConstraintSpace, 2, 1>,
        Cotangent<CartesianQ, 2>,
        ConstraintResidualV1<ConstraintSpace, 1>,
    ) {
        let mass = Metric::admit([[2.0, 0.0], [0.0, 4.0]], profile()).unwrap();
        let jacobian = [[1.0, 0.0]];
        let delassus = DelassusMetricV1::from_mass_jacobian(&mass, jacobian, profile()).unwrap();
        let force = Cotangent::new([2.25, -8.0]).unwrap();
        let constraint_rhs = ConstraintResidualV1::new([1.0]).unwrap();
        (mass, delassus, force, constraint_rhs)
    }

    #[test]
    fn dyadic_fixture_bounds_independently_known_forward_error() {
        let (_mass, delassus, force_rhs, constraint_rhs) = diagonal_fixture();
        let exact_acceleration = [1.0, -2.0];
        let exact_multiplier = 0.25;
        let computed_acceleration = Tangent::new([1.125, -1.75]).unwrap();
        let computed_multiplier = ConstraintMultiplierV1::new([0.3125]).unwrap();

        let receipt = certify_linear_constrained_solution(
            &delassus,
            &force_rhs,
            &constraint_rhs,
            &computed_acceleration,
            &computed_multiplier,
        )
        .unwrap();

        // Independent dyadic arithmetic:
        // r_m = [-5/16, -1], r_c = [-1/8], W = [1/2].
        assert_eq!(
            receipt.mass_equilibrium_residual.components(),
            &[-0.3125, -1.0]
        );
        assert_eq!(receipt.constraint_residual.components(), &[-0.125]);
        close(
            receipt.mass_residual_dual_norm.powi(2),
            0.298828125,
            2.0e-14,
        );
        close(
            receipt.constraint_residual_dual_norm.powi(2),
            0.03125,
            2.0e-14,
        );

        let acceleration_error = [
            exact_acceleration[0] - computed_acceleration.components()[0],
            exact_acceleration[1] - computed_acceleration.components()[1],
        ];
        let acceleration_error_norm =
            (2.0 * acceleration_error[0].powi(2) + 4.0 * acceleration_error[1].powi(2)).sqrt();
        let multiplier_error_norm =
            (0.5 * (exact_multiplier - computed_multiplier.components()[0]).powi(2)).sqrt();
        let combined_state_error_norm = acceleration_error_norm.hypot(multiplier_error_norm);

        assert!(acceleration_error_norm <= receipt.acceleration_error_bound);
        assert!(multiplier_error_norm <= receipt.multiplier_error_bound);
        assert!(combined_state_error_norm <= receipt.combined_state_error_bound);
    }

    #[test]
    fn exact_solution_has_zero_residual_and_zero_error_bounds() {
        let (_mass, delassus, force_rhs, constraint_rhs) = diagonal_fixture();
        let acceleration = Tangent::new([1.0, -2.0]).unwrap();
        let multiplier = ConstraintMultiplierV1::new([0.25]).unwrap();
        let receipt = certify_linear_constrained_solution(
            &delassus,
            &force_rhs,
            &constraint_rhs,
            &acceleration,
            &multiplier,
        )
        .unwrap();

        assert_eq!(receipt.mass_residual_dual_norm, 0.0);
        assert_eq!(receipt.constraint_residual_dual_norm, 0.0);
        assert_eq!(receipt.paired_residual_norm, 0.0);
        assert_eq!(receipt.acceleration_error_bound, 0.0);
        assert_eq!(receipt.multiplier_error_bound, 0.0);
        assert_eq!(receipt.combined_state_error_bound, 0.0);
    }

    #[test]
    fn acceleration_bound_is_sharp_for_a_constraint_nullspace_error() {
        let mass = Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], profile()).unwrap();
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 2, 1>::from_mass_jacobian(
            &mass,
            [[1.0, 0.0]],
            profile(),
        )
        .unwrap();
        let force_rhs = Cotangent::new([0.0, 0.0]).unwrap();
        let constraint_rhs = ConstraintResidualV1::new([0.0]).unwrap();
        let computed_acceleration = Tangent::new([0.0, -1.0]).unwrap();
        let computed_multiplier = ConstraintMultiplierV1::new([0.0]).unwrap();

        let receipt = certify_linear_constrained_solution(
            &delassus,
            &force_rhs,
            &constraint_rhs,
            &computed_acceleration,
            &computed_multiplier,
        )
        .unwrap();
        close(receipt.acceleration_error_bound, 1.0, 2.0e-14);
        close(receipt.paired_residual_norm, 1.0, 2.0e-14);
    }

    #[test]
    fn golden_ratio_constant_is_sharp_for_the_combined_kkt_state() {
        let mass = Metric::<CartesianQ, 2>::admit(
            [[1.0, 0.0], [0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let delassus =
            DelassusMetricV1::<CartesianQ, ConstraintSpace, 2, 1>::from_mass_jacobian(
                &mass,
                [[1.0, 0.0]],
                profile(),
            )
            .unwrap();
        let force_rhs = Cotangent::new([0.0, 0.0]).unwrap();
        let constraint_rhs = ConstraintResidualV1::new([0.0]).unwrap();
        let phi = KKT_COMBINED_STATE_ERROR_CONSTANT_V1;
        let computed_acceleration = Tangent::new([phi, 0.0]).unwrap();
        let computed_multiplier = ConstraintMultiplierV1::new([-phi * phi]).unwrap();

        let receipt = certify_linear_constrained_solution(
            &delassus,
            &force_rhs,
            &constraint_rhs,
            &computed_acceleration,
            &computed_multiplier,
        )
        .unwrap();

        let true_acceleration_error = phi;
        let true_multiplier_error = phi * phi;
        let combined_true_error = true_acceleration_error.hypot(true_multiplier_error);
        close(
            receipt.combined_state_error_bound,
            combined_true_error,
            5.0e-14,
        );
    }

    #[test]
    fn residual_and_bounds_are_invariant_under_dense_coordinate_rebasing() {
        let mass_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 8.0]],
            profile(),
        )
        .unwrap();
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q,
            jacobian_q,
            profile(),
        )
        .unwrap();
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y,
            jacobian_y,
            profile(),
        )
        .unwrap();

        let exact_acceleration_q = Tangent::new([1.0, -2.0, 0.5]).unwrap();
        let exact_multiplier = [0.25, -0.5];
        let computed_acceleration_q = Tangent::new([1.1, -2.2, 0.8]).unwrap();
        let computed_multiplier = ConstraintMultiplierV1::new([0.3, -0.6]).unwrap();

        let mass_force = mass_q.lower(&exact_acceleration_q).unwrap();
        let mut force_components = [0.0; 3];
        for dof in 0..3 {
            force_components[dof] = mass_force.components()[dof]
                + jacobian_q[0][dof] * exact_multiplier[0]
                + jacobian_q[1][dof] * exact_multiplier[1];
        }
        let force_q = Cotangent::new(force_components).unwrap();
        let target_q = ConstraintResidualV1::new([
            crate::dot(&jacobian_q[0], exact_acceleration_q.components()),
            crate::dot(&jacobian_q[1], exact_acceleration_q.components()),
        ])
        .unwrap();

        let receipt_q = certify_linear_constrained_solution(
            &delassus_q,
            &force_q,
            &target_q,
            &computed_acceleration_q,
            &computed_multiplier,
        )
        .unwrap();

        let force_y = rebasis.cotangent_to_to(&force_q).unwrap();
        let computed_acceleration_y = rebasis.tangent_to_to(&computed_acceleration_q).unwrap();
        let receipt_y = certify_linear_constrained_solution(
            &delassus_y,
            &force_y,
            &target_q,
            &computed_acceleration_y,
            &computed_multiplier,
        )
        .unwrap();

        close(
            receipt_q.mass_residual_dual_norm,
            receipt_y.mass_residual_dual_norm,
            2.0e-11,
        );
        close(
            receipt_q.constraint_residual_dual_norm,
            receipt_y.constraint_residual_dual_norm,
            2.0e-11,
        );
        close(
            receipt_q.paired_residual_norm,
            receipt_y.paired_residual_norm,
            2.0e-11,
        );
        close(
            receipt_q.combined_state_error_bound,
            receipt_y.combined_state_error_bound,
            2.0e-11,
        );
    }

    #[test]
    fn kkt_residual_certificate_is_invariant_under_independent_coordinate_and_constraint_rebases() {
        let mass_q = Metric::<CartesianQ, 3>::admit(
            [[2.0, 0.0, 0.0], [0.0, 4.0, 0.0], [0.0, 0.0, 8.0]],
            profile(),
        )
        .unwrap();
        let jacobian_q = [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]];
        let delassus_q =
            DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass_q,
                jacobian_q,
                profile(),
            )
            .unwrap();

        let exact_acceleration_q = Tangent::new([1.0, -2.0, 0.5]).unwrap();
        let exact_multiplier = [0.25, -0.5];
        let computed_acceleration_q = Tangent::new([1.1, -2.2, 0.8]).unwrap();
        let computed_multiplier_q =
            ConstraintMultiplierV1::<ConstraintSpace, 2>::new([0.3, -0.6]).unwrap();

        let mass_force = mass_q.lower(&exact_acceleration_q).unwrap();
        let mut force_components = [0.0; 3];
        for dof in 0..3 {
            force_components[dof] = mass_force.components()[dof]
                + jacobian_q[0][dof] * exact_multiplier[0]
                + jacobian_q[1][dof] * exact_multiplier[1];
        }
        let force_q = Cotangent::new(force_components).unwrap();
        let target_q = ConstraintResidualV1::<ConstraintSpace, 2>::new([
            crate::dot(&jacobian_q[0], exact_acceleration_q.components()),
            crate::dot(&jacobian_q[1], exact_acceleration_q.components()),
        ])
        .unwrap();
        let receipt_q = certify_linear_constrained_solution(
            &delassus_q,
            &force_q,
            &target_q,
            &computed_acceleration_q,
            &computed_multiplier_q,
        )
        .unwrap();

        // Generalized-coordinate rebasing q = S y.
        let rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();
        let force_y = rebasis.cotangent_to_to(&force_q).unwrap();
        let computed_acceleration_y = rebasis.tangent_to_to(&computed_acceleration_q).unwrap();

        // Independent constraint-equation re-description r_new = T r_old,
        // with T = [[1, 2], [0, 1]]. Therefore lambda_new = T^-T lambda_old.
        let jacobian_y_t = [
            std::array::from_fn(|dof| jacobian_y[0][dof] + 2.0 * jacobian_y[1][dof]),
            jacobian_y[1],
        ];
        let target_y_t = ConstraintResidualV1::<RebasedConstraintSpace, 2>::new([
            target_q.components()[0] + 2.0 * target_q.components()[1],
            target_q.components()[1],
        ])
        .unwrap();
        let computed_multiplier_y_t = ConstraintMultiplierV1::<RebasedConstraintSpace, 2>::new([
            computed_multiplier_q.components()[0],
            computed_multiplier_q.components()[1] - 2.0 * computed_multiplier_q.components()[0],
        ])
        .unwrap();
        let delassus_y_t =
            DelassusMetricV1::<DenseY, RebasedConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass_y,
                jacobian_y_t,
                profile(),
            )
            .unwrap();

        let receipt_y_t = certify_linear_constrained_solution(
            &delassus_y_t,
            &force_y,
            &target_y_t,
            &computed_acceleration_y,
            &computed_multiplier_y_t,
        )
        .unwrap();

        close(
            receipt_q.mass_residual_dual_norm,
            receipt_y_t.mass_residual_dual_norm,
            3.0e-11,
        );
        close(
            receipt_q.constraint_residual_dual_norm,
            receipt_y_t.constraint_residual_dual_norm,
            3.0e-11,
        );
        close(
            receipt_q.paired_residual_norm,
            receipt_y_t.paired_residual_norm,
            3.0e-11,
        );
        close(
            receipt_q.acceleration_error_bound,
            receipt_y_t.acceleration_error_bound,
            3.0e-11,
        );
        close(
            receipt_q.multiplier_error_bound,
            receipt_y_t.multiplier_error_bound,
            3.0e-11,
        );
        close(
            receipt_q.combined_state_error_bound,
            receipt_y_t.combined_state_error_bound,
            3.0e-11,
        );
    }

    #[test]
    fn redundant_constraints_fail_before_a_kkt_error_receipt_can_be_issued() {
        let mass = Metric::<CartesianQ, 2>::admit([[1.0, 0.0], [0.0, 1.0]], profile()).unwrap();
        let redundant = DelassusMetricV1::<CartesianQ, ConstraintSpace, 2, 2>::from_mass_jacobian(
            &mass,
            [[1.0, 0.0], [2.0, 0.0]],
            profile(),
        );
        assert!(matches!(
            redundant,
            Err(MetricErrorV1::NotPositiveDefinite { .. })
        ));
    }
}
