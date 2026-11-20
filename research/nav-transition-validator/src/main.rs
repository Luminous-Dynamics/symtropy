// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::validate_corpus;
use serde_json::Value;
use std::env;
use std::fs;
use std::process::ExitCode;

fn read_json(path: &str) -> Result<Value, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("{path}: unable to read file: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("{path}: invalid JSON: {error}"))
}

fn usage(program: &str) {
    eprintln!("Usage: {program} <fixtures.json> <manifest.json>");
    eprintln!("Performs semantic cross-record checks; it is not a JSON Schema validator.");
}

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        usage(args.first().map(String::as_str).unwrap_or("nav-transition-validate"));
        return ExitCode::from(2);
    }

    let fixture = match read_json(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let manifest = match read_json(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    match validate_corpus(&fixture, &manifest) {
        Ok(summary) => {
            println!(
                "STRUCTURAL_SEMANTIC_PASS cases={} sources={} phases={} observations={} transitions={} faults={} evaluator_truth_states={} positive_controls={} negative_controls={}",
                summary.case_count,
                summary.source_count,
                summary.phase_count,
                summary.observation_count,
                summary.transition_count,
                summary.fault_count,
                summary.evaluator_truth_state_count,
                summary.positive_control_count,
                summary.negative_control_count
            );
            println!(
                "LIMIT: this is not JSON Schema validation, physical-model validation, benchmark execution, CI qualification, or navigation safety approval."
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("STRUCTURAL_SEMANTIC_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}
