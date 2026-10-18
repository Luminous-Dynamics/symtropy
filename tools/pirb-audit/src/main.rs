use pirb_reference_audit::audit_corpus_json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| {
            format!(
                "{}/../../docs/fixtures/peace-infrastructure-resilience-v0.1.json",
                env!("CARGO_MANIFEST_DIR")
            )
        });

    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("could not read {}: {}", path, error);
            return ExitCode::FAILURE;
        }
    };

    let report = audit_corpus_json(&input);
    match serde_json::to_string_pretty(&report) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("could not serialize audit report: {}", error);
            return ExitCode::FAILURE;
        }
    }

    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
