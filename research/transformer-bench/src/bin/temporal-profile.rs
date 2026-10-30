//! Validates synchronized annual operating scenarios for transformer temporal models.
//! This validates chronology and joint input semantics; it does not run a thermal/ageing model.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 19] = [
    "scenario_id", "scenario_weight", "profile_id", "trace_year", "source_timezone",
    "interval_id", "start_utc", "end_utc", "current_pu_mean", "mean_square_current_pu2",
    "ambient_temp_c", "ev_load_pu", "pv_generation_pu", "price_per_kwh",
    "currency_code", "price_year", "price_basis", "carbon_intensity_kgco2e_per_kwh",
    "source_trace_id",
];
const WEIGHT_TOLERANCE: f64 = 1.0e-9;
const MOMENT_TOLERANCE: f64 = 1.0e-9;
const HOURS_TOLERANCE: f64 = 1.0e-8;

#[derive(Clone, Debug)]
struct Metadata {
    profile_id: String,
    trace_year: i32,
    source_timezone: String,
    currency_code: String,
    price_year: i32,
    price_basis: String,
}

#[derive(Clone, Debug)]
struct Interval {
    id: String,
    start: i64,
    end: i64,
    current_mean: f64,
    current_mean_square: f64,
    ambient_temp_c: f64,
    ev_load_pu: f64,
    pv_generation_pu: f64,
    price_per_kwh: f64,
    carbon_intensity: f64,
    source_trace_id: String,
}

#[derive(Clone, Debug)]
struct Scenario {
    weight: f64,
    intervals: Vec<Interval>,
    interval_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Summary {
    scenario_id: String,
    weight: f64,
    interval_count: usize,
    total_hours: f64,
    mean_current_pu: f64,
    mean_square_current_pu2: f64,
    mean_ambient_temp_c: f64,
    max_ambient_temp_c: f64,
    mean_ev_load_pu: f64,
    mean_pv_generation_pu: f64,
    mean_price_per_kwh: f64,
    mean_carbon_intensity: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("temporal-profile: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <synchronized-profile.csv> <scenario-summary.csv>",
            args.first().map(String::as_str).unwrap_or("temporal-profile")
        ));
    }
    let (metadata, scenarios) = read_scenarios(&args[1])?;
    let summaries = summarize(&scenarios)?;
    write_summary(&args[2], &metadata, &summaries)?;
    eprintln!(
        "temporal operating-profile validation passed: scenarios={}, trace_year={}, UTC intervals/scenario={}, profile={}, timezone_label={}, tariff={}/{} {}; chronology and joint interval alignment validated; timezone conversion/provenance not established",
        summaries.len(),
        metadata.trace_year,
        summaries.first().map(|row| row.interval_count).unwrap_or(0),
        metadata.profile_id,
        metadata.source_timezone,
        metadata.currency_code,
        metadata.price_year,
        metadata.price_basis
    );
    Ok(())
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { Err(format!("{context}: empty value is not allowed")) }
    else { Ok(value.to_string()) }
}

