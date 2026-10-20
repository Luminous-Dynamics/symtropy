use furnishing_layout_audit::audit_fixtures;
use serde_json::Value;
use std::{env, fs, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let default_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interiors/furnishing_layout_scenarios_v0_1.json");
    let path = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or(default_path);

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("ERROR: cannot read {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };
    let document: Value = match serde_json::from_slice(&bytes) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("ERROR: invalid JSON in {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
    };

    let errors = audit_fixtures(&document);
    if errors.is_empty() {
        println!("PASS: synthetic furnishing-layout fixture audit");
        println!("Input: {}", path.display());
        println!(
            "Scope: 2D geometric oracle only; not a building-code or physical-performance model"
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("FAIL: {} furnishing-layout audit finding(s)", errors.len());
        for error in errors {
            eprintln!(" - {error}");
        }
        ExitCode::FAILURE
    }
}
