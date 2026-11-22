//! Group-wise finite-sample calibration of positive NLOS excess-path bias.
//!
//! This module consumes evaluator-derived positive range residuals grouped by
//! predeclared independent calibration units (e.g. independent experiment or
//! environment campaigns). It collapses each unit to its maximum residual,
//! then selects a one-sided finite-sample order statistic across units.
//!
//! The resulting bound may be exported to controller code as
//! PositiveNlosBiasBound. Raw truth residuals and labels do not appear in that
//! controller-facing type.
//!
//! The coverage interpretation requires exchangeability of the independent
//! calibration units. It does not provide conditional coverage for arbitrary
//! obstacle materials, anchors, distances, environments or domain shifts.
//! Group definitions and the calibration split must be frozen before viewing
//! the residuals.

use crate::ds_twr_integrity::PositiveNlosBiasBound;
use std::collections::HashSet;
use std::fmt;

pub const ALPHA_PARTS_PER_MILLION: u32 = 1_000_000;
pub const GROUPED_POSITIVE_NLOS_BIAS_SCHEMA_VERSION: &str =
    "grouped-positive-nlos-bias/v0.1";

/// Evaluator-side residuals for one predeclared independent calibration unit.
///
/// A unit may contain many correlated measurements, but contributes only its
/// largest positive excess-path residual to the calibration order statistic.
#[derive(Clone, Debug, PartialEq)]
pub struct PositiveNlosBiasCalibrationUnit {
    pub independent_unit_id: u64,
    /// Evaluator-derived measured_range - independent_truth_range residuals.
    /// Negative / unknown-sign errors must not be passed into this
    /// positive-only calibration method.
    pub positive_bias_residuals_m: Vec<f64>,
}

/// Calibration configuration pinned before examining evaluator residuals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupedPositiveNlosBiasCalibrationConfig {
    /// Requested one-sided miscoverage in parts per million.
    /// 50_000 represents alpha=0.05.
    pub alpha_parts_per_million: u32,
    /// Stable identity of the calibration domain (environment/material/device
    /// scope) for which the caller is asserting group exchangeability.
    pub calibration_domain_id: u64,
    /// SHA-256 commitment to the immutable source payload inventory.
    pub dataset_sha256: [u8; 32],
    /// SHA-256 commitment to the frozen calibration split manifest.
    pub calibration_split_sha256: [u8; 32],
}

/// The resulting finite-sample order-statistic receipt. This object records
/// scope and derivation; it is not a general-domain safety certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct GroupedPositiveNlosBiasCalibrationReceipt {
    pub schema_version: &'static str,
    pub calibration_domain_id: u64,
    pub alpha_parts_per_million: u32,
    pub independent_group_count: usize,
    pub evaluator_observation_count: usize,
    /// One-based order-statistic rank selected from the sorted group maxima.
    pub selected_order_statistic_rank: usize,
    /// Sorted maximum residual for the selected rank.
    pub maximum_positive_bias_m: f64,
    pub maximum_group_residual_m: f64,
    pub dataset_sha256: [u8; 32],
    pub calibration_split_sha256: [u8; 32],
}

