// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Prequential adaptive conformal replay for simultaneous forecast blocks.
//!
//! This is a bounded ACI-style comparator. It uses a frozen calibration score
//! pool and updates the miscoverage level only after each complete evaluation
//! block has been observed. Every forecast must be issued after the preceding
//! evaluation block has ended. The block score is the same max normalized
//! absolute error used by the simultaneous split-conformal evaluator.
//!
//! The lower/upper miscoverage bounds are caller supplied. When projection
//! onto either bound occurs, the unprojected ACI long-run guarantee does not
//! transfer to this bounded replay variant. This module is a deterministic
//! comparator, not FACI, SAOCP, conformal PID, DRACP, PACE or a martingale
//! PAC-Bayes certificate.

use core::fmt;

use serde::Serialize;

use crate::{
    ForecastCalibrationBlock, ForecastCalibrationEvaluation,
};

const PPM_DENOMINATOR: u128 = 1_000_000;

/// Bounded adaptive conformal inference parameters.
///
/// All rates are parts per million. A step size of 10,000 ppm corresponds to
/// gamma = 0.01. The target miscoverage comes from the frozen base contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OnlineAdaptiveCalibrationContract {
    initial_miscoverage_ppm: u32,
    step_size_ppm: u32,
    minimum_miscoverage_ppm: u32,
    maximum_miscoverage_ppm: u32,
}

impl OnlineAdaptiveCalibrationContract {
    pub fn new(
        initial_miscoverage_ppm: u32,
        step_size_ppm: u32,
        minimum_miscoverage_ppm: u32,
        maximum_miscoverage_ppm: u32,
    ) -> Result<Self, OnlineAdaptiveCalibrationError> {
        for (name, value) in [
            ("initial miscoverage", initial_miscoverage_ppm),
            ("minimum miscoverage", minimum_miscoverage_ppm),
            ("maximum miscoverage", maximum_miscoverage_ppm),
        ] {
            if value == 0 || value >= PPM_DENOMINATOR as u32 {
                return Err(OnlineAdaptiveCalibrationError::InvalidMiscoverage {
                    name,
                    value,
                });
            }
        }
        if step_size_ppm == 0 || step_size_ppm > PPM_DENOMINATOR as u32 {
            return Err(OnlineAdaptiveCalibrationError::InvalidStepSize(step_size_ppm));
        }
        if minimum_miscoverage_ppm > maximum_miscoverage_ppm
            || initial_miscoverage_ppm < minimum_miscoverage_ppm
            || initial_miscoverage_ppm > maximum_miscoverage_ppm
        {
            return Err(OnlineAdaptiveCalibrationError::InvalidMiscoverageBounds {
                initial: initial_miscoverage_ppm,
                minimum: minimum_miscoverage_ppm,
                maximum: maximum_miscoverage_ppm,
            });
        }
        Ok(Self {
            initial_miscoverage_ppm,
            step_size_ppm,
            minimum_miscoverage_ppm,
            maximum_miscoverage_ppm,
        })
    }

    pub const fn initial_miscoverage_ppm(&self) -> u32 {
        self.initial_miscoverage_ppm
    }

    pub const fn step_size_ppm(&self) -> u32 {
        self.step_size_ppm
    }

    pub const fn minimum_miscoverage_ppm(&self) -> u32 {
        self.minimum_miscoverage_ppm
    }

    pub const fn maximum_miscoverage_ppm(&self) -> u32 {
        self.maximum_miscoverage_ppm
    }
}

/// One adaptive interval and its source evidence references.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OnlineAdaptiveCellReceipt {
    pub channel_id: String,
    pub horizon_index: usize,
    pub point_forecast: f64,
    pub realized_value: f64,
    pub normalization_scale: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub normalized_absolute_error: f64,
    pub covered: bool,
    pub normalization_scale_evidence_id: String,
    pub forecast_evidence_id: String,
    pub realization_evidence_id: String,
}

