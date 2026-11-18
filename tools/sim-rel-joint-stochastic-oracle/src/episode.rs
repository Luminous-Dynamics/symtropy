//! Interactive episode adapter for the exact joint stochastic world.
//!
//! The provider owns the sampled mechanism, physical state, sensor state, and
//! deterministic simulation PRNG. Policy-visible views contain only the public
//! action catalogue, observations already returned by the world, time/step
//! counters, budget counters, and the caller's own attempt dispositions.
//!
//! The SplitMix64 stream is reproducible simulation randomness, not a
//! cryptographic secret. A blind evaluation still requires the independent
//! runner/oracle custody boundary described in the research protocol.

use super::{JointKernel, Rational};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Hard cap on one in-memory episode's retained action-attempt records.
pub const MAX_ATTEMPT_RECEIPTS: usize = 65_536;

// SplitMix64 emits 2^53 equiprobable values. Requiring at least two grid
// points per event avoids silently treating a positive sub-resolution event
// as a reliably sampleable trajectory outcome.
const MIN_SAMPLE_MASS: f64 = 2.0 / 9_007_199_254_740_992.0;

/// Resource and horizon limits for one interactive episode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EpisodeConfig {
    /// Maximum number of accepted interventions.
    pub max_attempts: u64,
    /// Maximum accumulated cost of accepted interventions.
    pub max_cost_units: u64,
    /// Maximum number of world transitions.
    pub max_world_steps: u64,
}

impl Default for EpisodeConfig {
    fn default() -> Self {
        Self {
            max_attempts: 8,
            max_cost_units: 64,
            max_world_steps: 8,
        }
    }
}

/// Public description of one candidate mechanism in a closed-family identification task.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicHypothesis {
    /// Exact candidate ID shared with the evaluator kernel; never identifies the sampled truth.
    pub hypothesis_id: String,
    /// Declared structural family used for grouped discovery/transfer metrics.
    pub structural_family: String,
    /// Plain-language candidate mechanism description.
    pub description: String,
}

/// Public meaning and cost of one intervention; no transition probabilities or hidden state.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicActionDescription {
    /// Action ID shared with the evaluator kernel.
    pub action_id: String,
    /// What the instrument/intervention does from the policy's point of view.
    pub description: String,
    /// Declared target of the intervention, e.g. sensor, process, or observation.
    pub target: String,
}

/// Public meaning of one categorical observation returned by the model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicObservationDescription {
    /// Observation ID shared with the evaluator kernel.
    pub observation_id: String,
    /// Public channel identity.
    pub channel_id: String,
    /// Human-readable interpretation of this category.
    pub description: String,
    /// Measurement unit or categorical scale name.
    pub unit: String,
}

/// Public task/profile information visible to the research policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicResearchProfile {
    /// Research objective and task scope.
    pub task_description: String,
    /// The declared finite candidate family. No generating-truth label is included.
    pub candidate_hypotheses: Vec<PublicHypothesis>,
    /// Public action descriptions and semantics.
    pub actions: Vec<PublicActionDescription>,
    /// Public observation-channel/category meanings.
    pub observations: Vec<PublicObservationDescription>,
}

/// Public, non-secret identities and semantics that bind one episode to its exact model inputs.
///
/// All digests are lowercase SHA-256 hex strings supplied by the trusted runner.
/// These values may be shown to the research policy; per-episode seeds and hidden
/// initial-state realizations must not be included here.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeManifest {
    /// Digest of the public episode contract and admissible action/observation surface.
    pub contract_sha256: String,
    /// Digest of the exact owner-model implementation.
    pub world_model_sha256: String,
    /// Digest of the exact selected model/profile and applicability declaration.
    pub model_profile_sha256: String,
    /// Digest of the world-generation recipe, excluding private episode seed/state.
    pub world_recipe_sha256: String,
    /// Digest of the clock semantics used by this episode.
    pub clock_profile_sha256: String,
    /// Digest of the episode's time-origin definition.
    pub time_origin_sha256: String,
    /// Policy-visible task semantics and finite candidate/action/observation catalogue.
    pub public_profile: PublicResearchProfile,
}

impl EpisodeManifest {
    fn validate(&self) -> Result<(), EpisodeError> {
        for digest in [
            self.contract_sha256.as_str(),
            self.world_model_sha256.as_str(),
            self.model_profile_sha256.as_str(),
            self.world_recipe_sha256.as_str(),
            self.clock_profile_sha256.as_str(),
            self.time_origin_sha256.as_str(),
        ] {
            if digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            {
                return Err(EpisodeError::InvalidManifest);
            }
        }
        let profile = &self.public_profile;
        if profile.task_description.trim().is_empty() || profile.task_description.len() > 4096 {
            return Err(EpisodeError::InvalidManifest);
        }
        if profile.candidate_hypotheses.is_empty()
            || profile.actions.is_empty()
            || profile.observations.is_empty()
        {
            return Err(EpisodeError::InvalidManifest);
        }
        let mut hypothesis_ids = BTreeSet::new();
        for hypothesis in &profile.candidate_hypotheses {
            if hypothesis.hypothesis_id.trim().is_empty()
                || hypothesis.structural_family.trim().is_empty()
                || hypothesis.description.trim().is_empty()
                || hypothesis.description.len() > 2048
                || !hypothesis_ids.insert(hypothesis.hypothesis_id.as_str())
            {
                return Err(EpisodeError::InvalidManifest);
            }
        }
        let mut action_ids = BTreeSet::new();
        for action in &profile.actions {
            if action.action_id.trim().is_empty()
                || action.description.trim().is_empty()
                || action.target.trim().is_empty()
                || action.description.len() > 2048
                || !action_ids.insert(action.action_id.as_str())
            {
                return Err(EpisodeError::InvalidManifest);
            }
        }
        let mut observation_ids = BTreeSet::new();
        for observation in &profile.observations {
            if observation.observation_id.trim().is_empty()
                || observation.channel_id.trim().is_empty()
                || observation.description.trim().is_empty()
                || observation.unit.trim().is_empty()
                || observation.description.len() > 2048
                || !observation_ids.insert(observation.observation_id.as_str())
            {
                return Err(EpisodeError::InvalidManifest);
            }
        }
        Ok(())
    }

    fn validate_against_kernel(&self, kernel: &JointKernel) -> Result<(), EpisodeError> {
        let profile = &self.public_profile;
        let candidate_ids: BTreeSet<&str> = profile
            .candidate_hypotheses
            .iter()
            .map(|item| item.hypothesis_id.as_str())
            .collect();
        let owner_hypothesis_ids: BTreeSet<&str> =
            kernel.hypothesis_ids.iter().map(String::as_str).collect();
        let action_ids: Vec<&str> = profile
            .actions
            .iter()
            .map(|item| item.action_id.as_str())
            .collect();
        let observation_ids: Vec<&str> = profile
            .observations
            .iter()
            .map(|item| item.observation_id.as_str())
            .collect();
        if candidate_ids.is_empty()
            || !candidate_ids.is_subset(&owner_hypothesis_ids)
            || action_ids.len() != kernel.action_ids.len()
            || action_ids
                .iter()
                .zip(&kernel.action_ids)
                .any(|(public, owner)| *public != owner.as_str())
            || observation_ids.len() != kernel.observation_ids.len()
            || observation_ids
                .iter()
                .zip(&kernel.observation_ids)
                .any(|(public, owner)| *public != owner.as_str())
        {
            return Err(EpisodeError::InvalidManifest);
        }
        Ok(())
    }
}

/// Public description of one action available to the research policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicAction {
    /// Stable action identity from the admitted model profile.
    pub action_id: String,
    /// Policy-visible description of this intervention.
    pub description: String,
    /// Target instrument or system boundary.
    pub target: String,
    /// Declared cost for an accepted intervention.
    pub cost_units: u64,
}

/// An observation that has been emitted by the owner model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeObservation {
    /// Stable observation identity from the admitted model profile.
    pub observation_id: String,
    /// Public description of the observed category.
    pub description: String,
    /// Measurement unit or categorical scale.
    pub unit: String,
    /// Discrete world step at which the modeled phenomenon occurred.
    pub event_step: u64,
    /// Discrete world step at which the model's observation channel acquired it.
    pub acquisition_step: u64,
    /// Discrete world step at which the observation became policy-visible.
    pub available_at_step: u64,
}

