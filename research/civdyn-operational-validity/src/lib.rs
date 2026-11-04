// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Operational pattern comparison for independent empirical windows and
//! simulation replications.
//!
//! This module deliberately does not emit a single validity grade. It reports
//! a descriptive vector of means, quantiles, uncertainty intervals and
//! empirical distribution distances. It does not establish causal validity,
//! equivalence or forecasting skill by itself.

use core::fmt;

pub const MIN_INDEPENDENT_SAMPLES: usize = 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    /// A separate scope such as root_posts, comments, region_01 or network_core.
    pub stratum_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericSample {
    pub sample_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleSet {
    pub unit: String,
    pub samples: Vec<NumericSample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantileSummary {
    pub p10: f64,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SampleSummary {
    pub sample_count: usize,
    pub mean: f64,
    pub sample_standard_deviation: f64,
    pub median: f64,
    pub quantiles: QuantileSummary,
    /// Descriptive two-sided 99% t interval half-width for this sample mean.
    pub mean_99_ci_half_width: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OperationalComparison {
    pub metric: MetricSpec,
    pub observed: SampleSummary,
    pub simulated: SampleSummary,
    /// Positive values mean the simulation is larger than the empirical value.
    pub mean_delta_simulated_minus_observed: f64,
    pub p10_delta_simulated_minus_observed: f64,
    pub median_delta_simulated_minus_observed: f64,
    pub p90_delta_simulated_minus_observed: f64,
    pub p99_delta_simulated_minus_observed: f64,
    /// Two-sample Kolmogorov-Smirnov distance for the empirical CDFs.
    pub ks_distance: f64,
    /// One-dimensional empirical Wasserstein-1 distance in the declared unit.
    pub wasserstein_1: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyMetricId,
    EmptyUnit,
    EmptyStratumId,
    UnitMismatch {
        side: &'static str,
        expected: String,
        actual: String,
    },
    TooFewIndependentSamples {
        side: &'static str,
        actual: usize,
        minimum: usize,
    },
    EmptySampleId {
        side: &'static str,
        index: usize,
    },
    DuplicateSampleId {
        side: &'static str,
        sample_id: String,
    },
    NonFiniteValue {
        side: &'static str,
        sample_id: String,
        value: f64,
    },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMetricId => write!(f, "metric_id must not be empty"),
            Self::EmptyUnit => write!(f, "metric unit must not be empty"),
            Self::EmptyStratumId => write!(f, "stratum_id must not be empty"),
            Self::UnitMismatch { side, expected, actual } => write!(
                f,
                "{side} sample unit mismatch: expected {expected}, got {actual}"
            ),
            Self::TooFewIndependentSamples { side, actual, minimum } => write!(
                f,
                "{side} has {actual} independent samples; at least {minimum} required"
            ),
            Self::EmptySampleId { side, index } => {
                write!(f, "{side} sample at index {index} has an empty ID")
            }
            Self::DuplicateSampleId { side, sample_id } => {
                write!(f, "{side} sample ID is duplicated: {sample_id}")
            }
            Self::NonFiniteValue { side, sample_id, value } => write!(
                f,
                "{side} sample {sample_id} has a non-finite value: {value}"
            ),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Compare independent empirical windows with independent simulation runs.
///
/// Sample IDs must be unique within each side. A paired estimator is not used:
/// same-sized sample sets do not imply that an empirical window and a
/// simulation run are paired observations.
pub fn compare_metric(
    metric: MetricSpec,
    observed: &SampleSet,
    simulated: &SampleSet,
) -> Result<OperationalComparison, ValidationError> {
    validate_spec(&metric)?;
    validate_sample_set("observed", &metric.unit, observed)?;
    validate_sample_set("simulated", &metric.unit, simulated)?;

    let observed_values = sorted_values(observed);
    let simulated_values = sorted_values(simulated);
    let observed_summary = summarize(&observed_values);
    let simulated_summary = summarize(&simulated_values);

    Ok(OperationalComparison {
        metric,
        observed: observed_summary,
        simulated: simulated_summary,
        mean_delta_simulated_minus_observed: simulated_summary.mean - observed_summary.mean,
        p10_delta_simulated_minus_observed: simulated_summary.quantiles.p10
            - observed_summary.quantiles.p10,
        median_delta_simulated_minus_observed: simulated_summary.median
            - observed_summary.median,
        p90_delta_simulated_minus_observed: simulated_summary.quantiles.p90
            - observed_summary.quantiles.p90,
        p99_delta_simulated_minus_observed: simulated_summary.quantiles.p99
            - observed_summary.quantiles.p99,
        ks_distance: kolmogorov_smirnov_distance(&observed_values, &simulated_values),
        wasserstein_1: wasserstein_one(&observed_values, &simulated_values),
    })
}

fn validate_spec(metric: &MetricSpec) -> Result<(), ValidationError> {
    if metric.metric_id.trim().is_empty() {
        return Err(ValidationError::EmptyMetricId);
    }
    if metric.unit.trim().is_empty() {
        return Err(ValidationError::EmptyUnit);
    }
    if metric.stratum_id.trim().is_empty() {
        return Err(ValidationError::EmptyStratumId);
    }
    Ok(())
}

fn validate_sample_set(
    side: &'static str,
    expected_unit: &str,
    sample_set: &SampleSet,
) -> Result<(), ValidationError> {
    if sample_set.unit != expected_unit {
        return Err(ValidationError::UnitMismatch {
            side,
            expected: expected_unit.to_owned(),
            actual: sample_set.unit.clone(),
        });
    }
    if sample_set.samples.len() < MIN_INDEPENDENT_SAMPLES {
        return Err(ValidationError::TooFewIndependentSamples {
            side,
            actual: sample_set.samples.len(),
            minimum: MIN_INDEPENDENT_SAMPLES,
        });
    }

    let mut ids = std::collections::BTreeSet::new();
    for (index, sample) in sample_set.samples.iter().enumerate() {
        if sample.sample_id.trim().is_empty() {
            return Err(ValidationError::EmptySampleId { side, index });
        }
        if !ids.insert(sample.sample_id.as_str()) {
            return Err(ValidationError::DuplicateSampleId {
                side,
                sample_id: sample.sample_id.clone(),
            });
        }
        if !sample.value.is_finite() {
            return Err(ValidationError::NonFiniteValue {
                side,
                sample_id: sample.sample_id.clone(),
                value: sample.value,
            });
        }
    }
    Ok(())
}

fn sorted_values(sample_set: &SampleSet) -> Vec<f64> {
    let mut values = sample_set.samples.iter().map(|sample| sample.value).collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values
}

fn summarize(sorted: &[f64]) -> SampleSummary {
    let sample_count = sorted.len();
    let mean = sorted.iter().sum::<f64>() / sample_count as f64;
    let sum_squared = sorted
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    let sample_variance = sum_squared / (sample_count - 1) as f64;
    let quantiles = QuantileSummary {
        p10: quantile(sorted, 0.10),
        p50: quantile(sorted, 0.50),
        p90: quantile(sorted, 0.90),
        p99: quantile(sorted, 0.99),
    };

    SampleSummary {
        sample_count,
        mean,
        sample_standard_deviation: sample_variance.sqrt(),
        median: quantiles.p50,
        quantiles,
        mean_99_ci_half_width: conservative_t_99_critical(sample_count)
            * (sample_variance / sample_count as f64).sqrt(),
    }
}

/// Conservative two-sided 99% t critical values. Each bucket uses the
/// critical value for the smallest degrees of freedom in that bucket.
/// Samples smaller than 30 are rejected by the public validation contract.
fn conservative_t_99_critical(sample_count: usize) -> f64 {
    match sample_count {
        30..=39 => 2.756,
        40..=59 => 2.708,
        60..=119 => 2.660,
        _ => 2.617,
    }
}

/// Type-7 linear-interpolated sample quantile.
fn quantile(sorted: &[f64], probability: f64) -> f64 {
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        return sorted[lower];
    }
    let fraction = position - lower as f64;
    sorted[lower] + fraction * (sorted[upper] - sorted[lower])
}

fn kolmogorov_smirnov_distance(left: &[f64], right: &[f64]) -> f64 {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut maximum = 0.0_f64;

    while i < left.len() || j < right.len() {
        let next = match (left.get(i), right.get(j)) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };
        while i < left.len() && left[i] <= *next {
            i += 1;
        }
        while j < right.len() && right[j] <= *next {
            j += 1;
        }
        let cdf_left = i as f64 / left.len() as f64;
        let cdf_right = j as f64 / right.len() as f64;
        maximum = maximum.max((cdf_left - cdf_right).abs());
    }
    maximum
}

/// Exact integral of the absolute empirical-CDF difference on the real line.
fn wasserstein_one(left: &[f64], right: &[f64]) -> f64 {
    let mut i = 0usize;
    let mut j = 0usize;
    let mut previous: Option<f64> = None;
    let mut cdf_delta = 0.0_f64;
    let mut distance = 0.0_f64;

    while i < left.len() || j < right.len() {
        let next = match (left.get(i), right.get(j)) {
            (Some(a), Some(b)) => a.min(b),
            (Some(a), None) => a,
            (None, Some(b)) => b,
            (None, None) => break,
        };

        if let Some(previous_value) = previous {
            distance += cdf_delta.abs() * (*next - previous_value);
        }

        while i < left.len() && left[i] == *next {
            i += 1;
        }
        while j < right.len() && right[j] == *next {
            j += 1;
        }
        cdf_delta = i as f64 / left.len() as f64 - j as f64 / right.len() as f64;
        previous = Some(*next);
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(prefix: &str, values: &[f64], unit: &str) -> SampleSet {
        SampleSet {
            unit: unit.to_owned(),
            samples: values
                .iter()
                .enumerate()
                .map(|(index, value)| NumericSample {
                    sample_id: format!("{prefix}_{index:03}"),
                    value: *value,
                })
                .collect(),
        }
    }

    fn metric(stratum: &str) -> MetricSpec {
        MetricSpec {
            metric_id: "activity".into(),
            unit: "posts/day".into(),
            stratum_id: stratum.into(),
        }
    }

    fn values_0_to_29() -> Vec<f64> {
        (0..30).map(f64::from).collect()
    }

    #[test]
    fn identical_distributions_have_zero_distances_and_deltas() {
        let values = values_0_to_29();
        let result = compare_metric(
            metric("all"),
            &set("obs", &values, "posts/day"),
            &set("sim", &values, "posts/day"),
        )
        .unwrap();

        assert_eq!(result.mean_delta_simulated_minus_observed, 0.0);
        assert_eq!(result.ks_distance, 0.0);
        assert_eq!(result.wasserstein_1, 0.0);
        assert_eq!(result.median_delta_simulated_minus_observed, 0.0);
        assert_eq!(result.observed.mean_99_ci_half_width, result.simulated.mean_99_ci_half_width);
    }

    #[test]
    fn location_shift_changes_mean_and_distribution_distances() {
        let observed = values_0_to_29();
        let simulated = observed.iter().map(|value| value + 1.0).collect::<Vec<_>>();
        let result = compare_metric(
            metric("all"),
            &set("obs", &observed, "posts/day"),
            &set("sim", &simulated, "posts/day"),
        )
        .unwrap();

        assert!((result.mean_delta_simulated_minus_observed - 1.0).abs() < 1e-12);
        assert!(result.ks_distance > 0.0);
        assert!((result.wasserstein_1 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn tail_difference_is_visible_even_when_median_is_unchanged() {
        let observed = vec![0.0; 30];
        let mut simulated = vec![0.0; 29];
        simulated.push(100.0);
        let result = compare_metric(
            metric("root_posts"),
            &set("obs", &observed, "posts/day"),
            &set("sim", &simulated, "posts/day"),
        )
        .unwrap();

        assert_eq!(result.median_delta_simulated_minus_observed, 0.0);
        assert!((result.mean_delta_simulated_minus_observed - (100.0 / 30.0)).abs() < 1e-12);
        assert!(result.wasserstein_1 > 0.0);
        assert!(result.ks_distance > 0.0);
    }

    #[test]
    fn strata_remain_distinct_instead_of_being_pooled() {
        let observed = values_0_to_29();
        let root_sim = observed.iter().map(|value| value + 10.0).collect::<Vec<_>>();
        let comment_sim = observed.clone();

        let root = compare_metric(
            metric("root_posts"),
            &set("obs_root", &observed, "posts/day"),
            &set("sim_root", &root_sim, "posts/day"),
        )
        .unwrap();
        let comments = compare_metric(
            metric("comments"),
            &set("obs_comments", &observed, "posts/day"),
            &set("sim_comments", &comment_sim, "posts/day"),
        )
        .unwrap();

        assert!((root.mean_delta_simulated_minus_observed - 10.0).abs() < 1e-12);
        assert_eq!(comments.mean_delta_simulated_minus_observed, 0.0);
        assert_ne!(root.metric.stratum_id, comments.metric.stratum_id);
    }

    #[test]
    fn mismatched_units_fail_closed() {
        let values = values_0_to_29();
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day"),
                &set("sim", &values, "users/day"),
            ),
            Err(ValidationError::UnitMismatch { side: "simulated", .. })
        ));
    }

    #[test]
    fn too_few_samples_fail_closed() {
        let values = vec![1.0; MIN_INDEPENDENT_SAMPLES - 1];
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day"),
                &set("sim", &values, "posts/day"),
            ),
            Err(ValidationError::TooFewIndependentSamples { .. })
        ));
    }

    #[test]
    fn duplicate_ids_and_non_finite_values_fail_closed() {
        let values = values_0_to_29();
        let mut duplicate = set("obs", &values, "posts/day");
        duplicate.samples[1].sample_id = duplicate.samples[0].sample_id.clone();
        assert!(matches!(
            compare_metric(
                metric("all"),
                &duplicate,
                &set("sim", &values, "posts/day"),
            ),
            Err(ValidationError::DuplicateSampleId { .. })
        ));

        let mut non_finite = set("sim", &values, "posts/day");
        non_finite.samples[5].value = f64::NAN;
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day"),
                &non_finite,
            ),
            Err(ValidationError::NonFiniteValue { .. })
        ));
    }

    #[test]
    fn empty_metric_scope_and_unit_fail_closed() {
        let values = values_0_to_29();
        let mut invalid = metric("all");
        invalid.stratum_id.clear();
        assert!(matches!(
            compare_metric(
                invalid,
                &set("obs", &values, "posts/day"),
                &set("sim", &values, "posts/day"),
            ),
            Err(ValidationError::EmptyStratumId)
        ));
    }
}
