//! Joint Gaussian contact fusion with explicit cross-channel covariance.
//!
//! V0 accepts 1–3 simultaneous, time-qualified sensor measurements per contact
//! event. The supplied covariance is calibrated upstream and bound to sensor IDs.

use std::collections::BTreeSet;

pub const STATE_DIM: usize = 3;
pub const MAX_CHANNELS: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorChannel {
    D1Interface,
    D2NearFootWrench,
    D2VibroAcoustic,
    D3Proprioceptive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactGaussian {
    pub mean: [f64; STATE_DIM],
    pub covariance: [[f64; STATE_DIM]; STATE_DIM],
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointSensorSample {
    pub evidence_id: String,
    pub sensor_id: String,
    pub channel: SensorChannel,
    pub measurement_model_id: String,
    pub coordinate_frame_id: String,
    pub unit_id: String,
    pub calibration_id: String,
    pub observation_row: [f64; STATE_DIM],
    pub value: f64,
    pub sample_time_ns: u64,
    pub arrival_time_ns: u64,
    pub timebase_verified: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointMeasurementGroup {
    pub group_id: String,
    pub contact_event_id: String,
    pub foot_id: String,
    pub clock_id: String,
    pub covariance_model_id: String,
    pub covariance_calibration_id: String,
    /// Row/column identities for the active leading m x m block of R.
    pub noise_covariance_sensor_ids: Vec<String>,
    /// Active leading m x m block is used; all inactive cells must be zero.
    pub noise_covariance: [[f64; MAX_CHANNELS]; MAX_CHANNELS],
    pub samples: Vec<JointSensorSample>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointFusionError {
    EmptyIdentifier,
    InvalidDimension,
    NonFiniteValue,
    InvalidCovariance,
    DuplicateEvidenceId,
    DuplicateSensorId,
    DuplicateMeasurementModel,
    CovarianceManifestMismatch,
    ContactEventMismatch,
    FootIdentityMismatch,
    ClockMismatch,
    TimebaseNotVerified,
    AsynchronousObservation,
    InvalidTimestampOrder,
    FutureObservation,
    StaleObservation,
    InvalidObservationModel,
    NumericalFailure,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointFusionResult {
    pub posterior: ContactGaussian,
    pub expected_information_gain_nats: f64,
    pub ordered_sensor_ids: Vec<String>,
    pub group_id: String,
    pub covariance_model_id: String,
    pub covariance_calibration_id: String,
}

#[derive(Debug, Clone)]
struct Cholesky {
    lower: [[f64; MAX_CHANNELS]; MAX_CHANNELS],
    log_determinant: f64,
}

impl ContactGaussian {
    pub fn validate(&self) -> Result<(), JointFusionError> {
        if self.mean.iter().any(|value| !value.is_finite()) {
            return Err(JointFusionError::NonFiniteValue);
        }
        validate_state_covariance(&self.covariance)
    }
}

/// Update the latent contact state with a correlated, simultaneous measurement
/// group. R's channel mapping is explicit and input-order invariant.
pub fn update_joint_measurement(
    prior: &ContactGaussian,
    group: &JointMeasurementGroup,
    now_ns: u64,
    maximum_sample_age_ns: u64,
) -> Result<JointFusionResult, JointFusionError> {
    prior.validate()?;
    let (samples, noise_covariance) =
        validate_and_canonicalize_group(group, now_ns, maximum_sample_age_ns)?;
    let dimension = samples.len();

    let mut observation_times_state = [[0.0; STATE_DIM]; MAX_CHANNELS];
    let mut innovation = [0.0; MAX_CHANNELS];
    for (row_index, sample) in samples.iter().enumerate() {
        observation_times_state[row_index] =
            mat_vec_state(&prior.covariance, &sample.observation_row);
        innovation[row_index] = sample.value - dot_state(&sample.observation_row, &prior.mean);
    }

    let mut innovation_covariance = noise_covariance;
    for i in 0..dimension {
        for j in 0..dimension {
            innovation_covariance[i][j] +=
                dot_state(&samples[i].observation_row, &observation_times_state[j]);
        }
    }

    let chol_r = cholesky(&noise_covariance, dimension)?;
    let chol_s = cholesky(&innovation_covariance, dimension)?;

    let mut gain = [[0.0; MAX_CHANNELS]; STATE_DIM];
    for state_index in 0..STATE_DIM {
        let mut rhs = [0.0; MAX_CHANNELS];
        for measurement_index in 0..dimension {
            rhs[measurement_index] = observation_times_state[measurement_index][state_index];
        }
        let solved = solve_spd(&chol_s, &rhs, dimension)?;
        for measurement_index in 0..dimension {
            gain[state_index][measurement_index] = solved[measurement_index];
        }
    }

    let mut posterior = prior.clone();
    for state_index in 0..STATE_DIM {
        for measurement_index in 0..dimension {
            posterior.mean[state_index] +=
                gain[state_index][measurement_index] * innovation[measurement_index];
        }
        for other_state_index in 0..STATE_DIM {
            let reduction = (0..dimension)
                .map(|measurement_index| {
                    gain[state_index][measurement_index]
                        * observation_times_state[measurement_index][other_state_index]
                })
                .sum::<f64>();
            posterior.covariance[state_index][other_state_index] -= reduction;
        }
    }

    symmetrize_state(&mut posterior.covariance);
    posterior.validate()?;

    let raw_eig = 0.5 * (chol_s.log_determinant - chol_r.log_determinant);
    if !raw_eig.is_finite() || raw_eig < -1.0e-10 {
        return Err(JointFusionError::NumericalFailure);
    }
    let expected_information_gain_nats = raw_eig.max(0.0);

    Ok(JointFusionResult {
        posterior,
        expected_information_gain_nats,
        ordered_sensor_ids: samples.iter().map(|sample| sample.sensor_id.clone()).collect(),
        group_id: group.group_id.clone(),
        covariance_model_id: group.covariance_model_id.clone(),
        covariance_calibration_id: group.covariance_calibration_id.clone(),
    })
}

fn validate_and_canonicalize_group(
    group: &JointMeasurementGroup,
    now_ns: u64,
    maximum_sample_age_ns: u64,
) -> Result<(Vec<JointSensorSample>, [[f64; MAX_CHANNELS]; MAX_CHANNELS]), JointFusionError> {
    for id in [
        &group.group_id,
        &group.contact_event_id,
        &group.foot_id,
        &group.clock_id,
        &group.covariance_model_id,
        &group.covariance_calibration_id,
    ] {
        if id.trim().is_empty() {
            return Err(JointFusionError::EmptyIdentifier);
        }
    }
    let dimension = group.samples.len();
    if dimension == 0 || dimension > MAX_CHANNELS {
        return Err(JointFusionError::InvalidDimension);
    }
    if group.noise_covariance_sensor_ids.len() != dimension {
        return Err(JointFusionError::CovarianceManifestMismatch);
    }

    let mut evidence_ids = BTreeSet::new();
    let mut sensor_ids = BTreeSet::new();
    let mut model_ids = BTreeSet::new();
    let reference_time = group.samples[0].sample_time_ns;
    let mut samples = group.samples.clone();

    for sample in &samples {
        for id in [
            &sample.evidence_id,
            &sample.sensor_id,
            &sample.measurement_model_id,
            &sample.coordinate_frame_id,
            &sample.unit_id,
            &sample.calibration_id,
        ] {
            if id.trim().is_empty() {
                return Err(JointFusionError::EmptyIdentifier);
            }
        }
        if !sample.value.is_finite()
            || sample.observation_row.iter().any(|value| !value.is_finite())
        {
            return Err(JointFusionError::NonFiniteValue);
        }
        if !sample.timebase_verified {
            return Err(JointFusionError::TimebaseNotVerified);
        }
        if sample.sample_time_ns != reference_time {
            return Err(JointFusionError::AsynchronousObservation);
        }
        if sample.sample_time_ns > sample.arrival_time_ns {
            return Err(JointFusionError::InvalidTimestampOrder);
        }
        if sample.arrival_time_ns > now_ns {
            return Err(JointFusionError::FutureObservation);
        }
        if now_ns.saturating_sub(sample.sample_time_ns) > maximum_sample_age_ns {
            return Err(JointFusionError::StaleObservation);
        }
        if !evidence_ids.insert(sample.evidence_id.as_str()) {
            return Err(JointFusionError::DuplicateEvidenceId);
        }
        if !sensor_ids.insert(sample.sensor_id.as_str()) {
            return Err(JointFusionError::DuplicateSensorId);
        }
        if !model_ids.insert(sample.measurement_model_id.as_str()) {
            return Err(JointFusionError::DuplicateMeasurementModel);
        }
    }

    let mut covariance_ids = BTreeSet::new();
    for sensor_id in &group.noise_covariance_sensor_ids {
        if sensor_id.trim().is_empty() {
            return Err(JointFusionError::EmptyIdentifier);
        }
        if !covariance_ids.insert(sensor_id.as_str()) {
            return Err(JointFusionError::CovarianceManifestMismatch);
        }
    }
    if covariance_ids != sensor_ids {
        return Err(JointFusionError::CovarianceManifestMismatch);
    }

    // Validate all matrix entries and require unused padded cells to be zero.
    for i in 0..MAX_CHANNELS {
        for j in 0..MAX_CHANNELS {
            let value = group.noise_covariance[i][j];
            if !value.is_finite() {
                return Err(JointFusionError::InvalidCovariance);
            }
            if (i >= dimension || j >= dimension) && value != 0.0 {
                return Err(JointFusionError::InvalidCovariance);
            }
        }
    }

    samples.sort_by(|left, right| left.sensor_id.cmp(&right.sensor_id));
    let mut canonical_covariance = [[0.0; MAX_CHANNELS]; MAX_CHANNELS];
    for i in 0..dimension {
        let source_i = group
            .noise_covariance_sensor_ids
            .iter()
            .position(|id| id == &samples[i].sensor_id)
            .ok_or(JointFusionError::CovarianceManifestMismatch)?;
        for j in 0..dimension {
            let source_j = group
                .noise_covariance_sensor_ids
                .iter()
                .position(|id| id == &samples[j].sensor_id)
                .ok_or(JointFusionError::CovarianceManifestMismatch)?;
            canonical_covariance[i][j] = group.noise_covariance[source_i][source_j];
        }
    }

    cholesky(&canonical_covariance, dimension)?;
    Ok((samples, canonical_covariance))
}

fn validate_state_covariance(
    covariance: &[[f64; STATE_DIM]; STATE_DIM],
) -> Result<(), JointFusionError> {
    let mut expanded = [[0.0; MAX_CHANNELS]; MAX_CHANNELS];
    for (i, row) in covariance.iter().enumerate() {
        for (j, value) in row.iter().enumerate() {
            expanded[i][j] = *value;
        }
    }
    cholesky(&expanded, STATE_DIM)?;
    Ok(())
}

fn cholesky(
    matrix: &[[f64; MAX_CHANNELS]; MAX_CHANNELS],
    dimension: usize,
) -> Result<Cholesky, JointFusionError> {
    if dimension == 0 || dimension > MAX_CHANNELS {
        return Err(JointFusionError::InvalidDimension);
    }
    let mut lower = [[0.0; MAX_CHANNELS]; MAX_CHANNELS];
    for i in 0..dimension {
        for j in 0..=i {
            let mut sum = matrix[i][j];
            for (&left, &right) in lower[i][..j].iter().zip(lower[j][..j].iter()) {
                sum -= left * right;
            }
            if i == j {
                if !sum.is_finite() || sum <= 0.0 {
                    return Err(JointFusionError::InvalidCovariance);
                }
                lower[i][j] = sum.sqrt();
            } else {
                if lower[j][j] <= 0.0 {
                    return Err(JointFusionError::InvalidCovariance);
                }
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    let log_determinant = lower
        .iter()
        .enumerate()
        .take(dimension)
        .map(|(index, row)| 2.0 * row[index].ln())
        .sum::<f64>();
    if !log_determinant.is_finite() {
        return Err(JointFusionError::NumericalFailure);
    }
    Ok(Cholesky {
        lower,
        log_determinant,
    })
}

fn solve_spd(
    factor: &Cholesky,
    right_hand_side: &[f64; MAX_CHANNELS],
    dimension: usize,
) -> Result<[f64; MAX_CHANNELS], JointFusionError> {
    let mut forward = [0.0; MAX_CHANNELS];
    for i in 0..dimension {
        let mut sum = right_hand_side[i];
        for (&lower_entry, &prior_solution) in
            factor.lower[i][..i].iter().zip(forward[..i].iter())
        {
            sum -= lower_entry * prior_solution;
        }
        forward[i] = sum / factor.lower[i][i];
    }

    let mut solution = [0.0; MAX_CHANNELS];
    for i in (0..dimension).rev() {
        let mut sum = forward[i];
        for (row, &known_solution) in factor
            .lower
            .iter()
            .zip(solution.iter())
            .skip(i + 1)
            .take(dimension - i - 1)
        {
            sum -= row[i] * known_solution;
        }
        solution[i] = sum / factor.lower[i][i];
    }
    if solution[..dimension].iter().any(|value| !value.is_finite()) {
        return Err(JointFusionError::NumericalFailure);
    }
    Ok(solution)
}

fn mat_vec_state(
    matrix: &[[f64; STATE_DIM]; STATE_DIM],
    vector: &[f64; STATE_DIM],
) -> [f64; STATE_DIM] {
    let mut result = [0.0; STATE_DIM];
    for (i, row) in matrix.iter().enumerate() {
        result[i] = row.iter().zip(vector.iter()).map(|(a, b)| a * b).sum();
    }
    result
}

fn dot_state(left: &[f64; STATE_DIM], right: &[f64; STATE_DIM]) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn symmetrize_state(matrix: &mut [[f64; STATE_DIM]; STATE_DIM]) {
    let original = *matrix;
    let mut symmetric = original;
    for (i, row) in original.iter().enumerate() {
        for (j, &value) in row.iter().enumerate().take(i) {
            let average = 0.5 * (value + original[j][i]);
            symmetric[i][j] = average;
            symmetric[j][i] = average;
        }
    }
    *matrix = symmetric;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prior() -> ContactGaussian {
        ContactGaussian {
            mean: [0.0; STATE_DIM],
            covariance: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    fn sample(
        evidence_id: &str,
        sensor_id: &str,
        channel: SensorChannel,
        row: [f64; STATE_DIM],
        value: f64,
        sample_time_ns: u64,
    ) -> JointSensorSample {
        JointSensorSample {
            evidence_id: evidence_id.to_owned(),
            sensor_id: sensor_id.to_owned(),
            channel,
            measurement_model_id: format!("model-{sensor_id}"),
            coordinate_frame_id: format!("frame-{sensor_id}"),
            unit_id: format!("unit-{sensor_id}"),
            calibration_id: format!("cal-{sensor_id}"),
            observation_row: row,
            value,
            sample_time_ns,
            arrival_time_ns: sample_time_ns + 1,
            timebase_verified: true,
        }
    }

    fn group(samples: Vec<JointSensorSample>, covariance: [[f64; 3]; 3], sensor_ids: &[&str]) -> JointMeasurementGroup {
        JointMeasurementGroup {
            group_id: "group-1".to_owned(),
            contact_event_id: "contact-1".to_owned(),
            foot_id: "left-front".to_owned(),
            clock_id: "physics-clock".to_owned(),
            covariance_model_id: "joint-noise-v1".to_owned(),
            covariance_calibration_id: "joint-calibration-v3".to_owned(),
            noise_covariance_sensor_ids: sensor_ids.iter().map(|id| (*id).to_owned()).collect(),
            noise_covariance: covariance,
            samples,
        }
    }

    #[test]
    fn correlated_two_sensor_fixture_matches_exact_posterior_and_eig() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let result = update_joint_measurement(
            &prior(),
            &group(samples, noise, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();

        assert!((result.posterior.mean[0] - 1.2).abs() < 1.0e-12);
        assert!((result.posterior.mean[1] + 0.8).abs() < 1.0e-12);
        assert!((result.posterior.covariance[0][0] - 7.0 / 15.0).abs() < 1.0e-12);
        assert!((result.posterior.covariance[0][1] - 2.0 / 15.0).abs() < 1.0e-12);
        assert!((result.posterior.covariance[1][1] - 7.0 / 15.0).abs() < 1.0e-12);
        assert!((result.posterior.covariance[2][2] - 1.0).abs() < 1.0e-12);
        assert!((result.expected_information_gain_nats - 0.5 * 5.0_f64.ln()).abs() < 1.0e-12);
    }

    #[test]
    fn diagonal_noise_matches_sequential_independent_scalar_updates() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let result = update_joint_measurement(
            &prior(),
            &group(samples, noise, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();

        assert!((result.posterior.mean[0] - 1.0).abs() < 1.0e-12);
        assert!((result.posterior.mean[1] + 0.5).abs() < 1.0e-12);
        assert!((result.posterior.covariance[0][0] - 0.5).abs() < 1.0e-12);
        assert!((result.posterior.covariance[1][1] - 0.5).abs() < 1.0e-12);
        assert!((result.expected_information_gain_nats - 2.0 * 0.5 * 2.0_f64.ln()).abs() < 1.0e-12);
    }

    #[test]
    fn sensor_input_permutation_with_manifest_preserves_result() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let first = update_joint_measurement(
            &prior(),
            &group(samples.clone(), noise, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();
        let second = update_joint_measurement(
            &prior(),
            &group(samples.into_iter().rev().collect(), noise, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn covariance_manifest_mismatch_is_rejected() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(
            update_joint_measurement(
                &prior(),
                &group(samples, noise, &["sensor-a", "wrong-sensor"]),
                110,
                20,
            ),
            Err(JointFusionError::CovarianceManifestMismatch)
        );
    }

    #[test]
    fn asynchronous_samples_are_rejected_instead_of_silently_joined() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 102),
        ];
        let noise = [[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(
            update_joint_measurement(
                &prior(),
                &group(samples, noise, &["sensor-a", "sensor-b"]),
                110,
                20,
            ),
            Err(JointFusionError::AsynchronousObservation)
        );
    }

    #[test]
    fn non_spd_correlated_noise_is_rejected() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("eb", "sensor-b", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 2.0, 0.0], [2.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(
            update_joint_measurement(
                &prior(),
                &group(samples, noise, &["sensor-a", "sensor-b"]),
                110,
                20,
            ),
            Err(JointFusionError::InvalidCovariance)
        );
    }

    #[test]
    fn duplicated_sensor_and_evidence_ids_are_rejected() {
        let samples = vec![
            sample("same", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 2.0, 100),
            sample("same", "sensor-a", SensorChannel::D3Proprioceptive, [0.0, 1.0, 0.0], -1.0, 100),
        ];
        let noise = [[1.0, 0.5, 0.0], [0.5, 1.0, 0.0], [0.0, 0.0, 0.0]];
        assert_eq!(
            update_joint_measurement(
                &prior(),
                &group(samples, noise, &["sensor-a", "sensor-b"]),
                110,
                20,
            ),
            Err(JointFusionError::DuplicateEvidenceId)
        );
    }

    #[test]
    fn independent_noise_shortcut_overstates_information_for_correlated_duplicate_channels() {
        let samples = vec![
            sample("ea", "sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0], 1.0, 100),
            sample("eb", "sensor-b", SensorChannel::D2NearFootWrench, [1.0, 0.0, 0.0], 1.0, 100),
        ];
        let correlated = [[1.0, 0.9, 0.0], [0.9, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let diagonal = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 0.0]];
        let correlated_result = update_joint_measurement(
            &prior(),
            &group(samples.clone(), correlated, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();
        let diagonal_result = update_joint_measurement(
            &prior(),
            &group(samples, diagonal, &["sensor-a", "sensor-b"]),
            110,
            20,
        )
        .unwrap();
        assert!(correlated_result.expected_information_gain_nats < diagonal_result.expected_information_gain_nats);
    }
}
