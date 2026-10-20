// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Command-line validator for the ecological-transfer oracle contract.

use std::env;
use std::fs;
use std::path::PathBuf;
use symtropy_animal_ecology_oracle::{
    COMMITTED_ORACLE_JSON, COMMITTED_SCHEMA_JSON, validate_artifacts,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<PathBuf> = env::args_os().skip(1).map(PathBuf::from).collect();
    if args.len() > 2 {
        return Err("usage: symtropy-animal-ecology-oracle [oracle.json [schema.json]]".into());
    }

    let oracle = match args.first() {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_ORACLE_JSON.to_owned(),
    };
    let schema = match args.get(1) {
        Some(path) => fs::read_to_string(path)?,
        None => COMMITTED_SCHEMA_JSON.to_owned(),
    };

    let report = validate_artifacts(&oracle, &schema)?;
    println!(
        "PASS: {} unique {} cases satisfy the versioned contract structure.",
        report.unique_case_ids, report.schema_version
    );
    println!(
        "Scope: contract consistency only; no simulation or biological validation was run."
    );
    Ok(())
}
