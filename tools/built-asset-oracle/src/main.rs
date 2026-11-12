// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CaseResult {
    case_id: String,
    disposition: String,
    reason_codes: Vec<String>,
    expected_disposition: Option<String>,
    expected_reason_codes: Vec<String>,
    matches_expected: bool,
}

#[derive(Debug, Serialize)]
struct CorpusResult {
    corpus_id: String,
    schema_version: String,
    corpus_sha256: String,
    case_count: usize,
    matched_expected_cases: usize,
    mismatched_cases: usize,
    passed: bool,
    claim_ceiling: String,
    cases: Vec<CaseResult>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str()
}

fn number_at(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_f64()
}

fn string_array_at<'a>(value: &'a Value, path: &[&str]) -> Option<Vec<&'a str>> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    let values = current.as_array()?;
    values.iter().map(Value::as_str).collect()
}

fn result(
    case_id: &str,
    disposition: &str,
    reasons: &[&str],
    expected: Option<&str>,
) -> CaseResult {
    CaseResult {
        case_id: case_id.to_owned(),
        disposition: disposition.to_owned(),
        reason_codes: reasons.iter().map(|reason| (*reason).to_owned()).collect(),
        expected_disposition: expected.map(str::to_owned),
        expected_reason_codes: Vec::new(),
        matches_expected: expected == Some(disposition),
    }
}

