use shipyard_geometry_measurement_oracle::{Corpus, verify_corpus};
use std::env;
use std::fs;
use std::process::ExitCode;

const DEFAULT_CORPUS: &str = "benchmarks/shipyard/geometry-measurement-oracle/corpus-v0.1.json";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let corpus_path = args.next().unwrap_or_else(|| DEFAULT_CORPUS.to_owned());
    if args.next().is_some() {
        eprintln!("usage: shipyard-geometry-measurement-oracle [CORPUS_PATH]");
        return ExitCode::from(2);
    }
    let corpus_text = match fs::read_to_string(&corpus_path) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("cannot read corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus: Corpus = match serde_json::from_str(&corpus_text) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("cannot parse corpus {corpus_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match verify_corpus(&corpus) {
        Ok(summary) => {
            println!(
                "PASS: {cases} synthetic geometry cases ({qualified} qualified / {blocked} blocked); 2D profile comparison only",
                cases = summary.cases,
                qualified = summary.qualified,
                blocked = summary.blocked
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!(
                "FAIL: {} geometry-oracle discrepancy/discrepancies",
                errors.len()
            );
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}
