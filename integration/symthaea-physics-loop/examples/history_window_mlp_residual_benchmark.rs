//! Held-out benchmark for explicit finite-history raw-state residual prediction.
//!
//! The model receives a current state/action/dt row and four prior rows, with one mask per
//! historical lag. It uses the same train-only transition dataset, nominal target, test
//! episodes, deterministic seeds, PID action policy, and disturbance schedule as ridge and
//! HDC/LTC. It is a predictor-only baseline and does not actuate the learned residual.

use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use serde_json::{json, Value};
use symthaea_multirotor::{
    pid_baseline, FlightState, PdGains, PidState, PhysicsSimulator, QuadrotorCommand,
};
use symtropy_real_symthaea_physics_loop::{
    history_window_mlp::{
        history_features, HistoryTrainingSample, HistoryWindowMlpResidual, HISTORY_DEPTH,
        HISTORY_INPUT_DIM, HISTORY_LAGS,
    },
    scenario_matrix::{scenarios_for_split, FlightScenario, ScenarioSplit},
    symtropy_flight::SymtropyFlightSimulator,
    transition_residual::{LinearTransitionResidual, NominalLinearDynamics},
};

const MASS_KG: f64 = 0.027;
const GRAVITY_M_S2: f64 = 9.81;
const LINEAR_DRAG_N_S_PER_M: f64 = 0.01;
const DT_S: f64 = 1.0 / 500.0;
const TARGET_SCALE_M_S2: f64 = 10.0;

#[derive(Debug, Default)]
struct ErrorAccumulator {
    samples: usize,
    error_squared_sum: f64,
    zero_squared_sum: f64,
    max_vector_error_m_s2: f64,
    invalid_samples: usize,
    inference_us: Vec<f64>,
}

impl ErrorAccumulator {
    fn metrics(&self) -> Value {
        let denominator = (self.samples * 3).max(1) as f64;
        let rmse = (self.error_squared_sum / denominator).sqrt();
        let zero_rmse = (self.zero_squared_sum / denominator).sqrt();
        json!({
            "sample_count": self.samples,
            "history_window_mlp_residual_rmse_m_s2": rmse,
            "zero_residual_physics_only_rmse_m_s2": zero_rmse,
            "relative_rmse_improvement_vs_zero": if zero_rmse > 0.0 {
                Some(1.0 - rmse / zero_rmse)
            } else {
                None
            },
            "max_vector_prediction_error_m_s2": self.max_vector_error_m_s2,
            "inference_p50_us": percentile(&self.inference_us, 0.50),
            "inference_p95_us": percentile(&self.inference_us, 0.95),
            "inference_p99_us": percentile(&self.inference_us, 0.99),
            "invalid_samples": self.invalid_samples
        })
    }

    fn rmse(&self) -> f64 {
        (self.error_squared_sum / (self.samples * 3).max(1) as f64).sqrt()
    }

    fn zero_rmse(&self) -> f64 {
        (self.zero_squared_sum / (self.samples * 3).max(1) as f64).sqrt()
    }

    fn merge(&mut self, other: &Self) {
        self.samples += other.samples;
        self.error_squared_sum += other.error_squared_sum;
        self.zero_squared_sum += other.zero_squared_sum;
        self.max_vector_error_m_s2 = self
            .max_vector_error_m_s2
            .max(other.max_vector_error_m_s2);
        self.invalid_samples += other.invalid_samples;
        self.inference_us.extend(other.inference_us.iter().copied());
    }
}

