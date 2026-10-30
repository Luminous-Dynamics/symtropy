// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic experiment-selection benchmark harness.
//!
//! This crate compares budget-matched selection policies on synthetic
//! objectives with known optima. It deliberately has no dependency on
//! Symthaea or a physics solver: an adapter can implement CandidatePolicy
//! later, while the benchmark owns objective evaluation, feasibility,
//! accounting, and the oracle used for regret.
//!
//! Synthetic objective success is not physical-simulation success. The next
//! qualification rung must use manufactured fluid cases and then a pinned
//! external solver, with all solver failures charged to the budget.

use std::f64::consts::TAU;
use std::time::Instant;

const GP_NOISE_VARIANCE: f64 = 0.01;
const GP_LENGTH_SCALE: f64 = 0.22;
const GP_UCB_BETA: f64 = 1.5;
const GP_CANDIDATE_COUNT: usize = 512;
const INITIAL_DESIGN_SIZE: usize = 8;

/// Stable synthetic problem identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProblemId {
    SmoothBowl,
    Multimodal,
    NarrowPeak,
    ConstrainedBowl,
    NonIdentifiableRidge,
}

impl ProblemId {
    pub const ALL: [Self; 5] = [
        Self::SmoothBowl,
        Self::Multimodal,
        Self::NarrowPeak,
        Self::ConstrainedBowl,
        Self::NonIdentifiableRidge,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::SmoothBowl => "smooth_bowl_2d",
            Self::Multimodal => "multimodal_2d",
            Self::NarrowPeak => "narrow_peak_2d",
            Self::ConstrainedBowl => "constrained_bowl_2d",
            Self::NonIdentifiableRidge => "non_identifiable_ridge_2d",
        }
    }

    fn optimum_value(self) -> f64 {
        match self {
            Self::SmoothBowl | Self::Multimodal | Self::NarrowPeak | Self::NonIdentifiableRidge => 1.0,
            Self::ConstrainedBowl => 0.7975,
        }
    }

    fn optimum_point(self) -> [f64; 2] {
        match self {
            Self::SmoothBowl => [0.25, 0.25],
            Self::Multimodal => [0.22, 0.72],
            Self::NarrowPeak => [0.83, 0.17],
            Self::ConstrainedBowl => [0.575, 0.575],
            Self::NonIdentifiableRidge => [0.7, 0.0],
        }
    }
}

/// Frozen synthetic objective; all functions maximize on [0,1]².
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyntheticProblem {
    pub id: ProblemId,
    /// Standard deviation at the origin for heteroskedastic observation noise.
    pub noise_scale_milli: u32,
}

impl SyntheticProblem {
    pub fn new(id: ProblemId) -> Self {
        Self {
            id,
            noise_scale_milli: 20,
        }
    }

    pub fn noise_scale(self) -> f64 {
        self.noise_scale_milli as f64 / 1_000.0
    }

    pub fn optimum_value(self) -> f64 {
        self.id.optimum_value()
    }

    /// Returns the noiseless objective and signed constraint value.
    ///
    /// Constraint convention: violation <= 0 means feasible. For the
    /// constrained bowl, x + y <= 1.15. All other problems are unconstrained.
    pub fn truth(self, x: [f64; 2]) -> (f64, f64) {
        let dx = x[0];
        let dy = x[1];
        let objective = match self.id {
            ProblemId::SmoothBowl => {
                1.0 - 2.0 * ((dx - 0.25).powi(2) + (dy - 0.25).powi(2))
            }
            ProblemId::Multimodal => {
                let global = gaussian_peak(x, [0.22, 0.72], 0.055);
                let local = 0.92 * gaussian_peak(x, [0.78, 0.20], 0.09);
                global.max(local)
            }
            ProblemId::NarrowPeak => gaussian_peak(x, [0.83, 0.17], 0.035),
            ProblemId::ConstrainedBowl => {
                1.0 - 2.0 * ((dx - 0.8).powi(2) + (dy - 0.8).powi(2))
            }
            // Every y in [0,1] is optimal when x=0.7. This is a deliberate
            // non-identifiability control; a policy must not infer a unique y.
            ProblemId::NonIdentifiableRidge => 1.0 - 2.0 * (dx - 0.7).powi(2),
        };
        let violation = if self.id == ProblemId::ConstrainedBowl {
            dx + dy - 1.15
        } else {
            -1.0
        };
        (objective, violation)
    }

