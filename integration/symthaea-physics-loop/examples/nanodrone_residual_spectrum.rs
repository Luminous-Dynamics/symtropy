//! Frequency-domain diagnostics for multi-horizon Nano-Quadrotor prediction residuals.
//!
//! Reads one joined held-out run, verifies its source timebase and complete x0/u_seq
//! conditioning schema, then measures residual power by frequency at fixed horizons.
//! This is a diagnostic for deciding whether slow latent/actuator models are warranted,
//! not a significance test or a safety metric.

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io;
use std::path::Path;

use csv::ReaderBuilder;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

const SAMPLE_HZ: f64 = 100.0;
const EXPECTED_DT_S: f64 = 1.0 / SAMPLE_HZ;
const DT_TOLERANCE_S: f64 = 0.002;
const HORIZONS: [usize; 5] = [1, 5, 10, 20, 50];
const FFT_LEN: usize = 256;
const FFT_OVERLAP: usize = 128;
const LOW_BAND_HZ: f64 = 3.0;
const MID_BAND_HZ: f64 = 10.0;
const MOTOR_FIELDS: [&str; 4] = ["m1_rads", "m2_rads", "m3_rads", "m4_rads"];
const POSITION_FIELDS: [&str; 3] = ["x", "y", "z"];
const VELOCITY_FIELDS: [&str; 3] = ["vx", "vy", "vz"];
const ROTVEC_FIELDS: [&str; 3] = ["rx", "ry", "rz"];
const ANGULAR_RATE_FIELDS: [&str; 3] = ["wx", "wy", "wz"];

#[derive(Debug)]
struct JoinedRun {
    filename: String,
    sha256: String,
    rows: Vec<HashMap<String, f64>>,
}

#[derive(Clone)]
struct Spectrum {
    frequency_hz: Vec<f64>,
    psd: Vec<f64>,
    segment_count: usize,
}

fn invalid_input(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn prediction_key(field: &str, horizon: usize) -> String {
    format!("{field}_pred_h{horizon}")
}

fn target_key(field: &str, horizon: usize) -> String {
    format!("{field}_true_h{horizon}")
}

fn input_state_key(field: &str) -> String {
    format!("x0_{field}")
}

fn motor_sequence_key(field: &str, step: usize) -> String {
    format!("{field}_input_t{step}")
}

fn input_time_key(step: usize) -> String {
    format!("input_time_t{step}")
}

fn state_fields() -> Vec<&'static str> {
    let mut fields = POSITION_FIELDS.to_vec();
    fields.extend_from_slice(&VELOCITY_FIELDS);
    fields.extend_from_slice(&ROTVEC_FIELDS);
    fields.extend_from_slice(&ANGULAR_RATE_FIELDS);
    fields
}

fn parse_headers(headers: &csv::StringRecord) -> Result<HashMap<String, usize>, io::Error> {
    let mut result = HashMap::new();
    for (index, name) in headers.iter().enumerate() {
        if name.trim().is_empty() {
            return Err(invalid_input(format!("blank column name at index {index}")));
        }
        if result.insert(name.to_owned(), index).is_some() {
            return Err(invalid_input(format!("duplicate column name '{name}'")));
        }
    }
    Ok(result)
}

fn require_conditioning_schema(headers: &HashMap<String, usize>) -> Result<(), io::Error> {
    for name in ["t", "x0_t"] {
        if !headers.contains_key(name) {
            return Err(invalid_input(format!("missing required conditioning field '{name}'")));
        }
    }
    for field in state_fields() {
        let name = input_state_key(field);
        if !headers.contains_key(&name) {
            return Err(invalid_input(format!("missing initial-state field '{name}'")));
        }
    }
    for step in 0..50 {
        let name = input_time_key(step);
        if !headers.contains_key(&name) {
            return Err(invalid_input(format!("missing motor-input timestamp '{name}'")));
        }
        for field in MOTOR_FIELDS {
            let name = motor_sequence_key(field, step);
            if !headers.contains_key(&name) {
                return Err(invalid_input(format!("missing motor input '{name}'")));
            }
        }
    }
    Ok(())
}

