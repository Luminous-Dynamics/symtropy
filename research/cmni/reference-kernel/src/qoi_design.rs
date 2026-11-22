//! Goal-oriented experimental design for linear scientific quantities of interest.
//!
//! Parameter-space D-optimality can value information about parameters that do
//! not materially affect a downstream scientific conclusion. This module
//! projects prior and posterior covariance onto an explicitly declared linear
//! quantity-of-interest (QoI) map and scores uncertainty reduction in that
//! scientific subspace.
//!
//! For q = L theta, with Gaussian prior/posterior covariances P0 and P1:
//!
//!   Sigma_q,0 = L P0 L^T
//!   Sigma_q,1 = L P1 L^T
//!
//! The linear-Gaussian QoI EIG in nats is
//!
//!   0.5 * [logdet(Sigma_q,0) - logdet(Sigma_q,1)].
//!
//! This is exact for the declared linear-Gaussian model. It is not an exact
//! nonlinear predictive-QoI EIG estimator.

use crate::batch_design::BatchPortfolioScore;
use crate::ModelError;

/// Uncertainty reduction for a declared linear quantity-of-interest map.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearQuantityOfInterestScore {
    pub quantity_count: usize,
    pub prior_covariance: Vec<Vec<f64>>,
    pub posterior_covariance: Vec<Vec<f64>>,
    pub a_optimal_trace_reduction: f64,
    pub d_optimal_logdet_reduction: f64,
    pub expected_information_gain_nats: f64,
}

/// One portfolio ranked by its information about a linear scientific QoI.
#[derive(Debug, Clone, PartialEq)]
pub struct QuantityOfInterestPortfolioScore {
    pub portfolio: BatchPortfolioScore,
    pub quantity_score: LinearQuantityOfInterestScore,
}

/// Score a linear QoI map q = L theta under prior and posterior covariance.
///
/// Rows of the quantity map define scientific quantities; columns correspond
/// exactly to the parameter ordering in both covariance matrices. Parameter
/// and QoI coordinates should be dimensionless or explicitly scaled before
/// interpreting trace-based A-optimal utility.
pub fn score_linear_quantity_of_interest(
    prior_parameter_covariance: &[Vec<f64>],
    posterior_parameter_covariance: &[Vec<f64>],
    quantity_map: &[Vec<f64>],
) -> Result<LinearQuantityOfInterestScore, ModelError> {
    validate_square_covariance(prior_parameter_covariance, "qoi_prior_parameter_covariance")?;
    validate_square_covariance(
        posterior_parameter_covariance,
        "qoi_posterior_parameter_covariance",
    )?;
    let parameter_count = prior_parameter_covariance.len();
    if posterior_parameter_covariance.len() != parameter_count {
        return Err(ModelError::MismatchedDataLengths);
    }
    let quantity_count = quantity_map.len();
    if quantity_count == 0
        || quantity_map
            .iter()
            .any(|row| row.len() != parameter_count)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    if quantity_map.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue("qoi_linear_map"));
    }

    // A full-row-rank QoI map is required so that its prior and posterior
    // covariance matrices are positive definite and log determinants exist.
    let prior_qoi_covariance = project_covariance(prior_parameter_covariance, quantity_map)?;
    let posterior_qoi_covariance =
        project_covariance(posterior_parameter_covariance, quantity_map)?;
    let prior_logdet = logdet_spd(&prior_qoi_covariance, "qoi_prior_covariance")?;
    let posterior_logdet = logdet_spd(
        &posterior_qoi_covariance,
        "qoi_posterior_covariance",
    )?;
    let a_optimal_trace_reduction =
        trace(&prior_qoi_covariance)? - trace(&posterior_qoi_covariance)?;
    let d_optimal_logdet_reduction = prior_logdet - posterior_logdet;
    let expected_information_gain_nats = 0.5 * d_optimal_logdet_reduction;

    if !a_optimal_trace_reduction.is_finite()
        || !d_optimal_logdet_reduction.is_finite()
        || !expected_information_gain_nats.is_finite()
    {
        return Err(ModelError::InvalidParameter("qoi_utility_nonfinite"));
    }

    Ok(LinearQuantityOfInterestScore {
        quantity_count,
        prior_covariance: prior_qoi_covariance,
        posterior_covariance: posterior_qoi_covariance,
        a_optimal_trace_reduction,
        d_optimal_logdet_reduction,
        expected_information_gain_nats,
    })
}

