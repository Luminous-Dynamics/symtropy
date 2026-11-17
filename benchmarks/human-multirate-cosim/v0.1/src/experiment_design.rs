//! Active input design and model-discrimination reference for synthetic graph systems.
//!
//! This module compares candidate graph models under a shared bounded forcing
//! waveform and selected sensor set. It uses exact spectral response under
//! piecewise-constant inputs and covariance-aware pairwise Gaussian separation.
//! No human data, physiological units, model priors, or claims about nature are
//! used. The evaluator retains the candidate model responses and model identity.

use crate::network::{NetworkError, NetworkScenario};
use crate::sensor_design::{
    SensorCandidate, SensorDesignError, invert_spd, principal_submatrix, validate_spd,
};
use std::collections::{HashMap, HashSet};
use std::fmt;

const MAX_MODEL_CANDIDATES: usize = 8;
const MAX_SENSOR_CANDIDATES: usize = 16;
const FEASIBILITY_TOLERANCE: f64 = 1.0e-12;
const INDISTINGUISHABLE_TOLERANCE: f64 = 1.0e-12;

/// One alternative mathematical model for the same abstract state vector.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelCandidate {
    pub id: u32,
    pub label: &'static str,
    pub network: NetworkScenario,
}

/// One piecewise-constant external forcing interval.
///
/// `forcing` is the right-hand-side vector in
/// `C * dx/dt = -L*x + forcing`. It is not a state derivative.
#[derive(Debug, Clone, PartialEq)]
pub struct ForcingSegment {
    pub start_time: f64,
    pub end_time: f64,
    pub forcing: Vec<f64>,
}

/// A complete input waveform over the model horizon.
#[derive(Debug, Clone, PartialEq)]
pub struct InputProfile {
    pub id: u32,
    pub label: &'static str,
    pub segments: Vec<ForcingSegment>,
}

/// Frozen model-discrimination experiment. Sensor covariance rows and columns
/// follow the full sensor candidate vector's order.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentDesignScenario {
    pub name: &'static str,
    pub models: Vec<ModelCandidate>,
    pub sensors: Vec<SensorCandidate>,
    pub noise_covariance: Vec<Vec<f64>>,
    pub observation_times: Vec<f64>,
    pub sensor_budget: f64,
    pub max_abs_forcing: f64,
    /// Integral of squared forcing over time in arbitrary, consistent units.
    pub input_energy_budget: f64,
}

impl ExperimentDesignScenario {
    pub fn validate(&self) -> Result<(), ExcitationDesignError> {
        if self.models.len() < 2 || self.models.len() > MAX_MODEL_CANDIDATES {
            return Err(ExcitationDesignError::InvalidModelCount);
        }
        if self.sensors.is_empty() || self.sensors.len() > MAX_SENSOR_CANDIDATES {
            return Err(ExcitationDesignError::InvalidSensorCount);
        }
        if !self.sensor_budget.is_finite()
            || self.sensor_budget <= 0.0
            || !self.max_abs_forcing.is_finite()
            || self.max_abs_forcing <= 0.0
            || !self.input_energy_budget.is_finite()
            || self.input_energy_budget <= 0.0
        {
            return Err(ExcitationDesignError::InvalidBudget);
        }

        let reference = &self.models[0].network;
        reference
            .validate()
            .map_err(ExcitationDesignError::Network)?;
        let dimension = reference.capacities.len();
        let horizon = reference.horizon;

        let mut model_ids = HashSet::new();
        for model in &self.models {
            if !model_ids.insert(model.id) {
                return Err(ExcitationDesignError::DuplicateModelId(model.id));
            }
            model
                .network
                .validate()
                .map_err(ExcitationDesignError::Network)?;
            if model.network.capacities.len() != dimension
                || model.network.capacities != reference.capacities
                || model.network.initial != reference.initial
                || model.network.horizon != horizon
            {
                return Err(ExcitationDesignError::IncompatibleModelStateBasis);
            }
        }

        let mut sensor_ids = HashSet::new();
        for (index, sensor) in self.sensors.iter().enumerate() {
            if !sensor_ids.insert(sensor.id) {
                return Err(ExcitationDesignError::DuplicateSensorId(sensor.id));
            }
            if sensor.node >= dimension {
                return Err(ExcitationDesignError::UnknownSensorNode { sensor: index });
            }
            if !sensor.cost.is_finite() || sensor.cost <= 0.0 {
                return Err(ExcitationDesignError::InvalidSensorCost { sensor: index });
            }
        }

        if self.noise_covariance.len() != self.sensors.len()
            || self
                .noise_covariance
                .iter()
                .any(|row| row.len() != self.sensors.len())
        {
            return Err(ExcitationDesignError::CovarianceShapeMismatch);
        }
        validate_spd(&self.noise_covariance)
            .map_err(ExcitationDesignError::SensorCovariance)?;

        if self.observation_times.is_empty() {
            return Err(ExcitationDesignError::MissingObservationTimes);
        }
        let mut previous_time = -1.0;
        for time in &self.observation_times {
            if !time.is_finite()
                || *time < 0.0
                || *time <= previous_time
                || *time > horizon
            {
                return Err(ExcitationDesignError::InvalidObservationTime);
            }
            previous_time = *time;
        }
        Ok(())
    }

