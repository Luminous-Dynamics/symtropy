// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Empirical cross-site dependence analysis for data-centre fleet power.
//!
//! The evaluator tests whether independent-site aggregation is a defensible
//! approximation for a frozen, aligned set of power traces. It reports sample
//! covariance, correlation, lagged dependence, aggregate variance, ramp and
//! a declared threshold-exceedance frequency.
//!
//! This module does not fit a probability distribution, forecast future load,
//! synthesize correlated traces, infer causal workload coupling, or estimate
//! grid reserve by itself. Results describe the supplied trace window only.
//! Clock alignment evidence must be supplied by the caller; this crate checks
//! its declared bounds and never edits timestamps, interpolates or resamples.

use core::fmt;

use serde::Serialize;

/// One site's regularly sampled signed PCC or other declared electrical-boundary
/// power series. Values use kW and interval-average semantics.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetSiteSeries {
    site_id: String,
    grid_region_id: String,
    measurement_boundary_id: String,
    source_digest: String,
    alignment_model_id: String,
    start_timestamp_ns: u64,
    interval_duration_ns: u64,
    clock_uncertainty_ns: u64,
    samples_kw: Vec<f64>,
}

impl FleetSiteSeries {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        site_id: impl Into<String>,
        grid_region_id: impl Into<String>,
        measurement_boundary_id: impl Into<String>,
        source_digest: impl Into<String>,
        alignment_model_id: impl Into<String>,
        start_timestamp_ns: u64,
        interval_duration_ns: u64,
        clock_uncertainty_ns: u64,
        samples_kw: Vec<f64>,
    ) -> Result<Self, FleetCorrelationError> {
        let series = Self {
            site_id: site_id.into(),
            grid_region_id: grid_region_id.into(),
            measurement_boundary_id: measurement_boundary_id.into(),
            source_digest: source_digest.into(),
            alignment_model_id: alignment_model_id.into(),
            start_timestamp_ns,
            interval_duration_ns,
            clock_uncertainty_ns,
            samples_kw,
        };
        for (field, value) in [
            ("site ID", series.site_id.as_str()),
            ("grid region ID", series.grid_region_id.as_str()),
            ("measurement boundary ID", series.measurement_boundary_id.as_str()),
            ("source digest", series.source_digest.as_str()),
            ("alignment model ID", series.alignment_model_id.as_str()),
        ] {
            require_id(field, value)?;
        }
        if interval_duration_ns == 0 {
            return Err(FleetCorrelationError::InvalidIntervalDuration);
        }
        if series.samples_kw.len() < 2 {
            return Err(FleetCorrelationError::InsufficientSamples {
                site_id: series.site_id.clone(),
                observed: series.samples_kw.len(),
                required: 2,
            });
        }
        if let Some((index, value)) = series
            .samples_kw
            .iter()
            .copied()
            .enumerate()
            .find(|(_, value)| !value.is_finite())
        {
            return Err(FleetCorrelationError::NonFiniteSample {
                site_id: series.site_id.clone(),
                index,
                value,
            });
        }
        let final_offset = (series.samples_kw.len() - 1) as u64;
        start_timestamp_ns
            .checked_add(
                interval_duration_ns
                    .checked_mul(final_offset)
                    .ok_or(FleetCorrelationError::TimestampOverflow)?,
            )
            .ok_or(FleetCorrelationError::TimestampOverflow)?;

        Ok(series)
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn samples_kw(&self) -> &[f64] {
        &self.samples_kw
    }
}

/// Policy for a frozen empirical dependence analysis.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetCorrelationContract {
    maximum_pairwise_clock_uncertainty_ns: u64,
    maximum_lag_intervals: usize,
    minimum_overlap_samples: usize,
    peak_threshold_kw: f64,
    ramp_quantile_basis_points: u16,
}

