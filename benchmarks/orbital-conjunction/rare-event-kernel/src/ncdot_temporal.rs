// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic temporal aggregation of fixed-time Ncdot evidence.
//!
//! This module consumes already-computed, identity-bound fixed-time Ncdot
//! reports. It implements the NASA-compatible trapezoidal time integration of
//! expected collision rate and the rate-weighted significant-Lebedev-node
//! statistic:
//!
//!   Nc = integral Ncdot(t) dt
//!   AvgLebNumb = integral(Ncdot(t) * LebNumb(t), dt) / Nc
//!
//! It does not generate orbital states, covariance transport, PeakOverlapPos,
//! or adaptive ephemeris nodes. A caller-declared complete horizon is an input
//! assertion, not a proof produced by this aggregator.

use crate::KernelError;
use crate::lebedev::LebedevRule;
use crate::ncdot_sphere::NcdotSphereReport;

/// Immutable event, model and coordinate identity shared by all epoch reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NcdotTemporalIdentity {
    /// Stable conjunction/event identifier.
    pub event_id: String,
    /// Immutable event-definition revision.
    pub event_revision: String,
    /// SHA-256 digest of event, HBR, time-horizon and contact semantics.
    pub event_definition_sha256: String,
    /// Immutable physical-model revision.
    pub model_revision: String,
    /// SHA-256 digest of the exact physical-model/configuration inputs.
    pub model_sha256: String,
    /// SHA-256 digest of the target state/covariance probability measure.
    pub target_distribution_sha256: String,
    /// Declared relative-state reference-frame identifier.
    pub reference_frame: String,
    /// Declared time scale, e.g. TAI or UTC.
    pub time_scale: String,
}

/// Caller-declared completeness of the temporal event horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalHorizonCoverageClaim {
    /// Caller asserts that the supplied samples cover the declared full horizon.
    CallerDeclaredComplete,
    /// Caller knows that one or more parts of the declared horizon are absent.
    KnownIncomplete,
}

/// One immutable fixed-time Ncdot result and its provenance.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotTemporalSample {
    /// Epoch in seconds on the identity's declared time scale.
    pub epoch_seconds: f64,
    /// Stable unique sample identifier.
    pub sample_id: String,
    /// SHA-256 of the exact Ncdot report and configuration artifact.
    pub source_sha256: String,
    /// Event, model, target distribution, reference frame and time-scale identity.
    pub identity: NcdotTemporalIdentity,
    /// Fixed-time sphere integration report.
    pub report: NcdotSphereReport,
    /// Caller assertion that all upstream fixed-time prerequisites passed,
    /// including any future PeakOverlapPos / covariance-transport qualification.
    pub upstream_qualified: bool,
}

/// Declared temporal horizon and numerical sampling policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotTemporalHorizon {
    /// Inclusive start epoch in seconds.
    pub start_seconds: f64,
    /// Inclusive end epoch in seconds.
    pub end_seconds: f64,
    /// Maximum permitted spacing between adjacent supplied epochs.
    ///
    /// Larger gaps are returned as refinement-required and prevent qualification.
    pub maximum_interval_seconds: f64,
    /// Caller declaration of complete or known-incomplete horizon inventory.
    pub coverage_claim: TemporalHorizonCoverageClaim,
}

/// Type of sampled local Ncdot extremum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcdotTemporalExtremumKind {
    /// Strict or plateau local maximum.
    Maximum,
    /// Strict or plateau local minimum.
    Minimum,
}

/// A sampled local extremum, retaining plateau span where applicable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotTemporalExtremum {
    /// Maximum or minimum.
    pub kind: NcdotTemporalExtremumKind,
    /// First sample index in the equal-valued run.
    pub first_sample_index: usize,
    /// Last sample index in the equal-valued run.
    pub last_sample_index: usize,
    /// Epoch at the first sample in the run.
    pub first_epoch_seconds: f64,
    /// Epoch at the last sample in the run.
    pub last_epoch_seconds: f64,
    /// Log Ncdot value shared by all samples in the run.
    pub log_rate: f64,
    /// True when the extremum is a multi-sample plateau.
    pub plateau: bool,
}