fn percentile(values: &[f64], p: f64) -> Option<f64> {
    if values.is_empty() || !p.is_finite() || !(0.0..=1.0).contains(&p) {
        return None;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((sorted.len() - 1) as f64 * p).ceil() as usize;
    sorted.get(index).copied()
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
        .max(1)
}

fn number(value: &Value, label: &str) -> f64 {
    value
        .as_f64()
        .unwrap_or_else(|| panic!("invalid numeric dataset field: {label}"))
}

fn array<const N: usize>(value: &Value, label: &str) -> [f64; N] {
    let values = value
        .as_array()
        .unwrap_or_else(|| panic!("expected array dataset field: {label}"));
    assert_eq!(values.len(), N, "wrong dataset vector length: {label}");
    std::array::from_fn(|index| number(&values[index], label))
}

fn parse_state(value: &Value) -> FlightState {
    FlightState {
        timestamp: number(&value["timestamp_s"], "timestamp_s"),
        position: array(&value["position_world_m"], "position_world_m"),
        quaternion: array(&value["quaternion_wxyz"], "quaternion_wxyz"),
        linear_velocity: array(
            &value["linear_velocity_world_m_s"],
            "linear_velocity_world_m_s",
        ),
        angular_velocity: array(
            &value["angular_velocity_body_rad_s"],
            "angular_velocity_body_rad_s",
        ),
    }
}

fn parse_action(value: &Value) -> QuadrotorCommand {
    let raw = array::<4>(value, "applied_action_ctrl");
    QuadrotorCommand::from_raw(raw.map(|component| component as f32)).clamped()
}

fn training_samples(path: &str) -> Vec<HistoryTrainingSample> {
    let bytes = fs::read(path)
        .unwrap_or_else(|error| panic!("cannot read training dataset at {path}: {error}"));
    let root: Value = serde_json::from_slice(&bytes).expect("training dataset must be JSON");
    assert_eq!(
        root["benchmark_id"].as_str(),
        Some("symtropy-causal-transition-residual-dataset-v0.1"),
        "unexpected or unsupported training dataset"
    );
    assert_eq!(
        root["data_schema_version"].as_str(),
        Some("causal-transition-residual-dataset-v0.1"),
        "unsupported training dataset schema"
    );
    assert_eq!(
        root["residual_target_algorithm_version"].as_str(),
        Some("observed-minus-nominal-world-acceleration-v0.1"),
        "unsupported residual-target algorithm"
    );
    assert_eq!(
        root["claim_ceiling"].as_str(),
        Some("training_split_transition_dataset_and_target_plumbing_only"),
        "unexpected dataset claim ceiling"
    );
    assert_eq!(
        root["collection"]["invalid_target_count"].as_u64(),
        Some(0),
        "refusing dataset with invalid targets"
    );

    let rows = root["samples"]
        .as_array()
        .expect("training dataset samples must be an array");
    let mut samples = Vec::with_capacity(rows.len());
    let mut previous_features: Vec<[f64; 18]> = Vec::with_capacity(HISTORY_LAGS);
    let mut current_episode = None;
    let mut previous_transition = None;

    for (row_index, row) in rows.iter().enumerate() {
        assert_eq!(row["split"].as_str(), Some("train"), "training-only split required");
        assert_eq!(
            row["data_schema_version"].as_str(),
            Some("causal-transition-residual-row-v0.1"),
            "unsupported training-row schema"
        );
        let episode = row["episode_index"]
            .as_u64()
            .unwrap_or_else(|| panic!("missing episode index in row {row_index}"));
        let transition = row["transition_index"]
            .as_u64()
            .unwrap_or_else(|| panic!("missing transition index in row {row_index}"));

        if current_episode != Some(episode) {
            if let Some(previous) = current_episode {
                assert!(episode > previous, "training episodes must be strictly ordered");
            }
            assert_eq!(transition, 0, "each episode must start at transition zero");
            previous_features.clear();
            current_episode = Some(episode);
            previous_transition = None;
        } else if let Some(previous) = previous_transition {
            assert_eq!(
                transition,
                previous + 1,
                "training episode contains a gap or reordered transition"
            );
        }
        previous_transition = Some(transition);

        let state = parse_state(&row["state_t"]);
        let action = parse_action(&row["applied_action_ctrl"]);
        let dt_s = number(&row["dt_s"], "dt_s");
        let target = array::<3>(
            &row["target"]["residual_acceleration_world_m_s2"],
            "residual_acceleration_world_m_s2",
        );
        assert!(
            dt_s.is_finite() && dt_s > 0.0 && target.iter().all(|value| value.is_finite()),
            "invalid sample in training dataset row {row_index}"
        );

        let features = history_features(&state, &action, dt_s, &previous_features)
            .unwrap_or_else(|| panic!("invalid history features in training row {row_index}"));
        let current = std::array::from_fn(|index| features[index]);
        samples.push(HistoryTrainingSample {
            features,
            target_residual_acceleration_world_m_s2: target,
        });
        previous_features.insert(0, current);
        previous_features.truncate(HISTORY_LAGS);
    }

    assert!(!samples.is_empty(), "refusing to fit on empty training data");
    samples
}

fn evaluate_episode(
    model: &HistoryWindowMlpResidual,
    scenario: &FlightScenario,
    seed: u64,
    steps: usize,
    nominal: NominalLinearDynamics,
    gains: &PdGains,
) -> ErrorAccumulator {
    let mut simulator = SymtropyFlightSimulator::new();
    simulator.reset_with_perturbation(scenario.setpoint.position[2], 0.06, seed);
    let mut pid_state = PidState::default();
    let mut previous_features: Vec<[f64; 18]> = Vec::with_capacity(HISTORY_LAGS);
    let mut metrics = ErrorAccumulator::default();

    for step in 0..steps {
        let before = simulator.state().clone();
        let action = pid_baseline(&before, &scenario.setpoint, gains, &mut pid_state, DT_S)
            .clamped();

        let started = Instant::now();
        let features = history_features(&before, &action, DT_S, &previous_features);
        let prediction = features
            .as_ref()
            .and_then(|values| model.predict_features(values).ok());
        metrics.inference_us.push(started.elapsed().as_secs_f64() * 1.0e6);
        let current_features = features.map(|values| std::array::from_fn(|i| values[i]));

        // Disturbance acts on the held-out plant after prediction and is never a model input.
        let force = scenario.force_at(seed, step, MASS_KG);
        simulator.apply_external_force(force);
        simulator.step(&action, DT_S);
        let after = simulator.state().clone();
        let target =
            LinearTransitionResidual::from_transition(&before, &action, &after, nominal);

        match (prediction, target) {
            (Some(prediction), Ok(target)) => {
                let mut squared_error_norm = 0.0;
                for axis in 0..3 {
                    let error =
                        target.residual_acceleration_world_m_s2[axis] - prediction[axis];
                    squared_error_norm += error * error;
                    metrics.error_squared_sum += error * error;
                    metrics.zero_squared_sum +=
                        target.residual_acceleration_world_m_s2[axis].powi(2);
                }
                metrics.max_vector_error_m_s2 = metrics
                    .max_vector_error_m_s2
                    .max(squared_error_norm.sqrt());
                metrics.samples += 1;
            }
            _ => metrics.invalid_samples = metrics.invalid_samples.saturating_add(1),
        }

        let current = current_features
            .unwrap_or_else(|| panic!("valid simulator state produced invalid history features"));
        previous_features.insert(0, current);
        previous_features.truncate(HISTORY_LAGS);
    }
    metrics
}

fn main() {
    let input_path = std::env::var("STFR_DATASET_INPUT_PATH")
        .expect("STFR_DATASET_INPUT_PATH must identify the training-split dataset");
    let samples = training_samples(&input_path);
    let training_started = Instant::now();
    let model = HistoryWindowMlpResidual::fit(&samples)
        .expect("history-window MLP training must succeed");
    let training_seconds = training_started.elapsed().as_secs_f64();

    let episodes = env_usize("STFR_HISTORY_EVAL_EPISODES", 18);
    let steps_per_episode = env_usize("STFR_HISTORY_EVAL_STEPS", 64);
    let scenarios = scenarios_for_split(ScenarioSplit::Test);
    assert!(!scenarios.is_empty(), "held-out test scenario catalogue must not be empty");
    let nominal = NominalLinearDynamics {
        mass_kg: MASS_KG,
        gravity_m_s2: GRAVITY_M_S2,
        linear_drag_n_s_per_m: LINEAR_DRAG_N_S_PER_M,
    };
    let gains = PdGains::default();
    let mut overall = ErrorAccumulator::default();
    let mut by_family: BTreeMap<String, ErrorAccumulator> = BTreeMap::new();
    let mut episode_details = Vec::with_capacity(episodes);

    for episode in 0..episodes {
        let seed = 0x5359_5445_5354_0000_u64.wrapping_add(episode as u64 + 1);
        let scenario = &scenarios[episode % scenarios.len()];
        let metrics = evaluate_episode(
            &model, scenario, seed, steps_per_episode, nominal, &gains,
        );
        overall.merge(&metrics);
        by_family
            .entry(scenario.family.to_owned())
            .or_default()
            .merge(&metrics);
        episode_details.push(json!({
            "episode_index": episode,
            "episode_seed_hex": format!("{seed:016x}"),
            "scenario_id": scenario.id,
            "scenario_family": scenario.family,
            "sample_count": metrics.samples,
            "rmse_m_s2": metrics.rmse(),
            "zero_residual_rmse_m_s2": metrics.zero_rmse(),
            "max_vector_prediction_error_m_s2": metrics.max_vector_error_m_s2,
            "invalid_samples": metrics.invalid_samples,
            "inference_p95_us": percentile(&metrics.inference_us, 0.95)
        }));
    }

    let family_metrics = by_family
        .into_iter()
        .map(|(family, metrics)| (family, metrics.metrics()))
        .collect::<serde_json::Map<_, _>>();

    println!("{}", json!({
        "benchmark_id": "symtropy-explicit-history-mlp-residual-heldout-v0.1",
        "claim_ceiling": "explicit_history_mlp_one_step_heldout_prediction_only_no_control_or_safety_claim",
        "source_receipt": {
            "symtropy_git_sha": std::env::var("STFR_SYMTROPY_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "symthaea_git_sha": std::env::var("STFR_SYMTHAEA_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "rustc_version": std::env::var("STFR_RUSTC_VERSION").unwrap_or_else(|_| "unset".to_owned()),
            "scenario_matrix_sha256": std::env::var("STFR_SCENARIO_MATRIX_SHA256").unwrap_or_else(|_| "unset".to_owned()),
            "training_dataset_sha256": std::env::var("STFR_TRAIN_DATASET_SHA256").unwrap_or_else(|_| "unset".to_owned())
        },
        "representation": {
            "input_features_per_transition": 18,
            "history_depth_including_current": HISTORY_DEPTH,
            "historical_lags": HISTORY_LAGS,
            "input_feature_dimension": HISTORY_INPUT_DIM,
            "historical_lag_mask_count": HISTORY_LAGS,
            "hidden_units": model.hidden_units(),
            "output_features": 3,
            "activation": "tanh",
            "parameter_count": model.parameter_count(),
            "initialization": "fixed_hash_seed",
            "initialization_seed_hex": format!("{:016x}", symtropy_real_symthaea_physics_loop::history_window_mlp::INITIALIZATION_SEED),
            "optimizer": "full_batch_adam",
            "training_epochs": model.training_epochs(),
            "learning_rate": model.learning_rate(),
            "target_scale_m_s2": TARGET_SCALE_M_S2,
            "weight_decay_l2": 1.0e-5,
            "gradient_norm_clip": 5.0,
            "features_shared_with": "raw_state_ridge_instantaneous_transform_per_lag",
            "history_boundary": "reset at each episode; no cross-episode state",
            "temporal_representation": "explicit four-step causal delay embedding; no recurrent hidden state"
        },
        "training": {
            "split": "train",
            "sample_count": model.training_sample_count(),
            "training_seconds": training_seconds,
            "final_training_rmse_m_s2": model.final_training_rmse_m_s2(),
            "hidden_force_labels_used_as_features": false
        },
        "evaluation": {
            "split": "test",
            "episodes_requested": episodes,
            "steps_per_episode": steps_per_episode,
            "sample_count": overall.samples,
            "invalid_samples": overall.invalid_samples,
            "metrics": overall.metrics(),
            "family_metrics": family_metrics,
            "episode_details": episode_details,
            "test_scenario_ids": scenarios.iter().map(|scenario| scenario.id).collect::<Vec<_>>(),
            "test_episode_seed_hexes": (0..episodes)
                .map(|episode| format!("{:016x}", 0x5359_5445_5354_0000_u64.wrapping_add(episode as u64 + 1)))
                .collect::<Vec<_>>(),
            "test_episode_scenario_ids": (0..episodes)
                .map(|episode| scenarios[episode % scenarios.len()].id)
                .collect::<Vec<_>>(),
            "nominal_model": {
                "mass_kg": nominal.mass_kg,
                "gravity_m_s2": nominal.gravity_m_s2,
                "linear_drag_n_s_per_m": nominal.linear_drag_n_s_per_m
            }
        },
        "comparison_contract": "Same training dataset, held-out scenario families, exact seed order, PID policy, nominal dynamics and evaluation horizon as ridge/HDC-LTC/current-state MLP. Prediction-only results; compare explicit raw history against learned HDC/LTC memory before attributing gain to HDC representation."
    }));
}