/// Evaluate candidate observations against the declared synthetic subject.
///
/// Deliberately does not read sealed_reference_truth or expected_oracle.
/// The evaluator only checks declared evidence context, observation coverage,
/// source independence, and explicitly supplied intervention/functional-test
/// outcomes. This is a finite contract oracle, not a perception model or
/// engineering solver.
fn evaluate_case(case: &Value, reference: &Value) -> CaseResult {
    let case_id = case
        .get("case_id")
        .and_then(Value::as_str)
        .unwrap_or("MISSING_CASE_ID");
    let expected = string_at(case, &["expected_oracle", "disposition"]);
    let Some(input) = case.get("candidate_input") else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["CANDIDATE_INPUT_MISSING"],
            expected,
        );
    };

    let Some(asset_id) = string_at(input, &["asset_id"]) else {
        return result(case_id, "INVALID_INPUT", &["ASSET_ID_MISSING"], expected);
    };
    let expected_asset = match (
        reference.get("building_id").and_then(Value::as_str),
        reference.get("assembly_id").and_then(Value::as_str),
    ) {
        (Some(building), Some(assembly)) => format!("{building}/{assembly}"),
        _ => {
            return result(
                case_id,
                "INVALID_INPUT",
                &["REFERENCE_ASSET_ID_MISSING"],
                expected,
            );
        }
    };
    if asset_id != expected_asset {
        return result(case_id, "INVALID_INPUT", &["ASSET_ID_MISMATCH"], expected);
    }

    let Some(unit) = string_at(input, &["unit"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["REQUIRED_UNIT_FIELD_MISSING_OR_NULL"],
            expected,
        );
    };
    if unit.is_empty() {
        return result(
            case_id,
            "INVALID_INPUT",
            &["REQUIRED_UNIT_FIELD_MISSING_OR_NULL"],
            expected,
        );
    }
    if let Some(observation_unit) = string_at(input, &["finding", "value", "unit"])
        && observation_unit != unit
    {
        return result(
            case_id,
            "INVALID_INPUT",
            &["INCONSISTENT_QUANTITY_UNITS", "NO_IMPLICIT_UNIT_CONVERSION"],
            expected,
        );
    }

    let Some(frame_id) = string_at(input, &["frame_id"]) else {
        return result(case_id, "INVALID_INPUT", &["FRAME_ID_MISSING"], expected);
    };
    if Some(frame_id) != reference.get("coordinate_frame").and_then(Value::as_str) {
        return result(
            case_id,
            "INVALID_INPUT",
            &["FRAME_ID_MISMATCH", "GEOMETRY_COMPARISON_BLOCKED"],
            expected,
        );
    }

    let Some(calibration_id) = string_at(input, &["calibration_id"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["CALIBRATION_ID_MISSING"],
            expected,
        );
    };
    let expected_calibration = reference
        .get("expected_calibration_id")
        .and_then(Value::as_str);
    let calibration_state = string_at(input, &["calibration_state"]);
    if calibration_state != Some("valid_for_fixture")
        || expected_calibration.is_some_and(|known| calibration_id != known)
    {
        return result(
            case_id,
            "STALE_EVIDENCE",
            &[
                "CALIBRATION_NOT_VALID_FOR_CAPTURE",
                "OBSERVATION_NOT_ADMISSIBLE_FOR_CLAIM",
            ],
            expected,
        );
    }

    let Some(design_revision) = string_at(input, &["design_revision"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["DESIGN_REVISION_MISSING"],
            expected,
        );
    };
    if Some(design_revision) != reference.get("design_revision").and_then(Value::as_str) {
        return result(
            case_id,
            "STALE_EVIDENCE",
            &["DESIGN_REVISION_SUPERSEDED", "REOBSERVATION_REQUIRED"],
            expected,
        );
    }

    let Some(observed_regions) = string_array_at(input, &["observed_regions"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["OBSERVED_REGIONS_MISSING_OR_INVALID"],
            expected,
        );
    };
    let Some(required_regions) = string_array_at(reference, &["required_regions"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["REFERENCE_REQUIRED_REGIONS_INVALID"],
            expected,
        );
    };
    let observed: BTreeSet<&str> = observed_regions.into_iter().collect();
    if required_regions
        .iter()
        .any(|region| !observed.contains(region))
    {
        return result(
            case_id,
            "UNKNOWN_REQUIRED_COVERAGE",
            &[
                "REQUIRED_SEAL_REGION_UNOBSERVED",
                "NO_FALSE_CLEAR_FROM_MISSING_OBSERVATION",
            ],
            expected,
        );
    }

    let Some(lineages) = string_array_at(input, &["sensor_lineages"]) else {
        return result(
            case_id,
            "INVALID_INPUT",
            &["SENSOR_LINEAGE_MISSING_OR_INVALID"],
            expected,
        );
    };
    let unique_lineages: BTreeSet<&str> = lineages.iter().copied().collect();
    if unique_lineages.len() != lineages.len() {
        return result(
            case_id,
            "INSUFFICIENT_INDEPENDENT_EVIDENCE",
            &[
                "DUPLICATE_UPSTREAM_SENSOR_LINEAGE",
                "CORROBORATION_NOT_INDEPENDENT",
            ],
            expected,
        );
    }

    if let Some(measurement_value) = input.get("independent_measurements") {
        let Some(measurements) = measurement_value.as_array() else {
            return result(
                case_id,
                "INVALID_INPUT",
                &["INDEPENDENT_MEASUREMENTS_MUST_BE_ARRAY"],
                expected,
            );
        };
        let offsets: Option<Vec<f64>> = measurements
            .iter()
            .map(|measurement| measurement.get("offset_mm").and_then(Value::as_f64))
            .collect();
        let Some(offsets) = offsets else {
            return result(
                case_id,
                "INVALID_INPUT",
                &["INDEPENDENT_MEASUREMENT_OFFSET_MISSING_OR_INVALID"],
                expected,
            );
        };
        if let (Some(minimum), Some(maximum)) = (
            offsets.iter().copied().reduce(f64::min),
            offsets.iter().copied().reduce(f64::max),
        ) {
            let Some(threshold) = reference
                .get("reference_conflict_threshold_mm")
                .and_then(Value::as_f64)
            else {
                return result(
                    case_id,
                    "INVALID_INPUT",
                    &["REFERENCE_CONFLICT_THRESHOLD_MISSING_OR_INVALID"],
                    expected,
                );
            };
            if maximum - minimum > threshold {
                return result(
                    case_id,
                    "CONFLICTED_EVIDENCE",
                    &[
                        "INDEPENDENT_REFERENCE_MEASUREMENTS_CONFLICT",
                        "ADJUDICATION_REQUIRED",
                    ],
                    expected,
                );
            }
        }
    }

    if let Some(material_class) = input.get("material_class").and_then(Value::as_str) {
        let supported =
            string_array_at(reference, &["supported_material_classes"]).unwrap_or_default();
        if !supported.contains(&material_class) {
            return result(
                case_id,
                "OUT_OF_DOMAIN",
                &[
                    "MATERIAL_CLASS_NOT_IN_VALIDATED_PROFILE",
                    "MODEL_SCORE_DOES_NOT_EXTEND_CLAIM_SCOPE",
                ],
                expected,
            );
        }
    }

    if let Some(repair) = input.get("repair") {
        let authorization = repair.get("human_authorization").and_then(Value::as_str);
        if authorization != Some("present") {
            return result(
                case_id,
                "INTERVENTION_NOT_AUTHORIZED",
                &[
                    "REQUIRED_HUMAN_AUTHORIZATION_ABSENT",
                    "NO_ACTUATION_OR_WORK_RELEASE",
                ],
                expected,
            );
        }
        let functional_result = string_at(repair, &["independent_functional_test", "result"]);
        match functional_result {
            Some("fail") => {
                return result(
                    case_id,
                    "REPAIR_REJECTED_FUNCTIONAL_TEST",
                    &[
                        "INDEPENDENT_FUNCTIONAL_TEST_FAILED",
                        "TASK_COMPLETION_NOT_ACCEPTANCE",
                    ],
                    expected,
                );
            }
            Some("pass") => {
                return result(
                    case_id,
                    "REPAIR_ACCEPTED_FOR_DECLARED_NONSTRUCTURAL_FUNCTION",
                    &[
                        "INDEPENDENT_FUNCTIONAL_TEST_PASSED",
                        "FUNCTION_NOT_VISUAL_IDENTITY_IS_ACCEPTANCE_BASIS",
                    ],
                    expected,
                );
            }
            _ => {
                return result(
                    case_id,
                    "NOT_EVALUATED",
                    &["INDEPENDENT_FUNCTIONAL_TEST_MISSING_OR_UNRECOGNIZED"],
                    expected,
                );
            }
        }
    }

    match string_at(input, &["finding", "kind"]) {
        Some("panel_displacement") => {
            let offset = number_at(input, &["finding", "value", "offset_mm"]);
            let tolerance = number_at(input, &["finding", "value", "tolerance_mm"]);
            match (offset, tolerance) {
                (Some(measured), Some(limit)) if measured > limit => result(
                    case_id,
                    "DEFECT_OBSERVED",
                    &["OBSERVED_GEOMETRY_OUTSIDE_DECLARED_SYNTHETIC_TOLERANCE"],
                    expected,
                ),
                (Some(_), Some(_)) => result(
                    case_id,
                    "ACCEPTED_FOR_DECLARED_SYNTHETIC_CHECK",
                    &["OBSERVED_GEOMETRY_WITHIN_DECLARED_SYNTHETIC_TOLERANCE"],
                    expected,
                ),
                _ => result(
                    case_id,
                    "NOT_EVALUATED",
                    &["PANEL_DISPLACEMENT_MEASUREMENT_OR_TOLERANCE_MISSING"],
                    expected,
                ),
            }
        }
        Some("missing_fastener") => result(
            case_id,
            "DEFECT_OBSERVED",
            &["FASTENER_ABSENCE_OBSERVED_AT_REQUIRED_LOCATION"],
            expected,
        ),
        Some("candidate_seal_discontinuity" | "candidate_panel_displacement") => result(
            case_id,
            "CANDIDATE_NOT_CONFIRMED",
            &[
                "LOOKALIKE_REQUIRES_ADDITIONAL_EVIDENCE",
                "NO_CONFIRMED_FUNCTIONAL_DEFECT",
            ],
            expected,
        ),
        Some("none") => result(
            case_id,
            "ACCEPTED_FOR_DECLARED_SYNTHETIC_CHECK",
            &["REQUIRED_COVERAGE_COMPLETE", "EVIDENCE_CONTEXT_CURRENT"],
            expected,
        ),
        _ => result(
            case_id,
            "OUT_OF_DOMAIN",
            &["FINDING_KIND_UNSUPPORTED_OR_MISSING"],
            expected,
        ),
    }
}