/// Log-domain and provenance report for one temporal Ncdot aggregation.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotTemporalReport {
    /// Event/model identity used for the complete series.
    pub identity: NcdotTemporalIdentity,
    /// Declared horizon start.
    pub horizon_start_seconds: f64,
    /// Declared horizon end.
    pub horizon_end_seconds: f64,
    /// Caller completeness assertion.
    pub coverage_claim: TemporalHorizonCoverageClaim,
    /// Lebedev rule used by every epoch.
    pub rule: LebedevRule,
    /// Number of epoch samples.
    pub sample_count: usize,
    /// Number of trapezoidal time intervals.
    pub interval_count: usize,
    /// Maximum supplied interval width in seconds.
    pub maximum_interval_seconds: f64,
    /// Indices of intervals exceeding the configured maximum spacing.
    pub intervals_requiring_refinement: Vec<usize>,
    /// Trapezoidal expected collision count in log domain.
    ///
    /// Negative infinity denotes exact zero; finite values survive f64 underflow.
    pub log_expected_collision_count: f64,
    /// Trapezoidal expected collision count in f64.
    pub expected_collision_count: f64,
    /// True when a positive expected collision count underflowed in f64.
    pub collision_count_underflowed: bool,
    /// True when expected collision count exceeds f64 range.
    pub collision_count_overflowed: bool,
    /// Rate-weighted average significant Lebedev-node count, undefined when Nc=0.
    pub rate_weighted_average_significant_nodes: Option<f64>,
    /// Cumulative log expected collision count at each supplied epoch.
    pub cumulative_log_expected_collision_count: Vec<f64>,
    /// Sampled local maxima/minima, including plateaus and endpoint extrema.
    pub extrema: Vec<NcdotTemporalExtremum>,
    /// True only if every fixed-time source asserts its upstream prerequisites passed.
    pub all_upstream_qualified: bool,
    /// True only when caller coverage, temporal spacing and upstream evidence pass.
    ///
    /// This is still conditional on caller-provided full-horizon coverage and
    /// is not a proof of continuous-time peak resolution.
    pub qualified: bool,
}

