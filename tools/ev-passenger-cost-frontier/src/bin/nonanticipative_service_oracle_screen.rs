// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::nonanticipative_service_oracle::{
    NonanticipativeServiceOracleInput, evaluate_nonanticipative_service_oracle,
};
use ev_passenger_cost_frontier::sustainable_mobility_service_capacity::SustainableServiceCapacityInput;
use ev_passenger_cost_frontier::sustainable_mobility_service_quality::LinkedServiceQualityInput;

fn read_json<T: serde::de::DeserializeOwned>(path: &std::ffi::OsStr) -> Result<T, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("cannot read input {}: {error}", path.to_string_lossy()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("invalid JSON {}: {error}", path.to_string_lossy()))
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 {
        eprintln!(
            "usage: nonanticipative_service_oracle_screen <scenario-tree.json> <linked-service.json> <capacity.json>"
        );
        return ExitCode::from(2);
    }

    let tree: NonanticipativeServiceOracleInput = match read_json(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let service: LinkedServiceQualityInput = match read_json(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let capacity: SustainableServiceCapacityInput = match read_json(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    let report = match evaluate_nonanticipative_service_oracle(&tree, &service, &capacity) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("nonanticipative scenario tree rejected:");
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
            eprintln!("cannot encode nonanticipative report: {error}");
            ExitCode::from(1)
        }
    }
}
