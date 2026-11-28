// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Component, Path, PathBuf},
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    CandidateEpisode, CandidateSubmission, EvaluatorReport, EvaluatorWorldPack,
    canonical_sorted_json_sha256, evaluate_json, sha256_hex,
    blind_assignment_v0_1::{
        CandidateBlindManifest, EvaluatorBlindManifest, verify_post_run_reveal,
    },
    blind_projection_audit_v0_1::{
        BlindProjectionContext, BlindProjectionReceipt, MetadataAttackRow,
        MetadataLeakageReport, BlindProjectionSecretV0_1, audit_metadata_leakage,
        candidate_metadata_features, project_candidate_episode,
        project_candidate_submission, verify_projected_episode,
    },
};
use serde::{Deserialize, Serialize};

const READY_SCHEMA: &str = "aerb-blind-campaign-ready-v0.1";
const METADATA_INPUT_SCHEMA: &str = "aerb-metadata-attack-input-v0.1";
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;
const CUSTODY_DOMAIN: &[u8] = b"AERB-BLIND-CAMPAIGN-CUSTODY-V0.1\0";

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateArtifactSeal {
    episode_id: String,
    relative_file_name: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EvaluatorArtifactSeal {
    relative_file_name: String,
    sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadyMarker {
    schema_version: String,
    campaign_id: String,
    manifest_sha256: String,
    custody_commitment_sha256: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataAuditInput {
    schema_version: String,
    campaign_id: String,
    requested_folds: usize,
    rows: Vec<MetadataAttackRow>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct DecisionStratum {
    scenario_n: usize,
    candidate_contract_valid_n: usize,
    hidden_truth_covered_n: usize,
    open_world_residual_retained_n: usize,
    disposition_admissible_n: usize,
    unsafe_fresh_attempt_n: usize,
    support_size_sum: usize,
    worst_case_loss_sum: f64,
    evaluator_truth_loss_sum: f64,
}

#[derive(Clone, Debug, Serialize)]
struct InvalidEpisode {
    episode_id: String,
    reason: String,
}

#[derive(Clone, Debug, Serialize)]
struct BlindCampaignReport {
    schema_version: String,
    benchmark: String,
    campaign_id: String,
    candidate_manifest_sha256: String,
    evaluator_manifest_sha256: String,
    custody_seal_sha256: String,
    scheduled_n: usize,
    evaluated_n: usize,
    structurally_invalid_n: usize,
    candidate_contract_valid_n: usize,
    candidate_contract_invalid_n: usize,
    candidate_contract_valid_rate_of_scheduled: f64,
    hidden_truth_covered_n: usize,
    hidden_truth_support_coverage_rate_of_scheduled: f64,
    open_world_residual_retained_n: usize,
    open_world_residual_retention_rate_of_scheduled: f64,
    disposition_admissible_n: usize,
    disposition_admissibility_rate_of_scheduled: f64,
    unsafe_fresh_attempt_n: usize,
    mean_selected_worst_case_loss: f64,
    mean_evaluator_truth_loss: f64,
    world_strata: BTreeMap<String, DecisionStratum>,
    source_cluster_strata: BTreeMap<String, DecisionStratum>,
    metadata_leakage: MetadataLeakageReport,
    invalid_episodes: Vec<InvalidEpisode>,
    statistical_interpretation: String,
    claim_ceiling: String,
}

fn safe_leaf_name(value: &str) -> Result<(), String> {
    let mut components = Path::new(value).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None)
            if !value.trim().is_empty() && !value.contains('\0') =>
        {
            Ok(())
        }
        _ => Err(format!("UNSAFE_ARTIFACT_NAME: {value}")),
    }
}

fn read_confined(root: &Path, relative_name: &str) -> Result<Vec<u8>, String> {
    safe_leaf_name(relative_name)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_ROOT_UNAVAILABLE: {error}"))?;
    let path = canonical_root.join(relative_name);
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("ARTIFACT_UNAVAILABLE {relative_name}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("ARTIFACT_NOT_REGULAR_FILE: {relative_name}"));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!("ARTIFACT_EXCEEDS_16_MIB: {relative_name}"));
    }
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("ARTIFACT_CANONICALIZE_FAILED: {error}"))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!("ARTIFACT_ESCAPES_ROOT: {relative_name}"));
    }
    fs::read(canonical).map_err(|error| format!("ARTIFACT_READ_FAILED: {error}"))
}

