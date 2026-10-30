//! Small-island unilateral contact reference solver.
//!
//! This module is a mathematical oracle for normal-contact experiments, not
//! the production contact solver. It accepts a Delassus matrix W and free
//! normal velocity q and solves:
//!
//!     lambda >= 0,  w = W * lambda + q >= 0,  lambda_i * w_i = 0.
//!
//! Active sets are exhaustively enumerated in ascending bit-mask order. The
//! implementation is intentionally limited to eight rows (at most 256 active
//! sets) so it can serve as a transparent reference for production solvers.
//! Tangential friction, restitution target construction, compliance assembly,
//! and generalized rigid-body Jacobians are deliberately outside this API.

use nalgebra::{DMatrix, DVector};

/// Errors from Delassus validation or the bounded reference LCP solve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContactLcpError {
    /// The matrix must be nonempty and square, with a matching velocity vector.
    DimensionMismatch,
    /// More than eight normal rows were supplied to the exhaustive reference.
    TooManyRows,
    /// A matrix, velocity, or tolerance contains an invalid number.
    NonFinite,
    /// The requested numerical tolerance is not positive.
    InvalidTolerance,
    /// The Delassus matrix is not symmetric within the requested tolerance.
    NotSymmetric,
    /// The Delassus matrix has a materially negative eigenvalue.
    NotPositiveSemidefinite,
    /// No enumerated active set satisfies the unilateral KKT conditions.
    NoFeasibleActiveSet,
}

/// Maximum KKT violations for a candidate unilateral contact solution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContactLcpKktResidual {
    /// Smallest component of the normal impulse vector.
    pub min_impulse: f64,
    /// Smallest component of the post-solve normal velocity/slack.
    pub min_slack: f64,
    /// Maximum absolute complementarity product |lambda_i * slack_i|.
    pub max_complementarity: f64,
}

/// Solution returned by exhaustive active-set enumeration.
#[derive(Clone, Debug)]
pub struct ContactLcpSolution {
    /// Nonnegative normal impulse magnitudes (up to the supplied tolerance).
    pub impulses: DVector<f64>,
    /// Post-solve normal velocity/slack: W * impulses + free_velocity.
    pub slack: DVector<f64>,
    /// Bit i is set when row i belongs to the enumerated active set.
    pub active_set_mask: u16,
    /// Number of active sets tested before finding the first feasible set.
    pub enumerated_active_sets: u16,
    /// Independently recomputed unilateral KKT residuals.
    pub kkt: ContactLcpKktResidual,
}

/// Validate the effective inverse-mass matrix before solving.
///
/// The check is scale-relative: symmetry and PSD tolerances are multiplied by
/// the largest absolute matrix entry. A small roundoff-sized negative
/// eigenvalue is tolerated, but the matrix is not silently projected.
pub fn validate_delassus(
    delassus: &DMatrix<f64>,
    relative_tolerance: f64,
) -> Result<(), ContactLcpError> {
    if !relative_tolerance.is_finite() {
        return Err(ContactLcpError::NonFinite);
    }
    if relative_tolerance <= 0.0 {
        return Err(ContactLcpError::InvalidTolerance);
    }
    if delassus.nrows() == 0 || delassus.nrows() != delassus.ncols() {
        return Err(ContactLcpError::DimensionMismatch);
    }
    if !delassus.iter().all(|value| value.is_finite()) {
        return Err(ContactLcpError::NonFinite);
    }

    let scale = delassus
        .iter()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
        .max(f64::MIN_POSITIVE);
    let symmetry_tolerance = relative_tolerance * scale;
    for row in 0..delassus.nrows() {
        for column in (row + 1)..delassus.ncols() {
            if (delassus[(row, column)] - delassus[(column, row)]).abs()
                > symmetry_tolerance
            {
                return Err(ContactLcpError::NotSymmetric);
            }
        }
    }

    let symmetric = (delassus.clone() + delassus.transpose()) * 0.5;
    let eigenvalues = nalgebra::linalg::SymmetricEigen::new(symmetric).eigenvalues;
    let minimum_eigenvalue = eigenvalues
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    if minimum_eigenvalue < -relative_tolerance * scale {
        return Err(ContactLcpError::NotPositiveSemidefinite);
    }

    Ok(())
}

