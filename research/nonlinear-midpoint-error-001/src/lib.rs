// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! A bounded Newton-Kantorovich reference for a nonlinear holonomic midpoint KKT step.
//!
//! This is a dimensionless one-degree-of-freedom fixture. It is deliberately
//! separate from the production dumbbell integrator. The Jacobian-Lipschitz
//! witness is analytically derived for this polynomial fixture; the inverse
//! norm is a floating-point estimate, so the returned radius is an estimate,
//! not a formally certified interval enclosure.

use std::fmt;

pub const KKT_DIMENSION: usize = 3;
pub type Vector3 = [f64; KKT_DIMENSION];
pub type Matrix3 = [[f64; KKT_DIMENSION]; KKT_DIMENSION];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidpointParametersV1 {
    pub q0: f64,
    pub v0: f64,
    pub time_step: f64,
    pub mass: f64,
    pub applied_force: f64,
}

impl MidpointParametersV1 {
    pub fn dimensionless_fixture_default() -> Self {
        Self {
            q0: 0.0,
            v0: 0.0,
            time_step: 1.0,
            mass: 1.0,
            applied_force: 0.0,
        }
    }

    fn validate(self) -> Result<(), KktErrorV1> {
        let values = [
            self.q0,
            self.v0,
            self.time_step,
            self.mass,
            self.applied_force,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(KktErrorV1::NonFiniteInput);
        }
        if self.time_step <= 0.0 || self.mass <= 0.0 {
            return Err(KktErrorV1::InvalidPhysicalParameter);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MidpointKktStateV1 {
    pub q1: f64,
    pub v1: f64,
    pub multiplier: f64,
}

impl MidpointKktStateV1 {
    pub fn components(self) -> Vector3 {
        [self.q1, self.v1, self.multiplier]
    }

    pub fn known_positive_branch_root(parameters: MidpointParametersV1) -> Self {
        let q1 = 2.0 - parameters.q0;
        let v1 = 2.0 * (q1 - parameters.q0) / parameters.time_step - parameters.v0;
        let q_mid = 0.5 * (parameters.q0 + q1);
        let multiplier = (parameters.mass * (v1 - parameters.v0)
            - parameters.time_step * parameters.applied_force)
            / (2.0 * q_mid);
        Self { q1, v1, multiplier }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JacobianLipschitzWitnessV1 {
    pub upper_bound: f64,
    pub derivation_id: &'static str,
}

impl JacobianLipschitzWitnessV1 {
    /// Analytic Frobenius/operator bound for this exact polynomial KKT fixture.
    ///
    /// The nonconstant Jacobian entries vary by (-delta_lambda),
    /// (-delta_q1), and (delta_q1 / 2). Hence
    /// ||J(z)-J(w)||_F^2 <= delta_lambda^2 + 1.25 delta_q1^2
    /// <= 1.25 ||z-w||_2^2.
    pub fn for_quadratic_midpoint_fixture() -> Self {
        Self {
            upper_bound: 1.25_f64.sqrt(),
            derivation_id: "quadratic-holonomic-midpoint-jacobian-frobenius-v1",
        }
    }

    pub fn from_claimed_upper_bound(upper_bound: f64) -> Result<Self, KktErrorV1> {
        let analytic = Self::for_quadratic_midpoint_fixture();
        if !upper_bound.is_finite() || upper_bound < analytic.upper_bound {
            return Err(KktErrorV1::UnderstatedJacobianLipschitzBound);
        }
        Ok(Self {
            upper_bound,
            derivation_id: analytic.derivation_id,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KantorovichProfileV1 {
    pub maximum_kantorovich_product: f64,
    pub maximum_radius: f64,
    pub inverse_norm_relative_inflation: f64,
    pub pivot_relative_tolerance: f64,
}

impl Default for KantorovichProfileV1 {
    fn default() -> Self {
        Self {
            maximum_kantorovich_product: 0.5,
            maximum_radius: 0.25,
            inverse_norm_relative_inflation: 1.0e-10,
            pivot_relative_tolerance: 256.0 * f64::EPSILON,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconclusiveReasonV1 {
    SingularKktJacobian,
    KantorovichProductTooLarge,
    CandidateBallLeavesPositiveConstraintBranch,
    CandidateRadiusExceedsProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualificationStatusV1 {
    AdmissibleLocalRootEstimate,
    Inconclusive(InconclusiveReasonV1),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KantorovichReceiptV1 {
    pub residual_norm: f64,
    pub newton_correction_norm: f64,
    pub inverse_jacobian_norm_upper_estimate: f64,
    pub jacobian_lipschitz_upper_bound: f64,
    pub kantorovich_product: f64,
    pub local_radius_estimate: f64,
    pub distance_to_constraint_branch_boundary: f64,
    pub status: QualificationStatusV1,
    pub derivation_id: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KktErrorV1 {
    NonFiniteInput,
    InvalidPhysicalParameter,
    InvalidProfile,
    UnderstatedJacobianLipschitzBound,
    NonFiniteIntermediate,
    SingularLinearization,
}

impl fmt::Display for KktErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for KktErrorV1 {}

pub fn midpoint_kkt_residual_v1(
    parameters: MidpointParametersV1,
    state: MidpointKktStateV1,
) -> Result<Vector3, KktErrorV1> {
    parameters.validate()?;
    if state.components().iter().any(|value| !value.is_finite()) {
        return Err(KktErrorV1::NonFiniteInput);
    }
    let q_mid = 0.5 * (parameters.q0 + state.q1);
    let residual = [
        2.0 * (state.q1 - parameters.q0) - parameters.time_step * (parameters.v0 + state.v1),
        parameters.mass * (state.v1 - parameters.v0)
            - parameters.time_step * parameters.applied_force
            - 2.0 * q_mid * state.multiplier,
        q_mid * q_mid - 1.0,
    ];
    if residual.iter().any(|value| !value.is_finite()) {
        return Err(KktErrorV1::NonFiniteIntermediate);
    }
    Ok(residual)
}

pub fn midpoint_kkt_jacobian_v1(
    parameters: MidpointParametersV1,
    state: MidpointKktStateV1,
) -> Result<Matrix3, KktErrorV1> {
    parameters.validate()?;
    if state.components().iter().any(|value| !value.is_finite()) {
        return Err(KktErrorV1::NonFiniteInput);
    }
    let q_mid = 0.5 * (parameters.q0 + state.q1);
    Ok([
        [2.0, -parameters.time_step, 0.0],
        [-state.multiplier, parameters.mass, -2.0 * q_mid],
        [q_mid, 0.0, 0.0],
    ])
}

fn vector_norm(values: &Vector3) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn matrix_frobenius(matrix: &Matrix3) -> f64 {
    matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn matrix_inf(matrix: &Matrix3) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0, f64::max)
}

fn inverse_3x3(matrix: &Matrix3, relative_pivot_tolerance: f64) -> Result<Matrix3, KktErrorV1> {
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(KktErrorV1::NonFiniteIntermediate);
    }
    let scale = matrix_inf(matrix);
    if !scale.is_finite() || scale == 0.0 {
        return Err(KktErrorV1::SingularLinearization);
    }
    let pivot_threshold = relative_pivot_tolerance * scale;
    let mut left = *matrix;
    let mut right = [[0.0; KKT_DIMENSION]; KKT_DIMENSION];
    for (index, row) in right.iter_mut().enumerate() {
        row[index] = 1.0;
    }

    for column in 0..KKT_DIMENSION {
        let pivot_row = (column..KKT_DIMENSION)
            .max_by(|left_row, right_row| {
                left[*left_row][column]
                    .abs()
                    .total_cmp(&left[*right_row][column].abs())
            })
            .ok_or(KktErrorV1::SingularLinearization)?;
        let pivot_magnitude = left[pivot_row][column].abs();
        if !pivot_magnitude.is_finite() || pivot_magnitude <= pivot_threshold {
            return Err(KktErrorV1::SingularLinearization);
        }
        if pivot_row != column {
            left.swap(pivot_row, column);
            right.swap(pivot_row, column);
        }

        let pivot = left[column][column];
        for value in left[column].iter_mut() {
            *value /= pivot;
        }
        for value in right[column].iter_mut() {
            *value /= pivot;
        }

        for row in 0..KKT_DIMENSION {
            if row != column {
                let factor = left[row][column];
                for entry in 0..KKT_DIMENSION {
                    left[row][entry] -= factor * left[column][entry];
                    right[row][entry] -= factor * right[column][entry];
                }
            }
        }
    }

    if right.iter().flatten().any(|value| !value.is_finite()) {
        return Err(KktErrorV1::NonFiniteIntermediate);
    }
    Ok(right)
}

fn matrix_vector(matrix: &Matrix3, vector: &Vector3) -> Vector3 {
    std::array::from_fn(|row| {
        matrix[row]
            .iter()
            .zip(vector.iter())
            .map(|(a, b)| a * b)
            .sum()
    })
}

pub fn qualify_midpoint_state_v1(
    parameters: MidpointParametersV1,
    approximate_state: MidpointKktStateV1,
    lipschitz: JacobianLipschitzWitnessV1,
    profile: KantorovichProfileV1,
) -> Result<KantorovichReceiptV1, KktErrorV1> {
    parameters.validate()?;
    if !profile.maximum_kantorovich_product.is_finite()
        || profile.maximum_kantorovich_product <= 0.0
        || profile.maximum_kantorovich_product > 0.5
        || !profile.maximum_radius.is_finite()
        || profile.maximum_radius <= 0.0
        || !profile.inverse_norm_relative_inflation.is_finite()
        || profile.inverse_norm_relative_inflation < 0.0
        || !profile.pivot_relative_tolerance.is_finite()
        || profile.pivot_relative_tolerance <= 0.0
    {
        return Err(KktErrorV1::InvalidProfile);
    }
    let analytic_witness =
        JacobianLipschitzWitnessV1::from_claimed_upper_bound(lipschitz.upper_bound)?;

    let residual = midpoint_kkt_residual_v1(parameters, approximate_state)?;
    let jacobian = midpoint_kkt_jacobian_v1(parameters, approximate_state)?;
    let inverse = match inverse_3x3(&jacobian, profile.pivot_relative_tolerance) {
        Ok(inverse) => inverse,
        Err(KktErrorV1::SingularLinearization) => {
            return Ok(KantorovichReceiptV1 {
                residual_norm: vector_norm(&residual),
                newton_correction_norm: f64::INFINITY,
                inverse_jacobian_norm_upper_estimate: f64::INFINITY,
                jacobian_lipschitz_upper_bound: analytic_witness.upper_bound,
                kantorovich_product: f64::INFINITY,
                local_radius_estimate: f64::INFINITY,
                distance_to_constraint_branch_boundary: 0.0,
                status: QualificationStatusV1::Inconclusive(
                    InconclusiveReasonV1::SingularKktJacobian,
                ),
                derivation_id: analytic_witness.derivation_id,
            });
        }
        Err(error) => return Err(error),
    };

    let residual_norm = vector_norm(&residual);
    let correction = matrix_vector(&inverse, &residual);
    let newton_correction_norm = vector_norm(&correction);
    // The Frobenius norm upper-bounds the spectral norm. A small explicit
    // inflation reduces the chance of treating a rounded value as an upper
    // estimate; it does not turn the result into an interval proof.
    let inverse_jacobian_norm_upper_estimate =
        matrix_frobenius(&inverse) * (1.0 + profile.inverse_norm_relative_inflation);
    let gamma = inverse_jacobian_norm_upper_estimate * analytic_witness.upper_bound;
    let kantorovich_product = gamma * newton_correction_norm;
    if !residual_norm.is_finite()
        || !newton_correction_norm.is_finite()
        || !inverse_jacobian_norm_upper_estimate.is_finite()
        || !gamma.is_finite()
        || !kantorovich_product.is_finite()
    {
        return Err(KktErrorV1::NonFiniteIntermediate);
    }

    let q_mid = 0.5 * (parameters.q0 + approximate_state.q1);
    let distance_to_constraint_branch_boundary = 2.0 * q_mid;
    let status = if q_mid <= 0.0 {
        QualificationStatusV1::Inconclusive(
            InconclusiveReasonV1::CandidateBallLeavesPositiveConstraintBranch,
        )
    } else if kantorovich_product > profile.maximum_kantorovich_product
        || kantorovich_product >= 0.5
    {
        QualificationStatusV1::Inconclusive(InconclusiveReasonV1::KantorovichProductTooLarge)
    } else {
        QualificationStatusV1::AdmissibleLocalRootEstimate
    };

    let local_radius_estimate = if gamma == 0.0 {
        newton_correction_norm
    } else if kantorovich_product < 0.5 {
        (1.0 - (1.0 - 2.0 * kantorovich_product).sqrt()) / gamma
    } else {
        f64::INFINITY
    };

    let status = match status {
        QualificationStatusV1::AdmissibleLocalRootEstimate
            if local_radius_estimate >= distance_to_constraint_branch_boundary =>
        {
            QualificationStatusV1::Inconclusive(
                InconclusiveReasonV1::CandidateBallLeavesPositiveConstraintBranch,
            )
        }
        QualificationStatusV1::AdmissibleLocalRootEstimate
            if local_radius_estimate > profile.maximum_radius =>
        {
            QualificationStatusV1::Inconclusive(InconclusiveReasonV1::CandidateRadiusExceedsProfile)
        }
        other => other,
    };

    Ok(KantorovichReceiptV1 {
        residual_norm,
        newton_correction_norm,
        inverse_jacobian_norm_upper_estimate,
        jacobian_lipschitz_upper_bound: analytic_witness.upper_bound,
        kantorovich_product,
        local_radius_estimate,
        distance_to_constraint_branch_boundary,
        status,
        derivation_id: analytic_witness.derivation_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameters(mass: f64, time_step: f64) -> MidpointParametersV1 {
        MidpointParametersV1 {
            time_step,
            mass,
            ..MidpointParametersV1::dimensionless_fixture_default()
        }
    }

    fn approximate_state(mass: f64, time_step: f64) -> MidpointKktStateV1 {
        let epsilon = 2.0_f64.powi(-12);
        let lambda_root = 2.0 * mass / time_step;
        MidpointKktStateV1 {
            q1: 2.0 + epsilon,
            v1: 4.0 / time_step - epsilon / (2.0 * time_step),
            multiplier: lambda_root - epsilon * mass / (2.0 * time_step),
        }
    }

    fn close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left:.16e}, right={right:.16e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn analytic_jacobian_matches_finite_difference() {
        let problem = parameters(1.0, 1.0);
        let state = approximate_state(1.0, 1.0);
        let jacobian = midpoint_kkt_jacobian_v1(problem, state).unwrap();
        let base = midpoint_kkt_residual_v1(problem, state).unwrap();
        let components = state.components();
        let step = 1.0e-6;
        for column in 0..KKT_DIMENSION {
            let mut shifted = components;
            shifted[column] += step;
            let shifted_state = MidpointKktStateV1 {
                q1: shifted[0],
                v1: shifted[1],
                multiplier: shifted[2],
            };
            let residual = midpoint_kkt_residual_v1(problem, shifted_state).unwrap();
            for row in 0..KKT_DIMENSION {
                let finite_difference = (residual[row] - base[row]) / step;
                close(finite_difference, jacobian[row][column], 2.0e-6);
            }
        }
    }

    #[test]
    fn kantorovich_radius_contains_known_positive_branch_root() {
        let problem = parameters(1.0, 1.0);
        let approximate = approximate_state(1.0, 1.0);
        let root = MidpointKktStateV1::known_positive_branch_root(problem);
        let witness = JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture();
        let receipt = qualify_midpoint_state_v1(
            problem,
            approximate,
            witness,
            KantorovichProfileV1::default(),
        )
        .unwrap();

        assert_eq!(
            receipt.status,
            QualificationStatusV1::AdmissibleLocalRootEstimate
        );
        let observed_error = vector_norm(&std::array::from_fn(|index| {
            approximate.components()[index] - root.components()[index]
        }));
        assert!(observed_error <= receipt.local_radius_estimate);
        assert!(receipt.kantorovich_product < 0.5);
        assert!(receipt.local_radius_estimate < receipt.distance_to_constraint_branch_boundary);
    }

    #[test]
    fn analytic_jacobian_lipschitz_bound_covers_deterministic_pairs() {
        let parameters = parameters(2.0, 0.5);
        let first = MidpointKktStateV1 {
            q1: 2.1,
            v1: 7.9,
            multiplier: 3.8,
        };
        let witness = JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture();
        for delta in [
            [0.1, -0.2, 0.3],
            [-0.5, 0.0, 0.25],
            [0.0, 1.0, -0.75],
            [2.0, -1.0, 0.125],
        ] {
            let second = MidpointKktStateV1 {
                q1: first.q1 + delta[0],
                v1: first.v1 + delta[1],
                multiplier: first.multiplier + delta[2],
            };
            let first_jacobian = midpoint_kkt_jacobian_v1(parameters, first).unwrap();
            let second_jacobian = midpoint_kkt_jacobian_v1(parameters, second).unwrap();
            let difference: Matrix3 = std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    first_jacobian[row][column] - second_jacobian[row][column]
                })
            });
            let distance = vector_norm(&delta);
            assert!(matrix_frobenius(&difference) <= witness.upper_bound * distance + 1.0e-13);
        }
    }

    #[test]
    fn understated_jacobian_lipschitz_claim_is_rejected() {
        assert_eq!(
            JacobianLipschitzWitnessV1::from_claimed_upper_bound(1.0),
            Err(KktErrorV1::UnderstatedJacobianLipschitzBound)
        );
    }

    #[test]
    fn singular_kkt_linearization_is_inconclusive() {
        let problem = parameters(1.0, 1.0);
        let singular_state = MidpointKktStateV1 {
            q1: 0.0,
            v1: 0.0,
            multiplier: 0.0,
        };
        let receipt = qualify_midpoint_state_v1(
            problem,
            singular_state,
            JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture(),
            KantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            QualificationStatusV1::Inconclusive(InconclusiveReasonV1::SingularKktJacobian)
        );
    }

    #[test]
    fn wrong_constraint_branch_is_inconclusive() {
        let problem = parameters(1.0, 1.0);
        let negative_root = MidpointKktStateV1 {
            q1: -2.0 + 1.0e-4,
            v1: -4.0,
            multiplier: 2.0,
        };
        let receipt = qualify_midpoint_state_v1(
            problem,
            negative_root,
            JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture(),
            KantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            QualificationStatusV1::Inconclusive(
                InconclusiveReasonV1::CandidateBallLeavesPositiveConstraintBranch
            )
        );
    }

    #[test]
    fn poorly_conditioned_mass_regime_abstains_instead_of_fabricating_a_radius() {
        let problem = parameters(100.0, 1.0);
        let receipt = qualify_midpoint_state_v1(
            problem,
            approximate_state(100.0, 1.0),
            JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture(),
            KantorovichProfileV1::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.status,
            QualificationStatusV1::Inconclusive(InconclusiveReasonV1::KantorovichProductTooLarge)
        );
    }

    #[test]
    fn negative_mass_is_rejected_before_newton_estimation() {
        let problem = parameters(1.0, 1.0);
        let state = approximate_state(1.0, 1.0);
        let correct = qualify_midpoint_state_v1(
            problem,
            state,
            JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture(),
            KantorovichProfileV1::default(),
        )
        .unwrap();

        let mut wrong_problem = problem;
        wrong_problem.mass = -1.0;
        assert!(matches!(
            qualify_midpoint_state_v1(
                wrong_problem,
                state,
                JacobianLipschitzWitnessV1::for_quadratic_midpoint_fixture(),
                KantorovichProfileV1::default()
            ),
            Err(KktErrorV1::InvalidPhysicalParameter)
        ));
        assert_eq!(
            correct.status,
            QualificationStatusV1::AdmissibleLocalRootEstimate
        );
    }
}
