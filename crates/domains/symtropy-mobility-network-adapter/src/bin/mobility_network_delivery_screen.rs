// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process::ExitCode};

use symtropy_mobility_network_adapter::{
    MobilityNetworkAdapterInput, evaluate_mobility_network_adapter,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: mobility_network_delivery_screen <adapter-input.json>");
        return ExitCode::from(2);
    }
    let raw = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read mobility network adapter input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: MobilityNetworkAdapterInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid mobility network adapter JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_mobility_network_adapter(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("mobility network adapter rejected input: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode mobility network adapter report: {error}");
            ExitCode::from(1)
        }
    }
}
