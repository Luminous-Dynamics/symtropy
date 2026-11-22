//! Deterministic double-sided two-way ranging (DS-TWR) range interval.
//!
//! This implements the printed MILUV ranging equation over four explicitly
//! supplied positive duration terms. It does not infer how CSV tx/rx fields
//! map to the equation's clock domains. The caller must perform protocol-
//! versioned timestamp unwrapping, unit conversion, and duration construction.
//!
//! Input duration errors are hard absolute bounds in seconds. The module
//! propagates them with positive interval arithmetic; it does not assume
//! Gaussianity or statistical independence.

use std::fmt;

/// Numeric propagation-speed convention used by the pinned MILUV reference implementation.
pub const MILUV_REFERENCE_PROPAGATION_SPEED_MPS: f64 = 299_702_547.0;
/// Vacuum speed of light, available only when that physical convention is intended.
pub const VACUUM_PROPAGATION_SPEED_MPS: f64 = 299_792_458.0;

/// One timestamp from a processed MILUV CSV, already converted to nanoseconds
/// and unwrapped in its own radio clock domain.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiluvProcessedTimestampNs {
    /// Timestamp value in nanoseconds.
    pub value_ns: f64,
    /// Hard absolute timestamp error bound in nanoseconds.
    pub abs_error_bound_ns: f64,
}

/// Six DS-TWR events from the authors' processed/unwrapped MILUV representation.
/// The initiator and responder identities are retained to prevent accidental
/// mixing of two different ranging endpoints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiluvProcessedUnwrappedTimestampsNs {
    pub initiator_id: u32,
    pub responder_id: u32,
    pub initiator_tx1: MiluvProcessedTimestampNs,
    pub responder_rx1: MiluvProcessedTimestampNs,
    pub responder_tx2: MiluvProcessedTimestampNs,
    pub initiator_rx2: MiluvProcessedTimestampNs,
    pub responder_tx3: MiluvProcessedTimestampNs,
    pub initiator_rx3: MiluvProcessedTimestampNs,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DsTwrDurationInput {
    /// A = R2 - T1 in the published range equation, seconds.
    pub round_trip_term_s: f64,
    pub round_trip_abs_error_bound_s: f64,
    /// B = R3 - R2 in the published range equation, seconds.
    pub clock_scaled_response_term_s: f64,
    pub clock_scaled_response_abs_error_bound_s: f64,
    /// C = T3 - T2 in the published range equation, seconds.
    pub clock_scale_reference_term_s: f64,
    pub clock_scale_reference_abs_error_bound_s: f64,
    /// D = T2 - R1 in the published range equation, seconds.
    pub turnaround_term_s: f64,
    pub turnaround_abs_error_bound_s: f64,
    /// Explicit signal propagation speed selected for this reproduction/physical model.
    pub propagation_speed_mps: f64,
}

impl DsTwrDurationInput {
    /// Construct the published A/B/C/D durations from processed MILUV CSV
    /// timestamps. These inputs are already unwrapped nanoseconds: this method
    /// deliberately performs neither device-tick conversion nor rollover repair.
    ///
    /// Per-duration error bounds are sums of the endpoint hard bounds. This
    /// box bound is conservative and intentionally does not exploit correlation
    /// from shared timestamp endpoints.
    pub fn from_miluv_processed_unwrapped_ns(
        timestamps: MiluvProcessedUnwrappedTimestampsNs,
        propagation_speed_mps: f64,
    ) -> Result<Self, DsTwrRangeError> {
        if timestamps.initiator_id == timestamps.responder_id {
            return Err(DsTwrRangeError::InvalidRadioIdentity);
        }

        let events = [
            timestamps.initiator_tx1,
            timestamps.responder_rx1,
            timestamps.responder_tx2,
            timestamps.initiator_rx2,
            timestamps.responder_tx3,
            timestamps.initiator_rx3,
        ];
        if !events.iter().all(|event| {
            event.value_ns.is_finite()
                && event.value_ns >= 0.0
                && event.abs_error_bound_ns.is_finite()
                && event.abs_error_bound_ns >= 0.0
        }) {
            return Err(DsTwrRangeError::InvalidTimestampInput);
        }

        let (a_s, a_error_s) =
            processed_timestamp_difference_s(timestamps.initiator_rx2, timestamps.initiator_tx1)?;
        let (b_s, b_error_s) =
            processed_timestamp_difference_s(timestamps.initiator_rx3, timestamps.initiator_rx2)?;
        let (c_s, c_error_s) =
            processed_timestamp_difference_s(timestamps.responder_tx3, timestamps.responder_tx2)?;
        let (d_s, d_error_s) =
            processed_timestamp_difference_s(timestamps.responder_tx2, timestamps.responder_rx1)?;

        Ok(Self {
            round_trip_term_s: a_s,
            round_trip_abs_error_bound_s: a_error_s,
            clock_scaled_response_term_s: b_s,
            clock_scaled_response_abs_error_bound_s: b_error_s,
            clock_scale_reference_term_s: c_s,
            clock_scale_reference_abs_error_bound_s: c_error_s,
            turnaround_term_s: d_s,
            turnaround_abs_error_bound_s: d_error_s,
            propagation_speed_mps,
        })
    }
}

