#![forbid(unsafe_code)]

//! Deterministic source-to-actuator Age of Information (AoI) reference.
//!
//! V0 deliberately requires source and application timestamps to share one
//! robot monotonic clock and boot epoch. It does not synchronize clocks,
//! validate sensors, decide freshness eligibility, or authorize actuation.

use core::fmt;

/// Identifies one monotonic clock instance and its boot epoch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClockDomain {
    pub clock_id: u64,
    pub boot_epoch: u64,
}

/// A monotonic timestamp in integer nanoseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MonotonicNs(pub u64);

/// A source observation with immutable identity and generation timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceObservation {
    pub modality_id: u16,
    pub observation_id: u64,
    pub sequence: u64,
    pub generated_at: MonotonicNs,
    pub clock: ClockDomain,
}

/// A robot event timestamp, such as result receipt or actuator application.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RobotEventTime {
    pub event_id: u64,
    pub at: MonotonicNs,
    pub clock: ClockDomain,
}

/// Identity of the actuator command application event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActuatorApplication {
    pub actuator_id: u16,
    pub command_id: u64,
    pub decision_id: u64,
    pub time: RobotEventTime,
}

/// The measured age of one exact source observation at one event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceAge {
    pub modality_id: u16,
    pub observation_id: u64,
    pub sequence: u64,
    pub age_ns: u64,
}

/// Explicit failures; an invalid or unknown time relation never becomes zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoiError {
    ClockDomainMismatch,
    ObservationFromFuture,
    EventOrderUncertain,
    InvalidTimeInterval,
    MissingObservationLineage,
    DuplicateObservationIdentity,
    InvalidZeroDuration,
    InvalidQuantile,
    ArithmeticOverflow,
}

impl fmt::Display for AoiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for AoiError {}

/// Computes source-observation age at a robot event.
///
/// This function is valid only when both timestamps share the same clock ID
/// and boot epoch. A timestamp from a camera/edge/cloud clock must first be
/// mapped by a separately qualified clock-mapping owner.
pub fn source_age_at_event(
    source: SourceObservation,
    event: RobotEventTime,
) -> Result<SourceAge, AoiError> {
    if source.clock != event.clock {
        return Err(AoiError::ClockDomainMismatch);
    }

    let age_ns = event
        .at
        .0
        .checked_sub(source.generated_at.0)
        .ok_or(AoiError::ObservationFromFuture)?;

    Ok(SourceAge {
        modality_id: source.modality_id,
        observation_id: source.observation_id,
        sequence: source.sequence,
        age_ns,
    })
}

/// Computes the per-source AoI vector for one applied actuator command.
///
/// No average or scalar fusion is performed across sensor modalities. Each
/// exact observation dependency remains independently inspectable.
pub fn actuation_aoi_by_source(
    application: ActuatorApplication,
    sources: &[SourceObservation],
) -> Result<Vec<SourceAge>, AoiError> {
    if sources.is_empty() {
        return Err(AoiError::MissingObservationLineage);
    }

    let mut ages = Vec::with_capacity(sources.len());

    for (index, source) in sources.iter().copied().enumerate() {
        if sources[..index].iter().any(|previous| {
            previous.modality_id == source.modality_id
                && previous.observation_id == source.observation_id
        }) {
            return Err(AoiError::DuplicateObservationIdentity);
        }

        ages.push(source_age_at_event(source, application.time)?);
    }

    Ok(ages)
}


/// A bounded timestamp on one declared monotonic clock and boot epoch.
///
/// The interval represents timestamp uncertainty from capture/exposure,
/// device timestamping, event-latch timing, or instrumentation resolution.
/// It does not perform clock synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonotonicTimeInterval {
    pub earliest: MonotonicNs,
    pub latest: MonotonicNs,
    pub clock: ClockDomain,
}

impl MonotonicTimeInterval {
    pub fn new(
        earliest: MonotonicNs,
        latest: MonotonicNs,
        clock: ClockDomain,
    ) -> Result<Self, AoiError> {
        if earliest > latest {
            return Err(AoiError::InvalidTimeInterval);
        }
        Ok(Self {
            earliest,
            latest,
            clock,
        })
    }

    pub fn exact(at: MonotonicNs, clock: ClockDomain) -> Self {
        Self {
            earliest: at,
            latest: at,
            clock,
        }
    }

    fn validate(self) -> Result<Self, AoiError> {
        if self.earliest > self.latest {
            Err(AoiError::InvalidTimeInterval)
        } else {
            Ok(self)
        }
    }
}

