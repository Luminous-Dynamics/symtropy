//! Provenance-aware multi-rate CNC telemetry feature reference.
//!
//! Synthetic-only first tranche. Raw channels remain separate; timestamps are
//! mapped into one declared reference clock without interpolation or zero-fill.
//! SHA-256 source digests are supplied by the data-admission layer and carried
//! through each feature row; the workflow hashes the exact output bytes.
//!
//! Compile directly, outside the Bevy workspace:
//!   rustc --edition=2024 -D warnings --test tools/machining-telemetry/telemetry_reference.rs -o /tmp/telemetry-tests
//!   rustc --edition=2024 -D warnings tools/machining-telemetry/telemetry_reference.rs -o /tmp/telemetry-reference

const TRANSFORM_VERSION: &str = "mach-telemetry-features-v0.1.0";
const MAX_CLOCK_UNCERTAINTY_NS: u64 = 100_000;
const MAX_RELATIVE_SAMPLE_JITTER: f64 = 0.10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChannelKind { Vibration, Current }

#[derive(Clone, Copy, Debug)]
struct SensorMeta<'a> {
    sensor_id: &'a str,
    kind: ChannelKind,
    unit: &'a str,
    nominal_rate_hz: f64,
    clock_domain: &'a str,
    calibration_id: &'a str,
    calibration_valid_until_ref_ns: i64,
    source_sha256: &'a str,
}

#[derive(Clone, Copy, Debug)]
struct ClockMap<'a> {
    clock_domain: &'a str,
    reference_clock: &'a str,
    offset_ns: i64,
    origin_local_ns: i64,
    drift_ppb: i64,
    uncertainty_ns: u64,
}

#[derive(Clone, Copy, Debug)]
struct Window {
    start_ref_ns: i64,
    end_ref_ns: i64, // half-open [start, end)
}

struct RawChannel<'a> {
    meta: SensorMeta<'a>,
    timestamps_local_ns: &'a [i64],
    values: &'a [f64],
    clock_map: Option<ClockMap<'a>>,
}

#[derive(Debug)]
struct ExtractedChannel<'a> {
    reference_clock: &'a str,
    meta: SensorMeta<'a>,
    source_sha256: &'a str,
    mapped_timestamps_ns: Vec<i64>,
    values: Vec<f64>,
    expected_samples: usize,
    missing_samples: usize,
    clock_uncertainty_ns: u64,
    window: Window,
}

#[derive(Debug)]
struct Features {
    sensor_id: String,
    kind: &'static str,
    unit: String,
    source_sha256: String,
    window_start_ref_ns: i64,
    window_end_ref_ns: i64,
    nominal_rate_hz: f64,
    observed_samples: usize,
    expected_samples: usize,
    missing_samples: usize,
    missing_fraction: f64,
    mean: f64,
    rms: f64,
    peak_to_peak: f64,
    crest_factor: f64,
    unbiased_variance: f64,
    excess_kurtosis: f64,
    clock_uncertainty_ns: u64,
}