/// One prequential forecast-block decision and subsequent feedback update.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OnlineAdaptiveBlockReceipt {
    pub block_id: String,
    pub issued_at_ns: u64,
    pub valid_start_ns: u64,
    pub valid_end_ns: u64,
    pub miscoverage_before_ppm: u32,
    pub selected_order_statistic_rank: usize,
    pub calibration_block_count: usize,
    pub calibrated_scale_multiplier: f64,
    pub maximum_normalized_absolute_error: f64,
    pub joint_covered: bool,
    pub covered_cell_count: usize,
    pub total_cell_count: usize,
    pub mean_interval_width: f64,
    pub mean_normalized_absolute_error: f64,
    pub miscoverage_after_ppm: u32,
    pub clamped_to_minimum: bool,
    pub clamped_to_maximum: bool,
    pub cells: Vec<OnlineAdaptiveCellReceipt>,
}

/// Prequential replay report for a single bounded ACI expert.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OnlineAdaptiveCalibrationReport {
    pub benchmark_id: String,
    pub forecast_model_digest: String,
    pub calibration_set_digest: String,
    pub evaluation_set_digest: String,
    pub metric_convention_id: String,
    pub coverage_assumption: String,
    pub target_joint_coverage_ppm: u32,
    pub target_miscoverage_ppm: u32,
    pub initial_miscoverage_ppm: u32,
    pub final_miscoverage_ppm: u32,
    pub step_size_ppm: u32,
    pub minimum_miscoverage_ppm: u32,
    pub maximum_miscoverage_ppm: u32,
    pub calibration_block_count: usize,
    pub evaluation_block_count: usize,
    pub joint_covered_block_count: usize,
    pub joint_coverage_fraction: f64,
    pub cell_covered_count: usize,
    pub total_cell_count: usize,
    pub cell_coverage_fraction: f64,
    pub mean_interval_width: f64,
    pub maximum_interval_width: f64,
    pub mean_normalized_absolute_error: f64,
    pub minimum_bound_hit_count: usize,
    pub maximum_bound_hit_count: usize,
    pub empirical_miscoverage_ppm: u32,
    pub receipts: Vec<OnlineAdaptiveBlockReceipt>,
}

