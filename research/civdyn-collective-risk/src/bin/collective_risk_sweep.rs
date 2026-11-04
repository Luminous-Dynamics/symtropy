// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CIVDYN-CRG-001 source-aligned risk and fixed-partner benchmark sweep.

use civdyn_collective_risk::{
    exact_best_response, run_episode, GameConfig, Strategy, OptimalMode, SEATS,
};

const RISK_GRID: [f64; 11] = [
    0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0,
];
const BEST_RESPONSE_RISKS: [f64; 3] = [0.1, 0.5, 0.9];
const PARTNER_TYPES: [Strategy; 5] = [
    Strategy::AlwaysZero,
    Strategy::FixedFairShare,
    Strategy::FixedMaximum,
    Strategy::CopyPreviousGroupMean,
    Strategy::LastRoundDefection,
];

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn main() {
    println!(
        "record_type,risk,policy,mean_contribution,total_contribution,target_reached,overshoot,shortfall,per_seat_expected_welfare,group_expected_welfare,payoff_gini,optimality_gap,partner_policy,best_response_total,best_response_payoff,candidates_examined"
    );

    for risk in RISK_GRID {
        let config = GameConfig {
            catastrophe_probability: risk,
        };
        let optimum = config.welfare_optimum().expect("risk grid is valid");
        for policy in Strategy::ALL {
            let outcome = run_episode(config, &[policy; SEATS]).expect("policy must be valid");
            let gap = outcome.optimality_gap(config).expect("risk is valid");
            println!(
                "self_play,{},{},{},{},{},{},{},{},{},{},{},,,,",
                fmt(risk),
                policy.name(),
                fmt(outcome.mean_contribution_per_seat()),
                outcome.total_contribution,
                outcome.target_reached,
                outcome.overshoot,
                outcome.shortfall,
                fmt(outcome.expected_per_seat_welfare),
                fmt(outcome.expected_group_welfare),
                fmt(outcome.payoff_gini),
                fmt(gap),
            );
        }

        // The analytical optimum is included as an explicit reference row.
        let modes = optimum
            .optimal_modes
            .iter()
            .map(|mode| match mode {
                OptimalMode::NoContribution => "no_contribution",
                OptimalMode::ReachTarget => "reach_target",
            })
            .collect::<Vec<_>>()
            .join("|");
        println!(
            "analytical_optimum,{},{},,,,,,{},{},0.000000,0.000000,,,,",
            fmt(risk),
            modes,
            fmt(optimum.expected_per_seat_welfare),
            fmt(optimum.expected_group_welfare),
        );
    }

    for risk in BEST_RESPONSE_RISKS {
        let config = GameConfig {
            catastrophe_probability: risk,
        };
        for partner in PARTNER_TYPES {
            let response = exact_best_response(config, &[partner; SEATS - 1])
                .expect("scripted partner best response must validate");
            println!(
                "best_response,{},{},,{},{},{},{},{},{},{},{},{},{},{},{}",
                fmt(risk),
                "exact_enumeration",
                response.total_contribution,
                response.outcome.target_reached,
                response.outcome.overshoot,
                response.outcome.shortfall,
                fmt(response.outcome.expected_per_seat_welfare),
                fmt(response.outcome.expected_group_welfare),
                fmt(response.outcome.payoff_gini),
                fmt(
                    (config.welfare_optimum().unwrap().expected_group_welfare
                        - response.outcome.expected_group_welfare)
                        .max(0.0)
                ),
                partner.name(),
                response.total_contribution,
                fmt(response.expected_payoff),
                response.candidates_examined,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risk_grid_includes_zero_pivot_and_certainty() {
        assert_eq!(RISK_GRID[0], 0.0);
        assert_eq!(RISK_GRID[5], 0.5);
        assert_eq!(RISK_GRID[10], 1.0);
    }

    #[test]
    fn fixed_partner_sweep_uses_distinct_partner_types() {
        assert_eq!(PARTNER_TYPES.len(), 5);
        assert_ne!(PARTNER_TYPES[0], PARTNER_TYPES[1]);
        assert_ne!(PARTNER_TYPES[1], PARTNER_TYPES[2]);
    }
}
