use std::env;
use std::fs;

use haps_fleet_availability::{evaluate_case, parse_case_json};

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-fleet-availability: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: haps-fleet-availability <fleet-case.json>")?;
    if env::args().nth(2).is_some() {
        return Err("only one input path is accepted".into());
    }

    let bytes = fs::read(&path)?;
    let case = parse_case_json(&bytes)?;
    let report = evaluate_case(&case)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
