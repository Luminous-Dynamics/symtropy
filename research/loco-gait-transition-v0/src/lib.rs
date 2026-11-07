//! Matched up/down gait-transition analysis with ramp-rate latency controls.
//!
//! Under matched approximately linear speed ramps, a constant effective
//! transition-loop delay tau adds 2 * tau * ramp_rate to the observed
//! up/down transition-width. The intercept is a quasi-static width under this
//! model; it is not automatically a biological hysteresis claim.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Regime {
    Low,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SweepDirection {
    Increasing,
    Decreasing,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SweepSample {
    pub timestamp: f64,
    /// Measured/achieved speed, not the commanded setpoint.
    pub measured_speed: f64,
    pub regime: Regime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SweepTrace {
    pub trace_id: String,
    /// Matches an up-sweep and down-sweep with the same individual,
    /// context, preparation, terrain and repetition.
    pub pair_id: String,
    pub direction: SweepDirection,
    pub samples: Vec<SweepSample>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionBracket {
    pub direction: SweepDirection,
    pub before_timestamp: f64,
    pub after_timestamp: f64,
    /// Sorted measured-speed interval containing the transition.
    pub lower_speed: f64,
    pub upper_speed: f64,
    pub midpoint_speed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PairMeasurement {
    pub pair_id: String,
    pub increasing_trace_id: String,
    pub decreasing_trace_id: String,
    pub increasing_transition: TransitionBracket,
    pub decreasing_transition: TransitionBracket,
    pub increasing_ramp_rate: f64,
    pub decreasing_ramp_rate: f64,
    pub matched_ramp_rate: f64,
    pub apparent_width: f64,
    pub width_lower_bound: f64,
    pub width_upper_bound: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearFit {
    pub intercept: f64,
    pub slope: f64,
    pub slope_standard_error: f64,
    pub r_squared: f64,
    pub observation_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HysteresisEstimate {
    pub pairs: Vec<PairMeasurement>,
    /// Width extrapolated to zero measured ramp rate.
    /// This is a quasi-static width conditional on the constant-delay model.
    pub quasi_static_width: f64,
    /// Half the width/rate slope. This is an effective loop delay, not
    /// a receptor-specific or neural-conduction latency.
    pub effective_loop_delay: f64,
    pub effective_loop_delay_standard_error: f64,
    pub fit_r_squared: f64,
    pub width_rate_fit: LinearFit,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnalysisConfig {
    /// Maximum relative mismatch between paired up/down measured ramp rates.
    pub maximum_pair_rate_relative_error: f64,
    /// Minimum R^2 for the measured speed/time linearity check.
    pub minimum_ramp_linearity_r_squared: f64,
    /// Tolerance used to identify distinct ramp-rate levels.
    pub distinct_rate_tolerance: f64,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            maximum_pair_rate_relative_error: 0.10,
            minimum_ramp_linearity_r_squared: 0.98,
            distinct_rate_tolerance: 1.0e-6,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionError {
    NoTraces,
    EmptyTraceId,
    EmptyPairId,
    DuplicateTraceId(String),
    InvalidSampleCount(String),
    NonFiniteSample {
        trace_id: String,
        sample_index: usize,
    },
    NonIncreasingTimestamp { trace_id: String, sample_index: usize },
    NonMonotonicMeasuredSpeed { trace_id: String, sample_index: usize },
    WrongInitialRegime(String),
    WrongFinalRegime(String),
    MissingTransition(String),
    MultipleTransitions(String),
    InvalidRampLinearity {
        trace_id: String,
        r_squared: f64,
    },
    PairMustContainOneSweepEach(String),
    PairRampRatesMismatch {
        pair_id: String,
        relative_error: f64,
    },
    TooFewDistinctRampRates,
    InvalidAnalysisConfig,
    NumericalFailure,
}

#[derive(Debug, Clone)]
struct SweepSummary {
    trace_id: String,
    pair_id: String,
    direction: SweepDirection,
    transition: TransitionBracket,
    ramp_rate: f64,
}

#[derive(Debug, Clone, Copy)]
impl AnalysisConfig {
    pub fn validate(self) -> Result<(), TransitionError> {
        if !self.maximum_pair_rate_relative_error.is_finite()
            || self.maximum_pair_rate_relative_error < 0.0
            || !self.minimum_ramp_linearity_r_squared.is_finite()
            || !(0.0..=1.0).contains(&self.minimum_ramp_linearity_r_squared)
            || !self.distinct_rate_tolerance.is_finite()
            || self.distinct_rate_tolerance <= 0.0
        {
            return Err(TransitionError::InvalidAnalysisConfig);
        }
        Ok(())
    }
}

/// Analyze matched increasing/decreasing sweeps and regress apparent width
/// against measured ramp rate:
///
///     width = quasi_static_width + 2 * effective_loop_delay * rate
///
/// At least three distinct ramp-rate levels are required for a non-saturated
/// residual degree of freedom and a meaningful standard-error estimate.
pub fn analyze_hysteresis(
    traces: &[SweepTrace],
    config: AnalysisConfig,
) -> Result<HysteresisEstimate, TransitionError> {
    config.validate()?;
    if traces.is_empty() {
        return Err(TransitionError::NoTraces);
    }

    let mut trace_ids = BTreeSet::new();
    let mut grouped: BTreeMap<String, Vec<SweepSummary>> = BTreeMap::new();

    for trace in traces {
        if trace.trace_id.trim().is_empty() {
            return Err(TransitionError::EmptyTraceId);
        }
        if trace.pair_id.trim().is_empty() {
            return Err(TransitionError::EmptyPairId);
        }
        if !trace_ids.insert(trace.trace_id.clone()) {
            return Err(TransitionError::DuplicateTraceId(trace.trace_id.clone()));
        }

        let summary = summarize_trace(trace, config.minimum_ramp_linearity_r_squared)?;
        grouped
            .entry(trace.pair_id.clone())
            .or_default()
            .push(summary);
    }

    let mut pairs = Vec::with_capacity(grouped.len());
    for (pair_id, summaries) in grouped {
        if summaries.len() != 2 {
            return Err(TransitionError::PairMustContainOneSweepEach(pair_id));
        }
        let increasing = summaries
            .iter()
            .find(|summary| summary.direction == SweepDirection::Increasing)
            .ok_or_else(|| TransitionError::PairMustContainOneSweepEach(pair_id.clone()))?;
        let decreasing = summaries
            .iter()
            .find(|summary| summary.direction == SweepDirection::Decreasing)
            .ok_or_else(|| TransitionError::PairMustContainOneSweepEach(pair_id.clone()))?;

        let denominator = (increasing.ramp_rate + decreasing.ramp_rate) / 2.0;
        if !denominator.is_finite() || denominator <= 0.0 {
            return Err(TransitionError::NumericalFailure);
        }
        let relative_error = (increasing.ramp_rate - decreasing.ramp_rate).abs() / denominator;
        if relative_error > config.maximum_pair_rate_relative_error {
            return Err(TransitionError::PairRampRatesMismatch {
                pair_id,
                relative_error,
            });
        }

        let matched_ramp_rate = denominator;
        let up = increasing.transition;
        let down = decreasing.transition;
        let apparent_width = up.midpoint_speed - down.midpoint_speed;
        let width_lower_bound = up.lower_speed - down.upper_speed;
        let width_upper_bound = up.upper_speed - down.lower_speed;

        if !apparent_width.is_finite()
            || !width_lower_bound.is_finite()
            || !width_upper_bound.is_finite()
        {
            return Err(TransitionError::NumericalFailure);
        }

        pairs.push(PairMeasurement {
            pair_id,
            increasing_trace_id: increasing.trace_id.clone(),
            decreasing_trace_id: decreasing.trace_id.clone(),
            increasing_transition: up,
            decreasing_transition: down,
            increasing_ramp_rate: increasing.ramp_rate,
            decreasing_ramp_rate: decreasing.ramp_rate,
            matched_ramp_rate,
            apparent_width,
            width_lower_bound,
            width_upper_bound,
        });
    }

    pairs.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    let rates: Vec<f64> = pairs.iter().map(|pair| pair.matched_ramp_rate).collect();
    ensure_distinct_rates(&rates, config.distinct_rate_tolerance)?;

    let widths: Vec<f64> = pairs.iter().map(|pair| pair.apparent_width).collect();
    let fit = ordinary_least_squares(&rates, &widths)?;
    let quasi_static_width = fit.intercept;
    let effective_loop_delay = fit.slope / 2.0;
    let effective_loop_delay_standard_error = fit.slope_standard_error / 2.0;

    if !quasi_static_width.is_finite()
        || !effective_loop_delay.is_finite()
        || !effective_loop_delay_standard_error.is_finite()
    {
        return Err(TransitionError::NumericalFailure);
    }

    Ok(HysteresisEstimate {
        pairs,
        quasi_static_width,
        effective_loop_delay,
        effective_loop_delay_standard_error,
        fit_r_squared: fit.r_squared,
        width_rate_fit: fit,
    })
}

fn summarize_trace(
    trace: &SweepTrace,
    minimum_r_squared: f64,
) -> Result<SweepSummary, TransitionError> {
    if trace.samples.len() < 3 {
        return Err(TransitionError::InvalidSampleCount(trace.trace_id.clone()));
    }

    for (index, sample) in trace.samples.iter().enumerate() {
        if !sample.timestamp.is_finite() || !sample.measured_speed.is_finite() {
            return Err(TransitionError::NonFiniteSample {
                trace_id: trace.trace_id.clone(),
                sample_index: index,
            });
        }
        if index > 0 {
            let previous = &trace.samples[index - 1];
            if sample.timestamp <= previous.timestamp {
                return Err(TransitionError::NonIncreasingTimestamp {
                    trace_id: trace.trace_id.clone(),
                    sample_index: index,
                });
            }
            let monotonic = match trace.direction {
                SweepDirection::Increasing => sample.measured_speed > previous.measured_speed,
                SweepDirection::Decreasing => sample.measured_speed < previous.measured_speed,
            };
            if !monotonic {
                return Err(TransitionError::NonMonotonicMeasuredSpeed {
                    trace_id: trace.trace_id.clone(),
                    sample_index: index,
                });
            }
        }
    }

    let expected_initial = match trace.direction {
        SweepDirection::Increasing => Regime::Low,
        SweepDirection::Decreasing => Regime::High,
    };
    let expected_final = match trace.direction {
        SweepDirection::Increasing => Regime::High,
        SweepDirection::Decreasing => Regime::Low,
    };
    if trace.samples.first().map(|sample| sample.regime) != Some(expected_initial) {
        return Err(TransitionError::WrongInitialRegime(trace.trace_id.clone()));
    }
    if trace.samples.last().map(|sample| sample.regime) != Some(expected_final) {
        return Err(TransitionError::WrongFinalRegime(trace.trace_id.clone()));
    }

    let transition_indices: Vec<usize> = trace
        .samples
        .windows(2)
        .enumerate()
        .filter_map(|(index, window)| (window[0].regime != window[1].regime).then_some(index))
        .collect();
    if transition_indices.is_empty() {
        return Err(TransitionError::MissingTransition(trace.trace_id.clone()));
    }
    if transition_indices.len() != 1 {
        return Err(TransitionError::MultipleTransitions(trace.trace_id.clone()));
    }

    let index = transition_indices[0];
    let before = &trace.samples[index];
    let after = &trace.samples[index + 1];
    let lower_speed = before.measured_speed.min(after.measured_speed);
    let upper_speed = before.measured_speed.max(after.measured_speed);
    let transition = TransitionBracket {
        direction: trace.direction,
        before_timestamp: before.timestamp,
        after_timestamp: after.timestamp,
        lower_speed,
        upper_speed,
        midpoint_speed: (lower_speed + upper_speed) / 2.0,
    };

    let (ramp_rate, ramp_r_squared) = measured_ramp_rate(&trace.samples)?;
    if ramp_r_squared < minimum_r_squared {
        return Err(TransitionError::InvalidRampLinearity {
            trace_id: trace.trace_id.clone(),
            r_squared: ramp_r_squared,
        });
    }

    Ok(SweepSummary {
        trace_id: trace.trace_id.clone(),
        pair_id: trace.pair_id.clone(),
        direction: trace.direction,
        transition,
        ramp_rate,
    })
}

fn measured_ramp_rate(samples: &[SweepSample]) -> Result<(f64, f64), TransitionError> {
    let timestamps: Vec<f64> = samples.iter().map(|sample| sample.timestamp).collect();
    let speeds: Vec<f64> = samples.iter().map(|sample| sample.measured_speed).collect();
    let fit = ordinary_least_squares(&timestamps, &speeds)?;
    let rate = fit.slope.abs();
    if !rate.is_finite() || rate <= 0.0 {
        return Err(TransitionError::NumericalFailure);
    }
    Ok((rate, fit.r_squared))
}

fn ensure_distinct_rates(rates: &[f64], tolerance: f64) -> Result<(), TransitionError> {
    let mut unique = Vec::new();
    for rate in rates {
        if !unique.iter().any(|existing: &f64| {
            (*existing - *rate).abs() <= tolerance * existing.abs().max(rate.abs()).max(1.0)
        }) {
            unique.push(*rate);
        }
    }
    if unique.len() < 3 {
        return Err(TransitionError::TooFewDistinctRampRates);
    }
    Ok(())
}

fn ordinary_least_squares(x: &[f64], y: &[f64]) -> Result<LinearFit, TransitionError> {
    if x.len() != y.len() || x.len() < 2 {
        return Err(TransitionError::NumericalFailure);
    }
    let count = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / count;
    let mean_y = y.iter().sum::<f64>() / count;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    let mut sxy = 0.0;
    for (x_value, y_value) in x.iter().zip(y) {
        let dx = x_value - mean_x;
        let dy = y_value - mean_y;
        sxx += dx * dx;
        syy += dy * dy;
        sxy += dx * dy;
    }
    if !sxx.is_finite() || sxx <= f64::EPSILON {
        return Err(TransitionError::NumericalFailure);
    }

    let slope = sxy / sxx;
    let intercept = mean_y - slope * mean_x;
    let residual_sum_squares = x
        .iter()
        .zip(y)
        .map(|(x_value, y_value)| {
            let residual = y_value - (intercept + slope * x_value);
            residual * residual
        })
        .sum::<f64>();
    let r_squared = if syy <= f64::EPSILON {
        if residual_sum_squares <= f64::EPSILON {
            1.0
        } else {
            0.0
        }
    } else {
        (1.0 - residual_sum_squares / syy).clamp(0.0, 1.0)
    };
    let slope_standard_error = if x.len() > 2 {
        (residual_sum_squares / (x.len() - 2) as f64 / sxx).sqrt()
    } else {
        f64::NAN
    };

    if !slope.is_finite()
        || !intercept.is_finite()
        || !r_squared.is_finite()
        || (x.len() > 2 && !slope_standard_error.is_finite())
    {
        return Err(TransitionError::NumericalFailure);
    }

    Ok(LinearFit {
        intercept,
        slope,
        slope_standard_error,
        r_squared,
        observation_count: x.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trace(
        pair_id: &str,
        direction: SweepDirection,
        ramp_rate: f64,
        transition_speed: f64,
    ) -> SweepTrace {
        let times = [0.0, 1.0, 2.0, 3.0, 4.0];
        let offsets = [-1.5, -0.5, 0.5, 1.5, 2.5];
        let mut samples = Vec::new();
        for (index, (time, offset)) in times.iter().zip(offsets).enumerate() {
            let sign = match direction {
                SweepDirection::Increasing => 1.0,
                SweepDirection::Decreasing => -1.0,
            };
            let measured_speed = transition_speed + sign * ramp_rate * offset;
            let regime = match (direction, index) {
                (SweepDirection::Increasing, 0..=1) => Regime::Low,
                (SweepDirection::Increasing, _) => Regime::High,
                (SweepDirection::Decreasing, 0..=1) => Regime::High,
                (SweepDirection::Decreasing, _) => Regime::Low,
            };
            samples.push(SweepSample {
                timestamp: *time,
                measured_speed,
                regime,
            });
        }
        let direction_name = match direction {
            SweepDirection::Increasing => "up",
            SweepDirection::Decreasing => "down",
        };
        SweepTrace {
            trace_id: format!("{pair_id}-{direction_name}"),
            pair_id: pair_id.to_owned(),
            direction,
            samples,
        }
    }

    fn synthetic_traces(static_width: f64, effective_delay: f64, rates: &[f64]) -> Vec<SweepTrace> {
        let centre = 3.0;
        let mut traces = Vec::new();
        for (index, rate) in rates.iter().enumerate() {
            let up = centre + static_width / 2.0 + rate * effective_delay;
            let down = centre - static_width / 2.0 - rate * effective_delay;
            traces.push(trace(
                &format!("pair-{index}"),
                SweepDirection::Increasing,
                *rate,
                up,
            ));
            traces.push(trace(
                &format!("pair-{index}"),
                SweepDirection::Decreasing,
                *rate,
                down,
            ));
        }
        traces
    }

    fn config() -> AnalysisConfig {
        AnalysisConfig::default()
    }

    #[test]
    fn recovers_static_hysteresis_with_no_loop_delay() {
        let fit =
            analyze_hysteresis(&synthetic_traces(0.4, 0.0, &[0.5, 1.0, 2.0]), config()).unwrap();
        assert!((fit.quasi_static_width - 0.4).abs() < 1.0e-10);
        assert!(fit.effective_loop_delay.abs() < 1.0e-10);
        assert!((fit.fit_r_squared - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn delay_only_apparent_loop_has_zero_static_width_intercept() {
        let fit =
            analyze_hysteresis(&synthetic_traces(0.0, 0.25, &[0.5, 1.0, 2.0]), config()).unwrap();
        assert!(fit.quasi_static_width.abs() < 1.0e-10);
        assert!((fit.effective_loop_delay - 0.25).abs() < 1.0e-10);
    }

    #[test]
    fn separates_static_hysteresis_from_effective_delay() {
        let fit = analyze_hysteresis(
            &synthetic_traces(0.3, 0.2, &[0.25, 0.75, 1.5, 2.0]),
            config(),
        )
        .unwrap();
        assert!((fit.quasi_static_width - 0.3).abs() < 1.0e-10);
        assert!((fit.effective_loop_delay - 0.2).abs() < 1.0e-10);
        assert!(fit.effective_loop_delay_standard_error < 1.0e-10);
    }

    #[test]
    fn transition_brackets_are_retained_instead_of_fabricating_exact_events() {
        let traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        let fit = analyze_hysteresis(&traces, config()).unwrap();
        for pair in fit.pairs {
            assert!(
                pair.increasing_transition.lower_speed <= pair.increasing_transition.midpoint_speed
            );
            assert!(
                pair.increasing_transition.midpoint_speed <= pair.increasing_transition.upper_speed
            );
            assert!(
                pair.decreasing_transition.lower_speed <= pair.decreasing_transition.midpoint_speed
            );
            assert!(
                pair.decreasing_transition.midpoint_speed <= pair.decreasing_transition.upper_speed
            );
            assert!(pair.width_lower_bound <= pair.apparent_width);
            assert!(pair.apparent_width <= pair.width_upper_bound);
        }
    }

    #[test]
    fn result_is_invariant_to_trace_input_order() {
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        let left = analyze_hysteresis(&traces, config()).unwrap();
        traces.reverse();
        let right = analyze_hysteresis(&traces, config()).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn duplicate_trace_id_is_rejected() {
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        traces[1].trace_id = traces[0].trace_id.clone();
        assert!(matches!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::DuplicateTraceId(_))
        ));
    }

    #[test]
    fn missing_or_multiple_transitions_are_rejected() {
        let mut missing = trace("p", SweepDirection::Increasing, 1.0, 3.0);
        for sample in &mut missing.samples {
            sample.regime = Regime::Low;
        }
        let mut down = trace("p", SweepDirection::Decreasing, 1.0, 2.0);
        let mut multiple = trace("q", SweepDirection::Increasing, 1.0, 3.0);
        multiple.samples[2].regime = Regime::High;
        multiple.samples[3].regime = Regime::Low;
        let result = analyze_hysteresis(&[missing, down], config());
        assert!(matches!(result, Err(TransitionError::WrongFinalRegime(_))));
        let down = trace("p", SweepDirection::Decreasing, 1.0, 2.0);
        let result = analyze_hysteresis(&[multiple, down], config());
        assert!(matches!(
            result,
            Err(TransitionError::MultipleTransitions(_))
        ));
    }

    #[test]
    fn nonmonotonic_timestamp_and_speed_are_rejected() {
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        traces[0].samples[2].timestamp = traces[0].samples[1].timestamp;
        assert!(matches!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::NonIncreasingTimestamp { .. })
        ));
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        traces[0].samples[2].measured_speed = traces[0].samples[1].measured_speed;
        assert!(matches!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::NonMonotonicMeasuredSpeed { .. })
        ));
    }

    #[test]
    fn mismatched_up_down_rates_are_rejected() {
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        for sample in &mut traces[0].samples {
            sample.measured_speed = 3.0 + 0.2 * (sample.measured_speed - 3.0);
        }
        assert!(matches!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::PairRampRatesMismatch { .. })
        ));
    }

    #[test]
    fn too_few_distinct_ramp_rates_are_rejected() {
        let traces = synthetic_traces(0.3, 0.2, &[1.0, 1.0, 1.0]);
        assert_eq!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::TooFewDistinctRampRates)
        );
    }

    #[test]
    fn nonlinear_ramp_is_not_silently_treated_as_linear() {
        let mut traces = synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]);
        for sample in &mut traces[0].samples {
            sample.measured_speed += 0.2 * sample.timestamp * sample.timestamp;
        }
        assert!(matches!(
            analyze_hysteresis(&traces, config()),
            Err(TransitionError::InvalidRampLinearity { .. })
        ));
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        let mut cfg = config();
        cfg.minimum_ramp_linearity_r_squared = 1.5;
        assert_eq!(
            analyze_hysteresis(&synthetic_traces(0.3, 0.2, &[0.5, 1.0, 2.0]), cfg),
            Err(TransitionError::InvalidAnalysisConfig)
        );
    }
}
