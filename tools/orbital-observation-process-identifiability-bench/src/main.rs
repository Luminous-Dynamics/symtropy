use orbital_observation_process_identifiability_bench::{Corpus, evaluate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(fixture_path) = args.next() else {
        eprintln!(
            "usage: orbital-observation-process-identifiability-bench <corpus.json> --source-commit <40-hex>"
        );
        return ExitCode::from(2);
    };
    let Some(flag) = args.next() else {
        eprintln!("missing --source-commit");
        return ExitCode::from(2);
    };
    let Some(source_commit) = args.next() else {
        eprintln!("missing source commit");
        return ExitCode::from(2);
    };
    if flag != "--source-commit" || !valid_commit(&source_commit) || args.next().is_some() {
        eprintln!(
            "expected exactly --source-commit followed by 40 lowercase hexadecimal characters"
        );
        return ExitCode::from(2);
    }

    let raw = match fs::read(&fixture_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: Corpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match evaluate(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("observation identifiability evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };

    let receipt = json!({
        "receipt_schema": "luminous.orbital-observation-identifiability.receipt.v0.1",
        "source_commit": source_commit,
        "fixture_sha256": sha256_hex(&raw),
        "result": report.result,
        "report": report,
        "physical_reliability_claim_granted": false,
        "qualification_granted": false,
        "authority_granted": false
    });
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => {
            println!("{output}");
            if receipt["result"] == "pass" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("failed to serialize receipt: {error}");
            ExitCode::from(1)
        }
    }
}
