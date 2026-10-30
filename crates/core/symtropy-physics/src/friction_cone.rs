// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact small-reference solver for a planar single-contact Coulomb law.
//!
//! The variable order is (normal_impulse, tangential_impulse). The problem
//! combines Signorini normal complementarity with maximum-dissipation
//! Coulomb friction. It is an oracle for one 2D contact, not a production
//! multi-contact solver.

use nalgebra::{Matrix2, SVector};

/// Failure while solving the planar friction reference problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrictionConeError {
    NonFinite,
    InvalidFrictionCoefficient,
    InvalidTolerance,
    NotSymmetric,
    NotPositiveDefinite,
    NoAdmissibleSolution,
}

/// Exact solution of the planar single-contact Signorini/Coulomb problem.
#[derive(Clone, Debug)]
pub struct PlanarFrictionConeSolution {
    /// (normal, tangent) impulse components.
    pub impulse: SVector<f64, 2>,
    /// Post-impulse normal/tangential relative velocity, W*impulse + q.
    pub post_velocity: SVector<f64, 2>,
    /// Absolute Coulomb-cone violation.
    pub cone_violation: f64,
    /// Normalized residual for normal complementarity and the friction law.
    pub normalized_contact_residual: f64,
}

#[derive(Clone, Copy)]
enum ContactMode {
    Separating,
    Sticking,
    SlidingPositive,
    SlidingNegative,
}

