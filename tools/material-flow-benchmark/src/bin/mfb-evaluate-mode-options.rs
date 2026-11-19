// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::mode_options::{ModeOptionsInput, evaluate_mode_options};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <mode-options.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one mode-options input path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read mode-options input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: ModeOptionsInput = match serde_json::from_str(&raw) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid mode-options input JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_mode_options(&input) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("mode-options input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode mode-options report: {error}");
            ExitCode::from(1)
        }
    }
}
