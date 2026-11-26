// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const SCHEMA_VERSION: &str = "0.1";
pub const BENCHMARK: &str = "AERB";
pub const WORLD_VOCABULARY_VERSION: &str = "aerb-ambiguous-command-history-v0.1";
pub const OPEN_WORLD_RESIDUAL: &str = "open_world_residual";
pub const SCOPE_CLOSURE_PROBE_ID: &str = "reconcile_effect_and_scope";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceDomain {
    FormalModel,
    ProcessModel,
    Simulator,
    Hil,
    Physical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    EndpointJournal,
    ControllerTelemetry,
    IndependentSensor,
    ModelOracle,
    Operator,
    CoordinatorLog,
    CommandHolderInventory,
    IndependentInventoryAdjudicator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationQuality {
    Admitted,
    Stale,
    Unknown,
    Disputed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Disposition {
    HoldReconcile,
    RequestObservation,
    Abstain,
    RecommendFreshAttempt,
}

impl Disposition {
    pub const ALL: [Self; 4] = [
        Self::HoldReconcile,
        Self::RequestObservation,
        Self::Abstain,
        Self::RecommendFreshAttempt,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::HoldReconcile => "hold_reconcile",
            Self::RequestObservation => "request_observation",
            Self::Abstain => "abstain",
            Self::RecommendFreshAttempt => "recommend_fresh_attempt",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchState {
    NotDispatched,
    Dispatched,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectState {
    None,
    Once,
    Partial,
    Duplicate,
    StaleEpoch,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityRelation {
    CurrentEpoch,
    StaleEpoch,
    UnresolvedHolder,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcknowledgementState {
    Received,
    Lost,
    Delayed,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ClockStamp {
    pub clock_domain_id: String,
    pub ticks_ns: i64,
    pub uncertainty_ns: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommandContext {
    pub command_id: String,
    pub operation_id: String,
    pub payload_sha256: String,
    pub action_prefix_sha256: String,
    pub endpoint_id: String,
    pub endpoint_profile_sha256: String,
    pub authority_epoch: u64,
    pub queue_generation: u64,
    pub execution_scope_id: String,
    pub system_boundary_manifest_sha256: String,
    pub effect_producing_inventory_profile_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub observation_id: String,
    pub source_id: String,
    pub source_role: SourceRole,
    pub quality: ObservationQuality,
    pub ingest_sequence: u64,
    pub event_time: ClockStamp,
    pub acquired_time: ClockStamp,
    pub artifact_sha256: String,
    pub observation_kind: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PermittedProbe {
    pub probe_id: String,
    pub acquisition_cost_units: f64,
    pub result_delay_steps: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateEpisode {
    pub schema_version: String,
    pub benchmark: String,
    pub episode_id: String,
    pub scenario_id: String,
    pub attempt_id: String,
    pub trajectory_cluster_id: String,
    pub evidence_domain: EvidenceDomain,
    pub world_vocabulary_version: String,
    pub decision_cut_sequence: u64,
    pub ledger_frontier_sequence: u64,
    pub current_step: u64,
    pub decision_deadline_step: u64,
    pub remaining_probe_budget_units: f64,
    pub command_context: CommandContext,
    pub observations: Vec<Observation>,
    pub permitted_probes: Vec<PermittedProbe>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateSubmission {
    pub schema_version: String,
    pub benchmark: String,
    pub episode_id: String,
    pub candidate_snapshot_sha256: String,
    pub model_sha256: String,
    pub observer_profile_sha256: String,
    pub possible_world_support_ids: Vec<String>,
    pub disposition: Disposition,
    pub requested_probe_id: Option<String>,
    pub reason_codes: Vec<String>,
    #[serde(default)]
    pub explanation_observation_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorWorld {
    pub world_id: String,
    pub dispatch_state: DispatchState,
    pub effect_state: EffectState,
    pub effect_count: u64,
    pub authority_relation: AuthorityRelation,
    pub acknowledgement_state: AcknowledgementState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionLoss {
    pub disposition: Disposition,
    pub world_id: String,
    pub loss: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeOutcomeSupport {
    pub outcome_id: String,
    pub possible_world_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSupport {
    pub probe_id: String,
    pub outcomes: Vec<ProbeOutcomeSupport>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvaluatorWorldPack {
    pub schema_version: String,
    pub benchmark: String,
    pub episode_id: String,
    pub hidden_truth_world_id: String,
    pub evaluator_profile_sha256: String,
    pub maximum_worst_case_loss: f64,
    pub worlds: Vec<EvaluatorWorld>,
    pub decision_losses: Vec<DecisionLoss>,
    pub probe_outcome_supports: Vec<ProbeSupport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAdmission {
    NotRequested,
    Eligible,
    NotPermitted,
    OverBudget,
    MissesDeadline,
    RequestMissing,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EvaluatorReport {
    pub schema_version: String,
    pub benchmark: String,
    pub episode_id: String,
    pub candidate_snapshot_sha256: String,
    pub evaluator_profile_sha256: String,
    pub candidate_contract_valid: bool,
    pub candidate_reason_codes: Vec<String>,
    pub scheduled_observation_n: usize,
    pub admitted_observation_n: usize,
    pub stale_observation_n: usize,
    pub unknown_observation_n: usize,
    pub disputed_observation_n: usize,
    pub possible_world_support_n: usize,
    pub evaluator_world_n: usize,
    pub hidden_truth_in_candidate_support: bool,
    pub open_world_residual_retained: bool,
    pub selected_disposition: Disposition,
    pub selected_disposition_worst_case_loss: Option<f64>,
    pub selected_disposition_admissible: bool,
    pub evaluator_truth_disposition_loss: f64,
    pub requested_probe_id: Option<String>,
    pub probe_admission: ProbeAdmission,
    pub probe_acquisition_cost_units: Option<f64>,
    pub probe_result_delay_steps: Option<u64>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluatorError {
    Json(String),
    UnsupportedSchema(String),
    IdentityMismatch(String),
    InvalidEpisode(String),
    InvalidSubmission(String),
    InvalidWorldPack(String),
    SnapshotDigestMismatch,
    CandidateArtifactMalformed(String),
}

impl std::fmt::Display for EvaluatorError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(reason) => write!(formatter, "JSON invalid: {reason}"),
            Self::UnsupportedSchema(reason) => write!(formatter, "unsupported schema: {reason}"),
            Self::IdentityMismatch(reason) => write!(formatter, "identity mismatch: {reason}"),
            Self::InvalidEpisode(reason) => write!(formatter, "candidate episode invalid: {reason}"),
            Self::InvalidSubmission(reason) => write!(formatter, "candidate submission invalid: {reason}"),
            Self::InvalidWorldPack(reason) => write!(formatter, "evaluator world pack invalid: {reason}"),
            Self::SnapshotDigestMismatch => write!(formatter, "candidate snapshot digest mismatch"),
            Self::CandidateArtifactMalformed(reason) => write!(formatter, "candidate artifact malformed: {reason}"),
        }
    }
}

impl std::error::Error for EvaluatorError {}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Version 0.1 uses serde_json's compact serialization of a Value.
/// serde_json's default map implementation sorts object keys; arrays retain order.
/// The profile deliberately restricts benchmark numeric fields to integers except
/// finite costs/losses, and is not asserted to be general RFC 8785 JCS.
pub fn canonical_sorted_json_sha256(raw_json: &str) -> Result<String, EvaluatorError> {
    let value: Value =
        serde_json::from_str(raw_json).map_err(|error| EvaluatorError::Json(error.to_string()))?;
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| EvaluatorError::Json(error.to_string()))?;
    Ok(sha256_hex(&bytes))
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_episode(episode: &CandidateEpisode) -> Result<(), EvaluatorError> {
    if episode.schema_version != SCHEMA_VERSION || episode.benchmark != BENCHMARK {
        return Err(EvaluatorError::UnsupportedSchema("candidate episode".to_owned()));
    }
    if episode.world_vocabulary_version != WORLD_VOCABULARY_VERSION {
        return Err(EvaluatorError::UnsupportedSchema("world vocabulary".to_owned()));
    }
    if ![
        episode.episode_id.as_str(),
        episode.scenario_id.as_str(),
        episode.attempt_id.as_str(),
        episode.trajectory_cluster_id.as_str(),
        episode.command_context.command_id.as_str(),
        episode.command_context.operation_id.as_str(),
        episode.command_context.endpoint_id.as_str(),
        episode.command_context.execution_scope_id.as_str(),
    ]
    .iter()
    .all(|value| nonempty(value))
    {
        return Err(EvaluatorError::InvalidEpisode("required identity field empty".to_owned()));
    }
    if episode.decision_cut_sequence != episode.ledger_frontier_sequence {
        return Err(EvaluatorError::InvalidEpisode("decision cut differs from ledger frontier".to_owned()));
    }
    if episode.current_step > episode.decision_deadline_step {
        return Err(EvaluatorError::InvalidEpisode("current step is beyond decision deadline".to_owned()));
    }
    if !episode.remaining_probe_budget_units.is_finite()
        || episode.remaining_probe_budget_units < 0.0
    {
        return Err(EvaluatorError::InvalidEpisode("remaining probe budget invalid".to_owned()));
    }
    for digest in [
        &episode.command_context.payload_sha256,
        &episode.command_context.action_prefix_sha256,
        &episode.command_context.endpoint_profile_sha256,
        &episode.command_context.system_boundary_manifest_sha256,
        &episode.command_context.effect_producing_inventory_profile_sha256,
    ] {
        if !is_sha256(digest) {
            return Err(EvaluatorError::InvalidEpisode("command context digest invalid".to_owned()));
        }
    }
    if episode.observations.is_empty() {
        return Err(EvaluatorError::InvalidEpisode("observation list empty".to_owned()));
    }

    let mut observation_ids = BTreeSet::new();
    let mut ingest_sequences = BTreeSet::new();
    for observation in &episode.observations {
        if !nonempty(&observation.observation_id)
            || !nonempty(&observation.source_id)
            || !nonempty(&observation.observation_kind)
            || !nonempty(&observation.event_time.clock_domain_id)
            || !nonempty(&observation.acquired_time.clock_domain_id)
        {
            return Err(EvaluatorError::InvalidEpisode("observation identity or clock empty".to_owned()));
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(EvaluatorError::InvalidEpisode("duplicate observation ID".to_owned()));
        }
        if !ingest_sequences.insert(observation.ingest_sequence) {
            return Err(EvaluatorError::InvalidEpisode("duplicate ingest sequence".to_owned()));
        }
        if observation.ingest_sequence > episode.decision_cut_sequence {
            return Err(EvaluatorError::InvalidEpisode("observation ingested after decision cut".to_owned()));
        }
        if !is_sha256(&observation.artifact_sha256) {
            return Err(EvaluatorError::InvalidEpisode("observation artifact digest invalid".to_owned()));
        }
        if observation.event_time.clock_domain_id == observation.acquired_time.clock_domain_id
            && observation.acquired_time.ticks_ns < observation.event_time.ticks_ns
        {
            return Err(EvaluatorError::InvalidEpisode("acquisition precedes event in same clock domain".to_owned()));
        }
    }

    let mut probe_ids = BTreeSet::new();
    for probe in &episode.permitted_probes {
        if !nonempty(&probe.probe_id) || !probe_ids.insert(probe.probe_id.as_str()) {
            return Err(EvaluatorError::InvalidEpisode("probe ID empty or duplicated".to_owned()));
        }
        if !probe.acquisition_cost_units.is_finite() || probe.acquisition_cost_units < 0.0 {
            return Err(EvaluatorError::InvalidEpisode("probe cost invalid".to_owned()));
        }
    }
    Ok(())
}

fn validate_submission(
    submission: &CandidateSubmission,
    episode: &CandidateEpisode,
    snapshot_sha256: &str,
) -> Result<(), EvaluatorError> {
    if submission.schema_version != SCHEMA_VERSION || submission.benchmark != BENCHMARK {
        return Err(EvaluatorError::UnsupportedSchema("candidate submission".to_owned()));
    }
    if submission.episode_id != episode.episode_id {
        return Err(EvaluatorError::IdentityMismatch("candidate submission episode ID".to_owned()));
    }
    if !is_sha256(&submission.model_sha256) || !is_sha256(&submission.observer_profile_sha256) {
        return Err(EvaluatorError::InvalidSubmission("model/profile digest invalid".to_owned()));
    }
    if submission.candidate_snapshot_sha256 != snapshot_sha256 {
        return Err(EvaluatorError::SnapshotDigestMismatch);
    }
    if submission.possible_world_support_ids.is_empty() {
        return Err(EvaluatorError::InvalidSubmission("possible-world support empty".to_owned()));
    }
    let unique = submission
        .possible_world_support_ids
        .iter()
        .collect::<BTreeSet<_>>();
    if unique.len() != submission.possible_world_support_ids.len() {
        return Err(EvaluatorError::InvalidSubmission("duplicate possible-world support ID".to_owned()));
    }
    let observation_ids = episode
        .observations
        .iter()
        .map(|observation| observation.observation_id.as_str())
        .collect::<BTreeSet<_>>();
    if submission
        .explanation_observation_ids
        .iter()
        .any(|observation_id| !observation_ids.contains(observation_id.as_str()))
    {
        return Err(EvaluatorError::InvalidSubmission("explanation cites unknown observation".to_owned()));
    }
    match submission.disposition {
        Disposition::RequestObservation if submission.requested_probe_id.is_none() => {
            return Err(EvaluatorError::InvalidSubmission("request_observation requires a probe ID".to_owned()));
        }
        Disposition::HoldReconcile
        | Disposition::Abstain
        | Disposition::RecommendFreshAttempt
            if submission.requested_probe_id.is_some() =>
        {
            return Err(EvaluatorError::InvalidSubmission("non-probe disposition must not request probe".to_owned()));
        }
        _ => {}
    }
    Ok(())
}

fn build_loss_table(
    pack: &EvaluatorWorldPack,
    worlds: &BTreeSet<&str>,
) -> Result<BTreeMap<(Disposition, String), f64>, EvaluatorError> {
    let mut table = BTreeMap::new();
    for row in &pack.decision_losses {
        if !worlds.contains(row.world_id.as_str()) {
            return Err(EvaluatorError::InvalidWorldPack("decision loss references unknown world".to_owned()));
        }
        if !row.loss.is_finite() || !(0.0..=1.0).contains(&row.loss) {
            return Err(EvaluatorError::InvalidWorldPack("decision loss outside [0,1]".to_owned()));
        }
        if table.insert((row.disposition, row.world_id.clone()), row.loss).is_some() {
            return Err(EvaluatorError::InvalidWorldPack("duplicate decision loss row".to_owned()));
        }
    }
    for world_id in worlds {
        for disposition in Disposition::ALL {
            if !table.contains_key(&(disposition, (*world_id).to_owned())) {
                return Err(EvaluatorError::InvalidWorldPack("decision loss matrix incomplete".to_owned()));
            }
        }
    }
    if table.len() != worlds.len() * Disposition::ALL.len() {
        return Err(EvaluatorError::InvalidWorldPack("decision loss matrix has extra rows".to_owned()));
    }
    Ok(table)
}

fn validate_world_pack<'a>(
    pack: &'a EvaluatorWorldPack,
    episode: &CandidateEpisode,
) -> Result<(BTreeMap<&'a str, &'a EvaluatorWorld>, BTreeMap<(Disposition, String), f64>), EvaluatorError> {
    if pack.schema_version != SCHEMA_VERSION || pack.benchmark != BENCHMARK {
        return Err(EvaluatorError::UnsupportedSchema("evaluator world pack".to_owned()));
    }
    if pack.episode_id != episode.episode_id {
        return Err(EvaluatorError::IdentityMismatch("evaluator world pack episode ID".to_owned()));
    }
    if !is_sha256(&pack.evaluator_profile_sha256) {
        return Err(EvaluatorError::InvalidWorldPack("evaluator profile digest invalid".to_owned()));
    }
    if !pack.maximum_worst_case_loss.is_finite()
        || !(0.0..=1.0).contains(&pack.maximum_worst_case_loss)
    {
        return Err(EvaluatorError::InvalidWorldPack("maximum worst-case loss threshold invalid".to_owned()));
    }
    if pack.worlds.is_empty() {
        return Err(EvaluatorError::InvalidWorldPack("world catalog empty".to_owned()));
    }
    let mut worlds = BTreeMap::new();
    for world in &pack.worlds {
        if !nonempty(&world.world_id) || worlds.insert(world.world_id.as_str(), world).is_some() {
            return Err(EvaluatorError::InvalidWorldPack("world ID empty or duplicated".to_owned()));
        }
    }
    if !worlds.contains_key(pack.hidden_truth_world_id.as_str()) {
        return Err(EvaluatorError::InvalidWorldPack("hidden truth not in world catalog".to_owned()));
    }
    if !worlds.contains_key(OPEN_WORLD_RESIDUAL) {
        return Err(EvaluatorError::InvalidWorldPack("open-world residual absent".to_owned()));
    }
    let world_ids = worlds.keys().copied().collect::<BTreeSet<_>>();
    let loss_table = build_loss_table(pack, &world_ids)?;

    let candidate_probe_ids = episode
        .permitted_probes
        .iter()
        .map(|probe| probe.probe_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut pack_probe_ids = BTreeSet::new();
    for probe in &pack.probe_outcome_supports {
        if !pack_probe_ids.insert(probe.probe_id.as_str()) {
            return Err(EvaluatorError::InvalidWorldPack("duplicate probe-support ID".to_owned()));
        }
        if !candidate_probe_ids.contains(probe.probe_id.as_str()) {
            return Err(EvaluatorError::InvalidWorldPack("probe support not in candidate menu".to_owned()));
        }
        if probe.outcomes.is_empty() {
            return Err(EvaluatorError::InvalidWorldPack("probe outcome set empty".to_owned()));
        }
        let mut outcome_ids = BTreeSet::new();
        let mut covered_worlds = BTreeSet::new();
        for outcome in &probe.outcomes {
            if !nonempty(&outcome.outcome_id) || !outcome_ids.insert(outcome.outcome_id.as_str()) {
                return Err(EvaluatorError::InvalidWorldPack("probe outcome ID empty or duplicated".to_owned()));
            }
            if outcome.possible_world_ids.is_empty() {
                return Err(EvaluatorError::InvalidWorldPack("probe outcome support empty".to_owned()));
            }
            let mut outcome_worlds = BTreeSet::new();
            for world_id in &outcome.possible_world_ids {
                if !world_ids.contains(world_id.as_str()) {
                    return Err(EvaluatorError::InvalidWorldPack("probe support references unknown world".to_owned()));
                }
                if !outcome_worlds.insert(world_id.as_str()) {
                    return Err(EvaluatorError::InvalidWorldPack("duplicate world in one outcome support".to_owned()));
                }
                covered_worlds.insert(world_id.as_str());
            }
        }
        if covered_worlds != world_ids {
            return Err(EvaluatorError::InvalidWorldPack("probe outcome supports do not cover world catalog".to_owned()));
        }

        if probe.probe_id != SCOPE_CLOSURE_PROBE_ID {
            if probe.outcomes.iter().any(|outcome| {
                !outcome.possible_world_ids.iter().any(|world_id| world_id == OPEN_WORLD_RESIDUAL)
            }) {
                return Err(EvaluatorError::InvalidWorldPack(
                    "ordinary observation probe attempts to eliminate open-world residual".to_owned()
                ));
            }
        } else {
            let has_unresolved_scope_outcome = probe.outcomes.iter().any(|outcome| {
                outcome.possible_world_ids.iter().any(|world_id| world_id == OPEN_WORLD_RESIDUAL)
            });
            let has_scope_closed_outcome = probe.outcomes.iter().any(|outcome| {
                !outcome.possible_world_ids.iter().any(|world_id| world_id == OPEN_WORLD_RESIDUAL)
            });
            if !has_unresolved_scope_outcome || !has_scope_closed_outcome {
                return Err(EvaluatorError::InvalidWorldPack(
                    "scope-closure probe must distinguish closed and unresolved scope outcomes".to_owned()
                ));
            }
        }
    }
    if pack_probe_ids != candidate_probe_ids {
        return Err(EvaluatorError::InvalidWorldPack("probe support menu differs from candidate permitted probes".to_owned()));
    }
    Ok((worlds, loss_table))
}

fn probe_admission(
    requested_probe_id: Option<&str>,
    disposition: Disposition,
    episode: &CandidateEpisode,
) -> (ProbeAdmission, Option<f64>, Option<u64>) {
    if disposition != Disposition::RequestObservation {
        return (ProbeAdmission::NotRequested, None, None);
    }
    let Some(probe_id) = requested_probe_id else {
        return (ProbeAdmission::RequestMissing, None, None);
    };
    let Some(probe) = episode.permitted_probes.iter().find(|probe| probe.probe_id == probe_id) else {
        return (ProbeAdmission::NotPermitted, None, None);
    };
    let cost = Some(probe.acquisition_cost_units);
    let delay = Some(probe.result_delay_steps);
    if probe.acquisition_cost_units > episode.remaining_probe_budget_units {
        return (ProbeAdmission::OverBudget, cost, delay);
    }
    if episode.current_step.saturating_add(probe.result_delay_steps) > episode.decision_deadline_step {
        return (ProbeAdmission::MissesDeadline, cost, delay);
    }
    (ProbeAdmission::Eligible, cost, delay)
}

pub fn evaluate_json(
    episode_json: &str,
    submission_json: &str,
    evaluator_world_pack_json: &str,
) -> Result<EvaluatorReport, EvaluatorError> {
    let episode: CandidateEpisode =
        serde_json::from_str(episode_json).map_err(|error| EvaluatorError::Json(error.to_string()))?;
    let submission: CandidateSubmission =
        serde_json::from_str(submission_json).map_err(|error| EvaluatorError::Json(error.to_string()))?;
    let pack: EvaluatorWorldPack =
        serde_json::from_str(evaluator_world_pack_json).map_err(|error| EvaluatorError::Json(error.to_string()))?;

    validate_episode(&episode)?;
    let snapshot_sha256 = canonical_sorted_json_sha256(episode_json)?;
    validate_submission(&submission, &episode, &snapshot_sha256)?;
    let (worlds, loss_table) = validate_world_pack(&pack, &episode)?;

    let world_ids = worlds.keys().copied().collect::<BTreeSet<_>>();
    let mut candidate_reasons = Vec::new();
    let mut support_ids = BTreeSet::new();
    for world_id in &submission.possible_world_support_ids {
        if !world_ids.contains(world_id.as_str()) {
            candidate_reasons.push("UNKNOWN_WORLD_IN_CANDIDATE_SUPPORT".to_owned());
        } else {
            support_ids.insert(world_id.as_str());
        }
    }
    let open_world_residual_retained = support_ids.contains(OPEN_WORLD_RESIDUAL);
    if !open_world_residual_retained {
        candidate_reasons.push("OPEN_WORLD_RESIDUAL_DROPPED".to_owned());
    }
    let truth_covered = support_ids.contains(pack.hidden_truth_world_id.as_str());
    if !truth_covered {
        candidate_reasons.push("HIDDEN_TRUTH_NOT_IN_CANDIDATE_SUPPORT".to_owned());
    }

    let mut worst_case_loss: Option<f64> = None;
    if support_ids.len() == submission.possible_world_support_ids.len() && !support_ids.is_empty() {
        let mut worst = 0.0_f64;
        for world_id in &support_ids {
            if let Some(loss) = loss_table.get(&(submission.disposition, (*world_id).to_owned())) {
                worst = worst.max(*loss);
            }
        }
        worst_case_loss = Some(worst);
    }
    let disposition_admissible = worst_case_loss
        .is_some_and(|loss| loss <= pack.maximum_worst_case_loss + 1.0e-12);
    if !disposition_admissible {
        candidate_reasons.push("SELECTED_DISPOSITION_EXCEEDS_HARD_RISK_CEILING_OR_SUPPORT_INVALID".to_owned());
    }

    let truth_loss = *loss_table
        .get(&(submission.disposition, pack.hidden_truth_world_id.clone()))
        .ok_or_else(|| EvaluatorError::InvalidWorldPack("truth loss row missing".to_owned()))?;
    let (probe_status, probe_cost, probe_delay) = probe_admission(
        submission.requested_probe_id.as_deref(),
        submission.disposition,
        &episode,
    );
    match probe_status {
        ProbeAdmission::Eligible | ProbeAdmission::NotRequested => {}
        ProbeAdmission::NotPermitted => candidate_reasons.push("PROBE_NOT_PERMITTED".to_owned()),
        ProbeAdmission::OverBudget => candidate_reasons.push("PROBE_OVER_BUDGET".to_owned()),
        ProbeAdmission::MissesDeadline => candidate_reasons.push("PROBE_MISSES_DECISION_DEADLINE".to_owned()),
        ProbeAdmission::RequestMissing => candidate_reasons.push("PROBE_REQUEST_MISSING".to_owned()),
    }

    let admitted_n = episode.observations.iter().filter(|row| row.quality == ObservationQuality::Admitted).count();
    let stale_n = episode.observations.iter().filter(|row| row.quality == ObservationQuality::Stale).count();
    let unknown_n = episode.observations.iter().filter(|row| row.quality == ObservationQuality::Unknown).count();
    let disputed_n = episode.observations.iter().filter(|row| row.quality == ObservationQuality::Disputed).count();
    let candidate_contract_valid = candidate_reasons.is_empty();

    Ok(EvaluatorReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK.to_owned(),
        episode_id: episode.episode_id.clone(),
        candidate_snapshot_sha256: snapshot_sha256,
        evaluator_profile_sha256: pack.evaluator_profile_sha256.clone(),
        candidate_contract_valid,
        candidate_reason_codes: candidate_reasons,
        scheduled_observation_n: episode.observations.len(),
        admitted_observation_n: admitted_n,
        stale_observation_n: stale_n,
        unknown_observation_n: unknown_n,
        disputed_observation_n: disputed_n,
        possible_world_support_n: support_ids.len(),
        evaluator_world_n: worlds.len(),
        hidden_truth_in_candidate_support: truth_covered,
        open_world_residual_retained,
        selected_disposition: submission.disposition,
        selected_disposition_worst_case_loss: worst_case_loss,
        selected_disposition_admissible: disposition_admissible,
        evaluator_truth_disposition_loss: truth_loss,
        requested_probe_id: submission.requested_probe_id,
        probe_admission: probe_status,
        probe_acquisition_cost_units: probe_cost,
        probe_result_delay_steps: probe_delay,
        claim_ceiling: "Evaluator-side software/process-model report only. No physical effect truth, causal identification, safety certification, simulator/HIL/physical qualification, author archive reproduction or SOTA claim.".to_owned(),
    })
}

pub fn canonical_report_json(report: &EvaluatorReport) -> Result<Vec<u8>, EvaluatorError> {
    serde_json::to_vec(report).map_err(|error| EvaluatorError::Json(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPISODE: &str =
        include_str!("../../fixtures/candidate_episode_synthetic_v0_1.json");
    const SUBMISSION: &str =
        include_str!("../../fixtures/candidate_submission_synthetic_v0_1.json");
    const WORLDPACK: &str =
        include_str!("../../fixtures/evaluator_only/world_pack_synthetic_v0_1.json");

    fn reseal_candidate_snapshot(snapshot: &Value, submission: &mut Value) {
        let bytes = serde_json::to_vec(snapshot).expect("canonical candidate snapshot bytes");
        submission["candidate_snapshot_sha256"] = serde_json::json!(sha256_hex(&bytes));
    }

    fn values() -> (Value, Value, Value) {
        (
            serde_json::from_str(EPISODE).expect("episode"),
            serde_json::from_str(SUBMISSION).expect("submission"),
            serde_json::from_str(WORLDPACK).expect("world pack"),
        )
    }

    #[test]
    fn known_answer_preserves_truth_and_open_world_residual() {
        let report = evaluate_json(EPISODE, SUBMISSION, WORLDPACK).expect("evaluation");
        assert!(report.candidate_contract_valid);
        assert!(report.hidden_truth_in_candidate_support);
        assert!(report.open_world_residual_retained);
        assert_eq!(report.possible_world_support_n, 8);
        assert_eq!(report.evaluator_world_n, 8);
        assert_eq!(report.probe_admission, ProbeAdmission::Eligible);
        assert_eq!(report.selected_disposition_worst_case_loss, Some(0.3));
        assert!(report.selected_disposition_admissible);
        assert!(!serde_json::to_string(&report).unwrap().contains("applied_once_ack_lost"));
    }

    #[test]
    fn candidate_snapshot_digest_tampering_is_rejected() {
        let (episode, mut submission, worldpack) = values();
        submission["candidate_snapshot_sha256"] = serde_json::json!("0".repeat(64));
        assert_eq!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ).unwrap_err(),
            EvaluatorError::SnapshotDigestMismatch
        );
    }

    #[test]
    fn dropping_open_world_residual_is_a_candidate_violation() {
        let (episode, mut submission, worldpack) = values();
        let support = submission["possible_world_support_ids"].as_array_mut().unwrap();
        support.retain(|value| value.as_str() != Some(OPEN_WORLD_RESIDUAL));
        let episode = serde_json::to_string(&episode).unwrap();
        let submission = serde_json::to_string(&submission).unwrap();
        let worldpack = serde_json::to_string(&worldpack).unwrap();
        let report = evaluate_json(&episode, &submission, &worldpack).expect("score invalid candidate");
        assert!(!report.candidate_contract_valid);
        assert!(!report.open_world_residual_retained);
        assert!(report.candidate_reason_codes.iter().any(|code| code == "OPEN_WORLD_RESIDUAL_DROPPED"));
    }

    #[test]
    fn truth_omission_is_reported_without_revealing_world_id() {
        let (episode, mut submission, worldpack) = values();
        submission["possible_world_support_ids"] = serde_json::json!(["not_dispatched", OPEN_WORLD_RESIDUAL]);
        let report = evaluate_json(
            &serde_json::to_string(&episode).unwrap(),
            &serde_json::to_string(&submission).unwrap(),
            &serde_json::to_string(&worldpack).unwrap()
        ).expect("score");
        assert!(!report.hidden_truth_in_candidate_support);
        assert!(!serde_json::to_string(&report).unwrap().contains("applied_once_ack_lost"));
    }

    #[test]
    fn hard_risk_ceiling_rejects_fresh_attempt_disposition() {
        let (episode, mut submission, worldpack) = values();
        submission["disposition"] = serde_json::json!("recommend_fresh_attempt");
        submission["requested_probe_id"] = Value::Null;
        let report = evaluate_json(
            &serde_json::to_string(&episode).unwrap(),
            &serde_json::to_string(&submission).unwrap(),
            &serde_json::to_string(&worldpack).unwrap()
        ).expect("score");
        assert!(!report.selected_disposition_admissible);
        assert!(report.selected_disposition_worst_case_loss.unwrap() > 0.45);
    }

    #[test]
    fn unknown_probe_is_not_permitted() {
        let (episode, mut submission, worldpack) = values();
        submission["requested_probe_id"] = serde_json::json!("unlisted_probe");
        let report = evaluate_json(
            &serde_json::to_string(&episode).unwrap(),
            &serde_json::to_string(&submission).unwrap(),
            &serde_json::to_string(&worldpack).unwrap()
        ).expect("score invalid candidate");
        assert_eq!(report.probe_admission, ProbeAdmission::NotPermitted);
        assert!(!report.candidate_contract_valid);
    }

    #[test]
    fn ordinary_probe_cannot_eliminate_open_world_residual() {
        let (episode, submission, mut worldpack) = values();
        worldpack["probe_outcome_supports"][0]["outcomes"][0]["possible_world_ids"] =
            serde_json::json!(["not_dispatched", "dispatched_no_effect"]);
        assert!(matches!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ),
            Err(EvaluatorError::InvalidWorldPack(_))
        ));
    }

    #[test]
    fn scope_closure_probe_must_preserve_an_unresolved_outcome_and_a_closed_outcome() {
        let (episode, submission, mut worldpack) = values();
        let index = worldpack["probe_outcome_supports"]
            .as_array().unwrap()
            .iter()
            .position(|probe| probe["probe_id"] == SCOPE_CLOSURE_PROBE_ID)
            .unwrap();
        worldpack["probe_outcome_supports"][index]["outcomes"][3]["possible_world_ids"] =
            serde_json::json!(["unresolved_command_holder"]);
        assert!(matches!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ),
            Err(EvaluatorError::InvalidWorldPack(_))
        ));
    }

    #[test]
    fn incomplete_probe_support_is_rejected_by_evaluator() {
        let (episode, submission, mut worldpack) = values();
        worldpack["probe_outcome_supports"][0]["outcomes"][0]["possible_world_ids"] = serde_json::json!(["not_dispatched"]);
        assert!(matches!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ),
            Err(EvaluatorError::InvalidWorldPack(_))
        ));
    }

    #[test]
    fn late_probe_is_rejected_by_deadline() {
        let (mut episode, mut submission, worldpack) = values();
        submission["requested_probe_id"] = serde_json::json!("independent_pose_observation");
        episode["current_step"] = serde_json::json!(53);
        reseal_candidate_snapshot(&episode, &mut submission);
        let report = evaluate_json(
            &serde_json::to_string(&episode).unwrap(),
            &serde_json::to_string(&submission).unwrap(),
            &serde_json::to_string(&worldpack).unwrap()
        ).expect("score late probe");
        assert_eq!(report.probe_admission, ProbeAdmission::MissesDeadline);
        assert!(!report.candidate_contract_valid);
    }

    #[test]
    fn over_budget_probe_is_rejected() {
        let (mut episode, mut submission, worldpack) = values();
        submission["requested_probe_id"] = serde_json::json!("independent_pose_observation");
        episode["remaining_probe_budget_units"] = serde_json::json!(1.0);
        reseal_candidate_snapshot(&episode, &mut submission);
        let report = evaluate_json(
            &serde_json::to_string(&episode).unwrap(),
            &serde_json::to_string(&submission).unwrap(),
            &serde_json::to_string(&worldpack).unwrap()
        ).expect("score over budget probe");
        assert_eq!(report.probe_admission, ProbeAdmission::OverBudget);
    }

    #[test]
    fn non_request_disposition_cannot_carry_probe_id() {
        let (episode, mut submission, worldpack) = values();
        submission["disposition"] = serde_json::json!("hold_reconcile");
        assert!(matches!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ),
            Err(EvaluatorError::InvalidSubmission(_))
        ));
    }

    #[test]
    fn incomplete_decision_loss_matrix_is_rejected() {
        let (episode, submission, mut worldpack) = values();
        worldpack["decision_losses"].as_array_mut().unwrap().pop();
        assert!(matches!(
            evaluate_json(
                &serde_json::to_string(&episode).unwrap(),
                &serde_json::to_string(&submission).unwrap(),
                &serde_json::to_string(&worldpack).unwrap()
            ),
            Err(EvaluatorError::InvalidWorldPack(_))
        ));
    }

    #[test]
    fn frozen_inputs_produce_identical_report_bytes() {
        let first = evaluate_json(EPISODE, SUBMISSION, WORLDPACK).expect("first");
        let second = evaluate_json(EPISODE, SUBMISSION, WORLDPACK).expect("second");
        assert_eq!(canonical_report_json(&first).unwrap(), canonical_report_json(&second).unwrap());
    }
}
