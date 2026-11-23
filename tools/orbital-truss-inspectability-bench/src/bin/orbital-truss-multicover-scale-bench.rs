use orbital_truss_inspectability_bench::multicover::audit_multicover;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: orbital-truss-multicover-scale-bench <multicover-corpus.json> <40-hex-source-commit>"
        );
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read multicover corpus: {error}");
            process::exit(2);
        }
    };
    let receipt = match audit_multicover(&bytes, &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("multicover audit failed: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("multicover receipt serializes")
    );
    if !receipt.selected_attainable_targets_fully_covered || !receipt.dual_bound.dual_constraints_verified {
        process::exit(1);
    }
}
