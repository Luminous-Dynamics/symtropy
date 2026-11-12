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

/// Social-interaction kernel shared by the published discrete update and the
/// pinned MATLAB reference implementation (both use sin((pi/2) * delta_x)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmInteractionKernel {
    PublishedEquationAndMatlabSource,
}

impl ScmInteractionKernel {
    pub const fn argument_scale(self) -> f64 {
        match self {
            Self::PublishedEquationAndMatlabSource => PI / 2.0,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::PublishedEquationAndMatlabSource => {
                "published_eq10_and_matlab_tstep_sin_pi_over_2"
            }
        }
    }
}

/// Consensus-window discretization. The paper describes two equal half-intervals;
/// the inspected MATLAB loop uses inclusive array endpoints that produce one extra
/// older sample and schedules the first update at tick tau_steps + 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScmConsensusWindow {
    /// Equal, disjoint half-windows over exactly tau_steps consensus values.
    PublishedEqualHalfIntervals,
    /// Reproduces the inspected MATLAB replication script's inclusive index endpoints:
    /// early window has tau_steps/2 + 1 values, late window tau_steps/2 values.
    RepositoryMatlabInclusiveEndpoints,
}

/// Communication topology for the consensus/search controller.
///
/// `CompleteGraph` is the paper/source baseline. `Ring`, `MetricRadius`, and
/// `NoCommunication` are OLI extensions that constrain or disable direct social messages.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScmCommunicationTopology {
    CompleteGraph,
    Ring,
    MetricRadius {
        radius: f64,
    },
    /// Explicitly empty adjacency. Unlike a zero-radius metric graph, coincident
    /// agents never become neighbours under this topology.
    NoCommunication,
}

