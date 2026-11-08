// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Import the original HumanStudy-Bench paper runner's native per-study files.
//!
//! This is an adapter, not a second scorer. PAS/ECS are copied from the exact
//! upstream evaluation JSON. Raw native bytes are hashed before parsing.
//! Source execution is bound to an operator-authored attestation sidecar; the
//! adapter validates the sidecar against the frozen source pins but cannot
//! prove that the external process actually ran those sources.

use super::{
    ArtifactReceipt, BenchmarkRunReceipt, EffectEstimate, IndependentUnit,
    PAPER_12_METRIC_CONTRACT_ID, PAPER_12_PROTOCOL_ID, PAPER_12_SOURCE_COMMIT,
    PAPER_12_SOURCE_REPOSITORY, PAPER_12_SOURCE_ROOT_TREE, PAPER_12_STUDY_PINS, StudyReceipt,
    is_lower_hex, validate_run,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

const ATTESTATION_FILE: &str = "civdyn_paper_run_attestation.json";
const FULL_BENCHMARK_FILE: &str = "full_benchmark.json";
const DETAILED_STATS_FILE: &str = "detailed_stats.csv";
const EVALUATION_FILE: &str = "evaluation_results.json";
const RAW_RESPONSES_FILE: &str = "raw_responses.json";

const DETAILED_STATS_HEADER: &str = "Study_ID,Sub_Study_ID,Finding_ID,Test_Name,Scenario,Statistical_Test_Type,Human_Test_Statistic,Agent_Test_Statistic,Z_Diff,ECS_Test,Agent_Effect_Size,Human_Effect_Size,Agent_Effect_d,Human_Effect_d";

/// SHA-256 of the checked-in frozen source-pin manifest, not of a re-created
/// JSON object. It includes global runner/scorer pins and per-study tree/config
/// and evaluator identities.
pub fn paper_execution_manifest_sha256() -> String {
    sha256_hex(include_bytes!("../foundational_12_profile.json"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaperRunAttestation {
    pub protocol_id: String,
    pub source_repository: String,
    pub source_commit: String,
    pub study_id: String,
    pub study_config_blob_sha: String,
    pub evaluator_blob_sha: String,
    pub baseline_runner_blob_sha: String,
    pub generation_runner_blob_sha: String,
    pub generation_pipeline_blob_sha: String,
    pub evaluator_runner_blob_sha: String,
    pub stats_lib_blob_sha: String,
    /// Operator attestation that the pinned analysis implementation was not
    /// substituted. The source blob checks below validate the declaration;
    /// this boolean alone is not execution proof.
    pub analysis_protocol_preserved: bool,
    pub run_id: String,
    pub model_id: String,
    pub prompt_preset: String,
    /// SHA-256 of the exact resolved prompt-template source bytes used by the
    /// run wrapper. The importer validates its format and records it; the
    /// wrapper remains responsible for computing it from the pinned file.
    pub prompt_template_sha256: String,
    pub independent_unit: IndependentUnit,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaperStudyImport {
    pub receipt: StudyReceipt,
    pub run_id: String,
    pub model_id: String,
    pub prompt_preset: String,
    pub test_count: usize,
    pub run_count: usize,
    pub usage_stats: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperImportError {
    pub message: String,
}

impl PaperImportError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PaperImportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PaperImportError {}

#[derive(Clone, Copy)]
struct PaperStudyExecutionPin {
    study_id: &'static str,
    config_blob_sha: &'static str,
    evaluator_blob_sha: &'static str,
}

const PAPER_STUDY_EXECUTION_PINS: [PaperStudyExecutionPin; 12] = [
    PaperStudyExecutionPin {
        study_id: "study_001",
        config_blob_sha: "63fe04388eccca7088ba8e76834014511797e2bd",
        evaluator_blob_sha: "612428dac96758f7d3e68fe8882133af13be5242",
    },
    PaperStudyExecutionPin {
        study_id: "study_002",
        config_blob_sha: "77cfac304a8f376fd3cd475f4470b0fa8c2dfba3",
        evaluator_blob_sha: "f8eff4c480813e675be2b72bb88cbbdd77f6014b",
    },
    PaperStudyExecutionPin {
        study_id: "study_003",
        config_blob_sha: "9b5a519d33663d3a9ab60d7720478131d211adb1",
        evaluator_blob_sha: "b242e6c3c3d108ebef8f7ba9271c43c22e211713",
    },
    PaperStudyExecutionPin {
        study_id: "study_004",
        config_blob_sha: "b644f2a6a1088a4c6b18f8bfe24e3a2e6c602ea4",
        evaluator_blob_sha: "3c42b0302401f80c1991fc61fdc86bd71ac9f8d9",
    },
    PaperStudyExecutionPin {
        study_id: "study_005",
        config_blob_sha: "1f780a80d98e8c4095f483881c35e561716f8496",
        evaluator_blob_sha: "ba4897043e3cd7b18d2ceaa7fa7711d57a1e9991",
    },
    PaperStudyExecutionPin {
        study_id: "study_006",
        config_blob_sha: "807c7b4c64e803cba5b21db1f3bfa59fb29203d3",
        evaluator_blob_sha: "4f76cbec3111e89c9cf4521342ca8a9ca0afc0a6",
    },
    PaperStudyExecutionPin {
        study_id: "study_007",
        config_blob_sha: "62bbfbab3416502c8db9f501510d641ae40ce5d5",
        evaluator_blob_sha: "8db9bb70b85961733756caf4a949252a281bd0de",
    },
    PaperStudyExecutionPin {
        study_id: "study_008",
        config_blob_sha: "89d466998bba5d7b32f02f0e725ffe0ce66bdce6",
        evaluator_blob_sha: "4afe5389289a76d048ca568a5dbddc856c04db1e",
    },
    PaperStudyExecutionPin {
        study_id: "study_009",
        config_blob_sha: "e315825882da6756fdd81abab8fd8a3ea55df852",
        evaluator_blob_sha: "6026055161eeb345efc7c6bce9ee0182a46f9e9c",
    },
    PaperStudyExecutionPin {
        study_id: "study_010",
        config_blob_sha: "7f3269e999afc7f9a2777844ef430ae1a0c21308",
        evaluator_blob_sha: "6d01030daa5388b38a8154d91527c3ad4d1c0b2e",
    },
    PaperStudyExecutionPin {
        study_id: "study_011",
        config_blob_sha: "acf7c3e8f58218716a4d3959d81191eb40f3e287",
        evaluator_blob_sha: "8204cef15b8ab1b2f1f5abaf330a36c8955c4cb3",
    },
    PaperStudyExecutionPin {
        study_id: "study_012",
        config_blob_sha: "5f462363823526e5e77af52a142f098b1831c356",
        evaluator_blob_sha: "2af9777975c68978c0f27f8dcb6da62857218797",
    },
];

fn json_string<'a>(value: &'a Value, key: &str) -> Result<&'a str, PaperImportError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| PaperImportError::new(format!("missing or invalid string field: {key}")))
}

fn json_number(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn read_native_file(directory: &Path, name: &str) -> Result<Vec<u8>, PaperImportError> {
    let path = directory.join(name);
    fs::read(&path).map_err(|error| {
        PaperImportError::new(format!(
            "cannot read required native artifact {}: {error}",
            path.display()
        ))
    })
}

fn parse_json(bytes: &[u8], filename: &str) -> Result<Value, PaperImportError> {
    serde_json::from_slice(bytes)
        .map_err(|error| PaperImportError::new(format!("{filename}: invalid JSON: {error}")))
}

fn expected_execution_source_sha(path: &str) -> Result<String, PaperImportError> {
    let profile = parse_json(
        include_bytes!("../foundational_12_profile.json"),
        "foundational_12_profile.json",
    )?;
    let sources = profile
        .get("execution_sources")
        .ok_or_else(|| PaperImportError::new("paper profile is missing execution_sources"))?;
    Ok(json_string(sources, path)?.to_string())
}

fn parse_independent_unit(value: &str) -> Result<IndependentUnit, PaperImportError> {
    match value {
        "participant" => Ok(IndependentUnit::Participant),
        "group" => Ok(IndependentUnit::Group),
        "session" => Ok(IndependentUnit::Session),
        "cluster" => Ok(IndependentUnit::Cluster),
        other => Err(PaperImportError::new(format!(
            "unsupported independent_unit: {other}"
        ))),
    }
}

fn execution_pin(
    study_id: &str,
) -> Result<(PaperStudyExecutionPin, &'static str), PaperImportError> {
    let config = PAPER_STUDY_EXECUTION_PINS
        .iter()
        .copied()
        .find(|pin| pin.study_id == study_id)
        .ok_or_else(|| {
            PaperImportError::new(format!(
                "study is not in the paper-12 source profile: {study_id}"
            ))
        })?;
    let tree = PAPER_12_STUDY_PINS
        .iter()
        .find(|pin| pin.study_id == study_id)
        .map(|pin| pin.tree_sha)
        .ok_or_else(|| {
            PaperImportError::new(format!("study has no pinned data tree: {study_id}"))
        })?;
    Ok((config, tree))
}

fn parse_attestation(bytes: &[u8]) -> Result<PaperRunAttestation, PaperImportError> {
    let json = parse_json(bytes, ATTESTATION_FILE)?;
    let independent_unit = parse_independent_unit(json_string(&json, "independent_unit")?)?;
    Ok(PaperRunAttestation {
        protocol_id: json_string(&json, "protocol_id")?.to_string(),
        source_repository: json_string(&json, "source_repository")?.to_string(),
        source_commit: json_string(&json, "source_commit")?.to_string(),
        study_id: json_string(&json, "study_id")?.to_string(),
        study_config_blob_sha: json_string(&json, "study_config_blob_sha")?.to_string(),
        evaluator_blob_sha: json_string(&json, "evaluator_blob_sha")?.to_string(),
        baseline_runner_blob_sha: json_string(&json, "baseline_runner_blob_sha")?.to_string(),
        generation_runner_blob_sha: json_string(&json, "generation_runner_blob_sha")?.to_string(),
        generation_pipeline_blob_sha: json_string(&json, "generation_pipeline_blob_sha")?
            .to_string(),
        evaluator_runner_blob_sha: json_string(&json, "evaluator_runner_blob_sha")?.to_string(),
        stats_lib_blob_sha: json_string(&json, "stats_lib_blob_sha")?.to_string(),
        analysis_protocol_preserved: json
            .get("analysis_protocol_preserved")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                PaperImportError::new(
                    "missing or invalid boolean field: analysis_protocol_preserved",
                )
            })?,
        run_id: json_string(&json, "run_id")?.to_string(),
        model_id: json_string(&json, "model_id")?.to_string(),
        prompt_preset: json_string(&json, "prompt_preset")?.to_string(),
        prompt_template_sha256: json_string(&json, "prompt_template_sha256")?.to_string(),
        independent_unit,
    })
}

fn validate_attestation(attestation: &PaperRunAttestation) -> Result<String, PaperImportError> {
    if attestation.protocol_id != PAPER_12_PROTOCOL_ID
        || attestation.source_repository != PAPER_12_SOURCE_REPOSITORY
        || attestation.source_commit != PAPER_12_SOURCE_COMMIT
    {
        return Err(PaperImportError::new(
            "run attestation does not match the pinned HumanStudy-Bench paper source",
        ));
    }
    if attestation.run_id.trim().is_empty() || attestation.model_id.trim().is_empty() {
        return Err(PaperImportError::new(
            "run attestation requires non-empty run_id and model_id",
        ));
    }
    if attestation.prompt_preset.trim().is_empty() {
        return Err(PaperImportError::new(
            "run attestation requires a prompt_preset",
        ));
    }
    if !is_lower_hex(&attestation.prompt_template_sha256, 64) {
        return Err(PaperImportError::new(
            "prompt_template_sha256 must be a 64-character lowercase SHA-256",
        ));
    }
    if !attestation.analysis_protocol_preserved {
        return Err(PaperImportError::new(
            "run attestation must explicitly declare analysis_protocol_preserved=true",
        ));
    }
    let (pin, _) = execution_pin(&attestation.study_id)?;
    if attestation.study_config_blob_sha != pin.config_blob_sha
        || attestation.evaluator_blob_sha != pin.evaluator_blob_sha
    {
        return Err(PaperImportError::new(format!(
            "{}: study config/evaluator blob identities do not match the pinned paper source",
            attestation.study_id
        )));
    }

    for (field, path, actual) in [
        (
            "baseline_runner_blob_sha",
            "scripts/run_baseline_pipeline.py",
            &attestation.baseline_runner_blob_sha,
        ),
        (
            "generation_runner_blob_sha",
            "generation_pipeline/run.py",
            &attestation.generation_runner_blob_sha,
        ),
        (
            "generation_pipeline_blob_sha",
            "generation_pipeline/pipeline.py",
            &attestation.generation_pipeline_blob_sha,
        ),
        (
            "evaluator_runner_blob_sha",
            "src/evaluation/evaluator_runner.py",
            &attestation.evaluator_runner_blob_sha,
        ),
        (
            "stats_lib_blob_sha",
            "src/evaluation/stats_lib.py",
            &attestation.stats_lib_blob_sha,
        ),
    ] {
        let expected = expected_execution_source_sha(path)?;
        if actual != &expected {
            return Err(PaperImportError::new(format!(
                "{}: {field} does not match the pinned execution source {path}",
                attestation.study_id
            )));
        }
    }
    Ok(attestation.study_id.clone())
}

fn count_native_trials(full_benchmark: &Value) -> Result<(u64, usize), PaperImportError> {
    let runs = full_benchmark
        .get("all_runs_raw_results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            PaperImportError::new("full_benchmark.json is missing all_runs_raw_results")
        })?;
    if runs.is_empty() {
        return Err(PaperImportError::new(
            "full_benchmark.json contains no completed raw runs",
        ));
    }

    let mut trial_count = 0u64;
    for (run_index, run) in runs.iter().enumerate() {
        let individual = run
            .get("individual_data")
            .and_then(Value::as_array)
            .ok_or_else(|| PaperImportError::new(format!(
                "full_benchmark.json all_runs_raw_results[{run_index}] is missing individual_data"
            )))?;
        for record in individual {
            if let Some(responses) = record.get("responses").and_then(Value::as_array) {
                trial_count = trial_count
                    .checked_add(responses.len() as u64)
                    .ok_or_else(|| PaperImportError::new("trial count overflow"))?;
            } else {
                // Legacy flat format: one response record per item in individual_data.
                trial_count = trial_count
                    .checked_add(1)
                    .ok_or_else(|| PaperImportError::new("trial count overflow"))?;
            }
        }
    }
    if trial_count == 0 {
        return Err(PaperImportError::new(
            "full_benchmark.json contains zero trial responses",
        ));
    }
    Ok((trial_count, runs.len()))
}

fn csv_record_count(bytes: &[u8], filename: &str) -> Result<u64, PaperImportError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| PaperImportError::new(format!("{filename}: not UTF-8: {error}")))?;
    let header = text
        .lines()
        .next()
        .unwrap_or("")
        .trim_start_matches('\u{feff}');
    if header != DETAILED_STATS_HEADER {
        return Err(PaperImportError::new(format!(
            "{filename}: header does not match the pinned 14-column paper output contract"
        )));
    }

    // Count CSV records rather than newline characters, respecting quoted
    // embedded newlines and escaped double-quotes.
    let raw = text.as_bytes();
    let mut in_quotes = false;
    let mut at_field_start = true;
    let mut records = 0u64;
    let mut has_content_in_last_record = false;
    let mut index = 0usize;
    while index < raw.len() {
        let byte = raw[index];
        if in_quotes {
            if byte == b'"' {
                if index + 1 < raw.len() && raw[index + 1] == b'"' {
                    index += 2;
                    has_content_in_last_record = true;
                    continue;
                }
                in_quotes = false;
            }
            has_content_in_last_record = true;
        } else {
            match byte {
                b'"' if at_field_start => {
                    in_quotes = true;
                    at_field_start = false;
                    has_content_in_last_record = true;
                }
                b',' => {
                    at_field_start = true;
                    has_content_in_last_record = true;
                }
                b'\n' => {
                    records = records.saturating_add(1);
                    at_field_start = true;
                    has_content_in_last_record = false;
                }
                b'\r' => {
                    if index + 1 >= raw.len() || raw[index + 1] != b'\n' {
                        records = records.saturating_add(1);
                        at_field_start = true;
                        has_content_in_last_record = false;
                    }
                }
                _ => {
                    at_field_start = false;
                    has_content_in_last_record = true;
                }
            }
        }
        index += 1;
    }
    if in_quotes {
        return Err(PaperImportError::new(format!(
            "{filename}: unterminated quoted CSV field"
        )));
    }
    if has_content_in_last_record {
        records = records.saturating_add(1);
    }
    records
        .checked_sub(1)
        .ok_or_else(|| PaperImportError::new(format!("{filename}: CSV has no header record")))
}

