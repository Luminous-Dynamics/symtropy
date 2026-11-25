// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process::ExitCode};

use symtropy_mobility_policy_tournament::{
    MobilityPolicyTournamentInput, evaluate_mobility_policy_tournament,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mobility_policy_tournament <matched-policy-runs.json>");
        return ExitCode::from(2);
    }
    let raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read policy tournament input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: MobilityPolicyTournamentInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid mobility policy tournament JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_mobility_policy_tournament(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mobility policy tournament rejected input: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize mobility policy tournament report: {error}");
            ExitCode::from(1)
        }
    }
}
