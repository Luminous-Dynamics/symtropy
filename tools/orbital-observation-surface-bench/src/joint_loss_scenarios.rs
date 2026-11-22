//! Exact joint loss-model scenario frontiers over an upstream compatible-world set.
//!
//! Each scenario carries a coherent vector of losses for every declared action. The
//! evaluator preserves this coupling for joint regret and computes an explicitly
//! conservative rectangularized comparator that allows each action's loss model to
//! vary independently within the same physical world. It never chooses or authorizes
//! an action and fails closed when any required loss scenario is unqualified.

use crate::decision_identifiability::Rational;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-joint-loss-scenario.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-joint-loss-scenario-regret-algebra-only";
const MAX_CASES: usize = 256;
const MAX_WORLDS: usize = 64;
const MAX_ACTIONS: usize = 64;
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
pub struct ActionCost {
    pub reference_trials: u64,
    pub exposure_micro_units: u64,
    pub elapsed_millis: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub action_id: String,
    pub admissibility_status: String,
    pub admissibility_receipt_id: Option<String>,
    pub cost: ActionCost,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LossScenario {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
    /// A coherent joint vector: one loss for every action, including excluded actions.
    pub action_losses: BTreeMap<String, Rational>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LossScenarioManifest {
    pub source_manifest_id: String,
    /// SHA-256 over the canonical JSON of all scenario records, sorted by scenario_id.
    pub scenarios_sha256: String,
    pub scenarios: Vec<LossScenario>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCase {
    pub case_id: String,
    pub compatible_world_set: CompatibleWorldSet,
    pub loss_scenario_manifest: LossScenarioManifest,
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
pub struct ExcludedAction {
    pub action_id: String,
    pub admissibility_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioEvidenceIssue {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub evidence_status: String,
    pub evidence_receipt_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScenarioRegret {
    pub scenario_id: String,
    pub world_id: String,
    pub loss_model_id: String,
    pub loss: Rational,
    pub joint_regret: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionMetric {
    pub action_id: String,
    pub admissibility_receipt_id: String,
    pub worst_case_loss: Rational,
    pub worst_case_joint_regret: Rational,
    pub worst_case_rectangular_regret: Rational,
    pub rectangular_regret_excess: Rational,
    pub worst_joint_regret_scenarios: Vec<String>,
    pub worst_rectangular_regret_worlds: Vec<String>,
    pub uniformly_optimal_across_joint_scenarios: bool,
    pub joint_scenario_best_count: usize,
    pub cost: ActionCost,
    pub per_scenario: Vec<ScenarioRegret>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub compatible_world_set_receipt_id: String,
    pub compatible_world_ids_sha256: String,
    pub loss_scenario_manifest_id: String,
    pub loss_scenarios_sha256: String,
    pub compatible_world_count: usize,
    pub loss_scenario_count: usize,
    pub loss_model_ids: Vec<String>,
    pub uniformly_optimal_action_ids: Vec<String>,
    pub joint_minimax_regret_action_ids: Vec<String>,
    pub rectangular_minimax_regret_action_ids: Vec<String>,
    pub joint_scenario_pareto_frontier_action_ids: Vec<String>,
    pub rectangular_pareto_frontier_action_ids: Vec<String>,
    pub excluded_actions: Vec<ExcludedAction>,
    pub scenario_evidence_issues: Vec<ScenarioEvidenceIssue>,
    pub action_metrics: Vec<ActionMetric>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub no_admissible_action_cases: usize,
    pub loss_model_indeterminate_cases: usize,
    pub out_of_profile_cases: usize,
    pub uniform_optimum_cases: usize,
    pub no_uniform_optimum_cases: usize,
    pub admissible_action_count: usize,
    pub excluded_action_count: usize,
    pub scenario_evidence_issue_count: usize,
    pub joint_frontier_action_count: usize,
    pub rectangular_frontier_action_count: usize,
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

fn digest_world_ids(world_ids: &[String]) -> Result<String, String> {
    let mut sorted = world_ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to serialize canonical world IDs: {error}"))?;
    Ok(hex_sha256(&bytes))
}

fn canonical_scenario_manifest(scenarios: &[LossScenario]) -> Result<Value, String> {
    let mut sorted = scenarios.to_vec();
    sorted.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    let value = serde_json::to_value(sorted)
        .map_err(|error| format!("failed to encode loss scenario manifest: {error}"))?;
    Ok(value)
}

fn digest_scenarios(scenarios: &[LossScenario]) -> Result<String, String> {
    let canonical = canonical_scenario_manifest(scenarios)?;
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| format!("failed to serialize canonical loss scenarios: {error}"))?;
    Ok(hex_sha256(&bytes))
}

fn validate_ascii_id(value: &str) -> bool {
    !value.trim().is_empty() && value.is_ascii()
}

fn validate_corpus(corpus: &DecisionCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported joint-loss corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if !validate_ascii_id(&case.case_id)
            || !case_ids.insert(case.case_id.as_str())
            || !validate_ascii_id(&case.compatible_world_set.source_receipt_id)
            || !validate_ascii_id(&case.loss_scenario_manifest.source_manifest_id)
            || case.compatible_world_set.world_ids.is_empty()
            || case.compatible_world_set.world_ids.len() > MAX_WORLDS
            || case.loss_scenario_manifest.scenarios.is_empty()
            || case.loss_scenario_manifest.scenarios.len() > MAX_SCENARIOS
            || case.actions.is_empty()
            || case.actions.len() > MAX_ACTIONS
        {
            return Err("case identity, manifests or finite fixture bounds are invalid".to_owned());
        }

        let mut world_ids = BTreeSet::new();
        for world_id in &world_ids {
            if !validate_ascii_id(world_id) || !world_ids.insert(world_id.as_str()) {
                return Err("compatible-world IDs must be unique, nonempty ASCII strings".to_owned());
            }
        }
        let world_digest = digest_world_ids(&case.compatible_world_set.world_ids)?;
        if case.compatible_world_set.world_ids_sha256 != world_digest {
            return Err("compatible-world ID digest mismatch".to_owned());
        }

        let mut action_ids = BTreeSet::new();
        for action in &case.actions {
            if !validate_ascii_id(&action.action_id)
                || !action_ids.insert(action.action_id.as_str())
                || !matches!(
                    action.admissibility_status.as_str(),
                    "admissible" | "inadmissible" | "indeterminate"
                )
                || (action.admissibility_status == "admissible"
                    && action
                        .admissibility_receipt_id
                        .as_deref()
                        .is_none_or(|value| !validate_ascii_id(value)))
                || [
                    action.cost.reference_trials,
                    action.cost.exposure_micro_units,
                    action.cost.elapsed_millis,
                ]
                .iter()
                .any(|value| *value > MAX_SAFE_INTEGER)
            {
                return Err("action identity, admissibility receipt, status or cost is invalid".to_owned());
            }
        }

        let mut scenario_ids = BTreeSet::new();
        let mut covered_worlds = BTreeSet::new();
        for scenario in &case.loss_scenario_manifest.scenarios {
            if !validate_ascii_id(&scenario.scenario_id)
                || !scenario_ids.insert(scenario.scenario_id.as_str())
                || !validate_ascii_id(&scenario.world_id)
                || !world_ids.contains(scenario.world_id.as_str())
                || !validate_ascii_id(&scenario.loss_model_id)
                || !matches!(
                    scenario.evidence_status.as_str(),
                    "admissible" | "inadmissible" | "indeterminate" | "out_of_profile"
                )
            {
                return Err("loss scenario identity, world reference or evidence status is invalid".to_owned());
            }
            if scenario.evidence_status == "admissible"
                && scenario
                    .evidence_receipt_id
                    .as_deref()
                    .is_none_or(|value| !validate_ascii_id(value))
            {
                return Err("admissible loss scenario requires an evidence receipt identity".to_owned());
            }
            covered_worlds.insert(scenario.world_id.as_str());

            let supplied_actions = scenario
                .action_losses
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if supplied_actions != action_ids {
                return Err("every loss scenario must include exactly one loss for every declared action".to_owned());
            }
            for loss in scenario.action_losses.values() {
                loss.validate()
                    .map_err(|error| format!("loss scenario contains invalid value: {error}"))?;
            }
        }
        if covered_worlds != world_ids {
            return Err("loss scenario manifest must cover every compatible world".to_owned());
        }

        let scenarios_digest = digest_scenarios(&case.loss_scenario_manifest.scenarios)?;
        if case.loss_scenario_manifest.scenarios_sha256 != scenarios_digest {
            return Err("loss scenario manifest digest mismatch".to_owned());
        }
    }
    Ok(())
}

fn argmin_action_ids<F>(metrics: &[ActionMetric], value: F) -> Vec<String>
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

fn dominates(left: &ActionMetric, right: &ActionMetric, rectangular: bool) -> bool {
    let regret_order = if rectangular {
        left.worst_case_rectangular_regret
            .compare_exact(&right.worst_case_rectangular_regret)
    } else {
        left.worst_case_joint_regret
            .compare_exact(&right.worst_case_joint_regret)
    };
    let dimensions = [
        left.worst_case_loss.compare_exact(&right.worst_case_loss),
        regret_order,
        left.cost.reference_trials.cmp(&right.cost.reference_trials),
        left.cost
            .exposure_micro_units
            .cmp(&right.cost.exposure_micro_units),
        left.cost.elapsed_millis.cmp(&right.cost.elapsed_millis),
    ];
    dimensions.iter().all(|order| *order != Ordering::Greater)
        && dimensions.contains(&Ordering::Less)
}

fn empty_case_result(
    case: &DecisionCase,
    status: &str,
    scenario_evidence_issues: Vec<ScenarioEvidenceIssue>,
    excluded_actions: Vec<ExcludedAction>,
) -> CaseResult {
    CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
        compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
        loss_scenario_manifest_id: case.loss_scenario_manifest.source_manifest_id.clone(),
        loss_scenarios_sha256: case.loss_scenario_manifest.scenarios_sha256.clone(),
        compatible_world_count: case.compatible_world_set.world_ids.len(),
        loss_scenario_count: case.loss_scenario_manifest.scenarios.len(),
        loss_model_ids: Vec::new(),
        uniformly_optimal_action_ids: Vec::new(),
        joint_minimax_regret_action_ids: Vec::new(),
        rectangular_minimax_regret_action_ids: Vec::new(),
        joint_scenario_pareto_frontier_action_ids: Vec::new(),
        rectangular_pareto_frontier_action_ids: Vec::new(),
        excluded_actions,
        scenario_evidence_issues,
        action_metrics: Vec::new(),
    }
}

fn evaluate_case(case: &DecisionCase) -> Result<CaseResult, String> {
    let mut admitted = case
        .actions
        .iter()
        .filter(|action| action.admissibility_status == "admissible")
        .collect::<Vec<_>>();
    admitted.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let mut excluded_actions = case
        .actions
        .iter()
        .filter(|action| action.admissibility_status != "admissible")
        .map(|action| ExcludedAction {
            action_id: action.action_id.clone(),
            admissibility_status: action.admissibility_status.clone(),
        })
        .collect::<Vec<_>>();
    excluded_actions.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    let mut evidence_issues = case
        .loss_scenario_manifest
        .scenarios
        .iter()
        .filter(|scenario| scenario.evidence_status != "admissible")
        .map(|scenario| ScenarioEvidenceIssue {
            scenario_id: scenario.scenario_id.clone(),
            world_id: scenario.world_id.clone(),
            loss_model_id: scenario.loss_model_id.clone(),
            evidence_status: scenario.evidence_status.clone(),
            evidence_receipt_id: scenario.evidence_receipt_id.clone(),
        })
        .collect::<Vec<_>>();
    evidence_issues.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    if evidence_issues
        .iter()
        .any(|issue| issue.evidence_status == "out_of_profile")
    {
        return Ok(empty_case_result(
            case,
            "OUT_OF_PROFILE",
            evidence_issues,
            excluded_actions,
        ));
    }
    if !evidence_issues.is_empty() {
        return Ok(empty_case_result(
            case,
            "LOSS_MODEL_INDETERMINATE",
            evidence_issues,
            excluded_actions,
        ));
    }
    if admitted.is_empty() {
        return Ok(empty_case_result(
            case,
            "NO_ADMISSIBLE_ACTION",
            evidence_issues,
            excluded_actions,
        ));
    }

    let mut world_ids = case.compatible_world_set.world_ids.clone();
    world_ids.sort();
    let mut scenarios = case.loss_scenario_manifest.scenarios.iter().collect::<Vec<_>>();
    scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    let mut best_loss_by_scenario = BTreeMap::new();
    let mut worst_loss_by_world_action = BTreeMap::new();
    let mut best_loss_lower_by_world = BTreeMap::new();

    for scenario in &scenarios {
        let best = admitted
            .iter()
            .filter_map(|action| scenario.action_losses.get(&action.action_id))
            .min_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| format!("no admissible action loss in scenario {}", scenario.scenario_id))?
            .canonicalized();
        best_loss_by_scenario.insert(scenario.scenario_id.clone(), best);
    }

    // Per-world loss envelopes are intentionally rectangularized only in this block.
    for world_id in &case.compatible_world_set.world_ids {
        let world_scenarios = scenarios
            .iter()
            .filter(|scenario| scenario.world_id == *world_id)
            .collect::<Vec<_>>();
        for action in &admitted {
            let mut minimum: Option<Rational> = None;
            let mut maximum: Option<Rational> = None;
            for scenario in &world_scenarios {
                let value = scenario
                    .action_losses
                    .get(&action.action_id)
                    .ok_or_else(|| format!("missing loss for action {} in {}", action.action_id, scenario.scenario_id))?
                    .canonicalized();
                if minimum
                    .as_ref()
                    .is_none_or(|current| value.compare_exact(current) == Ordering::Less)
                {
                    minimum = Some(value.clone());
                }
                if maximum
                    .as_ref()
                    .is_none_or(|current| value.compare_exact(current) == Ordering::Greater)
                {
                    maximum = Some(value);
                }
            }
            let minimum = minimum.ok_or_else(|| format!("world {world_id} has no loss scenarios"))?;
            let maximum = maximum.ok_or_else(|| format!("world {world_id} has no loss scenarios"))?;
            worst_loss_by_world_action.insert((action.action_id.clone(), world_id.clone()), maximum);
            best_loss_lower_by_world
                .entry(world_id.clone())
                .and_modify(|current: &mut Rational| {
                    if minimum.compare_exact(current) == Ordering::Less {
                        *current = minimum.clone();
                    }
                })
                .or_insert(minimum);
        }
    }

    let mut action_metrics = Vec::with_capacity(admitted.len());
    for action in &admitted {
        let receipt_id = action
            .admissibility_receipt_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("admissible action {} has no receipt identity", action.action_id))?
            .to_owned();

        let mut per_scenario = Vec::with_capacity(scenarios.len());
        let mut best_count = 0usize;
        for scenario in &scenarios {
            let loss = scenario
                .action_losses
                .get(&action.action_id)
                .ok_or_else(|| format!("missing action loss in scenario {}", scenario.scenario_id))?
                .canonicalized();
            let best = best_loss_by_scenario
                .get(&scenario.scenario_id)
                .ok_or_else(|| format!("missing scenario optimum {}", scenario.scenario_id))?;
            let joint_regret = loss
                .checked_sub(best)
                .map_err(|error| format!("joint regret underflow in {}", scenario.scenario_id))?;
            if joint_regret.numerator == 0 {
                best_count += 1;
            }
            per_scenario.push(ScenarioRegret {
                scenario_id: scenario.scenario_id.clone(),
                world_id: scenario.world_id.clone(),
                loss,
                joint_regret,
            });
        }

        let worst_case_loss = per_scenario
            .iter()
            .map(|item| &item.loss)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "loss scenario set unexpectedly empty".to_owned())?
            .canonicalized();
        let worst_case_joint_regret = per_scenario
            .iter()
            .map(|item| &item.joint_regret)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "loss scenario set unexpectedly empty".to_owned())?
            .canonicalized();
        let worst_joint_regret_scenarios = per_scenario
            .iter()
            .filter(|item| item.joint_regret.exactly_equals(&worst_case_joint_regret))
            .map(|item| item.scenario_id.clone())
            .collect::<Vec<_>>();

        let mut rectangular_by_world = Vec::new();
        for world_id in &case.compatible_world_set.world_ids {
            let action_upper = worst_loss_by_world_action
                .get(&(action.action_id.clone(), world_id.clone()))
                .ok_or_else(|| format!("missing rectangular upper loss for {} / {world_id}", action.action_id))?;
            let comparator_lower = best_loss_lower_by_world
                .get(world_id)
                .ok_or_else(|| format!("missing rectangular comparator lower loss for {world_id}"))?;
            rectangular_by_world.push((
                world_id.clone(),
                action_upper
                    .checked_sub(comparator_lower)
                    .map_err(|error| format!("rectangular regret underflow for {} / {world_id}", action.action_id))?,
            ));
        }
        let worst_case_rectangular_regret = rectangular_by_world
            .iter()
            .map(|(_, regret)| regret)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "compatible-world set unexpectedly empty".to_owned())?
            .canonicalized();
        let worst_rectangular_regret_worlds = rectangular_by_world
            .iter()
            .filter(|(_, regret)| regret.exactly_equals(&worst_case_rectangular_regret))
            .map(|(world_id, _)| world_id.clone())
            .collect::<Vec<_>>();
        let rectangular_regret_excess = worst_case_rectangular_regret
            .checked_sub(&worst_case_joint_regret)
            .map_err(|_| "rectangular regret must not be below joint regret".to_owned())?;

        action_metrics.push(ActionMetric {
            action_id: action.action_id.clone(),
            admissibility_receipt_id: receipt_id,
            worst_case_loss,
            worst_case_joint_regret,
            worst_case_rectangular_regret,
            rectangular_regret_excess,
            worst_joint_regret_scenarios,
            worst_rectangular_regret_worlds,
            uniformly_optimal_across_joint_scenarios: best_count == scenarios.len(),
            joint_scenario_best_count: best_count,
            cost: action.cost.clone(),
            per_scenario,
        });
    }