    /// Deterministic, heteroskedastic observation keyed by trial seed and
    /// coordinates. Two policies evaluating the same candidate in the same
    /// paired trial receive the same noise realization.
    pub fn observe(self, x: [f64; 2], trial_seed: u64) -> Observation {
        let (truth, violation) = self.truth(x);
        let heteroskedastic_factor = 0.5 + 0.5 * (x[0] + x[1]) / 2.0;
        let sigma = self.noise_scale() * heteroskedastic_factor;
        let noise = deterministic_normal(trial_seed, x) * sigma;
        Observation {
            point: x,
            objective: truth + noise,
            constraint_violation: violation.max(0.0),
            feasible: violation <= 0.0,
            sigma,
        }
    }
}

fn gaussian_peak(x: [f64; 2], center: [f64; 2], width: f64) -> f64 {
    let distance_squared = (x[0] - center[0]).powi(2) + (x[1] - center[1]).powi(2);
    (-distance_squared / (2.0 * width * width)).exp()
}

/// One high-fidelity observation. The objective is noisy; the regret oracle
/// is private to the benchmark runner and is never passed to a policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Observation {
    pub point: [f64; 2],
    pub objective: f64,
    pub constraint_violation: f64,
    pub feasible: bool,
    pub sigma: f64,
}

impl Observation {
    pub fn penalized_reward(self) -> f64 {
        self.objective - 4.0 * self.constraint_violation
    }
}

/// A candidate policy only proposes locations. It never receives the known
/// optimum, truth function, or held-out confirmation observations.
pub trait CandidatePolicy {
    fn name(&self) -> &'static str;

    fn propose(
        &mut self,
        observations: &[Observation],
        proposal_index: usize,
        seed: u64,
    ) -> [f64; 2];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyKind {
    Random,
    Sobol,
    Halton,
    LatinHypercube,
    GaussianProcessUcb,
}

impl PolicyKind {
    pub const ALL: [Self; 4] = [
        Self::Random,
        Self::Sobol,
        Self::Halton,
        Self::LatinHypercube,
        Self::GaussianProcessUcb,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Random => "random",
            Self::Sobol => "sobol",
            Self::Halton => "halton",
            Self::LatinHypercube => "latin_hypercube",
            Self::GaussianProcessUcb => "gp_ucb",
        }
    }
}

/// Paired initial design shared by every policy in a trial.
pub fn initial_design(seed: u64, count: usize) -> Vec<[f64; 2]> {
    (0..count)
        .map(|index| sobol_point((index + 1) as u64, seed))
        .collect()
}

pub struct RandomPolicy {
    rng: SplitMix64,
}

impl RandomPolicy {
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SplitMix64::new(seed),
        }
    }
}

impl CandidatePolicy for RandomPolicy {
    fn name(&self) -> &'static str {
        "random"
    }

    fn propose(
        &mut self,
        _observations: &[Observation],
        _proposal_index: usize,
        _seed: u64,
    ) -> [f64; 2] {
        [self.rng.next_f64(), self.rng.next_f64()]
    }
}

pub struct SobolPolicy;

impl CandidatePolicy for SobolPolicy {
    fn name(&self) -> &'static str {
        "sobol"
    }

    fn propose(
        &mut self,
        _observations: &[Observation],
        proposal_index: usize,
        seed: u64,
    ) -> [f64; 2] {
        sobol_point((proposal_index + INITIAL_DESIGN_SIZE + 1) as u64, seed)
    }
}

pub struct HaltonPolicy;

