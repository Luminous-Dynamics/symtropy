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
    interval.lower.is_finite() && interval.upper.is_finite() && interval.lower <= interval.upper
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
        integrated_absolute_error += delta_y * (left_error.abs() + right_error.abs()) * 0.5;
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

/// Symmetric three-component Reynolds-stress tensor, with components in the
/// order uu, vv, ww, uv, uw, vw. Off-diagonal terms are covariances, not
/// correlation coefficients.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReynoldsStressTensor3D {
    pub uu: f64,
    pub vv: f64,
    pub ww: f64,
    pub uv: f64,
    pub uw: f64,
    pub vw: f64,
}

impl ReynoldsStressTensor3D {
    pub fn turbulent_kinetic_energy(self) -> f64 {
        0.5 * (self.uu + self.vv + self.ww)
    }

    pub fn components(self) -> [f64; 6] {
        [self.uu, self.vv, self.ww, self.uv, self.uw, self.vw]
    }

    /// Tests positive semidefiniteness of the 3x3 symmetric covariance
    /// matrix. All principal minors are checked after scale normalization.
    /// The tolerance is relative and dimensionless.
    pub fn is_positive_semidefinite(
        self,
        relative_tolerance: f64,
    ) -> Result<bool, StressTensorError> {
        if !relative_tolerance.is_finite() || relative_tolerance < 0.0 {
            return Err(StressTensorError::InvalidTolerance);
        }
        let values = self.components();
        if values.iter().any(|value| !value.is_finite()) {
            return Err(StressTensorError::NonFiniteComponent);
        }
        let scale = values
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if scale == 0.0 {
            return Ok(true);
        }

        let a = self.uu / scale;
        let d = self.vv / scale;
        let f = self.ww / scale;
        let b = self.uv / scale;
        let c = self.uw / scale;
        let e = self.vw / scale;

        let diagonal_minors = [a, d, f];
        let second_order_minors = [a * d - b * b, a * f - c * c, d * f - e * e];
        let determinant = a * d * f + 2.0 * b * c * e - a * e * e - d * c * c - f * b * b;

        Ok(diagonal_minors
            .iter()
            .chain(second_order_minors.iter())
            .all(|minor| *minor >= -relative_tolerance)
            && determinant >= -relative_tolerance)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StressProfilePoint {
    pub y_plus: f64,
    pub estimate: Option<ReynoldsStressTensor3D>,
    pub reference: ReynoldsStressTensor3D,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StressProfileInput {
    pub unit: String,
    pub points: Vec<StressProfilePoint>,
    pub bands: Vec<YPlusBand>,
    pub psd_relative_tolerance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StressBandMetrics {
    pub band_id: String,
    pub point_count: usize,
    pub estimated_points: usize,
    pub point_coverage_fraction: f64,
    /// RMSE order: uu, vv, ww, uv, uw, vw.
    pub component_rmse: [Option<f64>; 6],
    pub tke_bias: Option<f64>,
    pub tke_rmse: Option<f64>,
    /// RMSE of the full 3x3 tensor Frobenius norm. Off-diagonal entries count
    /// twice, as they do in the symmetric matrix.
    pub tensor_frobenius_rmse: Option<f64>,
    pub tensor_frobenius_nrmse: Option<f64>,
    pub reference_non_psd_points: usize,
    pub estimate_non_psd_points: usize,
    pub estimate_realizability_violation_fraction: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StressProfileReport {
    pub unit: String,
    pub bands: Vec<StressBandMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StressTensorError {
    InvalidTolerance,
    NonFiniteComponent,
    EmptyUnit,
    TooFewProfilePoints,
    EmptyBands,
    NonFiniteCoordinate { index: usize },
    NonIncreasingCoordinate { index: usize },
    InvalidBand { band_id: String },
    BandHasTooFewPoints { band_id: String },
    InvalidReferenceTensor { index: usize },
    InvalidEstimateTensor { index: usize },
}

impl fmt::Display for StressTensorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid Reynolds-stress profile: {self:?}")
    }
}

impl Error for StressTensorError {}

pub fn compare_reynolds_stress_profile(
    input: &StressProfileInput,
) -> Result<StressProfileReport, StressTensorError> {
    if input.unit.trim().is_empty() {
        return Err(StressTensorError::EmptyUnit);
    }
    if input.points.len() < 2 {
        return Err(StressTensorError::TooFewProfilePoints);
    }
    if input.bands.is_empty() {
        return Err(StressTensorError::EmptyBands);
    }
    if !input.psd_relative_tolerance.is_finite() || input.psd_relative_tolerance < 0.0 {
        return Err(StressTensorError::InvalidTolerance);
    }

    for (index, point) in input.points.iter().enumerate() {
        if !point.y_plus.is_finite() {
            return Err(StressTensorError::NonFiniteCoordinate { index });
        }
        if index > 0 && point.y_plus <= input.points[index - 1].y_plus {
            return Err(StressTensorError::NonIncreasingCoordinate { index });
        }
        if point
            .reference
            .is_positive_semidefinite(input.psd_relative_tolerance)
            .is_err()
        {
            return Err(StressTensorError::InvalidReferenceTensor { index });
        }
        if let Some(estimate) = point.estimate {
            if estimate
                .is_positive_semidefinite(input.psd_relative_tolerance)
                .is_err()
            {
                return Err(StressTensorError::InvalidEstimateTensor { index });
            }
        }
    }

    let mut reports = Vec::with_capacity(input.bands.len());
    for band in &input.bands {
        if band.id.trim().is_empty()
            || !band.lower.is_finite()
            || !band.upper.is_finite()
            || band.lower >= band.upper
        {
            return Err(StressTensorError::InvalidBand {
                band_id: band.id.clone(),
            });
        }
        let selected: Vec<&StressProfilePoint> = input
            .points
            .iter()
            .filter(|point| point.y_plus >= band.lower && point.y_plus <= band.upper)
            .collect();
        if selected.len() < 2 {
            return Err(StressTensorError::BandHasTooFewPoints {
                band_id: band.id.clone(),
            });
        }

        let point_count = selected.len();
        let estimates: Vec<(&StressProfilePoint, ReynoldsStressTensor3D)> = selected
            .iter()
            .filter_map(|point| point.estimate.map(|estimate| (*point, estimate)))
            .collect();
        let estimated_points = estimates.len();
        let mut component_squared_errors = [0.0; 6];
        let mut tke_error_sum = 0.0;
        let mut tke_squared_error_sum = 0.0;
        let mut tensor_squared_error_sum = 0.0;
        let mut reference_tensor_squared_sum = 0.0;
        let mut reference_non_psd_points = 0usize;
        let mut estimate_non_psd_points = 0usize;

        for point in &selected {
            if !point
                .reference
                .is_positive_semidefinite(input.psd_relative_tolerance)?
            {
                reference_non_psd_points += 1;
            }
            if let Some(estimate) = point.estimate {
                if !estimate.is_positive_semidefinite(input.psd_relative_tolerance)? {
                    estimate_non_psd_points += 1;
                }
            }
        }

        for (point, estimate) in &estimates {
            let reference_values = point.reference.components();
            let estimate_values = estimate.components();
            for (error_sum, (estimate_value, reference_value)) in component_squared_errors
                .iter_mut()
                .zip(estimate_values.iter().zip(reference_values.iter()))
            {
                let difference = estimate_value - reference_value;
                *error_sum += difference * difference;
            }

            let tke_error =
                estimate.turbulent_kinetic_energy() - point.reference.turbulent_kinetic_energy();
            tke_error_sum += tke_error;
            tke_squared_error_sum += tke_error * tke_error;

            let [duu, dvv, dww, duv, duw, dvw] =
                std::array::from_fn(|index| estimate_values[index] - reference_values[index]);
            tensor_squared_error_sum +=
                duu * duu + dvv * dvv + dww * dww + 2.0 * (duv * duv + duw * duw + dvw * dvw);
            let [ruu, rvv, rww, ruv, ruw, rvw] = reference_values;
            reference_tensor_squared_sum +=
                ruu * ruu + rvv * rvv + rww * rww + 2.0 * (ruv * ruv + ruw * ruw + rvw * rvw);
        }

        let component_rmse = std::array::from_fn(|index| {
            if estimated_points == 0 {
                None
            } else {
                Some((component_squared_errors[index] / estimated_points as f64).sqrt())
            }
        });
        let tke_bias = mean(tke_error_sum, estimated_points);
        let tke_rmse = mean(tke_squared_error_sum, estimated_points).map(f64::sqrt);
        let tensor_frobenius_rmse = mean(tensor_squared_error_sum, estimated_points).map(f64::sqrt);
        let tensor_frobenius_nrmse = if reference_tensor_squared_sum > 0.0 {
            safe_ratio(
                tensor_squared_error_sum.sqrt(),
                reference_tensor_squared_sum.sqrt(),
            )
        } else {
            None
        };

        reports.push(StressBandMetrics {
            band_id: band.id.clone(),
            point_count,
            estimated_points,
            point_coverage_fraction: estimated_points as f64 / point_count as f64,
            component_rmse,
            tke_bias,
            tke_rmse,
            tensor_frobenius_rmse,
            tensor_frobenius_nrmse,
            reference_non_psd_points,
            estimate_non_psd_points,
            estimate_realizability_violation_fraction: ratio(
                estimate_non_psd_points,
                estimated_points,
            ),
        });
    }

    Ok(StressProfileReport {
        unit: input.unit.clone(),
        bands: reports,
    })
}

#[cfg(test)]
mod stress_tensor_tests {
    use super::*;

    fn tensor(scale: f64) -> ReynoldsStressTensor3D {
        ReynoldsStressTensor3D {
            uu: 4.0 * scale,
            vv: 2.0 * scale,
            ww: 1.0 * scale,
            uv: 0.5 * scale,
            uw: 0.2 * scale,
            vw: 0.1 * scale,
        }
    }

    fn stress_input(
        estimates: [Option<ReynoldsStressTensor3D>; 3],
        references: [ReynoldsStressTensor3D; 3],
    ) -> StressProfileInput {
        StressProfileInput {
            unit: "wall_velocity_squared".into(),
            points: (0..3)
                .map(|index| StressProfilePoint {
                    y_plus: [1.0, 10.0, 100.0][index],
                    estimate: estimates[index],
                    reference: references[index],
                })
                .collect(),
            bands: vec![YPlusBand {
                id: "full".into(),
                lower: 1.0,
                upper: 100.0,
            }],
            psd_relative_tolerance: 1.0e-10,
        }
    }

    #[test]
    fn positive_semidefinite_tensor_is_accepted() {
        assert!(tensor(1.0).is_positive_semidefinite(1.0e-10).unwrap());
    }

    #[test]
    fn impossible_cross_covariance_is_not_realizable() {
        let invalid = ReynoldsStressTensor3D {
            uu: 1.0,
            vv: 1.0,
            ww: 1.0,
            uv: 1.5,
            uw: 0.0,
            vw: 0.0,
        };
        assert!(!invalid.is_positive_semidefinite(1.0e-10).unwrap());
    }

    #[test]
    fn exact_reynolds_stress_profile_has_zero_component_and_tke_error() {
        let value = tensor(1.0);
        let report =
            compare_reynolds_stress_profile(&stress_input([Some(value); 3], [value; 3])).unwrap();
        let metrics = &report.bands[0];
        assert_eq!(metrics.point_coverage_fraction, 1.0);
        assert!(
            metrics
                .component_rmse
                .iter()
                .all(|value| *value == Some(0.0))
        );
        assert_eq!(metrics.tke_bias, Some(0.0));
        assert_eq!(metrics.tke_rmse, Some(0.0));
        assert_eq!(metrics.estimate_non_psd_points, 0);
    }

    #[test]
    fn each_stress_component_is_reported_separately() {
        let reference = tensor(1.0);
        let mut changed = reference;
        changed.uv += 1.0;
        let report =
            compare_reynolds_stress_profile(&stress_input([Some(changed); 3], [reference; 3]))
                .unwrap();
        let components = report.bands[0].component_rmse;
        assert_eq!(components[0], Some(0.0));
        assert_eq!(components[3], Some(1.0));
    }

    #[test]
    fn non_realizable_estimates_remain_visible_in_the_scorecard() {
        let reference = tensor(1.0);
        let invalid = ReynoldsStressTensor3D {
            uu: 1.0,
            vv: 1.0,
            ww: 1.0,
            uv: 2.0,
            uw: 0.0,
            vw: 0.0,
        };
        let report =
            compare_reynolds_stress_profile(&stress_input([Some(invalid); 3], [reference; 3]))
                .unwrap();
        assert_eq!(report.bands[0].estimate_non_psd_points, 3);
        assert_eq!(
            report.bands[0].estimate_realizability_violation_fraction,
            Some(1.0)
        );
    }

    #[test]
    fn missing_tensor_estimates_reduce_stress_profile_coverage() {
        let reference = tensor(1.0);
        let report = compare_reynolds_stress_profile(&stress_input(
            [Some(reference), None, Some(reference)],
            [reference; 3],
        ))
        .unwrap();
        assert!((report.bands[0].point_coverage_fraction - 2.0 / 3.0).abs() < 1.0e-12);
        assert_eq!(
            report.bands[0].estimate_realizability_violation_fraction,
            Some(0.0)
        );
    }

    #[test]
    fn stress_profile_rejects_nonfinite_tensors() {
        let mut invalid = tensor(1.0);
        invalid.uv = f64::NAN;
        let result = compare_reynolds_stress_profile(&stress_input(
            [Some(tensor(1.0)); 3],
            [tensor(1.0), invalid, tensor(1.0)],
        ));
        assert_eq!(
            result,
            Err(StressTensorError::InvalidReferenceTensor { index: 1 })
        );
    }

    #[test]
    fn tke_error_does_not_hide_individual_stress_component_errors() {
        let reference = tensor(1.0);
        let mut changed = reference;
        changed.uu += 1.0;
        changed.vv -= 1.0;
        let report =
            compare_reynolds_stress_profile(&stress_input([Some(changed); 3], [reference; 3]))
                .unwrap();
        assert_eq!(report.bands[0].tke_bias, Some(0.0));
        assert!(report.bands[0].component_rmse[0].unwrap() > 0.0);
        assert!(report.bands[0].component_rmse[1].unwrap() > 0.0);
    }
    #[test]
    fn singular_positive_semidefinite_stress_tensor_is_accepted() {
        let rank_one = ReynoldsStressTensor3D {
            uu: 1.0,
            vv: 1.0,
            ww: 0.0,
            uv: 1.0,
            uw: 0.0,
            vw: 0.0,
        };
        assert!(rank_one.is_positive_semidefinite(1.0e-10).unwrap());
    }

    #[test]
    fn non_realizable_reference_tensor_is_counted_not_silently_corrected() {
        let mut invalid_reference = tensor(1.0);
        invalid_reference.uv = 4.0;
        let predicted = tensor(1.0);
        let report = compare_reynolds_stress_profile(&stress_input(
            [Some(predicted); 3],
            [invalid_reference; 3],
        ))
        .unwrap();
        assert_eq!(report.bands[0].reference_non_psd_points, 3);
        assert_eq!(report.bands[0].estimate_non_psd_points, 0);
    }

}
