//! Provenance-preserving parser and profiler for the NIST/MTC cleaned
//! MTConnect operation-trace format (UTC timestamp | field | value).
//!
//! This crate describes events only. It never interprets an absent event as
//! idle, available, safe, productive, or unavailable equipment.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const NANOS_PER_SECOND: i128 = 1_000_000_000;
const SECONDS_PER_DAY: i128 = 86_400;

/// A parsed UTC event. Values and field names are preserved as strings because
/// source-specific semantics/units must be mapped explicitly by a later layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub timestamp_utc: String,
    pub timestamp_unix_ns: i128,
    pub field: String,
    pub value: String,
    pub line_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSummary {
    pub record_count: u64,
    pub distinct_field_count: usize,
    pub field_counts: BTreeMap<String, u64>,
    pub first_event_utc: String,
    pub last_event_utc: String,
    pub timestamp_inversions: u64,
    /// Adjacent events with the same timestamp. Such ties are valid for
    /// event-driven MTConnect records and are not automatically duplicates.
    pub adjacent_same_timestamp_pairs: u64,
    pub unique_timestamp_count: usize,
    pub maximum_gap_ns: i128,
    pub maximum_gap_start_utc_ns: i128,
    pub maximum_gap_end_utc_ns: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    EmptyTrace,
    InvalidRow { line_number: usize, reason: &'static str },
    InvalidTimestamp { line_number: usize, value: String, reason: &'static str },
    CountOverflow,
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trace has no event records"),
            Self::InvalidRow { line_number, reason } => {
                write!(f, "invalid row at line {line_number}: {reason}")
            }
            Self::InvalidTimestamp { line_number, value, reason } => {
                write!(f, "invalid UTC timestamp at line {line_number} ({value:?}): {reason}")
            }
            Self::CountOverflow => write!(f, "trace counter overflow"),
        }
    }
}

impl std::error::Error for TraceError {}

/// Parse a full trace. Blank lines are skipped; all nonblank rows must conform
/// to exactly three pipe-separated columns. A malformed row fails the entire
/// parse rather than being silently dropped.
pub fn parse_trace(input: &str) -> Result<Vec<TraceEvent>, TraceError> {
    let mut events = Vec::new();
    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            continue;
        }
        if line.matches('|').count() != 2 {
            return Err(TraceError::InvalidRow {
                line_number,
                reason: "expected exactly three pipe-separated columns",
            });
        }
        let mut columns = line.splitn(3, '|');
        let timestamp = columns.next().unwrap_or_default().trim();
        let field = columns.next().unwrap_or_default().trim();
        let value = columns.next().unwrap_or_default();
        if timestamp.is_empty() {
            return Err(TraceError::InvalidRow {
                line_number,
                reason: "timestamp column must not be empty",
            });
        }
        if field.is_empty() {
            return Err(TraceError::InvalidRow {
                line_number,
                reason: "field column must not be empty",
            });
        }
        let timestamp_unix_ns = parse_utc_timestamp_ns(timestamp).map_err(|reason| {
            TraceError::InvalidTimestamp {
                line_number,
                value: timestamp.to_owned(),
                reason,
            }
        })?;
        events.push(TraceEvent {
            timestamp_utc: timestamp.to_owned(),
            timestamp_unix_ns,
            field: field.to_owned(),
            value: value.to_owned(),
            line_number,
        });
    }
    if events.is_empty() {
        return Err(TraceError::EmptyTrace);
    }
    Ok(events)
}

