use std::{env, fs, process};

use haps_fleet_reliability::service_availability::{
    evaluate_service_availability, parse_service_availability_case_json,
};

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: haps-fleet-service-replay <service-availability-case.json>".to_owned())?;
    let bytes = fs::read(&path).map_err(|error| format!("could not read {path}: {error}"))?;
    let case = parse_service_availability_case_json(&bytes).map_err(|error| error.to_string())?;
    let report =
        evaluate_service_availability(&case).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("could not serialize report: {error}"))?;
    println!("{json}");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-fleet-service-replay: {error}");
        process::exit(2);
    }
}
