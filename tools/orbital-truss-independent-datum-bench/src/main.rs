use orbital_truss_independent_datum_bench::audit_corpus;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!(
            "usage: orbital-truss-independent-datum-bench <corpus.json> <40-hex-source-commit>"
        );
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
    if receipt.declared_mismatches != 0 {
        process::exit(1);
    }
}