/// Summarize event order and temporal coverage without assigning equipment
/// states to missing intervals. Maximum gap is between sorted unique event
/// timestamps; it is a sampling-coverage fact, never an availability claim.
pub fn summarize_events(events: &[TraceEvent]) -> Result<TraceSummary, TraceError> {
    let first = events.first().ok_or(TraceError::EmptyTrace)?;
    let last = events.last().ok_or(TraceError::EmptyTrace)?;
    let mut field_counts = BTreeMap::<String, u64>::new();
    let mut unique_times = BTreeSet::<i128>::new();
    let mut inversions = 0_u64;
    let mut same_timestamp_pairs = 0_u64;
    let mut previous: Option<i128> = None;

    for event in events {
        let count = field_counts.entry(event.field.clone()).or_insert(0);
        *count = count.checked_add(1).ok_or(TraceError::CountOverflow)?;
        unique_times.insert(event.timestamp_unix_ns);
        if let Some(previous_timestamp) = previous {
            if event.timestamp_unix_ns < previous_timestamp {
                inversions = inversions.checked_add(1).ok_or(TraceError::CountOverflow)?;
            } else if event.timestamp_unix_ns == previous_timestamp {
                same_timestamp_pairs = same_timestamp_pairs
                    .checked_add(1)
                    .ok_or(TraceError::CountOverflow)?;
            }
        }
        previous = Some(event.timestamp_unix_ns);
    }

    let mut maximum_gap_ns = 0_i128;
    let mut maximum_gap_start_utc_ns = 0_i128;
    let mut maximum_gap_end_utc_ns = 0_i128;
    let mut sorted = unique_times.iter();
    if let Some(mut prior) = sorted.next().copied() {
        for current in sorted {
            let gap = *current - prior;
            if gap > maximum_gap_ns {
                maximum_gap_ns = gap;
                maximum_gap_start_utc_ns = prior;
                maximum_gap_end_utc_ns = *current;
            }
            prior = *current;
        }
    }

    Ok(TraceSummary {
        record_count: u64::try_from(events.len()).map_err(|_| TraceError::CountOverflow)?,
        distinct_field_count: field_counts.len(),
        field_counts,
        first_event_utc: first.timestamp_utc.clone(),
        last_event_utc: last.timestamp_utc.clone(),
        timestamp_inversions: inversions,
        adjacent_same_timestamp_pairs: same_timestamp_pairs,
        unique_timestamp_count: unique_times.len(),
        maximum_gap_ns,
        maximum_gap_start_utc_ns,
        maximum_gap_end_utc_ns,
    })
}

pub fn parse_and_summarize(input: &str) -> Result<TraceSummary, TraceError> {
    let events = parse_trace(input)?;
    summarize_events(&events)
}

/// Parse an RFC 3339 subset in UTC only:
/// YYYY-MM-DDTHH:MM:SS[.fraction]Z, with 1..=9 fractional digits.
///
/// Returning integer nanoseconds avoids precision loss from floating-point
/// datetime conversions. Date and clock fields are fully range-checked.
pub fn parse_utc_timestamp_ns(value: &str) -> Result<i128, &'static str> {
    if !value.ends_with('Z') {
        return Err("timestamp must use an explicit UTC Z suffix");
    }
    let body = &value[..value.len() - 1];
    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (body, None),
    };
    if whole.len() != 19 || !whole.is_ascii() {
        return Err("expected YYYY-MM-DDTHH:MM:SS with optional fractional seconds");
    }
    let bytes = whole.as_bytes();
    if bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return Err("timestamp separators or T separator are invalid");
    }

    let year = parse_digits(&whole[0..4])?;
    let month = parse_digits(&whole[5..7])?;
    let day = parse_digits(&whole[8..10])?;
    let hour = parse_digits(&whole[11..13])?;
    let minute = parse_digits(&whole[14..16])?;
    let second = parse_digits(&whole[17..19])?;

    if !(1970..=9999).contains(&year) {
        return Err("year must be in 1970..=9999");
    }
    if !(1..=12).contains(&month) {
        return Err("month must be in 1..=12");
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return Err("invalid month"),
    };
    if !(1..=days_in_month).contains(&day) {
        return Err("day is invalid for the selected month");
    }
    if hour > 23 {
        return Err("hour must be in 0..=23");
    }
    if minute > 59 {
        return Err("minute must be in 0..=59");
    }
    if second > 59 {
        return Err("second must be in 0..=59; leap seconds are not supported");
    }

    let fractional_ns = match fraction {
        None => 0_i128,
        Some(digits) => {
            if digits.is_empty() || digits.len() > 9 || !digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err("fraction must contain 1..=9 decimal digits");
            }
            let raw = digits.parse::<u32>().map_err(|_| "fraction is invalid")?;
            (raw as i128) * 10_i128.pow((9 - digits.len()) as u32)
        }
    };

    let days_since_epoch = days_from_civil(year, month, day);
    let seconds_since_epoch = days_since_epoch
        .checked_mul(SECONDS_PER_DAY)
        .and_then(|days| days.checked_add((hour * 3_600 + minute * 60 + second) as i128))
        .ok_or("timestamp arithmetic overflow")?;
    seconds_since_epoch
        .checked_mul(NANOS_PER_SECOND)
        .and_then(|ns| ns.checked_add(fractional_ns))
        .ok_or("timestamp arithmetic overflow")
}

fn parse_digits(value: &str) -> Result<u32, &'static str> {
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err("numeric timestamp component is invalid");
    }
    value.parse::<u32>().map_err(|_| "numeric timestamp component is invalid")
}

