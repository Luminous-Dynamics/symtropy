// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Exact reference task for a six-seat collective-risk dilemma.
//!
//! The task contract is source-aligned with Huynh et al. (2026),
//! "Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma".
//! This crate reconstructs the public game and its analytical benchmark; it
//! does not reproduce the paper's prompts, model outputs or population study.

use core::fmt;

pub const SEATS: usize = 6;
pub const ROUNDS: usize = 10;
pub const ENDOWMENT: u32 = 40;
pub const TARGET: u32 = 120;
pub const ACTIONS: [u8; 3] = [0, 2, 4];
pub const RISK_PIVOT: f64 = TARGET as f64 / (SEATS as f64 * ENDOWMENT as f64);
const MAX_PLAN_CANDIDATES: u64 = 3_u64.pow(ROUNDS as u32);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameConfig {
    /// Probability of catastrophe when the shared target is not reached.
    pub catastrophe_probability: f64,
}

impl GameConfig {
    pub fn validate(self) -> Result<(), GameError> {
        if !self.catastrophe_probability.is_finite()
            || !(0.0..=1.0).contains(&self.catastrophe_probability)
        {
            return Err(GameError::InvalidRisk(self.catastrophe_probability));
        }
        Ok(())
    }

    pub fn welfare_optimum(self) -> Result<WelfareOptimum, GameError> {
        self.validate()?;
        let no_contribution =
            (1.0 - self.catastrophe_probability) * ENDOWMENT as f64 * SEATS as f64;
        let exact_target = (ENDOWMENT * SEATS as u32 - TARGET) as f64;
        let group_welfare = no_contribution.max(exact_target);
        let pivot = RISK_PIVOT;
        let optimal_modes = if (self.catastrophe_probability - pivot).abs() <= f64::EPSILON {
            vec![OptimalMode::NoContribution, OptimalMode::ReachTarget]
        } else if self.catastrophe_probability < pivot {
            vec![OptimalMode::NoContribution]
        } else {
            vec![OptimalMode::ReachTarget]
        };
        Ok(WelfareOptimum {
            expected_group_welfare: group_welfare,
            expected_per_seat_welfare: group_welfare / SEATS as f64,
            pivot_probability: pivot,
            optimal_modes,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimalMode {
    NoContribution,
    ReachTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WelfareOptimum {
    pub expected_group_welfare: f64,
    pub expected_per_seat_welfare: f64,
    pub pivot_probability: f64,
    pub optimal_modes: Vec<OptimalMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strategy {
    /// Contribute nothing in every round.
    AlwaysZero,
    /// Contribute two units in each round, even if the target is already met.
    FixedFairShare,
    /// Contribute four units in each round until the endowment is exhausted.
    FixedMaximum,
    /// Contribute nothing below the analytical risk pivot; contribute two
    /// units until the target is reached above the pivot.
    RiskPivot,
    /// Contribute two units except in the final round.
    LastRoundDefection,
    /// Start at two, then copy the nearest allowed action to the mean of the
    /// other seats' contributions in the previous round.
    CopyPreviousGroupMean,
}

impl Strategy {
    pub const ALL: [Self; 6] = [
        Self::AlwaysZero,
        Self::FixedFairShare,
        Self::FixedMaximum,
        Self::RiskPivot,
        Self::LastRoundDefection,
        Self::CopyPreviousGroupMean,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::AlwaysZero => "always_zero",
            Self::FixedFairShare => "fixed_fair_share",
            Self::FixedMaximum => "fixed_maximum",
            Self::RiskPivot => "risk_pivot",
            Self::LastRoundDefection => "last_round_defection",
            Self::CopyPreviousGroupMean => "copy_previous_group_mean",
        }
    }

    fn choose(
        self,
        config: GameConfig,
        round: usize,
        pool_before_round: u32,
        own_total: u32,
        previous_actions: &[u8; SEATS],
        seat: usize,
    ) -> u8 {
        let remaining = ENDOWMENT.saturating_sub(own_total);
        let legal_max = if remaining >= 4 {
            4
        } else if remaining >= 2 {
            2
        } else {
            0
        };

        let requested = match self {
            Self::AlwaysZero => 0,
            Self::FixedFairShare => 2,
            Self::FixedMaximum => 4,
            Self::RiskPivot => {
                if config.catastrophe_probability < RISK_PIVOT || pool_before_round >= TARGET {
                    0
                } else {
                    2
                }
            }
            Self::LastRoundDefection => {
                if round + 1 == ROUNDS {
                    0
                } else {
                    2
                }
            }
            Self::CopyPreviousGroupMean => {
                if round == 0 {
                    2
                } else {
                    let sum: u32 = previous_actions
                        .iter()
                        .enumerate()
                        .filter(|(other, _)| *other != seat)
                        .map(|(_, action)| u32::from(*action))
                        .sum();
                    let mean = sum as f64 / (SEATS - 1) as f64;
                    nearest_legal_action(mean, legal_max)
                }
            }
        };

        requested.min(legal_max)
    }
}

fn nearest_legal_action(value: f64, legal_max: u8) -> u8 {
    ACTIONS
        .into_iter()
        .filter(|action| *action <= legal_max)
        .min_by(|left, right| {
            let left_distance = (f64::from(*left) - value).abs();
            let right_distance = (f64::from(*right) - value).abs();
            left_distance
                .total_cmp(&right_distance)
                .then_with(|| left.cmp(right))
        })
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    InvalidRisk(f64),
    WrongSeatCount {
        expected: usize,
        actual: usize,
    },
    WrongPartnerCount {
        expected: usize,
        actual: usize,
    },
    InvalidActionPlan {
        seat: usize,
        round: usize,
        action: u8,
        remaining_endowment: u32,
    },
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRisk(value) => {
                write!(
                    f,
                    "catastrophe probability must be finite and in [0, 1], got {value}"
                )
            }
            Self::WrongSeatCount { expected, actual } => {
                write!(f, "expected {expected} seat strategies, got {actual}")
            }
            Self::WrongPartnerCount { expected, actual } => {
                write!(f, "expected {expected} scripted partners, got {actual}")
            }
            Self::InvalidActionPlan {
                seat,
                round,
                action,
                remaining_endowment,
            } => write!(
                f,
                "seat {seat} planned contribution {action} in round {round} with only {remaining_endowment} endowment remaining"
            ),
        }
    }
}

impl std::error::Error for GameError {}

#[derive(Debug, Clone, PartialEq)]
pub struct EpisodeOutcome {
    pub risk: f64,
    pub strategies: Vec<Strategy>,
    pub contribution_history: Vec<[u8; SEATS]>,
    pub contributions_by_seat: [u32; SEATS],
    pub total_contribution: u32,
    pub target_reached: bool,
    pub overshoot: u32,
    pub shortfall: u32,
    pub expected_payoffs: [f64; SEATS],
    pub expected_group_welfare: f64,
    pub expected_per_seat_welfare: f64,
    pub payoff_gini: f64,
}

impl EpisodeOutcome {
    pub fn mean_contribution_per_seat(&self) -> f64 {
        self.total_contribution as f64 / SEATS as f64
    }

    pub fn optimality_gap(&self, config: GameConfig) -> Result<f64, GameError> {
        let optimum = config.welfare_optimum()?.expected_group_welfare;
        Ok((optimum - self.expected_group_welfare).max(0.0))
    }
}

pub fn run_episode(
    config: GameConfig,
    strategies: &[Strategy],
) -> Result<EpisodeOutcome, GameError> {
    run_internal(config, strategies, None)
}

/// Exhaustively enumerate every legal ten-round action sequence for one focal
/// seat against five fixed or deterministic scripted partners.
///
/// There are at most 3^10 = 59,049 candidate sequences before endowment
/// filtering. Expected payoffs are evaluated analytically; no catastrophe
/// lottery draw is sampled.
pub fn exact_best_response(
    config: GameConfig,
    partners: &[Strategy],
) -> Result<BestResponse, GameError> {
    config.validate()?;
    if partners.len() != SEATS - 1 {
        return Err(GameError::WrongPartnerCount {
            expected: SEATS - 1,
            actual: partners.len(),
        });
    }

    let mut best: Option<BestResponse> = None;
    for encoded in 0..MAX_PLAN_CANDIDATES {
        let mut value = encoded;
        let mut plan = [0_u8; ROUNDS];
        let mut own_total = 0_u32;
        let mut legal = true;
        for action in &mut plan {
            let digit = (value % 3) as usize;
            value /= 3;
            *action = ACTIONS[digit];
            own_total += u32::from(*action);
            if own_total > ENDOWMENT {
                legal = false;
                break;
            }
        }
        if !legal {
            continue;
        }

        let strategies = insert_focal_strategy(partners);
        let outcome = run_internal(config, &strategies, Some((0, &plan)))?;
        let candidate = BestResponse {
            actions: plan,
            total_contribution: own_total,
            expected_payoff: outcome.expected_payoffs[0],
            outcome,
            candidates_examined: 0,
        };

        let replace = match &best {
            None => true,
            Some(current) => {
                candidate.expected_payoff > current.expected_payoff + 1e-12
                    || ((candidate.expected_payoff - current.expected_payoff).abs() <= 1e-12
                        && (candidate.total_contribution < current.total_contribution
                            || (candidate.total_contribution == current.total_contribution
                                && candidate.actions < current.actions)))
            }
        };
        if replace {
            best = Some(candidate);
        }
    }

    let mut best = best.expect("the all-zero plan is always legal");
    best.candidates_examined = MAX_PLAN_CANDIDATES as usize;
    Ok(best)
}

#[derive(Debug, Clone, PartialEq)]
pub struct BestResponse {
    pub actions: [u8; ROUNDS],
    pub total_contribution: u32,
    pub expected_payoff: f64,
    pub outcome: EpisodeOutcome,
    pub candidates_examined: usize,
}

fn insert_focal_strategy(partners: &[Strategy]) -> Vec<Strategy> {
    let mut strategies = Vec::with_capacity(SEATS);
    strategies.push(Strategy::AlwaysZero);
    strategies.extend_from_slice(partners);
    strategies
}

fn run_internal(
    config: GameConfig,
    strategies: &[Strategy],
    focal_plan: Option<(usize, &[u8; ROUNDS])>,
) -> Result<EpisodeOutcome, GameError> {
    config.validate()?;
    if strategies.len() != SEATS {
        return Err(GameError::WrongSeatCount {
            expected: SEATS,
            actual: strategies.len(),
        });
    }

    let mut contributions_by_seat = [0_u32; SEATS];
    let mut contribution_history = Vec::with_capacity(ROUNDS);
    let mut previous_actions = [0_u8; SEATS];
    let mut pool = 0_u32;

    for round in 0..ROUNDS {
        // Every policy observes the same pre-round pool and previous-round
        // contributions. Actions are committed simultaneously.
        let mut actions = [0_u8; SEATS];
        for seat in 0..SEATS {
            let remaining = ENDOWMENT - contributions_by_seat[seat];
            let requested = match focal_plan {
                Some((focal_seat, plan)) if focal_seat == seat => plan[round],
                _ => strategies[seat].choose(
                    config,
                    round,
                    pool,
                    contributions_by_seat[seat],
                    &previous_actions,
                    seat,
                ),
            };
            if !ACTIONS.contains(&requested) || requested > remaining {
                return Err(GameError::InvalidActionPlan {
                    seat,
                    round,
                    action: requested,
                    remaining_endowment: remaining,
                });
            }
            actions[seat] = requested;
        }

        for seat in 0..SEATS {
            contributions_by_seat[seat] += u32::from(actions[seat]);
        }
        pool += actions.iter().map(|action| u32::from(*action)).sum::<u32>();
        previous_actions = actions;
        contribution_history.push(actions);
    }

    let target_reached = pool >= TARGET;
    let remaining: [u32; SEATS] =
        core::array::from_fn(|seat| ENDOWMENT - contributions_by_seat[seat]);
    let expected_payoffs = core::array::from_fn(|seat| {
        let remaining = remaining[seat] as f64;
        if target_reached {
            remaining
        } else {
            (1.0 - config.catastrophe_probability) * remaining
        }
    });
    let expected_group_welfare = expected_payoffs.iter().sum::<f64>();
    let expected_per_seat_welfare = expected_group_welfare / SEATS as f64;
    let payoff_gini = gini(&expected_payoffs);

    Ok(EpisodeOutcome {
        risk: config.catastrophe_probability,
        strategies: strategies.to_vec(),
        contribution_history,
        contributions_by_seat,
        total_contribution: pool,
        target_reached,
        overshoot: pool.saturating_sub(TARGET),
        shortfall: TARGET.saturating_sub(pool),
        expected_payoffs,
        expected_group_welfare,
        expected_per_seat_welfare,
        payoff_gini,
    })
}

fn gini(values: &[f64; SEATS]) -> f64 {
    let mut sorted = *values;
    sorted.sort_by(f64::total_cmp);
    let total = sorted.iter().sum::<f64>();
    if total <= f64::EPSILON {
        return 0.0;
    }
    let weighted = sorted
        .iter()
        .enumerate()
        .map(|(index, value)| (index + 1) as f64 * value)
        .sum::<f64>();
    (2.0 * weighted / (SEATS as f64 * total) - (SEATS as f64 + 1.0) / SEATS as f64).clamp(0.0, 1.0)
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
    fn source_task_constants_match_the_published_game() {
        assert_eq!(SEATS, 6);
        assert_eq!(ROUNDS, 10);
        assert_eq!(ENDOWMENT, 40);
        assert_eq!(TARGET, 120);
        assert_eq!(ACTIONS, [0, 2, 4]);
        assert!((RISK_PIVOT - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn analytical_welfare_optimum_switches_at_the_exact_risk_pivot() {
        let low = config(0.1).welfare_optimum().unwrap();
        let pivot = config(0.5).welfare_optimum().unwrap();
        let high = config(0.9).welfare_optimum().unwrap();

        assert_eq!(low.optimal_modes, vec![OptimalMode::NoContribution]);
        assert!((low.expected_per_seat_welfare - 36.0).abs() < 1e-12);
        assert_eq!(
            pivot.optimal_modes,
            vec![OptimalMode::NoContribution, OptimalMode::ReachTarget]
        );
        assert!((pivot.expected_per_seat_welfare - 20.0).abs() < 1e-12);
        assert_eq!(high.optimal_modes, vec![OptimalMode::ReachTarget]);
        assert!((high.expected_per_seat_welfare - 20.0).abs() < 1e-12);
    }

    #[test]
    fn risk_pivot_policy_adapts_across_zero_and_high_risk() {
        let low = run_episode(config(0.1), &[Strategy::RiskPivot; SEATS]).unwrap();
        let high = run_episode(config(0.9), &[Strategy::RiskPivot; SEATS]).unwrap();

        assert_eq!(low.total_contribution, 0);
        assert!(!low.target_reached);
        assert_eq!(low.expected_group_welfare, 216.0);
        assert_eq!(high.total_contribution, TARGET);
        assert!(high.target_reached);
        assert_eq!(high.expected_group_welfare, 120.0);
        assert_eq!(high.overshoot, 0);
        assert_eq!(high.shortfall, 0);
    }

    #[test]
    fn cooperative_self_play_is_not_enough_to_establish_adaptation() {
        let fixed = run_episode(config(0.0), &[Strategy::FixedFairShare; SEATS]).unwrap();
        let adaptive = run_episode(config(0.0), &[Strategy::RiskPivot; SEATS]).unwrap();

        assert_eq!(fixed.total_contribution, TARGET);
        assert_eq!(adaptive.total_contribution, 0);
        assert!(adaptive.expected_group_welfare > fixed.expected_group_welfare);
    }

    #[test]
    fn over_contribution_after_the_target_is_settled_is_measured() {
        let outcome = run_episode(config(0.9), &[Strategy::FixedMaximum; SEATS]).unwrap();
        assert_eq!(outcome.total_contribution, 240);
        assert_eq!(outcome.overshoot, 120);
        assert!(outcome.target_reached);
        assert_eq!(outcome.expected_group_welfare, 0.0);
    }

    #[test]
    fn last_round_defection_misses_the_target_by_twelve_units() {
        let outcome = run_episode(config(0.9), &[Strategy::LastRoundDefection; SEATS]).unwrap();
        assert_eq!(outcome.total_contribution, 108);
        assert_eq!(outcome.shortfall, 12);
        assert!(!outcome.target_reached);
    }

    #[test]
    fn best_response_to_zero_partners_is_zero() {
        let response =
            exact_best_response(config(0.9), &[Strategy::AlwaysZero; SEATS - 1]).unwrap();
        assert_eq!(response.total_contribution, 0);
        assert!((response.expected_payoff - 4.0).abs() < 1e-12);
        assert!(response.candidates_examined > 0);
    }

    #[test]
    fn best_response_to_max_partners_is_zero_when_target_is_guaranteed() {
        let response =
            exact_best_response(config(0.9), &[Strategy::FixedMaximum; SEATS - 1]).unwrap();
        assert_eq!(response.total_contribution, 0);
        assert_eq!(response.outcome.total_contribution, 200);
        assert!(response.outcome.target_reached);
    }

    #[test]
    fn best_response_to_fair_share_partners_changes_at_the_pivot() {
        let partners = [Strategy::FixedFairShare; SEATS - 1];
        let low = exact_best_response(config(0.1), &partners).unwrap();
        let high = exact_best_response(config(0.9), &partners).unwrap();

        assert_eq!(low.total_contribution, 0);
        assert_eq!(high.total_contribution, 20);
        assert!(low.expected_payoff > high.expected_payoff);
    }

    #[test]
    fn best_response_tie_at_pivot_prefers_minimum_contribution() {
        let response =
            exact_best_response(config(RISK_PIVOT), &[Strategy::FixedFairShare; SEATS - 1])
                .unwrap();
        assert_eq!(response.total_contribution, 0);
        assert!((response.expected_payoff - 20.0).abs() < 1e-12);
    }

    #[test]
    fn scripted_copy_policy_uses_public_previous_round_history() {
        let outcome = run_episode(config(0.9), &[Strategy::CopyPreviousGroupMean; SEATS]).unwrap();
        assert_eq!(outcome.total_contribution, TARGET);
        assert!(
            outcome
                .contribution_history
                .iter()
                .all(|round| round == &[2; SEATS])
        );
    }

    #[test]
    fn gini_reports_distribution_not_only_mean() {
        let equal = [20.0; SEATS];
        let unequal = [0.0, 0.0, 0.0, 0.0, 0.0, 40.0];
        assert_eq!(gini(&equal), 0.0);
        assert!((gini(&unequal) - 5.0 / 6.0).abs() < 1e-12);
    }

    #[test]
    fn invalid_risk_and_policy_vector_length_fail_closed() {
        assert!(matches!(
            run_episode(config(f64::NAN), &[Strategy::AlwaysZero; SEATS]),
            Err(GameError::InvalidRisk(_))
        ));
        assert_eq!(
            run_episode(config(0.5), &[Strategy::AlwaysZero; SEATS - 1]),
            Err(GameError::WrongSeatCount {
                expected: SEATS,
                actual: SEATS - 1,
            })
        );
    }

    #[test]
    fn exact_best_response_to_last_round_defectors_is_risk_sensitive() {
        let partners = [Strategy::LastRoundDefection; SEATS - 1];
        let low = exact_best_response(config(0.1), &partners).unwrap();
        let high = exact_best_response(config(0.9), &partners).unwrap();

        assert_eq!(low.total_contribution, 0);
        assert_eq!(high.total_contribution, 30);
        assert_eq!(high.outcome.total_contribution, TARGET);
    }

    #[test]
    fn best_response_reports_the_full_enumeration_budget() {
        let response =
            exact_best_response(config(0.9), &[Strategy::FixedFairShare; SEATS - 1]).unwrap();
        assert_eq!(response.candidates_examined, 59_049);
    }

    #[test]
    fn internal_runner_rejects_noncanonical_focal_actions() {
        let strategies = [Strategy::AlwaysZero; SEATS];
        let invalid_plan = [6_u8; ROUNDS];
        assert_eq!(
            run_internal(config(0.5), &strategies, Some((0, &invalid_plan))),
            Err(GameError::InvalidActionPlan {
                seat: 0,
                round: 0,
                action: 6,
                remaining_endowment: ENDOWMENT,
            })
        );
    }

    #[test]
    fn exact_best_response_rejects_wrong_number_of_partners() {
        assert_eq!(
            exact_best_response(config(0.5), &[Strategy::AlwaysZero; 4]),
            Err(GameError::WrongPartnerCount {
                expected: 5,
                actual: 4,
            })
        );
    }
}
