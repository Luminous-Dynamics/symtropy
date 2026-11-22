// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::mobility_commitment_lease::{
    MobilityCommitmentLeaseInput, evaluate_mobility_commitment_lease,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mobility_commitment_lease_screen <commitment-lease.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read commitment lease input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: MobilityCommitmentLeaseInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid commitment lease JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_mobility_commitment_lease(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("commitment lease input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            if report.candidate_model_admissible {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("cannot encode commitment lease report: {error}");
            ExitCode::from(1)
        }
    }
}
