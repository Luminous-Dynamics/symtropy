// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::fs;
use std::process::ExitCode;

use accessible_home_evidence_lifecycle_validator::evaluate_bytes;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input_path = None;
    let mut output_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => input_path = args.next(),
            "--output" => output_path = args.next(),
            "--help" | "-h" => {
                println!("Usage: accessible-home-evidence-lifecycle-validator --input FILE [--output FILE]");
                return Ok(());
            }
            unknown => return Err(format!("unknown argument {unknown:?}; use --help")),
        }
    }
    let input_path = input_path.ok_or_else(|| "missing required --input FILE".to_owned())?;
    let bytes = fs::read(&input_path).map_err(|error| format!("cannot read {input_path:?}: {error}"))?;
    let report = evaluate_bytes(&bytes).map_err(|errors| format!("validation failed closed:\n- {}", errors.join("\n- ")))?;
    let receipt = serde_json::to_string_pretty(&report).map_err(|error| format!("cannot serialize receipt: {error}"))?;
    if let Some(output_path) = output_path {
        fs::write(&output_path, format!("{receipt}\n"))
            .map_err(|error| format!("cannot write {output_path:?}: {error}"))?;
    } else {
        println!("{receipt}");
    }
    Ok(())
}
