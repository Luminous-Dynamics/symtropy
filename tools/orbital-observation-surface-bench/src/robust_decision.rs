//! Exact finite-world loss and regret frontier over an upstream compatible-world set.
//!
//! This module does not construct or authenticate the compatible-world set. It evaluates
//! a supplied, digest-bound finite set and a domain-owned loss matrix. It never selects
//! an action and never grants physical qualification or execution authority.

use crate::decision_identifiability::Rational;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_SCHEMA: &str = "luminous.orbital-decision-regret-frontier.corpus.v0.1";
const CLAIM_CEILING: &str = "finite-compatible-world-loss-and-regret-algebra-only";
const MAX_CASES: usize = 256;
const MAX_WORLDS: usize = 64;
const MAX_ACTIONS: usize = 64;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompatibleWorldSet {
    /// Upstream receipt identity, treated as an opaque reference rather than authenticated.
    pub source_receipt_id: String,
    /// SHA-256 of the sorted JSON array of world IDs, recomputed by this evaluator.
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
    /// Complete normalized loss for every world in the declared compatible-world set.
    pub world_losses: BTreeMap<String, Rational>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionCase {
    pub case_id: String,
    pub compatible_world_set: CompatibleWorldSet,
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
pub struct WorldLoss {
    pub world_id: String,
    pub loss: Rational,
    pub regret: Rational,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionMetric {
    pub action_id: String,
    pub admissibility_receipt_id: String,
    pub worst_case_loss: Rational,
    pub worst_case_regret: Rational,
    pub worst_loss_world_ids: Vec<String>,
    pub worst_regret_world_ids: Vec<String>,
    pub uniformly_optimal: bool,
    pub cost: ActionCost,
    pub per_world: Vec<WorldLoss>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CaseResult {
    pub case_id: String,
    pub status: String,
    pub compatible_world_set_receipt_id: String,
    pub compatible_world_ids_sha256: String,
    pub compatible_world_count: usize,
    pub uniformly_optimal_action_ids: Vec<String>,
    pub minimax_loss_action_ids: Vec<String>,
    pub minimax_regret_action_ids: Vec<String>,
    pub joint_pareto_frontier_action_ids: Vec<String>,
    pub excluded_actions: Vec<ExcludedAction>,
    pub action_metrics: Vec<ActionMetric>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub case_count: usize,
    pub no_admissible_action_cases: usize,
    pub uniform_optimum_cases: usize,
    pub no_uniform_optimum_cases: usize,
    pub admissible_action_count: usize,
    pub excluded_action_count: usize,
    pub frontier_action_count: usize,
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

fn world_ids_digest(world_ids: &[String]) -> Result<String, String> {
    let mut sorted = world_ids.to_vec();
    sorted.sort();
    let bytes = serde_json::to_vec(&sorted)
        .map_err(|error| format!("failed to canonicalize compatible world IDs: {error}"))?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn validate_corpus(corpus: &DecisionCorpus) -> Result<(), String> {
    if corpus.schema != CORPUS_SCHEMA
        || corpus.evidence_class != "synthetic"
        || corpus.claim_ceiling != CLAIM_CEILING
        || corpus.cases.is_empty()
        || corpus.cases.len() > MAX_CASES
    {
        return Err("unsupported decision-regret corpus or claim ceiling".to_owned());
    }

    let mut case_ids = BTreeSet::new();
    for case in &corpus.cases {
        if case.case_id.trim().is_empty()
            || !case_ids.insert(case.case_id.as_str())
            || case.compatible_world_set.source_receipt_id.trim().is_empty()
            || case.compatible_world_set.world_ids.is_empty()
            || case.compatible_world_set.world_ids.len() > MAX_WORLDS
            || case.actions.is_empty()
            || case.actions.len() > MAX_ACTIONS
        {
            return Err("case identity, compatible-world reference or fixture size is invalid".to_owned());
        }

        let mut world_ids = BTreeSet::new();
        for world_id in &case.compatible_world_set.world_ids {
            if world_id.trim().is_empty() || !world_ids.insert(world_id.as_str()) {
                return Err("compatible-world IDs must be nonempty and unique".to_owned());
            }
        }
        let digest = world_ids_digest(&case.compatible_world_set.world_ids)?;
        if case.compatible_world_set.world_ids_sha256 != digest {
            return Err("compatible-world ID digest does not match the canonical world set".to_owned());
        }

        let mut action_ids = BTreeSet::new();
        for action in &case.actions {
            if action.action_id.trim().is_empty()
                || !action_ids.insert(action.action_id.as_str())
                || !matches!(
                    action.admissibility_status.as_str(),
                    "admissible" | "inadmissible" | "indeterminate"
                )
                || (action.admissibility_status == "admissible"
                    && action
                        .admissibility_receipt_id
                        .as_deref()
                        .is_none_or(str::is_empty))
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

            let supplied_world_ids = action
                .world_losses
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>();
            if supplied_world_ids != world_ids {
                return Err("every action must supply exactly one loss for every compatible world".to_owned());
            }
            for loss in action.world_losses.values() {
                loss.validate()
                    .map_err(|error| format!("action loss is invalid: {error}"))?;
            }
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

fn dominates(left: &ActionMetric, right: &ActionMetric) -> bool {
    let dimensions = [
        left.worst_case_loss.compare_exact(&right.worst_case_loss),
        left.worst_case_regret.compare_exact(&right.worst_case_regret),
        left.cost.reference_trials.cmp(&right.cost.reference_trials),
        left.cost.exposure_micro_units.cmp(&right.cost.exposure_micro_units),
        left.cost.elapsed_millis.cmp(&right.cost.elapsed_millis),
    ];
    dimensions.iter().all(|order| *order != Ordering::Greater)
        && dimensions.iter().any(|order| *order == Ordering::Less)
}

fn evaluate_case(case: &DecisionCase) -> Result<CaseResult, String> {
    let mut world_ids = case.compatible_world_set.world_ids.clone();
    world_ids.sort();

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

    if admitted.is_empty() {
        return Ok(CaseResult {
            case_id: case.case_id.clone(),
            status: "NO_ADMISSIBLE_ACTION".to_owned(),
            compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
            compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
            compatible_world_count: world_ids.len(),
            uniformly_optimal_action_ids: Vec::new(),
            minimax_loss_action_ids: Vec::new(),
            minimax_regret_action_ids: Vec::new(),
            joint_pareto_frontier_action_ids: Vec::new(),
            excluded_actions,
            action_metrics: Vec::new(),
        });
    }

    // The per-world comparator is the best loss among currently admissible actions only.
    let mut best_loss_by_world = BTreeMap::new();
    for world_id in &world_ids {
        let best = admitted
            .iter()
            .filter_map(|action| action.world_losses.get(world_id))
            .min_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| format!("no admissible loss value for world {world_id}"))?;
        best_loss_by_world.insert(world_id.clone(), best.clone());
    }

    let mut action_metrics = Vec::with_capacity(admitted.len());
    for action in admitted {
        let receipt_id = action
            .admissibility_receipt_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("admissible action {} has no receipt identity", action.action_id))?
            .to_owned();

        let mut per_world = Vec::with_capacity(world_ids.len());
        for world_id in &world_ids {
            let loss = action
                .world_losses
                .get(world_id)
                .ok_or_else(|| format!("action {} is missing world {world_id}", action.action_id))?
                .clone();
            let best = best_loss_by_world
                .get(world_id)
                .ok_or_else(|| format!("no per-world optimum for {world_id}"))?;
            let regret = loss
                .checked_sub(best)
                .map_err(|error| format!("regret underflow for {} / {world_id}: {error}", action.action_id))?;
            per_world.push(WorldLoss {
                world_id: world_id.clone(),
                loss,
                regret,
            });
        }

        let worst_case_loss = per_world
            .iter()
            .map(|item| &item.loss)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "compatible-world set unexpectedly empty".to_owned())?
            .clone();
        let worst_case_regret = per_world
            .iter()
            .map(|item| &item.regret)
            .max_by(|left, right| left.compare_exact(right))
            .ok_or_else(|| "compatible-world set unexpectedly empty".to_owned())?
            .clone();
        let worst_loss_world_ids = per_world
            .iter()
            .filter(|item| item.loss.exactly_equals(&worst_case_loss))
            .map(|item| item.world_id.clone())
            .collect();
        let worst_regret_world_ids = per_world
            .iter()
            .filter(|item| item.regret.exactly_equals(&worst_case_regret))
            .map(|item| item.world_id.clone())
            .collect();
        let uniformly_optimal = per_world.iter().all(|item| item.regret.numerator == 0);

        action_metrics.push(ActionMetric {
            action_id: action.action_id.clone(),
            admissibility_receipt_id: receipt_id,
            worst_case_loss,
            worst_case_regret,
            worst_loss_world_ids,
            worst_regret_world_ids,
            uniformly_optimal,
            cost: action.cost.clone(),
            per_world,
        });
    }

    let uniformly_optimal_action_ids = action_metrics
        .iter()
        .filter(|metric| metric.uniformly_optimal)
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();
    let minimax_loss_action_ids = argmin_action_ids(&action_metrics, |metric| &metric.worst_case_loss);
    let minimax_regret_action_ids =
        argmin_action_ids(&action_metrics, |metric| &metric.worst_case_regret);

    let joint_pareto_frontier_action_ids = action_metrics
        .iter()
        .filter(|candidate| {
            !action_metrics
                .iter()
                .any(|other| other.action_id != candidate.action_id && dominates(other, candidate))
        })
        .map(|metric| metric.action_id.clone())
        .collect::<Vec<_>>();

    let status = if uniformly_optimal_action_ids.is_empty() {
        "NO_UNIFORM_OPTIMUM_ROBUST_FRONTIER"
    } else {
        "UNIFORM_OPTIMUM_EXISTS"
    };

    Ok(CaseResult {
        case_id: case.case_id.clone(),
        status: status.to_owned(),
        compatible_world_set_receipt_id: case.compatible_world_set.source_receipt_id.clone(),
        compatible_world_ids_sha256: case.compatible_world_set.world_ids_sha256.clone(),
        compatible_world_count: world_ids.len(),
        uniformly_optimal_action_ids,
        minimax_loss_action_ids,
        minimax_regret_action_ids,
        joint_pareto_frontier_action_ids,
        excluded_actions,
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

    let mut summary = Summary {
        case_count: cases.len(),
        no_admissible_action_cases: 0,
        uniform_optimum_cases: 0,
        no_uniform_optimum_cases: 0,
        admissible_action_count: 0,
        excluded_action_count: 0,
        frontier_action_count: 0,
    };
    for case in &cases {
        if case.status == "NO_ADMISSIBLE_ACTION" {
            summary.no_admissible_action_cases += 1;
        } else if case.status == "UNIFORM_OPTIMUM_EXISTS" {
            summary.uniform_optimum_cases += 1;
        } else {
            summary.no_uniform_optimum_cases += 1;
        }
        summary.admissible_action_count += case.action_metrics.len();
        summary.excluded_action_count += case.excluded_actions.len();
        summary.frontier_action_count += case.joint_pareto_frontier_action_ids.len();
    }

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
        serde_json::from_str(include_str!("../fixtures/v0.1/decision-regret-cases.json"))
            .expect("decision-regret fixture must parse")
    }

    fn case<'a>(evaluation: &'a Evaluation, id: &str) -> &'a CaseResult {
        evaluation.cases.iter().find(|item| item.case_id == id).expect("case exists")
    }

    #[test]
    fn minimax_loss_and_minimax_regret_remain_distinct() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "loss-regret-disagreement");
        assert_eq!(result.status, "NO_UNIFORM_OPTIMUM_ROBUST_FRONTIER");
        assert!(result.uniformly_optimal_action_ids.is_empty());
        assert_eq!(result.minimax_loss_action_ids, vec!["balanced"]);
        assert_eq!(result.minimax_regret_action_ids, vec!["aggressive"]);
        assert_eq!(
            result.joint_pareto_frontier_action_ids,
            vec!["aggressive", "balanced"]
        );
    }

    #[test]
    fn uniform_optimum_can_survive_parameter_world_ambiguity() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "uniform-optimum-across-worlds");
        assert_eq!(result.status, "UNIFORM_OPTIMUM_EXISTS");
        assert_eq!(result.uniformly_optimal_action_ids, vec!["conservative"]);
        assert_eq!(result.minimax_loss_action_ids, vec!["conservative"]);
    }

    #[test]
    fn inadmissible_low_loss_action_is_never_in_the_frontier() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "inadmissible-perfect-action");
        assert_eq!(result.excluded_actions.len(), 1);
        assert_eq!(result.excluded_actions[0].action_id, "unsafe-perfect");
        assert_eq!(result.minimax_loss_action_ids, vec!["admissible-action"]);
        assert!(!result.joint_pareto_frontier_action_ids.contains(&"unsafe-perfect".to_owned()));
    }

    #[test]
    fn no_admissible_action_does_not_fall_back_to_an_unsafe_choice() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        let result = case(&evaluation, "all-actions-excluded");
        assert_eq!(result.status, "NO_ADMISSIBLE_ACTION");
        assert!(result.action_metrics.is_empty());
        assert!(result.minimax_loss_action_ids.is_empty());
        assert!(result.minimax_regret_action_ids.is_empty());
    }

    #[test]
    fn compatible_world_digest_is_recomputed_not_trusted() {
        let mut corpus = fixture();
        corpus.cases[0].compatible_world_set.world_ids_sha256 = "0".repeat(64);
        assert!(evaluate(&corpus).is_err());
    }

    #[test]
    fn output_never_grants_physical_qualification_or_authority() {
        let evaluation = evaluate(&fixture()).expect("fixture evaluates");
        assert!(!evaluation.physical_qualification_granted);
        assert!(!evaluation.authority_granted);
    }
}
