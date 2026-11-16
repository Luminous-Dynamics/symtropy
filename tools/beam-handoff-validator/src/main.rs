use beam_handoff_validator::validate_files;
use serde_json::json;
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
    let mut args = std::env::args().skip(1);
    let mut schema: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--schema" => {
                schema = Some(PathBuf::from(
                    args.next().ok_or_else(|| (64, "--schema requires a path".to_owned()))?,
                ));
            }
            "--input" => {
                input = Some(PathBuf::from(
                    args.next().ok_or_else(|| (64, "--input requires a path".to_owned()))?,
                ));
            }
            "--help" | "-h" => {
                println!(
                    "beam-handoff-validator --schema SCHEMA.json --input HANDOFF.json\n\
                     Validates schema, population accounting, asset integrity and the supported CSV adapter."
                );
                return Ok(());
            }
            unknown => {
                return Err((
                    64,
                    format!("unknown argument {unknown:?}; use --help for usage"),
                ));
            }
        }
    }

    let schema = schema.ok_or_else(|| (64, "missing --schema; use --help".to_owned()))?;
    let input = input.ok_or_else(|| (64, "missing --input; use --help".to_owned()))?;

    match validate_files(&schema, &input) {
        Ok(report) => {
            let output = json!({
                "validator": "beam-handoff-validator",
                "validator_version": env!("CARGO_PKG_VERSION"),
                "status": "passed",
                "handoff_id": report.handoff_id,
                "stage": report.stage,
                "schema_version": report.schema_version,
                "checks": report.checks,
                "asset_sha256": report.asset_sha256,
                "macro_particle_count": report.macro_particle_count
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&output)
                    .map_err(|error| (70, format!("cannot serialize report: {error}")))?
            );
            Ok(())
        }
        Err(errors) => {
            let output = json!({
                "validator": "beam-handoff-validator",
                "validator_version": env!("CARGO_PKG_VERSION"),
                "status": "rejected",
                "input": input.display().to_string(),
                "errors": errors
            });
            Err((
                2,
                serde_json::to_string_pretty(&output)
                    .unwrap_or_else(|_| "validation failed; report serialization failed".to_owned()),
            ))
        }
    }
}