    let uniformly_optimal_action_ids = action_metrics
        .iter()
        .filter(|metric| metric.uniformly_optimal_across_joint_scenarios)
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();
    let joint_minimax_regret_action_ids =
        argmin_action_ids(&action_metrics, |metric| &metric.worst_case_joint_regret);
    let rectangular_minimax_regret_action_ids =
        argmin_action_ids(&action_metrics, |metric| &metric.worst_case_rectangular_regret);

    let joint_scenario_pareto_frontier_action_ids = action_metrics
        .iter()
        .filter(|candidate| {
            !action_metrics.iter().any(|other| {
                other.action_id != candidate.action_id && dominates(other, candidate, false)
            })
        })
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();
    let rectangular_pareto_frontier_action_ids = action_metrics
        .iter()
        .filter(|candidate| {
            !action_metrics.iter().any(|other| {
                other.action_id != candidate.action_id && dominates(other, candidate, true)
            })
        })
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();

    let loss_model_ids = scenarios
        .iter()
        .map(|scenario| scenario.loss_model_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let status = if uniformly_optimal_action_ids.is_empty() {
        "NO_UNIFORM_OPTIMUM_JOINT_SCENARIO_FRONTIER"
    } else {
        "UNIFORM_OPTIMUM_ACROSS_JOINT_SCENARIOS"
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
        compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
        loss_scenario_manifest_id: case.loss_scenario_manifest.source_manifest_id.clone(),
        loss_scenarios_sha256: case.loss_scenario_manifest.scenarios_sha256.clone(),
        compatible_world_count: case.compatible_world_set.world_ids.len(),
        loss_scenario_count: scenarios.len(),
        loss_model_ids,
        uniformly_optimal_action_ids,
        joint_minimax_regret_action_ids,
        rectangular_minimax_regret_action_ids,
        joint_scenario_pareto_frontier_action_ids,
        rectangular_pareto_frontier_action_ids,
        excluded_actions,
        scenario_evidence_issues: evidence_issues,
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
        no_admissible_action_cases: cases.iter().filter(|item| item.status == "NO_ADMISSIBLE_ACTION").count(),
        loss_model_indeterminate_cases: cases.iter().filter(|item| item.status == "LOSS_MODEL_INDETERMINATE").count(),
        out_of_profile_cases: cases.iter().filter(|item| item.status == "OUT_OF_PROFILE").count(),
        uniform_optimum_cases: cases.iter().filter(|item| item.status == "UNIFORM_OPTIMUM_ACROSS_JOINT_SCENARIOS").count(),
        no_uniform_optimum_cases: cases.iter().filter(|item| item.status == "NO_UNIFORM_OPTIMUM_JOINT_SCENARIO_FRONTIER").count(),
        admissible_action_count: cases.iter().map(|item| item.action_metrics.len()).sum(),
        excluded_action_count: cases.iter().map(|item| item.excluded_actions.len()).sum(),
        scenario_evidence_issue_count: cases.iter().map(|item| item.scenario_evidence_issues.len()).sum(),
        joint_frontier_action_count: cases.iter().map(|item| item.joint_scenario_pareto_frontier_action_ids.len()).sum(),
        rectangular_frontier_action_count: cases.iter().map(|item| item.rectangular_pareto_frontier_action_ids.len()).sum(),
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
        serde_json::from_str(include_str!("../fixtures/v0.1/joint-loss-scenarios.json"))
            .expect("joint loss scenario fixture must parse")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation.cases.iter().find(|item| item.case_id == id).expect("case exists")
    }

    #[test]
    fn joint_scenarios_preserve_correlation_and_expose_rectangular_inflation() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "correlated-common-loss-shift");
        let metric = result.action_metrics.iter().find(|item| item.action_id == "alpha").expect("alpha metric exists");
        assert_eq!(metric.worst_case_joint_regret.numerator, 0);
        assert_eq!(metric.worst_case_joint_regret.denominator, 1);
        assert_eq!(metric.worst_case_rectangular_regret.numerator, 1);
        assert_eq!(metric.worst_case_rectangular_regret.denominator, 2);
        assert_eq!(metric.rectangular_regret_excess.numerator, 1);
        assert_eq!(metric.rectangular_regret_excess.denominator, 2);
        assert_eq!(result.joint_minimax_regret_action_ids, vec!["alpha"]);
        assert_eq!(result.rectangular_minimax_regret_action_ids, vec!["alpha"]);
    }

    #[test]
    fn action_rank_reversal_is_preserved_in_joint_regret() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "action-rank-reversal");
        let alpha = result.action_metrics.iter().find(|item| item.action_id == "alpha").expect("alpha exists");
        let beta = result.action_metrics.iter().find(|item| item.action_id == "beta").expect("beta exists");
        assert_eq!(alpha.worst_case_joint_regret.numerator, 4);
        assert_eq!(alpha.worst_case_joint_regret.denominator, 5);
        assert_eq!(beta.worst_case_joint_regret.numerator, 1);
        assert_eq!(beta.worst_case_joint_regret.denominator, 5);
        assert_eq!(result.joint_minimax_regret_action_ids, vec!["beta"]);
    }

    #[test]
    fn unqualified_loss_scenario_fails_closed_without_metrics() {
        let evaluation = evaluate(&fixture()).expect("unqualified scenario is an explicit result");
        let result = case(&evaluation, "unqualified-loss-model");
        assert_eq!(result.status, "LOSS_MODEL_INDETERMINATE");
        assert!(result.action_metrics.is_empty());
        assert_eq!(result.scenario_evidence_issues.len(), 1);
    }

    #[test]
    fn out_of_profile_loss_scenario_is_not_dropped() {
        let evaluation = evaluate(&fixture()).expect("out of profile is an explicit result");
        let result = case(&evaluation, "out-of-profile-loss-model");
        assert_eq!(result.status, "OUT_OF_PROFILE");
        assert!(result.action_metrics.is_empty());
        assert_eq!(result.loss_scenario_count, 1);
    }

    #[test]
    fn inadmissible_action_is_excluded_before_each_scenario_oracle() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "inadmissible-low-loss-action");
        assert_eq!(result.excluded_actions.len(), 1);
        assert_eq!(result.excluded_actions[0].action_id, "unsafe");
        assert_eq!(result.joint_minimax_regret_action_ids, vec!["safe"]);
    }

    #[test]
    fn no_admissible_action_does_not_create_a_fallback() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "all-actions-excluded");
        assert_eq!(result.status, "NO_ADMISSIBLE_ACTION");
        assert!(result.action_metrics.is_empty());
    }

    #[test]
    fn scenario_manifest_digest_binds_losses_not_only_scenario_ids() {
        let mut corpus = fixture();
        let scenario = &mut corpus.cases[0].loss_scenario_manifest.scenarios[0];
        scenario.action_losses.get_mut("alpha").expect("loss exists").numerator = 1;
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn scenario_and_action_ordering_does_not_change_results() {
        let corpus = fixture();
        let mut reversed = fixture();
        reversed.cases.reverse();
        for case in &mut reversed.cases {
            case.actions.reverse();
            case.compatible_world_set.world_ids.reverse();
            case.loss_scenario_manifest.scenarios.reverse();
        }
        let a = evaluate(&corpus).expect("original evaluates");
        let b = evaluate(&reversed).expect("reversed evaluates");
        assert_eq!(
            serde_json::to_value(a).expect("serialize original"),
            serde_json::to_value(b).expect("serialize reversed")
        );
    }

    #[test]
    fn rectangular_regret_is_never_below_joint_regret() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        for item in &evaluation.cases {
            for metric in &item.action_metrics {
                assert!(
                    metric.worst_case_rectangular_regret
                        .compare_exact(&metric.worst_case_joint_regret)
                        != Ordering::Less,
                    "rectangular regret fell below joint regret for {}",
                    metric.action_id
                );
                assert_eq!(
                    metric.rectangular_regret_excess,
                    metric
                        .worst_case_rectangular_regret
                        .checked_sub(&metric.worst_case_joint_regret)
                        .expect("rectangular regret excess is nonnegative")
                );
            }
        }
    }

    #[test]
    fn multi_world_rectangular_regret_uses_the_worst_physical_world() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "multi-world-strata");
        assert_eq!(result.compatible_world_count, 2);
        let alpha = result
            .action_metrics
            .iter()
            .find(|item| item.action_id == "alpha")
            .expect("alpha metric exists");
        let beta = result
            .action_metrics
            .iter()
            .find(|item| item.action_id == "beta")
            .expect("beta metric exists");
        assert_eq!(alpha.worst_case_joint_regret.numerator, 1);
        assert_eq!(alpha.worst_case_joint_regret.denominator, 5);
        assert_eq!(beta.worst_case_joint_regret.numerator, 1);
        assert_eq!(beta.worst_case_joint_regret.denominator, 5);
        assert_eq!(alpha.worst_rectangular_regret_worlds, vec!["w1"]);
        assert_eq!(beta.worst_rectangular_regret_worlds, vec!["w0"]);
    }

    #[test]
    fn authority_and_physical_qualification_are_never_granted() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!evaluation.physical_qualification_granted);
        assert!(!evaluation.authority_granted);
    }
}
