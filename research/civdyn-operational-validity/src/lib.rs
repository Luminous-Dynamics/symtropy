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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricSpec {
    pub metric_id: String,
    pub unit: String,
    /// A separate scope such as root_posts, comments, region_01 or network_core.
    pub stratum_id: String,
    /// Declared independent unit for empirical observations (e.g. non-overlapping time window).
    pub observed_independent_unit: String,
    /// Declared independent unit for simulation replications (e.g. independent seed run).
    pub simulated_independent_unit: String,
    /// Minimum sample counts are design-specific, not universal constants.
    pub minimum_observed_samples: usize,
    pub minimum_simulated_samples: usize,
    /// Dependence assessment and sample-size justification from the source design.
    pub dependence_assessment: String,
    pub sample_size_rationale: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumericSample {
    pub sample_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SampleSet {
    pub unit: String,
    /// Caller-declared independent unit; this is checked against MetricSpec but cannot be proven from IDs.
    pub independent_unit: String,
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
    EmptyDependenceAssessment,
    EmptySampleSizeRationale,
    MinimumSampleCountTooSmall { side: &'static str, minimum: usize },
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
    IndependentUnitMismatch {
        side: &'static str,
        expected: String,
        actual: String,
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
            Self::EmptyDependenceAssessment => write!(f, "dependence_assessment must not be empty"),
            Self::EmptySampleSizeRationale => write!(f, "sample_size_rationale must not be empty"),
            Self::MinimumSampleCountTooSmall { side, minimum } => write!(f, "{side} minimum sample count must be at least 2 for variance and t intervals, got {minimum}"),
            Self::UnitMismatch {
                side,
                expected,
                actual,
            } => write!(
                f,
                "{side} sample unit mismatch: expected {expected}, got {actual}"
            ),
            Self::TooFewIndependentSamples {
                side,
                actual,
                minimum,
            } => write!(
                f,
                "{side} has {actual} independent samples; at least {minimum} required by this metric design"
            ),
            Self::IndependentUnitMismatch { side, expected, actual } => write!(
                f,
                "{side} independent unit mismatch: expected {expected}, got {actual}"
            ),
            Self::EmptySampleId { side, index } => {
                write!(f, "{side} sample at index {index} has an empty ID")
            }
            Self::DuplicateSampleId { side, sample_id } => {
                write!(f, "{side} sample ID is duplicated: {sample_id}")
            }
            Self::NonFiniteValue {
                side,
                sample_id,
                value,
            } => write!(
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
    validate_sample_set(
        "observed",
        &metric.unit,
        &metric.observed_independent_unit,
        metric.minimum_observed_samples,
        observed,
    )?;
    validate_sample_set(
        "simulated",
        &metric.unit,
        &metric.simulated_independent_unit,
        metric.minimum_simulated_samples,
        simulated,
    )?;

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
        median_delta_simulated_minus_observed: simulated_summary.median - observed_summary.median,
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
    if metric.dependence_assessment.trim().is_empty() {
        return Err(ValidationError::EmptyDependenceAssessment);
    }
    if metric.sample_size_rationale.trim().is_empty() {
        return Err(ValidationError::EmptySampleSizeRationale);
    }
    for (side, minimum) in [
        ("observed", metric.minimum_observed_samples),
        ("simulated", metric.minimum_simulated_samples),
    ] {
        if minimum < 2 {
            return Err(ValidationError::MinimumSampleCountTooSmall { side, minimum });
        }
    }
    Ok(())
}

fn validate_sample_set(
    side: &'static str,
    expected_unit: &str,
    expected_independent_unit: &str,
    minimum_samples: usize,
    sample_set: &SampleSet,
) -> Result<(), ValidationError> {
    if sample_set.unit != expected_unit {
        return Err(ValidationError::UnitMismatch {
            side,
            expected: expected_unit.to_owned(),
            actual: sample_set.unit.clone(),
        });
    }
    if sample_set.independent_unit != expected_independent_unit {
        return Err(ValidationError::IndependentUnitMismatch {
            side,
            expected: expected_independent_unit.to_owned(),
            actual: sample_set.independent_unit.clone(),
        });
    }
    if sample_set.samples.len() < minimum_samples {
        return Err(ValidationError::TooFewIndependentSamples {
            side,
            actual: sample_set.samples.len(),
            minimum: minimum_samples,
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
    let mut values = sample_set
        .samples
        .iter()
        .map(|sample| sample.value)
        .collect::<Vec<_>>();
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

/// Conservative two-sided 99% Student-t critical values.
/// For n=2..29 the values are the corresponding df=n-1 quantiles.
/// Larger ranges use the critical value for the smallest df in the bucket,
/// remaining conservative as the sample size increases.
fn conservative_t_99_critical(sample_count: usize) -> f64 {
    match sample_count {
        2 => 63.657,
        3 => 9.925,
        4 => 5.841,
        5 => 4.604,
        6 => 4.032,
        7 => 3.707,
        8 => 3.499,
        9 => 3.355,
        10 => 3.250,
        11 => 3.169,
        12 => 3.106,
        13 => 3.055,
        14 => 3.012,
        15 => 2.977,
        16 => 2.947,
        17 => 2.921,
        18 => 2.898,
        19 => 2.878,
        20 => 2.861,
        21 => 2.845,
        22 => 2.831,
        23 => 2.819,
        24 => 2.807,
        25 => 2.797,
        26 => 2.787,
        27 => 2.779,
        28 => 2.771,
        29 => 2.763,
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
            (Some(a), Some(b)) => {
                if a.total_cmp(b).is_le() {
                    *a
                } else {
                    *b
                }
            }
            (Some(a), None) => *a,
            (None, Some(b)) => *b,
            (None, None) => break,
        };
        while i < left.len() && left[i] <= next {
            i += 1;
        }
        while j < right.len() && right[j] <= next {
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
            (Some(a), Some(b)) => {
                if a.total_cmp(b).is_le() {
                    *a
                } else {
                    *b
                }
            }
            (Some(a), None) => *a,
            (None, Some(b)) => *b,
            (None, None) => break,
        };

        if let Some(previous_value) = previous {
            distance += cdf_delta.abs() * (next - previous_value);
        }

        while i < left.len() && left[i] == next {
            i += 1;
        }
        while j < right.len() && right[j] == next {
            j += 1;
        }
        cdf_delta = i as f64 / left.len() as f64 - j as f64 / right.len() as f64;
        previous = Some(next);
    }
    distance
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(prefix: &str, values: &[f64], unit: &str, independent_unit: &str) -> SampleSet {
        SampleSet {
            unit: unit.to_owned(),
            independent_unit: independent_unit.to_owned(),
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
            observed_independent_unit: "non_overlapping_empirical_window".into(),
            simulated_independent_unit: "independent_simulation_run".into(),
            minimum_observed_samples: 30,
            minimum_simulated_samples: 30,
            dependence_assessment: "Empirical windows do not overlap; each simulation run uses an independent seed.".into(),
            sample_size_rationale: "This synthetic fixture mirrors the 30-window/30-run source study protocol; a different source requires its own power/dependence rationale.".into(),
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
            &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
            &set("sim", &values, "posts/day", "independent_simulation_run"),
        )
        .unwrap();

        assert_eq!(result.mean_delta_simulated_minus_observed, 0.0);
        assert_eq!(result.ks_distance, 0.0);
        assert_eq!(result.wasserstein_1, 0.0);
        assert_eq!(result.median_delta_simulated_minus_observed, 0.0);
        assert_eq!(
            result.observed.mean_99_ci_half_width,
            result.simulated.mean_99_ci_half_width
        );
    }

    #[test]
    fn location_shift_changes_mean_and_distribution_distances() {
        let observed = values_0_to_29();
        let simulated = observed.iter().map(|value| value + 1.0).collect::<Vec<_>>();
        let result = compare_metric(
            metric("all"),
            &set("obs", &observed, "posts/day", "non_overlapping_empirical_window"),
            &set("sim", &simulated, "posts/day", "independent_simulation_run"),
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
            &set("obs", &observed, "posts/day", "non_overlapping_empirical_window"),
            &set("sim", &simulated, "posts/day", "independent_simulation_run"),
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
        let root_sim = observed
            .iter()
            .map(|value| value + 10.0)
            .collect::<Vec<_>>();
        let comment_sim = observed.clone();

        let root = compare_metric(
            metric("root_posts"),
            &set("obs_root", &observed, "posts/day", "non_overlapping_empirical_window"),
            &set("sim_root", &root_sim, "posts/day", "independent_simulation_run"),
        )
        .unwrap();
        let comments = compare_metric(
            metric("comments"),
            &set("obs_comments", &observed, "posts/day", "non_overlapping_empirical_window"),
            &set("sim_comments", &comment_sim, "posts/day", "independent_simulation_run"),
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
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
                &set("sim", &values, "users/day", "independent_simulation_run"),
            ),
            Err(ValidationError::UnitMismatch {
                side: "simulated",
                ..
            })
        ));
    }

    #[test]
    fn too_few_samples_fail_closed() {
        let values = vec![1.0; 29];
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
                &set("sim", &values, "posts/day", "independent_simulation_run"),
            ),
            Err(ValidationError::TooFewIndependentSamples { .. })
        ));
    }

    #[test]
    fn metric_specific_sample_minima_can_be_below_thirty() {
        let values = (0..8).map(f64::from).collect::<Vec<_>>();
        let mut specification = metric("small_sample_reference");
        specification.minimum_observed_samples = 8;
        specification.minimum_simulated_samples = 8;
        specification.sample_size_rationale = "Eight independent units are justified by the predeclared exact small-sample reference fixture; not an empirical inference claim.".into();
        let result = compare_metric(
            specification,
            &set("obs_small", &values, "posts/day", "non_overlapping_empirical_window"),
            &set("sim_small", &values, "posts/day", "independent_simulation_run"),
        ).unwrap();
        assert_eq!(result.observed.sample_count, 8);
        assert!(result.observed.mean_99_ci_half_width > 0.0);
        let standard_error = result.observed.sample_standard_deviation / (8.0_f64).sqrt();
        assert!((result.observed.mean_99_ci_half_width - 3.499 * standard_error).abs() < 1e-9);
    }

    #[test]
    fn independent_unit_mismatch_fails_closed() {
        let values = values_0_to_29();
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day", "overlapping_empirical_rows"),
                &set("sim", &values, "posts/day", "independent_simulation_run"),
            ),
            Err(ValidationError::IndependentUnitMismatch { side: "observed", .. })
        ));
    }

    #[test]
    fn empty_design_rationale_and_minimum_below_two_fail_closed() {
        let values = values_0_to_29();
        let mut missing_rationale = metric("all");
        missing_rationale.sample_size_rationale.clear();
        assert!(matches!(
            compare_metric(
                missing_rationale,
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
                &set("sim", &values, "posts/day", "independent_simulation_run"),
            ),
            Err(ValidationError::EmptySampleSizeRationale)
        ));

        let mut invalid_minimum = metric("all");
        invalid_minimum.minimum_observed_samples = 1;
        assert!(matches!(
            compare_metric(
                invalid_minimum,
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
                &set("sim", &values, "posts/day", "independent_simulation_run"),
            ),
            Err(ValidationError::MinimumSampleCountTooSmall { side: "observed", .. })
        ));
    }

    #[test]
    fn duplicate_ids_and_non_finite_values_fail_closed() {
        let values = values_0_to_29();
        let mut duplicate = set("obs", &values, "posts/day");
        duplicate.samples[1].sample_id = duplicate.samples[0].sample_id.clone();
        assert!(matches!(
            compare_metric(metric("all"), &duplicate, &set("sim", &values, "posts/day", "independent_simulation_run"),),
            Err(ValidationError::DuplicateSampleId { .. })
        ));

        let mut non_finite = set("sim", &values, "posts/day");
        non_finite.samples[5].value = f64::NAN;
        assert!(matches!(
            compare_metric(
                metric("all"),
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
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
                &set("obs", &values, "posts/day", "non_overlapping_empirical_window"),
                &set("sim", &values, "posts/day", "independent_simulation_run"),
            ),
            Err(ValidationError::EmptyStratumId)
        ));
    }
}