/// Source-observation lineage with a bounded generation-time interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceObservationInterval {
    pub modality_id: u16,
    pub observation_id: u64,
    pub sequence: u64,
    pub generated: MonotonicTimeInterval,
}

/// Conservative source age interval at the actuator application event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceAgeBounds {
    pub modality_id: u16,
    pub observation_id: u64,
    pub sequence: u64,
    pub minimum_age_ns: u64,
    pub maximum_age_ns: u64,
}

/// Computes a conservative source-to-actuator age interval.
///
/// The observation and application intervals must share a clock/boot epoch,
/// and the earliest possible application must not precede the latest possible
/// observation generation. Otherwise event order is unknown and the function
/// fails closed rather than manufacturing a zero or negative age.
pub fn source_age_bounds_at_event(
    source: SourceObservationInterval,
    application: MonotonicTimeInterval,
) -> Result<SourceAgeBounds, AoiError> {
    let generated = source.generated.validate()?;
    let applied = application.validate()?;

    if generated.clock != applied.clock {
        return Err(AoiError::ClockDomainMismatch);
    }

    let minimum_age_ns = applied
        .earliest
        .0
        .checked_sub(generated.latest.0)
        .ok_or(AoiError::EventOrderUncertain)?;
    let maximum_age_ns = applied
        .latest
        .0
        .checked_sub(generated.earliest.0)
        .ok_or(AoiError::ObservationFromFuture)?;

    if minimum_age_ns > maximum_age_ns {
        return Err(AoiError::InvalidTimeInterval);
    }

    Ok(SourceAgeBounds {
        modality_id: source.modality_id,
        observation_id: source.observation_id,
        sequence: source.sequence,
        minimum_age_ns,
        maximum_age_ns,
    })
}

/// Computes bounded per-source AoI for one actuator application interval.
///
/// Every sensor dependency stays distinct. No cross-modality scalar average
/// is produced, and every source must be traceable on the same robot clock.
pub fn actuation_aoi_bounds_by_source(
    application: MonotonicTimeInterval,
    sources: &[SourceObservationInterval],
) -> Result<Vec<SourceAgeBounds>, AoiError> {
    if sources.is_empty() {
        return Err(AoiError::MissingObservationLineage);
    }

    let mut bounds = Vec::with_capacity(sources.len());
    for (index, source) in sources.iter().copied().enumerate() {
        if sources[..index].iter().any(|previous| {
            previous.modality_id == source.modality_id
                && previous.observation_id == source.observation_id
        }) {
            return Err(AoiError::DuplicateObservationIdentity);
        }
        bounds.push(source_age_bounds_at_event(source, application)?);
    }
    Ok(bounds)
}

/// One interval during which a command remains applied and AoI grows at 1 ns/ns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AoiHoldSegment {
    pub age_at_start_ns: u64,
    pub duration_ns: u64,
}

impl AoiHoldSegment {
    pub fn new(age_at_start_ns: u64, duration_ns: u64) -> Result<Self, AoiError> {
        if duration_ns == 0 {
            return Err(AoiError::InvalidZeroDuration);
        }
        age_at_start_ns
            .checked_add(duration_ns)
            .ok_or(AoiError::ArithmeticOverflow)?;
        Ok(Self {
            age_at_start_ns,
            duration_ns,
        })
    }

    pub fn age_at_end_ns(self) -> Result<u64, AoiError> {
        self.age_at_start_ns
            .checked_add(self.duration_ns)
            .ok_or(AoiError::ArithmeticOverflow)
    }

    /// Returns twice the area under the linear AoI curve, in ns^2.
    ///
    /// Keeping twice-area integral avoids rounding half-nanosecond results:
    /// 2 * area = 2 * age_start * duration + duration^2.
    pub fn twice_area_ns2(self) -> Result<u128, AoiError> {
        if self.duration_ns == 0 {
            return Err(AoiError::InvalidZeroDuration);
        }
        self.age_at_end_ns()?;

        let age = u128::from(self.age_at_start_ns);
        let duration = u128::from(self.duration_ns);
        let first = age
            .checked_mul(duration)
            .and_then(|value| value.checked_mul(2))
            .ok_or(AoiError::ArithmeticOverflow)?;
        let second = duration
            .checked_mul(duration)
            .ok_or(AoiError::ArithmeticOverflow)?;
        first
            .checked_add(second)
            .ok_or(AoiError::ArithmeticOverflow)
    }

