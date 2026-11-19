use std::{env, fs, process};

use haps_fleet_service::{evaluate_case, parse_case_json};

fn usage() -> &'static str {
    "Usage:
  haps-fleet-service <fleet-service-case.json>

Replays a deterministic HAPS fleet event trace and verifies its expected oracle.
This tool computes service-availability arithmetic only; it is not a stochastic
reliability model or an aircraft/network/thermal physics simulator."
}

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("{}", usage());
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("{}", usage());
        process::exit(2);
    }

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            process::exit(1);
        }
    };
    let case = match parse_case_json(&bytes) {
        Ok(case) => case,
        Err(error) => {
            eprintln!("invalid fleet service case: {error}");
            process::exit(1);
        }
    };
    match evaluate_case(&case) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("failed to serialize report: {error}");
                process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("fleet service replay failed: {error}");
            process::exit(1);
        }
    }
}
