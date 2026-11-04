// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact finite-population fitness, Fermi imitation and rare-mutation
//! selection for the inspectable collective-risk strategies.
//!
//! The procedure is source-aligned with the population-selection method in
//! Huynh et al. (2026), but uses this crate's scripted baseline strategies.
//! It does not reproduce the paper's model-specific fitness measurements.

use crate::{GameConfig, SEATS, Strategy, run_episode};
use core::fmt;

pub const STRATEGY_COUNT: usize = Strategy::ALL.len();
pub const GROUP_SIZE: usize = SEATS;
pub const TABLEMATE_COUNT: usize = GROUP_SIZE - 1;
pub const MAX_POPULATION_SIZE: usize = 10_000;
pub const MAX_SELECTION_INTENSITY: f64 = 100.0;
pub const DEFAULT_SELECTION_INTENSITY: f64 = 1.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PopulationState {
    /// Counts are indexed by Strategy::ALL.
    pub counts: [usize; STRATEGY_COUNT],
}

impl PopulationState {
    pub fn new(counts: [usize; STRATEGY_COUNT]) -> Result<Self, PopulationError> {
        let state = Self { counts };
        state.validate()?;
        Ok(state)
    }

    pub fn population_size(self) -> usize {
        self.counts
            .iter()
            .copied()
            .fold(0_usize, usize::saturating_add)
    }

    pub fn count(self, strategy: Strategy) -> usize {
        self.counts[strategy_index(strategy)]
    }

    pub fn validate(self) -> Result<(), PopulationError> {
        let population_size = self.population_size();
        if !(GROUP_SIZE..=MAX_POPULATION_SIZE).contains(&population_size) {
            return Err(PopulationError::InvalidPopulationSize(population_size));
        }
        Ok(())
    }

    pub fn monomorphic(
        strategy: Strategy,
        population_size: usize,
    ) -> Result<Self, PopulationError> {
        let mut counts = [0; STRATEGY_COUNT];
        counts[strategy_index(strategy)] = population_size;
        Self::new(counts)
    }

