// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use civ_infra_benchmark::{evaluate_json, verify_report};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let _program = arguments.next();
    let Some(input_path) = arguments.next() else {
        eprintln!("usage: civ-infra-benchmark <fixture.json>");
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        eprintln!("error: expected exactly one fixture path");
        return ExitCode::from(2);
    }

    let input_path = input_path;
    let bytes = match fs::read(&input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not read {:?}: {error}", input_path);
            return ExitCode::from(2);
        }
    };

    let report = match evaluate_json(&bytes) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("fixture rejected: {error}");
            }
            return ExitCode::from(2);
        }
    };

    if let Err(errors) = verify_report(&bytes, &report) {
        for error in errors {
            eprintln!("reference verification failed: {error}");
        }
        return ExitCode::from(3);
    }

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: could not serialize report: {error}");
            ExitCode::from(4)
        }
    }
}
