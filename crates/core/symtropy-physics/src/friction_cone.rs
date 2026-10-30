// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact small-reference solver for a planar single-contact Coulomb cone.
//!
//! The variable order is (normal_impulse, tangential_impulse). This is a
//! correctness oracle for one 2D contact, not a scalable production solver.
//! The convex problem is solved by enumerating the unconstrained interior,
//! both cone boundary rays, and the apex.

use nalgebra::{Matrix2, SVector};

/// Failure while solving the planar friction-cone reference problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrictionConeError {
    NonFinite,
    InvalidFrictionCoefficient,
    InvalidTolerance,
    NotSymmetric,
    NotPositiveDefinite,
}

/// Exact solution of the planar single-contact maximum-dissipation problem.
#[derive(Clone, Debug)]
pub struct PlanarFrictionConeSolution {
    /// (normal, tangent) impulse components.
    pub impulse: SVector<f64, 2>,
    /// Post-impulse normal/tangential relative velocity, W*impulse + q.
    pub post_velocity: SVector<f64, 2>,
    /// Objective 0.5 * impulseᵀ W impulse + qᵀ impulse.
    pub objective: f64,
    /// Absolute Coulomb-cone violation.
    pub cone_violation: f64,
    /// Normalized KKT residual for the selected candidate.
    pub normalized_kkt_residual: f64,
}

#[derive(Clone, Copy)]
enum CandidateKind {
    Apex,
    Interior,
    PositiveBoundary,
    NegativeBoundary,
}

struct Candidate {
    impulse: SVector<f64, 2>,
    objective: f64,
    kind: CandidateKind,
}

