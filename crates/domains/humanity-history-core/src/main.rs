// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use humanity_history_core::{analyze_estimate_contrast, analyze_estimate_delta, validate, Dataset};
use std::{env, fs, process};

fn usage() -> ! {
    eprintln!(
        "usage:\n  humanity-history-validate <dataset.json>\n  humanity-history-validate analyze-sensitivity <dataset.json>"
    );
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (mode, path) = match args.as_slice() {
        [path] => ("validate", path.as_str()),
        [mode, path] if mode == "analyze-sensitivity" || mode == "analyze-contrasts" => ("analyze-sensitivity", path.as_str()),
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

    if mode == "analyze-sensitivity" {
        let interval_diagnostics: Result<Vec<_>, _> = dataset
            .estimate_contrasts
            .iter()
            .map(|contrast| analyze_estimate_contrast(&dataset, contrast))
            .collect();
        let interval_diagnostics = match interval_diagnostics {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                eprintln!("cannot analyze estimate interval contrasts: {error}");
                process::exit(2);
            }
        };
        let delta_diagnostics: Result<Vec<_>, _> = dataset
            .estimate_deltas
            .iter()
            .map(|delta| analyze_estimate_delta(&dataset, delta))
            .collect();
        let delta_diagnostics = match delta_diagnostics {
            Ok(diagnostics) => diagnostics,
            Err(error) => {
                eprintln!("cannot analyze estimate deltas: {error}");
                process::exit(2);
            }
        };
        let output = serde_json::json!({
            "valid": true,
            "kind": "descriptive_sensitivity_analysis",
            "schema_version": dataset.schema_version,
            "point_interval_diagnostic_count": interval_diagnostics.len(),
            "paired_delta_diagnostic_count": delta_diagnostics.len(),
            "interpretation_warning": "These calculations describe source-reported values. They do not independently reproduce the source's model, calculate new posterior probabilities, estimate Bayes factors, or establish a historical verdict. Analyses of full and trimmed versions of the same observations are not independent replications.",
            "point_interval_diagnostics": interval_diagnostics,
            "paired_delta_diagnostics": delta_diagnostics,
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
