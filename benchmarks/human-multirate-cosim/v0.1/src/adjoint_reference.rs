//! Independent sampled-observation adjoint for fixed linear graph models.
//!
//! The adjoint is derived from C dx/dt = -L x + u(t). It handles discrete
//! observation-time costs as jumps in the adjoint and integrates B^T p exactly
//! over each piecewise-constant control segment using the spectral graph oracle.

use crate::experiment_design::{
    evaluate_input_profile, ExperimentDesignScenario, ExcitationDesignError, InputProfile,
};
use crate::network::NetworkError;
use crate::noise_robustness::{CovarianceError, ObservationCovarianceV1};
use crate::total_kl_trust_region::ObservationDesignIdentityV1;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct AdjointGradientReportV1 {
    pub left_model_id: u32,
    pub right_model_id: u32,
    pub selected_sensor_ids: Vec<u32>,
    pub actuator_nodes: Vec<usize>,
    /// Segment-major, then actuator-node order.
    pub control_gradient: Vec<f64>,
    pub total_gaussian_kl: f64,
    pub gradient_norm: f64,
    /// Residual of gradient - lambda * W * u for the supplied energy multiplier.
    pub energy_ball_stationarity_residual: f64,
    pub covariance_kind: &'static str,
    pub covariance_id: String,
    pub observation_process_id: String,
    pub sampling_policy_id: String,
    pub observation_operator_id: String,
    pub availability_cutoff_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdjointReferenceError {
    Experiment(ExcitationDesignError),
    Network(NetworkError),
    Covariance(CovarianceError),
    MissingModelPrediction(u32),
    InvalidPredictionShape,
    InvalidIdentity,
    IdentityMismatch,
    InvalidActuatorSet,
    NonFiniteAdjoint,
    NonFiniteObjective,
}

impl fmt::Display for AdjointReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Experiment(error) => write!(f, "adjoint experiment rejected: {error}"),
            Self::Network(error) => write!(f, "adjoint graph oracle failed: {error}"),
            Self::Covariance(error) => write!(f, "adjoint covariance rejected: {error}"),
            Self::MissingModelPrediction(id) => {
                write!(f, "adjoint is missing model prediction {id}")
            }
            Self::InvalidPredictionShape => {
                write!(f, "adjoint predictions do not match the bound output dimensions")
            }
            Self::InvalidIdentity => {
                write!(f, "adjoint expected observation identity is incomplete")
            }
            Self::IdentityMismatch => {
                write!(f, "adjoint covariance identity, sensor order or times do not match request")
            }
            Self::InvalidActuatorSet => {
                write!(f, "adjoint actuator nodes must be unique and inside the graph")
            }
            Self::NonFiniteAdjoint => write!(f, "adjoint contains non-finite values"),
            Self::NonFiniteObjective => write!(f, "adjoint total-KL objective is non-finite"),
        }
    }
}

impl std::error::Error for AdjointReferenceError {}

