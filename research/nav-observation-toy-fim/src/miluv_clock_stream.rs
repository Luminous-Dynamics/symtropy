//! Evidence-bounded reconstruction of one UWB radio's uint32 clock stream.
//!
//! Each sample pairs a raw device counter with an independently recorded host
//! acquisition timestamp. The caller must supply hard acquisition-time bounds
//! that include publisher/transport latency, and a hard oscillator-rate
//! tolerance that includes device skew. The algorithm enumerates the integer
//! number of uint32 wraps consistent with those bounds and accepts only a
//! unique result.
//!
//! This is deliberately not a heuristic unwrap. Missing bounds, clock resets,
//! mixed clock identities, gaps beyond the declared horizon, or multiple
//! admissible wrap counts fail closed. A stable boot_epoch is an externally
//! established radio boot/reset epoch; repeating the same value does not by
//! itself prove that a hardware reset did not occur.

use std::fmt;

const TICKS_PER_NS_NUMERATOR: u128 = 638_976;
const TICKS_PER_NS_DENOMINATOR: u128 = 10_000;
const PPM_DENOMINATOR: u128 = 1_000_000;
const EXPECTED_TICKS_DENOMINATOR: u128 =
    TICKS_PER_NS_DENOMINATOR * PPM_DENOMINATOR;
const UINT32_MODULUS: u128 = 1_u128 << 32;
const MAX_CLOCK_RATE_TOLERANCE_PPM: u32 = 100_000;

/// One radio's raw device timestamp paired with host-side acquisition time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiluvClockStreamSample {
    /// Physical radio/tag identity; all samples in one stream must match.
    pub clock_id: u32,
    /// Externally established boot/reset epoch; all samples must match.
    pub boot_epoch: u64,
    /// Strictly increasing source transaction/event sequence.
    pub source_sequence: u64,
    /// Host acquisition time in nanoseconds, in one monotonic time domain.
    pub host_time_ns: u64,
    /// Hard absolute error bound on host acquisition time, including event-to-
    /// host capture latency and publisher/transport jitter, not just quantization.
    pub host_time_abs_error_ns: u64,
    /// Original unsigned 32-bit radio counter, before conversion/unwrapping.
    pub raw_ticks: u32,
}

/// Explicit assumptions required to reconstruct a radio clock stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiluvClockStreamConfig {
    /// Hard relative bound on the radio's tick rate versus the documented
    /// nominal DW1000 rate, in parts per million.
    pub clock_rate_tolerance_ppm: u32,
    /// Maximum allowed host-time separation between adjacent retained samples.
    /// Larger gaps are segmented rather than silently bridged.
    pub maximum_host_time_gap_ns: u64,
}

/// One reconstructed stream observation, with the inferred counter history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MiluvUnwrappedClockSample {
    pub source_sequence: u64,
    pub host_time_ns: u64,
    pub raw_ticks: u32,
    pub unwrapped_ticks: u128,
    /// Number of uint32 wrap boundaries crossed since the first sample.
    pub cumulative_rollovers: u128,
    /// Unique tick advance from the preceding sample; absent for the first.
    pub inferred_tick_delta: Option<u128>,
    /// Feasible tick-advance interval derived from host time and hard bounds.
    pub feasible_tick_delta_lower: Option<u128>,
    pub feasible_tick_delta_upper: Option<u128>,
}

/// Auditable result for one contiguous, single-radio clock segment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiluvClockStreamReceipt {
    pub clock_id: u32,
    pub boot_epoch: u64,
    pub clock_rate_tolerance_ppm: u32,
    pub maximum_host_time_gap_ns: u64,
    pub samples: Vec<MiluvUnwrappedClockSample>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MiluvClockStreamError {
    EmptyStream,
    InvalidConfig,
    InvalidHostTimeBounds,
    MixedClockIdentity,
    MixedBootEpoch,
    NonIncreasingSourceSequence,
    NonMonotonicHostTime,
    HostTimeGapTooLarge,
    NoConsistentClockAdvance,
    AmbiguousClockAdvance,
    ArithmeticOverflow,
}

