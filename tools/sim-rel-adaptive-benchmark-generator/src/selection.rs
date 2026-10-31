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


/// Paired method comparison within one task family.
///
/// Regret differences are left minus right, so a negative mean favours the
/// left method. Confidence limits use a deterministic paired bootstrap over
/// task seeds. Risk compliance is reported separately from regret.
#[derive(Clone, Debug, PartialEq)]
pub struct PairedMethodComparison {
    pub family: TaskFamily,
    pub left_method_id: String,
    pub right_method_id: String,
    pub paired_task_count: usize,
    pub mean_left_minus_right_regret: f64,
    pub confidence_95_lower: f64,
    pub confidence_95_upper: f64,
    pub left_risk_compliance_rate: f64,
    pub right_risk_compliance_rate: f64,
    pub enough_paired_tasks: bool,
    pub directional_interval_excludes_zero: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PairedComparisonConfig {
    pub minimum_paired_tasks: usize,
    pub bootstrap_replicates: usize,
    pub root_seed: u64,
}

impl Default for PairedComparisonConfig {
    fn default() -> Self {
        Self {
            minimum_paired_tasks: 30,
            bootstrap_replicates: 2_000,
            root_seed: 0xC0FF_EE12_3456_7890,
        }
    }
}

/// Compare every method pair using the same exact task seeds within each family.
///
/// Results must contain exactly one outcome per method/task pair and the same
/// complete method roster on every task. Bootstrap sampling is deterministic
/// from the supplied root seed and family ID.
pub fn compare_methods_paired(
    tasks: &[GeneratedTask],
    results: &[MethodTaskResult],
    config: &PairedComparisonConfig,
) -> Result<Vec<PairedMethodComparison>, String> {
    if tasks.is_empty() || tasks.len() > MAX_CANDIDATES {
        return Err(format!("paired task count must be in 1..={MAX_CANDIDATES}"));
    }
    if config.minimum_paired_tasks == 0
        || config.bootstrap_replicates < 100
        || config.bootstrap_replicates > 100_000
    {
        return Err("paired comparison requires positive sample count and 100..=100000 bootstrap replicates".into());
    }

    let mut task_ids = BTreeSet::new();
    let mut methods = BTreeSet::new();
    for task in tasks {
        verify_generated_task(task)?;
        if !task_ids.insert(task.receipt.task_id.clone()) {
            return Err("paired comparison contains duplicate task IDs".into());
        }
    }
    for result in results {
        if !task_ids.contains(&result.task_id) || result.method_id.trim().is_empty() {
            return Err("paired result references an unknown task or empty method ID".into());
        }
        methods.insert(result.method_id.clone());
    }
    if methods.len() < 2 || methods.len() > MAX_METHODS {
        return Err(format!("paired method roster must contain 2..={MAX_METHODS} methods"));
    }

    let method_ids: Vec<String> = methods.into_iter().collect();
    let mut results_by_task: BTreeMap<String, BTreeMap<String, &MethodTaskResult>> =
        BTreeMap::new();
    for result in results {
        let per_method = results_by_task.entry(result.task_id.clone()).or_default();
        if per_method.insert(result.method_id.clone(), result).is_some() {
            return Err("paired comparison has duplicate method/task observations".into());
        }
    }

    let expected_roster: BTreeSet<String> = method_ids.iter().cloned().collect();
    let mut tasks_by_family: BTreeMap<TaskFamily, Vec<&GeneratedTask>> = BTreeMap::new();
    for task in tasks {
        let per_method = results_by_task
            .get(&task.receipt.task_id)
            .ok_or_else(|| format!("task {} has no method results", task.receipt.task_id))?;
        let roster: BTreeSet<String> = per_method.keys().cloned().collect();
        if roster != expected_roster {
            return Err(format!(
                "task {} does not have the complete paired method roster",
                task.receipt.task_id
            ));
        }
        tasks_by_family.entry(task.receipt.family).or_default().push(task);
    }

    let mut comparisons = Vec::new();
    for (family, mut family_tasks) in tasks_by_family {
        family_tasks.sort_by(|left, right| {
            left.receipt.seed.cmp(&right.receipt.seed)
                .then_with(|| left.receipt.task_id.cmp(&right.receipt.task_id))
        });

        for left_index in 0..method_ids.len() {
            for right_index in (left_index + 1)..method_ids.len() {
                let left_method = &method_ids[left_index];
                let right_method = &method_ids[right_index];
                let mut differences = Vec::with_capacity(family_tasks.len());
                let mut left_passes = 0_usize;
                let mut right_passes = 0_usize;

                for task in &family_tasks {
                    let per_method = &results_by_task[&task.receipt.task_id];
                    let left = per_method[left_method];
                    let right = per_method[right_method];
                    differences.push(
                        rational_to_f64(left.exact_regret)
                            - rational_to_f64(right.exact_regret),
                    );
                    left_passes += if left.risk_compliant { 1 } else { 0 };
                    right_passes += if right.risk_compliant { 1 } else { 0 };
                }

                let count = differences.len();
                let mean = differences.iter().sum::<f64>() / count as f64;
                let (lower, upper) = paired_bootstrap_interval(
                    &differences,
                    config.bootstrap_replicates,
                    config.root_seed ^ family.code()
                        ^ ((left_index as u64) << 32)
                        ^ right_index as u64,
                );
                comparisons.push(PairedMethodComparison {
                    family,
                    left_method_id: left_method.clone(),
                    right_method_id: right_method.clone(),
                    paired_task_count: count,
                    mean_left_minus_right_regret: mean,
                    confidence_95_lower: lower,
                    confidence_95_upper: upper,
                    left_risk_compliance_rate: left_passes as f64 / count as f64,
                    right_risk_compliance_rate: right_passes as f64 / count as f64,
                    enough_paired_tasks: count >= config.minimum_paired_tasks,
                    directional_interval_excludes_zero: lower > 0.0 || upper < 0.0,
                });
            }
        }
    }
    Ok(comparisons)
}

fn rational_to_f64(value: Rational) -> f64 {
    value.numerator() as f64 / value.denominator() as f64
}

#[derive(Clone, Copy, Debug)]
struct BootstrapRng {
    state: u64,
}

impl BootstrapRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }
}

