//! Total-KL optimal experiment design for one fixed pair of linear graph models.
//!
//! The passive observation difference creates a linear term in the energy-ball
//! objective. This module solves that offset quadratic globally with the exact
//! small-dimensional trust-region solver, including the hard case.

use crate::energy_optimal_control::{
    ControlCoefficientV1, EnergyOptimalControlError,
};
use crate::experiment_design::{
    evaluate_input_profile, ExperimentDesignScenario, ExcitationDesignError, ForcingSegment,
    InputProfile,
};
use crate::sensor_design::invert_spd;
use crate::trust_region::{
    maximize_psd_quadratic_on_ball, TrustRegionDisposition, TrustRegionError,
};
use std::collections::HashSet;
use std::fmt;

const TOTAL_KL_TOLERANCE: f64 = 1.0e-10;
const MAX_TOTAL_KL_BASIS: usize = 16;
const MAX_TOTAL_KL_SEGMENTS: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotalKlOptimalityDisposition {
    ExactTotalKlTrustRegionOptimum,
    HardCaseTotalKlTrustRegionOptimum,
    PeakConstraintActive,
    NoControlEffect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TotalKlTrustRegionReportV1 {
    pub left_model_id: u32,
    pub right_model_id: u32,
    pub selected_sensor_ids: Vec<u32>,
    pub actuator_nodes: Vec<usize>,
    pub time_boundaries: Vec<f64>,
    pub coefficients: Vec<ControlCoefficientV1>,
    pub profile: InputProfile,
    pub optimality_disposition: TotalKlOptimalityDisposition,
    pub basis_dimension: usize,
    pub passive_baseline_gaussian_kl: f64,
    pub achieved_incremental_control_gaussian_kl: f64,
    pub achieved_total_gaussian_kl: f64,
    /// Upper bound over the energy ellipsoid before peak-amplitude clipping.
    pub unconstrained_total_gaussian_kl_upper_bound: f64,
    pub trust_region_multiplier: f64,
    pub largest_whitened_eigenvalue: f64,
    pub trust_region_stationarity_residual: f64,
    pub energy_feasibility_residual: f64,
    pub trust_region_secular_residual: f64,
    pub replay_total_kl_abs_error: f64,
    pub energy_budget: f64,
    pub achieved_integrated_squared_input: f64,
    pub peak_amplitude_limit: f64,
    pub achieved_peak_amplitude: f64,
    pub peak_constraint_active: bool,
    pub trust_region_iterations: usize,
    pub assumptions: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TotalKlTrustRegionError {
    Energy(EnergyOptimalControlError),
    TrustRegion(TrustRegionError),
    InvalidModelPair,
    UnknownModelId(u32),
    InvalidControlPartition,
    TooManyControlSegments,
    EmptyActuatorSet,
    DuplicateActuatorNode(usize),
    UnknownActuatorNode(usize),
    ExcessiveControlBasis,
    InvalidProbeAmplitude,
    NonFiniteOffset,
    NonFiniteCrossTerm,
    NonFiniteReplay,
    MissingModelPrediction(u32),
    InvalidPredictionShape,
}

impl fmt::Display for TotalKlTrustRegionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Energy(error) => write!(f, "energy-ball setup failed: {error}"),
            Self::TrustRegion(error) => write!(f, "total-KL trust-region solve failed: {error}"),
            Self::InvalidModelPair => write!(f, "two distinct model IDs are required"),
            Self::UnknownModelId(id) => write!(f, "model ID {id} is not registered"),
            Self::InvalidControlPartition => {
                write!(f, "control boundaries must partition [0, horizon]")
            }
            Self::TooManyControlSegments => {
                write!(f, "control partition exceeds the segment limit")
            }
            Self::EmptyActuatorSet => write!(f, "at least one actuator node is required"),
            Self::DuplicateActuatorNode(node) => write!(f, "actuator node {node} is repeated"),
            Self::UnknownActuatorNode(node) => {
                write!(f, "actuator node {node} is outside the graph")
            }
            Self::ExcessiveControlBasis => {
                write!(f, "actuator/time basis exceeds the exact dimension limit")
            }
            Self::InvalidProbeAmplitude => {
                write!(f, "could not construct a feasible positive basis probe")
            }
            Self::NonFiniteOffset => write!(f, "passive Gaussian KL offset is non-finite"),
            Self::NonFiniteCrossTerm => write!(f, "total-KL linear cross term is non-finite"),
            Self::NonFiniteReplay => {
                write!(f, "replayed total Gaussian KL is missing or non-finite")
            }
            Self::MissingModelPrediction(id) => {
                write!(f, "model ID {id} is missing from an observation prediction")
            }
            Self::InvalidPredictionShape => {
                write!(f, "model prediction does not match the observation dimensions")
            }
        }
    }
}

