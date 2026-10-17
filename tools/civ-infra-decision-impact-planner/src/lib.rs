#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "civ-infra-decision-impact/v0.1";
pub const EVALUATOR_VERSION: &str = "0.1.0";
pub const MAX_STATES: usize = 100_000;
pub const MAX_ACTIONS: usize = 10_000;
pub const MAX_OUTCOMES: usize = 100_000;
pub const MAX_STATE_ACTION_EVALUATIONS: usize = 10_000_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlannerInput {
    pub schema_version: String,
    pub world_id: String,
    pub candidate_ids: Vec<String>,
    pub state_signatures: Vec<StateSignature>,
    pub actions: Vec<MeasurementAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StateSignature {
    pub state_id: String,
    pub signature: DecisionSignature,
    pub upstream_case_ref: String,
    #[serde(default)]
    pub upstream_report_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DecisionSignature {
    pub feasible_candidates: Vec<String>,
    pub infeasible_candidates: Vec<String>,
    pub unknown_candidates: Vec<String>,
    pub frontier_candidates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MeasurementAction {
    pub action_id: String,
    pub description: String,
    pub outcome_model_ref: String,
    #[serde(default)]
    pub outcome_model_sha256: Option<String>,
    pub outcomes: Vec<ActionOutcome>,
    #[serde(default)]
    pub estimated_cost: Option<Estimate>,
    #[serde(default)]
    pub estimated_duration: Option<Estimate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcome {
    pub outcome_id: String,
    pub state_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub value: u64,
    pub unit: String,
    pub basis: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanReport {
    pub schema_version: String,
    pub evaluator_version: String,
    pub world_id: String,
    pub input_sha256: String,
    pub claim_ceiling: String,
    pub ranking_policy: String,
    pub state_count: usize,
    pub candidate_count: usize,
    pub decision_signature_classes_before_observation: usize,
    pub state_bindings: Vec<StateBindingReceipt>,
    pub actions_ranked: Vec<ActionAssessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateBindingReceipt {
    pub state_id: String,
    pub decision_signature_sha256: String,
    pub upstream_case_ref: String,
    pub upstream_report_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActionAssessment {
    pub rank: usize,
    pub action_id: String,
    pub description: String,
    pub outcome_model_ref: String,
    pub outcome_model_sha256: Option<String>,
    pub impact: DecisionImpact,
    pub estimated_cost: Option<Estimate>,
    pub estimated_duration: Option<Estimate>,
    pub outcomes: Vec<OutcomeAssessment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionImpact {
    pub outcome_count: usize,
    pub best_case_remaining_states: usize,
    pub worst_case_remaining_states: usize,
    pub best_case_eliminated_states: usize,
    pub worst_case_eliminated_states: usize,
    pub worst_case_decision_signature_classes: usize,
    pub worst_case_decision_discordant_pairs_remaining: u64,
    pub homogeneous_outcome_count: usize,
    pub homogeneous_state_count: usize,
    pub total_decision_discordant_pairs_before_observation: u64,
    pub decision_discordant_pairs_separated: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeAssessment {
    pub outcome_id: String,
    pub state_count: usize,
    pub distinct_decision_signature_count: usize,
    pub decision_discordant_pairs_remaining: u64,
    pub decision_signature_sha256s: Vec<String>,
    pub state_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct NormalizedState {
    state_id: String,
    signature_key: String,
    signature_sha256: String,
    upstream_case_ref: String,
    upstream_report_sha256: Option<String>,
}

#[derive(Debug, Clone)]
struct RawActionAssessment {
    action_id: String,
    description: String,
    outcome_model_ref: String,
    outcome_model_sha256: Option<String>,
    impact: DecisionImpact,
    estimated_cost: Option<Estimate>,
    estimated_duration: Option<Estimate>,
    outcomes: Vec<OutcomeAssessment>,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Plan measurements by their exact finite-state decision impact. This function does
/// not assign probabilities, calculate expected value of information, or claim physical
/// validity for upstream decision signatures.
pub fn plan(input: &PlannerInput, input_sha256: &str) -> Result<PlanReport, String> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {:?}; expected {:?}",
            input.schema_version, SCHEMA_VERSION
        ));
    }
    validate_name(&input.world_id, "world_id")?;
    validate_digest(input_sha256, "input_sha256")?;

    if input.candidate_ids.is_empty() {
        return Err("candidate_ids must not be empty".to_owned());
    }
    if input.candidate_ids.len() > 100_000 {
        return Err("candidate_ids exceeds the 100000 item limit".to_owned());
    }
    let candidate_set = unique_name_set(&input.candidate_ids, "candidate_ids")?;

    if input.state_signatures.is_empty() {
        return Err("state_signatures must not be empty".to_owned());
    }
    if input.state_signatures.len() > MAX_STATES {
        return Err(format!("state count exceeds limit {MAX_STATES}"));
    }

    let mut states_by_id = BTreeMap::<String, NormalizedState>::new();
    let mut signature_counts = BTreeMap::<String, u64>::new();
    for state in &input.state_signatures {
        validate_name(&state.state_id, "state_id")?;
        validate_name(&state.upstream_case_ref, "upstream_case_ref")?;
        if let Some(digest) = &state.upstream_report_sha256 {
            validate_digest(digest, "upstream_report_sha256")?;
        }

        validate_signature(&state.signature, &candidate_set)?;
        let normalized = normalized_signature(&state.signature);
        let signature_json = serde_json::to_string(&normalized)
            .map_err(|err| format!("cannot canonicalize decision signature: {err}"))?;
        let signature_sha256 = sha256_hex(signature_json.as_bytes());
        *signature_counts.entry(signature_json.clone()).or_insert(0) += 1;

        let record = NormalizedState {
            state_id: state.state_id.clone(),
            signature_key: signature_json,
            signature_sha256,
            upstream_case_ref: state.upstream_case_ref.clone(),
            upstream_report_sha256: state.upstream_report_sha256.clone(),
        };
        if states_by_id.insert(state.state_id.clone(), record).is_some() {
            return Err(format!("duplicate state_id {:?}", state.state_id));
        }
    }
    let state_ids: BTreeSet<String> = states_by_id.keys().cloned().collect();
    let state_count = state_ids.len();
    let total_discordant_pairs = discordant_pairs(&signature_counts);

    if input.actions.is_empty() {
        return Err("actions must not be empty".to_owned());
    }
    if input.actions.len() > MAX_ACTIONS {
        return Err(format!("action count exceeds limit {MAX_ACTIONS}"));
    }
    validate_evaluation_bound(input.actions.len(), state_count)?;

    let mut action_ids = BTreeSet::new();
    let mut raw_assessments = Vec::with_capacity(input.actions.len());
    let mut total_outcome_count = 0usize;
    for action in &input.actions {
        validate_name(&action.action_id, "action_id")?;
        validate_name(&action.description, "action description")?;
        validate_name(&action.outcome_model_ref, "outcome_model_ref")?;
        if let Some(digest) = &action.outcome_model_sha256 {
            validate_digest(digest, "outcome_model_sha256")?;
        }
        if !action_ids.insert(action.action_id.clone()) {
            return Err(format!("duplicate action_id {:?}", action.action_id));
        }
        if action.outcomes.is_empty() {
            return Err(format!("action {:?} has no outcomes", action.action_id));
        }
        validate_estimate(action.estimated_cost.as_ref(), "estimated_cost")?;
        validate_estimate(action.estimated_duration.as_ref(), "estimated_duration")?;

        total_outcome_count = total_outcome_count
            .checked_add(action.outcomes.len())
            .ok_or_else(|| "outcome count overflow".to_owned())?;
        if total_outcome_count > MAX_OUTCOMES {
            return Err(format!("total outcome count exceeds limit {MAX_OUTCOMES}"));
        }

        let mut outcome_ids = BTreeSet::new();
        for outcome in &action.outcomes {
            validate_name(&outcome.outcome_id, "outcome_id")?;
            if !outcome_ids.insert(outcome.outcome_id.clone()) {
                return Err(format!(
                    "action {:?} has duplicate outcome_id {:?}",
                    action.action_id, outcome.outcome_id
                ));
            }
        }

        let mut seen_state_ids = BTreeSet::new();
        let mut outcome_assessments = Vec::with_capacity(action.outcomes.len());
        let mut outcome_class_counts = Vec::with_capacity(action.outcomes.len());
        let mut outcome_state_counts = Vec::with_capacity(action.outcomes.len());
        let mut outcome_discordant_pairs = Vec::with_capacity(action.outcomes.len());
        let mut homogeneous_outcome_count = 0usize;
        let mut homogeneous_state_count = 0usize;

        for outcome in &action.outcomes {
            validate_name(&outcome.outcome_id, "outcome_id")?;
            if outcome.state_ids.is_empty() {
                return Err(format!(
                    "action {:?} outcome {:?} has no states",
                    action.action_id, outcome.outcome_id
                ));
            }
            let mut local_class_counts = BTreeMap::<String, u64>::new();
            let mut canonical_state_ids = outcome.state_ids.clone();
            canonical_state_ids.sort();
            for state_id in &outcome.state_ids {
                if !state_ids.contains(state_id) {
                    return Err(format!(
                        "action {:?} outcome {:?} references unknown state {:?}",
                        action.action_id, outcome.outcome_id, state_id
                    ));
                }
                if !seen_state_ids.insert(state_id.clone()) {
                    return Err(format!(
                        "action {:?} contains state {:?} in more than one outcome",
                        action.action_id, state_id
                    ));
                }
                let state = states_by_id.get(state_id).ok_or_else(|| {
                    format!("internal state lookup failed for {:?}", state_id)
                })?;
                *local_class_counts
                    .entry(state.signature_key.clone())
                    .or_insert(0) += 1;
            }

            let n = outcome.state_ids.len();
            let local_discordant_pairs = discordant_pairs(&local_class_counts);
            if local_class_counts.len() == 1 {
                homogeneous_outcome_count += 1;
                homogeneous_state_count += n;
            }
            let mut signature_sha256s: Vec<String> = local_class_counts
                .keys()
                .map(|signature| sha256_hex(signature.as_bytes()))
                .collect();
            signature_sha256s.sort();

            outcome_assessments.push(OutcomeAssessment {
                outcome_id: outcome.outcome_id.clone(),
                state_count: n,
                distinct_decision_signature_count: local_class_counts.len(),
                decision_discordant_pairs_remaining: local_discordant_pairs,
                decision_signature_sha256s: signature_sha256s,
                state_ids: canonical_state_ids,
            });
            outcome_class_counts.push(local_class_counts);
            outcome_state_counts.push(n);
            outcome_discordant_pairs.push(local_discordant_pairs);
        }

        if seen_state_ids != state_ids {
            let missing: Vec<String> = state_ids.difference(&seen_state_ids).cloned().collect();
            return Err(format!(
                "action {:?} outcome partition does not cover every state; missing {:?}",
                action.action_id, missing
            ));
        }

        let best_case_remaining_states = *outcome_state_counts
            .iter()
            .min()
            .ok_or_else(|| "empty outcome sizes".to_owned())?;
        let worst_case_remaining_states = *outcome_state_counts
            .iter()
            .max()
            .ok_or_else(|| "empty outcome sizes".to_owned())?;
        let worst_case_decision_signature_classes = outcome_class_counts
            .iter()
            .map(BTreeMap::len)
            .max()
            .ok_or_else(|| "empty outcome class counts".to_owned())?;
        let worst_case_decision_discordant_pairs_remaining = outcome_discordant_pairs
            .iter()
            .copied()
            .max()
            .ok_or_else(|| "empty outcome pair counts".to_owned())?;

        let mut all_remaining_pairs = 0u64;
        for pairs in &outcome_discordant_pairs {
            all_remaining_pairs = all_remaining_pairs
                .checked_add(*pairs)
                .ok_or_else(|| "decision-discordant pair count overflow".to_owned())?;
        }
        let decision_discordant_pairs_separated = total_discordant_pairs
            .checked_sub(all_remaining_pairs)
            .ok_or_else(|| "partition pair accounting invariant failed".to_owned())?;

        let mut outcomes = outcome_assessments;
        outcomes.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));

        raw_assessments.push(RawActionAssessment {
            action_id: action.action_id.clone(),
            description: action.description.clone(),
            outcome_model_ref: action.outcome_model_ref.clone(),
            outcome_model_sha256: action.outcome_model_sha256.clone(),
            impact: DecisionImpact {
                outcome_count: action.outcomes.len(),
                best_case_remaining_states,
                worst_case_remaining_states,
                best_case_eliminated_states: state_count - best_case_remaining_states,
                worst_case_eliminated_states: state_count - worst_case_remaining_states,
                worst_case_decision_signature_classes,
                worst_case_decision_discordant_pairs_remaining,
                homogeneous_outcome_count,
                homogeneous_state_count,
                total_decision_discordant_pairs_before_observation: total_discordant_pairs,
                decision_discordant_pairs_separated,
            },
            estimated_cost: action.estimated_cost.clone(),
            estimated_duration: action.estimated_duration.clone(),
            outcomes,
        });
    }

    // A transparent lexicographic policy: decision ambiguity first, then worst-case
    // remaining decision-discordant pairs, then raw state count, then separated pairs.
    // Cost and duration are reported independently and are deliberately not scalarized.
    raw_assessments.sort_by(|left, right| {
        left.impact
            .worst_case_decision_signature_classes
            .cmp(&right.impact.worst_case_decision_signature_classes)
            .then_with(|| {
                left.impact
                    .worst_case_decision_discordant_pairs_remaining
                    .cmp(&right.impact.worst_case_decision_discordant_pairs_remaining)
            })
            .then_with(|| {
                left.impact
                    .worst_case_remaining_states
                    .cmp(&right.impact.worst_case_remaining_states)
            })
            .then_with(|| {
                right
                    .impact
                    .decision_discordant_pairs_separated
                    .cmp(&left.impact.decision_discordant_pairs_separated)
            })
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let actions_ranked = raw_assessments
        .into_iter()
        .enumerate()
        .map(|(index, raw)| ActionAssessment {
            rank: index + 1,
            action_id: raw.action_id,
            description: raw.description,
            outcome_model_ref: raw.outcome_model_ref,
            outcome_model_sha256: raw.outcome_model_sha256,
            impact: raw.impact,
            estimated_cost: raw.estimated_cost,
            estimated_duration: raw.estimated_duration,
            outcomes: raw.outcomes,
        })
        .collect();

    let state_bindings = states_by_id
        .values()
        .map(|state| StateBindingReceipt {
            state_id: state.state_id.clone(),
            decision_signature_sha256: state.signature_sha256.clone(),
            upstream_case_ref: state.upstream_case_ref.clone(),
            upstream_report_sha256: state.upstream_report_sha256.clone(),
        })
        .collect();

    Ok(PlanReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        evaluator_version: EVALUATOR_VERSION.to_owned(),
        world_id: input.world_id.clone(),
        input_sha256: input_sha256.to_owned(),
        claim_ceiling: "finite_declared_state_partitions_and_upstream_decision_signatures_only; no_probability_or_physical_validity_claim".to_owned(),
        ranking_policy: "lexicographic: worst_case_decision_signature_classes ASC; worst_case_decision_discordant_pairs_remaining ASC; worst_case_remaining_states ASC; decision_discordant_pairs_separated DESC; action_id ASC. Cost and duration are reported separately, not scalarized.".to_owned(),
        state_count,
        candidate_count: candidate_set.len(),
        decision_signature_classes_before_observation: signature_counts.len(),
        state_bindings,
        actions_ranked,
    })
}

fn validate_evaluation_bound(action_count: usize, state_count: usize) -> Result<usize, String> {
    let evaluations = action_count
        .checked_mul(state_count)
        .ok_or_else(|| "state-action evaluation count overflow".to_owned())?;
    if evaluations > MAX_STATE_ACTION_EVALUATIONS {
        return Err(format!(
            "state-action evaluation count {evaluations} exceeds limit {MAX_STATE_ACTION_EVALUATIONS}"
        ));
    }
    Ok(evaluations)
}

fn validate_signature(
    signature: &DecisionSignature,
    candidate_set: &BTreeSet<String>,
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
        if let Some(outside) = values.difference(candidate_set).next() {
            return Err(format!("{label} references unknown candidate {outside:?}"));
        }
    }

    if !feasible.is_disjoint(&infeasible)
        || !feasible.is_disjoint(&unknown)
        || !infeasible.is_disjoint(&unknown)
    {
        return Err("feasible, infeasible, and unknown candidate classes must be disjoint".to_owned());
    }
    let classified: BTreeSet<String> = feasible
        .union(&infeasible)
        .cloned()
        .chain(unknown.iter().cloned())
        .collect();
    if &classified != candidate_set {
        let missing: Vec<String> = candidate_set.difference(&classified).cloned().collect();
        return Err(format!(
            "each candidate must be classified feasible, infeasible, or unknown exactly once; missing {:?}",
            missing
        ));
    }
    if let Some(outside) = frontier.difference(&feasible).next() {
        return Err(format!(
            "frontier candidate {outside:?} is not classified feasible in this state"
        ));
    }
    Ok(())
}

fn normalized_signature(signature: &DecisionSignature) -> DecisionSignature {
    let mut normalized = signature.clone();
    normalized.feasible_candidates.sort();
    normalized.infeasible_candidates.sort();
    normalized.unknown_candidates.sort();
    normalized.frontier_candidates.sort();
    normalized
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
        return Err(format!("{label} must be a lowercase 64-character SHA-256 hex digest"));
    }
    Ok(())
}

fn validate_estimate(estimate: Option<&Estimate>, label: &str) -> Result<(), String> {
    if let Some(value) = estimate {
        validate_name(&value.unit, &format!("{label}.unit"))?;
        validate_name(&value.basis, &format!("{label}.basis"))?;
    }
    Ok(())
}

fn choose2(value: u64) -> u64 {
    if value < 2 {
        0
    } else {
        value * (value - 1) / 2
    }
}

fn discordant_pairs(class_counts: &BTreeMap<String, u64>) -> u64 {
    let total: u64 = class_counts.values().sum();
    let all_pairs = choose2(total);
    let same_signature_pairs: u64 = class_counts.values().map(|count| choose2(*count)).sum();
    all_pairs - same_signature_pairs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn signature(feasible: &[&str], infeasible: &[&str], unknown: &[&str], frontier: &[&str]) -> DecisionSignature {
        DecisionSignature {
            feasible_candidates: feasible.iter().map(|value| (*value).to_owned()).collect(),
            infeasible_candidates: infeasible.iter().map(|value| (*value).to_owned()).collect(),
            unknown_candidates: unknown.iter().map(|value| (*value).to_owned()).collect(),
            frontier_candidates: frontier.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn outcome(id: &str, states: &[&str]) -> ActionOutcome {
        ActionOutcome {
            outcome_id: id.to_owned(),
            state_ids: states.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn base_input() -> PlannerInput {
        PlannerInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            world_id: "unit-world".to_owned(),
            candidate_ids: vec!["A".to_owned(), "B".to_owned(), "C".to_owned()],
            state_signatures: vec![
                StateSignature {
                    state_id: "s1".to_owned(),
                    signature: signature(&["B"], &["A", "C"], &[], &["B"]),
                    upstream_case_ref: "fixture#s1".to_owned(),
                    upstream_report_sha256: None,
                },
                StateSignature {
                    state_id: "s2".to_owned(),
                    signature: signature(&["A", "B"], &["C"], &[], &["A", "B"]),
                    upstream_case_ref: "fixture#s2".to_owned(),
                    upstream_report_sha256: None,
                },
                StateSignature {
                    state_id: "s3".to_owned(),
                    signature: signature(&["B", "C"], &["A"], &[], &["B", "C"]),
                    upstream_case_ref: "fixture#s3".to_owned(),
                    upstream_report_sha256: None,
                },
                StateSignature {
                    state_id: "s4".to_owned(),
                    signature: signature(&["B"], &["A", "C"], &[], &["B"]),
                    upstream_case_ref: "fixture#s4".to_owned(),
                    upstream_report_sha256: None,
                },
            ],
            actions: vec![
                MeasurementAction {
                    action_id: "broad-survey".to_owned(),
                    description: "Separate dry-like and wet-like states".to_owned(),
                    outcome_model_ref: "synthetic-partition-model:broad-survey".to_owned(),
                    outcome_model_sha256: None,
                    outcomes: vec![outcome("dry-like", &["s1", "s2"]), outcome("wet-like", &["s3", "s4"])],
                    estimated_cost: Some(Estimate {
                        value: 2,
                        unit: "crew-hour".to_owned(),
                        basis: "synthetic fixture estimate".to_owned(),
                    }),
                    estimated_duration: Some(Estimate {
                        value: 1,
                        unit: "day".to_owned(),
                        basis: "synthetic fixture estimate".to_owned(),
                    }),
                },
                MeasurementAction {
                    action_id: "targeted-survey".to_owned(),
                    description: "Separate each declared state".to_owned(),
                    outcome_model_ref: "synthetic-partition-model:targeted-survey".to_owned(),
                    outcome_model_sha256: None,
                    outcomes: vec![
                        outcome("o1", &["s1"]),
                        outcome("o2", &["s2"]),
                        outcome("o3", &["s3"]),
                        outcome("o4", &["s4"]),
                    ],
                    estimated_cost: Some(Estimate {
                        value: 8,
                        unit: "crew-hour".to_owned(),
                        basis: "synthetic fixture estimate".to_owned(),
                    }),
                    estimated_duration: Some(Estimate {
                        value: 2,
                        unit: "day".to_owned(),
                        basis: "synthetic fixture estimate".to_owned(),
                    }),
                },
                MeasurementAction {
                    action_id: "cheap-check".to_owned(),
                    description: "Low-cost broad split".to_owned(),
                    outcome_model_ref: "synthetic-partition-model:cheap-check".to_owned(),
                    outcome_model_sha256: None,
                    outcomes: vec![outcome("match", &["s1", "s4"]), outcome("other", &["s2", "s3"])],
                    estimated_cost: Some(Estimate {
                        value: 1,
                        unit: "crew-hour".to_owned(),
                        basis: "synthetic fixture estimate".to_owned(),
                    }),
                    estimated_duration: None,
                },
            ],
        }
    }

    fn fixture_digest(input: &PlannerInput) -> String {
        sha256_hex(&serde_json::to_vec(input).expect("fixture input serializes"))
    }

    fn signature_key(signature: &DecisionSignature) -> String {
        serde_json::to_string(&normalized_signature(signature)).expect("signature serializes")
    }

    #[test]
    fn frozen_fixture_is_valid_and_ranked_deterministically() {
        let input: PlannerInput = serde_json::from_str(include_str!("../fixtures/decision-impact-v0.1.json"))
            .expect("frozen input parses");
        let report = plan(&input, &fixture_digest(&input)).expect("valid fixture plans");
        assert_eq!(report.state_count, 4);
        assert_eq!(report.candidate_count, 3);
        assert_eq!(report.decision_signature_classes_before_observation, 3);
        assert_eq!(report.actions_ranked[0].action_id, "targeted-survey");
        assert_eq!(report.actions_ranked[0].impact.worst_case_remaining_states, 1);
        assert_eq!(report.actions_ranked[0].impact.worst_case_decision_signature_classes, 1);
    }

    #[test]
    fn same_input_and_bytes_replay_identically() {
        let input = base_input();
        let digest = fixture_digest(&input);
        assert_eq!(plan(&input, &digest), plan(&input, &digest));
    }

    #[test]
    fn signature_list_permutation_does_not_change_metrics_or_ranking() {
        let mut first = base_input();
        let mut second = first.clone();
        second.state_signatures.reverse();
        second.candidate_ids.reverse();
        for state in &mut second.state_signatures {
            state.signature.feasible_candidates.reverse();
            state.signature.infeasible_candidates.reverse();
            state.signature.frontier_candidates.reverse();
        }
        second.actions.reverse();
        for action in &mut second.actions {
            action.outcomes.reverse();
            for item in &mut action.outcomes {
                item.state_ids.reverse();
            }
        }
        let left = plan(&first, &fixture_digest(&first)).expect("first plan");
        let right = plan(&second, &fixture_digest(&second)).expect("second plan");
        assert_eq!(
            left.actions_ranked.iter().map(|item| (&item.action_id, &item.impact)).collect::<Vec<_>>(),
            right.actions_ranked.iter().map(|item| (&item.action_id, &item.impact)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_missing_state_from_a_partition() {
        let mut input = base_input();
        input.actions[0].outcomes[0].state_ids.pop();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("does not cover every state"));
    }

    #[test]
    fn rejects_state_duplicated_across_outcomes() {
        let mut input = base_input();
        input.actions[0].outcomes[1].state_ids.push("s1".to_owned());
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("more than one outcome"));
    }

    #[test]
    fn rejects_unknown_state_reference() {
        let mut input = base_input();
        input.actions[0].outcomes[0].state_ids.push("not-a-state".to_owned());
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("unknown state"));
    }

    #[test]
    fn rejects_empty_outcome() {
        let mut input = base_input();
        input.actions[0].outcomes[0].state_ids.clear();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("has no states"));
    }

    #[test]
    fn rejects_duplicate_action_and_outcome_ids() {
        let mut input = base_input();
        input.actions[1].action_id = input.actions[0].action_id.clone();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("duplicate action_id"));
        let mut input = base_input();
        input.actions[0].outcomes[1].outcome_id = input.actions[0].outcomes[0].outcome_id.clone();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("duplicate outcome_id"));
    }

    #[test]
    fn rejects_bad_upstream_digest_and_bad_input_digest() {
        let mut input = base_input();
        input.state_signatures[0].upstream_report_sha256 = Some("NOT-A-DIGEST".to_owned());
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("upstream_report_sha256"));
        let input = base_input();
        assert!(plan(&input, "invalid").unwrap_err().contains("input_sha256"));
    }

    #[test]
    fn rejects_signature_class_overlap_or_missing_candidate() {
        let mut input = base_input();
        input.state_signatures[0].signature.unknown_candidates.push("B".to_owned());
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("must be disjoint"));
        let mut input = base_input();
        input.state_signatures[0].signature.infeasible_candidates.pop();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("must be classified"));
    }

    #[test]
    fn rejects_frontier_candidate_not_feasible() {
        let mut input = base_input();
        input.state_signatures[0].signature.frontier_candidates = vec!["A".to_owned()];
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("not classified feasible"));
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let mut value = serde_json::to_value(base_input()).expect("serialize fixture");
        value.as_object_mut().expect("object").insert("magic_probability".to_owned(), serde_json::json!(0.99));
        assert!(serde_json::from_value::<PlannerInput>(value).is_err());
    }

    fn independent_pair_oracle(input: &PlannerInput, action: &MeasurementAction) -> (usize, usize, u64, u64) {
        let by_id: BTreeMap<&str, &DecisionSignature> = input
            .state_signatures
            .iter()
            .map(|state| (state.state_id.as_str(), &state.signature))
            .collect();
        let mut max_states = 0usize;
        let mut max_classes = 0usize;
        let mut max_discordant_pairs = 0u64;
        for outcome in &action.outcomes {
            max_states = max_states.max(outcome.state_ids.len());
            let keys: Vec<String> = outcome
                .state_ids
                .iter()
                .map(|state_id| signature_key(by_id[state_id.as_str()]))
                .collect();
            let distinct: BTreeSet<String> = keys.iter().cloned().collect();
            max_classes = max_classes.max(distinct.len());
            let mut within_discordant = 0u64;
            for left in 0..keys.len() {
                for right in (left + 1)..keys.len() {
                    if keys[left] != keys[right] {
                        within_discordant += 1;
                    }
                }
            }
            max_discordant_pairs = max_discordant_pairs.max(within_discordant);
        }

        let mut global_discordant_pairs = 0u64;
        for left in 0..input.state_signatures.len() {
            for right in (left + 1)..input.state_signatures.len() {
                if signature_key(&input.state_signatures[left].signature)
                    != signature_key(&input.state_signatures[right].signature)
                {
                    global_discordant_pairs += 1;
                }
            }
        }
        let remaining: u64 = action
            .outcomes
            .iter()
            .map(|outcome| {
                let keys: Vec<String> = outcome
                    .state_ids
                    .iter()
                    .map(|state_id| signature_key(by_id[state_id.as_str()]))
                    .collect();
                let mut pairs = 0u64;
                for left in 0..keys.len() {
                    for right in (left + 1)..keys.len() {
                        if keys[left] != keys[right] {
                            pairs += 1;
                        }
                    }
                }
                pairs
            })
            .sum();
        let separated_pairs = global_discordant_pairs - remaining;
        (max_states, max_classes, max_discordant_pairs, separated_pairs)
    }

    fn generated_input(world: usize, seed: u64) -> PlannerInput {
        let seed_index = seed as usize;
        let candidate_ids = vec!["A".to_owned(), "B".to_owned(), "C".to_owned(), "D".to_owned()];
        let mut state_signatures = Vec::new();
        for state_index in 0..20usize {
            let mut feasible = Vec::new();
            let mut infeasible = Vec::new();
            let mut unknown = Vec::new();
            let mut frontier = Vec::new();
            for (candidate_index, candidate) in candidate_ids.iter().enumerate() {
                match (seed_index % 3 + state_index * 2 + candidate_index * 7) % 3 {
                    0 => {
                        feasible.push(candidate.clone());
                        if (seed_index + state_index + candidate_index) % 2 == 0 {
                            frontier.push(candidate.clone());
                        }
                    }
                    1 => infeasible.push(candidate.clone()),
                    _ => unknown.push(candidate.clone()),
                }
            }
            let state_id = format!("w{world:03}-s{state_index:02}");
            state_signatures.push(StateSignature {
                state_id: state_id.clone(),
                signature: DecisionSignature {
                    feasible_candidates: feasible,
                    infeasible_candidates: infeasible,
                    unknown_candidates: unknown,
                    frontier_candidates: frontier,
                },
                upstream_case_ref: format!("synthetic:w{world:03}#{state_id}"),
                upstream_report_sha256: None,
            });
        }

        let mut actions = Vec::new();
        for action_index in 0..5usize {
            let outcome_count = 1 + ((seed_index + action_index) % 5);
            let mut outcome_states = vec![Vec::new(); outcome_count];
            for state_index in 0..20usize {
                let bucket = (state_index + seed_index * (action_index + 1)) % outcome_count;
                outcome_states[bucket].push(format!("w{world:03}-s{state_index:02}"));
            }
            let outcomes = outcome_states
                .iter()
                .enumerate()
                .map(|(index, states)| ActionOutcome {
                    outcome_id: format!("o{index:02}"),
                    state_ids: states.clone(),
                })
                .collect();
            actions.push(MeasurementAction {
                action_id: format!("measure-{action_index}"),
                description: format!("synthetic measurement action {action_index}"),
                outcome_model_ref: format!("synthetic-generator:seeded-modular-partition-v1:world{world}:action{action_index}"),
                outcome_model_sha256: None,
                outcomes,
                estimated_cost: Some(Estimate {
                    value: 1 + action_index as u64,
                    unit: "crew-hour".to_owned(),
                    basis: "synthetic generator parameter; not measured".to_owned(),
                }),
                estimated_duration: Some(Estimate {
                    value: 30 * (1 + action_index as u64),
                    unit: "minute".to_owned(),
                    basis: "synthetic generator parameter; not measured".to_owned(),
                }),
            });
        }

        PlannerInput {
            schema_version: SCHEMA_VERSION.to_owned(),
            world_id: format!("generated-world-{world:03}"),
            candidate_ids,
            state_signatures,
            actions,
        }
    }

    #[test]
    fn generated_suite_matches_independent_pairwise_oracle_and_emits_receipts() {
        let mut cases = Vec::new();
        let mut total_state_action_evaluations = 0usize;
        for world in 0..100usize {
            let seed = 20261017u64 + world as u64;
            let input = generated_input(world, seed);
            let bytes = serde_json::to_vec(&input).expect("generated input serializes");
            let input_digest = sha256_hex(&bytes);
            let report = plan(&input, &input_digest).expect("generated world validates");
            total_state_action_evaluations += input.state_signatures.len() * input.actions.len();

            let mut oracle_action_summaries = Vec::new();
            let mut oracle_ranking = Vec::new();
            for action in &input.actions {
                let (max_states, max_classes, max_discordant_pairs, separated_pairs) =
                    independent_pair_oracle(&input, action);
                let assessed = report
                    .actions_ranked
                    .iter()
                    .find(|item| item.action_id == action.action_id)
                    .expect("action appears in report");
                assert_eq!(assessed.impact.worst_case_remaining_states, max_states);
                assert_eq!(assessed.impact.worst_case_decision_signature_classes, max_classes);
                assert_eq!(assessed.impact.worst_case_decision_discordant_pairs_remaining, max_discordant_pairs);
                assert_eq!(assessed.impact.decision_discordant_pairs_separated, separated_pairs);
                oracle_ranking.push((
                    action.action_id.clone(),
                    max_classes,
                    max_discordant_pairs,
                    max_states,
                    separated_pairs,
                ));
                oracle_action_summaries.push(serde_json::json!({
                    "action_id": action.action_id,
                    "worst_case_remaining_states": max_states,
                    "worst_case_decision_signature_classes": max_classes,
                    "worst_case_decision_discordant_pairs_remaining": max_discordant_pairs,
                    "decision_discordant_pairs_separated": separated_pairs
                }));
            }
            oracle_ranking.sort_by(|left, right| {
                left.1.cmp(&right.1)
                    .then_with(|| left.2.cmp(&right.2))
                    .then_with(|| left.3.cmp(&right.3))
                    .then_with(|| right.4.cmp(&left.4))
                    .then_with(|| left.0.cmp(&right.0))
            });
            let oracle_action_ids: Vec<String> =
                oracle_ranking.into_iter().map(|summary| summary.0).collect();
            let evaluated_action_ids: Vec<String> = report
                .actions_ranked
                .iter()
                .map(|assessment| assessment.action_id.clone())
                .collect();
            assert_eq!(evaluated_action_ids, oracle_action_ids);
            cases.push(serde_json::json!({
                "world_index": world,
                "seed": seed,
                "input_sha256": input_digest,
                "input": input,
                "oracle_action_summaries": oracle_action_summaries,
                "report": report
            }));
        }
        assert_eq!(total_state_action_evaluations, 10_000);

        if let Ok(path) = env::var("CIV_INFRA_DECISION_CORPUS_OUT") {
            let artifact = serde_json::json!({
                "corpus_version": "civ-infra-decision-impact-generated/v0.1",
                "generator": "seeded-modular-partition-v1",
                "seed_base": 20261017,
                "world_count": 100,
                "states_per_world": 20,
                "actions_per_world": 5,
                "state_action_evaluations": total_state_action_evaluations,
                "claim_ceiling": "synthetic finite-state software oracle only",
                "cases": cases
            });
            let bytes = serde_json::to_vec_pretty(&artifact).expect("artifact serializes");
            fs::write(path, bytes).expect("write generated decision-planning corpus artifact");
        }
    }

    #[test]
    fn missing_and_duplicate_state_records_fail_closed() {
        let mut input = base_input();
        input.state_signatures.pop();
        assert!(plan(&input, &fixture_digest(&input)).is_err());
        let mut input = base_input();
        input.state_signatures[1].state_id = input.state_signatures[0].state_id.clone();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("duplicate state_id"));
    }

    #[test]
    fn generator_seed_changes_signatures_and_worlds_have_multiple_classes() {
        let first = generated_input(0, 20261017);
        let second = generated_input(0, 20261018);
        let first_keys: Vec<String> = first
            .state_signatures
            .iter()
            .map(|state| signature_key(&state.signature))
            .collect();
        let second_keys: Vec<String> = second
            .state_signatures
            .iter()
            .map(|state| signature_key(&state.signature))
            .collect();
        assert_ne!(first_keys, second_keys);
        assert!(first_keys.iter().collect::<BTreeSet<_>>().len() > 1);
        assert!(second_keys.iter().collect::<BTreeSet<_>>().len() > 1);
    }

    #[test]
    fn state_action_work_bound_accepts_limit_and_rejects_overflow() {
        assert_eq!(
            validate_evaluation_bound(1000, 10_000).expect("exact limit is accepted"),
            MAX_STATE_ACTION_EVALUATIONS
        );
        assert!(validate_evaluation_bound(1001, 10_000)
            .unwrap_err()
            .contains("exceeds limit"));
        assert!(validate_evaluation_bound(usize::MAX, 2)
            .unwrap_err()
            .contains("overflow"));
    }

    #[test]
    fn outcome_model_provenance_is_required_and_digest_is_checked() {
        let mut input = base_input();
        input.actions[0].outcome_model_ref = " ".to_owned();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("outcome_model_ref"));
        let mut input = base_input();
        input.actions[0].outcome_model_sha256 = Some("not-a-digest".to_owned());
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("outcome_model_sha256"));
    }

    #[test]
    fn rejects_unsupported_schema_and_negative_empty_names() {
        let mut input = base_input();
        input.schema_version = "v9".to_owned();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("unsupported schema_version"));
        let mut input = base_input();
        input.actions[0].action_id = "  ".to_owned();
        assert!(plan(&input, &fixture_digest(&input)).unwrap_err().contains("must not be empty"));
    }
}