/// Solve the planar frictional contact problem
///
/// min 0.5 * λᵀ W λ + qᵀ λ subject to
/// λ_n >= 0 and |λ_t| <= μ λ_n.
///
/// W must be symmetric positive definite. The active-set enumeration is
/// exhaustive in two dimensions: an optimum is either the unconstrained
/// minimizer in the cone interior, on one of its two boundary rays, or at the
/// apex. No iterative tolerance or diagonal regularizer is used.
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

    let symmetric = (effective_mass + effective_mass.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(symmetric).eigenvalues;
    let minimum_eigenvalue = eigenvalues.iter().fold(f64::INFINITY, |minimum, value| {
        minimum.min(*value)
    });
    if minimum_eigenvalue <= relative_tolerance * matrix_scale {
        return Err(FrictionConeError::NotPositiveDefinite);
    }

    let mu = friction_coefficient;
    let positive_ray = SVector::new(1.0, mu);
    let negative_ray = SVector::new(1.0, -mu);
    let mut candidates = Vec::with_capacity(4);

    candidates.push(Candidate {
        impulse: SVector::zeros(),
        objective: 0.0,
        kind: CandidateKind::Apex,
    });

    if let Some(interior) = symmetric.lu().solve(&(-free_velocity)) {
        let cone_slack = mu * interior[0] - interior[1].abs();
        if interior[0] >= -relative_tolerance && cone_slack >= -relative_tolerance {
            let objective = 0.5 * interior.dot(&(symmetric * interior))
                + free_velocity.dot(&interior);
            candidates.push(Candidate {
                impulse: interior,
                objective,
                kind: CandidateKind::Interior,
            });
        }
    }

    for (ray, kind) in [
        (positive_ray, CandidateKind::PositiveBoundary),
        (negative_ray, CandidateKind::NegativeBoundary),
    ] {
        let curvature = ray.dot(&(symmetric * ray));
        let unconstrained_alpha = -ray.dot(free_velocity) / curvature;
        let alpha = unconstrained_alpha.max(0.0);
        let impulse = ray * alpha;
        let objective =
            0.5 * impulse.dot(&(symmetric * impulse)) + free_velocity.dot(&impulse);
        candidates.push(Candidate {
            impulse,
            objective,
            kind,
        });
    }

    let best = candidates
        .into_iter()
        .min_by(|left, right| left.objective.total_cmp(&right.objective))
        .expect("apex candidate is always present");
    let post_velocity = symmetric * best.impulse + free_velocity;

    let kkt_residual = match best.kind {
        CandidateKind::Apex => {
            let positive_slack = post_velocity.dot(&positive_ray);
            let negative_slack = post_velocity.dot(&negative_ray);
            (-positive_slack).max(0.0).max((-negative_slack).max(0.0))
        }
        CandidateKind::Interior => post_velocity.norm(),
        CandidateKind::PositiveBoundary => post_velocity
            .dot(&positive_ray)
            .abs()
            .max((-post_velocity.dot(&negative_ray)).max(0.0)),
        CandidateKind::NegativeBoundary => post_velocity
            .dot(&negative_ray)
            .abs()
            .max((-post_velocity.dot(&positive_ray)).max(0.0)),
    };
    let residual_scale = free_velocity
        .norm()
        .max((symmetric.norm() * best.impulse.norm()).abs())
        .max(f64::MIN_POSITIVE);
    let normalized_kkt_residual = kkt_residual / residual_scale;
    let cone_violation = (best.impulse[1].abs() - mu * best.impulse[0])
        .max(0.0)
        .max((-best.impulse[0]).max(0.0));

    if !best.objective.is_finite()
        || !normalized_kkt_residual.is_finite()
        || !cone_violation.is_finite()
    {
        return Err(FrictionConeError::NonFinite);
    }

    Ok(PlanarFrictionConeSolution {
        impulse: best.impulse,
        post_velocity,
        objective: best.objective,
        cone_violation,
        normalized_kkt_residual,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sticking_solution_is_the_unconstrained_minimizer_when_inside_the_cone() {
        let effective_mass = Matrix2::from_diagonal(&SVector::new(2.0, 1.0));
        let free_velocity = SVector::new(-1.0, -0.2);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 1.0, 1e-12).unwrap();

        assert!((solution.impulse[0] - 0.5).abs() < 1e-12);
        assert!((solution.impulse[1] - 0.2).abs() < 1e-12);
        assert!(solution.post_velocity.norm() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_kkt_residual < 1e-12);
    }

    #[test]
    fn sliding_solution_lies_on_the_coulomb_boundary_and_satisfies_kkt() {
        let effective_mass = Matrix2::from_diagonal(&SVector::new(2.0, 1.0));
        let free_velocity = SVector::new(-1.0, -1.0);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.5, 1e-12).unwrap();

        assert!((solution.impulse[0] - 2.0 / 3.0).abs() < 1e-12);
        assert!((solution.impulse[1] - 1.0 / 3.0).abs() < 1e-12);
        assert!((solution.impulse[1].abs() - 0.5 * solution.impulse[0]).abs() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_kkt_residual < 1e-12);
    }

    #[test]
    fn separating_contact_returns_the_cone_apex() {
        let effective_mass = Matrix2::identity();
        let free_velocity = SVector::new(1.0, 0.2);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.5, 1e-12).unwrap();

        assert!(solution.impulse.norm() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_kkt_residual < 1e-12);
    }

    #[test]
    fn zero_friction_reduces_to_a_normal_only_contact() {
        let effective_mass = Matrix2::new(2.0, 0.25, 0.25, 1.5);
        let free_velocity = SVector::new(-1.0, -0.8);
        let solution =
            solve_planar_friction_cone(&effective_mass, &free_velocity, 0.0, 1e-12).unwrap();

        assert!(solution.impulse[0] >= 0.0);
        assert!(solution.impulse[1].abs() < 1e-12);
        assert!(solution.cone_violation < 1e-12);
        assert!(solution.normalized_kkt_residual < 1e-12);
    }

    #[test]
    fn cone_reference_rejects_invalid_inputs_and_non_spd_metrics() {
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
    fn planar_cone_solution_is_invariant_to_common_metric_velocity_scaling() {
        let matrix = Matrix2::new(2.0, 0.25, 0.25, 1.5);
        let velocity = SVector::new(-1.0, -0.8);
        let reference = solve_planar_friction_cone(&matrix, &velocity, 0.4, 1e-12).unwrap();
        for scale in [1e-12, 1.0, 1e12] {
            let scaled =
                solve_planar_friction_cone(&(matrix * scale), &(velocity * scale), 0.4, 1e-12)
                    .unwrap();
            assert!((scaled.impulse - reference.impulse).norm() < 1e-9);
            assert!(scaled.normalized_kkt_residual < 1e-10);
        }
    }
}
