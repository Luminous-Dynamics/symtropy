// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Small-island Delassus assembly and an exhaustive frictionless LCP oracle.
//!
//! This module is deliberately a reference implementation, not the production
//! contact solver. It favors transparent algebra and exhaustive active-set
//! enumeration for at most eight rows. The existing sequential-impulse solver
//! remains the runtime baseline until matched-work experiments justify a change.

use nalgebra::{DMatrix, DVector, SVector};

use crate::body::{BodyHandle, RigidBody};
use crate::inertia::RotationalInertiaError;
use crate::integrator::angular_point_velocity;
use symtropy_math::Bivector;

/// A frictionless unilateral contact row, with lever arms measured from each
/// body's center of mass and a unit normal pointing from A to B.
#[derive(Clone, Debug)]
pub struct FrictionlessContactRow<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub offset_a: SVector<f64, D>,
    pub offset_b: SVector<f64, D>,
    pub normal: SVector<f64, D>,
}

/// Failure while assembling a reference Delassus matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelassusAssemblyError {
    InvalidTolerance,
    InvalidRow,
    TooManyRows,
    MissingBody(BodyHandle),
    RotationalInertia(RotationalInertiaError),
}

/// Failure while solving a small frictionless LCP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceLcpError {
    InvalidDimensions,
    TooManyRows,
    InvalidTolerance,
    NonFinite,
    NotSymmetric,
    NotPositiveSemidefinite,
    NoFeasibleActiveSet,
}

/// A validated small-island reference LCP solution.
///
/// The residual is w = W*lambda + q. Complementarity requires lambda >= 0,
/// w >= 0, and lambda_i*w_i = 0 for every row.
#[derive(Clone, Debug)]
pub struct ReferenceLcpSolution {
    pub impulses: DVector<f64>,
    pub residual: DVector<f64>,
    pub active_rows: Vec<usize>,
    pub max_primal_violation: f64,
    pub max_complementarity_residual: f64,
}

/// Assemble W = J M^-1 J^T for frictionless contact rows.
///
/// The generalized response uses the same translational inverse mass,
/// world-rotated bivector inertia and physical point-velocity sign convention
/// as the runtime integrator. Rows may share bodies in arbitrary patterns.
/// The returned matrix is not symmetrized: callers and tests should observe
/// any asymmetry introduced by a broken response implementation.
pub fn assemble_delassus<const D: usize>(
    bodies: &[RigidBody<D>],
    rows: &[FrictionlessContactRow<D>],
    relative_tolerance: f64,
) -> Result<DMatrix<f64>, DelassusAssemblyError> {
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
        return Err(DelassusAssemblyError::InvalidTolerance);
    }
    if rows.is_empty() {
        return Err(DelassusAssemblyError::InvalidRow);
    }
    if rows.len() > 8 {
        return Err(DelassusAssemblyError::TooManyRows);
    }

    for row in rows {
        if row.body_a == row.body_b
            || !row.offset_a.iter().all(|value| value.is_finite())
            || !row.offset_b.iter().all(|value| value.is_finite())
            || !row.normal.iter().all(|value| value.is_finite())
            || (row.normal.norm() - 1.0).abs() > 1e-10
        {
            return Err(DelassusAssemblyError::InvalidRow);
        }
        if !bodies.iter().any(|body| body.handle == row.body_a) {
            return Err(DelassusAssemblyError::MissingBody(row.body_a));
        }
        if !bodies.iter().any(|body| body.handle == row.body_b) {
            return Err(DelassusAssemblyError::MissingBody(row.body_b));
        }
    }

    let count = rows.len();
    let mut matrix = DMatrix::<f64>::zeros(count, count);
    for i in 0..count {
        for j in 0..count {
            let row_i = &rows[i];
            let row_j = &rows[j];
            let mut relative_point_response = SVector::<f64, D>::zeros();

            // Relative point velocity is v_B(point) - v_A(point). A row-j
            // impulse therefore enters row-i's A endpoint with a minus sign
            // and its B endpoint with a plus sign.
            for (body_handle, sample_offset, relative_sign) in [
                (row_i.body_a, &row_i.offset_a, -1.0),
                (row_i.body_b, &row_i.offset_b, 1.0),
            ] {
                let Some(body) = bodies.iter().find(|body| body.handle == body_handle) else {
                    return Err(DelassusAssemblyError::MissingBody(body_handle));
                };

                let (impulse, impulse_offset) = if body_handle == row_j.body_a {
                    (-row_j.normal, &row_j.offset_a)
                } else if body_handle == row_j.body_b {
                    (row_j.normal, &row_j.offset_b)
                } else {
                    continue;
                };

                let point_response = impulse_response_at(
                    body,
                    &impulse,
                    impulse_offset,
                    sample_offset,
                    relative_tolerance,
                )
                .map_err(DelassusAssemblyError::RotationalInertia)?;
                relative_point_response += point_response * relative_sign;
            }

            matrix[(i, j)] = row_i.normal.dot(&relative_point_response);
        }
    }

    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(DelassusAssemblyError::RotationalInertia(
            RotationalInertiaError::NonFinite,
        ));
    }
    Ok(matrix)
}

