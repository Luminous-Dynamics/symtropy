// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_solver_contract::validate_request_bytes;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("physical-solver-contract: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut request_path: Option<String> = None;
    let mut validator_revision: Option<String> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--request" => {
                if request_path.is_some() {
                    return Err("--request may be supplied only once".to_owned());
                }
                request_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--request requires a path".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            "--validator-revision" => {
                if validator_revision.is_some() {
                    return Err("--validator-revision may be supplied only once".to_owned());
                }
                validator_revision = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--validator-revision requires a value".to_owned())?
                        .clone(),
                );
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    let path = request_path.ok_or_else(|| {
        "usage: physical-solver-contract --request <path> --validator-revision <git-sha>".to_owned()
    })?;
    let revision = validator_revision.ok_or_else(|| {
        "usage: physical-solver-contract --request <path> --validator-revision <git-sha>".to_owned()
    })?;
    let bytes = fs::read(&path).map_err(|error| format!("could not read {path}: {error}"))?;
    let receipt = validate_request_bytes(&bytes, &revision)?;
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("receipt serialization failed: {error}"))?
    );
    Ok(())
}
