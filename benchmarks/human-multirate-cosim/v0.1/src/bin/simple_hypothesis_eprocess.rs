use human_multirate_cosim::experiment_design::{
    ExperimentDesignScenario, ForcingSegment, InputProfile, ModelCandidate,
};
use human_multirate_cosim::network::{Edge, NetworkScenario};
use human_multirate_cosim::noise_robustness::{
    Ar1KroneckerCovariance, DenseSpdCovariance, ObservationCovarianceV1, SplitMix64,
};
use human_multirate_cosim::sensor_design::SensorCandidate;
use human_multirate_cosim::sequential_evidence::{
    choose_greedy_information_action, decide_sequential_stop,
    ActionObservationV1, ModelPriorV1, SequentialActionV1,
    SequentialDecisionV1, SequentialPolicyConfigV1,
};
use human_multirate_cosim::simple_hypothesis_eprocess::{
    evaluate_simple_hypothesis_eprocess, SimpleHypothesisEProcessDecisionV1,
};
use std::error::Error;

fn model(id: u32, label: &'static str, bypass: bool) -> ModelCandidate {
    let mut edges = vec![
        Edge { left: 0, right: 1, conductance: 1.0 },
        Edge { left: 1, right: 2, conductance: 1.0 },
    ];
    if bypass {
        edges.push(Edge {
            left: 0,
            right: 2,
            conductance: 0.4,
        });
    }
    ModelCandidate {
        id,
        label,
        network: NetworkScenario {
            name: label,
            capacities: vec![1.0, 1.0, 1.0],
            initial: vec![0.2, 0.0, 0.0],
            edges,
            horizon: 2.0,
        },
    }
}

fn scenario() -> ExperimentDesignScenario {
    ExperimentDesignScenario {
        name: "simple-eprocess-chain-bypass",
        models: vec![
            model(1, "chain", false),
            model(2, "chain-with-bypass", true),
        ],
        sensors: vec![
            SensorCandidate {
                id: 10,
                node: 1,
                cost: 1.0,
                common_cause_group: None,
            },
            SensorCandidate {
                id: 11,
                node: 2,
                cost: 1.0,
                common_cause_group: None,
            },
        ],
        noise_covariance: vec![vec![0.02, 0.005], vec![0.005, 0.03]],
        observation_times: vec![0.4, 0.8, 1.2, 1.6, 2.0],
        sensor_budget: 2.0,
        max_abs_forcing: 2.0,
        input_energy_budget: 0.5,
    }
}

fn action(
    scenario: &ExperimentDesignScenario,
    id: u32,
    label: &'static str,
    node: usize,
    amplitude: f64,
    duration: f64,
) -> Result<SequentialActionV1, Box<dyn Error>> {
    let mut pulse = vec![0.0; 3];
    pulse[node] = amplitude;
    let profile = InputProfile {
        id,
        label,
        segments: vec![
            ForcingSegment {
                start_time: 0.0,
                end_time: duration,
                forcing: pulse,
            },
            ForcingSegment {
                start_time: duration,
                end_time: 2.0,
                forcing: vec![0.0, 0.0, 0.0],
            },
        ],
    };
    let covariance_id = format!("covariance:eprocess-action-{id}-ar1");
    let process_id = "process:eprocess-ar1-rho-0.6".to_owned();
    let sampling_id = "sampling:eprocess-uniform-five".to_owned();
    let operator_id = "operator:eprocess-sensors-10-11".to_owned();
    let cutoff_id = "availability:eprocess-all-observations".to_owned();
    let covariance = ObservationCovarianceV1::new(
        covariance_id.clone(),
        process_id.clone(),
        sampling_id.clone(),
        operator_id.clone(),
        cutoff_id.clone(),
        vec![10, 11],
        scenario.observation_times.clone(),
        Ar1KroneckerCovariance::new(
            scenario.observation_times.len(),
            0.6,
            DenseSpdCovariance::new(scenario.noise_covariance.clone())?,
        )?,
    )?;
    let identity = human_multirate_cosim::total_kl_trust_region::ObservationDesignIdentityV1 {
        covariance_id,
        observation_process_id: process_id,
        sampling_policy_id: sampling_id,
        observation_operator_id: operator_id,
        availability_cutoff_id: cutoff_id,
    };
    Ok(human_multirate_cosim::sequential_evidence::prepare_sequential_action(
        scenario,
        id,
        profile,
        vec![10, 11],
        covariance,
        identity,
        1.0,
    )?)
}