fn number(value: &str, context: &str) -> Result<f64, String> {
    let parsed: f64 = value.trim().parse()
        .map_err(|_| format!("{context}: expected finite number, got {value:?}"))?;
    if !parsed.is_finite() { return Err(format!("{context}: non-finite number")); }
    Ok(parsed)
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_before_year(year: i32) -> i64 {
    let mut days = 0_i64;
    if year >= 1970 {
        for current in 1970..year {
            days += if is_leap_year(current) { 366 } else { 365 };
        }
    } else {
        for current in year..1970 {
            days -= if is_leap_year(current) { 366 } else { 365 };
        }
    }
    days
}

fn parse_utc(value: &str, context: &str) -> Result<i64, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-' || bytes[7] != b'-'
        || bytes[10] != b'T' || bytes[13] != b':'
        || bytes[16] != b':' || bytes[19] != b'Z'
    {
        return Err(format!("{context}: timestamp must be YYYY-MM-DDTHH:MM:SSZ"));
    }
    let digits = |range: std::ops::Range<usize>| -> Result<i32, String> {
        let slice = &bytes[range.clone()];
        if !slice.iter().all(u8::is_ascii_digit) {
            return Err(format!("{context}: timestamp contains non-digit fields"));
        }
        let text = std::str::from_utf8(slice).map_err(|_| format!("{context}: invalid timestamp"))?;
        text.parse().map_err(|_| format!("{context}: invalid timestamp number"))
    };
    let year = digits(0..4)?;
    let month = digits(5..7)?;
    let day = digits(8..10)?;
    let hour = digits(11..13)?;
    let minute = digits(14..16)?;
    let second = digits(17..19)?;
    if !(1900..=2200).contains(&year) || !(1..=12).contains(&month) {
        return Err(format!("{context}: year/month out of supported range"));
    }
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => unreachable!(),
    };
    if !(1..=month_days).contains(&day)
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return Err(format!("{context}: invalid calendar/time value"));
    }
    let mut days = days_before_year(year);
    for m in 1..month {
        days += match m {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => unreachable!(),
        };
    }
    days += i64::from(day - 1);
    Ok(days * 86_400 + i64::from(hour * 3_600 + minute * 60 + second))
}

