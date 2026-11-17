// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use serde_json::Value;
use wpt_energy_interface_validator::{
    SessionValidator, evaluate_record, evaluate_transport_session_binding,
};

fn read_json(path: &OsString) -> Result<Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.to_string_lossy()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("invalid JSON in {}: {error}", path.to_string_lossy()))
}

fn run_binding(binding_path: OsString, context_path: OsString) -> Result<(), String> {
    let binding = read_json(&binding_path)?;
    let context = read_json(&context_path)?;
    let report = evaluate_transport_session_binding(&binding, &context);
    println!("{report}");
    match report.get("disposition").and_then(Value::as_str) {
        Some("METADATA_BOUND") => Ok(()),
        Some(disposition) => Err(format!("transport-session binding disposition: {disposition}")),
        None => Err("validator returned no binding disposition".into()),
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let first = args.next().ok_or_else(|| {
        "usage: wpt-energy-interface-validator <known-truth.jsonl> | validate-binding <binding.json> <trusted-context.json>"
            .to_owned()
    })?;
    if first.to_string_lossy() == "validate-binding" {
        let binding_path = args.next().ok_or_else(|| {
            "usage: wpt-energy-interface-validator validate-binding <binding.json> <trusted-context.json>"
                .to_owned()
        })?;
        let context_path = args.next().ok_or_else(|| {
            "usage: wpt-energy-interface-validator validate-binding <binding.json> <trusted-context.json>"
                .to_owned()
        })?;
        if args.next().is_some() {
            return Err("too many arguments for validate-binding".into());
        }
        return run_binding(binding_path, context_path);
    }
    if args.next().is_some() {
        return Err(
            "usage: wpt-energy-interface-validator <known-truth.jsonl> | validate-binding <binding.json> <trusted-context.json>"
                .into(),
        );
    }

    let file = fs::File::open(&first).map_err(|error| format!("cannot open input: {error}"))?;
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
        let event_id = record
            .get("event_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if !event_ids.insert(event_id.to_owned()) {
            eprintln!("duplicate event_id on line {line_number}: {event_id}");
            mismatches += 1;
        }
        let report = evaluate_record(&record, &mut sessions);
        let actual = report
            .get("disposition")
            .and_then(Value::as_str)
            .unwrap_or("INVALID_REPORT");
        if let Some(expected) = record.get("expected_disposition").and_then(Value::as_str)
            && expected != actual
        {
            eprintln!(
                "line {line_number} ({event_id}): expected {expected}, got {actual}: {}",
                report.get("message").and_then(Value::as_str).unwrap_or("")
            );
            mismatches += 1;
        }
        if actual == "INVALID_INPUT" {
            eprintln!(
                "line {line_number} ({event_id}): {}",
                report
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("invalid input")
            );
        }
        println!("{report}");
        records += 1;
    }
    if records == 0 {
        return Err("input contains no JSONL records".into());
    }
    if mismatches > 0 {
        return Err(format!(
            "{mismatches} service-interface conformance checks failed"
        ));
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
