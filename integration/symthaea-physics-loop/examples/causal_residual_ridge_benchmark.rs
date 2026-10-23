//! Held-out one-step residual-prediction smoke for the transparent ridge baseline.
//!
//! Reads the training-split artifact from causal_transition_dataset.rs and evaluates
//! on the predeclared test split. The test generator sees disturbance forces solely to
//! evolve the simulator; the predictor receives only state-at-t, applied action and dt.

use std::collections::BTreeMap;
use std::fs;

use serde_json::{json, Value};
use symthaea_multirotor::{
    pid_baseline, FlightState, PdGains, PidState, PhysicsSimulator, QuadrotorCommand,
};
use symtropy_real_symthaea_physics_loop::{
    linear_residual_ridge::{LinearResidualRidge, ResidualTrainingSample},
    scenario_matrix::{scenarios_for_split, ScenarioSplit},
    symtropy_flight::SymtropyFlightSimulator,
    transition_residual::{NominalLinearDynamics, ResidualTargetError},
};

const MASS_KG: f64 = 0.027;
const GRAVITY_M_S2: f64 = 9.81;
const LINEAR_DRAG_N_S_PER_M: f64 = 0.01;
const DT_S: f64 = 1.0 / 500.0;

#[derive(Debug, Default)]
struct ErrorAccumulator {
    samples: usize,
    ridge_squared_error_sum: f64,
    zero_squared_error_sum: f64,
    max_target_norm: f64,
    invalid_samples: usize,
}

impl ErrorAccumulator {
    fn add(&mut self, target: [f64; 3], prediction: [f64; 3]) {
        self.samples += 1;
        for axis in 0..3 {
            self.ridge_squared_error_sum += (target[axis] - prediction[axis]).powi(2);
            self.zero_squared_error_sum += target[axis].powi(2);
        }
        self.max_target_norm = self.max_target_norm.max(
            target.iter().map(|value| value * value).sum::<f64>().sqrt(),
        );
    }

    fn metrics(&self) -> Value {
        let denominator = (self.samples * 3).max(1) as f64;
        let ridge_rmse = (self.ridge_squared_error_sum / denominator).sqrt();
        let zero_rmse = (self.zero_squared_error_sum / denominator).sqrt();
        json!({
            "sample_count": self.samples,
            "ridge_residual_rmse_m_s2": ridge_rmse,
            "zero_residual_physics_only_rmse_m_s2": zero_rmse,
            "relative_rmse_improvement_vs_zero": if zero_rmse > 0.0 {
                Some(1.0 - ridge_rmse / zero_rmse)
            } else {
                None
            },
            "max_target_residual_norm_m_s2": self.max_target_norm,
            "invalid_samples": self.invalid_samples
        })
    }
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

fn parse_training_samples(path: &str) -> Vec<ResidualTrainingSample> {
    let bytes = fs::read(path).unwrap_or_else(|error| {
        panic!("cannot read training dataset at {path}: {error}");
    });
    let root: Value = serde_json::from_slice(&bytes).expect("training dataset must be JSON");
    assert_eq!(
        root["benchmark_id"].as_str(),
        Some("symtropy-causal-transition-residual-dataset-v0.1"),
        "unexpected or unsupported training dataset schema"
    );
    assert_eq!(
        root["claim_ceiling"].as_str(),
        Some("training_split_transition_dataset_and_target_plumbing_only"),
        "dataset claim ceiling must remain explicit"
    );
    assert_eq!(
        root["collection"]["invalid_target_count"].as_u64(),
        Some(0),
        "refusing a dataset with invalid target records"
    );

    let rows = root["samples"]
        .as_array()
        .expect("dataset samples must be an array");
    let mut samples = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        assert_eq!(row["split"].as_str(), Some("train"), "train-only dataset required");
        let state_t = parse_state(&row["state_t"]);
        let action = parse_action(&row["applied_action_ctrl"]);
        let dt_s = number(&row["dt_s"], "dt_s");
        let target = array::<3>(
            &row["target"]["residual_acceleration_world_m_s2"],
            "residual_acceleration_world_m_s2",
        );
        if !dt_s.is_finite() || dt_s <= 0.0 || !target.iter().all(|value| value.is_finite()) {
            panic!("invalid training row at index {index}");
        }
        samples.push(ResidualTrainingSample {
            state_t,
            applied_action: action,
            dt_s,
            target_residual_acceleration_world_m_s2: target,
        });
    }
    assert!(!samples.is_empty(), "refusing to fit ridge baseline on empty data");
    samples
}

