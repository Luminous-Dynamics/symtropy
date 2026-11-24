use orbital_truss_inspectability_bench::planning::audit_planning_corpus;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: orbital-truss-inspection-planning-bench <planning-corpus.json> <40-hex-source-commit>"
        );
        process::exit(2);
    }

    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read planning corpus: {error}");
            process::exit(2);
        }
    };

    let receipt = match audit_planning_corpus(&bytes, &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("planning corpus audit failed: {error}");
            process::exit(1);
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("planning receipt serializes")
    );
    if receipt.expected_outcome_mismatches != 0 {
        process::exit(1);
    }
}
