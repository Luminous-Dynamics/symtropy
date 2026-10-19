// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{env, fs, process};
use symtropy_experiment_core::{units::UNIT_REGISTRY_VERSION, ExperimentGraph};

fn usage() -> ! {
    eprintln!(
        "usage:\n  symtropy-experiment-run <experiment.json>\n  symtropy-experiment-run inspect <experiment.json>\n  symtropy-experiment-run sweep <experiment.json> <variable-id> <comma-separated-values>\n  symtropy-experiment-run oscillator <oscillator-spec.json>\n  symtropy-experiment-run oscillator-convergence <oscillator-spec.json> <coarse-to-fine-dt-csv>"
    );
    process::exit(2);
}

fn load_graph(path: &str) -> ExperimentGraph {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };

    match serde_json::from_str(&raw) {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("invalid experiment JSON in '{path}': {error}");
            process::exit(2);
        }
    }
}

fn print_json(value: serde_json::Value, fallback: &str) {
    match serde_json::to_string_pretty(&value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("{error}");
            println!("{fallback}");
            process::exit(2);
        }
    }
}

fn run_once(path: &str) {
    let graph = load_graph(path);
    let report = graph.validate();
    if !report.is_valid() {
        print_json(
            serde_json::json!({
                "valid": false,
                "unit_registry_version": UNIT_REGISTRY_VERSION,
                "experiment_id": graph.id,
                "issues": report.issues,
            }),
            "{\"valid\":false,\"error\":\"failed to serialize validation report\"}",
        );
        process::exit(1);
    }

    match graph.evaluate() {
        Ok(values) => print_json(
            serde_json::json!({
                "valid": true,
                "executed": true,
                "kind": "acyclic_arithmetic_reference_interpreter",
                "schema_version": graph.schema_version,
                "unit_registry_version": UNIT_REGISTRY_VERSION,
                "experiment_id": graph.id,
                "values": values,
                "limitations": graph.limitations,
            }),
            "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize report\"}",
        ),
        Err(error) => {
            eprintln!("experiment graph could not be evaluated: {error}");
            process::exit(1);
        }
    }
}

fn run_inspect(path: &str) {
    let graph = load_graph(path);
    let projection = match graph.visual_math_projection() {
        Ok(projection) => projection,
        Err(error) => {
            eprintln!("experiment graph cannot be projected: {error}");
            process::exit(1);
        }
    };
    let baseline_evaluation = match graph.evaluate() {
        Ok(values) => serde_json::json!({
            "executed": true,
            "values": values,
            "error": null
        }),
        Err(error) => serde_json::json!({
            "executed": false,
            "values": null,
            "error": error
        }),
    };
    print_json(
        serde_json::json!({
            "valid": true,
            "kind": "renderer_neutral_visual_math_projection",
            "unit_registry_version": UNIT_REGISTRY_VERSION,
            "projection": projection,
            "baseline_evaluation": baseline_evaluation,
            "interpretation_warning": "Formula labels are derived from the executable operator graph. The projection preserves semantic IDs and evidence references but does not prove that equations are scientifically or historically correct. An incomplete model can still be inspected when its baseline evaluation fails."
        }),
        "{\"valid\":true,\"error\":\"failed to serialize visual projection\"}",
    );
}

fn run_sweep(path: &str, variable_id: &str, values_csv: &str) {
    let graph = load_graph(path);
    let values: Vec<String> = values_csv.split(',').map(str::trim).map(str::to_owned).collect();
    if values.iter().any(String::is_empty) {
        eprintln!("sweep values must be non-empty comma-separated finite numbers");
        process::exit(2);
    }

    match graph.run_parameter_sweep(variable_id, &values) {
        Ok(points) => print_json(
            serde_json::json!({
                "valid": true,
                "executed": true,
                "kind": "deterministic_one_parameter_sweep",
                "schema_version": graph.schema_version,
                "unit_registry_version": UNIT_REGISTRY_VERSION,
                "experiment_id": graph.id,
                "swept_variable_id": variable_id,
                "point_count": points.len(),
                "points": points,
                "limitations": graph.limitations,
                "interpretation_warning": "This is a deterministic scenario sweep, not a probabilistic uncertainty interval or global sensitivity analysis."
            }),
            "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize sweep report\"}",
        ),
        Err(error) => {
            eprintln!("experiment sweep could not be executed: {error}");
            process::exit(1);
        }
    }
}

fn run_oscillator(path: &str) {
    use symtropy_experiment_core::{HarmonicOscillatorSpec, simulate_harmonic_oscillator};

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };
    let spec: HarmonicOscillatorSpec = match serde_json::from_str(&raw) {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("invalid harmonic-oscillator JSON in '{path}': {error}");
            process::exit(2);
        }
    };

    match simulate_harmonic_oscillator(&spec) {
        Ok(result) => print_json(
            serde_json::json!({
                "valid": true,
                "executed": true,
                "kind": "analytical_reference_harmonic_oscillator",
                "result": result,
                "interpretation_warning": "Errors compare a numerical integrator with the exact solution of the stated ideal oscillator. They are not physical measurement errors or proof that a real apparatus follows this model."
            }),
            "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize oscillator result\"}",
        ),
        Err(error) => {
            eprintln!("harmonic oscillator could not be simulated: {error}");
            process::exit(1);
        }
    }
}

fn run_oscillator_convergence(path: &str, time_steps_csv: &str) {
    use symtropy_experiment_core::{analyze_harmonic_oscillator_convergence, HarmonicOscillatorSpec};

    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };
    let spec: HarmonicOscillatorSpec = match serde_json::from_str(&raw) {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("invalid harmonic-oscillator JSON in '{path}': {error}");
            process::exit(2);
        }
    };
    let mut time_steps_s = Vec::new();
    for token in time_steps_csv.split(',').map(str::trim) {
        match token.parse::<f64>() {
            Ok(value) if value.is_finite() && value > 0.0 => time_steps_s.push(value),
            _ => {
                eprintln!("time-step values must be finite positive numbers");
                process::exit(2);
            }
        }
    }

    match analyze_harmonic_oscillator_convergence(&spec, &time_steps_s) {
        Ok(report) => print_json(
            serde_json::json!({
                "valid": true,
                "executed": true,
                "kind": "analytical_reference_convergence_study",
                "report": report,
                "interpretation_warning": "Observed order estimates are conditional on this ideal oscillator, initial state, finite time horizon, and time-step sequence. They are not a universal solver-order or physical-validation claim."
            }),
            "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize convergence report\"}",
        ),
        Err(error) => {
            eprintln!("oscillator convergence study could not be executed: {error}");
            process::exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [path] => run_once(path),
        [mode, path] if mode == "inspect" => run_inspect(path),
        [mode, path, variable_id, values_csv] if mode == "sweep" => {
            run_sweep(path, variable_id, values_csv)
        }
        [mode, path] if mode == "oscillator" => run_oscillator(path),
        [mode, path, time_steps] if mode == "oscillator-convergence" => {
            run_oscillator_convergence(path, time_steps)
        }
        _ => usage(),
    }
}
