// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Implicit sensitivities of a converged, fixed-active-set frictionless LCP.
//!
//! This is a research primitive, not a differentiable simulator. It computes
//! derivatives through the active contact equations after a reference LCP
//! solve. It intentionally rejects active-set transitions and ill-conditioned
//! active blocks instead of returning misleading gradients.

use nalgebra::{DMatrix, DVector, Matrix2, SVector};

use crate::delassus::ReferenceLcpSolution;
use crate::friction_cone::{PlanarContactMode, PlanarFrictionConeSolution};

/// Failure to obtain a locally valid implicit contact derivative.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactAdjointError {
    InvalidDimensions,
    InvalidTolerance,
    NonFinite,
    InconsistentPrimalSolution,
    NonMonotoneMatrix,
    ActiveSetBoundary,
    IllConditionedActiveBlock,
    SingularAdjointSolve,
    PlanarModeBoundary,
    InvalidFrictionParameter,
    IllConditionedPlanarMode,
}

/// Sensitivity of a scalar objective through a fixed-active-set contact LCP.
///
/// impulse_sensitivity is d(lambda)/d(theta) for one scalar model parameter.
/// adjoint solves W_AA^T psi = dL/d(lambda_A). The scalar parameter gradient
/// includes direct_parameter_gradient plus the implicit contact contribution.
#[derive(Clone, Debug)]
pub struct ContactAdjointResult {
    pub active_rows: Vec<usize>,
    pub adjoint: DVector<f64>,
    pub impulse_sensitivity: DVector<f64>,
    pub objective_parameter_gradient: f64,
    pub active_condition_estimate: f64,
}

