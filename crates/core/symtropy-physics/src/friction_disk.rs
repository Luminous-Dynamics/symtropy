// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Maximum-dissipation tangential friction reference for a 3D point contact.
//!
//! This solves the two-dimensional tangential impulse subproblem for a fixed,
//! already-selected normal impulse. Given the tangential effective-mass block
//! W_tt and the tangential velocity after the normal impulse, it minimizes
//!
//!   1/2 j_t^T W_tt j_t + v_t^T j_t
//!
//! subject to ||j_t|| <= mu * lambda_n.
//!
//! The SPD metric may be anisotropic and off-diagonal. The solution is either
//! unconstrained sticking or a boundary sliding solution found from the
//! monotone trust-region secular equation. This is an oracle for the
//! tangential subproblem, NOT a coupled normal-complementarity solve and NOT
//! a multi-contact island solver.

use nalgebra::{Matrix2, SVector};

/// Failures from the fixed-normal-load tangential friction reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrictionDiskError {
    NonFinite,
    InvalidFrictionCoefficient,
    InvalidNormalImpulse,
    InvalidTolerance,
    NotSymmetric,
    NotPositiveDefinite,
    SingularMetric,
    BracketFailed,
    NonDissipativeSolution,
}

/// Solution of one 3D contact's two-dimensional tangential impulse subproblem.
#[derive(Clone, Debug)]
pub struct TangentialFrictionDiskSolution {
    /// Optimal two-component tangential impulse in the caller's contact frame.
    pub impulse: SVector<f64, 2>,
    /// Tangential velocity after applying the impulse.
    pub post_slip_velocity: SVector<f64, 2>,
    /// Radius mu * lambda_n of the Coulomb impulse disk.
    pub impulse_radius: f64,
    /// KKT multiplier for the active disk boundary; zero for an interior stick.
    pub disk_multiplier: f64,
    /// Absolute violation of the Coulomb impulse disk.
    pub cone_violation: f64,
    /// Dimensionless stationarity residual.
    pub normalized_stationarity_residual: f64,
    /// Signed kinetic-energy change due to this impulse; must be non-positive.
    pub frictional_work_delta: f64,
    /// True when the optimal impulse lies on the friction disk boundary.
    pub is_sliding: bool,
}

