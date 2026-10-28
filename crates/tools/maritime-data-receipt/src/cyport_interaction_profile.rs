// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

//! Strict local profiler for the pinned CyPort port-cyclone interaction CSV.
//!
//! This utility inventories and validates source structure only. It does not
//! fit a model, infer causality, validate recovery predictions, or authorize
//! physical/operational decisions. The source dataset is deliberately not
//! copied into this repository; callers provide a local file.

use super::JsonValue;
use serde::Serialize;
use sha1::{Digest as Sha1Digest, Sha1};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;

const MAX_CSV_BYTES: usize = 5 * 1024 * 1024;
const PINNED_BYTES: usize = 643_047;
const PINNED_GIT_BLOB_SHA1: &str = "21beb5b7673918143a0ab108b03acc3cccaeb330";
const PINNED_REVISION: &str = "5cb1d8778d3968c5e50d0b8cb8213f2414d692d3";

const EXPECTED_HEADER: &[&str] = &[
    "ID",
    "SID",
    "PID",
    "port",
    "PORT_LON",
    "PORT_LAT",
    "start_date",
    "end_date",
    "start_recovery_date",
    "end_recovery_date",
    "total_impact",
    "total_impact_value",
    "max_impact",
    "day_of_recover",
    "DD",
    "CCD",
    "BCD",
    "SSHS",
    "SSHS_1",
    "SSHS_2",
    "SSHS_3",
    "SSHS_4",
    "SSHS_5",
    "IF_LANDFALL_CLOSE2PORT",
    "IF_LANDFALL",
    "WIND",
    "PRESSURE",
    "DISTANCE",
    "Wind_speed",
    "Surge_height",
    "Rainfall",
    "Port_ranking",
    "D_normal",
    "D_cyclone",
    "C_normal",
    "C_cyclone",
    "B_normal",
    "B_cyclone",
    "E_normal",
    "E_cyclone",
    "Seaport",
    "Pop_C",
    "Pop_Tract",
    "PCT_Pov",
    "WF",
    "PCT_TA",
    "PCT_ACEW",
    "PCT_TI",
    "PCT_SV",
    "SL",
    "Railway_Length",
    "Highway_Length",
    "Dock_Count",
    "Coast_Gulf_of_Mexico",
    "Coast_Atlantic",
    "Coast_Pacific",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct CyPortInteractionReport {
    status: &'static str,
    source_id: &'static str,
    source_revision_expected: &'static str,
    source_path_expected: &'static str,
    source_git_blob_sha1_expected: &'static str,
    source_git_blob_sha1_observed: String,
    source_sha256_observed: String,
    source_byte_length: usize,
    pinned_source_identity_verified: bool,
    column_count: usize,
    data_row_count: usize,
    unique_interaction_ids: usize,
    duplicate_interaction_id_rows: usize,
    unique_storm_ids: usize,
    unique_port_ids: usize,
    unique_storm_port_pairs: usize,
    duplicate_storm_port_pair_rows: usize,
    zero_total_impact_rows: usize,
    nonzero_total_impact_rows: usize,
    zero_impact_all_recovery_sentinels_rows: usize,
    zero_impact_all_recovery_fields_present_rows: usize,
    nonzero_impact_all_recovery_fields_present_rows: usize,
    no_impact_window_recorded_rows: usize,
    impact_window_recovery_markers_present_rows: usize,
    mixed_recovery_field_state_rows: usize,
    nonzero_impact_without_recovery_markers_rows: usize,
    start_recovery_date_sentinel_rows: usize,
    end_recovery_date_sentinel_rows: usize,
    day_of_recover_sentinel_rows: usize,
    earliest_event_start: String,
    latest_event_start: String,
    earliest_event_end: String,
    latest_event_end: String,
    episode_split_policy_id: &'static str,
    split_summaries: Vec<SplitSummary>,
    split_outcome_summaries: Vec<SplitOutcomeSummary>,
    episode_groups: Vec<StormEpisodeSplit>,
    data_capture_persisted: bool,
    model_validation_performed: bool,
    physical_simulation_performed: bool,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct StormEpisodeSplit {
    storm_id: String,
    first_event_start: String,
    last_event_start: String,
    split: &'static str,
    interaction_rows: usize,
    unique_port_ids: usize,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SplitSummary {
    split: &'static str,
    storm_count: usize,
    interaction_rows: usize,
    unique_port_ids: Option<usize>,
    first_event_start: Option<String>,
    last_event_start: Option<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SplitOutcomeSummary {
    split: &'static str,
    storm_count: usize,
    interaction_rows: usize,
    zero_total_impact_rows: usize,
    nonzero_total_impact_rows: usize,
    no_impact_window_recorded_rows: usize,
    impact_window_recovery_markers_present_rows: usize,
    mixed_recovery_field_state_rows: usize,
    nonzero_impact_without_recovery_markers_rows: usize,
}

#[derive(Default)]
struct StormAccumulator {
    row_count: usize,
    port_ids: BTreeSet<String>,
    first_event_start: Option<String>,
    last_event_start: Option<String>,
    zero_total_impact_rows: usize,
    nonzero_total_impact_rows: usize,
    no_impact_window_recorded_rows: usize,
    impact_window_recovery_markers_present_rows: usize,
    mixed_recovery_field_state_rows: usize,
    nonzero_impact_without_recovery_markers_rows: usize,
}

#[derive(Default)]
struct SplitAccumulator {
    storm_count: usize,
    interaction_rows: usize,
    port_ids: BTreeSet<String>,
    first_event_start: Option<String>,
    last_event_start: Option<String>,
    zero_total_impact_rows: usize,
    nonzero_total_impact_rows: usize,
    no_impact_window_recorded_rows: usize,
    impact_window_recovery_markers_present_rows: usize,
    mixed_recovery_field_state_rows: usize,
    nonzero_impact_without_recovery_markers_rows: usize,
}

#[derive(Default)]
struct ProfileCounts {
    ids: BTreeSet<String>,
    storm_ids: BTreeSet<String>,
    port_ids: BTreeSet<String>,
    storm_port_pairs: BTreeSet<(String, String)>,
    storm_groups: BTreeMap<String, StormAccumulator>,
    duplicate_id_rows: usize,
    duplicate_pair_rows: usize,
    zero_total_impact_rows: usize,
    nonzero_total_impact_rows: usize,
    zero_impact_all_recovery_sentinels: usize,
    zero_impact_all_recovery_fields_present: usize,
    nonzero_impact_all_recovery_fields_present: usize,
    mixed_recovery_field_states: usize,
    nonzero_impact_without_recovery_markers: usize,
    start_recovery_sentinels: usize,
    end_recovery_sentinels: usize,
    day_of_recover_sentinels: usize,
    event_starts: Vec<String>,
    event_ends: Vec<String>,
}

const EPISODE_SPLIT_POLICY_ID: &str =
    "cyport-sid-temporal-v0.1:train<=2020;calibration=2021;test>=2022";

fn split_for_date(date: &str) -> Result<&'static str, String> {
    let year = date
        .get(0..4)
        .ok_or_else(|| format!("date lacks a four-digit year: {date:?}"))?
        .parse::<u32>()
        .map_err(|_| format!("date has an invalid year: {date:?}"))?;
    Ok(if year <= 2020 {
        "train"
    } else if year == 2021 {
        "calibration"
    } else {
        "test"
    })
}

fn parse_csv(bytes: &[u8]) -> Result<Vec<Vec<String>>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = Vec::<u8>::new();
    let mut in_quotes = false;
    let mut after_quote = false;
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if in_quotes {
            if byte == b'"' {
                if bytes.get(index + 1) == Some(&b'"') {
                    field.push(b'"');
                    index += 1;
                } else {
                    in_quotes = false;
                    after_quote = true;
                }
            } else {
                field.push(byte);
            }
        } else if after_quote {
            match byte {
                b',' => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                    after_quote = false;
                }
                b'\n' => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                    push_nonblank_row(&mut rows, std::mem::take(&mut row));
                    after_quote = false;
                }
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                    push_nonblank_row(&mut rows, std::mem::take(&mut row));
                    index += 1;
                    after_quote = false;
                }
                _ => {
                    return Err(format!(
                        "CSV row {}, column {}: unexpected byte after closing quote",
                        rows.len() + 1,
                        row.len() + 1
                    ));
                }
            }
        } else {
            match byte {
                b'"' if field.is_empty() => in_quotes = true,
                b'"' => {
                    return Err(format!(
                        "CSV row {}, column {}: quote inside unquoted field",
                        rows.len() + 1,
                        row.len() + 1
                    ));
                }
                b',' => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                }
                b'\n' => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                    push_nonblank_row(&mut rows, std::mem::take(&mut row));
                }
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    row.push(decode_field(
                        std::mem::take(&mut field),
                        rows.len(),
                        row.len(),
                    )?);
                    push_nonblank_row(&mut rows, std::mem::take(&mut row));
                    index += 1;
                }
                b'\r' => {
                    return Err(format!(
                        "CSV row {}, column {}: standalone carriage return is unsupported",
                        rows.len() + 1,
                        row.len() + 1
                    ));
                }
                _ => field.push(byte),
            }
        }
        index += 1;
    }

    if in_quotes {
        return Err("CSV ends inside a quoted field".to_owned());
    }
    if after_quote || !field.is_empty() || !row.is_empty() {
        row.push(decode_field(field, rows.len(), row.len())?);
        push_nonblank_row(&mut rows, row);
    }
    if rows.is_empty() {
        return Err("CSV is empty".to_owned());
    }
    Ok(rows)
}