impl CandidatePolicy for HaltonPolicy {
    fn name(&self) -> &'static str {
        "halton"
    }

    fn propose(
        &mut self,
        _observations: &[Observation],
        proposal_index: usize,
        seed: u64,
    ) -> [f64; 2] {
        let shift_x = unit_from_seed(seed ^ 0xa409_3822_299f_31d0);
        let shift_y = unit_from_seed(seed ^ 0x082e_fa98_ec4e_6c89);
        [
            (radical_inverse((proposal_index + INITIAL_DESIGN_SIZE + 1) as u64, 2)
                + shift_x)
                .fract(),
            (radical_inverse((proposal_index + INITIAL_DESIGN_SIZE + 1) as u64, 3)
                + shift_y)
                .fract(),
        ]
    }
}

pub struct LatinHypercubePolicy {
    points: Vec<[f64; 2]>,
}

impl LatinHypercubePolicy {
    pub fn new(seed: u64, budget_after_initial: usize) -> Self {
        let count = budget_after_initial.max(1);
        let mut rng = SplitMix64::new(seed ^ 0x4528_21e6_38d0_1377);
        let mut x_strata: Vec<usize> = (0..count).collect();
        let mut y_strata: Vec<usize> = (0..count).collect();
        shuffle(&mut x_strata, &mut rng);
        shuffle(&mut y_strata, &mut rng);
        let points = (0..count)
            .map(|index| {
                [
                    (x_strata[index] as f64 + rng.next_f64()) / count as f64,
                    (y_strata[index] as f64 + rng.next_f64()) / count as f64,
                ]
            })
            .collect();
        Self { points }
    }
}

impl CandidatePolicy for LatinHypercubePolicy {
    fn name(&self) -> &'static str {
        "latin_hypercube"
    }

    fn propose(
        &mut self,
        _observations: &[Observation],
        proposal_index: usize,
        _seed: u64,
    ) -> [f64; 2] {
        self.points[proposal_index % self.points.len()]
    }
}

pub struct GaussianProcessUcbPolicy {
    candidate_pool: Vec<[f64; 2]>,
}

impl GaussianProcessUcbPolicy {
    pub fn new(seed: u64, candidate_count: usize) -> Self {
        let shift_x = unit_from_seed(seed ^ 0x299f_31d0_082e_fa98);
        let shift_y = unit_from_seed(seed ^ 0xec4e_6c89_4528_21e6);
        let candidate_pool = (0..candidate_count.max(1))
            .map(|index| {
                [
                    (radical_inverse((index + 1) as u64, 2) + shift_x).fract(),
                    (radical_inverse((index + 1) as u64, 3) + shift_y).fract(),
                ]
            })
            .collect();
        Self { candidate_pool }
    }
}

impl CandidatePolicy for GaussianProcessUcbPolicy {
    fn name(&self) -> &'static str {
        "gp_ucb"
    }

    fn propose(
        &mut self,
        observations: &[Observation],
        proposal_index: usize,
        _seed: u64,
    ) -> [f64; 2] {
        let Some(model) = GaussianProcess::fit(observations) else {
            return self.candidate_pool[proposal_index % self.candidate_pool.len()];
        };

        let mut best_point = self.candidate_pool[0];
        let mut best_score = f64::NEG_INFINITY;
        for point in &self.candidate_pool {
            if observations
                .iter()
                .any(|observation| squared_distance(observation.point, *point) < 1.0e-12)
            {
                continue;
            }
            let (mean, variance) = model.predict(*point);
            let score = mean + GP_UCB_BETA * variance.sqrt();
            if score > best_score {
                best_score = score;
                best_point = *point;
            }
        }
        best_point
    }
}

