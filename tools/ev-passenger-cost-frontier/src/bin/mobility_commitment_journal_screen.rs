// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::mobility_commitment_journal::{
    JournalScenario, evaluate_journal_scenario,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mobility_commitment_journal_screen <journal-scenario.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read journal scenario: {error}");
            return ExitCode::from(2);
        }
    };
    let scenario: JournalScenario = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid commitment journal JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_journal_scenario(&scenario) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("commitment journal scenario rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode commitment journal report: {error}");
            ExitCode::from(1)
        }
    }
}
