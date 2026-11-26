// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Component, Path},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "0.1";
pub const BENCHMARK: &str = "RRVR-REPLAY-PROVENANCE";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    RoboTwin,
    Libero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Split {
    Train,
    Test,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayManifest {
    schema_version: String,
    benchmark: String,
    profile_id: String,
    metric_profile_id: String,
    campaign_id: String,
    split_manifest_sha256: String,
    data_license_and_provenance: String,
    candidates: Vec<ReplayCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayCandidate {
    scenario_id: String,
    platform: Platform,
    split: Split,
    source_episode_sha256: String,
    source_trajectory_id: String,
    trajectory_cluster_id: String,
    instruction_sha256: String,
    action_prefix_sha256: String,
    split_leakage_keys_sha256: Vec<String>,
    simulator_profile_sha256: String,
    robot_model_profile_sha256: String,
    controller_profile_sha256: String,
    rendering_profile_sha256: String,
    reset_receipt_sha256: String,
    action_prefix_receipt_sha256: String,
    difficulty_screen: DifficultyScreen,
    spot_check: SpotCheck,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DifficultyScreen {
    trials: Vec<DifficultyTrial>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct DifficultyTrial {
    attempt_id: String,
    policy_id: String,
    trial_index: u8,
    success: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SpotCheck {
    seed: u64,
    selection_method_id: String,
    selection_record_path: String,
    selection_record_sha256: String,
    selected_attempt_ids: Vec<String>,
    attempts: Vec<SpotCheckAttempt>,
    manifest_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct SpotCheckAttempt {
    attempt_id: String,
    original_state_path: String,
    original_state_sha256: String,
    replay_state_path: String,
    replay_state_sha256: String,
    final_frame_pairs: Vec<FramePairAsset>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FramePairAsset {
    frame_index: u8,
    original_path: String,
    original_sha256: String,
    replay_path: String,
    replay_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SpotCheckSelectionRecord {
    schema_version: String,
    seed: u64,
    selection_method_id: String,
    selected_attempt_ids: Vec<String>,
}

#[derive(Serialize)]
struct SpotCheckManifestMaterial<'a> {
    seed: u64,
    selection_method_id: &'a str,
    selection_record_path: &'a str,
    selection_record_sha256: &'a str,
    selected_attempt_ids: &'a [String],
    attempts: &'a [SpotCheckAttempt],
}

fn spot_check_manifest_digest(spot: &SpotCheck) -> Result<String, String> {
    let material = SpotCheckManifestMaterial {
        seed: spot.seed,
        selection_method_id: &spot.selection_method_id,
        selection_record_path: &spot.selection_record_path,
        selection_record_sha256: &spot.selection_record_sha256,
        selected_attempt_ids: &spot.selected_attempt_ids,
        attempts: &spot.attempts,
    };
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("SPOT_CHECK_MANIFEST_SERIALIZATION_FAILED: {error}"))?;
    Ok(sha256_hex(&bytes))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PositionMm {
    coordinate_frame_id: String,
    x_mm: f64,
    y_mm: f64,
    z_mm: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayStateSnapshotArtifact {
    schema_version: String,
    scenario_id: String,
    attempt_id: String,
    simulator_profile_sha256: String,
    end_effector_position: PositionMm,
    object_position: PositionMm,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub relative_path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceManifest {
    pub schema_version: String,
    pub benchmark: String,
    pub replay_manifest_sha256: String,
    pub split_manifest: ArtifactRef,
    pub policy_roster_profile_id: String,
    pub candidates: Vec<ProvenanceCandidate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceCandidate {
    pub scenario_id: String,
    pub platform: Platform,
    pub split: Split,
    pub source_trajectory_id: String,
    pub trajectory_cluster_id: String,
    pub execution_scope_id: String,
    pub source_episode: ArtifactRef,
    pub instruction: ArtifactRef,
    /// Index artifact containing the distinct raw action-prefix artifact for
    /// each of the three selected replay attempts.
    pub action_prefix_index: ArtifactRef,
    pub simulator_profile: ArtifactRef,
    pub robot_model_profile: ArtifactRef,
    pub controller_profile: ArtifactRef,
    pub rendering_profile: ArtifactRef,
    pub policy_roster: ArtifactRef,
    /// Bundle receipt covering the three selected reset attempts.
    pub reset_receipt_bundle: ArtifactRef,
    /// Bundle receipt covering each selected attempt's action-prefix replay.
    pub action_prefix_receipt_bundle: ArtifactRef,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEpisodeArtifact {
    schema_version: String,
    artifact_kind: String,
    scenario_id: String,
    platform: Platform,
    split: Split,
    source_trajectory_id: String,
    execution_scope_id: String,
    instruction_sha256: String,
    action_prefix_index_sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProfileArtifact {
    schema_version: String,
    artifact_kind: String,
    profile_id: String,
    execution_scope_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PolicyRosterArtifact {
    schema_version: String,
    artifact_kind: String,
    policy_roster_id: String,
    policy_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionPrefixIndexArtifact {
    schema_version: String,
    artifact_kind: String,
    scenario_id: String,
    execution_scope_id: String,
    attempts: Vec<IndexedActionPrefix>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexedActionPrefix {
    attempt_id: String,
    policy_id: String,
    action_prefix: ArtifactRef,
    action_step_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetReceiptBundle {
    schema_version: String,
    receipt_type: String,
    scenario_id: String,
    execution_scope_id: String,
    source_episode_sha256: String,
    simulator_profile_sha256: String,
    robot_model_profile_sha256: String,
    attempts: Vec<ResetAttemptReceipt>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResetAttemptReceipt {
    attempt_id: String,
    policy_id: String,
    status: String,
    initial_state_sha256: String,
    reset_generation: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionPrefixReceiptBundle {
    schema_version: String,
    receipt_type: String,
    scenario_id: String,
    execution_scope_id: String,
    source_episode_sha256: String,
    instruction_sha256: String,
    action_prefix_index_sha256: String,
    reset_receipt_sha256: String,
    simulator_profile_sha256: String,
    robot_model_profile_sha256: String,
    controller_profile_sha256: String,
    rendering_profile_sha256: String,
    attempts: Vec<ActionPrefixAttemptReceipt>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionPrefixAttemptReceipt {
    attempt_id: String,
    policy_id: String,
    action_prefix_sha256: String,
    action_step_count: u64,
    command_id: String,
    status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateProvenanceReport {
    pub scenario_id: String,
    pub status: String,
    pub selected_attempt_n: usize,
    pub verified_artifact_n: usize,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvenanceReport {
    pub schema_version: String,
    pub benchmark: String,
    pub replay_manifest_sha256: String,
    pub candidate_n: usize,
    pub valid_n: usize,
    pub invalid_n: usize,
    pub candidate_reports: Vec<CandidateProvenanceReport>,
    pub claim_ceiling: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn read_artifact(
    root: &Path,
    reference: &ArtifactRef,
    label: &str,
) -> Result<Vec<u8>, String> {
    if !is_sha256(&reference.sha256) {
        return Err(format!("{label}_DIGEST_SYNTAX_INVALID"));
    }
    let relative = Path::new(&reference.relative_path);
    if reference.relative_path.trim().is_empty() || relative.is_absolute()
        || relative.components().any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("{label}_PATH_UNSAFE"));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|e| format!("ARTIFACT_ROOT_UNAVAILABLE: {e}"))?;
    let candidate = canonical_root.join(relative);
    let canonical_path = candidate
        .canonicalize()
        .map_err(|e| format!("{label}_ARTIFACT_UNAVAILABLE: {e}"))?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(format!("{label}_PATH_ESCAPES_ROOT"));
    }
    let bytes = fs::read(canonical_path)
        .map_err(|e| format!("{label}_ARTIFACT_READ_FAILED: {e}"))?;
    if sha256_hex(&bytes) != reference.sha256 {
        return Err(format!("{label}_RAW_DIGEST_MISMATCH"));
    }
    Ok(bytes)
}

fn parse_artifact<T: for<'de> Deserialize<'de>>(
    root: &Path,
    reference: &ArtifactRef,
    label: &str,
) -> Result<T, String> {
    let bytes = read_artifact(root, reference, label)?;
    serde_json::from_slice(&bytes).map_err(|e| format!("{label}_JSON_INVALID: {e}"))
}

fn assert_eq_field<T: PartialEq + std::fmt::Debug>(
    actual: T,
    expected: T,
    code: &str,
) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{code}: expected {expected:?}, got {actual:?}"))
    }
}

fn validate_state_snapshot_artifact(
    root: &Path,
    relative_path: &str,
    expected_sha256: &str,
    scenario_id: &str,
    attempt_id: &str,
    simulator_profile_sha256: &str,
    label: &str,
) -> Result<String, String> {
    let reference = ArtifactRef {
        relative_path: relative_path.to_owned(),
        sha256: expected_sha256.to_owned(),
    };
    let snapshot: ReplayStateSnapshotArtifact =
        parse_artifact(root, &reference, label)?;
    assert_eq_field(
        snapshot.schema_version,
        SCHEMA_VERSION.to_owned(),
        "STATE_ARTIFACT_SCHEMA_VERSION_MISMATCH",
    )?;
    assert_eq_field(
        snapshot.scenario_id,
        scenario_id.to_owned(),
        "STATE_ARTIFACT_SCENARIO_MISMATCH",
    )?;
    assert_eq_field(
        snapshot.attempt_id,
        attempt_id.to_owned(),
        "STATE_ARTIFACT_ATTEMPT_MISMATCH",
    )?;
    assert_eq_field(
        snapshot.simulator_profile_sha256,
        simulator_profile_sha256.to_owned(),
        "STATE_ARTIFACT_SIMULATOR_PROFILE_MISMATCH",
    )?;
    let end_effector_frame = &snapshot.end_effector_position.coordinate_frame_id;
    let object_frame = &snapshot.object_position.coordinate_frame_id;
    if end_effector_frame.trim().is_empty()
        || object_frame.trim().is_empty()
        || end_effector_frame != object_frame
    {
        return Err("STATE_ARTIFACT_COORDINATE_FRAME_INVALID".to_owned());
    }
    for position in [&snapshot.end_effector_position, &snapshot.object_position] {
        if ![
            position.x_mm,
            position.y_mm,
            position.z_mm,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err("STATE_ARTIFACT_POSITION_INVALID".to_owned());
        }
    }
    Ok(end_effector_frame.clone())
}

fn validate_profile(
    root: &Path,
    reference: &ArtifactRef,
    expected_kind: &str,
    scope_id: &str,
) -> Result<(), String> {
    let profile: ProfileArtifact = parse_artifact(root, reference, expected_kind)?;
    assert_eq_field(profile.schema_version, SCHEMA_VERSION.to_owned(), "PROFILE_SCHEMA_VERSION_MISMATCH")?;
    assert_eq_field(profile.artifact_kind, expected_kind.to_owned(), "PROFILE_KIND_MISMATCH")?;
    if profile.profile_id.trim().is_empty() {
        return Err("PROFILE_ID_MISSING".to_owned());
    }
    assert_eq_field(profile.execution_scope_id, scope_id.to_owned(), "PROFILE_SCOPE_MISMATCH")
}

fn policy_map(candidate: &ReplayCandidate) -> Result<BTreeMap<String, String>, String> {
    if candidate.difficulty_screen.trials.len() != 20 {
        return Err("DIFFICULTY_SCREEN_NOT_20_TRIALS".to_owned());
    }
    let mut attempts = BTreeMap::new();
    for trial in &candidate.difficulty_screen.trials {
        if trial.attempt_id.trim().is_empty() || trial.policy_id.trim().is_empty() || trial.trial_index > 4 {
            return Err("DIFFICULTY_TRIAL_IDENTITY_INVALID".to_owned());
        }
        if attempts.insert(trial.attempt_id.clone(), trial.policy_id.clone()).is_some() {
            return Err("DIFFICULTY_ATTEMPT_ID_DUPLICATED".to_owned());
        }
    }
    let success_n = candidate.difficulty_screen.trials.iter().filter(|row| row.success).count();
    if !(2..=18).contains(&success_n) {
        return Err("DIFFICULTY_SCREEN_SUCCESS_COUNT_OUT_OF_RANGE".to_owned());
    }
    let policies: BTreeSet<&str> = candidate
        .difficulty_screen
        .trials
        .iter()
        .map(|row| row.policy_id.as_str())
        .collect();
    if policies.len() != 4 {
        return Err("DIFFICULTY_POLICY_ROSTER_NOT_FOUR".to_owned());
    }
    for policy in policies {
        let indices: BTreeSet<u8> = candidate
            .difficulty_screen
            .trials
            .iter()
            .filter(|row| row.policy_id == policy)
            .map(|row| row.trial_index)
            .collect();
        if indices != (0..5).collect() {
            return Err(format!("DIFFICULTY_POLICY_TRIAL_INDEX_INVALID: {policy}"));
        }
    }
    Ok(attempts)
}

fn validate_replay_candidate_set(candidates: &[ReplayCandidate]) -> Result<(), String> {
    if candidates.is_empty() {
        return Err("NO_REPLAY_CANDIDATES".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut keys_by_platform_and_split: BTreeMap<(&'static str, &'static str), BTreeSet<&str>> =
        BTreeMap::new();

    for candidate in candidates {
        if candidate.scenario_id.trim().is_empty()
            || candidate.source_trajectory_id.trim().is_empty()
            || candidate.trajectory_cluster_id.trim().is_empty()
        {
            return Err("CANDIDATE_IDENTITY_INCOMPLETE".to_owned());
        }
        let platform_id = match candidate.platform {
            Platform::RoboTwin => "robo_twin",
            Platform::Libero => "libero",
        };
        if !scenario_ids.insert((platform_id, candidate.scenario_id.as_str())) {
            return Err(format!("DUPLICATE_SCENARIO_ID: {}", candidate.scenario_id));
        }

        let digest_fields = [
            candidate.source_episode_sha256.as_str(),
            candidate.instruction_sha256.as_str(),
            candidate.action_prefix_sha256.as_str(),
            candidate.simulator_profile_sha256.as_str(),
            candidate.robot_model_profile_sha256.as_str(),
            candidate.controller_profile_sha256.as_str(),
            candidate.rendering_profile_sha256.as_str(),
            candidate.reset_receipt_sha256.as_str(),
            candidate.action_prefix_receipt_sha256.as_str(),
        ];
        if digest_fields.iter().any(|digest| !is_sha256(digest)) {
            return Err(format!("CANDIDATE_DIGEST_INVALID: {}", candidate.scenario_id));
        }

        if candidate.split_leakage_keys_sha256.is_empty() {
            return Err(format!("SPLIT_LEAKAGE_KEYS_MISSING: {}", candidate.scenario_id));
        }
        let mut local_keys = BTreeSet::new();
        for key in &candidate.split_leakage_keys_sha256 {
            if !is_sha256(key) || !local_keys.insert(key.as_str()) {
                return Err(format!("SPLIT_LEAKAGE_KEYS_INVALID_OR_DUPLICATED: {}", candidate.scenario_id));
            }
            let split_id = match candidate.split {
                Split::Train => "train",
                Split::Test => "test",
            };
            keys_by_platform_and_split
                .entry((platform_id, split_id))
                .or_default()
                .insert(key.as_str());
        }
    }

    for ((platform_id, split_id), keys) in &keys_by_platform_and_split {
        let opposite_id = if *split_id == "train" { "test" } else { "train" };
        if let Some(opposite) = keys_by_platform_and_split.get(&(*platform_id, opposite_id)) {
            if let Some(overlap) = keys.intersection(opposite).next() {
                return Err(format!("TRAIN_TEST_LEAKAGE_KEY_OVERLAP: {overlap}"));
            }
        }
    }
    Ok(())
}

fn validate_candidate(
    replay: &ReplayCandidate,
    provenance: &ProvenanceCandidate,
    policy_roster_profile_id: &str,
    root: &Path,
) -> Result<usize, String> {
    assert_eq_field(replay.scenario_id.clone(), provenance.scenario_id.clone(), "SCENARIO_ID_MISMATCH")?;
    assert_eq_field(replay.platform, provenance.platform, "PLATFORM_MISMATCH")?;
    assert_eq_field(replay.split, provenance.split, "SPLIT_MISMATCH")?;
    assert_eq_field(
        replay.source_trajectory_id.clone(),
        provenance.source_trajectory_id.clone(),
        "SOURCE_TRAJECTORY_ID_MISMATCH",
    )?;
    assert_eq_field(
        replay.trajectory_cluster_id.clone(),
        provenance.trajectory_cluster_id.clone(),
        "TRAJECTORY_CLUSTER_ID_MISMATCH",
    )?;
    if replay.split_leakage_keys_sha256.is_empty() {
        return Err("SPLIT_LEAKAGE_KEYS_MISSING".to_owned());
    }
    let mut leakage_keys = BTreeSet::new();
    for key in &replay.split_leakage_keys_sha256 {
        if !is_sha256(key) || !leakage_keys.insert(key.as_str()) {
            return Err("SPLIT_LEAKAGE_KEYS_INVALID_OR_DUPLICATED".to_owned());
        }
    }
    if provenance.execution_scope_id.trim().is_empty() {
        return Err("EXECUTION_SCOPE_ID_MISSING".to_owned());
    }

    let source_episode_bytes = read_artifact(root, &provenance.source_episode, "SOURCE_EPISODE")?;
    assert_eq_field(
        sha256_hex(&source_episode_bytes),
        replay.source_episode_sha256.clone(),
        "SOURCE_EPISODE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    let source: SourceEpisodeArtifact = serde_json::from_slice(&source_episode_bytes)
        .map_err(|e| format!("SOURCE_EPISODE_JSON_INVALID: {e}"))?;
    assert_eq_field(source.schema_version, SCHEMA_VERSION.to_owned(), "SOURCE_EPISODE_SCHEMA_VERSION_MISMATCH")?;
    assert_eq_field(source.artifact_kind, "source_episode".to_owned(), "SOURCE_EPISODE_KIND_MISMATCH")?;
    assert_eq_field(source.scenario_id, replay.scenario_id.clone(), "SOURCE_EPISODE_SCENARIO_MISMATCH")?;
    assert_eq_field(source.platform, replay.platform, "SOURCE_EPISODE_PLATFORM_MISMATCH")?;
    assert_eq_field(source.split, replay.split, "SOURCE_EPISODE_SPLIT_MISMATCH")?;
    assert_eq_field(
        source.source_trajectory_id,
        replay.source_trajectory_id.clone(),
        "SOURCE_EPISODE_TRAJECTORY_MISMATCH",
    )?;
    assert_eq_field(
        source.execution_scope_id,
        provenance.execution_scope_id.clone(),
        "SOURCE_EPISODE_SCOPE_MISMATCH",
    )?;

    let instruction_bytes = read_artifact(root, &provenance.instruction, "INSTRUCTION")?;
    assert_eq_field(
        sha256_hex(&instruction_bytes),
        replay.instruction_sha256.clone(),
        "INSTRUCTION_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    assert_eq_field(
        source.instruction_sha256,
        replay.instruction_sha256.clone(),
        "SOURCE_EPISODE_INSTRUCTION_DIGEST_MISMATCH",
    )?;

    let action_index_bytes = read_artifact(root, &provenance.action_prefix_index, "ACTION_PREFIX_INDEX")?;
    assert_eq_field(
        sha256_hex(&action_index_bytes),
        replay.action_prefix_sha256.clone(),
        "ACTION_PREFIX_INDEX_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    assert_eq_field(
        source.action_prefix_index_sha256,
        replay.action_prefix_sha256.clone(),
        "SOURCE_EPISODE_ACTION_PREFIX_INDEX_DIGEST_MISMATCH",
    )?;
    let action_index: ActionPrefixIndexArtifact = serde_json::from_slice(&action_index_bytes)
        .map_err(|e| format!("ACTION_PREFIX_INDEX_JSON_INVALID: {e}"))?;
    assert_eq_field(action_index.schema_version, SCHEMA_VERSION.to_owned(), "ACTION_PREFIX_INDEX_SCHEMA_VERSION_MISMATCH")?;
    assert_eq_field(action_index.artifact_kind, "action_prefix_index".to_owned(), "ACTION_PREFIX_INDEX_KIND_MISMATCH")?;
    assert_eq_field(action_index.scenario_id, replay.scenario_id.clone(), "ACTION_PREFIX_INDEX_SCENARIO_MISMATCH")?;
    assert_eq_field(
        action_index.execution_scope_id,
        provenance.execution_scope_id.clone(),
        "ACTION_PREFIX_INDEX_SCOPE_MISMATCH",
    )?;

    validate_profile(root, &provenance.simulator_profile, "simulator_profile", &provenance.execution_scope_id)?;
    assert_eq_field(
        provenance.simulator_profile.sha256.clone(),
        replay.simulator_profile_sha256.clone(),
        "SIMULATOR_PROFILE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    validate_profile(root, &provenance.robot_model_profile, "robot_model_profile", &provenance.execution_scope_id)?;
    assert_eq_field(
        provenance.robot_model_profile.sha256.clone(),
        replay.robot_model_profile_sha256.clone(),
        "ROBOT_MODEL_PROFILE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    validate_profile(root, &provenance.controller_profile, "controller_profile", &provenance.execution_scope_id)?;
    assert_eq_field(
        provenance.controller_profile.sha256.clone(),
        replay.controller_profile_sha256.clone(),
        "CONTROLLER_PROFILE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    validate_profile(root, &provenance.rendering_profile, "rendering_profile", &provenance.execution_scope_id)?;
    assert_eq_field(
        provenance.rendering_profile.sha256.clone(),
        replay.rendering_profile_sha256.clone(),
        "RENDERING_PROFILE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;

    let policy_roster: PolicyRosterArtifact = parse_artifact(root, &provenance.policy_roster, "POLICY_ROSTER")?;
    assert_eq_field(policy_roster.schema_version, SCHEMA_VERSION.to_owned(), "POLICY_ROSTER_SCHEMA_VERSION_MISMATCH")?;
    assert_eq_field(policy_roster.artifact_kind, "policy_roster".to_owned(), "POLICY_ROSTER_KIND_MISMATCH")?;
    if policy_roster.policy_roster_id.trim().is_empty() {
        return Err("POLICY_ROSTER_ID_MISSING".to_owned());
    }
    assert_eq_field(
        policy_roster.policy_roster_id,
        policy_roster_profile_id.to_owned(),
        "POLICY_ROSTER_PROFILE_ID_MISMATCH",
    )?;
    let screen_attempts = policy_map(replay)?;
    let expected_policies: BTreeSet<String> = screen_attempts.values().cloned().collect();
    let actual_policies: BTreeSet<String> = policy_roster.policy_ids.iter().cloned().collect();
    if policy_roster.policy_ids.len() != actual_policies.len() || expected_policies != actual_policies {
        return Err("POLICY_ROSTER_DOES_NOT_MATCH_DIFFICULTY_SCREEN".to_owned());
    }

    let selected = &replay.spot_check.selected_attempt_ids;
    if replay.spot_check.selection_method_id.trim().is_empty()
        || replay.spot_check.attempts.iter().any(|attempt| !is_sha256(&attempt.replay_state_sha256))
        || replay.spot_check.attempts.iter().any(|attempt| !is_sha256(&attempt.original_state_sha256))
    {
        return Err("SPOT_CHECK_SELECTION_OR_STATE_DIGEST_INVALID".to_owned());
    }
    if replay.spot_check.seed != 42
        || selected.len() != 3
        || replay.spot_check.attempts.len() != 3
        || selected.iter().collect::<BTreeSet<_>>().len() != 3
    {
        return Err("SELECTED_ATTEMPT_SET_INVALID".to_owned());
    }
    if !is_sha256(&replay.spot_check.selection_record_sha256)
        || !is_sha256(&replay.spot_check.manifest_sha256)
        || replay.spot_check.selection_record_path.trim().is_empty()
    {
        return Err("SPOT_CHECK_SELECTION_RECEIPT_METADATA_INVALID".to_owned());
    }

    let selection_reference = ArtifactRef {
        relative_path: replay.spot_check.selection_record_path.clone(),
        sha256: replay.spot_check.selection_record_sha256.clone(),
    };
    let selection_record: SpotCheckSelectionRecord =
        parse_artifact(root, &selection_reference, "SPOT_CHECK_SELECTION_RECORD")?;
    assert_eq_field(
        selection_record.schema_version,
        SCHEMA_VERSION.to_owned(),
        "SPOT_CHECK_SELECTION_RECORD_SCHEMA_MISMATCH",
    )?;
    assert_eq_field(
        selection_record.seed,
        replay.spot_check.seed,
        "SPOT_CHECK_SELECTION_RECORD_SEED_MISMATCH",
    )?;
    assert_eq_field(
        selection_record.selection_method_id,
        replay.spot_check.selection_method_id.clone(),
        "SPOT_CHECK_SELECTION_RECORD_METHOD_MISMATCH",
    )?;
    assert_eq_field(
        selection_record.selected_attempt_ids,
        selected.clone(),
        "SPOT_CHECK_SELECTION_RECORD_ATTEMPTS_MISMATCH",
    )?;

    let expected_spot_check_digest = spot_check_manifest_digest(&replay.spot_check)
        .map_err(|error| format!("SPOT_CHECK_MANIFEST_DIGEST_COMPUTE_FAILED: {error}"))?;
    assert_eq_field(
        expected_spot_check_digest,
        replay.spot_check.manifest_sha256.clone(),
        "SPOT_CHECK_MANIFEST_DIGEST_MISMATCH",
    )?;
    verified += 1;
    if selected
        != &replay
            .spot_check
            .attempts
            .iter()
            .map(|attempt| attempt.attempt_id.clone())
            .collect::<Vec<_>>()
    {
        return Err("SELECTED_ATTEMPT_ORDER_MISMATCH".to_owned());
    }
    let selected_policies: Vec<String> = selected
        .iter()
        .map(|attempt_id| {
            screen_attempts
                .get(attempt_id)
                .cloned()
                .ok_or_else(|| format!("SELECTED_ATTEMPT_NOT_IN_DIFFICULTY_SCREEN: {attempt_id}"))
        })
        .collect::<Result<_, _>>()?;

    let reset_bundle: ResetReceiptBundle = parse_artifact(root, &provenance.reset_receipt_bundle, "RESET_RECEIPT_BUNDLE")?;
    assert_eq_field(
        provenance.reset_receipt_bundle.sha256.clone(),
        replay.reset_receipt_sha256.clone(),
        "RESET_RECEIPT_BUNDLE_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    assert_eq_field(reset_bundle.schema_version, SCHEMA_VERSION.to_owned(), "RESET_RECEIPT_SCHEMA_VERSION_MISMATCH")?;
    assert_eq_field(reset_bundle.receipt_type, "reset_receipt_bundle".to_owned(), "RESET_RECEIPT_TYPE_MISMATCH")?;
    assert_eq_field(reset_bundle.scenario_id, replay.scenario_id.clone(), "RESET_RECEIPT_SCENARIO_MISMATCH")?;
    assert_eq_field(
        reset_bundle.execution_scope_id,
        provenance.execution_scope_id.clone(),
        "RESET_RECEIPT_SCOPE_MISMATCH",
    )?;
    assert_eq_field(
        reset_bundle.source_episode_sha256,
        replay.source_episode_sha256.clone(),
        "RESET_RECEIPT_SOURCE_EPISODE_MISMATCH",
    )?;
    assert_eq_field(
        reset_bundle.simulator_profile_sha256,
        replay.simulator_profile_sha256.clone(),
        "RESET_RECEIPT_SIMULATOR_PROFILE_MISMATCH",
    )?;
    assert_eq_field(
        reset_bundle.robot_model_profile_sha256,
        replay.robot_model_profile_sha256.clone(),
        "RESET_RECEIPT_ROBOT_MODEL_PROFILE_MISMATCH",
    )?;
    if reset_bundle.attempts.len() != 3 {
        return Err("RESET_RECEIPT_ATTEMPT_COUNT_NOT_THREE".to_owned());
    }

    let action_receipt: ActionPrefixReceiptBundle =
        parse_artifact(root, &provenance.action_prefix_receipt_bundle, "ACTION_PREFIX_RECEIPT_BUNDLE")?;
    assert_eq_field(
        provenance.action_prefix_receipt_bundle.sha256.clone(),
        replay.action_prefix_receipt_sha256.clone(),
        "ACTION_PREFIX_RECEIPT_REPLAY_MANIFEST_DIGEST_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.schema_version,
        SCHEMA_VERSION.to_owned(),
        "ACTION_PREFIX_RECEIPT_SCHEMA_VERSION_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.receipt_type,
        "action_prefix_receipt_bundle".to_owned(),
        "ACTION_PREFIX_RECEIPT_TYPE_MISMATCH",
    )?;
    assert_eq_field(action_receipt.scenario_id, replay.scenario_id.clone(), "ACTION_PREFIX_RECEIPT_SCENARIO_MISMATCH")?;
    assert_eq_field(
        action_receipt.execution_scope_id,
        provenance.execution_scope_id.clone(),
        "ACTION_PREFIX_RECEIPT_SCOPE_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.source_episode_sha256,
        replay.source_episode_sha256.clone(),
        "ACTION_PREFIX_RECEIPT_SOURCE_EPISODE_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.instruction_sha256,
        replay.instruction_sha256.clone(),
        "ACTION_PREFIX_RECEIPT_INSTRUCTION_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.action_prefix_index_sha256,
        replay.action_prefix_sha256.clone(),
        "ACTION_PREFIX_RECEIPT_PREFIX_INDEX_MISMATCH",
    )?;
    assert_eq_field(
        action_receipt.reset_receipt_sha256,
        replay.reset_receipt_sha256.clone(),
        "ACTION_PREFIX_RECEIPT_RESET_RECEIPT_MISMATCH",
    )?;
    for (actual, expected, code) in [
        (
            action_receipt.simulator_profile_sha256,
            replay.simulator_profile_sha256.clone(),
            "ACTION_PREFIX_RECEIPT_SIMULATOR_PROFILE_MISMATCH",
        ),
        (
            action_receipt.robot_model_profile_sha256,
            replay.robot_model_profile_sha256.clone(),
            "ACTION_PREFIX_RECEIPT_ROBOT_MODEL_PROFILE_MISMATCH",
        ),
        (
            action_receipt.controller_profile_sha256,
            replay.controller_profile_sha256.clone(),
            "ACTION_PREFIX_RECEIPT_CONTROLLER_PROFILE_MISMATCH",
        ),
        (
            action_receipt.rendering_profile_sha256,
            replay.rendering_profile_sha256.clone(),
            "ACTION_PREFIX_RECEIPT_RENDERING_PROFILE_MISMATCH",
        ),
    ] {
        assert_eq_field(actual, expected, code)?;
    }

    if action_index.attempts.len() != 3 || action_receipt.attempts.len() != 3 {
        return Err("ACTION_PREFIX_ATTEMPT_COUNT_NOT_THREE".to_owned());
    }
    let mut verified = 11usize; // source episode + instruction + index + four profiles + policy roster + two receipt bundles + selection record
    let mut frame_asset_paths = BTreeSet::new();
    for (index, attempt_id) in selected.iter().enumerate() {
        let expected_policy = &selected_policies[index];
        let reset = &reset_bundle.attempts[index];
        let indexed_prefix = &action_index.attempts[index];
        let receipt = &action_receipt.attempts[index];
        let replay_attempt = &replay.spot_check.attempts[index];
        let expected_initial_state_sha = &replay_attempt.original_state_sha256;

        let original_frame = validate_state_snapshot_artifact(
            root,
            &replay_attempt.original_state_path,
            &replay_attempt.original_state_sha256,
            &replay.scenario_id,
            attempt_id,
            &replay.simulator_profile_sha256,
            "ORIGINAL_STATE",
        )?;
        let replay_frame = validate_state_snapshot_artifact(
            root,
            &replay_attempt.replay_state_path,
            &replay_attempt.replay_state_sha256,
            &replay.scenario_id,
            attempt_id,
            &replay.simulator_profile_sha256,
            "REPLAY_STATE",
        )?;
        assert_eq_field(
            original_frame,
            replay_frame,
            "STATE_ARTIFACT_COORDINATE_FRAME_MISMATCH",
        )?;
        verified += 2;

        if replay_attempt.final_frame_pairs.len() != 5 {
            return Err("SPOT_CHECK_FRAME_PAIR_COUNT_NOT_FIVE".to_owned());
        }
        for (expected_index, frame_pair) in replay_attempt.final_frame_pairs.iter().enumerate() {
            if usize::from(frame_pair.frame_index) != expected_index
                || !is_sha256(&frame_pair.original_sha256)
                || !is_sha256(&frame_pair.replay_sha256)
            {
                return Err("SPOT_CHECK_FRAME_PAIR_IDENTITY_INVALID".to_owned());
            }
            if frame_pair.original_path == frame_pair.replay_path
                || !frame_asset_paths.insert(frame_pair.original_path.as_str())
                || !frame_asset_paths.insert(frame_pair.replay_path.as_str())
            {
                return Err("SPOT_CHECK_FRAME_ASSET_PATH_DUPLICATED".to_owned());
            }
            let original_frame_ref = ArtifactRef {
                relative_path: frame_pair.original_path.clone(),
                sha256: frame_pair.original_sha256.clone(),
            };
            let replay_frame_ref = ArtifactRef {
                relative_path: frame_pair.replay_path.clone(),
                sha256: frame_pair.replay_sha256.clone(),
            };
            let _original_bytes =
                read_artifact(root, &original_frame_ref, "ORIGINAL_FRAME")?;
            let _replay_bytes =
                read_artifact(root, &replay_frame_ref, "REPLAY_FRAME")?;
            verified += 2;
        }

        assert_eq_field(reset.attempt_id.clone(), attempt_id.clone(), "RESET_RECEIPT_ATTEMPT_ID_MISMATCH")?;
        assert_eq_field(reset.policy_id.clone(), expected_policy.clone(), "RESET_RECEIPT_POLICY_ID_MISMATCH")?;
        assert_eq_field(reset.status.clone(), "accepted".to_owned(), "RESET_RECEIPT_NOT_ACCEPTED")?;
        if reset.reset_generation == 0 {
            return Err("RESET_RECEIPT_GENERATION_INVALID".to_owned());
        }
        if !is_sha256(&reset.initial_state_sha256) {
            return Err("RESET_RECEIPT_INITIAL_STATE_DIGEST_INVALID".to_owned());
        }
        assert_eq_field(
            reset.initial_state_sha256.clone(),
            replay_attempt.original_state_sha256.clone(),
            "RESET_RECEIPT_INITIAL_STATE_MISMATCH",
        )?;

        assert_eq_field(indexed_prefix.attempt_id.clone(), attempt_id.clone(), "ACTION_PREFIX_INDEX_ATTEMPT_ID_MISMATCH")?;
        assert_eq_field(indexed_prefix.policy_id.clone(), expected_policy.clone(), "ACTION_PREFIX_INDEX_POLICY_ID_MISMATCH")?;
        if indexed_prefix.action_step_count == 0 {
            return Err("ACTION_PREFIX_STEP_COUNT_ZERO".to_owned());
        }
        let prefix_bytes = read_artifact(root, &indexed_prefix.action_prefix, "ATTEMPT_ACTION_PREFIX")?;
        if sha256_hex(&prefix_bytes) != indexed_prefix.action_prefix.sha256 {
            return Err("ATTEMPT_ACTION_PREFIX_DIGEST_MISMATCH".to_owned());
        }

        assert_eq_field(receipt.attempt_id.clone(), attempt_id.clone(), "ACTION_PREFIX_RECEIPT_ATTEMPT_ID_MISMATCH")?;
        assert_eq_field(receipt.policy_id.clone(), expected_policy.clone(), "ACTION_PREFIX_RECEIPT_POLICY_ID_MISMATCH")?;
        assert_eq_field(
            receipt.action_prefix_sha256.clone(),
            indexed_prefix.action_prefix.sha256.clone(),
            "ACTION_PREFIX_RECEIPT_ARTIFACT_DIGEST_MISMATCH",
        )?;
        assert_eq_field(
            receipt.action_step_count,
            indexed_prefix.action_step_count,
            "ACTION_PREFIX_RECEIPT_STEP_COUNT_MISMATCH",
        )?;
        if receipt.command_id.trim().is_empty() {
            return Err("ACTION_PREFIX_RECEIPT_COMMAND_ID_MISSING".to_owned());
        }
        assert_eq_field(receipt.status.clone(), "accepted".to_owned(), "ACTION_PREFIX_RECEIPT_NOT_ACCEPTED")?;
        verified += 1;
    }

    let index_ids: Vec<&str> = action_index.attempts.iter().map(|a| a.attempt_id.as_str()).collect();
    let reset_ids: Vec<&str> = reset_bundle.attempts.iter().map(|a| a.attempt_id.as_str()).collect();
    let receipt_ids: Vec<&str> = action_receipt.attempts.iter().map(|a| a.attempt_id.as_str()).collect();
    let selected_ids: Vec<&str> = selected.iter().map(String::as_str).collect();
    if index_ids != selected_ids || reset_ids != selected_ids || receipt_ids != selected_ids {
        return Err("PER_ATTEMPT_RECEIPT_SELECTION_ORDER_MISMATCH".to_owned());
    }
    Ok(verified)
}

pub fn validate_provenance_json(
    replay_manifest_json: &str,
    provenance_manifest_json: &str,
    artifact_root: &Path,
) -> Result<ProvenanceReport, String> {
    let replay: ReplayManifest = serde_json::from_str(replay_manifest_json)
        .map_err(|e| format!("REPLAY_MANIFEST_JSON_INVALID: {e}"))?;
    let provenance: ProvenanceManifest = serde_json::from_str(provenance_manifest_json)
        .map_err(|e| format!("PROVENANCE_MANIFEST_JSON_INVALID: {e}"))?;
    if replay.schema_version != SCHEMA_VERSION
        || replay.benchmark != "RRVR-REPLAY-FIDELITY"
        || replay.profile_id != "RoboRecoverPublishedThresholdsV1"
        || replay.metric_profile_id != "RRVR-SSIM-GAUSSIAN11-SIGMA1_5-RGB8-V0_1"
        || replay.campaign_id.trim().is_empty()
        || replay.data_license_and_provenance.trim().is_empty()
        || !is_sha256(&replay.split_manifest_sha256)
    {
        return Err("REPLAY_MANIFEST_PROFILE_OR_CAMPAIGN_UNSUPPORTED".to_owned());
    }
    if provenance.schema_version != SCHEMA_VERSION || provenance.benchmark != BENCHMARK {
        return Err("PROVENANCE_MANIFEST_PROFILE_UNSUPPORTED".to_owned());
    }
    if replay.campaign_id.trim().is_empty() || replay.data_license_and_provenance.trim().is_empty()
        || !is_sha256(&replay.split_manifest_sha256)
    {
        return Err("REPLAY_CAMPAIGN_OR_SPLIT_MANIFEST_METADATA_INVALID".to_owned());
    }
    let split_manifest_bytes =
        read_artifact(artifact_root, &provenance.split_manifest, "SPLIT_MANIFEST")?;
    if provenance.split_manifest.sha256 != replay.split_manifest_sha256
        || sha256_hex(&split_manifest_bytes) != replay.split_manifest_sha256
    {
        return Err("SPLIT_MANIFEST_REPLAY_DIGEST_MISMATCH".to_owned());
    }
    validate_replay_candidate_set(&replay.candidates)?;
    let replay_digest = sha256_hex(replay_manifest_json.as_bytes());
    if !is_sha256(&provenance.replay_manifest_sha256)
        || provenance.replay_manifest_sha256 != replay_digest
    {
        return Err("REPLAY_MANIFEST_RAW_DIGEST_MISMATCH".to_owned());
    }
    if provenance.candidates.len() != replay.candidates.len() {
        return Err("PROVENANCE_CANDIDATE_COUNT_MISMATCH".to_owned());
    }
    let mut replay_candidates = BTreeMap::new();
    for candidate in &replay.candidates {
        if replay_candidates.insert(candidate.scenario_id.as_str(), candidate).is_some() {
            return Err("REPLAY_SCENARIO_ID_DUPLICATED".to_owned());
        }
    }
    let mut seen = BTreeSet::new();
    let mut reports = Vec::with_capacity(provenance.candidates.len());
    for candidate in &provenance.candidates {
        if !seen.insert(candidate.scenario_id.as_str()) {
            return Err("PROVENANCE_SCENARIO_ID_DUPLICATED".to_owned());
        }
        let replay_candidate = replay_candidates
            .get(candidate.scenario_id.as_str())
            .ok_or_else(|| format!("PROVENANCE_SCENARIO_NOT_IN_REPLAY_MANIFEST: {}", candidate.scenario_id))?;
        match validate_candidate(replay_candidate, candidate, &provenance.policy_roster_profile_id, artifact_root) {
            Ok(verified_artifact_n) => reports.push(CandidateProvenanceReport {
                scenario_id: candidate.scenario_id.clone(),
                status: "valid".to_owned(),
                selected_attempt_n: 3,
                verified_artifact_n,
                reason_codes: Vec::new(),
            }),
            Err(reason) => reports.push(CandidateProvenanceReport {
                scenario_id: candidate.scenario_id.clone(),
                status: "invalid".to_owned(),
                selected_attempt_n: replay_candidate.spot_check.selected_attempt_ids.len(),
                verified_artifact_n: 0,
                reason_codes: vec![reason],
            }),
        }
    }
    let valid_n = reports.iter().filter(|r| r.status == "valid").count();
    Ok(ProvenanceReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        replay_manifest_sha256: replay_digest,
        candidate_n: reports.len(),
        valid_n,
        invalid_n: reports.len() - valid_n,
        candidate_reports: reports,
        claim_ceiling: "Artifact-backed source and receipt provenance preflight only. No issuer signature, author archive, simulator, HIL, physical, safety or SOTA claim.".to_owned(),
    })
}

pub fn canonical_report_json(report: &ProvenanceReport) -> Result<Vec<u8>, String> {
    serde_json::to_vec(report).map_err(|e| format!("REPORT_SERIALIZATION_FAILED: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, sync::atomic::{AtomicUsize, Ordering}};

    static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);

    fn temp_root() -> PathBuf {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("rrvr-provenance-{}-{id}", std::process::id()));
        fs::create_dir_all(&path).expect("temp root");
        path
    }

    fn write_json(root: &Path, relative: &str, value: &serde_json::Value) -> ArtifactRef {
        let bytes = serde_json::to_vec(value).expect("fixture json");
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, &bytes).expect("write");
        ArtifactRef {
            relative_path: relative.to_owned(),
            sha256: sha256_hex(&bytes),
        }
    }

    fn write_bytes(root: &Path, relative: &str, bytes: &[u8]) -> ArtifactRef {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
        ArtifactRef {
            relative_path: relative.to_owned(),
            sha256: sha256_hex(bytes),
        }
    }

    fn reseal_action_receipt(
        root: &Path,
        replay_json: &str,
        provenance_json: &str,
        mutate: impl FnOnce(&mut serde_json::Value),
    ) -> (String, String) {
        let mut replay: serde_json::Value = serde_json::from_str(replay_json).expect("replay json");
        let mut provenance: serde_json::Value =
            serde_json::from_str(provenance_json).expect("provenance json");
        let receipt_path = root.join("receipts/action-prefix.json");
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("read receipt"))
                .expect("receipt json");
        mutate(&mut receipt);
        let receipt_bytes = serde_json::to_vec(&receipt).expect("receipt bytes");
        fs::write(&receipt_path, &receipt_bytes).expect("write receipt");
        let receipt_sha = sha256_hex(&receipt_bytes);
        provenance["candidates"][0]["action_prefix_receipt_bundle"]["sha256"] =
            serde_json::json!(receipt_sha);
        replay["candidates"][0]["action_prefix_receipt_sha256"] =
            serde_json::json!(receipt_sha);
        let replay_json = serde_json::to_string(&replay).expect("replay json");
        provenance["replay_manifest_sha256"] =
            serde_json::json!(sha256_hex(replay_json.as_bytes()));
        let provenance_json = serde_json::to_string(&provenance).expect("provenance json");
        (replay_json, provenance_json)
    }

    fn finish_resealed_spot_check(
        mut replay: serde_json::Value,
        mut provenance: serde_json::Value,
    ) -> (String, String) {
        let spot_value = &mut replay["candidates"][0]["spot_check"];
        let spot: SpotCheck =
            serde_json::from_value(spot_value.clone()).expect("spot-check after mutation");
        spot_value["manifest_sha256"] = serde_json::json!(
            spot_check_manifest_digest(&spot).expect("recompute spot-check manifest digest")
        );
        let replay_json = serde_json::to_string(&replay).expect("re-serialize replay manifest");
        provenance["replay_manifest_sha256"] =
            serde_json::json!(sha256_hex(replay_json.as_bytes()));
        let provenance_json =
            serde_json::to_string(&provenance).expect("re-serialize provenance manifest");
        (replay_json, provenance_json)
    }

    fn reseal_spot_check(
        replay_json: &str,
        provenance_json: &str,
        mutate: impl FnOnce(&mut serde_json::Value),
    ) -> (String, String) {
        let mut replay: serde_json::Value =
            serde_json::from_str(replay_json).expect("replay json");
        let provenance: serde_json::Value =
            serde_json::from_str(provenance_json).expect("provenance json");
        mutate(&mut replay["candidates"][0]["spot_check"]);
        finish_resealed_spot_check(replay, provenance)
    }

    fn reseal_selection_record(
        root: &Path,
        replay_json: &str,
        provenance_json: &str,
        mutate: impl FnOnce(&mut serde_json::Value),
    ) -> (String, String) {
        let mut replay: serde_json::Value =
            serde_json::from_str(replay_json).expect("replay json");
        let provenance: serde_json::Value =
            serde_json::from_str(provenance_json).expect("provenance json");
        let selection_path = replay["candidates"][0]["spot_check"]["selection_record_path"]
            .as_str()
            .expect("selection path")
            .to_owned();
        let artifact_path = root.join(selection_path);
        let mut record: serde_json::Value =
            serde_json::from_slice(&fs::read(&artifact_path).expect("read selection record"))
                .expect("selection record json");
        mutate(&mut record);
        let record_bytes = serde_json::to_vec(&record).expect("selection record bytes");
        fs::write(&artifact_path, &record_bytes).expect("write selection record");
        replay["candidates"][0]["spot_check"]["selection_record_sha256"] =
            serde_json::json!(sha256_hex(&record_bytes));
        finish_resealed_spot_check(replay, provenance)
    }

    fn fixture() -> (PathBuf, String, String) {
        let root = temp_root();
        let instruction = write_bytes(&root, "source/instruction.txt", b"Pick up the red block.");
        let action0 = write_bytes(&root, "actions/attempt-0.json", br#"[{"step":0,"joint":1}] "#);
        let action1 = write_bytes(&root, "actions/attempt-1.json", br#"[{"step":0,"joint":2}] "#);
        let action2 = write_bytes(&root, "actions/attempt-2.json", br#"[{"step":0,"joint":3}] "#);
        let scope = "robot-cell-01";
        let profile = |root: &Path, path: &str, kind: &str, id: &str| {
            write_json(
                root,
                path,
                &serde_json::json!({
                    "schema_version":"0.1",
                    "artifact_kind":kind,
                    "profile_id":id,
                    "execution_scope_id":scope
                }),
            )
        };
        let simulator = profile(&root, "profiles/simulator.json", "simulator_profile", "sim-v1");
        let robot = profile(&root, "profiles/robot.json", "robot_model_profile", "robot-v1");
        let controller = profile(&root, "profiles/controller.json", "controller_profile", "controller-v1");
        let rendering = profile(&root, "profiles/rendering.json", "rendering_profile", "render-v1");
        let roster = write_json(
            &root,
            "profiles/policy-roster.json",
            &serde_json::json!({
                "schema_version":"0.1",
                "artifact_kind":"policy_roster",
                "policy_roster_id":"frozen-four-policy-roster-v1",
                "policy_ids":["policy-0","policy-1","policy-2","policy-3"]
            }),
        );
        let index_attempts = vec![
            serde_json::json!({"attempt_id":"attempt-0","policy_id":"policy-0","action_prefix":action0,"action_step_count":3}),
            serde_json::json!({"attempt_id":"attempt-1","policy_id":"policy-1","action_prefix":action1,"action_step_count":4}),
            serde_json::json!({"attempt_id":"attempt-2","policy_id":"policy-2","action_prefix":action2,"action_step_count":5}),
        ];
        let action_index_value = serde_json::json!({
            "schema_version":"0.1",
            "artifact_kind":"action_prefix_index",
            "scenario_id":"scenario-1",
            "execution_scope_id":scope,
            "attempts":index_attempts
        });
        let action_index = write_json(&root, "actions/index.json", &action_index_value);
        let source_value = serde_json::json!({
            "schema_version":"0.1",
            "artifact_kind":"source_episode",
            "scenario_id":"scenario-1",
            "platform":"robo_twin",
            "split":"test",
            "source_trajectory_id":"trajectory-1",
            "execution_scope_id":scope,
            "instruction_sha256":instruction.sha256,
            "action_prefix_index_sha256":action_index.sha256
        });
        let source_episode = write_json(&root, "source/episode.json", &source_value);

        let state_artifacts: Vec<(ArtifactRef, ArtifactRef)> = (0..3)
            .map(|index| {
                let attempt_id = format!("attempt-{index}");
                let offset_mm = index as f64 * 10.0;
                let original = write_json(
                    &root,
                    &format!("states/original-{attempt_id}.json"),
                    &serde_json::json!({
                        "schema_version":"0.1",
                        "scenario_id":"scenario-1",
                        "attempt_id":attempt_id,
                        "simulator_profile_sha256":simulator.sha256,
                        "end_effector_position":{
                            "coordinate_frame_id":"robot-base",
                            "x_mm":100.0 + offset_mm,
                            "y_mm":20.0,
                            "z_mm":300.0
                        },
                        "object_position":{
                            "coordinate_frame_id":"robot-base",
                            "x_mm":120.0 + offset_mm,
                            "y_mm":20.0,
                            "z_mm":40.0
                        }
                    }),
                );
                let replayed = write_json(
                    &root,
                    &format!("states/replay-{attempt_id}.json"),
                    &serde_json::json!({
                        "schema_version":"0.1",
                        "scenario_id":"scenario-1",
                        "attempt_id":attempt_id,
                        "simulator_profile_sha256":simulator.sha256,
                        "end_effector_position":{
                            "coordinate_frame_id":"robot-base",
                            "x_mm":101.0 + offset_mm,
                            "y_mm":20.0,
                            "z_mm":300.0
                        },
                        "object_position":{
                            "coordinate_frame_id":"robot-base",
                            "x_mm":121.0 + offset_mm,
                            "y_mm":20.0,
                            "z_mm":40.0
                        }
                    }),
                );
                (original, replayed)
            })
            .collect();

        let reset_value = serde_json::json!({
            "schema_version":"0.1","receipt_type":"reset_receipt_bundle",
            "scenario_id":"scenario-1","execution_scope_id":scope,
            "source_episode_sha256":source_episode.sha256,
            "simulator_profile_sha256":simulator.sha256,
            "robot_model_profile_sha256":robot.sha256,
            "attempts":[
                {"attempt_id":"attempt-0","policy_id":"policy-0","status":"accepted","initial_state_sha256":state_artifacts[0].0.sha256.clone(),"reset_generation":1},
                {"attempt_id":"attempt-1","policy_id":"policy-1","status":"accepted","initial_state_sha256":state_artifacts[1].0.sha256.clone(),"reset_generation":1},
                {"attempt_id":"attempt-2","policy_id":"policy-2","status":"accepted","initial_state_sha256":state_artifacts[2].0.sha256.clone(),"reset_generation":1}
            ]
        });
        let reset = write_json(&root, "receipts/reset.json", &reset_value);
        let action_receipt_value = serde_json::json!({
            "schema_version":"0.1","receipt_type":"action_prefix_receipt_bundle",
            "scenario_id":"scenario-1","execution_scope_id":scope,
            "source_episode_sha256":source_episode.sha256,
            "instruction_sha256":instruction.sha256,
            "action_prefix_index_sha256":action_index.sha256,
            "reset_receipt_sha256":reset.sha256,
            "simulator_profile_sha256":simulator.sha256,
            "robot_model_profile_sha256":robot.sha256,
            "controller_profile_sha256":controller.sha256,
            "rendering_profile_sha256":rendering.sha256,
            "attempts":[
                {"attempt_id":"attempt-0","policy_id":"policy-0","action_prefix_sha256":action0.sha256,"action_step_count":3,"command_id":"cmd-0","status":"accepted"},
                {"attempt_id":"attempt-1","policy_id":"policy-1","action_prefix_sha256":action1.sha256,"action_step_count":4,"command_id":"cmd-1","status":"accepted"},
                {"attempt_id":"attempt-2","policy_id":"policy-2","action_prefix_sha256":action2.sha256,"action_step_count":5,"command_id":"cmd-2","status":"accepted"}
            ]
        });
        let action_receipt = write_json(&root, "receipts/action-prefix.json", &action_receipt_value);

        let selected_attempt_ids = vec![
            "attempt-0".to_owned(),
            "attempt-1".to_owned(),
            "attempt-2".to_owned(),
        ];
        let selection_record_value = serde_json::json!({
            "schema_version":"0.1",
            "seed":42,
            "selection_method_id":"upstream-seed42",
            "selected_attempt_ids":selected_attempt_ids
        });
        let selection_record_bytes =
            serde_json::to_vec(&selection_record_value).expect("selection record bytes");
        let selection_record = write_bytes(
            &root,
            "receipts/selection-record.json",
            &selection_record_bytes,
        );

        let mut spot_attempts = Vec::new();
        for index in 0..3 {
            let attempt_id = format!("attempt-{index}");
            let mut final_frame_pairs = Vec::new();
            for frame_index in 0..5 {
                let original_path =
                    format!("frames/original-{attempt_id}-{frame_index}.frame");
                let replay_path =
                    format!("frames/replay-{attempt_id}-{frame_index}.frame");
                let original_bytes =
                    format!("original-frame-bytes:{attempt_id}:{frame_index}");
                let replay_bytes =
                    format!("replay-frame-bytes:{attempt_id}:{frame_index}");
                let original_frame =
                    write_bytes(&root, &original_path, original_bytes.as_bytes());
                let replay_frame =
                    write_bytes(&root, &replay_path, replay_bytes.as_bytes());
                final_frame_pairs.push(FramePairAsset {
                    frame_index: frame_index as u8,
                    original_path: original_frame.relative_path,
                    original_sha256: original_frame.sha256,
                    replay_path: replay_frame.relative_path,
                    replay_sha256: replay_frame.sha256,
                });
            }
            spot_attempts.push(SpotCheckAttempt {
                attempt_id: attempt_id.clone(),
                original_state_path: state_artifacts[index].0.relative_path.clone(),
                original_state_sha256: state_artifacts[index].0.sha256.clone(),
                replay_state_path: state_artifacts[index].1.relative_path.clone(),
                replay_state_sha256: state_artifacts[index].1.sha256.clone(),
                final_frame_pairs,
            });
        }
        let mut spot_check = SpotCheck {
            seed: 42,
            selection_method_id: "upstream-seed42".to_owned(),
            selection_record_path: selection_record.relative_path,
            selection_record_sha256: selection_record.sha256,
            selected_attempt_ids,
            attempts: spot_attempts,
            manifest_sha256: String::new(),
        };
        spot_check.manifest_sha256 =
            spot_check_manifest_digest(&spot_check).expect("spot-check digest");

        let replay = serde_json::json!({
            "schema_version":"0.1","benchmark":"RRVR-REPLAY-FIDELITY",
            "profile_id":"RoboRecoverPublishedThresholdsV1",
            "metric_profile_id":"RRVR-SSIM-GAUSSIAN11-SIGMA1_5-RGB8-V0_1",
            "campaign_id":"rrvr-provenance-known-answer-v0.1",
            "split_manifest_sha256":"f".repeat(64),
            "data_license_and_provenance":"Synthetic sidecar fixture only.",
            "candidates":[{
                "scenario_id":"scenario-1","platform":"robo_twin","split":"test",
                "source_episode_sha256":source_episode.sha256,
                "source_trajectory_id":"trajectory-1",
                "trajectory_cluster_id":"cluster-1",
                "instruction_sha256":instruction.sha256,
                "action_prefix_sha256":action_index.sha256,
                "split_leakage_keys_sha256":["d".repeat(64)],
                "simulator_profile_sha256":simulator.sha256,
                "robot_model_profile_sha256":robot.sha256,
                "controller_profile_sha256":controller.sha256,
                "rendering_profile_sha256":rendering.sha256,
                "reset_receipt_sha256":reset.sha256,
                "action_prefix_receipt_sha256":action_receipt.sha256,
                "difficulty_screen":{"trials":(0..20).map(|i|{
                    let attempt_id = match i {
                        0 => "attempt-0".to_owned(),
                        6 => "attempt-1".to_owned(),
                        12 => "attempt-2".to_owned(),
                        _ => format!("screen-{i}"),
                    };
                    serde_json::json!({
                        "attempt_id":attempt_id,
                        "policy_id":format!("policy-{}",i/5),
                        "trial_index":i%5,
                        "success":i<2
                    })
                }).collect::<Vec<_>>()},
                "spot_check":serde_json::to_value(&spot_check).expect("serialize spot-check manifest")
            }]
        });
        let replay_json = serde_json::to_string(&replay).expect("replay json");
        let replay_hash = sha256_hex(replay_json.as_bytes());
        let provenance = serde_json::json!({
            "schema_version":"0.1","benchmark":"RRVR-REPLAY-PROVENANCE",
            "replay_manifest_sha256":replay_hash,
            "policy_roster_profile_id":"frozen-four-policy-roster-v1",
            "candidates":[{
                "scenario_id":"scenario-1","platform":"robo_twin","split":"test",
                "source_trajectory_id":"trajectory-1","trajectory_cluster_id":"cluster-1","execution_scope_id":scope,
                "source_episode":source_episode,"instruction":instruction,
                "action_prefix_index":action_index,
                "simulator_profile":simulator,"robot_model_profile":robot,
                "controller_profile":controller,"rendering_profile":rendering,
                "policy_roster":roster,
                "reset_receipt_bundle":reset,"action_prefix_receipt_bundle":action_receipt
            }]
        });
        let provenance_json = serde_json::to_string(&provenance).expect("provenance json");
        (root, replay_json, provenance_json)
    }

    #[test]
    fn known_answer_verifies_all_artifacts_and_selected_attempt_receipts() {
        let (root, replay, provenance) = fixture();
        let report = validate_provenance_json(&replay, &provenance, &root).expect("valid provenance");
        assert_eq!(report.candidate_n, 1);
        assert_eq!(report.valid_n, 1);
        assert_eq!(report.invalid_n, 0);
        assert_eq!(report.candidate_reports[0].selected_attempt_n, 3);
        assert_eq!(report.candidate_reports[0].verified_artifact_n, 50);
    }

    #[test]
    fn raw_selection_record_tampering_is_rejected() {
        let (root, replay, provenance) = fixture();
        let replay_value: serde_json::Value =
            serde_json::from_str(&replay).expect("replay json");
        let relative = replay_value["candidates"][0]["spot_check"]["selection_record_path"]
            .as_str()
            .expect("selection record path");
        fs::write(root.join(relative), b"tampered").expect("tamper selection record");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "SPOT_CHECK_SELECTION_RECORD_RAW_DIGEST_MISMATCH"
        );
    }

    #[test]
    fn correctly_resealed_but_mismatched_selection_record_is_rejected() {
        let (root, replay, provenance) = fixture();
        let (replay, provenance) = reseal_selection_record(
            &root,
            &replay,
            &provenance,
            |record| {
                record["selected_attempt_ids"][1] = serde_json::json!("attempt-0");
            },
        );
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "SPOT_CHECK_SELECTION_RECORD_ATTEMPTS_MISMATCH"
        );
    }

    #[test]
    fn duplicate_frame_index_is_rejected_after_manifest_reseal() {
        let (root, replay, provenance) = fixture();
        let (replay, provenance) = reseal_spot_check(
            &replay,
            &provenance,
            |spot| {
                spot["attempts"][0]["final_frame_pairs"][1]["frame_index"] =
                    serde_json::json!(0);
            },
        );
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "SPOT_CHECK_FRAME_PAIR_IDENTITY_INVALID"
        );
    }

    #[test]
    fn duplicated_raw_frame_path_is_rejected_after_manifest_reseal() {
        let (root, replay, provenance) = fixture();
        let (replay, provenance) = reseal_spot_check(
            &replay,
            &provenance,
            |spot| {
                let first = spot["attempts"][0]["final_frame_pairs"][0].clone();
                spot["attempts"][0]["final_frame_pairs"][1]["original_path"] =
                    first["original_path"].clone();
                spot["attempts"][0]["final_frame_pairs"][1]["original_sha256"] =
                    first["original_sha256"].clone();
            },
        );
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "SPOT_CHECK_FRAME_ASSET_PATH_DUPLICATED"
        );
    }

    #[test]
    fn raw_frame_asset_tampering_is_rejected() {
        let (root, replay, provenance) = fixture();
        let replay_value: serde_json::Value =
            serde_json::from_str(&replay).expect("replay json");
        let relative = replay_value["candidates"][0]["spot_check"]["attempts"][1]
            ["final_frame_pairs"][3]["original_path"]
            .as_str()
            .expect("original frame path");
        fs::write(root.join(relative), b"tampered-frame").expect("tamper frame");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "ORIGINAL_FRAME_RAW_DIGEST_MISMATCH"
        );
    }

    #[test]
    fn raw_replay_state_tampering_is_rejected() {
        let (root, replay, provenance) = fixture();
        fs::write(root.join("states/replay-attempt-1.json"), b"tampered")
            .expect("tamper replay state");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "REPLAY_STATE_RAW_DIGEST_MISMATCH"
        );
    }

    #[test]
    fn raw_source_episode_tampering_is_rejected() {
        let (root, replay, provenance) = fixture();
        fs::write(root.join("source/episode.json"), b"tampered").expect("tamper");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(report.candidate_reports[0].reason_codes[0], "SOURCE_EPISODE_RAW_DIGEST_MISMATCH");
    }

    #[test]
    fn profile_artifact_substitution_is_rejected() {
        let (root, replay, provenance) = fixture();
        fs::write(root.join("profiles/controller.json"), br#"{"schema_version":"0.1","artifact_kind":"controller_profile","profile_id":"controller-v1","execution_scope_id":"wrong-cell"}"#).expect("replace");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(report.candidate_reports[0].reason_codes[0], "CONTROLLER_PROFILE_RAW_DIGEST_MISMATCH");
    }

    #[test]
    fn profile_scope_mismatch_is_rejected_after_hashes_are_resealed() {
        let (root, replay, provenance) = fixture();
        let mut replay_value: serde_json::Value =
            serde_json::from_str(&replay).expect("replay json");
        let mut sidecar_value: serde_json::Value =
            serde_json::from_str(&provenance).expect("sidecar json");
        let profile_path = root.join("profiles/controller.json");
        let mut profile: serde_json::Value =
            serde_json::from_slice(&fs::read(&profile_path).expect("read profile"))
                .expect("profile json");
        profile["execution_scope_id"] = serde_json::json!("wrong-cell");
        let profile_bytes = serde_json::to_vec(&profile).expect("profile bytes");
        fs::write(&profile_path, &profile_bytes).expect("write profile");
        let profile_sha = sha256_hex(&profile_bytes);
        sidecar_value["candidates"][0]["controller_profile"]["sha256"] =
            serde_json::json!(profile_sha);
        replay_value["candidates"][0]["controller_profile_sha256"] =
            serde_json::json!(profile_sha);

        let receipt_path = root.join("receipts/action-prefix.json");
        let mut receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("read receipt"))
                .expect("receipt json");
        receipt["controller_profile_sha256"] = serde_json::json!(profile_sha);
        let receipt_bytes = serde_json::to_vec(&receipt).expect("receipt bytes");
        fs::write(&receipt_path, &receipt_bytes).expect("write receipt");
        let receipt_sha = sha256_hex(&receipt_bytes);
        sidecar_value["candidates"][0]["action_prefix_receipt_bundle"]["sha256"] =
            serde_json::json!(receipt_sha);
        replay_value["candidates"][0]["action_prefix_receipt_sha256"] =
            serde_json::json!(receipt_sha);

        let replay = serde_json::to_string(&replay_value).expect("resealed replay");
        sidecar_value["replay_manifest_sha256"] =
            serde_json::json!(sha256_hex(replay.as_bytes()));
        let sidecar = serde_json::to_string(&sidecar_value).expect("resealed sidecar");
        let report = validate_provenance_json(&replay, &sidecar, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "PROFILE_SCOPE_MISMATCH"
        );
    }

    #[test]
    fn action_receipt_wrong_policy_is_rejected_after_raw_digest_is_resealed() {
        let (root, replay, provenance) = fixture();
        let (replay, provenance) = reseal_action_receipt(
            &root,
            &replay,
            &provenance,
            |receipt| receipt["attempts"][1]["policy_id"] = serde_json::json!("policy-3"),
        );
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "ACTION_PREFIX_RECEIPT_POLICY_ID_MISMATCH"
        );
    }

    #[test]
    fn policy_roster_wrong_policy_is_rejected_after_raw_digest_is_resealed() {
        let (root, replay, provenance) = fixture();
        let roster_path = root.join("profiles/policy-roster.json");
        let mut roster: serde_json::Value =
            serde_json::from_slice(&fs::read(&roster_path).expect("read roster"))
                .expect("roster json");
        roster["policy_ids"][3] = serde_json::json!("unlisted-policy");
        let roster_bytes = serde_json::to_vec(&roster).expect("roster bytes");
        fs::write(&roster_path, &roster_bytes).expect("write roster");
        let mut sidecar: serde_json::Value =
            serde_json::from_str(&provenance).expect("provenance json");
        sidecar["candidates"][0]["policy_roster"]["sha256"] =
            serde_json::json!(sha256_hex(&roster_bytes));
        let sidecar = serde_json::to_string(&sidecar).expect("sidecar json");
        let report = validate_provenance_json(&replay, &sidecar, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(
            report.candidate_reports[0].reason_codes[0],
            "POLICY_ROSTER_DOES_NOT_MATCH_DIFFICULTY_SCREEN"
        );
    }

    #[test]
    fn unsafe_artifact_paths_are_rejected() {
        let (root, replay, provenance) = fixture();
        let mut value: serde_json::Value = serde_json::from_str(&provenance).expect("json");
        value["candidates"][0]["instruction"]["relative_path"] = serde_json::json!("../instruction.txt");
        let mutated = serde_json::to_string(&value).expect("serialize");
        let report = validate_provenance_json(&replay, &mutated, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(report.candidate_reports[0].reason_codes[0], "INSTRUCTION_PATH_UNSAFE");
    }

    #[test]
    fn mismatched_replay_manifest_bytes_are_rejected() {
        let (root, replay, provenance) = fixture();
        let changed = replay.replace("RoboRecoverPublishedThresholdsV1", "other-profile");
        assert_eq!(
            validate_provenance_json(&changed, &provenance, &root).unwrap_err(),
            "REPLAY_MANIFEST_RAW_DIGEST_MISMATCH"
        );
    }

    #[test]
    fn policy_roster_must_match_screening_policy_ids() {
        let (root, replay, provenance) = fixture();
        let path = root.join("profiles/policy-roster.json");
        fs::write(path, br#"{"schema_version":"0.1","artifact_kind":"policy_roster","policy_roster_id":"bad","policy_ids":["policy-0","policy-1","policy-2","other"]}"#).expect("replace");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert_eq!(report.candidate_reports[0].reason_codes[0], "POLICY_ROSTER_RAW_DIGEST_MISMATCH");
    }

    #[test]
    fn missing_selected_reset_attempt_is_rejected() {
        let (root, replay, provenance) = fixture();
        let path = root.join("receipts/reset.json");
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
        value["attempts"].as_array_mut().unwrap().pop();
        fs::write(path, serde_json::to_vec(&value).expect("json bytes")).expect("replace");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert!(report.candidate_reports[0].reason_codes[0].contains("RESET_RECEIPT_BUNDLE_RAW_DIGEST_MISMATCH"));
    }
}
