use orbital_interface_lifecycle_bench::{Corpus, evaluate, is_valid_source_commit, sha256_hex};
use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-interface-lifecycle-bench <cases.json> --source-commit <40-hex-sha>"
            .to_owned()
    })?;
    let flag = args
        .next()
        .ok_or_else(|| "missing --source-commit".to_owned())?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args
        .next()
        .ok_or_else(|| "missing source commit value".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    if !is_valid_source_commit(&source_commit) {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }

    let input = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read fixture {fixture_path}: {error}"))?;
    let corpus: Corpus = serde_json::from_slice(&input)
        .map_err(|error| format!("invalid lifecycle corpus JSON: {error}"))?;
    let report = evaluate(&corpus)?;
    let result = report.result.clone();
    let receipt = json!({
        "receipt_schema": "luminous.orbital-interface-lifecycle.receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": source_commit,
        "fixture_sha256": sha256_hex(&input),
        "result": result,
        "report": report,
        "authority_granted": false
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize lifecycle receipt: {error}"))?
    );

    if receipt["result"] == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("orbital-interface-lifecycle-bench: {error}");
            ExitCode::FAILURE
        }
    }
}