fn is_leap_year(year: u32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Howard Hinnant's civil-date conversion, returning days from 1970-01-01.
/// Inputs are already constrained to years 1970..=9999.
fn days_from_civil(year: u32, month: u32, day: u32) -> i128 {
    let y = year as i128 - i128::from(month <= 2);
    let era = y / 400;
    let year_of_era = y - era * 400;
    let month_prime = month as i128 + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day as i128 - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXCERPT: &str = concat!(
        "2017-02-02T16:37:08.9062500Z|Block_Number|0\n",
        "2017-02-02T18:06:38.8437500Z|Block_Number|9\n",
        "2017-02-02T18:06:38.9531250Z|Program_Status|ACTIVE\n",
        "2017-02-02T18:06:38.9531250Z|Program_Runtime_Seconds|1500\n",
        "2017-02-02T18:06:38.9843750Z|Program_Runtime_Seconds|0\n",
        "2017-02-02T18:06:39.0000000Z|Spindle_Speed|248.880040\n",
    );

    #[test]
    fn parses_pinned_source_excerpt_without_losing_fractional_precision() {
        let events = parse_trace(EXCERPT).expect("source excerpt parses");
        assert_eq!(events.len(), 6);
        assert_eq!(events[0].timestamp_unix_ns % NANOS_PER_SECOND, 906_250_000);
        assert_eq!(events[1].timestamp_unix_ns % NANOS_PER_SECOND, 843_750_000);
        assert_eq!(events[2].field, "Program_Status");
        assert_eq!(events[2].value, "ACTIVE");
    }

    #[test]
    fn summary_preserves_event_ties_and_reports_long_unobserved_gap() {
        let summary = parse_and_summarize(EXCERPT).expect("excerpt summarizes");
        assert_eq!(summary.record_count, 6);
        assert_eq!(summary.distinct_field_count, 4);
        assert_eq!(summary.adjacent_same_timestamp_pairs, 1);
        assert_eq!(summary.unique_timestamp_count, 5);
        assert_eq!(summary.timestamp_inversions, 0);
        assert_eq!(summary.maximum_gap_ns, 5_369_937_500_000);
        assert_eq!(
            summary.maximum_gap_end_utc_ns - summary.maximum_gap_start_utc_ns,
            summary.maximum_gap_ns
        );
        assert_eq!(summary.field_counts.get("Block_Number"), Some(&2));
    }

    #[test]
    fn timestamps_are_monotonic_metric_not_an_availability_claim() {
        let out_of_order = concat!(
            "2017-02-02T03:00:00Z|Program_Status|ACTIVE\n",
            "2017-02-02T02:00:00Z|Program_Status|READY\n",
        );
        let summary = parse_and_summarize(out_of_order).expect("both rows are valid");
        assert_eq!(summary.timestamp_inversions, 1);
        assert_eq!(summary.maximum_gap_ns, 3_600 * NANOS_PER_SECOND);
    }

    #[test]
    fn skips_blank_lines_but_rejects_malformed_nonblank_rows() {
        let clean = "\n2017-02-02T03:00:00Z|Program_Status|ACTIVE\n\n";
        assert_eq!(parse_trace(clean).unwrap().len(), 1);
        assert!(matches!(
            parse_trace("2017-02-02T03:00:00Z|Program_Status|ACTIVE|extra"),
            Err(TraceError::InvalidRow { line_number: 1, .. })
        ));
        assert!(matches!(parse_trace("\n\n"), Err(TraceError::EmptyTrace)));
    }

    #[test]
    fn rejects_non_utc_malformed_dates_and_ambiguous_precision() {
        assert!(parse_utc_timestamp_ns("2017-02-02T03:00:00+00:00").is_err());
        assert!(parse_utc_timestamp_ns("2017-02-29T03:00:00Z").is_err());
        assert!(parse_utc_timestamp_ns("2017-02-02T03:00:60Z").is_err());
        assert!(parse_utc_timestamp_ns("2017-02-02T03:00:00.1234567890Z").is_err());
        assert!(parse_utc_timestamp_ns("2017-02-02T03:00:00.Z").is_err());
    }

    #[test]
    fn counts_fields_deterministically() {
        let summary = parse_and_summarize(EXCERPT).expect("excerpt summarizes");
        let names: Vec<&str> = summary.field_counts.keys().map(String::as_str).collect();
        assert_eq!(
            names,
            vec!["Block_Number", "Program_Runtime_Seconds", "Program_Status", "Spindle_Speed"]
        );
    }

    #[test]
    fn accepts_leap_day_only_in_leap_years() {
        assert!(parse_utc_timestamp_ns("2020-02-29T00:00:00Z").is_ok());
        assert!(parse_utc_timestamp_ns("2100-02-29T00:00:00Z").is_err());
        assert!(parse_utc_timestamp_ns("2000-02-29T00:00:00Z").is_ok());
    }

    #[test]
    fn preserves_blank_value_as_an_event_value() {
        let trace = "2017-02-02T03:00:00Z|Program_Name_Running| \n";
        let events = parse_trace(trace).expect("blank event value is allowed");
        assert_eq!(events[0].value, " ");
    }
}
