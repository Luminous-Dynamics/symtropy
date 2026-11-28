// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    CandidateEpisode, CandidateSubmission, EvaluatorWorldPack,
    blind_assignment_v0_1::{
        CandidateBlindManifest, EvaluatorBlindManifest, generate_blind_campaign_from_worlds,
    },
    blind_projection_audit_v0_1::{
        BlindProjectionContext, MetadataAttackRow,
        candidate_metadata_features, project_candidate_episode,
        project_candidate_submission,
    },
    canonical_sorted_json_sha256, sha256_hex,
};
use serde::{Deserialize, Serialize};

const SOURCE_SCHEMA_VERSION: &str = "0.1";
const SOURCE_BENCHMARK: &str = "AERB";
const WORLD_VOCABULARY_VERSION: &str = "aerb-ambiguous-command-history-v0.1";
const METADATA_INPUT_SCHEMA: &str = "aerb-metadata-attack-input-v0.1";
const MAX_EPISODES: usize = 100_000;
const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_READ_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const MAX_TOTAL_OUTPUT_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const CUSTODY_DOMAIN: &[u8] = b"AERB-BLIND-CAMPAIGN-CUSTODY-V0.1\0";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceCandidateManifest {
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
struct SourceEvaluatorManifest {
    schema_version: String,
    benchmark: String,
    generator_profile_id: String,
    generator_profile_sha256: String,
    #[serde(rename = "seed")]
    _seed: u64,
    scenario_count: usize,
    hidden_truth_by_episode: BTreeMap<String, String>,
    claim_ceiling: String,
}

#[derive(Clone)]
struct SourceCaseIndex {
    episode_file: String,
    submission_file: String,
    world_pack_file: String,
    original_episode_id: String,
    original_trajectory_cluster_id: String,
    hidden_truth_world_id: String,
}

#[derive(Clone, Debug, Serialize)]
struct CandidateArtifactSeal {
    episode_id: String,
    relative_file_name: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct EvaluatorArtifactSeal {
    relative_file_name: String,
    sha256: String,
}

#[derive(Clone, Debug, Serialize)]
struct CampaignCustodySeal {
    schema_version: String,
    campaign_id: String,
    source_candidate_manifest_sha256: String,
    source_evaluator_manifest_sha256: String,
    candidate_manifest_sha256: String,
    evaluator_manifest_sha256: String,
    projection_secret_sha256: String,
    candidate_artifacts: Vec<CandidateArtifactSeal>,
    evaluator_artifacts: Vec<EvaluatorArtifactSeal>,
    metadata_attack_input_sha256: String,
    custody_commitment_sha256: String,
}

#[derive(Serialize)]
struct MetadataAuditInput {
    schema_version: String,
    campaign_id: String,
    requested_folds: usize,
    rows: Vec<MetadataAttackRow>,
}

#[derive(Serialize)]
struct ReadyMarker {
    schema_version: String,
    campaign_id: String,
    manifest_sha256: String,
    custody_commitment_sha256: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn safe_leaf_name(value: &str) -> Result<&str, String> {
    let path = Path::new(value);
    let mut components = path.components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None)
            if !value.trim().is_empty() && !value.contains('\0') =>
        {
            Ok(value)
        }
        _ => Err(format!("UNSAFE_SOURCE_ARTIFACT_NAME: {value}")),
    }
}

fn path_overlaps(left: &Path, right: &Path) -> bool {
    left == right || left.starts_with(right) || right.starts_with(left)
}

fn prospective_root_path(path: &Path) -> Result<PathBuf, String> {
    if fs::symlink_metadata(path).is_ok() {
        return Err(format!("OUTPUT_ROOT_ALREADY_EXISTS: {}", path.display()));
    }
    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = parent_path
        .canonicalize()
        .map_err(|error| format!("OUTPUT_ROOT_PARENT_UNAVAILABLE: {error}"))?;
    let name = path
        .file_name()
        .ok_or_else(|| "OUTPUT_ROOT_NAME_MISSING".to_owned())?;
    Ok(parent.join(name))
}

struct OutputRootCleanup {
    roots: Vec<PathBuf>,
    armed: bool,
}

impl OutputRootCleanup {
    fn new(candidate: PathBuf, evaluator: PathBuf) -> Self {
        Self {
            roots: vec![candidate, evaluator],
            armed: true,
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for OutputRootCleanup {
    fn drop(&mut self) {
        if self.armed {
            for root in &self.roots {
                let _ = fs::remove_dir_all(root);
            }
        }
    }
}

fn create_private_root(path: &Path) -> Result<PathBuf, String> {
    if fs::symlink_metadata(path).is_ok() {
        return Err(format!("OUTPUT_ROOT_ALREADY_EXISTS: {}", path.display()));
    }
    let parent_path = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = parent_path
        .canonicalize()
        .map_err(|error| format!("OUTPUT_ROOT_PARENT_UNAVAILABLE: {error}"))?;
    let name = path
        .file_name()
        .ok_or_else(|| "OUTPUT_ROOT_NAME_MISSING".to_owned())?;
    let destination = parent.join(name);
    fs::create_dir(&destination)
        .map_err(|error| format!("OUTPUT_ROOT_CREATE_FAILED: {error}"))?;
    if let Err(error) = fs::set_permissions(&destination, fs::Permissions::from_mode(0o700)) {
        let _ = fs::remove_dir(&destination);
        return Err(format!("OUTPUT_ROOT_PERMISSION_FAILED: {error}"));
    }
    destination
        .canonicalize()
        .map_err(|error| format!("OUTPUT_ROOT_CANONICALIZE_FAILED: {error}"))
}

fn read_confined(
    root: &Path,
    relative_name: &str,
    total_read_bytes: &mut u64,
) -> Result<Vec<u8>, String> {
    safe_leaf_name(relative_name)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("SOURCE_ROOT_UNAVAILABLE: {error}"))?;
    let path = canonical_root.join(relative_name);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("SOURCE_ARTIFACT_UNAVAILABLE: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("SOURCE_ARTIFACT_NOT_REGULAR_FILE: {relative_name}"));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!("SOURCE_ARTIFACT_EXCEEDS_16_MIB: {relative_name}"));
    }
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("SOURCE_ARTIFACT_CANONICALIZE_FAILED: {error}"))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!("SOURCE_ARTIFACT_ESCAPES_ROOT: {relative_name}"));
    }
    *total_read_bytes = total_read_bytes
        .checked_add(metadata.len())
        .ok_or_else(|| "SOURCE_TOTAL_SIZE_OVERFLOW".to_owned())?;
    if *total_read_bytes > MAX_TOTAL_READ_BYTES {
        return Err("SOURCE_TOTAL_READ_BYTES_EXCEEDS_4_GIB".to_owned());
    }
    fs::read(canonical_path).map_err(|error| format!("SOURCE_ARTIFACT_READ_FAILED: {error}"))
}