/// Recompute unilateral contact KKT residuals from a candidate impulse vector.
///
/// A feasible solution requires lambda >= 0, W*lambda+q >= 0, and
/// lambda_i*(W*lambda+q)_i = 0. This function reports residuals rather than
/// deciding whether they pass a caller's tolerance.
pub fn contact_lcp_kkt_residual(
    delassus: &DMatrix<f64>,
    free_velocity: &DVector<f64>,
    impulses: &DVector<f64>,
) -> Result<ContactLcpKktResidual, ContactLcpError> {
    if delassus.nrows() == 0
        || delassus.nrows() != delassus.ncols()
        || free_velocity.len() != delassus.nrows()
        || impulses.len() != delassus.nrows()
    {
        return Err(ContactLcpError::DimensionMismatch);
    }
    if !delassus.iter().all(|value| value.is_finite())
        || !free_velocity.iter().all(|value| value.is_finite())
        || !impulses.iter().all(|value| value.is_finite())
    {
        return Err(ContactLcpError::NonFinite);
    }

    let slack = delassus * impulses + free_velocity;
    let min_impulse = impulses
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    let min_slack = slack
        .iter()
        .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
    let max_complementarity = impulses
        .iter()
        .zip(slack.iter())
        .fold(0.0_f64, |maximum, (lambda, velocity)| {
            maximum.max((lambda * velocity).abs())
        });

    Ok(ContactLcpKktResidual {
        min_impulse,
        min_slack,
        max_complementarity,
    })
}

