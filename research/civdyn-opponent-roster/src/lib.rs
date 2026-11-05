// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Opponent-roster-dependent meta-game evaluation.
//!
//! A scalar rank is only meaningful relative to a declared opponent roster.
//! The kernel reports roster-conditional fitness, best-response exploitability,
//! pairwise cycles and an auditable distribution-error regret bound.

use core::fmt;
use std::collections::BTreeSet;

const TOLERANCE: f64 = 1e-12;
const PROBABILITY_TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, PartialEq)]
pub struct MetaGame {
    pub strategy_names: Vec<String>,
    /// Payoff to the row strategy when playing the column strategy.
    pub payoffs: Vec<Vec<f64>>,
}

impl MetaGame {
    pub fn validate(&self) -> Result<(), MetaGameError> {
        let count = self.strategy_names.len();
        if count < 3 {
            return Err(MetaGameError::TooFewStrategies(count));
        }
        let mut names = BTreeSet::new();
        for name in &self.strategy_names {
            if name.trim().is_empty() {
                return Err(MetaGameError::EmptyStrategyName);
            }
            if !names.insert(name) {
                return Err(MetaGameError::DuplicateStrategyName(name.clone()));
            }
        }
        if self.payoffs.len() != count {
            return Err(MetaGameError::PayoffRowCount {
                expected: count,
                actual: self.payoffs.len(),
            });
        }
        for (row, values) in self.payoffs.iter().enumerate() {
            if values.len() != count {
                return Err(MetaGameError::PayoffColumnCount {
                    row,
                    expected: count,
                    actual: values.len(),
                });
            }
            for (column, value) in values.iter().enumerate() {
                if !value.is_finite() {
                    return Err(MetaGameError::NonFinitePayoff {
                        row,
                        column,
                        value: *value,
                    });
                }
            }
        }
        Ok(())
    }

    pub fn payoff_range(&self) -> Result<f64, MetaGameError> {
        self.validate()?;
        let minimum = self
            .payoffs
            .iter()
            .flatten()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let maximum = self
            .payoffs
            .iter()
            .flatten()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        Ok(maximum - minimum)
    }

