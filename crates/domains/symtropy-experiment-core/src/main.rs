// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::{collections::BTreeMap, env, fs, process};

use serde::Serialize;
use serde_json::Value;
use symtropy_experiment_core::{
    units::UNIT_REGISTRY_VERSION, ExperimentGraph, HarmonicOscillatorResult,
    HarmonicOscillatorSpec, OscillatorConvergenceReport, RunReceipt, SweepPoint,
    provenance::{RECEIPT_CANONICALIZATION_ALGORITHM, RUN_RECEIPT_SCHEMA_VERSION},
};

fn usage() -> ! {
    eprintln!(
        "usage:\n  symtropy-experiment-run <experiment.json>\n  symtropy-experiment-run inspect <experiment.json>\n  symtropy-experiment-run sweep <experiment.json> <variable-id> <comma-separated-values>\n  symtropy-experiment-run oscillator <oscillator-spec.json>\n  symtropy-experiment-run oscillator-convergence <oscillator-spec.json> <coarse-to-fine-dt-csv>\n  symtropy-experiment-run verify-receipt <result-envelope.json> <original-input.json>"
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

fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json_canonicalizer::to_vec(value)
        .map_err(|error| format!("could not canonicalize JSON as RFC 8785 JCS: {error}"))
}

fn canonical_output_payload_bytes(
    envelope: &Value,
    receipt: &RunReceipt,
) -> Result<Vec<u8>, String> {
    let mode = receipt.payload.execution_mode.as_str();
    let failure_payload = || {
        serde_json::json!({
            "executed": false,
            "error": envelope.get("error").cloned().unwrap_or(Value::Null),
        })
    };

    let payload: Value = match mode {
        "run" => {
            if let Some(values) = envelope.get("values") {
                return canonical_json_bytes(values)
                    .map_err(|error| format!("could not canonicalize run values: {error}"));
            }
            if envelope.get("issues").is_some() {
                serde_json::json!({
                    "valid": false,
                    "experiment_id": envelope.get("experiment_id").cloned().unwrap_or(Value::Null),
                    "issues": envelope.get("issues").cloned().unwrap_or(Value::Null),
                })
            } else {
                failure_payload()
            }
        }
        "inspect" => serde_json::json!({
            "projection": envelope.get("projection").cloned().unwrap_or(Value::Null),
            "baseline_evaluation": envelope.get("baseline_evaluation").cloned().unwrap_or(Value::Null),
        }),
        "sweep" => {
            if let Some(points_value) = envelope.get("points") {
                let points: Vec<SweepPoint> = serde_json::from_value(points_value.clone())
                    .map_err(|error| format!("invalid sweep points in result envelope: {error}"))?;
                return canonical_json_bytes(&points)
                    .map_err(|error| format!("could not canonicalize sweep points: {error}"));
            }
            failure_payload()
        }
        "oscillator" => {
            if let Some(result_value) = envelope.get("result") {
                let result: HarmonicOscillatorResult = serde_json::from_value(result_value.clone())
                    .map_err(|error| format!("invalid oscillator result in envelope: {error}"))?;
                return canonical_json_bytes(&result)
                    .map_err(|error| format!("could not canonicalize oscillator result: {error}"));
            }
            failure_payload()
        }
        "oscillator-convergence" => {
            if let Some(report_value) = envelope.get("report") {
                let report: OscillatorConvergenceReport = serde_json::from_value(report_value.clone())
                    .map_err(|error| format!("invalid convergence report in envelope: {error}"))?;
                return canonical_json_bytes(&report)
                    .map_err(|error| format!("could not canonicalize convergence report: {error}"));
            }
            failure_payload()
        }
        other => return Err(format!("unsupported receipt execution mode '{other}'")),
    };
    canonical_json_bytes(&payload).map_err(|error| format!("could not canonicalize output payload: {error}"))
}

fn run_verify_receipt(envelope_path: &str, input_path: &str) {
    let envelope_raw = read_text(envelope_path);
    let envelope: Value = match serde_json::from_str(&envelope_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid result envelope JSON in '{envelope_path}': {error}");
            process::exit(2);
        }
    };
    let Some(receipt_value) = envelope.get("run_receipt").cloned() else {
        eprintln!("result envelope has no 'run_receipt' field");
        process::exit(2);
    };
    let receipt: RunReceipt = match serde_json::from_value(receipt_value) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid run receipt in '{envelope_path}': {error}");
            process::exit(2);
        }
    };

    let input_bytes = match fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read original input artifact '{input_path}': {error}");
            process::exit(2);
        }
    };
    let (model_bytes, model_metadata_matches) = match receipt.payload.execution_mode.as_str() {
        "run" | "inspect" | "sweep" => {
            let graph: ExperimentGraph = match serde_json::from_slice(&input_bytes) {
                Ok(graph) => graph,
                Err(error) => {
                    eprintln!("input artifact is not a valid experiment graph: {error}");
                    process::exit(2);
                }
            };
            let metadata_matches = receipt.payload.experiment_id.as_deref() == Some(graph.id.as_str())
                && receipt.payload.model_schema_version.as_deref() == Some(graph.schema_version.as_str())
                && receipt.payload.unit_registry_version.as_deref() == Some(UNIT_REGISTRY_VERSION);
            let bytes = match canonical_json_bytes(&graph) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("could not serialize parsed experiment graph: {error}");
                    process::exit(2);
                }
            };
            (bytes, metadata_matches)
        }
        "oscillator" | "oscillator-convergence" => {
            let spec: HarmonicOscillatorSpec = match serde_json::from_slice(&input_bytes) {
                Ok(spec) => spec,
                Err(error) => {
                    eprintln!("input artifact is not a valid oscillator specification: {error}");
                    process::exit(2);
                }
            };
            let metadata_matches = receipt.payload.experiment_id.as_deref() == Some(spec.id.as_str())
                && receipt.payload.model_schema_version.as_deref() == Some(spec.schema_version.as_str())
                && receipt.payload.unit_registry_version.is_none();
            let bytes = match canonical_json_bytes(&spec) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("could not serialize parsed oscillator specification: {error}");
                    process::exit(2);
                }
            };
            (bytes, metadata_matches)
        }
        other => {
            eprintln!("unsupported receipt execution mode '{other}'");
            process::exit(2);
        }
    };
    let output_bytes = match canonical_output_payload_bytes(&envelope, &receipt) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let verification = match receipt.verify_artifact_bytes(&input_bytes, &model_bytes, &output_bytes) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("could not verify run receipt: {error}");
            process::exit(2);
        }
    };
    let receipt_contract_supported = receipt.payload.receipt_schema_version == RUN_RECEIPT_SCHEMA_VERSION
        && receipt.payload.digest_algorithm == "sha256"
        && receipt.payload.canonicalization_algorithm == RECEIPT_CANONICALIZATION_ALGORITHM;
    let valid = verification.is_valid() && model_metadata_matches && receipt_contract_supported;
    print_json(
        serde_json::json!({
            "valid": valid,
            "receipt_sha256": receipt.receipt_sha256,
            "model_metadata_matches": model_metadata_matches,
            "receipt_contract_supported": receipt_contract_supported,
            "verification": &verification,
            "scope_warning": "This verifies receipt self-integrity and matches the saved input artifact, parsed-model serialization, output payload, and model identity/version fields to recorded digests. It does not authenticate the producer, prove that execution occurred, or validate scientific correctness."
        }),
        "{"valid":false,"error":"could not serialize receipt verification report"}",
    );
    if !valid {
        process::exit(1);
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
        [mode, envelope_path, input_path] if mode == "verify-receipt" => {
            run_verify_receipt(envelope_path, input_path)
        }
        _ => usage(),
    }
}