fn optional_ci(test: &Value, key: &str) -> Option<[f64; 2]> {
    let values = test.get(key)?.as_array()?;
    if values.len() != 2 {
        return None;
    }
    let lower = values[0].as_f64()?;
    let upper = values[1].as_f64()?;
    if lower.is_finite() && upper.is_finite() && lower <= upper {
        Some([lower, upper])
    } else {
        None
    }
}

fn extract_effects(test_results: &[Value]) -> Vec<EffectEstimate> {
    let mut effects = Vec::new();
    for (index, test) in test_results.iter().enumerate() {
        let human_estimate = match json_number(test, "human_effect_d") {
            Some(value) => value,
            None => continue,
        };
        let simulated_estimate = match json_number(test, "agent_effect_d") {
            Some(value) => value,
            None => continue,
        };
        let finding_id = match test.get("finding_id").and_then(Value::as_str) {
            Some(value) if !value.trim().is_empty() => value.to_string(),
            _ => continue,
        };
        let test_name = test
            .get("test_name")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unnamed_test");
        let sub_study = test
            .get("sub_study_id")
            .and_then(Value::as_str)
            .unwrap_or("unspecified_sub_study");
        let scenario = test
            .get("scenario")
            .and_then(Value::as_str)
            .unwrap_or("unspecified_scenario");
        effects.push(EffectEstimate {
            finding_id,
            test_id: format!("{sub_study}:{test_name}:{scenario}:{index}"),
            measure: "standardized_effect_size".to_string(),
            unit: "cohen_d".to_string(),
            human_estimate,
            simulated_estimate,
            human_ci95: optional_ci(test, "human_ci95"),
            simulated_ci95: optional_ci(test, "agent_ci95"),
        });
    }
    effects
}

