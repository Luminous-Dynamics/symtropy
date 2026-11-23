use orbital_gsd_kcoverage_bench::evaluate_file;
use std::{env, process};

fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!("usage: orbital-gsd-kcoverage-bench <case.json> <40-hex-source-commit>");
        process::exit(2);
    }
    let receipt = match evaluate_file(&args[1], &args[2]) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("GSD voxel visibility audit failed: {error}");
            process::exit(1);
        }
    };
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));
}
