// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic error summaries for analytical shallow-water benchmark fields.

/// Per-field comparison statistics. Bias is mean(actual - expected), so its sign
/// indicates whether the numerical field tends to over- or under-predict.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldErrorMetrics {
    pub sample_count: usize,
    pub mean_absolute_error: f64,
    pub root_mean_squared_error: f64,
    pub bias: f64,
    pub maximum_absolute_error: f64,
}

/// Failure modes for a field-metric calculation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricError {
    EmptySamples,
    LengthMismatch {
        actual_len: usize,
        expected_len: usize,
    },
    NonFiniteValue {
        index: usize,
    },
    NonFiniteAggregate,
}

/// Calculate MAE, RMSE, bias and maximum absolute error over paired field samples.
///
/// Callers are responsible for choosing a physically meaningful support. For
/// example, velocity should generally be compared only on an explicitly declared
/// wet-cell mask, while depth and discharge can be scored on the full domain.
/// The function rejects empty, mismatched and non-finite samples rather than
/// silently dropping them.
pub fn field_error_metrics(
    actual: &[f64],
    expected: &[f64],
) -> Result<FieldErrorMetrics, MetricError> {
    if actual.is_empty() || expected.is_empty() {
        return Err(MetricError::EmptySamples);
    }
    if actual.len() != expected.len() {
        return Err(MetricError::LengthMismatch {
            actual_len: actual.len(),
            expected_len: expected.len(),
        });
    }

    let mut absolute_sum = 0.0;
    let mut squared_sum = 0.0;
    let mut signed_sum = 0.0;
    let mut maximum_absolute_error: f64 = 0.0;

    for (index, (actual_value, expected_value)) in actual.iter().zip(expected.iter()).enumerate() {
        if !actual_value.is_finite() || !expected_value.is_finite() {
            return Err(MetricError::NonFiniteValue { index });
        }

        let error = actual_value - expected_value;
        let absolute_error = error.abs();
        if !error.is_finite() || !absolute_error.is_finite() {
            return Err(MetricError::NonFiniteAggregate);
        }
        absolute_sum += absolute_error;
        squared_sum += error * error;
        signed_sum += error;
        maximum_absolute_error = maximum_absolute_error.max(absolute_error);
    }

    let sample_count = actual.len();
    let count = sample_count as f64;
    let metrics = FieldErrorMetrics {
        sample_count,
        mean_absolute_error: absolute_sum / count,
        root_mean_squared_error: (squared_sum / count).sqrt(),
        bias: signed_sum / count,
        maximum_absolute_error,
    };
    if !metrics.mean_absolute_error.is_finite()
        || !metrics.root_mean_squared_error.is_finite()
        || !metrics.bias.is_finite()
        || !metrics.maximum_absolute_error.is_finite()
    {
        return Err(MetricError::NonFiniteAggregate);
    }
    Ok(metrics)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_known_field_errors_and_signed_bias() {
        let metrics = field_error_metrics(&[1.0, 2.0, 4.0], &[1.0, 3.0, 2.0]).unwrap();
        assert_eq!(metrics.sample_count, 3);
        assert!((metrics.mean_absolute_error - 1.0).abs() < 1.0e-12);
        assert!((metrics.root_mean_squared_error - (5.0_f64 / 3.0).sqrt()).abs() < 1.0e-12);
        assert!((metrics.bias - (1.0 / 3.0)).abs() < 1.0e-12);
        assert!((metrics.maximum_absolute_error - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_empty_mismatched_and_non_finite_samples() {
        assert_eq!(
            field_error_metrics(&[], &[]),
            Err(MetricError::EmptySamples)
        );
        assert_eq!(
            field_error_metrics(&[1.0], &[1.0, 2.0]),
            Err(MetricError::LengthMismatch {
                actual_len: 1,
                expected_len: 2
            })
        );
        assert_eq!(
            field_error_metrics(&[1.0, f64::NAN], &[1.0, 2.0]),
            Err(MetricError::NonFiniteValue { index: 1 })
        );
        assert_eq!(
            field_error_metrics(&[f64::MAX], &[-f64::MAX]),
            Err(MetricError::NonFiniteAggregate)
        );
    }
}