#[derive(Debug)]
struct PairedFeatures {
    pair_count: usize,
    pearson_r: f64,
    max_nearest_skew_ns: i64,
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

fn map_timestamp(map: ClockMap<'_>, local_ns: i64) -> Result<i64, String> {
    if map.clock_domain.trim().is_empty() || map.reference_clock.trim().is_empty() {
        return Err("CLOCK_MAP_INVALID: clock identities must be non-empty".into());
    }
    let delta = (local_ns as i128) - (map.origin_local_ns as i128);
    let drift = delta
        .checked_mul(map.drift_ppb as i128)
        .ok_or("CLOCK_MAP_OVERFLOW: drift multiplication overflow")? / 1_000_000_000_i128;
    let mapped = (local_ns as i128)
        .checked_add(map.offset_ns as i128)
        .and_then(|v| v.checked_add(drift))
        .ok_or("CLOCK_MAP_OVERFLOW: mapped timestamp overflow")?;
    i64::try_from(mapped).map_err(|_| "CLOCK_MAP_OVERFLOW: mapped timestamp outside i64".into())
}

fn validate_meta(meta: SensorMeta<'_>) -> Result<(), String> {
    if meta.sensor_id.trim().is_empty() || meta.clock_domain.trim().is_empty()
        || meta.calibration_id.trim().is_empty() {
        return Err("METADATA_INVALID: sensor, clock and calibration identities are required".into());
    }
    if !meta.nominal_rate_hz.is_finite() || meta.nominal_rate_hz <= 0.0
        || meta.nominal_rate_hz > 1_000_000.0 {
        return Err("SAMPLE_RATE_INVALID: nominal rate must be in (0, 1e6] Hz".into());
    }
    let valid_unit = match meta.kind {
        ChannelKind::Vibration => meta.unit == "m/s^2",
        ChannelKind::Current => meta.unit == "A",
    };
    if !valid_unit {
        return Err("UNIT_INVALID: v0.1 supports vibration=m/s^2 and current=A only".into());
    }
    if !is_sha256(meta.source_sha256) {
        return Err("SOURCE_DIGEST_INVALID: source SHA-256 must be 64 hex characters".into());
    }
    Ok(())
}

fn extract(raw: RawChannel<'_>, window: Window) -> Result<ExtractedChannel<'_>, String> {
    validate_meta(raw.meta)?;
    if window.end_ref_ns <= window.start_ref_ns {
        return Err("WINDOW_INVALID: require end > start for half-open window".into());
    }
    let map = raw.clock_map.ok_or("INCONCLUSIVE: channel has no clock mapping")?;
    if map.clock_domain != raw.meta.clock_domain {
        return Err("CLOCK_DOMAIN_MISMATCH: map does not describe source channel clock".into());
    }
    if map.uncertainty_ns > MAX_CLOCK_UNCERTAINTY_NS {
        return Err("INCONCLUSIVE: clock mapping uncertainty exceeds 100 us".into());
    }
    if raw.meta.calibration_valid_until_ref_ns < window.end_ref_ns {
        return Err("CALIBRATION_STALE: calibration does not cover the complete feature window".into());
    }
    if raw.timestamps_local_ns.len() != raw.values.len() || raw.values.len() < 2 {
        return Err("SAMPLE_SHAPE_INVALID: timestamps and values must match and contain >=2 samples".into());
    }
    if raw.values.iter().any(|v| !v.is_finite()) {
        return Err("SAMPLE_INVALID: NaN/Inf samples are not silently imputed".into());
    }

    let period_ns = 1.0e9 / raw.meta.nominal_rate_hz;
    if !period_ns.is_finite() || period_ns < 1.0 {
        return Err("SAMPLE_RATE_INVALID: period cannot be represented in nanoseconds".into());
    }

    let mut mapped_all = Vec::with_capacity(raw.timestamps_local_ns.len());
    for (i, &local) in raw.timestamps_local_ns.iter().enumerate() {
        if i > 0 {
            let delta_i128 = (local as i128) - (raw.timestamps_local_ns[i - 1] as i128);
            if delta_i128 <= 0 {
                return Err("TIMESTAMP_ORDER_INVALID: local timestamps must be strictly increasing".into());
            }
            let multiple = ((delta_i128 as f64 / period_ns).round() as i64).max(1);
            let expected_delta = multiple as f64 * period_ns;
            let relative_jitter = (delta_i128 as f64 - expected_delta).abs() / period_ns;
            if relative_jitter > MAX_RELATIVE_SAMPLE_JITTER {
                return Err("TIMESTAMP_JITTER_EXCESSIVE: interval differs from an integer sample period by >10%".into());
            }
        }
        mapped_all.push(map_timestamp(map, local)?);
    }

    for pair in mapped_all.windows(2) {
        if pair[1] <= pair[0] {
            return Err("CLOCK_MAP_NON_MONOTONE: mapping did not preserve timestamp order".into());
        }
    }

    let mut mapped = Vec::new();
    let mut values = Vec::new();
    for (&timestamp, &value) in mapped_all.iter().zip(raw.values.iter()) {
        if timestamp >= window.start_ref_ns && timestamp < window.end_ref_ns {
            mapped.push(timestamp);
            values.push(value);
        }
    }
    if mapped.len() < 2 {
        return Err("WINDOW_EMPTY: fewer than two observed samples in declared window".into());
    }

    let duration_ns = (window.end_ref_ns as i128) - (window.start_ref_ns as i128);
    let expected_f = (duration_ns as f64 / period_ns).round();
    if !expected_f.is_finite() || expected_f < 2.0 || expected_f > usize::MAX as f64 {
        return Err("EXPECTED_SAMPLE_COUNT_INVALID: window/rate combination outside supported range".into());
    }
    let expected = expected_f as usize;
    let missing = expected.saturating_sub(values.len());

    // Count explicit gaps wholly inside the selected window. This complements
    // boundary-aware expected-vs-observed count and never synthesizes samples.
    let mut gap_missing = 0usize;
    for pair in mapped.windows(2) {
        let delta = ((pair[1] as i128) - (pair[0] as i128)) as f64;
        let multiple = ((delta / period_ns).round() as usize).max(1);
        gap_missing = gap_missing.saturating_add(multiple.saturating_sub(1));
    }
    let missing = missing.max(gap_missing).min(expected);

    Ok(ExtractedChannel {
        reference_clock: map.reference_clock,
        meta: raw.meta,
        source_sha256: raw.meta.source_sha256,
        mapped_timestamps_ns: mapped,
        values,
        expected_samples: expected,
        missing_samples: missing,
        clock_uncertainty_ns: map.uncertainty_ns,
        window,
    })
}

fn calculate_features(channel: &ExtractedChannel<'_>) -> Result<Features, String> {
    let x = &channel.values;
    let n = x.len();
    if n < 2 { return Err("FEATURE_INSUFFICIENT_SAMPLES: require >=2 observations".into()); }

    let mean = x.iter().sum::<f64>() / n as f64;
    let mut m2 = 0.0;
    let mut m4 = 0.0;
    let mut sum_sq = 0.0;
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut peak = 0.0_f64;
    for &value in x {
        let delta = value - mean;
        let sq = delta * delta;
        m2 += sq;
        m4 += sq * sq;
        sum_sq += value * value;
        min = min.min(value);
        max = max.max(value);
        peak = peak.max(value.abs());
    }
    let population_variance = m2 / n as f64;
    let unbiased_variance = m2 / (n - 1) as f64;
    let rms = (sum_sq / n as f64).sqrt();
    let excess_kurtosis = if population_variance <= f64::MIN_POSITIVE {
        0.0
    } else {
        (m4 / n as f64) / (population_variance * population_variance) - 3.0
    };
    let missing_fraction = channel.missing_samples as f64 / channel.expected_samples as f64;
    if !mean.is_finite() || !rms.is_finite() || !unbiased_variance.is_finite()
        || !excess_kurtosis.is_finite() {
        return Err("FEATURE_NONFINITE: statistic calculation overflowed".into());
    }

    Ok(Features {
        sensor_id: channel.meta.sensor_id.to_string(),
        kind: match channel.meta.kind { ChannelKind::Vibration => "vibration", ChannelKind::Current => "current" },
        unit: channel.meta.unit.to_string(),
        source_sha256: channel.source_sha256.to_string(),
        window_start_ref_ns: channel.window.start_ref_ns,
        window_end_ref_ns: channel.window.end_ref_ns,
        nominal_rate_hz: channel.meta.nominal_rate_hz,
        observed_samples: n,
        expected_samples: channel.expected_samples,
        missing_samples: channel.missing_samples,
        missing_fraction,
        mean,
        rms,
        peak_to_peak: max - min,
        crest_factor: if rms <= f64::MIN_POSITIVE { 0.0 } else { peak / rms },
        unbiased_variance,
        excess_kurtosis,
        clock_uncertainty_ns: channel.clock_uncertainty_ns,
    })
}

fn paired_exact_rate_pearson(
    a: &ExtractedChannel<'_>,
    b: &ExtractedChannel<'_>,
) -> Result<PairedFeatures, String> {
    if a.window.start_ref_ns != b.window.start_ref_ns || a.window.end_ref_ns != b.window.end_ref_ns {
        return Err("WINDOW_MISMATCH: paired channels must use the same reference window".into());
    }
    if a.reference_clock != b.reference_clock {
        return Err("INCONCLUSIVE: channels do not map to the same reference clock".into());
    }
    if a.meta.sensor_id == b.meta.sensor_id || a.meta.kind == b.meta.kind {
        return Err("PAIR_KIND_INVALID: pair requires distinct vibration and current sensors".into());
    }
    let combined_uncertainty = a.clock_uncertainty_ns.saturating_add(b.clock_uncertainty_ns);
    if combined_uncertainty > MAX_CLOCK_UNCERTAINTY_NS {
        return Err("INCONCLUSIVE: combined clock uncertainty exceeds 100 us".into());
    }
    let rate_scale = a.meta.nominal_rate_hz.abs().max(b.meta.nominal_rate_hz.abs()).max(1.0);
    if (a.meta.nominal_rate_hz - b.meta.nominal_rate_hz).abs() / rate_scale > 1.0e-9 {
        return Err("INCONCLUSIVE: multi-rate pairing requires a pinned anti-alias resampling transform".into());
    }
    if a.mapped_timestamps_ns.len() < 3 || b.mapped_timestamps_ns.len() < 3 {
        return Err("PAIR_INSUFFICIENT_SAMPLES: require at least three samples per channel".into());
    }

    // Exact timestamp intersection only. No nearest-neighbour matching or interpolation.
    let mut i = 0usize;
    let mut j = 0usize;
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    while i < a.mapped_timestamps_ns.len() && j < b.mapped_timestamps_ns.len() {
        let ta = a.mapped_timestamps_ns[i];
        let tb = b.mapped_timestamps_ns[j];
        if ta == tb {
            xs.push(a.values[i]);
            ys.push(b.values[j]);
            i += 1;
            j += 1;
        } else if ta < tb {
            i += 1;
        } else {
            j += 1;
        }
    }
    if xs.len() < 3 {
        return Err("INCONCLUSIVE: fewer than three exact co-timed sample pairs".into());
    }

    let mx = xs.iter().sum::<f64>() / xs.len() as f64;
    let my = ys.iter().sum::<f64>() / ys.len() as f64;
    let mut covariance = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    for (&x, &y) in xs.iter().zip(ys.iter()) {
        let dx = x - mx;
        let dy = y - my;
        covariance += dx * dy;
        vx += dx * dx;
        vy += dy * dy;
    }
    if vx <= f64::MIN_POSITIVE || vy <= f64::MIN_POSITIVE {
        return Err("PAIR_DEGENERATE: Pearson correlation undefined for a constant channel".into());
    }
    let r = covariance / (vx * vy).sqrt();
    if !r.is_finite() { return Err("PAIR_NONFINITE".into()); }
    Ok(PairedFeatures { pair_count: xs.len(), pearson_r: r.clamp(-1.0, 1.0) })
}

fn emit_csv() -> Result<(), String> {
    let n_vib = 1000usize;
    let n_current = 100usize;
    let vib_t: Vec<i64> = (0..n_vib).map(|i| i as i64 * 1_000_000).collect();
    let cur_t: Vec<i64> = (0..n_current).map(|i| i as i64 * 10_000_000).collect();
    let vib_x: Vec<f64> = (0..n_vib).map(|i| 2.0 * (std::f64::consts::TAU * 10.0 * i as f64 / 1000.0).sin()).collect();
    let cur_x: Vec<f64> = (0..n_current).map(|i| 5.0 + 0.5 * (std::f64::consts::TAU * 10.0 * i as f64 / 100.0).sin()).collect();
    let window = Window { start_ref_ns: 0, end_ref_ns: 1_000_000_000 };
    let vib_meta = SensorMeta {
        sensor_id: "fixture-vibration-01", kind: ChannelKind::Vibration, unit: "m/s^2",
        nominal_rate_hz: 1000.0, clock_domain: "vib-local", calibration_id: "cal-vib-01",
        calibration_valid_until_ref_ns: 2_000_000_000,
        source_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    };
    let current_meta = SensorMeta {
        sensor_id: "fixture-current-01", kind: ChannelKind::Current, unit: "A",
        nominal_rate_hz: 100.0, clock_domain: "current-local", calibration_id: "cal-current-01",
        calibration_valid_until_ref_ns: 2_000_000_000,
        source_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    };
    let vib = extract(RawChannel {
        meta: vib_meta, timestamps_local_ns: &vib_t, values: &vib_x,
        clock_map: Some(ClockMap { clock_domain: "vib-local", reference_clock: "cycle-clock",
            offset_ns: 0, origin_local_ns: 0, drift_ppb: 0, uncertainty_ns: 100 }),
    }, window)?;
    let current = extract(RawChannel {
        meta: current_meta, timestamps_local_ns: &cur_t, values: &cur_x,
        clock_map: Some(ClockMap { clock_domain: "current-local", reference_clock: "cycle-clock",
            offset_ns: 0, origin_local_ns: 0, drift_ppb: 0, uncertainty_ns: 100 }),
    }, window)?;
    let vf = calculate_features(&vib)?;
    let cf = calculate_features(&current)?;
    let pair_result = paired_exact_rate_pearson(&current, &vib);
    let pair_status = match &pair_result {
        Ok(_) => "PAIRED_EXACT_RATE".to_string(),
        Err(e) => e.replace(',', ";"),
    };
    let pair = pair_result.ok();

    println!("transform_version,sensor_id,kind,unit,source_sha256,window_start_ref_ns,window_end_ref_ns,nominal_rate_hz,observed_samples,expected_samples,missing_samples,missing_fraction,mean,rms,peak_to_peak,crest_factor,unbiased_variance,excess_kurtosis,clock_uncertainty_ns,paired_sensor_id,paired_status,paired_count,paired_pearson_r");
    for (f, paired_id) in [(&vf, cf.sensor_id.as_str()), (&cf, vf.sensor_id.as_str())] {
        let pair_count = pair.as_ref().map(|p| p.pair_count.to_string()).unwrap_or_default();
        let pair_r = pair.as_ref().map(|p| format!("{:.12e}", p.pearson_r)).unwrap_or_default();
        println!("{},{},{},{},{},{},{},{:.6},{},{},{},{:.8},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{},{},{}",
            TRANSFORM_VERSION, f.sensor_id, f.kind, f.unit, f.source_sha256,
            f.window_start_ref_ns, f.window_end_ref_ns, f.nominal_rate_hz,
            f.observed_samples, f.expected_samples, f.missing_samples, f.missing_fraction,
            f.mean, f.rms, f.peak_to_peak, f.crest_factor, f.unbiased_variance,
            f.excess_kurtosis, f.clock_uncertainty_ns, paired_id, pair_status,
            pair_count, pair_r);
    }
    Ok(())
}

fn verify() -> Result<(), String> {
    let n = 1000usize;
    let timestamps: Vec<i64> = (0..n).map(|i| i as i64 * 1_000_000).collect();
    let values: Vec<f64> = (0..n).map(|i| 2.0 * (std::f64::consts::TAU * 10.0 * i as f64 / 1000.0).sin()).collect();
    let meta = SensorMeta {
        sensor_id: "test-vib", kind: ChannelKind::Vibration, unit: "m/s^2",
        nominal_rate_hz: 1000.0, clock_domain: "local", calibration_id: "cal-1",
        calibration_valid_until_ref_ns: 2_000_000_000,
        source_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    };
    let map = ClockMap { clock_domain: "local", reference_clock: "ref", offset_ns: 0,
        origin_local_ns: 0, drift_ppb: 0, uncertainty_ns: 50 };
    let window = Window { start_ref_ns: 0, end_ref_ns: 1_000_000_000 };
    let extracted = extract(RawChannel { meta, timestamps_local_ns: &timestamps, values: &values, clock_map: Some(map) }, window)?;
    let f = calculate_features(&extracted)?;
    if f.observed_samples != 1000 || f.expected_samples != 1000 || f.missing_samples != 0 {
        return Err("known-answer sample count mismatch".into());
    }
    if f.mean.abs() > 1.0e-12 || (f.rms - 2.0 / 2.0_f64.sqrt()).abs() > 1.0e-12
        || (f.peak_to_peak - 4.0).abs() > 1.0e-12
        || (f.excess_kurtosis + 1.5).abs() > 1.0e-10 {
        return Err("known-answer sinusoid feature mismatch".into());
    }
    if f.clock_uncertainty_ns > MAX_CLOCK_UNCERTAINTY_NS || f.missing_fraction != 0.0 {
        return Err("valid clean channel metadata mismatch".into());
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [] => verify().map(|()| println!("MACH_TELEMETRY_OK: known-answer signal features and clock admission")),
        [a] if a == "--verify" => verify().map(|()| println!("MACH_TELEMETRY_OK: known-answer signal features and clock admission")),
        [a] if a == "--csv" => emit_csv(),
        [a] if a == "--help" || a == "-h" => { println!("Usage: telemetry-reference [--verify|--csv|--help]"); Ok(()) },
        _ => Err("expected --verify, --csv or --help".into())
    };
    if let Err(e) = result { eprintln!("MACH_TELEMETRY_ERROR: {e}"); std::process::exit(1); }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture<'a>(timestamps: &'a [i64], values: &'a [f64]) -> RawChannel<'a> {
        RawChannel {
            meta: SensorMeta { sensor_id:"sensor-1",kind:ChannelKind::Vibration,unit:"m/s^2",
                nominal_rate_hz:1000.0,clock_domain:"local",calibration_id:"cal-1",
                calibration_valid_until_ref_ns:2_000_000_000,
                source_sha256:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
            timestamps_local_ns:timestamps, values,
            clock_map:Some(ClockMap {clock_domain:"local",reference_clock:"ref",offset_ns:0,
                origin_local_ns:0,drift_ppb:0,uncertainty_ns:50})
        }
    }

    #[test]
    fn sinusoid_known_answers() { verify().unwrap(); }

    #[test]
    fn missing_sample_is_reported_not_imputed() {
        let t:Vec<i64>=(0..1000).filter(|i|*i!=300).map(|i|i as i64*1_000_000).collect();
        let x:Vec<f64>=(0..1000).filter(|i|*i!=300).map(|i|2.0*(std::f64::consts::TAU*10.0*i as f64/1000.0).sin()).collect();
        let result=extract(fixture(&t,&x),Window{start_ref_ns:0,end_ref_ns:1_000_000_000}).unwrap();
        let f=calculate_features(&result).unwrap();
        assert_eq!(f.observed_samples,999);
        assert_eq!(f.missing_samples,1);
        assert!((f.missing_fraction-0.001).abs()<1.0e-12);
    }

    #[test]
    fn non_monotone_timestamps_reject() {
        let t=[0,1_000_000,900_000,3_000_000];
        let x=[1.0,2.0,3.0,4.0];
        assert!(extract(fixture(&t,&x),Window{start_ref_ns:0,end_ref_ns:4_000_000}).is_err());
    }

    #[test]
    fn excessive_jitter_rejects() {
        let t=[0,1_300_000,2_000_000,3_000_000];
        let x=[1.0,2.0,3.0,4.0];
        let err=extract(fixture(&t,&x),Window{start_ref_ns:0,end_ref_ns:4_000_000}).unwrap_err();
        assert!(err.contains("TIMESTAMP_JITTER_EXCESSIVE"));
    }

    #[test]
    fn missing_clock_mapping_is_inconclusive() {
        let t=[0,1_000_000,2_000_000];
        let x=[1.0,2.0,3.0];
        let mut r=fixture(&t,&x);
        r.clock_map=None;
        assert!(extract(r,Window{start_ref_ns:0,end_ref_ns:3_000_000}).unwrap_err().contains("INCONCLUSIVE"));
    }

    #[test]
    fn stale_calibration_and_unit_mismatch_reject() {
        let t=[0,1_000_000,2_000_000];
        let x=[1.0,2.0,3.0];
        let mut r=fixture(&t,&x);
        r.meta.calibration_valid_until_ref_ns=1_000_000;
        assert!(extract(r,Window{start_ref_ns:0,end_ref_ns:3_000_000}).unwrap_err().contains("CALIBRATION_STALE"));
        let mut r=fixture(&t,&x);
        r.meta.unit="g";
        assert!(extract(r,Window{start_ref_ns:0,end_ref_ns:3_000_000}).unwrap_err().contains("UNIT_INVALID"));
    }

    #[test]
    fn source_digest_and_nonfinite_samples_reject() {
        let t=[0,1_000_000,2_000_000];
        let x=[1.0,f64::NAN,3.0];
        let mut r=fixture(&t,&x);
        assert!(extract(r,Window{start_ref_ns:0,end_ref_ns:3_000_000}).unwrap_err().contains("SAMPLE_INVALID"));
        let valid=[1.0,2.0,3.0];
        r=fixture(&t,&valid);
        r.meta.source_sha256="not-a-digest";
        assert!(extract(r,Window{start_ref_ns:0,end_ref_ns:3_000_000}).unwrap_err().contains("SOURCE_DIGEST_INVALID"));
    }

    #[test]
    fn cross_channel_feature_requires_clock_alignment() {
        let ta:Vec<i64>=(0..100).map(|i|i*10_000_000).collect();
        let tb:Vec<i64>=(0..1000).map(|i|i*1_000_000).collect();
        let xa:Vec<f64>=(0..100).map(|i|5.0+(std::f64::consts::TAU*10.0*i as f64/100.0).sin()).collect();
        let xb:Vec<f64>=(0..1000).map(|i|(std::f64::consts::TAU*10.0*i as f64/1000.0).sin()).collect();
        let a_meta=SensorMeta {sensor_id:"current",kind:ChannelKind::Current,unit:"A",nominal_rate_hz:100.0,
            clock_domain:"c",calibration_id:"cc",calibration_valid_until_ref_ns:2_000_000_000,
            source_sha256:"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"};
        let b_meta=SensorMeta {sensor_id:"vibration",kind:ChannelKind::Vibration,unit:"m/s^2",nominal_rate_hz:1000.0,
            clock_domain:"v",calibration_id:"vv",calibration_valid_until_ref_ns:2_000_000_000,
            source_sha256:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"};
        let a=extract(RawChannel{meta:a_meta,timestamps_local_ns:&ta,values:&xa,clock_map:Some(ClockMap{clock_domain:"c",reference_clock:"ref",offset_ns:0,origin_local_ns:0,drift_ppb:0,uncertainty_ns:50})},Window{start_ref_ns:0,end_ref_ns:1_000_000_000}).unwrap();
        let b=extract(RawChannel{meta:b_meta,timestamps_local_ns:&tb,values:&xb,clock_map:Some(ClockMap{clock_domain:"v",reference_clock:"ref",offset_ns:0,origin_local_ns:0,drift_ppb:0,uncertainty_ns:50})},Window{start_ref_ns:0,end_ref_ns:1_000_000_000}).unwrap();
        assert_eq!(a.reference_clock, b.reference_clock);
        let err=paired_exact_rate_pearson(&a,&b).unwrap_err();
        assert!(err.contains("multi-rate pairing requires a pinned anti-alias"));
        let mut bad=b;
        bad.clock_uncertainty_ns=MAX_CLOCK_UNCERTAINTY_NS+1;
        assert!(paired_exact_rate_pearson(&a,&bad).unwrap_err().contains("INCONCLUSIVE"));

        let v_same: Vec<f64>=(0..100).map(|i|(std::f64::consts::TAU*10.0*i as f64/100.0).sin()).collect();
        let b_same_meta=SensorMeta {sensor_id:"vibration-same-rate",kind:ChannelKind::Vibration,unit:"m/s^2",nominal_rate_hz:100.0,
            clock_domain:"v",calibration_id:"vv",calibration_valid_until_ref_ns:2_000_000_000,
            source_sha256:"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"};
        let b_same=extract(RawChannel{meta:b_same_meta,timestamps_local_ns:&ta,values:&v_same,clock_map:Some(ClockMap{clock_domain:"v",reference_clock:"ref",offset_ns:0,origin_local_ns:0,drift_ppb:0,uncertainty_ns:50})},Window{start_ref_ns:0,end_ref_ns:1_000_000_000}).unwrap();
        let p=paired_exact_rate_pearson(&a,&b_same).unwrap();
        assert_eq!(p.pair_count,100);
        assert!(p.pearson_r>0.99);
    }

    #[test]
    fn clock_drift_mapping_is_deterministic() {
        let m=ClockMap{clock_domain:"local",reference_clock:"ref",offset_ns:1000,origin_local_ns:0,drift_ppb:1000,uncertainty_ns:10};
        assert_eq!(map_timestamp(m,1_000_000_000).unwrap(),1_000_002_000);
    }
}
