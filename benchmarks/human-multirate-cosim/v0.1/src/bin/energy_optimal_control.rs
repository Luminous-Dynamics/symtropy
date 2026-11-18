use human_multirate_cosim::energy_optimal_control::{
    optimize_pairwise_energy_ball_control, EnergyBallOptimalityDisposition,
};
use human_multirate_cosim::experiment_design::{
    evaluate_input_tournament, ExperimentDesignScenario, ForcingSegment, InputProfile,
    ModelCandidate,
};
use human_multirate_cosim::network::{Edge, NetworkScenario};
use human_multirate_cosim::sensor_design::SensorCandidate;

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

fn scenario(max_abs_forcing: f64) -> ExperimentDesignScenario {
    ExperimentDesignScenario {
        name: "chain-versus-bypass-continuous-excitation",
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
        observation_times: vec![0.25, 0.5, 1.0, 1.5, 2.0],
        sensor_budget: 2.0,
        max_abs_forcing,
        input_energy_budget: 0.5,
    }
}

fn pulse_profile(id: u32, amplitude: f64, duration: f64) -> InputProfile {
    InputProfile {
        id,
        label: "finite-grid-single-pulse",
        segments: vec![
            ForcingSegment {
                start_time: 0.0,
                end_time: duration,
                forcing: vec![amplitude, 0.0, 0.0],
            },
            ForcingSegment {
                start_time: duration,
                end_time: 2.0,
                forcing: vec![0.0, 0.0, 0.0],
            },
        ],
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = scenario(2.0);
    let mut profiles = vec![InputProfile {
        id: 1,
        label: "passive-zero-input",
        segments: vec![ForcingSegment {
            start_time: 0.0,
            end_time: 2.0,
            forcing: vec![0.0, 0.0, 0.0],
        }],
    }];
    let mut next_id = 2;
    for amplitude in [-1.0, -0.5, 0.5, 1.0] {
        for duration in [0.25, 0.5] {
            profiles.push(pulse_profile(next_id, amplitude, duration));
            next_id += 1;
        }
    }

    let finite_grid = evaluate_input_tournament(&scenario, &profiles, &[10, 11])?;
    let discrete = finite_grid
        .best_worst_case
        .as_ref()
        .ok_or_else(|| {
            std::io::Error::other("finite-grid tournament returned no feasible profile")
        })?;
    let continuous = optimize_pairwise_energy_ball_control(
        &scenario,
        1,
        2,
        &[10, 11],
        &[0, 1, 2],
        &[0.0, 0.5, 1.0, 2.0],
    )?;

    println!(
        "energy_control,finite_grid,best_profile_id,{},minimum_pairwise_kl,{:.12e},energy,{:.12e},peak,{:.12e}",
        discrete.profile_id,
        discrete.minimum_pairwise_gaussian_kl,
        discrete.integrated_squared_input,
        discrete.maximum_abs_forcing,
    );
    println!(
        "energy_control,continuous_energy_ball,disposition,{:?},basis_dimension,{},top_eigenvalue,{:.12e},unconstrained_upper_bound,{:.12e},achieved_kl,{:.12e},energy,{:.12e},peak,{:.12e},replay_abs_error,{:.12e}",
        continuous.optimality_disposition,
        continuous.basis_dimension,
        continuous.top_generalized_eigenvalue,
        continuous.unconstrained_energy_ball_upper_bound,
        continuous.achieved_gaussian_kl,
        continuous.achieved_integrated_squared_input,
        continuous.achieved_peak_amplitude,
        continuous.replay_objective_abs_error,
    );
    if continuous.optimality_disposition
        == EnergyBallOptimalityDisposition::ExactEnergyBallOptimum
    {
        println!(
            "energy_control,comparison,continuous_over_grid,{:.12e}",
            continuous.achieved_gaussian_kl
                / discrete.minimum_pairwise_gaussian_kl.max(1.0e-300),
        );
    }
    for coefficient in &continuous.coefficients {
        println!(
            "energy_control,coefficient,segment,{},start,{:.6},end,{:.6},node,{},amplitude,{:.12e}",
            coefficient.segment_index,
            coefficient.start_time,
            coefficient.end_time,
            coefficient.actuator_node,
            coefficient.amplitude,
        );
    }
    Ok(())
}
