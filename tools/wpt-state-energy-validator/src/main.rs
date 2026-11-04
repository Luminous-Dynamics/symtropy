// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use serde_json::Value;
use wpt_state_energy_validator::evaluate;

fn run() -> Result<(), String> {
    let path = env::args_os()
        .nth(1)
        .ok_or_else(|| "usage: wpt-state-energy-validator <known-truth.jsonl>".to_owned())?;
    let file = File::open(&path).map_err(|error| format!("cannot open input: {error}"))?;
    let reader = BufReader::new(file);
    let mut seen = HashSet::new();
    let mut mismatches = 0usize;
    let mut count = 0usize;

    for (index, line) in reader.lines().enumerate() {
        let line_number = index + 1;
        let line = line.map_err(|error| format!("cannot read line {line_number}: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let record: Value = serde_json::from_str(&line)
            .map_err(|error| format!("invalid JSON on line {line_number}: {error}"))?;
        let id = record
            .get("record_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if !seen.insert(id.to_owned()) {
            eprintln!("duplicate record_id on line {line_number}: {id}");
            mismatches += 1;
        }
        let report = evaluate(&record);
        let actual = report
            .get("disposition")
            .and_then(Value::as_str)
            .unwrap_or("INVALID_REPORT");
        if let Some(expected) = record.get("expected_disposition").and_then(Value::as_str) {
            if expected != actual {
                eprintln!("line {line_number} ({id}): expected {expected}, got {actual}");
                mismatches += 1;
            }
        }
        println!("{report}");
        count += 1;
    }
    if count == 0 {
        return Err("input contains no JSONL records".into());
    }
    if mismatches != 0 {
        return Err(format!(
            "{mismatches} known-truth or identity checks failed"
        ));
    }
    eprintln!("validated {count} WPT state-energy records");
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