/// Convert a pair of already-unwrapped nanosecond timestamps into a duration
/// and conservative endpoint-error bound in seconds.
fn processed_timestamp_difference_s(
    later: MiluvProcessedTimestampNs,
    earlier: MiluvProcessedTimestampNs,
) -> Result<(f64, f64), DsTwrRangeError> {
    let centre_s = (later.value_ns - earlier.value_ns) * 1.0e-9;
    let abs_error_bound_s =
        (later.abs_error_bound_ns + earlier.abs_error_bound_ns) * 1.0e-9;
    if !centre_s.is_finite() || !abs_error_bound_s.is_finite() {
        return Err(DsTwrRangeError::InvalidTimestampInput);
    }
    Ok((centre_s, abs_error_bound_s))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DsTwrRangeInterval {
    pub nominal_range_m: f64,
    pub lower_bound_m: f64,
    pub upper_bound_m: f64,
    pub clock_ratio_nominal: f64,
    pub clock_ratio_lower: f64,
    pub clock_ratio_upper: f64,
    pub time_of_flight_nominal_s: f64,
    pub time_of_flight_lower_bound_s: f64,
    pub time_of_flight_upper_bound_s: f64,
}

/// A qualified bound on nonnegative excess path length for an NLOS range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PositiveNlosBiasBound {
    pub minimum_bias_m: f64,
    pub maximum_bias_m: f64,
}

/// Possible true range after accounting for a separate positive NLOS-bias interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiasCorrectedRangeInterval {
    pub nominal_corrected_range_m: f64,
    pub lower_bound_m: f64,
    pub upper_bound_m: f64,
    pub applied_bias_lower_bound_m: f64,
    pub applied_bias_upper_bound_m: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DsTwrRangeError {
    NonFiniteOrNegativeInput,
    InvalidTimestampInput,
    InvalidRadioIdentity,
    InvalidPropagationSpeed,
    DurationIntervalNotStrictlyPositive(&'static str),
    NonPhysicalRangeInterval,
    InvalidNlosBiasBound,
    NonFiniteOutput,
}

impl fmt::Display for DsTwrRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteOrNegativeInput => write!(f, "DS-TWR duration centres must be finite and positive and error bounds finite and nonnegative"),
            Self::InvalidTimestampInput => write!(f, "processed MILUV timestamps and absolute error bounds must be finite and nonnegative"),
            Self::InvalidRadioIdentity => write!(f, "MILUV initiator and responder IDs must be different"),
            Self::InvalidPropagationSpeed => write!(f, "propagation speed must be finite and strictly positive"),
            Self::DurationIntervalNotStrictlyPositive(name) => write!(f, "DS-TWR duration interval must remain strictly positive: {name}"),
            Self::NonPhysicalRangeInterval => write!(f, "DS-TWR time-of-flight interval contains no physically admissible nonnegative range"),
            Self::InvalidNlosBiasBound => write!(f, "positive NLOS bias interval must be finite, nonnegative and ordered"),
            Self::NonFiniteOutput => write!(f, "DS-TWR interval arithmetic produced a non-finite result"),
        }
    }
}
impl std::error::Error for DsTwrRangeError {}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PositiveInterval {
    lower: f64,
    upper: f64,
}

fn positive_interval(
    centre: f64,
    abs_error: f64,
    name: &'static str,
) -> Result<PositiveInterval, DsTwrRangeError> {
    if !centre.is_finite() || centre <= 0.0 || !abs_error.is_finite() || abs_error < 0.0 {
        return Err(DsTwrRangeError::NonFiniteOrNegativeInput);
    }
    let lower = centre - abs_error;
    let upper = centre + abs_error;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(DsTwrRangeError::NonFiniteOutput);
    }
    if lower <= 0.0 {
        return Err(DsTwrRangeError::DurationIntervalNotStrictlyPositive(name));
    }
    Ok(PositiveInterval { lower, upper })
}

