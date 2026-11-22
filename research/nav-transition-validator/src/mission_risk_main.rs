// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use nav_transition_validator::mission_risk_contract::validate_mission_risk_contract;
use serde_json::Value;
use std::env;
use std::fs;
use std::process::ExitCode;

fn read_json(path: &str) -> Result<Value, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("{path}: unable to read file: {error}"))?;
    serde_json::from_str(&contents)
        .map_err(|error| format!("{path}: invalid JSON: {error}"))
}

fn main() -> ExitCode {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!(
            "Usage: {} <mission-risk-schema.json> <mission-risk-contract.json>",
            args.first().map(String::as_str).unwrap_or("nav-mission-risk-validate")
        );
        return ExitCode::from(2);
    }

    let schema = match read_json(&args[1]) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
    };
    let manifest = match read_json(&args[2]) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
    };

    let validator = match jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(&schema)
    {
        Ok(validator) => validator,
        Err(error) => {
            eprintln!("invalid Draft 2020-12 mission risk schema: {error}");
            return ExitCode::from(2);
        }
    };
    let schema_errors: Vec<String> = validator
        .iter_errors(&manifest)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        eprintln!("MISSION_RISK_SCHEMA_FAIL errors={}", schema_errors.len());
        for error in schema_errors { eprintln!("- {error}"); }
        return ExitCode::FAILURE;
    }
    println!("MISSION_RISK_SCHEMA_DRAFT_2020_12_PASS");

    match validate_mission_risk_contract(&manifest) {
        Ok(summary) => {
            println!(
                "MISSION_RISK_CONTRACT_SEMANTIC_PASS profile={} claim_status={} checks={} qualified_checks={} mission_budget_quanta={} allocated_quanta={} unallocated_quanta={} check_set_frozen={} structurally_complete={}",
                summary.mission_profile_id,
                summary.claim_status,
                summary.check_count,
                summary.qualified_check_count,
                summary.mission_budget_quanta,
                summary.allocated_quanta,
                summary.unallocated_quanta,
                summary.check_set_frozen,
                summary.structurally_complete,
            );
            println!("LIMIT: semantic consistency does not verify receipt contents, statistical calibration, independence, completeness of the declared hazard/check family, truth validity, continuous-time coverage or safety approval.");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("MISSION_RISK_CONTRACT_SEMANTIC_FAIL errors={}", errors.len());
            for error in errors { eprintln!("- {error}"); }
            ExitCode::FAILURE
        }
    }
}