#[cfg(test)]
mod receipt_cli_tests {
    use super::*;
    use symtropy_experiment_core::{
        analyze_harmonic_oscillator_convergence, simulate_harmonic_oscillator,
    };

    fn verify<M: Serialize>(
        receipt: &RunReceipt,
        input: &str,
        model: &M,
        envelope: &Value,
    ) -> symtropy_experiment_core::RunReceiptVerification {
        let model_bytes = canonical_json_bytes(model).unwrap();
        let output_bytes = canonical_output_payload_bytes(envelope, receipt).unwrap();
        receipt
            .verify_artifact_bytes(input.as_bytes(), &model_bytes, &output_bytes)
            .unwrap()
    }

    fn graph_fixture() -> (String, ExperimentGraph) {
        let raw = include_str!("../fixtures/gear-ratio.json").to_owned();
        let graph = serde_json::from_str(&raw).unwrap();
        (raw, graph)
    }

    #[test]
    fn run_envelope_verifies_against_original_input_and_output() {
        let (raw, graph) = graph_fixture();
        let values = graph.evaluate().unwrap();
        let receipt = RunReceipt::build(
            "run",
            raw.as_bytes(),
            &graph,
            &values,
            Some(graph.id.clone()),
            Some(graph.schema_version.clone()),
            Some(UNIT_REGISTRY_VERSION.to_owned()),
            settings(&[("numeric_format", "IEEE-754 binary64".to_owned())]),
        ).unwrap();
        let envelope = serde_json::json!({"values": &values, "run_receipt": &receipt});
        assert!(verify(&receipt, &raw, &graph, &envelope).is_valid());
    }

