//! Deterministic capability-discriminating benchmark task selection.
//!
//! The selector uses measured method outcomes, exact oracle regret and explicit
//! risk compliance. It favours tasks that separate methods while ensuring each
//! selected task is solvable by at least one evaluated method and respecting
//! family coverage caps.

use crate::{GeneratedTask, TaskFamily, verify_generated_task};
use sim_rel_joint_stochastic_oracle::Rational;
use std::collections::{BTreeMap, BTreeSet};

const MAX_METHODS: usize = 128;
const MAX_CANDIDATES: usize = 16_384;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MethodTaskResult {
    pub task_id: String,
    pub method_id: String,
    /// Whether the method policy met the task's predeclared hazard,
    /// false-identification and abstention budgets.
    pub risk_compliant: bool,
    /// Exact cost regret to the constrained finite-horizon oracle.
    pub exact_regret: Rational,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SelectionConfig {
    pub max_selected_tasks: usize,
    pub max_per_family: usize,
    /// Pairwise regret differences smaller than this are treated as ties.
    pub minimum_regret_gap: Rational,
    /// Reject tasks that do not separate at least this many method pairs.
    pub minimum_discriminating_pairs: usize,
}

impl Default for SelectionConfig {
    fn default() -> Self {
        Self {
            max_selected_tasks: 64,
            max_per_family: 16,
            minimum_regret_gap: Rational::new(1, 100).expect("valid fixed rational"),
            minimum_discriminating_pairs: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateScore {
    pub task_id: String,
    pub family: TaskFamily,
    pub exact_frontier_points: usize,
    pub method_count: usize,
    pub risk_compliant_methods: usize,
    pub discriminating_method_pairs: usize,
    pub regret_spread: Rational,
    pub best_exact_regret: Rational,
    pub worst_exact_regret: Rational,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionReport {
    pub method_ids: Vec<String>,
    pub selected_task_ids: Vec<String>,
    /// Every valid candidate with score, in deterministic priority order.
    pub ranked_candidates: Vec<CandidateScore>,
    /// Candidate tasks excluded because no method satisfied the task's declared risk budgets.
    pub unsolved_task_ids: Vec<String>,
}

pub fn select_discriminative_tasks(
    tasks: &[GeneratedTask],
    results: &[MethodTaskResult],
    config: &SelectionConfig,
) -> Result<SelectionReport, String> {
    if tasks.is_empty() || tasks.len() > MAX_CANDIDATES {
        return Err(format!("candidate task count must be in 1..={MAX_CANDIDATES}"));
    }
    if config.max_selected_tasks == 0 || config.max_per_family == 0 {
        return Err("selection and family-cap budgets must be positive".into());
    }
    if results.is_empty() {
        return Err("method-result table is empty".into());
    }

    let mut task_by_id = BTreeMap::new();
    for task in tasks {
        verify_generated_task(task)?;
        let id = &task.receipt.task_id;
        if task_by_id.insert(id.clone(), task).is_some() {
            return Err(format!("duplicate candidate task ID {id}"));
        }
    }

    let mut method_ids = BTreeSet::new();
    let mut result_by_task: BTreeMap<String, BTreeMap<String, &MethodTaskResult>> =
        BTreeMap::new();
    for result in results {
        if result.method_id.trim().is_empty() || !task_by_id.contains_key(&result.task_id) {
            return Err("method result has an empty method ID or unknown task ID".into());
        }
        method_ids.insert(result.method_id.clone());
        let per_method = result_by_task.entry(result.task_id.clone()).or_default();
        if per_method.insert(result.method_id.clone(), result).is_some() {
            return Err(format!(
                "duplicate result for method {} on task {}",
                result.method_id, result.task_id
            ));
        }
    }
    if method_ids.len() < 2 || method_ids.len() > MAX_METHODS {
        return Err(format!("method roster must contain 2..={MAX_METHODS} methods"));
    }
    let method_ids: Vec<String> = method_ids.into_iter().collect();
    let expected_methods: BTreeSet<String> = method_ids.iter().cloned().collect();

    let mut ranked_candidates = Vec::new();
    let mut unsolved_task_ids = Vec::new();
    for (task_id, task) in &task_by_id {
        let per_method = result_by_task
            .get(task_id)
            .ok_or_else(|| format!("task {task_id} has no method results"))?;
        let observed_methods: BTreeSet<String> = per_method.keys().cloned().collect();
        if observed_methods != expected_methods {
            return Err(format!(
                "task {task_id} does not have exactly one result from every method"
            ));
        }

        let scores: Vec<&MethodTaskResult> = method_ids
            .iter()
            .map(|method| per_method[method])
            .collect();
        let risk_compliant_methods =
            scores.iter().filter(|score| score.risk_compliant).count();
        if risk_compliant_methods == 0 {
            unsolved_task_ids.push(task_id.clone());
            continue;
        }

        let mut discriminating_method_pairs = 0_usize;
        for left in 0..scores.len() {
            for right in (left + 1)..scores.len() {
                let regret_gap = absolute_difference(
                    scores[left].exact_regret,
                    scores[right].exact_regret,
                )?;
                if scores[left].risk_compliant != scores[right].risk_compliant
                    || regret_gap >= config.minimum_regret_gap
                {
                    discriminating_method_pairs = discriminating_method_pairs
                        .checked_add(1)
                        .ok_or_else(|| "method-pair discrimination counter overflow".to_string())?;
                }
            }
        }

        let best_exact_regret = scores
            .iter()
            .map(|score| score.exact_regret)
            .min()
            .ok_or_else(|| "empty method score vector".to_string())?;
        let worst_exact_regret = scores
            .iter()
            .map(|score| score.exact_regret)
            .max()
            .ok_or_else(|| "empty method score vector".to_string())?;
        let regret_spread = worst_exact_regret.checked_sub(best_exact_regret)?;

        if discriminating_method_pairs >= config.minimum_discriminating_pairs {
            ranked_candidates.push(CandidateScore {
                task_id: task_id.clone(),
                family: task.receipt.family,
                exact_frontier_points: task.receipt.exact_frontier_points,
                method_count: scores.len(),
                risk_compliant_methods,
                discriminating_method_pairs,
                regret_spread,
                best_exact_regret,
                worst_exact_regret,
            });
        }
    }

    ranked_candidates.sort_by(candidate_priority);
    let mut family_counts: BTreeMap<TaskFamily, usize> = BTreeMap::new();
    let mut selected_task_ids = Vec::new();
    for candidate in &ranked_candidates {
        if selected_task_ids.len() >= config.max_selected_tasks {
            break;
        }
        let count = family_counts.entry(candidate.family).or_default();
        if *count >= config.max_per_family {
            continue;
        }
        selected_task_ids.push(candidate.task_id.clone());
        *count += 1;
    }

    Ok(SelectionReport {
        method_ids,
        selected_task_ids,
        ranked_candidates,
        unsolved_task_ids,
    })
}

fn absolute_difference(left: Rational, right: Rational) -> Result<Rational, String> {
    if left >= right {
        left.checked_sub(right)
    } else {
        right.checked_sub(left)
    }
}

fn candidate_priority(left: &CandidateScore, right: &CandidateScore) -> std::cmp::Ordering {
    right
        .discriminating_method_pairs
        .cmp(&left.discriminating_method_pairs)
        .then_with(|| right.regret_spread.cmp(&left.regret_spread))
        .then_with(|| right.risk_compliant_methods.cmp(&left.risk_compliant_methods))
        .then_with(|| right.exact_frontier_points.cmp(&left.exact_frontier_points))
        .then_with(|| left.family.cmp(&right.family))
        .then_with(|| left.task_id.cmp(&right.task_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{generate_task, TaskFamily};

    fn methods_for(task: &GeneratedTask, regrets: [u128; 3], passes: [bool; 3]) -> Vec<MethodTaskResult> {
        ["random", "one-step-eig", "exact-sequential"]
            .iter()
            .enumerate()
            .map(|(index, method)| MethodTaskResult {
                task_id: task.receipt.task_id.clone(),
                method_id: (*method).to_string(),
                risk_compliant: passes[index],
                exact_regret: Rational::from_u64(regrets[index] as u64),
            })
            .collect()
    }

    #[test]
    fn selects_tasks_that_discriminate_methods_and_respects_family_caps() {
        let a = generate_task(TaskFamily::SensorPhysicalConfounding, 1).unwrap();
        let b = generate_task(TaskFamily::PhysicalCarryover, 2).unwrap();
        let c = generate_task(TaskFamily::NoisySensorConfounding, 3).unwrap();
        let mut results = Vec::new();
        results.extend(methods_for(&a, [10, 8, 0], [false, false, true]));
        results.extend(methods_for(&b, [1, 1, 0], [true, true, true]));
        results.extend(methods_for(&c, [10, 0, 0], [false, true, true]));

        let report = select_discriminative_tasks(
            &[a.clone(), b.clone(), c.clone()],
            &results,
            &SelectionConfig {
                max_selected_tasks: 3,
                max_per_family: 1,
                minimum_regret_gap: Rational::from_u64(2),
                minimum_discriminating_pairs: 1,
            },
        ).unwrap();

        assert_eq!(report.selected_task_ids.len(), 2);
        assert!(!report.selected_task_ids.contains(&b.receipt.task_id));
        assert!(report.selected_task_ids.contains(&a.receipt.task_id));
        assert!(report.selected_task_ids.contains(&c.receipt.task_id));
        assert_eq!(report.unsolved_task_ids, Vec::<String>::new());
    }

    #[test]
    fn rejects_incomplete_or_duplicate_method_rosters() {
        let task = generate_task(TaskFamily::SensorPhysicalConfounding, 7).unwrap();
        let mut results = methods_for(&task, [3, 2, 0], [true, true, true]);
        results.pop();
        assert!(select_discriminative_tasks(
            &[task.clone()],
            &results,
            &SelectionConfig::default(),
        ).is_err());

        let mut duplicated = methods_for(&task, [3, 2, 0], [true, true, true]);
        duplicated.push(duplicated[0].clone());
        assert!(select_discriminative_tasks(
            &[task],
            &duplicated,
            &SelectionConfig::default(),
        ).is_err());
    }

    #[test]
    fn rejects_tasks_that_no_method_can_solve_under_declared_risk_budgets() {
        let task = generate_task(TaskFamily::SensorPhysicalConfounding, 11).unwrap();
        let results = methods_for(&task, [10, 8, 5], [false, false, false]);
        let report = select_discriminative_tasks(
            &[task.clone()],
            &results,
            &SelectionConfig::default(),
        ).unwrap();
        assert!(report.selected_task_ids.is_empty());
        assert_eq!(report.unsolved_task_ids, vec![task.receipt.task_id]);
    }
}
