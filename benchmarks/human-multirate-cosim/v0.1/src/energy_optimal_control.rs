//! Exact energy-ball optimum for one fixed pair of linear graph models
//! under a declared piecewise-constant actuator basis.
//!
//! The optimizer is continuous-valued in the basis coefficients. It solves
//! the exact Rayleigh-quotient maximum over the integrated-energy ellipsoid.
//! A peak-amplitude cap is checked separately: if the unconstrained optimum
//! violates it, the returned scaled profile is feasible but only a lower bound
//! on the optimum under the additional box constraint.
//!
//! This is not arbitrary-waveform Pontryagin control, nuisance-profiled
//! T-optimal design, or a maximin multi-model-family optimizer.

// Matrix kernels below use explicit row/column indices so that symmetry and
// coefficient ordering remain visible in the reference implementation.
#![allow(clippy::needless_range_loop)]

use crate::experiment_design::{
    evaluate_input_profile, ExperimentDesignScenario, ExcitationDesignError, ForcingSegment,
    InputProfile,
};
use crate::network::{symmetric_eigen, NetworkError};
use crate::sensor_design::{invert_spd, SensorDesignError};
use std::collections::HashSet;
use std::fmt;

const CONTROL_TOLERANCE: f64 = 1.0e-11;
const MAX_CONTROL_BASIS: usize = 16;
const MAX_CONTROL_SEGMENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyBallOptimalityDisposition {
    /// The top-eigenvector control satisfies the peak cap, so it is globally
    /// optimal over the complete declared piecewise-constant energy ball.
    ExactEnergyBallOptimum,
    /// The energy-ball eigenvector violates the peak cap. The returned profile
    /// is scaled into the feasible set; its score is a lower bound only.
    PeakConstraintActive,
    /// Every control direction in the declared basis has zero output
    /// separation for this model pair under the selected observation map.
    NoDiscriminatingControl,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ControlCoefficientV1 {
    pub segment_index: usize,
    pub start_time: f64,
    pub end_time: f64,
    pub actuator_node: usize,
    pub amplitude: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnergyOptimalControlReportV1 {
    pub left_model_id: u32,
    pub right_model_id: u32,
    pub selected_sensor_ids: Vec<u32>,
    pub actuator_nodes: Vec<usize>,
    pub time_boundaries: Vec<f64>,
    pub coefficients: Vec<ControlCoefficientV1>,
    pub profile: InputProfile,
    pub optimality_disposition: EnergyBallOptimalityDisposition,
    pub basis_dimension: usize,
    pub top_generalized_eigenvalue: f64,
    /// Maximum objective over the energy ball before applying the peak cap.
    pub unconstrained_energy_ball_upper_bound: f64,
    /// Objective attained by the returned, replayed profile.
    pub achieved_gaussian_kl: f64,
    pub energy_budget: f64,
    pub achieved_integrated_squared_input: f64,
    pub peak_amplitude_limit: f64,
    pub achieved_peak_amplitude: f64,
    pub peak_constraint_active: bool,
    pub replay_objective_abs_error: f64,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyOptimalControlError {
    Experiment(ExcitationDesignError),
    SensorDesign(SensorDesignError),
    Network(NetworkError),
    InvalidModelPair,
    UnknownModelId(u32),
    InvalidControlPartition,
    TooManyControlSegments,
    EmptyActuatorSet,
    DuplicateActuatorNode(usize),
    UnknownActuatorNode(usize),
    ExcessiveControlBasis,
    MissingModelPrediction(u32),
    InvalidPredictionShape,
    NonFinitePrediction,
    InvalidProbeAmplitude,
    NonFiniteGramMatrix,
    SpectralFailure,
    InvalidReplayObjective,
}

impl fmt::Display for EnergyOptimalControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Experiment(error) => write!(f, "experiment scenario/profile invalid: {error}"),
            Self::SensorDesign(error) => write!(f, "selected sensor covariance invalid: {error}"),
            Self::Network(error) => write!(f, "graph spectral oracle failed: {error}"),
            Self::InvalidModelPair => write!(f, "two distinct model IDs are required"),
            Self::UnknownModelId(id) => write!(f, "model ID {id} is not registered"),
            Self::InvalidControlPartition => {
                write!(f, "control boundaries must strictly partition [0, horizon]")
            }
            Self::TooManyControlSegments => {
                write!(f, "control partition exceeds the segment limit")
            }
            Self::EmptyActuatorSet => write!(f, "at least one actuator node is required"),
            Self::DuplicateActuatorNode(node) => {
                write!(f, "actuator node {node} is repeated")
            }
            Self::UnknownActuatorNode(node) => {
                write!(f, "actuator node {node} is outside the graph")
            }
            Self::ExcessiveControlBasis => {
                write!(f, "actuator/time basis exceeds the exact spectral dimension limit")
            }
            Self::MissingModelPrediction(id) => {
                write!(f, "model ID {id} is missing from the experiment prediction")
            }
            Self::InvalidPredictionShape => {
                write!(f, "model predictions do not match the observation dimensions")
            }
            Self::NonFinitePrediction => write!(f, "model prediction contains a non-finite value"),
            Self::InvalidProbeAmplitude => {
                write!(f, "could not construct a feasible positive control-basis probe")
            }
            Self::NonFiniteGramMatrix => {
                write!(f, "control discrimination Gram matrix is non-finite")
            }
            Self::SpectralFailure => write!(f, "symmetric control Gram eigensolver failed"),
            Self::InvalidReplayObjective => {
                write!(f, "replayed model-pair score is missing or non-finite")
            }
        }
    }
}