pub fn make_policy(kind: PolicyKind, seed: u64, budget: usize) -> Box<dyn CandidatePolicy> {
    match kind {
        PolicyKind::Random => Box::new(RandomPolicy::new(seed ^ 0x9e37_79b9_7f4a_7c15)),
        PolicyKind::Sobol => Box::new(SobolPolicy),
        PolicyKind::Halton => Box::new(HaltonPolicy),
        PolicyKind::LatinHypercube => {
            Box::new(LatinHypercubePolicy::new(seed, budget.saturating_sub(INITIAL_DESIGN_SIZE)))
        }
        PolicyKind::GaussianProcessUcb => {
            Box::new(GaussianProcessUcbPolicy::new(seed, GP_CANDIDATE_COUNT))
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrialResult {
    pub problem: ProblemId,
    pub policy: PolicyKind,
    pub seed: u64,
    pub budget: usize,
    pub initial_design_size: usize,
    pub observations: Vec<Observation>,
    pub best_feasible_observed: Option<f64>,
    pub best_feasible_truth: Option<f64>,
    pub final_regret: f64,
    pub infeasible_calls: usize,
    pub elapsed_nanos: u128,
}

impl TrialResult {
    pub fn high_fidelity_calls(&self) -> usize {
        self.observations.len()
    }
}

/// Run one deterministic, budget-matched synthetic trial.
///
/// initial_design_size is identical across all policies. Every observation,
/// including infeasible observations, consumes one evaluation from budget.
pub fn run_trial(
    problem: SyntheticProblem,
    policy_kind: PolicyKind,
    seed: u64,
    budget: usize,
    initial_design_size: usize,
) -> TrialResult {
    assert!(budget > 0, "evaluation budget must be positive");
    assert!(
        initial_design_size <= budget,
        "initial design cannot exceed the total budget"
    );

    let started = Instant::now();
    let design = initial_design(seed, initial_design_size);
    let mut observations: Vec<Observation> = design
        .into_iter()
        .map(|point| problem.observe(point, seed))
        .collect();
    let mut policy = make_policy(policy_kind, seed, budget);

    while observations.len() < budget {
        let proposal_index = observations.len() - initial_design_size;
        let point = policy.propose(&observations, proposal_index, seed);
        let observation = problem.observe(point, seed);
        observations.push(observation);
    }

    let best_feasible_observed = observations
        .iter()
        .filter(|observation| observation.feasible)
        .map(|observation| observation.objective)
        .reduce(f64::max);
    let best_feasible_truth = observations
        .iter()
        .filter(|observation| observation.feasible)
        .map(|observation| problem.truth(observation.point).0)
        .reduce(f64::max);
    let final_regret = best_feasible_truth
        .map(|best| (problem.optimum_value() - best).max(0.0))
        .unwrap_or(problem.optimum_value());
    let infeasible_calls = observations
        .iter()
        .filter(|observation| !observation.feasible)
        .count();

    TrialResult {
        problem: problem.id,
        policy: policy_kind,
        seed,
        budget,
        initial_design_size,
        observations,
        best_feasible_observed,
        best_feasible_truth,
        final_regret,
        infeasible_calls,
        elapsed_nanos: started.elapsed().as_nanos(),
    }
}

/// Paired campaign over problem families and seeds.
pub fn run_campaign(
    seeds: &[u64],
    budget: usize,
    initial_design_size: usize,
) -> Vec<TrialResult> {
    let mut results = Vec::new();
    for problem_id in ProblemId::ALL {
        let problem = SyntheticProblem::new(problem_id);
        for &seed in seeds {
            for policy in PolicyKind::ALL {
                results.push(run_trial(
                    problem,
                    policy,
                    seed,
                    budget,
                    initial_design_size,
                ));
            }
        }
    }
    results
}

#[derive(Clone, Debug, PartialEq)]
pub struct SummaryStats {
    pub problem: ProblemId,
    pub policy: PolicyKind,
    pub trials: usize,
    pub evaluations_per_trial: usize,
    pub mean_regret: f64,
    pub median_regret: f64,
    pub p90_regret: f64,
    pub mean_infeasible_calls: f64,
    pub mean_elapsed_nanos: f64,
}

pub fn summarize(results: &[TrialResult]) -> Vec<SummaryStats> {
    let mut summaries = Vec::new();
    for problem in ProblemId::ALL {
        for policy in PolicyKind::ALL {
            let mut group: Vec<&TrialResult> = results
                .iter()
                .filter(|result| result.problem == problem && result.policy == policy)
                .collect();
            if group.is_empty() {
                continue;
            }
            group.sort_by(|left, right| left.final_regret.total_cmp(&right.final_regret));
            let count = group.len();
            let mean_regret =
                group.iter().map(|result| result.final_regret).sum::<f64>() / count as f64;
            let mean_infeasible_calls = group
                .iter()
                .map(|result| result.infeasible_calls as f64)
                .sum::<f64>()
                / count as f64;
            let mean_elapsed_nanos = group
                .iter()
                .map(|result| result.elapsed_nanos as f64)
                .sum::<f64>()
                / count as f64;
            let median_regret = group[count / 2].final_regret;
            let p90_index = ((count as f64 * 0.9).ceil() as usize)
                .saturating_sub(1)
                .min(count - 1);
            summaries.push(SummaryStats {
                problem,
                policy,
                trials: count,
                evaluations_per_trial: group[0].budget,
                mean_regret,
                median_regret,
                p90_regret: group[p90_index].final_regret,
                mean_infeasible_calls,
                mean_elapsed_nanos,
            });
        }
    }
    summaries
}

/// Emit stable machine-readable per-trial receipts. The source SHA is supplied
/// by the caller because a compiled binary cannot reliably infer its Git tree.
pub fn render_receipts(source_sha: &str, results: &[TrialResult]) -> String {
    let mut output = String::new();
    output.push_str(
        "EXPERIMENT_RECEIPT,v1,source_sha,problem,policy,seed,budget,initial_design,calls,best_feasible_observed,best_feasible_truth,regret,infeasible_calls,elapsed_nanos\n",
    );
    for result in results {
        let observed = result
            .best_feasible_observed
            .map(|value| format!("{value:.12e}"))
            .unwrap_or_else(|| "none".to_string());
        let truth = result
            .best_feasible_truth
            .map(|value| format!("{value:.12e}"))
            .unwrap_or_else(|| "none".to_string());
        output.push_str(&format!(
            "EXPERIMENT_RECEIPT,v1,{source_sha},{},{},{},{},{},{},{observed},{truth},{:.12e},{},{}\n",
            result.problem.as_str(),
            result.policy.as_str(),
            result.seed,
            result.budget,
            result.initial_design_size,
            result.high_fidelity_calls(),
            result.final_regret,
            result.infeasible_calls,
            result.elapsed_nanos
        ));
    }
    output
}

struct GaussianProcess {
    lower_cholesky: Vec<Vec<f64>>,
    alpha: Vec<f64>,
    points: Vec<[f64; 2]>,
}

impl GaussianProcess {
    fn fit(observations: &[Observation]) -> Option<Self> {
        if observations.len() < 2 {
            return None;
        }
        let points: Vec<[f64; 2]> = observations.iter().map(|item| item.point).collect();
        let values: Vec<f64> = observations
            .iter()
            .map(|item| item.penalized_reward())
            .collect();
        let count = points.len();
        let mut covariance = vec![vec![0.0; count]; count];
        for row in 0..count {
            for column in 0..count {
                covariance[row][column] = rbf_kernel(points[row], points[column]);
            }
            covariance[row][row] += GP_NOISE_VARIANCE;
        }
        let lower_cholesky = cholesky(&covariance)?;
        let alpha = cholesky_solve(&lower_cholesky, &values)?;
        Some(Self {
            lower_cholesky,
            alpha,
            points,
        })
    }

    fn predict(&self, point: [f64; 2]) -> (f64, f64) {
        let kernel: Vec<f64> = self
            .points
            .iter()
            .map(|training| rbf_kernel(*training, point))
            .collect();
        let mean = dot(&kernel, &self.alpha);
        let projected = forward_substitute(&self.lower_cholesky, &kernel)
            .unwrap_or_else(|| vec![0.0; kernel.len()]);
        let variance = (1.0 - dot(&projected, &projected)).max(0.0);
        (mean, variance)
    }
}

fn rbf_kernel(left: [f64; 2], right: [f64; 2]) -> f64 {
    let distance_squared = squared_distance(left, right);
    (-0.5 * distance_squared / GP_LENGTH_SCALE.powi(2)).exp()
}

fn cholesky(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let count = matrix.len();
    let mut lower = vec![vec![0.0; count]; count];
    for row in 0..count {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for index in 0..column {
                value -= lower[row][index] * lower[column][index];
            }
            if row == column {
                if !value.is_finite() || value <= 1.0e-14 {
                    return None;
                }
                lower[row][column] = value.sqrt();
            } else {
                let diagonal = lower[column][column];
                if diagonal <= 0.0 || !diagonal.is_finite() {
                    return None;
                }
                lower[row][column] = value / diagonal;
            }
        }
    }
    Some(lower)
}

fn cholesky_solve(lower: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let intermediate = forward_substitute(lower, rhs)?;
    let count = rhs.len();
    let mut solution = vec![0.0; count];
    for row in (0..count).rev() {
        let mut value = intermediate[row];
        for column in (row + 1)..count {
            value -= lower[column][row] * solution[column];
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return None;
        }
    }
    Some(solution)
}

fn forward_substitute(lower: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    if lower.len() != rhs.len() {
        return None;
    }
    let mut solution = vec![0.0; rhs.len()];
    for row in 0..rhs.len() {
        let mut value = rhs[row];
        for column in 0..row {
            value -= lower[row][column] * solution[column];
        }
        let diagonal = lower[row][row];
        if diagonal <= 0.0 || !diagonal.is_finite() {
            return None;
        }
        solution[row] = value / diagonal;
        if !solution[row].is_finite() {
            return None;
        }
    }
    Some(solution)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn squared_distance(left: [f64; 2], right: [f64; 2]) -> f64 {
    (left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2)
}

/// Two-dimensional Sobol sequence with a seeded digital XOR shift.
/// Direction numbers use the standard first coordinate and the degree-one
/// primitive-polynomial recurrence for the second coordinate.
fn sobol_point(index: u64, seed: u64) -> [f64; 2] {
    let mut second_directions = [0_u64; 64];
    second_directions[0] = 1_u64 << 63;
    for bit in 1..64 {
        second_directions[bit] =
            second_directions[bit - 1] ^ (second_directions[bit - 1] >> 1);
    }

    let mut gray = index ^ (index >> 1);
    let mut first = 0_u64;
    let mut second = 0_u64;
    let mut bit = 0_usize;
    while gray != 0 && bit < 64 {
        if gray & 1 == 1 {
            first ^= 1_u64 << (63 - bit);
            second ^= second_directions[bit];
        }
        gray >>= 1;
        bit += 1;
    }

    let mut scramble = SplitMix64::new(seed ^ 0xbe54_66cf_34e9_0c6c);
    first ^= scramble.next_u64();
    second ^= scramble.next_u64();
    let scale = 1.0 / ((1_u64 << 53) as f64);
    [
        (first >> 11) as f64 * scale,
        (second >> 11) as f64 * scale,
    ]
}

fn radical_inverse(mut index: u64, base: u64) -> f64 {
    let inverse_base = 1.0 / base as f64;
    let mut factor = inverse_base;
    let mut result = 0.0;
    while index > 0 {
        result += (index % base) as f64 * factor;
        index /= base;
        factor *= inverse_base;
    }
    result
}

fn deterministic_normal(seed: u64, point: [f64; 2]) -> f64 {
    let mut rng = SplitMix64::new(
        seed
            ^ point[0].to_bits().rotate_left(17)
            ^ point[1].to_bits().rotate_left(41)
            ^ 0x9e37_79b9_7f4a_7c15,
    );
    let first = rng.next_f64().max(f64::MIN_POSITIVE);
    let second = rng.next_f64();
    (-2.0 * first.ln()).sqrt() * (TAU * second).cos()
}

fn unit_from_seed(seed: u64) -> f64 {
    SplitMix64::new(seed).next_f64()
}

#[derive(Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1_u64 << 53) as f64)
    }

    fn range(&mut self, upper_exclusive: usize) -> usize {
        if upper_exclusive <= 1 {
            return 0;
        }
        (self.next_u64() % upper_exclusive as u64) as usize
    }
}

fn shuffle(values: &mut [usize], rng: &mut SplitMix64) {
    for index in (1..values.len()).rev() {
        let other = rng.range(index + 1);
        values.swap(index, other);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CandidatePolicy, GaussianProcessUcbPolicy, PolicyKind, ProblemId, SyntheticProblem,
        initial_design, make_policy, run_campaign, run_trial, summarize,
    };

    #[test]
    fn all_problems_have_the_declared_oracle_optimum() {
        for problem_id in ProblemId::ALL {
            let problem = SyntheticProblem::new(problem_id);
            let point = problem.id.optimum_point();
            let (objective, violation) = problem.truth(point);
            assert!((objective - problem.optimum_value()).abs() < 1.0e-10);
            assert!(violation <= 0.0);
        }
    }

    #[test]
    fn sobol_policy_generates_distinct_in_domain_points() {
        let mut policy = super::SobolPolicy;
        let points: Vec<_> = (0..32)
            .map(|index| policy.propose(&[], index, 77))
            .collect();
        assert!(points.iter().all(|point| {
            (0.0..1.0).contains(&point[0]) && (0.0..1.0).contains(&point[1])
        }));
        let distinct: std::collections::BTreeSet<_> = points
            .iter()
            .map(|point| (point[0].to_bits(), point[1].to_bits()))
            .collect();
        assert_eq!(distinct.len(), points.len());
    }

    #[test]
    fn initial_design_is_deterministic_and_in_bounds() {
        let first = initial_design(42, 16);
        let second = initial_design(42, 16);
        assert_eq!(first, second);
        assert!(first.iter().all(|point| {
            (0.0..1.0).contains(&point[0]) && (0.0..1.0).contains(&point[1])
        }));
    }

    #[test]
    fn every_policy_obeys_the_same_high_fidelity_budget() {
        for problem_id in ProblemId::ALL {
            for policy in PolicyKind::ALL {
                let result = run_trial(SyntheticProblem::new(problem_id), policy, 7, 32, 8);
                assert_eq!(result.high_fidelity_calls(), 32);
                assert_eq!(
                    result.infeasible_calls
                        + result.observations.iter().filter(|item| item.feasible).count(),
                    32
                );
                assert!(result.final_regret.is_finite());
            }
        }
    }

    #[test]
    fn same_policy_and_trial_seed_reproduce_proposals_and_observations() {
        let first = run_trial(
            SyntheticProblem::new(ProblemId::Multimodal),
            PolicyKind::GaussianProcessUcb,
            1234,
            24,
            8,
        );
        let second = run_trial(
            SyntheticProblem::new(ProblemId::Multimodal),
            PolicyKind::GaussianProcessUcb,
            1234,
            24,
            8,
        );
        assert_eq!(first.observations, second.observations);
        assert_eq!(first.final_regret, second.final_regret);
    }

    #[test]
    fn paired_initial_design_is_identical_across_policies() {
        let seed = 991;
        let random = run_trial(
            SyntheticProblem::new(ProblemId::NarrowPeak),
            PolicyKind::Random,
            seed,
            24,
            8,
        );
        for policy in PolicyKind::ALL {
            let result = run_trial(
                SyntheticProblem::new(ProblemId::NarrowPeak),
                policy,
                seed,
                24,
                8,
            );
            assert_eq!(&result.observations[..8], &random.observations[..8]);
        }
    }

    #[test]
    fn gp_ucb_returns_finite_in_domain_candidates() {
        let problem = SyntheticProblem::new(ProblemId::SmoothBowl);
        let observations: Vec<_> = initial_design(55, 8)
            .into_iter()
            .map(|point| problem.observe(point, 55))
            .collect();
        let mut policy = GaussianProcessUcbPolicy::new(55, 128);
        for index in 0..16 {
            let point = policy.propose(&observations, index, 55);
            assert!((0.0..=1.0).contains(&point[0]));
            assert!((0.0..=1.0).contains(&point[1]));
        }
    }

    #[test]
    fn campaign_summary_covers_each_problem_and_policy() {
        let seeds = [1, 2, 3];
        let results = run_campaign(&seeds, 16, 8);
        let summaries = summarize(&results);
        assert_eq!(results.len(), ProblemId::ALL.len() * seeds.len() * PolicyKind::ALL.len());
        assert_eq!(summaries.len(), ProblemId::ALL.len() * PolicyKind::ALL.len());
        assert!(summaries.iter().all(|summary| summary.trials == seeds.len()));
    }

    #[test]
    fn factory_exposes_the_named_conventional_baselines() {
        for kind in PolicyKind::ALL {
            let policy = make_policy(kind, 1, 16);
            assert_eq!(policy.name(), kind.as_str());
        }
    }
}
