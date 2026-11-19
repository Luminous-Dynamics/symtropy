// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::shared_fleet::{
    SharedFleetInput, evaluate_shared_fleet,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <shared-fleet.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one shared-fleet input path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read shared-fleet input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: SharedFleetInput = match serde_json::from_str(&raw) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid shared-fleet input JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_shared_fleet(&input) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("shared-fleet input rejected:");
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
            eprintln!("cannot encode shared-fleet report: {error}");
            ExitCode::from(1)
        }
    }
}
