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
            || sample.report.log_sphere_integral.is_nan()
            || sample.report.log_sphere_integral == f64::INFINITY
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if sample.report.node_count != sample.report.rule.node_count()
            || sample.report.significant_node_count > sample.report.node_count
            || sample.report.node_count == 0
            || sample.report.node_underflow_count > sample.report.node_count
            || sample.report.node_overflow_count > sample.report.node_count
            || sample.report.zero_variance_fallback_count > sample.report.node_count
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if !sample.report.weight_sum.is_finite()
            || (sample.report.weight_sum - 4.0 * core::f64::consts::PI).abs() > 2.0e-12
            || !sample.report.relative_contribution_threshold.is_finite()
            || !(0.0..=1.0).contains(&sample.report.relative_contribution_threshold)
            || !sample.report.significant_weight_fraction.is_finite()
            || !(0.0..=1.0).contains(&sample.report.significant_weight_fraction)
            || !sample.report.significant_integral_fraction.is_finite()
            || !(0.0..=1.0).contains(&sample.report.significant_integral_fraction)
            || sample.report.exact_zero_integrand
                != (sample.report.log_collision_rate == f64::NEG_INFINITY)
        {
            return Err(KernelError::InvalidTemporalRate);
        }
        if sample.report.collision_rate.is_nan()
            || sample.report.collision_rate < 0.0
            || sample.report.collision_rate_underflowed
                != (sample.report.log_collision_rate.is_finite()
                    && sample.report.collision_rate == 0.0)
            || sample.report.collision_rate_overflowed != sample.report.collision_rate.is_infinite()
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

    let last_epoch = samples
        .last()
        .ok_or(KernelError::InsufficientTemporalSamples)?
        .epoch_seconds;
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
        let interval_log = log_trapezoid_scale + log_add_exp(log_rate_lower, log_rate_upper);
        interval_log_contributions.push(interval_log);

        let lower_weighted = add_log_count(log_rate_lower, significant_nodes[index]);
        let upper_weighted = add_log_count(log_rate_upper, significant_nodes[index + 1]);
        weighted_node_log_contributions
            .push(log_trapezoid_scale + log_add_exp(lower_weighted, upper_weighted));
    }

    let log_expected_collision_count = log_sum_exp(&interval_log_contributions)?;
    let log_weighted_significant_nodes = log_sum_exp(&weighted_node_log_contributions)?;
    let rate_weighted_average_significant_nodes =
        if log_expected_collision_count == f64::NEG_INFINITY {
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
    let qualified =
        horizon_complete && intervals_requiring_refinement.is_empty() && upstream_qualified;

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
                extrema.push(extremum(
                    samples,
                    start,
                    end,
                    value,
                    NcdotTemporalExtremumKind::Maximum,
                ));
            }
            if is_minimum {
                extrema.push(extremum(
                    samples,
                    start,
                    end,
                    value,
                    NcdotTemporalExtremumKind::Minimum,
                ));
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

#[cfg(test)]
mod tests {
    use super::*;

    const HASH_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const HASH_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
    const HASH_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

    fn identity() -> NcdotTemporalIdentity {
        NcdotTemporalIdentity {
            event_id: "event-42".to_owned(),
            event_revision: "event-r9".to_owned(),
            event_definition_sha256: HASH_A.to_owned(),
            model_revision: "force-model-r4".to_owned(),
            model_sha256: HASH_B.to_owned(),
            target_distribution_sha256: HASH_C.to_owned(),
            reference_frame: "GCRF".to_owned(),
            time_scale: "TAI".to_owned(),
        }
    }

    fn report(
        log_rate: f64,
        significant_node_count: usize,
        rule: LebedevRule,
    ) -> NcdotSphereReport {
        let materialized = log_rate.exp();
        NcdotSphereReport {
            rule,
            node_count: rule.node_count(),
            weight_sum: 4.0 * core::f64::consts::PI,
            log_sphere_integral: log_rate,
            sphere_integral: materialized,
            sphere_integral_underflowed: log_rate.is_finite() && materialized == 0.0,
            sphere_integral_overflowed: log_rate > f64::MAX.ln(),
            log_collision_rate: log_rate,
            collision_rate: materialized,
            collision_rate_underflowed: log_rate.is_finite() && materialized == 0.0,
            collision_rate_overflowed: log_rate > f64::MAX.ln(),
            maximum_log_integrand: 0.0,
            relative_contribution_threshold: 1.0e-3,
            significant_node_count,
            significant_weight_fraction: significant_node_count as f64 / rule.node_count() as f64,
            significant_integral_fraction: 1.0,
            maximum_node_integral_fraction: 1.0 / rule.node_count() as f64,
            node_underflow_count: 0,
            node_overflow_count: 0,
            zero_variance_fallback_count: 0,
            exact_zero_integrand: log_rate == f64::NEG_INFINITY,
            adequacy_satisfied: true,
        }
    }

    fn sample(
        index: usize,
        epoch_seconds: f64,
        log_rate: f64,
        significant_node_count: usize,
        rule: LebedevRule,
    ) -> NcdotTemporalSample {
        let source_sha256 = match index % 4 {
            0 => HASH_D,
            1 => "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            2 => "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
            _ => "1111111111111111111111111111111111111111111111111111111111111111",
        };
        NcdotTemporalSample {
            epoch_seconds,
            sample_id: format!("epoch-{index}"),
            source_sha256: source_sha256.to_owned(),
            identity: identity(),
            report: report(log_rate, significant_node_count, rule),
            upstream_qualified: true,
        }
    }

    fn horizon(end_seconds: f64, max_interval: f64) -> NcdotTemporalHorizon {
        NcdotTemporalHorizon {
            start_seconds: 0.0,
            end_seconds,
            maximum_interval_seconds: max_interval,
            coverage_claim: TemporalHorizonCoverageClaim::CallerDeclaredComplete,
        }
    }

    #[test]
    fn constant_rate_on_irregular_grid_matches_analytic_nc_and_weighted_support() {
        let samples = [
            sample(0, 0.0, 2.0_f64.ln(), 7, LebedevRule::Points26),
            sample(1, 2.0, 2.0_f64.ln(), 7, LebedevRule::Points26),
            sample(2, 5.0, 2.0_f64.ln(), 7, LebedevRule::Points26),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(5.0, 3.0), &samples)
            .expect("valid constant-rate series");
        assert!((result.expected_collision_count - 10.0).abs() < 1.0e-12);
        assert_eq!(result.rate_weighted_average_significant_nodes, Some(7.0));
        assert_eq!(result.cumulative_log_expected_collision_count.len(), 3);
        assert!(result.qualified);
        assert!(result.extrema.is_empty());
    }

    #[test]
    fn significant_node_average_is_rate_weighted_not_an_arithmetic_mean() {
        let samples = [
            sample(0, 0.0, 0.0, 1, LebedevRule::Points5810),
            sample(1, 1.0, 1.0e-6_f64.ln(), 100, LebedevRule::Points5810),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples)
            .expect("valid uneven-risk series");
        let expected = (0.5 * (1.0 + 100.0e-6)) / (0.5 * (1.0 + 1.0e-6));
        let average = result.rate_weighted_average_significant_nodes.unwrap();
        assert!((average - expected).abs() < 1.0e-12);
        assert!(average < 1.001);
        assert!(average < 50.5);
    }

    #[test]
    fn two_peaks_and_a_flat_minimum_plateau_are_preserved() {
        let rates = [1.0, 3.0, 2.0, 2.0, 4.0, 1.0];
        let samples = rates
            .iter()
            .enumerate()
            .map(|(i, rate)| sample(i, i as f64, rate.ln(), 10, LebedevRule::Points14))
            .collect::<Vec<_>>();
        let result = integrate_ncdot_time_series(&identity(), horizon(5.0, 1.0), &samples)
            .expect("multimodal series");
        let maxima = result
            .extrema
            .iter()
            .filter(|extremum| extremum.kind == NcdotTemporalExtremumKind::Maximum)
            .collect::<Vec<_>>();
        let minima = result
            .extrema
            .iter()
            .filter(|extremum| extremum.kind == NcdotTemporalExtremumKind::Minimum)
            .collect::<Vec<_>>();
        assert_eq!(maxima.len(), 2);
        assert_eq!(minima.len(), 3);
        assert!(minima.iter().any(|extremum| extremum.plateau
            && extremum.first_sample_index == 2
            && extremum.last_sample_index == 3));
        assert_eq!(maxima[0].first_sample_index, 1);
        assert_eq!(maxima[1].first_sample_index, 4);
    }

    #[test]
    fn endpoints_are_reported_as_sampled_extrema() {
        let rates = [4.0, 2.0, 1.0];
        let samples = rates
            .iter()
            .enumerate()
            .map(|(i, rate)| sample(i, i as f64, rate.ln(), 5, LebedevRule::Points6))
            .collect::<Vec<_>>();
        let result = integrate_ncdot_time_series(&identity(), horizon(2.0, 1.0), &samples)
            .expect("endpoint maximum");
        assert_eq!(result.extrema.len(), 2);
        assert_eq!(result.extrema[0].kind, NcdotTemporalExtremumKind::Maximum);
        assert_eq!(result.extrema[0].first_sample_index, 0);
        assert_eq!(result.extrema[1].kind, NcdotTemporalExtremumKind::Minimum);
        assert_eq!(result.extrema[1].first_sample_index, 2);
    }

    #[test]
    fn excessive_time_gap_requests_refinement_and_prevents_qualification() {
        let samples = [
            sample(0, 0.0, 0.0, 10, LebedevRule::Points26),
            sample(1, 5.0, 0.0, 10, LebedevRule::Points26),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(5.0, 1.0), &samples)
            .expect("sparse but structurally valid samples");
        assert_eq!(result.intervals_requiring_refinement, vec![0]);
        assert!(!result.qualified);
        assert_eq!(result.maximum_interval_seconds, 5.0);
    }

    #[test]
    fn known_incomplete_horizon_never_qualifies() {
        let samples = [
            sample(0, 0.0, 0.0, 10, LebedevRule::Points6),
            sample(1, 2.0, 0.0, 10, LebedevRule::Points6),
        ];
        let mut declared = horizon(2.0, 2.0);
        declared.coverage_claim = TemporalHorizonCoverageClaim::KnownIncomplete;
        let result = integrate_ncdot_time_series(&identity(), declared, &samples)
            .expect("partial aggregation can still be recorded");
        assert!(!result.qualified);
        assert_eq!(
            result.coverage_claim,
            TemporalHorizonCoverageClaim::KnownIncomplete
        );
    }

    #[test]
    fn zero_collision_rate_has_undefined_average_not_nan_or_fake_zero() {
        let samples = [
            sample(0, 0.0, f64::NEG_INFINITY, 0, LebedevRule::Points6),
            sample(1, 3.0, f64::NEG_INFINITY, 0, LebedevRule::Points6),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(3.0, 3.0), &samples)
            .expect("exact zero collision rate");
        assert_eq!(result.log_expected_collision_count, f64::NEG_INFINITY);
        assert_eq!(result.expected_collision_count, 0.0);
        assert_eq!(result.rate_weighted_average_significant_nodes, None);
        assert!(!result.collision_count_underflowed);
        assert!(result.qualified);
    }

    #[test]
    fn rare_positive_rates_retain_finite_log_nc_after_f64_underflow() {
        let samples = [
            sample(0, 0.0, -900.0, 3, LebedevRule::Points6),
            sample(1, 10.0, -900.0, 3, LebedevRule::Points6),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(10.0, 10.0), &samples)
            .expect("rare positive rate");
        assert!(
            (result.log_expected_collision_count - (-900.0_f64 + 10.0_f64.ln())).abs() < 1.0e-12
        );
        assert_eq!(result.expected_collision_count, 0.0);
        assert!(result.collision_count_underflowed);
        assert_eq!(result.rate_weighted_average_significant_nodes, Some(3.0));
    }

    #[test]
    fn overflowed_nc_retains_finite_log_count() {
        let samples = [
            sample(0, 0.0, 800.0, 3, LebedevRule::Points6),
            sample(1, 1.0, 800.0, 3, LebedevRule::Points6),
        ];
        let result = integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples)
            .expect("large positive rate");
        assert!(result.log_expected_collision_count.is_finite());
        assert!(result.expected_collision_count.is_infinite());
        assert!(result.collision_count_overflowed);
    }

    #[test]
    fn mixed_identity_digest_rule_and_invalid_rate_fail_closed() {
        let samples = [
            sample(0, 0.0, 0.0, 3, LebedevRule::Points6),
            sample(1, 1.0, 0.0, 3, LebedevRule::Points14),
        ];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples),
            Err(KernelError::TemporalLebedevRuleMismatch)
        );

        let mut mismatched = sample(1, 1.0, 0.0, 3, LebedevRule::Points6);
        mismatched.identity.model_revision = "other-model".to_owned();
        let samples = [sample(0, 0.0, 0.0, 3, LebedevRule::Points6), mismatched];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples),
            Err(KernelError::EvidenceIdentityMismatch)
        );

        let mut invalid_rate = sample(1, 1.0, 0.0, 3, LebedevRule::Points6);
        invalid_rate.report.log_collision_rate = f64::NAN;
        let samples = [sample(0, 0.0, 0.0, 3, LebedevRule::Points6), invalid_rate];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples),
            Err(KernelError::InvalidTemporalRate)
        );
    }

    #[test]
    fn duplicated_or_non_increasing_epochs_fail_closed() {
        let duplicated = [
            sample(0, 0.0, 0.0, 3, LebedevRule::Points6),
            sample(1, 0.0, 0.0, 3, LebedevRule::Points6),
        ];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &duplicated),
            Err(KernelError::NonIncreasingTemporalEpochs)
        );

        let mut repeated_id = sample(1, 1.0, 0.0, 3, LebedevRule::Points6);
        repeated_id.sample_id = "epoch-0".to_owned();
        let repeated = [sample(0, 0.0, 0.0, 3, LebedevRule::Points6), repeated_id];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &repeated),
            Err(KernelError::DuplicateTemporalSample)
        );
    }

    #[test]
    fn horizon_endpoints_and_sample_inventory_are_mandatory() {
        let one_sample = [sample(0, 0.0, 0.0, 3, LebedevRule::Points6)];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &one_sample),
            Err(KernelError::InsufficientTemporalSamples)
        );

        let samples = [
            sample(0, 0.5, 0.0, 3, LebedevRule::Points6),
            sample(1, 1.0, 0.0, 3, LebedevRule::Points6),
        ];
        assert_eq!(
            integrate_ncdot_time_series(&identity(), horizon(1.0, 1.0), &samples),
            Err(KernelError::TemporalHorizonEndpointMismatch)
        );
    }
}
