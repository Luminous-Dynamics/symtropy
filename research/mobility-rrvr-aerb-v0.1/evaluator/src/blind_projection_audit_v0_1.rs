// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Blind candidate projection and evaluator-side metadata-leakage audit.
//!
//! Projection preserves the typed evidence and within-episode causal timing
//! needed by AERB, while replacing incidental identities, absolute clocks,
//! global sequence offsets, endpoint names and raw content/profile digests.
//! Original identifiers and trajectory-cluster mappings remain in an
//! evaluator-only receipt and must never be mounted into the candidate process.
//!
//! The metadata auditor performs grouped held-out categorical attacks. It is a
//! diagnostic baseline, not a proof against arbitrary learned classifiers or
//! adaptive stateful candidates.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{CandidateEpisode, Observation, sha256_hex, SCHEMA_VERSION, BENCHMARK, OPEN_WORLD_RESIDUAL};

#[cfg(test)]
use super::{
    ClockStamp, CommandContext, EvidenceDomain, ObservationQuality, PermittedProbe, SourceRole,
};

const PROJECTION_DOMAIN: &[u8] = b"AERB-BLIND-CANDIDATE-PROJECTION-V0.1\0";
const RECEIPT_DOMAIN: &[u8] = b"AERB-BLIND-PROJECTION-RECEIPT-V0.1\0";
const VALID_WORLD_IDS: [&str; 8] = [
    "not_dispatched",
    "dispatched_no_effect",
    "applied_once_ack_lost",
    "partial_effect",
    "duplicate_effect",
    "stale_authority_effect",
    "unresolved_command_holder",
    OPEN_WORLD_RESIDUAL,
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    EntropyUnavailable(String),
    InvalidCandidateEpisode(&'static str),
    InvalidOpaqueEpisodeId,
    ClockRebaseOverflow,
    SequenceRebaseOverflow,
    SerializationFailed,
    InvalidProjectionReceipt(&'static str),
    InvalidMetadataAudit(&'static str),
    TooFewGroups,
    TooManyFolds,
}

impl std::fmt::Display for ProjectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EntropyUnavailable(reason) => write!(formatter, "projection entropy unavailable: {reason}"),
            Self::InvalidCandidateEpisode(field) => write!(formatter, "candidate episode invalid for blind projection: {field}"),
            Self::InvalidOpaqueEpisodeId => formatter.write_str("blind episode ID must match ep_ plus 32 lowercase hexadecimal characters"),
            Self::ClockRebaseOverflow => formatter.write_str("clock rebasing overflow"),
            Self::SequenceRebaseOverflow => formatter.write_str("sequence rebasing overflow"),
            Self::SerializationFailed => formatter.write_str("blind projection serialization failed"),
            Self::InvalidProjectionReceipt(field) => write!(formatter, "invalid evaluator projection receipt: {field}"),
            Self::InvalidMetadataAudit(field) => write!(formatter, "invalid metadata audit input: {field}"),
            Self::TooFewGroups => formatter.write_str("metadata audit requires at least two independent groups"),
            Self::TooManyFolds => formatter.write_str("metadata audit fold count must be between two and the independent group count"),
        }
    }
}

impl std::error::Error for ProjectionError {}

/// Campaign-scoped secret used to create non-reversible, stable pseudonyms.
/// It must remain evaluator-only and must not be serialized to candidate input.
#[derive(Clone)]
pub struct BlindProjectionContext {
    salt: [u8; 32],
}

impl std::fmt::Debug for BlindProjectionContext {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("BlindProjectionContext([REDACTED])")
    }
}

impl BlindProjectionContext {
    pub fn generate() -> Result<Self, ProjectionError> {
        let mut salt = [0u8; 32];
        getrandom::fill(&mut salt)
            .map_err(|error| ProjectionError::EntropyUnavailable(error.to_string()))?;
        Ok(Self { salt })
    }

    /// Construct from a campaign secret held by the evaluator. Callers should
    /// normally generate a fresh independent salt and keep it outside candidate custody.
    pub fn from_secret(salt: [u8; 32]) -> Self {
        Self { salt }
    }

    fn pseudonym_hex(&self, namespace: &str, raw: &str) -> String {
        let mut preimage = Vec::with_capacity(
            PROJECTION_DOMAIN.len() + self.salt.len() + namespace.len() + raw.len() + 16,
        );
        preimage.extend_from_slice(PROJECTION_DOMAIN);
        preimage.extend_from_slice(&self.salt);
        append_length_prefixed(&mut preimage, namespace.as_bytes());
        append_length_prefixed(&mut preimage, raw.as_bytes());
        sha256_hex(&preimage)
    }

    fn pseudonym_id(&self, prefix: &str, namespace: &str, raw: &str) -> String {
        let digest = self.pseudonym_hex(namespace, raw);
        format!("{prefix}_{}", &digest[..32])
    }

    fn pseudonym_digest(&self, namespace: &str, raw_digest: &str) -> String {
        self.pseudonym_hex(namespace, raw_digest)
    }

    fn pseudonym_u64(&self, namespace: &str, raw: u64) -> u64 {
        let digest = self.pseudonym_hex(namespace, &raw.to_string());
        u64::from_str_radix(&digest[..16], 16).unwrap_or(0)
    }
}