impl std::error::Error for TotalKlTrustRegionError {}

/// Globally optimize total controlled Gaussian KL for a fixed model pair:
///
///   0.5 * (d0 + G u)^T P (d0 + G u)
///
/// subject to u^T W u <= energy_budget. The returned exactness disposition
/// applies only when the peak-amplitude constraint is inactive. When the peak
/// cap is active, a feasible scaled point and the unconstrained upper bound
/// are reported without claiming constrained optimality.
pub fn optimize_pairwise_total_kl_trust_region(
    scenario: &ExperimentDesignScenario,
    left_model_id: u32,
    right_model_id: u32,
    selected_sensor_ids: &[u32],
    actuator_nodes: &[usize],
    time_boundaries: &[f64],
) -> Result<TotalKlTrustRegionReportV1, TotalKlTrustRegionError> {
    scenario
        .validate()
        .map_err(EnergyOptimalControlError::Experiment)
        .map_err(TotalKlTrustRegionError::Energy)?;
    if left_model_id == right_model_id {
        return Err(TotalKlTrustRegionError::InvalidModelPair);
    }
    if !scenario.models.iter().any(|model| model.id == left_model_id) {
        return Err(TotalKlTrustRegionError::UnknownModelId(left_model_id));
    }
    if !scenario.models.iter().any(|model| model.id == right_model_id) {
        return Err(TotalKlTrustRegionError::UnknownModelId(right_model_id));
    }
    if actuator_nodes.is_empty() {
        return Err(TotalKlTrustRegionError::EmptyActuatorSet);
    }
    if time_boundaries.len() < 2 {
        return Err(TotalKlTrustRegionError::InvalidControlPartition);
    }
    let segment_count = time_boundaries.len() - 1;
    if segment_count > MAX_TOTAL_KL_SEGMENTS {
        return Err(TotalKlTrustRegionError::TooManyControlSegments);
    }
    let horizon = scenario.horizon();
    if time_boundaries.iter().any(|time| !time.is_finite())
        || time_boundaries[0].abs() > TOTAL_KL_TOLERANCE
        || (time_boundaries[time_boundaries.len() - 1] - horizon).abs()
            > TOTAL_KL_TOLERANCE
        || time_boundaries.windows(2).any(|pair| pair[1] <= pair[0])
    {
        return Err(TotalKlTrustRegionError::InvalidControlPartition);
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
        return Err(TotalKlTrustRegionError::DuplicateActuatorNode(duplicate[0]));
    }
    let graph_dimension = scenario.models[0].network.capacities.len();
    if let Some(unknown) = ordered_actuators
        .iter()
        .find(|node| **node >= graph_dimension)
    {
        return Err(TotalKlTrustRegionError::UnknownActuatorNode(*unknown));
    }
    let basis_dimension = segment_count
        .checked_mul(ordered_actuators.len())
        .ok_or(TotalKlTrustRegionError::ExcessiveControlBasis)?;
    if basis_dimension == 0 || basis_dimension > MAX_TOTAL_KL_BASIS {
        return Err(TotalKlTrustRegionError::ExcessiveControlBasis);
    }

    let passive_profile = InputProfile {
        id: 0,
        label: "total_kl_passive_zero_input_reference",
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
    .map_err(EnergyOptimalControlError::Experiment)
    .map_err(TotalKlTrustRegionError::Energy)?;
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
    let passive_delta: Vec<f64> = passive_left
        .iter()
        .zip(passive_right.iter())
        .map(|(left, right)| left - right)
        .collect();

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
        return Err(TotalKlTrustRegionError::InvalidProbeAmplitude);
    }

    let sensor_indices = selected_sensor_indices(scenario, selected_sensor_ids)?;
    let selected_covariance = principal_submatrix(&scenario.noise_covariance, &sensor_indices);
    let covariance_inverse = invert_spd(&selected_covariance)
        .map_err(EnergyOptimalControlError::SensorDesign)
        .map_err(TotalKlTrustRegionError::Energy)?;

    let sensor_count = selected_sensor_ids.len();
    let output_dimension = scenario
        .observation_times
        .len()
        .checked_mul(sensor_count)
        .ok_or(TotalKlTrustRegionError::ExcessiveControlBasis)?;
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
                    .map_err(|_| TotalKlTrustRegionError::ExcessiveControlBasis)?,
                label: "total_kl_piecewise_constant_basis_probe",
                segments,
            };
            let score = evaluate_input_profile(
                scenario,
                &basis_profile,
                selected_sensor_ids,
            )
            .map_err(EnergyOptimalControlError::Experiment)
            .map_err(TotalKlTrustRegionError::Energy)?;
            let left = prediction_vector(
                &score,
                left_model_id,
                scenario.observation_times.len(),
                sensor_count,
            )?;
            let right = prediction_vector(
                &score,
                right_model_id,
                scenario.observation_times.len(),
                sensor_count,
            )?;
            for output_index in 0..output_dimension {
                response_columns[coefficient_index][output_index] =
                    ((left[output_index] - right[output_index])
                        - passive_delta[output_index])
                        / probe_amplitude;
            }
        }
    }

    let mut gram = vec![vec![0.0; basis_dimension]; basis_dimension];
    let mut cross = vec![0.0; basis_dimension];
    for column in 0..basis_dimension {
        for time_index in 0..scenario.observation_times.len() {
            let offset = time_index * sensor_count;
            let response = &response_columns[column][offset..offset + sensor_count];
            let passive = &passive_delta[offset..offset + sensor_count];
            let weighted_passive = matrix_vector_product(&covariance_inverse, passive);
            cross[column] += dot(response, &weighted_passive);
        }
        if !cross[column].is_finite() {
            return Err(TotalKlTrustRegionError::NonFiniteCrossTerm);
        }
    }
    for left_column in 0..basis_dimension {
        for right_column in left_column..basis_dimension {
            let mut value = 0.0;
            for time_index in 0..scenario.observation_times.len() {
                let offset = time_index * sensor_count;
                let left = &response_columns[left_column][offset..offset + sensor_count];
                let right = &response_columns[right_column][offset..offset + sensor_count];
                let weighted_right = matrix_vector_product(&covariance_inverse, right);
                value += dot(left, &weighted_right);
            }
            if !value.is_finite() {
                return Err(TotalKlTrustRegionError::NonFiniteCrossTerm);
            }
            gram[left_column][right_column] = value;
            gram[right_column][left_column] = value;
        }
    }

    let mut passive_baseline_gaussian_kl = 0.0;
    for time_index in 0..scenario.observation_times.len() {
        let offset = time_index * sensor_count;
        let passive = &passive_delta[offset..offset + sensor_count];
        let weighted = matrix_vector_product(&covariance_inverse, passive);
        passive_baseline_gaussian_kl += 0.5 * dot(passive, &weighted);
    }
    if !passive_baseline_gaussian_kl.is_finite() {
        return Err(TotalKlTrustRegionError::NonFiniteOffset);
    }

    let mut whitened_gram = vec![vec![0.0; basis_dimension]; basis_dimension];
    let mut whitened_cross = vec![0.0; basis_dimension];
    for row in 0..basis_dimension {
        whitened_cross[row] = cross[row] / durations[row].sqrt();
        for column in 0..basis_dimension {
            whitened_gram[row][column] =
                gram[row][column] / (durations[row] * durations[column]).sqrt();
        }
    }
    if whitened_cross.iter().any(|value| !value.is_finite())
        || whitened_gram.iter().flatten().any(|value| !value.is_finite())
    {
        return Err(TotalKlTrustRegionError::NonFiniteCrossTerm);
    }

    let trust = maximize_psd_quadratic_on_ball(
        &whitened_gram,
        &whitened_cross,
        scenario.input_energy_budget.sqrt(),
    )
    .map_err(TotalKlTrustRegionError::TrustRegion)?;

    let mut amplitudes: Vec<f64> = trust
        .point
        .iter()
        .zip(durations.iter())
        .map(|(value, duration)| value / duration.sqrt())
        .collect();
    let raw_peak = amplitudes
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max);
    let peak_constraint_active =
        raw_peak > scenario.max_abs_forcing + TOTAL_KL_TOLERANCE;
    if peak_constraint_active && raw_peak > 0.0 {
        let scale = scenario.max_abs_forcing / raw_peak;
        for value in &mut amplitudes {
            *value *= scale;
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
        id: u32::MAX - 2,
        label: "total_kl_trust_region_optimal_control",
        segments: profile_segments,
    };
    let replay = evaluate_input_profile(scenario, &profile, selected_sensor_ids)
        .map_err(EnergyOptimalControlError::Experiment)
        .map_err(TotalKlTrustRegionError::Energy)?;
    let achieved_total_gaussian_kl =
        pairwise_gaussian_kl(&replay, left_model_id, right_model_id)?;
    let achieved_incremental_control_gaussian_kl =
        quadratic_objective(&gram, &amplitudes);
    let linear_objective = dot(&cross, &amplitudes);
    let predicted_total = passive_baseline_gaussian_kl
        + linear_objective
        + achieved_incremental_control_gaussian_kl;
    let replay_total_kl_abs_error = (predicted_total - achieved_total_gaussian_kl).abs();

    let (achieved_integrated_squared_input, achieved_peak_amplitude) =
        profile_cost(&profile);
    let unconstrained_total_gaussian_kl_upper_bound =
        passive_baseline_gaussian_kl + trust.objective_without_constant;
    let energy_feasibility_residual =
        (achieved_integrated_squared_input - scenario.input_energy_budget).max(0.0);
    let exactness_disposition = match trust.disposition {
        TrustRegionDisposition::FlatObjective => TotalKlOptimalityDisposition::NoControlEffect,
        TrustRegionDisposition::RegularBoundaryOptimum
            if peak_constraint_active =>
        {
            TotalKlOptimalityDisposition::PeakConstraintActive
        }
        TrustRegionDisposition::HardCaseBoundaryOptimum
            if peak_constraint_active =>
        {
            TotalKlOptimalityDisposition::PeakConstraintActive
        }
        TrustRegionDisposition::RegularBoundaryOptimum => {
            TotalKlOptimalityDisposition::ExactTotalKlTrustRegionOptimum
        }
        TrustRegionDisposition::HardCaseBoundaryOptimum => {
            TotalKlOptimalityDisposition::HardCaseTotalKlTrustRegionOptimum
        }
    };

    if !achieved_total_gaussian_kl.is_finite()
        || !achieved_incremental_control_gaussian_kl.is_finite()
        || !predicted_total.is_finite()
    {
        return Err(TotalKlTrustRegionError::NonFiniteReplay);
    }

    Ok(TotalKlTrustRegionReportV1 {
        left_model_id,
        right_model_id,
        selected_sensor_ids: selected_sensor_ids.to_vec(),
        actuator_nodes: ordered_actuators,
        time_boundaries: boundaries,
        coefficients,
        profile,
        optimality_disposition: exactness_disposition,
        basis_dimension,
        passive_baseline_gaussian_kl,
        achieved_incremental_control_gaussian_kl,
        achieved_total_gaussian_kl,
        unconstrained_total_gaussian_kl_upper_bound,
        trust_region_multiplier: trust.multiplier,
        largest_whitened_eigenvalue: trust.largest_eigenvalue,
        trust_region_stationarity_residual: trust.stationarity_residual,
        energy_feasibility_residual,
        trust_region_secular_residual: trust.secular_residual,
        replay_total_kl_abs_error,
        energy_budget: scenario.input_energy_budget,
        achieved_integrated_squared_input,
        peak_amplitude_limit: scenario.max_abs_forcing,
        achieved_peak_amplitude,
        peak_constraint_active,
        trust_region_iterations: trust.iterations,
        assumptions: assumptions(),
    })
}

