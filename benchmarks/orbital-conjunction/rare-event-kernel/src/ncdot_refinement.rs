// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic, replayable controller for NASA-style sampled Ncdot refinement.
//!
//! The controller consumes a frozen vector of sampled epochs, Ncdot values and
//! effective Mahalanobis distances. It returns the next epochs to evaluate or
//! an explicit stop disposition. It reproduces the source-level refinement
//! branch ordering documented in NASA CARA's pinned Pc3D_Hall.m implementation:
//! boundary extension, top-four sampled-peak bisection, zero-rate MD2 fallback,
//! trapezoidal-contribution bisection and significant-rate-region spacing.
//!
//! This module does not propagate an orbit, evaluate Ncdot, prove continuous
//! peak completeness, or bound the integral between samples. It is a sampled
//! compatibility controller, not the certified space-time enclosure research
//! tracked separately. In particular, no sampled local maximum is evidence
//! that a narrow between-sample maximum does not exist.

use crate::KernelError;

/// NASA-compatible sampled temporal-refinement policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NcdotRefinementPolicy {
    /// Initial number of ephemeris points used by the source method.
    pub initial_epoch_count: usize,
    /// Maximum source-compatible refinement count (NASA default: 250).
    pub maximum_refinements: usize,
    /// Peak-to-neighbour relative rate drop that triggers bisection (NASA: 0.2).
    pub peak_relative_drop_tolerance: f64,
    /// Fraction of total trapezoidal contribution triggering node bisection (NASA: 0.02).
    pub integration_node_share_tolerance: f64,
    /// Input gamma parameter; the source uses max(1e-6, gamma).
    pub gamma: f64,
    /// Whether the source's isolated-conjunction accelerated termination is enabled.
    pub accelerate_refinement: bool,
    /// Absolute MD2 convergence tolerance for the zero-rate fallback (NASA: 1e-2).
    pub zero_rate_absolute_md2_tolerance: f64,
    /// Relative MD2 convergence tolerance for the zero-rate fallback (NASA: 1e-2).
    pub zero_rate_relative_md2_tolerance: f64,
    /// Opposite-side versus near-side gap ratio for zero-rate refinement (NASA: 3).
    pub zero_rate_opposite_gap_ratio: f64,
    /// Maximum near-side gap as a fraction of the current sampled span (NASA: 0.1).
    pub zero_rate_max_near_gap_fraction: f64,
}

impl Default for NcdotRefinementPolicy {
    fn default() -> Self {
        Self {
            initial_epoch_count: 101,
            maximum_refinements: 250,
            peak_relative_drop_tolerance: 0.2,
            integration_node_share_tolerance: 0.02,
            gamma: 1.0e-6,
            accelerate_refinement: true,
            zero_rate_absolute_md2_tolerance: 1.0e-2,
            zero_rate_relative_md2_tolerance: 1.0e-2,
            zero_rate_opposite_gap_ratio: 3.0,
            zero_rate_max_near_gap_fraction: 0.1,
        }
    }
}

/// Frozen input to one deterministic refinement decision.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotRefinementSnapshot {
    /// Strictly increasing sample epochs in seconds relative to the declared TCA.
    pub epochs_seconds: Vec<f64>,
    /// Nonnegative finite sampled Ncdot values at the corresponding epochs.
    pub collision_rates: Vec<f64>,
    /// Nonnegative finite effective MD2 values at the corresponding epochs.
    pub effective_md2: Vec<f64>,
    /// Lower allowed epoch, in seconds relative to TCA.
    pub lower_time_limit_seconds: f64,
    /// Upper allowed epoch, in seconds relative to TCA.
    pub upper_time_limit_seconds: f64,
    /// True when the sampled interval starts at the source's Coppola limits.
    pub starts_at_coppola_limits: bool,
    /// Whether PeakOverlapPos converged at each supplied epoch.
    pub peak_overlap_converged: Vec<bool>,
}

/// Why the controller selected an action or stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcdotRefinementReason {
    /// Extend the sampled interval toward the lower time limit.
    ExtendLowerBoundary,
    /// Extend the sampled interval toward the upper time limit.
    ExtendUpperBoundary,
    /// Bisect one or more intervals adjacent to the four highest sampled peaks.
    RefineSampledPeaks,
    /// Refine near the minimum effective MD2 because all sampled rates are zero.
    RefineZeroRateMd2Minimum,
    /// Bisect trapezoidal nodes whose contribution exceeds the configured share.
    RefineDominantIntegrationContributions,
    /// Enforce NASA's maximum spacing over the significant-rate region.
    RefineSignificantRateSpacing,
    /// The source's accelerated isolated-conjunction termination conditions hold.
    AcceleratedIsolatedConjunction,
    /// No sampled refinement branch requests another epoch.
    NoAdditionalSampledRefinement,
    /// No additional representable f64 epoch can be inserted where refinement is required.
    FloatingPointResolutionLimit,
    /// The source refinement counter exceeded its declared maximum.
    RefinementBudgetExceeded,
}

