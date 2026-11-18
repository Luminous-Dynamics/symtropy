use std::time::Instant;

use human_multirate_cosim::experiment_design::{
    ExperimentDesignScenario, ModelCandidate,
};
use human_multirate_cosim::network::{Edge, NetworkScenario};
use human_multirate_cosim::noise_robustness::{
    Ar1KroneckerCovariance, DenseSpdCovariance, ObservationCovarianceV1,
};
use human_multirate_cosim::sensor_design::SensorCandidate;
use human_multirate_cosim::total_kl_trust_region::{
    optimize_pairwise_total_kl_trust_region_with_covariance,
    ObservationDesignIdentityV1,
};

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
        name: "nested-control-mesh-chain-versus-bypass",
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

fn boundaries(segment_count: usize, horizon: f64) -> Vec<f64> {
    (0..=segment_count)
        .map(|index| {
            if index == segment_count {
                horizon
            } else {
                horizon * index as f64 / segment_count as f64
            }
        })
        .collect()
}

fn forcing_at(
    profile: &human_multirate_cosim::experiment_design::InputProfile,
    time: f64,
    actuators: &[usize],
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let segment = profile
        .segments
        .iter()
        .find(|segment| {
            time >= segment.start_time && time < segment.end_time
        })
        .ok_or_else(|| std::io::Error::other("control profile does not cover comparison time"))?;
    Ok(actuators
        .iter()
        .map(|node| {
            segment
                .forcing
                .get(*node)
                .copied()
                .ok_or_else(|| std::io::Error::other("actuator is outside forcing vector"))
        })
        .collect::<Result<Vec<_>, _>>()?)
}

fn normalized_projection_difference(
    previous: &human_multirate_cosim::experiment_design::InputProfile,
    current: &human_multirate_cosim::experiment_design::InputProfile,
    horizon: f64,
    comparison_segments: usize,
    actuators: &[usize],
    current_energy: f64,
) -> Result<f64, Box<dyn std::error::Error>> {
    let duration = horizon / comparison_segments as f64;
    let mut difference_squared = 0.0;
    for index in 0..comparison_segments {
        let midpoint = (index as f64 + 0.5) * duration;
        let previous_values = forcing_at(previous, midpoint, actuators)?;
        let current_values = forcing_at(current, midpoint, actuators)?;
        for (left, right) in previous_values.iter().zip(current_values.iter()) {
            let delta = left - right;
            difference_squared += duration * delta * delta;
        }
    }
    Ok(difference_squared.sqrt() / current_energy.sqrt().max(1.0e-15))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let scenario = scenario();
    let actuator_nodes = [0_usize, 2_usize];
    let selected_sensor_ids = [10_u32, 11_u32];
    let covariance_id = "covariance:mesh-study-ar1-rho-0.6-v1";
    let process_id = "observation-process:ar1-rho-0.6-v1";
    let sampling_id = "sampling:uniform-five-times-v1";
    let operator_id = "operator:selected-sensors-10-11-v1";
    let cutoff_id = "availability:all-five-observations-v1";

    let sensor_covariance =
        DenseSpdCovariance::new(scenario.noise_covariance.clone())?;
    let structured_covariance = Ar1KroneckerCovariance::new(
        scenario.observation_times.len(),
        0.6,
        sensor_covariance,
    )?;
    let covariance = ObservationCovarianceV1::new(
        covariance_id.to_owned(),
        process_id.to_owned(),
        sampling_id.to_owned(),
        operator_id.to_owned(),
        cutoff_id.to_owned(),
        selected_sensor_ids.to_vec(),
        scenario.observation_times.clone(),
        structured_covariance,
    )?;
    let identity = ObservationDesignIdentityV1 {
        covariance_id: covariance_id.to_owned(),
        observation_process_id: process_id.to_owned(),
        sampling_policy_id: sampling_id.to_owned(),
        observation_operator_id: operator_id.to_owned(),
        availability_cutoff_id: cutoff_id.to_owned(),
    };

    let segment_counts = [1_usize, 2, 4, 8];
    let mut previous_report = None;
    let mut previous_segments = 0_usize;
    let mut previous_total_kl = 0.0_f64;
    let horizon = scenario.horizon();

    println!(
        "mesh,segments,basis_dimension,total_kl,passive_kl,incremental_kl,energy,peak,multiplier,stationarity_residual,replay_residual,objective_gain,normalized_projection_delta,wall_seconds,disposition"
    );

    for segment_count in segment_counts {
        let partition = boundaries(segment_count, horizon);
        let start = Instant::now();
        let report = optimize_pairwise_total_kl_trust_region_with_covariance(
            &scenario,
            1,
            2,
            &selected_sensor_ids,
            &actuator_nodes,
            &partition,
            &covariance,
            &identity,
        )?;
        let elapsed = start.elapsed().as_secs_f64();

        if report.peak_constraint_active {
            return Err(std::io::Error::other(
                "mesh-convergence fixture unexpectedly activates peak constraint",
            )
            .into());
        }
        if report.achieved_total_gaussian_kl + 1.0e-9 < previous_total_kl {
            return Err(std::io::Error::other(
                "nested control basis reduced the global total-KL optimum",
            )
            .into());
        }

        let objective_gain = if previous_segments == 0 {
            0.0
        } else {
            report.achieved_total_gaussian_kl - previous_total_kl
        };
        let projection_delta = if let Some(previous) = previous_report.as_ref() {
            normalized_projection_difference(
                &previous.profile,
                &report.profile,
                horizon,
                segment_count,
                &actuator_nodes,
                report.achieved_integrated_squared_input,
            )?
        } else {
            0.0
        };

        println!(
            "mesh,{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.6e},{:?}",
            segment_count,
            report.basis_dimension,
            report.achieved_total_gaussian_kl,
            report.passive_baseline_gaussian_kl,
            report.achieved_incremental_control_gaussian_kl,
            report.achieved_integrated_squared_input,
            report.achieved_peak_amplitude,
            report.trust_region_multiplier,
            report.trust_region_stationarity_residual,
            report.replay_total_kl_abs_error,
            objective_gain,
            projection_delta,
            elapsed,
            report.optimality_disposition,
        );

        previous_total_kl = report.achieved_total_gaussian_kl;
        previous_segments = segment_count;
        previous_report = Some(report);
    }

    println!(
        "mesh,monotonicity_check,PASS,levels,{}",
        segment_counts.len(),
    );
    println!(
        "mesh,exactness_scope,finite_piecewise_constant_basis,arbitrary_waveform_claim,false"
    );
    Ok(())
}
