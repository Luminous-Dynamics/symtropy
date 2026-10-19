// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::BTreeMap, env, fs, process};

use serde::Serialize;
use serde_json::Value;
use symtropy_experiment_core::{
    units::UNIT_REGISTRY_VERSION, ExperimentGraph, RunReceipt,
};

fn usage() -> ! {
    eprintln!(
        "usage:\n  symtropy-experiment-run <experiment.json>\n  symtropy-experiment-run inspect <experiment.json>\n  symtropy-experiment-run sweep <experiment.json> <variable-id> <comma-separated-values>\n  symtropy-experiment-run oscillator <oscillator-spec.json>\n  symtropy-experiment-run oscillator-convergence <oscillator-spec.json> <coarse-to-fine-dt-csv>"
    );
    process::exit(2);
}

fn load_graph(path: &str) -> (String, ExperimentGraph) {
    let raw = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    };

    let graph = match serde_json::from_str(&raw) {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("invalid experiment JSON in '{path}': {error}");
            process::exit(2);
        }
    };
    (raw, graph)
}

#[allow(clippy::too_many_arguments)]
fn run_receipt_value<M: Serialize, O: Serialize>(
    mode: &str,
    raw_input: &str,
    model: &M,
    output_payload: &O,
    experiment_id: Option<&str>,
    model_schema_version: Option<&str>,
    unit_registry_version: Option<&str>,
    execution_settings: BTreeMap<String, String>,
) -> Value {
    let receipt = RunReceipt::build(
        mode,
        raw_input.as_bytes(),
        model,
        output_payload,
        experiment_id.map(str::to_owned),
        model_schema_version.map(str::to_owned),
        unit_registry_version.map(str::to_owned),
        execution_settings,
    )
    .unwrap_or_else(|error| {
        eprintln!("could not create run receipt: {error}");
        process::exit(2);
    });
    serde_json::to_value(receipt).unwrap_or_else(|error| {
        eprintln!("could not serialize run receipt: {error}");
        process::exit(2);
    })
}

fn settings(entries: &[(&str, String)]) -> BTreeMap<String, String> {
    entries
        .iter()
        .map(|(key, value)| ((*key).to_owned(), value.clone()))
        .collect()
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
    let (raw, graph) = load_graph(path);
    let report = graph.validate();
    if !report.is_valid() {
        let output_payload = serde_json::json!({
            "valid": false,
            "experiment_id": &graph.id,
            "issues": &report.issues,
        });
        let receipt = run_receipt_value(
            "run",
            &raw,
            &graph,
            &output_payload,
            Some(&graph.id),
            Some(&graph.schema_version),
            Some(UNIT_REGISTRY_VERSION),
            settings(&[("numeric_format", "IEEE-754 binary64".to_owned())]),
        );
        print_json(
            serde_json::json!({
                "valid": false,
                "unit_registry_version": UNIT_REGISTRY_VERSION,
                "experiment_id": graph.id,
                "issues": report.issues,
                "run_receipt": receipt,
            }),
            "{\"valid\":false,\"error\":\"failed to serialize validation report\"}",
        );
        process::exit(1);
    }

    match graph.evaluate() {
        Ok(values) => {
            let receipt = run_receipt_value(
                "run",
                &raw,
                &graph,
                &values,
                Some(&graph.id),
                Some(&graph.schema_version),
                Some(UNIT_REGISTRY_VERSION),
                settings(&[("numeric_format", "IEEE-754 binary64".to_owned())]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": true,
                    "kind": "acyclic_arithmetic_reference_interpreter",
                    "schema_version": graph.schema_version,
                    "unit_registry_version": UNIT_REGISTRY_VERSION,
                    "experiment_id": graph.id,
                    "values": values,
                    "limitations": graph.limitations,
                    "run_receipt": receipt,
                }),
                "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize report\"}",
            );
        }
        Err(error) => {
            let output_payload = serde_json::json!({
                "executed": false,
                "error": &error,
            });
            let receipt = run_receipt_value(
                "run",
                &raw,
                &graph,
                &output_payload,
                Some(&graph.id),
                Some(&graph.schema_version),
                Some(UNIT_REGISTRY_VERSION),
                settings(&[("numeric_format", "IEEE-754 binary64".to_owned())]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": false,
                    "experiment_id": graph.id,
                    "error": error,
                    "run_receipt": receipt,
                }),
                "{"valid":true,"executed":false,"error":"could not serialize failed-run receipt"}",
            );
            process::exit(1);
        }
    }
}

