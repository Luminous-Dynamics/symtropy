use std::env;
use std::fs;
use std::process::ExitCode;

use symtropy_world_experiment_lease::{LeaseInputV1, compile_lease};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or("usage: world-experiment-lease <input.json>")?;
    if args.next().is_some() {
        return Err("expected exactly one input path".into());
    }

    let bytes = fs::read(&path)?;
    let input: LeaseInputV1 = serde_json::from_slice(&bytes)?;
    let receipt = compile_lease(input)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("world-experiment-lease: {error}");
            ExitCode::FAILURE
        }
    }
}