fn official_ecs(evaluation: &Value) -> Option<f64> {
    json_number(evaluation, "ecs_corr_study")
        .or_else(|| json_number(evaluation, "ecs_corr"))
        .or_else(|| {
            evaluation
                .get("ecs_corr")
                .and_then(|value| json_number(value, "ecs_overall"))
        })
}

fn artifact(
    role: &str,
    path: &str,
    bytes: &[u8],
    record_count: u64,
    rights_basis: &str,
) -> ArtifactReceipt {
    ArtifactReceipt {
        role: role.to_string(),
        path: path.to_string(),
        sha256: sha256_hex(bytes),
        record_count,
        contains_record_level_human_data: false,
        published_in_repository: false,
        redistribution_authorized: false,
        rights_basis: rights_basis.to_string(),
    }
}

/// Import one directory containing the paper runner's three native artifacts
/// plus the CIVDYN source-attestation sidecar.
///
/// The sidecar is operator-authored evidence: it identifies the code revision,
/// study config/evaluator Git blobs, prompt template digest and independent unit.
/// The adapter verifies every declared pin, but cannot independently prove
/// which external Python process produced an already-existing output directory.
pub fn import_paper_study_run(run_directory: &Path) -> Result<PaperStudyImport, PaperImportError> {
    let attestation_bytes = read_native_file(run_directory, ATTESTATION_FILE)?;
    let attestation = parse_attestation(&attestation_bytes)?;
    let study_id = validate_attestation(&attestation)?;
    let (_execution_pin, study_tree_sha) = execution_pin(&study_id)?;

    let full_bytes = read_native_file(run_directory, FULL_BENCHMARK_FILE)?;
    let csv_bytes = read_native_file(run_directory, DETAILED_STATS_FILE)?;
    let evaluation_bytes = read_native_file(run_directory, EVALUATION_FILE)?;

    let full = parse_json(&full_bytes, FULL_BENCHMARK_FILE)?;
    let evaluation = parse_json(&evaluation_bytes, EVALUATION_FILE)?;

    let full_study_id = json_string(&full, "study_id")?;
    if full_study_id != study_id {
        return Err(PaperImportError::new(format!(
            "full_benchmark study_id {full_study_id} does not match attested study {study_id}"
        )));
    }
    let full_model = json_string(&full, "model")?;
    if full_model != attestation.model_id {
        return Err(PaperImportError::new(format!(
            "full_benchmark model {full_model} does not match attested model {}",
            attestation.model_id
        )));
    }
    let full_prompt = json_string(&full, "system_prompt_preset")?;
    if full_prompt != attestation.prompt_preset {
        return Err(PaperImportError::new(format!(
            "full_benchmark prompt preset {full_prompt} does not match attested prompt preset {}",
            attestation.prompt_preset
        )));
    }
    if let Some(evaluation_study_id) = evaluation.get("study_id").and_then(Value::as_str)
        && evaluation_study_id != study_id
    {
        return Err(PaperImportError::new(format!(
            "evaluation_results study_id {evaluation_study_id} does not match attested study {study_id}"
        )));
    }

    let official_pas = json_number(&evaluation, "score")
        .filter(|value| (0.0..=1.0).contains(value))
        .ok_or_else(|| {
            PaperImportError::new(
                "evaluation_results.json is missing a finite official PAS score in [0, 1]",
            )
        })?;
    // ECS is optional per study in the upstream implementation (e.g. fewer
    // than three valid effect-size pairs). We preserve it when available.
    let ecs = official_ecs(&evaluation).filter(|value| (-1.0..=1.0).contains(value));

    let participant_count = evaluation
        .get("n_participants")
        .and_then(Value::as_u64)
        .filter(|count| *count > 0)
        .ok_or_else(|| {
            PaperImportError::new("evaluation_results.json is missing positive n_participants")
        })?;
    let (trial_count, run_count) = count_native_trials(&full)?;

    let test_results = evaluation
        .get("test_results")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            PaperImportError::new("evaluation_results.json is missing test_results array")
        })?;
    let csv_rows = csv_record_count(&csv_bytes, DETAILED_STATS_FILE)?;
    if csv_rows != test_results.len() as u64 {
        return Err(PaperImportError::new(format!(
            "detailed_stats.csv has {csv_rows} test rows but evaluation_results.json has {} test_results",
            test_results.len()
        )));
    }

    let mut agent_config = Map::new();
    for key in [
        "model",
        "reasoning",
        "use_real_llm",
        "system_prompt_preset",
        "random_seed",
        "temperature",
        "enable_reasoning",
    ] {
        if let Some(value) = full.get(key) {
            agent_config.insert(key.to_string(), value.clone());
        }
    }
    let agent_config_bytes = serde_json::to_vec(&Value::Object(agent_config)).map_err(|error| {
        PaperImportError::new(format!("cannot serialize native agent config: {error}"))
    })?;
    let agent_config_sha256 = sha256_hex(&agent_config_bytes);
    let analysis_pipeline_sha256 = paper_execution_manifest_sha256();

    let rights = "Native simulator output retained as a local run artifact; this adapter does not redistribute study instruments or record-level human reference data. Review source/data rights before publication.";
    let mut artifacts = vec![
        artifact(
            "paper_run_attestation",
            ATTESTATION_FILE,
            &attestation_bytes,
            1,
            "Operator-authored source-execution attestation; Git source pins are checked against the frozen paper profile.",
        ),
        artifact(
            "paper_full_benchmark",
            FULL_BENCHMARK_FILE,
            &full_bytes,
            trial_count,
            rights,
        ),
        artifact(
            "paper_detailed_stats",
            DETAILED_STATS_FILE,
            &csv_bytes,
            csv_rows,
            rights,
        ),
        artifact(
            "paper_evaluation_results",
            EVALUATION_FILE,
            &evaluation_bytes,
            test_results.len() as u64,
            rights,
        ),
    ];

    let raw_responses_path: PathBuf = run_directory.join(RAW_RESPONSES_FILE);
    if raw_responses_path.exists() {
        let raw_bytes = fs::read(&raw_responses_path).map_err(|error| {
            PaperImportError::new(format!(
                "cannot read optional native artifact {}: {error}",
                raw_responses_path.display()
            ))
        })?;
        artifacts.push(artifact(
            "paper_raw_responses",
            RAW_RESPONSES_FILE,
            &raw_bytes,
            0,
            "Optional model-generated response trace. Kept local and access-controlled; not a human-subject source dataset.",
        ));
    }

    let receipt = StudyReceipt {
        study_id: study_id.clone(),
        study_tree_sha: study_tree_sha.to_string(),
        participant_count,
        trial_count,
        independent_unit: attestation.independent_unit,
        analysis_pipeline_sha256,
        agent_config_sha256,
        prompt_template_sha256: attestation.prompt_template_sha256.clone(),
        // The sidecar and pinned source hashes document the operator's
        // execution claim. They are not independent proof of process execution.
        analysis_protocol_preserved: attestation.analysis_protocol_preserved,
        official_pas: Some(official_pas),
        official_ecs: ecs,
        effects: extract_effects(test_results),
        artifacts,
    };

    Ok(PaperStudyImport {
        receipt,
        run_id: attestation.run_id,
        model_id: attestation.model_id,
        prompt_preset: attestation.prompt_preset,
        test_count: test_results.len(),
        run_count,
        usage_stats: evaluation.get("usage_stats").cloned(),
    })
}

