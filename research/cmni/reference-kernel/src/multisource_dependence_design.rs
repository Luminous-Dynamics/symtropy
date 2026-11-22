//! Minimax design ranking over finite positive-semidefinite multi-source
//! correlation scenarios.
//!
//! Unlike independent pairwise correlation intervals, each scenario is a
//! complete correlation matrix. This preserves the joint PSD constraint. The
//! source covariance is D_sigma R D_sigma, the observation covariance is
//! R_noise + J D_sigma R D_sigma J^T, and scalar-parameter information is
//! h^T R_observation^-1 h.
//!
//! The ranking is exact over the finite supplied scenario set up to f64
//! arithmetic. It is not a continuous elliptope optimizer, a formal interval
//! certificate, or an authenticated physical dependence model.

use crate::ModelError;

/// One feasible joint correlation matrix for the complete shared-source set.
#[derive(Debug, Clone, PartialEq)]
pub struct FiniteCorrelationScenario {
    pub scenario_id: String,
    /// Symmetric m x m correlation matrix, with unit diagonal and PSD.
    pub correlation_matrix: Vec<Vec<f64>>,
}

/// Candidate design for a scalar-parameter, multiple-observation model.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiSourceDependenceDesignCandidate {
    pub candidate_id: String,
    pub prior_parameter_variance: f64,
    /// Observation sensitivities h in y = h theta + J u + epsilon.
    pub observation_sensitivities: Vec<f64>,
    /// Diagonal independent observation noise variances; all strictly positive.
    pub independent_observation_noise_variances: Vec<f64>,
    /// Marginal standard deviations for the m shared sources.
    pub source_standard_deviations: Vec<f64>,
    /// n-observation by m-source Jacobian.
    pub source_jacobians: Vec<Vec<f64>>,
}

/// Per-candidate worst/best EIG across the finite PSD scenario set.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiSourceDependenceDesignScore {
    pub candidate_id: String,
    pub nominal_independence_eig_nats: Option<f64>,
    pub worst_case_eig_nats: f64,
    pub best_case_eig_nats: f64,
    pub worst_case_posterior_parameter_variance: f64,
    pub best_case_posterior_parameter_variance: f64,
    pub worst_case_scenario_id: String,
    pub best_case_scenario_id: String,
    pub robust_rank: usize,
    pub nominal_independence_rank: Option<usize>,
}

/// Ranking over a common finite set of PSD-feasible source-correlation matrices.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiSourceDependenceDesignRanking {
    /// Receipt reference covering the joint source definitions and scenario set.
    /// This identifier is not authenticated by the numerical kernel.
    pub dependence_scope_receipt_id: String,
    pub source_count: usize,
    pub observation_count: usize,
    pub scenario_ids: Vec<String>,
    pub nominal_independence_preferred_candidate_id: Option<String>,
    pub robust_preferred_candidate_id: String,
    pub ranking_reversed: Option<bool>,
    /// Sorted by robust rank, descending worst-case EIG.
    pub scores: Vec<MultiSourceDependenceDesignScore>,
    /// Exact over the finite scenario set up to floating-point arithmetic only.
    pub finite_scenario_exact: bool,
    /// Always false until outward-rounded arithmetic is implemented.
    pub formal_interval_arithmetic: bool,
}

