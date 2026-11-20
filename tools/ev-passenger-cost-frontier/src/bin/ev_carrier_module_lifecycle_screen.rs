// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::carrier_module_lifecycle::{
    FactoryReport, LifecycleCatalog, validate_and_evaluate,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: ev_carrier_module_lifecycle_screen <factory-module-report.json> <lifecycle-catalog.json>");
        return ExitCode::from(2);
    }
    let factory_raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read factory module report: {error}");
            return ExitCode::from(2);
        }
    };
    let lifecycle_raw = match fs::read_to_string(&args[1]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read lifecycle catalog: {error}");
            return ExitCode::from(2);
        }
    };
    let factory: FactoryReport = match serde_json::from_str(&factory_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid factory module report: {error}");
            return ExitCode::from(2);
        }
    };
    let lifecycle: LifecycleCatalog = match serde_json::from_str(&lifecycle_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid lifecycle catalog: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match validate_and_evaluate(&factory, &lifecycle) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("invalid lifecycle input:");
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
            eprintln!("cannot serialize lifecycle report: {error}");
            ExitCode::from(1)
        }
    }
}
