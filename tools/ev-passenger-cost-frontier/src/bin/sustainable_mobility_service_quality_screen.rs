// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::sustainable_mobility_service_capacity::SustainableServiceCapacityInput;
use ev_passenger_cost_frontier::sustainable_mobility_service_quality::{
    LinkedServiceQualityInput, evaluate_linked_service_quality,
};

fn read_json(path: &std::ffi::OsStr) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read input: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 2 {
        eprintln!(
            "usage: sustainable_mobility_service_quality_screen <linked-requests.json> <capacity-input.json>"
        );
        return ExitCode::from(2);
    }

    let request_raw = match read_json(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let capacity_raw = match read_json(&args[1]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };

    let requests: LinkedServiceQualityInput = match serde_json::from_str(&request_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid linked service quality JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let capacity: SustainableServiceCapacityInput = match serde_json::from_str(&capacity_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid capacity source JSON: {error}");
            return ExitCode::from(1);
        }
    };

    let report = match evaluate_linked_service_quality(&requests, &capacity) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("linked service quality input rejected:");
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
            eprintln!("cannot encode linked service quality report: {error}");
            ExitCode::from(1)
        }
    }
}