    pub fn invasion_graph(&self) -> Result<InvasionGraph, MetaGameError> {
        self.validate()?;
        let count = self.strategy_names.len();
        let mut edges = Vec::new();
        let mut adjacency = vec![vec![false; count]; count];

        for left in 0..count {
            for right in (left + 1)..count {
                let left_advantage = self.payoffs[left][right] - self.payoffs[right][left];
                if left_advantage > TOLERANCE {
                    adjacency[left][right] = true;
                    edges.push(InvasionEdge {
                        winner: left,
                        loser: right,
                        net_advantage: left_advantage,
                    });
                } else if left_advantage < -TOLERANCE {
                    adjacency[right][left] = true;
                    edges.push(InvasionEdge {
                        winner: right,
                        loser: left,
                        net_advantage: -left_advantage,
                    });
                }
            }
        }

        let mut three_cycles = Vec::new();
        for first in 0..count {
            for second in (first + 1)..count {
                for third in (second + 1)..count {
                    if adjacency[first][second]
                        && adjacency[second][third]
                        && adjacency[third][first]
                    {
                        three_cycles.push(ThreeCycle {
                            strategies: [first, second, third],
                            orientation: CycleOrientation::Forward,
                        });
                    }
                    if adjacency[first][third]
                        && adjacency[third][second]
                        && adjacency[second][first]
                    {
                        three_cycles.push(ThreeCycle {
                            strategies: [first, second, third],
                            orientation: CycleOrientation::Reverse,
                        });
                    }
                }
            }
        }

        Ok(InvasionGraph {
            edge_count: edges.len(),
            edges,
            three_cycles,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentRoster {
    pub roster_id: String,
    /// Number of opponents of each strategy, in strategy-name order.
    pub counts: Vec<usize>,
}

impl OpponentRoster {
    pub fn validate(&self, strategy_count: usize) -> Result<usize, MetaGameError> {
        if self.roster_id.trim().is_empty() {
            return Err(MetaGameError::EmptyRosterId);
        }
        if self.counts.len() != strategy_count {
            return Err(MetaGameError::RosterCountMismatch {
                expected: strategy_count,
                actual: self.counts.len(),
            });
        }
        let total = self.counts.iter().try_fold(0usize, |sum, count| {
            sum.checked_add(*count)
                .ok_or(MetaGameError::RosterSizeOverflow)
        })?;
        if total < 2 {
            return Err(MetaGameError::RosterTooSmall(total));
        }
        Ok(total)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StrategyOutcome {
    pub strategy_index: usize,
    pub strategy_name: String,
    pub expected_payoff: f64,
    pub best_response_payoff: f64,
    /// Difference between the best strategy against this roster and this strategy.
    pub exploitability: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetaGameEvaluation {
    pub roster_id: String,
    pub population_size: usize,
    /// Original strategy-name order.
    pub outcomes: Vec<StrategyOutcome>,
    /// Strategy indices ordered by expected payoff (descending, stable name tie-break).
    pub ranked_indices: Vec<usize>,
    pub invasion_graph: InvasionGraph,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvasionEdge {
    pub winner: usize,
    pub loser: usize,
    pub net_advantage: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleOrientation {
    Forward,
    Reverse,
}

impl CycleOrientation {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Reverse => "reverse",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreeCycle {
    /// Three strategy indices in increasing order. Orientation determines edges.
    pub strategies: [usize; 3],
    pub orientation: CycleOrientation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InvasionGraph {
    pub edge_count: usize,
    pub edges: Vec<InvasionEdge>,
    pub three_cycles: Vec<ThreeCycle>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RegretCertificate {
    pub predicted_best_response: usize,
    pub actual_best_response: usize,
    pub actual_regret: f64,
    pub opponent_distribution_l1_error: f64,
    /// For payoff range [m, M], the bound is (M - m) * ||p - q||_1.
    pub certified_upper_bound: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MetaGameError {
    TooFewStrategies(usize),
    EmptyStrategyName,
    DuplicateStrategyName(String),
    PayoffRowCount {
        expected: usize,
        actual: usize,
    },
    PayoffColumnCount {
        row: usize,
        expected: usize,
        actual: usize,
    },
    NonFinitePayoff {
        row: usize,
        column: usize,
        value: f64,
    },
    EmptyRosterId,
    RosterCountMismatch {
        expected: usize,
        actual: usize,
    },
    RosterSizeOverflow,
    RosterTooSmall(usize),
    DistributionLengthMismatch {
        expected: usize,
        actual: usize,
    },
    InvalidDistributionValue {
        index: usize,
        value: f64,
    },
    DistributionMassMismatch(f64),
    RosterStrategyMismatch,
}

impl fmt::Display for MetaGameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewStrategies(count) => {
                write!(f, "at least three strategies are required, got {count}")
            }
            Self::EmptyStrategyName => write!(f, "strategy names must not be empty"),
            Self::DuplicateStrategyName(name) => write!(f, "duplicate strategy name: {name}"),
            Self::PayoffRowCount { expected, actual } => {
                write!(f, "expected {expected} payoff rows, got {actual}")
            }
            Self::PayoffColumnCount {
                row,
                expected,
                actual,
            } => write!(
                f,
                "payoff row {row} expected {expected} columns, got {actual}"
            ),
            Self::NonFinitePayoff { row, column, value } => {
                write!(f, "payoff ({row}, {column}) is non-finite: {value}")
            }
            Self::EmptyRosterId => write!(f, "roster ID must not be empty"),
            Self::RosterCountMismatch { expected, actual } => {
                write!(
                    f,
                    "roster expected {expected} strategy counts, got {actual}"
                )
            }
            Self::RosterSizeOverflow => write!(f, "roster size overflowed usize"),
            Self::RosterTooSmall(count) => {
                write!(
                    f,
                    "opponent roster must contain at least two agents, got {count}"
                )
            }
            Self::DistributionLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "expected probability distribution length {expected}, got {actual}"
                )
            }
            Self::InvalidDistributionValue { index, value } => {
                write!(f, "invalid probability at index {index}: {value}")
            }
            Self::DistributionMassMismatch(sum) => {
                write!(f, "probability distribution sums to {sum}, not one")
            }
            Self::RosterStrategyMismatch => {
                write!(
                    f,
                    "rank-correlation evaluations contain different strategy sets"
                )
            }
        }
    }
}

impl std::error::Error for MetaGameError {}

pub fn evaluate_roster(
    game: &MetaGame,
    roster: &OpponentRoster,
) -> Result<MetaGameEvaluation, MetaGameError> {
    game.validate()?;
    let population_size = roster.validate(game.strategy_names.len())?;
    let strategy_count = game.strategy_names.len();
    let mut outcomes = Vec::with_capacity(strategy_count);

    for focal in 0..strategy_count {
        // The roster describes opponents faced by each focal strategy. Do not
        // subtract the focal strategy from this distribution.
        let expected_payoff = (0..strategy_count)
            .map(|opponent| game.payoffs[focal][opponent] * roster.counts[opponent] as f64)
            .sum::<f64>()
            / population_size as f64;

        let best_response_payoff = (0..strategy_count)
            .map(|candidate| {
                (0..strategy_count)
                    .map(|opponent| {
                        game.payoffs[candidate][opponent] * roster.counts[opponent] as f64
                    })
                    .sum::<f64>()
                    / population_size as f64
            })
            .fold(f64::NEG_INFINITY, f64::max);

        outcomes.push(StrategyOutcome {
            strategy_index: focal,
            strategy_name: game.strategy_names[focal].clone(),
            expected_payoff,
            best_response_payoff,
            exploitability: (best_response_payoff - expected_payoff).max(0.0),
        });
    }

    let mut ranked_indices = (0..strategy_count).collect::<Vec<_>>();
    ranked_indices.sort_by(|left, right| {
        outcomes[*right]
            .expected_payoff
            .total_cmp(&outcomes[*left].expected_payoff)
            .then_with(|| {
                outcomes[*left]
                    .strategy_name
                    .cmp(&outcomes[*right].strategy_name)
            })
    });

    Ok(MetaGameEvaluation {
        roster_id: roster.roster_id.clone(),
        population_size,
        outcomes,
        ranked_indices,
        invasion_graph: game.invasion_graph()?,
    })
}

/// Spearman rank correlation between two roster-specific strategy score vectors.
///
/// Ties receive average ranks. A constant ranking (such as balanced RPS)
/// has zero variance and returns no correlation value.
pub fn spearman_rank_correlation(
    left: &MetaGameEvaluation,
    right: &MetaGameEvaluation,
) -> Result<Option<f64>, MetaGameError> {
    if left.outcomes.len() != right.outcomes.len()
        || left
            .outcomes
            .iter()
            .zip(&right.outcomes)
            .any(|(a, b)| a.strategy_name != b.strategy_name)
    {
        return Err(MetaGameError::RosterStrategyMismatch);
    }
    let left_values = left
        .outcomes
        .iter()
        .map(|outcome| outcome.expected_payoff)
        .collect::<Vec<_>>();
    let right_values = right
        .outcomes
        .iter()
        .map(|outcome| outcome.expected_payoff)
        .collect::<Vec<_>>();
    let left_ranks = average_ranks_descending(&left_values);
    let right_ranks = average_ranks_descending(&right_values);
    Ok(pearson_correlation(&left_ranks, &right_ranks))
}

pub fn regret_certificate(
    game: &MetaGame,
    actual_opponent_distribution: &[f64],
    predicted_opponent_distribution: &[f64],
) -> Result<RegretCertificate, MetaGameError> {
    game.validate()?;
    let strategy_count = game.strategy_names.len();
    validate_distribution(actual_opponent_distribution, strategy_count)?;
    validate_distribution(predicted_opponent_distribution, strategy_count)?;

    let predicted_best_response = best_response(game, predicted_opponent_distribution);
    let actual_best_response = best_response(game, actual_opponent_distribution);
    let actual_regret =
        (expected_action_payoff(game, actual_best_response, actual_opponent_distribution)
            - expected_action_payoff(game, predicted_best_response, actual_opponent_distribution))
        .max(0.0);
    let l1_error = actual_opponent_distribution
        .iter()
        .zip(predicted_opponent_distribution)
        .map(|(actual, predicted)| (actual - predicted).abs())
        .sum::<f64>();
    let certified_upper_bound = game.payoff_range()? * l1_error;

    Ok(RegretCertificate {
        predicted_best_response,
        actual_best_response,
        actual_regret,
        opponent_distribution_l1_error: l1_error,
        certified_upper_bound,
    })
}

fn best_response(game: &MetaGame, distribution: &[f64]) -> usize {
    let mut best_index = 0usize;
    let mut best_payoff = f64::NEG_INFINITY;
    for strategy in 0..game.strategy_names.len() {
        let payoff = expected_action_payoff(game, strategy, distribution);
        if payoff > best_payoff + TOLERANCE {
            best_payoff = payoff;
            best_index = strategy;
        }
    }
    best_index
}

fn expected_action_payoff(game: &MetaGame, action: usize, distribution: &[f64]) -> f64 {
    game.payoffs[action]
        .iter()
        .zip(distribution)
        .map(|(payoff, probability)| payoff * probability)
        .sum()
}

fn validate_distribution(distribution: &[f64], expected: usize) -> Result<(), MetaGameError> {
    if distribution.len() != expected {
        return Err(MetaGameError::DistributionLengthMismatch {
            expected,
            actual: distribution.len(),
        });
    }
    for (index, value) in distribution.iter().enumerate() {
        if !value.is_finite() || *value < 0.0 {
            return Err(MetaGameError::InvalidDistributionValue {
                index,
                value: *value,
            });
        }
    }
    let sum = distribution.iter().sum::<f64>();
    if (sum - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(MetaGameError::DistributionMassMismatch(sum));
    }
    Ok(())
}

fn average_ranks_descending(values: &[f64]) -> Vec<f64> {
    let mut order = (0..values.len()).collect::<Vec<_>>();
    order.sort_by(|left, right| values[*right].total_cmp(&values[*left]));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0usize;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && (values[order[start]] - values[order[end]]).abs() <= TOLERANCE {
            end += 1;
        }
        let average_rank = ((start + 1) as f64 + end as f64) / 2.0;
        for index in &order[start..end] {
            ranks[*index] = average_rank;
        }
        start = end;
    }
    ranks
}

fn pearson_correlation(left: &[f64], right: &[f64]) -> Option<f64> {
    if left.len() != right.len() || left.len() < 2 {
        return None;
    }
    let left_mean = left.iter().sum::<f64>() / left.len() as f64;
    let right_mean = right.iter().sum::<f64>() / right.len() as f64;
    let numerator = left
        .iter()
        .zip(right)
        .map(|(a, b)| (a - left_mean) * (b - right_mean))
        .sum::<f64>();
    let left_variance = left
        .iter()
        .map(|value| (value - left_mean).powi(2))
        .sum::<f64>();
    let right_variance = right
        .iter()
        .map(|value| (value - right_mean).powi(2))
        .sum::<f64>();
    let denominator = (left_variance * right_variance).sqrt();
    if denominator <= f64::EPSILON {
        None
    } else {
        Some((numerator / denominator).clamp(-1.0, 1.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rps_game() -> MetaGame {
        MetaGame {
            strategy_names: vec!["rock".into(), "paper".into(), "scissors".into()],
            payoffs: vec![
                vec![0.0, -1.0, 1.0],
                vec![1.0, 0.0, -1.0],
                vec![-1.0, 1.0, 0.0],
            ],
        }
    }

    fn roster(id: &str, counts: [usize; 3]) -> OpponentRoster {
        OpponentRoster {
            roster_id: id.into(),
            counts: counts.into(),
        }
    }

    #[test]
    fn exact_rps_matrix_has_one_three_strategy_cycle() {
        let graph = rps_game().invasion_graph().unwrap();
        assert_eq!(graph.edge_count, 3);
        assert_eq!(graph.three_cycles.len(), 1);
        assert_eq!(graph.three_cycles[0].orientation, CycleOrientation::Reverse);
    }

    #[test]
    fn balanced_roster_has_no_unique_winner() {
        let result = evaluate_roster(&rps_game(), &roster("balanced", [10, 10, 10])).unwrap();
        assert!(
            result
                .outcomes
                .iter()
                .all(|outcome| outcome.expected_payoff.abs() < 1e-12)
        );
        assert!(
            spearman_rank_correlation(&result, &result)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn roster_composition_changes_the_winning_strategy() {
        let game = rps_game();
        let rock = evaluate_roster(&game, &roster("rock_heavy", [24, 3, 3])).unwrap();
        let paper = evaluate_roster(&game, &roster("paper_heavy", [3, 24, 3])).unwrap();
        let scissors = evaluate_roster(&game, &roster("scissors_heavy", [3, 3, 24])).unwrap();
        assert_eq!(game.strategy_names[rock.ranked_indices[0]], "paper");
        assert_eq!(game.strategy_names[paper.ranked_indices[0]], "scissors");
        assert_eq!(game.strategy_names[scissors.ranked_indices[0]], "rock");
    }

    #[test]
    fn ranking_correlation_reflects_roster_shift() {
        let game = rps_game();
        let rock = evaluate_roster(&game, &roster("rock_heavy", [24, 3, 3])).unwrap();
        let paper = evaluate_roster(&game, &roster("paper_heavy", [3, 24, 3])).unwrap();
        let rho = spearman_rank_correlation(&rock, &paper).unwrap().unwrap();
        assert!((rho + 0.5).abs() < 1e-12);
    }

    #[test]
    fn regret_certificate_bounds_actual_best_response_regret() {
        let certificate =
            regret_certificate(&rps_game(), &[0.1, 0.8, 0.1], &[0.8, 0.1, 0.1]).unwrap();
        assert_eq!(certificate.predicted_best_response, 1);
        assert_eq!(certificate.actual_best_response, 2);
        assert!((certificate.actual_regret - 0.7).abs() < 1e-12);
        assert!(certificate.actual_regret <= certificate.certified_upper_bound);
        assert!((certificate.certified_upper_bound - 2.8).abs() < 1e-12);
    }

    #[test]
    fn exploitability_is_reported_per_roster_and_strategy() {
        let result = evaluate_roster(&rps_game(), &roster("rock_heavy", [24, 3, 3])).unwrap();
        assert!(
            result
                .outcomes
                .iter()
                .all(|outcome| outcome.exploitability >= 0.0)
        );
        assert!(
            result
                .outcomes
                .iter()
                .any(|outcome| outcome.exploitability > 0.1)
        );
    }

    #[test]
    fn malformed_payoff_matrices_and_rosters_fail_closed() {
        let mut game = rps_game();
        game.payoffs[0].pop();
        assert!(matches!(
            game.validate(),
            Err(MetaGameError::PayoffColumnCount { .. })
        ));
        let invalid_roster = roster("bad", [1, 1, 1]);
        assert!(matches!(
            invalid_roster.validate(2),
            Err(MetaGameError::RosterCountMismatch { .. })
        ));
    }

    #[test]
    fn malformed_probability_distributions_fail_closed() {
        assert!(matches!(
            regret_certificate(&rps_game(), &[0.5, 0.5], &[1.0, 0.0, 0.0]),
            Err(MetaGameError::DistributionLengthMismatch { .. })
        ));
        assert!(matches!(
            regret_certificate(&rps_game(), &[0.5, 0.5, 0.5], &[1.0, 0.0, 0.0]),
            Err(MetaGameError::DistributionMassMismatch(_))
        ));
    }
}