/// Disposition of one sampled-refinement decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NcdotRefinementDisposition {
    /// One or more new epochs must be evaluated before another decision.
    InsertEpochs,
    /// The sampled NASA-compatible controller has no further refinement request.
    Stop,
    /// The source refinement limit was exceeded; the assessment is non-converged.
    BudgetExceeded,
    /// A required midpoint/extension cannot be represented as a distinct f64 epoch.
    ResolutionLimit,
}

/// Replayable result of one controller transition.
#[derive(Debug, Clone, PartialEq)]
pub struct NcdotRefinementDecision {
    /// Zero-based refinement iteration supplied by the caller.
    pub iteration: usize,
    /// Action disposition.
    pub disposition: NcdotRefinementDisposition,
    /// Branch that selected the action or stop.
    pub reason: NcdotRefinementReason,
    /// Strictly increasing new epochs, with existing epochs and duplicates removed.
    pub inserted_epochs_seconds: Vec<f64>,
    /// Resolved NASA Ncdot rate cutoff for this decision.
    pub rate_cutoff: f64,
    /// Resolved significant-region sample count used to derive maximum spacing.
    pub significant_region_sample_count: usize,
}

/// Reproduce one deterministic NASA-style sampled Ncdot refinement transition.
///
/// The caller must evaluate all returned epochs, merge their fixed-time reports
/// into a new sorted snapshot, and call this function again with iteration + 1.
/// The exact snapshot, policy and iteration are sufficient to replay the
/// decision. The controller does not retain hidden mutable state.
pub fn decide_ncdot_refinement(
    snapshot: &NcdotRefinementSnapshot,
    policy: NcdotRefinementPolicy,
    iteration: usize,
) -> Result<NcdotRefinementDecision, KernelError> {
    validate_policy(policy)?;
    validate_snapshot(snapshot)?;

    let rate_cutoff = nasa_rate_cutoff(policy.gamma)?;
    let significant_region_sample_count = policy
        .initial_epoch_count
        .saturating_sub(1)
        .div_ceil(2)
        .max(51);

    if iteration > policy.maximum_refinements {
        return Ok(decision(
            iteration,
            NcdotRefinementDisposition::BudgetExceeded,
            NcdotRefinementReason::RefinementBudgetExceeded,
            Vec::new(),
            rate_cutoff,
            significant_region_sample_count,
        ));
    }

    let epochs = &snapshot.epochs_seconds;
    let rates = &snapshot.collision_rates;
    let md2 = &snapshot.effective_md2;
    let count = epochs.len();
    let maximum_rate = rates.iter().copied().fold(0.0_f64, f64::max);
    let maximum_index = rates
        .iter()
        .position(|rate| *rate == maximum_rate)
        .ok_or(KernelError::InvalidNcdotRefinementSnapshot)?;
    let expansion_step = initial_expansion_step(snapshot, policy)?;

    if policy.accelerate_refinement
        && iteration == 0
        && snapshot.starts_at_coppola_limits
        && maximum_rate > 0.0
        && rates[0] < maximum_rate * rate_cutoff
        && rates[count - 1] < maximum_rate * rate_cutoff
        && snapshot.peak_overlap_converged.iter().all(|value| *value)
    {
        return Ok(decision(
            iteration,
            NcdotRefinementDisposition::Stop,
            NcdotRefinementReason::AcceleratedIsolatedConjunction,
            Vec::new(),
            rate_cutoff,
            significant_region_sample_count,
        ));
    }

    if maximum_rate == 0.0 {
        return zero_rate_refinement(
            snapshot,
            policy,
            iteration,
            rate_cutoff,
            significant_region_sample_count,
        );
    }

    // NASA first extends the ephemeris if the sampled maximum or a boundary
    // rate above the Gaussian-derived cutoff indicates an unbounded rate span.
    if maximum_index == 0 && epochs[0] > snapshot.lower_time_limit_seconds {
        let target = (epochs[0] - expansion_step).max(snapshot.lower_time_limit_seconds);
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::ExtendLowerBoundary,
            vec![target],
            rate_cutoff,
            significant_region_sample_count,
        );
    }
    if maximum_index == count - 1 && epochs[count - 1] < snapshot.upper_time_limit_seconds {
        let target = (epochs[count - 1] + expansion_step).min(snapshot.upper_time_limit_seconds);
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::ExtendUpperBoundary,
            vec![target],
            rate_cutoff,
            significant_region_sample_count,
        );
    }
    if rates[0] > maximum_rate * rate_cutoff
        && epochs[0] > snapshot.lower_time_limit_seconds
    {
        let target = (epochs[0] - expansion_step).max(snapshot.lower_time_limit_seconds);
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::ExtendLowerBoundary,
            vec![target],
            rate_cutoff,
            significant_region_sample_count,
        );
    }
    if rates[count - 1] > maximum_rate * rate_cutoff
        && epochs[count - 1] < snapshot.upper_time_limit_seconds
    {
        let target = (epochs[count - 1] + expansion_step).min(snapshot.upper_time_limit_seconds);
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::ExtendUpperBoundary,
            vec![target],
            rate_cutoff,
            significant_region_sample_count,
        );
    }

    // The pinned CARA implementation refines only the four highest strict
    // sampled local maxima in this branch; equal-height plateaus are not
    // strict maxima in that source implementation.
    let peaks = top_four_sampled_peaks(rates);
    let mut peak_insertions = Vec::new();
    for peak in peaks {
        let left_gap = epochs[peak] - epochs[peak - 1];
        let right_gap = epochs[peak + 1] - epochs[peak];
        let peak_rate = rates[peak];
        let tolerance = policy.peak_relative_drop_tolerance * peak_rate;

        let left_drop = peak_rate - rates[peak - 1];
        if left_drop > tolerance || left_gap >= 2.0 * right_gap {
            peak_insertions.push(midpoint(epochs[peak - 1], epochs[peak])?);
        }

        let right_drop = peak_rate - rates[peak + 1];
        if right_drop > tolerance || right_gap >= 2.0 * left_gap {
            peak_insertions.push(midpoint(epochs[peak], epochs[peak + 1])?);
        }
    }
    if !peak_insertions.is_empty() {
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::RefineSampledPeaks,
            peak_insertions,
            rate_cutoff,
            significant_region_sample_count,
        );
    }

    // Match NASA's node contribution weights for the composite trapezoidal
    // sum: half of the adjacent interval at an endpoint and half the sum of
    // neighbouring intervals at an interior epoch.
    let node_time_weights = trapezoidal_node_weights(epochs)?;
    let contributions = node_time_weights
        .iter()
        .zip(rates)
        .map(|(time_weight, rate)| time_weight * rate)
        .collect::<Vec<_>>();
    let total_contribution = contributions.iter().sum::<f64>();
    if !total_contribution.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }

    let mut integration_insertions = Vec::new();
    if total_contribution > 0.0 {
        for (index, contribution) in contributions.iter().enumerate() {
            if *contribution <= policy.integration_node_share_tolerance * total_contribution {
                continue;
            }
            if index == 0 {
                if epochs[0] > snapshot.lower_time_limit_seconds {
                    integration_insertions.push(
                        (epochs[0] - expansion_step).max(snapshot.lower_time_limit_seconds),
                    );
                }
                integration_insertions.push(midpoint(epochs[0], epochs[1])?);
            } else if index == count - 1 {
                if epochs[count - 1] < snapshot.upper_time_limit_seconds {
                    integration_insertions.push(
                        (epochs[count - 1] + expansion_step).min(snapshot.upper_time_limit_seconds),
                    );
                }
                integration_insertions.push(midpoint(epochs[count - 2], epochs[count - 1])?);
            } else {
                integration_insertions.push(midpoint(epochs[index - 1], epochs[index])?);
                integration_insertions.push(midpoint(epochs[index], epochs[index + 1])?);
            }
        }
    }
    if !integration_insertions.is_empty() {
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::RefineDominantIntegrationContributions,
            integration_insertions,
            rate_cutoff,
            significant_region_sample_count,
        );
    }

    let mut significant_indices = rates
        .iter()
        .enumerate()
        .filter_map(|(index, rate)| (*rate > maximum_rate * rate_cutoff).then_some(index))
        .collect::<Vec<_>>();
    if significant_indices.is_empty() {
        return Ok(decision(
            iteration,
            NcdotRefinementDisposition::Stop,
            NcdotRefinementReason::NoAdditionalSampledRefinement,
            Vec::new(),
            rate_cutoff,
            significant_region_sample_count,
        ));
    }
    if significant_indices[0] > 0 {
        let preceding_index = significant_indices[0] - 1;
        significant_indices.insert(0, preceding_index);
    }
    if let Some(last) = significant_indices.last().copied()
        && last < count - 1
    {
        significant_indices.push(last + 1);
    }

    let first_epoch = epochs[*significant_indices.first().expect("nonempty significant indices")];
    let last_epoch = epochs[*significant_indices.last().expect("nonempty significant indices")];
    let maximum_spacing =
        (last_epoch - first_epoch) / significant_region_sample_count as f64;
    if !maximum_spacing.is_finite() || maximum_spacing <= 0.0 {
        return Err(KernelError::InvalidNcdotRefinementSnapshot);
    }

    let mut spacing_insertions = Vec::new();
    for index in significant_indices {
        if index > 0 && epochs[index] - epochs[index - 1] > maximum_spacing {
            spacing_insertions.push(midpoint(epochs[index - 1], epochs[index])?);
        }
        if index + 1 < count && epochs[index + 1] - epochs[index] > maximum_spacing {
            spacing_insertions.push(midpoint(epochs[index], epochs[index + 1])?);
        }
    }
    if !spacing_insertions.is_empty() {
        return make_decision(
            snapshot,
            iteration,
            NcdotRefinementReason::RefineSignificantRateSpacing,
            spacing_insertions,
            rate_cutoff,
            significant_region_sample_count,
        );
    }

    Ok(decision(
        iteration,
        NcdotRefinementDisposition::Stop,
        NcdotRefinementReason::NoAdditionalSampledRefinement,
        Vec::new(),
        rate_cutoff,
        significant_region_sample_count,
    ))
}

