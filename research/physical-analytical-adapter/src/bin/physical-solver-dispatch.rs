// SPDX-License-Identifier: AGPL-3.0-or-later
use physical_analytical_adapter::dispatch::dispatch_request_bytes;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("physical-solver-dispatch: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut request_path: Option<String> = None;
    let mut dispatcher_revision: Option<String> = None;
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
            "--dispatcher-revision" => {
                if dispatcher_revision.is_some() {
                    return Err("--dispatcher-revision may be supplied only once".to_owned());
                }
                dispatcher_revision = Some(
                    args.get(index + 1)
                        .ok_or_else(|| "--dispatcher-revision requires a value".to_owned())?
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

    let path = request_path.ok_or_else(|| {
        "usage: physical-solver-dispatch --request <path> --dispatcher-revision <git-sha> --adapter-revision <git-sha>"
            .to_owned()
    })?;
    let dispatcher_revision = dispatcher_revision.ok_or_else(|| {
        "usage: physical-solver-dispatch --request <path> --dispatcher-revision <git-sha> --adapter-revision <git-sha>"
            .to_owned()
    })?;
    let adapter_revision = adapter_revision.ok_or_else(|| {
        "usage: physical-solver-dispatch --request <path> --dispatcher-revision <git-sha> --adapter-revision <git-sha>"
            .to_owned()
    })?;
    let bytes =
        fs::read(&path).map_err(|error| format!("could not read request {path}: {error}"))?;
    let receipt = dispatch_request_bytes(&bytes, &dispatcher_revision, &adapter_revision)?;
    println!(
        "{}",
        serde_json::to_string(&receipt)
            .map_err(|error| format!("could not serialize dispatch receipt: {error}"))?
    );
    Ok(())
}
