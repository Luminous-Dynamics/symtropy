#![forbid(unsafe_code)]

//! Bounded reciprocal-episode export into the SAM #10514 trajectory-query shape.
//!
//! This crate is an adapter and conformance oracle. It does not implement a
//! simulator, infer human behaviour, estimate empirical support, or make a
//! safety decision. The SAM support implementation remains the sole owner of
//! finite trajectory membership semantics.

use std::collections::{BTreeMap, BTreeSet};

const EPISODE_SCHEMA: &str = "symtropy.road-egci-reciprocal-episode.v0.1";
const QUERY_SCHEMA: &str = "sam.policy-trajectory-query.v0.1";
const HASH_DOMAIN: &str = "Symtropy EGCI reciprocal episode export v0.1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExperimentManifest {
    pub experiment_id: String,
    pub scenario_digest: String,
    pub odd_scope_digest: String,
    pub initial_world_snapshot_digest: String,
    pub physics_profile_digest: String,
    pub execution_profile_digest: String,
    pub observation_schedule_digest: String,
    pub action_domain_digest: String,
    pub units_profile_digest: String,
    pub tick_hz_numerator: u64,
    pub tick_hz_denominator: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyProfile {
    pub actor_id: String,
    pub policy_digest: String,
    pub config_digest: String,
    pub initial_policy_state_digest: String,
    pub observation_profile_digest: String,
    pub action_domain_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationDisposition {
    Delivered,
    Occluded,
    Dropped,
    Stale,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObservationDelivery {
    pub cue_id: String,
    pub source_event_id: String,
    pub recipient_actor_id: String,
    pub observation_payload_digest: String,
    pub observed_tick: u64,
    pub generated_tick: u64,
    pub delivered_tick: Option<u64>,
    /// Monotone within each recipient's delivered observation channel.
    pub delivery_sequence: u64,
    pub visibility_profile_digest: String,
    pub disposition: ObservationDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecisionDisposition {
    Completed,
    TimedOut,
    Failed,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyDecision {
    pub decision_receipt_id: String,
    pub actor_id: String,
    pub tick: u64,
    pub policy_digest: String,
    pub config_digest: String,
    pub prior_policy_state_digest: String,
    pub next_policy_state_digest: Option<String>,
    /// All observation cues admitted into this decision's visible history,
    /// retained in the order actually delivered to the policy.
    pub visible_cue_ids: Vec<String>,
    /// The subset of visible cues actually consumed by this decision.
    pub consumed_cue_ids: Vec<String>,
    /// Earlier decisions by this same actor that form part of its permitted history.
    pub prior_action_receipt_ids: Vec<String>,
    /// Re-derived by this exporter from policy state, cues and prior actions.
    pub permitted_history_digest: String,
    pub deadline_tick: u64,
    pub command_latch_tick: Option<u64>,
    pub action_id: Option<String>,
    pub action_payload_digest: Option<String>,
    pub disposition: DecisionDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionDisposition {
    Completed,
    Failed,
    Censored,
    Missing,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JointTransition {
    pub tick: u64,
    pub parent_snapshot_digest: String,
    pub child_snapshot_digest: String,
    pub joint_action_digest: String,
    pub physics_step_receipt_digest: String,
    pub disposition: TransitionDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchOutcome {
    Completed,
    TaskFailed,
    TimedOut,
    ExecutionFailed,
    Censored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayDisposition {
    ExactSameRuntimeBytes,
    PortableSemanticEquivalent,
    NotRun,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayEvidence {
    pub replay_profile_digest: String,
    pub expected_branch_payload_root: String,
    pub replayed_branch_payload_root: Option<String>,
    pub disposition: ReplayDisposition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CandidateUniverseDisposition {
    ExhaustiveWithinDeclaredScope,
    Incomplete,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HypothesisSetDisposition {
    ExhaustiveWithinDeclaredModelFamily,
    IncompleteOrBudgetLimited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorrespondenceReceipt {
    pub hypothesis_id: String,
    pub correspondence_digest: String,
    pub causal_dispositions_digest: String,
    pub projection_input_digest: String,
    pub draw_bundle_digest: String,
    pub verifier_profile_digest: String,
    pub verifier_receipt_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchExport {
    pub branch_id: String,
    pub parent_snapshot_digest: String,
    pub intervention_digest: String,
    pub outcome: BranchOutcome,
    pub candidate_universe: CandidateUniverseDisposition,
    pub hypothesis_set: HypothesisSetDisposition,
    pub correspondences: Vec<CorrespondenceReceipt>,
    pub observations: Vec<ObservationDelivery>,
    pub decisions: Vec<PolicyDecision>,
    pub transitions: Vec<JointTransition>,
    pub replay: ReplayEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EpisodeExport {
    pub schema: String,
    pub manifest: ExperimentManifest,
    pub policies: Vec<PolicyProfile>,
    pub branches: Vec<BranchExport>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawBundleAnchor {
    pub branch_id: String,
    pub hypothesis_id: String,
    pub expected_bundle_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalAnchor {
    pub authority_reference: String,
    pub expected_episode_root: String,
    pub draw_bundle_anchors: Vec<DrawBundleAnchor>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandoffError {
    WrongEpisodeSchema,
    EmptyIdentity(&'static str),
    InvalidTickRate,
    DuplicateActor,
    DuplicateBranch,
    DuplicateCue,
    DuplicateDecisionReceipt,
    DuplicateDecisionTickActor,
    DuplicateTransitionTick,
    DuplicateHypothesis,
    MissingPolicy,
    PolicyMismatch,
    MissingCue,
    WrongCueRecipient,
    CueNotDelivered,
    FutureCueLeak,
    InvalidObservationTiming,
    ConsumedCueNotVisible,
    MissingPriorAction,
    PriorActionWrongActor,
    PriorActionNotEarlier,
    HistoryDigestMismatch,
    InvalidDecisionReceipt,
    InvalidTransitionReceipt,
    TransitionChainMismatch,
    JointActionDigestMismatch,
    MissingBranchParent,
    MissingCorrespondenceEvidence,
    MissingDrawBundleAnchor,
    DrawBundleAnchorMismatch,
    EpisodeAnchorMismatch,
    BranchReplayAnchorMismatch,
    ReplayReceiptMismatch,
    NonCanonicalSamQuery,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamActorAction {
    pub actor_id: String,
    pub action_id: String,
    pub payload_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamDecisionStep {
    pub tick: u64,
    pub history_digest: String,
    pub joint_actions: Vec<SamActorAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamTrajectoryQuery {
    pub target_policy_digest: String,
    pub context_digest: String,
    pub complete: bool,
    pub steps: Vec<SamDecisionStep>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HypothesisQuery {
    pub hypothesis_id: String,
    pub draw_bundle_digest: String,
    /// This is the exact four-field SAM #10514 query DTO.
    pub query: SamTrajectoryQuery,
    /// Canonical JSON ready for a SAM adapter consumer.
    pub query_json: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchHandoff {
    pub branch_id: String,
    pub branch_payload_root: String,
    pub queries: Vec<HypothesisQuery>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    SimulationGenerated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceEligibility {
    FormalConformanceOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SamHandoff {
    pub episode_root: String,
    pub anchor_authority_reference: String,
    pub evidence_class: EvidenceClass,
    pub eligibility: EvidenceEligibility,
    pub branches: Vec<BranchHandoff>,
}

#[derive(Clone, Debug)]
struct VerifiedDecision {
    receipt_id: String,
    actor_id: String,
    tick: u64,
    history_digest: String,
    action_id: String,
    payload_digest: String,
    latch_tick: u64,
    disposition: DecisionDisposition,
}

impl EpisodeExport {
    /// Hash the complete episode inputs. This is an unkeyed content commitment,
    /// not a signature and not proof that source evidence is true.
    pub fn episode_root(&self) -> Result<String, HandoffError> {
        if self.schema != EPISODE_SCHEMA {
            return Err(HandoffError::WrongEpisodeSchema);
        }
        let mut h = blake3::Hasher::new_derive_key(HASH_DOMAIN);
        hash_manifest(&mut h, &self.manifest);

        let mut policies: Vec<_> = self.policies.iter().collect();
        policies.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
        hash_len(&mut h, policies.len());
        for policy in policies {
            hash_policy(&mut h, policy);
        }

        let mut branches: Vec<_> = self.branches.iter().collect();
        branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
        hash_len(&mut h, branches.len());
        for branch in branches {
            hash_str(&mut h, &branch.branch_id);
            hash_str(&mut h, &branch_payload_root(branch));
            hash_replay(&mut h, &branch.replay);
        }
        Ok(hex(h.finalize().as_bytes()))
    }
}

/// Validate an exported episode against a separately supplied digest anchor,
/// then produce one SAM query per branch and per retained correspondence map.
///
/// This adapter never runs SAM's support assessment. It only produces the
/// exact query DTO consumed by SAM #10514.
pub fn export_to_sam(
    episode: &EpisodeExport,
    anchor: &ExternalAnchor,
) -> Result<SamHandoff, HandoffError> {
    require(&anchor.authority_reference, "anchor.authority_reference")?;
    let episode_root = episode.episode_root()?;
    if episode_root != anchor.expected_episode_root {
        return Err(HandoffError::EpisodeAnchorMismatch);
    }

    validate_manifest(&episode.manifest)?;
    let policies = policy_index(&episode.policies)?;
    let target_policy_digest = target_policy_digest(&episode.policies)?;
    let context_digest = context_digest(&episode.manifest, &episode.policies)?;
    let anchors = draw_anchor_index(&anchor.draw_bundle_anchors)?;

    let mut branches: Vec<_> = episode.branches.iter().collect();
    branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    let mut output = Vec::with_capacity(branches.len());

    for branch in branches {
        validate_branch_identity(branch)?;
        if branch.parent_snapshot_digest != episode.manifest.initial_world_snapshot_digest {
            return Err(HandoffError::MissingBranchParent);
        }

        let payload_root = branch_payload_root(branch);
        if branch.replay.expected_branch_payload_root != payload_root {
            return Err(HandoffError::BranchReplayAnchorMismatch);
        }
        match branch.replay.disposition {
            ReplayDisposition::ExactSameRuntimeBytes
            | ReplayDisposition::PortableSemanticEquivalent => {
                if branch.replay.replayed_branch_payload_root.as_deref()
                    != Some(payload_root.as_str())
                {
                    return Err(HandoffError::ReplayReceiptMismatch);
                }
            }
            ReplayDisposition::NotRun | ReplayDisposition::Failed => {}
        }

        let receipts = correspondence_index(&branch.correspondences)?;
        let cues = observation_index(&branch.observations)?;
        let decisions = verify_decisions(branch, &policies, &cues)?;
        let steps = build_joint_steps(branch, &policies, &decisions)?;

        let source_complete = branch.candidate_universe
            == CandidateUniverseDisposition::ExhaustiveWithinDeclaredScope
            && branch.hypothesis_set
                == HypothesisSetDisposition::ExhaustiveWithinDeclaredModelFamily;
        let replay_complete = matches!(
            branch.replay.disposition,
            ReplayDisposition::ExactSameRuntimeBytes
                | ReplayDisposition::PortableSemanticEquivalent
        );
        let execution_complete = matches!(
            branch.outcome,
            BranchOutcome::Completed | BranchOutcome::TaskFailed
        ) && transitions_complete(branch, &steps);
        let branch_complete = source_complete && replay_complete && execution_complete;

        let mut queries = Vec::with_capacity(receipts.len());
        for (hypothesis_id, receipt) in receipts {
            let expected = anchors
                .get(&(branch.branch_id.clone(), hypothesis_id.clone()))
                .ok_or(HandoffError::MissingDrawBundleAnchor)?;
            if expected != &receipt.draw_bundle_digest {
                return Err(HandoffError::DrawBundleAnchorMismatch);
            }

            let query = SamTrajectoryQuery {
                target_policy_digest: target_policy_digest.clone(),
                context_digest: context_digest.clone(),
                complete: branch_complete,
                steps: steps.clone(),
            };
            let query_json = sam_query_json(&query);
            queries.push(HypothesisQuery {
                hypothesis_id,
                draw_bundle_digest: receipt.draw_bundle_digest.clone(),
                query,
                query_json,
            });
        }
        output.push(BranchHandoff {
            branch_id: branch.branch_id.clone(),
            branch_payload_root: payload_root,
            queries,
        });
    }

    if anchors.len() != output.iter().map(|b| b.queries.len()).sum::<usize>() {
        return Err(HandoffError::MissingDrawBundleAnchor);
    }

    Ok(SamHandoff {
        episode_root,
        anchor_authority_reference: anchor.authority_reference.clone(),
        evidence_class: EvidenceClass::SimulationGenerated,
        eligibility: EvidenceEligibility::FormalConformanceOnly,
        branches: output,
    })
}

fn validate_manifest(manifest: &ExperimentManifest) -> Result<(), HandoffError> {
    require(&manifest.experiment_id, "experiment_id")?;
    require(&manifest.scenario_digest, "scenario_digest")?;
    require(&manifest.odd_scope_digest, "odd_scope_digest")?;
    require(&manifest.initial_world_snapshot_digest, "initial_world_snapshot_digest")?;
    require(&manifest.physics_profile_digest, "physics_profile_digest")?;
    require(&manifest.execution_profile_digest, "execution_profile_digest")?;
    require(&manifest.observation_schedule_digest, "observation_schedule_digest")?;
    require(&manifest.action_domain_digest, "action_domain_digest")?;
    require(&manifest.units_profile_digest, "units_profile_digest")?;
    if manifest.tick_hz_numerator == 0 || manifest.tick_hz_denominator == 0 {
        return Err(HandoffError::InvalidTickRate);
    }
    Ok(())
}

fn validate_branch_identity(branch: &BranchExport) -> Result<(), HandoffError> {
    require(&branch.branch_id, "branch_id")?;
    require(&branch.parent_snapshot_digest, "parent_snapshot_digest")?;
    require(&branch.intervention_digest, "intervention_digest")?;
    require(&branch.replay.replay_profile_digest, "replay_profile_digest")?;
    require(&branch.replay.expected_branch_payload_root, "expected_branch_payload_root")?;
    Ok(())
}

fn policy_index<'a>(
    policies: &'a [PolicyProfile],
) -> Result<BTreeMap<&'a str, &'a PolicyProfile>, HandoffError> {
    let mut index = BTreeMap::new();
    for policy in policies {
        require(&policy.actor_id, "actor_id")?;
        require(&policy.policy_digest, "policy_digest")?;
        require(&policy.config_digest, "config_digest")?;
        require(&policy.initial_policy_state_digest, "initial_policy_state_digest")?;
        require(&policy.observation_profile_digest, "observation_profile_digest")?;
        require(&policy.action_domain_digest, "action_domain_digest")?;
        if index.insert(policy.actor_id.as_str(), policy).is_some() {
            return Err(HandoffError::DuplicateActor);
        }
    }
    if index.len() < 2 {
        return Err(HandoffError::MissingPolicy);
    }
    Ok(index)
}

fn observation_index<'a>(
    observations: &'a [ObservationDelivery],
) -> Result<BTreeMap<&'a str, &'a ObservationDelivery>, HandoffError> {
    let mut index = BTreeMap::new();
    for cue in observations {
        require(&cue.cue_id, "cue_id")?;
        require(&cue.source_event_id, "source_event_id")?;
        require(&cue.recipient_actor_id, "recipient_actor_id")?;
        require(&cue.observation_payload_digest, "observation_payload_digest")?;
        require(&cue.visibility_profile_digest, "visibility_profile_digest")?;
        if cue.observed_tick > cue.generated_tick {
            return Err(HandoffError::InvalidObservationTiming);
        }
        match (cue.disposition, cue.delivered_tick) {
            (ObservationDisposition::Delivered, Some(delivered)) => {
                if delivered < cue.generated_tick {
                    return Err(HandoffError::InvalidObservationTiming);
                }
            }
            (ObservationDisposition::Delivered, None)
            | (ObservationDisposition::Occluded, Some(_))
            | (ObservationDisposition::Dropped, Some(_))
            | (ObservationDisposition::Stale, Some(_)) => {
                return Err(HandoffError::InvalidObservationTiming);
            }
            _ => {}
        }
        if index.insert(cue.cue_id.as_str(), cue).is_some() {
            return Err(HandoffError::DuplicateCue);
        }
    }
    Ok(index)
}

fn verify_decisions<'a>(
    branch: &'a BranchExport,
    policies: &BTreeMap<&'a str, &'a PolicyProfile>,
    cues: &BTreeMap<&'a str, &'a ObservationDelivery>,
) -> Result<Vec<VerifiedDecision>, HandoffError> {
    let mut receipts = BTreeMap::new();
    let mut tick_actor = BTreeSet::new();
    for decision in &branch.decisions {
        require(&decision.decision_receipt_id, "decision_receipt_id")?;
        let policy = policies
            .get(decision.actor_id.as_str())
            .ok_or(HandoffError::MissingPolicy)?;
        if policy.policy_digest != decision.policy_digest
            || policy.config_digest != decision.config_digest
        {
            return Err(HandoffError::PolicyMismatch);
        }
        if !tick_actor.insert((decision.tick, decision.actor_id.as_str())) {
            return Err(HandoffError::DuplicateDecisionTickActor);
        }
        if receipts
            .insert(decision.decision_receipt_id.as_str(), decision)
            .is_some()
        {
            return Err(HandoffError::DuplicateDecisionReceipt);
        }
    }

    let mut ordered: Vec<_> = branch.decisions.iter().collect();
    ordered.sort_by(|a, b| (a.tick, &a.actor_id).cmp(&(b.tick, &b.actor_id)));
    let mut verified = Vec::with_capacity(ordered.len());

    for decision in ordered {
        let policy = policies[decision.actor_id.as_str()];
        let mut visible = BTreeSet::new();
        let mut expected_visible: Vec<_> = cues
            .values()
            .filter(|cue| {
                cue.recipient_actor_id == decision.actor_id
                    && cue.disposition == ObservationDisposition::Delivered
                    && cue.delivered_tick.is_some_and(|tick| tick <= decision.tick)
            })
            .collect();
        expected_visible.sort_by_key(|cue| (cue.delivered_tick.unwrap(), cue.delivery_sequence));
        if expected_visible
            .iter()
            .map(|cue| cue.cue_id.as_str())
            .collect::<Vec<_>>()
            != decision
                .visible_cue_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        {
            return Err(HandoffError::CueNotDelivered);
        }

        let mut history =
            blake3::Hasher::new_derive_key("Symtropy EGCI permitted policy history v0.1");
        hash_str(&mut history, &decision.actor_id);
        hash_u64(&mut history, decision.tick);
        hash_str(&mut history, &decision.prior_policy_state_digest);
        hash_str(&mut history, &policy.observation_profile_digest);
        hash_len(&mut history, decision.visible_cue_ids.len());

        for cue_id in &decision.visible_cue_ids {
            if !visible.insert(cue_id.as_str()) {
                return Err(HandoffError::DuplicateCue);
            }
            let cue = cues.get(cue_id.as_str()).ok_or(HandoffError::MissingCue)?;
            if cue.recipient_actor_id != decision.actor_id {
                return Err(HandoffError::WrongCueRecipient);
            }
            let delivered_tick = cue.delivered_tick.ok_or(HandoffError::CueNotDelivered)?;
            if cue.disposition != ObservationDisposition::Delivered
                || delivered_tick > decision.tick
            {
                return Err(HandoffError::FutureCueLeak);
            }
            hash_str(&mut history, &cue.cue_id);
            hash_str(&mut history, &cue.source_event_id);
            hash_str(&mut history, &cue.observation_payload_digest);
            hash_u64(&mut history, cue.observed_tick);
            hash_u64(&mut history, cue.generated_tick);
            hash_u64(&mut history, delivered_tick);
            hash_u64(&mut history, cue.delivery_sequence);
            hash_str(&mut history, &cue.visibility_profile_digest);
        }

        let mut consumed = BTreeSet::new();
        for cue_id in &decision.consumed_cue_ids {
            if !consumed.insert(cue_id.as_str()) || !visible.contains(cue_id.as_str()) {
                return Err(HandoffError::ConsumedCueNotVisible);
            }
        }

        let mut expected_prior: Vec<_> = branch
            .decisions
            .iter()
            .filter(|prior| {
                prior.actor_id == decision.actor_id
                    && prior.tick < decision.tick
                    && prior.disposition == DecisionDisposition::Completed
            })
            .collect();
        expected_prior.sort_by_key(|prior| prior.tick);
        if expected_prior
            .iter()
            .map(|prior| prior.decision_receipt_id.as_str())
            .collect::<Vec<_>>()
            != decision
                .prior_action_receipt_ids
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        {
            return Err(HandoffError::MissingPriorAction);
        }

        hash_len(&mut history, decision.prior_action_receipt_ids.len());
        for prior_id in &decision.prior_action_receipt_ids {
            let prior = receipts
                .get(prior_id.as_str())
                .ok_or(HandoffError::MissingPriorAction)?;
            if prior.actor_id != decision.actor_id {
                return Err(HandoffError::PriorActionWrongActor);
            }
            if prior.tick >= decision.tick
                || prior.disposition != DecisionDisposition::Completed
            {
                return Err(HandoffError::PriorActionNotEarlier);
            }
            hash_str(&mut history, prior_id);
        }

        let expected_prior_state = expected_prior
            .last()
            .and_then(|prior| prior.next_policy_state_digest.as_deref())
            .unwrap_or(policy.initial_policy_state_digest.as_str());
        if decision.prior_policy_state_digest != expected_prior_state {
            return Err(HandoffError::PolicyMismatch);
        }

        let history_digest = hex(history.finalize().as_bytes());
        if history_digest != decision.permitted_history_digest {
            return Err(HandoffError::HistoryDigestMismatch);
        }

        let complete_action = decision.action_id.is_some()
            && decision.action_payload_digest.is_some()
            && decision.command_latch_tick.is_some()
            && decision.next_policy_state_digest.is_some();
        match decision.disposition {
            DecisionDisposition::Completed => {
                if !complete_action
                    || decision.command_latch_tick.is_none_or(|tick| {
                        tick < decision.tick || tick > decision.deadline_tick
                    })
                {
                    return Err(HandoffError::InvalidDecisionReceipt);
                }
            }
            DecisionDisposition::TimedOut
            | DecisionDisposition::Failed
            | DecisionDisposition::Missing => {}
        }

        if let Some(action_id) = &decision.action_id {
            require(action_id, "action_id")?;
        }
        if let Some(payload) = &decision.action_payload_digest {
            require(payload, "action_payload_digest")?;
        }
        if let Some(next_state) = &decision.next_policy_state_digest {
            require(next_state, "next_policy_state_digest")?;
        }

        verified.push(VerifiedDecision {
            actor_id: decision.actor_id.clone(),
            tick: decision.tick,
            history_digest,
            action_id: decision.action_id.clone().unwrap_or_default(),
            payload_digest: decision.action_payload_digest.clone().unwrap_or_default(),
            latch_tick: decision.command_latch_tick.unwrap_or(u64::MAX),
            disposition: decision.disposition,
        });
    }
    Ok(verified)
}

fn build_joint_steps(
    branch: &BranchExport,
    policies: &BTreeMap<&str, &PolicyProfile>,
    decisions: &[VerifiedDecision],
) -> Result<Vec<SamDecisionStep>, HandoffError> {
    let mut steps = Vec::new();
    for transition in &branch.transitions {
        let mut at_tick: Vec<_> = decisions
            .iter()
            .filter(|decision| decision.tick == transition.tick)
            .collect();
        at_tick.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));

        if at_tick.len() != policies.len()
            || at_tick.iter().any(|d| d.disposition != DecisionDisposition::Completed)
        {
            continue;
        }
        if at_tick.iter().any(|d| d.latch_tick != transition.tick) {
            return Err(HandoffError::InvalidDecisionReceipt);
        }

        let mut actions = Vec::with_capacity(at_tick.len());
        let mut action_hash = blake3::Hasher::new_derive_key(
            "Symtropy EGCI joint action receipt v0.1",
        );
        for decision in at_tick {
            if !policies.contains_key(decision.actor_id.as_str()) {
                return Err(HandoffError::MissingPolicy);
            }
            actions.push(SamActorAction {
                actor_id: decision.actor_id.clone(),
                action_id: decision.action_id.clone(),
                payload_digest: decision.payload_digest.clone(),
            });
            hash_str(&mut action_hash, &decision.actor_id);
            hash_str(&mut action_hash, &decision.action_id);
            hash_str(&mut action_hash, &decision.payload_digest);
        }
        if hex(action_hash.finalize().as_bytes()) != transition.joint_action_digest {
            return Err(HandoffError::JointActionDigestMismatch);
        }
        if transition.disposition != TransitionDisposition::Completed {
            continue;
        }
        let joint_history = joint_history_digest(transition.tick, &actions, decisions);
        steps.push(SamDecisionStep {
            tick: transition.tick,
            history_digest: joint_history,
            joint_actions: actions,
        });
    }
    Ok(steps)
}

fn joint_history_digest(
    tick: u64,
    actions: &[SamActorAction],
    decisions: &[VerifiedDecision],
) -> String {
    let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI joint history v0.1");
    hash_u64(&mut h, tick);
    let mut selected: Vec<_> = decisions.iter().filter(|d| d.tick == tick).collect();
    selected.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    hash_len(&mut h, selected.len());
    for decision in selected {
        hash_str(&mut h, &decision.actor_id);
        hash_str(&mut h, &decision.history_digest);
    }
    hash_len(&mut h, actions.len());
    for action in actions {
        hash_str(&mut h, &action.actor_id);
        hash_str(&mut h, &action.action_id);
        hash_str(&mut h, &action.payload_digest);
    }
    hex(h.finalize().as_bytes())
}

fn transitions_complete(branch: &BranchExport, steps: &[SamDecisionStep]) -> bool {
    if branch.transitions.is_empty() || steps.len() != branch.transitions.len() {
        return false;
    }
    let mut prior_child: Option<&str> = None;
    let mut prior_tick = None;
    for (index, transition) in branch.transitions.iter().enumerate() {
        if transition.disposition != TransitionDisposition::Completed
            || steps.get(index).is_none_or(|step| step.tick != transition.tick)
        {
            return false;
        }
        if let Some(child) = prior_child {
            if transition.parent_snapshot_digest != child {
                return false;
            }
        } else if transition.parent_snapshot_digest != branch.parent_snapshot_digest {
            return false;
        }
        if prior_tick.is_some_and(|tick: u64| transition.tick != tick + 1) {
            return false;
        }
        prior_tick = Some(transition.tick);
        prior_child = Some(&transition.child_snapshot_digest);
    }
    true
}

fn correspondence_index<'a>(
    receipts: &'a [CorrespondenceReceipt],
) -> Result<BTreeMap<String, &'a CorrespondenceReceipt>, HandoffError> {
    let mut index = BTreeMap::new();
    for receipt in receipts {
        require(&receipt.hypothesis_id, "hypothesis_id")?;
        require(&receipt.correspondence_digest, "correspondence_digest")?;
        require(&receipt.causal_dispositions_digest, "causal_dispositions_digest")?;
        require(&receipt.projection_input_digest, "projection_input_digest")?;
        require(&receipt.draw_bundle_digest, "draw_bundle_digest")?;
        require(&receipt.verifier_profile_digest, "verifier_profile_digest")?;
        require(&receipt.verifier_receipt_digest, "verifier_receipt_digest")?;
        if index.insert(receipt.hypothesis_id.clone(), receipt).is_some() {
            return Err(HandoffError::DuplicateHypothesis);
        }
    }
    if index.is_empty() {
        return Err(HandoffError::MissingCorrespondenceEvidence);
    }
    Ok(index)
}

fn draw_anchor_index(
    anchors: &[DrawBundleAnchor],
) -> Result<BTreeMap<(String, String), String>, HandoffError> {
    let mut index = BTreeMap::new();
    for anchor in anchors {
        require(&anchor.branch_id, "draw anchor branch_id")?;
        require(&anchor.hypothesis_id, "draw anchor hypothesis_id")?;
        require(&anchor.expected_bundle_digest, "expected_bundle_digest")?;
        let key = (anchor.branch_id.clone(), anchor.hypothesis_id.clone());
        if index.insert(key, anchor.expected_bundle_digest.clone()).is_some() {
            return Err(HandoffError::DuplicateHypothesis);
        }
    }
    Ok(index)
}

fn target_policy_digest(policies: &[PolicyProfile]) -> Result<String, HandoffError> {
    let mut ordered = policies.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    let mut h = blake3::Hasher::new_derive_key("SAM target policy portfolio v0.1");
    hash_len(&mut h, ordered.len());
    for policy in ordered {
        hash_policy(&mut h, policy);
    }
    Ok(hex(h.finalize().as_bytes()))
}

fn context_digest(
    manifest: &ExperimentManifest,
    policies: &[PolicyProfile],
) -> Result<String, HandoffError> {
    let mut h = blake3::Hasher::new_derive_key("SAM reciprocal episode context v0.1");
    hash_manifest(&mut h, manifest);
    let mut ordered = policies.iter().collect::<Vec<_>>();
    ordered.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
    hash_len(&mut h, ordered.len());
    for policy in ordered {
        hash_str(&mut h, &policy.actor_id);
        hash_str(&mut h, &policy.observation_profile_digest);
        hash_str(&mut h, &policy.action_domain_digest);
    }
    Ok(hex(h.finalize().as_bytes()))
}

fn sam_query_json(query: &SamTrajectoryQuery) -> String {
    let mut output = String::new();
    output.push_str("{\"target_policy_digest\":");
    json_string(&mut output, &query.target_policy_digest);
    output.push_str(",\"context_digest\":");
    json_string(&mut output, &query.context_digest);
    output.push_str(",\"complete\":");
    output.push_str(if query.complete { "true" } else { "false" });
    output.push_str(",\"steps\":[");
    for (index, step) in query.steps.iter().enumerate() {
        if index > 0 { output.push(','); }
        output.push_str("{\"tick\":");
        output.push_str(&step.tick.to_string());
        output.push_str(",\"history_digest\":");
        json_string(&mut output, &step.history_digest);
        output.push_str(",\"joint_actions\":[");
        for (action_index, action) in step.joint_actions.iter().enumerate() {
            if action_index > 0 { output.push(','); }
            output.push_str("{\"actor_id\":");
            json_string(&mut output, &action.actor_id);
            output.push_str(",\"action_id\":");
            json_string(&mut output, &action.action_id);
            output.push_str(",\"payload_digest\":");
            json_string(&mut output, &action.payload_digest);
            output.push('}');
        }
        output.push_str("]}");
    }
    output.push_str("]}");
    output
}

fn json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            c if c <= '\u{1f}' => {
                use std::fmt::Write;
                let _ = write!(output, "\\u{:04x}", c as u32);
            }
            c => output.push(c),
        }
    }
    output.push('"');
}

fn branch_payload_root(branch: &BranchExport) -> String {
    let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI branch payload v0.1");
    hash_str(&mut h, &branch.branch_id);
    hash_str(&mut h, &branch.parent_snapshot_digest);
    hash_str(&mut h, &branch.intervention_digest);
    h.update(&[branch_outcome_tag(branch.outcome)]);
    h.update(&[match branch.candidate_universe {
        CandidateUniverseDisposition::ExhaustiveWithinDeclaredScope => 0,
        CandidateUniverseDisposition::Incomplete => 1,
    }]);
    h.update(&[match branch.hypothesis_set {
        HypothesisSetDisposition::ExhaustiveWithinDeclaredModelFamily => 0,
        HypothesisSetDisposition::IncompleteOrBudgetLimited => 1,
    }]);

    let mut correspondences = branch.correspondences.iter().collect::<Vec<_>>();
    correspondences.sort_by(|a, b| a.hypothesis_id.cmp(&b.hypothesis_id));
    hash_len(&mut h, correspondences.len());
    for receipt in correspondences {
        hash_str(&mut h, &receipt.hypothesis_id);
        hash_str(&mut h, &receipt.correspondence_digest);
        hash_str(&mut h, &receipt.causal_dispositions_digest);
        hash_str(&mut h, &receipt.projection_input_digest);
        hash_str(&mut h, &receipt.draw_bundle_digest);
        hash_str(&mut h, &receipt.verifier_profile_digest);
        hash_str(&mut h, &receipt.verifier_receipt_digest);
    }

    let mut observations = branch.observations.iter().collect::<Vec<_>>();
    observations.sort_by(|a, b| (&a.recipient_actor_id, &a.cue_id).cmp(&(&b.recipient_actor_id, &b.cue_id)));
    hash_len(&mut h, observations.len());
    for cue in observations {
        hash_str(&mut h, &cue.cue_id);
        hash_str(&mut h, &cue.source_event_id);
        hash_str(&mut h, &cue.recipient_actor_id);
        hash_str(&mut h, &cue.observation_payload_digest);
        hash_u64(&mut h, cue.observed_tick);
        hash_u64(&mut h, cue.generated_tick);
        hash_opt_u64(&mut h, cue.delivered_tick);
        hash_str(&mut h, &cue.visibility_profile_digest);
        h.update(&[match cue.disposition {
            ObservationDisposition::Delivered => 0,
            ObservationDisposition::Occluded => 1,
            ObservationDisposition::Dropped => 2,
            ObservationDisposition::Stale => 3,
        }]);
    }

    let mut decisions = branch.decisions.iter().collect::<Vec<_>>();
    decisions.sort_by(|a, b| (a.tick, &a.actor_id).cmp(&(b.tick, &b.actor_id)));
    hash_len(&mut h, decisions.len());
    for decision in decisions {
        hash_str(&mut h, &decision.decision_receipt_id);
        hash_str(&mut h, &decision.actor_id);
        hash_u64(&mut h, decision.tick);
        hash_str(&mut h, &decision.policy_digest);
        hash_str(&mut h, &decision.config_digest);
        hash_str(&mut h, &decision.prior_policy_state_digest);
        hash_strs(&mut h, &decision.visible_cue_ids);
        hash_strs(&mut h, &decision.consumed_cue_ids);
        hash_strs(&mut h, &decision.prior_action_receipt_ids);
        hash_str(&mut h, &decision.permitted_history_digest);
        hash_u64(&mut h, decision.deadline_tick);
        hash_opt_u64(&mut h, decision.command_latch_tick);
        hash_opt_str(&mut h, decision.action_id.as_deref());
        hash_opt_str(&mut h, decision.action_payload_digest.as_deref());
        h.update(&[match decision.disposition {
            DecisionDisposition::Completed => 0,
            DecisionDisposition::TimedOut => 1,
            DecisionDisposition::Failed => 2,
            DecisionDisposition::Missing => 3,
        }]);
    }

    let mut transitions = branch.transitions.iter().collect::<Vec<_>>();
    transitions.sort_by_key(|t| t.tick);
    hash_len(&mut h, transitions.len());
    for transition in transitions {
        hash_u64(&mut h, transition.tick);
        hash_str(&mut h, &transition.parent_snapshot_digest);
        hash_str(&mut h, &transition.child_snapshot_digest);
        hash_str(&mut h, &transition.joint_action_digest);
        hash_str(&mut h, &transition.physics_step_receipt_digest);
        h.update(&[match transition.disposition {
            TransitionDisposition::Completed => 0,
            TransitionDisposition::Failed => 1,
            TransitionDisposition::Censored => 2,
            TransitionDisposition::Missing => 3,
        }]);
    }
    hex(h.finalize().as_bytes())
}

fn hash_manifest(h: &mut blake3::Hasher, m: &ExperimentManifest) {
    hash_str(h, &m.experiment_id);
    hash_str(h, &m.scenario_digest);
    hash_str(h, &m.odd_scope_digest);
    hash_str(h, &m.initial_world_snapshot_digest);
    hash_str(h, &m.physics_profile_digest);
    hash_str(h, &m.execution_profile_digest);
    hash_str(h, &m.observation_schedule_digest);
    hash_str(h, &m.action_domain_digest);
    hash_str(h, &m.units_profile_digest);
    hash_u64(h, m.tick_hz_numerator);
    hash_u64(h, m.tick_hz_denominator);
}

fn hash_policy(h: &mut blake3::Hasher, p: &PolicyProfile) {
    hash_str(h, &p.actor_id);
    hash_str(h, &p.policy_digest);
    hash_str(h, &p.config_digest);
    hash_str(h, &p.initial_policy_state_digest);
    hash_str(h, &p.observation_profile_digest);
    hash_str(h, &p.action_domain_digest);
}

fn hash_replay(h: &mut blake3::Hasher, r: &ReplayEvidence) {
    hash_str(h, &r.replay_profile_digest);
    hash_str(h, &r.expected_branch_payload_root);
    hash_opt_str(h, r.replayed_branch_payload_root.as_deref());
    h.update(&[match r.disposition {
        ReplayDisposition::ExactSameRuntimeBytes => 0,
        ReplayDisposition::PortableSemanticEquivalent => 1,
        ReplayDisposition::NotRun => 2,
        ReplayDisposition::Failed => 3,
    }]);
}

fn hash_strs(h: &mut blake3::Hasher, values: &[String]) {
    hash_len(h, values.len());
    for value in values { hash_str(h, value); }
}

fn hash_str(h: &mut blake3::Hasher, value: &str) {
    hash_len(h, value.len());
    h.update(value.as_bytes());
}

fn hash_opt_str(h: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => { h.update(&[1]); hash_str(h, value); }
        None => { h.update(&[0]); }
    }
}

fn hash_opt_u64(h: &mut blake3::Hasher, value: Option<u64>) {
    match value {
        Some(value) => { h.update(&[1]); hash_u64(h, value); }
        None => { h.update(&[0]); }
    }
}

fn hash_u64(h: &mut blake3::Hasher, value: u64) {
    h.update(&value.to_le_bytes());
}

fn hash_len(h: &mut blake3::Hasher, length: usize) {
    let length = u64::try_from(length).expect("finite manifest length must fit u64");
    h.update(&length.to_le_bytes());
}

fn branch_outcome_tag(value: BranchOutcome) -> u8 {
    match value {
        BranchOutcome::Completed => 0,
        BranchOutcome::TaskFailed => 1,
        BranchOutcome::TimedOut => 2,
        BranchOutcome::ExecutionFailed => 3,
        BranchOutcome::Censored => 4,
    }
}

fn require(value: &str, field: &'static str) -> Result<(), HandoffError> {
    if value.trim().is_empty() {
        Err(HandoffError::EmptyIdentity(field))
    } else {
        Ok(())
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> String {
        format!("sha256:{value}")
    }

    fn policy(actor: &str) -> PolicyProfile {
        PolicyProfile {
            actor_id: actor.into(),
            policy_digest: d(&format!("policy-{actor}")),
            config_digest: d(&format!("config-{actor}")),
            initial_policy_state_digest: d(&format!("state-{actor}")),
            observation_profile_digest: d(&format!("view-{actor}")),
            action_domain_digest: d("actions-v1"),
        }
    }

    fn cue(
        id: &str,
        recipient: &str,
        observed: u64,
        generated: u64,
        delivered: u64,
    ) -> ObservationDelivery {
        ObservationDelivery {
            cue_id: id.into(),
            source_event_id: d(&format!("event-{id}")),
            recipient_actor_id: recipient.into(),
            observation_payload_digest: d(&format!("payload-{id}")),
            observed_tick: observed,
            generated_tick: generated,
            delivered_tick: Some(delivered),
            delivery_sequence: delivered,
            visibility_profile_digest: d(&format!("visibility-{recipient}")),
            disposition: ObservationDisposition::Delivered,
        }
    }

    fn decision(
        actor: &str,
        tick: u64,
        visible: Vec<String>,
        consumed: Vec<String>,
        prior: Vec<String>,
        cue_index: &BTreeMap<String, ObservationDelivery>,
        prior_state: &str,
        action: &str,
    ) -> PolicyDecision {
        let policy = policy(actor);
        let mut h = blake3::Hasher::new_derive_key(
            "Symtropy EGCI permitted policy history v0.1",
        );
        hash_str(&mut h, actor);
        hash_u64(&mut h, tick);
        hash_str(&mut h, prior_state);
        hash_str(&mut h, &policy.observation_profile_digest);
        hash_len(&mut h, visible.len());
        for id in &visible {
            let cue = &cue_index[id];
            hash_str(&mut h, &cue.cue_id);
            hash_str(&mut h, &cue.source_event_id);
            hash_str(&mut h, &cue.observation_payload_digest);
            hash_u64(&mut h, cue.observed_tick);
            hash_u64(&mut h, cue.generated_tick);
            hash_u64(&mut h, cue.delivered_tick.unwrap());
            hash_u64(&mut h, cue.delivery_sequence);
            hash_str(&mut h, &cue.visibility_profile_digest);
        }
        hash_len(&mut h, prior.len());
        for id in &prior { hash_str(&mut h, id); }
        PolicyDecision {
            decision_receipt_id: d(&format!("decision-{actor}-{tick}")),
            actor_id: actor.into(),
            tick,
            policy_digest: policy.policy_digest,
            config_digest: policy.config_digest,
            prior_policy_state_digest: prior_state.into(),
            next_policy_state_digest: Some(d(&format!("next-{actor}-{tick}"))),
            visible_cue_ids: visible,
            consumed_cue_ids: consumed,
            prior_action_receipt_ids: prior,
            permitted_history_digest: hex(h.finalize().as_bytes()),
            deadline_tick: tick + 1,
            command_latch_tick: Some(tick),
            action_id: Some(action.into()),
            action_payload_digest: Some(d(&format!("action-{actor}-{tick}"))),
            disposition: DecisionDisposition::Completed,
        }
    }

    fn fixture() -> (EpisodeExport, ExternalAnchor) {
        let policies = vec![policy("ego"), policy("actor-1")];
        let observations = vec![
            cue("ego-sees-actor", "ego", 0, 0, 0),
            cue("actor-sees-ego", "actor-1", 0, 0, 0),
            cue("ego-sees-yield", "ego", 0, 1, 1),
            cue("actor-sees-continue", "actor-1", 0, 1, 1),
        ];
        let mut cue_index = BTreeMap::new();
        for item in &observations { cue_index.insert(item.cue_id.clone(), item.clone()); }

        let decisions = vec![
            decision("ego", 0, vec!["ego-sees-actor".into()], vec!["ego-sees-actor".into()], vec![], &cue_index, &d("state-ego"), "continue"),
            decision("actor-1", 0, vec!["actor-sees-ego".into()], vec!["actor-sees-ego".into()], vec![], &cue_index, &d("state-actor-1"), "yield"),
            decision("ego", 1, vec!["ego-sees-actor".into(), "ego-sees-yield".into()], vec!["ego-sees-yield".into()], vec![d("decision-ego-0")], &cue_index, &d("next-ego-0"), "yield"),
            decision("actor-1", 1, vec!["actor-sees-ego".into(), "actor-sees-continue".into()], vec!["actor-sees-continue".into()], vec![d("decision-actor-1-0")], &cue_index, &d("next-actor-1-0"), "continue"),
        ];

        let mut transitions = vec![
            JointTransition {
                tick: 0,
                parent_snapshot_digest: d("world-0"),
                child_snapshot_digest: d("world-1"),
                joint_action_digest: joint_action_digest(&decisions, 0),
                physics_step_receipt_digest: d("physics-0"),
                disposition: TransitionDisposition::Completed,
            },
            JointTransition {
                tick: 1,
                parent_snapshot_digest: d("world-1"),
                child_snapshot_digest: d("world-2"),
                joint_action_digest: joint_action_digest(&decisions, 1),
                physics_step_receipt_digest: d("physics-1"),
                disposition: TransitionDisposition::Completed,
            },
        ];
        transitions.sort_by_key(|t| t.tick);

        let corr = vec![
            CorrespondenceReceipt {
                hypothesis_id: "actor-keyed".into(),
                correspondence_digest: d("map-actor"),
                causal_dispositions_digest: d("causal-actor"),
                projection_input_digest: d("projection-actor"),
                draw_bundle_digest: d("draw-actor"),
                verifier_profile_digest: d("verifier"),
                verifier_receipt_digest: d("verification-receipt"),
            },
            CorrespondenceReceipt {
                hypothesis_id: "slot-keyed".into(),
                correspondence_digest: d("map-slot"),
                causal_dispositions_digest: d("causal-slot"),
                projection_input_digest: d("projection-slot"),
                draw_bundle_digest: d("draw-slot"),
                verifier_profile_digest: d("verifier"),
                verifier_receipt_digest: d("verification-receipt"),
            },
        ];
        let branch = BranchExport {
            branch_id: "continue-branch".into(),
            parent_snapshot_digest: d("world-0"),
            intervention_digest: d("continue-intervention"),
            outcome: BranchOutcome::Completed,
            candidate_universe: CandidateUniverseDisposition::ExhaustiveWithinDeclaredScope,
            hypothesis_set: HypothesisSetDisposition::ExhaustiveWithinDeclaredModelFamily,
            correspondences: corr,
            observations,
            decisions,
            transitions,
            replay: ReplayEvidence {
                replay_profile_digest: d("fresh-process-replay"),
                expected_branch_payload_root: String::new(),
                replayed_branch_payload_root: Some(payload_root.clone()),
                disposition: ReplayDisposition::ExactSameRuntimeBytes,
            },
        };
        let mut branch = branch;
        let payload_root = branch_payload_root(&branch);
        branch.replay.expected_branch_payload_root = payload_root.clone();
        branch.replay.replayed_branch_payload_root = Some(payload_root);

        let manifest = ExperimentManifest {
            experiment_id: "crossing-001".into(),
            scenario_digest: d("scenario"),
            odd_scope_digest: d("odd"),
            initial_world_snapshot_digest: d("world-0"),
            physics_profile_digest: d("physics-profile"),
            execution_profile_digest: d("execution-profile"),
            observation_schedule_digest: d("observation-schedule"),
            action_domain_digest: d("action-domain"),
            units_profile_digest: d("metric-si"),
            tick_hz_numerator: 20,
            tick_hz_denominator: 1,
        };
        let episode = EpisodeExport {
            schema: EPISODE_SCHEMA.into(),
            manifest,
            policies,
            branches: vec![branch],
        };
        let episode_root = episode.episode_root().unwrap();
        let anchors = vec![
            DrawBundleAnchor { branch_id: "continue-branch".into(), hypothesis_id: "actor-keyed".into(), expected_bundle_digest: d("draw-actor") },
            DrawBundleAnchor { branch_id: "continue-branch".into(), hypothesis_id: "slot-keyed".into(), expected_bundle_digest: d("draw-slot") },
        ];
        let anchor = ExternalAnchor {
            authority_reference: "independent-evaluator:fixture-anchor-001".into(),
            expected_episode_root: episode_root,
            draw_bundle_anchors: anchors,
        };
        (episode, anchor)
    }

    fn joint_action_digest(decisions: &[PolicyDecision], tick: u64) -> String {
        let mut at_tick: Vec<_> = decisions.iter().filter(|d| d.tick == tick).collect();
        at_tick.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
        let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI joint action receipt v0.1");
        for decision in at_tick {
            hash_str(&mut h, &decision.actor_id);
            hash_str(&mut h, decision.action_id.as_deref().unwrap());
            hash_str(&mut h, decision.action_payload_digest.as_deref().unwrap());
        }
        hex(h.finalize().as_bytes())
    }

    #[test]
    fn emits_exact_sam_query_shape_and_preserves_each_correspondence_hypothesis() {
        let (episode, anchor) = fixture();
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert_eq!(handoff.evidence_class, EvidenceClass::SimulationGenerated);
        assert_eq!(handoff.eligibility, EvidenceEligibility::FormalConformanceOnly);
        assert_eq!(handoff.branches.len(), 1);
        assert_eq!(handoff.branches[0].queries.len(), 2);
        assert_ne!(
            handoff.branches[0].queries[0].hypothesis_id,
            handoff.branches[0].queries[1].hypothesis_id
        );
        for query in &handoff.branches[0].queries {
            assert_eq!(query.query.steps.len(), 2);
            assert_eq!(query.query.steps[0].joint_actions.len(), 2);
            assert_eq!(query.query.target_policy_digest.len(), 64);
            assert!(query.query_json.starts_with("{\"target_policy_digest\":"));
        }
    }

    #[test]
    fn future_or_undelivered_observation_is_rejected() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].observations[2].delivered_tick = Some(2);
        let root = episode.episode_root().unwrap();
        let anchor = ExternalAnchor { expected_episode_root: root, ..anchor };
        assert_eq!(export_to_sam(&episode, &anchor), Err(HandoffError::FutureCueLeak));
    }

    #[test]
    fn missing_decision_and_incomplete_replay_remain_incomplete() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].decisions.retain(|d| !(d.tick == 1 && d.actor_id == "actor-1"));
        let root = episode.episode_root().unwrap();
        let mut branch = episode.branches[0].clone();
        branch.replay.expected_branch_payload_root = branch_payload_root(&branch);
        branch.replay.replayed_branch_payload_root = None;
        branch.replay.disposition = ReplayDisposition::NotRun;
        episode.branches[0] = branch;
        let anchor = ExternalAnchor { expected_episode_root: episode.episode_root().unwrap(), ..anchor };
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(handoff.branches[0].queries.iter().all(|q| !q.query.complete));
    }

    #[test]
    fn incomplete_correspondence_search_cannot_be_promoted_by_replay() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].hypothesis_set = HypothesisSetDisposition::IncompleteOrBudgetLimited;
        let payload = branch_payload_root(&episode.branches[0]);
        episode.branches[0].replay.expected_branch_payload_root = payload.clone();
        episode.branches[0].replay.replayed_branch_payload_root = Some(payload);
        let anchor = ExternalAnchor { expected_episode_root: episode.episode_root().unwrap(), ..anchor };
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(handoff.branches[0].queries.iter().all(|q| !q.query.complete));
    }

    #[test]
    fn externally_anchored_bundle_digest_must_match_exact_hypothesis() {
        let (episode, mut anchor) = fixture();
        anchor.draw_bundle_anchors[0].expected_bundle_digest = d("wrong-draw");
        anchor.expected_episode_root = episode.episode_root().unwrap();
        assert_eq!(export_to_sam(&episode, &anchor), Err(HandoffError::DrawBundleAnchorMismatch));
    }

    #[test]
    fn branch_replay_root_and_episode_root_are_separate_anchors() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].replay.replayed_branch_payload_root = Some(d("replacement-root"));
        let root = episode.episode_root().unwrap();
        let anchor = ExternalAnchor { expected_episode_root: root, ..anchor };
        assert_eq!(export_to_sam(&episode, &anchor), Err(HandoffError::ReplayReceiptMismatch));
    }

    #[test]
    fn task_failure_is_a_complete_path_not_a_censored_record() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].outcome = BranchOutcome::TaskFailed;
        let payload = branch_payload_root(&episode.branches[0]);
        episode.branches[0].replay.expected_branch_payload_root = payload.clone();
        episode.branches[0].replay.replayed_branch_payload_root = Some(payload);
        let anchor = ExternalAnchor { expected_episode_root: episode.episode_root().unwrap(), ..anchor };
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(handoff.branches[0].queries.iter().all(|q| q.query.complete));
    }

    #[test]
    fn joint_action_order_is_canonical_and_json_escapes_strings() {
        let query = SamTrajectoryQuery {
            target_policy_digest: "policy".into(),
            context_digest: "context".into(),
            complete: true,
            steps: vec![SamDecisionStep {
                tick: 0,
                history_digest: "history".into(),
                joint_actions: vec![
                    SamActorAction { actor_id: "z".into(), action_id: "quote\"".into(), payload_digest: "p".into() },
                    SamActorAction { actor_id: "a".into(), action_id: "yield".into(), payload_digest: "q".into() },
                ],
            }],
        };
        let json = sam_query_json(&query);
        assert!(json.contains("quote\\\""));
    }
}
