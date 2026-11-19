use std::{env, fs, process};

use haps_ops_evidence::validate_ledger_json;

fn usage() -> &'static str {
    "Usage:
  haps-ops-evidence <ops-evidence-ledger.json>

Validates HAPS event order, lifecycle transitions, configuration references,
time/censoring semantics and the external return-to-service authority boundary.
It does not validate maintenance quality, cryptographic signatures or airworthiness."
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("{}", usage());
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("{}", usage());
        process::exit(2);
    }

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            process::exit(1);
        }
    };
    match validate_ledger_json(&bytes) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("failed to serialize report: {error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("operations evidence validation failed: {error}");
            process::exit(1);
        }
    }
}
