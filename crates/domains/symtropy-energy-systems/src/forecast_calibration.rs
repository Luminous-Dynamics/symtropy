// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Simultaneous forecast-envelope calibration over chronologically held-out blocks.
//!
//! The evaluator uses one block-max, scale-normalized absolute-error score across
//! every declared channel and forecast horizon. Its split-conformal order
//! statistic produces a common interval multiplier. This avoids treating
//! separately calibrated marginals as though they establish simultaneous
//! path coverage.
//!
//! The finite-sample split-conformal statement requires exchangeable
//! calibration and future blocks. Chronological separation prevents look-ahead
//! leakage but does not establish exchangeability. Serial dependence, regime
//! changes and covariate shift can invalidate nominal coverage. The report
//! therefore includes measured holdout coverage and never claims arbitrary
//! future-time guarantees.
//!
//! This module calibrates an interval envelope. It does not estimate a full
//! joint distribution, preserve a copula, generate scenarios, fit a DRO
//! ambiguity set, forecast any channel, or authenticate evidence bytes.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

const COVERAGE_DENOMINATOR: u128 = 1_000_000;

/// Frozen forecast-calibration contract.
///
/// Coverage is represented in parts per million so the conformal rank and the
/// holdout gate can be evaluated using integer arithmetic.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationContract {
    benchmark_id: String,
    forecast_model_digest: String,
    calibration_set_digest: String,
    evaluation_set_digest: String,
    configuration_digest: String,
    channels: Vec<String>,
    horizon_count: usize,
    target_joint_coverage_ppm: u32,
    minimum_evaluation_blocks: usize,
}

impl ForecastCalibrationContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        benchmark_id: impl Into<String>,
        forecast_model_digest: impl Into<String>,
        calibration_set_digest: impl Into<String>,
        evaluation_set_digest: impl Into<String>,
        configuration_digest: impl Into<String>,
        mut channels: Vec<String>,
        horizon_count: usize,
        target_joint_coverage_ppm: u32,
        minimum_evaluation_blocks: usize,
    ) -> Result<Self, ForecastCalibrationError> {
        let contract = Self {
            benchmark_id: benchmark_id.into(),
            forecast_model_digest: forecast_model_digest.into(),
            calibration_set_digest: calibration_set_digest.into(),
            evaluation_set_digest: evaluation_set_digest.into(),
            configuration_digest: configuration_digest.into(),
            channels: {
                channels.sort();
                channels
            },
            horizon_count,
            target_joint_coverage_ppm,
            minimum_evaluation_blocks,
        };
        for (field, value) in [
            ("benchmark ID", contract.benchmark_id.as_str()),
            ("forecast-model digest", contract.forecast_model_digest.as_str()),
            ("calibration-set digest", contract.calibration_set_digest.as_str()),
            ("evaluation-set digest", contract.evaluation_set_digest.as_str()),
            ("configuration digest", contract.configuration_digest.as_str()),
        ] {
            require_id(field, value)?;
        }
        if contract.channels.is_empty() {
            return Err(ForecastCalibrationError::EmptyChannelSet);
        }
        for channel in &contract.channels {
            require_id("channel ID", channel)?;
        }
        if contract.channels.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ForecastCalibrationError::DuplicateExpectedChannel);
        }
        if contract.horizon_count == 0 {
            return Err(ForecastCalibrationError::InvalidHorizonCount);
        }
        if contract.target_joint_coverage_ppm == 0
            || contract.target_joint_coverage_ppm >= COVERAGE_DENOMINATOR as u32
        {
            return Err(ForecastCalibrationError::InvalidCoverage(
                contract.target_joint_coverage_ppm,
            ));
        }
        if contract.minimum_evaluation_blocks == 0 {
            return Err(ForecastCalibrationError::InvalidMinimumEvaluationBlocks);
        }
        Ok(contract)
    }

    pub fn benchmark_id(&self) -> &str {
        &self.benchmark_id
    }

    pub fn forecast_model_digest(&self) -> &str {
        &self.forecast_model_digest
    }

    pub fn channels(&self) -> &[String] {
        &self.channels
    }

    pub const fn horizon_count(&self) -> usize {
        self.horizon_count
    }

    pub const fn target_joint_coverage_ppm(&self) -> u32 {
        self.target_joint_coverage_ppm
    }
}

/// One forecast/realization pair at a declared channel and forecast horizon.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationCell {
    channel_id: String,
    horizon_index: usize,
    point_forecast: f64,
    realized_value: f64,
    normalization_scale: f64,
    normalization_scale_evidence_id: String,
    forecast_evidence_id: String,
    realization_evidence_id: String,
}

