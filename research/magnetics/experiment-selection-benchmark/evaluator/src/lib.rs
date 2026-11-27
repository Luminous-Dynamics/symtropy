//! Replay and evaluate matched experiment-selection policy traces.
//!
//! This crate does not implement Bayesian optimization or operate laboratory
//! equipment. It computes decision-quality metrics from frozen experiment
//! traces and rejects incomparable budgets, inputs and holdout splits.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PolicyFamily {
    ExpertPrior,
    FactorialDoe,
    ResponseSurfaceDoe,
    Random,
    LatinHypercube,
    MaximinSpaceFilling,
    ClusterRepresentative,
    GaussianProcessExpectedImprovement,
    GaussianProcessUpperConfidenceBound,
    ProbabilityOfFeasibility,
    ThompsonSampling,
    KnowledgeGradient,
    ExpectedHypervolumeImprovement,
    CostAwareBatch,
    FeasibilityDiversityCombined,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplitUnit {
    MaterialLot,
    Build,
    ProcessRevision,
    RandomRows,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Minimize,
    Maximize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExperimentDisposition {
    Qualified,
    Fail,
    Indeterminate,
    NotEvaluable,
    Missing,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObjectiveDefinition {
    pub metric_id: String,
    pub unit: String,
    pub direction: ObjectiveDirection,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkTask {
    pub task_id: String,
    pub candidate_space_digest: String,
    pub target_function_digest: String,
    pub acceptance_policy_digest: String,
    pub initial_design_digest: String,
    pub holdout_manifest_digest: String,
    pub split_unit: SplitUnit,
    pub experiment_budget: u32,
    pub objectives: Vec<ObjectiveDefinition>,
    pub oracle_best_objective_0: Option<f64>,
    /// Required only for the two-objective hypervolume diagnostic.
    pub hypervolume_reference_point: Option<Vec<f64>>,
    pub false_feasible_probability_threshold: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExperimentOutcome {
    pub experiment_id: String,
    pub candidate_id: String,
    pub material_lot_id: String,
    pub build_id: String,
    pub process_revision: String,
    pub iteration: u32,
    pub disposition: ExperimentDisposition,
    pub cost_minor: u64,
    pub objective_values: Vec<f64>,
    pub design_features: Vec<f64>,
    pub predicted_feasibility_probability: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PredictionInterval {
    pub objective_index: usize,
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HoldoutObservation {
    pub holdout_id: String,
    pub candidate_id: String,
    pub material_lot_id: String,
    pub build_id: String,
    pub process_revision: String,
    pub disposition: ExperimentDisposition,
    pub objective_values: Vec<f64>,
    pub design_features: Vec<f64>,
    pub predicted_feasibility_probability: f64,
    pub predicted_intervals: Vec<PredictionInterval>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PolicyTrace {
    pub policy: PolicyFamily,
    pub matched_seed_id: String,
    pub training_group_ids: Vec<String>,
    pub holdout_group_ids: Vec<String>,
    /// Every initiated experiment has exactly one outcome, including missing,
    /// failed, indeterminate and not-evaluable outcomes.
    pub initiated_experiment_count: u32,
    pub outcomes: Vec<ExperimentOutcome>,
    pub holdout: Vec<HoldoutObservation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkRun {
    pub task: BenchmarkTask,
    pub traces: Vec<PolicyTrace>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Blocker {
    MissingTaskIdentity,
    InvalidTaskDigest,
    InvalidObjective,
    InvalidBudget,
    InvalidProbabilityThreshold,
    UnsupportedRandomRowHoldout,
    NoPolicyTraces,
    DuplicatePolicy,
    UnmatchedSeed,
    UnmatchedBudget,
    UnmatchedTrainingGroups,
    UnmatchedHoldoutGroups,
    HoldoutIdentityMismatch,
    TrainingHoldoutLeakage,
    MissingTrainingGroup,
    MissingHoldoutGroup,
    InitiatedOutcomeCountMismatch,
    DuplicateExperimentId,
    DuplicateHoldoutId,
    MissingExperimentIdentity,
    InvalidObjectiveVector,
    InvalidDesignFeatureVector,
    InvalidProbability,
    InvalidInterval,
    DuplicatePredictionInterval,
    MissingPredictionInterval,
    InvalidHoldoutSplit,
    InvalidOracle,
    InvalidHypervolumeReference,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceMetrics {
    pub policy: PolicyFamily,
    pub experiment_count: u64,
    pub qualified_experiments: u64,
    pub confirmed_failures: u64,
    pub unresolved_experiments: u64,
    pub first_qualified_iteration: Option<u32>,
    pub total_experiment_cost_minor: u64,
    pub cost_to_first_qualified_minor: Option<u64>,
    pub false_feasible_selections: u64,
    pub unresolved_feasibility_selections: u64,
    pub simple_regret_objective_0: Option<f64>,
    pub pareto_hypervolume_2d: Option<f64>,
    pub unique_candidate_fraction: f64,
    pub mean_pairwise_design_distance: f64,
    pub holdout_interval_coverage: Vec<Option<f64>>,
    pub holdout_mean_interval_width: Vec<Option<f64>>,
    pub holdout_feasibility_brier: Option<f64>,
    pub holdout_unresolved_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkEvaluation {
    pub valid: bool,
    pub blockers: Vec<Blocker>,
    pub trace_metrics: Vec<TraceMetrics>,
}

pub fn evaluate_run(run: &BenchmarkRun) -> BenchmarkEvaluation {
    let mut blockers = validate_task(&run.task);
    if run.traces.is_empty() {
        blockers.push(Blocker::NoPolicyTraces);
    }

    let mut seen_policies = BTreeSet::new();
    let first = run.traces.first();
    for trace in &run.traces {
        if !seen_policies.insert(trace.policy) {
            blockers.push(Blocker::DuplicatePolicy);
        }
        blockers.extend(validate_trace(&run.task, trace));
        if let Some(reference) = first {
            if trace.matched_seed_id != reference.matched_seed_id {
                blockers.push(Blocker::UnmatchedSeed);
            }
            if trace.outcomes.len() != reference.outcomes.len()
                || trace.initiated_experiment_count != reference.initiated_experiment_count
            {
                blockers.push(Blocker::UnmatchedBudget);
            }
            if sorted(&trace.training_group_ids) != sorted(&reference.training_group_ids) {
                blockers.push(Blocker::UnmatchedTrainingGroups);
            }
            if sorted(&trace.holdout_group_ids) != sorted(&reference.holdout_group_ids) {
                blockers.push(Blocker::UnmatchedHoldoutGroups);
            }
            if !same_holdout_actuals(&trace.holdout, &reference.holdout) {
                blockers.push(Blocker::HoldoutIdentityMismatch);
            }
        }
    }

    let mut metrics = Vec::new();
    if blockers.is_empty() {
        for trace in &run.traces {
            metrics.push(score_trace(&run.task, trace));
        }
    }

    BenchmarkEvaluation {
        valid: blockers.is_empty(),
        blockers: dedupe(blockers),
        trace_metrics: metrics,
    }
}

fn validate_task(task: &BenchmarkTask) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if task.task_id.trim().is_empty() || task.objectives.is_empty() {
        blockers.push(Blocker::MissingTaskIdentity);
    }
    for digest in [
        &task.candidate_space_digest,
        &task.target_function_digest,
        &task.acceptance_policy_digest,
        &task.initial_design_digest,
        &task.holdout_manifest_digest,
    ] {
        if !valid_sha256(digest) {
            blockers.push(Blocker::InvalidTaskDigest);
        }
    }
    if task.experiment_budget == 0 {
        blockers.push(Blocker::InvalidBudget);
    }
    if !task.false_feasible_probability_threshold.is_finite()
        || !(0.0..=1.0).contains(&task.false_feasible_probability_threshold)
    {
        blockers.push(Blocker::InvalidProbabilityThreshold);
    }
    for objective in &task.objectives {
        if objective.metric_id.trim().is_empty() || objective.unit.trim().is_empty() {
            blockers.push(Blocker::InvalidObjective);
        }
    }
    if let Some(oracle) = task.oracle_best_objective_0 {
        if !oracle.is_finite() {
            blockers.push(Blocker::InvalidOracle);
        }
    }
    if task.objectives.len() >= 2 {
        if let Some(reference) = &task.hypervolume_reference_point {
            if reference.len() != 2 || reference.iter().any(|value| !value.is_finite()) {
                blockers.push(Blocker::InvalidHypervolumeReference);
            }
        }
    }
    blockers
}

fn validate_trace(task: &BenchmarkTask, trace: &PolicyTrace) -> Vec<Blocker> {
    let mut blockers = Vec::new();
    if trace.matched_seed_id.trim().is_empty() {
        blockers.push(Blocker::MissingTaskIdentity);
    }
    if task.split_unit == SplitUnit::RandomRows {
        blockers.push(Blocker::UnsupportedRandomRowHoldout);
    }
    if trace.initiated_experiment_count != trace.outcomes.len() as u32
        || trace.initiated_experiment_count > task.experiment_budget
    {
        blockers.push(Blocker::InitiatedOutcomeCountMismatch);
    }
    if trace.training_group_ids.is_empty() || trace.holdout_group_ids.is_empty() {
        blockers.push(Blocker::InvalidHoldoutSplit);
    }

    let training_groups = trace.training_group_ids.iter().collect::<BTreeSet<_>>();
    let holdout_groups = trace.holdout_group_ids.iter().collect::<BTreeSet<_>>();
    if !training_groups.is_disjoint(&holdout_groups) {
        blockers.push(Blocker::TrainingHoldoutLeakage);
    }

    let mut experiment_ids = BTreeSet::new();
    let mut candidate_dimensions = None;
    for outcome in &trace.outcomes {
        if outcome.experiment_id.trim().is_empty()
            || outcome.candidate_id.trim().is_empty()
            || outcome.material_lot_id.trim().is_empty()
            || outcome.build_id.trim().is_empty()
            || outcome.process_revision.trim().is_empty()
        {
            blockers.push(Blocker::MissingExperimentIdentity);
        }
        if !experiment_ids.insert(outcome.experiment_id.as_str()) {
            blockers.push(Blocker::DuplicateExperimentId);
        }
        if !training_groups.contains(&group_key(task.split_unit, outcome)) {
            blockers.push(Blocker::MissingTrainingGroup);
        }
        if outcome.objective_values.len() != task.objectives.len()
            || outcome.objective_values.iter().any(|v| !v.is_finite())
        {
            blockers.push(Blocker::InvalidObjectiveVector);
        }
        if outcome.design_features.is_empty()
            || outcome.design_features.iter().any(|v| !v.is_finite())
        {
            blockers.push(Blocker::InvalidDesignFeatureVector);
        }
        match candidate_dimensions {
            Some(dimension) if dimension != outcome.design_features.len() => {
                blockers.push(Blocker::InvalidDesignFeatureVector);
            }
            None => candidate_dimensions = Some(outcome.design_features.len()),
            _ => {}
        }
        if !outcome.predicted_feasibility_probability.is_finite()
            || !(0.0..=1.0).contains(&outcome.predicted_feasibility_probability)
        {
            blockers.push(Blocker::InvalidProbability);
        }
    }

    let mut holdout_ids = BTreeSet::new();
    for observation in &trace.holdout {
        if observation.holdout_id.trim().is_empty()
            || observation.candidate_id.trim().is_empty()
            || observation.material_lot_id.trim().is_empty()
            || observation.build_id.trim().is_empty()
            || observation.process_revision.trim().is_empty()
        {
            blockers.push(Blocker::MissingExperimentIdentity);
        }
        if !holdout_ids.insert(observation.holdout_id.as_str()) {
            blockers.push(Blocker::DuplicateHoldoutId);
        }
        if !holdout_groups.contains(&group_key_holdout(task.split_unit, observation)) {
            blockers.push(Blocker::MissingHoldoutGroup);
        }
        if observation.objective_values.len() != task.objectives.len()
            || observation.objective_values.iter().any(|v| !v.is_finite())
        {
            blockers.push(Blocker::InvalidObjectiveVector);
        }
        if observation.design_features.is_empty()
            || observation.design_features.iter().any(|v| !v.is_finite())
        {
            blockers.push(Blocker::InvalidDesignFeatureVector);
        }
        if candidate_dimensions.is_some_and(|dimension| dimension != observation.design_features.len()) {
            blockers.push(Blocker::InvalidDesignFeatureVector);
        }
        if !observation.predicted_feasibility_probability.is_finite()
            || !(0.0..=1.0).contains(&observation.predicted_feasibility_probability)
        {
            blockers.push(Blocker::InvalidProbability);
        }
        let mut interval_indices = BTreeSet::new();
        for interval in &observation.predicted_intervals {
            if interval.objective_index >= task.objectives.len()
                || !interval.lower.is_finite()
                || !interval.upper.is_finite()
                || interval.lower > interval.upper
            {
                blockers.push(Blocker::InvalidInterval);
            }
            if !interval_indices.insert(interval.objective_index) {
                blockers.push(Blocker::DuplicatePredictionInterval);
            }
        }
        if interval_indices.len() != task.objectives.len() {
            blockers.push(Blocker::MissingPredictionInterval);
        }
    }
    blockers
}

fn group_key<'a>(unit: SplitUnit, outcome: &'a ExperimentOutcome) -> &'a str {
    match unit {
        SplitUnit::MaterialLot => &outcome.material_lot_id,
        SplitUnit::Build => &outcome.build_id,
        SplitUnit::ProcessRevision => &outcome.process_revision,
        SplitUnit::RandomRows => &outcome.experiment_id,
    }
}

fn group_key_holdout<'a>(unit: SplitUnit, observation: &'a HoldoutObservation) -> &'a str {
    match unit {
        SplitUnit::MaterialLot => &observation.material_lot_id,
        SplitUnit::Build => &observation.build_id,
        SplitUnit::ProcessRevision => &observation.process_revision,
        SplitUnit::RandomRows => &observation.holdout_id,
    }
}

fn score_trace(task: &BenchmarkTask, trace: &PolicyTrace) -> TraceMetrics {
    let qualified = trace
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == ExperimentDisposition::Qualified)
        .collect::<Vec<_>>();
    let confirmed_failures = trace
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == ExperimentDisposition::Fail)
        .count() as u64;
    let unresolved = trace
        .outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.disposition,
                ExperimentDisposition::Indeterminate
                    | ExperimentDisposition::NotEvaluable
                    | ExperimentDisposition::Missing
            )
        })
        .count() as u64;
    let first_qualified = trace
        .outcomes
        .iter()
        .filter(|outcome| outcome.disposition == ExperimentDisposition::Qualified)
        .min_by_key(|outcome| outcome.iteration);
    let mut accumulated_cost = 0u64;
    let mut cost_to_first = None;
    for outcome in {
        let mut values = trace.outcomes.iter().collect::<Vec<_>>();
        values.sort_by_key(|outcome| outcome.iteration);
        values
    } {
        accumulated_cost = accumulated_cost.saturating_add(outcome.cost_minor);
        if outcome.disposition == ExperimentDisposition::Qualified {
            cost_to_first = Some(accumulated_cost);
            break;
        }
    }

    let false_feasible_selections = trace
        .outcomes
        .iter()
        .filter(|outcome| {
            outcome.predicted_feasibility_probability >= task.false_feasible_probability_threshold
                && outcome.disposition == ExperimentDisposition::Fail
        })
        .count() as u64;
    let unresolved_feasibility_selections = trace
        .outcomes
        .iter()
        .filter(|outcome| {
            outcome.predicted_feasibility_probability >= task.false_feasible_probability_threshold
                && matches!(
                    outcome.disposition,
                    ExperimentDisposition::Indeterminate
                        | ExperimentDisposition::NotEvaluable
                        | ExperimentDisposition::Missing
                )
        })
        .count() as u64;

    let simple_regret = task.oracle_best_objective_0.and_then(|oracle| {
        qualified
            .iter()
            .map(|outcome| outcome.objective_values[0])
            .reduce(f64::min)
            .map(|observed| match task.objectives[0].direction {
                ObjectiveDirection::Minimize => (observed - oracle).max(0.0),
                ObjectiveDirection::Maximize => (oracle - observed).max(0.0),
            })
    });

    let hypervolume = if task.objectives.len() == 2 {
        task.hypervolume_reference_point.as_ref().map(|reference| {
            hypervolume_2d(
                &qualified
                    .iter()
                    .map(|outcome| outcome.objective_values.clone())
                    .collect::<Vec<_>>(),
                reference,
                &task.objectives,
            )
        })
    } else {
        None
    };

    let holdout_coverage = (0..task.objectives.len())
        .map(|objective_index| {
            if trace.holdout.is_empty() {
                None
            } else {
                let covered = trace
                    .holdout
                    .iter()
                    .filter(|observation| {
                        let interval = observation
                            .predicted_intervals
                            .iter()
                            .find(|interval| interval.objective_index == objective_index);
                        interval.is_some_and(|interval| {
                            let actual = observation.objective_values[objective_index];
                            actual >= interval.lower && actual <= interval.upper
                        })
                    })
                    .count();
                Some(covered as f64 / trace.holdout.len() as f64)
            }
        })
        .collect::<Vec<_>>();
    let holdout_width = (0..task.objectives.len())
        .map(|objective_index| {
            if trace.holdout.is_empty() {
                None
            } else {
                Some(
                    trace
                        .holdout
                        .iter()
                        .filter_map(|observation| {
                            observation
                                .predicted_intervals
                                .iter()
                                .find(|interval| interval.objective_index == objective_index)
                                .map(|interval| interval.upper - interval.lower)
                        })
                        .sum::<f64>()
                        / trace.holdout.len() as f64,
                )
            }
        })
        .collect::<Vec<_>>();

    let confirmed_holdout = trace
        .holdout
        .iter()
        .filter(|observation| {
            matches!(
                observation.disposition,
                ExperimentDisposition::Qualified | ExperimentDisposition::Fail
            )
        })
        .collect::<Vec<_>>();
    let holdout_brier = if confirmed_holdout.is_empty() {
        None
    } else {
        Some(
            confirmed_holdout
                .iter()
                .map(|observation| {
                    let actual = f64::from(observation.disposition == ExperimentDisposition::Qualified);
                    (observation.predicted_feasibility_probability - actual).powi(2)
                })
                .sum::<f64>()
                / confirmed_holdout.len() as f64,
        )
    };
    let holdout_unresolved_count = trace
        .holdout
        .iter()
        .filter(|observation| {
            matches!(
                observation.disposition,
                ExperimentDisposition::Indeterminate
                    | ExperimentDisposition::NotEvaluable
                    | ExperimentDisposition::Missing
            )
        })
        .count() as u64;

    let unique_candidates = trace
        .outcomes
        .iter()
        .map(|outcome| outcome.candidate_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let unique_candidate_fraction = if trace.outcomes.is_empty() {
        0.0
    } else {
        unique_candidates as f64 / trace.outcomes.len() as f64
    };
    let mean_pairwise_design_distance = mean_pairwise_distance(
        &trace
            .outcomes
            .iter()
            .map(|outcome| outcome.design_features.as_slice())
            .collect::<Vec<_>>(),
    );

    TraceMetrics {
        policy: trace.policy,
        experiment_count: trace.outcomes.len() as u64,
        qualified_experiments: qualified.len() as u64,
        confirmed_failures,
        unresolved_experiments: unresolved,
        first_qualified_iteration: first_qualified.map(|outcome| outcome.iteration),
        total_experiment_cost_minor: trace
            .outcomes
            .iter()
            .map(|outcome| outcome.cost_minor)
            .sum(),
        cost_to_first_qualified_minor: cost_to_first,
        false_feasible_selections,
        unresolved_feasibility_selections,
        simple_regret_objective_0: simple_regret,
        pareto_hypervolume_2d: hypervolume,
        unique_candidate_fraction,
        mean_pairwise_design_distance,
        holdout_interval_coverage: holdout_coverage,
        holdout_mean_interval_width: holdout_width,
        holdout_feasibility_brier: holdout_brier,
        holdout_unresolved_count,
    }
}

fn hypervolume_2d(
    points: &[Vec<f64>],
    reference: &[f64],
    objectives: &[ObjectiveDefinition],
) -> f64 {
    if points.is_empty() || reference.len() != 2 {
        return 0.0;
    }
    let transform = |value: f64, direction: ObjectiveDirection| match direction {
        ObjectiveDirection::Minimize => value,
        ObjectiveDirection::Maximize => -value,
    };
    let ref_x = transform(reference[0], objectives[0].direction);
    let ref_y = transform(reference[1], objectives[1].direction);
    let mut transformed = points
        .iter()
        .filter(|point| point.len() == 2)
        .map(|point| {
            (
                transform(point[0], objectives[0].direction),
                transform(point[1], objectives[1].direction),
            )
        })
        .filter(|(x, y)| *x < ref_x && *y < ref_y)
        .collect::<Vec<_>>();
    transformed.sort_by(|left, right| left.0.total_cmp(&right.0));

    let mut area = 0.0;
    let mut best_y = ref_y;
    let mut previous_x = transformed.first().map_or(ref_x, |point| point.0);
    for (x, y) in transformed {
        if x > previous_x {
            area += (x - previous_x) * (ref_y - best_y);
        }
        best_y = best_y.min(y);
        previous_x = x;
    }
    if previous_x < ref_x {
        area += (ref_x - previous_x) * (ref_y - best_y);
    }
    area.max(0.0)
}

fn mean_pairwise_distance(points: &[&[f64]]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut total = 0.0;
    let mut pairs = 0u64;
    for left in 0..points.len() {
        for right in (left + 1)..points.len() {
            let distance = points[left]
                .iter()
                .zip(points[right])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
            total += distance;
            pairs += 1;
        }
    }
    total / pairs as f64
}

fn same_holdout_actuals(left: &[HoldoutObservation], right: &[HoldoutObservation]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let left_by_id = left
        .iter()
        .map(|observation| (observation.holdout_id.as_str(), observation))
        .collect::<BTreeMap<_, _>>();
    right.iter().all(|observation| {
        left_by_id.get(observation.holdout_id.as_str()).is_some_and(|other| {
            other.candidate_id == observation.candidate_id
                && other.material_lot_id == observation.material_lot_id
                && other.build_id == observation.build_id
                && other.process_revision == observation.process_revision
                && other.disposition == observation.disposition
                && other.objective_values == observation.objective_values
                && other.design_features == observation.design_features
        })
    })
}

fn sorted(values: &[String]) -> BTreeSet<&str> {
    values.iter().map(String::as_str).collect()
}

fn dedupe(mut blockers: Vec<Blocker>) -> Vec<Blocker> {
    blockers.sort_by_key(|blocker| format!("{blocker:?}"));
    blockers.dedup();
    blockers
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(ch: char) -> String {
        format!("sha256:{}", ch.to_string().repeat(64))
    }

    fn task() -> BenchmarkTask {
        BenchmarkTask {
            task_id: "MAGNETIC-AL-TASK-001".to_string(),
            candidate_space_digest: digest('a'),
            target_function_digest: digest('b'),
            acceptance_policy_digest: digest('c'),
            initial_design_digest: digest('d'),
            holdout_manifest_digest: digest('e'),
            split_unit: SplitUnit::MaterialLot,
            experiment_budget: 4,
            objectives: vec![
                ObjectiveDefinition {
                    metric_id: "field_error".to_string(),
                    unit: "mT".to_string(),
                    direction: ObjectiveDirection::Minimize,
                },
                ObjectiveDefinition {
                    metric_id: "remanence".to_string(),
                    unit: "T".to_string(),
                    direction: ObjectiveDirection::Maximize,
                },
            ],
            oracle_best_objective_0: Some(5.0),
            hypervolume_reference_point: Some(vec![50.0, 0.0]),
            false_feasible_probability_threshold: 0.5,
        }
    }

    fn outcome(
        id: &str,
        candidate: &str,
        iteration: u32,
        disposition: ExperimentDisposition,
        cost: u64,
        objectives: Vec<f64>,
        probability: f64,
    ) -> ExperimentOutcome {
        ExperimentOutcome {
            experiment_id: id.to_string(),
            candidate_id: candidate.to_string(),
            material_lot_id: "LOT-A".to_string(),
            build_id: format!("BUILD-{iteration}"),
            process_revision: "MEX-v1".to_string(),
            iteration,
            disposition,
            cost_minor: cost,
            objective_values: objectives,
            design_features: vec![iteration as f64, iteration as f64 * 0.5],
            predicted_feasibility_probability: probability,
        }
    }

    fn holdout(
        id: &str,
        candidate: &str,
        disposition: ExperimentDisposition,
        objectives: Vec<f64>,
        probability: f64,
    ) -> HoldoutObservation {
        HoldoutObservation {
            holdout_id: id.to_string(),
            candidate_id: candidate.to_string(),
            material_lot_id: "LOT-B".to_string(),
            build_id: format!("HOLDOUT-BUILD-{id}"),
            process_revision: "MEX-v1".to_string(),
            disposition,
            objective_values: objectives,
            design_features: vec![0.5, 0.75],
            predicted_feasibility_probability: probability,
            predicted_intervals: vec![
                PredictionInterval { objective_index: 0, lower: 10.0, upper: 14.0 },
                PredictionInterval { objective_index: 1, lower: 0.35, upper: 0.50 },
            ],
        }
    }

    fn trace(policy: PolicyFamily) -> PolicyTrace {
        let outcomes = vec![
            outcome("EXP-1", "CAND-1", 1, ExperimentDisposition::Fail, 2_000, vec![30.0, 0.2], 0.95),
            outcome("EXP-2", "CAND-2", 2, ExperimentDisposition::Indeterminate, 3_000, vec![20.0, 0.3], 0.90),
            outcome("EXP-3", "CAND-3", 3, ExperimentDisposition::Qualified, 4_000, vec![10.0, 0.4], 0.80),
            outcome("EXP-4", "CAND-4", 4, ExperimentDisposition::Qualified, 2_500, vec![15.0, 0.45], 0.85),
        ];
        PolicyTrace {
            policy,
            matched_seed_id: "SEED-001".to_string(),
            training_group_ids: vec!["LOT-A".to_string()],
            holdout_group_ids: vec!["LOT-B".to_string()],
            initiated_experiment_count: 4,
            outcomes,
            holdout: vec![
                holdout("HOLD-1", "HC-1", ExperimentDisposition::Qualified, vec![12.0, 0.42], 0.80),
                holdout("HOLD-2", "HC-2", ExperimentDisposition::Fail, vec![30.0, 0.1], 0.20),
            ],
        }
    }

    fn run() -> BenchmarkRun {
        BenchmarkRun {
            task: task(),
            traces: vec![
                trace(PolicyFamily::ExpertPrior),
                trace(PolicyFamily::GaussianProcessExpectedImprovement),
            ],
        }
    }

    #[test]
    fn matched_policies_produce_known_answer_metrics() {
        let result = evaluate_run(&run());
        assert!(result.valid, "{:?}", result.blockers);
        assert_eq!(result.trace_metrics.len(), 2);
        let metrics = &result.trace_metrics[0];
        assert_eq!(metrics.qualified_experiments, 2);
        assert_eq!(metrics.confirmed_failures, 1);
        assert_eq!(metrics.unresolved_experiments, 1);
        assert_eq!(metrics.first_qualified_iteration, Some(3));
        assert_eq!(metrics.cost_to_first_qualified_minor, Some(9_000));
        assert_eq!(metrics.total_experiment_cost_minor, 11_500);
        assert_eq!(metrics.false_feasible_selections, 1);
        assert_eq!(metrics.unresolved_feasibility_selections, 1);
        assert_eq!(metrics.simple_regret_objective_0, Some(5.0));
        assert_eq!(metrics.pareto_hypervolume_2d, Some(17.75));
        assert_eq!(metrics.holdout_interval_coverage, vec![Some(1.0), Some(1.0)]);
        assert_eq!(metrics.holdout_mean_interval_width, vec![Some(4.0), Some(0.15)]);
        assert_eq!(metrics.holdout_feasibility_brier, Some(0.04));
        assert_eq!(metrics.holdout_unresolved_count, 0);
    }

    #[test]
    fn lot_holdout_prevents_same_lot_leakage() {
        let mut run = run();
        run.traces[0].holdout[0].material_lot_id = "LOT-A".to_string();
        let result = evaluate_run(&run);
        assert!(!result.valid);
        assert!(result.blockers.contains(&Blocker::TrainingHoldoutLeakage));
    }

    #[test]
    fn random_row_splits_are_not_qualification_grade() {
        let mut run = run();
        run.task.split_unit = SplitUnit::RandomRows;
        let result = evaluate_run(&run);
        assert!(!result.valid);
        assert!(result.blockers.contains(&Blocker::UnsupportedRandomRowHoldout));
    }

    #[test]
    fn missing_outcome_cannot_be_dropped_from_initiated_denominator() {
        let mut run = run();
        run.traces[0].outcomes.pop();
        let result = evaluate_run(&run);
        assert!(!result.valid);
        assert!(result.blockers.contains(&Blocker::InitiatedOutcomeCountMismatch));
    }

    #[test]
    fn unresolved_selection_is_not_reported_as_confirmed_false_feasible() {
        let result = evaluate_run(&run());
        assert!(result.valid);
        assert_eq!(result.trace_metrics[0].false_feasible_selections, 1);
        assert_eq!(result.trace_metrics[0].unresolved_feasibility_selections, 1);
    }

    #[test]
    fn different_matched_seed_invalidates_policy_comparison() {
        let mut run = run();
        run.traces[1].matched_seed_id = "SEED-002".to_string();
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::UnmatchedSeed));
    }

    #[test]
    fn unequal_experiment_budgets_invalidate_comparison() {
        let mut run = run();
        run.traces[1].outcomes.pop();
        run.traces[1].initiated_experiment_count = 3;
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::UnmatchedBudget));
    }

    #[test]
    fn holdout_candidate_identities_must_match_across_policies() {
        let mut run = run();
        run.traces[1].holdout[0].candidate_id = "OTHER".to_string();
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::HoldoutIdentityMismatch));
    }

    #[test]
    fn duplicate_experiment_ids_are_rejected() {
        let mut run = run();
        run.traces[0].outcomes[1].experiment_id = "EXP-1".to_string();
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::DuplicateExperimentId));
    }

    #[test]
    fn invalid_prediction_probability_is_rejected() {
        let mut run = run();
        run.traces[0].holdout[0].predicted_feasibility_probability = 1.5;
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::InvalidProbability));
    }

    #[test]
    fn duplicate_policy_in_matched_comparison_is_rejected() {
        let mut run = run();
        run.traces[1].policy = PolicyFamily::ExpertPrior;
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::DuplicatePolicy));
    }

    #[test]
    fn unqualified_trials_still_contribute_to_total_and_first_pass_cost() {
        let mut trace = trace(PolicyFamily::Random);
        trace.outcomes[0].disposition = ExperimentDisposition::Fail;
        trace.outcomes[1].disposition = ExperimentDisposition::Fail;
        trace.outcomes[2].disposition = ExperimentDisposition::Fail;
        let metrics = score_trace(&task(), &trace);
        assert_eq!(metrics.cost_to_first_qualified_minor, None);
        assert_eq!(metrics.total_experiment_cost_minor, 11_500);
        assert_eq!(metrics.qualified_experiments, 1);
    }

    #[test]
    fn missing_holdout_interval_invalidates_the_trace() {
        let mut run = run();
        run.traces[0].holdout[0].predicted_intervals.pop();
        let result = evaluate_run(&run);
        assert!(result.blockers.contains(&Blocker::MissingPredictionInterval));
    }

    #[test]
    fn holdout_unknowns_are_excluded_from_brier_not_assumed_false() {
        let mut run = run();
        run.traces[0].holdout[1].disposition = ExperimentDisposition::Indeterminate;
        let metrics = score_trace(&run.task, &run.traces[0]);
        assert_eq!(metrics.holdout_feasibility_brier, Some(0.03999999999999998));
        assert_eq!(metrics.holdout_unresolved_count, 1);
    }
}
