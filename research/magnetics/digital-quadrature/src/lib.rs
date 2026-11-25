//! Hall-latch hysteresis and digital quadrature reference for multipole encoders.
//!
//! This is a sampled-input state-machine reference. It does not interpolate
//! field crossings, model an ASIC's analog transfer function, or authenticate
//! any vendor-specific threshold / bandwidth specification.

use std::f64::consts::{PI, TAU};

const ANGLE_TOLERANCE_RAD: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    SyntheticScenario,
    SolverPrediction,
    CalibratedFieldMeasurement,
    CapturedDigitalOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SensorArchitecture {
    DiscreteSpatialLatches,
    IntegratedTwoAxisLatch,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InPlaneField {
    pub radial_t: f64,
    pub tangential_t: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncoderSample {
    /// Mechanical reference angle, including both ends of one full revolution.
    pub mechanical_angle_rad: f64,
    pub timestamp_ns: Option<u64>,
    pub channel_a_field: InPlaneField,
    pub channel_b_field: InPlaneField,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderTrace {
    pub trace_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub angular_grid_id: String,
    pub evidence_kind: EvidenceKind,
    pub samples: Vec<EncoderSample>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HallLatchConfig {
    pub axis_angle_rad: f64,
    pub bop_threshold_t: f64,
    pub brp_threshold_t: f64,
    pub initial_high: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeRatePolicy {
    pub rotational_speed_rpm: f64,
    /// Caller-declared transition-rate ceiling, not automatically a datasheet bandwidth.
    pub maximum_transition_rate_hz: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuadratureConfig {
    pub pole_pairs: u32,
    pub architecture: SensorArchitecture,
    pub channel_a: HallLatchConfig,
    pub channel_b: HallLatchConfig,
    pub edge_rate_policy: Option<EdgeRatePolicy>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuadratureState {
    pub channel_a_high: bool,
    pub channel_b_high: bool,
}

impl QuadratureState {
    fn code(self) -> u8 {
        (u8::from(self.channel_a_high) << 1) | u8::from(self.channel_b_high)
    }

    // Forward Gray cycle for increasing angle when A = cos(p*theta), B = sin(p*theta):
    // 10 -> 11 -> 01 -> 00 -> 10.
    fn gray_index(self) -> u8 {
        match self.code() {
            0b00 => 0,
            0b10 => 1,
            0b11 => 2,
            0b01 => 3,
            _ => unreachable!("two-bit state has four encodings"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionDirection {
    Forward,
    Reverse,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuadratureTransition {
    pub sample_after_index: usize,
    pub from: QuadratureState,
    pub to: QuadratureState,
    pub direction: Option<TransitionDirection>,
    /// A transition is localized only to this sample interval.
    pub lower_angle_rad: f64,
    pub upper_angle_rad: f64,
    pub timestamp_bracket_ns: Option<(u64, u64)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChannelEdgeCounts {
    pub channel_a_rising: usize,
    pub channel_a_falling: usize,
    pub channel_b_rising: usize,
    pub channel_b_falling: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuadratureMetrics {
    pub expected_transitions_per_revolution: usize,
    pub observed_valid_transitions: usize,
    pub illegal_two_bit_transitions: usize,
    pub missing_transition_count: usize,
    pub excess_transition_count: usize,
    pub direction_reversal_count: usize,
    pub periodic_state_match: bool,
    pub channel_edge_counts: ChannelEdgeCounts,
    /// Absolute deviation from 90 degrees between integrated sensor axes.
    /// None for two physically separated discrete latches.
    pub integrated_axis_nonorthogonality_rad: Option<f64>,
    pub expected_quadrature_spacing_rad: f64,
    pub quadrature_spacing_interval_count: usize,
    /// Lower and upper bounds on the maximum absolute edge-spacing error.
    pub maximum_spacing_error_lower_bound_rad: Option<f64>,
    pub maximum_spacing_error_upper_bound_rad: Option<f64>,
    pub minimum_transition_bracket_rad: Option<f64>,
    pub maximum_transition_bracket_rad: Option<f64>,
    pub expected_transition_rate_hz: Option<f64>,
    pub maximum_transition_rate_hz: Option<f64>,
    pub within_edge_rate_policy: Option<bool>,
    pub quadrature_sequence_valid: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SensorEvaluation {
    pub trace_id: String,
    pub architecture: SensorArchitecture,
    pub sample_count: usize,
    pub states: Vec<QuadratureState>,
    pub transitions: Vec<QuadratureTransition>,
    pub metrics: QuadratureMetrics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvaluationError {
    InvalidMetadata,
    InvalidConfiguration,
    InvalidThresholdHysteresis,
    InvalidTimingPolicy,
    TooFewSamples,
    NonFiniteSample { index: usize },
    NonMonotonicAngleGrid { index: usize },
    NonUniformOrIncompleteRevolution { index: usize },
    MixedTimestampAvailability,
    NonMonotonicTimestamp { index: usize },
    IntegratedSensorFieldMismatch { index: usize },
    InitialStateInconsistent { channel: char },
    ProjectedFieldOverflow { index: usize, channel: char },
    TransitionCountOverflow,
    InvalidHarmonicRequest,
    HarmonicOrderAboveNyquist,
    SpectralNumericOverflow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_latch(config: HallLatchConfig) -> Result<(), EvaluationError> {
    if !config.axis_angle_rad.is_finite()
        || !config.bop_threshold_t.is_finite()
        || !config.brp_threshold_t.is_finite()
    {
        return Err(EvaluationError::InvalidConfiguration);
    }
    if config.bop_threshold_t <= config.brp_threshold_t {
        return Err(EvaluationError::InvalidThresholdHysteresis);
    }
    Ok(())
}

fn validate_config(config: QuadratureConfig) -> Result<(), EvaluationError> {
    if config.pole_pairs == 0 {
        return Err(EvaluationError::InvalidConfiguration);
    }
    validate_latch(config.channel_a)?;
    validate_latch(config.channel_b)?;
    if let Some(policy) = config.edge_rate_policy {
        if !policy.rotational_speed_rpm.is_finite()
            || policy.rotational_speed_rpm < 0.0
            || !policy.maximum_transition_rate_hz.is_finite()
            || policy.maximum_transition_rate_hz <= 0.0
        {
            return Err(EvaluationError::InvalidTimingPolicy);
        }
    }
    Ok(())
}

fn project(field: InPlaneField, axis_angle_rad: f64) -> f64 {
    field.radial_t * axis_angle_rad.cos() + field.tangential_t * axis_angle_rad.sin()
}

fn latch_next_state(current_high: bool, projected_field_t: f64, config: HallLatchConfig) -> bool {
    if current_high {
        projected_field_t > config.brp_threshold_t
    } else {
        projected_field_t >= config.bop_threshold_t
    }
}

fn latch_is_consistent_at_start(
    initial_high: bool,
    projected_field_t: f64,
    config: HallLatchConfig,
) -> bool {
    if initial_high {
        projected_field_t > config.brp_threshold_t
    } else {
        projected_field_t < config.bop_threshold_t
    }
}

fn classify_transition(
    from: QuadratureState,
    to: QuadratureState,
) -> (Option<TransitionDirection>, bool) {
    if (from.code() ^ to.code()).count_ones() != 1 {
        return (None, true);
    }
    let from_index = from.gray_index();
    let to_index = to.gray_index();
    if to_index == (from_index + 1) % 4 {
        (Some(TransitionDirection::Forward), false)
    } else if from_index == (to_index + 1) % 4 {
        (Some(TransitionDirection::Reverse), false)
    } else {
        (None, true)
    }
}


fn spacing_error_bounds(
    interval_lower_rad: f64,
    interval_upper_rad: f64,
    expected_spacing_rad: f64,
) -> (f64, f64) {
    let lower_bound = if expected_spacing_rad < interval_lower_rad {
        interval_lower_rad - expected_spacing_rad
    } else if expected_spacing_rad > interval_upper_rad {
        expected_spacing_rad - interval_upper_rad
    } else {
        0.0
    };
    let upper_bound = (interval_lower_rad - expected_spacing_rad)
        .abs()
        .max((interval_upper_rad - expected_spacing_rad).abs());
    (lower_bound, upper_bound)
}

/// Evaluate two independent hysteretic Hall latch channels over one complete
/// uniformly sampled mechanical revolution. DiscreteSpatialLatches may use two
/// different field positions. IntegratedTwoAxisLatch requires the same local
/// field vector for both orthogonal sensing axes.
///
/// The initial states must be compatible with the first sample. Field crossings
/// are reported as sample intervals; no sub-sample edge time is fabricated.
pub fn evaluate_trace(
    trace: &EncoderTrace,
    config: QuadratureConfig,
) -> Result<SensorEvaluation, EvaluationError> {
    validate_config(config)?;
    if !nonempty(&trace.trace_id)
        || !nonempty(&trace.coordinate_frame_id)
        || !nonempty(&trace.operating_condition_id)
        || !nonempty(&trace.angular_grid_id)
    {
        return Err(EvaluationError::InvalidMetadata);
    }
    if trace.samples.len() < 5 {
        return Err(EvaluationError::TooFewSamples);
    }

    let sample_count = trace.samples.len();
    let start_angle = trace.samples[0].mechanical_angle_rad;
    let step = TAU / (sample_count - 1) as f64;
    let timestamp_presence = trace.samples[0].timestamp_ns.is_some();
    let first = trace.samples[0];

    if !first.mechanical_angle_rad.is_finite()
        || !first.channel_a_field.radial_t.is_finite()
        || !first.channel_a_field.tangential_t.is_finite()
        || !first.channel_b_field.radial_t.is_finite()
        || !first.channel_b_field.tangential_t.is_finite()
    {
        return Err(EvaluationError::NonFiniteSample { index: 0 });
    }

    if config.architecture == SensorArchitecture::IntegratedTwoAxisLatch
        && (first.channel_a_field.radial_t.to_bits() != first.channel_b_field.radial_t.to_bits()
            || first.channel_a_field.tangential_t.to_bits()
                != first.channel_b_field.tangential_t.to_bits())
    {
        return Err(EvaluationError::IntegratedSensorFieldMismatch { index: 0 });
    }

    let first_a = project(first.channel_a_field, config.channel_a.axis_angle_rad);
    let first_b = project(first.channel_b_field, config.channel_b.axis_angle_rad);
    if !first_a.is_finite() {
        return Err(EvaluationError::ProjectedFieldOverflow {
            index: 0,
            channel: 'A',
        });
    }
    if !first_b.is_finite() {
        return Err(EvaluationError::ProjectedFieldOverflow {
            index: 0,
            channel: 'B',
        });
    }
    if !latch_is_consistent_at_start(config.channel_a.initial_high, first_a, config.channel_a) {
        return Err(EvaluationError::InitialStateInconsistent { channel: 'A' });
    }
    if !latch_is_consistent_at_start(config.channel_b.initial_high, first_b, config.channel_b) {
        return Err(EvaluationError::InitialStateInconsistent { channel: 'B' });
    }

    let mut state = QuadratureState {
        channel_a_high: config.channel_a.initial_high,
        channel_b_high: config.channel_b.initial_high,
    };
    let mut states = Vec::with_capacity(sample_count);
    states.push(state);

    for (index, sample) in trace.samples.iter().enumerate().skip(1) {
        if !sample.mechanical_angle_rad.is_finite()
            || !sample.channel_a_field.radial_t.is_finite()
            || !sample.channel_a_field.tangential_t.is_finite()
            || !sample.channel_b_field.radial_t.is_finite()
            || !sample.channel_b_field.tangential_t.is_finite()
        {
            return Err(EvaluationError::NonFiniteSample { index });
        }

        let previous = trace.samples[index - 1];
        if sample.mechanical_angle_rad <= previous.mechanical_angle_rad {
            return Err(EvaluationError::NonMonotonicAngleGrid { index });
        }
        let expected_angle = start_angle + index as f64 * step;
        if (sample.mechanical_angle_rad - expected_angle).abs() > ANGLE_TOLERANCE_RAD {
            return Err(EvaluationError::NonUniformOrIncompleteRevolution { index });
        }

        if sample.timestamp_ns.is_some() != timestamp_presence {
            return Err(EvaluationError::MixedTimestampAvailability);
        }
        if let (Some(previous_time), Some(current_time)) =
            (previous.timestamp_ns, sample.timestamp_ns)
        {
            if current_time <= previous_time {
                return Err(EvaluationError::NonMonotonicTimestamp { index });
            }
        }

        if config.architecture == SensorArchitecture::IntegratedTwoAxisLatch
            && (sample.channel_a_field.radial_t.to_bits()
                != sample.channel_b_field.radial_t.to_bits()
                || sample.channel_a_field.tangential_t.to_bits()
                    != sample.channel_b_field.tangential_t.to_bits())
        {
            return Err(EvaluationError::IntegratedSensorFieldMismatch { index });
        }

        let projected_a = project(sample.channel_a_field, config.channel_a.axis_angle_rad);
        let projected_b = project(sample.channel_b_field, config.channel_b.axis_angle_rad);
        if !projected_a.is_finite() {
            return Err(EvaluationError::ProjectedFieldOverflow {
                index,
                channel: 'A',
            });
        }
        if !projected_b.is_finite() {
            return Err(EvaluationError::ProjectedFieldOverflow {
                index,
                channel: 'B',
            });
        }

        state.channel_a_high =
            latch_next_state(state.channel_a_high, projected_a, config.channel_a);
        state.channel_b_high =
            latch_next_state(state.channel_b_high, projected_b, config.channel_b);
        states.push(state);
    }

    let final_angle = trace.samples[sample_count - 1].mechanical_angle_rad;
    if (final_angle - (start_angle + TAU)).abs() > ANGLE_TOLERANCE_RAD {
        return Err(EvaluationError::NonUniformOrIncompleteRevolution {
            index: sample_count - 1,
        });
    }

    let first_state = states[0];
    let last_state = *states.last().expect("validated trace has samples");
    let periodic_state_match = first_state == last_state;

    let mut transitions = Vec::new();
    let mut illegal_two_bit_transitions = 0_usize;
    let mut direction_reversal_count = 0_usize;
    let mut previous_direction: Option<TransitionDirection> = None;
    let mut channel_edge_counts = ChannelEdgeCounts::default();
    let mut minimum_transition_bracket_rad = f64::INFINITY;
    let mut maximum_transition_bracket_rad = 0.0_f64;

    for index in 1..sample_count {
        let from = states[index - 1];
        let to = states[index];
        if from == to {
            continue;
        }

        let previous_sample = trace.samples[index - 1];
        let current_sample = trace.samples[index];
        let bracket_width = current_sample.mechanical_angle_rad
            - previous_sample.mechanical_angle_rad;
        minimum_transition_bracket_rad = minimum_transition_bracket_rad.min(bracket_width);
        maximum_transition_bracket_rad = maximum_transition_bracket_rad.max(bracket_width);

        if !from.channel_a_high && to.channel_a_high {
            channel_edge_counts.channel_a_rising += 1;
        } else if from.channel_a_high && !to.channel_a_high {
            channel_edge_counts.channel_a_falling += 1;
        }
        if !from.channel_b_high && to.channel_b_high {
            channel_edge_counts.channel_b_rising += 1;
        } else if from.channel_b_high && !to.channel_b_high {
            channel_edge_counts.channel_b_falling += 1;
        }

        let (direction, illegal) = classify_transition(from, to);
        if illegal {
            illegal_two_bit_transitions += 1;
        } else if let Some(direction) = direction {
            if previous_direction.is_some_and(|previous| previous != direction) {
                direction_reversal_count += 1;
            }
            previous_direction = Some(direction);
        }

        transitions.push(QuadratureTransition {
            sample_after_index: index,
            from,
            to,
            direction,
            lower_angle_rad: previous_sample.mechanical_angle_rad,
            upper_angle_rad: current_sample.mechanical_angle_rad,
            timestamp_bracket_ns: previous_sample
                .timestamp_ns
                .zip(current_sample.timestamp_ns),
        });
    }

    let observed_valid_transitions = transitions
        .iter()
        .filter(|transition| transition.direction.is_some())
        .count();
    let expected_transitions_per_revolution = usize::try_from(config.pole_pairs)
        .ok()
        .and_then(|pairs| pairs.checked_mul(4))
        .ok_or(EvaluationError::TransitionCountOverflow)?;
    let missing_transition_count =
        expected_transitions_per_revolution.saturating_sub(observed_valid_transitions);
    let excess_transition_count =
        observed_valid_transitions.saturating_sub(expected_transitions_per_revolution);

    let (expected_transition_rate_hz, maximum_transition_rate_hz, within_edge_rate_policy) =
        if let Some(policy) = config.edge_rate_policy {
            let expected_rate = expected_transitions_per_revolution as f64
                * policy.rotational_speed_rpm
                / 60.0;
            if !expected_rate.is_finite() {
                return Err(EvaluationError::InvalidTimingPolicy);
            }
            (
                Some(expected_rate),
                Some(policy.maximum_transition_rate_hz),
                Some(expected_rate <= policy.maximum_transition_rate_hz),
            )
        } else {
            (None, None, None)
        };


    let expected_quadrature_spacing_rad =
        TAU / expected_transitions_per_revolution as f64;
    let valid_transition_indices = transitions
        .iter()
        .enumerate()
        .filter_map(|(index, transition)| transition.direction.map(|_| index))
        .collect::<Vec<_>>();
    let mut maximum_spacing_error_lower_bound_rad = 0.0_f64;
    let mut maximum_spacing_error_upper_bound_rad = 0.0_f64;
    let mut quadrature_spacing_interval_count = 0_usize;

    let mut compare_spacing_interval = |lower: f64, upper: f64| {
        let (error_lower, error_upper) = spacing_error_bounds(
            lower,
            upper,
            expected_quadrature_spacing_rad,
        );
        maximum_spacing_error_lower_bound_rad =
            maximum_spacing_error_lower_bound_rad.max(error_lower);
        maximum_spacing_error_upper_bound_rad =
            maximum_spacing_error_upper_bound_rad.max(error_upper);
        quadrature_spacing_interval_count += 1;
    };

    for adjacent in valid_transition_indices.windows(2) {
        let first_edge = &transitions[adjacent[0]];
        let second_edge = &transitions[adjacent[1]];
        compare_spacing_interval(
            second_edge.lower_angle_rad - first_edge.upper_angle_rad,
            second_edge.upper_angle_rad - first_edge.lower_angle_rad,
        );
    }
    if periodic_state_match && valid_transition_indices.len() >= 2 {
        let first_edge = &transitions[valid_transition_indices[0]];
        let last_edge = &transitions[*valid_transition_indices.last().unwrap()];
        compare_spacing_interval(
            first_edge.lower_angle_rad + TAU - last_edge.upper_angle_rad,
            first_edge.upper_angle_rad + TAU - last_edge.lower_angle_rad,
        );
    }

    let quadrature_sequence_valid = periodic_state_match
        && illegal_two_bit_transitions == 0
        && direction_reversal_count == 0
        && observed_valid_transitions == expected_transitions_per_revolution;

    Ok(SensorEvaluation {
        trace_id: trace.trace_id.clone(),
        architecture: config.architecture,
        sample_count,
        states,
        transitions,
        metrics: QuadratureMetrics {
            expected_transitions_per_revolution,
            observed_valid_transitions,
            illegal_two_bit_transitions,
            missing_transition_count,
            excess_transition_count,
            direction_reversal_count,
            periodic_state_match,
            channel_edge_counts,
            integrated_axis_nonorthogonality_rad: (config.architecture
                == SensorArchitecture::IntegratedTwoAxisLatch)
                .then(|| {
                    let axis_dot = config.channel_a.axis_angle_rad.cos()
                        * config.channel_b.axis_angle_rad.cos()
                        + config.channel_a.axis_angle_rad.sin()
                            * config.channel_b.axis_angle_rad.sin();
                    axis_dot.abs().clamp(0.0, 1.0).asin()
                }),
            expected_quadrature_spacing_rad,
            quadrature_spacing_interval_count,
            maximum_spacing_error_lower_bound_rad: (quadrature_spacing_interval_count > 0)
                .then_some(maximum_spacing_error_lower_bound_rad),
            maximum_spacing_error_upper_bound_rad: (quadrature_spacing_interval_count > 0)
                .then_some(maximum_spacing_error_upper_bound_rad),
            minimum_transition_bracket_rad: minimum_transition_bracket_rad
                .is_finite()
                .then_some(minimum_transition_bracket_rad),
            maximum_transition_bracket_rad: (maximum_transition_bracket_rad > 0.0)
                .then_some(maximum_transition_bracket_rad),
            expected_transition_rate_hz,
            maximum_transition_rate_hz,
            within_edge_rate_policy,
            quadrature_sequence_valid,
        },
    })
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HallChannel {
    A,
    B,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpatialHarmonic {
    /// Multiple of the mechanical pole-pair fundamental.
    pub harmonic_multiple: u32,
    pub amplitude_t: f64,
    pub phase_rad: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HarmonicSpectrum {
    pub trace_id: String,
    pub channel: HallChannel,
    pub pole_pairs: u32,
    pub maximum_harmonic_multiple: u32,
    pub dc_offset_t: f64,
    pub total_rms_t: f64,
    pub fundamental_amplitude_t: Option<f64>,
    pub fundamental_phase_rad: Option<f64>,
    /// RMS of the reported integer harmonics 2..K divided by fundamental RMS.
    pub total_harmonic_distortion_ratio: Option<f64>,
    /// RMS of spectral content not represented by DC or reported integer harmonics.
    pub unresolved_spectral_residual_ratio: Option<f64>,
    /// Combined higher-harmonic and unresolved residual RMS / fundamental RMS.
    pub combined_nonfundamental_ratio: Option<f64>,
    /// Absolute difference between projected first and final endpoint samples.
    pub endpoint_discontinuity_t: f64,
    pub harmonics: Vec<SpatialHarmonic>,
}

/// Decompose one projected Hall-channel field waveform into spatial harmonics.
///
/// The trace must include both endpoints of one full revolution. The repeated
/// final endpoint is excluded from the discrete Fourier sum. Harmonic orders are
/// integer multiples of pole_pairs and must remain strictly below Nyquist.
/// This is a diagnostic decomposition, not a quadrature accuracy or qualification score.
pub fn analyze_channel_harmonics(
    trace: &EncoderTrace,
    channel: HallChannel,
    pole_pairs: u32,
    axis_angle_rad: f64,
    maximum_harmonic_multiple: u32,
) -> Result<HarmonicSpectrum, EvaluationError> {
    if !nonempty(&trace.trace_id)
        || !nonempty(&trace.coordinate_frame_id)
        || !nonempty(&trace.operating_condition_id)
        || !nonempty(&trace.angular_grid_id)
        || pole_pairs == 0
        || maximum_harmonic_multiple == 0
        || !axis_angle_rad.is_finite()
    {
        return Err(EvaluationError::InvalidHarmonicRequest);
    }
    if trace.samples.len() < 5 {
        return Err(EvaluationError::TooFewSamples);
    }

    let unique_count = trace.samples.len() - 1;
    let maximum_order = u64::from(pole_pairs)
        .checked_mul(u64::from(maximum_harmonic_multiple))
        .ok_or(EvaluationError::HarmonicOrderAboveNyquist)?;
    if maximum_order.checked_mul(2).is_none_or(|twice| twice >= unique_count as u64) {
        return Err(EvaluationError::HarmonicOrderAboveNyquist);
    }

    let start_angle = trace.samples[0].mechanical_angle_rad;
    let step = TAU / unique_count as f64;
    let mut projected = Vec::with_capacity(unique_count);
    for (index, sample) in trace.samples.iter().enumerate() {
        let field = match channel {
            HallChannel::A => sample.channel_a_field,
            HallChannel::B => sample.channel_b_field,
        };
        if !sample.mechanical_angle_rad.is_finite()
            || !field.radial_t.is_finite()
            || !field.tangential_t.is_finite()
        {
            return Err(EvaluationError::NonFiniteSample { index });
        }
        let expected_angle = start_angle + index as f64 * step;
        if (sample.mechanical_angle_rad - expected_angle).abs() > ANGLE_TOLERANCE_RAD {
            return Err(EvaluationError::NonUniformOrIncompleteRevolution { index });
        }
        let value = project(field, axis_angle_rad);
        if !value.is_finite() {
            return Err(EvaluationError::ProjectedFieldOverflow {
                index,
                channel: match channel { HallChannel::A => 'A', HallChannel::B => 'B' },
            });
        }
        if index < unique_count {
            projected.push(value);
        }
    }

    let endpoint_discontinuity_t = {
        let first_field = match channel {
            HallChannel::A => trace.samples[0].channel_a_field,
            HallChannel::B => trace.samples[0].channel_b_field,
        };
        let final_field = match channel {
            HallChannel::A => trace.samples[trace.samples.len() - 1].channel_a_field,
            HallChannel::B => trace.samples[trace.samples.len() - 1].channel_b_field,
        };
        (project(first_field, axis_angle_rad) - project(final_field, axis_angle_rad)).abs()
    };

    let dc_offset_t = projected.iter().sum::<f64>() / unique_count as f64;
    let total_rms_t = (projected.iter().map(|value| value * value).sum::<f64>()
        / unique_count as f64)
        .sqrt();
    if !dc_offset_t.is_finite() || !total_rms_t.is_finite() || !endpoint_discontinuity_t.is_finite() {
        return Err(EvaluationError::SpectralNumericOverflow);
    }

    let mut harmonics = Vec::with_capacity(maximum_harmonic_multiple as usize);
    let mut harmonic_energy_t2 = 0.0_f64;
    for multiple in 1..=maximum_harmonic_multiple {
        let order = f64::from(pole_pairs) * f64::from(multiple);
        let mut real = 0.0_f64;
        let mut imaginary = 0.0_f64;
        for (index, value) in projected.iter().enumerate() {
            let angle = trace.samples[index].mechanical_angle_rad;
            let phase = order * angle;
            real += value * phase.cos();
            imaginary -= value * phase.sin();
        }
        real /= unique_count as f64;
        imaginary /= unique_count as f64;
        let amplitude_t = 2.0 * real.hypot(imaginary);
        let phase_rad = imaginary.atan2(real);
        if !amplitude_t.is_finite() || !phase_rad.is_finite() {
            return Err(EvaluationError::SpectralNumericOverflow);
        }
        harmonic_energy_t2 += amplitude_t * amplitude_t / 2.0;
        harmonics.push(SpatialHarmonic {
            harmonic_multiple: multiple,
            amplitude_t,
            phase_rad,
        });
    }

    let fundamental_amplitude_t = harmonics.first().map(|component| component.amplitude_t);
    let fundamental_phase_rad = harmonics.first().map(|component| component.phase_rad);
    let residual_energy_t2 = (total_rms_t * total_rms_t
        - dc_offset_t * dc_offset_t
        - harmonic_energy_t2)
        .max(0.0);
    let unresolved_residual_rms_t = residual_energy_t2.sqrt();

    let (total_harmonic_distortion_ratio, unresolved_spectral_residual_ratio, combined_nonfundamental_ratio) =
        if let Some(fundamental) = fundamental_amplitude_t.filter(|amplitude| *amplitude > 0.0) {
            let higher_harmonic_energy_t2 = harmonics
                .iter()
                .skip(1)
                .map(|component| component.amplitude_t * component.amplitude_t / 2.0)
                .sum::<f64>();
            let fundamental_rms_t = fundamental / 2.0_f64.sqrt();
            let thd = (higher_harmonic_energy_t2.sqrt()) / fundamental_rms_t;
            let residual_ratio = unresolved_residual_rms_t / fundamental_rms_t;
            let combined = (higher_harmonic_energy_t2 + residual_energy_t2).sqrt() / fundamental_rms_t;
            (Some(thd), Some(residual_ratio), Some(combined))
        } else {
            (None, None, None)
        };

    if [total_harmonic_distortion_ratio, unresolved_spectral_residual_ratio, combined_nonfundamental_ratio]
        .into_iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        return Err(EvaluationError::SpectralNumericOverflow);
    }

    Ok(HarmonicSpectrum {
        trace_id: trace.trace_id.clone(),
        channel,
        pole_pairs,
        maximum_harmonic_multiple,
        dc_offset_t,
        total_rms_t,
        fundamental_amplitude_t,
        fundamental_phase_rad,
        total_harmonic_distortion_ratio,
        unresolved_spectral_residual_ratio,
        combined_nonfundamental_ratio,
        endpoint_discontinuity_t,
        harmonics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn latch(axis_angle_rad: f64, initial_high: bool, bop: f64, brp: f64) -> HallLatchConfig {
        HallLatchConfig {
            axis_angle_rad,
            bop_threshold_t: bop,
            brp_threshold_t: brp,
            initial_high,
        }
    }

    fn config(pole_pairs: u32) -> QuadratureConfig {
        QuadratureConfig {
            pole_pairs,
            architecture: SensorArchitecture::IntegratedTwoAxisLatch,
            channel_a: latch(0.0, true, 0.0001, -0.0001),
            channel_b: latch(PI / 2.0, false, 0.0001, -0.0001),
            edge_rate_policy: None,
        }
    }

    fn sinusoidal_trace(
        trace_id: &str,
        pole_pairs: u32,
        sample_count: usize,
        amplitude_t: f64,
        phase_direction: f64,
    ) -> EncoderTrace {
        let samples = (0..sample_count)
            .map(|index| {
                let angle = TAU * index as f64 / (sample_count - 1) as f64;
                let phase = phase_direction * pole_pairs as f64 * angle;
                let field = InPlaneField {
                    radial_t: amplitude_t * phase.cos(),
                    tangential_t: amplitude_t * phase.sin(),
                };
                EncoderSample {
                    mechanical_angle_rad: angle,
                    timestamp_ns: Some(index as u64 * 100_000),
                    channel_a_field: field,
                    channel_b_field: field,
                }
            })
            .collect();

        EncoderTrace {
            trace_id: trace_id.to_owned(),
            coordinate_frame_id: "ring-sensor-frame-v0".to_owned(),
            operating_condition_id: "gap-3p25mm-temp-25C-v0".to_owned(),
            angular_grid_id: format!("full-turn-{sample_count}-uniform"),
            evidence_kind: EvidenceKind::SyntheticScenario,
            samples,
        }
    }


    fn discrete_spatial_trace(
        trace_id: &str,
        pole_pairs: u32,
        sample_count: usize,
        amplitude_t: f64,
        phase_direction: f64,
    ) -> EncoderTrace {
        let samples = (0..sample_count)
            .map(|index| {
                let angle = TAU * index as f64 / (sample_count - 1) as f64;
                let phase = phase_direction * pole_pairs as f64 * angle;
                EncoderSample {
                    mechanical_angle_rad: angle,
                    timestamp_ns: Some(index as u64 * 100_000),
                    channel_a_field: InPlaneField {
                        radial_t: amplitude_t * phase.cos(),
                        tangential_t: 0.0,
                    },
                    channel_b_field: InPlaneField {
                        radial_t: amplitude_t * phase.sin(),
                        tangential_t: 0.0,
                    },
                }
            })
            .collect();

        EncoderTrace {
            trace_id: trace_id.to_owned(),
            coordinate_frame_id: "two-latch-ring-frame-v0".to_owned(),
            operating_condition_id: "two-latch-9-degree-spacing-gap-3p25mm-v0".to_owned(),
            angular_grid_id: format!("full-turn-{sample_count}-uniform"),
            evidence_kind: EvidenceKind::SyntheticScenario,
            samples,
        }
    }


    #[test]
    fn pure_fundamental_field_has_negligible_higher_harmonics() {
        let trace = sinusoidal_trace("pure-fundamental", 10, 3601, 0.008, 1.0);
        let spectrum = analyze_channel_harmonics(&trace, HallChannel::A, 10, 0.0, 5).unwrap();

        assert!((spectrum.fundamental_amplitude_t.unwrap() - 0.008).abs() < 1.0e-10);
        assert!(spectrum.total_harmonic_distortion_ratio.unwrap() < 1.0e-10);
        assert!(spectrum.endpoint_discontinuity_t < 1.0e-10);
    }

    #[test]
    fn second_spatial_harmonic_is_measured_independently() {
        let mut trace = sinusoidal_trace("second-harmonic", 10, 3601, 0.008, 1.0);
        for sample in &mut trace.samples {
            let angle = sample.mechanical_angle_rad;
            sample.channel_a_field.radial_t =
                0.008 * (10.0 * angle).cos() + 0.0008 * (20.0 * angle + 0.31).cos();
        }
        let spectrum = analyze_channel_harmonics(&trace, HallChannel::A, 10, 0.0, 5).unwrap();

        assert!((spectrum.harmonics[1].amplitude_t - 0.0008).abs() < 1.0e-9);
        assert!((spectrum.total_harmonic_distortion_ratio.unwrap() - 0.1).abs() < 1.0e-7);
    }

    #[test]
    fn reports_endpoint_discontinuity_separately() {
        let mut trace = sinusoidal_trace("seam-discontinuity", 2, 721, 0.008, 1.0);
        trace.samples.last_mut().unwrap().channel_a_field.radial_t += 0.001;
        let spectrum = analyze_channel_harmonics(&trace, HallChannel::A, 2, 0.0, 3).unwrap();

        assert!((spectrum.endpoint_discontinuity_t - 0.001).abs() < 1.0e-12);
    }

    #[test]
    fn rejects_harmonic_order_at_or_above_nyquist() {
        let trace = sinusoidal_trace("alias-control", 10, 101, 0.008, 1.0);
        assert_eq!(
            analyze_channel_harmonics(&trace, HallChannel::A, 10, 0.0, 5),
            Err(EvaluationError::HarmonicOrderAboveNyquist)
        );
    }

    #[test]
    fn ideal_single_pole_pair_produces_four_valid_edges() {
        let trace = sinusoidal_trace("ideal-one-pair", 1, 3601, 0.008, 1.0);
        let result = evaluate_trace(&trace, config(1)).unwrap();
        assert!(result.metrics.quadrature_sequence_valid);
        assert_eq!(result.metrics.expected_transitions_per_revolution, 4);
        assert_eq!(result.metrics.observed_valid_transitions, 4);
        assert_eq!(result.metrics.illegal_two_bit_transitions, 0);
        assert!(result.transitions.iter().all(|edge| {
            edge.direction == Some(TransitionDirection::Forward)
        }));
    }

    #[test]
    fn twenty_pole_ring_means_ten_pairs_and_forty_edges() {
        let trace = sinusoidal_trace("ti-twenty-pole-geometry-control", 10, 3601, 0.008, 1.0);
        let result = evaluate_trace(&trace, config(10)).unwrap();
        assert!(result.metrics.quadrature_sequence_valid);
        assert_eq!(result.metrics.expected_transitions_per_revolution, 40);
        assert_eq!(result.metrics.observed_valid_transitions, 40);
    }

    #[test]
    fn reverse_field_rotation_reverses_quadrature_direction() {
        let trace = sinusoidal_trace("reverse-direction-control", 3, 3601, 0.008, -1.0);
        let result = evaluate_trace(&trace, config(3)).unwrap();
        assert!(result.metrics.quadrature_sequence_valid);
        assert!(result.transitions.iter().all(|edge| {
            edge.direction == Some(TransitionDirection::Reverse)
        }));
    }

    #[test]
    fn bop_brp_hysteresis_prevents_chatter_inside_deadband() {
        let latch_config = latch(0.0, false, 0.002, -0.002);
        let mut state = false;
        let field_values = [0.0, 0.001, 0.0019, 0.0021, 0.001, -0.0019, -0.001, -0.0021];
        let mut output = Vec::new();
        for field in field_values {
            state = latch_next_state(state, field, latch_config);
            output.push(state);
        }
        assert_eq!(output, vec![false, false, false, true, true, true, true, false]);
    }

    #[test]
    fn threshold_mismatch_keeps_channel_edges_explicit() {
        let trace = sinusoidal_trace("threshold-mismatch", 10, 3601, 0.008, 1.0);
        let mut cfg = config(10);
        cfg.channel_a = latch(0.0, true, 0.001, -0.001);
        cfg.channel_b = latch(PI / 2.0, false, 0.007, -0.007);
        let result = evaluate_trace(&trace, cfg).unwrap();
        assert_eq!(result.metrics.observed_valid_transitions, 40);
        let a_edges = result.metrics.channel_edge_counts.channel_a_rising
            + result.metrics.channel_edge_counts.channel_a_falling;
        let b_edges = result.metrics.channel_edge_counts.channel_b_rising
            + result.metrics.channel_edge_counts.channel_b_falling;
        assert_eq!(a_edges, 20);
        assert_eq!(b_edges, 20);
    }



    #[test]
    fn discrete_latches_can_use_separate_spatial_field_traces() {
        let trace = discrete_spatial_trace("two-latch-20-pole-control", 10, 3601, 0.008, 1.0);
        let mut cfg = config(10);
        cfg.architecture = SensorArchitecture::DiscreteSpatialLatches;
        cfg.channel_a = latch(0.0, true, 0.0001, -0.0001);
        cfg.channel_b = latch(0.0, false, 0.0001, -0.0001);

        let result = evaluate_trace(&trace, cfg).unwrap();
        assert!(result.metrics.quadrature_sequence_valid);
        assert_eq!(result.metrics.observed_valid_transitions, 40);
    }

    #[test]
    fn discrete_two_latch_threshold_mismatch_has_bounded_spacing_error() {
        let trace = discrete_spatial_trace("two-latch-threshold-mismatch", 10, 36001, 0.008, 1.0);
        let mut cfg = config(10);
        cfg.architecture = SensorArchitecture::DiscreteSpatialLatches;
        cfg.channel_a = latch(0.0, true, 0.001, -0.001);
        cfg.channel_b = latch(0.0, false, 0.007, -0.007);

        let result = evaluate_trace(&trace, cfg).unwrap();
        assert_eq!(result.metrics.observed_valid_transitions, 40);
        assert!(result.metrics.maximum_spacing_error_lower_bound_rad.unwrap() > 0.0);
        assert!(
            result.metrics.maximum_spacing_error_upper_bound_rad.unwrap()
                > result.metrics.maximum_spacing_error_lower_bound_rad.unwrap()
        );
    }

    #[test]
    fn threshold_mismatch_increases_bounded_quadrature_spacing_error() {
        let trace = sinusoidal_trace("spacing-baseline", 10, 36001, 0.008, 1.0);
        let baseline = evaluate_trace(&trace, config(10)).unwrap();

        let mut mismatch_config = config(10);
        mismatch_config.channel_a = latch(0.0, true, 0.001, -0.001);
        mismatch_config.channel_b = latch(PI / 2.0, false, 0.007, -0.007);
        let mismatch = evaluate_trace(&trace, mismatch_config).unwrap();

        assert_eq!(baseline.metrics.expected_quadrature_spacing_rad, TAU / 40.0);
        assert!(baseline.metrics.maximum_spacing_error_upper_bound_rad.is_some());
        assert!(
            mismatch.metrics.maximum_spacing_error_upper_bound_rad.unwrap()
                > baseline.metrics.maximum_spacing_error_upper_bound_rad.unwrap()
        );
    }

    #[test]
    fn detects_simultaneous_two_bit_change_as_illegal() {
        let mut trace = sinusoidal_trace("illegal-transition", 1, 5, 0.008, 1.0);
        trace.samples[0].channel_a_field = InPlaneField { radial_t: 0.0, tangential_t: 0.0 };
        trace.samples[0].channel_b_field = trace.samples[0].channel_a_field;
        trace.samples[1].channel_a_field = InPlaneField { radial_t: 0.008, tangential_t: 0.0 };
        trace.samples[1].channel_b_field = trace.samples[1].channel_a_field;
        trace.samples[2].channel_a_field = trace.samples[1].channel_a_field;
        trace.samples[2].channel_b_field = trace.samples[2].channel_a_field;
        trace.samples[3].channel_a_field = InPlaneField { radial_t: -0.008, tangential_t: 0.0 };
        trace.samples[3].channel_b_field = trace.samples[3].channel_a_field;
        trace.samples[4].channel_a_field = trace.samples[0].channel_a_field;
        trace.samples[4].channel_b_field = trace.samples[4].channel_a_field;
        let mut cfg = config(1);
        cfg.channel_a = latch(0.0, false, 0.002, -0.002);
        cfg.channel_b = latch(0.0, false, 0.002, -0.002);
        let result = evaluate_trace(&trace, cfg).unwrap();
        assert!(result.metrics.illegal_two_bit_transitions > 0);
        assert!(!result.metrics.quadrature_sequence_valid);
    }

    #[test]
    fn reports_missing_edges_for_constant_field() {
        let mut trace = sinusoidal_trace("missing-edges", 1, 361, 0.0, 1.0);
        for sample in &mut trace.samples {
            sample.channel_a_field = InPlaneField { radial_t: 0.0, tangential_t: 0.0 };
            sample.channel_b_field = sample.channel_a_field;
        }
        let mut cfg = config(1);
        cfg.channel_a.initial_high = false;
        let result = evaluate_trace(&trace, cfg).unwrap();
        assert_eq!(result.metrics.observed_valid_transitions, 0);
        assert_eq!(result.metrics.missing_transition_count, 4);
        assert!(!result.metrics.quadrature_sequence_valid);
    }

    #[test]
    fn computes_declared_edge_rate_budget_for_ten_pair_ring() {
        let trace = sinusoidal_trace("speed-budget", 10, 3601, 0.008, 1.0);
        let mut cfg = config(10);
        cfg.edge_rate_policy = Some(EdgeRatePolicy {
            rotational_speed_rpm: 6000.0,
            maximum_transition_rate_hz: 5000.0,
        });
        let result = evaluate_trace(&trace, cfg).unwrap();
        assert_eq!(result.metrics.expected_transition_rate_hz, Some(4000.0));
        assert_eq!(result.metrics.within_edge_rate_policy, Some(true));
    }

    #[test]
    fn rejects_speed_above_declared_edge_rate_policy() {
        let trace = sinusoidal_trace("speed-over-budget", 10, 3601, 0.008, 1.0);
        let mut cfg = config(10);
        cfg.edge_rate_policy = Some(EdgeRatePolicy {
            rotational_speed_rpm: 6000.0,
            maximum_transition_rate_hz: 3000.0,
        });
        let result = evaluate_trace(&trace, cfg).unwrap();
        assert_eq!(result.metrics.expected_transition_rate_hz, Some(4000.0));
        assert_eq!(result.metrics.within_edge_rate_policy, Some(false));
    }

    #[test]
    fn transition_brackets_preserve_sample_interval_uncertainty() {
        let trace = sinusoidal_trace("edge-bracket", 1, 3601, 0.008, 1.0);
        let result = evaluate_trace(&trace, config(1)).unwrap();
        assert!(result.transitions.iter().all(|edge| {
            edge.upper_angle_rad > edge.lower_angle_rad && edge.timestamp_bracket_ns.is_some()
        }));
        assert!(result.metrics.maximum_transition_bracket_rad.unwrap() <= TAU / 3600.0 + 1e-12);
    }

    #[test]
    fn rejects_bop_not_greater_than_brp() {
        let trace = sinusoidal_trace("invalid-hysteresis", 1, 361, 0.008, 1.0);
        let mut cfg = config(1);
        cfg.channel_b.bop_threshold_t = -0.001;
        cfg.channel_b.brp_threshold_t = 0.001;
        assert_eq!(
            evaluate_trace(&trace, cfg),
            Err(EvaluationError::InvalidThresholdHysteresis)
        );
    }

    #[test]
    fn requires_initial_latch_state_to_match_starting_field() {
        let trace = sinusoidal_trace("bad-initial-state", 1, 361, 0.008, 1.0);
        let mut cfg = config(1);
        cfg.channel_a.initial_high = false;
        assert_eq!(
            evaluate_trace(&trace, cfg),
            Err(EvaluationError::InitialStateInconsistent { channel: 'A' })
        );
    }


    #[test]
    fn integrated_sensor_reports_nonorthogonal_sensing_axes() {
        let trace = sinusoidal_trace("nonorthogonal-integrated-axes", 1, 361, 0.008, 1.0);
        let mut cfg = config(1);
        cfg.channel_b.axis_angle_rad = PI / 3.0;

        let result = evaluate_trace(&trace, cfg).unwrap();
        assert!(
            (result.metrics.integrated_axis_nonorthogonality_rad.unwrap() - PI / 6.0).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn integrated_sensor_requires_one_shared_local_field_vector() {
        let mut trace = sinusoidal_trace("integrated-field-mismatch", 1, 361, 0.008, 1.0);
        trace.samples[22].channel_b_field.radial_t += 0.0001;
        assert_eq!(
            evaluate_trace(&trace, config(1)),
            Err(EvaluationError::IntegratedSensorFieldMismatch { index: 22 })
        );
    }

    #[test]
    fn requires_uniform_full_revolution() {
        let mut trace = sinusoidal_trace("incomplete-revolution", 1, 361, 0.008, 1.0);
        trace.samples[100].mechanical_angle_rad += 0.001;
        assert_eq!(
            evaluate_trace(&trace, config(1)),
            Err(EvaluationError::NonUniformOrIncompleteRevolution { index: 100 })
        );
    }

    #[test]
    fn rejects_nonfinite_field_values() {
        let mut trace = sinusoidal_trace("nonfinite-field", 1, 361, 0.008, 1.0);
        trace.samples[22].channel_b_field.tangential_t = f64::NAN;
        assert_eq!(
            evaluate_trace(&trace, config(1)),
            Err(EvaluationError::NonFiniteSample { index: 22 })
        );
    }

    #[test]
    fn rejects_nonmonotonic_timestamps() {
        let mut trace = sinusoidal_trace("bad-time", 1, 361, 0.008, 1.0);
        trace.samples[100].timestamp_ns = trace.samples[99].timestamp_ns;
        assert_eq!(
            evaluate_trace(&trace, config(1)),
            Err(EvaluationError::NonMonotonicTimestamp { index: 100 })
        );
    }

    #[test]
    fn requires_timestamps_for_all_samples_or_none() {
        let mut trace = sinusoidal_trace("mixed-time", 1, 361, 0.008, 1.0);
        trace.samples[100].timestamp_ns = None;
        assert_eq!(
            evaluate_trace(&trace, config(1)),
            Err(EvaluationError::MixedTimestampAvailability)
        );
    }
}