impl ForecastCalibrationCell {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        channel_id: impl Into<String>,
        horizon_index: usize,
        point_forecast: f64,
        realized_value: f64,
        normalization_scale: f64,
        normalization_scale_evidence_id: impl Into<String>,
        forecast_evidence_id: impl Into<String>,
        realization_evidence_id: impl Into<String>,
    ) -> Result<Self, ForecastCalibrationError> {
        let cell = Self {
            channel_id: channel_id.into(),
            horizon_index,
            point_forecast,
            realized_value,
            normalization_scale,
            normalization_scale_evidence_id: normalization_scale_evidence_id.into(),
            forecast_evidence_id: forecast_evidence_id.into(),
            realization_evidence_id: realization_evidence_id.into(),
        };
        require_id("forecast channel ID", &cell.channel_id)?;
        require_id(
            "normalization-scale evidence ID",
            &cell.normalization_scale_evidence_id,
        )?;
        require_id("forecast evidence ID", &cell.forecast_evidence_id)?;
        require_id("realization evidence ID", &cell.realization_evidence_id)?;
        for (field, value) in [
            ("point forecast", cell.point_forecast),
            ("realized value", cell.realized_value),
            ("normalization scale", cell.normalization_scale),
        ] {
            if !value.is_finite() {
                return Err(ForecastCalibrationError::NonFiniteCellValue {
                    field,
                    value,
                });
            }
        }
        if cell.normalization_scale <= 0.0 {
            return Err(ForecastCalibrationError::NonPositiveNormalizationScale(
                cell.normalization_scale,
            ));
        }
        Ok(cell)
    }

    pub fn channel_id(&self) -> &str {
        &self.channel_id
    }

    pub const fn horizon_index(&self) -> usize {
        self.horizon_index
    }

    pub const fn point_forecast(&self) -> f64 {
        self.point_forecast
    }

    pub const fn realized_value(&self) -> f64 {
        self.realized_value
    }

    pub const fn normalization_scale(&self) -> f64 {
        self.normalization_scale
    }

    pub fn normalization_scale_evidence_id(&self) -> &str {
        &self.normalization_scale_evidence_id
    }
}

/// One forecast issued before a contiguous target window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationBlock {
    block_id: String,
    issued_at_ns: u64,
    valid_start_ns: u64,
    valid_end_ns: u64,
    cells: Vec<ForecastCalibrationCell>,
}

impl ForecastCalibrationBlock {
    pub fn new(
        block_id: impl Into<String>,
        issued_at_ns: u64,
        valid_start_ns: u64,
        valid_end_ns: u64,
        mut cells: Vec<ForecastCalibrationCell>,
    ) -> Result<Self, ForecastCalibrationError> {
        let block_id = block_id.into();
        require_id("forecast block ID", &block_id)?;
        if valid_end_ns <= valid_start_ns {
            return Err(ForecastCalibrationError::InvalidBlockWindow {
                block_id,
                valid_start_ns,
                valid_end_ns,
            });
        }
        if issued_at_ns >= valid_start_ns {
            return Err(ForecastCalibrationError::ForecastIssuedAfterValidityStarted {
                block_id,
                issued_at_ns,
                valid_start_ns,
            });
        }
        if cells.is_empty() {
            return Err(ForecastCalibrationError::EmptyBlockCells(block_id));
        }
        let mut keys = BTreeSet::new();
        for cell in &cells {
            let key = (cell.channel_id.clone(), cell.horizon_index);
            if !keys.insert(key.clone()) {
                return Err(ForecastCalibrationError::DuplicateCell {
                    block_id,
                    channel_id: key.0,
                    horizon_index: key.1,
                });
            }
        }
        cells.sort_by(|left, right| {
            left.channel_id
                .cmp(&right.channel_id)
                .then_with(|| left.horizon_index.cmp(&right.horizon_index))
        });
        Ok(Self {
            block_id,
            issued_at_ns,
            valid_start_ns,
            valid_end_ns,
            cells,
        })
    }

    pub fn block_id(&self) -> &str {
        &self.block_id
    }

    pub const fn issued_at_ns(&self) -> u64 {
        self.issued_at_ns
    }

    pub const fn valid_start_ns(&self) -> u64 {
        self.valid_start_ns
    }

    pub const fn valid_end_ns(&self) -> u64 {
        self.valid_end_ns
    }

    pub fn cells(&self) -> &[ForecastCalibrationCell] {
        &self.cells
    }
}

/// Chronological calibration and untouched future evaluation blocks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationEvaluation {
    contract: ForecastCalibrationContract,
    calibration_blocks: Vec<ForecastCalibrationBlock>,
    evaluation_blocks: Vec<ForecastCalibrationBlock>,
}

impl ForecastCalibrationEvaluation {
    pub fn new(
        contract: ForecastCalibrationContract,
        mut calibration_blocks: Vec<ForecastCalibrationBlock>,
        mut evaluation_blocks: Vec<ForecastCalibrationBlock>,
    ) -> Result<Self, ForecastCalibrationError> {
        if calibration_blocks.is_empty() {
            return Err(ForecastCalibrationError::EmptyCalibrationBlocks);
        }
        if evaluation_blocks.len() < contract.minimum_evaluation_blocks {
            return Err(ForecastCalibrationError::InsufficientEvaluationBlocks {
                available: evaluation_blocks.len(),
                required: contract.minimum_evaluation_blocks,
            });
        }

        calibration_blocks.sort_by_key(ForecastCalibrationBlock::valid_start_ns);
        evaluation_blocks.sort_by_key(ForecastCalibrationBlock::valid_start_ns);

        let mut block_ids = BTreeSet::new();
        for block in calibration_blocks.iter().chain(evaluation_blocks.iter()) {
            if !block_ids.insert(block.block_id.clone()) {
                return Err(ForecastCalibrationError::DuplicateBlockId(
                    block.block_id.clone(),
                ));
            }
            validate_block_grid(&contract, block)?;
        }

        validate_non_overlapping_windows(&calibration_blocks)?;
        validate_non_overlapping_windows(&evaluation_blocks)?;

        let last_calibration_end = calibration_blocks
            .last()
            .expect("non-empty calibration blocks")
            .valid_end_ns;
        let first_evaluation_start = evaluation_blocks
            .first()
            .expect("minimum evaluation blocks is non-zero")
            .valid_start_ns;
        if first_evaluation_start < last_calibration_end {
            return Err(ForecastCalibrationError::EvaluationNotAfterCalibration {
                last_calibration_end_ns: last_calibration_end,
                first_evaluation_start_ns: first_evaluation_start,
            });
        }
        for block in &evaluation_blocks {
            if block.issued_at_ns < last_calibration_end {
                return Err(
                    ForecastCalibrationError::EvaluationForecastIssuedBeforeCalibration {
                        block_id: block.block_id.clone(),
                        issued_at_ns: block.issued_at_ns,
                        calibration_available_at_ns: last_calibration_end,
                    },
                );
            }
        }

        Ok(Self {
            contract,
            calibration_blocks,
            evaluation_blocks,
        })
    }