/// Invalid bounded-ACI settings, temporal leakage or invalid arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnlineAdaptiveCalibrationError {
    InvalidMiscoverage {
        name: &'static str,
        value: u32,
    },
    InvalidStepSize(u32),
    InvalidMiscoverageBounds {
        initial: u32,
        minimum: u32,
        maximum: u32,
    },
    TargetMiscoverageOutsideBounds {
        target: u32,
        minimum: u32,
        maximum: u32,
    },
    FiniteQuantileUnavailable {
        calibration_blocks: usize,
        minimum_miscoverage_ppm: u32,
        required_minimum_miscoverage_ppm: u32,
    },
    EvaluationForecastIssuedBeforePreviousOutcome {
        block_id: String,
        issued_at_ns: u64,
        previous_outcome_available_at_ns: u64,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for OnlineAdaptiveCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMiscoverage { name, value } => {
                write!(f, "{name} must be in 1..1,000,000 ppm; got {value}")
            }
            Self::InvalidStepSize(value) => {
                write!(f, "ACI step size must be in 1..=1,000,000 ppm; got {value}")
            }
            Self::InvalidMiscoverageBounds {
                initial,
                minimum,
                maximum,
            } => write!(
                f,
                "initial miscoverage {initial} must lie within [{minimum}, {maximum}]"
            ),
            Self::TargetMiscoverageOutsideBounds {
                target,
                minimum,
                maximum,
            } => write!(
                f,
                "target miscoverage {target} is outside adaptive bounds [{minimum}, {maximum}]"
            ),
            Self::FiniteQuantileUnavailable {
                calibration_blocks,
                minimum_miscoverage_ppm,
                required_minimum_miscoverage_ppm,
            } => write!(
                f,
                "{calibration_blocks} calibration blocks require minimum miscoverage of at least {required_minimum_miscoverage_ppm} ppm for a finite quantile; configured {minimum_miscoverage_ppm} ppm"
            ),
            Self::EvaluationForecastIssuedBeforePreviousOutcome {
                block_id,
                issued_at_ns,
                previous_outcome_available_at_ns,
            } => write!(
                f,
                "evaluation forecast {block_id} was issued at {issued_at_ns} before the preceding outcome was available at {previous_outcome_available_at_ns}"
            ),
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite or overflowing arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for OnlineAdaptiveCalibrationError {}

/// Replay one bounded ACI expert over the held-out forecast stream.
///
/// The forecast decision uses only the frozen calibration scores and current
/// adaptive miscoverage level. The realized block is used only after its valid
/// window ends to update the next block's level. Held-out scores are not added
/// to the quantile pool, keeping the adaptive effect attributable to alpha
/// feedback.
pub fn evaluate_online_adaptive_calibration(
    evaluation: &ForecastCalibrationEvaluation,
    adaptive_contract: &OnlineAdaptiveCalibrationContract,
) -> Result<OnlineAdaptiveCalibrationReport, OnlineAdaptiveCalibrationError> {
    let base_contract = evaluation.contract();
    let target_miscoverage_ppm = 1_000_000_u32 - base_contract.target_joint_coverage_ppm();
    if target_miscoverage_ppm < adaptive_contract.minimum_miscoverage_ppm
        || target_miscoverage_ppm > adaptive_contract.maximum_miscoverage_ppm
    {
        return Err(OnlineAdaptiveCalibrationError::TargetMiscoverageOutsideBounds {
            target: target_miscoverage_ppm,
            minimum: adaptive_contract.minimum_miscoverage_ppm,
            maximum: adaptive_contract.maximum_miscoverage_ppm,
        });
    }

    let calibration_scores = evaluation
        .calibration_blocks()
        .iter()
        .map(block_max_score)
        .collect::<Result<Vec<_>, _>>()?;
    let calibration_block_count = calibration_scores.len();
    let required_minimum_miscoverage_ppm =
        minimum_finite_miscoverage_ppm(calibration_block_count)?;
    if adaptive_contract.minimum_miscoverage_ppm < required_minimum_miscoverage_ppm {
        return Err(OnlineAdaptiveCalibrationError::FiniteQuantileUnavailable {
            calibration_blocks: calibration_block_count,
            minimum_miscoverage_ppm: adaptive_contract.minimum_miscoverage_ppm,
            required_minimum_miscoverage_ppm,
        });
    }

    let mut sorted_scores = calibration_scores;
    sorted_scores.sort_by(f64::total_cmp);

    let evaluation_blocks = evaluation.evaluation_blocks();
    let mut previous_outcome_available_at_ns = evaluation
        .calibration_blocks()
        .last()
        .expect("calibration set is validated as non-empty")
        .valid_end_ns();

    let mut current_miscoverage_ppm = adaptive_contract.initial_miscoverage_ppm;
    let mut receipts = Vec::with_capacity(evaluation_blocks.len());
    let mut joint_covered_block_count = 0_usize;
    let mut cell_covered_count = 0_usize;
    let mut total_cell_count = 0_usize;
    let mut interval_width_sum = 0.0_f64;
    let mut maximum_interval_width = 0.0_f64;
    let mut normalized_error_sum = 0.0_f64;
    let mut minimum_bound_hit_count = 0_usize;
    let mut maximum_bound_hit_count = 0_usize;

    for block in evaluation_blocks {
        if block.issued_at_ns() < previous_outcome_available_at_ns {
            return Err(
                OnlineAdaptiveCalibrationError::EvaluationForecastIssuedBeforePreviousOutcome {
                    block_id: block.block_id().to_string(),
                    issued_at_ns: block.issued_at_ns(),
                    previous_outcome_available_at_ns,
                },
            );
        }

        let rank = conformal_rank(calibration_block_count, current_miscoverage_ppm)?;
        if rank == 0 || rank > sorted_scores.len() {
            return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                "finite conformal rank after bound validation",
            ));
        }
        let multiplier = sorted_scores[rank - 1];
        let mut maximum_score = 0.0_f64;
        let mut covered_cell_count = 0_usize;
        let mut block_width_sum = 0.0_f64;
        let mut block_error_sum = 0.0_f64;
        let mut cell_receipts = Vec::with_capacity(block.cells().len());

        for cell in block.cells() {
            let absolute_error = (cell.realized_value() - cell.point_forecast()).abs();
            let normalized_error = absolute_error / cell.normalization_scale();
            let radius = multiplier * cell.normalization_scale();
            let interval_width = 2.0 * radius;
            let lower = cell.point_forecast() - radius;
            let upper = cell.point_forecast() + radius;
            if !absolute_error.is_finite()
                || !normalized_error.is_finite()
                || !radius.is_finite()
                || !interval_width.is_finite()
                || !lower.is_finite()
                || !upper.is_finite()
            {
                return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                    "online interval and residual calculation",
                ));
            }
            maximum_score = maximum_score.max(normalized_error);
            let covered = cell.realized_value() >= lower && cell.realized_value() <= upper;
            if covered {
                covered_cell_count += 1;
            }
            cell_receipts.push(OnlineAdaptiveCellReceipt {
                channel_id: cell.channel_id().to_string(),
                horizon_index: cell.horizon_index(),
                point_forecast: cell.point_forecast(),
                realized_value: cell.realized_value(),
                normalization_scale: cell.normalization_scale(),
                lower_bound: lower,
                upper_bound: upper,
                normalized_absolute_error: normalized_error,
                covered,
                normalization_scale_evidence_id: cell.normalization_scale_evidence_id().to_string(),
                forecast_evidence_id: cell.forecast_evidence_id().to_string(),
                realization_evidence_id: cell.realization_evidence_id().to_string(),
            });
            block_width_sum += interval_width;
            block_error_sum += normalized_error;
            if !block_width_sum.is_finite() || !block_error_sum.is_finite() {
                return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                    "block aggregate metrics",
                ));
            }
        }

        let total_cells = block.cells().len();
        if total_cells == 0 {
            return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                "empty evaluation block",
            ));
        }
        let joint_covered = covered_cell_count == total_cells;
        if joint_covered {
            joint_covered_block_count += 1;
        }
        cell_covered_count += covered_cell_count;
        total_cell_count += total_cells;
        interval_width_sum += block_width_sum;
        normalized_error_sum += block_error_sum;
        maximum_interval_width = maximum_interval_width.max(
            block
                .cells()
                .iter()
                .map(|cell| 2.0 * multiplier * cell.normalization_scale())
                .fold(0.0_f64, f64::max),
        );
        if !interval_width_sum.is_finite() || !normalized_error_sum.is_finite() {
            return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                "global aggregation",
            ));
        }

        let observed_miss_ppm = if joint_covered { 0_i128 } else { 1_000_000_i128 };
        let target_error_ppm = i128::from(target_miscoverage_ppm) - observed_miss_ppm;
        let adjustment = (i128::from(adaptive_contract.step_size_ppm) * target_error_ppm)
            / i128::from(PPM_DENOMINATOR as u64);
        let unbounded_next = i128::from(current_miscoverage_ppm) + adjustment;
        // The bounded controller intentionally projects out-of-range ACI
        // updates onto the caller-pinned interval; negative intermediate
        // alpha values are therefore saturation, not arithmetic overflow.
        let clamped_to_minimum =
            unbounded_next < i128::from(adaptive_contract.minimum_miscoverage_ppm);
        let clamped_to_maximum =
            unbounded_next > i128::from(adaptive_contract.maximum_miscoverage_ppm);
        let next_miscoverage_ppm = unbounded_next
            .clamp(
                i128::from(adaptive_contract.minimum_miscoverage_ppm),
                i128::from(adaptive_contract.maximum_miscoverage_ppm),
            ) as u32;
        if clamped_to_minimum {
            minimum_bound_hit_count += 1;
        }
        if clamped_to_maximum {
            maximum_bound_hit_count += 1;
        }

        receipts.push(OnlineAdaptiveBlockReceipt {
            block_id: block.block_id().to_string(),
            issued_at_ns: block.issued_at_ns(),
            valid_start_ns: block.valid_start_ns(),
            valid_end_ns: block.valid_end_ns(),
            miscoverage_before_ppm: current_miscoverage_ppm,
            selected_order_statistic_rank: rank,
            calibration_block_count,
            calibrated_scale_multiplier: multiplier,
            maximum_normalized_absolute_error: maximum_score,
            joint_covered,
            covered_cell_count,
            total_cell_count: total_cells,
            mean_interval_width: block_width_sum / total_cells as f64,
            mean_normalized_absolute_error: block_error_sum / total_cells as f64,
            miscoverage_after_ppm: next_miscoverage_ppm,
            clamped_to_minimum,
            clamped_to_maximum,
            cells: cell_receipts,
        });

        current_miscoverage_ppm = next_miscoverage_ppm;
        previous_outcome_available_at_ns = block.valid_end_ns();
    }

    let evaluation_block_count = receipts.len();
    if evaluation_block_count == 0 || total_cell_count == 0 {
        return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
            "empty online evaluation aggregation",
        ));
    }
    let empirical_miscoverage_ppm = u32::try_from(
        (((evaluation_block_count - joint_covered_block_count) as u128) * PPM_DENOMINATOR)
            / evaluation_block_count as u128,
    )
    .map_err(|_| {
        OnlineAdaptiveCalibrationError::NumericalOverflow(
            "empirical miscoverage conversion",
        )
    })?;

    Ok(OnlineAdaptiveCalibrationReport {
        benchmark_id: base_contract.benchmark_id().to_string(),
        forecast_model_digest: base_contract.forecast_model_digest().to_string(),
        calibration_set_digest: base_contract.calibration_set_digest().to_string(),
        evaluation_set_digest: base_contract.evaluation_set_digest().to_string(),
        metric_convention_id: "SYMTROPY-PREQUENTIAL-BOUNDED-ACI-BLOCK-MAX-V0.1".to_string(),
        coverage_assumption: "EMPIRICAL_PREQUENTIAL_COVERAGE_NO_EXCHANGEABILITY_GUARANTEE_UNDER_DRIFT".to_string(),
        target_joint_coverage_ppm: base_contract.target_joint_coverage_ppm(),
        target_miscoverage_ppm,
        initial_miscoverage_ppm: adaptive_contract.initial_miscoverage_ppm,
        final_miscoverage_ppm: current_miscoverage_ppm,
        step_size_ppm: adaptive_contract.step_size_ppm,
        minimum_miscoverage_ppm: adaptive_contract.minimum_miscoverage_ppm,
        maximum_miscoverage_ppm: adaptive_contract.maximum_miscoverage_ppm,
        calibration_block_count,
        evaluation_block_count,
        joint_covered_block_count,
        joint_coverage_fraction: joint_covered_block_count as f64
            / evaluation_block_count as f64,
        cell_covered_count,
        total_cell_count,
        cell_coverage_fraction: cell_covered_count as f64 / total_cell_count as f64,
        mean_interval_width: interval_width_sum / total_cell_count as f64,
        maximum_interval_width,
        mean_normalized_absolute_error: normalized_error_sum / total_cell_count as f64,
        minimum_bound_hit_count,
        maximum_bound_hit_count,
        empirical_miscoverage_ppm,
        receipts,
    })
}

