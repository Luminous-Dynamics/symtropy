// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! CIVDYN-CRG-001 source-aligned risk and fixed-partner benchmark sweep.

use civdyn_collective_risk::{
    GameConfig, OptimalMode, SEATS, Strategy, exact_best_response, run_episode,
};

const RISK_GRID: [f64; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];
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

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), 16, "every CSV record has 16 columns");
    println!("{}", fields.join(","));
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
            emit(vec![
                "self_play".into(),
                fmt(risk),
                policy.name().into(),
                fmt(outcome.mean_contribution_per_seat()),
                outcome.total_contribution.to_string(),
                outcome.target_reached.to_string(),
                outcome.overshoot.to_string(),
                outcome.shortfall.to_string(),
                fmt(outcome.expected_per_seat_welfare),
                fmt(outcome.expected_group_welfare),
                fmt(outcome.payoff_gini),
                fmt(gap),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }

        let modes = optimum
            .optimal_modes
            .iter()
            .map(|mode| match mode {
                OptimalMode::NoContribution => "no_contribution",
                OptimalMode::ReachTarget => "reach_target",
            })
            .collect::<Vec<_>>()
            .join("|");
        emit(vec![
            "analytical_optimum".into(),
            fmt(risk),
            modes,
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            fmt(optimum.expected_per_seat_welfare),
            fmt(optimum.expected_group_welfare),
            "0.000000".into(),
            "0.000000".into(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ]);
    }

    for risk in BEST_RESPONSE_RISKS {
        let config = GameConfig {
            catastrophe_probability: risk,
        };
        for partner in PARTNER_TYPES {
            let response = exact_best_response(config, &[partner; SEATS - 1])
                .expect("scripted partner best response must validate");
            emit(vec![
                "best_response".into(),
                fmt(risk),
                "exact_enumeration".into(),
                String::new(),
                response.total_contribution.to_string(),
                response.outcome.target_reached.to_string(),
                response.outcome.overshoot.to_string(),
                response.outcome.shortfall.to_string(),
                fmt(response.outcome.expected_per_seat_welfare),
                fmt(response.outcome.expected_group_welfare),
                fmt(response.outcome.payoff_gini),
                fmt((config.welfare_optimum().unwrap().expected_group_welfare
                    - response.outcome.expected_group_welfare)
                    .max(0.0)),
                partner.name().into(),
                response.total_contribution.to_string(),
                fmt(response.expected_payoff),
                response.candidates_examined.to_string(),
            ]);
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
