use plasma_pressure_tensor_audit::{PressureGridDisposition, PressureGridRequest, audit_periodic_pressure_grid};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: pressure-grid-audit <pressure-grid-request.json>");
        return ExitCode::from(2);
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read pressure-grid request: {error}");
            return ExitCode::from(2);
        }
    };
    let request: PressureGridRequest = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("invalid pressure-grid request: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match audit_periodic_pressure_grid(&request) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("pressure-grid audit failed: {error}");
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            match report.body.disposition {
                PressureGridDisposition::Admissible => ExitCode::SUCCESS,
                PressureGridDisposition::Rejected => ExitCode::from(1),
            }
        }
        Err(error) => {
            eprintln!("could not serialize pressure-grid report: {error}");
            ExitCode::from(2)
        }
    }
}