fn conformal_rank(
    calibration_count: usize,
    miscoverage_ppm: u32,
) -> Result<usize, OnlineAdaptiveCalibrationError> {
    let count_plus_one = calibration_count
        .checked_add(1)
        .ok_or(OnlineAdaptiveCalibrationError::NumericalOverflow(
            "conformal rank count",
        ))?;
    let coverage_ppm = PPM_DENOMINATOR - u128::from(miscoverage_ppm);
    let numerator = (count_plus_one as u128)
        .checked_mul(coverage_ppm)
        .ok_or(OnlineAdaptiveCalibrationError::NumericalOverflow(
            "conformal rank numerator",
        ))?;
    let rank = numerator
        .checked_add(PPM_DENOMINATOR - 1)
        .ok_or(OnlineAdaptiveCalibrationError::NumericalOverflow(
            "conformal rank ceiling",
        ))?
        / PPM_DENOMINATOR;
    usize::try_from(rank).map_err(|_| {
        OnlineAdaptiveCalibrationError::NumericalOverflow("conformal rank conversion")
    })
}

fn minimum_finite_miscoverage_ppm(
    calibration_count: usize,
) -> Result<u32, OnlineAdaptiveCalibrationError> {
    let denominator = (calibration_count as u128) + 1;
    let required = (PPM_DENOMINATOR + denominator - 1) / denominator;
    u32::try_from(required).map_err(|_| {
        OnlineAdaptiveCalibrationError::NumericalOverflow(
            "minimum finite miscoverage conversion",
        )
    })
}

