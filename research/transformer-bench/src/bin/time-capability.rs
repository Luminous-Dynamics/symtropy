//! Time-windowed, calibration-aware process capability diagnostics.
//! This intentionally reports descriptive process indices and drift indicators;
//! it does not certify process control, standards conformity, or product release.
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const SAMPLE_HEADER: [&str; 14] = [
    "product_family", "product_id", "component_id", "component_lot_id", "station_id",
    "characteristic", "unit", "window_id", "sample_id", "measured_utc",
    "observed_value", "standard_uncertainty", "instrument_id", "calibration_id",
];
const SPEC_HEADER: [&str; 11] = [
    "product_family", "characteristic", "unit", "lower_limit", "upper_limit",
    "coverage_factor", "max_standard_uncertainty", "calibration_class",
    "min_samples_per_window", "max_window_mean_range_fraction", "max_window_sigma_ratio",
];
const CALIBRATION_HEADER: [&str; 8] = [
    "calibration_id", "instrument_id", "calibration_class", "valid_from_utc",
    "valid_to_utc", "status", "certificate_artifact_id", "certificate_sha256",
];

#[derive(Clone, Debug)]
struct Sample {
    family: String,
    product: String,
    component: String,
    lot: String,
    station: String,
    characteristic: String,
    unit: String,
    window: String,
    id: String,
    measured_at: String,
    value: f64,
    uncertainty: f64,
    instrument: String,
    calibration_id: String,
}
#[derive(Clone, Debug)]
struct Spec {
    family: String,
    characteristic: String,
    unit: String,
    lower: f64,
    upper: f64,
    coverage: f64,
    max_uncertainty: f64,
    calibration_class: String,
    min_samples: usize,
    max_mean_range_fraction: f64,
    max_window_sigma_ratio: f64,
}
#[derive(Clone, Debug)]
struct Calibration {
    instrument: String,
    class: String,
    valid_from: String,
    valid_to: String,
    status: String,
    _certificate_id: String,
}
#[derive(Clone, Debug)]
struct WindowStats {
    start: String,
    mean: f64,
    standard_deviation: f64,
    guarded_cpk: f64,
}
#[derive(Clone, Debug)]
struct Report {
    family: String,
    product: String,
    component: String,
    lot: String,
    station: String,
    characteristic: String,
    unit: String,
    windows: usize,
    valid_samples: usize,
    invalid_metrology_records: usize,
    guarded_exceptions: usize,
    insufficient_windows: usize,
    pooled_cpk: Option<f64>,
    worst_window_cpk: Option<f64>,
    window_mean_range_fraction: Option<f64>,
    max_window_sigma: Option<f64>,
    min_window_sigma: Option<f64>,
    sigma_ratio: Option<f64>,
    drift_warning: bool,
    variance_shift_warning: bool,
    diagnostic: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("time-capability: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 {
        return Err(format!(
            "usage: {} <samples.csv> <specs.csv> <calibrations.csv> <report.csv>",
            args.first().map(String::as_str).unwrap_or("time-capability")
        ));
    }
    let samples = read_samples(&args[1])?;
    let specs = read_specs(&args[2])?;
    let calibrations = read_calibrations(&args[3])?;
    let reports = evaluate(&samples, &specs, &calibrations)?;
    write_report(&args[4], &reports)?;
    for report in &reports {
        eprintln!(
            "product={} characteristic={} windows={} n={} worst_window_guarded_cpk={} drift_warning={} diagnostic={}",
            report.product,
            report.characteristic,
            report.windows,
            report.valid_samples,
            format_optional(report.worst_window_cpk),
            report.drift_warning,
            report.diagnostic
        );
    }
    eprintln!("time-capability complete: reports={}; diagnostics are not release decisions", reports.len());
    Ok(())
}

fn read_table(path: &str, expected: &[&str]) -> Result<Vec<Vec<String>>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing CSV header"))?.trim_end_matches('\r');
    if header != expected.join(",") {
        return Err(format!("{path}: unexpected header; expected {}", expected.join(",")));
    }
    let mut rows = Vec::new();
    for (i, raw) in lines.enumerate() {
        let line_no = i + 2;
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{path}:{line_no}: blank row"));
        }
        if line.contains('"') {
            return Err(format!("{path}:{line_no}: quoted CSV fields are unsupported"));
        }
        let fields: Vec<String> = line.split(',').map(|v| v.trim().to_string()).collect();
        if fields.len() != expected.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, found {}", expected.len(), fields.len()));
        }
        rows.push(fields);
    }
    if rows.is_empty() {
        return Err(format!("{path}: no data rows"));
    }
    Ok(rows)
}

