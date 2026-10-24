//! Held-out one-step residual-prediction and episode-block calibration benchmark.
//!
//! Fits ridge on the training artifact, calibrates a vector-error radius using validation
//! episodes (each score is the maximum one-step residual-vector error in that episode),
//! and then reports episode and step coverage on disjoint test scenario families.
//!
//! Calibration frames within an episode are NOT treated as independent conformal samples.
//! Since validation and test scenario families are intentionally shifted, test coverage is
//! a robustness diagnostic and not a formal guarantee under distribution shift.

use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use serde_json::{json, Map, Value};
use symthaea_multirotor::{
    pid_baseline, FlightState, PdGains, PidState, PhysicsSimulator, QuadrotorCommand,
};
use symtropy_real_symthaea_physics_loop::{
    episode_conformal::split_conformal_radius,
    linear_residual_ridge::{LinearResidualRidge, ResidualTrainingSample},
    scenario_matrix::{scenarios_for_split, FlightScenario, ScenarioSplit},
    symtropy_flight::SymtropyFlightSimulator,
    transition_residual::{LinearTransitionResidual, NominalLinearDynamics},
};

const MASS_KG: f64 = 0.027;
const GRAVITY_M_S2: f64 = 9.81;
const LINEAR_DRAG_N_S_PER_M: f64 = 0.01;
const DT_S: f64 = 1.0 / 500.0;
const CONFORMAL_ALPHA: f64 = 0.10;

#[derive(Debug, Default)]
struct ErrorAccumulator {
    samples: usize,
    ridge_squared_error_sum: f64,
    zero_squared_error_sum: f64,
    max_prediction_error_norm: f64,
    invalid_samples: usize,
    ground_contact_steps: u64,
    inference_us: Vec<f64>,
}

impl ErrorAccumulator {
    fn merge(&mut self, other: &Self) {
        self.samples += other.samples;
        self.ridge_squared_error_sum += other.ridge_squared_error_sum;
        self.zero_squared_error_sum += other.zero_squared_error_sum;
        self.max_prediction_error_norm = self
            .max_prediction_error_norm
            .max(other.max_prediction_error_norm);
        self.invalid_samples += other.invalid_samples;
        self.ground_contact_steps += other.ground_contact_steps;
        self.inference_us.extend(other.inference_us.iter().copied());
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
            "max_prediction_error_norm_m_s2": self.max_prediction_error_norm,
            "invalid_samples": self.invalid_samples,
            "ground_contact_steps": self.ground_contact_steps,
            "inference_p50_us": percentile(&self.inference_us, 0.50),
            "inference_p95_us": percentile(&self.inference_us, 0.95),
            "inference_p99_us": percentile(&self.inference_us, 0.99)
        })
    }
}

struct EpisodeEvaluation {
    scenario_id: &'static str,
    scenario_family: &'static str,
    seed: u64,
    sample_count: usize,
    error_squared_sum: f64,
    zero_squared_sum: f64,
    max_error_norm: f64,
    step_error_norms: Vec<f64>,
    invalid_samples: usize,
    ground_contact_steps: u64,
    inference_us: Vec<f64>,
}

impl EpisodeEvaluation {
    fn error_accumulator(&self) -> ErrorAccumulator {
        ErrorAccumulator {
            samples: self.sample_count,
            ridge_squared_error_sum: self.error_squared_sum,
            zero_squared_error_sum: self.zero_squared_sum,
            max_prediction_error_norm: self.max_error_norm,
            invalid_samples: self.invalid_samples,
            ground_contact_steps: self.ground_contact_steps,
            inference_us: self.inference_us.clone(),
        }
    }

