// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Read-only validator for a machine-cycle to QIF inspection association projection.

use mfg_architecture_profile_validator::inspection_evaluator::evaluate_inspection_join;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

fn main() -> ExitCode {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    if args.len() != 2 {
        eprintln!("usage: validate-inspection-join <inspection-join.schema.json> <candidate.json>");
        return ExitCode::from(2);
    }

    let schema = match read_json(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let candidate = match read_json(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };

    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        eprintln!("schema: expected JSON Schema Draft 2020-12");
        return ExitCode::FAILURE;
    }
    if let Err(error) = jsonschema::draft202012::meta::validate(&schema) {
        eprintln!("schema: invalid Draft 2020-12 schema: {error}");
        return ExitCode::FAILURE;
    }

    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
    {
        Ok(value) => value,
        Err(error) => {
            eprintln!("schema: cannot build validator: {error}");
            return ExitCode::FAILURE;
        }
    };

    let schema_errors: Vec<String> = validator
        .iter_errors(&candidate)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    if !schema_errors.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema_valid": false,
                "admissible_for_declared_synthetic_join": false,
                "schema_errors": schema_errors,
                "violations": []
            }))
            .expect("JSON report serializes")
        );
        return ExitCode::FAILURE;
    }

    let evaluation = evaluate_inspection_join(&candidate);
    let violations: Vec<Value> = evaluation
        .violations
        .iter()
        .map(|violation| {
            json!({
                "code": violation.code,
                "field": violation.field
            })
        })
        .collect();

    let report = json!({
        "schema_valid": true,
        "admissible_for_declared_synthetic_join": evaluation.admissible_for_declared_synthetic_join,
        "metrology_reference_state": evaluation.metrology_reference_state,
        "physical_metrology_qualified": evaluation.physical_metrology_qualified,
        "violations": violations,
        "claim_ceiling": "synthetic structural consistency only; no source authority, clock map, metrology, conformity, product release or physical manufacturing is qualified"
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("JSON report serializes")
    );

    if evaluation.admissible_for_declared_synthetic_join {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