impl FleetCorrelationContract {
    pub fn new(
        maximum_pairwise_clock_uncertainty_ns: u64,
        maximum_lag_intervals: usize,
        minimum_overlap_samples: usize,
        peak_threshold_kw: f64,
        ramp_quantile_basis_points: u16,
    ) -> Result<Self, FleetCorrelationError> {
        if minimum_overlap_samples < 2 {
            return Err(FleetCorrelationError::InvalidMinimumOverlap {
                minimum_overlap_samples,
            });
        }
        if !peak_threshold_kw.is_finite() {
            return Err(FleetCorrelationError::InvalidContractQuantity {
                field: "peak threshold",
                value: peak_threshold_kw,
            });
        }
        if !(1..=10_000).contains(&ramp_quantile_basis_points) {
            return Err(FleetCorrelationError::InvalidRampQuantile {
                ramp_quantile_basis_points,
            });
        }
        Ok(Self {
            maximum_pairwise_clock_uncertainty_ns,
            maximum_lag_intervals,
            minimum_overlap_samples,
            peak_threshold_kw,
            ramp_quantile_basis_points,
        })
    }
}

/// Per-site empirical moments.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SiteMomentReceipt {
    pub site_id: String,
    pub grid_region_id: String,
    pub sample_count: usize,
    pub mean_power_kw: f64,
    pub sample_variance_kw2: f64,
    pub sample_standard_deviation_kw: f64,
    pub maximum_power_kw: f64,
    pub minimum_power_kw: f64,
}

/// Empirical contemporaneous and bounded-lag dependence between two sites.
///
/// Positive lag means the second site is evaluated later:
/// corr(A[t], B[t + lag]). A positive lag therefore means B's pattern follows
/// A's pattern by that number of intervals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairDependenceReceipt {
    pub site_a_id: String,
    pub site_b_id: String,
    pub contemporaneous_correlation: Option<f64>,
    pub strongest_positive_correlation: Option<f64>,
    pub strongest_positive_lag_intervals: Option<isize>,
    pub strongest_negative_correlation: Option<f64>,
    pub strongest_negative_lag_intervals: Option<isize>,
    pub evaluated_lag_count: usize,
}

/// Replayable analysis of one aligned multi-site time window.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FleetCorrelationReport {
    pub alignment_model_id: String,
    pub site_count: usize,
    pub sample_count: usize,
    pub start_timestamp_ns: u64,
    pub interval_duration_ns: u64,
    pub maximum_observed_pairwise_clock_uncertainty_ns: u64,
    pub maximum_lag_intervals: usize,
    pub site_moments: Vec<SiteMomentReceipt>,
    pub pair_dependence: Vec<PairDependenceReceipt>,
    pub portfolio_mean_power_kw: f64,
    pub portfolio_sample_variance_kw2: f64,
    pub independent_site_variance_sum_kw2: f64,
    pub covariance_excess_variance_kw2: f64,
    /// Actual portfolio variance divided by the sum of site variances.
    /// Values above 1 indicate that contemporaneous covariance amplifies
    /// variation relative to the independent-site assumption.
    pub variance_ratio_vs_independence: Option<f64>,
    pub portfolio_peak_power_kw: f64,
    pub portfolio_minimum_power_kw: f64,
    pub threshold_kw: f64,
    pub threshold_exceedance_fraction: f64,
    pub maximum_absolute_portfolio_ramp_kw_per_second: f64,
    pub portfolio_absolute_ramp_quantile_kw_per_second: f64,
    pub variance_identity_residual_kw2: f64,
    pub sub_interval_transients_unverified: bool,
}