/// Rank exact batch portfolios by information about the declared linear QoI.
///
/// This ranking is deliberately separate from parameter-space ranking: the
/// same experiment portfolios can have different orderings for different
/// scientific targets.
pub fn rank_portfolios_by_linear_quantity_of_interest(
    portfolios: &[BatchPortfolioScore],
    prior_parameter_covariance: &[Vec<f64>],
    quantity_map: &[Vec<f64>],
) -> Result<Vec<QuantityOfInterestPortfolioScore>, ModelError> {
    if portfolios.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut ranked = Vec::with_capacity(portfolios.len());
    for portfolio in portfolios {
        let quantity_score = score_linear_quantity_of_interest(
            prior_parameter_covariance,
            &portfolio.posterior_covariance,
            quantity_map,
        )?;
        ranked.push(QuantityOfInterestPortfolioScore {
            portfolio: portfolio.clone(),
            quantity_score,
        });
    }
    ranked.sort_by(|left, right| {
        right
            .quantity_score
            .expected_information_gain_nats
            .total_cmp(&left.quantity_score.expected_information_gain_nats)
            .then_with(|| {
                right
                    .quantity_score
                    .a_optimal_trace_reduction
                    .total_cmp(&left.quantity_score.a_optimal_trace_reduction)
            })
            .then_with(|| {
                left.portfolio
                    .candidate_ids
                    .cmp(&right.portfolio.candidate_ids)
            })
    });
    Ok(ranked)
}

fn validate_square_covariance(
    matrix: &[Vec<f64>],
    name: &'static str,
) -> Result<(), ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::NonFiniteValue(name));
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let symmetry_tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (value - matrix[column_index][row_index]).abs() > symmetry_tolerance {
                return Err(ModelError::InvalidCovariance(name));
            }
        }
    }
    Ok(())
}

fn project_covariance(
    parameter_covariance: &[Vec<f64>],
    quantity_map: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let map_transpose = transpose(quantity_map);
    let left = multiply(quantity_map, parameter_covariance)?;
    multiply(&left, &map_transpose)
}

fn multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty() || right.is_empty() {
        return Err(ModelError::MismatchedDataLengths);
    }
    let inner = left[0].len();
    let columns = right[0].len();
    if inner == 0
        || columns == 0
        || left.iter().any(|row| row.len() != inner)
        || right.len() != inner
        || right.iter().any(|row| row.len() != columns)
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut output = vec![vec![0.0; columns]; left.len()];
    for (row_index, output_row) in output.iter_mut().enumerate() {
        for (column_index, output_value) in output_row.iter_mut().enumerate() {
            *output_value = (0..inner)
                .map(|index| left[row_index][index] * right[index][column_index])
                .sum();
        }
    }
    if output.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter("qoi_covariance_product_nonfinite"));
    }
    Ok(output)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut output = vec![vec![0.0; rows]; columns];
    for (row_index, matrix_row) in matrix.iter().enumerate() {
        for (column_index, value) in matrix_row.iter().enumerate() {
            output[column_index][row_index] = *value;
        }
    }
    output
}

fn trace(matrix: &[Vec<f64>]) -> Result<f64, ModelError> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok(matrix
        .iter()
        .enumerate()
        .map(|(index, row)| row[index])
        .sum())
}

fn logdet_spd(matrix: &[Vec<f64>], name: &'static str) -> Result<f64, ModelError> {
    let dimension = matrix.len();
    if dimension == 0 || matrix.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let tolerance = 1.0e-10 * scale.max(f64::MIN_POSITIVE);
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate().take(row_index) {
            if (value - matrix[column_index][row_index]).abs() > tolerance {
                return Err(ModelError::InvalidCovariance(name));
            }
        }
    }
    let mut lower = vec![vec![0.0; dimension]; dimension];
    for (row_index, matrix_row) in matrix.iter().enumerate() {
        for (column_index, matrix_value) in matrix_row.iter().enumerate().take(row_index + 1) {
            let correction: f64 = lower[row_index][..column_index]
                .iter()
                .zip(lower[column_index][..column_index].iter())
                .map(|(left, right)| left * right)
                .sum();
            let residual = matrix_value - correction;
            if row_index == column_index {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(ModelError::InvalidCovariance(name));
                }
                lower[row_index][column_index] = residual.sqrt();
            } else {
                lower[row_index][column_index] = residual / lower[column_index][column_index];
            }
        }
    }
    let logdet =
        2.0 * (0..dimension).map(|index| lower[index][index].ln()).sum::<f64>();
    if !logdet.is_finite() {
        return Err(ModelError::InvalidCovariance(name));
    }
    Ok(logdet)
}

