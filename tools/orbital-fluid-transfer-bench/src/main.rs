use std::env;
use std::fs;
use std::process::ExitCode;

use orbital_fluid_transfer_bench::{evaluate, sha256_hex, valid_source_commit, Fixture};

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-fluid-transfer-bench <fixture.json> --source-commit <40-hex-sha>".to_owned()
    })?;
    let flag = args.next().ok_or_else(|| "missing --source-commit argument".to_owned())?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or_else(|| "missing source commit value".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    if !valid_source_commit(&source_commit) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }

    let fixture_bytes = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read fixture {fixture_path}: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&fixture_bytes)
        .map_err(|error| format!("invalid fixture JSON: {error}"))?;
    let report = evaluate(&fixture)?;
    let receipt = serde_json::json!({
        "receipt_schema": "luminous.orbital-fluid-transfer.receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": source_commit,
        "fixture_sha256": sha256_hex(&fixture_bytes),
        "result": "pass",
        "report": report
    });
    let rendered = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("cannot serialize receipt: {error}"))?;
    println!("{rendered}");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orbital-fluid-transfer-bench: {error}");
            ExitCode::FAILURE
        }
    }
}