    pub fn horizon(&self) -> f64 {
        self.models[0].network.horizon
    }
}

/// Covariance-aware pairwise model separation. The score is a sum over
/// observation times, assuming independent noise blocks across those times.
#[derive(Debug, Clone, PartialEq)]
pub struct PairwiseModelSeparation {
    pub left_model_id: u32,
    pub right_model_id: u32,
    /// One half of the accumulated squared Mahalanobis distance.
    pub t_optimal_separation: f64,
}

/// Predicted sensor means for one candidate model, retained for the evaluator.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelObservationMeans {
    pub model_id: u32,
    /// Outer index is observation time; inner index follows selected sensor IDs.
    pub observations: Vec<Vec<f64>>,
}

/// Score vector for one input profile and one selected sensor set.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentDesignScore {
    pub profile_id: u32,
    pub profile_label: &'static str,
    pub selected_sensor_ids: Vec<u32>,
    pub sensor_cost: f64,
    pub maximum_abs_forcing: f64,
    pub integrated_squared_input: f64,
    pub pairwise_separations: Vec<PairwiseModelSeparation>,
    pub minimum_pairwise_separation: f64,
    pub mean_pairwise_separation: f64,
    pub worst_case_indistinguishable: bool,
    pub model_observation_means: Vec<ModelObservationMeans>,
}

/// Scores for a frozen set of candidate input profiles.
#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentDesignTournament {
    pub scores: Vec<ExperimentDesignScore>,
    pub best_worst_case: Option<ExperimentDesignScore>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExcitationDesignError {
    Network(NetworkError),
    SensorCovariance(SensorDesignError),
    InvalidModelCount,
    InvalidSensorCount,
    InvalidBudget,
    DuplicateModelId(u32),
    DuplicateSensorId(u32),
    DuplicateProfileId(u32),
    IncompatibleModelStateBasis,
    UnknownSensorNode { sensor: usize },
    InvalidSensorCost { sensor: usize },
    CovarianceShapeMismatch,
    MissingObservationTimes,
    InvalidObservationTime,
    InvalidProfileSegmentCount,
    InvalidProfileTime,
    ProfileHasGapOrOverlap,
    ForcingDimensionMismatch { segment: usize },
    NonFiniteForcing { segment: usize },
    ForcingAmplitudeExceeded,
    InputEnergyExceeded,
    UnknownSensorId(u32),
    EmptySensorSelection,
    DuplicateSelectedSensor(u32),
    SensorBudgetExceeded,
    NoProfiles,
}