/// Solve a small unilateral normal-contact LCP by exhaustive active sets.
///
/// free_velocity is the normal relative velocity before this island's
/// impulses, including any declared bias/target term. Therefore a closing
/// contact generally has a negative component. For a candidate active set A,
/// this solves W_AA * lambda_A = -q_A and sets all inactive impulses to zero.
///
/// The first candidate whose impulses, slack, and complementarity products
/// satisfy the relative tolerance is returned. The ordering is deterministic
/// but does not promise a minimum-norm solution when the LCP is non-unique.
/// Singular active-set linear systems are skipped; another active set may
/// still provide a valid solution (including for redundant PSD rows).
pub fn solve_normal_contact_lcp_reference(
    delassus: &DMatrix<f64>,
    free_velocity: &DVector<f64>,
    relative_tolerance: f64,
) -> Result<ContactLcpSolution, ContactLcpError> {
    validate_delassus(delassus, relative_tolerance)?;

    let row_count = delassus.nrows();
    if free_velocity.len() != row_count {
        return Err(ContactLcpError::DimensionMismatch);
    }
    if row_count > 8 {
        return Err(ContactLcpError::TooManyRows);
    }
    if !free_velocity.iter().all(|value| value.is_finite()) {
        return Err(ContactLcpError::NonFinite);
    }

    let active_set_count = 1_usize << row_count;
    for mask in 0..active_set_count {
        let active_rows: Vec<usize> = (0..row_count)
            .filter(|row| (mask & (1_usize << row)) != 0)
            .collect();
        let mut impulses = DVector::zeros(row_count);

        if !active_rows.is_empty() {
            let active_count = active_rows.len();
            let mut active_delassus = DMatrix::zeros(active_count, active_count);
            let mut rhs = DVector::zeros(active_count);
            for (active_row, &global_row) in active_rows.iter().enumerate() {
                rhs[active_row] = -free_velocity[global_row];
                for (active_column, &global_column) in active_rows.iter().enumerate() {
                    active_delassus[(active_row, active_column)] =
                        delassus[(global_row, global_column)];
                }
            }

            let Some(active_impulses) = active_delassus.lu().solve(&rhs) else {
                continue;
            };
            for (active_index, &global_row) in active_rows.iter().enumerate() {
                impulses[global_row] = active_impulses[active_index];
            }
        }

        let kkt = contact_lcp_kkt_residual(delassus, free_velocity, &impulses)?;
        let impulse_scale = impulses
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
            .max(f64::MIN_POSITIVE);
        let slack = delassus * &impulses + free_velocity;
        let slack_scale = slack
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()))
            .max(f64::MIN_POSITIVE);

        if kkt.min_impulse >= -relative_tolerance * impulse_scale
            && kkt.min_slack >= -relative_tolerance * slack_scale
            && kkt.max_complementarity
                <= relative_tolerance * impulse_scale * slack_scale
        {
            return Ok(ContactLcpSolution {
                impulses,
                slack,
                active_set_mask: mask as u16,
                enumerated_active_sets: (mask + 1) as u16,
                kkt,
            });
        }
    }

    Err(ContactLcpError::NoFeasibleActiveSet)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_row_contact_matches_analytical_impulse() {
        let delassus = DMatrix::from_element(1, 1, 0.5);
        let free_velocity = DVector::from_element(1, -2.0);
        let solution =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap();

        assert!((solution.impulses[0] - 4.0).abs() < 1e-12);
        assert!(solution.slack[0].abs() < 1e-12);
        assert!(solution.kkt.min_impulse >= -1e-12);
        assert!(solution.kkt.min_slack >= -1e-12);
        assert!(solution.kkt.max_complementarity <= 1e-12);
    }

    #[test]
    fn coupled_two_row_lcp_matches_analytical_solution() {
        let delassus = DMatrix::from_row_slice(2, 2, &[2.0, 1.0, 1.0, 2.0]);
        let free_velocity = DVector::from_row_slice(&[-1.0, -1.0]);
        let solution =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap();

        assert!((solution.impulses[0] - 1.0 / 3.0).abs() < 1e-12);
        assert!((solution.impulses[1] - 1.0 / 3.0).abs() < 1e-12);
        assert!(solution.slack.norm() < 1e-12);
        assert_eq!(solution.active_set_mask, 0b11);
    }

    #[test]
    fn separating_rows_receive_no_impulse() {
        let delassus = DMatrix::from_row_slice(2, 2, &[1.0, 0.25, 0.25, 1.0]);
        let free_velocity = DVector::from_row_slice(&[0.5, 1.0]);
        let solution =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap();

        assert!(solution.impulses.norm() == 0.0);
        assert_eq!(solution.active_set_mask, 0);
        assert_eq!(solution.slack, free_velocity);
    }

    #[test]
    fn redundant_psd_rows_can_use_a_nonsingular_active_subset() {
        let delassus = DMatrix::from_row_slice(2, 2, &[1.0, 1.0, 1.0, 1.0]);
        let free_velocity = DVector::from_row_slice(&[-1.0, -1.0]);
        let solution =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap();

        let residual = contact_lcp_kkt_residual(&delassus, &free_velocity, &solution.impulses)
            .unwrap();
        assert!(residual.min_impulse >= -1e-12);
        assert!(residual.min_slack >= -1e-12);
        assert!(residual.max_complementarity <= 1e-12);
    }

    #[test]
    fn rejects_asymmetric_delassus_matrix() {
        let delassus = DMatrix::from_row_slice(2, 2, &[1.0, 0.2, 0.3, 1.0]);
        assert_eq!(
            validate_delassus(&delassus, 1e-12),
            Err(ContactLcpError::NotSymmetric)
        );
    }

    #[test]
    fn rejects_indefinite_delassus_matrix() {
        let delassus = DMatrix::from_row_slice(2, 2, &[1.0, 2.0, 2.0, 1.0]);
        assert_eq!(
            validate_delassus(&delassus, 1e-12),
            Err(ContactLcpError::NotPositiveSemidefinite)
        );
    }

    #[test]
    fn rejects_more_than_eight_rows() {
        let delassus = DMatrix::identity(9, 9);
        let free_velocity = DVector::from_element(9, -1.0);
        let error =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap_err();
        assert_eq!(error, ContactLcpError::TooManyRows);
    }

    #[test]
    fn eight_row_reference_enumerates_all_active_rows_within_bound() {
        let delassus = DMatrix::identity(8, 8);
        let free_velocity = DVector::from_element(8, -1.0);
        let solution =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap();

        assert_eq!(solution.active_set_mask, 0xff);
        assert_eq!(solution.enumerated_active_sets, 256);
        assert!((solution.impulses - DVector::from_element(8, 1.0)).norm() < 1e-12);
        assert!(solution.slack.norm() < 1e-12);
    }

    #[test]
    fn reports_infeasible_zero_effective_mass_row() {
        let delassus = DMatrix::zeros(1, 1);
        let free_velocity = DVector::from_element(1, -1.0);
        let error =
            solve_normal_contact_lcp_reference(&delassus, &free_velocity, 1e-12).unwrap_err();
        assert_eq!(error, ContactLcpError::NoFeasibleActiveSet);
    }
}