fn write_private_file(
    root: &Path,
    relative_name: &str,
    bytes: &[u8],
    total_written_bytes: &mut u64,
) -> Result<String, String> {
    safe_leaf_name(relative_name)?;
    *total_written_bytes = total_written_bytes
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| "OUTPUT_TOTAL_SIZE_OVERFLOW".to_owned())?;
    if *total_written_bytes > MAX_TOTAL_OUTPUT_BYTES {
        return Err("OUTPUT_TOTAL_BYTES_EXCEEDS_4_GIB".to_owned());
    }
    let path = root.join(relative_name);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| format!("OUTPUT_FILE_CREATE_FAILED {relative_name}: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("OUTPUT_FILE_WRITE_FAILED {relative_name}: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("OUTPUT_FILE_FSYNC_FAILED {relative_name}: {error}"))?;
    Ok(sha256_hex(bytes))
}

fn serialize_pretty<T: Serialize>(value: &T, label: &str) -> Result<Vec<u8>, String> {
    serde_json::to_vec_pretty(value)
        .map_err(|error| format!("{label}_SERIALIZATION_FAILED: {error}"))
}

fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("OUTPUT_DIRECTORY_FSYNC_FAILED: {error}"))
}

fn custody_commitment(seal: &CampaignCustodySeal) -> Result<String, String> {
    let mut unsigned = seal.clone();
    unsigned.custody_commitment_sha256.clear();
    let bytes = serde_json::to_vec(&unsigned)
        .map_err(|error| format!("CUSTODY_SEAL_SERIALIZATION_FAILED: {error}"))?;
    let mut preimage = CUSTODY_DOMAIN.to_vec();
    preimage.extend_from_slice(&bytes);
    Ok(sha256_hex(&preimage))
}

