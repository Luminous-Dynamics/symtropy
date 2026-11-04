// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact synthetic RPS opponent-roster and regret-certificate sweep.

use civdyn_opponent_roster::{
    CycleOrientation, MetaGame, OpponentRoster, evaluate_roster, regret_certificate,
    spearman_rank_correlation,
};

const ROSTERS: [(&str, [usize; 3]); 4] = [
    ("balanced", [10, 10, 10]),
    ("rock_heavy", [24, 3, 3]),
    ("paper_heavy", [3, 24, 3]),
    ("scissors_heavy", [3, 3, 24]),
];
const CSV_COLUMNS: usize = 21;

fn game() -> MetaGame {
    MetaGame {
        strategy_names: vec!["rock".into(), "paper".into(), "scissors".into()],
        payoffs: vec![
            vec![0.0, -1.0, 1.0],
            vec![1.0, 0.0, -1.0],
            vec![-1.0, 1.0, 0.0],
        ],
    }
}

fn roster(roster_id: &str, counts: [usize; 3]) -> OpponentRoster {
    OpponentRoster {
        roster_id: roster_id.into(),
        counts: counts.into(),
    }
}

fn fmt(value: f64) -> String {
    format!("{value:.6}")
}

fn empty_row(record_type: &str, roster_id: &str, population_size: usize) -> Vec<String> {
    vec![
        record_type.into(),
        roster_id.into(),
        population_size.to_string(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ]
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "opponent-roster CSV width must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn score_rows(game: &MetaGame, roster_id: &str, counts: [usize; 3]) {
    let evaluation = evaluate_roster(game, &roster(roster_id, counts))
        .expect("declared RPS roster must validate");
    for outcome in &evaluation.outcomes {
        let mut row = empty_row("roster_score", roster_id, evaluation.population_size);
        row[3] = outcome.strategy_name.clone();
        row[4] = (evaluation
            .ranked_indices
            .iter()
            .position(|index| *index == outcome.strategy_index)
            .unwrap()
            + 1)
        .to_string();
        row[5] = fmt(outcome.expected_payoff);
        row[6] = fmt(outcome.best_response_payoff);
        row[7] = fmt(outcome.exploitability);
        row[8] = evaluation.invasion_graph.edge_count.to_string();
        row[9] = evaluation.invasion_graph.three_cycles.len().to_string();
        emit(row);
    }

    let mut graph_row = empty_row("invasion_graph", roster_id, evaluation.population_size);
    graph_row[8] = evaluation.invasion_graph.edge_count.to_string();
    graph_row[9] = evaluation.invasion_graph.three_cycles.len().to_string();
    emit(graph_row);

    for cycle in &evaluation.invasion_graph.three_cycles {
        let mut row = empty_row("invasion_cycle", roster_id, evaluation.population_size);
        row[12] = game.strategy_names[cycle.strategies[0]].clone();
        row[13] = game.strategy_names[cycle.strategies[1]].clone();
        row[14] = game.strategy_names[cycle.strategies[2]].clone();
        row[15] = match cycle.orientation {
            CycleOrientation::Forward => "forward".into(),
            CycleOrientation::Reverse => "reverse".into(),
        };
        emit(row);
    }
}

fn correlation_rows(game: &MetaGame) {
    let evaluations = ROSTERS
        .iter()
        .map(|(id, counts)| {
            evaluate_roster(game, &roster(id, *counts)).expect("fixed RPS roster must validate")
        })
        .collect::<Vec<_>>();
    for left in 0..evaluations.len() {
        for right in (left + 1)..evaluations.len() {
            let rho = spearman_rank_correlation(&evaluations[left], &evaluations[right])
                .expect("roster strategy sets match");
            let mut row = empty_row(
                "rank_correlation",
                &evaluations[left].roster_id,
                evaluations[left].population_size,
            );
            row[10] = evaluations[right].roster_id.clone();
            row[11] = rho.map_or_else(String::new, fmt);
            emit(row);
        }
    }
}

fn certificate_row(game: &MetaGame) {
    let certificate = regret_certificate(game, &[0.1, 0.8, 0.1], &[0.8, 0.1, 0.1])
        .expect("RPS opponent distributions must validate");
    let mut row = empty_row("regret_certificate", "prediction_shift", 0);
    row[16] = game.strategy_names[certificate.predicted_best_response].clone();
    row[17] = game.strategy_names[certificate.actual_best_response].clone();
    row[18] = fmt(certificate.actual_regret);
    row[19] = fmt(certificate.opponent_distribution_l1_error);
    row[20] = fmt(certificate.certified_upper_bound);
    emit(row);
}

fn main() {
    println!(
        "record_type,roster_id,population_size,strategy,rank,expected_payoff,best_response_payoff,exploitability,invasion_edge_count,three_cycle_count,comparison_roster,spearman_rho,cycle_strategy_a,cycle_strategy_b,cycle_strategy_c,cycle_orientation,predicted_best_response,actual_best_response,actual_regret,opponent_distribution_l1_error,certificate_upper_bound"
    );

    let game = game();
    for (roster_id, counts) in ROSTERS {
        score_rows(&game, roster_id, counts);
    }
    correlation_rows(&game);
    certificate_row(&game);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_roster_portfolio_covers_balanced_and_three_skewed_opponents() {
        assert_eq!(ROSTERS.len(), 4);
        assert_eq!(ROSTERS[0].1, [10, 10, 10]);
        assert_eq!(ROSTERS[1].1, [24, 3, 3]);
        assert_eq!(ROSTERS[2].1, [3, 24, 3]);
        assert_eq!(ROSTERS[3].1, [3, 3, 24]);
    }

    #[test]
    fn rps_payoff_range_is_two() {
        assert_eq!(game().payoff_range().unwrap(), 2.0);
    }
}