impl fmt::Display for ExcitationDesignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(error) => write!(f, "network model invalid: {error}"),
            Self::SensorCovariance(error) => write!(f, "sensor covariance invalid: {error}"),
            Self::InvalidModelCount => {
                write!(f, "model candidate count must be between two and eight")
            }
            Self::InvalidSensorCount => {
                write!(f, "sensor candidate count must be between one and sixteen")
            }
            Self::InvalidBudget => {
                write!(f, "sensor and input budgets must be finite and positive")
            }
            Self::DuplicateModelId(id) => write!(f, "duplicate model candidate ID {id}"),
            Self::DuplicateSensorId(id) => write!(f, "duplicate sensor candidate ID {id}"),
            Self::DuplicateProfileId(id) => write!(f, "duplicate input profile ID {id}"),
            Self::IncompatibleModelStateBasis => {
                write!(f, "candidate models must share capacities, initial state and horizon")
            }
            Self::UnknownSensorNode { sensor } => {
                write!(f, "sensor candidate {sensor} references an unknown graph node")
            }
            Self::InvalidSensorCost { sensor } => {
                write!(f, "sensor candidate {sensor} has an invalid cost")
            }
            Self::CovarianceShapeMismatch => {
                write!(f, "noise covariance shape must match sensor candidate count")
            }
            Self::MissingObservationTimes => write!(f, "at least one observation time is required"),
            Self::InvalidObservationTime => {
                write!(f, "observation times must be finite, increasing and within the horizon")
            }
            Self::InvalidProfileSegmentCount => {
                write!(f, "an input profile must contain at least one segment")
            }
            Self::InvalidProfileTime => {
                write!(f, "input segment times must be finite and have positive duration")
            }
            Self::ProfileHasGapOrOverlap => {
                write!(f, "input segments must cover the horizon exactly without gaps or overlaps")
            }
            Self::ForcingDimensionMismatch { segment } => {
                write!(f, "forcing vector in segment {segment} has the wrong dimension")
            }
            Self::NonFiniteForcing { segment } => {
                write!(f, "forcing vector in segment {segment} contains a non-finite value")
            }
            Self::ForcingAmplitudeExceeded => {
                write!(f, "input profile exceeds the declared maximum forcing amplitude")
            }
            Self::InputEnergyExceeded => {
                write!(f, "input profile exceeds the declared integrated squared-input budget")
            }
            Self::UnknownSensorId(id) => write!(f, "unknown selected sensor ID {id}"),
            Self::EmptySensorSelection => write!(f, "at least one sensor must be selected"),
            Self::DuplicateSelectedSensor(id) => {
                write!(f, "sensor ID {id} was selected more than once")
            }
            Self::SensorBudgetExceeded => write!(f, "selected sensors exceed the declared budget"),
            Self::NoProfiles => write!(f, "at least one input profile is required"),
        }
    }
}

impl std::error::Error for ExcitationDesignError {}

/// Evaluate one bounded waveform and sensor subset against every candidate model.
pub fn evaluate_input_profile(
    scenario: &ExperimentDesignScenario,
    profile: &InputProfile,
    selected_sensor_ids: &[u32],
) -> Result<ExperimentDesignScore, ExcitationDesignError> {
    scenario.validate()?;
    let selected_indices = resolve_sensor_indices(scenario, selected_sensor_ids)?;
    let selected_ids: Vec<u32> = selected_indices
        .iter()
        .map(|index| scenario.sensors[*index].id)
        .collect();
    let (maximum_abs_forcing, integrated_squared_input) =
        validate_input_profile(scenario, profile)?;

    let covariance = principal_submatrix(&scenario.noise_covariance, &selected_indices);
    let covariance_inverse =
        invert_spd(&covariance).map_err(ExcitationDesignError::SensorCovariance)?;

    let mut model_means = Vec::with_capacity(scenario.models.len());
    for model in &scenario.models {
        let oracle = model
            .network
            .spectral_oracle()
            .map_err(ExcitationDesignError::Network)?;
        let mut observations = Vec::with_capacity(scenario.observation_times.len());
        for time in &scenario.observation_times {
            let state = forced_state_at(&oracle, &model.network.initial, profile, *time)?;
            let observed: Vec<f64> = selected_indices
                .iter()
                .map(|index| state[scenario.sensors[*index].node])
                .collect();
            observations.push(observed);
        }
        model_means.push(ModelObservationMeans {
            model_id: model.id,
            observations,
        });
    }

    let mut pairwise_separations = Vec::new();
    for left in 0..model_means.len() {
        for right in (left + 1)..model_means.len() {
            let mut separation = 0.0;
            for time_index in 0..scenario.observation_times.len() {
                let left_values = &model_means[left].observations[time_index];
                let right_values = &model_means[right].observations[time_index];
                let difference: Vec<f64> = left_values
                    .iter()
                    .zip(right_values.iter())
                    .map(|(left_value, right_value)| left_value - right_value)
                    .collect();
                let weighted = matrix_vector_product(&covariance_inverse, &difference);
                separation += 0.5
                    * difference
                        .iter()
                        .zip(weighted.iter())
                        .map(|(value, weighted_value)| value * weighted_value)
                        .sum::<f64>();
            }
            pairwise_separations.push(PairwiseModelSeparation {
                left_model_id: model_means[left].model_id,
                right_model_id: model_means[right].model_id,
                t_optimal_separation: separation.max(0.0),
            });
        }
    }

    let minimum_pairwise_separation = pairwise_separations
        .iter()
        .map(|pair| pair.t_optimal_separation)
        .fold(f64::INFINITY, f64::min);
    let mean_pairwise_separation = pairwise_separations
        .iter()
        .map(|pair| pair.t_optimal_separation)
        .sum::<f64>()
        / pairwise_separations.len() as f64;
    let sensor_cost: f64 = selected_indices
        .iter()
        .map(|index| scenario.sensors[*index].cost)
        .sum();

    Ok(ExperimentDesignScore {
        profile_id: profile.id,
        profile_label: profile.label,
        selected_sensor_ids: selected_ids,
        sensor_cost,
        maximum_abs_forcing,
        integrated_squared_input,
        pairwise_separations,
        minimum_pairwise_separation,
        mean_pairwise_separation,
        worst_case_indistinguishable: minimum_pairwise_separation
            <= INDISTINGUISHABLE_TOLERANCE,
        model_observation_means: model_means,
    })
}