impl GroupedPositiveNlosBiasCalibrationReceipt {
    /// Export the result for the controller-side interval kernel.
    ///
    /// The minimum is zero because this method calibrates only a nonnegative
    /// upper bound. A separate lower-bound model would need its own validated
    /// signed-error assumptions.
    pub fn to_controller_bound(&self) -> PositiveNlosBiasBound {
        PositiveNlosBiasBound {
            minimum_bias_m: 0.0,
            maximum_bias_m: self.maximum_positive_bias_m,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GroupedPositiveNlosBiasCalibrationError {
    InvalidAlpha,
    InvalidCalibrationDomain,
    MissingDatasetCommitment,
    MissingCalibrationSplitCommitment,
    EmptyCalibrationUnits,
    DuplicateIndependentUnit(u64),
    EmptyIndependentUnit(u64),
    InvalidIndependentUnitIdentity,
    NegativeOrNonFiniteBiasResidual { unit_id: u64, row_index: usize },
    ArithmeticOverflow,
    InsufficientIndependentGroups { required_rank: usize, available_groups: usize },
}

impl fmt::Display for GroupedPositiveNlosBiasCalibrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidAlpha => write!(f, "alpha must be strictly between zero and one"),
            Self::InvalidCalibrationDomain => write!(f, "calibration domain ID must be nonzero"),
            Self::MissingDatasetCommitment => write!(f, "dataset SHA-256 commitment is missing"),
            Self::MissingCalibrationSplitCommitment => {
                write!(f, "calibration split SHA-256 commitment is missing")
            }
            Self::EmptyCalibrationUnits => write!(f, "at least one independent calibration unit is required"),
            Self::DuplicateIndependentUnit(id) => {
                write!(f, "independent calibration unit appears more than once: {id}")
            }
            Self::EmptyIndependentUnit(id) => {
                write!(f, "independent calibration unit has no residual samples: {id}")
            }
            Self::InvalidIndependentUnitIdentity => {
                write!(f, "independent calibration unit identity must be nonzero")
            }
            Self::NegativeOrNonFiniteBiasResidual { unit_id, row_index } => {
                write!(f, "positive NLOS residual is negative or non-finite in unit {unit_id}, row {row_index}")
            }
            Self::ArithmeticOverflow => write!(f, "grouped calibration arithmetic overflowed"),
            Self::InsufficientIndependentGroups { required_rank, available_groups } => {
                write!(f, "requested one-sided coverage requires order statistic {required_rank}, but only {available_groups} independent groups are available")
            }
        }
    }
}

impl std::error::Error for GroupedPositiveNlosBiasCalibrationError {}