fn append_length_prefixed(output: &mut Vec<u8>, bytes: &[u8]) {
    output.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    output.extend_from_slice(bytes);
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn valid_blind_episode_id(value: &str) -> bool {
    value.len() == 35
        && value.starts_with("ep_")
        && value[3..]
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn rebase_clock_stamps(
    observations: &mut [Observation],
) -> Result<BTreeMap<String, i64>, ProjectionError> {
    let mut minima = BTreeMap::<String, i64>::new();
    for observation in observations.iter() {
        for stamp in [&observation.event_time, &observation.acquired_time] {
            minima
                .entry(stamp.clock_domain_id.clone())
                .and_modify(|minimum| *minimum = (*minimum).min(stamp.ticks_ns))
                .or_insert(stamp.ticks_ns);
        }
    }
    for observation in observations.iter_mut() {
        for stamp in [
            &mut observation.event_time,
            &mut observation.acquired_time,
        ] {
            let minimum = minima
                .get(&stamp.clock_domain_id)
                .ok_or(ProjectionError::InvalidCandidateEpisode("clock domain"))?;
            stamp.ticks_ns = stamp
                .ticks_ns
                .checked_sub(*minimum)
                .ok_or(ProjectionError::ClockRebaseOverflow)?;
        }
    }
    Ok(minima)
}

fn pseudonymize_clock_domains(
    context: &BlindProjectionContext,
    observations: &mut [Observation],
) {
    for observation in observations {
        observation.event_time.clock_domain_id = context.pseudonym_id(
            "clk",
            "clock-domain",
            &observation.event_time.clock_domain_id,
        );
        observation.acquired_time.clock_domain_id = context.pseudonym_id(
            "clk",
            "clock-domain",
            &observation.acquired_time.clock_domain_id,
        );
    }
}

fn min_ingest_sequence(observations: &[Observation]) -> Option<u64> {
    observations
        .iter()
        .map(|observation| observation.ingest_sequence)
        .min()
}

/// Evaluator-only mapping receipt. Keep this in the evaluator custody root.
/// The candidate process receives only the projected CandidateEpisode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlindProjectionReceipt {
    pub schema_version: String,
    pub candidate_episode_id: String,
    pub original_episode_id: String,
    pub original_scenario_id: String,
    pub original_attempt_id: String,
    pub original_trajectory_cluster_id: String,
    pub original_episode_sha256: String,
    pub projected_episode_sha256: String,
    pub projection_profile_sha256: String,
    pub original_clock_minima_ns: BTreeMap<String, i64>,
    pub forbidden_raw_values: Vec<String>,
    pub original_ingest_sequence_minimum: u64,
    pub original_current_step: u64,
    pub projection_receipt_sha256: String,
}

impl BlindProjectionReceipt {
    pub fn verify(&self) -> Result<(), ProjectionError> {
        if self.schema_version != "aerb-blind-projection-v0.1" {
            return Err(ProjectionError::InvalidProjectionReceipt("schema_version"));
        }
        if !valid_blind_episode_id(&self.candidate_episode_id) {
            return Err(ProjectionError::InvalidProjectionReceipt("candidate_episode_id"));
        }
        if self.original_episode_id.trim().is_empty()
            || self.original_scenario_id.trim().is_empty()
            || self.original_attempt_id.trim().is_empty()
            || self.original_trajectory_cluster_id.trim().is_empty()
        {
            return Err(ProjectionError::InvalidProjectionReceipt("original identity"));
        }
        for digest in [
            self.original_episode_sha256.as_str(),
            self.projected_episode_sha256.as_str(),
            self.projection_profile_sha256.as_str(),
            self.projection_receipt_sha256.as_str(),
        ] {
            if !valid_sha256(digest) {
                return Err(ProjectionError::InvalidProjectionReceipt("digest"));
            }
        }
        if receipt_commitment(self)? != self.projection_receipt_sha256 {
            return Err(ProjectionError::InvalidProjectionReceipt("commitment"));
        }
        Ok(())
    }
}

fn receipt_commitment(
    receipt: &BlindProjectionReceipt,
) -> Result<String, ProjectionError> {
    let mut payload = receipt.clone();
    payload.projection_receipt_sha256.clear();
    let bytes = serde_json::to_vec(&payload).map_err(|_| ProjectionError::SerializationFailed)?;
    let mut preimage = RECEIPT_DOMAIN.to_vec();
    preimage.extend_from_slice(&bytes);
    Ok(sha256_hex(&preimage))
}

/// Project one candidate episode into a blind-safe representation.
///
/// The supplied episode ID must come from the CSPRNG-generated candidate
/// manifest. This function never derives candidate order or truth assignments.
pub fn project_candidate_episode(
    context: &BlindProjectionContext,
    original: &CandidateEpisode,
    candidate_episode_id: &str,
) -> Result<(CandidateEpisode, BlindProjectionReceipt), ProjectionError> {
    if !valid_blind_episode_id(candidate_episode_id) {
        return Err(ProjectionError::InvalidOpaqueEpisodeId);
    }
    if original.schema_version != SCHEMA_VERSION || original.benchmark != BENCHMARK {
        return Err(ProjectionError::InvalidCandidateEpisode("schema"));
    }
    if original.observations.is_empty() {
        return Err(ProjectionError::InvalidCandidateEpisode("observations"));
    }
    if original.current_step > original.decision_deadline_step {
        return Err(ProjectionError::InvalidCandidateEpisode("deadline"));
    }
    for digest in [
        original.command_context.payload_sha256.as_str(),
        original.command_context.action_prefix_sha256.as_str(),
        original.command_context.endpoint_profile_sha256.as_str(),
        original.command_context.system_boundary_manifest_sha256.as_str(),
        original.command_context.effect_producing_inventory_profile_sha256.as_str(),
    ] {
        if !valid_sha256(digest) {
            return Err(ProjectionError::InvalidCandidateEpisode("command digest"));
        }
    }
    for observation in &original.observations {
        if !valid_sha256(&observation.artifact_sha256) {
            return Err(ProjectionError::InvalidCandidateEpisode("observation artifact digest"));
        }
        if observation.observation_id.trim().is_empty()
            || observation.source_id.trim().is_empty()
            || observation.event_time.clock_domain_id.trim().is_empty()
            || observation.acquired_time.clock_domain_id.trim().is_empty()
        {
            return Err(ProjectionError::InvalidCandidateEpisode("observation identity"));
        }
    }

    let original_json =
        serde_json::to_vec(original).map_err(|_| ProjectionError::SerializationFailed)?;
    let original_episode_sha256 = sha256_hex(&original_json);
    let original_minimum = min_ingest_sequence(&original.observations)
        .ok_or(ProjectionError::InvalidCandidateEpisode("ingest sequence"))?;
    let original_current_step = original.current_step;

    let mut projected = original.clone();
    projected.episode_id = candidate_episode_id.to_owned();
    // Scenario and trajectory group labels can act as lookup keys. Scenario
    // identity is intentionally per-episode; evaluator grouping stays private.
    projected.scenario_id =
        context.pseudonym_id("scn", "scenario-instance", &original.episode_id);
    projected.attempt_id =
        context.pseudonym_id("att", "attempt", &original.attempt_id);
    projected.trajectory_cluster_id =
        context.pseudonym_id("trj", "trajectory-private", &original.episode_id);

    projected.command_context.command_id =
        context.pseudonym_id("cmd", "command-id", &original.command_context.command_id);
    projected.command_context.operation_id =
        context.pseudonym_id("op", "operation-id", &original.command_context.operation_id);
    projected.command_context.payload_sha256 = context.pseudonym_digest(
        "payload-digest",
        &original.command_context.payload_sha256,
    );
    projected.command_context.action_prefix_sha256 = context.pseudonym_digest(
        "action-prefix-digest",
        &original.command_context.action_prefix_sha256,
    );
    projected.command_context.endpoint_id =
        context.pseudonym_id("end", "endpoint-id", &original.command_context.endpoint_id);
    projected.command_context.endpoint_profile_sha256 = context.pseudonym_digest(
        "endpoint-profile-digest",
        &original.command_context.endpoint_profile_sha256,
    );
    projected.command_context.authority_epoch =
        context.pseudonym_u64("authority-epoch", original.command_context.authority_epoch);
    projected.command_context.queue_generation =
        context.pseudonym_u64("queue-generation", original.command_context.queue_generation);
    projected.command_context.execution_scope_id = context.pseudonym_id(
        "scp",
        "execution-scope",
        &original.command_context.execution_scope_id,
    );
    projected.command_context.system_boundary_manifest_sha256 = context.pseudonym_digest(
        "system-boundary-profile",
        &original.command_context.system_boundary_manifest_sha256,
    );
    projected.command_context.effect_producing_inventory_profile_sha256 =
        context.pseudonym_digest(
            "effect-inventory-profile",
            &original
                .command_context
                .effect_producing_inventory_profile_sha256,
        );

    let clock_minima = rebase_clock_stamps(&mut projected.observations)?;
    for observation in &mut projected.observations {
        observation.observation_id = context.pseudonym_id(
            "obs",
            "observation-id",
            &observation.observation_id,
        );
        observation.source_id =
            context.pseudonym_id("src", "source-id", &observation.source_id);
        observation.artifact_sha256 = context.pseudonym_digest(
            "observation-artifact",
            &observation.artifact_sha256,
        );
        observation.ingest_sequence = observation
            .ingest_sequence
            .checked_sub(original_minimum)
            .ok_or(ProjectionError::SequenceRebaseOverflow)?;
    }
    pseudonymize_clock_domains(context, &mut projected.observations);

    projected.decision_cut_sequence = original
        .decision_cut_sequence
        .checked_sub(original_minimum)
        .ok_or(ProjectionError::SequenceRebaseOverflow)?;
    projected.ledger_frontier_sequence = original
        .ledger_frontier_sequence
        .checked_sub(original_minimum)
        .ok_or(ProjectionError::SequenceRebaseOverflow)?;
    projected.current_step = 0;
    projected.decision_deadline_step = original
        .decision_deadline_step
        .checked_sub(original_current_step)
        .ok_or(ProjectionError::SequenceRebaseOverflow)?;

    let projected_json =
        serde_json::to_vec(&projected).map_err(|_| ProjectionError::SerializationFailed)?;
    let projected_episode_sha256 = sha256_hex(&projected_json);

    let mut receipt = BlindProjectionReceipt {
        schema_version: "aerb-blind-projection-v0.1".to_owned(),
        candidate_episode_id: candidate_episode_id.to_owned(),
        original_episode_id: original.episode_id.clone(),
        original_scenario_id: original.scenario_id.clone(),
        original_attempt_id: original.attempt_id.clone(),
        original_trajectory_cluster_id: original.trajectory_cluster_id.clone(),
        original_episode_sha256,
        projected_episode_sha256,
        projection_profile_sha256: projection_profile_sha256(),
        original_clock_minima_ns: clock_minima,
        forbidden_raw_values: collect_forbidden_raw_values(original),
        original_ingest_sequence_minimum: original_minimum,
        original_current_step,
        projection_receipt_sha256: String::new(),
    };
    receipt.projection_receipt_sha256 = receipt_commitment(&receipt)?;

    Ok((projected, receipt))
}

pub fn projection_profile_sha256() -> String {
    sha256_hex(b"AERB-BLIND-PROJECTION-PROFILE-V0.1:opaque-identities; salted-digests; relative-clocks; relative-sequences; private-clusters; preserved-source-role-quality-probe-cost-and-within-episode-evidence")
}

pub fn verify_projected_episode(
    projected: &CandidateEpisode,
    receipt: &BlindProjectionReceipt,
) -> Result<(), ProjectionError> {
    receipt.verify()?;
    if projected.episode_id != receipt.candidate_episode_id {
        return Err(ProjectionError::InvalidProjectionReceipt("projected episode ID"));
    }
    let projected_json =
        serde_json::to_vec(projected).map_err(|_| ProjectionError::SerializationFailed)?;
    if sha256_hex(&projected_json) != receipt.projected_episode_sha256 {
        return Err(ProjectionError::InvalidProjectionReceipt("projected episode digest"));
    }
    if projected.scenario_id == receipt.original_scenario_id
        || projected.attempt_id == receipt.original_attempt_id
        || projected.trajectory_cluster_id == receipt.original_trajectory_cluster_id
    {
        return Err(ProjectionError::InvalidProjectionReceipt("unredacted metadata"));
    }
    Ok(())
}

fn collect_forbidden_raw_values(episode: &CandidateEpisode) -> Vec<String> {
    let mut values = BTreeSet::new();
    for value in [
        episode.episode_id.as_str(),
        episode.scenario_id.as_str(),
        episode.attempt_id.as_str(),
        episode.trajectory_cluster_id.as_str(),
        episode.command_context.command_id.as_str(),
        episode.command_context.operation_id.as_str(),
        episode.command_context.payload_sha256.as_str(),
        episode.command_context.action_prefix_sha256.as_str(),
        episode.command_context.endpoint_id.as_str(),
        episode.command_context.endpoint_profile_sha256.as_str(),
        episode.command_context.execution_scope_id.as_str(),
        episode.command_context.system_boundary_manifest_sha256.as_str(),
        episode.command_context.effect_producing_inventory_profile_sha256.as_str(),
    ] {
        if !value.is_empty() {
            values.insert(value.to_owned());
        }
    }
    values.insert(episode.command_context.authority_epoch.to_string());
    values.insert(episode.command_context.queue_generation.to_string());
    values.insert(episode.decision_cut_sequence.to_string());
    values.insert(episode.ledger_frontier_sequence.to_string());
    values.insert(episode.current_step.to_string());
    values.insert(episode.decision_deadline_step.to_string());
    for observation in &episode.observations {
        for value in [
            observation.observation_id.as_str(),
            observation.source_id.as_str(),
            observation.event_time.clock_domain_id.as_str(),
            observation.acquired_time.clock_domain_id.as_str(),
            observation.artifact_sha256.as_str(),
        ] {
            if !value.is_empty() {
                values.insert(value.to_owned());
            }
        }
        values.insert(observation.event_time.ticks_ns.to_string());
        values.insert(observation.acquired_time.ticks_ns.to_string());
        values.insert(observation.ingest_sequence.to_string());
    }
    values.into_iter().collect()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSurfaceFinding {
    pub location: String,
    pub finding_kind: String,
    pub token_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSurfaceAudit {
    pub schema_version: String,
    pub candidate_episode_n: usize,
    pub projection_receipt_n: usize,
    pub finding_n: usize,
    pub passed: bool,
    pub findings: Vec<CandidateSurfaceFinding>,
    pub interpretation: String,
}

fn json_string_leaves(value: &serde_json::Value, path: &str, output: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::String(text) => output.push((path.to_owned(), text.clone())),
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                json_string_leaves(item, &format!("{path}[{index}]"), output);
            }
        }
        serde_json::Value::Object(fields) => {
            for (key, item) in fields {
                json_string_leaves(item, &format!("{path}.{key}"), output);
            }
        }
        _ => {}
    }
}

