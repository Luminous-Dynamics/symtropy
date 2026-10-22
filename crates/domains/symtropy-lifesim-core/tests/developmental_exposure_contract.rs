// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable oracle for one specific developmental-history sufficient statistic:
//! exact cumulative signed exposure over authoritative simulation ticks.
//!
//! This is deliberately not a universal development model. It applies only to
//! drivers whose qualified semantics declare the time integral of a quantized
//! stimulus to be sufficient. Order-sensitive, thresholded, decaying, peak, or
//! windowed histories require distinct versioned accumulators.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ExposureSegment {
    start_tick: u64,
    end_tick: u64,
    stimulus_q: i32,
}

impl ExposureSegment {
    const fn new(start_tick: u64, end_tick: u64, stimulus_q: i32) -> Self {
        Self {
            start_tick,
            end_tick,
            stimulus_q,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CumulativeExposure {
    origin_tick: u64,
    end_tick: u64,
    dose_q_ticks: i128,
}

impl CumulativeExposure {
    const fn new(origin_tick: u64) -> Self {
        Self {
            origin_tick,
            end_tick: origin_tick,
            dose_q_ticks: 0,
        }
    }

    fn append(self, segment: ExposureSegment) -> Result<Self, ExposureError> {
        if segment.end_tick <= segment.start_tick {
            return Err(ExposureError::NonPositiveDuration {
                start_tick: segment.start_tick,
                end_tick: segment.end_tick,
            });
        }
        if segment.start_tick != self.end_tick {
            return Err(ExposureError::NonContiguousSegment {
                expected_start_tick: self.end_tick,
                actual_start_tick: segment.start_tick,
            });
        }

        let duration = segment.end_tick - segment.start_tick;
        let contribution = i128::from(segment.stimulus_q)
            .checked_mul(i128::from(duration))
            .ok_or(ExposureError::DoseOverflow)?;
        let dose_q_ticks = self
            .dose_q_ticks
            .checked_add(contribution)
            .ok_or(ExposureError::DoseOverflow)?;

        Ok(Self {
            origin_tick: self.origin_tick,
            end_tick: segment.end_tick,
            dose_q_ticks,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExposureError {
    NonPositiveDuration {
        start_tick: u64,
        end_tick: u64,
    },
    NonContiguousSegment {
        expected_start_tick: u64,
        actual_start_tick: u64,
    },
    DoseOverflow,
}

fn integrate(
    origin_tick: u64,
    segments: &[ExposureSegment],
) -> Result<CumulativeExposure, ExposureError> {
    segments
        .iter()
        .copied()
        .try_fold(CumulativeExposure::new(origin_tick), CumulativeExposure::append)
}

#[test]
fn constant_exposure_integrates_exactly_over_authoritative_ticks() {
    let state = integrate(100, &[ExposureSegment::new(100, 600, 7)]).unwrap();

    assert_eq!(state.origin_tick, 100);
    assert_eq!(state.end_tick, 600);
    assert_eq!(state.dose_q_ticks, 3_500);
}

#[test]
fn splitting_a_constant_interval_does_not_change_cumulative_exposure() {
    let whole = integrate(0, &[ExposureSegment::new(0, 10_000, -17)]).unwrap();
    let split = integrate(
        0,
        &[
            ExposureSegment::new(0, 1, -17),
            ExposureSegment::new(1, 37, -17),
            ExposureSegment::new(37, 9_999, -17),
            ExposureSegment::new(9_999, 10_000, -17),
        ],
    )
    .unwrap();

    assert_eq!(split, whole);
}

#[test]
fn fine_and_coarse_execution_match_when_cumulative_dose_is_declared_sufficient() {
    let fine_segments = (0u64..1_000)
        .map(|tick| ExposureSegment::new(tick, tick + 1, 23))
        .collect::<Vec<_>>();
    let fine = integrate(0, &fine_segments).unwrap();
    let coarse = integrate(0, &[ExposureSegment::new(0, 1_000, 23)]).unwrap();

    assert_eq!(fine, coarse);
}

#[test]
fn save_reload_preserves_future_exposure_exactly() {
    let before_save = integrate(
        10,
        &[
            ExposureSegment::new(10, 20, 5),
            ExposureSegment::new(20, 35, -2),
        ],
    )
    .unwrap();

    // `before_save` is the serialized/restored canonical sufficient statistic.
    let restored = before_save;
    let after_reload = restored
        .append(ExposureSegment::new(35, 70, 11))
        .unwrap();

    let continuous = integrate(
        10,
        &[
            ExposureSegment::new(10, 20, 5),
            ExposureSegment::new(20, 35, -2),
            ExposureSegment::new(35, 70, 11),
        ],
    )
    .unwrap();

    assert_eq!(after_reload, continuous);
}

#[test]
fn gaps_and_overlaps_fail_closed_instead_of_fabricating_history() {
    let initial = CumulativeExposure::new(100);

    assert_eq!(
        initial.append(ExposureSegment::new(101, 110, 3)),
        Err(ExposureError::NonContiguousSegment {
            expected_start_tick: 100,
            actual_start_tick: 101,
        })
    );

    let advanced = initial
        .append(ExposureSegment::new(100, 110, 3))
        .unwrap();
    assert_eq!(
        advanced.append(ExposureSegment::new(109, 120, 4)),
        Err(ExposureError::NonContiguousSegment {
            expected_start_tick: 110,
            actual_start_tick: 109,
        })
    );
}

#[test]
fn zero_or_backward_duration_is_rejected() {
    let state = CumulativeExposure::new(7);

    assert_eq!(
        state.append(ExposureSegment::new(7, 7, 1)),
        Err(ExposureError::NonPositiveDuration {
            start_tick: 7,
            end_tick: 7,
        })
    );
    assert_eq!(
        state.append(ExposureSegment::new(7, 6, 1)),
        Err(ExposureError::NonPositiveDuration {
            start_tick: 7,
            end_tick: 6,
        })
    );
}

#[test]
fn signed_stimuli_can_cancel_exactly_without_floating_point_drift() {
    let state = integrate(
        0,
        &[
            ExposureSegment::new(0, 100, 37),
            ExposureSegment::new(100, 137, -100),
        ],
    )
    .unwrap();

    assert_eq!(state.dose_q_ticks, 0);
}

#[test]
fn boundary_duration_and_stimulus_use_widened_arithmetic() {
    let state = integrate(
        0,
        &[ExposureSegment::new(0, u64::MAX, i32::MAX)],
    )
    .unwrap();
    assert_eq!(
        state.dose_q_ticks,
        i128::from(i32::MAX) * i128::from(u64::MAX)
    );

    let negative = integrate(
        0,
        &[ExposureSegment::new(0, u64::MAX, i32::MIN)],
    )
    .unwrap();
    assert_eq!(
        negative.dose_q_ticks,
        i128::from(i32::MIN) * i128::from(u64::MAX)
    );
}

#[test]
fn cumulative_dose_intentionally_forgets_order_and_must_not_be_used_for_order_sensitive_drivers() {
    let a_then_b = integrate(
        0,
        &[
            ExposureSegment::new(0, 10, 2),
            ExposureSegment::new(10, 20, 8),
        ],
    )
    .unwrap();
    let b_then_a = integrate(
        0,
        &[
            ExposureSegment::new(0, 10, 8),
            ExposureSegment::new(10, 20, 2),
        ],
    )
    .unwrap();

    assert_eq!(a_then_b, b_then_a);
    assert_eq!(a_then_b.dose_q_ticks, 100);

    // This equality is a *declared limitation*: this accumulator is qualified
    // only where cumulative dose is sufficient. A process for which exposure
    // order matters needs a different versioned history representation.
}