fn required(value: &str, name: &str) -> Result<String, String> {
    if value.trim().is_empty() {
        Err(format!("{name}: empty value"))
    } else {
        Ok(value.trim().to_string())
    }
}

fn number(value: &str, name: &str) -> Result<f64, String> {
    let parsed: f64 = value.parse().map_err(|_| format!("{name}: invalid number {value:?}"))?;
    if parsed.is_finite() {
        Ok(parsed)
    } else {
        Err(format!("{name}: non-finite number"))
    }
}

fn sha256_syntax(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

fn parse_utc(value: &str, context: &str) -> Result<i64, String> {
    let b = value.as_bytes();
    if b.len() != 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T'
        || b[13] != b':' || b[16] != b':' || b[19] != b'Z' {
        return Err(format!("{context}: expected canonical UTC YYYY-MM-DDTHH:MM:SSZ"));
    }
    let part = |range: std::ops::Range<usize>| -> Result<i32, String> {
        if !b[range.clone()].iter().all(u8::is_ascii_digit) {
            return Err(format!("{context}: non-numeric timestamp field"));
        }
        std::str::from_utf8(&b[range]).map_err(|_| format!("{context}: invalid timestamp"))?
            .parse().map_err(|_| format!("{context}: invalid timestamp number"))
    };
    let year = part(0..4)?;
    let month = part(5..7)?;
    let day = part(8..10)?;
    let hour = part(11..13)?;
    let minute = part(14..16)?;
    let second = part(17..19)?;
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return Err(format!("{context}: year/month outside supported range"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if !(1..=days_in_month).contains(&day) || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute) || !(0..=59).contains(&second) {
        return Err(format!("{context}: invalid calendar or clock value"));
    }
    let mut days = 0_i64;
    if year >= 1970 {
        for y in 1970..year {
            days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        }
    } else {
        for y in year..1970 {
            days -= if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        }
    }
    for m in 1..month {
        days += match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => unreachable!(),
        };
    }
    days += i64::from(day - 1);
    Ok(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second))
}

fn read_samples(path: &str) -> Result<Vec<Sample>, String> {
    let rows = read_table(path, &SAMPLE_HEADER)?;
    let mut output = Vec::new();
    let mut ids = BTreeSet::new();
    for (i, f) in rows.iter().enumerate() {
        let row = i + 2;
        let id = required(&f[8], "sample_id")?;
        if !ids.insert(id.clone()) {
            return Err(format!("{path}:{row}: duplicate sample_id {id:?}"));
        }
        let measured_at = required(&f[9], "measured_utc")?;
        parse_utc(&measured_at, "measured_utc")?;
        let uncertainty = number(&f[11], "standard_uncertainty")?;
        if uncertainty < 0.0 {
            return Err(format!("{path}:{row}: standard uncertainty cannot be negative"));
        }
        output.push(Sample {
            family: required(&f[0], "product_family")?,
            product: required(&f[1], "product_id")?,
            component: required(&f[2], "component_id")?,
            lot: required(&f[3], "component_lot_id")?,
            station: required(&f[4], "station_id")?,
            characteristic: required(&f[5], "characteristic")?,
            unit: required(&f[6], "unit")?,
            window: required(&f[7], "window_id")?,
            id,
            measured_at,
            value: number(&f[10], "observed_value")?,
            uncertainty,
            instrument: required(&f[12], "instrument_id")?,
            calibration_id: required(&f[13], "calibration_id")?,
        });
    }
    Ok(output)
}

