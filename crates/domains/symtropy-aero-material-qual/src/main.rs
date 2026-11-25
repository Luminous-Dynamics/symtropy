// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
use std::env;
use std::fs;
use std::process::ExitCode;

use symtropy_aero_material_qual::evaluate_case;

fn main() -> ExitCode {
    let mut arguments = env::args_os();
    let program = arguments
        .next()
        .unwrap_or_else(|| "aero-material-qual".into());
    let Some(input_argument) = arguments.next() else {
        eprintln!(
            "Usage: {} <case.json>",
            std::path::Path::new(&program).display()
        );
        return ExitCode::from(2);
    };
    if arguments.next().is_some() {
        eprintln!("Expected exactly one case path.");
        return ExitCode::from(2);
    }

    let input_path = std::path::PathBuf::from(input_argument);
    let input = match fs::read_to_string(&input_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("Could not read {}: {error}", input_path.display());
            return ExitCode::FAILURE;
        }
    };

    let case: serde_json::Value = match serde_json::from_str(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Input is not valid JSON: {error}");
            return ExitCode::FAILURE;
        }
    };

    let report = evaluate_case(&case);
    if let Err(error) = serde_json::to_writer_pretty(std::io::stdout().lock(), &report) {
        eprintln!("Could not serialize evaluation report: {error}");
        return ExitCode::FAILURE;
    }
    println!();

    if report.structurally_complete {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