fn parse_json<T: for<'de> Deserialize<'de>>(bytes: &[u8], label: &str) -> Result<T, String> {
    serde_json::from_slice(bytes).map_err(|error| format!("{label}_INVALID_JSON: {error}"))
}

fn verify_directory_entries(root: &Path, expected: &BTreeSet<String>) -> Result<(), String> {
    let observed = fs::read_dir(root)
        .map_err(|error| format!("DIRECTORY_READ_FAILED: {error}"))?
        .map(|entry| {
            entry
                .map(|value| value.file_name().to_string_lossy().into_owned())
                .map_err(|error| format!("DIRECTORY_ENTRY_READ_FAILED: {error}"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if &observed != expected {
        return Err(format!(
            "DIRECTORY_CONTENT_SET_MISMATCH expected={} observed={}",
            expected.len(),
            observed.len()
        ));
    }
    Ok(())
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

fn read_hash_sealed(
    root: &Path,
    file_name: &str,
    expected_sha256: &str,
) -> Result<Vec<u8>, String> {
    let bytes = read_confined(root, file_name)?;
    if sha256_hex(&bytes) != expected_sha256 {
        return Err(format!("ARTIFACT_DIGEST_MISMATCH: {file_name}"));
    }
    Ok(bytes)
}

fn artifact_map(
    entries: &[EvaluatorArtifactSeal],
) -> Result<BTreeMap<String, String>, String> {
    let mut map = BTreeMap::new();
    for entry in entries {
        safe_leaf_name(&entry.relative_file_name)?;
        if entry.sha256.len() != 64
            || !entry
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("EVALUATOR_ARTIFACT_SEAL_DIGEST_INVALID".to_owned());
        }
        if map
            .insert(entry.relative_file_name.clone(), entry.sha256.clone())
            .is_some()
        {
            return Err("DUPLICATE_EVALUATOR_ARTIFACT_SEAL".to_owned());
        }
    }
    Ok(map)
}

fn candidate_artifact_map(
    entries: &[CandidateArtifactSeal],
) -> Result<BTreeMap<String, (String, String)>, String> {
    let mut map = BTreeMap::new();
    for entry in entries {
        safe_leaf_name(&entry.relative_file_name)?;
        if entry.sha256.len() != 64
            || !entry
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("CANDIDATE_ARTIFACT_SEAL_DIGEST_INVALID".to_owned());
        }
        if map
            .insert(
                entry.relative_file_name.clone(),
                (entry.episode_id.clone(), entry.sha256.clone()),
            )
            .is_some()
        {
            return Err("DUPLICATE_CANDIDATE_ARTIFACT_SEAL".to_owned());
        }
    }
    Ok(map)
}

fn record_stratum(
    row: &mut DecisionStratum,
    report: &EvaluatorReport,
    unsafe_fresh: bool,
) {
    row.scenario_n += 1;
    row.candidate_contract_valid_n += usize::from(report.candidate_contract_valid);
    row.hidden_truth_covered_n += usize::from(report.hidden_truth_in_candidate_support);
    row.open_world_residual_retained_n += usize::from(report.open_world_residual_retained);
    row.disposition_admissible_n += usize::from(report.selected_disposition_admissible);
    row.unsafe_fresh_attempt_n += usize::from(unsafe_fresh);
    row.support_size_sum += report.possible_world_support_n;
    row.worst_case_loss_sum += report.selected_disposition_worst_case_loss.unwrap_or(1.0);
    row.evaluator_truth_loss_sum += report.evaluator_truth_disposition_loss;
}

fn unsafe_fresh_recommendation(
    submission: &CandidateSubmission,
    hidden_truth_world_id: &str,
) -> bool {
    submission.disposition
        == mobility_rrvr_aerb_evaluator::Disposition::RecommendFreshAttempt
        && !matches!(
            hidden_truth_world_id,
            "not_dispatched" | "dispatched_no_effect"
        )
}

fn write_report(root: &Path, path: &Path, report: &BlindCampaignReport) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(report)
        .map_err(|error| format!("REPORT_SERIALIZATION_FAILED: {error}"))?;
    if path.parent().and_then(|parent| parent.canonicalize().ok()).as_deref() != Some(root) {
        return Err("REPORT_PATH_MUST_REMAIN_INSIDE_EVALUATOR_ROOT".to_owned());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|error| format!("REPORT_CREATE_FAILED: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("REPORT_WRITE_FAILED: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("REPORT_FSYNC_FAILED: {error}"))?;
    fs::File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("REPORT_DIRECTORY_FSYNC_FAILED: {error}"))?;
    Ok(())
}

fn campaign_report(
    candidate_root: &Path,
    evaluator_root: &Path,
) -> Result<BlindCampaignReport, String> {
    let candidate_root = candidate_root
        .canonicalize()
        .map_err(|error| format!("CANDIDATE_ROOT_UNAVAILABLE: {error}"))?;
    let evaluator_root = evaluator_root
        .canonicalize()
        .map_err(|error| format!("EVALUATOR_ROOT_UNAVAILABLE: {error}"))?;
    if candidate_root == evaluator_root
        || candidate_root.starts_with(&evaluator_root)
        || evaluator_root.starts_with(&candidate_root)
    {
        return Err("CANDIDATE_AND_EVALUATOR_ROOTS_MUST_BE_DISJOINT".to_owned());
    }

    let candidate_manifest_bytes =
        read_confined(&candidate_root, "candidate_manifest.json")?;
    let evaluator_manifest_bytes =
        read_confined(&evaluator_root, "evaluator_manifest.json")?;
    let custody_seal_bytes =
        read_confined(&evaluator_root, "campaign-custody-seal.json")?;
    let candidate_manifest: CandidateBlindManifest =
        parse_json(&candidate_manifest_bytes, "CANDIDATE_MANIFEST")?;
    let evaluator_manifest: EvaluatorBlindManifest =
        parse_json(&evaluator_manifest_bytes, "EVALUATOR_MANIFEST")?;
    let custody_seal: CampaignCustodySeal =
        parse_json(&custody_seal_bytes, "CAMPAIGN_CUSTODY_SEAL")?;

    verify_post_run_reveal(&candidate_manifest, &evaluator_manifest)
        .map_err(|error| format!("BLIND_REVEAL_VERIFICATION_FAILED: {error}"))?;
    let candidate_manifest_sha256 = sha256_hex(&candidate_manifest_bytes);
    let evaluator_manifest_sha256 = sha256_hex(&evaluator_manifest_bytes);
    let custody_seal_sha256 = sha256_hex(&custody_seal_bytes);
    if custody_seal.schema_version != "aerb-blind-campaign-custody-v0.1"
        || custody_seal.campaign_id != candidate_manifest.campaign_id
        || custody_seal.candidate_manifest_sha256 != candidate_manifest_sha256
        || custody_seal.evaluator_manifest_sha256 != evaluator_manifest_sha256
        || custody_commitment(&custody_seal)? != custody_seal.custody_commitment_sha256
    {
        return Err("CAMPAIGN_CUSTODY_SEAL_VERIFICATION_FAILED".to_owned());
    }

    let candidate_ready_bytes = read_confined(&candidate_root, "READY.json")?;
    let evaluator_ready_bytes = read_confined(&evaluator_root, "READY.json")?;
    let candidate_ready: ReadyMarker = parse_json(&candidate_ready_bytes, "CANDIDATE_READY")?;
    let evaluator_ready: ReadyMarker = parse_json(&evaluator_ready_bytes, "EVALUATOR_READY")?;
    for (marker, manifest_sha256) in [
        (&candidate_ready, &candidate_manifest_sha256),
        (&evaluator_ready, &evaluator_manifest_sha256),
    ] {
        if marker.schema_version != READY_SCHEMA
            || marker.campaign_id != candidate_manifest.campaign_id
            || marker.manifest_sha256.as_str() != manifest_sha256.as_str()
            || marker.custody_commitment_sha256 != custody_seal.custody_commitment_sha256
        {
            return Err("CAMPAIGN_READY_MARKER_MISMATCH".to_owned());
        }
    }

    let scenario_count = candidate_manifest.episode_ids.len();
    if scenario_count == 0 || scenario_count != evaluator_manifest.assignments.len() {
        return Err("BLIND_MANIFEST_EPISODE_COUNT_MISMATCH".to_owned());
    }
    let assignment_ids = evaluator_manifest
        .assignments
        .iter()
        .map(|assignment| assignment.episode_id.clone())
        .collect::<Vec<_>>();
    if assignment_ids != candidate_manifest.episode_ids {
        return Err("BLIND_MANIFEST_ASSIGNMENT_ORDER_MISMATCH".to_owned());
    }

    let candidate_seals = candidate_artifact_map(&custody_seal.candidate_artifacts)?;
    let evaluator_seals = artifact_map(&custody_seal.evaluator_artifacts)?;
    if candidate_seals.len() != scenario_count * 2
        || evaluator_seals.len() != scenario_count * 4 + 2
    {
        return Err("CAMPAIGN_CUSTODY_ARTIFACT_COUNT_MISMATCH".to_owned());
    }

    let mut expected_candidate_files = BTreeSet::from([
        "candidate_manifest.json".to_owned(),
        "READY.json".to_owned(),
    ]);
    let mut expected_evaluator_files = BTreeSet::from([
        "evaluator_manifest.json".to_owned(),
        "projection-secret.json".to_owned(),
        "metadata_attack_input.json".to_owned(),
        "campaign-custody-seal.json".to_owned(),
        "READY.json".to_owned(),
    ]);
    for assignment in &evaluator_manifest.assignments {
        expected_candidate_files.insert(format!("episode-{}.json", assignment.episode_id));
        expected_candidate_files.insert(format!("submission-{}.json", assignment.episode_id));
        expected_evaluator_files.insert(format!("world-pack-{}.json", assignment.episode_id));
        expected_evaluator_files.insert(format!("projection-receipt-{}.json", assignment.episode_id));
        expected_evaluator_files.insert(format!("source-episode-{}.json", assignment.episode_id));
        expected_evaluator_files.insert(format!("source-submission-{}.json", assignment.episode_id));
    }
    verify_directory_entries(&candidate_root, &expected_candidate_files)?;
    verify_directory_entries(&evaluator_root, &expected_evaluator_files)?;

    let projection_secret_bytes = read_hash_sealed(
        &evaluator_root,
        "projection-secret.json",
        &custody_seal.projection_secret_sha256,
    )?;
    let projection_secret: BlindProjectionSecretV0_1 =
        parse_json(&projection_secret_bytes, "PROJECTION_SECRET")?;
    let projection_context = BlindProjectionContext::from_evaluator_secret(&projection_secret)
        .map_err(|error| format!("PROJECTION_SECRET_INVALID: {error}"))?;

    let metadata_input_bytes = read_hash_sealed(
        &evaluator_root,
        "metadata_attack_input.json",
        &custody_seal.metadata_attack_input_sha256,
    )?;
    let metadata_input: MetadataAuditInput =
        parse_json(&metadata_input_bytes, "METADATA_ATTACK_INPUT")?;
    if metadata_input.schema_version != METADATA_INPUT_SCHEMA
        || metadata_input.campaign_id != candidate_manifest.campaign_id
        || metadata_input.rows.len() != scenario_count
    {
        return Err("METADATA_ATTACK_INPUT_CAMPAIGN_MISMATCH".to_owned());
    }
    let metadata_by_episode = metadata_input
        .rows
        .iter()
        .map(|row| (row.episode_id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    if metadata_by_episode.len() != scenario_count {
        return Err("METADATA_ATTACK_INPUT_EPISODE_ID_DUPLICATE".to_owned());
    }

    let mut invalid = Vec::new();
    let mut evaluated_n = 0_usize;
    let mut candidate_contract_valid_n = 0_usize;
    let mut hidden_truth_covered_n = 0_usize;
    let mut residual_retained_n = 0_usize;
    let mut disposition_admissible_n = 0_usize;
    let mut unsafe_fresh_n = 0_usize;
    let mut worst_loss_sum = 0.0_f64;
    let mut truth_loss_sum = 0.0_f64;
    let mut world_strata = BTreeMap::<String, DecisionStratum>::new();
    let mut source_cluster_strata = BTreeMap::<String, DecisionStratum>::new();

    for (file_order_index, assignment) in evaluator_manifest.assignments.iter().enumerate() {
        let episode_id = &assignment.episode_id;
        let episode_name = format!("episode-{episode_id}.json");
        let submission_name = format!("submission-{episode_id}.json");
        let world_pack_name = format!("world-pack-{episode_id}.json");
        let receipt_name = format!("projection-receipt-{episode_id}.json");
        let source_episode_name = format!("source-episode-{episode_id}.json");
        let source_submission_name = format!("source-submission-{episode_id}.json");

        let episode_result = (|| -> Result<
            (EvaluatorReport, bool, String),
            String,
        > {
            let episode_seal = candidate_seals
                .get(&episode_name)
                .ok_or_else(|| "CANDIDATE_EPISODE_SEAL_MISSING".to_owned())?;
            let submission_seal = candidate_seals
                .get(&submission_name)
                .ok_or_else(|| "CANDIDATE_SUBMISSION_SEAL_MISSING".to_owned())?;
            if episode_seal.0.as_str() != episode_id.as_str() || submission_seal.0.as_str() != episode_id.as_str() {
                return Err("CANDIDATE_ARTIFACT_EPISODE_ID_MISMATCH".to_owned());
            }
            let episode_bytes = read_hash_sealed(&candidate_root, &episode_name, &episode_seal.1)?;
            let submission_bytes =
                read_hash_sealed(&candidate_root, &submission_name, &submission_seal.1)?;
            let world_pack_sha = evaluator_seals
                .get(&world_pack_name)
                .ok_or_else(|| "EVALUATOR_WORLD_PACK_SEAL_MISSING".to_owned())?;
            let receipt_sha = evaluator_seals
                .get(&receipt_name)
                .ok_or_else(|| "EVALUATOR_PROJECTION_RECEIPT_SEAL_MISSING".to_owned())?;
            let source_episode_sha = evaluator_seals
                .get(&source_episode_name)
                .ok_or_else(|| "EVALUATOR_SOURCE_EPISODE_SEAL_MISSING".to_owned())?;
            let source_submission_sha = evaluator_seals
                .get(&source_submission_name)
                .ok_or_else(|| "EVALUATOR_SOURCE_SUBMISSION_SEAL_MISSING".to_owned())?;
            let world_pack_bytes =
                read_hash_sealed(&evaluator_root, &world_pack_name, world_pack_sha)?;
            let receipt_bytes =
                read_hash_sealed(&evaluator_root, &receipt_name, receipt_sha)?;
            let source_episode_bytes =
                read_hash_sealed(&evaluator_root, &source_episode_name, source_episode_sha)?;
            let source_submission_bytes =
                read_hash_sealed(&evaluator_root, &source_submission_name, source_submission_sha)?;

            let episode_json = String::from_utf8(episode_bytes.clone())
                .map_err(|error| format!("CANDIDATE_EPISODE_NOT_UTF8: {error}"))?;
            let submission_json = String::from_utf8(submission_bytes.clone())
                .map_err(|error| format!("CANDIDATE_SUBMISSION_NOT_UTF8: {error}"))?;
            let world_pack_json = String::from_utf8(world_pack_bytes.clone())
                .map_err(|error| format!("EVALUATOR_WORLD_PACK_NOT_UTF8: {error}"))?;
            let original_episode: CandidateEpisode = parse_json(&source_episode_bytes, "SOURCE_EPISODE")?;
            let original_submission: CandidateSubmission =
                parse_json(&source_submission_bytes, "SOURCE_SUBMISSION")?;
            let projected_episode: CandidateEpisode = parse_json(&episode_bytes, "CANDIDATE_EPISODE")?;
            let projected_submission: CandidateSubmission =
                parse_json(&submission_bytes, "CANDIDATE_SUBMISSION")?;
            let world_pack: EvaluatorWorldPack = parse_json(&world_pack_bytes, "WORLD_PACK")?;
            let receipt: BlindProjectionReceipt = parse_json(&receipt_bytes, "PROJECTION_RECEIPT")?;

            if world_pack.episode_id != *episode_id
                || world_pack.hidden_truth_world_id != assignment.hidden_truth_world_id
            {
                return Err("EVALUATOR_WORLD_PACK_TRUTH_OR_ID_MISMATCH".to_owned());
            }
            if original_episode.episode_id != receipt.original_episode_id
                || original_submission.episode_id != original_episode.episode_id
            {
                return Err("SOURCE_ARCHIVE_AND_PROJECTION_RECEIPT_MISMATCH".to_owned());
            }
            let original_episode_json = serde_json::to_string(&original_episode)
                .map_err(|error| format!("SOURCE_EPISODE_SERIALIZATION_FAILED: {error}"))?;
            let source_snapshot = canonical_sorted_json_sha256(&original_episode_json)
                .map_err(|error| format!("SOURCE_EPISODE_DIGEST_FAILED: {error}"))?;
            if original_submission.candidate_snapshot_sha256 != source_snapshot {
                return Err("SOURCE_SUBMISSION_SNAPSHOT_DIGEST_MISMATCH".to_owned());
            }

            verify_projected_episode(&projected_episode, &receipt)
                .map_err(|error| format!("PROJECTION_RECEIPT_INVALID: {error}"))?;
            let (recomputed_episode, recomputed_receipt) = project_candidate_episode(
                &projection_context,
                &original_episode,
                episode_id,
            )
            .map_err(|error| format!("PROJECTION_REPLAY_FAILED: {error}"))?;
            let recomputed_episode_json = serde_json::to_string(&recomputed_episode)
                .map_err(|error| format!("PROJECTION_REPLAY_SERIALIZATION_FAILED: {error}"))?;
            let projected_episode_sha = canonical_sorted_json_sha256(&episode_json)
                .map_err(|error| format!("PROJECTED_EPISODE_DIGEST_FAILED: {error}"))?;
            let recomputed_episode_sha = canonical_sorted_json_sha256(&recomputed_episode_json)
                .map_err(|error| format!("RECOMPUTED_EPISODE_DIGEST_FAILED: {error}"))?;
            if projected_episode_sha != recomputed_episode_sha
                || recomputed_receipt.projection_receipt_sha256 != receipt.projection_receipt_sha256
            {
                return Err("PROJECTION_REPLAY_MISMATCH".to_owned());
            }

            let recomputed_submission = project_candidate_submission(
                &projection_context,
                &original_submission,
                &original_episode,
                &recomputed_episode,
                &recomputed_receipt,
            )
            .map_err(|error| format!("SUBMISSION_PROJECTION_REPLAY_FAILED: {error}"))?;
            let recomputed_submission_json = serde_json::to_string(&recomputed_submission)
                .map_err(|error| format!("SUBMISSION_REPLAY_SERIALIZATION_FAILED: {error}"))?;
            let projected_submission_sha = canonical_sorted_json_sha256(&submission_json)
                .map_err(|error| format!("PROJECTED_SUBMISSION_DIGEST_FAILED: {error}"))?;
            let recomputed_submission_sha =
                canonical_sorted_json_sha256(&recomputed_submission_json)
                    .map_err(|error| format!("RECOMPUTED_SUBMISSION_DIGEST_FAILED: {error}"))?;
            if projected_submission_sha != recomputed_submission_sha
                || projected_submission.episode_id != *episode_id
            {
                return Err("SUBMISSION_PROJECTION_REPLAY_MISMATCH".to_owned());
            }

            let metadata_row = metadata_by_episode
                .get(episode_id.as_str())
                .ok_or_else(|| "METADATA_AUDIT_ROW_MISSING".to_owned())?;
            if metadata_row.campaign_id != candidate_manifest.campaign_id
                || metadata_row.hidden_truth_world_id != assignment.hidden_truth_world_id
                || metadata_row.evaluator_cluster_id != receipt.original_trajectory_cluster_id
            {
                return Err("METADATA_AUDIT_TRUTH_OR_CLUSTER_MISMATCH".to_owned());
            }
            let expected_features = candidate_metadata_features(
                &projected_episode,
                file_order_index,
                &episode_name,
                episode_bytes.len(),
            );
            if metadata_row.candidate_visible_features != expected_features {
                return Err("METADATA_ATTACK_FEATURES_DO_NOT_MATCH_CANDIDATE_BYTES".to_owned());
            }

            let report = evaluate_json(&episode_json, &submission_json, &world_pack_json)
                .map_err(|error| format!("EPISODE_EVALUATION_FAILED: {error}"))?;
            let unsafe_fresh =
                unsafe_fresh_recommendation(&projected_submission, &assignment.hidden_truth_world_id);
            Ok((report, unsafe_fresh, receipt.original_trajectory_cluster_id))
        })();

        match episode_result {
            Ok((report, unsafe_fresh, source_cluster_id)) => {
                evaluated_n += 1;
                candidate_contract_valid_n += usize::from(report.candidate_contract_valid);
                hidden_truth_covered_n += usize::from(report.hidden_truth_in_candidate_support);
                residual_retained_n += usize::from(report.open_world_residual_retained);
                disposition_admissible_n += usize::from(report.selected_disposition_admissible);
                unsafe_fresh_n += usize::from(unsafe_fresh);
                worst_loss_sum += report.selected_disposition_worst_case_loss.unwrap_or(1.0);
                truth_loss_sum += report.evaluator_truth_disposition_loss;
                record_stratum(
                    world_strata
                        .entry(assignment.hidden_truth_world_id.clone())
                        .or_default(),
                    &report,
                    unsafe_fresh,
                );
                record_stratum(
                    source_cluster_strata.entry(source_cluster_id).or_default(),
                    &report,
                    unsafe_fresh,
                );
            }
            Err(reason) => invalid.push(InvalidEpisode {
                episode_id: episode_id.clone(),
                reason,
            }),
        }
    }

    let verified_metadata_rows = metadata_input
        .rows
        .iter()
        .map(|row| row.clone())
        .collect::<Vec<_>>();
    let metadata_leakage = audit_metadata_leakage(
        &verified_metadata_rows,
        metadata_input.requested_folds,
    )
    .map_err(|error| format!("METADATA_LEAKAGE_AUDIT_FAILED: {error}"))?;

    let scheduled = scenario_count as f64;
    let evaluated_denominator = evaluated_n.max(1) as f64;
    Ok(BlindCampaignReport {
        schema_version: "aerb-blind-campaign-report-v0.1".to_owned(),
        benchmark: "AERB".to_owned(),
        campaign_id: candidate_manifest.campaign_id,
        candidate_manifest_sha256,
        evaluator_manifest_sha256,
        custody_seal_sha256,
        scheduled_n: scenario_count,
        evaluated_n,
        structurally_invalid_n: invalid.len(),
        candidate_contract_valid_n,
        candidate_contract_invalid_n: evaluated_n - candidate_contract_valid_n,
        candidate_contract_valid_rate_of_scheduled: candidate_contract_valid_n as f64 / scheduled,
        hidden_truth_covered_n,
        hidden_truth_support_coverage_rate_of_scheduled: hidden_truth_covered_n as f64 / scheduled,
        open_world_residual_retained_n: residual_retained_n,
        open_world_residual_retention_rate_of_scheduled: residual_retained_n as f64 / scheduled,
        disposition_admissible_n: disposition_admissible_n,
        disposition_admissibility_rate_of_scheduled: disposition_admissible_n as f64 / scheduled,
        unsafe_fresh_attempt_n: unsafe_fresh_n,
        mean_selected_worst_case_loss: worst_loss_sum / evaluated_denominator,
        mean_evaluator_truth_loss: truth_loss_sum / evaluated_denominator,
        world_strata,
        source_cluster_strata,
        metadata_leakage,
        invalid_episodes: invalid,
        statistical_interpretation: "Blind evaluator-only software/process-model campaign. Per-episode counts preserve the scheduled denominator; structural invalids remain explicit. Metadata attack is a grouped categorical diagnostic, not proof of no inferential leakage. Truth-class counts are inherited from the source corpus and are not event probabilities.".to_owned(),
        claim_ceiling: "Synthetic/process-model evidence only. No simulator, HIL, physical, calibrated-risk, causal-identification, certification or SOTA claim.".to_owned(),
    })
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(
            "usage: aerb_blind_campaign_evaluate <candidate-directory> <evaluator-directory> <evaluator-report.json>"
                .to_owned(),
        );
    }
    let candidate_root = PathBuf::from(&args[0]);
    let evaluator_root = PathBuf::from(&args[1]);
    let report_path = PathBuf::from(&args[2]);
    let report_parent = report_path
        .parent()
        .ok_or_else(|| "REPORT_PARENT_MISSING".to_owned())?
        .canonicalize()
        .map_err(|error| format!("REPORT_PARENT_UNAVAILABLE: {error}"))?;
    let canonical_evaluator = evaluator_root
        .canonicalize()
        .map_err(|error| format!("EVALUATOR_ROOT_UNAVAILABLE: {error}"))?;
    if report_parent != canonical_evaluator {
        return Err("EVALUATOR_REPORT_MUST_REMAIN_INSIDE_EVALUATOR_CUSTODY_ROOT".to_owned());
    }
    let report = campaign_report(&candidate_root, &evaluator_root)?;
    write_report(&canonical_evaluator, &report_path, &report)?;
    println!(
        "AERB blind evaluator: scheduled={} evaluated={} structurally_invalid={} contract_valid={} unsafe_fresh={} metadata_only_balanced_accuracy={:.4}",
        report.scheduled_n,
        report.evaluated_n,
        report.structurally_invalid_n,
        report.candidate_contract_valid_n,
        report.unsafe_fresh_attempt_n,
        report.metadata_leakage.combined_metadata_only_balanced_accuracy
    );
    println!("Evaluator-only report is sealed to the campaign custody root; no physical qualification claimed.");
    if report.structurally_invalid_n > 0
        || report.candidate_contract_valid_n != report.scheduled_n
        || report.unsafe_fresh_attempt_n > 0
    {
        return Err("BLIND_CAMPAIGN_QUALIFICATION_GATE_FAILED".to_owned());
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_BLIND_CAMPAIGN_EVALUATION_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluator_leaf_names_reject_path_traversal() {
        assert!(safe_leaf_name("episode-ep_00000000000000000000000000000000.json").is_ok());
        assert!(safe_leaf_name("../projection-secret.json").is_err());
        assert!(safe_leaf_name("nested/episode.json").is_err());
    }

    #[test]
    fn unsafe_retry_is_defined_as_a_new_attempt_with_nonzero_possible_effect() {
        use mobility_rrvr_aerb_evaluator::Disposition;
        let mut submission = CandidateSubmission {
            schema_version: "0.1".to_owned(),
            benchmark: "AERB".to_owned(),
            episode_id: "ep_00000000000000000000000000000000".to_owned(),
            candidate_snapshot_sha256: "a".repeat(64),
            model_sha256: "b".repeat(64),
            observer_profile_sha256: "c".repeat(64),
            possible_world_support_ids: vec![],
            disposition: Disposition::RecommendFreshAttempt,
            requested_probe_id: None,
            reason_codes: vec![],
            explanation_observation_ids: vec![],
        };
        assert!(unsafe_fresh_recommendation(&submission, "applied_once_ack_lost"));
        assert!(!unsafe_fresh_recommendation(&submission, "dispatched_no_effect"));
        assert!(!unsafe_fresh_recommendation(&submission, "not_dispatched"));
        submission.disposition = Disposition::Abstain;
        assert!(!unsafe_fresh_recommendation(&submission, "applied_once_ack_lost"));
    }

    #[test]
    fn custody_commitment_changes_with_input_digest() {
        let mut seal = CampaignCustodySeal {
            schema_version: "aerb-blind-campaign-custody-v0.1".to_owned(),
            campaign_id: "camp_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            source_candidate_manifest_sha256: "a".repeat(64),
            source_evaluator_manifest_sha256: "b".repeat(64),
            candidate_manifest_sha256: "c".repeat(64),
            evaluator_manifest_sha256: "d".repeat(64),
            projection_secret_sha256: "e".repeat(64),
            candidate_artifacts: vec![],
            evaluator_artifacts: vec![],
            metadata_attack_input_sha256: "f".repeat(64),
            custody_commitment_sha256: String::new(),
        };
        let first = custody_commitment(&seal).expect("commitment");
        seal.metadata_attack_input_sha256 = "0".repeat(64);
        let second = custody_commitment(&seal).expect("changed commitment");
        assert_ne!(first, second);
    }
}