fn load_source_index(
    candidate_root: &Path,
    evaluator_root: &Path,
    candidate_manifest: &SourceCandidateManifest,
    evaluator_manifest: &SourceEvaluatorManifest,
    total_read_bytes: &mut u64,
) -> Result<BTreeMap<String, Vec<SourceCaseIndex>>, String> {
    let mut by_world = BTreeMap::<String, Vec<SourceCaseIndex>>::new();
    let mut episode_ids = BTreeSet::new();
    let episode_files = candidate_manifest
        .episode_files
        .iter()
        .map(|name| safe_leaf_name(name).map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let submission_files = candidate_manifest
        .submission_files
        .iter()
        .map(|name| safe_leaf_name(name).map(str::to_owned))
        .collect::<Result<BTreeSet<_>, _>>()?;
    if episode_files.len() != candidate_manifest.scenario_count
        || submission_files.len() != candidate_manifest.scenario_count
        || !episode_files.is_disjoint(&submission_files)
        || episode_files.contains("candidate_manifest.json")
        || submission_files.contains("candidate_manifest.json")
    {
        return Err("SOURCE_CANDIDATE_FILE_LISTS_NOT_UNIQUE_OR_DISJOINT".to_owned());
    }
    if candidate_root.join("evaluator_manifest.json").exists()
        || fs::read_dir(candidate_root)
            .map_err(|error| format!("SOURCE_CANDIDATE_DIRECTORY_READ_FAILED: {error}"))?
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().starts_with("world-pack-"))
    {
        return Err("EVALUATOR_ARTIFACT_FOUND_IN_SOURCE_CANDIDATE_ROOT".to_owned());
    }

    for index in 0..candidate_manifest.scenario_count {
        let episode_file = safe_leaf_name(&candidate_manifest.episode_files[index])?.to_owned();
        let submission_file =
            safe_leaf_name(&candidate_manifest.submission_files[index])?.to_owned();
        if !episode_files.contains(&episode_file) || !submission_files.contains(&submission_file) {
            return Err("SOURCE_FILE_LIST_CHANGED_DURING_INDEX".to_owned());
        }
        let world_pack_file = format!("world-pack-{index:06}.json");
        let episode_bytes =
            read_confined(candidate_root, &episode_file, total_read_bytes)?;
        let episode: CandidateEpisode = serde_json::from_slice(&episode_bytes)
            .map_err(|error| format!("SOURCE_EPISODE_INVALID {episode_file}: {error}"))?;
        if episode.schema_version != SOURCE_SCHEMA_VERSION
            || episode.benchmark != SOURCE_BENCHMARK
            || episode.world_vocabulary_version != WORLD_VOCABULARY_VERSION
        {
            return Err(format!("SOURCE_EPISODE_PROFILE_MISMATCH: {episode_file}"));
        }
        if episode.episode_id.trim().is_empty()
            || episode.trajectory_cluster_id.trim().is_empty()
            || !episode_ids.insert(episode.episode_id.clone())
        {
            return Err(format!("SOURCE_EPISODE_IDENTITY_INVALID: {episode_file}"));
        }
        let world_pack_bytes =
            read_confined(evaluator_root, &world_pack_file, total_read_bytes)?;
        let world_pack: EvaluatorWorldPack = serde_json::from_slice(&world_pack_bytes)
            .map_err(|error| format!("SOURCE_WORLD_PACK_INVALID {world_pack_file}: {error}"))?;
        let expected_truth = evaluator_manifest
            .hidden_truth_by_episode
            .get(&episode.episode_id)
            .ok_or_else(|| format!("SOURCE_HIDDEN_TRUTH_MISSING: {}", episode.episode_id))?;
        if world_pack.episode_id != episode.episode_id
            || world_pack.hidden_truth_world_id != *expected_truth
        {
            return Err(format!("SOURCE_WORLD_PACK_IDENTITY_MISMATCH: {world_pack_file}"));
        }
        by_world
            .entry(world_pack.hidden_truth_world_id.clone())
            .or_default()
            .push(SourceCaseIndex {
                episode_file,
                submission_file,
                world_pack_file,
                original_episode_id: episode.episode_id,
                original_trajectory_cluster_id: episode.trajectory_cluster_id,
                hidden_truth_world_id: world_pack.hidden_truth_world_id,
            });
    }

    if episode_ids.len() != evaluator_manifest.hidden_truth_by_episode.len()
        || evaluator_manifest
            .hidden_truth_by_episode
            .keys()
            .any(|episode_id| !episode_ids.contains(episode_id))
    {
        return Err("SOURCE_EVALUATOR_TRUTH_MAP_NOT_BIJECTIVE".to_owned());
    }
    Ok(by_world)
}