fn assumptions() -> Vec<&'static str> {
    vec![
        "one fixed pair of linear graph models",
        "common state basis, capacities, initial state and horizon",
        "fixed sensors, sample schedule and common sensor covariance",
        "independent observation-noise blocks across observation times",
        "continuous control amplitudes over a declared piecewise-constant basis",
        "exact total controlled Gaussian KL objective, including passive offset and linear cross term",
        "integrated squared-input energy ellipsoid",
        "global trust-region optimum applies only when peak-amplitude cap is inactive",
        "fixed candidate parameters; nuisance profiling is not included",
        "no slew-rate or actuator-state constraint",
        "synthetic mathematical benchmark only",
    ]
}

fn selected_sensor_indices(
    scenario: &ExperimentDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<Vec<usize>, TotalKlTrustRegionError> {
    if selected_sensor_ids.is_empty() {
        return Err(TotalKlTrustRegionError::Energy(
            EnergyOptimalControlError::Experiment(ExcitationDesignError::EmptySensorSelection),
        ));
    }
    let mut seen = HashSet::new();
    let mut indices = Vec::with_capacity(selected_sensor_ids.len());
    for sensor_id in selected_sensor_ids {
        if !seen.insert(*sensor_id) {
            return Err(TotalKlTrustRegionError::Energy(
                EnergyOptimalControlError::Experiment(
                    ExcitationDesignError::DuplicateSelectedSensor(*sensor_id),
                ),
            ));
        }
        let index = scenario
            .sensors
            .iter()
            .position(|sensor| sensor.id == *sensor_id)
            .ok_or_else(|| TotalKlTrustRegionError::Energy(
                EnergyOptimalControlError::Experiment(
                    ExcitationDesignError::UnknownSensorId(*sensor_id),
                ),
            ))?;
        indices.push(index);
    }
    let total_cost: f64 = indices
        .iter()
        .map(|index| scenario.sensors[*index].cost)
        .sum();
    if total_cost > scenario.sensor_budget + TOTAL_KL_TOLERANCE {
        return Err(TotalKlTrustRegionError::Energy(
            EnergyOptimalControlError::Experiment(
                ExcitationDesignError::SensorBudgetExceeded,
            ),
        ));
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
) -> Result<Vec<f64>, TotalKlTrustRegionError> {
    let prediction = score
        .model_observation_means
        .iter()
        .find(|prediction| prediction.model_id == model_id)
        .ok_or(TotalKlTrustRegionError::MissingModelPrediction(model_id))?;
    if prediction.observations.len() != time_count
        || prediction
            .observations
            .iter()
            .any(|row| row.len() != sensor_count)
    {
        return Err(TotalKlTrustRegionError::InvalidPredictionShape);
    }
    if prediction.observations.iter().flatten().any(|value| !value.is_finite()) {
        return Err(TotalKlTrustRegionError::NonFiniteReplay);
    }
    Ok(prediction.observations.iter().flatten().copied().collect())
}

fn pairwise_gaussian_kl(
    score: &crate::experiment_design::ExperimentDesignScore,
    left_model_id: u32,
    right_model_id: u32,
) -> Result<f64, TotalKlTrustRegionError> {
    let value = score
        .pairwise_separations
        .iter()
        .find(|pair| {
            (pair.left_model_id == left_model_id && pair.right_model_id == right_model_id)
                || (pair.left_model_id == right_model_id
                    && pair.right_model_id == left_model_id)
        })
        .map(|pair| pair.gaussian_kl_separation)
        .ok_or(TotalKlTrustRegionError::NonFiniteReplay)?;
    if !value.is_finite() {
        return Err(TotalKlTrustRegionError::NonFiniteReplay);
    }
    Ok(value.max(0.0))
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn quadratic_objective(matrix: &[Vec<f64>], vector: &[f64]) -> f64 {
    0.5 * dot(vector, &matrix_vector_product(matrix, vector))
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

    fn network(edges: Vec<Edge>, initial: Vec<f64>) -> NetworkScenario {
        NetworkScenario {
            name: "total-kl-three-node-fixture",
            capacities: vec![1.0, 1.0, 1.0],
            initial,
            edges,
            horizon: 2.0,
        }
    }

    fn scenario(max_abs_forcing: f64) -> ExperimentDesignScenario {
        ExperimentDesignScenario {
            name: "total-kl-chain-bypass-fixture",
            models: vec![
                ModelCandidate {
                    id: 1,
                    label: "chain",
                    network: network(
                        vec![
                            Edge { left: 0, right: 1, conductance: 1.0 },
                            Edge { left: 1, right: 2, conductance: 1.0 },
                        ],
                        vec![0.2, 0.0, 0.0],
                    ),
                },
                ModelCandidate {
                    id: 2,
                    label: "chain-bypass",
                    network: network(
                        vec![
                            Edge { left: 0, right: 1, conductance: 1.0 },
                            Edge { left: 1, right: 2, conductance: 1.0 },
                            Edge { left: 0, right: 2, conductance: 0.4 },
                        ],
                        vec![0.2, 0.0, 0.0],
                    ),
                },
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

    #[test]
    fn total_kl_matches_independent_dense_scalar_trust_region_reference() {
        let fixture = scenario(2.0);
        let report = optimize_pairwise_total_kl_trust_region(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 2.0],
        )
        .unwrap();
        let passive_profile = InputProfile {
            id: 0,
            label: "independent-total-kl-passive",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 2.0,
                forcing: vec![0.0, 0.0, 0.0],
            }],
        };
        let passive = evaluate_input_profile(&fixture, &passive_profile, &[10, 11]).unwrap();
        let passive_kl = passive
            .pairwise_separations
            .iter()
            .find(|pair| pair.left_model_id == 1 && pair.right_model_id == 2)
            .unwrap()
            .gaussian_kl_separation;
        let boundary = (fixture.input_energy_budget / fixture.horizon()).sqrt();
        let positive = InputProfile {
            id: 90,
            label: "independent-total-kl-positive-boundary",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 2.0,
                forcing: vec![boundary, 0.0, 0.0],
            }],
        };
        let negative = InputProfile {
            id: 91,
            label: "independent-total-kl-negative-boundary",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 2.0,
                forcing: vec![-boundary, 0.0, 0.0],
            }],
        };
        let positive_score = evaluate_input_profile(&fixture, &positive, &[10, 11]).unwrap();
        let negative_score = evaluate_input_profile(&fixture, &negative, &[10, 11]).unwrap();
        let positive_kl = positive_score
            .pairwise_separations.iter()
            .find(|pair| pair.left_model_id == 1 && pair.right_model_id == 2)
            .unwrap().gaussian_kl_separation;
        let negative_kl = negative_score
            .pairwise_separations.iter()
            .find(|pair| pair.left_model_id == 1 && pair.right_model_id == 2)
            .unwrap().gaussian_kl_separation;
        let dense_scalar_best = positive_kl.max(negative_kl);

        assert!(report.achieved_total_gaussian_kl + 1.0e-9 >= dense_scalar_best);
        assert!((report.passive_baseline_gaussian_kl - passive_kl).abs() < 1.0e-9);
        assert!(
            (report.achieved_integrated_squared_input - fixture.input_energy_budget).abs()
                < 1.0e-8
        );
        assert!(report.trust_region_stationarity_residual < 1.0e-7);
        assert!(report.replay_total_kl_abs_error < 1.0e-8);
        assert_eq!(
            report.optimality_disposition,
            TotalKlOptimalityDisposition::ExactTotalKlTrustRegionOptimum
        );
    }

    #[test]
    fn total_kl_design_dominates_incremental_design_on_total_kl() {
        let fixture = scenario(2.0);
        let incremental = crate::energy_optimal_control::optimize_pairwise_energy_ball_control(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0, 1, 2],
            &[0.0, 0.5, 1.0, 2.0],
        )
        .unwrap();
        let total = optimize_pairwise_total_kl_trust_region(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0, 1, 2],
            &[0.0, 0.5, 1.0, 2.0],
        )
        .unwrap();
        assert!(
            total.achieved_total_gaussian_kl + 1.0e-8
                >= incremental.achieved_total_gaussian_kl
        );
        assert!(
            total.unconstrained_total_gaussian_kl_upper_bound + 1.0e-8
                >= total.achieved_total_gaussian_kl
        );
        assert!(!total.peak_constraint_active);
        assert!(total.replay_total_kl_abs_error < 1.0e-8);
    }

    #[test]
    fn passive_identical_models_produce_no_control_effect_and_zero_waveform() {
        let mut fixture = scenario(2.0);
        fixture.models[1].network = fixture.models[0].network.clone();
        let report = optimize_pairwise_total_kl_trust_region(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 2.0],
        )
        .unwrap();
        assert_eq!(report.optimality_disposition, TotalKlOptimalityDisposition::NoControlEffect);
        assert_eq!(report.achieved_total_gaussian_kl, 0.0);
        assert_eq!(report.achieved_integrated_squared_input, 0.0);
    }

    #[test]
    fn active_peak_constraint_is_not_reported_as_a_global_optimum() {
        let report = optimize_pairwise_total_kl_trust_region(
            &scenario(0.05),
            1,
            2,
            &[10, 11],
            &[0],
            &[0.0, 2.0],
        )
        .unwrap();
        assert_eq!(
            report.optimality_disposition,
            TotalKlOptimalityDisposition::PeakConstraintActive
        );
        assert!(report.peak_constraint_active);
        assert!(report.achieved_peak_amplitude <= 0.05 + TOTAL_KL_TOLERANCE);
        assert!(report.achieved_integrated_squared_input <= report.energy_budget + 1.0e-9);
        assert!(report.unconstrained_total_gaussian_kl_upper_bound + 1.0e-9
            >= report.achieved_total_gaussian_kl);
    }
}
