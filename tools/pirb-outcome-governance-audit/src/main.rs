// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_outcome_governance_audit::{validate_example_metric, validate_schema_document};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{env, fs, path::PathBuf, process};

const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../../docs/schemas/peacebuilding-outcome-governance-v0.2.schema.json");
const FIXTURE_BYTES: &[u8] =
    include_bytes!("../../../docs/fixtures/peacebuilding-outcome-governance-v0.2.json");

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn argument_value(args: &[String], name: &str) -> Result<String, String> {
    let index = args
        .iter()
        .position(|arg| arg == name)
        .ok_or_else(|| format!("required argument {name} is missing"))?;
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .ok_or_else(|| format!("argument {name} has no value"))
}

fn valid_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn create_receipt(args: &[String]) -> Result<(Value, bool), String> {
    let source_commit = argument_value(args, "--source-commit")?;
    let rustc_version = argument_value(args, "--rustc-version")?;
    let cargo_version = argument_value(args, "--cargo-version")?;
    if !valid_commit_sha(&source_commit) {
        return Err("--source-commit must be a lowercase 40-character Git SHA".to_owned());
    }

    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("cannot read generated dependency lock: {error}"))?;
    let schema_result = validate_schema_document();
    let validation_result = validate_example_metric();
    let schema_meta_valid = schema_result.is_ok();

    let validation_errors = match validation_result {
        Ok(()) => Vec::new(),
        Err(errors) => errors,
    };
    let example_valid = validation_errors.is_empty();
    let passed = schema_meta_valid && example_valid;

    let fixture: Value = serde_json::from_slice(FIXTURE_BYTES)
        .map_err(|error| format!("cannot parse checked-in example fixture: {error}"))?;
    let metric_id = fixture
        .get("metric_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "fixture has no metric_id".to_owned())?;
    let metric_version = fixture
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| "fixture has no positive integer version".to_owned())?;
    let governance_event_count = fixture
        .get("governance_events")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or_else(|| "fixture has no governance_events array".to_owned())?;

    let receipt = json!({
        "receipt_version": "0.1",
        "audit": "PIRB v0.2 outcome-governance contract",
        "source_commit": source_commit,
        "schema_sha256": sha256_hex(SCHEMA_BYTES),
        "fixture_sha256": sha256_hex(FIXTURE_BYTES),
        "dependency_lock_sha256": sha256_hex(&lock_bytes),
        "rustc_version": rustc_version,
        "cargo_version": cargo_version,
        "schema_meta_valid": schema_meta_valid,
        "example_valid": example_valid,
        "passed": passed,
        "metric_id": metric_id,
        "metric_version": metric_version,
        "governance_event_count": governance_event_count,
        "validation_error_count": validation_errors.len(),
        "validation_errors": validation_errors,
        "claim_ceiling": "Synthetic contract conformance only; this receipt does not prove authority legitimacy, consent, real-world measurement validity, causal impact, or deployment safety."
    });
    Ok((receipt, passed))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let (receipt, passed) = create_receipt(&args)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot encode receipt: {error}"))?
    );
    if !passed {
        return Err("PIRB v0.2 outcome-governance contract validation failed".to_owned());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_sha256_is_deterministic() {
        assert_eq!(
            sha256_hex(b"pirb receipt test"),
            sha256_hex(b"pirb receipt test")
        );
    }

    #[test]
    fn source_commit_must_be_exact_lowercase_git_sha() {
        assert!(valid_commit_sha("a".repeat(40).as_str()));
        assert!(!valid_commit_sha("A".repeat(40).as_str()));
        assert!(!valid_commit_sha("abcd"));
    }
}