    #[test]
    fn edited_run_output_fails_artifact_verification() {
        let (raw, graph) = graph_fixture();
        let values = graph.evaluate().unwrap();
        let receipt = RunReceipt::build(
            "run",
            raw.as_bytes(),
            &graph,
            &values,
            Some(graph.id.clone()),
            Some(graph.schema_version.clone()),
            Some(UNIT_REGISTRY_VERSION.to_owned()),
            settings(&[("numeric_format", "IEEE-754 binary64".to_owned())]),
        ).unwrap();
        let mut changed_values = values;
        changed_values.insert("variable:theta_output".to_owned(), "999".to_owned());
        let envelope = serde_json::json!({"values": changed_values, "run_receipt": &receipt});
        let verified = verify(&receipt, &raw, &graph, &envelope);
        assert!(verified.receipt_digest_valid);
        assert!(!verified.output_payload_digest_matches);
        assert!(!verified.is_valid());
    }

    #[test]
    fn sweep_envelope_reconstructs_typed_sweep_payload() {
        let (raw, graph) = graph_fixture();
        let points = graph.run_parameter_sweep(
            "variable:n_driver",
            &["47".to_owned(), "48".to_owned(), "49".to_owned()],
        ).unwrap();
        let receipt = RunReceipt::build(
            "sweep",
            raw.as_bytes(),
            &graph,
            &points,
            Some(graph.id.clone()),
            Some(graph.schema_version.clone()),
            Some(UNIT_REGISTRY_VERSION.to_owned()),
            settings(&[
                ("swept_variable_id", "variable:n_driver".to_owned()),
                ("sweep_values", "47,48,49".to_owned()),
            ]),
        ).unwrap();
        let envelope = serde_json::json!({"points": &points, "run_receipt": &receipt});
        assert!(verify(&receipt, &raw, &graph, &envelope).is_valid());
    }

    #[test]
    fn inspection_envelope_reconstructs_projection_payload() {
        let (raw, graph) = graph_fixture();
        let projection = graph.visual_math_projection().unwrap();
        let baseline = serde_json::json!({"executed": true, "values": graph.evaluate().unwrap(), "error": null});
        let output = serde_json::json!({"projection": &projection, "baseline_evaluation": &baseline});
        let receipt = RunReceipt::build(
            "inspect",
            raw.as_bytes(),
            &graph,
            &output,
            Some(graph.id.clone()),
            Some(graph.schema_version.clone()),
            Some(UNIT_REGISTRY_VERSION.to_owned()),
            settings(&[("projection", "renderer_neutral_visual_math_projection".to_owned())]),
        ).unwrap();
        let envelope = serde_json::json!({
            "projection": projection,
            "baseline_evaluation": baseline,
            "run_receipt": &receipt,
        });
        assert!(verify(&receipt, &raw, &graph, &envelope).is_valid());
    }

    #[test]
    fn oscillator_envelope_verifies_against_typed_result_payload() {
        let raw = include_str!("../fixtures/harmonic-oscillator.json");
        let spec: HarmonicOscillatorSpec = serde_json::from_str(raw).unwrap();
        let result = simulate_harmonic_oscillator(&spec).unwrap();
        let receipt = RunReceipt::build(
            "oscillator",
            raw.as_bytes(),
            &spec,
            &result,
            Some(spec.id.clone()),
            Some(spec.schema_version.clone()),
            None,
            settings(&[
                ("integrator", format!("{:?}", spec.integrator)),
                ("time_step_s", spec.time_step_s.to_string()),
                ("duration_s", spec.duration_s.to_string()),
            ]),
        ).unwrap();
        let envelope = serde_json::json!({"result": &result, "run_receipt": &receipt});
        assert!(verify(&receipt, raw, &spec, &envelope).is_valid());
    }

    #[test]
    fn convergence_envelope_verifies_typed_report_and_step_sequence() {
        let raw = include_str!("../fixtures/harmonic-oscillator.json");
        let spec: HarmonicOscillatorSpec = serde_json::from_str(raw).unwrap();
        let steps = vec![0.04, 0.02, 0.01, 0.005];
        let report = analyze_harmonic_oscillator_convergence(&spec, &steps).unwrap();
        let step_csv = "0.04,0.02,0.01,0.005";
        let receipt = RunReceipt::build(
            "oscillator-convergence",
            raw.as_bytes(),
            &spec,
            &report,
            Some(spec.id.clone()),
            Some(spec.schema_version.clone()),
            None,
            settings(&[
                ("integrator", format!("{:?}", spec.integrator)),
                ("coarse_to_fine_time_steps_s", step_csv.to_owned()),
                ("duration_s", spec.duration_s.to_string()),
            ]),
        ).unwrap();
        let envelope = serde_json::json!({"report": &report, "run_receipt": &receipt});
        assert!(verify(&receipt, raw, &spec, &envelope).is_valid());
    }
}