/// Solve the maximum-dissipation tangential impulse subproblem for fixed
/// normal impulse lambda_n.
///
/// The supplied free_tangential_velocity must already include the velocity
/// contribution from the selected normal impulse, including W_tn * lambda_n.
/// The normal complementarity equation is intentionally not solved here.
///
/// The objective is strictly convex because effective_mass_tangent must be
/// symmetric positive definite. On the boundary, the KKT system is
///
///   (W_tt + tau I) j_t = -v_t,  ||j_t|| = mu * lambda_n,  tau >= 0.
///
/// The norm of the left-hand solution is monotone decreasing in tau, so a
/// bracketed scalar solve is deterministic and fail-closed.
pub fn solve_tangential_friction_disk(
    effective_mass_tangent: &Matrix2<f64>,
    free_tangential_velocity: &SVector<f64, 2>,
    friction_coefficient: f64,
    normal_impulse: f64,
    relative_tolerance: f64,
) -> Result<TangentialFrictionDiskSolution, FrictionDiskError> {
    if !effective_mass_tangent.iter().all(|value| value.is_finite())
        || !free_tangential_velocity.iter().all(|value| value.is_finite())
        || !friction_coefficient.is_finite()
        || !normal_impulse.is_finite()
    {
        return Err(FrictionDiskError::NonFinite);
    }
    if friction_coefficient < 0.0 {
        return Err(FrictionDiskError::InvalidFrictionCoefficient);
    }
    if normal_impulse < 0.0 {
        return Err(FrictionDiskError::InvalidNormalImpulse);
    }
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
        return Err(FrictionDiskError::InvalidTolerance);
    }

    let matrix_scale = effective_mass_tangent.norm().max(f64::MIN_POSITIVE);
    if (effective_mass_tangent - effective_mass_tangent.transpose()).norm()
        > relative_tolerance * matrix_scale
    {
        return Err(FrictionDiskError::NotSymmetric);
    }
    let metric = (effective_mass_tangent + effective_mass_tangent.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(metric).eigenvalues;
    let minimum_eigenvalue = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if minimum_eigenvalue <= relative_tolerance * matrix_scale {
        return Err(FrictionDiskError::NotPositiveDefinite);
    }

    let radius = friction_coefficient * normal_impulse;
    if !radius.is_finite() {
        return Err(FrictionDiskError::NonFinite);
    }

    let zero = SVector::<f64, 2>::zeros();
    let zero_result = |impulse: SVector<f64, 2>| {
        let post_slip_velocity = free_tangential_velocity + metric * impulse;
        let frictional_work_delta = impulse.dot(
            &(free_tangential_velocity + 0.5 * metric * impulse),
        );
        TangentialFrictionDiskSolution {
            impulse,
            post_slip_velocity,
            impulse_radius: radius,
            disk_multiplier: 0.0,
            cone_violation: 0.0,
            normalized_stationarity_residual: 0.0,
            frictional_work_delta,
            is_sliding: false,
        }
    };

    if radius == 0.0 || free_tangential_velocity.norm() == 0.0 {
        return Ok(zero_result(zero));
    }

    let unconstrained = metric
        .lu()
        .solve(&(-free_tangential_velocity))
        .ok_or(FrictionDiskError::SingularMetric)?;
    if !unconstrained.iter().all(|value| value.is_finite()) {
        return Err(FrictionDiskError::NonFinite);
    }
    let impulse_scale = unconstrained.norm().max(radius).max(f64::MIN_POSITIVE);
    let radius_tolerance = relative_tolerance * impulse_scale;

    if unconstrained.norm() <= radius + radius_tolerance {
        let mut solution = zero_result(unconstrained);
        solution.post_slip_velocity = free_tangential_velocity + metric * solution.impulse;
        solution.frictional_work_delta = solution.impulse.dot(
            &(free_tangential_velocity + 0.5 * metric * solution.impulse),
        );
        let velocity_scale = free_tangential_velocity
            .norm()
            .max(matrix_scale * solution.impulse.norm())
            .max(f64::MIN_POSITIVE);
        solution.normalized_stationarity_residual =
            solution.post_slip_velocity.norm() / velocity_scale;
        let work_scale = solution.impulse.norm()
            * (free_tangential_velocity.norm() + (metric * solution.impulse).norm())
                .max(f64::MIN_POSITIVE);
        validate_dissipation(&solution, relative_tolerance, work_scale)?;
        return Ok(solution);
    }

    let solve_for_multiplier = |multiplier: f64| {
        let shifted = metric + Matrix2::identity() * multiplier;
        shifted
            .lu()
            .solve(&(-free_tangential_velocity))
            .ok_or(FrictionDiskError::SingularMetric)
    };

    // At tau=0 the unconstrained impulse is outside the disk. Increase the
    // upper bracket until the impulse norm is below the disk radius.
    let mut lower = 0.0_f64;
    let mut upper = matrix_scale
        .max(free_tangential_velocity.norm() / radius)
        .max(f64::MIN_POSITIVE);
    let mut upper_impulse = solve_for_multiplier(upper)?;
    let mut bracketed = upper_impulse.norm() <= radius;
    for _ in 0..128 {
        if bracketed {
            break;
        }
        upper *= 2.0;
        if !upper.is_finite() {
            return Err(FrictionDiskError::BracketFailed);
        }
        upper_impulse = solve_for_multiplier(upper)?;
        bracketed = upper_impulse.norm() <= radius;
    }
    if !bracketed {
        return Err(FrictionDiskError::BracketFailed);
    }

    // Deterministic bisection on the strictly monotone secular function.
    for _ in 0..128 {
        let midpoint = lower + 0.5 * (upper - lower);
        let candidate = solve_for_multiplier(midpoint)?;
        if candidate.norm() > radius {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
        let interval_scale = lower.abs().max(upper.abs()).max(f64::MIN_POSITIVE);
        if (upper - lower) <= relative_tolerance * interval_scale {
            break;
        }
    }

    let multiplier = lower + 0.5 * (upper - lower);
    let impulse = solve_for_multiplier(multiplier)?;
    if !impulse.iter().all(|value| value.is_finite()) {
        return Err(FrictionDiskError::NonFinite);
    }

    let post_slip_velocity = free_tangential_velocity + metric * impulse;
    let stationarity = post_slip_velocity + impulse * multiplier;
    let velocity_scale = free_tangential_velocity
        .norm()
        .max(matrix_scale * impulse.norm())
        .max(f64::MIN_POSITIVE);
    let normalized_stationarity_residual = stationarity.norm() / velocity_scale;
    let cone_violation = (impulse.norm() - radius).max(0.0);
    let frictional_work_delta = impulse.dot(
        &(free_tangential_velocity + 0.5 * metric * impulse),
    );

    let solution = TangentialFrictionDiskSolution {
        impulse,
        post_slip_velocity,
        impulse_radius: radius,
        disk_multiplier: multiplier,
        cone_violation,
        normalized_stationarity_residual,
        frictional_work_delta,
        is_sliding: true,
    };
    let work_scale = solution.impulse.norm()
        * (free_tangential_velocity.norm() + (metric * solution.impulse).norm())
            .max(f64::MIN_POSITIVE);
    validate_dissipation(&solution, relative_tolerance, work_scale)?;
    if normalized_stationarity_residual > 10.0 * relative_tolerance
        || cone_violation > 10.0 * relative_tolerance * impulse_scale
    {
        return Err(FrictionDiskError::BracketFailed);
    }
    Ok(solution)
}

fn validate_dissipation(
    solution: &TangentialFrictionDiskSolution,
    relative_tolerance: f64,
    work_scale: f64,
) -> Result<(), FrictionDiskError> {
    if !solution.frictional_work_delta.is_finite()
        || !solution.post_slip_velocity.iter().all(|value| value.is_finite())
    {
        return Err(FrictionDiskError::NonFinite);
    }
    if !work_scale.is_finite() || work_scale < 0.0 {
        return Err(FrictionDiskError::NonFinite);
    }
    if solution.frictional_work_delta > relative_tolerance * work_scale.max(f64::MIN_POSITIVE) {
        return Err(FrictionDiskError::NonDissipativeSolution);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check_kkt(solution: &TangentialFrictionDiskSolution, tolerance: f64) {
        assert!(solution.cone_violation <= tolerance);
        assert!(solution.frictional_work_delta <= tolerance);
        assert!(solution.normalized_stationarity_residual <= tolerance);
        if solution.is_sliding {
            assert!(solution.disk_multiplier > 0.0);
            assert!((solution.impulse.norm() - solution.impulse_radius).abs() <= tolerance);
            let cross = solution.impulse[0] * solution.post_slip_velocity[1]
                - solution.impulse[1] * solution.post_slip_velocity[0];
            assert!(cross.abs() <= tolerance * solution.impulse.norm().max(1.0));
            assert!(solution.impulse.dot(&solution.post_slip_velocity) <= tolerance);
        } else {
            assert_eq!(solution.disk_multiplier, 0.0);
            assert!(solution.post_slip_velocity.norm() <= tolerance);
        }
    }

    #[test]
    fn sticking_solution_is_the_unconstrained_anisotropic_minimizer() {
        let metric = Matrix2::new(2.0, 0.35, 0.35, 1.2);
        let free_velocity = SVector::new(-0.2, 0.1);
        let solution =
            solve_tangential_friction_disk(&metric, &free_velocity, 0.8, 1.0, 1e-12)
                .unwrap();
        assert!(!solution.is_sliding);
        check_kkt(&solution, 1e-10);
    }

    #[test]
    fn sliding_solution_saturates_the_coulomb_disk_and_opposes_slip() {
        let metric = Matrix2::new(2.0, 0.6, 0.6, 1.0);
        let free_velocity = SVector::new(-3.0, 1.0);
        let solution =
            solve_tangential_friction_disk(&metric, &free_velocity, 0.2, 0.5, 1e-12)
                .unwrap();
        assert!(solution.is_sliding);
        check_kkt(&solution, 2e-10);
    }

    #[test]
    fn normal_tangent_coupling_is_folded_into_free_slip_before_disk_solve() {
        let full_metric = nalgebra::Matrix3::new(
            3.0, 0.4, -0.2,
            0.4, 2.0, 0.5,
            -0.2, 0.5, 1.5,
        );
        let normal_impulse = 0.6;
        let q_tangent = SVector::new(1.2, -0.9);
        let normal_tangent_column = SVector::new(full_metric[(1, 0)], full_metric[(2, 0)]);
        let free_slip = q_tangent + normal_tangent_column * normal_impulse;
        let tangent_metric = Matrix2::new(
            full_metric[(1, 1)], full_metric[(1, 2)],
            full_metric[(2, 1)], full_metric[(2, 2)],
        );
        let solution = solve_tangential_friction_disk(
            &tangent_metric,
            &free_slip,
            0.25,
            normal_impulse,
            1e-12,
        ).unwrap();
        check_kkt(&solution, 2e-10);
        assert!(solution.impulse_radius > 0.0);
    }

    #[test]
    fn tangential_solver_is_equivariant_under_contact_frame_rotation() {
        let metric = Matrix2::new(1.8, 0.4, 0.4, 2.3);
        let velocity = SVector::new(-2.0, 0.7);
        let (s, c) = 0.63_f64.sin_cos();
        let rotation = Matrix2::new(c, -s, s, c);
        let rotated_metric = rotation * metric * rotation.transpose();
        let rotated_velocity = rotation * velocity;
        let direct =
            solve_tangential_friction_disk(&metric, &velocity, 0.45, 0.8, 1e-12)
                .unwrap();
        let rotated = solve_tangential_friction_disk(
            &rotated_metric,
            &rotated_velocity,
            0.45,
            0.8,
            1e-12,
        ).unwrap();
        assert!((rotated.impulse - rotation * direct.impulse).norm() < 2e-10);
        assert!((rotated.post_slip_velocity - rotation * direct.post_slip_velocity).norm() < 2e-10);
        assert!((rotated.disk_multiplier - direct.disk_multiplier).abs() < 2e-10);
    }

    #[test]
    fn frictionless_and_zero_load_controls_return_zero_impulse() {
        let metric = Matrix2::new(2.0, 0.2, 0.2, 1.0);
        let velocity = SVector::new(2.0, -1.0);
        for (mu, load) in [(0.0, 2.0), (0.5, 0.0)] {
            let solution =
                solve_tangential_friction_disk(&metric, &velocity, mu, load, 1e-12)
                    .unwrap();
            assert_eq!(solution.impulse, SVector::zeros());
            assert_eq!(solution.frictional_work_delta, 0.0);
        }
    }

    #[test]
    fn rejects_invalid_normal_load_friction_and_metric() {
        let metric = Matrix2::identity();
        let velocity = SVector::new(1.0, 0.2);
        assert_eq!(
            solve_tangential_friction_disk(&metric, &velocity, -0.1, 1.0, 1e-12)
                .unwrap_err(),
            FrictionDiskError::InvalidFrictionCoefficient
        );
        assert_eq!(
            solve_tangential_friction_disk(&metric, &velocity, 0.1, -1.0, 1e-12)
                .unwrap_err(),
            FrictionDiskError::InvalidNormalImpulse
        );
        let asymmetric = Matrix2::new(1.0, 0.4, 0.1, 1.0);
        assert_eq!(
            solve_tangential_friction_disk(&asymmetric, &velocity, 0.1, 1.0, 1e-12)
                .unwrap_err(),
            FrictionDiskError::NotSymmetric
        );
        let indefinite = Matrix2::new(1.0, 2.0, 2.0, 1.0);
        assert_eq!(
            solve_tangential_friction_disk(&indefinite, &velocity, 0.1, 1.0, 1e-12)
                .unwrap_err(),
            FrictionDiskError::NotPositiveDefinite
        );
    }

    #[test]
    fn deterministic_anisotropic_panel_satisfies_the_kkt_contract() {
        for seed in 1..=128_u64 {
            let a = ((seed * 17) % 53) as f64 / 31.0;
            let b = ((seed * 29) % 47) as f64 / 37.0 - 0.5;
            let c = ((seed * 13) % 41) as f64 / 29.0;
            let metric = Matrix2::new(1.0 + a * a, 0.3 * a * c + b * 0.1,
                0.3 * a * c + b * 0.1, 1.0 + c * c);
            let velocity = SVector::new(
                ((seed * 7) % 61) as f64 / 13.0 - 2.0,
                ((seed * 11) % 59) as f64 / 17.0 - 1.5,
            );
            let solution = solve_tangential_friction_disk(
                &metric,
                &velocity,
                0.05 + ((seed * 3) % 9) as f64 * 0.12,
                0.1 + ((seed * 5) % 11) as f64 * 0.2,
                1e-11,
            ).unwrap_or_else(|error| panic!("seed={seed}: {error:?}"));
            check_kkt(&solution, 2e-8);
        }
    }
}