fn evaluate_corpus(bytes: &[u8]) -> Result<CorpusResult, Box<dyn Error>> {
    let corpus: Value = serde_json::from_slice(bytes)?;
    let corpus_id = corpus
        .get("corpus_id")
        .and_then(Value::as_str)
        .ok_or("corpus_id missing")?
        .to_owned();
    let schema_version = corpus
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or("schema_version missing")?
        .to_owned();
    let claim_ceiling = corpus
        .get("claim_ceiling")
        .and_then(Value::as_str)
        .ok_or("claim_ceiling missing")?
        .to_owned();
    let reference = corpus
        .get("reference_subject")
        .ok_or("reference_subject missing")?;
    let cases = corpus
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("cases must be an array")?;

    let mut case_ids = BTreeSet::new();
    for case in cases {
        let Some(case_id) = case.get("case_id").and_then(Value::as_str) else {
            return Err("case_id missing or invalid".into());
        };
        if case_id.is_empty() || !case_ids.insert(case_id) {
            return Err(format!("case_id empty or duplicated: {case_id}").into());
        }
    }
    let mut results: Vec<CaseResult> = cases
        .iter()
        .map(|case| evaluate_case(case, reference))
        .collect();
    for (evaluated, source_case) in results.iter_mut().zip(cases.iter()) {
        let expected_codes = source_case
            .get("expected_oracle")
            .and_then(|expected| expected.get("reason_codes"))
            .and_then(Value::as_array)
            .map(|codes| {
                codes
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        evaluated.matches_expected &= evaluated.reason_codes == expected_codes;
        evaluated.expected_reason_codes = expected_codes;
    }
    let matched_expected_cases = results.iter().filter(|case| case.matches_expected).count();
    let mismatched_cases = results.len() - matched_expected_cases;

    Ok(CorpusResult {
        corpus_id,
        schema_version,
        corpus_sha256: sha256_hex(bytes),
        case_count: results.len(),
        matched_expected_cases,
        mismatched_cases,
        passed: !results.is_empty() && mismatched_cases == 0,
        claim_ceiling,
        cases: results,
    })
}

fn parse_args() -> Result<(PathBuf, Option<PathBuf>), Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut corpus = PathBuf::from("docs/research/built-asset/fixtures/v0.1/corpus.json");
    let mut output = None;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--corpus" => {
                corpus = args.next().ok_or("--corpus requires a path")?.into();
            }
            "--output" => {
                output = Some(args.next().ok_or("--output requires a path")?.into());
            }
            "--help" | "-h" => {
                println!(
                    "built-asset-oracle [--corpus PATH] [--output PATH]\n\
                     Defaults to the repository v0.1 corpus and writes JSON to stdout."
                );
                std::process::exit(0);
            }
            unknown => return Err(format!("unknown argument: {unknown}").into()),
        }
    }
    Ok((corpus, output))
}

