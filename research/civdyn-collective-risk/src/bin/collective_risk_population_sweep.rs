// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact N=6/N=30 population-composition, Fermi and rare-mutation sweep.

use civdyn_collective_risk::population::{
    DEFAULT_SELECTION_INTENSITY, PopulationState, expected_type_fitness, invasion_graph,
    rare_mutation_chain, stationary_distribution,
};
use civdyn_collective_risk::{GameConfig, Strategy};

const POPULATION_SIZES: [usize; 2] = [6, 30];
const RISK_GRID: [f64; 3] = [0.1, 0.5, 0.9];
const BETA_GRID: [f64; 4] = [0.0, 0.1, DEFAULT_SELECTION_INTENSITY, 5.0];
const STATIONARY_TOLERANCE: f64 = 1e-10;
const STATIONARY_MAX_ITERATIONS: usize = 200_000;

fn fmt(value: f64) -> String {
    format!("{value:.12}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(fields.len(), 24, "every population record has 24 columns");
    println!("{}", fields.join(","));
}

fn empty_fields(record_type: &str, population_size: usize, risk: f64, beta: f64) -> Vec<String> {
    vec![
        record_type.into(),
        population_size.to_string(),
        fmt(risk),
        fmt(beta),
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
        String::new(),
        String::new(),
    ]
}

fn population_profiles(
    population_size: usize,
) -> Vec<(&'static str, PopulationState)> {
    match population_size {
        6 => vec![(
            "one_of_each",
            PopulationState::new([1, 1, 1, 1, 1, 1]).unwrap(),
        )],
        30 => vec![
            (
                "balanced",
                PopulationState::new([5, 5, 5, 5, 5, 5]).unwrap(),
            ),
            (
                "zero_heavy",
                PopulationState::new([12, 3, 3, 3, 3, 6]).unwrap(),
            ),
            (
                "fair_share_heavy",
                PopulationState::new([2, 12, 2, 6, 2, 6]).unwrap(),
            ),
            (
                "maximum_heavy",
                PopulationState::new([2, 2, 12, 2, 8, 4]).unwrap(),
            ),
            (
                "copy_heavy",
                PopulationState::new([2, 2, 2, 2, 4, 18]).unwrap(),
            ),
        ],
        _ => panic!("unsupported finite-population reference size {population_size}"),
    }
}

fn main() {
    println!(
        "record_type,population_size,risk,beta,strategy,other_strategy,population_count,expected_fitness,expected_group_welfare,target_reach_probability,expected_group_contribution,composition_count,fixation_probability,log_fixation,stationary_weight,stationary_iterations,stationary_residual_l1,invasion_edge_count,three_cycle_count,cycle_strategy_a,cycle_strategy_b,cycle_strategy_c,cycle_orientation,roster_id"
    );

    for population_size in POPULATION_SIZES {
        for risk in RISK_GRID {
            let config = GameConfig {
                catastrophe_probability: risk,
            };
            for beta in BETA_GRID {
                for (roster_id, population) in population_profiles(population_size) {
                    for strategy in Strategy::ALL {
                        let fitness = expected_type_fitness(config, population, strategy)
                            .expect("reference population fitness must be defined");
                        let mut row = empty_fields("mixture_fitness", population_size, risk, beta);
                        row[4] = strategy.name().into();
                        row[6] = fitness.population_count.to_string();
                        row[7] = fmt(fitness.expected_payoff);
                        row[8] = fmt(fitness.expected_group_welfare);
                        row[9] = fmt(fitness.target_reach_probability);
                        row[10] = fmt(fitness.expected_group_contribution);
                        row[11] = fitness.compositions_evaluated.to_string();
                        row[23] = roster_id.into();
                        emit(row);
                    }
                }

                let chain = rare_mutation_chain(config, population_size, beta)
                    .expect("rare-mutation transition chain must be defined");
                let stationary = stationary_distribution(
                    &chain,
                    STATIONARY_TOLERANCE,
                    STATIONARY_MAX_ITERATIONS,
                )
                .expect("stationary distribution must converge within the declared limit");

                for resident_index in 0..Strategy::ALL.len() {
                    for mutant_index in 0..Strategy::ALL.len() {
                        if resident_index == mutant_index {
                            continue;
                        }
                        let mut row = empty_fields("fixation", population_size, risk, beta);
                        row[4] = Strategy::ALL[mutant_index].name().into();
                        row[5] = Strategy::ALL[resident_index].name().into();
                        let log_fixation = chain.log_fixation_matrix[resident_index][mutant_index];
                        row[12] = fmt(log_fixation.exp());
                        row[13] = fmt(log_fixation);
                        emit(row);
                    }
                }

                for strategy_index in 0..Strategy::ALL.len() {
                    let mut row = empty_fields("stationary", population_size, risk, beta);
                    row[4] = Strategy::ALL[strategy_index].name().into();
                    row[14] = fmt(stationary.weights[strategy_index]);
                    row[15] = stationary.iterations.to_string();
                    row[16] = fmt(stationary.residual_l1);
                    emit(row);
                }

                let graph = invasion_graph(&chain, population_size)
                    .expect("pairwise invasion graph must validate");
                let mut graph_row = empty_fields("invasion_graph", population_size, risk, beta);
                graph_row[17] = graph.edge_count.to_string();
                graph_row[18] = graph.three_cycles.len().to_string();
                emit(graph_row);

                for cycle in graph.three_cycles {
                    let mut row = empty_fields("invasion_cycle", population_size, risk, beta);
                    row[19] = cycle.strategies[0].name().into();
                    row[20] = cycle.strategies[1].name().into();
                    row[21] = cycle.strategies[2].name().into();
                    row[22] = match cycle.orientation {
                        civdyn_collective_risk::population::InvasionCycleOrientation::Forward => {
                            "forward".into()
                        }
                        civdyn_collective_risk::population::InvasionCycleOrientation::Reverse => {
                            "reverse".into()
                        }
                    };
                    emit(row);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_grid_covers_population_size_risk_and_selection_strength() {
        assert_eq!(POPULATION_SIZES, [6, 30]);
        assert_eq!(RISK_GRID, [0.1, 0.5, 0.9]);
        assert_eq!(BETA_GRID, [0.0, 0.1, 1.0, 5.0]);
        assert_eq!(population_profiles(6).len(), 1);
        assert_eq!(population_profiles(30).len(), 5);
    }

    #[test]
    fn balanced_population_is_one_of_each_type_for_n_six() {
        let population = PopulationState::balanced(6).unwrap();
        assert_eq!(population.counts, [1; 6]);
    }
}
