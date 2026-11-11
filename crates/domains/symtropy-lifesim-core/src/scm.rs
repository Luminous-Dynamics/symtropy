// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent CPU reference for a consensus-coupled, adaptive-noise swarm search model.
//!
//! Independently authored from Nitti et al. (2025), Nature Communications 16, 6572:
//! https://doi.org/10.1038/s41467-025-61985-7
//!
//! The accessible paper HTML and inspected repository source disagree on the sine argument
//! scale in the social interaction term. Both interpretations are explicit variants here.
//! This is a bounded landscape optimizer reference, not a universal collective-intelligence
//! score, animal model, biological-life detector, or qualified AUV controller.

use std::collections::VecDeque;
use std::error::Error;
use std::f64::consts::{PI, TAU};
use std::fmt;

/// The two social-interaction sine scalings that require source reconciliation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmInteractionKernel {
    /// Argument scale rendered in the accessible HTML equation: 2/pi.
    PublishedHtmlEquation,
    /// Argument scale in the inspected GitHub MATLAB source: pi/2.
    RepositoryMatlabSource,
}

impl ScmInteractionKernel {
    pub const fn argument_scale(self) -> f64 {
        match self {
            Self::PublishedHtmlEquation => 2.0 / PI,
            Self::RepositoryMatlabSource => PI / 2.0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::PublishedHtmlEquation => "published_html_equation_sin_2_over_pi",
            Self::RepositoryMatlabSource => "repository_matlab_source_sin_pi_over_2",
        }
    }
}

/// Model configuration. Positions and landscape values are dimensionless and bounded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScmConfig {
    pub dt: f64,
    pub tau_steps: usize,
    pub omega: f64,
    pub sigma_initial: f64,
    pub sigma_limit: f64,
    /// Conical continuation height outside [0,1]^d; source parameter H = -0.5.
    pub barrier_height: f64,
    /// Conical continuation angle in degrees; source parameter is 20 degrees.
    pub barrier_angle_degrees: f64,
    /// Values below this threshold are treated as degenerate numerical quantities.
    pub numerical_epsilon: f64,
    pub kernel: ScmInteractionKernel,
}

impl Default for ScmConfig {
    fn default() -> Self {
        Self {
            dt: 0.1,
            tau_steps: 60,
            omega: 0.2,
            sigma_initial: 0.05,
            sigma_limit: 0.3,
            barrier_height: -0.5,
            barrier_angle_degrees: 20.0,
            numerical_epsilon: 1.0e-12,
            // Explicitly choose the publication-rendered variant, not the repository
            // source variant. The profile requires this choice to be recorded.
            kernel: ScmInteractionKernel::PublishedHtmlEquation,
        }
    }
}

impl ScmConfig {
    fn validate(self) -> Result<(), ScmError> {
        for (name, value) in [
            ("dt", self.dt),
            ("omega", self.omega),
            ("sigma_initial", self.sigma_initial),
            ("sigma_limit", self.sigma_limit),
            ("barrier_height", self.barrier_height),
            ("barrier_angle_degrees", self.barrier_angle_degrees),
            ("numerical_epsilon", self.numerical_epsilon),
        ] {
            if !value.is_finite() {
                return Err(ScmError::InvalidParameter { name, value });
            }
        }
        if self.dt <= 0.0
            || self.omega < 0.0
            || self.sigma_initial < 0.0
            || self.sigma_limit <= 0.0
            || self.sigma_initial > self.sigma_limit
            || self.barrier_angle_degrees <= 0.0
            || self.barrier_angle_degrees >= 90.0
            || self.numerical_epsilon <= 0.0
            || self.tau_steps < 2
            || self.tau_steps % 2 != 0
        {
            return Err(ScmError::InvalidConfiguration);
        }
        Ok(())
    }

    pub const fn tau(self) -> f64 {
        self.dt * self.tau_steps as f64
    }
}

/// Agent state; all agents share the same number of decision dimensions.
#[derive(Debug, Clone, PartialEq)]
pub struct ScmAgent {
    pub position: Vec<f64>,
}