/// Invalid trace, policy or arithmetic input.
#[derive(Debug, Clone, PartialEq)]
pub enum FleetCorrelationError {
    MissingIdentity { field: &'static str },
    InvalidIntervalDuration,
    InvalidMinimumOverlap { minimum_overlap_samples: usize },
    InvalidRampQuantile { ramp_quantile_basis_points: u16 },
    InvalidContractQuantity { field: &'static str, value: f64 },
    InsufficientSites { observed: usize, required: usize },
    InsufficientSamples { site_id: String, observed: usize, required: usize },
    NonFiniteSample { site_id: String, index: usize, value: f64 },
    DuplicateSiteId { site_id: String },
    AlignmentModelMismatch { site_id: String },
    TimestampAxisMismatch { site_id: String },
    IntervalDurationMismatch { site_id: String },
    SampleCountMismatch { site_id: String, expected: usize, observed: usize },
    ClockUncertaintyExceedsContract { observed_ns: u64, maximum_ns: u64 },
    NoLagHasMinimumOverlap { requested_lag: usize, minimum_overlap_samples: usize, sample_count: usize },
    TimestampOverflow,
    NumericalOverflow(&'static str),
}

impl fmt::Display for FleetCorrelationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidIntervalDuration => write!(f, "sample interval duration must be positive"),
            Self::InvalidMinimumOverlap { minimum_overlap_samples } => write!(f, "minimum lag overlap must be at least 2; got {minimum_overlap_samples}"),
            Self::InvalidRampQuantile { ramp_quantile_basis_points } => write!(f, "ramp quantile must be in 1..=10000 basis points; got {ramp_quantile_basis_points}"),
            Self::InvalidContractQuantity { field, value } => write!(f, "{field} must be finite; got {value}"),
            Self::InsufficientSites { observed, required } => write!(f, "fleet requires at least {required} sites; got {observed}"),
            Self::InsufficientSamples { site_id, observed, required } => write!(f, "site {site_id} has {observed} samples; at least {required} are required"),
            Self::NonFiniteSample { site_id, index, value } => write!(f, "site {site_id} sample {index} is non-finite: {value}"),
            Self::DuplicateSiteId { site_id } => write!(f, "duplicate fleet site ID: {site_id}"),
            Self::AlignmentModelMismatch { site_id } => write!(f, "site {site_id} uses a different clock alignment model"),
            Self::TimestampAxisMismatch { site_id } => write!(f, "site {site_id} start timestamp does not match the fleet time axis"),
            Self::IntervalDurationMismatch { site_id } => write!(f, "site {site_id} interval duration does not match the fleet time axis"),
            Self::SampleCountMismatch { site_id, expected, observed } => write!(f, "site {site_id} has {observed} samples; expected {expected}"),
            Self::ClockUncertaintyExceedsContract { observed_ns, maximum_ns } => write!(f, "maximum pairwise clock uncertainty {observed_ns} ns exceeds contract {maximum_ns} ns"),
            Self::NoLagHasMinimumOverlap { requested_lag, minimum_overlap_samples, sample_count } => write!(f, "maximum lag {requested_lag} leaves insufficient overlap of {minimum_overlap_samples} samples for trace length {sample_count}"),
            Self::TimestampOverflow => write!(f, "timestamp arithmetic overflow"),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
        }
    }
}

impl std::error::Error for FleetCorrelationError {}

