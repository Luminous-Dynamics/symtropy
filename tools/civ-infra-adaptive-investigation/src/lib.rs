#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "civ-infra-adaptive-investigation/v0.1";
pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const MAX_INPUT_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_STATES: usize = 1_000;
pub const MAX_CANDIDATES: usize = 1_000;
pub const MAX_ACTIONS: usize = 1_000;
pub const MAX_OUTCOMES_PER_ACTION: usize = 1_000;
pub const MAX_POLICY_DEPTH: usize = 16;
pub const MAX_POLICY_EVALUATION_OPERATIONS: usize = 100_000;
pub const MAX_POLICY_LEAVES: usize = 10_000;
pub const MAX_POLICY_LEAF_STATE_MEMBERSHIPS: usize = 100_000;
const CLAIM_CEILING: &str =
    "synthetic_compatibility_graph_only_no_physical_measurement_or_engineering_validity_claim";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyInput {
    pub schema_version: String,
    pub world_id: String,
    pub candidate_ids: Vec<String>,
    pub state_signatures: Vec<StateSignatureInput>,
    pub actions: Vec<ActionInput>,
    pub candidate_policy: CandidatePolicy,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateSignatureInput {
    pub state_id: String,
    pub signature: CandidateSignature,
    pub upstream_case_ref: String,
    #[serde(default)]
    pub upstream_report_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidateSignature {
    pub feasible_candidates: Vec<String>,
    pub infeasible_candidates: Vec<String>,
    pub unknown_candidates: Vec<String>,
    pub frontier_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionInput {
    pub action_id: String,
    pub description: String,
    pub compatibility_report_ref: String,
    #[serde(default)]
    pub compatibility_report_sha256: Option<String>,
    pub model_support: ModelSupport,
    pub outcomes: Vec<OutcomeInput>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelSupport {
    Closed,
    Open,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OutcomeInput {
    pub outcome_id: String,
    pub compatible_state_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CandidatePolicy {
    pub root_action_id: String,
    pub branches: Vec<PolicyBranch>,
    pub support_failure_terminal: SupportFailureTerminal,
    pub max_supported_action_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PolicyBranch {
    pub at_action_id: String,
    pub outcome_id: String,
    pub next_action_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SupportFailureTerminal {
    ReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub world_id: String,
    pub input_sha256: String,
    pub claim_ceiling: String,
    pub ranking_policy: String,
    pub state_count: usize,
    pub candidate_count: usize,
    pub action_count: usize,
    pub policy_depth_limit: usize,
    pub leaves: Vec<PolicyLeaf>,
    pub supported_leaf_count: usize,
    pub support_failure_leaf_count: usize,
    pub worst_supported_leaf_state_count: Option<usize>,
    pub worst_supported_leaf_signature_classes: Option<usize>,
    pub worst_supported_leaf_discordant_pairs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyStep {
    pub action_id: String,
    pub outcome_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LeafTerminal {
    DecisionLeaf,
    ReviewRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyLeaf {
    pub path: Vec<PolicyStep>,
    pub terminal: LeafTerminal,
    pub state_ids: Vec<String>,
    pub state_count: usize,
    pub decision_signature_class_count: usize,
    pub decision_discordant_pairs: u64,
}

#[derive(Clone)]
struct NormalizedState {
    signature_key: String,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Hash the exact input bytes, validate the contract, and evaluate every outcome branch.
/// Open-support actions also emit a review terminal that preserves all current states.
pub fn evaluate_bytes(bytes: &[u8]) -> Result<PolicyReport, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(format!("input byte count exceeds limit {MAX_INPUT_BYTES}"));
    }
    let input_sha256 = sha256_hex(bytes);
    let input: PolicyInput =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid input JSON: {error}"))?;
    evaluate(&input, &input_sha256)
}

fn evaluate(input: &PolicyInput, input_sha256: &str) -> Result<PolicyReport, String> {
    validate_digest(input_sha256, "input_sha256")?;
    validate_input(input)?;

    let mut states = BTreeMap::<String, NormalizedState>::new();
    for state in &input.state_signatures {
        let mut signature = state.signature.clone();
        signature.feasible_candidates.sort();
        signature.infeasible_candidates.sort();
        signature.unknown_candidates.sort();
        signature.frontier_candidates.sort();
        let signature_key = serde_json::to_string(&signature)
            .map_err(|error| format!("cannot normalize state signature: {error}"))?;
        states.insert(state.state_id.clone(), NormalizedState { signature_key });
    }

    let actions = input
        .actions
        .iter()
        .map(|action| (action.action_id.clone(), action))
        .collect::<BTreeMap<_, _>>();
    let branches = input
        .candidate_policy
        .branches
        .iter()
        .map(|branch| {
            (
                (branch.at_action_id.clone(), branch.outcome_id.clone()),
                branch.next_action_id.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let initial_states = states.keys().cloned().collect::<BTreeSet<_>>();
    let mut leaves = Vec::new();
    let mut policy_operations = 0usize;
    let mut leaf_state_memberships = 0usize;
    visit_policy(
        &input.candidate_policy.root_action_id,
        &initial_states,
        1,
        &input.candidate_policy,
        &actions,
        &branches,
        &states,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut policy_operations,
        &mut leaf_state_memberships,
        &mut leaves,
    )?;

    leaves.sort_by(|left, right| {
        path_key(&left.path)
            .cmp(&path_key(&right.path))
            .then_with(|| left.terminal.cmp(&right.terminal))
            .then_with(|| left.state_ids.cmp(&right.state_ids))
    });

    let supported = leaves
        .iter()
        .filter(|leaf| leaf.terminal == LeafTerminal::DecisionLeaf)
        .collect::<Vec<_>>();
    let support_failure_leaf_count = leaves
        .iter()
        .filter(|leaf| leaf.terminal == LeafTerminal::ReviewRequired)
        .count();

    Ok(PolicyReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        world_id: input.world_id.clone(),
        input_sha256: input_sha256.to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        ranking_policy: "supported leaves only: worst distinct decision-signature classes ASC; then worst decision-discordant state pairs ASC; then worst residual state count ASC. Support-failure leaves preserve their full prior state set and terminate in review; no probabilities or expected VoI are inferred.".to_owned(),
        state_count: states.len(),
        candidate_count: input.candidate_ids.len(),
        action_count: input.actions.len(),
        policy_depth_limit: input.candidate_policy.max_supported_action_depth,
        supported_leaf_count: supported.len(),
        support_failure_leaf_count,
        worst_supported_leaf_state_count: supported.iter().map(|leaf| leaf.state_count).max(),
        worst_supported_leaf_signature_classes: supported
            .iter()
            .map(|leaf| leaf.decision_signature_class_count)
            .max(),
        worst_supported_leaf_discordant_pairs: supported
            .iter()
            .map(|leaf| leaf.decision_discordant_pairs)
            .max(),
        leaves,
    })
}

#[allow(clippy::too_many_arguments)]
fn visit_policy(
    action_id: &str,
    active_state_ids: &BTreeSet<String>,
    depth: usize,
    policy: &CandidatePolicy,
    actions: &BTreeMap<String, &ActionInput>,
    branches: &BTreeMap<(String, String), String>,
    states: &BTreeMap<String, NormalizedState>,
    path: &mut Vec<PolicyStep>,
    ancestors: &mut BTreeSet<String>,
    policy_operations: &mut usize,
    leaf_state_memberships: &mut usize,
    leaves: &mut Vec<PolicyLeaf>,
) -> Result<(), String> {
    consume_policy_operation(policy_operations)?;
    if depth > policy.max_supported_action_depth {
        return Err("candidate policy exceeds max_supported_action_depth".to_owned());
    }
    if !ancestors.insert(action_id.to_owned()) {
        return Err(format!("cycle detected at action {action_id:?}"));
    }

    let action = actions
        .get(action_id)
        .ok_or_else(|| format!("unknown action {action_id:?}"))?;

    if action.model_support == ModelSupport::Open {
        path.push(PolicyStep {
            action_id: action_id.to_owned(),
            outcome_id: "out_of_model_support".to_owned(),
        });
        record_leaf(
            leaves,
            leaf_state_memberships,
            make_leaf(
                path.clone(),
                LeafTerminal::ReviewRequired,
                active_state_ids,
                states,
            )?,
        )?;
        path.pop();
    }

    for outcome in &action.outcomes {
        consume_policy_operation(policy_operations)?;
        let outcome_states = outcome
            .compatible_state_ids
            .iter()
            .filter(|state_id| active_state_ids.contains(*state_id))
            .cloned()
            .collect::<BTreeSet<_>>();
        if outcome_states.is_empty() {
            continue;
        }

        path.push(PolicyStep {
            action_id: action_id.to_owned(),
            outcome_id: outcome.outcome_id.clone(),
        });

        if let Some(next_action_id) =
            branches.get(&(action_id.to_owned(), outcome.outcome_id.clone()))
        {
            if depth >= policy.max_supported_action_depth {
                return Err(format!(
                    "policy branch {:?}/{:?} exceeds declared action depth",
                    action_id, outcome.outcome_id
                ));
            }
            visit_policy(
                next_action_id,
                &outcome_states,
                depth + 1,
                policy,
                actions,
                branches,
                states,
                path,
                ancestors,
                policy_operations,
                leaf_state_memberships,
                leaves,
            )?;
        } else {
            record_leaf(
                leaves,
                leaf_state_memberships,
                make_leaf(
                    path.clone(),
                    LeafTerminal::DecisionLeaf,
                    &outcome_states,
                    states,
                )?,
            )?;
        }

        path.pop();
    }

    ancestors.remove(action_id);
    Ok(())
}

fn make_leaf(
    path: Vec<PolicyStep>,
    terminal: LeafTerminal,
    state_ids: &BTreeSet<String>,
    states: &BTreeMap<String, NormalizedState>,
) -> Result<PolicyLeaf, String> {
    let mut class_counts = BTreeMap::<String, u64>::new();
    for state_id in state_ids {
        let state = states
            .get(state_id)
            .ok_or_else(|| format!("internal missing state {state_id:?}"))?;
        *class_counts.entry(state.signature_key.clone()).or_insert(0) += 1;
    }

    let state_count =
        u64::try_from(state_ids.len()).map_err(|_| "state count conversion overflow".to_owned())?;
    let all_pairs = choose_two(state_count)?;
    let same_class_pairs = class_counts
        .values()
        .try_fold(0_u64, |total, count| {
            total.checked_add(choose_two(*count).ok()?)
        })
        .ok_or_else(|| "same-signature pair count overflow".to_owned())?;
    let discordant_pairs = all_pairs
        .checked_sub(same_class_pairs)
        .ok_or_else(|| "decision-discordant pair accounting underflow".to_owned())?;

    Ok(PolicyLeaf {
        path,
        terminal,
        state_ids: state_ids.iter().cloned().collect(),
        state_count: state_ids.len(),
        decision_signature_class_count: class_counts.len(),
        decision_discordant_pairs: discordant_pairs,
    })
}

fn consume_policy_operation(operations: &mut usize) -> Result<(), String> {
    *operations = operations
        .checked_add(1)
        .ok_or_else(|| "policy evaluation operation counter overflow".to_owned())?;
    if *operations > MAX_POLICY_EVALUATION_OPERATIONS {
        return Err(format!(
            "policy evaluation exceeds operation budget {MAX_POLICY_EVALUATION_OPERATIONS}"
        ));
    }
    Ok(())
}

fn record_leaf(
    leaves: &mut Vec<PolicyLeaf>,
    total_state_memberships: &mut usize,
    leaf: PolicyLeaf,
) -> Result<(), String> {
    if leaves.len() >= MAX_POLICY_LEAVES {
        return Err(format!(
            "policy expansion exceeds leaf budget {MAX_POLICY_LEAVES}"
        ));
    }
    let next_membership_total = total_state_memberships
        .checked_add(leaf.state_count)
        .ok_or_else(|| "policy leaf state-membership counter overflow".to_owned())?;
    if next_membership_total > MAX_POLICY_LEAF_STATE_MEMBERSHIPS {
        return Err(format!(
            "policy expansion exceeds state-membership budget {MAX_POLICY_LEAF_STATE_MEMBERSHIPS}"
        ));
    }
    *total_state_memberships = next_membership_total;
    leaves.push(leaf);
    Ok(())
}

fn choose_two(value: u64) -> Result<u64, String> {
    if value < 2 {
        return Ok(0);
    }
    value
        .checked_mul(value - 1)
        .map(|product| product / 2)
        .ok_or_else(|| "pair count overflow".to_owned())
}

fn path_key(path: &[PolicyStep]) -> String {
    path.iter()
        .map(|step| format!("{}:{}", step.action_id, step.outcome_id))
        .collect::<Vec<_>>()
        .join("|")
}

fn validate_input(input: &PolicyInput) -> Result<(), String> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}",
            input.schema_version
        ));
    }
    validate_name(&input.world_id, "world_id")?;
    if input.claim_ceiling != CLAIM_CEILING {
        return Err("claim_ceiling must equal the fixed synthetic-only ceiling".to_owned());
    }

    let candidates = unique_name_set(&input.candidate_ids, "candidate_ids")?;
    if candidates.is_empty() || candidates.len() > MAX_CANDIDATES {
        return Err(format!(
            "candidate count must be within 1..={MAX_CANDIDATES}"
        ));
    }
    if input.state_signatures.is_empty() || input.state_signatures.len() > MAX_STATES {
        return Err(format!("state count must be within 1..={MAX_STATES}"));
    }

    let mut state_ids = BTreeSet::new();
    for state in &input.state_signatures {
        validate_name(&state.state_id, "state_id")?;
        validate_name(&state.upstream_case_ref, "upstream_case_ref")?;
        if let Some(digest) = &state.upstream_report_sha256 {
            validate_digest(digest, "upstream_report_sha256")?;
        }
        if !state_ids.insert(state.state_id.clone()) {
            return Err(format!("duplicate state_id {:?}", state.state_id));
        }
        validate_signature(&state.signature, &candidates)?;
    }

    if input.actions.is_empty() || input.actions.len() > MAX_ACTIONS {
        return Err(format!("action count must be within 1..={MAX_ACTIONS}"));
    }
    let mut action_ids = BTreeSet::new();
    for action in &input.actions {
        validate_name(&action.action_id, "action_id")?;
        validate_name(&action.description, "action description")?;
        validate_name(&action.compatibility_report_ref, "compatibility_report_ref")?;
        if let Some(digest) = &action.compatibility_report_sha256 {
            validate_digest(digest, "compatibility_report_sha256")?;
        }
        if !action_ids.insert(action.action_id.clone()) {
            return Err(format!("duplicate action_id {:?}", action.action_id));
        }
        if action.outcomes.is_empty() || action.outcomes.len() > MAX_OUTCOMES_PER_ACTION {
            return Err(format!(
                "outcome count for {:?} must be within 1..={MAX_OUTCOMES_PER_ACTION}",
                action.action_id
            ));
        }

        let mut outcome_ids = BTreeSet::new();
        let mut covered = BTreeSet::new();
        for outcome in &action.outcomes {
            validate_name(&outcome.outcome_id, "outcome_id")?;
            if !outcome_ids.insert(outcome.outcome_id.clone()) {
                return Err(format!("duplicate outcome_id {:?}", outcome.outcome_id));
            }
            if outcome.compatible_state_ids.is_empty() {
                return Err(format!(
                    "empty compatibility outcome {:?}",
                    outcome.outcome_id
                ));
            }
            let mut local = BTreeSet::new();
            for state_id in &outcome.compatible_state_ids {
                if !state_ids.contains(state_id) {
                    return Err(format!(
                        "outcome {:?} references unknown state {:?}",
                        outcome.outcome_id, state_id
                    ));
                }
                if !local.insert(state_id.clone()) {
                    return Err(format!(
                        "duplicate compatibility edge ({state_id:?}, {:?})",
                        outcome.outcome_id
                    ));
                }
                covered.insert(state_id.clone());
            }
        }
        if action.model_support == ModelSupport::Closed && covered != state_ids {
            let missing = state_ids.difference(&covered).cloned().collect::<Vec<_>>();
            return Err(format!(
                "closed support for action {:?} leaves states without an outcome: {:?}",
                action.action_id, missing
            ));
        }
    }

    let policy = &input.candidate_policy;
    validate_name(&policy.root_action_id, "root_action_id")?;
    if !action_ids.contains(&policy.root_action_id) {
        return Err(format!(
            "unknown root_action_id {:?}",
            policy.root_action_id
        ));
    }
    if !(1..=MAX_POLICY_DEPTH).contains(&policy.max_supported_action_depth) {
        return Err(format!(
            "max_supported_action_depth must be within 1..={MAX_POLICY_DEPTH}"
        ));
    }

    let action_map = input
        .actions
        .iter()
        .map(|action| (action.action_id.as_str(), action))
        .collect::<BTreeMap<_, _>>();
    let mut branch_keys = BTreeSet::new();
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    for branch in &policy.branches {
        let action = action_map
            .get(branch.at_action_id.as_str())
            .ok_or_else(|| format!("branch references unknown action {:?}", branch.at_action_id))?;
        validate_name(&branch.outcome_id, "branch outcome_id")?;
        validate_name(&branch.next_action_id, "branch next_action_id")?;
        if !action_map.contains_key(branch.next_action_id.as_str()) {
            return Err(format!(
                "branch references unknown next action {:?}",
                branch.next_action_id
            ));
        }
        if !action
            .outcomes
            .iter()
            .any(|outcome| outcome.outcome_id == branch.outcome_id)
        {
            return Err(format!(
                "branch references unknown outcome {:?} for action {:?}",
                branch.outcome_id, branch.at_action_id
            ));
        }
        if !branch_keys.insert((branch.at_action_id.clone(), branch.outcome_id.clone())) {
            return Err(format!(
                "duplicate policy transition {:?}/{:?}",
                branch.at_action_id, branch.outcome_id
            ));
        }
        adjacency
            .entry(branch.at_action_id.clone())
            .or_default()
            .insert(branch.next_action_id.clone());
    }

    if policy.support_failure_terminal != SupportFailureTerminal::ReviewRequired {
        return Err("unsupported support_failure_terminal".to_owned());
    }
    validate_policy_graph(
        &policy.root_action_id,
        policy.max_supported_action_depth,
        &adjacency,
    )
}

fn validate_policy_graph(
    root: &str,
    max_depth: usize,
    adjacency: &BTreeMap<String, BTreeSet<String>>,
) -> Result<(), String> {
    fn longest_path(
        current: &str,
        adjacency: &BTreeMap<String, BTreeSet<String>>,
        visiting: &mut BTreeSet<String>,
        memo: &mut BTreeMap<String, usize>,
    ) -> Result<usize, String> {
        if let Some(longest) = memo.get(current) {
            return Ok(*longest);
        }
        if !visiting.insert(current.to_owned()) {
            return Err(format!("policy transition cycle detected at {current:?}"));
        }

        let mut longest = 1usize;
        if let Some(next_actions) = adjacency.get(current) {
            for next in next_actions {
                let suffix = longest_path(next, adjacency, visiting, memo)?;
                let candidate = suffix
                    .checked_add(1)
                    .ok_or_else(|| "policy path depth overflow".to_owned())?;
                longest = longest.max(candidate);
            }
        }

        visiting.remove(current);
        memo.insert(current.to_owned(), longest);
        Ok(longest)
    }

    let longest = longest_path(root, adjacency, &mut BTreeSet::new(), &mut BTreeMap::new())?;
    if longest > max_depth {
        return Err(format!(
            "policy transition longest path {longest} exceeds max depth {max_depth}"
        ));
    }
    Ok(())
}

fn validate_signature(
    signature: &CandidateSignature,
    candidates: &BTreeSet<String>,
) -> Result<(), String> {
    let feasible = unique_name_set(&signature.feasible_candidates, "feasible_candidates")?;
    let infeasible = unique_name_set(&signature.infeasible_candidates, "infeasible_candidates")?;
    let unknown = unique_name_set(&signature.unknown_candidates, "unknown_candidates")?;
    let frontier = unique_name_set(&signature.frontier_candidates, "frontier_candidates")?;

    for (label, values) in [
        ("feasible_candidates", &feasible),
        ("infeasible_candidates", &infeasible),
        ("unknown_candidates", &unknown),
    ] {
        if let Some(outside) = values.difference(candidates).next() {
            return Err(format!("{label} references unknown candidate {outside:?}"));
        }
    }
    if !feasible.is_disjoint(&infeasible)
        || !feasible.is_disjoint(&unknown)
        || !infeasible.is_disjoint(&unknown)
    {
        return Err("feasible, infeasible, and unknown candidate classes overlap".to_owned());
    }
    let classified = feasible
        .union(&infeasible)
        .cloned()
        .chain(unknown.iter().cloned())
        .collect::<BTreeSet<_>>();
    if classified != *candidates {
        let missing = candidates
            .difference(&classified)
            .cloned()
            .collect::<Vec<_>>();
        return Err(format!(
            "every candidate must be classified exactly once; missing {:?}",
            missing
        ));
    }
    if let Some(outside) = frontier.difference(&feasible).next() {
        return Err(format!("frontier candidate {outside:?} is not feasible"));
    }
    Ok(())
}

fn unique_name_set(values: &[String], label: &str) -> Result<BTreeSet<String>, String> {
    let mut result = BTreeSet::new();
    for value in values {
        validate_name(value, label)?;
        if !result.insert(value.clone()) {
            return Err(format!("duplicate value {value:?} in {label}"));
        }
    }
    Ok(result)
}

fn validate_name(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    if value.len() > 1024 {
        return Err(format!("{label} exceeds 1024 UTF-8 bytes"));
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        ));
    }
    Ok(())
}