/// Calibrate a positive NLOS excess-path upper bound using one maximum score
/// per predeclared independent unit.
///
/// The selected rank is:
/// ceil((n + 1) * (1 - alpha)).
/// If that rank exceeds n, there are not enough independent calibration units
/// for the requested finite bound and the function refuses to extrapolate.
pub fn calibrate_grouped_positive_nlos_bias_upper_bound(
    config: GroupedPositiveNlosBiasCalibrationConfig,
    units: &[PositiveNlosBiasCalibrationUnit],
) -> Result<GroupedPositiveNlosBiasCalibrationReceipt, GroupedPositiveNlosBiasCalibrationError> {
    if !(1..ALPHA_PARTS_PER_MILLION).contains(&config.alpha_parts_per_million) {
        return Err(GroupedPositiveNlosBiasCalibrationError::InvalidAlpha);
    }
    if config.calibration_domain_id == 0 {
        return Err(GroupedPositiveNlosBiasCalibrationError::InvalidCalibrationDomain);
    }
    if config.dataset_sha256.iter().all(|byte| *byte == 0) {
        return Err(GroupedPositiveNlosBiasCalibrationError::MissingDatasetCommitment);
    }
    if config
        .calibration_split_sha256
        .iter()
        .all(|byte| *byte == 0)
    {
        return Err(GroupedPositiveNlosBiasCalibrationError::MissingCalibrationSplitCommitment);
    }
    if units.is_empty() {
        return Err(GroupedPositiveNlosBiasCalibrationError::EmptyCalibrationUnits);
    }

    let mut seen_units = HashSet::with_capacity(units.len());
    let mut group_maxima = Vec::with_capacity(units.len());
    let mut observation_count = 0_usize;

    for unit in units {
        if unit.independent_unit_id == 0 {
            return Err(GroupedPositiveNlosBiasCalibrationError::InvalidIndependentUnitIdentity);
        }
        if !seen_units.insert(unit.independent_unit_id) {
            return Err(GroupedPositiveNlosBiasCalibrationError::DuplicateIndependentUnit(
                unit.independent_unit_id,
            ));
        }
        if unit.positive_bias_residuals_m.is_empty() {
            return Err(GroupedPositiveNlosBiasCalibrationError::EmptyIndependentUnit(
                unit.independent_unit_id,
            ));
        }
        observation_count = observation_count
            .checked_add(unit.positive_bias_residuals_m.len())
            .ok_or(GroupedPositiveNlosBiasCalibrationError::ArithmeticOverflow)?;

        let mut group_max = 0.0_f64;
        for (row_index, residual) in unit.positive_bias_residuals_m.iter().copied().enumerate() {
            if !residual.is_finite() || residual < 0.0 {
                return Err(
                    GroupedPositiveNlosBiasCalibrationError::NegativeOrNonFiniteBiasResidual {
                        unit_id: unit.independent_unit_id,
                        row_index,
                    },
                );
            }
            group_max = group_max.max(residual);
        }
        group_maxima.push(group_max);
    }

    let n = group_maxima.len() as u128;
    let numerator = (n + 1)
        .checked_mul(
            (ALPHA_PARTS_PER_MILLION - config.alpha_parts_per_million) as u128,
        )
        .ok_or(GroupedPositiveNlosBiasCalibrationError::ArithmeticOverflow)?;
    let denominator = ALPHA_PARTS_PER_MILLION as u128;
    let selected_rank = div_ceil(numerator, denominator);
    if selected_rank == 0 || selected_rank > n {
        let required_rank = usize::try_from(selected_rank)
            .map_err(|_| GroupedPositiveNlosBiasCalibrationError::ArithmeticOverflow)?;
        return Err(
            GroupedPositiveNlosBiasCalibrationError::InsufficientIndependentGroups {
                required_rank,
                available_groups: units.len(),
            },
        );
    }

    group_maxima.sort_by(f64::total_cmp);
    let maximum_positive_bias_m = group_maxima[selected_rank as usize - 1];
    let maximum_group_residual_m = *group_maxima
        .last()
        .ok_or(GroupedPositiveNlosBiasCalibrationError::EmptyCalibrationUnits)?;

    Ok(GroupedPositiveNlosBiasCalibrationReceipt {
        schema_version: GROUPED_POSITIVE_NLOS_BIAS_SCHEMA_VERSION,
        calibration_domain_id: config.calibration_domain_id,
        alpha_parts_per_million: config.alpha_parts_per_million,
        independent_group_count: units.len(),
        evaluator_observation_count: observation_count,
        selected_order_statistic_rank: selected_rank as usize,
        maximum_positive_bias_m,
        maximum_group_residual_m,
        dataset_sha256: config.dataset_sha256,
        calibration_split_sha256: config.calibration_split_sha256,
    })
}