/// Disposition of one policy action request.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum AttemptDisposition {
    /// The intervention was executed and returned a safe observation.
    Applied {
        /// Observation emitted by the model.
        observation_id: String,
    },
    /// The intervention was executed and entered a terminal hazard state.
    HazardTerminal,
    /// No action with this identity exists in the public action catalogue.
    RejectedUnknownAction,
    /// The episode was already terminal.
    RejectedTerminal,
    /// The request exceeded the configured accepted-intervention count budget.
    RejectedAttemptBudget,
    /// The request would exceed the configured cumulative cost budget.
    RejectedCostBudget,
    /// The request exceeded the configured world-step horizon.
    RejectedStepBudget,
}

/// Append-only record for one policy action request.
///
/// Rejected requests are retained. requested_cost_units is the catalogue
/// cost when the action is known. charged_cost_units is non-zero only when
/// the owner model actually executed the intervention.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeAttempt {
    /// Stable episode-local attempt identity.
    pub attempt_id: String,
    /// Contiguous zero-based ordinal.
    pub ordinal: u64,
    /// Action identity supplied by the policy, including unknown IDs.
    pub requested_action_id: String,
    /// Catalogue cost, or None for an unknown action.
    pub requested_cost_units: Option<u64>,
    /// Cost charged for an executed intervention.
    pub charged_cost_units: u64,
    /// Decision step at which the request was received.
    pub requested_at_step: u64,
    /// World step at which the model executed the action; absent for rejected requests.
    pub executed_at_step: Option<u64>,
    /// Provider-owned outcome.
    pub disposition: AttemptDisposition,
}

/// Policy-visible snapshot of an interactive episode.
///
/// The hidden hypothesis index, physical state, sensor state, full joint
/// kernel, initial-belief realization, and PRNG seed are intentionally absent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeView {
    /// Opaque episode identity supplied by the trusted runner.
    pub episode_id: String,
    /// Public exact model/profile/recipe/clock identities.
    pub manifest: EpisodeManifest,
    /// Current discrete world step.
    pub current_step: u64,
    /// Public action catalogue.
    pub actions: Vec<PublicAction>,
    /// Observations already emitted and available by current_step.
    pub observations: Vec<EpisodeObservation>,
    /// Attempt receipts, including rejected and failed requests.
    pub attempts: Vec<EpisodeAttempt>,
    /// Number of accepted interventions.
    pub accepted_attempts: u64,
    /// Accumulated cost of accepted interventions.
    pub spent_cost_units: u64,
    /// Frozen accepted-intervention count budget.
    pub max_attempts: u64,
    /// Frozen cost budget.
    pub max_cost_units: u64,
    /// Frozen world-step horizon.
    pub max_world_steps: u64,
    /// Number of requests not individually retained after the receipt cap was reached.
    pub dropped_attempts: u64,
    /// False once any attempt receipt has been dropped; never promote an incomplete trace.
    pub trace_complete: bool,
    /// Agent-reported terminal epistemic status, if the policy has submitted one.
    pub finish: Option<EpisodeFinish>,
    /// Whether a hazard or terminal condition ended the episode.
    pub terminal: bool,
}

/// Protocol identifier shared by the Symtropy provider and external research agents.
pub const EPISODE_PROTOCOL_VERSION: &str = "symtropy-joint-world-episode-v0";

/// Agent-reported epistemic status at the end of an episode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeFinishDisposition {
    /// The available evidence supports a unique candidate mechanism.
    Identified,
    /// Multiple candidate mechanisms remain compatible with the evidence.
    Ambiguous,
    /// No declared candidate mechanism remains compatible with the evidence.
    NoCompatibleModel,
    /// The generating mechanism is outside the declared candidate family.
    OutsideCandidateFamily,
    /// The agent deliberately declines to make an identification claim.
    Abstain,
}

/// Final policy decision. This reports only epistemic status, not hidden truth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeFinish {
    /// Agent-reported epistemic status.
    pub disposition: EpisodeFinishDisposition,
    /// Exact candidate IDs the agent judges compatible with the visible evidence.
    pub compatible_hypothesis_ids: Vec<String>,
    /// Bounded rationale supplied by the policy.
    pub rationale: String,
}

/// Versioned request frame sent from the trusted runner to the policy process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeRequest {
    /// Exact wire protocol version.
    pub protocol_version: String,
    /// Policy-visible snapshot only; never contains the hidden joint state or kernel.
    pub view: EpisodeView,
}

/// Typed action returned by the policy process.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload", rename_all = "snake_case")]
pub enum EpisodeCommand {
    /// Apply one action from the public action catalogue.
    Probe {
        /// Public action identity.
        action_id: String,
    },
    /// Stop experimentation and submit an epistemic-status decision.
    Finish {
        /// Final decision and rationale.
        finish: EpisodeFinish,
    },
}

/// Versioned action frame returned by the policy process to the trusted runner.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EpisodeResponse {
    /// Exact wire protocol version.
    pub protocol_version: String,
    /// Command selected by the agent for the current public view.
    pub command: EpisodeCommand,
}

/// Evaluator-only score computed after the agent submits its final epistemic status.
///
/// This type must never be serialized back into an EpisodeResponse or made available
/// to the policy before its run is complete.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeEvaluation {
    /// Opaque episode identity.
    pub episode_id: String,
    /// Agent's final epistemic-status report.
    pub reported_disposition: EpisodeFinishDisposition,
    /// Exact compatible candidate IDs computed by the trusted evaluator.
    pub compatible_hypotheses: Vec<String>,
    /// Candidate IDs reported by the policy.
    pub claimed_compatible_hypotheses: Vec<String>,
    /// Exact equality between the reported candidate set and the evaluator version space.
    pub version_space_exact_match: bool,
    /// Number of correctly reported candidates.
    pub correct_candidate_count: usize,
    /// Whether the sampled generating hypothesis remains in the exact version space.
    pub generating_hypothesis_retained: bool,
    /// Whether the evidence identifies exactly one candidate.
    pub evidence_identifies_unique_hypothesis: bool,
    /// A false-unique claim: the agent reports unique identification when the exact version
    /// space is not a singleton or does not retain the generating hypothesis.
    pub false_unique_claim: bool,
    /// Abstention is epistemically safe because the exact version space is not a singleton.
    pub safe_abstention: bool,
    /// The agent abstained despite the exact evidence identifying one candidate.
    pub unnecessary_abstention: bool,
    /// Whether the reported epistemic status agrees with the exact version-space cardinality.
    pub disposition_correct: bool,
    /// Number of interventions actually applied by the provider.
    pub applied_interventions: usize,
    /// Total retained requests, including rejections.
    pub total_requests: usize,
}

/// Fail-closed episode-provider error.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EpisodeError {
    /// The joint stochastic model did not pass its owner validator.
    InvalidKernel(String),
    /// The episode ID is malformed.
    InvalidEpisodeId,
    /// One or more required public manifest digests are malformed.
    InvalidManifest,
    /// The configured horizon exceeds the joint oracle's supported horizon.
    InvalidConfig,
    /// A validated kernel unexpectedly lacks a transition row or observation.
    MissingTransitionData,
    /// A kernel row contains no outcomes.
    EmptyOutcomeRow,
    /// The floating-point sampler could not construct a valid distribution.
    InvalidSamplingDistribution,
    /// A positive rational event probability is below the 53-bit sampler resolution.
    SamplingResolutionTooCoarse,
    /// The action-attempt receipt cap was reached.
    AttemptReceiptLimit,
    /// An attempt ordinal overflowed.
    AttemptOrdinalOverflow,
    /// The total accepted cost overflowed.
    CostOverflow,
    /// The process peer used a different episode wire protocol version.
    InvalidProtocolVersion,
    /// A final decision contains invalid or overlong rationale.
    InvalidFinish,
    /// The agent tried to finish after a hazard or after an earlier finish.
    EpisodeAlreadyTerminal,
    /// The agent has not submitted a final epistemic-status decision.
    EpisodeNotFinished,
    /// The action trace is incomplete and cannot support evaluator qualification.
    IncompleteTrace,
    /// Exact version-space arithmetic or action reconstruction failed.
    EvaluationFailure(String),
}

/// One active scientific-world episode over a frozen exact joint kernel.
///
/// The hidden state is evaluator/provider-owned. Consumers receive cloned
/// EpisodeView values and cannot read or mutate the hidden tuple.
pub struct EpisodeProvider {
    episode_id: String,
    manifest: EpisodeManifest,
    kernel: JointKernel,
    hypothesis: usize,
    physical_state: usize,
    sensor_state: usize,
    rng: SplitMix64,
    config: EpisodeConfig,
    current_step: u64,
    accepted_attempts: u64,
    spent_cost_units: u64,
    terminal: bool,
    dropped_attempts: u64,
    trace_complete: bool,
    observations: Vec<EpisodeObservation>,
    attempts: Vec<EpisodeAttempt>,
    finish: Option<EpisodeFinish>,
}