/// Differentiate the converged active equations:
///
///     W_AA lambda_A + q_A = 0
///
///     d(lambda_A)/d(theta) = -W_AA^-1 (dW_AA/d(theta) lambda_A + dq_A/d(theta))
///
///     W_AA^T psi = dL/d(lambda_A)
///
///     dL/d(theta) = direct - psi^T (dW_AA/d(theta) lambda_A + dq_A/d(theta))
///
/// Active-set validity is checked using caller-supplied absolute impulse and
/// velocity margins. They have different physical units and must not be
/// conflated. A derivative is rejected when any active impulse or inactive
/// slack is inside its corresponding margin.
pub fn differentiate_fixed_active_set_lcp(
    matrix: &DMatrix<f64>,
    q: &DVector<f64>,
    solution: &ReferenceLcpSolution,
    matrix_derivative: &DMatrix<f64>,
    q_derivative: &DVector<f64>,
    objective_impulse_gradient: &DVector<f64>,
    direct_parameter_gradient: f64,
    impulse_margin: f64,
    velocity_margin: f64,
    relative_tolerance: f64,
) -> Result<ContactAdjointResult, ContactAdjointError> {
    let n = q.len();
    if n == 0
        || matrix.nrows() != n
        || matrix.ncols() != n
        || matrix_derivative.nrows() != n
        || matrix_derivative.ncols() != n
        || q_derivative.len() != n
        || objective_impulse_gradient.len() != n
        || solution.impulses.len() != n
        || solution.residual.len() != n
    {
        return Err(ContactAdjointError::InvalidDimensions);
    }
    if !impulse_margin.is_finite()
        || impulse_margin <= 0.0
        || !velocity_margin.is_finite()
        || velocity_margin <= 0.0
        || !relative_tolerance.is_finite()
        || relative_tolerance <= 0.0
    {
        return Err(ContactAdjointError::InvalidTolerance);
    }
    if !matrix.iter().all(|value| value.is_finite())
        || !matrix_derivative.iter().all(|value| value.is_finite())
        || !q.iter().all(|value| value.is_finite())
        || !q_derivative.iter().all(|value| value.is_finite())
        || !solution.impulses.iter().all(|value| value.is_finite())
        || !solution.residual.iter().all(|value| value.is_finite())
        || !objective_impulse_gradient.iter().all(|value| value.is_finite())
        || !direct_parameter_gradient.is_finite()
    {
        return Err(ContactAdjointError::NonFinite);
    }

    let matrix_scale = matrix.norm().max(f64::MIN_POSITIVE);
    let asymmetry = (matrix - matrix.transpose()).norm();
    if asymmetry > relative_tolerance * matrix_scale
        || (matrix_derivative - matrix_derivative.transpose()).norm()
            > relative_tolerance * matrix_derivative.norm().max(f64::MIN_POSITIVE)
    {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }
    let symmetric_matrix = (matrix + matrix.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(symmetric_matrix).eigenvalues;
    let minimum_eigenvalue = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if minimum_eigenvalue < -relative_tolerance * matrix_scale {
        return Err(ContactAdjointError::NonMonotoneMatrix);
    }

    let reconstructed_residual = matrix * &solution.impulses + q;
    let residual_scale = reconstructed_residual
        .norm()
        .max(solution.residual.norm())
        .max(q.norm())
        .max(f64::MIN_POSITIVE);
    if (&reconstructed_residual - &solution.residual).norm()
        > relative_tolerance * residual_scale
    {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }

    let impulse_scale = solution.impulses.norm().max(f64::MIN_POSITIVE);
    let velocity_scale = reconstructed_residual
        .norm()
        .max(q.norm())
        .max(f64::MIN_POSITIVE);
    if solution
        .impulses
        .iter()
        .any(|value| *value < -relative_tolerance * impulse_scale)
        || reconstructed_residual
            .iter()
            .any(|value| *value < -relative_tolerance * velocity_scale)
    {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }
    let complementarity = solution
        .impulses
        .iter()
        .zip(reconstructed_residual.iter())
        .fold(0.0_f64, |maximum, (impulse, slack)| {
            maximum.max((impulse * slack).abs())
        });
    if complementarity > relative_tolerance * impulse_scale * velocity_scale {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }

    let active_rows = &solution.active_rows;
    let mut seen = vec![false; n];
    for &index in active_rows {
        if index >= n || seen[index] {
            return Err(ContactAdjointError::InvalidDimensions);
        }
        seen[index] = true;
        if solution.impulses[index] <= impulse_margin
            || reconstructed_residual[index].abs() > relative_tolerance * velocity_scale
        {
            return Err(ContactAdjointError::ActiveSetBoundary);
        }
    }
    for index in 0..n {
        if !seen[index]
            && (reconstructed_residual[index] <= velocity_margin
                || solution.impulses[index].abs() > relative_tolerance * impulse_scale)
        {
            return Err(ContactAdjointError::ActiveSetBoundary);
        }
    }

    let mut full_sensitivity = DVector::<f64>::zeros(n);
    let mut full_adjoint = DVector::<f64>::zeros(n);
    if active_rows.is_empty() {
        return Ok(ContactAdjointResult {
            active_rows: Vec::new(),
            adjoint: full_adjoint,
            impulse_sensitivity: full_sensitivity,
            objective_parameter_gradient: direct_parameter_gradient,
            active_condition_estimate: 1.0,
        });
    }

    let active_count = active_rows.len();
    let active_matrix = DMatrix::<f64>::from_fn(active_count, active_count, |i, j| {
        matrix[(active_rows[i], active_rows[j])]
    });
    let active_matrix_derivative =
        DMatrix::<f64>::from_fn(active_count, active_count, |i, j| {
            matrix_derivative[(active_rows[i], active_rows[j])]
        });
    let active_impulses = DVector::<f64>::from_iterator(
        active_count,
        active_rows.iter().map(|index| solution.impulses[*index]),
    );
    let active_q_derivative = DVector::<f64>::from_iterator(
        active_count,
        active_rows.iter().map(|index| q_derivative[*index]),
    );
    let active_objective_gradient = DVector::<f64>::from_iterator(
        active_count,
        active_rows
            .iter()
            .map(|index| objective_impulse_gradient[*index]),
    );

    // The implicit derivative requires a locally unique active solution.
    // Reject singular/redundant active blocks; do not regularize them.
    let singular_values = active_matrix.clone().svd(false, false).singular_values;
    let maximum_singular_value = singular_values
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(*value));
    let minimum_singular_value = singular_values
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if !maximum_singular_value.is_finite()
        || !minimum_singular_value.is_finite()
        || maximum_singular_value <= f64::MIN_POSITIVE
        || minimum_singular_value <= relative_tolerance * maximum_singular_value
    {
        return Err(ContactAdjointError::IllConditionedActiveBlock);
    }
    let active_condition_estimate = maximum_singular_value / minimum_singular_value;

    let parameter_rhs =
        active_matrix_derivative * active_impulses + active_q_derivative;
    let Some(active_sensitivity) = active_matrix
        .clone()
        .lu()
        .solve(&(-&parameter_rhs))
    else {
        return Err(ContactAdjointError::SingularAdjointSolve);
    };
    let Some(active_adjoint) = active_matrix
        .transpose()
        .lu()
        .solve(&active_objective_gradient)
    else {
        return Err(ContactAdjointError::SingularAdjointSolve);
    };

    for (local_index, &global_index) in active_rows.iter().enumerate() {
        full_sensitivity[global_index] = active_sensitivity[local_index];
        full_adjoint[global_index] = active_adjoint[local_index];
    }

    let objective_parameter_gradient =
        direct_parameter_gradient - active_adjoint.dot(&parameter_rhs);
    if !objective_parameter_gradient.is_finite()
        || !full_sensitivity.iter().all(|value| value.is_finite())
        || !full_adjoint.iter().all(|value| value.is_finite())
    {
        return Err(ContactAdjointError::NonFinite);
    }

    Ok(ContactAdjointResult {
        active_rows: active_rows.clone(),
        adjoint: full_adjoint,
        impulse_sensitivity: full_sensitivity,
        objective_parameter_gradient,
        active_condition_estimate,
    })
}

