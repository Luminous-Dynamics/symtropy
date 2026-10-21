// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_analytical_adapter::evaluate_request_bytes;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("physical-analytical-adapter: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut request_path: Option<String> = None;
    let mut adapter_revision: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--request" => {
                if request_path.is_some() {
                    return Err("--request may be supplied only once".to_owned());
                }
                request_path = Some(
                    args.get(index + 1)
                        .ok_or_else(|| "--request requires a path".to_owned())?
                        .clone(),
                );
                index += 2;
            }
            "--adapter-revision" => {
                if adapter_revision.is_some() {
                    return Err("--adapter-revision may be supplied only once".to_owned());
                }
                adapter_revision = Some(
                    args.get(index + 1)
                        .ok_or_else(|| "--adapter-revision requires a value".to_owned())?
                        .clone(),
                );
                index += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let request_path = request_path.ok_or_else(|| {
        "usage: physical-analytical-adapter --request <path> --adapter-revision <40-char-git-sha>"
            .to_owned()
    })?;
    let adapter_revision = adapter_revision.ok_or_else(|| {
        "usage: physical-analytical-adapter --request <path> --adapter-revision <40-char-git-sha>"
            .to_owned()
    })?;
    let bytes = fs::read(&request_path)
        .map_err(|error| format!("could not read request {request_path}: {error}"))?;
    let receipt = evaluate_request_bytes(&bytes, &adapter_revision)?;
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("could not serialize analytical receipt: {error}"))?
    );
    Ok(())
}
