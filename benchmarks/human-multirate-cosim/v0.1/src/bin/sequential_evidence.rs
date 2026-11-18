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
    estimate_action_expected_information_gain, prepare_sequential_action,
    ActionObservationV1, ModelPriorV1, SequentialActionV1, SequentialDecisionV1,
    SequentialPolicyConfigV1,
};
use std::error::Error;

fn model(id: u32, label: &'static str, bypass: bool) -> ModelCandidate {
    let mut edges = vec![
        Edge {
            left: 0,
            right: 1,
            conductance: 1.0,
        },
        Edge {
            left: 1,
            right: 2,
            conductance: 1.0,
        },
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
        name: "sequential-chain-versus-bypass",
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

fn make_action(
    scenario: &ExperimentDesignScenario,
    action_id: u32,
    label: &'static str,
    node: usize,
    amplitude: f64,
    pulse_duration: f64,
) -> Result<SequentialActionV1, Box<dyn Error>> {
    let mut pulse = vec![0.0; 3];
    pulse[node] = amplitude;
    let profile = InputProfile {
        id: action_id,
        label,
        segments: vec![
            ForcingSegment {
                start_time: 0.0,
                end_time: pulse_duration,
                forcing: pulse,
            },
            ForcingSegment {
                start_time: pulse_duration,
                end_time: 2.0,
                forcing: vec![0.0; 3],
            },
        ],
    };
    let covariance_id = format!("covariance:sequential-action-{action_id}-ar1-rho-0.6");
    let process_id = "process:sequential-ar1-rho-0.6".to_owned();
    let sampling_id = "sampling:uniform-five-times".to_owned();
    let operator_id = "operator:sensors-10-11".to_owned();
    let cutoff_id = "availability:all-five-observations".to_owned();
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
    Ok(prepare_sequential_action(
        scenario,
        action_id,
        profile,
        vec![10, 11],
        covariance,
        identity,
        1.0,
    )?)
}

/// Evaluator-only generator. The true model ID is intentionally supplied only
/// here and never inserted into ActionObservationV1 or policy structures.
fn evaluator_sample(
    action: &SequentialActionV1,
    true_model_id: u32,
    scenario: &ExperimentDesignScenario,
    seed: u64,
) -> Result<ActionObservationV1, Box<dyn Error>> {
    let means = action
        .model_means
        .iter()
        .find(|item| item.model_id == true_model_id)
        .ok_or_else(|| std::io::Error::other("evaluator truth is not a candidate model"))?;
    let flattened: Vec<f64> = means.observations.iter().flatten().copied().collect();
    let mut rng = SplitMix64::new(seed);
    let noise = action.covariance.covariance().sample_gaussian(&mut rng)?;
    if flattened.len() != noise.len() {
        return Err(std::io::Error::other("noise/prediction dimensions differ").into());
    }
    let values: Vec<f64> = flattened
        .iter()
        .zip(noise.iter())
        .map(|(mean, error)| mean + error)
        .collect();
    if values.iter().any(|value| !value.is_finite()) {
        return Err(std::io::Error::other("evaluator generated non-finite observation").into());
    }
    let rows = values
        .chunks(action.selected_sensor_ids.len())
        .map(|row| row.to_vec())
        .collect();
    Ok(ActionObservationV1 {
        action_id: action.action_id,
        sensor_ids: action.selected_sensor_ids.clone(),
        times: scenario.observation_times.clone(),
        values: rows,
    })
}

#[derive(Default)]
struct Counts {
    correct_model_commits: usize,
    wrong_model_commits: usize,
    parameter_not_identified_commits: usize,
    abstentions: usize,
}

fn record_decision(counts: &mut Counts, decision: &SequentialDecisionV1, truth: u32) {
    match decision {
        SequentialDecisionV1::CommitModel { model_id, .. } if *model_id == truth => {
            counts.correct_model_commits += 1;
        }
        SequentialDecisionV1::CommitModel { .. } => {
            counts.wrong_model_commits += 1;
        }
        SequentialDecisionV1::CommitFamilyParameterNotIdentified { .. } => {
            counts.parameter_not_identified_commits += 1;
        }
        _ => counts.abstentions += 1,
    }
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
        make_action(&scenario, 11, "early-source-pulse", 0, 1.0, 0.5)?,
        make_action(&scenario, 12, "early-sink-pulse", 2, -1.0, 0.5)?,
        make_action(&scenario, 13, "short-middle-pulse", 1, 0.8, 0.5)?,
    ];
    let config = SequentialPolicyConfigV1 {
        posterior_commit_threshold: 0.95,
        maximum_campaign_residual: 12.0,
        maximum_experiments: 2,
        eig_replicates_per_model: 256,
        eig_seed: 0x5EED_6127,
    };

    let mut rankings = Vec::new();
    for action in &actions {
        rankings.push(estimate_action_expected_information_gain(
            &scenario,
            &actions,
            &prior,
            &[],
            action.action_id,
            config.eig_replicates_per_model,
            config.eig_seed,
        )?);
    }
    rankings.sort_by(|left, right| {
        right
            .gain_per_cost
            .total_cmp(&left.gain_per_cost)
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    let open_loop_sequence: Vec<u32> = rankings
        .iter()
        .take(config.maximum_experiments)
        .map(|score| score.action_id)
        .collect();
    let first_action = choose_greedy_information_action(
        &scenario,
        &actions,
        &prior,
        &[],
        config.eig_replicates_per_model,
        config.eig_seed,
    )?
    .action_id;
    if open_loop_sequence.first().copied() != Some(first_action) {
        return Err(std::io::Error::other(
            "open-loop ranking and greedy first action disagree",
        )
        .into());
    }

    let trials = 400_usize;
    let mut open_loop = Counts::default();
    let mut adaptive_greedy = Counts::default();

    for trial in 0..trials {
        // Balanced held-out evaluator truth schedule, not exposed to either policy.
        let truth = if trial % 2 == 0 { 1_u32 } else { 2_u32 };
        let trial_seed = 0xA11C_E612_7000_0000_u64 ^ trial as u64 * 0x9E37_79B9;

        let first = actions
            .iter()
            .find(|action| action.action_id == first_action)
            .ok_or_else(|| std::io::Error::other("first action disappeared"))?;
        let first_observation = evaluator_sample(
            first,
            truth,
            &scenario,
            trial_seed ^ u64::from(first.action_id),
        )?;

        let mut open_history = vec![first_observation.clone()];
        let fixed_second = open_loop_sequence[1];
        let second_open = actions
            .iter()
            .find(|action| action.action_id == fixed_second)
            .ok_or_else(|| std::io::Error::other("fixed second action disappeared"))?;
        open_history.push(evaluator_sample(
            second_open,
            truth,
            &scenario,
            trial_seed ^ u64::from(second_open.action_id),
        )?);
        let open_report =
            decide_sequential_stop(&scenario, &actions, &prior, &open_history, config)?;
        record_decision(&mut open_loop, &open_report.decision, truth);

        let mut adaptive_history = vec![first_observation];
        let selected_second = choose_greedy_information_action(
            &scenario,
            &actions,
            &prior,
            &adaptive_history,
            config.eig_replicates_per_model,
            trial_seed ^ config.eig_seed,
        )?
        .action_id;
        let second_adaptive = actions
            .iter()
            .find(|action| action.action_id == selected_second)
            .ok_or_else(|| std::io::Error::other("adaptive second action disappeared"))?;
        adaptive_history.push(evaluator_sample(
            second_adaptive,
            truth,
            &scenario,
            trial_seed ^ u64::from(second_adaptive.action_id),
        )?);
        let adaptive_report =
            decide_sequential_stop(&scenario, &actions, &prior, &adaptive_history, config)?;
        record_decision(&mut adaptive_greedy, &adaptive_report.decision, truth);
    }

    let rate = |count: usize| count as f64 / trials as f64;
    println!("sequential,policy,open_loop,first_action,{},second_action,{}", open_loop_sequence[0], open_loop_sequence[1]);
    println!("sequential,policy,adaptive_greedy,first_action,{},second_action,conditional", first_action);
    println!(
        "sequential,open_loop,correct_model_commit_rate,{:.6},wrong_model_commit_rate,{:.6},family_param_not_identified_rate,{:.6},abstention_rate,{:.6}",
        rate(open_loop.correct_model_commits),
        rate(open_loop.wrong_model_commits),
        rate(open_loop.parameter_not_identified_commits),
        rate(open_loop.abstentions),
    );
    println!(
        "sequential,adaptive_greedy,correct_model_commit_rate,{:.6},wrong_model_commit_rate,{:.6},family_param_not_identified_rate,{:.6},abstention_rate,{:.6}",
        rate(adaptive_greedy.correct_model_commits),
        rate(adaptive_greedy.wrong_model_commits),
        rate(adaptive_greedy.parameter_not_identified_commits),
        rate(adaptive_greedy.abstentions),
    );
    println!(
        "sequential,evaluator,trials,{},truth_balance,50/50,commit_threshold,{:.4},campaign_residual_threshold,{:.4},eig_replicates_per_model,{}",
        trials,
        config.posterior_commit_threshold,
        config.maximum_campaign_residual,
        config.eig_replicates_per_model,
    );
    println!("sequential,claim_ceiling,Monte_Carlo_greedy_reference,no_sequential_error_guarantee");
    Ok(())
}