/// Evaluate a deterministic input-profile catalogue. The best profile
/// maximizes the minimum pairwise model separation; exact ties prefer lower
/// input energy, then lower peak amplitude, then smaller profile ID.
pub fn evaluate_input_tournament(
    scenario: &ExperimentDesignScenario,
    profiles: &[InputProfile],
    selected_sensor_ids: &[u32],
) -> Result<ExperimentDesignTournament, ExcitationDesignError> {
    if profiles.is_empty() {
        return Err(ExcitationDesignError::NoProfiles);
    }
    let mut profile_ids = HashSet::new();
    for profile in profiles {
        if !profile_ids.insert(profile.id) {
            return Err(ExcitationDesignError::DuplicateProfileId(profile.id));
        }
    }

    let mut scores = Vec::with_capacity(profiles.len());
    for profile in profiles {
        scores.push(evaluate_input_profile(
            scenario,
            profile,
            selected_sensor_ids,
        )?);
    }

    let mut best: Option<ExperimentDesignScore> = None;
    for score in &scores {
        let replace = best.as_ref().is_none_or(|current| {
            score.minimum_pairwise_separation
                > current.minimum_pairwise_separation + FEASIBILITY_TOLERANCE
                || ((score.minimum_pairwise_separation
                    - current.minimum_pairwise_separation)
                    .abs()
                    <= FEASIBILITY_TOLERANCE
                    && (score.integrated_squared_input
                        < current.integrated_squared_input - FEASIBILITY_TOLERANCE
                        || ((score.integrated_squared_input - current.integrated_squared_input)
                            .abs()
                            <= FEASIBILITY_TOLERANCE
                            && (score.maximum_abs_forcing
                                < current.maximum_abs_forcing - FEASIBILITY_TOLERANCE
                                || ((score.maximum_abs_forcing - current.maximum_abs_forcing).abs()
                                    <= FEASIBILITY_TOLERANCE
                                    && score.profile_id < current.profile_id)))))
        });
        if replace {
            best = Some(score.clone());
        }
    }

    Ok(ExperimentDesignTournament { scores, best_worst_case: best })
}

