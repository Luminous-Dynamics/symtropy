// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use scan_bim_decision_capture_adapter::{
    EMBEDDED_ADAPTER_CORPUS, EMBEDDED_ADAPTER_SCHEMA, validate_adapter_bytes,
};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process;

fn run() -> Result<i32, Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let mut corpus_path: Option<PathBuf> = None;
    let mut schema_path: Option<PathBuf> = None;

    while let Some(argument) = args.next() {
        let value = args.next().ok_or_else(|| {
            format!("missing value after {}", argument.to_string_lossy())
        })?;
        match argument.to_str() {
            Some("--corpus") => corpus_path = Some(PathBuf::from(value)),
            Some("--schema") => schema_path = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument: {}", argument.to_string_lossy()).into()),
        }
    }

    let corpus = match corpus_path {
        Some(path) => fs::read(path)?,
        None => EMBEDDED_ADAPTER_CORPUS.as_bytes().to_vec(),
    };
    let schema = match schema_path {
        Some(path) => fs::read(path)?,
        None => EMBEDDED_ADAPTER_SCHEMA.as_bytes().to_vec(),
    };

    let report = validate_adapter_bytes(&corpus, &schema);
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(if report.core.passed { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(error) => {
            eprintln!("scan-bim-decision-capture-adapter: {error}");
            process::exit(2);
        }
    }
}
