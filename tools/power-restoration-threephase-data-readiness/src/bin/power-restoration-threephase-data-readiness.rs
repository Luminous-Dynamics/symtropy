use power_restoration_threephase_data_readiness::{ThreePhaseReadinessReport, parse_and_audit};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn read(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn run() -> Result<ThreePhaseReadinessReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 2 {
        return Err(format!(
            "Usage: {} <threephase-input-package.json> <benchmark-case.json>",
            program.to_string_lossy()
        ));
    }
    let paths: Vec<PathBuf> = values.into_iter().map(PathBuf::from).collect();
    let package = read(&paths[0])?;
    let case = read(&paths[1])?;
    parse_and_audit(&package, &case).map_err(|errors| errors.join("\n"))
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if report.blocker_count == 0 {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
            Err(error) => {
                eprintln!("three-phase readiness report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("three-phase input readiness audit failed:\n{error}");
            ExitCode::from(1)
        }
    }
}
