use orbital_feature_measurability_bench::audit_corpus;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: orbital-feature-measurability-bench <feature-corpus.json> <40-hex-source-commit>"
        );
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read feature corpus: {error}");
            process::exit(2);
        }
    };
    let receipt = match audit_corpus(&bytes, &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("feature measurability audit failed: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("feature receipt serializes")
    );
    if receipt.expected_outcome_mismatches != 0 {
        process::exit(1);
    }
}