fn main() {
    let input_path = std::env::var("STFR_DATASET_INPUT_PATH")
        .expect("STFR_DATASET_INPUT_PATH must point to a verified training-split JSON artifact");
    let train_samples = parse_training_samples(&input_path);
    let ridge_lambda = std::env::var("STFR_RIDGE_LAMBDA")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(1.0e-3);
    let model =
        LinearResidualRidge::fit(&train_samples, ridge_lambda).expect("ridge fit must be valid");

    let episodes = env_usize("STFR_RIDGE_EVAL_EPISODES", 6);
    let steps_per_episode = env_usize("STFR_RIDGE_EVAL_STEPS", 128);
    let test_scenarios = scenarios_for_split(ScenarioSplit::Test);
    assert!(!test_scenarios.is_empty(), "test scenario catalogue must not be empty");
    let nominal = NominalLinearDynamics {
        mass_kg: MASS_KG,
        gravity_m_s2: GRAVITY_M_S2,
        linear_drag_n_s_per_m: LINEAR_DRAG_N_S_PER_M,
    };
    let gains = PdGains::default();
    let mut simulator = SymtropyFlightSimulator::new();
    let mut overall = ErrorAccumulator::default();
    let mut by_family: BTreeMap<String, ErrorAccumulator> = BTreeMap::new();
    let mut ground_contact_steps = 0_u64;
    let mut invalid_inputs = 0_u64;

    for episode in 0..episodes {
        let seed = 0x5359_5249_4447_0000_u64.wrapping_add(episode as u64 + 1);
        let scenario = &test_scenarios[episode % test_scenarios.len()];
        simulator.reset_with_perturbation(scenario.setpoint.position[2], 0.06, seed);
        let mut pid_state = PidState::default();
        let family_accumulator = by_family
            .entry(scenario.family.to_owned())
            .or_default();

        for _step in 0..steps_per_episode {
            let before = simulator.state().clone();
            let action = pid_baseline(&before, &scenario.setpoint, &gains, &mut pid_state, DT_S)
                .clamped();
            // Hidden disturbance is used only to evolve the held-out simulator.
            // Neither this force nor scenario labels enter model.predict().
            let force = scenario.force_at(seed, _step, MASS_KG);
            simulator.apply_external_force(force);
            simulator.step(&action, DT_S);
            let after = simulator.state().clone();

            match ResidualTrainingSample::from_transition(&before, &action, &after, nominal) {
                Ok(sample) => match model.predict(&sample.state_t, &sample.applied_action, sample.dt_s) {
                    Ok(prediction) => {
                        overall.add(sample.target_residual_acceleration_world_m_s2, prediction);
                        family_accumulator
                            .add(sample.target_residual_acceleration_world_m_s2, prediction);
                    }
                    Err(_) => {
                        invalid_inputs = invalid_inputs.saturating_add(1);
                        overall.invalid_samples += 1;
                        family_accumulator.invalid_samples += 1;
                    }
                },
                Err(ResidualTargetError::NonFiniteState)
                | Err(ResidualTargetError::InvalidQuaternion)
                | Err(ResidualTargetError::NonIncreasingTimestamp)
                | Err(ResidualTargetError::NonFiniteDerivedTarget)
                | Err(ResidualTargetError::InvalidNominalParameters)
                | Err(ResidualTargetError::NonFiniteCommand) => {
                    invalid_inputs = invalid_inputs.saturating_add(1);
                    overall.invalid_samples += 1;
                    family_accumulator.invalid_samples += 1;
                }
            }
        }
        ground_contact_steps += simulator.ground_contact_count();
    }

    let per_family = by_family
        .iter()
        .map(|(family, metrics)| (family.clone(), metrics.metrics()))
        .collect::<serde_json::Map<_, _>>();
    println!("{}", json!({
        "benchmark_id": "symtropy-causal-transition-residual-ridge-heldout-v0.1",
        "claim_ceiling": "one_step_heldout_simulator_prediction_comparison_not_control_or_safety_evidence",
        "source_receipt": {
            "symtropy_git_sha": std::env::var("STFR_SYMTROPY_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "symthaea_git_sha": std::env::var("STFR_SYMTHAEA_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "rustc_version": std::env::var("STFR_RUSTC_VERSION").unwrap_or_else(|_| "unset".to_owned()),
            "scenario_matrix_sha256": std::env::var("STFR_SCENARIO_MATRIX_SHA256").unwrap_or_else(|_| "unset".to_owned()),
            "training_dataset_sha256": std::env::var("STFR_TRAIN_DATASET_SHA256").unwrap_or_else(|_| "unset".to_owned())
        },
        "training": {
            "sample_count": train_samples.len(),
            "fit_method": "fixed_scale_ridge_normal_equations_partial_pivoting",
            "ridge_lambda": model.ridge_lambda(),
            "feature_boundary": "state_t, applied_action, dt_s only; residual target is label",
            "hidden_force_labels_used_as_features": false
        },
        "evaluation": {
            "split": "test",
            "episodes": episodes,
            "steps_per_episode": steps_per_episode,
            "test_sample_count": overall.samples,
            "scenario_ids": test_scenarios.iter().map(|scenario| scenario.id).collect::<Vec<_>>(),
            "nominal_model": {
                "mass_kg": nominal.mass_kg,
                "gravity_m_s2": nominal.gravity_m_s2,
                "linear_drag_n_s_per_m": nominal.linear_drag_n_s_per_m
            },
            "metrics": overall.metrics(),
            "per_family": per_family,
            "invalid_input_count": invalid_inputs,
            "ground_contact_steps": ground_contact_steps
        }
    }));
}
