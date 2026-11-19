// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::carrier_module_factory::{
    FactoryReport, ModuleProcessCatalog, validate_and_evaluate,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        eprintln!("usage: ev_carrier_module_factory_screen <factory-report.json> <module-processes.json>");
        return ExitCode::from(2);
    }
    let base_raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read factory report: {error}");
            return ExitCode::from(2);
        }
    };
    let module_raw = match fs::read_to_string(&args[1]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read module process catalog: {error}");
            return ExitCode::from(2);
        }
    };
    let base: FactoryReport = match serde_json::from_str(&base_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid factory report: {error}");
            return ExitCode::from(2);
        }
    };
    let modules: ModuleProcessCatalog = match serde_json::from_str(&module_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid module process catalog: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match validate_and_evaluate(&base, &modules) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("invalid module manufacturing input:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize module factory report: {error}");
            ExitCode::from(2)
        }
    }
}
