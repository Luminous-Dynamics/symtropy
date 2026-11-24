// SPDX-License-Identifier: AGPL-3.0-or-later

//! Profile-level comparison for external PIV benchmark references.
//!
//! V0 accepts normalized one-dimensional profile arrays after an upstream
//! importer has handled the source format, units and coordinate conventions.
//! It deliberately does not parse TIFF, MATLAB MAT or HDF5 files.
//!
//! Metrics preserve missingness and use trapezoidal quadrature for irregular
//! y+ spacing. Reference intervals and prediction intervals are reported as
//! agreement/coverage diagnostics, not as a claim of calibrated uncertainty.

#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Estimate {
    pub value: f64,
    pub interval: Option<Interval>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferenceObservation {
    pub value: f64,
    pub interval: Option<Interval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfilePoint {
    /// Wall-normal coordinate in wall units, y+.
    pub y_plus: f64,
    /// Missing estimates remain missing and lower coverage.
    pub estimate: Option<Estimate>,
    pub reference: ReferenceObservation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct YPlusBand {
    pub id: String,
    pub lower: f64,
    pub upper: f64,
}

impl YPlusBand {
    pub fn full_profile(points: &[ProfilePoint]) -> Option<Self> {
        let first = points.first()?;
        let last = points.last()?;
        Some(Self {
            id: "full_profile".to_owned(),
            lower: first.y_plus,
            upper: last.y_plus,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileInput {
    /// E.g. "mean_u_plus", "uu_plus", "vv_plus", "ww_plus", "uv_plus".
    pub channel: String,
    /// Unit string must be explicit, e.g. "wall_velocity_units".
    pub unit: String,
    /// Points must be strictly increasing in y+ and share the reference grid.
    pub points: Vec<ProfilePoint>,
    /// Include full-profile and predeclared near-wall / buffer / log-layer bands.
    pub bands: Vec<YPlusBand>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BandMetrics {
    pub band_id: String,
    pub lower_y_plus: f64,
    pub upper_y_plus: f64,
    pub reference_points: usize,
    pub estimated_points: usize,
    pub point_coverage_fraction: f64,
    /// Equal-point-weight metrics. Kept separate from quadrature metrics.
    pub pointwise_bias: Option<f64>,
    pub pointwise_rmse: Option<f64>,
    pub pointwise_nrmse_reference_rms: Option<f64>,
    /// Fraction of estimates lying inside the supplied reference interval.
    /// This is an agreement diagnostic, not formal coverage.
    pub reference_interval_points: usize,
    pub reference_interval_agreement_fraction: Option<f64>,
    /// Fraction of reference values covered by supplied prediction intervals.
    /// This is empirical coverage, not a calibrated guarantee.
    pub prediction_interval_points: usize,
    pub prediction_interval_coverage_fraction: Option<f64>,
    /// Trapezoidal metrics respect nonuniform y+ spacing and skip any segment
    /// whose left or right estimate is missing.
    pub supported_y_plus_span: f64,
    pub total_y_plus_span: f64,
    pub span_coverage_fraction: f64,
    pub trapezoidal_bias: Option<f64>,
    pub trapezoidal_rmse: Option<f64>,
    pub trapezoidal_nrmse_reference_rms: Option<f64>,
    pub integrated_absolute_error: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProfileReport {
    pub channel: String,
    pub unit: String,
    pub bands: Vec<BandMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProfileMetricError {
    EmptyChannel,
    EmptyUnit,
    TooFewProfilePoints,
    EmptyBands,
    NonFiniteCoordinate { index: usize },
    NonIncreasingCoordinate { index: usize },
    NonFiniteReference { index: usize },
    NonFiniteEstimate { index: usize },
    InvalidReferenceInterval { index: usize },
    InvalidPredictionInterval { index: usize },
    InvalidBand { band_id: String },
    BandHasTooFewPoints { band_id: String },
    BandHasNoAdjacentSegments { band_id: String },
}

impl fmt::Display for ProfileMetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid external PIV profile: {self:?}")
    }
}

impl Error for ProfileMetricError {}

pub fn compare_profile(input: &ProfileInput) -> Result<ProfileReport, ProfileMetricError> {
    if input.channel.trim().is_empty() {
        return Err(ProfileMetricError::EmptyChannel);
    }
    if input.unit.trim().is_empty() {
        return Err(ProfileMetricError::EmptyUnit);
    }
    if input.points.len() < 2 {
        return Err(ProfileMetricError::TooFewProfilePoints);
    }
    if input.bands.is_empty() {
        return Err(ProfileMetricError::EmptyBands);
    }

    validate_points(&input.points)?;
    let mut reports = Vec::with_capacity(input.bands.len());
    for band in &input.bands {
        validate_band(band)?;
        reports.push(compare_band(&input.points, band)?);
    }

    Ok(ProfileReport {
        channel: input.channel.clone(),
        unit: input.unit.clone(),
        bands: reports,
    })
}

fn validate_points(points: &[ProfilePoint]) -> Result<(), ProfileMetricError> {
    for (index, point) in points.iter().enumerate() {
        if !point.y_plus.is_finite() {
            return Err(ProfileMetricError::NonFiniteCoordinate { index });
        }
        if index > 0 && point.y_plus <= points[index - 1].y_plus {
            return Err(ProfileMetricError::NonIncreasingCoordinate { index });
        }
        if !point.reference.value.is_finite() {
            return Err(ProfileMetricError::NonFiniteReference { index });
        }
        if let Some(interval) = point.reference.interval {
            if !valid_interval(interval) {
                return Err(ProfileMetricError::InvalidReferenceInterval { index });
            }
        }
        if let Some(estimate) = point.estimate {
            if !estimate.value.is_finite() {
                return Err(ProfileMetricError::NonFiniteEstimate { index });
            }
            if let Some(interval) = estimate.interval {
                if !valid_interval(interval) {
                    return Err(ProfileMetricError::InvalidPredictionInterval { index });
                }
            }
        }
    }
    Ok(())
}

fn validate_band(band: &YPlusBand) -> Result<(), ProfileMetricError> {
    if band.id.trim().is_empty()
        || !band.lower.is_finite()
        || !band.upper.is_finite()
        || band.lower >= band.upper
    {
        return Err(ProfileMetricError::InvalidBand {
            band_id: band.id.clone(),
        });
    }
    Ok(())
}

fn valid_interval(interval: Interval) -> bool {
    interval.lower.is_finite()
        && interval.upper.is_finite()
        && interval.lower <= interval.upper
}

fn compare_band(
    points: &[ProfilePoint],
    band: &YPlusBand,
) -> Result<BandMetrics, ProfileMetricError> {
    let selected: Vec<&ProfilePoint> = points
        .iter()
        .filter(|point| point.y_plus >= band.lower && point.y_plus <= band.upper)
        .collect();
    if selected.len() < 2 {
        return Err(ProfileMetricError::BandHasTooFewPoints {
            band_id: band.id.clone(),
        });
    }

    let reference_points = selected.len();
    let estimated: Vec<(&ProfilePoint, Estimate)> = selected
        .iter()
        .filter_map(|point| point.estimate.map(|estimate| (*point, estimate)))
        .collect();
    let estimated_points = estimated.len();
    let point_coverage_fraction = estimated_points as f64 / reference_points as f64;

    let mut error_sum = 0.0;
    let mut error_squared_sum = 0.0;
    let mut reference_squared_sum = 0.0;
    let mut reference_interval_count = 0usize;
    let mut reference_interval_agreements = 0usize;
    let mut prediction_interval_count = 0usize;
    let mut prediction_interval_coverage = 0usize;

    for (point, estimate) in &estimated {
        let error = estimate.value - point.reference.value;
        error_sum += error;
        error_squared_sum += error * error;
        reference_squared_sum += point.reference.value * point.reference.value;

        if let Some(interval) = point.reference.interval {
            reference_interval_count += 1;
            if estimate.value >= interval.lower && estimate.value <= interval.upper {
                reference_interval_agreements += 1;
            }
        }
        if let Some(interval) = estimate.interval {
            prediction_interval_count += 1;
            if point.reference.value >= interval.lower && point.reference.value <= interval.upper {
                prediction_interval_coverage += 1;
            }
        }
    }

    let pointwise_bias = mean(error_sum, estimated_points);
    let pointwise_rmse = mean(error_squared_sum, estimated_points).map(f64::sqrt);
    let pointwise_nrmse_reference_rms = pointwise_rmse.and_then(|rmse| {
        mean(reference_squared_sum, estimated_points)
            .map(f64::sqrt)
            .and_then(|reference_rms| safe_ratio(rmse, reference_rms))
    });
    let reference_interval_agreement_fraction =
        ratio(reference_interval_agreements, reference_interval_count);
    let prediction_interval_coverage_fraction =
        ratio(prediction_interval_coverage, prediction_interval_count);

    let mut total_y_plus_span = 0.0;
    let mut supported_y_plus_span = 0.0;
    let mut integrated_signed_error = 0.0;
    let mut integrated_squared_error = 0.0;
    let mut integrated_reference_squared = 0.0;
    let mut integrated_absolute_error = 0.0;

    for pair in points.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if left.y_plus < band.lower
            || left.y_plus > band.upper
            || right.y_plus < band.lower
            || right.y_plus > band.upper
        {
            continue;
        }
        let delta_y = right.y_plus - left.y_plus;
        total_y_plus_span += delta_y;
        let (Some(left_estimate), Some(right_estimate)) = (left.estimate, right.estimate) else {
            continue;
        };

        supported_y_plus_span += delta_y;
        let left_error = left_estimate.value - left.reference.value;
        let right_error = right_estimate.value - right.reference.value;
        integrated_signed_error += delta_y * (left_error + right_error) * 0.5;
        integrated_squared_error +=
            delta_y * (left_error * left_error + right_error * right_error) * 0.5;
        integrated_absolute_error +=
            delta_y * (left_error.abs() + right_error.abs()) * 0.5;
        integrated_reference_squared += delta_y
            * (left.reference.value * left.reference.value
                + right.reference.value * right.reference.value)
            * 0.5;
    }

    if total_y_plus_span <= 0.0 {
        return Err(ProfileMetricError::BandHasNoAdjacentSegments {
            band_id: band.id.clone(),
        });
    }
    let span_coverage_fraction = supported_y_plus_span / total_y_plus_span;
    let trapezoidal_bias = if supported_y_plus_span > 0.0 {
        Some(integrated_signed_error / supported_y_plus_span)
    } else {
        None
    };
    let trapezoidal_rmse = if supported_y_plus_span > 0.0 {
        Some((integrated_squared_error / supported_y_plus_span).sqrt())
    } else {
        None
    };
    let trapezoidal_nrmse_reference_rms = trapezoidal_rmse.and_then(|rmse| {
        if supported_y_plus_span > 0.0 {
            safe_ratio(
                rmse,
                (integrated_reference_squared / supported_y_plus_span).sqrt(),
            )
        } else {
            None
        }
    });
    let integrated_absolute_error = if supported_y_plus_span > 0.0 {
        Some(integrated_absolute_error)
    } else {
        None
    };

    Ok(BandMetrics {
        band_id: band.id.clone(),
        lower_y_plus: band.lower,
        upper_y_plus: band.upper,
        reference_points,
        estimated_points,
        point_coverage_fraction,
        pointwise_bias,
        pointwise_rmse,
        pointwise_nrmse_reference_rms,
        reference_interval_points: reference_interval_count,
        reference_interval_agreement_fraction,
        prediction_interval_points: prediction_interval_count,
        prediction_interval_coverage_fraction,
        supported_y_plus_span,
        total_y_plus_span,
        span_coverage_fraction,
        trapezoidal_bias,
        trapezoidal_rmse,
        trapezoidal_nrmse_reference_rms,
        integrated_absolute_error,
    })
}

fn mean(sum: f64, count: usize) -> Option<f64> {
    if count == 0 {
        None
    } else {
        Some(sum / count as f64)
    }
}

fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

fn safe_ratio(numerator: f64, denominator: f64) -> Option<f64> {
    if !numerator.is_finite() || denominator == 0.0 || !denominator.is_finite() {
        None
    } else {
        Some(numerator / denominator)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(y_plus: f64, estimate: Option<f64>, reference: f64) -> ProfilePoint {
        ProfilePoint {
            y_plus,
            estimate: estimate.map(|value| Estimate {
                value,
                interval: None,
            }),
            reference: ReferenceObservation {
                value: reference,
                interval: Some(Interval {
                    lower: reference - 0.2,
                    upper: reference + 0.2,
                }),
            },
        }
    }

    fn input(points: Vec<ProfilePoint>) -> ProfileInput {
        let lower = points.first().expect("points").y_plus;
        let upper = points.last().expect("points").y_plus;
        ProfileInput {
            channel: "mean_u_plus".into(),
            unit: "wall_velocity_units".into(),
            points,
            bands: vec![YPlusBand {
                id: "full".into(),
                lower,
                upper,
            }],
        }
    }

    #[test]
    fn exact_profile_has_zero_error_and_full_coverage() {
        let report = compare_profile(&input(vec![
            point(1.0, Some(1.0), 1.0),
            point(2.0, Some(2.0), 2.0),
            point(5.0, Some(5.0), 5.0),
        ]))
        .expect("valid profile");
        let metrics = &report.bands[0];
        assert_eq!(metrics.point_coverage_fraction, 1.0);
        assert_eq!(metrics.pointwise_bias, Some(0.0));
        assert_eq!(metrics.pointwise_rmse, Some(0.0));
        assert_eq!(metrics.trapezoidal_rmse, Some(0.0));
        assert_eq!(metrics.span_coverage_fraction, 1.0);
    }

    #[test]
    fn constant_bias_is_reported_correctly() {
        let report = compare_profile(&input(vec![
            point(1.0, Some(1.5), 1.0),
            point(2.0, Some(2.5), 2.0),
            point(3.0, Some(3.5), 3.0),
        ]))
        .expect("valid profile");
        assert_eq!(report.bands[0].pointwise_bias, Some(0.5));
        assert_eq!(report.bands[0].pointwise_rmse, Some(0.5));
        assert_eq!(report.bands[0].trapezoidal_bias, Some(0.5));
    }

    #[test]
    fn missing_estimates_reduce_point_and_span_coverage() {
        let report = compare_profile(&input(vec![
            point(1.0, Some(1.0), 1.0),
            point(2.0, None, 2.0),
            point(3.0, Some(3.0), 3.0),
        ]))
        .expect("valid profile with missing estimate");
        let metrics = &report.bands[0];
        assert!((metrics.point_coverage_fraction - (2.0 / 3.0)).abs() < 1.0e-12);
        assert_eq!(metrics.span_coverage_fraction, 0.0);
        assert_eq!(metrics.trapezoidal_rmse, None);
        assert_eq!(metrics.pointwise_rmse, Some(0.0));
    }

    #[test]
    fn trapezoidal_metrics_respect_nonuniform_y_plus_spacing() {
        let report = compare_profile(&input(vec![
            point(0.0, Some(2.0), 1.0),
            point(1.0, Some(2.0), 1.0),
            point(9.0, Some(1.0), 1.0),
            point(10.0, Some(1.0), 1.0),
        ]))
        .expect("valid nonuniform profile");
        let metrics = &report.bands[0];
        assert!((metrics.pointwise_rmse.unwrap() - 0.5_f64.sqrt()).abs() < 1.0e-12);
        assert!((metrics.trapezoidal_rmse.unwrap() - 0.5_f64.sqrt()).abs() < 1.0e-12);
    }

    #[test]
    fn confidence_diagnostics_are_reported_only_when_intervals_exist() {
        let mut points = vec![point(1.0, Some(1.1), 1.0), point(2.0, Some(2.2), 2.0)];
        points[0].estimate.as_mut().unwrap().interval = Some(Interval {
            lower: 0.9,
            upper: 1.2,
        });
        points[1].estimate.as_mut().unwrap().interval = Some(Interval {
            lower: 2.1,
            upper: 2.3,
        });
        let report = compare_profile(&input(points)).expect("valid intervals");
        let metrics = &report.bands[0];
        assert_eq!(metrics.reference_interval_agreement_fraction, Some(1.0));
        assert_eq!(metrics.prediction_interval_coverage_fraction, Some(0.5));
    }

    #[test]
    fn y_plus_bands_produce_independent_near_wall_and_outer_metrics() {
        let mut profile = input(vec![
            point(1.0, Some(0.5), 1.0),
            point(5.0, Some(4.0), 4.0),
            point(10.0, Some(10.0), 10.0),
            point(50.0, Some(50.0), 50.0),
        ]);
        profile.bands = vec![
            YPlusBand {
                id: "near_wall".into(),
                lower: 1.0,
                upper: 10.0,
            },
            YPlusBand {
                id: "outer".into(),
                lower: 10.0,
                upper: 50.0,
            },
        ];
        let report = compare_profile(&profile).expect("valid bands");
        assert_eq!(report.bands.len(), 2);
        assert!(report.bands[0].pointwise_rmse.unwrap() > 0.0);
        assert_eq!(report.bands[1].pointwise_rmse, Some(0.0));
    }

    #[test]
    fn nonmonotone_y_plus_is_rejected() {
        let result = compare_profile(&input(vec![
            point(1.0, Some(1.0), 1.0),
            point(3.0, Some(3.0), 3.0),
            point(2.0, Some(2.0), 2.0),
        ]));
        assert_eq!(
            result,
            Err(ProfileMetricError::NonIncreasingCoordinate { index: 2 })
        );
    }

    #[test]
    fn zero_reference_rms_does_not_produce_nan_normalization() {
        let report = compare_profile(&input(vec![
            point(1.0, Some(0.1), 0.0),
            point(2.0, Some(0.2), 0.0),
        ]))
        .expect("zero reference is valid");
        assert_eq!(report.bands[0].pointwise_nrmse_reference_rms, None);
        assert_eq!(report.bands[0].trapezoidal_nrmse_reference_rms, None);
    }

    #[test]
    fn invalid_prediction_interval_is_rejected() {
        let mut points = vec![point(1.0, Some(1.0), 1.0), point(2.0, Some(2.0), 2.0)];
        points[0].estimate.as_mut().unwrap().interval = Some(Interval {
            lower: 2.0,
            upper: 1.0,
        });
        assert_eq!(
            compare_profile(&input(points)),
            Err(ProfileMetricError::InvalidPredictionInterval { index: 0 })
        );
    }
}