impl fmt::Display for MiluvClockStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStream => write!(f, "clock stream must contain at least one sample"),
            Self::InvalidConfig => write!(
                f,
                "clock-rate tolerance must be <= 100000 ppm and maximum host-time gap must be positive"
            ),
            Self::InvalidHostTimeBounds => write!(
                f,
                "host timestamp uncertainty bounds are inconsistent with the declared gap"
            ),
            Self::MixedClockIdentity => {
                write!(f, "clock stream contains more than one radio identity")
            }
            Self::MixedBootEpoch => {
                write!(f, "clock stream crosses an unknown or changed radio boot/reset epoch")
            }
            Self::NonIncreasingSourceSequence => {
                write!(f, "source transaction/event sequence must strictly increase")
            }
            Self::NonMonotonicHostTime => {
                write!(f, "host acquisition timestamps must be nondecreasing")
            }
            Self::HostTimeGapTooLarge => {
                write!(f, "host-time gap exceeds the declared clock-stream continuity horizon")
            }
            Self::NoConsistentClockAdvance => {
                write!(f, "no uint32 wrap count satisfies the clock-rate and host-time bounds")
            }
            Self::AmbiguousClockAdvance => {
                write!(f, "multiple uint32 wrap counts satisfy the clock-rate and host-time bounds")
            }
            Self::ArithmeticOverflow => {
                write!(f, "clock-stream interval arithmetic overflowed")
            }
        }
    }
}

impl std::error::Error for MiluvClockStreamError {}

/// Reconstruct one radio's clock stream while preserving fail-closed
/// ambiguity handling.
///
/// The input must already be grouped by radio/clock identity and ordered by
/// source sequence. Equal host timestamps are permitted only when their hard
/// uncertainty bounds still make the raw clock advance unique. Samples from
/// different radios or boot epochs must be processed in separate streams.
///
/// The nominal device rate is 499.2 MHz * 128 = 63,897.6 ticks per microsecond
/// (63.8976 ticks/ns). The integer rational below avoids floating-point
/// rounding when selecting the number of counter wraps.
pub fn unwrap_miluv_clock_stream(
    samples: &[MiluvClockStreamSample],
    config: MiluvClockStreamConfig,
) -> Result<MiluvClockStreamReceipt, MiluvClockStreamError> {
    if samples.is_empty() {
        return Err(MiluvClockStreamError::EmptyStream);
    }
    if config.maximum_host_time_gap_ns == 0
        || config.clock_rate_tolerance_ppm > MAX_CLOCK_RATE_TOLERANCE_PPM
    {
        return Err(MiluvClockStreamError::InvalidConfig);
    }

    let first = samples[0];
    let mut output = Vec::with_capacity(samples.len());
    output.push(MiluvUnwrappedClockSample {
        source_sequence: first.source_sequence,
        host_time_ns: first.host_time_ns,
        raw_ticks: first.raw_ticks,
        unwrapped_ticks: first.raw_ticks as u128,
        cumulative_rollovers: 0,
        inferred_tick_delta: None,
        feasible_tick_delta_lower: None,
        feasible_tick_delta_upper: None,
    });

    for pair in samples.windows(2) {
        let previous = pair[0];
        let current = pair[1];

        if current.clock_id != first.clock_id || previous.clock_id != first.clock_id {
            return Err(MiluvClockStreamError::MixedClockIdentity);
        }
        if current.boot_epoch != first.boot_epoch || previous.boot_epoch != first.boot_epoch {
            return Err(MiluvClockStreamError::MixedBootEpoch);
        }
        if current.source_sequence <= previous.source_sequence {
            return Err(MiluvClockStreamError::NonIncreasingSourceSequence);
        }
        if current.host_time_ns < previous.host_time_ns {
            return Err(MiluvClockStreamError::NonMonotonicHostTime);
        }

        let host_delta_ns =
            (current.host_time_ns - previous.host_time_ns) as u128;
        let host_uncertainty_ns =
            previous.host_time_abs_error_ns as u128
                + current.host_time_abs_error_ns as u128;
        let host_delta_lower_ns = host_delta_ns.saturating_sub(host_uncertainty_ns);
        let host_delta_upper_ns = host_delta_ns
            .checked_add(host_uncertainty_ns)
            .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;

        if host_delta_upper_ns > config.maximum_host_time_gap_ns as u128 {
            return Err(MiluvClockStreamError::HostTimeGapTooLarge);
        }
        if host_delta_lower_ns > host_delta_upper_ns {
            return Err(MiluvClockStreamError::InvalidHostTimeBounds);
        }

        let (tick_delta, feasible_lower, feasible_upper) =
            infer_unique_tick_advance(previous.raw_ticks, current.raw_ticks,
                host_delta_lower_ns, host_delta_upper_ns,
                config.clock_rate_tolerance_ppm)?;

        let previous_unwrapped = output
            .last()
            .expect("the first sample is always inserted")
            .unwrapped_ticks;
        let unwrapped_ticks = previous_unwrapped
            .checked_add(tick_delta)
            .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;

        output.push(MiluvUnwrappedClockSample {
            source_sequence: current.source_sequence,
            host_time_ns: current.host_time_ns,
            raw_ticks: current.raw_ticks,
            unwrapped_ticks,
            cumulative_rollovers: unwrapped_ticks / UINT32_MODULUS,
            inferred_tick_delta: Some(tick_delta),
            feasible_tick_delta_lower: Some(feasible_lower),
            feasible_tick_delta_upper: Some(feasible_upper),
        });
    }

    Ok(MiluvClockStreamReceipt {
        clock_id: first.clock_id,
        boot_epoch: first.boot_epoch,
        clock_rate_tolerance_ppm: config.clock_rate_tolerance_ppm,
        maximum_host_time_gap_ns: config.maximum_host_time_gap_ns,
        samples: output,
    })
}

