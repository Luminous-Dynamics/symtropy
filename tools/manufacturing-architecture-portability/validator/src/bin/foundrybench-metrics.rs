// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline command-line evaluator for FoundryBench's synthetic metric fixtures.

use mfg_architecture_profile_validator::bench_metrics::{
    DiagnosisDisposition, FeatureSubsetRun, RunPartitionManifest, TepOnsetRecord,
    evaluate_binary_calibration, evaluate_tep_onset_horizon, inspect_run_partition_leakage,
    measure_feature_subset_stability, BinaryCalibrationObservation,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, env, fs, path::PathBuf, process::ExitCode};

fn read_json(path: &PathBuf) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: invalid JSON: {error}", path.display()))
}

fn validate_schema(schema: &Value, value: &Value) -> Result<(), Vec<String>> {
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err(vec!["expected JSON Schema Draft 2020-12".to_owned()]);
    }
    jsonschema::draft202012::meta::validate(schema)
        .map_err(|error| vec![format!("invalid JSON Schema: {error}")])?;
    let validator = jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| vec![format!("cannot build input validator: {error}")])?;
    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn string_set(value: &Value, field: &str) -> Result<BTreeSet<String>, String> {
    let values = value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field}: expected array"))?;
    let strings: Vec<String> = values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{field}: expected string entries"))
        })
        .collect::<Result<_, _>>()?;
    let set: BTreeSet<String> = strings.iter().cloned().collect();
    if set.len() != strings.len() {
        return Err(format!("{field}: duplicate entries are not permitted"));
    }
    Ok(set)
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{field}: expected string"))
}

fn required_u64(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{field}: expected unsigned integer"))
}

fn nullable_u64(value: &Value, field: &str) -> Result<Option<u64>, String> {
    match value.get(field) {
        Some(Value::Null) => Ok(None),
        Some(Value::Number(number)) => number
            .as_u64()
            .map(Some)
            .ok_or_else(|| format!("{field}: expected unsigned integer or null")),
        _ => Err(format!("{field}: expected unsigned integer or null")),
    }
}