    fn time_at_or_below(self, age_ns: u64) -> Result<u128, AoiError> {
        let end = self.age_at_end_ns()?;
        if age_ns <= self.age_at_start_ns {
            Ok(0)
        } else {
            Ok(u128::from(age_ns.min(end) - self.age_at_start_ns))
        }
    }
}

/// Continuous-time summary of an AoI trace, weighted by elapsed time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeWeightedAoiSummary {
    pub total_duration_ns: u128,
    pub twice_area_ns2: u128,
    pub p50_ns: u64,
    pub p95_ns: u64,
    pub p99_ns: u64,
    pub maximum_ns: u64,
}

impl TimeWeightedAoiSummary {
    /// Returns the time-weighted mean AoI in nanoseconds.
    pub fn mean_aoi_ns(self) -> f64 {
        self.twice_area_ns2 as f64 / (2.0 * self.total_duration_ns as f64)
    }
}

/// Summarizes a piecewise-linear AoI trace with duration-weighted percentiles.
///
/// Each segment represents a held command: AoI starts at its declared start
/// age and increases linearly for the declared duration. Irregular update
/// intervals are correctly weighted by their actual elapsed duration.
pub fn summarize_time_weighted_aoi(
    segments: &[AoiHoldSegment],
) -> Result<TimeWeightedAoiSummary, AoiError> {
    if segments.is_empty() {
        return Err(AoiError::InvalidZeroDuration);
    }

    let mut total_duration_ns = 0_u128;
    let mut twice_area_ns2 = 0_u128;
    let mut minimum_ns = u64::MAX;
    let mut maximum_ns = 0_u64;

    for segment in segments.iter().copied() {
        if segment.duration_ns == 0 {
            return Err(AoiError::InvalidZeroDuration);
        }

        total_duration_ns = total_duration_ns
            .checked_add(u128::from(segment.duration_ns))
            .ok_or(AoiError::ArithmeticOverflow)?;
        twice_area_ns2 = twice_area_ns2
            .checked_add(segment.twice_area_ns2()?)
            .ok_or(AoiError::ArithmeticOverflow)?;

        minimum_ns = minimum_ns.min(segment.age_at_start_ns);
        maximum_ns = maximum_ns.max(segment.age_at_end_ns()?);
    }

    let p50_ns = time_weighted_quantile_ns(segments, 50, 100)?;
    let p95_ns = time_weighted_quantile_ns(segments, 95, 100)?;
    let p99_ns = time_weighted_quantile_ns(segments, 99, 100)?;

    if p50_ns < minimum_ns || p99_ns > maximum_ns {
        return Err(AoiError::ArithmeticOverflow);
    }

    Ok(TimeWeightedAoiSummary {
        total_duration_ns,
        twice_area_ns2,
        p50_ns,
        p95_ns,
        p99_ns,
        maximum_ns,
    })
}

