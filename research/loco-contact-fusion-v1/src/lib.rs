// SPDX-License-Identifier: AGPL-3.0-or-later
//! Joint Gaussian fusion for time-aligned, correlated legged-contact channels.
//!
//! V1 complements the scalar, action/clock-receipt-qualified contact-information
//! reference. It consumes one synchronized observation group and a full
//! cross-channel covariance. It does not infer covariance from data, authenticate
//! upstream receipts, estimate physical friction, or certify an action safe.

use std::collections::BTreeSet;

pub const CONTACT_STATE_DIM: usize = 3;
pub const MAX_CHANNELS: usize = 3;
pub type Matrix3 = [[f64; 3]; 3];

const SYMMETRY_TOLERANCE: f64 = 1.0e-10;
const NEGATIVE_EIG_TOLERANCE: f64 = 1.0e-10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorChannel {
    D1Interface,
    D2NearFootWrench,
    D2VibroAcoustic,
    D3Proprioceptive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FusionError {
    EmptyIdentifier,
    InvalidChannelCount,
    ChannelOrderNotCanonical,
    ChannelManifestMismatch,
    ChannelManifestDigestMismatch,
    DuplicateSensorId,
    DuplicateEvidenceId,
    UnknownSensor(String),
    MissingSensor(String),
    SensorChannelMismatch(String),
    CoordinateFrameMismatch(String),
    UnitMismatch(String),
    ModelIdentityMismatch(String),
    CalibrationIdentityMismatch(String),
    ObservationModelIdentityMismatch(String),
    InvalidCovarianceShape,
    CovarianceNotSymmetric,
    CovarianceNotPositiveDefinite,
    NonFiniteValue,
    ActionReceiptNotVerified,
    ActionReceiptMismatch,
    ActuatorModelMismatch,
    ClockReceiptNotVerified,
    ClockMismatch,
    ClockMappingReceiptMismatch,
    ClockMappingOutsideValidity,
    TimebaseNotVerified,
    AsynchronousSampleTime,
    InvalidTimestampOrder,
    FutureObservation,
    StaleObservation,
    ContactEventMismatch,
    FootIdentityMismatch,
    InvalidInformationGain,
}

/// Latent per-foot state. The three coordinates and their physical scaling are
/// fixture-defined; this crate does not label them as friction or force unless
/// the caller's source-qualified model makes that interpretation.
#[derive(Debug, Clone, PartialEq)]
pub struct ContactGaussian {
    pub mean: [f64; CONTACT_STATE_DIM],
    pub covariance: Matrix3,
}

/// One row of a calibrated action-conditioned observation map. Rows are stored
/// in canonical ascending sensor-ID order.
#[derive(Debug, Clone, PartialEq)]
pub struct ChannelModel {
    pub sensor_id: String,
    pub channel: SensorChannel,
    pub coordinate_frame_id: String,
    pub unit_id: String,
    pub observation_row: [f64; CONTACT_STATE_DIM],
}

/// Joint observation model y = H x + epsilon, epsilon ~ N(0, R).
///
/// The leading m by m block is used, where m is the channel count. Entries
/// outside that block must be zero. R follows channels and the manifest order.
#[derive(Debug, Clone, PartialEq)]
pub struct JointMeasurementModel {
    pub model_id: String,
    pub action_id: String,
    pub actuator_model_digest: String,
    pub calibration_digest: String,
    pub observation_model_digest: String,
    pub covariance_calibration_id: String,
    pub channel_order_manifest: Vec<String>,
    /// Caller-supplied identity for the canonical channel order. Checked for
    /// consistency but not cryptographically authenticated in this crate.
    pub channel_order_manifest_digest: String,
    pub channels: Vec<ChannelModel>,
    pub noise_covariance: Matrix3,
}

/// Upstream action realization evidence. Verification flags and digests are
/// trust inputs from an independent verifier; this crate checks their binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionExecutionReceipt {
    pub receipt_id: String,
    pub command_action_id: String,
    pub delivered_action_id: String,
    pub measured_actuation_digest: String,
    pub actuator_model_digest: String,
    pub delivery_verified: bool,
}

/// Upstream clock mapping evidence. The sample time must lie inside the
/// inclusive mapping-validity interval. Receipt authenticity remains external.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockMappingReceipt {
    pub receipt_id: String,
    pub clock_id: String,
    pub mapping_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub verified: bool,
}

/// One source-qualified channel observation. Values from different channels
/// retain their own units and coordinate frames.
#[derive(Debug, Clone, PartialEq)]
pub struct JointObservationSample {
    pub evidence_id: String,
    pub contact_event_id: String,
    pub foot_id: String,
    pub sensor_id: String,
    pub channel: SensorChannel,
    pub measurement_model_id: String,
    pub action_execution_receipt_id: String,
    pub clock_id: String,
    pub clock_mapping_receipt_id: String,
    pub coordinate_frame_id: String,
    pub unit_id: String,
    pub calibration_digest: String,
    pub observation_model_digest: String,
    pub sample_time_ns: u64,
    pub arrival_time_ns: u64,
    pub timebase_verified: bool,
    pub value: f64,
}

