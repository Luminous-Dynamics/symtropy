// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::schema_validation::validate_fixture_schema;
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
    eprintln!("Usage: {program} <schema.json> <fixtures.json> <manifest.json>");
    eprintln!("Runs JSON Schema Draft 2020-12 validation followed by semantic cross-record checks.");
}

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        usage(args.first().map(String::as_str).unwrap_or("nav-transition-validate"));
        return ExitCode::from(2);
    }

    let schema = match read_json(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let fixture = match read_json(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let manifest = match read_json(&args[3]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    match validate_fixture_schema(&schema, &fixture) {
        Ok(case_count) => println!("JSON_SCHEMA_DRAFT_2020_12_PASS cases={case_count}"),
        Err(errors) => {
            eprintln!("JSON_SCHEMA_DRAFT_2020_12_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::FAILURE;
        }
    }

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
                "LIMIT: schema and semantic passes do not establish physical-model validity, benchmark execution, navigation integrity or safety approval."
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
