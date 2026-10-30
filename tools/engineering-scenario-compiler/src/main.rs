use engineering_scenario_compiler::{compile_bytes, Disposition};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <scenario.toml>", program.to_string_lossy());
        return ExitCode::from(64);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one scenario path");
        return ExitCode::from(64);
    }
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read scenario source: {error}");
            return ExitCode::from(66);
        }
    };
    let report = compile_bytes(&bytes);
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("could not serialize compiler report: {error}");
            return ExitCode::from(70);
        }
    }
    match report.disposition {
        Disposition::PlanReady => ExitCode::SUCCESS,
        Disposition::Invalid => ExitCode::from(2),
        Disposition::Blocked => ExitCode::from(3),
        Disposition::Unsupported => ExitCode::from(4),
    }
}
