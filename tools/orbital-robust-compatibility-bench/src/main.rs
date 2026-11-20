use orbital_robust_compatibility_bench::{evaluate, sha256_hex, RobustFixture};
use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        "usage: orbital-robust-compatibility-bench <cases.json> --source-commit <40-hex-sha>".to_owned()
    })?;
    if args.next().as_deref() != Some("--source-commit") {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or_else(|| "missing source commit".to_owned())?;
    if source_commit.len() != 40 || !source_commit.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("source commit must be 40 hexadecimal characters".to_owned());
    }
    if args.next().is_some() { return Err("unexpected trailing arguments".to_owned()); }

    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let fixture: RobustFixture = serde_json::from_slice(&bytes).map_err(|e| format!("invalid fixture: {e}"))?;
    let report = evaluate(&fixture)?;
    let result = report.result.clone();
    let receipt = json!({
        "receipt_schema": "luminous.orbital-robust-compatibility.receipt.v0.1",
        "source_commit": source_commit,
        "fixture_sha256": sha256_hex(&bytes),
        "result": result,
        "authority_granted": false,
        "report": report
    });
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| format!("cannot encode receipt: {e}"))?);
    if receipt["result"] == "pass" { Ok(ExitCode::SUCCESS) } else { Ok(ExitCode::FAILURE) }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("orbital-robust-compatibility-bench: {error}");
            ExitCode::FAILURE
        }
    }
}