/// Rank candidate designs by worst-case EIG over complete PSD-feasible
/// multi-source correlation scenarios.
///
/// All candidates must share the same prior variance and source marginal scales.
/// Candidate-specific observation sensitivities, noise variances and source
/// Jacobians may differ. Every scenario matrix must be symmetric, have unit
/// diagonal, and be positive-semidefinite (PSD). Singular PSD matrices are
/// allowed; strictly positive independent observation noise keeps the
/// observation covariance positive-definite.
///
/// The nominal rho=0 baseline is reported only when the identity matrix is
/// explicitly present in the supplied scenario set. No interpolation or
/// extrapolation beyond that finite set is performed.
pub fn rank_designs_under_finite_correlation_scenarios(
    candidates: &[MultiSourceDependenceDesignCandidate],
    scenarios: &[FiniteCorrelationScenario],
    dependence_scope_receipt_id: &str,
) -> Result<MultiSourceDependenceDesignRanking, ModelError> {
    if candidates.is_empty() || scenarios.is_empty() {
        return Err(ModelError::InvalidParameter(
            "multi_source_design_candidates_and_scenarios_must_be_nonempty",
        ));
    }
    if dependence_scope_receipt_id.trim().is_empty() {
        return Err(ModelError::InvalidParameter(
            "multi_source_dependence_scope_receipt_must_be_nonempty",
        ));
    }

    let source_count = candidates[0].source_standard_deviations.len();
    let observation_count = candidates[0].observation_sensitivities.len();
    if source_count == 0 || observation_count == 0 {
        return Err(ModelError::MismatchedDataLengths);
    }

    let mut scenario_ids = std::collections::BTreeSet::new();
    let mut validated_scenarios = Vec::with_capacity(scenarios.len());
    let mut identity_scenario_id = None;
    for scenario in scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "multi_source_scenario_ids_must_be_nonempty_and_unique",
            ));
        }
        let correlation = canonicalize_and_validate_correlation_matrix(
            &scenario.correlation_matrix,
            source_count,
        )?;
        if is_identity_correlation_matrix(&correlation) {
            identity_scenario_id = Some(scenario.scenario_id.as_str());
        }
        validated_scenarios.push((scenario.scenario_id.clone(), correlation));
    }

    let reference = &candidates[0];
    validate_candidate(reference, observation_count, source_count)?;
    let mut candidate_ids = std::collections::BTreeSet::new();
    let mut scores = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        if candidate.candidate_id.trim().is_empty()
            || !candidate_ids.insert(candidate.candidate_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "multi_source_candidate_ids_must_be_nonempty_and_unique",
            ));
        }
        validate_candidate(candidate, observation_count, source_count)?;
        validate_common_candidate_scope(reference, candidate)?;

        let mut worst_eig = f64::INFINITY;
        let mut best_eig = f64::NEG_INFINITY;
        let mut worst_variance = f64::NEG_INFINITY;
        let mut best_variance = f64::INFINITY;
        let mut worst_scenario_id = validated_scenarios[0].0.clone();
        let mut best_scenario_id = validated_scenarios[0].0.clone();
        let mut nominal_independence_eig_nats = None;

        for (scenario_id, correlation) in &validated_scenarios {
            let information = scalar_parameter_information(candidate, correlation)?;
            let posterior_variance = 1.0 / (1.0 / candidate.prior_parameter_variance + information);
            let eig = 0.5 * (1.0 + candidate.prior_parameter_variance * information).ln();
            if !posterior_variance.is_finite() || !eig.is_finite() {
                return Err(ModelError::InvalidParameter(
                    "multi_source_design_utility_nonfinite",
                ));
            }

            if eig < worst_eig {
                worst_eig = eig;
                worst_variance = posterior_variance;
                worst_scenario_id = scenario_id.clone();
            }
            if eig > best_eig {
                best_eig = eig;
                best_variance = posterior_variance;
                best_scenario_id = scenario_id.clone();
            }
            if Some(scenario_id.as_str()) == identity_scenario_id {
                nominal_independence_eig_nats = Some(eig);
            }
        }

        scores.push(MultiSourceDependenceDesignScore {
            candidate_id: candidate.candidate_id.clone(),
            nominal_independence_eig_nats,
            worst_case_eig_nats: worst_eig,
            best_case_eig_nats: best_eig,
            worst_case_posterior_parameter_variance: worst_variance,
            best_case_posterior_parameter_variance: best_variance,
            worst_case_scenario_id: worst_scenario_id,
            best_case_scenario_id: best_scenario_id,
            robust_rank: 0,
            nominal_independence_rank: None,
        });
    }

    let mut robust_order: Vec<usize> = (0..scores.len()).collect();
    robust_order.sort_by(|left, right| {
        scores[*right]
            .worst_case_eig_nats
            .total_cmp(&scores[*left].worst_case_eig_nats)
            .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
    });
    for (rank, index) in robust_order.iter().enumerate() {
        scores[*index].robust_rank = rank + 1;
    }

    let mut nominal_independence_preferred_candidate_id = None;
    let mut ranking_reversed = None;
    if identity_scenario_id.is_some() {
        let mut nominal_order: Vec<usize> = (0..scores.len()).collect();
        nominal_order.sort_by(|left, right| {
            scores[*right]
                .nominal_independence_eig_nats
                .expect("identity-scenario EIG is present")
                .total_cmp(
                    &scores[*left]
                        .nominal_independence_eig_nats
                        .expect("identity-scenario EIG is present"),
                )
                .then_with(|| scores[*left].candidate_id.cmp(&scores[*right].candidate_id))
        });
        for (rank, index) in nominal_order.iter().enumerate() {
            scores[*index].nominal_independence_rank = Some(rank + 1);
        }
        nominal_independence_preferred_candidate_id =
            Some(scores[nominal_order[0]].candidate_id.clone());

        let mut reversed = false;
        for left in 0..scores.len() {
            for right in (left + 1)..scores.len() {
                if scores[left].robust_rank.cmp(&scores[right].robust_rank)
                    != scores[left]
                        .nominal_independence_rank
                        .cmp(&scores[right].nominal_independence_rank)
                {
                    reversed = true;
                }
            }
        }
        ranking_reversed = Some(reversed);
    }

    scores.sort_by(|left, right| left.robust_rank.cmp(&right.robust_rank));
    let robust_preferred_candidate_id = scores[0].candidate_id.clone();

    Ok(MultiSourceDependenceDesignRanking {
        dependence_scope_receipt_id: dependence_scope_receipt_id.to_owned(),
        source_count,
        observation_count,
        scenario_ids: scenarios
            .iter()
            .map(|scenario| scenario.scenario_id.clone())
            .collect(),
        nominal_independence_preferred_candidate_id,
        robust_preferred_candidate_id,
        ranking_reversed,
        scores,
        finite_scenario_exact: true,
        formal_interval_arithmetic: false,
    })
}

