// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process};
use symtropy_experiment_core::ExperimentGraph;

fn usage() -> ! {
    eprintln!("usage: symtropy-experiment-run <experiment.json>");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let [path] = args.as_slice() else {
        usage();
    };

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };

    let graph: ExperimentGraph = match serde_json::from_str(&raw) {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("invalid experiment JSON in '{path}': {error}");
            process::exit(2);
        }
    };

    let report = graph.validate();
    if !report.is_valid() {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "valid": false,
                "experiment_id": graph.id,
                "issues": report.issues,
            }))
            .unwrap_or_else(|_| "{\"valid\":false,\"error\":\"failed to serialize report\"}".to_owned())
        );
        process::exit(1);
    }

    match graph.evaluate() {
        Ok(values) => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "valid": true,
                "executed": true,
                "kind": "acyclic_arithmetic_reference_interpreter",
                "schema_version": graph.schema_version,
                "experiment_id": graph.id,
                "values": values,
                "limitations": graph.limitations,
            }))
            .unwrap_or_else(|_| "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize report\"}".to_owned())
        ),
        Err(error) => {
            eprintln!("experiment graph could not be evaluated: {error}");
            process::exit(1);
        }
    }
}
