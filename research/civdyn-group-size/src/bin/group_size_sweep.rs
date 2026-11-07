// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Synthetic finite-memory population-size sweep; no LLM policy/data is imported.

use civdyn_group_size::{
    DEFAULT_CONVERGENCE_THRESHOLD, DEFAULT_CONVERGENCE_WINDOW, DEFAULT_MATCH_PAYOFF,
    DEFAULT_MAX_POPULATION_ROUNDS, DEFAULT_MEMORY_CAPACITY, DEFAULT_MISMATCH_PAYOFF, GameConfig,
    PolicyTable, memory_independent_oracle, memory_state_count, simulate_population,
};

const POPULATION_SIZES: [usize; 7] = [2, 3, 4, 6, 10, 24, 100];
const REPLICATES: usize = 30;
const MASTER_SEED: u64 = 0x4353_5347_5250_3031;
const CSV_COLUMNS: usize = 26;

fn labels() -> [String; 2] {
    ["convention_0".into(), "convention_1".into()]
}

fn policies() -> Vec<PolicyTable> {
    vec![
        PolicyTable::memory_independent("always_option_0", DEFAULT_MEMORY_CAPACITY, labels(), 1.0)
            .expect("option-0 policy table is valid"),
        PolicyTable::memory_independent("always_option_1", DEFAULT_MEMORY_CAPACITY, labels(), 0.0)
            .expect("option-1 policy table is valid"),
        PolicyTable::memory_independent(
            "neutral_memory_independent",
            DEFAULT_MEMORY_CAPACITY,
            labels(),
            0.5,
        )
        .expect("neutral policy table is valid"),
        PolicyTable::memory_independent(
            "weak_option_0_bias",
            DEFAULT_MEMORY_CAPACITY,
            labels(),
            0.55,
        )
        .expect("biased policy table is valid"),
        PolicyTable::copy_partner_majority(
            "copy_partner_majority",
            DEFAULT_MEMORY_CAPACITY,
            labels(),
            0.5,
            0.5,
            false,
        )
        .expect("copy policy table is valid"),
        PolicyTable::copy_partner_majority(
            "anti_copy_partner_majority",
            DEFAULT_MEMORY_CAPACITY,
            labels(),
            0.5,
            0.5,
            true,
        )
        .expect("anti-copy policy table is valid"),
    ]
}

fn stable_seed(population_size: usize, policy_index: usize, replicate: usize) -> u64 {
    MASTER_SEED
        ^ (population_size as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (policy_index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03)
        ^ (replicate as u64).wrapping_mul(0x94D0_49BB_1331_11EB)
}

fn fmt(value: f64) -> String {
    format!("{value:.9}")
}

fn emit(fields: Vec<String>) {
    assert_eq!(
        fields.len(),
        CSV_COLUMNS,
        "group-size CSV width must remain fixed"
    );
    println!("{}", fields.join(","));
}

fn replicate_row(
    policy: &PolicyTable,
    population_size: usize,
    replicate: usize,
    seed: u64,
    result: &civdyn_group_size::RunOutcome,
) {
    let oracle = policy
        .memory_independent_probability_option0()
        .map(|probability| {
            memory_independent_oracle(probability, population_size)
                .expect("policy and population size are valid for the analytical oracle")
        });
    let row = vec![
        "replicate".into(),
        policy.policy_id.clone(),
        population_size.to_string(),
        replicate.to_string(),
        seed.to_string(),
        result.memory_capacity.to_string(),
        memory_state_count(result.memory_capacity)
            .expect("policy capacity was validated")
            .to_string(),
        fmt(policy.empty_memory_probability_option0() - 0.5),
        result.rounds_run().to_string(),
        result.total_pairwise_interactions.to_string(),
        result.agreement_converged.to_string(),
        result
            .agreement_convergence_round
            .map_or_else(String::new, |v| v.to_string()),
        result
            .convergence_window_convention
            .map_or_else(String::new, |v| v.to_string()),
        result
            .true_action_unanimity_round
            .map_or_else(String::new, |v| v.to_string()),
        fmt(result.final_option0_action_share),
        fmt(result.final_agreement_rate),
        fmt(result.final_mean_agent_payoff),
        result.final_minimum_agent_payoff.to_string(),
        result.final_maximum_agent_payoff.to_string(),
        result.final_unique_memory_states.to_string(),
        fmt(result.final_memory_state_entropy_bits),
        "synthetic_policy_table".into(),
        oracle.map_or_else(String::new, |value| fmt(value.expected_option0_action_share)),
        oracle.map_or_else(String::new, |value| fmt(value.expected_pairwise_agreement_rate)),
        oracle.map_or_else(String::new, |value| fmt(value.option0_action_share_standard_deviation)),
        oracle.map_or_else(String::new, |value| fmt(value.pairwise_agreement_standard_deviation)),
    ];
    emit(row);
}

fn main() {
    println!(
        "record_type,policy_id,population_size,replicate_id,seed,memory_capacity,memory_state_count,initial_individual_bias,rounds_run,total_pairwise_interactions,agreement_converged,agreement_convergence_round,convergence_window_convention,true_action_unanimity_round,final_option0_action_share,final_agreement_rate,final_mean_cumulative_payoff,final_minimum_cumulative_payoff,final_maximum_cumulative_payoff,final_unique_memory_states,final_memory_state_entropy_bits,policy_source_kind,oracle_expected_option0_action_share,oracle_expected_pairwise_agreement_rate,oracle_option0_share_standard_deviation,oracle_pairwise_agreement_standard_deviation"
    );

    let policy_tables = policies();
    for (policy_index, policy) in policy_tables.iter().enumerate() {
        for population_size in POPULATION_SIZES {
            let config = GameConfig {
                population_size,
                memory_capacity: DEFAULT_MEMORY_CAPACITY,
                max_population_rounds: DEFAULT_MAX_POPULATION_ROUNDS,
                convergence_window: DEFAULT_CONVERGENCE_WINDOW,
                convergence_threshold: DEFAULT_CONVERGENCE_THRESHOLD,
                match_payoff: DEFAULT_MATCH_PAYOFF,
                mismatch_payoff: DEFAULT_MISMATCH_PAYOFF,
            };
            for replicate in 0..REPLICATES {
                let seed = stable_seed(population_size, policy_index, replicate);
                let result = simulate_population(config, policy, seed)
                    .expect("frozen synthetic group-size benchmark must validate");
                replicate_row(policy, population_size, replicate, seed, &result);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sweep_has_six_policies_and_seven_population_sizes() {
        assert_eq!(policies().len(), 6);
        assert_eq!(POPULATION_SIZES.len(), 7);
        assert_eq!(REPLICATES, 30);
    }

    #[test]
    fn seed_coordinates_are_deterministic_and_distinct() {
        let seed = stable_seed(24, 3, 11);
        assert_eq!(seed, stable_seed(24, 3, 11));
        assert_ne!(seed, stable_seed(24, 3, 12));
        assert_ne!(seed, stable_seed(100, 3, 11));
        assert_ne!(seed, stable_seed(24, 4, 11));
    }
}
