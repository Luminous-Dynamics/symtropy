use std::{env, fs, process};

use haps_fleet_availability::{evaluate_case, parse_case_json};

const DEFAULT_CASE: &str = include_str!(
    "../../../docs/research/aviation/examples/synthetic_haps_fleet_availability_case_v1.json"
);

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let bytes = if let Some(path) = args.first() {
        fs::read(path).map_err(|error| format!("could not read {path}: {error}"))?
    } else {
        DEFAULT_CASE.as_bytes().to_vec()
    };
    let case = parse_case_json(&bytes).map_err(|error| error.to_string())?;
    let report = evaluate_case(&case).map_err(|error| error.to_string())?;
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize result: {error}"))?;
    println!("{output}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-fleet-availability: {error}");
        process::exit(2);
    }
}