fn validate_input_profile(
    scenario: &ExperimentDesignScenario,
    profile: &InputProfile,
) -> Result<(f64, f64), ExcitationDesignError> {
    if profile.segments.is_empty() {
        return Err(ExcitationDesignError::InvalidProfileSegmentCount);
    }
    let horizon = scenario.horizon();
    let dimension = scenario.models[0].network.capacities.len();
    let mut expected_start = 0.0;
    let mut maximum_abs_forcing: f64 = 0.0;
    let mut integrated_squared_input = 0.0;

    for (index, segment) in profile.segments.iter().enumerate() {
        if !segment.start_time.is_finite()
            || !segment.end_time.is_finite()
            || segment.end_time <= segment.start_time
        {
            return Err(ExcitationDesignError::InvalidProfileTime);
        }
        if (segment.start_time - expected_start).abs() > FEASIBILITY_TOLERANCE {
            return Err(ExcitationDesignError::ProfileHasGapOrOverlap);
        }
        if segment.end_time > horizon + FEASIBILITY_TOLERANCE {
            return Err(ExcitationDesignError::ProfileHasGapOrOverlap);
        }
        if segment.forcing.len() != dimension {
            return Err(ExcitationDesignError::ForcingDimensionMismatch { segment: index });
        }
        if segment.forcing.iter().any(|value| !value.is_finite()) {
            return Err(ExcitationDesignError::NonFiniteForcing { segment: index });
        }
        for value in &segment.forcing {
            maximum_abs_forcing = maximum_abs_forcing.max(value.abs());
            integrated_squared_input +=
                (segment.end_time - segment.start_time) * value * value;
        }
        expected_start = segment.end_time;
    }

    if (expected_start - horizon).abs() > FEASIBILITY_TOLERANCE {
        return Err(ExcitationDesignError::ProfileHasGapOrOverlap);
    }
    if maximum_abs_forcing > scenario.max_abs_forcing + FEASIBILITY_TOLERANCE {
        return Err(ExcitationDesignError::ForcingAmplitudeExceeded);
    }
    if integrated_squared_input > scenario.input_energy_budget + FEASIBILITY_TOLERANCE {
        return Err(ExcitationDesignError::InputEnergyExceeded);
    }
    Ok((maximum_abs_forcing, integrated_squared_input))
}

fn resolve_sensor_indices(
    scenario: &ExperimentDesignScenario,
    selected_sensor_ids: &[u32],
) -> Result<Vec<usize>, ExcitationDesignError> {
    if selected_sensor_ids.is_empty() {
        return Err(ExcitationDesignError::EmptySensorSelection);
    }
    let mut seen = HashSet::new();
    let lookup: HashMap<u32, usize> = scenario
        .sensors
        .iter()
        .enumerate()
        .map(|(index, sensor)| (sensor.id, index))
        .collect();
    let mut indices = Vec::with_capacity(selected_sensor_ids.len());
    for id in selected_sensor_ids {
        if !seen.insert(*id) {
            return Err(ExcitationDesignError::DuplicateSelectedSensor(*id));
        }
        let index = lookup
            .get(id)
            .copied()
            .ok_or(ExcitationDesignError::UnknownSensorId(*id))?;
        indices.push(index);
    }
    indices.sort_unstable();
    let sensor_cost: f64 = indices
        .iter()
        .map(|index| scenario.sensors[*index].cost)
        .sum();
    if sensor_cost > scenario.sensor_budget + FEASIBILITY_TOLERANCE {
        return Err(ExcitationDesignError::SensorBudgetExceeded);
    }
    Ok(indices)
}