pub(crate) fn validate_candidate(
    candidate: &MultiSourceDependenceDesignCandidate,
    observation_count: usize,
    source_count: usize,
) -> Result<(), ModelError> {
    if !candidate.prior_parameter_variance.is_finite() || candidate.prior_parameter_variance <= 0.0
    {
        return Err(ModelError::InvalidParameter(
            "multi_source_prior_parameter_variance_must_be_positive",
        ));
    }
    if candidate.observation_sensitivities.len() != observation_count
        || candidate.independent_observation_noise_variances.len() != observation_count
        || candidate.source_jacobians.len() != observation_count
        || candidate.source_standard_deviations.len() != source_count
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if candidate
        .observation_sensitivities
        .iter()
        .any(|value| !value.is_finite())
        || candidate
            .independent_observation_noise_variances
            .iter()
            .any(|value| !value.is_finite())
        || candidate
            .source_standard_deviations
            .iter()
            .any(|value| !value.is_finite())
        || candidate
            .source_jacobians
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("multi_source_design_input"));
    }
    if candidate
        .independent_observation_noise_variances
        .iter()
        .any(|value| *value <= 0.0)
        || candidate
            .source_standard_deviations
            .iter()
            .any(|value| *value < 0.0)
        || candidate
            .source_jacobians
            .iter()
            .any(|row| row.len() != source_count)
    {
        return Err(ModelError::InvalidParameter(
            "multi_source_design_noise_scales_or_jacobians_invalid",
        ));
    }
    Ok(())
}

fn validate_common_candidate_scope(
    reference: &MultiSourceDependenceDesignCandidate,
    candidate: &MultiSourceDependenceDesignCandidate,
) -> Result<(), ModelError> {
    if !nearly_equal(
        reference.prior_parameter_variance,
        candidate.prior_parameter_variance,
    ) || reference
        .source_standard_deviations
        .iter()
        .zip(candidate.source_standard_deviations.iter())
        .any(|(left, right)| !nearly_equal(*left, *right))
    {
        return Err(ModelError::InvalidParameter(
            "multi_source_candidates_must_share_prior_and_source_marginals",
        ));
    }
    Ok(())
}

fn nearly_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-12 * left.abs().max(right.abs()).max(1.0)
}

