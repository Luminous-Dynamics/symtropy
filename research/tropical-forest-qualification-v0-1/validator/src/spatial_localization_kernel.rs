// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Experimental distance-localized residual intervals for point-referenced forest data.
//!
//! Gaussian proximity weights are a diagnostic localization heuristic, not density
//! ratios. This module deliberately makes no distribution-free, exchangeability, or
//! conditional-coverage guarantee. Compare it empirically with pooled/block conformal
//! and specialist spatial methods such as spconform on matched held-out supports.

use crate::conformal_kernel::split_conformal_radius;
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpatialLocalizationError {
    EmptyCoordinateSystem,
    InvalidAlpha,
    InvalidBandwidth,
    InvalidEffectiveSampleThreshold,
    EmptyCalibration,
    EmptyTestSet,
    EmptyIdentity,
    DuplicateIdentity,
    NonFiniteCoordinate,
    InvalidResidual,
    NonFinitePredictionOrObservation,
    NumericOverflow,
}

impl fmt::Display for SpatialLocalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SpatialLocalizationError {}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialCalibrationPoint {
    pub point_id: String,
    /// Coordinates in metres in the scenario's declared projected metric CRS.
    pub x_m: f64,
    pub y_m: f64,
    /// Absolute out-of-sample residual from the frozen base predictor.
    pub absolute_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialTestPoint {
    pub point_id: String,
    /// Coordinates in metres in the scenario's declared projected metric CRS.
    pub x_m: f64,
    pub y_m: f64,
    pub point_prediction: f64,
    pub observation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialLocalizationScenario {
    /// Non-empty exact identity of one projected metric coordinate system.
    /// CRS transformation is intentionally outside this bounded kernel.
    pub coordinate_system_id: String,
    pub bandwidth_m: f64,
    pub alpha: f64,
    /// Refuse finite local intervals if the weighted ESS is below this threshold.
    pub minimum_effective_sample_size: f64,
    pub calibration_points: Vec<SpatialCalibrationPoint>,
    pub test_points: Vec<SpatialTestPoint>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LocalizedPredictionResult {
    pub point_id: String,
    pub calibration_count: usize,
    pub weighted_calibration_mass: f64,
    pub effective_sample_size: f64,
    pub maximum_normalized_weight: f64,
    pub nearest_calibration_distance_m: f64,
    pub radius: Option<f64>,
    pub interval_lower: Option<f64>,
    pub interval_upper: Option<f64>,
    pub interval_width: Option<f64>,
    /// None means no finite local interval was issued; it is not counted as covered.
    pub covered: Option<bool>,
    pub status: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SpatialLocalizationResult {
    pub coordinate_system_id: String,
    pub bandwidth_m: f64,
    pub alpha: f64,
    /// Standard pooled split-conformal comparator over the same residual records.
    /// This comparator itself assumes its own exchangeability conditions.
    pub pooled_radius: Option<f64>,
    pub predictions: Vec<LocalizedPredictionResult>,
    pub method_status: &'static str,
}

fn validate_coordinate(x: f64, y: f64) -> Result<(), SpatialLocalizationError> {
    if !x.is_finite() || !y.is_finite() {
        return Err(SpatialLocalizationError::NonFiniteCoordinate);
    }
    Ok(())
}

fn gaussian_weight(distance_m: f64, bandwidth_m: f64) -> Result<f64, SpatialLocalizationError> {
    let scaled = distance_m / bandwidth_m;
    if !scaled.is_finite() {
        return Err(SpatialLocalizationError::NumericOverflow);
    }
    let exponent = -0.5 * scaled * scaled;
    // Underflow to zero is legitimate: the point has no numerically meaningful
    // influence at this bandwidth. If every point underflows, the interval is unbounded.
    Ok(exponent.exp())
}

fn localized_radius(
    residuals_and_weights: &[(f64, f64)],
    alpha: f64,
) -> Result<Option<f64>, SpatialLocalizationError> {
    let total_weight: f64 = residuals_and_weights.iter().map(|(_, weight)| weight).sum();
    if !total_weight.is_finite() {
        return Err(SpatialLocalizationError::NumericOverflow);
    }
    if total_weight <= 0.0 {
        return Ok(None);
    }

    // Weighted split-conformal-style correction adds test-point mass at infinity.
    // This is used as a conservative diagnostic construction only: Gaussian similarity
    // weights are not justified density ratios and do not confer a validity guarantee.
    let target_mass = (1.0 - alpha) * (total_weight + 1.0);
    if !target_mass.is_finite() {
        return Err(SpatialLocalizationError::NumericOverflow);
    }
    if target_mass > total_weight {
        return Ok(None);
    }

    let mut ordered = residuals_and_weights.to_vec();
    ordered.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut cumulative = 0.0;
    for (residual, weight) in ordered {
        cumulative += weight;
        if !cumulative.is_finite() {
            return Err(SpatialLocalizationError::NumericOverflow);
        }
        if cumulative >= target_mass {
            return Ok(Some(residual));
        }
    }
    Ok(None)
}

/// Evaluate experimental Gaussian-kernel localized residual intervals.
///
/// Coordinates must already share the exact declared projected metric CRS. The kernel
/// refuses missing/invalid values and duplicated identities. It returns pooled split
/// conformal on the same residuals as a comparator, local ESS and the maximum normalized
/// weight for every prediction. A low-ESS or low-mass locality receives no finite interval.
pub fn evaluate_spatial_localization(
    scenario: &SpatialLocalizationScenario,
) -> Result<SpatialLocalizationResult, SpatialLocalizationError> {
    if scenario.coordinate_system_id.trim().is_empty() {
        return Err(SpatialLocalizationError::EmptyCoordinateSystem);
    }
    if !scenario.alpha.is_finite() || scenario.alpha <= 0.0 || scenario.alpha >= 1.0 {
        return Err(SpatialLocalizationError::InvalidAlpha);
    }
    if !scenario.bandwidth_m.is_finite() || scenario.bandwidth_m <= 0.0 {
        return Err(SpatialLocalizationError::InvalidBandwidth);
    }
    if !scenario.minimum_effective_sample_size.is_finite()
        || scenario.minimum_effective_sample_size < 0.0
    {
        return Err(SpatialLocalizationError::InvalidEffectiveSampleThreshold);
    }
    if scenario.calibration_points.is_empty() {
        return Err(SpatialLocalizationError::EmptyCalibration);
    }
    if scenario.test_points.is_empty() {
        return Err(SpatialLocalizationError::EmptyTestSet);
    }

    let mut identities = BTreeSet::new();
    let mut residuals = Vec::with_capacity(scenario.calibration_points.len());
    for point in &scenario.calibration_points {
        if point.point_id.trim().is_empty() {
            return Err(SpatialLocalizationError::EmptyIdentity);
        }
        if !identities.insert(point.point_id.as_str()) {
            return Err(SpatialLocalizationError::DuplicateIdentity);
        }
        validate_coordinate(point.x_m, point.y_m)?;
        if !point.absolute_residual.is_finite() || point.absolute_residual < 0.0 {
            return Err(SpatialLocalizationError::InvalidResidual);
        }
        residuals.push(point.absolute_residual);
    }

    for point in &scenario.test_points {
        if point.point_id.trim().is_empty() {
            return Err(SpatialLocalizationError::EmptyIdentity);
        }
        if !identities.insert(point.point_id.as_str()) {
            return Err(SpatialLocalizationError::DuplicateIdentity);
        }
        validate_coordinate(point.x_m, point.y_m)?;
        if !point.point_prediction.is_finite() || !point.observation.is_finite() {
            return Err(SpatialLocalizationError::NonFinitePredictionOrObservation);
        }
    }

    let pooled_radius = split_conformal_radius(&residuals, scenario.alpha)
        .map_err(|_| SpatialLocalizationError::InvalidResidual)?;

    let mut predictions = Vec::with_capacity(scenario.test_points.len());
    for test in &scenario.test_points {
        let mut local = Vec::with_capacity(scenario.calibration_points.len());
        let mut nearest_distance = f64::INFINITY;
        let mut weight_sum = 0.0;
        let mut squared_weight_sum = 0.0;
        let mut maximum_weight: f64 = 0.0;

        for calibration in &scenario.calibration_points {
            let dx = test.x_m - calibration.x_m;
            let dy = test.y_m - calibration.y_m;
            let distance = dx.hypot(dy);
            if !distance.is_finite() {
                return Err(SpatialLocalizationError::NumericOverflow);
            }
            nearest_distance = nearest_distance.min(distance);
            let weight = gaussian_weight(distance, scenario.bandwidth_m)?;
            local.push((calibration.absolute_residual, weight));
            weight_sum += weight;
            squared_weight_sum += weight * weight;
            maximum_weight = maximum_weight.max(weight);
        }

        if !weight_sum.is_finite() || !squared_weight_sum.is_finite() {
            return Err(SpatialLocalizationError::NumericOverflow);
        }

        let effective_sample_size = if squared_weight_sum > 0.0 {
            weight_sum * weight_sum / squared_weight_sum
        } else {
            0.0
        };
        let maximum_normalized_weight = if weight_sum > 0.0 {
            maximum_weight / weight_sum
        } else {
            0.0
        };
        if !effective_sample_size.is_finite() || !maximum_normalized_weight.is_finite() {
            return Err(SpatialLocalizationError::NumericOverflow);
        }

        let radius = if effective_sample_size < scenario.minimum_effective_sample_size {
            None
        } else {
            localized_radius(&local, scenario.alpha)?
        };
        let (interval_lower, interval_upper, interval_width, covered, status) =
            if let Some(radius) = radius {
                let lower = test.point_prediction - radius;
                let upper = test.point_prediction + radius;
                if !lower.is_finite() || !upper.is_finite() {
                    return Err(SpatialLocalizationError::NumericOverflow);
                }
                let width = 2.0 * radius;
                if !width.is_finite() {
                    return Err(SpatialLocalizationError::NumericOverflow);
                }
                (
                    Some(lower),
                    Some(upper),
                    Some(width),
                    Some((test.observation - test.point_prediction).abs() <= radius),
                    "FINITE_LOCAL_INTERVAL",
                )
            } else if effective_sample_size < scenario.minimum_effective_sample_size {
                (None, None, None, None, "ABSTAIN_LOW_LOCAL_ESS")
            } else {
                (None, None, None, None, "ABSTAIN_INSUFFICIENT_LOCAL_CALIBRATION_MASS")
            };

        predictions.push(LocalizedPredictionResult {
            point_id: test.point_id.clone(),
            calibration_count: scenario.calibration_points.len(),
            weighted_calibration_mass: weight_sum,
            effective_sample_size,
            maximum_normalized_weight,
            nearest_calibration_distance_m: nearest_distance,
            radius,
            interval_lower,
            interval_upper,
            interval_width,
            covered,
            status,
        });
    }

    Ok(SpatialLocalizationResult {
        coordinate_system_id: scenario.coordinate_system_id.clone(),
        bandwidth_m: scenario.bandwidth_m,
        alpha: scenario.alpha,
        pooled_radius,
        predictions,
        method_status: "EXPERIMENTAL_LOCALIZED_DIAGNOSTIC_NO_GUARANTEE",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn calibration(id: &str, x_m: f64, residual: f64) -> SpatialCalibrationPoint {
        SpatialCalibrationPoint {
            point_id: id.to_string(),
            x_m,
            y_m: 0.0,
            absolute_residual: residual,
        }
    }

    fn test_point(id: &str, x_m: f64, observation: f64) -> SpatialTestPoint {
        SpatialTestPoint {
            point_id: id.to_string(),
            x_m,
            y_m: 0.0,
            point_prediction: 100.0,
            observation,
        }
    }

    fn scenario() -> SpatialLocalizationScenario {
        SpatialLocalizationScenario {
            coordinate_system_id: "LOCAL_PROJECTED_METRES_A".to_string(),
            bandwidth_m: 10.0,
            alpha: 0.1,
            minimum_effective_sample_size: 2.0,
            calibration_points: (0..10)
                .map(|index| calibration(&format!("a-{index}"), 0.0, 1.0))
                .collect(),
            test_points: vec![test_point("test-a", 0.0, 101.0)],
        }
    }

    #[test]
    fn co_located_points_return_finite_local_interval_and_ess() {
        let result = evaluate_spatial_localization(&scenario()).expect("localized result");
        let local = &result.predictions[0];
        assert_eq!(result.pooled_radius, Some(1.0));
        assert_eq!(local.radius, Some(1.0));
        assert!((local.effective_sample_size - 10.0).abs() < 1e-12);
        assert!((local.maximum_normalized_weight - 0.1).abs() < 1e-12);
        assert_eq!(local.covered, Some(true));
        assert_eq!(
            result.method_status,
            "EXPERIMENTAL_LOCALIZED_DIAGNOSTIC_NO_GUARANTEE"
        );
    }

    #[test]
    fn remote_calibration_underflows_to_explicit_no_local_support() {
        let mut value = scenario();
        value.bandwidth_m = 0.001;
        value.test_points[0].x_m = 1_000_000.0;
        let result = evaluate_spatial_localization(&value).expect("remote result abstains");
        let local = &result.predictions[0];
        assert_eq!(local.radius, None);
        assert_eq!(local.covered, None);
        assert_eq!(local.status, "ABSTAIN_LOW_LOCAL_ESS");
        assert_eq!(local.effective_sample_size, 0.0);
    }

    #[test]
    fn minimum_effective_sample_size_forces_abstention() {
        let mut value = scenario();
        value.minimum_effective_sample_size = 11.0;
        let result = evaluate_spatial_localization(&value).expect("low ESS abstention");
        assert_eq!(result.predictions[0].status, "ABSTAIN_LOW_LOCAL_ESS");
        assert_eq!(result.predictions[0].radius, None);
    }

    #[test]
    fn local_diagnostics_expose_a_pocket_hidden_by_pooled_coverage() {
        let mut value = scenario();
        value.calibration_points = (0..10)
            .map(|index| calibration(&format!("a-{index}"), 0.0, 0.5))
            .chain((0..10).map(|index| calibration(&format!("b-{index}"), 100.0, 0.5)))
            .collect();
        value.test_points = (0..90)
            .map(|index| test_point(&format!("test-a-{index}"), 0.0, 100.5))
            .chain((0..10).map(|index| test_point(&format!("test-b-{index}"), 100.0, 108.0)))
            .collect();
        let result = evaluate_spatial_localization(&value).expect("local coverage result");
        assert_eq!(result.pooled_radius, Some(0.5));
        let covered = result
            .predictions
            .iter()
            .filter(|prediction| prediction.covered == Some(true))
            .count();
        let uncovered = result
            .predictions
            .iter()
            .filter(|prediction| prediction.covered == Some(false))
            .count();
        assert_eq!(covered, 90);
        assert_eq!(uncovered, 10);
        assert!(result.predictions[..90].iter().all(|prediction| prediction.covered == Some(true)));
        assert!(result.predictions[90..].iter().all(|prediction| prediction.covered == Some(false)));
    }

    #[test]
    fn larger_bandwidth_increases_remote_local_calibration_mass() {
        let mut narrow = scenario();
        narrow.test_points[0].x_m = 20.0;
        narrow.bandwidth_m = 2.0;
        let mut wide = narrow.clone();
        wide.bandwidth_m = 20.0;
        let narrow_result =
            evaluate_spatial_localization(&narrow).expect("narrow bandwidth result");
        let wide_result = evaluate_spatial_localization(&wide).expect("wide bandwidth result");
        assert!(
            wide_result.predictions[0].weighted_calibration_mass
                > narrow_result.predictions[0].weighted_calibration_mass
        );
        assert!(wide_result.predictions[0].effective_sample_size > 0.0);
    }

    #[test]
    fn duplicate_ids_across_calibration_and_test_are_rejected() {
        let mut value = scenario();
        value.test_points[0].point_id = value.calibration_points[0].point_id.clone();
        assert_eq!(
            evaluate_spatial_localization(&value),
            Err(SpatialLocalizationError::DuplicateIdentity)
        );
    }

    #[test]
    fn blank_coordinate_system_and_non_finite_geometry_are_rejected() {
        let mut blank = scenario();
        blank.coordinate_system_id = "  ".to_string();
        assert_eq!(
            evaluate_spatial_localization(&blank),
            Err(SpatialLocalizationError::EmptyCoordinateSystem)
        );
        let mut invalid = scenario();
        invalid.calibration_points[0].x_m = f64::NAN;
        assert_eq!(
            evaluate_spatial_localization(&invalid),
            Err(SpatialLocalizationError::NonFiniteCoordinate)
        );
    }

    #[test]
    fn invalid_bandwidth_alpha_and_residuals_fail_closed() {
        let mut bad_bandwidth = scenario();
        bad_bandwidth.bandwidth_m = 0.0;
        assert_eq!(
            evaluate_spatial_localization(&bad_bandwidth),
            Err(SpatialLocalizationError::InvalidBandwidth)
        );
        let mut bad_alpha = scenario();
        bad_alpha.alpha = 1.0;
        assert_eq!(
            evaluate_spatial_localization(&bad_alpha),
            Err(SpatialLocalizationError::InvalidAlpha)
        );
        let mut bad_residual = scenario();
        bad_residual.calibration_points[0].absolute_residual = -1.0;
        assert_eq!(
            evaluate_spatial_localization(&bad_residual),
            Err(SpatialLocalizationError::InvalidResidual)
        );
    }
}