/// Evaluate empirical cross-site dependence and portfolio variability.
///
/// The input traces must already be corrected to the same physical time axis
/// using the same declared alignment model. The worst-case pair clock
/// uncertainty is the sum of the two largest individual uncertainty bounds.
pub fn evaluate_fleet_correlation(
    sites: &[FleetSiteSeries],
    contract: &FleetCorrelationContract,
) -> Result<FleetCorrelationReport, FleetCorrelationError> {
    if sites.len() < 2 {
        return Err(FleetCorrelationError::InsufficientSites {
            observed: sites.len(),
            required: 2,
        });
    }
    let first = &sites[0];
    let sample_count = first.samples_kw.len();
    let start_timestamp_ns = first.start_timestamp_ns;
    let interval_duration_ns = first.interval_duration_ns;
    let alignment_model_id = first.alignment_model_id.as_str();

    let mut ids = std::collections::BTreeSet::new();
    let mut clock_uncertainties = Vec::with_capacity(sites.len());
    for site in sites {
        if !ids.insert(site.site_id.as_str()) {
            return Err(FleetCorrelationError::DuplicateSiteId {
                site_id: site.site_id.clone(),
            });
        }
        if site.alignment_model_id != alignment_model_id {
            return Err(FleetCorrelationError::AlignmentModelMismatch {
                site_id: site.site_id.clone(),
            });
        }
        if site.start_timestamp_ns != start_timestamp_ns {
            return Err(FleetCorrelationError::TimestampAxisMismatch {
                site_id: site.site_id.clone(),
            });
        }
        if site.interval_duration_ns != interval_duration_ns {
            return Err(FleetCorrelationError::IntervalDurationMismatch {
                site_id: site.site_id.clone(),
            });
        }
        if site.samples_kw.len() != sample_count {
            return Err(FleetCorrelationError::SampleCountMismatch {
                site_id: site.site_id.clone(),
                expected: sample_count,
                observed: site.samples_kw.len(),
            });
        }
        clock_uncertainties.push(site.clock_uncertainty_ns);
    }
    clock_uncertainties.sort_unstable_by(|a, b| b.cmp(a));
    let maximum_observed_pairwise_clock_uncertainty_ns =
        clock_uncertainties[0].saturating_add(clock_uncertainties[1]);
    if maximum_observed_pairwise_clock_uncertainty_ns
        > contract.maximum_pairwise_clock_uncertainty_ns
    {
        return Err(FleetCorrelationError::ClockUncertaintyExceedsContract {
            observed_ns: maximum_observed_pairwise_clock_uncertainty_ns,
            maximum_ns: contract.maximum_pairwise_clock_uncertainty_ns,
        });
    }
    if contract.minimum_overlap_samples > sample_count {
        return Err(FleetCorrelationError::NoLagHasMinimumOverlap {
            requested_lag: contract.maximum_lag_intervals,
            minimum_overlap_samples: contract.minimum_overlap_samples,
            sample_count,
        });
    }
    if contract.maximum_lag_intervals > 0
        && sample_count.saturating_sub(contract.maximum_lag_intervals)
            < contract.minimum_overlap_samples
    {
        return Err(FleetCorrelationError::NoLagHasMinimumOverlap {
            requested_lag: contract.maximum_lag_intervals,
            minimum_overlap_samples: contract.minimum_overlap_samples,
            sample_count,
        });
    }

    let mut site_moments = Vec::with_capacity(sites.len());
    for site in sites {
        let mean = mean(&site.samples_kw);
        let variance = sample_variance(&site.samples_kw, mean);
        site_moments.push(SiteMomentReceipt {
            site_id: site.site_id.clone(),
            grid_region_id: site.grid_region_id.clone(),
            sample_count,
            mean_power_kw: mean,
            sample_variance_kw2: variance,
            sample_standard_deviation_kw: variance.sqrt(),
            maximum_power_kw: site.samples_kw.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            minimum_power_kw: site.samples_kw.iter().copied().fold(f64::INFINITY, f64::min),
        });
    }

    let mut portfolio = vec![0.0; sample_count];
    for site in sites {
        for (total, power) in portfolio.iter_mut().zip(&site.samples_kw) {
            *total += power;
        }
    }
    if let Some((index, _)) = portfolio.iter().enumerate().find(|(_, x)| !x.is_finite()) {
        return Err(FleetCorrelationError::NumericalOverflow("portfolio power"));
    }

    let portfolio_mean_power_kw = mean(&portfolio);
    let portfolio_sample_variance_kw2 = sample_variance(&portfolio, portfolio_mean_power_kw);
    let independent_site_variance_sum_kw2 =
        site_moments.iter().map(|site| site.sample_variance_kw2).sum::<f64>();
    let covariance_excess_variance_kw2 =
        portfolio_sample_variance_kw2 - independent_site_variance_sum_kw2;
    let variance_ratio_vs_independence = if independent_site_variance_sum_kw2 > f64::EPSILON {
        Some(portfolio_sample_variance_kw2 / independent_site_variance_sum_kw2)
    } else {
        None
    };
    let variance_identity_residual_kw2 = portfolio_sample_variance_kw2
        - independent_site_variance_sum_kw2
        - covariance_excess_variance_kw2;

    let mut pair_dependence = Vec::new();
    for i in 0..sites.len() {
        for j in (i + 1)..sites.len() {
            pair_dependence.push(pair_dependence_receipt(
                &sites[i],
                &sites[j],
                contract.maximum_lag_intervals,
                contract.minimum_overlap_samples,
            ));
        }
    }

    let mut absolute_ramps = Vec::with_capacity(sample_count - 1);
    let interval_seconds = interval_duration_ns as f64 / 1_000_000_000.0;
    for pair in portfolio.windows(2) {
        absolute_ramps.push((pair[1] - pair[0]).abs() / interval_seconds);
    }
    absolute_ramps.sort_by(f64::total_cmp);
    let maximum_absolute_portfolio_ramp_kw_per_second =
        absolute_ramps.last().copied().unwrap_or(0.0);
    let quantile_index = ((contract.ramp_quantile_basis_points as usize * absolute_ramps.len())
        .div_ceil(10_000))
        .saturating_sub(1)
        .min(absolute_ramps.len() - 1);
    let portfolio_absolute_ramp_quantile_kw_per_second = absolute_ramps[quantile_index];

    let threshold_exceedance_count = portfolio
        .iter()
        .filter(|power| **power >= contract.peak_threshold_kw)
        .count();
    Ok(FleetCorrelationReport {
        alignment_model_id: alignment_model_id.to_owned(),
        site_count: sites.len(),
        sample_count,
        start_timestamp_ns,
        interval_duration_ns,
        maximum_observed_pairwise_clock_uncertainty_ns,
        maximum_lag_intervals: contract.maximum_lag_intervals,
        site_moments,
        pair_dependence,
        portfolio_mean_power_kw,
        portfolio_sample_variance_kw2,
        independent_site_variance_sum_kw2,
        covariance_excess_variance_kw2,
        variance_ratio_vs_independence,
        portfolio_peak_power_kw: portfolio.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        portfolio_minimum_power_kw: portfolio.iter().copied().fold(f64::INFINITY, f64::min),
        threshold_kw: contract.peak_threshold_kw,
        threshold_exceedance_fraction: threshold_exceedance_count as f64 / sample_count as f64,
        maximum_absolute_portfolio_ramp_kw_per_second,
        portfolio_absolute_ramp_quantile_kw_per_second,
        variance_identity_residual_kw2,
        sub_interval_transients_unverified: true,
    })
}

