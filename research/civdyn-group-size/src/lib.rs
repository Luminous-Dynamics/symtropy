// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Finite-memory local-coordination population dynamics.
//!
//! The engine accepts an explicit policy table P(option 0 | own/partner
//! interaction history). Synthetic policies are software controls, not
//! extracted LLM policies or human behavioural models.

use core::fmt;
use std::collections::BTreeMap;

pub const DEFAULT_MEMORY_CAPACITY: usize = 5;
pub const DEFAULT_MATCH_PAYOFF: i64 = 100;
pub const DEFAULT_MISMATCH_PAYOFF: i64 = -50;
pub const DEFAULT_CONVERGENCE_WINDOW: usize = 3;
pub const DEFAULT_CONVERGENCE_THRESHOLD: f64 = 0.98;
pub const DEFAULT_MAX_POPULATION_ROUNDS: usize = 1_000;
pub const MAX_MEMORY_CAPACITY: usize = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryState {
    len: u8,
    /// Ordered two-bit (own, partner) pairs; oldest pair occupies the high
    /// used bits and the newest pair occupies the low two bits.
    packed_pairs: u64,
}

impl MemoryState {
    pub const fn empty() -> Self {
        Self {
            len: 0,
            packed_pairs: 0,
        }
    }

    pub fn from_id(id: usize, memory_capacity: usize) -> Result<Self, GroupSizeError> {
        let total = memory_state_count(memory_capacity)?;
        if id >= total {
            return Err(GroupSizeError::InvalidMemoryStateId {
                id,
                state_count: total,
            });
        }

        let mut offset = 0usize;
        let mut span = 1usize;
        for len in 0..=memory_capacity {
            let end = offset
                .checked_add(span)
                .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
            if id < end {
                return Ok(Self {
                    len: len as u8,
                    packed_pairs: (id - offset) as u64,
                });
            }
            offset = end;
            span = span
                .checked_mul(4)
                .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
        }
        Err(GroupSizeError::InvalidMemoryStateId {
            id,
            state_count: total,
        })
    }

    pub fn to_id(self, memory_capacity: usize) -> Result<usize, GroupSizeError> {
        validate_memory_capacity(memory_capacity)?;
        let len = usize::from(self.len);
        if len > memory_capacity {
            return Err(GroupSizeError::MemoryLengthExceedsCapacity {
                length: len,
                capacity: memory_capacity,
            });
        }
        let used_bits = 2 * len;
        if used_bits < 64 && self.packed_pairs >= (1u64 << used_bits) {
            return Err(GroupSizeError::InvalidPackedMemoryState);
        }
        let offset = memory_state_offset(len)?;
        offset
            .checked_add(self.packed_pairs as usize)
            .ok_or(GroupSizeError::MemoryStateCountOverflow)
    }

    pub fn observe(
        self,
        own_action: u8,
        partner_action: u8,
        memory_capacity: usize,
    ) -> Result<Self, GroupSizeError> {
        validate_memory_capacity(memory_capacity)?;
        validate_action(own_action)?;
        validate_action(partner_action)?;

        let len = usize::from(self.len);
        if len > memory_capacity {
            return Err(GroupSizeError::MemoryLengthExceedsCapacity {
                length: len,
                capacity: memory_capacity,
            });
        }
        if memory_capacity == 0 {
            return Ok(Self::empty());
        }

        let pair = (u64::from(own_action) << 1) | u64::from(partner_action);
        let capacity_bits = 2 * memory_capacity;
        let mask = (1u64 << capacity_bits) - 1;
        let packed_pairs = ((self.packed_pairs << 2) | pair) & mask;
        let new_len = (len + 1).min(memory_capacity);

        Ok(Self {
            len: new_len as u8,
            packed_pairs,
        })
    }

    pub fn len(self) -> usize {
        usize::from(self.len)
    }

    pub fn pair_at(self, index: usize) -> Option<(u8, u8)> {
        if index >= self.len() {
            return None;
        }
        let shift = 2 * (self.len() - index - 1);
        let pair = ((self.packed_pairs >> shift) & 0b11) as u8;
        Some(((pair >> 1) & 1, pair & 1))
    }

