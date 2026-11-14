// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Delassus-induced primal/dual norms for full-row-rank holonomic constraints.
//!
//! For an SPD mass metric M and constraint Jacobian J:
//!     W = J M^-1 J^T
//! maps constraint multipliers to constraint-space velocity corrections.
//! The multiplier norm is sqrt(lambda^T W lambda); the residual norm is
//! sqrt(r^T W^-1 r). Both are invariant under pure generalized-coordinate
//! rebasing when M and J are transformed with their correct tensor variance.

use core::marker::PhantomData;

use crate::{
    CoordinateFrame, Cotangent, Metric, MetricAdmissionReceiptV1, MetricErrorV1, MetricProfileV1,
    Rebasis, Tangent,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintMultiplierV1<Space: CoordinateFrame, const K: usize> {
    components: [f64; K],
    _space: PhantomData<Space>,
}

impl<Space: CoordinateFrame, const K: usize> ConstraintMultiplierV1<Space, K> {
    pub fn new(components: [f64; K]) -> Result<Self, MetricErrorV1> {
        if K == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if components.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        Ok(Self {
            components,
            _space: PhantomData,
        })
    }

    pub fn components(&self) -> &[f64; K] {
        &self.components
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstraintResidualV1<Space: CoordinateFrame, const K: usize> {
    components: [f64; K],
    _space: PhantomData<Space>,
}

impl<Space: CoordinateFrame, const K: usize> ConstraintResidualV1<Space, K> {
    pub fn new(components: [f64; K]) -> Result<Self, MetricErrorV1> {
        if K == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        if components.iter().any(|value| !value.is_finite()) {
            return Err(MetricErrorV1::NonFiniteVector);
        }
        Ok(Self {
            components,
            _space: PhantomData,
        })
    }

    pub fn components(&self) -> &[f64; K] {
        &self.components
    }
}

/// W = J M^-1 J^T, retaining the exact admitted mass metric and Jacobian
/// that generated the constraint-space metric.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DelassusMetricV1<
    Frame: CoordinateFrame,
    Space: CoordinateFrame,
    const N: usize,
    const K: usize,
> {
    mass: Metric<Frame, N>,
    jacobian: [[f64; N]; K],
    whitened_jacobian: [[f64; N]; K],
    metric: Metric<Space, K>,
    _spaces: PhantomData<(Frame, Space)>,
}

impl<Frame: CoordinateFrame, Space: CoordinateFrame, const N: usize, const K: usize>
    DelassusMetricV1<Frame, Space, N, K>
{
    /// Build W by applying the mass Riesz inverse to each constraint row.
    /// No explicit inverse of M is formed.
    pub fn from_mass_jacobian(
        mass: &Metric<Frame, N>,
        jacobian: [[f64; N]; K],
        profile: MetricProfileV1,
    ) -> Result<Self, MetricErrorV1> {
        if N == 0 || K == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }

        // Whiten each constraint covector with L^-1 for M = L L^T.
        // The resulting rows are Y = J L^-T and W = Y Y^T.
        // This is equivalent to J M^-1 J^T but avoids a full mass solve
        // for each pair of constraint rows and never forms M^-1.
        let mut whitened_jacobian = [[0.0; N]; K];
        for (row, components) in jacobian.iter().enumerate() {
            let constraint_covector = Cotangent::<Frame, N>::new(*components)?;
            whitened_jacobian[row] = mass.whiten_cotangent(&constraint_covector)?;
        }

        let mut delassus = [[0.0; K]; K];
        for row in 0..K {
            for column in row..K {
                let value = crate::dot(&whitened_jacobian[row], &whitened_jacobian[column]);
                if !value.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                delassus[row][column] = value;
                delassus[column][row] = value;
            }
        }

        let metric = Metric::<Space, K>::admit(delassus, profile)?;
        Ok(Self {
            mass: *mass,
            jacobian,
            whitened_jacobian,
            metric,
            _spaces: PhantomData,
        })
    }

    pub fn matrix(&self) -> &[[f64; K]; K] {
        self.metric.matrix()
    }

    pub fn admission_receipt(&self) -> MetricAdmissionReceiptV1 {
        self.metric.admission_receipt()
    }

    pub fn jacobian(&self) -> &[[f64; N]; K] {
        &self.jacobian
    }

    /// Whitened constraint rows Y = J L^-T for M = L L^T.
    /// The Delassus operator is the row Gram matrix W = Y Y^T.
    pub fn whitened_jacobian(&self) -> &[[f64; N]; K] {
        &self.whitened_jacobian
    }

    /// ||lambda||_W = sqrt(lambda^T W lambda).
    /// This equals the kinetic norm of M^-1 J^T lambda.
    pub fn multiplier_norm(
        &self,
        multiplier: &ConstraintMultiplierV1<Space, K>,
    ) -> Result<f64, MetricErrorV1> {
        let multiplier_coordinates = Tangent::<Space, K>::new(multiplier.components)?;
        self.metric.tangent_norm(&multiplier_coordinates)
    }

    /// ||r||_(W^-1) = sqrt(r^T W^-1 r).
    /// This is the minimum kinetic norm needed to realize J delta_v = r.
    pub fn residual_norm(
        &self,
        residual: &ConstraintResidualV1<Space, K>,
    ) -> Result<f64, MetricErrorV1> {
        let residual_coordinates = Cotangent::<Space, K>::new(residual.components)?;
        self.metric.cotangent_norm(&residual_coordinates)
    }

    /// delta_v = M^-1 J^T lambda.
    pub fn induced_velocity_correction(
        &self,
        multiplier: &ConstraintMultiplierV1<Space, K>,
    ) -> Result<Tangent<Frame, N>, MetricErrorV1> {
        let mut generalized_force = [0.0; N];
        for (constraint, row) in self.jacobian.iter().enumerate() {
            for (dof, force) in generalized_force.iter_mut().enumerate() {
                *force += row[dof] * multiplier.components[constraint];
                if !force.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
            }
        }
        self.mass
            .raise(&Cotangent::<Frame, N>::new(generalized_force)?)
    }

    /// Compute J delta_v in the retained physical constraint map.
    pub fn apply_jacobian(
        &self,
        velocity: &Tangent<Frame, N>,
    ) -> Result<ConstraintResidualV1<Space, K>, MetricErrorV1> {
        let mut residual = [0.0; K];
        for (row_index, row) in self.jacobian.iter().enumerate() {
            let mut value = 0.0;
            for (dof, component) in row.iter().zip(velocity.components().iter()) {
                value += *dof * *component;
            }
            if !value.is_finite() {
                return Err(MetricErrorV1::NonFiniteIntermediate);
            }
            residual[row_index] = value;
        }
        ConstraintResidualV1::new(residual)
    }

    /// Return the minimum-kinetic-norm delta_v satisfying J delta_v = residual.
    /// Full row rank is enforced by SPD admission of W.
    pub fn minimum_kinetic_correction(
        &self,
        residual: &ConstraintResidualV1<Space, K>,
    ) -> Result<Tangent<Frame, N>, MetricErrorV1> {
        let residual_as_covector = Cotangent::<Space, K>::new(residual.components)?;
        let multiplier = self.metric.raise(&residual_as_covector)?;
        let multiplier = ConstraintMultiplierV1::<Space, K>::new(*multiplier.components())?;
        self.induced_velocity_correction(&multiplier)
    }
}

/// A nonsingular change of constraint-equation basis.
///
/// The represented map is r_new = T r_old and J_new = T J_old.
/// Multiplier covectors transform by the inverse transpose:
/// lambda_new = T^-T lambda_old.
///
/// This is independent of a generalized-coordinate Rebasis.
#[derive(Debug, Clone, Copy)]
pub struct ConstraintEquationRebasisV1<
    NewSpace: CoordinateFrame,
    OldSpace: CoordinateFrame,
    const K: usize,
> {
    rebasis: Rebasis<NewSpace, OldSpace, K>,
}

impl<NewSpace: CoordinateFrame, OldSpace: CoordinateFrame, const K: usize>
    ConstraintEquationRebasisV1<NewSpace, OldSpace, K>
{
    /// Construct from the constraint residual map r_new = T r_old.
    pub fn new(transform: [[f64; K]; K], profile: MetricProfileV1) -> Result<Self, MetricErrorV1> {
        Ok(Self {
            rebasis: Rebasis::new(transform, profile)?,
        })
    }

    pub fn matrix(&self) -> &[[f64; K]; K] {
        self.rebasis.matrix()
    }

    pub fn condition_inf_estimate(&self) -> Result<f64, MetricErrorV1> {
        self.rebasis.condition_inf_estimate()
    }

    /// Residual pushforward: r_new = T r_old.
    pub fn residual_to_new(
        &self,
        residual_old: &ConstraintResidualV1<OldSpace, K>,
    ) -> Result<ConstraintResidualV1<NewSpace, K>, MetricErrorV1> {
        let old_tangent = Tangent::<OldSpace, K>::new(residual_old.components)?;
        let new_tangent = self.rebasis.tangent_to_from(&old_tangent)?;
        ConstraintResidualV1::new(*new_tangent.components())
    }

    /// Multiplier pullback: lambda_new = T^-T lambda_old.
    pub fn multiplier_to_new(
        &self,
        multiplier_old: &ConstraintMultiplierV1<OldSpace, K>,
        profile: MetricProfileV1,
    ) -> Result<ConstraintMultiplierV1<NewSpace, K>, MetricErrorV1> {
        let old_cotangent = Cotangent::<OldSpace, K>::new(multiplier_old.components)?;
        let new_cotangent = self.rebasis.cotangent_to_from(&old_cotangent, profile)?;
        ConstraintMultiplierV1::new(*new_cotangent.components())
    }

    /// Constraint Jacobian left action: J_new = T J_old.
    pub fn jacobian_to_new<const N: usize>(
        &self,
        jacobian_old: &[[f64; N]; K],
    ) -> Result<[[f64; N]; K], MetricErrorV1> {
        if K == 0 || N == 0 {
            return Err(MetricErrorV1::ZeroDimension);
        }
        let mut jacobian_new = [[0.0; N]; K];
        for (new_row, output_row) in jacobian_new.iter_mut().enumerate() {
            for (dof, output) in output_row.iter_mut().enumerate() {
                let mut value = 0.0;
                for (old_row, source_row) in jacobian_old.iter().enumerate() {
                    value += self.rebasis.matrix()[new_row][old_row] * source_row[dof];
                }
                if !value.is_finite() {
                    return Err(MetricErrorV1::NonFiniteIntermediate);
                }
                *output = value;
            }
        }
        Ok(jacobian_new)
    }
}

/// Dual pairing between a constraint multiplier and constraint residual.
pub fn constraint_pairing<Space: CoordinateFrame, const K: usize>(
    multiplier: &ConstraintMultiplierV1<Space, K>,
    residual: &ConstraintResidualV1<Space, K>,
) -> Result<f64, MetricErrorV1> {
    let value = multiplier
        .components
        .iter()
        .zip(residual.components.iter())
        .map(|(left, right)| left * right)
        .sum::<f64>();
    if value.is_finite() {
        Ok(value)
    } else {
        Err(MetricErrorV1::NonFiniteIntermediate)
    }
}

/// Rebase a constraint Jacobian for q = S y: J_y = J_q S.
/// Constraint coordinates themselves are unchanged.
pub fn rebase_constraint_jacobian<
    From: CoordinateFrame,
    To: CoordinateFrame,
    const N: usize,
    const K: usize,
>(
    jacobian_from: &[[f64; N]; K],
    rebasis: &Rebasis<From, To, N>,
) -> Result<[[f64; N]; K], MetricErrorV1> {
    let mut jacobian_to = [[0.0; N]; K];
    for (row_index, row) in jacobian_from.iter().enumerate() {
        let covector = Cotangent::<From, N>::new(*row)?;
        jacobian_to[row_index] = *rebasis.cotangent_to_to(&covector)?.components();
    }
    Ok(jacobian_to)
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

    fn profile() -> MetricProfileV1 {
        MetricProfileV1::default()
    }

    fn mass_q() -> Metric<CartesianQ, 3> {
        Metric::admit(
            [[2.0, 0.0, 0.0], [0.0, 3.0, 0.0], [0.0, 0.0, 5.0]],
            profile(),
        )
        .unwrap()
    }

    fn rebasis() -> Rebasis<CartesianQ, DenseY, 3> {
        Rebasis::new(
            [[1.0, 2.0, 1.0], [0.0, 1.0, 3.0], [0.0, 0.0, 1.0]],
            profile(),
        )
        .unwrap()
    }

    fn jacobian_q() -> [[f64; 3]; 2] {
        [[1.0, 0.0, 1.0], [0.0, 2.0, -1.0]]
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        let scale = 1.0_f64.max(left.abs()).max(right.abs());
        assert!(
            (left - right).abs() <= tolerance * scale,
            "{left:.17e} != {right:.17e}"
        );
    }

    fn congruence_2(
        transform: &[[f64; 2]; 2],
        metric: &[[f64; 2]; 2],
        row: usize,
        column: usize,
    ) -> f64 {
        transform[row][0]
            * (metric[0][0] * transform[column][0] + metric[0][1] * transform[column][1])
            + transform[row][1]
                * (metric[1][0] * transform[column][0] + metric[1][1] * transform[column][1])
    }

    #[test]
    fn delassus_operator_is_invariant_under_dense_generalized_coordinate_rebasing() {
        let mass_q = mass_q();
        let rebasis = rebasis();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_q = jacobian_q();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();

        let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q,
            jacobian_q,
            profile(),
        )
        .unwrap();
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y,
            jacobian_y,
            profile(),
        )
        .unwrap();

        for row in 0..2 {
            for column in 0..2 {
                close(
                    delassus_q.matrix()[row][column],
                    delassus_y.matrix()[row][column],
                    2.0e-12,
                );
            }
        }
    }

    #[test]
    fn whitened_jacobian_reconstructs_delassus_and_exact_diagonal() {
        let mass = mass_q();
        let jacobian = jacobian_q();
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass,
            jacobian,
            profile(),
        )
        .unwrap();

        for row in 0..2 {
            for column in 0..2 {
                let gram = crate::dot(
                    &delassus.whitened_jacobian()[row],
                    &delassus.whitened_jacobian()[column],
                );
                close(gram, delassus.matrix()[row][column], 2.0e-13);
            }
        }

        // Independent formulation: solve M x = J_j^T, then evaluate J_i x.
        for row in 0..2 {
            for column in 0..2 {
                let column_covector = Cotangent::<CartesianQ, 3>::new(jacobian[column]).unwrap();
                let raised_column = mass.raise(&column_covector).unwrap();
                let row_covector = Cotangent::<CartesianQ, 3>::new(jacobian[row]).unwrap();
                close(
                    delassus.matrix()[row][column],
                    crate::pairing(&row_covector, &raised_column),
                    2.0e-13,
                );
            }
        }
    }

    #[test]
    fn multiplier_norm_equals_kinetic_norm_of_induced_velocity_correction() {
        let mass = mass_q();
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass,
            jacobian_q(),
            profile(),
        )
        .unwrap();
        let multiplier = ConstraintMultiplierV1::new([0.75, -1.25]).unwrap();
        let correction = delassus.induced_velocity_correction(&multiplier).unwrap();

        close(
            delassus.multiplier_norm(&multiplier).unwrap(),
            mass.tangent_norm(&correction).unwrap(),
            2.0e-13,
        );
    }

    #[test]
    fn residual_norm_equals_minimum_kinetic_correction_and_reconstructs_residual() {
        let delassus = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q(),
            jacobian_q(),
            profile(),
        )
        .unwrap();
        let residual = ConstraintResidualV1::new([0.25, -0.4]).unwrap();
        let correction = delassus.minimum_kinetic_correction(&residual).unwrap();
        let reconstructed = delassus.apply_jacobian(&correction).unwrap();

        for index in 0..2 {
            close(
                residual.components()[index],
                reconstructed.components()[index],
                2.0e-12,
            );
        }
        close(
            delassus.residual_norm(&residual).unwrap(),
            mass_q().tangent_norm(&correction).unwrap(),
            2.0e-12,
        );
    }

    #[test]
    fn multiplier_and_residual_norms_are_invariant_under_coordinate_rebasing() {
        let mass_q = mass_q();
        let rebasis = rebasis();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_q = jacobian_q();
        let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();
        let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q,
            jacobian_q,
            profile(),
        )
        .unwrap();
        let delassus_y = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y,
            jacobian_y,
            profile(),
        )
        .unwrap();

        let multiplier = ConstraintMultiplierV1::new([1.25, -0.75]).unwrap();
        let residual = ConstraintResidualV1::new([-0.3, 0.6]).unwrap();
        close(
            delassus_q.multiplier_norm(&multiplier).unwrap(),
            delassus_y.multiplier_norm(&multiplier).unwrap(),
            2.0e-12,
        );
        close(
            delassus_q.residual_norm(&residual).unwrap(),
            delassus_y.residual_norm(&residual).unwrap(),
            2.0e-12,
        );
    }

    #[test]
    fn wrong_jacobian_variance_law_changes_delassus_operator() {
        let mass_q = mass_q();
        let rebasis = rebasis();
        let mass_y = rebasis.metric_to_to(&mass_q, profile()).unwrap();
        let jacobian_q = jacobian_q();
        let correct = rebase_constraint_jacobian(&jacobian_q, &rebasis).unwrap();

        let mut wrong = [[0.0; 3]; 2];
        for (row, wrong_row) in wrong.iter_mut().enumerate() {
            for (column, value) in wrong_row.iter_mut().enumerate() {
                for (inner, jacobian_value) in jacobian_q[row].iter().enumerate() {
                    *value += jacobian_value * rebasis.matrix()[column][inner];
                }
            }
        }

        let correct_w = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y,
            correct,
            profile(),
        )
        .unwrap();
        let wrong_w = DelassusMetricV1::<DenseY, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_y,
            wrong,
            profile(),
        )
        .unwrap();

        let maximum_difference = (0..2)
            .flat_map(|row| (0..2).map(move |column| (row, column)))
            .map(|(row, column)| {
                (correct_w.matrix()[row][column] - wrong_w.matrix()[row][column]).abs()
            })
            .fold(0.0_f64, f64::max);
        assert!(maximum_difference > 1.0e-3);
    }

    #[test]
    fn redundant_constraint_rows_are_rejected_as_singular_delassus_metric() {
        let mass = mass_q();
        let redundant = [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0]];
        let result = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass,
            redundant,
            profile(),
        );
        assert!(matches!(
            result,
            Err(MetricErrorV1::NotPositiveDefinite { .. })
        ));
    }

    #[test]
    fn dense_constraint_equation_rebasis_preserves_delassus_dual_pair() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceNew;
        impl CoordinateFrame for ConstraintSpaceNew {}

        let mass = mass_q();
        let jacobian_old = jacobian_q();
        let delassus_old =
            DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_old,
                profile(),
            )
            .unwrap();
        let equation_rebasis =
            ConstraintEquationRebasisV1::<ConstraintSpaceNew, ConstraintSpace, 2>::new(
                [[1.0, 3.0], [0.0, 1.0]],
                profile(),
            )
            .unwrap();
        let jacobian_new = equation_rebasis.jacobian_to_new(&jacobian_old).unwrap();
        let delassus_new =
            DelassusMetricV1::<CartesianQ, ConstraintSpaceNew, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_new,
                profile(),
            )
            .unwrap();

        let multiplier_old = ConstraintMultiplierV1::new([0.75, -1.25]).unwrap();
        let residual_old = ConstraintResidualV1::new([0.25, -0.4]).unwrap();
        let multiplier_new = equation_rebasis
            .multiplier_to_new(&multiplier_old, profile())
            .unwrap();
        let residual_new = equation_rebasis.residual_to_new(&residual_old).unwrap();

        let transform = equation_rebasis.matrix();
        let old_metric = delassus_old.matrix();
        let new_metric = delassus_new.matrix();
        close(
            new_metric[0][0],
            congruence_2(transform, old_metric, 0, 0),
            2.0e-12,
        );
        close(
            new_metric[0][1],
            congruence_2(transform, old_metric, 0, 1),
            2.0e-12,
        );
        close(
            new_metric[1][0],
            congruence_2(transform, old_metric, 1, 0),
            2.0e-12,
        );
        close(
            new_metric[1][1],
            congruence_2(transform, old_metric, 1, 1),
            2.0e-12,
        );

        close(
            delassus_old.multiplier_norm(&multiplier_old).unwrap(),
            delassus_new.multiplier_norm(&multiplier_new).unwrap(),
            2.0e-12,
        );
        close(
            delassus_old.residual_norm(&residual_old).unwrap(),
            delassus_new.residual_norm(&residual_new).unwrap(),
            2.0e-12,
        );
        close(
            constraint_pairing(&multiplier_old, &residual_old).unwrap(),
            constraint_pairing(&multiplier_new, &residual_new).unwrap(),
            2.0e-12,
        );

        let correction_old = delassus_old
            .induced_velocity_correction(&multiplier_old)
            .unwrap();
        let correction_new = delassus_new
            .induced_velocity_correction(&multiplier_new)
            .unwrap();
        for (left, right) in correction_old
            .components()
            .iter()
            .zip(correction_new.components().iter())
        {
            close(*left, *right, 2.0e-12);
        }

        let minimum_old = delassus_old
            .minimum_kinetic_correction(&residual_old)
            .unwrap();
        let minimum_new = delassus_new
            .minimum_kinetic_correction(&residual_new)
            .unwrap();
        for (left, right) in minimum_old
            .components()
            .iter()
            .zip(minimum_new.components().iter())
        {
            close(*left, *right, 2.0e-12);
        }
    }

    #[test]
    fn constraint_basis_rebasis_is_invariant_over_a_deterministic_shear_family() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceSheared;
        impl CoordinateFrame for ConstraintSpaceSheared {}

        let mass = mass_q();
        let jacobian_old = jacobian_q();
        let delassus_old =
            DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_old,
                profile(),
            )
            .unwrap();
        let multiplier_old = ConstraintMultiplierV1::new([-0.5, 1.25]).unwrap();
        let residual_old = ConstraintResidualV1::new([0.3, -0.8]).unwrap();
        let norm_multiplier_old = delassus_old.multiplier_norm(&multiplier_old).unwrap();
        let norm_residual_old = delassus_old.residual_norm(&residual_old).unwrap();
        let pairing_old = constraint_pairing(&multiplier_old, &residual_old).unwrap();

        for shear in -8..=8 {
            let change =
                ConstraintEquationRebasisV1::<ConstraintSpaceSheared, ConstraintSpace, 2>::new(
                    [[1.0, shear as f64], [0.0, 1.0]],
                    profile(),
                )
                .unwrap();
            let jacobian_new = change.jacobian_to_new(&jacobian_old).unwrap();
            let delassus_new =
                DelassusMetricV1::<CartesianQ, ConstraintSpaceSheared, 3, 2>::from_mass_jacobian(
                    &mass,
                    jacobian_new,
                    profile(),
                )
                .unwrap();
            let multiplier_new = change
                .multiplier_to_new(&multiplier_old, profile())
                .unwrap();
            let residual_new = change.residual_to_new(&residual_old).unwrap();

            close(
                delassus_new.multiplier_norm(&multiplier_new).unwrap(),
                norm_multiplier_old,
                3.0e-11,
            );
            close(
                delassus_new.residual_norm(&residual_new).unwrap(),
                norm_residual_old,
                3.0e-11,
            );
            close(
                constraint_pairing(&multiplier_new, &residual_new).unwrap(),
                pairing_old,
                3.0e-11,
            );
        }
    }

    #[test]
    fn wrong_constraint_multiplier_or_residual_variance_is_detected() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceNew;
        impl CoordinateFrame for ConstraintSpaceNew {}

        let mass = mass_q();
        let jacobian_old = jacobian_q();
        let rebasis = ConstraintEquationRebasisV1::<ConstraintSpaceNew, ConstraintSpace, 2>::new(
            [[1.0, 2.0], [0.0, 1.0]],
            profile(),
        )
        .unwrap();
        let jacobian_new = rebasis.jacobian_to_new(&jacobian_old).unwrap();
        let delassus_new =
            DelassusMetricV1::<CartesianQ, ConstraintSpaceNew, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_new,
                profile(),
            )
            .unwrap();
        let multiplier_old = ConstraintMultiplierV1::new([0.8, -0.4]).unwrap();
        let residual_old = ConstraintResidualV1::new([0.3, 1.1]).unwrap();
        let multiplier_correct = rebasis
            .multiplier_to_new(&multiplier_old, profile())
            .unwrap();
        let residual_correct = rebasis.residual_to_new(&residual_old).unwrap();

        // Incorrectly treating the multiplier as a tangent applies T, not T^-T.
        let lambda = multiplier_old.components();
        let multiplier_wrong = ConstraintMultiplierV1::<ConstraintSpaceNew, 2>::new([
            lambda[0] + 2.0 * lambda[1],
            lambda[1],
        ])
        .unwrap();
        // Incorrectly treating the residual as a covector applies T^-T, not T.
        let residual = residual_old.components();
        let residual_wrong = ConstraintResidualV1::<ConstraintSpaceNew, 2>::new([
            residual[0],
            -2.0 * residual[0] + residual[1],
        ])
        .unwrap();

        let correct_multiplier_norm = delassus_new.multiplier_norm(&multiplier_correct).unwrap();
        let wrong_multiplier_norm = delassus_new.multiplier_norm(&multiplier_wrong).unwrap();
        let correct_residual_norm = delassus_new.residual_norm(&residual_correct).unwrap();
        let wrong_residual_norm = delassus_new.residual_norm(&residual_wrong).unwrap();

        assert!((correct_multiplier_norm - wrong_multiplier_norm).abs() > 1.0e-3);
        assert!((correct_residual_norm - wrong_residual_norm).abs() > 1.0e-3);
    }

    #[test]
    fn generalized_and_constraint_rebases_commute_for_delassus_physics() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceNew;
        impl CoordinateFrame for ConstraintSpaceNew {}

        let mass_q = mass_q();
        let jacobian_q = jacobian_q();
        let delassus_q = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q,
            jacobian_q,
            profile(),
        )
        .unwrap();
        let multiplier_q = ConstraintMultiplierV1::new([1.25, -0.6]).unwrap();
        let residual_q = ConstraintResidualV1::new([-0.2, 0.75]).unwrap();

        for shear_q in [1.0, 2.0, 4.0] {
            let generalized_rebasis = Rebasis::<CartesianQ, DenseY, 3>::new(
                [
                    [1.0, shear_q, 0.5 * shear_q * shear_q],
                    [0.0, 1.0, shear_q],
                    [0.0, 0.0, 1.0],
                ],
                profile(),
            )
            .unwrap();
            let mass_y = generalized_rebasis
                .metric_to_to(&mass_q, profile())
                .unwrap();
            let jacobian_y = rebase_constraint_jacobian(&jacobian_q, &generalized_rebasis).unwrap();

            for shear_constraint in [1.0, 3.0, 5.0] {
                let constraint_rebasis =
                    ConstraintEquationRebasisV1::<ConstraintSpaceNew, ConstraintSpace, 2>::new(
                        [[1.0, shear_constraint], [0.0, 1.0]],
                        profile(),
                    )
                    .unwrap();
                let jacobian_y_new = constraint_rebasis.jacobian_to_new(&jacobian_y).unwrap();
                let delassus_y_new =
                    DelassusMetricV1::<DenseY, ConstraintSpaceNew, 3, 2>::from_mass_jacobian(
                        &mass_y,
                        jacobian_y_new,
                        profile(),
                    )
                    .unwrap();

                let multiplier_new = constraint_rebasis
                    .multiplier_to_new(&multiplier_q, profile())
                    .unwrap();
                let residual_new = constraint_rebasis.residual_to_new(&residual_q).unwrap();
                close(
                    delassus_q.multiplier_norm(&multiplier_q).unwrap(),
                    delassus_y_new.multiplier_norm(&multiplier_new).unwrap(),
                    5.0e-10,
                );
                close(
                    delassus_q.residual_norm(&residual_q).unwrap(),
                    delassus_y_new.residual_norm(&residual_new).unwrap(),
                    5.0e-10,
                );

                let correction_q = delassus_q.minimum_kinetic_correction(&residual_q).unwrap();
                let correction_y = delassus_y_new
                    .minimum_kinetic_correction(&residual_new)
                    .unwrap();
                let lifted_correction_y =
                    generalized_rebasis.tangent_to_from(&correction_y).unwrap();
                for (left, right) in correction_q
                    .components()
                    .iter()
                    .zip(lifted_correction_y.components().iter())
                {
                    close(*left, *right, 8.0e-10);
                }
            }
        }
    }

    #[test]
    fn constraint_equation_rebasis_preserves_physics_under_row_rescaling() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceScaled;
        impl CoordinateFrame for ConstraintSpaceScaled {}

        let mass = mass_q();
        let jacobian_old = jacobian_q();
        let delassus_old =
            DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
                &mass,
                jacobian_old,
                profile(),
            )
            .unwrap();
        let multiplier_old = ConstraintMultiplierV1::new([0.75, -1.25]).unwrap();
        let residual_old = ConstraintResidualV1::new([0.25, -0.4]).unwrap();

        for scale in [0.01_f64, 0.1, 0.5, 1.0, 2.0, 10.0, 100.0] {
            let change =
                ConstraintEquationRebasisV1::<ConstraintSpaceScaled, ConstraintSpace, 2>::new(
                    [[scale, 0.0], [0.0, 1.0]],
                    profile(),
                )
                .unwrap();
            let jacobian_new = change.jacobian_to_new(&jacobian_old).unwrap();
            let delassus_new =
                DelassusMetricV1::<CartesianQ, ConstraintSpaceScaled, 3, 2>::from_mass_jacobian(
                    &mass,
                    jacobian_new,
                    profile(),
                )
                .unwrap();
            let multiplier_new = change
                .multiplier_to_new(&multiplier_old, profile())
                .unwrap();
            let residual_new = change.residual_to_new(&residual_old).unwrap();

            close(
                delassus_old.multiplier_norm(&multiplier_old).unwrap(),
                delassus_new.multiplier_norm(&multiplier_new).unwrap(),
                3.0e-10,
            );
            close(
                delassus_old.residual_norm(&residual_old).unwrap(),
                delassus_new.residual_norm(&residual_new).unwrap(),
                3.0e-10,
            );
            close(
                constraint_pairing(&multiplier_old, &residual_old).unwrap(),
                constraint_pairing(&multiplier_new, &residual_new).unwrap(),
                3.0e-10,
            );

            let correction_old = delassus_old
                .minimum_kinetic_correction(&residual_old)
                .unwrap();
            let correction_new = delassus_new
                .minimum_kinetic_correction(&residual_new)
                .unwrap();
            for (left, right) in correction_old
                .components()
                .iter()
                .zip(correction_new.components().iter())
            {
                close(*left, *right, 3.0e-10);
            }
        }
    }

    #[test]
    fn near_singular_constraint_equation_scaling_is_rejected_by_profile() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceNearSingular;
        impl CoordinateFrame for ConstraintSpaceNearSingular {}

        let result =
            ConstraintEquationRebasisV1::<ConstraintSpaceNearSingular, ConstraintSpace, 2>::new(
                [[1.0e-18, 0.0], [0.0, 1.0]],
                profile(),
            );
        assert!(matches!(
            result,
            Err(MetricErrorV1::SingularCoordinateTransform { .. })
        ));
    }

    #[test]
    fn singular_constraint_equation_rebasis_fails_closed() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct ConstraintSpaceNew;
        impl CoordinateFrame for ConstraintSpaceNew {}

        let result = ConstraintEquationRebasisV1::<ConstraintSpaceNew, ConstraintSpace, 2>::new(
            [[1.0, 2.0], [2.0, 4.0]],
            profile(),
        );
        assert!(matches!(
            result,
            Err(MetricErrorV1::SingularCoordinateTransform { .. })
        ));
    }

    #[test]
    fn nonfinite_constraint_jacobian_is_rejected() {
        let result = DelassusMetricV1::<CartesianQ, ConstraintSpace, 3, 2>::from_mass_jacobian(
            &mass_q(),
            [[1.0, f64::NAN, 0.0], [0.0, 1.0, 0.0]],
            profile(),
        );
        assert_eq!(result, Err(MetricErrorV1::NonFiniteVector));
    }
}
