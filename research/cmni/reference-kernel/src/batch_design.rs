//! Exact small-set batch design using joint common-cause covariance.
//!
//! This exhaustive implementation is a reference oracle for tractable discrete
//! candidate sets, not a scalable production optimizer. It evaluates a
//! linear-Gaussian posterior using the full observation covariance assembled
//! from independent noise and named shared uncertainty sources.

use crate::common_cause_covariance::{assemble_joint_experiment_covariance, ExperimentNoiseModel};
use crate::ModelError;
use std::collections::BTreeSet;

/// One discrete candidate experiment in a batch-design problem.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchDesignCandidate {
    pub candidate_id: String,
    pub cost: f64,
    pub feasible: bool,
    /// Observation Jacobian, shaped observation_count x parameter_count.
    pub design_jacobian: Vec<Vec<f64>>,
    /// Independent observation noise and named shared-source sensitivities.
    pub noise_model: ExperimentNoiseModel,
}

/// Exact posterior and utility for one candidate portfolio.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchPortfolioScore {
    /// Candidate IDs in deterministic input order.
    pub candidate_ids: Vec<String>,
    pub total_cost: f64,
    pub observation_count: usize,
    pub posterior_covariance: Vec<Vec<f64>>,
    /// Trace reduction in the caller's declared parameter coordinates.
    pub a_optimal_trace_reduction: f64,
    /// Log-determinant reduction in the caller's declared parameter coordinates.
    pub d_optimal_logdet_reduction: f64,
    /// Exact linear-Gaussian expected information gain, in nats.
    /// Equals one half of the D-optimal log-determinant reduction.
    pub linear_gaussian_expected_information_gain_nats: f64,
    /// False means unknown common causes may still be absent from the covariance.
    pub lineage_coverage_complete: bool,
    pub shared_source_ids: Vec<String>,
}

/// Exhaustive small-set result, including explicit exclusion counts.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchPortfolioEnumeration {
    /// Sorted by descending A-optimal utility, then cost, then candidate IDs.
    pub portfolios: Vec<BatchPortfolioScore>,
    pub skipped_infeasible_candidates: usize,
    pub skipped_over_budget_portfolios: usize,
    pub skipped_incomplete_lineage_portfolios: usize,
}

/// Greedy batch selection, with every marginal utility recomputed using the
/// full common-cause covariance of the already-selected candidates.
#[derive(Debug, Clone, PartialEq)]
pub struct GreedyBatchSelection {
    pub portfolio: BatchPortfolioScore,
    pub selection_order: Vec<String>,
    pub marginal_a_optimal_gains: Vec<f64>,
}

