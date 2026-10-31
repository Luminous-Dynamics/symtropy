//! Algorithm-level qualification on analytic multi-objective functions.
//!
//! This lane executes actual optimization policies on ZDT1/ZDT2, whose Pareto fronts are known.
//! It measures quality at fixed objective-evaluation checkpoints with identical per-run budgets.
//! These are cheap synthetic objective calls, not CFD solves or aerodynamic evidence.
//!
//! Baselines: uniform random search and NSGA-II (elitist nondominated sorting + crowding).
//! Every decision vector is exported so an independent implementation can replay objectives,
//! rebuild the archive, and recompute hypervolume and inverted generational distance.

use std::cmp::Ordering;
use std::collections::HashSet;

const REFERENCE_POINT: (f64, f64) = (1.1, 1.1);
const SBX_ETA: f64 = 15.0;
const MUTATION_ETA: f64 = 20.0;
const DIFFERENTIAL_WEIGHT: f64 = 0.5;
const DIFFERENTIAL_CROSSOVER: f64 = 0.9;
const IGD_REFERENCE_POINTS: usize = 201;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticTask {
    Zdt1,
    Zdt2,
}

impl AnalyticTask {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zdt1 => "zdt1",
            Self::Zdt2 => "zdt2",
        }
    }

    fn exact_hypervolume(self) -> f64 {
        let (reference_x, reference_y) = REFERENCE_POINT;
        let curve_integral = match self {
            Self::Zdt1 => (reference_y - 1.0) + 2.0 / 3.0,
            Self::Zdt2 => (reference_y - 1.0) + 1.0 / 3.0,
        };
        curve_integral + (reference_x - 1.0) * reference_y
    }

    fn pareto_y(self, x: f64) -> f64 {
        match self {
            Self::Zdt1 => 1.0 - x.sqrt(),
            Self::Zdt2 => 1.0 - x * x,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizerQualificationConfig {
    pub decision_dimensions: usize,
    pub population_size: usize,
    pub maximum_evaluations: usize,
    pub checkpoints: Vec<usize>,
    pub seeds: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizerCandidateTrace {
    pub task_id: String,
    pub policy_id: String,
    pub seed: u64,
    pub evaluation: usize,
    pub decision_vector: Vec<f64>,
    pub objective_1: f64,
    pub objective_2: f64,
    pub g_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizerQualityCheckpoint {
    pub task_id: String,
    pub policy_id: String,
    pub seed: u64,
    pub evaluation_budget: usize,
    pub actual_evaluations: usize,
    pub archive_size: usize,
    pub hypervolume: f64,
    pub exact_front_hypervolume: f64,
    pub hypervolume_fraction_of_exact_front: f64,
    pub inverted_generational_distance: f64,
    pub mean_g_value: f64,
    pub evidence_level: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptimizerQualificationReport {
    pub decision_dimensions: usize,
    pub population_size: usize,
    pub maximum_evaluations: usize,
    pub checkpoints: Vec<usize>,
    pub seeds: Vec<u64>,
    pub summaries: Vec<OptimizerQualityCheckpoint>,
    pub candidates: Vec<OptimizerCandidateTrace>,
}

#[derive(Debug, Clone)]
struct Individual {
    x: Vec<f64>,
    f1: f64,
    f2: f64,
}

#[derive(Debug, Clone)]
struct RankedIndividual {
    rank: usize,
    crowding: f64,
}

#[derive(Debug, Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
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

    fn unit(&mut self) -> f64 {
        const SCALE: f64 = 1.0 / ((1_u64 << 53) as f64);
        ((self.next_u64() >> 11) as f64) * SCALE
    }

    fn index(&mut self, upper: usize) -> usize {
        (self.next_u64() as usize) % upper
    }
}

pub fn evaluate_optimizer_qualification(
    config: &OptimizerQualificationConfig,
) -> Result<OptimizerQualificationReport, String> {
    validate_config(config)?;

    let tasks = [AnalyticTask::Zdt1, AnalyticTask::Zdt2];
    let policies = ["uniform_random", "nsga2", "gde3"];
    let mut summaries = Vec::with_capacity(
        tasks.len() * policies.len() * config.seeds.len() * config.checkpoints.len(),
    );
    let mut candidates = Vec::with_capacity(
        tasks.len() * policies.len() * config.seeds.len() * config.maximum_evaluations,
    );

    for task in tasks {
        for seed in &config.seeds {
            for policy_id in policies {
                let run = run_policy(
                    task,
                    policy_id,
                    *seed,
                    config.decision_dimensions,
                    config.population_size,
                    config.maximum_evaluations,
                )?;
                for checkpoint in &config.checkpoints {
                    summaries.push(summarize_checkpoint(
                        task,
                        policy_id,
                        *seed,
                        *checkpoint,
                        &run[..*checkpoint],
                    )?);
                }
                candidates.extend(run);
            }
        }
    }

    Ok(OptimizerQualificationReport {
        decision_dimensions: config.decision_dimensions,
        population_size: config.population_size,
        maximum_evaluations: config.maximum_evaluations,
        checkpoints: config.checkpoints.clone(),
        seeds: config.seeds.clone(),
        summaries,
        candidates,
    })
}

fn validate_config(config: &OptimizerQualificationConfig) -> Result<(), String> {
    if config.decision_dimensions < 2 || config.decision_dimensions > 256 {
        return Err("decision_dimensions must be in [2, 256]".to_owned());
    }
    if config.population_size < 4 || config.population_size > 4096 {
        return Err("population_size must be in [4, 4096]".to_owned());
    }
    if config.maximum_evaluations < config.population_size {
        return Err("maximum_evaluations must be >= population_size".to_owned());
    }
    if config.seeds.is_empty() {
        return Err("at least one deterministic seed is required".to_owned());
    }
    let mut seeds = HashSet::new();
    for seed in &config.seeds {
        if !seeds.insert(*seed) {
            return Err(format!("duplicate optimizer seed {seed}"));
        }
    }
    if config.checkpoints.is_empty() {
        return Err("at least one evaluation checkpoint is required".to_owned());
    }
    let mut previous = 0;
    for checkpoint in &config.checkpoints {
        if *checkpoint <= previous || *checkpoint > config.maximum_evaluations {
            return Err("checkpoints must be strictly increasing and <= maximum_evaluations".to_owned());
        }
        previous = *checkpoint;
    }
    if config.checkpoints.last().copied() != Some(config.maximum_evaluations) {
        return Err("the final checkpoint must equal maximum_evaluations".to_owned());
    }
    Ok(())
}

fn run_policy(
    task: AnalyticTask,
    policy_id: &str,
    seed: u64,
    dimensions: usize,
    population_size: usize,
    maximum_evaluations: usize,
) -> Result<Vec<OptimizerCandidateTrace>, String> {
    if !matches!(policy_id, "uniform_random" | "nsga2" | "gde3") {
        return Err(format!("unknown optimizer policy {policy_id}"));
    }
    // Common random numbers make the initial population identical across policies.
    let mut rng = SplitMix64::new(seed);
    let mut trace = Vec::with_capacity(maximum_evaluations);

    if policy_id == "uniform_random" {
        while trace.len() < maximum_evaluations {
            let x = random_vector(dimensions, &mut rng);
            trace.push(evaluate_trace(task, policy_id, seed, trace.len() + 1, x)?);
        }
        return Ok(trace);
    }

    let initial_size = population_size.min(maximum_evaluations);
    let mut population = Vec::with_capacity(population_size);
    for _ in 0..initial_size {
        let x = random_vector(dimensions, &mut rng);
        let candidate = evaluate_trace(task, policy_id, seed, trace.len() + 1, x)?;
        population.push(individual_from_trace(&candidate));
        trace.push(candidate);
    }

    if policy_id == "gde3" {
        while trace.len() < maximum_evaluations {
            let batch_size = population_size.min(maximum_evaluations - trace.len());
            let mut competition = Vec::with_capacity(population_size + batch_size);
            for target_index in 0..batch_size {
                let donors = distinct_donors(target_index, population.len(), &mut rng);
                let forced_coordinate = rng.index(dimensions);
                let mut trial_x = Vec::with_capacity(dimensions);
                for coordinate in 0..dimensions {
                    let mutant = population[donors[0]].x[coordinate]
                        + DIFFERENTIAL_WEIGHT
                            * (population[donors[1]].x[coordinate]
                                - population[donors[2]].x[coordinate]);
                    let use_mutant = coordinate == forced_coordinate
                        || rng.unit() < DIFFERENTIAL_CROSSOVER;
                    trial_x.push(if use_mutant {
                        mutant.clamp(0.0, 1.0)
                    } else {
                        population[target_index].x[coordinate]
                    });
                }
                let trial_trace = evaluate_trace(
                    task,
                    policy_id,
                    seed,
                    trace.len() + 1,
                    trial_x,
                )?;
                let trial = individual_from_trace(&trial_trace);
                let parent = &population[target_index];
                if dominates(&trial, parent) {
                    competition.push(trial);
                } else if dominates(parent, &trial) {
                    competition.push(parent.clone());
                } else {
                    competition.push(parent.clone());
                    competition.push(trial);
                }
                trace.push(trial_trace);
            }
            population = environmental_selection(competition, population_size);
        }
        if trace.len() != maximum_evaluations {
            return Err("GDE3 exceeded or under-ran its declared evaluation budget".to_owned());
        }
        return Ok(trace);
    }

    if policy_id != "nsga2" {
        return Err(format!("unknown optimizer policy {policy_id}"));
    }

    while trace.len() < maximum_evaluations {
        let parent_scores = rank_and_crowding(&population);
        let batch_size = population_size.min(maximum_evaluations - trace.len());
        let mut offspring = Vec::with_capacity(batch_size);
        for _ in 0..batch_size {
            let first = tournament(&parent_scores, &mut rng);
            let second = tournament(&parent_scores, &mut rng);
            let child_x = make_child(&population[first].x, &population[second].x, &mut rng);
            let candidate = evaluate_trace(task, policy_id, seed, trace.len() + 1, child_x)?;
            offspring.push(individual_from_trace(&candidate));
            trace.push(candidate);
        }
        let mut combined = population;
        combined.extend(offspring);
        population = environmental_selection(combined, population_size);
    }

    if trace.len() != maximum_evaluations {
        return Err("optimizer exceeded or under-ran its declared evaluation budget".to_owned());
    }
    Ok(trace)
}

fn distinct_donors(
    target: usize,
    population_size: usize,
    rng: &mut SplitMix64,
) -> [usize; 3] {
    let mut donors = [usize::MAX; 3];
    for position in 0..3 {
        loop {
            let candidate = rng.index(population_size);
            if candidate != target && !donors[..position].contains(&candidate) {
                donors[position] = candidate;
                break;
            }
        }
    }
    donors
}

fn random_vector(dimensions: usize, rng: &mut SplitMix64) -> Vec<f64> {
    (0..dimensions).map(|_| rng.unit()).collect()
}

fn evaluate_trace(
    task: AnalyticTask,
    policy_id: &str,
    seed: u64,
    evaluation: usize,
    decision_vector: Vec<f64>,
) -> Result<OptimizerCandidateTrace, String> {
    let (objective_1, objective_2, g_value) = evaluate_objectives(task, &decision_vector)?;
    Ok(OptimizerCandidateTrace {
        task_id: task.as_str().to_owned(),
        policy_id: policy_id.to_owned(),
        seed,
        evaluation,
        decision_vector,
        objective_1,
        objective_2,
        g_value,
    })
}

pub fn evaluate_objectives(
    task: AnalyticTask,
    decision_vector: &[f64],
) -> Result<(f64, f64, f64), String> {
    if decision_vector.len() < 2 {
        return Err("ZDT objective requires at least two decision variables".to_owned());
    }
    if decision_vector
        .iter()
        .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
    {
        return Err("ZDT decision variables must be finite and in [0, 1]".to_owned());
    }

    let objective_1 = decision_vector[0];
    let mean_tail =
        decision_vector[1..].iter().sum::<f64>() / (decision_vector.len() - 1) as f64;
    let g_value = 1.0 + 9.0 * mean_tail;
    let ratio = objective_1 / g_value;
    let shape = match task {
        AnalyticTask::Zdt1 => 1.0 - ratio.sqrt(),
        AnalyticTask::Zdt2 => 1.0 - ratio * ratio,
    };
    let objective_2 = g_value * shape;
    if !objective_1.is_finite() || !objective_2.is_finite() || !g_value.is_finite() {
        return Err("ZDT objective evaluation produced non-finite output".to_owned());
    }
    Ok((objective_1, objective_2, g_value))
}

fn individual_from_trace(trace: &OptimizerCandidateTrace) -> Individual {
    Individual {
        x: trace.decision_vector.clone(),
        f1: trace.objective_1,
        f2: trace.objective_2,
    }
}

fn dominates(left: &Individual, right: &Individual) -> bool {
    left.f1 <= right.f1
        && left.f2 <= right.f2
        && (left.f1 < right.f1 || left.f2 < right.f2)
}

fn nondominated_fronts(population: &[Individual]) -> Vec<Vec<usize>> {
    let mut dominates_list = vec![Vec::<usize>::new(); population.len()];
    let mut dominated_count = vec![0_usize; population.len()];
    for left in 0..population.len() {
        for right in (left + 1)..population.len() {
            if dominates(&population[left], &population[right]) {
                dominates_list[left].push(right);
                dominated_count[right] += 1;
            } else if dominates(&population[right], &population[left]) {
                dominates_list[right].push(left);
                dominated_count[left] += 1;
            }
        }
    }

    let mut current = (0..population.len())
        .filter(|index| dominated_count[*index] == 0)
        .collect::<Vec<_>>();
    let mut fronts = Vec::new();
    while !current.is_empty() {
        let mut next = Vec::new();
        for index in &current {
            for dominated in &dominates_list[*index] {
                dominated_count[*dominated] -= 1;
                if dominated_count[*dominated] == 0 {
                    next.push(*dominated);
                }
            }
        }
        current.sort_unstable();
        fronts.push(current);
        current = next;
    }
    fronts
}

fn rank_and_crowding(population: &[Individual]) -> Vec<RankedIndividual> {
    let fronts = nondominated_fronts(population);
    let mut result = vec![
        RankedIndividual {
            rank: usize::MAX,
            crowding: 0.0,
        };
        population.len()
    ];
    for (rank, front) in fronts.iter().enumerate() {
        let distances = crowding_distances(front, population);
        for index in front {
            result[*index] = RankedIndividual {
                rank,
                crowding: distances[*index],
            };
        }
    }
    result
}

fn crowding_distances(front: &[usize], population: &[Individual]) -> Vec<f64> {
    let mut distances = vec![0.0; population.len()];
    if front.len() <= 2 {
        for index in front {
            distances[*index] = f64::INFINITY;
        }
        return distances;
    }

    for objective in 0..2 {
        let mut sorted = front.to_vec();
        sorted.sort_by(|left, right| {
            objective_value(&population[*left], objective)
                .total_cmp(&objective_value(&population[*right], objective))
        });
        let minimum = objective_value(&population[sorted[0]], objective);
        let last = *sorted.last().expect("non-empty front");
        let maximum = objective_value(&population[last], objective);
        distances[sorted[0]] = f64::INFINITY;
        distances[last] = f64::INFINITY;
        if maximum <= minimum {
            continue;
        }
        for position in 1..(sorted.len() - 1) {
            let index = sorted[position];
            if distances[index].is_finite() {
                let previous = objective_value(&population[sorted[position - 1]], objective);
                let next = objective_value(&population[sorted[position + 1]], objective);
                distances[index] += (next - previous) / (maximum - minimum);
            }
        }
    }
    distances
}

fn objective_value(individual: &Individual, objective: usize) -> f64 {
    if objective == 0 {
        individual.f1
    } else {
        individual.f2
    }
}

fn tournament(scores: &[RankedIndividual], rng: &mut SplitMix64) -> usize {
    let left = rng.index(scores.len());
    let right = rng.index(scores.len());
    match compare_ranked(&scores[left], &scores[right]) {
        Ordering::Less => left,
        Ordering::Greater => right,
        Ordering::Equal => {
            if rng.unit() < 0.5 {
                left
            } else {
                right
            }
        }
    }
}

fn compare_ranked(left: &RankedIndividual, right: &RankedIndividual) -> Ordering {
    left.rank.cmp(&right.rank).then_with(|| {
        right
            .crowding
            .partial_cmp(&left.crowding)
            .unwrap_or(Ordering::Equal)
    })
}

fn make_child(left: &[f64], right: &[f64], rng: &mut SplitMix64) -> Vec<f64> {
    let mut child = Vec::with_capacity(left.len());
    let mutation_probability = 1.0 / left.len() as f64;
    for (left_value, right_value) in left.iter().zip(right) {
        let mut value = if rng.unit() <= 0.5 {
            sbx_child(*left_value, *right_value, rng)
        } else {
            *left_value
        };
        if rng.unit() < mutation_probability {
            value = polynomial_mutation(value, rng);
        }
        child.push(value.clamp(0.0, 1.0));
    }
    child
}

fn sbx_child(left: f64, right: f64, rng: &mut SplitMix64) -> f64 {
    if (left - right).abs() <= 1.0e-14 {
        return left;
    }
    let u = rng.unit();
    let beta = if u <= 0.5 {
        (2.0 * u).powf(1.0 / (SBX_ETA + 1.0))
    } else {
        (1.0 / (2.0 * (1.0 - u))).powf(1.0 / (SBX_ETA + 1.0))
    };
    0.5 * ((1.0 + beta) * left + (1.0 - beta) * right)
}

fn polynomial_mutation(value: f64, rng: &mut SplitMix64) -> f64 {
    let random = rng.unit();
    let delta_1 = value;
    let delta_2 = 1.0 - value;
    let mutation_power = 1.0 / (MUTATION_ETA + 1.0);
    let delta_q = if random <= 0.5 {
        let xy = 1.0 - delta_1;
        let value_term =
            2.0 * random + (1.0 - 2.0 * random) * xy.powf(MUTATION_ETA + 1.0);
        value_term.powf(mutation_power) - 1.0
    } else {
        let xy = 1.0 - delta_2;
        let value_term = 2.0 * (1.0 - random)
            + 2.0 * (random - 0.5) * xy.powf(MUTATION_ETA + 1.0);
        1.0 - value_term.powf(mutation_power)
    };
    (value + delta_q).clamp(0.0, 1.0)
}

fn environmental_selection(combined: Vec<Individual>, population_size: usize) -> Vec<Individual> {
    let fronts = nondominated_fronts(&combined);
    let mut selected = Vec::with_capacity(population_size);
    for front in fronts {
        if selected.len() + front.len() <= population_size {
            selected.extend(front.into_iter().map(|index| combined[index].clone()));
            continue;
        }
        let distances = crowding_distances(&front, &combined);
        let mut boundary = front;
        boundary.sort_by(|left, right| {
            distances[*right]
                .partial_cmp(&distances[*left])
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.cmp(right))
        });
        let remaining = population_size - selected.len();
        selected.extend(
            boundary
                .into_iter()
                .take(remaining)
                .map(|index| combined[index].clone()),
        );
        break;
    }
    selected
}

fn summarize_checkpoint(
    task: AnalyticTask,
    policy_id: &str,
    seed: u64,
    evaluation_budget: usize,
    trace: &[OptimizerCandidateTrace],
) -> Result<OptimizerQualityCheckpoint, String> {
    if trace.len() != evaluation_budget {
        return Err(format!(
            "checkpoint expected {evaluation_budget} traces, found {}",
            trace.len()
        ));
    }
    let points = trace
        .iter()
        .map(|row| (row.objective_1, row.objective_2))
        .collect::<Vec<_>>();
    let archive = nondominated_points(&points);
    let hypervolume = hypervolume_2d(&archive, REFERENCE_POINT.0, REFERENCE_POINT.1);
    let exact_hypervolume = task.exact_hypervolume();
    let fraction = hypervolume / exact_hypervolume;
    if !hypervolume.is_finite()
        || !fraction.is_finite()
        || !(-1.0e-12..=1.0 + 1.0e-9).contains(&fraction)
    {
        return Err(format!(
            "{} {} seed {} produced invalid normalized hypervolume {}",
            task.as_str(), policy_id, seed, fraction
        ));
    }
    let igd = inverted_generational_distance(task, &archive);
    let mean_g = trace.iter().map(|row| row.g_value).sum::<f64>() / trace.len() as f64;
    if !igd.is_finite() || !mean_g.is_finite() {
        return Err("optimizer checkpoint produced non-finite quality metric".to_owned());
    }

    Ok(OptimizerQualityCheckpoint {
        task_id: task.as_str().to_owned(),
        policy_id: policy_id.to_owned(),
        seed,
        evaluation_budget,
        actual_evaluations: trace.len(),
        archive_size: archive.len(),
        hypervolume,
        exact_front_hypervolume: exact_hypervolume,
        hypervolume_fraction_of_exact_front: fraction,
        inverted_generational_distance: igd,
        mean_g_value: mean_g,
        evidence_level: "synthetic_analytic_optimizer_qualification_not_aerodynamic_evidence",
    })
}

pub fn nondominated_points(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let mut sorted = points
        .iter()
        .copied()
        .filter(|(f1, f2)| f1.is_finite() && f2.is_finite())
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| {
        left.0.total_cmp(&right.0).then_with(|| left.1.total_cmp(&right.1))
    });
    let mut archive = Vec::new();
    let mut best_second = f64::INFINITY;
    for point in sorted {
        if point.1 < best_second {
            best_second = point.1;
            archive.push(point);
        }
    }
    archive
}

pub fn hypervolume_2d(points: &[(f64, f64)], reference_x: f64, reference_y: f64) -> f64 {
    let eligible = points
        .iter()
        .copied()
        .filter(|(x, y)| {
            *x >= 0.0 && *y >= 0.0 && *x < reference_x && *y < reference_y
        })
        .collect::<Vec<_>>();
    let archive = nondominated_points(&eligible);
    let mut area = 0.0;
    for (index, point) in archive.iter().enumerate() {
        let next_x = archive
            .get(index + 1)
            .map(|next| next.0)
            .unwrap_or(reference_x);
        area += (next_x - point.0) * (reference_y - point.1);
    }
    area.max(0.0)
}

pub fn inverted_generational_distance(task: AnalyticTask, archive: &[(f64, f64)]) -> f64 {
    if archive.is_empty() {
        return f64::INFINITY;
    }
    let mut total = 0.0;
    for index in 0..IGD_REFERENCE_POINTS {
        let x = index as f64 / (IGD_REFERENCE_POINTS - 1) as f64;
        let target = (x, task.pareto_y(x));
        let nearest = archive
            .iter()
            .map(|point| {
                let dx = point.0 - target.0;
                let dy = point.1 - target.1;
                (dx * dx + dy * dy).sqrt()
            })
            .fold(f64::INFINITY, f64::min);
        total += nearest;
    }
    total / IGD_REFERENCE_POINTS as f64
}

pub const OPTIMIZER_SUMMARY_HEADER: &str = "task_id,policy_id,seed,evaluation_budget,actual_evaluations,archive_size,hypervolume,exact_front_hypervolume,hypervolume_fraction_of_exact_front,inverted_generational_distance,mean_g_value,evidence_level";
pub const OPTIMIZER_CANDIDATE_HEADER: &str = "task_id,policy_id,seed,evaluation,objective_1,objective_2,g_value,decision_vector_semicolon_separated";

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

pub fn summary_to_csv(report: &OptimizerQualificationReport) -> String {
    let mut output = format!("{OPTIMIZER_SUMMARY_HEADER}\n");
    for row in &report.summaries {
        output.push_str(&format!(
            "{},{},{},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{}\n",
            row.task_id,
            row.policy_id,
            row.seed,
            row.evaluation_budget,
            row.actual_evaluations,
            row.archive_size,
            row.hypervolume,
            row.exact_front_hypervolume,
            row.hypervolume_fraction_of_exact_front,
            row.inverted_generational_distance,
            row.mean_g_value,
            row.evidence_level
        ));
    }
    output
}

pub fn candidates_to_csv(report: &OptimizerQualificationReport) -> String {
    let mut output = format!("{OPTIMIZER_CANDIDATE_HEADER}\n");
    for row in &report.candidates {
        let vector = row
            .decision_vector
            .iter()
            .map(|value| format!("{value:.17}"))
            .collect::<Vec<_>>()
            .join(";");
        output.push_str(&format!(
            "{},{},{},{},{:.17},{:.17},{:.17},{}\n",
            row.task_id,
            row.policy_id,
            row.seed,
            row.evaluation,
            row.objective_1,
            row.objective_2,
            row.g_value,
            csv_escape(&vector)
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OptimizerQualificationConfig {
        OptimizerQualificationConfig {
            decision_dimensions: 10,
            population_size: 8,
            maximum_evaluations: 32,
            checkpoints: vec![8, 16, 32],
            seeds: vec![11, 29],
        }
    }

    #[test]
    fn zdt_known_points_match_their_exact_pareto_fronts() {
        let x = vec![0.36, 0.0, 0.0, 0.0];
        let zdt1 = evaluate_objectives(AnalyticTask::Zdt1, &x).expect("ZDT1 point");
        let zdt2 = evaluate_objectives(AnalyticTask::Zdt2, &x).expect("ZDT2 point");
        assert!((zdt1.0 - 0.36).abs() < 1.0e-12);
        assert!((zdt1.1 - 0.4).abs() < 1.0e-12);
        assert!((zdt2.1 - 0.8704).abs() < 1.0e-12);
        assert_eq!(zdt1.2, 1.0);
        assert_eq!(zdt2.2, 1.0);
    }

    #[test]
    fn rejects_out_of_domain_vectors_and_bad_run_budgets() {
        assert!(evaluate_objectives(AnalyticTask::Zdt1, &[0.5]).is_err());
        assert!(evaluate_objectives(AnalyticTask::Zdt1, &[0.5, 1.1]).is_err());
        let mut invalid = config();
        invalid.checkpoints = vec![8, 7, 32];
        assert!(evaluate_optimizer_qualification(&invalid).is_err());
        let mut invalid = config();
        invalid.seeds.push(11);
        assert!(evaluate_optimizer_qualification(&invalid)
            .expect_err("duplicate seed must reject")
            .contains("duplicate optimizer seed"));
    }

    #[test]
    fn algorithms_use_exact_budgets_and_repeat_deterministically() {
        let first = evaluate_optimizer_qualification(&config()).expect("valid qualification");
        let second = evaluate_optimizer_qualification(&config()).expect("repeat qualification");
        assert_eq!(first, second);
        assert_eq!(first.summaries.len(), 2 * 3 * 2 * 3);
        assert_eq!(first.candidates.len(), 2 * 3 * 2 * 32);
        assert!(first.summaries.iter().all(|row| {
            row.actual_evaluations == row.evaluation_budget
                && row.evidence_level.contains("not_aerodynamic")
                && row.hypervolume_fraction_of_exact_front <= 1.0 + 1.0e-9
        }));
        for task in ["zdt1", "zdt2"] {
            for seed in [11, 29] {
                for policy in ["uniform_random", "nsga2", "gde3"] {
                    let trace = first
                        .candidates
                        .iter()
                        .filter(|row| {
                            row.task_id == task && row.policy_id == policy && row.seed == seed
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(trace.len(), 32);
                    assert!(trace.iter().enumerate().all(|(index, row)| {
                        row.evaluation == index + 1
                            && row
                                .decision_vector
                                .iter()
                                .all(|value| (0.0..=1.0).contains(value))
                    }));
                }
                let random_initial = first
                    .candidates
                    .iter()
                    .filter(|row| {
                        row.task_id == task
                            && row.policy_id == "uniform_random"
                            && row.seed == seed
                    })
                    .take(8)
                    .map(|row| row.decision_vector.clone())
                    .collect::<Vec<_>>();
                let policy_initial = first
                    .candidates
                    .iter()
                    .filter(|row| {
                        row.task_id == task && row.policy_id == policy && row.seed == seed
                    })
                    .take(8)
                    .map(|row| row.decision_vector.clone())
                    .collect::<Vec<_>>();
                assert_eq!(random_initial, policy_initial);
            }
        }
    }

    #[test]
    fn archive_removes_dominated_points_and_hypervolume_is_correct() {
        let points = vec![(1.0, 8.0), (4.0, 3.0), (5.0, 9.0), (4.0, 2.0)];
        let archive = nondominated_points(&points);
        assert_eq!(archive, vec![(1.0, 8.0), (4.0, 2.0)]);
        assert!((hypervolume_2d(&points, 10.0, 10.0) - 54.0).abs() < 1.0e-12);
    }

    #[test]
    fn normalized_hypervolume_uses_known_continuous_pareto_front() {
        let zdt1 = AnalyticTask::Zdt1.exact_hypervolume();
        let zdt2 = AnalyticTask::Zdt2.exact_hypervolume();
        assert!((zdt1 - (0.1 + 2.0 / 3.0 + 0.11)).abs() < 1.0e-12);
        assert!((zdt2 - (0.1 + 1.0 / 3.0 + 0.11)).abs() < 1.0e-12);
    }
}
