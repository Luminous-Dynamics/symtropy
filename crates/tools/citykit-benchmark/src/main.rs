use std::env;
use std::fs;
use std::process::ExitCode;

use citykit_reference_oracle::{evaluate_case, matches_expected};
use serde_json::json;
use sha2::{Digest, Sha256};

fn valid_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(corpus_path) = args.next() else {
        eprintln!("usage: citykit-reference-oracle CORPUS.json --source-commit <40-hex-sha>");
        return ExitCode::from(2);
    };
    let Some(flag) = args.next() else {
        eprintln!("missing --source-commit argument");
        return ExitCode::from(2);
    };
    let Some(source_commit) = args.next() else {
        eprintln!("missing source commit SHA");
        return ExitCode::from(2);
    };
    if flag != "--source-commit" || !valid_commit(&source_commit) {
        eprintln!("source commit must be supplied as --source-commit followed by 40 hex characters");
        return ExitCode::from(2);
    }

    let bytes = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read corpus {corpus_path}: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid CityKit corpus JSON: {error}");
            return ExitCode::from(2);
        }
    };
    let Some(cases) = corpus.get("cases").and_then(serde_json::Value::as_array) else {
        eprintln!("corpus is missing a cases array");
        return ExitCode::from(2);
    };

    let digest = Sha256::digest(&bytes);
    let corpus_sha256 = digest.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    let mut scenario_receipts = Vec::with_capacity(cases.len());
    let mut failures = 0usize;

    for case in cases {
        let actual = evaluate_case(case);
        let expected = case.get("expected").unwrap_or(&serde_json::Value::Null);
        let passed = matches_expected(&actual, expected);
        if !passed {
            failures += 1;
        }
        scenario_receipts.push(json!({
            "case_id": case.get("case_id").and_then(serde_json::Value::as_str).unwrap_or("missing-case-id"),
            "status": if passed { "pass" } else { "fail" },
            "actual": actual,
        }));
    }

    let result = if failures == 0 { "pass" } else { "fail" };
    let receipt = json!({
        "schema": "org.luminousdynamics.citykit.reference-receipt",
        "schema_version": "0.1.0",
        "result": result,
        "corpus_id": corpus.get("corpus_id").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "corpus_classification": corpus.get("classification").and_then(serde_json::Value::as_str).unwrap_or("unknown"),
        "scenario_count": cases.len(),
        "failure_count": failures,
        "corpus_sha256": corpus_sha256,
        "source_commit": source_commit,
        "claim_ceiling": "synthetic reference-oracle agreement only; not city, engineering, physical robot, economic, safety, or statutory qualification",
        "scenarios": scenario_receipts,
    });
    println!("{}", serde_json::to_string_pretty(&receipt).expect("receipt serializes"));

    if failures == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
