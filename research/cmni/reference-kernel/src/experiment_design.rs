//! Research-only Fisher-information and local-identifiability primitives.
//!
//! Derivatives are with respect to caller-defined parameters. Scale parameters
//! to explicit dimensionless coordinates before interpreting condition numbers
//! or D-optimal scores across different parameterizations.

use crate::ModelError;

/// Local Fisher information for independent Poisson observations.
/// Jacobian rows are measured count channels; columns are caller-defined
/// parameters. Entries are d(expected_count_i)/d(parameter_j) at a design point.
pub fn poisson_fisher_information(
    expected_counts: &[f64],
    jacobian: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    if expected_counts.is_empty() || jacobian.len() != expected_counts.len() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let parameter_count = jacobian.first().map_or(0, Vec::len);
    if parameter_count == 0 || jacobian.iter().any(|row| row.len() != parameter_count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    for expected in expected_counts {
        if !expected.is_finite() {
            return Err(ModelError::NonFiniteValue("poisson_expected_count"));
        }
        if *expected <= 0.0 {
            return Err(ModelError::NonPositiveValue("poisson_expected_count"));
        }
    }
    for derivative in jacobian.iter().flatten() {
        if !derivative.is_finite() {
            return Err(ModelError::NonFiniteValue("observation_jacobian"));
        }
    }
    let mut information = vec![vec![0.0; parameter_count]; parameter_count];
    for (expected, row) in expected_counts.iter().zip(jacobian.iter()) {
        let inverse_mean = 1.0 / expected;
        for left in 0..parameter_count {
            for right in left..parameter_count {
                information[left][right] += row[left] * row[right] * inverse_mean;
            }
        }
    }
    for left in 0..parameter_count {
        for right in 0..left {
            information[left][right] = information[right][left];
        }
    }
    if information.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter("poisson_fisher_information_overflowed"));
    }
    Ok(information)
}