fn decode_field(bytes: Vec<u8>, row: usize, column: usize) -> Result<String, String> {
    String::from_utf8(bytes).map_err(|error| {
        format!(
            "CSV row {}, column {} is not valid UTF-8: {error}",
            row + 1,
            column + 1
        )
    })
}

fn push_nonblank_row(rows: &mut Vec<Vec<String>>, row: Vec<String>) {
    if !(row.len() == 1 && row[0].is_empty()) {
        rows.push(row);
    }
}

fn git_blob_sha1(bytes: &[u8]) -> String {
    let header = format!("blob {}\0", bytes.len());
    let mut hasher = Sha1::new();
    hasher.update(header.as_bytes());
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn is_sentinel(value: &str) -> bool {
    let value = value.trim();
    value.is_empty()
        || value == "0"
        || value.eq_ignore_ascii_case("NA")
        || value.eq_ignore_ascii_case("N/A")
}

fn column_index(header: &[String], name: &str) -> Result<usize, String> {
    header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("required CyPort column {name:?} is missing"))
}

fn record_field<'a>(
    record: &'a [String],
    indexes: &BTreeMap<&str, usize>,
    name: &str,
    row_number: usize,
) -> Result<&'a str, String> {
    let index = indexes
        .get(name)
        .copied()
        .ok_or_else(|| format!("required CyPort column {name:?} is missing"))?;
    record
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("CSV data row {row_number} has no field {name:?}"))
}

