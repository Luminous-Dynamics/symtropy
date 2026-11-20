use power_restoration_run_receipt::{qualify, ReceiptEnvelope};
use std::env;
use std::fs;
use std::process;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: power-restoration-run-receipt <receipt.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    let envelope: ReceiptEnvelope = serde_json::from_str(&input).unwrap_or_else(|error| {
        eprintln!("receipt JSON decode error: {error}");
        process::exit(2);
    });
    match qualify(&envelope) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serialization")),
        Err(errors) => {
            eprintln!("run receipt rejected:");
            for error in errors { eprintln!("- {error}"); }
            process::exit(1);
        }
    }
}
