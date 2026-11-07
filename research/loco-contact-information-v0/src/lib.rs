//! Action-conditioned contact information and safe haptic exploration.
//!
//! V0 is a fixed three-dimensional linear-Gaussian reference oracle. Observation
//! rows and noise variances are supplied by a separately calibrated model; this
//! package does not claim to estimate physical friction or support directly.

use std::collections::BTreeSet;

pub const CONTACT_STATE_DIM: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorChannel {
    D1Interface,
    D2NearFootWrench,
    D2VibroAcoustic,
    D3Proprioceptive,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactGaussian {
    pub mean: [f64; CONTACT_STATE_DIM],
    pub covariance: [[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM],
}

#[derive(Debug, Clone, PartialEq)]
pub struct MeasurementModel {
    pub model_id: String,
    pub sensor_id: String,
    pub channel: SensorChannel,
    pub observation_row: [f64; CONTACT_STATE_DIM],
    pub noise_variance: f64,
    /// True only when the conditional independence assumption is qualified.
    pub independent_noise_verified: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContactObservation {
    pub evidence_id: String,
    pub contact_event_id: String,
    pub foot_id: String,
    pub sensor_id: String,
    pub channel: SensorChannel,
    pub measurement_model_id: String,
    pub clock_id: String,
    pub sample_time_ns: u64,
    pub arrival_time_ns: u64,
    pub timebase_verified: bool,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ActionEnvelope {
    pub energy_upper_bound_joules: f64,
    pub support_margin_lower_bound: f64,
    pub peak_normal_force_upper_bound_newtons: f64,
    pub impact_impulse_upper_bound_newton_seconds: f64,
    pub actuator_load_fraction_upper_bound: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SafetyLimits {
    pub maximum_energy_joules: f64,
    pub minimum_support_margin: f64,
    pub maximum_peak_normal_force_newtons: f64,
    pub maximum_impact_impulse_newton_seconds: f64,
    pub maximum_actuator_load_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionCandidate {
    pub action_id: String,
    pub envelope: ActionEnvelope,
    pub measurement_models: Vec<MeasurementModel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintViolation {
    EnergyBudgetExceeded,
    InsufficientSupportMargin,
    PeakNormalForceExceeded,
    ImpactImpulseExceeded,
    ActuatorLoadExceeded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionAssessment {
    pub action_id: String,
    pub expected_information_gain_nats: f64,
    pub information_nats_per_joule: f64,
    pub envelope: ActionEnvelope,
    pub feasible: bool,
    pub rejection: Option<ConstraintViolation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactInformationError {
    EmptyIdentifier,
    NoMeasurements,
    InvalidCovariance,
    NonFiniteValue,
    InvalidNoiseVariance,
    NoiseIndependenceNotVerified,
    DuplicateModelId,
    DuplicateEvidenceId,
    DuplicateMeasurementModel,
    UnknownMeasurementModel,
    SensorModelIdentityMismatch,
    ContactEventMismatch,
    FootIdentityMismatch,
    ClockMismatch,
    TimebaseNotVerified,
    InvalidTimestampOrder,
    FutureObservation,
    StaleObservation,
    InvalidActionEnvelope,
    InvalidSafetyLimits,
    DuplicateActionId,
    NumericalFailure,
}

impl ContactGaussian {
    pub fn validate(&self) -> Result<(), ContactInformationError> {
        if self.mean.iter().any(|value| !value.is_finite()) {
            return Err(ContactInformationError::NonFiniteValue);
        }
        validate_covariance(&self.covariance)
    }
}

impl MeasurementModel {
    pub fn validate(&self) -> Result<(), ContactInformationError> {
        if self.model_id.trim().is_empty() || self.sensor_id.trim().is_empty() {
            return Err(ContactInformationError::EmptyIdentifier);
        }
        if self.observation_row.iter().any(|value| !value.is_finite()) {
            return Err(ContactInformationError::NonFiniteValue);
        }
        if !self.noise_variance.is_finite() || self.noise_variance <= 0.0 {
            return Err(ContactInformationError::InvalidNoiseVariance);
        }
        if !self.independent_noise_verified {
            return Err(ContactInformationError::NoiseIndependenceNotVerified);
        }
        Ok(())
    }
}

impl SafetyLimits {
    pub fn validate(&self) -> Result<(), ContactInformationError> {
        if !self.maximum_energy_joules.is_finite()
            || self.maximum_energy_joules <= 0.0
            || !self.minimum_support_margin.is_finite()
            || !self.maximum_peak_normal_force_newtons.is_finite()
            || self.maximum_peak_normal_force_newtons <= 0.0
            || !self.maximum_impact_impulse_newton_seconds.is_finite()
            || self.maximum_impact_impulse_newton_seconds <= 0.0
            || !self.maximum_actuator_load_fraction.is_finite()
            || !(0.0..=1.0).contains(&self.maximum_actuator_load_fraction)
        {
            return Err(ContactInformationError::InvalidSafetyLimits);
        }
        Ok(())
    }
}

/// Expected information gain for a set of conditionally independent scalar
/// measurements. Models are sorted by stable model ID before accumulation.
pub fn expected_information_gain(
    prior: &ContactGaussian,
    models: &[MeasurementModel],
) -> Result<f64, ContactInformationError> {
    prior.validate()?;
    if models.is_empty() {
        return Err(ContactInformationError::NoMeasurements);
    }

    let ordered = validated_sorted_models(models)?;
    let mut covariance = prior.covariance;
    let mut total_nats = 0.0;

    for model in ordered {
        let (next_covariance, information_gain) = covariance_information_update(
            &covariance,
            &model.observation_row,
            model.noise_variance,
        )?;
        covariance = next_covariance;
        total_nats += information_gain;
    }

    if !total_nats.is_finite() || total_nats < 0.0 {
        return Err(ContactInformationError::NumericalFailure);
    }
    Ok(total_nats)
}

/// Update a prior using measured, time-qualified contact observations.
///
/// V0 accepts one sample per measurement model and one foot/contact event per
/// call. Repeated or correlated evidence is rejected rather than double-counted.
pub fn update_with_observations(
    prior: &ContactGaussian,
    models: &[MeasurementModel],
    observations: &[ContactObservation],
    now_ns: u64,
    maximum_sample_age_ns: u64,
) -> Result<ContactGaussian, ContactInformationError> {
    prior.validate()?;
    if observations.is_empty() {
        return Err(ContactInformationError::NoMeasurements);
    }

    let ordered_models = validated_sorted_models(models)?;
    let mut model_by_id = std::collections::BTreeMap::new();
    for model in ordered_models {
        model_by_id.insert(model.model_id.as_str(), model);
    }

    let mut seen_evidence = BTreeSet::new();
    let mut seen_models = BTreeSet::new();
    let first = &observations[0];
    if first.contact_event_id.trim().is_empty()
        || first.foot_id.trim().is_empty()
        || first.clock_id.trim().is_empty()
    {
        return Err(ContactInformationError::EmptyIdentifier);
    }

    let mut ordered_observations: Vec<&ContactObservation> = observations.iter().collect();
    for observation in &ordered_observations {
        if observation.evidence_id.trim().is_empty()
            || observation.contact_event_id.trim().is_empty()
            || observation.foot_id.trim().is_empty()
            || observation.sensor_id.trim().is_empty()
            || observation.measurement_model_id.trim().is_empty()
            || observation.clock_id.trim().is_empty()
        {
            return Err(ContactInformationError::EmptyIdentifier);
        }
        if !observation.value.is_finite() {
            return Err(ContactInformationError::NonFiniteValue);
        }
        if !observation.timebase_verified {
            return Err(ContactInformationError::TimebaseNotVerified);
        }
        if observation.sample_time_ns > observation.arrival_time_ns {
            return Err(ContactInformationError::InvalidTimestampOrder);
        }
        if observation.arrival_time_ns > now_ns {
            return Err(ContactInformationError::FutureObservation);
        }
        if now_ns.saturating_sub(observation.sample_time_ns) > maximum_sample_age_ns {
            return Err(ContactInformationError::StaleObservation);
        }
        if observation.contact_event_id != first.contact_event_id {
            return Err(ContactInformationError::ContactEventMismatch);
        }
        if observation.foot_id != first.foot_id {
            return Err(ContactInformationError::FootIdentityMismatch);
        }
        if observation.clock_id != first.clock_id {
            return Err(ContactInformationError::ClockMismatch);
        }
        if !seen_evidence.insert(observation.evidence_id.as_str()) {
            return Err(ContactInformationError::DuplicateEvidenceId);
        }
        if !seen_models.insert(observation.measurement_model_id.as_str()) {
            return Err(ContactInformationError::DuplicateMeasurementModel);
        }

        let model = model_by_id
            .get(observation.measurement_model_id.as_str())
            .ok_or(ContactInformationError::UnknownMeasurementModel)?;
        if model.sensor_id != observation.sensor_id || model.channel != observation.channel {
            return Err(ContactInformationError::SensorModelIdentityMismatch);
        }
    }

    ordered_observations.sort_by(|left, right| {
        left.sample_time_ns
            .cmp(&right.sample_time_ns)
            .then_with(|| left.sensor_id.cmp(&right.sensor_id))
            .then_with(|| left.evidence_id.cmp(&right.evidence_id))
    });

    let mut posterior = prior.clone();
    for observation in ordered_observations {
        let model = model_by_id[observation.measurement_model_id.as_str()];
        posterior = update_one(
            &posterior,
            &model.observation_row,
            model.noise_variance,
            observation.value,
        )?;
    }
    posterior.validate()?;
    Ok(posterior)
}

/// Assess actions using expected contact information and conservative supplied
/// physical envelopes. Hard constraints are evaluated before Pareto ranking.
pub fn assess_actions(
    prior: &ContactGaussian,
    candidates: &[ActionCandidate],
    limits: SafetyLimits,
) -> Result<Vec<ActionAssessment>, ContactInformationError> {
    prior.validate()?;
    limits.validate()?;

    let mut seen_action_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.action_id.trim().is_empty() {
            return Err(ContactInformationError::EmptyIdentifier);
        }
        if !seen_action_ids.insert(candidate.action_id.as_str()) {
            return Err(ContactInformationError::DuplicateActionId);
        }
        validate_action_envelope(candidate.envelope)?;
        let eig = expected_information_gain(prior, &candidate.measurement_models)?;
        let energy = candidate.envelope.energy_upper_bound_joules;
        let information_per_joule = eig / energy;
        if !information_per_joule.is_finite() || information_per_joule < 0.0 {
            return Err(ContactInformationError::NumericalFailure);
        }

        let rejection = if energy > limits.maximum_energy_joules {
            Some(ConstraintViolation::EnergyBudgetExceeded)
        } else if candidate.envelope.support_margin_lower_bound
            < limits.minimum_support_margin
        {
            Some(ConstraintViolation::InsufficientSupportMargin)
        } else if candidate.envelope.peak_normal_force_upper_bound_newtons
            > limits.maximum_peak_normal_force_newtons
        {
            Some(ConstraintViolation::PeakNormalForceExceeded)
        } else if candidate.envelope.impact_impulse_upper_bound_newton_seconds
            > limits.maximum_impact_impulse_newton_seconds
        {
            Some(ConstraintViolation::ImpactImpulseExceeded)
        } else if candidate.envelope.actuator_load_fraction_upper_bound
            > limits.maximum_actuator_load_fraction
        {
            Some(ConstraintViolation::ActuatorLoadExceeded)
        } else {
            None
        };

        assessments.push(ActionAssessment {
            action_id: candidate.action_id.clone(),
            expected_information_gain_nats: eig,
            information_nats_per_joule: information_per_joule,
            envelope: candidate.envelope,
            feasible: rejection.is_none(),
            rejection,
        });
    }

    assessments.sort_by(|left, right| left.action_id.cmp(&right.action_id));
    Ok(assessments)
}

/// Return feasible, non-dominated action IDs. Objectives are kept separate:
/// maximize information and support margin; minimize energy, peak force,
/// impact impulse and actuator load. No weighted aggregate is manufactured.
pub fn pareto_frontier_action_ids(assessments: &[ActionAssessment]) -> Vec<String> {
    let feasible: Vec<&ActionAssessment> = assessments
        .iter()
        .filter(|assessment| assessment.feasible && assessment.rejection.is_none())
        .collect();

    let mut frontier: Vec<String> = feasible
        .iter()
        .filter(|candidate| {
            !feasible
                .iter()
                .any(|other| other.action_id != candidate.action_id && dominates(other, candidate))
        })
        .map(|assessment| assessment.action_id.clone())
        .collect();
    frontier.sort();
    frontier
}

fn dominates(left: &ActionAssessment, right: &ActionAssessment) -> bool {
    let left_better_or_equal = left.expected_information_gain_nats
        >= right.expected_information_gain_nats
        && left.envelope.energy_upper_bound_joules <= right.envelope.energy_upper_bound_joules
        && left.envelope.support_margin_lower_bound >= right.envelope.support_margin_lower_bound
        && left.envelope.peak_normal_force_upper_bound_newtons
            <= right.envelope.peak_normal_force_upper_bound_newtons
        && left.envelope.impact_impulse_upper_bound_newton_seconds
            <= right.envelope.impact_impulse_upper_bound_newton_seconds
        && left.envelope.actuator_load_fraction_upper_bound
            <= right.envelope.actuator_load_fraction_upper_bound;

    let strictly_better = left.expected_information_gain_nats
        > right.expected_information_gain_nats
        || left.envelope.energy_upper_bound_joules < right.envelope.energy_upper_bound_joules
        || left.envelope.support_margin_lower_bound > right.envelope.support_margin_lower_bound
        || left.envelope.peak_normal_force_upper_bound_newtons
            < right.envelope.peak_normal_force_upper_bound_newtons
        || left.envelope.impact_impulse_upper_bound_newton_seconds
            < right.envelope.impact_impulse_upper_bound_newton_seconds
        || left.envelope.actuator_load_fraction_upper_bound
            < right.envelope.actuator_load_fraction_upper_bound;

    left_better_or_equal && strictly_better
}

fn validate_action_envelope(
    envelope: ActionEnvelope,
) -> Result<(), ContactInformationError> {
    if !envelope.energy_upper_bound_joules.is_finite()
        || envelope.energy_upper_bound_joules <= 0.0
        || !envelope.support_margin_lower_bound.is_finite()
        || !envelope.peak_normal_force_upper_bound_newtons.is_finite()
        || envelope.peak_normal_force_upper_bound_newtons < 0.0
        || !envelope.impact_impulse_upper_bound_newton_seconds.is_finite()
        || envelope.impact_impulse_upper_bound_newton_seconds < 0.0
        || !envelope.actuator_load_fraction_upper_bound.is_finite()
        || !(0.0..=1.0).contains(&envelope.actuator_load_fraction_upper_bound)
    {
        return Err(ContactInformationError::InvalidActionEnvelope);
    }
    Ok(())
}

fn validated_sorted_models(
    models: &[MeasurementModel],
) -> Result<Vec<&MeasurementModel>, ContactInformationError> {
    if models.is_empty() {
        return Err(ContactInformationError::NoMeasurements);
    }
    let mut ordered: Vec<&MeasurementModel> = models.iter().collect();
    for model in &ordered {
        model.validate()?;
    }
    ordered.sort_by(|left, right| left.model_id.cmp(&right.model_id));
    for pair in ordered.windows(2) {
        if pair[0].model_id == pair[1].model_id {
            return Err(ContactInformationError::DuplicateModelId);
        }
    }
    Ok(ordered)
}

fn update_one(
    prior: &ContactGaussian,
    observation_row: &[f64; CONTACT_STATE_DIM],
    noise_variance: f64,
    value: f64,
) -> Result<ContactGaussian, ContactInformationError> {
    prior.validate()?;
    if !noise_variance.is_finite() || noise_variance <= 0.0 {
        return Err(ContactInformationError::InvalidNoiseVariance);
    }
    if observation_row.iter().any(|entry| !entry.is_finite()) || !value.is_finite() {
        return Err(ContactInformationError::NonFiniteValue);
    }

    let covariance_times_row = mat_vec(&prior.covariance, observation_row);
    let projected_variance = dot(observation_row, &covariance_times_row);
    let innovation_variance = noise_variance + projected_variance;
    if !innovation_variance.is_finite() || innovation_variance <= 0.0 {
        return Err(ContactInformationError::NumericalFailure);
    }

    let predicted = dot(observation_row, &prior.mean);
    let innovation = value - predicted;
    let gain_scale = innovation / innovation_variance;

    let mut posterior = prior.clone();
    for i in 0..CONTACT_STATE_DIM {
        posterior.mean[i] += covariance_times_row[i] * gain_scale;
        for j in 0..CONTACT_STATE_DIM {
            posterior.covariance[i][j] -=
                covariance_times_row[i] * covariance_times_row[j] / innovation_variance;
        }
    }
    symmetrize(&mut posterior.covariance);
    posterior.validate()?;
    Ok(posterior)
}

fn covariance_information_update(
    covariance: &[[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM],
    observation_row: &[f64; CONTACT_STATE_DIM],
    noise_variance: f64,
) -> Result<([[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM], f64), ContactInformationError> {
    validate_covariance(covariance)?;
    if !noise_variance.is_finite() || noise_variance <= 0.0 {
        return Err(ContactInformationError::InvalidNoiseVariance);
    }
    if observation_row.iter().any(|entry| !entry.is_finite()) {
        return Err(ContactInformationError::NonFiniteValue);
    }

    let covariance_times_row = mat_vec(covariance, observation_row);
    let projected_variance = dot(observation_row, &covariance_times_row);
    if !projected_variance.is_finite() || projected_variance < -1.0e-12 {
        return Err(ContactInformationError::NumericalFailure);
    }
    let information = 0.5 * (1.0 + projected_variance.max(0.0) / noise_variance).ln();
    if !information.is_finite() || information < 0.0 {
        return Err(ContactInformationError::NumericalFailure);
    }

    let innovation_variance = noise_variance + projected_variance.max(0.0);
    let mut posterior = *covariance;
    for i in 0..CONTACT_STATE_DIM {
        for j in 0..CONTACT_STATE_DIM {
            posterior[i][j] -=
                covariance_times_row[i] * covariance_times_row[j] / innovation_variance;
        }
    }
    symmetrize(&mut posterior);
    validate_covariance(&posterior)?;
    Ok((posterior, information))
}

fn validate_covariance(
    covariance: &[[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM],
) -> Result<(), ContactInformationError> {
    for i in 0..CONTACT_STATE_DIM {
        for j in 0..CONTACT_STATE_DIM {
            if !covariance[i][j].is_finite() {
                return Err(ContactInformationError::InvalidCovariance);
            }
            let scale = 1.0_f64.max(covariance[i][j].abs()).max(covariance[j][i].abs());
            if (covariance[i][j] - covariance[j][i]).abs() > 1.0e-10 * scale {
                return Err(ContactInformationError::InvalidCovariance);
            }
        }
    }

    let mut lower = [[0.0; CONTACT_STATE_DIM]; CONTACT_STATE_DIM];
    for i in 0..CONTACT_STATE_DIM {
        for j in 0..=i {
            let mut sum = covariance[i][j];
            for k in 0..j {
                sum -= lower[i][k] * lower[j][k];
            }
            if i == j {
                if !sum.is_finite() || sum <= 0.0 {
                    return Err(ContactInformationError::InvalidCovariance);
                }
                lower[i][j] = sum.sqrt();
            } else {
                if lower[j][j] <= 0.0 {
                    return Err(ContactInformationError::InvalidCovariance);
                }
                lower[i][j] = sum / lower[j][j];
            }
        }
    }
    Ok(())
}

fn mat_vec(
    matrix: &[[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM],
    vector: &[f64; CONTACT_STATE_DIM],
) -> [f64; CONTACT_STATE_DIM] {
    let mut result = [0.0; CONTACT_STATE_DIM];
    for i in 0..CONTACT_STATE_DIM {
        for j in 0..CONTACT_STATE_DIM {
            result[i] += matrix[i][j] * vector[j];
        }
    }
    result
}

fn dot(
    left: &[f64; CONTACT_STATE_DIM],
    right: &[f64; CONTACT_STATE_DIM],
) -> f64 {
    left.iter().zip(right.iter()).map(|(a, b)| a * b).sum()
}

fn symmetrize(matrix: &mut [[f64; CONTACT_STATE_DIM]; CONTACT_STATE_DIM]) {
    for i in 0..CONTACT_STATE_DIM {
        for j in 0..i {
            let average = 0.5 * (matrix[i][j] + matrix[j][i]);
            matrix[i][j] = average;
            matrix[j][i] = average;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prior() -> ContactGaussian {
        ContactGaussian {
            mean: [0.0; CONTACT_STATE_DIM],
            covariance: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        }
    }

    fn model(
        model_id: &str,
        sensor_id: &str,
        channel: SensorChannel,
        observation_row: [f64; CONTACT_STATE_DIM],
        noise_variance: f64,
    ) -> MeasurementModel {
        MeasurementModel {
            model_id: model_id.to_owned(),
            sensor_id: sensor_id.to_owned(),
            channel,
            observation_row,
            noise_variance,
            independent_noise_verified: true,
        }
    }

    fn observation(
        evidence_id: &str,
        event_id: &str,
        foot_id: &str,
        measurement_model_id: &str,
        sensor_id: &str,
        channel: SensorChannel,
        value: f64,
        sample_time_ns: u64,
        arrival_time_ns: u64,
    ) -> ContactObservation {
        ContactObservation {
            evidence_id: evidence_id.to_owned(),
            contact_event_id: event_id.to_owned(),
            foot_id: foot_id.to_owned(),
            sensor_id: sensor_id.to_owned(),
            channel,
            measurement_model_id: measurement_model_id.to_owned(),
            clock_id: "physics-clock".to_owned(),
            sample_time_ns,
            arrival_time_ns,
            timebase_verified: true,
            value,
        }
    }

    fn limits() -> SafetyLimits {
        SafetyLimits {
            maximum_energy_joules: 5.0,
            minimum_support_margin: 0.05,
            maximum_peak_normal_force_newtons: 20.0,
            maximum_impact_impulse_newton_seconds: 2.0,
            maximum_actuator_load_fraction: 0.9,
        }
    }

    fn envelope(
        energy: f64,
        support: f64,
        force: f64,
        impulse: f64,
        actuator: f64,
    ) -> ActionEnvelope {
        ActionEnvelope {
            energy_upper_bound_joules: energy,
            support_margin_lower_bound: support,
            peak_normal_force_upper_bound_newtons: force,
            impact_impulse_upper_bound_newton_seconds: impulse,
            actuator_load_fraction_upper_bound: actuator,
        }
    }

    #[test]
    fn scalar_measurement_matches_exact_gaussian_posterior_and_eig() {
        let measurement_model = model(
            "model-x",
            "sensor-interface",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let eig = expected_information_gain(&prior(), std::slice::from_ref(&measurement_model))
            .unwrap();
        assert!((eig - 0.5 * 2.0_f64.ln()).abs() < 1.0e-12);

        let measured = observation(
            "evidence-1",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-interface",
            SensorChannel::D1Interface,
            2.0,
            100,
            101,
        );
        let posterior = update_with_observations(
            &prior(),
            &[measurement_model],
            &[measured],
            110,
            20,
        )
        .unwrap();
        assert!((posterior.mean[0] - 1.0).abs() < 1.0e-12);
        assert!((posterior.covariance[0][0] - 0.5).abs() < 1.0e-12);
        assert!((posterior.covariance[1][1] - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn independent_channels_compose_and_are_input_order_invariant() {
        let a = model(
            "model-a",
            "interface-sensor",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let b = model(
            "model-b",
            "joint-sensor",
            SensorChannel::D3Proprioceptive,
            [0.0, 1.0, 0.0],
            1.0,
        );
        let first = expected_information_gain(&prior(), &[a.clone(), b.clone()]).unwrap();
        let second = expected_information_gain(&prior(), &[b, a]).unwrap();
        assert!((first - 2.0 * (0.5 * 2.0_f64.ln())).abs() < 1.0e-12);
        assert!((first - second).abs() < 1.0e-12);
    }

    #[test]
    fn unobservable_measurement_has_zero_information_gain() {
        let m = model(
            "model-null",
            "sensor-null",
            SensorChannel::D2VibroAcoustic,
            [0.0, 0.0, 0.0],
            0.1,
        );
        assert_eq!(expected_information_gain(&prior(), &[m]).unwrap(), 0.0);
    }

    #[test]
    fn duplicate_evidence_id_is_rejected() {
        let m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let a = observation(
            "same-evidence",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            1.0,
            100,
            101,
        );
        let mut b = a.clone();
        b.value = 2.0;
        assert_eq!(
            update_with_observations(&prior(), &[m], &[a, b], 110, 20),
            Err(ContactInformationError::DuplicateEvidenceId)
        );
    }

    #[test]
    fn stale_observation_is_rejected_without_imputation() {
        let m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let o = observation(
            "evidence-1",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            1.0,
            10,
            11,
        );
        assert_eq!(
            update_with_observations(&prior(), &[m], &[o], 100, 20),
            Err(ContactInformationError::StaleObservation)
        );
    }

    #[test]
    fn unverified_timebase_is_rejected() {
        let m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let mut o = observation(
            "evidence-1",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            1.0,
            10,
            11,
        );
        o.timebase_verified = false;
        assert_eq!(
            update_with_observations(&prior(), &[m], &[o], 20, 20),
            Err(ContactInformationError::TimebaseNotVerified)
        );
    }

    #[test]
    fn mismatched_clock_ids_are_rejected() {
        let a = model(
            "model-a",
            "sensor-a",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let b = model(
            "model-b",
            "sensor-b",
            SensorChannel::D3Proprioceptive,
            [0.0, 1.0, 0.0],
            1.0,
        );
        let mut oa = observation(
            "ea",
            "contact-1",
            "left-front",
            "model-a",
            "sensor-a",
            SensorChannel::D1Interface,
            1.0,
            10,
            11,
        );
        let ob = observation(
            "eb",
            "contact-1",
            "left-front",
            "model-b",
            "sensor-b",
            SensorChannel::D3Proprioceptive,
            1.0,
            12,
            13,
        );
        oa.clock_id = "clock-a".to_owned();
        let mut ob = ob;
        ob.clock_id = "clock-b".to_owned();
        assert_eq!(
            update_with_observations(&prior(), &[a, b], &[oa, ob], 20, 20),
            Err(ContactInformationError::ClockMismatch)
        );
    }

    #[test]
    fn correlated_measurement_noise_is_rejected() {
        let mut m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D2NearFootWrench,
            [1.0, 0.0, 0.0],
            1.0,
        );
        m.independent_noise_verified = false;
        assert_eq!(
            expected_information_gain(&prior(), &[m]),
            Err(ContactInformationError::NoiseIndependenceNotVerified)
        );
    }

    #[test]
    fn unsafe_high_information_action_is_rejected_before_pareto_ranking() {
        let unsafe_action = ActionCandidate {
            action_id: "unsafe-high-eig".to_owned(),
            envelope: envelope(1.0, 0.01, 10.0, 0.5, 0.5),
            measurement_models: vec![model(
                "model-high",
                "sensor-high",
                SensorChannel::D1Interface,
                [4.0, 0.0, 0.0],
                0.1,
            )],
        };
        let safe_action = ActionCandidate {
            action_id: "safe-lower-eig".to_owned(),
            envelope: envelope(1.0, 0.4, 8.0, 0.2, 0.4),
            measurement_models: vec![model(
                "model-low",
                "sensor-low",
                SensorChannel::D3Proprioceptive,
                [1.0, 0.0, 0.0],
                1.0,
            )],
        };
        let assessments = assess_actions(&prior(), &[unsafe_action, safe_action], limits()).unwrap();
        let unsafe_result = assessments
            .iter()
            .find(|assessment| assessment.action_id == "unsafe-high-eig")
            .unwrap();
        assert!(!unsafe_result.feasible);
        assert_eq!(
            unsafe_result.rejection,
            Some(ConstraintViolation::InsufficientSupportMargin)
        );
        let safe_result = assessments
            .iter()
            .find(|assessment| assessment.action_id == "safe-lower-eig")
            .unwrap();
        assert!(safe_result.feasible);
        assert_eq!(
            pareto_frontier_action_ids(&assessments),
            vec!["safe-lower-eig".to_owned()]
        );
    }

    #[test]
    fn duplicate_measurement_model_is_rejected() {
        let m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        let a = observation(
            "evidence-a",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            1.0,
            10,
            11,
        );
        let mut b = observation(
            "evidence-b",
            "contact-1",
            "left-front",
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            2.0,
            12,
            13,
        );
        b.clock_id = a.clock_id.clone();
        assert_eq!(
            update_with_observations(&prior(), &[m], &[a, b], 20, 20),
            Err(ContactInformationError::DuplicateMeasurementModel)
        );
    }

    #[test]
    fn invalid_non_spd_prior_is_rejected() {
        let mut p = prior();
        p.covariance[0][0] = -1.0;
        let m = model(
            "model-x",
            "sensor-x",
            SensorChannel::D1Interface,
            [1.0, 0.0, 0.0],
            1.0,
        );
        assert_eq!(
            expected_information_gain(&p, &[m]),
            Err(ContactInformationError::InvalidCovariance)
        );
    }
}
