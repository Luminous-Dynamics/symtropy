// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use humanity_history_core::{analyze_estimate_contrast, validate, Dataset};
use std::{env, fs, process};

fn usage() -> ! {
    eprintln!(
        "usage:\n  humanity-history-validate <dataset.json>\n  humanity-history-validate analyze-contrasts <dataset.json>"
    );
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [path] => ("validate", path.as_str()),
        [mode, path] if mode == "analyze-contrasts" => ("analyze-contrasts", path.as_str()),
        _ => usage(),
    };

    let raw = match fs::read_to_string(path) {
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
    if !report.is_valid() {
        let output = serde_json::json!({
            "valid": false,
            "schema_version": dataset.schema_version,
            "error_count": report.error_count(),
            "issues": report.issues,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|_| {
            "{\"valid\":false,\"error\":\"failed to serialize validation report\"}".to_owned()
        }));
        process::exit(1);
    }

    if mode == "analyze-contrasts" {
        let diagnostics: Result<Vec<_>, _> = dataset
            .estimate_contrasts
            .iter()
            .map(|contrast| analyze_estimate_contrast(&dataset, contrast))
            .collect();
        let diagnostics = match diagnostics {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                eprintln!("cannot analyze estimate contrasts: {error}");
                process::exit(2);
            }
        };
        let output = serde_json::json!({
            "valid": true,
            "kind": "descriptive_point_interval_sensitivity",
            "schema_version": dataset.schema_version,
            "diagnostic_count": diagnostics.len(),
            "interpretation_warning": "Interval position and distance are descriptive only. They are not posterior model probabilities, Bayes factors, calibrated confidence scores, or standalone historical verdicts.",
            "diagnostics": diagnostics,
        });
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("could not serialize sensitivity report: {error}");
                process::exit(2);
            }
        }
    } else {
        let output = serde_json::json!({
            "valid": true,
            "schema_version": dataset.schema_version,
            "error_count": 0,
            "issues": [],
        });
        match serde_json::to_string_pretty(&output) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("could not serialize validation report: {error}");
                process::exit(2);
            }
        }
    }
}
