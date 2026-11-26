// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use mobility_rrvr_paired_inference::evaluate_json;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: rrvr_paired_inference <manifest.json>");
        return ExitCode::from(2);
    };
    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("MANIFEST_READ_FAILED: {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match evaluate_json(&input) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(output) => {
                println!("{output}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("REPORT_SERIALIZATION_FAILED: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("INFERENCE_MANIFEST_INVALID: {error}");
            ExitCode::FAILURE
        }
    }
}
