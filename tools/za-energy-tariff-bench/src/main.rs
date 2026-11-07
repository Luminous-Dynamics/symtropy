use std::env;
use std::fs;
use std::process::ExitCode;

use za_energy_tariff_bench::{evaluate_fixture, Fixture};

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "fixtures/synthetic-citypower-winter-demand-ratchet.json".to_owned());

    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let fixture: Fixture = match serde_json::from_str(&input) {
        Ok(fixture) => fixture,
        Err(error) => {
            eprintln!("invalid fixture {path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let results = match evaluate_fixture(&fixture) {
        Ok(results) => results,
        Err(error) => {
            eprintln!("billing reference rejected fixture: {error}");
            return ExitCode::FAILURE;
        }
    };

    let output = serde_json::json!({
        "tool": "za-energy-tariff-bench",
        "version": env!("CARGO_PKG_VERSION"),
        "profile_id": fixture.profile_id,
        "source_note": fixture.source_note,
        "results": results
    });

    match serde_json::to_string_pretty(&output) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize evaluation: {error}");
            ExitCode::FAILURE
        }
    }
}
