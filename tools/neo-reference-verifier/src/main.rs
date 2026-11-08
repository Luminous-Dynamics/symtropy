use neo_reference_verifier::validate_case;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, process};

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/space-engineering/reference-cases/dart-dimorphos-v0.1.json".to_owned());
    let bytes = fs::read(&path).map_err(|err| format!("cannot read {path}: {err}"))?;
    let case: Value = serde_json::from_slice(&bytes)
        .map_err(|err| format!("invalid JSON in {path}: {err}"))?;
    validate_case(&case)?;

    let digest = format!("{:x}", Sha256::digest(&bytes));
    let receipt = json!({
        "receipt_version": "0.1.0",
        "case_id": case["case_id"],
        "input_sha256": digest,
        "validator": "neo-reference-verifier",
        "validation_scope": "data_contract_and_frozen_reference_semantics_only",
        "status": "PASS_DATA_CONTRACT_ONLY",
        "claim_ceiling": case["claim_ceiling"]
    });
    println!("{}", serde_json::to_string(&receipt).map_err(|err| err.to_string())?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("REJECTED: {error}");
        process::exit(2);
    }
}