fn load_joined_run(path: &Path) -> Result<JoinedRun, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    if bytes.is_empty() {
        return Err(invalid_input(format!("{} is empty", path.display())).into());
    }
    let filename = path.file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| invalid_input("input filename must be UTF-8"))?
        .to_owned();
    let digest = sha256(&bytes);
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(bytes.as_slice());
    let raw_headers = reader.headers()?.clone();
    let headers = parse_headers(&raw_headers)?;
    require_conditioning_schema(&headers)?;

    for field in state_fields() {
        for horizon in 1..=50 {
            for name in [prediction_key(field, horizon), target_key(field, horizon)] {
                if !headers.contains_key(&name) {
                    return Err(invalid_input(format!("missing required evaluation field '{name}'")).into());
                }
            }
        }
    }

    let required = headers.keys().cloned().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut previous_time = None::<f64>;
    for (index, record_result) in reader.records().enumerate() {
        let record = record_result?;
        let mut row = HashMap::with_capacity(required.len());
        for name in &required {
            let column = headers[name];
            let cell = record.get(column).ok_or_else(|| {
                invalid_input(format!("row {} missing value in '{name}'", index + 2))
            })?;
            let value = cell.parse::<f64>().map_err(|error| {
                invalid_input(format!("row {} column '{name}' is not numeric: {error}", index + 2))
            })?;
            if !value.is_finite() {
                return Err(invalid_input(format!("row {} column '{name}' is non-finite", index + 2)).into());
            }
            row.insert(name.clone(), value);
        }

        let time = row["t"];
        if let Some(previous) = previous_time {
            let dt = time - previous;
            if !dt.is_finite() || dt <= 0.0 || (dt - EXPECTED_DT_S).abs() > DT_TOLERANCE_S {
                return Err(invalid_input(format!(
                    "row {} source timestamp delta {dt:.9}s is outside 100 Hz tolerance",
                    index + 2
                )).into());
            }
        }
        previous_time = Some(time);
        validate_conditioning_row(&row, index + 2)?;

        for field in MOTOR_FIELDS {
            for step in 0..50 {
                if row[&motor_sequence_key(field, step)] < 0.0 {
                    return Err(invalid_input(format!(
                        "row {} motor input {field} step {step} must be non-negative",
                        index + 2
                    )).into());
                }
            }
        }
        rows.push(row);
    }

    if rows.len() < FFT_LEN {
        return Err(invalid_input(format!(
            "need at least {FFT_LEN} joined prediction windows for spectral analysis; found {}",
            rows.len()
        )).into());
    }

    let lower_name = filename.to_ascii_lowercase();
    if !lower_name.starts_with("melon_")
        || !["_run1.csv", "_run2.csv", "_run3.csv"].iter().any(|suffix| lower_name.ends_with(suffix))
    {
        return Err(invalid_input(format!(
            "'{filename}' must be one of the held-out Melon run1/run2/run3 files"
        )).into());
    }
    Ok(JoinedRun { filename, sha256: digest, rows })
}

fn validate_conditioning_row(row: &HashMap<String, f64>, row_number: usize) -> Result<(), io::Error> {
    let time = row["t"];
    let x0_time = row["x0_t"];
    if (time - x0_time).abs() > 1.0e-9 {
        return Err(invalid_input(format!(
            "row {row_number}: prediction timestamp {time} does not match x0_t {x0_time}"
        )));
    }
    if (time - row[&input_time_key(0)]).abs() > 1.0e-9 {
        return Err(invalid_input(format!(
            "row {row_number}: t does not match first motor-input timestamp"
        )));
    }
    let mut previous = row[&input_time_key(0)];
    for step in 1..50 {
        let current = row[&input_time_key(step)];
        let dt = current - previous;
        if !dt.is_finite() || dt <= 0.0 || (dt - EXPECTED_DT_S).abs() > DT_TOLERANCE_S {
            return Err(invalid_input(format!(
                "row {row_number}: motor-input timestamp step {step} has delta {dt:.9}s"
            )));
        }
        previous = current;
    }
    Ok(())
}

fn rotvec_to_quaternion(r: [f64; 3]) -> [f64; 4] {
    let angle = (r[0] * r[0] + r[1] * r[1] + r[2] * r[2]).sqrt();
    if angle <= f64::EPSILON {
        return [0.0, 0.0, 0.0, 1.0];
    }
    let scale = (angle * 0.5).sin() / angle;
    [r[0] * scale, r[1] * scale, r[2] * scale, (angle * 0.5).cos()]
}

