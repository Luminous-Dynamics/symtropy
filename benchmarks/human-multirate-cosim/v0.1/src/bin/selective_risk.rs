use human_multirate_cosim::experiment_design::{
    ExperimentDesignScenario, ForcingSegment, InputProfile, ModelCandidate,
};
use human_multirate_cosim::network::{Edge, NetworkScenario};
use human_multirate_cosim::noise_robustness::{
    Ar1KroneckerCovariance, DenseSpdCovariance, ObservationCovarianceV1,
};
use human_multirate_cosim::selective_risk::{
    estimate_action_selective_risk_evaluator_only,
    estimate_held_out_model_false_acceptance_evaluator_only,
    risk_adjusted_action_score,
};
use human_multirate_cosim::sequential_evidence::{
    estimate_action_expected_information_gain, prepare_sequential_action,
    ModelPriorV1, SequentialActionV1, SequentialPolicyConfigV1,
};
use human_multirate_cosim::total_kl_trust_region::ObservationDesignIdentityV1;
use std::error::Error;

fn model(
    id: u32,
    label: &'static str,
    bypass_conductance: Option<f64>,
) -> ModelCandidate {
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
    if let Some(conductance) = bypass_conductance {
        edges.push(Edge {
            left: 0,
            right: 2,
            conductance,
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
        name: "selective-risk-chain-bypass",
        models: vec![
            model(1, "chain", None),
            model(2, "chain-with-bypass", Some(0.4)),
        ],
        sensors: vec![
            human_multirate_cosim::sensor_design::SensorCandidate {
                id: 10,
                node: 1,
                cost: 1.0,
                common_cause_group: None,
            },
            human_multirate_cosim::sensor_design::SensorCandidate {
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
    duration: f64,
) -> Result<SequentialActionV1, Box<dyn Error>> {
    let mut pulse = vec![0.0; 3];
    pulse[node] = amplitude;
    let profile = InputProfile {
        id: action_id,
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
    let covariance_id = format!("covariance:risk-action-{action_id}-ar1-rho-0.6");
    let process_id = "process:risk-ar1-rho-0.6".to_owned();
    let sampling_id = "sampling:risk-uniform-five".to_owned();
    let operator_id = "operator:risk-sensors-10-11".to_owned();
    let cutoff_id = "availability:risk-all-observations".to_owned();
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
    let identity = ObservationDesignIdentityV1 {
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

fn main() -> Result<(), Box<dyn Error>> {
    let candidate_scenario = scenario();
    let mut evaluator_scenario = candidate_scenario.clone();
    evaluator_scenario
        .models
        .push(model(99, "held-out-stronger-bypass", Some(0.9)));

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
        make_action(&candidate_scenario, 11, "source-node-pulse", 0, 1.0, 0.5)?,
        make_action(&candidate_scenario, 12, "sink-node-pulse", 2, -1.0, 0.5)?,
        make_action(&candidate_scenario, 13, "middle-node-pulse", 1, 0.8, 0.5)?,
    ];
    let config = SequentialPolicyConfigV1 {
        posterior_commit_threshold: 0.95,
        maximum_campaign_residual: 16.0,
        maximum_experiments: 1,
        maximum_total_cost: 1.0,
        eig_replicates_per_model: 512,
        eig_seed: 0x6129_5EED,
    };
    let risk_replicates = 1_000_usize;

    println!(
        "risk,action_id,profile_id,eig_per_cost,eig_mc_se,false_model_commit,false_family_commit,false_family_commit_wilson_low,false_family_commit_wilson_high,in_family_ood_rejection,parameter_unidentified,other_abstention,brier,log_loss"
    );
    for action in &actions {
        let eig = estimate_action_expected_information_gain(
            &candidate_scenario,
            &actions,
            &prior,
            &[],
            action.action_id(),
            config.eig_replicates_per_model,
            config.eig_seed,
        )?;
        let in_family = estimate_action_selective_risk_evaluator_only(
            &candidate_scenario,
            &actions,
            &prior,
            &[],
            action.action_id(),
            config,
            risk_replicates,
            config.eig_seed ^ u64::from(action.action_id()),
        )?;
        let false_family = in_family
            .per_model
            .iter()
            .map(|entry| {
                entry.prior_probability * entry.wrong_family_commit.estimate
            })
            .sum::<f64>();
        let _score_sweep: Vec<_> = [0.0_f64, 0.25, 0.5, 1.0, 2.0]
            .iter()
            .map(|penalty| risk_adjusted_action_score(&eig, &in_family, *penalty))
            .collect::<Result<Vec<_>, _>>()?;

        println!(
            "risk,{},{},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8},{:.8}",
            action.action_id(),
            action.profile_id(),
            eig.gain_per_cost,
            eig.standard_error_nats,
            in_family.expected_wrong_model_commit_probability,
            false_family,
            in_family
                .per_model
                .iter()
                .map(|entry| entry.prior_probability * entry.wrong_family_commit.lower_95)
                .sum::<f64>(),
            in_family
                .per_model
                .iter()
                .map(|entry| entry.prior_probability * entry.wrong_family_commit.upper_95)
                .sum::<f64>(),
            in_family.expected_in_family_ood_rejection_probability,
            in_family.expected_family_parameter_unidentified_probability,
            in_family.expected_other_abstention_probability,
            in_family.expected_brier_score,
            in_family.expected_log_loss,
        );

        let held_out = estimate_held_out_model_false_acceptance_evaluator_only(
            &candidate_scenario,
            &evaluator_scenario,
            &actions,
            &prior,
            &[],
            action.action_id(),
            99,
            config,
            risk_replicates,
            config.eig_seed ^ (u64::from(action.action_id()) << 1),
        )?;
        println!(
            "ood,{},{},false_candidate_acceptance,{:.8},wilson_low,{:.8},wilson_high,{:.8},correct_ood_abstention,{:.8},other_abstention,{:.8},mean_max_candidate_posterior,{:.8}",
            action.action_id(),
            held_out.held_out_model_id,
            held_out.false_candidate_acceptance.estimate,
            held_out.false_candidate_acceptance.lower_95,
            held_out.false_candidate_acceptance.upper_95,
            held_out.correct_out_of_model_abstention.estimate,
            held_out.other_abstention.estimate,
            held_out.mean_max_candidate_posterior,
        );
    }

    println!(
        "risk,evaluator,registered_models,2,held_out_models,1,replicates_per_truth,{},policy,one_step_stop_gate,claim,fixed_sample_Wilson_only",
        risk_replicates,
    );
    Ok(())
}