/// Finite-scenario risk summary for a declared utility (larger is better).
///
/// The lower-tail CVaR uses equal scenario weights and the exact fractional
/// mass at the alpha quantile. This is a finite-scenario risk functional, not
/// distributionally robust Bayesian design or a probabilistic guarantee.
#[derive(Debug, Clone, PartialEq)]
pub struct FiniteScenarioRiskScore {
    pub portfolio_id: String,
    pub scenario_count: usize,
    pub alpha: f64,
    pub expected_utility: f64,
    pub lower_tail_cvar: f64,
    pub worst_case_utility: f64,
}

/// Rank portfolios using equal-weight finite-scenario lower-tail CVaR.
///
/// This convenience wrapper treats every declared scenario as equally likely.
pub fn rank_portfolios_by_finite_scenario_lower_cvar(
    utilities_by_portfolio: &[(String, Vec<f64>)],
    alpha: f64,
) -> Result<Vec<FiniteScenarioRiskScore>, ModelError> {
    let scenario_count = utilities_by_portfolio
        .first()
        .map(|(_, utilities)| utilities.len())
        .ok_or(ModelError::MismatchedDataLengths)?;
    rank_portfolios_by_weighted_finite_scenario_lower_cvar(
        utilities_by_portfolio,
        &vec![1.0; scenario_count],
        alpha,
    )
}