fn parse_mdy_date(value: &str, path: &str) -> Result<String, String> {
    let mut tokens = value.split_ascii_whitespace();
    let date_part = tokens.next().unwrap_or_default();
    if let Some(time_part) = tokens.next() {
        let mut time_components = time_part.split(':');
        let hour = time_components
            .next()
            .ok_or_else(|| format!("{path}: invalid hour in time {time_part:?}"))?
            .parse::<u32>()
            .map_err(|_| format!("{path}: invalid hour in time {time_part:?}"))?;
        let minute = time_components
            .next()
            .ok_or_else(|| format!("{path}: invalid minute in time {time_part:?}"))?
            .parse::<u32>()
            .map_err(|_| format!("{path}: invalid minute in time {time_part:?}"))?;
        if time_components.next().is_some() || hour > 23 || minute > 59 {
            return Err(format!("{path}: invalid HH:MM time {time_part:?}"));
        }
        if tokens.next().is_some() {
            return Err(format!(
                "{path}: unexpected trailing date/time components in {value:?}"
            ));
        }
    }
    let mut pieces = date_part.split('/');
    let month = pieces
        .next()
        .ok_or_else(|| format!("{path}: invalid month/day/year date"))?
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid month"))?;
    let day = pieces
        .next()
        .ok_or_else(|| format!("{path}: invalid month/day/year date"))?
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid day"))?;
    let year = pieces
        .next()
        .ok_or_else(|| format!("{path}: invalid month/day/year date"))?
        .parse::<u32>()
        .map_err(|_| format!("{path}: invalid year"))?;
    if pieces.next().is_some() {
        return Err(format!("{path}: unexpected date components"));
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    if year == 0 || day == 0 || day > max_day {
        return Err(format!("{path}: impossible calendar date {value:?}"));
    }
    Ok(format!("{year:04}-{month:02}-{day:02}"))
}

fn profile_bytes(bytes: &[u8]) -> Result<CyPortInteractionReport, String> {
    if bytes.len() > MAX_CSV_BYTES {
        return Err(format!(
            "CyPort interaction CSV exceeds the {}-byte limit",
            MAX_CSV_BYTES
        ));
    }
    if bytes.len() == PINNED_BYTES {
        let observed_blob = git_blob_sha1(bytes);
        if observed_blob != PINNED_GIT_BLOB_SHA1 {
            return Err(format!(
                "CSV has the pinned resource byte length but Git blob SHA-1 mismatch: expected {PINNED_GIT_BLOB_SHA1}, observed {observed_blob}"
            ));
        }
    }

    let rows = parse_csv(bytes)?;
    let header = &rows[0];
    if header.len() != EXPECTED_HEADER.len() {
        return Err(format!(
            "CyPort header has {} columns; expected {}",
            header.len(),
            EXPECTED_HEADER.len()
        ));
    }
    let mut seen_headers = BTreeSet::new();
    for (index, expected) in EXPECTED_HEADER.iter().enumerate() {
        if header[index] != *expected {
            return Err(format!(
                "CyPort header column {} mismatch: expected {:?}, got {:?}",
                index + 1,
                expected,
                header[index]
            ));
        }
        if !seen_headers.insert(header[index].clone()) {
            return Err(format!("duplicate header name {:?}", header[index]));
        }
    }

    let indexes = BTreeMap::from([
        ("ID", column_index(header, "ID")?),
        ("SID", column_index(header, "SID")?),
        ("PID", column_index(header, "PID")?),
        ("port", column_index(header, "port")?),
        ("start_date", column_index(header, "start_date")?),
        ("end_date", column_index(header, "end_date")?),
        (
            "start_recovery_date",
            column_index(header, "start_recovery_date")?,
        ),
        (
            "end_recovery_date",
            column_index(header, "end_recovery_date")?,
        ),
        ("total_impact", column_index(header, "total_impact")?),
        ("day_of_recover", column_index(header, "day_of_recover")?),
    ]);
    let records = &rows[1..];
    if records.is_empty() {
        return Err("CyPort interaction CSV has a header but no data rows".to_owned());
    }
    let mut counts = ProfileCounts::default();

    for (row_index, record) in records.iter().enumerate() {
        if record.len() != header.len() {
            return Err(format!(
                "CSV data row {} has {} fields; expected {}",
                row_index + 2,
                record.len(),
                header.len()
            ));
        }
        for name in ["ID", "SID", "PID", "port"] {
            if is_sentinel(record_field(record, &indexes, name, row_index + 2)?) {
                return Err(format!(
                    "CSV row {} has missing required identity field {name}",
                    row_index + 2
                ));
            }
        }

        let id = record_field(record, &indexes, "ID", row_index + 2)?.to_owned();
        if !counts.ids.insert(id) {
            counts.duplicate_id_rows += 1;
        }
        let storm = record_field(record, &indexes, "SID", row_index + 2)?.to_owned();
        let port_id = record_field(record, &indexes, "PID", row_index + 2)?.to_owned();
        counts.storm_ids.insert(storm.clone());
        counts.port_ids.insert(port_id.clone());
        if !counts
            .storm_port_pairs
            .insert((storm.clone(), port_id.clone()))
        {
            counts.duplicate_pair_rows += 1;
        }

        let impact_raw = record_field(record, &indexes, "total_impact", row_index + 2)?;
        let impact = impact_raw.parse::<u64>().map_err(|_| {
            format!(
                "CSV row {} has invalid non-negative integer total_impact {impact_raw:?}",
                row_index + 2
            )
        })?;
        if impact == 0 {
            counts.zero_total_impact_rows += 1;
        } else {
            counts.nonzero_total_impact_rows += 1;
        }

        let start = parse_mdy_date(
            record_field(record, &indexes, "start_date", row_index + 2)?,
            &format!("row {} start_date", row_index + 2),
        )?;
        let end = parse_mdy_date(
            record_field(record, &indexes, "end_date", row_index + 2)?,
            &format!("row {} end_date", row_index + 2),
        )?;
        if end < start {
            return Err(format!(
                "CSV row {} ends before its start date ({start} > {end})",
                row_index + 2
            ));
        }
        counts.event_starts.push(start.clone());
        counts.event_ends.push(end);

        let group = counts.storm_groups.entry(storm.clone()).or_default();
        group.row_count += 1;
        group.port_ids.insert(port_id);
        match group.first_event_start.as_ref() {
            Some(current) if current <= &start => {}
            _ => group.first_event_start = Some(start.clone()),
        }
        match group.last_event_start.as_ref() {
            Some(current) if current >= &start => {}
            _ => group.last_event_start = Some(start),
        }

        let start_recovery = record_field(record, &indexes, "start_recovery_date", row_index + 2)?;
        let end_recovery = record_field(record, &indexes, "end_recovery_date", row_index + 2)?;
        let recovery_duration = record_field(record, &indexes, "day_of_recover", row_index + 2)?;
        let start_recovery_sentinel = is_sentinel(start_recovery);
        let end_recovery_sentinel = is_sentinel(end_recovery);
        let duration_sentinel = is_sentinel(recovery_duration);
        if start_recovery_sentinel {
            counts.start_recovery_sentinels += 1;
        } else {
            let _ = parse_mdy_date(
                start_recovery,
                &format!("row {} start_recovery_date", row_index + 2),
            )?;
        }
        if end_recovery_sentinel {
            counts.end_recovery_sentinels += 1;
        } else {
            let _ = parse_mdy_date(
                end_recovery,
                &format!("row {} end_recovery_date", row_index + 2),
            )?;
        }
        if duration_sentinel {
            counts.day_of_recover_sentinels += 1;
        } else {
            let duration = recovery_duration.parse::<u64>().map_err(|_| {
                format!(
                    "CSV row {} has invalid non-negative integer day_of_recover {recovery_duration:?}",
                    row_index + 2
                )
            })?;
            if duration == 0 {
                return Err(format!(
                    "CSV row {} has a zero recovery duration that was not recognized as a sentinel",
                    row_index + 2
                ));
            }
        }

        let (
            no_impact_window_recorded,
            impact_window_markers_present,
            mixed_marker_state,
            nonzero_impact_without_markers,
        ) = match (
            start_recovery_sentinel,
            end_recovery_sentinel,
            duration_sentinel,
        ) {
            (true, true, true) if impact == 0 => {
                counts.zero_impact_all_recovery_sentinels += 1;
                (true, false, false, false)
            }
            (true, true, true) => {
                counts.nonzero_impact_without_recovery_markers += 1;
                (false, false, false, true)
            }
            (false, false, false) if impact == 0 => {
                counts.zero_impact_all_recovery_fields_present += 1;
                (false, true, false, false)
            }
            (false, false, false) => {
                counts.nonzero_impact_all_recovery_fields_present += 1;
                (false, true, false, false)
            }
            _ => {
                counts.mixed_recovery_field_states += 1;
                (false, false, true, false)
            }
        };
        let group = counts
            .storm_groups
            .get_mut(&storm)
            .ok_or_else(|| format!("missing cyclone group after row {}", row_index + 2))?;
        if impact == 0 {
            group.zero_total_impact_rows += 1;
        } else {
            group.nonzero_total_impact_rows += 1;
        }
        if no_impact_window_recorded {
            group.no_impact_window_recorded_rows += 1;
        } else if impact_window_markers_present {
            group.impact_window_recovery_markers_present_rows += 1;
        }
        if mixed_marker_state {
            group.mixed_recovery_field_state_rows += 1;
        }
        if nonzero_impact_without_markers {
            group.nonzero_impact_without_recovery_markers_rows += 1;
        }
    }

    counts.event_starts.sort();
    counts.event_ends.sort();

    let mut episode_groups: Vec<StormEpisodeSplit> = counts
        .storm_groups
        .iter()
        .map(|(storm_id, group)| {
            let first_event_start = group
                .first_event_start
                .clone()
                .expect("storm group is created only after a valid event start");
            let last_event_start = group
                .last_event_start
                .clone()
                .expect("storm group is created only after a valid event start");
            Ok(StormEpisodeSplit {
                storm_id: storm_id.clone(),
                split: split_for_date(&first_event_start)?,
                first_event_start,
                last_event_start,
                interaction_rows: group.row_count,
                unique_port_ids: group.port_ids.len(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    episode_groups.sort_by(|left, right| {
        left.first_event_start
            .cmp(&right.first_event_start)
            .then_with(|| left.storm_id.cmp(&right.storm_id))
    });

    let split_names: &[&str] = &["train", "calibration", "test"];
    let mut split_accumulators: BTreeMap<&str, SplitAccumulator> = split_names
        .iter()
        .map(|name| (*name, SplitAccumulator::default()))
        .collect();
    for episode in &episode_groups {
        let summary = split_accumulators
            .get_mut(episode.split)
            .ok_or_else(|| format!("unknown split assignment {:?}", episode.split))?;
        summary.storm_count += 1;
        summary.interaction_rows += episode.interaction_rows;
        match summary.first_event_start.as_ref() {
            Some(current) if current <= &episode.first_event_start => {}
            _ => summary.first_event_start = Some(episode.first_event_start.clone()),
        }
        match summary.last_event_start.as_ref() {
            Some(current) if current >= &episode.last_event_start => {}
            _ => summary.last_event_start = Some(episode.last_event_start.clone()),
        }
        let group = counts
            .storm_groups
            .get(&episode.storm_id)
            .ok_or_else(|| format!("missing storm group {:?}", episode.storm_id))?;
        summary.port_ids.extend(group.port_ids.iter().cloned());
        summary.zero_total_impact_rows += group.zero_total_impact_rows;
        summary.nonzero_total_impact_rows += group.nonzero_total_impact_rows;
        summary.no_impact_window_recorded_rows += group.no_impact_window_recorded_rows;
        summary.impact_window_recovery_markers_present_rows +=
            group.impact_window_recovery_markers_present_rows;
        summary.mixed_recovery_field_state_rows += group.mixed_recovery_field_state_rows;
        summary.nonzero_impact_without_recovery_markers_rows +=
            group.nonzero_impact_without_recovery_markers_rows;
    }
    let split_summaries: Vec<SplitSummary> = split_names
        .iter()
        .map(|name| {
            let summary = split_accumulators
                .get(name)
                .expect("all declared split accumulators were inserted");
            SplitSummary {
                split: if *name == "train" {
                    "train"
                } else if *name == "calibration" {
                    "calibration"
                } else {
                    "test"
                },
                storm_count: summary.storm_count,
                interaction_rows: summary.interaction_rows,
                unique_port_ids: Some(summary.port_ids.len()),
                first_event_start: summary.first_event_start.clone(),
                last_event_start: summary.last_event_start.clone(),
            }
        })
        .collect();
    let split_outcome_summaries: Vec<SplitOutcomeSummary> = split_names
        .iter()
        .map(|name| {
            let summary = split_accumulators
                .get(name)
                .expect("all declared outcome accumulators were inserted");
            SplitOutcomeSummary {
                split: if *name == "train" {
                    "train"
                } else if *name == "calibration" {
                    "calibration"
                } else {
                    "test"
                },
                storm_count: summary.storm_count,
                interaction_rows: summary.interaction_rows,
                zero_total_impact_rows: summary.zero_total_impact_rows,
                nonzero_total_impact_rows: summary.nonzero_total_impact_rows,
                no_impact_window_recorded_rows: summary.no_impact_window_recorded_rows,
                impact_window_recovery_markers_present_rows: summary
                    .impact_window_recovery_markers_present_rows,
                mixed_recovery_field_state_rows: summary.mixed_recovery_field_state_rows,
                nonzero_impact_without_recovery_markers_rows: summary
                    .nonzero_impact_without_recovery_markers_rows,
            }
        })
        .collect();

    let observed_blob_sha1 = git_blob_sha1(bytes);
    let pinned_source_identity_verified =
        bytes.len() == PINNED_BYTES && observed_blob_sha1 == PINNED_GIT_BLOB_SHA1;

    if pinned_source_identity_verified {
        let expected = [
            ("data rows", records.len(), 1_927usize),
            ("unique storms", counts.storm_ids.len(), 90usize),
            ("unique ports", counts.port_ids.len(), 122usize),
            ("unique interaction IDs", counts.ids.len(), 1_927usize),
            (
                "unique storm-port pairs",
                counts.storm_port_pairs.len(),
                1_927usize,
            ),
            (
                "zero-impact rows",
                counts.zero_total_impact_rows,
                1_654usize,
            ),
            (
                "nonzero-impact rows",
                counts.nonzero_total_impact_rows,
                273usize,
            ),
            (
                "zero-impact rows with all recovery sentinels",
                counts.zero_impact_all_recovery_sentinels,
                1_650usize,
            ),
            (
                "zero-impact rows with all recovery fields present",
                counts.zero_impact_all_recovery_fields_present,
                4usize,
            ),
            (
                "nonzero-impact rows with all recovery fields present",
                counts.nonzero_impact_all_recovery_fields_present,
                273usize,
            ),
            (
                "no-impact-window marker triplets",
                counts.zero_impact_all_recovery_sentinels,
                1_650usize,
            ),
            (
                "recorded impact-window marker triplets",
                counts.zero_impact_all_recovery_fields_present
                    + counts.nonzero_impact_all_recovery_fields_present,
                277usize,
            ),
            (
                "mixed recovery-field states",
                counts.mixed_recovery_field_states,
                0usize,
            ),
            (
                "nonzero impact without recovery markers",
                counts.nonzero_impact_without_recovery_markers,
                0usize,
            ),
            (
                "start-recovery sentinels",
                counts.start_recovery_sentinels,
                1_650usize,
            ),
            (
                "end-recovery sentinels",
                counts.end_recovery_sentinels,
                1_650usize,
            ),
            (
                "recovery-day sentinels",
                counts.day_of_recover_sentinels,
                1_650usize,
            ),
        ];
        for (name, actual, target) in expected {
            if actual != target {
                return Err(format!(
                    "pinned CyPort profile drift for {name}: expected {target}, observed {actual}"
                ));
            }
        }
        if counts.duplicate_id_rows != 0 || counts.duplicate_pair_rows != 0 {
            return Err(
                "pinned CyPort profile contains duplicate interaction IDs or storm-port pairs"
                    .to_owned(),
            );
        }
        let expected_splits = [
            ("train", 59usize, 1_471usize, 122usize),
            ("calibration", 13usize, 308usize, 92usize),
            ("test", 18usize, 148usize, 61usize),
        ];
        for (name, storms, interaction_rows, ports) in expected_splits {
            let actual = split_summaries
                .iter()
                .find(|summary| summary.split == name)
                .ok_or_else(|| format!("missing expected split {name}"))?;
            if (
                actual.storm_count,
                actual.interaction_rows,
                actual.unique_port_ids,
            ) != (storms, interaction_rows, Some(ports))
            {
                return Err(format!(
                    "pinned CyPort {name} split drift: expected ({storms} storms, {interaction_rows} rows, {ports} ports), observed ({} storms, {} rows, {} ports)",
                    actual.storm_count,
                    actual.interaction_rows,
                    actual.unique_port_ids.unwrap_or_default()
                ));
            }
        }
        let expected_split_outcomes = [
            (
                "train", 59usize, 1_471usize, 1_266usize, 205usize, 1_263usize, 208usize, 0usize,
                0usize,
            ),
            (
                "calibration",
                13usize,
                308usize,
                265usize,
                43usize,
                265usize,
                43usize,
                0usize,
                0usize,
            ),
            (
                "test", 18usize, 148usize, 123usize, 25usize, 122usize, 26usize, 0usize, 0usize,
            ),
        ];
        for (
            name,
            storms,
            rows,
            zero_impact,
            nonzero_impact,
            no_window,
            markers_present,
            mixed,
            nonzero_without_markers,
        ) in expected_split_outcomes
        {
            let actual = split_outcome_summaries
                .iter()
                .find(|summary| summary.split == name)
                .ok_or_else(|| format!("missing expected impact-state split {name}"))?;
            let observed = (
                actual.storm_count,
                actual.interaction_rows,
                actual.zero_total_impact_rows,
                actual.nonzero_total_impact_rows,
                actual.no_impact_window_recorded_rows,
                actual.impact_window_recovery_markers_present_rows,
                actual.mixed_recovery_field_state_rows,
                actual.nonzero_impact_without_recovery_markers_rows,
            );
            if observed
                != (
                    storms,
                    rows,
                    zero_impact,
                    nonzero_impact,
                    no_window,
                    markers_present,
                    mixed,
                    nonzero_without_markers,
                )
            {
                return Err(format!(
                    "pinned CyPort {name} impact-state split drift: observed {observed:?}"
                ));
            }
        }
        if counts.event_starts.first().map(String::as_str) != Some("2015-05-06")
            || counts.event_starts.last().map(String::as_str) != Some("2023-10-22")
            || counts.event_ends.first().map(String::as_str) != Some("2015-05-06")
            || counts.event_ends.last().map(String::as_str) != Some("2023-10-23")
        {
            return Err(
                "pinned CyPort event date range differs from the inspected source profile"
                    .to_owned(),
            );
        }
    }

    let status = if pinned_source_identity_verified {
        "PINNED_SOURCE_PROFILED_NOT_MODEL_VALIDATED"
    } else {
        "SCHEMA_VALID_PROFILE_NOT_PINNED"
    };
    Ok(CyPortInteractionReport {
        status,
        source_id: "cyport_us_port_tropical_cyclone_resilience",
        source_revision_expected: PINNED_REVISION,
        source_path_expected: "Tropical_cyclone_interaction/interaction_data.csv",
        source_git_blob_sha1_expected: PINNED_GIT_BLOB_SHA1,
        source_git_blob_sha1_observed: observed_blob_sha1,
        source_sha256_observed: super::sha256_bytes(bytes),
        source_byte_length: bytes.len(),
        pinned_source_identity_verified,
        column_count: header.len(),
        data_row_count: records.len(),
        unique_interaction_ids: counts.ids.len(),
        duplicate_interaction_id_rows: counts.duplicate_id_rows,
        unique_storm_ids: counts.storm_ids.len(),
        unique_port_ids: counts.port_ids.len(),
        unique_storm_port_pairs: counts.storm_port_pairs.len(),
        duplicate_storm_port_pair_rows: counts.duplicate_pair_rows,
        zero_total_impact_rows: counts.zero_total_impact_rows,
        nonzero_total_impact_rows: counts.nonzero_total_impact_rows,
        zero_impact_all_recovery_sentinels_rows: counts.zero_impact_all_recovery_sentinels,
        zero_impact_all_recovery_fields_present_rows: counts
            .zero_impact_all_recovery_fields_present,
        nonzero_impact_all_recovery_fields_present_rows: counts
            .nonzero_impact_all_recovery_fields_present,
        no_impact_window_recorded_rows: counts.zero_impact_all_recovery_sentinels,
        impact_window_recovery_markers_present_rows: counts.zero_impact_all_recovery_fields_present
            + counts.nonzero_impact_all_recovery_fields_present,
        mixed_recovery_field_state_rows: counts.mixed_recovery_field_states,
        nonzero_impact_without_recovery_markers_rows: counts
            .nonzero_impact_without_recovery_markers,
        start_recovery_date_sentinel_rows: counts.start_recovery_sentinels,
        end_recovery_date_sentinel_rows: counts.end_recovery_sentinels,
        day_of_recover_sentinel_rows: counts.day_of_recover_sentinels,
        earliest_event_start: counts.event_starts[0].clone(),
        latest_event_start: counts.event_starts[counts.event_starts.len() - 1].clone(),
        earliest_event_end: counts.event_ends[0].clone(),
        latest_event_end: counts.event_ends[counts.event_ends.len() - 1].clone(),
        episode_split_policy_id: EPISODE_SPLIT_POLICY_ID,
        split_summaries,
        split_outcome_summaries,
        episode_groups,
        data_capture_persisted: false,
        model_validation_performed: false,
        physical_simulation_performed: false,
        claim_ceiling: "Local source-structure and profile check only; no prediction, causal inference, physical simulation, model validation or operational claim.",
    })
}

const MAX_SPLIT_MANIFEST_BYTES: usize = 500_000;
const SPLIT_ROOT_KEYS: &[&str] = &[
    "schema_id",
    "schema_version",
    "status",
    "source",
    "policy",
    "split_summary",
    "total_unique_storms",
    "total_interaction_rows",
    "outcomes_excluded_from_manifest",
    "episodes",
];
const SPLIT_SOURCE_KEYS: &[&str] = &[
    "repository",
    "revision",
    "path",
    "git_blob_sha1",
    "byte_length",
    "license_statement",
    "license_uri",
    "citation",
];
const SPLIT_POLICY_KEYS: &[&str] = &[
    "split_unit",
    "date_basis",
    "train",
    "calibration",
    "test",
    "leakage_rule",
    "external_holdout_caveat",
];
const SPLIT_SUMMARY_KEYS: &[&str] = &[
    "storms",
    "interaction_rows",
    "first_event_start",
    "last_event_start",
];
const EPISODE_KEYS: &[&str] = &[
    "sid",
    "first_event_start",
    "last_event_start",
    "interaction_rows",
    "split",
];

#[derive(Debug, Serialize, PartialEq, Eq)]
struct SplitManifestValidationReport {
    status: &'static str,
    schema_id: &'static str,
    schema_version: &'static str,
    manifest_sha256: String,
    manifest_byte_length: usize,
    source_revision: &'static str,
    source_path: &'static str,
    source_git_blob_sha1: &'static str,
    episode_count: usize,
    interaction_row_count: u64,
    split_summaries: Vec<SplitSummary>,
    source_bytes_compared: bool,
    predictive_validation_performed: bool,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct EpisodeSplitComparisonReport {
    status: &'static str,
    source_git_blob_sha1_observed: String,
    source_sha256_observed: String,
    source_byte_length: usize,
    source_identity_verified: bool,
    split_manifest_sha256: String,
    split_manifest_byte_length: usize,
    cyclone_groups_compared: usize,
    interaction_rows_compared: usize,
    split_summaries: Vec<SplitSummary>,
    data_capture_persisted: bool,
    predictive_validation_performed: bool,
    claim_ceiling: &'static str,
}

struct ParsedSplitManifest {
    sha256: String,
    byte_length: usize,
    episodes: Vec<StormEpisodeSplit>,
    split_summaries: Vec<SplitSummary>,
    total_interaction_rows: u64,
}

fn manifest_text<'a>(root: &'a JsonValue, key: &str, path: &str) -> Result<&'a str, String> {
    super::text(super::field(root, key, path)?, &format!("{path}.{key}"))
}

fn valid_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| index == 4 || index == 7 || byte.is_ascii_digit())
    {
        return false;
    }
    let year = match value[0..4].parse::<u32>() {
        Ok(value) if value > 0 => value,
        _ => return false,
    };
    let month = match value[5..7].parse::<u32>() {
        Ok(value) => value,
        _ => return false,
    };
    let day = match value[8..10].parse::<u32>() {
        Ok(value) => value,
        _ => return false,
    };
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let max_day = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 0,
    };
    day > 0 && day <= max_day
}

fn split_from_iso_date(date: &str) -> Result<&'static str, String> {
    if !valid_iso_date(date) {
        return Err(format!("invalid canonical ISO date {date:?}"));
    }
    let year = date[0..4]
        .parse::<u32>()
        .map_err(|_| format!("invalid date year in {date:?}"))?;
    Ok(if year <= 2020 {
        "train"
    } else if year == 2021 {
        "calibration"
    } else {
        "test"
    })
}

fn compute_split_summaries(episodes: &[StormEpisodeSplit]) -> Result<Vec<SplitSummary>, String> {
    let names = ["train", "calibration", "test"];
    let mut result = Vec::with_capacity(names.len());
    for name in names {
        let group: Vec<&StormEpisodeSplit> = episodes
            .iter()
            .filter(|episode| episode.split == name)
            .collect();
        let mut row_count = 0u64;
        let mut first_dates = Vec::new();
        let mut last_dates = Vec::new();
        for episode in &group {
            row_count = row_count
                .checked_add(episode.interaction_rows as u64)
                .ok_or_else(|| format!("split {name}: interaction row count overflow"))?;
            first_dates.push(episode.first_event_start.as_str());
            last_dates.push(episode.last_event_start.as_str());
        }
        first_dates.sort_unstable();
        last_dates.sort_unstable();
        let first = first_dates
            .first()
            .ok_or_else(|| format!("split {name} must contain at least one episode"))?;
        let last = last_dates
            .last()
            .ok_or_else(|| format!("split {name} must contain at least one episode"))?;
        result.push(SplitSummary {
            split: match name {
                "train" => "train",
                "calibration" => "calibration",
                _ => "test",
            },
            storm_count: group.len(),
            interaction_rows: usize::try_from(row_count)
                .map_err(|_| format!("split {name}: row count overflows platform size"))?,
            unique_port_ids: None,
            first_event_start: Some(first.to_string()),
            last_event_start: Some(last.to_string()),
        });
    }
    Ok(result)
}

fn parse_episode_split_manifest(bytes: &[u8]) -> Result<(JsonValue, ParsedSplitManifest), String> {
    if bytes.len() > MAX_SPLIT_MANIFEST_BYTES {
        return Err(format!(
            "CyPort episode split manifest exceeds {MAX_SPLIT_MANIFEST_BYTES} bytes"
        ));
    }
    let root = super::parse_strict(bytes)?;
    super::exact_keys(&root, SPLIT_ROOT_KEYS, "split_manifest")?;
    super::expect_text(
        super::field(&root, "schema_id", "split_manifest")?,
        "split_manifest.schema_id",
        "luminous.symtropy.maritime.cyport-episode-split",
    )?;
    super::expect_text(
        super::field(&root, "schema_version", "split_manifest")?,
        "split_manifest.schema_version",
        "0.1.0",
    )?;
    super::expect_text(
        super::field(&root, "status", "split_manifest")?,
        "split_manifest.status",
        "FROZEN_PRE_FIT_TEMPORAL_SPLIT_FROM_PINNED_SOURCE",
    )?;
    if !super::bool_value(
        super::field(&root, "outcomes_excluded_from_manifest", "split_manifest")?,
        "split_manifest.outcomes_excluded_from_manifest",
    )? {
        return Err("split_manifest.outcomes_excluded_from_manifest must remain true".to_owned());
    }

    let source = super::field(&root, "source", "split_manifest")?;
    super::exact_keys(source, SPLIT_SOURCE_KEYS, "split_manifest.source")?;
    super::expect_text(
        super::field(source, "repository", "split_manifest.source")?,
        "split_manifest.source.repository",
        "https://github.com/ChenchenMobility/Maritime-Data-CyPort",
    )?;
    super::expect_text(
        super::field(source, "revision", "split_manifest.source")?,
        "split_manifest.source.revision",
        PINNED_REVISION,
    )?;
    super::expect_text(
        super::field(source, "path", "split_manifest.source")?,
        "split_manifest.source.path",
        "Tropical_cyclone_interaction/interaction_data.csv",
    )?;
    super::expect_text(
        super::field(source, "git_blob_sha1", "split_manifest.source")?,
        "split_manifest.source.git_blob_sha1",
        PINNED_GIT_BLOB_SHA1,
    )?;
    if super::integer(
        super::field(source, "byte_length", "split_manifest.source")?,
        "split_manifest.source.byte_length",
    )? != PINNED_BYTES as u64
    {
        return Err("split_manifest source byte length does not match the pinned CSV".to_owned());
    }
    super::expect_text(
        super::field(source, "license_uri", "split_manifest.source")?,
        "split_manifest.source.license_uri",
        "https://creativecommons.org/licenses/by/4.0/",
    )?;

    let policy = super::field(&root, "policy", "split_manifest")?;
    super::exact_keys(policy, SPLIT_POLICY_KEYS, "split_manifest.policy")?;
    let expected_policy = [
        ("split_unit", "cyclone SID"),
        (
            "date_basis",
            "earliest start_date among all interaction rows belonging to the SID",
        ),
        ("train", "earliest event start on or before 2020-12-31"),
        (
            "calibration",
            "earliest event start during calendar year 2021",
        ),
        ("test", "earliest event start on or after 2022-01-01"),
        (
            "leakage_rule",
            "Every port-interaction row associated with one SID belongs to exactly one split. No row-level or port-row randomization is permitted.",
        ),
    ];
    for (key, expected) in expected_policy {
        super::expect_text(
            super::field(policy, key, "split_manifest.policy")?,
            &format!("split_manifest.policy.{key}"),
            expected,
        )?;
    }
    let caveat = manifest_text(policy, "external_holdout_caveat", "split_manifest.policy")?;
    if !caveat.contains("not a secret blind test") {
        return Err("split_manifest must state public-data holdout limitations".to_owned());
    }

    let episodes_json = super::array(
        super::field(&root, "episodes", "split_manifest")?,
        "split_manifest.episodes",
    )?;
    if episodes_json.is_empty() {
        return Err("split_manifest.episodes must not be empty".to_owned());
    }
    let mut episodes = Vec::with_capacity(episodes_json.len());
    let mut storm_ids = BTreeSet::new();
    let mut previous_sort_key: Option<(String, String)> = None;
    let mut computed_rows = 0u64;
    for (index, value) in episodes_json.iter().enumerate() {
        let path = format!("split_manifest.episodes[{index}]");
        super::exact_keys(value, EPISODE_KEYS, &path)?;
        let storm_id = manifest_text(value, "sid", &path)?.to_owned();
        if storm_id.trim().is_empty() || !storm_ids.insert(storm_id.clone()) {
            return Err(format!("{path}.sid: empty or duplicate cyclone ID"));
        }
        let first = manifest_text(value, "first_event_start", &path)?.to_owned();
        let last = manifest_text(value, "last_event_start", &path)?.to_owned();
        if !valid_iso_date(&first) || !valid_iso_date(&last) || last < first {
            return Err(format!("{path}: invalid or reverse event-start date range"));
        }
        let expected_split = split_from_iso_date(&first)?;
        super::expect_text(
            super::field(value, "split", &path)?,
            &format!("{path}.split"),
            expected_split,
        )?;
        let rows = super::integer(
            super::field(value, "interaction_rows", &path)?,
            &format!("{path}.interaction_rows"),
        )?;
        if rows == 0 {
            return Err(format!("{path}.interaction_rows must be positive"));
        }
        computed_rows = computed_rows
            .checked_add(rows)
            .ok_or_else(|| "split manifest interaction row count overflow".to_owned())?;
        let sort_key = (first.clone(), storm_id.clone());
        if previous_sort_key
            .as_ref()
            .is_some_and(|previous| previous > &sort_key)
        {
            return Err(format!(
                "{path}: episodes must be sorted by first date then cyclone ID"
            ));
        }
        previous_sort_key = Some(sort_key);
        episodes.push(StormEpisodeSplit {
            storm_id,
            first_event_start: first,
            last_event_start: last,
            split: expected_split,
            interaction_rows: usize::try_from(rows)
                .map_err(|_| format!("{path}.interaction_rows overflows platform size"))?,
            unique_port_ids: 0,
        });
    }
    if episodes.len() != 90 {
        return Err(format!(
            "split manifest: expected 90 cyclone groups, found {}",
            episodes.len()
        ));
    }
    if super::integer(
        super::field(&root, "total_unique_storms", "split_manifest")?,
        "split_manifest.total_unique_storms",
    )? != episodes.len() as u64
    {
        return Err("split_manifest.total_unique_storms does not match episode count".to_owned());
    }
    let declared_rows = super::integer(
        super::field(&root, "total_interaction_rows", "split_manifest")?,
        "split_manifest.total_interaction_rows",
    )?;
    if declared_rows != computed_rows || computed_rows != 1_927 {
        return Err(format!(
            "split manifest row count mismatch: episodes sum to {computed_rows}, declared {declared_rows}"
        ));
    }

    let summaries_json = super::field(&root, "split_summary", "split_manifest")?;
    super::exact_keys(
        summaries_json,
        &["train", "calibration", "test"],
        "split_manifest.split_summary",
    )?;
    let computed_summaries = compute_split_summaries(&episodes)?;
    for summary in &computed_summaries {
        let actual = super::field(
            summaries_json,
            summary.split,
            "split_manifest.split_summary",
        )?;
        super::exact_keys(
            actual,
            SPLIT_SUMMARY_KEYS,
            &format!("split_manifest.split_summary.{}", summary.split),
        )?;
        let actual_storms = super::integer(
            super::field(actual, "storms", summary.split)?,
            "split_summary.storms",
        )?;
        let actual_rows = super::integer(
            super::field(actual, "interaction_rows", summary.split)?,
            "split_summary.interaction_rows",
        )?;
        let actual_first = manifest_text(actual, "first_event_start", summary.split)?;
        let actual_last = manifest_text(actual, "last_event_start", summary.split)?;
        if actual_storms != summary.storm_count as u64
            || actual_rows != summary.interaction_rows as u64
            || Some(actual_first) != summary.first_event_start.as_deref()
            || Some(actual_last) != summary.last_event_start.as_deref()
        {
            return Err(format!(
                "split manifest summary drift for {}",
                summary.split
            ));
        }
    }

    Ok((
        root,
        ParsedSplitManifest {
            sha256: super::sha256_bytes(bytes),
            byte_length: bytes.len(),
            episodes,
            split_summaries: computed_summaries,
            total_interaction_rows: computed_rows,
        },
    ))
}

fn validate_and_report_split_manifest(
    bytes: &[u8],
) -> Result<SplitManifestValidationReport, String> {
    let (_root, manifest) = parse_episode_split_manifest(bytes)?;
    Ok(SplitManifestValidationReport {
        status: "EPISODE_SPLIT_MANIFEST_VALIDATED_NOT_SOURCE_RECOMPUTED",
        schema_id: "luminous.symtropy.maritime.cyport-episode-split",
        schema_version: "0.1.0",
        manifest_sha256: manifest.sha256,
        manifest_byte_length: manifest.byte_length,
        source_revision: PINNED_REVISION,
        source_path: "Tropical_cyclone_interaction/interaction_data.csv",
        source_git_blob_sha1: PINNED_GIT_BLOB_SHA1,
        episode_count: manifest.episodes.len(),
        interaction_row_count: manifest.total_interaction_rows,
        split_summaries: manifest.split_summaries,
        source_bytes_compared: false,
        predictive_validation_performed: false,
        claim_ceiling: "Frozen split-manifest structure and internal totals checked only; raw source bytes and all episode assignments have not been recomputed by this command.",
    })
}

fn compare_episode_manifest_to_profile(
    manifest: &ParsedSplitManifest,
    profile: &CyPortInteractionReport,
) -> Result<(), String> {
    if !profile.pinned_source_identity_verified {
        return Err(
            "source identity must match pinned CyPort Git blob before split comparison".to_owned(),
        );
    }
    if manifest.episodes.len() != profile.episode_groups.len() {
        return Err(format!(
            "source/manifest cyclone count mismatch: {} vs {}",
            profile.episode_groups.len(),
            manifest.episodes.len()
        ));
    }
    for (index, (expected, actual)) in manifest
        .episodes
        .iter()
        .zip(&profile.episode_groups)
        .enumerate()
    {
        if expected.storm_id != actual.storm_id
            || expected.first_event_start != actual.first_event_start
            || expected.last_event_start != actual.last_event_start
            || expected.split != actual.split
            || expected.interaction_rows != actual.interaction_rows
        {
            return Err(format!(
                "CyPort episode split mismatch at sorted entry {index}: manifest SID {:?}, source SID {:?}",
                expected.storm_id, actual.storm_id
            ));
        }
    }
    if manifest.total_interaction_rows as usize != profile.data_row_count {
        return Err("source/manifest interaction row count mismatch".to_owned());
    }
    let comparable: Vec<(&str, usize, usize)> = manifest
        .split_summaries
        .iter()
        .map(|summary| (summary.split, summary.storm_count, summary.interaction_rows))
        .collect();
    let source_comparable: Vec<(&str, usize, usize)> = profile
        .split_summaries
        .iter()
        .map(|summary| (summary.split, summary.storm_count, summary.interaction_rows))
        .collect();
    if comparable != source_comparable {
        return Err("source/manifest split summary mismatch".to_owned());
    }
    Ok(())
}

pub(super) fn validate_episode_split_manifest_cli(manifest_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(manifest_path).map_err(|error| {
        format!("cannot stat CyPort episode split manifest {manifest_path}: {error}")
    })?;
    if metadata.len() > MAX_SPLIT_MANIFEST_BYTES as u64 {
        return Err(format!(
            "CyPort split manifest exceeds {MAX_SPLIT_MANIFEST_BYTES} bytes"
        ));
    }
    let bytes = fs::read(manifest_path).map_err(|error| {
        format!("cannot read CyPort episode split manifest {manifest_path}: {error}")
    })?;
    let report = validate_and_report_split_manifest(&bytes)?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize split validation report: {error}"))?
    );
    Ok(())
}