fn forbidden_field_names() -> [&'static str; 9] {
    [
        "hidden_truth_world_id",
        "manifest_salt_hex",
        "evaluator_seed",
        "hidden_truth_by_episode",
        "scenario_count",
        "world-pack",
        "evaluator_manifest",
        "evaluator_world_pack",
        "assignments",
    ]
}

/// Scan candidate manifest and episode JSON against hidden labels and raw
/// evaluator identifiers recorded in the evaluator-only projection receipts.
pub fn audit_candidate_surface(
    candidate_manifest_json: &str,
    candidate_episode_jsons: &[String],
    receipts: &[BlindProjectionReceipt],
    hidden_world_ids: &[String],
    evaluator_secret_tokens: &[String],
    candidate_file_names: &[String],
) -> Result<CandidateSurfaceAudit, ProjectionError> {
    if candidate_episode_jsons.len() != receipts.len()
        || candidate_file_names.len() != receipts.len()
    {
        return Err(ProjectionError::InvalidMetadataAudit("candidate file/receipt count mismatch"));
    }
    let manifest: serde_json::Value = serde_json::from_str(candidate_manifest_json)
        .map_err(|_| ProjectionError::InvalidMetadataAudit("candidate manifest JSON"))?;
    let manifest_ids = manifest
        .get("episode_ids")
        .and_then(serde_json::Value::as_array)
        .ok_or(ProjectionError::InvalidMetadataAudit("candidate manifest episode_ids"))?
        .iter()
        .map(|item| item.as_str().ok_or(ProjectionError::InvalidMetadataAudit("manifest episode ID type")))
        .collect::<Result<Vec<_>, _>>()?;
    if manifest_ids.len() != receipts.len() {
        return Err(ProjectionError::InvalidMetadataAudit("manifest/receipt count mismatch"));
    }
    let mut findings = Vec::new();
    let mut expected_ids = receipts.iter().map(|receipt| receipt.candidate_episode_id.as_str()).collect::<Vec<_>>();
    expected_ids.sort_unstable();
    let mut observed_ids = manifest_ids.clone();
    observed_ids.sort_unstable();
    if expected_ids != observed_ids {
        findings.push(CandidateSurfaceFinding {
            location: "candidate_manifest.episode_ids".to_owned(),
            finding_kind: "projection_identity_set_mismatch".to_owned(),
            token_sha256: sha256_hex(b"projection-identity-set-mismatch"),
        });
    }

    let mut leaves = Vec::new();
    json_string_leaves(&manifest, "$manifest", &mut leaves);
    for (index, raw) in candidate_episode_jsons.iter().enumerate() {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| ProjectionError::InvalidMetadataAudit("candidate episode JSON"))?;
        json_string_leaves(&value, &format!("$episode[{index}]"), &mut leaves);
    }
    for file_name in candidate_file_names {
        leaves.push(("$candidate_file_name".to_owned(), file_name.clone()));
    }

    // Field-name checks include nested objects and arrays.
    fn scan_keys(value: &serde_json::Value, path: &str, forbidden: &[&str], findings: &mut Vec<CandidateSurfaceFinding>) {
        match value {
            serde_json::Value::Object(fields) => {
                for (key, item) in fields {
                    if forbidden.contains(&key.as_str()) {
                        findings.push(CandidateSurfaceFinding {
                            location: format!("{path}.{key}"),
                            finding_kind: "forbidden_evaluator_field".to_owned(),
                            token_sha256: sha256_hex(key.as_bytes()),
                        });
                    }
                    scan_keys(item, &format!("{path}.{key}"), forbidden, findings);
                }
            }
            serde_json::Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    scan_keys(item, &format!("{path}[{index}]"), forbidden, findings);
                }
            }
            _ => {}
        }
    }
    scan_keys(&manifest, "$manifest", &forbidden_field_names(), &mut findings);
    for (index, raw) in candidate_episode_jsons.iter().enumerate() {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| ProjectionError::InvalidMetadataAudit("candidate episode JSON"))?;
        scan_keys(&value, &format!("$episode[{index}]"), &forbidden_field_names(), &mut findings);
    }

    let forbidden_values = receipts
        .iter()
        .flat_map(|receipt| receipt.forbidden_raw_values.iter().cloned())
        .chain(hidden_world_ids.iter().cloned())
        .chain(evaluator_secret_tokens.iter().cloned())
        .filter(|value| value.len() >= 8)
        .collect::<BTreeSet<_>>();
    for (location, value) in leaves {
        for forbidden in &forbidden_values {
            if value == *forbidden || value.contains(forbidden) {
                findings.push(CandidateSurfaceFinding {
                    location: location.clone(),
                    finding_kind: "raw_evaluator_value_leak".to_owned(),
                    token_sha256: sha256_hex(forbidden.as_bytes()),
                });
            }
        }
    }

    findings.sort_by(|left, right| {
        (&left.location, &left.finding_kind, &left.token_sha256)
            .cmp(&(&right.location, &right.finding_kind, &right.token_sha256))
    });
    findings.dedup();
    Ok(CandidateSurfaceAudit {
        schema_version: "aerb-candidate-surface-audit-v0.1".to_owned(),
        candidate_episode_n: candidate_episode_jsons.len(),
        projection_receipt_n: receipts.len(),
        finding_n: findings.len(),
        passed: findings.is_empty(),
        findings,
        interpretation: "Candidate-surface scan detects known forbidden field names and known raw evaluator tokens; it does not prove resistance to statistical, semantic, temporal, adaptive or stateful metadata inference.".to_owned(),
    })
}