fn pair_dependence_receipt(
    a: &FleetSiteSeries,
    b: &FleetSiteSeries,
    maximum_lag: usize,
    minimum_overlap: usize,
) -> PairDependenceReceipt {
    let contemporaneous_correlation = correlation(&a.samples_kw, &b.samples_kw);
    let mut strongest_positive_correlation: Option<f64> = None;
    let mut strongest_positive_lag_intervals: Option<isize> = None;
    let mut strongest_negative_correlation: Option<f64> = None;
    let mut strongest_negative_lag_intervals: Option<isize> = None;
    let mut evaluated_lag_count = 0usize;

    for lag in -(maximum_lag as isize)..=(maximum_lag as isize) {
        let (a_values, b_values) = lagged_overlap(&a.samples_kw, &b.samples_kw, lag);
        if a_values.len() < minimum_overlap {
            continue;
        }
        let Some(rho) = correlation(a_values, b_values) else {
            continue;
        };
        evaluated_lag_count += 1;
        if strongest_positive_correlation.is_none_or(|current| rho > current) {
            strongest_positive_correlation = Some(rho);
            strongest_positive_lag_intervals = Some(lag);
        }
        if strongest_negative_correlation.is_none_or(|current| rho < current) {
            strongest_negative_correlation = Some(rho);
            strongest_negative_lag_intervals = Some(lag);
        }
    }

    PairDependenceReceipt {
        site_a_id: a.site_id.clone(),
        site_b_id: b.site_id.clone(),
        contemporaneous_correlation,
        strongest_positive_correlation,
        strongest_positive_lag_intervals,
        strongest_negative_correlation,
        strongest_negative_lag_intervals,
        evaluated_lag_count,
    }
}

fn lagged_overlap<'a>(a: &'a [f64], b: &'a [f64], lag: isize) -> (&'a [f64], &'a [f64]) {
    if lag >= 0 {
        let offset = lag as usize;
        (&a[..(a.len() - offset)], &b[offset..])
    } else {
        let offset = lag.unsigned_abs();
        (&a[offset..], &b[..(b.len() - offset)])
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_variance(values: &[f64], values_mean: f64) -> f64 {
    values
        .iter()
        .map(|value| (value - values_mean).powi(2))
        .sum::<f64>()
        / (values.len() - 1) as f64
}

fn correlation(a: &[f64], b: &[f64]) -> Option<f64> {
    if a.len() != b.len() || a.len() < 2 {
        return None;
    }
    let mean_a = mean(a);
    let mean_b = mean(b);
    let mut covariance = 0.0;
    let mut variance_a = 0.0;
    let mut variance_b = 0.0;
    for (x, y) in a.iter().zip(b) {
        covariance += (x - mean_a) * (y - mean_b);
        variance_a += (x - mean_a).powi(2);
        variance_b += (y - mean_b).powi(2);
    }
    if variance_a <= f64::EPSILON || variance_b <= f64::EPSILON {
        return None;
    }
    Some((covariance / (variance_a * variance_b).sqrt()).clamp(-1.0, 1.0))
}

fn require_id(field: &'static str, value: &str) -> Result<(), FleetCorrelationError> {
    if value.trim().is_empty() {
        Err(FleetCorrelationError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