    pub fn contract(&self) -> &ForecastCalibrationContract {
        &self.contract
    }

    pub fn calibration_blocks(&self) -> &[ForecastCalibrationBlock] {
        &self.calibration_blocks
    }

    pub fn evaluation_blocks(&self) -> &[ForecastCalibrationBlock] {
        &self.evaluation_blocks
    }
}

/// Calibration score for one block. The selected order statistic is marked.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationBlockReceipt {
    pub block_id: String,
    pub valid_start_ns: u64,
    pub valid_end_ns: u64,
    pub maximum_normalized_absolute_error: f64,
    pub selected_order_statistic: bool,
}

/// One held-out cell interval and its evidence references.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastIntervalCellReceipt {
    pub channel_id: String,
    pub horizon_index: usize,
    pub point_forecast: f64,
    pub realized_value: f64,
    pub normalization_scale: f64,
    pub normalization_scale_evidence_id: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
    pub normalized_absolute_error: f64,
    pub covered: bool,
    pub forecast_evidence_id: String,
    pub realization_evidence_id: String,
}

/// Simultaneous coverage result for one untouched evaluation block.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationEvaluationBlockReceipt {
    pub block_id: String,
    pub valid_start_ns: u64,
    pub valid_end_ns: u64,
    pub maximum_normalized_absolute_error: f64,
    pub joint_covered: bool,
    pub cells: Vec<ForecastIntervalCellReceipt>,
}

/// Calibration report. Nominal coverage is conditional on exchangeable blocks.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ForecastCalibrationReport {
    pub benchmark_id: String,
    pub forecast_model_digest: String,
    pub calibration_set_digest: String,
    pub evaluation_set_digest: String,
    pub configuration_digest: String,
    pub metric_convention_id: String,
    pub coverage_assumption: String,
    pub target_joint_coverage_ppm: u32,
    pub calibration_block_count: usize,
    pub evaluation_block_count: usize,
    pub selected_order_statistic_rank: usize,
    pub selected_calibration_block_id: String,
    pub calibrated_scale_multiplier: f64,
    pub joint_covered_block_count: usize,
    pub joint_coverage_fraction: f64,
    pub cell_covered_count: usize,
    pub total_cell_count: usize,
    pub cell_coverage_fraction: f64,
    pub mean_interval_width: f64,
    pub maximum_interval_width: f64,
    pub mean_normalized_absolute_error: f64,
    pub holdout_coverage_gate_passed: bool,
    pub calibration_receipts: Vec<ForecastCalibrationBlockReceipt>,
    pub evaluation_receipts: Vec<ForecastCalibrationEvaluationBlockReceipt>,
}

