use std::env;
use std::fs;
use std::process::ExitCode;

use joy_worlds_profile_validator::{
    validate_decision_contract_json, validate_portfolio_json, validate_regional_profiles_json,
};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut portfolio_path = None;
    let mut regional_path = None;
    let mut decision_contract_path = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--portfolio" => portfolio_path = args.next(),
            "--regional" => regional_path = args.next(),
            "--decision-contract" => decision_contract_path = args.next(),
            "--help" | "-h" => {
                println!(
                    "Usage: joy-worlds-profile-validator --portfolio PATH --regional PATH --decision-contract PATH"
                );
                println!("Checks structure and cross-references only; it does not qualify site feasibility, impact or safety.");
                return ExitCode::SUCCESS;
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                eprintln!("Use --help for usage.");
                return ExitCode::FAILURE;
            }
        }
    }

    let (Some(portfolio_path), Some(regional_path), Some(decision_contract_path)) =
        (portfolio_path, regional_path, decision_contract_path)
    else {
        eprintln!("--portfolio PATH, --regional PATH and --decision-contract PATH are required.");
        eprintln!("Use --help for usage.");
        return ExitCode::FAILURE;
    };

    let read = |path: &str, label: &str| match fs::read_to_string(path) {
        Ok(content) => Some(content),
        Err(error) => {
            eprintln!("Cannot read {label} manifest '{path}': {error}");
            None
        }
    };

    let Some(portfolio) = read(&portfolio_path, "portfolio") else {
        return ExitCode::FAILURE;
    };
    let Some(regional) = read(&regional_path, "regional") else {
        return ExitCode::FAILURE;
    };
    let Some(decision_contract) = read(&decision_contract_path, "decision-contract") else {
        return ExitCode::FAILURE;
    };

    let mut failures = Vec::new();
    if let Err(errors) = validate_portfolio_json(&portfolio) {
        failures.extend(errors.into_iter().map(|error| format!("PORTFOLIO: {error}")));
    }
    if let Err(errors) = validate_regional_profiles_json(&regional) {
        failures.extend(errors.into_iter().map(|error| format!("REGIONAL: {error}")));
    }
    if let Err(errors) = validate_decision_contract_json(&decision_contract) {
        failures.extend(errors.into_iter().map(|error| format!("DECISION-CONTRACT: {error}")));
    }

    if !failures.is_empty() {
        eprintln!("STRUCTURAL-VALIDATION-FAIL: {} finding(s)", failures.len());
        for failure in failures {
            eprintln!("- {failure}");
        }
        return ExitCode::FAILURE;
    }

    println!("STRUCTURAL-VALIDATION-PASS");
    println!("Portfolio, regional profiles and decision-contract structure are consistent.");
    println!("This result does not establish site feasibility, sustainability, safety, or forecast validity.");
    ExitCode::SUCCESS
}
