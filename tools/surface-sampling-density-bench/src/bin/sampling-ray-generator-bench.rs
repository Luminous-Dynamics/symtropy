use std::{env, fs, process};
use surface_sampling_density_bench::ray_geometry::audit_geometry_corpus;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: sampling-ray-generator-bench <corpus.json> <40-hex-source-commit>");
        process::exit(2);
    }
    let bytes = match fs::read(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read ray-geometry corpus: {error}");
            process::exit(2);
        }
    };
    let receipt = match audit_geometry_corpus(&bytes, &args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("ray-geometry audit failed: {error}");
            process::exit(1);
        }
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("ray-geometry receipt serializes")
    );
    if receipt.expected_outcome_mismatches != 0 {
        process::exit(1);
    }
}
