// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_reference_audit::{audit_corpus_json, validate_corpus_schema};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

const UNPINNED_LOCAL_RUN: &str = "UNPINNED_LOCAL_RUN";

fn main() -> ExitCode {
    let mut path: Option<String> = None;
    let mut source_commit = UNPINNED_LOCAL_RUN.to_owned();
    let mut args = env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--source-commit" => {
                let Some(value) = args.next() else {
                    eprintln!("--source-commit requires a full 40-character commit SHA");
                    return ExitCode::FAILURE;
                };
                source_commit = value;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: pirb-reference-audit [CORPUS.json] [--source-commit FULL_SHA]\n\
                     A local run without --source-commit is explicitly marked unpinned."
                );
                return ExitCode::SUCCESS;
            }
            value if value.starts_with('-') => {
                eprintln!("unknown option: {value}");
                return ExitCode::FAILURE;
            }
            value => {
                if path.replace(value.to_owned()).is_some() {
                    eprintln!("only one corpus path may be supplied");
                    return ExitCode::FAILURE;
                }
            }
        }
    }

    if source_commit != UNPINNED_LOCAL_RUN
        && (source_commit.len() != 40
            || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        eprintln!("source commit must be a full 40-character hexadecimal SHA");
        return ExitCode::FAILURE;
    }

    let path = path.unwrap_or_else(|| {
        format!(
            "{}/../../docs/fixtures/peace-infrastructure-resilience-v0.1.json",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let input = match fs::read_to_string(&path) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("could not read {}: {}", path, error);
            return ExitCode::FAILURE;
        }
    };

    let schema_path = format!(
        "{}/../../docs/schemas/peace-infrastructure-resilience-v0.1.schema.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let schema_input = match fs::read_to_string(&schema_path) {
        Ok(schema) => schema,
        Err(error) => {
            eprintln!("could not read schema {}: {}", schema_path, error);
            return ExitCode::FAILURE;
        }
    };
    if let Err(errors) = validate_corpus_schema(&schema_input, &input) {
        eprintln!("PIRB schema validation failed; refusing semantic audit:");
        for error in errors {
            eprintln!("- {error}");
        }
        return ExitCode::FAILURE;
    }

    let schema_sha256 = format!("{:x}", Sha256::digest(schema_input.as_bytes()));
    let corpus_sha256 = format!("{:x}", Sha256::digest(input.as_bytes()));
    let audit = audit_corpus_json(&input);
    let parsed = serde_json::from_str::<Value>(&input).ok();
    let scenario_values = parsed
        .as_ref()
        .and_then(|value| value.get("scenarios"))
        .and_then(Value::as_array);
    let scenario_receipts: Vec<Value> = scenario_values
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(index, scenario)| {
            let scenario_id = scenario
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN_ID");
            let prefix = format!("$.scenarios[{index}]");
            let failed = audit
                .findings
                .iter()
                .any(|finding| finding.path.starts_with(&prefix));
            json!({
                "scenario_id": scenario_id,
                "status": if failed { "fail" } else { "pass" }
            })
        })
        .collect();

    let passed = audit.passed;
    let scenario_count = audit.scenario_count;
    let findings = audit.findings;
    let report = json!({
        "result": if passed { "pass" } else { "fail" },
        "benchmark_version": "PIRB-0.1",
        "execution_claim": "synthetic_conformance_only",
        "corpus_path": path,
        "corpus_sha256": corpus_sha256,
        "schema_path": schema_path,
        "schema_sha256": schema_sha256,
        "source_commit": source_commit,
        "scenario_count": scenario_count,
        "scenarios": scenario_receipts,
        "findings": findings
    });

    match serde_json::to_string_pretty(&report) {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("could not serialize audit report: {}", error);
            return ExitCode::FAILURE;
        }
    }

    if passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