/// Integrate an ordered series of fixed-time Ncdot reports by trapezoidal quadrature.
///
/// The output reproduces NASA's time-integration convention and rate-weighted
/// AvgLebNumb calculation. The significant-node count is the unweighted
/// per-epoch threshold diagnostic from the sphere integrator; only its temporal
/// average is weighted by Ncdot contribution.
///
/// The function rejects mixed event/model/target/time identities, duplicate
/// sample IDs, non-increasing epochs, inconsistent Lebedev rules, malformed
/// evidence digests and reports containing invalid positive-rate log values.
pub fn integrate_ncdot_time_series(
    identity: &NcdotTemporalIdentity,
    horizon: NcdotTemporalHorizon,
    samples: &[NcdotTemporalSample],
) -> Result<NcdotTemporalReport, KernelError> {
    validate_identity(identity)?;
    if !horizon.start_seconds.is_finite()
        || !horizon.end_seconds.is_finite()
        || !horizon.maximum_interval_seconds.is_finite()
        || horizon.start_seconds >= horizon.end_seconds
        || horizon.maximum_interval_seconds <= 0.0
    {
        return Err(KernelError::InvalidTemporalHorizon);
    }
    if samples.len() < 2 {
        return Err(KernelError::InsufficientTemporalSamples);
    }

    let rule = samples[0].report.rule;
    let mut seen_ids = std::collections::HashSet::with_capacity(samples.len());
    let mut log_rates = Vec::with_capacity(samples.len());
    let mut significant_nodes = Vec::with_capacity(samples.len());
    let mut upstream_qualified = true;

    for (index, sample) in samples.iter().enumerate() {
        if !sample.epoch_seconds.is_finite() {
            return Err(KernelError::NonFiniteValue);
        }
        validate_identifier(&sample.sample_id)?;
        validate_digest(&sample.source_sha256)?;
        if !seen_ids.insert(sample.sample_id.as_str()) {
            return Err(KernelError::DuplicateTemporalSample);
        }
        validate_identity(&sample.identity)?;
        if sample.identity != *identity {
            return Err(KernelError::EvidenceIdentityMismatch);
        }
        if sample.report.rule != rule {
            return Err(KernelError::TemporalLebedevRuleMismatch);
        }
        if sample.report.log_collision_rate.is_nan()
            || sample.report.log_collision_rate == f64::INFINITY
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if sample.report.significant_node_count > sample.report.node_count
            || sample.report.node_count == 0
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if !sample.report.significant_weight_fraction.is_finite()
            || !(0.0..=1.0).contains(&sample.report.significant_weight_fraction)
            || !sample.report.significant_integral_fraction.is_finite()
            || !(0.0..=1.0).contains(&sample.report.significant_integral_fraction)
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if index == 0 && sample.epoch_seconds != horizon.start_seconds {
            return Err(KernelError::TemporalHorizonEndpointMismatch);
        }
        if index > 0 && sample.epoch_seconds <= samples[index - 1].epoch_seconds {
            return Err(KernelError::NonIncreasingTemporalEpochs);
        }
        log_rates.push(sample.report.log_collision_rate);
        significant_nodes.push(sample.report.significant_node_count as f64);
        upstream_qualified &= sample.upstream_qualified && sample.report.adequacy_satisfied;
    }

    let last_epoch = samples.last().ok_or(KernelError::InsufficientTemporalSamples)?.epoch_seconds;
    if last_epoch != horizon.end_seconds {
        return Err(KernelError::TemporalHorizonEndpointMismatch);
    }

    let mut interval_log_contributions = Vec::with_capacity(samples.len() - 1);
    let mut weighted_node_log_contributions = Vec::with_capacity(samples.len() - 1);
    let mut intervals_requiring_refinement = Vec::new();
    let mut maximum_interval_seconds: f64 = 0.0;

    for index in 0..(samples.len() - 1) {
        let lower = samples[index].epoch_seconds;
        let upper = samples[index + 1].epoch_seconds;
        let delta = upper - lower;
        maximum_interval_seconds = maximum_interval_seconds.max(delta);
        if delta > horizon.maximum_interval_seconds {
            intervals_requiring_refinement.push(index);
        }

        let log_rate_lower = log_rates[index];
        let log_rate_upper = log_rates[index + 1];
        let log_trapezoid_scale = (0.5 * delta).ln();
        let interval_log = log_trapezoid_scale
            + log_add_exp(log_rate_lower, log_rate_upper);
        interval_log_contributions.push(interval_log);

        let lower_weighted = add_log_count(log_rate_lower, significant_nodes[index]);
        let upper_weighted = add_log_count(log_rate_upper, significant_nodes[index + 1]);
        weighted_node_log_contributions.push(
            log_trapezoid_scale + log_add_exp(lower_weighted, upper_weighted)
        );
    }

    let log_expected_collision_count = log_sum_exp(&interval_log_contributions)?;
    let log_weighted_significant_nodes = log_sum_exp(&weighted_node_log_contributions)?;
    let rate_weighted_average_significant_nodes = if log_expected_collision_count == f64::NEG_INFINITY {
        None
    } else {
        let value = (log_weighted_significant_nodes - log_expected_collision_count).exp();
        if !value.is_finite() || value < 0.0 {
            return Err(KernelError::NumericalOverflow);
        }
        Some(value)
    };

    let mut cumulative_log_expected_collision_count = Vec::with_capacity(samples.len());
    cumulative_log_expected_collision_count.push(f64::NEG_INFINITY);
    let mut cumulative_parts = Vec::with_capacity(interval_log_contributions.len());
    for interval in &interval_log_contributions {
        cumulative_parts.push(*interval);
        cumulative_log_expected_collision_count.push(log_sum_exp(&cumulative_parts)?);
    }

    let extrema = find_sampled_extrema(samples, &log_rates);
    let (expected_collision_count, collision_count_underflowed, collision_count_overflowed) =
        materialize_log_value(log_expected_collision_count)?;

    let horizon_complete =
        horizon.coverage_claim == TemporalHorizonCoverageClaim::CallerDeclaredComplete;
    let qualified = horizon_complete
        && intervals_requiring_refinement.is_empty()
        && upstream_qualified;

    Ok(NcdotTemporalReport {
        identity: identity.clone(),
        horizon_start_seconds: horizon.start_seconds,
        horizon_end_seconds: horizon.end_seconds,
        coverage_claim: horizon.coverage_claim,
        rule,
        sample_count: samples.len(),
        interval_count: interval_log_contributions.len(),
        maximum_interval_seconds,
        intervals_requiring_refinement,
        log_expected_collision_count,
        expected_collision_count,
        collision_count_underflowed,
        collision_count_overflowed,
        rate_weighted_average_significant_nodes,
        cumulative_log_expected_collision_count,
        extrema,
        all_upstream_qualified: upstream_qualified,
        qualified,
    })
}