fn read_scenarios(path: &str) -> Result<(Metadata, BTreeMap<String, Scenario>), String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let mut lines = content.lines();
    let header = lines.next().ok_or_else(|| format!("{path}: missing header"))?.trim_end_matches('\r');
    if header.split(',').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(format!("{path}: expected header [{}]", HEADER.join(",")));
    }

    let mut metadata: Option<Metadata> = None;
    let mut scenarios: BTreeMap<String, Scenario> = BTreeMap::new();
    let mut weights: BTreeMap<String, f64> = BTreeMap::new();

    for (index, raw_line) in lines.enumerate() {
        let line_no = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() { return Err(format!("{path}:{line_no}: blank rows are not allowed")); }
        if line.contains('"') { return Err(format!("{path}:{line_no}: quoted fields are not supported")); }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!("{path}:{line_no}: expected {} fields, got {}", HEADER.len(), fields.len()));
        }

        let scenario_id = nonempty(fields[0], "scenario_id")?;
        let scenario_weight = number(fields[1], "scenario_weight")?;
        if !(0.0..=1.0).contains(&scenario_weight) || scenario_weight == 0.0 {
            return Err(format!("{path}:{line_no}: scenario_weight must be in (0, 1]"));
        }
        let row_metadata = Metadata {
            profile_id: nonempty(fields[2], "profile_id")?,
            trace_year: fields[3].trim().parse().map_err(|_| format!("{path}:{line_no}: trace_year must be an integer"))?,
            source_timezone: nonempty(fields[4], "source_timezone")?,
            currency_code: nonempty(fields[14], "currency_code")?,
            price_year: fields[15].trim().parse().map_err(|_| format!("{path}:{line_no}: price_year must be an integer"))?,
            price_basis: nonempty(fields[16], "price_basis")?,
        };
        if !(1900..=2199).contains(&row_metadata.trace_year) || !(1900..=2200).contains(&row_metadata.price_year) {
            return Err(format!("{path}:{line_no}: trace_year and price_year must be in 1900..=2200"));
        }
        if row_metadata.currency_code.len() != 3 || !row_metadata.currency_code.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(format!("{path}:{line_no}: currency_code must be three uppercase ASCII letters (syntax only)"));
        }
        if !matches!(row_metadata.price_basis.as_str(), "REAL" | "NOMINAL") {
            return Err(format!("{path}:{line_no}: price_basis must be REAL or NOMINAL"));
        }
        if let Some(first) = metadata.as_ref() {
            if first.profile_id != row_metadata.profile_id
                || first.trace_year != row_metadata.trace_year
                || first.source_timezone != row_metadata.source_timezone
                || first.currency_code != row_metadata.currency_code
                || first.price_year != row_metadata.price_year
                || first.price_basis != row_metadata.price_basis
            {
                return Err(format!("{path}:{line_no}: profile, trace year, timezone label and tariff metadata must match across all scenarios"));
            }
        } else {
            metadata = Some(row_metadata);
        }

        match weights.get(&scenario_id) {
            Some(previous) if (previous - scenario_weight).abs() > WEIGHT_TOLERANCE => {
                return Err(format!("{path}:{line_no}: inconsistent scenario weight for {scenario_id:?}"));
            }
            None => { weights.insert(scenario_id.clone(), scenario_weight); }
            _ => {}
        }

        let interval_id = nonempty(fields[5], "interval_id")?;
        let start = parse_utc(fields[6], &format!("{path}:{line_no}: start_utc"))?;
        let end = parse_utc(fields[7], &format!("{path}:{line_no}: end_utc"))?;
        if end <= start { return Err(format!("{path}:{line_no}: end_utc must be after start_utc")); }
        let current_mean = number(fields[8], "current_pu_mean")?;
        let current_mean_square = number(fields[9], "mean_square_current_pu2")?;
        let ambient_temp_c = number(fields[10], "ambient_temp_c")?;
        let ev_load_pu = number(fields[11], "ev_load_pu")?;
        let pv_generation_pu = number(fields[12], "pv_generation_pu")?;
        let price_per_kwh = number(fields[13], "price_per_kwh")?;
        let carbon_intensity = number(fields[17], "carbon_intensity_kgco2e_per_kwh")?;
        let source_trace_id = nonempty(fields[18], "source_trace_id")?;
        if current_mean < 0.0 || current_mean_square < 0.0 || ev_load_pu < 0.0
            || pv_generation_pu < 0.0 || price_per_kwh < 0.0 || carbon_intensity < 0.0
        {
            return Err(format!("{path}:{line_no}: current, EV/PV, price and carbon intensity must be non-negative"));
        }
        let square = current_mean * current_mean;
        if !square.is_finite() {
            return Err(format!("{path}:{line_no}: current mean too large to square safely"));
        }
        let tolerance = MOMENT_TOLERANCE * square.max(current_mean_square).max(1.0);
        if current_mean_square + tolerance < square {
            return Err(format!("{path}:{line_no}: mean_square_current_pu2 must be >= current_pu_mean squared"));
        }

        let scenario = scenarios.entry(scenario_id.clone()).or_insert_with(|| Scenario {
            weight: scenario_weight,
            intervals: Vec::new(),
            interval_ids: BTreeSet::new(),
        });
        if !scenario.interval_ids.insert(interval_id.clone()) {
            return Err(format!("{path}:{line_no}: duplicate interval_id {interval_id:?} in scenario {scenario_id:?}"));
        }
        scenario.intervals.push(Interval {
            id: interval_id, start, end, current_mean, current_mean_square,
            ambient_temp_c, ev_load_pu, pv_generation_pu, price_per_kwh,
            carbon_intensity, source_trace_id,
        });
    }

    if scenarios.len() < 2 { return Err(format!("{path}: at least two scenarios are required")); }
    let total_weight: f64 = weights.values().sum();
    if !total_weight.is_finite() || (total_weight - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(format!("{path}: scenario weights must sum to 1.0; got {total_weight}"));
    }
    validate_common_utc_grid(path, &scenarios, metadata.as_ref().ok_or_else(|| format!("{path}: no metadata"))?)?;
    Ok((metadata.unwrap(), scenarios))
}

