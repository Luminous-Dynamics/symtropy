// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::generalization_split::validate_split_manifest;
use serde_json::Value;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 2 {
        eprintln!("Usage: {} <split-manifest.json>", args.first().map(String::as_str).unwrap_or("nav-generalization-validate"));
        return ExitCode::from(2);
    }

    let path = &args[1];
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("{path}: unable to read manifest: {error}");
            return ExitCode::from(2);
        }
    };
    let manifest: Value = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{path}: invalid JSON: {error}");
            return ExitCode::from(2);
        }
    };

    match validate_split_manifest(&manifest) {
        Ok(summary) => {
            println!(
                "GENERALIZATION_SPLIT_PASS fold={} protocol={} axis={} held_out={} records={} train={} validation={} test={} sequence_groups={} event_groups={} failure_domains={}",
                summary.fold_id,
                summary.protocol,
                summary.holdout_axis,
                summary.held_out_value.as_deref().unwrap_or("none"),
                summary.record_count,
                summary.train_count,
                summary.validation_count,
                summary.test_count,
                summary.sequence_group_count,
                summary.event_group_count,
                summary.failure_domain_count,
            );
            println!("LIMIT: manifest disjointness does not prove raw-data independence, oracle correctness, or rights clearance.");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("GENERALIZATION_SPLIT_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}
