//! Deterministic training-split transition collection for causal residual learning.
//!
//! The exported targets are computed only from observable state transitions and the
//! applied command. Hidden force values are intentionally never written to each sample
//! so they cannot accidentally become a supervised input feature.

use serde_json::json;
use symthaea_multirotor::{pid_baseline, PdGains, PidState, PhysicsSimulator};
use symtropy_real_symthaea_physics_loop::{
    scenario_matrix::{scenarios_for_split, ScenarioSplit},
    symtropy_flight::SymtropyFlightSimulator,
    transition_residual::{LinearTransitionResidual, NominalLinearDynamics},
};

const MASS_KG: f64 = 0.027;
const GRAVITY_M_S2: f64 = 9.81;
const LINEAR_DRAG_N_S_PER_M: f64 = 0.01;
const DT_S: f64 = 1.0 / 500.0;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
        .max(1)
}

fn signed_unit(mut value: u64) -> f64 {
    // Fixed integer mixing: reproducible without an additional RNG dependency.
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    (value as f64 / u64::MAX as f64) * 2.0 - 1.0
}

/// Training-only exogenous forces. The exact force is deliberately not written into
/// dataset rows; the target extractor must recover unexplained acceleration from states.
fn randomized_training_force(seed: u64, step: usize) -> [f64; 3] {
    let weight_n = MASS_KG * GRAVITY_M_S2;
    let amplitude_seed = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    let amplitude = (0.02 + ((amplitude_seed >> 32) % 5) as f64 * 0.02) * weight_n;
    let onset = 20 + (seed % 31) as usize;
    let t = step as f64 * DT_S;

    match seed % 4 {
        0 => {
            if (onset..onset + 45).contains(&step) {
                [amplitude, 0.0, 0.0]
            } else {
                [0.0; 3]
            }
        }
        1 => [
            amplitude * (2.0 * t).sin(),
            amplitude * 0.7 * (1.37 * t + seed as f64 * 1.0e-5).cos(),
            0.0,
        ],
        2 => {
            let block = (step / 25) as u64;
            [
                amplitude * signed_unit(seed ^ block.wrapping_mul(0x9e37_79b9)),
                amplitude * 0.6 * signed_unit(seed.rotate_left(13) ^ block),
                0.0,
            ]
        }
        _ => {
            if step < onset {
                [0.0; 3]
            } else {
                [
                    amplitude * (0.83 * t).sin(),
                    amplitude * 0.5 * (1.61 * t).cos(),
                    amplitude * 0.2 * (0.4 * t).sin(),
                ]
            }
        }
    }
}

fn state_json(state: &symthaea_multirotor::FlightState) -> serde_json::Value {
    json!({
        "timestamp_s": state.timestamp,
        "position_world_m": state.position,
        "quaternion_wxyz": state.quaternion,
        "linear_velocity_world_m_s": state.linear_velocity,
        "angular_velocity_body_rad_s": state.angular_velocity
    })
}