fn forced_state_at(
    oracle: &crate::network::SpectralOracle,
    initial_state: &[f64],
    profile: &InputProfile,
    time: f64,
) -> Result<Vec<f64>, ExcitationDesignError> {
    let mut state = initial_state.to_vec();
    for segment in &profile.segments {
        if time <= segment.start_time {
            break;
        }
        let elapsed = time.min(segment.end_time) - segment.start_time;
        if elapsed > 0.0 {
            state = oracle
                .state_after_constant_forcing(&state, &segment.forcing, elapsed)
                .map_err(ExcitationDesignError::Network)?;
        }
        if time <= segment.end_time {
            break;
        }
    }
    Ok(state)
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::Edge;

    fn model(id: u32, label: &'static str, bypass: bool) -> ModelCandidate {
        let mut edges = vec![
            Edge { left: 0, right: 1, conductance: 1.0 },
            Edge { left: 1, right: 2, conductance: 1.0 },
        ];
        if bypass {
            edges.push(Edge { left: 0, right: 2, conductance: 0.5 });
        }
        ModelCandidate {
            id,
            label,
            network: NetworkScenario {
                name: label,
                capacities: vec![1.0, 1.0, 1.0],
                initial: vec![0.0, 0.0, 0.0],
                edges,
                horizon: 2.0,
            },
        }
    }

    fn scenario() -> ExperimentDesignScenario {
        ExperimentDesignScenario {
            name: "chain_vs_bypass_active_excitation",
            models: vec![
                model(1, "chain", false),
                model(2, "chain_with_bypass", true),
            ],
            sensors: vec![SensorCandidate {
                id: 30,
                node: 2,
                cost: 1.0,
                common_cause_group: None,
            }],
            noise_covariance: vec![vec![0.01]],
            observation_times: vec![0.25, 0.5, 1.0, 2.0],
            sensor_budget: 1.0,
            max_abs_forcing: 1.0,
            input_energy_budget: 0.5,
        }
    }

    fn passive_profile() -> InputProfile {
        InputProfile {
            id: 1,
            label: "passive_zero_input",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 2.0,
                forcing: vec![0.0, 0.0, 0.0],
            }],
        }
    }

    fn pulse_profile() -> InputProfile {
        InputProfile {
            id: 2,
            label: "bounded_node_zero_pulse",
            segments: vec![
                ForcingSegment {
                    start_time: 0.0,
                    end_time: 0.5,
                    forcing: vec![1.0, 0.0, 0.0],
                },
                ForcingSegment {
                    start_time: 0.5,
                    end_time: 2.0,
                    forcing: vec![0.0, 0.0, 0.0],
                },
            ],
        }
    }

    #[test]
    fn passive_observation_does_not_discriminate_equivalent_zero_state_models() {
        let score = evaluate_input_profile(&scenario(), &passive_profile(), &[30]).unwrap();
        assert_eq!(score.minimum_pairwise_separation, 0.0);
        assert!(score.worst_case_indistinguishable);
    }

    #[test]
    fn bounded_pulse_discriminates_chain_from_bypass_model() {
        let score = evaluate_input_profile(&scenario(), &pulse_profile(), &[30]).unwrap();
        assert!(score.minimum_pairwise_separation > 1.0e-6);
        assert!(!score.worst_case_indistinguishable);
        assert!((score.integrated_squared_input - 0.5).abs() < 1.0e-12);
        assert_eq!(score.maximum_abs_forcing, 1.0);
    }

    #[test]
    fn profile_tournament_prefers_pulse_to_passive_observation() {
        let tournament = evaluate_input_tournament(
            &scenario(),
            &[passive_profile(), pulse_profile()],
            &[30],
        )
        .unwrap();
        assert_eq!(tournament.best_worst_case.unwrap().profile_id, 2);
    }

    #[test]
    fn over_amplitude_and_energy_profiles_reject_before_evaluation() {
        let mut high_amplitude = pulse_profile();
        high_amplitude.segments[0].forcing[0] = 1.1;
        assert_eq!(
            evaluate_input_profile(&scenario(), &high_amplitude, &[30]),
            Err(ExcitationDesignError::ForcingAmplitudeExceeded)
        );

        let mut high_energy = pulse_profile();
        high_energy.segments[0].end_time = 1.0;
        high_energy.segments[1].start_time = 1.0;
        assert_eq!(
            evaluate_input_profile(&scenario(), &high_energy, &[30]),
            Err(ExcitationDesignError::InputEnergyExceeded)
        );
    }

    #[test]
    fn input_profile_gaps_reject() {
        let mut gap = pulse_profile();
        gap.segments[1].start_time = 0.6;
        assert_eq!(
            evaluate_input_profile(&scenario(), &gap, &[30]),
            Err(ExcitationDesignError::ProfileHasGapOrOverlap)
        );
    }

    #[test]
    fn sensor_budget_and_covariance_validation_remain_fail_closed() {
        let mut problem = scenario();
        problem.sensor_budget = 0.5;
        assert_eq!(
            evaluate_input_profile(&problem, &pulse_profile(), &[30]),
            Err(ExcitationDesignError::SensorBudgetExceeded)
        );
        let mut invalid = scenario();
        invalid.noise_covariance = vec![vec![0.0]];
        assert!(matches!(
            evaluate_input_profile(&invalid, &pulse_profile(), &[30]),
            Err(ExcitationDesignError::SensorCovariance(
                SensorDesignError::CovarianceNotPositiveDefinite
            ))
        ));
    }
}
