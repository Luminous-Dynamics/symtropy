use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

use joy_worlds_profile_validator::comparison::compare_scenarios_json;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let mut input_path = None;
    let mut output_path = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => input_path = args.next(),
            "--output" => output_path = args.next(),
            "--help" | "-h" => {
                println!("Usage: portfolio-compare --input SYNTHETIC_FIXTURE.json [--output DECISION_RECEIPT.json]");
                println!("The first comparator version only accepts status=synthetic_fixture.");
                println!("Its receipt is decision support; it does not certify real-world feasibility or safety.");
                return ExitCode::SUCCESS;
            }
            unknown => {
                eprintln!("Unknown argument: {unknown}");
                eprintln!("Use --help for usage.");
                return ExitCode::FAILURE;
            }
        }
    }

    let Some(input_path) = input_path else {
        eprintln!("--input PATH is required.");
        eprintln!("Use --help for usage.");
        return ExitCode::FAILURE;
    };

    let raw = match fs::read_to_string(&input_path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("Cannot read input fixture '{input_path}': {error}");
            return ExitCode::FAILURE;
        }
    };
    let receipt = match compare_scenarios_json(&raw) {
        Ok(receipt) => receipt,
        Err(errors) => {
            eprintln!("COMPARISON-INPUT-REJECTED: {} finding(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::FAILURE;
        }
    };
    let serialized = match serde_json::to_string_pretty(&receipt) {
        Ok(content) => format!("{content}\n"),
        Err(error) => {
            eprintln!("Cannot serialize receipt: {error}");
            return ExitCode::FAILURE;
        }
    };

    if let Some(path) = output_path {
        if let Err(error) = fs::write(&path, serialized) {
            eprintln!("Cannot write receipt '{path}': {error}");
            return ExitCode::FAILURE;
        }
    } else {
        let mut stdout = io::stdout().lock();
        if let Err(error) = stdout.write_all(serialized.as_bytes()) {
            eprintln!("Cannot write receipt to stdout: {error}");
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