fn div_ceil(numerator: u128, denominator: u128) -> u128 {
    numerator.div_ceil(denominator)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(alpha_parts_per_million: u32) -> GroupedPositiveNlosBiasCalibrationConfig {
        GroupedPositiveNlosBiasCalibrationConfig {
            alpha_parts_per_million,
            calibration_domain_id: 7,
            dataset_sha256: [1_u8; 32],
            calibration_split_sha256: [2_u8; 32],
        }
    }

    fn unit(id: u64, residuals: &[f64]) -> PositiveNlosBiasCalibrationUnit {
        PositiveNlosBiasCalibrationUnit {
            independent_unit_id: id,
            positive_bias_residuals_m: residuals.to_vec(),
        }
    }

    #[test]
    fn calibrates_95_percent_group_level_upper_bound_with_exact_rank() {
        let units: Vec<_> = (1..=19)
            .map(|id| unit(id, &[id as f64]))
            .collect();
        let receipt =
            calibrate_grouped_positive_nlos_bias_upper_bound(config(50_000), &units).unwrap();
        assert_eq!(receipt.independent_group_count, 19);
        assert_eq!(receipt.evaluator_observation_count, 19);
        assert_eq!(receipt.selected_order_statistic_rank, 19);
        assert_eq!(receipt.maximum_positive_bias_m, 19.0);
        assert_eq!(receipt.maximum_group_residual_m, 19.0);
        assert_eq!(receipt.to_controller_bound().maximum_bias_m, 19.0);
    }

    #[test]
    fn aggregates_by_group_maximum_instead_of_row_weighting() {
        let mut units = vec![unit(1, &[0.1; 100]), unit(2, &[2.0])];
        units.extend((3..=19).map(|id| unit(id, &[id as f64])));
        let receipt =
            calibrate_grouped_positive_nlos_bias_upper_bound(config(50_000), &units).unwrap();
        // There are 119 observations but only 19 calibration units. The
        // selected statistic is computed from 19 group maxima.
        assert_eq!(receipt.evaluator_observation_count, 118);
        assert_eq!(receipt.independent_group_count, 19);
        assert_eq!(receipt.selected_order_statistic_rank, 19);
        assert_eq!(receipt.maximum_positive_bias_m, 19.0);
    }

    #[test]
    fn refuses_95_percent_bound_with_only_eighteen_independent_units() {
        let units: Vec<_> = (1..=18)
            .map(|id| unit(id, &[id as f64]))
            .collect();
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(config(50_000), &units),
            Err(
                GroupedPositiveNlosBiasCalibrationError::InsufficientIndependentGroups {
                    required_rank: 19,
                    available_groups: 18,
                }
            )
        );
    }

    #[test]
    fn supports_lower_confidence_target_when_the_rank_is_available() {
        let units: Vec<_> = (1..=9)
            .map(|id| unit(id, &[id as f64]))
            .collect();
        let receipt =
            calibrate_grouped_positive_nlos_bias_upper_bound(config(100_000), &units).unwrap();
        assert_eq!(receipt.selected_order_statistic_rank, 9);
        assert_eq!(receipt.maximum_positive_bias_m, 9.0);
    }

    #[test]
    fn duplicate_independent_units_are_rejected() {
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(
                config(50_000),
                &[unit(1, &[1.0]), unit(1, &[2.0])]
            ),
            Err(GroupedPositiveNlosBiasCalibrationError::DuplicateIndependentUnit(1))
        );
    }

    #[test]
    fn negative_or_nonfinite_positive_bias_residuals_are_rejected() {
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(
                config(50_000),
                &[unit(1, &[1.0, -0.1])]
            ),
            Err(
                GroupedPositiveNlosBiasCalibrationError::NegativeOrNonFiniteBiasResidual {
                    unit_id: 1,
                    row_index: 1,
                }
            )
        );
        assert!(matches!(
            calibrate_grouped_positive_nlos_bias_upper_bound(
                config(50_000),
                &[unit(1, &[f64::NAN])]
            ),
            Err(GroupedPositiveNlosBiasCalibrationError::NegativeOrNonFiniteBiasResidual { .. })
        ));
    }

    #[test]
    fn rejects_missing_dataset_or_split_commitments_and_invalid_domain() {
        let mut cfg = config(50_000);
        cfg.dataset_sha256 = [0; 32];
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(cfg, &[unit(1, &[1.0])]),
            Err(GroupedPositiveNlosBiasCalibrationError::MissingDatasetCommitment)
        );

        let mut cfg = config(50_000);
        cfg.calibration_split_sha256 = [0; 32];
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(cfg, &[unit(1, &[1.0])]),
            Err(GroupedPositiveNlosBiasCalibrationError::MissingCalibrationSplitCommitment)
        );

        let mut cfg = config(50_000);
        cfg.calibration_domain_id = 0;
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(cfg, &[unit(1, &[1.0])]),
            Err(GroupedPositiveNlosBiasCalibrationError::InvalidCalibrationDomain)
        );
    }

    #[test]
    fn invalid_alpha_empty_units_and_empty_independent_unit_fail_closed() {
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(config(0), &[unit(1, &[1.0])]),
            Err(GroupedPositiveNlosBiasCalibrationError::InvalidAlpha)
        );
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(config(50_000), &[]),
            Err(GroupedPositiveNlosBiasCalibrationError::EmptyCalibrationUnits)
        );
        assert_eq!(
            calibrate_grouped_positive_nlos_bias_upper_bound(config(50_000), &[unit(1, &[])]),
            Err(GroupedPositiveNlosBiasCalibrationError::EmptyIndependentUnit(1))
        );
    }
}