/// Add independent Fisher-information contributions from candidate runs.
pub fn combine_fisher_information(
    contributions: &[Vec<Vec<f64>>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let first = contributions.first().ok_or(ModelError::MismatchedDataLengths)?;
    let dimension = first.len();
    if dimension == 0 || first.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut combined = vec![vec![0.0; dimension]; dimension];
    for contribution in contributions {
        if contribution.len() != dimension
            || contribution.iter().any(|row| row.len() != dimension)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        for row in 0..dimension {
            for column in 0..dimension {
                let value = contribution[row][column];
                if !value.is_finite() {
                    return Err(ModelError::NonFiniteValue("fisher_information_matrix"));
                }
                combined[row][column] += value;
            }
        }
    }
    Ok(combined)
}

/// Rank and D-optimal diagnostics for a symmetric Fisher-information matrix.
#[derive(Debug, Clone, PartialEq)]
pub struct IdentifiabilitySummary {
    pub parameter_count: usize,
    pub effective_rank: usize,
    /// Eigenvalues in descending order, in caller-specified parameter scaling.
    pub eigenvalues: Vec<f64>,
    /// Present only when every parameter is locally identifiable at tolerance.
    pub log_determinant: Option<f64>,
    /// Present only for a full-rank matrix at tolerance.
    pub condition_number: Option<f64>,
}

/// Summarize local identifiability using a symmetric Jacobi eigensolver.
/// The relative eigenvalue tolerance is a numerical diagnostic, not a universal
/// scientific identifiability threshold.
pub fn identifiability_summary(
    information: &[Vec<f64>],
    relative_eigenvalue_tolerance: f64,
) -> Result<IdentifiabilitySummary, ModelError> {
    if information.is_empty()
        || information.iter().any(|row| row.len() != information.len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if !relative_eigenvalue_tolerance.is_finite()
        || relative_eigenvalue_tolerance <= 0.0
        || relative_eigenvalue_tolerance >= 1.0
    {
        return Err(ModelError::InvalidParameter(
            "relative_eigenvalue_tolerance_must_be_between_zero_and_one",
        ));
    }
    for value in information.iter().flatten() {
        if !value.is_finite() {
            return Err(ModelError::NonFiniteValue("fisher_information_matrix"));
        }
    }
    let dimension = information.len();
    let matrix_scale = information.iter().flatten().map(|value| value.abs()).fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * matrix_scale.max(f64::MIN_POSITIVE);
    for row in 0..dimension {
        for column in 0..row {
            if (information[row][column] - information[column][row]).abs() > symmetry_tolerance {
                return Err(ModelError::InvalidCovariance("fisher_information_must_be_symmetric"));
            }
        }
    }

    let eigenvalues = symmetric_eigenvalues(information)?;
    let maximum = eigenvalues.first().copied().unwrap_or(0.0).max(0.0);
    let negative_tolerance = 1.0e-9 * maximum.max(f64::MIN_POSITIVE);
    if eigenvalues.iter().any(|value| *value < -negative_tolerance) {
        return Err(ModelError::InvalidCovariance("fisher_information_must_be_positive_semidefinite"));
    }
    let threshold = maximum * relative_eigenvalue_tolerance;
    let effective_rank = eigenvalues.iter()
        .filter(|value| **value > threshold && **value > 0.0).count();
    let (log_determinant, condition_number) = if effective_rank == dimension {
        let minimum = *eigenvalues.last().ok_or(ModelError::MismatchedDataLengths)?;
        (Some(eigenvalues.iter().map(|value| value.ln()).sum()), Some(maximum / minimum))
    } else {
        (None, None)
    };
    Ok(IdentifiabilitySummary { parameter_count: dimension, effective_rank, eigenvalues, log_determinant, condition_number })
}

fn symmetric_eigenvalues(matrix: &[Vec<f64>]) -> Result<Vec<f64>, ModelError> {
    let dimension = matrix.len();
    let mut work = matrix.to_vec();
    let scale = matrix.iter().flatten().map(|value| value.abs()).fold(0.0_f64, f64::max);
    let convergence_tolerance = 1.0e-13 * scale.max(f64::MIN_POSITIVE);
    let maximum_iterations = 100_usize.saturating_mul(dimension).saturating_mul(dimension);
    let mut converged = dimension == 1;
    for _ in 0..maximum_iterations {
        let mut largest = 0.0_f64;
        let mut p = 0;
        let mut q = 0;
        for row in 0..dimension {
            for column in (row + 1)..dimension {
                if work[row][column].abs() > largest {
                    largest = work[row][column].abs();
                    p = row;
                    q = column;
                }
            }
        }
        if largest <= convergence_tolerance {
            converged = true;
            break;
        }
        let off_diagonal = work[p][q];
        let tau = (work[q][q] - work[p][p]) / (2.0 * off_diagonal);
        let tangent = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;
        let pp = work[p][p];
        let qq = work[q][q];
        work[p][p] = pp - tangent * off_diagonal;
        work[q][q] = qq + tangent * off_diagonal;
        work[p][q] = 0.0;
        work[q][p] = 0.0;
        for index in 0..dimension {
            if index != p && index != q {
                let ip = work[index][p];
                let iq = work[index][q];
                let new_ip = cosine * ip - sine * iq;
                let new_iq = sine * ip + cosine * iq;
                work[index][p] = new_ip;
                work[p][index] = new_ip;
                work[index][q] = new_iq;
                work[q][index] = new_iq;
            }
        }
    }
    if !converged {
        return Err(ModelError::InvalidParameter("fisher_eigensolver_did_not_converge"));
    }
    let mut eigenvalues: Vec<f64> = (0..dimension).map(|index| work[index][index]).collect();
    eigenvalues.sort_by(|left, right| right.total_cmp(left));
    Ok(eigenvalues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisson_fisher_information_matches_scalar_count_formula() {
        let information = poisson_fisher_information(&[20.0], &[vec![4.0, 2.0]]).expect("information");
        assert_eq!(information, vec![vec![0.8, 0.4], vec![0.4, 0.2]]);
    }

    #[test]
    fn independent_observables_produce_full_rank_information() {
        let information = poisson_fisher_information(&[10.0, 20.0], &[vec![10.0, 0.0], vec![0.0, 20.0]]).expect("information");
        let summary = identifiability_summary(&information, 1.0e-10).expect("summary");
        assert_eq!(summary.effective_rank, 2);
        assert!(summary.log_determinant.is_some());
        assert!(summary.condition_number.is_some());
    }

    #[test]
    fn collinear_signatures_are_non_identifiable() {
        let information = poisson_fisher_information(&[100.0, 200.0], &[vec![1.0, 2.0], vec![2.0, 4.0]]).expect("information");
        let summary = identifiability_summary(&information, 1.0e-9).expect("summary");
        assert_eq!(summary.effective_rank, 1);
        assert_eq!(summary.log_determinant, None);
        assert_eq!(summary.condition_number, None);
    }

    #[test]
    fn complementary_runs_can_identify_parameters_hidden_in_one_run() {
        let first = poisson_fisher_information(&[100.0], &[vec![1.0, 0.0]]).expect("first");
        let second = poisson_fisher_information(&[100.0], &[vec![0.0, 1.0]]).expect("second");
        let combined = combine_fisher_information(&[first, second]).expect("combined");
        let summary = identifiability_summary(&combined, 1.0e-10).expect("summary");
        assert_eq!(summary.effective_rank, 2);
    }

    #[test]
    fn poisson_weighting_uses_expected_count_in_denominator() {
        let low = poisson_fisher_information(&[10.0], &[vec![5.0]]).expect("low");
        let high = poisson_fisher_information(&[40.0], &[vec![5.0]]).expect("high");
        assert!((low[0][0] - 2.5).abs() < 1.0e-12);
        assert!((high[0][0] - 0.625).abs() < 1.0e-12);
    }

    #[test]
    fn information_combination_rejects_incompatible_dimensions() {
        assert_eq!(combine_fisher_information(&[vec![vec![1.0]], vec![vec![1.0, 0.0], vec![0.0, 1.0]]]), Err(ModelError::MismatchedDataLengths));
    }

    #[test]
    fn summary_rejects_non_symmetric_and_indefinite_matrices() {
        assert_eq!(identifiability_summary(&[vec![1.0, 0.5], vec![0.4, 1.0]], 1.0e-8), Err(ModelError::InvalidCovariance("fisher_information_must_be_symmetric")));
        assert_eq!(identifiability_summary(&[vec![1.0, 2.0], vec![2.0, 1.0]], 1.0e-8), Err(ModelError::InvalidCovariance("fisher_information_must_be_positive_semidefinite")));
    }

    #[test]
    fn scalar_eigenspectrum_has_unit_condition_number() {
        let summary = identifiability_summary(&[vec![7.0]], 1.0e-10).expect("summary");
        assert_eq!(summary.effective_rank, 1);
        assert!((summary.log_determinant.expect("logdet") - 7.0_f64.ln()).abs() < 1.0e-12);
        assert_eq!(summary.condition_number, Some(1.0));
    }
}
