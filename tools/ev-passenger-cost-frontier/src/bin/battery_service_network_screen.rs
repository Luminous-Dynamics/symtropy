// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::battery_service_network::{
    BatteryServiceNetworkInput, evaluate_battery_service_network,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: battery_service_network_screen <battery-service-network.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read battery service network input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: BatteryServiceNetworkInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid battery service network JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_battery_service_network(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("battery service network input rejected:");
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
            eprintln!("cannot encode battery service network report: {error}");
            ExitCode::from(1)
        }
    }
}