pub(super) fn verify_episode_split_cli(csv_path: &str, manifest_path: &str) -> Result<(), String> {
    let csv_metadata = fs::metadata(csv_path)
        .map_err(|error| format!("cannot stat CyPort interaction CSV {csv_path}: {error}"))?;
    if csv_metadata.len() > MAX_CSV_BYTES as u64 {
        return Err(format!(
            "CyPort interaction CSV exceeds the {}-byte limit",
            MAX_CSV_BYTES
        ));
    }
    let manifest_metadata = fs::metadata(manifest_path)
        .map_err(|error| format!("cannot stat CyPort split manifest {manifest_path}: {error}"))?;
    if manifest_metadata.len() > MAX_SPLIT_MANIFEST_BYTES as u64 {
        return Err(format!(
            "CyPort split manifest exceeds {MAX_SPLIT_MANIFEST_BYTES} bytes"
        ));
    }
    let csv_bytes = fs::read(csv_path)
        .map_err(|error| format!("cannot read CyPort interaction CSV {csv_path}: {error}"))?;
    let manifest_bytes = fs::read(manifest_path)
        .map_err(|error| format!("cannot read CyPort split manifest {manifest_path}: {error}"))?;
    let profile = profile_bytes(&csv_bytes)?;
    let (_, manifest) = parse_episode_split_manifest(&manifest_bytes)?;
    compare_episode_manifest_to_profile(&manifest, &profile)?;
    let report = EpisodeSplitComparisonReport {
        status: "PINNED_SOURCE_MATCHES_FROZEN_EPISODE_SPLIT_NOT_MODEL_VALIDATED",
        source_git_blob_sha1_observed: profile.source_git_blob_sha1_observed,
        source_sha256_observed: profile.source_sha256_observed,
        source_byte_length: profile.source_byte_length,
        source_identity_verified: profile.pinned_source_identity_verified,
        split_manifest_sha256: manifest.sha256,
        split_manifest_byte_length: manifest.byte_length,
        cyclone_groups_compared: manifest.episodes.len(),
        interaction_rows_compared: manifest.total_interaction_rows as usize,
        split_summaries: manifest.split_summaries,
        data_capture_persisted: false,
        predictive_validation_performed: false,
        claim_ceiling: "Pinned source-to-split reproducibility only; no predictive/causal validation, safety claim, or operational authority.",
    };
    println!(
        "{}",
        serde_json::to_string(&report).map_err(|error| {
            format!("cannot serialize source-to-split verification report: {error}")
        })?
    );
    Ok(())
}