/// Compute the exact gradient of total controlled Gaussian KL with respect to
/// each piecewise-constant actuator coefficient.
///
/// Observation contributions are injected as adjoint jumps H^T Sigma^-1 d at
/// each sample time. Between samples the adjoint propagates backward under
/// the transpose graph dynamics. The integrated control gradient is evaluated
/// using the same exact spectral oracle, not a time-stepping approximation.
///
/// The covariance and identity binding must have the same sensor order and
/// sample schedule as the request. This function is for fixed linear graph
/// models in the synthetic benchmark; it is not a general nonlinear adjoint.
pub fn total_kl_control_gradient_adjoint(
    scenario: &ExperimentDesignScenario,
    profile: &InputProfile,
    left_model_id: u32,
    right_model_id: u32,
    selected_sensor_ids: &[u32],
    actuator_nodes: &[usize],
    covariance: &ObservationCovarianceV1,
    expected_identity: &ObservationDesignIdentityV1,
    trust_region_multiplier: Option<f64>,
) -> Result<AdjointGradientReportV1, AdjointReferenceError> {
    scenario.validate().map_err(AdjointReferenceError::Experiment)?;
    if left_model_id == right_model_id {
        return Err(AdjointReferenceError::MissingModelPrediction(left_model_id));
    }
    if expected_identity.covariance_id.trim().is_empty()
        || expected_identity.observation_process_id.trim().is_empty()
        || expected_identity.sampling_policy_id.trim().is_empty()
        || expected_identity.observation_operator_id.trim().is_empty()
        || expected_identity.availability_cutoff_id.trim().is_empty()
    {
        return Err(AdjointReferenceError::InvalidIdentity);
    }
    if expected_identity.covariance_id != covariance.covariance_id
        || expected_identity.observation_process_id != covariance.observation_process_id
        || expected_identity.sampling_policy_id != covariance.sampling_policy_id
        || expected_identity.observation_operator_id != covariance.observation_operator_id
        || expected_identity.availability_cutoff_id != covariance.availability_cutoff_id
        || covariance.sensor_ids != selected_sensor_ids
        || covariance.times != scenario.observation_times
    {
        return Err(AdjointReferenceError::IdentityMismatch);
    }

    if actuator_nodes.is_empty() {
        return Err(AdjointReferenceError::InvalidActuatorSet);
    }
    let graph_dimension = scenario.models[0].network.capacities.len();
    let mut sorted_actuators = actuator_nodes.to_vec();
    sorted_actuators.sort_unstable();
    if sorted_actuators.iter().any(|node| *node >= graph_dimension)
        || sorted_actuators.windows(2).any(|pair| pair[0] == pair[1])
    {
        return Err(AdjointReferenceError::InvalidActuatorSet);
    }

    let score = evaluate_input_profile(scenario, profile, selected_sensor_ids)
        .map_err(AdjointReferenceError::Experiment)?;
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
    let difference: Vec<f64> = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| left_value - right_value)
        .collect();
    let precision_difference = covariance
        .gaussian_precision_apply(&difference)
        .map_err(AdjointReferenceError::Covariance)?;
    let total_gaussian_kl = 0.5
        * difference
            .iter()
            .zip(precision_difference.iter())
            .map(|(left_value, right_value)| left_value * right_value)
            .sum::<f64>();
    if !total_gaussian_kl.is_finite() {
        return Err(AdjointReferenceError::NonFiniteObjective);
    }

    let sensor_nodes = selected_sensor_ids
        .iter()
        .map(|sensor_id| {
            scenario
                .sensors
                .iter()
                .find(|sensor| sensor.id == *sensor_id)
                .map(|sensor| sensor.node)
                .ok_or(AdjointReferenceError::InvalidPredictionShape)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut control_gradient =
        vec![0.0; profile.segments.len() * sorted_actuators.len()];
    accumulate_model_adjoint_gradient(
        scenario,
        left_model_id,
        1.0,
        &sensor_nodes,
        &precision_difference,
        profile,
        &sorted_actuators,
        &mut control_gradient,
    )?;
    accumulate_model_adjoint_gradient(
        scenario,
        right_model_id,
        -1.0,
        &sensor_nodes,
        &precision_difference,
        profile,
        &sorted_actuators,
        &mut control_gradient,
    )?;
    if control_gradient.iter().any(|value| !value.is_finite()) {
        return Err(AdjointReferenceError::NonFiniteAdjoint);
    }

    let mut energy_ball_stationarity_residual = 0.0;
    if let Some(multiplier) = trust_region_multiplier {
        if !multiplier.is_finite() || multiplier < 0.0 {
            return Err(AdjointReferenceError::NonFiniteAdjoint);
        }
        let mut squared_residual = 0.0;
        for (segment_index, segment) in profile.segments.iter().enumerate() {
            let duration = segment.end_time - segment.start_time;
            for (actuator_index, node) in sorted_actuators.iter().enumerate() {
                let coefficient_index =
                    segment_index * sorted_actuators.len() + actuator_index;
                let amplitude = segment.forcing[*node];
                let residual = control_gradient[coefficient_index]
                    - multiplier * duration * amplitude;
                squared_residual += residual * residual;
            }
        }
        energy_ball_stationarity_residual = squared_residual.sqrt();
    }

    Ok(AdjointGradientReportV1 {
        left_model_id,
        right_model_id,
        selected_sensor_ids: selected_sensor_ids.to_vec(),
        actuator_nodes: sorted_actuators,
        gradient_norm: vector_norm(&control_gradient),
        control_gradient,
        total_gaussian_kl,
        energy_ball_stationarity_residual,
        covariance_kind: covariance.covariance().kind_label(),
        covariance_id: expected_identity.covariance_id.clone(),
        observation_process_id: expected_identity.observation_process_id.clone(),
        sampling_policy_id: expected_identity.sampling_policy_id.clone(),
        observation_operator_id: expected_identity.observation_operator_id.clone(),
        availability_cutoff_id: expected_identity.availability_cutoff_id.clone(),
    })
}

fn accumulate_model_adjoint_gradient(
    scenario: &ExperimentDesignScenario,
    model_id: u32,
    objective_sign: f64,
    sensor_nodes: &[usize],
    precision_difference: &[f64],
    profile: &InputProfile,
    actuator_nodes: &[usize],
    gradient: &mut [f64],
) -> Result<(), AdjointReferenceError> {
    let model = scenario
        .models
        .iter()
        .find(|model| model.id == model_id)
        .ok_or(AdjointReferenceError::MissingModelPrediction(model_id))?;
    let oracle = model
        .network
        .spectral_oracle()
        .map_err(AdjointReferenceError::Network)?;
    let capacities = &model.network.capacities;
    let graph_dimension = capacities.len();
    let sensor_count = sensor_nodes.len();
    let time_count = scenario.observation_times.len();
    if precision_difference.len() != time_count * sensor_count
        || gradient.len() != profile.segments.len() * actuator_nodes.len()
    {
        return Err(AdjointReferenceError::InvalidPredictionShape);
    }

    let zero = vec![0.0; graph_dimension];
    let mut adjoint_at_sample = vec![0.0; graph_dimension];

    for time_index in (0..time_count).rev() {
        let sample_offset = time_index * sensor_count;
        for (sensor_index, node) in sensor_nodes.iter().enumerate() {
            adjoint_at_sample[*node] +=
                objective_sign * precision_difference[sample_offset + sensor_index];
        }

        let sample_time = scenario.observation_times[time_index];
        let lower_time = if time_index == 0 {
            0.0
        } else {
            scenario.observation_times[time_index - 1]
        };

        for (segment_index, segment) in profile.segments.iter().enumerate() {
            let overlap_start = segment.start_time.max(lower_time);
            let overlap_end = segment.end_time.min(sample_time);
            if overlap_end <= overlap_start {
                continue;
            }
            let tau_high = sample_time - overlap_start;
            let tau_low = sample_time - overlap_end;
            let integral_high = oracle
                .state_after_constant_forcing(
                    &zero,
                    &adjoint_at_sample,
                    tau_high,
                )
                .map_err(AdjointReferenceError::Network)?;
            let integral_low = oracle
                .state_after_constant_forcing(
                    &zero,
                    &adjoint_at_sample,
                    tau_low,
                )
                .map_err(AdjointReferenceError::Network)?;
            for (actuator_index, node) in actuator_nodes.iter().enumerate() {
                let coefficient_index =
                    segment_index * actuator_nodes.len() + actuator_index;
                gradient[coefficient_index] +=
                    integral_high[*node] - integral_low[*node];
            }
        }

        let duration = sample_time - lower_time;
        if duration > 0.0 {
            let adjoint_density: Vec<f64> = adjoint_at_sample
                .iter()
                .zip(capacities.iter())
                .map(|(value, capacity)| value / capacity)
                .collect();
            let propagated = oracle
                .state_after_constant_forcing(&adjoint_density, &zero, duration)
                .map_err(AdjointReferenceError::Network)?;
            for node in 0..graph_dimension {
                adjoint_at_sample[node] = capacities[node] * propagated[node];
            }
        }
    }
    Ok(())
}

fn prediction_vector(
    score: &crate::experiment_design::ExperimentDesignScore,
    model_id: u32,
    time_count: usize,
    sensor_count: usize,
) -> Result<Vec<f64>, AdjointReferenceError> {
    let prediction = score
        .model_observation_means
        .iter()
        .find(|prediction| prediction.model_id == model_id)
        .ok_or(AdjointReferenceError::MissingModelPrediction(model_id))?;
    if prediction.observations.len() != time_count
        || prediction
            .observations
            .iter()
            .any(|row| row.len() != sensor_count)
    {
        return Err(AdjointReferenceError::InvalidPredictionShape);
    }
    Ok(prediction.observations.iter().flatten().copied().collect())
}

fn vector_norm(values: &[f64]) -> f64 {
    values.iter().map(|value| value * value).sum::<f64>().sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::experiment_design::{ForcingSegment, ModelCandidate};
    use crate::network::{Edge, NetworkScenario};
    use crate::noise_robustness::{Ar1KroneckerCovariance, DenseSpdCovariance};
    use crate::sensor_design::SensorCandidate;
    use crate::total_kl_trust_region::{
        optimize_pairwise_total_kl_trust_region_with_covariance,
        ObservationDesignIdentityV1,
    };

    fn scenario() -> ExperimentDesignScenario {
        let chain = NetworkScenario {
            name: "adjoint-chain",
            capacities: vec![1.0, 1.0, 1.0],
            initial: vec![0.2, 0.0, 0.0],
            edges: vec![
                Edge { left: 0, right: 1, conductance: 1.0 },
                Edge { left: 1, right: 2, conductance: 1.0 },
            ],
            horizon: 2.0,
        };
        let bypass = NetworkScenario {
            name: "adjoint-bypass",
            capacities: vec![1.0, 1.0, 1.0],
            initial: vec![0.2, 0.0, 0.0],
            edges: vec![
                Edge { left: 0, right: 1, conductance: 1.0 },
                Edge { left: 1, right: 2, conductance: 1.0 },
                Edge { left: 0, right: 2, conductance: 0.4 },
            ],
            horizon: 2.0,
        };
        ExperimentDesignScenario {
            name: "adjoint-chain-bypass",
            models: vec![
                ModelCandidate {
                    id: 1,
                    label: "chain",
                    network: chain,
                },
                ModelCandidate {
                    id: 2,
                    label: "bypass",
                    network: bypass,
                },
            ],
            sensors: vec![
                SensorCandidate { id: 10, node: 1, cost: 1.0, common_cause_group: None },
                SensorCandidate { id: 11, node: 2, cost: 1.0, common_cause_group: None },
            ],
            noise_covariance: vec![vec![0.02, 0.005], vec![0.005, 0.03]],
            observation_times: vec![0.4, 0.8, 1.2, 1.6, 2.0],
            sensor_budget: 2.0,
            max_abs_forcing: 2.0,
            input_energy_budget: 0.5,
        }
    }

    fn binding(
        fixture: &ExperimentDesignScenario,
    ) -> (ObservationCovarianceV1, ObservationDesignIdentityV1) {
        let covariance_id = "covariance:adjoint-ar1-rho-0.6";
        let process_id = "process:adjoint-ar1-rho-0.6";
        let sampling_id = "sampling:adjoint-uniform-five";
        let operator_id = "operator:adjoint-sensors-10-11";
        let cutoff_id = "availability:adjoint-all-observations";
        let covariance = ObservationCovarianceV1::new(
            covariance_id.to_owned(),
            process_id.to_owned(),
            sampling_id.to_owned(),
            operator_id.to_owned(),
            cutoff_id.to_owned(),
            vec![10, 11],
            fixture.observation_times.clone(),
            Ar1KroneckerCovariance::new(
                fixture.observation_times.len(),
                0.6,
                DenseSpdCovariance::new(fixture.noise_covariance.clone()).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let identity = ObservationDesignIdentityV1 {
            covariance_id: covariance_id.to_owned(),
            observation_process_id: process_id.to_owned(),
            sampling_policy_id: sampling_id.to_owned(),
            observation_operator_id: operator_id.to_owned(),
            availability_cutoff_id: cutoff_id.to_owned(),
        };
        (covariance, identity)
    }

    fn total_kl_for_profile(
        fixture: &ExperimentDesignScenario,
        profile: &InputProfile,
        covariance: &ObservationCovarianceV1,
    ) -> f64 {
        let score = evaluate_input_profile(fixture, profile, &[10, 11]).unwrap();
        let left = prediction_vector(&score, 1, fixture.observation_times.len(), 2).unwrap();
        let right = prediction_vector(&score, 2, fixture.observation_times.len(), 2).unwrap();
        let difference: Vec<f64> = left
            .iter()
            .zip(right.iter())
            .map(|(a, b)| a - b)
            .collect();
        covariance.gaussian_quadratic_form_half(&difference).unwrap()
    }

    #[test]
    fn adjoint_gradient_matches_central_finite_difference_under_correlated_noise() {
        let fixture = scenario();
        let (covariance, identity) = binding(&fixture);
        let boundaries = [0.0, 0.5, 1.0, 2.0];
        let report = optimize_pairwise_total_kl_trust_region_with_covariance(
            &fixture,
            1,
            2,
            &[10, 11],
            &[0, 2],
            &boundaries,
            &covariance,
            &identity,
        )
        .unwrap();
        assert!(!report.peak_constraint_active);

        let adjoint = total_kl_control_gradient_adjoint(
            &fixture,
            &report.profile,
            1,
            2,
            &[10, 11],
            &[0, 2],
            &covariance,
            &identity,
            Some(report.trust_region_multiplier),
        )
        .unwrap();
        assert!(
            (adjoint.total_gaussian_kl - report.achieved_total_gaussian_kl).abs()
                < 1.0e-9
        );
        assert!(adjoint.energy_ball_stationarity_residual < 1.0e-7);

        let mut unconstrained_fixture = fixture.clone();
        unconstrained_fixture.max_abs_forcing = 100.0;
        unconstrained_fixture.input_energy_budget = 100.0;
        let epsilon = 1.0e-6;
        let mut maximum_gradient_error = 0.0_f64;
        let mut coefficient_index = 0;
        for segment_index in 0..report.profile.segments.len() {
            for actuator_node in [0_usize, 2_usize] {
                let mut plus = report.profile.clone();
                let mut minus = report.profile.clone();
                plus.segments[segment_index].forcing[actuator_node] += epsilon;
                minus.segments[segment_index].forcing[actuator_node] -= epsilon;
                let finite_difference = (
                    total_kl_for_profile(&unconstrained_fixture, &plus, &covariance)
                        - total_kl_for_profile(&unconstrained_fixture, &minus, &covariance)
                ) / (2.0 * epsilon);
                maximum_gradient_error = maximum_gradient_error.max(
                    (finite_difference - adjoint.control_gradient[coefficient_index]).abs(),
                );
                coefficient_index += 1;
            }
        }
        assert!(maximum_gradient_error < 1.0e-6);
    }

    #[test]
    fn adjoint_rejects_request_identity_mismatch() {
        let fixture = scenario();
        let (covariance, mut identity) = binding(&fixture);
        identity.observation_process_id = "stale-process".to_owned();
        let profile = InputProfile {
            id: 1,
            label: "passive",
            segments: vec![ForcingSegment {
                start_time: 0.0,
                end_time: 2.0,
                forcing: vec![0.0, 0.0, 0.0],
            }],
        };
        assert_eq!(
            total_kl_control_gradient_adjoint(
                &fixture,
                &profile,
                1,
                2,
                &[10, 11],
                &[0, 2],
                &covariance,
                &identity,
                None,
            ),
            Err(AdjointReferenceError::IdentityMismatch)
        );
    }
}
