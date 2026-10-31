//! Exact fresh-fold sequential conformal rank calibration.
//!
//! This implements the finite-sample order-statistic bookkeeping for
//! sequential calibration under recursive selection. It does not itself
//! establish exchangeability, predictable refitting or distribution
//! stationarity; those are explicit caller/data-lineage assumptions. Each
//! stage must use a distinct fresh fold, and its score fit may only depend on
//! earlier calibration folds.

use crate::Rational;
use std::collections::BTreeSet;

const MAX_CONFORMAL_STAGES: usize = 64;
const MAX_FRESH_FOLD_SAMPLES: usize = 1_000_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshFoldStageInput {
    /// Zero-based stage in the sequential decision/calibration chain.
    pub stage_index: usize,
    /// Immutable identifier for this stage's disjoint fresh calibration fold.
    pub fold_id: String,
    /// Calibration folds used to fit/refit this stage's score function.
    /// These must all precede the current stage.
    pub score_fit_fold_ids: Vec<String>,
    /// Stable identifier for the fitted score function and its parameters.
    pub score_version: String,
    /// Number of units in the fresh fold before applying earlier-stage filters.
    pub fold_sample_count: usize,
    /// Scores for units in this fresh fold that survived all earlier stage
    /// filters. The caller supplies this survivor selection receipt.
    pub survivor_scores: Vec<Rational>,
    /// Target conditional coverage among the survivors entering this stage.
    pub target_conditional_coverage: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshFoldStageReceipt {
    pub stage_index: usize,
    pub fold_id: String,
    pub score_version: String,
    pub score_fit_fold_ids: Vec<String>,
    pub fold_sample_count: usize,
    pub survivor_count: usize,
    pub target_conditional_coverage: Rational,
    /// One-based augmented order-statistic rank ceil((M + 1) * beta).
    pub augmented_rank: usize,
    /// None means the rank is M + 1 and the finite-sample rule is unbounded.
    pub finite_threshold: Option<Rational>,
    /// The product of conditional targets through this stage.
    /// This is the marginal prefix-coverage lower bound under the paper's
    /// assumptions, not a guarantee under arbitrary distribution shift.
    pub prefix_coverage_lower_bound: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshFoldSequentialCalibration {
    pub stage_receipts: Vec<FreshFoldStageReceipt>,
    pub prefix_coverage_lower_bounds: Vec<Rational>,
}

impl FreshFoldStageInput {
    fn validate_shape(&self, expected_stage: usize) -> Result<(), String> {
        if self.stage_index != expected_stage {
            return Err("fresh-fold calibration stages must be contiguous and ordered".into());
        }
        if self.fold_id.trim().is_empty() || self.score_version.trim().is_empty() {
            return Err("fold ID and score version must be non-empty".into());
        }
        if self.fold_sample_count == 0
            || self.fold_sample_count > MAX_FRESH_FOLD_SAMPLES
            || self.survivor_scores.len() > self.fold_sample_count
        {
            return Err("fresh-fold sample count is zero, exceeds its cap, or survivor scores exceed it".into());
        }
        if self.target_conditional_coverage.is_zero()
            || !self.target_conditional_coverage.is_probability()
        {
            return Err("conditional coverage target must lie in (0, 1]".into());
        }
        let mut fit_folds = BTreeSet::new();
        if self
            .score_fit_fold_ids
            .iter()
            .any(|fold_id| fold_id.trim().is_empty() || !fit_folds.insert(fold_id))
        {
            return Err("score-fit fold IDs must be non-empty and unique".into());
        }
        Ok(())
    }
}

/// Compute a fresh-fold sequential conformal calibration plan.
///
/// For stage k, let M_k be the number of survivors in its fresh fold and
/// beta_k the target conditional coverage among previous-stage survivors.
/// The finite-sample augmented rank is r_k = ceil((M_k + 1) * beta_k).
/// If r_k = M_k + 1, the threshold is unbounded (None): the data do not
/// support a finite threshold at the requested target.
///
/// The product of beta values gives the marginal prefix lower bound under
/// the fresh-fold/exchangeability assumptions. It is not valid to reuse
/// adaptive survivors as fresh calibration scores without a separate proof.
pub fn calibrate_fresh_fold_sequence(
    stages: &[FreshFoldStageInput],
) -> Result<FreshFoldSequentialCalibration, String> {
    if stages.is_empty() || stages.len() > MAX_CONFORMAL_STAGES {
        return Err(format!(
            "fresh-fold calibration stage count must be in 1..={MAX_CONFORMAL_STAGES}"
        ));
    }

    let mut fold_positions = BTreeSet::new();
    for (index, stage) in stages.iter().enumerate() {
        stage.validate_shape(index)?;
        if !fold_positions.insert(stage.fold_id.as_str()) {
            return Err("fresh calibration fold IDs must be disjoint across stages".into());
        }
    }

    for (index, stage) in stages.iter().enumerate() {
        for fit_fold in &stage.score_fit_fold_ids {
            let Some(position) = stages
                .iter()
                .position(|candidate| candidate.fold_id == *fit_fold)
            else {
                return Err(
                    "score-fit fold ID is absent from the declared fresh-fold sequence".into(),
                );
            };
            if position >= index {
                return Err(
                    "stage score refit cannot use its current or a future calibration fold"
                        .into(),
                );
            }
        }
    }

    let mut prefix_coverage = Rational::one();
    let mut receipts = Vec::with_capacity(stages.len());
    let mut prefix_bounds = Vec::with_capacity(stages.len());

    for stage in stages {
        prefix_coverage = prefix_coverage.checked_mul(stage.target_conditional_coverage)?;
        let survivor_count = stage.survivor_scores.len();
        let augmented_sample_count = survivor_count
            .checked_add(1)
            .ok_or_else(|| "augmented survivor count overflow".to_string())?;
        let rank = ceil_product(augmented_sample_count, stage.target_conditional_coverage)?;
        let finite_threshold = if rank <= survivor_count {
            let mut ordered_scores = stage.survivor_scores.clone();
            ordered_scores.sort();
            Some(ordered_scores[rank - 1])
        } else {
            None
        };

        let receipt = FreshFoldStageReceipt {
            stage_index: stage.stage_index,
            fold_id: stage.fold_id.clone(),
            score_version: stage.score_version.clone(),
            score_fit_fold_ids: stage.score_fit_fold_ids.clone(),
            fold_sample_count: stage.fold_sample_count,
            survivor_count,
            target_conditional_coverage: stage.target_conditional_coverage,
            augmented_rank: rank,
            finite_threshold,
            prefix_coverage_lower_bound: prefix_coverage,
        };
        prefix_bounds.push(prefix_coverage);
        receipts.push(receipt);
    }

    Ok(FreshFoldSequentialCalibration {
        stage_receipts: receipts,
        prefix_coverage_lower_bounds: prefix_bounds,
    })
}

fn ceil_product(integer: usize, multiplier: Rational) -> Result<usize, String> {
    let integer = u128::try_from(integer)
        .map_err(|_| "calibration sample count does not fit exact rational arithmetic".to_string())?;
    let numerator = integer
        .checked_mul(multiplier.numerator())
        .ok_or_else(|| "conformal rank numerator overflow".to_string())?;
    let denominator = multiplier.denominator();
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let rounded = quotient
        .checked_add(if remainder == 0 { 0 } else { 1 })
        .ok_or_else(|| "conformal rank overflow".to_string())?;
    usize::try_from(rounded).map_err(|_| "conformal rank does not fit usize".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rational(numerator: u128, denominator: u128) -> Rational {
        Rational::new(numerator, denominator).unwrap()
    }

    fn stage(
        index: usize,
        fold_id: &str,
        beta: Rational,
        scores: &[u128],
        fit_folds: &[&str],
    ) -> FreshFoldStageInput {
        FreshFoldStageInput {
            stage_index: index,
            fold_id: fold_id.into(),
            score_fit_fold_ids: fit_folds.iter().map(|id| (*id).into()).collect(),
            score_version: format!("score-v{index}"),
            fold_sample_count: scores.len() + 3,
            survivor_scores: scores
                .iter()
                .map(|score| Rational::new(*score, 1).unwrap())
                .collect(),
            target_conditional_coverage: beta,
        }
    }

    #[test]
    fn fresh_fold_rank_uses_exact_augmented_order_statistic() {
        let calibration = calibrate_fresh_fold_sequence(&[stage(
            0,
            "fold-a",
            rational(3, 4),
            &[4, 1, 3],
            &[],
        )])
        .unwrap();
        let receipt = &calibration.stage_receipts[0];
        assert_eq!(receipt.survivor_count, 3);
        assert_eq!(receipt.augmented_rank, 3);
        assert_eq!(receipt.finite_threshold, Some(Rational::from_u64(3)));
        assert_eq!(receipt.prefix_coverage_lower_bound, rational(3, 4));
    }

    #[test]
    fn insufficient_survivors_produce_an_unbounded_threshold() {
        let calibration = calibrate_fresh_fold_sequence(&[stage(
            0,
            "fold-a",
            rational(9, 10),
            &[1, 2],
            &[],
        )])
        .unwrap();
        let receipt = &calibration.stage_receipts[0];
        assert_eq!(receipt.augmented_rank, 3);
        assert_eq!(receipt.finite_threshold, None);
    }

    #[test]
    fn prefix_coverage_bound_is_the_exact_product_of_stage_targets() {
        let calibration = calibrate_fresh_fold_sequence(&[
            stage(0, "fold-a", rational(9, 10), &[1, 2, 3, 4, 5, 6, 7, 8, 9], &[]),
            stage(1, "fold-b", rational(9, 10), &[1, 2, 3, 4, 5, 6, 7, 8, 9], &["fold-a"]),
        ])
        .unwrap();
        assert_eq!(
            calibration.prefix_coverage_lower_bounds,
            vec![rational(9, 10), rational(81, 100)]
        );
        assert_eq!(calibration.stage_receipts[1].score_fit_fold_ids, vec!["fold-a"]);
    }

    #[test]
    fn ties_use_the_weak_inequality_threshold_and_remain_conservative() {
        let calibration = calibrate_fresh_fold_sequence(&[stage(
            0,
            "fold-a",
            rational(1, 2),
            &[5, 5, 5],
            &[],
        )])
        .unwrap();
        assert_eq!(
            calibration.stage_receipts[0].finite_threshold,
            Some(Rational::from_u64(5))
        );
    }

    #[test]
    fn duplicate_folds_and_adaptive_current_fold_reuse_fail_closed() {
        let duplicate = [
            stage(0, "fold-a", rational(1, 2), &[1, 2, 3], &[]),
            stage(1, "fold-a", rational(1, 2), &[1, 2, 3], &[]),
        ];
        assert!(calibrate_fresh_fold_sequence(&duplicate).is_err());

        let reused_current = [stage(
            0,
            "fold-a",
            rational(1, 2),
            &[1, 2, 3],
            &["fold-a"],
        )];
        assert!(calibrate_fresh_fold_sequence(&reused_current).is_err());
    }

    #[test]
    fn later_score_refit_may_use_earlier_folds_but_not_future_folds() {
        let valid = [
            stage(0, "fold-a", rational(1, 2), &[1, 2, 3], &[]),
            stage(1, "fold-b", rational(1, 2), &[1, 2, 3], &["fold-a"]),
        ];
        assert!(calibrate_fresh_fold_sequence(&valid).is_ok());

        let future_reuse = [
            stage(0, "fold-a", rational(1, 2), &[1, 2, 3], &["fold-b"]),
            stage(1, "fold-b", rational(1, 2), &[1, 2, 3], &[]),
        ];
        assert!(calibrate_fresh_fold_sequence(&future_reuse).is_err());

        let undeclared_reuse = [stage(
            0,
            "fold-a",
            rational(1, 2),
            &[1, 2, 3],
            &["unknown-fold"],
        )];
        assert!(calibrate_fresh_fold_sequence(&undeclared_reuse).is_err());
    }

    #[test]
    fn rank_and_fold_contract_reject_invalid_targets_and_survivor_counts() {
        let mut invalid_target = stage(0, "fold-a", Rational::zero(), &[1], &[]);
        assert!(calibrate_fresh_fold_sequence(&[invalid_target.clone()]).is_err());
        invalid_target.target_conditional_coverage = rational(1, 2);
        invalid_target.fold_sample_count = 0;
        assert!(calibrate_fresh_fold_sequence(&[invalid_target]).is_err());
    }
}