/// Candidate-visible feature map for evaluator-side metadata-only attacks.
/// The path, file index and serialized length are included because they are
/// commonly overlooked leakage channels.
pub fn candidate_metadata_features(
    episode: &CandidateEpisode,
    file_order_index: usize,
    file_name: &str,
    serialized_size_bytes: usize,
) -> BTreeMap<String, String> {
    let mut features = BTreeMap::new();
    features.insert("episode_id".to_owned(), episode.episode_id.clone());
    features.insert("scenario_id".to_owned(), episode.scenario_id.clone());
    features.insert("attempt_id".to_owned(), episode.attempt_id.clone());
    features.insert("trajectory_cluster_id".to_owned(), episode.trajectory_cluster_id.clone());
    features.insert("evidence_domain".to_owned(), format!("{:?}", episode.evidence_domain));
    features.insert("file_order_index".to_owned(), file_order_index.to_string());
    for modulus in [2_usize, 3, 4, 5, 7, 8, 11, 13, 16, 17, 31, 32, 64, 127] {
        features.insert(format!("file_order_mod_{modulus}"), (file_order_index % modulus).to_string());
    }
    for (name, value) in [
        ("episode_id", episode.episode_id.as_str()),
        ("attempt_id", episode.attempt_id.as_str()),
    ] {
        let suffix = value.chars().rev().take_while(|character| character.is_ascii_digit()).collect::<String>();
        if !suffix.is_empty() {
            if let Ok(number) = suffix.chars().rev().collect::<String>().parse::<u64>() {
                for modulus in [2_u64, 3, 4, 5, 7, 8, 11, 13, 16, 17, 31, 32, 64, 127] {
                    features.insert(format!("{name}_suffix_mod_{modulus}"), (number % modulus).to_string());
                }
            }
        }
    }
    features.insert("file_name".to_owned(), file_name.to_owned());
    features.insert("serialized_size_bytes".to_owned(), serialized_size_bytes.to_string());
    features.insert("current_step".to_owned(), episode.current_step.to_string());
    features.insert("decision_deadline_step".to_owned(), episode.decision_deadline_step.to_string());
    features.insert("remaining_probe_budget_units".to_owned(), episode.remaining_probe_budget_units.to_bits().to_string());

    let context = &episode.command_context;
    features.insert("command_id".to_owned(), context.command_id.clone());
    features.insert("operation_id".to_owned(), context.operation_id.clone());
    features.insert("payload_sha256".to_owned(), context.payload_sha256.clone());
    features.insert("action_prefix_sha256".to_owned(), context.action_prefix_sha256.clone());
    features.insert("endpoint_id".to_owned(), context.endpoint_id.clone());
    features.insert("endpoint_profile_sha256".to_owned(), context.endpoint_profile_sha256.clone());
    features.insert("authority_epoch".to_owned(), context.authority_epoch.to_string());
    features.insert("queue_generation".to_owned(), context.queue_generation.to_string());
    features.insert("execution_scope_id".to_owned(), context.execution_scope_id.clone());
    features.insert("system_boundary_manifest_sha256".to_owned(), context.system_boundary_manifest_sha256.clone());
    features.insert("effect_producing_inventory_profile_sha256".to_owned(), context.effect_producing_inventory_profile_sha256.clone());

    for (index, observation) in episode.observations.iter().enumerate() {
        let prefix = format!("observation_{index}");
        features.insert(format!("{prefix}_id"), observation.observation_id.clone());
        features.insert(format!("{prefix}_source_id"), observation.source_id.clone());
        features.insert(format!("{prefix}_source_role"), format!("{:?}", observation.source_role));
        features.insert(format!("{prefix}_quality"), format!("{:?}", observation.quality));
        features.insert(format!("{prefix}_ingest_sequence"), observation.ingest_sequence.to_string());
        features.insert(format!("{prefix}_event_clock"), observation.event_time.clock_domain_id.clone());
        features.insert(format!("{prefix}_event_ticks"), observation.event_time.ticks_ns.to_string());
        features.insert(format!("{prefix}_event_uncertainty"), observation.event_time.uncertainty_ns.to_string());
        features.insert(format!("{prefix}_acquired_clock"), observation.acquired_time.clock_domain_id.clone());
        features.insert(format!("{prefix}_acquired_ticks"), observation.acquired_time.ticks_ns.to_string());
        features.insert(format!("{prefix}_acquired_uncertainty"), observation.acquired_time.uncertainty_ns.to_string());
        features.insert(format!("{prefix}_artifact_sha256"), observation.artifact_sha256.clone());
        features.insert(format!("{prefix}_kind"), observation.observation_kind.clone());
    }
    for (index, probe) in episode.permitted_probes.iter().enumerate() {
        let prefix = format!("probe_{index}");
        features.insert(format!("{prefix}_id"), probe.probe_id.clone());
        features.insert(format!("{prefix}_cost_bits"), probe.acquisition_cost_units.to_bits().to_string());
        features.insert(format!("{prefix}_delay"), probe.result_delay_steps.to_string());
    }
    features
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetadataAttackRow {
    /// Evaluator-only grouping key; never candidate-visible.
    pub campaign_id: String,
    pub episode_id: String,
    /// Grouped cross-validation must keep all rows in this cluster in one fold.
    pub evaluator_cluster_id: String,
    pub hidden_truth_world_id: String,
    pub candidate_visible_features: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetadataFeatureScore {
    pub feature_name: String,
    pub sample_n: usize,
    pub heldout_accuracy: f64,
    pub majority_baseline_accuracy: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetadataLeakageReport {
    pub schema_version: String,
    pub campaign_id: String,
    pub sample_n: usize,
    pub world_class_n: usize,
    pub independent_cluster_n: usize,
    pub fold_n: usize,
    pub majority_baseline_accuracy: f64,
    pub combined_metadata_only_accuracy: f64,
    pub combined_metadata_only_balanced_accuracy: f64,
    pub per_feature_scores: Vec<MetadataFeatureScore>,
    pub interpretation: String,
}

fn stable_fold(campaign_id: &str, cluster_id: &str, folds: usize) -> usize {
    let mut preimage = b"AERB-METADATA-GROUP-FOLD-V0.1\0".to_vec();
    append_length_prefixed(&mut preimage, campaign_id.as_bytes());
    append_length_prefixed(&mut preimage, cluster_id.as_bytes());
    let digest = sha256_hex(&preimage);
    usize::from_str_radix(&digest[..8], 16).unwrap_or(0) % folds
}

fn majority_world<'a, I>(rows: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a MetadataAttackRow>,
{
    let mut counts = BTreeMap::<&str, usize>::new();
    for row in rows {
        *counts.entry(&row.hidden_truth_world_id).or_default() += 1;
    }
    counts
        .into_iter()
        .max_by(|(left_world, left_n), (right_world, right_n)| {
            left_n.cmp(right_n).then_with(|| right_world.cmp(left_world))
        })
        .map(|(world, _)| world)
}

fn predict_univariate(
    train: &[&MetadataAttackRow],
    feature: &str,
    value: &str,
    majority: &str,
) -> String {
    let matching = train
        .iter()
        .filter(|row| row.candidate_visible_features.get(feature).is_some_and(|v| v == value));
    majority_world(matching).unwrap_or(majority).to_owned()
}

fn predict_naive_bayes(
    train: &[&MetadataAttackRow],
    features: &[String],
    row: &MetadataAttackRow,
) -> String {
    let mut class_counts = BTreeMap::<&str, usize>::new();
    let mut vocabularies = BTreeMap::<&str, BTreeSet<&str>>::new();
    let mut feature_counts =
        BTreeMap::<(&str, &str, &str), usize>::new();

    for sample in train {
        *class_counts
            .entry(&sample.hidden_truth_world_id)
            .or_default() += 1;
        for feature in features {
            if let Some(value) = sample.candidate_visible_features.get(feature) {
                vocabularies
                    .entry(feature)
                    .or_default()
                    .insert(value);
                *feature_counts
                    .entry((feature, value, &sample.hidden_truth_world_id))
                    .or_default() += 1;
            }
        }
    }

    let total = train.len().max(1) as f64;
    let mut best_world = String::new();
    let mut best_score = f64::NEG_INFINITY;
    for (world, class_n) in &class_counts {
        let mut score = ((*class_n as f64) / total).ln();
        for feature in features {
            let Some(value) = row.candidate_visible_features.get(feature) else {
                continue;
            };
            let vocabulary_n = vocabularies.get(feature.as_str()).map_or(0, BTreeSet::len);
            let category_n = *feature_counts
                .get(&(feature.as_str(), value.as_str(), *world))
                .unwrap_or(&0);
            let denominator = *class_n as f64 + vocabulary_n as f64 + 1.0;
            score += ((category_n as f64 + 1.0) / denominator).ln();
        }
        if score > best_score || (score == best_score && (best_world.is_empty() || *world < &best_world)) {
            best_score = score;
            best_world = (*world).to_owned();
        }
    }
    best_world
}

fn accuracy(
    predictions: &[(String, String)],
    world_classes: &[String],
) -> (f64, f64) {
    if predictions.is_empty() {
        return (0.0, 0.0);
    }
    let correct = predictions
        .iter()
        .filter(|(predicted, truth)| predicted == truth)
        .count();
    let accuracy = correct as f64 / predictions.len() as f64;
    let recalls = world_classes
        .iter()
        .map(|world| {
            let class_rows = predictions.iter().filter(|(_, truth)| truth == world).collect::<Vec<_>>();
            if class_rows.is_empty() {
                0.0
            } else {
                class_rows
                    .iter()
                    .filter(|(predicted, _)| predicted == world)
                    .count() as f64
                    / class_rows.len() as f64
            }
        })
        .collect::<Vec<_>>();
    let balanced = if recalls.is_empty() {
        0.0
    } else {
        recalls.iter().sum::<f64>() / recalls.len() as f64
    };
    (accuracy, balanced)
}

/// Evaluate single-feature and combined categorical attacks using grouped
/// cross-validation. Clusters never span train/test in the same fold.
pub fn audit_metadata_leakage(
    rows: &[MetadataAttackRow],
    requested_folds: usize,
) -> Result<MetadataLeakageReport, ProjectionError> {
    if rows.len() < 2 {
        return Err(ProjectionError::InvalidMetadataAudit("at least two rows required"));
    }
    let campaign_ids = rows
        .iter()
        .map(|row| row.campaign_id.as_str())
        .collect::<BTreeSet<_>>();
    if campaign_ids.len() != 1 || campaign_ids.first().is_none_or(|id| id.trim().is_empty()) {
        return Err(ProjectionError::InvalidMetadataAudit("one non-empty campaign ID required"));
    }
    let mut episode_ids = BTreeSet::new();
    let mut clusters = BTreeSet::new();
    let mut worlds = BTreeSet::new();
    for row in rows {
        if row.episode_id.trim().is_empty()
            || row.evaluator_cluster_id.trim().is_empty()
            || row.hidden_truth_world_id.trim().is_empty()
        {
            return Err(ProjectionError::InvalidMetadataAudit("empty audit identity"));
        }
        if !episode_ids.insert(row.episode_id.as_str()) {
            return Err(ProjectionError::InvalidMetadataAudit("duplicate episode ID"));
        }
        if !VALID_WORLD_IDS.contains(&row.hidden_truth_world_id.as_str()) {
            return Err(ProjectionError::InvalidMetadataAudit("unknown hidden world"));
        }
        if row.candidate_visible_features.is_empty() {
            return Err(ProjectionError::InvalidMetadataAudit("empty candidate feature map"));
        }
        clusters.insert(row.evaluator_cluster_id.as_str());
        worlds.insert(row.hidden_truth_world_id.as_str());
    }
    if clusters.len() < 2 {
        return Err(ProjectionError::TooFewGroups);
    }
    if requested_folds < 2 || requested_folds > clusters.len() {
        return Err(ProjectionError::TooManyFolds);
    }
    let campaign_id = rows[0].campaign_id.clone();
    let fold_by_cluster = clusters
        .iter()
        .map(|cluster| {
            (
                *cluster,
                stable_fold(&campaign_id, cluster, requested_folds),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let used_folds = fold_by_cluster.values().copied().collect::<BTreeSet<_>>();
    if used_folds.len() < 2 {
        return Err(ProjectionError::TooFewGroups);
    }

    let feature_names = rows
        .iter()
        .flat_map(|row| row.candidate_visible_features.keys().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let world_classes = worlds.iter().map(|world| (*world).to_owned()).collect::<Vec<_>>();

    let mut combined_predictions = Vec::new();
    let mut univariate_predictions =
        BTreeMap::<String, Vec<(String, String)>>::new();
    let mut baseline_predictions = Vec::new();

    for fold in 0..requested_folds {
        let train = rows
            .iter()
            .filter(|row| fold_by_cluster.get(row.evaluator_cluster_id.as_str()) != Some(&fold))
            .collect::<Vec<_>>();
        let test = rows
            .iter()
            .filter(|row| fold_by_cluster.get(row.evaluator_cluster_id.as_str()) == Some(&fold))
            .collect::<Vec<_>>();
        if train.is_empty() || test.is_empty() {
            continue;
        }
        let majority = majority_world(train.iter().copied())
            .ok_or(ProjectionError::TooFewGroups)?
            .to_owned();
        for row in test {
            let combined = predict_naive_bayes(&train, &feature_names, row);
            combined_predictions.push((combined, row.hidden_truth_world_id.clone()));
            baseline_predictions.push((majority.clone(), row.hidden_truth_world_id.clone()));
            for feature in &feature_names {
                let value = row
                    .candidate_visible_features
                    .get(feature)
                    .map(String::as_str)
                    .unwrap_or("<missing>");
                let prediction = predict_univariate(&train, feature, value, &majority);
                univariate_predictions
                    .entry(feature.clone())
                    .or_default()
                    .push((prediction, row.hidden_truth_world_id.clone()));
            }
        }
    }
    if combined_predictions.is_empty() {
        return Err(ProjectionError::TooFewGroups);
    }

    let (combined_accuracy, combined_balanced) =
        accuracy(&combined_predictions, &world_classes);
    let (majority_accuracy, _) = accuracy(&baseline_predictions, &world_classes);
    let per_feature_scores = univariate_predictions
        .into_iter()
        .map(|(feature_name, predictions)| {
            let (heldout_accuracy, _) = accuracy(&predictions, &world_classes);
            MetadataFeatureScore {
                feature_name,
                sample_n: predictions.len(),
                heldout_accuracy,
                majority_baseline_accuracy: majority_accuracy,
            }
        })
        .collect();

    Ok(MetadataLeakageReport {
        schema_version: "aerb-metadata-leakage-audit-v0.1".to_owned(),
        campaign_id,
        sample_n: combined_predictions.len(),
        world_class_n: world_classes.len(),
        independent_cluster_n: clusters.len(),
        fold_n: used_folds.len(),
        majority_baseline_accuracy: majority_accuracy,
        combined_metadata_only_accuracy: combined_accuracy,
        combined_metadata_only_balanced_accuracy: combined_balanced,
        per_feature_scores,
        interpretation: "Grouped held-out categorical attack baseline only. Above-baseline accuracy is a leakage signal requiring investigation; a low score does not prove absence of leakage. This audit does not test arbitrary neural, temporal, adaptive or stateful attackers.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(label: &str) -> String {
        sha256_hex(label.as_bytes())
    }

    fn episode(index: usize) -> CandidateEpisode {
        let event = 1_000_000_i64 + index as i64 * 1000;
        CandidateEpisode {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark: BENCHMARK.to_owned(),
            episode_id: format!("raw-episode-{index:04}"),
            scenario_id: format!("revealing-scenario-{index % 8}"),
            attempt_id: format!("attempt-{index:04}"),
            trajectory_cluster_id: format!("cluster-{}", index / 2),
            evidence_domain: EvidenceDomain::ProcessModel,
            world_vocabulary_version: WORLD_VOCABULARY_VERSION.to_owned(),
            decision_cut_sequence: 8,
            ledger_frontier_sequence: 8,
            current_step: 40 + index as u64,
            decision_deadline_step: 60 + index as u64,
            remaining_probe_budget_units: 5.0,
            command_context: CommandContext {
                command_id: format!("command-{index:04}"),
                operation_id: format!("operation-{index:04}"),
                payload_sha256: digest(&format!("payload-{index}")),
                action_prefix_sha256: digest(&format!("prefix-{index}")),
                endpoint_id: "arm-a".to_owned(),
                endpoint_profile_sha256: digest("endpoint-profile"),
                authority_epoch: 100 + index as u64,
                queue_generation: 20 + index as u64,
                execution_scope_id: "robot-cell-a".to_owned(),
                system_boundary_manifest_sha256: digest("system-boundary"),
                effect_producing_inventory_profile_sha256: digest("inventory-profile"),
            },
            observations: vec![
                Observation {
                    observation_id: format!("observation-{index}-a"),
                    source_id: "sensor-a".to_owned(),
                    source_role: SourceRole::CoordinatorLog,
                    quality: ObservationQuality::Admitted,
                    ingest_sequence: 4,
                    event_time: ClockStamp {
                        clock_domain_id: "clock-a".to_owned(),
                        ticks_ns: event - 100,
                        uncertainty_ns: 10,
                    },
                    acquired_time: ClockStamp {
                        clock_domain_id: "clock-a".to_owned(),
                        ticks_ns: event,
                        uncertainty_ns: 12,
                    },
                    artifact_sha256: digest(&format!("observation-a-{index}")),
                    observation_kind: "event-a".to_owned(),
                },
                Observation {
                    observation_id: format!("observation-{index}-b"),
                    source_id: "sensor-b".to_owned(),
                    source_role: SourceRole::EndpointJournal,
                    quality: ObservationQuality::Admitted,
                    ingest_sequence: 8,
                    event_time: ClockStamp {
                        clock_domain_id: "clock-b".to_owned(),
                        ticks_ns: event - 50,
                        uncertainty_ns: 20,
                    },
                    acquired_time: ClockStamp {
                        clock_domain_id: "clock-a".to_owned(),
                        ticks_ns: event + 50,
                        uncertainty_ns: 20,
                    },
                    artifact_sha256: digest(&format!("observation-b-{index}")),
                    observation_kind: "event-b".to_owned(),
                },
            ],
            permitted_probes: vec![PermittedProbe {
                probe_id: "read_endpoint_journal".to_owned(),
                acquisition_cost_units: 1.0,
                result_delay_steps: 2,
            }],
        }
    }

    #[test]
    fn projection_removes_raw_identifiers_and_absolute_clock_offsets() {
        let context = BlindProjectionContext::from_secret([0x5a; 32]);
        let original = episode(7);
        let (projected, receipt) =
            project_candidate_episode(&context, &original, "ep_0123456789abcdef0123456789abcdef")
                .expect("projection");
        let serialized = serde_json::to_string(&projected).expect("projected JSON");
        for forbidden in [
            original.episode_id.as_str(),
            original.scenario_id.as_str(),
            original.attempt_id.as_str(),
            original.trajectory_cluster_id.as_str(),
            original.command_context.command_id.as_str(),
            original.command_context.operation_id.as_str(),
            original.command_context.endpoint_id.as_str(),
            original.command_context.execution_scope_id.as_str(),
            "sensor-a",
            "sensor-b",
            "clock-a",
            "clock-b",
        ] {
            assert!(!serialized.contains(forbidden), "projection leaked {forbidden}");
        }
        assert!(projected.observations.iter().all(|observation| observation.event_time.ticks_ns >= 0));
        assert_eq!(projected.current_step, 0);
        assert_eq!(projected.decision_cut_sequence, 4);
        assert_eq!(projected.observations[0].ingest_sequence, 0);
        assert_eq!(projected.observations[1].ingest_sequence, 4);
        verify_projected_episode(&projected, &receipt).expect("receipt verifies");
    }

    #[test]
    fn projection_preserves_source_equality_but_not_source_names() {
        let context = BlindProjectionContext::from_secret([0x33; 32]);
        let mut first = episode(1);
        let mut second = episode(2);
        first.observations[0].source_id = "shared-source".to_owned();
        second.observations[0].source_id = "shared-source".to_owned();
        let (p1, _) =
            project_candidate_episode(&context, &first, "ep_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .expect("first");
        let (p2, _) =
            project_candidate_episode(&context, &second, "ep_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
                .expect("second");
        assert_eq!(p1.observations[0].source_id, p2.observations[0].source_id);
        assert_ne!(p1.observations[0].source_id, "shared-source");
    }

    #[test]
    fn projection_receipt_detects_candidate_episode_tampering() {
        let context = BlindProjectionContext::from_secret([0x12; 32]);
        let (mut projected, receipt) =
            project_candidate_episode(&context, &episode(2), "ep_cccccccccccccccccccccccccccccccc")
                .expect("projection");
        projected.observations[0].quality = ObservationQuality::Disputed;
        assert_eq!(
            verify_projected_episode(&projected, &receipt),
            Err(ProjectionError::InvalidProjectionReceipt("projected episode digest"))
        );
    }

    #[test]
    fn invalid_candidate_episode_id_is_rejected() {
        let context = BlindProjectionContext::from_secret([0x12; 32]);
        assert_eq!(
            project_candidate_episode(&context, &episode(2), "episode-0002"),
            Err(ProjectionError::InvalidOpaqueEpisodeId)
        );
    }

    #[test]
    fn metadata_attack_detects_a_scenario_label_side_channel() {
        let mut rows = Vec::new();
        for index in 0..160 {
            let world = VALID_WORLD_IDS[index % VALID_WORLD_IDS.len()];
            let mut features = BTreeMap::new();
            features.insert("scenario_alias".to_owned(), world.to_owned());
            features.insert("independent_noise".to_owned(), (index * 13 % 17).to_string());
            rows.push(MetadataAttackRow {
                campaign_id: "camp_test".to_owned(),
                episode_id: format!("ep_{index:032x}"),
                evaluator_cluster_id: format!("cluster-{}", index / 2),
                hidden_truth_world_id: world.to_owned(),
                candidate_visible_features: features,
            });
        }
        let report = audit_metadata_leakage(&rows, 5).expect("metadata audit");
        assert!(report.combined_metadata_only_accuracy > 0.99);
        let scenario_score = report
            .per_feature_scores
            .iter()
            .find(|score| score.feature_name == "scenario_alias")
            .expect("scenario feature");
        assert!(scenario_score.heldout_accuracy > 0.99);
    }

    #[test]
    fn grouped_metadata_folds_keep_clusters_wholly_in_one_partition() {
        let mut rows = Vec::new();
        for index in 0..80 {
            let mut features = BTreeMap::new();
            features.insert("opaque_id".to_owned(), format!("episode-{index}"));
            rows.push(MetadataAttackRow {
                campaign_id: "camp_grouped".to_owned(),
                episode_id: format!("episode-{index}"),
                evaluator_cluster_id: format!("cluster-{}", index / 4),
                hidden_truth_world_id: VALID_WORLD_IDS[index % VALID_WORLD_IDS.len()].to_owned(),
                candidate_visible_features: features,
            });
        }
        let clusters = rows.iter().map(|row| row.evaluator_cluster_id.as_str()).collect::<BTreeSet<_>>();
        let assignments = clusters.iter().map(|cluster| (*cluster, stable_fold("camp_grouped", cluster, 5))).collect::<BTreeMap<_,_>>();
        for cluster in clusters {
            let members = rows.iter().filter(|row| row.evaluator_cluster_id == cluster).collect::<Vec<_>>();
            let folds = members.iter().map(|row| assignments[row.evaluator_cluster_id.as_str()]).collect::<BTreeSet<_>>();
            assert_eq!(folds.len(), 1);
        }
    }

    #[test]
    fn metadata_attack_rejects_duplicate_episode_identity() {
        let row = MetadataAttackRow {
            campaign_id: "camp_test".to_owned(),
            episode_id: "ep_same".to_owned(),
            evaluator_cluster_id: "cluster-a".to_owned(),
            hidden_truth_world_id: VALID_WORLD_IDS[0].to_owned(),
            candidate_visible_features: BTreeMap::from([("x".to_owned(), "1".to_owned())]),
        };
        assert_eq!(
            audit_metadata_leakage(&[row.clone(), row], 2),
            Err(ProjectionError::InvalidMetadataAudit("duplicate episode ID"))
        );
    }

    #[test]
    fn candidate_surface_audit_detects_raw_evaluator_ids_and_hidden_labels() {
        let context = BlindProjectionContext::from_secret([0x42; 32]);
        let original = episode(11);
        let (projected, receipt) =
            project_candidate_episode(&context, &original, "ep_11111111111111111111111111111111")
                .expect("projection");
        let projected_json = serde_json::to_string(&projected).expect("episode JSON");
        let manifest = serde_json::json!({
            "schema_version": "aerb-blind-campaign-v0.1",
            "corpus_class": "blind_evaluation",
            "campaign_id": "camp_22222222222222222222222222222222",
            "campaign_commitment_sha256": "a".repeat(64),
            "episode_ids": ["ep_11111111111111111111111111111111"]
        });
        let raw_manifest = serde_json::to_string(&manifest).expect("manifest");
        let safe = audit_candidate_surface(
            &raw_manifest,
            &[projected_json.clone()],
            std::slice::from_ref(&receipt),
            &[],
            &["private-evaluator-salt-should-not-appear".to_owned()],
            &["ep_11111111111111111111111111111111.json".to_owned()],
        ).expect("safe surface audit");
        assert!(safe.passed, "{safe:?}");

        let leaked_json = projected_json.replace(
            &projected.episode_id,
            &original.episode_id,
        );
        let unsafe_audit = audit_candidate_surface(
            &raw_manifest,
            &[leaked_json],
            std::slice::from_ref(&receipt),
            &[VALID_WORLD_IDS[4].to_owned()],
            &["private-evaluator-salt-should-not-appear".to_owned()],
            &["raw-episode-0011.json".to_owned()],
        ).expect("unsafe surface audit is reportable");
        assert!(!unsafe_audit.passed);
        assert!(unsafe_audit.finding_n >= 3);
    }

    #[test]
    fn evaluator_secret_token_is_detected_even_when_not_a_schema_field() {
        let context = BlindProjectionContext::from_secret([0x10; 32]);
        let (projected, receipt) =
            project_candidate_episode(&context, &episode(1), "ep_33333333333333333333333333333333")
                .expect("projection");
        let mut raw = serde_json::to_string(&projected).expect("episode");
        raw.push_str("evaluator-secret-token-opaque-abcdef");
        let manifest = serde_json::json!({
            "episode_ids": ["ep_33333333333333333333333333333333"]
        });
        let report = audit_candidate_surface(
            &serde_json::to_string(&manifest).unwrap(),
            &[raw],
            std::slice::from_ref(&receipt),
            &[],
            &["evaluator-secret-token-opaque-abcdef".to_owned()],
            &["ep_33333333333333333333333333333333.json".to_owned()],
        ).expect("audit");
        assert!(!report.passed);
        assert!(report.findings.iter().any(|finding| finding.finding_kind == "raw_evaluator_value_leak"));
    }

}
