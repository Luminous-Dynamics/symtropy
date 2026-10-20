// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_solver_contract::{validate_request_bytes, validate_solver_receipt_bytes};
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
    let mut receipt_path: Option<String> = None;
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
            "--receipt" => {
                if receipt_path.is_some() {
                    return Err("--receipt may be supplied only once".to_owned());
                }
                receipt_path = Some(
                    args.get(i + 1)
                        .ok_or_else(|| "--receipt requires a path".to_owned())?
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

    let request_path = request_path.ok_or_else(|| {
        "usage: physical-solver-contract --request <path> --validator-revision <git-sha> OR --request <path> --receipt <receipt-path>".to_owned()
    })?;
    let request_bytes = fs::read(&request_path)
        .map_err(|error| format!("could not read {request_path}: {error}"))?;

    let report = if let Some(receipt_path) = receipt_path {
        if validator_revision.is_some() {
            return Err("--validator-revision is only valid for request validation".to_owned());
        }
        let receipt_bytes = fs::read(&receipt_path)
            .map_err(|error| format!("could not read {receipt_path}: {error}"))?;
        validate_solver_receipt_bytes(&request_bytes, &receipt_bytes)?
    } else {
        let revision = validator_revision.ok_or_else(|| {
            "request validation requires --validator-revision <git-sha>".to_owned()
        })?;
        validate_request_bytes(&request_bytes, &revision)?
    };

    println!(
        "{}",
        serde_json::to_string(&report)
            .map_err(|error| format!("report serialization failed: {error}"))?
    );
    Ok(())
}
