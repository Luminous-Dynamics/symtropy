use std::env;

use holofrac_spatial_retrieval::experiment_selection::{
    synthetic_voi_problem, ExperimentSelectionPolicy, PolicyOutcome,
};

const RANDOM_SEEDS: [u64; 10] = [11, 23, 37, 41, 53, 67, 79, 89, 101, 113];

fn print_outcome(outcome: &PolicyOutcome) {
    println!(
        "{policy},{seed},{selected},{mutual_information:.8},{cost:.8},{expected_value:.8},{net_value:.8},{regret:.8},{decision}",
        policy = outcome.policy.as_str(),
        seed = outcome.seed,
        selected = outcome.selected_experiment_id.as_deref().unwrap_or("none"),
        mutual_information = outcome.mutual_information_bits,
        cost = outcome.experiment_cost,
        expected_value = outcome.expected_value_after_observation,
        net_value = outcome.net_expected_decision_value,
        regret = outcome.expected_decision_regret,
        decision = outcome.decision_without_measurement
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac exact finite-state value-of-information benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("# random_seed_replicates={}", RANDOM_SEEDS.len());
    println!(
        "policy,seed,selected_experiment,mutual_information_bits,experiment_cost,expected_value_after_observation,net_expected_decision_value,expected_decision_regret,decision_without_measurement"
    );

    let problem = synthetic_voi_problem();
    problem.validate()?;

    for policy in [
        ExperimentSelectionPolicy::NoTest,
        ExperimentSelectionPolicy::MaximumInformationGain,
        ExperimentSelectionPolicy::ExpectedDecisionValue,
    ] {
        let outcome = problem.run_policy(policy, 0)?;
        print_outcome(&outcome);
    }

    for seed in RANDOM_SEEDS {
        let outcome = problem.run_policy(ExperimentSelectionPolicy::Random, seed)?;
        print_outcome(&outcome);
    }

    Ok(())
}
