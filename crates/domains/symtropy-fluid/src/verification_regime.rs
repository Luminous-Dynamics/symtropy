// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Cross-resolution verification summaries without promotion verdicts.
//!
//! A smaller finest-grid error alone does not establish an asymptotic regime.
//! This module summarizes retained refinement evidence using raw scale/error
//! points, monotonicity counts, adjacent refinement ratios, and an all-point
//! log-log fit. It intentionally does not compare errors from different
//! executable fixtures merely because they share units. No acceptance threshold
//! or asymptotic-regime verdict is defined here.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::verification_ladder::{RefinementAxis, VerificationCaseKind, VerificationLadderReport};

pub const REFINEMENT_TREND_SCHEMA_ID: &str = "continuum-refinement-trend-v0.1";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefinementMetric {
    VelocityRmsError,
    KineticEnergyRelativeError,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefinementTrendPoint {
    pub ladder_index: usize,
    /// `h` for spatial refinement and `dt` for temporal refinement.
    pub refinement_scale: f64,
    pub error: f64,
}

/// Least-squares fit of `ln(error) = intercept + p * ln(scale)` across every
/// strictly positive retained point. The fit is descriptive evidence only.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LogLogRefinementFit {
    pub point_count: usize,
    pub apparent_order: f64,
    pub log_intercept: f64,
    pub r_squared: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefinementTrendReport {
    pub schema_id: String,
    pub case_kind: VerificationCaseKind,
    pub refinement_axis: RefinementAxis,
    pub metric: RefinementMetric,
    pub points: Vec<RefinementTrendPoint>,
    pub adjacent_refinement_ratios: Vec<f64>,
    pub minimum_adjacent_refinement_ratio: Option<f64>,
    pub maximum_adjacent_refinement_ratio: Option<f64>,
    pub error_decrease_count: usize,
    pub error_equal_count: usize,
    pub error_increase_count: usize,
    pub adjacent_order_minimum: Option<f64>,
    pub adjacent_order_maximum: Option<f64>,
    pub log_log_fit: Option<LogLogRefinementFit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerificationRegimeError {
    TooFewLadderPoints,
    InvalidRefinementScale,
    InvalidError,
    NonRefiningScale,
    NonFiniteDerivedMetric,
}

impl fmt::Display for VerificationRegimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewLadderPoints => write!(f, "refinement trend requires at least two points"),
            Self::InvalidRefinementScale => {
                write!(f, "refinement scale must be finite and strictly positive")
            }
            Self::InvalidError => write!(f, "refinement error must be finite and non-negative"),
            Self::NonRefiningScale => {
                write!(f, "ladder scale must strictly decrease with refinement")
            }
            Self::NonFiniteDerivedMetric => {
                write!(
                    f,
                    "verification summary produced a non-finite derived metric"
                )
            }
        }
    }
}

impl std::error::Error for VerificationRegimeError {}

pub fn summarize_refinement_trend(
    ladder: &VerificationLadderReport,
    metric: RefinementMetric,
) -> Result<RefinementTrendReport, VerificationRegimeError> {
    if ladder.points.len() < 2 {
        return Err(VerificationRegimeError::TooFewLadderPoints);
    }

    let mut points = Vec::with_capacity(ladder.points.len());
    for (ladder_index, point) in ladder.points.iter().enumerate() {
        let refinement_scale = match ladder.refinement_axis {
            RefinementAxis::Spatial => point.minimum_resolved_length_m,
            RefinementAxis::Temporal => point.dt_s,
        };
        if !refinement_scale.is_finite() || refinement_scale <= 0.0 {
            return Err(VerificationRegimeError::InvalidRefinementScale);
        }
        let error = match metric {
            RefinementMetric::VelocityRmsError => point.velocity_rms_error_mps,
            RefinementMetric::KineticEnergyRelativeError => point.kinetic_energy_relative_error,
        };
        if !error.is_finite() || error < 0.0 {
            return Err(VerificationRegimeError::InvalidError);
        }
        points.push(RefinementTrendPoint {
            ladder_index,
            refinement_scale,
            error,
        });
    }

    let mut adjacent_refinement_ratios = Vec::with_capacity(points.len() - 1);
    let mut error_decrease_count = 0;
    let mut error_equal_count = 0;
    let mut error_increase_count = 0;
    for pair in points.windows(2) {
        if pair[1].refinement_scale >= pair[0].refinement_scale {
            return Err(VerificationRegimeError::NonRefiningScale);
        }
        let ratio = pair[0].refinement_scale / pair[1].refinement_scale;
        if !ratio.is_finite() {
            return Err(VerificationRegimeError::NonFiniteDerivedMetric);
        }
        adjacent_refinement_ratios.push(ratio);
        match pair[1].error.total_cmp(&pair[0].error) {
            std::cmp::Ordering::Less => error_decrease_count += 1,
            std::cmp::Ordering::Equal => error_equal_count += 1,
            std::cmp::Ordering::Greater => error_increase_count += 1,
        }
    }

    let minimum_adjacent_refinement_ratio =
        adjacent_refinement_ratios.iter().copied().reduce(f64::min);
    let maximum_adjacent_refinement_ratio =
        adjacent_refinement_ratios.iter().copied().reduce(f64::max);

    let adjacent_orders = ladder
        .adjacent_observed_orders
        .iter()
        .filter_map(|pair| match metric {
            RefinementMetric::VelocityRmsError => pair.velocity_rms_order,
            RefinementMetric::KineticEnergyRelativeError => pair.kinetic_energy_order,
        })
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let adjacent_order_minimum = adjacent_orders.iter().copied().reduce(f64::min);
    let adjacent_order_maximum = adjacent_orders.iter().copied().reduce(f64::max);
    let log_log_fit = fit_positive_log_log(&points)?;

    Ok(RefinementTrendReport {
        schema_id: REFINEMENT_TREND_SCHEMA_ID.to_owned(),
        case_kind: ladder.case_kind,
        refinement_axis: ladder.refinement_axis,
        metric,
        points,
        adjacent_refinement_ratios,
        minimum_adjacent_refinement_ratio,
        maximum_adjacent_refinement_ratio,
        error_decrease_count,
        error_equal_count,
        error_increase_count,
        adjacent_order_minimum,
        adjacent_order_maximum,
        log_log_fit,
    })
}