    pub fn balanced(population_size: usize) -> Result<Self, PopulationError> {
        if population_size < GROUP_SIZE || !population_size.is_multiple_of(STRATEGY_COUNT) {
            return Err(PopulationError::InvalidBalancedPopulation(population_size));
        }
        Self::new([population_size / STRATEGY_COUNT; STRATEGY_COUNT])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupComposition {
    /// Number of sampled tablemates of each strategy type.
    pub tablemate_counts: [usize; STRATEGY_COUNT],
    /// Exact multivariate-hypergeometric probability, represented as f64.
    pub probability: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TypeFitness {
    pub strategy: Strategy,
    pub population_count: usize,
    pub expected_payoff: f64,
    pub expected_group_welfare: f64,
    pub target_reach_probability: f64,
    pub expected_group_contribution: f64,
    pub payoff_variance_across_group_compositions: f64,
    pub compositions_evaluated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FermiTransitionRates {
    pub increase_probability: f64,
    pub decrease_probability: f64,
    pub stay_probability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RareMutationChain {
    /// Row i contains transition probabilities from resident i to mutant j.
    pub transition_matrix: Vec<Vec<f64>>,
    /// log_fixation[i][j] is log probability that j fixes in resident i.
    pub log_fixation_matrix: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StationaryDistribution {
    pub weights: Vec<f64>,
    pub iterations: usize,
    /// L1 residual against the original (non-lazy) transition matrix.
    pub residual_l1: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvasionCycleOrientation {
    Forward,
    Reverse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvasionCycle {
    /// A -> B -> C -> A for Forward; A -> C -> B -> A for Reverse.
    pub strategies: [Strategy; 3],
    pub orientation: InvasionCycleOrientation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvasionGraph {
    /// Edge i -> j means j has fixation probability above neutral 1/N
    /// when introduced as a single mutant into a population of i.
    pub adjacency: Vec<Vec<bool>>,
    pub edge_count: usize,
    pub three_cycles: Vec<InvasionCycle>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PopulationError {
    InvalidPopulationSize(usize),
    InvalidBalancedPopulation(usize),
    AbsentFocalStrategy(Strategy),
    InvalidGameRisk(f64),
    InvalidSelectionIntensity(f64),
    InvalidMutantCount {
        population_size: usize,
        mutant_count: usize,
    },
    InvalidFitness(f64),
    InvalidTransitionMatrix,
    InvalidInvasionMatrix,
    CombinatorialOverflow,
    ProbabilityMassError(f64),
    NoStationaryConvergence {
        iterations: usize,
        residual: f64,
    },
}

impl fmt::Display for PopulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPopulationSize(size) => write!(
                f,
                "population size must be in {GROUP_SIZE}..={MAX_POPULATION_SIZE}, got {size}"
            ),
            Self::InvalidBalancedPopulation(size) => write!(
                f,
                "balanced population size must be at least {GROUP_SIZE} and divisible by {STRATEGY_COUNT}, got {size}"
            ),
            Self::AbsentFocalStrategy(strategy) => {
                write!(
                    f,
                    "focal strategy {} is absent from the population",
                    strategy.name()
                )
            }
            Self::InvalidGameRisk(risk) => {
                write!(f, "game risk must be finite and in [0, 1], got {risk}")
            }
            Self::InvalidSelectionIntensity(beta) => write!(
                f,
                "selection intensity beta must be finite and in [0, {MAX_SELECTION_INTENSITY}], got {beta}"
            ),
            Self::InvalidMutantCount {
                population_size,
                mutant_count,
            } => write!(
                f,
                "mutant count must be in 1..{population_size}, got {mutant_count}"
            ),
            Self::InvalidFitness(value) => {
                write!(
                    f,
                    "expected fitness must be finite and non-negative, got {value}"
                )
            }
            Self::InvalidTransitionMatrix => write!(
                f,
                "transition matrix must be finite, non-negative and row-stochastic"
            ),
            Self::InvalidInvasionMatrix => {
                write!(f, "log-fixation matrix must be square and contain valid log probabilities")
            },
            Self::CombinatorialOverflow => {
                write!(f, "hypergeometric combination arithmetic overflow")
            }
            Self::ProbabilityMassError(sum) => write!(
                f,
                "hypergeometric probability mass differs from one by {}",
                (sum - 1.0).abs()
            ),
            Self::NoStationaryConvergence {
                iterations,
                residual,
            } => write!(
                f,
                "stationary iteration did not converge after {iterations} iterations; L1 residual {residual}"
            ),
        }
    }
}

impl std::error::Error for PopulationError {}

pub fn strategy_index(strategy: Strategy) -> usize {
    Strategy::ALL
        .iter()
        .position(|candidate| *candidate == strategy)
        .expect("Strategy::ALL contains every public strategy")
}

fn validate_selection_intensity(beta: f64) -> Result<(), PopulationError> {
    if !beta.is_finite() || !(0.0..=MAX_SELECTION_INTENSITY).contains(&beta) {
        return Err(PopulationError::InvalidSelectionIntensity(beta));
    }
    Ok(())
}

/// Enumerate every possible set of five tablemate strategy counts exactly.
/// The probability is prod_j C(available_j, q_j) / C(N-1,5).
pub fn group_compositions(
    population: PopulationState,
    focal_strategy: Strategy,
) -> Result<Vec<GroupComposition>, PopulationError> {
    population.validate()?;
    let focal_index = strategy_index(focal_strategy);
    if population.counts[focal_index] == 0 {
        return Err(PopulationError::AbsentFocalStrategy(focal_strategy));
    }

    let mut available = population.counts;
    available[focal_index] -= 1;
    let denominator = choose(population.population_size() - 1, TABLEMATE_COUNT)?;
    let mut raw = Vec::new();
    let mut current = [0_usize; STRATEGY_COUNT];
    enumerate_compositions(&available, 0, TABLEMATE_COUNT, &mut current, 1, &mut raw)?;

    let mut compositions = Vec::with_capacity(raw.len());
    let mut probability_sum = 0.0_f64;
    for (tablemate_counts, numerator) in raw {
        let probability = numerator as f64 / denominator as f64;
        probability_sum += probability;
        compositions.push(GroupComposition {
            tablemate_counts,
            probability,
        });
    }
    if (probability_sum - 1.0).abs() > 1e-12 {
        return Err(PopulationError::ProbabilityMassError(probability_sum));
    }
    Ok(compositions)
}

fn enumerate_compositions(
    available: &[usize; STRATEGY_COUNT],
    index: usize,
    remaining: usize,
    current: &mut [usize; STRATEGY_COUNT],
    numerator: u128,
    output: &mut Vec<([usize; STRATEGY_COUNT], u128)>,
) -> Result<(), PopulationError> {
    if index + 1 == STRATEGY_COUNT {
        if remaining > available[index] {
            return Ok(());
        }
        let coefficient = choose(available[index], remaining)?;
        let final_numerator = numerator
            .checked_mul(coefficient)
            .ok_or(PopulationError::CombinatorialOverflow)?;
        current[index] = remaining;
        output.push((*current, final_numerator));
        return Ok(());
    }

    let maximum = available[index].min(remaining);
    for selected in 0..=maximum {
        let coefficient = choose(available[index], selected)?;
        let next_numerator = numerator
            .checked_mul(coefficient)
            .ok_or(PopulationError::CombinatorialOverflow)?;
        current[index] = selected;
        enumerate_compositions(
            available,
            index + 1,
            remaining - selected,
            current,
            next_numerator,
            output,
        )?;
    }
    current[index] = 0;
    Ok(())
}

fn choose(n: usize, k: usize) -> Result<u128, PopulationError> {
    if k > n {
        return Ok(0);
    }
    let k = k.min(n - k);
    let mut result = 1_u128;
    for i in 1..=k {
        result = result
            .checked_mul((n - k + i) as u128)
            .ok_or(PopulationError::CombinatorialOverflow)?
            / i as u128;
    }
    Ok(result)
}

pub fn expected_type_fitness(
    config: GameConfig,
    population: PopulationState,
    focal_strategy: Strategy,
) -> Result<TypeFitness, PopulationError> {
    population.validate()?;
    if !config.catastrophe_probability.is_finite()
        || !(0.0..=1.0).contains(&config.catastrophe_probability)
    {
        return Err(PopulationError::InvalidGameRisk(
            config.catastrophe_probability,
        ));
    }
    let population_count = population.count(focal_strategy);
    if population_count == 0 {
        return Err(PopulationError::AbsentFocalStrategy(focal_strategy));
    }

    let compositions = group_compositions(population, focal_strategy)?;
    let mut expected_payoff = 0.0;
    let mut expected_payoff_squared = 0.0;
    let mut expected_group_welfare = 0.0;
    let mut target_reach_probability = 0.0;
    let mut expected_group_contribution = 0.0;

    for composition in &compositions {
        let mut table = Vec::with_capacity(GROUP_SIZE);
        table.push(focal_strategy);
        for (strategy_type, count) in composition.tablemate_counts.iter().enumerate() {
            table.extend(core::iter::repeat_n(Strategy::ALL[strategy_type], *count));
        }
        if table.len() != GROUP_SIZE {
            return Err(PopulationError::ProbabilityMassError(table.len() as f64));
        }

        let outcome = run_episode(config, &table)
            .map_err(|_| PopulationError::InvalidGameRisk(config.catastrophe_probability))?;
        let payoff = outcome.expected_payoffs[0];
        let weight = composition.probability;
        expected_payoff += weight * payoff;
        expected_payoff_squared += weight * payoff * payoff;
        expected_group_welfare += weight * outcome.expected_group_welfare;
        target_reach_probability += if outcome.target_reached { weight } else { 0.0 };
        expected_group_contribution += weight * f64::from(outcome.total_contribution);
    }

    let variance = (expected_payoff_squared - expected_payoff * expected_payoff).max(0.0);
    Ok(TypeFitness {
        strategy: focal_strategy,
        population_count,
        expected_payoff,
        expected_group_welfare,
        target_reach_probability,
        expected_group_contribution,
        payoff_variance_across_group_compositions: variance,
        compositions_evaluated: compositions.len(),
    })
}

pub fn fermi_transition_rates(
    population_size: usize,
    mutant_count: usize,
    mutant_fitness: f64,
    resident_fitness: f64,
    beta: f64,
) -> Result<FermiTransitionRates, PopulationError> {
    validate_selection_intensity(beta)?;
    if population_size < 2 || mutant_count == 0 || mutant_count >= population_size {
        return Err(PopulationError::InvalidMutantCount {
            population_size,
            mutant_count,
        });
    }
    for value in [mutant_fitness, resident_fitness] {
        if !value.is_finite() || value < 0.0 {
            return Err(PopulationError::InvalidFitness(value));
        }
    }

    let resident_count = population_size - mutant_count;
    let ordered_cross_type_pair_probability = resident_count as f64 / population_size as f64
        * mutant_count as f64
        / (population_size - 1) as f64;
    let payoff_difference = mutant_fitness - resident_fitness;
    let increase_probability =
        ordered_cross_type_pair_probability * logistic(beta * payoff_difference);
    let decrease_probability =
        ordered_cross_type_pair_probability * logistic(-beta * payoff_difference);
    let stay_probability = (1.0 - increase_probability - decrease_probability).max(0.0);

    Ok(FermiTransitionRates {
        increase_probability,
        decrease_probability,
        stay_probability,
    })
}

fn logistic(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exponential = value.exp();
        exponential / (1.0 + exponential)
    }
}

/// Log probability that one mutant fixes in a monomorphic resident population.
pub fn log_fixation_probability(
    config: GameConfig,
    population_size: usize,
    mutant: Strategy,
    resident: Strategy,
    beta: f64,
) -> Result<f64, PopulationError> {
    validate_selection_intensity(beta)?;
    if !(GROUP_SIZE..=MAX_POPULATION_SIZE).contains(&population_size) || mutant == resident {
        return Err(PopulationError::InvalidPopulationSize(population_size));
    }

    let mut cumulative_log_ratio = 0.0_f64;
    let mut log_terms = Vec::with_capacity(population_size);
    log_terms.push(0.0_f64);

    for mutant_count in 1..population_size {
        let mut counts = [0_usize; STRATEGY_COUNT];
        counts[strategy_index(mutant)] = mutant_count;
        counts[strategy_index(resident)] = population_size - mutant_count;
        let population = PopulationState::new(counts)?;
        let mutant_fitness = expected_type_fitness(config, population, mutant)?.expected_payoff;
        let resident_fitness = expected_type_fitness(config, population, resident)?.expected_payoff;

        // For Fermi imitation, T_i^-/T_i^+ = exp(-beta * (f_mutant - f_resident)).
        cumulative_log_ratio -= beta * (mutant_fitness - resident_fitness);
        log_terms.push(cumulative_log_ratio);
    }

    Ok(-log_sum_exp(&log_terms))
}

pub fn fixation_probability(
    config: GameConfig,
    population_size: usize,
    mutant: Strategy,
    resident: Strategy,
    beta: f64,
) -> Result<f64, PopulationError> {
    Ok(log_fixation_probability(config, population_size, mutant, resident, beta)?.exp())
}

pub fn rare_mutation_chain(
    config: GameConfig,
    population_size: usize,
    beta: f64,
) -> Result<RareMutationChain, PopulationError> {
    validate_selection_intensity(beta)?;
    if !(GROUP_SIZE..=MAX_POPULATION_SIZE).contains(&population_size) {
        return Err(PopulationError::InvalidPopulationSize(population_size));
    }

    let mut transition_matrix = vec![vec![0.0; STRATEGY_COUNT]; STRATEGY_COUNT];
    let mut log_fixation_matrix = vec![vec![f64::NEG_INFINITY; STRATEGY_COUNT]; STRATEGY_COUNT];

    for resident_index in 0..STRATEGY_COUNT {
        let resident = Strategy::ALL[resident_index];
        let mut row_log_weights = Vec::new();
        let mut mutant_indices = Vec::new();
        for (mutant_index, log_fixation_slot) in
            log_fixation_matrix[resident_index].iter_mut().enumerate()
        {
            if mutant_index == resident_index {
                continue;
            }
            let mutant = Strategy::ALL[mutant_index];
            let log_fixation =
                log_fixation_probability(config, population_size, mutant, resident, beta)?;
            *log_fixation_slot = log_fixation;
            row_log_weights.push(log_fixation);
            mutant_indices.push(mutant_index);
        }

        let normalizer = log_sum_exp(&row_log_weights);
        for (position, mutant_index) in mutant_indices.iter().enumerate() {
            transition_matrix[resident_index][*mutant_index] =
                (row_log_weights[position] - normalizer).exp();
        }
    }

    validate_transition_matrix(&transition_matrix)?;
    Ok(RareMutationChain {
        transition_matrix,
        log_fixation_matrix,
    })
}

/// Construct the directed invasion graph from pairwise fixation probabilities.
///
/// An edge from resident i to mutant j is present when rho(j | i) exceeds the
/// neutral benchmark 1/N. Three-cycles are recorded in canonical strategy-index
/// order, retaining their orientation rather than forcing a total ranking.
pub fn invasion_graph(
    chain: &RareMutationChain,
    population_size: usize,
) -> Result<InvasionGraph, PopulationError> {
    if !(GROUP_SIZE..=MAX_POPULATION_SIZE).contains(&population_size) {
        return Err(PopulationError::InvalidPopulationSize(population_size));
    }
    let matrix = &chain.log_fixation_matrix;
    if matrix.len() != STRATEGY_COUNT
        || matrix.iter().any(|row| row.len() != STRATEGY_COUNT)
        || matrix.iter().flatten().any(|value| value.is_nan() || *value > 1e-12)
    {
        return Err(PopulationError::InvalidInvasionMatrix);
    }

    let neutral_log_fixation = -(population_size as f64).ln();
    let mut adjacency = vec![vec![false; STRATEGY_COUNT]; STRATEGY_COUNT];
    let mut edge_count = 0_usize;

    for (resident_index, row) in matrix.iter().enumerate() {
        for (mutant_index, log_probability) in row.iter().enumerate() {
            if resident_index == mutant_index {
                continue;
            }
            let edge = *log_probability > neutral_log_fixation + 1e-12;
            adjacency[resident_index][mutant_index] = edge;
            edge_count += usize::from(edge);
        }
    }

    let mut three_cycles = Vec::new();
    for first in 0..STRATEGY_COUNT {
        for second in (first + 1)..STRATEGY_COUNT {
            for third in (second + 1)..STRATEGY_COUNT {
                let forward = adjacency[first][second]
                    && adjacency[second][third]
                    && adjacency[third][first];
                let reverse = adjacency[first][third]
                    && adjacency[third][second]
                    && adjacency[second][first];
                if forward {
                    three_cycles.push(InvasionCycle {
                        strategies: [
                            Strategy::ALL[first],
                            Strategy::ALL[second],
                            Strategy::ALL[third],
                        ],
                        orientation: InvasionCycleOrientation::Forward,
                    });
                }
                if reverse {
                    three_cycles.push(InvasionCycle {
                        strategies: [
                            Strategy::ALL[first],
                            Strategy::ALL[second],
                            Strategy::ALL[third],
                        ],
                        orientation: InvasionCycleOrientation::Reverse,
                    });
                }
            }
        }
    }

    Ok(InvasionGraph {
        adjacency,
        edge_count,
        three_cycles,
    })
}

pub fn stationary_distribution(
    chain: &RareMutationChain,
    tolerance: f64,
    max_iterations: usize,
) -> Result<StationaryDistribution, PopulationError> {
    validate_transition_matrix(&chain.transition_matrix)?;
    if !tolerance.is_finite() || tolerance <= 0.0 || max_iterations == 0 {
        return Err(PopulationError::InvalidTransitionMatrix);
    }

    let size = chain.transition_matrix.len();
    let mut weights = vec![1.0 / size as f64; size];
    let mut residual = f64::INFINITY;

    // Lazy iteration preserves the stationary distribution and removes period-two oscillation.
    for iteration in 1..=max_iterations {
        let mut next = vec![0.0; size];
        for from in 0..size {
            next[from] += 0.5 * weights[from];
            for (to, next_weight) in next.iter_mut().enumerate() {
                *next_weight += 0.5 * weights[from] * chain.transition_matrix[from][to];
            }
        }
        residual = next
            .iter()
            .zip(&weights)
            .map(|(left, right)| (left - right).abs())
            .sum::<f64>();
        weights = next;
        if residual <= tolerance {
            let stationary_residual = stationary_residual_l1(&weights, &chain.transition_matrix);
            return Ok(StationaryDistribution {
                weights,
                iterations: iteration,
                residual_l1: stationary_residual,
            });
        }
    }

    Err(PopulationError::NoStationaryConvergence {
        iterations: max_iterations,
        residual,
    })
}

fn stationary_residual_l1(weights: &[f64], transition_matrix: &[Vec<f64>]) -> f64 {
    let size = weights.len();
    let mut projected = vec![0.0; size];
    for from in 0..size {
        for to in 0..size {
            projected[to] += weights[from] * transition_matrix[from][to];
        }
    }
    projected
        .iter()
        .zip(weights)
        .map(|(left, right)| (left - right).abs())
        .sum()
}

fn validate_transition_matrix(matrix: &[Vec<f64>]) -> Result<(), PopulationError> {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err(PopulationError::InvalidTransitionMatrix);
    }
    for row in matrix {
        if row.iter().any(|value| !value.is_finite() || *value < 0.0) {
            return Err(PopulationError::InvalidTransitionMatrix);
        }
        if (row.iter().sum::<f64>() - 1.0).abs() > 1e-10 {
            return Err(PopulationError::InvalidTransitionMatrix);
        }
    }
    Ok(())
}

fn log_sum_exp(values: &[f64]) -> f64 {
    let maximum = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !maximum.is_finite() {
        return maximum;
    }
    maximum
        + values
            .iter()
            .map(|value| (value - maximum).exp())
            .sum::<f64>()
            .ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(risk: f64) -> GameConfig {
        GameConfig {
            catastrophe_probability: risk,
        }
    }

    #[test]
    fn n_six_has_exactly_one_table_composition() {
        let population = PopulationState::balanced(6).unwrap();
        for strategy in Strategy::ALL {
            let compositions = group_compositions(population, strategy).unwrap();
            assert_eq!(compositions.len(), 1);
            assert!((compositions[0].probability - 1.0).abs() < 1e-12);
            assert_eq!(
                compositions[0].tablemate_counts.iter().sum::<usize>(),
                TABLEMATE_COUNT
            );
        }
    }

    #[test]
    fn two_type_hypergeometric_example_matches_hand_calculation() {
        let population = PopulationState::new([2, 5, 0, 0, 0, 0]).unwrap();
        let compositions = group_compositions(population, Strategy::AlwaysZero).unwrap();
        assert_eq!(compositions.len(), 2);
        let zero_other_count = compositions
            .iter()
            .find(|composition| composition.tablemate_counts[0] == 0)
            .unwrap();
        let one_other_count = compositions
            .iter()
            .find(|composition| composition.tablemate_counts[0] == 1)
            .unwrap();
        assert!((zero_other_count.probability - 1.0 / 6.0).abs() < 1e-12);
        assert!((one_other_count.probability - 5.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn hypergeometric_mass_is_one_for_a_mixed_thirty_agent_population() {
        let population = PopulationState::balanced(30).unwrap();
        for strategy in Strategy::ALL {
            let sum = group_compositions(population, strategy)
                .unwrap()
                .iter()
                .map(|composition| composition.probability)
                .sum::<f64>();
            assert!((sum - 1.0).abs() < 1e-12);
        }
    }

    #[test]
    fn monomorphic_population_fitness_equals_six_agent_self_play() {
        for strategy in Strategy::ALL {
            let population = PopulationState::monomorphic(strategy, 6).unwrap();
            let fitness = expected_type_fitness(config(0.9), population, strategy).unwrap();
            let outcome = run_episode(config(0.9), &[strategy; SEATS]).unwrap();
            assert!((fitness.expected_payoff - outcome.expected_per_seat_welfare).abs() < 1e-12);
            assert!(
                (fitness.expected_group_welfare - outcome.expected_group_welfare).abs() < 1e-12
            );
        }
    }

    #[test]
    fn fermi_probabilities_conserve_mass() {
        let rates = fermi_transition_rates(30, 10, 8.0, 4.0, 1.0).unwrap();
        assert!(
            (rates.increase_probability + rates.decrease_probability + rates.stay_probability
                - 1.0)
                .abs()
                < 1e-12
        );
        assert!(rates.increase_probability > rates.decrease_probability);
    }

    #[test]
    fn neutral_fixation_is_exactly_one_over_population_size() {
        let probability = fixation_probability(
            config(0.9),
            6,
            Strategy::AlwaysZero,
            Strategy::FixedMaximum,
            0.0,
        )
        .unwrap();
        assert!((probability - 1.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn neutral_fixation_is_one_over_population_size_for_n_thirty() {
        let probability = fixation_probability(
            config(0.9),
            30,
            Strategy::AlwaysZero,
            Strategy::FixedMaximum,
            0.0,
        )
        .unwrap();
        assert!((probability - 1.0 / 30.0).abs() < 1e-12);
    }

    #[test]
    fn malicious_count_overflow_fails_closed() {
        let population = PopulationState {
            counts: [usize::MAX, 1, 0, 0, 0, 0],
        };
        assert_eq!(
            population.validate(),
            Err(PopulationError::InvalidPopulationSize(usize::MAX))
        );
    }

    #[test]
    fn advantaged_zero_contributor_invades_maximum_contributors() {
        let probability = fixation_probability(
            config(0.9),
            6,
            Strategy::AlwaysZero,
            Strategy::FixedMaximum,
            1.0,
        )
        .unwrap();
        assert!(probability > 1.0 / 6.0);
    }

    #[test]
    fn rare_mutation_chain_and_stationary_weights_are_valid() {
        let chain = rare_mutation_chain(config(0.9), 6, 1.0).unwrap();
        for row in &chain.transition_matrix {
            assert!((row.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        }
        let stationary = stationary_distribution(&chain, 1e-12, 100_000).unwrap();
        assert!(
            stationary
                .weights
                .iter()
                .all(|weight| weight.is_finite() && *weight >= 0.0)
        );
        assert!((stationary.weights.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!(stationary.residual_l1 <= 1e-10);
    }

    #[test]
    fn balanced_population_fitness_is_composition_weighted() {
        let population = PopulationState::balanced(30).unwrap();
        let zero = expected_type_fitness(config(0.9), population, Strategy::AlwaysZero).unwrap();
        let maximum =
            expected_type_fitness(config(0.9), population, Strategy::FixedMaximum).unwrap();
        assert!(zero.compositions_evaluated > 1);
        assert!(maximum.compositions_evaluated > 1);
        assert!(zero.expected_payoff.is_finite());
        assert!(maximum.expected_payoff.is_finite());
    }

    #[test]
    fn invasion_graph_detects_a_directed_three_cycle() {
        let neutral = -(6.0_f64).ln();
        let advantaged = 0.4_f64.ln();
        let mut matrix = vec![vec![f64::NEG_INFINITY; STRATEGY_COUNT]; STRATEGY_COUNT];
        for (resident, row) in matrix.iter_mut().enumerate() {
            for (mutant, value) in row.iter_mut().enumerate() {
                if resident != mutant {
                    *value = neutral;
                }
            }
        }
        // AlwaysZero -> FixedFairShare -> FixedMaximum -> AlwaysZero.
        matrix[0][1] = advantaged;
        matrix[1][2] = advantaged;
        matrix[2][0] = advantaged;

        let graph = invasion_graph(
            &RareMutationChain {
                transition_matrix: vec![vec![1.0 / 6.0; STRATEGY_COUNT]; STRATEGY_COUNT],
                log_fixation_matrix: matrix,
            },
            6,
        )
        .unwrap();

        assert_eq!(graph.edge_count, 3);
        assert_eq!(graph.three_cycles.len(), 1);
        assert_eq!(
            graph.three_cycles[0].strategies,
            [
                Strategy::AlwaysZero,
                Strategy::FixedFairShare,
                Strategy::FixedMaximum
            ]
        );
        assert_eq!(
            graph.three_cycles[0].orientation,
            InvasionCycleOrientation::Forward
        );
    }

    #[test]
    fn neutral_fixation_matrix_has_no_invasion_edges_or_cycles() {
        let neutral = -(30.0_f64).ln();
        let mut matrix = vec![vec![f64::NEG_INFINITY; STRATEGY_COUNT]; STRATEGY_COUNT];
        for (resident, row) in matrix.iter_mut().enumerate() {
            for (mutant, value) in row.iter_mut().enumerate() {
                if resident != mutant {
                    *value = neutral;
                }
            }
        }
        let graph = invasion_graph(
            &RareMutationChain {
                transition_matrix: vec![vec![1.0 / 6.0; STRATEGY_COUNT]; STRATEGY_COUNT],
                log_fixation_matrix: matrix,
            },
            30,
        )
        .unwrap();
        assert_eq!(graph.edge_count, 0);
        assert!(graph.three_cycles.is_empty());
    }

    #[test]
    fn invalid_selection_inputs_fail_closed() {
        assert!(matches!(
            fermi_transition_rates(30, 0, 1.0, 1.0, 1.0),
            Err(PopulationError::InvalidMutantCount { .. })
        ));
        assert!(matches!(
            fermi_transition_rates(30, 1, 1.0, 1.0, f64::NAN),
            Err(PopulationError::InvalidSelectionIntensity(_))
        ));
    }
}
