// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Command-line validator for the ecological-transfer oracle contract.

use std::env;
use std::fs;
use std::path::PathBuf;
use symtropy_animal_ecology_oracle::{
    COMMITTED_ORACLE_JSON, COMMITTED_PILOT_JSON, COMMITTED_PILOT_SCHEMA_JSON,
    COMMITTED_SCHEMA_JSON, validate_artifacts, validate_pilot_manifest,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    if args.len() > 4 {
        return Err(
            "usage: symtropy-animal-ecology-oracle [oracle.json [schema.json [pilot.json [pilot-schema.json]]]]"
                .into(),
        );
    }

    let oracle = match args.first() {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_ORACLE_JSON.to_owned(),
    };
    let schema = match args.get(1) {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_SCHEMA_JSON.to_owned(),
    };

    let pilot = match args.get(2) {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_PILOT_JSON.to_owned(),
    };
    let pilot_schema = match args.get(3) {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_PILOT_SCHEMA_JSON.to_owned(),
    };

    let report = validate_artifacts(&oracle, &schema)?;
    let pilot_report = validate_pilot_manifest(&pilot, &pilot_schema)?;
    println!(
        "PASS: {} unique {} cases satisfy the versioned contract structure.",
        report.unique_case_ids, report.schema_version
    );
    println!(
        "PASS: pilot {} with {} seeds and {} model structures satisfies its scenario contract.",
        pilot_report.scenario_id, pilot_report.seed_count, pilot_report.model_count
    );
    println!("Scope: contract consistency only; no simulation or biological validation was run.");
    Ok(())
}