fn quaternion_conjugate(q: [f64; 4]) -> [f64; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

fn quaternion_multiply(a: [f64; 4], b: [f64; 4]) -> [f64; 4] {
    let [x1, y1, z1, w1] = a;
    let [x2, y2, z2, w2] = b;
    [
        w1 * x2 + x1 * w2 + y1 * z2 - z1 * y2,
        w1 * y2 - x1 * z2 + y1 * w2 + z1 * x2,
        w1 * z2 + x1 * y2 - y1 * x2 + z1 * w2,
        w1 * w2 - x1 * x2 - y1 * y2 - z1 * z2,
    ]
}

fn relative_rotvec(truth: [f64; 3], prediction: [f64; 3]) -> [f64; 3] {
    let q_truth = rotvec_to_quaternion(truth);
    let q_prediction = rotvec_to_quaternion(prediction);
    let mut relative = quaternion_multiply(quaternion_conjugate(q_truth), q_prediction);
    if relative[3] < 0.0 {
        for component in &mut relative {
            *component = -*component;
        }
    }
    let vector_norm = (relative[0] * relative[0] + relative[1] * relative[1] + relative[2] * relative[2]).sqrt();
    if vector_norm <= f64::EPSILON {
        return [0.0, 0.0, 0.0];
    }
    let angle = 2.0 * vector_norm.atan2(relative[3].max(0.0));
    let scale = angle / vector_norm;
    [relative[0] * scale, relative[1] * scale, relative[2] * scale]
}

fn fft(values: &mut [(f64, f64)]) {
    let n = values.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            values.swap(i, j);
        }
    }

    let mut length = 2usize;
    while length <= n {
        let angle = -2.0 * std::f64::consts::PI / length as f64;
        let w_length = (angle.cos(), angle.sin());
        for start in (0..n).step_by(length) {
            let mut w = (1.0, 0.0);
            for offset in 0..(length / 2) {
                let even = values[start + offset];
                let odd_raw = values[start + offset + length / 2];
                let odd = (
                    odd_raw.0 * w.0 - odd_raw.1 * w.1,
                    odd_raw.0 * w.1 + odd_raw.1 * w.0,
                );
                values[start + offset] = (even.0 + odd.0, even.1 + odd.1);
                values[start + offset + length / 2] = (even.0 - odd.0, even.1 - odd.1);
                w = (
                    w.0 * w_length.0 - w.1 * w_length.1,
                    w.0 * w_length.1 + w.1 * w_length.0,
                );
            }
        }
        length <<= 1;
    }
}

fn estimate_psd(series: &[f64]) -> Result<Spectrum, io::Error> {
    if series.len() < FFT_LEN {
        return Err(invalid_input(format!(
            "residual series has {} samples; Welch window requires {FFT_LEN}",
            series.len()
        )));
    }
    let step = FFT_LEN - FFT_OVERLAP;
    let bin_count = FFT_LEN / 2 + 1;
    let mut psd_sum = vec![0.0; bin_count];
    let mut segment_count = 0usize;
    let window = (0..FFT_LEN)
        .map(|index| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * index as f64 / (FFT_LEN - 1) as f64).cos())
        .collect::<Vec<_>>();
    let window_energy = window.iter().map(|value| value * value).sum::<f64>();

    for start in (0..=(series.len() - FFT_LEN)).step_by(step) {
        let segment = &series[start..start + FFT_LEN];
        let mean = segment.iter().sum::<f64>() / FFT_LEN as f64;
        let mut transform = segment.iter().zip(&window)
            .map(|(value, taper)| ((value - mean) * taper, 0.0))
            .collect::<Vec<_>>();
        fft(&mut transform);
        for bin in 0..bin_count {
            let magnitude_sq = transform[bin].0 * transform[bin].0 + transform[bin].1 * transform[bin].1;
            let one_sided_factor = if bin == 0 || bin == FFT_LEN / 2 { 1.0 } else { 2.0 };
            psd_sum[bin] += one_sided_factor * magnitude_sq / (SAMPLE_HZ * window_energy);
        }
        segment_count += 1;
    }

    if segment_count == 0 {
        return Err(invalid_input("no complete Welch segments found"));
    }
    for value in &mut psd_sum {
        *value /= segment_count as f64;
    }
    let frequency_hz = (0..bin_count)
        .map(|bin| bin as f64 * SAMPLE_HZ / FFT_LEN as f64)
        .collect();
    Ok(Spectrum { frequency_hz, psd: psd_sum, segment_count })
}

