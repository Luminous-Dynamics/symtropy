//! Evaluate one finite sequence-level degradation/recovery episode.
//!
//! Usage:
//!   recoverability-envelope <episode.json> <report.json>
//!
//! Exit codes: 0 = structurally valid; 1 = rejected input/trace; 2 = file or JSON error.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use recoverability_envelope::RecoveryEpisode;
use serde_json::{Value, json};

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn write_report(path: &Path, value: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: recoverability-envelope <episode.json> <report.json>".into());
    }

    let input_path = Path::new(&args[0]);
    let report_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("episode input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let episode = RecoveryEpisode::from_json(&input)
        .map_err(|error| format!("typed episode input rejected: {error}"))?;

    match episode.evaluate() {
        Ok(report) => {
            let value = serde_json::to_value(report)?;
            write_report(report_path, &value)?;
            println!(
                "recoverability episode valid: {} faults; {} recovered; {} contract-met; report {}",
                value["fault_event_count"].as_u64().unwrap_or(0),
                value["recovered_fault_count"].as_u64().unwrap_or(0),
                value["contract_met_fault_count"].as_u64().unwrap_or(0),
                report_path.display()
            );
            Ok(0)
        }
        Err(errors) => {
            let value = json!({
                "schema_version": 1,
                "status": "rejected",
                "run_id": episode.run_id,
                "expected_ticks": episode.expected_ticks,
                "observed_ticks": episode.trace.len(),
                "fault_event_count": episode.fault_events.len(),
                "errors": errors,
                "claim_ceiling": "Rejected structural evidence; no recovery or safety claim is made."
            });
            write_report(report_path, &value)?;
            eprintln!(
                "recoverability episode rejected with {} finding(s); report {}",
                value["errors"].as_array().map_or(0, Vec::len),
                report_path.display()
            );
            Ok(1)
        }
    }
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("recoverability-envelope: {error}");
            std::process::exit(2);
        }
    }
}
