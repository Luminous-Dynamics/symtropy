// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::battery_service_network::{
    ExclusiveResourceScheduleInput, validate_exclusive_resource_schedule,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: battery_resource_occupancy_screen <resource-occupancy.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read resource occupancy input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: ExclusiveResourceScheduleInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid resource occupancy JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match validate_exclusive_resource_schedule(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("exclusive resource schedule rejected:");
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
            eprintln!("cannot encode resource occupancy report: {error}");
            ExitCode::from(1)
        }
    }
}