fn run() -> Result<bool, Box<dyn Error>> {
    let (corpus_path, output_path) = parse_args()?;
    let bytes = fs::read(&corpus_path)?;
    let report = evaluate_corpus(&bytes)?;
    let rendered = serde_json::to_string_pretty(&report)?;

    if let Some(path) = output_path {
        fs::write(path, format!("{rendered}\n"))?;
    } else {
        println!("{rendered}");
    }

    Ok(report.passed)
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("built-asset-oracle: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str =
        include_str!("../../../docs/research/built-asset/fixtures/v0.1/corpus.json");

    #[test]
    fn frozen_corpus_matches_all_declared_dispositions_and_reasons() {
        let report = evaluate_corpus(CORPUS.as_bytes()).expect("corpus parses");
        assert_eq!(report.case_count, 16);
        assert_eq!(report.matched_expected_cases, 16);
        assert_eq!(report.mismatched_cases, 0);
        assert!(report.passed);
        for case in &report.cases {
            assert_eq!(
                case.reason_codes, case.expected_reason_codes,
                "{}",
                case.case_id
            );
        }
    }

    #[test]
    fn observation_order_does_not_change_coverage_disposition() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let original = evaluate_case(case, reference);

        let mut permuted = case.clone();
        permuted["candidate_input"]["observed_regions"]
            .as_array_mut()
            .expect("regions are an array")
            .reverse();
        let changed = evaluate_case(&permuted, reference);
        assert_eq!(original.disposition, changed.disposition);
        assert_eq!(original.reason_codes, changed.reason_codes);
    }

    #[test]
    fn removing_a_required_region_never_becomes_clear() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["observed_regions"]
            .as_array_mut()
            .expect("regions are an array")
            .retain(|region| region.as_str() != Some("SEAL-PERIMETER"));
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "UNKNOWN_REQUIRED_COVERAGE");
    }

    #[test]
    fn stale_calibration_blocks_a_defect_observation() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-002")
            .expect("defect case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["calibration_state"] = Value::String("expired".to_owned());
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "STALE_EVIDENCE");
    }

    #[test]
    fn wrong_coordinate_frame_blocks_geometry_evidence() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["frame_id"] = Value::String("WRONG-FRAME".to_owned());
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "INVALID_INPUT");
    }

    #[test]
    fn stale_design_revision_requires_reobservation() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["design_revision"] = Value::String("D-01".to_owned());
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "STALE_EVIDENCE");
    }

    #[test]
    fn duplicate_sensor_lineage_is_not_independent_corroboration() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["sensor_lineages"] =
            serde_json::json!(["LINEAGE-A", "LINEAGE-A"]);
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "INSUFFICIENT_INDEPENDENT_EVIDENCE");
    }

    #[test]
    fn malformed_independent_measurement_is_rejected() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-001")
            .expect("nominal case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["independent_measurements"] =
            serde_json::json!([{"offset_mm": "not-a-number"}]);
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "INVALID_INPUT");
        assert_eq!(
            evaluated.reason_codes,
            vec!["INDEPENDENT_MEASUREMENT_OFFSET_MISSING_OR_INVALID"]
        );
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let cases = corpus["cases"].as_array_mut().expect("cases exist");
        let first_id = cases[0]["case_id"].clone();
        cases[1]["case_id"] = first_id;
        let bytes = serde_json::to_vec(&corpus).expect("corpus serializes");
        assert!(evaluate_corpus(&bytes).is_err());
    }

    #[test]
    fn failed_functional_test_rejects_a_visually_complete_repair() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("corpus parses");
        let reference = corpus.get("reference_subject").expect("reference exists");
        let case = corpus["cases"]
            .as_array()
            .expect("cases exist")
            .iter()
            .find(|case| case["case_id"] == "BA-012")
            .expect("repair case exists");
        let mut mutated = case.clone();
        mutated["candidate_input"]["repair"]["independent_functional_test"]["result"] =
            Value::String("fail".to_owned());
        let evaluated = evaluate_case(&mutated, reference);
        assert_eq!(evaluated.disposition, "REPAIR_REJECTED_FUNCTIONAL_TEST");
    }
}
