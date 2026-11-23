// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::sustainable_mobility_service_quality::{
    ServiceQualityInput, evaluate_service_quality,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: sustainable_mobility_service_quality_screen <requests.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read service quality input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: ServiceQualityInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid service quality JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_service_quality(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("service quality input rejected:");
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
            eprintln!("cannot encode service quality report: {error}");
            ExitCode::from(1)
        }
    }
}