/// Sample vector order is not meaningful: samples are mapped by stable sensor
/// ID into the model's canonical channel order.
#[derive(Debug, Clone, PartialEq)]
pub struct JointObservationGroup {
    pub samples: Vec<JointObservationSample>,
    pub channel_order_manifest_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointFusionReport {
    pub posterior: ContactGaussian,
    pub expected_information_gain_nats: f64,
    pub model_id: String,
    pub action_id: String,
    pub contact_event_id: String,
    pub foot_id: String,
    pub action_execution_receipt_id: String,
    pub clock_mapping_receipt_id: String,
    pub clock_id: String,
    pub common_sample_time_ns: u64,
    pub evaluated_at_ns: u64,
    pub maximum_sample_age_ns: u64,
    pub channel_count: usize,
    pub ordered_sensor_ids: Vec<String>,
    pub covariance_calibration_id: String,
    pub channel_order_manifest_digest: String,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

fn all_finite(values: &[f64]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn zero_matrix() -> Matrix3 {
    [[0.0; 3]; 3]
}

fn cholesky_lower(matrix: &Matrix3, dimension: usize) -> Result<Matrix3, FusionError> {
    if dimension == 0 || dimension > MAX_CHANNELS {
        return Err(FusionError::InvalidCovarianceShape);
    }
    for (row, row_values) in matrix.iter().enumerate().take(dimension) {
        for (column, left) in row_values.iter().copied().enumerate().take(dimension) {
            if !left.is_finite() {
                return Err(FusionError::NonFiniteValue);
            }
            let right = matrix[column][row];
            let scale = 1.0_f64.max(left.abs()).max(right.abs());
            if (left - right).abs() > SYMMETRY_TOLERANCE * scale {
                return Err(FusionError::CovarianceNotSymmetric);
            }
        }
    }

    let mut lower = zero_matrix();
    for row in 0..dimension {
        for column in 0..=row {
            let mut value = matrix[row][column];
            for (inner, lower_value) in lower[row].iter().copied().enumerate().take(column) {
                value -= lower_value * lower[column][inner];
            }
            if row == column {
                if !value.is_finite() || value <= 0.0 {
                    return Err(FusionError::CovarianceNotPositiveDefinite);
                }
                lower[row][column] = value.sqrt();
            } else {
                let entry = value / lower[column][column];
                if !entry.is_finite() {
                    return Err(FusionError::NonFiniteValue);
                }
                lower[row][column] = entry;
            }
        }
    }
    Ok(lower)
}

fn validate_covariance(matrix: &Matrix3, dimension: usize) -> Result<(), FusionError> {
    let _ = cholesky_lower(matrix, dimension)?;
    Ok(())
}

fn solve_spd_vector(
    matrix: &Matrix3,
    vector: &[f64; 3],
    dimension: usize,
) -> Result<[f64; 3], FusionError> {
    let lower = cholesky_lower(matrix, dimension)?;
    if !all_finite(&vector[..dimension]) {
        return Err(FusionError::NonFiniteValue);
    }

    let mut forward = [0.0; 3];
    for row in 0..dimension {
        let mut value = vector[row];
        for (inner, forward_value) in forward.iter().copied().enumerate().take(row) {
            value -= lower[row][inner] * forward_value;
        }
        forward[row] = value / lower[row][row];
        if !forward[row].is_finite() {
            return Err(FusionError::NonFiniteValue);
        }
    }

    let mut solution = [0.0; 3];
    for reverse in 0..dimension {
        let row = dimension - 1 - reverse;
        let mut value = forward[row];
        for (inner, solution_value) in solution
            .iter()
            .copied()
            .enumerate()
            .skip(row + 1)
            .take(dimension.saturating_sub(row + 1))
        {
            value -= lower[inner][row] * solution_value;
        }
        solution[row] = value / lower[row][row];
        if !solution[row].is_finite() {
            return Err(FusionError::NonFiniteValue);
        }
    }
    Ok(solution)
}

/// Solve A X = B for SPD A using Cholesky substitutions. B has dimension rows
/// and column_count right-hand sides; no explicit inverse is formed.
fn solve_spd_matrix(
    matrix: &Matrix3,
    right_hand_side: &Matrix3,
    dimension: usize,
    column_count: usize,
) -> Result<Matrix3, FusionError> {
    if column_count == 0 || column_count > CONTACT_STATE_DIM {
        return Err(FusionError::InvalidCovarianceShape);
    }
    let mut result = zero_matrix();
    for column in 0..column_count {
        let mut vector = [0.0; 3];
        for row in 0..dimension {
            vector[row] = right_hand_side[row][column];
        }
        let solution = solve_spd_vector(matrix, &vector, dimension)?;
        for (row, value) in solution.iter().copied().enumerate().take(dimension) {
            result[row][column] = value;
        }
    }
    Ok(result)
}

fn log_determinant_spd(matrix: &Matrix3, dimension: usize) -> Result<f64, FusionError> {
    let lower = cholesky_lower(matrix, dimension)?;
    let mut log_determinant = 0.0;
    for (index, row) in lower.iter().enumerate().take(dimension) {
        log_determinant += 2.0 * row[index].ln();
    }
    if log_determinant.is_finite() {
        Ok(log_determinant)
    } else {
        Err(FusionError::NonFiniteValue)
    }
}

impl ContactGaussian {
    pub fn validate(&self) -> Result<(), FusionError> {
        if !all_finite(&self.mean) {
            return Err(FusionError::NonFiniteValue);
        }
        validate_covariance(&self.covariance, CONTACT_STATE_DIM)
    }
}

impl JointMeasurementModel {
    pub fn validate(&self) -> Result<(), FusionError> {
        if [
            self.model_id.as_str(),
            self.action_id.as_str(),
            self.actuator_model_digest.as_str(),
            self.calibration_digest.as_str(),
            self.observation_model_digest.as_str(),
            self.covariance_calibration_id.as_str(),
            self.channel_order_manifest_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(FusionError::EmptyIdentifier);
        }

        let count = self.channels.len();
        if count == 0 || count > MAX_CHANNELS {
            return Err(FusionError::InvalidChannelCount);
        }
        if self.channel_order_manifest.len() != count {
            return Err(FusionError::ChannelManifestMismatch);
        }

        let mut previous_sensor: Option<&str> = None;
        for (index, channel) in self.channels.iter().enumerate() {
            if [
                channel.sensor_id.as_str(),
                channel.coordinate_frame_id.as_str(),
                channel.unit_id.as_str(),
            ]
            .iter()
            .any(|value| !nonblank(value))
            {
                return Err(FusionError::EmptyIdentifier);
            }
            if !all_finite(&channel.observation_row) {
                return Err(FusionError::NonFiniteValue);
            }
            if self.channel_order_manifest[index] != channel.sensor_id {
                return Err(FusionError::ChannelManifestMismatch);
            }
            if let Some(previous) = previous_sensor {
                if previous == channel.sensor_id {
                    return Err(FusionError::DuplicateSensorId);
                }
                if previous > channel.sensor_id.as_str() {
                    return Err(FusionError::ChannelOrderNotCanonical);
                }
            }
            previous_sensor = Some(&channel.sensor_id);
        }

        validate_covariance(&self.noise_covariance, count)?;
        for row in 0..MAX_CHANNELS {
            for column in 0..MAX_CHANNELS {
                if (row >= count || column >= count) && self.noise_covariance[row][column] != 0.0 {
                    return Err(FusionError::InvalidCovarianceShape);
                }
            }
        }
        Ok(())
    }
}

impl ActionExecutionReceipt {
    pub fn validate(&self) -> Result<(), FusionError> {
        if [
            self.receipt_id.as_str(),
            self.command_action_id.as_str(),
            self.delivered_action_id.as_str(),
            self.measured_actuation_digest.as_str(),
            self.actuator_model_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(FusionError::EmptyIdentifier);
        }
        if !self.delivery_verified {
            return Err(FusionError::ActionReceiptNotVerified);
        }
        Ok(())
    }
}

impl ClockMappingReceipt {
    pub fn validate_for_sample(&self, sample_time_ns: u64) -> Result<(), FusionError> {
        if [
            self.receipt_id.as_str(),
            self.clock_id.as_str(),
            self.mapping_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(FusionError::EmptyIdentifier);
        }
        if !self.verified {
            return Err(FusionError::ClockReceiptNotVerified);
        }
        if self.valid_from_ns > self.valid_until_ns
            || sample_time_ns < self.valid_from_ns
            || sample_time_ns > self.valid_until_ns
        {
            return Err(FusionError::ClockMappingOutsideValidity);
        }
        Ok(())
    }
}

fn matrix_product(
    left: &Matrix3,
    right: &Matrix3,
    left_rows: usize,
    shared: usize,
    right_columns: usize,
) -> Result<Matrix3, FusionError> {
    let mut result = zero_matrix();
    for row in 0..left_rows {
        for column in 0..right_columns {
            let mut sum = 0.0;
            for inner in 0..shared {
                sum += left[row][inner] * right[inner][column];
            }
            if !sum.is_finite() {
                return Err(FusionError::NonFiniteValue);
            }
            result[row][column] = sum;
        }
    }
    Ok(result)
}

/// Fuse one synchronized multichannel observation group using a full joint
/// covariance. Samples are aligned by stable sensor ID to the model's canonical
/// channel order.
///
/// For x ~ N(mu,P), y = Hx + epsilon, epsilon ~ N(0,R):
///   S = H P H^T + R
///   K = P H^T S^-1
///   mu+ = mu + K(y - H mu)
///   P+ = P - K H P
///   EIG = 0.5 * (logdet(S) - logdet(R))
///
/// S inverse is never formed explicitly; Cholesky solves are used for K.
pub fn update_joint_contact_belief(
    prior: &ContactGaussian,
    model: &JointMeasurementModel,
    group: &JointObservationGroup,
    action_receipt: &ActionExecutionReceipt,
    clock_receipt: &ClockMappingReceipt,
    evaluated_at_ns: u64,
    maximum_sample_age_ns: u64,
) -> Result<JointFusionReport, FusionError> {
    prior.validate()?;
    model.validate()?;
    action_receipt.validate()?;

    if group.samples.is_empty() {
        return Err(FusionError::InvalidChannelCount);
    }
    if group.channel_order_manifest_digest != model.channel_order_manifest_digest {
        return Err(FusionError::ChannelManifestDigestMismatch);
    }
    if action_receipt.command_action_id != model.action_id {
        return Err(FusionError::ActionReceiptMismatch);
    }
    if action_receipt.actuator_model_digest != model.actuator_model_digest {
        return Err(FusionError::ActuatorModelMismatch);
    }

    let channel_count = model.channels.len();
    if group.samples.len() != channel_count {
        if group.samples.len() < channel_count {
            return Err(FusionError::MissingSensor(
                model
                    .channels
                    .iter()
                    .find(|channel| {
                        !group
                            .samples
                            .iter()
                            .any(|sample| sample.sensor_id == channel.sensor_id)
                    })
                    .map(|channel| channel.sensor_id.clone())
                    .unwrap_or_default(),
            ));
        }
        return Err(FusionError::UnknownSensor(
            group
                .samples
                .iter()
                .find(|sample| {
                    !model
                        .channels
                        .iter()
                        .any(|channel| channel.sensor_id == sample.sensor_id)
                })
                .map(|sample| sample.sensor_id.clone())
                .unwrap_or_default(),
        ));
    }

    let mut evidence_ids = BTreeSet::new();
    let mut sample_sensor_ids = BTreeSet::new();
    let first = &group.samples[0];
    if [
        first.contact_event_id.as_str(),
        first.foot_id.as_str(),
        first.clock_id.as_str(),
        first.clock_mapping_receipt_id.as_str(),
        first.action_execution_receipt_id.as_str(),
    ]
    .iter()
    .any(|value| !nonblank(value))
    {
        return Err(FusionError::EmptyIdentifier);
    }
    if !first.timebase_verified {
        return Err(FusionError::TimebaseNotVerified);
    }
    if evaluated_at_ns < first.sample_time_ns {
        return Err(FusionError::FutureObservation);
    }

    clock_receipt.validate_for_sample(first.sample_time_ns)?;
    if first.clock_id != clock_receipt.clock_id {
        return Err(FusionError::ClockMismatch);
    }
    if first.clock_mapping_receipt_id != clock_receipt.receipt_id {
        return Err(FusionError::ClockMappingReceiptMismatch);
    }
    if first.action_execution_receipt_id != action_receipt.receipt_id {
        return Err(FusionError::ActionReceiptMismatch);
    }
    if first.arrival_time_ns < first.sample_time_ns {
        return Err(FusionError::InvalidTimestampOrder);
    }
    if evaluated_at_ns < first.arrival_time_ns {
        return Err(FusionError::FutureObservation);
    }
    if evaluated_at_ns - first.sample_time_ns > maximum_sample_age_ns {
        return Err(FusionError::StaleObservation);
    }

    let mut y = [0.0; 3];
    let mut observed = vec![false; channel_count];

    for sample in &group.samples {
        if [
            sample.evidence_id.as_str(),
            sample.contact_event_id.as_str(),
            sample.foot_id.as_str(),
            sample.sensor_id.as_str(),
            sample.measurement_model_id.as_str(),
            sample.action_execution_receipt_id.as_str(),
            sample.clock_id.as_str(),
            sample.clock_mapping_receipt_id.as_str(),
            sample.coordinate_frame_id.as_str(),
            sample.unit_id.as_str(),
            sample.calibration_digest.as_str(),
            sample.observation_model_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(FusionError::EmptyIdentifier);
        }
        if !sample.value.is_finite() {
            return Err(FusionError::NonFiniteValue);
        }
        if !sample.timebase_verified {
            return Err(FusionError::TimebaseNotVerified);
        }
        if !evidence_ids.insert(sample.evidence_id.clone()) {
            return Err(FusionError::DuplicateEvidenceId);
        }
        if !sample_sensor_ids.insert(sample.sensor_id.clone()) {
            return Err(FusionError::DuplicateSensorId);
        }
        if sample.contact_event_id != first.contact_event_id {
            return Err(FusionError::ContactEventMismatch);
        }
        if sample.foot_id != first.foot_id {
            return Err(FusionError::FootIdentityMismatch);
        }
        if sample.clock_id != first.clock_id {
            return Err(FusionError::ClockMismatch);
        }
        if sample.clock_mapping_receipt_id != clock_receipt.receipt_id {
            return Err(FusionError::ClockMappingReceiptMismatch);
        }
        if sample.action_execution_receipt_id != action_receipt.receipt_id {
            return Err(FusionError::ActionReceiptMismatch);
        }
        if sample.sample_time_ns != first.sample_time_ns {
            return Err(FusionError::AsynchronousSampleTime);
        }
        if sample.arrival_time_ns < sample.sample_time_ns {
            return Err(FusionError::InvalidTimestampOrder);
        }
        if evaluated_at_ns < sample.arrival_time_ns {
            return Err(FusionError::FutureObservation);
        }
        if evaluated_at_ns - sample.sample_time_ns > maximum_sample_age_ns {
            return Err(FusionError::StaleObservation);
        }
        if sample.measurement_model_id != model.model_id {
            return Err(FusionError::ModelIdentityMismatch(sample.sensor_id.clone()));
        }
        if sample.calibration_digest != model.calibration_digest {
            return Err(FusionError::CalibrationIdentityMismatch(
                sample.sensor_id.clone(),
            ));
        }
        if sample.observation_model_digest != model.observation_model_digest {
            return Err(FusionError::ObservationModelIdentityMismatch(
                sample.sensor_id.clone(),
            ));
        }

        let channel_index = model
            .channels
            .iter()
            .position(|channel| channel.sensor_id == sample.sensor_id)
            .ok_or_else(|| FusionError::UnknownSensor(sample.sensor_id.clone()))?;
        let declared_channel = &model.channels[channel_index];
        if declared_channel.channel != sample.channel {
            return Err(FusionError::SensorChannelMismatch(sample.sensor_id.clone()));
        }
        if declared_channel.coordinate_frame_id != sample.coordinate_frame_id {
            return Err(FusionError::CoordinateFrameMismatch(
                sample.sensor_id.clone(),
            ));
        }
        if declared_channel.unit_id != sample.unit_id {
            return Err(FusionError::UnitMismatch(sample.sensor_id.clone()));
        }
        if observed[channel_index] {
            return Err(FusionError::DuplicateSensorId);
        }
        observed[channel_index] = true;
        y[channel_index] = sample.value;
    }

    for (index, channel) in model.channels.iter().enumerate() {
        if !observed[index] {
            return Err(FusionError::MissingSensor(channel.sensor_id.clone()));
        }
    }

    let mut h = zero_matrix();
    for (row, channel) in model.channels.iter().enumerate() {
        h[row] = channel.observation_row;
    }

    // S = H P H^T + R.
    let h_p = matrix_product(
        &h,
        &prior.covariance,
        channel_count,
        CONTACT_STATE_DIM,
        CONTACT_STATE_DIM,
    )?;
    let mut h_transpose = zero_matrix();
    for row in 0..CONTACT_STATE_DIM {
        for column in 0..channel_count {
            h_transpose[row][column] = h[column][row];
        }
    }
    let h_p_h_t = matrix_product(
        &h_p,
        &h_transpose,
        channel_count,
        CONTACT_STATE_DIM,
        channel_count,
    )?;
    let mut innovation_covariance = zero_matrix();
    for row in 0..channel_count {
        for column in 0..channel_count {
            innovation_covariance[row][column] =
                h_p_h_t[row][column] + model.noise_covariance[row][column];
        }
    }
    validate_covariance(&innovation_covariance, channel_count)?;

    // Cross = P H^T (state x channels).
    let cross = matrix_product(
        &prior.covariance,
        &h_transpose,
        CONTACT_STATE_DIM,
        CONTACT_STATE_DIM,
        channel_count,
    )?;

    // Solve S K^T = (P H^T)^T; do not form S inverse.
    let mut cross_transpose = zero_matrix();
    for row in 0..channel_count {
        for column in 0..CONTACT_STATE_DIM {
            cross_transpose[row][column] = cross[column][row];
        }
    }
    let gain_transpose = solve_spd_matrix(
        &innovation_covariance,
        &cross_transpose,
        channel_count,
        CONTACT_STATE_DIM,
    )?;
    let mut gain = zero_matrix();
    for row in 0..CONTACT_STATE_DIM {
        for column in 0..channel_count {
            gain[row][column] = gain_transpose[column][row];
        }
    }

    let mut innovation = [0.0; 3];
    for row in 0..channel_count {
        let predicted = model.channels[row]
            .observation_row
            .iter()
            .zip(prior.mean)
            .map(|(coefficient, mean)| coefficient * mean)
            .sum::<f64>();
        innovation[row] = y[row] - predicted;
    }
    if !all_finite(&innovation[..channel_count]) {
        return Err(FusionError::NonFiniteValue);
    }

    let mut posterior_mean = prior.mean;
    for row in 0..CONTACT_STATE_DIM {
        for column in 0..channel_count {
            posterior_mean[row] += gain[row][column] * innovation[column];
        }
    }
    if !all_finite(&posterior_mean) {
        return Err(FusionError::NonFiniteValue);
    }

    // P+ = P - K H P, followed by a symmetry projection and SPD validation.
    let k_h = matrix_product(
        &gain,
        &h,
        CONTACT_STATE_DIM,
        channel_count,
        CONTACT_STATE_DIM,
    )?;
    let k_h_p = matrix_product(
        &k_h,
        &prior.covariance,
        CONTACT_STATE_DIM,
        CONTACT_STATE_DIM,
        CONTACT_STATE_DIM,
    )?;
    let mut posterior_covariance = zero_matrix();
    for (row, posterior_row) in posterior_covariance.iter_mut().enumerate() {
        for (column, posterior_value) in posterior_row.iter_mut().enumerate() {
            *posterior_value = prior.covariance[row][column] - k_h_p[row][column];
        }
    }
    let mut symmetric_covariance = posterior_covariance;
    for (row, row_values) in posterior_covariance.iter().enumerate().skip(1) {
        for (column, value) in row_values.iter().copied().enumerate().take(row) {
            let average = 0.5 * (value + posterior_covariance[column][row]);
            symmetric_covariance[row][column] = average;
            symmetric_covariance[column][row] = average;
        }
    }
    posterior_covariance = symmetric_covariance;
    validate_covariance(&posterior_covariance, CONTACT_STATE_DIM)?;

    let expected_information_gain_nats = 0.5
        * (log_determinant_spd(&innovation_covariance, channel_count)?
            - log_determinant_spd(&model.noise_covariance, channel_count)?);
    if !expected_information_gain_nats.is_finite()
        || expected_information_gain_nats < -NEGATIVE_EIG_TOLERANCE
    {
        return Err(FusionError::InvalidInformationGain);
    }

    let mut maximum_sample_age_ns = 0;
    for sample in &group.samples {
        maximum_sample_age_ns = maximum_sample_age_ns.max(evaluated_at_ns - sample.sample_time_ns);
    }

    Ok(JointFusionReport {
        posterior: ContactGaussian {
            mean: posterior_mean,
            covariance: posterior_covariance,
        },
        expected_information_gain_nats: expected_information_gain_nats.max(0.0),
        model_id: model.model_id.clone(),
        action_id: model.action_id.clone(),
        contact_event_id: first.contact_event_id.clone(),
        foot_id: first.foot_id.clone(),
        action_execution_receipt_id: action_receipt.receipt_id.clone(),
        clock_mapping_receipt_id: clock_receipt.receipt_id.clone(),
        clock_id: clock_receipt.clock_id.clone(),
        common_sample_time_ns: first.sample_time_ns,
        evaluated_at_ns,
        maximum_sample_age_ns,
        channel_count,
        ordered_sensor_ids: model
            .channels
            .iter()
            .map(|channel| channel.sensor_id.clone())
            .collect(),
        covariance_calibration_id: model.covariance_calibration_id.clone(),
        channel_order_manifest_digest: model.channel_order_manifest_digest.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity_covariance() -> Matrix3 {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    }

    fn prior() -> ContactGaussian {
        ContactGaussian {
            mean: [0.0; CONTACT_STATE_DIM],
            covariance: identity_covariance(),
        }
    }

    fn model(channels: Vec<ChannelModel>, noise_covariance: Matrix3) -> JointMeasurementModel {
        let channel_order_manifest = channels
            .iter()
            .map(|channel| channel.sensor_id.clone())
            .collect::<Vec<_>>();
        JointMeasurementModel {
            model_id: "contact-model-v1".to_owned(),
            action_id: "stance-action-17".to_owned(),
            actuator_model_digest: "actuator-map-v1".to_owned(),
            calibration_digest: "sensor-cal-v1".to_owned(),
            observation_model_digest: "observation-map-v1".to_owned(),
            covariance_calibration_id: "cross-cov-v1".to_owned(),
            channel_order_manifest,
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
            channels,
            noise_covariance,
        }
    }

    fn channel(sensor_id: &str, sensor_channel: SensorChannel, row: [f64; 3]) -> ChannelModel {
        ChannelModel {
            sensor_id: sensor_id.to_owned(),
            channel: sensor_channel,
            coordinate_frame_id: "foot-frame-v1".to_owned(),
            unit_id: format!("unit-{sensor_id}"),
            observation_row: row,
        }
    }

    fn action_receipt() -> ActionExecutionReceipt {
        ActionExecutionReceipt {
            receipt_id: "action-receipt-17".to_owned(),
            command_action_id: "stance-action-17".to_owned(),
            delivered_action_id: "delivered-stance-17".to_owned(),
            measured_actuation_digest: "measured-actuation-v1".to_owned(),
            actuator_model_digest: "actuator-map-v1".to_owned(),
            delivery_verified: true,
        }
    }

    fn clock_receipt() -> ClockMappingReceipt {
        ClockMappingReceipt {
            receipt_id: "clock-map-v1".to_owned(),
            clock_id: "robot-monotonic-v1".to_owned(),
            mapping_digest: "clock-map-digest-v1".to_owned(),
            valid_from_ns: 1_000,
            valid_until_ns: 10_000,
            verified: true,
        }
    }

    fn sample(
        sensor_id: &str,
        sensor_channel: SensorChannel,
        value: f64,
    ) -> JointObservationSample {
        JointObservationSample {
            evidence_id: format!("evidence-{sensor_id}"),
            contact_event_id: "contact-event-7".to_owned(),
            foot_id: "front-left".to_owned(),
            sensor_id: sensor_id.to_owned(),
            channel: sensor_channel,
            measurement_model_id: "contact-model-v1".to_owned(),
            action_execution_receipt_id: "action-receipt-17".to_owned(),
            clock_id: "robot-monotonic-v1".to_owned(),
            clock_mapping_receipt_id: "clock-map-v1".to_owned(),
            coordinate_frame_id: "foot-frame-v1".to_owned(),
            unit_id: format!("unit-{sensor_id}"),
            calibration_digest: "sensor-cal-v1".to_owned(),
            observation_model_digest: "observation-map-v1".to_owned(),
            sample_time_ns: 5_000,
            arrival_time_ns: 5_100,
            timebase_verified: true,
            value,
        }
    }

    fn update(
        measurement_model: &JointMeasurementModel,
        samples: Vec<JointObservationSample>,
    ) -> Result<JointFusionReport, FusionError> {
        let group = JointObservationGroup {
            samples,
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        update_joint_contact_belief(
            &prior(),
            measurement_model,
            &group,
            &action_receipt(),
            &clock_receipt(),
            5_200,
            500,
        )
    }

    #[test]
    fn one_channel_update_matches_scalar_gaussian_reference() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let report = update(
            &measurement_model,
            vec![sample("sensor-a", SensorChannel::D1Interface, 2.0)],
        )
        .unwrap();
        assert!((report.posterior.mean[0] - 1.0).abs() < 1.0e-12);
        assert!((report.posterior.covariance[0][0] - 0.5).abs() < 1.0e-12);
        assert!((report.expected_information_gain_nats - 0.5 * 2.0_f64.ln()).abs() < 1.0e-12);
    }

    #[test]
    fn diagonal_joint_covariance_matches_independent_scalar_updates() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.0, 0.0], [0.0, 2.0, 0.0], [0.0; 3]],
        );
        let report = update(
            &measurement_model,
            vec![
                sample("sensor-a", SensorChannel::D1Interface, 1.0),
                sample("sensor-b", SensorChannel::D2NearFootWrench, 2.0),
            ],
        )
        .unwrap();

        assert!((report.posterior.mean[0] - 0.5).abs() < 1.0e-12);
        assert!((report.posterior.mean[1] - (2.0 / 3.0)).abs() < 1.0e-12);
        assert!((report.posterior.covariance[0][0] - 0.5).abs() < 1.0e-12);
        assert!((report.posterior.covariance[1][1] - (2.0 / 3.0)).abs() < 1.0e-12);
        assert!(report.posterior.covariance[0][1].abs() < 1.0e-12);
        assert!(
            (report.expected_information_gain_nats - 0.5 * (2.0_f64.ln() + 1.5_f64.ln())).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn correlated_channels_do_not_double_count_as_independent() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [1.0, 0.0, 0.0]),
            ],
            [[1.0, 0.9, 0.0], [0.9, 1.0, 0.0], [0.0; 3]],
        );
        let report = update(
            &measurement_model,
            vec![
                sample("sensor-a", SensorChannel::D1Interface, 1.0),
                sample("sensor-b", SensorChannel::D2NearFootWrench, 1.0),
            ],
        )
        .unwrap();

