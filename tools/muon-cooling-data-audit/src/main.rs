// SPDX-License-Identifier: AGPL-3.0-or-later

use muon_cooling_data_audit::audit_files;
use serde_json::Value;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let mut args = env::args().skip(1);
    let mut manifest: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--manifest" => {
                manifest = Some(PathBuf::from(
                    args.next().ok_or_else(|| (64, "--manifest requires a path".to_owned()))?,
                ));
            }
            "--input" => {
                input = Some(PathBuf::from(
                    args.next().ok_or_else(|| (64, "--input requires a path".to_owned()))?,
                ));
            }
            "--help" | "-h" => {
                println!(
                    "muon-cooling-data-audit --manifest SOURCE.json --input lowemittancebeam.dat\n\
                     Audits a pinned BDSIM userfile without transforming native angles into momenta."
                );
                return Ok(());
            }
            unknown => {
                return Err((64, format!("unknown argument {unknown:?}; use --help")));
            }
        }
    }

    let manifest = manifest.ok_or_else(|| (64, "missing --manifest".to_owned()))?;
    let input = input.ok_or_else(|| (64, "missing --input".to_owned()))?;

    match audit_files(&manifest, &input) {
        Ok(report) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| (70, format!("cannot serialize report: {error}")))?
            );
            Ok(())
        }
        Err(errors) => {
            let output = Value::Array(errors.into_iter().map(Value::String).collect());
            Err((
                2,
                serde_json::to_string_pretty(&output)
                    .unwrap_or_else(|_| "native input audit rejected".to_owned()),
            ))
        }
    }
}
