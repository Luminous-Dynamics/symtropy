// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::battery_charge_queue::{
    BatteryChargeQueueInput, evaluate_battery_charge_queue,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: battery_charge_queue_screen <battery-charge-queue.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read battery charge queue input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: BatteryChargeQueueInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid battery charge queue JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_battery_charge_queue(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("battery charge queue input rejected:");
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
            eprintln!("cannot encode battery charge queue report: {error}");
            ExitCode::from(1)
        }
    }
}