fn find_sampled_extrema(
    samples: &[NcdotTemporalSample],
    log_rates: &[f64],
) -> Vec<NcdotTemporalExtremum> {
    let mut extrema = Vec::new();
    let mut start = 0;
    while start < log_rates.len() {
        let mut end = start;
        while end + 1 < log_rates.len() && log_rates[end + 1] == log_rates[start] {
            end += 1;
        }

        let value = log_rates[start];
        if value != f64::NEG_INFINITY {
            let left = (start > 0).then_some(log_rates[start - 1]);
            let right = (end + 1 < log_rates.len()).then_some(log_rates[end + 1]);
            let greater_left = left.is_some_and(|other| value > other);
            let greater_right = right.is_some_and(|other| value > other);
            let lower_left = left.is_some_and(|other| value < other);
            let lower_right = right.is_some_and(|other| value < other);

            let is_maximum = match (left, right) {
                (Some(_), Some(_)) => greater_left && greater_right,
                (Some(_), None) => greater_left,
                (None, Some(_)) => greater_right,
                (None, None) => false,
            };
            let is_minimum = match (left, right) {
                (Some(_), Some(_)) => lower_left && lower_right,
                (Some(_), None) => lower_left,
                (None, Some(_)) => lower_right,
                (None, None) => false,
            };

            if is_maximum {
                extrema.push(extremum(samples, start, end, value, NcdotTemporalExtremumKind::Maximum));
            }
            if is_minimum {
                extrema.push(extremum(samples, start, end, value, NcdotTemporalExtremumKind::Minimum));
            }
        }
        start = end + 1;
    }
    extrema
}

fn extremum(
    samples: &[NcdotTemporalSample],
    first: usize,
    last: usize,
    log_rate: f64,
    kind: NcdotTemporalExtremumKind,
) -> NcdotTemporalExtremum {
    NcdotTemporalExtremum {
        kind,
        first_sample_index: first,
        last_sample_index: last,
        first_epoch_seconds: samples[first].epoch_seconds,
        last_epoch_seconds: samples[last].epoch_seconds,
        log_rate,
        plateau: first != last,
    }
}

fn add_log_count(log_rate: f64, count: f64) -> f64 {
    if log_rate == f64::NEG_INFINITY || count == 0.0 {
        f64::NEG_INFINITY
    } else {
        log_rate + count.ln()
    }
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_sum_exp(values: &[f64]) -> Result<f64, KernelError> {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if maximum == f64::NEG_INFINITY {
        return Ok(f64::NEG_INFINITY);
    }
    if !maximum.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    let sum = values
        .iter()
        .filter(|value| **value != f64::NEG_INFINITY)
        .map(|value| (*value - maximum).exp())
        .sum::<f64>();
    if !sum.is_finite() || sum <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    let result = maximum + sum.ln();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(KernelError::NumericalOverflow)
    }
}

fn materialize_log_value(log_value: f64) -> Result<(f64, bool, bool), KernelError> {
    if log_value == f64::NEG_INFINITY {
        return Ok((0.0, false, false));
    }
    if !log_value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if log_value > f64::MAX.ln() {
        return Ok((f64::INFINITY, false, true));
    }
    let value = log_value.exp();
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((value, value == 0.0, false))
}

fn validate_identity(identity: &NcdotTemporalIdentity) -> Result<(), KernelError> {
    validate_identifier(&identity.event_id)?;
    validate_identifier(&identity.event_revision)?;
    validate_digest(&identity.event_definition_sha256)?;
    validate_identifier(&identity.model_revision)?;
    validate_digest(&identity.model_sha256)?;
    validate_digest(&identity.target_distribution_sha256)?;
    validate_identifier(&identity.reference_frame)?;
    validate_identifier(&identity.time_scale)
}

fn validate_identifier(value: &str) -> Result<(), KernelError> {
    if value.trim().is_empty() {
        Err(KernelError::InvalidEvidenceIdentifier)
    } else {
        Ok(())
    }
}

fn validate_digest(value: &str) -> Result<(), KernelError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Err(KernelError::InvalidEvidenceDigest)
    } else {
        Ok(())
    }
}

fn log_fraction(log_numerator: f64, log_denominator: f64) -> f64 {
    if log_numerator == f64::NEG_INFINITY || log_denominator == f64::NEG_INFINITY {
        0.0
    } else {
        (log_numerator - log_denominator).exp().clamp(0.0, 1.0)
    }
}
