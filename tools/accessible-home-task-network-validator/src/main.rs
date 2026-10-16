// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use accessible_home_task_network_validator::evaluate_any_bytes;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process;

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input_path: Option<PathBuf> = None;
    let mut output_path: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                let value = args.next().ok_or("--input requires a path")?;
                input_path = Some(PathBuf::from(value));
            }
            "--output" => {
                let value = args.next().ok_or("--output requires a path")?;
                output_path = Some(PathBuf::from(value));
            }
            "--help" | "-h" => {
                println!("Usage: accessible-home-task-network-validator --input <json> [--output <json>]");
                println!("Validates synthetic task-network shape, graph references, path assessments and failure-mode expectations.");
                return Ok(());
            }
            unknown => return Err(format!("unknown argument '{unknown}' (use --help)")),
        }
    }

    let input_path = input_path.ok_or("--input is required (use --help)")?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("could not read {}: {error}", input_path.display()))?;
    let report = evaluate_any_bytes(&input)
        .map_err(|errors| format!("validation failed closed:\n- {}", errors.join("\n- ")))?;
    let output = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("could not serialize receipt: {error}"))?;

    if let Some(path) = output_path {
        fs::write(&path, &output)
            .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    } else {
        io::stdout().write_all(&output)
            .map_err(|error| format!("could not write receipt to stdout: {error}"))?;
        println!();
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}
