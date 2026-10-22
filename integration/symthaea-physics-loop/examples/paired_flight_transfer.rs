//! Paired transfer experiment: train Symthaea on its native simple plant,
//! then evaluate on Symtropy PhysicsWorld against same-world PD and untrained
//! Symthaea controls. No policy is assumed to win.

use serde::Serialize;
use std::time::Instant;
use serde_json::json;
use symthaea_core::genesis::GenesisSeed;
use symthaea_multirotor::{
    pd_baseline, ActiveInferenceFlightAgent, FlightConfig, FlightController,
    FlightFepConfig, FlightSetpoint, FlightTrainer, PdGains, PhysicsSimulator,
    QuadrotorHdcEncoder,
};
use symtropy_real_symthaea_physics_loop::symtropy_flight::SymtropyFlightSimulator;

#[derive(Debug, Clone, Copy, Serialize)]
struct Metrics {
    rmse_m: f64,
    mean_position_error_m: f64,
    final_position_error_m: f64,
    max_position_error_m: f64,
    within_5cm_fraction: f64,
    thrust_impulse_n_s: f64,
    ground_contact_steps: u64,
    invalid_inputs: u64,
    controller_compute_p50_us: f64,
    controller_compute_p95_us: f64,
    fep_update_p95_us: f64,
}

fn percentile(values: &[f64], q: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * q.clamp(0.0, 1.0)).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

/// One deterministic disturbance schedule per seed, shared by every policy in a pair.
fn disturbance_for_seed(seed: u64, step: usize) -> Option<[f64; 3]> {
    let first_start = 60 + (seed % 20) as usize;
    let second_start = 200 + (seed % 20) as usize;
    if (first_start..first_start + 40).contains(&step)
        || (second_start..second_start + 40).contains(&step)
    {
        let amplitude = 0.001 + (seed % 5) as f64 * 0.0004;
        let x_sign = if seed % 2 == 0 { 1.0 } else { -1.0 };
        let y_sign = if seed % 3 == 0 { 1.0 } else { -1.0 };
        Some([amplitude * x_sign, amplitude * 0.5 * y_sign, 0.0])
    } else {
        None
    }
}

fn evaluate_pd(
    seed: u64,
    steps: usize,
    setpoint: &FlightSetpoint,
    gains: &PdGains,
    force_enabled: bool,
) -> Metrics {
    let mut sim = SymtropyFlightSimulator::new();
    sim.reset_with_perturbation(setpoint.position[2], 0.08, seed);
    let (mut squared, mut absolute, mut impulse) = (0.0, 0.0, 0.0);
    let mut controller_latencies_us = Vec::with_capacity(steps);
    let mut max_error: f64 = 0.0;
    let mut within = 0usize;
    let dt = 1.0 / 500.0;
    for step in 0..steps {
        let state = sim.state().clone();
        let error = setpoint.position_error_magnitude(&state);
        squared += error * error;
        absolute += error;
        max_error = max_error.max(error);
        if error < 0.05 {
            within += 1;
        }
        let command_started = Instant::now();
        let command = pd_baseline(&state, setpoint, gains).clamped();
        controller_latencies_us.push(command_started.elapsed().as_secs_f64() * 1.0e6);
        impulse += command.thrust as f64 * dt;
        if force_enabled {
            if let Some(force) = disturbance_for_seed(seed, step) {
                sim.apply_external_force(force);
            }
        }
        sim.step(&command, dt);
    }
    Metrics {
        rmse_m: (squared / steps.max(1) as f64).sqrt(),
        mean_position_error_m: absolute / steps.max(1) as f64,
        final_position_error_m: setpoint.position_error_magnitude(sim.state()),
        max_position_error_m: max_error,
        within_5cm_fraction: within as f64 / steps.max(1) as f64,
        thrust_impulse_n_s: impulse,
        ground_contact_steps: sim.ground_contact_count(),
        invalid_inputs: sim.invalid_input_count(),
        controller_compute_p50_us: percentile(&controller_latencies_us, 0.50),
        controller_compute_p95_us: percentile(&controller_latencies_us, 0.95),
        fep_update_p95_us: 0.0,
    }
}

