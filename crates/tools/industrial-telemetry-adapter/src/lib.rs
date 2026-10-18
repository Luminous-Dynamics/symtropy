// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

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
    pub maximum_gap_ns: Option<i128>,
    pub maximum_gap_start_timestamp_unix_ns: Option<i128>,
    pub maximum_gap_end_timestamp_unix_ns: Option<i128>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TraceError {
    EmptyTrace,
    InvalidRow {
        line_number: usize,
        reason: &'static str,
    },
    InvalidTimestamp {
        line_number: usize,
        value: String,
        reason: &'static str,
    },
    InvalidCoverageInterval,
    InvalidGapThreshold,
    CountOverflow,
}

impl fmt::Display for TraceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTrace => write!(f, "trace has no event records"),
            Self::InvalidRow {
                line_number,
                reason,
            } => {
                write!(f, "invalid row at line {line_number}: {reason}")
            }
            Self::InvalidTimestamp {
                line_number,
                value,
                reason,
            } => {
                write!(
                    f,
                    "invalid UTC timestamp at line {line_number} ({value:?}): {reason}"
                )
            }
            Self::InvalidCoverageInterval => {
                write!(f, "coverage interval must have start < end")
            }
            Self::InvalidGapThreshold => {
                write!(
                    f,
                    "maximum unobserved interval threshold must be non-negative"
                )
            }
            Self::CountOverflow => write!(f, "trace counter overflow"),
        }
    }
}

impl std::error::Error for TraceError {}

/// Classification of an unobserved time span. Edge gaps mean the requested
/// interval extends outside the first/last event in the supplied trace; they
/// do not assert anything about the machine before/after the trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageGapKind {
    NoEvents,
    BeforeFirstEvent,
    BetweenEvents,
    AfterLastEvent,
}

/// A threshold-exceeding interval with no event timestamp supporting it.
///
/// `overlap_start_ns..overlap_end_ns` is the portion intersecting the caller's
/// requested interval. `source_gap_duration_ns` is known for between-event
/// gaps and absent for open-ended/no-event regions. This type only describes
/// event chronology. It does not qualify equipment state or authorize work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    pub kind: CoverageGapKind,
    pub overlap_start_ns: i128,
    pub overlap_end_ns: i128,
    pub overlap_duration_ns: i128,
    pub source_gap_duration_ns: Option<i128>,
}