fn summarize_psd(spectrum: &Spectrum, residual_unit: &str) -> Value {
    let delta_f = SAMPLE_HZ / FFT_LEN as f64;
    let mut low = 0.0;
    let mut mid = 0.0;
    let mut high = 0.0;
    let mut total = 0.0;
    let mut peak_power = 0.0;
    let mut peak_frequency = 0.0;

    for (index, power_density) in spectrum.psd.iter().enumerate().skip(1) {
        let frequency = spectrum.frequency_hz[index];
        let power = power_density * delta_f;
        total += power;
        if frequency < LOW_BAND_HZ {
            low += power;
        } else if frequency < MID_BAND_HZ {
            mid += power;
        } else {
            high += power;
        }
        if *power_density > peak_power {
            peak_power = *power_density;
            peak_frequency = frequency;
        }
    }

    json!({
        "residual_unit": residual_unit,
        "power_spectral_density_unit": format!("({residual_unit})^2/Hz"),
        "welch_segment_count": spectrum.segment_count,
        "welch_window_samples": FFT_LEN,
        "welch_overlap_samples": FFT_OVERLAP,
        "frequency_resolution_hz": delta_f,
        "peak_residual_frequency_hz": peak_frequency,
        "integrated_residual_power": total,
        "band_power": {
            "low_0_to_3_hz": low,
            "mid_3_to_10_hz": mid,
            "high_10_to_50_hz": high
        },
        "low_frequency_power_fraction": if total > f64::EPSILON { json!(low / total) } else { Value::Null },
        "mid_frequency_power_fraction": if total > f64::EPSILON { json!(mid / total) } else { Value::Null },
        "high_frequency_power_fraction": if total > f64::EPSILON { json!(high / total) } else { Value::Null }
    })
}

fn group_spectrum(run: &JoinedRun, fields: &[&str], horizon: usize, is_orientation: bool, residual_unit: &str) -> Result<Value, Box<dyn Error>> {
    let mut axis_summaries = Map::new();
    let mut combined_psd: Option<Spectrum> = None;
    let series_count = run.rows.len();

    for field in fields {
        let mut residuals = Vec::with_capacity(series_count);
        for row in &run.rows {
            let residual = if is_orientation {
                let true_r = [
                    row[&target_key("rx", horizon)],
                    row[&target_key("ry", horizon)],
                    row[&target_key("rz", horizon)],
                ];
                let pred_r = [
                    row[&prediction_key("rx", horizon)],
                    row[&prediction_key("ry", horizon)],
                    row[&prediction_key("rz", horizon)],
                ];
                relative_rotvec(true_r, pred_r)[match *field {
                    "rx" => 0,
                    "ry" => 1,
                    _ => 2,
                }]
            } else {
                row[&prediction_key(field, horizon)] - row[&target_key(field, horizon)]
            };
            residuals.push(residual);
        }
        let spectrum = estimate_psd(&residuals)?;
        axis_summaries.insert((*field).to_owned(), summarize_psd(&spectrum, residual_unit));
        if let Some(accumulator) = &mut combined_psd {
            for (left, right) in accumulator.psd.iter_mut().zip(&spectrum.psd) {
                *left += *right;
            }
            accumulator.segment_count = accumulator.segment_count.min(spectrum.segment_count);
        } else {
            combined_psd = Some(spectrum);
        }
    }

    let mut combined = combined_psd.ok_or_else(|| invalid_input("empty residual channel group"))?;
    for value in &mut combined.psd {
        *value /= fields.len() as f64;
    }
    Ok(json!({
        "coordinate_space": if is_orientation { "shortest_relative_SO3_log_error" } else { "state_component_prediction_minus_truth" },
        "axis_spectra": Value::Object(axis_summaries),
        "combined_axis_mean_spectrum": summarize_psd(&combined, residual_unit)
    }))
}