fn zero_rate_refinement(
    snapshot: &NcdotRefinementSnapshot,
    policy: NcdotRefinementPolicy,
    iteration: usize,
    rate_cutoff: f64,
    significant_region_sample_count: usize,
) -> Result<NcdotRefinementDecision, KernelError> {
    let epochs = &snapshot.epochs_seconds;
    let md2 = &snapshot.effective_md2;
    let minimum_md2 = md2.iter().copied().fold(f64::INFINITY, f64::min);
    let minimum_index = md2
        .iter()
        .position(|value| *value == minimum_md2)
        .ok_or(KernelError::InvalidNcdotRefinementSnapshot)?;
    let middle = minimum_index.clamp(1, epochs.len() - 2);
    let lower = middle - 1;
    let upper = middle + 1;

    let (near, far) = if md2[lower] > md2[upper] {
        (lower, upper)
    } else {
        (upper, lower)
    };
    let near_gap = (epochs[middle] - epochs[near]).abs();
    let far_gap = (epochs[middle] - epochs[far]).abs();

    let target = if far_gap > policy.zero_rate_opposite_gap_ratio * near_gap {
        midpoint(epochs[middle], epochs[far])?
    } else {
        let difference = (md2[near] - md2[middle]).abs();
        let minimum_converged = difference
            < policy.zero_rate_absolute_md2_tolerance
            || difference < policy.zero_rate_relative_md2_tolerance * minimum_md2;
        let gap_converged = near_gap
            < policy.zero_rate_max_near_gap_fraction * (epochs[epochs.len() - 1] - epochs[0]);
        if minimum_converged && gap_converged {
            return Ok(decision(
                iteration,
                NcdotRefinementDisposition::Stop,
                NcdotRefinementReason::NoAdditionalSampledRefinement,
                Vec::new(),
                rate_cutoff,
                significant_region_sample_count,
            ));
        }
        midpoint(epochs[middle], epochs[near])?
    };

    make_decision(
        snapshot,
        iteration,
        NcdotRefinementReason::RefineZeroRateMd2Minimum,
        vec![target],
        rate_cutoff,
        significant_region_sample_count,
    )
}