/// Validate a correlation matrix and return a canonical symmetric matrix.
///
/// Inputs within 1e-12 of unit diagonal and symmetry are canonicalized to exact
/// unit diagonal and averaged symmetric off-diagonal entries before use. The
/// minimum eigenvalue is checked with a Jacobi iteration; values down to -1e-10
/// are accepted as floating-point PSD tolerance. This is not a formal PSD proof.
pub(crate) fn canonicalize_and_validate_correlation_matrix(
    matrix: &[Vec<f64>],
    source_count: usize,
) -> Result<Vec<Vec<f64>>, ModelError> {
    if matrix.len() != source_count || matrix.iter().any(|row| row.len() != source_count) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("source_correlation_matrix"));
    }

    let mut canonical = vec![vec![0.0; source_count]; source_count];
    for row in 0..source_count {
        if !nearly_equal(matrix[row][row], 1.0) {
            return Err(ModelError::InvalidCovariance(
                "source_correlation_matrix_diagonal_must_be_one",
            ));
        }
        canonical[row][row] = 1.0;
        for column in (row + 1)..source_count {
            let upper = matrix[row][column];
            let lower = matrix[column][row];
            if !nearly_equal(upper, lower)
                || upper.abs() > 1.0 + 1.0e-12
                || lower.abs() > 1.0 + 1.0e-12
            {
                return Err(ModelError::InvalidCovariance(
                    "source_correlation_matrix_must_be_symmetric_and_bounded",
                ));
            }
            let symmetric_value = 0.5 * (upper + lower);
            canonical[row][column] = symmetric_value;
            canonical[column][row] = symmetric_value;
        }
    }

    let minimum_eigenvalue = minimum_symmetric_eigenvalue(&canonical)?;
    if minimum_eigenvalue < -1.0e-10 {
        return Err(ModelError::InvalidCovariance(
            "source_correlation_matrix_must_be_positive_semidefinite",
        ));
    }
    Ok(canonical)
}

