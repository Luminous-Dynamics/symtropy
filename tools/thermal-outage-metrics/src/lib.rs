//! Independent arithmetic oracle for generic cold/hot degree-hour exposure.
//!
//! This is not a thermal model, comfort standard, health-risk assessor, or trajectory
//! engine. It integrates a declared threshold against a temperature time series using
//! piecewise-linear interpolation only across valid, bounded sampling intervals.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreachSide {
    BelowFloor,
    AboveCeiling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationWindow {
    pub start_seconds: u64,
    pub end_seconds: u64,
    pub max_gap_seconds: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TemperatureSample {
    pub at_seconds: u64,
    /// None means missing/invalid for this arithmetic oracle, never zero.
    pub temperature_c: Option<f64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricInputError {
    EmptySamples,
    InvalidWindow,
    ZeroMaximumGap,
    InvalidThreshold,
    SampleBoundsDoNotMatchWindow,
    NonMonotonicTimestamps,
    NonFiniteTemperature,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThermalExposureMetrics {
    /// Exposure integrated only over intervals with two valid endpoints and a gap
    /// no larger than max_gap_seconds. This is not a whole-window value if incomplete.
    pub degree_hours_over_covered_intervals: f64,
    /// Breach duration integrated only over covered intervals.
    pub breach_hours_over_covered_intervals: f64,
    /// Maximum endpoint departure observed on covered intervals.
    pub max_departure_c_over_covered_intervals: Option<f64>,
    /// First breach detected in the covered intervals. If an earlier interval is
    /// missing, this is only the first observed breach, not necessarily the true first.
    pub first_observed_breach_seconds: Option<f64>,
    /// Set only when every interval in the requested window is covered.
    pub first_breach_seconds_if_complete: Option<f64>,
    pub total_window_seconds: u64,
    pub covered_seconds: u64,
    pub missing_or_rejected_interval_count: usize,
    pub coverage_fraction: f64,
    pub complete: bool,
}

fn signed_departure(temperature_c: f64, threshold_c: f64, side: BreachSide) -> f64 {
    match side {
        BreachSide::BelowFloor => threshold_c - temperature_c,
        BreachSide::AboveCeiling => temperature_c - threshold_c,
    }
}

/// Exact integral, in degree-Celsius-seconds, of the positive part of a linear
/// function whose values at the interval endpoints are x0 and x1.
fn integrate_positive_linear(x0: f64, x1: f64, duration_seconds: f64) -> f64 {
    if x0 <= 0.0 && x1 <= 0.0 {
        return 0.0;
    }
    if x0 >= 0.0 && x1 >= 0.0 {
        return 0.5 * (x0 + x1) * duration_seconds;
    }
    if x0 > 0.0 {
        let positive_fraction = x0 / (x0 - x1);
        return 0.5 * x0 * positive_fraction * duration_seconds;
    }
    let positive_fraction = x1 / (x1 - x0);
    0.5 * x1 * positive_fraction * duration_seconds
}

/// Duration, in seconds, for which a linear interval has a positive value.
fn positive_duration(x0: f64, x1: f64, duration_seconds: f64) -> f64 {
    if x0 <= 0.0 && x1 <= 0.0 {
        return 0.0;
    }
    if x0 >= 0.0 && x1 >= 0.0 {
        return duration_seconds;
    }
    if x0 > 0.0 {
        return (x0 / (x0 - x1)) * duration_seconds;
    }
    (x1 / (x1 - x0)) * duration_seconds
}

fn first_positive_time(
    x0: f64,
    x1: f64,
    start_seconds: u64,
    duration_seconds: f64,
) -> Option<f64> {
    if x0 > 0.0 {
        return Some(start_seconds as f64);
    }
    if x1 <= 0.0 {
        return None;
    }
    let crossing_fraction = (-x0 / (x1 - x0)).clamp(0.0, 1.0);
    Some(start_seconds as f64 + crossing_fraction * duration_seconds)
}

fn validate(
    samples: &[TemperatureSample],
    window: ObservationWindow,
    threshold_c: f64,
) -> Result<(), MetricInputError> {
    if samples.is_empty() {
        return Err(MetricInputError::EmptySamples);
    }
    if window.end_seconds <= window.start_seconds {
        return Err(MetricInputError::InvalidWindow);
    }
    if window.max_gap_seconds == 0 {
        return Err(MetricInputError::ZeroMaximumGap);
    }
    if !threshold_c.is_finite() {
        return Err(MetricInputError::InvalidThreshold);
    }
    if samples.first().map(|sample| sample.at_seconds) != Some(window.start_seconds)
        || samples.last().map(|sample| sample.at_seconds) != Some(window.end_seconds)
    {
        return Err(MetricInputError::SampleBoundsDoNotMatchWindow);
    }
    if samples
        .windows(2)
        .any(|pair| pair[0].at_seconds >= pair[1].at_seconds)
    {
        return Err(MetricInputError::NonMonotonicTimestamps);
    }
    if samples
        .iter()
        .filter_map(|sample| sample.temperature_c)
        .any(|temperature| !temperature.is_finite())
    {
        return Err(MetricInputError::NonFiniteTemperature);
    }
    Ok(())
}

/// Calculate one generic threshold-exposure metric.
///
/// The threshold is supplied by the caller and is not selected by this crate.
/// BelowFloor computes cold-side deficit; AboveCeiling computes hot-side exceedance.
/// Time stamps are monotonic seconds relative to an externally declared origin. Each
/// interval is integrated only if both endpoints are present and its duration is at
/// most window.max_gap_seconds.
///
/// The interpolation assumption is linear between valid samples. No extrapolation
/// or imputation occurs. Incomplete coverage is returned explicitly; consumers must
/// not promote covered-interval values to a whole-window result.
pub fn calculate_degree_hour_exposure(
    samples: &[TemperatureSample],
    window: ObservationWindow,
    threshold_c: f64,
    side: BreachSide,
) -> Result<ThermalExposureMetrics, MetricInputError> {
    validate(samples, window, threshold_c)?;

    let mut degree_c_seconds = 0.0;
    let mut breach_seconds = 0.0;
    let mut covered_seconds = 0_u64;
    let mut missing_or_rejected_interval_count = 0_usize;
    let mut max_departure_c: Option<f64> = None;
    let mut first_observed_breach_seconds: Option<f64> = None;

    for pair in samples.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let interval_seconds = right.at_seconds - left.at_seconds;

        let (Some(left_c), Some(right_c)) = (left.temperature_c, right.temperature_c) else {
            missing_or_rejected_interval_count += 1;
            continue;
        };

        if interval_seconds > window.max_gap_seconds {
            missing_or_rejected_interval_count += 1;
            continue;
        }

        covered_seconds += interval_seconds;
        let x0 = signed_departure(left_c, threshold_c, side);
        let x1 = signed_departure(right_c, threshold_c, side);
        let duration = interval_seconds as f64;

        degree_c_seconds += integrate_positive_linear(x0, x1, duration);
        breach_seconds += positive_duration(x0, x1, duration);

        let interval_max = x0.max(x1).max(0.0);
        if interval_max > 0.0 {
            max_departure_c = Some(
                max_departure_c
                    .map_or(interval_max, |current| current.max(interval_max)),
            );
        }

        if first_observed_breach_seconds.is_none() {
            first_observed_breach_seconds =
                first_positive_time(x0, x1, left.at_seconds, duration);
        }
    }

    let total_window_seconds = window.end_seconds - window.start_seconds;
    let complete = covered_seconds == total_window_seconds;
    let coverage_fraction = covered_seconds as f64 / total_window_seconds as f64;

    Ok(ThermalExposureMetrics {
        degree_hours_over_covered_intervals: degree_c_seconds / 3_600.0,
        breach_hours_over_covered_intervals: breach_seconds / 3_600.0,
        max_departure_c_over_covered_intervals: max_departure_c,
        first_observed_breach_seconds,
        first_breach_seconds_if_complete: complete
            .then_some(first_observed_breach_seconds)
            .flatten(),
        total_window_seconds,
        covered_seconds,
        missing_or_rejected_interval_count,
        coverage_fraction,
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(at_seconds: u64, temperature_c: Option<f64>) -> TemperatureSample {
        TemperatureSample {
            at_seconds,
            temperature_c,
        }
    }

    fn window(end_seconds: u64) -> ObservationWindow {
        ObservationWindow {
            start_seconds: 0,
            end_seconds,
            max_gap_seconds: 3_600,
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-9,
            "actual={actual}, expected={expected}"
        );
    }

    #[test]
    fn cold_deficit_integrates_exact_piecewise_linear_crossings() {
        let samples = [
            sample(0, Some(20.0)),
            sample(3_600, Some(16.0)),
            sample(7_200, Some(20.0)),
        ];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(7_200),
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 1.0);
        close(result.breach_hours_over_covered_intervals, 1.0);
        close(result.max_departure_c_over_covered_intervals.unwrap(), 2.0);
        close(result.first_observed_breach_seconds.unwrap(), 1_800.0);
        close(result.first_breach_seconds_if_complete.unwrap(), 1_800.0);
        assert!(result.complete);
    }

    #[test]
    fn hot_exceedance_uses_the_same_geometry_with_opposite_direction() {
        let samples = [
            sample(0, Some(24.0)),
            sample(3_600, Some(28.0)),
            sample(7_200, Some(24.0)),
        ];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(7_200),
            26.0,
            BreachSide::AboveCeiling,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 2.0);
        close(result.breach_hours_over_covered_intervals, 2.0);
        close(result.max_departure_c_over_covered_intervals.unwrap(), 2.0);
        close(result.first_observed_breach_seconds.unwrap(), 1_800.0);
    }

    #[test]
    fn always_within_threshold_has_zero_exposure() {
        let samples = [
            sample(0, Some(20.0)),
            sample(3_600, Some(19.0)),
            sample(7_200, Some(20.0)),
        ];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(7_200),
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 0.0);
        close(result.breach_hours_over_covered_intervals, 0.0);
        assert_eq!(result.max_departure_c_over_covered_intervals, None);
        assert_eq!(result.first_breach_seconds_if_complete, None);
        assert!(result.complete);
    }

    #[test]
    fn constant_two_degree_deficit_for_one_hour_equals_two_degree_hours() {
        let samples = [sample(0, Some(16.0)), sample(3_600, Some(16.0))];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(3_600),
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 2.0);
        close(result.breach_hours_over_covered_intervals, 1.0);
        close(result.max_departure_c_over_covered_intervals.unwrap(), 2.0);
    }

    #[test]
    fn constant_three_degree_hot_exceedance_for_two_hours_equals_six_degree_hours() {
        let samples = [sample(0, Some(29.0)), sample(7_200, Some(29.0))];
        let result = calculate_degree_hour_exposure(
            &samples,
            ObservationWindow {
                start_seconds: 0,
                end_seconds: 7_200,
                max_gap_seconds: 7_200,
            },
            26.0,
            BreachSide::AboveCeiling,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 6.0);
        close(result.breach_hours_over_covered_intervals, 2.0);
    }

    #[test]
    fn invalid_window_is_rejected() {
        let samples = [sample(0, Some(20.0)), sample(3_600, Some(16.0))];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                ObservationWindow {
                    start_seconds: 3_600,
                    end_seconds: 0,
                    max_gap_seconds: 3_600,
                },
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::InvalidWindow)
        );
    }

    #[test]
    fn exactly_on_threshold_is_not_a_breach() {
        let samples = [sample(0, Some(18.0)), sample(3_600, Some(18.0))];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(3_600),
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 0.0);
        close(result.breach_hours_over_covered_intervals, 0.0);
        assert_eq!(result.first_observed_breach_seconds, None);
    }

    #[test]
    fn missing_sample_is_not_imputed_as_zero() {
        let samples = [
            sample(0, Some(20.0)),
            sample(3_600, Some(16.0)),
            sample(7_200, None),
            sample(10_800, Some(22.0)),
        ];
        let result = calculate_degree_hour_exposure(
            &samples,
            ObservationWindow {
                start_seconds: 0,
                end_seconds: 10_800,
                max_gap_seconds: 3_600,
            },
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 0.5);
        close(result.coverage_fraction, 1.0 / 3.0);
        assert_eq!(result.covered_seconds, 3_600);
        assert_eq!(result.missing_or_rejected_interval_count, 2);
        assert!(!result.complete);
        assert_eq!(result.first_breach_seconds_if_complete, None);
        close(result.first_observed_breach_seconds.unwrap(), 1_800.0);
    }

    #[test]
    fn excessive_sample_gap_is_rejected_from_integration() {
        let samples = [sample(0, Some(20.0)), sample(7_200, Some(16.0))];
        let result = calculate_degree_hour_exposure(
            &samples,
            ObservationWindow {
                start_seconds: 0,
                end_seconds: 7_200,
                max_gap_seconds: 3_600,
            },
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        close(result.degree_hours_over_covered_intervals, 0.0);
        close(result.coverage_fraction, 0.0);
        assert!(!result.complete);
        assert_eq!(result.missing_or_rejected_interval_count, 1);
    }

    #[test]
    fn first_sample_below_floor_breaches_at_window_start() {
        let samples = [sample(0, Some(16.0)), sample(3_600, Some(17.0))];
        let result = calculate_degree_hour_exposure(
            &samples,
            window(3_600),
            18.0,
            BreachSide::BelowFloor,
        )
        .unwrap();

        assert_eq!(result.first_breach_seconds_if_complete, Some(0.0));
    }

    #[test]
    fn unsorted_samples_are_rejected() {
        let samples = [
            sample(0, Some(20.0)),
            sample(3_600, Some(19.0)),
            sample(1_800, Some(17.0)),
        ];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::NonMonotonicTimestamps)
        );
    }

    #[test]
    fn duplicate_timestamps_are_rejected() {
        let samples = [
            sample(0, Some(20.0)),
            sample(0, Some(17.0)),
            sample(3_600, Some(16.0)),
        ];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::NonMonotonicTimestamps)
        );
    }

    #[test]
    fn non_finite_temperature_is_rejected() {
        let samples = [sample(0, Some(20.0)), sample(3_600, Some(f64::NAN))];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::NonFiniteTemperature)
        );
    }

    #[test]
    fn missing_window_boundary_is_rejected() {
        let samples = [sample(60, Some(20.0)), sample(3_600, Some(16.0))];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::SampleBoundsDoNotMatchWindow)
        );
    }

    #[test]
    fn zero_maximum_gap_is_rejected() {
        let samples = [sample(0, Some(20.0)), sample(3_600, Some(16.0))];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                ObservationWindow {
                    start_seconds: 0,
                    end_seconds: 3_600,
                    max_gap_seconds: 0,
                },
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::ZeroMaximumGap)
        );
    }

    #[test]
    fn non_finite_threshold_is_rejected() {
        let samples = [sample(0, Some(20.0)), sample(3_600, Some(16.0))];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                f64::INFINITY,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::InvalidThreshold)
        );
    }

    #[test]
    fn empty_series_is_rejected() {
        let samples: [TemperatureSample; 0] = [];
        assert_eq!(
            calculate_degree_hour_exposure(
                &samples,
                window(3_600),
                18.0,
                BreachSide::BelowFloor
            ),
            Err(MetricInputError::EmptySamples)
        );
    }
}