fn main() {
    let episodes = env_usize("STFR_DATASET_EPISODES", 4);
    let steps_per_episode = env_usize("STFR_DATASET_STEPS", 512);
    let scenarios = scenarios_for_split(ScenarioSplit::Train);
    assert!(!scenarios.is_empty(), "training scenario catalogue must not be empty");

    let model = NominalLinearDynamics {
        mass_kg: MASS_KG,
        gravity_m_s2: GRAVITY_M_S2,
        linear_drag_n_s_per_m: LINEAR_DRAG_N_S_PER_M,
    };
    let gains = PdGains::default();
    let mut simulator = SymtropyFlightSimulator::new();
    let mut samples = Vec::with_capacity(episodes * steps_per_episode);
    let mut residual_norm_sum = 0.0_f64;
    let mut residual_norm_max = 0.0_f64;
    let mut contact_steps = 0_u64;
    let mut invalid_targets = 0_u64;
    let mut disturbance_family_counts = [0_u64; 4];

    for episode in 0..episodes {
        let seed = 0x5359_4D54_5252_0000_u64
            .wrapping_add(episode as u64 + 1);
        let scenario = &scenarios[episode % scenarios.len()];
        simulator.reset_with_perturbation(
            scenario.setpoint.position[2],
            0.06,
            seed,
        );
        let mut pid_state = PidState::default();
        disturbance_family_counts[(seed % 4) as usize] += 1;

        for step in 0..steps_per_episode {
            let before = simulator.state().clone();
            let requested = pid_baseline(
                &before,
                &scenario.setpoint,
                &gains,
                &mut pid_state,
                DT_S,
            )
            .clamped();
            let applied_action = requested.clamped();
            let force = randomized_training_force(seed, step);
            simulator.apply_external_force(force);
            simulator.step(&applied_action, DT_S);
            let after = simulator.state().clone();

            let target = LinearTransitionResidual::from_transition(
                &before,
                &applied_action,
                &after,
                model,
            )
            .unwrap_or_else(|error| {
                invalid_targets += 1;
                panic!(
                    "refusing to emit invalid supervised target; episode={episode}, step={step}, error={error:?}"
                )
            });

            let residual = target.residual_acceleration_world_m_s2;
            let norm = residual.iter().map(|v| v * v).sum::<f64>().sqrt();
            residual_norm_sum += norm;
            residual_norm_max = residual_norm_max.max(norm);

            samples.push(json!({
                "episode_index": episode,
                "transition_index": step,
                "split": "train",
                "scenario_id": scenario.id,
                "dt_s": target.dt_s,
                "state_t": state_json(&before),
                "applied_action_ctrl": applied_action.to_ctrl(),
                "state_t1": state_json(&after),
                "target": {
                    "observed_acceleration_world_m_s2": target.observed_acceleration_world_m_s2,
                    "nominal_acceleration_world_m_s2": target.nominal_acceleration_world_m_s2,
                    "residual_acceleration_world_m_s2": residual
                }
            }));
        }
        contact_steps += simulator.ground_contact_count();
    }

    println!("{}", json!({
        "benchmark_id": "symtropy-causal-transition-residual-dataset-v0.1",
        "claim_ceiling": "training_split_transition_dataset_and_target_plumbing_only",
        "source_receipt": {
            "symtropy_git_sha": std::env::var("STFR_SYMTROPY_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "symthaea_git_sha": std::env::var("STFR_SYMTHAEA_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "rustc_version": std::env::var("STFR_RUSTC_VERSION").unwrap_or_else(|_| "unset".to_owned()),
            "scenario_matrix_sha256": std::env::var("STFR_SCENARIO_MATRIX_SHA256").unwrap_or_else(|_| "unset".to_owned())
        },
        "collection": {
            "episodes": episodes,
            "steps_per_episode": steps_per_episode,
            "sample_count": samples.len(),
            "simulator": "Symtropy PhysicsWorld simplified spherical-body backend",
            "controller": "PdGains::default with pid_baseline; data-collection policy only",
            "dt_s": DT_S,
            "training_scenario_ids": scenarios.iter().map(|scenario| scenario.id).collect::<Vec<_>>(),
            "training_randomization": {
                "families": [
                    "seeded_lateral_pulse",
                    "correlated_sinusoid",
                    "piecewise_held_noise",
                    "mixed_axis_time_varying_gust"
                ],
                "episode_family_counts": disturbance_family_counts,
                "force_values_exported_per_sample": false,
                "force_labels_available_to_target_extractor": false
            },
            "nominal_model_id": "linear-thrust-gravity-linear-drag-v0.1",
            "nominal_model": {
                "mass_kg": model.mass_kg,
                "gravity_m_s2": model.gravity_m_s2,
                "linear_drag_n_s_per_m": model.linear_drag_n_s_per_m
            },
            "invalid_target_count": invalid_targets,
            "ground_contact_steps": contact_steps,
            "mean_residual_norm_m_s2": residual_norm_sum / samples.len().max(1) as f64,
            "max_residual_norm_m_s2": residual_norm_max
        },
        "feature_boundary": "Only state_t, applied_action_ctrl, and dt_s are model inputs; state_t1 is used to derive the target, and split/seed/provenance fields are metadata.",
        "samples": samples
    }));
}
