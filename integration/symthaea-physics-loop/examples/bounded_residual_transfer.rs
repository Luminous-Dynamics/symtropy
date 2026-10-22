//! Matched-seed benchmark for a bounded Symthaea residual over PID.
//!
//! This is a simulator research experiment, not a safety certificate or flight-ready controller.
//! It varies residual authority while keeping trained weights, PID gains, initial seed,
//! and disturbance schedule paired across arms.

use serde::Serialize;
use serde_json::json;
use std::time::Instant;
use symthaea_core::genesis::GenesisSeed;
use symthaea_multirotor::{
    pid_baseline, FlightConfig, FlightController, FlightTrainer, PdGains, PidState,
    PhysicsSimulator, QuadrotorHdcEncoder,
};
use symtropy_real_symthaea_physics_loop::{
    bounded_residual::ResidualEnvelope,
    scenario_matrix::{scenarios_for_split, training_setpoints, FlightScenario, ScenarioSplit},
    symtropy_flight::SymtropyFlightSimulator,
};

const AUTHORITIES: [f32; 6] = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0];
// Deliberately explicit study parameters, not universal stability/safety limits.
// Channel order: thrust N, roll N·m, pitch N·m, yaw N·m.
const MAX_RESIDUAL: [f32; 4] = [0.04, 0.001, 0.001, 0.0004];

#[derive(Debug, Clone, Copy, Serialize)]
struct Metrics {
    authority: f32,
    rmse_m: f64,
    mean_position_error_m: f64,
    final_position_error_m: f64,
    max_position_error_m: f64,
    within_5cm_fraction: f64,
    thrust_impulse_n_s: f64,
    residual_clipped_steps: u64,
    guard_rejections: u64,
    controller_compute_p95_us: f64,
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

fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

/// Deterministic percentile bootstrap over paired seed-level deltas.
fn bootstrap_mean_ci(values: &[f64], resamples: usize, seed: u64) -> Option<[f64; 2]> {
    if values.len() < 2 || resamples == 0 {
        return None;
    }
    let mut rng = if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed };
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut total = 0.0;
        for _ in 0..values.len() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            total += values[(rng % values.len() as u64) as usize];
        }
        means.push(total / values.len() as f64);
    }
    Some([percentile(&means, 0.025), percentile(&means, 0.975)])
}

fn evaluate(
    seed: u64,
    authority: f32,
    steps: usize,
    scenario: &FlightScenario,
    config: &FlightConfig,
    trained: &FlightController,
    gains: &PdGains,
) -> Metrics {
    let mut sim = SymtropyFlightSimulator::new();
    sim.reset_with_perturbation(scenario.setpoint.position[2], 0.08, seed);
    let genesis = GenesisSeed::from_phrase(&config.genesis_phrase);
    let mut encoder = QuadrotorHdcEncoder::new(&genesis, config.num_levels);
    let mut controller = trained.clone();
    controller.reset();
    let mut pid_state = PidState::default();
    let envelope = ResidualEnvelope {
        authority,
        max_abs_delta: MAX_RESIDUAL,
    };

    let dt = 1.0 / 500.0;
    let (mut squared, mut absolute, mut impulse) = (0.0, 0.0, 0.0);
    let mut max_error: f64 = 0.0;
    let mut within = 0usize;
    let mut clipped_steps = 0u64;
    let mut guard_rejections = 0u64;
    let mut latencies_us = Vec::with_capacity(steps);

    for step in 0..steps {
        let state = sim.state().clone();
        let error = scenario.setpoint.position_error_magnitude(&state);
        squared += error * error;
        absolute += error;
        max_error = max_error.max(error);
        if error < 0.05 {
            within += 1;
        }

        let started = Instant::now();
        let nominal = pid_baseline(&state, &scenario.setpoint, gains, &mut pid_state, dt).clamped();
        let command = if authority == 0.0 {
            // The zero-authority arm is precisely the PID baseline, without hidden HDC compute.
            nominal
        } else {
            let observation = encoder.encode(&state);
            let cognitive = controller.forward(&observation, dt as f32);
            let n = nominal.to_ctrl();
            let c = cognitive.to_ctrl();
            if (0..4).any(|i| (c[i] - n[i]).abs() > MAX_RESIDUAL[i] as f64) {
                clipped_steps += 1;
            }
            match envelope.apply(nominal, cognitive) {
                Some(command) => command,
                None => {
                    guard_rejections += 1;
                    nominal
                }
            }
        };
        latencies_us.push(started.elapsed().as_secs_f64() * 1.0e6);
        impulse += command.thrust as f64 * dt;

        let force = scenario.force_at(seed, step, 0.027);
        if force.iter().any(|component| *component != 0.0) {
            sim.apply_external_force(force);
        }
        sim.step(&command, dt);
        if authority > 0.0 && step > 0 && step % 100 == 0 {
            controller.normalize_states();
        }
    }

    Metrics {
        authority,
        rmse_m: (squared / steps.max(1) as f64).sqrt(),
        mean_position_error_m: absolute / steps.max(1) as f64,
        final_position_error_m: scenario.setpoint.position_error_magnitude(sim.state()),
        max_position_error_m: max_error,
        within_5cm_fraction: within as f64 / steps.max(1) as f64,
        thrust_impulse_n_s: impulse,
        residual_clipped_steps: clipped_steps,
        guard_rejections,
        controller_compute_p95_us: percentile(&latencies_us, 0.95),
    }
}