fn top_four_sampled_peaks(rates: &[f64]) -> Vec<usize> {
    let mut peaks = (1..rates.len() - 1)
        .filter(|index| {
            rates[*index] > 0.0
                && rates[*index] > rates[*index - 1]
                && rates[*index] > rates[*index + 1]
        })
        .collect::<Vec<_>>();
    peaks.sort_by(|left, right| {
        rates[*right]
            .total_cmp(&rates[*left])
            .then_with(|| left.cmp(right))
    });
    peaks.truncate(4);
    peaks
}

fn trapezoidal_node_weights(epochs: &[f64]) -> Result<Vec<f64>, KernelError> {
    let intervals = epochs
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    let mut weights = Vec::with_capacity(epochs.len());
    weights.push(0.5 * intervals[0]);
    for pair in intervals.windows(2) {
        weights.push(0.5 * (pair[0] + pair[1]));
    }
    weights.push(0.5 * intervals[intervals.len() - 1]);
    if weights.iter().any(|value| !value.is_finite() || *value <= 0.0) {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(weights)
}

fn initial_expansion_step(
    snapshot: &NcdotRefinementSnapshot,
    policy: NcdotRefinementPolicy,
) -> Result<f64, KernelError> {
    let sampled_half_width =
        0.5 * (snapshot.epochs_seconds[snapshot.epochs_seconds.len() - 1]
            - snapshot.epochs_seconds[0]);
    let refinement_divisor = (policy.maximum_refinements as f64 / 3.0).max(1.0);
    let lower_scale = snapshot.lower_time_limit_seconds.abs() / refinement_divisor;
    let upper_scale = snapshot.upper_time_limit_seconds.abs() / refinement_divisor;
    let step = sampled_half_width.max(lower_scale).max(upper_scale);
    if !step.is_finite() || step <= 0.0 {
        return Err(KernelError::InvalidNcdotRefinementSnapshot);
    }
    Ok(step)
}

fn nasa_rate_cutoff(policy_gamma: f64) -> Result<f64, KernelError> {
    let gamma = policy_gamma.max(1.0e-6);
    if !gamma.is_finite() || !(0.0..=1.0).contains(&gamma) || gamma == 0.0 {
        return Err(KernelError::InvalidNcdotRefinementPolicy);
    }
    let inverse = inverse_erfc_nonnegative(gamma)?;
    let x = core::f64::consts::SQRT_2 * inverse;
    let cutoff = (-0.5 * x * x).exp() / (2.0 * core::f64::consts::PI).sqrt();
    if !cutoff.is_finite() || cutoff <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(cutoff)
}

fn inverse_erfc_nonnegative(value: f64) -> Result<f64, KernelError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) || value == 0.0 {
        return Err(KernelError::InvalidNcdotRefinementPolicy);
    }
    if value == 1.0 {
        return Ok(0.0);
    }
    let mut lower = 0.0;
    let mut upper = 32.0;
    for _ in 0..128 {
        let middle = lower + 0.5 * (upper - lower);
        if libm::erfc(middle) > value {
            lower = middle;
        } else {
            upper = middle;
        }
    }
    let result = lower + 0.5 * (upper - lower);
    if !result.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(result)
}

