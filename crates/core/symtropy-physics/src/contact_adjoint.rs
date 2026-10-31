// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Implicit sensitivities of a converged, fixed-active-set frictionless LCP.
//!
//! This is a research primitive, not a differentiable simulator. It computes
//! derivatives through the active contact equations after a reference LCP
//! solve. It intentionally rejects active-set transitions and ill-conditioned
//! active blocks instead of returning misleading gradients.

use nalgebra::{DMatrix, DVector};

use crate::delassus::ReferenceLcpSolution;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::delassus::solve_frictionless_lcp;

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
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-12).unwrap();
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