fn validate_source_manifests(
    candidate: &SourceCandidateManifest,
    evaluator: &SourceEvaluatorManifest,
) -> Result<(), String> {
    if candidate.schema_version != SOURCE_SCHEMA_VERSION
        || evaluator.schema_version != SOURCE_SCHEMA_VERSION
        || candidate.benchmark != SOURCE_BENCHMARK
        || evaluator.benchmark != SOURCE_BENCHMARK
    {
        return Err("SOURCE_MANIFEST_SCHEMA_OR_BENCHMARK_MISMATCH".to_owned());
    }
    if !(8..=MAX_EPISODES).contains(&candidate.scenario_count)
        || candidate.scenario_count != evaluator.scenario_count
        || candidate.episode_files.len() != candidate.scenario_count
        || candidate.submission_files.len() != candidate.scenario_count
        || evaluator.hidden_truth_by_episode.len() != candidate.scenario_count
    {
        return Err("SOURCE_CAMPAIGN_COUNTS_INCONSISTENT".to_owned());
    }
    if candidate.generator_profile_id != evaluator.generator_profile_id
        || candidate.generator_profile_sha256 != evaluator.generator_profile_sha256
        || !valid_sha256(&candidate.generator_profile_sha256)
        || candidate.world_vocabulary_version != WORLD_VOCABULARY_VERSION
        || candidate.submission_baseline_id.trim().is_empty()
        || evaluator.claim_ceiling.trim().is_empty()
    {
        return Err("SOURCE_CAMPAIGN_PROFILE_INCONSISTENT".to_owned());
    }
    Ok(())
}