/// Finds the smallest integer-nanosecond age whose time-weighted CDF reaches q.
///
/// q_numerator / q_denominator must be in [0, 1]. The CDF integrates elapsed
/// time along each continuously increasing hold segment rather than counting
/// one sample per command.
pub fn time_weighted_quantile_ns(
    segments: &[AoiHoldSegment],
    q_numerator: u32,
    q_denominator: u32,
) -> Result<u64, AoiError> {
    if segments.is_empty() {
        return Err(AoiError::InvalidZeroDuration);
    }
    if q_denominator == 0 || q_numerator > q_denominator {
        return Err(AoiError::InvalidQuantile);
    }

    let mut total_duration = 0_u128;
    let mut low = u64::MAX;
    let mut high = 0_u64;

    for segment in segments.iter().copied() {
        if segment.duration_ns == 0 {
            return Err(AoiError::InvalidZeroDuration);
        }
        total_duration = total_duration
            .checked_add(u128::from(segment.duration_ns))
            .ok_or(AoiError::ArithmeticOverflow)?;
        low = low.min(segment.age_at_start_ns);
        high = high.max(segment.age_at_end_ns()?);
    }

    if q_numerator == 0 {
        return Ok(low);
    }
    if q_numerator == q_denominator {
        return Ok(high);
    }

    let target = total_duration
        .checked_mul(u128::from(q_numerator))
        .ok_or(AoiError::ArithmeticOverflow)?;

    while low < high {
        let middle = low + (high - low) / 2;
        let mut accumulated = 0_u128;

        for segment in segments.iter().copied() {
            accumulated = accumulated
                .checked_add(segment.time_at_or_below(middle)?)
                .ok_or(AoiError::ArithmeticOverflow)?;
        }

        let lhs = accumulated
            .checked_mul(u128::from(q_denominator))
            .ok_or(AoiError::ArithmeticOverflow)?;

        if lhs >= target {
            high = middle;
        } else {
            low = middle + 1;
        }
    }

    Ok(low)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock() -> ClockDomain {
        ClockDomain {
            clock_id: 7,
            boot_epoch: 42,
        }
    }

    fn source(modality_id: u16, observation_id: u64, generated_at_ns: u64) -> SourceObservation {
        SourceObservation {
            modality_id,
            observation_id,
            sequence: observation_id,
            generated_at: MonotonicNs(generated_at_ns),
            clock: clock(),
        }
    }

    fn application(applied_at_ns: u64) -> ActuatorApplication {
        ActuatorApplication {
            actuator_id: 3,
            command_id: 100,
            decision_id: 55,
            time: RobotEventTime {
                event_id: 900,
                at: MonotonicNs(applied_at_ns),
                clock: clock(),
            },
        }
    }

    #[test]
    fn source_age_is_measured_at_actual_actuator_application() {
        let observation = source(1, 17, 1_000_000);
        let applied = application(1_050_000);

        let measured = source_age_at_event(observation, applied.time).unwrap();

        assert_eq!(measured.age_ns, 50_000);
        assert_eq!(measured.observation_id, 17);
        assert_eq!(measured.modality_id, 1);
    }

    #[test]
    fn queue_dwell_increases_age_after_result_receipt() {
        let observation = source(1, 18, 1_000_000);
        let received = RobotEventTime {
            event_id: 901,
            at: MonotonicNs(1_040_000),
            clock: clock(),
        };
        let applied = application(1_240_000);

        let receipt_age = source_age_at_event(observation, received).unwrap();
        let actuator_age = source_age_at_event(observation, applied.time).unwrap();

        assert_eq!(receipt_age.age_ns, 40_000);
        assert_eq!(actuator_age.age_ns, 240_000);
        assert!(actuator_age.age_ns > receipt_age.age_ns);
    }

    #[test]
    fn multiple_sensor_dependencies_remain_a_vector() {
        let sources = [
            source(1, 21, 1_000_000),
            source(2, 37, 1_020_000),
            source(3, 44, 1_030_000),
        ];

        let ages = actuation_aoi_by_source(application(1_080_000), &sources).unwrap();

        assert_eq!(ages.len(), 3);
        assert_eq!(ages[0].age_ns, 80_000);
        assert_eq!(ages[1].age_ns, 60_000);
        assert_eq!(ages[2].age_ns, 50_000);
        assert_eq!(ages[0].modality_id, 1);
        assert_eq!(ages[1].modality_id, 2);
        assert_eq!(ages[2].modality_id, 3);
    }

    #[test]
    fn held_command_mean_and_percentiles_are_time_weighted() {
        let segment = AoiHoldSegment::new(50_000_000, 20_000_000).unwrap();
        let summary = summarize_time_weighted_aoi(&[segment]).unwrap();

        assert_eq!(summary.total_duration_ns, 20_000_000);
        assert_eq!(summary.p50_ns, 60_000_000);
        assert_eq!(summary.p95_ns, 69_000_000);
        assert_eq!(summary.p99_ns, 69_800_000);
        assert_eq!(summary.maximum_ns, 70_000_000);
        assert!((summary.mean_aoi_ns() - 60_000_000.0).abs() < 0.5);
    }

    #[test]
    fn irregular_update_intervals_are_weighted_by_elapsed_time() {
        let segments = [
            AoiHoldSegment::new(0, 10).unwrap(),
            AoiHoldSegment::new(100, 100).unwrap(),
        ];
        let summary = summarize_time_weighted_aoi(&segments).unwrap();

        assert_eq!(summary.total_duration_ns, 110);
        assert_eq!(summary.p50_ns, 145);
        assert_eq!(summary.p95_ns, 195);
        assert_eq!(summary.p99_ns, 199);
    }

    #[test]
    fn cross_clock_or_boot_epoch_subtraction_is_rejected() {
        let observation = source(1, 25, 1_000);
        let other_clock = ClockDomain {
            clock_id: 8,
            boot_epoch: 42,
        };
        let event = RobotEventTime {
            event_id: 902,
            at: MonotonicNs(2_000),
            clock: other_clock,
        };

        assert_eq!(
            source_age_at_event(observation, event),
            Err(AoiError::ClockDomainMismatch)
        );
    }

    #[test]
    fn future_observation_and_missing_lineage_are_rejected() {
        let future = source(1, 26, 2_000);
        assert_eq!(
            source_age_at_event(future, application(1_000).time),
            Err(AoiError::ObservationFromFuture)
        );
        assert_eq!(
            actuation_aoi_by_source(application(1_000), &[]),
            Err(AoiError::MissingObservationLineage)
        );
    }

    #[test]
    fn duplicate_source_identity_is_rejected() {
        let observation = source(1, 27, 1_000);
        let sources = [observation, observation];

        assert_eq!(
            actuation_aoi_by_source(application(2_000), &sources),
            Err(AoiError::DuplicateObservationIdentity)
        );
    }


    #[test]
    fn timestamp_uncertainty_produces_conservative_age_bounds() {
        let source = SourceObservationInterval {
            modality_id: 1,
            observation_id: 31,
            sequence: 31,
            generated: MonotonicTimeInterval::new(
                MonotonicNs(1_000),
                MonotonicNs(1_010),
                clock(),
            )
            .unwrap(),
        };
        let application = MonotonicTimeInterval::new(
            MonotonicNs(1_050),
            MonotonicNs(1_060),
            clock(),
        )
        .unwrap();

        let bounds = source_age_bounds_at_event(source, application).unwrap();

        assert_eq!(bounds.minimum_age_ns, 40);
        assert_eq!(bounds.maximum_age_ns, 60);
    }

    #[test]
    fn overlapping_event_intervals_are_not_declared_fresh() {
        let source = SourceObservationInterval {
            modality_id: 1,
            observation_id: 32,
            sequence: 32,
            generated: MonotonicTimeInterval::new(
                MonotonicNs(100),
                MonotonicNs(120),
                clock(),
            )
            .unwrap(),
        };
        let application = MonotonicTimeInterval::new(
            MonotonicNs(115),
            MonotonicNs(130),
            clock(),
        )
        .unwrap();

        assert_eq!(
            source_age_bounds_at_event(source, application),
            Err(AoiError::EventOrderUncertain)
        );
    }

    #[test]
    fn invalid_and_cross_clock_uncertainty_intervals_fail_closed() {
        assert_eq!(
            MonotonicTimeInterval::new(MonotonicNs(20), MonotonicNs(10), clock()),
            Err(AoiError::InvalidTimeInterval)
        );

        let source = SourceObservationInterval {
            modality_id: 1,
            observation_id: 33,
            sequence: 33,
            generated: MonotonicTimeInterval::exact(MonotonicNs(10), clock()),
        };
        let other_clock = ClockDomain {
            clock_id: 8,
            boot_epoch: 42,
        };
        let application =
            MonotonicTimeInterval::exact(MonotonicNs(20), other_clock);

        assert_eq!(
            source_age_bounds_at_event(source, application),
            Err(AoiError::ClockDomainMismatch)
        );
    }

    #[test]
    fn uncertain_multisensor_ages_remain_separate() {
        let sources = [
            SourceObservationInterval {
                modality_id: 1,
                observation_id: 41,
                sequence: 41,
                generated: MonotonicTimeInterval::new(
                    MonotonicNs(1_000),
                    MonotonicNs(1_005),
                    clock(),
                )
                .unwrap(),
            },
            SourceObservationInterval {
                modality_id: 2,
                observation_id: 42,
                sequence: 42,
                generated: MonotonicTimeInterval::new(
                    MonotonicNs(1_010),
                    MonotonicNs(1_020),
                    clock(),
                )
                .unwrap(),
            },
        ];
        let application = MonotonicTimeInterval::new(
            MonotonicNs(1_050),
            MonotonicNs(1_060),
            clock(),
        )
        .unwrap();

        let bounds = actuation_aoi_bounds_by_source(application, &sources).unwrap();

        assert_eq!(bounds.len(), 2);
        assert_eq!(bounds[0].minimum_age_ns, 45);
        assert_eq!(bounds[0].maximum_age_ns, 60);
        assert_eq!(bounds[1].minimum_age_ns, 30);
        assert_eq!(bounds[1].maximum_age_ns, 50);
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        let segment = AoiHoldSegment {
            age_at_start_ns: u64::MAX,
            duration_ns: u64::MAX,
        };
        assert_eq!(
            segment.twice_area_ns2(),
            Err(AoiError::ArithmeticOverflow)
        );
        assert_eq!(
            AoiHoldSegment::new(u64::MAX, 1),
            Err(AoiError::ArithmeticOverflow)
        );
    }
}
