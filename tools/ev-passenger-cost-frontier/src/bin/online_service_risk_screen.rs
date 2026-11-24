// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::online_service_risk::{
    OnlineServiceRiskInput, evaluate_online_service_risk,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: online_service_risk_screen <online-service-risk.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read online service risk input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: OnlineServiceRiskInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid online service risk JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_online_service_risk(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("online service risk input rejected:");
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
            eprintln!("cannot encode online service risk report: {error}");
            ExitCode::from(1)
        }
    }
}