/// The truth ID is used only in evaluator-local sampling; it is not written
/// into the observation sent to the sequential policy.
fn evaluator_observation(
    scenario: &ExperimentDesignScenario,
    action: &SequentialActionV1,
    true_model_id: u32,
    seed: u64,
) -> Result<ActionObservationV1, Box<dyn Error>> {
    let means = action
        .model_means_for(true_model_id)
        .ok_or_else(|| std::io::Error::other("evaluator truth is not in the candidate set"))?;
    let mean_values: Vec<f64> = means.observations.iter().flatten().copied().collect();
    let mut rng = SplitMix64::new(seed);
    let noise = action.covariance().covariance().sample_gaussian(&mut rng)?;
    let values: Vec<f64> = mean_values
        .iter()
        .zip(noise.iter())
        .map(|(mean, error)| mean + error)
        .collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(std::io::Error::other("evaluator observation is non-finite").into());
    }
    let rows = values
        .chunks(action.selected_sensor_ids().len())
        .map(|row| row.to_vec())
        .collect();
    Ok(ActionObservationV1 {
        action_id: action.action_id(),
        sensor_ids: action.selected_sensor_ids().to_vec(),
        times: scenario.observation_times.clone(),
        values: rows,
    })
}

#[derive(Default)]
struct Counts {
    null_false_commit: usize,
    alternative_false_commit: usize,
    correct_commit: usize,
    abstain_or_insufficient: usize,
    experiments: usize,
}