fn read_specs(path: &str) -> Result<Vec<Spec>, String> {
    let rows = read_table(path, &SPEC_HEADER)?;
    let mut output = Vec::new();
    let mut keys = BTreeSet::new();
    for f in rows {
        let family = required(&f[0], "spec product_family")?;
        let characteristic = required(&f[1], "spec characteristic")?;
        let key = (family.clone(), characteristic.clone());
        if !keys.insert(key) {
            return Err(format!("{path}: duplicate specification {family}/{characteristic}"));
        }
        let lower = number(&f[3], "lower_limit")?;
        let upper = number(&f[4], "upper_limit")?;
        let coverage = number(&f[5], "coverage_factor")?;
        let max_uncertainty = number(&f[6], "max_standard_uncertainty")?;
        let min_samples: usize = f[8].parse().map_err(|_| format!("{path}: min_samples_per_window invalid"))?;
        let max_mean_range_fraction = number(&f[9], "max_window_mean_range_fraction")?;
        let max_window_sigma_ratio = number(&f[10], "max_window_sigma_ratio")?;
        if lower >= upper || coverage <= 0.0 || max_uncertainty <= 0.0
            || min_samples < 2 || !(0.0..=1.0).contains(&max_mean_range_fraction)
            || max_window_sigma_ratio < 1.0 {
            return Err(format!("{path}: invalid limits, uncertainty, sample minimum, or drift threshold"));
        }
        output.push(Spec {
            family,
            characteristic,
            unit: required(&f[2], "spec unit")?,
            lower,
            upper,
            coverage,
            max_uncertainty,
            calibration_class: required(&f[7], "calibration_class")?,
            min_samples,
            max_mean_range_fraction,
            max_window_sigma_ratio,
        });
    }
    Ok(output)
}

fn read_calibrations(path: &str) -> Result<BTreeMap<String, Calibration>, String> {
    let rows = read_table(path, &CALIBRATION_HEADER)?;
    let mut output = BTreeMap::new();
    for f in rows {
        let id = required(&f[0], "calibration_id")?;
        let from = required(&f[3], "valid_from_utc")?;
        let to = required(&f[4], "valid_to_utc")?;
        if parse_utc(&to, "valid_to_utc")? <= parse_utc(&from, "valid_from_utc")? {
            return Err(format!("{path}: reversed calibration interval for {id:?}"));
        }
        if !matches!(f[5].as_str(), "CURRENT" | "EXPIRED" | "SUSPENDED" | "REVOKED") {
            return Err(format!("{path}: invalid calibration status {:?}", f[5]));
        }
        if !sha256_syntax(&f[7]) {
            return Err(format!("{path}: malformed certificate SHA-256 for {id:?}"));
        }
        let value = Calibration {
            instrument: required(&f[1], "instrument_id")?,
            class: required(&f[2], "calibration_class")?,
            valid_from: from,
            valid_to: to,
            status: f[5].clone(),
            _certificate_id: required(&f[6], "certificate_artifact_id")?,
        };
        if output.insert(id.clone(), value).is_some() {
            return Err(format!("{path}: duplicate calibration_id {id:?}"));
        }
    }
    Ok(output)
}

fn calibration_valid(calibration: &Calibration, sample: &Sample, spec: &Spec) -> bool {
    calibration.status == "CURRENT"
        && calibration.instrument == sample.instrument
        && calibration.class == spec.calibration_class
        && sample.measured_at >= calibration.valid_from
        && sample.measured_at <= calibration.valid_to
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn sample_standard_deviation(values: &[f64], center: f64) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let sum_squares = values.iter().map(|value| (value - center).powi(2)).sum::<f64>();
    Some((sum_squares / (values.len() - 1) as f64).sqrt())
}

fn guarded_cpk(values: &[f64], spec: &Spec) -> Option<f64> {
    let center = mean(values);
    let sigma = sample_standard_deviation(values, center)?;
    let guard = spec.coverage * spec.max_uncertainty;
    let lower = spec.lower + guard;
    let upper = spec.upper - guard;
    if lower >= upper || sigma <= 0.0 {
        return None;
    }
    Some(((upper - center).min(center - lower)) / (3.0 * sigma))
}