impl EpisodeProvider {
    /// Admit a validated kernel, sample one private initial joint state, and
    /// create an episode. The supplied seed remains private to this provider.
    pub fn new(
        episode_id: impl Into<String>,
        manifest: EpisodeManifest,
        kernel: JointKernel,
        config: EpisodeConfig,
        seed: u64,
    ) -> Result<Self, EpisodeError> {
        let episode_id = episode_id.into();
        if !valid_episode_id(&episode_id) {
            return Err(EpisodeError::InvalidEpisodeId);
        }
        if config.max_world_steps > 12
            || config.max_attempts > MAX_ATTEMPT_RECEIPTS as u64
        {
            return Err(EpisodeError::InvalidConfig);
        }
        manifest.validate()?;
        kernel
            .validate()
            .map_err(EpisodeError::InvalidKernel)?;
        manifest.validate_against_kernel(&kernel)?;

        let mut rng = SplitMix64::new(seed);
        let selected = sample_index(
            &kernel.initial_belief,
            |point| point.mass,
            &mut rng,
        )?;
        let initial = &kernel.initial_belief[selected];
        let hypothesis = initial.hypothesis;
        let physical_state = initial.physical_state;
        let sensor_state = initial.sensor_state;

        Ok(Self {
            episode_id,
            manifest,
            kernel,
            hypothesis,
            physical_state,
            sensor_state,
            rng,
            config,
            current_step: 0,
            accepted_attempts: 0,
            spent_cost_units: 0,
            terminal: false,
            dropped_attempts: 0,
            trace_complete: true,
            observations: Vec::new(),
            attempts: Vec::new(),
            finish: None,
        })
    }

    /// Return a cloned policy-visible episode snapshot.
    ///
    /// Only observations whose available_at_step is no later than the
    /// current decision step are returned.
    pub fn view(&self) -> EpisodeView {
        let actions = self
            .kernel
            .action_ids
            .iter()
            .zip(self.kernel.action_costs.iter())
            .zip(self.manifest.public_profile.actions.iter())
            .map(|((action_id, cost_units), description)| PublicAction {
                action_id: action_id.clone(),
                description: description.description.clone(),
                target: description.target.clone(),
                cost_units: *cost_units,
            })
            .collect();

        let observations = self
            .observations
            .iter()
            .filter(|observation| observation.available_at_step <= self.current_step)
            .cloned()
            .collect();

        EpisodeView {
            episode_id: self.episode_id.clone(),
            manifest: self.manifest.clone(),
            current_step: self.current_step,
            actions,
            observations,
            attempts: self.attempts.clone(),
            accepted_attempts: self.accepted_attempts,
            spent_cost_units: self.spent_cost_units,
            max_attempts: self.config.max_attempts,
            max_cost_units: self.config.max_cost_units,
            max_world_steps: self.config.max_world_steps,
            dropped_attempts: self.dropped_attempts,
            trace_complete: self.trace_complete,
            finish: self.finish.clone(),
            terminal: self.terminal,
        }
    }

    /// Construct the versioned public request frame for an external policy process.
    pub fn request(&self) -> EpisodeRequest {
        EpisodeRequest {
            protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
            view: self.view(),
        }
    }

    /// Apply one response from an external policy process.
    ///
    /// This method validates the wire version, then applies a probe or an
    /// epistemic-status finish command. It never returns the hidden version
    /// space or true generating hypothesis in the public response.
    pub fn apply_response(
        &mut self,
        response: EpisodeResponse,
    ) -> Result<EpisodeView, EpisodeError> {
        if response.protocol_version != EPISODE_PROTOCOL_VERSION {
            return Err(EpisodeError::InvalidProtocolVersion);
        }
        match response.command {
            EpisodeCommand::Probe { action_id } => {
                self.submit_action(&action_id)?;
            }
            EpisodeCommand::Finish { finish } => {
                self.finish_episode(finish)?;
            }
        }
        Ok(self.view())
    }

    /// Record the policy's final epistemic-status decision.
    ///
    /// A finish decision is terminal. The evaluator result is not included in
    /// EpisodeView and must be computed separately after the child policy exits.
    pub fn finish_episode(&mut self, finish: EpisodeFinish) -> Result<(), EpisodeError> {
        if self.terminal || self.finish.is_some() {
            return Err(EpisodeError::EpisodeAlreadyTerminal);
        }
        if finish.rationale.trim().is_empty() || finish.rationale.len() > 4096 {
            return Err(EpisodeError::InvalidFinish);
        }
        let declared: BTreeSet<&str> = self
            .manifest
            .public_profile
            .candidate_hypotheses
            .iter()
            .map(|candidate| candidate.hypothesis_id.as_str())
            .collect();
        let mut claimed = BTreeSet::new();
        for hypothesis_id in &finish.compatible_hypothesis_ids {
            if !declared.contains(hypothesis_id.as_str())
                || !claimed.insert(hypothesis_id.as_str())
            {
                return Err(EpisodeError::InvalidFinish);
            }
        }
        self.finish = Some(finish);
        self.terminal = true;
        Ok(())
    }

    /// Compute the exact candidate version space from the public action/observation
    /// history. This evaluator-only method must run in the trusted runner after finish.
    pub fn evaluate_after_finish(&self) -> Result<EpisodeEvaluation, EpisodeError> {
        let finish = self
            .finish
            .as_ref()
            .ok_or(EpisodeError::EpisodeNotFinished)?;
        if !self.trace_complete {
            return Err(EpisodeError::IncompleteTrace);
        }
        if self.attempts.iter().any(|attempt| {
            matches!(&attempt.disposition, AttemptDisposition::HazardTerminal)
        }) {
            return Err(EpisodeError::EpisodeAlreadyTerminal);
        }

        let compatible_hypotheses = self.exact_compatible_hypotheses()?;
        let generating_id = self
            .kernel
            .hypothesis_ids
            .get(self.hypothesis)
            .ok_or_else(|| EpisodeError::EvaluationFailure(
                "sampled hypothesis index is outside the kernel catalogue".to_owned(),
            ))?;
        let generating_hypothesis_retained =
            compatible_hypotheses.iter().any(|candidate| candidate == generating_id);

        let exact_set: BTreeSet<&str> =
            compatible_hypotheses.iter().map(String::as_str).collect();
        let claimed_set: BTreeSet<&str> = finish
            .compatible_hypothesis_ids
            .iter()
            .map(String::as_str)
            .collect();
        let correct_candidate_count = claimed_set.intersection(&exact_set).count();
        let version_space_exact_match = claimed_set == exact_set;
        let evidence_identifies_unique_hypothesis = compatible_hypotheses.len() == 1;
        let claimed_generation = finish
            .compatible_hypothesis_ids
            .iter()
            .any(|candidate| candidate == generating_id);
        let false_unique_claim = finish.disposition == EpisodeFinishDisposition::Identified
            && (!evidence_identifies_unique_hypothesis
                || !generating_hypothesis_retained
                || !claimed_generation);
        let safe_abstention = finish.disposition == EpisodeFinishDisposition::Abstain
            && (!generating_hypothesis_retained || compatible_hypotheses.len() > 1);
        let unnecessary_abstention = finish.disposition == EpisodeFinishDisposition::Abstain
            && evidence_identifies_unique_hypothesis;
        let disposition_correct = match finish.disposition {
            EpisodeFinishDisposition::Identified => {
                evidence_identifies_unique_hypothesis && generating_hypothesis_retained
            }
            EpisodeFinishDisposition::Ambiguous => compatible_hypotheses.len() > 1
            // The current provider only admits in-support kernels. An empty
            // candidate set is a task/model contradiction and fails closed.
            EpisodeFinishDisposition::NoCompatibleModel => {
                compatible_hypotheses.is_empty() && !generating_hypothesis_retained
            }
            EpisodeFinishDisposition::OutsideCandidateFamily => !generating_hypothesis_retained,
            EpisodeFinishDisposition::Abstain => safe_abstention,
        };
        let applied_interventions = self
            .attempts
            .iter()
            .filter(|attempt| matches!(&attempt.disposition, AttemptDisposition::Applied { .. }))
            .count();

        Ok(EpisodeEvaluation {
            episode_id: self.episode_id.clone(),
            reported_disposition: finish.disposition,
            compatible_hypotheses,
            claimed_compatible_hypotheses: finish.compatible_hypothesis_ids.clone(),
            version_space_exact_match,
            correct_candidate_count,
            generating_hypothesis_retained,
            evidence_identifies_unique_hypothesis,
            false_unique_claim,
            safe_abstention,
            unnecessary_abstention,
            disposition_correct,
            applied_interventions,
            total_requests: self.attempts.len(),
        })
    }