/// Symmetric Jacobi eigenvalue iteration, used only for small correlation
/// matrices. A residual check prevents accepting a partially diagonalized
/// matrix as PSD when the iteration budget is exhausted.
#[allow(clippy::needless_range_loop)]
fn minimum_symmetric_eigenvalue(matrix: &[Vec<f64>]) -> Result<f64, ModelError> {
    let dimension = matrix.len();
    if dimension == 0 {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut values = matrix.to_vec();
    let iterations = 100usize.saturating_mul(dimension).saturating_mul(dimension);
    const JACOBI_OFF_DIAGONAL_TOLERANCE: f64 = 1.0e-14;
    let mut converged = false;

    for _ in 0..iterations {
        let mut largest = 0.0;
        let mut pivot = (0usize, 0usize);
        for row in 0..dimension {
            for column in (row + 1)..dimension {
                let magnitude = values[row][column].abs();
                if magnitude > largest {
                    largest = magnitude;
                    pivot = (row, column);
                }
            }
        }
        if largest <= JACOBI_OFF_DIAGONAL_TOLERANCE {
            converged = true;
            break;
        }

        let (p, q) = pivot;
        let off_diagonal = values[p][q];
        let tau = (values[q][q] - values[p][p]) / (2.0 * off_diagonal);
        let tangent = if tau >= 0.0 {
            1.0 / (tau + (1.0 + tau * tau).sqrt())
        } else {
            -1.0 / (-tau + (1.0 + tau * tau).sqrt())
        };
        let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
        let sine = tangent * cosine;
        let pp = values[p][p];
        let qq = values[q][q];

        values[p][p] = pp - tangent * off_diagonal;
        values[q][q] = qq + tangent * off_diagonal;
        values[p][q] = 0.0;
        values[q][p] = 0.0;

        for index in 0..dimension {
            if index == p || index == q {
                continue;
            }
            let ip = values[index][p];
            let iq = values[index][q];
            values[index][p] = cosine * ip - sine * iq;
            values[p][index] = values[index][p];
            values[index][q] = sine * ip + cosine * iq;
            values[q][index] = values[index][q];
        }
    }

    if !converged {
        let mut residual = 0.0_f64;
        for row in 0..dimension {
            for column in (row + 1)..dimension {
                residual = residual.max(values[row][column].abs());
            }
        }
        if residual > JACOBI_OFF_DIAGONAL_TOLERANCE {
            return Err(ModelError::InvalidCovariance(
                "source_correlation_matrix_eigen_iteration_did_not_converge",
            ));
        }
    }

    let minimum = (0..dimension)
        .map(|index| values[index][index])
        .fold(f64::INFINITY, f64::min);
    if !minimum.is_finite() {
        return Err(ModelError::InvalidCovariance(
            "source_correlation_matrix_eigenvalue_nonfinite",
        ));
    }
    Ok(minimum)
}

fn is_identity_correlation_matrix(matrix: &[Vec<f64>]) -> bool {
    matrix.iter().enumerate().all(|(row, values)| {
        values
            .iter()
            .enumerate()
            .all(|(column, value)| nearly_equal(*value, if row == column { 1.0 } else { 0.0 }))
    })
}

pub(crate) fn scalar_parameter_information(
    candidate: &MultiSourceDependenceDesignCandidate,
    correlation: &[Vec<f64>],
) -> Result<f64, ModelError> {
    scalar_parameter_information_and_source_gradient(candidate, correlation)
        .map(|(information, _)| information)
}

/// Return (h^T V^{-1} h) and its source-coordinate vector
/// (b = D_\sigma J^T V^{-1} h), used by convex tangent lower bounds.
pub(crate) fn scalar_parameter_information_and_source_gradient(
    candidate: &MultiSourceDependenceDesignCandidate,
    correlation: &[Vec<f64>],
) -> Result<(f64, Vec<f64>), ModelError> {
    let observation_count = candidate.observation_sensitivities.len();
    let source_count = candidate.source_standard_deviations.len();
    if correlation.len() != source_count
        || correlation.iter().any(|row| row.len() != source_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut covariance = vec![vec![0.0; observation_count]; observation_count];

    for row in 0..observation_count {
        for column in 0..observation_count {
            covariance[row][column] = if row == column {
                candidate.independent_observation_noise_variances[row]
            } else {
                0.0
            };
            for source_row in 0..source_count {
                for source_column in 0..source_count {
                    let source_covariance = candidate.source_standard_deviations[source_row]
                        * correlation[source_row][source_column]
                        * candidate.source_standard_deviations[source_column];
                    covariance[row][column] += candidate.source_jacobians[row][source_row]
                        * source_covariance
                        * candidate.source_jacobians[column][source_column];
                }
            }
        }
    }

    let lower = cholesky_decomposition(&covariance)?;
    let solved = cholesky_solve(&lower, &candidate.observation_sensitivities)?;
    let information = candidate
        .observation_sensitivities
        .iter()
        .zip(solved.iter())
        .map(|(sensitivity, solution)| sensitivity * solution)
        .sum::<f64>();
    if !information.is_finite() || information < -1.0e-10 {
        return Err(ModelError::InvalidParameter(
            "multi_source_parameter_information_invalid",
        ));
    }

    let source_gradient_coordinates: Vec<f64> = (0..source_count)
        .map(|source_index| {
            candidate.source_standard_deviations[source_index]
                * (0..observation_count)
                    .map(|observation_index| {
                        candidate.source_jacobians[observation_index][source_index]
                            * solved[observation_index]
                    })
                    .sum::<f64>()
        })
        .collect();
    if source_gradient_coordinates
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue(
            "multi_source_information_source_gradient",
        ));
    }
    Ok((information.max(0.0), source_gradient_coordinates))
}

#[allow(clippy::needless_range_loop)]
fn cholesky_decomposition(matrix: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let correction = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum::<f64>();
            let residual = matrix[row][column] - correction;
            if row == column {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(ModelError::InvalidCovariance(
                        "multi_source_observation_covariance_not_positive_definite",
                    ));
                }
                lower[row][column] = residual.sqrt();
            } else {
                lower[row][column] = residual / lower[column][column];
            }
        }
    }
    Ok(lower)
}