    pub fn partner_majority(self) -> Option<u8> {
        if self.len() == 0 {
            return None;
        }
        let mut zeros = 0usize;
        let mut ones = 0usize;
        for index in 0..self.len() {
            match self
                .pair_at(index)
                .expect("index is within memory length")
                .1
            {
                0 => zeros += 1,
                1 => ones += 1,
                _ => unreachable!("binary action validation guarantees 0 or 1"),
            }
        }
        match zeros.cmp(&ones) {
            std::cmp::Ordering::Greater => Some(0),
            std::cmp::Ordering::Less => Some(1),
            std::cmp::Ordering::Equal => None,
        }
    }
}

pub fn memory_state_count(memory_capacity: usize) -> Result<usize, GroupSizeError> {
    validate_memory_capacity(memory_capacity)?;
    let mut total = 0usize;
    let mut span = 1usize;
    for _ in 0..=memory_capacity {
        total = total
            .checked_add(span)
            .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
        span = span
            .checked_mul(4)
            .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
    }
    Ok(total)
}

fn memory_state_offset(length: usize) -> Result<usize, GroupSizeError> {
    let mut offset = 0usize;
    let mut span = 1usize;
    for _ in 0..length {
        offset = offset
            .checked_add(span)
            .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
        span = span
            .checked_mul(4)
            .ok_or(GroupSizeError::MemoryStateCountOverflow)?;
    }
    Ok(offset)
}

fn validate_memory_capacity(memory_capacity: usize) -> Result<(), GroupSizeError> {
    if memory_capacity > MAX_MEMORY_CAPACITY {
        return Err(GroupSizeError::MemoryCapacityTooLarge {
            requested: memory_capacity,
            maximum: MAX_MEMORY_CAPACITY,
        });
    }
    Ok(())
}