/// Apply a unit generalized impulse at one lever arm and evaluate its point
/// velocity response at another point on the same rigid body.
fn impulse_response_at<const D: usize>(
    body: &RigidBody<D>,
    impulse: &SVector<f64, D>,
    impulse_offset: &SVector<f64, D>,
    sample_offset: &SVector<f64, D>,
    relative_tolerance: f64,
) -> Result<SVector<f64, D>, RotationalInertiaError> {
    if !body.is_dynamic() {
        return Ok(SVector::zeros());
    }

    let linear_response = impulse * body.inv_mass;
    let angular_impulse = Bivector::from_wedge(impulse, impulse_offset);
    let stored_angular_response = if let Some(inertia) = &body.rotational_inertia {
        inertia
            .rotated_by(&body.transform.rotation)
            .angular_velocity_from_momentum(&angular_impulse, relative_tolerance)?
            .scale(-1.0)
    } else {
        let inverse_mean_inertia = body.inv_inertia.sum() / D as f64;
        angular_impulse.scale(-inverse_mean_inertia)
    };

    Ok(linear_response + angular_point_velocity(&stored_angular_response, sample_offset))
}

/// Solve the monotone frictionless contact LCP:
///
///     lambda >= 0,  w = W*lambda + q >= 0,  lambda^T*w = 0.
///
/// This oracle enumerates all active sets and is intentionally capped at eight
/// rows (256 subsets). Singular active blocks are skipped. The production
/// solver should be compared against this reference on frozen small islands.
pub fn solve_frictionless_lcp(
    matrix: &DMatrix<f64>,
    q: &DVector<f64>,
    tolerance: f64,
) -> Result<ReferenceLcpSolution, ReferenceLcpError> {
    let n = q.len();
    if n == 0 || matrix.nrows() != n || matrix.ncols() != n {
        return Err(ReferenceLcpError::InvalidDimensions);
    }
    if n > 8 {
        return Err(ReferenceLcpError::TooManyRows);
    }
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(ReferenceLcpError::InvalidTolerance);
    }
    if !matrix.iter().all(|value| value.is_finite())
        || !q.iter().all(|value| value.is_finite())
    {
        return Err(ReferenceLcpError::NonFinite);
    }

    let matrix_scale = matrix.norm().max(1.0);
    let mut symmetry_error = 0.0_f64;
    for i in 0..n {
        for j in 0..n {
            symmetry_error = symmetry_error.max((matrix[(i, j)] - matrix[(j, i)]).abs());
        }
    }
    if symmetry_error > tolerance * matrix_scale {
        return Err(ReferenceLcpError::NotSymmetric);
    }

    let symmetric = (matrix + matrix.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(symmetric).eigenvalues;
    let min_eigenvalue = eigenvalues.iter().fold(f64::INFINITY, |minimum, value| {
        minimum.min(*value)
    });
    if min_eigenvalue < -tolerance * matrix_scale {
        return Err(ReferenceLcpError::NotPositiveSemidefinite);
    }

    let mut best: Option<ReferenceLcpSolution> = None;
    let subset_count = 1_u32 << n;
    for mask in 0..subset_count {
        let active: Vec<usize> = (0..n).filter(|index| (mask & (1 << index)) != 0).collect();
        let mut impulses = DVector::<f64>::zeros(n);

        if !active.is_empty() {
            let active_matrix = DMatrix::<f64>::from_fn(active.len(), active.len(), |i, j| {
                matrix[(active[i], active[j])]
            });
            let rhs = DVector::<f64>::from_iterator(
                active.len(),
                active.iter().map(|index| -q[*index]),
            );
            // SVD supplies a least-norm solution for rank-deficient active
            // blocks (e.g. redundant contact rows). Feasibility and the
            // original unregularized equations are checked below; no diagonal
            // regularizer is added to conceal a singular physical system.
            let Ok(active_solution) = active_matrix.svd(true, true).solve(&rhs, tolerance) else {
                continue;
            };
            for (local_index, global_index) in active.iter().enumerate() {
                impulses[*global_index] = active_solution[local_index];
            }
        }

        let feasibility_scale = 1.0 + matrix_scale * impulses.norm() + q.norm();
        let feasibility_tolerance = tolerance * feasibility_scale;
        if impulses.iter().any(|value| *value < -feasibility_tolerance) {
            continue;
        }
        for value in impulses.iter_mut() {
            if *value < 0.0 {
                *value = 0.0;
            }
        }

        let residual = matrix * &impulses + q;
        if residual.iter().any(|value| *value < -feasibility_tolerance) {
            continue;
        }
        let max_primal_violation = impulses
            .iter()
            .chain(residual.iter())
            .fold(0.0_f64, |maximum, value| maximum.max((-*value).max(0.0)));
        let max_complementarity_residual = impulses
            .iter()
            .zip(residual.iter())
            .fold(0.0_f64, |maximum, (lambda, slack)| {
                maximum.max((lambda * slack).abs())
            });
        if max_complementarity_residual > tolerance * feasibility_scale.powi(2) {
            continue;
        }

        let active_rows = impulses
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value > feasibility_tolerance).then_some(index))
            .collect();
        let candidate = ReferenceLcpSolution {
            impulses,
            residual,
            active_rows,
            max_primal_violation,
            max_complementarity_residual,
        };

        let candidate_score = candidate.max_primal_violation
            + candidate.max_complementarity_residual / feasibility_scale.max(1.0);
        let replace_best = best.as_ref().is_none_or(|current| {
            let current_score = current.max_primal_violation
                + current.max_complementarity_residual / feasibility_scale.max(1.0);
            candidate_score < current_score
        });
        if replace_best {
            best = Some(candidate);
        }
    }

    best.ok_or(ReferenceLcpError::NoFeasibleActiveSet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inertia::BivectorInertia;
    use nalgebra::{SVector, DMatrix, DVector};
    use symtropy_math::Point;

    #[test]
    fn one_row_lcp_matches_analytic_impulse() {
        let matrix = DMatrix::from_element(1, 1, 2.0);
        let q = DVector::from_vec(vec![-1.0]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();

        assert!((solution.impulses[0] - 0.5).abs() < 1e-12);
        assert!(solution.residual[0].abs() < 1e-12);
        assert!(solution.max_complementarity_residual < 1e-12);
    }

    #[test]
    fn coupled_two_row_lcp_matches_analytic_solution() {
        let matrix = DMatrix::from_row_slice(2, 2, &[2.0, 0.5, 0.5, 1.0]);
        let q = DVector::from_vec(vec![-1.125, -0.5]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();

        assert!((solution.impulses[0] - 0.5).abs() < 1e-12);
        assert!((solution.impulses[1] - 0.25).abs() < 1e-12);
        assert!(solution.residual.norm() < 1e-12);
    }

    #[test]
    fn inactive_contact_row_receives_zero_impulse() {
        let matrix = DMatrix::from_diagonal(&DVector::from_vec(vec![2.0, 1.0]));
        let q = DVector::from_vec(vec![-1.0, 0.5]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();

        assert!((solution.impulses[0] - 0.5).abs() < 1e-12);
        assert_eq!(solution.impulses[1], 0.0);
        assert!((solution.residual[1] - 0.5).abs() < 1e-12);
    }

    #[test]
    fn redundant_contact_rows_are_solved_without_regularization() {
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 1.0, 1.0, 1.0]);
        let q = DVector::from_vec(vec![-1.0, -1.0]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();

        assert!((matrix * &solution.impulses + &q).norm() < 1e-10);
        assert!(solution.impulses.iter().all(|value| *value >= 0.0));
        assert!(solution.max_complementarity_residual < 1e-10);
    }

    #[test]
    fn solver_rejects_non_psd_and_asymmetric_matrices() {
        let q = DVector::from_vec(vec![-1.0, -1.0]);
        let asymmetric = DMatrix::from_row_slice(2, 2, &[1.0, 0.2, 0.0, 1.0]);
        assert_eq!(
            solve_frictionless_lcp(&asymmetric, &q, 1e-12).unwrap_err(),
            ReferenceLcpError::NotSymmetric
        );

        let indefinite = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 1.0]);
        assert_eq!(
            solve_frictionless_lcp(&indefinite, &q, 1e-12).unwrap_err(),
            ReferenceLcpError::NotPositiveSemidefinite
        );
    }

    #[test]
    fn assembled_rows_are_symmetric_and_retain_asymmetric_rotational_coupling() {
        let mut body_a =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body_a.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(
                1.0,
                SVector::new(2.0, 1.0, 0.5),
            )
            .unwrap(),
        );
        let body_b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let bodies = vec![body_a, body_b];
        let rows = vec![
            FrictionlessContactRow {
                body_a: BodyHandle(0),
                body_b: BodyHandle(1),
                offset_a: SVector::new(0.0, 1.0, 0.0),
                offset_b: SVector::new(0.0, 1.0, 0.0),
                normal: SVector::new(1.0, 0.0, 0.0),
            },
            FrictionlessContactRow {
                body_a: BodyHandle(0),
                body_b: BodyHandle(1),
                offset_a: SVector::new(1.0, 0.0, 0.0),
                offset_b: SVector::new(1.0, 0.0, 0.0),
                normal: SVector::new(0.0, 1.0, 0.0),
            },
        ];

        let matrix = assemble_delassus(&bodies, &rows, 1e-12).unwrap();
        assert!((matrix[(0, 1)] - matrix[(1, 0)]).abs() < 1e-12);
        assert!(matrix[(0, 1)].abs() > 1e-6, "expected coupled off-diagonal response");
        let eigenvalues = nalgebra::linalg::SymmetricEigen::new(
            (&matrix + matrix.transpose()) * 0.5,
        )
        .eigenvalues;
        let minimum_eigenvalue = eigenvalues.iter().fold(f64::INFINITY, |min, value| min.min(*value));
        assert!(minimum_eigenvalue >= -1e-12);

        // Solve a constructed impact target from the assembled operator. This
        // closes the loop between generalized impulse response and the LCP
        // oracle rather than testing the two pieces only in isolation.
        let expected_impulses = DVector::from_vec(vec![0.25, 0.1]);
        let q = -(&matrix * &expected_impulses);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-10).unwrap();
        assert!((solution.impulses - expected_impulses).norm() < 1e-9);
        assert!(solution.residual.norm() < 1e-9);
    }

    #[test]
    fn isolated_sphere_contact_has_analytic_delassus_diagonal() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 2.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 2.0),
        ];
        let rows = vec![FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::zeros(),
            normal: SVector::new(1.0, 0.0, 0.0),
        }];
        let matrix = assemble_delassus(&bodies, &rows, 1e-12).unwrap();
        assert!((matrix[(0, 0)] - 1.0).abs() < 1e-12);
    }
}