fn infer_unique_tick_advance(
    previous_raw: u32,
    current_raw: u32,
    host_delta_lower_ns: u128,
    host_delta_upper_ns: u128,
    clock_rate_tolerance_ppm: u32,
) -> Result<(u128, u128, u128), MiluvClockStreamError> {
    let raw_modular_delta = if current_raw >= previous_raw {
        (current_raw - previous_raw) as u128
    } else {
        UINT32_MODULUS - previous_raw as u128 + current_raw as u128
    };

    let rate_low_ppm = PPM_DENOMINATOR
        .checked_sub(clock_rate_tolerance_ppm as u128)
        .ok_or(MiluvClockStreamError::InvalidConfig)?;
    let rate_high_ppm = PPM_DENOMINATOR
        .checked_add(clock_rate_tolerance_ppm as u128)
        .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;

    let lower_numerator = host_delta_lower_ns
        .checked_mul(TICKS_PER_NS_NUMERATOR)
        .and_then(|value| value.checked_mul(rate_low_ppm))
        .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;
    let upper_numerator = host_delta_upper_ns
        .checked_mul(TICKS_PER_NS_NUMERATOR)
        .and_then(|value| value.checked_mul(rate_high_ppm))
        .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;

    let feasible_lower = div_ceil(lower_numerator, EXPECTED_TICKS_DENOMINATOR);
    let feasible_upper = upper_numerator / EXPECTED_TICKS_DENOMINATOR;

    if feasible_upper < raw_modular_delta {
        return Err(MiluvClockStreamError::NoConsistentClockAdvance);
    }

    let minimum_wraps = if feasible_lower <= raw_modular_delta {
        0
    } else {
        div_ceil(
            feasible_lower - raw_modular_delta,
            UINT32_MODULUS,
        )
    };
    let maximum_wraps =
        (feasible_upper - raw_modular_delta) / UINT32_MODULUS;

    if minimum_wraps > maximum_wraps {
        return Err(MiluvClockStreamError::NoConsistentClockAdvance);
    }
    if minimum_wraps != maximum_wraps {
        return Err(MiluvClockStreamError::AmbiguousClockAdvance);
    }

    let tick_delta = raw_modular_delta
        .checked_add(
            minimum_wraps
                .checked_mul(UINT32_MODULUS)
                .ok_or(MiluvClockStreamError::ArithmeticOverflow)?,
        )
        .ok_or(MiluvClockStreamError::ArithmeticOverflow)?;

    if tick_delta == 0 {
        return Err(MiluvClockStreamError::NoConsistentClockAdvance);
    }

    Ok((tick_delta, feasible_lower, feasible_upper))
}