/// Rank portfolios by weighted lower-tail CVaR over a common finite scenario set.
///
/// Scenario weights are nonnegative and normalized internally. Zero-weight
/// scenarios are ignored for expected utility and worst-case reporting. The
/// lower-tail CVaR takes exactly the worst alpha probability mass, including
/// a fractional contribution from the boundary scenario when necessary.
///
/// This is a finite-scenario risk functional, not distributionally robust
/// Bayesian design or a probabilistic guarantee.
pub fn rank_portfolios_by_weighted_finite_scenario_lower_cvar(
    utilities_by_portfolio: &[(String, Vec<f64>)],
    scenario_weights: &[f64],
    alpha: f64,
) -> Result<Vec<FiniteScenarioRiskScore>, ModelError> {
    if !alpha.is_finite() {
        return Err(ModelError::NonFiniteValue("risk_alpha"));
    }
    if alpha <= 0.0 || alpha > 1.0 {
        return Err(ModelError::InvalidParameter("risk_alpha_must_be_in_(0,1]"));
    }
    let scenario_count = utilities_by_portfolio
        .first()
        .map(|(_, utilities)| utilities.len())
        .ok_or(ModelError::MismatchedDataLengths)?;
    if scenario_count == 0 || scenario_weights.len() != scenario_count {
        return Err(ModelError::MismatchedDataLengths);
    }
    if scenario_weights.iter().any(|weight| !weight.is_finite()) {
        return Err(ModelError::NonFiniteValue("risk_scenario_weight"));
    }
    if scenario_weights.iter().any(|weight| *weight < 0.0) {
        return Err(ModelError::InvalidParameter(
            "risk_scenario_weights_must_be_nonnegative",
        ));
    }
    let total_weight = scenario_weights.iter().sum::<f64>();
    if !total_weight.is_finite() {
        return Err(ModelError::InvalidParameter("risk_scenario_weight_sum_nonfinite"));
    }
    if total_weight <= 0.0 {
        return Err(ModelError::NonPositiveValue("risk_scenario_weight_sum"));
    }
    let normalized_weights: Vec<f64> = scenario_weights
        .iter()
        .map(|weight| weight / total_weight)
        .collect();

    let mut seen = std::collections::BTreeSet::new();
    let mut scores = Vec::with_capacity(utilities_by_portfolio.len());
    for (portfolio_id, utilities) in utilities_by_portfolio {
        if portfolio_id.trim().is_empty() || !seen.insert(portfolio_id.as_str()) {
            return Err(ModelError::InvalidParameter(
                "risk_portfolio_ids_must_be_nonempty_and_unique",
            ));
        }
        if utilities.len() != scenario_count {
            return Err(ModelError::MismatchedDataLengths);
        }
        if utilities.iter().any(|value| !value.is_finite()) {
            return Err(ModelError::NonFiniteValue("risk_scenario_utility"));
        }

        let mut weighted_scenarios: Vec<(f64, f64, usize)> = utilities
            .iter()
            .copied()
            .zip(normalized_weights.iter().copied())
            .enumerate()
            .filter_map(|(index, (utility, weight))| {
                (weight > 0.0).then_some((utility, weight, index))
            })
            .collect();
        weighted_scenarios.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.2.cmp(&right.2))
        });

        let expected_utility = weighted_scenarios
            .iter()
            .map(|(utility, weight, _)| utility * weight)
            .sum::<f64>();
        let worst_case_utility = weighted_scenarios[0].0;
        let mut remaining_mass = alpha;
        let mut lower_tail_sum = 0.0;
        for (utility, weight, _) in &weighted_scenarios {
            if remaining_mass <= 1.0e-14 {
                break;
            }
            let included_mass = weight.min(remaining_mass);
            lower_tail_sum += utility * included_mass;
            remaining_mass -= included_mass;
        }
        if remaining_mass > 1.0e-10 {
            return Err(ModelError::InvalidParameter(
                "risk_scenario_weights_do_not_cover_probability_mass",
            ));
        }
        let lower_tail_cvar = lower_tail_sum / alpha;
        if !lower_tail_cvar.is_finite() || !expected_utility.is_finite() {
            return Err(ModelError::InvalidParameter("risk_utility_nonfinite"));
        }

        scores.push(FiniteScenarioRiskScore {
            portfolio_id: portfolio_id.clone(),
            scenario_count,
            alpha,
            expected_utility,
            lower_tail_cvar,
            worst_case_utility,
        });
    }

    scores.sort_by(|left, right| {
        right
            .lower_tail_cvar
            .total_cmp(&left.lower_tail_cvar)
            .then_with(|| right.expected_utility.total_cmp(&left.expected_utility))
            .then_with(|| left.portfolio_id.cmp(&right.portfolio_id))
    });
    Ok(scores)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn portfolio(id: &str, posterior_covariance: Vec<Vec<f64>>) -> BatchPortfolioScore {
        BatchPortfolioScore {
            candidate_ids: vec![id.to_owned()],
            total_cost: 1.0,
            observation_count: 1,
            posterior_covariance,
            a_optimal_trace_reduction: 0.0,
            d_optimal_logdet_reduction: 0.0,
            linear_gaussian_expected_information_gain_nats: 0.0,
            lineage_coverage_complete: true,
            inter_source_dependence_complete: true,
            inter_source_dependence_receipt_ids: Vec::new(),
            shared_source_ids: Vec::new(),
        }
    }

    #[test]
    fn quantity_of_interest_can_reverse_parameter_space_portfolio_ranking() {
        let prior = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let ab = portfolio(
            "A+B",
            vec![vec![181.0 / 201.0, 0.0], vec![0.0, 1.0]],
        );
        let ac = portfolio(
            "A+C",
            vec![vec![91.0 / 101.0, 0.0], vec![0.0, 20.0 / 21.0]],
        );
        let portfolios = vec![ab, ac];

        // A physical-component target slightly prefers the shared pair.
        let physical = rank_portfolios_by_linear_quantity_of_interest(
            &portfolios,
            &prior,
            &[vec![1.0, 0.0]],
        )
        .expect("physical QoI ranking");
        assert_eq!(physical[0].portfolio.candidate_ids, vec!["A+B"]);
        assert!(
            (physical[0].quantity_score.expected_information_gain_nats
                - 0.5 * (201.0_f64 / 181.0).ln())
                .abs()
                < 1.0e-12
        );

        // A measurement-artifact target prefers the complementary experiment.
        let artifact = rank_portfolios_by_linear_quantity_of_interest(
            &portfolios,
            &prior,
            &[vec![0.0, 1.0]],
        )
        .expect("artifact QoI ranking");
        assert_eq!(artifact[0].portfolio.candidate_ids, vec!["A+C"]);
        assert!(
            (artifact[0].quantity_score.expected_information_gain_nats
                - 0.5 * (21.0_f64 / 20.0).ln())
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn full_identity_qoi_matches_parameter_space_linear_gaussian_eig() {
        let prior = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let posterior = vec![vec![0.8, 0.1], vec![0.1, 0.6]];
        let result = score_linear_quantity_of_interest(
            &prior,
            &posterior,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
        )
        .expect("identity QoI");
        let posterior_det = 0.8_f64 * 0.6 - 0.01;
        assert!(
            (result.expected_information_gain_nats - 0.5 * -posterior_det.ln()).abs() < 1.0e-12
        );
    }

    #[test]
    fn lower_tail_cvar_uses_fractional_probability_mass() {
        let scores = rank_portfolios_by_finite_scenario_lower_cvar(
            &[("portfolio".to_owned(), vec![0.1, 0.2, 0.3, 1.0])],
            0.5,
        )
        .expect("lower-tail score");
        assert_eq!(scores.len(), 1);
        assert!((scores[0].lower_tail_cvar - 0.15).abs() < 1.0e-12);
        let worst_quartile = rank_portfolios_by_finite_scenario_lower_cvar(
            &[("portfolio".to_owned(), vec![0.1, 0.2, 0.3, 1.0])],
            0.25,
        )
        .expect("worst-quartile score");
        assert!((worst_quartile[0].lower_tail_cvar - 0.1).abs() < 1.0e-12);
        let full_mean = rank_portfolios_by_finite_scenario_lower_cvar(
            &[("portfolio".to_owned(), vec![0.1, 0.2, 0.3, 1.0])],
            1.0,
        )
        .expect("full-distribution score");
        assert!((full_mean[0].lower_tail_cvar - 0.4).abs() < 1.0e-12);
    }

    #[test]
    fn lower_tail_cvar_can_reverse_expected_utility_ranking() {
        let scores = rank_portfolios_by_finite_scenario_lower_cvar(
            &[
                ("high_variance".to_owned(), vec![0.0, 0.0, 100.0]),
                ("steady".to_owned(), vec![10.0, 10.0, 10.0]),
            ],
            1.0 / 3.0,
        )
        .expect("risk-aware ranking");
        assert_eq!(scores[0].portfolio_id, "steady");
        assert_eq!(scores[0].lower_tail_cvar, 10.0);
        assert_eq!(scores[1].lower_tail_cvar, 0.0);
        assert!(scores[1].expected_utility > scores[0].expected_utility);
    }

    #[test]
    fn weighted_lower_tail_cvar_respects_declared_scenario_probabilities() {
        let scores = rank_portfolios_by_weighted_finite_scenario_lower_cvar(
            &[("weighted".to_owned(), vec![0.0, 10.0, 20.0])],
            &[0.1, 0.7, 0.2],
            0.25,
        )
        .expect("weighted lower-tail score");
        assert_eq!(scores.len(), 1);
        assert!((scores[0].expected_utility - 11.0).abs() < 1.0e-12);
        assert!((scores[0].lower_tail_cvar - 6.0).abs() < 1.0e-12);
        assert_eq!(scores[0].worst_case_utility, 0.0);
    }

    #[test]
    fn weighted_lower_tail_cvar_rejects_invalid_weights() {
        assert_eq!(
            rank_portfolios_by_weighted_finite_scenario_lower_cvar(
                &[("p".to_owned(), vec![1.0, 2.0])],
                &[0.0, 0.0],
                0.5
            ),
            Err(ModelError::NonPositiveValue("risk_scenario_weight_sum"))
        );
        assert_eq!(
            rank_portfolios_by_weighted_finite_scenario_lower_cvar(
                &[("p".to_owned(), vec![1.0, 2.0])],
                &[0.5, -0.5],
                0.5
            ),
            Err(ModelError::InvalidParameter(
                "risk_scenario_weights_must_be_nonnegative"
            ))
        );
    }

    #[test]
    fn lower_tail_cvar_rejects_invalid_scenario_inputs() {
        assert_eq!(
            rank_portfolios_by_finite_scenario_lower_cvar(
                &[("p".to_owned(), vec![1.0, 2.0])],
                0.0
            ),
            Err(ModelError::InvalidParameter("risk_alpha_must_be_in_(0,1]"))
        );
        assert_eq!(
            rank_portfolios_by_finite_scenario_lower_cvar(
                &[("p".to_owned(), vec![1.0, f64::NAN])],
                0.5
            ),
            Err(ModelError::NonFiniteValue("risk_scenario_utility"))
        );
    }

    #[test]
    fn dependent_or_empty_quantity_maps_fail_closed() {
        let prior = vec![vec![1.0, 0.0], vec![0.0, 1.0]];
        let posterior = vec![vec![0.8, 0.0], vec![0.0, 0.6]];
        assert_eq!(
            score_linear_quantity_of_interest(&prior, &posterior, &[]),
            Err(ModelError::MismatchedDataLengths)
        );
        assert_eq!(
            score_linear_quantity_of_interest(
                &prior,
                &posterior,
                &[vec![1.0, 0.0], vec![2.0, 0.0]],
            ),
            Err(ModelError::InvalidCovariance("qoi_prior_covariance"))
        );
    }
}
