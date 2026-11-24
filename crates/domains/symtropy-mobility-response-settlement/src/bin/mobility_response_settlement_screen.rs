// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process::ExitCode};

use symtropy_mobility_response_settlement::{
    MobilityResponseSettlementInput, evaluate_mobility_response_settlement,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mobility_response_settlement_screen <four-report-bundle.json>");
        return ExitCode::from(2);
    }
    let raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read mobility response settlement bundle: {error}");
            return ExitCode::from(2);
        }
    };
    let input: MobilityResponseSettlementInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid mobility response settlement JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_mobility_response_settlement(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mobility response settlement reconciliation rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize mobility response settlement report: {error}");
            ExitCode::from(1)
        }
    }
}
