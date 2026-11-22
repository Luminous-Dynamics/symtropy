// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::generalization_split::validate_split_manifest;
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

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <schema.json> <split-manifest.json>",
            args.first().map(String::as_str).unwrap_or("nav-generalization-validate")
        );
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

    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            eprintln!("invalid Draft 2020-12 schema: {error}");
            return ExitCode::from(2);
        }
    };
    let schema_errors: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        eprintln!("GENERALIZATION_SCHEMA_FAIL errors={}", schema_errors.len());
        for error in schema_errors {
            eprintln!("- {error}");
        }
        return ExitCode::FAILURE;
    }
    println!("GENERALIZATION_SCHEMA_DRAFT_2020_12_PASS");

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