/// Forward sensitivity for one fixed mode of the planar maximum-dissipation
/// Coulomb reference. The derivative is valid only while the mode inequalities
/// remain strict; transition cases return PlanarModeBoundary.
#[derive(Clone, Debug)]
pub struct PlanarFrictionSensitivity {
    pub mode: PlanarContactMode,
    pub impulse_sensitivity: SVector<f64, 2>,
    pub objective_parameter_gradient: f64,
    pub mode_condition_estimate: f64,
}

/// Differentiate one planar friction solution with respect to a scalar model
/// parameter. The parameter may change W, free velocity q, and the friction
/// coefficient mu. Sticking differentiates the full two-equation system;
/// sliding differentiates the active normal equation plus the Coulomb edge.
/// Separation has zero contact impulse sensitivity while the positive normal
/// separating velocity remains outside its transition margin.
pub fn differentiate_planar_friction_mode(
    effective_mass: &Matrix2<f64>,
    free_velocity: &SVector<f64, 2>,
    solution: &PlanarFrictionConeSolution,
    friction_coefficient: f64,
    mass_derivative: &Matrix2<f64>,
    free_velocity_derivative: &SVector<f64, 2>,
    friction_derivative: f64,
    objective_impulse_gradient: &SVector<f64, 2>,
    direct_parameter_gradient: f64,
    impulse_margin: f64,
    velocity_margin: f64,
    friction_coefficient_margin: f64,
    relative_tolerance: f64,
) -> Result<PlanarFrictionSensitivity, ContactAdjointError> {
    if !effective_mass.iter().all(|value| value.is_finite())
        || !mass_derivative.iter().all(|value| value.is_finite())
        || !free_velocity.iter().all(|value| value.is_finite())
        || !free_velocity_derivative.iter().all(|value| value.is_finite())
        || !solution.impulse.iter().all(|value| value.is_finite())
        || !solution.post_velocity.iter().all(|value| value.is_finite())
        || !objective_impulse_gradient.iter().all(|value| value.is_finite())
        || !friction_coefficient.is_finite()
        || !friction_derivative.is_finite()
        || !direct_parameter_gradient.is_finite()
    {
        return Err(ContactAdjointError::NonFinite);
    }
    if !impulse_margin.is_finite()
        || impulse_margin <= 0.0
        || !velocity_margin.is_finite()
        || velocity_margin <= 0.0
        || !friction_coefficient_margin.is_finite()
        || friction_coefficient_margin <= 0.0
        || !relative_tolerance.is_finite()
        || relative_tolerance <= 0.0
        || friction_coefficient < 0.0
    {
        return Err(ContactAdjointError::InvalidFrictionParameter);
    }

    let matrix_scale = effective_mass.norm().max(f64::MIN_POSITIVE);
    if (effective_mass - effective_mass.transpose()).norm() > relative_tolerance * matrix_scale
        || (mass_derivative - mass_derivative.transpose()).norm()
            > relative_tolerance * mass_derivative.norm().max(f64::MIN_POSITIVE)
    {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }
    let eigenvalues =
        nalgebra::linalg::SymmetricEigen::new(*effective_mass).eigenvalues;
    let minimum_eigenvalue = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if minimum_eigenvalue <= relative_tolerance * matrix_scale {
        return Err(ContactAdjointError::NonMonotoneMatrix);
    }

    let post_velocity = effective_mass * solution.impulse + free_velocity;
    let velocity_scale = post_velocity
        .norm()
        .max(solution.post_velocity.norm())
        .max(free_velocity.norm())
        .max(f64::MIN_POSITIVE);
    let impulse_scale = solution.impulse.norm().max(f64::MIN_POSITIVE);
    if (post_velocity - solution.post_velocity).norm() > relative_tolerance * velocity_scale
        || solution.impulse[0] < -relative_tolerance * impulse_scale
        || post_velocity[0] < -relative_tolerance * velocity_scale
        || (solution.impulse[0] * post_velocity[0]).abs()
            > relative_tolerance * impulse_scale * velocity_scale
        || solution.cone_violation > relative_tolerance * impulse_scale
        || solution.frictional_work_delta
            > relative_tolerance * impulse_scale * velocity_scale
    {
        return Err(ContactAdjointError::InconsistentPrimalSolution);
    }

    let zero = SVector::<f64, 2>::zeros();
    let sensitivity;
    let condition_estimate;

    match solution.mode {
        PlanarContactMode::Separating => {
            if free_velocity[0] <= velocity_margin
                || solution.impulse.norm() > impulse_margin
            {
                return Err(ContactAdjointError::PlanarModeBoundary);
            }
            sensitivity = zero;
            condition_estimate = 1.0;
        }
        PlanarContactMode::Sticking => {
            let cone_slack =
                friction_coefficient * solution.impulse[0] - solution.impulse[1].abs();
            if solution.impulse[0] <= impulse_margin
                || cone_slack <= impulse_margin
                || solution.post_velocity[0].abs() > velocity_margin
                || solution.post_velocity[1].abs() > velocity_margin
            {
                return Err(ContactAdjointError::PlanarModeBoundary);
            }

            let singular_values = (*effective_mass).svd(false, false).singular_values;
            let maximum = singular_values.iter().fold(0.0_f64, |m, v| m.max(*v));
            let minimum = singular_values
                .iter()
                .fold(f64::INFINITY, |m, v| m.min(*v));
            if !maximum.is_finite()
                || !minimum.is_finite()
                || maximum <= f64::MIN_POSITIVE
                || minimum <= relative_tolerance * maximum
            {
                return Err(ContactAdjointError::IllConditionedPlanarMode);
            }
            condition_estimate = maximum / minimum;

            let parameter_rhs = mass_derivative * solution.impulse + free_velocity_derivative;
            let Some(value) = effective_mass.lu().solve(&(-parameter_rhs)) else {
                return Err(ContactAdjointError::SingularAdjointSolve);
            };
            sensitivity = value;
        }
        PlanarContactMode::SlidingPositive | PlanarContactMode::SlidingNegative => {
            if solution.impulse[0] <= impulse_margin
                || friction_coefficient <= friction_coefficient_margin
                || solution.post_velocity[0].abs() > velocity_margin
                || solution.post_velocity[1].abs() <= velocity_margin
            {
                return Err(ContactAdjointError::PlanarModeBoundary);
            }
            let slip_sign = if solution.mode == PlanarContactMode::SlidingPositive {
                1.0
            } else {
                -1.0
            };
            if solution.post_velocity[1] * slip_sign <= velocity_margin {
                return Err(ContactAdjointError::PlanarModeBoundary);
            }

            let cone_edge_residual = if slip_sign > 0.0 {
                solution.impulse[1] + friction_coefficient * solution.impulse[0]
            } else {
                solution.impulse[1] - friction_coefficient * solution.impulse[0]
            };
            if cone_edge_residual.abs() > relative_tolerance * impulse_scale {
                return Err(ContactAdjointError::InconsistentPrimalSolution);
            }

            let cone_direction = SVector::new(1.0, -friction_coefficient * slip_sign);
            let cone_direction_derivative = SVector::new(0.0, -friction_derivative * slip_sign);
            let metric_cone_direction = effective_mass * cone_direction;
            let denominator = metric_cone_direction[0];
            if !denominator.is_finite()
                || denominator <= relative_tolerance * matrix_scale
            {
                return Err(ContactAdjointError::IllConditionedPlanarMode);
            }
            condition_estimate = matrix_scale / denominator;

            let directional_parameter_response =
                (mass_derivative * cone_direction
                    + effective_mass * cone_direction_derivative)
                    * solution.impulse[0]
                    + free_velocity_derivative;
            let normal_impulse_derivative = -directional_parameter_response[0] / denominator;
            sensitivity =
                cone_direction * normal_impulse_derivative
                    + cone_direction_derivative * solution.impulse[0];
        }
    }

    let objective_parameter_gradient =
        direct_parameter_gradient + objective_impulse_gradient.dot(&sensitivity);
    if !objective_parameter_gradient.is_finite()
        || !sensitivity.iter().all(|value| value.is_finite())
        || !condition_estimate.is_finite()
    {
        return Err(ContactAdjointError::NonFinite);
    }

    Ok(PlanarFrictionSensitivity {
        mode: solution.mode,
        impulse_sensitivity: sensitivity,
        objective_parameter_gradient,
        mode_condition_estimate: condition_estimate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delassus::solve_frictionless_lcp;

    #[test]
    fn planar_sticking_sensitivity_matches_finite_difference() {
        let matrix = Matrix2::new(2.0, 0.0, 0.0, 1.0);
        let q = SVector::new(-1.0, -0.2);
        let mu = 1.0;
        let matrix_derivative = Matrix2::new(0.2, 0.0, 0.0, 0.1);
        let q_derivative = SVector::new(0.1, -0.1);
        let mu_derivative = 0.05;
        let objective_gradient = SVector::new(1.5, -0.5);
        let direct_gradient = 0.03;
        let solution = solve_planar_friction_cone(&matrix, &q, mu, 1e-12).unwrap();
        assert_eq!(solution.mode, PlanarContactMode::Sticking);

        let result = differentiate_planar_friction_mode(
            &matrix,
            &q,
            &solution,
            mu,
            &matrix_derivative,
            &q_derivative,
            mu_derivative,
            &objective_gradient,
            direct_gradient,
            1e-8,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();
        assert!((result.impulse_sensitivity - SVector::new(-0.1, 0.08)).norm() < 1e-12);

        let epsilon = 1e-6;
        let plus = solve_planar_friction_cone(
            &(matrix + matrix_derivative * epsilon),
            &(q + q_derivative * epsilon),
            mu + mu_derivative * epsilon,
            1e-12,
        )
        .unwrap();
        let minus = solve_planar_friction_cone(
            &(matrix - matrix_derivative * epsilon),
            &(q - q_derivative * epsilon),
            mu - mu_derivative * epsilon,
            1e-12,
        )
        .unwrap();
        let plus_objective =
            objective_gradient.dot(&plus.impulse) + direct_gradient * epsilon;
        let minus_objective =
            objective_gradient.dot(&minus.impulse) - direct_gradient * epsilon;
        let finite_difference =
            (plus_objective - minus_objective) / (2.0 * epsilon);
        assert!((finite_difference - result.objective_parameter_gradient).abs() < 1e-8);
    }

    #[test]
    fn planar_sliding_sensitivity_includes_friction_coefficient_derivative() {
        let matrix = Matrix2::new(2.0, 0.0, 0.0, 1.0);
        let q = SVector::new(-1.0, 1.0);
        let mu = 0.5;
        let matrix_derivative = Matrix2::new(0.2, 0.0, 0.0, 0.1);
        let q_derivative = SVector::new(0.1, -0.05);
        let mu_derivative = 0.05;
        let objective_gradient = SVector::new(1.0, 0.7);
        let direct_gradient = -0.02;
        let solution = solve_planar_friction_cone(&matrix, &q, mu, 1e-12).unwrap();
        assert_eq!(solution.mode, PlanarContactMode::SlidingPositive);

        let result = differentiate_planar_friction_mode(
            &matrix,
            &q,
            &solution,
            mu,
            &matrix_derivative,
            &q_derivative,
            mu_derivative,
            &objective_gradient,
            direct_gradient,
            1e-8,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();
        assert!((result.impulse_sensitivity - SVector::new(-0.1, 0.025)).norm() < 1e-12);

        let epsilon = 1e-6;
        let plus = solve_planar_friction_cone(
            &(matrix + matrix_derivative * epsilon),
            &(q + q_derivative * epsilon),
            mu + mu_derivative * epsilon,
            1e-12,
        )
        .unwrap();
        let minus = solve_planar_friction_cone(
            &(matrix - matrix_derivative * epsilon),
            &(q - q_derivative * epsilon),
            mu - mu_derivative * epsilon,
            1e-12,
        )
        .unwrap();
        let plus_objective =
            objective_gradient.dot(&plus.impulse) + direct_gradient * epsilon;
        let minus_objective =
            objective_gradient.dot(&minus.impulse) - direct_gradient * epsilon;
        let finite_difference =
            (plus_objective - minus_objective) / (2.0 * epsilon);
        assert!((finite_difference - result.objective_parameter_gradient).abs() < 1e-8);
    }

    #[test]
    fn planar_negative_sliding_sensitivity_uses_opposite_cone_edge() {
        let matrix = Matrix2::new(2.0, 0.0, 0.0, 1.0);
        let q = SVector::new(-1.0, -1.0);
        let mu = 0.5;
        let solution = solve_planar_friction_cone(&matrix, &q, mu, 1e-12).unwrap();
        assert_eq!(solution.mode, PlanarContactMode::SlidingNegative);
        let result = differentiate_planar_friction_mode(
            &matrix,
            &q,
            &solution,
            mu,
            &Matrix2::zeros(),
            &SVector::zeros(),
            0.1,
            &SVector::new(0.0, 1.0),
            0.0,
            1e-8,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();

        assert!((result.impulse_sensitivity - SVector::new(0.0, 0.05)).norm() < 1e-12);
    }

    #[test]
    fn planar_friction_sensitivity_rejects_stick_slip_boundary() {
        let matrix = Matrix2::new(2.0, 0.0, 0.0, 1.0);
        let q = SVector::new(-1.0, -0.25);
        let solution = solve_planar_friction_cone(&matrix, &q, 0.5, 1e-12).unwrap();
        assert_eq!(solution.mode, PlanarContactMode::Sticking);
        let error = differentiate_planar_friction_mode(
            &matrix,
            &q,
            &solution,
            0.5,
            &Matrix2::zeros(),
            &SVector::zeros(),
            0.0,
            &SVector::new(1.0, 0.0),
            0.0,
            1e-8,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap_err();
        assert_eq!(error, ContactAdjointError::PlanarModeBoundary);
    }

    #[test]
    fn planar_separating_sensitivity_is_zero_away_from_impact_boundary() {
        let matrix = Matrix2::identity();
        let q = SVector::new(0.1, 5.0);
        let solution = solve_planar_friction_cone(&matrix, &q, 0.5, 1e-12).unwrap();
        assert_eq!(solution.mode, PlanarContactMode::Separating);
        let result = differentiate_planar_friction_mode(
            &matrix,
            &q,
            &solution,
            0.5,
            &Matrix2::zeros(),
            &SVector::new(1.0, 1.0),
            0.1,
            &SVector::new(1.0, 1.0),
            0.25,
            1e-8,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();
        assert_eq!(result.impulse_sensitivity, SVector::zeros());
        assert_eq!(result.objective_parameter_gradient, 0.25);
    }

    #[test]
    fn implicit_adjoint_matches_closed_form_and_finite_difference() {
        // W(theta)=2+theta, q=-1, lambda=1/W. At theta=0,
        // d(lambda)/d(theta)=-1/4 and d(2*lambda)/d(theta)=-1/2.
        let matrix = DMatrix::from_element(1, 1, 2.0);
        let q = DVector::from_vec(vec![-1.0]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();
        let matrix_derivative = DMatrix::from_element(1, 1, 1.0);
        let q_derivative = DVector::zeros(1);
        let objective_gradient = DVector::from_vec(vec![2.0]);
        let result = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &matrix_derivative,
            &q_derivative,
            &objective_gradient,
            0.0,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();

        assert!((result.impulse_sensitivity[0] + 0.25).abs() < 1e-12);
        assert!((result.objective_parameter_gradient + 0.5).abs() < 1e-12);
        assert!((result.adjoint[0] - 1.0).abs() < 1e-12);

        let epsilon = 1e-5;
        let objective = |theta: f64| {
            let perturbed = DMatrix::from_element(1, 1, 2.0 + theta);
            let solved = solve_frictionless_lcp(&perturbed, &q, 1e-12).unwrap();
            2.0 * solved.impulses[0]
        };
        let finite_difference =
            (objective(epsilon) - objective(-epsilon)) / (2.0 * epsilon);
        assert!((finite_difference - result.objective_parameter_gradient).abs() < 1e-8);
    }

    #[test]
    fn implicit_adjoint_includes_bias_parameter_sensitivity() {
        // W=2, q(theta)=-1+theta, lambda=(1-theta)/2.
        let matrix = DMatrix::from_element(1, 1, 2.0);
        let q = DVector::from_vec(vec![-1.0]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();
        let result = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &DMatrix::zeros(1, 1),
            &DVector::from_vec(vec![1.0]),
            &DVector::from_vec(vec![3.0]),
            0.25,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();

        assert!((result.impulse_sensitivity[0] + 0.5).abs() < 1e-12);
        assert!((result.objective_parameter_gradient + 1.25).abs() < 1e-12);
    }

    #[test]
    fn implicit_adjoint_matches_finite_differences_for_randomized_spd_blocks() {
        for n in 1_usize..=4 {
            for seed in 1_u64..=6 {
                let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                let mut sample = || {
                    state ^= state << 13;
                    state ^= state >> 7;
                    state ^= state << 17;
                    ((*state >> 11) as f64 / ((1_u64 << 53) as f64)) * 2.0 - 1.0
                };
                let lower = DMatrix::<f64>::from_fn(n, n, |i, j| {
                    if j > i {
                        0.0
                    } else if i == j {
                        1.0 + 0.1 * i as f64
                    } else {
                        0.05 * sample()
                    }
                });
                let matrix = &lower * lower.transpose()
                    + DMatrix::identity(n, n) * 0.5;
                let raw_matrix_derivative =
                    DMatrix::<f64>::from_fn(n, n, |_, _| 0.01 * sample());
                let matrix_derivative =
                    (&raw_matrix_derivative + raw_matrix_derivative.transpose()) * 0.5;
                let q_derivative =
                    DVector::<f64>::from_fn(n, |_, _| 0.02 * sample());
                let target_impulses =
                    DVector::<f64>::from_fn(n, |i, _| 0.1 + 0.03 * i as f64);
                let q = -(&matrix * &target_impulses);
                let objective_gradient =
                    DVector::<f64>::from_fn(n, |_, _| sample());
                let direct_gradient = 0.07;
                let solution = solve_frictionless_lcp(&matrix, &q, 1e-11).unwrap();

                let result = differentiate_fixed_active_set_lcp(
                    &matrix,
                    &q,
                    &solution,
                    &matrix_derivative,
                    &q_derivative,
                    &objective_gradient,
                    direct_gradient,
                    1e-8,
                    1e-8,
                    1e-11,
                )
                .unwrap_or_else(|error| panic!("n={n}, seed={seed}: {error:?}"));

                let epsilon = 1e-6;
                let matrix_plus = &matrix + &matrix_derivative * epsilon;
                let matrix_minus = &matrix - &matrix_derivative * epsilon;
                let q_plus = &q + &q_derivative * epsilon;
                let q_minus = &q - &q_derivative * epsilon;
                let solution_plus =
                    solve_frictionless_lcp(&matrix_plus, &q_plus, 1e-11).unwrap();
                let solution_minus =
                    solve_frictionless_lcp(&matrix_minus, &q_minus, 1e-11).unwrap();

                let finite_difference_sensitivity =
                    (&solution_plus.impulses - &solution_minus.impulses) / (2.0 * epsilon);
                let sensitivity_scale = finite_difference_sensitivity
                    .norm()
                    .max(result.impulse_sensitivity.norm())
                    .max(1.0);
                assert!(
                    (&finite_difference_sensitivity - &result.impulse_sensitivity).norm()
                        <= 1e-6 * sensitivity_scale,
                    "n={n}, seed={seed}: sensitivity discrepancy"
                );

                let objective_plus =
                    objective_gradient.dot(&solution_plus.impulses) + direct_gradient * epsilon;
                let objective_minus =
                    objective_gradient.dot(&solution_minus.impulses) - direct_gradient * epsilon;
                let finite_difference_gradient =
                    (objective_plus - objective_minus) / (2.0 * epsilon);
                assert!(
                    (finite_difference_gradient - result.objective_parameter_gradient).abs()
                        <= 1e-6 * finite_difference_gradient.abs().max(1.0),
                    "n={n}, seed={seed}: adjoint discrepancy, finite_difference={finite_difference_gradient}, adjoint={}",
                    result.objective_parameter_gradient
                );
            }
        }
    }

    #[test]
    fn implicit_adjoint_returns_direct_gradient_for_strictly_inactive_contact() {
        let matrix = DMatrix::from_element(1, 1, 2.0);
        let q = DVector::from_vec(vec![1.0]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();
        let result = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &DMatrix::from_element(1, 1, 0.1),
            &DVector::from_vec(vec![0.2]),
            &DVector::from_vec(vec![1.0]),
            0.5,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap();

        assert!(result.active_rows.is_empty());
        assert_eq!(result.impulse_sensitivity[0], 0.0);
        assert_eq!(result.adjoint[0], 0.0);
        assert_eq!(result.objective_parameter_gradient, 0.5);
    }

    #[test]
    fn implicit_adjoint_rejects_non_monotone_contact_matrix() {
        let matrix = DMatrix::from_element(1, 1, -1.0);
        let q = DVector::from_vec(vec![1.0]);
        let solution = ReferenceLcpSolution {
            impulses: DVector::zeros(1),
            residual: DVector::from_vec(vec![1.0]),
            active_rows: Vec::new(),
            max_primal_violation: 0.0,
            max_complementarity_residual: 0.0,
        };
        let error = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &DMatrix::zeros(1, 1),
            &DVector::zeros(1),
            &DVector::zeros(1),
            0.0,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap_err();
        assert_eq!(error, ContactAdjointError::NonMonotoneMatrix);
    }

    #[test]
    fn implicit_adjoint_rejects_active_set_transition() {
        let matrix = DMatrix::from_element(1, 1, 2.0);
        let q = DVector::from_vec(vec![-1e-12]);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-14).unwrap();
        let error = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &DMatrix::zeros(1, 1),
            &DVector::zeros(1),
            &DVector::from_vec(vec![1.0]),
            0.0,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap_err();
        assert_eq!(error, ContactAdjointError::ActiveSetBoundary);
    }

    #[test]
    fn implicit_adjoint_rejects_rank_deficient_active_contacts() {
        let matrix = DMatrix::from_row_slice(2, 2, &[1.0, 1.0, 1.0, 1.0]);
        let q = DVector::from_vec(vec![-1.0, -1.0]);
        // Explicitly select the valid two-row active representation.
        // The reference oracle may also return an equivalent one-row solution
        // because redundant LCP impulses are non-unique.
        let solution = ReferenceLcpSolution {
            impulses: DVector::from_vec(vec![0.5, 0.5]),
            residual: DVector::zeros(2),
            active_rows: vec![0, 1],
            max_primal_violation: 0.0,
            max_complementarity_residual: 0.0,
        };
        let error = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &DMatrix::zeros(2, 2),
            &DVector::zeros(2),
            &DVector::from_vec(vec![1.0, 1.0]),
            0.0,
            1e-8,
            1e-8,
            1e-12,
        )
        .unwrap_err();
        assert_eq!(error, ContactAdjointError::IllConditionedActiveBlock);
    }
}