fn make_decision(
    snapshot: &NcdotRefinementSnapshot,
    iteration: usize,
    reason: NcdotRefinementReason,
    candidates: Vec<f64>,
    rate_cutoff: f64,
    significant_region_sample_count: usize,
) -> Result<NcdotRefinementDecision, KernelError> {
    let mut inserted = Vec::with_capacity(candidates.len());
    let mut resolution_limit = false;
    for candidate in candidates {
        if !candidate.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        if candidate < snapshot.lower_time_limit_seconds
            || candidate > snapshot.upper_time_limit_seconds
        {
            continue;
        }
        if snapshot.epochs_seconds.binary_search_by(|value| value.total_cmp(&candidate)).is_ok()
        {
            continue;
        }
        let first_epoch = snapshot.epochs_seconds[0];
        let last_epoch = snapshot.epochs_seconds[snapshot.epochs_seconds.len() - 1];
        let is_lower_extension =
            candidate >= snapshot.lower_time_limit_seconds && candidate < first_epoch;
        let is_upper_extension =
            candidate <= snapshot.upper_time_limit_seconds && candidate > last_epoch;
        let left = snapshot.epochs_seconds.partition_point(|value| *value < candidate);
        let is_between = left > 0
            && left < snapshot.epochs_seconds.len()
            && snapshot.epochs_seconds[left - 1] < candidate
            && candidate < snapshot.epochs_seconds[left];
        if is_between || is_lower_extension || is_upper_extension {
            inserted.push(candidate);
        } else {
            resolution_limit = true;
        }
    }
    inserted.sort_by(f64::total_cmp);
    inserted.dedup_by(|left, right| left.to_bits() == right.to_bits());

    if inserted.is_empty() {
        let disposition = if resolution_limit {
            NcdotRefinementDisposition::ResolutionLimit
        } else {
            NcdotRefinementDisposition::Stop
        };
        let final_reason = if resolution_limit {
            NcdotRefinementReason::FloatingPointResolutionLimit
        } else {
            NcdotRefinementReason::NoAdditionalSampledRefinement
        };
        return Ok(decision(
            iteration,
            disposition,
            final_reason,
            Vec::new(),
            rate_cutoff,
            significant_region_sample_count,
        ));
    }

    Ok(decision(
        iteration,
        NcdotRefinementDisposition::InsertEpochs,
        reason,
        inserted,
        rate_cutoff,
        significant_region_sample_count,
    ))
}