fn evaluate_symthaea(
    seed: u64,
    steps: usize,
    setpoint: &FlightSetpoint,
    config: &FlightConfig,
    template: &FlightController,
    force_enabled: bool,
    use_fep: bool,
) -> Metrics {
    let genesis = GenesisSeed::from_phrase(&config.genesis_phrase);
    let mut encoder = QuadrotorHdcEncoder::new(&genesis, config.num_levels);
    let mut controller = template.clone();
    controller.reset();
    let mut fep = ActiveInferenceFlightAgent::new(FlightFepConfig::default());
    fep.reset();

    let mut sim = SymtropyFlightSimulator::new();
    sim.reset_with_perturbation(setpoint.position[2], 0.08, seed);
    let mut fep_result = fep.initial_step(sim.state(), setpoint);

    let (mut squared, mut absolute, mut impulse) = (0.0, 0.0, 0.0);
    let mut controller_latencies_us = Vec::with_capacity(steps);
    let mut fep_latencies_us = Vec::with_capacity(steps / 20 + 1);
    let mut max_error: f64 = 0.0;
    let mut within = 0usize;
    let dt = 1.0 / 500.0;
    let cognitive_interval = (config.motor_hz / config.cognitive_hz).max(1.0) as usize;

    for step in 0..steps {
        let state = sim.state().clone();
        let error = setpoint.position_error_magnitude(&state);
        squared += error * error;
        absolute += error;
        max_error = max_error.max(error);
        if error < 0.05 {
            within += 1;
        }

        let observation = encoder.encode(&state);
        let command_started = Instant::now();
        let mut command = controller.forward(&observation, dt as f32);
        controller_latencies_us.push(command_started.elapsed().as_secs_f64() * 1.0e6);
        if use_fep {
            if let Some(noise) = &fep_result.exploration_noise {
                command = command.with_noise(noise);
            }
        }
        command = command.clamped();
        impulse += command.thrust as f64 * dt;

        if force_enabled {
            if let Some(force) = disturbance_for_seed(seed, step) {
                sim.apply_external_force(force);
            }
        }
        sim.step(&command, dt);

        if use_fep && step % cognitive_interval == 0 {
            let fep_started = Instant::now();
            fep_result = fep.step(sim.state(), setpoint);
            fep_latencies_us.push(fep_started.elapsed().as_secs_f64() * 1.0e6);
            controller.modulate_tau(fep_result.tau_factor);
        }
        if step > 0 && step % 100 == 0 {
            controller.normalize_states();
        }
    }

    Metrics {
        rmse_m: (squared / steps.max(1) as f64).sqrt(),
        mean_position_error_m: absolute / steps.max(1) as f64,
        final_position_error_m: setpoint.position_error_magnitude(sim.state()),
        max_position_error_m: max_error,
        within_5cm_fraction: within as f64 / steps.max(1) as f64,
        thrust_impulse_n_s: impulse,
        ground_contact_steps: sim.ground_contact_count(),
        invalid_inputs: sim.invalid_input_count(),
        controller_compute_p50_us: percentile(&controller_latencies_us, 0.50),
        controller_compute_p95_us: percentile(&controller_latencies_us, 0.95),
        fep_update_p95_us: percentile(&fep_latencies_us, 0.95),
    }
}

