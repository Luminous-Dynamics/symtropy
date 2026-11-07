#![forbid(unsafe_code)]

//! Bounded reciprocal-episode export into the SAM #10514 trajectory-query shape.
//!
//! This crate is an adapter and conformance oracle. It does not implement a
//! simulator, infer human behaviour, estimate empirical support, or make a
//! safety decision. The SAM support implementation remains the sole owner of
//! finite trajectory membership semantics.

use std::collections::{BTreeMap, BTreeSet};

const EPISODE_SCHEMA: &str = "symtropy.road-egci-reciprocal-episode.v0.2";
const QUERY_SCHEMA: &str = "sam.policy-trajectory-query.v0.1";
const HASH_DOMAIN: &str = "Symtropy EGCI reciprocal episode export v0.2";

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
    /// Branch-global event ordinal at which the source observation was generated.
    pub generated_event_sequence: u64,
    pub delivered_tick: Option<u64>,
    /// Branch-global event ordinal at which the cue became available to its recipient.
    pub delivered_event_sequence: Option<u64>,
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
    /// Shared per-tick boundary after all cues admitted to this policy input have arrived.
    pub input_cutoff_event_sequence: u64,
    /// Branch-global event ordinal when this policy decision was committed.
    pub decision_event_sequence: u64,
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
    /// Branch-global event ordinal when the command was latched, if latched.
    pub command_latch_event_sequence: Option<u64>,
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
    /// Branch-global event ordinal when this physics transition committed.
    pub physics_event_sequence: u64,
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

impl BranchExport {
    /// Recompute the canonical v0.2 branch payload commitment. This is a
    /// content hash, not a signature or attestation.
    pub fn payload_root(&self) -> String {
        branch_payload_root(self)
    }
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
    /// One #3743 bundle contains all admitted correspondence hypotheses for this branch.
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
    DuplicateEventSequence,
    InvalidEventOrdering,
    NonMonotoneDeliverySequence,
    DuplicateDecisionReceipt,
    DuplicateDecisionTickActor,
    DuplicateTransitionTick,
    DuplicateHypothesis,
    MissingPolicy,
    PolicyMismatch,
    MissingCue,
    WrongCueRecipient,
    ObservationProfileMismatch,
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
    pub query_schema_id: String,
    pub episode_root: String,
    /// Full canonical source episode artifact; not just the reduced SAM query.
    pub episode_json: String,
    pub anchor_authority_reference: String,
    pub evidence_class: EvidenceClass,
    pub eligibility: EvidenceEligibility,
    pub branches: Vec<BranchHandoff>,
}

#[derive(Clone, Debug)]
struct VerifiedDecision {
    actor_id: String,
    tick: u64,
    history_digest: String,
    input_cutoff_event_sequence: u64,
    decision_event_sequence: u64,
    latch_event_sequence: Option<u64>,
    action_id: String,
    payload_digest: String,
    latch_tick: u64,
    disposition: DecisionDisposition,
}

