// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use serde_json::Value;
use wpt_energy_interface_validator::{evaluate_record, SessionValidator};

fn run() -> Result<(), String> {
    let path = env::args_os().nth(1)
        .ok_or_else(|| "usage: wpt-energy-interface-validator <known-truth.jsonl>".to_owned())?;
    let file = File::open(&path).map_err(|error| format!("cannot open input: {error}"))?;
    let reader = BufReader::new(file);
    let mut sessions = SessionValidator::default();
    let mut event_ids = HashSet::new();
    let mut mismatches = 0usize;
    let mut records = 0usize;

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|error| format!("cannot read line {line_number}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid JSON on line {line_number}: {error}"))?;
        let event_id = record.get("event_id").and_then(Value::as_str).unwrap_or("unknown");
        if !event_ids.insert(event_id.to_owned()) {
            eprintln!("duplicate event_id on line {line_number}: {event_id}");
            mismatches += 1;
        }
        let report = evaluate_record(&record, &mut sessions);
        let actual = report.get("disposition").and_then(Value::as_str).unwrap_or("INVALID_REPORT");
        if let Some(expected) = record.get("expected_disposition").and_then(Value::as_str)
            && expected != actual
        {
            eprintln!("line {line_number} ({event_id}): expected {expected}, got {actual}: {}", report.get("message").and_then(Value::as_str).unwrap_or(""));
            mismatches += 1;
        }
        if actual == "INVALID_INPUT" {
            eprintln!("line {line_number} ({event_id}): {}", report.get("message").and_then(Value::as_str).unwrap_or("invalid input"));
        }
        println!("{report}");
        records += 1;
    }
    if records == 0 {
        return Err("input contains no JSONL records".into());
    }
    if mismatches > 0 {
        return Err(format!("{mismatches} service-interface conformance checks failed"));
    }
    eprintln!("validated {records} WPT energy-service events");
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
