// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::nonanticipative_service_oracle::{
    NonanticipativeOracleInput, OracleStatus, evaluate_nonanticipative_oracle,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: nonanticipative_service_oracle_screen <scenario-tree.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read nonanticipative oracle input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: NonanticipativeOracleInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid nonanticipative oracle JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_nonanticipative_oracle(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("nonanticipative oracle input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            if report.status == OracleStatus::Evaluated && report.nonanticipativity_verified {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("cannot encode nonanticipative oracle report: {error}");
            ExitCode::from(1)
        }
    }
}