impl std::error::Error for EnergyOptimalControlError {}

/// Solve the continuous-valued energy-ball optimum for a fixed pair of
/// candidate models over a piecewise-constant time/actuator basis.
///
/// Input coefficient ordering is segment-major, then actuator-node order.
/// The energy metric repeats each segment duration once per actuator. Sensor
/// covariance is shared across observation times, which remain independent.
/// The existing experiment evaluator is used to replay the final waveform.
pub fn optimize_pairwise_energy_ball_control(
    scenario: &ExperimentDesignScenario,
    left_model_id: u32,
    right_model_id: u32,
    selected_sensor_ids: &[u32],
    actuator_nodes: &[usize],
    time_boundaries: &[f64],
) -> Result<EnergyOptimalControlReportV1, EnergyOptimalControlError> {
    scenario
        .validate()
        .map_err(EnergyOptimalControlError::Experiment)?;
    if left_model_id == right_model_id {
        return Err(EnergyOptimalControlError::InvalidModelPair);
    }
    if !scenario.models.iter().any(|model| model.id == left_model_id) {
        return Err(EnergyOptimalControlError::UnknownModelId(left_model_id));
    }
    if !scenario.models.iter().any(|model| model.id == right_model_id) {
        return Err(EnergyOptimalControlError::UnknownModelId(right_model_id));
    }
    if actuator_nodes.is_empty() {
        return Err(EnergyOptimalControlError::EmptyActuatorSet);
    }
    if time_boundaries.len() < 2 {
        return Err(EnergyOptimalControlError::InvalidControlPartition);
    }
    let segment_count = time_boundaries.len() - 1;
    if segment_count > MAX_CONTROL_SEGMENTS {
        return Err(EnergyOptimalControlError::TooManyControlSegments);
    }
    let horizon = scenario.horizon();
    if !time_boundaries[0].is_finite()
        || !time_boundaries[time_boundaries.len() - 1].is_finite()
        || time_boundaries[0].abs() > CONTROL_TOLERANCE
        || (time_boundaries[time_boundaries.len() - 1] - horizon).abs()
            > CONTROL_TOLERANCE
        || time_boundaries.iter().any(|time| !time.is_finite())
        || time_boundaries.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(EnergyOptimalControlError::InvalidControlPartition);
    }
    let mut boundaries = time_boundaries.to_vec();
    boundaries[0] = 0.0;
    let last = boundaries.len() - 1;
    boundaries[last] = horizon;

    let mut ordered_actuators = actuator_nodes.to_vec();
    ordered_actuators.sort_unstable();
    if let Some(duplicate) = ordered_actuators
        .windows(2)
        .find(|pair| pair[0] == pair[1])
    {
        return Err(EnergyOptimalControlError::DuplicateActuatorNode(
            duplicate[0],
        ));
    }
    let graph_dimension = scenario.models[0].network.capacities.len();
    if let Some(unknown) = ordered_actuators
        .iter()
        .find(|node| **node >= graph_dimension)
    {
        return Err(EnergyOptimalControlError::UnknownActuatorNode(
            *unknown,
        ));
    }

    let basis_dimension = segment_count
        .checked_mul(ordered_actuators.len())
        .ok_or(EnergyOptimalControlError::ExcessiveControlBasis)?;
    if basis_dimension == 0 || basis_dimension > MAX_CONTROL_BASIS {
        return Err(EnergyOptimalControlError::ExcessiveControlBasis);
    }

    let passive_profile = InputProfile {
        id: 0,
        label: "continuous_control_passive_reference",
        segments: boundaries
            .windows(2)
            .map(|pair| ForcingSegment {
                start_time: pair[0],
                end_time: pair[1],
                forcing: vec![0.0; graph_dimension],
            })
            .collect(),
    };
    let passive_score = evaluate_input_profile(
        scenario,
        &passive_profile,
        selected_sensor_ids,
    )
    .map_err(EnergyOptimalControlError::Experiment)?;
    let passive_left = prediction_vector(
        &passive_score,
        left_model_id,
        scenario.observation_times.len(),
        selected_sensor_ids.len(),
    )?;
    let passive_right = prediction_vector(
        &passive_score,
        right_model_id,
        scenario.observation_times.len(),
        selected_sensor_ids.len(),
    )?;

    let maximum_segment_duration = boundaries
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .fold(0.0_f64, f64::max);
    let probe_amplitude = scenario
        .max_abs_forcing
        .min((scenario.input_energy_budget / maximum_segment_duration).sqrt())
        * 0.5;
    if !probe_amplitude.is_finite()
        || probe_amplitude <= 0.0
        || probe_amplitude * probe_amplitude * maximum_segment_duration
            > scenario.input_energy_budget
    {
        return Err(EnergyOptimalControlError::InvalidProbeAmplitude);
    }

    let sensor_indices = selected_sensor_indices(scenario, selected_sensor_ids)
        .map_err(EnergyOptimalControlError::Experiment)?;
    let selected_covariance = principal_submatrix(&scenario.noise_covariance, &sensor_indices);
    let covariance_inverse = invert_spd(&selected_covariance)
        .map_err(EnergyOptimalControlError::SensorDesign)?;

    let output_dimension = scenario
        .observation_times
        .len()
        .checked_mul(selected_sensor_ids.len())
        .ok_or(EnergyOptimalControlError::ExcessiveControlBasis)?;
    let mut response_columns = vec![vec![0.0; output_dimension]; basis_dimension];
    let mut durations = Vec::with_capacity(basis_dimension);

    for segment_index in 0..segment_count {
        let duration = boundaries[segment_index + 1] - boundaries[segment_index];
        for actuator_index in 0..ordered_actuators.len() {
            let coefficient_index =
                segment_index * ordered_actuators.len() + actuator_index;
            durations.push(duration);
            let mut segments = Vec::with_capacity(segment_count);
            for candidate_segment in 0..segment_count {
                let mut forcing = vec![0.0; graph_dimension];
                if candidate_segment == segment_index {
                    forcing[ordered_actuators[actuator_index]] = probe_amplitude;
                }
                segments.push(ForcingSegment {
                    start_time: boundaries[candidate_segment],
                    end_time: boundaries[candidate_segment + 1],
                    forcing,
                });
            }
            let basis_profile = InputProfile {
                id: u32::try_from(coefficient_index + 1)
                    .map_err(|_| EnergyOptimalControlError::ExcessiveControlBasis)?,
                label: "continuous_control_unit_basis_probe",
                segments,
            };
            let score = evaluate_input_profile(
                scenario,
                &basis_profile,
                selected_sensor_ids,
            )
            .map_err(EnergyOptimalControlError::Experiment)?;
            let left = prediction_vector(
                &score,
                left_model_id,
                scenario.observation_times.len(),
                selected_sensor_ids.len(),
            )?;
            let right = prediction_vector(
                &score,
                right_model_id,
                scenario.observation_times.len(),
                selected_sensor_ids.len(),
            )?;
            for output_index in 0..output_dimension {
                response_columns[coefficient_index][output_index] =
                    ((left[output_index] - right[output_index])
                        - (passive_left[output_index] - passive_right[output_index]))
                        / probe_amplitude;
            }
        }
    }

    let mut gram = vec![vec![0.0; basis_dimension]; basis_dimension];
    let sensor_count = selected_sensor_ids.len();
    for left_column in 0..basis_dimension {
        for right_column in left_column..basis_dimension {
            let mut value = 0.0;
            for time_index in 0..scenario.observation_times.len() {
                let offset = time_index * sensor_count;
                let left_values = &response_columns[left_column][offset..offset + sensor_count];
                let right_values =
                    &response_columns[right_column][offset..offset + sensor_count];
                let weighted_right = matrix_vector_product(&covariance_inverse, right_values);
                value += left_values
                    .iter()
                    .zip(weighted_right.iter())
                    .map(|(left, right)| left * right)
                    .sum::<f64>();
            }
            if !value.is_finite() {
                return Err(EnergyOptimalControlError::NonFiniteGramMatrix);
            }
            gram[left_column][right_column] = value;
            gram[right_column][left_column] = value;
        }
    }

    let mut energy_whitened_gram = vec![vec![0.0; basis_dimension]; basis_dimension];
    for row in 0..basis_dimension {
        for column in 0..basis_dimension {
            energy_whitened_gram[row][column] =
                gram[row][column] / (durations[row] * durations[column]).sqrt();
        }
    }
    let (eigenvalues, eigenvectors) = symmetric_eigen(energy_whitened_gram)
        .map_err(|_| EnergyOptimalControlError::SpectralFailure)?;
    let top_index = eigenvalues
        .iter()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(right.1))
        .map(|(index, _)| index)
        .ok_or(EnergyOptimalControlError::SpectralFailure)?;
    let top_eigenvalue = eigenvalues[top_index];
    if !top_eigenvalue.is_finite() || top_eigenvalue < -CONTROL_TOLERANCE {
        return Err(EnergyOptimalControlError::SpectralFailure);
    }
    let top_eigenvalue = top_eigenvalue.max(0.0);
    let unconstrained_upper_bound =
        0.5 * scenario.input_energy_budget * top_eigenvalue;
    let zero_tolerance = CONTROL_TOLERANCE
        * (1.0 + gram.iter().flatten().map(|value| value.abs()).fold(0.0_f64, f64::max));
    if top_eigenvalue <= zero_tolerance {
        let zero_profile = InputProfile {
            id: u32::MAX,
            label: "no_discriminating_control_zero_profile",
            segments: boundaries
                .windows(2)
                .map(|pair| ForcingSegment {
                    start_time: pair[0],
                    end_time: pair[1],
                    forcing: vec![0.0; graph_dimension],
                })
                .collect(),
        };
        return Ok(EnergyOptimalControlReportV1 {
            left_model_id,
            right_model_id,
            selected_sensor_ids: selected_sensor_ids.to_vec(),
            actuator_nodes: ordered_actuators,
            time_boundaries: boundaries,
            coefficients: Vec::new(),
            profile: zero_profile,
            optimality_disposition: EnergyBallOptimalityDisposition::NoDiscriminatingControl,
            basis_dimension,
            top_generalized_eigenvalue: top_eigenvalue,
            unconstrained_energy_ball_upper_bound: 0.0,
            achieved_gaussian_kl: 0.0,
            energy_budget: scenario.input_energy_budget,
            achieved_integrated_squared_input: 0.0,
            peak_amplitude_limit: scenario.max_abs_forcing,
            achieved_peak_amplitude: 0.0,
            peak_constraint_active: false,
            replay_objective_abs_error: 0.0,
            assumptions: assumptions(),
        });
    }

    let mut amplitudes = vec![0.0; basis_dimension];
    let energy_scale = scenario.input_energy_budget.sqrt();
    let mut raw_peak: f64 = 0.0;
    for index in 0..basis_dimension {
        amplitudes[index] = energy_scale * eigenvectors[index][top_index]
            / durations[index].sqrt();
        raw_peak = raw_peak.max(amplitudes[index].abs());
    }
    let peak_constraint_active =
        raw_peak > scenario.max_abs_forcing + CONTROL_TOLERANCE;
    if peak_constraint_active && raw_peak > 0.0 {
        let scale = scenario.max_abs_forcing / raw_peak;
        for amplitude in &mut amplitudes {
            *amplitude *= scale;
        }
    }

    let coefficients: Vec<ControlCoefficientV1> = (0..basis_dimension)
        .map(|index| {
            let segment_index = index / ordered_actuators.len();
            let actuator_index = index % ordered_actuators.len();
            ControlCoefficientV1 {
                segment_index,
                start_time: boundaries[segment_index],
                end_time: boundaries[segment_index + 1],
                actuator_node: ordered_actuators[actuator_index],
                amplitude: amplitudes[index],
            }
        })
        .collect();

    let mut profile_segments = Vec::with_capacity(segment_count);
    for segment_index in 0..segment_count {
        let mut forcing = vec![0.0; graph_dimension];
        for actuator_index in 0..ordered_actuators.len() {
            let coefficient_index =
                segment_index * ordered_actuators.len() + actuator_index;
            forcing[ordered_actuators[actuator_index]] = amplitudes[coefficient_index];
        }
        profile_segments.push(ForcingSegment {
            start_time: boundaries[segment_index],
            end_time: boundaries[segment_index + 1],
            forcing,
        });
    }
    let profile = InputProfile {
        id: u32::MAX - 1,
        label: "spectral_energy_ball_optimal_control",
        segments: profile_segments,
    };
    let replay = evaluate_input_profile(scenario, &profile, selected_sensor_ids)
        .map_err(EnergyOptimalControlError::Experiment)?;
    let achieved_gaussian_kl = replay
        .pairwise_separations
        .iter()
        .find(|pair| {
            (pair.left_model_id == left_model_id && pair.right_model_id == right_model_id)
                || (pair.left_model_id == right_model_id
                    && pair.right_model_id == left_model_id)
        })
        .map(|pair| pair.gaussian_kl_separation)
        .ok_or(EnergyOptimalControlError::InvalidReplayObjective)?;
    if !achieved_gaussian_kl.is_finite() {
        return Err(EnergyOptimalControlError::InvalidReplayObjective);
    }
    let predicted_objective = quadratic_objective(&gram, &amplitudes);
    let replay_objective_abs_error = (predicted_objective - achieved_gaussian_kl).abs();
    let (achieved_integrated_squared_input, achieved_peak_amplitude) =
        profile_cost(&profile);
    let optimality_disposition = if peak_constraint_active {
        EnergyBallOptimalityDisposition::PeakConstraintActive
    } else {
        EnergyBallOptimalityDisposition::ExactEnergyBallOptimum
    };

    Ok(EnergyOptimalControlReportV1 {
        left_model_id,
        right_model_id,
        selected_sensor_ids: selected_sensor_ids.to_vec(),
        actuator_nodes: ordered_actuators,
        time_boundaries: boundaries,
        coefficients,
        profile,
        optimality_disposition,
        basis_dimension,
        top_generalized_eigenvalue: top_eigenvalue,
        unconstrained_energy_ball_upper_bound: unconstrained_upper_bound,
        achieved_gaussian_kl,
        energy_budget: scenario.input_energy_budget,
        achieved_integrated_squared_input,
        peak_amplitude_limit: scenario.max_abs_forcing,
        achieved_peak_amplitude,
        peak_constraint_active,
        replay_objective_abs_error,
        assumptions: assumptions(),
    })
}

