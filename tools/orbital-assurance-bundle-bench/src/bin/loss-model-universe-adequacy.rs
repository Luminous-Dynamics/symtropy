use orbital_assurance_bundle_bench::loss_model_universe_adequacy::{Corpus, evaluate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        "usage: loss-model-universe-adequacy <cases.json> --source-commit <sha>".to_owned()
    })?;
    if args.next().as_deref() != Some("--source-commit") {
        return Err("expected --source-commit".to_owned());
    }
    let commit = args.next().ok_or_else(|| "missing source commit".to_owned())?;
    if commit.len() != 40 || !commit.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("source commit must be 40 lowercase hexadecimal characters".to_owned());
    }
    if args.next().is_some() { return Err("unexpected trailing arguments".to_owned()); }

    let bytes = fs::read(&path).map_err(|e| format!("cannot read {path}: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid corpus: {e}"))?;
    let evaluation = evaluate(&corpus)?;
    let receipt = json!({
        "receipt_schema": "luminous.orbital-loss-model-universe-adequacy.receipt.v0.1",
        "source_commit": commit,
        "fixture_sha256": digest(&bytes),
        "result": evaluation.result,
        "evaluation": evaluation
    });
    println!("{}", serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("cannot serialize receipt: {e}"))?);
    if receipt["result"] == "pass" { Ok(ExitCode::SUCCESS) } else { Ok(ExitCode::FAILURE) }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => { eprintln!("loss-model-universe-adequacy: {error}"); ExitCode::FAILURE }
    }
}