fn analyse_run(run: &JoinedRun) -> Result<Value, Box<dyn Error>> {
    let mut horizon_results = Vec::new();
    for horizon in HORIZONS {
        horizon_results.push(json!({
            "horizon_steps": horizon,
            "horizon_seconds": horizon as f64 * EXPECTED_DT_S,
            "channels": {
                "position": group_spectrum(run, &POSITION_FIELDS, horizon, false, "m")?,
                "linear_velocity": group_spectrum(run, &VELOCITY_FIELDS, horizon, false, "m/s")?,
                "orientation": group_spectrum(run, &ROTVEC_FIELDS, horizon, true, "rad")?,
                "body_angular_rate": group_spectrum(run, &ANGULAR_RATE_FIELDS, horizon, false, "rad/s")?
            }
        }));
    }
    Ok(json!({
        "diagnostic_id": "symtropy-nanodrone-residual-spectrum-v0.1",
        "claim_ceiling": "frequency_domain_residual_diagnostic_only; not statistical significance, controller quality, or flight safety evidence",
        "source": {
            "joined_prediction_file": run.filename,
            "joined_prediction_sha256": run.sha256,
            "source_timebase_hz": SAMPLE_HZ,
            "joined_window_count": run.rows.len(),
            "conditioning_schema_verified": true,
            "full_50_step_motor_input_window_present": true
        },
        "method": {
            "horizons": HORIZONS,
            "welch": {
                "window_samples": FFT_LEN,
                "overlap_samples": FFT_OVERLAP,
                "window": "Hann",
                "detrend": "subtract each segment mean",
                "frequency_resolution_hz": SAMPLE_HZ / FFT_LEN as f64
            },
            "frequency_bands_hz": {
                "low": [0.0, LOW_BAND_HZ],
                "mid": [LOW_BAND_HZ, MID_BAND_HZ],
                "high": [MID_BAND_HZ, SAMPLE_HZ / 2.0]
            },
            "overlapping_prediction_windows": "diagnostic samples are temporally dependent; do not use row count as independent-trial count",
            "orientation": "compute shortest relative SO(3) logarithm between each true and predicted rotation vector; report component and combined-axis PSD"
        },
        "per_horizon": horizon_results
    }))
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(invalid_input(
            "usage: nanodrone_residual_spectrum <joined_melon_runN.csv> <new_output_spectrum.json>",
        ).into());
    }
    let run = load_joined_run(Path::new(&args[0]))?;
    let report = analyse_run(&run)?;
    let output = Path::new(&args[1]);
    if output.exists() {
        return Err(invalid_input(format!(
            "output '{}' already exists; choose a fresh path to avoid stale-result mixing",
            output.display()
        )).into());
    }
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    let bytes = serde_json::to_vec_pretty(&report)?;
    let tmp = output.with_extension(format!("tmp-{}", std::process::id()));
    if let Err(error) = fs::write(&tmp, &bytes) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    if let Err(error) = fs::rename(&tmp, output) {
        let _ = fs::remove_file(&tmp);
        return Err(error.into());
    }
    println!("{}", String::from_utf8(bytes)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn welch_spectrum_separates_slow_and_fast_residuals() {
        let sample_count = 2048usize;
        let slow = (0..sample_count)
            .map(|index| (2.0 * std::f64::consts::PI * 1.0 * index as f64 / SAMPLE_HZ).sin())
            .collect::<Vec<_>>();
        let fast = (0..sample_count)
            .map(|index| (2.0 * std::f64::consts::PI * 20.0 * index as f64 / SAMPLE_HZ).sin())
            .collect::<Vec<_>>();
        let slow_psd = estimate_psd(&slow).unwrap();
        let fast_psd = estimate_psd(&fast).unwrap();
        let slow_report = summarize_psd(&slow_psd, "m");
        let fast_report = summarize_psd(&fast_psd, "m");
        assert!(slow_report["low_frequency_power_fraction"].as_f64().unwrap() > 0.95);
        assert!(fast_report["high_frequency_power_fraction"].as_f64().unwrap() > 0.95);
        assert!(slow_report["peak_residual_frequency_hz"].as_f64().unwrap() < LOW_BAND_HZ);
        assert!(fast_report["peak_residual_frequency_hz"].as_f64().unwrap() > MID_BAND_HZ);
    }

    #[test]
    fn shortest_relative_orientation_residual_treats_equivalent_rotations_as_zero() {
        let truth = [0.0, 0.0, std::f64::consts::PI / 2.0];
        let prediction = [0.0, 0.0, std::f64::consts::PI / 2.0];
        let residual = relative_rotvec(truth, prediction);
        assert!(residual.iter().all(|value| value.abs() < 1.0e-12));
    }

    #[test]
    fn residual_spectrum_rejects_gapped_conditioning_timestamps() {
        let mut row = HashMap::new();
        row.insert("t".to_owned(), 0.0);
        row.insert("x0_t".to_owned(), 0.0);
        for step in 0..50 {
            row.insert(input_time_key(step), step as f64 * EXPECTED_DT_S);
        }
        row.insert(input_time_key(49), 0.9);
        assert!(validate_conditioning_row(&row, 2).is_err());
    }
}
