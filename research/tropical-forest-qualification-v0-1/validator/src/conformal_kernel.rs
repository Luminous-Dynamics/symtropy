// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Block-aware split conformal calibration for bounded synthetic forest cases.
//!
//! The calibration unit is a declared spatial/campaign block. The block score is the
//! maximum absolute residual within an equal-sized block, so a calibrated radius can be
//! evaluated for all observations in a held-out block. This gives a finite-sample
//! marginal statement only if calibration and test blocks are exchangeable. The module
//! checks identity separation but cannot establish that statistical assumption.

use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConformalError {
    InvalidMiscoverage,
    InvalidTestRegime,
    EmptyCalibration,
    EmptyBlockIdentity,
    DuplicateBlockIdentity,
    DuplicateGroupIdentity,
    EmptyTrainingGroupIdentity,
    DuplicateTrainingGroupIdentity,
    GroupLeakage,
    UnequalBlockSize,
    EmptyBlockObservations,
    NonFiniteOrNegativeResidual,
    NonFinitePrediction,
    NumericOverflow,
}

impl fmt::Display for ConformalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ConformalError {}

#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationBlock {
    pub block_id: String,
    pub group_id: String,
    /// Absolute out-of-sample residuals for this block.
    pub absolute_residuals: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TestBlock {
    pub block_id: String,
    pub group_id: String,
    pub regime: String,
    pub point_predictions: Vec<f64>,
    pub observations: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockConformalScenario {
    pub alpha: f64,
    pub training_group_ids: Vec<String>,
    pub calibration_blocks: Vec<CalibrationBlock>,
    pub test_blocks: Vec<TestBlock>,
    /// A statement supplied by the experiment designer, not a verified statistical fact.
    pub exchangeability_basis: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockCoverage {
    pub block_id: String,
    pub observation_count: usize,
    pub covered_observation_count: usize,
    pub observation_coverage_fraction: f64,
    pub block_fully_covered: bool,
    pub regime_is_declared_shift: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockConformalResult {
    pub calibration_block_count: usize,
    pub observations_per_block: usize,
    pub pooled_observation_radius: Option<f64>,
    pub block_maximum_radius: Option<f64>,
    pub pooled_interval_width: Option<f64>,
    pub block_interval_width: Option<f64>,
    pub test_block_count: usize,
    pub test_observation_count: usize,
    pub pooled_observation_coverage: Option<f64>,
    pub pooled_full_block_coverage: Option<f64>,
    pub block_observation_coverage: Option<f64>,
    pub block_full_block_coverage: Option<f64>,
    pub declared_in_domain_block_coverage: Option<f64>,
    pub declared_shift_block_coverage: Option<f64>,
    pub test_block_results: Vec<BlockCoverage>,
    pub exchangeability_status: &'static str,
}

/// Finite-sample split-conformal order statistic.
///
/// Returns None (an unbounded interval) if ceil((n+1)(1-alpha)) > n.
pub fn split_conformal_radius(
    nonconformity_scores: &[f64],
    alpha: f64,
) -> Result<Option<f64>, ConformalError> {
    if !alpha.is_finite() || !(0.0..1.0).contains(&alpha) {
        return Err(ConformalError::InvalidMiscoverage);
    }
    if nonconformity_scores.is_empty() || nonconformity_scores.len() > 1_000_000 {
        return Err(ConformalError::EmptyCalibration);
    }
    if nonconformity_scores
        .iter()
        .any(|score| !score.is_finite() || *score < 0.0)
    {
        return Err(ConformalError::NonFiniteOrNegativeResidual);
    }

    let count = u32::try_from(nonconformity_scores.len())
        .map_err(|_| ConformalError::NumericOverflow)?;
    let rank = (f64::from(count) + 1.0) * (1.0 - alpha);
    let required_rank = rank.ceil();
    if required_rank > f64::from(count) {
        return Ok(None);
    }

    let mut sorted = nonconformity_scores.to_vec();
    sorted.sort_by(f64::total_cmp);
    for (index, score) in sorted.iter().enumerate() {
        let one_based = u32::try_from(index + 1).map_err(|_| ConformalError::NumericOverflow)?;
        if f64::from(one_based) >= required_rank {
            return Ok(Some(*score));
        }
    }
    Err(ConformalError::NumericOverflow)
}

fn validate_identity(
    block_id: &str,
    group_id: &str,
    seen_blocks: &mut BTreeSet<String>,
    seen_groups: &mut BTreeSet<String>,
) -> Result<(), ConformalError> {
    if block_id.trim().is_empty() || group_id.trim().is_empty() {
        return Err(ConformalError::EmptyBlockIdentity);
    }
    if !seen_blocks.insert(block_id.to_string()) {
        return Err(ConformalError::DuplicateBlockIdentity);
    }
    if !seen_groups.insert(group_id.to_string()) {
        return Err(ConformalError::DuplicateGroupIdentity);
    }
    Ok(())
}

fn checked_count(count: usize) -> Result<f64, ConformalError> {
    let count = u32::try_from(count).map_err(|_| ConformalError::NumericOverflow)?;
    Ok(f64::from(count))
}

fn coverage_for_radius(
    predictions: &[f64],
    observations: &[f64],
    radius: Option<f64>,
) -> Result<(usize, bool), ConformalError> {
    if predictions.is_empty() || predictions.len() != observations.len() {
        return Err(ConformalError::UnequalBlockSize);
    }
    let Some(radius) = radius else {
        return Ok((observations.len(), true));
    };
    let mut covered = 0usize;
    for (prediction, observation) in predictions.iter().zip(observations) {
        if !prediction.is_finite() || !observation.is_finite() {
            return Err(ConformalError::NonFinitePrediction);
        }
        if (prediction - observation).abs() <= radius {
            covered += 1;
        }
    }
    Ok((covered, covered == observations.len()))
}

pub fn evaluate_block_conformal(
    scenario: &BlockConformalScenario,
) -> Result<BlockConformalResult, ConformalError> {
    if scenario.exchangeability_basis.trim().is_empty() {
        return Err(ConformalError::InvalidMiscoverage);
    }
    if scenario.calibration_blocks.is_empty() {
        return Err(ConformalError::EmptyCalibration);
    }

    let mut training_groups = BTreeSet::new();
    for group_id in &scenario.training_group_ids {
        if group_id.trim().is_empty() {
            return Err(ConformalError::EmptyTrainingGroupIdentity);
        }
        if !training_groups.insert(group_id.as_str()) {
            return Err(ConformalError::DuplicateTrainingGroupIdentity);
        }
    }

    let mut calibration_block_ids = BTreeSet::new();
    let mut calibration_groups = BTreeSet::new();
    let mut block_size = None;
    let mut block_scores = Vec::with_capacity(scenario.calibration_blocks.len());
    let mut pooled_scores = Vec::new();

    for block in &scenario.calibration_blocks {
        validate_identity(
            &block.block_id,
            &block.group_id,
            &mut calibration_block_ids,
            &mut calibration_groups,
        )?;
        if training_groups.contains(block.group_id.as_str()) {
            return Err(ConformalError::GroupLeakage);
        }
        if block.absolute_residuals.is_empty() {
            return Err(ConformalError::EmptyBlockObservations);
        }
        match block_size {
            Some(expected) if expected != block.absolute_residuals.len() => {
                return Err(ConformalError::UnequalBlockSize);
            }
            None => block_size = Some(block.absolute_residuals.len()),
            _ => {}
        }
        let mut maximum = 0.0_f64;
        for residual in &block.absolute_residuals {
            if !residual.is_finite() || *residual < 0.0 {
                return Err(ConformalError::NonFiniteOrNegativeResidual);
            }
            maximum = maximum.max(*residual);
            pooled_scores.push(*residual);
        }
        block_scores.push(maximum);
    }

    let mut test_block_ids = BTreeSet::new();
    let mut test_groups = BTreeSet::new();
    let expected_test_size = block_size.ok_or(ConformalError::EmptyCalibration)?;
    for block in &scenario.test_blocks {
        validate_identity(
            &block.block_id,
            &block.group_id,
            &mut test_block_ids,
            &mut test_groups,
        )?;
        if training_groups.contains(block.group_id.as_str())
            || calibration_groups.contains(block.group_id.as_str())
        {
            return Err(ConformalError::GroupLeakage);
        }
        if block.regime != "DECLARED_IN_DOMAIN"
            && block.regime != "KNOWN_SHIFT_NEGATIVE_CONTROL"
        {
            return Err(ConformalError::InvalidTestRegime);
        }
        if block.point_predictions.len() != expected_test_size
            || block.observations.len() != expected_test_size
            || block.point_predictions.is_empty()
        {
            return Err(ConformalError::UnequalBlockSize);
        }
        if block
            .point_predictions
            .iter()
            .chain(&block.observations)
            .any(|value| !value.is_finite())
        {
            return Err(ConformalError::NonFinitePrediction);
        }
    }

    let pooled_observation_radius = split_conformal_radius(&pooled_scores, scenario.alpha)?;
    let block_maximum_radius = split_conformal_radius(&block_scores, scenario.alpha)?;
    let mut pooled_observation_covered = 0usize;
    let mut block_observation_covered = 0usize;
    let mut pooled_fully_covered_blocks = 0usize;
    let mut block_fully_covered_blocks = 0usize;
    let mut in_domain_covered_blocks = 0usize;
    let mut in_domain_block_count = 0usize;
    let mut shifted_covered_blocks = 0usize;
    let mut shifted_block_count = 0usize;
    let mut test_observation_count = 0usize;
    let mut test_block_results = Vec::with_capacity(scenario.test_blocks.len());

    for block in &scenario.test_blocks {
        let (pooled_covered, pooled_full) = coverage_for_radius(
            &block.point_predictions,
            &block.observations,
            pooled_observation_radius,
        )?;
        let (block_covered, block_full) = coverage_for_radius(
            &block.point_predictions,
            &block.observations,
            block_maximum_radius,
        )?;
        pooled_observation_covered += pooled_covered;
        block_observation_covered += block_covered;
        pooled_fully_covered_blocks += if pooled_full { 1 } else { 0 };
        block_fully_covered_blocks += if block_full { 1 } else { 0 };
        test_observation_count += block.observations.len();
        let is_shifted = block.regime == "KNOWN_SHIFT_NEGATIVE_CONTROL";
        if is_shifted {
            shifted_block_count += 1;
            shifted_covered_blocks += if block_full { 1 } else { 0 };
        } else {
            in_domain_block_count += 1;
            in_domain_covered_blocks += if block_full { 1 } else { 0 };
        }
        test_block_results.push(BlockCoverage {
            block_id: block.block_id.clone(),
            observation_count: block.observations.len(),
            covered_observation_count: block_covered,
            observation_coverage_fraction: checked_count(block_covered)?
                / checked_count(block.observations.len())?,
            block_fully_covered: block_full,
            regime_is_declared_shift: is_shifted,
        });
    }

    let test_block_count = scenario.test_blocks.len();
    let pooled_interval_width = pooled_observation_radius.map(|radius| 2.0 * radius);
    let block_interval_width = block_maximum_radius.map(|radius| 2.0 * radius);
    let pooled_observation_coverage = if test_observation_count == 0 {
        None
    } else {
        Some(checked_count(pooled_observation_covered)? / checked_count(test_observation_count)?)
    };
    let block_observation_coverage = if test_observation_count == 0 {
        None
    } else {
        Some(checked_count(block_observation_covered)? / checked_count(test_observation_count)?)
    };
    let pooled_full_block_coverage = if test_block_count == 0 {
        None
    } else {
        Some(checked_count(pooled_fully_covered_blocks)? / checked_count(test_block_count)?)
    };
    let block_full_block_coverage = if test_block_count == 0 {
        None
    } else {
        Some(checked_count(block_fully_covered_blocks)? / checked_count(test_block_count)?)
    };
    let declared_in_domain_block_coverage = if in_domain_block_count == 0 {
        None
    } else {
        Some(checked_count(in_domain_covered_blocks)? / checked_count(in_domain_block_count)?)
    };
    let declared_shift_block_coverage = if shifted_block_count == 0 {
        None
    } else {
        Some(checked_count(shifted_covered_blocks)? / checked_count(shifted_block_count)?)
    };

    Ok(BlockConformalResult {
        calibration_block_count: scenario.calibration_blocks.len(),
        observations_per_block: expected_test_size,
        pooled_observation_radius,
        block_maximum_radius,
        pooled_interval_width,
        block_interval_width,
        test_block_count,
        test_observation_count,
        pooled_observation_coverage,
        pooled_full_block_coverage,
        block_observation_coverage,
        block_full_block_coverage,
        declared_in_domain_block_coverage,
        declared_shift_block_coverage,
        test_block_results,
        exchangeability_status: "DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocks() -> Vec<CalibrationBlock> {
        (1..=9)
            .map(|block| {
                let mut residuals = vec![0.1; 9];
                residuals.push(f64::from(block));
                CalibrationBlock {
                    block_id: format!("cal-block-{block}"),
                    group_id: format!("cal-group-{block}"),
                    absolute_residuals: residuals,
                }
            })
            .collect()
    }

    fn scenario() -> BlockConformalScenario {
        let mut in_domain_a = vec![100.5; 9];
        in_domain_a.push(108.0);
        let mut in_domain_b = vec![100.5; 9];
        in_domain_b.push(109.5);
        let mut shifted = vec![100.5; 9];
        shifted.push(110.0);
        let test_block = |id: &str, regime: &str, observations: Vec<f64>| TestBlock {
            block_id: id.to_string(),
            group_id: format!("test-group-{id}"),
            regime: regime.to_string(),
            point_predictions: vec![100.0; 10],
            observations,
        };
        BlockConformalScenario {
            alpha: 0.1,
            training_group_ids: vec!["train-region-1".to_string()],
            calibration_blocks: blocks(),
            test_blocks: vec![
                test_block("in-domain-a", "DECLARED_IN_DOMAIN", in_domain_a),
                test_block("in-domain-b", "DECLARED_IN_DOMAIN", in_domain_b),
                test_block("shifted-c", "KNOWN_SHIFT_NEGATIVE_CONTROL", shifted),
            ],
            exchangeability_basis: "SYNTHETIC_EQUAL_SIZED_BLOCKS_DECLARED_EXCHANGEABLE".to_string(),
        }
    }

    #[test]
    fn split_conformal_rank_uses_n_plus_one_order_statistic() {
        let result = split_conformal_radius(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 0.1)
            .expect("finite sample radius");
        assert_eq!(result, Some(9.0));
    }

    #[test]
    fn insufficient_calibration_blocks_return_unbounded_radius() {
        let result = split_conformal_radius(&[1.0, 2.0, 3.0, 4.0], 0.1)
            .expect("insufficient calibration has a defined unbounded result");
        assert_eq!(result, None);
    }

    #[test]
    fn zero_miscoverage_and_nonfinite_miscoverage_are_rejected() {
        assert_eq!(split_conformal_radius(&[1.0], 0.0), Err(ConformalError::InvalidMiscoverage));
        assert_eq!(split_conformal_radius(&[1.0], f64::NAN), Err(ConformalError::InvalidMiscoverage));
    }

    #[test]
    fn block_maximum_radius_is_wider_than_pooled_pixel_radius_under_clustered_residuals() {
        let result = evaluate_block_conformal(&scenario()).expect("block conformal");
        assert_eq!(result.pooled_observation_radius, Some(1.0));
        assert_eq!(result.block_maximum_radius, Some(9.0));
        assert_eq!(result.pooled_interval_width, Some(2.0));
        assert_eq!(result.block_interval_width, Some(18.0));
    }

    #[test]
    fn observation_coverage_and_whole_block_coverage_are_distinct() {
        let result = evaluate_block_conformal(&scenario()).expect("block conformal");
        assert_eq!(result.block_observation_coverage, Some(28.0 / 30.0));
        assert_eq!(result.block_full_block_coverage, Some(1.0 / 3.0));
        assert_eq!(result.pooled_observation_coverage, Some(27.0 / 30.0));
        assert_eq!(result.pooled_full_block_coverage, Some(0.0));
    }

    #[test]
    fn declared_shift_failure_is_reported_separately() {
        let result = evaluate_block_conformal(&scenario()).expect("block conformal");
        assert_eq!(result.declared_in_domain_block_coverage, Some(1.0 / 2.0));
        assert_eq!(result.declared_shift_block_coverage, Some(0.0));
        assert_eq!(result.exchangeability_status, "DESIGNER_DECLARED_NOT_STATISTICALLY_VERIFIED");
    }

    #[test]
    fn calibration_and_test_groups_must_be_disjoint_from_training_and_each_other() {
        let mut value = scenario();
        value.test_blocks[0].group_id = value.calibration_blocks[0].group_id.clone();
        assert_eq!(evaluate_block_conformal(&value), Err(ConformalError::GroupLeakage));
    }


    #[test]
    fn calibration_groups_must_be_disjoint_from_training_groups() {
        let mut value = scenario();
        value.calibration_blocks[0].group_id = "train-region-1".to_string();
        assert_eq!(evaluate_block_conformal(&value), Err(ConformalError::GroupLeakage));
    }

    #[test]
    fn unknown_test_regime_is_rejected_not_treated_as_in_domain() {
        let mut value = scenario();
        value.test_blocks[0].regime = "MAYBE_SHIFTED".to_string();
        assert_eq!(evaluate_block_conformal(&value), Err(ConformalError::InvalidTestRegime));
    }

    #[test]
    fn ninety_five_percent_target_needs_at_least_nineteen_calibration_blocks() {
        let scores: Vec<f64> = (1..=19).map(f64::from).collect();
        assert_eq!(split_conformal_radius(&scores, 0.05), Ok(Some(19.0)));
        assert_eq!(split_conformal_radius(&scores[..18], 0.05), Ok(None));
    }

    #[test]
    fn calibration_blocks_must_have_equal_size() {
        let mut value = scenario();
        value.calibration_blocks[0].absolute_residuals.pop();
        assert_eq!(evaluate_block_conformal(&value), Err(ConformalError::UnequalBlockSize));
    }

    #[test]
    fn invalid_residual_or_test_prediction_is_rejected() {
        let mut bad_residual = scenario();
        bad_residual.calibration_blocks[0].absolute_residuals[0] = f64::NAN;
        assert_eq!(
            evaluate_block_conformal(&bad_residual),
            Err(ConformalError::NonFiniteOrNegativeResidual)
        );
        let mut bad_prediction = scenario();
        bad_prediction.test_blocks[0].point_predictions[0] = f64::INFINITY;
        assert_eq!(
            evaluate_block_conformal(&bad_prediction),
            Err(ConformalError::NonFinitePrediction)
        );
    }

    #[test]
    fn permuting_calibration_block_order_does_not_change_radii_or_coverage() {
        let mut left = scenario();
        let mut right = scenario();
        right.calibration_blocks.reverse();
        let a = evaluate_block_conformal(&left).expect("left");
        let b = evaluate_block_conformal(&right).expect("right");
        assert_eq!(a.pooled_observation_radius, b.pooled_observation_radius);
        assert_eq!(a.block_maximum_radius, b.block_maximum_radius);
        assert_eq!(a.block_full_block_coverage, b.block_full_block_coverage);
        left.test_blocks.reverse();
        let c = evaluate_block_conformal(&left).expect("reordered tests");
        assert_eq!(a.block_maximum_radius, c.block_maximum_radius);
    }
}