fn midpoint(lower: f64, upper: f64) -> Result<f64, KernelError> {
    let value = 0.5 * lower + 0.5 * upper;
    if !value.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    if value <= lower || value >= upper {
        return Err(KernelError::NcdotRefinementResolutionLimit);
    }
    Ok(value)
}

fn decision(
    iteration: usize,
    disposition: NcdotRefinementDisposition,
    reason: NcdotRefinementReason,
    inserted_epochs_seconds: Vec<f64>,
    rate_cutoff: f64,
    significant_region_sample_count: usize,
) -> NcdotRefinementDecision {
    NcdotRefinementDecision {
        iteration,
        disposition,
        reason,
        inserted_epochs_seconds,
        rate_cutoff,
        significant_region_sample_count,
    }
}

fn validate_policy(policy: NcdotRefinementPolicy) -> Result<(), KernelError> {
    if policy.initial_epoch_count < 3
        || policy.maximum_refinements == 0
        || !policy.peak_relative_drop_tolerance.is_finite()
        || !(0.0..=1.0).contains(&policy.peak_relative_drop_tolerance)
        || !policy.integration_node_share_tolerance.is_finite()
        || !(0.0..=1.0).contains(&policy.integration_node_share_tolerance)
        || !policy.gamma.is_finite()
        || !(0.0..=1.0).contains(&policy.gamma)
        || policy.gamma == 0.0
        || !policy.zero_rate_absolute_md2_tolerance.is_finite()
        || policy.zero_rate_absolute_md2_tolerance < 0.0
        || !policy.zero_rate_relative_md2_tolerance.is_finite()
        || policy.zero_rate_relative_md2_tolerance < 0.0
        || !policy.zero_rate_opposite_gap_ratio.is_finite()
        || policy.zero_rate_opposite_gap_ratio <= 1.0
        || !policy.zero_rate_max_near_gap_fraction.is_finite()
        || !(0.0..=1.0).contains(&policy.zero_rate_max_near_gap_fraction)
    {
        return Err(KernelError::InvalidNcdotRefinementPolicy);
    }
    Ok(())
}

