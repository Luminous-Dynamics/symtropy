use orbital_truss_inspectability_bench::support_frustum::audit_rectilinear_frustum_corpus;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: orbital-truss-rectilinear-frustum-bench <corpus.json> <40-hex-source-commit>"
        );
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read frustum corpus: {error}");
            process::exit(2);
        }
    };
    let receipt = match audit_rectilinear_frustum_corpus(&bytes, &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("rectilinear-frustum audit failed: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("frustum receipt serializes")
    );
    if receipt.expected_outcome_mismatches != 0 {
        process::exit(1);
    }
}
