// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use mfg_architecture_profile_validator::{parse_json, validate_profile};
use std::{env, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let schema_path = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../profile.schema.json"));
    let profile_paths: Vec<PathBuf> = args.map(PathBuf::from).collect();

    if profile_paths.is_empty() {
        eprintln!("usage: mfg-architecture-profile-validator <schema.json> <profile.json>...");
        return ExitCode::from(2);
    }

    let schema_bytes = match fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}: {error}", schema_path.display());
            return ExitCode::FAILURE;
        }
    };
    let schema = match parse_json(&schema_bytes, &schema_path.display().to_string()) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    let mut failures = 0usize;
    for profile_path in profile_paths {
        let bytes = match fs::read(&profile_path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("FAIL {}: {error}", profile_path.display());
                failures += 1;
                continue;
            }
        };
        let label = profile_path.display().to_string();
        let profile = match parse_json(&bytes, &label) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("FAIL {error}");
                failures += 1;
                continue;
            }
        };
        match validate_profile(&schema, &profile) {
            Ok(summary) => println!(
                "PASS {}: {} layers; scope={}; source={}",
                summary.profile_id,
                summary.layer_count,
                summary.execution_scope,
                summary.source_class
            ),
            Err(errors) => {
                eprintln!("FAIL {label}:");
                for error in errors {
                    eprintln!("  - {error}");
                }
                failures += 1;
            }
        }
    }

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        eprintln!("{failures} profile(s) failed validation");
        ExitCode::FAILURE
    }
}
