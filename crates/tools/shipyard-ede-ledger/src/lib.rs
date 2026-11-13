//! Synthetic-only tandem-GMAW electrical deposition efficiency ledger.
//!
//! This crate calculates an EDE estimate from independently sampled leading
//! and trailing wire power/feed streams. It does not qualify source sensors,
//! admit physical BAM data, measure retained filler mass, or establish weld
//! acceptance. The discrete integration and uncertainty rules are explicit.

use std::f64::consts::PI;

const JOULES_PER_KWH: f64 = 3_600_000.0;
const GRAMS_PER_KG: f64 = 1_000.0;
const NANOS_PER_SECOND: f64 = 1_000_000_000.0;
const PPM_SCALE: f64 = 1_000_000.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerSample {
    pub timestamp_ns: u64,
    pub voltage_v: f64,
    pub current_a: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WireFeedSample {
    pub timestamp_ns: u64,
    pub speed_m_per_s: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelativeUncertaintyPpm {
    pub voltage: u64,
    pub current: u64,
    pub wire_feed: u64,
    pub density: u64,
    pub wire_diameter: u64,
    pub power_quadrature: u64,
    pub feed_quadrature: u64,
    pub clock_alignment: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EdeInput {
    pub workpiece_id: String,
    pub pass_id: String,
    pub start_time_ns: u64,
    pub end_time_ns: u64,
    pub maximum_sample_gap_ns: u64,
    pub weld_length_m: f64,
    pub density_kg_per_m3: f64,
    pub wire_diameter_m: f64,
    pub source_trace_sha256: [u8; 32],
    pub alignment_receipt_sha256: [u8; 32],
    pub sampling_qualification_receipt_sha256: [u8; 32],
    pub leading_power: Vec<PowerSample>,
    pub trailing_power: Vec<PowerSample>,
    pub leading_wire_feed: Vec<WireFeedSample>,
    pub trailing_wire_feed: Vec<WireFeedSample>,
    pub uncertainty: RelativeUncertaintyPpm,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdeError {
    EmptyIdentity,
    InvalidTimeWindow,
    InvalidMaximumSampleGap,
    MissingSourceTraceDigest,
    MissingAlignmentReceiptDigest,
    MissingSamplingQualificationReceipt,
    InvalidWeldLength,
    InvalidDensity,
    InvalidWireDiameter,
    TooFewSamples,
    WrongWindowStart,
    WrongWindowEnd,
    NonMonotonicTimestamp,
    SampleGapTooLarge,
    NonFiniteVoltage,
    NonFiniteCurrent,
    NegativeVoltage,
    NegativeCurrent,
    NonFiniteWireFeed,
    NegativeWireFeed,
    NonFiniteIntegration,
    ZeroArcEnergy,
    ZeroWireFeed,
    InvalidMassUncertainty,
    InvalidEnergyUncertainty,
    UncertaintySumOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StreamIntegration {
    pub integral: f64,
    pub sample_count: usize,
    pub maximum_observed_gap_ns: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdeResult {
    pub leading_arc_energy_j: f64,
    pub trailing_arc_energy_j: f64,
    pub total_arc_energy_j: f64,
    pub leading_wire_length_m: f64,
    pub trailing_wire_length_m: f64,
    pub leading_estimated_mass_g: f64,
    pub trailing_estimated_mass_g: f64,
    pub total_estimated_mass_g: f64,
    pub energy_kwh: f64,
    pub ede_g_per_kwh: f64,
    pub ede_lower_g_per_kwh: f64,
    pub ede_upper_g_per_kwh: f64,
    pub energy_per_length_kj_per_mm: f64,
    pub duration_s: f64,
    pub leading_power_sample_count: usize,
    pub trailing_power_sample_count: usize,
    pub leading_feed_sample_count: usize,
    pub trailing_feed_sample_count: usize,
    pub maximum_observed_gap_ns: u64,
    pub relative_mass_uncertainty: f64,
    pub relative_energy_uncertainty: f64,
    pub claim_ceiling: &'static str,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn digest_is_zero(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
}

fn add_ppm(values: &[u64]) -> Result<u64, EdeError> {
    values.iter().try_fold(0_u64, |sum, value| {
        sum.checked_add(*value)
            .ok_or(EdeError::UncertaintySumOverflow)
    })
}

fn validate_times(
    timestamps: &[u64],
    start_time_ns: u64,
    end_time_ns: u64,
    maximum_sample_gap_ns: u64,
) -> Result<u64, EdeError> {
    if timestamps.len() < 2 {
        return Err(EdeError::TooFewSamples);
    }
    if timestamps[0] != start_time_ns {
        return Err(EdeError::WrongWindowStart);
    }
    if timestamps[timestamps.len() - 1] != end_time_ns {
        return Err(EdeError::WrongWindowEnd);
    }

    let mut maximum_gap = 0_u64;
    for pair in timestamps.windows(2) {
        if pair[1] <= pair[0] {
            return Err(EdeError::NonMonotonicTimestamp);
        }
        let gap = pair[1] - pair[0];
        if gap > maximum_sample_gap_ns {
            return Err(EdeError::SampleGapTooLarge);
        }
        maximum_gap = maximum_gap.max(gap);
    }
    Ok(maximum_gap)
}

fn integrate_power(
    samples: &[PowerSample],
    start_time_ns: u64,
    end_time_ns: u64,
    maximum_sample_gap_ns: u64,
) -> Result<StreamIntegration, EdeError> {
    let timestamps: Vec<u64> = samples.iter().map(|sample| sample.timestamp_ns).collect();
    let maximum_gap = validate_times(
        &timestamps,
        start_time_ns,
        end_time_ns,
        maximum_sample_gap_ns,
    )?;

    let mut energy_j = 0.0;
    for pair in samples.windows(2) {
        for sample in pair {
            if !sample.voltage_v.is_finite() {
                return Err(EdeError::NonFiniteVoltage);
            }
            if !sample.current_a.is_finite() {
                return Err(EdeError::NonFiniteCurrent);
            }
            if sample.voltage_v < 0.0 {
                return Err(EdeError::NegativeVoltage);
            }
            if sample.current_a < 0.0 {
                return Err(EdeError::NegativeCurrent);
            }
        }

        // The numerical policy trapezoid-integrates sampled instantaneous
        // power products. It does not multiply separately averaged V and I.
        let left_power_w = pair[0].voltage_v * pair[0].current_a;
        let right_power_w = pair[1].voltage_v * pair[1].current_a;
        let delta_s = (pair[1].timestamp_ns - pair[0].timestamp_ns) as f64
            / NANOS_PER_SECOND;
        energy_j += (left_power_w + right_power_w) * 0.5 * delta_s;
    }
    if !energy_j.is_finite() {
        return Err(EdeError::NonFiniteIntegration);
    }

    Ok(StreamIntegration {
        integral: energy_j,
        sample_count: samples.len(),
        maximum_observed_gap_ns: maximum_gap,
    })
}

fn integrate_wire_feed(
    samples: &[WireFeedSample],
    start_time_ns: u64,
    end_time_ns: u64,
    maximum_sample_gap_ns: u64,
) -> Result<StreamIntegration, EdeError> {
    let timestamps: Vec<u64> = samples.iter().map(|sample| sample.timestamp_ns).collect();
    let maximum_gap = validate_times(
        &timestamps,
        start_time_ns,
        end_time_ns,
        maximum_sample_gap_ns,
    )?;

    let mut wire_length_m = 0.0;
    for pair in samples.windows(2) {
        for sample in pair {
            if !sample.speed_m_per_s.is_finite() {
                return Err(EdeError::NonFiniteWireFeed);
            }
            if sample.speed_m_per_s < 0.0 {
                return Err(EdeError::NegativeWireFeed);
            }
        }
        let delta_s = (pair[1].timestamp_ns - pair[0].timestamp_ns) as f64
            / NANOS_PER_SECOND;
        wire_length_m +=
            (pair[0].speed_m_per_s + pair[1].speed_m_per_s) * 0.5 * delta_s;
    }
    if !wire_length_m.is_finite() {
        return Err(EdeError::NonFiniteIntegration);
    }

    Ok(StreamIntegration {
        integral: wire_length_m,
        sample_count: samples.len(),
        maximum_observed_gap_ns: maximum_gap,
    })
}

fn validate_input(input: &EdeInput) -> Result<(), EdeError> {
    if is_empty(&input.workpiece_id) || is_empty(&input.pass_id) {
        return Err(EdeError::EmptyIdentity);
    }
    if input.end_time_ns <= input.start_time_ns {
        return Err(EdeError::InvalidTimeWindow);
    }
    if input.maximum_sample_gap_ns == 0 {
        return Err(EdeError::InvalidMaximumSampleGap);
    }
    if digest_is_zero(&input.source_trace_sha256) {
        return Err(EdeError::MissingSourceTraceDigest);
    }
    if digest_is_zero(&input.alignment_receipt_sha256) {
        return Err(EdeError::MissingAlignmentReceiptDigest);
    }
    if digest_is_zero(&input.sampling_qualification_receipt_sha256) {
        return Err(EdeError::MissingSamplingQualificationReceipt);
    }
    if !input.weld_length_m.is_finite() || input.weld_length_m <= 0.0 {
        return Err(EdeError::InvalidWeldLength);
    }
    if !input.density_kg_per_m3.is_finite()
        || !(1_000.0..=30_000.0).contains(&input.density_kg_per_m3)
    {
        return Err(EdeError::InvalidDensity);
    }
    if !input.wire_diameter_m.is_finite()
        || !(0.0001..=0.02).contains(&input.wire_diameter_m)
    {
        return Err(EdeError::InvalidWireDiameter);
    }
    Ok(())
}

/// Integrate the two independently sampled wire-power and wire-feed streams.
///
/// Every stream must span the exact same declared window. Each stream keeps
/// its own native timestamps and is integrated independently. Any excessive
/// gap, missing endpoint, malformed value, or missing source/alignment digest
/// rejects the calculation. Output is an estimated filler-wire mass per
/// electrical kWh, not measured retained deposit mass or weld acceptance.
pub fn calculate_ede(input: &EdeInput) -> Result<EdeResult, EdeError> {
    validate_input(input)?;

    let leading_power = integrate_power(
        &input.leading_power,
        input.start_time_ns,
        input.end_time_ns,
        input.maximum_sample_gap_ns,
    )?;
    let trailing_power = integrate_power(
        &input.trailing_power,
        input.start_time_ns,
        input.end_time_ns,
        input.maximum_sample_gap_ns,
    )?;
    let leading_feed = integrate_wire_feed(
        &input.leading_wire_feed,
        input.start_time_ns,
        input.end_time_ns,
        input.maximum_sample_gap_ns,
    )?;
    let trailing_feed = integrate_wire_feed(
        &input.trailing_wire_feed,
        input.start_time_ns,
        input.end_time_ns,
        input.maximum_sample_gap_ns,
    )?;

    let total_energy_j = leading_power.integral + trailing_power.integral;
    if !total_energy_j.is_finite() {
        return Err(EdeError::NonFiniteIntegration);
    }
    if total_energy_j <= 0.0 {
        return Err(EdeError::ZeroArcEnergy);
    }
    if leading_feed.integral + trailing_feed.integral <= 0.0 {
        return Err(EdeError::ZeroWireFeed);
    }

    let cross_section_area_m2 =
        PI * input.wire_diameter_m * input.wire_diameter_m * 0.25;
    let grams_per_m = input.density_kg_per_m3 * cross_section_area_m2 * GRAMS_PER_KG;
    let leading_mass_g = grams_per_m * leading_feed.integral;
    let trailing_mass_g = grams_per_m * trailing_feed.integral;
    let total_mass_g = leading_mass_g + trailing_mass_g;
    let energy_kwh = total_energy_j / JOULES_PER_KWH;
    let ede = total_mass_g / energy_kwh;
    let duration_s =
        (input.end_time_ns - input.start_time_ns) as f64 / NANOS_PER_SECOND;
    let energy_per_length = total_energy_j / 1_000_000.0 / input.weld_length_m;

    if !cross_section_area_m2.is_finite()
        || !grams_per_m.is_finite()
        || !total_mass_g.is_finite()
        || !energy_kwh.is_finite()
        || !ede.is_finite()
        || !duration_s.is_finite()
        || !energy_per_length.is_finite()
    {
        return Err(EdeError::NonFiniteIntegration);
    }

    let uncertainty = input.uncertainty;
    let mass_uncertainty_ppm = add_ppm(&[
        uncertainty.density,
        uncertainty.diameter.checked_mul(2).ok_or(EdeError::UncertaintySumOverflow)?,
        uncertainty.wire_feed,
        uncertainty.feed_quadrature,
        uncertainty.clock_alignment,
    ])?;
    let energy_uncertainty_ppm = add_ppm(&[
        uncertainty.voltage,
        uncertainty.current,
        uncertainty.power_quadrature,
        uncertainty.clock_alignment,
    ])?;
    if mass_uncertainty_ppm >= 1_000_000 {
        return Err(EdeError::InvalidMassUncertainty);
    }
    if energy_uncertainty_ppm >= 1_000_000 {
        return Err(EdeError::InvalidEnergyUncertainty);
    }

    let mass_uncertainty = mass_uncertainty_ppm as f64 / PPM_SCALE;
    let energy_uncertainty = energy_uncertainty_ppm as f64 / PPM_SCALE;
    let ede_lower = ede * (1.0 - mass_uncertainty) / (1.0 + energy_uncertainty);
    let ede_upper = ede * (1.0 + mass_uncertainty) / (1.0 - energy_uncertainty);
    let maximum_observed_gap = [
        leading_power.maximum_observed_gap_ns,
        trailing_power.maximum_observed_gap_ns,
        leading_feed.maximum_observed_gap_ns,
        trailing_feed.maximum_observed_gap_ns,
    ]
    .into_iter()
    .max()
    .unwrap_or(0);

    if !ede_lower.is_finite() || !ede_upper.is_finite() {
        return Err(EdeError::NonFiniteIntegration);
    }

    Ok(EdeResult {
        leading_arc_energy_j: leading_power.integral,
        trailing_arc_energy_j: trailing_power.integral,
        total_arc_energy_j: total_energy_j,
        leading_wire_length_m: leading_feed.integral,
        trailing_wire_length_m: trailing_feed.integral,
        leading_estimated_mass_g: leading_mass_g,
        trailing_estimated_mass_g: trailing_mass_g,
        total_estimated_mass_g: total_mass_g,
        energy_kwh,
        ede_g_per_kwh: ede,
        ede_lower_g_per_kwh: ede_lower,
        ede_upper_g_per_kwh: ede_upper,
        energy_per_length_kj_per_mm: energy_per_length,
        duration_s,
        leading_power_sample_count: leading_power.sample_count,
        trailing_power_sample_count: trailing_power.sample_count,
        leading_feed_sample_count: leading_feed.sample_count,
        trailing_feed_sample_count: trailing_feed.sample_count,
        maximum_observed_gap_ns: maximum_observed_gap,
        relative_mass_uncertainty: mass_uncertainty,
        relative_energy_uncertainty: energy_uncertainty,
        claim_ceiling: "SYNTHETIC_EDE_CALCULATION_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_input() -> EdeInput {
        let start = 0;
        let middle = 1_000_000_000;
        let end = 2_000_000_000;
        EdeInput {
            workpiece_id: "WID-SYNTH-01".to_owned(),
            pass_id: "PASS-SYNTH-03".to_owned(),
            start_time_ns: start,
            end_time_ns: end,
            maximum_sample_gap_ns: 1_000_000_000,
            weld_length_m: 0.3,
            density_kg_per_m3: 7_850.0,
            wire_diameter_m: 0.0012,
            source_trace_sha256: [0x11; 32],
            alignment_receipt_sha256: [0x22; 32],
            sampling_qualification_receipt_sha256: [0x33; 32],
            leading_power: vec![
                PowerSample { timestamp_ns: start, voltage_v: 30.0, current_a: 200.0 },
                PowerSample { timestamp_ns: middle, voltage_v: 32.0, current_a: 210.0 },
                PowerSample { timestamp_ns: end, voltage_v: 30.0, current_a: 200.0 },
            ],
            trailing_power: vec![
                PowerSample { timestamp_ns: start, voltage_v: 24.0, current_a: 150.0 },
                PowerSample { timestamp_ns: middle, voltage_v: 26.0, current_a: 160.0 },
                PowerSample { timestamp_ns: end, voltage_v: 24.0, current_a: 150.0 },
            ],
            leading_wire_feed: vec![
                WireFeedSample { timestamp_ns: start, speed_m_per_s: 0.19 },
                WireFeedSample { timestamp_ns: middle, speed_m_per_s: 0.21 },
                WireFeedSample { timestamp_ns: end, speed_m_per_s: 0.19 },
            ],
            trailing_wire_feed: vec![
                WireFeedSample { timestamp_ns: start, speed_m_per_s: 0.11 },
                WireFeedSample { timestamp_ns: middle, speed_m_per_s: 0.13 },
                WireFeedSample { timestamp_ns: end, speed_m_per_s: 0.11 },
            ],
            uncertainty: RelativeUncertaintyPpm {
                voltage: 10_000,
                current: 10_000,
                wire_feed: 5_000,
                density: 1_000,
                wire_diameter: 1_000,
                power_quadrature: 2_000,
                feed_quadrature: 1_000,
                clock_alignment: 1_000,
            },
        }
    }

    #[test]
    fn synthetic_known_answer_matches_electrical_deposition_efficiency() {
        let result = calculate_ede(&base_input()).expect("synthetic case is valid");
        assert!((result.leading_arc_energy_j - 12_720.0).abs() < 1e-9);
        assert!((result.trailing_arc_energy_j - 7_760.0).abs() < 1e-9);
        assert!((result.total_arc_energy_j - 20_480.0).abs() < 1e-9);
        assert!((result.leading_wire_length_m - 0.4).abs() < 1e-12);
        assert!((result.trailing_wire_length_m - 0.24).abs() < 1e-12);
        assert!((result.total_estimated_mass_g - 5.682_010_136_988_642).abs() < 1e-10);
        assert!((result.ede_g_per_kwh - 998.790_844_392_534_6).abs() < 1e-9);
        assert!((result.ede_lower_g_per_kwh - 966.571_784_896_001_3).abs() < 1e-9);
        assert!((result.ede_upper_g_per_kwh - 1_032.526_870_866_386_8).abs() < 1e-9);
        assert!((result.energy_per_length_kj_per_mm - 0.068_266_666_666_666_67).abs() < 1e-12);
        assert_eq!(result.claim_ceiling, "SYNTHETIC_EDE_CALCULATION_ONLY");
    }

    #[test]
    fn integration_uses_sampled_voltage_times_current() {
        let result = calculate_ede(&base_input()).expect("synthetic case is valid");
        let mean_v = (30.0 + 32.0 + 30.0) / 3.0;
        let mean_i = (200.0 + 210.0 + 200.0) / 3.0;
        let mean_product = (6_000.0 + 6_720.0 + 6_000.0) / 3.0;
        assert!((mean_v * mean_i - mean_product).abs() > 1.0);
        assert_eq!(result.leading_arc_energy_j, 12_720.0);
    }

    #[test]
    fn leading_and_trailing_streams_are_integrated_separately() {
        let result = calculate_ede(&base_input()).expect("synthetic case is valid");
        assert_eq!(result.leading_arc_energy_j, 12_720.0);
        assert_eq!(result.trailing_arc_energy_j, 7_760.0);
        assert_ne!(result.leading_arc_energy_j, result.trailing_arc_energy_j);
        assert_eq!(result.leading_wire_length_m, 0.4);
        assert_eq!(result.trailing_wire_length_m, 0.24);
    }

    #[test]
    fn irregular_native_cadences_are_not_forced_onto_one_grid() {
        let mut input = base_input();
        input.trailing_power = vec![
            PowerSample { timestamp_ns: 0, voltage_v: 24.0, current_a: 150.0 },
            PowerSample { timestamp_ns: 500_000_000, voltage_v: 26.0, current_a: 160.0 },
            PowerSample { timestamp_ns: 2_000_000_000, voltage_v: 24.0, current_a: 150.0 },
        ];
        input.maximum_sample_gap_ns = 2_000_000_000;
        input.trailing_wire_feed = vec![
            WireFeedSample { timestamp_ns: 0, speed_m_per_s: 0.11 },
            WireFeedSample { timestamp_ns: 250_000_000, speed_m_per_s: 0.12 },
            WireFeedSample { timestamp_ns: 2_000_000_000, speed_m_per_s: 0.11 },
        ];
        let result = calculate_ede(&input).expect("native cadences can differ");
        assert_eq!(result.trailing_power_sample_count, 3);
        assert_eq!(result.trailing_feed_sample_count, 3);
    }

    #[test]
    fn missing_source_trace_digest_is_rejected() {
        let mut input = base_input();
        input.source_trace_sha256 = [0; 32];
        assert_eq!(calculate_ede(&input), Err(EdeError::MissingSourceTraceDigest));
    }

    #[test]
    fn missing_alignment_receipt_digest_is_rejected() {
        let mut input = base_input();
        input.alignment_receipt_sha256 = [0; 32];
        assert_eq!(calculate_ede(&input), Err(EdeError::MissingAlignmentReceiptDigest));
    }

    #[test]
    fn missing_sampling_qualification_receipt_is_rejected() {
        let mut input = base_input();
        input.sampling_qualification_receipt_sha256 = [0; 32];
        assert_eq!(
            calculate_ede(&input),
            Err(EdeError::MissingSamplingQualificationReceipt)
        );
    }

    #[test]
    fn mismatched_stream_window_start_is_rejected() {
        let mut input = base_input();
        input.trailing_wire_feed[0].timestamp_ns = 1;
        assert_eq!(calculate_ede(&input), Err(EdeError::WrongWindowStart));
    }

    #[test]
    fn mismatched_stream_window_end_is_rejected() {
        let mut input = base_input();
        input.trailing_power.pop();
        assert_eq!(calculate_ede(&input), Err(EdeError::WrongWindowEnd));
    }

    #[test]
    fn non_monotonic_timestamp_is_rejected() {
        let mut input = base_input();
        input.leading_power.swap(1, 2);
        assert_eq!(calculate_ede(&input), Err(EdeError::NonMonotonicTimestamp));
    }

    #[test]
    fn duplicate_timestamp_is_rejected() {
        let mut input = base_input();
        input.trailing_power[1].timestamp_ns = input.trailing_power[0].timestamp_ns;
        assert_eq!(calculate_ede(&input), Err(EdeError::NonMonotonicTimestamp));
    }

    #[test]
    fn excessive_sample_gap_is_not_bridged() {
        let mut input = base_input();
        input.maximum_sample_gap_ns = 500_000_000;
        assert_eq!(calculate_ede(&input), Err(EdeError::SampleGapTooLarge));
    }

    #[test]
    fn zero_length_window_is_rejected() {
        let mut input = base_input();
        input.end_time_ns = input.start_time_ns;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidTimeWindow));
    }

    #[test]
    fn zero_maximum_gap_policy_is_rejected() {
        let mut input = base_input();
        input.maximum_sample_gap_ns = 0;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidMaximumSampleGap));
    }

    #[test]
    fn negative_current_is_rejected_without_implicit_absolute_value() {
        let mut input = base_input();
        input.leading_power[1].current_a = -1.0;
        assert_eq!(calculate_ede(&input), Err(EdeError::NegativeCurrent));
    }

    #[test]
    fn non_finite_voltage_is_rejected() {
        let mut input = base_input();
        input.leading_power[1].voltage_v = f64::NAN;
        assert_eq!(calculate_ede(&input), Err(EdeError::NonFiniteVoltage));
    }

    #[test]
    fn negative_wire_feed_is_rejected() {
        let mut input = base_input();
        input.trailing_wire_feed[1].speed_m_per_s = -0.1;
        assert_eq!(calculate_ede(&input), Err(EdeError::NegativeWireFeed));
    }

    #[test]
    fn non_finite_wire_feed_is_rejected() {
        let mut input = base_input();
        input.leading_wire_feed[1].speed_m_per_s = f64::INFINITY;
        assert_eq!(calculate_ede(&input), Err(EdeError::NonFiniteWireFeed));
    }

    #[test]
    fn zero_arc_energy_is_rejected() {
        let mut input = base_input();
        for sample in &mut input.leading_power {
            sample.current_a = 0.0;
        }
        for sample in &mut input.trailing_power {
            sample.current_a = 0.0;
        }
        assert_eq!(calculate_ede(&input), Err(EdeError::ZeroArcEnergy));
    }

    #[test]
    fn zero_wire_feed_is_rejected() {
        let mut input = base_input();
        for sample in &mut input.leading_wire_feed {
            sample.speed_m_per_s = 0.0;
        }
        for sample in &mut input.trailing_wire_feed {
            sample.speed_m_per_s = 0.0;
        }
        assert_eq!(calculate_ede(&input), Err(EdeError::ZeroWireFeed));
    }

    #[test]
    fn density_with_grams_per_cubic_centimetre_units_is_rejected() {
        let mut input = base_input();
        input.density_kg_per_m3 = 7.85;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidDensity));
    }

    #[test]
    fn diameter_with_millimetres_as_metres_is_rejected() {
        let mut input = base_input();
        input.wire_diameter_m = 1.2;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidWireDiameter));
    }

    #[test]
    fn zero_or_unbounded_mass_uncertainty_is_rejected() {
        let mut input = base_input();
        input.uncertainty.diameter = 600_000;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidMassUncertainty));
    }

    #[test]
    fn unbounded_energy_uncertainty_is_rejected() {
        let mut input = base_input();
        input.uncertainty.voltage = 1_000_000;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidEnergyUncertainty));
    }

    #[test]
    fn uncertainty_interval_is_ordered_and_conservative() {
        let result = calculate_ede(&base_input()).expect("synthetic case is valid");
        assert!(result.ede_lower_g_per_kwh < result.ede_g_per_kwh);
        assert!(result.ede_g_per_kwh < result.ede_upper_g_per_kwh);
        assert_eq!(result.relative_mass_uncertainty, 0.01);
        assert_eq!(result.relative_energy_uncertainty, 0.023);
    }

    #[test]
    fn workpiece_and_pass_identity_are_mandatory() {
        let mut input = base_input();
        input.workpiece_id = "  ".to_owned();
        assert_eq!(calculate_ede(&input), Err(EdeError::EmptyIdentity));
        let mut input = base_input();
        input.pass_id.clear();
        assert_eq!(calculate_ede(&input), Err(EdeError::EmptyIdentity));
    }

    #[test]
    fn weld_length_must_be_positive_and_finite() {
        let mut input = base_input();
        input.weld_length_m = 0.0;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidWeldLength));
        let mut input = base_input();
        input.weld_length_m = f64::INFINITY;
        assert_eq!(calculate_ede(&input), Err(EdeError::InvalidWeldLength));
    }
}
