use orbital_assurance_bundle_bench::coalition_consequence_binding::{evaluate, Corpus};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, process::ExitCode};

fn run() -> Result<ExitCode, String> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 || args[2] != "--source-commit" {
        return Err("usage: coalition-consequence-binding <cases.json> --source-commit <40-hex-sha>".into());
    }
    if args[3].len() != 40 || !args[3].bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()) {
        return Err("source commit must be 40 lowercase hexadecimal characters".into());
    }
    let bytes = fs::read(&args[1]).map_err(|e| format!("cannot read fixture: {e}"))?;
    let corpus: Corpus = serde_json::from_slice(&bytes).map_err(|e| format!("invalid fixture: {e}"))?;
    let evaluation = evaluate(&corpus)?;
    let fixture_sha256 = Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect::<String>();
    let result = evaluation.result.clone();
    let receipt = json!({
        "receipt_schema": "luminous.orbital-coalition-consequence-binding.receipt.v0.1",
        "source_commit": args[3],
        "fixture_sha256": fixture_sha256,
        "result": result,
        "evaluation": evaluation,
        "authority_granted": false
    });
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| format!("serialize receipt: {e}"))?);
    Ok(if result == "pass" { ExitCode::SUCCESS } else { ExitCode::FAILURE })
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("coalition-consequence-binding: {error}");
            ExitCode::FAILURE
        }
    }
}
