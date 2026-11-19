// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_drone_operations_baseline::EvidenceLedger;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let path = match args.as_slice() {
        [path] => path,
        [] => {
            eprintln!("usage: ev-drone-operations-baseline <evidence-ledger.json>");
            return ExitCode::from(2);
        }
        _ => {
            eprintln!("expected exactly one evidence-ledger path");
            return ExitCode::from(2);
        }
    };
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };
    let ledger: EvidenceLedger = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid evidence-ledger JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match ledger.summary() {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("evidence-ledger validation failed:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize evidence-ledger summary: {error}");
            ExitCode::from(1)
        }
    }
}
