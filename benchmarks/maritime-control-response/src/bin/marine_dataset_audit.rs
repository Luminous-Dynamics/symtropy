#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use liu_ice_control_response_evaluator::csv_audit::audit_marine_dataset_directory;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err("usage: marine-dataset-audit <extracted-dataset-root> <archive-sha256>".into());
    }
    let root = PathBuf::from(&arguments[0]);
    let archive_sha256 = arguments[1]
        .to_str()
        .ok_or_else(|| "archive SHA-256 argument is not valid UTF-8".to_owned())?;

    let report = audit_marine_dataset_directory(&root, archive_sha256)
        .map_err(|error| format!("marine dataset audit rejected: {error}"))?;
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode audit report: {error}"))?;
    println!("{json}");
    Ok(())
}