fn evaluate(
    samples: &[Sample],
    specs: &[Spec],
    calibrations: &BTreeMap<String, Calibration>,
) -> Result<Vec<Report>, String> {
    let mut groups: BTreeMap<(String, String, String, String, String, String), Vec<&Sample>> = BTreeMap::new();
    for sample in samples {
        let spec = specs.iter().find(|s| s.family == sample.family && s.characteristic == sample.characteristic)
            .ok_or_else(|| format!("sample {:?} has no declared characteristic specification", sample.id))?;
        if sample.unit != spec.unit {
            return Err(format!("sample {:?}: unit does not match controlled specification", sample.id));
        }
        let key = (
            sample.family.clone(), sample.product.clone(), sample.component.clone(),
            sample.lot.clone(), sample.station.clone(), sample.characteristic.clone(),
        );
        groups.entry(key).or_default().push(sample);
    }

    let mut reports = Vec::new();
    for ((family, product, component, lot, station, characteristic), group) in groups {
        let spec = specs.iter().find(|s| s.family == family && s.characteristic == characteristic)
            .expect("spec resolved above");
        let mut windows: BTreeMap<String, Vec<&Sample>> = BTreeMap::new();
        let mut seen_calibrated = 0usize;
        let mut invalid_metrology_records = 0usize;
        let mut guarded_exceptions = 0usize;
        let mut valid_samples = Vec::new();

        for sample in &group {
            let valid_cal = calibrations.get(&sample.calibration_id)
                .is_some_and(|cal| calibration_valid(cal, sample, spec));
            if !valid_cal || sample.uncertainty > spec.max_uncertainty {
                invalid_metrology_records += 1;
                continue;
            }
            seen_calibrated += 1;
            let expanded = spec.coverage * sample.uncertainty;
            if sample.value - expanded < spec.lower || sample.value + expanded > spec.upper {
                guarded_exceptions += 1;
            }
            valid_samples.push(sample.value);
            windows.entry(sample.window.clone()).or_default().push(sample);
        }

        let total_windows = group.iter().map(|sample| sample.window.as_str()).collect::<BTreeSet<_>>().len();
        let mut window_stats = Vec::new();
        let mut insufficient_windows = total_windows.saturating_sub(windows.len());
        for (_window_id, mut window) in windows {
            window.sort_by(|a, b| a.measured_at.cmp(&b.measured_at));
            let values: Vec<f64> = window.iter().map(|s| s.value).collect();
            if values.len() < spec.min_samples {
                insufficient_windows += 1;
                continue;
            }
            let center = mean(&values);
            let Some(sd) = sample_standard_deviation(&values, center) else {
                insufficient_windows += 1;
                continue;
            };
            let Some(cpk) = guarded_cpk(&values, spec) else {
                insufficient_windows += 1;
                continue;
            };
            window_stats.push(WindowStats {
                start: window.first().map(|s| s.measured_at.clone()).unwrap_or_default(),
                mean: center,
                standard_deviation: sd,
                guarded_cpk: cpk,
            });
        }
        window_stats.sort_by(|a, b| a.start.cmp(&b.start));

        let pooled_cpk = guarded_cpk(&valid_samples, spec);
        let worst_window_cpk = window_stats.iter().map(|w| w.guarded_cpk).reduce(f64::min);
        let means: Vec<f64> = window_stats.iter().map(|w| w.mean).collect();
        let window_mean_range_fraction = if means.len() >= 2 {
            Some((means.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                - means.iter().copied().fold(f64::INFINITY, f64::min))
                / (spec.upper - spec.lower))
        } else {
            None
        };
        let standard_deviations: Vec<f64> = window_stats.iter().map(|w| w.standard_deviation).collect();
        let max_window_sigma = standard_deviations.iter().copied().reduce(f64::max);
        let min_window_sigma = standard_deviations.iter().copied().reduce(f64::min);
        let sigma_ratio = match (max_window_sigma, min_window_sigma) {
            (Some(max), Some(min)) if min > 0.0 => Some(max / min),
            _ => None,
        };
        let drift_warning = window_mean_range_fraction.is_some_and(|v| v > spec.max_mean_range_fraction);
        let variance_shift_warning = sigma_ratio.is_some_and(|v| v > spec.max_window_sigma_ratio);

        let diagnostic = if invalid_metrology_records > 0 {
            "BLOCKED_INVALID_METROLOGY"
        } else if insufficient_windows > 0 || window_stats.is_empty() {
            "INSUFFICIENT_WINDOW_DATA"
        } else if guarded_exceptions > 0 {
            "GUARDED_MEASUREMENT_EXCEPTIONS"
        } else if drift_warning || variance_shift_warning {
            "TIME_DEPENDENT_DRIFT_OR_VARIANCE_WARNING"
        } else {
            "REVIEW_REQUIRED_PROCESS_STABILITY_NOT_ESTABLISHED"
        };
        reports.push(Report {
            family, product, component, lot, station, characteristic, unit: spec.unit.clone(),
            windows: total_windows, valid_samples: seen_calibrated, invalid_metrology_records,
            guarded_exceptions, insufficient_windows, pooled_cpk, worst_window_cpk,
            window_mean_range_fraction, max_window_sigma, min_window_sigma, sigma_ratio,
            drift_warning, variance_shift_warning, diagnostic: diagnostic.to_string(),
        });
    }
    Ok(reports)
}