/// Assemble exactly one source-pinned 12-study paper run from imported
/// per-study directories. The original upstream Python scorer remains the
/// authority for every PAS/ECS value; this function only joins receipts.
/// Serialize a fully validated paper-12 receipt into the versioned common
/// JSON schema. The function is intentionally a serializer only: it does not
/// calculate or aggregate PAS/ECS.
pub fn paper_receipt_to_json(receipt: &BenchmarkRunReceipt) -> Value {
    let studies: Vec<Value> = receipt
        .studies
        .iter()
        .map(|study| {
            let independent_unit = match study.independent_unit {
                IndependentUnit::Participant => "participant",
                IndependentUnit::Group => "group",
                IndependentUnit::Session => "session",
                IndependentUnit::Cluster => "cluster",
            };
            let effects: Vec<Value> = study
                .effects
                .iter()
                .map(|effect| {
                    json!({
                        "finding_id": effect.finding_id,
                        "test_id": effect.test_id,
                        "measure": effect.measure,
                        "unit": effect.unit,
                        "human_estimate": effect.human_estimate,
                        "simulated_estimate": effect.simulated_estimate,
                        "human_ci95": effect.human_ci95,
                        "simulated_ci95": effect.simulated_ci95
                    })
                })
                .collect();
            let artifacts: Vec<Value> = study
                .artifacts
                .iter()
                .map(|artifact| {
                    json!({
                        "role": artifact.role,
                        "path": artifact.path,
                        "sha256": artifact.sha256,
                        "record_count": artifact.record_count,
                        "contains_record_level_human_data": artifact.contains_record_level_human_data,
                        "published_in_repository": artifact.published_in_repository,
                        "redistribution_authorized": artifact.redistribution_authorized,
                        "rights_basis": artifact.rights_basis
                    })
                })
                .collect();
            json!({
                "study_id": study.study_id,
                "study_tree_sha": study.study_tree_sha,
                "participant_count": study.participant_count,
                "trial_count": study.trial_count,
                "independent_unit": independent_unit,
                "analysis_pipeline_sha256": study.analysis_pipeline_sha256,
                "agent_config_sha256": study.agent_config_sha256,
                "prompt_template_sha256": study.prompt_template_sha256,
                "analysis_protocol_preserved": study.analysis_protocol_preserved,
                "official_pas": study.official_pas,
                "official_ecs": study.official_ecs,
                "effects": effects,
                "artifacts": artifacts
            })
        })
        .collect();
    json!({
        "protocol_id": receipt.protocol_id,
        "source_repository": receipt.source_repository,
        "source_commit": receipt.source_commit,
        "source_root_tree": receipt.source_root_tree,
        "metric_contract_id": receipt.metric_contract_id,
        "run_id": receipt.run_id,
        "model_id": receipt.model_id,
        "studies": studies
    })
}

