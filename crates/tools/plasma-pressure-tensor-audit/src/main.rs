use plasma_pressure_tensor_audit::{AuditDisposition, AuditRequest, audit_pressure_tensor};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: plasma-pressure-tensor-audit <request.json>");
        return ExitCode::from(2);
    };

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read request: {error}");
            return ExitCode::from(2);
        }
    };
    let request: AuditRequest = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("invalid request: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match audit_pressure_tensor(&request) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("audit failed: {error}");
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            match report.body.disposition {
                AuditDisposition::Admissible => ExitCode::SUCCESS,
                AuditDisposition::Rejected => ExitCode::from(1),
            }
        }
        Err(error) => {
            eprintln!("could not serialize report: {error}");
            ExitCode::from(2)
        }
    }
}
