use orbital_admitted_observation_bench::audit_corpus;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: orbital-admitted-observation-bench <corpus.json> <40-hex-source-commit>");
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus: {error}");
            process::exit(2);
        }
    };
    let receipt = match audit_corpus(&bytes, &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("audit failed: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serializes")
    );
    if receipt.expected_disposition_mismatches != 0 {
        eprintln!(
            "expected disposition mismatches: {}",
            receipt.expected_disposition_mismatches
        );
        for case in &receipt.cases {
            eprintln!("case={} disposition={:?}", case.case_id, case.disposition);
        }
        process::exit(1);
    }
}