pub fn assemble_paper_run_receipt(
    imports: Vec<PaperStudyImport>,
) -> Result<BenchmarkRunReceipt, PaperImportError> {
    if imports.len() != PAPER_12_STUDY_PINS.len() {
        return Err(PaperImportError::new(format!(
            "paper protocol requires {} studies, got {}",
            PAPER_12_STUDY_PINS.len(),
            imports.len()
        )));
    }
    let first = imports
        .first()
        .ok_or_else(|| PaperImportError::new("no study imports supplied"))?;
    let run_id = first.run_id.clone();
    let model_id = first.model_id.clone();
    let prompt_preset = first.prompt_preset.clone();
    let expected_config = first.receipt.agent_config_sha256.clone();
    let expected_prompt_digest = first.receipt.prompt_template_sha256.clone();

    let mut seen = BTreeSet::new();
    let mut studies = Vec::with_capacity(imports.len());
    for import in imports {
        if import.run_id != run_id
            || import.model_id != model_id
            || import.prompt_preset != prompt_preset
            || import.receipt.agent_config_sha256 != expected_config
            || import.receipt.prompt_template_sha256 != expected_prompt_digest
        {
            return Err(PaperImportError::new(
                "paper study imports do not belong to one matched model/prompt/configuration run",
            ));
        }
        if !seen.insert(import.receipt.study_id.clone()) {
            return Err(PaperImportError::new(format!(
                "duplicate paper study import: {}",
                import.receipt.study_id
            )));
        }
        studies.push(import.receipt);
    }

    let receipt = BenchmarkRunReceipt {
        protocol_id: PAPER_12_PROTOCOL_ID.to_string(),
        source_repository: PAPER_12_SOURCE_REPOSITORY.to_string(),
        source_commit: PAPER_12_SOURCE_COMMIT.to_string(),
        source_root_tree: PAPER_12_SOURCE_ROOT_TREE.to_string(),
        metric_contract_id: PAPER_12_METRIC_CONTRACT_ID.to_string(),
        run_id,
        model_id,
        studies,
    };
    validate_run(&receipt).map_err(|errors| {
        PaperImportError::new(format!(
            "assembled paper receipt failed conformance: {}",
            errors.join("; ")
        ))
    })?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn temp_run_dir() -> PathBuf {
        let id = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "civdyn-humanstudy-paper-import-{}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("create temp import dir");
        path
    }

    fn attestation(study_id: &str) -> Value {
        let pin = PAPER_STUDY_EXECUTION_PINS
            .iter()
            .find(|pin| pin.study_id == study_id)
            .expect("paper source pin");
        let baseline_runner_blob_sha =
            expected_execution_source_sha("scripts/run_baseline_pipeline.py")
                .expect("baseline runner source pin");
        let generation_runner_blob_sha =
            expected_execution_source_sha("generation_pipeline/run.py")
                .expect("generation runner source pin");
        let generation_pipeline_blob_sha =
            expected_execution_source_sha("generation_pipeline/pipeline.py")
                .expect("generation pipeline source pin");
        let evaluator_runner_blob_sha =
            expected_execution_source_sha("src/evaluation/evaluator_runner.py")
                .expect("evaluator runner source pin");
        let stats_lib_blob_sha = expected_execution_source_sha("src/evaluation/stats_lib.py")
            .expect("statistics implementation source pin");
        json!({
            "protocol_id": PAPER_12_PROTOCOL_ID,
            "source_repository": PAPER_12_SOURCE_REPOSITORY,
            "source_commit": PAPER_12_SOURCE_COMMIT,
            "study_id": study_id,
            "study_config_blob_sha": pin.config_blob_sha,
            "evaluator_blob_sha": pin.evaluator_blob_sha,
            "baseline_runner_blob_sha": baseline_runner_blob_sha,
            "generation_runner_blob_sha": generation_runner_blob_sha,
            "generation_pipeline_blob_sha": generation_pipeline_blob_sha,
            "evaluator_runner_blob_sha": evaluator_runner_blob_sha,
            "stats_lib_blob_sha": stats_lib_blob_sha,
            "analysis_protocol_preserved": true,
            "run_id": "synthetic-paper-run-001",
            "model_id": "synthetic-model-not-a-real-run",
            "prompt_preset": "v3_human_plus_demo",
            "prompt_template_sha256": "d".repeat(64),
            "independent_unit": "participant"
        })
    }

    fn write_fixture(path: &Path, study_id: &str, with_ecs: bool) {
        let attestation = attestation(study_id);
        fs::write(
            path.join(ATTESTATION_FILE),
            serde_json::to_vec_pretty(&attestation).expect("serialize attestation"),
        )
        .expect("write attestation");

        let full = json!({
            "study_id": study_id,
            "title": "Synthetic study fixture",
            "model": "synthetic-model-not-a-real-run",
            "reasoning": "default",
            "use_real_llm": true,
            "system_prompt_preset": "v3_human_plus_demo",
            "random_seed": 42,
            "repeats_completed": 1,
            "repeats_total": 1,
            "all_runs_raw_results": [{
                "individual_data": [
                    {"participant_id": "p1", "responses": [{"response_text": "a"}, {"response_text": "b"}]},
                    {"participant_id": "p2", "responses": [{"response_text": "c"}, {"response_text": "d"}]}
                ]
            }]
        });
        fs::write(
            path.join(FULL_BENCHMARK_FILE),
            serde_json::to_vec_pretty(&full).expect("serialize full benchmark"),
        )
        .expect("write full benchmark");

        let mut evaluation = json!({
            "study_id": study_id,
            "score": 0.75,
            "n_participants": 2,
            "n_runs": 1,
            "test_results": [
                {
                    "sub_study_id": "sub_a",
                    "finding_id": "F1",
                    "test_name": "two_sample_t",
                    "scenario": "condition_a",
                    "agent_effect_d": 0.42,
                    "human_effect_d": 0.40
                },
                {
                    "sub_study_id": "sub_a",
                    "finding_id": "F2",
                    "test_name": "one_sample_t",
                    "scenario": "condition_b",
                    "agent_effect_d": -0.25,
                    "human_effect_d": -0.20
                }
            ],
            "usage_stats": {"total_tokens": 1200, "total_cost": 0.012}
        });
        if with_ecs {
            evaluation["ecs_corr_study"] = json!(0.4);
        }
        fs::write(
            path.join(EVALUATION_FILE),
            serde_json::to_vec_pretty(&evaluation).expect("serialize evaluation"),
        )
        .expect("write evaluation");

        let csv = format!(
            "{DETAILED_STATS_HEADER}
             {study_id},sub_a,F1,two_sample_t,condition_a,t,1.2,1.1,0.1,0.9,0.42,0.40,0.42,0.40
             {study_id},sub_a,F2,one_sample_t,condition_b,t,-0.5,-0.4,0.1,0.8,-0.25,-0.20,-0.25,-0.20
"
        );
        fs::write(path.join(DETAILED_STATS_FILE), csv).expect("write detailed stats");
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn imports_native_paper_outputs_and_hashes_exact_bytes() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let imported = import_paper_study_run(&path).expect("synthetic paper run imports");
        assert_eq!(imported.receipt.study_id, "study_001");
        assert_eq!(imported.receipt.participant_count, 2);
        assert_eq!(imported.receipt.trial_count, 4);
        assert_eq!(imported.receipt.official_pas, Some(0.75));
        assert_eq!(imported.receipt.official_ecs, Some(0.4));
        assert_eq!(imported.receipt.effects.len(), 2);
        assert_eq!(imported.test_count, 2);
        assert_eq!(imported.run_count, 1);
        assert_eq!(imported.receipt.artifacts.len(), 4);
        for artifact in &imported.receipt.artifacts {
            let native = fs::read(path.join(&artifact.path)).expect("read hashed fixture");
            assert_eq!(artifact.sha256, sha256_hex(&native));
        }
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn unavailable_per_study_ecs_is_preserved_as_none() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", false);
        let imported = import_paper_study_run(&path).expect("PAS-only study imports");
        assert_eq!(imported.receipt.official_pas, Some(0.75));
        assert_eq!(imported.receipt.official_ecs, None);
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn mismatched_source_commit_fails_closed() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let mut att = attestation("study_001");
        att["source_commit"] = json!("0".repeat(40));
        fs::write(
            path.join(ATTESTATION_FILE),
            serde_json::to_vec(&att).unwrap(),
        )
        .unwrap();
        assert!(import_paper_study_run(&path).is_err());
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn mismatched_global_runner_source_fails_closed() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let mut att = attestation("study_001");
        att["generation_pipeline_blob_sha"] = json!("0".repeat(40));
        fs::write(
            path.join(ATTESTATION_FILE),
            serde_json::to_vec(&att).expect("serialize invalid runner attestation"),
        )
        .expect("write invalid runner attestation");
        assert!(import_paper_study_run(&path).is_err());
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn mismatch_between_csv_and_evaluator_test_count_fails_closed() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let csv = format!(
            "{DETAILED_STATS_HEADER}
only,one,row
"
        );
        fs::write(path.join(DETAILED_STATS_FILE), csv).unwrap();
        assert!(import_paper_study_run(&path).is_err());
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn wrong_native_model_fails_closed() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let full: Value =
            serde_json::from_slice(&fs::read(path.join(FULL_BENCHMARK_FILE)).unwrap()).unwrap();
        let mut changed = full.clone();
        changed["model"] = json!("different-model");
        fs::write(
            path.join(FULL_BENCHMARK_FILE),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        assert!(import_paper_study_run(&path).is_err());
        fs::remove_dir_all(path).expect("remove temp directory");
    }

    #[test]
    fn assembles_and_serializes_complete_paper_twelve_run() {
        let root = temp_run_dir();
        let mut imports = Vec::new();
        for pin in PAPER_12_STUDY_PINS {
            let path = root.join(pin.study_id);
            fs::create_dir_all(&path).expect("create study fixture directory");
            write_fixture(&path, pin.study_id, true);
            imports.push(import_paper_study_run(&path).expect("paper study fixture imports"));
        }
        let receipt =
            assemble_paper_run_receipt(imports).expect("complete paper-12 receipt assembles");
        let json = paper_receipt_to_json(&receipt);
        assert_eq!(json["protocol_id"], PAPER_12_PROTOCOL_ID);
        assert_eq!(json["studies"].as_array().map(Vec::len), Some(12));
        assert_eq!(
            json["studies"][0]["artifacts"].as_array().map(Vec::len),
            Some(4)
        );
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn benchmark_assembly_rejects_incomplete_study_set() {
        let path = temp_run_dir();
        write_fixture(&path, "study_001", true);
        let one = import_paper_study_run(&path).expect("one study imports");
        assert!(assemble_paper_run_receipt(vec![one]).is_err());
        fs::remove_dir_all(path).expect("remove temp directory");
    }
}
