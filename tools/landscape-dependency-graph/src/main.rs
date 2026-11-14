// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use landscape_dependency_graph::{default_fixture_path, default_semantics_path, evaluate_fixture_bytes};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 2 {
        eprintln!("usage: landscape-dependency-graph [fixture.json] [semantics.json]");
        return ExitCode::from(2);
    }
    let fixture_path = args.first().map(PathBuf::from).unwrap_or_else(default_fixture_path);
    let semantics_path = args.get(1).map(PathBuf::from).unwrap_or_else(default_semantics_path);
    let fixture_bytes = match fs::read(&fixture_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", fixture_path.display());
            return ExitCode::FAILURE;
        }
    };
    let semantics_bytes = match fs::read(&semantics_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", semantics_path.display());
            return ExitCode::FAILURE;
        }
    };
    match evaluate_fixture_bytes(&fixture_bytes, &semantics_bytes)
        .and_then(|report| serde_json::to_vec_pretty(&report).map_err(|error| vec![error.to_string()]))
    {
        Ok(bytes) => {
            println!("{}", String::from_utf8_lossy(&bytes));
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}
