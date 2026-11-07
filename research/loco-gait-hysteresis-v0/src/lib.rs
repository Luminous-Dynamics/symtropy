//! Deterministic reference models for history-dependent gait-regime transitions.
//!
//! This crate qualifies a small Schmitt-trigger-style hysteresis model against
//! a memoryless single-threshold baseline. It does not infer biological
//! hysteresis, model sensory latency or simulate body/contact mechanics.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaitRegime {
    Wave,
    Tripod,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TransitionRule {
    /// One speed threshold controls both directions. At the threshold the
    /// selected regime changes, which can chatter when speed remains there.
    SingleThreshold { threshold_metres_per_second: f64 },
    /// Separate thresholds retain the current regime inside the hysteresis
    /// band. Wave -> Tripod uses the upper threshold; Tripod -> Wave uses
    /// the lower threshold.
    Hysteretic {
        wave_to_tripod_metres_per_second: f64,
        tripod_to_wave_metres_per_second: f64,
    },
}

impl TransitionRule {
    pub fn validate(self) -> Result<(), TransitionError> {
        match self {
            Self::SingleThreshold {
                threshold_metres_per_second,
            } => {
                if !threshold_metres_per_second.is_finite() || threshold_metres_per_second < 0.0 {
                    return Err(TransitionError::InvalidThreshold);
                }
            }
            Self::Hysteretic {
                wave_to_tripod_metres_per_second,
                tripod_to_wave_metres_per_second,
            } => {
                if !wave_to_tripod_metres_per_second.is_finite()
                    || !tripod_to_wave_metres_per_second.is_finite()
                    || tripod_to_wave_metres_per_second < 0.0
                    || wave_to_tripod_metres_per_second <= tripod_to_wave_metres_per_second
                {
                    return Err(TransitionError::InvalidHysteresisBand);
                }
            }
        }
        Ok(())
    }

