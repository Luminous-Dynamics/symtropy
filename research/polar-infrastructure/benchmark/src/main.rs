use std::env;
use std::error::Error;
use std::fs;
use std::io;

use polar_resilience_score::{score, ScoreInput};

fn main() {
    if let Err(error) = run() {
        eprintln!("polar-resilience-score: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = env::args().nth(1).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: polar-resilience-score <synthetic-score-input.json>",
        )
    })?;

    let raw = fs::read_to_string(&path)?;
    let input: ScoreInput = serde_json::from_str(&raw)?;
    let report = score(&input)
        .map_err(|message| io::Error::new(io::ErrorKind::InvalidData, message))?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
