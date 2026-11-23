//! Finite-world incumbent decision adequacy gate.
//!
//! This module decides whether a supplied incumbent remains within a declared
//! finite-world regret envelope. It never selects or executes a replacement action,
//! estimates probabilities, or grants operational authority.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const SCHEMA: &str = "luminous.orbital-incumbent-adequacy.corpus.v0.1";
const CLAIM_CEILING: &str = "synthetic-finite-world-incumbent-adequacy-only";
const MAX_CASES: usize = 128;
const MAX_ACTIONS: usize = 4096;
const MAX_WORLDS: usize = 4096;
const MAX_ROWS: usize = 16384;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorldLoss {
    pub action_id: String,
    pub loss: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct World {
    pub world_id: String,
    pub loss_unit: String,
    pub scope_sha256: String,
    pub feasible_action_ids: Vec<String>,
    pub losses: Vec<WorldLoss>,
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
    pub world_id: String,
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

fn valid_gate_states(case: &Case) -> bool {
    matches!(
        case.world_set_state.as_str(),
        "CURRENT" | "STALE" | "UNAVAILABLE"
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
    world_id: String,
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
        || case.worlds.len() > MAX_WORLDS
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

    // An empty surviving-world set is not certainty and cannot justify retention.
    if case.worlds.is_empty() {
        return abstain(case, "NO_SURVIVING_WORLDS", Some(retention_threshold));
    }
    if case.worlds.len() > MAX_WORLDS {
        return invalid_input(case, "surviving-world set exceeds bound");
    }

    // External/producer state gates take precedence over numerical comparison.
    if case.world_set_state == "UNAVAILABLE" {
        return abstain(case, "WORLD_SET_UNAVAILABLE", Some(retention_threshold));
    }
    if case.world_set_state == "STALE" {
        return abstain(case, "STALE_WORLD_SET", Some(retention_threshold));
    }
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

    let mut world_ids = BTreeSet::new();
    let mut total_loss_rows = 0usize;
    let mut validated_worlds = Vec::with_capacity(case.worlds.len());
    let mut incumbent_infeasible = false;

    for world in &case.worlds {
        if !valid_id(&world.world_id)
            || !world_ids.insert(world.world_id.as_str())
            || !valid_id(&world.loss_unit)
            || !valid_sha256(&world.scope_sha256)
        {
            return invalid_input(case, "world identity is duplicate, empty or malformed");
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
            if !valid_id(&loss.action_id)
                || loss.loss < 0
                || !feasible.contains(&loss.action_id)
                || losses.insert(loss.action_id.clone(), loss.loss).is_some()
            {
                return invalid_input(
                    case,
                    "loss row is malformed, duplicated or not feasible in its world",
                );
            }
        }

        if feasible.iter().any(|action_id| !losses.contains_key(action_id)) {
            return abstain(case, "INCOMPLETE_LOSS_MATRIX", Some(retention_threshold));
        }

        let incumbent_feasible = feasible.contains(&case.incumbent_action_id);
        incumbent_infeasible |= !incumbent_feasible;
        validated_worlds.push(ValidatedWorld {
            world_id: world.world_id.clone(),
            feasible_action_ids: feasible,
            losses,
        });
    }

    // An infeasible incumbent is an unconditional review trigger. Do not surface
    // a replacement candidate: this evaluator never selects or authorizes actions.
    if incumbent_infeasible {
        let assessments = validated_worlds
            .iter()
            .map(|world| WorldAssessment {
                world_id: world.world_id.clone(),
                incumbent_feasible: world.feasible_action_ids.contains(&case.incumbent_action_id),
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
                world_id: world.world_id.clone(),
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
            .filter(|case| case.trigger_reason == "WORST_CASE_REGRET_EXCEEDS_RETENTION_THRESHOLD")
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
    fn stable_incumbent_is_retained_with_exact_regret() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "baseline_retain");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_regret, Some(5));
    }

    #[test]
    fn decision_irrelevant_world_shift_does_not_force_reoptimization() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "decision_irrelevant_shift");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_regret, Some(0));
    }

    #[test]
    fn different_world_optima_can_still_support_incumbent_retention() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "opposite_optima_within_margin");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_regret, Some(4));
    }

    #[test]
    fn exceeding_materiality_plus_switching_cost_triggers_review() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "threshold_exceeded");
        assert_eq!(result.disposition, "REOPTIMIZATION_WARRANTED");
        assert_eq!(
            result.trigger_reason,
            "WORST_CASE_REGRET_EXCEEDS_RETENTION_THRESHOLD"
        );
    }

    #[test]
    fn equality_at_retention_threshold_is_retained() {
        let report = evaluate(&fixture()).expect("corpus evaluates");
        let result = case(&report, "threshold_equality");
        assert_eq!(result.disposition, "RETAIN_INCUMBENT");
        assert_eq!(result.worst_case_regret, result.retention_threshold);
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
        assert_eq!(result.worst_case_regret, None);
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
            worst_case_regret: None,
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