impl ScmCommunicationTopology {
    pub const fn label(self) -> &'static str {
        match self {
            Self::CompleteGraph => "complete_graph",
            Self::Ring => "ring",
            Self::MetricRadius { .. } => "metric_radius",
            Self::NoCommunication => "no_communication",
        }
    }

    fn validate(self) -> Result<(), ScmError> {
        if let Self::MetricRadius { radius } = self
            && (!radius.is_finite() || radius < 0.0)
        {
            return Err(ScmError::InvalidParameter {
                name: "communication_radius",
                value: radius,
            });
        }
        Ok(())
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
    pub consensus_window: ScmConsensusWindow,
    pub communication_topology: ScmCommunicationTopology,
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
            kernel: ScmInteractionKernel::PublishedEquationAndMatlabSource,
            consensus_window: ScmConsensusWindow::PublishedEqualHalfIntervals,
            communication_topology: ScmCommunicationTopology::CompleteGraph,
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
        self.communication_topology.validate()?;
        if self.dt <= 0.0
            || self.omega < 0.0
            || self.sigma_initial < 0.0
            || self.sigma_limit <= 0.0
            || self.sigma_initial > self.sigma_limit
            || self.barrier_angle_degrees <= 0.0
            || self.barrier_angle_degrees >= 90.0
            || self.numerical_epsilon <= 0.0
            || self.tau_steps < 2
            || !self.tau_steps.is_multiple_of(2)
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

/// A landscape returns a dimensionless fitness in [0, 1] to maximize and its gradient.
///
/// The caller supplies a bounded fitness and a finite gradient of matching dimensionality.
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
                write!(
                    f,
                    "SCM dimension mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidParameter { name, value } => {
                write!(f, "invalid SCM parameter {name}={value}")
            }
            Self::InvalidConfiguration => write!(f, "invalid SCM configuration"),
            Self::NonFiniteInitialPosition { agent, dimension } => {
                write!(
                    f,
                    "agent {agent} has non-finite initial coordinate {dimension}"
                )
            }
            Self::LandscapeFailure { agent, reason } => {
                write!(f, "landscape evaluation failed for agent {agent}: {reason}")
            }
            Self::InvalidLandscapeObservation { agent } => {
                write!(
                    f,
                    "landscape returned invalid fitness/gradient for agent {agent}"
                )
            }
            Self::NonFiniteState { agent, dimension } => {
                write!(
                    f,
                    "SCM generated a non-finite coordinate for agent {agent}, dimension {dimension}"
                )
            }
            Self::TickOverflow => write!(f, "SCM tick counter overflow"),
            Self::EvaluationCounterOverflow => {
                write!(f, "SCM function-evaluation counter overflow")
            }
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

/// A failed batch retains how many per-agent evaluation attempts were already made,
/// including the call that failed. This supports cost accounting without committing
/// partially updated agent state.
#[derive(Debug, Clone, PartialEq)]
struct ScmObservationFailure {
    error: ScmError,
    attempted_agent_evaluations: usize,
}

/// One synchronous SCM update report.
#[derive(Debug, Clone, PartialEq)]
pub struct ScmStepReport {
    pub tick: u64,
    pub interaction_kernel: ScmInteractionKernel,
    pub consensus_window: ScmConsensusWindow,
    pub communication_topology: ScmCommunicationTopology,
    /// Best objective fitness among the current agents (dimensionless, [0, 1]).
    pub best_agent_fitness: f64,
    /// Best objective fitness observed at any acquired position up through this tick.
    pub best_fitness_seen: f64,
    /// Index of the best-performing current agent; ties are resolved by lowest index.
    pub best_agent_index: usize,
    /// Mean objective fitness across current agents.
    pub mean_agent_fitness: f64,
    /// Population variance of current-agent objective fitness values.
    pub agent_fitness_variance: f64,
    /// Consensus is a separate social-state observable, not task performance.
    pub consensus: f64,
    pub sigma_before_update: f64,
    pub sigma_after_update: f64,
    pub centroid: Vec<f64>,
    pub mean_agent_distance: f64,
    pub maximum_pairwise_distance: f64,
    pub mean_gradient_norm_before_update: f64,
    pub gradient_degenerate: bool,
    pub gradient_degenerate_agents: usize,
    pub swarm_spread_degenerate: bool,
    pub locally_degenerate_spread_agents: usize,
    pub mean_communicating_neighbors: f64,
    pub isolated_agents: usize,
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
    best_fitness_seen: Option<f64>,
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
            best_fitness_seen: None,
            consensus_history: VecDeque::with_capacity(config.tau_steps + 1),
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

    pub const fn best_fitness_seen(&self) -> Option<f64> {
        self.best_fitness_seen
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
        let mut initial_evaluations_this_call = 0usize;
        if self.observations.is_none() {
            // A completed initial observation is an acquired measurement. Retain its cost
            // and cache even if a later successor observation fails.
            let initial = match self.observe_agents(&self.agents, landscape) {
                Ok(observations) => observations,
                Err(failure) => {
                    self.record_evaluation_attempts(failure.attempted_agent_evaluations)?;
                    return Err(failure.error);
                }
            };
            let initial_count = self.agents.len();
            self.record_evaluation_attempts(initial_count)?;
            initial_evaluations_this_call = initial_count;
            self.best_fitness_seen = initial
                .iter()
                .map(|observation| observation.fitness)
                .max_by(f64::total_cmp);
            self.observations = Some(initial);
        }
        let current_observations = self
            .observations
            .as_ref()
            .expect("initial observations are cached")
            .clone();

        let previous_positions: Vec<Vec<f64>> = self
            .agents
            .iter()
            .map(|agent| agent.position.clone())
            .collect();
        let communication_neighbors =
            build_communication_neighbors(&previous_positions, self.config.communication_topology);
        let current_network = communication_consensus_state(
            &previous_positions,
            &current_observations,
            &communication_neighbors,
            self.config.numerical_epsilon,
        );

        let gradient_norms: Vec<f64> = current_observations
            .iter()
            .map(|observation| l2_norm(&observation.gradient))
            .collect();
        let mean_gradient_norm = gradient_norms
            .iter()
            .map(|value| *value / gradient_norms.len() as f64)
            .sum::<f64>();
        let mut gradient_gain_by_agent = vec![0.0; previous_positions.len()];
        let mut gradient_degenerate_agents = 0usize;
        for (agent_index, gradient_gain) in gradient_gain_by_agent.iter_mut().enumerate() {
            let local_indices = local_neighborhood_indices(agent_index, &communication_neighbors);
            let local_mean_gradient_norm = local_indices
                .iter()
                .map(|index| gradient_norms[*index] / local_indices.len() as f64)
                .sum::<f64>();
            if !local_mean_gradient_norm.is_finite()
                || local_mean_gradient_norm <= self.config.numerical_epsilon
            {
                gradient_degenerate_agents += 1;
                *gradient_gain = 0.0;
            } else {
                *gradient_gain = 1.0 / local_mean_gradient_norm;
            }
        }
        let gradient_degenerate = gradient_degenerate_agents == previous_positions.len();
        let lambdas = &current_network.lambdas;

        // Work against cloned random state and candidate positions. If the next landscape
        // observation fails, no agent/RNG/tick/cost state is committed by this update.
        let mut next_rng = self.rng;
        let dimension = previous_positions[0].len();
        let kernel_scale = self.config.kernel.argument_scale();
        let noise_scale = (2.0 / PI) * self.sigma;
        let social_scale = 2.0 / PI;
        let sqrt_dt = self.config.dt.sqrt();
        let mut next_agents = Vec::with_capacity(self.agents.len());

        for agent_index in 0..previous_positions.len() {
            let mut next = vec![0.0; dimension];
            for coordinate in 0..dimension {
                let neighbor_indices = &communication_neighbors[agent_index];
                let mut social_sum = 0.0;
                for &other_index in neighbor_indices {
                    let delta = previous_positions[agent_index][coordinate]
                        - previous_positions[other_index][coordinate];
                    social_sum += (kernel_scale * delta).sin();
                }
                let social_gradient = if neighbor_indices.is_empty() {
                    0.0
                } else {
                    -social_sum / neighbor_indices.len() as f64
                };
                let fitness_gradient = current_observations[agent_index].gradient[coordinate];
                let fitness_scale = gradient_gain_by_agent[agent_index] * 4.0 / (PI * PI);
                let noise = noise_scale * lambdas[agent_index] * sqrt_dt * next_rng.next_normal();
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
            next_agents.push(ScmAgent { position: next });
        }

        let updated_observations = match self.observe_agents(&next_agents, landscape) {
            Ok(observations) => observations,
            Err(failure) => {
                self.record_evaluation_attempts(failure.attempted_agent_evaluations)?;
                return Err(failure.error);
            }
        };
        let successor_evaluations = next_agents.len();
        let evaluations_this_step = initial_evaluations_this_call + successor_evaluations;
        let new_total_function_evaluations = self
            .total_function_evaluations
            .checked_add(
                u64::try_from(successor_evaluations)
                    .map_err(|_| ScmError::EvaluationCounterOverflow)?,
            )
            .ok_or(ScmError::EvaluationCounterOverflow)?;

        let updated_positions: Vec<Vec<f64>> = next_agents
            .iter()
            .map(|agent| agent.position.clone())
            .collect();
        let updated_neighbors =
            build_communication_neighbors(&updated_positions, self.config.communication_topology);
        let updated_network = communication_consensus_state(
            &updated_positions,
            &updated_observations,
            &updated_neighbors,
            self.config.numerical_epsilon,
        );
        let updated_consensus = updated_network.mean_consensus;

        let history_capacity = match self.config.consensus_window {
            ScmConsensusWindow::PublishedEqualHalfIntervals => self.config.tau_steps,
            ScmConsensusWindow::RepositoryMatlabInclusiveEndpoints => self.config.tau_steps + 1,
        };
        let mut next_history = self.consensus_history.clone();
        next_history.push_back(updated_consensus);
        while next_history.len() > history_capacity {
            next_history.pop_front();
        }

        let sigma_before_update = self.sigma;
        let mut next_sigma = self.sigma;
        let mut consensus_window_delta = None;
        let mut convergence_window_satisfied = false;
        let tau_steps = self.config.tau_steps as u64;
        let window_ready = match self.config.consensus_window {
            ScmConsensusWindow::PublishedEqualHalfIntervals => {
                next_tick % tau_steps == 0 && next_history.len() == self.config.tau_steps
            }
            ScmConsensusWindow::RepositoryMatlabInclusiveEndpoints => {
                next_tick > tau_steps
                    && (next_tick - 1) % tau_steps == 0
                    && next_history.len() == self.config.tau_steps + 1
            }
        };

        if window_ready {
            let half = self.config.tau_steps / 2;
            let (early, late) = match self.config.consensus_window {
                ScmConsensusWindow::PublishedEqualHalfIntervals => {
                    let early = next_history.iter().take(half).sum::<f64>() * self.config.dt;
                    let late = next_history.iter().skip(half).sum::<f64>() * self.config.dt;
                    (early, late)
                }
                ScmConsensusWindow::RepositoryMatlabInclusiveEndpoints => {
                    let early_count = half + 1;
                    let early = next_history.iter().take(early_count).sum::<f64>() * self.config.dt;
                    let late = next_history.iter().skip(early_count).sum::<f64>() * self.config.dt;
                    (early, late)
                }
            };
            let delta = late - early;
            consensus_window_delta = Some(delta);

            // Source rule: increase sigma by omega*dt only when consensus flattens/declines.
            // A zero delta is handled explicitly rather than evaluating 0/0.
            if delta < 0.0 {
                next_sigma += self.config.dt * self.config.omega;
                if next_sigma >= self.config.sigma_limit {
                    next_sigma = self.config.sigma_initial;
                }
            }
            let gamma = (1.0 - (next_agents.len() as f64 / 100.0).tanh()) / 20.0 + 0.9;
            convergence_window_satisfied = late + early > gamma * self.config.tau();
        }

        let outside_domain_agents = updated_observations
            .iter()
            .filter(|observation| observation.outside_domain)
            .count();
        let (best_agent_index, best_agent_fitness) = updated_observations
            .iter()
            .enumerate()
            .max_by(|(left_index, left), (right_index, right)| {
                left.fitness
                    .total_cmp(&right.fitness)
                    // max_by returns the later item on equality; reverse the index
                    // comparison so ties deterministically resolve to the lowest index.
                    .then_with(|| right_index.cmp(left_index))
            })
            .map(|(index, observation)| (index, observation.fitness))
            .expect("SCM validates a non-empty agent population");
        let mean_agent_fitness = updated_observations
            .iter()
            .map(|observation| observation.fitness)
            .sum::<f64>()
            / updated_observations.len() as f64;
        let next_best_fitness_seen = self
            .best_fitness_seen
            .map_or(best_agent_fitness, |best| best.max(best_agent_fitness));
        let agent_fitness_variance = updated_observations
            .iter()
            .map(|observation| {
                let delta = observation.fitness - mean_agent_fitness;
                delta * delta
            })
            .sum::<f64>()
            / updated_observations.len() as f64;
        let report = ScmStepReport {
            tick: next_tick,
            interaction_kernel: self.config.kernel,
            consensus_window: self.config.consensus_window,
            communication_topology: self.config.communication_topology,
            best_agent_fitness,
            best_fitness_seen: next_best_fitness_seen,
            best_agent_index,
            mean_agent_fitness,
            agent_fitness_variance,
            consensus: updated_consensus,
            sigma_before_update,
            sigma_after_update: next_sigma,
            centroid: updated_network.mean_local_centroid,
            mean_agent_distance: updated_network
                .distances_to_local_centroid
                .iter()
                .sum::<f64>()
                / updated_network.distances_to_local_centroid.len() as f64,
            maximum_pairwise_distance: updated_network.maximum_local_pairwise_distance,
            mean_gradient_norm_before_update: mean_gradient_norm,
            gradient_degenerate,
            gradient_degenerate_agents,
            swarm_spread_degenerate: updated_network.locally_degenerate_spread_agents
                == next_agents.len(),
            locally_degenerate_spread_agents: updated_network.locally_degenerate_spread_agents,
            mean_communicating_neighbors: updated_network.mean_neighbor_count,
            isolated_agents: updated_network.isolated_agents,
            outside_domain_agents,
            consensus_window_delta,
            convergence_window_satisfied,
            function_evaluations_this_step: evaluations_this_step,
            total_function_evaluations: new_total_function_evaluations,
        };

        // Commit the complete state transition only after the successor observation succeeds.
        self.agents = next_agents;
        self.rng = next_rng;
        self.observations = Some(updated_observations);
        self.best_fitness_seen = Some(next_best_fitness_seen);
        self.total_function_evaluations = new_total_function_evaluations;
        self.consensus_history = next_history;
        self.sigma = next_sigma;
        self.tick = next_tick;

        Ok(report)
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

    fn record_evaluation_attempts(&mut self, count: usize) -> Result<(), ScmError> {
        let new_total = self
            .total_function_evaluations
            .checked_add(u64::try_from(count).map_err(|_| ScmError::EvaluationCounterOverflow)?)
            .ok_or(ScmError::EvaluationCounterOverflow)?;
        self.total_function_evaluations = new_total;
        Ok(())
    }

    fn observe_agents<L: ScmLandscape>(
        &self,
        agents: &[ScmAgent],
        landscape: &L,
    ) -> Result<Vec<ScmObservation>, ScmObservationFailure> {
        let mut observations = Vec::with_capacity(agents.len());
        for (agent_index, agent) in agents.iter().enumerate() {
            if agent
                .position
                .iter()
                .any(|value| !(0.0..=1.0).contains(value))
            {
                observations.push(ScmObservation {
                    fitness: 0.0,
                    gradient: conical_barrier_gradient(&agent.position, self.config),
                    outside_domain: true,
                });
                continue;
            }
            let (fitness, gradient) = match landscape.fitness_and_gradient(&agent.position) {
                Ok(result) => result,
                Err(reason) => {
                    return Err(ScmObservationFailure {
                        error: ScmError::LandscapeFailure {
                            agent: agent_index,
                            reason,
                        },
                        attempted_agent_evaluations: agent_index + 1,
                    });
                }
            };
            if !fitness.is_finite()
                || !(0.0..=1.0).contains(&fitness)
                || gradient.len() != agent.position.len()
                || gradient.iter().any(|value| !value.is_finite())
            {
                return Err(ScmObservationFailure {
                    error: ScmError::InvalidLandscapeObservation { agent: agent_index },
                    attempted_agent_evaluations: agent_index + 1,
                });
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
    delta
        .into_iter()
        .map(|value| magnitude * (value / norm))
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
struct ScmCommunicationState {
    lambdas: Vec<f64>,
    distances_to_local_centroid: Vec<f64>,
    maximum_local_pairwise_distance: f64,
    mean_local_centroid: Vec<f64>,
    mean_consensus: f64,
    locally_degenerate_spread_agents: usize,
    mean_neighbor_count: f64,
    isolated_agents: usize,
}

/// Build deterministic, sorted neighbor lists. Self is never included here; local
/// consensus adds the focal agent explicitly. CompleteGraph is the published SCM baseline.
fn build_communication_neighbors(
    positions: &[Vec<f64>],
    topology: ScmCommunicationTopology,
) -> Vec<Vec<usize>> {
    let count = positions.len();
    let mut neighbors = vec![Vec::new(); count];
    match topology {
        ScmCommunicationTopology::CompleteGraph => {
            for (focal, list) in neighbors.iter_mut().enumerate() {
                list.extend((0..count).filter(|candidate| *candidate != focal));
            }
        }
        ScmCommunicationTopology::Ring => {
            for (focal, list) in neighbors.iter_mut().enumerate() {
                if count > 1 {
                    let previous = (focal + count - 1) % count;
                    let next = (focal + 1) % count;
                    if previous != focal {
                        list.push(previous);
                    }
                    if next != focal && next != previous {
                        list.push(next);
                    }
                    list.sort_unstable();
                }
            }
        }
        ScmCommunicationTopology::NoCommunication => {}
        ScmCommunicationTopology::MetricRadius { radius } => {
            for focal in 0..count {
                for candidate in 0..count {
                    if candidate == focal {
                        continue;
                    }
                    let difference: Vec<f64> = positions[focal]
                        .iter()
                        .zip(positions[candidate].iter())
                        .map(|(left, right)| left - right)
                        .collect();
                    if l2_norm(&difference) <= radius {
                        neighbors[focal].push(candidate);
                    }
                }
            }
        }
    }
    neighbors
}

fn local_neighborhood_indices(focal: usize, neighbors: &[Vec<usize>]) -> Vec<usize> {
    let mut indices = Vec::with_capacity(neighbors[focal].len() + 1);
    indices.push(focal);
    indices.extend(neighbors[focal].iter().copied());
    indices.sort_unstable();
    indices.dedup();
    indices
}

fn communication_consensus_state(
    positions: &[Vec<f64>],
    observations: &[ScmObservation],
    neighbors: &[Vec<usize>],
    epsilon: f64,
) -> ScmCommunicationState {
    let count = positions.len();
    let dimension = positions[0].len();
    let mut lambdas = vec![0.0; count];
    let mut distances_to_local_centroid = vec![0.0; count];
    let mut mean_local_centroid = vec![0.0; dimension];
    let mut consensus_sum = 0.0;
    let mut maximum_local_pairwise_distance: f64 = 0.0;
    let mut locally_degenerate_spread_agents = 0usize;
    let total_neighbors = neighbors.iter().map(Vec::len).sum::<usize>();
    let isolated_agents = neighbors.iter().filter(|list| list.is_empty()).count();

    for focal in 0..count {
        let local_indices = local_neighborhood_indices(focal, neighbors);
        let local_count = local_indices.len();
        let minimum_fitness = local_indices
            .iter()
            .map(|index| observations[*index].fitness)
            .fold(f64::INFINITY, f64::min);
        let weights: Vec<f64> = local_indices
            .iter()
            .map(|index| (observations[*index].fitness - minimum_fitness).max(0.0))
            .collect();
        let weight_sum = weights.iter().sum::<f64>();

        let mut centroid = vec![0.0; dimension];
        if weight_sum <= epsilon {
            for index in &local_indices {
                for coordinate in 0..dimension {
                    centroid[coordinate] += positions[*index][coordinate] / local_count as f64;
                }
            }
        } else {
            for (index, weight) in local_indices.iter().zip(weights.iter()) {
                for coordinate in 0..dimension {
                    centroid[coordinate] += positions[*index][coordinate] * weight / weight_sum;
                }
            }
        }

        for coordinate in 0..dimension {
            mean_local_centroid[coordinate] += centroid[coordinate] / count as f64;
        }

        let mut local_distances = Vec::with_capacity(local_count);
        for index in &local_indices {
            let difference: Vec<f64> = positions[*index]
                .iter()
                .zip(centroid.iter())
                .map(|(left, right)| left - right)
                .collect();
            local_distances.push(l2_norm(&difference));
        }
        let focal_local_index = local_indices
            .iter()
            .position(|index| *index == focal)
            .expect("focal agent is included in its local neighborhood");
        let focal_distance = local_distances[focal_local_index];
        distances_to_local_centroid[focal] = focal_distance;

        let mut local_max_distance: f64 = 0.0;
        for left in 0..local_count {
            for right in (left + 1)..local_count {
                let difference: Vec<f64> = positions[local_indices[left]]
                    .iter()
                    .zip(positions[local_indices[right]].iter())
                    .map(|(a, b)| a - b)
                    .collect();
                local_max_distance = local_max_distance.max(l2_norm(&difference));
            }
        }
        maximum_local_pairwise_distance = maximum_local_pairwise_distance.max(local_max_distance);

        let raw_consensus = 1.0 - (2.0 / local_count as f64) * local_distances.iter().sum::<f64>();
        let local_consensus = raw_consensus.max(0.05);
        consensus_sum += local_consensus / count as f64;

        if local_max_distance <= epsilon {
            locally_degenerate_spread_agents += 1;
            lambdas[focal] = 0.0;
        } else {
            lambdas[focal] = focal_distance / (local_max_distance * local_consensus);
        }
    }

    ScmCommunicationState {
        lambdas,
        distances_to_local_centroid,
        maximum_local_pairwise_distance,
        mean_local_centroid,
        mean_consensus: consensus_sum,
        locally_degenerate_spread_agents,
        mean_neighbor_count: total_neighbors as f64 / count as f64,
        isolated_agents,
    }
}

fn l2_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
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
            let fitness = 1.0 - delta.iter().map(|value| value * value).sum::<f64>();
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
            ScmAgent {
                position: vec![0.2, 0.2],
            },
            ScmAgent {
                position: vec![0.8, 0.2],
            },
            ScmAgent {
                position: vec![0.5, 0.8],
            },
        ]
    }

    #[test]
    fn published_discrete_equation_and_matlab_source_share_pi_over_two_kernel() {
        assert!(
            (ScmInteractionKernel::PublishedEquationAndMatlabSource.argument_scale() - PI / 2.0)
                .abs()
                < 1.0e-15
        );
        assert_eq!(
            ScmInteractionKernel::PublishedEquationAndMatlabSource.label(),
            "published_eq10_and_matlab_tstep_sin_pi_over_2"
        );
    }

    #[test]
    fn deterministic_seed_replays_the_rust_reference() {
        let config = ScmConfig {
            tau_steps: 4,
            ..ScmConfig::default()
        };
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
            ScmAgent {
                position: vec![0.5, 0.5],
            },
            ScmAgent {
                position: vec![0.5, 0.5],
            },
        ];
        let mut model = ScmModel::new(agents, ScmConfig::default(), 7).unwrap();
        let report = model.step(&Quadratic).unwrap();
        assert!(report.swarm_spread_degenerate);
        assert!(report.consensus.is_finite());
        assert!(
            model
                .agents()
                .iter()
                .all(|agent| agent.position.iter().all(|x| x.is_finite()))
        );
    }

    #[test]
    fn zero_mean_gradient_uses_finite_gain_fallback() {
        let mut model = ScmModel::new(small_agents(), ScmConfig::default(), 11).unwrap();
        let report = model.step(&Flat).unwrap();
        assert!(report.gradient_degenerate);
        assert_eq!(report.mean_gradient_norm_before_update, 0.0);
        assert!(report.consensus.is_finite());
    }

    #[test]
    fn flat_consensus_window_does_not_generate_nan_noise_adaptation() {
        let agents = vec![
            ScmAgent {
                position: vec![0.5, 0.5],
            },
            ScmAgent {
                position: vec![0.5, 0.5],
            },
        ];
        let config = ScmConfig {
            tau_steps: 4,
            ..ScmConfig::default()
        };
        let mut model = ScmModel::new(agents, config, 19).unwrap();
        let reports = model.run_steps(&Flat, 8).unwrap();
        assert!(
            reports
                .iter()
                .all(|report| report.sigma_after_update.is_finite())
        );
        assert!((model.sigma() - config.sigma_initial).abs() < 1.0e-15);
    }

    #[test]
    fn out_of_domain_agents_use_a_finite_conical_barrier_observation() {
        let agents = vec![
            ScmAgent {
                position: vec![1.1, 0.5],
            },
            ScmAgent {
                position: vec![1.2, 0.5],
            },
        ];
        let mut model = ScmModel::new(agents, ScmConfig::default(), 23).unwrap();
        let report = model.step(&Quadratic).unwrap();
        assert_eq!(report.outside_domain_agents, 2);
        assert!(
            model
                .agents()
                .iter()
                .all(|agent| agent.position.iter().all(|x| x.is_finite()))
        );
    }

    struct FailOnNextObservationBatch {
        calls: std::cell::Cell<usize>,
        successful_calls: usize,
    }

    impl ScmLandscape for FailOnNextObservationBatch {
        fn fitness_and_gradient(&self, position: &[f64]) -> Result<(f64, Vec<f64>), String> {
            let call = self.calls.get();
            self.calls.set(call + 1);
            if call >= self.successful_calls {
                return Err("injected observation failure".to_owned());
            }
            Ok((0.5, vec![0.0; position.len()]))
        }
    }

    #[test]
    fn failed_successor_observation_preserves_motion_but_accounts_initial_measurements() {
        let mut model = ScmModel::new(small_agents(), ScmConfig::default(), 31).unwrap();
        let before_agents = model.agents.clone();
        let before_rng = model.rng;
        let before_tick = model.tick;
        let failure = FailOnNextObservationBatch {
            calls: std::cell::Cell::new(0),
            successful_calls: 4,
        };

        assert!(matches!(
            model.step(&failure),
            Err(ScmError::LandscapeFailure { .. })
        ));
        assert_eq!(model.agents, before_agents);
        assert_eq!(model.rng, before_rng);
        assert_eq!(model.tick, before_tick);
        assert!(model.observations.is_some());
        // Three initial readings and one successful successor reading plus the failed
        // second successor request are all counted as evaluation attempts.
        assert_eq!(model.total_function_evaluations, 5);
    }

    #[test]
    fn extreme_finite_out_of_domain_position_keeps_barrier_gradient_finite() {
        let agents = vec![
            ScmAgent {
                position: vec![f64::MAX, 0.5],
            },
            ScmAgent {
                position: vec![f64::MAX, 0.5],
            },
        ];
        let config = ScmConfig::default();
        let gradient = conical_barrier_gradient(&agents[0].position, config);
        assert!(gradient.iter().all(|value| value.is_finite()));

        let mut model = ScmModel::new(agents, config, 37).unwrap();
        let report = model.step(&Flat).unwrap();
        assert_eq!(report.outside_domain_agents, 2);
        assert!(
            model
                .agents()
                .iter()
                .all(|agent| { agent.position.iter().all(|value| value.is_finite()) })
        );
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
    fn no_communication_topology_never_connects_coincident_agents() {
        let agents = vec![
            ScmAgent {
                position: vec![0.5, 0.5],
            },
            ScmAgent {
                position: vec![0.5, 0.5],
            },
            ScmAgent {
                position: vec![0.2, 0.2],
            },
        ];
        let mut model = ScmModel::new(
            agents,
            ScmConfig {
                communication_topology: ScmCommunicationTopology::NoCommunication,
                ..ScmConfig::default()
            },
            909,
        )
        .unwrap();

        let report = model.step(&Quadratic).unwrap();
        assert_eq!(
            report.communication_topology,
            ScmCommunicationTopology::NoCommunication
        );
        assert_eq!(report.mean_communicating_neighbors, 0.0);
        assert_eq!(report.isolated_agents, 3);
        assert!(report.best_fitness_seen >= report.best_agent_fitness);
    }

    #[test]
    fn complete_graph_matches_metric_radius_that_connects_every_agent() {
        let agents = small_agents();
        let complete_config = ScmConfig {
            tau_steps: 4,
            communication_topology: ScmCommunicationTopology::CompleteGraph,
            ..ScmConfig::default()
        };
        let radius_config = ScmConfig {
            tau_steps: 4,
            communication_topology: ScmCommunicationTopology::MetricRadius { radius: 2.0 },
            ..ScmConfig::default()
        };
        let mut complete = ScmModel::new(agents.clone(), complete_config, 211).unwrap();
        let mut radius = ScmModel::new(agents, radius_config, 211).unwrap();

        let complete_reports = complete.run_steps(&Quadratic, 12).unwrap();
        let radius_reports = radius.run_steps(&Quadratic, 12).unwrap();

        assert_eq!(complete.agents(), radius.agents());
        assert_eq!(complete.sigma(), radius.sigma());
        assert_eq!(
            complete.total_function_evaluations(),
            radius.total_function_evaluations()
        );
        for (left, right) in complete_reports.iter().zip(radius_reports.iter()) {
            assert_eq!(left.consensus, right.consensus);
            assert_eq!(left.centroid, right.centroid);
            assert_eq!(left.mean_agent_distance, right.mean_agent_distance);
            assert_eq!(
                left.maximum_pairwise_distance,
                right.maximum_pairwise_distance
            );
            assert_eq!(left.mean_communicating_neighbors, 2.0);
            assert_eq!(right.mean_communicating_neighbors, 2.0);
        }
    }

    #[test]
    fn task_fitness_metrics_are_reported_separately_from_consensus() {
        let positions = vec![
            ScmAgent {
                position: vec![0.1, 0.1],
            },
            ScmAgent {
                position: vec![0.9, 0.1],
            },
            ScmAgent {
                position: vec![0.1, 0.9],
            },
            ScmAgent {
                position: vec![0.9, 0.9],
            },
        ];
        let landscape = Quadratic;
        let mut model = ScmModel::new(
            positions,
            ScmConfig {
                sigma_initial: 0.0,
                sigma_limit: 0.0,
                omega: 0.0,
                tau_steps: 4,
                ..ScmConfig::default()
            },
            77,
        )
        .unwrap();

        let report = model.step(&landscape).unwrap();
        let observations: Vec<f64> = model
            .agents()
            .iter()
            .map(|agent| landscape.fitness_and_gradient(&agent.position).unwrap().0)
            .collect();
        let expected_best_index = observations
            .iter()
            .enumerate()
            .max_by(|(li, left), (ri, right)| left.total_cmp(right).then_with(|| ri.cmp(li)))
            .map(|(index, _)| index)
            .unwrap();
        let expected_best = observations[expected_best_index];
        let expected_mean = observations.iter().sum::<f64>() / observations.len() as f64;
        let expected_variance = observations
            .iter()
            .map(|fitness| {
                let delta = *fitness - expected_mean;
                delta * delta
            })
            .sum::<f64>()
            / observations.len() as f64;

        assert_eq!(report.best_agent_index, expected_best_index);
        assert!((report.best_agent_fitness - expected_best).abs() < 1.0e-12);
        assert!((report.mean_agent_fitness - expected_mean).abs() < 1.0e-12);
        assert!((report.agent_fitness_variance - expected_variance).abs() < 1.0e-12);
        assert!((0.0..=1.0).contains(&report.best_agent_fitness));
        assert!((0.0..=1.0).contains(&report.mean_agent_fitness));
        assert!(report.agent_fitness_variance.is_finite());
        // A high consensus is not used as a substitute for measured task fitness.
        assert!(report.consensus.is_finite());
    }

    #[test]
    fn best_fitness_seen_is_monotonic_and_includes_initial_observations() {
        let mut model = ScmModel::new(
            vec![
                ScmAgent {
                    position: vec![0.2, 0.2],
                },
                ScmAgent {
                    position: vec![0.8, 0.8],
                },
            ],
            ScmConfig {
                sigma_initial: 0.0,
                sigma_limit: 0.0,
                omega: 0.0,
                tau_steps: 4,
                communication_topology: ScmCommunicationTopology::NoCommunication,
                ..ScmConfig::default()
            },
            101,
        )
        .unwrap();
        assert!(model.best_fitness_seen().is_none());

        let first = model.step(&Quadratic).unwrap();
        assert_eq!(model.best_fitness_seen(), Some(first.best_fitness_seen));
        assert!(first.best_fitness_seen >= first.best_agent_fitness);

        let second = model.step(&Quadratic).unwrap();
        assert!(second.best_fitness_seen >= first.best_fitness_seen);
        assert_eq!(model.best_fitness_seen(), Some(second.best_fitness_seen));
    }

    #[test]
    fn communication_topology_baselines_share_exact_objective_evaluation_budgets() {
        let topologies = [
            ScmCommunicationTopology::CompleteGraph,
            ScmCommunicationTopology::Ring,
            ScmCommunicationTopology::NoCommunication,
        ];
        let expected_total_evaluations = small_agents().len() as u64 * 6;
        let mut final_best_values = Vec::new();

        for topology in topologies {
            let mut model = ScmModel::new(
                small_agents(),
                ScmConfig {
                    sigma_initial: 0.0,
                    sigma_limit: 0.0,
                    omega: 0.0,
                    tau_steps: 4,
                    communication_topology: topology,
                    ..ScmConfig::default()
                },
                31337,
            )
            .unwrap();
            let reports = model.run_steps(&Quadratic, 5).unwrap();

            assert_eq!(reports.len(), 5);
            assert_eq!(
                reports.last().unwrap().total_function_evaluations,
                expected_total_evaluations
            );
            assert_eq!(
                model.total_function_evaluations(),
                expected_total_evaluations
            );
            assert!(reports.iter().all(|report| {
                report.best_agent_fitness.is_finite()
                    && (0.0..=1.0).contains(&report.best_agent_fitness)
                    && report.best_fitness_seen.is_finite()
                    && (0.0..=1.0).contains(&report.best_fitness_seen)
                    && report.mean_agent_fitness.is_finite()
                    && (0.0..=1.0).contains(&report.mean_agent_fitness)
                    && report.agent_fitness_variance.is_finite()
                    && report.agent_fitness_variance >= 0.0
            }));
            assert!(
                reports
                    .windows(2)
                    .all(|pair| { pair[1].best_fitness_seen >= pair[0].best_fitness_seen })
            );
            final_best_values.push(reports.last().unwrap().best_fitness_seen);
        }

        // The baseline comparison is valid at equal observation cost. It does not
        // presume that more communication must improve objective performance.
        assert_eq!(final_best_values.len(), topologies.len());
    }

    #[test]
    fn ring_topology_reports_two_neighbors_per_agent_for_four_agent_ring() {
        let agents = vec![
            ScmAgent {
                position: vec![0.1, 0.1],
            },
            ScmAgent {
                position: vec![0.8, 0.1],
            },
            ScmAgent {
                position: vec![0.8, 0.8],
            },
            ScmAgent {
                position: vec![0.1, 0.8],
            },
        ];
        let config = ScmConfig {
            communication_topology: ScmCommunicationTopology::Ring,
            ..ScmConfig::default()
        };
        let mut model = ScmModel::new(agents, config, 223).unwrap();
        let report = model.step(&Flat).unwrap();

        assert_eq!(report.mean_communicating_neighbors, 2.0);
        assert_eq!(report.isolated_agents, 0);
        assert_eq!(
            report.communication_topology,
            ScmCommunicationTopology::Ring
        );
    }

    #[test]
    fn zero_radius_topology_is_a_no_communication_control() {
        let agents = small_agents();
        let config = ScmConfig {
            communication_topology: ScmCommunicationTopology::MetricRadius { radius: 0.0 },
            ..ScmConfig::default()
        };
        let mut model = ScmModel::new(agents.clone(), config, 227).unwrap();
        let report = model.step(&Flat).unwrap();

        assert_eq!(report.mean_communicating_neighbors, 0.0);
        assert_eq!(report.isolated_agents, agents.len());
        assert_eq!(report.locally_degenerate_spread_agents, agents.len());
        assert!(report.swarm_spread_degenerate);
        assert_eq!(
            model
                .agents()
                .iter()
                .map(|agent| agent.position.clone())
                .collect::<Vec<_>>(),
            agents
                .iter()
                .map(|agent| agent.position.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn rejects_negative_metric_communication_radius() {
        assert!(matches!(
            ScmModel::new(
                small_agents(),
                ScmConfig {
                    communication_topology: ScmCommunicationTopology::MetricRadius { radius: -0.1 },
                    ..ScmConfig::default()
                },
                229
            ),
            Err(ScmError::InvalidParameter {
                name: "communication_radius",
                ..
            })
        ));
    }

    struct UnboundedFitness;

    impl ScmLandscape for UnboundedFitness {
        fn fitness_and_gradient(&self, position: &[f64]) -> Result<(f64, Vec<f64>), String> {
            Ok((1.1, vec![0.0; position.len()]))
        }
    }

    #[test]
    fn source_kernel_uses_the_published_and_matlab_pi_over_two_social_term() {
        let agents = small_agents();
        let config = ScmConfig {
            tau_steps: 4,
            kernel: ScmInteractionKernel::PublishedEquationAndMatlabSource,
            ..ScmConfig::default()
        };
        let mut model = ScmModel::new(agents.clone(), config, 101).unwrap();

        let before = agents
            .iter()
            .map(|agent| agent.position.clone())
            .collect::<Vec<_>>();
        model.step(&Flat).unwrap();

        assert!(model.agents().iter().all(|agent| {
            agent
                .position
                .iter()
                .all(|coordinate| coordinate.is_finite())
        }));
        assert_ne!(
            model
                .agents()
                .iter()
                .map(|agent| agent.position.clone())
                .collect::<Vec<_>>(),
            before
        );
    }

    #[test]
    fn rejects_fitness_outside_dimensionless_unit_interval() {
        let mut model = ScmModel::new(small_agents(), ScmConfig::default(), 103).unwrap();
        assert!(matches!(
            model.step(&UnboundedFitness),
            Err(ScmError::InvalidLandscapeObservation { .. })
        ));
    }

    #[test]
    fn consensus_window_conventions_have_distinct_first_update_ticks() {
        let mut published = ScmModel::new(
            small_agents(),
            ScmConfig {
                tau_steps: 4,
                consensus_window: ScmConsensusWindow::PublishedEqualHalfIntervals,
                ..ScmConfig::default()
            },
            107,
        )
        .unwrap();
        let mut repository = ScmModel::new(
            small_agents(),
            ScmConfig {
                tau_steps: 4,
                consensus_window: ScmConsensusWindow::RepositoryMatlabInclusiveEndpoints,
                ..ScmConfig::default()
            },
            107,
        )
        .unwrap();

        let published_reports = published.run_steps(&Flat, 5).unwrap();
        let repository_reports = repository.run_steps(&Flat, 5).unwrap();

        assert!(published_reports[3].consensus_window_delta.is_some());
        assert!(published_reports[4].consensus_window_delta.is_none());
        assert!(repository_reports[3].consensus_window_delta.is_none());
        assert!(repository_reports[4].consensus_window_delta.is_some());
    }

    #[test]
    fn rejects_one_agent_and_odd_consensus_window() {
        assert!(matches!(
            ScmModel::new(
                vec![ScmAgent {
                    position: vec![0.1]
                }],
                ScmConfig::default(),
                1
            ),
            Err(ScmError::TooFewAgents(1))
        ));
        assert!(matches!(
            ScmModel::new(
                small_agents(),
                ScmConfig {
                    tau_steps: 3,
                    ..ScmConfig::default()
                },
                1
            ),
            Err(ScmError::InvalidConfiguration)
        ));
    }
}