fn div_ceil(numerator: u128, denominator: u128) -> u128 {
    numerator / denominator + u128::from(numerator % denominator != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(
        source_sequence: u64,
        host_time_ns: u64,
        host_time_abs_error_ns: u64,
        raw_ticks: u32,
    ) -> MiluvClockStreamSample {
        MiluvClockStreamSample {
            clock_id: 10,
            boot_epoch: 7,
            source_sequence,
            host_time_ns,
            host_time_abs_error_ns,
            raw_ticks,
        }
    }

    fn config(maximum_host_time_gap_ns: u64) -> MiluvClockStreamConfig {
        MiluvClockStreamConfig {
            clock_rate_tolerance_ppm: 0,
            maximum_host_time_gap_ns,
        }
    }

    #[test]
    fn reconstructs_normal_monotonic_stream_with_exact_rational_tick_rate() {
        // 625 ns * 63.8976 ticks/ns = exactly 39,936 ticks.
        let receipt = unwrap_miluv_clock_stream(
            &[
                sample(1, 10_000, 0, 1_000),
                sample(2, 10_625, 0, 40_936),
            ],
            config(1_000),
        )
        .unwrap();

        assert_eq!(receipt.samples[0].unwrapped_ticks, 1_000);
        assert_eq!(receipt.samples[1].inferred_tick_delta, Some(39_936));
        assert_eq!(receipt.samples[1].unwrapped_ticks, 40_936);
        assert_eq!(receipt.samples[1].cumulative_rollovers, 0);
    }

    #[test]
    fn reconstructs_one_wrap_using_host_time_and_radio_rate_bounds() {
        let near_wrap = u32::MAX - 199; // 2^32 - 200.
        let receipt = unwrap_miluv_clock_stream(
            &[
                sample(1, 1_000, 1, near_wrap),
                sample(2, 1_005, 1, 100),
            ],
            config(10),
        )
        .unwrap();

        assert_eq!(receipt.samples[1].inferred_tick_delta, Some(300));
        assert_eq!(
            receipt.samples[1].unwrapped_ticks,
            UINT32_MODULUS + 100
        );
        assert_eq!(receipt.samples[1].cumulative_rollovers, 1);
    }

    #[test]
    fn reconstructs_multiple_wraps_when_the_host_time_bound_is_unique() {
        // 150 ms at 63.8976 ticks/ns is exactly 9,584,640,000 ticks (> 2 wraps).
        let raw_start = 100_000_000_u32;
        let tick_delta = 9_584_640_000_u128;
        let raw_end =
            ((raw_start as u128 + tick_delta) % UINT32_MODULUS) as u32;

        let receipt = unwrap_miluv_clock_stream(
            &[
                sample(10, 1_000_000_000, 0, raw_start),
                sample(11, 1_150_000_000, 0, raw_end),
            ],
            config(200_000_000),
        )
        .unwrap();

        assert_eq!(receipt.samples[1].inferred_tick_delta, Some(tick_delta));
        assert_eq!(
            receipt.samples[1].unwrapped_ticks,
            raw_start as u128 + tick_delta
        );
        assert_eq!(receipt.samples[1].cumulative_rollovers, 2);
    }

    #[test]
    fn rejects_ambiguous_multi_wrap_interval_instead_of_guessing() {
        let raw_start = 100_000_000_u32;
        let tick_delta = 9_584_640_000_u128;
        let raw_end =
            ((raw_start as u128 + tick_delta) % UINT32_MODULUS) as u32;

        assert_eq!(
            unwrap_miluv_clock_stream(
                &[
                    sample(10, 1_000_000_000, 70_000_000, raw_start),
                    sample(11, 1_150_000_000, 70_000_000, raw_end),
                ],
                config(250_000_000),
            ),
            Err(MiluvClockStreamError::AmbiguousClockAdvance)
        );
    }

    #[test]
    fn rejects_a_raw_clock_advance_inconsistent_with_host_time() {
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[
                    sample(1, 10_000, 0, 1_000),
                    sample(2, 10_625, 0, 1_300),
                ],
                config(1_000),
            ),
            Err(MiluvClockStreamError::NoConsistentClockAdvance)
        );
    }

    #[test]
    fn accepts_equal_host_timestamps_only_when_uncertainty_supports_unique_advance() {
        // A 2 ns host interval with 2 ns per-endpoint uncertainty permits a
        // 300-tick delta, but not an unbounded number of counter wraps.
        let receipt = unwrap_miluv_clock_stream(
            &[
                sample(1, 10_000, 2, 1_000),
                sample(2, 10_000, 2, 1_300),
            ],
            config(10),
        )
        .unwrap();

        assert_eq!(receipt.samples[1].inferred_tick_delta, Some(300));
    }

    #[test]
    fn rejects_mixed_radio_identities() {
        let mut second = sample(2, 10_625, 0, 40_936);
        second.clock_id = 11;
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[sample(1, 10_000, 0, 1_000), second],
                config(1_000),
            ),
            Err(MiluvClockStreamError::MixedClockIdentity)
        );
    }

    #[test]
    fn rejects_boot_epoch_changes_and_does_not_bridge_resets() {
        let mut second = sample(2, 10_625, 0, 40_936);
        second.boot_epoch = 8;
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[sample(1, 10_000, 0, 1_000), second],
                config(1_000),
            ),
            Err(MiluvClockStreamError::MixedBootEpoch)
        );
    }

    #[test]
    fn rejects_non_increasing_source_sequence_and_host_time() {
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[
                    sample(2, 10_000, 0, 1_000),
                    sample(2, 10_625, 0, 40_936),
                ],
                config(1_000),
            ),
            Err(MiluvClockStreamError::NonIncreasingSourceSequence)
        );

        assert_eq!(
            unwrap_miluv_clock_stream(
                &[
                    sample(1, 10_625, 0, 1_000),
                    sample(2, 10_000, 0, 40_936),
                ],
                config(1_000),
            ),
            Err(MiluvClockStreamError::NonMonotonicHostTime)
        );
    }

    #[test]
    fn rejects_gaps_beyond_declared_clock_continuity_horizon() {
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[
                    sample(1, 10_000, 1, 1_000),
                    sample(2, 20_000, 1, 639_976),
                ],
                config(1_000),
            ),
            Err(MiluvClockStreamError::HostTimeGapTooLarge)
        );
    }

    #[test]
    fn rejects_invalid_configuration_and_empty_stream() {
        assert_eq!(
            unwrap_miluv_clock_stream(&[], config(1_000)),
            Err(MiluvClockStreamError::EmptyStream)
        );
        assert_eq!(
            unwrap_miluv_clock_stream(
                &[sample(1, 10_000, 0, 1_000)],
                MiluvClockStreamConfig {
                    clock_rate_tolerance_ppm: 100_001,
                    maximum_host_time_gap_ns: 1_000,
                },
            ),
            Err(MiluvClockStreamError::InvalidConfig)
        );
    }
}
