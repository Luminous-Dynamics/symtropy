// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, path::Path, process::ExitCode};

use physical_solver_contract::symmetry_handoff::verify_symmetry_handoff_artifact_bundle;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("symmetry-handoff-verify: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "usage: symmetry-handoff-verify --handoff <json> --manifest <json> --artifact-root <dir>\n\
             Verifies exact byte identity only; does not qualify physical equivalence."
        );
        return Ok(());
    }
    if args.is_empty() {
        return Err(
            "usage: symmetry-handoff-verify --handoff <json> --manifest <json> --artifact-root <dir>"
                .to_owned(),
        );
    }

    let mut handoff_path: Option<String> = None;
    let mut manifest_path: Option<String> = None;
    let mut artifact_root: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{} requires a value", args[index]))?
            .clone();
        match args[index].as_str() {
            "--handoff" => {
                if handoff_path.replace(value).is_some() {
                    return Err("--handoff may be supplied only once".to_owned());
                }
            }
            "--manifest" => {
                if manifest_path.replace(value).is_some() {
                    return Err("--manifest may be supplied only once".to_owned());
                }
            }
            "--artifact-root" => {
                if artifact_root.replace(value).is_some() {
                    return Err("--artifact-root may be supplied only once".to_owned());
                }
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 2;
    }

    let handoff_path = handoff_path.ok_or_else(|| "--handoff is required".to_owned())?;
    let manifest_path = manifest_path.ok_or_else(|| "--manifest is required".to_owned())?;
    let artifact_root = artifact_root.ok_or_else(|| "--artifact-root is required".to_owned())?;

    let handoff_bytes = fs::read(&handoff_path)
        .map_err(|error| format!("cannot read handoff '{handoff_path}': {error}"))?;
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("cannot read manifest '{manifest_path}': {error}"))?;
    let report = verify_symmetry_handoff_artifact_bundle(
        &handoff_bytes,
        &manifest_bytes,
        Path::new(&artifact_root),
    )?;
    let rendered = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize verification report: {error}"))?;
    println!("{rendered}");
    Ok(())
}