/// Compute the nominal DS-TWR range and a hard interval under bounded
/// uncertainty in the four published duration terms.
///
/// The paper's formula is:
///
/// `y = c/2 * (A - (B/C) * D)`
///
/// with A = R2-T1, B = R3-R2, C = T3-T2, and D = T2-R1.
///
/// The input is deliberately expressed as four duration terms, not six raw
/// timestamp fields. The MILUV paper states that timestamps are held in the
/// respective transceiver clocks; the caller must verify which raw tx/rx
/// values form each duration under the exact protocol/version and correctly
/// unwrap device counter rollovers before calling this helper.
///
/// For positive intervals A, B, C, D, the clock-ratio interval is
/// `[B_low/C_high, B_high/C_low]`; multiplication by D and subtraction from
/// A are propagated conservatively. The returned lower distance is clipped
/// at zero because physical range is nonnegative. If even the upper time-of-
/// flight bound is negative, the transaction is rejected.
pub fn ds_twr_range_interval(
    input: DsTwrDurationInput,
) -> Result<DsTwrRangeInterval, DsTwrRangeError> {
    if !input.propagation_speed_mps.is_finite() || input.propagation_speed_mps <= 0.0 {
        return Err(DsTwrRangeError::InvalidPropagationSpeed);
    }
    let a = positive_interval(input.round_trip_term_s, input.round_trip_abs_error_bound_s, "A=R2-T1")?;
    let b = positive_interval(input.clock_scaled_response_term_s, input.clock_scaled_response_abs_error_bound_s, "B=R3-R2")?;
    let c = positive_interval(input.clock_scale_reference_term_s, input.clock_scale_reference_abs_error_bound_s, "C=T3-T2")?;
    let d = positive_interval(input.turnaround_term_s, input.turnaround_abs_error_bound_s, "D=T2-R1")?;

    let ratio_nominal = input.clock_scaled_response_term_s / input.clock_scale_reference_term_s;
    let ratio_lower = b.lower / c.upper;
    let ratio_upper = b.upper / c.lower;
    let product_nominal = ratio_nominal * input.turnaround_term_s;
    let product_lower = ratio_lower * d.lower;
    let product_upper = ratio_upper * d.upper;
    let tof_nominal = input.round_trip_term_s - product_nominal;
    let tof_lower = a.lower - product_upper;
    let tof_upper = a.upper - product_lower;

    if ![ratio_nominal, ratio_lower, ratio_upper, product_nominal, product_lower, product_upper, tof_nominal, tof_lower, tof_upper]
        .iter().all(|value| value.is_finite())
    {
        return Err(DsTwrRangeError::NonFiniteOutput);
    }
    if tof_nominal < 0.0 || tof_upper < 0.0 {
        return Err(DsTwrRangeError::NonPhysicalRangeInterval);
    }

    let scale = 0.5 * input.propagation_speed_mps;
    let output = DsTwrRangeInterval {
        nominal_range_m: scale * tof_nominal,
        lower_bound_m: scale * tof_lower.max(0.0),
        upper_bound_m: scale * tof_upper,
        clock_ratio_nominal: ratio_nominal,
        clock_ratio_lower: ratio_lower,
        clock_ratio_upper: ratio_upper,
        time_of_flight_nominal_s: tof_nominal,
        time_of_flight_lower_bound_s: tof_lower.max(0.0),
        time_of_flight_upper_bound_s: tof_upper,
    };
    let outputs = [output.nominal_range_m, output.lower_bound_m, output.upper_bound_m];
    if !outputs.iter().all(|value| value.is_finite() && *value >= 0.0)
        || output.lower_bound_m > output.upper_bound_m
    {
        return Err(DsTwrRangeError::NonFiniteOutput);
    }
    Ok(output)
}