/// Enumerate every feasible portfolio of exactly the requested cardinality.
///
/// Shared-source IDs induce cross-candidate covariance. When complete lineage
/// is required, portfolios with incomplete provenance are excluded and counted.
/// Otherwise they are scored but remain explicitly unqualified.
///
/// Exponential complexity is intentional: this is an exact oracle for small
/// sets against which scalable heuristics can be measured.
pub fn enumerate_batch_portfolios(
    candidates: &[BatchDesignCandidate],
    prior_covariance: &[Vec<f64>],
    batch_size: usize,
    budget: f64,
    require_complete_lineage: bool,
) -> Result<BatchPortfolioEnumeration, ModelError> {
    validate_prior(prior_covariance)?;
    if !budget.is_finite() {
        return Err(ModelError::NonFiniteValue("batch_design_budget"));
    }
    if budget <= 0.0 {
        return Err(ModelError::NonPositiveValue("batch_design_budget"));
    }
    if batch_size == 0 {
        return Err(ModelError::InvalidParameter("batch_size_must_be_positive"));
    }

    let parameter_count = prior_covariance.len();
    let mut candidate_ids = BTreeSet::new();
    let mut feasible_indices = Vec::new();
    let mut skipped_infeasible_candidates = 0usize;

    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.candidate_id.trim().is_empty()
            || !candidate_ids.insert(candidate.candidate_id.as_str())
        {
            return Err(ModelError::InvalidParameter(
                "batch_candidate_ids_must_be_nonempty_and_unique",
            ));
        }
        if !candidate.cost.is_finite() {
            return Err(ModelError::NonFiniteValue("batch_candidate_cost"));
        }
        if candidate.cost <= 0.0 {
            return Err(ModelError::NonPositiveValue("batch_candidate_cost"));
        }
        if candidate.noise_model.experiment_id != candidate.candidate_id {
            return Err(ModelError::InvalidParameter(
                "batch_candidate_and_noise_model_ids_must_match",
            ));
        }
        if candidate.design_jacobian.is_empty()
            || candidate
                .design_jacobian
                .iter()
                .any(|row| row.len() != parameter_count)
        {
            return Err(ModelError::MismatchedDataLengths);
        }
        if candidate
            .design_jacobian
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(ModelError::NonFiniteValue("batch_design_jacobian"));
        }
        if candidate
            .noise_model
            .independent_observation_covariance
            .len()
            != candidate.design_jacobian.len()
        {
            return Err(ModelError::MismatchedDataLengths);
        }

        // Validate covariance and shared-source dimensions even for candidates
        // that are later excluded as physically infeasible.
        assemble_joint_experiment_covariance(std::slice::from_ref(&candidate.noise_model))?;

        if candidate.feasible {
            feasible_indices.push(index);
        } else {
            skipped_infeasible_candidates += 1;
        }
    }

    let mut combinations = Vec::new();
    let mut current = Vec::with_capacity(batch_size);
    enumerate_combinations(
        &feasible_indices,
        0,
        batch_size,
        &mut current,
        &mut combinations,
    );

    let mut portfolios = Vec::new();
    let mut skipped_over_budget_portfolios = 0usize;
    let mut skipped_incomplete_lineage_portfolios = 0usize;

    for combination in combinations {
        let selected: Vec<&BatchDesignCandidate> = combination
            .iter()
            .map(|index| &candidates[*index])
            .collect();
        let total_cost: f64 = selected.iter().map(|candidate| candidate.cost).sum();
        if !total_cost.is_finite() {
            return Err(ModelError::InvalidParameter(
                "batch_portfolio_cost_overflowed",
            ));
        }
        if total_cost > budget {
            skipped_over_budget_portfolios += 1;
            continue;
        }

        let lineage_coverage_complete = selected
            .iter()
            .all(|candidate| candidate.noise_model.lineage_coverage_complete);
        if require_complete_lineage && !lineage_coverage_complete {
            skipped_incomplete_lineage_portfolios += 1;
            continue;
        }

        let noise_models: Vec<ExperimentNoiseModel> = selected
            .iter()
            .map(|candidate| candidate.noise_model.clone())
            .collect();
        let joint = assemble_joint_experiment_covariance(&noise_models)?;
        let mut design_jacobian = Vec::new();
        for candidate in &selected {
            design_jacobian.extend(candidate.design_jacobian.iter().cloned());
        }
        if design_jacobian.len() != joint.covariance.len() {
            return Err(ModelError::MismatchedDataLengths);
        }

        let (posterior_covariance, prior_logdet, posterior_precision_logdet) =
            gaussian_linear_posterior(prior_covariance, &joint.covariance, &design_jacobian)?;
        let prior_trace = trace(prior_covariance)?;
        let posterior_trace = trace(&posterior_covariance)?;
        let a_optimal_trace_reduction = prior_trace - posterior_trace;
        // logdet(P_prior) - logdet(P_posterior) equals
        // logdet(P_prior) + logdet(P_posterior_precision).
        let d_optimal_logdet_reduction = prior_logdet + posterior_precision_logdet;
        let linear_gaussian_expected_information_gain_nats = 0.5 * d_optimal_logdet_reduction;
        if !a_optimal_trace_reduction.is_finite()
            || !d_optimal_logdet_reduction.is_finite()
            || !linear_gaussian_expected_information_gain_nats.is_finite()
        {
            return Err(ModelError::InvalidParameter(
                "batch_portfolio_utility_nonfinite",
            ));
        }

        portfolios.push(BatchPortfolioScore {
            candidate_ids: selected
                .iter()
                .map(|candidate| candidate.candidate_id.clone())
                .collect(),
            total_cost,
            observation_count: design_jacobian.len(),
            posterior_covariance,
            a_optimal_trace_reduction,
            d_optimal_logdet_reduction,
            linear_gaussian_expected_information_gain_nats,
            lineage_coverage_complete: joint.lineage_coverage_complete,
            shared_source_ids: joint.shared_source_ids,
        });
    }

    portfolios.sort_by(|left, right| {
        right
            .a_optimal_trace_reduction
            .total_cmp(&left.a_optimal_trace_reduction)
            .then_with(|| left.total_cost.total_cmp(&right.total_cost))
            .then_with(|| left.candidate_ids.cmp(&right.candidate_ids))
    });

    Ok(BatchPortfolioEnumeration {
        portfolios,
        skipped_infeasible_candidates,
        skipped_over_budget_portfolios,
        skipped_incomplete_lineage_portfolios,
    })
}