        let correct_variance = report.posterior.covariance[0][0];
        let incorrect_independent_variance = 1.0 / 3.0;
        assert!(correct_variance > incorrect_independent_variance);
        assert!((correct_variance - (1.0 / (1.0 + (2.0 / 1.9)))).abs() < 1.0e-10);
    }

    #[test]
    fn sample_order_is_canonical_and_does_not_change_posterior_or_eig() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2VibroAcoustic, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.25, 0.0], [0.25, 1.0, 0.0], [0.0; 3]],
        );
        let a = sample("sensor-a", SensorChannel::D1Interface, 0.25);
        let b = sample("sensor-b", SensorChannel::D2VibroAcoustic, -0.5);
        let first = update(&measurement_model, vec![a.clone(), b.clone()]).unwrap();
        let second = update(&measurement_model, vec![b, a]).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn asymmetric_or_non_positive_noise_covariance_is_rejected() {
        let asymmetric = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.1, 0.0], [0.2, 1.0, 0.0], [0.0; 3]],
        );
        assert_eq!(
            asymmetric.validate(),
            Err(FusionError::CovarianceNotSymmetric)
        );

        let singular = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0; 3]],
        );
        assert_eq!(
            singular.validate(),
            Err(FusionError::CovarianceNotPositiveDefinite)
        );
    }

    #[test]
    fn missing_channel_is_not_imputed_as_a_zero_measurement() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 3]],
        );
        let result = update(
            &measurement_model,
            vec![sample("sensor-a", SensorChannel::D1Interface, 0.0)],
        );
        assert_eq!(
            result,
            Err(FusionError::MissingSensor("sensor-b".to_owned()))
        );
    }

    #[test]
    fn asynchronous_acquisition_times_are_rejected_even_when_arrival_is_close() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 3]],
        );
        let mut b = sample("sensor-b", SensorChannel::D2NearFootWrench, 0.0);
        b.sample_time_ns += 1;
        b.arrival_time_ns += 1;
        assert_eq!(
            update(
                &measurement_model,
                vec![sample("sensor-a", SensorChannel::D1Interface, 0.0), b]
            ),
            Err(FusionError::AsynchronousSampleTime)
        );
    }

    #[test]
    fn mismatched_clock_mapping_receipt_is_rejected() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let mut a = sample("sensor-a", SensorChannel::D1Interface, 0.0);
        a.clock_mapping_receipt_id = "other-clock-map".to_owned();
        assert_eq!(
            update(&measurement_model, vec![a]),
            Err(FusionError::ClockMappingReceiptMismatch)
        );
    }

    #[test]
    fn unverified_clock_mapping_receipt_is_rejected() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let mut receipt = clock_receipt();
        receipt.verified = false;
        let group = JointObservationGroup {
            samples: vec![sample("sensor-a", SensorChannel::D1Interface, 0.0)],
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        assert_eq!(
            update_joint_contact_belief(
                &prior(),
                &measurement_model,
                &group,
                &action_receipt(),
                &receipt,
                5_200,
                500,
            ),
            Err(FusionError::ClockReceiptNotVerified)
        );
    }

    #[test]
    fn stale_samples_and_arrival_before_sample_are_rejected() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let group = JointObservationGroup {
            samples: vec![sample("sensor-a", SensorChannel::D1Interface, 0.0)],
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        assert_eq!(
            update_joint_contact_belief(
                &prior(),
                &measurement_model,
                &group,
                &action_receipt(),
                &clock_receipt(),
                6_000,
                500,
            ),
            Err(FusionError::StaleObservation)
        );

        let mut a = sample("sensor-a", SensorChannel::D1Interface, 0.0);
        a.arrival_time_ns = a.sample_time_ns - 1;
        assert_eq!(
            update(&measurement_model, vec![a]),
            Err(FusionError::InvalidTimestampOrder)
        );
    }

    #[test]
    fn action_execution_receipt_and_actuator_model_are_bound() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let group = JointObservationGroup {
            samples: vec![sample("sensor-a", SensorChannel::D1Interface, 0.0)],
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        let mut receipt = action_receipt();
        receipt.delivery_verified = false;
        assert_eq!(
            update_joint_contact_belief(
                &prior(),
                &measurement_model,
                &group,
                &receipt,
                &clock_receipt(),
                5_200,
                500,
            ),
            Err(FusionError::ActionReceiptNotVerified)
        );
    }

    #[test]
    fn covariance_channel_manifest_mismatch_is_rejected() {
        let mut measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.2, 0.0], [0.2, 1.0, 0.0], [0.0; 3]],
        );
        measurement_model.channel_order_manifest[0] = "sensor-b".to_owned();
        assert_eq!(
            measurement_model.validate(),
            Err(FusionError::ChannelManifestMismatch)
        );
    }

    #[test]
    fn action_model_and_calibration_digest_mismatch_are_rejected() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let mut a = sample("sensor-a", SensorChannel::D1Interface, 0.0);
        a.measurement_model_id = "other-model".to_owned();
        assert_eq!(
            update(&measurement_model, vec![a]),
            Err(FusionError::ModelIdentityMismatch("sensor-a".to_owned()))
        );

        let mut a = sample("sensor-a", SensorChannel::D1Interface, 0.0);
        a.calibration_digest = "other-calibration".to_owned();
        assert_eq!(
            update(&measurement_model, vec![a]),
            Err(FusionError::CalibrationIdentityMismatch(
                "sensor-a".to_owned()
            ))
        );
    }

    #[test]
    fn duplicate_sensor_and_evidence_ids_are_rejected() {
        let measurement_model = model(
            vec![
                channel("sensor-a", SensorChannel::D1Interface, [1.0, 0.0, 0.0]),
                channel("sensor-b", SensorChannel::D2NearFootWrench, [0.0, 1.0, 0.0]),
            ],
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0; 3]],
        );
        let mut duplicate = sample("sensor-b", SensorChannel::D2NearFootWrench, 0.0);
        duplicate.sensor_id = "sensor-a".to_owned();
        assert_eq!(
            update(
                &measurement_model,
                vec![
                    sample("sensor-a", SensorChannel::D1Interface, 0.0),
                    duplicate
                ]
            ),
            Err(FusionError::DuplicateSensorId)
        );

        let mut duplicate = sample("sensor-b", SensorChannel::D2NearFootWrench, 0.0);
        duplicate.evidence_id = "evidence-sensor-a".to_owned();
        assert_eq!(
            update(
                &measurement_model,
                vec![
                    sample("sensor-a", SensorChannel::D1Interface, 0.0),
                    duplicate
                ]
            ),
            Err(FusionError::DuplicateEvidenceId)
        );
    }

    #[test]
    fn covariance_calibration_identity_is_required() {
        let mut measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        measurement_model.covariance_calibration_id.clear();
        assert_eq!(
            measurement_model.validate(),
            Err(FusionError::EmptyIdentifier)
        );
    }

    #[test]
    fn sample_age_and_clock_mapping_validity_boundaries_are_inclusive() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let group = JointObservationGroup {
            samples: vec![sample("sensor-a", SensorChannel::D1Interface, 0.0)],
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        let mut clock = clock_receipt();
        clock.valid_from_ns = 5_000;
        clock.valid_until_ns = 5_200;
        let report = update_joint_contact_belief(
            &prior(),
            &measurement_model,
            &group,
            &action_receipt(),
            &clock,
            5_200,
            200,
        )
        .unwrap();
        assert_eq!(report.maximum_sample_age_ns, 200);
    }

    #[test]
    fn nonfinite_values_and_invalid_prior_are_rejected() {
        let measurement_model = model(
            vec![channel(
                "sensor-a",
                SensorChannel::D1Interface,
                [1.0, 0.0, 0.0],
            )],
            [[1.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
        );
        let mut a = sample("sensor-a", SensorChannel::D1Interface, f64::NAN);
        assert_eq!(
            update(&measurement_model, vec![a.clone()]),
            Err(FusionError::NonFiniteValue)
        );
        a.value = 0.0;

        let mut invalid_prior = prior();
        invalid_prior.covariance[0][0] = -1.0;
        let group = JointObservationGroup {
            samples: vec![a],
            channel_order_manifest_digest: "canonical-channels-v1".to_owned(),
        };
        assert_eq!(
            update_joint_contact_belief(
                &invalid_prior,
                &measurement_model,
                &group,
                &action_receipt(),
                &clock_receipt(),
                5_200,
                500,
            ),
            Err(FusionError::CovarianceNotPositiveDefinite)
        );
    }

    #[test]
    fn logdet_and_cholesky_solver_match_diagonal_known_answer() {
        let matrix = [[4.0, 0.0, 0.0], [0.0, 9.0, 0.0], [0.0; 3]];
        let logdet = log_determinant_spd(&matrix, 2).unwrap();
        assert!((logdet - 4.0_f64.ln() - 9.0_f64.ln()).abs() < 1.0e-12);
        let solution = solve_spd_vector(&matrix, &[8.0, 18.0, 0.0], 2).unwrap();
        assert!((solution[0] - 2.0).abs() < 1.0e-12);
        assert!((solution[1] - 2.0).abs() < 1.0e-12);
    }
}
