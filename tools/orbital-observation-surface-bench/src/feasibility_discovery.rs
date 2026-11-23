//! Exact finite query-conditioned non-anticipative robust feasibility policies.
//!
//! v0.1 accepts passive, deterministic, scope-bound observations over a frozen
//! compatible-world set. It enumerates every branch-action mapping that remains
//! feasible across all worlds/loss-model rows sharing the same observable outcome.
//! Thus the policy may branch on observed outcomes, never on hidden world IDs.
//!
//! Query kernels, evidence authenticity, statistical calibration and physical
//! query-safety are upstream contracts; this module only checks finite structure
//! and exact policy/regret algebra. It reports vector frontiers, not a scalar
//! exchange rate between information, loss and operational cost.

use crate::decision_identifiability::Rational;
use crate::feasibility_regret::{ActionCost, DecisionCase, DecisionCorpus, FeasibilityScenario};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-feasibility-discovery.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-passive-query-policy-tree-algebra-only";
const MAX_CASES: usize = 128;
const MAX_QUERIES: usize = 64;
const MAX_WORLD_OBSERVATIONS: usize = 64;
const MAX_POLICIES_PER_QUERY: usize = 4096;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const NO_QUERY_ID: &str = "__NO_QUERY__";
const NO_QUERY_OUTCOME: &str = "__NO_QUERY__";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Query {
    pub query_id: String,
    /// v0.1 is passive observation only; state-changing interventions need a transition model.
    pub query_kind: String,
    pub admissibility_status: String,
    pub admissibility_receipt_id: Option<String>,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
    pub expected_world_ids: Vec<String>,
    pub expected_world_ids_sha256: String,
    /// Exact deterministic public result for each compatible physical world.
    pub world_observations: BTreeMap<String, String>,
    pub cost: ActionCost,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryUniverse {
    pub source_receipt_id: String,
    pub scope_id: String,
    pub expected_query_ids: Vec<String>,
    pub expected_query_ids_sha256: String,
    /// SHA-256 over canonical query records sorted by query_id.
    pub queries_sha256: String,
    pub queries: Vec<Query>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCase {
    pub case_id: String,
    pub base_decision: DecisionCase,
    pub query_universe: QueryUniverse,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryCorpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<DiscoveryCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SignedDifference {
    /// baseline minus candidate: positive means the candidate reduced worst-case loss.
    pub direction: String,
    pub magnitude: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyScenarioResult {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub observed_outcome_id: String,
    pub selected_action_id: String,
    pub loss: Rational,
    pub wait_and_see_regret: Rational,
    pub outcome_class_regret: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyMetric {
    pub policy_id: String,
    pub query_id: Option<String>,
    /// Keys are observable outcomes (or __NO_QUERY__), never hidden world/scenario IDs.
    pub branch_actions: BTreeMap<String, String>,
    pub observation_class_world_ids: BTreeMap<String, Vec<String>>,
    pub worst_case_loss: Rational,
    pub worst_case_wait_and_see_regret: Rational,
    pub worst_case_outcome_class_regret: Rational,
    pub worst_loss_scenario_ids: Vec<String>,
    pub worst_wait_and_see_scenario_ids: Vec<String>,
    pub worst_outcome_class_regret_scenario_ids: Vec<String>,
    pub worst_loss_reduction_vs_best_no_query: Option<SignedDifference>,
    pub query_cost: ActionCost,
    /// Maximum execution cost over the policy's observable branches, not their sum.
    pub branch_action_cost_upper_bound: ActionCost,
    /// Query cost plus worst branch execution cost, kept as separate native dimensions.
    pub total_cost_upper_bound: ActionCost,
    pub per_scenario: Vec<PolicyScenarioResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryReport {
    pub query_id: String,
    pub status: String,
    pub observable_outcome_ids: Vec<String>,
    pub observation_class_world_ids: BTreeMap<String, Vec<String>>,
    pub common_feasible_action_ids_by_outcome: BTreeMap<String, Vec<String>>,
    pub generated_policy_count: usize,
    pub policy_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub base_feasibility_status: String,
    pub compatible_world_ids_sha256: String,
    pub action_ids_sha256: String,
    pub scenario_manifest_sha256: String,
    pub query_universe_receipt_id: String,
    pub query_universe_scope_id: String,
    pub expected_query_ids_sha256: String,
    pub queries_sha256: String,
    pub no_query_policy_ids: Vec<String>,
    pub query_reports: Vec<QueryReport>,
    pub policy_metrics: Vec<PolicyMetric>,
    pub policy_pareto_frontier_ids: Vec<String>,
    pub best_no_query_worst_case_loss: Option<Rational>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub base_decision_indeterminate_cases: usize,
    pub no_query_policy_cases: usize,
    pub no_safe_query_policy_cases: usize,
    pub query_evidence_indeterminate_count: usize,
    pub query_prohibited_count: usize,
    pub generated_policy_count: usize,
    pub pareto_frontier_policy_count: usize,
    pub positive_worst_loss_reduction_policy_count: usize,
    pub policy_enumeration_limit_cases: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Evaluation {
    pub result: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub summary: Summary,
    pub cases: Vec<CaseResult>,
    pub physical_qualification_granted: bool,
    pub authority_granted: bool,
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical IDs: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn canonical_queries(queries: &[Query]) -> Result<Value, String> {
    let mut sorted = queries.to_vec();
    sorted.sort_by(|left, right| left.query_id.cmp(&right.query_id));
    serde_json::to_value(sorted)
        .map_err(|error| format!("failed to encode canonical query records: {error}"))
}

fn digest_queries(queries: &[Query]) -> Result<String, String> {
    let bytes = serde_json::to_vec(&canonical_queries(queries)?)
        .map_err(|error| format!("failed to serialize query records: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn checked_add_cost(left: &ActionCost, right: &ActionCost) -> Result<ActionCost, String> {
    Ok(ActionCost {
        reference_trials: left
            .reference_trials
            .checked_add(right.reference_trials)
            .filter(|value| *value <= MAX_SAFE_INTEGER)
            .ok_or_else(|| "reference trial cost overflow".to_owned())?,
        exposure_micro_units: left
            .exposure_micro_units
            .checked_add(right.exposure_micro_units)
            .filter(|value| *value <= MAX_SAFE_INTEGER)
            .ok_or_else(|| "exposure cost overflow".to_owned())?,
        elapsed_millis: left
            .elapsed_millis
            .checked_add(right.elapsed_millis)
            .filter(|value| *value <= MAX_SAFE_INTEGER)
            .ok_or_else(|| "elapsed cost overflow".to_owned())?,
    })
}

fn max_cost(left: &ActionCost, right: &ActionCost) -> ActionCost {
    ActionCost {
        reference_trials: left.reference_trials.max(right.reference_trials),
        exposure_micro_units: left.exposure_micro_units.max(right.exposure_micro_units),
        elapsed_millis: left.elapsed_millis.max(right.elapsed_millis),
    }
}

fn validate_query_universe(case: &DiscoveryCase) -> Result<(), String> {
    let universe = &case.query_universe;
    let worlds = &case.base_decision.compatible_world_set.world_ids;
    if !valid_id(&case.case_id)
        || !valid_id(&universe.source_receipt_id)
        || !valid_id(&universe.scope_id)
        || universe.scope_id != case.base_decision.decision_scope_id
        || universe.queries.is_empty()
        || universe.queries.len() > MAX_QUERIES
        || universe.expected_query_ids.is_empty()
        || universe.expected_query_ids.len() != universe.queries.len()
    {
        return Err("query-universe identity, scope or finite bounds are invalid".to_owned());
    }

    let mut expected_queries = BTreeSet::new();
    for query_id in &universe.expected_query_ids {
        if !valid_id(query_id) || !expected_queries.insert(query_id.as_str()) {
            return Err("planned query IDs must be unique nonempty ASCII strings".to_owned());
        }
    }
    if digest_ids(&universe.expected_query_ids)? != universe.expected_query_ids_sha256 {
        return Err("planned query-ID inventory digest mismatch".to_owned());
    }

    let mut observed_queries = BTreeSet::new();
    for query in &universe.queries {
        if !valid_id(&query.query_id)
            || !observed_queries.insert(query.query_id.as_str())
            || query.query_kind != "passive_observation"
            || !matches!(
                query.admissibility_status.as_str(),
                "admissible" | "prohibited" | "indeterminate"
            )
            || !matches!(
                query.evidence_status.as_str(),
                "admissible" | "indeterminate" | "out_of_profile"
            )
            || query.expected_world_ids.len() != worlds.len()
            || query.expected_world_ids_sha256
                != case.base_decision.compatible_world_set.world_ids_sha256
            || query.world_observations.len() != worlds.len()
            || query.world_observations.len() > MAX_WORLD_OBSERVATIONS
            || [
                query.cost.reference_trials,
                query.cost.exposure_micro_units,
                query.cost.elapsed_millis,
            ]
            .iter()
            .any(|value| *value > MAX_SAFE_INTEGER)
        {
            return Err(format!(
                "query {} identity, kind, evidence or bounds are invalid",
                query.query_id
            ));
        }
        let expected_worlds = query
            .expected_world_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let base_worlds = worlds.iter().map(String::as_str).collect::<BTreeSet<_>>();
        if expected_worlds != base_worlds
            || digest_ids(&query.expected_world_ids)? != query.expected_world_ids_sha256
        {
            return Err(format!(
                "query {} expected-world inventory does not match the base world set",
                query.query_id
            ));
        }
        for (value, field) in [
            (query.admissibility_receipt_id.as_deref(), "admissibility receipt"),
            (query.evidence_receipt_id.as_deref(), "evidence receipt"),
        ] {
            if query.admissibility_status == "admissible"
                && query.evidence_status == "admissible"
                && value.is_none_or(|item| !valid_id(item))
            {
                return Err(format!("query {} lacks a valid {field}", query.query_id));
            }
        }

        let expected_worlds = worlds.iter().map(String::as_str).collect::<BTreeSet<_>>();
        let observed_worlds = query
            .world_observations
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        if expected_worlds != observed_worlds {
            return Err(format!(
                "query {} must provide exactly one observed outcome for each compatible world",
                query.query_id
            ));
        }
        if query
            .world_observations
            .values()
            .any(|outcome| !valid_id(outcome))
        {
            return Err(format!("query {} contains an invalid observable outcome ID", query.query_id));
        }
    }

    if observed_queries != expected_queries {
        return Err("observed query IDs do not match the planned query inventory".to_owned());
    }
    if digest_queries(&universe.queries)? != universe.queries_sha256 {
        return Err("full query-universe digest mismatch".to_owned());
    }
    Ok(())
}

fn empty_case_result(case: &DiscoveryCase, status: &str, base_status: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        base_feasibility_status: base_status.to_owned(),
        compatible_world_ids_sha256: case.base_decision.compatible_world_set.world_ids_sha256.clone(),
        action_ids_sha256: case.base_decision.action_universe.expected_action_ids_sha256.clone(),
        scenario_manifest_sha256: case.base_decision.scenario_manifest.scenarios_sha256.clone(),
        query_universe_receipt_id: case.query_universe.source_receipt_id.clone(),
        query_universe_scope_id: case.query_universe.scope_id.clone(),
        expected_query_ids_sha256: case.query_universe.expected_query_ids_sha256.clone(),
        queries_sha256: case.query_universe.queries_sha256.clone(),
        no_query_policy_ids: Vec::new(),
        query_reports: Vec::new(),
        policy_metrics: Vec::new(),
        policy_pareto_frontier_ids: Vec::new(),
        best_no_query_worst_case_loss: None,
    }
}

fn build_observation_classes(
    world_observations: &BTreeMap<String, String>,
) -> BTreeMap<String, Vec<String>> {
    let mut classes = BTreeMap::<String, Vec<String>>::new();
    for (world_id, outcome_id) in world_observations {
        classes
            .entry(outcome_id.clone())
            .or_default()
            .push(world_id.clone());
    }
    for worlds in classes.values_mut() {
        worlds.sort();
    }
    classes
}

fn class_scenarios<'a>(
    scenarios: &'a [FeasibilityScenario],
    class_worlds: &[String],
) -> Vec<&'a FeasibilityScenario> {
    let world_set = class_worlds.iter().map(String::as_str).collect::<BTreeSet<_>>();
    scenarios
        .iter()
        .filter(|scenario| world_set.contains(scenario.world_id.as_str()))
        .collect()
}

fn common_feasible_actions(
    actions: &[crate::feasibility_regret::Action],
    scenarios: &[&FeasibilityScenario],
) -> Result<Vec<String>, String> {
    if scenarios.is_empty() {
        return Err("observable outcome class has no linked joint scenarios".to_owned());
    }
    let mut common = actions
        .iter()
        .filter(|action| action.eligibility_status == "eligible")
        .map(|action| action.action_id.clone())
        .collect::<BTreeSet<_>>();

    for scenario in scenarios {
        common.retain(|action_id| {
            scenario
                .action_outcomes
                .get(action_id)
                .is_some_and(|outcome| outcome.feasibility_status == "feasible" && outcome.loss.is_some())
        });
    }

    Ok(common.into_iter().collect())
}

fn scenario_best_loss(scenario: &FeasibilityScenario) -> Result<Rational, String> {
    scenario
        .action_outcomes
        .values()
        .filter(|outcome| outcome.feasibility_status == "feasible")
        .filter_map(|outcome| outcome.loss.as_ref())
        .min_by(|left, right| left.compare_exact(right))
        .map(Rational::canonicalized)
        .ok_or_else(|| format!("scenario {} has no feasible action loss", scenario.scenario_id))
}

fn policy_identifier(
    query_id: Option<&str>,
    branches: &BTreeMap<String, String>,
) -> Result<String, String> {
    let value = json!({
        "query_id": query_id,
        "branch_actions": branches
    });
    let bytes = serde_json::to_vec(&value)
        .map_err(|error| format!("failed to serialize canonical policy ID: {error}"))?;
    Ok(sha256_hex(&bytes))
}

fn signed_difference(baseline: &Rational, candidate: &Rational) -> Result<SignedDifference, String> {
    match baseline.compare_exact(candidate) {
        Ordering::Greater => Ok(SignedDifference {
            direction: "positive_improvement".to_owned(),
            magnitude: baseline
                .checked_sub(candidate)
                .map_err(|error| format!("loss improvement underflow: {error}"))?
                .canonicalized(),
        }),
        Ordering::Less => Ok(SignedDifference {
            direction: "negative_improvement".to_owned(),
            magnitude: candidate
                .checked_sub(baseline)
                .map_err(|error| format!("loss difference underflow: {error}"))?
                .canonicalized(),
        }),
        Ordering::Equal => Ok(SignedDifference {
            direction: "no_change".to_owned(),
            magnitude: Rational {
                numerator: 0,
                denominator: 1,
            },
        }),
    }
}

fn build_policy(
    base: &DecisionCase,
    query_id: Option<&str>,
    query_cost: &ActionCost,
    observations: &BTreeMap<String, String>,
    classes: &BTreeMap<String, Vec<String>>,
    class_options: &BTreeMap<String, Vec<String>>,
    branches: BTreeMap<String, String>,
) -> Result<PolicyMetric, String> {
    let mut sorted_scenarios = base.scenario_manifest.scenarios.iter().collect::<Vec<_>>();
    sorted_scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let scenarios = sorted_scenarios;
    let actions = &base.actions;
    let mut action_cost_upper_bound = ActionCost {
        reference_trials: 0,
        exposure_micro_units: 0,
        elapsed_millis: 0,
    };

    for action_id in branches.values() {
        let action = actions
            .iter()
            .find(|item| item.action_id == *action_id)
            .ok_or_else(|| format!("policy references unknown action {action_id}"))?;
        action_cost_upper_bound = max_cost(&action_cost_upper_bound, &action.cost);
    }
    let total_cost_upper_bound = checked_add_cost(query_cost, &action_cost_upper_bound)?;

    // Compare each branch against the best *single* action robust across its
    // observable class. Do not let the comparator see the hidden scenario.
    let mut outcome_class_regret_by_observation = BTreeMap::new();
    for (outcome_id, class_worlds) in classes {
        let class_rows =
            class_scenarios(&base.scenario_manifest.scenarios, class_worlds);
        let options = class_options
            .get(outcome_id)
            .ok_or_else(|| format!("missing common action set for {outcome_id}"))?;
        let mut best_worst_class_loss: Option<Rational> = None;
        let branch_action_id = branches
            .get(outcome_id)
            .ok_or_else(|| format!("policy omitted observable outcome {outcome_id}"))?;
        let mut selected_worst_class_loss: Option<Rational> = None;

        for action_id in options {
            let mut worst_for_action: Option<Rational> = None;
            for row in &class_rows {
                let loss = row
                    .action_outcomes
                    .get(action_id)
                    .and_then(|outcome| outcome.loss.as_ref())
                    .filter(|_| {
                        row.action_outcomes
                            .get(action_id)
                            .is_some_and(|outcome| outcome.feasibility_status == "feasible")
                    })
                    .ok_or_else(|| {
                        format!(
                            "class action {action_id} is not feasible throughout observation {outcome_id}"
                        )
                    })?
                    .canonicalized();
                if worst_for_action
                    .as_ref()
                    .is_none_or(|current| loss.compare_exact(current) == Ordering::Greater)
                {
                    worst_for_action = Some(loss);
                }
            }
            let worst_for_action = worst_for_action.ok_or_else(|| {
                format!("observation class {outcome_id} has no linked scenarios")
            })?;
            if action_id == branch_action_id {
                selected_worst_class_loss = Some(worst_for_action.clone());
            }
            if best_worst_class_loss
                .as_ref()
                .is_none_or(|current| worst_for_action.compare_exact(current) == Ordering::Less)
            {
                best_worst_class_loss = Some(worst_for_action);
            }
        }

        let selected_worst_class_loss = selected_worst_class_loss.ok_or_else(|| {
            format!("selected action {branch_action_id} is absent from class {outcome_id}")
        })?;
        let best_worst_class_loss = best_worst_class_loss.ok_or_else(|| {
            format!("observation class {outcome_id} has no robust comparator action")
        })?;
        let class_regret = selected_worst_class_loss
            .checked_sub(&best_worst_class_loss)
            .map_err(|error| format!("outcome-class regret underflow: {error}"))?;
        outcome_class_regret_by_observation.insert(outcome_id.clone(), class_regret);
    }

    let mut per_scenario = Vec::with_capacity(scenarios.len());

    for scenario in &scenarios {
        let outcome_id = observations
            .get(&scenario.world_id)
            .ok_or_else(|| format!("scenario world {} has no query outcome", scenario.world_id))?;
        let action_id = branches
            .get(outcome_id)
            .ok_or_else(|| format!("policy omitted observable outcome {outcome_id}"))?;
        let outcome = scenario
            .action_outcomes
            .get(action_id)
            .ok_or_else(|| format!("policy action {action_id} missing from scenario {}", scenario.scenario_id))?;
        if outcome.feasibility_status != "feasible" {
            return Err(format!(
                "policy action {} is not feasible in scenario {}",
                action_id, scenario.scenario_id
            ));
        }
        let loss = outcome
            .loss
            .as_ref()
            .ok_or_else(|| format!("policy action {} has no loss", action_id))?
            .canonicalized();
        let wait_and_see_best = scenario_best_loss(scenario)?;
        let outcome_class_regret = outcome_class_regret_by_observation
            .get(outcome_id)
            .ok_or_else(|| format!("missing outcome-class regret for {outcome_id}"))?
            .canonicalized();

        per_scenario.push(PolicyScenarioResult {
            scenario_id: scenario.scenario_id.clone(),
            world_id: scenario.world_id.clone(),
            loss_model_id: scenario.loss_model_id.clone(),
            observed_outcome_id: outcome_id.clone(),
            selected_action_id: action_id.clone(),
            loss: loss.clone(),
            wait_and_see_regret: loss
                .checked_sub(&wait_and_see_best)
                .map_err(|error| format!("wait-and-see regret underflow: {error}"))?,
            outcome_class_regret: loss
                .checked_sub(&outcome_class_best)
                .map_err(|error| format!("outcome-class regret underflow: {error}"))?,
        });
    }

    let worst_case_loss = per_scenario
        .iter()
        .map(|item| &item.loss)
        .max_by(|left, right| left.compare_exact(right))
        .ok_or_else(|| "policy has no scenario losses".to_owned())?
        .canonicalized();
    let worst_case_wait_and_see_regret = per_scenario
        .iter()
        .map(|item| &item.wait_and_see_regret)
        .max_by(|left, right| left.compare_exact(right))
        .ok_or_else(|| "policy has no wait-and-see regrets".to_owned())?
        .canonicalized();
    let worst_case_outcome_class_regret = per_scenario
        .iter()
        .map(|item| &item.outcome_class_regret)
        .max_by(|left, right| left.compare_exact(right))
        .ok_or_else(|| "policy has no outcome-class regrets".to_owned())?
        .canonicalized();
    let worst_loss_scenario_ids = per_scenario
        .iter()
        .filter(|item| item.loss.exactly_equals(&worst_case_loss))
        .map(|item| item.scenario_id.clone())
        .collect();
    let worst_wait_and_see_scenario_ids = per_scenario
        .iter()
        .filter(|item| item.wait_and_see_regret.exactly_equals(&worst_case_wait_and_see_regret))
        .map(|item| item.scenario_id.clone())
        .collect();
    let worst_outcome_class_regret_scenario_ids = per_scenario
        .iter()
        .filter(|item| item.outcome_class_regret.exactly_equals(&worst_case_outcome_class_regret))
        .map(|item| item.scenario_id.clone())
        .collect();

    Ok(PolicyMetric {
        policy_id: policy_identifier(query_id, &branches)?,
        query_id: query_id.map(str::to_owned),
        branch_actions: branches,
        observation_class_world_ids: classes.clone(),
        worst_case_loss,
        worst_case_wait_and_see_regret,
        worst_case_outcome_class_regret,
        worst_loss_scenario_ids,
        worst_wait_and_see_scenario_ids,
        worst_outcome_class_regret_scenario_ids,
        worst_loss_reduction_vs_best_no_query: None,
        query_cost: query_cost.clone(),
        branch_action_cost_upper_bound: action_cost_upper_bound,
        total_cost_upper_bound,
        per_scenario,
    })
}

fn enumerate_policy_branches(
    choices: &[(String, Vec<String>)],
    index: usize,
    current: &mut BTreeMap<String, String>,
    output: &mut Vec<BTreeMap<String, String>>,
) {
    if output.len() >= MAX_POLICIES_PER_QUERY {
        return;
    }
    if index == choices.len() {
        output.push(current.clone());
        return;
    }
    let (outcome_id, actions) = &choices[index];
    for action_id in actions {
        current.insert(outcome_id.clone(), action_id.clone());
        enumerate_policy_branches(choices, index + 1, current, output);
        if output.len() >= MAX_POLICIES_PER_QUERY {
            break;
        }
    }
    current.remove(outcome_id);
}

fn dominates(left: &PolicyMetric, right: &PolicyMetric) -> bool {
    let dimensions = [
        left.worst_case_loss.compare_exact(&right.worst_case_loss),
        left.worst_case_wait_and_see_regret
            .compare_exact(&right.worst_case_wait_and_see_regret),
        left.worst_case_outcome_class_regret
            .compare_exact(&right.worst_case_outcome_class_regret),
        left.total_cost_upper_bound
            .reference_trials
            .cmp(&right.total_cost_upper_bound.reference_trials),
        left.total_cost_upper_bound
            .exposure_micro_units
            .cmp(&right.total_cost_upper_bound.exposure_micro_units),
        left.total_cost_upper_bound
            .elapsed_millis
            .cmp(&right.total_cost_upper_bound.elapsed_millis),
    ];
    dimensions.iter().all(|order| *order != Ordering::Greater)
        && dimensions.contains(&Ordering::Less)
}

fn evaluate_one_query(
    base: &DecisionCase,
    query: &Query,
) -> Result<(QueryReport, Vec<PolicyMetric>), String> {
    let mut report = QueryReport {
        query_id: query.query_id.clone(),
        status: "READY".to_owned(),
        observable_outcome_ids: Vec::new(),
        observation_class_world_ids: BTreeMap::new(),
        common_feasible_action_ids_by_outcome: BTreeMap::new(),
        generated_policy_count: 0,
        policy_ids: Vec::new(),
    };

    if query.admissibility_status == "prohibited" {
        report.status = "QUERY_PROHIBITED".to_owned();
        return Ok((report, Vec::new()));
    }
    if query.admissibility_status != "admissible" {
        report.status = "QUERY_ADMISSIBILITY_INDETERMINATE".to_owned();
        return Ok((report, Vec::new()));
    }
    if query.evidence_status == "out_of_profile" {
        report.status = "QUERY_OUT_OF_PROFILE".to_owned();
        return Ok((report, Vec::new()));
    }
    if query.evidence_status != "admissible" {
        report.status = "QUERY_EVIDENCE_INDETERMINATE".to_owned();
        return Ok((report, Vec::new()));
    }

    let classes = build_observation_classes(&query.world_observations);
    report.observable_outcome_ids = classes.keys().cloned().collect();
    report.observation_class_world_ids = classes.clone();

    let mut choices = Vec::with_capacity(classes.len());
    for (outcome_id, worlds) in &classes {
        let rows = class_scenarios(&base.scenario_manifest.scenarios, worlds);
        let common = common_feasible_actions(&base.actions, &rows)?;
        report
            .common_feasible_action_ids_by_outcome
            .insert(outcome_id.clone(), common.clone());
        if common.is_empty() {
            report.status = "NO_SAFE_POLICY_FOR_OUTCOME".to_owned();
            return Ok((report, Vec::new()));
        }
        choices.push((outcome_id.clone(), common));
    }

    let product_size = choices.iter().try_fold(1usize, |product, (_, actions)| {
        product.checked_mul(actions.len())
    });
    if product_size.is_none_or(|value| value > MAX_POLICIES_PER_QUERY) {
        report.status = "POLICY_ENUMERATION_LIMIT_EXCEEDED".to_owned();
        return Ok((report, Vec::new()));
    }

    let mut branch_candidates = Vec::new();
    enumerate_policy_branches(
        &choices,
        0,
        &mut BTreeMap::new(),
        &mut branch_candidates,
    );

    let mut policies = Vec::with_capacity(branch_candidates.len());
    for branches in branch_candidates {
        let metric = build_policy(
            base,
            Some(&query.query_id),
            &query.cost,
            &query.world_observations,
            &classes,
            &report.common_feasible_action_ids_by_outcome,
            branches,
        )?;
        report.policy_ids.push(metric.policy_id.clone());
        policies.push(metric);
    }
    report.generated_policy_count = policies.len();
    Ok((report, policies))
}

fn evaluate_no_query(base: &DecisionCase) -> Result<Vec<PolicyMetric>, String> {
    let observations = base
        .compatible_world_set
        .world_ids
        .iter()
        .map(|world| (world.clone(), NO_QUERY_OUTCOME.to_owned()))
        .collect::<BTreeMap<_, _>>();
    let classes = BTreeMap::from([(
        NO_QUERY_OUTCOME.to_owned(),
        base.compatible_world_set.world_ids.clone(),
    )]);
    let rows = class_scenarios(
        &base.scenario_manifest.scenarios,
        &base.compatible_world_set.world_ids,
    );
    let common = common_feasible_actions(&base.actions, &rows)?;
    if common.is_empty() {
        return Ok(Vec::new());
    }
    let class_options = BTreeMap::from([(NO_QUERY_OUTCOME.to_owned(), common.clone())]);
    let choices = vec![(NO_QUERY_OUTCOME.to_owned(), common)];
    let mut branches = Vec::new();
    enumerate_policy_branches(&choices, 0, &mut BTreeMap::new(), &mut branches);
    let zero_cost = ActionCost {
        reference_trials: 0,
        exposure_micro_units: 0,
        elapsed_millis: 0,
    };
    branches
        .into_iter()
        .map(|mapping| {
            build_policy(
                base,
                None,
                &zero_cost,
                &observations,
                &classes,
                &class_options,
                mapping,
            )
        })
        .collect()
}

fn evaluate_case(case: &DiscoveryCase) -> Result<CaseResult, String> {
    let base_corpus = DecisionCorpus {
        schema: "luminous.orbital-feasibility-regret.corpus.v0.1".to_owned(),
        evidence_class: "synthetic".to_owned(),
        claim_ceiling: "finite-joint-feasibility-and-regret-algebra-only".to_owned(),
        cases: vec![case.base_decision.clone()],
    };
    let base_evaluation = crate::feasibility_regret::evaluate(&base_corpus)?;
    let base_result = base_evaluation
        .cases
        .first()
        .ok_or_else(|| "base feasibility evaluator returned no case".to_owned())?;
    let base_status = base_result.status.clone();

    if matches!(
        base_status.as_str(),
        "ACTION_UNIVERSE_OPEN"
            | "ACTION_UNIVERSE_INDETERMINATE"
            | "ACTION_ADMISSIBILITY_INDETERMINATE"
            | "OUT_OF_PROFILE"
            | "LOSS_MODEL_INDETERMINATE"
            | "FEASIBILITY_INDETERMINATE"
            | "NO_FEASIBLE_ACTION_IN_SCENARIO"
    ) {
        return Ok(empty_case_result(case, "BASE_DECISION_INDETERMINATE", &base_status));
    }

    validate_query_universe(case)?;
    let mut policy_metrics = evaluate_no_query(&case.base_decision)?;
    let no_query_policy_ids = policy_metrics
        .iter()
        .map(|item| item.policy_id.clone())
        .collect::<Vec<_>>();
    let no_query_best = policy_metrics
        .iter()
        .map(|item| &item.worst_case_loss)
        .min_by(|left, right| left.compare_exact(right))
        .map(Rational::canonicalized);

    let mut sorted_queries = case.query_universe.queries.iter().collect::<Vec<_>>();
    sorted_queries.sort_by(|left, right| left.query_id.cmp(&right.query_id));
    let mut query_reports = Vec::with_capacity(sorted_queries.len());
    for query in sorted_queries {
        let (report, mut policies) = evaluate_one_query(&case.base_decision, query)?;
        query_reports.push(report);
        policy_metrics.append(&mut policies);
    }

    for policy in &mut policy_metrics {
        if policy.query_id.is_some() {
            if let Some(baseline) = &no_query_best {
                policy.worst_loss_reduction_vs_best_no_query =
                    Some(signed_difference(baseline, &policy.worst_case_loss)?);
            }
        }
    }

    policy_metrics.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
    let policy_pareto_frontier_ids = policy_metrics
        .iter()
        .filter(|candidate| {
            !policy_metrics
                .iter()
                .any(|other| other.policy_id != candidate.policy_id && dominates(other, candidate))
        })
        .map(|metric| metric.policy_id.clone())
        .collect::<Vec<_>>();

    let query_policy_count = policy_metrics.iter().filter(|item| item.query_id.is_some()).count();
    let has_no_query = !no_query_policy_ids.is_empty();
    let status = if policy_metrics.is_empty() {
        "NO_FEASIBLE_QUERY_OR_STATIC_POLICY"
    } else if !has_no_query && query_policy_count > 0 {
        "QUERY_CONDITIONED_POLICIES_AVAILABLE_NO_STATIC_BASELINE"
    } else {
        "POLICY_FRONTIER_AVAILABLE"
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        base_feasibility_status: base_status,
        compatible_world_ids_sha256: case.base_decision.compatible_world_set.world_ids_sha256.clone(),
        action_ids_sha256: case.base_decision.action_universe.expected_action_ids_sha256.clone(),
        scenario_manifest_sha256: case.base_decision.scenario_manifest.scenarios_sha256.clone(),
        query_universe_receipt_id: case.query_universe.source_receipt_id.clone(),
        query_universe_scope_id: case.query_universe.scope_id.clone(),
        expected_query_ids_sha256: case.query_universe.expected_query_ids_sha256.clone(),
        queries_sha256: case.query_universe.queries_sha256.clone(),
        no_query_policy_ids,
        query_reports,
        policy_metrics,
        policy_pareto_frontier_ids,
        best_no_query_worst_case_loss: no_query_best,
    })
}

pub fn evaluate(corpus: &DiscoveryCorpus) -> Result<Evaluation, String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported feasibility-discovery corpus or claim ceiling".to_owned());
    }
    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !case_ids.insert(case.case_id.as_str()) {
            return Err("discovery case IDs must be unique".to_owned());
        }
        validate_query_universe(case)?;
    }

    let mut cases = corpus
        .cases
        .iter()
        .map(evaluate_case)
        .collect::<Result<Vec<_>, _>>()?;
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let summary = Summary {
        case_count: cases.len(),
        base_decision_indeterminate_cases: cases
            .iter()
            .filter(|item| item.status == "BASE_DECISION_INDETERMINATE")
            .count(),
        no_query_policy_cases: cases
            .iter()
            .filter(|item| !item.no_query_policy_ids.is_empty())
            .count(),
        no_safe_query_policy_cases: cases
            .iter()
            .flat_map(|item| &item.query_reports)
            .filter(|report| report.status == "NO_SAFE_POLICY_FOR_OUTCOME")
            .count(),
        query_evidence_indeterminate_count: cases
            .iter()
            .flat_map(|item| &item.query_reports)
            .filter(|report| {
                matches!(
                    report.status.as_str(),
                    "QUERY_EVIDENCE_INDETERMINATE" | "QUERY_OUT_OF_PROFILE"
                )
            })
            .count(),
        query_prohibited_count: cases
            .iter()
            .flat_map(|item| &item.query_reports)
            .filter(|report| report.status == "QUERY_PROHIBITED")
            .count(),
        generated_policy_count: cases.iter().map(|item| item.policy_metrics.len()).sum(),
        pareto_frontier_policy_count: cases
            .iter()
            .map(|item| item.policy_pareto_frontier_ids.len())
            .sum(),
        positive_worst_loss_reduction_policy_count: cases
            .iter()
            .flat_map(|item| &item.policy_metrics)
            .filter(|metric| {
                metric
                    .worst_loss_reduction_vs_best_no_query
                    .as_ref()
                    .is_some_and(|difference| difference.direction == "positive_improvement")
            })
            .count(),
        policy_enumeration_limit_cases: cases
            .iter()
            .flat_map(|item| &item.query_reports)
            .filter(|report| report.status == "POLICY_ENUMERATION_LIMIT_EXCEEDED")
            .count(),
    };

    Ok(Evaluation {
        result: "pass".to_owned(),
        evidence_class: corpus.evidence_class.clone(),
        claim_ceiling: corpus.claim_ceiling.clone(),
        summary,
        cases,
        physical_qualification_granted: false,
        authority_granted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> DiscoveryCorpus {
        serde_json::from_str(include_str!("../fixtures/v0.1/feasibility-discovery-cases.json"))
            .expect("feasibility discovery fixture parses")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("discovery case exists")
    }

    #[test]
    fn safe_query_can_resolve_disjoint_static_feasibility_without_hidden_world_branching() {
        let evaluation = evaluate(&fixture()).expect("discovery corpus evaluates");
        let result = case(&evaluation, "query-resolves-disjoint-worlds");
        assert_eq!(
            result.status,
            "QUERY_CONDITIONED_POLICIES_AVAILABLE_NO_STATIC_BASELINE"
        );
        assert!(result.no_query_policy_ids.is_empty());
        let query = result
            .query_reports
            .iter()
            .find(|item| item.query_id == "safe-identifying-query")
            .expect("query result exists");
        assert_eq!(query.status, "READY");
        assert_eq!(query.generated_policy_count, 1);
        let metric = result
            .policy_metrics
            .iter()
            .find(|item| item.query_id.as_deref() == Some("safe-identifying-query"))
            .expect("query policy exists");
        assert_eq!(
            metric.branch_actions,
            BTreeMap::from([
                ("north".to_owned(), "north_action".to_owned()),
                ("south".to_owned(), "south_action".to_owned())
            ])
        );
        assert_eq!(metric.worst_case_loss.numerator, 3);
        assert_eq!(metric.worst_case_loss.denominator, 10);
        assert!(metric
            .per_scenario
            .iter()
            .all(|scenario| scenario.wait_and_see_regret.numerator == 0));
    }

    #[test]
    fn query_that_merges_incompatible_worlds_has_no_safe_policy() {
        let evaluation = evaluate(&fixture()).expect("discovery corpus evaluates");
        let result = case(&evaluation, "query-merges-incompatible-worlds");
        let query = result
            .query_reports
            .iter()
            .find(|item| item.query_id == "blind-query")
            .expect("blind query exists");
        assert_eq!(query.status, "NO_SAFE_POLICY_FOR_OUTCOME");
        assert_eq!(query.generated_policy_count, 0);
    }

    #[test]
    fn dominated_positive_cost_query_does_not_enter_policy_frontier() {
        let evaluation = evaluate(&fixture()).expect("discovery corpus evaluates");
        let result = case(&evaluation, "no_query_dominates_paid_query");
        assert!(!result.no_query_policy_ids.is_empty());
        let query_policy_ids = result
            .policy_metrics
            .iter()
            .filter(|item| item.query_id.as_deref() == Some("redundant-query"))
            .map(|item| item.policy_id.as_str())
            .collect::<Vec<_>>();
        assert!(!query_policy_ids.is_empty());
        for policy_id in query_policy_ids {
            assert!(!result.policy_pareto_frontier_ids.iter().any(|frontier| frontier == policy_id));
        }
    }

    #[test]
    fn inadmissible_high_information_query_is_not_executed() {
        let evaluation = evaluate(&fixture()).expect("discovery corpus evaluates");
        let result = case(&evaluation, "query-merges-incompatible-worlds");
        let query = result
            .query_reports
            .iter()
            .find(|item| item.query_id == "prohibited-query")
            .expect("prohibited query exists");
        assert_eq!(query.status, "QUERY_PROHIBITED");
        assert!(query.policy_ids.is_empty());
    }

    #[test]
    fn query_and_branch_input_order_does_not_change_policy_frontier() {
        let input = fixture();
        let mut reversed = fixture();
        reversed.cases.reverse();
        for case in &mut reversed.cases {
            case.query_universe.expected_query_ids.reverse();
            case.query_universe.queries.reverse();
            for query in &mut case.query_universe.queries {
                query.expected_world_ids.reverse();
                query.world_observations = query
                    .world_observations
                    .iter()
                    .rev()
                    .map(|(world, observation)| (world.clone(), observation.clone()))
                    .collect();
            }
            case.base_decision.actions.reverse();
            case.base_decision.compatible_world_set.world_ids.reverse();
            case.base_decision.scenario_manifest.scenarios.reverse();
        }
        assert_eq!(
            serde_json::to_value(evaluate(&input).expect("original evaluates")).unwrap(),
            serde_json::to_value(evaluate(&reversed).expect("reversed evaluates")).unwrap()
        );
    }

    #[test]
    fn outcome_class_regret_uses_one_common_action_not_hidden_scenario_oracle() {
        let mut corpus = fixture();
        let decision = &mut corpus.cases[1].base_decision;
        let safe = decision
            .scenario_manifest
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.scenario_id == "v0-s")
            .expect("v0 scenario exists");
        safe.action_outcomes
            .get_mut("safe")
            .expect("safe row exists")
            .loss = Some(Rational {
            numerator: 1,
            denominator: 10,
        });
        safe.action_outcomes
            .get_mut("alternate")
            .expect("alternate row exists")
            .loss = Some(Rational {
            numerator: 6,
            denominator: 10,
        });
        let other = decision
            .scenario_manifest
            .scenarios
            .iter_mut()
            .find(|scenario| scenario.scenario_id == "v1-s")
            .expect("v1 scenario exists");
        other
            .action_outcomes
            .get_mut("safe")
            .expect("safe row exists")
            .loss = Some(Rational {
            numerator: 9,
            denominator: 10,
        });
        other
            .action_outcomes
            .get_mut("alternate")
            .expect("alternate row exists")
            .loss = Some(Rational {
            numerator: 6,
            denominator: 10,
        });
        decision.scenario_manifest.scenarios_sha256 =
            digest_scenarios(&decision.scenario_manifest.scenarios)
                .expect("mutated scenario rows can be digested");

        let evaluation = evaluate(&corpus).expect("mutated corpus evaluates");
        let result = case(&evaluation, "no_query_dominates_paid_query");
        let policy = result
            .policy_metrics
            .iter()
            .find(|metric| {
                metric.query_id.as_deref() == Some("redundant-query")
                    && metric.branch_actions.get("same").is_some_and(|id| id == "alternate")
            })
            .expect("alternate branch policy exists");
        assert_eq!(policy.worst_case_wait_and_see_regret.numerator, 1);
        assert_eq!(policy.worst_case_wait_and_see_regret.denominator, 2);
        assert_eq!(policy.worst_case_outcome_class_regret.numerator, 0);
        assert_eq!(policy.worst_case_outcome_class_regret.denominator, 1);

        let safe_policy = result
            .policy_metrics
            .iter()
            .find(|metric| {
                metric.query_id.as_deref() == Some("redundant-query")
                    && metric.branch_actions.get("same").is_some_and(|id| id == "safe")
            })
            .expect("safe branch policy exists");
        assert_eq!(safe_policy.worst_case_outcome_class_regret.numerator, 3);
        assert_eq!(safe_policy.worst_case_outcome_class_regret.denominator, 10);
    }

    #[test]
    fn no_physical_qualification_or_authority_is_granted() {
        let evaluation = evaluate(&fixture()).expect("discovery corpus evaluates");
        assert!(!evaluation.physical_qualification_granted);
        assert!(!evaluation.authority_granted);
    }
}
