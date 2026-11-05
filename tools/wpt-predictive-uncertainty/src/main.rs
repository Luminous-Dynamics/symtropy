// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{env, fs, process::ExitCode};
use wpt_predictive_uncertainty::{EvaluationRecord, evaluate};

fn main() -> ExitCode {
    let path = match env::args().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: wpt-predictive-uncertainty <test-records.jsonl>");
            return ExitCode::from(2);
        }
    };
    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut records = Vec::new();
    for (index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<EvaluationRecord>(line) {
            Ok(record) => records.push(record),
            Err(error) => {
                eprintln!("invalid JSONL record on line {}: {error}", index + 1);
                return ExitCode::FAILURE;
            }
        }
    }
    match evaluate(&records).and_then(|report| {
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))
    }) {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("evaluation failed: {error}");
            ExitCode::FAILURE
        }
    }
}
