// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::carrier_qualification::evaluate_qualification_chain_from_bytes;

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        eprintln!(
            "usage: ev_carrier_qualification_screen <carrier-engineering-report.json> <module-factory-report.json> <qualification-catalog.json>"
        );
        return ExitCode::from(2);
    }
    let mut inputs = Vec::new();
    for path in &args {
        match fs::read(path) {
            Ok(bytes) => inputs.push(bytes),
            Err(error) => {
                eprintln!("cannot read {}: {error}", path.to_string_lossy());
                return ExitCode::from(2);
            }
        }
    }
    let report = match evaluate_qualification_chain_from_bytes(
        &inputs[0],
        &inputs[1],
        &inputs[2],
    ) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("qualification chain input rejected:");
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
            eprintln!("cannot encode qualification report: {error}");
            ExitCode::from(1)
        }
    }
}