fn materialize(
    source_candidate_path: &Path,
    source_evaluator_path: &Path,
    candidate_output_path: &Path,
    evaluator_output_path: &Path,
) -> Result<(), String> {
    let source_candidate_root = source_candidate_path
        .canonicalize()
        .map_err(|error| format!("SOURCE_CANDIDATE_ROOT_UNAVAILABLE: {error}"))?;
    let source_evaluator_root = source_evaluator_path
        .canonicalize()
        .map_err(|error| format!("SOURCE_EVALUATOR_ROOT_UNAVAILABLE: {error}"))?;
    if path_overlaps(&source_candidate_root, &source_evaluator_root) {
        return Err("SOURCE_CANDIDATE_AND_EVALUATOR_ROOTS_OVERLAP".to_owned());
    }

    let mut total_read_bytes = 0_u64;
    let source_candidate_manifest_bytes =
        read_confined(&source_candidate_root, "candidate_manifest.json", &mut total_read_bytes)?;
    let source_evaluator_manifest_bytes =
        read_confined(&source_evaluator_root, "evaluator_manifest.json", &mut total_read_bytes)?;
    let source_candidate_manifest: SourceCandidateManifest =
        serde_json::from_slice(&source_candidate_manifest_bytes)
            .map_err(|error| format!("SOURCE_CANDIDATE_MANIFEST_INVALID: {error}"))?;
    let source_evaluator_manifest: SourceEvaluatorManifest =
        serde_json::from_slice(&source_evaluator_manifest_bytes)
            .map_err(|error| format!("SOURCE_EVALUATOR_MANIFEST_INVALID: {error}"))?;
    validate_source_manifests(&source_candidate_manifest, &source_evaluator_manifest)?;

    let mut source_by_world = load_source_index(
        &source_candidate_root,
        &source_evaluator_root,
        &source_candidate_manifest,
        &source_evaluator_manifest,
        &mut total_read_bytes,
    )?;
    let world_schedule = source_by_world
        .values()
        .flat_map(|cases| cases.iter().map(|case| case.hidden_truth_world_id.clone()))
        .collect::<Vec<_>>();
    if world_schedule.len() != source_candidate_manifest.scenario_count {
        return Err("SOURCE_WORLD_SCHEDULE_INCOMPLETE".to_owned());
    }

    let candidate_target = prospective_root_path(candidate_output_path)?;
    let evaluator_target = prospective_root_path(evaluator_output_path)?;
    let intended_roots = [
        &source_candidate_root,
        &source_evaluator_root,
        &candidate_target,
        &evaluator_target,
    ];
    for (index, root) in intended_roots.iter().enumerate() {
        for other in intended_roots.iter().skip(index + 1) {
            if path_overlaps(root, other) {
                return Err("SOURCE_AND_BLIND_CAMPAIGN_ROOTS_MUST_BE_PAIRWISE_DISJOINT".to_owned());
            }
        }
    }

    let candidate_root = create_private_root(candidate_output_path)?;
    let evaluator_root = match create_private_root(evaluator_output_path) {
        Ok(path) => path,
        Err(error) => {
            let _ = fs::remove_dir(&candidate_root);
            return Err(error);
        }
    };
    let mut output_cleanup =
        OutputRootCleanup::new(candidate_root.clone(), evaluator_root.clone());

    let generated = generate_blind_campaign_from_worlds(&world_schedule)
        .map_err(|error| format!("BLIND_ASSIGNMENT_FAILED: {error}"))?;
    let projection_context = BlindProjectionContext::generate()
        .map_err(|error| format!("BLIND_PROJECTION_KEY_GENERATION_FAILED: {error}"))?;
    let projection_secret = projection_context.export_evaluator_secret();
    let projection_secret_bytes =
        serialize_pretty(&projection_secret, "PROJECTION_SECRET")?;

    let mut candidate_artifacts = Vec::with_capacity(generated.evaluator_manifest.assignments.len() * 2);
    let mut evaluator_artifacts = Vec::with_capacity(generated.evaluator_manifest.assignments.len() * 4 + 2);
    let mut metadata_rows = Vec::with_capacity(generated.evaluator_manifest.assignments.len());
    let mut candidate_output_bytes = 0_u64;
    let mut evaluator_output_bytes = 0_u64;
    let candidate_manifest_json = serialize_pretty(&generated.candidate_manifest, "CANDIDATE_MANIFEST")?;
    let evaluator_manifest_json = serialize_pretty(&generated.evaluator_manifest, "EVALUATOR_MANIFEST")?;

    for (file_order_index, assignment) in generated.evaluator_manifest.assignments.iter().enumerate() {
        let source_cases = source_by_world
            .get_mut(&assignment.hidden_truth_world_id)
            .ok_or_else(|| "SOURCE_WORLD_CLASS_NOT_FOUND".to_owned())?;
        let source = source_cases
            .pop()
            .ok_or_else(|| "SOURCE_WORLD_CLASS_UNDERFLOW".to_owned())?;

        let episode_bytes =
            read_confined(&source_candidate_root, &source.episode_file, &mut total_read_bytes)?;
        let submission_bytes =
            read_confined(&source_candidate_root, &source.submission_file, &mut total_read_bytes)?;
        let world_pack_bytes =
            read_confined(&source_evaluator_root, &source.world_pack_file, &mut total_read_bytes)?;
        let episode_json = String::from_utf8(episode_bytes)
            .map_err(|error| format!("SOURCE_EPISODE_NOT_UTF8: {error}"))?;
        let submission_json = String::from_utf8(submission_bytes)
            .map_err(|error| format!("SOURCE_SUBMISSION_NOT_UTF8: {error}"))?;
        let world_pack_json = String::from_utf8(world_pack_bytes)
            .map_err(|error| format!("SOURCE_WORLD_PACK_NOT_UTF8: {error}"))?;
        let original_episode: CandidateEpisode = serde_json::from_str(&episode_json)
            .map_err(|error| format!("SOURCE_EPISODE_INVALID: {error}"))?;
        let original_submission: CandidateSubmission = serde_json::from_str(&submission_json)
            .map_err(|error| format!("SOURCE_SUBMISSION_INVALID: {error}"))?;
        let mut world_pack: EvaluatorWorldPack = serde_json::from_str(&world_pack_json)
            .map_err(|error| format!("SOURCE_WORLD_PACK_INVALID: {error}"))?;

        if original_episode.episode_id != source.original_episode_id
            || original_episode.trajectory_cluster_id != source.original_trajectory_cluster_id
            || original_submission.episode_id != original_episode.episode_id
            || world_pack.episode_id != original_episode.episode_id
            || world_pack.hidden_truth_world_id != assignment.hidden_truth_world_id
        {
            return Err("SOURCE_ARTIFACT_IDENTITY_CHANGED_DURING_MATERIALIZATION".to_owned());
        }
        let source_snapshot = canonical_sorted_json_sha256(&episode_json)
            .map_err(|error| format!("SOURCE_EPISODE_DIGEST_FAILED: {error}"))?;
        if original_submission.candidate_snapshot_sha256 != source_snapshot {
            return Err("SOURCE_SUBMISSION_SNAPSHOT_DIGEST_MISMATCH".to_owned());
        }

        let (projected_episode, receipt) = project_candidate_episode(
            &projection_context,
            &original_episode,
            &assignment.episode_id,
        )
        .map_err(|error| format!("CANDIDATE_EPISODE_PROJECTION_FAILED: {error}"))?;
        let projected_submission = project_candidate_submission(
            &projection_context,
            &original_submission,
            &original_episode,
            &projected_episode,
            &receipt,
        )
        .map_err(|error| format!("CANDIDATE_SUBMISSION_PROJECTION_FAILED: {error}"))?;
        world_pack.episode_id = assignment.episode_id.clone();

        let episode_name = format!("episode-{}.json", assignment.episode_id);
        let submission_name = format!("submission-{}.json", assignment.episode_id);
        let world_pack_name = format!("world-pack-{}.json", assignment.episode_id);
        let receipt_name = format!("projection-receipt-{}.json", assignment.episode_id);
        let source_episode_name = format!("source-episode-{}.json", assignment.episode_id);
        let source_submission_name = format!("source-submission-{}.json", assignment.episode_id);

        let episode_output_bytes = serialize_pretty(&projected_episode, "PROJECTED_EPISODE")?;
        let submission_output_bytes = serialize_pretty(&projected_submission, "PROJECTED_SUBMISSION")?;
        let world_pack_output_bytes = serialize_pretty(&world_pack, "EVALUATOR_WORLD_PACK")?;
        let receipt_output_bytes = serialize_pretty(&receipt, "PROJECTION_RECEIPT")?;
        let source_episode_output_bytes = serialize_pretty(&original_episode, "SOURCE_EPISODE_ARCHIVE")?;
        let source_submission_output_bytes =
            serialize_pretty(&original_submission, "SOURCE_SUBMISSION_ARCHIVE")?;

        let source_episode_sha256 = write_private_file(
            &evaluator_root,
            &source_episode_name,
            &source_episode_output_bytes,
            &mut evaluator_output_bytes,
        )?;
        let source_submission_sha256 = write_private_file(
            &evaluator_root,
            &source_submission_name,
            &source_submission_output_bytes,
            &mut evaluator_output_bytes,
        )?;
        evaluator_artifacts.push(EvaluatorArtifactSeal {
            relative_file_name: source_episode_name,
            sha256: source_episode_sha256,
        });
        evaluator_artifacts.push(EvaluatorArtifactSeal {
            relative_file_name: source_submission_name,
            sha256: source_submission_sha256,
        });

        let episode_sha256 = write_private_file(
            &candidate_root,
            &episode_name,
            &episode_output_bytes,
            &mut candidate_output_bytes,
        )?;
        let submission_sha256 = write_private_file(
            &candidate_root,
            &submission_name,
            &submission_output_bytes,
            &mut candidate_output_bytes,
        )?;
        let world_pack_sha256 = write_private_file(
            &evaluator_root,
            &world_pack_name,
            &world_pack_output_bytes,
            &mut evaluator_output_bytes,
        )?;
        let receipt_sha256 = write_private_file(
            &evaluator_root,
            &receipt_name,
            &receipt_output_bytes,
            &mut evaluator_output_bytes,
        )?;

        candidate_artifacts.push(CandidateArtifactSeal {
            episode_id: assignment.episode_id.clone(),
            relative_file_name: episode_name.clone(),
            sha256: episode_sha256,
        });
        candidate_artifacts.push(CandidateArtifactSeal {
            episode_id: assignment.episode_id.clone(),
            relative_file_name: submission_name,
            sha256: submission_sha256,
        });
        evaluator_artifacts.push(EvaluatorArtifactSeal {
            relative_file_name: world_pack_name,
            sha256: world_pack_sha256,
        });
        evaluator_artifacts.push(EvaluatorArtifactSeal {
            relative_file_name: receipt_name,
            sha256: receipt_sha256,
        });

        let features = candidate_metadata_features(
            &projected_episode,
            file_order_index,
            &episode_name,
            episode_output_bytes.len(),
        );
        metadata_rows.push(MetadataAttackRow {
            campaign_id: generated.candidate_manifest.campaign_id.clone(),
            episode_id: assignment.episode_id.clone(),
            evaluator_cluster_id: source.original_trajectory_cluster_id,
            hidden_truth_world_id: assignment.hidden_truth_world_id.clone(),
            candidate_visible_features: features,
        });
    }

    if source_by_world.values().any(|cases| !cases.is_empty()) {
        return Err("SOURCE_WORLD_CLASS_OVERFLOW_AFTER_ASSIGNMENT".to_owned());
    }
    let independent_cluster_n = metadata_rows
        .iter()
        .map(|row| row.evaluator_cluster_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    if independent_cluster_n < 2 {
        return Err("BLIND_METADATA_AUDIT_REQUIRES_TWO_INDEPENDENT_TRAJECTORY_CLUSTERS".to_owned());
    }
    let requested_folds = independent_cluster_n.min(5);

    let metadata_input = MetadataAuditInput {
        schema_version: METADATA_INPUT_SCHEMA.to_owned(),
        campaign_id: generated.candidate_manifest.campaign_id.clone(),
        requested_folds,
        rows: metadata_rows,
    };
    let metadata_input_bytes = serialize_pretty(&metadata_input, "METADATA_ATTACK_INPUT")?;
    let metadata_input_sha256 = write_private_file(
        &evaluator_root,
        "metadata_attack_input.json",
        &metadata_input_bytes,
        &mut evaluator_output_bytes,
    )?;
    evaluator_artifacts.push(EvaluatorArtifactSeal {
        relative_file_name: "metadata_attack_input.json".to_owned(),
        sha256: metadata_input_sha256.clone(),
    });

    let projection_secret_sha256 = write_private_file(
        &evaluator_root,
        "projection-secret.json",
        &projection_secret_bytes,
        &mut evaluator_output_bytes,
    )?;
    evaluator_artifacts.push(EvaluatorArtifactSeal {
        relative_file_name: "projection-secret.json".to_owned(),
        sha256: projection_secret_sha256.clone(),
    });

    let candidate_manifest_sha256 = write_private_file(
        &candidate_root,
        "candidate_manifest.json",
        &candidate_manifest_json,
        &mut candidate_output_bytes,
    )?;
    let evaluator_manifest_sha256 = write_private_file(
        &evaluator_root,
        "evaluator_manifest.json",
        &evaluator_manifest_json,
        &mut evaluator_output_bytes,
    )?;

    let mut seal = CampaignCustodySeal {
        schema_version: "aerb-blind-campaign-custody-v0.1".to_owned(),
        campaign_id: generated.candidate_manifest.campaign_id.clone(),
        source_candidate_manifest_sha256: sha256_hex(&source_candidate_manifest_bytes),
        source_evaluator_manifest_sha256: sha256_hex(&source_evaluator_manifest_bytes),
        candidate_manifest_sha256: candidate_manifest_sha256.clone(),
        evaluator_manifest_sha256: evaluator_manifest_sha256.clone(),
        projection_secret_sha256,
        candidate_artifacts,
        evaluator_artifacts,
        metadata_attack_input_sha256: metadata_input_sha256,
        custody_commitment_sha256: String::new(),
    };
    seal.custody_commitment_sha256 = custody_commitment(&seal)?;
    let seal_bytes = serialize_pretty(&seal, "CAMPAIGN_CUSTODY_SEAL")?;
    let seal_sha256 = write_private_file(
        &evaluator_root,
        "campaign-custody-seal.json",
        &seal_bytes,
        &mut evaluator_output_bytes,
    )?;

    let candidate_ready = ReadyMarker {
        schema_version: "aerb-blind-campaign-ready-v0.1".to_owned(),
        campaign_id: generated.candidate_manifest.campaign_id.clone(),
        manifest_sha256: candidate_manifest_sha256,
        custody_commitment_sha256: seal.custody_commitment_sha256.clone(),
    };
    let evaluator_ready = ReadyMarker {
        schema_version: "aerb-blind-campaign-ready-v0.1".to_owned(),
        campaign_id: generated.candidate_manifest.campaign_id.clone(),
        manifest_sha256: evaluator_manifest_sha256,
        custody_commitment_sha256: seal.custody_commitment_sha256.clone(),
    };
    let candidate_ready_bytes = serialize_pretty(&candidate_ready, "CANDIDATE_READY_MARKER")?;
    let evaluator_ready_bytes = serialize_pretty(&evaluator_ready, "EVALUATOR_READY_MARKER")?;
    write_private_file(
        &candidate_root,
        "READY.json",
        &candidate_ready_bytes,
        &mut candidate_output_bytes,
    )?;
    write_private_file(
        &evaluator_root,
        "READY.json",
        &evaluator_ready_bytes,
        &mut evaluator_output_bytes,
    )?;
    sync_directory(&candidate_root)?;
    sync_directory(&evaluator_root)?;

    println!(
        "AERB blind campaign materialized: campaign={} episodes={} clusters={} candidate_bytes={} evaluator_bytes={}",
        generated.candidate_manifest.campaign_id,
        generated.candidate_manifest.episode_ids.len(),
        independent_cluster_n,
        candidate_output_bytes,
        evaluator_output_bytes
    );
    println!(
        "Evaluator-only custody seal={} (not an issuer signature); candidate/evaluator roots are separate and mode 0700.",
        seal_sha256
    );
    println!("Claim ceiling: synthetic/process-model input only; no HIL, physical or SOTA qualification.");
    output_cleanup.disarm();
    Ok(())
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(
            "usage: aerb_blind_materialize <source-candidate-dir> <source-evaluator-dir> <candidate-output-dir> <evaluator-output-dir>"
                .to_owned(),
        );
    }
    materialize(
        Path::new(&args[0]),
        Path::new(&args[1]),
        Path::new(&args[2]),
        Path::new(&args[3]),
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_BLIND_MATERIALIZE_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_artifact_names_must_be_single_safe_path_components() {
        assert!(safe_leaf_name("episode-000001.json").is_ok());
        assert!(safe_leaf_name("../episode.json").is_err());
        assert!(safe_leaf_name("/tmp/episode.json").is_err());
        assert!(safe_leaf_name("nested/episode.json").is_err());
    }

    #[test]
    fn source_and_output_root_overlap_is_detected() {
        let root = PathBuf::from("/tmp/aerb/root");
        assert!(path_overlaps(&root, &root.join("candidate")));
        assert!(path_overlaps(&root.join("candidate"), &root));
        assert!(!path_overlaps(&root.join("candidate"), &root.join("evaluator")));
    }

    #[test]
    fn custody_seal_changes_when_any_artifact_digest_changes() {
        let mut seal = CampaignCustodySeal {
            schema_version: "aerb-blind-campaign-custody-v0.1".to_owned(),
            campaign_id: "camp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            source_candidate_manifest_sha256: "a".repeat(64),
            source_evaluator_manifest_sha256: "b".repeat(64),
            candidate_manifest_sha256: "c".repeat(64),
            evaluator_manifest_sha256: "d".repeat(64),
            projection_secret_sha256: "e".repeat(64),
            candidate_artifacts: vec![CandidateArtifactSeal {
                episode_id: "ep_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
                relative_file_name: "episode-ep_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.json".to_owned(),
                sha256: "f".repeat(64),
            }],
            evaluator_artifacts: Vec::new(),
            metadata_attack_input_sha256: "0".repeat(64),
            custody_commitment_sha256: String::new(),
        };
        let original = custody_commitment(&seal).expect("initial commitment");
        seal.candidate_artifacts[0].sha256 = "1".repeat(64);
        let changed = custody_commitment(&seal).expect("changed commitment");
        assert_ne!(original, changed);
    }
}