fn assumptions() -> Vec<&'static str> {
    vec![
        "one fixed pair of linear graph models",
        "common state basis, capacity vector, initial state and horizon",
        "piecewise-constant control over the declared temporal partition",
        "continuous-valued amplitudes optimized over an integrated-energy ellipsoid",
        "fixed selected sensors and common sensor covariance",
        "independent observation-noise blocks across observation times",
        "peak amplitude is checked after the energy-ball optimum; active peak constraints are not globally optimized",
        "fixed model parameters; no nuisance-parameter profiling",
        "no slew-rate or actuator-state constraint",
        "synthetic mathematical benchmark only",
    ]
}

fn selected_sensor_indices(
    scenario: &ExperimentDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<Vec<usize>, ExcitationDesignError> {
    if selected_sensor_ids.is_empty() {
        return Err(ExcitationDesignError::EmptySensorSelection);
    }
    let mut seen = HashSet::new();
    let mut indices = Vec::with_capacity(selected_sensor_ids.len());
    for sensor_id in selected_sensor_ids {
        if !seen.insert(*sensor_id) {
            return Err(ExcitationDesignError::DuplicateSelectedSensor(
                *sensor_id,
            ));
        }
        let index = scenario
            .sensors
            .iter()
            .position(|sensor| sensor.id == *sensor_id)
            .ok_or(ExcitationDesignError::UnknownSensorId(*sensor_id))?;
        indices.push(index);
    }
    let total_cost: f64 = indices
        .iter()
        .map(|index| scenario.sensors[*index].cost)
        .sum();
    if total_cost > scenario.sensor_budget + CONTROL_TOLERANCE {
        return Err(ExcitationDesignError::SensorBudgetExceeded);
    }
    Ok(indices)
}

fn principal_submatrix(matrix: &[Vec<f64>], indices: &[usize]) -> Vec<Vec<f64>> {
    indices
        .iter()
        .map(|row| indices.iter().map(|column| matrix[*row][*column]).collect())
        .collect()
}

fn prediction_vector(
    score: &crate::experiment_design::ExperimentDesignScore,
    model_id: u32,
    time_count: usize,
    sensor_count: usize,
) -> Result<Vec<f64>, EnergyOptimalControlError> {
    let prediction = score
        .model_observation_means
        .iter()
        .find(|prediction| prediction.model_id == model_id)
        .ok_or(EnergyOptimalControlError::MissingModelPrediction(model_id))?;
    if prediction.observations.len() != time_count
        || prediction
            .observations
            .iter()
            .any(|row| row.len() != sensor_count)
    {
        return Err(EnergyOptimalControlError::InvalidPredictionShape);
    }
    if prediction.observations.iter().flatten().any(|value| !value.is_finite()) {
        return Err(EnergyOptimalControlError::NonFinitePrediction);
    }
    Ok(prediction.observations.iter().flatten().copied().collect())
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

fn quadratic_objective(matrix: &[Vec<f64>], vector: &[f64]) -> f64 {
    0.5 * vector
        .iter()
        .zip(matrix_vector_product(matrix, vector).iter())
        .map(|(left, right)| left * right)
        .sum::<f64>()
}

fn profile_cost(profile: &InputProfile) -> (f64, f64) {
    let mut energy = 0.0;
    let mut peak: f64 = 0.0;
    for segment in &profile.segments {
        let duration = segment.end_time - segment.start_time;
        for amplitude in &segment.forcing {
            energy += duration * amplitude * amplitude;
            peak = peak.max(amplitude.abs());
        }
    }
    (energy, peak)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::ModelCandidate;
    use crate::network::{Edge, NetworkScenario};
    use crate::sensor_design::SensorCandidate;

    fn network(edges: Vec<Edge>) -> NetworkScenario {
        NetworkScenario {
            name: "three-node-control-fixture",
            capacities: vec![1.0, 1.0, 1.0],
            initial: vec![0.2, 0.0, 0.0],
            edges,
            horizon: 2.0,
        }
    }

    fn scenario(max_abs_forcing: f64) -> ExperimentDesignScenario {
        ExperimentDesignScenario {
            name: "energy-ball-control-fixture",
            models: vec![
                ModelCandidate {
                    id: 1,
                    label: "chain",
                    network: network(vec![
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
                    ]),
                },
                ModelCandidate {
                    id: 2,
                    label: "chain-with-bypass",
                    network: network(vec![
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
                        Edge {
                            left: 0,
                            right: 2,
                            conductance: 0.4,
                        },
                    ]),
                },
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

    #[test]
    fn spectral_energy_ball_solution_replays_exact_pairwise_kl() {
        let report = optimize_pairwise_energy_ball_control(
            &scenario(2.0),
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 0.5, 1.0, 2.0],
        )
        .unwrap();
        assert_eq!(
            report.optimality_disposition,
            EnergyBallOptimalityDisposition::ExactEnergyBallOptimum
        );
        assert!(!report.peak_constraint_active);
        assert!((report.achieved_integrated_squared_input - 0.5).abs() < 1.0e-9);
        assert!(report.achieved_peak_amplitude <= report.peak_amplitude_limit);
        assert!(report.unconstrained_energy_ball_upper_bound > 0.0);
        assert!((report.achieved_gaussian_kl
            - report.unconstrained_energy_ball_upper_bound)
            .abs()
            < 1.0e-8);
        assert!(report.replay_objective_abs_error < 1.0e-9);
    }

    #[test]
    fn replayed_score_matches_independently_assembled_dense_kl() {
        let fixture = scenario(2.0);
        let report = optimize_pairwise_energy_ball_control(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 0.5, 1.0, 2.0],
        )
        .unwrap();
        let replay = evaluate_input_profile(&fixture, &report.profile, &[10, 11]).unwrap();
        let left = replay
            .model_observation_means
            .iter()
            .find(|prediction| prediction.model_id == 1)
            .unwrap();
        let right = replay
            .model_observation_means
            .iter()
            .find(|prediction| prediction.model_id == 2)
            .unwrap();

        // Independent closed-form inverse for R = [[0.02, 0.005], [0.005, 0.03]].
        let a = 0.02_f64;
        let b = 0.005_f64;
        let c = 0.03_f64;
        let determinant = a * c - b * b;
        let inverse = [
            [c / determinant, -b / determinant],
            [-b / determinant, a / determinant],
        ];
        let mut dense_reference = 0.0;
        for time_index in 0..fixture.observation_times.len() {
            let d0 = left.observations[time_index][0]
                - right.observations[time_index][0];
            let d1 = left.observations[time_index][1]
                - right.observations[time_index][1];
            dense_reference += 0.5
                * (d0 * (inverse[0][0] * d0 + inverse[0][1] * d1)
                    + d1 * (inverse[1][0] * d0 + inverse[1][1] * d1));
        }
        assert!((dense_reference - report.achieved_gaussian_kl).abs() < 1.0e-8);
        assert!(
            (dense_reference - report.unconstrained_energy_ball_upper_bound).abs()
                < 1.0e-8
        );
    }

    #[test]
    fn active_peak_constraint_returns_feasible_lower_bound_not_an_optimality_claim() {
        let report = optimize_pairwise_energy_ball_control(
            &scenario(0.1),
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 0.5, 1.0, 2.0],
        )
        .unwrap();
        assert_eq!(
            report.optimality_disposition,
            EnergyBallOptimalityDisposition::PeakConstraintActive
        );
        assert!(report.peak_constraint_active);
        assert!(report.achieved_peak_amplitude <= 0.1 + CONTROL_TOLERANCE);
        assert!(report.achieved_integrated_squared_input <= report.energy_budget);
        assert!(
            report.achieved_gaussian_kl
                < report.unconstrained_energy_ball_upper_bound
        );
        assert!(report.replay_objective_abs_error < 1.0e-9);
    }

    #[test]
    fn indistinguishable_models_return_no_discriminating_control() {
        let mut fixture = scenario(2.0);
        fixture.models[1].network = fixture.models[0].network.clone();
        let report = optimize_pairwise_energy_ball_control(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0, 2],
            &[0.0, 1.0, 2.0],
        )
        .unwrap();
        assert_eq!(
            report.optimality_disposition,
            EnergyBallOptimalityDisposition::NoDiscriminatingControl
        );
        assert_eq!(report.achieved_gaussian_kl, 0.0);
        assert_eq!(profile_cost(&report.profile).0, 0.0);
    }

    #[test]
    fn invalid_partition_duplicate_actuator_and_invalid_pair_fail_closed() {
        let fixture = scenario(2.0);
        assert!(matches!(
            optimize_pairwise_energy_ball_control(
                &fixture,
                1,
                2,
                &[10],
                &[0],
                &[0.0, 0.7, 0.6, 2.0]
            ),
            Err(EnergyOptimalControlError::InvalidControlPartition)
        ));
        assert!(matches!(
            optimize_pairwise_energy_ball_control(
                &fixture,
                1,
                2,
                &[10],
                &[0, 0],
                &[0.0, 1.0, 2.0]
            ),
            Err(EnergyOptimalControlError::DuplicateActuatorNode(0))
        ));
        assert!(matches!(
            optimize_pairwise_energy_ball_control(
                &fixture,
                1,
                1,
                &[10],
                &[0],
                &[0.0, 1.0, 2.0]
            ),
            Err(EnergyOptimalControlError::InvalidModelPair)
        ));
    }

    #[test]
    fn selected_sensor_order_is_preserved_in_report() {
        let report = optimize_pairwise_energy_ball_control(
            &scenario(2.0),
            1,
            2,
            &[11, 10],
            &[2, 0],
            &[0.0, 1.0, 2.0],
        )
        .unwrap();
        assert_eq!(report.selected_sensor_ids, vec![11, 10]);
        assert_eq!(report.actuator_nodes, vec![0, 2]);
    }
}
