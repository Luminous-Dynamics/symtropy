use plasma_pressure_tensor_audit::{
    PressureDivergenceComparisonReport, PressureGridRequest,
    compare_periodic_pressure_divergence,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn read_request(path: &std::ffi::OsStr) -> Result<PressureGridRequest, String> {
    let bytes = fs::read(path).map_err(|error| format!("could not read {}: {error}", path.to_string_lossy()))?;
    serde_json::from_slice(&bytes).map_err(|error| {
        format!("invalid pressure-grid request {}: {error}", path.to_string_lossy())
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(reference_path) = args.next() else {
        eprintln!("usage: pressure-divergence-compare <reference.json> <predicted.json>");
        return ExitCode::from(2);
    };
    let Some(predicted_path) = args.next() else {
        eprintln!("usage: pressure-divergence-compare <reference.json> <predicted.json>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("usage: pressure-divergence-compare <reference.json> <predicted.json>");
        return ExitCode::from(2);
    }

    let reference = match read_request(&reference_path) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let predicted = match read_request(&predicted_path) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(2);
        }
    };
    let report: PressureDivergenceComparisonReport =
        match compare_periodic_pressure_divergence(&reference, &predicted) {
            Ok(report) => report,
            Err(error) => {
                eprintln!("pressure-divergence comparison failed: {error}");
                return ExitCode::from(2);
            }
        };

    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            if report.body.both_fields_admissible {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("could not serialize comparison report: {error}");
            ExitCode::from(2)
        }
    }
}
