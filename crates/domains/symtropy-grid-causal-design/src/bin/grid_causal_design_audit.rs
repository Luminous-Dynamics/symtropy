// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process::ExitCode};

use symtropy_grid_causal_design::{GridCausalDesignInput, audit_causal_design};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: grid_causal_design_audit <preregistered-plan.json>");
        return ExitCode::from(2);
    }
    let raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read causal design plan: {error}");
            return ExitCode::from(2);
        }
    };
    let input: GridCausalDesignInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid causal design JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match audit_causal_design(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("causal design plan rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize causal design report: {error}");
            ExitCode::from(1)
        }
    }
}