fn validate_action(action: u8) -> Result<(), GroupSizeError> {
    if action > 1 {
        return Err(GroupSizeError::InvalidAction(action));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyTable {
    pub policy_id: String,
    pub memory_capacity: usize,
    pub convention_order: [String; 2],
    probabilities_option0: Vec<f64>,
}

impl PolicyTable {
    pub fn new(
        policy_id: impl Into<String>,
        memory_capacity: usize,
        convention_order: [String; 2],
        probabilities_option0: Vec<f64>,
    ) -> Result<Self, GroupSizeError> {
        let table = Self {
            policy_id: policy_id.into(),
            memory_capacity,
            convention_order,
            probabilities_option0,
        };
        table.validate()?;
        Ok(table)
    }

    pub fn validate(&self) -> Result<(), GroupSizeError> {
        if self.policy_id.trim().is_empty() {
            return Err(GroupSizeError::EmptyPolicyId);
        }
        validate_memory_capacity(self.memory_capacity)?;
        for (index, option) in self.convention_order.iter().enumerate() {
            if option.trim().is_empty() {
                return Err(GroupSizeError::EmptyConventionLabel(index));
            }
        }
        if self.convention_order[0] == self.convention_order[1] {
            return Err(GroupSizeError::DuplicateConventionLabel);
        }
        let expected = memory_state_count(self.memory_capacity)?;
        if self.probabilities_option0.len() != expected {
            return Err(GroupSizeError::PolicyTableLengthMismatch {
                expected,
                actual: self.probabilities_option0.len(),
            });
        }
        for (state_id, probability) in self.probabilities_option0.iter().enumerate() {
            if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                return Err(GroupSizeError::InvalidPolicyProbability {
                    state_id,
                    value: *probability,
                });
            }
        }
        Ok(())
    }

    pub fn probability_option0(&self, state_id: usize) -> Result<f64, GroupSizeError> {
        self.probabilities_option0.get(state_id).copied().ok_or(
            GroupSizeError::InvalidMemoryStateId {
                id: state_id,
                state_count: self.probabilities_option0.len(),
            },
        )
    }

    pub fn empty_memory_probability_option0(&self) -> f64 {
        self.probabilities_option0[0]
    }

    pub fn state_count(&self) -> usize {
        self.probabilities_option0.len()
    }

    pub fn probabilities_option0(&self) -> &[f64] {
        &self.probabilities_option0
    }

    /// Returns the shared action probability when every policy-table state
    /// has exactly the same value. Otherwise the policy is memory-dependent.
    pub fn memory_independent_probability_option0(&self) -> Option<f64> {
        let first = *self.probabilities_option0.first()?;
        self.probabilities_option0
            .iter()
            .all(|probability| *probability == first)
            .then_some(first)
    }

    pub fn memory_independent(
        policy_id: impl Into<String>,
        memory_capacity: usize,
        convention_order: [String; 2],
        probability_option0: f64,
    ) -> Result<Self, GroupSizeError> {
        if !probability_option0.is_finite() || !(0.0..=1.0).contains(&probability_option0) {
            return Err(GroupSizeError::InvalidPolicyProbability {
                state_id: 0,
                value: probability_option0,
            });
        }
        let state_count = memory_state_count(memory_capacity)?;
        Self::new(
            policy_id,
            memory_capacity,
            convention_order,
            vec![probability_option0; state_count],
        )
    }

    pub fn copy_partner_majority(
        policy_id: impl Into<String>,
        memory_capacity: usize,
        convention_order: [String; 2],
        empty_memory_probability_option0: f64,
        tie_probability_option0: f64,
        invert_majority: bool,
    ) -> Result<Self, GroupSizeError> {
        for (state_id, probability) in [
            (0, empty_memory_probability_option0),
            (1, tie_probability_option0),
        ] {
            if !probability.is_finite() || !(0.0..=1.0).contains(&probability) {
                return Err(GroupSizeError::InvalidPolicyProbability {
                    state_id,
                    value: probability,
                });
            }
        }

        let state_count = memory_state_count(memory_capacity)?;
        let mut probabilities = Vec::with_capacity(state_count);
        for state_id in 0..state_count {
            let state = MemoryState::from_id(state_id, memory_capacity)?;
            let probability = match state.partner_majority() {
                None if state.len() == 0 => empty_memory_probability_option0,
                None => tie_probability_option0,
                Some(majority) => {
                    let selected = if invert_majority {
                        1 - majority
                    } else {
                        majority
                    };
                    if selected == 0 { 1.0 } else { 0.0 }
                }
            };
            probabilities.push(probability);
        }
        Self::new(policy_id, memory_capacity, convention_order, probabilities)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameConfig {
    pub population_size: usize,
    pub memory_capacity: usize,
    pub max_population_rounds: usize,
    pub convergence_window: usize,
    pub convergence_threshold: f64,
    pub match_payoff: i64,
    pub mismatch_payoff: i64,
}

impl GameConfig {
    pub fn validate(self) -> Result<(), GroupSizeError> {
        if self.population_size < 2 {
            return Err(GroupSizeError::PopulationTooSmall(self.population_size));
        }
        validate_memory_capacity(self.memory_capacity)?;
        if self.max_population_rounds == 0 {
            return Err(GroupSizeError::ZeroPopulationRounds);
        }
        if self.convergence_window == 0 {
            return Err(GroupSizeError::ZeroConvergenceWindow);
        }
        if !self.convergence_threshold.is_finite()
            || !(0.0..=1.0).contains(&self.convergence_threshold)
        {
            return Err(GroupSizeError::InvalidConvergenceThreshold(
                self.convergence_threshold,
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RoundMetrics {
    pub population_round: usize,
    /// Exactly N ordered, distinct-agent pairwise encounters.
    pub pairwise_interactions: usize,
    pub matching_interactions: usize,
    pub agreement_rate: f64,
    pub option0_actions: usize,
    pub option1_actions: usize,
    /// Fraction of all 2N emitted actions that selected convention 0.
    pub option0_action_share: f64,
    pub total_payoff_added: i64,
    pub mean_cumulative_agent_payoff: f64,
    pub minimum_cumulative_agent_payoff: i64,
    pub maximum_cumulative_agent_payoff: i64,
    pub unique_memory_states: usize,
    pub memory_state_entropy_bits: f64,
    /// True only if every one of the 2N actions in this population round is equal.
    pub action_unanimity: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOutcome {
    pub policy_id: String,
    pub population_size: usize,
    pub memory_capacity: usize,
    pub seed: u64,
    pub rounds: Vec<RoundMetrics>,
    pub total_pairwise_interactions: usize,
    pub agreement_converged: bool,
    pub agreement_convergence_round: Option<usize>,
    /// Convention inferred from the mean option-0 action share across the
    /// convergence window. This is not an absorbing-state certificate.
    pub convergence_window_convention: Option<u8>,
    pub true_action_unanimity_round: Option<usize>,
    pub final_option0_action_share: f64,
    pub final_agreement_rate: f64,
    pub final_mean_agent_payoff: f64,
    pub final_minimum_agent_payoff: i64,
    pub final_maximum_agent_payoff: i64,
    pub final_unique_memory_states: usize,
    pub final_memory_state_entropy_bits: f64,
}

impl RunOutcome {
    pub fn rounds_run(&self) -> usize {
        self.rounds.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MemoryIndependentOracle {
    pub expected_option0_action_share: f64,
    pub expected_pairwise_agreement_rate: f64,
    /// Per-round population sampling standard deviation of the option-0 share.
    pub option0_action_share_standard_deviation: f64,
    /// Per-round standard deviation of the N pairwise match indicators.
    pub pairwise_agreement_standard_deviation: f64,
}

/// Exact one-population-round expectation for a homogeneous policy whose
/// action probability is independent of memory. The per-action draws and
/// pairwise matches are independent under this restricted policy family.
pub fn memory_independent_oracle(
    probability_option0: f64,
    population_size: usize,
) -> Result<MemoryIndependentOracle, GroupSizeError> {
    if population_size < 2 {
        return Err(GroupSizeError::PopulationTooSmall(population_size));
    }
    if !probability_option0.is_finite() || !(0.0..=1.0).contains(&probability_option0) {
        return Err(GroupSizeError::InvalidPolicyProbability {
            state_id: 0,
            value: probability_option0,
        });
    }
    let probability_option1 = 1.0 - probability_option0;
    let agreement = probability_option0.powi(2) + probability_option1.powi(2);
    let action_share_variance =
        probability_option0 * probability_option1 / (2.0 * population_size as f64);
    let agreement_variance = agreement * (1.0 - agreement) / population_size as f64;

    Ok(MemoryIndependentOracle {
        expected_option0_action_share: probability_option0,
        expected_pairwise_agreement_rate: agreement,
        option0_action_share_standard_deviation: action_share_variance.sqrt(),
        pairwise_agreement_standard_deviation: agreement_variance.sqrt(),
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum GroupSizeError {
    MemoryCapacityTooLarge { requested: usize, maximum: usize },
    MemoryStateCountOverflow,
    InvalidMemoryStateId { id: usize, state_count: usize },
    MemoryLengthExceedsCapacity { length: usize, capacity: usize },
    InvalidPackedMemoryState,
    InvalidAction(u8),
    EmptyPolicyId,
    EmptyConventionLabel(usize),
    DuplicateConventionLabel,
    PolicyTableLengthMismatch { expected: usize, actual: usize },
    InvalidPolicyProbability { state_id: usize, value: f64 },
    PopulationTooSmall(usize),
    ZeroPopulationRounds,
    ZeroConvergenceWindow,
    InvalidConvergenceThreshold(f64),
    PolicyMemoryCapacityMismatch { config: usize, policy: usize },
    PayoffOverflow,
}

impl fmt::Display for GroupSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemoryCapacityTooLarge { requested, maximum } => write!(
                f,
                "memory capacity {requested} exceeds supported maximum {maximum}"
            ),
            Self::MemoryStateCountOverflow => write!(f, "memory state count overflow"),
            Self::InvalidMemoryStateId { id, state_count } => {
                write!(f, "memory state ID {id} is outside 0..{state_count}")
            }
            Self::MemoryLengthExceedsCapacity { length, capacity } => write!(
                f,
                "memory length {length} exceeds configured capacity {capacity}"
            ),
            Self::InvalidPackedMemoryState => {
                write!(f, "packed memory state uses bits outside its length")
            }
            Self::InvalidAction(action) => {
                write!(f, "binary convention action must be 0 or 1, got {action}")
            }
            Self::EmptyPolicyId => write!(f, "policy ID must not be empty"),
            Self::EmptyConventionLabel(index) => {
                write!(f, "convention label {index} must not be empty")
            }
            Self::DuplicateConventionLabel => {
                write!(f, "the two convention labels must be distinct")
            }
            Self::PolicyTableLengthMismatch { expected, actual } => write!(
                f,
                "policy table has {actual} rows; memory contract requires {expected}"
            ),
            Self::InvalidPolicyProbability { state_id, value } => write!(
                f,
                "policy probability at state {state_id} must be finite and in [0,1], got {value}"
            ),
            Self::PopulationTooSmall(size) => {
                write!(f, "population size must be at least two, got {size}")
            }
            Self::ZeroPopulationRounds => write!(f, "max population rounds must be positive"),
            Self::ZeroConvergenceWindow => write!(f, "convergence window must be positive"),
            Self::InvalidConvergenceThreshold(value) => write!(
                f,
                "convergence threshold must be finite and in [0,1], got {value}"
            ),
            Self::PolicyMemoryCapacityMismatch { config, policy } => write!(
                f,
                "config memory capacity {config} does not match policy capacity {policy}"
            ),
            Self::PayoffOverflow => write!(f, "cumulative or aggregate payoff overflow"),
        }
    }
}

impl std::error::Error for GroupSizeError {}

#[derive(Debug, Clone)]
struct AgentState {
    memory: MemoryState,
    cumulative_payoff: i64,
}

pub fn simulate_population(
    config: GameConfig,
    policy: &PolicyTable,
    seed: u64,
) -> Result<RunOutcome, GroupSizeError> {
    config.validate()?;
    policy.validate()?;
    if config.memory_capacity != policy.memory_capacity {
        return Err(GroupSizeError::PolicyMemoryCapacityMismatch {
            config: config.memory_capacity,
            policy: policy.memory_capacity,
        });
    }

    let population_size = config.population_size;
    let mut agents = vec![
        AgentState {
            memory: MemoryState::empty(),
            cumulative_payoff: 0,
        };
        population_size
    ];
    let mut rng = SplitMix64::new(seed);
    let mut rounds = Vec::with_capacity(config.max_population_rounds);
    let mut rolling_agreement = Vec::with_capacity(config.convergence_window);
    let mut rolling_option0_share = Vec::with_capacity(config.convergence_window);
    let mut agreement_convergence_round = None;
    let mut convergence_window_convention = None;
    let mut true_action_unanimity_round = None;

    for round_index in 0..config.max_population_rounds {
        let mut matching_interactions = 0usize;
        let mut option0_actions = 0usize;
        let mut total_payoff_added = 0i64;
        let mut all_actions_same = true;
        let mut first_action_in_round = None;

        for _ in 0..population_size {
            let first_agent = rng.below(population_size);
            let mut second_agent = rng.below(population_size - 1);
            if second_agent >= first_agent {
                second_agent += 1;
            }

            let first_state_id = agents[first_agent].memory.to_id(policy.memory_capacity)?;
            let second_state_id = agents[second_agent].memory.to_id(policy.memory_capacity)?;
            let first_probability = policy.probability_option0(first_state_id)?;
            let second_probability = policy.probability_option0(second_state_id)?;
            let first_action = u8::from(rng.unit_f64() >= first_probability);
            let second_action = u8::from(rng.unit_f64() >= second_probability);

            option0_actions += usize::from(first_action == 0);
            option0_actions += usize::from(second_action == 0);
            if first_action == second_action {
                matching_interactions += 1;
            } else {
                all_actions_same = false;
            }
            match first_action_in_round {
                None => first_action_in_round = Some(first_action),
                Some(previous) if previous != first_action => all_actions_same = false,
                Some(_) => {}
            }
            let payoff = if first_action == second_action {
                config.match_payoff
            } else {
                config.mismatch_payoff
            };
            agents[first_agent].cumulative_payoff = agents[first_agent]
                .cumulative_payoff
                .checked_add(payoff)
                .ok_or(GroupSizeError::PayoffOverflow)?;
            agents[second_agent].cumulative_payoff = agents[second_agent]
                .cumulative_payoff
                .checked_add(payoff)
                .ok_or(GroupSizeError::PayoffOverflow)?;
            let pair_payoff = payoff.checked_mul(2).ok_or(GroupSizeError::PayoffOverflow)?;
            total_payoff_added = total_payoff_added
                .checked_add(pair_payoff)
                .ok_or(GroupSizeError::PayoffOverflow)?;

            let first_new_memory = agents[first_agent].memory.observe(
                first_action,
                second_action,
                policy.memory_capacity,
            )?;
            let second_new_memory = agents[second_agent].memory.observe(
                second_action,
                first_action,
                policy.memory_capacity,
            )?;
            agents[first_agent].memory = first_new_memory;
            agents[second_agent].memory = second_new_memory;
        }

        let agreement_rate = matching_interactions as f64 / population_size as f64;
        let option0_action_share = option0_actions as f64 / (2 * population_size) as f64;
        let payoffs = agents
            .iter()
            .map(|agent| agent.cumulative_payoff)
            .collect::<Vec<_>>();
        let mean_cumulative_agent_payoff =
            payoffs.iter().map(|value| *value as f64).sum::<f64>() / population_size as f64;
        let minimum_cumulative_agent_payoff = payoffs.iter().copied().min().unwrap_or(0);
        let maximum_cumulative_agent_payoff = payoffs.iter().copied().max().unwrap_or(0);
        let (unique_memory_states, memory_state_entropy_bits) =
            memory_distribution_metrics(&agents, policy.memory_capacity)?;

        rounds.push(RoundMetrics {
            population_round: round_index + 1,
            pairwise_interactions: population_size,
            matching_interactions,
            agreement_rate,
            option0_actions,
            option1_actions: 2 * population_size - option0_actions,
            option0_action_share,
            total_payoff_added,
            mean_cumulative_agent_payoff,
            minimum_cumulative_agent_payoff,
            maximum_cumulative_agent_payoff,
            unique_memory_states,
            memory_state_entropy_bits,
            action_unanimity: all_actions_same,
        });

        rolling_agreement.push(agreement_rate);
        rolling_option0_share.push(option0_action_share);
        if rolling_agreement.len() > config.convergence_window {
            rolling_agreement.remove(0);
            rolling_option0_share.remove(0);
        }

        if true_action_unanimity_round.is_none()
            && rolling_action_unanimity(&rounds, config.convergence_window)
        {
            true_action_unanimity_round = Some(round_index + 1);
        }

        if agreement_convergence_round.is_none()
            && rolling_agreement.len() == config.convergence_window
        {
            let mean_agreement =
                rolling_agreement.iter().sum::<f64>() / config.convergence_window as f64;
            if mean_agreement >= config.convergence_threshold {
                agreement_convergence_round = Some(round_index + 1);
                let mean_option0 =
                    rolling_option0_share.iter().sum::<f64>() / config.convergence_window as f64;
                convergence_window_convention = Some(u8::from(mean_option0 < 0.5));
                break;
            }
        }
    }

    let last = rounds.last().expect("positive population-round config");
    let total_pairwise_interactions = rounds
        .iter()
        .try_fold(0usize, |total, row| total.checked_add(row.pairwise_interactions))
        .ok_or(GroupSizeError::PayoffOverflow)?;
    Ok(RunOutcome {
        policy_id: policy.policy_id.clone(),
        population_size,
        memory_capacity: policy.memory_capacity,
        seed,
        rounds,
        total_pairwise_interactions,
        agreement_converged: agreement_convergence_round.is_some(),
        agreement_convergence_round,
        convergence_window_convention,
        true_action_unanimity_round,
        final_option0_action_share: last.option0_action_share,
        final_agreement_rate: last.agreement_rate,
        final_mean_agent_payoff: last.mean_cumulative_agent_payoff,
        final_minimum_agent_payoff: last.minimum_cumulative_agent_payoff,
        final_maximum_agent_payoff: last.maximum_cumulative_agent_payoff,
        final_unique_memory_states: last.unique_memory_states,
        final_memory_state_entropy_bits: last.memory_state_entropy_bits,
    })
}

fn rolling_action_unanimity(rounds: &[RoundMetrics], window: usize) -> bool {
    if rounds.len() < window {
        return false;
    }
    rounds[rounds.len() - window..]
        .iter()
        .all(|round| round.action_unanimity)
}

fn memory_distribution_metrics(
    agents: &[AgentState],
    memory_capacity: usize,
) -> Result<(usize, f64), GroupSizeError> {
    let mut counts = BTreeMap::<usize, usize>::new();
    for agent in agents {
        let state_id = agent.memory.to_id(memory_capacity)?;
        *counts.entry(state_id).or_default() += 1;
    }
    let unique = counts.len();
    let entropy = counts
        .values()
        .map(|count| {
            let probability = *count as f64 / agents.len() as f64;
            -probability * probability.log2()
        })
        .sum();
    Ok((unique, entropy))
}

#[derive(Debug, Clone)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D049BB133111EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, upper_bound: usize) -> usize {
        assert!(upper_bound > 0);
        let bound = upper_bound as u64;
        let limit = u64::MAX - (u64::MAX % bound);
        loop {
            let candidate = self.next_u64();
            if candidate < limit {
                return (candidate % bound) as usize;
            }
        }
    }

    fn unit_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / ((1u64 << 53) as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order() -> [String; 2] {
        ["convention_a".into(), "convention_b".into()]
    }

    fn config(population_size: usize, rounds: usize) -> GameConfig {
        GameConfig {
            population_size,
            memory_capacity: 5,
            max_population_rounds: rounds,
            convergence_window: 3,
            convergence_threshold: 0.98,
            match_payoff: DEFAULT_MATCH_PAYOFF,
            mismatch_payoff: DEFAULT_MISMATCH_PAYOFF,
        }
    }

    #[test]
    fn five_interaction_memory_has_1365_distinct_states() {
        assert_eq!(memory_state_count(5).unwrap(), 1_365);
    }

    #[test]
    fn empty_memory_and_short_all_zero_histories_are_distinct() {
        let empty = MemoryState::empty();
        let one_zero = empty.observe(0, 0, 5).unwrap();
        let two_zeros = one_zero.observe(0, 0, 5).unwrap();
        assert_eq!(empty.to_id(5).unwrap(), 0);
        assert_ne!(empty.to_id(5).unwrap(), one_zero.to_id(5).unwrap());
        assert_ne!(one_zero.to_id(5).unwrap(), two_zeros.to_id(5).unwrap());
    }

    #[test]
    fn every_canonical_state_round_trips() {
        let count = memory_state_count(5).unwrap();
        for state_id in 0..count {
            let state = MemoryState::from_id(state_id, 5).unwrap();
            assert_eq!(state.to_id(5).unwrap(), state_id);
        }
    }

    #[test]
    fn memory_appends_ordered_pairs_and_drops_only_the_oldest() {
        let mut state = MemoryState::empty();
        for (own, partner) in [(0, 1), (1, 0), (1, 1), (0, 0), (1, 0)] {
            state = state.observe(own, partner, 5).unwrap();
        }
        assert_eq!(state.len(), 5);
        assert_eq!(state.pair_at(0), Some((0, 1)));
        assert_eq!(state.pair_at(4), Some((1, 0)));
        state = state.observe(0, 0, 5).unwrap();
        assert_eq!(state.pair_at(0), Some((1, 0)));
        assert_eq!(state.pair_at(4), Some((0, 0)));
    }

    #[test]
    fn partner_majority_uses_only_the_partner_history() {
        let mut state = MemoryState::empty();
        state = state.observe(1, 0, 5).unwrap();
        state = state.observe(1, 0, 5).unwrap();
        state = state.observe(0, 1, 5).unwrap();
        assert_eq!(state.partner_majority(), Some(0));
    }

    #[test]
    fn policy_table_rejects_missing_states_and_invalid_probabilities() {
        assert!(matches!(
            PolicyTable::new("bad", 5, order(), vec![0.5; 1_024]),
            Err(GroupSizeError::PolicyTableLengthMismatch {
                expected: 1_365,
                actual: 1_024
            })
        ));
        let mut probabilities = vec![0.5; 1_365];
        probabilities[17] = f64::NAN;
        assert!(matches!(
            PolicyTable::new("bad", 5, order(), probabilities),
            Err(GroupSizeError::InvalidPolicyProbability { state_id: 17, .. })
        ));
    }

    #[test]
    fn memory_independent_mean_field_oracle_includes_finite_population_variance() {
        let oracle = memory_independent_oracle(0.7, 100).unwrap();
        assert!((oracle.expected_option0_action_share - 0.7).abs() < 1e-12);
        assert!((oracle.expected_pairwise_agreement_rate - 0.58).abs() < 1e-12);
        assert!((oracle.option0_action_share_standard_deviation
            - (0.7_f64 * 0.3 / 200.0).sqrt())
            .abs() < 1e-12);
        assert!((oracle.pairwise_agreement_standard_deviation
            - (0.58_f64 * 0.42 / 100.0).sqrt())
            .abs() < 1e-12);
    }

    #[test]
    fn policy_table_identifies_memory_independent_and_contextual_policies() {
        let constant = PolicyTable::memory_independent("constant", 5, order(), 0.55).unwrap();
        assert_eq!(constant.memory_independent_probability_option0(), Some(0.55));

        let copy =
            PolicyTable::copy_partner_majority("copy", 5, order(), 0.5, 0.5, false).unwrap();
        assert_eq!(copy.memory_independent_probability_option0(), None);
    }

    #[test]
    fn always_option_zero_converges_after_three_rounds_and_is_true_action_unanimity() {
        let policy = PolicyTable::memory_independent("always_option_0", 5, order(), 1.0).unwrap();
        let result = simulate_population(config(10, 50), &policy, 7).unwrap();
        assert_eq!(result.agreement_convergence_round, Some(3));
        assert_eq!(result.true_action_unanimity_round, Some(3));
        assert_eq!(result.convergence_window_convention, Some(0));
        assert_eq!(result.total_pairwise_interactions, 30);
        assert_eq!(result.final_option0_action_share, 1.0);
    }

    #[test]
    fn always_option_one_converges_to_the_other_convention() {
        let policy = PolicyTable::memory_independent("always_option_1", 5, order(), 0.0).unwrap();
        let result = simulate_population(config(10, 50), &policy, 8).unwrap();
        assert_eq!(result.agreement_convergence_round, Some(3));
        assert_eq!(result.true_action_unanimity_round, Some(3));
        assert_eq!(result.convergence_window_convention, Some(1));
        assert_eq!(result.final_option0_action_share, 0.0);
    }

    #[test]
    fn same_seed_and_policy_table_replay_identically() {
        let policy =
            PolicyTable::copy_partner_majority("copy", 5, order(), 0.5, 0.5, false).unwrap();
        let first = simulate_population(config(24, 100), &policy, 1234).unwrap();
        let second = simulate_population(config(24, 100), &policy, 1234).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn a_different_seed_changes_a_stochastic_fixture_trace() {
        let policy = PolicyTable::memory_independent("neutral", 5, order(), 0.5).unwrap();
        let first = simulate_population(config(24, 25), &policy, 11).unwrap();
        let second = simulate_population(config(24, 25), &policy, 12).unwrap();
        assert_ne!(first.rounds, second.rounds);
    }

    #[test]
    fn each_population_round_has_exactly_n_pairwise_interactions() {
        let policy =
            PolicyTable::memory_independent("neutral", 5, order(), 0.5).unwrap();
        let result = simulate_population(config(24, 17), &policy, 42).unwrap();
        assert!(
            result
                .rounds
                .iter()
                .all(|row| row.pairwise_interactions == 24)
        );
        assert_eq!(result.total_pairwise_interactions, 24 * result.rounds_run());
    }

    #[test]
    fn overflowing_payoff_accounting_fails_closed() {
        let policy = PolicyTable::memory_independent("always_match", 5, order(), 1.0).unwrap();
        let mut invalid = config(2, 1);
        invalid.match_payoff = i64::MAX;
        assert!(matches!(
            simulate_population(invalid, &policy, 17),
            Err(GroupSizeError::PayoffOverflow)
        ));
    }

    #[test]
    fn mismatched_config_and_policy_memory_capacities_fail_closed() {
        let policy = PolicyTable::memory_independent("policy_h5", 5, order(), 0.5).unwrap();
        let mut invalid = config(10, 20);
        invalid.memory_capacity = 4;
        assert!(matches!(
            simulate_population(invalid, &policy, 1),
            Err(GroupSizeError::PolicyMemoryCapacityMismatch {
                config: 4,
                policy: 5
            })
        ));
    }

    #[test]
    fn memory_capacity_zero_has_one_state_and_never_accumulates_memory() {
        assert_eq!(memory_state_count(0).unwrap(), 1);
        let empty = MemoryState::empty();
        assert_eq!(empty.observe(1, 0, 0).unwrap(), empty);
        let policy = PolicyTable::memory_independent("zero_memory", 0, order(), 0.4).unwrap();
        assert_eq!(policy.state_count(), 1);
    }

    #[test]
    fn memory_independent_simulation_matches_expected_action_bias_within_sampling_error() {
        let p0 = 0.7;
        let policy = PolicyTable::memory_independent("biased", 5, order(), p0).unwrap();
        let result = simulate_population(config(100, 200), &policy, 2026).unwrap();
        let expected_agreement = p0 * p0 + (1.0 - p0) * (1.0 - p0);
        let final_share = result
            .rounds
            .iter()
            .skip(50)
            .map(|row| row.option0_action_share)
            .sum::<f64>()
            / (result.rounds.len().saturating_sub(50).max(1)) as f64;
        let final_agreement = result
            .rounds
            .iter()
            .skip(50)
            .map(|row| row.agreement_rate)
            .sum::<f64>()
            / (result.rounds.len().saturating_sub(50).max(1)) as f64;
        assert!((final_share - p0).abs() < 0.04);
        assert!((final_agreement - expected_agreement).abs() < 0.04);
    }
}