fn run_inspect(path: &str) {
    let (raw, graph) = load_graph(path);
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
    let output_payload = serde_json::json!({
        "projection": &projection,
        "baseline_evaluation": &baseline_evaluation,
    });
    let receipt = run_receipt_value(
        "inspect",
        &raw,
        &graph,
        &output_payload,
        Some(&graph.id),
        Some(&graph.schema_version),
        Some(UNIT_REGISTRY_VERSION),
        settings(&[(
            "projection",
            "renderer_neutral_visual_math_projection".to_owned(),
        )]),
    );
    print_json(
        serde_json::json!({
            "valid": true,
            "kind": "renderer_neutral_visual_math_projection",
            "unit_registry_version": UNIT_REGISTRY_VERSION,
            "projection": projection,
            "baseline_evaluation": baseline_evaluation,
            "run_receipt": receipt,
            "interpretation_warning": "Formula labels are derived from the executable operator graph. The projection preserves semantic IDs and evidence references but does not prove that equations are scientifically or historically correct. An incomplete model can still be inspected when its baseline evaluation fails."
        }),
        "{\"valid\":true,\"error\":\"failed to serialize visual projection\"}",
    );
}

fn run_sweep(path: &str, variable_id: &str, values_csv: &str) {
    let (raw, graph) = load_graph(path);
    let values: Vec<String> = values_csv.split(',').map(str::trim).map(str::to_owned).collect();
    if values.iter().any(String::is_empty) {
        let message = "sweep values must be non-empty comma-separated finite numbers";
        let output_payload = serde_json::json!({
            "executed": false,
            "error": message,
        });
        let receipt = run_receipt_value(
            "sweep",
            &raw,
            &graph,
            &output_payload,
            Some(&graph.id),
            Some(&graph.schema_version),
            Some(UNIT_REGISTRY_VERSION),
            settings(&[
                ("swept_variable_id", variable_id.to_owned()),
                ("sweep_values", values_csv.to_owned()),
            ]),
        );
        print_json(
            serde_json::json!({
                "valid": true,
                "executed": false,
                "error": message,
                "run_receipt": receipt,
            }),
            "{"valid":true,"executed":false,"error":"could not serialize invalid-sweep receipt"}",
        );
        process::exit(2);
    }

    match graph.run_parameter_sweep(variable_id, &values) {
        Ok(points) => {
            let receipt = run_receipt_value(
                "sweep",
                &raw,
                &graph,
                &points,
                Some(&graph.id),
                Some(&graph.schema_version),
                Some(UNIT_REGISTRY_VERSION),
                settings(&[
                    ("swept_variable_id", variable_id.to_owned()),
                    ("sweep_values", values_csv.to_owned()),
                ]),
            );
            print_json(
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
                    "run_receipt": receipt,
                    "interpretation_warning": "This is a deterministic scenario sweep, not a probabilistic uncertainty interval or global sensitivity analysis."
                }),
                "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize sweep report\"}",
            );
        }
        Err(error) => {
            let output_payload = serde_json::json!({
                "executed": false,
                "error": &error,
            });
            let receipt = run_receipt_value(
                "sweep",
                &raw,
                &graph,
                &output_payload,
                Some(&graph.id),
                Some(&graph.schema_version),
                Some(UNIT_REGISTRY_VERSION),
                settings(&[
                    ("swept_variable_id", variable_id.to_owned()),
                    ("sweep_values", values_csv.to_owned()),
                ]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": false,
                    "experiment_id": graph.id,
                    "error": error,
                    "run_receipt": receipt,
                }),
                "{"valid":true,"executed":false,"error":"could not serialize failed-sweep receipt"}",
            );
            process::exit(1);
        }
    }
}

fn read_text(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("could not read '{path}': {error}");
            process::exit(2);
        }
    }
}

