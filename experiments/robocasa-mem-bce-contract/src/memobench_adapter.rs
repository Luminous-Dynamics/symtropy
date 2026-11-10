//! Thin typed adapter contract for the public MEMOBench checkpoint schema.
//!
//! The adapter models the upstream memory_level selection, operation codes,
//! checkpoint predicates and official operation-rate denominator. It stores
//! only digests for source annotations; benchmark text, demonstrations and
//! object assets are not copied into this repository.
//!
//! MEMOBench is CC BY-NC 4.0. A non-commercial research attestation or separate
//! commercial permission evidence is required before this adapter qualifies a
//! native run. This module cannot adjudicate the legal validity of an attestation
//! or permission document.

use std::collections::{BTreeMap, BTreeSet};

use crate::memory_operations::MemoryOperationKind;

const MEMOBENCH_LICENSE_ID: &str = "CC-BY-NC-4.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemobenchMemoryLevel {
    Level0,
    Level1,
    Level2,
}

impl MemobenchMemoryLevel {
    pub fn index(self) -> usize {
        match self {
            Self::Level0 => 0,
            Self::Level1 => 1,
            Self::Level2 => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum MemobenchOperationCode {
    Storage = 0,
    Update = 1,
    Compression = 2,
}

impl TryFrom<i32> for MemobenchOperationCode {
    type Error = MemobenchAdapterError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Storage),
            1 => Ok(Self::Update),
            2 => Ok(Self::Compression),
            _ => Err(MemobenchAdapterError::UnknownOperationCode),
        }
    }
}

