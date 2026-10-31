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
use crate::contact_frame::ContactFrame;
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

/// A complete normal/tangent/rolling/torsional contact-wrench row.
#[derive(Clone, Debug)]
pub struct ContactWrenchRow<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub offset_a: SVector<f64, D>,
    pub offset_b: SVector<f64, D>,
    pub frame: ContactFrame<D>,
}

#[derive(Clone, Debug)]
enum ContactWrenchBasis<const D: usize> {
    Linear(SVector<f64, D>),
    Angular(Bivector<D>),
}

fn contact_wrench_basis<const D: usize>(frame: &ContactFrame<D>) -> Vec<ContactWrenchBasis<D>> {
    let mut basis = Vec::with_capacity(D * (D + 1) / 2);
    basis.push(ContactWrenchBasis::Linear(*frame.normal()));
    basis.extend(
        frame
            .tangent_basis()
            .iter()
            .copied()
            .map(ContactWrenchBasis::Linear),
    );
    basis.extend(
        frame
            .rolling_basis()
            .iter()
            .cloned()
            .map(ContactWrenchBasis::Angular),
    );
    basis.extend(
        frame
            .torsional_basis()
            .iter()
            .cloned()
            .map(ContactWrenchBasis::Angular),
    );
    basis
}

fn bivector_inner_product<const D: usize>(left: &Bivector<D>, right: &Bivector<D>) -> f64 {
    let mut result = 0.0;
    for i in 0..D {
        for j in (i + 1)..D {
            result += left.get(i, j) * right.get(i, j);
        }
    }
    result
}