fn fit_positive_log_log(
    points: &[RefinementTrendPoint],
) -> Result<Option<LogLogRefinementFit>, VerificationRegimeError> {
    if points.iter().any(|point| point.error == 0.0) {
        return Ok(None);
    }
    let n = points.len() as f64;
    let mean_x = points
        .iter()
        .map(|point| point.refinement_scale.ln())
        .sum::<f64>()
        / n;
    let mean_y = points.iter().map(|point| point.error.ln()).sum::<f64>() / n;
    let mut covariance = 0.0;
    let mut variance_x = 0.0;
    let mut variance_y = 0.0;
    for point in points {
        let dx = point.refinement_scale.ln() - mean_x;
        let dy = point.error.ln() - mean_y;
        covariance += dx * dy;
        variance_x += dx * dx;
        variance_y += dy * dy;
    }
    if variance_x == 0.0 {
        return Ok(None);
    }
    let apparent_order = covariance / variance_x;
    let log_intercept = mean_y - apparent_order * mean_x;
    if !apparent_order.is_finite() || !log_intercept.is_finite() {
        return Err(VerificationRegimeError::NonFiniteDerivedMetric);
    }
    let r_squared = if variance_y > 0.0 {
        let value = covariance * covariance / (variance_x * variance_y);
        if !value.is_finite() {
            return Err(VerificationRegimeError::NonFiniteDerivedMetric);
        }
        Some(value.clamp(0.0, 1.0))
    } else {
        None
    };
    Ok(Some(LogLogRefinementFit {
        point_count: points.len(),
        apparent_order,
        log_intercept,
        r_squared,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verification_ladder::{ObservedOrderPair, VerificationLadderPoint};

    fn ladder(axis: RefinementAxis, errors: &[f64]) -> VerificationLadderReport {
        let points = errors
            .iter()
            .enumerate()
            .map(|(index, error)| {
                let scale = 1.0 / (1usize << index) as f64;
                VerificationLadderPoint {
                    solver_profile: format!("solver-{index}"),
                    manufactured_profile: Some("manufactured".to_owned()),
                    resolution: 8usize << index,
                    steps: 8usize << index,
                    minimum_resolved_length_m: scale,
                    dt_s: scale,
                    final_time_s: 1.0,
                    velocity_rms_error_mps: *error,
                    velocity_max_error_mps: *error,
                    kinetic_energy_relative_error: *error,
                    maximum_observed_advective_cfl: 0.1,
                    maximum_observed_divergence_rms_per_s: 0.0,
                    maximum_observed_pressure_residual_rms_pa_per_m2: 0.0,
                }
            })
            .collect::<Vec<_>>();
        let adjacent_observed_orders = (0..errors.len() - 1)
            .map(|index| ObservedOrderPair {
                coarse_index: index,
                fine_index: index + 1,
                refinement_ratio: 2.0,
                velocity_rms_order: Some(2.0),
                kinetic_energy_order: Some(2.0),
            })
            .collect();
        VerificationLadderReport {
            schema_id: "test".to_owned(),
            case_kind: VerificationCaseKind::ManufacturedTaylorGreen,
            refinement_axis: axis,
            points,
            adjacent_observed_orders,
        }
    }

    #[test]
    fn exact_second_order_sequence_has_second_order_global_fit() {
        let report = ladder(RefinementAxis::Spatial, &[1.0, 0.25, 0.0625]);
        let trend =
            summarize_refinement_trend(&report, RefinementMetric::VelocityRmsError).unwrap();
        let fit = trend.log_log_fit.unwrap();
        assert!((fit.apparent_order - 2.0).abs() < 1.0e-12);
        assert!((fit.r_squared.unwrap() - 1.0).abs() < 1.0e-12);
        assert_eq!(trend.error_decrease_count, 2);
        assert_eq!(trend.error_increase_count, 0);
    }

    #[test]
    fn non_monotone_error_is_retained_not_reclassified() {
        let report = ladder(RefinementAxis::Spatial, &[1.0, 0.5, 0.6]);
        let trend =
            summarize_refinement_trend(&report, RefinementMetric::VelocityRmsError).unwrap();
        assert_eq!(trend.error_decrease_count, 1);
        assert_eq!(trend.error_increase_count, 1);
    }

    #[test]
    fn zero_error_disables_log_fit_instead_of_fabricating_epsilon() {
        let report = ladder(RefinementAxis::Temporal, &[1.0, 0.25, 0.0]);
        let trend =
            summarize_refinement_trend(&report, RefinementMetric::VelocityRmsError).unwrap();
        assert_eq!(trend.log_log_fit, None);
    }
}