/// Invalid forecast grids, chronology, evidence or numerical operations.
#[derive(Debug, Clone, PartialEq)]
pub enum ForecastCalibrationError {
    EmptyIdentifier(&'static str),
    EmptyChannelSet,
    DuplicateExpectedChannel,
    InvalidHorizonCount,
    InvalidCoverage(u32),
    InvalidMinimumEvaluationBlocks,
    NonFiniteCellValue {
        field: &'static str,
        value: f64,
    },
    NonPositiveNormalizationScale(f64),
    InvalidBlockWindow {
        block_id: String,
        valid_start_ns: u64,
        valid_end_ns: u64,
    },
    ForecastIssuedAfterValidityStarted {
        block_id: String,
        issued_at_ns: u64,
        valid_start_ns: u64,
    },
    EmptyBlockCells(String),
    DuplicateCell {
        block_id: String,
        channel_id: String,
        horizon_index: usize,
    },
    EmptyCalibrationBlocks,
    InsufficientEvaluationBlocks {
        available: usize,
        required: usize,
    },
    DuplicateBlockId(String),
    UnexpectedChannel {
        block_id: String,
        channel_id: String,
    },
    UnexpectedHorizon {
        block_id: String,
        channel_id: String,
        horizon_index: usize,
    },
    IncompleteCellGrid {
        block_id: String,
        expected: usize,
        observed: usize,
    },
    OverlappingWindows {
        earlier_block_id: String,
        later_block_id: String,
    },
    EvaluationNotAfterCalibration {
        last_calibration_end_ns: u64,
        first_evaluation_start_ns: u64,
    },
    EvaluationForecastIssuedBeforeCalibration {
        block_id: String,
        issued_at_ns: u64,
        calibration_available_at_ns: u64,
    },
    InsufficientCalibrationBlocks {
        available: usize,
        required: usize,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for ForecastCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(f, "{field} must not be empty"),
            Self::EmptyChannelSet => write!(f, "at least one forecast channel is required"),
            Self::DuplicateExpectedChannel => write!(f, "forecast channel IDs must be unique"),
            Self::InvalidHorizonCount => write!(f, "forecast horizon count must be positive"),
            Self::InvalidCoverage(value) => {
                write!(f, "joint coverage must be in 1..1,000,000 ppm; got {value}")
            }
            Self::InvalidMinimumEvaluationBlocks => {
                write!(f, "minimum evaluation block count must be positive")
            }
            Self::NonFiniteCellValue { field, value } => {
                write!(f, "{field} must be finite; got {value}")
            }
            Self::NonPositiveNormalizationScale(value) => {
                write!(f, "normalization scale must be positive; got {value}")
            }
            Self::InvalidBlockWindow {
                block_id,
                valid_start_ns,
                valid_end_ns,
            } => write!(
                f,
                "block {block_id} has invalid window [{valid_start_ns}, {valid_end_ns})"
            ),
            Self::ForecastIssuedAfterValidityStarted {
                block_id,
                issued_at_ns,
                valid_start_ns,
            } => write!(
                f,
                "block {block_id} was issued at {issued_at_ns}, not before validity {valid_start_ns}"
            ),
            Self::EmptyBlockCells(block_id) => {
                write!(f, "block {block_id} has no forecast cells")
            }
            Self::DuplicateCell {
                block_id,
                channel_id,
                horizon_index,
            } => write!(
                f,
                "block {block_id} duplicates channel {channel_id} horizon {horizon_index}"
            ),
            Self::EmptyCalibrationBlocks => write!(f, "calibration block set is empty"),
            Self::InsufficientEvaluationBlocks { available, required } => write!(
                f,
                "evaluation set has {available} blocks; {required} required"
            ),
            Self::DuplicateBlockId(id) => write!(f, "duplicate forecast block ID {id}"),
            Self::UnexpectedChannel {
                block_id,
                channel_id,
            } => write!(
                f,
                "block {block_id} contains undeclared channel {channel_id}"
            ),
            Self::UnexpectedHorizon {
                block_id,
                channel_id,
                horizon_index,
            } => write!(
                f,
                "block {block_id} channel {channel_id} has undeclared horizon {horizon_index}"
            ),
            Self::IncompleteCellGrid {
                block_id,
                expected,
                observed,
            } => write!(
                f,
                "block {block_id} has {observed} forecast cells; expected {expected}"
            ),
            Self::OverlappingWindows {
                earlier_block_id,
                later_block_id,
            } => write!(
                f,
                "forecast blocks {earlier_block_id} and {later_block_id} overlap or are unordered"
            ),
            Self::EvaluationNotAfterCalibration {
                last_calibration_end_ns,
                first_evaluation_start_ns,
            } => write!(
                f,
                "evaluation starts at {first_evaluation_start_ns} before calibration ends at {last_calibration_end_ns}"
            ),
            Self::EvaluationForecastIssuedBeforeCalibration {
                block_id,
                issued_at_ns,
                calibration_available_at_ns,
            } => write!(
                f,
                "evaluation forecast {block_id} was issued at {issued_at_ns} before calibration was available at {calibration_available_at_ns}"
            ),
            Self::InsufficientCalibrationBlocks { available, required } => write!(
                f,
                "{available} calibration blocks available; {required} required for finite conformal rank"
            ),
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for ForecastCalibrationError {}

/// Calibrate a simultaneous block envelope and evaluate it on future blocks.
///
/// The finite-sample split-conformal order statistic is
/// ceil((n + 1) * target_coverage), using integer parts-per-million arithmetic.
/// An unavailable finite rank is an error; this routine never substitutes an
/// interpolated quantile or an arbitrary maximum score.
pub fn evaluate_forecast_calibration(
    evaluation: &ForecastCalibrationEvaluation,
) -> Result<ForecastCalibrationReport, ForecastCalibrationError> {
    let contract = &evaluation.contract;
    let calibration_scores = evaluation
        .calibration_blocks
        .iter()
        .map(|block| {
            Ok((
                block,
                block_max_score(block)?,
            ))
        })
        .collect::<Result<Vec<_>, ForecastCalibrationError>>()?;

    let count_plus_one = calibration_scores
        .len()
        .checked_add(1)
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "conformal calibration rank",
        ))?;
    let numerator = (count_plus_one as u128)
        .checked_mul(u128::from(contract.target_joint_coverage_ppm))
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "conformal calibration rank",
        ))?;
    let rank = numerator
        .checked_add(COVERAGE_DENOMINATOR - 1)
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "conformal calibration rank",
        ))?
        / COVERAGE_DENOMINATOR;
    let rank = usize::try_from(rank).map_err(|_| {
        ForecastCalibrationError::NumericalOverflow("conformal calibration rank conversion")
    })?;
    if rank == 0 {
        return Err(ForecastCalibrationError::NumericalOverflow(
            "zero conformal calibration rank",
        ));
    }
    if rank > calibration_scores.len() {
        return Err(ForecastCalibrationError::InsufficientCalibrationBlocks {
            available: calibration_scores.len(),
            required: rank,
        });
    }

    let mut ordered_scores = calibration_scores
        .iter()
        .map(|(block, score)| (score, block.block_id.as_str()))
        .collect::<Vec<_>>();
    ordered_scores.sort_by(|left, right| {
        left.0
            .total_cmp(right.0)
            .then_with(|| left.1.cmp(right.1))
    });
    let (calibrated_scale_multiplier, selected_calibration_block_id) =
        ordered_scores[rank - 1];
    let calibrated_scale_multiplier = *calibrated_scale_multiplier;
    if !calibrated_scale_multiplier.is_finite() || calibrated_scale_multiplier < 0.0 {
        return Err(ForecastCalibrationError::NumericalOverflow(
            "selected conformal scale multiplier",
        ));
    }

    let mut calibration_receipts = calibration_scores
        .iter()
        .map(|(block, score)| ForecastCalibrationBlockReceipt {
            block_id: block.block_id.clone(),
            valid_start_ns: block.valid_start_ns,
            valid_end_ns: block.valid_end_ns,
            maximum_normalized_absolute_error: *score,
            selected_order_statistic: block.block_id == selected_calibration_block_id,
        })
        .collect::<Vec<_>>();
    calibration_receipts.sort_by_key(|receipt| receipt.valid_start_ns);

    let mut evaluation_receipts = Vec::with_capacity(evaluation.evaluation_blocks.len());
    let mut joint_covered_block_count = 0_usize;
    let mut cell_covered_count = 0_usize;
    let mut total_cell_count = 0_usize;
    let mut interval_width_sum = 0.0_f64;
    let mut maximum_interval_width = 0.0_f64;
    let mut normalized_error_sum = 0.0_f64;

    for block in &evaluation.evaluation_blocks {
        let mut cell_receipts = Vec::with_capacity(block.cells.len());
        let mut maximum_score = 0.0_f64;
        let mut joint_covered = true;

        for cell in &block.cells {
            let normalized_error = normalized_absolute_error(cell)?;
            let radius = calibrated_scale_multiplier * cell.normalization_scale;
            let lower_bound = cell.point_forecast - radius;
            let upper_bound = cell.point_forecast + radius;
            let interval_width = upper_bound - lower_bound;
            if !radius.is_finite()
                || !lower_bound.is_finite()
                || !upper_bound.is_finite()
                || !interval_width.is_finite()
            {
                return Err(ForecastCalibrationError::NumericalOverflow(
                    "held-out interval construction",
                ));
            }
            let covered = cell.realized_value >= lower_bound
                && cell.realized_value <= upper_bound;
            maximum_score = maximum_score.max(normalized_error);
            joint_covered &= covered;
            if covered {
                cell_covered_count += 1;
            }
            total_cell_count += 1;
            interval_width_sum += interval_width;
            normalized_error_sum += normalized_error;
            maximum_interval_width = maximum_interval_width.max(interval_width);
            if !interval_width_sum.is_finite() || !normalized_error_sum.is_finite() {
                return Err(ForecastCalibrationError::NumericalOverflow(
                    "held-out aggregate metrics",
                ));
            }
            cell_receipts.push(ForecastIntervalCellReceipt {
                channel_id: cell.channel_id.clone(),
                horizon_index: cell.horizon_index,
                point_forecast: cell.point_forecast,
                realized_value: cell.realized_value,
                normalization_scale: cell.normalization_scale,
                normalization_scale_evidence_id: cell.normalization_scale_evidence_id.clone(),
                lower_bound,
                upper_bound,
                normalized_absolute_error: normalized_error,
                covered,
                forecast_evidence_id: cell.forecast_evidence_id.clone(),
                realization_evidence_id: cell.realization_evidence_id.clone(),
            });
        }
        if joint_covered {
            joint_covered_block_count += 1;
        }
        evaluation_receipts.push(ForecastCalibrationEvaluationBlockReceipt {
            block_id: block.block_id.clone(),
            valid_start_ns: block.valid_start_ns,
            valid_end_ns: block.valid_end_ns,
            maximum_normalized_absolute_error: maximum_score,
            joint_covered,
            cells: cell_receipts,
        });
    }

    if total_cell_count == 0 {
        return Err(ForecastCalibrationError::NumericalOverflow(
            "empty held-out cell aggregation",
        ));
    }
    let evaluation_block_count = evaluation_receipts.len();
    let joint_coverage_fraction =
        joint_covered_block_count as f64 / evaluation_block_count as f64;
    let cell_coverage_fraction = cell_covered_count as f64 / total_cell_count as f64;
    let mean_interval_width = interval_width_sum / total_cell_count as f64;
    let mean_normalized_absolute_error = normalized_error_sum / total_cell_count as f64;
    let coverage_lhs = (joint_covered_block_count as u128)
        .checked_mul(COVERAGE_DENOMINATOR)
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "holdout coverage comparison",
        ))?;
    let coverage_rhs = (evaluation_block_count as u128)
        .checked_mul(u128::from(contract.target_joint_coverage_ppm))
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "holdout coverage comparison",
        ))?;
    let holdout_coverage_gate_passed = coverage_lhs >= coverage_rhs;

    Ok(ForecastCalibrationReport {
        benchmark_id: contract.benchmark_id.clone(),
        forecast_model_digest: contract.forecast_model_digest.clone(),
        calibration_set_digest: contract.calibration_set_digest.clone(),
        evaluation_set_digest: contract.evaluation_set_digest.clone(),
        configuration_digest: contract.configuration_digest.clone(),
        metric_convention_id: "SYMTROPY-BLOCK-MAX-SPLIT-CONFORMAL-V0.1".to_string(),
        coverage_assumption: "EXCHANGEABLE_CALIBRATION_AND_FUTURE_BLOCKS_REQUIRED".to_string(),
        target_joint_coverage_ppm: contract.target_joint_coverage_ppm,
        calibration_block_count: calibration_scores.len(),
        evaluation_block_count,
        selected_order_statistic_rank: rank,
        selected_calibration_block_id: selected_calibration_block_id.to_string(),
        calibrated_scale_multiplier,
        joint_covered_block_count,
        joint_coverage_fraction,
        cell_covered_count,
        total_cell_count,
        cell_coverage_fraction,
        mean_interval_width,
        maximum_interval_width,
        mean_normalized_absolute_error,
        holdout_coverage_gate_passed,
        calibration_receipts,
        evaluation_receipts,
    })
}

