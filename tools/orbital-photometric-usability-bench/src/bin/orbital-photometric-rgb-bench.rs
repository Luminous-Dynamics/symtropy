use orbital_photometric_usability_bench::rgb::audit_rgb_corpus;
use std::{env,fs,process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: orbital-photometric-rgb-bench <corpus.json> <40-hex-source-commit>");
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(value) => value,
        Err(error) => { eprintln!("cannot read RGB corpus: {error}"); process::exit(2); }
    };
    let receipt = match audit_rgb_corpus(&bytes, &args[2]) {
        Ok(value) => value,
        Err(error) => { eprintln!("RGB audit failed: {error}"); process::exit(1); }
    };
    println!("{}",serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
    if receipt.expected_mismatches != 0 { process::exit(1); }
}