/// Solve one planar contact with
///
/// - normal complementarity: lambda_n >= 0, u_n >= 0,
///   lambda_n * u_n = 0;
/// - maximum-dissipation Coulomb friction: if lambda_n > 0, then either
///   u_t = 0 with |lambda_t| <= mu*lambda_n, or sliding with
///   lambda_t = -mu*lambda_n*sign(u_t).
///
/// The effective mass matrix W must be symmetric positive definite and
/// u = W*lambda + q. The exhaustive cases are separation, sticking,
/// positive-direction sliding and negative-direction sliding. Singular or
/// Painleve-like cases with no admissible solution are reported explicitly.
pub fn solve_planar_friction_cone(
    effective_mass: &Matrix2<f64>,
    free_velocity: &SVector<f64, 2>,
    friction_coefficient: f64,
    relative_tolerance: f64,
) -> Result<PlanarFrictionConeSolution, FrictionConeError> {
    if !effective_mass.iter().all(|value| value.is_finite())
        || !free_velocity.iter().all(|value| value.is_finite())
    {
        return Err(FrictionConeError::NonFinite);
    }
    if !friction_coefficient.is_finite() || friction_coefficient < 0.0 {
        return Err(FrictionConeError::InvalidFrictionCoefficient);
    }
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
        return Err(FrictionConeError::InvalidTolerance);
    }

    let matrix_scale = effective_mass.norm().max(f64::MIN_POSITIVE);
    if (effective_mass - effective_mass.transpose()).norm()
        > relative_tolerance * matrix_scale
    {
        return Err(FrictionConeError::NotSymmetric);
    }

    let matrix = (effective_mass + effective_mass.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(matrix).eigenvalues;
    let minimum_eigenvalue = eigenvalues.iter().fold(f64::INFINITY, |minimum, value| {
        minimum.min(*value)
    });
    if minimum_eigenvalue <= relative_tolerance * matrix_scale {
        return Err(FrictionConeError::NotPositiveDefinite);
    }

    let mu = friction_coefficient;
    let velocity_scale = free_velocity.norm().max(f64::MIN_POSITIVE);
    let impulse_scale = (velocity_scale / matrix_scale).max(f64::MIN_POSITIVE);
    let velocity_tolerance = relative_tolerance * velocity_scale;
    let impulse_tolerance = relative_tolerance * impulse_scale;

    let (impulse, mode) = if free_velocity[0] >= -velocity_tolerance {
        (SVector::zeros(), ContactMode::Separating)
    } else {
        let mut admissible = Vec::with_capacity(3);

        if let Some(sticking) = matrix.lu().solve(&(-free_velocity)) {
            let cone_slack = mu * sticking[0] - sticking[1].abs();
            if sticking[0] > impulse_tolerance && cone_slack >= -impulse_tolerance {
                let post = matrix * sticking + free_velocity;
                if post.norm() <= 10.0 * velocity_tolerance {
                    admissible.push((sticking, ContactMode::Sticking));
                }
            }
        }

        // Sliding with positive tangential post-velocity: lambda_t=-mu*lambda_n.
        let positive_denominator = matrix[(0, 0)] - mu * matrix[(0, 1)];
        if positive_denominator > relative_tolerance * matrix_scale {
            let normal_impulse = -free_velocity[0] / positive_denominator;
            if normal_impulse > impulse_tolerance {
                let sliding = SVector::new(normal_impulse, -mu * normal_impulse);
                let post = matrix * sliding + free_velocity;
                if post[0].abs() <= 10.0 * velocity_tolerance
                    && post[1] >= -velocity_tolerance
                {
                    admissible.push((sliding, ContactMode::SlidingPositive));
                }
            }
        }

        // Sliding with negative tangential post-velocity: lambda_t=+mu*lambda_n.
        let negative_denominator = matrix[(0, 0)] + mu * matrix[(0, 1)];
        if negative_denominator > relative_tolerance * matrix_scale {
            let normal_impulse = -free_velocity[0] / negative_denominator;
            if normal_impulse > impulse_tolerance {
                let sliding = SVector::new(normal_impulse, mu * normal_impulse);
                let post = matrix * sliding + free_velocity;
                if post[0].abs() <= 10.0 * velocity_tolerance
                    && post[1] <= velocity_tolerance
                {
                    admissible.push((sliding, ContactMode::SlidingNegative));
                }
            }
        }

        admissible
            .into_iter()
            .next()
            .ok_or(FrictionConeError::NoAdmissibleSolution)?
    };

    let post_velocity = matrix * impulse + free_velocity;
    let cone_violation = (impulse[1].abs() - mu * impulse[0])
        .max(0.0)
        .max((-impulse[0]).max(0.0));

    let normal_complementarity = (-impulse[0])
        .max(0.0)
        .max((-post_velocity[0]).max(0.0))
        .max((impulse[0] * post_velocity[0]).abs() / velocity_scale);
    let tangential_residual = match mode {
        ContactMode::Separating => impulse[1].abs(),
        ContactMode::Sticking => post_velocity[1].abs().max(cone_violation),
        ContactMode::SlidingPositive => {
            (-post_velocity[1]).max(0.0).max((impulse[1] + mu * impulse[0]).abs()
                * matrix_scale
                / velocity_scale)
        }
        ContactMode::SlidingNegative => {
            post_velocity[1].max(0.0).max((impulse[1] - mu * impulse[0]).abs()
                * matrix_scale
                / velocity_scale)
        }
    };
    let normalized_contact_residual =
        normal_complementarity.max(tangential_residual) / velocity_scale.max(1.0);

    if !post_velocity.iter().all(|value| value.is_finite())
        || !normalized_contact_residual.is_finite()
        || !cone_violation.is_finite()
    {
        return Err(FrictionConeError::NonFinite);
    }

    Ok(PlanarFrictionConeSolution {
        impulse,
        post_velocity,
        cone_violation,
        normalized_contact_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticking_solution_satisfies_normal_and_tangential_constraints() {
        let effective_mass = Matrix2::from_diagonal(&SVector::new(2.0, 1.0));
        let free_velocity = SVector::new(-1.0, -0.2);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 1.0, 1e-12).unwrap();

        assert!((solution.impulse[0] - 0.5).abs() < 1e-12);
        assert!((solution.impulse[1] - 0.2).abs() < 1e-12);
        assert!(solution.post_velocity.norm() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_contact_residual < 1e-12);
    }

    #[test]
    fn sliding_solution_satisfies_signorini_and_maximum_dissipation() {
        let effective_mass = Matrix2::from_diagonal(&SVector::new(2.0, 1.0));
        let free_velocity = SVector::new(-1.0, -1.0);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.5, 1e-12).unwrap();

        assert!((solution.impulse[0] - 0.5).abs() < 1e-12);
        assert!((solution.impulse[1] - 0.25).abs() < 1e-12);
        assert!(solution.post_velocity[0].abs() < 1e-12);
        assert!(solution.post_velocity[1] < 0.0);
        assert!((solution.impulse[1] - 0.5 * solution.impulse[0]).abs() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_contact_residual < 1e-12);
    }

    #[test]
    fn separating_contact_returns_zero_impulse_even_with_large_tangential_slip() {
        let effective_mass = Matrix2::identity();
        let free_velocity = SVector::new(0.1, 10.0);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.5, 1e-12).unwrap();

        assert!(solution.impulse.norm() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_contact_residual < 1e-12);
    }

    #[test]
    fn zero_friction_reduces_to_a_normal_only_contact() {
        let effective_mass = Matrix2::new(2.0, 0.25, 0.25, 1.5);
        let free_velocity = SVector::new(-1.0, -0.8);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.0, 1e-12).unwrap();

        assert!(solution.impulse[0] > 0.0);
        assert!(solution.impulse[1].abs() < 1e-12);
        assert!(solution.post_velocity[0].abs() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
    }

    #[test]
    fn friction_reference_rejects_invalid_inputs_and_non_spd_metrics() {
        let identity = Matrix2::identity();
        let velocity = SVector::new(-1.0, 0.0);
        assert_eq!(
            solve_planar_friction_cone(&identity, &velocity, -0.1, 1e-12).unwrap_err(),
            FrictionConeError::InvalidFrictionCoefficient
        );
        assert_eq!(
            solve_planar_friction_cone(&identity, &velocity, 0.5, 0.0).unwrap_err(),
            FrictionConeError::InvalidTolerance
        );
        let indefinite = Matrix2::new(1.0, 2.0, 2.0, 1.0);
        assert_eq!(
            solve_planar_friction_cone(&indefinite, &velocity, 0.5, 1e-12).unwrap_err(),
            FrictionConeError::NotPositiveDefinite
        );
    }

    #[test]
    fn planar_contact_solution_is_invariant_to_common_metric_velocity_scaling() {
        let matrix = Matrix2::new(2.0, 0.25, 0.25, 1.5);
        let velocity = SVector::new(-1.0, -0.8);
        let reference = solve_planar_friction_cone(&matrix, &velocity, 0.4, 1e-12).unwrap();
        for scale in [1e-12, 1.0, 1e12] {
            let scaled =
                solve_planar_friction_cone(&(matrix * scale), &(velocity * scale), 0.4, 1e-12)
                    .unwrap();
            assert!((scaled.impulse - reference.impulse).norm() < 1e-9);
            assert!(scaled.normalized_contact_residual < 1e-10);
        }
    }
}
