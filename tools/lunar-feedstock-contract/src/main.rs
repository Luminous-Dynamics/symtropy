// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use lunar_feedstock_drift::compare_quality_samples;
use serde_json::Value;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!("usage: lunar-feedstock-drift RUN.json FROM_SAMPLE_ID TO_SAMPLE_ID");
        return ExitCode::FAILURE;
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {}: {error}", args[0]);
            return ExitCode::FAILURE;
        }
    };

    let run: Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid run JSON: {error}");
            return ExitCode::FAILURE;
        }
    };

    match compare_quality_samples(&run, &args[1], &args[2]) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(rendered) => {
                println!("{rendered}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize drift report: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("feedstock drift error: {error}");
            ExitCode::FAILURE
        }
    }
}