pub(super) fn run_cli(csv_path: &str) -> Result<(), String> {
    let metadata = fs::metadata(csv_path)
        .map_err(|error| format!("cannot stat CyPort interaction CSV {csv_path}: {error}"))?;
    if metadata.len() > MAX_CSV_BYTES as u64 {
        return Err(format!(
            "CyPort interaction CSV exceeds the {}-byte limit",
            MAX_CSV_BYTES
        ));
    }
    let bytes = fs::read(csv_path)
        .map_err(|error| format!("cannot read CyPort interaction CSV {csv_path}: {error}"))?;
    let report = profile_bytes(&bytes)?;
    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("cannot serialize CyPort profile: {error}"))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const SYNTHETIC_FIXTURE: &[u8] =
        include_bytes!("../tests/fixtures/cyport-interaction-profile-synthetic.csv");

    #[test]
    fn synthetic_fixture_profiles_without_claiming_source_identity() {
        let report = profile_bytes(SYNTHETIC_FIXTURE).expect("synthetic fixture should profile");
        assert_eq!(report.status, "SCHEMA_VALID_PROFILE_NOT_PINNED");
        assert_eq!(report.column_count, 56);
        assert_eq!(report.data_row_count, 5);
        assert_eq!(report.unique_storm_ids, 2);
        assert_eq!(report.unique_port_ids, 4);
        assert_eq!(report.unique_storm_port_pairs, 5);
        assert_eq!(report.zero_total_impact_rows, 3);
        assert_eq!(report.nonzero_total_impact_rows, 2);
        assert_eq!(report.zero_impact_all_recovery_sentinels_rows, 2);
        assert_eq!(report.zero_impact_all_recovery_fields_present_rows, 1);
        assert_eq!(report.nonzero_impact_all_recovery_fields_present_rows, 2);
        assert_eq!(report.no_impact_window_recorded_rows, 2);
        assert_eq!(report.impact_window_recovery_markers_present_rows, 3);
        assert_eq!(report.mixed_recovery_field_state_rows, 0);
        assert_eq!(report.start_recovery_date_sentinel_rows, 2);
        assert!(!report.pinned_source_identity_verified);
        assert!(!report.data_capture_persisted);
        assert!(!report.model_validation_performed);
        assert!(!report.physical_simulation_performed);
    }

    #[test]
    fn empty_synthetic_splits_report_unknown_date_bounds_not_panics() {
        let report = profile_bytes(SYNTHETIC_FIXTURE).expect("synthetic fixture should profile");
        let train = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "train")
            .unwrap();
        let calibration = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "calibration")
            .unwrap();
        let test = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "test")
            .unwrap();
        assert_eq!(train.storm_count, 0);
        assert_eq!(train.first_event_start, None);
        assert_eq!(train.last_event_start, None);
        assert_eq!(calibration.storm_count, 0);
        assert_eq!(calibration.first_event_start, None);
        assert_eq!(calibration.last_event_start, None);
        assert_eq!(test.storm_count, 2);
        assert_eq!(test.first_event_start.as_deref(), Some("2022-10-01"));
        assert_eq!(test.last_event_start.as_deref(), Some("2023-09-12"));
    }

    #[test]
    fn all_rows_for_each_synthetic_storm_remain_in_one_temporal_split() {
        let report = profile_bytes(SYNTHETIC_FIXTURE).expect("fixture should profile");
        assert_eq!(report.episode_split_policy_id, EPISODE_SPLIT_POLICY_ID);
        assert_eq!(report.episode_groups.len(), 2);
        assert_eq!(report.episode_groups[0].storm_id, "SYNTHETIC_STORM_A");
        assert_eq!(report.episode_groups[0].interaction_rows, 3);
        assert_eq!(report.episode_groups[0].split, "test");
        assert_eq!(report.episode_groups[1].storm_id, "SYNTHETIC_STORM_B");
        assert_eq!(report.episode_groups[1].interaction_rows, 2);
        assert_eq!(report.episode_groups[1].split, "test");
        let test = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "test")
            .unwrap();
        assert_eq!(test.storm_count, 2);
        assert_eq!(test.interaction_rows, 5);
        assert_eq!(test.unique_port_ids, Some(4));
        let split_outcomes = report
            .split_outcome_summaries
            .iter()
            .find(|summary| summary.split == "test")
            .unwrap();
        assert_eq!(split_outcomes.zero_total_impact_rows, 3);
        assert_eq!(split_outcomes.nonzero_total_impact_rows, 2);
        assert_eq!(split_outcomes.no_impact_window_recorded_rows, 2);
        assert_eq!(
            split_outcomes.impact_window_recovery_markers_present_rows,
            3
        );
        assert_eq!(split_outcomes.mixed_recovery_field_state_rows, 0);
    }

    const FROZEN_SPLIT_MANIFEST: &[u8] =
        include_bytes!("../../../../docs/tech/CYPORT_EPISODE_SPLIT_V0_1.json");

    #[test]
    fn frozen_cyclone_split_manifest_has_consistent_totals_and_boundaries() {
        let report = validate_and_report_split_manifest(FROZEN_SPLIT_MANIFEST)
            .expect("frozen split manifest should validate");
        assert_eq!(
            report.status,
            "EPISODE_SPLIT_MANIFEST_VALIDATED_NOT_SOURCE_RECOMPUTED"
        );
        assert_eq!(report.episode_count, 90);
        assert_eq!(report.interaction_row_count, 1_927);
        let train = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "train")
            .unwrap();
        assert_eq!(train.storm_count, 59);
        assert_eq!(train.interaction_rows, 1_471);
        assert_eq!(train.unique_port_ids, None);
        assert_eq!(train.first_event_start.as_deref(), Some("2015-05-06"));
        assert_eq!(train.last_event_start.as_deref(), Some("2020-11-13"));
        let calibration = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "calibration")
            .unwrap();
        assert_eq!(calibration.storm_count, 13);
        assert_eq!(calibration.interaction_rows, 308);
        let test = report
            .split_summaries
            .iter()
            .find(|summary| summary.split == "test")
            .unwrap();
        assert_eq!(test.storm_count, 18);
        assert_eq!(test.interaction_rows, 148);
        assert!(!report.source_bytes_compared);
        assert!(!report.predictive_validation_performed);
    }

    #[test]
    fn split_manifest_digest_binds_exact_bytes() {
        let first = validate_and_report_split_manifest(FROZEN_SPLIT_MANIFEST).unwrap();
        let second = validate_and_report_split_manifest(FROZEN_SPLIT_MANIFEST).unwrap();
        assert_eq!(first.manifest_sha256, second.manifest_sha256);
        let changed = [FROZEN_SPLIT_MANIFEST, &b"\n"[..]].concat();
        let third = validate_and_report_split_manifest(&changed).unwrap();
        assert_ne!(first.manifest_sha256, third.manifest_sha256);
        assert_eq!(first.episode_count, third.episode_count);
    }

    #[test]
    fn duplicate_cyclone_id_is_rejected() {
        let mut root: Value = serde_json::from_slice(FROZEN_SPLIT_MANIFEST).unwrap();
        let first_sid = root["episodes"][0]["sid"].clone();
        root["episodes"][1]["sid"] = first_sid;
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_and_report_split_manifest(&changed)
                .unwrap_err()
                .contains("duplicate cyclone ID")
        );
    }

    #[test]
    fn assigning_a_storm_to_the_wrong_temporal_split_is_rejected() {
        let mut root: Value = serde_json::from_slice(FROZEN_SPLIT_MANIFEST).unwrap();
        root["episodes"][0]["split"] = Value::String("test".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_and_report_split_manifest(&changed)
                .unwrap_err()
                .contains("split_manifest.episodes[0].split")
        );
    }

    #[test]
    fn episode_row_count_drift_is_rejected() {
        let mut root: Value = serde_json::from_slice(FROZEN_SPLIT_MANIFEST).unwrap();
        root["episodes"][0]["interaction_rows"] = Value::Number(serde_json::Number::from(28));
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_and_report_split_manifest(&changed)
                .unwrap_err()
                .contains("row count mismatch")
        );
    }

    #[test]
    fn mutable_or_incorrect_source_blob_is_rejected() {
        let mut root: Value = serde_json::from_slice(FROZEN_SPLIT_MANIFEST).unwrap();
        root["source"]["git_blob_sha1"] =
            Value::String("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(validate_and_report_split_manifest(&changed).is_err());
    }

    #[test]
    fn manifest_must_disclose_that_public_data_is_not_a_blind_holdout() {
        let mut root: Value = serde_json::from_slice(FROZEN_SPLIT_MANIFEST).unwrap();
        root["policy"]["external_holdout_caveat"] = Value::String("blind test".to_owned());
        let changed = serde_json::to_vec(&root).unwrap();
        assert!(
            validate_and_report_split_manifest(&changed)
                .unwrap_err()
                .contains("public-data holdout limitations")
        );
    }

    #[test]
    fn a_synthetic_profile_cannot_claim_a_pinned_source_split_match() {
        let profile = profile_bytes(SYNTHETIC_FIXTURE).unwrap();
        let (_, manifest) = parse_episode_split_manifest(FROZEN_SPLIT_MANIFEST).unwrap();
        assert!(
            compare_episode_manifest_to_profile(&manifest, &profile)
                .unwrap_err()
                .contains("source identity must match pinned CyPort Git blob")
        );
    }

    #[test]
    fn empty_git_blob_hash_matches_git_object_vector() {
        assert_eq!(
            git_blob_sha1(b""),
            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391"
        );
    }

    #[test]
    fn pinned_length_with_wrong_git_blob_identity_fails_closed() {
        let bytes = vec![b'x'; PINNED_BYTES];
        let error = profile_bytes(&bytes).unwrap_err();
        assert!(error.contains("Git blob SHA-1 mismatch"));
    }

    #[test]
    fn header_drift_is_rejected() {
        let mut bytes = SYNTHETIC_FIXTURE.to_vec();
        let marker = b"PORT_LON";
        let position = bytes
            .windows(marker.len())
            .position(|window| window == marker)
            .unwrap();
        bytes[position] = b'X';
        assert!(profile_bytes(&bytes).unwrap_err().contains("header column"));
    }

    #[test]
    fn ragged_data_row_is_rejected() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let last_line = text.lines().last().unwrap();
        let truncated = last_line.split(',').take(10).collect::<Vec<_>>().join(",");
        let changed = format!(
            "{}\n{}\n",
            text.lines().take(4).collect::<Vec<_>>().join("\n"),
            truncated
        );
        assert!(
            profile_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("fields; expected")
        );
    }

    #[test]
    fn unbalanced_quoted_field_is_rejected() {
        let bytes = b"ID,SID\n1,\"unterminated";
        assert!(
            parse_csv(bytes)
                .unwrap_err()
                .contains("inside a quoted field")
        );
    }

    #[test]
    fn duplicate_interaction_ids_are_counted_for_schema_fixture() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let mut rows: Vec<String> = text.lines().map(str::to_owned).collect();
        let first = rows[1].clone();
        let mut second_fields: Vec<String> = first.split(',').map(str::to_owned).collect();
        second_fields[0] = "1".to_owned();
        rows[2] = second_fields.join(",");
        let changed = rows.join("\n") + "\n";
        let report = profile_bytes(changed.as_bytes()).unwrap();
        assert_eq!(report.duplicate_interaction_id_rows, 1);
    }

    #[test]
    fn impossible_event_date_is_rejected() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let changed = text.replacen("10/1/2022", "2/31/2022", 1);
        assert_ne!(text, changed);
        assert!(
            profile_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("impossible calendar date")
        );
    }

    #[test]
    fn invalid_recovery_time_suffix_is_rejected() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let changed = text.replacen("10/4/2022 0:00", "10/4/2022 24:00", 1);
        assert_ne!(text, changed);
        assert!(
            profile_bytes(changed.as_bytes())
                .unwrap_err()
                .contains("invalid HH:MM time")
        );
    }

    #[test]
    fn rounded_zero_impact_does_not_erase_present_recovery_markers() {
        let report = profile_bytes(SYNTHETIC_FIXTURE).expect("fixture should profile");
        assert_eq!(report.zero_total_impact_rows, 3);
        assert_eq!(report.zero_impact_all_recovery_sentinels_rows, 2);
        assert_eq!(report.zero_impact_all_recovery_fields_present_rows, 1);
        assert_eq!(report.no_impact_window_recorded_rows, 2);
        assert_eq!(report.impact_window_recovery_markers_present_rows, 3);
    }

    #[test]
    fn nonzero_impact_without_recovery_markers_is_not_mislabeled_mixed() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let target = text.lines().nth(2).unwrap();
        let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
        let fields: Vec<&str> = target.split(',').collect();
        let mut owned: Vec<String> = fields.iter().map(|x| x.to_string()).collect();
        owned[10] = "1".to_owned();
        owned[8] = "0".to_owned();
        owned[9] = "0".to_owned();
        owned[13] = "0".to_owned();
        lines[2] = owned.join(",");
        let changed = lines.join("\n") + "\n";
        let report = profile_bytes(changed.as_bytes()).unwrap();
        assert_eq!(report.nonzero_impact_without_recovery_markers_rows, 1);
        assert_eq!(report.mixed_recovery_field_state_rows, 0);
        let test = report
            .split_outcome_summaries
            .iter()
            .find(|x| x.split == "test")
            .unwrap();
        assert_eq!(test.nonzero_impact_without_recovery_markers_rows, 1);
    }

    #[test]
    fn partially_present_recovery_triplet_is_counted_as_mixed() {
        let text = String::from_utf8(SYNTHETIC_FIXTURE.to_vec()).unwrap();
        let changed = text.replacen("10/4/2022 0:00", "0", 1);
        assert_ne!(text, changed);
        let report = profile_bytes(changed.as_bytes()).unwrap();
        assert_eq!(report.mixed_recovery_field_state_rows, 1);
        assert_eq!(report.nonzero_impact_without_recovery_markers_rows, 0);
    }

    #[test]
    fn zero_sentinel_is_not_reported_as_a_recovery_time() {
        let report = profile_bytes(SYNTHETIC_FIXTURE).unwrap();
        assert_eq!(report.start_recovery_date_sentinel_rows, 2);
        assert_eq!(report.end_recovery_date_sentinel_rows, 2);
        assert_eq!(report.day_of_recover_sentinel_rows, 2);
    }
}
