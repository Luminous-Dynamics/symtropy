// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::carrier_module_factory::{
    FactoryReport, ModuleProcessCatalog, SharedCellCatalog, evaluate_shared_cells,
};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        eprintln!(
            "usage: ev_carrier_shared_cell_screen <factory-report.json> <module-processes.json> <shared-cell-mappings.json>"
        );
        return ExitCode::from(2);
    }

    let read_input = |path: &str, label: &str| -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {label} {path}: {error}"))
    };

    let base_raw = match read_input(&args[0], "factory report") {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let module_raw = match read_input(&args[1], "module process catalog") {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let mapping_raw = match read_input(&args[2], "shared-cell mapping catalog") {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{error}");
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
    let mappings: SharedCellCatalog = match serde_json::from_str(&mapping_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid shared-cell mapping catalog: {error}");
            return ExitCode::from(2);
        }
    };

    let report = match evaluate_shared_cells(&base, &modules, &mappings) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("shared-cell mapping input rejected:");
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
            eprintln!("cannot serialize shared-cell report: {error}");
            ExitCode::from(1)
        }
    }
}
