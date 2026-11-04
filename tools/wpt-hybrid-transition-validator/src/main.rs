// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use serde_json::Value;
use wpt_hybrid_transition_validator::evaluate;

fn validate_stream_order(
    last_by_stream: &mut HashMap<String, (u64, u64)>,
    stream_id: &str,
    sequence: u64,
    start_ns: u64,
    end_ns: u64,
) -> Result<(), String> {
    if stream_id.trim().is_empty() {
        return Err("transition_stream_id must not be empty".into());
    }
    if start_ns >= end_ns {
        return Err("event window must be a non-empty half-open interval".into());
    }
    if let Some((last_sequence, last_end_ns)) = last_by_stream.get(stream_id) {
        let expected_sequence = last_sequence
            .checked_add(1)
            .ok_or_else(|| format!("transition sequence overflow for stream {stream_id}"))?;
        if sequence != expected_sequence {
            return Err(format!(
                "stream {stream_id} sequence {sequence} is not contiguous; expected {expected_sequence}"
            ));
        }
        if start_ns < *last_end_ns {
            return Err(format!(
                "stream {stream_id} event window overlaps or is reordered: start {start_ns} precedes previous end {last_end_ns}"
            ));
        }
    }
    last_by_stream.insert(stream_id.to_owned(), (sequence, end_ns));
    Ok(())
}

fn run() -> Result<(), String> {
    let path = env::args_os()
        .nth(1)
        .ok_or_else(|| "usage: wpt-hybrid-transition-validator <known-truth.jsonl>".to_owned())?;
    let file = File::open(&path).map_err(|error| format!("cannot open input: {error}"))?;
    let reader = BufReader::new(file);
    let mut seen_ids = HashSet::new();
    let mut last_by_stream = HashMap::new();
    let mut mismatches = 0usize;
    let mut records = 0usize;

    for (line_index, line) in reader.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line.map_err(|error| format!("cannot read line {line_number}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }

        let record: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid JSON on line {line_number}: {error}"))?;
        let event_id = record
            .get("event_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        if !seen_ids.insert(event_id.to_owned()) {
            eprintln!("duplicate event_id on line {line_number}: {event_id}");
            mismatches += 1;
        }

        let order_result = (|| -> Result<(), String> {
            let stream_id = record
                .get("transition_stream_id")
                .and_then(Value::as_str)
                .ok_or_else(|| "missing transition_stream_id".to_owned())?;
            let sequence = record
                .get("transition_sequence")
                .and_then(Value::as_u64)
                .ok_or_else(|| "missing or invalid transition_sequence".to_owned())?;
            let window = record
                .get("event_window")
                .ok_or_else(|| "missing event_window".to_owned())?;
            let start_ns = window
                .get("start_ns")
                .and_then(Value::as_u64)
                .ok_or_else(|| "missing or invalid event_window.start_ns".to_owned())?;
            let end_ns = window
                .get("end_ns")
                .and_then(Value::as_u64)
                .ok_or_else(|| "missing or invalid event_window.end_ns".to_owned())?;
            validate_stream_order(&mut last_by_stream, stream_id, sequence, start_ns, end_ns)
        })();
        if let Err(error) = order_result {
            eprintln!("line {line_number} ({event_id}): {error}");
            mismatches += 1;
        }

        let report = evaluate(&record);
        let actual = report
            .get("disposition")
            .and_then(Value::as_str)
            .unwrap_or("INVALID_REPORT");
        let expected = record.get("expected_disposition").and_then(Value::as_str);

        if let Some(expected) = expected
            && expected != actual
        {
            eprintln!("line {line_number} ({event_id}): expected {expected}, got {actual}");
            mismatches += 1;
        }

        println!("{report}");
        records += 1;
    }

    if records == 0 {
        return Err("input contains no JSONL records".to_owned());
    }
    if mismatches > 0 {
        return Err(format!(
            "{mismatches} known-truth or identity checks failed"
        ));
    }

    eprintln!("validated {records} hybrid transition records");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_stream_order;
    use std::collections::HashMap;

    #[test]
    fn accepts_non_overlapping_contiguous_transition_windows() {
        let mut streams = HashMap::new();
        assert!(validate_stream_order(&mut streams, "roadway-1", 1, 100, 200).is_ok());
        assert!(validate_stream_order(&mut streams, "roadway-1", 2, 200, 300).is_ok());
        assert!(validate_stream_order(&mut streams, "roadway-2", 1, 100, 150).is_ok());
    }

    #[test]
    fn rejects_overlapping_or_reordered_transition_windows() {
        let mut streams = HashMap::new();
        assert!(validate_stream_order(&mut streams, "roadway-1", 1, 100, 200).is_ok());
        assert!(validate_stream_order(&mut streams, "roadway-1", 2, 150, 250).is_err());
    }

    #[test]
    fn rejects_sequence_gaps_and_replays() {
        let mut streams = HashMap::new();
        assert!(validate_stream_order(&mut streams, "roadway-1", 1, 100, 200).is_ok());
        assert!(validate_stream_order(&mut streams, "roadway-1", 3, 200, 300).is_err());
        assert!(validate_stream_order(&mut streams, "roadway-1", 1, 200, 300).is_err());
    }

    #[test]
    fn isolates_ordering_state_between_streams() {
        let mut streams = HashMap::new();
        assert!(validate_stream_order(&mut streams, "roadway-1", 1, 100, 200).is_ok());
        assert!(validate_stream_order(&mut streams, "roadway-2", 1, 100, 200).is_ok());
    }
}