/// A landscape returns a scalar fitness to maximize and its gradient.
///
/// The caller supplies a finite fitness and a gradient of matching dimensionality.
/// Out-of-domain points are handled by the declared conical barrier and do not invoke
/// the landscape implementation.
pub trait ScmLandscape {
    fn fitness_and_gradient(&self, position: &[f64]) -> Result<(f64, Vec<f64>), String>;
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScmError {
    EmptySwarm,
    TooFewAgents(usize),
    EmptyDimension,
    DimensionMismatch { expected: usize, actual: usize },
    InvalidParameter { name: &'static str, value: f64 },
    InvalidConfiguration,
    NonFiniteInitialPosition { agent: usize, dimension: usize },
    LandscapeFailure { agent: usize, reason: String },
    InvalidLandscapeObservation { agent: usize },
    NonFiniteState { agent: usize, dimension: usize },
    TickOverflow,
    EvaluationCounterOverflow,
}

impl fmt::Display for ScmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySwarm => write!(f, "SCM requires at least one agent"),
            Self::TooFewAgents(count) => write!(f, "SCM requires at least two agents, got {count}"),
            Self::EmptyDimension => write!(f, "SCM requires at least one decision dimension"),
            Self::DimensionMismatch { expected, actual } => {
                write!(f, "SCM dimension mismatch: expected {expected}, got {actual}")
            }
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid SCM parameter {name}={value}")
            }
            Self::InvalidConfiguration => write!(f, "invalid SCM configuration"),
            Self::NonFiniteInitialPosition { agent, dimension } => {
                write!(f, "agent {agent} has non-finite initial coordinate {dimension}")
            }
            Self::LandscapeFailure { agent, reason } => {
                write!(f, "landscape evaluation failed for agent {agent}: {reason}")
            }
            Self::InvalidLandscapeObservation { agent } => {
                write!(f, "landscape returned invalid fitness/gradient for agent {agent}")
            }
            Self::NonFiniteState { agent, dimension } => {
                write!(f, "SCM generated a non-finite coordinate for agent {agent}, dimension {dimension}")
            }
            Self::TickOverflow => write!(f, "SCM tick counter overflow"),
            Self::EvaluationCounterOverflow => write!(f, "SCM function-evaluation counter overflow"),
        }
    }
}

impl Error for ScmError {}

#[derive(Debug, Clone, PartialEq)]
struct ScmObservation {
    fitness: f64,
    gradient: Vec<f64>,
    outside_domain: bool,
}

/// One synchronous SCM update report.
#[derive(Debug, Clone, PartialEq)]
pub struct ScmStepReport {
    pub tick: u64,
    pub interaction_kernel: ScmInteractionKernel,
    pub consensus: f64,
    pub sigma_before_update: f64,
    pub sigma_after_update: f64,
    pub centroid: Vec<f64>,
    pub mean_agent_distance: f64,
    pub maximum_pairwise_distance: f64,
    pub mean_gradient_norm: f64,
    pub gradient_degenerate: bool,
    pub swarm_spread_degenerate: bool,
    pub outside_domain_agents: usize,
    pub consensus_window_delta: Option<f64>,
    pub convergence_window_satisfied: bool,
    pub function_evaluations_this_step: usize,
    pub total_function_evaluations: u64,
}

/// Independent CPU reference. Updates are synchronous: every next state is computed from
/// the same previous agent/observation snapshot before any agent position is replaced.
#[derive(Debug, Clone, PartialEq)]
pub struct ScmModel {
    config: ScmConfig,
    agents: Vec<ScmAgent>,
    sigma: f64,
    rng: ScmRng,
    tick: u64,
    total_function_evaluations: u64,
    observations: Option<Vec<ScmObservation>>,
    consensus_history: VecDeque<f64>,
}

impl ScmModel {
    pub fn new(agents: Vec<ScmAgent>, config: ScmConfig, seed: u64) -> Result<Self, ScmError> {
        config.validate()?;
        if agents.is_empty() {
            return Err(ScmError::EmptySwarm);
        }
        if agents.len() < 2 {
            return Err(ScmError::TooFewAgents(agents.len()));
        }
        let dimension = agents[0].position.len();
        if dimension == 0 {
            return Err(ScmError::EmptyDimension);
        }
        for (agent_index, agent) in agents.iter().enumerate() {
            if agent.position.len() != dimension {
                return Err(ScmError::DimensionMismatch {
                    expected: dimension,
                    actual: agent.position.len(),
                });
            }
            for (coordinate, value) in agent.position.iter().enumerate() {
                if !value.is_finite() {
                    return Err(ScmError::NonFiniteInitialPosition {
                        agent: agent_index,
                        dimension: coordinate,
                    });
                }
            }
        }

        Ok(Self {
            config,
            agents,
            sigma: config.sigma_initial,
            rng: ScmRng::new(seed),
            tick: 0,
            total_function_evaluations: 0,
            observations: None,
            consensus_history: VecDeque::with_capacity(config.tau_steps),
        })
    }