fn validate_common_utc_grid(path: &str, scenarios: &BTreeMap<String, Scenario>, metadata: &Metadata) -> Result<(), String> {
    let year_start = parse_utc(&format!("{:04}-01-01T00:00:00Z", metadata.trace_year), "year start")?;
    let next_year_start = parse_utc(&format!("{:04}-01-01T00:00:00Z", metadata.trace_year + 1), "next year start")?;
    let expected_hours = if is_leap_year(metadata.trace_year) { 8_784.0 } else { 8_760.0 };
    let mut reference_grid: Option<Vec<(i64, i64)>> = None;

    for (scenario_id, scenario) in scenarios {
        let mut intervals = scenario.intervals.clone();
        intervals.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.end.cmp(&b.end)));
        if intervals.first().map(|row| row.start) != Some(year_start)
            || intervals.last().map(|row| row.end) != Some(next_year_start)
        {
            return Err(format!("{path}: scenario {scenario_id:?} must cover the complete UTC calendar year"));
        }
        let mut hours = 0.0;
        let mut grid = Vec::with_capacity(intervals.len());
        for (index, interval) in intervals.iter().enumerate() {
            if index > 0 {
                let previous = &intervals[index - 1];
                if interval.start < previous.end {
                    return Err(format!("{path}: scenario {scenario_id:?} has overlapping intervals"));
                }
                if interval.start > previous.end {
                    return Err(format!("{path}: scenario {scenario_id:?} has a gap between intervals"));
                }
            }
            hours += (interval.end - interval.start) as f64 / 3_600.0;
            grid.push((interval.start, interval.end));
        }
        if !hours.is_finite() || (hours - expected_hours).abs() > HOURS_TOLERANCE {
            return Err(format!("{path}: scenario {scenario_id:?} covers {hours} hours, expected {expected_hours}"));
        }
        if let Some(reference) = reference_grid.as_ref() {
            if reference != &grid {
                return Err(format!("{path}: scenario {scenario_id:?} has a different UTC interval grid; joint scenarios must be aligned"));
            }
        } else {
            reference_grid = Some(grid);
        }
    }
    Ok(())
}

fn summarize(scenarios: &BTreeMap<String, Scenario>) -> Result<Vec<Summary>, String> {
    let mut output = Vec::new();
    for (scenario_id, scenario) in scenarios {
        let mut total_hours = 0.0;
        let mut weighted = [0.0; 7];
        let mut max_ambient_temp_c = f64::NEG_INFINITY;
        let mut ordered_intervals = scenario.intervals.clone();
        ordered_intervals.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.end.cmp(&b.end)));
        for interval in &ordered_intervals {
            let hours = (interval.end - interval.start) as f64 / 3_600.0;
            total_hours += hours;
            weighted[0] += interval.current_mean * hours;
            weighted[1] += interval.current_mean_square * hours;
            weighted[2] += interval.ambient_temp_c * hours;
            weighted[3] += interval.ev_load_pu * hours;
            weighted[4] += interval.pv_generation_pu * hours;
            weighted[5] += interval.price_per_kwh * hours;
            weighted[6] += interval.carbon_intensity * hours;
            max_ambient_temp_c = max_ambient_temp_c.max(interval.ambient_temp_c);
        }
        if !total_hours.is_finite() || total_hours <= 0.0
            || weighted.iter().any(|value| !value.is_finite())
            || !max_ambient_temp_c.is_finite()
        {
            return Err(format!("scenario {scenario_id:?}: derived temporal summary is non-finite"));
        }
        output.push(Summary {
            scenario_id: scenario_id.clone(),
            weight: scenario.weight,
            interval_count: scenario.intervals.len(),
            total_hours,
            mean_current_pu: weighted[0] / total_hours,
            mean_square_current_pu2: weighted[1] / total_hours,
            mean_ambient_temp_c: weighted[2] / total_hours,
            max_ambient_temp_c,
            mean_ev_load_pu: weighted[3] / total_hours,
            mean_pv_generation_pu: weighted[4] / total_hours,
            mean_price_per_kwh: weighted[5] / total_hours,
            mean_carbon_intensity: weighted[6] / total_hours,
        });
    }
    Ok(output)
}