/// Find threshold-exceeding event-silence intervals intersecting a proposed
/// operation window `[interval_start_ns, interval_end_ns)`.
///
/// Input events may be out of order; the function uses sorted unique timestamp
/// values and never modifies the caller's evidence. A between-event gap is
/// reported if the original gap exceeds the threshold even when only a small
/// portion intersects the requested operation window. Before-first, after-last
/// and no-event regions are measured only over the requested window.
///
/// **No returned gaps is not an availability or safety approval.** It means
/// only that this detector found no gap exceeding the caller's threshold.
pub fn find_unobserved_intervals(
    events: &[TraceEvent],
    interval_start_ns: i128,
    interval_end_ns: i128,
    max_unobserved_ns: i128,
) -> Result<Vec<CoverageGap>, TraceError> {
    if interval_start_ns >= interval_end_ns {
        return Err(TraceError::InvalidCoverageInterval);
    }
    if max_unobserved_ns < 0 {
        return Err(TraceError::InvalidGapThreshold);
    }

    let query_duration = interval_end_ns - interval_start_ns;
    if events.is_empty() {
        return Ok(if query_duration > max_unobserved_ns {
            vec![CoverageGap {
                kind: CoverageGapKind::NoEvents,
                overlap_start_ns: interval_start_ns,
                overlap_end_ns: interval_end_ns,
                overlap_duration_ns: query_duration,
                source_gap_duration_ns: None,
            }]
        } else {
            Vec::new()
        });
    }

    let unique_times: BTreeSet<i128> = events.iter().map(|event| event.timestamp_unix_ns).collect();
    let times: Vec<i128> = unique_times.into_iter().collect();
    let mut gaps = Vec::new();

    let first = times[0];
    if interval_start_ns < first {
        let overlap_end_ns = interval_end_ns.min(first);
        if overlap_end_ns > interval_start_ns {
            let duration = overlap_end_ns - interval_start_ns;
            if duration > max_unobserved_ns {
                gaps.push(CoverageGap {
                    kind: CoverageGapKind::BeforeFirstEvent,
                    overlap_start_ns: interval_start_ns,
                    overlap_end_ns,
                    overlap_duration_ns: duration,
                    source_gap_duration_ns: None,
                });
            }
        }
    }

    for pair in times.windows(2) {
        let gap_start_ns = pair[0];
        let gap_end_ns = pair[1];
        let source_gap_duration_ns = gap_end_ns - gap_start_ns;
        if source_gap_duration_ns <= max_unobserved_ns {
            continue;
        }
        let overlap_start_ns = interval_start_ns.max(gap_start_ns);
        let overlap_end_ns = interval_end_ns.min(gap_end_ns);
        if overlap_start_ns < overlap_end_ns {
            gaps.push(CoverageGap {
                kind: CoverageGapKind::BetweenEvents,
                overlap_start_ns,
                overlap_end_ns,
                overlap_duration_ns: overlap_end_ns - overlap_start_ns,
                source_gap_duration_ns: Some(source_gap_duration_ns),
            });
        }
    }

    let last = *times.last().ok_or(TraceError::EmptyTrace)?;
    if interval_end_ns > last {
        let overlap_start_ns = interval_start_ns.max(last);
        if overlap_start_ns < interval_end_ns {
            let duration = interval_end_ns - overlap_start_ns;
            if duration > max_unobserved_ns {
                gaps.push(CoverageGap {
                    kind: CoverageGapKind::AfterLastEvent,
                    overlap_start_ns,
                    overlap_end_ns: interval_end_ns,
                    overlap_duration_ns: duration,
                    source_gap_duration_ns: None,
                });
            }
        }
    }

    Ok(gaps)
}

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
        let timestamp_unix_ns =
            parse_utc_timestamp_ns(timestamp).map_err(|reason| TraceError::InvalidTimestamp {
                line_number,
                value: timestamp.to_owned(),
                reason,
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

    let mut maximum_gap_ns: Option<i128> = None;
    let mut maximum_gap_start_timestamp_unix_ns: Option<i128> = None;
    let mut maximum_gap_end_timestamp_unix_ns: Option<i128> = None;
    let mut sorted = unique_times.iter();
    if let Some(mut prior) = sorted.next().copied() {
        for current in sorted {
            let gap = *current - prior;
            if maximum_gap_ns.is_none_or(|observed| gap > observed) {
                maximum_gap_ns = Some(gap);
                maximum_gap_start_timestamp_unix_ns = Some(prior);
                maximum_gap_end_timestamp_unix_ns = Some(*current);
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
        maximum_gap_start_timestamp_unix_ns,
        maximum_gap_end_timestamp_unix_ns,
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
            if digits.is_empty() || digits.len() > 9 || !digits.bytes().all(|b| b.is_ascii_digit())
            {
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
    value
        .parse::<u32>()
        .map_err(|_| "numeric timestamp component is invalid")
}

fn is_leap_year(year: u32) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

/// Howard Hinnant's civil-date conversion, returning days from 1970-01-01.
/// Inputs are already constrained to years 1970..=9999.
fn days_from_civil(year: u32, month: u32, day: u32) -> i128 {
    let y = year as i128 - if month <= 2 { 1 } else { 0 };
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

    const EXCERPT: &str = include_str!("../fixtures/nist_mtc_operation_excerpt.txt");

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
    fn utc_epoch_and_pinned_source_timestamps_use_correct_absolute_time() {
        assert_eq!(parse_utc_timestamp_ns("1970-01-01T00:00:00Z"), Ok(0));
        assert_eq!(
            parse_utc_timestamp_ns("2017-02-02T16:37:08.9062500Z"),
            Ok(1_486_053_428_906_250_000_i128)
        );
        assert_eq!(
            parse_utc_timestamp_ns("2017-02-02T18:06:38.8437500Z"),
            Ok(1_486_058_798_843_750_000_i128)
        );
    }

    #[test]
    fn summary_preserves_event_ties_and_reports_long_unobserved_gap() {
        let summary = parse_and_summarize(EXCERPT).expect("excerpt summarizes");
        assert_eq!(summary.record_count, 6);
        assert_eq!(summary.distinct_field_count, 4);
        assert_eq!(summary.adjacent_same_timestamp_pairs, 1);
        assert_eq!(summary.unique_timestamp_count, 5);
        assert_eq!(summary.timestamp_inversions, 0);
        assert_eq!(summary.maximum_gap_ns, Some(5_369_937_500_000));
        assert_eq!(
            summary.maximum_gap_end_timestamp_unix_ns.unwrap()
                - summary.maximum_gap_start_timestamp_unix_ns.unwrap(),
            summary.maximum_gap_ns.unwrap()
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
        assert_eq!(summary.maximum_gap_ns, Some(3_600 * NANOS_PER_SECOND));
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
            vec![
                "Block_Number",
                "Program_Runtime_Seconds",
                "Program_Status",
                "Spindle_Speed"
            ]
        );
    }

    #[test]
    fn accepts_leap_day_only_in_leap_years() {
        assert!(parse_utc_timestamp_ns("2020-02-29T00:00:00Z").is_ok());
        assert!(parse_utc_timestamp_ns("2100-02-29T00:00:00Z").is_err());
        assert!(parse_utc_timestamp_ns("2000-02-29T00:00:00Z").is_ok());
    }

    #[test]
    fn one_unique_timestamp_has_no_defined_inter_event_gap() {
        let trace = concat!(
            "2017-02-02T03:00:00Z|Program_Status|ACTIVE\n",
            "2017-02-02T03:00:00Z|Spindle_Speed|0\n",
        );
        let summary = parse_and_summarize(trace).expect("trace parses");
        assert_eq!(summary.unique_timestamp_count, 1);
        assert_eq!(summary.maximum_gap_ns, None);
        assert_eq!(summary.maximum_gap_start_timestamp_unix_ns, None);
        assert_eq!(summary.maximum_gap_end_timestamp_unix_ns, None);
    }

    fn timestamp(value: &str) -> i128 {
        parse_utc_timestamp_ns(value).expect("fixture timestamp parses")
    }

    #[test]
    fn flags_a_proposed_operation_that_crosses_a_long_event_gap() {
        let events = parse_trace(EXCERPT).expect("excerpt parses");
        let interval_start = timestamp("2017-02-02T17:00:00Z");
        let interval_end = timestamp("2017-02-02T17:15:00Z");
        let gaps =
            find_unobserved_intervals(&events, interval_start, interval_end, 60 * NANOS_PER_SECOND)
                .expect("interval evaluated");
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].kind, CoverageGapKind::BetweenEvents);
        assert_eq!(gaps[0].source_gap_duration_ns, Some(5_369_937_500_000_i128));
        assert_eq!(gaps[0].overlap_duration_ns, 15 * 60 * NANOS_PER_SECOND);
    }

    #[test]
    fn does_not_flag_short_local_event_spacing_but_makes_no_availability_claim() {
        let events = parse_trace(EXCERPT).expect("excerpt parses");
        let start = timestamp("2017-02-02T18:06:38.9531250Z");
        let end = timestamp("2017-02-02T18:06:39.0000000Z");
        let gaps = find_unobserved_intervals(&events, start, end, NANOS_PER_SECOND)
            .expect("interval evaluated");
        assert!(gaps.is_empty());
    }

    #[test]
    fn reports_intervals_before_after_and_without_any_events() {
        let events = parse_trace(EXCERPT).expect("excerpt parses");
        let before = find_unobserved_intervals(
            &events,
            timestamp("2017-02-02T16:00:00Z"),
            timestamp("2017-02-02T16:30:00Z"),
            60 * NANOS_PER_SECOND,
        )
        .expect("pre-trace interval evaluated");
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].kind, CoverageGapKind::BeforeFirstEvent);

        let after = find_unobserved_intervals(
            &events,
            timestamp("2017-02-02T19:00:00Z"),
            timestamp("2017-02-02T19:30:00Z"),
            60 * NANOS_PER_SECOND,
        )
        .expect("post-trace interval evaluated");
        assert_eq!(after.len(), 1);
        assert_eq!(after[0].kind, CoverageGapKind::AfterLastEvent);

        let no_events =
            find_unobserved_intervals(&[], 0, 120 * NANOS_PER_SECOND, 60 * NANOS_PER_SECOND)
                .expect("empty evidence interval assessed");
        assert_eq!(no_events.len(), 1);
        assert_eq!(no_events[0].kind, CoverageGapKind::NoEvents);
    }

    #[test]
    fn rejects_invalid_gap_query_bounds() {
        let events = parse_trace(EXCERPT).expect("excerpt parses");
        assert!(matches!(
            find_unobserved_intervals(&events, 10, 10, 0),
            Err(TraceError::InvalidCoverageInterval)
        ));
        assert!(matches!(
            find_unobserved_intervals(&events, 0, 10, -1),
            Err(TraceError::InvalidGapThreshold)
        ));
    }

    #[test]
    fn preserves_blank_value_as_an_event_value() {
        let trace = "2017-02-02T03:00:00Z|Program_Name_Running| \n";
        let events = parse_trace(trace).expect("blank event value is allowed");
        assert_eq!(events[0].value, " ");
    }
}
