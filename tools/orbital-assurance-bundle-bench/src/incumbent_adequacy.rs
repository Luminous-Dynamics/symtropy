//! Finite-world incumbent decision adequacy gate.
//!
//! This module decides whether a supplied incumbent remains within a declared
//! finite-world common-action improvement envelope. It never selects or executes a replacement action,
//! estimates probabilities, or grants operational authority.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "luminous.orbital-incumbent-adequacy.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-finite-world-incumbent-adequacy-only";
const MAX_CASES: usize = 128;
const MAX_ACTIONS: usize = 4096;
const MAX_WORLDS: usize = 4096;
const MAX_SCENARIOS: usize = 16384;
const MAX_ROWS: usize = 65536;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const JOINT_LOSS_RECEIPT_SCHEMA: &str = "luminous.orbital-joint-loss-scenario.receipt.v0.1";
const JOINT_LOSS_CLAIM_CEILING: &str = "finite-joint-loss-scenario-regret-algebra-only";
const WORLD_SET_RECEIPT_SCHEMA: &str = "luminous.orbital-decision-identifiability.receipt.v0.1";
const WORLD_SET_CLAIM_CEILING: &str = "exact-finite-world-decision-set-algebra-only";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldLoss {
    pub action_id: String,
    /// Exact source rational from the upstream joint loss-scenario receipt.
    pub source_numerator: u64,
    pub source_denominator: u64,
    /// Exact integer projection into the declared case loss unit.
    pub loss: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct World {
    /// Unique joint physical-world / loss-model scenario ID.
    pub scenario_id: String,
    /// Physical/observation world ID. Multiple loss models may share this ID.
    pub world_id: String,
    pub loss_model_id: String,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
    pub loss_unit: String,
    pub scope_sha256: String,
    pub feasible_action_ids: Vec<String>,
    pub losses: Vec<WorldLoss>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LossModelSetEvidence {
    pub state: String,
    pub source_receipt_schema: String,
    pub source_claim_ceiling: String,
    pub source_commit: String,
    pub source_receipt_sha256: String,
    pub source_case_id: String,
    pub source_manifest_id: String,
    /// Digest of the canonical loss-scenario records in the upstream #6939 receipt.
    pub source_scenarios_sha256: String,
    pub source_loss_unit: String,
    pub projection_id: String,
    pub projection_scale_numerator: u64,
    pub projection_scale_denominator: u64,
    /// Exact joint scenario IDs planned by the loss-model manifest.
    pub expected_scenario_ids: Vec<String>,
    pub expected_scenario_ids_sha256: String,
    /// Digest of the projected scenario rows consumed by this incumbent evaluator.
    pub projected_scenarios_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Expected {
    pub disposition: String,
    pub trigger_reason: String,
    pub worst_case_improvement: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub case_id: String,
    pub scope_sha256: String,
    pub loss_unit: String,
    pub incumbent_action_id: String,
    pub action_ids: Vec<String>,
    pub worlds: Vec<World>,
    pub world_set_state: String,
    pub world_set_receipt_schema: String,
    pub world_set_claim_ceiling: String,
    pub world_set_source_commit: String,
    pub world_set_source_case_id: String,
    pub world_set_receipt_id: String,
    pub world_set_receipt_sha256: String,
    pub expected_world_ids: Vec<String>,
    pub expected_world_ids_sha256: String,
    pub loss_model_set: LossModelSetEvidence,
    pub domain_closure_state: String,
    pub dependency_graph_state: String,
    pub model_family_state: String,
    pub evidence_state: String,
    pub materiality_margin: i64,
    pub switching_cost: i64,
    pub expected: Expected,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<Case>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct WorldAssessment {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub evidence_receipt_id: String,
    pub incumbent_feasible: bool,
    pub incumbent_loss: Option<i64>,
    pub reference_common_action_loss: Option<i64>,
    pub incumbent_minus_reference_loss: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CaseResult {
    pub case_id: String,
    pub disposition: String,
    pub trigger_reason: String,
    pub loss_unit: String,
    pub materiality_margin: i64,
    pub switching_cost: i64,
    pub retention_threshold: Option<i64>,
    pub world_set_receipt_schema: String,
    pub world_set_claim_ceiling: String,
    pub world_set_source_commit: String,
    pub world_set_source_case_id: String,
    pub world_set_receipt_id: String,
    pub world_set_receipt_sha256: String,
    pub world_ids_sha256: String,
    pub world_set_state: String,
    pub loss_model_state: String,
    pub loss_model_source_case_id: String,
    pub loss_model_source_commit: String,
    pub loss_model_source_unit: String,
    pub loss_projection_id: String,
    pub loss_projection_scale_numerator: u64,
    pub loss_projection_scale_denominator: u64,
    pub loss_model_source_receipt_sha256: String,
    pub loss_model_manifest_id: String,
    pub source_loss_scenarios_sha256: String,
    pub projected_scenarios_sha256: String,
    pub joint_scenario_count: usize,
    pub loss_model_ids: Vec<String>,
    pub reference_common_action_id: Option<String>,
    pub current_worst_case_loss: Option<i64>,
    pub reference_common_worst_case_loss: Option<i64>,
    pub worst_case_improvement: Option<i64>,
    pub world_assessments: Vec<WorldAssessment>,
    pub authority_granted: bool,
    pub expected_result_matched: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub retain_incumbent_cases: usize,
    pub reoptimization_warranted_cases: usize,
    pub abstain_cases: usize,
    pub invalid_input_cases: usize,
    pub incumbent_infeasible_cases: usize,
    pub threshold_exceeded_cases: usize,
    pub loss_model_set_blocked_cases: usize,
    pub loss_scenario_evidence_blocked_cases: usize,
    pub joint_scenario_inventory_blocked_cases: usize,
    pub expected_result_mismatches: usize,
    pub authority_grants: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub authority_granted: bool,
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() || byte == b' ')
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode canonical ID inventory: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn exact_projected_loss(
    source_numerator: u64,
    source_denominator: u64,
    scale_numerator: u64,
    scale_denominator: u64,
) -> Option<i64> {
    if source_denominator == 0 || scale_denominator == 0 {
        return None;
    }
    let numerator = u128::from(source_numerator) * u128::from(scale_numerator);
    let denominator = u128::from(source_denominator) * u128::from(scale_denominator);
    if numerator % denominator != 0 {
        return None;
    }
    let projected = numerator / denominator;
    (projected <= i64::MAX as u128).then_some(projected as i64)
}

fn projected_scenarios_digest(scenarios: &[World]) -> Result<String, String> {
    let mut sorted = scenarios.to_vec();
    for scenario in &mut sorted {
        scenario.feasible_action_ids.sort();
        scenario
            .losses
            .sort_by(|left, right| left.action_id.cmp(&right.action_id));
    }
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("cannot encode projected joint scenarios: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn valid_gate_states(case: &Case) -> bool {
    matches!(
        case.world_set_state.as_str(),
        "CURRENT" | "STALE" | "UNAVAILABLE"
    ) && matches!(
        case.loss_model_set.state.as_str(),
        "CURRENT" | "STALE" | "INDETERMINATE" | "OUT_OF_PROFILE" | "INADEQUATE"
    ) && matches!(
        case.domain_closure_state.as_str(),
        "CLOSED_WITHIN_DECLARED_SCOPE" | "OPEN" | "STALE" | "INADEQUATE"
    ) && matches!(
        case.dependency_graph_state.as_str(),
        "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
            | "MISSING_DEPENDENCY_EDGE"
            | "COMMON_MODE_DEPENDENCY_OMISSION"
            | "CONSUMER_MANIFEST_OMISSION"
            | "UNDECLARED_DEPENDENCY_EDGE"
            | "DEPENDENCY_MODEL_INADEQUATE"
            | "DEPENDENCY_SOURCE_UNAVAILABLE"
            | "DEPENDENCY_SCOPE_MISMATCH"
            | "INVALID_GRAPH"
            | "STALE"
            | "INDETERMINATE"
    ) && matches!(case.model_family_state.as_str(), "ADEQUATE" | "INADEQUATE")
        && matches!(
            case.evidence_state.as_str(),
            "CURRENT" | "STALE" | "CONTRADICTED" | "INDETERMINATE"
        )
}

#[derive(Debug, Default)]
struct ComparisonMetrics {
    reference_common_action_id: Option<String>,
    current_worst_case_loss: Option<i64>,
    reference_common_worst_case_loss: Option<i64>,
    worst_case_improvement: Option<i64>,
}

#[derive(Debug)]
struct ValidatedWorld {
    scenario_id: String,
    world_id: String,
    loss_model_id: String,
    evidence_receipt_id: String,
    feasible_action_ids: BTreeSet<String>,
    losses: BTreeMap<String, i64>,
}

fn result(
    case: &Case,
    disposition: &str,
    trigger_reason: &str,
    retention_threshold: Option<i64>,
    metrics: ComparisonMetrics,
    world_assessments: Vec<WorldAssessment>,
) -> CaseResult {
    let expected_result_matched = case.expected.disposition == disposition
        && case.expected.trigger_reason == trigger_reason
        && case.expected.worst_case_improvement == metrics.worst_case_improvement;
    CaseResult {
        case_id: case.case_id.clone(),
        disposition: disposition.to_owned(),
        trigger_reason: trigger_reason.to_owned(),
        loss_unit: case.loss_unit.clone(),
        materiality_margin: case.materiality_margin,
        switching_cost: case.switching_cost,
        retention_threshold,
        world_set_receipt_schema: case.world_set_receipt_schema.clone(),
        world_set_claim_ceiling: case.world_set_claim_ceiling.clone(),
        world_set_source_commit: case.world_set_source_commit.clone(),
        world_set_source_case_id: case.world_set_source_case_id.clone(),
        world_set_receipt_id: case.world_set_receipt_id.clone(),
        world_set_receipt_sha256: case.world_set_receipt_sha256.clone(),
        world_ids_sha256: case.expected_world_ids_sha256.clone(),
        world_set_state: case.world_set_state.clone(),
        loss_model_state: case.loss_model_set.state.clone(),
        loss_model_source_case_id: case.loss_model_set.source_case_id.clone(),
        loss_model_source_commit: case.loss_model_set.source_commit.clone(),
        loss_model_source_unit: case.loss_model_set.source_loss_unit.clone(),
        loss_projection_id: case.loss_model_set.projection_id.clone(),
        loss_projection_scale_numerator: case.loss_model_set.projection_scale_numerator,
        loss_projection_scale_denominator: case.loss_model_set.projection_scale_denominator,
        loss_model_source_receipt_sha256: case.loss_model_set.source_receipt_sha256.clone(),
        loss_model_manifest_id: case.loss_model_set.source_manifest_id.clone(),
        source_loss_scenarios_sha256: case.loss_model_set.source_scenarios_sha256.clone(),
        projected_scenarios_sha256: case.loss_model_set.projected_scenarios_sha256.clone(),
        joint_scenario_count: case.worlds.len(),
        loss_model_ids: case
            .worlds
            .iter()
            .map(|world| world.loss_model_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        reference_common_action_id: metrics.reference_common_action_id,
        current_worst_case_loss: metrics.current_worst_case_loss,
        reference_common_worst_case_loss: metrics.reference_common_worst_case_loss,
        worst_case_improvement: metrics.worst_case_improvement,
        world_assessments,
        authority_granted: false,
        expected_result_matched,
    }
}

fn invalid_input(case: &Case, reason: &str) -> CaseResult {
    result(
        case,
        "INVALID_INPUT",
        reason,
        None,
        ComparisonMetrics::default(),
        Vec::new(),
    )
}

fn abstain(case: &Case, reason: &str, threshold: Option<i64>) -> CaseResult {
    result(
        case,
        "ABSTAIN",
        reason,
        threshold,
        ComparisonMetrics::default(),
        Vec::new(),
    )
}

fn evaluate_case(case: &Case) -> CaseResult {
    if !valid_id(&case.case_id)
        || !valid_id(&case.loss_unit)
        || !valid_id(&case.incumbent_action_id)
        || !valid_sha256(&case.scope_sha256)
        || case.action_ids.is_empty()
        || case.action_ids.len() > MAX_ACTIONS
        || case.expected_world_ids.len() > MAX_WORLDS
        || case.worlds.len() > MAX_SCENARIOS
        || case.loss_model_set.expected_scenario_ids.len() > MAX_SCENARIOS
        || case.materiality_margin < 0
        || case.switching_cost < 0
        || !valid_gate_states(case)
    {
        return invalid_input(
            case,
            "invalid case identity, bounds, threshold or gate state",
        );
    }

    let Some(retention_threshold) = case.materiality_margin.checked_add(case.switching_cost) else {
        return invalid_input(
            case,
            "materiality margin plus switching cost overflows loss units",
        );
    };

    let mut action_ids = BTreeSet::new();
    for action_id in &case.action_ids {
        if !valid_id(action_id) || !action_ids.insert(action_id.clone()) {
            return invalid_input(
                case,
                "action universe contains an empty or duplicate action ID",
            );
        }
    }
    if !action_ids.contains(&case.incumbent_action_id) {
        return invalid_input(
            case,
            "incumbent action is absent from the declared finite action universe",
        );
    }

    // A source-pinned physical-world inventory and a separate joint loss-model inventory
    // are mandatory. Physical-world coverage does not imply loss-model coverage.
    if case.world_set_receipt_schema != WORLD_SET_RECEIPT_SCHEMA
        || case.world_set_claim_ceiling != WORLD_SET_CLAIM_CEILING
        || !valid_commit(&case.world_set_source_commit)
        || !valid_id(&case.world_set_source_case_id)
        || !valid_id(&case.world_set_receipt_id)
        || !valid_sha256(&case.world_set_receipt_sha256)
        || !valid_commit(&case.loss_model_set.source_commit)
        || !valid_sha256(&case.loss_model_set.source_receipt_sha256)
        || !valid_id(&case.loss_model_set.source_case_id)
        || !valid_id(&case.loss_model_set.source_manifest_id)
        || !valid_sha256(&case.loss_model_set.source_scenarios_sha256)
        || !valid_sha256(&case.loss_model_set.expected_scenario_ids_sha256)
        || !valid_sha256(&case.loss_model_set.projected_scenarios_sha256)
        || !valid_id(&case.loss_model_set.source_loss_unit)
        || !valid_id(&case.loss_model_set.projection_id)
        || case.loss_model_set.projection_scale_numerator == 0
        || case.loss_model_set.projection_scale_denominator == 0
        || case.loss_model_set.projection_scale_numerator > MAX_SAFE_INTEGER
        || case.loss_model_set.projection_scale_denominator > MAX_SAFE_INTEGER
        || case.loss_model_set.source_receipt_schema != JOINT_LOSS_RECEIPT_SCHEMA
        || case.loss_model_set.source_claim_ceiling != JOINT_LOSS_CLAIM_CEILING
    {
        return invalid_input(
            case,
            "upstream world or loss-model receipt pins are malformed",
        );
    }

    // Source state outranks empty-set semantics: a stale empty set is not evidence
    // that no worlds survive.
    if case.world_set_state == "UNAVAILABLE" {
        return abstain(case, "WORLD_SET_UNAVAILABLE", Some(retention_threshold));
    }
    if case.world_set_state == "STALE" {
        return abstain(case, "STALE_WORLD_SET", Some(retention_threshold));
    }
    if case.loss_model_set.state == "STALE" {
        return abstain(case, "STALE_LOSS_MODEL_SET", Some(retention_threshold));
    }
    if case.loss_model_set.state == "INDETERMINATE" {
        return abstain(
            case,
            "INDETERMINATE_LOSS_MODEL_SET",
            Some(retention_threshold),
        );
    }
    if case.loss_model_set.state == "OUT_OF_PROFILE" {
        return abstain(
            case,
            "LOSS_MODEL_SET_OUT_OF_PROFILE",
            Some(retention_threshold),
        );
    }
    if case.loss_model_set.state == "INADEQUATE" {
        return abstain(case, "LOSS_MODEL_SET_INADEQUATE", Some(retention_threshold));
    }

    let expected_world_ids = case
        .expected_world_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_world_digest = match digest_ids(&case.expected_world_ids) {
        Ok(digest) => digest,
        Err(_) => {
            return invalid_input(case, "planned physical-world inventory cannot be encoded");
        }
    };
    if expected_world_ids.len() != case.expected_world_ids.len()
        || expected_world_ids.iter().any(|id| !valid_id(id))
        || expected_world_digest != case.expected_world_ids_sha256
    {
        return invalid_input(
            case,
            "planned physical-world inventory or digest is invalid",
        );
    }

    let expected_scenario_ids = case
        .loss_model_set
        .expected_scenario_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let expected_scenario_digest = match digest_ids(&case.loss_model_set.expected_scenario_ids) {
        Ok(digest) => digest,
        Err(_) => {
            return invalid_input(
                case,
                "planned joint loss-scenario inventory cannot be encoded",
            );
        }
    };
    if expected_scenario_ids.len() != case.loss_model_set.expected_scenario_ids.len()
        || expected_scenario_ids.iter().any(|id| !valid_id(id))
        || expected_scenario_digest != case.loss_model_set.expected_scenario_ids_sha256
    {
        return invalid_input(
            case,
            "planned joint loss-scenario inventory or digest is invalid",
        );
    }
    if case.worlds.is_empty() {
        return abstain(case, "NO_SURVIVING_WORLDS", Some(retention_threshold));
    }
    if case.worlds.len() > MAX_SCENARIOS {
        return invalid_input(case, "joint loss-scenario set exceeds bound");
    }

    let projected_digest = match projected_scenarios_digest(&case.worlds) {
        Ok(value) => value,
        Err(_) => {
            return invalid_input(
                case,
                "projected joint loss-scenario manifest cannot be encoded",
            );
        }
    };
    if projected_digest != case.loss_model_set.projected_scenarios_sha256 {
        return invalid_input(case, "projected joint loss-scenario digest mismatch");
    }
    let actual_scenario_ids = case
        .worlds
        .iter()
        .map(|world| world.scenario_id.clone())
        .collect::<BTreeSet<_>>();
    let actual_world_ids = case
        .worlds
        .iter()
        .map(|world| world.world_id.clone())
        .collect::<BTreeSet<_>>();
    if actual_scenario_ids != expected_scenario_ids {
        return invalid_input(
            case,
            "observed joint scenarios do not match planned loss-model inventory",
        );
    }
    if actual_world_ids != expected_world_ids {
        return invalid_input(
            case,
            "joint loss scenarios do not cover the exact surviving physical-world inventory",
        );
    }

    // Remaining evidence and domain gates precede numeric comparison.
    if case.domain_closure_state == "OPEN" {
        return abstain(case, "DOMAIN_NOT_CLOSED", Some(retention_threshold));
    }
    if case.domain_closure_state == "STALE" {
        return abstain(case, "STALE_DOMAIN_CLOSURE", Some(retention_threshold));
    }
    if case.domain_closure_state == "INADEQUATE" || case.model_family_state == "INADEQUATE" {
        return abstain(case, "MODEL_FAMILY_INADEQUATE", Some(retention_threshold));
    }
    if case.dependency_graph_state != "GRAPH_COMPLETE_WITHIN_DECLARED_GRAMMAR"
        && case.dependency_graph_state != "STALE"
        && case.dependency_graph_state != "INDETERMINATE"
    {
        return abstain(
            case,
            "DEPENDENCY_GRAPH_INADEQUATE",
            Some(retention_threshold),
        );
    }
    if case.dependency_graph_state == "STALE" {
        return abstain(case, "STALE_DEPENDENCY_GRAPH", Some(retention_threshold));
    }
    if case.dependency_graph_state == "INDETERMINATE" {
        return abstain(
            case,
            "INDETERMINATE_DEPENDENCY_GRAPH",
            Some(retention_threshold),
        );
    }
    if case.evidence_state == "STALE" {
        return abstain(case, "STALE_EVIDENCE", Some(retention_threshold));
    }
    if case.evidence_state == "CONTRADICTED" {
        return abstain(case, "CONTRADICTED_EVIDENCE", Some(retention_threshold));
    }
    if case.evidence_state == "INDETERMINATE" {
        return abstain(case, "INDETERMINATE_EVIDENCE", Some(retention_threshold));
    }

    let mut scenario_ids = BTreeSet::new();
    let mut total_loss_rows = 0usize;
    let mut validated_worlds = Vec::with_capacity(case.worlds.len());
    let mut incumbent_infeasible = false;

    for world in &case.worlds {
        if !valid_id(&world.scenario_id)
            || !scenario_ids.insert(world.scenario_id.as_str())
            || !valid_id(&world.world_id)
            || !expected_world_ids.contains(&world.world_id)
            || !valid_id(&world.loss_model_id)
            || !valid_id(&world.loss_unit)
            || !valid_sha256(&world.scope_sha256)
            || !matches!(
                world.evidence_status.as_str(),
                "admissible" | "inadmissible" | "indeterminate" | "out_of_profile"
            )
        {
            return invalid_input(
                case,
                "joint loss-scenario identity, model, status or scope is malformed",
            );
        }
        if world.evidence_status == "out_of_profile" {
            return abstain(
                case,
                "LOSS_SCENARIO_OUT_OF_PROFILE",
                Some(retention_threshold),
            );
        }
        if world.evidence_status == "indeterminate" || world.evidence_status == "inadmissible" {
            return abstain(
                case,
                "LOSS_SCENARIO_EVIDENCE_INDETERMINATE",
                Some(retention_threshold),
            );
        }
        let Some(evidence_receipt_id) = world.evidence_receipt_id.as_deref() else {
            return abstain(
                case,
                "LOSS_SCENARIO_RECEIPT_MISSING",
                Some(retention_threshold),
            );
        };
        if !valid_id(evidence_receipt_id) {
            return invalid_input(case, "joint loss-scenario evidence receipt ID is malformed");
        }
        if world.loss_unit != case.loss_unit {
            return abstain(case, "LOSS_UNIT_MISMATCH", Some(retention_threshold));
        }
        if world.scope_sha256 != case.scope_sha256 {
            return abstain(case, "SCOPE_MISMATCH", Some(retention_threshold));
        }
        if world.feasible_action_ids.is_empty() {
            return abstain(
                case,
                "NO_FEASIBLE_ACTION_IN_WORLD",
                Some(retention_threshold),
            );
        }

        let mut feasible = BTreeSet::new();
        for action_id in &world.feasible_action_ids {
            if !action_ids.contains(action_id) || !feasible.insert(action_id.clone()) {
                return invalid_input(
                    case,
                    "world feasible set contains an unknown or duplicate action",
                );
            }
        }

        total_loss_rows = total_loss_rows.saturating_add(world.losses.len());
        if total_loss_rows > MAX_ROWS {
            return invalid_input(case, "loss matrix exceeds row bound");
        }

        let mut losses = BTreeMap::new();
        for loss in &world.losses {
            if loss.source_numerator > MAX_SAFE_INTEGER
                || loss.source_denominator == 0
                || loss.source_denominator > MAX_SAFE_INTEGER
            {
                return invalid_input(
                    case,
                    "source rational loss exceeds exact JSON integer bounds",
                );
            }
            let Some(projected_loss) = exact_projected_loss(
                loss.source_numerator,
                loss.source_denominator,
                case.loss_model_set.projection_scale_numerator,
                case.loss_model_set.projection_scale_denominator,
            ) else {
                return invalid_input(
                    case,
                    "source rational loss cannot be projected exactly into the declared loss unit",
                );
            };
            if !valid_id(&loss.action_id)
                || loss.loss < 0
                || !feasible.contains(&loss.action_id)
                || losses.contains_key(&loss.action_id)
            {
                return invalid_input(
                    case,
                    "loss row is malformed, duplicated or not feasible in its joint scenario",
                );
            }
            if loss.loss != projected_loss {
                return invalid_input(
                    case,
                    "integer loss does not match exact source rational projection",
                );
            }
            losses.insert(loss.action_id.clone(), loss.loss);
        }

        if feasible
            .iter()
            .any(|action_id| !losses.contains_key(action_id))
        {
            return abstain(case, "INCOMPLETE_LOSS_MATRIX", Some(retention_threshold));
        }

        let incumbent_feasible = feasible.contains(&case.incumbent_action_id);
        incumbent_infeasible |= !incumbent_feasible;
        validated_worlds.push(ValidatedWorld {
            scenario_id: world.scenario_id.clone(),
            world_id: world.world_id.clone(),
            loss_model_id: world.loss_model_id.clone(),
            evidence_receipt_id: evidence_receipt_id.to_owned(),
            feasible_action_ids: feasible,
            losses,
        });
    }

    validated_worlds.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    // An infeasible incumbent is an unconditional review trigger. Do not surface
    // a replacement candidate: this evaluator never selects or authorizes actions.
    if incumbent_infeasible {
        let assessments = validated_worlds
            .iter()
            .map(|world| WorldAssessment {
                scenario_id: world.scenario_id.clone(),
                world_id: world.world_id.clone(),
                loss_model_id: world.loss_model_id.clone(),
                evidence_receipt_id: world.evidence_receipt_id.clone(),
                incumbent_feasible: world
                    .feasible_action_ids
                    .contains(&case.incumbent_action_id),
                incumbent_loss: world.losses.get(&case.incumbent_action_id).copied(),
                reference_common_action_loss: None,
                incumbent_minus_reference_loss: None,
            })
            .collect();
        return result(
            case,
            "REOPTIMIZATION_WARRANTED",
            "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD",
            Some(retention_threshold),
            ComparisonMetrics::default(),
            assessments,
        );
    }

    // The incumbent is feasible in every surviving world, so the common feasible
    // action set must contain it. This is a static non-anticipative comparator:
    // one action must be feasible without knowing which hidden world is true.
    let mut common_feasible_actions = validated_worlds[0].feasible_action_ids.clone();
    for world in validated_worlds.iter().skip(1) {
        common_feasible_actions.retain(|action_id| world.feasible_action_ids.contains(action_id));
    }
    if common_feasible_actions.is_empty()
        || !common_feasible_actions.contains(&case.incumbent_action_id)
    {
        return invalid_input(
            case,
            "common feasible action set is empty despite incumbent feasibility",
        );
    }

    let mut current_worst_case_loss = 0i64;
    for world in &validated_worlds {
        let Some(loss) = world.losses.get(&case.incumbent_action_id).copied() else {
            return abstain(case, "INCOMPLETE_LOSS_MATRIX", Some(retention_threshold));
        };
        current_worst_case_loss = current_worst_case_loss.max(loss);
    }

    // The best common action is a benchmark comparator only, not a recommendation.
    // BTreeSet iteration plus the explicit tie-break makes it deterministic.
    let mut reference_common_action: Option<(i64, String)> = None;
    for action_id in &common_feasible_actions {
        let mut worst_case_loss = 0i64;
        for world in &validated_worlds {
            let Some(loss) = world.losses.get(action_id).copied() else {
                return abstain(case, "INCOMPLETE_LOSS_MATRIX", Some(retention_threshold));
            };
            worst_case_loss = worst_case_loss.max(loss);
        }
        let candidate = (worst_case_loss, action_id.clone());
        if reference_common_action
            .as_ref()
            .is_none_or(|best| (&candidate.0, &candidate.1) < (&best.0, &best.1))
        {
            reference_common_action = Some(candidate);
        }
    }
    let Some((reference_common_worst_case_loss, reference_common_action_id)) =
        reference_common_action
    else {
        return invalid_input(case, "no common feasible reference action was derived");
    };

    let Some(worst_case_improvement) =
        current_worst_case_loss.checked_sub(reference_common_worst_case_loss)
    else {
        return invalid_input(case, "common-action improvement subtraction overflowed");
    };
    if worst_case_improvement < 0 {
        return invalid_input(
            case,
            "reference common action is worse than the incumbent in worst-case loss",
        );
    }

    let assessments = validated_worlds
        .iter()
        .map(|world| {
            let incumbent_loss = world.losses[&case.incumbent_action_id];
            let reference_loss = world.losses[&reference_common_action_id];
            let incumbent_minus_reference_loss = incumbent_loss
                .checked_sub(reference_loss)
                .expect("difference of two non-negative i64 losses is representable");
            WorldAssessment {
                scenario_id: world.scenario_id.clone(),
                world_id: world.world_id.clone(),
                loss_model_id: world.loss_model_id.clone(),
                evidence_receipt_id: world.evidence_receipt_id.clone(),
                incumbent_feasible: true,
                incumbent_loss: Some(incumbent_loss),
                reference_common_action_loss: Some(reference_loss),
                incumbent_minus_reference_loss: Some(incumbent_minus_reference_loss),
            }
        })
        .collect();

    let metrics = ComparisonMetrics {
        reference_common_action_id: Some(reference_common_action_id),
        current_worst_case_loss: Some(current_worst_case_loss),
        reference_common_worst_case_loss: Some(reference_common_worst_case_loss),
        worst_case_improvement: Some(worst_case_improvement),
    };

    if worst_case_improvement > retention_threshold {
        return result(
            case,
            "REOPTIMIZATION_WARRANTED",
            "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD",
            Some(retention_threshold),
            metrics,
            assessments,
        );
    }

    result(
        case,
        "RETAIN_INCUMBENT",
        "WITHIN_FINITE_WORLD_RETENTION_ENVELOPE",
        Some(retention_threshold),
        metrics,
        assessments,
    )
}

pub fn evaluate(corpus: &Corpus) -> Result<Evaluation, String> {
    if corpus.schema != SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported incumbent-adequacy corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("incumbent-adequacy case IDs must be unique".to_owned());
        }
        cases.push(evaluate_case(case));
    }
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let count = |disposition: &str| {
        cases
            .iter()
            .filter(|case| case.disposition == disposition)
            .count()
    };
    let summary = Summary {
        case_count: cases.len(),
        retain_incumbent_cases: count("RETAIN_INCUMBENT"),
        reoptimization_warranted_cases: count("REOPTIMIZATION_WARRANTED"),
        abstain_cases: count("ABSTAIN"),
        invalid_input_cases: count("INVALID_INPUT"),
        incumbent_infeasible_cases: cases
            .iter()
            .filter(|case| case.trigger_reason == "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD")
            .count(),
        threshold_exceeded_cases: cases
            .iter()
            .filter(|case| {
                case.trigger_reason == "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD"
            })
            .count(),
        loss_model_set_blocked_cases: cases
            .iter()
            .filter(|case| {
                matches!(
                    case.trigger_reason.as_str(),
                    "STALE_LOSS_MODEL_SET"
                        | "INDETERMINATE_LOSS_MODEL_SET"
                        | "LOSS_MODEL_SET_OUT_OF_PROFILE"
                        | "LOSS_MODEL_SET_INADEQUATE"
                )
            })
            .count(),
        loss_scenario_evidence_blocked_cases: cases
            .iter()
            .filter(|case| {
                matches!(
                    case.trigger_reason.as_str(),
                    "LOSS_SCENARIO_OUT_OF_PROFILE"
                        | "LOSS_SCENARIO_EVIDENCE_INDETERMINATE"
                        | "LOSS_SCENARIO_RECEIPT_MISSING"
                )
            })
            .count(),
        joint_scenario_inventory_blocked_cases: cases
            .iter()
            .filter(|case| {
                case.trigger_reason.contains("joint scenario")
                    || case.trigger_reason.contains("joint loss-scenario")
                    || case.trigger_reason.contains("physical-world inventory")
            })
            .count(),
        expected_result_mismatches: cases
            .iter()
            .filter(|case| !case.expected_result_matched)
            .count(),
        authority_grants: cases.iter().filter(|case| case.authority_granted).count(),
    };

    let pass = summary.expected_result_mismatches == 0
        && summary.retain_incumbent_cases >= 1
        && summary.reoptimization_warranted_cases >= 1
        && summary.abstain_cases >= 1
        && summary.loss_model_set_blocked_cases >= 1
        && summary.loss_scenario_evidence_blocked_cases >= 1
        && summary.joint_scenario_inventory_blocked_cases >= 1
        && summary.authority_grants == 0;

    Ok(Evaluation {
        result: if pass { "pass" } else { "fail" }.to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        summary,
        cases,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Corpus {
        serde_json::from_str(include_str!(
            "../fixtures/v0.1/incumbent-adequacy-cases.json"
        ))
        .expect("incumbent adequacy corpus parses")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("case exists")
    }

    #[test]
    fn rational_loss_projection_is_exact_and_never_rounds() {
        assert_eq!(exact_projected_loss(1, 10, 1_000, 1), Some(100));
        assert_eq!(exact_projected_loss(95, 1_000, 1_000, 1), Some(95));
        assert_eq!(exact_projected_loss(1, 3, 1, 1), None);
        assert_eq!(exact_projected_loss(1, 0, 1, 1), None);
        assert_eq!(exact_projected_loss(1, 1, 1, 0), None);
    }

    #[test]
    fn projected_integer_loss_must_match_source_rational_exactly() {
        let mut corpus = fixture();
        let item = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "fractional_loss_projection")
            .expect("fractional projection case exists");
        item.worlds[0].losses[0].loss += 1;
        item.loss_model_set.projected_scenarios_sha256 =
            projected_scenarios_digest(&item.worlds).expect("projection digest recomputes");
        item.expected = Expected {
            disposition: "INVALID_INPUT".to_owned(),
            trigger_reason: "integer loss does not match exact source rational projection"
                .to_owned(),
            worst_case_improvement: None,
        };
        let report = evaluate(&corpus).expect("mismatch corpus evaluates");
        assert_eq!(
            case(&report, "fractional_loss_projection").disposition,
            "INVALID_INPUT"
        );
        assert_eq!(report.summary.expected_result_mismatches, 0);
    }

    #[test]
    fn multiple_loss_models_for_one_physical_world_are_distinct_joint_scenarios() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "multi_loss_models_same_world");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.joint_scenario_count, 2);
        assert_eq!(result.loss_model_ids, vec!["loss-model-A", "loss-model-B"]);
        assert_eq!(result.world_assessments.len(), 2);
        assert!(
            result
                .world_assessments
                .iter()
                .all(|assessment| { assessment.world_id == "physical-world-shared" })
        );
    }

    #[test]
    fn stale_loss_model_set_blocks_retention_even_when_world_set_is_current() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "loss_model_set_stale");
        assert_eq!(result.disposition, "ABSTAIN");
        assert_eq!(result.trigger_reason, "STALE_LOSS_MODEL_SET");
        assert_eq!(result.world_set_state, "CURRENT");
        assert_eq!(result.loss_model_state, "STALE");
    }

    #[test]
    fn loss_scenario_out_of_profile_blocks_numeric_decision_metrics() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "loss_scenario_out_of_profile");
        assert_eq!(result.disposition, "ABSTAIN");
        assert_eq!(result.trigger_reason, "LOSS_SCENARIO_OUT_OF_PROFILE");
        assert_eq!(result.worst_case_improvement, None);
    }

    #[test]
    fn omitted_joint_scenario_is_rejected_against_planned_inventory() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "joint_scenario_omission");
        assert_eq!(result.disposition, "INVALID_INPUT");
        assert_eq!(
            result.trigger_reason,
            "observed joint scenarios do not match planned loss-model inventory"
        );
    }

    #[test]
    fn omitted_physical_world_is_not_hidden_by_remaining_loss_models() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "physical_world_omission");
        assert_eq!(result.disposition, "INVALID_INPUT");
        assert_eq!(
            result.trigger_reason,
            "joint loss scenarios do not cover the exact surviving physical-world inventory"
        );
    }

    #[test]
    fn stable_incumbent_is_retained_against_common_action_comparator() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "baseline_retain");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_improvement, Some(5));
        assert_eq!(result.reference_common_action_id.as_deref(), Some("B"));
        assert_eq!(result.current_worst_case_loss, Some(100));
        assert_eq!(result.reference_common_worst_case_loss, Some(95));
    }

    #[test]
    fn decision_irrelevant_world_shift_does_not_force_reoptimization() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "decision_irrelevant_shift");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_improvement, Some(0));
        assert_eq!(result.reference_common_action_id.as_deref(), Some("A"));
    }

    #[test]
    fn opposite_world_optima_do_not_enable_hidden_world_switching() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "opposite_optima_within_margin");
        // A hindsight comparator would claim a 4-unit improvement in each world
        // and trigger against margin 3. No single common action improves worst-case
        // loss, so the incumbent remains adequate without observing the hidden world.
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.materiality_margin, 3);
        assert_eq!(result.worst_case_improvement, Some(0));
        assert_eq!(result.reference_common_action_id.as_deref(), Some("A"));
        assert_eq!(result.current_worst_case_loss, Some(104));
        assert_eq!(result.reference_common_worst_case_loss, Some(104));
    }

    #[test]
    fn exceeding_materiality_plus_switching_cost_triggers_review() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "threshold_exceeded");
        assert_eq!(result.disposition, "REOPTIMIZATION_WARRANTED");
        assert_eq!(
            result.trigger_reason,
            "COMMON_ACTION_IMPROVEMENT_EXCEEDS_RETENTION_THRESHOLD"
        );
    }

    #[test]
    fn equality_at_retention_threshold_is_retained() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "threshold_equality");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_improvement, result.retention_threshold);
    }

    #[test]
    fn incumbent_infeasibility_triggers_reoptimization_review() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "incumbent_infeasible");
        assert_eq!(result.disposition, "REOPTIMIZATION_WARRANTED");
        assert_eq!(
            result.trigger_reason,
            "INCUMBENT_INFEASIBLE_IN_SURVIVING_WORLD"
        );
        assert_eq!(result.worst_case_improvement, None);
    }

    #[test]
    fn empty_world_set_abstains_instead_of_claiming_certainty() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "no_surviving_worlds").trigger_reason,
            "NO_SURVIVING_WORLDS"
        );
        assert_eq!(case(&report, "no_surviving_worlds").disposition, "ABSTAIN");
    }

    #[test]
    fn inadequate_dependency_graph_blocks_retention() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "dependency_graph_inadequate").disposition,
            "ABSTAIN"
        );
        assert_eq!(
            case(&report, "dependency_graph_inadequate").trigger_reason,
            "DEPENDENCY_GRAPH_INADEQUATE"
        );
    }

    #[test]
    fn stale_dependency_graph_blocks_retention() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "dependency_graph_stale").trigger_reason,
            "STALE_DEPENDENCY_GRAPH"
        );
    }

    #[test]
    fn open_domain_and_stale_evidence_block_retention() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "domain_open").trigger_reason,
            "DOMAIN_NOT_CLOSED"
        );
        assert_eq!(
            case(&report, "stale_evidence").trigger_reason,
            "STALE_EVIDENCE"
        );
    }

    #[test]
    fn loss_unit_mismatch_abstains() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(case(&report, "loss_unit_mismatch").disposition, "ABSTAIN");
        assert_eq!(
            case(&report, "loss_unit_mismatch").trigger_reason,
            "LOSS_UNIT_MISMATCH"
        );
    }

    #[test]
    fn incomplete_loss_matrix_abstains() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert_eq!(
            case(&report, "incomplete_loss_matrix").trigger_reason,
            "INCOMPLETE_LOSS_MATRIX"
        );
    }

    #[test]
    fn threshold_overflow_and_duplicate_loss_rows_fail_closed() {
        let mut corpus = fixture();
        let overflow = corpus
            .cases
            .iter_mut()
            .find(|case| case.case_id == "baseline_retain")
            .expect("baseline case exists");
        overflow.materiality_margin = i64::MAX;
        overflow.switching_cost = 1;
        overflow.expected = Expected {
            disposition: "INVALID_INPUT".to_owned(),
            trigger_reason: "materiality margin plus switching cost overflows loss units"
                .to_owned(),
            worst_case_improvement: None,
        };
        let report = evaluate(&corpus).expect("overflow corpus evaluates");
        assert_eq!(
            case(&report, "baseline_retain").disposition,
            "INVALID_INPUT"
        );
        assert_eq!(report.summary.expected_result_mismatches, 0);

        let regular = evaluate(&fixture()).expect("regular corpus evaluates");
        assert_eq!(
            case(&regular, "duplicate_loss_row").disposition,
            "INVALID_INPUT"
        );
    }

    #[test]
    fn all_permutations_produce_identical_results() {
        let expected = evaluate(&fixture()).expect("corpus evaluates");
        let mut permuted = fixture();
        for case in &mut permuted.cases {
            case.action_ids.reverse();
            case.worlds.reverse();
            for world in &mut case.worlds {
                world.feasible_action_ids.reverse();
                world.losses.reverse();
            }
        }
        let actual = evaluate(&permuted).expect("permuted corpus evaluates");
        assert_eq!(
            serde_json::to_value(actual).expect("actual serializes"),
            serde_json::to_value(expected).expect("expected serializes")
        );
    }

    #[test]
    fn no_path_grants_operational_authority() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        assert!(!report.authority_granted);
        assert!(report.cases.iter().all(|case| !case.authority_granted));
    }
}