fn block_max_score(
    block: &ForecastCalibrationBlock,
) -> Result<f64, ForecastCalibrationError> {
    let mut maximum = 0.0_f64;
    for cell in &block.cells {
        maximum = maximum.max(normalized_absolute_error(cell)?);
    }
    Ok(maximum)
}

fn normalized_absolute_error(
    cell: &ForecastCalibrationCell,
) -> Result<f64, ForecastCalibrationError> {
    let absolute_error = (cell.realized_value - cell.point_forecast).abs();
    let score = absolute_error / cell.normalization_scale;
    if !absolute_error.is_finite() || !score.is_finite() {
        return Err(ForecastCalibrationError::NumericalOverflow(
            "normalized forecast residual",
        ));
    }
    Ok(score)
}

fn validate_block_grid(
    contract: &ForecastCalibrationContract,
    block: &ForecastCalibrationBlock,
) -> Result<(), ForecastCalibrationError> {
    let expected_count = contract
        .channels
        .len()
        .checked_mul(contract.horizon_count)
        .ok_or(ForecastCalibrationError::NumericalOverflow(
            "expected channel-horizon grid size",
        ))?;
    if block.cells.len() != expected_count {
        return Err(ForecastCalibrationError::IncompleteCellGrid {
            block_id: block.block_id.clone(),
            expected: expected_count,
            observed: block.cells.len(),
        });
    }
    let mut observed = BTreeSet::new();
    for cell in &block.cells {
        if contract.channels.binary_search(&cell.channel_id).is_err() {
            return Err(ForecastCalibrationError::UnexpectedChannel {
                block_id: block.block_id.clone(),
                channel_id: cell.channel_id.clone(),
            });
        }
        if cell.horizon_index >= contract.horizon_count {
            return Err(ForecastCalibrationError::UnexpectedHorizon {
                block_id: block.block_id.clone(),
                channel_id: cell.channel_id.clone(),
                horizon_index: cell.horizon_index,
            });
        }
        observed.insert((cell.channel_id.as_str(), cell.horizon_index));
    }
    if observed.len() != expected_count {
        return Err(ForecastCalibrationError::IncompleteCellGrid {
            block_id: block.block_id.clone(),
            expected: expected_count,
            observed: observed.len(),
        });
    }
    Ok(())
}