fn block_max_score(
    block: &ForecastCalibrationBlock,
) -> Result<f64, OnlineAdaptiveCalibrationError> {
    if block.cells().is_empty() {
        return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
            "empty calibration block",
        ));
    }
    let mut maximum = 0.0_f64;
    for cell in block.cells() {
        let absolute_error = (cell.realized_value() - cell.point_forecast()).abs();
        let normalized_error = absolute_error / cell.normalization_scale();
        if !absolute_error.is_finite() || !normalized_error.is_finite() {
            return Err(OnlineAdaptiveCalibrationError::NumericalOverflow(
                "adaptive calibration residual",
            ));
        }
        maximum = maximum.max(normalized_error);
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ForecastCalibrationCell, ForecastCalibrationContract, ForecastCalibrationEvaluation,
    };

    const CHANNELS: [&str; 2] = ["pv", "workload"];

    fn base_contract() -> ForecastCalibrationContract {
        ForecastCalibrationContract::new(
            "aci-benchmark",
            "model-digest",
            "calibration-digest",
            "evaluation-digest",
            "config-digest",
            CHANNELS.iter().map(|value| (*value).to_string()).collect(),
            1,
            950_000,
            1,
        )
        .unwrap()
    }

    fn cell(channel: &str, error: f64) -> ForecastCalibrationCell {
        ForecastCalibrationCell::new(
            channel,
            0,
            100.0,
            100.0 + error,
            1.0,
            format!("scale-{channel}"),
            format!("forecast-{channel}"),
            format!("actual-{channel}"),
        )
        .unwrap()
    }

    fn block(id: &str, start: u64, error: f64) -> ForecastCalibrationBlock {
        ForecastCalibrationBlock::new(
            id,
            start - 10,
            start,
            start + 10,
            vec![cell("pv", error), cell("workload", error * 0.5)],
        )
        .unwrap()
    }

    fn stream(evaluation_errors: &[f64]) -> ForecastCalibrationEvaluation {
        let calibration = (0_u32..99)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1 + f64::from(index) * 0.1,
                )
            })
            .collect();
        let evaluation = evaluation_errors
            .iter()
            .enumerate()
            .map(|(index, error)| {
                block(&format!("eval-{index}"), 3_000 + (index as u64) * 20, *error)
            })
            .collect();
        ForecastCalibrationEvaluation::new(base_contract(), calibration, evaluation).unwrap()
    }

    fn config() -> OnlineAdaptiveCalibrationContract {
        OnlineAdaptiveCalibrationContract::new(50_000, 10_000, 10_000, 300_000).unwrap()
    }

    #[test]
    fn miss_reduces_miscoverage_and_raises_the_next_quantile() {
        let report = evaluate_online_adaptive_calibration(&stream(&[9.9, 9.6]), &config()).unwrap();
        assert_eq!(report.receipts[0].selected_order_statistic_rank, 95);
        assert_eq!(report.receipts[0].miscoverage_after_ppm, 40_500);
        assert_eq!(report.receipts[1].selected_order_statistic_rank, 96);
        assert!((report.receipts[1].calibrated_scale_multiplier - 9.6).abs() < 1e-9);
        assert!(!report.receipts[0].joint_covered);
        assert!(report.receipts[1].joint_covered);
        assert_eq!(report.receipts[1].miscoverage_after_ppm, 41_000);
    }

    #[test]
    fn repeated_extreme_misses_reach_and_report_the_lower_bound() {
        let report = evaluate_online_adaptive_calibration(
            &stream(&[100.0, 100.0, 100.0, 100.0, 100.0]),
            &config(),
        )
        .unwrap();
        assert_eq!(report.final_miscoverage_ppm, 10_000);
        assert!(report.minimum_bound_hit_count > 0);
        assert!(report.receipts.last().unwrap().clamped_to_minimum);
    }

    #[test]
    fn repeated_covered_blocks_reach_and_report_the_upper_bound() {
        let bounded =
            OnlineAdaptiveCalibrationContract::new(50_000, 10_000, 10_000, 50_500).unwrap();
        let report =
            evaluate_online_adaptive_calibration(&stream(&[0.1, 0.1]), &bounded).unwrap();
        assert_eq!(report.final_miscoverage_ppm, 50_500);
        assert!(report.maximum_bound_hit_count > 0);
        assert!(report.receipts.last().unwrap().clamped_to_maximum);
    }

    #[test]
    fn large_step_projects_negative_miscoverage_onto_the_declared_lower_bound() {
        let aggressive =
            OnlineAdaptiveCalibrationContract::new(50_000, 1_000_000, 10_000, 300_000)
                .unwrap();
        let report =
            evaluate_online_adaptive_calibration(&stream(&[100.0]), &aggressive).unwrap();
        assert_eq!(report.final_miscoverage_ppm, 10_000);
        assert!(report.receipts[0].clamped_to_minimum);
    }

    #[test]
    fn rejects_a_non_finite_quantile_under_the_configured_lower_bound() {
        let bounded =
            OnlineAdaptiveCalibrationContract::new(50_000, 10_000, 9_999, 300_000).unwrap();
        assert!(matches!(
            evaluate_online_adaptive_calibration(&stream(&[0.1]), &bounded),
            Err(OnlineAdaptiveCalibrationError::FiniteQuantileUnavailable {
                calibration_blocks: 99,
                minimum_miscoverage_ppm: 9_999,
                required_minimum_miscoverage_ppm: 10_000,
            })
        ));
    }

    #[test]
    fn rejects_forecast_issued_before_the_previous_evaluation_outcome() {
        let calibration = (0_u32..99)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1 + f64::from(index) * 0.1,
                )
            })
            .collect();
        let first = block("eval-0", 3_000, 0.1);
        let second = ForecastCalibrationBlock::new(
            "eval-1-early",
            3_005,
            3_020,
            3_030,
            vec![cell("pv", 0.1), cell("workload", 0.05)],
        )
        .unwrap();
        let evaluation =
            ForecastCalibrationEvaluation::new(base_contract(), calibration, vec![first, second])
                .unwrap();
        assert!(matches!(
            evaluate_online_adaptive_calibration(&evaluation, &config()),
            Err(OnlineAdaptiveCalibrationError::EvaluationForecastIssuedBeforePreviousOutcome {
                ..
            })
        ));
    }

    #[test]
    fn stationary_covered_stream_increases_miscoverage_to_sharpen_future_intervals() {
        let report =
            evaluate_online_adaptive_calibration(&stream(&[1.0, 1.0, 1.0]), &config()).unwrap();
        assert!(report.final_miscoverage_ppm > report.initial_miscoverage_ppm);
        assert!(report.mean_interval_width.is_finite());
        assert!(report.joint_coverage_fraction >= 0.0);
    }

    #[test]
    fn rejects_invalid_rates_and_alpha_bounds() {
        assert!(OnlineAdaptiveCalibrationContract::new(0, 1, 1, 2).is_err());
        assert!(OnlineAdaptiveCalibrationContract::new(50_000, 0, 1, 100_000).is_err());
        assert!(OnlineAdaptiveCalibrationContract::new(50_000, 1, 60_000, 100_000).is_err());
        assert!(OnlineAdaptiveCalibrationContract::new(50_000, 1, 1, 1_000_000).is_err());
    }

    #[test]
    fn reports_joint_coverage_and_cell_coverage_separately() {
        let report =
            evaluate_online_adaptive_calibration(&stream(&[9.9, 0.1]), &config()).unwrap();
        assert!(report.joint_coverage_fraction <= report.cell_coverage_fraction);
        assert_eq!(report.receipts.len(), 2);
        assert_eq!(report.receipts[0].total_cell_count, 2);
        let pv = &report.receipts[0].cells[0];
        assert_eq!(pv.channel_id, "pv");
        assert_eq!(pv.normalization_scale_evidence_id, "scale-pv");
        assert_eq!(pv.forecast_evidence_id, "forecast-pv");
        assert_eq!(pv.realization_evidence_id, "actual-pv");
    }

    #[test]
    fn rejects_target_miscoverage_outside_the_adaptive_bounds() {
        let too_narrow = OnlineAdaptiveCalibrationContract::new(
            100_000, 10_000, 100_000, 300_000
        )
        .unwrap();
        assert!(matches!(
            evaluate_online_adaptive_calibration(&stream(&[0.1]), &too_narrow),
            Err(OnlineAdaptiveCalibrationError::TargetMiscoverageOutsideBounds {
                target: 50_000,
                ..
            })
        ));
    }

    #[test]
    fn deterministic_replay_preserves_every_alpha_and_quantile_decision() {
        let input = stream(&[9.9, 9.6, 9.7, 2.0, 1.0]);
        let first = evaluate_online_adaptive_calibration(&input, &config()).unwrap();
        let second = evaluate_online_adaptive_calibration(&input, &config()).unwrap();
        assert_eq!(first, second);
    }
}
