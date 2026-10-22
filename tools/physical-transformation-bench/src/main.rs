use std::env;
use std::fs;
use std::process::ExitCode;
use symtropy_physical_transformation_bench::{Corpus, evaluate_corpus};

fn main() -> ExitCode {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/physical-transformation-bench/fixtures/physical_transformation_v0_1.json".to_string()
    });

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus: Corpus = match serde_json::from_slice(&bytes) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("failed to parse {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let receipt = match evaluate_corpus(&corpus) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("benchmark rejected: {error}");
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string_pretty(&receipt) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("failed to serialize receipt: {error}");
            return ExitCode::FAILURE;
        }
    }

    if receipt.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