fn wilson(successes: usize, trials: usize) -> (f64, f64, f64) {
    let z = 1.959_963_984_540_054_f64;
    let n = trials as f64;
    let p = successes as f64 / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let half = z
        * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt()
        / denominator;
    (
        p,
        (centre - half).max(0.0),
        (centre + half).min(1.0),
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let scenario = scenario();
    let prior = vec![
        ModelPriorV1 {
            model_id: 1,
            family_id: 100,
            probability: 0.5,
        },
        ModelPriorV1 {
            model_id: 2,
            family_id: 200,
            probability: 0.5,
        },
    ];
    let actions = vec![
        action(&scenario, 21, "source-node-pulse", 0, 1.0, 0.5)?,
        action(&scenario, 22, "sink-node-pulse", 2, -1.0, 0.5)?,
        action(&scenario, 23, "middle-node-pulse", 1, 0.8, 0.5)?,
    ];
    let config = SequentialPolicyConfigV1 {
        posterior_commit_threshold: 0.95,
        maximum_campaign_residual: 16.0,
        maximum_experiments: 3,
        maximum_total_cost: 3.0,
        eig_replicates_per_model: 128,
        eig_seed: 0x6132_E123,
    };
    let alpha = 0.05;
    let trials_per_model = 500_usize;
    let mut eprocess = [Counts::default(), Counts::default()];
    let mut fixed_posterior = [Counts::default(), Counts::default()];

    for truth_index in 0..2 {
        let truth_model_id = if truth_index == 0 { 1_u32 } else { 2_u32 };
        for trial in 0..trials_per_model {
            let seed = 0xE123_6132_0000_0000_u64
                ^ (truth_model_id as u64).wrapping_mul(0x9E37_79B9)
                ^ trial as u64;

            let mut e_history = Vec::new();
            let mut posterior_history = Vec::new();
            let mut e_stopped = false;
            let mut posterior_stopped = false;

            for look in 0..config.maximum_experiments {
                if !e_stopped {
                    let selected_id = choose_greedy_information_action(
                        &scenario,
                        &actions,
                        &prior,
                        &e_history,
                        config.maximum_total_cost,
                        config.eig_replicates_per_model,
                        seed ^ (look as u64).wrapping_mul(0xA5A5_6123),
                    )?
                    .action_id;
                    let selected = actions
                        .iter()
                        .find(|action| action.action_id() == selected_id)
                        .ok_or_else(|| std::io::Error::other("selected action is missing"))?;
                    let observation = evaluator_observation(
                        &scenario,
                        selected,
                        truth_model_id,
                        seed ^ u64::from(selected_id),
                    )?;
                    e_history.push(observation);
                    let e_report = evaluate_simple_hypothesis_eprocess(
                        &scenario,
                        &actions,
                        &prior,
                        &e_history,
                        1,
                        2,
                        alpha,
                    )?;
                    match e_report.final_decision {
                        SimpleHypothesisEProcessDecisionV1::SupportNull => {
                            if truth_model_id == 1 {
                                eprocess[truth_index].correct_commit += 1;
                            } else {
                                eprocess[truth_index].alternative_false_commit += 1;
                            }
                            e_stopped = true;
                        }
                        SimpleHypothesisEProcessDecisionV1::SupportAlternative => {
                            if truth_model_id == 2 {
                                eprocess[truth_index].correct_commit += 1;
                            } else {
                                eprocess[truth_index].null_false_commit += 1;
                            }
                            e_stopped = true;
                        }
                        SimpleHypothesisEProcessDecisionV1::InsufficientEvidence => {
                            if look + 1 == config.maximum_experiments {
                                eprocess[truth_index].abstain_or_insufficient += 1;
                                e_stopped = true;
                            }
                        }
                    }
                    eprocess[truth_index].experiments += 1;
                }

                if !posterior_stopped {
                    let selected_id = choose_greedy_information_action(
                        &scenario,
                        &actions,
                        &prior,
                        &posterior_history,
                        config.maximum_total_cost,
                        config.eig_replicates_per_model,
                        seed ^ (look as u64).wrapping_mul(0x5A5A_6132),
                    )?
                    .action_id;
                    let selected = actions
                        .iter()
                        .find(|action| action.action_id() == selected_id)
                        .ok_or_else(|| std::io::Error::other("posterior action is missing"))?;
                    let observation = evaluator_observation(
                        &scenario,
                        selected,
                        truth_model_id,
                        seed ^ u64::from(selected_id),
                    )?;
                    posterior_history.push(observation);
                    let report = decide_sequential_stop(
                        &scenario,
                        &actions,
                        &prior,
                        &posterior_history,
                        config,
                    )?;
                    match report.decision {
                        SequentialDecisionV1::CommitModel { model_id, .. } => {
                            if model_id == truth_model_id {
                                fixed_posterior[truth_index].correct_commit += 1;
                            } else if truth_model_id == 1 {
                                fixed_posterior[truth_index].null_false_commit += 1;
                            } else {
                                fixed_posterior[truth_index].alternative_false_commit += 1;
                            }
                            posterior_stopped = true;
                        }
                        SequentialDecisionV1::Continue { .. } => {
                            if look + 1 == config.maximum_experiments {
                                fixed_posterior[truth_index].abstain_or_insufficient += 1;
                                posterior_stopped = true;
                            }
                        }
                        _ => {
                            fixed_posterior[truth_index].abstain_or_insufficient += 1;
                            posterior_stopped = true;
                        }
                    }
                    fixed_posterior[truth_index].experiments += 1;
                }

                if e_stopped && posterior_stopped {
                    break;
                }
            }
        }
    }

    let report = |name: &str, counts: &Counts| {
        let trials = trials_per_model;
        let (null_rate, null_low, null_high) =
            wilson(counts.null_false_commit, trials);
        let (alt_rate, alt_low, alt_high) =
            wilson(counts.alternative_false_commit, trials);
        println!(
            "eprocess,{name},null_false_commit,{null_rate:.6},null_low,{null_low:.6},null_high,{null_high:.6},alternative_false_commit,{alt_rate:.6},alternative_low,{alt_low:.6},alternative_high,{alt_high:.6},correct_commit_rate,{:.6},abstain_rate,{:.6},mean_looks,{:.6}",
            counts.correct_commit as f64 / trials as f64,
            counts.abstain_or_insufficient as f64 / trials as f64,
            counts.experiments as f64 / trials as f64,
        );
    };

    report("e_process", &eprocess[0]);
    report("e_process", &eprocess[1]);
    report("fixed_posterior", &fixed_posterior[0]);
    report("fixed_posterior", &fixed_posterior[1]);
    println!(
        "eprocess,truth_strata,500_each,alpha_total,{:.4},per_direction_alpha,{:.4},threshold,{:.8},looks_max,{}",
        alpha,
        alpha / 2.0,
        (2.0 / alpha).ln(),
        config.maximum_experiments,
    );
    println!(
        "eprocess,claim,likelihood_ratio_e_process,adaptive_predictable_actions,conditional_independence_assumed"
    );
    Ok(())
}