impl EpisodeExport {
    /// Serialize the complete handoff input with deterministic collection ordering.
    /// The digest is over typed fields, not claimed to be a signature of this JSON.
    pub fn canonical_json(&self) -> Result<String, HandoffError> {
        if self.schema != EPISODE_SCHEMA {
            return Err(HandoffError::WrongEpisodeSchema);
        }
        let mut output = String::new();
        output.push_str("{\"schema\":");
        json_string(&mut output, &self.schema);
        output.push_str(",\"manifest\":");
        write_manifest_json(&mut output, &self.manifest);
        output.push_str(",\"policies\":[");
        let mut policies: Vec<_> = self.policies.iter().collect();
        policies.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
        for (index, policy) in policies.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write_policy_json(&mut output, policy);
        }
        output.push_str("],\"branches\":[");
        let mut branches: Vec<_> = self.branches.iter().collect();
        branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
        for (index, branch) in branches.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write_branch_json(&mut output, branch);
        }
        output.push_str("]}");
        Ok(output)
    }

    /// Hash the complete episode inputs. This is an unkeyed content commitment,
    /// not a signature and not proof that source evidence is true.
    pub fn episode_root(&self) -> Result<String, HandoffError> {
        if self.schema != EPISODE_SCHEMA {
            return Err(HandoffError::WrongEpisodeSchema);
        }
        let mut h = blake3::Hasher::new_derive_key(HASH_DOMAIN);
        hash_str(&mut h, &self.schema);
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
    if episode.branches.is_empty() {
        return Err(HandoffError::MissingBranchParent);
    }
    for policy in policies.values() {
        if policy.action_domain_digest != episode.manifest.action_domain_digest {
            return Err(HandoffError::PolicyMismatch);
        }
    }
    let target_policy_digest = target_policy_digest(&episode.policies)?;
    let base_context_digest = context_digest(&episode.manifest, &episode.policies)?;
    let anchors = draw_anchor_index(&anchor.draw_bundle_anchors)?;

    let mut branches: Vec<_> = episode.branches.iter().collect();
    branches.sort_by(|a, b| a.branch_id.cmp(&b.branch_id));
    let mut branch_ids = BTreeSet::new();
    if branches
        .iter()
        .any(|branch| !branch_ids.insert(branch.branch_id.as_str()))
    {
        return Err(HandoffError::DuplicateBranch);
    }
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
            ReplayDisposition::NotRun | ReplayDisposition::Failed => {
                if branch.replay.replayed_branch_payload_root.is_some() {
                    return Err(HandoffError::ReplayReceiptMismatch);
                }
            }
        }

        let receipts = correspondence_index(&branch.correspondences)?;
        let cues = observation_index(&branch.observations)?;
        let decisions = verify_decisions(branch, &policies, &cues)?;
        validate_branch_event_order(branch, &cues, &decisions)?;
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
                .get(&branch.branch_id)
                .ok_or(HandoffError::MissingDrawBundleAnchor)?;
            if expected != &receipt.draw_bundle_digest {
                return Err(HandoffError::DrawBundleAnchorMismatch);
            }

            let hypothesis_context_digest = hypothesis_context_digest(
                &base_context_digest,
                &branch.branch_id,
                &hypothesis_id,
                receipt,
            );
            let query = SamTrajectoryQuery {
                target_policy_digest: target_policy_digest.clone(),
                context_digest: hypothesis_context_digest,
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

    if anchors.len() != output.len() {
        return Err(HandoffError::MissingDrawBundleAnchor);
    }

    Ok(SamHandoff {
        query_schema_id: QUERY_SCHEMA.to_owned(),
        episode_root,
        episode_json: episode.canonical_json()?,
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
    require(
        &manifest.initial_world_snapshot_digest,
        "initial_world_snapshot_digest",
    )?;
    require(&manifest.physics_profile_digest, "physics_profile_digest")?;
    require(
        &manifest.execution_profile_digest,
        "execution_profile_digest",
    )?;
    require(
        &manifest.observation_schedule_digest,
        "observation_schedule_digest",
    )?;
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
    require(
        &branch.replay.replay_profile_digest,
        "replay_profile_digest",
    )?;
    require(
        &branch.replay.expected_branch_payload_root,
        "expected_branch_payload_root",
    )?;
    Ok(())
}

fn policy_index(
    policies: &[PolicyProfile],
) -> Result<BTreeMap<&str, &PolicyProfile>, HandoffError> {
    let mut index = BTreeMap::new();
    for policy in policies {
        require(&policy.actor_id, "actor_id")?;
        require(&policy.policy_digest, "policy_digest")?;
        require(&policy.config_digest, "config_digest")?;
        require(
            &policy.initial_policy_state_digest,
            "initial_policy_state_digest",
        )?;
        require(
            &policy.observation_profile_digest,
            "observation_profile_digest",
        )?;
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

fn observation_index(
    observations: &[ObservationDelivery],
) -> Result<BTreeMap<&str, &ObservationDelivery>, HandoffError> {
    let mut index = BTreeMap::new();
    let mut delivery_sequences = BTreeSet::new();
    for cue in observations {
        require(&cue.cue_id, "cue_id")?;
        require(&cue.source_event_id, "source_event_id")?;
        require(&cue.recipient_actor_id, "recipient_actor_id")?;
        require(
            &cue.observation_payload_digest,
            "observation_payload_digest",
        )?;
        require(&cue.visibility_profile_digest, "visibility_profile_digest")?;
        if cue.observed_tick > cue.generated_tick
            || cue
                .delivered_event_sequence
                .is_some_and(|sequence| sequence <= cue.generated_event_sequence)
        {
            return Err(HandoffError::InvalidObservationTiming);
        }
        if cue.disposition == ObservationDisposition::Delivered
            && !delivery_sequences.insert((cue.recipient_actor_id.as_str(), cue.delivery_sequence))
        {
            return Err(HandoffError::DuplicateCue);
        }
        match (cue.disposition, cue.delivered_tick) {
            (ObservationDisposition::Delivered, Some(delivered)) => {
                if delivered < cue.generated_tick || cue.delivered_event_sequence.is_none() {
                    return Err(HandoffError::InvalidObservationTiming);
                }
            }
            (ObservationDisposition::Delivered, None)
            | (ObservationDisposition::Occluded, Some(_))
            | (ObservationDisposition::Dropped, Some(_))
            | (ObservationDisposition::Stale, Some(_)) => {
                return Err(HandoffError::InvalidObservationTiming);
            }
            _ => {
                if cue.delivered_event_sequence.is_some() {
                    return Err(HandoffError::InvalidObservationTiming);
                }
            }
        }
        if index.insert(cue.cue_id.as_str(), cue).is_some() {
            return Err(HandoffError::DuplicateCue);
        }
    }
    let mut per_recipient: BTreeMap<&str, Vec<&ObservationDelivery>> = BTreeMap::new();
    for cue in index
        .values()
        .filter(|cue| cue.disposition == ObservationDisposition::Delivered)
    {
        per_recipient
            .entry(cue.recipient_actor_id.as_str())
            .or_default()
            .push(*cue);
    }
    for cues in per_recipient.values_mut() {
        cues.sort_by_key(|cue| cue.delivery_sequence);
        if cues
            .windows(2)
            .any(|pair| pair[0].delivered_tick.unwrap() > pair[1].delivered_tick.unwrap())
        {
            return Err(HandoffError::NonMonotoneDeliverySequence);
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

        // Check explicitly admitted cues before comparing the complete visible
        // channel, so a same-tick post-cutoff cue reports the causal leak rather
        // than only the secondary visible-history mismatch.
        for cue_id in &decision.visible_cue_ids {
            let cue = cues.get(cue_id.as_str()).ok_or(HandoffError::MissingCue)?;
            if cue.recipient_actor_id == decision.actor_id
                && cue.disposition == ObservationDisposition::Delivered
                && cue.delivered_tick.is_some_and(|tick| tick <= decision.tick)
                && cue
                    .delivered_event_sequence
                    .is_some_and(|sequence| sequence >= decision.input_cutoff_event_sequence)
            {
                return Err(HandoffError::FutureCueLeak);
            }
        }

        let mut expected_visible: Vec<_> = cues
            .values()
            .filter(|cue| {
                cue.recipient_actor_id == decision.actor_id
                    && cue.disposition == ObservationDisposition::Delivered
                    && cue.delivered_tick.is_some_and(|tick| tick <= decision.tick)
                    && cue
                        .delivered_event_sequence
                        .is_some_and(|sequence| sequence < decision.input_cutoff_event_sequence)
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
            blake3::Hasher::new_derive_key("Symtropy EGCI permitted policy history v0.2");
        hash_str(&mut history, &decision.actor_id);
        hash_u64(&mut history, decision.tick);
        hash_u64(&mut history, decision.input_cutoff_event_sequence);
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
            if cue.visibility_profile_digest != policy.observation_profile_digest {
                return Err(HandoffError::ObservationProfileMismatch);
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
            hash_u64(&mut history, cue.generated_event_sequence);
            hash_u64(&mut history, delivered_tick);
            hash_u64(
                &mut history,
                cue.delivered_event_sequence
                    .ok_or(HandoffError::CueNotDelivered)?,
            );
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
            if prior.tick >= decision.tick || prior.disposition != DecisionDisposition::Completed {
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
            && decision.command_latch_event_sequence.is_some()
            && decision.next_policy_state_digest.is_some();
        if decision.command_latch_tick.is_some() != decision.command_latch_event_sequence.is_some()
            || decision.decision_event_sequence <= decision.input_cutoff_event_sequence
            || decision
                .command_latch_event_sequence
                .is_some_and(|sequence| sequence <= decision.decision_event_sequence)
        {
            return Err(HandoffError::InvalidEventOrdering);
        }
        match decision.disposition {
            DecisionDisposition::Completed => {
                if !complete_action
                    || decision
                        .command_latch_tick
                        .is_none_or(|tick| tick < decision.tick || tick > decision.deadline_tick)
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
            input_cutoff_event_sequence: decision.input_cutoff_event_sequence,
            decision_event_sequence: decision.decision_event_sequence,
            latch_event_sequence: decision.command_latch_event_sequence,
            action_id: decision.action_id.clone().unwrap_or_default(),
            payload_digest: decision.action_payload_digest.clone().unwrap_or_default(),
            latch_tick: decision.command_latch_tick.unwrap_or(u64::MAX),
            disposition: decision.disposition,
        });
    }
    Ok(verified)
}

fn validate_branch_event_order(
    branch: &BranchExport,
    cues: &BTreeMap<&str, &ObservationDelivery>,
    decisions: &[VerifiedDecision],
) -> Result<(), HandoffError> {
    let mut event_sequences = BTreeSet::new();
    let mut cutoffs_by_tick = BTreeMap::new();

    let mut generated_sources = BTreeMap::new();
    for cue in cues.values() {
        match generated_sources.insert(cue.generated_event_sequence, cue.source_event_id.as_str()) {
            Some(existing_source) if existing_source != cue.source_event_id => {
                return Err(HandoffError::DuplicateEventSequence);
            }
            Some(_) => {}
            None => {
                if !event_sequences.insert(cue.generated_event_sequence) {
                    return Err(HandoffError::DuplicateEventSequence);
                }
            }
        }
        if let Some(sequence) = cue.delivered_event_sequence
            && !event_sequences.insert(sequence)
        {
            return Err(HandoffError::DuplicateEventSequence);
        }
    }

    for decision in decisions {
        match cutoffs_by_tick.insert(decision.tick, decision.input_cutoff_event_sequence) {
            Some(existing) if existing != decision.input_cutoff_event_sequence => {
                return Err(HandoffError::InvalidEventOrdering);
            }
            _ => {}
        }
        if !event_sequences.insert(decision.decision_event_sequence) {
            return Err(HandoffError::DuplicateEventSequence);
        }
        if let Some(sequence) = decision.latch_event_sequence
            && !event_sequences.insert(sequence)
        {
            return Err(HandoffError::DuplicateEventSequence);
        }
    }

    let mut transitions: Vec<_> = branch.transitions.iter().collect();
    transitions.sort_by_key(|transition| transition.tick);
    let mut prior_physics = None;
    let mut physics_by_tick = BTreeMap::new();
    for transition in &transitions {
        if !event_sequences.insert(transition.physics_event_sequence) {
            return Err(HandoffError::DuplicateEventSequence);
        }
        if prior_physics.is_some_and(|prior| transition.physics_event_sequence <= prior) {
            return Err(HandoffError::InvalidEventOrdering);
        }
        prior_physics = Some(transition.physics_event_sequence);
        physics_by_tick.insert(transition.tick, transition.physics_event_sequence);
    }

    let mut prior_cutoff = None;
    for (tick, cutoff) in &cutoffs_by_tick {
        if !event_sequences.insert(*cutoff) {
            return Err(HandoffError::DuplicateEventSequence);
        }
        if prior_cutoff.is_some_and(|prior| *cutoff <= prior) {
            return Err(HandoffError::InvalidEventOrdering);
        }
        prior_cutoff = Some(*cutoff);
        let physics = physics_by_tick
            .get(tick)
            .ok_or(HandoffError::InvalidEventOrdering)?;
        if cutoff >= physics {
            return Err(HandoffError::InvalidEventOrdering);
        }
    }

    for cue in cues.values() {
        let generated_cutoff = cutoffs_by_tick
            .get(&cue.generated_tick)
            .ok_or(HandoffError::InvalidObservationTiming)?;
        if cue.generated_event_sequence >= *generated_cutoff {
            return Err(HandoffError::InvalidObservationTiming);
        }
        if cue.generated_tick > 0 {
            let prior_tick = cue.generated_tick - 1;
            let prior_physics = physics_by_tick
                .get(&prior_tick)
                .ok_or(HandoffError::InvalidObservationTiming)?;
            if cue.generated_event_sequence <= *prior_physics {
                return Err(HandoffError::InvalidObservationTiming);
            }
        }
        if let (Some(delivered_tick), Some(delivered_sequence)) =
            (cue.delivered_tick, cue.delivered_event_sequence)
        {
            let delivery_cutoff = cutoffs_by_tick
                .get(&delivered_tick)
                .ok_or(HandoffError::InvalidObservationTiming)?;
            if delivered_sequence >= *delivery_cutoff {
                return Err(HandoffError::InvalidObservationTiming);
            }
            if delivered_tick > 0 {
                let prior_physics = physics_by_tick
                    .get(&(delivered_tick - 1))
                    .ok_or(HandoffError::InvalidObservationTiming)?;
                if delivered_sequence <= *prior_physics {
                    return Err(HandoffError::InvalidObservationTiming);
                }
            }
        }
    }

    Ok(())
}

fn build_joint_steps(
    branch: &BranchExport,
    policies: &BTreeMap<&str, &PolicyProfile>,
    decisions: &[VerifiedDecision],
) -> Result<Vec<SamDecisionStep>, HandoffError> {
    let mut transitions: Vec<_> = branch.transitions.iter().collect();
    transitions.sort_by_key(|transition| transition.tick);
    if transitions
        .windows(2)
        .any(|pair| pair[0].tick == pair[1].tick)
    {
        return Err(HandoffError::DuplicateTransitionTick);
    }

    let mut steps = Vec::new();
    for transition in transitions {
        require(
            &transition.physics_step_receipt_digest,
            "transition.physics_step_receipt_digest",
        )?;

        let mut at_tick: Vec<_> = decisions
            .iter()
            .filter(|decision| decision.tick == transition.tick)
            .collect();
        at_tick.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));

        if at_tick.iter().any(|decision| {
            decision.input_cutoff_event_sequence >= transition.physics_event_sequence
        }) {
            return Err(HandoffError::InvalidEventOrdering);
        }
        let max_decision_sequence = at_tick
            .iter()
            .map(|decision| decision.decision_event_sequence)
            .max();
        let latch_sequences: Vec<_> = at_tick
            .iter()
            .filter_map(|decision| decision.latch_event_sequence)
            .collect();
        if !latch_sequences.is_empty()
            && (*latch_sequences.iter().max().unwrap() >= transition.physics_event_sequence
                || max_decision_sequence.is_some_and(|max_decision| {
                    *latch_sequences.iter().min().unwrap() <= max_decision
                }))
        {
            return Err(HandoffError::InvalidEventOrdering);
        }
        if at_tick.len() != policies.len()
            || at_tick
                .iter()
                .any(|decision| decision.disposition != DecisionDisposition::Completed)
        {
            continue;
        }
        if at_tick
            .iter()
            .any(|decision| decision.latch_tick != transition.tick)
        {
            return Err(HandoffError::InvalidDecisionReceipt);
        }

        let mut actions = Vec::with_capacity(at_tick.len());
        let mut action_hash =
            blake3::Hasher::new_derive_key("Symtropy EGCI joint action receipt v0.1");
        hash_len(&mut action_hash, at_tick.len());
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
    let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI joint history v0.2");
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

    let mut transitions: Vec<_> = branch.transitions.iter().collect();
    transitions.sort_by_key(|transition| transition.tick);
    if transitions
        .windows(2)
        .any(|pair| pair[0].tick == pair[1].tick)
    {
        return false;
    }

    let transition_ticks: BTreeSet<_> = transitions
        .iter()
        .map(|transition| transition.tick)
        .collect();
    let decision_ticks: BTreeSet<_> = branch
        .decisions
        .iter()
        .map(|decision| decision.tick)
        .collect();
    if transition_ticks != decision_ticks {
        return false;
    }

    let mut prior_child: Option<&str> = None;
    let mut prior_tick = None;
    for (index, transition) in transitions.iter().enumerate() {
        if transition.disposition != TransitionDisposition::Completed
            || steps
                .get(index)
                .is_none_or(|step| step.tick != transition.tick)
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

fn correspondence_index(
    receipts: &[CorrespondenceReceipt],
) -> Result<BTreeMap<String, &CorrespondenceReceipt>, HandoffError> {
    let mut index = BTreeMap::new();
    for receipt in receipts {
        require(&receipt.hypothesis_id, "hypothesis_id")?;
        require(&receipt.correspondence_digest, "correspondence_digest")?;
        require(
            &receipt.causal_dispositions_digest,
            "causal_dispositions_digest",
        )?;
        require(&receipt.projection_input_digest, "projection_input_digest")?;
        require(&receipt.draw_bundle_digest, "draw_bundle_digest")?;
        require(&receipt.verifier_profile_digest, "verifier_profile_digest")?;
        require(&receipt.verifier_receipt_digest, "verifier_receipt_digest")?;
        if index
            .insert(receipt.hypothesis_id.clone(), receipt)
            .is_some()
        {
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
) -> Result<BTreeMap<String, String>, HandoffError> {
    let mut index = BTreeMap::new();
    for anchor in anchors {
        require(&anchor.branch_id, "draw anchor branch_id")?;
        require(&anchor.expected_bundle_digest, "expected_bundle_digest")?;
        if index
            .insert(
                anchor.branch_id.clone(),
                anchor.expected_bundle_digest.clone(),
            )
            .is_some()
        {
            return Err(HandoffError::DuplicateBranch);
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

fn hypothesis_context_digest(
    base_context_digest: &str,
    branch_id: &str,
    hypothesis_id: &str,
    receipt: &CorrespondenceReceipt,
) -> String {
    let mut h =
        blake3::Hasher::new_derive_key("Symtropy EGCI hypothesis-qualified SAM context v0.1");
    hash_str(&mut h, base_context_digest);
    hash_str(&mut h, branch_id);
    hash_str(&mut h, hypothesis_id);
    hash_str(&mut h, &receipt.correspondence_digest);
    hash_str(&mut h, &receipt.causal_dispositions_digest);
    hash_str(&mut h, &receipt.projection_input_digest);
    hash_str(&mut h, &receipt.draw_bundle_digest);
    hash_str(&mut h, &receipt.verifier_profile_digest);
    hash_str(&mut h, &receipt.verifier_receipt_digest);
    hex(h.finalize().as_bytes())
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
        if index > 0 {
            output.push(',');
        }
        output.push_str("{\"tick\":");
        output.push_str(&step.tick.to_string());
        output.push_str(",\"history_digest\":");
        json_string(&mut output, &step.history_digest);
        output.push_str(",\"joint_actions\":[");
        for (action_index, action) in step.joint_actions.iter().enumerate() {
            if action_index > 0 {
                output.push(',');
            }
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

fn write_manifest_json(out: &mut String, m: &ExperimentManifest) {
    out.push_str("{\"experiment_id\":");
    json_string(out, &m.experiment_id);
    out.push_str(",\"scenario_digest\":");
    json_string(out, &m.scenario_digest);
    out.push_str(",\"odd_scope_digest\":");
    json_string(out, &m.odd_scope_digest);
    out.push_str(",\"initial_world_snapshot_digest\":");
    json_string(out, &m.initial_world_snapshot_digest);
    out.push_str(",\"physics_profile_digest\":");
    json_string(out, &m.physics_profile_digest);
    out.push_str(",\"execution_profile_digest\":");
    json_string(out, &m.execution_profile_digest);
    out.push_str(",\"observation_schedule_digest\":");
    json_string(out, &m.observation_schedule_digest);
    out.push_str(",\"action_domain_digest\":");
    json_string(out, &m.action_domain_digest);
    out.push_str(",\"units_profile_digest\":");
    json_string(out, &m.units_profile_digest);
    out.push_str(",\"tick_hz_numerator\":");
    out.push_str(&m.tick_hz_numerator.to_string());
    out.push_str(",\"tick_hz_denominator\":");
    out.push_str(&m.tick_hz_denominator.to_string());
    out.push('}');
}

fn write_policy_json(out: &mut String, p: &PolicyProfile) {
    out.push_str("{\"actor_id\":");
    json_string(out, &p.actor_id);
    out.push_str(",\"policy_digest\":");
    json_string(out, &p.policy_digest);
    out.push_str(",\"config_digest\":");
    json_string(out, &p.config_digest);
    out.push_str(",\"initial_policy_state_digest\":");
    json_string(out, &p.initial_policy_state_digest);
    out.push_str(",\"observation_profile_digest\":");
    json_string(out, &p.observation_profile_digest);
    out.push_str(",\"action_domain_digest\":");
    json_string(out, &p.action_domain_digest);
    out.push('}');
}

fn write_string_array(out: &mut String, values: &[String]) {
    out.push('[');
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        json_string(out, value);
    }
    out.push(']');
}

fn write_optional_string(out: &mut String, value: Option<&str>) {
    match value {
        Some(value) => json_string(out, value),
        None => out.push_str("null"),
    }
}

fn write_optional_u64(out: &mut String, value: Option<u64>) {
    match value {
        Some(value) => out.push_str(&value.to_string()),
        None => out.push_str("null"),
    }
}

fn write_branch_json(out: &mut String, b: &BranchExport) {
    out.push_str("{\"branch_id\":");
    json_string(out, &b.branch_id);
    out.push_str(",\"parent_snapshot_digest\":");
    json_string(out, &b.parent_snapshot_digest);
    out.push_str(",\"intervention_digest\":");
    json_string(out, &b.intervention_digest);
    out.push_str(",\"outcome\":");
    json_string(out, branch_outcome_name(b.outcome));
    out.push_str(",\"candidate_universe\":");
    json_string(out, candidate_universe_name(b.candidate_universe));
    out.push_str(",\"hypothesis_set\":");
    json_string(out, hypothesis_set_name(b.hypothesis_set));
    out.push_str(",\"correspondences\":[");
    let mut correspondences: Vec<_> = b.correspondences.iter().collect();
    correspondences.sort_by(|a, c| a.hypothesis_id.cmp(&c.hypothesis_id));
    for (index, receipt) in correspondences.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"hypothesis_id\":");
        json_string(out, &receipt.hypothesis_id);
        out.push_str(",\"correspondence_digest\":");
        json_string(out, &receipt.correspondence_digest);
        out.push_str(",\"causal_dispositions_digest\":");
        json_string(out, &receipt.causal_dispositions_digest);
        out.push_str(",\"projection_input_digest\":");
        json_string(out, &receipt.projection_input_digest);
        out.push_str(",\"draw_bundle_digest\":");
        json_string(out, &receipt.draw_bundle_digest);
        out.push_str(",\"verifier_profile_digest\":");
        json_string(out, &receipt.verifier_profile_digest);
        out.push_str(",\"verifier_receipt_digest\":");
        json_string(out, &receipt.verifier_receipt_digest);
        out.push('}');
    }

    out.push_str("],\"observations\":[");
    let mut observations: Vec<_> = b.observations.iter().collect();
    observations.sort_by(|a, c| {
        (
            &a.recipient_actor_id,
            a.delivered_tick,
            a.delivery_sequence,
            &a.cue_id,
        )
            .cmp(&(
                &c.recipient_actor_id,
                c.delivered_tick,
                c.delivery_sequence,
                &c.cue_id,
            ))
    });
    for (index, cue) in observations.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"cue_id\":");
        json_string(out, &cue.cue_id);
        out.push_str(",\"source_event_id\":");
        json_string(out, &cue.source_event_id);
        out.push_str(",\"recipient_actor_id\":");
        json_string(out, &cue.recipient_actor_id);
        out.push_str(",\"observation_payload_digest\":");
        json_string(out, &cue.observation_payload_digest);
        out.push_str(",\"observed_tick\":");
        out.push_str(&cue.observed_tick.to_string());
        out.push_str(",\"generated_tick\":");
        out.push_str(&cue.generated_tick.to_string());
        out.push_str(",\"generated_event_sequence\":");
        out.push_str(&cue.generated_event_sequence.to_string());
        out.push_str(",\"delivered_tick\":");
        write_optional_u64(out, cue.delivered_tick);
        out.push_str(",\"delivered_event_sequence\":");
        write_optional_u64(out, cue.delivered_event_sequence);
        out.push_str(",\"delivery_sequence\":");
        out.push_str(&cue.delivery_sequence.to_string());
        out.push_str(",\"visibility_profile_digest\":");
        json_string(out, &cue.visibility_profile_digest);
        out.push_str(",\"disposition\":");
        json_string(out, observation_disposition_name(cue.disposition));
        out.push('}');
    }

    out.push_str("],\"decisions\":[");
    let mut decisions: Vec<_> = b.decisions.iter().collect();
    decisions.sort_by(|a, c| (a.tick, &a.actor_id).cmp(&(c.tick, &c.actor_id)));
    for (index, decision) in decisions.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"decision_receipt_id\":");
        json_string(out, &decision.decision_receipt_id);
        out.push_str(",\"actor_id\":");
        json_string(out, &decision.actor_id);
        out.push_str(",\"tick\":");
        out.push_str(&decision.tick.to_string());
        out.push_str(",\"input_cutoff_event_sequence\":");
        out.push_str(&decision.input_cutoff_event_sequence.to_string());
        out.push_str(",\"decision_event_sequence\":");
        out.push_str(&decision.decision_event_sequence.to_string());
        out.push_str(",\"policy_digest\":");
        json_string(out, &decision.policy_digest);
        out.push_str(",\"config_digest\":");
        json_string(out, &decision.config_digest);
        out.push_str(",\"prior_policy_state_digest\":");
        json_string(out, &decision.prior_policy_state_digest);
        out.push_str(",\"next_policy_state_digest\":");
        write_optional_string(out, decision.next_policy_state_digest.as_deref());
        out.push_str(",\"visible_cue_ids\":");
        write_string_array(out, &decision.visible_cue_ids);
        out.push_str(",\"consumed_cue_ids\":");
        write_string_array(out, &decision.consumed_cue_ids);
        out.push_str(",\"prior_action_receipt_ids\":");
        write_string_array(out, &decision.prior_action_receipt_ids);
        out.push_str(",\"permitted_history_digest\":");
        json_string(out, &decision.permitted_history_digest);
        out.push_str(",\"deadline_tick\":");
        out.push_str(&decision.deadline_tick.to_string());
        out.push_str(",\"command_latch_tick\":");
        write_optional_u64(out, decision.command_latch_tick);
        out.push_str(",\"command_latch_event_sequence\":");
        write_optional_u64(out, decision.command_latch_event_sequence);
        out.push_str(",\"action_id\":");
        write_optional_string(out, decision.action_id.as_deref());
        out.push_str(",\"action_payload_digest\":");
        write_optional_string(out, decision.action_payload_digest.as_deref());
        out.push_str(",\"disposition\":");
        json_string(out, decision_disposition_name(decision.disposition));
        out.push('}');
    }

    out.push_str("],\"transitions\":[");
    let mut transitions: Vec<_> = b.transitions.iter().collect();
    transitions.sort_by_key(|transition| transition.tick);
    for (index, transition) in transitions.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"tick\":");
        out.push_str(&transition.tick.to_string());
        out.push_str(",\"physics_event_sequence\":");
        out.push_str(&transition.physics_event_sequence.to_string());
        out.push_str(",\"parent_snapshot_digest\":");
        json_string(out, &transition.parent_snapshot_digest);
        out.push_str(",\"child_snapshot_digest\":");
        json_string(out, &transition.child_snapshot_digest);
        out.push_str(",\"joint_action_digest\":");
        json_string(out, &transition.joint_action_digest);
        out.push_str(",\"physics_step_receipt_digest\":");
        json_string(out, &transition.physics_step_receipt_digest);
        out.push_str(",\"disposition\":");
        json_string(out, transition_disposition_name(transition.disposition));
        out.push('}');
    }

    out.push_str("],\"replay\":{");
    out.push_str("\"replay_profile_digest\":");
    json_string(out, &b.replay.replay_profile_digest);
    out.push_str(",\"expected_branch_payload_root\":");
    json_string(out, &b.replay.expected_branch_payload_root);
    out.push_str(",\"replayed_branch_payload_root\":");
    write_optional_string(out, b.replay.replayed_branch_payload_root.as_deref());
    out.push_str(",\"disposition\":");
    json_string(out, replay_disposition_name(b.replay.disposition));
    out.push_str("}}");
}

fn observation_disposition_name(value: ObservationDisposition) -> &'static str {
    match value {
        ObservationDisposition::Delivered => "delivered",
        ObservationDisposition::Occluded => "occluded",
        ObservationDisposition::Dropped => "dropped",
        ObservationDisposition::Stale => "stale",
    }
}

fn decision_disposition_name(value: DecisionDisposition) -> &'static str {
    match value {
        DecisionDisposition::Completed => "completed",
        DecisionDisposition::TimedOut => "timed_out",
        DecisionDisposition::Failed => "failed",
        DecisionDisposition::Missing => "missing",
    }
}

fn transition_disposition_name(value: TransitionDisposition) -> &'static str {
    match value {
        TransitionDisposition::Completed => "completed",
        TransitionDisposition::Failed => "failed",
        TransitionDisposition::Censored => "censored",
        TransitionDisposition::Missing => "missing",
    }
}

fn branch_outcome_name(value: BranchOutcome) -> &'static str {
    match value {
        BranchOutcome::Completed => "completed",
        BranchOutcome::TaskFailed => "task_failed",
        BranchOutcome::TimedOut => "timed_out",
        BranchOutcome::ExecutionFailed => "execution_failed",
        BranchOutcome::Censored => "censored",
    }
}

fn candidate_universe_name(value: CandidateUniverseDisposition) -> &'static str {
    match value {
        CandidateUniverseDisposition::ExhaustiveWithinDeclaredScope => {
            "exhaustive_within_declared_scope"
        }
        CandidateUniverseDisposition::Incomplete => "incomplete",
    }
}

fn hypothesis_set_name(value: HypothesisSetDisposition) -> &'static str {
    match value {
        HypothesisSetDisposition::ExhaustiveWithinDeclaredModelFamily => {
            "exhaustive_within_declared_model_family"
        }
        HypothesisSetDisposition::IncompleteOrBudgetLimited => "incomplete_or_budget_limited",
    }
}

fn replay_disposition_name(value: ReplayDisposition) -> &'static str {
    match value {
        ReplayDisposition::ExactSameRuntimeBytes => "exact_same_runtime_bytes",
        ReplayDisposition::PortableSemanticEquivalent => "portable_semantic_equivalent",
        ReplayDisposition::NotRun => "not_run",
        ReplayDisposition::Failed => "failed",
    }
}

fn branch_payload_root(branch: &BranchExport) -> String {
    let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI branch payload v0.2");
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
    observations.sort_by(|a, b| {
        (&a.recipient_actor_id, &a.cue_id).cmp(&(&b.recipient_actor_id, &b.cue_id))
    });
    hash_len(&mut h, observations.len());
    for cue in observations {
        hash_str(&mut h, &cue.cue_id);
        hash_str(&mut h, &cue.source_event_id);
        hash_str(&mut h, &cue.recipient_actor_id);
        hash_str(&mut h, &cue.observation_payload_digest);
        hash_u64(&mut h, cue.observed_tick);
        hash_u64(&mut h, cue.generated_tick);
        hash_u64(&mut h, cue.generated_event_sequence);
        hash_opt_u64(&mut h, cue.delivered_tick);
        hash_opt_u64(&mut h, cue.delivered_event_sequence);
        hash_u64(&mut h, cue.delivery_sequence);
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
        hash_u64(&mut h, decision.input_cutoff_event_sequence);
        hash_u64(&mut h, decision.decision_event_sequence);
        hash_str(&mut h, &decision.policy_digest);
        hash_str(&mut h, &decision.config_digest);
        hash_str(&mut h, &decision.prior_policy_state_digest);
        hash_opt_str(&mut h, decision.next_policy_state_digest.as_deref());
        hash_strs(&mut h, &decision.visible_cue_ids);
        hash_strs(&mut h, &decision.consumed_cue_ids);
        hash_strs(&mut h, &decision.prior_action_receipt_ids);
        hash_str(&mut h, &decision.permitted_history_digest);
        hash_u64(&mut h, decision.deadline_tick);
        hash_opt_u64(&mut h, decision.command_latch_tick);
        hash_opt_u64(&mut h, decision.command_latch_event_sequence);
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
        hash_u64(&mut h, transition.physics_event_sequence);
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
    for value in values {
        hash_str(h, value);
    }
}

fn hash_str(h: &mut blake3::Hasher, value: &str) {
    hash_len(h, value.len());
    h.update(value.as_bytes());
}

fn hash_opt_str(h: &mut blake3::Hasher, value: Option<&str>) {
    match value {
        Some(value) => {
            h.update(&[1]);
            hash_str(h, value);
        }
        None => {
            h.update(&[0]);
        }
    }
}

fn hash_opt_u64(h: &mut blake3::Hasher, value: Option<u64>) {
    match value {
        Some(value) => {
            h.update(&[1]);
            hash_u64(h, value);
        }
        None => {
            h.update(&[0]);
        }
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
            observation_profile_digest: d(&format!("visibility-{actor}")),
            action_domain_digest: d("action-domain"),
        }
    }

    fn cue(
        id: &str,
        recipient: &str,
        observed: u64,
        generated: u64,
        delivered: u64,
        generated_event_sequence: u64,
        delivered_event_sequence: u64,
    ) -> ObservationDelivery {
        ObservationDelivery {
            cue_id: id.into(),
            source_event_id: d(&format!("event-{id}")),
            recipient_actor_id: recipient.into(),
            observation_payload_digest: d(&format!("payload-{id}")),
            observed_tick: observed,
            generated_tick: generated,
            generated_event_sequence,
            delivered_tick: Some(delivered),
            delivered_event_sequence: Some(delivered_event_sequence),
            delivery_sequence: delivered,
            visibility_profile_digest: d(&format!("visibility-{recipient}")),
            disposition: ObservationDisposition::Delivered,
        }
    }

    struct DecisionSpec<'a> {
        actor: &'a str,
        tick: u64,
        input_cutoff_event_sequence: u64,
        decision_event_sequence: u64,
        command_latch_event_sequence: u64,
        visible: Vec<String>,
        consumed: Vec<String>,
        prior: Vec<String>,
        prior_state: &'a str,
        action: &'a str,
    }

    fn decision(
        spec: DecisionSpec<'_>,
        cue_index: &BTreeMap<String, ObservationDelivery>,
    ) -> PolicyDecision {
        let DecisionSpec {
            actor,
            tick,
            input_cutoff_event_sequence,
            decision_event_sequence,
            command_latch_event_sequence,
            visible,
            consumed,
            prior,
            prior_state,
            action,
        } = spec;
        let policy = policy(actor);
        let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI permitted policy history v0.2");
        hash_str(&mut h, actor);
        hash_u64(&mut h, tick);
        hash_u64(&mut h, input_cutoff_event_sequence);
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
            hash_u64(&mut h, cue.generated_event_sequence);
            hash_u64(&mut h, cue.delivered_tick.unwrap());
            hash_u64(&mut h, cue.delivered_event_sequence.unwrap());
            hash_u64(&mut h, cue.delivery_sequence);
            hash_str(&mut h, &cue.visibility_profile_digest);
        }
        hash_len(&mut h, prior.len());
        for id in &prior {
            hash_str(&mut h, id);
        }
        PolicyDecision {
            decision_receipt_id: d(&format!("decision-{actor}-{tick}")),
            actor_id: actor.into(),
            tick,
            input_cutoff_event_sequence,
            decision_event_sequence,
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
            command_latch_event_sequence: Some(command_latch_event_sequence),
            action_id: Some(action.into()),
            action_payload_digest: Some(d(&format!("action-{actor}-{tick}"))),
            disposition: DecisionDisposition::Completed,
        }
    }

    fn fixture() -> (EpisodeExport, ExternalAnchor) {
        let policies = vec![policy("ego"), policy("actor-1")];
        let observations = vec![
            cue("ego-sees-actor", "ego", 0, 0, 0, 1, 2),
            cue("actor-sees-ego", "actor-1", 0, 0, 0, 3, 4),
            cue("ego-sees-yield", "ego", 0, 1, 1, 11, 12),
            cue("actor-sees-continue", "actor-1", 0, 1, 1, 13, 14),
        ];
        let mut cue_index = BTreeMap::new();
        for item in &observations {
            cue_index.insert(item.cue_id.clone(), item.clone());
        }

        let decisions = vec![
            decision(
                DecisionSpec {
                    actor: "ego",
                    tick: 0,
                    input_cutoff_event_sequence: 5,
                    decision_event_sequence: 6,
                    command_latch_event_sequence: 8,
                    visible: vec!["ego-sees-actor".into()],
                    consumed: vec!["ego-sees-actor".into()],
                    prior: vec![],
                    prior_state: &d("state-ego"),
                    action: "continue",
                },
                &cue_index,
            ),
            decision(
                DecisionSpec {
                    actor: "actor-1",
                    tick: 0,
                    input_cutoff_event_sequence: 5,
                    decision_event_sequence: 7,
                    command_latch_event_sequence: 9,
                    visible: vec!["actor-sees-ego".into()],
                    consumed: vec!["actor-sees-ego".into()],
                    prior: vec![],
                    prior_state: &d("state-actor-1"),
                    action: "yield",
                },
                &cue_index,
            ),
            decision(
                DecisionSpec {
                    actor: "ego",
                    tick: 1,
                    input_cutoff_event_sequence: 15,
                    decision_event_sequence: 16,
                    command_latch_event_sequence: 18,
                    visible: vec!["ego-sees-actor".into(), "ego-sees-yield".into()],
                    consumed: vec!["ego-sees-yield".into()],
                    prior: vec![d("decision-ego-0")],
                    prior_state: &d("next-ego-0"),
                    action: "yield",
                },
                &cue_index,
            ),
            decision(
                DecisionSpec {
                    actor: "actor-1",
                    tick: 1,
                    input_cutoff_event_sequence: 15,
                    decision_event_sequence: 17,
                    command_latch_event_sequence: 19,
                    visible: vec!["actor-sees-ego".into(), "actor-sees-continue".into()],
                    consumed: vec!["actor-sees-continue".into()],
                    prior: vec![d("decision-actor-1-0")],
                    prior_state: &d("next-actor-1-0"),
                    action: "continue",
                },
                &cue_index,
            ),
        ];

        let mut transitions = vec![
            JointTransition {
                tick: 0,
                physics_event_sequence: 10,
                parent_snapshot_digest: d("world-0"),
                child_snapshot_digest: d("world-1"),
                joint_action_digest: joint_action_digest(&decisions, 0),
                physics_step_receipt_digest: d("physics-0"),
                disposition: TransitionDisposition::Completed,
            },
            JointTransition {
                tick: 1,
                physics_event_sequence: 20,
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
                draw_bundle_digest: d("draw-bundle-continue"),
                verifier_profile_digest: d("verifier"),
                verifier_receipt_digest: d("verification-receipt"),
            },
            CorrespondenceReceipt {
                hypothesis_id: "slot-keyed".into(),
                correspondence_digest: d("map-slot"),
                causal_dispositions_digest: d("causal-slot"),
                projection_input_digest: d("projection-slot"),
                draw_bundle_digest: d("draw-bundle-continue"),
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
                replayed_branch_payload_root: None,
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
        let anchors = vec![DrawBundleAnchor {
            branch_id: "continue-branch".into(),
            expected_bundle_digest: d("draw-bundle-continue"),
        }];
        let anchor = ExternalAnchor {
            authority_reference: "independent-evaluator:fixture-anchor-001".into(),
            expected_episode_root: episode_root,
            draw_bundle_anchors: anchors,
        };
        (episode, anchor)
    }

    fn reanchor(episode: &mut EpisodeExport, anchor: &ExternalAnchor) -> ExternalAnchor {
        for branch in &mut episode.branches {
            let payload = branch_payload_root(branch);
            branch.replay.expected_branch_payload_root = payload.clone();
            branch.replay.replayed_branch_payload_root = match branch.replay.disposition {
                ReplayDisposition::ExactSameRuntimeBytes
                | ReplayDisposition::PortableSemanticEquivalent => Some(payload),
                ReplayDisposition::NotRun | ReplayDisposition::Failed => None,
            };
        }
        ExternalAnchor {
            expected_episode_root: episode.episode_root().expect("episode root"),
            ..anchor.clone()
        }
    }

    fn two_branch_fixture() -> (EpisodeExport, ExternalAnchor) {
        let (mut episode, mut anchor) = fixture();
        let mut second = episode.branches[0].clone();
        second.branch_id = "yield-branch".into();
        second.intervention_digest = d("yield-intervention");
        for receipt in &mut second.correspondences {
            receipt.draw_bundle_digest = d("draw-bundle-yield");
            receipt.correspondence_digest = d(&format!("yield-{}", receipt.hypothesis_id));
            receipt.causal_dispositions_digest =
                d(&format!("yield-causal-{}", receipt.hypothesis_id));
            receipt.projection_input_digest =
                d(&format!("yield-projection-{}", receipt.hypothesis_id));
        }
        second.outcome = BranchOutcome::Completed;
        episode.branches.push(second);
        anchor.draw_bundle_anchors.push(DrawBundleAnchor {
            branch_id: "yield-branch".into(),
            expected_bundle_digest: d("draw-bundle-yield"),
        });
        let anchor = reanchor(&mut episode, &anchor);
        (episode, anchor)
    }

    fn joint_action_digest(decisions: &[PolicyDecision], tick: u64) -> String {
        let mut at_tick: Vec<_> = decisions.iter().filter(|d| d.tick == tick).collect();
        at_tick.sort_by(|a, b| a.actor_id.cmp(&b.actor_id));
        let mut h = blake3::Hasher::new_derive_key("Symtropy EGCI joint action receipt v0.1");
        hash_len(&mut h, at_tick.len());
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
        assert_eq!(handoff.query_schema_id, QUERY_SCHEMA);
        assert_eq!(handoff.evidence_class, EvidenceClass::SimulationGenerated);
        assert_eq!(
            handoff.eligibility,
            EvidenceEligibility::FormalConformanceOnly
        );
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
            assert_eq!(query.query.context_digest.len(), 64);
            assert!(query.query_json.starts_with("{\"target_policy_digest\":"));
        }
        assert_ne!(
            handoff.branches[0].queries[0].query.context_digest,
            handoff.branches[0].queries[1].query.context_digest
        );
    }

    #[test]
    fn canonical_episode_artifact_preserves_every_source_layer_and_is_order_independent() {
        let (episode, anchor) = fixture();
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(
            handoff
                .episode_json
                .contains("\"observation_payload_digest\"")
        );
        assert!(
            handoff
                .episode_json
                .contains("\"prior_action_receipt_ids\"")
        );
        assert!(handoff.episode_json.contains("\"correspondence_digest\""));
        assert!(
            handoff
                .episode_json
                .contains("\"physics_step_receipt_digest\"")
        );

        let mut reordered = episode.clone();
        reordered.policies.reverse();
        reordered.branches.reverse();
        for branch in &mut reordered.branches {
            branch.correspondences.reverse();
            branch.observations.reverse();
            branch.decisions.reverse();
            branch.transitions.reverse();
        }
        assert_eq!(
            episode.canonical_json().unwrap(),
            reordered.canonical_json().unwrap()
        );
        assert_eq!(
            episode.episode_root().unwrap(),
            reordered.episode_root().unwrap()
        );
    }

    #[test]
    fn both_policies_react_after_the_other_policy_observation_is_delivered() {
        let (episode, anchor) = fixture();
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        let query = &handoff.branches[0].queries[0].query;
        let action = |tick: usize, actor: &str| {
            query.steps[tick]
                .joint_actions
                .iter()
                .find(|action| action.actor_id == actor)
                .unwrap()
                .action_id
                .as_str()
        };
        assert_eq!(action(0, "ego"), "continue");
        assert_eq!(action(1, "ego"), "yield");
        assert_eq!(action(0, "actor-1"), "yield");
        assert_eq!(action(1, "actor-1"), "continue");
        assert!(episode.branches[0].decisions.iter().any(|decision| {
            decision.actor_id == "ego"
                && decision.tick == 1
                && decision
                    .consumed_cue_ids
                    .contains(&"ego-sees-yield".to_owned())
        }));
        assert!(episode.branches[0].decisions.iter().any(|decision| {
            decision.actor_id == "actor-1"
                && decision.tick == 1
                && decision
                    .consumed_cue_ids
                    .contains(&"actor-sees-continue".to_owned())
        }));
    }

    #[test]
    fn delivered_cue_cannot_be_omitted_from_visible_history() {
        let (mut episode, anchor) = fixture();
        let decision = episode.branches[0]
            .decisions
            .iter_mut()
            .find(|decision| decision.actor_id == "ego" && decision.tick == 1)
            .unwrap();
        decision
            .visible_cue_ids
            .retain(|cue| cue != "ego-sees-yield");
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::CueNotDelivered)
        );
    }

    #[test]
    fn policy_state_chain_rejects_stale_prior_state() {
        let (mut episode, anchor) = fixture();
        episode.branches[0]
            .decisions
            .iter_mut()
            .find(|decision| decision.actor_id == "ego" && decision.tick == 1)
            .unwrap()
            .prior_policy_state_digest = d("stale-policy-state");
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::PolicyMismatch)
        );
    }

    #[test]
    fn policy_cannot_import_another_actors_prior_action_into_its_history() {
        let (mut episode, anchor) = fixture();
        episode.branches[0]
            .decisions
            .iter_mut()
            .find(|decision| decision.actor_id == "actor-1" && decision.tick == 1)
            .unwrap()
            .prior_action_receipt_ids
            .push(d("decision-ego-0"));
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::MissingPriorAction)
        );
    }

    #[test]
    fn joint_action_payload_mutation_is_rejected_by_transition_receipt() {
        let (mut episode, anchor) = fixture();
        episode.branches[0]
            .decisions
            .iter_mut()
            .find(|decision| decision.actor_id == "ego" && decision.tick == 0)
            .unwrap()
            .action_payload_digest = Some(d("tampered-action"));
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::JointActionDigestMismatch)
        );
    }

    #[test]
    fn censored_transition_remains_incomplete_even_when_other_receipts_replay() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].transitions[1].disposition = TransitionDisposition::Censored;
        let anchor = reanchor(&mut episode, &anchor);
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(
            handoff.branches[0]
                .queries
                .iter()
                .all(|query| !query.query.complete)
        );
    }

    #[test]
    fn policy_and_context_changes_rebind_the_sam_query() {
        let (episode, anchor) = fixture();
        let baseline = export_to_sam(&episode, &anchor).unwrap();

        let mut changed_policy = episode.clone();
        let changed_actor = changed_policy.policies[0].actor_id.clone();
        let changed_digest = d("new-policy");
        changed_policy.policies[0].policy_digest = changed_digest.clone();
        for decision in &mut changed_policy.branches[0].decisions {
            if decision.actor_id == changed_actor {
                decision.policy_digest = changed_digest.clone();
            }
        }
        let policy_anchor = reanchor(&mut changed_policy, &anchor);
        let policy_updated = export_to_sam(&changed_policy, &policy_anchor).unwrap();
        assert_ne!(
            baseline.branches[0].queries[0].query.target_policy_digest,
            policy_updated.branches[0].queries[0]
                .query
                .target_policy_digest
        );
        assert_eq!(
            baseline.branches[0].queries[0].query.context_digest,
            policy_updated.branches[0].queries[0].query.context_digest
        );

        let mut changed_context = episode.clone();
        changed_context.manifest.scenario_digest = d("new-scenario");
        let context_anchor = reanchor(&mut changed_context, &anchor);
        let context_updated = export_to_sam(&changed_context, &context_anchor).unwrap();
        assert_eq!(
            baseline.branches[0].queries[0].query.target_policy_digest,
            context_updated.branches[0].queries[0]
                .query
                .target_policy_digest
        );
        assert_ne!(
            baseline.branches[0].queries[0].query.context_digest,
            context_updated.branches[0].queries[0].query.context_digest
        );
    }

    #[test]
    fn exports_each_branch_and_preserves_branch_specific_incompleteness() {
        let (mut episode, anchor) = two_branch_fixture();
        episode
            .branches
            .iter_mut()
            .find(|branch| branch.branch_id == "yield-branch")
            .unwrap()
            .transitions[1]
            .disposition = TransitionDisposition::Censored;
        let anchor = reanchor(&mut episode, &anchor);
        let handoff = export_to_sam(&episode, &anchor).unwrap();

        assert_eq!(handoff.branches.len(), 2);
        let complete = handoff
            .branches
            .iter()
            .find(|branch| branch.branch_id == "continue-branch")
            .unwrap();
        let censored = handoff
            .branches
            .iter()
            .find(|branch| branch.branch_id == "yield-branch")
            .unwrap();
        assert_eq!(complete.queries.len(), 2);
        assert_eq!(censored.queries.len(), 2);
        assert!(complete.queries.iter().all(|query| query.query.complete));
        assert!(censored.queries.iter().all(|query| !query.query.complete));
    }

    #[test]
    fn duplicate_branch_identity_is_rejected() {
        let (mut episode, anchor) = two_branch_fixture();
        episode.branches[1].branch_id = episode.branches[0].branch_id.clone();
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::DuplicateBranch)
        );
    }

    #[test]
    fn observation_delivery_sequence_must_follow_delivery_time() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].observations[0].delivery_sequence = 2;
        episode.branches[0].observations[2].delivery_sequence = 1;
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::NonMonotoneDeliverySequence)
        );
    }

    #[test]
    fn same_tick_post_physics_observation_cannot_enter_an_already_closed_policy_input() {
        let (mut episode, anchor) = fixture();
        let cue = episode.branches[0]
            .observations
            .iter_mut()
            .find(|cue| cue.cue_id == "ego-sees-yield")
            .unwrap();
        // Tick 1 physics commits at event 20. This cue is generated afterward,
        // but its integer generation and delivery ticks still say "1".
        cue.generated_event_sequence = 21;
        cue.delivered_event_sequence = Some(22);
        let anchor = reanchor(&mut episode, &anchor);
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::FutureCueLeak)
        );
    }

    #[test]
    fn same_tick_predecision_delivery_remains_visible() {
        let (episode, anchor) = fixture();
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(
            handoff.branches[0]
                .queries
                .iter()
                .all(|query| query.query.complete)
        );
        assert!(episode.branches[0].decisions.iter().any(|decision| {
            decision.actor_id == "ego"
                && decision.tick == 1
                && decision
                    .consumed_cue_ids
                    .contains(&"ego-sees-yield".to_owned())
        }));
    }

    #[test]
    fn future_or_undelivered_observation_is_rejected() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].observations[2].delivered_tick = Some(2);
        let payload = branch_payload_root(&episode.branches[0]);
        episode.branches[0].replay.expected_branch_payload_root = payload.clone();
        episode.branches[0].replay.replayed_branch_payload_root = Some(payload);
        let root = episode.episode_root().unwrap();
        let anchor = ExternalAnchor {
            expected_episode_root: root,
            ..anchor
        };
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::CueNotDelivered)
        );
    }

    #[test]
    fn missing_decision_and_incomplete_replay_remain_incomplete() {
        let (mut episode, anchor) = fixture();
        episode.branches[0]
            .decisions
            .retain(|d| !(d.tick == 1 && d.actor_id == "actor-1"));
        let mut branch = episode.branches[0].clone();
        branch.replay.expected_branch_payload_root = branch_payload_root(&branch);
        branch.replay.replayed_branch_payload_root = None;
        branch.replay.disposition = ReplayDisposition::NotRun;
        episode.branches[0] = branch;
        let anchor = ExternalAnchor {
            expected_episode_root: episode.episode_root().unwrap(),
            ..anchor
        };
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(
            handoff.branches[0]
                .queries
                .iter()
                .all(|q| !q.query.complete)
        );
    }

    #[test]
    fn incomplete_correspondence_search_cannot_be_promoted_by_replay() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].hypothesis_set = HypothesisSetDisposition::IncompleteOrBudgetLimited;
        let payload = branch_payload_root(&episode.branches[0]);
        episode.branches[0].replay.expected_branch_payload_root = payload.clone();
        episode.branches[0].replay.replayed_branch_payload_root = Some(payload);
        let anchor = ExternalAnchor {
            expected_episode_root: episode.episode_root().unwrap(),
            ..anchor
        };
        let handoff = export_to_sam(&episode, &anchor).unwrap();
        assert!(
            handoff.branches[0]
                .queries
                .iter()
                .all(|q| !q.query.complete)
        );
    }

    #[test]
    fn externally_anchored_bundle_digest_must_match_exact_hypothesis() {
        let (episode, mut anchor) = fixture();
        anchor.draw_bundle_anchors[0].expected_bundle_digest = d("wrong-draw");
        anchor.expected_episode_root = episode.episode_root().unwrap();
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::DrawBundleAnchorMismatch)
        );
    }

    #[test]
    fn branch_replay_root_and_episode_root_are_separate_anchors() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].replay.replayed_branch_payload_root = Some(d("replacement-root"));
        let root = episode.episode_root().unwrap();
        let anchor = ExternalAnchor {
            expected_episode_root: root,
            ..anchor
        };
        assert_eq!(
            export_to_sam(&episode, &anchor),
            Err(HandoffError::ReplayReceiptMismatch)
        );
    }

    #[test]
    fn task_failure_is_a_complete_path_not_a_censored_record() {
        let (mut episode, anchor) = fixture();
        episode.branches[0].outcome = BranchOutcome::TaskFailed;
        let payload = branch_payload_root(&episode.branches[0]);
        episode.branches[0].replay.expected_branch_payload_root = payload.clone();
        episode.branches[0].replay.replayed_branch_payload_root = Some(payload);
        let anchor = ExternalAnchor {
            expected_episode_root: episode.episode_root().unwrap(),
            ..anchor
        };
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
                    SamActorAction {
                        actor_id: "z".into(),
                        action_id: "quote\"".into(),
                        payload_digest: "p".into(),
                    },
                    SamActorAction {
                        actor_id: "a".into(),
                        action_id: "yield".into(),
                        payload_digest: "q".into(),
                    },
                ],
            }],
        };
        let json = sam_query_json(&query);
        assert!(json.contains("quote\\\""));
    }
}