fn main() {
    let train_episodes = std::env::var("STFB_TRAIN_EPISODES")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(4usize);
    let train_steps = std::env::var("STFB_TRAIN_STEPS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(256usize);
    let eval_steps = std::env::var("STFB_EVAL_STEPS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(320usize);
    let trials = std::env::var("STFB_TRIALS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(30usize);
    let force_enabled = std::env::var("STFB_FORCE_DISTURBANCE")
        .map(|v| v != "0").unwrap_or(true);

    let setpoint = FlightSetpoint {
        position: [0.04, -0.025, 0.15],
        yaw: 0.0,
    };
    let config = FlightConfig {
        num_episodes: train_episodes.max(1),
        steps_per_episode: train_steps.max(1),
        train_every: 4,
        replay_buffer_size: 16,
        replay_count: 1,
        enable_lr_schedule: true,
        early_termination: true,
        genesis_phrase: "symthaea-symtropy-sim-transfer-v0.1".to_owned(),
        curriculum: vec![setpoint.clone()],
        ..FlightConfig::default()
    };

    // Train on Symthaea's native simple plant, then evaluate on Symtropy
    // without further policy training.
    let mut trainer = FlightTrainer::new(config.clone());
    let (trained_controller, training_metrics) = trainer.train_returning_controller();
    let genesis = GenesisSeed::from_phrase(&config.genesis_phrase);
    let untrained_controller = FlightController::new(&genesis, &config);
    let gains = PdGains::default();

    let mut paired_rows = Vec::new();
    for index in 0..trials.max(1) {
        let seed = 0x53594d5400000000_u64.wrapping_add(index as u64 + 1);
        let pd = evaluate_pd(seed, eval_steps, &setpoint, &gains, force_enabled);
        let untrained = evaluate_symthaea(
            seed, eval_steps, &setpoint, &config, &untrained_controller, force_enabled, false,
        );
        let trained_no_online_fep = evaluate_symthaea(
            seed, eval_steps, &setpoint, &config, &trained_controller, force_enabled, false,
        );
        let trained_with_fep = evaluate_symthaea(
            seed, eval_steps, &setpoint, &config, &trained_controller, force_enabled, true,
        );
        paired_rows.push(json!({
            "trial_seed": seed,
            "evidence_class": "simulator_experiment",
            "physics_backend": "Symtropy PhysicsWorld",
            "training_backend": "Symthaea SimplePhysicsSimulator",
            "controller": {
                "pd_baseline": pd,
                "symthaea_untrained_no_fep_ablation": untrained,
                "symthaea_trained_no_online_fep": trained_no_online_fep,
                "symthaea_trained_with_fep": trained_with_fep
            },
            "deltas_vs_pd": {
                "trained_with_fep_rmse_m": trained_with_fep.rmse_m - pd.rmse_m,
                "trained_with_fep_final_error_m": trained_with_fep.final_position_error_m - pd.final_position_error_m,
                "trained_with_fep_within_5cm_fraction": trained_with_fep.within_5cm_fraction - pd.within_5cm_fraction,
                "trained_no_online_fep_rmse_m": trained_no_online_fep.rmse_m - pd.rmse_m,
                "untrained_rmse_m": untrained.rmse_m - pd.rmse_m
            },
            "fep_effect_vs_trained_no_fep": {
                "rmse_m": trained_with_fep.rmse_m - trained_no_online_fep.rmse_m,
                "within_5cm_fraction": trained_with_fep.within_5cm_fraction - trained_no_online_fep.within_5cm_fraction,
                "controller_compute_p95_us": trained_with_fep.controller_compute_p95_us - trained_no_online_fep.controller_compute_p95_us,
                "fep_update_p95_us": trained_with_fep.fep_update_p95_us
            }
        }));
    }

    println!("{}", json!({
        "benchmark_id": "symthaea-symtropy-flight-transfer-v0.1",
        "claim_ceiling": "simulator_transfer_results_do_not_establish_physical_performance_or_safety",
        "training": {
            "backend": "Symthaea SimplePhysicsSimulator",
            "episodes": training_metrics.len(),
            "steps_per_episode": train_steps,
            "policy_genesis": config.genesis_phrase
        },
        "evaluation": {
            "backend": "Symtropy PhysicsWorld",
            "trials": trials.max(1),
            "steps_per_trial": eval_steps,
            "matched_seeds": true,
            "matched_initial_conditions_and_disturbances": true,
            "fep_adaptation": true,
            "force_disturbances_enabled": force_enabled
        },
        "metrics_note": "Position errors are sampled before each physics step. thrust_impulse_n_s is thrust impulse, not electrical energy. ground_contact_steps counts explicit ground clamps. Controller p50/p95 and FEP p95 latencies are wall-clock microseconds measured in-process, are machine/load dependent, and are not cross-machine latency comparisons. No win is assumed.",
        "trials": paired_rows
    }));
}
