// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use rural_industrial_electrification_evaluator::{
    CompatibilityInput,
    evaluate_compatibility,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <interface-contract.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one interface-contract input path");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read interface contract: {error}");
            return ExitCode::from(2);
        }
    };
    let input: CompatibilityInput = match serde_json::from_str(&raw) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid interface contract JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_compatibility(&input) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("interface contract rejected:");
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
            eprintln!("cannot serialize compatibility report: {error}");
            ExitCode::from(1)
        }
    }
}
