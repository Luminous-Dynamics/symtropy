use orbital_interface_lifecycle_bench::event_replay::{evaluate, sha256_hex, Corpus};
use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-interface-event-replay <cases.json> --source-commit <40-hex-sha>"
            .to_owned()
    })?;
    let flag = args.next().ok_or_else(|| "missing --source-commit".to_owned())?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or_else(|| "missing source commit value".to_owned())?;
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }

    let input = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read event corpus {fixture_path}: {error}"))?;
    let corpus: Corpus = serde_json::from_slice(&input)
        .map_err(|error| format!("invalid event replay corpus JSON: {error}"))?;
    let report = evaluate(&corpus)?;
    let receipt = json!({
        "receipt_schema": "luminous.orbital-interface-event-replay.receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": source_commit,
        "corpus_sha256": sha256_hex(&input),
        "result": report.result,
        "report": report,
        "authority_granted": false
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize event replay receipt: {error}"))?
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
            eprintln!("orbital-interface-event-replay: {error}");
            ExitCode::FAILURE
        }
    }
}
