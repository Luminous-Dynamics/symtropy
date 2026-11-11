use std::env;
use std::fs;
use std::process::ExitCode;

use shipyard_weld_scale_transfer::validate_corpus;

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/shipyard/scale-transfer/corpus-v0.1.json".to_owned());
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match validate_corpus(&bytes) {
        Ok(summary) => {
            println!(
                "PASS {}: {} cases ({} positive, {} negative), fnv1a64={}",
                summary.benchmark_id,
                summary.cases,
                summary.positive,
                summary.negative,
                summary.corpus_fingerprint_fnv1a64
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("scale-transfer corpus rejected: {error}");
            ExitCode::FAILURE
        }
    }
}
