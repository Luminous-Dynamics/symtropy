// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Versioned, deterministic gait-event and inter-leg coordination metrics.
//!
//! This crate computes raw per-event/per-trial values. It is not a scorecard,
//! statistical aggregator, evidence registry or controller. All timestamps are
//! integer ticks; callers must bind the tick timebase in their run manifest.

#![forbid(unsafe_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TcsMetricId {
    /// Natural variability and individuality in Drosophila walking (2024):
    /// common swing overlap / (latest swing termination - earliest swing onset).
    NatVar2024,
    /// Sapkal et al. bioRxiv 2026.04.29.721658 v2:
    /// common swing overlap / (latest swing onset - earliest swing onset).
    Sapkal2026V2,
}

impl TcsMetricId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NatVar2024 => "TCS_NATVAR_2024",
            Self::Sapkal2026V2 => "TCS_SAPKAL_2026_V2",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwingInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TcsScore {
    pub metric_id: TcsMetricId,
    pub overlap_ticks: u64,
    pub denominator_ticks: u64,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReasonCode {
    InvalidSwingInterval,
    ZeroTcsDenominator,
    EmptyTcsSampleSet,
    MixedTcsMetricIds,
    NonFiniteTcsScore,
    EmptyPhaseSeries,
    PhaseLengthMismatch,
    NonFinitePhase,
    ZeroCycleDuration,
    DutyDurationOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricFailure {
    reasons: Vec<ReasonCode>,
}

impl MetricFailure {
    fn new(reason: ReasonCode) -> Self {
        Self {
            reasons: vec![reason],
        }
    }

    fn from_reasons(mut reasons: Vec<ReasonCode>) -> Self {
        reasons.sort_unstable();
        reasons.dedup();
        Self { reasons }
    }

    pub fn reasons(&self) -> &[ReasonCode] {
        &self.reasons
    }

    pub fn contains(&self, reason: ReasonCode) -> bool {
        self.reasons.binary_search(&reason).is_ok()
    }
}

/// Calculates one versioned tripod-coordination-strength value from three swing intervals.
///
/// The returned score is not clamped. In particular, the Sapkal v2 denominator is
/// the onset-to-onset span and can differ from the full event span used by NatVar 2024.
/// A zero denominator is explicitly not estimable and returns an error.
pub fn tripod_coordination_strength(
    intervals: [SwingInterval; 3],
    metric_id: TcsMetricId,
) -> Result<TcsScore, MetricFailure> {
    if intervals
        .iter()
        .any(|interval| interval.end_tick <= interval.start_tick)
    {
        return Err(MetricFailure::new(ReasonCode::InvalidSwingInterval));
    }

    let earliest_onset = intervals
        .iter()
        .map(|interval| interval.start_tick)
        .min()
        .expect("fixed-size interval array is non-empty");
    let latest_onset = intervals
        .iter()
        .map(|interval| interval.start_tick)
        .max()
        .expect("fixed-size interval array is non-empty");
    let latest_termination = intervals
        .iter()
        .map(|interval| interval.end_tick)
        .max()
        .expect("fixed-size interval array is non-empty");
    let latest_common_onset = intervals
        .iter()
        .map(|interval| interval.start_tick)
        .max()
        .expect("fixed-size interval array is non-empty");
    let earliest_common_termination = intervals
        .iter()
        .map(|interval| interval.end_tick)
        .min()
        .expect("fixed-size interval array is non-empty");

    let overlap_ticks = earliest_common_termination.saturating_sub(latest_common_onset);
    let denominator_ticks = match metric_id {
        TcsMetricId::NatVar2024 => latest_termination - earliest_onset,
        TcsMetricId::Sapkal2026V2 => latest_onset - earliest_onset,
    };

    if denominator_ticks == 0 {
        return Err(MetricFailure::new(ReasonCode::ZeroTcsDenominator));
    }

    Ok(TcsScore {
        metric_id,
        overlap_ticks,
        denominator_ticks,
        value: overlap_ticks as f64 / denominator_ticks as f64,
    })
}

/// Median of per-event TCS scores from exactly one metric definition.
///
/// The caller decides the trial and stratum boundary. Non-tripod events must be
/// represented as zero scores before calling this function when required by the
/// selected paper's event protocol. Undefined events must not be imputed as zero.
pub fn median_tcs(scores: &[TcsScore]) -> Result<(TcsMetricId, f64), MetricFailure> {
    if scores.is_empty() {
        return Err(MetricFailure::new(ReasonCode::EmptyTcsSampleSet));
    }

    let metric_id = scores[0].metric_id;
    let mut values = Vec::with_capacity(scores.len());
    for score in scores {
        if score.metric_id != metric_id {
            return Err(MetricFailure::new(ReasonCode::MixedTcsMetricIds));
        }
        if !score.value.is_finite() || score.value < 0.0 {
            return Err(MetricFailure::new(ReasonCode::NonFiniteTcsScore));
        }
        values.push(score.value);
    }

    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    let median = if values.len() % 2 == 0 {
        values[middle - 1] / 2.0 + values[middle] / 2.0
    } else {
        values[middle]
    };
    Ok((metric_id, median))
}

/// Calculates conventional pairwise phase-vector strength from supplied instantaneous phases.
///
/// This function does not filter signals or calculate a Hilbert transform. The
/// phase-estimation method, sampling interval and preprocessing must be bound
/// separately in the experiment manifest. It must not be substituted for a
/// paper-specific PLV implementation without confirming formula equivalence.
pub fn phase_vector_strength(phase_a: &[f64], phase_b: &[f64]) -> Result<f64, MetricFailure> {
    if phase_a.is_empty() || phase_b.is_empty() {
        return Err(MetricFailure::new(ReasonCode::EmptyPhaseSeries));
    }
    if phase_a.len() != phase_b.len() {
        return Err(MetricFailure::new(ReasonCode::PhaseLengthMismatch));
    }
    if phase_a
        .iter()
        .chain(phase_b.iter())
        .any(|phase| !phase.is_finite())
    {
        return Err(MetricFailure::new(ReasonCode::NonFinitePhase));
    }

    let mut sum_cos = 0.0;
    let mut sum_sin = 0.0;
    for (left, right) in phase_a.iter().zip(phase_b.iter()) {
        let difference = left - right;
        sum_cos += difference.cos();
        sum_sin += difference.sin();
    }
    let count = phase_a.len() as f64;
    Ok((sum_cos.hypot(sum_sin) / count).clamp(0.0, 1.0))
}

/// Computes stance duty factor = stance / (stance + swing).
///
/// Zero durations for one phase are permitted; a zero-length complete cycle and
/// integer overflow are rejected. The returned value is in [0, 1].
pub fn duty_factor(stance_ticks: u64, swing_ticks: u64) -> Result<f64, MetricFailure> {
    let cycle_ticks = stance_ticks
        .checked_add(swing_ticks)
        .ok_or_else(|| MetricFailure::new(ReasonCode::DutyDurationOverflow))?;
    if cycle_ticks == 0 {
        return Err(MetricFailure::new(ReasonCode::ZeroCycleDuration));
    }
    Ok(stance_ticks as f64 / cycle_ticks as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn swing(start_tick: u64, end_tick: u64) -> SwingInterval {
        SwingInterval {
            start_tick,
            end_tick,
        }
    }

    #[test]
    fn natvar_tcs_is_one_for_perfect_synchronous_tripod() {
        let score = tripod_coordination_strength(
            [swing(0, 10), swing(0, 10), swing(0, 10)],
            TcsMetricId::NatVar2024,
        )
        .expect("NatVar denominator is non-zero");
        assert_eq!(score.overlap_ticks, 10);
        assert_eq!(score.denominator_ticks, 10);
        assert_eq!(score.value, 1.0);
    }

    #[test]
    fn sapkal_tcs_reports_zero_onset_span_as_not_estimable() {
        let error = tripod_coordination_strength(
            [swing(0, 10), swing(0, 11), swing(0, 12)],
            TcsMetricId::Sapkal2026V2,
        )
        .expect_err("simultaneous onsets yield zero denominator");
        assert!(error.contains(ReasonCode::ZeroTcsDenominator));
    }

    #[test]
    fn natvar_and_sapkal_definitions_keep_distinct_denominators() {
        let intervals = [swing(0, 10), swing(2, 12), swing(4, 14)];
        let natvar =
            tripod_coordination_strength(intervals, TcsMetricId::NatVar2024).expect("NatVar score");
        let sapkal = tripod_coordination_strength(intervals, TcsMetricId::Sapkal2026V2)
            .expect("Sapkal score");
        assert_eq!(natvar.overlap_ticks, 6);
        assert_eq!(natvar.denominator_ticks, 14);
        assert_eq!(sapkal.overlap_ticks, 6);
        assert_eq!(sapkal.denominator_ticks, 4);
        assert_ne!(natvar.value, sapkal.value);
        assert_eq!(natvar.value, 6.0 / 14.0);
        assert_eq!(sapkal.value, 1.5, "Sapkal v2 result must not be clamped");
    }

    #[test]
    fn natvar_tcs_is_bounded_by_geometry() {
        let score = tripod_coordination_strength(
            [swing(0, 10), swing(3, 12), swing(4, 14)],
            TcsMetricId::NatVar2024,
        )
        .expect("valid overlap");
        assert!((0.0..=1.0).contains(&score.value));
    }

    #[test]
    fn tcs_is_zero_when_three_way_overlap_is_absent() {
        let score = tripod_coordination_strength(
            [swing(0, 2), swing(3, 5), swing(6, 8)],
            TcsMetricId::NatVar2024,
        )
        .expect("positive full event span");
        assert_eq!(score.overlap_ticks, 0);
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn pairwise_overlap_without_three_way_overlap_is_zero() {
        let score = tripod_coordination_strength(
            [swing(0, 4), swing(3, 7), swing(6, 10)],
            TcsMetricId::NatVar2024,
        )
        .expect("valid intervals");
        assert_eq!(score.overlap_ticks, 0);
        assert_eq!(score.value, 0.0);
    }

    #[test]
    fn tcs_is_input_order_invariant() {
        let first = tripod_coordination_strength(
            [swing(0, 10), swing(2, 12), swing(4, 14)],
            TcsMetricId::NatVar2024,
        )
        .expect("first order");
        let second = tripod_coordination_strength(
            [swing(4, 14), swing(0, 10), swing(2, 12)],
            TcsMetricId::NatVar2024,
        )
        .expect("permuted order");
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_reversed_swing_interval_rejects() {
        let error = tripod_coordination_strength(
            [swing(0, 10), swing(5, 5), swing(3, 12)],
            TcsMetricId::NatVar2024,
        )
        .expect_err("zero-width interval must reject");
        assert!(error.contains(ReasonCode::InvalidSwingInterval));
    }

    #[test]
    fn zero_width_swing_rejects() {
        let error = tripod_coordination_strength(
            [swing(0, 10), swing(4, 4), swing(3, 12)],
            TcsMetricId::NatVar2024,
        )
        .expect_err("zero-width swing");
        assert!(error.contains(ReasonCode::InvalidSwingInterval));
    }

    #[test]
    fn tcs_handles_large_tick_values_without_integer_overflow() {
        let base = u64::MAX - 100;
        let score = tripod_coordination_strength(
            [
                swing(base, base + 20),
                swing(base + 5, base + 25),
                swing(base + 10, base + 30),
            ],
            TcsMetricId::NatVar2024,
        )
        .expect("bounded tick differences");
        assert_eq!(score.overlap_ticks, 10);
        assert_eq!(score.denominator_ticks, 30);
    }

    #[test]
    fn median_tcs_uses_middle_for_odd_sample_count() {
        let scores = [
            tripod_coordination_strength(
                [swing(0, 5), swing(3, 8), swing(6, 11)],
                TcsMetricId::NatVar2024,
            )
            .expect("first"),
            tripod_coordination_strength(
                [swing(0, 10), swing(2, 12), swing(4, 14)],
                TcsMetricId::NatVar2024,
            )
            .expect("second"),
            tripod_coordination_strength(
                [swing(0, 20), swing(1, 21), swing(2, 22)],
                TcsMetricId::NatVar2024,
            )
            .expect("third"),
        ];
        let (_, median) = median_tcs(&scores).expect("median");
        assert!((median - (6.0 / 14.0)).abs() < 1e-12);
    }

    #[test]
    fn median_tcs_averages_two_middle_values_for_even_count() {
        let scores = [
            tripod_coordination_strength(
                [swing(0, 5), swing(3, 8), swing(6, 11)],
                TcsMetricId::NatVar2024,
            )
            .expect("first"),
            tripod_coordination_strength(
                [swing(0, 20), swing(1, 21), swing(2, 22)],
                TcsMetricId::NatVar2024,
            )
            .expect("second"),
        ];
        let (_, median) = median_tcs(&scores).expect("median");
        let expected = (0.0 + (18.0 / 22.0)) / 2.0;
        assert!((median - expected).abs() < 1e-12);
    }

    #[test]
    fn median_tcs_rejects_empty_samples() {
        let error = median_tcs(&[]).expect_err("empty values");
        assert!(error.contains(ReasonCode::EmptyTcsSampleSet));
    }

    #[test]
    fn median_tcs_rejects_mixed_metric_definitions() {
        let natvar = tripod_coordination_strength(
            [swing(0, 10), swing(2, 12), swing(4, 14)],
            TcsMetricId::NatVar2024,
        )
        .expect("NatVar");
        let sapkal = tripod_coordination_strength(
            [swing(0, 10), swing(2, 12), swing(4, 14)],
            TcsMetricId::Sapkal2026V2,
        )
        .expect("Sapkal");
        let error = median_tcs(&[natvar, sapkal]).expect_err("mixed metric IDs");
        assert!(error.contains(ReasonCode::MixedTcsMetricIds));
    }

    #[test]
    fn equal_phase_difference_has_unit_phase_vector_strength() {
        let phase_a = [0.0, 0.5, 1.0, 1.5, 2.0];
        let phase_b = phase_a.map(|phase| phase + 0.7);
        let value = phase_vector_strength(&phase_a, &phase_b).expect("PLV");
        assert!((value - 1.0).abs() < 1e-12);
    }

    #[test]
    fn uniformly_distributed_phase_differences_have_near_zero_strength() {
        let phase_a = [0.0, 0.0, 0.0, 0.0];
        let phase_b = [
            0.0,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
            3.0 * std::f64::consts::FRAC_PI_2,
        ];
        let value = phase_vector_strength(&phase_a, &phase_b).expect("PLV");
        assert!(value < 1e-12);
    }

    #[test]
    fn phase_vector_strength_rejects_empty_input() {
        let error = phase_vector_strength(&[], &[]).expect_err("empty phase arrays");
        assert!(error.contains(ReasonCode::EmptyPhaseSeries));
    }

    #[test]
    fn phase_vector_strength_rejects_length_mismatch() {
        let error =
            phase_vector_strength(&[0.0, 1.0], &[0.0]).expect_err("different phase lengths");
        assert!(error.contains(ReasonCode::PhaseLengthMismatch));
    }

    #[test]
    fn phase_vector_strength_rejects_non_finite_phase() {
        let error =
            phase_vector_strength(&[0.0, f64::NAN], &[0.0, 1.0]).expect_err("non-finite phase");
        assert!(error.contains(ReasonCode::NonFinitePhase));
    }

    #[test]
    fn duty_factor_uses_stance_over_complete_cycle() {
        assert_eq!(duty_factor(6, 4).expect("valid cycle"), 0.6);
    }

    #[test]
    fn duty_factor_allows_zero_stance_or_swing_boundary() {
        assert_eq!(duty_factor(0, 10).expect("zero stance"), 0.0);
        assert_eq!(duty_factor(10, 0).expect("zero swing"), 1.0);
    }

    #[test]
    fn duty_factor_rejects_zero_cycle() {
        let error = duty_factor(0, 0).expect_err("zero cycle");
        assert!(error.contains(ReasonCode::ZeroCycleDuration));
    }

    #[test]
    fn duty_factor_rejects_duration_overflow() {
        let error = duty_factor(u64::MAX, 1).expect_err("overflowing cycle");
        assert!(error.contains(ReasonCode::DutyDurationOverflow));
    }
}