/// Select a fixed-cardinality portfolio by sequential marginal A-optimal gain.
///
/// At each step every remaining candidate is rescored jointly with the selected
/// set. This preserves shared-source cross-covariance when calculating the
/// marginal value. This is a practical baseline heuristic, not a global
/// optimality guarantee.
pub fn select_greedy_batch_portfolio(
    candidates: &[BatchDesignCandidate],
    prior_covariance: &[Vec<f64>],
    batch_size: usize,
    budget: f64,
    require_complete_lineage: bool,
) -> Result<GreedyBatchSelection, ModelError> {
    if batch_size == 0 {
        return Err(ModelError::InvalidParameter("batch_size_must_be_positive"));
    }

    // Reuse the exact evaluator to validate candidate definitions and the prior.
    enumerate_batch_portfolios(candidates, prior_covariance, 1, budget, false)?;

    let mut selected_indices = Vec::with_capacity(batch_size);
    let mut selection_order = Vec::with_capacity(batch_size);
    let mut marginal_a_optimal_gains = Vec::with_capacity(batch_size);
    let mut current_utility = 0.0;
    let mut final_score: Option<BatchPortfolioScore> = None;

    for _ in 0..batch_size {
        let mut best: Option<(usize, BatchPortfolioScore, f64)> = None;
        for candidate_index in 0..candidates.len() {
            let candidate = &candidates[candidate_index];
            if !candidate.feasible || selected_indices.contains(&candidate_index) {
                continue;
            }

            let mut trial: Vec<BatchDesignCandidate> = selected_indices
                .iter()
                .map(|index| candidates[*index].clone())
                .collect();
            trial.push(candidate.clone());

            let mut scored = enumerate_batch_portfolios(
                &trial,
                prior_covariance,
                trial.len(),
                budget,
                require_complete_lineage,
            )?;
            if scored.portfolios.len() != 1 {
                continue;
            }

            let score = scored.portfolios.remove(0);
            let marginal_gain = score.a_optimal_trace_reduction - current_utility;
            let replace_best = match &best {
                None => true,
                Some((best_index, _, best_gain)) => {
                    marginal_gain > *best_gain
                        || (marginal_gain == *best_gain
                            && candidate.candidate_id < candidates[*best_index].candidate_id)
                }
            };
            if replace_best {
                best = Some((candidate_index, score, marginal_gain));
            }
        }

        let Some((candidate_index, score, marginal_gain)) = best else {
            return Err(ModelError::InvalidParameter(
                "no_feasible_greedy_batch_portfolio",
            ));
        };
        selected_indices.push(candidate_index);
        selection_order.push(candidates[candidate_index].candidate_id.clone());
        marginal_a_optimal_gains.push(marginal_gain);
        current_utility = score.a_optimal_trace_reduction;
        final_score = Some(score);
    }

    Ok(GreedyBatchSelection {
        portfolio: final_score.ok_or(ModelError::InvalidParameter(
            "greedy_batch_portfolio_missing_final_score",
        ))?,
        selection_order,
        marginal_a_optimal_gains,
    })
}

fn enumerate_combinations(
    values: &[usize],
    start: usize,
    remaining: usize,
    current: &mut Vec<usize>,
    output: &mut Vec<Vec<usize>>,
) {
    if remaining == 0 {
        output.push(current.clone());
        return;
    }
    if values.len().saturating_sub(start) < remaining {
        return;
    }
    let final_start = values.len() - remaining;
    for position in start..=final_start {
        current.push(values[position]);
        enumerate_combinations(values, position + 1, remaining - 1, current, output);
        current.pop();
    }
}

fn validate_prior(prior_covariance: &[Vec<f64>]) -> Result<(), ModelError> {
    let dimension = prior_covariance.len();
    if dimension == 0 || prior_covariance.iter().any(|row| row.len() != dimension) {
        return Err(ModelError::MismatchedDataLengths);
    }
    if prior_covariance
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(ModelError::NonFiniteValue("batch_prior_covariance"));
    }
    cholesky(prior_covariance, "batch_prior_covariance").map(|_| ())
}

