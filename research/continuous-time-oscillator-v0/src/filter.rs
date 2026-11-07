//! Multichannel Kalman filtering for the exact continuous-time oscillator.
//!
//! The latent state is [z1, z2, b_1, ..., b_C], where z1/z2 form the
//! oscillator and b_c is a static animal-level random intercept for channel
//! c. Oscillator state resets at a new session; animal effects persist.
//! Missing channels are omitted, never imputed as zero.

use std::collections::BTreeMap;

use crate::{OscillatorError, OscillatorParameters};

type Matrix = Vec<Vec<f64>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelSpec {
    pub id: String,
    pub loading: [f64; 2],
    pub intercept: f64,
    pub observation_variance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelMeasurement {
    pub channel_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObservationForecast {
    pub mean: f64,
    pub variance: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhaseEstimate {
    pub phase_radians: f64,
    pub amplitude: f64,
    pub phase_variance: f64,
    pub amplitude_signal_to_noise: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FilterUpdate {
    pub timestamp: f64,
    pub observations_used: usize,
    pub incremental_log_likelihood: f64,
    pub cumulative_log_likelihood: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterError {
    InvalidOscillatorParameters(OscillatorError),
    NoChannels,
    EmptyChannelId,
    DuplicateChannelId,
    InvalidChannelLoading,
    InvalidChannelIntercept,
    InvalidObservationVariance,
    InvalidAnimalEffectVariance,
    EmptySessionId,
    DuplicateSessionId,
    NoActiveSession,
    InvalidTimestamp,
    NonMonotonicTimestamp,
    UnknownChannel,
    DuplicateChannelMeasurement,
    InvalidObservation,
    InvalidPhasePerturbation,
    InvalidPhaseSnrThreshold,
    NumericalFailure,
}

impl From<OscillatorError> for FilterError {
    fn from(value: OscillatorError) -> Self {
        Self::InvalidOscillatorParameters(value)
    }
}

#[derive(Debug, Clone)]
pub struct OscillatorObservationFilter {
    parameters: OscillatorParameters,
    channels: Vec<ChannelSpec>,
    channel_indices: BTreeMap<String, usize>,
    mean: Vec<f64>,
    covariance: Matrix,
    active_session: Option<String>,
    last_timestamp: Option<f64>,
    cumulative_log_likelihood: f64,
}

impl OscillatorObservationFilter {
    pub fn new(
        parameters: OscillatorParameters,
        channels: Vec<ChannelSpec>,
        animal_effect_variance: f64,
    ) -> Result<Self, FilterError> {
        parameters.validate()?;
        if channels.is_empty() {
            return Err(FilterError::NoChannels);
        }
        if !animal_effect_variance.is_finite() || animal_effect_variance < 0.0 {
            return Err(FilterError::InvalidAnimalEffectVariance);
        }

        let mut channel_indices = BTreeMap::new();
        for (index, channel) in channels.iter().enumerate() {
            if channel.id.trim().is_empty() {
                return Err(FilterError::EmptyChannelId);
            }
            if channel_indices.insert(channel.id.clone(), index).is_some() {
                return Err(FilterError::DuplicateChannelId);
            }
            if !channel.loading.iter().all(|value| value.is_finite()) {
                return Err(FilterError::InvalidChannelLoading);
            }
            if !channel.intercept.is_finite() {
                return Err(FilterError::InvalidChannelIntercept);
            }
            if !channel.observation_variance.is_finite() || channel.observation_variance <= 0.0 {
                return Err(FilterError::InvalidObservationVariance);
            }
        }

        let dimension = 2 + channels.len();
        let mean = vec![0.0; dimension];
        let mut covariance = zero_matrix(dimension);
        covariance[0][0] = parameters.stationary_variance;
        covariance[1][1] = parameters.stationary_variance;
        for (channel_index, _) in channels.iter().enumerate() {
            covariance[2 + channel_index][2 + channel_index] = animal_effect_variance;
        }

        Ok(Self {
            parameters,
            channels,
            channel_indices,
            mean,
            covariance,
            active_session: None,
            last_timestamp: None,
            cumulative_log_likelihood: 0.0,
        })
    }

    /// Reset the oscillator prior while preserving posterior animal effects.
    pub fn start_session(&mut self, session_id: &str) -> Result<(), FilterError> {
        if session_id.trim().is_empty() {
            return Err(FilterError::EmptySessionId);
        }
        if self.active_session.as_deref() == Some(session_id) {
            return Err(FilterError::DuplicateSessionId);
        }

        let dimension = self.mean.len();
        let mut reset_covariance = zero_matrix(dimension);
        reset_covariance[0][0] = self.parameters.stationary_variance;
        reset_covariance[1][1] = self.parameters.stationary_variance;
        for (row_index, row) in reset_covariance.iter_mut().enumerate().skip(2) {
            for (column_index, value) in row.iter_mut().enumerate().skip(2) {
                *value = self.covariance[row_index][column_index];
            }
        }

        self.mean[0] = 0.0;
        self.mean[1] = 0.0;
        self.covariance = reset_covariance;
        self.active_session = Some(session_id.to_owned());
        self.last_timestamp = None;
        Ok(())
    }

    /// Apply a declared instantaneous phase perturbation at the current time.
    /// The caller must independently bind it to the intervention assignment
    /// and delivery record; this method does not authenticate the action.
    pub fn apply_phase_perturbation(
        &mut self,
        delta_phase_radians: f64,
    ) -> Result<(), FilterError> {
        if self.active_session.is_none() {
            return Err(FilterError::NoActiveSession);
        }
        if !delta_phase_radians.is_finite() {
            return Err(FilterError::InvalidPhasePerturbation);
        }

        let (sin_phase, cos_phase) = delta_phase_radians.sin_cos();
        let dimension = self.mean.len();
        let mut transform = identity_matrix(dimension);
        transform[0][0] = cos_phase;
        transform[0][1] = -sin_phase;
        transform[1][0] = sin_phase;
        transform[1][1] = cos_phase;

        let rotated_mean = matrix_vector_product(&transform, &self.mean);
        let rotated_covariance = matrix_product(
            &matrix_product(&transform, &self.covariance),
            &transpose(&transform),
        );
        if !rotated_mean.iter().all(|value| value.is_finite())
            || !matrix_is_finite(&rotated_covariance)
        {
            return Err(FilterError::NumericalFailure);
        }
        self.mean = rotated_mean;
        self.covariance = rotated_covariance;
        Ok(())
    }

    /// Advance to an irregular timestamp, updating only channels actually
    /// observed. An empty slice records an all-channel dropout while
    /// propagating the latent state forward.
    ///
    /// Measurements are processed in canonical channel order so results are
    /// deterministic with respect to caller ordering. Each channel may appear
    /// at most once per timestamp.
    pub fn observe_batch(
        &mut self,
        timestamp: f64,
        observations: &[ChannelMeasurement],
    ) -> Result<FilterUpdate, FilterError> {
        // Apply a complete timestamp atomically. A bad late channel or
        // numerical overflow must not leave a partially conditioned posterior.
        let mut candidate = self.clone();
        let update = candidate.observe_batch_inner(timestamp, observations)?;
        *self = candidate;
        Ok(update)
    }

    fn observe_batch_inner(
        &mut self,
        timestamp: f64,
        observations: &[ChannelMeasurement],
    ) -> Result<FilterUpdate, FilterError> {
        if self.active_session.is_none() {
            return Err(FilterError::NoActiveSession);
        }
        if !timestamp.is_finite() {
            return Err(FilterError::InvalidTimestamp);
        }
        if self
            .last_timestamp
            .is_some_and(|previous| timestamp <= previous)
        {
            return Err(FilterError::NonMonotonicTimestamp);
        }

        let mut ordered = Vec::with_capacity(observations.len());
        let mut seen = BTreeMap::new();
        for observation in observations {
            if !observation.value.is_finite() {
                return Err(FilterError::InvalidObservation);
            }
            let channel_index = self
                .channel_indices
                .get(&observation.channel_id)
                .copied()
                .ok_or(FilterError::UnknownChannel)?;
            if seen.insert(channel_index, ()).is_some() {
                return Err(FilterError::DuplicateChannelMeasurement);
            }
            ordered.push((channel_index, observation));
        }
        ordered.sort_by_key(|(channel_index, _)| *channel_index);

        if let Some(previous) = self.last_timestamp {
            self.propagate(timestamp - previous)?;
        }
        self.last_timestamp = Some(timestamp);

        let mut incremental_log_likelihood = 0.0;
        for (channel_index, observation) in &ordered {
            incremental_log_likelihood += self.update_one(*channel_index, observation.value)?;
        }
        if !incremental_log_likelihood.is_finite() {
            return Err(FilterError::NumericalFailure);
        }
        self.cumulative_log_likelihood += incremental_log_likelihood;
        if !self.cumulative_log_likelihood.is_finite() {
            return Err(FilterError::NumericalFailure);
        }

        Ok(FilterUpdate {
            timestamp,
            observations_used: ordered.len(),
            incremental_log_likelihood,
            cumulative_log_likelihood: self.cumulative_log_likelihood,
        })
    }

    pub fn predict_observation(
        &self,
        channel_id: &str,
    ) -> Result<ObservationForecast, FilterError> {
        let channel_index = self
            .channel_indices
            .get(channel_id)
            .copied()
            .ok_or(FilterError::UnknownChannel)?;
        let channel = &self.channels[channel_index];
        let mut loading = vec![0.0; self.mean.len()];
        loading[0] = channel.loading[0];
        loading[1] = channel.loading[1];
        loading[2 + channel_index] = 1.0;

        let mean = channel.intercept + dot(&loading, &self.mean);
        let covariance_loading = matrix_vector_product(&self.covariance, &loading);
        let variance = dot(&loading, &covariance_loading) + channel.observation_variance;
        if !mean.is_finite() || !variance.is_finite() || variance <= 0.0 {
            return Err(FilterError::NumericalFailure);
        }
        Ok(ObservationForecast { mean, variance })
    }

    /// Posterior covariance between the oscillator coordinates and one
    /// channel's static animal effect. Useful for verifying session resets.
    pub fn latent_effect_cross_covariance(
        &self,
        channel_id: &str,
    ) -> Result<[f64; 2], FilterError> {
        let channel_index = self
            .channel_indices
            .get(channel_id)
            .copied()
            .ok_or(FilterError::UnknownChannel)?;
        let effect_index = 2 + channel_index;
        Ok([
            self.covariance[0][effect_index],
            self.covariance[1][effect_index],
        ])
    }

    /// The channel random effect is animal-level and is intentionally carried
    /// into every later session for the same filter instance.
    pub fn channel_effect_posterior(&self, channel_id: &str) -> Result<(f64, f64), FilterError> {
        let channel_index = self
            .channel_indices
            .get(channel_id)
            .copied()
            .ok_or(FilterError::UnknownChannel)?;
        let index = 2 + channel_index;
        Ok((self.mean[index], self.covariance[index][index]))
    }

    /// Return no phase when amplitude is too small relative to latent
    /// uncertainty. Phase variance uses a local delta-method approximation.
    pub fn phase_estimate(
        &self,
        minimum_amplitude_signal_to_noise: f64,
    ) -> Result<Option<PhaseEstimate>, FilterError> {
        if !minimum_amplitude_signal_to_noise.is_finite() || minimum_amplitude_signal_to_noise < 0.0
        {
            return Err(FilterError::InvalidPhaseSnrThreshold);
        }

        let z1 = self.mean[0];
        let z2 = self.mean[1];
        let radius_squared = z1.mul_add(z1, z2 * z2);
        let amplitude = radius_squared.sqrt();
        if !amplitude.is_finite() || amplitude <= 0.0 {
            return Ok(None);
        }

        let uncertainty_squared = (self.covariance[0][0] + self.covariance[1][1]).max(0.0);
        let uncertainty = uncertainty_squared.sqrt();
        let signal_to_noise = if uncertainty > 0.0 {
            amplitude / uncertainty
        } else {
            f64::INFINITY
        };
        if signal_to_noise < minimum_amplitude_signal_to_noise {
            return Ok(None);
        }

        let gradient = [-z2 / radius_squared, z1 / radius_squared];
        let phase_variance = gradient[0] * gradient[0] * self.covariance[0][0]
            + 2.0 * gradient[0] * gradient[1] * self.covariance[0][1]
            + gradient[1] * gradient[1] * self.covariance[1][1];
        let phase_variance = phase_variance.max(0.0);
        if !phase_variance.is_finite() || !signal_to_noise.is_finite() {
            return Ok(None);
        }

        Ok(Some(PhaseEstimate {
            phase_radians: z2.atan2(z1),
            amplitude,
            phase_variance,
            amplitude_signal_to_noise: signal_to_noise,
        }))
    }

    pub fn latent_mean(&self) -> [f64; 2] {
        [self.mean[0], self.mean[1]]
    }

    pub fn latent_covariance(&self) -> [[f64; 2]; 2] {
        [
            [self.covariance[0][0], self.covariance[0][1]],
            [self.covariance[1][0], self.covariance[1][1]],
        ]
    }

    pub fn active_session(&self) -> Option<&str> {
        self.active_session.as_deref()
    }

    pub fn last_timestamp(&self) -> Option<f64> {
        self.last_timestamp
    }

    pub fn cumulative_log_likelihood(&self) -> f64 {
        self.cumulative_log_likelihood
    }

    fn propagate(&mut self, elapsed_time: f64) -> Result<(), FilterError> {
        let transition = self.parameters.transition(elapsed_time)?;
        let dimension = self.mean.len();
        let mut transform = identity_matrix(dimension);
        transform[0][0] = transition.matrix[0][0];
        transform[0][1] = transition.matrix[0][1];
        transform[1][0] = transition.matrix[1][0];
        transform[1][1] = transition.matrix[1][1];

        self.mean = matrix_vector_product(&transform, &self.mean);
        let mut process_covariance = zero_matrix(dimension);
        process_covariance[0][0] = transition.process_covariance[0][0];
        process_covariance[0][1] = transition.process_covariance[0][1];
        process_covariance[1][0] = transition.process_covariance[1][0];
        process_covariance[1][1] = transition.process_covariance[1][1];

        self.covariance = matrix_sum(
            &matrix_product(
                &matrix_product(&transform, &self.covariance),
                &transpose(&transform),
            ),
            &process_covariance,
        );
        if !self.mean.iter().all(|value| value.is_finite()) || !matrix_is_finite(&self.covariance) {
            return Err(FilterError::NumericalFailure);
        }
        Ok(())
    }

    fn update_one(&mut self, channel_index: usize, value: f64) -> Result<f64, FilterError> {
        let channel = &self.channels[channel_index];
        let mut loading = vec![0.0; self.mean.len()];
        loading[0] = channel.loading[0];
        loading[1] = channel.loading[1];
        loading[2 + channel_index] = 1.0;

        let covariance_loading = matrix_vector_product(&self.covariance, &loading);
        let innovation_variance = dot(&loading, &covariance_loading) + channel.observation_variance;
        let innovation = value - channel.intercept - dot(&loading, &self.mean);
        if !innovation_variance.is_finite() || innovation_variance <= 0.0 || !innovation.is_finite()
        {
            return Err(FilterError::NumericalFailure);
        }

        let log_likelihood = -0.5
            * ((2.0 * std::f64::consts::PI * innovation_variance).ln()
                + innovation * innovation / innovation_variance);
        let gain: Vec<f64> = covariance_loading
            .iter()
            .map(|component| component / innovation_variance)
            .collect();
        for (mean_component, gain_component) in self.mean.iter_mut().zip(&gain) {
            *mean_component += gain_component * innovation;
        }

        // Joseph-form covariance update avoids the less stable P <- P - KHP.
        let mut residual_transform = identity_matrix(self.mean.len());
        for (row_index, row) in residual_transform.iter_mut().enumerate() {
            for (column_index, element) in row.iter_mut().enumerate() {
                *element -= gain[row_index] * loading[column_index];
            }
        }
        let left = matrix_product(
            &matrix_product(&residual_transform, &self.covariance),
            &transpose(&residual_transform),
        );
        let noise_term = outer_product(&gain, &gain, channel.observation_variance);
        self.covariance = matrix_sum(&left, &noise_term);
        symmetrize(&mut self.covariance);

        if !self.mean.iter().all(|component| component.is_finite())
            || !matrix_is_finite(&self.covariance)
            || !log_likelihood.is_finite()
        {
            return Err(FilterError::NumericalFailure);
        }
        Ok(log_likelihood)
    }
}

fn zero_matrix(dimension: usize) -> Matrix {
    vec![vec![0.0; dimension]; dimension]
}

fn identity_matrix(dimension: usize) -> Matrix {
    let mut matrix = zero_matrix(dimension);
    for (index, row) in matrix.iter_mut().enumerate() {
        row[index] = 1.0;
    }
    matrix
}

fn transpose(matrix: &Matrix) -> Matrix {
    let columns = matrix.first().map_or(0, Vec::len);
    let mut result = vec![vec![0.0; matrix.len()]; columns];
    for (row_index, row) in matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            result[column_index][row_index] = *value;
        }
    }
    result
}

fn matrix_product(left: &Matrix, right: &Matrix) -> Matrix {
    let right_transposed = transpose(right);
    left.iter()
        .map(|row| {
            right_transposed
                .iter()
                .map(|column| dot(row, column))
                .collect()
        })
        .collect()
}

fn matrix_vector_product(matrix: &Matrix, vector: &[f64]) -> Vec<f64> {
    matrix.iter().map(|row| dot(row, vector)).collect()
}

fn matrix_sum(left: &Matrix, right: &Matrix) -> Matrix {
    left.iter()
        .zip(right)
        .map(|(left_row, right_row)| {
            left_row
                .iter()
                .zip(right_row)
                .map(|(left_value, right_value)| left_value + right_value)
                .collect()
        })
        .collect()
}

fn outer_product(left: &[f64], right: &[f64], scale: f64) -> Matrix {
    left.iter()
        .map(|left_value| {
            right
                .iter()
                .map(|right_value| left_value * right_value * scale)
                .collect()
        })
        .collect()
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn symmetrize(matrix: &mut Matrix) {
    let transposed = transpose(matrix);
    for (row, transposed_row) in matrix.iter_mut().zip(transposed) {
        for (value, transposed_value) in row.iter_mut().zip(transposed_row) {
            *value = 0.5 * (*value + transposed_value);
        }
    }
}

fn matrix_is_finite(matrix: &Matrix) -> bool {
    matrix.iter().flatten().all(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(id: &str, loading: [f64; 2]) -> ChannelSpec {
        ChannelSpec {
            id: id.to_owned(),
            loading,
            intercept: 0.0,
            observation_variance: 0.25,
        }
    }

    fn filter() -> OscillatorObservationFilter {
        OscillatorObservationFilter::new(
            OscillatorParameters {
                angular_frequency: 2.0,
                damping_rate: 0.1,
                stationary_variance: 1.0,
            },
            vec![channel("left", [1.0, 0.0]), channel("right", [1.0, 0.0])],
            0.25,
        )
        .unwrap()
    }

    fn measurement(channel_id: &str, value: f64) -> ChannelMeasurement {
        ChannelMeasurement {
            channel_id: channel_id.to_owned(),
            value,
        }
    }

    fn assert_close(left: f64, right: f64, tolerance: f64) {
        assert!(
            (left - right).abs() <= tolerance,
            "left={left}, right={right}, tolerance={tolerance}"
        );
    }

    #[test]
    fn known_answer_update_partitions_information_between_oscillator_and_animal_effect() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        let result = filter
            .observe_batch(0.0, &[measurement("left", 2.0)])
            .unwrap();

        // Innovation variance = 1.0 + 0.25 + 0.25.
        assert_close(filter.latent_mean()[0], 2.0 / 1.5, 1.0e-12);
        let (effect_mean, effect_variance) = filter.channel_effect_posterior("left").unwrap();
        assert_close(effect_mean, 0.25 * 2.0 / 1.5, 1.0e-12);
        assert!(effect_variance > 0.0);
        assert_eq!(result.observations_used, 1);
        assert!(result.incremental_log_likelihood.is_finite());
    }

    #[test]
    fn a_measurement_in_one_channel_updates_forecasts_in_another() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        filter
            .observe_batch(0.0, &[measurement("left", 1.0)])
            .unwrap();

        let right = filter.predict_observation("right").unwrap();
        assert!(right.mean > 0.0);
        assert!(right.variance > 0.0);
        let (right_effect_mean, _) = filter.channel_effect_posterior("right").unwrap();
        assert_close(right_effect_mean, 0.0, 1.0e-12);
    }

    #[test]
    fn joint_measurement_result_is_independent_of_input_channel_order() {
        let mut forward = filter();
        let mut reverse = filter();
        forward.start_session("animal-forward").unwrap();
        reverse.start_session("animal-reverse").unwrap();

        let left = measurement("left", 1.2);
        let right = measurement("right", -0.3);
        let a = forward
            .observe_batch(0.0, &[left.clone(), right.clone()])
            .unwrap();
        let b = reverse.observe_batch(0.0, &[right, left]).unwrap();

        let forward_mean = forward.latent_mean();
        let reverse_mean = reverse.latent_mean();
        for (actual, expected) in forward_mean.iter().zip(reverse_mean) {
            assert_close(*actual, expected, 1.0e-12);
        }
        assert_close(
            a.incremental_log_likelihood,
            b.incremental_log_likelihood,
            1.0e-12,
        );
    }

    #[test]
    fn new_session_resets_oscillator_but_retains_animal_effect_posterior() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        filter
            .observe_batch(0.0, &[measurement("left", 2.0)])
            .unwrap();
        let (effect_before, variance_before) = filter.channel_effect_posterior("left").unwrap();

        filter.start_session("animal-1-session-2").unwrap();
        assert_eq!(filter.latent_mean(), [0.0, 0.0]);
        let (effect_after, variance_after) = filter.channel_effect_posterior("left").unwrap();
        assert_close(effect_after, effect_before, 1.0e-12);
        assert_close(variance_after, variance_before, 1.0e-12);
        assert_eq!(filter.last_timestamp(), None);
    }

    #[test]
    fn session_reset_removes_cross_covariance_with_new_oscillator_state() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        filter
            .observe_batch(0.0, &[measurement("left", 1.0)])
            .unwrap();
        let cross_before = filter.latent_effect_cross_covariance("left").unwrap();
        assert!(cross_before[0].abs() > 0.0);
        filter.start_session("animal-1-session-2").unwrap();

        let cross_after = filter.latent_effect_cross_covariance("left").unwrap();
        assert_close(cross_after[0], 0.0, 1.0e-12);
        assert_close(cross_after[1], 0.0, 1.0e-12);
        assert!(filter.channel_effect_posterior("left").is_ok());
    }

    #[test]
    fn irregular_elapsed_time_and_phase_perturbation_use_exact_transition() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        filter
            .observe_batch(0.0, &[measurement("left", 2.0)])
            .unwrap();
        filter
            .apply_phase_perturbation(std::f64::consts::FRAC_PI_2)
            .unwrap();

        let before = filter.latent_mean();
        filter.observe_batch(0.37, &[]).unwrap();
        let after = filter.latent_mean();
        let attenuation = (-0.1_f64 * 0.37).exp();
        let phase = 2.0_f64 * 0.37;
        let (sin_phase, cos_phase) = phase.sin_cos();
        let expected = [
            attenuation * (cos_phase * before[0] - sin_phase * before[1]),
            attenuation * (sin_phase * before[0] + cos_phase * before[1]),
        ];
        assert_close(after[0], expected[0], 1.0e-12);
        assert_close(after[1], expected[1], 1.0e-12);
    }

    #[test]
    fn omitted_channels_are_missing_not_zero_valued_measurements() {
        let mut observed = filter();
        let mut dropped = filter();
        observed.start_session("observed").unwrap();
        dropped.start_session("dropped").unwrap();
        observed
            .observe_batch(0.0, &[measurement("left", 1.5)])
            .unwrap();
        dropped.observe_batch(0.0, &[]).unwrap();

        let observed_right = observed.predict_observation("right").unwrap();
        let dropped_right = dropped.predict_observation("right").unwrap();
        assert!(observed_right.mean > 0.0);
        assert_close(dropped_right.mean, 0.0, 1.0e-12);
        assert!(observed_right.variance.is_finite());
    }

    #[test]
    fn phase_estimation_abstains_at_zero_amplitude_and_reports_uncertainty_after_signal() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        assert_eq!(filter.phase_estimate(1.0).unwrap(), None);

        filter
            .observe_batch(0.0, &[measurement("left", 4.0)])
            .unwrap();
        let estimate = filter.phase_estimate(0.0).unwrap().unwrap();
        assert!(estimate.amplitude > 0.0);
        assert!(estimate.phase_variance.is_finite());
        assert!(estimate.amplitude_signal_to_noise > 0.0);
    }

    #[test]
    fn rejects_duplicate_unknown_nonfinite_and_nonmonotonic_observations() {
        let mut filter = filter();
        assert_eq!(
            filter.observe_batch(0.0, &[]),
            Err(FilterError::NoActiveSession)
        );
        filter.start_session("animal-1-session-1").unwrap();
        assert_eq!(
            filter.observe_batch(0.0, &[measurement("left", 1.0), measurement("left", 2.0)]),
            Err(FilterError::DuplicateChannelMeasurement)
        );
        assert_eq!(
            filter.observe_batch(0.0, &[measurement("missing", 1.0)]),
            Err(FilterError::UnknownChannel)
        );
        assert_eq!(
            filter.observe_batch(0.0, &[measurement("left", f64::NAN)]),
            Err(FilterError::InvalidObservation)
        );
        filter.observe_batch(0.0, &[]).unwrap();
        assert_eq!(
            filter.observe_batch(0.0, &[]),
            Err(FilterError::NonMonotonicTimestamp)
        );
    }

    #[test]
    fn rejects_invalid_phase_perturbations_and_thresholds() {
        let mut filter = filter();
        assert_eq!(
            filter.apply_phase_perturbation(1.0),
            Err(FilterError::NoActiveSession)
        );
        filter.start_session("animal-1-session-1").unwrap();
        assert_eq!(
            filter.apply_phase_perturbation(f64::NAN),
            Err(FilterError::InvalidPhasePerturbation)
        );
        assert_eq!(
            filter.phase_estimate(-1.0),
            Err(FilterError::InvalidPhaseSnrThreshold)
        );
    }

    #[test]
    fn rejects_invalid_channel_and_filter_definitions() {
        let parameters = OscillatorParameters {
            angular_frequency: 1.0,
            damping_rate: 0.1,
            stationary_variance: 1.0,
        };
        assert!(matches!(
            OscillatorObservationFilter::new(parameters, vec![], 0.1),
            Err(FilterError::NoChannels)
        ));
        assert!(matches!(
            OscillatorObservationFilter::new(parameters, vec![channel("", [1.0, 0.0])], 0.1),
            Err(FilterError::EmptyChannelId)
        ));
        assert!(matches!(
            OscillatorObservationFilter::new(
                parameters,
                vec![channel("same", [1.0, 0.0]), channel("same", [0.0, 1.0])],
                0.1
            ),
            Err(FilterError::DuplicateChannelId)
        ));
        assert!(matches!(
            OscillatorObservationFilter::new(
                parameters,
                vec![channel("left", [f64::NAN, 0.0])],
                0.1
            ),
            Err(FilterError::InvalidChannelLoading)
        ));
    }

    #[test]
    fn rejects_duplicate_or_empty_session_ids() {
        let mut filter = filter();
        assert_eq!(filter.start_session(""), Err(FilterError::EmptySessionId));
        filter.start_session("session-1").unwrap();
        assert_eq!(
            filter.start_session("session-1"),
            Err(FilterError::DuplicateSessionId)
        );
    }

    #[test]
    fn a_numerical_failure_does_not_partially_commit_a_timestamp() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        let before_mean = filter.latent_mean();
        let result = filter.observe_batch(0.0, &[measurement("left", f64::MAX)]);
        assert_eq!(result, Err(FilterError::NumericalFailure));
        assert_eq!(filter.last_timestamp(), None);
        assert_eq!(filter.latent_mean(), before_mean);
    }

    #[test]
    fn batch_log_likelihood_matches_sequential_innovation_definition() {
        let mut filter = filter();
        filter.start_session("animal-1-session-1").unwrap();
        let forecast = filter.predict_observation("left").unwrap();
        let observation = 1.0;
        let expected = -0.5
            * ((2.0 * std::f64::consts::PI * forecast.variance).ln()
                + (observation - forecast.mean).powi(2) / forecast.variance);
        let result = filter
            .observe_batch(0.0, &[measurement("left", observation)])
            .unwrap();
        assert_close(result.incremental_log_likelihood, expected, 1.0e-12);
    }
}
