use plasma_stream_telemetry::{ReplayRequest, replay_trace};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: plasma-stream-telemetry <trace.json>");
        return ExitCode::from(2);
    };

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read trace: {error}");
            return ExitCode::from(2);
        }
    };
    let request: ReplayRequest = match serde_json::from_slice(&bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("invalid trace: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match replay_trace(&request) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("replay failed: {error}");
            return ExitCode::from(2);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("could not serialize report: {error}");
            ExitCode::from(2)
        }
    }
}
