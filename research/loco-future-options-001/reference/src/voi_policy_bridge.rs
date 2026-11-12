//! Bridge from viability-conditioned VOI outcomes into bounded policy beliefs.
//!
//! This module does not calculate VOI, joint probability mass, probe safety,
//! physical successor sets or viability. It consumes the exact outputs of the
//! LOCO-VIABLE-VOI-001 owners and maps them into the finite policy-tree input.
//!
//! A critical composition rule: two VOI outcome IDs that map to the same
//! delivered observation-equivalence class do NOT create two separately
//! selectable actions. The child belief node receives the intersection of
//! both outcomes' upstream-admitted action sets. The outcome-local selected
//! actions are retained as diagnostics, not used to grant hidden information.

use std::collections::{BTreeMap, BTreeSet};

use super::EvidenceManifest;
use super::policy_tree::{
    BeliefNode, BeliefStateReceipt, FinitePolicyTree, ObservationDisposition,
    PolicyTransition, StateActionEvidence,
};

pub const VOI_JOINT_MASS_SCALE_PPB: u64 = 1_000_000_000;
const UNKNOWN_OBSERVATION_CLASS: &str = "__unknown_observation__";
const INVALID_OBSERVATION_CLASS: &str = "__invalid_observation__";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamVoiGate {
    Evaluated,
    PhysicalRejected,
    PhysicalUnknown,
    ResourceRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchOutcomeKey {
    pub branch_id: String,
    pub outcome_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePolicyContext {
    pub root_node_id: String,
    pub probe_action_id: String,
    pub probe_receipt_digest: String,
    pub initial_observation_receipt_digest: String,
    pub initial_belief_state_set_digest: String,
    pub initial_state: BeliefStateReceipt,
    pub action_domain: EvidenceManifest,
    pub action_ids: Vec<String>,
    /// Exact physical branch/outcome support from ProbeAdmission.
    pub branch_outcome_manifest: EvidenceManifest,
    pub physically_possible_branch_outcomes: Vec<BranchOutcomeKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiOutcomeDecisionView {
    pub outcome_id: String,
    pub unknown: bool,
    pub joint_mass_ppb: u64,
    pub admitted_action_ids: Vec<String>,
    pub selected_action_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiContinuationEvidenceView {
    pub outcome_id: String,
    pub receipt_digest: String,
    pub post_state_set_digest: String,
    pub robust_admitted_action_ids: Vec<String>,
    pub explicit_fallback_action_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeObservationClassView {
    pub outcome_id: String,
    /// The actual delivered observation-equivalence class, not a latent
    /// hypothesis or hidden-state label.
    pub equivalence_class: String,
    /// This digest identifies the entire physical state set for the class.
    /// Multiple outcome IDs in one class must carry the same set digest.
    pub belief_state_set_digest: String,
    /// A valid "Unknown bucket" is Qualified only when it is itself a
    /// delivered, time-qualified outcome. Stale/missing observations are Unknown.
    pub disposition: ObservationDisposition,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePhysicalSuccessorView {
    pub branch_id: String,
    pub outcome_id: String,
    pub state: BeliefStateReceipt,
    pub transition_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoiPolicyBridgeInput {
    pub gate: UpstreamVoiGate,
    pub unknown_outcome_mass_ppb: u64,
    pub context: ProbePolicyContext,
    pub outcome_decisions: Vec<VoiOutcomeDecisionView>,
    pub continuation_evidence: Vec<VoiContinuationEvidenceView>,
    pub observations: Vec<ProbeObservationClassView>,
    pub physical_successors: Vec<ProbePhysicalSuccessorView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiPolicyBridgeError {
    GateNotEvaluated(UpstreamVoiGate),
    EmptyField(&'static str),
    DuplicateIdentifier(&'static str, String),
    DuplicateBranchOutcome(BranchOutcomeKey),
    DuplicateSuccessorState { class_id: String, state_id: String },
    ConflictingSuccessorState { class_id: String, state_id: String },
    OutcomeCoverageMismatch,
    JointMassNotNormalized(u128),
    UnknownMassMismatch { expected: u64, observed: u128 },
    DuplicateAction(String),
    UnknownAction(String),
    OutcomeActionSetMismatch(String),
    SelectedActionNotAdmitted(String),
    UnknownFallbackMismatch(String),
    KnownOutcomeHasFallback(String),
    ObservationClassConflict(String),
    BranchOutcomeSupportMismatch,
    MissingSuccessorForBranchOutcome(BranchOutcomeKey),
    SuccessorOutsideBranchOutcomeSupport(BranchOutcomeKey),
    EmptyPhysicalSupport,
    InvalidMass(u64),
}

impl std::fmt::Display for VoiPolicyBridgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for VoiPolicyBridgeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePolicyPrefix {
    /// Root probe node plus one node per delivered observation-equivalence class.
    /// Outcome nodes intentionally have incomplete downstream transition
    /// manifests until #4293 / other transition owners supply the next layer.
    pub tree: FinitePolicyTree,
    pub outcome_to_belief_node_id: BTreeMap<String, String>,
    /// One-step VOI-local selections, for comparison only.
    pub selected_action_by_outcome: BTreeMap<String, String>,
    /// Whether every outcome-local selected action in a class agrees with the
    /// robust action intersection for that class.
    pub local_selection_is_deployable_by_observation: BTreeMap<String, bool>,
    /// Exact intersection consumed by the child belief node.
    pub deployable_actions_by_observation: BTreeMap<String, Vec<String>>,
    pub unknown_outcome_mass_ppb: u64,
    /// Prefix construction is retrospective and never authorizes actuation.
    pub actuation_authorized: bool,
}

fn require_nonempty(fields: &[(&'static str, &str)]) -> Result<(), VoiPolicyBridgeError> {
    for (name, value) in fields {
        if value.trim().is_empty() {
            return Err(VoiPolicyBridgeError::EmptyField(name));
        }
    }
    Ok(())
}

fn unique_strings<'a>(
    values: impl IntoIterator<Item = &'a String>,
    field: &'static str,
) -> Result<BTreeSet<&'a str>, VoiPolicyBridgeError> {
    let mut output = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            return Err(VoiPolicyBridgeError::EmptyField(field));
        }
        if !output.insert(value.as_str()) {
            return Err(VoiPolicyBridgeError::DuplicateIdentifier(
                field,
                value.clone(),
            ));
        }
    }
    Ok(output)
}

fn observation_group_id(
    binding: &ProbeObservationClassView,
) -> String {
    match binding.disposition {
        ObservationDisposition::Qualified => binding.equivalence_class.clone(),
        ObservationDisposition::Unknown => UNKNOWN_OBSERVATION_CLASS.to_owned(),
        ObservationDisposition::Invalid => INVALID_OBSERVATION_CLASS.to_owned(),
    }
}

fn receipt_state_complete(state: &BeliefStateReceipt) -> bool {
    !state.physical_state_digest.trim().is_empty()
        && !state.reserve_state_digest.trim().is_empty()
        && !state.thermal_state_digest.trim().is_empty()
        && !state.receipt_digest.trim().is_empty()
}

/// Compose one already-evaluated probe report into a policy-tree prefix.
///
/// Expected source mapping:
/// - `gate`, outcome decisions, unknown mass: `ViableVoiReport`;
/// - `probe_action_id`, receipt, branch/outcome support: `ViableVoiProblem.probe`;
/// - continuation receipt/action sets/fallback: `ViableVoiProblem.continuation_evidence`;
/// - observation equivalence and successor state receipts: upstream #4293/#4119 outputs.
///
/// This function never recomputes probability, expected loss, viable VOI,
/// physical branch completeness or robust viability. It checks exact agreement
/// between those independently supplied records and preserves all physical
/// branches, including zero-joint-mass branches.
pub fn build_probe_policy_prefix(
    input: &VoiPolicyBridgeInput,
) -> Result<ProbePolicyPrefix, VoiPolicyBridgeError> {
    if input.gate != UpstreamVoiGate::Evaluated {
        return Err(VoiPolicyBridgeError::GateNotEvaluated(input.gate));
    }
    let context = &input.context;
    require_nonempty(&[
        ("root_node_id", &context.root_node_id),
        ("probe_action_id", &context.probe_action_id),
        ("probe_receipt_digest", &context.probe_receipt_digest),
        (
            "initial_belief_state_set_digest",
            &context.initial_belief_state_set_digest,
        ),
        ("initial_state.state_id", &context.initial_state.state_id),
        ("action_domain.digest", &context.action_domain.digest),
        (
            "branch_outcome_manifest.digest",
            &context.branch_outcome_manifest.digest,
        ),
    ])?;

    let action_ids = unique_strings(&context.action_ids, "action_id")?;
    if !action_ids.contains(context.probe_action_id.as_str()) {
        return Err(VoiPolicyBridgeError::UnknownAction(
            context.probe_action_id.clone(),
        ));
    }
    if context.action_domain.complete && context.action_ids.is_empty() {
        return Err(VoiPolicyBridgeError::EmptyField("action_domain.action_ids"));
    }

    let mut support = BTreeSet::new();
    for pair in &context.physically_possible_branch_outcomes {
        require_nonempty(&[
            ("branch_id", &pair.branch_id),
            ("outcome_id", &pair.outcome_id),
        ])?;
        if !support.insert(pair.clone()) {
            return Err(VoiPolicyBridgeError::DuplicateBranchOutcome(pair.clone()));
        }
    }
    if support.is_empty() {
        return Err(VoiPolicyBridgeError::EmptyPhysicalSupport);
    }

    let mut decisions = BTreeMap::new();
    let mut total_mass = 0_u128;
    let mut unknown_mass = 0_u128;
    let mut selected_action_by_outcome = BTreeMap::new();
    for decision in &input.outcome_decisions {
        require_nonempty(&[
            ("outcome_id", &decision.outcome_id),
            ("selected_action_id", &decision.selected_action_id),
        ])?;
        if decision.joint_mass_ppb > VOI_JOINT_MASS_SCALE_PPB {
            return Err(VoiPolicyBridgeError::InvalidMass(decision.joint_mass_ppb));
        }
        if decisions
            .insert(decision.outcome_id.clone(), decision)
            .is_some()
        {
            return Err(VoiPolicyBridgeError::DuplicateIdentifier(
                "outcome_id",
                decision.outcome_id.clone(),
            ));
        }
        total_mass = total_mass.saturating_add(u128::from(decision.joint_mass_ppb));
        if decision.unknown {
            unknown_mass = unknown_mass.saturating_add(u128::from(decision.joint_mass_ppb));
        }
        selected_action_by_outcome.insert(
            decision.outcome_id.clone(),
            decision.selected_action_id.clone(),
        );
        let admitted = unique_strings(&decision.admitted_action_ids, "admitted_action_id")?;
        for action in &admitted {
            if !action_ids.contains(action) {
                return Err(VoiPolicyBridgeError::UnknownAction((*action).to_owned()));
            }
        }
        if !admitted.contains(decision.selected_action_id.as_str()) {
            return Err(VoiPolicyBridgeError::SelectedActionNotAdmitted(
                decision.outcome_id.clone(),
            ));
        }
    }
    if total_mass != u128::from(VOI_JOINT_MASS_SCALE_PPB) {
        return Err(VoiPolicyBridgeError::JointMassNotNormalized(total_mass));
    }
    if unknown_mass != u128::from(input.unknown_outcome_mass_ppb) {
        return Err(VoiPolicyBridgeError::UnknownMassMismatch {
            expected: input.unknown_outcome_mass_ppb,
            observed: unknown_mass,
        });
    }

    let mut continuations = BTreeMap::new();
    for evidence in &input.continuation_evidence {
        require_nonempty(&[
            ("continuation.outcome_id", &evidence.outcome_id),
            ("continuation.receipt_digest", &evidence.receipt_digest),
            (
                "continuation.post_state_set_digest",
                &evidence.post_state_set_digest,
            ),
        ])?;
        if continuations
            .insert(evidence.outcome_id.clone(), evidence)
            .is_some()
        {
            return Err(VoiPolicyBridgeError::DuplicateIdentifier(
                "continuation.outcome_id",
                evidence.outcome_id.clone(),
            ));
        }
        let decision = decisions
            .get(&evidence.outcome_id)
            .ok_or(VoiPolicyBridgeError::OutcomeCoverageMismatch)?;
        let decision_actions =
            unique_strings(&decision.admitted_action_ids, "decision.admitted_action_id")?;
        let evidence_actions = unique_strings(
            &evidence.robust_admitted_action_ids,
            "continuation.robust_admitted_action_id",
        )?;
        if decision_actions != evidence_actions {
            return Err(VoiPolicyBridgeError::OutcomeActionSetMismatch(
                evidence.outcome_id.clone(),
            ));
        }
        if decision.unknown {
            let fallback = evidence
                .explicit_fallback_action_id
                .as_ref()
                .ok_or_else(|| {
                    VoiPolicyBridgeError::UnknownFallbackMismatch(evidence.outcome_id.clone())
                })?;
            if fallback != &decision.selected_action_id
                || evidence.robust_admitted_action_ids.len() != 1
                || !evidence.robust_admitted_action_ids.contains(fallback)
            {
                return Err(VoiPolicyBridgeError::UnknownFallbackMismatch(
                    evidence.outcome_id.clone(),
                ));
            }
        } else if evidence.explicit_fallback_action_id.is_some() {
            return Err(VoiPolicyBridgeError::KnownOutcomeHasFallback(
                evidence.outcome_id.clone(),
            ));
        }
    }
    if continuations.len() != decisions.len() {
        return Err(VoiPolicyBridgeError::OutcomeCoverageMismatch);
    }

    let mut observations = BTreeMap::new();
    for binding in &input.observations {
        require_nonempty(&[
            ("observation.outcome_id", &binding.outcome_id),
            ("observation.equivalence_class", &binding.equivalence_class),
            (
                "observation.belief_state_set_digest",
                &binding.belief_state_set_digest,
            ),
        ])?;
        if binding.disposition == ObservationDisposition::Qualified
            && binding.receipt_digest.trim().is_empty()
        {
            return Err(VoiPolicyBridgeError::EmptyField(
                "qualified_observation.receipt_digest",
            ));
        }
        if observations
            .insert(binding.outcome_id.clone(), binding)
            .is_some()
        {
            return Err(VoiPolicyBridgeError::DuplicateIdentifier(
                "observation.outcome_id",
                binding.outcome_id.clone(),
            ));
        }
    }
    if observations.len() != decisions.len()
        || decisions.keys().any(|outcome| !observations.contains_key(outcome))
    {
        return Err(VoiPolicyBridgeError::OutcomeCoverageMismatch);
    }
    for (outcome_id, observation) in &observations {
        if continuations[outcome_id].post_state_set_digest
            != observation.belief_state_set_digest
        {
            return Err(VoiPolicyBridgeError::ObservationClassConflict(
                observation.equivalence_class.clone(),
            ));
        }
    }

    let mut grouped_observations: BTreeMap<String, Vec<&ProbeObservationClassView>> =
        BTreeMap::new();
    let mut outcome_to_group = BTreeMap::new();
    for (outcome_id, binding) in &observations {
        let group = observation_group_id(binding);
        outcome_to_group.insert(outcome_id.clone(), group.clone());
        grouped_observations.entry(group).or_default().push(binding);
    }

    // One delivered observation class must have one class-level receipt and one
    // exact post-observation state-set digest. Unknown observations are merged
    // into a single conservative belief rather than split by untrusted labels.
    let mut group_identity = BTreeMap::new();
    let mut group_disposition = BTreeMap::new();
    for (group, bindings) in &grouped_observations {
        let first = bindings[0];
        let identity = (
            first.belief_state_set_digest.as_str(),
            first.receipt_digest.as_str(),
        );
        for binding in bindings.iter().skip(1) {
            if (
                binding.belief_state_set_digest.as_str(),
                binding.receipt_digest.as_str(),
            ) != identity
            {
                return Err(VoiPolicyBridgeError::ObservationClassConflict(
                    group.clone(),
                ));
            }
        }
        let disposition = if bindings
            .iter()
            .any(|b| b.disposition == ObservationDisposition::Invalid)
        {
            ObservationDisposition::Invalid
        } else if bindings
            .iter()
            .any(|b| b.disposition == ObservationDisposition::Unknown)
        {
            ObservationDisposition::Unknown
        } else {
            ObservationDisposition::Qualified
        };
        group_identity.insert(
            group.clone(),
            (
                first.belief_state_set_digest.clone(),
                first.receipt_digest.clone(),
            ),
        );
        group_disposition.insert(group.clone(), disposition);
    }

    let mut observed_support = BTreeSet::new();
    let mut successor_by_group: BTreeMap<
        String,
        BTreeMap<String, BeliefStateReceipt>,
    > = BTreeMap::new();
    let mut transition_by_group: BTreeMap<String, Vec<PolicyTransition>> = BTreeMap::new();
    for successor in &input.physical_successors {
        require_nonempty(&[
            ("successor.branch_id", &successor.branch_id),
            ("successor.outcome_id", &successor.outcome_id),
            ("successor.state.state_id", &successor.state.state_id),
        ])?;
        let pair = BranchOutcomeKey {
            branch_id: successor.branch_id.clone(),
            outcome_id: successor.outcome_id.clone(),
        };
        if !support.contains(&pair) {
            return Err(VoiPolicyBridgeError::SuccessorOutsideBranchOutcomeSupport(
                pair,
            ));
        }
        observed_support.insert(pair.clone());
        let group = outcome_to_group
            .get(&successor.outcome_id)
            .ok_or(VoiPolicyBridgeError::OutcomeCoverageMismatch)?
            .clone();
        let state_map = successor_by_group.entry(group.clone()).or_default();
        if let Some(previous) = state_map.get(&successor.state.state_id) {
            if previous != &successor.state {
                return Err(VoiPolicyBridgeError::ConflictingSuccessorState {
                    class_id: group,
                    state_id: successor.state.state_id.clone(),
                });
            }
        } else {
            state_map.insert(successor.state.state_id.clone(), successor.state.clone());
        }

        let evidence_complete = !successor.transition_receipt_digest.trim().is_empty()
            && receipt_state_complete(&successor.state)
            && receipt_state_complete(&context.initial_state);
        transition_by_group
            .entry(group.clone())
            .or_default()
            .push(PolicyTransition {
                from_state_id: context.initial_state.state_id.clone(),
                action_id: context.probe_action_id.clone(),
                to_state_id: successor.state.state_id.clone(),
                from_state_digest: context.initial_state.physical_state_digest.clone(),
                to_state_digest: successor.state.physical_state_digest.clone(),
                predecessor_reserve_state_digest: context.initial_state.reserve_state_digest.clone(),
                resulting_reserve_state_digest: successor.state.reserve_state_digest.clone(),
                predecessor_thermal_state_digest: context.initial_state.thermal_state_digest.clone(),
                resulting_thermal_state_digest: successor.state.thermal_state_digest.clone(),
                next_belief_node_id: format!("{}/observation/{}", context.root_node_id, group),
                physically_possible: true,
                evidence_complete,
                receipt_digest: successor.transition_receipt_digest.clone(),
            });
    }

    if context.branch_outcome_manifest.complete && observed_support != support {
        return Err(VoiPolicyBridgeError::BranchOutcomeSupportMismatch);
    }
    for pair in &support {
        if context.branch_outcome_manifest.complete && !observed_support.contains(pair) {
            return Err(VoiPolicyBridgeError::MissingSuccessorForBranchOutcome(
                pair.clone(),
            ));
        }
    }

    let mut root = BeliefNode {
        node_id: context.root_node_id.clone(),
        observation_receipt_digest: context.initial_observation_receipt_digest.clone(),
        observation_disposition: if context.initial_observation_receipt_digest.trim().is_empty() {
            ObservationDisposition::Unknown
        } else {
            ObservationDisposition::Qualified
        },
        state_ids: vec![context.initial_state.state_id.clone()],
        belief_state_set_digest: context.initial_belief_state_set_digest.clone(),
        state_evidence: vec![context.initial_state.clone()],
        transition_manifest: context.branch_outcome_manifest.clone(),
        state_action_evidence: vec![StateActionEvidence {
            state_id: context.initial_state.state_id.clone(),
            complete: !context.probe_receipt_digest.trim().is_empty(),
            receipt_digest: context.probe_receipt_digest.clone(),
            supporting_receipt_digests: vec![context.probe_receipt_digest.clone()],
            admitted_action_ids: vec![context.probe_action_id.clone()],
        }],
        transitions: Vec::new(),
        terminal_recovery: None,
    };
    for transitions in transition_by_group.values() {
        root.transitions.extend(transitions.iter().cloned());
    }
    root.transitions.sort_by(|a, b| {
        (
            &a.next_belief_node_id,
            &a.to_state_id,
            &a.receipt_digest,
        )
            .cmp(&(
                &b.next_belief_node_id,
                &b.to_state_id,
                &b.receipt_digest,
            ))
    });

    let mut nodes = vec![root];
    let mut node_ids = BTreeMap::new();
    let mut deployable_actions_by_observation = BTreeMap::new();
    let mut local_selection_is_deployable_by_observation = BTreeMap::new();

    for (group, bindings) in grouped_observations {
        let node_id = format!("{}/observation/{}", context.root_node_id, group);
        let state_map = successor_by_group.get(&group).cloned().unwrap_or_default();
        if state_map.is_empty() {
            return Err(VoiPolicyBridgeError::MissingSuccessorForBranchOutcome(
                support.iter().find(|pair| outcome_to_group.get(&pair.outcome_id) == Some(&group))
                    .cloned()
                    .unwrap_or(BranchOutcomeKey {
                        branch_id: "unknown".to_owned(),
                        outcome_id: group.clone(),
                    }),
            ));
        }
        let mut outcome_ids = bindings
            .iter()
            .map(|binding| binding.outcome_id.clone())
            .collect::<Vec<_>>();
        outcome_ids.sort();

        let mut common_actions: Option<BTreeSet<String>> = None;
        let mut supporting_receipts = Vec::new();
        let mut local_selected_actions = BTreeSet::new();
        for outcome_id in &outcome_ids {
            let evidence = continuations[outcome_id];
            let admitted = evidence
                .robust_admitted_action_ids
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            common_actions = Some(match common_actions {
                None => admitted,
                Some(current) => current.intersection(&admitted).cloned().collect(),
            });
            supporting_receipts.push(evidence.receipt_digest.clone());
            local_selected_actions.insert(decisions[outcome_id].selected_action_id.clone());
        }
        supporting_receipts.sort();
        supporting_receipts.dedup();
        let common_actions = common_actions.unwrap_or_default();
        let common_vec = common_actions.iter().cloned().collect::<Vec<_>>();
        let (belief_set_digest, observation_receipt_digest) = group_identity[&group].clone();
        let disposition = group_disposition[&group];

        let local_selection_deployable = local_selected_actions.len() == 1
            && local_selected_actions
                .iter()
                .next()
                .is_some_and(|selected| common_actions.contains(selected));
        local_selection_is_deployable_by_observation
            .insert(group.clone(), local_selection_deployable);
        deployable_actions_by_observation.insert(group.clone(), common_vec.clone());

        let state_ids = state_map.keys().cloned().collect::<Vec<_>>();
        let state_evidence = state_map.values().cloned().collect::<Vec<_>>();
        let state_action_evidence = state_ids
            .iter()
            .map(|state_id| StateActionEvidence {
                state_id: state_id.clone(),
                complete: supporting_receipts.iter().all(|value| !value.trim().is_empty()),
                receipt_digest: observation_receipt_digest.clone(),
                supporting_receipt_digests: supporting_receipts.clone(),
                admitted_action_ids: common_vec.clone(),
            })
            .collect::<Vec<_>>();

        for outcome_id in outcome_ids {
            node_ids.insert(outcome_id, node_id.clone());
        }

        nodes.push(BeliefNode {
            node_id,
            observation_receipt_digest,
            observation_disposition: disposition,
            state_ids,
            belief_state_set_digest: belief_set_digest,
            state_evidence,
            // Downstream physical transitions have not been supplied yet.
            // This is explicitly incomplete, so a policy search cannot claim a
            // multi-step win based only on the one-step VOI receipt.
            transition_manifest: EvidenceManifest {
                complete: false,
                digest: String::new(),
            },
            state_action_evidence,
            transitions: Vec::new(),
            terminal_recovery: None,
        });
    }

    Ok(ProbePolicyPrefix {
        tree: FinitePolicyTree {
            root_node_id: context.root_node_id.clone(),
            action_domain: context.action_domain.clone(),
            action_ids: context.action_ids.clone(),
            nodes,
        },
        outcome_to_belief_node_id: node_ids,
        selected_action_by_outcome,
        local_selection_is_deployable_by_observation,
        deployable_actions_by_observation,
        unknown_outcome_mass_ppb: input.unknown_outcome_mass_ppb,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(id: &str) -> BeliefStateReceipt {
        BeliefStateReceipt {
            state_id: id.to_owned(),
            physical_state_digest: format!("physical:{id}"),
            reserve_state_digest: format!("reserve:{id}"),
            thermal_state_digest: format!("thermal:{id}"),
            receipt_digest: format!("state-receipt:{id}"),
        }
    }

    fn input() -> VoiPolicyBridgeInput {
        let branch_outcomes = vec![
            BranchOutcomeKey { branch_id: "b1".into(), outcome_id: "rough-a".into() },
            BranchOutcomeKey { branch_id: "b2".into(), outcome_id: "rough-b".into() },
            BranchOutcomeKey { branch_id: "b3".into(), outcome_id: "unknown".into() },
        ];
        let decisions = vec![
            VoiOutcomeDecisionView {
                outcome_id: "rough-a".into(),
                unknown: false,
                joint_mass_ppb: 600_000_000,
                admitted_action_ids: vec!["continue".into(), "retreat".into()],
                selected_action_id: "continue".into(),
            },
            VoiOutcomeDecisionView {
                outcome_id: "rough-b".into(),
                unknown: false,
                joint_mass_ppb: 300_000_000,
                admitted_action_ids: vec!["retreat".into()],
                selected_action_id: "retreat".into(),
            },
            VoiOutcomeDecisionView {
                outcome_id: "unknown".into(),
                unknown: true,
                joint_mass_ppb: 100_000_000,
                admitted_action_ids: vec!["fallback".into()],
                selected_action_id: "fallback".into(),
            },
        ];
        let continuations = vec![
            VoiContinuationEvidenceView {
                outcome_id: "rough-a".into(),
                receipt_digest: "continuation:rough-a".into(),
                post_state_set_digest: "state-set:rough".into(),
                robust_admitted_action_ids: vec!["retreat".into(), "continue".into()],
                explicit_fallback_action_id: None,
            },
            VoiContinuationEvidenceView {
                outcome_id: "rough-b".into(),
                receipt_digest: "continuation:rough-b".into(),
                post_state_set_digest: "state-set:rough".into(),
                robust_admitted_action_ids: vec!["retreat".into()],
                explicit_fallback_action_id: None,
            },
            VoiContinuationEvidenceView {
                outcome_id: "unknown".into(),
                receipt_digest: "continuation:unknown".into(),
                post_state_set_digest: "state-set:unknown".into(),
                robust_admitted_action_ids: vec!["fallback".into()],
                explicit_fallback_action_id: Some("fallback".into()),
            },
        ];
        let observations = vec![
            ProbeObservationClassView {
                outcome_id: "rough-a".into(),
                equivalence_class: "rough".into(),
                belief_state_set_digest: "state-set:rough".into(),
                disposition: ObservationDisposition::Qualified,
                receipt_digest: "observation:rough".into(),
            },
            ProbeObservationClassView {
                outcome_id: "rough-b".into(),
                equivalence_class: "rough".into(),
                belief_state_set_digest: "state-set:rough".into(),
                disposition: ObservationDisposition::Qualified,
                receipt_digest: "observation:rough".into(),
            },
            ProbeObservationClassView {
                outcome_id: "unknown".into(),
                equivalence_class: "unknown-sensor-label".into(),
                belief_state_set_digest: "state-set:unknown".into(),
                disposition: ObservationDisposition::Qualified,
                receipt_digest: "observation:unknown".into(),
            },
        ];
        let physical_successors = vec![
            ProbePhysicalSuccessorView {
                branch_id: "b1".into(),
                outcome_id: "rough-a".into(),
                state: state("s1"),
                transition_receipt_digest: "probe-transition:s1".into(),
            },
            ProbePhysicalSuccessorView {
                branch_id: "b2".into(),
                outcome_id: "rough-b".into(),
                state: state("s2"),
                transition_receipt_digest: "probe-transition:s2".into(),
            },
            ProbePhysicalSuccessorView {
                branch_id: "b3".into(),
                outcome_id: "unknown".into(),
                state: state("s3"),
                transition_receipt_digest: "probe-transition:s3".into(),
            },
        ];
        VoiPolicyBridgeInput {
            gate: UpstreamVoiGate::Evaluated,
            unknown_outcome_mass_ppb: 100_000_000,
            context: ProbePolicyContext {
                root_node_id: "mission-root".into(),
                probe_action_id: "probe".into(),
                probe_receipt_digest: "probe-gate-receipt".into(),
                initial_observation_receipt_digest: "observation:initial".into(),
                initial_belief_state_set_digest: "state-set:initial".into(),
                initial_state: state("initial"),
                action_domain: EvidenceManifest {
                    complete: true,
                    digest: "actions".into(),
                },
                action_ids: vec![
                    "probe".into(),
                    "continue".into(),
                    "retreat".into(),
                    "fallback".into(),
                ],
                branch_outcome_manifest: EvidenceManifest {
                    complete: true,
                    digest: "branch-outcomes".into(),
                },
                physically_possible_branch_outcomes: branch_outcomes,
            },
            outcome_decisions: decisions,
            continuation_evidence: continuations,
            observations,
            physical_successors,
        }
    }

    #[test]
    fn indistinguishable_voi_outcomes_intersect_action_sets_before_policy_choice() {
        let prefix = build_probe_policy_prefix(&input()).unwrap();
        assert_eq!(
            prefix.deployable_actions_by_observation["rough"],
            vec!["retreat".to_owned()]
        );
        assert!(!prefix.local_selection_is_deployable_by_observation["rough"]);
        assert_eq!(
            prefix.deployable_actions_by_observation["unknown-sensor-label"],
            vec!["fallback".to_owned()]
        );
        assert!(prefix.local_selection_is_deployable_by_observation["unknown-sensor-label"]);
        assert!(!prefix.actuation_authorized);
    }

    #[test]
    fn zero_mass_but_physically_possible_branch_is_preserved_in_the_prefix() {
        let mut source = input();
        source.outcome_decisions[2].joint_mass_ppb = 0;
        source.outcome_decisions[0].joint_mass_ppb = 700_000_000;
        source.outcome_decisions[1].joint_mass_ppb = 300_000_000;
        source.unknown_outcome_mass_ppb = 0;
        let prefix = build_probe_policy_prefix(&source).unwrap();
        assert!(prefix.tree.nodes[0]
            .transitions
            .iter()
            .any(|edge| edge.to_state_id == "s3"));
        assert!(prefix.tree.nodes.iter().any(|node| node.state_ids.contains(&"s3".to_owned())));
    }

    #[test]
    fn physical_rejection_unknown_or_resource_rejection_never_builds_a_policy() {
        for gate in [
            UpstreamVoiGate::PhysicalRejected,
            UpstreamVoiGate::PhysicalUnknown,
            UpstreamVoiGate::ResourceRejected,
        ] {
            let mut source = input();
            source.gate = gate;
            assert!(matches!(
                build_probe_policy_prefix(&source),
                Err(VoiPolicyBridgeError::GateNotEvaluated(_))
            ));
        }
    }

    #[test]
    fn local_action_set_must_match_its_upstream_continuation_receipt() {
        let mut source = input();
        source.continuation_evidence[0]
            .robust_admitted_action_ids
            .push("fallback".into());
        assert!(matches!(
            build_probe_policy_prefix(&source),
            Err(VoiPolicyBridgeError::OutcomeActionSetMismatch(_))
        ));
    }

    #[test]
    fn unknown_outcome_requires_one_selected_explicit_fallback() {
        let mut source = input();
        source.continuation_evidence[2].robust_admitted_action_ids =
            vec!["fallback".into(), "retreat".into()];
        source.outcome_decisions[2].admitted_action_ids =
            vec!["fallback".into(), "retreat".into()];
        assert!(matches!(
            build_probe_policy_prefix(&source),
            Err(VoiPolicyBridgeError::UnknownFallbackMismatch(_))
        ));
    }

    #[test]
    fn unknown_mass_must_equal_the_sum_of_unknown_outcome_masses() {
        let mut source = input();
        source.unknown_outcome_mass_ppb = 0;
        assert!(matches!(
            build_probe_policy_prefix(&source),
            Err(VoiPolicyBridgeError::UnknownMassMismatch { .. })
        ));
    }

    #[test]
    fn stale_observation_is_preserved_as_unknown_not_used_to_split() {
        let mut source = input();
        source.observations[0].disposition = ObservationDisposition::Unknown;
        source.observations[1].disposition = ObservationDisposition::Unknown;
        source.observations[0].equivalence_class = "stale-a".into();
        source.observations[1].equivalence_class = "stale-b".into();
        source.observations[0].belief_state_set_digest = "state-set:unknown-observation".into();
        source.observations[1].belief_state_set_digest = "state-set:unknown-observation".into();
        source.observations[0].receipt_digest = String::new();
        source.observations[1].receipt_digest = String::new();
        let prefix = build_probe_policy_prefix(&source).unwrap();
        let unknown = prefix.tree.nodes.iter().find(|node| node.node_id.ends_with(UNKNOWN_OBSERVATION_CLASS)).unwrap();
        assert_eq!(unknown.observation_disposition, ObservationDisposition::Unknown);
        assert_eq!(unknown.state_ids.len(), 2);
    }

    #[test]
    fn invalid_observation_is_not_reported_as_a_qualified_policy_prefix() {
        let mut source = input();
        source.observations[0].disposition = ObservationDisposition::Invalid;
        source.observations[1].disposition = ObservationDisposition::Invalid;
        source.observations[0].equivalence_class = "invalid-a".into();
        source.observations[1].equivalence_class = "invalid-b".into();
        source.observations[0].belief_state_set_digest = "state-set:invalid".into();
        source.observations[1].belief_state_set_digest = "state-set:invalid".into();
        source.observations[0].receipt_digest = String::new();
        source.observations[1].receipt_digest = String::new();
        let prefix = build_probe_policy_prefix(&source).unwrap();
        let invalid = prefix.tree.nodes.iter().find(|node| node.node_id.ends_with(INVALID_OBSERVATION_CLASS)).unwrap();
        assert_eq!(invalid.observation_disposition, ObservationDisposition::Invalid);
    }

    #[test]
    fn branch_outcome_support_must_be_complete() {
        let mut source = input();
        source.physical_successors.pop();
        assert!(matches!(
            build_probe_policy_prefix(&source),
            Err(VoiPolicyBridgeError::BranchOutcomeSupportMismatch)
                | Err(VoiPolicyBridgeError::MissingSuccessorForBranchOutcome(_))
        ));
    }

    #[test]
    fn same_observation_class_requires_one_exact_state_set_and_receipt_identity() {
        let mut source = input();
        source.observations[1].belief_state_set_digest = "different-set".into();
        assert!(matches!(
            build_probe_policy_prefix(&source),
            Err(VoiPolicyBridgeError::ObservationClassConflict(_))
        ));
    }

    #[test]
    fn downstream_policy_tree_remains_unknown_until_physical_transition_layer_arrives() {
        let prefix = build_probe_policy_prefix(&input()).unwrap();
        let report = super::super::policy_tree::evaluate_bounded_policy_tree(&prefix.tree, 2)
            .unwrap();
        assert_eq!(
            report.disposition,
            super::super::policy_tree::PolicyTreeDisposition::Unknown
        );
    }

    #[test]
    fn missing_probe_transition_receipt_does_not_make_a_positive_prefix() {
        let mut source = input();
        source.physical_successors[0].transition_receipt_digest.clear();
        let prefix = build_probe_policy_prefix(&source).unwrap();
        let report = super::super::policy_tree::evaluate_bounded_policy_tree(&prefix.tree, 2)
            .unwrap();
        assert_eq!(
            report.disposition,
            super::super::policy_tree::PolicyTreeDisposition::Unknown
        );
    }
}