    pub const fn config(&self) -> ScmConfig {
        self.config
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn sigma(&self) -> f64 {
        self.sigma
    }

    pub const fn total_function_evaluations(&self) -> u64 {
        self.total_function_evaluations
    }

    pub fn agents(&self) -> &[ScmAgent] {
        &self.agents
    }

    /// Advance one synchronous update and return post-update swarm metrics.
    ///
    /// The first call evaluates the initial state and the successor state. Later calls reuse
    /// the previous observation and evaluate only the newly advanced agent positions.
    pub fn step<L: ScmLandscape>(&mut self, landscape: &L) -> Result<ScmStepReport, ScmError> {
        let next_tick = self.tick.checked_add(1).ok_or(ScmError::TickOverflow)?;
        let mut evaluations_this_step = 0usize;

        if self.observations.is_none() {
            let initial = self.observe_all(landscape)?;
            self.total_function_evaluations = self
                .total_function_evaluations
                .checked_add(initial.len() as u64)
                .ok_or(ScmError::EvaluationCounterOverflow)?;
            evaluations_this_step += initial.len();
            self.observations = Some(initial);
        }

        let previous_positions: Vec<Vec<f64>> = self
            .agents
            .iter()
            .map(|agent| agent.position.clone())
            .collect();
        let observations = self.observations.as_ref().expect("observations initialized");
        let (centroid, distances, max_distance, consensus, lambdas, spread_degenerate) =
            consensus_state(&previous_positions, observations, self.config.numerical_epsilon);

        let gradient_norms: Vec<f64> = observations
            .iter()
            .map(|observation| l2_norm(&observation.gradient))
            .collect();
        let mean_gradient_norm =
            gradient_norms.iter().sum::<f64>() / gradient_norms.len() as f64;
        let gradient_degenerate = !mean_gradient_norm.is_finite()
            || mean_gradient_norm <= self.config.numerical_epsilon;
        // xi = 1/mean(||grad V||) is singular when every gradient is zero.
        // The independent reference uses an explicit finite zero-gain fallback.
        let fitness_gradient_gain = if gradient_degenerate {
            0.0
        } else {
            1.0 / mean_gradient_norm
        };

        let dimension = previous_positions[0].len();
        let kernel_scale = self.config.kernel.argument_scale();
        let noise_scale = (2.0 / PI) * self.sigma;
        let social_scale = 2.0 / PI;
        let fitness_scale = fitness_gradient_gain * 4.0 / (PI * PI);
        let sqrt_dt = self.config.dt.sqrt();
        let mut next_positions = Vec::with_capacity(self.agents.len());

        for agent_index in 0..previous_positions.len() {
            let mut next = vec![0.0; dimension];
            for coordinate in 0..dimension {
                let mut social_sum = 0.0;
                for other_index in 0..previous_positions.len() {
                    if other_index == agent_index {
                        continue;
                    }
                    let delta = previous_positions[agent_index][coordinate]
                        - previous_positions[other_index][coordinate];
                    social_sum += (kernel_scale * delta).sin();
                }
                let social_gradient =
                    -social_sum / (previous_positions.len().saturating_sub(1) as f64);
                let fitness_gradient = observations[agent_index].gradient[coordinate];
                let noise = noise_scale
                    * lambdas[agent_index]
                    * sqrt_dt
                    * self.rng.next_normal();
                next[coordinate] = previous_positions[agent_index][coordinate]
                    + self.config.dt
                        * (social_scale * social_gradient + fitness_scale * fitness_gradient)
                    + noise;
                if !next[coordinate].is_finite() {
                    return Err(ScmError::NonFiniteState {
                        agent: agent_index,
                        dimension: coordinate,
                    });
                }
            }
            next_positions.push(next);
        }

        for (agent, position) in self.agents.iter_mut().zip(next_positions) {
            agent.position = position;
        }

        let updated_observations = self.observe_all(landscape)?;
        evaluations_this_step += updated_observations.len();
        self.total_function_evaluations = self
            .total_function_evaluations
            .checked_add(updated_observations.len() as u64)
            .ok_or(ScmError::EvaluationCounterOverflow)?;
        self.observations = Some(updated_observations.clone());

        let updated_positions: Vec<Vec<f64>> = self
            .agents
            .iter()
            .map(|agent| agent.position.clone())
            .collect();
        let (updated_centroid, updated_distances, updated_max_distance, updated_consensus, _, updated_spread_degenerate) =
            consensus_state(
                &updated_positions,
                &updated_observations,
                self.config.numerical_epsilon,
            );

        self.tick = next_tick;
        self.consensus_history.push_back(updated_consensus);
        while self.consensus_history.len() > self.config.tau_steps {
            self.consensus_history.pop_front();
        }

        let sigma_before_update = self.sigma;
        let mut consensus_window_delta = None;
        let mut convergence_window_satisfied = false;
        let tau_steps = self.config.tau_steps as u64;

        if self.tick % tau_steps == 0 && self.consensus_history.len() == self.config.tau_steps {
            let half = self.config.tau_steps / 2;
            let early = self
                .consensus_history
                .iter()
                .take(half)
                .sum::<f64>()
                * self.config.dt;
            let late = self
                .consensus_history
                .iter()
                .skip(half)
                .sum::<f64>()
                * self.config.dt;
            let delta = late - early;
            consensus_window_delta = Some(delta);

            // Source rule: increase sigma by omega*dt only when consensus flattens/declines.
            // A zero delta is handled explicitly rather than evaluating 0/0.
            if delta < 0.0 {
                self.sigma += self.config.dt * self.config.omega;
                if self.sigma >= self.config.sigma_limit {
                    self.sigma = self.config.sigma_initial;
                }
            }
            let gamma = (1.0 - (self.agents.len() as f64 / 100.0).tanh()) / 20.0 + 0.9;
            convergence_window_satisfied = late + early > gamma * self.config.tau();
        }

        let outside_domain_agents = updated_observations
            .iter()
            .filter(|observation| observation.outside_domain)
            .count();

        Ok(ScmStepReport {
            tick: self.tick,
            interaction_kernel: self.config.kernel,
            consensus: updated_consensus,
            sigma_before_update,
            sigma_after_update: self.sigma,
            centroid: updated_centroid,
            mean_agent_distance: updated_distances.iter().sum::<f64>()
                / updated_distances.len() as f64,
            maximum_pairwise_distance: updated_max_distance,
            mean_gradient_norm,
            gradient_degenerate,
            swarm_spread_degenerate: spread_degenerate || updated_spread_degenerate,
            outside_domain_agents,
            consensus_window_delta,
            convergence_window_satisfied,
            function_evaluations_this_step: evaluations_this_step,
            total_function_evaluations: self.total_function_evaluations,
        })
    }

    pub fn run_steps<L: ScmLandscape>(
        &mut self,
        landscape: &L,
        steps: usize,
    ) -> Result<Vec<ScmStepReport>, ScmError> {
        let mut reports = Vec::with_capacity(steps);
        for _ in 0..steps {
            reports.push(self.step(landscape)?);
        }
        Ok(reports)
    }

    fn observe_all<L: ScmLandscape>(&self, landscape: &L) -> Result<Vec<ScmObservation>, ScmError> {
        let mut observations = Vec::with_capacity(self.agents.len());
        for (agent_index, agent) in self.agents.iter().enumerate() {
            if agent.position.iter().any(|value| !(0.0..=1.0).contains(value)) {
                observations.push(ScmObservation {
                    fitness: 0.0,
                    gradient: conical_barrier_gradient(&agent.position, self.config),
                    outside_domain: true,
                });
                continue;
            }
            let (fitness, gradient) = landscape
                .fitness_and_gradient(&agent.position)
                .map_err(|reason| ScmError::LandscapeFailure {
                    agent: agent_index,
                    reason,
                })?;
            if !fitness.is_finite()
                || gradient.len() != agent.position.len()
                || gradient.iter().any(|value| !value.is_finite())
            {
                return Err(ScmError::InvalidLandscapeObservation { agent: agent_index });
            }
            observations.push(ScmObservation {
                fitness,
                gradient,
                outside_domain: false,
            });
        }
        Ok(observations)
    }
}

fn conical_barrier_gradient(position: &[f64], config: ScmConfig) -> Vec<f64> {
    let delta: Vec<f64> = position.iter().map(|value| value - 0.5).collect();
    let norm = l2_norm(&delta);
    if norm <= config.numerical_epsilon {
        return vec![0.0; position.len()];
    }
    let angle = config.barrier_angle_degrees.to_radians();
    let magnitude = config.barrier_height * (angle.cos() / angle.sin());
    delta.into_iter().map(|value| magnitude * value / norm).collect()
}

fn consensus_state(
    positions: &[Vec<f64>],
    observations: &[ScmObservation],
    epsilon: f64,
) -> (Vec<f64>, Vec<f64>, f64, f64, Vec<f64>, bool) {
    let count = positions.len();
    let dimension = positions[0].len();
    let minimum_fitness = observations
        .iter()
        .map(|observation| observation.fitness)
        .fold(f64::INFINITY, f64::min);
    let weights: Vec<f64> = observations
        .iter()
        .map(|observation| (observation.fitness - minimum_fitness).max(0.0))
        .collect();
    let weight_sum = weights.iter().sum::<f64>();

    // Fitness-shifted centroid; equal/degenerate fitness uses an unweighted finite fallback.
    let mut centroid = vec![0.0; dimension];
    if weight_sum <= epsilon {
        for position in positions {
            for coordinate in 0..dimension {
                centroid[coordinate] += position[coordinate] / count as f64;
            }
        }
    } else {
        for (position, weight) in positions.iter().zip(weights.iter()) {
            for coordinate in 0..dimension {
                centroid[coordinate] += position[coordinate] * weight / weight_sum;
            }
        }
    }

    let distances: Vec<f64> = positions
        .iter()
        .map(|position| {
            let difference: Vec<f64> = position
                .iter()
                .zip(centroid.iter())
                .map(|(left, right)| left - right)
                .collect();
            l2_norm(&difference)
        })
        .collect();

    let mut maximum_pairwise_distance: f64 = 0.0;
    for left in 0..count {
        for right in (left + 1)..count {
            let difference: Vec<f64> = positions[left]
                .iter()
                .zip(positions[right].iter())
                .map(|(a, b)| a - b)
                .collect();
            maximum_pairwise_distance =
                maximum_pairwise_distance.max(l2_norm(&difference));
        }
    }

    let raw_consensus = 1.0 - (2.0 / count as f64) * distances.iter().sum::<f64>();
    let consensus = raw_consensus.max(0.05);
    let spread_degenerate = maximum_pairwise_distance <= epsilon;
    let lambdas = if spread_degenerate {
        vec![0.0; count]
    } else {
        distances
            .iter()
            .map(|distance| distance / (maximum_pairwise_distance * consensus))
            .collect()
    };

    (
        centroid,
        distances,
        maximum_pairwise_distance,
        consensus,
        lambdas,
        spread_degenerate,
    )
}

fn l2_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

/// Deterministic xorshift64 + Box-Muller Gaussian stream. This is not MATLAB's RNG and
/// supports independent Rust replay/statistical comparison, not bitwise MATLAB parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScmRng {
    state: u64,
}