fn validate_snapshot(snapshot: &NcdotRefinementSnapshot) -> Result<(), KernelError> {
    let count = snapshot.epochs_seconds.len();
    if count < 3
        || snapshot.collision_rates.len() != count
        || snapshot.effective_md2.len() != count
        || snapshot.peak_overlap_converged.len() != count
    {
        return Err(KernelError::InvalidNcdotRefinementSnapshot);
    }
    if !snapshot.lower_time_limit_seconds.is_finite()
        || !snapshot.upper_time_limit_seconds.is_finite()
        || snapshot.lower_time_limit_seconds >= snapshot.upper_time_limit_seconds
        || snapshot.epochs_seconds[0] < snapshot.lower_time_limit_seconds
        || snapshot.epochs_seconds[count - 1] > snapshot.upper_time_limit_seconds
    {
        return Err(KernelError::InvalidNcdotRefinementSnapshot);
    }
    if snapshot
        .epochs_seconds
        .iter()
        .any(|epoch| !epoch.is_finite())
        || snapshot.epochs_seconds.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(KernelError::NonIncreasingTemporalEpochs);
    }
    if snapshot
        .collision_rates
        .iter()
        .any(|rate| !rate.is_finite() || *rate < 0.0)
        || snapshot
            .effective_md2
            .iter()
            .any(|value| !value.is_finite() || *value < 0.0)
    {
        return Err(KernelError::InvalidTemporalRate);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(
        epochs: &[f64],
        rates: &[f64],
        md2: &[f64],
        lower_limit: f64,
        upper_limit: f64,
    ) -> NcdotRefinementSnapshot {
        NcdotRefinementSnapshot {
            epochs_seconds: epochs.to_vec(),
            collision_rates: rates.to_vec(),
            effective_md2: md2.to_vec(),
            lower_time_limit_seconds: lower_limit,
            upper_time_limit_seconds: upper_limit,
            starts_at_coppola_limits: false,
            peak_overlap_converged: vec![true; epochs.len()],
        }
    }

    fn policy() -> NcdotRefinementPolicy {
        NcdotRefinementPolicy::default()
    }

    #[test]
    fn default_policy_reproduces_pinned_nasa_thresholds() {
        let policy = policy();
        assert_eq!(policy.maximum_refinements, 250);
        assert_eq!(policy.peak_relative_drop_tolerance, 0.2);
        assert_eq!(policy.integration_node_share_tolerance, 0.02);
        assert_eq!(policy.gamma, 1.0e-6);
        assert_eq!(policy.initial_epoch_count, 101);
    }

    #[test]
    fn boundary_rate_maximum_extends_lower_horizon_before_peak_refinement() {
        let input = snapshot(&[-10.0, 0.0, 10.0], &[10.0, 4.0, 1.0], &[3.0, 2.0, 3.0], -40.0, 40.0);
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(result.reason, NcdotRefinementReason::ExtendLowerBoundary);
        assert_eq!(result.inserted_epochs_seconds, vec![-20.0]);
    }

    #[test]
    fn upper_boundary_rate_maximum_extends_upper_horizon_before_peak_refinement() {
        let input = snapshot(&[-10.0, 0.0, 10.0], &[1.0, 4.0, 10.0], &[3.0, 2.0, 3.0], -40.0, 40.0);
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(result.reason, NcdotRefinementReason::ExtendUpperBoundary);
        assert_eq!(result.inserted_epochs_seconds, vec![20.0]);
    }

    #[test]
    fn strict_sampled_peak_refines_both_sides_when_rate_drop_exceeds_twenty_percent() {
        let input = snapshot(
            &[-20.0, -10.0, 0.0, 10.0, 20.0],
            &[0.0, 4.0, 10.0, 4.0, 0.0],
            &[8.0, 4.0, 0.0, 4.0, 8.0],
            -20.0,
            20.0,
        );
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(
            result.reason,
            NcdotRefinementReason::RefineDominantIntegrationContributions
        );
        assert_eq!(result.inserted_epochs_seconds, vec![-5.0, 5.0]);
    }

    #[test]
    fn equal_rate_plateau_is_not_a_strict_local_maximum() {
        let input = snapshot(
            &[-20.0, -10.0, 0.0, 10.0, 20.0],
            &[0.0, 10.0, 10.0, 4.0, 0.0],
            &[8.0, 4.0, 0.0, 4.0, 8.0],
            -20.0,
            20.0,
        );
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_ne!(result.reason, NcdotRefinementReason::RefineSampledPeaks);
    }

    #[test]
    fn trapezoidal_dominant_node_refinement_bisects_adjacent_intervals() {
        let input = snapshot(
            &[-10.0, 0.0, 10.0],
            &[0.0, 50.0, 100.0],
            &[2.0, 1.0, 2.0],
            -10.0,
            10.0,
        );
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(result.reason, NcdotRefinementReason::RefineSampledPeaks);
        assert_eq!(result.inserted_epochs_seconds, vec![-5.0, 5.0]);
    }

    #[test]
    fn significant_rate_spacing_bisects_intervals_that_exceed_the_51_point_limit() {
        let mut epochs = (0..101)
            .map(|index| -10.0 + 0.2 * index as f64)
            .collect::<Vec<_>>();
        let mut rates = (0..101)
            .map(|index| 1.0 - (0.999 / 100.0) * index as f64)
            .collect::<Vec<_>>();
        let mut md2 = vec![1.0; 101];
        epochs.push(110.0);
        rates.push(0.001);
        md2.push(2.0);
        for index in 1..=50 {
            epochs.push(110.0 + (890.0 / 50.0) * index as f64);
            rates.push(0.001);
            md2.push(2.0);
        }
        let input = snapshot(&epochs, &rates, &md2, -10.0, 1000.0);
        let result = decide_ncdot_refinement(&input, policy(), 1).unwrap();
        assert_eq!(result.significant_region_sample_count, 51);
        assert_eq!(
            result.reason,
            NcdotRefinementReason::RefineSignificantRateSpacing
        );
        assert!(result.inserted_epochs_seconds.contains(&60.0));
    }

    #[test]
    fn zero_rate_fallback_refines_around_minimum_md2() {
        let input = snapshot(
            &[-10.0, 0.0, 10.0],
            &[0.0, 0.0, 0.0],
            &[6.0, 1.0, 5.0],
            -10.0,
            10.0,
        );
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(result.reason, NcdotRefinementReason::RefineZeroRateMd2Minimum);
        assert_eq!(result.inserted_epochs_seconds, vec![-5.0]);
    }

    #[test]
    fn zero_rate_fallback_stops_only_when_md2_and_near_gap_are_converged() {
        let input = snapshot(
            &[-10.0, -1.0, 0.0, 1.0, 10.0],
            &[0.0, 0.0, 0.0, 0.0, 0.0],
            &[4.0, 1.005, 1.0, 1.005, 4.0],
            -10.0,
            10.0,
        );
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(result.disposition, NcdotRefinementDisposition::Stop);
        assert_eq!(result.reason, NcdotRefinementReason::NoAdditionalSampledRefinement);
    }

    #[test]
    fn accelerated_termination_requires_all_source_preconditions() {
        let mut input = snapshot(
            &[-10.0, 0.0, 10.0],
            &[0.0, 10.0, 0.0],
            &[2.0, 0.0, 2.0],
            -10.0,
            10.0,
        );
        input.starts_at_coppola_limits = true;
        let result = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_eq!(
            result.reason,
            NcdotRefinementReason::AcceleratedIsolatedConjunction
        );
        input.peak_overlap_converged[1] = false;
        let unqualified = decide_ncdot_refinement(&input, policy(), 0).unwrap();
        assert_ne!(
            unqualified.reason,
            NcdotRefinementReason::AcceleratedIsolatedConjunction
        );
    }

    #[test]
    fn top_four_peak_selection_is_deterministic_and_excludes_the_fifth_peak() {
        let epochs = (0..13).map(|index| index as f64).collect::<Vec<_>>();
        let rates = vec![0.0, 10.0, 0.0, 9.0, 0.0, 8.0, 0.0, 7.0, 0.0, 6.0, 0.0, 0.0, 0.0];
        let md2 = vec![10.0; epochs.len()];
        let input = snapshot(&epochs, &rates, &md2, 0.0, 12.0);
        let first = decide_ncdot_refinement(&input, policy(), 1).unwrap();
        let second = decide_ncdot_refinement(&input, policy(), 1).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.reason, NcdotRefinementReason::RefineSampledPeaks);
        assert!(!first.inserted_epochs_seconds.contains(&9.5));
    }

    #[test]
    fn refinement_budget_is_reported_after_the_declared_maximum() {
        let input = snapshot(
            &[-10.0, 0.0, 10.0],
            &[0.0, 1.0, 0.0],
            &[2.0, 0.0, 2.0],
            -10.0,
            10.0,
        );
        let at_limit = decide_ncdot_refinement(&input, policy(), 250).unwrap();
        let beyond_limit = decide_ncdot_refinement(&input, policy(), 251).unwrap();
        assert_ne!(at_limit.disposition, NcdotRefinementDisposition::BudgetExceeded);
        assert_eq!(beyond_limit.disposition, NcdotRefinementDisposition::BudgetExceeded);
    }

    #[test]
    fn duplicate_and_nonrepresentable_insertions_do_not_create_new_samples() {
        let input = snapshot(
            &[1.0, 1.0 + f64::EPSILON, 1.0 + 2.0 * f64::EPSILON],
            &[0.0, 1.0, 0.0],
            &[2.0, 0.0, 2.0],
            1.0,
            1.0 + 2.0 * f64::EPSILON,
        );
        let result = decide_ncdot_refinement(&input, policy(), 1).unwrap();
        assert_eq!(
            result,
            Err(KernelError::NcdotRefinementResolutionLimit)
        );
    }

    #[test]
    fn invalid_policy_and_mismatched_snapshot_shapes_fail_closed() {
        let input = snapshot(&[-1.0, 0.0, 1.0], &[0.0, 1.0, 0.0], &[2.0, 0.0, 2.0], -1.0, 1.0);
        let invalid = NcdotRefinementPolicy {
            peak_relative_drop_tolerance: f64::NAN,
            ..policy()
        };
        assert_eq!(
            decide_ncdot_refinement(&input, invalid, 0),
            Err(KernelError::InvalidNcdotRefinementPolicy)
        );
        let mut invalid_snapshot = input;
        invalid_snapshot.effective_md2.pop();
        assert_eq!(
            decide_ncdot_refinement(&invalid_snapshot, policy(), 0),
            Err(KernelError::InvalidNcdotRefinementSnapshot)
        );
    }
}