fn gaussian_linear_posterior(
    prior_covariance: &[Vec<f64>],
    observation_covariance: &[Vec<f64>],
    design_jacobian: &[Vec<f64>],
) -> Result<(Vec<Vec<f64>>, f64, f64), ModelError> {
    if observation_covariance.len() != design_jacobian.len()
        || observation_covariance
            .iter()
            .any(|row| row.len() != observation_covariance.len())
        || design_jacobian.is_empty()
        || design_jacobian
            .iter()
            .any(|row| row.len() != prior_covariance.len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }

    let prior_lower = cholesky(prior_covariance, "batch_prior_covariance")?;
    let observation_lower = cholesky(observation_covariance, "batch_observation_covariance")?;
    let prior_precision = inverse_from_cholesky(&prior_lower)?;
    let observation_weighted_design = solve_spd(&observation_lower, design_jacobian)?;
    let information_increment =
        multiply(&transpose(design_jacobian), &observation_weighted_design)?;
    let posterior_precision = add(&prior_precision, &information_increment)?;
    let posterior_lower = cholesky(&posterior_precision, "batch_posterior_precision")?;
    let posterior_covariance = inverse_from_cholesky(&posterior_lower)?;

    Ok((
        posterior_covariance,
        logdet_from_cholesky(&prior_lower),
        logdet_from_cholesky(&posterior_lower),
    ))
}

fn cholesky(matrix: &[Vec<f64>], name: &'static str) -> Result<Vec<Vec<f64>>, ModelError> {
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

    let mut lower = vec![vec![0.0; dimension]; dimension];
    for row in 0..dimension {
        for column in 0..=row {
            let correction: f64 = (0..column)
                .map(|index| lower[row][index] * lower[column][index])
                .sum();
            let residual = matrix[row][column] - correction;
            if row == column {
                if !residual.is_finite() || residual <= 0.0 {
                    return Err(ModelError::InvalidCovariance(name));
                }
                lower[row][column] = residual.sqrt();
            } else {
                let pivot = lower[column][column];
                if pivot <= 0.0 {
                    return Err(ModelError::InvalidCovariance(name));
                }
                lower[row][column] = residual / pivot;
            }
        }
    }
    Ok(lower)
}

fn solve_spd(
    lower: &[Vec<f64>],
    right_hand_side: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = lower.len();
    if dimension == 0
        || lower.iter().any(|row| row.len() != dimension)
        || right_hand_side.len() != dimension
        || right_hand_side.is_empty()
        || right_hand_side
            .iter()
            .any(|row| row.len() != right_hand_side[0].len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let columns = right_hand_side[0].len();
    let mut forward = vec![vec![0.0; columns]; dimension];
    for row in 0..dimension {
        for column in 0..columns {
            let correction: f64 = (0..row)
                .map(|index| lower[row][index] * forward[index][column])
                .sum();
            forward[row][column] = (right_hand_side[row][column] - correction) / lower[row][row];
        }
    }

    let mut solution = vec![vec![0.0; columns]; dimension];
    for row in (0..dimension).rev() {
        for column in 0..columns {
            let correction: f64 = (row + 1..dimension)
                .map(|index| lower[index][row] * solution[index][column])
                .sum();
            solution[row][column] = (forward[row][column] - correction) / lower[row][row];
        }
    }
    if solution.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter("batch_spd_solve_nonfinite"));
    }
    Ok(solution)
}

fn inverse_from_cholesky(lower: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    let dimension = lower.len();
    let identity: Vec<Vec<f64>> = (0..dimension)
        .map(|row| {
            (0..dimension)
                .map(|column| if row == column { 1.0 } else { 0.0 })
                .collect()
        })
        .collect();
    solve_spd(lower, &identity)
}

fn logdet_from_cholesky(lower: &[Vec<f64>]) -> f64 {
    2.0 * (0..lower.len())
        .map(|index| lower[index][index].ln())
        .sum::<f64>()
}

fn trace(matrix: &[Vec<f64>]) -> Result<f64, ModelError> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err(ModelError::MismatchedDataLengths);
    }
    Ok((0..matrix.len()).map(|index| matrix[index][index]).sum())
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
    for row in 0..left.len() {
        for column in 0..columns {
            output[row][column] = (0..inner)
                .map(|index| left[row][index] * right[index][column])
                .sum();
        }
    }
    if output.iter().flatten().any(|value| !value.is_finite()) {
        return Err(ModelError::InvalidParameter(
            "batch_matrix_product_nonfinite",
        ));
    }
    Ok(output)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    let mut output = vec![vec![0.0; rows]; columns];
    for row in 0..rows {
        for column in 0..columns {
            output[column][row] = matrix[row][column];
        }
    }
    output
}

