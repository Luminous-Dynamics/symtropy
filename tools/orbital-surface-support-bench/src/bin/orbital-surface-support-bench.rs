use orbital_surface_support_bench::evaluate_bytes;
use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: orbital-surface-support-bench <gsd-case.json> <40-hex-source-commit>");
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read source GSD case: {error}");
            process::exit(2);
        }
    };
    match evaluate_bytes(&bytes, &args[2]) {
        Ok(receipt) => println!(
            "{}",
            serde_json::to_string_pretty(&receipt).expect("support receipt serializes")
        ),
        Err(error) => {
            eprintln!("surface support certification failed: {error}");
            process::exit(1);
        }
    }
}