fn main() {
    let train_episodes = std::env::var("STFR_TRAIN_EPISODES")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(4usize);
    let train_steps = std::env::var("STFR_TRAIN_STEPS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(256usize);
    let eval_steps = std::env::var("STFR_EVAL_STEPS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(320usize);
    let trials = std::env::var("STFR_TRIALS")
        .ok().and_then(|v| v.parse().ok()).unwrap_or(30usize);

    let config = FlightConfig {
        num_episodes: train_episodes.max(1),
        steps_per_episode: train_steps.max(1),
        train_every: 4,
        replay_buffer_size: 16,
        replay_count: 1,
        enable_lr_schedule: true,
        early_termination: true,
        genesis_phrase: "symthaea-symtropy-bounded-residual-v0.1".to_owned(),
        curriculum: training_setpoints(),
        use_pid: false,
        ..FlightConfig::default()
    };
    let mut trainer = FlightTrainer::new(config.clone());
    let (trained, training_metrics) = trainer.train_returning_controller();
    let gains = PdGains::default();

    let mut evaluation_scenarios = scenarios_for_split(ScenarioSplit::Validation);
    evaluation_scenarios.extend(scenarios_for_split(ScenarioSplit::Test));
    let validation_ids = scenarios_for_split(ScenarioSplit::Validation)
        .iter()
        .map(|scenario| scenario.id)
        .collect::<Vec<_>>();
    let test_ids = scenarios_for_split(ScenarioSplit::Test)
        .iter()
        .map(|scenario| scenario.id)
        .collect::<Vec<_>>();
    let training_ids = scenarios_for_split(ScenarioSplit::Train)
        .iter()
        .map(|scenario| scenario.id)
        .collect::<Vec<_>>();

    let mut trial_rows = Vec::with_capacity(evaluation_scenarios.len() * trials.max(1));
    let mut scenario_summaries = Vec::with_capacity(evaluation_scenarios.len());

    for scenario in &evaluation_scenarios {
        let mut rmse_deltas: Vec<Vec<f64>> =
            vec![Vec::with_capacity(trials.max(1)); AUTHORITIES.len()];
        let mut within_deltas: Vec<Vec<f64>> =
            vec![Vec::with_capacity(trials.max(1)); AUTHORITIES.len()];

        for index in 0..trials.max(1) {
            let seed = 0x5359_4D52_0000_0000_u64.wrapping_add(index as u64 + 1);
            let arms: Vec<Metrics> = AUTHORITIES
                .iter()
                .map(|authority| evaluate(seed, *authority, eval_steps, scenario, &config, &trained, &gains))
                .collect();
            let baseline = arms[0];
            for (i, arm) in arms.iter().enumerate() {
                rmse_deltas[i].push(arm.rmse_m - baseline.rmse_m);
                within_deltas[i].push(arm.within_5cm_fraction - baseline.within_5cm_fraction);
            }
            trial_rows.push(json!({
                "scenario_id": scenario.id,
                "scenario_family": scenario.family,
                "scenario_split": scenario.split,
                "trial_seed": seed,
                "evidence_class": "simulator_experiment",
                "matched_seed_across_authorities": true,
                "arms": arms
            }));
        }

        let arm_summaries: Vec<serde_json::Value> = AUTHORITIES
            .iter()
            .enumerate()
            .map(|(i, authority)| {
                json!({
                    "authority": authority,
                    "paired_trial_count": rmse_deltas[i].len(),
                    "mean_delta_rmse_vs_pid_m": mean(&rmse_deltas[i]),
                    "bootstrap_95_ci_delta_rmse_vs_pid_m": bootstrap_mean_ci(
                        &rmse_deltas[i], 2000, 0x5354_5200_u64.wrapping_add(i as u64)
                    ),
                    "mean_delta_within_5cm_fraction_vs_pid": mean(&within_deltas[i]),
                    "bootstrap_95_ci_delta_within_5cm_fraction_vs_pid": bootstrap_mean_ci(
                        &within_deltas[i], 2000, 0x5354_5210_u64.wrapping_add(i as u64)
                    ),
                    "all_trials_guard_rejections_zero": trial_rows.iter()
                        .filter(|row| row["scenario_id"] == scenario.id)
                        .all(|row| row["arms"][i]["guard_rejections"] == 0)
                })
            })
            .collect();

        scenario_summaries.push(json!({
            "scenario_id": scenario.id,
            "scenario_family": scenario.family,
            "scenario_split": scenario.split,
            "setpoint_m": scenario.setpoint.position,
            "disturbance_profile": scenario.disturbance,
            "disturbance_fraction_of_weight": scenario.disturbance_fraction_of_weight,
            "paired_summary_vs_pid": arm_summaries
        }));
    }

    println!("{}", json!({
        "benchmark_id": "symthaea-symtropy-bounded-residual-matrix-v0.2",
        "claim_ceiling": "heldout_simulator_generalization_study_not_a_stability_certificate_or_physical_safety_claim",
        "training": {
            "backend": "Symthaea SimplePhysicsSimulator",
            "episodes_reported_by_trainer": training_metrics.len(),
            "steps_per_episode_configured": train_steps,
            "training_scenario_ids": training_ids,
            "training_target_policy": "FlightTrainer PD command imitation (FlightConfig.use_pid=false)",
            "policy_genesis": config.genesis_phrase
        },
        "evaluation": {
            "backend": "Symtropy PhysicsWorld",
            "trials_per_scenario": trials.max(1),
            "scenario_count": evaluation_scenarios.len(),
            "scenario_trial_pair_count": trial_rows.len(),
            "steps_per_trial": eval_steps,
            "validation_scenario_ids": validation_ids,
            "heldout_test_scenario_ids": test_ids,
            "matched_seed_across_authority_arms": true,
            "classical_nominal_controller": "Symthaea pid_baseline with PdGains::default",
            "online_fep": false,
            "disturbance_force_scale": "fraction_of_vehicle_weight",
            "authorities": AUTHORITIES,
            "max_abs_residual": MAX_RESIDUAL,
            "residual_limits_are_airframe_safety_limits": false
        },
        "metrics_note": "Positive RMSE deltas mean worse tracking; positive within-5cm deltas mean better tracking. Thrust impulse is not electrical energy. Controller p95 is in-process wall time for nominal controller, cognitive forward when enabled, and residual guard; it is hardware/load dependent and excludes full sensor-to-actuator latency. Guard rejection falls back to nominal PID and is counted. Residual clipping counts steps where any requested cognitive-minus-PID channel exceeds its configured per-channel cap. Forces are normalized by vehicle weight. Bootstrap intervals from the two-trial CI smoke are descriptive only and are reported per scenario.",
        "paired_summary_by_scenario": scenario_summaries,
        "trials": trial_rows
    }));
}