fn diagnosis(value: &Value) -> Result<DiagnosisDisposition, String> {
    match value.as_str() {
        Some("correct") => Ok(DiagnosisDisposition::Correct),
        Some("confident_wrong") => Ok(DiagnosisDisposition::ConfidentWrong),
        Some("safe_deferral") => Ok(DiagnosisDisposition::SafeDeferral),
        Some("not_attempted") => Ok(DiagnosisDisposition::NotAttempted),
        Some("not_applicable") => Ok(DiagnosisDisposition::NotApplicable),
        _ => Err("diagnosis: unsupported disposition".to_owned()),
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  foundrybench-metrics feature-stability <input.schema.json> <manifest.json>\n  foundrybench-metrics tep-onset <input.schema.json> <manifest.json>\n  foundrybench-metrics calibration <input.schema.json> <manifest.json>\n  foundrybench-metrics run-partitions <input.schema.json> <manifest.json>"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        return usage();
    }
    let command = args[0].as_str();
    let schema_path = PathBuf::from(&args[1]);
    let manifest_path = PathBuf::from(&args[2]);

    let schema = match read_json(&schema_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let manifest = match read_json(&manifest_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(errors) = validate_schema(&schema, &manifest) {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "schema_valid": false,
                "errors": errors
            }))
            .expect("schema error report serializes")
        );
        return ExitCode::FAILURE;
    }

    match command {
        "feature-stability" => {
            let universe = match string_set(&manifest, "feature_universe") {
                Ok(values) => values,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let run_values = manifest
                .get("runs")
                .and_then(Value::as_array)
                .expect("schema requires runs array");
            let mut runs = Vec::with_capacity(run_values.len());
            for run in run_values {
                let run_id = match required_string(run, "run_id") {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                let selected = match string_set(run, "selected_features") {
                    Ok(values) => values,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                runs.push(FeatureSubsetRun {
                    run_id,
                    selected_features: selected.into_iter().collect(),
                });
            }

            match measure_feature_subset_stability(&universe, &runs) {
                Ok(report) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "schema_valid": true,
                            "run_count": report.run_count,
                            "pair_count": report.pair_count,
                            "universe_size": report.universe_size,
                            "mean_selected_feature_count": report.mean_selected_feature_count,
                            "selected_feature_count_stddev": report.selected_feature_count_stddev,
                            "mean_jaccard": report.mean_jaccard,
                            "median_jaccard": report.median_jaccard,
                            "p10_jaccard": report.p10_jaccard,
                            "minimum_jaccard": report.minimum_jaccard,
                            "kuncheva_pair_count": report.kuncheva_pair_count,
                            "mean_kuncheva": report.mean_kuncheva,
                            "claim_ceiling": "synthetic feature-set metric only; no selector was trained"
                        }))
                        .expect("feature stability report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        "tep-onset" => {
            let horizon = match required_u64(&manifest, "evaluation_horizon_ns") {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let record_values = manifest
                .get("records")
                .and_then(Value::as_array)
                .expect("schema requires records array");
            let mut records = Vec::with_capacity(record_values.len());
            for record in record_values {
                let run_id = match required_string(record, "run_id") {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                let scenario_id = match record.get("scenario_id") {
                    Some(Value::Null) => None,
                    Some(Value::Number(number)) => match number.as_u64() {
                        Some(value) => match u8::try_from(value) {
                            Ok(value) => Some(value),
                            Err(_) => {
                                eprintln!("scenario_id: value exceeds u8");
                                return ExitCode::FAILURE;
                            }
                        },
                        None => {
                            eprintln!("scenario_id: expected unsigned integer or null");
                            return ExitCode::FAILURE;
                        }
                    },
                    _ => {
                        eprintln!("scenario_id: expected unsigned integer or null");
                        return ExitCode::FAILURE;
                    }
                };
                let detection_delay_ns = match nullable_u64(record, "detection_delay_ns") {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                let disposition = match record.get("diagnosis").map(diagnosis) {
                    Some(Ok(value)) => value,
                    Some(Err(error)) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                    None => {
                        eprintln!("diagnosis: missing field");
                        return ExitCode::FAILURE;
                    }
                };
                let false_alarms = match required_u64(record, "normal_false_alarm_count") {
                    Ok(value) => match u32::try_from(value) {
                        Ok(value) => value,
                        Err(_) => {
                            eprintln!("normal_false_alarm_count: value exceeds u32");
                            return ExitCode::FAILURE;
                        }
                    },
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                let exposure = match required_u64(record, "normal_exposure_ns") {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                records.push(TepOnsetRecord {
                    run_id,
                    scenario_id,
                    detection_delay_ns,
                    diagnosis: disposition,
                    normal_false_alarm_count: false_alarms,
                    normal_exposure_ns: exposure,
                });
            }

            match evaluate_tep_onset_horizon(&records, horizon) {
                Ok(report) => {
                    let scenarios: Vec<Value> = report
                        .scenarios
                        .iter()
                        .map(|scenario| {
                            json!({
                                "scenario_id": scenario.scenario_id,
                                "fault_run_count": scenario.fault_run_count,
                                "detected_run_count": scenario.detected_run_count,
                                "detection_recall": scenario.detection_recall,
                                "confident_wrong_count": scenario.confident_wrong_count,
                                "safe_deferral_count": scenario.safe_deferral_count,
                                "diagnosis_coverage": scenario.diagnosis_coverage,
                                "selective_diagnosis_accuracy": scenario.selective_diagnosis_accuracy,
                                "mean_detection_delay_ns": scenario.mean_detection_delay_ns
                            })
                        })
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "schema_valid": true,
                            "evaluation_horizon_ns": report.evaluation_horizon_ns,
                            "fault_run_count": report.fault_run_count,
                            "detected_fault_run_count": report.detected_fault_run_count,
                            "detection_recall": report.detection_recall,
                            "confident_wrong_count": report.confident_wrong_count,
                            "confident_wrong_rate_detected": report.confident_wrong_rate_detected,
                            "safe_deferral_count": report.safe_deferral_count,
                            "safe_deferral_rate_detected": report.safe_deferral_rate_detected,
                            "diagnosis_coverage": report.diagnosis_coverage,
                            "selective_diagnosis_accuracy": report.selective_diagnosis_accuracy,
                            "mean_detection_delay_ns": report.mean_detection_delay_ns,
                            "normal_false_alarm_count": report.normal_false_alarm_count,
                            "normal_exposure_hours": report.normal_exposure_hours,
                            "false_alarm_rate_per_normal_hour": report.false_alarm_rate_per_normal_hour,
                            "scenarios": scenarios,
                            "claim_ceiling": "synthetic onset metric only; adjudication labels are supplied, no TEP model or dataset is evaluated"
                        }))
                        .expect("TEP onset report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        "calibration" => {
            let bin_count = match required_u64(&manifest, "bin_count") {
                Ok(value) => match usize::try_from(value) {
                    Ok(value) => value,
                    Err(_) => {
                        eprintln!("bin_count: value exceeds usize");
                        return ExitCode::FAILURE;
                    }
                },
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let observation_values = manifest
                .get("observations")
                .and_then(Value::as_array)
                .expect("schema requires observations array");
            let mut observations = Vec::with_capacity(observation_values.len());
            for observation in observation_values {
                let sample_id = match required_string(observation, "sample_id") {
                    Ok(value) => value,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                };
                let actual_positive = match observation.get("actual_positive").and_then(Value::as_bool) {
                    Some(value) => value,
                    None => {
                        eprintln!("actual_positive: expected boolean");
                        return ExitCode::FAILURE;
                    }
                };
                let predicted_probability = match observation.get("predicted_probability").and_then(Value::as_f64) {
                    Some(value) => value,
                    None => {
                        eprintln!("predicted_probability: expected number");
                        return ExitCode::FAILURE;
                    }
                };
                observations.push(BinaryCalibrationObservation {
                    sample_id,
                    actual_positive,
                    predicted_probability,
                });
            }

            match evaluate_binary_calibration(&observations, bin_count) {
                Ok(report) => {
                    let bins: Vec<Value> = report.bins
                        .iter()
                        .map(|bin| json!({
                            "lower_bound": bin.lower_bound,
                            "upper_bound": bin.upper_bound,
                            "sample_count": bin.sample_count,
                            "mean_predicted_probability": bin.mean_predicted_probability,
                            "observed_positive_rate": bin.observed_positive_rate
                        }))
                        .collect();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "schema_valid": true,
                            "observation_count": report.observation_count,
                            "brier_score": report.brier_score,
                            "negative_log_likelihood": report.negative_log_likelihood,
                            "expected_calibration_error": report.expected_calibration_error,
                            "maximum_calibration_error": report.maximum_calibration_error,
                            "bins": bins,
                            "claim_ceiling": "synthetic calibration metric only; no trained model or real observations are evaluated"
                        }))
                        .expect("calibration report serializes")
                    );
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        "run-partitions" => {
            let partitions = RunPartitionManifest {
                training_run_ids: match string_set(&manifest, "training_run_ids") {
                    Ok(values) => values,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                },
                validation_run_ids: match string_set(&manifest, "validation_run_ids") {
                    Ok(values) => values,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                },
                test_run_ids: match string_set(&manifest, "test_run_ids") {
                    Ok(values) => values,
                    Err(error) => {
                        eprintln!("{error}");
                        return ExitCode::FAILURE;
                    }
                },
            };
            match inspect_run_partition_leakage(&partitions) {
                Ok(leakage) => {
                    let leakage_free = leakage.is_empty();
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "schema_valid": true,
                            "leakage_free": leakage_free,
                            "train_validation_overlap": leakage.train_validation,
                            "train_test_overlap": leakage.train_test,
                            "validation_test_overlap": leakage.validation_test,
                            "claim_ceiling": "run-ID partition check only; this does not detect leakage hidden by incorrect source run identities"
                        }))
                        .expect("partition report serializes")
                    );
                    if leakage_free {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(error) => {
                    eprintln!("{error}");
                    ExitCode::FAILURE
                }
            }
        }
        _ => usage(),
    }
}
