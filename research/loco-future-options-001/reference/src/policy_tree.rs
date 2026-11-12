//! Bounded finite policy-tree synthesis under partial observability.
//!
//! A policy node represents every hidden state consistent with one delivered
//! observation history. One action must be admitted for every state in that
//! belief. Every physically possible successor is then grouped by its *next*
//! delivered observation node. At the leaves, a qualified terminal
//! safe-stop/recovery receipt is required.
//!
//! This is a finite AND/OR policy evaluator, not a dynamics model, a continuous
//! viability proof, a controller, or an actuation authority.

use std::collections::{BTreeMap, BTreeSet};

use super::EvidenceManifest;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefStateReceipt {
    pub state_id: String,
    pub physical_state_digest: String,
    pub reserve_state_digest: String,
    pub thermal_state_digest: String,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateActionEvidence {
    pub state_id: String,
    /// Complete status over the frozen action domain. Omitted actions are not
    /// admitted only when this flag and the receipt digest are qualified.
    pub complete: bool,
    pub receipt_digest: String,
    pub admitted_action_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyTransition {
    pub from_state_id: String,
    pub action_id: String,
    pub to_state_id: String,
    pub from_state_digest: String,
    pub to_state_digest: String,
    pub predecessor_reserve_state_digest: String,
    pub resulting_reserve_state_digest: String,
    pub predecessor_thermal_state_digest: String,
    pub resulting_thermal_state_digest: String,
    /// Node for the delivered next observation-equivalence class.
    pub next_belief_node_id: String,
    pub physically_possible: bool,
    pub evidence_complete: bool,
    pub receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalRecoveryReceipt {
    pub state_id: String,
    pub reserve_state_digest: String,
    pub topology_digest: String,
    pub terminal_safe_stop_reachable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalRecoveryEvidence {
    Qualified {
        state_receipts: Vec<TerminalRecoveryReceipt>,
    },
    Unknown {
        reason: String,
    },
    Invalid {
        reason: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationDisposition {
    Qualified,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeliefNode {
    pub node_id: String,
    /// Receipt that justifies this delivered observation-equivalence class.
    pub observation_receipt_digest: String,
    pub observation_disposition: ObservationDisposition,
    /// All hidden states that produce this observation history.
    pub state_ids: Vec<String>,
    pub state_evidence: Vec<BeliefStateReceipt>,
    pub transition_manifest: EvidenceManifest,
    pub state_action_evidence: Vec<StateActionEvidence>,
    pub transitions: Vec<PolicyTransition>,
    /// Required only at horizon-zero leaves.
    pub terminal_recovery: Option<TerminalRecoveryEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FinitePolicyTree {
    pub root_node_id: String,
    pub action_domain: EvidenceManifest,
    pub action_ids: Vec<String>,
    pub nodes: Vec<BeliefNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyTreeDisposition {
    WinningPolicyFound,
    NoWinningPolicyInDeclaredDomain,
    Unknown,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyTreeError {
    EmptyField(&'static str),
    DuplicateActionId(String),
    DuplicateNodeId(String),
    DuplicateStateId { node_id: String, state_id: String },
    DuplicateStateEvidence { node_id: String, state_id: String },
    DuplicateTerminalRecoveryState(String),
    UnknownActionId(String),
    MissingRootNode(String),
    InvalidHorizon(usize),
    CyclicPolicyGraph(String),
}

impl std::fmt::Display for PolicyTreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for PolicyTreeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyTreeReport {
    pub disposition: PolicyTreeDisposition,
    /// Every action that has a complete winning continuation at the root.
    pub root_winning_action_ids: Vec<String>,
    /// One deterministic witness policy: observation-history node -> action.
    pub policy_witness: BTreeMap<String, String>,
    pub unknown_reasons: Vec<String>,
    pub invalid_reasons: Vec<String>,
    pub explored_belief_nodes: usize,
    pub horizon: usize,
    pub actuation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NodeSolution {
    disposition: PolicyTreeDisposition,
    winning_action_ids: Vec<String>,
    witness: BTreeMap<String, String>,
    unknown_reasons: Vec<String>,
    invalid_reasons: Vec<String>,
}

fn require_nonempty(fields: &[(&'static str, &str)]) -> Result<(), PolicyTreeError> {
    for (name, value) in fields {
        if value.trim().is_empty() {
            return Err(PolicyTreeError::EmptyField(name));
        }
    }
    Ok(())
}

fn index_state_receipts(
    node: &BeliefNode,
) -> Result<BTreeMap<String, &BeliefStateReceipt>, PolicyTreeError> {
    let mut indexed = BTreeMap::new();
    for receipt in &node.state_evidence {
        require_nonempty(&[("belief_state_receipt.state_id", &receipt.state_id)])?;
        if indexed.insert(receipt.state_id.clone(), receipt).is_some() {
            return Err(PolicyTreeError::DuplicateStateEvidence {
                node_id: node.node_id.clone(),
                state_id: receipt.state_id.clone(),
            });
        }
    }
    Ok(indexed)
}

fn state_receipts_complete(
    node: &BeliefNode,
    receipts: &BTreeMap<String, &BeliefStateReceipt>,
) -> bool {
    receipts.len() == node.state_ids.len()
        && node.state_ids.iter().all(|state_id| {
            receipts.get(state_id).is_some_and(|receipt| {
                !receipt.physical_state_digest.trim().is_empty()
                    && !receipt.reserve_state_digest.trim().is_empty()
                    && !receipt.thermal_state_digest.trim().is_empty()
                    && !receipt.receipt_digest.trim().is_empty()
            })
        })
}

fn solve_node(
    node_id: &str,
    depth: usize,
    nodes: &BTreeMap<String, &BeliefNode>,
    action_domain: &BTreeSet<String>,
    active: &mut BTreeSet<(String, usize)>,
    memo: &mut BTreeMap<(String, usize), NodeSolution>,
    explored: &mut BTreeSet<String>,
) -> Result<NodeSolution, PolicyTreeError> {
    let key = (node_id.to_owned(), depth);
    if let Some(found) = memo.get(&key) {
        return Ok(found.clone());
    }
    if !active.insert(key.clone()) {
        return Err(PolicyTreeError::CyclicPolicyGraph(node_id.to_owned()));
    }
    explored.insert(node_id.to_owned());

    let node = nodes
        .get(node_id)
        .copied()
        .ok_or_else(|| PolicyTreeError::MissingRootNode(node_id.to_owned()))?;

    let mut all_state_ids = BTreeSet::new();
    for state_id in &node.state_ids {
        require_nonempty(&[("state_id", state_id)])?;
        if !all_state_ids.insert(state_id.as_str()) {
            return Err(PolicyTreeError::DuplicateStateId {
                node_id: node_id.to_owned(),
                state_id: state_id.clone(),
            });
        }
    }

    let state_receipts = index_state_receipts(node)?;
    let state_receipts_valid = state_receipts_complete(node, &state_receipts);

    let solution = if node.observation_disposition == ObservationDisposition::Invalid {
        NodeSolution {
            disposition: PolicyTreeDisposition::Invalid,
            winning_action_ids: Vec::new(),
            witness: BTreeMap::new(),
            unknown_reasons: Vec::new(),
            invalid_reasons: vec![format!("{node_id}:observation-receipt-invalid")],
        }
    } else if node.observation_disposition == ObservationDisposition::Unknown
        || node.observation_receipt_digest.trim().is_empty()
    {
        NodeSolution {
            disposition: PolicyTreeDisposition::Unknown,
            winning_action_ids: Vec::new(),
            witness: BTreeMap::new(),
            unknown_reasons: vec![format!("{node_id}:observation-receipt-unknown-or-missing")],
            invalid_reasons: Vec::new(),
        }
    } else if node.state_ids.is_empty() {
        NodeSolution {
            disposition: PolicyTreeDisposition::Unknown,
            winning_action_ids: Vec::new(),
            witness: BTreeMap::new(),
            unknown_reasons: vec![format!("{node_id}:empty-belief-state")],
            invalid_reasons: Vec::new(),
        }
    } else if !state_receipts_valid {
        NodeSolution {
            disposition: PolicyTreeDisposition::Unknown,
            winning_action_ids: Vec::new(),
            witness: BTreeMap::new(),
            unknown_reasons: vec![format!(
                "{node_id}:physical-reserve-or-thermal-state-evidence-incomplete"
            )],
            invalid_reasons: Vec::new(),
        }
    } else if depth == 0 {
        match &node.terminal_recovery {
            Some(TerminalRecoveryEvidence::Qualified {
                state_receipts: recovery_state_receipts,
            }) => {
                let mut by_state = BTreeMap::new();
                let mut duplicate = None;
                for receipt in recovery_state_receipts {
                    if by_state.insert(receipt.state_id.clone(), receipt).is_some() {
                        duplicate = Some(receipt.state_id.clone());
                        break;
                    }
                }
                if let Some(state_id) = duplicate {
                    return Err(PolicyTreeError::DuplicateTerminalRecoveryState(state_id));
                }
                if by_state.len() != node.state_ids.len()
                    || node
                        .state_ids
                        .iter()
                        .any(|state_id| !by_state.contains_key(state_id))
                {
                    NodeSolution {
                        disposition: PolicyTreeDisposition::Unknown,
                        winning_action_ids: Vec::new(),
                        witness: BTreeMap::new(),
                        unknown_reasons: vec![format!(
                            "{node_id}:terminal-recovery-state-coverage-incomplete"
                        )],
                        invalid_reasons: Vec::new(),
                    }
                } else {
                    let mut mismatch = None;
                    let mut unknown = None;
                    let mut safe_stop_missing = false;
                    for state_id in &node.state_ids {
                        let receipt = by_state[state_id];
                        if receipt.reserve_state_digest.trim().is_empty()
                            || receipt.topology_digest.trim().is_empty()
                        {
                            unknown = Some(format!(
                                "{node_id}:{state_id}:terminal-recovery-identity-missing"
                            ));
                            break;
                        }
                        let state = state_receipts[state_id];
                        if receipt.reserve_state_digest != state.reserve_state_digest {
                            mismatch = Some(format!(
                                "{node_id}:{state_id}:terminal-reserve-state-mismatch"
                            ));
                            break;
                        }
                        if !receipt.terminal_safe_stop_reachable {
                            safe_stop_missing = true;
                        }
                    }
                    if let Some(reason) = mismatch {
                        NodeSolution {
                            disposition: PolicyTreeDisposition::Invalid,
                            winning_action_ids: Vec::new(),
                            witness: BTreeMap::new(),
                            unknown_reasons: Vec::new(),
                            invalid_reasons: vec![reason],
                        }
                    } else if let Some(reason) = unknown {
                        NodeSolution {
                            disposition: PolicyTreeDisposition::Unknown,
                            winning_action_ids: Vec::new(),
                            witness: BTreeMap::new(),
                            unknown_reasons: vec![reason],
                            invalid_reasons: Vec::new(),
                        }
                    } else if safe_stop_missing {
                        NodeSolution {
                            disposition: PolicyTreeDisposition::NoWinningPolicyInDeclaredDomain,
                            winning_action_ids: Vec::new(),
                            witness: BTreeMap::new(),
                            unknown_reasons: Vec::new(),
                            invalid_reasons: Vec::new(),
                        }
                    } else {
                        NodeSolution {
                            disposition: PolicyTreeDisposition::WinningPolicyFound,
                            winning_action_ids: Vec::new(),
                            witness: BTreeMap::new(),
                            unknown_reasons: Vec::new(),
                            invalid_reasons: Vec::new(),
                        }
                    }
                }
            }
            Some(TerminalRecoveryEvidence::Unknown { reason }) => NodeSolution {
                disposition: PolicyTreeDisposition::Unknown,
                winning_action_ids: Vec::new(),
                witness: BTreeMap::new(),
                unknown_reasons: vec![format!("{node_id}:terminal:{reason}")],
                invalid_reasons: Vec::new(),
            },
            Some(TerminalRecoveryEvidence::Invalid { reason }) => NodeSolution {
                disposition: PolicyTreeDisposition::Invalid,
                winning_action_ids: Vec::new(),
                witness: BTreeMap::new(),
                unknown_reasons: Vec::new(),
                invalid_reasons: vec![format!("{node_id}:terminal:{reason}")],
            },
            None => NodeSolution {
                disposition: PolicyTreeDisposition::Unknown,
                winning_action_ids: Vec::new(),
                witness: BTreeMap::new(),
                unknown_reasons: vec![format!("{node_id}:terminal-recovery-receipt-missing")],
                invalid_reasons: Vec::new(),
            },
        }
    } else if !node.transition_manifest.complete
        || node.transition_manifest.digest.trim().is_empty()
    {
        NodeSolution {
            disposition: PolicyTreeDisposition::Unknown,
            winning_action_ids: Vec::new(),
            witness: BTreeMap::new(),
            unknown_reasons: vec![format!("{node_id}:transition-manifest-incomplete")],
            invalid_reasons: Vec::new(),
        }
    } else {
        let mut evidence_by_state = BTreeMap::new();
        for evidence in &node.state_action_evidence {
            require_nonempty(&[
                ("state_action.state_id", &evidence.state_id),
                ("state_action.receipt_digest", &evidence.receipt_digest),
            ])?;
            if evidence_by_state
                .insert(evidence.state_id.as_str(), evidence)
                .is_some()
            {
                return Err(PolicyTreeError::DuplicateStateEvidence {
                    node_id: node_id.to_owned(),
                    state_id: evidence.state_id.clone(),
                });
            }
            for action_id in &evidence.admitted_action_ids {
                if !action_domain.contains(action_id) {
                    return Err(PolicyTreeError::UnknownActionId(action_id.clone()));
                }
            }
        }

        let evidence_complete = node.state_ids.iter().all(|state_id| {
            evidence_by_state
                .get(state_id.as_str())
                .is_some_and(|e| e.complete && !e.receipt_digest.trim().is_empty())
        });
        if !evidence_complete || evidence_by_state.len() != node.state_ids.len() {
            NodeSolution {
                disposition: PolicyTreeDisposition::Unknown,
                winning_action_ids: Vec::new(),
                witness: BTreeMap::new(),
                unknown_reasons: vec![format!("{node_id}:state-action-evidence-incomplete")],
                invalid_reasons: Vec::new(),
            }
        } else {
            // One action must be admitted in every hidden state consistent with
            // the current delivered observation history.
            let mut candidate_actions: BTreeSet<String> = action_domain.clone();
            for state_id in &node.state_ids {
                let admitted: BTreeSet<String> = evidence_by_state[state_id.as_str()]
                    .admitted_action_ids
                    .iter()
                    .cloned()
                    .collect();
                candidate_actions.retain(|action| admitted.contains(action));
            }

            let mut winning_actions = Vec::new();
            let mut chosen_witness = BTreeMap::new();
            let mut any_unknown = false;
            let mut any_invalid = false;
            let mut unknown_reasons = Vec::new();
            let mut invalid_reasons = Vec::new();

            for action_id in candidate_actions {
                let mut children: BTreeMap<String, BTreeMap<String, (String, String, String)>> =
                    BTreeMap::new();
                let mut action_unknown = false;
                let mut action_invalid = false;
                let mut action_no_policy = false;

                for state_id in &node.state_ids {
                    let state_edges: Vec<&PolicyTransition> = node
                        .transitions
                        .iter()
                        .filter(|edge| {
                            edge.from_state_id == *state_id
                                && edge.action_id == action_id
                                && edge.physically_possible
                        })
                        .collect();
                    if state_edges.is_empty() {
                        action_unknown = true;
                        unknown_reasons.push(format!(
                            "{node_id}:{action_id}:{state_id}:successor-evidence-missing"
                        ));
                        continue;
                    }
                    for edge in state_edges {
                        if edge.to_state_id.trim().is_empty()
                            || edge.next_belief_node_id.trim().is_empty()
                            || edge.from_state_digest.trim().is_empty()
                            || edge.to_state_digest.trim().is_empty()
                            || edge.predecessor_reserve_state_digest.trim().is_empty()
                            || edge.resulting_reserve_state_digest.trim().is_empty()
                            || edge.predecessor_thermal_state_digest.trim().is_empty()
                            || edge.resulting_thermal_state_digest.trim().is_empty()
                            || !edge.evidence_complete
                            || edge.receipt_digest.trim().is_empty()
                        {
                            action_unknown = true;
                            unknown_reasons.push(format!(
                                "{node_id}:{action_id}:{state_id}:transition-receipt-incomplete"
                            ));
                            continue;
                        }
                        let parent_state = state_receipts[state_id];
                        if edge.from_state_digest != parent_state.physical_state_digest
                            || edge.predecessor_reserve_state_digest
                                != parent_state.reserve_state_digest
                            || edge.predecessor_thermal_state_digest
                                != parent_state.thermal_state_digest
                        {
                            action_invalid = true;
                            invalid_reasons.push(format!(
                                "{node_id}:{action_id}:{state_id}:transition-predecessor-binding-mismatch"
                            ));
                            continue;
                        }
                        let child_states = children
                            .entry(edge.next_belief_node_id.clone())
                            .or_default();
                        let resulting_identity = (
                            edge.to_state_digest.clone(),
                            edge.resulting_reserve_state_digest.clone(),
                            edge.resulting_thermal_state_digest.clone(),
                        );
                        if let Some(previous) = child_states
                            .insert(edge.to_state_id.clone(), resulting_identity.clone())
                            && previous != resulting_identity
                        {
                            action_invalid = true;
                            invalid_reasons.push(format!(
                                "{node_id}:{action_id}:conflicting-successor-state-receipts:{}",
                                edge.to_state_id
                            ));
                        }
                    }
                }

                if action_invalid {
                    any_invalid = true;
                    continue;
                }
                if action_unknown {
                    any_unknown = true;
                    continue;
                }

                let mut merged_witness = BTreeMap::new();
                for (child_id, reached_state_receipts) in &children {
                    let Some(child) = nodes.get(child_id) else {
                        action_unknown = true;
                        unknown_reasons.push(format!(
                            "{node_id}:{action_id}:missing-child-node:{child_id}"
                        ));
                        break;
                    };
                    let reached_states: BTreeSet<String> =
                        reached_state_receipts.keys().cloned().collect();
                    let child_states: BTreeSet<String> = child.state_ids.iter().cloned().collect();
                    if child_states != reached_states {
                        action_unknown = true;
                        unknown_reasons.push(format!(
                            "{node_id}:{action_id}:observation-partition-mismatch:{child_id}"
                        ));
                        break;
                    }
                    let child_state_receipts = index_state_receipts(child)?;
                    if !state_receipts_complete(child, &child_state_receipts) {
                        action_unknown = true;
                        unknown_reasons.push(format!(
                            "{node_id}:{action_id}:child-state-evidence-incomplete:{child_id}"
                        ));
                        break;
                    }
                    for (state_id, (physical_digest, reserve_digest, thermal_digest)) in
                        reached_state_receipts
                    {
                        let child_receipt = child_state_receipts[state_id];
                        if *physical_digest != child_receipt.physical_state_digest
                            || *reserve_digest != child_receipt.reserve_state_digest
                            || *thermal_digest != child_receipt.thermal_state_digest
                        {
                            action_invalid = true;
                            invalid_reasons.push(format!(
                                "{node_id}:{action_id}:successor-state-binding-mismatch:{child_id}:{state_id}"
                            ));
                            break;
                        }
                    }
                    if action_invalid {
                        break;
                    }
                    let child_solution = solve_node(
                        child_id,
                        depth - 1,
                        nodes,
                        action_domain,
                        active,
                        memo,
                        explored,
                    )?;
                    match child_solution.disposition {
                        PolicyTreeDisposition::WinningPolicyFound => {
                            merged_witness.extend(child_solution.witness);
                        }
                        PolicyTreeDisposition::Unknown => {
                            action_unknown = true;
                            unknown_reasons.extend(child_solution.unknown_reasons);
                        }
                        PolicyTreeDisposition::Invalid => {
                            action_invalid = true;
                            invalid_reasons.extend(child_solution.invalid_reasons);
                        }
                        PolicyTreeDisposition::NoWinningPolicyInDeclaredDomain => {
                            // AND semantics: one known losing successor makes the
                            // parent action non-winning, even if other children win.
                            action_no_policy = true;
                        }
                    }
                    if action_unknown || action_invalid {
                        break;
                    }
                }

                if action_unknown {
                    any_unknown = true;
                    continue;
                }
                if action_invalid {
                    any_invalid = true;
                    continue;
                }
                if action_no_policy {
                    continue;
                }

                winning_actions.push(action_id.clone());
                // Iteration is canonical BTreeSet order; the first complete
                // witness is deterministic even if several actions can win.
                if chosen_witness.is_empty() {
                    merged_witness.insert(node_id.to_owned(), action_id.clone());
                    chosen_witness = merged_witness;
                }
            }

            if !winning_actions.is_empty() {
                NodeSolution {
                    disposition: PolicyTreeDisposition::WinningPolicyFound,
                    winning_action_ids: winning_actions,
                    witness: chosen_witness,
                    unknown_reasons,
                    invalid_reasons,
                }
            } else if any_unknown {
                NodeSolution {
                    disposition: PolicyTreeDisposition::Unknown,
                    winning_action_ids: Vec::new(),
                    witness: BTreeMap::new(),
                    unknown_reasons,
                    invalid_reasons,
                }
            } else if any_invalid {
                NodeSolution {
                    disposition: PolicyTreeDisposition::Invalid,
                    winning_action_ids: Vec::new(),
                    witness: BTreeMap::new(),
                    unknown_reasons,
                    invalid_reasons,
                }
            } else {
                NodeSolution {
                    disposition: PolicyTreeDisposition::NoWinningPolicyInDeclaredDomain,
                    winning_action_ids: Vec::new(),
                    witness: BTreeMap::new(),
                    unknown_reasons,
                    invalid_reasons,
                }
            }
        }
    };

    active.remove(&key);
    memo.insert(key, solution.clone());
    Ok(solution)
}

/// Synthesize a policy over a finite observation-belief tree with horizon 1–3.
///
/// Each non-leaf node must select one action admitted for every hidden state in
/// that belief. Every physically possible successor must map to the exact state
/// set of its next-observation node. Every horizon-zero leaf must carry a
/// qualified terminal recovery/safe-stop receipt.
pub fn evaluate_bounded_policy_tree(
    tree: &FinitePolicyTree,
    horizon: usize,
) -> Result<PolicyTreeReport, PolicyTreeError> {
    if !(1..=3).contains(&horizon) {
        return Err(PolicyTreeError::InvalidHorizon(horizon));
    }
    require_nonempty(&[
        ("root_node_id", &tree.root_node_id),
        ("action_domain.digest", &tree.action_domain.digest),
    ])?;
    if tree.action_ids.is_empty() {
        return Err(PolicyTreeError::EmptyField("action_domain.action_ids"));
    }

    let mut action_domain = BTreeSet::new();
    for action in &tree.action_ids {
        require_nonempty(&[("action_id", action)])?;
        if !action_domain.insert(action.clone()) {
            return Err(PolicyTreeError::DuplicateActionId(action.clone()));
        }
    }

    if !tree.action_domain.complete {
        return Ok(PolicyTreeReport {
            disposition: PolicyTreeDisposition::Unknown,
            root_winning_action_ids: Vec::new(),
            policy_witness: BTreeMap::new(),
            unknown_reasons: vec!["action-domain-incomplete".to_owned()],
            invalid_reasons: Vec::new(),
            explored_belief_nodes: 0,
            horizon,
            actuation_authorized: false,
        });
    }

    let mut nodes = BTreeMap::new();
    for node in &tree.nodes {
        require_nonempty(&[("belief_node.node_id", &node.node_id)])?;
        if nodes.insert(node.node_id.clone(), node).is_some() {
            return Err(PolicyTreeError::DuplicateNodeId(node.node_id.clone()));
        }
        for edge in &node.transitions {
            require_nonempty(&[
                ("transition.from_state_id", &edge.from_state_id),
                ("transition.action_id", &edge.action_id),
                ("transition.to_state_id", &edge.to_state_id),
                ("transition.next_belief_node_id", &edge.next_belief_node_id),
            ])?;
            if !action_domain.contains(&edge.action_id) {
                return Err(PolicyTreeError::UnknownActionId(edge.action_id.clone()));
            }
        }
    }
    if !nodes.contains_key(&tree.root_node_id) {
        return Err(PolicyTreeError::MissingRootNode(tree.root_node_id.clone()));
    }

    let mut active = BTreeSet::new();
    let mut memo = BTreeMap::new();
    let mut explored = BTreeSet::new();
    let solved = solve_node(
        &tree.root_node_id,
        horizon,
        &nodes,
        &action_domain,
        &mut active,
        &mut memo,
        &mut explored,
    )?;

    Ok(PolicyTreeReport {
        disposition: solved.disposition,
        root_winning_action_ids: solved.winning_action_ids,
        policy_witness: solved.witness,
        unknown_reasons: solved.unknown_reasons,
        invalid_reasons: solved.invalid_reasons,
        explored_belief_nodes: explored.len(),
        horizon,
        actuation_authorized: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn physical_digest(state_id: &str) -> String {
        format!("physical:{state_id}")
    }

    fn reserve_digest(state_id: &str) -> String {
        format!("reserve:{state_id}")
    }

    fn thermal_digest(state_id: &str) -> String {
        format!("thermal:{state_id}")
    }

    fn state_receipt(state_id: &str) -> BeliefStateReceipt {
        BeliefStateReceipt {
            state_id: state_id.to_owned(),
            physical_state_digest: physical_digest(state_id),
            reserve_state_digest: reserve_digest(state_id),
            thermal_state_digest: thermal_digest(state_id),
            receipt_digest: format!("state-receipt:{state_id}"),
        }
    }

    fn qualified_leaf(node_id: &str, state_id: &str) -> BeliefNode {
        BeliefNode {
            node_id: node_id.to_owned(),
            observation_receipt_digest: format!("observation:{node_id}"),
            observation_disposition: ObservationDisposition::Qualified,
            state_ids: vec![state_id.to_owned()],
            state_evidence: vec![state_receipt(state_id)],
            transition_manifest: EvidenceManifest {
                complete: true,
                digest: format!("manifest:{node_id}"),
            },
            state_action_evidence: Vec::new(),
            transitions: Vec::new(),
            terminal_recovery: Some(TerminalRecoveryEvidence::Qualified {
                state_receipts: vec![TerminalRecoveryReceipt {
                    state_id: state_id.to_owned(),
                    reserve_state_digest: reserve_digest(state_id),
                    topology_digest: format!("topology:{state_id}"),
                    terminal_safe_stop_reachable: true,
                }],
            }),
        }
    }

    fn state_actions(state_id: &str, actions: &[&str]) -> StateActionEvidence {
        StateActionEvidence {
            state_id: state_id.to_owned(),
            complete: true,
            receipt_digest: format!("admission:{state_id}"),
            admitted_action_ids: actions.iter().map(|a| (*a).to_owned()).collect(),
        }
    }

    fn edge(from: &str, action: &str, to: &str, next: &str) -> PolicyTransition {
        PolicyTransition {
            from_state_id: from.to_owned(),
            action_id: action.to_owned(),
            to_state_id: to.to_owned(),
            from_state_digest: physical_digest(from),
            to_state_digest: physical_digest(to),
            predecessor_reserve_state_digest: reserve_digest(from),
            resulting_reserve_state_digest: reserve_digest(to),
            predecessor_thermal_state_digest: thermal_digest(from),
            resulting_thermal_state_digest: thermal_digest(to),
            next_belief_node_id: next.to_owned(),
            physically_possible: true,
            evidence_complete: true,
            receipt_digest: format!("transition:{from}:{action}:{to}"),
        }
    }

    fn root_two_step_tree() -> FinitePolicyTree {
        let root = BeliefNode {
            node_id: "root".to_owned(),
            observation_receipt_digest: "observation:root".to_owned(),
            observation_disposition: ObservationDisposition::Qualified,
            state_ids: vec!["s1".to_owned(), "s2".to_owned()],
            state_evidence: vec![state_receipt("s1"), state_receipt("s2")],
            transition_manifest: EvidenceManifest {
                complete: true,
                digest: "root-transitions".to_owned(),
            },
            state_action_evidence: vec![
                state_actions("s1", &["probe"]),
                state_actions("s2", &["probe"]),
            ],
            transitions: vec![
                edge("s1", "probe", "s1-rough", "rough-observation"),
                edge("s2", "probe", "s2-firm", "firm-observation"),
            ],
            terminal_recovery: None,
        };
        let rough = BeliefNode {
            node_id: "rough-observation".to_owned(),
            observation_receipt_digest: "observation:rough".to_owned(),
            observation_disposition: ObservationDisposition::Qualified,
            state_ids: vec!["s1-rough".to_owned()],
            state_evidence: vec![state_receipt("s1-rough")],
            transition_manifest: EvidenceManifest {
                complete: true,
                digest: "rough-transitions".to_owned(),
            },
            state_action_evidence: vec![state_actions("s1-rough", &["retreat"])],
            transitions: vec![edge("s1-rough", "retreat", "s1-safe", "rough-terminal")],
            terminal_recovery: None,
        };
        let firm = BeliefNode {
            node_id: "firm-observation".to_owned(),
            observation_receipt_digest: "observation:firm".to_owned(),
            observation_disposition: ObservationDisposition::Qualified,
            state_ids: vec!["s2-firm".to_owned()],
            state_evidence: vec![state_receipt("s2-firm")],
            transition_manifest: EvidenceManifest {
                complete: true,
                digest: "firm-transitions".to_owned(),
            },
            state_action_evidence: vec![state_actions("s2-firm", &["continue"])],
            transitions: vec![edge("s2-firm", "continue", "s2-safe", "firm-terminal")],
            terminal_recovery: None,
        };
        FinitePolicyTree {
            root_node_id: "root".to_owned(),
            action_domain: EvidenceManifest {
                complete: true,
                digest: "action-domain".to_owned(),
            },
            action_ids: vec![
                "probe".to_owned(),
                "retreat".to_owned(),
                "continue".to_owned(),
            ],
            nodes: vec![
                root,
                rough,
                firm,
                qualified_leaf("rough-terminal", "s1-safe"),
                qualified_leaf("firm-terminal", "s2-safe"),
            ],
        }
    }

    #[test]
    fn bounded_policy_tree_uses_one_action_per_observation_and_safe_leaves() {
        let tree = root_two_step_tree();
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(
            report.disposition,
            PolicyTreeDisposition::WinningPolicyFound
        );
        assert_eq!(report.root_winning_action_ids, vec!["probe".to_owned()]);
        assert_eq!(report.policy_witness["root"], "probe");
        assert_eq!(report.policy_witness["rough-observation"], "retreat");
        assert_eq!(report.policy_witness["firm-observation"], "continue");
        assert!(!report.actuation_authorized);
    }

    #[test]
    fn hidden_states_with_disjoint_admitted_actions_have_no_policy() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].state_action_evidence = vec![
            state_actions("s1", &["retreat"]),
            state_actions("s2", &["continue"]),
        ];
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(
            report.disposition,
            PolicyTreeDisposition::NoWinningPolicyInDeclaredDomain
        );
        assert!(report.policy_witness.is_empty());
    }

    #[test]
    fn unknown_transition_manifest_does_not_produce_a_policy() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].transition_manifest.complete = false;
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
        assert!(report.policy_witness.is_empty());
    }

    #[test]
    fn unknown_leaf_recovery_receipt_does_not_produce_a_policy() {
        let mut tree = root_two_step_tree();
        tree.nodes[3].terminal_recovery = Some(TerminalRecoveryEvidence::Unknown {
            reason: "thermal reserve receipt stale".to_owned(),
        });
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
    }

    #[test]
    fn unreachable_terminal_safe_stop_removes_the_policy() {
        let mut tree = root_two_step_tree();
        tree.nodes[3].terminal_recovery = Some(TerminalRecoveryEvidence::Qualified {
            state_receipts: vec![TerminalRecoveryReceipt {
                state_id: "s1-safe".to_owned(),
                reserve_state_digest: reserve_digest("s1-safe"),
                topology_digest: "topology".to_owned(),
                terminal_safe_stop_reachable: false,
            }],
        });
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(
            report.disposition,
            PolicyTreeDisposition::NoWinningPolicyInDeclaredDomain
        );
    }

    #[test]
    fn transition_to_wrong_observation_state_set_is_unknown() {
        let mut tree = root_two_step_tree();
        tree.nodes[1].state_ids = vec!["unreached-state".to_owned()];
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
        assert!(
            report
                .unknown_reasons
                .iter()
                .any(|r| r.contains("partition-mismatch"))
        );
    }

    #[test]
    fn transition_receipt_unknown_propagates_unknown() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].transitions[0].evidence_complete = false;
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
    }

    #[test]
    fn physically_impossible_transition_does_not_enter_the_policy_tree() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].transitions.push(PolicyTransition {
            from_state_id: "s1".to_owned(),
            action_id: "probe".to_owned(),
            to_state_id: "impossible".to_owned(),
            from_state_digest: String::new(),
            to_state_digest: String::new(),
            predecessor_reserve_state_digest: String::new(),
            resulting_reserve_state_digest: String::new(),
            predecessor_thermal_state_digest: String::new(),
            resulting_thermal_state_digest: String::new(),
            next_belief_node_id: "not-a-node".to_owned(),
            physically_possible: false,
            evidence_complete: false,
            receipt_digest: String::new(),
        });
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(
            report.disposition,
            PolicyTreeDisposition::WinningPolicyFound
        );
    }

    #[test]
    fn transition_predecessor_reserve_and_thermal_digests_must_match_parent_state() {
        let mut reserve = root_two_step_tree();
        reserve.nodes[0].transitions[0].predecessor_reserve_state_digest =
            "reserve:other".to_owned();
        assert_eq!(
            evaluate_bounded_policy_tree(&reserve, 2)
                .unwrap()
                .disposition,
            PolicyTreeDisposition::Invalid
        );

        let mut thermal = root_two_step_tree();
        thermal.nodes[0].transitions[0].predecessor_thermal_state_digest =
            "thermal:other".to_owned();
        assert_eq!(
            evaluate_bounded_policy_tree(&thermal, 2)
                .unwrap()
                .disposition,
            PolicyTreeDisposition::Invalid
        );
    }

    #[test]
    fn transition_result_digests_must_match_child_belief_state() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].transitions[0].resulting_thermal_state_digest =
            "thermal:wrong-successor".to_owned();
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Invalid);
        assert!(
            report
                .invalid_reasons
                .iter()
                .any(|reason| reason.contains("successor-state-binding-mismatch"))
        );
    }

    #[test]
    fn terminal_recovery_must_cover_every_hidden_state_at_the_leaf() {
        let mut tree = root_two_step_tree();
        tree.nodes[3].state_ids = vec!["s1-safe".to_owned(), "s1-safe-alt".to_owned()];
        tree.nodes[3]
            .state_evidence
            .push(state_receipt("s1-safe-alt"));
        // The terminal receipt still covers only s1-safe.
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
    }

    #[test]
    fn terminal_reserve_digest_must_match_each_leaf_state() {
        let mut tree = root_two_step_tree();
        tree.nodes[3].terminal_recovery = Some(TerminalRecoveryEvidence::Qualified {
            state_receipts: vec![TerminalRecoveryReceipt {
                state_id: "s1-safe".to_owned(),
                reserve_state_digest: "reserve:stale".to_owned(),
                topology_digest: "topology:s1-safe".to_owned(),
                terminal_safe_stop_reachable: true,
            }],
        });
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Invalid);
        assert!(
            report
                .invalid_reasons
                .iter()
                .any(|reason| reason.contains("terminal-reserve-state-mismatch"))
        );
    }

    #[test]
    fn unknown_or_stale_observation_receipt_cannot_split_a_belief() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].observation_disposition = ObservationDisposition::Unknown;
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
        assert!(report.policy_witness.is_empty());
    }

    #[test]
    fn invalid_observation_receipt_rejects_a_policy_witness() {
        let mut tree = root_two_step_tree();
        tree.nodes[1].observation_disposition = ObservationDisposition::Invalid;
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Invalid);
        assert!(report.policy_witness.is_empty());
    }

    #[test]
    fn action_domain_unknown_prevents_policy_synthesis() {
        let mut tree = root_two_step_tree();
        tree.action_domain.complete = false;
        let report = evaluate_bounded_policy_tree(&tree, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
    }

    #[test]
    fn policy_tree_permutation_keeps_the_same_witness() {
        let mut tree_a = root_two_step_tree();
        let mut tree_b = root_two_step_tree();
        tree_a.nodes.reverse();
        tree_b.nodes.reverse();
        tree_b.nodes[0].transitions.reverse();
        let a = evaluate_bounded_policy_tree(&tree_a, 2).unwrap();
        let b = evaluate_bounded_policy_tree(&tree_b, 2).unwrap();
        assert_eq!(a.disposition, b.disposition);
        assert_eq!(a.policy_witness, b.policy_witness);
    }

    #[test]
    fn invalid_action_identifiers_and_missing_child_nodes_are_not_accepted() {
        let mut tree = root_two_step_tree();
        tree.nodes[0].state_action_evidence[0]
            .admitted_action_ids
            .push("not-in-domain".to_owned());
        assert!(matches!(
            evaluate_bounded_policy_tree(&tree, 2),
            Err(PolicyTreeError::UnknownActionId(_))
        ));

        let mut missing_child = root_two_step_tree();
        missing_child.nodes[0].transitions[0].next_belief_node_id =
            "unprovided-observation-node".to_owned();
        let report = evaluate_bounded_policy_tree(&missing_child, 2).unwrap();
        assert_eq!(report.disposition, PolicyTreeDisposition::Unknown);
        assert!(
            report
                .unknown_reasons
                .iter()
                .any(|reason| reason.contains("missing-child-node"))
        );
    }

    #[test]
    fn policy_horizon_is_explicitly_bounded() {
        let tree = root_two_step_tree();
        assert!(matches!(
            evaluate_bounded_policy_tree(&tree, 0),
            Err(PolicyTreeError::InvalidHorizon(0))
        ));
        assert!(matches!(
            evaluate_bounded_policy_tree(&tree, 4),
            Err(PolicyTreeError::InvalidHorizon(4))
        ));
    }
}
