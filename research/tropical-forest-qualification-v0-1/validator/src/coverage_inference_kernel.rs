// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Finite-sample uncertainty for realized coverage across declared independent blocks.
//!
//! This module does not estimate spatial/temporal dependence or prove independence.
//! Its Hoeffding interval is only justified when the evaluation block outcomes are
//! independent under the study design. The effective sample size is the number of
//! block-level coverage indicators, never the number of stems or pixels in those blocks.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CoverageInferenceError {
    EmptyEvaluation,
    CoveredBlocksExceedTotal,
    InvalidConfidenceLevel,
    TooManyBlocks,
    NumericOverflow,
}

impl fmt::Display for CoverageInferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CoverageInferenceError {}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockCoverageInterval {
    pub covered_blocks: usize,
    pub total_blocks: usize,
    pub observed_block_coverage: f64,
    pub confidence_level: f64,
    pub hoeffding_radius: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    /// A design declaration only. Distinct identifiers do not establish independence.
    pub independence_status: &'static str,
    pub interval_method: &'static str,
}

/// Compute a two-sided Hoeffding interval for the mean of block-coverage indicators.
///
/// If X_b is 1 when held-out block b is fully covered and 0 otherwise, this bounds
/// the average of E[X_b] across the evaluated block population with confidence
/// `confidence_level`, provided the block indicators are independent. Identical
/// distributions are not required for the bound on the average mean, but a common
/// coverage-probability interpretation does require an appropriate sampling design.
///
/// The radius is sqrt(ln(2 / delta) / (2 n)), where delta = 1 - confidence_level.
/// Bounds are clipped to [0, 1]. The result labels independence as declared, not
/// statistically verified. If blocks remain spatially or temporally dependent, callers
/// must not present this interval as dependence-corrected inference.
///
/// Pass block-level counts. Passing stem- or pixel-level counts for correlated records
/// is pseudo-replication and invalidates the independent-block interpretation.
pub fn hoeffding_block_coverage_interval(
    covered_blocks: usize,
    total_blocks: usize,
    confidence_level: f64,
) -> Result<BlockCoverageInterval, CoverageInferenceError> {
    if total_blocks == 0 {
        return Err(CoverageInferenceError::EmptyEvaluation);
    }
    if covered_blocks > total_blocks {
        return Err(CoverageInferenceError::CoveredBlocksExceedTotal);
    }
    if !confidence_level.is_finite() || confidence_level <= 0.0 || confidence_level >= 1.0 {
        return Err(CoverageInferenceError::InvalidConfidenceLevel);
    }

    let count = u32::try_from(total_blocks).map_err(|_| CoverageInferenceError::TooManyBlocks)?;
    let covered = u32::try_from(covered_blocks)
        .map_err(|_| CoverageInferenceError::TooManyBlocks)?;
    let n = f64::from(count);
    let point = f64::from(covered) / n;
    let delta = 1.0 - confidence_level;
    let radius = ((2.0 / delta).ln() / (2.0 * n)).sqrt();
    if !point.is_finite() || !radius.is_finite() {
        return Err(CoverageInferenceError::NumericOverflow);
    }

    Ok(BlockCoverageInterval {
        covered_blocks,
        total_blocks,
        observed_block_coverage: point,
        confidence_level,
        hoeffding_radius: radius,
        lower_bound: (point - radius).max(0.0),
        upper_bound: (point + radius).min(1.0),
        independence_status: "INDEPENDENT_BLOCK_ASSUMPTION_DECLARED_NOT_VERIFIED",
        interval_method: "TWO_SIDED_HOEFFDING",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOLERANCE: f64 = 1e-12;

    #[test]
    fn three_test_blocks_produce_an_uninformative_95_percent_interval() {
        let result = hoeffding_block_coverage_interval(1, 3, 0.95)
            .expect("three declared independent block outcomes");
        assert!((result.observed_block_coverage - (1.0 / 3.0)).abs() < TOLERANCE);
        assert!((result.hoeffding_radius - 0.784_100_275_699_685_3).abs() < TOLERANCE);
        assert_eq!(result.lower_bound, 0.0);
        assert_eq!(result.upper_bound, 1.0);
    }

    #[test]
    fn block_count_and_observation_count_are_not_interchangeable() {
        let block_result = hoeffding_block_coverage_interval(1, 3, 0.95)
            .expect("three block outcomes");
        let observation_level_control = hoeffding_block_coverage_interval(28, 30, 0.95)
            .expect("deliberate pseudo-replication control");
        assert_eq!(block_result.total_blocks, 3);
        assert_eq!(observation_level_control.total_blocks, 30);
        assert!(block_result.hoeffding_radius > observation_level_control.hoeffding_radius);
        assert_eq!(
            block_result.independence_status,
            "INDEPENDENT_BLOCK_ASSUMPTION_DECLARED_NOT_VERIFIED"
        );
    }

    #[test]
    fn zero_successes_and_all_successes_keep_bounds_inside_unit_interval() {
        let zero = hoeffding_block_coverage_interval(0, 3, 0.95).expect("zero successes");
        let all = hoeffding_block_coverage_interval(3, 3, 0.95).expect("all successes");
        assert_eq!(zero.lower_bound, 0.0);
        assert!(zero.upper_bound > 0.0 && zero.upper_bound < 1.0);
        assert!(all.lower_bound > 0.0 && all.lower_bound < 1.0);
        assert_eq!(all.upper_bound, 1.0);
    }

    #[test]
    fn conservative_radius_scales_with_independent_block_count() {
        let twenty = hoeffding_block_coverage_interval(18, 20, 0.95).expect("twenty blocks");
        let one_hundred =
            hoeffding_block_coverage_interval(90, 100, 0.95).expect("one hundred blocks");
        let seven_hundred_thirty_eight =
            hoeffding_block_coverage_interval(702, 738, 0.95).expect("738 blocks");
        assert!(twenty.hoeffding_radius > one_hundred.hoeffding_radius);
        assert!(one_hundred.hoeffding_radius > seven_hundred_thirty_eight.hoeffding_radius);
        assert!(seven_hundred_thirty_eight.hoeffding_radius <= 0.05);
    }

    #[test]
    fn empty_or_inconsistent_block_counts_are_rejected() {
        assert_eq!(
            hoeffding_block_coverage_interval(0, 0, 0.95),
            Err(CoverageInferenceError::EmptyEvaluation)
        );
        assert_eq!(
            hoeffding_block_coverage_interval(4, 3, 0.95),
            Err(CoverageInferenceError::CoveredBlocksExceedTotal)
        );
    }

    #[test]
    fn confidence_level_must_be_finite_and_strictly_between_zero_and_one() {
        for invalid in [0.0, 1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                hoeffding_block_coverage_interval(1, 3, invalid),
                Err(CoverageInferenceError::InvalidConfidenceLevel)
            );
        }
    }

    #[test]
    fn confidence_interval_is_monotone_in_confidence_level() {
        let ninety = hoeffding_block_coverage_interval(90, 100, 0.90)
            .expect("90 percent confidence");
        let ninety_five = hoeffding_block_coverage_interval(90, 100, 0.95)
            .expect("95 percent confidence");
        assert!(ninety_five.hoeffding_radius > ninety.hoeffding_radius);
        assert!(ninety_five.lower_bound <= ninety.lower_bound);
        assert!(ninety_five.upper_bound >= ninety.upper_bound);
    }
}
