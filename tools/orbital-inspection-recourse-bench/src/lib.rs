use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const CORPUS_SCHEMA: &str = "luminous.orbital-inspection-recourse.corpus.v0.1";
pub const RECEIPT_SCHEMA: &str = "luminous.orbital-inspection-recourse.receipt.v0.1";
pub const BENCHMARK_ID: &str = "ORBITAL-INSPECTION-RECOURSE-001";
pub const CLAIM_CEILING: &str =
    "synthetic_finite_contingent_policy_over_declared_outcome_support_only";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GateState {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MappingState {
    Known,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PolicyStatus {
    Resolved,
    Abstain,
    SearchLimitNotProven,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GateEvidence {
    pub gate_id: String,
    pub state: GateState,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub world_id: String,
    pub decision_class: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldOutcomeSupport {
    pub world_id: String,
    pub mapping_state: MappingState,
    pub possible_outcomes: Vec<String>,
    pub receipt_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationAction {
    pub action_id: String,
    pub duration_ms: u64,
    pub energy_mj: u64,
    pub model_receipt_sha256: String,
    pub gates: Vec<GateEvidence>,
    pub outcomes_by_world: Vec<WorldOutcomeSupport>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub worlds: Vec<World>,
    pub actions: Vec<ObservationAction>,
    pub max_actions: u32,
    pub max_search_states: u64,
    pub budget_time_ms: u64,
    pub budget_energy_mj: u64,
    pub expected_status: PolicyStatus,
    pub expected_root_action: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub benchmark_id: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyBranch {
    pub outcome: String,
    pub child: PolicyNode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyNode {
    pub terminal_decision: Option<String>,
    pub action_id: Option<String>,
    pub survivor_world_count: usize,
    pub action_evidence_sha256: Vec<String>,
    pub branches: Vec<PolicyBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyObjective {
    pub worst_case_duration_ms: u64,
    pub worst_case_energy_mj: u64,
    pub worst_case_action_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicyPlan {
    pub root: PolicyNode,
    pub objective: PolicyObjective,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: PolicyStatus,
    pub initial_world_count: usize,
    pub initial_decision_class_count: usize,
    pub search_states_expanded: u64,
    pub exactness_proven: bool,
    pub rejected_actions: BTreeMap<String, Vec<String>>,
    pub policy: Option<PolicyPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Receipt {
    pub schema: String,
    pub benchmark_id: String,
    pub source_commit: String,
    pub input_sha256: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub case_count: usize,
    pub expected_outcome_matches: usize,
    pub expected_outcome_mismatches: usize,
    pub authority_granted: bool,
    pub physical_validation: String,
    pub cases: Vec<CaseResult>,
}

#[derive(Clone)]
struct SolvedPolicy {
    node: PolicyNode,
    objective: PolicyObjective,
}

impl PolicyObjective {
    fn key(&self) -> (u64, u64, u32) {
        (
            self.worst_case_duration_ms,
            self.worst_case_energy_mj,
            self.worst_case_action_count,
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_case(case: &Case) -> Result<(), String> {
    if !valid_text(&case.case_id)
        || case.worlds.is_empty()
        || case.max_search_states == 0
        || case.max_actions > 32
        || case.worlds.len() > 64
        || case.actions.len() > 64
    {
        return Err("CASE_LIMIT_OR_REQUIRED_COLLECTION_INVALID".to_owned());
    }

    let mut world_ids = BTreeSet::new();
    for world in &case.worlds {
        if !valid_text(&world.world_id)
            || !valid_text(&world.decision_class)
            || !world_ids.insert(world.world_id.clone())
        {
            return Err("WORLD_ID_OR_DECISION_INVALID".to_owned());
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &case.actions {
        if !valid_text(&action.action_id)
            || !action_ids.insert(action.action_id.clone())
            || !valid_sha256(&action.model_receipt_sha256)
            || action.gates.is_empty()
        {
            return Err("ACTION_ID_OR_MODEL_RECEIPT_INVALID".to_owned());
        }

        let mut gate_ids = BTreeSet::new();
        for gate in &action.gates {
            if !valid_text(&gate.gate_id)
                || !valid_sha256(&gate.receipt_sha256)
                || !gate_ids.insert(gate.gate_id.clone())
            {
                return Err("ACTION_GATE_EVIDENCE_INVALID".to_owned());
            }
        }

        let mut mapped_world_ids = BTreeSet::new();
        for mapping in &action.outcomes_by_world {
            if !world_ids.contains(&mapping.world_id)
                || !mapped_world_ids.insert(mapping.world_id.clone())
                || !valid_sha256(&mapping.receipt_sha256)
            {
                return Err("ACTION_WORLD_MAPPING_INVALID".to_owned());
            }
            let mut outcomes = BTreeSet::new();
            match mapping.mapping_state {
                MappingState::Known => {
                    if mapping.possible_outcomes.is_empty()
                        || mapping.possible_outcomes.iter().any(|outcome| {
                            !valid_text(outcome) || !outcomes.insert(outcome.clone())
                        })
                    {
                        return Err("KNOWN_OUTCOME_SUPPORT_INVALID".to_owned());
                    }
                }
                MappingState::Unknown => {
                    if !mapping.possible_outcomes.is_empty() {
                        return Err("UNKNOWN_MAPPING_MUST_NOT_ASSERT_OUTCOMES".to_owned());
                    }
                }
            }
        }
        if mapped_world_ids != world_ids {
            return Err("ACTION_WORLD_MAPPING_INCOMPLETE".to_owned());
        }
    }
    Ok(())
}

fn decision_classes(case: &Case, survivors: &BTreeSet<String>) -> BTreeSet<String> {
    case.worlds
        .iter()
        .filter(|world| survivors.contains(&world.world_id))
        .map(|world| world.decision_class.clone())
        .collect()
}

fn action_rejections(case: &Case) -> BTreeMap<String, Vec<String>> {
    let mut rejected = BTreeMap::new();
    for action in &case.actions {
        let mut reasons = BTreeSet::new();
        for gate in &action.gates {
            match gate.state {
                GateState::Pass => {}
                GateState::Fail => {
                    reasons.insert(format!("GATE_FAIL:{}", gate.gate_id));
                }
                GateState::Unknown => {
                    reasons.insert(format!("GATE_UNKNOWN:{}", gate.gate_id));
                }
            }
        }
        if action
            .outcomes_by_world
            .iter()
            .any(|mapping| mapping.mapping_state == MappingState::Unknown)
        {
            reasons.insert("OUTCOME_MODEL_UNKNOWN".to_owned());
        }
        if !reasons.is_empty() {
            rejected.insert(action.action_id.clone(), reasons.into_iter().collect());
        }
    }
    rejected
}

fn action_is_admissible(action: &ObservationAction) -> bool {
    action
        .gates
        .iter()
        .all(|gate| gate.state == GateState::Pass)
        && action
            .outcomes_by_world
            .iter()
            .all(|mapping| mapping.mapping_state == MappingState::Known)
}

fn solve_state(
    case: &Case,
    survivors: &BTreeSet<String>,
    used_actions: &BTreeSet<String>,
    remaining_time_ms: u64,
    remaining_energy_mj: u64,
    remaining_actions: u32,
    states_expanded: &mut u64,
    cap_hit: &mut bool,
) -> Option<SolvedPolicy> {
    if *cap_hit {
        return None;
    }
    *states_expanded = states_expanded.saturating_add(1);
    if *states_expanded > case.max_search_states {
        *cap_hit = true;
        return None;
    }

    let decisions = decision_classes(case, survivors);
    if decisions.len() == 1 {
        return Some(SolvedPolicy {
            node: PolicyNode {
                terminal_decision: decisions.iter().next().cloned(),
                action_id: None,
                survivor_world_count: survivors.len(),
                action_evidence_sha256: Vec::new(),
                branches: Vec::new(),
            },
            objective: PolicyObjective {
                worst_case_duration_ms: 0,
                worst_case_energy_mj: 0,
                worst_case_action_count: 0,
            },
        });
    }
    if decisions.is_empty() || remaining_actions == 0 {
        return None;
    }

    let mut ordered_actions: Vec<&ObservationAction> = case.actions.iter().collect();
    ordered_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    let mut best: Option<SolvedPolicy> = None;

    for action in ordered_actions {
        if used_actions.contains(&action.action_id)
            || !action_is_admissible(action)
            || action.duration_ms > remaining_time_ms
            || action.energy_mj > remaining_energy_mj
        {
            continue;
        }

        let mut outcomes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for mapping in &action.outcomes_by_world {
            if !survivors.contains(&mapping.world_id) {
                continue;
            }
            for outcome in &mapping.possible_outcomes {
                outcomes
                    .entry(outcome.clone())
                    .or_default()
                    .insert(mapping.world_id.clone());
            }
        }
        if outcomes.len() < 2 || outcomes.values().all(|branch| branch == survivors) {
            continue;
        }

        let mut next_used = used_actions.clone();
        next_used.insert(action.action_id.clone());
        let mut branches = Vec::new();
        let mut worst_child_time = 0_u64;
        let mut worst_child_energy = 0_u64;
        let mut worst_child_actions = 0_u32;
        let mut complete = true;

        for (outcome, branch_worlds) in outcomes {
            let child = solve_state(
                case,
                &branch_worlds,
                &next_used,
                remaining_time_ms - action.duration_ms,
                remaining_energy_mj - action.energy_mj,
                remaining_actions - 1,
                states_expanded,
                cap_hit,
            );
            let Some(child) = child else {
                complete = false;
                break;
            };
            worst_child_time = worst_child_time.max(child.objective.worst_case_duration_ms);
            worst_child_energy = worst_child_energy.max(child.objective.worst_case_energy_mj);
            worst_child_actions = worst_child_actions.max(child.objective.worst_case_action_count);
            branches.push(PolicyBranch {
                outcome,
                child: child.node,
            });
        }

        if *cap_hit {
            return None;
        }
        if !complete || branches.len() < 2 {
            continue;
        }

        let mut evidence = BTreeSet::new();
        evidence.insert(action.model_receipt_sha256.clone());
        for gate in &action.gates {
            evidence.insert(gate.receipt_sha256.clone());
        }
        for mapping in &action.outcomes_by_world {
            if survivors.contains(&mapping.world_id) {
                evidence.insert(mapping.receipt_sha256.clone());
            }
        }

        let candidate = SolvedPolicy {
            node: PolicyNode {
                terminal_decision: None,
                action_id: Some(action.action_id.clone()),
                survivor_world_count: survivors.len(),
                action_evidence_sha256: evidence.into_iter().collect(),
                branches,
            },
            objective: PolicyObjective {
                worst_case_duration_ms: action.duration_ms.saturating_add(worst_child_time),
                worst_case_energy_mj: action.energy_mj.saturating_add(worst_child_energy),
                worst_case_action_count: 1_u32.saturating_add(worst_child_actions),
            },
        };

        let replace = best
            .as_ref()
            .is_none_or(|current| candidate.objective.key() < current.objective.key());
        if replace {
            best = Some(candidate);
        }
    }
    best
}

fn evaluate_case(case: &Case) -> Result<(CaseResult, Option<String>), String> {
    validate_case(case)?;
    let survivors: BTreeSet<String> = case
        .worlds
        .iter()
        .map(|world| world.world_id.clone())
        .collect();
    let initial_decisions = decision_classes(case, &survivors);
    let rejected_actions = action_rejections(case);
    let mut states_expanded = 0_u64;
    let mut cap_hit = false;
    let policy = solve_state(
        case,
        &survivors,
        &BTreeSet::new(),
        case.budget_time_ms,
        case.budget_energy_mj,
        case.max_actions,
        &mut states_expanded,
        &mut cap_hit,
    );

    let (status, exactness_proven, plan) = if cap_hit {
        (PolicyStatus::SearchLimitNotProven, false, None)
    } else if let Some(policy) = policy {
        (
            PolicyStatus::Resolved,
            true,
            Some(PolicyPlan {
                root: policy.node,
                objective: policy.objective,
            }),
        )
    } else {
        (PolicyStatus::Abstain, true, None)
    };
    let root_action = plan.as_ref().and_then(|plan| plan.root.action_id.clone());
    Ok((
        CaseResult {
            case_id: case.case_id.clone(),
            status,
            initial_world_count: survivors.len(),
            initial_decision_class_count: initial_decisions.len(),
            search_states_expanded: states_expanded,
            exactness_proven,
            rejected_actions,
            policy: plan,
        },
        root_action,
    ))
}

pub fn audit_corpus(bytes: &[u8], source_commit: &str) -> Result<Receipt, String> {
    if source_commit.len() != 40 || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SOURCE_COMMIT_MUST_BE_40_HEX_CHARACTERS".to_owned());
    }
    let corpus: Corpus =
        serde_json::from_slice(bytes).map_err(|error| format!("CORPUS_JSON_INVALID: {error}"))?;
    if corpus.schema != CORPUS_SCHEMA
        || corpus.benchmark_id != BENCHMARK_ID
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > 128
    {
        return Err("CORPUS_IDENTITY_OR_CLAIM_CEILING_INVALID".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::new();
    let mut expected_outcome_matches = 0_usize;
    for case in &corpus.cases {
        if !case_ids.insert(&case.case_id) {
            return Err("CASE_ID_DUPLICATE".to_owned());
        }
        let (result, root_action) = evaluate_case(case)?;
        if result.status == case.expected_status && root_action == case.expected_root_action {
            expected_outcome_matches += 1;
        }
        cases.push(result);
    }

    Ok(Receipt {
        schema: RECEIPT_SCHEMA.to_owned(),
        benchmark_id: BENCHMARK_ID.to_owned(),
        source_commit: source_commit.to_ascii_lowercase(),
        input_sha256: sha256_hex(bytes),
        evidence_class: corpus.evidence_class,
        claim_ceiling: corpus.claim_ceiling,
        case_count: cases.len(),
        expected_outcome_matches,
        expected_outcome_mismatches: cases.len() - expected_outcome_matches,
        authority_granted: false,
        physical_validation: "NOT_ASSESSED".to_owned(),
        cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus_bytes() -> &'static [u8] {
        include_bytes!(
            "../../../docs/research/orbital-truss/fixtures/inspection-recourse-v0_1.json"
        )
    }

    #[test]
    fn frozen_corpus_matches_all_expected_policies() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("frozen corpus validates");
        assert_eq!(receipt.case_count, 8);
        assert_eq!(receipt.expected_outcome_matches, 8);
        assert_eq!(receipt.expected_outcome_mismatches, 0);
    }

    #[test]
    fn same_decision_worlds_need_no_extra_observation() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus validates");
        let case = receipt
            .cases
            .iter()
            .find(|case| case.case_id == "IR-003-SAME-DECISION")
            .unwrap();
        assert_eq!(case.status, PolicyStatus::Resolved);
        assert_eq!(
            case.policy
                .as_ref()
                .unwrap()
                .objective
                .worst_case_action_count,
            0
        );
    }

    #[test]
    fn unknown_or_failed_gate_never_becomes_an_action() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus validates");
        let case = receipt
            .cases
            .iter()
            .find(|case| case.case_id == "IR-004-UNKNOWN-GATE")
            .unwrap();
        assert_eq!(case.status, PolicyStatus::Abstain);
        assert!(case.policy.is_none());
        assert!(case.rejected_actions.contains_key("UNQUALIFIED_VIEW"));
    }

    #[test]
    fn overlapping_support_preserves_worlds_in_each_possible_branch() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus validates");
        let case = receipt
            .cases
            .iter()
            .find(|case| case.case_id == "IR-002-CONTINGENT-TREE")
            .unwrap();
        let policy = case.policy.as_ref().expect("policy resolves");
        assert_eq!(policy.root.action_id.as_deref(), Some("VIEW-1"));
        let ambiguous = policy
            .root
            .branches
            .iter()
            .find(|branch| branch.outcome == "AMBIGUOUS")
            .unwrap();
        assert_eq!(ambiguous.child.action_id.as_deref(), Some("VIEW-2"));
    }

    #[test]
    fn search_cap_never_claims_exactness() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus validates");
        let case = receipt
            .cases
            .iter()
            .find(|case| case.case_id == "IR-006-SEARCH-CAP")
            .unwrap();
        assert_eq!(case.status, PolicyStatus::SearchLimitNotProven);
        assert!(!case.exactness_proven);
    }

    #[test]
    fn common_outcome_support_cannot_resolve_opposite_decisions() {
        let receipt = audit_corpus(corpus_bytes(), "0123456789abcdef0123456789abcdef01234567")
            .expect("corpus validates");
        let case = receipt
            .cases
            .iter()
            .find(|case| case.case_id == "IR-005-COMMON-CAUSE")
            .unwrap();
        assert_eq!(case.status, PolicyStatus::Abstain);
    }

    #[test]
    fn input_permutation_does_not_change_canonical_policy() {
        let corpus: Corpus = serde_json::from_slice(corpus_bytes()).unwrap();
        let source = "0123456789abcdef0123456789abcdef01234567";
        let original = audit_corpus(corpus_bytes(), source).unwrap();
        let mut permuted = corpus.clone();
        for case in &mut permuted.cases {
            case.worlds.reverse();
            case.actions.reverse();
            for action in &mut case.actions {
                action.gates.reverse();
                action.outcomes_by_world.reverse();
                for mapping in &mut action.outcomes_by_world {
                    mapping.possible_outcomes.reverse();
                }
            }
        }
        let bytes = serde_json::to_vec(&permuted).unwrap();
        let replay = audit_corpus(&bytes, source).unwrap();
        for (left, right) in original.cases.iter().zip(replay.cases.iter()) {
            assert_eq!(left.status, right.status);
            assert_eq!(left.policy, right.policy);
        }
    }
}