impl From<MemobenchOperationCode> for MemoryOperationKind {
    fn from(value: MemobenchOperationCode) -> Self {
        match value {
            MemobenchOperationCode::Storage => Self::Storage,
            MemobenchOperationCode::Update => Self::Update,
            MemobenchOperationCode::Compression => Self::Compression,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemobenchUsePurpose {
    NonCommercialScientificResearch,
    CommercialDevelopment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchLicenseEvidence {
    pub code_license_id: String,
    pub dataset_license_id: String,
    pub assets_license_id: String,
    pub code_license_sha256: String,
    pub dataset_license_sha256: String,
    pub assets_license_sha256: String,
    /// Separate manifest for third-party simulator/runtime licenses such as LIBERO.
    pub third_party_license_manifest_sha256: String,
    pub purpose: MemobenchUsePurpose,
    /// Responsible-party attestation of the declared use purpose.
    pub purpose_attestation_sha256: String,
    /// Required for commercial development; the caller must obtain it separately.
    pub commercial_permission_sha256: Option<String>,
    /// Independent verification receipt for the rightsholder permission.
    pub commercial_permission_verification_receipt_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchCheckpointAnnotation {
    pub checkpoint_index: u32,
    /// Hashes of the three memory-description entries; raw licensed text is not stored.
    pub memory_description_sha256_by_level: [String; 3],
    /// Upstream operating_type row. Codes are 0=Storage, 1=Update, 2=Compression.
    pub operation_code_by_level: [i32; 3],
    /// Hash of the evaluator-private BDDL subtask predicate.
    pub subtask_goal_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchTaskAnnotation {
    pub task_id: String,
    pub task_family: String,
    pub task_metadata_sha256: String,
    pub task_definition_sha256: String,
    pub checkpoints: Vec<MemobenchCheckpointAnnotation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchEpisodeIdentity {
    pub task_id: String,
    pub episode_id: String,
    /// Scene/seed cluster or other preregistered independent sampling unit.
    pub independence_unit_id: String,
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchEvaluationManifest {
    pub benchmark_source_commit: String,
    pub evaluation_protocol_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub memory_level: MemobenchMemoryLevel,
    pub license: MemobenchLicenseEvidence,
    /// Exact task IDs from the independently frozen public task-tree manifest.
    pub expected_task_ids: Vec<String>,
    /// Exact number of episodes expected for every task in this run.
    pub expected_episodes_per_task: usize,
    /// Exact checkpoint-row count for each task, pinned separately from imported rows.
    pub expected_checkpoint_count_by_task: BTreeMap<String, usize>,
    pub tasks: Vec<MemobenchTaskAnnotation>,
    pub episodes: Vec<MemobenchEpisodeIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedMemobenchCheckpoint {
    pub task_id: String,
    pub task_family: String,
    pub task_metadata_sha256: String,
    pub task_definition_sha256: String,
    pub checkpoint_index: u32,
    pub memory_level: MemobenchMemoryLevel,
    pub memory_description_sha256: String,
    pub operation_code: MemobenchOperationCode,
    pub operation: MemoryOperationKind,
    pub subtask_goal_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchCheckpointOutcome {
    pub benchmark_source_commit: String,
    pub evaluation_protocol_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub task_id: String,
    pub episode_id: String,
    pub independence_unit_id: String,
    pub seed: u64,
    pub task_metadata_sha256: String,
    pub task_definition_sha256: String,
    pub checkpoint_index: u32,
    pub memory_level: MemobenchMemoryLevel,
    pub memory_description_sha256: String,
    pub operation_code: i32,
    pub subtask_goal_sha256: String,
    /// Digest of the independently preserved episode/video/trajectory artifact.
    pub episode_artifact_sha256: Option<String>,
    /// Digest of the evaluator's checkpoint-result receipt.
    pub evaluator_receipt_sha256: Option<String>,
    /// Exact upstream checkpoint predicate result: true=completed, false=not completed.
    /// None means the outcome record is missing/unresolved, not an implicit failure.
    pub completed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchEpisodeOutcome {
    pub benchmark_source_commit: String,
    pub evaluation_protocol_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub task_id: String,
    pub episode_id: String,
    pub independence_unit_id: String,
    pub seed: u64,
    pub episode_artifact_sha256: Option<String>,
    pub evaluator_receipt_sha256: Option<String>,
    /// Official terminal task-success verdict, kept separate from checkpoint rates.
    pub task_success: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemobenchAdapterError {
    InvalidSourceCommit,
    InvalidDigest,
    EmptyIdentity,
    UnknownOperationCode,
    InvalidLicenseIdentifier,
    InvalidUseAttestation,
    CommercialUseNotAuthorized,
    TaskRosterMismatch,
    CheckpointCountMismatch,
    EmptyTaskRoster,
    EmptyCheckpointRoster,
    EmptyEpisodeRoster,
    DuplicateTask,
    DuplicateCheckpoint,
    DuplicateEpisode,
    UnexpectedTask,
    MetadataLengthMismatch,
    UnknownCheckpointOutcome,
    DuplicateCheckpointOutcome,
    CheckpointOutcomeIdentityMismatch,
    UnknownEpisodeOutcome,
    DuplicateEpisodeOutcome,
    EpisodeOutcomeIdentityMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemobenchCheckpointStatus {
    Completed,
    NotCompleted,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemobenchCheckpointResult {
    pub task_id: String,
    pub episode_id: String,
    pub independence_unit_id: String,
    pub checkpoint_index: u32,
    pub operation: MemoryOperationKind,
    pub subtask_goal_sha256: String,
    pub status: MemobenchCheckpointStatus,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemobenchOperationRate {
    /// None on operation-wide summaries; Some on task-stratified summaries.
    pub task_id: Option<String>,
    pub operation: MemoryOperationKind,
    pub total_checkpoints: usize,
    pub completed_count: usize,
    pub not_completed_count: usize,
    pub unresolved_count: usize,
    /// Same numerator and denominator as upstream compute_operation_rates().
    /// With incomplete imports this remains None rather than silently scoring
    /// missing artifacts as false.
    pub official_operation_rate: Option<f64>,
    /// Equal-task macro mean over task rows with a complete official rate.
    pub task_macro_operation_rate: Option<f64>,
    pub contributing_task_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemobenchTaskSuccessRate {
    /// None for the micro aggregate; Some for a per-task row.
    pub task_id: Option<String>,
    pub total_episodes: usize,
    pub successful_episodes: usize,
    pub unsuccessful_episodes: usize,
    pub unresolved_episodes: usize,
    pub official_task_success_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemobenchEvaluationAudit {
    pub benchmark_source_commit: String,
    pub evaluation_protocol_sha256: String,
    pub policy_artifact_sha256: String,
    pub memory_system_sha256: String,
    pub evaluator_sha256: String,
    pub memory_level: MemobenchMemoryLevel,
    pub license: MemobenchLicenseEvidence,
    pub selected_checkpoints: Vec<SelectedMemobenchCheckpoint>,
    pub checkpoint_results: Vec<MemobenchCheckpointResult>,
    pub operation_rates: Vec<MemobenchOperationRate>,
    pub task_operation_rates: Vec<MemobenchOperationRate>,
    pub task_success: MemobenchTaskSuccessRate,
    pub task_success_by_task: Vec<MemobenchTaskSuccessRate>,
    pub checkpoint_outcome_count: usize,
    pub episode_outcome_count: usize,
}

#[derive(Debug, Default)]
struct RateAccumulator {
    total: usize,
    completed: usize,
    not_completed: usize,
    unresolved: usize,
}

impl RateAccumulator {
    fn observe(&mut self, status: &MemobenchCheckpointStatus) {
        self.total += 1;
        match status {
            MemobenchCheckpointStatus::Completed => self.completed += 1,
            MemobenchCheckpointStatus::NotCompleted => self.not_completed += 1,
            MemobenchCheckpointStatus::Unresolved => self.unresolved += 1,
        }
    }

    fn finish(self, task_id: Option<String>, operation: MemoryOperationKind) -> MemobenchOperationRate {
        let official_operation_rate = (self.total > 0 && self.unresolved == 0)
            .then(|| self.completed as f64 / self.total as f64);
        MemobenchOperationRate {
            task_id,
            operation,
            total_checkpoints: self.total,
            completed_count: self.completed,
            not_completed_count: self.not_completed,
            unresolved_count: self.unresolved,
            official_operation_rate,
            task_macro_operation_rate: None,
            contributing_task_count: 0,
        }
    }
}

fn validate_license(evidence: &MemobenchLicenseEvidence) -> Result<(), MemobenchAdapterError> {
    if evidence.code_license_id != MEMOBENCH_LICENSE_ID
        || evidence.dataset_license_id != MEMOBENCH_LICENSE_ID
        || evidence.assets_license_id != MEMOBENCH_LICENSE_ID
    {
        return Err(MemobenchAdapterError::InvalidLicenseIdentifier);
    }
    for digest in [
        evidence.code_license_sha256.as_str(),
        evidence.dataset_license_sha256.as_str(),
        evidence.assets_license_sha256.as_str(),
        evidence.third_party_license_manifest_sha256.as_str(),
        evidence.purpose_attestation_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            return Err(MemobenchAdapterError::InvalidDigest);
        }
    }
    match evidence.purpose {
        MemobenchUsePurpose::NonCommercialScientificResearch => {
            if evidence.commercial_permission_sha256.is_some()
                || evidence.commercial_permission_verification_receipt_sha256.is_some()
            {
                return Err(MemobenchAdapterError::InvalidUseAttestation);
            }
        }
        MemobenchUsePurpose::CommercialDevelopment => {
            let permission = evidence.commercial_permission_sha256.as_deref()
                .ok_or(MemobenchAdapterError::CommercialUseNotAuthorized)?;
            let verification = evidence.commercial_permission_verification_receipt_sha256.as_deref()
                .ok_or(MemobenchAdapterError::CommercialUseNotAuthorized)?;
            if !is_sha256_hex(permission) || !is_sha256_hex(verification) {
                return Err(MemobenchAdapterError::InvalidDigest);
            }
        }
    }
    Ok(())
}

fn validate_manifest(
    manifest: &MemobenchEvaluationManifest,
) -> Result<Vec<SelectedMemobenchCheckpoint>, MemobenchAdapterError> {
    if !is_git_commit(&manifest.benchmark_source_commit) {
        return Err(MemobenchAdapterError::InvalidSourceCommit);
    }
    for digest in [
        manifest.evaluation_protocol_sha256.as_str(),
        manifest.policy_artifact_sha256.as_str(),
        manifest.memory_system_sha256.as_str(),
        manifest.evaluator_sha256.as_str(),
    ] {
        if !is_sha256_hex(digest) {
            return Err(MemobenchAdapterError::InvalidDigest);
        }
    }
    validate_license(&manifest.license)?;

    if manifest.tasks.is_empty() || manifest.expected_task_ids.is_empty() {
        return Err(MemobenchAdapterError::EmptyTaskRoster);
    }
    if manifest.episodes.is_empty() {
        return Err(MemobenchAdapterError::EmptyEpisodeRoster);
    }
    if manifest.expected_episodes_per_task == 0 {
        return Err(MemobenchAdapterError::EpisodeCountMismatch);
    }

    let mut expected_task_ids = BTreeSet::new();
    for task_id in &manifest.expected_task_ids {
        if task_id.trim().is_empty() {
            return Err(MemobenchAdapterError::EmptyIdentity);
        }
        if !expected_task_ids.insert(task_id.as_str()) {
            return Err(MemobenchAdapterError::DuplicateTask);
        }
    }

    let expected_checkpoint_task_ids = manifest
        .expected_checkpoint_count_by_task
        .keys()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if expected_checkpoint_task_ids != expected_task_ids
        || manifest.expected_checkpoint_count_by_task.values().any(|count| *count == 0)
    {
        return Err(MemobenchAdapterError::CheckpointCountMismatch);
    }

    let mut tasks = BTreeMap::<&str, &MemobenchTaskAnnotation>::new();
    for task in &manifest.tasks {
        if task.task_id.trim().is_empty() || task.task_family.trim().is_empty() {
            return Err(MemobenchAdapterError::EmptyIdentity);
        }
        if !is_sha256_hex(&task.task_metadata_sha256)
            || !is_sha256_hex(&task.task_definition_sha256)
        {
            return Err(MemobenchAdapterError::InvalidDigest);
        }
        if task.checkpoints.is_empty() {
            return Err(MemobenchAdapterError::EmptyCheckpointRoster);
        }
        if tasks.insert(task.task_id.as_str(), task).is_some() {
            return Err(MemobenchAdapterError::DuplicateTask);
        }
        let expected_count = manifest.expected_checkpoint_count_by_task
            .get(&task.task_id)
            .copied()
            .ok_or(MemobenchAdapterError::TaskRosterMismatch)?;
        if task.checkpoints.len() != expected_count {
            return Err(MemobenchAdapterError::CheckpointCountMismatch);
        }
    }

    if tasks.keys().copied().collect::<BTreeSet<_>>() != expected_task_ids {
        return Err(MemobenchAdapterError::TaskRosterMismatch);
    }

    let mut episodes = BTreeSet::new();
    let mut episode_counts = BTreeMap::<&str, usize>::new();
    for episode in &manifest.episodes {
        if episode.task_id.trim().is_empty()
            || episode.episode_id.trim().is_empty()
            || episode.independence_unit_id.trim().is_empty()
        {
            return Err(MemobenchAdapterError::EmptyIdentity);
        }
        if !tasks.contains_key(episode.task_id.as_str()) {
            return Err(MemobenchAdapterError::UnexpectedTask);
        }
        if !episodes.insert((episode.task_id.as_str(), episode.episode_id.as_str())) {
            return Err(MemobenchAdapterError::DuplicateEpisode);
        }
        *episode_counts.entry(episode.task_id.as_str()).or_default() += 1;
    }
    if episode_counts.len() != expected_task_ids.len()
        || expected_task_ids.iter().any(|task_id| {
            episode_counts.get(task_id).copied()
                != Some(manifest.expected_episodes_per_task)
        })
    {
        return Err(MemobenchAdapterError::EpisodeCountMismatch);
    }

    let mut selected = Vec::new();
    for task in &manifest.tasks {
        let mut seen_checkpoint_indices = BTreeSet::new();
        for checkpoint in &task.checkpoints {
            if !seen_checkpoint_indices.insert(checkpoint.checkpoint_index) {
                return Err(MemobenchAdapterError::DuplicateCheckpoint);
            }
            for digest in checkpoint.memory_description_sha256_by_level.iter() {
                if !is_sha256_hex(digest) {
                    return Err(MemobenchAdapterError::InvalidDigest);
                }
            }
            if !is_sha256_hex(&checkpoint.subtask_goal_sha256) {
                return Err(MemobenchAdapterError::InvalidDigest);
            }
            let level_index = manifest.memory_level.index();
            let operation_code = MemobenchOperationCode::try_from(
                checkpoint.operation_code_by_level[level_index],
            )?;
            selected.push(SelectedMemobenchCheckpoint {
                task_id: task.task_id.clone(),
                task_family: task.task_family.clone(),
                task_metadata_sha256: task.task_metadata_sha256.clone(),
                task_definition_sha256: task.task_definition_sha256.clone(),
                checkpoint_index: checkpoint.checkpoint_index,
                memory_level: manifest.memory_level,
                memory_description_sha256: checkpoint.memory_description_sha256_by_level[level_index].clone(),
                operation: operation_code.into(),
                operation_code,
                subtask_goal_sha256: checkpoint.subtask_goal_sha256.clone(),
            });
        }
    }
    Ok(selected)
}

/// Select one of the three metadata columns exactly as upstream does:
/// operating_type_row[memory_level], with codes 0=Storage, 1=Update, 2=Compression.
/// The raw memory text and BDDL predicates stay outside this Rust crate; only
/// their source digests are retained in the selected roster.
pub fn select_memobench_checkpoints(
    manifest: &MemobenchEvaluationManifest,
) -> Result<Vec<SelectedMemobenchCheckpoint>, MemobenchAdapterError> {
    validate_manifest(manifest)
}

/// Reproduce upstream operation success denominators over the selected
/// checkpoint roster and expected episode roster. Every expected episode gets
/// one expected result per selected checkpoint for its task. Some(false) is an
/// explicit checkpoint predicate miss and counts in the denominator. None is
/// missing evidence and makes the official rate unavailable for that stratum.
pub fn audit_memobench_operations(
    manifest: &MemobenchEvaluationManifest,
    checkpoint_outcomes: &[MemobenchCheckpointOutcome],
    episode_outcomes: &[MemobenchEpisodeOutcome],
) -> Result<MemobenchEvaluationAudit, MemobenchAdapterError> {
    let selected_checkpoints = validate_manifest(manifest)?;
    let tasks = manifest.tasks.iter().map(|task| (task.task_id.as_str(), task)).collect::<BTreeMap<_, _>>();

    let mut expected = BTreeMap::<(String, String, u32), &SelectedMemobenchCheckpoint>::new();
    let mut expected_per_task = BTreeMap::<String, usize>::new();
    for episode in &manifest.episodes {
        for checkpoint in selected_checkpoints.iter().filter(|row| row.task_id == episode.task_id) {
            expected.insert(
                (episode.task_id.clone(), episode.episode_id.clone(), checkpoint.checkpoint_index),
                checkpoint,
            );
            *expected_per_task.entry(episode.task_id.clone()).or_default() += 1;
        }
    }

    let mut observed = BTreeMap::new();
    for outcome in checkpoint_outcomes {
        if !context_matches(
            manifest,
            &outcome.benchmark_source_commit,
            &outcome.evaluation_protocol_sha256,
            &outcome.policy_artifact_sha256,
            &outcome.memory_system_sha256,
            &outcome.evaluator_sha256,
        ) {
            return Err(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch);
        }
        let key = (
            outcome.task_id.clone(),
            outcome.episode_id.clone(),
            outcome.checkpoint_index,
        );
        let Some(expected_checkpoint) = expected.get(&key).copied() else {
            return Err(MemobenchAdapterError::UnknownCheckpointOutcome);
        };
        let episode = manifest.episodes.iter().find(|episode| {
            episode.task_id == outcome.task_id && episode.episode_id == outcome.episode_id
        }).ok_or(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch)?;
        if outcome.independence_unit_id != episode.independence_unit_id
            || outcome.seed != episode.seed
            || outcome.task_metadata_sha256 != expected_checkpoint.task_metadata_sha256
            || outcome.task_definition_sha256 != expected_checkpoint.task_definition_sha256
            || outcome.memory_level != manifest.memory_level
            || outcome.memory_description_sha256 != expected_checkpoint.memory_description_sha256
            || outcome.operation_code != expected_checkpoint.operation_code as i32
            || outcome.subtask_goal_sha256 != expected_checkpoint.subtask_goal_sha256
        {
            return Err(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch);
        }
        for digest in [
            outcome.episode_artifact_sha256.as_deref(),
            outcome.evaluator_receipt_sha256.as_deref(),
        ].into_iter().flatten() {
            if !is_sha256_hex(digest) {
                return Err(MemobenchAdapterError::InvalidDigest);
            }
        }
        if observed.insert(key, outcome).is_some() {
            return Err(MemobenchAdapterError::DuplicateCheckpointOutcome);
        }
    }

    let mut checkpoint_results = Vec::with_capacity(expected.len());
    let mut operation_rates = BTreeMap::<MemoryOperationKind, RateAccumulator>::new();
    let mut task_operation_rates =
        BTreeMap::<(String, MemoryOperationKind), RateAccumulator>::new();
    for (key, checkpoint) in &expected {
        let outcome = observed.get(key).copied().and_then(|row| {
            if row.episode_artifact_sha256.is_some()
                && row.evaluator_receipt_sha256.is_some()
            {
                row.completed
            } else {
                None
            }
        });
        let status = match outcome {
            Some(true) => MemobenchCheckpointStatus::Completed,
            Some(false) => MemobenchCheckpointStatus::NotCompleted,
            None => MemobenchCheckpointStatus::Unresolved,
        };
        operation_rates.entry(checkpoint.operation).or_default().observe(&status);
        task_operation_rates
            .entry((checkpoint.task_id.clone(), checkpoint.operation))
            .or_default()
            .observe(&status);
        checkpoint_results.push(MemobenchCheckpointResult {
            task_id: checkpoint.task_id.clone(),
            episode_id: key.1.clone(),
            independence_unit_id: manifest.episodes.iter().find(|episode| {
                episode.task_id == key.0 && episode.episode_id == key.1
            }).expect("expected checkpoint episode exists").independence_unit_id.clone(),
            checkpoint_index: checkpoint.checkpoint_index,
            operation: checkpoint.operation,
            subtask_goal_sha256: checkpoint.subtask_goal_sha256.clone(),
            status,
        });
    }

    let mut episode_observed = BTreeMap::new();
    for outcome in episode_outcomes {
        if !context_matches(
            manifest,
            &outcome.benchmark_source_commit,
            &outcome.evaluation_protocol_sha256,
            &outcome.policy_artifact_sha256,
            &outcome.memory_system_sha256,
            &outcome.evaluator_sha256,
        ) {
            return Err(MemobenchAdapterError::EpisodeOutcomeIdentityMismatch);
        }
        let exists = manifest.episodes.iter().any(|episode| {
            episode.task_id == outcome.task_id
                && episode.episode_id == outcome.episode_id
                && episode.independence_unit_id == outcome.independence_unit_id
        });
        if !exists {
            return Err(MemobenchAdapterError::UnknownEpisodeOutcome);
        }
        let expected_episode = manifest.episodes.iter().find(|episode| {
            episode.task_id == outcome.task_id && episode.episode_id == outcome.episode_id
        }).expect("episode existence checked");
        if outcome.seed != expected_episode.seed {
            return Err(MemobenchAdapterError::EpisodeOutcomeIdentityMismatch);
        }
        for digest in [
            outcome.episode_artifact_sha256.as_deref(),
            outcome.evaluator_receipt_sha256.as_deref(),
        ].into_iter().flatten() {
            if !is_sha256_hex(digest) {
                return Err(MemobenchAdapterError::InvalidDigest);
            }
        }
        if episode_observed.insert((outcome.task_id.clone(), outcome.episode_id.clone()), outcome).is_some() {
            return Err(MemobenchAdapterError::DuplicateEpisodeOutcome);
        }
    }

    let mut task_success_by_task = Vec::new();
    let mut success_total = 0usize;
    let mut success_count = 0usize;
    let mut success_unresolved = 0usize;
    for task_id in tasks.keys().copied() {
        let task_episodes = manifest.episodes.iter().filter(|episode| episode.task_id == task_id).collect::<Vec<_>>();
        let mut total = 0usize;
        let mut success = 0usize;
        let mut failed = 0usize;
        let mut unresolved = 0usize;
        for episode in task_episodes {
            total += 1;
            match episode_observed
                .get(&(episode.task_id.clone(), episode.episode_id.clone()))
                .filter(|row| row.episode_artifact_sha256.is_some()
                    && row.evaluator_receipt_sha256.is_some())
                .and_then(|row| row.task_success)
            {
                Some(true) => success += 1,
                Some(false) => failed += 1,
                None => unresolved += 1,
            }
        }
        success_total += total;
        success_count += success;
        success_unresolved += unresolved;
        task_success_by_task.push(MemobenchTaskSuccessRate {
            task_id: Some(task_id.to_owned()),
            total_episodes: total,
            successful_episodes: success,
            unsuccessful_episodes: failed,
            unresolved_episodes: unresolved,
            official_task_success_rate: (total > 0 && unresolved == 0).then(|| success as f64 / total as f64),
        });
    }
    let success_failed = success_total.saturating_sub(success_count + success_unresolved);
    let task_success = MemobenchTaskSuccessRate {
        task_id: None,
        total_episodes: success_total,
        successful_episodes: success_count,
        unsuccessful_episodes: success_failed,
        unresolved_episodes: success_unresolved,
        official_task_success_rate: (success_total > 0 && success_unresolved == 0)
            .then(|| success_count as f64 / success_total as f64),
    };

    let task_operation_rates = task_operation_rates
        .into_iter()
        .map(|((task_id, operation), accumulator)| accumulator.finish(Some(task_id), operation))
        .collect::<Vec<_>>();
    let mut operation_rates = operation_rates
        .into_iter()
        .map(|(operation, accumulator)| accumulator.finish(None, operation))
        .collect::<Vec<_>>();
    for rate in &mut operation_rates {
        let task_rows = task_operation_rates.iter()
            .filter(|row| row.operation == rate.operation)
            .collect::<Vec<_>>();
        rate.contributing_task_count = task_rows.len();
        if !task_rows.is_empty() && task_rows.iter().all(|row| row.official_operation_rate.is_some()) {
            rate.task_macro_operation_rate = Some(
                task_rows.iter()
                    .filter_map(|row| row.official_operation_rate)
                    .sum::<f64>() / task_rows.len() as f64,
            );
        }
    }
    Ok(MemobenchEvaluationAudit {
        benchmark_source_commit: manifest.benchmark_source_commit.clone(),
        evaluation_protocol_sha256: manifest.evaluation_protocol_sha256.clone(),
        policy_artifact_sha256: manifest.policy_artifact_sha256.clone(),
        memory_system_sha256: manifest.memory_system_sha256.clone(),
        evaluator_sha256: manifest.evaluator_sha256.clone(),
        memory_level: manifest.memory_level,
        license: manifest.license.clone(),
        selected_checkpoints,
        checkpoint_results,
        operation_rates,
        task_operation_rates,
        task_success,
        task_success_by_task,
        checkpoint_outcome_count: checkpoint_outcomes.len(),
        episode_outcome_count: episode_outcomes.len(),
    })
}

fn context_matches(
    manifest: &MemobenchEvaluationManifest,
    benchmark_commit: &str,
    protocol: &str,
    policy: &str,
    memory_system: &str,
    evaluator: &str,
) -> bool {
    benchmark_commit == manifest.benchmark_source_commit
        && protocol == manifest.evaluation_protocol_sha256
        && policy == manifest.policy_artifact_sha256
        && memory_system == manifest.memory_system_sha256
        && evaluator == manifest.evaluator_sha256
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_git_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(char: char) -> String {
        std::iter::repeat_n(char, 64).collect()
    }

    fn license() -> MemobenchLicenseEvidence {
        MemobenchLicenseEvidence {
            code_license_id: MEMOBENCH_LICENSE_ID.to_owned(),
            dataset_license_id: MEMOBENCH_LICENSE_ID.to_owned(),
            assets_license_id: MEMOBENCH_LICENSE_ID.to_owned(),
            code_license_sha256: digest('a'),
            dataset_license_sha256: digest('b'),
            assets_license_sha256: digest('c'),
            third_party_license_manifest_sha256: digest('d'),
            purpose: MemobenchUsePurpose::NonCommercialScientificResearch,
            purpose_attestation_sha256: digest('e'),
            commercial_permission_sha256: None,
            commercial_permission_verification_receipt_sha256: None,
        }
    }

    fn task(task_id: &str, family: &str) -> MemobenchTaskAnnotation {
        MemobenchTaskAnnotation {
            task_id: task_id.to_owned(),
            task_family: family.to_owned(),
            task_metadata_sha256: digest('1'),
            task_definition_sha256: digest('2'),
            checkpoints: vec![
                MemobenchCheckpointAnnotation {
                    checkpoint_index: 0,
                    memory_description_sha256_by_level: [digest('3'), digest('4'), digest('5')],
                    operation_code_by_level: [0, 1, 2],
                    subtask_goal_sha256: digest('6'),
                },
                MemobenchCheckpointAnnotation {
                    checkpoint_index: 1,
                    memory_description_sha256_by_level: [digest('7'), digest('8'), digest('9')],
                    operation_code_by_level: [2, 0, 1],
                    subtask_goal_sha256: digest('a'),
                },
            ],
        }
    }

    fn manifest() -> MemobenchEvaluationManifest {
        MemobenchEvaluationManifest {
            benchmark_source_commit: "1".repeat(40),
            evaluation_protocol_sha256: digest('b'),
            policy_artifact_sha256: digest('c'),
            memory_system_sha256: digest('d'),
            evaluator_sha256: digest('e'),
            memory_level: MemobenchMemoryLevel::Level1,
            license: license(),
            expected_task_ids: vec!["WA-01".to_owned()],
            expected_episodes_per_task: 2,
            expected_checkpoint_count_by_task: BTreeMap::from([("WA-01".to_owned(), 2)]),
            tasks: vec![task("WA-01", "temporal")],
            episodes: vec![
                MemobenchEpisodeIdentity {
                    task_id: "WA-01".to_owned(),
                    episode_id: "episode-1".to_owned(),
                    independence_unit_id: "scene-seed-1".to_owned(),
                    seed: 1,
                },
                MemobenchEpisodeIdentity {
                    task_id: "WA-01".to_owned(),
                    episode_id: "episode-2".to_owned(),
                    independence_unit_id: "scene-seed-2".to_owned(),
                    seed: 2,
                },
            ],
        }
    }

    fn checkpoint_outcome(
        manifest: &MemobenchEvaluationManifest,
        checkpoint_index: u32,
        episode_id: &str,
        completed: Option<bool>,
    ) -> MemobenchCheckpointOutcome {
        let task = &manifest.tasks[0];
        let checkpoint = task.checkpoints.iter().find(|row| row.checkpoint_index == checkpoint_index).unwrap();
        let selected = checkpoint.operation_code_by_level[manifest.memory_level.index()];
        let episode = manifest.episodes.iter().find(|row| row.episode_id == episode_id).unwrap();
        MemobenchCheckpointOutcome {
            benchmark_source_commit: manifest.benchmark_source_commit.clone(),
            evaluation_protocol_sha256: manifest.evaluation_protocol_sha256.clone(),
            policy_artifact_sha256: manifest.policy_artifact_sha256.clone(),
            memory_system_sha256: manifest.memory_system_sha256.clone(),
            evaluator_sha256: manifest.evaluator_sha256.clone(),
            task_id: task.task_id.clone(),
            episode_id: episode_id.to_owned(),
            independence_unit_id: episode.independence_unit_id.clone(),
            seed: episode.seed,
            task_metadata_sha256: task.task_metadata_sha256.clone(),
            task_definition_sha256: task.task_definition_sha256.clone(),
            checkpoint_index,
            memory_level: manifest.memory_level,
            memory_description_sha256: checkpoint.memory_description_sha256_by_level[manifest.memory_level.index()].clone(),
            operation_code: selected,
            subtask_goal_sha256: checkpoint.subtask_goal_sha256.clone(),
            episode_artifact_sha256: Some(digest('f')),
            evaluator_receipt_sha256: Some(digest('a')),
            completed,
        }
    }

    fn episode_outcome(
        manifest: &MemobenchEvaluationManifest,
        episode_id: &str,
        task_success: Option<bool>,
    ) -> MemobenchEpisodeOutcome {
        let episode = manifest.episodes.iter().find(|row| row.episode_id == episode_id).unwrap();
        MemobenchEpisodeOutcome {
            benchmark_source_commit: manifest.benchmark_source_commit.clone(),
            evaluation_protocol_sha256: manifest.evaluation_protocol_sha256.clone(),
            policy_artifact_sha256: manifest.policy_artifact_sha256.clone(),
            memory_system_sha256: manifest.memory_system_sha256.clone(),
            evaluator_sha256: manifest.evaluator_sha256.clone(),
            task_id: episode.task_id.clone(),
            episode_id: episode_id.to_owned(),
            independence_unit_id: episode.independence_unit_id.clone(),
            seed: episode.seed,
            episode_artifact_sha256: Some(digest('f')),
            evaluator_receipt_sha256: Some(digest('a')),
            task_success,
        }
    }

    #[test]
    fn upstream_memory_level_selects_exact_operation_code_and_memory_digest() {
        let mut input = manifest();
        let l0 = select_memobench_checkpoints(&input).unwrap();
        assert_eq!(l0[0].operation, MemoryOperationKind::Update);
        assert_eq!(l0[0].memory_description_sha256, digest('4'));

        input.memory_level = MemobenchMemoryLevel::Level0;
        let l1 = select_memobench_checkpoints(&input).unwrap();
        assert_eq!(l1[0].operation, MemoryOperationKind::Storage);
        assert_eq!(l1[0].memory_description_sha256, digest('3'));

        input.memory_level = MemobenchMemoryLevel::Level2;
        let l2 = select_memobench_checkpoints(&input).unwrap();
        assert_eq!(l2[0].operation, MemoryOperationKind::Compression);
        assert_eq!(l2[0].memory_description_sha256, digest('5'));
    }

    #[test]
    fn official_operation_rates_match_subtask_predicate_completion_denominators() {
        let input = manifest();
        let outcomes = vec![
            checkpoint_outcome(&input, 0, "episode-1", Some(true)),
            checkpoint_outcome(&input, 1, "episode-1", Some(false)),
            checkpoint_outcome(&input, 0, "episode-2", Some(true)),
            checkpoint_outcome(&input, 1, "episode-2", Some(true)),
        ];
        let episodes = vec![
            episode_outcome(&input, "episode-1", Some(true)),
            episode_outcome(&input, "episode-2", Some(false)),
        ];
        let audit = audit_memobench_operations(&input, &outcomes, &episodes).unwrap();
        assert_eq!(audit.operation_rates.len(), 2);
        assert_eq!(audit.task_success.official_task_success_rate, Some(0.5));
        let update = audit.operation_rates.iter()
            .find(|rate| rate.operation == MemoryOperationKind::Update).unwrap();
        assert_eq!(update.total_checkpoints, 2);
        assert_eq!(update.completed_count, 2);
        assert_eq!(update.official_operation_rate, Some(1.0));
        assert_eq!(update.task_macro_operation_rate, Some(1.0));
        let storage = audit.operation_rates.iter()
            .find(|rate| rate.operation == MemoryOperationKind::Storage).unwrap();
        assert_eq!(storage.total_checkpoints, 2);
        assert_eq!(storage.not_completed_count, 1);
        assert_eq!(storage.official_operation_rate, Some(0.5));
    }

    #[test]
    fn missing_checkpoint_or_task_outcome_is_unresolved_not_a_false_label() {
        let input = manifest();
        let outcomes = vec![
            checkpoint_outcome(&input, 0, "episode-1", Some(true)),
            checkpoint_outcome(&input, 1, "episode-1", Some(false)),
            checkpoint_outcome(&input, 0, "episode-2", None),
        ];
        let episodes = vec![
            episode_outcome(&input, "episode-1", Some(true)),
            episode_outcome(&input, "episode-2", None),
        ];
        let audit = audit_memobench_operations(&input, &outcomes, &episodes).unwrap();
        assert_eq!(audit.operation_rates.iter().any(|rate| rate.unresolved_count > 0), true);
        assert!(audit.operation_rates.iter().all(|rate| rate.official_operation_rate.is_none()));
        assert_eq!(audit.task_success.unresolved_episodes, 1);
        assert_eq!(audit.task_success.official_task_success_rate, None);
    }

    #[test]
    fn duplicate_and_unknown_checkpoint_outcomes_reject() {
        let input = manifest();
        let row = checkpoint_outcome(&input, 0, "episode-1", Some(true));
        assert_eq!(
            audit_memobench_operations(&input, &[row.clone(), row.clone()], &[]),
            Err(MemobenchAdapterError::DuplicateCheckpointOutcome)
        );
        let mut unknown = row;
        unknown.checkpoint_index = 99;
        assert_eq!(
            audit_memobench_operations(&input, &[unknown], &[]),
            Err(MemobenchAdapterError::UnknownCheckpointOutcome)
        );
    }

    #[test]
    fn mismatched_policy_or_task_predicate_context_rejects() {
        let input = manifest();
        let mut row = checkpoint_outcome(&input, 0, "episode-1", Some(true));
        row.memory_system_sha256 = digest('f');
        assert_eq!(
            audit_memobench_operations(&input, &[row], &[]),
            Err(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch)
        );

        let mut row = checkpoint_outcome(&input, 0, "episode-1", Some(true));
        row.subtask_goal_sha256 = digest('f');
        assert_eq!(
            audit_memobench_operations(&input, &[row], &[]),
            Err(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch)
        );
    }

    #[test]
    fn exact_license_gate_blocks_commercial_use_without_separate_permission() {
        let mut input = manifest();
        input.license.purpose = MemobenchUsePurpose::CommercialDevelopment;
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::CommercialUseNotAuthorized)
        );
        input.license.commercial_permission_sha256 = Some(digest('f'));
        input.license.commercial_permission_verification_receipt_sha256 = Some(digest('a'));
        assert!(select_memobench_checkpoints(&input).is_ok());
    }

    #[test]
    fn frozen_task_and_checkpoint_rosters_reject_omissions_and_extra_tasks() {
        let mut input = manifest();
        input.expected_task_ids.push("missing-task".to_owned());
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::TaskRosterMismatch)
        );

        input = manifest();
        input.expected_checkpoint_count_by_task.insert("WA-01".to_owned(), 3);
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::CheckpointCountMismatch)
        );
    }

    #[test]
    fn episode_seed_and_artifact_receipts_are_required_for_qualified_rates() {
        let input = manifest();
        let mut checkpoint = checkpoint_outcome(&input, 0, "episode-1", Some(true));
        checkpoint.episode_artifact_sha256 = None;
        let mut other = checkpoint_outcome(&input, 1, "episode-1", Some(false));
        other.seed = 99;
        assert_eq!(
            audit_memobench_operations(&input, &[checkpoint, other], &[]),
            Err(MemobenchAdapterError::CheckpointOutcomeIdentityMismatch)
        );

        let mut episodes = vec![
            episode_outcome(&input, "episode-1", Some(true)),
            episode_outcome(&input, "episode-2", Some(false)),
        ];
        episodes[0].evaluator_receipt_sha256 = None;
        let report = audit_memobench_operations(&input, &[], &episodes).unwrap();
        assert_eq!(report.task_success.unresolved_episodes, 1);
        assert_eq!(report.task_success.official_task_success_rate, None);
    }

    #[test]
    fn commercial_use_permission_requires_a_separate_verification_receipt() {
        let mut input = manifest();
        input.license.purpose = MemobenchUsePurpose::CommercialDevelopment;
        input.license.commercial_permission_sha256 = Some(digest('f'));
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::CommercialUseNotAuthorized)
        );
        input.license.commercial_permission_verification_receipt_sha256 = Some(digest('a'));
        assert!(select_memobench_checkpoints(&input).is_ok());
    }

    #[test]
    fn operation_code_outside_upstream_enum_rejects() {
        let mut input = manifest();
        input.tasks[0].checkpoints[0].operation_code_by_level[1] = 3;
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::UnknownOperationCode)
        );
    }

    #[test]
    fn metadata_checkpoint_counts_and_digests_must_be_complete() {
        let mut input = manifest();
        input.tasks[0].checkpoints[0].memory_description_sha256_by_level[2] = "bad".to_owned();
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::InvalidDigest)
        );
        input = manifest();
        input.tasks[0].checkpoints.pop();
        assert_eq!(
            select_memobench_checkpoints(&input),
            Err(MemobenchAdapterError::CheckpointCountMismatch)
        );
    }
}