fn contact_wrench_body_response<const D: usize>(
    body: &RigidBody<D>,
    linear_impulse: &SVector<f64, D>,
    angular_impulse: &Bivector<D>,
    impulse_offset: &SVector<f64, D>,
    sample_offset: &SVector<f64, D>,
    relative_tolerance: f64,
) -> Result<(SVector<f64, D>, Bivector<D>), RotationalInertiaError> {
    if !body.is_dynamic() {
        return Ok((SVector::zeros(), Bivector::zero()));
    }

    let linear_response = linear_impulse * body.inv_mass;
    let lever_arm_angular_impulse = Bivector::from_wedge(linear_impulse, impulse_offset);
    let total_angular_impulse = angular_impulse.add(&lever_arm_angular_impulse);
    let physical_angular_response = if let Some(inertia) = &body.rotational_inertia {
        inertia.validate_body_mass(body.mass, relative_tolerance)?;
        inertia
            .rotated_by(&body.transform.rotation)
            .angular_velocity_from_momentum(&total_angular_impulse, relative_tolerance)?
    } else {
        let inverse_mean_inertia = body.inv_inertia.sum() / D as f64;
        total_angular_impulse.scale(inverse_mean_inertia)
    };

    // Convert the physical angular response to the stored state sign only at
    // the shared point-velocity helper boundary.
    let stored_angular_response = physical_angular_response.scale(-1.0);
    let point_velocity_response =
        linear_response + angular_point_velocity(&stored_angular_response, sample_offset);
    Ok((point_velocity_response, physical_angular_response))
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
/// w >= 0, and lambda_i*w_i = 0 for every row. The reported primal violation
/// is dimensionless, normalized separately by impulse and velocity scales.
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
        inertia.validate_body_mass(body.mass, relative_tolerance)?;
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

/// Assemble the complete contact-wrench Delassus matrix for normal,
/// tangential, rolling and torsional contact coordinates.
///
/// Each row contributes D linear coordinates (normal plus D-1 tangents) and
/// D(D-1)/2 angular coordinates (rolling plus torsion), for D(D+1)/2 total.
/// The matrix retains cross-coupling between every channel and both bodies.
/// This is a reference assembler only: it does not select a friction law or
/// solve the resulting coupled contact problem.
pub fn assemble_contact_wrench_delassus<const D: usize>(
    bodies: &[RigidBody<D>],
    rows: &[ContactWrenchRow<D>],
    relative_tolerance: f64,
) -> Result<DMatrix<f64>, DelassusAssemblyError> {
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 {
        return Err(DelassusAssemblyError::InvalidTolerance);
    }
    if D < 2 || rows.is_empty() {
        return Err(DelassusAssemblyError::InvalidRow);
    }

    let mut row_bases = Vec::with_capacity(rows.len());
    let mut coordinate_ranges = Vec::with_capacity(rows.len());
    let mut total_coordinates = 0usize;
    for row in rows {
        if row.body_a == row.body_b
            || !row.offset_a.iter().all(|value| value.is_finite())
            || !row.offset_b.iter().all(|value| value.is_finite())
        {
            return Err(DelassusAssemblyError::InvalidRow);
        }
        if !bodies.iter().any(|body| body.handle == row.body_a) {
            return Err(DelassusAssemblyError::MissingBody(row.body_a));
        }
        if !bodies.iter().any(|body| body.handle == row.body_b) {
            return Err(DelassusAssemblyError::MissingBody(row.body_b));
        }
        let basis = contact_wrench_basis(&row.frame);
        if basis.len() != D * (D + 1) / 2 {
            return Err(DelassusAssemblyError::InvalidRow);
        }
        coordinate_ranges.push(total_coordinates..total_coordinates + basis.len());
        total_coordinates += basis.len();
        row_bases.push(basis);
    }

    let mut matrix = DMatrix::<f64>::zeros(total_coordinates, total_coordinates);
    for (row_index_i, row_i) in rows.iter().enumerate() {
        for (local_i, basis_i) in row_bases[row_index_i].iter().enumerate() {
            let global_i = coordinate_ranges[row_index_i].start + local_i;
            for (row_index_j, row_j) in rows.iter().enumerate() {
                for (local_j, basis_j) in row_bases[row_index_j].iter().enumerate() {
                    let global_j = coordinate_ranges[row_index_j].start + local_j;
                    let mut relative_linear_response = SVector::<f64, D>::zeros();
                    let mut relative_angular_response = Bivector::<D>::zero();

                    for (body_handle, sample_offset, relative_sign) in [
                        (row_i.body_a, &row_i.offset_a, -1.0),
                        (row_i.body_b, &row_i.offset_b, 1.0),
                    ] {
                        let body = bodies
                            .iter()
                            .find(|body| body.handle == body_handle)
                            .ok_or(DelassusAssemblyError::MissingBody(body_handle))?;

                        let impulse_sign = if body_handle == row_j.body_a {
                            -1.0
                        } else if body_handle == row_j.body_b {
                            1.0
                        } else {
                            continue;
                        };

                        let (linear_impulse, angular_impulse, impulse_offset) = match basis_j {
                            ContactWrenchBasis::Linear(direction) => (
                                direction * impulse_sign,
                                Bivector::zero(),
                                if impulse_sign < 0.0 {
                                    &row_j.offset_a
                                } else {
                                    &row_j.offset_b
                                },
                            ),
                            ContactWrenchBasis::Angular(angular) => (
                                SVector::zeros(),
                                angular.scale(impulse_sign),
                                if impulse_sign < 0.0 {
                                    &row_j.offset_a
                                } else {
                                    &row_j.offset_b
                                },
                            ),
                        };

                        let (point_response, angular_response) = contact_wrench_body_response(
                            body,
                            &linear_impulse,
                            &angular_impulse,
                            impulse_offset,
                            sample_offset,
                            relative_tolerance,
                        )
                        .map_err(DelassusAssemblyError::RotationalInertia)?;
                        relative_linear_response += point_response * relative_sign;
                        relative_angular_response = relative_angular_response
                            .add(&angular_response.scale(relative_sign));
                    }

                    matrix[(global_i, global_j)] = match basis_i {
                        ContactWrenchBasis::Linear(direction) => {
                            direction.dot(&relative_linear_response)
                        }
                        ContactWrenchBasis::Angular(angular) => {
                            bivector_inner_product(angular, &relative_angular_response)
                        }
                    };
                }
            }
        }
    }

    if !matrix.iter().all(|value| value.is_finite()) {
        return Err(DelassusAssemblyError::RotationalInertia(
            RotationalInertiaError::NonFinite,
        ));
    }
    Ok(matrix)
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
    if !matrix.iter().all(|value| value.is_finite()) || !q.iter().all(|value| value.is_finite()) {
        return Err(ReferenceLcpError::NonFinite);
    }

    let matrix_scale = matrix.norm().max(f64::MIN_POSITIVE);
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
    let min_eigenvalue = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
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
            let rhs =
                DVector::<f64>::from_iterator(active.len(), active.iter().map(|index| -q[*index]));
            // SVD supplies a least-norm solution for rank-deficient active
            // blocks (e.g. redundant contact rows). Feasibility and the
            // original unregularized equations are checked below; no diagonal
            // regularizer is added to conceal a singular physical system.
            let active_scale = active_matrix.norm().max(f64::MIN_POSITIVE);
            let svd_tolerance = tolerance * active_scale;
            let Ok(active_solution) = active_matrix.svd(true, true).solve(&rhs, svd_tolerance)
            else {
                continue;
            };
            for (local_index, global_index) in active.iter().enumerate() {
                impulses[*global_index] = active_solution[local_index];
            }
        }

        let impulse_scale = impulses.norm().max(f64::MIN_POSITIVE);
        let impulse_tolerance = tolerance * impulse_scale;
        let max_impulse_violation = impulses
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max((-*value).max(0.0)));
        if impulses.iter().any(|value| *value < -impulse_tolerance) {
            continue;
        }
        for value in impulses.iter_mut() {
            if *value < 0.0 {
                *value = 0.0;
            }
        }

        let residual = matrix * &impulses + q;
        let velocity_scale = (matrix_scale * impulses.norm())
            .max(q.norm())
            .max(f64::MIN_POSITIVE);
        let velocity_tolerance = tolerance * velocity_scale;
        if residual.iter().any(|value| *value < -velocity_tolerance) {
            continue;
        }
        let max_velocity_violation = residual
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max((-*value).max(0.0)));
        // Normalize the two primal feasibility terms separately because
        // impulses and residual velocities have different physical units.
        let max_primal_violation =
            (max_impulse_violation / impulse_scale).max(max_velocity_violation / velocity_scale);
        let max_complementarity_residual = impulses
            .iter()
            .zip(residual.iter())
            .fold(0.0_f64, |maximum, (lambda, slack)| {
                maximum.max((lambda * slack).abs())
            });
        let complementarity_scale = impulse_scale * velocity_scale;
        if max_complementarity_residual > tolerance * complementarity_scale {
            continue;
        }

        let active_rows = impulses
            .iter()
            .enumerate()
            .filter_map(|(index, value)| (*value > impulse_tolerance).then_some(index))
            .collect();
        let candidate = ReferenceLcpSolution {
            impulses,
            residual,
            active_rows,
            max_primal_violation,
            max_complementarity_residual,
        };

        let candidate_score = candidate.max_primal_violation
            + candidate.max_complementarity_residual / complementarity_scale;
        let replace_best = best.as_ref().is_none_or(|current| {
            let current_impulse_scale = current.impulses.norm().max(f64::MIN_POSITIVE);
            let current_velocity_scale = (matrix_scale * current.impulses.norm())
                .max(q.norm())
                .max(f64::MIN_POSITIVE);
            let current_complementarity_scale = current_impulse_scale * current_velocity_scale;
            let current_score = current.max_primal_violation
                + current.max_complementarity_residual / current_complementarity_scale;
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
    use nalgebra::{DMatrix, DVector, SVector};
    use symtropy_math::{Point, Rotor};

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
    fn lcp_reference_is_stable_under_common_coefficient_scaling() {
        for scale in [1e-15_f64, 1.0, 1e15] {
            let matrix = DMatrix::from_element(1, 1, 2.0 * scale);
            let q = DVector::from_vec(vec![-scale]);
            let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();
            assert!(
                (solution.impulses[0] - 0.5).abs() < 1e-12,
                "scale={scale:e}, impulse={}",
                solution.impulses[0]
            );
            assert!(solution.residual[0].abs() <= 1e-12 * scale);
        }
    }

    #[test]
    fn lcp_reference_preserves_impulse_under_si_to_g_cm_ms_conversion() {
        let si_matrix = DMatrix::from_element(1, 1, 2.0);
        let si_q = DVector::from_vec(vec![-1.0]);
        let si_solution = solve_frictionless_lcp(&si_matrix, &si_q, 1e-12).unwrap();

        // kg -> g multiplies mass by 1,000; m -> cm multiplies length by
        // 100; s -> ms multiplies time by 1,000. Thus the numeric inverse-mass
        // matrix scales by 1/1,000, velocity by 0.1, and impulse by 100.
        let converted_matrix = &si_matrix * 1e-3;
        let converted_q = &si_q * 0.1;
        let converted_solution =
            solve_frictionless_lcp(&converted_matrix, &converted_q, 1e-12).unwrap();

        assert!((si_solution.impulses[0] - 0.5).abs() < 1e-12);
        assert!((converted_solution.impulses[0] - 100.0 * si_solution.impulses[0]).abs() < 1e-9);
        assert!(converted_solution.residual[0].abs() < 1e-12);
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
        let mut body_a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body_a.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap(),
        );
        let body_b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
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
        assert!(
            matrix[(0, 1)].abs() > 1e-6,
            "expected coupled off-diagonal response"
        );
        let eigenvalues =
            nalgebra::linalg::SymmetricEigen::new((&matrix + matrix.transpose()) * 0.5).eigenvalues;
        let minimum_eigenvalue = eigenvalues
            .iter()
            .fold(f64::INFINITY, |min, value| min.min(*value));
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
    fn delassus_is_invariant_to_contact_permutation_and_global_rigid_rotation() {
        let mut body_a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body_a.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap(),
        );
        let mut body_b =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 0.5, -0.25]), 0.5, 1.5);
        body_b.transform.rotation = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(1, 2), 0.37);
        body_b.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.5, SVector::new(0.8, 1.2, 1.7))
                .unwrap(),
        );

        let bodies = vec![body_a, body_b];
        let rows = vec![
            FrictionlessContactRow {
                body_a: BodyHandle(0),
                body_b: BodyHandle(1),
                offset_a: SVector::new(0.2, -0.1, 0.3),
                offset_b: SVector::new(-0.1, 0.25, 0.2),
                normal: SVector::new(1.0, 0.0, 0.0),
            },
            FrictionlessContactRow {
                body_a: BodyHandle(0),
                body_b: BodyHandle(1),
                offset_a: SVector::new(-0.15, 0.2, 0.1),
                offset_b: SVector::new(0.3, 0.1, -0.2),
                normal: SVector::new(0.0, 1.0, 0.0),
            },
            FrictionlessContactRow {
                body_a: BodyHandle(0),
                body_b: BodyHandle(1),
                offset_a: SVector::new(0.1, 0.15, -0.25),
                offset_b: SVector::new(0.2, -0.3, 0.1),
                normal: SVector::new(0.0, 0.0, 1.0),
            },
        ];
        let reference = assemble_delassus(&bodies, &rows, 1e-12).unwrap();

        // Reordering contact rows must only apply the same permutation to W.
        let permutation = [2_usize, 0, 1];
        let permuted_rows: Vec<_> = permutation
            .iter()
            .map(|index| rows[*index].clone())
            .collect();
        let permuted = assemble_delassus(&bodies, &permuted_rows, 1e-12).unwrap();
        for i in 0..permutation.len() {
            for j in 0..permutation.len() {
                assert!(
                    (permuted[(i, j)] - reference[(permutation[i], permutation[j])]).abs() < 1e-10,
                    "row permutation mismatch at ({i}, {j})"
                );
            }
        }

        // A common rigid rotation of every world-space quantity cannot change
        // the scalar Delassus operator.
        let global_rotation = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.61);
        let rotated_bodies: Vec<_> = bodies
            .iter()
            .map(|body| {
                let mut rotated = RigidBody::<3>::dynamic_sphere(
                    body.handle,
                    Point::new(global_rotation.rotate_vector(&body.transform.translation.0)),
                    0.5,
                    body.mass,
                );
                rotated.transform.rotation = global_rotation.compose(&body.transform.rotation);
                rotated.linear_velocity = global_rotation.rotate_vector(&body.linear_velocity);
                rotated.angular_velocity = body.angular_velocity.clone();
                rotated.inv_mass = body.inv_mass;
                rotated.inv_inertia = body.inv_inertia;
                rotated.friction = body.friction;
                rotated.restitution = body.restitution;
                rotated.set_rotational_inertia_operator(
                    body.rotational_inertia
                        .as_ref()
                        .expect("fixture carries authoritative inertia")
                        .clone(),
                );
                rotated
            })
            .collect();
        let rotated_rows: Vec<_> = rows
            .iter()
            .map(|row| FrictionlessContactRow {
                body_a: row.body_a,
                body_b: row.body_b,
                offset_a: global_rotation.rotate_vector(&row.offset_a),
                offset_b: global_rotation.rotate_vector(&row.offset_b),
                normal: global_rotation.rotate_vector(&row.normal),
            })
            .collect();
        let rotated = assemble_delassus(&rotated_bodies, &rotated_rows, 1e-12).unwrap();
        assert!(
            (&rotated - &reference).norm() < 1e-10,
            "global-rotation covariance error={}",
            (&rotated - &reference).norm()
        );
    }

    fn assert_isotropic_contact_wrench_diagonal<const D: usize>() {
        let bodies = vec![
            RigidBody::<D>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 2.0),
            RigidBody::<D>::dynamic_sphere(BodyHandle(1), Point::origin(), 1.0, 2.0),
        ];
        let mut normal = SVector::<f64, D>::zeros();
        normal[0] = 1.0;
        let row = ContactWrenchRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::zeros(),
            frame: ContactFrame::<D>::new(normal, 1e-12).unwrap(),
        };
        let matrix = assemble_contact_wrench_delassus(&bodies, &[row], 1e-12).unwrap();
        let coordinate_count = D * (D + 1) / 2;
        assert_eq!(matrix.nrows(), coordinate_count);
        assert_eq!(matrix.ncols(), coordinate_count);

        // Two 2 kg unit-radius D-balls give a translational inverse-mass sum
        // of 1 and an angular inverse-inertia sum of 2 / (4/(D+2)).
        let angular_diagonal = (D as f64 + 2.0) / 2.0;
        for coordinate in 0..D {
            assert!(
                (matrix[(coordinate, coordinate)] - 1.0).abs() < 1e-10,
                "D={D}, linear coordinate={coordinate}, diagonal={}",
                matrix[(coordinate, coordinate)]
            );
        }
        for coordinate in D..coordinate_count {
            assert!(
                (matrix[(coordinate, coordinate)] - angular_diagonal).abs() < 1e-10,
                "D={D}, angular coordinate={coordinate}, diagonal={}",
                matrix[(coordinate, coordinate)]
            );
        }
        for i in 0..coordinate_count {
            for j in 0..coordinate_count {
                if i != j {
                    assert!(
                        matrix[(i, j)].abs() < 1e-10,
                        "D={D}, off-diagonal ({i},{j})={}",
                        matrix[(i, j)]
                    );
                }
            }
        }
    }

    #[test]
    fn contact_wrench_delassus_matches_isotropic_analytic_metric_in_2d_3d_4d() {
        assert_isotropic_contact_wrench_diagonal::<2>();
        assert_isotropic_contact_wrench_diagonal::<3>();
        assert_isotropic_contact_wrench_diagonal::<4>();
    }

    #[test]
    fn asymmetric_contact_wrench_delassus_is_symmetric_psd_and_coupled() {
        use crate::body::BodyType;
        use symtropy_math::{Sphere, Transform};

        let static_body = RigidBody::<3>::new(
            BodyHandle(0),
            BodyType::Static,
            Transform::identity(),
            Box::new(Sphere::new(Point::origin(), 0.5)),
            0.0,
            SVector::zeros(),
        );
        let mut dynamic_body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        dynamic_body.try_set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(
                1.0,
                SVector::new(1.0, 0.8, 0.6),
            )
            .unwrap(),
        )
        .unwrap();
        let bodies = vec![static_body, dynamic_body];
        let frame =
            ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let row = ContactWrenchRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::new(1.0, 0.0, 0.0),
            frame,
        };

        let matrix = assemble_contact_wrench_delassus(&bodies, &[row], 1e-12).unwrap();
        let scale = matrix.norm().max(f64::MIN_POSITIVE);
        assert!(
            (&matrix - matrix.transpose()).norm() <= 1e-10 * scale,
            "contact-wrench metric is not symmetric"
        );
        let eigenvalues =
            nalgebra::linalg::SymmetricEigen::new((&matrix + matrix.transpose()) * 0.5)
                .eigenvalues;
        let minimum_eigenvalue = eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        assert!(
            minimum_eigenvalue >= -1e-10 * scale,
            "contact-wrench metric is not PSD: min eigenvalue={minimum_eigenvalue}"
        );

        // Coordinate 1 is the first tangent, coordinate 3 the first rolling
        // direction n∧t. An off-centre tangent impulse creates exactly this
        // rolling angular response, so the full metric must retain it.
        assert!(
            matrix[(1, 3)].abs() > 1e-6,
            "expected tangent/rolling coupling, got {}",
            matrix[(1, 3)]
        );
    }

    #[test]
    fn contact_wrench_delassus_is_invariant_to_global_rigid_rotation() {
        use crate::body::BodyType;
        use symtropy_math::{Sphere, Transform};

        let mut static_body = RigidBody::<3>::new(
            BodyHandle(0),
            BodyType::Static,
            Transform::identity(),
            Box::new(Sphere::new(Point::origin(), 0.5)),
            0.0,
            SVector::zeros(),
        );
        let mut dynamic_body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        dynamic_body
            .try_set_rotational_inertia_operator(
                BivectorInertia::<3>::uniform_solid_ellipsoid(
                    1.0,
                    SVector::new(1.0, 0.8, 0.6),
                )
                .unwrap(),
            )
            .unwrap();
        let frame =
            ContactFrame::<3>::new(SVector::new(1.0, 0.0, 0.0), 1e-12).unwrap();
        let row = ContactWrenchRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::new(1.0, 0.0, 0.0),
            frame,
        };
        let mut bodies = vec![static_body, dynamic_body];
        let reference = assemble_contact_wrench_delassus(&bodies, &[row.clone()], 1e-12)
            .unwrap();

        let rotation =
            Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.53);
        for body in &mut bodies {
            body.transform.translation.0 =
                rotation.rotate_vector(&body.transform.translation.0);
            body.transform.rotation = rotation.compose(&body.transform.rotation);
        }
        let rotated_row = ContactWrenchRow {
            body_a: row.body_a,
            body_b: row.body_b,
            offset_a: rotation.rotate_vector(&row.offset_a),
            offset_b: rotation.rotate_vector(&row.offset_b),
            frame: row.frame.rotated_by(&rotation),
        };
        let rotated = assemble_contact_wrench_delassus(
            &bodies,
            &[rotated_row],
            1e-12,
        )
        .unwrap();
        assert!(
            (&rotated - reference).norm() < 1e-10,
            "global rotation changed contact-wrench metric by {}",
            (&rotated - reference).norm()
        );
    }

    #[test]
    fn production_single_contact_impulse_matches_delassus_lcp_oracle() {
        use crate::world::PhysicsWorld;

        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        let body_a = world.add_sphere(Point::origin(), 0.5, 1.0);
        let body_b = world.add_sphere(Point::new([0.999, 0.0, 0.0]), 0.5, 1.0);
        world.bodies[0].linear_velocity = SVector::new(2.0, 0.0, 0.0);
        world.bodies[0].restitution = 0.2;
        world.bodies[1].restitution = 0.2;
        world.bodies[0].friction = 0.0;
        world.bodies[1].friction = 0.0;

        let relative_velocity_before =
            (world.bodies[1].linear_velocity - world.bodies[0].linear_velocity);
        world.try_step(0.001).unwrap();
        assert_eq!(world.contacts.len(), 1);

        let manifold = &world.contacts[0];
        let point = manifold.primary_point();
        let row = FrictionlessContactRow {
            body_a,
            body_b,
            offset_a: point.position - world.bodies[0].position(),
            offset_b: point.position - world.bodies[1].position(),
            normal: manifold.normal,
        };
        let matrix = assemble_delassus(&world.bodies, &[row], 1e-12).unwrap();
        let relative_normal_speed = relative_velocity_before.dot(&manifold.normal);
        let restitution_bias = if relative_normal_speed < -1.0 {
            -0.2 * relative_normal_speed
        } else {
            0.0
        };
        let q = DVector::from_vec(vec![relative_normal_speed - restitution_bias]);
        let reference = solve_frictionless_lcp(&matrix, &q, 1e-10).unwrap();
        let applied_impulse = manifold.primary_point().lambda;

        assert!(
            (applied_impulse - reference.impulses[0]).abs() < 1e-8,
            "production impulse={applied_impulse}, oracle={}",
            reference.impulses[0]
        );
    }

    #[test]
    fn production_asymmetric_box_plane_contact_matches_lcp_velocity_solution() {
        use crate::body::BodyType;
        use crate::world::PhysicsWorld;
        use nalgebra::SMatrix;
        use symtropy_math::{HalfSpace, HyperBox, Transform};

        let mut world = PhysicsWorld::<3>::new(SVector::zeros());
        world.solver_iterations = 128;
        world.compliance = 0.0;
        world.baumgarte = 0.0;

        let floor = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::origin(),
            Box::new(HalfSpace::<3>::new(SVector::new(1.0, 0.0, 0.0), 0.0)),
        );
        let floor_handle = world.add_body(floor);

        let mut block = RigidBody::<3>::new(
            BodyHandle(1),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([0.499, 0.0, 0.0])),
            Box::new(HyperBox::<3>::cube(0.5)),
            1.0,
            SVector::from_element(1.0),
        );
        block.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(1.0, 0.7, 0.4))
                .unwrap(),
        );
        block.linear_velocity = SVector::new(-0.1, 0.0, 0.0);
        block.friction = 0.0;
        block.restitution = 0.0;
        let block_handle = world.add_body(block);
        world.bodies[0].friction = 0.0;
        world.bodies[0].restitution = 0.0;

        world.try_step(0.001).unwrap();
        let manifold = world
            .contacts
            .iter()
            .find(|contact| contact.body_a == floor_handle && contact.body_b == block_handle)
            .expect("box should contact the half-space");
        assert!(
            manifold.points.len() >= 3,
            "expected a multi-point face contact, got {} points",
            manifold.points.len()
        );

        let rows: Vec<_> = manifold
            .points
            .iter()
            .map(|point| FrictionlessContactRow {
                body_a: floor_handle,
                body_b: block_handle,
                offset_a: point.position - world.bodies[0].position(),
                offset_b: point.position - world.bodies[1].position(),
                normal: manifold.normal,
            })
            .collect();
        let matrix = assemble_delassus(&world.bodies, &rows, 1e-12).unwrap();
        // Baumgarte and restitution are disabled above; q is the pre-contact
        // normal relative velocity for every point. The row set is redundant
        // because multiple coplanar points share only three normal-response
        // degrees of freedom, so the LCP impulse vector need not be unique.
        let q = DVector::from_vec(vec![-0.1; rows.len()]);
        let reference = solve_frictionless_lcp(&matrix, &q, 1e-10).unwrap();
        let production_impulses = DVector::from_iterator(
            manifold.points.len(),
            manifold.points.iter().map(|p| p.lambda),
        );
        let production_residual = &matrix * &production_impulses + &q;

        let min_production_residual = production_residual
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        let max_production_complementarity = production_impulses
            .iter()
            .zip(production_residual.iter())
            .fold(0.0_f64, |maximum, (impulse, residual)| {
                maximum.max((impulse * residual).abs())
            });

        assert!(
            production_impulses.iter().all(|impulse| *impulse >= -1e-12),
            "production solver pulled at a unilateral contact"
        );
        assert!(
            min_production_residual >= -1e-6,
            "production normal residual={min_production_residual}"
        );
        assert!(
            max_production_complementarity < 1e-5,
            "production complementarity residual={max_production_complementarity}"
        );
        assert!(
            (&production_residual - &reference.residual).norm() < 1e-5,
            "production and reference velocity residuals differ: production={}, reference={}",
            production_residual.norm(),
            reference.residual.norm()
        );
    }

    #[test]
    fn assembler_propagates_singular_rotational_plane_error() {
        use crate::inertia::RotationalInertiaError;
        use nalgebra::SMatrix;

        let mut body_a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body_a.set_rotational_inertia_operator(
            BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::from_diagonal(
                &SVector::new(1.0, 0.0, 0.0),
            ))
            .unwrap(),
        );
        let body_b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let bodies = vec![body_a, body_b];
        let rows = vec![FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::new(0.0, 0.0, 1.0),
            offset_b: SVector::zeros(),
            normal: SVector::new(0.0, 1.0, 0.0),
        }];

        assert_eq!(
            assemble_delassus(&bodies, &rows, 1e-12).unwrap_err(),
            DelassusAssemblyError::RotationalInertia(RotationalInertiaError::SingularInertia)
        );
    }

    #[test]
    fn delassus_reference_rejects_body_inertia_mass_mismatch() {
        let mut body_a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
        body_a.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.0, 0.8, 0.6))
                .unwrap(),
        );
        let body_b = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0);
        let bodies = vec![body_a, body_b];
        let rows = vec![FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::zeros(),
            normal: SVector::new(1.0, 0.0, 0.0),
        }];

        assert_eq!(
            assemble_delassus(&bodies, &rows, 1e-12).unwrap_err(),
            DelassusAssemblyError::RotationalInertia(
                crate::inertia::RotationalInertiaError::MassMismatch
            )
        );
    }

    #[test]
    fn assembler_enforces_eight_row_reference_limit() {
        let bodies = vec![
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0),
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.0),
        ];
        let row = FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::zeros(),
            normal: SVector::new(1.0, 0.0, 0.0),
        };
        let rows = vec![row; 9];
        assert_eq!(
            assemble_delassus(&bodies, &rows, 1e-12).unwrap_err(),
            DelassusAssemblyError::TooManyRows
        );
    }

    fn assert_isolated_sphere_delassus<const D: usize>() {
        let bodies = vec![
            RigidBody::<D>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 2.0),
            RigidBody::<D>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 2.0),
        ];
        let mut normal = SVector::<f64, D>::zeros();
        normal[0] = 1.0;
        let rows = vec![FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a: SVector::zeros(),
            offset_b: SVector::zeros(),
            normal,
        }];
        let matrix = assemble_delassus(&bodies, &rows, 1e-12).unwrap();
        assert!(
            (matrix[(0, 0)] - 1.0).abs() < 1e-12,
            "D={D}: diagonal={}",
            matrix[(0, 0)]
        );
    }

    #[test]
    fn isolated_sphere_contact_has_analytic_delassus_diagonal_in_2d_3d_4d() {
        assert_isolated_sphere_delassus::<2>();
        assert_isolated_sphere_delassus::<3>();
        assert_isolated_sphere_delassus::<4>();
    }

    fn deterministic_sample(state: &mut u64) -> f64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        ((*state >> 11) as f64 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
    }

    fn assert_randomized_delassus_invariants<const D: usize>() {
        for seed in 1_u64..=12 {
            let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let extents_a =
                SVector::<f64, D>::from_fn(|_, _| 0.35 + deterministic_sample(&mut state).abs());
            let extents_b =
                SVector::<f64, D>::from_fn(|_, _| 0.4 + deterministic_sample(&mut state).abs());
            let mut body_a =
                RigidBody::<D>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 0.8);
            body_a.set_rotational_inertia_operator(
                BivectorInertia::<D>::uniform_solid_ellipsoid(0.8, extents_a).unwrap(),
            );
            let mut body_b =
                RigidBody::<D>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.5, 1.3);
            body_b.transform.rotation = Rotor::from_plane_angle(
                &Bivector::<D>::unit_plane(0, 1),
                deterministic_sample(&mut state) * 1.2,
            );
            body_b.set_rotational_inertia_operator(
                BivectorInertia::<D>::uniform_solid_ellipsoid(1.3, extents_b).unwrap(),
            );
            let bodies = vec![body_a, body_b];

            let row_count = 2 + (seed as usize % 5);
            let mut rows = Vec::with_capacity(row_count);
            for _ in 0..row_count {
                let mut normal =
                    SVector::<f64, D>::from_fn(|_, _| deterministic_sample(&mut state));
                let norm = normal.norm();
                if norm <= 1e-8 {
                    normal[0] = 1.0;
                } else {
                    normal /= norm;
                }
                let offset_a =
                    SVector::<f64, D>::from_fn(|_, _| 0.4 * deterministic_sample(&mut state));
                let offset_b =
                    SVector::<f64, D>::from_fn(|_, _| 0.4 * deterministic_sample(&mut state));
                rows.push(FrictionlessContactRow {
                    body_a: BodyHandle(0),
                    body_b: BodyHandle(1),
                    offset_a,
                    offset_b,
                    normal,
                });
            }

            let matrix = assemble_delassus(&bodies, &rows, 1e-12)
                .unwrap_or_else(|error| panic!("D={D}, seed={seed}: {error:?}"));
            let matrix_scale = matrix.norm().max(f64::MIN_POSITIVE);
            let mut symmetry_error = 0.0_f64;
            for i in 0..row_count {
                for j in 0..row_count {
                    symmetry_error = symmetry_error.max((matrix[(i, j)] - matrix[(j, i)]).abs());
                }
            }
            assert!(
                symmetry_error <= 1e-10 * matrix_scale,
                "D={D}, seed={seed}: symmetry error={symmetry_error}"
            );

            let eigenvalues =
                nalgebra::linalg::SymmetricEigen::new((&matrix + matrix.transpose()) * 0.5)
                    .eigenvalues;
            let minimum_eigenvalue = eigenvalues
                .iter()
                .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
            assert!(
                minimum_eigenvalue >= -1e-10 * matrix_scale,
                "D={D}, seed={seed}: min eigenvalue={minimum_eigenvalue}"
            );

            let permutation: Vec<usize> = (0..row_count).rev().collect();
            let permuted_rows: Vec<_> = permutation
                .iter()
                .map(|index| rows[*index].clone())
                .collect();
            let permuted = assemble_delassus(&bodies, &permuted_rows, 1e-12).unwrap();
            for i in 0..row_count {
                for j in 0..row_count {
                    assert!(
                        (permuted[(i, j)] - matrix[(permutation[i], permutation[j])]).abs()
                            <= 1e-10 * matrix_scale,
                        "D={D}, seed={seed}: row permutation mismatch ({i}, {j})"
                    );
                }
            }

            // Construct a feasible LCP from a positive impulse vector. This
            // tests the oracle even when redundant rows make impulses
            // non-unique: residual and complementarity, not one arbitrary
            // impulse distribution, are the invariant.
            let target_impulses = DVector::from_iterator(
                row_count,
                (0..row_count).map(|index| 0.05 + index as f64 * 0.01),
            );
            let q = -(&matrix * target_impulses);
            let solution = solve_frictionless_lcp(&matrix, &q, 1e-9)
                .unwrap_or_else(|error| panic!("D={D}, seed={seed}: {error:?}"));
            assert!(
                solution.impulses.iter().all(|value| *value >= -1e-9),
                "D={D}, seed={seed}: negative reference impulse"
            );
            assert!(
                solution.residual.iter().all(|value| *value >= -1e-8),
                "D={D}, seed={seed}: negative reference slack"
            );
            assert!(
                solution.max_complementarity_residual < 1e-8,
                "D={D}, seed={seed}: complementarity={}",
                solution.max_complementarity_residual
            );
        }
    }

    #[test]
    fn randomized_delassus_invariants_hold_in_2d_3d_and_4d() {
        assert_randomized_delassus_invariants::<2>();
        assert_randomized_delassus_invariants::<3>();
        assert_randomized_delassus_invariants::<4>();
    }
}