fn write_summary(path: &str, metadata: &Metadata, summaries: &[Summary]) -> Result<(), String> {
    let mut output = String::from("profile_id,trace_year,source_timezone,currency_code,price_year,price_basis,scenario_id,scenario_weight,interval_count,total_hours,mean_current_pu,mean_square_current_pu2,mean_ambient_temp_c,max_ambient_temp_c,mean_ev_load_pu,mean_pv_generation_pu,mean_price_per_kwh,mean_carbon_intensity_kgco2e_per_kwh\n");
    for row in summaries {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{:.9},{},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9},{:.9}\n",
            metadata.profile_id, metadata.trace_year, metadata.source_timezone, metadata.currency_code,
            metadata.price_year, metadata.price_basis, row.scenario_id, row.weight,
            row.interval_count, row.total_hours, row.mean_current_pu, row.mean_square_current_pu2,
            row.mean_ambient_temp_c, row.max_ambient_temp_c, row.mean_ev_load_pu,
            row.mean_pv_generation_pu, row.mean_price_per_kwh, row.mean_carbon_intensity
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_parser_handles_leap_day_and_rejects_bad_calendar_times() {
        assert!(parse_utc("2024-02-29T00:00:00Z", "test").is_ok());
        assert!(parse_utc("2025-02-29T00:00:00Z", "test").is_err());
        assert!(parse_utc("2025-04-31T00:00:00Z", "test").is_err());
        assert!(parse_utc("2025-01-01T24:00:00Z", "test").is_err());
        assert!(parse_utc("2025-01-01T00:00:00+02:00", "test").is_err());
    }

    #[test]
    fn current_second_moment_must_dominate_squared_mean() {
        let path = env::temp_dir().join("temporal-profile-moment-test.csv");
        let path = path.to_string_lossy().into_owned();
        let header = HEADER.join(",");
        let row = "s1,0.5,P,2025,Africa/Johannesburg,i1,2025-01-01T00:00:00Z,2025-01-01T01:00:00Z,0.5,0.2,25,0,0,2,ZAR,2026,REAL,0.8,src";
        fs::write(&path, format!("{header}\n{row}\n")).unwrap();
        assert!(read_scenarios(&path).unwrap_err().contains("mean_square_current_pu2"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn checked_fixture_rejects_a_shifted_scenario_grid() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/temporal-profile-leap-year-synthetic.csv");
        let content = fs::read_to_string(&path).unwrap();
        let shifted = content
            .replacen(
                "stress,0.2,SA-MUNI-11KV-50HZ,2024,Africa/Johannesburg,Q1,2024-01-01T00:00:00Z,2024-04-01T00:00:00Z",
                "stress,0.2,SA-MUNI-11KV-50HZ,2024,Africa/Johannesburg,Q1,2024-01-01T00:00:00Z,2024-04-01T00:15:00Z",
                1,
            )
            .replacen(
                "stress,0.2,SA-MUNI-11KV-50HZ,2024,Africa/Johannesburg,Q2,2024-04-01T00:00:00Z",
                "stress,0.2,SA-MUNI-11KV-50HZ,2024,Africa/Johannesburg,Q2,2024-04-01T00:15:00Z",
                1,
            );
        let temp = env::temp_dir().join("temporal-profile-shifted-grid.csv");
        fs::write(&temp, shifted).unwrap();
        assert!(read_scenarios(temp.to_str().unwrap()).unwrap_err().contains("different UTC interval grid"));
        let _ = fs::remove_file(temp);
    }

    #[test]
    fn fixture_has_aligned_leap_year_scenarios_and_summaries() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/temporal-profile-leap-year-synthetic.csv");
        let (_, scenarios) = read_scenarios(path.to_str().unwrap()).unwrap();
        assert_eq!(scenarios.len(), 2);
        assert!(scenarios.values().all(|scenario| scenario.intervals.len() == 4));
        let summary = summarize(&scenarios).unwrap();
        assert_eq!(summary.len(), 2);
        assert!(summary.iter().all(|row| (row.total_hours - 8_784.0).abs() < HOURS_TOLERANCE));
    }
}
