//! Global annual-profile integrity validator for transformer engineering inputs.
//! This validates chronology and metadata consistency, not meter calibration, tariff legality,
//! IANA timezone correctness, physical-model accuracy, or transformer qualification.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process;

const HEADER: [&str; 14] = [
    "scenario_id",
    "weight",
    "period_start_utc",
    "period_end_utc",
    "source_timezone",
    "interval_id",
    "start_utc",
    "end_utc",
    "current_pu_mean",
    "mean_square_current_pu2",
    "price_per_kwh",
    "currency_code",
    "price_year",
    "price_basis",
];
const WEIGHT_TOLERANCE: f64 = 1.0e-9;
const MOMENT_REL_TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct UtcStamp {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
    epoch_seconds: i64,
}

#[derive(Clone, Debug)]
struct TraceInterval {
    interval_id: String,
    start: UtcStamp,
    end: UtcStamp,
    current_pu_mean: f64,
    mean_square_current_pu2: f64,
    price_per_kwh: f64,
}

#[derive(Clone, Debug)]
struct TraceScenario {
    scenario_id: String,
    weight: f64,
    period_start_utc: String,
    period_end_utc: String,
    period_start: UtcStamp,
    period_end: UtcStamp,
    source_timezone: String,
    currency_code: String,
    price_year: i32,
    price_basis: String,
    intervals: Vec<TraceInterval>,
    interval_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct ScenarioSummary {
    scenario_id: String,
    weight: f64,
    source_timezone: String,
    period_start_utc: String,
    period_end_utc: String,
    interval_count: usize,
    annual_hours: u32,
    currency_code: String,
    price_year: i32,
    price_basis: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("global-trace-validate: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        return Err(format!(
            "usage: {} <annual-traces.csv> <summary.csv>",
            args.first().map(String::as_str).unwrap_or("global-trace-validate")
        ));
    }
    let content =
        fs::read_to_string(&args[1]).map_err(|error| format!("cannot read {}: {error}", args[1]))?;
    let scenarios = parse_trace(&content, &args[1])?;
    let summaries = summarize(&scenarios);
    write_report(&args[2], &summaries)?;
    eprintln!(
        "UTC annual profile integrity passed: scenarios={}, intervals={}, annual-hours include leap-year handling; source timezone names and source measurement quality are not independently verified",
        summaries.len(),
        summaries.iter().map(|row| row.interval_count).sum::<usize>()
    );
    Ok(())
}

fn parse_number(value: &str, context: &str) -> Result<f64, String> {
    let number: f64 = value
        .trim()
        .parse()
        .map_err(|_| format!("{context}: expected a finite number, got {value:?}"))?;
    if !number.is_finite() {
        return Err(format!("{context}: non-finite values are not allowed"));
    }
    Ok(number)
}

fn nonempty(value: &str, context: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{context}: empty value is not allowed"))
    } else {
        Ok(value.to_string())
    }
}

fn parse_digits(bytes: &[u8], start: usize, len: usize, context: &str) -> Result<u32, String> {
    let slice = bytes
        .get(start..start + len)
        .ok_or_else(|| format!("{context}: invalid UTC timestamp"))?;
    if !slice.iter().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{context}: timestamp fields must be decimal digits"));
    }
    let text = std::str::from_utf8(slice)
        .map_err(|_| format!("{context}: timestamp fields are not ASCII"))?;
    text.parse()
        .map_err(|_| format!("{context}: invalid timestamp field"))
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> Option<u32> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 if is_leap_year(year) => Some(29),
        2 => Some(28),
        _ => None,
    }
}

fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    // Howard Hinnant's civil-date algorithm; only years 1900..=2199 are admitted here.
    let mut y = i64::from(year);
    let m = i64::from(month);
    y -= if m <= 2 { 1 } else { 0 };
    let era = y / 400;
    let year_of_era = y - era * 400;
    let adjusted_month = m + if m > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + i64::from(day) - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn parse_utc_timestamp(value: &str, context: &str) -> Result<UtcStamp, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || bytes.get(19) != Some(&b'Z')
    {
        return Err(format!(
            "{context}: expected canonical UTC timestamp YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    let year = parse_digits(bytes, 0, 4, context)? as i32;
    let month = parse_digits(bytes, 5, 2, context)?;
    let day = parse_digits(bytes, 8, 2, context)?;
    let hour = parse_digits(bytes, 11, 2, context)?;
    let minute = parse_digits(bytes, 14, 2, context)?;
    let second = parse_digits(bytes, 17, 2, context)?;

    if !(1900..=2200).contains(&year) {
        return Err(format!("{context}: timestamp year must be in 1900..=2200"));
    }
    let max_day =
        days_in_month(year, month).ok_or_else(|| format!("{context}: invalid month {month}"))?;
    if day == 0 || day > max_day {
        return Err(format!("{context}: invalid day {day} for {year:04}-{month:02}"));
    }
    if hour > 23 || minute > 59 || second > 59 {
        return Err(format!("{context}: invalid clock time"));
    }

    let epoch_seconds = days_from_civil(year, month, day) * 86_400
        + i64::from(hour) * 3_600
        + i64::from(minute) * 60
        + i64::from(second);
    Ok(UtcStamp {
        year,
        month,
        day,
        hour,
        minute,
        second,
        epoch_seconds,
    })
}

fn validate_annual_period(start: UtcStamp, end: UtcStamp, context: &str) -> Result<u32, String> {
    if start.month != 1 || start.day != 1 || start.hour != 0 || start.minute != 0 || start.second != 0 {
        return Err(format!("{context}: period_start_utc must be January 1 at 00:00:00Z"));
    }
    if end.year != start.year + 1
        || end.month != 1
        || end.day != 1
        || end.hour != 0
        || end.minute != 0
        || end.second != 0
    {
        return Err(format!("{context}: period_end_utc must be January 1 of the next year at 00:00:00Z"));
    }
    let seconds = end.epoch_seconds - start.epoch_seconds;
    let expected_hours = if is_leap_year(start.year) { 8_784 } else { 8_760 };
    if seconds != i64::from(expected_hours) * 3_600 {
        return Err(format!("{context}: calendar-year duration did not match leap-year rules"));
    }
    Ok(expected_hours)
}

fn valid_currency_syntax(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|byte| byte.is_ascii_uppercase())
}

fn parse_trace(input: &str, source: &str) -> Result<BTreeMap<String, TraceScenario>, String> {
    let mut lines = input.lines();
    let header = lines
        .next()
        .ok_or_else(|| format!("{source}: missing CSV header"))?
        .trim_end_matches('\r');
    let header_columns: Vec<&str> = header.split(',').collect();
    if header_columns.as_slice() != HEADER.as_slice() {
        return Err(format!(
            "{source}: expected header [{}], got [{}]",
            HEADER.join(","),
            header_columns.join(",")
        ));
    }

    let mut scenarios: BTreeMap<String, TraceScenario> = BTreeMap::new();
    for (index, raw_line) in lines.enumerate() {
        let line_number = index + 2;
        let line = raw_line.trim_end_matches('\r');
        if line.trim().is_empty() {
            return Err(format!("{source}:{line_number}: blank rows are not allowed"));
        }
        if line.contains('"') {
            return Err(format!("{source}:{line_number}: quoted CSV fields are not supported"));
        }
        let fields: Vec<&str> = line.split(',').collect();
        if fields.len() != HEADER.len() {
            return Err(format!(
                "{source}:{line_number}: expected {} fields, got {}",
                HEADER.len(),
                fields.len()
            ));
        }

        let scenario_id = nonempty(fields[0], "scenario_id")?;
        let weight = parse_number(fields[1], "weight")?;
        if !(0.0..=1.0).contains(&weight) || weight == 0.0 {
            return Err(format!("{source}:{line_number}: weight must be in (0, 1]"));
        }
        let period_start_utc = nonempty(fields[2], "period_start_utc")?;
        let period_end_utc = nonempty(fields[3], "period_end_utc")?;
        let period_start = parse_utc_timestamp(&period_start_utc, "period_start_utc")?;
        let period_end = parse_utc_timestamp(&period_end_utc, "period_end_utc")?;
        validate_annual_period(period_start, period_end, &format!("{source}:{line_number}"))?;
        let source_timezone = nonempty(fields[4], "source_timezone")?;
        let interval_id = nonempty(fields[5], "interval_id")?;
        let start = parse_utc_timestamp(fields[6], "start_utc")?;
        let end = parse_utc_timestamp(fields[7], "end_utc")?;
        if start.epoch_seconds >= end.epoch_seconds {
            return Err(format!("{source}:{line_number}: interval must have positive duration"));
        }
        if start.epoch_seconds < period_start.epoch_seconds || end.epoch_seconds > period_end.epoch_seconds {
            return Err(format!("{source}:{line_number}: interval lies outside the declared annual period"));
        }

        let current_pu_mean = parse_number(fields[8], "current_pu_mean")?;
        let mean_square_current_pu2 = parse_number(fields[9], "mean_square_current_pu2")?;
        let price_per_kwh = parse_number(fields[10], "price_per_kwh")?;
        if current_pu_mean < 0.0 || mean_square_current_pu2 < 0.0 || price_per_kwh < 0.0 {
            return Err(format!("{source}:{line_number}: current moments and price must be non-negative"));
        }
        let squared_mean = current_pu_mean * current_pu_mean;
        if !squared_mean.is_finite() {
            return Err(format!("{source}:{line_number}: current mean cannot be squared safely"));
        }
        let moment_tolerance =
            MOMENT_REL_TOLERANCE * squared_mean.max(mean_square_current_pu2).max(1.0);
        if mean_square_current_pu2 + moment_tolerance < squared_mean {
            return Err(format!("{source}:{line_number}: mean-square current cannot be below squared mean current"));
        }

        let currency_code = nonempty(fields[11], "currency_code")?;
        if !valid_currency_syntax(&currency_code) {
            return Err(format!("{source}:{line_number}: currency_code must be three uppercase ASCII letters (syntax only; registry membership is not checked)"));
        }
        let price_year: i32 = fields[12]
            .trim()
            .parse()
            .map_err(|_| format!("{source}:{line_number}: price_year must be an integer"))?;
        if !(1900..=2200).contains(&price_year) {
            return Err(format!("{source}:{line_number}: price_year must be in 1900..=2200"));
        }
        let price_basis = nonempty(fields[13], "price_basis")?;
        if !matches!(price_basis.as_str(), "REAL" | "NOMINAL") {
            return Err(format!("{source}:{line_number}: price_basis must be REAL or NOMINAL"));
        }

        let new_interval = TraceInterval {
            interval_id: interval_id.clone(),
            start,
            end,
            current_pu_mean,
            mean_square_current_pu2,
            price_per_kwh,
        };
        if let Some(existing) = scenarios.get_mut(&scenario_id) {
            if (existing.weight - weight).abs() > WEIGHT_TOLERANCE
                || existing.period_start_utc != period_start_utc
                || existing.period_end_utc != period_end_utc
                || existing.source_timezone != source_timezone
                || existing.currency_code != currency_code
                || existing.price_year != price_year
                || existing.price_basis != price_basis
            {
                return Err(format!("{source}:{line_number}: scenario metadata (weight, period, timezone, currency or price basis/year) is inconsistent"));
            }
            if !existing.interval_ids.insert(interval_id.clone()) {
                return Err(format!("{source}:{line_number}: duplicate interval_id {interval_id:?} in scenario {scenario_id:?}"));
            }
            existing.intervals.push(new_interval);
        } else {
            let mut interval_ids = BTreeSet::new();
            interval_ids.insert(interval_id);
            scenarios.insert(
                scenario_id.clone(),
                TraceScenario {
                    scenario_id,
                    weight,
                    period_start_utc,
                    period_end_utc,
                    period_start,
                    period_end,
                    source_timezone,
                    currency_code,
                    price_year,
                    price_basis,
                    intervals: vec![new_interval],
                    interval_ids,
                },
            );
        }
    }

    if scenarios.is_empty() {
        return Err(format!("{source}: at least one interval is required"));
    }
    let total_weight: f64 = scenarios.values().map(|scenario| scenario.weight).sum();
    if (total_weight - 1.0).abs() > WEIGHT_TOLERANCE {
        return Err(format!("{source}: scenario weights must sum to 1.0; got {total_weight:.12}"));
    }

    for scenario in scenarios.values_mut() {
        scenario.intervals.sort_by(|left, right| {
            left.start
                .epoch_seconds
                .cmp(&right.start.epoch_seconds)
                .then_with(|| left.end.epoch_seconds.cmp(&right.end.epoch_seconds))
                .then_with(|| left.interval_id.cmp(&right.interval_id))
        });
        let first = scenario.intervals.first().ok_or_else(|| {
            format!("scenario {:?}: no intervals", scenario.scenario_id)
        })?;
        if first.start.epoch_seconds != scenario.period_start.epoch_seconds {
            return Err(format!("scenario {:?}: first interval does not begin at period_start_utc", scenario.scenario_id));
        }
        for pair in scenario.intervals.windows(2) {
            let previous = &pair[0];
            let next = &pair[1];
            if next.start.epoch_seconds < previous.end.epoch_seconds {
                return Err(format!(
                    "scenario {:?}: overlap between intervals {:?} and {:?}",
                    scenario.scenario_id, previous.interval_id, next.interval_id
                ));
            }
            if next.start.epoch_seconds > previous.end.epoch_seconds {
                return Err(format!(
                    "scenario {:?}: gap between intervals {:?} and {:?}",
                    scenario.scenario_id, previous.interval_id, next.interval_id
                ));
            }
        }
        let last = scenario.intervals.last().ok_or_else(|| {
            format!("scenario {:?}: no intervals", scenario.scenario_id)
        })?;
        if last.end.epoch_seconds != scenario.period_end.epoch_seconds {
            return Err(format!("scenario {:?}: final interval does not end at period_end_utc", scenario.scenario_id));
        }
        let duration_seconds: i64 = scenario
            .intervals
            .iter()
            .map(|interval| interval.end.epoch_seconds - interval.start.epoch_seconds)
            .sum();
        let expected_hours = validate_annual_period(
            scenario.period_start,
            scenario.period_end,
            &format!("scenario {:?}", scenario.scenario_id),
        )?;
        if duration_seconds != i64::from(expected_hours) * 3_600 {
            return Err(format!("scenario {:?}: interval durations do not match exact annual coverage", scenario.scenario_id));
        }
    }

    Ok(scenarios)
}

fn summarize(scenarios: &BTreeMap<String, TraceScenario>) -> Vec<ScenarioSummary> {
    scenarios
        .values()
        .map(|scenario| {
            let annual_hours = if is_leap_year(scenario.period_start.year) {
                8_784
            } else {
                8_760
            };
            ScenarioSummary {
                scenario_id: scenario.scenario_id.clone(),
                weight: scenario.weight,
                source_timezone: scenario.source_timezone.clone(),
                period_start_utc: scenario.period_start_utc.clone(),
                period_end_utc: scenario.period_end_utc.clone(),
                interval_count: scenario.intervals.len(),
                annual_hours,
                currency_code: scenario.currency_code.clone(),
                price_year: scenario.price_year,
                price_basis: scenario.price_basis.clone(),
            }
        })
        .collect()
}

fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn write_report(path: &str, summaries: &[ScenarioSummary]) -> Result<(), String> {
    let mut output = String::from(
        "scenario_id,weight,source_timezone,period_start_utc,period_end_utc,interval_count,annual_hours,currency_code,price_year,price_basis,utc_annual_coverage_validated\n",
    );
    for row in summaries {
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},true\n",
            csv_escape(&row.scenario_id),
            row.weight,
            csv_escape(&row.source_timezone),
            row.period_start_utc,
            row.period_end_utc,
            row.interval_count,
            row.annual_hours,
            row.currency_code,
            row.price_year,
            row.price_basis
        ));
    }
    fs::write(path, output).map_err(|error| format!("cannot write {path}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_parser_handles_leap_day_and_rejects_offsets_and_impossible_dates() {
        let leap = parse_utc_timestamp("2024-02-29T12:30:00Z", "test").unwrap();
        assert_eq!(leap.month, 2);
        assert_eq!(leap.day, 29);
        assert!(parse_utc_timestamp("2025-02-29T12:30:00Z", "test").is_err());
        assert!(parse_utc_timestamp("2025-01-01T00:00:00+00:00", "test").is_err());
        assert!(parse_utc_timestamp("2025-01-01T00:00Z", "test").is_err());
    }

    #[test]
    fn common_year_fixture_has_contiguous_8760_hour_coverage() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/global-traces-common-year-synthetic.csv");
        let content = fs::read_to_string(path).unwrap();
        let scenarios = parse_trace(&content, "fixture").unwrap();
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios["typical_demo"].intervals.len(), 2);
        assert_eq!(summarize(&scenarios)[0].annual_hours, 8_760);
    }

    #[test]
    fn leap_year_fixture_has_contiguous_8784_hour_coverage() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/global-traces-leap-year-synthetic.csv");
        let content = fs::read_to_string(path).unwrap();
        let scenarios = parse_trace(&content, "fixture").unwrap();
        assert_eq!(scenarios.len(), 1);
        assert_eq!(summarize(&scenarios)[0].annual_hours, 8_784);
    }

    #[test]
    fn compensating_gap_and_overlap_reject_even_when_duration_sum_is_one_year() {
        let header = HEADER.join(",");
        let text = format!(
            "{header}\n\
             broken,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,a,2025-01-01T00:00:00Z,2025-04-11T00:00:00Z,0.5,0.25,1.0,USD,2026,REAL\n\
             broken,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,b,2025-04-10T00:00:00Z,2025-07-20T00:00:00Z,0.5,0.25,1.0,USD,2026,REAL\n\
             broken,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,c,2025-07-21T00:00:00Z,2026-01-01T00:00:00Z,0.5,0.25,1.0,USD,2026,REAL\n"
        );
        let error = parse_trace(&text, "hostile").unwrap_err();
        assert!(error.contains("overlap") || error.contains("gap"));
    }

    #[test]
    fn rejects_currency_basis_and_moment_errors() {
        let header = HEADER.join(",");
        let bad_currency = format!(
            "{header}\nx,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,a,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,0.5,0.25,1.0,US$,2026,REAL\n"
        );
        assert!(parse_trace(&bad_currency, "bad_currency").is_err());
        let bad_basis = format!(
            "{header}\nx,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,a,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,0.5,0.25,1.0,USD,2026,INDEXED\n"
        );
        assert!(parse_trace(&bad_basis, "bad_basis").is_err());
        let bad_moment = format!(
            "{header}\nx,1.0,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,UTC,a,2025-01-01T00:00:00Z,2026-01-01T00:00:00Z,0.5,0.2,1.0,USD,2026,REAL\n"
        );
        assert!(parse_trace(&bad_moment, "bad_moment").is_err());
    }
}
