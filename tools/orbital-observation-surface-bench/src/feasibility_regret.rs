//! Exact joint feasibility and regret over a declared finite action universe.
//!
//! The evaluator requires an upstream-declared exhaustive action universe and explicit
//! per-scenario feasibility outcomes. It computes the intersection of scenario-feasible
//! sets for one fixed, non-anticipating action. It reports both wait-and-see regret
//! (against each scenario's feasible set) and robust-set regret (against the intersection).
//! It does not authenticate closure evidence, calibrate feasibility models, infer scenario
//! probabilities, select actions, or grant execution authority.

use crate::decision_identifiability::Rational;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-feasibility-regret.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-joint-feasibility-and-regret-algebra-only";
const MAX_CASES: usize = 256;
const MAX_WORLDS: usize = 64;
const MAX_ACTIONS: usize = 128;
const MAX_SCENARIOS: usize = 4096;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibleWorldSet {
    pub source_receipt_id: String,
    pub world_ids_sha256: String,
    pub world_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionUniverseManifest {
    pub source_receipt_id: String,
    pub scope_id: String,
    /// finite_exhaustive is a declared closure claim, not issuer-authenticated proof.
    pub closure_kind: String,
    pub enumeration_method: String,
    pub evidence_status: String,
    pub expected_action_ids: Vec<String>,
    pub expected_action_ids_sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionCost {
    pub reference_trials: u64,
    pub exposure_micro_units: u64,
    pub elapsed_millis: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub action_id: String,
    pub eligibility_status: String,
    pub eligibility_receipt_id: Option<String>,
    pub cost: ActionCost,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionOutcome {
    pub feasibility_status: String,
    pub loss: Option<Rational>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FeasibilityScenario {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
    pub action_outcomes: BTreeMap<String, ActionOutcome>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioManifest {
    pub source_manifest_id: String,
    pub expected_scenario_ids: Vec<String>,
    pub expected_scenario_ids_sha256: String,
    pub scenarios_sha256: String,
    pub scenarios: Vec<FeasibilityScenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCase {
    pub case_id: String,
    pub decision_scope_id: String,
    pub compatible_world_set: CompatibleWorldSet,
    pub action_universe: ActionUniverseManifest,
    pub scenario_manifest: ScenarioManifest,
    pub actions: Vec<Action>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCorpus {
    pub schema: String,
    pub evidence_class: String,
    pub claim_ceiling: String,
    pub cases: Vec<DecisionCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionFeasibilityClass {
    pub action_id: String,
    pub classification: String,
    pub feasible_scenario_count: usize,
    pub infeasible_scenario_count: usize,
    pub eligibility_status: String,
    pub eligibility_receipt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioFeasibleSet {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub feasible_action_ids: Vec<String>,
    pub infeasible_action_ids: Vec<String>,
    pub prohibited_action_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioRegret {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub loss: Rational,
    pub wait_and_see_regret: Rational,
    pub robust_set_regret: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionMetric {
    pub action_id: String,
    pub eligibility_receipt_id: String,
    pub worst_case_loss: Rational,
    pub worst_case_wait_and_see_regret: Rational,
    pub worst_case_robust_set_regret: Rational,
    pub worst_loss_scenario_ids: Vec<String>,
    pub worst_wait_and_see_scenario_ids: Vec<String>,
    pub worst_robust_set_scenario_ids: Vec<String>,
    pub uniformly_best_against_scenario_feasible_sets: bool,
    pub cost: ActionCost,
    pub per_scenario: Vec<ScenarioRegret>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub compatible_world_set_receipt_id: String,
    pub compatible_world_ids_sha256: String,
    pub action_universe_receipt_id: String,
    pub action_universe_scope_id: String,
    pub action_universe_closure_kind: String,
    pub action_universe_enumeration_method: String,
    pub action_ids_sha256: String,
    pub scenario_manifest_id: String,
    pub expected_scenario_ids_sha256: String,
    pub scenarios_sha256: String,
    pub compatible_world_count: usize,
    pub action_count: usize,
    pub scenario_count: usize,
    pub robust_feasible_action_ids: Vec<String>,
    pub scenario_feasible_action_ids: Vec<String>,
    pub action_feasibility_classes: Vec<ActionFeasibilityClass>,
    pub scenario_feasible_sets: Vec<ScenarioFeasibleSet>,
    pub minimax_wait_and_see_regret_action_ids: Vec<String>,
    pub minimax_robust_set_regret_action_ids: Vec<String>,
    pub pareto_frontier_action_ids: Vec<String>,
    pub action_metrics: Vec<ActionMetric>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub action_universe_open_cases: usize,
    pub action_universe_indeterminate_cases: usize,
    pub action_admissibility_indeterminate_cases: usize,
    pub feasibility_indeterminate_cases: usize,
    pub loss_model_indeterminate_cases: usize,
    pub no_feasible_scenario_cases: usize,
    pub no_robust_feasible_action_cases: usize,
    pub robust_decision_cases: usize,
    pub declared_action_count: usize,
    pub robust_feasible_action_count: usize,
    pub sometimes_feasible_action_count: usize,
    pub never_feasible_action_count: usize,
    pub pareto_frontier_action_count: usize,
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

fn hex_sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn digest_ids(ids: &[String]) -> Result<String, String> {
    let mut sorted = ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical IDs: {error}"))?;
    Ok(hex_sha256(&bytes))
}

fn canonical_scenarios(scenarios: &[FeasibilityScenario]) -> Result<Value, String> {
    let mut sorted = scenarios.to_vec();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    serde_json::to_value(sorted)
        .map_err(|error| format!("failed to encode canonical feasibility scenarios: {error}"))
}

fn digest_scenarios(scenarios: &[FeasibilityScenario]) -> Result<String, String> {
    let canonical = canonical_scenarios(scenarios)?;
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to serialize feasibility scenarios: {error}"))?;
    Ok(hex_sha256(&bytes))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn validate_corpus(corpus: &DecisionCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported feasibility-regret corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !valid_id(&case.case_id)
            || !case_ids.insert(case.case_id.as_str())
            || !valid_id(&case.decision_scope_id)
            || !valid_id(&case.compatible_world_set.source_receipt_id)
            || !valid_id(&case.action_universe.source_receipt_id)
            || !valid_id(&case.action_universe.scope_id)
            || !valid_id(&case.action_universe.enumeration_method)
            || !valid_id(&case.scenario_manifest.source_manifest_id)
            || case.compatible_world_set.world_ids.is_empty()
            || case.compatible_world_set.world_ids.len() > MAX_WORLDS
            || case.action_universe.expected_action_ids.is_empty()
            || case.action_universe.expected_action_ids.len() > MAX_ACTIONS
            || case.scenario_manifest.scenarios.is_empty()
            || case.scenario_manifest.scenarios.len() > MAX_SCENARIOS
            || case.actions.is_empty()
            || case.actions.len() > MAX_ACTIONS
        {
            return Err("case identity, manifests or finite fixture bounds are invalid".to_owned());
        }

        let mut worlds = BTreeSet::new();
        for world_id in &case.compatible_world_set.world_ids {
            if !valid_id(world_id) || !worlds.insert(world_id.as_str()) {
                return Err("compatible-world IDs must be unique nonempty ASCII strings".to_owned());
            }
        }
        let world_digest = digest_ids(&case.compatible_world_set.world_ids)?;
        if case.compatible_world_set.world_ids_sha256 != world_digest {
            return Err("compatible-world ID digest mismatch".to_owned());
        }

        if case.action_universe.scope_id != case.decision_scope_id {
            return Err("action-universe scope does not match the decision scope".to_owned());
        }
        if !matches!(
            case.action_universe.closure_kind.as_str(),
            "finite_exhaustive" | "open" | "indeterminate"
        ) || !matches!(
            case.action_universe.evidence_status.as_str(),
            "admissible" | "indeterminate" | "out_of_scope"
        ) {
            return Err("action-universe closure kind or evidence status is invalid".to_owned());
        }
        if case.action_universe.closure_kind == "finite_exhaustive"
            && case.action_universe.evidence_status == "admissible"
            && case.action_universe.source_receipt_id.trim().is_empty()
        {
            return Err(
                "exhaustive action universe requires a closure evidence receipt".to_owned(),
            );
        }

        let mut expected_actions = BTreeSet::new();
        for action_id in &case.action_universe.expected_action_ids {
            if !valid_id(action_id) || !expected_actions.insert(action_id.as_str()) {
                return Err("planned action IDs must be unique nonempty ASCII strings".to_owned());
            }
        }
        if digest_ids(&case.action_universe.expected_action_ids)?
            != case.action_universe.expected_action_ids_sha256
        {
            return Err("planned action-ID inventory digest mismatch".to_owned());
        }

        let mut action_ids = BTreeSet::new();
        for action in &case.actions {
            if !valid_id(&action.action_id)
                || !action_ids.insert(action.action_id.as_str())
                || !matches!(
                    action.eligibility_status.as_str(),
                    "eligible" | "prohibited" | "indeterminate"
                )
                || (action.eligibility_status != "eligible"
                    && action
                        .eligibility_receipt_id
                        .as_deref()
                        .is_none_or(|value| !valid_id(value)))
                || (action.eligibility_status == "eligible"
                    && action
                        .eligibility_receipt_id
                        .as_deref()
                        .is_none_or(|value| !valid_id(value)))
                || [
                    action.cost.reference_trials,
                    action.cost.exposure_micro_units,
                    action.cost.elapsed_millis,
                ]
                .iter()
                .any(|value| *value > MAX_SAFE_INTEGER)
            {
                return Err(
                    "action identity, eligibility evidence or native cost is invalid".to_owned(),
                );
            }
        }
        if action_ids != expected_actions {
            return Err(
                "observed action IDs do not match the planned exhaustive action inventory"
                    .to_owned(),
            );
        }

        if !valid_sha256(&case.action_universe.expected_action_ids_sha256)
            || !valid_sha256(&case.scenario_manifest.expected_scenario_ids_sha256)
            || !valid_sha256(&case.scenario_manifest.scenarios_sha256)
        {
            return Err("action or scenario manifest digest is malformed".to_owned());
        }

        let mut expected_scenarios = BTreeSet::new();
        for scenario_id in &case.scenario_manifest.expected_scenario_ids {
            if !valid_id(scenario_id) || !expected_scenarios.insert(scenario_id.as_str()) {
                return Err("planned scenario IDs must be unique nonempty ASCII strings".to_owned());
            }
        }
        if digest_ids(&case.scenario_manifest.expected_scenario_ids)?
            != case.scenario_manifest.expected_scenario_ids_sha256
        {
            return Err("planned scenario-ID inventory digest mismatch".to_owned());
        }

        let mut observed_scenarios = BTreeSet::new();
        let mut covered_worlds = BTreeSet::new();
        for scenario in &case.scenario_manifest.scenarios {
            if !valid_id(&scenario.scenario_id)
                || !observed_scenarios.insert(scenario.scenario_id.as_str())
                || !valid_id(&scenario.world_id)
                || !worlds.contains(scenario.world_id.as_str())
                || !valid_id(&scenario.loss_model_id)
                || !matches!(
                    scenario.evidence_status.as_str(),
                    "admissible" | "indeterminate" | "out_of_profile"
                )
                || (scenario.evidence_status == "admissible"
                    && scenario
                        .evidence_receipt_id
                        .as_deref()
                        .is_none_or(|value| !valid_id(value)))
            {
                return Err(
                    "scenario identity, world, evidence status or receipt is invalid".to_owned(),
                );
            }
            covered_worlds.insert(scenario.world_id.as_str());

            let supplied = scenario
                .action_outcomes
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if supplied != action_ids {
                return Err(format!(
                    "scenario {} must include exactly one outcome for every planned action",
                    scenario.scenario_id
                ));
            }

            for action in &case.actions {
                let outcome = scenario
                    .action_outcomes
                    .get(&action.action_id)
                    .ok_or_else(|| {
                        format!(
                            "scenario {} is missing action {}",
                            scenario.scenario_id, action.action_id
                        )
                    })?;
                match action.eligibility_status.as_str() {
                    "prohibited" => {
                        if outcome.feasibility_status != "prohibited" || outcome.loss.is_some() {
                            return Err(format!(
                                "prohibited action {} must have a prohibited null-loss outcome",
                                action.action_id
                            ));
                        }
                    }
                    "indeterminate" => {
                        if outcome.feasibility_status != "indeterminate" || outcome.loss.is_some() {
                            return Err(format!(
                                "indeterminate action {} must have an indeterminate null-loss outcome",
                                action.action_id
                            ));
                        }
                    }
                    "eligible" => match outcome.feasibility_status.as_str() {
                        "feasible" => {
                            let loss = outcome.loss.as_ref().ok_or_else(|| {
                                format!("feasible action {} has no scenario loss", action.action_id)
                            })?;
                            loss.validate()
                                .map_err(|error| format!("scenario loss is invalid: {error}"))?;
                        }
                        "infeasible" | "indeterminate" => {
                            if outcome.loss.is_some() {
                                return Err(format!(
                                    "non-feasible action {} must not carry a loss",
                                    action.action_id
                                ));
                            }
                        }
                        _ => {
                            return Err(format!(
                                "eligible action {} has an invalid scenario feasibility status",
                                action.action_id
                            ));
                        }
                    },
                    _ => return Err("unsupported static action eligibility status".to_owned()),
                }
            }
        }

        if observed_scenarios != expected_scenarios {
            return Err(
                "observed scenario IDs do not match the upstream planned scenario inventory"
                    .to_owned(),
            );
        }
        if covered_worlds != worlds {
            return Err("scenario manifest must cover every compatible physical world".to_owned());
        }
        if digest_scenarios(&case.scenario_manifest.scenarios)?
            != case.scenario_manifest.scenarios_sha256
        {
            return Err("full scenario outcome manifest digest mismatch".to_owned());
        }
    }

    Ok(())
}

fn empty_case_result(case: &DecisionCase, status: &str) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
        compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
        action_universe_receipt_id: case.action_universe.source_receipt_id.clone(),
        action_universe_scope_id: case.action_universe.scope_id.clone(),
        action_universe_closure_kind: case.action_universe.closure_kind.clone(),
        action_universe_enumeration_method: case.action_universe.enumeration_method.clone(),
        action_ids_sha256: case.action_universe.expected_action_ids_sha256.clone(),
        scenario_manifest_id: case.scenario_manifest.source_manifest_id.clone(),
        expected_scenario_ids_sha256: case.scenario_manifest.expected_scenario_ids_sha256.clone(),
        scenarios_sha256: case.scenario_manifest.scenarios_sha256.clone(),
        compatible_world_count: case.compatible_world_set.world_ids.len(),
        action_count: case.actions.len(),
        scenario_count: case.scenario_manifest.scenarios.len(),
        robust_feasible_action_ids: Vec::new(),
        scenario_feasible_action_ids: Vec::new(),
        action_feasibility_classes: Vec::new(),
        scenario_feasible_sets: Vec::new(),
        minimax_wait_and_see_regret_action_ids: Vec::new(),
        minimax_robust_set_regret_action_ids: Vec::new(),
        pareto_frontier_action_ids: Vec::new(),
        action_metrics: Vec::new(),
    }
}

fn argmin_ids<F>(metrics: &[ActionMetric], value: F) -> Vec<String>
where
    F: for<'a> Fn(&'a ActionMetric) -> &'a Rational,
{
    let Some(best) = metrics
        .iter()
        .map(&value)
        .min_by(|left, right| left.compare_exact(right))
    else {
        return Vec::new();
    };
    metrics
        .iter()
        .filter(|metric| value(metric).exactly_equals(best))
        .map(|metric| metric.action_id.clone())
        .collect()
}

fn dominates(left: &ActionMetric, right: &ActionMetric) -> bool {
    let dimensions = [
        left.worst_case_loss.compare_exact(&right.worst_case_loss),
        left.worst_case_wait_and_see_regret
            .compare_exact(&right.worst_case_wait_and_see_regret),
        left.worst_case_robust_set_regret
            .compare_exact(&right.worst_case_robust_set_regret),
        left.cost.reference_trials.cmp(&right.cost.reference_trials),
        left.cost
            .exposure_micro_units
            .cmp(&right.cost.exposure_micro_units),
        left.cost.elapsed_millis.cmp(&right.cost.elapsed_millis),
    ];
    dimensions.iter().all(|order| *order != Ordering::Greater)
        && dimensions.contains(&Ordering::Less)
}

fn evaluate_case(case: &DecisionCase) -> Result<CaseResult, String> {
    if case.action_universe.closure_kind == "open" {
        return Ok(empty_case_result(case, "ACTION_UNIVERSE_OPEN"));
    }
    if case.action_universe.closure_kind == "indeterminate"
        || case.action_universe.evidence_status != "admissible"
    {
        return Ok(empty_case_result(case, "ACTION_UNIVERSE_INDETERMINATE"));
    }

    if case
        .actions
        .iter()
        .any(|action| action.eligibility_status == "indeterminate")
    {
        return Ok(empty_case_result(
            case,
            "ACTION_ADMISSIBILITY_INDETERMINATE",
        ));
    }

    if case
        .scenario_manifest
        .scenarios
        .iter()
        .any(|scenario| scenario.evidence_status == "out_of_profile")
    {
        return Ok(empty_case_result(case, "OUT_OF_PROFILE"));
    }
    if case
        .scenario_manifest
        .scenarios
        .iter()
        .any(|scenario| scenario.evidence_status != "admissible")
    {
        return Ok(empty_case_result(case, "LOSS_MODEL_INDETERMINATE"));
    }

    let mut scenarios = case.scenario_manifest.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut scenario_sets = Vec::with_capacity(scenarios.len());
    let mut per_action_feasible_count = BTreeMap::<String, usize>::new();
    let mut per_scenario_best = BTreeMap::<String, Rational>::new();
    let mut robust_set: Option<BTreeSet<String>> = None;

    for scenario in &scenarios {
        let mut feasible = Vec::new();
        let mut infeasible = Vec::new();
        let mut prohibited = Vec::new();
        let mut scenario_best: Option<Rational> = None;

        for action in &case.actions {
            let outcome = scenario
                .action_outcomes
                .get(&action.action_id)
                .ok_or_else(|| {
                    format!(
                        "missing outcome for {} / {}",
                        scenario.scenario_id, action.action_id
                    )
                })?;
            match outcome.feasibility_status.as_str() {
                "feasible" => {
                    feasible.push(action.action_id.clone());
                    *per_action_feasible_count
                        .entry(action.action_id.clone())
                        .or_default() += 1;
                    let loss = outcome.loss.as_ref().ok_or_else(|| {
                        format!("feasible action {} has no loss", action.action_id)
                    })?;
                    if scenario_best
                        .as_ref()
                        .is_none_or(|best| loss.compare_exact(best) == Ordering::Less)
                    {
                        scenario_best = Some(loss.canonicalized());
                    }
                }
                "infeasible" => infeasible.push(action.action_id.clone()),
                "prohibited" => prohibited.push(action.action_id.clone()),
                "indeterminate" => {
                    return Ok(empty_case_result(case, "FEASIBILITY_INDETERMINATE"));
                }
                _ => {
                    return Err("validated scenario contains unknown feasibility status".to_owned());
                }
            }
        }

        if feasible.is_empty() {
            return Ok(empty_case_result(case, "NO_FEASIBLE_ACTION_IN_SCENARIO"));
        }
        let best =
            scenario_best.ok_or_else(|| "scenario best feasible loss is missing".to_owned())?;
        per_scenario_best.insert(scenario.scenario_id.clone(), best);

        let feasible_set = feasible.iter().cloned().collect::<BTreeSet<_>>();
        robust_set = Some(match robust_set {
            Some(current) => current.intersection(&feasible_set).cloned().collect(),
            None => feasible_set,
        });

        feasible.sort();
        infeasible.sort();
        prohibited.sort();
        scenario_sets.push(ScenarioFeasibleSet {
            scenario_id: scenario.scenario_id.clone(),
            world_id: scenario.world_id.clone(),
            loss_model_id: scenario.loss_model_id.clone(),
            feasible_action_ids: feasible,
            infeasible_action_ids: infeasible,
            prohibited_action_ids: prohibited,
        });
    }

    let robust_set = robust_set.unwrap_or_default();
    let mut actions_by_id = case.actions.iter().collect::<Vec<_>>();
    actions_by_id.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    let action_feasibility_classes = actions_by_id
        .iter()
        .map(|action| {
            let count = per_action_feasible_count
                .get(&action.action_id)
                .copied()
                .unwrap_or(0);
            let (classification, feasible_count, infeasible_count) =
                if action.eligibility_status == "prohibited" {
                    ("PROHIBITED", 0, 0)
                } else if count == scenarios.len() {
                    ("ALWAYS_FEASIBLE", count, 0)
                } else if count == 0 {
                    ("NEVER_FEASIBLE", 0, scenarios.len())
                } else {
                    ("SOMETIMES_FEASIBLE", count, scenarios.len() - count)
                };
            ActionFeasibilityClass {
                action_id: action.action_id.clone(),
                classification: classification.to_owned(),
                feasible_scenario_count: feasible_count,
                infeasible_scenario_count: infeasible_count,
                eligibility_status: action.eligibility_status.clone(),
                eligibility_receipt_id: action.eligibility_receipt_id.clone(),
            }
        })
        .collect::<Vec<_>>();

    let scenario_feasible_action_ids = scenario_sets
        .iter()
        .flat_map(|item| item.feasible_action_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let robust_feasible_action_ids = robust_set.iter().cloned().collect::<Vec<_>>();

    if robust_set.is_empty() {
        let mut result = empty_case_result(case, "NO_ROBUST_FEASIBLE_ACTION");
        result.action_feasibility_classes = action_feasibility_classes;
        result.scenario_feasible_sets = scenario_sets;
        result.scenario_feasible_action_ids = scenario_feasible_action_ids;
        return Ok(result);
    }

    let mut robust_actions = case
        .actions
        .iter()
        .filter(|action| robust_set.contains(&action.action_id))
        .collect::<Vec<_>>();
    robust_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let robust_best_by_scenario = scenarios
        .iter()
        .map(|scenario| {
            let best = robust_actions
                .iter()
                .filter_map(|action| scenario.action_outcomes.get(&action.action_id))
                .filter_map(|outcome| outcome.loss.as_ref())
                .min_by(|left, right| left.compare_exact(right))
                .ok_or_else(|| {
                    format!("no robust-set loss in scenario {}", scenario.scenario_id)
                })?;
            Ok((scenario.scenario_id.clone(), best.canonicalized()))
        })
        .collect::<Result<BTreeMap<_, _>, String>>()?;

    let mut action_metrics = Vec::with_capacity(robust_actions.len());
    for action in &robust_actions {
        let eligibility_receipt_id = action
            .eligibility_receipt_id
            .as_deref()
            .filter(|value| valid_id(value))
            .ok_or_else(|| {
                format!(
                    "robust action {} has no eligibility receipt",
                    action.action_id
                )
            })?
            .to_owned();

        let mut per_scenario = Vec::with_capacity(scenarios.len());
        for scenario in &scenarios {
            let outcome = scenario
                .action_outcomes
                .get(&action.action_id)
                .ok_or_else(|| {
                    format!(
                        "robust action {} missing a scenario outcome",
                        action.action_id
                    )
                })?;
            let loss = outcome
                .loss
                .as_ref()
                .ok_or_else(|| format!("robust action {} is missing loss", action.action_id))?
                .canonicalized();
            let wait_and_see_best = per_scenario_best
                .get(&scenario.scenario_id)
                .ok_or_else(|| format!("missing scenario oracle {}", scenario.scenario_id))?;
            let robust_best = robust_best_by_scenario
                .get(&scenario.scenario_id)
                .ok_or_else(|| format!("missing robust-set oracle {}", scenario.scenario_id))?;
            per_scenario.push(ScenarioRegret {
                scenario_id: scenario.scenario_id.clone(),
                world_id: scenario.world_id.clone(),
                loss_model_id: scenario.loss_model_id.clone(),
                loss: loss.clone(),
                wait_and_see_regret: loss
                    .checked_sub(wait_and_see_best)
                    .map_err(|error| format!("wait-and-see regret underflow: {error}"))?,
                robust_set_regret: loss
                    .checked_sub(robust_best)
                    .map_err(|error| format!("robust-set regret underflow: {error}"))?,
            });
        }

        let worst_case_loss = per_scenario
            .iter()
            .map(|item| &item.loss)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "per-scenario loss set unexpectedly empty".to_owned())?
            .canonicalized();
        let worst_case_wait_and_see_regret = per_scenario
            .iter()
            .map(|item| &item.wait_and_see_regret)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "wait-and-see regret set unexpectedly empty".to_owned())?
            .canonicalized();
        let worst_case_robust_set_regret = per_scenario
            .iter()
            .map(|item| &item.robust_set_regret)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "robust-set regret set unexpectedly empty".to_owned())?
            .canonicalized();

        action_metrics.push(ActionMetric {
            action_id: action.action_id.clone(),
            eligibility_receipt_id,
            worst_loss_scenario_ids: per_scenario
                .iter()
                .filter(|item| item.loss.exactly_equals(&worst_case_loss))
                .map(|item| item.scenario_id.clone())
                .collect(),
            worst_wait_and_see_scenario_ids: per_scenario
                .iter()
                .filter(|item| {
                    item.wait_and_see_regret
                        .exactly_equals(&worst_case_wait_and_see_regret)
                })
                .map(|item| item.scenario_id.clone())
                .collect(),
            worst_robust_set_scenario_ids: per_scenario
                .iter()
                .filter(|item| {
                    item.robust_set_regret
                        .exactly_equals(&worst_case_robust_set_regret)
                })
                .map(|item| item.scenario_id.clone())
                .collect(),
            uniformly_best_against_scenario_feasible_sets: per_scenario
                .iter()
                .all(|item| item.wait_and_see_regret.numerator == 0),
            worst_case_loss,
            worst_case_wait_and_see_regret,
            worst_case_robust_set_regret,
            cost: action.cost.clone(),
            per_scenario,
        });
    }

    let minimax_wait_and_see_regret_action_ids = argmin_ids(&action_metrics, |metric| {
        &metric.worst_case_wait_and_see_regret
    });
    let minimax_robust_set_regret_action_ids = argmin_ids(&action_metrics, |metric| {
        &metric.worst_case_robust_set_regret
    });
    let pareto_frontier_action_ids = action_metrics
        .iter()
        .filter(|candidate| {
            !action_metrics
                .iter()
                .any(|other| other.action_id != candidate.action_id && dominates(other, candidate))
        })
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: "ROBUST_FEASIBLE_DECISION_SET_AVAILABLE".to_owned(),
        compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
        compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
        action_universe_receipt_id: case.action_universe.source_receipt_id.clone(),
        action_universe_scope_id: case.action_universe.scope_id.clone(),
        action_universe_closure_kind: case.action_universe.closure_kind.clone(),
        action_universe_enumeration_method: case.action_universe.enumeration_method.clone(),
        action_ids_sha256: case.action_universe.expected_action_ids_sha256.clone(),
        scenario_manifest_id: case.scenario_manifest.source_manifest_id.clone(),
        expected_scenario_ids_sha256: case.scenario_manifest.expected_scenario_ids_sha256.clone(),
        scenarios_sha256: case.scenario_manifest.scenarios_sha256.clone(),
        compatible_world_count: case.compatible_world_set.world_ids.len(),
        action_count: case.actions.len(),
        scenario_count: scenarios.len(),
        robust_feasible_action_ids,
        scenario_feasible_action_ids,
        action_feasibility_classes,
        scenario_feasible_sets: scenario_sets,
        minimax_wait_and_see_regret_action_ids,
        minimax_robust_set_regret_action_ids,
        pareto_frontier_action_ids,
        action_metrics,
    })
}

pub fn evaluate(corpus: &DecisionCorpus) -> Result<Evaluation, String> {
    validate_corpus(corpus)?;
    let mut cases = corpus
        .cases
        .iter()
        .map(evaluate_case)
        .collect::<Result<Vec<_>, _>>()?;
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let summary = Summary {
        case_count: cases.len(),
        action_universe_open_cases: cases
            .iter()
            .filter(|item| item.status == "ACTION_UNIVERSE_OPEN")
            .count(),
        action_universe_indeterminate_cases: cases
            .iter()
            .filter(|item| item.status == "ACTION_UNIVERSE_INDETERMINATE")
            .count(),
        action_admissibility_indeterminate_cases: cases
            .iter()
            .filter(|item| item.status == "ACTION_ADMISSIBILITY_INDETERMINATE")
            .count(),
        feasibility_indeterminate_cases: cases
            .iter()
            .filter(|item| item.status == "FEASIBILITY_INDETERMINATE")
            .count(),
        loss_model_indeterminate_cases: cases
            .iter()
            .filter(|item| {
                item.status == "LOSS_MODEL_INDETERMINATE" || item.status == "OUT_OF_PROFILE"
            })
            .count(),
        no_feasible_scenario_cases: cases
            .iter()
            .filter(|item| item.status == "NO_FEASIBLE_ACTION_IN_SCENARIO")
            .count(),
        no_robust_feasible_action_cases: cases
            .iter()
            .filter(|item| item.status == "NO_ROBUST_FEASIBLE_ACTION")
            .count(),
        robust_decision_cases: cases
            .iter()
            .filter(|item| item.status == "ROBUST_FEASIBLE_DECISION_SET_AVAILABLE")
            .count(),
        declared_action_count: cases.iter().map(|item| item.action_count).sum(),
        robust_feasible_action_count: cases
            .iter()
            .map(|item| item.robust_feasible_action_ids.len())
            .sum(),
        sometimes_feasible_action_count: cases
            .iter()
            .map(|item| {
                item.action_feasibility_classes
                    .iter()
                    .filter(|class| class.classification == "SOMETIMES_FEASIBLE")
                    .count()
            })
            .sum(),
        never_feasible_action_count: cases
            .iter()
            .map(|item| {
                item.action_feasibility_classes
                    .iter()
                    .filter(|class| class.classification == "NEVER_FEASIBLE")
                    .count()
            })
            .sum(),
        pareto_frontier_action_count: cases
            .iter()
            .map(|item| item.pareto_frontier_action_ids.len())
            .sum(),
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

    fn fixture() -> DecisionCorpus {
        serde_json::from_str(include_str!(
            "../fixtures/v0.1/feasibility-regret-cases.json"
        ))
        .expect("feasibility regret corpus must parse")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation
            .cases
            .iter()
            .find(|item| item.case_id == id)
            .expect("case exists")
    }

    #[test]
    fn infeasible_zero_loss_action_never_enters_any_scenario_comparator() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "infeasible-zero-loss-comparator");
        assert_eq!(result.robust_feasible_action_ids, vec!["robust"]);
        let metric = result
            .action_metrics
            .iter()
            .find(|item| item.action_id == "robust")
            .unwrap();
        assert_eq!(metric.worst_case_wait_and_see_regret.numerator, 2);
        assert_eq!(metric.worst_case_wait_and_see_regret.denominator, 5);
        assert_eq!(metric.worst_case_robust_set_regret.numerator, 0);
        assert_eq!(metric.worst_case_robust_set_regret.denominator, 1);
    }

    #[test]
    fn action_rank_reversal_is_retained_across_scenarios() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "action-rank-reversal");
        assert_eq!(result.robust_feasible_action_ids, vec!["alpha", "beta"]);
        let alpha = result
            .action_metrics
            .iter()
            .find(|item| item.action_id == "alpha")
            .expect("alpha metric exists");
        assert_eq!(alpha.worst_case_wait_and_see_regret.numerator, 3);
        assert_eq!(alpha.worst_case_wait_and_see_regret.denominator, 10);
        assert_eq!(
            result.minimax_wait_and_see_regret_action_ids,
            vec!["alpha", "beta"]
        );
    }

    #[test]
    fn static_admissibility_indeterminate_blocks_global_regret() {
        let mut corpus = fixture();
        let decision_case = &mut corpus.cases[0];
        let action = decision_case
            .actions
            .iter_mut()
            .find(|item| item.action_id == "conditional")
            .expect("conditional action exists");
        action.eligibility_status = "indeterminate".to_owned();
        for scenario in &mut decision_case.scenario_manifest.scenarios {
            let outcome = scenario
                .action_outcomes
                .get_mut("conditional")
                .expect("conditional outcome exists");
            outcome.feasibility_status = "indeterminate".to_owned();
            outcome.loss = None;
        }
        decision_case.scenario_manifest.scenarios_sha256 =
            digest_scenarios(&decision_case.scenario_manifest.scenarios)
                .expect("scenario rows can be re-digested");
        let evaluation = evaluate(&corpus).expect("indeterminate evidence is an explicit result");
        assert_eq!(
            case(&evaluation, "infeasible-zero-loss-comparator").status,
            "ACTION_ADMISSIBILITY_INDETERMINATE"
        );
        assert!(
            case(&evaluation, "infeasible-zero-loss-comparator")
                .action_metrics
                .is_empty()
        );
    }

    #[test]
    fn multi_world_intersection_keeps_only_actions_feasible_in_every_row() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "multi-world-multi-loss-model");
        assert_eq!(result.compatible_world_count, 2);
        assert_eq!(result.scenario_count, 4);
        assert_eq!(result.robust_feasible_action_ids, vec!["always"]);
        let conditional = result
            .action_feasibility_classes
            .iter()
            .find(|item| item.action_id == "conditional")
            .expect("conditional classification exists");
        assert_eq!(conditional.classification, "SOMETIMES_FEASIBLE");
        assert_eq!(conditional.feasible_scenario_count, 2);
    }

    #[test]
    fn no_fixed_action_is_selected_when_scenario_feasible_sets_are_disjoint() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "disjoint-feasible-sets");
        assert_eq!(result.status, "NO_ROBUST_FEASIBLE_ACTION");
        assert!(result.action_metrics.is_empty());
        assert!(result.robust_feasible_action_ids.is_empty());
    }

    #[test]
    fn uncertain_feasibility_produces_no_numeric_regret_frontier() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "indeterminate-feasibility");
        assert_eq!(result.status, "FEASIBILITY_INDETERMINATE");
        assert!(result.action_metrics.is_empty());
    }

    #[test]
    fn open_action_universe_cannot_claim_global_optimality() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "open-action-universe");
        assert_eq!(result.status, "ACTION_UNIVERSE_OPEN");
        assert!(result.action_metrics.is_empty());
    }

    #[test]
    fn scenario_without_any_feasible_action_is_not_hidden() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "no-feasible-scenario");
        assert_eq!(result.status, "NO_FEASIBLE_ACTION_IN_SCENARIO");
        assert!(result.action_metrics.is_empty());
    }

    #[test]
    fn planned_action_omission_is_rejected_even_if_other_action_outcomes_are_complete() {
        let mut corpus = fixture();
        corpus.cases[0].actions.pop();
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn planned_scenario_omission_is_rejected_when_world_remains_covered() {
        let mut corpus = fixture();
        let manifest = &mut corpus.cases[0].scenario_manifest;
        manifest.scenarios.pop();
        manifest.scenarios_sha256 =
            digest_scenarios(&manifest.scenarios).expect("remaining scenarios can be digested");
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn action_and_scenario_input_order_does_not_change_results() {
        let original = fixture();
        let mut reversed = fixture();
        reversed.cases.reverse();
        for case in &mut reversed.cases {
            case.actions.reverse();
            case.compatible_world_set.world_ids.reverse();
            case.action_universe.expected_action_ids.reverse();
            case.scenario_manifest.scenarios.reverse();
            case.scenario_manifest.expected_scenario_ids.reverse();
        }
        assert_eq!(
            serde_json::to_value(evaluate(&original).expect("original evaluates")).unwrap(),
            serde_json::to_value(evaluate(&reversed).expect("reversed evaluates")).unwrap()
        );
    }

    #[test]
    fn no_physical_qualification_or_authority_is_granted() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!evaluation.physical_qualification_granted);
        assert!(!evaluation.authority_granted);
    }

    #[test]
    fn prohibited_action_never_enters_feasible_sets() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "infeasible-zero-loss-comparator");
        assert!(
            result
                .robust_feasible_action_ids
                .iter()
                .all(|id| id != "prohibited")
        );
        assert!(
            result
                .scenario_feasible_sets
                .iter()
                .all(|set| !set.feasible_action_ids.contains(&"prohibited".to_owned()))
        );
    }
}
