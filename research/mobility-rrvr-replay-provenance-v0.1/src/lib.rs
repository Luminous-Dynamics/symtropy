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
struct ReplayManifest {
    schema_version: String,
    benchmark: String,
    profile_id: String,
    metric_profile_id: String,
    candidates: Vec<ReplayCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
struct ReplayCandidate {
    scenario_id: String,
    platform: Platform,
    split: Split,
    source_episode_sha256: String,
    source_trajectory_id: String,
    instruction_sha256: String,
    action_prefix_sha256: String,
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
struct DifficultyScreen {
    trials: Vec<DifficultyTrial>,
}

#[derive(Debug, Clone, Deserialize)]
struct DifficultyTrial {
    attempt_id: String,
    policy_id: String,
    trial_index: u8,
    success: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct SpotCheck {
    seed: u64,
    selection_method_id: String,
    selected_attempt_ids: Vec<String>,
    attempts: Vec<SpotCheckAttempt>,
}

#[derive(Debug, Clone, Deserialize)]
struct SpotCheckAttempt {
    attempt_id: String,
    original_state_sha256: String,
    replay_state_sha256: String,
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
    let mut verified = 10usize; // source episode + instruction + index + four profiles + policy roster + two receipt bundles
    for (index, attempt_id) in selected.iter().enumerate() {
        let expected_policy = &selected_policies[index];
        let reset = &reset_bundle.attempts[index];
        let indexed_prefix = &action_index.attempts[index];
        let receipt = &action_receipt.attempts[index];
        let expected_initial_state_sha = &replay.spot_check.attempts[index].original_state_sha256;

        assert_eq_field(reset.attempt_id.clone(), attempt_id.clone(), "RESET_RECEIPT_ATTEMPT_ID_MISMATCH")?;
        assert_eq_field(reset.policy_id.clone(), expected_policy.clone(), "RESET_RECEIPT_POLICY_ID_MISMATCH")?;
        assert_eq_field(reset.status.clone(), "accepted".to_owned(), "RESET_RECEIPT_NOT_ACCEPTED")?;
        if reset.reset_generation == 0 {
            return Err("RESET_RECEIPT_GENERATION_INVALID".to_owned());
        }
        if !is_sha256(&reset.initial_state_sha256) {
            return Err("RESET_RECEIPT_INITIAL_STATE_DIGEST_INVALID".to_owned());
        }
        assert_eq_field(reset.initial_state_sha256.clone(), expected_initial_state_sha.clone(), "RESET_RECEIPT_INITIAL_STATE_MISMATCH")?;

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
    {
        return Err("REPLAY_MANIFEST_PROFILE_UNSUPPORTED".to_owned());
    }
    if provenance.schema_version != SCHEMA_VERSION || provenance.benchmark != BENCHMARK {
        return Err("PROVENANCE_MANIFEST_PROFILE_UNSUPPORTED".to_owned());
    }
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

        let reset_value = serde_json::json!({
            "schema_version":"0.1","receipt_type":"reset_receipt_bundle",
            "scenario_id":"scenario-1","execution_scope_id":scope,
            "source_episode_sha256":source_episode.sha256,
            "simulator_profile_sha256":simulator.sha256,
            "robot_model_profile_sha256":robot.sha256,
            "attempts":[
                {"attempt_id":"attempt-0","policy_id":"policy-0","status":"accepted","initial_state_sha256":"a".repeat(64),"reset_generation":1},
                {"attempt_id":"attempt-1","policy_id":"policy-1","status":"accepted","initial_state_sha256":"b".repeat(64),"reset_generation":1},
                {"attempt_id":"attempt-2","policy_id":"policy-2","status":"accepted","initial_state_sha256":"c".repeat(64),"reset_generation":1}
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

        let replay = serde_json::json!({
            "schema_version":"0.1","benchmark":"RRVR-REPLAY-FIDELITY",
            "profile_id":"RoboRecoverPublishedThresholdsV1",
            "metric_profile_id":"RRVR-SSIM-GAUSSIAN11-SIGMA1_5-RGB8-V0_1",
            "candidates":[{
                "scenario_id":"scenario-1","platform":"robo_twin","split":"test",
                "source_episode_sha256":source_episode.sha256,
                "source_trajectory_id":"trajectory-1",
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
                    serde_json::json!({
                        "attempt_id":format!("screen-{i}"),
                        "policy_id":format!("policy-{}",i/5),
                        "trial_index":i%5,
                        "success":i<2
                    })
                }).collect::<Vec<_>>()},
                "spot_check":{
                    "seed":42,
                    "selection_method_id":"upstream-seed42",
                    "selected_attempt_ids":["attempt-0","attempt-1","attempt-2"],
                    "attempts":[
                        {"attempt_id":"attempt-0","original_state_sha256":"a".repeat(64),"replay_state_sha256":"1".repeat(64)},
                        {"attempt_id":"attempt-1","original_state_sha256":"b".repeat(64),"replay_state_sha256":"2".repeat(64)},
                        {"attempt_id":"attempt-2","original_state_sha256":"c".repeat(64),"replay_state_sha256":"3".repeat(64)}
                    ]
                }
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
                "source_trajectory_id":"trajectory-1","execution_scope_id":scope,
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
        assert_eq!(report.candidate_reports[0].verified_artifact_n, 13);
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
    fn action_receipt_wrong_policy_is_rejected() {
        let (root, replay, provenance) = fixture();
        let path = root.join("receipts/action-prefix.json");
        let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).expect("read")).expect("json");
        value["attempts"][1]["policy_id"] = serde_json::json!("policy-3");
        fs::write(path, serde_json::to_vec(&value).expect("json bytes")).expect("replace");
        let report = validate_provenance_json(&replay, &provenance, &root).expect("report");
        assert_eq!(report.invalid_n, 1);
        assert!(report.candidate_reports[0].reason_codes[0].contains("RAW_DIGEST_MISMATCH"));
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
