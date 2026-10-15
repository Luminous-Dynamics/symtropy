use std::env;
use std::fs;
use std::process::ExitCode;

use joy_worlds_profile_validator::{validate_portfolio_json, validate_regional_profiles_json};

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut portfolio_path = None;
    let mut regional_path = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--portfolio" => portfolio_path = args.next(),
            "--regional" => regional_path = args.next(),
            "--help" | "-h" => {
                println!(
                    "Usage: joy-worlds-profile-validator --portfolio PATH --regional PATH"
                );
                println!("Checks manifest structure and cross-references only; it does not qualify site feasibility or safety.");
                return ExitCode::SUCCESS;
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                eprintln!("Use --help for usage.");
                return ExitCode::FAILURE;
            }
        }
    }

    let (Some(portfolio_path), Some(regional_path)) = (portfolio_path, regional_path) else {
        eprintln!("Both --portfolio PATH and --regional PATH are required.");
        eprintln!("Use --help for usage.");
        return ExitCode::FAILURE;
    };

    let portfolio = match fs::read_to_string(&portfolio_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Cannot read portfolio manifest '{}': {error}", portfolio_path);
            return ExitCode::FAILURE;
        }
    };
    let regional = match fs::read_to_string(&regional_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Cannot read regional profile manifest '{}': {error}", regional_path);
            return ExitCode::FAILURE;
        }
    };

    let mut failures = Vec::new();
    if let Err(errors) = validate_portfolio_json(&portfolio) {
        failures.extend(errors.into_iter().map(|error| format!("PORTFOLIO: {error}")));
    }
    if let Err(errors) = validate_regional_profiles_json(&regional) {
        failures.extend(errors.into_iter().map(|error| format!("REGIONAL: {error}")));
    }

    if !failures.is_empty() {
        eprintln!("STRUCTURAL-VALIDATION-FAIL: {} finding(s)", failures.len());
        for failure in failures {
            eprintln!("- {failure}");
        }
        return ExitCode::FAILURE;
    }

    println!("STRUCTURAL-VALIDATION-PASS");
    println!("Portfolio and regional profile structure/cross-references are consistent.");
    println!("This result does not establish site feasibility, sustainability, safety, or forecast validity.");
    ExitCode::SUCCESS
}
