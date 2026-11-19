use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

use reusable_launch_production_bench::{evaluate, is_valid_source_commit, sha256_hex, Fixture};

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: reusable-launch-production-bench <fixture.json> --source-commit <40-hex-sha>"
            .to_owned()
    })?;
    let flag = args.next().ok_or_else(|| {
        "missing --source-commit argument".to_owned()
    })?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or_else(|| {
        "missing source commit value".to_owned()
    })?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    if !is_valid_source_commit(&source_commit) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }

    let fixture_bytes = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read fixture {fixture_path}: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&fixture_bytes)
        .map_err(|error| format!("invalid fixture JSON: {error}"))?;
    let report = evaluate(&fixture)?;
    let receipt = json!({
        "receipt_schema": "luminous.reusable-launch-production.receipt.v0.1",
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
            eprintln!("reusable-launch-production-bench: {error}");
            ExitCode::FAILURE
        }
    }
}
