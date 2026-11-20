// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::carrier_lifecycle::{
    LifecycleCatalog, ModuleFactoryReport, evaluate_lifecycle,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: ev_carrier_lifecycle_screen <module-factory-report.json> <lifecycle-catalog.json>");
        return ExitCode::from(2);
    }

    let factory_raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read module factory report: {error}");
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
    let factory: ModuleFactoryReport = match serde_json::from_str(&factory_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid module factory report JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let catalog: LifecycleCatalog = match serde_json::from_str(&lifecycle_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid lifecycle catalog JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_lifecycle(&factory, &catalog) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("lifecycle input rejected:");
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
            eprintln!("cannot encode lifecycle report: {error}");
            ExitCode::from(1)
        }
    }
}
