//! Unit-aware reference metrics for candidate cross-species kinematic invariants.
//!
//! This crate deliberately stops at measurement qualification. It does not
//! infer a universal motor-control law, homologous anatomy, neural mechanism,
//! or transferable controller.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// Joint angle measured in radians.
    JointAngleRadians,
    /// Segment or landmark displacement measured in metres.
    SegmentDisplacementMetres,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelSpec {
    pub id: String,
    pub quantity: Quantity,
    /// 0.0 is distal and 1.0 is proximal in this trial's declared body-plan
    /// ordering. This is a normalized measurement coordinate, not a claim of
    /// anatomical homology across species.
    pub proximal_coordinate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeSample {
    pub timestamp_seconds: f64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChannelObservation {
    pub channel_id: String,
    /// None means the channel was explicitly unavailable. It is never
    /// replaced with a zero-valued trajectory.
    pub samples: Option<Vec<TimeSample>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KinematicTrial {
    pub species_id: String,
    pub individual_id: String,
    pub session_id: String,
    pub body_length_metres: f64,
    pub cycle_duration_seconds: f64,
    pub channel_specs: Vec<ChannelSpec>,
    pub observations: Vec<ChannelObservation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedVelocity {
    pub channel_id: String,
    pub quantity: Quantity,
    pub proximal_coordinate: f64,
    /// Dimensionless cycle-normalized RMS velocity. Missing observations
    /// remain None.
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DistalProximalGradient {
    pub quantity: Quantity,
    /// Change in dimensionless RMS velocity per unit normalized proximal
    /// coordinate. Positive values mean higher values toward the proximal end.
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
    pub channels_used: usize,
    pub missing_channels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KinematicError {
    EmptyIdentifier,
    NoChannels,
    DuplicateChannelId,
    UnknownObservationChannel,
    DuplicateObservationChannel,
    MissingObservationRecord,
    InvalidBodyLength,
    InvalidCycleDuration,
    InvalidProximalCoordinate,
    TooFewSamples,
    NonFiniteSample,
    NonMonotonicTimestamp,
    NumericalFailure,
    InsufficientObservedChannels,
    DegenerateProximalCoordinates,
}

fn validate_trial(trial: &KinematicTrial) -> Result<(), KinematicError> {
    if trial.species_id.trim().is_empty()
        || trial.individual_id.trim().is_empty()
        || trial.session_id.trim().is_empty()
    {
        return Err(KinematicError::EmptyIdentifier);
    }
    if trial.channel_specs.is_empty() {
        return Err(KinematicError::NoChannels);
    }
    if !trial.body_length_metres.is_finite() || trial.body_length_metres <= 0.0 {
        return Err(KinematicError::InvalidBodyLength);
    }
    if !trial.cycle_duration_seconds.is_finite() || trial.cycle_duration_seconds <= 0.0 {
        return Err(KinematicError::InvalidCycleDuration);
    }

    for (index, spec) in trial.channel_specs.iter().enumerate() {
        if spec.id.trim().is_empty() {
            return Err(KinematicError::EmptyIdentifier);
        }
        if !spec.proximal_coordinate.is_finite()
            || !(0.0..=1.0).contains(&spec.proximal_coordinate)
        {
            return Err(KinematicError::InvalidProximalCoordinate);
        }
        if trial.channel_specs[..index]
            .iter()
            .any(|prior| prior.id == spec.id)
        {
            return Err(KinematicError::DuplicateChannelId);
        }
    }

    for (index, observation) in trial.observations.iter().enumerate() {
        if !trial
            .channel_specs
            .iter()
            .any(|spec| spec.id == observation.channel_id)
        {
            return Err(KinematicError::UnknownObservationChannel);
        }
        if trial.observations[..index]
            .iter()
            .any(|prior| prior.channel_id == observation.channel_id)
        {
            return Err(KinematicError::DuplicateObservationChannel);
        }
        if let Some(samples) = &observation.samples {
            if samples.len() < 2 {
                return Err(KinematicError::TooFewSamples);
            }
            let mut previous_time = None;
            for sample in samples {
                if !sample.timestamp_seconds.is_finite() || !sample.value.is_finite() {
                    return Err(KinematicError::NonFiniteSample);
                }
                if previous_time.is_some_and(|time| sample.timestamp_seconds <= time) {
                    return Err(KinematicError::NonMonotonicTimestamp);
                }
                previous_time = Some(sample.timestamp_seconds);
            }
        }
    }

    if trial.observations.len() != trial.channel_specs.len()
        || trial.channel_specs.iter().any(|spec| {
            !trial
                .observations
                .iter()
                .any(|observation| observation.channel_id == spec.id)
        })
    {
        return Err(KinematicError::MissingObservationRecord);
    }

    Ok(())
}

/// Compute time-weighted RMS velocity from irregularly sampled raw trajectories.
///
/// The normalized value is:
/// - angular: RMS(rad/s) * cycle_duration(s);
/// - linear: RMS(m/s) * cycle_duration(s) / body_length(m).
///
/// The original timestamps are used directly. Cycle resampling is not applied.
fn normalized_rms_velocity(
    samples: &[TimeSample],
    quantity: Quantity,
    cycle_duration_seconds: f64,
    body_length_metres: f64,
) -> Result<f64, KinematicError> {
    if samples.len() < 2 {
        return Err(KinematicError::TooFewSamples);
    }

    let first = samples.first().ok_or(KinematicError::TooFewSamples)?;
    let last = samples.last().ok_or(KinematicError::TooFewSamples)?;
    let total_duration = last.timestamp_seconds - first.timestamp_seconds;
    if !total_duration.is_finite() || total_duration <= 0.0 {
        return Err(KinematicError::NonMonotonicTimestamp);
    }

    let mut integrated_squared_velocity = 0.0;
    for pair in samples.windows(2) {
        let elapsed = pair[1].timestamp_seconds - pair[0].timestamp_seconds;
        let displacement = pair[1].value - pair[0].value;
        if !elapsed.is_finite() || elapsed <= 0.0 || !displacement.is_finite() {
            return Err(KinematicError::NonMonotonicTimestamp);
        }
        let velocity = displacement / elapsed;
        integrated_squared_velocity += velocity * velocity * elapsed;
    }

    let rms_velocity = (integrated_squared_velocity / total_duration).sqrt();
    let scale = match quantity {
        Quantity::JointAngleRadians => cycle_duration_seconds,
        Quantity::SegmentDisplacementMetres => cycle_duration_seconds / body_length_metres,
    };
    let normalized = rms_velocity * scale;
    if !normalized.is_finite() {
        return Err(KinematicError::NumericalFailure);
    }
    Ok(normalized)
}

/// Produce per-channel dimensionless velocity observations.
///
/// A missing sensor/channel is retained as None. The function never imputes
/// zero and never pools angular and linear quantities.
pub fn normalized_velocity_profile(
    trial: &KinematicTrial,
) -> Result<Vec<NormalizedVelocity>, KinematicError> {
    validate_trial(trial)?;

    let mut output = Vec::with_capacity(trial.channel_specs.len());
    for spec in &trial.channel_specs {
        let observation = trial
            .observations
            .iter()
            .find(|item| item.channel_id == spec.id)
            .ok_or(KinematicError::MissingObservationRecord)?;

        let value = match observation.samples.as_deref() {
            Some(samples) => Some(normalized_rms_velocity(
                samples,
                spec.quantity,
                trial.cycle_duration_seconds,
                trial.body_length_metres,
            )?),
            None => None,
        };

        output.push(NormalizedVelocity {
            channel_id: spec.id.clone(),
            quantity: spec.quantity,
            proximal_coordinate: spec.proximal_coordinate,
            value,
        });
    }
    Ok(output)
}

/// Estimate a least-squares distal-to-proximal gradient for one quantity.
///
/// At least three observed channels are required. Channels of other quantities
/// are excluded rather than pooled. Missing channels remain counted in the
/// output and do not contribute a fabricated zero.
pub fn estimate_distal_proximal_gradient(
    profile: &[NormalizedVelocity],
    quantity: Quantity,
) -> Result<DistalProximalGradient, KinematicError> {
    let selected: Vec<&NormalizedVelocity> = profile
        .iter()
        .filter(|channel| channel.quantity == quantity)
        .collect();

    let observed: Vec<&NormalizedVelocity> = selected
        .iter()
        .copied()
        .filter(|channel| channel.value.is_some())
        .collect();

    let missing_channels = selected.len() - observed.len();
    if observed.len() < 3 {
        return Err(KinematicError::InsufficientObservedChannels);
    }

    let count = observed.len() as f64;
    let mean_x = observed
        .iter()
        .map(|channel| channel.proximal_coordinate)
        .sum::<f64>()
        / count;
    let mean_y = observed
        .iter()
        .map(|channel| channel.value.unwrap_or(0.0))
        .sum::<f64>()
        / count;

    let mut sum_xx = 0.0;
    let mut sum_xy = 0.0;
    for channel in &observed {
        let dx = channel.proximal_coordinate - mean_x;
        let dy = channel.value.ok_or(KinematicError::NumericalFailure)? - mean_y;
        sum_xx += dx * dx;
        sum_xy += dx * dy;
    }
    if !sum_xx.is_finite() || sum_xx <= f64::EPSILON {
        return Err(KinematicError::DegenerateProximalCoordinates);
    }

    let slope = sum_xy / sum_xx;
    let intercept = mean_y - slope * mean_x;
    let mut residual_sum_squares = 0.0;
    let mut total_sum_squares = 0.0;
    for channel in &observed {
        let x = channel.proximal_coordinate;
        let y = channel.value.ok_or(KinematicError::NumericalFailure)?;
        let residual = y - (intercept + slope * x);
        residual_sum_squares += residual * residual;
        total_sum_squares += (y - mean_y) * (y - mean_y);
    }
    let r_squared = if total_sum_squares <= f64::EPSILON {
        if residual_sum_squares <= f64::EPSILON {
            1.0
        } else {
            0.0
        }
    } else {
        1.0 - residual_sum_squares / total_sum_squares
    };

    if !slope.is_finite() || !intercept.is_finite() || !r_squared.is_finite() {
        return Err(KinematicError::NumericalFailure);
    }

    Ok(DistalProximalGradient {
        quantity,
        slope,
        intercept,
        r_squared,
        channels_used: observed.len(),
        missing_channels,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn angular_trial(
        cycle_duration_seconds: f64,
        observations: Vec<ChannelObservation>,
    ) -> KinematicTrial {
        KinematicTrial {
            species_id: "species-a".to_owned(),
            individual_id: "animal-1".to_owned(),
            session_id: "session-1".to_owned(),
            body_length_metres: 1.0,
            cycle_duration_seconds,
            channel_specs: vec![
                ChannelSpec {
                    id: "distal".to_owned(),
                    quantity: Quantity::JointAngleRadians,
                    proximal_coordinate: 0.0,
                },
                ChannelSpec {
                    id: "middle".to_owned(),
                    quantity: Quantity::JointAngleRadians,
                    proximal_coordinate: 0.5,
                },
                ChannelSpec {
                    id: "proximal".to_owned(),
                    quantity: Quantity::JointAngleRadians,
                    proximal_coordinate: 1.0,
                },
            ],
            observations,
        }
    }

    fn samples(end_time: f64, end_value: f64) -> Option<Vec<TimeSample>> {
        Some(vec![
            TimeSample {
                timestamp_seconds: 0.0,
                value: 0.0,
            },
            TimeSample {
                timestamp_seconds: end_time,
                value: end_value,
            },
        ])
    }

    #[test]
    fn angular_velocity_is_invariant_to_time_unit_and_cycle_scale_when_motion_is_same_per_cycle() {
        let normal = angular_trial(
            2.0,
            vec![
                ChannelObservation {
                    channel_id: "distal".to_owned(),
                    samples: samples(2.0, 2.0),
                },
                ChannelObservation {
                    channel_id: "middle".to_owned(),
                    samples: samples(2.0, 4.0),
                },
                ChannelObservation {
                    channel_id: "proximal".to_owned(),
                    samples: samples(2.0, 6.0),
                },
            ],
        );
        let slower_clock = angular_trial(
            4.0,
            vec![
                ChannelObservation {
                    channel_id: "distal".to_owned(),
                    samples: samples(4.0, 2.0),
                },
                ChannelObservation {
                    channel_id: "middle".to_owned(),
                    samples: samples(4.0, 4.0),
                },
                ChannelObservation {
                    channel_id: "proximal".to_owned(),
                    samples: samples(4.0, 6.0),
                },
            ],
        );
        let a = normalized_velocity_profile(&normal).unwrap();
        let b = normalized_velocity_profile(&slower_clock).unwrap();
        for (left, right) in a.iter().zip(b) {
            assert!((left.value.unwrap() - right.value.unwrap()).abs() < 1.0e-12);
        }
    }

    #[test]
    fn linear_velocity_is_invariant_to_geometric_and_temporal_scaling() {
        let base = KinematicTrial {
            species_id: "small".to_owned(),
            individual_id: "a".to_owned(),
            session_id: "s".to_owned(),
            body_length_metres: 1.0,
            cycle_duration_seconds: 2.0,
            channel_specs: vec![
                ChannelSpec { id: "d".to_owned(), quantity: Quantity::SegmentDisplacementMetres, proximal_coordinate: 0.0 },
                ChannelSpec { id: "m".to_owned(), quantity: Quantity::SegmentDisplacementMetres, proximal_coordinate: 0.5 },
                ChannelSpec { id: "p".to_owned(), quantity: Quantity::SegmentDisplacementMetres, proximal_coordinate: 1.0 },
            ],
            observations: vec![
                ChannelObservation { channel_id: "d".to_owned(), samples: samples(2.0, 2.0) },
                ChannelObservation { channel_id: "m".to_owned(), samples: samples(2.0, 4.0) },
                ChannelObservation { channel_id: "p".to_owned(), samples: samples(2.0, 6.0) },
            ],
        };
        let mut scaled = base.clone();
        scaled.species_id = "large".to_owned();
        scaled.body_length_metres = 2.0;
        scaled.cycle_duration_seconds = 4.0;
        for observation in &mut scaled.observations {
            if let Some(values) = &mut observation.samples {
                for sample in values {
                    sample.timestamp_seconds *= 2.0;
                    sample.value *= 2.0;
                }
            }
        }
        let first = normalized_velocity_profile(&base).unwrap();
        let second = normalized_velocity_profile(&scaled).unwrap();
        for (left, right) in first.iter().zip(second) {
            assert!((left.value.unwrap() - right.value.unwrap()).abs() < 1.0e-12);
        }
    }

    #[test]
    fn irregular_timestamps_use_time_weighted_velocity_not_sample_count_weighting() {
        let trial = angular_trial(
            1.0,
            vec![
                ChannelObservation {
                    channel_id: "distal".to_owned(),
                    samples: Some(vec![
                        TimeSample { timestamp_seconds: 0.0, value: 0.0 },
                        TimeSample { timestamp_seconds: 0.25, value: 0.5 },
                        TimeSample { timestamp_seconds: 2.0, value: 4.0 },
                    ]),
                },
                ChannelObservation { channel_id: "middle".to_owned(), samples: samples(2.0, 4.0) },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(2.0, 6.0) },
            ],
        );
        let profile = normalized_velocity_profile(&trial).unwrap();
        assert!((profile[0].value.unwrap() - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn distal_to_proximal_gradient_recovers_known_answer_and_keeps_missingness() {
        let mut trial = angular_trial(
            1.0,
            vec![
                ChannelObservation { channel_id: "distal".to_owned(), samples: samples(1.0, 1.0) },
                ChannelObservation { channel_id: "middle".to_owned(), samples: samples(1.0, 2.0) },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(1.0, 3.0) },
            ],
        );
        let profile = normalized_velocity_profile(&trial).unwrap();
        let estimate = estimate_distal_proximal_gradient(&profile, Quantity::JointAngleRadians).unwrap();
        assert!((estimate.slope - 2.0).abs() < 1.0e-12);
        assert!((estimate.intercept - 1.0).abs() < 1.0e-12);
        assert!((estimate.r_squared - 1.0).abs() < 1.0e-12);
        assert_eq!(estimate.channels_used, 3);
        assert_eq!(estimate.missing_channels, 0);

        trial.observations[1].samples = None;
        let missing_profile = normalized_velocity_profile(&trial).unwrap();
        assert_eq!(missing_profile[1].value, None);
        assert_eq!(
            estimate_distal_proximal_gradient(&missing_profile, Quantity::JointAngleRadians),
            Err(KinematicError::InsufficientObservedChannels)
        );
    }

    #[test]
    fn quantity_families_are_scored_separately() {
        let profile = vec![
            NormalizedVelocity { channel_id: "a".to_owned(), quantity: Quantity::JointAngleRadians, proximal_coordinate: 0.0, value: Some(1.0) },
            NormalizedVelocity { channel_id: "b".to_owned(), quantity: Quantity::SegmentDisplacementMetres, proximal_coordinate: 0.5, value: Some(50.0) },
            NormalizedVelocity { channel_id: "c".to_owned(), quantity: Quantity::JointAngleRadians, proximal_coordinate: 0.5, value: Some(2.0) },
            NormalizedVelocity { channel_id: "d".to_owned(), quantity: Quantity::JointAngleRadians, proximal_coordinate: 1.0, value: Some(3.0) },
        ];
        let estimate = estimate_distal_proximal_gradient(&profile, Quantity::JointAngleRadians).unwrap();
        assert_eq!(estimate.channels_used, 3);
        assert!((estimate.slope - 2.0).abs() < 1.0e-12);
    }

    #[test]
    fn missing_channel_is_not_zero_imputed() {
        let trial = angular_trial(
            1.0,
            vec![
                ChannelObservation { channel_id: "distal".to_owned(), samples: samples(1.0, 1.0) },
                ChannelObservation { channel_id: "middle".to_owned(), samples: None },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(1.0, 3.0) },
            ],
        );
        let profile = normalized_velocity_profile(&trial).unwrap();
        assert_eq!(profile[1].value, None);
        assert_eq!(profile.len(), 3);
    }

    #[test]
    fn timestamps_must_be_strictly_increasing() {
        let trial = angular_trial(
            1.0,
            vec![
                ChannelObservation {
                    channel_id: "distal".to_owned(),
                    samples: Some(vec![
                        TimeSample { timestamp_seconds: 0.0, value: 0.0 },
                        TimeSample { timestamp_seconds: 0.0, value: 1.0 },
                    ]),
                },
                ChannelObservation { channel_id: "middle".to_owned(), samples: samples(1.0, 1.0) },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(1.0, 1.0) },
            ],
        );
        assert_eq!(normalized_velocity_profile(&trial), Err(KinematicError::NonMonotonicTimestamp));
    }

    #[test]
    fn duplicate_channel_identity_is_rejected() {
        let mut trial = angular_trial(
            1.0,
            vec![
                ChannelObservation { channel_id: "distal".to_owned(), samples: samples(1.0, 1.0) },
                ChannelObservation { channel_id: "middle".to_owned(), samples: samples(1.0, 2.0) },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(1.0, 3.0) },
            ],
        );
        trial.channel_specs[1].id = "distal".to_owned();
        assert_eq!(normalized_velocity_profile(&trial), Err(KinematicError::DuplicateChannelId));
    }

    #[test]
    fn missing_observation_record_is_rejected_instead_of_silently_omitted() {
        let mut trial = angular_trial(
            1.0,
            vec![
                ChannelObservation { channel_id: "distal".to_owned(), samples: samples(1.0, 1.0) },
                ChannelObservation { channel_id: "middle".to_owned(), samples: samples(1.0, 2.0) },
                ChannelObservation { channel_id: "proximal".to_owned(), samples: samples(1.0, 3.0) },
            ],
        );
        trial.observations.pop();
        assert_eq!(normalized_velocity_profile(&trial), Err(KinematicError::MissingObservationRecord));
    }
}
