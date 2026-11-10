use maritime_human_reentry_evaluator::evaluate_bytes;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let corpus_path = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../maritime/human-reentry-corpus-v0.1.json")
    });

    let bytes = match std::fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", corpus_path.display());
            return ExitCode::from(2);
        }
    };

    let report = match evaluate_bytes(&bytes) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("evaluation rejected input: {error}");
            return ExitCode::from(2);
        }
    };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot serialize evaluation report: {error}");
            return ExitCode::from(2);
        }
    }

    if report.all_pass {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