    fn calibration_score(&self) -> Option<f64> {
        if self.sample_count == 0 || self.invalid_samples != 0 {
            None
        } else {
            Some(self.max_error_norm)
        }
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
        root["data_schema_version"].as_str(),
        Some("causal-transition-residual-dataset-v0.1"),
        "unsupported training dataset schema version"
    );
    assert_eq!(
        root["residual_target_algorithm_version"].as_str(),
        Some("observed-minus-nominal-world-acceleration-v0.1"),
        "unsupported residual target algorithm version"
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
        assert_eq!(
            row["data_schema_version"].as_str(),
            Some("causal-transition-residual-row-v0.1"),
            "unsupported training row schema"
        );
        assert_eq!(
            row["residual_target_algorithm_version"].as_str(),
            Some("observed-minus-nominal-world-acceleration-v0.1"),
            "unsupported training target algorithm"
        );
        assert_eq!(
            row["nominal_model_id"].as_str(),
            Some("linear-thrust-gravity-linear-drag-v0.1"),
            "unsupported nominal model"
        );
        assert!(
            row["episode_seed"].as_u64().is_some()
                && row["scenario_family"].as_str().is_some(),
            "training row is missing provenance metadata at index {index}"
        );
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

fn evaluate_episode(
    model: &LinearResidualRidge,
    scenario: &FlightScenario,
    seed: u64,
    steps: usize,
    nominal: NominalLinearDynamics,
    gains: &PdGains,
) -> EpisodeEvaluation {
    let mut simulator = SymtropyFlightSimulator::new();
    simulator.reset_with_perturbation(scenario.setpoint.position[2], 0.06, seed);
    let mut pid_state = PidState::default();
    let mut error_squared_sum = 0.0;
    let mut zero_squared_sum = 0.0;
    let mut max_error_norm = 0.0_f64;
    let mut step_error_norms = Vec::with_capacity(steps);
    let mut invalid_samples = 0_usize;
    let mut inference_us = Vec::with_capacity(steps);

    for step in 0..steps {
        let before = simulator.state().clone();
        let action = pid_baseline(&before, &scenario.setpoint, gains, &mut pid_state, DT_S)
            .clamped();

        // Time only causal feature preparation and predictor inference. dt is known before
        // the transition and comes from the fixed simulator tick; no target/next state leaks in.
        let started = Instant::now();
        let prediction = model.predict(&before, &action, DT_S);
        inference_us.push(started.elapsed().as_secs_f64() * 1.0e6);

        // Hidden disturbance evolves the held-out plant only; it is not a predictor feature.
        let force = scenario.force_at(seed, step, MASS_KG);
        simulator.apply_external_force(force);
        simulator.step(&action, DT_S);
        let after = simulator.state().clone();
        let target = LinearTransitionResidual::from_transition(&before, &action, &after, nominal);

        match (prediction, target) {
            (Ok(prediction), Ok(target)) => {
                let mut squared_error_norm = 0.0;
                for axis in 0..3 {
                    let error = target.residual_acceleration_world_m_s2[axis] - prediction[axis];
                    squared_error_norm += error * error;
                    error_squared_sum += error * error;
                    zero_squared_sum += target.residual_acceleration_world_m_s2[axis].powi(2);
                }
                let error_norm = squared_error_norm.sqrt();
                max_error_norm = max_error_norm.max(error_norm);
                step_error_norms.push(error_norm);
            }
            _ => {
                invalid_samples = invalid_samples.saturating_add(1);
                step_error_norms.push(f64::INFINITY);
            }
        }
    }

    EpisodeEvaluation {
        scenario_id: scenario.id,
        scenario_family: scenario.family,
        seed,
        sample_count: step_error_norms.len().saturating_sub(invalid_samples),
        error_squared_sum,
        zero_squared_sum,
        max_error_norm: if invalid_samples == 0 { max_error_norm } else { f64::INFINITY },
        step_error_norms,
        invalid_samples,
        ground_contact_steps: simulator.ground_contact_count(),
        inference_us,
    }
}

fn summarize_split(
    episodes: &[EpisodeEvaluation],
    radius: Option<f64>,
) -> (Value, Value) {
    let mut all = ErrorAccumulator::default();
    let mut by_family: BTreeMap<String, Vec<&EpisodeEvaluation>> = BTreeMap::new();
    let mut covered_episodes = 0usize;
    let mut covered_steps = 0usize;
    let mut valid_steps = 0usize;

    for episode in episodes {
        all.merge(&episode.error_accumulator());
        by_family.entry(episode.scenario_family.to_owned()).or_default().push(episode);
        if let Some(radius) = radius {
            if episode.invalid_samples == 0 && episode.max_error_norm <= radius {
                covered_episodes += 1;
            }
            for error in &episode.step_error_norms {
                valid_steps += 1;
                if error.is_finite() && *error <= radius {
                    covered_steps += 1;
                }
            }
        }
    }

    let coverage = if let Some(radius) = radius {
        json!({
            "nominal_coverage": 1.0 - CONFORMAL_ALPHA,
            "radius_euclidean_m_s2": radius,
            "diameter_euclidean_m_s2": 2.0 * radius,
            "episode_coverage": covered_episodes as f64 / episodes.len().max(1) as f64,
            "step_coverage": if valid_steps > 0 {
                Some(covered_steps as f64 / valid_steps as f64)
            } else {
                None
            },
            "covered_episodes": covered_episodes,
            "episode_count": episodes.len(),
            "covered_steps": covered_steps,
            "valid_scored_steps": valid_steps,
            "interpretation": "empirical coverage diagnostic; validation and test scenario families differ, so no distribution-shift guarantee"
        })
    } else {
        json!({
            "nominal_coverage": 1.0 - CONFORMAL_ALPHA,
            "radius_euclidean_m_s2": null,
            "episode_coverage": null,
            "step_coverage": null,
            "episode_count": episodes.len(),
            "interpretation": "unavailable; too few valid independent calibration episodes or invalid calibration data"
        })
    };

    let families = by_family
        .into_iter()
        .map(|(family, rows)| {
            let mut accum = ErrorAccumulator::default();
            let mut episode_covered = 0usize;
            let mut step_covered = 0usize;
            let mut total_steps = 0usize;
            for episode in &rows {
                accum.merge(&episode.error_accumulator());
                if let Some(radius) = radius {
                    if episode.invalid_samples == 0 && episode.max_error_norm <= radius {
                        episode_covered += 1;
                    }
                    for error in &episode.step_error_norms {
                        total_steps += 1;
                        if error.is_finite() && *error <= radius {
                            step_covered += 1;
                        }
                    }
                }
            }
            let mut value = accum.metrics();
            value["episode_count"] = json!(rows.len());
            value["scenario_id_examples"] = json!(rows.iter().map(|episode| episode.scenario_id).collect::<Vec<_>>());
            value["episode_coverage"] = radius.map(|_| episode_covered as f64 / rows.len().max(1) as f64).map(Value::from).unwrap_or(Value::Null);
            value["step_coverage"] = radius.and_then(|_| {
                if total_steps > 0 {
                    Some(Value::from(step_covered as f64 / total_steps as f64))
                } else {
                    None
                }
            }).unwrap_or(Value::Null);
            (family, value)
        })
        .collect::<Map<_, _>>();

    let overall = json!({
        "metrics": all.metrics(),
        "conformal_coverage": coverage,
        "family_metrics": families,
        "episode_count": episodes.len(),
        "invalid_episode_count": episodes.iter().filter(|episode| episode.invalid_samples > 0).count()
    });
    let episode_rows = episodes
        .iter()
        .map(|episode| {
            let covered = radius.map(|radius| {
                episode.invalid_samples == 0 && episode.max_error_norm <= radius
            });
            json!({
                "scenario_id": episode.scenario_id,
                "scenario_family": episode.scenario_family,
                "episode_seed": episode.seed,
                "episode_seed_hex": format!("{:016x}", episode.seed),
                "sample_count": episode.sample_count,
                "invalid_samples": episode.invalid_samples,
                "rmse_m_s2": if episode.sample_count > 0 && episode.invalid_samples == 0 {
                    Some((episode.error_squared_sum / (episode.sample_count * 3) as f64).sqrt())
                } else { None },
                "zero_residual_rmse_m_s2": if episode.sample_count > 0 && episode.invalid_samples == 0 {
                    Some((episode.zero_squared_sum / (episode.sample_count * 3) as f64).sqrt())
                } else { None },
                "max_vector_prediction_error_m_s2": if episode.max_error_norm.is_finite() {
                    Some(episode.max_error_norm)
                } else { None },
                "episode_covered": covered,
                "ground_contact_steps": episode.ground_contact_steps,
                "inference_p50_us": percentile(&episode.inference_us, 0.50),
                "inference_p95_us": percentile(&episode.inference_us, 0.95),
                "inference_p99_us": percentile(&episode.inference_us, 0.99)
            })
        })
        .collect::<Vec<_>>();
    (overall, json!(episode_rows))
}

fn main() {
    let input_path = std::env::var("STFR_DATASET_INPUT_PATH")
        .expect("STFR_DATASET_INPUT_PATH must point to a verified training-split JSON artifact");
    let train_samples = parse_training_samples(&input_path);
    let ridge_lambda = std::env::var("STFR_RIDGE_LAMBDA")
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(1.0e-3);
    let training_started = Instant::now();
    let model =
        LinearResidualRidge::fit(&train_samples, ridge_lambda).expect("ridge fit must be valid");
    let training_seconds = training_started.elapsed().as_secs_f64();

    let calibration_episodes = env_usize("STFR_RIDGE_CALIBRATION_EPISODES", 18);
    let test_episodes = env_usize("STFR_RIDGE_EVAL_EPISODES", 18);
    let steps_per_episode = env_usize("STFR_RIDGE_EVAL_STEPS", 64);
    let validation_scenarios = scenarios_for_split(ScenarioSplit::Validation);
    let test_scenarios = scenarios_for_split(ScenarioSplit::Test);
    assert!(!validation_scenarios.is_empty(), "validation scenario catalogue must not be empty");
    assert!(!test_scenarios.is_empty(), "test scenario catalogue must not be empty");
    let nominal = NominalLinearDynamics {
        mass_kg: MASS_KG,
        gravity_m_s2: GRAVITY_M_S2,
        linear_drag_n_s_per_m: LINEAR_DRAG_N_S_PER_M,
    };
    let gains = PdGains::default();

    let validation_rows = (0..calibration_episodes)
        .map(|index| {
            let seed = 0x5359_4341_4C00_0000_u64.wrapping_add(index as u64 + 1);
            let scenario = &validation_scenarios[index % validation_scenarios.len()];
            evaluate_episode(&model, scenario, seed, steps_per_episode, nominal, &gains)
        })
        .collect::<Vec<_>>();
    let calibration_scores = validation_rows
        .iter()
        .filter_map(EpisodeEvaluation::calibration_score)
        .collect::<Vec<_>>();
    let radius = split_conformal_radius(&calibration_scores, CONFORMAL_ALPHA);

    let test_rows = (0..test_episodes)
        .map(|index| {
            let seed = 0x5359_5445_5354_0000_u64.wrapping_add(index as u64 + 1);
            let scenario = &test_scenarios[index % test_scenarios.len()];
            evaluate_episode(&model, scenario, seed, steps_per_episode, nominal, &gains)
        })
        .collect::<Vec<_>>();

    let (validation_summary, validation_episode_details) =
        summarize_split(&validation_rows, None);
    let (test_summary, test_episode_details) = summarize_split(&test_rows, radius);
    println!("{}", json!({
        "benchmark_id": "symtropy-causal-transition-residual-ridge-conformal-v0.1",
        "claim_ceiling": "one_step_heldout_prediction_and_empirical_episode_block_calibration_not_control_or_safety_evidence",
        "source_receipt": {
            "symtropy_git_sha": std::env::var("STFR_SYMTROPY_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "symthaea_git_sha": std::env::var("STFR_SYMTHAEA_SHA").unwrap_or_else(|_| "unset".to_owned()),
            "rustc_version": std::env::var("STFR_RUSTC_VERSION").unwrap_or_else(|_| "unset".to_owned()),
            "scenario_matrix_sha256": std::env::var("STFR_SCENARIO_MATRIX_SHA256").unwrap_or_else(|_| "unset".to_owned()),
            "training_dataset_sha256": std::env::var("STFR_TRAIN_DATASET_SHA256").unwrap_or_else(|_| "unset".to_owned())
        },
        "training": {
            "sample_count": train_samples.len(),
            "training_seconds": training_seconds,
            "fit_method": "fixed_scale_ridge_normal_equations_partial_pivoting",
            "ridge_lambda": model.ridge_lambda(),
            "feature_boundary": "state_t, applied_action, dt_s only; residual target is label",
            "hidden_force_labels_used_as_features": false
        },
        "calibration": {
            "split": "validation",
            "alpha": CONFORMAL_ALPHA,
            "score": "maximum one-step vector residual-prediction error per episode",
            "score_count_requested": calibration_episodes,
            "valid_score_count": calibration_scores.len(),
            "scenario_ids": validation_scenarios.iter().map(|scenario| scenario.id).collect::<Vec<_>>(),
            "scores_m_s2": calibration_scores,
            "conformal_radius_m_s2": radius,
            "validation_summary": validation_summary,
            "episode_details": validation_episode_details
        },
        "evaluation": {
            "split": "test",
            "episodes_requested": test_episodes,
            "steps_per_episode": steps_per_episode,
            "scenario_ids": test_scenarios.iter().map(|scenario| scenario.id).collect::<Vec<_>>(),
            "nominal_model": {
                "mass_kg": nominal.mass_kg,
                "gravity_m_s2": nominal.gravity_m_s2,
                "linear_drag_n_s_per_m": nominal.linear_drag_n_s_per_m
            },
            "summary": test_summary,
            "episode_details": test_episode_details,
        },
        "coverage_caveat": "The conformal radius is calibrated at the episode/block level to avoid treating autocorrelated frames as independent. Classical split-conformal finite-sample coverage requires exchangeability; validation and test scenario families intentionally differ, so reported test coverage is empirical shift robustness only, not a guarantee."
    }));
}
