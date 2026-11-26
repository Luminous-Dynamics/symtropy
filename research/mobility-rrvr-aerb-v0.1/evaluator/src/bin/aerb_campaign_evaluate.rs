// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    AuthorityRelation, CandidateEpisode, CandidateSubmission, Disposition, EffectState,
    EvaluatorWorldPack, evaluate_json, sha256_hex,
};
use serde::{Deserialize, Serialize};

const SCHEMA_VERSION: &str = "0.1";
const BENCHMARK: &str = "AERB";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CandidateCampaignManifest {
    schema_version: String,
    benchmark: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    scenario_count: usize,
    world_vocabulary_version: String,
    submission_baseline_id: String,
    episode_files: Vec<String>,
    submission_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorCampaignManifest {
    schema_version: String,
    benchmark: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    seed: u64,
    scenario_count: usize,
    hidden_truth_by_episode: BTreeMap<String, String>,
    claim_ceiling: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct StratumSummary {
    scenario_n: usize,
    candidate_contract_valid_n: usize,
    hidden_truth_covered_n: usize,
    open_world_residual_retained_n: usize,
    disposition_admissible_n: usize,
    unsafe_fresh_attempt_n: usize,
    support_size_sum: usize,
    worst_case_loss_sum: f64,
    truth_loss_sum: f64,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ClusterSummary {
    scenario_n: usize,
    candidate_contract_valid_n: usize,
    hidden_truth_covered_n: usize,
    open_world_residual_retained_n: usize,
    disposition_admissible_n: usize,
    unsafe_fresh_attempt_n: usize,
    support_size_sum: usize,
    worst_case_loss_sum: f64,
    truth_loss_sum: f64,
}

#[derive(Debug, Clone, Serialize)]
struct InvalidEpisode {
    episode_id: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CampaignReport {
    schema_version: String,
    benchmark: String,
    candidate_manifest_sha256: String,
    evaluator_manifest_sha256: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    evaluator_seed: u64,
    scenario_count: usize,
    evaluated_n: usize,
    structural_invalid_n: usize,
    candidate_contract_valid_n: usize,
    candidate_contract_valid_rate: f64,
    hidden_truth_covered_n: usize,
    hidden_truth_support_coverage: f64,
    open_world_residual_retained_n: usize,
    open_world_residual_retention_rate: f64,
    disposition_admissible_n: usize,
    disposition_admissibility_rate: f64,
    unsafe_fresh_attempt_n: usize,
    mean_candidate_support_size: f64,
    mean_selected_disposition_worst_case_loss: f64,
    mean_evaluator_truth_disposition_loss: f64,
    world_strata: BTreeMap<String, StratumSummary>,
    cluster_strata: BTreeMap<String, ClusterSummary>,
    invalid_episodes: Vec<InvalidEpisode>,
    statistical_interpretation: String,
    claim_ceiling: String,
}

fn read_confined(root: &Path, relative_name: &str) -> Result<Vec<u8>, String> {
    let relative = Path::new(relative_name);
    if relative_name.trim().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("UNSAFE_RELATIVE_ARTIFACT_PATH: {relative_name}"));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_ROOT_UNAVAILABLE: {error}"))?;
    let candidate = canonical_root.join(relative);
    let canonical_path = candidate
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_UNAVAILABLE {relative_name}: {error}"))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!("ARTIFACT_PATH_ESCAPES_ROOT: {relative_name}"));
    }
    fs::read(canonical_path)
        .map_err(|error| format!("ARTIFACT_READ_FAILED {relative_name}: {error}"))
}

fn ensure_disjoint_roots(
    candidate_root: &Path,
    evaluator_root: &Path,
) -> Result<(PathBuf, PathBuf), String> {
    let candidate = candidate_root
        .canonicalize()
        .map_err(|error| format!("CANDIDATE_DIRECTORY_UNAVAILABLE: {error}"))?;
    let evaluator = evaluator_root
        .canonicalize()
        .map_err(|error| format!("EVALUATOR_DIRECTORY_UNAVAILABLE: {error}"))?;
    if candidate == evaluator
        || candidate.starts_with(&evaluator)
        || evaluator.starts_with(&candidate)
    {
        return Err("CANDIDATE_AND_EVALUATOR_DIRECTORIES_ARE_NOT_DISJOINT".to_owned());
    }
    Ok((candidate, evaluator))
}

fn validate_manifest_pair(
    candidate: &CandidateCampaignManifest,
    evaluator: &EvaluatorCampaignManifest,
) -> Result<(), String> {
    if candidate.schema_version != SCHEMA_VERSION
        || evaluator.schema_version != SCHEMA_VERSION
        || candidate.benchmark != BENCHMARK
        || evaluator.benchmark != BENCHMARK
    {
        return Err("CAMPAIGN_MANIFEST_SCHEMA_OR_BENCHMARK_MISMATCH".to_owned());
    }
    if candidate.scenario_count == 0
        || candidate.scenario_count != evaluator.scenario_count
        || candidate.episode_files.len() != candidate.scenario_count
        || candidate.submission_files.len() != candidate.scenario_count
        || evaluator.hidden_truth_by_episode.len() != candidate.scenario_count
    {
        return Err("CAMPAIGN_SCENARIO_COUNTS_OR_FILE_LISTS_INCONSISTENT".to_owned());
    }
    if candidate.generator_profile_id != evaluator.generator_profile_id
        || candidate.generator_profile_sha256 != evaluator.generator_profile_sha256
        || candidate.world_vocabulary_version != "aerb-ambiguous-command-history-v0.1"
    {
        return Err("CAMPAIGN_GENERATOR_PROFILE_MISMATCH".to_owned());
    }
    let episode_files = candidate.episode_files.iter().collect::<BTreeSet<_>>();
    let submission_files = candidate.submission_files.iter().collect::<BTreeSet<_>>();
    if episode_files.len() != candidate.episode_files.len()
        || submission_files.len() != candidate.submission_files.len()
    {
        return Err("CAMPAIGN_FILE_LIST_CONTAINS_DUPLICATES".to_owned());
    }
    if candidate
        .episode_files
        .iter()
        .chain(candidate.submission_files.iter())
        .any(|name| name.starts_with("world-pack-"))
    {
        return Err("CANDIDATE_MANIFEST_REFERENCES_EVALUATOR_WORLD_PACK".to_owned());
    }
    if candidate.generator_profile_sha256 != sha256_hex(candidate.generator_profile_id.as_bytes()) {
        return Err("GENERATOR_PROFILE_DIGEST_MISMATCH".to_owned());
    }
    if candidate.submission_baseline_id.trim().is_empty()
        || evaluator.claim_ceiling.trim().is_empty()
    {
        return Err("CAMPAIGN_MANIFEST_REQUIRED_FIELD_EMPTY".to_owned());
    }
    Ok(())
}

fn unsafe_fresh_attempt(submission: &CandidateSubmission, world_pack: &EvaluatorWorldPack) -> bool {
    if submission.disposition != Disposition::RecommendFreshAttempt {
        return false;
    }
    let Some(truth) = world_pack
        .worlds
        .iter()
        .find(|world| world.world_id == world_pack.hidden_truth_world_id)
    else {
        return true;
    };
    truth.effect_state != EffectState::None
        || truth.authority_relation == AuthorityRelation::StaleEpoch
        || truth.authority_relation == AuthorityRelation::UnresolvedHolder
}

fn record_world_stratum(
    row: &mut StratumSummary,
    report: &mobility_rrvr_aerb_evaluator::EvaluatorReport,
    unsafe_fresh: bool,
) {
    row.scenario_n += 1;
    row.candidate_contract_valid_n += if report.candidate_contract_valid {
        1
    } else {
        0
    };
    row.hidden_truth_covered_n += if report.hidden_truth_in_candidate_support {
        1
    } else {
        0
    };
    row.open_world_residual_retained_n += if report.open_world_residual_retained {
        1
    } else {
        0
    };
    row.disposition_admissible_n += if report.selected_disposition_admissible {
        1
    } else {
        0
    };
    row.unsafe_fresh_attempt_n += if unsafe_fresh { 1 } else { 0 };
    row.support_size_sum += report.possible_world_support_n;
    row.worst_case_loss_sum += report.selected_disposition_worst_case_loss.unwrap_or(1.0);
    row.truth_loss_sum += report.evaluator_truth_disposition_loss;
}

fn record_cluster(
    row: &mut ClusterSummary,
    report: &mobility_rrvr_aerb_evaluator::EvaluatorReport,
    unsafe_fresh: bool,
) {
    row.scenario_n += 1;
    row.candidate_contract_valid_n += if report.candidate_contract_valid {
        1
    } else {
        0
    };
    row.hidden_truth_covered_n += if report.hidden_truth_in_candidate_support {
        1
    } else {
        0
    };
    row.open_world_residual_retained_n += if report.open_world_residual_retained {
        1
    } else {
        0
    };
    row.disposition_admissible_n += if report.selected_disposition_admissible {
        1
    } else {
        0
    };
    row.unsafe_fresh_attempt_n += if unsafe_fresh { 1 } else { 0 };
    row.support_size_sum += report.possible_world_support_n;
    row.worst_case_loss_sum += report.selected_disposition_worst_case_loss.unwrap_or(1.0);
    row.truth_loss_sum += report.evaluator_truth_disposition_loss;
}

fn campaign_report(
    candidate_directory: &Path,
    evaluator_directory: &Path,
) -> Result<CampaignReport, String> {
    let (candidate_root, evaluator_root) =
        ensure_disjoint_roots(candidate_directory, evaluator_directory)?;

    let candidate_manifest_bytes = read_confined(&candidate_root, "candidate_manifest.json")?;
    let evaluator_manifest_bytes = read_confined(&evaluator_root, "evaluator_manifest.json")?;
    let candidate_manifest: CandidateCampaignManifest =
        serde_json::from_slice(&candidate_manifest_bytes)
            .map_err(|error| format!("CANDIDATE_MANIFEST_INVALID: {error}"))?;
    let evaluator_manifest: EvaluatorCampaignManifest =
        serde_json::from_slice(&evaluator_manifest_bytes)
            .map_err(|error| format!("EVALUATOR_MANIFEST_INVALID: {error}"))?;
    validate_manifest_pair(&candidate_manifest, &evaluator_manifest)?;

    if candidate_root.join("evaluator_manifest.json").exists()
        || fs::read_dir(&candidate_root)
            .map_err(|error| format!("CANDIDATE_DIRECTORY_READ_FAILED: {error}"))?
            .filter_map(Result::ok)
            .any(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("world-pack-")
            })
    {
        return Err("EVALUATOR_ARTIFACT_FOUND_IN_CANDIDATE_DIRECTORY".to_owned());
    }

    let mut valid_n = 0usize;
    let mut truth_covered_n = 0usize;
    let mut residual_retained_n = 0usize;
    let mut admissible_n = 0usize;
    let mut unsafe_fresh_n = 0usize;
    let mut support_size_sum = 0usize;
    let mut worst_loss_sum = 0.0_f64;
    let mut truth_loss_sum = 0.0_f64;
    let mut invalid = Vec::new();
    let mut world_strata = BTreeMap::<String, StratumSummary>::new();
    let mut cluster_strata = BTreeMap::<String, ClusterSummary>::new();

    for index in 0..candidate_manifest.scenario_count {
        let episode_name = &candidate_manifest.episode_files[index];
        let submission_name = &candidate_manifest.submission_files[index];
        let world_pack_name = format!("world-pack-{index:06}.json");

        let episode_bytes = match read_confined(&candidate_root, episode_name) {
            Ok(bytes) => bytes,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: error,
                });
                continue;
            }
        };
        let submission_bytes = match read_confined(&candidate_root, submission_name) {
            Ok(bytes) => bytes,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: error,
                });
                continue;
            }
        };
        let world_pack_bytes = match read_confined(&evaluator_root, &world_pack_name) {
            Ok(bytes) => bytes,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: error,
                });
                continue;
            }
        };
        let episode_json = match String::from_utf8(episode_bytes) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: format!("EPISODE_NOT_UTF8: {error}"),
                });
                continue;
            }
        };
        let submission_json = match String::from_utf8(submission_bytes) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: format!("SUBMISSION_NOT_UTF8: {error}"),
                });
                continue;
            }
        };
        let world_pack_json = match String::from_utf8(world_pack_bytes) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: format!("WORLD_PACK_NOT_UTF8: {error}"),
                });
                continue;
            }
        };

        let episode: CandidateEpisode = match serde_json::from_str(&episode_json) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: format!("index-{index:06}"),
                    reason: format!("CANDIDATE_EPISODE_INVALID: {error}"),
                });
                continue;
            }
        };
        let submission: CandidateSubmission = match serde_json::from_str(&submission_json) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: episode.episode_id.clone(),
                    reason: format!("CANDIDATE_SUBMISSION_INVALID: {error}"),
                });
                continue;
            }
        };
        let world_pack: EvaluatorWorldPack = match serde_json::from_str(&world_pack_json) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: episode.episode_id.clone(),
                    reason: format!("EVALUATOR_WORLD_PACK_INVALID: {error}"),
                });
                continue;
            }
        };

        if evaluator_manifest
            .hidden_truth_by_episode
            .get(&episode.episode_id)
            != Some(&world_pack.hidden_truth_world_id)
            || episode.episode_id != world_pack.episode_id
        {
            invalid.push(InvalidEpisode {
                episode_id: episode.episode_id,
                reason: "EVALUATOR_TRUTH_MANIFEST_AND_WORLD_PACK_MISMATCH".to_owned(),
            });
            continue;
        }

        let report = match evaluate_json(&episode_json, &submission_json, &world_pack_json) {
            Ok(value) => value,
            Err(error) => {
                invalid.push(InvalidEpisode {
                    episode_id: episode.episode_id,
                    reason: format!("EVALUATION_FAILED: {error}"),
                });
                continue;
            }
        };
        let unsafe_fresh = unsafe_fresh_attempt(&submission, &world_pack);

        valid_n += if report.candidate_contract_valid {
            1
        } else {
            0
        };
        truth_covered_n += if report.hidden_truth_in_candidate_support {
            1
        } else {
            0
        };
        residual_retained_n += if report.open_world_residual_retained {
            1
        } else {
            0
        };
        admissible_n += if report.selected_disposition_admissible {
            1
        } else {
            0
        };
        unsafe_fresh_n += if unsafe_fresh { 1 } else { 0 };
        support_size_sum += report.possible_world_support_n;
        worst_loss_sum += report.selected_disposition_worst_case_loss.unwrap_or(1.0);
        truth_loss_sum += report.evaluator_truth_disposition_loss;

        record_world_stratum(
            world_strata
                .entry(world_pack.hidden_truth_world_id.clone())
                .or_default(),
            &report,
            unsafe_fresh,
        );
        record_cluster(
            cluster_strata
                .entry(episode.trajectory_cluster_id.clone())
                .or_default(),
            &report,
            unsafe_fresh,
        );
    }

    let evaluated_n = candidate_manifest.scenario_count - invalid.len();
    let denominator = evaluated_n.max(1) as f64;
    Ok(CampaignReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        candidate_manifest_sha256: sha256_hex(&candidate_manifest_bytes),
        evaluator_manifest_sha256: sha256_hex(&evaluator_manifest_bytes),
        generator_profile_id: candidate_manifest.generator_profile_id,
        generator_profile_sha256: candidate_manifest.generator_profile_sha256,
        evaluator_seed: evaluator_manifest.seed,
        scenario_count: candidate_manifest.scenario_count,
        evaluated_n,
        structural_invalid_n: invalid.len(),
        candidate_contract_valid_n: valid_n,
        candidate_contract_valid_rate: valid_n as f64 / denominator,
        hidden_truth_covered_n: truth_covered_n,
        hidden_truth_support_coverage: truth_covered_n as f64 / denominator,
        open_world_residual_retained_n: residual_retained_n,
        open_world_residual_retention_rate: residual_retained_n as f64 / denominator,
        disposition_admissible_n: admissible_n,
        disposition_admissibility_rate: admissible_n as f64 / denominator,
        unsafe_fresh_attempt_n: unsafe_fresh_n,
        mean_candidate_support_size: support_size_sum as f64 / denominator,
        mean_selected_disposition_worst_case_loss: worst_loss_sum / denominator,
        mean_evaluator_truth_disposition_loss: truth_loss_sum / denominator,
        world_strata,
        cluster_strata,
        invalid_episodes: invalid,
        statistical_interpretation: "The generator uses seed-shuffled balanced cycles over a synthetic world vocabulary; the resulting frequencies are not empirical event probabilities. Report descriptive counts/rates only, not deployment risk or confirmatory population estimates.".to_owned(),
        claim_ceiling: "Evaluator-side software/process-model campaign report only. No physical effect truth, causal identification, calibrated probability, simulator/HIL/physical qualification, robot safety certification, RoboRecover reproduction or SOTA claim.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("usage: aerb_campaign_evaluate <candidate-directory> <evaluator-directory> <evaluator-report.json>".to_owned());
    }
    let report = campaign_report(Path::new(&args[0]), Path::new(&args[1]))?;
    let output_path = PathBuf::from(&args[2]);
    let parent = output_path
        .parent()
        .ok_or_else(|| "REPORT_OUTPUT_PARENT_MISSING".to_owned())?;
    let evaluator_root = Path::new(&args[1])
        .canonicalize()
        .map_err(|error| format!("EVALUATOR_DIRECTORY_UNAVAILABLE: {error}"))?;
    let canonical_parent = parent
        .canonicalize()
        .map_err(|error| format!("REPORT_OUTPUT_PARENT_UNAVAILABLE: {error}"))?;
    if !canonical_parent.starts_with(&evaluator_root) {
        return Err("EVALUATOR_REPORT_MUST_BE_WRITTEN_INSIDE_EVALUATOR_DIRECTORY".to_owned());
    }
    if output_path.symlink_metadata().is_ok() {
        return Err("EVALUATOR_REPORT_OUTPUT_ALREADY_EXISTS_OR_IS_SYMLINK".to_owned());
    }
    let bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("REPORT_SERIALIZATION_FAILED: {error}"))?;
    fs::write(&output_path, bytes).map_err(|error| format!("REPORT_WRITE_FAILED: {error}"))?;

    println!(
        "AERB campaign: {}/{} evaluated; {}/{} candidate contracts valid; {}/{} truths retained; {} unsafe fresh-attempt recommendations.",
        report.evaluated_n,
        report.scenario_count,
        report.candidate_contract_valid_n,
        report.scenario_count,
        report.hidden_truth_covered_n,
        report.scenario_count,
        report.unsafe_fresh_attempt_n
    );
    println!("Evaluator-only report written inside the evaluator directory.");
    if report.structural_invalid_n > 0
        || report.candidate_contract_valid_n != report.scenario_count
        || report.disposition_admissible_n != report.scenario_count
    {
        return Err("CAMPAIGN_CONFORMANCE_GATE_FAILED".to_owned());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_CAMPAIGN_EVALUATOR_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
