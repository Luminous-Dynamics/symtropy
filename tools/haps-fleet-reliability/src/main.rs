use std::{env, fs, process};

use haps_fleet_reliability::{evaluate_case, parse_case_json};

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: haps-fleet-reliability <fleet-case.json>".to_owned())?;
    let bytes = fs::read(&path).map_err(|error| format!("could not read {path}: {error}"))?;
    let case = parse_case_json(&bytes).map_err(|error| error.to_string())?;
    let report = evaluate_case(&case).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize report: {error}"))?;
    println!("{json}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-fleet-reliability: {error}");
        process::exit(2);
    }
}