/// Convert an observed range interval into a possible true-range interval
/// using an explicit, nonnegative NLOS excess-path bias bound.
///
/// Measurement model: observed_range = true_range + bias, with bias in
/// [minimum_bias_m, maximum_bias_m]. Therefore true_range is bounded by
/// [observed.lower - bias.upper, observed.upper - bias.lower], intersected
/// with the physical range domain [0, +infinity).
///
/// This is not an NLOS classifier or bias estimator. The caller must obtain
/// and qualify the bias interval using independent calibration/validation
/// evidence. Do not use this positive-only model for unclassified errors that
/// may be negative.
pub fn correct_positive_nlos_bias_interval(
    observed: DsTwrRangeInterval,
    bias: PositiveNlosBiasBound,
) -> Result<BiasCorrectedRangeInterval, DsTwrRangeError> {
    if !bias.minimum_bias_m.is_finite()
        || !bias.maximum_bias_m.is_finite()
        || bias.minimum_bias_m < 0.0
        || bias.maximum_bias_m < bias.minimum_bias_m
    {
        return Err(DsTwrRangeError::InvalidNlosBiasBound);
    }
    let observed_values = [
        observed.nominal_range_m,
        observed.lower_bound_m,
        observed.upper_bound_m,
    ];
    if !observed_values.iter().all(|value| value.is_finite() && *value >= 0.0)
        || observed.lower_bound_m > observed.upper_bound_m
    {
        return Err(DsTwrRangeError::NonFiniteOutput);
    }
    let lower = (observed.lower_bound_m - bias.maximum_bias_m).max(0.0);
    let upper = observed.upper_bound_m - bias.minimum_bias_m;
    if !upper.is_finite() || upper < 0.0 || lower > upper {
        return Err(DsTwrRangeError::NonPhysicalRangeInterval);
    }
    let nominal = (observed.nominal_range_m
        - 0.5 * (bias.minimum_bias_m + bias.maximum_bias_m))
        .clamp(lower, upper);
    let result = BiasCorrectedRangeInterval {
        nominal_corrected_range_m: nominal,
        lower_bound_m: lower,
        upper_bound_m: upper,
        applied_bias_lower_bound_m: bias.minimum_bias_m,
        applied_bias_upper_bound_m: bias.maximum_bias_m,
    };
    if ![
        result.nominal_corrected_range_m,
        result.lower_bound_m,
        result.upper_bound_m,
    ]
    .iter()
    .all(|value| value.is_finite() && *value >= 0.0)
    {
        return Err(DsTwrRangeError::NonFiniteOutput);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn input(error: f64) -> DsTwrDurationInput {
        DsTwrDurationInput {
            round_trip_term_s: 300.0e-9,
            round_trip_abs_error_bound_s: error,
            clock_scaled_response_term_s: 300.0e-9,
            clock_scaled_response_abs_error_bound_s: error,
            clock_scale_reference_term_s: 300.0e-9,
            clock_scale_reference_abs_error_bound_s: error,
            turnaround_term_s: 100.0e-9,
            turnaround_abs_error_bound_s: error,
            propagation_speed_mps: VACUUM_PROPAGATION_SPEED_MPS,
        }
    }

    #[test]
    fn nominal_ds_twr_equation_matches_published_algebra() {
        let result = ds_twr_range_interval(input(0.0)).unwrap();
        assert!((result.clock_ratio_nominal - 1.0).abs() < 1.0e-12);
        assert!((result.time_of_flight_nominal_s - 200.0e-9).abs() < 1.0e-18);
        assert!((result.nominal_range_m - 29.979_245_8).abs() < 1.0e-9);
        assert_eq!(result.lower_bound_m, result.upper_bound_m);
    }

    #[test]
    fn bounded_timestamp_error_contains_nominal_range_and_widens_interval() {
        let exact = ds_twr_range_interval(input(0.0)).unwrap();
        let uncertain = ds_twr_range_interval(input(1.0e-9)).unwrap();
        assert!(uncertain.lower_bound_m <= exact.nominal_range_m);
        assert!(uncertain.upper_bound_m >= exact.nominal_range_m);
        assert!(uncertain.lower_bound_m < uncertain.nominal_range_m);
        assert!(uncertain.upper_bound_m > uncertain.nominal_range_m);
        assert!(uncertain.upper_bound_m - uncertain.lower_bound_m > 0.0);
    }

    #[test]
    fn increased_timestamp_error_never_narrows_the_range_interval() {
        let small = ds_twr_range_interval(input(0.1e-9)).unwrap();
        let large = ds_twr_range_interval(input(1.0e-9)).unwrap();
        assert!(large.lower_bound_m <= small.lower_bound_m);
        assert!(large.upper_bound_m >= small.upper_bound_m);
    }

    #[test]
    fn ratio_interval_contains_nominal_clock_ratio() {
        let result = ds_twr_range_interval(input(1.0e-9)).unwrap();
        assert!(result.clock_ratio_lower <= result.clock_ratio_nominal);
        assert!(result.clock_ratio_upper >= result.clock_ratio_nominal);
    }

    #[test]
    fn positive_nlos_bias_bound_widens_true_range_to_the_near_side() {
        let observed = ds_twr_range_interval(input(1.0e-9)).unwrap();
        let corrected = correct_positive_nlos_bias_interval(
            observed,
            PositiveNlosBiasBound { minimum_bias_m: 0.0, maximum_bias_m: 2.0 },
        ).unwrap();
        assert!((corrected.lower_bound_m - (observed.lower_bound_m - 2.0).max(0.0)).abs() < 1.0e-12);
        assert_eq!(corrected.upper_bound_m, observed.upper_bound_m);
        assert!(corrected.lower_bound_m <= corrected.nominal_corrected_range_m);
        assert!(corrected.nominal_corrected_range_m <= corrected.upper_bound_m);
    }

    #[test]
    fn known_minimum_nlos_bias_shifts_both_true_range_bounds() {
        let observed = ds_twr_range_interval(input(0.0)).unwrap();
        let corrected = correct_positive_nlos_bias_interval(
            observed,
            PositiveNlosBiasBound { minimum_bias_m: 0.5, maximum_bias_m: 1.0 },
        ).unwrap();
        assert!((corrected.lower_bound_m - 28.9792458).abs() < 1.0e-8);
        assert!((corrected.upper_bound_m - 29.4792458).abs() < 1.0e-8);
    }

    #[test]
    fn invalid_or_nonphysical_nlos_bias_intervals_fail_closed() {
        let observed = ds_twr_range_interval(input(0.0)).unwrap();
        assert_eq!(
            correct_positive_nlos_bias_interval(observed, PositiveNlosBiasBound { minimum_bias_m: -0.1, maximum_bias_m: 1.0 }),
            Err(DsTwrRangeError::InvalidNlosBiasBound)
        );
        assert_eq!(
            correct_positive_nlos_bias_interval(observed, PositiveNlosBiasBound { minimum_bias_m: 2.0, maximum_bias_m: 1.0 }),
            Err(DsTwrRangeError::InvalidNlosBiasBound)
        );
        assert_eq!(
            correct_positive_nlos_bias_interval(observed, PositiveNlosBiasBound { minimum_bias_m: 50.0, maximum_bias_m: 60.0 }),
            Err(DsTwrRangeError::NonPhysicalRangeInterval)
        );
    }
    #[test]
    fn duration_interval_touching_zero_fails_closed() {
        let mut bad = input(0.0);
        bad.clock_scale_reference_abs_error_bound_s = bad.clock_scale_reference_term_s;
        assert_eq!(
            ds_twr_range_interval(bad),
            Err(DsTwrRangeError::DurationIntervalNotStrictlyPositive("C=T3-T2"))
        );
    }

    #[test]
    fn malformed_non_finite_or_negative_durations_fail_closed() {
        let mut bad = input(0.0);
        bad.round_trip_term_s = f64::NAN;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::NonFiniteOrNegativeInput));
        let mut bad = input(0.0);
        bad.turnaround_abs_error_bound_s = -1.0;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::NonFiniteOrNegativeInput));
    }

    #[test]
    fn physically_impossible_negative_time_of_flight_fails_closed() {
        let mut bad = input(0.0);
        bad.round_trip_term_s = 50.0e-9;
        bad.clock_scaled_response_term_s = 300.0e-9;
        bad.clock_scale_reference_term_s = 100.0e-9;
        bad.turnaround_term_s = 100.0e-9;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::NonPhysicalRangeInterval));
    }



    fn processed_event(value_ns: f64, abs_error_bound_ns: f64) -> MiluvProcessedTimestampNs {
        MiluvProcessedTimestampNs {
            value_ns,
            abs_error_bound_ns,
        }
    }

    fn processed_miluv_timestamps() -> MiluvProcessedUnwrappedTimestampsNs {
        MiluvProcessedUnwrappedTimestampsNs {
            initiator_id: 10,
            responder_id: 20,
            initiator_tx1: processed_event(1_000.0, 0.25),
            responder_rx1: processed_event(2_000.0, 0.25),
            responder_tx2: processed_event(2_100.0, 0.25),
            initiator_rx2: processed_event(1_300.0, 0.25),
            responder_tx3: processed_event(2_200.0, 0.25),
            initiator_rx3: processed_event(1_400.0, 0.25),
        }
    }

    #[test]
    fn processed_miluv_adapter_maps_clock_local_events_and_ns_units() {
        let input = DsTwrDurationInput::from_miluv_processed_unwrapped_ns(
            processed_miluv_timestamps(),
            MILUV_REFERENCE_PROPAGATION_SPEED_MPS,
        )
        .unwrap();
        assert!((input.round_trip_term_s - 300.0e-9).abs() < 1.0e-18);
        assert!((input.clock_scaled_response_term_s - 100.0e-9).abs() < 1.0e-18);
        assert!((input.clock_scale_reference_term_s - 100.0e-9).abs() < 1.0e-18);
        assert!((input.turnaround_term_s - 100.0e-9).abs() < 1.0e-18);
        assert!((input.round_trip_abs_error_bound_s - 0.5e-9).abs() < 1.0e-18);
        assert!((input.clock_scaled_response_abs_error_bound_s - 0.5e-9).abs() < 1.0e-18);
        assert!((input.clock_scale_reference_abs_error_bound_s - 0.5e-9).abs() < 1.0e-18);
        assert!((input.turnaround_abs_error_bound_s - 0.5e-9).abs() < 1.0e-18);
    }

    #[test]
    fn processed_miluv_adapter_produces_reference_range_without_second_unwrap() {
        let input = DsTwrDurationInput::from_miluv_processed_unwrapped_ns(
            processed_miluv_timestamps(),
            MILUV_REFERENCE_PROPAGATION_SPEED_MPS,
        )
        .unwrap();
        let result = ds_twr_range_interval(input).unwrap();
        assert!((result.nominal_range_m - 29.970_254_7).abs() < 1.0e-9);
    }

    #[test]
    fn propagation_speed_is_explicit_and_changes_only_range_scale() {
        let mut air = input(0.0);
        air.propagation_speed_mps = MILUV_REFERENCE_PROPAGATION_SPEED_MPS;
        let vacuum = input(0.0);
        let air_result = ds_twr_range_interval(air).unwrap();
        let vacuum_result = ds_twr_range_interval(vacuum).unwrap();
        assert!((air_result.nominal_range_m - 29.970_254_7).abs() < 1.0e-9);
        assert!((vacuum_result.nominal_range_m - 29.979_245_8).abs() < 1.0e-9);
        assert!(air_result.lower_bound_m < vacuum_result.lower_bound_m);
        assert!(air_result.upper_bound_m < vacuum_result.upper_bound_m);
    }

    #[test]
    fn invalid_propagation_speed_fails_closed() {
        let mut bad = input(0.0);
        bad.propagation_speed_mps = 0.0;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::InvalidPropagationSpeed));
        let mut bad = input(0.0);
        bad.propagation_speed_mps = f64::NAN;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::InvalidPropagationSpeed));
    }

    #[test]
    fn processed_miluv_adapter_rejects_same_radio_identity() {
        let mut timestamps = processed_miluv_timestamps();
        timestamps.responder_id = timestamps.initiator_id;
        assert_eq!(
            DsTwrDurationInput::from_miluv_processed_unwrapped_ns(
                timestamps,
                MILUV_REFERENCE_PROPAGATION_SPEED_MPS,
            ),
            Err(DsTwrRangeError::InvalidRadioIdentity)
        );
    }

    #[test]
    fn processed_miluv_adapter_rejects_invalid_timestamp_or_error_bound() {
        let mut timestamps = processed_miluv_timestamps();
        timestamps.initiator_rx2.value_ns = f64::NAN;
        assert_eq!(
            DsTwrDurationInput::from_miluv_processed_unwrapped_ns(
                timestamps,
                MILUV_REFERENCE_PROPAGATION_SPEED_MPS,
            ),
            Err(DsTwrRangeError::InvalidTimestampInput)
        );
        let mut timestamps = processed_miluv_timestamps();
        timestamps.responder_tx2.abs_error_bound_ns = -0.1;
        assert_eq!(
            DsTwrDurationInput::from_miluv_processed_unwrapped_ns(
                timestamps,
                MILUV_REFERENCE_PROPAGATION_SPEED_MPS,
            ),
            Err(DsTwrRangeError::InvalidTimestampInput)
        );
    }

    #[test]
    fn non_finite_interval_arithmetic_fails_closed() {
        let mut bad = input(0.0);
        bad.round_trip_abs_error_bound_s = f64::MAX;
        assert_eq!(ds_twr_range_interval(bad), Err(DsTwrRangeError::DurationIntervalNotStrictlyPositive("A=R2-T1")));
    }
}
