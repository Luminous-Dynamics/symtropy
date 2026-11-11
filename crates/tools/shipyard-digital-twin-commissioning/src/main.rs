use shipyard_digital_twin_commissioning::{commission_twin, CommissioningDecision, CommissioningInput};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/shipyard/digital-twin/commissioning-valid-v0.1.json".to_owned());
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let input: CommissioningInput = match serde_json::from_str(&content) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid commissioning input {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = commission_twin(&input);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("could not serialize commissioning report: {error}");
            return ExitCode::FAILURE;
        }
    }
    if report.decision == CommissioningDecision::Ready {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