    fn next_regime(self, current: GaitRegime, speed: f64) -> GaitRegime {
        match (self, current) {
            (
                Self::SingleThreshold {
                    threshold_metres_per_second,
                },
                GaitRegime::Wave,
            ) if speed >= threshold_metres_per_second => GaitRegime::Tripod,
            (
                Self::SingleThreshold {
                    threshold_metres_per_second,
                },
                GaitRegime::Tripod,
            ) if speed <= threshold_metres_per_second => GaitRegime::Wave,
            (
                Self::Hysteretic {
                    wave_to_tripod_metres_per_second,
                    ..
                },
                GaitRegime::Wave,
            ) if speed >= wave_to_tripod_metres_per_second => GaitRegime::Tripod,
            (
                Self::Hysteretic {
                    tripod_to_wave_metres_per_second,
                    ..
                },
                GaitRegime::Tripod,
            ) if speed <= tripod_to_wave_metres_per_second => GaitRegime::Wave,
            (_, regime) => regime,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeedSample {
    pub timestamp_seconds: f64,
    /// Achieved/measured speed, not the commanded speed.
    pub speed_metres_per_second: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionDirection {
    WaveToTripod,
    TripodToWave,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionEvent {
    pub direction: TransitionDirection,
    pub timestamp_seconds: f64,
    /// Measured speed at the first sample where the state transition occurs.
    /// This is sample-grid dependent and is not an interpolated threshold
    /// estimate.
    pub measured_speed_metres_per_second: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegimeSample {
    pub timestamp_seconds: f64,
    pub speed_metres_per_second: f64,
    pub regime: GaitRegime,
    pub transition: Option<TransitionEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionTrace {
    pub rule: TransitionRule,
    pub initial_regime: GaitRegime,
    pub samples: Vec<RegimeSample>,
    pub events: Vec<TransitionEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HysteresisMeasurement {
    pub wave_to_tripod_speed_metres_per_second: f64,
    pub tripod_to_wave_speed_metres_per_second: f64,
    /// Difference between observed up- and down-transition sample speeds.
    /// Positive values indicate a hysteresis loop over the sampled sweep.
    pub width_metres_per_second: f64,
    pub wave_to_tripod_time_seconds: f64,
    pub tripod_to_wave_time_seconds: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionError {
    InvalidThreshold,
    InvalidHysteresisBand,
    TooFewSamples,
    NonFiniteTimestamp,
    NonFiniteSpeed,
    NegativeSpeed,
    NonMonotonicTimestamp,
    NoCompleteWaveTripodWaveSweep,
    InvalidSweepOrder,
    InvalidTimeConstant,
    InvalidSpeedThreshold,
    NoCompleteLatencyCrossing,
}

fn validate_samples(samples: &[SpeedSample]) -> Result<(), TransitionError> {
    if samples.len() < 2 {
        return Err(TransitionError::TooFewSamples);
    }
    let mut previous_time = None;
    for sample in samples {
        if !sample.timestamp_seconds.is_finite() {
            return Err(TransitionError::NonFiniteTimestamp);
        }
        if !sample.speed_metres_per_second.is_finite() {
            return Err(TransitionError::NonFiniteSpeed);
        }
        if sample.speed_metres_per_second < 0.0 {
            return Err(TransitionError::NegativeSpeed);
        }
        if previous_time.is_some_and(|time| sample.timestamp_seconds <= time) {
            return Err(TransitionError::NonMonotonicTimestamp);
        }
        previous_time = Some(sample.timestamp_seconds);
    }
    Ok(())
}

/// Replay a sequence of achieved-speed observations through a memoryless or
/// hysteretic gait-regime selector.
///
/// A caller must supply the initial regime; the selector does not infer state
/// from the current speed. This makes history dependence explicit and testable.
pub fn replay_speed_sweep(
    rule: TransitionRule,
    initial_regime: GaitRegime,
    speeds: &[SpeedSample],
) -> Result<TransitionTrace, TransitionError> {
    rule.validate()?;
    validate_samples(speeds)?;

    let mut regime = initial_regime;
    let mut output = Vec::with_capacity(speeds.len());
    let mut events = Vec::new();

    for sample in speeds {
        let next_regime = rule.next_regime(regime, sample.speed_metres_per_second);
        let transition = if next_regime == regime {
            None
        } else {
            let direction = match (regime, next_regime) {
                (GaitRegime::Wave, GaitRegime::Tripod) => TransitionDirection::WaveToTripod,
                (GaitRegime::Tripod, GaitRegime::Wave) => TransitionDirection::TripodToWave,
                _ => return Err(TransitionError::InvalidSweepOrder),
            };
            Some(TransitionEvent {
                direction,
                timestamp_seconds: sample.timestamp_seconds,
                measured_speed_metres_per_second: sample.speed_metres_per_second,
            })
        };

        if let Some(event) = transition {
            events.push(event);
        }
        regime = next_regime;
        output.push(RegimeSample {
            timestamp_seconds: sample.timestamp_seconds,
            speed_metres_per_second: sample.speed_metres_per_second,
            regime,
            transition,
        });
    }

    Ok(TransitionTrace {
        rule,
        initial_regime,
        samples: output,
        events,
    })
}

/// Measure the first complete Wave -> Tripod -> Wave cycle in a speed sweep.
///
/// Events are measured at the first sampled observation that triggered each
/// transition. Do not compare different sampling rates without a declared
/// interpolation/event-time method.
pub fn measure_hysteresis(
    trace: &TransitionTrace,
) -> Result<HysteresisMeasurement, TransitionError> {
    let up = trace
        .events
        .iter()
        .find(|event| event.direction == TransitionDirection::WaveToTripod)
        .copied()
        .ok_or(TransitionError::NoCompleteWaveTripodWaveSweep)?;

    let down = trace
        .events
        .iter()
        .find(|event| {
            event.direction == TransitionDirection::TripodToWave
                && event.timestamp_seconds > up.timestamp_seconds
        })
        .copied()
        .ok_or(TransitionError::NoCompleteWaveTripodWaveSweep)?;

    let width = up.measured_speed_metres_per_second - down.measured_speed_metres_per_second;
    if !width.is_finite() || width < 0.0 {
        return Err(TransitionError::InvalidSweepOrder);
    }

    Ok(HysteresisMeasurement {
        wave_to_tripod_speed_metres_per_second: up.measured_speed_metres_per_second,
        tripod_to_wave_speed_metres_per_second: down.measured_speed_metres_per_second,
        width_metres_per_second: width,
        wave_to_tripod_time_seconds: up.timestamp_seconds,
        tripod_to_wave_time_seconds: down.timestamp_seconds,
    })
}

/// Speed and first-order lagged observation at the same timestamp.
///
/// The actual speed is retained separately from the lagged signal so an
/// apparent transition loop can be quantified in physical-speed coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaggedSpeedSample {
    pub timestamp_seconds: f64,
    pub actual_speed_metres_per_second: f64,
    pub observed_speed_metres_per_second: f64,
}

/// Exact first-order response to a piecewise-linear achieved-speed trajectory:
///
/// ```text
/// tau * dy/dt = v(t) - y(t)
/// ```
///
/// The input is linearly interpolated between samples. This is a synthetic
/// latency/low-pass confound model, not a fitted sensor or actuator model.
pub fn apply_first_order_speed_lag(
    samples: &[SpeedSample],
    time_constant_seconds: f64,
) -> Result<Vec<LaggedSpeedSample>, TransitionError> {
    validate_samples(samples)?;
    if !time_constant_seconds.is_finite() || time_constant_seconds <= 0.0 {
        return Err(TransitionError::InvalidTimeConstant);
    }

    let mut output = Vec::with_capacity(samples.len());
    let first = samples[0];
    let mut observed_speed = first.speed_metres_per_second;
    output.push(LaggedSpeedSample {
        timestamp_seconds: first.timestamp_seconds,
        actual_speed_metres_per_second: first.speed_metres_per_second,
        observed_speed_metres_per_second: observed_speed,
    });

    for pair in samples.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        let elapsed = current.timestamp_seconds - previous.timestamp_seconds;
        let slope = (current.speed_metres_per_second - previous.speed_metres_per_second) / elapsed;
        let attenuation = (-elapsed / time_constant_seconds).exp();

        // Exact solution for linearly changing input over this interval.
        observed_speed = previous.speed_metres_per_second
            + slope * (elapsed - time_constant_seconds)
            + (observed_speed - previous.speed_metres_per_second + slope * time_constant_seconds)
                * attenuation;

        if !observed_speed.is_finite() {
            return Err(TransitionError::NonFiniteSpeed);
        }
        output.push(LaggedSpeedSample {
            timestamp_seconds: current.timestamp_seconds,
            actual_speed_metres_per_second: current.speed_metres_per_second,
            observed_speed_metres_per_second: observed_speed,
        });
    }
    Ok(output)
}

/// Measure the apparent hysteresis produced when a memoryless threshold is
/// applied to a lagged speed signal.
///
/// Up/down transitions are detected in observed-speed coordinates, but the
/// returned loop width is computed from actual speed at those event samples.
/// A positive width therefore demonstrates how latency plus a speed ramp can
/// mimic hysteresis without any state-dependent transition thresholds.
pub fn measure_latency_induced_hysteresis(
    samples: &[LaggedSpeedSample],
    threshold_metres_per_second: f64,
) -> Result<HysteresisMeasurement, TransitionError> {
    if samples.len() < 2 {
        return Err(TransitionError::TooFewSamples);
    }
    if !threshold_metres_per_second.is_finite() || threshold_metres_per_second < 0.0 {
        return Err(TransitionError::InvalidSpeedThreshold);
    }

    let mut previous = samples[0];
    if !previous.timestamp_seconds.is_finite()
        || !previous.actual_speed_metres_per_second.is_finite()
        || !previous.observed_speed_metres_per_second.is_finite()
    {
        return Err(TransitionError::NonFiniteSpeed);
    }
    if previous.actual_speed_metres_per_second < 0.0
        || previous.observed_speed_metres_per_second < 0.0
    {
        return Err(TransitionError::NegativeSpeed);
    }

    let mut up_event = None;
    let mut down_event = None;
    for current in samples.iter().copied().skip(1) {
        if !current.timestamp_seconds.is_finite()
            || !current.actual_speed_metres_per_second.is_finite()
            || !current.observed_speed_metres_per_second.is_finite()
        {
            return Err(TransitionError::NonFiniteSpeed);
        }
        if current.timestamp_seconds <= previous.timestamp_seconds {
            return Err(TransitionError::NonMonotonicTimestamp);
        }
        if current.actual_speed_metres_per_second < 0.0
            || current.observed_speed_metres_per_second < 0.0
        {
            return Err(TransitionError::NegativeSpeed);
        }

        if up_event.is_none()
            && previous.observed_speed_metres_per_second < threshold_metres_per_second
            && current.observed_speed_metres_per_second >= threshold_metres_per_second
        {
            up_event = Some(current);
        } else if up_event.is_some()
            && down_event.is_none()
            && previous.observed_speed_metres_per_second > threshold_metres_per_second
            && current.observed_speed_metres_per_second <= threshold_metres_per_second
        {
            down_event = Some(current);
            break;
        }
        previous = current;
    }

    let up = up_event.ok_or(TransitionError::NoCompleteLatencyCrossing)?;
    let down = down_event.ok_or(TransitionError::NoCompleteLatencyCrossing)?;
    let width = up.actual_speed_metres_per_second - down.actual_speed_metres_per_second;
    if !width.is_finite() || width < 0.0 {
        return Err(TransitionError::InvalidSweepOrder);
    }

    Ok(HysteresisMeasurement {
        wave_to_tripod_speed_metres_per_second: up.actual_speed_metres_per_second,
        tripod_to_wave_speed_metres_per_second: down.actual_speed_metres_per_second,
        width_metres_per_second: width,
        wave_to_tripod_time_seconds: up.timestamp_seconds,
        tripod_to_wave_time_seconds: down.timestamp_seconds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangular_sweep() -> Vec<SpeedSample> {
        let speeds = [
            0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0, 0.9, 0.8, 0.7, 0.6, 0.5, 0.4,
            0.3, 0.2, 0.1, 0.0,
        ];
        speeds
            .iter()
            .enumerate()
            .map(|(index, speed)| SpeedSample {
                timestamp_seconds: index as f64,
                speed_metres_per_second: *speed,
            })
            .collect()
    }

    #[test]
    fn hysteretic_selector_recovers_known_up_and_down_thresholds() {
        let rule = TransitionRule::Hysteretic {
            wave_to_tripod_metres_per_second: 0.7,
            tripod_to_wave_metres_per_second: 0.4,
        };
        let trace = replay_speed_sweep(rule, GaitRegime::Wave, &triangular_sweep()).unwrap();
        let measurement = measure_hysteresis(&trace).unwrap();

        assert_eq!(measurement.wave_to_tripod_speed_metres_per_second, 0.7);
        assert_eq!(measurement.tripod_to_wave_speed_metres_per_second, 0.4);
        assert!((measurement.width_metres_per_second - 0.3).abs() < 1.0e-12);
        assert!(measurement.tripod_to_wave_time_seconds > measurement.wave_to_tripod_time_seconds);
    }

    #[test]
    fn single_threshold_baseline_has_zero_sweep_width() {
        let rule = TransitionRule::SingleThreshold {
            threshold_metres_per_second: 0.6,
        };
        let trace = replay_speed_sweep(rule, GaitRegime::Wave, &triangular_sweep()).unwrap();
        let measurement = measure_hysteresis(&trace).unwrap();

        assert_eq!(measurement.wave_to_tripod_speed_metres_per_second, 0.6);
        assert_eq!(measurement.tripod_to_wave_speed_metres_per_second, 0.6);
        assert!((measurement.width_metres_per_second).abs() < 1.0e-12);
    }

    #[test]
    fn same_speed_can_preserve_different_regimes_inside_the_hysteresis_band() {
        let rule = TransitionRule::Hysteretic {
            wave_to_tripod_metres_per_second: 0.7,
            tripod_to_wave_metres_per_second: 0.4,
        };
        let speed = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.55,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 0.55,
            },
        ];

        let from_wave = replay_speed_sweep(rule, GaitRegime::Wave, &speed).unwrap();
        let from_tripod = replay_speed_sweep(rule, GaitRegime::Tripod, &speed).unwrap();

        assert!(
            from_wave
                .samples
                .iter()
                .all(|sample| sample.regime == GaitRegime::Wave)
        );
        assert!(
            from_tripod
                .samples
                .iter()
                .all(|sample| sample.regime == GaitRegime::Tripod)
        );
    }

    #[test]
    fn holding_exactly_on_a_single_threshold_can_chatter() {
        let rule = TransitionRule::SingleThreshold {
            threshold_metres_per_second: 0.6,
        };
        let speeds = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.6,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 0.6,
            },
            SpeedSample {
                timestamp_seconds: 2.0,
                speed_metres_per_second: 0.6,
            },
            SpeedSample {
                timestamp_seconds: 3.0,
                speed_metres_per_second: 0.6,
            },
        ];
        let trace = replay_speed_sweep(rule, GaitRegime::Wave, &speeds).unwrap();
        assert_eq!(trace.events.len(), 4);
    }

    #[test]
    fn hysteretic_selector_does_not_chatter_inside_the_band() {
        let rule = TransitionRule::Hysteretic {
            wave_to_tripod_metres_per_second: 0.7,
            tripod_to_wave_metres_per_second: 0.4,
        };
        let speeds = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.7,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 0.55,
            },
            SpeedSample {
                timestamp_seconds: 2.0,
                speed_metres_per_second: 0.55,
            },
            SpeedSample {
                timestamp_seconds: 3.0,
                speed_metres_per_second: 0.4,
            },
        ];
        let trace = replay_speed_sweep(rule, GaitRegime::Wave, &speeds).unwrap();
        assert_eq!(trace.events.len(), 2);
        assert_eq!(trace.samples[1].regime, GaitRegime::Tripod);
        assert_eq!(trace.samples[2].regime, GaitRegime::Tripod);
        assert_eq!(trace.samples[3].regime, GaitRegime::Wave);
    }

    #[test]
    fn hysteresis_band_must_have_strictly_ordered_thresholds() {
        assert_eq!(
            TransitionRule::Hysteretic {
                wave_to_tripod_metres_per_second: 0.5,
                tripod_to_wave_metres_per_second: 0.5,
            }
            .validate(),
            Err(TransitionError::InvalidHysteresisBand)
        );
        assert_eq!(
            TransitionRule::Hysteretic {
                wave_to_tripod_metres_per_second: 0.4,
                tripod_to_wave_metres_per_second: 0.5,
            }
            .validate(),
            Err(TransitionError::InvalidHysteresisBand)
        );
    }

    #[test]
    fn sweep_rejects_nonmonotonic_time_and_nonfinite_speed() {
        let nonmonotonic = [
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 0.0,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 1.0,
            },
        ];
        assert_eq!(
            replay_speed_sweep(
                TransitionRule::SingleThreshold {
                    threshold_metres_per_second: 0.5
                },
                GaitRegime::Wave,
                &nonmonotonic,
            ),
            Err(TransitionError::NonMonotonicTimestamp)
        );

        let nonfinite = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.0,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: f64::NAN,
            },
        ];
        assert_eq!(
            replay_speed_sweep(
                TransitionRule::SingleThreshold {
                    threshold_metres_per_second: 0.5
                },
                GaitRegime::Wave,
                &nonfinite,
            ),
            Err(TransitionError::NonFiniteSpeed)
        );
    }