fn format_optional(value: Option<f64>) -> String {
    value.map(|v| format!("{v:.6}")).unwrap_or_else(|| "NA".to_string())
}

fn write_report(path: &str, reports: &[Report]) -> Result<(), String> {
    let mut out = String::from(
        "product_family,product_id,component_id,component_lot_id,station_id,characteristic,unit,windows,valid_samples,invalid_metrology_records,guarded_exceptions,insufficient_windows,pooled_guarded_cpk,worst_window_guarded_cpk,window_mean_range_fraction,max_window_sigma,min_window_sigma,max_min_window_sigma_ratio,drift_warning,variance_shift_warning,diagnostic\n"
    );
    for r in reports {
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.family, r.product, r.component, r.lot, r.station, r.characteristic, r.unit,
            r.windows, r.valid_samples, r.invalid_metrology_records, r.guarded_exceptions,
            r.insufficient_windows, format_optional(r.pooled_cpk), format_optional(r.worst_window_cpk),
            format_optional(r.window_mean_range_fraction), format_optional(r.max_window_sigma),
            format_optional(r.min_window_sigma), format_optional(r.sigma_ratio),
            r.drift_warning, r.variance_shift_warning, r.diagnostic
        ));
    }
    fs::write(path, out).map_err(|e| format!("cannot write {path}: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(value: f64) -> Sample {
        Sample {
            family: "POWER_TRANSFORMER".into(), product: "TX-1".into(),
            component: "CORE-1".into(), lot: "GOES-1".into(), station: "CORE-CUT".into(),
            characteristic: "stack_height".into(), unit: "mm".into(), window: "W1".into(),
            id: "S-1".into(), measured_at: "2026-01-01T00:00:00Z".into(),
            value, uncertainty: 0.01, instrument: "DIM-1".into(), calibration_id: "CAL-1".into(),
        }
    }

    fn spec() -> Spec {
        Spec {
            family: "POWER_TRANSFORMER".into(), characteristic: "stack_height".into(),
            unit: "mm".into(), lower: 99.0, upper: 101.0, coverage: 2.0,
            max_uncertainty: 0.02, calibration_class: "LENGTH".into(),
            min_samples: 4, max_mean_range_fraction: 0.1, max_window_sigma_ratio: 3.0,
        }
    }

    #[test]
    fn guarded_capability_uses_worst_window_and_conservative_total_variation() {
        let spec = spec();
        let values = [100.0, 100.02, 99.98, 100.01];
        let cpk = guarded_cpk(&values, &spec).unwrap();
        assert!(cpk > 1.33);
        assert!(cpk.is_finite());
        assert_eq!(guarded_cpk(&[100.0, 100.0, 100.0], &spec), None);
    }

    #[test]
    fn parse_utc_rejects_impossible_dates_and_non_utc_offsets() {
        assert!(parse_utc("2024-02-29T23:59:59Z", "test").is_ok());
        assert!(parse_utc("2025-02-29T23:59:59Z", "test").is_err());
        assert!(parse_utc("2026-01-01T00:00:00+02:00", "test").is_err());
    }

    #[test]
    fn calibration_must_match_instrument_class_status_and_time() {
        let mut sample = sample(100.0);
        let cal = Calibration {
            instrument: "DIM-1".into(), class: "LENGTH".into(),
            valid_from: "2025-01-01T00:00:00Z".into(),
            valid_to: "2027-01-01T00:00:00Z".into(), status: "CURRENT".into(),
            _certificate_id: "CERT-1".into(),
        };
        assert!(calibration_valid(&cal, &sample, &spec()));
        sample.instrument = "OTHER".into();
        assert!(!calibration_valid(&cal, &sample, &spec()));
    }

    fn synthetic_calibration(status: &str) -> Calibration {
        Calibration {
            instrument: "DIM-1".into(),
            class: "LENGTH_GAUGE".into(),
            valid_from: "2025-01-01T00:00:00Z".into(),
            valid_to: "2027-12-31T23:59:59Z".into(),
            status: status.into(),
            _certificate_id: "CERT-1".into(),
        }
    }

    fn synthetic_window_samples(windows: &[(&str, &[f64])]) -> Vec<Sample> {
        let mut output = Vec::new();
        let mut index = 0usize;
        for (window_index, (window_id, values)) in windows.iter().enumerate() {
            for (sample_index, value) in values.iter().enumerate() {
                index += 1;
                let mut item = sample(*value);
                item.window = (*window_id).to_string();
                item.id = format!("S-{index}");
                item.measured_at = format!(
                    "2026-02-{:02}T{:02}:00:00Z",
                    window_index + 1,
                    7 + sample_index
                );
                output.push(item);
            }
        }
        output
    }

    #[test]
    fn windowed_diagnostic_detects_mean_drift_that_pooled_cpk_could_hide() {
        let samples = synthetic_window_samples(&[
            ("W1", &[99.88, 99.89, 99.87, 99.88]),
            ("W2", &[99.99, 100.00, 100.01, 100.00]),
            ("W3", &[100.10, 100.11, 100.12, 100.10]),
        ]);
        let reports = evaluate(
            &samples,
            &[spec()],
            &BTreeMap::from([("CAL-1".to_string(), synthetic_calibration("CURRENT"))]),
        )
        .unwrap();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].windows, 3);
        assert_eq!(reports[0].valid_samples, 12);
        assert!(reports[0].drift_warning);
        assert_eq!(reports[0].diagnostic, "TIME_DEPENDENT_DRIFT_OR_VARIANCE_WARNING");
        assert!(reports[0].pooled_cpk.is_some());
        assert!(reports[0].worst_window_cpk.is_some());
    }

    #[test]
    fn windowed_diagnostic_detects_variance_shift_even_when_window_means_match() {
        let samples = synthetic_window_samples(&[
            ("W1", &[99.995, 100.000, 100.005, 100.000]),
            ("W2", &[99.950, 100.000, 100.050, 100.000]),
            ("W3", &[99.800, 100.000, 100.200, 100.000]),
        ]);
        let reports = evaluate(
            &samples,
            &[spec()],
            &BTreeMap::from([("CAL-1".to_string(), synthetic_calibration("CURRENT"))]),
        )
        .unwrap();
        assert!(!reports[0].drift_warning);
        assert!(reports[0].variance_shift_warning);
        assert!(reports[0].sigma_ratio.unwrap() > 3.0);
    }

    #[test]
    fn expired_calibration_blocks_time_window_statistics() {
        let samples = synthetic_window_samples(&[
            ("W1", &[100.00, 100.01, 99.99, 100.00]),
            ("W2", &[100.01, 100.00, 99.99, 100.02]),
            ("W3", &[99.99, 100.00, 100.01, 100.00]),
        ]);
        let reports = evaluate(
            &samples,
            &[spec()],
            &BTreeMap::from([("CAL-1".to_string(), synthetic_calibration("EXPIRED"))]),
        )
        .unwrap();
        assert_eq!(reports[0].valid_samples, 0);
        assert_eq!(reports[0].invalid_metrology_records, 12);
        assert_eq!(reports[0].diagnostic, "BLOCKED_INVALID_METROLOGY");
    }

    #[test]
    fn mean_and_sample_standard_deviation_are_hand_checkable() {
        let values = [1.0, 2.0, 3.0];
        assert!((mean(&values) - 2.0).abs() < 1.0e-12);
        assert!((sample_standard_deviation(&values, 2.0).unwrap() - 1.0).abs() < 1.0e-12);
    }
}