fn add(left: &[Vec<f64>], right: &[Vec<f64>]) -> Result<Vec<Vec<f64>>, ModelError> {
    if left.is_empty()
        || left.len() != right.len()
        || left.iter().any(|row| row.len() != right.len())
        || right.iter().any(|row| row.len() != right.len())
    {
        return Err(ModelError::MismatchedDataLengths);
    }
    let mut output = vec![vec![0.0; left.len()]; left.len()];
    for row in 0..left.len() {
        for column in 0..left.len() {
            output[row][column] = left[row][column] + right[row][column];
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_cause_covariance::SharedUncertaintyContribution;

    fn candidate(
        id: &str,
        sensitivity: [f64; 2],
        independent_variance: f64,
        shared: bool,
        shared_variance: f64,
        lineage_complete: bool,
    ) -> BatchDesignCandidate {
        let shared_sources = if shared {
            vec![SharedUncertaintyContribution {
                source_id: "calibration:A-B".to_owned(),
                source_covariance: vec![vec![shared_variance]],
                observation_jacobian: vec![vec![1.0]],
            }]
        } else {
            Vec::new()
        };
        BatchDesignCandidate {
            candidate_id: id.to_owned(),
            cost: 1.0,
            feasible: true,
            design_jacobian: vec![vec![sensitivity[0], sensitivity[1]]],
            noise_model: ExperimentNoiseModel {
                experiment_id: id.to_owned(),
                independent_observation_covariance: vec![vec![independent_variance]],
                shared_sources,
                lineage_coverage_complete: lineage_complete,
            },
        }
    }

    #[test]
    fn exhaustive_oracle_ranks_complementary_measurement_above_shared_pair() {
        let candidates = vec![
            candidate("A", [1.0, 0.0], 0.1, true, 9.0, true),
            candidate("B", [1.0, 0.0], 0.1, true, 9.0, true),
            candidate("C", [0.0, 1.0], 20.0, false, 0.0, true),
        ];
        let result = enumerate_batch_portfolios(
            &candidates,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            2,
            2.0,
            true,
        )
        .expect("enumerate batch portfolios");
        assert_eq!(result.portfolios.len(), 3);
        assert_eq!(result.portfolios[0].candidate_ids, vec!["A", "C"]);
        let shared = result
            .portfolios
            .iter()
            .find(|score| score.candidate_ids == vec!["A", "B"])
            .expect("shared A+B portfolio");
        assert!((shared.posterior_covariance[0][0] - 181.0 / 201.0).abs() < 1.0e-12);
        assert!((shared.posterior_covariance[1][1] - 1.0).abs() < 1.0e-12);
        assert!((shared.a_optimal_trace_reduction - 20.0 / 201.0).abs() < 1.0e-12);
        assert!((shared.d_optimal_logdet_reduction - (201.0_f64 / 181.0).ln()).abs() < 1.0e-12);
        assert!(
            (shared.linear_gaussian_expected_information_gain_nats
                - 0.5 * (201.0_f64 / 181.0).ln())
            .abs()
                < 1.0e-12
        );
        let complementary = &result.portfolios[0];
        assert!((complementary.posterior_covariance[0][0] - 91.0 / 101.0).abs() < 1.0e-12);
        assert!((complementary.posterior_covariance[1][1] - 20.0 / 21.0).abs() < 1.0e-12);
        assert!((complementary.a_optimal_trace_reduction - 311.0 / 2121.0).abs() < 1.0e-12);
        assert!(
            (complementary.d_optimal_logdet_reduction - (2121.0_f64 / 1820.0).ln()).abs() < 1.0e-12
        );
        assert!(
            (complementary.linear_gaussian_expected_information_gain_nats
                - 0.5 * (2121.0_f64 / 1820.0).ln())
            .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn greedy_recomputes_marginal_utility_with_shared_covariance() {
        let candidates = vec![
            candidate("A", [1.0, 0.0], 0.1, true, 9.0, true),
            candidate("B", [1.0, 0.0], 0.1, true, 9.0, true),
            candidate("C", [0.0, 1.0], 20.0, false, 0.0, true),
        ];
        let selected = select_greedy_batch_portfolio(
            &candidates,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            2,
            2.0,
            true,
        )
        .expect("greedy batch selection");
        assert_eq!(selected.selection_order, vec!["A", "C"]);
        assert_eq!(selected.portfolio.candidate_ids, vec!["A", "C"]);
        assert!(selected.marginal_a_optimal_gains[0] > 0.0);
        assert!(selected.marginal_a_optimal_gains[1] > 0.0);
    }

    #[test]
    fn greedy_abstains_when_lineage_prevents_a_complete_batch() {
        let candidates = vec![
            candidate("A", [1.0, 0.0], 1.0, false, 0.0, true),
            candidate("B", [0.0, 1.0], 1.0, false, 0.0, false),
        ];
        assert_eq!(
            select_greedy_batch_portfolio(
                &candidates,
                &[vec![1.0, 0.0], vec![0.0, 1.0]],
                2,
                2.0,
                true
            ),
            Err(ModelError::InvalidParameter(
                "no_feasible_greedy_batch_portfolio"
            ))
        );
    }

    #[test]
    fn nonunit_prior_covariance_has_correct_d_optimal_sign_and_scale() {
        let candidates = vec![
            candidate("observe-x", [1.0, 0.0], 1.0, false, 0.0, true),
        ];
        let result = enumerate_batch_portfolios(
            &candidates,
            &[vec![2.0, 0.0], vec![0.0, 3.0]],
            1,
            1.0,
            true,
        )
        .expect("non-unit prior portfolio");
        let score = &result.portfolios[0];
        let expected_x_variance = 2.0 / 3.0;
        let expected_y_variance = 3.0;
        let expected_d_optimal = 3.0_f64.ln();
        assert!((score.posterior_covariance[0][0] - expected_x_variance).abs() < 1.0e-12);
        assert!((score.posterior_covariance[1][1] - expected_y_variance).abs() < 1.0e-12);
        assert!((score.d_optimal_logdet_reduction - expected_d_optimal).abs() < 1.0e-12);
        assert!(
            (score.linear_gaussian_expected_information_gain_nats
                - 0.5 * expected_d_optimal)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn incomplete_lineage_is_excluded_or_preserved_as_unqualified() {
        let candidates = vec![
            candidate("A", [1.0, 0.0], 1.0, false, 0.0, true),
            candidate("B", [0.0, 1.0], 1.0, false, 0.0, false),
            candidate("C", [1.0, 1.0], 1.0, false, 0.0, true),
        ];
        let strict = enumerate_batch_portfolios(
            &candidates,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            2,
            2.0,
            true,
        )
        .expect("strict lineage search");
        assert_eq!(strict.portfolios.len(), 1);
        assert_eq!(strict.portfolios[0].candidate_ids, vec!["A", "C"]);
        assert_eq!(strict.skipped_incomplete_lineage_portfolios, 2);

        let permissive = enumerate_batch_portfolios(
            &candidates,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            2,
            2.0,
            false,
        )
        .expect("permissive lineage search");
        assert_eq!(permissive.portfolios.len(), 3);
        assert!(permissive
            .portfolios
            .iter()
            .any(|score| !score.lineage_coverage_complete));
    }

    #[test]
    fn infeasible_candidates_and_over_budget_portfolios_are_counted() {
        let mut blocked = candidate("blocked", [1.0, 0.0], 1.0, false, 0.0, true);
        blocked.feasible = false;
        let candidates = vec![
            candidate("A", [1.0, 0.0], 1.0, false, 0.0, true),
            candidate("B", [0.0, 1.0], 1.0, false, 0.0, true),
            blocked,
        ];
        let result = enumerate_batch_portfolios(
            &candidates,
            &[vec![1.0, 0.0], vec![0.0, 1.0]],
            2,
            1.5,
            true,
        )
        .expect("budget search");
        assert_eq!(result.portfolios.len(), 0);
        assert_eq!(result.skipped_infeasible_candidates, 1);
        assert_eq!(result.skipped_over_budget_portfolios, 1);
    }

    #[test]
    fn duplicate_candidate_ids_and_singular_prior_fail_closed() {
        let duplicate = vec![
            candidate("same", [1.0, 0.0], 1.0, false, 0.0, true),
            candidate("same", [0.0, 1.0], 1.0, false, 0.0, true),
        ];
        assert_eq!(
            enumerate_batch_portfolios(&duplicate, &[vec![1.0, 0.0], vec![0.0, 1.0]], 2, 2.0, true),
            Err(ModelError::InvalidParameter(
                "batch_candidate_ids_must_be_nonempty_and_unique"
            ))
        );

        assert_eq!(
            enumerate_batch_portfolios(
                &[candidate("A", [1.0, 0.0], 1.0, false, 0.0, true)],
                &[vec![1.0, 1.0], vec![1.0, 1.0]],
                1,
                1.0,
                true
            ),
            Err(ModelError::InvalidCovariance("batch_prior_covariance"))
        );
    }
}