impl ScmRng {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn next_unit(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) / ((1u64 << 53) as f64)
    }

    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_unit().max(f64::MIN_POSITIVE);
        let u2 = self.next_unit();
        (-2.0 * u1.ln()).sqrt() * (TAU * u2).cos()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Quadratic;

    impl ScmLandscape for Quadratic {
        fn fitness_and_gradient(&self, position: &[f64]) -> Result<(f64, Vec<f64>), String> {
            let delta: Vec<f64> = position.iter().map(|value| value - 0.6).collect();
            let fitness = -delta.iter().map(|value| value * value).sum::<f64>();
            let gradient = delta.iter().map(|value| -2.0 * value).collect();
            Ok((fitness, gradient))
        }
    }

    struct Flat;

    impl ScmLandscape for Flat {
        fn fitness_and_gradient(&self, position: &[f64]) -> Result<(f64, Vec<f64>), String> {
            Ok((0.5, vec![0.0; position.len()]))
        }
    }

    fn small_agents() -> Vec<ScmAgent> {
        vec![
            ScmAgent { position: vec![0.2, 0.2] },
            ScmAgent { position: vec![0.8, 0.2] },
            ScmAgent { position: vec![0.5, 0.8] },
        ]
    }

    #[test]
    fn paper_and_repository_kernels_are_explicit_and_distinct() {
        assert!((ScmInteractionKernel::PublishedHtmlEquation.argument_scale() - 2.0 / PI).abs() < 1.0e-15);
        assert!((ScmInteractionKernel::RepositoryMatlabSource.argument_scale() - PI / 2.0).abs() < 1.0e-15);
        assert_ne!(
            ScmInteractionKernel::PublishedHtmlEquation.argument_scale(),
            ScmInteractionKernel::RepositoryMatlabSource.argument_scale()
        );
    }

    #[test]
    fn deterministic_seed_replays_the_rust_reference() {
        let config = ScmConfig { tau_steps: 4, ..ScmConfig::default() };
        let mut left = ScmModel::new(small_agents(), config, 42).unwrap();
        let mut right = ScmModel::new(small_agents(), config, 42).unwrap();
        let left_reports = left.run_steps(&Quadratic, 20).unwrap();
        let right_reports = right.run_steps(&Quadratic, 20).unwrap();
        assert_eq!(left, right);
        assert_eq!(left_reports, right_reports);
    }

    #[test]
    fn coincident_agents_have_finite_zero_noise_weights() {
        let agents = vec![
            ScmAgent { position: vec![0.5, 0.5] },
            ScmAgent { position: vec![0.5, 0.5] },
        ];
        let mut model = ScmModel::new(agents, ScmConfig::default(), 7).unwrap();
        let report = model.step(&Quadratic).unwrap();
        assert!(report.swarm_spread_degenerate);
        assert!(report.consensus.is_finite());
        assert!(model.agents().iter().all(|agent| agent.position.iter().all(|x| x.is_finite())));
    }

    #[test]
    fn zero_mean_gradient_uses_finite_gain_fallback() {
        let mut model = ScmModel::new(small_agents(), ScmConfig::default(), 11).unwrap();
        let report = model.step(&Flat).unwrap();
        assert!(report.gradient_degenerate);
        assert_eq!(report.mean_gradient_norm, 0.0);
        assert!(report.consensus.is_finite());
    }

    #[test]
    fn flat_consensus_window_does_not_generate_nan_noise_adaptation() {
        let agents = vec![
            ScmAgent { position: vec![0.5, 0.5] },
            ScmAgent { position: vec![0.5, 0.5] },
        ];
        let config = ScmConfig { tau_steps: 4, ..ScmConfig::default() };
        let mut model = ScmModel::new(agents, config, 19).unwrap();
        let reports = model.run_steps(&Flat, 8).unwrap();
        assert!(reports.iter().all(|report| report.sigma_after_update.is_finite()));
        assert!((model.sigma() - config.sigma_initial).abs() < 1.0e-15);
    }

    #[test]
    fn out_of_domain_agents_use_a_finite_conical_barrier_observation() {
        let agents = vec![
            ScmAgent { position: vec![1.1, 0.5] },
            ScmAgent { position: vec![1.2, 0.5] },
        ];
        let mut model = ScmModel::new(agents, ScmConfig::default(), 23).unwrap();
        let report = model.step(&Quadratic).unwrap();
        assert_eq!(report.outside_domain_agents, 2);
        assert!(model.agents().iter().all(|agent| agent.position.iter().all(|x| x.is_finite())));
    }

    #[test]
    fn function_evaluations_are_explicitly_counted() {
        let mut model = ScmModel::new(small_agents(), ScmConfig::default(), 29).unwrap();
        let first = model.step(&Quadratic).unwrap();
        assert_eq!(first.function_evaluations_this_step, 6);
        assert_eq!(first.total_function_evaluations, 6);
        let second = model.step(&Quadratic).unwrap();
        assert_eq!(second.function_evaluations_this_step, 3);
        assert_eq!(second.total_function_evaluations, 9);
    }

    #[test]
    fn rejects_one_agent_and_odd_consensus_window() {
        assert!(matches!(
            ScmModel::new(
                vec![ScmAgent { position: vec![0.1] }],
                ScmConfig::default(),
                1
            ),
            Err(ScmError::TooFewAgents(1))
        ));
        assert!(matches!(
            ScmModel::new(
                small_agents(),
                ScmConfig { tau_steps: 3, ..ScmConfig::default() },
                1
            ),
            Err(ScmError::InvalidConfiguration)
        ));
    }
}
