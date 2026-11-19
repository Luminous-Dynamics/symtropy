use std::env;
use std::fs;
use std::process::ExitCode;

use haps_fleet_continuity::{evaluate_case, parse_case, verify_expected};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: haps-fleet-continuity <case.json>");
        return ExitCode::from(2);
    };

    let input = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            return ExitCode::from(2);
        }
    };

    let case = match parse_case(&input) {
        Ok(case) => case,
        Err(error) => {
            eprintln!("invalid case: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match evaluate_case(&input) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("evaluation failed: {error}");
            return ExitCode::from(2);
        }
    };
    if let Err(error) = verify_expected(&report, &case.expected_oracle) {
        eprintln!("expected-oracle mismatch: {error}");
        return ExitCode::from(1);
    }

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("could not serialize report: {error}");
            ExitCode::from(2)
        }
    }
}
