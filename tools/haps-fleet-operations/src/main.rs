use std::env;
use std::fs;
use std::process::ExitCode;

use haps_fleet_operations::{evaluate_case, parse_case_json, verify_expected_oracle};

fn main() -> ExitCode {
    let path = match env::args_os().nth(1) {
        Some(path) => path,
        None => {
            eprintln!("usage: haps-fleet-operations <case.json>");
            return ExitCode::from(2);
        }
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {}: {error}", path.to_string_lossy());
            return ExitCode::from(2);
        }
    };
    let case = match parse_case_json(&bytes) {
        Ok(case) => case,
        Err(error) => {
            eprintln!("invalid case: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_case(&case) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };
    if let Err(error) = verify_expected_oracle(&report, &case.expected_oracle) {
        eprintln!("independent oracle mismatch: {error}");
        return ExitCode::from(1);
    }
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to serialize report: {error}");
            ExitCode::from(1)
        }
    }
}