fn run_oscillator(path: &str) {
    use symtropy_experiment_core::{simulate_harmonic_oscillator, HarmonicOscillatorSpec};

    let raw = read_text(path);
    let spec: HarmonicOscillatorSpec = match serde_json::from_str(&raw) {
        Ok(spec) => spec,
        Err(error) => {
            eprintln!("invalid harmonic-oscillator JSON in '{path}': {error}");
            process::exit(2);
        }
    };

    match simulate_harmonic_oscillator(&spec) {
        Ok(result) => {
            let receipt = run_receipt_value(
                "oscillator",
                &raw,
                &spec,
                &result,
                Some(&spec.id),
                Some(&spec.schema_version),
                None,
                settings(&[
                    ("integrator", format!("{:?}", spec.integrator)),
                    ("time_step_s", spec.time_step_s.to_string()),
                    ("duration_s", spec.duration_s.to_string()),
                ]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": true,
                    "kind": "analytical_reference_harmonic_oscillator",
                    "result": result,
                    "run_receipt": receipt,
                    "interpretation_warning": "Errors compare a numerical integrator with the exact solution of the stated ideal oscillator. They are not physical measurement errors or proof that a real apparatus follows this model."
                }),
                "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize oscillator result\"}",
            );
        }
        Err(error) => {
            let output_payload = serde_json::json!({
                "executed": false,
                "error": &error,
            });
            let receipt = run_receipt_value(
                "oscillator",
                &raw,
                &spec,
                &output_payload,
                Some(&spec.id),
                Some(&spec.schema_version),
                None,
                settings(&[
                    ("integrator", format!("{:?}", spec.integrator)),
                    ("time_step_s", spec.time_step_s.to_string()),
                    ("duration_s", spec.duration_s.to_string()),
                ]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": false,
                    "error": error,
                    "run_receipt": receipt,
                }),
                "{"valid":true,"executed":false,"error":"could not serialize failed-oscillator receipt"}",
            );
            process::exit(1);
        }
    }
}

fn run_oscillator_convergence(path: &str, time_steps_csv: &str) {
    use symtropy_experiment_core::{
        analyze_harmonic_oscillator_convergence, HarmonicOscillatorSpec,
    };

    let raw = read_text(path);
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
                let message = "time-step values must be finite positive numbers";
                let output_payload = serde_json::json!({
                    "executed": false,
                    "error": message,
                });
                let receipt = run_receipt_value(
                    "oscillator-convergence",
                    &raw,
                    &spec,
                    &output_payload,
                    Some(&spec.id),
                    Some(&spec.schema_version),
                    None,
                    settings(&[
                        ("integrator", format!("{:?}", spec.integrator)),
                        ("coarse_to_fine_time_steps_s", time_steps_csv.to_owned()),
                        ("duration_s", spec.duration_s.to_string()),
                    ]),
                );
                print_json(
                    serde_json::json!({
                        "valid": true,
                        "executed": false,
                        "error": message,
                        "run_receipt": receipt,
                    }),
                    "{"valid":true,"executed":false,"error":"could not serialize invalid-convergence receipt"}",
                );
                process::exit(2);
            }
        }
    }

    match analyze_harmonic_oscillator_convergence(&spec, &time_steps_s) {
        Ok(report) => {
            let receipt = run_receipt_value(
                "oscillator-convergence",
                &raw,
                &spec,
                &report,
                Some(&spec.id),
                Some(&spec.schema_version),
                None,
                settings(&[
                    ("integrator", format!("{:?}", spec.integrator)),
                    ("coarse_to_fine_time_steps_s", time_steps_csv.to_owned()),
                    ("duration_s", spec.duration_s.to_string()),
                ]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": true,
                    "kind": "analytical_reference_convergence_study",
                    "report": report,
                    "run_receipt": receipt,
                    "interpretation_warning": "Observed order estimates are conditional on this ideal oscillator, initial state, finite time horizon, and time-step sequence. They are not a universal solver-order or physical-validation claim."
                }),
                "{\"valid\":true,\"executed\":false,\"error\":\"failed to serialize convergence report\"}",
            );
        }
        Err(error) => {
            let output_payload = serde_json::json!({
                "executed": false,
                "error": &error,
            });
            let receipt = run_receipt_value(
                "oscillator-convergence",
                &raw,
                &spec,
                &output_payload,
                Some(&spec.id),
                Some(&spec.schema_version),
                None,
                settings(&[
                    ("integrator", format!("{:?}", spec.integrator)),
                    ("coarse_to_fine_time_steps_s", time_steps_csv.to_owned()),
                    ("duration_s", spec.duration_s.to_string()),
                ]),
            );
            print_json(
                serde_json::json!({
                    "valid": true,
                    "executed": false,
                    "error": error,
                    "run_receipt": receipt,
                }),
                "{"valid":true,"executed":false,"error":"could not serialize failed-convergence receipt"}",
            );
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