    fn exact_compatible_hypotheses(&self) -> Result<Vec<String>, EpisodeError> {
        let applied: Vec<(&str, &str)> = self
            .attempts
            .iter()
            .filter_map(|attempt| match &attempt.disposition {
                AttemptDisposition::Applied { observation_id } => {
                    Some((attempt.requested_action_id.as_str(), observation_id.as_str()))
                }
                _ => None,
            })
            .collect();

        let mut compatible = Vec::new();
        for public_candidate in &self.manifest.public_profile.candidate_hypotheses {
            let hypothesis = self
                .kernel
                .hypothesis_ids
                .iter()
                .position(|owner_id| owner_id == &public_candidate.hypothesis_id)
                .ok_or_else(|| EpisodeError::EvaluationFailure(
                    "public candidate is absent from the owner kernel".to_owned(),
                ))?;
            let mut belief: BTreeMap<(usize, usize), Rational> = BTreeMap::new();
            for point in &self.kernel.initial_belief {
                if point.hypothesis == hypothesis {
                    let key = (point.physical_state, point.sensor_state);
                    let current = belief.get(&key).copied().unwrap_or(Rational::zero());
                    let mass = current.checked_add(point.mass).map_err(EpisodeError::EvaluationFailure)?;
                    belief.insert(key, mass);
                }
            }

            for (action_id, observation_id) in &applied {
                let action = self
                    .kernel
                    .action_ids
                    .iter()
                    .position(|candidate| candidate.as_str() == *action_id)
                    .ok_or_else(|| EpisodeError::EvaluationFailure(
                        "applied attempt references an action outside the kernel".to_owned(),
                    ))?;
                let observation = self
                    .kernel
                    .observation_ids
                    .iter()
                    .position(|candidate| candidate.as_str() == *observation_id)
                    .ok_or_else(|| EpisodeError::EvaluationFailure(
                        "applied attempt references an observation outside the kernel".to_owned(),
                    ))?;

                let mut next: BTreeMap<(usize, usize), Rational> = BTreeMap::new();
                for ((physical_state, sensor_state), prior_mass) in &belief {
                    let row = self
                        .kernel
                        .transition_row(hypothesis, *physical_state, *sensor_state, action)
                        .ok_or_else(|| EpisodeError::EvaluationFailure(
                            "validated kernel is missing a transition row".to_owned(),
                        ))?;
                    for outcome in &row.outcomes {
                        if outcome.observation == Some(observation) {
                            let key = (outcome.next_physical_state, outcome.next_sensor_state);
                            let weighted = prior_mass
                                .checked_mul(outcome.probability)
                                .map_err(EpisodeError::EvaluationFailure)?;
                            let current = next.get(&key).copied().unwrap_or(Rational::zero());
                            let mass = current
                                .checked_add(weighted)
                                .map_err(EpisodeError::EvaluationFailure)?;
                            next.insert(key, mass);
                        }
                    }
                }
                belief = next;
                if belief.is_empty() {
                    break;
                }
            }

            if !belief.is_empty() {
                compatible.push(public_candidate.hypothesis_id.clone());
            }
        }
        Ok(compatible)
    }

    /// Submit one intervention request and retain its disposition.
    ///
    /// Requests are recorded even when the action is unknown, the episode is
    /// terminal, or a budget/horizon gate rejects the request. Rejected
    /// requests do not mutate the hidden world state and do not incur action
    /// cost. This provider's first profile is synchronous and discrete:
    /// event, acquisition, and availability steps share the resulting world
    /// step. Asynchronous observation timing requires a richer owner model.
    pub fn submit_action(
        &mut self,
        action_id: &str,
    ) -> Result<EpisodeAttempt, EpisodeError> {
        // A submitted finish is a hard protocol boundary. Do not append
        // post-submission requests to the terminal evidence trace.
        if self.finish.is_some() {
            return Err(EpisodeError::EpisodeAlreadyTerminal);
        }
        if self.attempts.len() >= MAX_ATTEMPT_RECEIPTS {
            self.dropped_attempts = self.dropped_attempts.saturating_add(1);
            self.trace_complete = false;
            self.terminal = true;
            return Err(EpisodeError::AttemptReceiptLimit);
        }

        let ordinal = u64::try_from(self.attempts.len())
            .map_err(|_| EpisodeError::AttemptOrdinalOverflow)?;
        let attempt_id = format!("attempt-{ordinal:08}");
        let requested_at_step = self.current_step;
        let action_index = self
            .kernel
            .action_ids
            .iter()
            .position(|candidate| candidate.as_str() == action_id);
        let requested_cost_units = action_index.map(|index| self.kernel.action_costs[index]);

        let disposition = match action_index {
            None => AttemptDisposition::RejectedUnknownAction,
            Some(_) if self.terminal => AttemptDisposition::RejectedTerminal,
            Some(_) if self.accepted_attempts >= self.config.max_attempts => {
                AttemptDisposition::RejectedAttemptBudget
            }
            Some(_) if self.current_step >= self.config.max_world_steps => {
                AttemptDisposition::RejectedStepBudget
            }
            Some(action_index) => {
                let action_cost = self.kernel.action_costs[action_index];
                let proposed_cost = self
                    .spent_cost_units
                    .checked_add(action_cost)
                    .ok_or(EpisodeError::CostOverflow)?;
                if proposed_cost > self.config.max_cost_units {
                    AttemptDisposition::RejectedCostBudget
                } else {
                    let outcome = {
                        let row = self
                            .kernel
                            .transition_row(
                                self.hypothesis,
                                self.physical_state,
                                self.sensor_state,
                                action_index,
                            )
                            .ok_or(EpisodeError::MissingTransitionData)?;
                        let outcome_index = sample_index(
                            &row.outcomes,
                            |item| item.probability,
                            &mut self.rng,
                        )?;
                        row.outcomes
                            .get(outcome_index)
                            .ok_or(EpisodeError::MissingTransitionData)?
                            .clone()
                    };

                    let observation_id = match outcome.observation {
                        Some(observation_index) => Some(
                            self.kernel
                                .observation_ids
                                .get(observation_index)
                                .ok_or(EpisodeError::MissingTransitionData)?
                                .clone(),
                        ),
                        None => None,
                    };

                    self.spent_cost_units = proposed_cost;
                    self.accepted_attempts += 1;
                    self.current_step += 1;
                    self.physical_state = outcome.next_physical_state;
                    self.sensor_state = outcome.next_sensor_state;

                    match observation_id {
                        Some(observation_id) => {
                            let description = self
                                .manifest
                                .public_profile
                                .observations
                                .iter()
                                .find(|item| item.observation_id == observation_id)
                                .ok_or(EpisodeError::MissingTransitionData)?;
                            self.observations.push(EpisodeObservation {
                                observation_id: observation_id.clone(),
                                description: description.description.clone(),
                                unit: description.unit.clone(),
                                event_step: self.current_step,
                                acquisition_step: self.current_step,
                                available_at_step: self.current_step,
                            });
                            AttemptDisposition::Applied { observation_id }
                        }
                        None => {
                            self.terminal = true;
                            AttemptDisposition::HazardTerminal
                        }
                    }
                }
            }
        };

        let charged_cost_units = if matches!(
            &disposition,
            AttemptDisposition::Applied { .. } | AttemptDisposition::HazardTerminal
        ) {
            requested_cost_units.unwrap_or(0)
        } else {
            0
        };

        let executed_at_step = (charged_cost_units > 0).then_some(self.current_step);
        let receipt = EpisodeAttempt {
            attempt_id,
            ordinal,
            requested_action_id: action_id.to_owned(),
            requested_cost_units,
            charged_cost_units,
            requested_at_step,
            executed_at_step,
            disposition,
        };
        self.attempts.push(receipt.clone());
        Ok(receipt)
    }
}

fn valid_episode_id(value: &str) -> bool {
    value.len() >= 16
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Deterministic, non-cryptographic SplitMix64 simulation stream.
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    /// Uniform value on [0, 1) with 53 bits of mantissa precision.
    fn next_unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / 9_007_199_254_740_992.0
    }
}

