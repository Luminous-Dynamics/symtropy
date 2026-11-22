// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::dataset_admission::validate_dataset_admission;
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
    eprintln!("Usage: {program} <dataset-schema.json> <datasets-manifest.json> <research_noncommercial|commercial>");
    eprintln!("Validates dataset admission evidence and requested rights scope.");
}

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        usage(args.first().map(String::as_str).unwrap_or("nav-dataset-admission-validate"));
        return ExitCode::from(2);
    }

    let schema = match read_json(&args[1]) {
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

    match validate_dataset_admission(&schema, &manifest, &args[3]) {
        Ok(summary) => {
            println!(
                "DATASET_ADMISSION_PASS use={} records={} admitted={} candidates={} quarantined={} unqualified_time={} unknown_time={} unqualified_frame={} unknown_frame={} clock_state_truth_not_provided={} clock_state_truth_unknown={} partial_ground_truth={}",
                summary.requested_use,
                summary.record_count,
                summary.admitted_count,
                summary.candidate_count,
                summary.quarantined_count,
                summary.unqualified_time_count,
                summary.unknown_time_count,
                summary.unqualified_frame_count,
                summary.unknown_frame_count,
                summary.clock_state_truth_not_provided_count,
                summary.clock_state_truth_unknown_count,
                summary.partial_ground_truth_count
            );
            println!(
                "LIMIT: candidate-only records are not approved benchmark payloads; no data files are downloaded or redistributed by this validator."
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("DATASET_ADMISSION_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}
