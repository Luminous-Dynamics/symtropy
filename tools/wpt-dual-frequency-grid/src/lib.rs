// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fail-closed validation and point metrics for dual-frequency WPT/grid spectra.
//!
//! This crate validates synchronized spectral line records and computes
//! conventional grid-current THD and resonant sideband-to-carrier RMS ratios.
//! It does not perform an FFT, identify lines from raw waveforms, estimate
//! uncertainty distributions, prove converter stability, or certify hardware.

use serde_json::Value;
use std::collections::BTreeSet;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    let map = object(value, path)?;
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn number_field(value: &Value, key: &str, path: &str) -> Result<f64, String> {
    let number = field(value, key, path)?
        .as_f64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be numeric"))?;
    if !number.is_finite() {
        return Err(fail(&format!("{path}.{key}"), "must be finite"));
    }
    Ok(number)
}

fn integer_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn is_utc_timestamp(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
        || !bytes.iter().enumerate().all(|(index, byte)| {
            matches!(index, 4 | 7 | 10 | 13 | 16 | 19) || byte.is_ascii_digit()
        })
    {
        return false;
    }

    let year = value[0..4].parse::<u16>().ok();
    let month = value[5..7].parse::<u8>().ok();
    let day = value[8..10].parse::<u8>().ok();
    let hour = value[11..13].parse::<u8>().ok();
    let minute = value[14..16].parse::<u8>().ok();
    let second = value[17..19].parse::<u8>().ok();
    let (Some(year), Some(month), Some(day), Some(hour), Some(minute), Some(second)) =
        (year, month, day, hour, minute, second)
    else {
        return false;
    };
    if year == 0 || !(1..=12).contains(&month) || hour > 23 || minute > 59 || second > 59 {
        return false;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    (1..=days_in_month).contains(&day)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMetrics {
    pub channel_id: String,
    pub channel_kind: String,
    /// Maximum integer grid-harmonic order included in the THD calculation.
    pub maximum_harmonic_order: Option<u64>,
    /// THD through the declared maximum integer harmonic order.
    pub thd_through_harmonic_order: Option<f64>,
    /// RMS sum of reported grid interharmonics divided by grid fundamental.
    pub interharmonic_to_fundamental_rms_ratio: Option<f64>,
    /// RMS sum of reported resonant sidebands divided by resonant carrier.
    pub sideband_to_carrier_rms_ratio: Option<f64>,
    pub line_count: usize,
    pub maximum_expanded_line_uncertainty: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunMetrics {
    pub run_id: String,
    pub nyquist_frequency_hz: f64,
    pub native_fourier_resolution_hz: f64,
    pub effective_frequency_resolution_hz: f64,
    pub channels: Vec<ChannelMetrics>,
}

fn validate_quality_flags(run: &Value, evidence_kind: &str) -> Result<(), String> {
    let flags = field(run, "quality_flags", "$")?
        .as_array()
        .ok_or_else(|| fail("$.quality_flags", "must be an array"))?;
    let mut seen = BTreeSet::new();
    let mut synthetic = false;
    for flag in flags {
        let value = flag
            .as_str()
            .ok_or_else(|| fail("$.quality_flags", "each flag must be a string"))?;
        if !seen.insert(value) {
            return Err(fail("$.quality_flags", "duplicate quality flag"));
        }
        if ![
            "SYNTHETIC_ONLY",
            "CLOCK_UNCERTAIN",
            "CALIBRATION_MISSING",
            "RAW_WAVEFORM_MISSING",
            "SPECTRAL_RESOLUTION_LIMITED",
        ]
        .contains(&value)
        {
            return Err(fail(
                "$.quality_flags",
                format!("unknown quality flag {value}"),
            ));
        }
        synthetic |= value == "SYNTHETIC_ONLY";
    }

    if evidence_kind == "SYNTHETIC_KNOWN_TRUTH" && !synthetic {
        return Err(fail(
            "$.quality_flags",
            "synthetic evidence requires SYNTHETIC_ONLY",
        ));
    }
    if evidence_kind != "SYNTHETIC_KNOWN_TRUTH" && synthetic {
        return Err(fail(
            "$.quality_flags",
            "SYNTHETIC_ONLY cannot label non-synthetic evidence",
        ));
    }
    Ok(())
}

/// Validate one run and recompute THD / resonant sideband ratios from its
/// synchronized RMS spectral lines. Returned ratios are point metrics only;
/// expanded line uncertainties are exposed separately and are not silently
/// treated as independent random errors.
pub fn evaluate_run(run: &Value) -> Result<RunMetrics, String> {
    exact_keys(
        run,
        &[
            "schema_version",
            "run_id",
            "evidence_kind",
            "analysis_model_class",
            "cross_frequency_transfer_claim",
            "grid_frequency_hz",
            "resonant_carrier_hz",
            "minimum_sideband_separation_hz",
            "effective_frequency_resolution_hz",
            "acquisition",
            "channels",
            "quality_flags",
        ],
        "$",
    )?;

    if string_field(run, "schema_version", "$")? != "wpt-dual-frequency-run-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let run_id = string_field(run, "run_id", "$")?.to_string();
    let evidence_kind = string_field(run, "evidence_kind", "$")?;
    if ![
        "MEASURED",
        "PUBLISHED_MEASURED",
        "SIMULATED",
        "SYNTHETIC_KNOWN_TRUTH",
    ]
    .contains(&evidence_kind)
    {
        return Err(fail("$.evidence_kind", "unknown evidence class"));
    }
    validate_quality_flags(run, evidence_kind)?;

    let model_class = string_field(run, "analysis_model_class", "$")?;
    if !["LTI", "LTP", "HSS", "SWITCHED_EMT"].contains(&model_class) {
        return Err(fail(
            "$.analysis_model_class",
            "unknown analysis model class",
        ));
    }
    let cross_frequency = field(run, "cross_frequency_transfer_claim", "$")?
        .as_bool()
        .ok_or_else(|| fail("$.cross_frequency_transfer_claim", "must be boolean"))?;
    if cross_frequency && model_class == "LTI" {
        return Err(fail(
            "$.analysis_model_class",
            "cross-frequency transfer claims require LTP, HSS or SWITCHED_EMT evidence",
        ));
    }

    let grid_frequency = number_field(run, "grid_frequency_hz", "$")?;
    let carrier_frequency = number_field(run, "resonant_carrier_hz", "$")?;
    let min_sideband_separation = number_field(run, "minimum_sideband_separation_hz", "$")?;
    let effective_resolution = number_field(run, "effective_frequency_resolution_hz", "$")?;
    if grid_frequency <= 0.0
        || carrier_frequency <= 0.0
        || min_sideband_separation <= 0.0
        || effective_resolution <= 0.0
    {
        return Err(fail(
            "$",
            "declared frequencies and resolutions must be strictly positive",
        ));
    }
    if effective_resolution > min_sideband_separation / 2.0 {
        return Err(fail(
            "$.effective_frequency_resolution_hz",
            "effective resolution is too coarse to resolve the declared minimum sideband separation",
        ));
    }

    let acquisition = field(run, "acquisition", "$")?;
    exact_keys(
        acquisition,
        &[
            "sample_rate_hz",
            "sample_count",
            "clock_id",
            "synchronization_group_id",
            "started_at_utc",
            "window_function",
            "raw_waveform_sha256",
            "calibration_receipt_id",
        ],
        "$.acquisition",
    )?;
    let sample_rate = number_field(acquisition, "sample_rate_hz", "$.acquisition")?;
    let sample_count = integer_field(acquisition, "sample_count", "$.acquisition")?;
    if sample_rate <= 0.0 || sample_count < 2 {
        return Err(fail(
            "$.acquisition",
            "sample rate must be positive and sample count >= 2",
        ));
    }
    string_field(acquisition, "clock_id", "$.acquisition")?;
    string_field(acquisition, "synchronization_group_id", "$.acquisition")?;
    let started = string_field(acquisition, "started_at_utc", "$.acquisition")?;
    if !is_utc_timestamp(started) {
        return Err(fail(
            "$.acquisition.started_at_utc",
            "must be UTC YYYY-MM-DDTHH:MM:SSZ",
        ));
    }
    if !["RECTANGULAR", "HANN", "BLACKMAN", "FLAT_TOP", "OTHER"].contains(&string_field(
        acquisition,
        "window_function",
        "$.acquisition",
    )?) {
        return Err(fail(
            "$.acquisition.window_function",
            "unknown window function",
        ));
    }
    let raw_digest = string_field(acquisition, "raw_waveform_sha256", "$.acquisition")?;
    if !is_sha256(raw_digest) {
        return Err(fail(
            "$.acquisition.raw_waveform_sha256",
            "must be lowercase SHA-256 hex",
        ));
    }
    string_field(acquisition, "calibration_receipt_id", "$.acquisition")?;

    let nyquist = sample_rate / 2.0;
    let native_resolution = sample_rate / sample_count as f64;
    if effective_resolution + f64::EPSILON < native_resolution {
        return Err(fail(
            "$.effective_frequency_resolution_hz",
            "declared resolution is finer than the finite acquisition window permits",
        ));
    }

    let channels = field(run, "channels", "$")?
        .as_array()
        .ok_or_else(|| fail("$.channels", "must be an array"))?;
    if channels.is_empty() {
        return Err(fail(
            "$.channels",
            "at least one spectral channel is required",
        ));
    }

    let mut channel_ids = BTreeSet::new();
    let mut channel_metrics = Vec::with_capacity(channels.len());
    let mut saw_grid = false;
    let mut saw_resonant = false;
    let mut global_max_frequency: f64 = 0.0;

    for (channel_index, channel) in channels.iter().enumerate() {
        let path = format!("$.channels[{channel_index}]");
        exact_keys(
            channel,
            &[
                "channel_id",
                "channel_kind",
                "measurement_plane_id",
                "phase_reference_id",
                "quantity_kind",
                "unit",
                "nominal_frequency_hz",
                "frequency_tolerance_hz",
                "maximum_harmonic_order",
                "lines",
            ],
            &path,
        )?;
        let channel_id = string_field(channel, "channel_id", &path)?.to_string();
        if !channel_ids.insert(channel_id.clone()) {
            return Err(fail(&path, format!("duplicate channel_id {channel_id}")));
        }
        let kind = string_field(channel, "channel_kind", &path)?;
        if !["GRID_CURRENT", "RESONANT_CURRENT"].contains(&kind) {
            return Err(fail(&path, "unknown channel kind"));
        }
        let quantity = string_field(channel, "quantity_kind", &path)?;
        let unit = string_field(channel, "unit", &path)?;
        if quantity != "RMS_CURRENT_SPECTRUM" || unit != "A" {
            return Err(fail(
                &path,
                "v0 admits RMS current spectra in amperes only; voltage channels require a later schema version",
            ));
        }
        string_field(channel, "measurement_plane_id", &path)?;
        string_field(channel, "phase_reference_id", &path)?;
        let nominal_frequency = number_field(channel, "nominal_frequency_hz", &path)?;
        let tolerance = number_field(channel, "frequency_tolerance_hz", &path)?;
        if nominal_frequency <= 0.0 || tolerance < 0.0 {
            return Err(fail(
                &path,
                "nominal frequency must be positive and tolerance non-negative",
            ));
        }
        let expected_nominal = match kind {
            "GRID_CURRENT" => grid_frequency,
            "RESONANT_CURRENT" => carrier_frequency,
            _ => unreachable!("channel kind checked above"),
        };
        if (nominal_frequency - expected_nominal).abs() > tolerance {
            return Err(fail(
                &path,
                "channel nominal frequency does not match the run-level grid/carrier declaration",
            ));
        }
        let maximum_harmonic_order = match kind {
            "GRID_CURRENT" => {
                let maximum = integer_field(channel, "maximum_harmonic_order", &path)?;
                if maximum == 0 || maximum > 4_096 {
                    return Err(fail(
                        &path,
                        "maximum_harmonic_order must be between 1 and 4096",
                    ));
                }
                let maximum_resolvable_order = (nyquist / grid_frequency).floor() as u64;
                if maximum > maximum_resolvable_order {
                    return Err(fail(
                        &path,
                        "maximum_harmonic_order exceeds the acquisition Nyquist coverage",
                    ));
                }
                Some(maximum)
            }
            "RESONANT_CURRENT" => {
                if !field(channel, "maximum_harmonic_order", &path)?.is_null() {
                    return Err(fail(
                        &path,
                        "resonant-current channel must have null maximum_harmonic_order",
                    ));
                }
                None
            }
            _ => unreachable!("channel kind checked above"),
        };

        let lines = field(channel, "lines", &path)?
            .as_array()
            .ok_or_else(|| fail(&format!("{path}.lines"), "must be an array"))?;
        if lines.is_empty() {
            return Err(fail(
                &format!("{path}.lines"),
                "at least one spectral line is required",
            ));
        }
        let mut previous_frequency = None::<f64>;
        let mut fundamental_amplitude = None::<f64>;
        let mut seen_harmonic_orders = BTreeSet::<u64>::new();
        let mut harmonic_square_sum = 0.0;
        let mut interharmonic_square_sum = 0.0;
        let mut carrier_amplitude = None::<f64>;
        let mut sideband_square_sum = 0.0;
        let mut line_uncertainty_max: f64 = 0.0;

        for (line_index, line) in lines.iter().enumerate() {
            let line_path = format!("{path}.lines[{line_index}]");
            exact_keys(
                line,
                &[
                    "frequency_hz",
                    "rms_amplitude",
                    "phase_deg",
                    "harmonic_order",
                    "sideband_offset_hz",
                    "expanded_uncertainty",
                ],
                &line_path,
            )?;
            let frequency = number_field(line, "frequency_hz", &line_path)?;
            let amplitude = number_field(line, "rms_amplitude", &line_path)?;
            let phase = number_field(line, "phase_deg", &line_path)?;
            let uncertainty = number_field(line, "expanded_uncertainty", &line_path)?;
            if frequency <= 0.0
                || amplitude < 0.0
                || !(-180.0..=180.0).contains(&phase)
                || uncertainty < 0.0
            {
                return Err(fail(
                    &line_path,
                    "invalid frequency, RMS amplitude, phase or uncertainty",
                ));
            }
            if previous_frequency.is_some_and(|previous| frequency <= previous) {
                return Err(fail(
                    &line_path,
                    "spectral frequencies must be strictly increasing with no duplicates",
                ));
            }
            previous_frequency = Some(frequency);
            global_max_frequency = global_max_frequency.max(frequency);
            line_uncertainty_max = line_uncertainty_max.max(uncertainty);

            match kind {
                "GRID_CURRENT" => {
                    if !field(line, "sideband_offset_hz", &line_path)?.is_null() {
                        return Err(fail(
                            &line_path,
                            "grid-current lines cannot carry resonant sideband offsets",
                        ));
                    }
                    let harmonic_value = field(line, "harmonic_order", &line_path)?;
                    if let Some(harmonic) = harmonic_value.as_u64() {
                        if harmonic == 0 {
                            return Err(fail(
                                &line_path,
                                "grid harmonic order must be at least one",
                            ));
                        }
                        if !seen_harmonic_orders.insert(harmonic) {
                            return Err(fail(
                                &line_path,
                                format!("duplicate grid harmonic order {harmonic}"),
                            ));
                        }
                        let expected = grid_frequency * harmonic as f64;
                        if (frequency - expected).abs() > tolerance {
                            return Err(fail(
                                &line_path,
                                format!(
                                    "frequency does not match harmonic order {harmonic} of the declared grid"
                                ),
                            ));
                        }
                        if harmonic == 1 {
                            if fundamental_amplitude.replace(amplitude).is_some() {
                                return Err(fail(&line_path, "duplicate grid fundamental"));
                            }
                        } else {
                            harmonic_square_sum += amplitude * amplitude;
                        }
                    } else if harmonic_value.is_null() {
                        let nearest_order = (frequency / grid_frequency).round();
                        if nearest_order >= 1.0
                            && (frequency - nearest_order * grid_frequency).abs() <= tolerance
                        {
                            return Err(fail(
                                &line_path,
                                "integer grid harmonic must declare harmonic_order",
                            ));
                        }
                        interharmonic_square_sum += amplitude * amplitude;
                    } else {
                        return Err(fail(
                            &line_path,
                            "grid harmonic_order must be a positive integer or null for an interharmonic",
                        ));
                    }
                    if !harmonic_square_sum.is_finite() || !interharmonic_square_sum.is_finite() {
                        return Err(fail(
                            &line_path,
                            "grid spectral power accumulation overflowed",
                        ));
                    }
                }
                "RESONANT_CURRENT" => {
                    if !field(line, "harmonic_order", &line_path)?.is_null() {
                        return Err(fail(
                            &line_path,
                            "resonant spectral lines must have null grid harmonic_order",
                        ));
                    }
                    let offset_value = field(line, "sideband_offset_hz", &line_path)?;
                    if offset_value.is_null() {
                        if (frequency - carrier_frequency).abs() > tolerance {
                            return Err(fail(
                                &line_path,
                                "carrier line does not match resonant_carrier_hz",
                            ));
                        }
                        if carrier_amplitude.replace(amplitude).is_some() {
                            return Err(fail(&line_path, "duplicate resonant carrier line"));
                        }
                    } else {
                        let offset = offset_value.as_f64().ok_or_else(|| {
                            fail(&line_path, "sideband_offset_hz must be numeric or null")
                        })?;
                        if !offset.is_finite() || offset.abs() < min_sideband_separation {
                            return Err(fail(
                                &line_path,
                                "sideband offset is below declared minimum separation",
                            ));
                        }
                        if (frequency - (carrier_frequency + offset)).abs() > tolerance {
                            return Err(fail(
                                &line_path,
                                "sideband frequency does not equal carrier plus offset",
                            ));
                        }
                        sideband_square_sum += amplitude * amplitude;
                        if !sideband_square_sum.is_finite() {
                            return Err(fail(
                                &line_path,
                                "resonant sideband power accumulation overflowed",
                            ));
                        }
                    }
                }
                _ => unreachable!("channel kind checked above"),
            }
        }

        let (thd_order, thd, interharmonic_ratio, sideband_ratio) = match kind {
            "GRID_CURRENT" => {
                saw_grid = true;
                let maximum =
                    maximum_harmonic_order.expect("grid channel declared harmonic coverage");
                let expected_orders = (1..=maximum).collect::<BTreeSet<_>>();
                if seen_harmonic_orders != expected_orders {
                    return Err(fail(
                        &path,
                        format!(
                            "grid spectrum must include every integer harmonic order from 1 through {maximum}"
                        ),
                    ));
                }
                let fundamental = fundamental_amplitude
                    .filter(|amplitude| *amplitude > 0.0)
                    .ok_or_else(|| {
                        fail(
                            &path,
                            "grid-current spectrum requires one non-zero fundamental",
                        )
                    })?;
                (
                    Some(maximum),
                    Some(harmonic_square_sum.sqrt() / fundamental),
                    Some(interharmonic_square_sum.sqrt() / fundamental),
                    None,
                )
            }
            "RESONANT_CURRENT" => {
                saw_resonant = true;
                let carrier = carrier_amplitude
                    .filter(|amplitude| *amplitude > 0.0)
                    .ok_or_else(|| {
                        fail(&path, "resonant spectrum requires one non-zero carrier")
                    })?;
                (None, None, None, Some(sideband_square_sum.sqrt() / carrier))
            }
            _ => unreachable!("channel kind checked above"),
        };

        for (metric_name, metric_value) in [
            ("THD", thd),
            ("interharmonic ratio", interharmonic_ratio),
            ("resonant sideband ratio", sideband_ratio),
        ] {
            if metric_value.is_some_and(|value| !value.is_finite()) {
                return Err(fail(&path, format!("{metric_name} calculation overflowed")));
            }
        }

        channel_metrics.push(ChannelMetrics {
            channel_id,
            channel_kind: kind.to_string(),
            maximum_harmonic_order: thd_order,
            thd_through_harmonic_order: thd,
            interharmonic_to_fundamental_rms_ratio: interharmonic_ratio,
            sideband_to_carrier_rms_ratio: sideband_ratio,
            line_count: lines.len(),
            maximum_expanded_line_uncertainty: line_uncertainty_max,
        });
    }

    if !saw_grid || !saw_resonant {
        return Err(fail(
            "$.channels",
            "v0 requires at least one grid-current and one resonant-current channel",
        ));
    }
    if global_max_frequency >= nyquist {
        return Err(fail(
            "$.acquisition.sample_rate_hz",
            format!(
                "Nyquist frequency {nyquist} Hz does not exceed maximum spectral frequency {global_max_frequency} Hz"
            ),
        ));
    }

    Ok(RunMetrics {
        run_id,
        nyquist_frequency_hz: nyquist,
        native_fourier_resolution_hz: native_resolution,
        effective_frequency_resolution_hz: effective_resolution,
        channels: channel_metrics,
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-dual-frequency-v0/dual-frequency-run.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic fixture JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn synthetic_known_truth_spectra_compute_expected_metrics() {
        let report = evaluate_run(&fixture()).unwrap();
        let grid = report
            .channels
            .iter()
            .find(|channel| channel.channel_kind == "GRID_CURRENT")
            .unwrap();
        let resonant = report
            .channels
            .iter()
            .find(|channel| channel.channel_kind == "RESONANT_CURRENT")
            .unwrap();

        let expected_thd_through_order_11 = 0.061644140029689765;
        let expected_interharmonic_ratio = 0.01;
        let expected_sideband_ratio = 0.032015621187164243;
        assert_eq!(grid.maximum_harmonic_order, Some(11));
        assert!(
            (grid.thd_through_harmonic_order.unwrap() - expected_thd_through_order_11).abs()
                < 1e-12
        );
        assert!(
            (grid.interharmonic_to_fundamental_rms_ratio.unwrap() - expected_interharmonic_ratio)
                .abs()
                < 1e-12
        );
        assert!(
            (resonant.sideband_to_carrier_rms_ratio.unwrap() - expected_sideband_ratio).abs()
                < 1e-12
        );
        assert_eq!(report.nyquist_frequency_hz, 500_000.0);
        assert_eq!(report.native_fourier_resolution_hz, 10.0);
    }

    #[test]
    fn rejects_missing_integer_harmonic_bin_within_declared_coverage() {
        let mut run = fixture();
        run["channels"][0]["lines"]
            .as_array_mut()
            .unwrap()
            .remove(4); // order 4 at 200 Hz
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("must include every integer harmonic order from 1 through 11")
        );
    }

    #[test]
    fn grid_interharmonics_are_reported_separately_from_thd() {
        let report = evaluate_run(&fixture()).unwrap();
        let grid = report
            .channels
            .iter()
            .find(|channel| channel.channel_kind == "GRID_CURRENT")
            .unwrap();
        assert_eq!(grid.maximum_harmonic_order, Some(11));
        assert!((grid.interharmonic_to_fundamental_rms_ratio.unwrap() - 0.01).abs() < 1e-12);
        assert!((grid.thd_through_harmonic_order.unwrap() - 0.061644140029689765).abs() < 1e-12);
    }

    #[test]
    fn rejects_harmonic_coverage_above_nyquist() {
        let mut run = fixture();
        run["channels"][0]["maximum_harmonic_order"] = Value::from(20_000);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("maximum_harmonic_order must be between 1 and 4096")
        );
    }

    #[test]
    fn rejects_non_finite_derived_thd() {
        let mut run = fixture();
        run["channels"][0]["lines"][3]["rms_amplitude"] = Value::from(1.0e-10);
        run["channels"][0]["lines"][0]["rms_amplitude"] = Value::from(1.0e-320);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("THD calculation overflowed")
        );
    }

    #[test]
    fn rejects_integer_multiple_mislabelled_as_interharmonic() {
        let mut run = fixture();
        run["channels"][0]["lines"][3]["frequency_hz"] = Value::from(150.01);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("integer grid harmonic must declare harmonic_order")
        );
    }

    #[test]
    fn rejects_spectral_power_accumulation_overflow() {
        let mut run = fixture();
        run["channels"][0]["lines"][1]["rms_amplitude"] = Value::from(1.0e308);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("grid spectral power accumulation overflowed")
        );
    }

    #[test]
    fn rejects_cross_frequency_transfer_claim_from_stationary_lti_model() {
        let mut run = fixture();
        run["analysis_model_class"] = Value::String("LTI".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("cross-frequency transfer claims require LTP, HSS or SWITCHED_EMT")
        );
    }

    #[test]
    fn rejects_sampling_rate_below_nyquist_requirement() {
        let mut run = fixture();
        run["acquisition"]["sample_rate_hz"] = Value::from(10_000.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("Nyquist frequency")
        );
    }

    #[test]
    fn rejects_effective_resolution_that_cannot_separate_sidebands() {
        let mut run = fixture();
        run["effective_frequency_resolution_hz"] = Value::from(26.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("too coarse to resolve")
        );
    }

    #[test]
    fn rejects_resolution_finer_than_acquisition_window() {
        let mut run = fixture();
        run["effective_frequency_resolution_hz"] = Value::from(5.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("finite acquisition window")
        );
    }

    #[test]
    fn rejects_unsorted_or_duplicate_spectral_lines() {
        let mut run = fixture();
        run["channels"][0]["lines"]
            .as_array_mut()
            .unwrap()
            .swap(5, 7);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("strictly increasing")
        );
    }

    #[test]
    fn rejects_grid_harmonic_at_wrong_frequency() {
        let mut run = fixture();
        run["channels"][0]["lines"][1]["frequency_hz"] = Value::from(102.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("does not match harmonic order")
        );
    }

    #[test]
    fn rejects_resonant_sideband_offset_mismatch() {
        let mut run = fixture();
        run["channels"][1]["lines"][0]["sideband_offset_hz"] = Value::from(50.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("carrier plus offset")
        );
    }

    #[test]
    fn rejects_missing_grid_fundamental() {
        let mut run = fixture();
        run["channels"][0]["lines"][0]["rms_amplitude"] = Value::from(0.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("requires one non-zero fundamental")
        );
    }

    #[test]
    fn rejects_impossible_utc_calendar_timestamp() {
        let mut run = fixture();
        run["acquisition"]["started_at_utc"] = Value::String("2026-02-30T00:00:00Z".into());
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("must be UTC YYYY-MM-DDTHH:MM:SSZ")
        );
    }

    #[test]
    fn rejects_channel_nominal_frequency_inconsistent_with_run() {
        let mut run = fixture();
        run["channels"][1]["nominal_frequency_hz"] = Value::from(84_000.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("does not match the run-level grid/carrier declaration")
        );
    }

    #[test]
    fn rejects_grid_harmonic_order_on_resonant_channel() {
        let mut run = fixture();
        run["channels"][1]["lines"][0]["harmonic_order"] = Value::from(1);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("must have null grid harmonic_order")
        );
    }

    #[test]
    fn rejects_missing_synthetic_only_flag() {
        let mut run = fixture();
        run["quality_flags"] = Value::Array(vec![]);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("requires SYNTHETIC_ONLY")
        );
    }

    #[test]
    fn rejects_unknown_properties() {
        let mut run = fixture();
        run["unreviewed_confidence"] = Value::from(0.99);
        assert!(evaluate_run(&run).unwrap_err().contains("unknown property"));
    }

    #[test]
    fn rejects_resonant_sideband_below_minimum_declared_separation() {
        let mut run = fixture();
        run["channels"][1]["lines"][1]["sideband_offset_hz"] = Value::from(10.0);
        run["channels"][1]["lines"][1]["frequency_hz"] = Value::from(85_010.0);
        assert!(
            evaluate_run(&run)
                .unwrap_err()
                .contains("below declared minimum")
        );
    }

    #[test]
    fn rejects_unsynchronized_phase_reference_acquisition_metadata() {
        let mut run = fixture();
        run["acquisition"]["synchronization_group_id"] = Value::String("other-sync".into());
        // A single acquisition object is the declared synchronization group;
        // a channel cannot override that group or smuggle a different clock.
        run["channels"][1]["unsynchronized_clock_id"] = Value::String("rogue-clock".into());
        assert!(evaluate_run(&run).unwrap_err().contains("unknown property"));
    }
}