fn rational_as_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

/// Sample an outcome from an exactly normalized rational row using a
/// reproducible 53-bit floating-point draw.
///
/// The exact joint oracle remains the authority for exact finite-horizon
/// probability/frontier calculations. This trajectory sampler is a bounded
/// simulation adapter, not an exact sampler and not cryptographic randomness.
fn sample_index<T>(
    values: &[T],
    mass: impl Fn(&T) -> Rational,
    rng: &mut SplitMix64,
) -> Result<usize, EpisodeError> {
    if values.is_empty() {
        return Err(EpisodeError::EmptyOutcomeRow);
    }

    let mut weights = Vec::with_capacity(values.len());
    let mut total = 0.0_f64;
    for value in values {
        let weight = rational_as_f64(mass(value));
        if !weight.is_finite() || weight < 0.0 {
            return Err(EpisodeError::InvalidSamplingDistribution);
        }
        if weight < MIN_SAMPLE_MASS {
            return Err(EpisodeError::SamplingResolutionTooCoarse);
        }
        total += weight;
        weights.push(weight);
    }
    if !total.is_finite() || total <= 0.0 {
        return Err(EpisodeError::InvalidSamplingDistribution);
    }

    let threshold = rng.next_unit_f64() * total;
    let mut cumulative = 0.0_f64;
    for (index, weight) in weights.iter().enumerate() {
        cumulative += weight;
        if threshold < cumulative {
            return Ok(index);
        }
    }

    // Floating-point summation can round the final cumulative value down.
    // The exact kernel validator has already established positive total mass.
    Ok(values.len() - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{JointMass, JointOutcome, JointKernel, KernelRow};

    fn deterministic_kernel(hazard: bool) -> JointKernel {
        let outcomes = if hazard {
            vec![JointOutcome {
                probability: Rational::one(),
                next_physical_state: 0,
                next_sensor_state: 0,
                observation: None,
            }]
        } else {
            vec![JointOutcome {
                probability: Rational::one(),
                next_physical_state: 0,
                next_sensor_state: 0,
                observation: Some(0),
            }]
        };

        JointKernel {
            hypothesis_ids: vec!["hidden-mechanism-0".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".to_owned()],
            observation_ids: vec!["signal".to_owned()],
            action_costs: vec![2],
            initial_belief: vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            rows: vec![KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes,
            }],
        }
    }

    fn persistent_sensor_confounding_kernel() -> JointKernel {
        let mut rows = Vec::new();
        for hypothesis in 0..2 {
            for physical_state in 0..2 {
                for sensor_state in 0..2 {
                    for action in 0..2 {
                        let outcome = if action == 0 {
                            JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: physical_state,
                                next_sensor_state: sensor_state,
                                observation: Some(physical_state ^ sensor_state),
                            }
                        } else {
                            JointOutcome {
                                probability: Rational::one(),
                                next_physical_state: physical_state,
                                next_sensor_state: 0,
                                observation: Some(0),
                            }
                        };
                        rows.push(KernelRow {
                            hypothesis,
                            physical_state,
                            sensor_state,
                            action,
                            outcomes: vec![outcome],
                        });
                    }
                }
            }
        }

        JointKernel {
            hypothesis_ids: vec!["latent-mechanism-a".to_owned(), "latent-mechanism-b".to_owned()],
            physical_state_count: 2,
            sensor_state_count: 2,
            action_ids: vec!["measure-combined-signal".to_owned(), "calibrate-sensor".to_owned()],
            observation_ids: vec!["zero".to_owned(), "one".to_owned()],
            action_costs: vec![1, 1],
            initial_belief: vec![
                JointMass {
                    hypothesis: 0,
                    physical_state: 0,
                    sensor_state: 1,
                    mass: Rational::new(1, 2).unwrap(),
                },
                JointMass {
                    hypothesis: 1,
                    physical_state: 1,
                    sensor_state: 0,
                    mass: Rational::new(1, 2).unwrap(),
                },
            ],
            rows,
        }
    }

    fn stochastic_kernel() -> JointKernel {
        JointKernel {
            hypothesis_ids: vec!["private-h0".to_owned()],
            physical_state_count: 1,
            sensor_state_count: 1,
            action_ids: vec!["measure".to_owned()],
            observation_ids: vec!["signal-a".to_owned(), "signal-b".to_owned()],
            action_costs: vec![1],
            initial_belief: vec![JointMass {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                mass: Rational::one(),
            }],
            rows: vec![KernelRow {
                hypothesis: 0,
                physical_state: 0,
                sensor_state: 0,
                action: 0,
                outcomes: vec![
                    JointOutcome {
                        probability: Rational::new(1, 2).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(0),
                    },
                    JointOutcome {
                        probability: Rational::new(1, 2).unwrap(),
                        next_physical_state: 0,
                        next_sensor_state: 0,
                        observation: Some(1),
                    },
                ],
            }],
        }
    }

    fn test_manifest(kernel: &JointKernel) -> EpisodeManifest {
        let candidate_hypotheses = kernel
            .hypothesis_ids
            .iter()
            .map(|id| PublicHypothesis {
                hypothesis_id: id.clone(),
                structural_family: "finite-test-family".to_owned(),
                description: format!(
                    "Declared candidate mechanism {id}; latent physical and sensor states remain hidden."
                ),
            })
            .collect();
        let actions = kernel
            .action_ids
            .iter()
            .map(|id| {
                let (description, target) = match id.as_str() {
                    "measure-combined-signal" => (
                        "Measure the combined categorical signal through the current sensor state.",
                        "observation_channel",
                    ),
                    "calibrate-sensor" => (
                        "Calibrate the sensor channel. In this declared profile this changes sensor state, not the underlying process state.",
                        "sensor",
                    ),
                    "measure" => (
                        "Acquire one categorical measurement from the declared observation channel.",
                        "observation_channel",
                    ),
                    "probe.input" => (
                        "Apply the declared bounded input intervention and observe its categorical output.",
                        "process",
                    ),
                    "dangerous-test" => (
                        "Apply the declared hazardous test action; the evaluator owns its hazard outcome.",
                        "process",
                    ),
                    _ => ("Apply the declared finite intervention.", "process"),
                };
                PublicActionDescription {
                    action_id: id.clone(),
                    description: description.to_owned(),
                    target: target.to_owned(),
                }
            })
            .collect();
        let observations = kernel
            .observation_ids
            .iter()
            .map(|id| {
                let (description, channel_id, unit) = match id.as_str() {
                    "zero" => (
                        "The instrument returned the zero category.",
                        "combined-signal",
                        "categorical",
                    ),
                    "one" => (
                        "The instrument returned the one category.",
                        "combined-signal",
                        "categorical",
                    ),
                    "signal" => (
                        "The instrument returned its declared signal category.",
                        "signal",
                        "categorical",
                    ),
                    "signal-a" => (
                        "The instrument returned category A.",
                        "signal",
                        "categorical",
                    ),
                    "signal-b" => (
                        "The instrument returned category B.",
                        "signal",
                        "categorical",
                    ),
                    _ => ("The instrument returned a declared categorical observation.", "signal", "categorical"),
                };
                PublicObservationDescription {
                    observation_id: id.clone(),
                    channel_id: channel_id.to_owned(),
                    description: description.to_owned(),
                    unit: unit.to_owned(),
                }
            })
            .collect();
        EpisodeManifest {
            contract_sha256: "a".repeat(64),
            world_model_sha256: "b".repeat(64),
            model_profile_sha256: "c".repeat(64),
            world_recipe_sha256: "d".repeat(64),
            clock_profile_sha256: "e".repeat(64),
            time_origin_sha256: "f".repeat(64),
            public_profile: PublicResearchProfile {
                task_description: "Identify which declared candidate mechanism is compatible with the intervention/observation history. Preserve multiple candidates when the available experiments do not identify a unique one.".to_owned(),
                candidate_hypotheses,
                actions,
                observations,
            },
        }
    }

    fn new_provider(kernel: JointKernel, seed: u64) -> EpisodeProvider {
        let manifest = test_manifest(&kernel);
        EpisodeProvider::new(
            "opaque-episode-0001",
            manifest,
            kernel,
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 10,
                max_world_steps: 4,
            },
            seed,
        )
        .expect("valid episode")
    }

    #[test]
    fn public_profile_ids_must_match_the_owner_kernel_catalogue_exactly() {
        let kernel = persistent_sensor_confounding_kernel();
        let mut manifest = test_manifest(&kernel);
        manifest.public_profile.candidate_hypotheses.swap(0, 1);
        assert!(matches!(
            EpisodeProvider::new(
                "profile-order-mismatch-0001",
                manifest,
                kernel,
                EpisodeConfig::default(),
                3,
            ),
            Err(EpisodeError::InvalidManifest)
        ));
    }

    #[test]
    fn checked_in_wire_examples_deserialize_to_typed_protocol_frames() {
        let request: EpisodeRequest = serde_json::from_str(include_str!(
            "../../../benchmarks/scientific-world-episodes/v0.1/wire/episode-request.example.json"
        ))
        .expect("request fixture must match the Rust protocol type");
        assert_eq!(request.protocol_version, EPISODE_PROTOCOL_VERSION);
        assert_eq!(request.view.actions.len(), 2);
        assert_eq!(request.view.manifest.public_profile.candidate_hypotheses.len(), 2);

        let probe: EpisodeResponse = serde_json::from_str(include_str!(
            "../../../benchmarks/scientific-world-episodes/v0.1/wire/episode-response-probe.example.json"
        ))
        .expect("probe fixture must match the Rust protocol type");
        assert!(matches!(
            probe.command,
            EpisodeCommand::Probe { ref action_id } if action_id == "calibrate-sensor"
        ));

        let finish: EpisodeResponse = serde_json::from_str(include_str!(
            "../../../benchmarks/scientific-world-episodes/v0.1/wire/episode-response-finish.example.json"
        ))
        .expect("finish fixture must match the Rust protocol type");
        match finish.command {
            EpisodeCommand::Finish { finish } => {
                assert_eq!(
                    finish.compatible_hypothesis_ids,
                    vec![
                        "latent-mechanism-a".to_owned(),
                        "latent-mechanism-b".to_owned()
                    ]
                );
            }
            other => panic!("expected finish fixture, got {other:?}"),
        }
    }

    #[test]
    fn status_correctness_is_separate_from_candidate_identity_correctness() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        for action in [
            "measure-combined-signal",
            "calibrate-sensor",
            "measure-combined-signal",
        ] {
            provider.submit_action(action).expect("apply known intervention");
        }
        provider
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Identified,
                compatible_hypothesis_ids: vec!["latent-mechanism-b".to_owned()],
                rationale: "The evidence identifies one mechanism, but this candidate label is deliberately wrong.".to_owned(),
            })
            .expect("finish with an incorrect singleton identity");

        let evaluation = provider.evaluate_after_finish().unwrap();
        assert!(evaluation.evidence_identifies_unique_hypothesis);
        assert!(evaluation.disposition_correct);
        assert!(!evaluation.version_space_exact_match);
        assert_eq!(evaluation.correct_candidate_count, 0);
        assert!(evaluation.false_unique_claim);
    }

    #[test]
    fn versioned_wire_frames_round_trip_without_hidden_kernel_or_seed() {
        let provider = new_provider(persistent_sensor_confounding_kernel(), 991);
        let request = provider.request();
        let encoded_request = serde_json::to_vec(&request).expect("serialize public request");
        let request_json = String::from_utf8(encoded_request.clone()).expect("JSON is UTF-8");
        assert!(request_json.contains(EPISODE_PROTOCOL_VERSION));
        assert!(request_json.contains("measure-combined-signal"));
        assert!(request_json.contains("latent-mechanism-a"));
        assert!(request_json.contains("latent-mechanism-b"));
        assert!(request_json.contains("Calibrate the sensor channel"));
        assert!(request_json.contains("combined categorical signal"));
        assert!(!request_json.contains("sensor_state"));
        assert!(!request_json.contains("991"));

        let decoded_request: EpisodeRequest =
            serde_json::from_slice(&encoded_request).expect("decode public request");
        assert_eq!(decoded_request, request);

        let response = EpisodeResponse {
            protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
            command: EpisodeCommand::Probe {
                action_id: "calibrate-sensor".to_owned(),
            },
        };
        let encoded_response = serde_json::to_vec(&response).expect("serialize policy response");
        let decoded_response: EpisodeResponse =
            serde_json::from_slice(&encoded_response).expect("decode policy response");
        assert_eq!(decoded_response, response);
    }

    #[test]
    fn versioned_probe_command_updates_only_the_public_observation_view() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        let response = EpisodeResponse {
            protocol_version: EPISODE_PROTOCOL_VERSION.to_owned(),
            command: EpisodeCommand::Probe {
                action_id: "measure-combined-signal".to_owned(),
            },
        };
        let view = provider.apply_response(response).expect("apply wire command");
        assert_eq!(view.accepted_attempts, 1);
        assert_eq!(view.observations.len(), 1);
        assert_eq!(view.observations[0].observation_id, "one");
        assert!(!view.terminal);
    }

    #[test]
    fn wire_protocol_version_mismatch_is_rejected_without_mutating_world() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        let before = provider.view();
        let response = EpisodeResponse {
            protocol_version: "wrong-protocol".to_owned(),
            command: EpisodeCommand::Probe {
                action_id: "measure-combined-signal".to_owned(),
            },
        };
        assert_eq!(
            provider.apply_response(response),
            Err(EpisodeError::InvalidProtocolVersion)
        );
        assert_eq!(provider.view(), before);
    }

    #[test]
    fn finish_rejects_unknown_or_duplicate_candidate_ids() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        assert_eq!(
            provider.finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Ambiguous,
                compatible_hypothesis_ids: vec!["not-in-the-public-catalogue".to_owned()],
                rationale: "Invalid candidate ID control.".to_owned(),
            }),
            Err(EpisodeError::InvalidFinish)
        );
        assert_eq!(
            provider.finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Ambiguous,
                compatible_hypothesis_ids: vec![
                    "latent-mechanism-a".to_owned(),
                    "latent-mechanism-a".to_owned(),
                ],
                rationale: "Duplicate candidate ID control.".to_owned(),
            }),
            Err(EpisodeError::InvalidFinish)
        );
        assert!(!provider.view().terminal);
    }

    #[test]
    fn exact_evaluator_rejects_a_false_unique_claim_before_discriminating_probe() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        provider
            .submit_action("measure-combined-signal")
            .expect("first ambiguous measurement");
        provider
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Identified,
                compatible_hypothesis_ids: vec!["latent-mechanism-a".to_owned()],
                rationale: "The first signal appears to identify one mechanism.".to_owned(),
            })
            .expect("finish with a status report");

        let evaluation = provider
            .evaluate_after_finish()
            .expect("complete exact evaluation");
        assert_eq!(evaluation.compatible_hypotheses.len(), 2);
        assert!(evaluation.generating_hypothesis_retained);
        assert!(!evaluation.evidence_identifies_unique_hypothesis);
        assert!(!evaluation.version_space_exact_match);
        assert_eq!(evaluation.correct_candidate_count, 1);
        assert!(evaluation.false_unique_claim);
        assert!(!evaluation.disposition_correct);
        assert_eq!(evaluation.applied_interventions, 1);
        assert_eq!(evaluation.total_requests, 1);
    }

    #[test]
    fn exact_evaluator_accepts_unique_status_only_after_discriminating_sequence() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        for action in [
            "measure-combined-signal",
            "calibrate-sensor",
            "measure-combined-signal",
        ] {
            provider.submit_action(action).expect("apply known intervention");
        }
        provider
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Identified,
                compatible_hypothesis_ids: vec!["latent-mechanism-a".to_owned()],
                rationale: "Calibration removed the persistent sensor confound; the later signal separates the remaining mechanisms.".to_owned(),
            })
            .expect("finish with unique status");

        let evaluation = provider
            .evaluate_after_finish()
            .expect("complete exact evaluation");
        assert_eq!(evaluation.compatible_hypotheses.len(), 1);
        assert!(evaluation.generating_hypothesis_retained);
        assert!(evaluation.evidence_identifies_unique_hypothesis);
        assert!(evaluation.version_space_exact_match);
        assert_eq!(evaluation.correct_candidate_count, 1);
        assert!(!evaluation.false_unique_claim);
        assert!(evaluation.disposition_correct);
        assert_eq!(evaluation.applied_interventions, 3);
    }

    #[test]
    fn exact_evaluator_distinguishes_safe_from_unnecessary_abstention() {
        let mut ambiguous = new_provider(persistent_sensor_confounding_kernel(), 3);
        ambiguous
            .submit_action("measure-combined-signal")
            .expect("ambiguous measurement");
        ambiguous
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Abstain,
                compatible_hypothesis_ids: vec![
                    "latent-mechanism-a".to_owned(),
                    "latent-mechanism-b".to_owned(),
                ],
                rationale: "The available observation is compatible with more than one mechanism.".to_owned(),
            })
            .expect("record safe abstention");
        let safe = ambiguous.evaluate_after_finish().unwrap();
        assert!(safe.safe_abstention);
        assert!(!safe.unnecessary_abstention);
        assert!(safe.disposition_correct);

        let mut unique = new_provider(persistent_sensor_confounding_kernel(), 3);
        for action in [
            "measure-combined-signal",
            "calibrate-sensor",
            "measure-combined-signal",
        ] {
            unique.submit_action(action).unwrap();
        }
        unique
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Abstain,
                compatible_hypothesis_ids: vec!["latent-mechanism-a".to_owned()],
                rationale: "I choose not to make a claim.".to_owned(),
            })
            .unwrap();
        let unnecessary = unique.evaluate_after_finish().unwrap();
        assert!(!unnecessary.safe_abstention);
        assert!(unnecessary.unnecessary_abstention);
        assert!(!unnecessary.disposition_correct);
    }

    #[test]
    fn post_finish_probe_is_rejected_without_mutating_the_evidence_trace() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        provider
            .finish_episode(EpisodeFinish {
                disposition: EpisodeFinishDisposition::Ambiguous,
                compatible_hypothesis_ids: vec![
                    "latent-mechanism-a".to_owned(),
                    "latent-mechanism-b".to_owned(),
                ],
                rationale: "The current evidence remains ambiguous.".to_owned(),
            })
            .expect("finish episode");
        let before = provider.view();
        assert_eq!(
            provider.submit_action("calibrate-sensor"),
            Err(EpisodeError::EpisodeAlreadyTerminal)
        );
        assert_eq!(provider.view(), before);
    }

    #[test]
    fn evaluator_refuses_to_promote_an_incomplete_attempt_trace() {
        let mut provider = new_provider(persistent_sensor_confounding_kernel(), 3);
        provider.trace_complete = false;
        provider.finish = Some(EpisodeFinish {
            disposition: EpisodeFinishDisposition::Abstain,
            compatible_hypothesis_ids: Vec::new(),
            rationale: "Incomplete trace control.".to_owned(),
        });
        provider.terminal = true;
        assert_eq!(
            provider.evaluate_after_finish(),
            Err(EpisodeError::IncompleteTrace)
        );
    }

    #[test]
    fn public_manifest_is_visible_but_contains_only_non_secret_digests() {
        let provider = new_provider(deterministic_kernel(false), 42);
        let view = provider.view();
        assert_eq!(view.manifest.contract_sha256, "a".repeat(64));
        assert_eq!(view.manifest.world_recipe_sha256, "d".repeat(64));
        assert!(!format!("{view:?}").contains("42"));
    }

    #[test]
    fn malformed_manifest_is_rejected_before_episode_creation() {
        let kernel = deterministic_kernel(false);
        let mut manifest = test_manifest(&kernel);
        manifest.world_model_sha256 = "not-a-digest".to_owned();
        assert!(matches!(
            EpisodeProvider::new(
                "opaque-episode-invalid-manifest",
                manifest,
                kernel,
                EpisodeConfig::default(),
                0,
            ),
            Err(EpisodeError::InvalidManifest)
        ));
    }

    #[test]
    fn receipt_cap_marks_trace_incomplete_and_counts_dropped_requests() {
        let mut provider = new_provider(deterministic_kernel(false), 77);
        for _ in 0..MAX_ATTEMPT_RECEIPTS {
            provider
                .submit_action("unknown-action")
                .expect("request retained under the receipt cap");
        }
        assert_eq!(
            provider.submit_action("unknown-action"),
            Err(EpisodeError::AttemptReceiptLimit)
        );
        let view = provider.view();
        assert_eq!(view.attempts.len(), MAX_ATTEMPT_RECEIPTS);
        assert_eq!(view.dropped_attempts, 1);
        assert!(!view.trace_complete);
        assert!(view.terminal);
    }

    #[test]
    fn sampler_rejects_positive_probability_below_53_bit_resolution() {
        let tiny = Rational::new(1, 1_u128 << 54).unwrap();
        let rest = Rational::new((1_u128 << 54) - 1, 1_u128 << 54).unwrap();
        let mut rng = SplitMix64::new(1);
        assert_eq!(
            sample_index(&[tiny, rest], |mass| *mass, &mut rng),
            Err(EpisodeError::SamplingResolutionTooCoarse)
        );
    }

    #[test]
    fn public_view_contains_catalogue_and_no_hidden_model_fields() {
        let provider = new_provider(deterministic_kernel(false), 7);
        let view = provider.view();
        assert_eq!(view.episode_id, "opaque-episode-0001");
        assert_eq!(view.manifest.world_model_sha256, "b".repeat(64));
        assert!(view.trace_complete);
        assert_eq!(view.dropped_attempts, 0);
        assert_eq!(view.actions[0].action_id, "measure");
        assert!(view.observations.is_empty());
        assert_eq!(view.current_step, 0);
        assert!(!view.terminal);
    }

    #[test]
    fn deterministic_kernel_emits_observation_and_charges_exact_action_cost() {
        let mut provider = new_provider(deterministic_kernel(false), 7);
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(receipt.requested_at_step, 0);
        assert_eq!(receipt.executed_at_step, Some(1));
        assert_eq!(receipt.charged_cost_units, 2);
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::Applied {
                observation_id: "signal".to_owned()
            }
        );

        let view = provider.view();
        assert_eq!(view.current_step, 1);
        assert_eq!(view.accepted_attempts, 1);
        assert_eq!(view.spent_cost_units, 2);
        assert_eq!(view.observations.len(), 1);
        assert_eq!(view.observations[0].event_step, 1);
        assert_eq!(view.observations[0].acquisition_step, 1);
        assert_eq!(view.observations[0].available_at_step, 1);
    }

    #[test]
    fn hazard_outcome_terminates_episode_without_fabricating_observation() {
        let mut provider = new_provider(deterministic_kernel(true), 7);
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(receipt.charged_cost_units, 2);
        assert_eq!(receipt.disposition, AttemptDisposition::HazardTerminal);

        let view = provider.view();
        assert!(view.terminal);
        assert!(view.observations.is_empty());

        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedTerminal);
        assert_eq!(rejected.charged_cost_units, 0);
        assert_eq!(provider.view().attempts.len(), 2);
    }

    #[test]
    fn unknown_action_is_retained_without_world_mutation_or_cost() {
        let mut provider = new_provider(deterministic_kernel(false), 7);
        let receipt = provider.submit_action("hidden-action").unwrap();
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::RejectedUnknownAction
        );
        assert_eq!(receipt.requested_cost_units, None);
        assert_eq!(receipt.charged_cost_units, 0);
        let view = provider.view();
        assert_eq!(view.attempts.len(), 1);
        assert_eq!(view.current_step, 0);
        assert_eq!(view.spent_cost_units, 0);
    }

    #[test]
    fn cost_budget_rejects_mutation_but_retains_request() {
        let kernel = deterministic_kernel(false);
        let manifest = test_manifest(&kernel);
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0002",
            manifest,
            kernel,
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 1,
                max_world_steps: 4,
            },
            3,
        )
        .unwrap();
        let receipt = provider.submit_action("measure").unwrap();
        assert_eq!(
            receipt.disposition,
            AttemptDisposition::RejectedCostBudget
        );
        assert_eq!(receipt.requested_cost_units, Some(2));
        assert_eq!(receipt.charged_cost_units, 0);
        let view = provider.view();
        assert_eq!(view.attempts.len(), 1);
        assert_eq!(view.current_step, 0);
        assert_eq!(view.spent_cost_units, 0);
    }

    #[test]
    fn accepted_attempt_budget_rejects_additional_interventions_and_records_them() {
        let kernel = deterministic_kernel(false);
        let manifest = test_manifest(&kernel);
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0003",
            manifest,
            kernel,
            EpisodeConfig {
                max_attempts: 1,
                max_cost_units: 10,
                max_world_steps: 4,
            },
            3,
        )
        .unwrap();

        assert!(matches!(
            provider.submit_action("measure").unwrap().disposition,
            AttemptDisposition::Applied { .. }
        ));
        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedAttemptBudget);
        assert_eq!(provider.view().attempts.len(), 2);
        assert_eq!(provider.view().current_step, 1);
    }

    #[test]
    fn physical_sensor_confound_requires_calibration_before_discriminating_measurement() {
        let kernel = persistent_sensor_confounding_kernel();
        let config = EpisodeConfig {
            max_attempts: 4,
            max_cost_units: 10,
            max_world_steps: 4,
        };
        // With this fixture and SplitMix64 mapping, seed 3 selects the first
        // initial tuple and seed 0 selects the second. The public histories
        // remain identical until the final post-calibration measurement.
        let first_manifest = test_manifest(&kernel);
        let mut first = EpisodeProvider::new(
            "opaque-confound-a-0001",
            first_manifest,
            kernel.clone(),
            config,
            3,
        )
        .expect("first episode");
        let second_manifest = test_manifest(&kernel);
        let mut second = EpisodeProvider::new(
            "opaque-confound-b-0001",
            second_manifest,
            kernel,
            config,
            0,
        )
        .expect("second episode");

        let first_measure = first.submit_action("measure-combined-signal").unwrap();
        let second_measure = second.submit_action("measure-combined-signal").unwrap();
        assert_eq!(
            first_measure.disposition,
            AttemptDisposition::Applied {
                observation_id: "one".to_owned()
            }
        );
        assert_eq!(first_measure.disposition, second_measure.disposition);
        assert_eq!(first.view().observations, second.view().observations);

        let first_calibration = first.submit_action("calibrate-sensor").unwrap();
        let second_calibration = second.submit_action("calibrate-sensor").unwrap();
        assert_eq!(
            first_calibration.disposition,
            AttemptDisposition::Applied {
                observation_id: "zero".to_owned()
            }
        );
        assert_eq!(first_calibration.disposition, second_calibration.disposition);
        assert_eq!(first.view().observations, second.view().observations);

        let first_discriminator = first.submit_action("measure-combined-signal").unwrap();
        let second_discriminator = second.submit_action("measure-combined-signal").unwrap();
        assert_eq!(
            first_discriminator.disposition,
            AttemptDisposition::Applied {
                observation_id: "zero".to_owned()
            }
        );
        assert_eq!(
            second_discriminator.disposition,
            AttemptDisposition::Applied {
                observation_id: "one".to_owned()
            }
        );
    }

    #[test]
    fn exact_joint_optimizer_policy_replays_through_episode_adapter_for_both_hidden_states() {
        let kernel = persistent_sensor_confounding_kernel();
        let optimized = crate::optimize_joint_kernel(
            &kernel,
            &crate::SolverConfig {
                horizon: 2,
                ..crate::SolverConfig::default()
            },
        )
        .expect("valid finite joint model");
        let selected = optimized
            .minimum_cost_under(Rational::zero(), Rational::zero(), Rational::zero())
            .expect("valid risk budgets")
            .expect("two-step policy exists")
            .clone();

        let first_action = match &selected.plan {
            crate::PolicyPlan::Action { action, .. } => *action,
            _ => panic!("oracle should select a first experiment"),
        };
        assert_eq!(kernel.action_ids[first_action], "calibrate-sensor");

        for (seed, expected_final_observation) in [(3_u64, "zero"), (0_u64, "one")] {
            let manifest = test_manifest(&kernel);
            let mut provider = EpisodeProvider::new(
                format!("oracle-replay-{seed:08}"),
                manifest,
                kernel.clone(),
                EpisodeConfig {
                    max_attempts: 2,
                    max_cost_units: 2,
                    max_world_steps: 2,
                },
                seed,
            )
            .expect("valid episode");

            let (first_action, first_branches) = match &selected.plan {
                crate::PolicyPlan::Action { action, branches } => (*action, branches),
                _ => panic!("oracle should select a first experiment"),
            };
            let first_receipt = provider
                .submit_action(&kernel.action_ids[first_action])
                .expect("first oracle action executes");
            let first_observation = match first_receipt.disposition {
                AttemptDisposition::Applied { observation_id } => observation_id,
                other => panic!("expected safe calibration, got {other:?}"),
            };
            let first_branch = first_branches
                .iter()
                .find(|branch| kernel.observation_ids[branch.observation] == first_observation)
                .expect("policy contains continuation for the returned public observation");

            let (second_action, _) = match first_branch.plan.as_ref() {
                crate::PolicyPlan::Action { action, branches } => (*action, branches),
                other => panic!("expected discriminating measurement after calibration, got {other:?}"),
            };
            assert_eq!(kernel.action_ids[second_action], "measure-combined-signal");
            let second_receipt = provider
                .submit_action(&kernel.action_ids[second_action])
                .expect("second oracle action executes");
            assert_eq!(
                second_receipt.disposition,
                AttemptDisposition::Applied {
                    observation_id: expected_final_observation.to_owned()
                }
            );
            assert_eq!(provider.view().accepted_attempts, 2);
            assert_eq!(provider.view().spent_cost_units, 2);
        }
    }

    #[test]
    fn same_seed_and_same_action_trace_produce_same_observation_trace() {
        let kernel_a = stochastic_kernel();
        let kernel_b = stochastic_kernel();
        let mut first = new_provider(kernel_a, 99);
        let mut second = new_provider(kernel_b, 99);

        let first_receipt = first.submit_action("measure").unwrap();
        let second_receipt = second.submit_action("measure").unwrap();
        assert_eq!(first_receipt, second_receipt);
        assert_eq!(first.view(), second.view());
    }

    #[test]
    fn rejected_unknown_request_does_not_advance_the_private_random_stream() {
        let mut with_rejection = new_provider(stochastic_kernel(), 1234);
        let mut without_rejection = new_provider(stochastic_kernel(), 1234);

        assert_eq!(
            with_rejection
                .submit_action("not-an-action")
                .unwrap()
                .disposition,
            AttemptDisposition::RejectedUnknownAction
        );

        let after_rejection = with_rejection.submit_action("measure").unwrap();
        let direct = without_rejection.submit_action("measure").unwrap();
        assert_eq!(after_rejection.disposition, direct.disposition);
        assert_eq!(with_rejection.view().observations, without_rejection.view().observations);
    }

    #[test]
    fn transition_row_accessor_rejects_out_of_range_hidden_tuple_indices() {
        let kernel = deterministic_kernel(false);
        assert!(kernel.transition_row(1, 0, 0, 0).is_none());
        assert!(kernel.transition_row(0, 1, 0, 0).is_none());
        assert!(kernel.transition_row(0, 0, 1, 0).is_none());
        assert!(kernel.transition_row(0, 0, 0, 1).is_none());
    }

    #[test]
    fn a_different_seed_is_not_exposed_in_the_policy_view() {
        let first = new_provider(stochastic_kernel(), 11);
        let second = new_provider(stochastic_kernel(), 12);
        // The initial public contract is identical; the private sampled state
        // and PRNG differ, but neither is exposed before an observation.
        assert_eq!(first.view(), second.view());
    }

    #[test]
    fn configured_world_horizon_is_enforced_and_request_is_retained() {
        let kernel = deterministic_kernel(false);
        let manifest = test_manifest(&kernel);
        let mut provider = EpisodeProvider::new(
            "opaque-episode-0004",
            manifest,
            kernel,
            EpisodeConfig {
                max_attempts: 4,
                max_cost_units: 10,
                max_world_steps: 1,
            },
            7,
        )
        .unwrap();

        assert!(matches!(
            provider.submit_action("measure").unwrap().disposition,
            AttemptDisposition::Applied { .. }
        ));
        let rejected = provider.submit_action("measure").unwrap();
        assert_eq!(rejected.disposition, AttemptDisposition::RejectedStepBudget);
        assert_eq!(provider.view().attempts.len(), 2);
        assert_eq!(provider.view().current_step, 1);
    }

    #[test]
    fn invalid_kernel_or_episode_id_is_rejected_before_episode_creation() {
        let kernel = deterministic_kernel(false);
        let manifest = test_manifest(&kernel);
        let invalid_id = EpisodeProvider::new(
            "short",
            manifest,
            kernel.clone(),
            EpisodeConfig::default(),
            0,
        );
        assert!(matches!(
            invalid_id,
            Err(EpisodeError::InvalidEpisodeId)
        ));

        let mut invalid = deterministic_kernel(false);
        invalid.rows.clear();
        assert!(matches!(
            EpisodeProvider::new(
                "opaque-episode-0005",
                {
                    let mut manifest = test_manifest(&invalid);
                    manifest
                },
                invalid,
                EpisodeConfig::default(),
                0,
            ),
            Err(EpisodeError::InvalidKernel(_))
        ));
    }
}
