// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use humanity_history_core::{validate, Dataset};
use std::{env, fs, process};

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: humanity-history-validate <dataset.json>");
        process::exit(2);
    };

    let raw = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };

    let dataset: Dataset = match serde_json::from_str(&raw) {
        Ok(dataset) => dataset,
        Err(error) => {
            eprintln!("invalid dataset JSON in '{path}': {error}");
            process::exit(2);
        }
    };

    let report = validate(&dataset);
    let valid = report.is_valid();
    let error_count = report.error_count();
    let output = serde_json::json!({
        "valid": valid,
        "schema_version": dataset.schema_version,
        "error_count": error_count,
        "issues": report.issues,
    });

    match serde_json::to_string_pretty(&output) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("could not serialize validation report: {error}");
            process::exit(2);
        }
    }

    if !valid {
        process::exit(1);
    }
}