fn validate_non_overlapping_windows(
    blocks: &[ForecastCalibrationBlock],
) -> Result<(), ForecastCalibrationError> {
    for pair in blocks.windows(2) {
        if pair[0].valid_end_ns > pair[1].valid_start_ns {
            return Err(ForecastCalibrationError::OverlappingWindows {
                earlier_block_id: pair[0].block_id.clone(),
                later_block_id: pair[1].block_id.clone(),
            });
        }
    }
    Ok(())
}

fn require_id(
    field: &'static str,
    value: &str,
) -> Result<(), ForecastCalibrationError> {
    if value.trim().is_empty() {
        return Err(ForecastCalibrationError::EmptyIdentifier(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHANNELS: [&str; 3] = ["ambient", "pv", "workload"];

    fn contract(target_ppm: u32, min_evaluation_blocks: usize) -> ForecastCalibrationContract {
        ForecastCalibrationContract::new(
            "benchmark-v1",
            "forecast-model-sha256",
            "calibration-set-sha256",
            "evaluation-set-sha256",
            "configuration-sha256",
            CHANNELS.iter().map(|value| (*value).to_string()).collect(),
            2,
            target_ppm,
            min_evaluation_blocks,
        )
        .unwrap()
    }

    fn cell(
        channel: &str,
        horizon: usize,
        error: f64,
    ) -> ForecastCalibrationCell {
        ForecastCalibrationCell::new(
            channel,
            horizon,
            100.0,
            100.0 + error,
            1.0,
            format!("scale-{channel}-{horizon}"),
            format!("forecast-{channel}-{horizon}"),
            format!("realized-{channel}-{horizon}"),
        )
        .unwrap()
    }

    fn block(
        id: &str,
        start: u64,
        max_error: f64,
    ) -> ForecastCalibrationBlock {
        let mut cells = Vec::new();
        for channel in CHANNELS {
            for horizon in 0..2 {
                cells.push(cell(channel, horizon, max_error * 0.5));
            }
        }
        cells[0] = cell("ambient", 0, max_error);
        ForecastCalibrationBlock::new(
            id,
            start - 10,
            start,
            start + 10,
            cells,
        )
        .unwrap()
    }

    fn valid_evaluation() -> ForecastCalibrationEvaluation {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1 + f64::from(index) * 0.1,
                )
            })
            .collect();
        let evaluation = vec![
            block("eval-0", 1_000, 0.3),
            block("eval-1", 1_020, 0.8),
        ];
        ForecastCalibrationEvaluation::new(
            contract(900_000, 2),
            calibration,
            evaluation,
        )
        .unwrap()
    }

    #[test]
    fn calibrates_joint_envelope_on_future_blocks() {
        let report = evaluate_forecast_calibration(&valid_evaluation()).unwrap();
        assert_eq!(report.selected_order_statistic_rank, 9);
        assert!((report.calibrated_scale_multiplier - 0.9).abs() < 1e-12);
        assert_eq!(report.joint_covered_block_count, 2);
        assert!(report.holdout_coverage_gate_passed);
        assert_eq!(report.total_cell_count, 12);
        assert_eq!(report.cell_covered_count, 12);
    }

    #[test]
    fn computes_finite_sample_rank_with_integer_coverage_arithmetic() {
        let report = evaluate_forecast_calibration(&valid_evaluation()).unwrap();
        assert_eq!(report.selected_order_statistic_rank, 9);
        assert_eq!(report.selected_calibration_block_id, "cal-8");
    }

    #[test]
    fn finite_rank_requires_nineteen_blocks_at_95_percent_and_ninety_nine_at_99() {
        let calibration_95 = (0_u32..19)
            .map(|index| {
                block(
                    &format!("cal95-{index}"),
                    100 + u64::from(index) * 20,
                    0.2,
                )
            })
            .collect();
        let input_95 = ForecastCalibrationEvaluation::new(
            contract(950_000, 1),
            calibration_95,
            vec![block("eval95", 10_000, 0.2)],
        )
        .unwrap();
        let report_95 = evaluate_forecast_calibration(&input_95).unwrap();
        assert_eq!(report_95.selected_order_statistic_rank, 19);

        let calibration_99 = (0_u32..98)
            .map(|index| {
                block(
                    &format!("cal99-{index}"),
                    100 + u64::from(index) * 20,
                    0.2,
                )
            })
            .collect();
        let input_99 = ForecastCalibrationEvaluation::new(
            contract(990_000, 1),
            calibration_99,
            vec![block("eval99", 10_000, 0.2)],
        )
        .unwrap();
        assert!(matches!(
            evaluate_forecast_calibration(&input_99),
            Err(ForecastCalibrationError::InsufficientCalibrationBlocks {
                available: 98,
                required: 99
            })
        ));
    }

    #[test]
    fn tied_calibration_scores_have_deterministic_order_statistic_identity() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("tie-{index}"),
                    100 + u64::from(index) * 20,
                    1.25,
                )
            })
            .collect();
        let evaluation = vec![block("eval-a", 1_000, 1.0), block("eval-b", 1_020, 1.0)];
        let input = ForecastCalibrationEvaluation::new(
            contract(900_000, 2),
            calibration,
            evaluation,
        )
        .unwrap();
        let report = evaluate_forecast_calibration(&input).unwrap();
        assert!((report.calibrated_scale_multiplier - 1.25).abs() < 1e-12);
        assert_eq!(report.selected_calibration_block_id, "tie-8");
    }

    #[test]
    fn refuses_insufficient_history_instead_of_using_infinite_or_maximum_radius() {
        let calibration = (0_u32..8)
            .map(|index| {
                block(
                    &format!("short-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect();
        let input = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![block("eval", 1_000, 0.1)],
        )
        .unwrap();
        assert!(matches!(
            evaluate_forecast_calibration(&input),
            Err(ForecastCalibrationError::InsufficientCalibrationBlocks {
                available: 8,
                required: 9
            })
        ));
    }

    #[test]
    fn rejects_overlapping_calibration_and_evaluation_windows() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect();
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![block("eval-overlap", 265, 0.1)],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::EvaluationNotAfterCalibration { .. })
        ));
    }

    #[test]
    fn rejects_overlapping_windows_within_a_partition() {
        let mut calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        calibration[1] = block("cal-1-overlap", 105, 0.1);
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![block("eval", 1_000, 0.1)],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::OverlappingWindows { .. })
        ));
    }

    #[test]
    fn rejects_evaluation_forecast_vintage_before_calibration_labels_are_available() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-vintage-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut evaluation_block = block("early-vintage", 1_000, 0.1);
        evaluation_block.issued_at_ns = 260;
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![evaluation_block],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::EvaluationForecastIssuedBeforeCalibration {
                ..
            })
        ));
    }

    #[test]
    fn rejects_forecast_issued_at_or_after_validity_start() {
        let result = ForecastCalibrationBlock::new(
            "late-issued",
            100,
            100,
            110,
            vec![cell("ambient", 0, 0.1)],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::ForecastIssuedAfterValidityStarted { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_block_identity_across_partitions() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut evaluation = vec![block("reused-id", 1_000, 0.1)];
        let mut calibration = calibration;
        calibration[0] = block("reused-id", 100, 0.1);
        let result =
            ForecastCalibrationEvaluation::new(contract(900_000, 1), calibration, evaluation);
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::DuplicateBlockId(_))
        ));
    }

    #[test]
    fn rejects_incomplete_channel_horizon_grid() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut incomplete = block("eval", 1_000, 0.1);
        incomplete.cells.pop();
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![incomplete],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::IncompleteCellGrid { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_channel_horizon_cell() {
        let result = ForecastCalibrationBlock::new(
            "duplicate-cell",
            10,
            20,
            30,
            vec![cell("ambient", 0, 0.1), cell("ambient", 0, 0.2)],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::DuplicateCell { .. })
        ));
    }

    #[test]
    fn rejects_undeclared_channel_and_horizon() {
        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut bad_channel = block("bad-channel", 1_000, 0.1);
        bad_channel.cells[0] = cell("unregistered", 0, 0.1);
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![bad_channel],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::UnexpectedChannel { .. })
        ));

        let calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("horizon-cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut bad_horizon = block("bad-horizon", 1_000, 0.1);
        bad_horizon.cells[0] = cell("ambient", 2, 0.1);
        let result = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![bad_horizon],
        );
        assert!(matches!(
            result,
            Err(ForecastCalibrationError::UnexpectedHorizon { .. })
        ));
    }

    #[test]
    fn rejects_invalid_scale_nonfinite_values_and_missing_evidence() {
        assert!(matches!(
            ForecastCalibrationCell::new(
                "pv", 0, 1.0, 2.0, 0.0, "scale", "forecast", "actual"
            ),
            Err(ForecastCalibrationError::NonPositiveNormalizationScale(_))
        ));
        assert!(matches!(
            ForecastCalibrationCell::new(
                "pv",
                0,
                f64::NAN,
                2.0,
                1.0,
                "scale",
                "forecast",
                "actual"
            ),
            Err(ForecastCalibrationError::NonFiniteCellValue { .. })
        ));
        assert!(matches!(
            ForecastCalibrationCell::new(
                "pv", 0, 1.0, 2.0, 1.0, "scale", "", "actual"
            ),
            Err(ForecastCalibrationError::EmptyIdentifier(_))
        ));
        assert!(matches!(
            ForecastCalibrationCell::new(
                "pv", 0, 1.0, 2.0, 1.0, "", "forecast", "actual"
            ),
            Err(ForecastCalibrationError::EmptyIdentifier(_))
        ));
    }

    #[test]
    fn one_hostile_cell_controls_the_simultaneous_block_envelope() {
        let mut calibration = (0_u32..9)
            .map(|index| {
                block(
                    &format!("cal-{index}"),
                    100 + u64::from(index) * 20,
                    0.1,
                )
            })
            .collect::<Vec<_>>();
        let mut dominant = block("cal-dominant", 300, 0.9);
        dominant.cells[3] = cell("pv", 1, 0.9);
        dominant.cells[0] = cell("ambient", 0, 0.2);
        calibration[8] = dominant;

        let mut eval = block("eval-dominant", 1_000, 0.1);
        eval.cells[4] = cell("workload", 0, 0.95);
        let input = ForecastCalibrationEvaluation::new(
            contract(900_000, 1),
            calibration,
            vec![eval],
        )
        .unwrap();
        let report = evaluate_forecast_calibration(&input).unwrap();
        assert!((report.calibrated_scale_multiplier - 0.9).abs() < 1e-12);
        assert!(!report.evaluation_receipts[0].joint_covered);
        assert!(!report.evaluation_receipts[0].cells[4].covered);
        assert!(report.evaluation_receipts[0].cells[0].covered);
    }

    #[test]
    fn holdout_miss_fails_the_joint_coverage_gate() {
        let mut input = valid_evaluation();
        input.evaluation_blocks[1] = block("eval-1", 1_020, 2.0);
        let report = evaluate_forecast_calibration(&input).unwrap();
        assert_eq!(report.joint_covered_block_count, 1);
        assert!(!report.holdout_coverage_gate_passed);
        assert_eq!(report.joint_coverage_fraction, 0.5);
    }

    #[test]
    fn input_order_does_not_change_calibration_or_evaluation_results() {
        let forward = valid_evaluation();
        let mut calibration = forward.calibration_blocks.clone();
        calibration.reverse();
        let mut evaluation = forward.evaluation_blocks.clone();
        evaluation.reverse();
        let reversed = ForecastCalibrationEvaluation::new(
            forward.contract.clone(),
            calibration,
            evaluation,
        )
        .unwrap();
        let a = evaluate_forecast_calibration(&forward).unwrap();
        let b = evaluate_forecast_calibration(&reversed).unwrap();
        assert_eq!(a.calibrated_scale_multiplier, b.calibrated_scale_multiplier);
        assert_eq!(a.selected_calibration_block_id, b.selected_calibration_block_id);
        assert_eq!(a.joint_covered_block_count, b.joint_covered_block_count);
        assert_eq!(a.calibration_receipts, b.calibration_receipts);
        assert_eq!(a.evaluation_receipts, b.evaluation_receipts);
    }
}
