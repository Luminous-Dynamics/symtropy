use human_multirate_cosim::energy_optimal_control::optimize_pairwise_energy_ball_control;
use human_multirate_cosim::experiment_design::{
    evaluate_input_profile, ExperimentDesignScenario, ForcingSegment, InputProfile, ModelCandidate,
};
use human_multirate_cosim::network::{Edge, NetworkScenario};
use human_multirate_cosim::sensor_design::SensorCandidate;
use human_multirate_cosim::total_kl_trust_region::{
    optimize_pairwise_total_kl_trust_region, TotalKlOptimalityDisposition,
};

fn model(id: u32, label: &'static str, bypass: bool) -> ModelCandidate {
    let mut edges = vec![
        Edge { left: 0, right: 1, conductance: 1.0 },
        Edge { left: 1, right: 2, conductance: 1.0 },
    ];
    if bypass {
        edges.push(Edge { left: 0, right: 2, conductance: 0.4 });
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
        name: "total-kl-chain-versus-bypass",
        models: vec![
            model(1, "chain", false),
            model(2, "chain-with-bypass", true),
        ],
        sensors: vec![
            SensorCandidate { id: 10, node: 1, cost: 1.0, common_cause_group: None },
            SensorCandidate { id: 11, node: 2, cost: 1.0, common_cause_group: None },
        ],
        noise_covariance: vec![vec![0.02, 0.005], vec![0.005, 0.03]],
        observation_times: vec![0.25, 0.5, 1.0, 1.5, 2.0],
        sensor_budget: 2.0,
        max_abs_forcing,
        input_energy_budget: 0.5,
    }
}

fn pair_kl(
    score: &human_multirate_cosim::experiment_design::ExperimentDesignScore,
) -> Result<f64, Box<dyn std::error::Error>> {
    Ok(score
        .pairwise_separations
        .iter()
        .find(|pair| {
            (pair.left_model_id == 1 && pair.right_model_id == 2)
                || (pair.left_model_id == 2 && pair.right_model_id == 1)
        })
        .ok_or_else(|| std::io::Error::other("missing chain/bypass pair KL"))?
        .gaussian_kl_separation)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = scenario(2.0);
    let boundaries = [0.0, 0.5, 1.0, 2.0];
    let passive_profile = InputProfile {
        id: 1,
        label: "passive-reference",
        segments: vec![ForcingSegment {
            start_time: 0.0,
            end_time: 2.0,
            forcing: vec![0.0, 0.0, 0.0],
        }],
    };
    let passive_score = evaluate_input_profile(&scenario, &passive_profile, &[10, 11])?;
    let passive_kl = pair_kl(&passive_score)?;

    let incremental = optimize_pairwise_energy_ball_control(
        &scenario,
        1,
        2,
        &[10, 11],
        &[0, 1, 2],
        &boundaries,
    )?;
    let total = optimize_pairwise_total_kl_trust_region(
        &scenario,
        1,
        2,
        &[10, 11],
        &[0, 1, 2],
        &boundaries,
    )?;

    let mut profiles = vec![passive_profile];
    let mut next_id = 2_u32;
    for actuator in [0_usize, 2_usize] {
        for amplitude in [-1.0, -0.5, 0.5, 1.0] {
            for duration in [0.25, 0.5] {
                let mut pulse = vec![0.0; 3];
                pulse[actuator] = amplitude;
                profiles.push(InputProfile {
                    id: next_id,
                    label: "finite-grid-pulse",
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
                });
                next_id += 1;
            }
        }
    }

    let mut best_grid: Option<(u32, f64, f64, f64)> = None;
    for profile in &profiles {
        let score = evaluate_input_profile(&scenario, profile, &[10, 11])?;
        let value = pair_kl(&score)?;
        let energy = score.integrated_squared_input;
        let peak = score.maximum_abs_forcing;
        let candidate = (profile.id, value, energy, peak);
        let replace = best_grid.is_none_or(|current| {
            candidate.1 > current.1 + 1.0e-12
                || ((candidate.1 - current.1).abs() <= 1.0e-12
                    && (candidate.2 < current.2 - 1.0e-12
                        || ((candidate.2 - current.2).abs() <= 1.0e-12
                            && candidate.0 < current.0)))
        });
        if replace {
            best_grid = Some(candidate);
        }
    }
    let finite = best_grid.ok_or_else(|| std::io::Error::other("finite grid is empty"))?;

    println!("total_kl,passive_baseline,{:.12e}", passive_kl);
    println!(
        "total_kl,finite_grid,best_profile_id,{},total_controlled_kl,{:.12e},energy,{:.12e},peak,{:.12e}",
        finite.0, finite.1, finite.2, finite.3,
    );
    println!(
        "total_kl,incremental_eigen_design,total_controlled_kl,{:.12e},incremental_control_kl,{:.12e},energy,{:.12e},peak,{:.12e}",
        incremental.achieved_total_gaussian_kl,
        incremental.achieved_control_induced_gaussian_kl,
        incremental.achieved_integrated_squared_input,
        incremental.achieved_peak_amplitude,
    );
    println!(
        "total_kl,trust_region_design,disposition,{:?},total_controlled_kl,{:.12e},upper_bound,{:.12e},incremental_control_kl,{:.12e},energy,{:.12e},peak,{:.12e},multiplier,{:.12e},stationarity_residual,{:.12e},replay_abs_error,{:.12e}",
        total.optimality_disposition,
        total.achieved_total_gaussian_kl,
        total.unconstrained_total_gaussian_kl_upper_bound,
        total.achieved_incremental_control_gaussian_kl,
        total.achieved_integrated_squared_input,
        total.achieved_peak_amplitude,
        total.trust_region_multiplier,
        total.trust_region_stationarity_residual,
        total.replay_total_kl_abs_error,
    );
    if matches!(
        total.optimality_disposition,
        TotalKlOptimalityDisposition::ExactTotalKlTrustRegionOptimum
            | TotalKlOptimalityDisposition::HardCaseTotalKlTrustRegionOptimum
    ) {
        println!(
            "total_kl,comparison,trust_region_over_finite_grid,{:.12e}",
            total.achieved_total_gaussian_kl / finite.1.max(1.0e-300),
        );
        println!(
            "total_kl,comparison,trust_region_over_incremental_design_total,{:.12e}",
            total.achieved_total_gaussian_kl
                / incremental.achieved_total_gaussian_kl.max(1.0e-300),
        );
    }
    Ok(())
}