fn paired_bootstrap_interval(
    differences: &[f64],
    replicates: usize,
    seed: u64,
) -> (f64, f64) {
    let mut rng = BootstrapRng::new(seed);
    let mut means = Vec::with_capacity(replicates);
    for _ in 0..replicates {
        let mut total = 0.0;
        for _ in 0..differences.len() {
            let index = (rng.next_u64() % differences.len() as u64) as usize;
            total += differences[index];
        }
        means.push(total / differences.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    let lower_index = ((means.len() - 1) as f64 * 0.025).floor() as usize;
    let upper_index = ((means.len() - 1) as f64 * 0.975).ceil() as usize;
    (means[lower_index], means[upper_index])
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

    #[test]
    fn paired_comparison_uses_seed_matched_regret_and_reports_power() {
        let tasks: Vec<GeneratedTask> = (0..32)
            .map(|seed| generate_task(TaskFamily::SensorPhysicalConfounding, seed).unwrap())
            .collect();
        let mut results = Vec::new();
        for task in &tasks {
            results.push(MethodTaskResult {
                task_id: task.receipt.task_id.clone(),
                method_id: "one-step-eig".into(),
                risk_compliant: false,
                exact_regret: Rational::from_u64(10),
            });
            results.push(MethodTaskResult {
                task_id: task.receipt.task_id.clone(),
                method_id: "exact-sequential".into(),
                risk_compliant: true,
                exact_regret: Rational::zero(),
            });
        }
        let config = PairedComparisonConfig {
            minimum_paired_tasks: 30,
            bootstrap_replicates: 500,
            root_seed: 1234,
        };
        let comparison = compare_methods_paired(&tasks, &results, &config).unwrap();
        assert_eq!(comparison.len(), 1);
        assert_eq!(comparison[0].paired_task_count, 32);
        assert!(comparison[0].enough_paired_tasks);
        assert!(comparison[0].directional_interval_excludes_zero);
        assert!(comparison[0].mean_left_minus_right_regret > 0.0);
        assert_eq!(comparison[0].left_risk_compliance_rate, 0.0);
        assert_eq!(comparison[0].right_risk_compliance_rate, 1.0);
    }

    #[test]
    fn paired_comparison_marks_small_samples_as_underpowered() {
        let tasks = vec![
            generate_task(TaskFamily::PhysicalCarryover, 1).unwrap(),
            generate_task(TaskFamily::PhysicalCarryover, 2).unwrap(),
        ];
        let mut results = Vec::new();
        for task in &tasks {
            results.push(MethodTaskResult {
                task_id: task.receipt.task_id.clone(),
                method_id: "a".into(),
                risk_compliant: true,
                exact_regret: Rational::from_u64(2),
            });
            results.push(MethodTaskResult {
                task_id: task.receipt.task_id.clone(),
                method_id: "b".into(),
                risk_compliant: true,
                exact_regret: Rational::zero(),
            });
        }
        let comparison = compare_methods_paired(
            &tasks,
            &results,
            &PairedComparisonConfig {
                minimum_paired_tasks: 30,
                bootstrap_replicates: 100,
                root_seed: 5,
            },
        ).unwrap();
        assert_eq!(comparison.len(), 1);
        assert!(!comparison[0].enough_paired_tasks);
    }

}