    #[test]
    fn negative_speed_is_rejected() {
        let samples = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.0,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: -0.1,
            },
        ];
        assert_eq!(
            replay_speed_sweep(
                TransitionRule::SingleThreshold {
                    threshold_metres_per_second: 0.5
                },
                GaitRegime::Wave,
                &samples,
            ),
            Err(TransitionError::NegativeSpeed)
        );
    }

    #[test]
    fn incomplete_sweep_does_not_manufacture_a_hysteresis_measurement() {
        let samples = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.0,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 1.0,
            },
        ];
        let trace = replay_speed_sweep(
            TransitionRule::Hysteretic {
                wave_to_tripod_metres_per_second: 0.7,
                tripod_to_wave_metres_per_second: 0.4,
            },
            GaitRegime::Wave,
            &samples,
        )
        .unwrap();
        assert_eq!(
            measure_hysteresis(&trace),
            Err(TransitionError::NoCompleteWaveTripodWaveSweep)
        );
    }

    fn fine_triangular_sweep(time_step_seconds: f64) -> Vec<SpeedSample> {
        let mut speeds: Vec<f64> = (0..=50).map(|index| index as f64 / 50.0).collect();
        speeds.extend((0..50).rev().map(|index| index as f64 / 50.0));
        speeds
            .iter()
            .enumerate()
            .map(|(index, speed)| SpeedSample {
                timestamp_seconds: index as f64 * time_step_seconds,
                speed_metres_per_second: *speed,
            })
            .collect()
    }

    #[test]
    fn first_order_speed_lag_matches_exact_linear_ramp_solution() {
        let samples = [
            SpeedSample {
                timestamp_seconds: 0.0,
                speed_metres_per_second: 0.0,
            },
            SpeedSample {
                timestamp_seconds: 1.0,
                speed_metres_per_second: 1.0,
            },
        ];
        let lagged = apply_first_order_speed_lag(&samples, 1.0).unwrap();
        assert!((lagged[1].observed_speed_metres_per_second - (-1.0_f64).exp()).abs() < 1.0e-12);
    }

    #[test]
    fn latency_alone_creates_apparent_hysteresis_without_stateful_thresholds() {
        let actual = fine_triangular_sweep(0.05);
        let lagged = apply_first_order_speed_lag(&actual, 1.0).unwrap();
        let apparent = measure_latency_induced_hysteresis(&lagged, 0.5).unwrap();

        assert!(apparent.width_metres_per_second > 0.5);
        assert!(apparent.wave_to_tripod_speed_metres_per_second > 0.5);
        assert!(apparent.tripod_to_wave_speed_metres_per_second < 0.5);
    }

    #[test]
    fn apparent_latency_loop_grows_with_ramp_rate() {
        let fast = apply_first_order_speed_lag(&fine_triangular_sweep(0.05), 1.0).unwrap();
        let slow = apply_first_order_speed_lag(&fine_triangular_sweep(0.5), 1.0).unwrap();
        let fast_loop = measure_latency_induced_hysteresis(&fast, 0.5).unwrap();
        let slow_loop = measure_latency_induced_hysteresis(&slow, 0.5).unwrap();

        assert!(fast_loop.width_metres_per_second > slow_loop.width_metres_per_second);
        assert!(slow_loop.width_metres_per_second >= 0.0);
    }

    #[test]
    fn speed_lag_rejects_nonpositive_time_constant() {
        let samples = triangular_sweep();
        assert_eq!(
            apply_first_order_speed_lag(&samples, 0.0),
            Err(TransitionError::InvalidTimeConstant)
        );
    }

    #[test]
    fn latency_measurement_rejects_missing_crossing_or_invalid_threshold() {
        let samples = [
            LaggedSpeedSample {
                timestamp_seconds: 0.0,
                actual_speed_metres_per_second: 0.0,
                observed_speed_metres_per_second: 0.0,
            },
            LaggedSpeedSample {
                timestamp_seconds: 1.0,
                actual_speed_metres_per_second: 0.1,
                observed_speed_metres_per_second: 0.1,
            },
        ];
        assert_eq!(
            measure_latency_induced_hysteresis(&samples, 0.5),
            Err(TransitionError::NoCompleteLatencyCrossing)
        );
        assert_eq!(
            measure_latency_induced_hysteresis(&samples, f64::NAN),
            Err(TransitionError::InvalidSpeedThreshold)
        );
    }
}