#[allow(clippy::needless_range_loop)]
fn cholesky_solve(lower: &[Vec<f64>], right_hand_side: &[f64]) -> Result<Vec<f64>, ModelError> {
    let dimension = lower.len();
    if right_hand_side.len() != dimension || lower.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut intermediate = vec![0.0; dimension];
    for row in 0..dimension {
        let correction = (0..row)
            .map(|column| lower[row][column] * intermediate[column])
            .sum::<f64>();
        intermediate[row] = (right_hand_side[row] - correction) / lower[row][row];
    }

    let mut solution = vec![0.0; dimension];
    for row in (0..dimension).rev() {
        let correction = (row + 1..dimension)
            .map(|column| lower[column][row] * solution[column])
            .sum::<f64>();
        solution[row] = (intermediate[row] - correction) / lower[row][row];
    }
    if solution.iter().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("multi_source_cholesky_solution"));
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_source_scenarios() -> Vec<FiniteCorrelationScenario> {
        vec![
            FiniteCorrelationScenario {
                scenario_id: "independent".to_owned(),
                correlation_matrix: vec![
                    vec![1.0, 0.0, 0.0],
                    vec![0.0, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0],
                ],
            },
            FiniteCorrelationScenario {
                scenario_id: "negative-source-correlation".to_owned(),
                correlation_matrix: vec![
                    vec![1.0, -0.75, 0.0],
                    vec![-0.75, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0],
                ],
            },
            FiniteCorrelationScenario {
                scenario_id: "positive-source-correlation".to_owned(),
                correlation_matrix: vec![
                    vec![1.0, 0.75, 0.0],
                    vec![0.75, 1.0, 0.0],
                    vec![0.0, 0.0, 1.0],
                ],
            },
        ]
    }

    fn candidates() -> Vec<MultiSourceDependenceDesignCandidate> {
        vec![
            MultiSourceDependenceDesignCandidate {
                candidate_id: "nominal-winner".to_owned(),
                prior_parameter_variance: 1.0,
                observation_sensitivities: vec![-2.0, -2.0],
                independent_observation_noise_variances: vec![1.0, 1.0],
                source_standard_deviations: vec![1.0, 1.0, 1.0],
                source_jacobians: vec![vec![-1.0, 1.0, 0.0], vec![0.0, 2.0, 0.0]],
            },
            MultiSourceDependenceDesignCandidate {
                candidate_id: "robust-winner".to_owned(),
                prior_parameter_variance: 1.0,
                observation_sensitivities: vec![2.0, 0.5],
                independent_observation_noise_variances: vec![1.0, 1.0],
                source_standard_deviations: vec![1.0, 1.0, 1.0],
                source_jacobians: vec![vec![-2.0, 0.0, 0.0], vec![0.0, 0.0, 0.0]],
            },
        ]
    }

    const RECEIPT: &str = "receipt:three-source-psd-dependence-set-v1";

    #[test]
    fn three_source_psd_scenarios_preserve_nominal_to_minimax_rank_reversal() {
        let result = rank_designs_under_finite_correlation_scenarios(
            &candidates(),
            &three_source_scenarios(),
            RECEIPT,
        )
        .expect("three-source robust ranking");

        assert_eq!(
            result
                .nominal_independence_preferred_candidate_id
                .as_deref(),
            Some("nominal-winner")
        );
        assert_eq!(result.robust_preferred_candidate_id, "robust-winner");
        assert_eq!(result.ranking_reversed, Some(true));
        assert_eq!(result.source_count, 3);
        assert_eq!(result.observation_count, 2);
        assert_eq!(result.scenario_ids.len(), 3);
        assert_eq!(result.scores[0].candidate_id, "robust-winner");

        let nominal = result
            .scores
            .iter()
            .find(|score| score.candidate_id == "nominal-winner")
            .expect("nominal score");
        let robust = result
            .scores
            .iter()
            .find(|score| score.candidate_id == "robust-winner")
            .expect("robust score");
        assert!((nominal.worst_case_eig_nats - 0.3404385439840655).abs() < 1.0e-12);
        assert!((robust.worst_case_eig_nats - 0.3589198965751584).abs() < 1.0e-12);
        assert!(result.finite_scenario_exact);
        assert!(!result.formal_interval_arithmetic);
    }

    #[test]
    fn rejects_pairwise_correlation_intervals_that_form_non_psd_matrix() {
        let invalid = FiniteCorrelationScenario {
            scenario_id: "impossible".to_owned(),
            correlation_matrix: vec![
                vec![1.0, -0.9, -0.9],
                vec![-0.9, 1.0, -0.9],
                vec![-0.9, -0.9, 1.0],
            ],
        };
        assert_eq!(
            rank_designs_under_finite_correlation_scenarios(&candidates(), &[invalid], RECEIPT,),
            Err(ModelError::InvalidCovariance(
                "source_correlation_matrix_must_be_positive_semidefinite"
            ))
        );
    }

    #[test]
    fn accepts_singular_psd_common_mode_correlation() {
        let common_mode = FiniteCorrelationScenario {
            scenario_id: "perfect-common-mode".to_owned(),
            correlation_matrix: vec![
                vec![1.0, 1.0, 1.0],
                vec![1.0, 1.0, 1.0],
                vec![1.0, 1.0, 1.0],
            ],
        };
        let result =
            rank_designs_under_finite_correlation_scenarios(&candidates(), &[common_mode], RECEIPT)
                .expect("singular PSD source correlation");
        assert_eq!(result.nominal_independence_preferred_candidate_id, None);
        assert_eq!(result.ranking_reversed, None);
    }

    #[test]
    fn requires_one_shared_prior_and_source_scale_scope() {
        let mut bad = candidates();
        bad[1].source_standard_deviations[2] = 1.2;
        assert_eq!(
            rank_designs_under_finite_correlation_scenarios(
                &bad,
                &three_source_scenarios(),
                RECEIPT,
            ),
            Err(ModelError::InvalidParameter(
                "multi_source_candidates_must_share_prior_and_source_marginals"
            ))
        );
    }

    #[test]
    fn malformed_scenario_shapes_and_duplicate_ids_fail_closed() {
        let malformed = FiniteCorrelationScenario {
            scenario_id: "bad-shape".to_owned(),
            correlation_matrix: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        };
        assert_eq!(
            rank_designs_under_finite_correlation_scenarios(&candidates(), &[malformed], RECEIPT,),
            Err(ModelError::MismatchedDataLengths)
        );

        let duplicate = vec![
            three_source_scenarios()[0].clone(),
            three_source_scenarios()[0].clone(),
        ];
        assert_eq!(
            rank_designs_under_finite_correlation_scenarios(&candidates(), &duplicate, RECEIPT),
            Err(ModelError::InvalidParameter(
                "multi_source_scenario_ids_must_be_nonempty_and_unique"
            ))
        );
    }

    #[test]
    fn near_symmetric_and_near_unit_diagonal_inputs_are_canonicalized() {
        let mut scenario = three_source_scenarios()[0].clone();
        scenario.correlation_matrix[0][0] = 1.0 + 5.0e-13;
        scenario.correlation_matrix[0][1] = 0.2 + 5.0e-13;
        scenario.correlation_matrix[1][0] = 0.2;
        scenario.correlation_matrix[0][2] = -0.1;
        scenario.correlation_matrix[2][0] = -0.1 - 5.0e-13;
        let candidates = candidates();
        let result = rank_designs_under_finite_correlation_scenarios(
            &candidates,
            &[scenario.clone()],
            RECEIPT,
        )
        .expect("near-symmetric input accepted and canonicalized");

        let mut exact = scenario;
        exact.correlation_matrix[0][0] = 1.0;
        exact.correlation_matrix[0][1] = 0.2 + 2.5e-13;
        exact.correlation_matrix[1][0] = 0.2 + 2.5e-13;
        exact.correlation_matrix[0][2] = -0.1 - 2.5e-13;
        exact.correlation_matrix[2][0] = -0.1 - 2.5e-13;
        let exact_result =
            rank_designs_under_finite_correlation_scenarios(&candidates, &[exact], RECEIPT)
                .expect("canonical symmetric reference");

        for (actual, expected) in result.scores.iter().zip(exact_result.scores.iter()) {
            assert!((actual.worst_case_eig_nats - expected.worst_case_eig_nats).abs() < 1.0e-14);
        }
    }

    #[test]
    fn non_identity_scenario_set_does_not_claim_independence_baseline() {
        let scenarios = vec![three_source_scenarios()[1].clone()];
        let result =
            rank_designs_under_finite_correlation_scenarios(&candidates(), &scenarios, RECEIPT)
                .expect("non-identity scenario set");
        assert!(result
            .scores
            .iter()
            .all(|score| score.nominal_independence_eig_nats.is_none()));
    }
}
