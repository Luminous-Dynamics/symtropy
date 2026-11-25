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

#[derive(Clone, Debug, PartialEq)]
pub struct QuadratureConfig {
    /// Immutable identity for the declared sensor/latch parameter set.
    pub configuration_id: String,
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
    pub sensor_configuration_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub angular_grid_id: String,
    pub pole_pairs: u32,
    pub architecture: SensorArchitecture,
    pub start_angle_rad: f64,
    pub end_angle_rad: f64,
    pub sample_count: usize,
    pub states: Vec<QuadratureState>,
    pub transitions: Vec<QuadratureTransition>,
    pub metrics: QuadratureMetrics,
}


#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioResult {
    pub trace_id: String,
    pub operating_condition_id: String,
    pub accepted: bool,
    pub metrics: QuadratureMetrics,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioSetMetrics {
    pub set_id: String,
    pub evidence_kind: EvidenceKind,
    pub coordinate_frame_id: String,
    pub angular_grid_id: String,
    pub scenario_count: usize,
    pub accepted_scenario_count: usize,
    pub rejected_scenario_count: usize,
    pub all_scenarios_accepted: bool,
    pub worst_missing_transition_count: usize,
    pub worst_excess_transition_count: usize,
    pub worst_illegal_two_bit_transition_count: usize,
    pub worst_direction_reversal_count: usize,
    pub worst_spacing_error_lower_bound_rad: Option<f64>,
    pub worst_spacing_error_upper_bound_rad: Option<f64>,
    pub worst_transition_bracket_rad: Option<f64>,
    pub scenarios: Vec<ScenarioResult>,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CapturedQuadratureEdge {
    /// Independent acquisition timestamp, in nanoseconds.
    pub timestamp_ns: u64,
    /// Rotor mechanical angle at the captured edge from the independent reference.
    pub mechanical_angle_rad: f64,
    /// Observed A/B state immediately after this edge.
    pub state_after: QuadratureState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CapturedQuadratureTrace {
    pub capture_id: String,
    pub source_field_trace_id: String,
    pub sensor_part_id: String,
    pub sensor_datasheet_revision: String,
    pub channel_mapping_id: String,
    pub sensor_configuration_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub rotor_reference_record_id: String,
    pub clock_calibration_record_id: String,
    pub synchronization_record_id: String,
    pub latency_calibration_record_id: String,
    /// Signed correction already applied to the reference/capture alignment.
    pub latency_correction_applied_ns: i64,
    pub latency_standard_uncertainty_ns: f64,
    pub clock_standard_uncertainty_ns: f64,
    pub rotor_angle_standard_uncertainty_rad: f64,
    pub start_angle_rad: f64,
    pub end_angle_rad: f64,
    pub initial_state: QuadratureState,
    pub transitions: Vec<CapturedQuadratureEdge>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CaptureReconciliationMetrics {
    pub capture_id: String,
    pub source_field_trace_id: String,
    pub sensor_part_id: String,
    pub sensor_configuration_id: String,
    pub predicted_transition_count: usize,
    pub captured_transition_count: usize,
    pub missing_captured_transition_count: usize,
    pub extra_captured_transition_count: usize,
    pub initial_state_match: bool,
    pub captured_illegal_two_bit_transition_count: usize,
    pub state_after_mismatch_count: usize,
    pub direction_mismatch_count: usize,
    pub captured_edges_inside_predicted_bracket_count: usize,
    pub captured_edges_outside_predicted_bracket_count: usize,
    pub maximum_angle_distance_to_predicted_bracket_rad: Option<f64>,
    pub maximum_absolute_midpoint_residual_rad: Option<f64>,
    pub mean_signed_midpoint_residual_rad: Option<f64>,
    pub minimum_captured_edge_interval_ns: Option<u64>,
    pub maximum_captured_edge_interval_ns: Option<u64>,
    pub latency_correction_applied_ns: i64,
    pub latency_standard_uncertainty_ns: f64,
    pub clock_standard_uncertainty_ns: f64,
    pub rotor_angle_standard_uncertainty_rad: f64,
    /// Exact transition/state correspondence and bracket inclusion, not a physical qualification.
    pub state_sequence_and_bracket_match: bool,
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
    EmptyScenarioSet,
    InvalidScenarioSetId,
    DuplicateScenarioTraceId,
    MixedScenarioEvidenceKinds,
    ScenarioFrameMismatch,
    ScenarioGridMismatch,
    ScenarioGridCoordinatesMismatch,
    InvalidCaptureMetadata,
    CaptureFieldTraceIdentityMismatch,
    CaptureSensorConfigurationMismatch,
    CaptureFrameMismatch,
    CaptureOperatingConditionMismatch,
    CaptureAngleWindowMismatch,
    NonFiniteCapturedAngle { index: usize },
    CapturedAngleOutOfWindow { index: usize },
    NonMonotonicCapturedAngle { index: usize },
    NonMonotonicCapturedTimestamp { index: usize },
    CapturedNoStateChange { index: usize },
    InvalidCaptureUncertainty,
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

fn validate_config(config: &QuadratureConfig) -> Result<(), EvaluationError> {
    if config.configuration_id.trim().is_empty() || config.pole_pairs == 0 {
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



/// Evaluate a finite, explicitly declared set of operating-condition traces
/// against one fixed sensor and latch configuration.
///
/// Different operating-condition IDs are allowed and preserved separately
/// (for example, distinct air gaps, eccentricity, tilt or temperature). All
/// scenarios must share evidence class, coordinate frame, angular-grid ID,
/// sample count and bit-identical angle coordinates. The same config is applied
/// to every trace. A finite set is not treated as an IID manufacturing sample
/// and no probability, yield or CVaR estimate is emitted.
pub fn evaluate_scenario_set(
    set_id: &str,
    traces: &[EncoderTrace],
    config: QuadratureConfig,
) -> Result<ScenarioSetMetrics, EvaluationError> {
    if set_id.trim().is_empty() {
        return Err(EvaluationError::InvalidScenarioSetId);
    }
    let first = traces.first().ok_or(EvaluationError::EmptyScenarioSet)?;
    let mut trace_ids = std::collections::HashSet::with_capacity(traces.len());
    let mut scenarios = Vec::with_capacity(traces.len());

    let mut accepted_scenario_count = 0_usize;
    let mut worst_missing_transition_count = 0_usize;
    let mut worst_excess_transition_count = 0_usize;
    let mut worst_illegal_two_bit_transition_count = 0_usize;
    let mut worst_direction_reversal_count = 0_usize;
    let mut worst_spacing_error_lower_bound_rad = None::<f64>;
    let mut worst_spacing_error_upper_bound_rad = None::<f64>;
    let mut worst_transition_bracket_rad = None::<f64>;

    for trace in traces {
        if !trace_ids.insert(trace.trace_id.as_str()) {
            return Err(EvaluationError::DuplicateScenarioTraceId);
        }
        if trace.evidence_kind != first.evidence_kind {
            return Err(EvaluationError::MixedScenarioEvidenceKinds);
        }
        if trace.coordinate_frame_id != first.coordinate_frame_id {
            return Err(EvaluationError::ScenarioFrameMismatch);
        }
        if trace.angular_grid_id != first.angular_grid_id {
            return Err(EvaluationError::ScenarioGridMismatch);
        }
        if trace.samples.len() != first.samples.len()
            || trace
                .samples
                .iter()
                .zip(first.samples.iter())
                .any(|(left, right)| {
                    left.mechanical_angle_rad.to_bits()
                        != right.mechanical_angle_rad.to_bits()
                })
        {
            return Err(EvaluationError::ScenarioGridCoordinatesMismatch);
        }

        let evaluation = evaluate_trace(trace, config.clone())?;
        let accepted = evaluation.metrics.quadrature_sequence_valid
            && evaluation.metrics.within_edge_rate_policy != Some(false);
        if accepted {
            accepted_scenario_count += 1;
        }

        let metrics = evaluation.metrics;
        worst_missing_transition_count =
            worst_missing_transition_count.max(metrics.missing_transition_count);
        worst_excess_transition_count =
            worst_excess_transition_count.max(metrics.excess_transition_count);
        worst_illegal_two_bit_transition_count = worst_illegal_two_bit_transition_count
            .max(metrics.illegal_two_bit_transitions);
        worst_direction_reversal_count =
            worst_direction_reversal_count.max(metrics.direction_reversal_count);
        if let Some(value) = metrics.maximum_spacing_error_lower_bound_rad {
            worst_spacing_error_lower_bound_rad = Some(
                worst_spacing_error_lower_bound_rad
                    .map_or(value, |current| current.max(value)),
            );
        }
        if let Some(value) = metrics.maximum_spacing_error_upper_bound_rad {
            worst_spacing_error_upper_bound_rad = Some(
                worst_spacing_error_upper_bound_rad
                    .map_or(value, |current| current.max(value)),
            );
        }
        if let Some(value) = metrics.maximum_transition_bracket_rad {
            worst_transition_bracket_rad =
                Some(worst_transition_bracket_rad.map_or(value, |current| current.max(value)));
        }

        scenarios.push(ScenarioResult {
            trace_id: trace.trace_id.clone(),
            operating_condition_id: trace.operating_condition_id.clone(),
            accepted,
            metrics,
        });
    }

    let rejected_scenario_count = traces.len() - accepted_scenario_count;
    Ok(ScenarioSetMetrics {
        set_id: set_id.to_owned(),
        evidence_kind: first.evidence_kind,
        coordinate_frame_id: first.coordinate_frame_id.clone(),
        angular_grid_id: first.angular_grid_id.clone(),
        scenario_count: traces.len(),
        accepted_scenario_count,
        rejected_scenario_count,
        all_scenarios_accepted: rejected_scenario_count == 0,
        worst_missing_transition_count,
        worst_excess_transition_count,
        worst_illegal_two_bit_transition_count,
        worst_direction_reversal_count,
        worst_spacing_error_lower_bound_rad,
        worst_spacing_error_upper_bound_rad,
        worst_transition_bracket_rad,
        scenarios,
    })
}


/// Reconcile a timestamped hardware A/B capture against predicted Hall-latch
/// transitions. The capture has its own identity and independent rotor/clock
/// calibration provenance; it is not a magnetic field trace.
///
/// Captured edge angles are compared with the predicted field-sample brackets.
/// No silent stream shifting, transition deletion or sub-sample interpolation is
/// performed. The supplied angle values must already reflect the declared
/// synchronization/latency correction; the correction and uncertainty are
/// preserved in the output rather than being guessed or applied here.
pub fn reconcile_captured_output(
    predicted: &SensorEvaluation,
    capture: &CapturedQuadratureTrace,
) -> Result<CaptureReconciliationMetrics, EvaluationError> {
    let required_ids = [
        capture.capture_id.as_str(),
        capture.source_field_trace_id.as_str(),
        capture.sensor_part_id.as_str(),
        capture.sensor_datasheet_revision.as_str(),
        capture.channel_mapping_id.as_str(),
        capture.sensor_configuration_id.as_str(),
        capture.coordinate_frame_id.as_str(),
        capture.operating_condition_id.as_str(),
        capture.rotor_reference_record_id.as_str(),
        capture.clock_calibration_record_id.as_str(),
        capture.synchronization_record_id.as_str(),
        capture.latency_calibration_record_id.as_str(),
    ];
    if required_ids.into_iter().any(|value| !nonempty(value)) {
        return Err(EvaluationError::InvalidCaptureMetadata);
    }

    if capture.source_field_trace_id != predicted.trace_id {
        return Err(EvaluationError::CaptureFieldTraceIdentityMismatch);
    }
    if capture.sensor_configuration_id != predicted.sensor_configuration_id {
        return Err(EvaluationError::CaptureSensorConfigurationMismatch);
    }
    if capture.coordinate_frame_id != predicted.coordinate_frame_id {
        return Err(EvaluationError::CaptureFrameMismatch);
    }
    if capture.operating_condition_id != predicted.operating_condition_id {
        return Err(EvaluationError::CaptureOperatingConditionMismatch);
    }

    if !capture.start_angle_rad.is_finite()
        || !capture.end_angle_rad.is_finite()
        || (capture.start_angle_rad - predicted.start_angle_rad).abs() > ANGLE_TOLERANCE_RAD
        || (capture.end_angle_rad - predicted.end_angle_rad).abs() > ANGLE_TOLERANCE_RAD
        || (capture.end_angle_rad - capture.start_angle_rad - TAU).abs()
            > ANGLE_TOLERANCE_RAD
    {
        return Err(EvaluationError::CaptureAngleWindowMismatch);
    }

    if !capture.latency_standard_uncertainty_ns.is_finite()
        || capture.latency_standard_uncertainty_ns < 0.0
        || !capture.clock_standard_uncertainty_ns.is_finite()
        || capture.clock_standard_uncertainty_ns < 0.0
        || !capture.rotor_angle_standard_uncertainty_rad.is_finite()
        || capture.rotor_angle_standard_uncertainty_rad < 0.0
    {
        return Err(EvaluationError::InvalidCaptureUncertainty);
    }

    let initial_state_match = capture.initial_state
        == predicted
            .transitions
            .first()
            .map_or_else(
                || predicted.states.first().copied().unwrap_or(capture.initial_state),
                |transition| transition.from,
            );

    let mut captured_illegal_two_bit_transition_count = 0_usize;
    let mut previous_state = capture.initial_state;
    let mut previous_timestamp = None::<u64>;
    let mut previous_angle = None::<f64>;
    let mut minimum_captured_edge_interval_ns = None::<u64>;
    let mut maximum_captured_edge_interval_ns = None::<u64>;

    for (index, edge) in capture.transitions.iter().enumerate() {
        if !edge.mechanical_angle_rad.is_finite() {
            return Err(EvaluationError::NonFiniteCapturedAngle { index });
        }
        if edge.mechanical_angle_rad <= capture.start_angle_rad
            || edge.mechanical_angle_rad > capture.end_angle_rad
        {
            return Err(EvaluationError::CapturedAngleOutOfWindow { index });
        }
        if previous_angle.is_some_and(|previous| edge.mechanical_angle_rad <= previous) {
            return Err(EvaluationError::NonMonotonicCapturedAngle { index });
        }
        if previous_timestamp.is_some_and(|previous| edge.timestamp_ns <= previous) {
            return Err(EvaluationError::NonMonotonicCapturedTimestamp { index });
        }
        if edge.state_after == previous_state {
            return Err(EvaluationError::CapturedNoStateChange { index });
        }

        let (_, illegal) = classify_transition(previous_state, edge.state_after);
        if illegal {
            captured_illegal_two_bit_transition_count += 1;
        }
        if let Some(previous) = previous_timestamp {
            let interval = edge.timestamp_ns - previous;
            minimum_captured_edge_interval_ns =
                Some(minimum_captured_edge_interval_ns.map_or(interval, |value| value.min(interval)));
            maximum_captured_edge_interval_ns =
                Some(maximum_captured_edge_interval_ns.map_or(interval, |value| value.max(interval)));
        }

        previous_state = edge.state_after;
        previous_timestamp = Some(edge.timestamp_ns);
        previous_angle = Some(edge.mechanical_angle_rad);
    }

    let predicted_transition_count = predicted.transitions.len();
    let captured_transition_count = capture.transitions.len();
    let matched_count = predicted_transition_count.min(captured_transition_count);
    let missing_captured_transition_count =
        predicted_transition_count.saturating_sub(captured_transition_count);
    let extra_captured_transition_count =
        captured_transition_count.saturating_sub(predicted_transition_count);

    let mut state_after_mismatch_count = 0_usize;
    let mut direction_mismatch_count = 0_usize;
    let mut captured_edges_inside_predicted_bracket_count = 0_usize;
    let mut captured_edges_outside_predicted_bracket_count = 0_usize;
    let mut maximum_angle_distance_to_predicted_bracket_rad = None::<f64>;
    let mut maximum_absolute_midpoint_residual_rad = None::<f64>;
    let mut signed_midpoint_residual_sum_rad = 0.0_f64;

    for index in 0..matched_count {
        let expected = &predicted.transitions[index];
        let observed = &capture.transitions[index];

        if observed.state_after != expected.to {
            state_after_mismatch_count += 1;
        }
        let observed_previous = if index == 0 {
            capture.initial_state
        } else {
            capture.transitions[index - 1].state_after
        };
        let (observed_direction, observed_illegal) =
            classify_transition(observed_previous, observed.state_after);
        if observed_illegal || observed_direction != expected.direction {
            direction_mismatch_count += 1;
        }

        let inside = observed.mechanical_angle_rad >= expected.lower_angle_rad
            && observed.mechanical_angle_rad <= expected.upper_angle_rad;
        if inside {
            captured_edges_inside_predicted_bracket_count += 1;
        } else {
            captured_edges_outside_predicted_bracket_count += 1;
        }

        let distance_to_bracket = if observed.mechanical_angle_rad < expected.lower_angle_rad {
            expected.lower_angle_rad - observed.mechanical_angle_rad
        } else if observed.mechanical_angle_rad > expected.upper_angle_rad {
            observed.mechanical_angle_rad - expected.upper_angle_rad
        } else {
            0.0
        };
        maximum_angle_distance_to_predicted_bracket_rad = Some(
            maximum_angle_distance_to_predicted_bracket_rad
                .map_or(distance_to_bracket, |value| value.max(distance_to_bracket)),
        );

        let midpoint = (expected.lower_angle_rad + expected.upper_angle_rad) / 2.0;
        let midpoint_residual = observed.mechanical_angle_rad - midpoint;
        signed_midpoint_residual_sum_rad += midpoint_residual;
        maximum_absolute_midpoint_residual_rad = Some(
            maximum_absolute_midpoint_residual_rad
                .map_or(midpoint_residual.abs(), |value| value.max(midpoint_residual.abs())),
        );
    }

    let mean_signed_midpoint_residual_rad = (matched_count > 0).then_some(
        signed_midpoint_residual_sum_rad / matched_count as f64,
    );
    let state_sequence_and_bracket_match = initial_state_match
        && missing_captured_transition_count == 0
        && extra_captured_transition_count == 0
        && captured_illegal_two_bit_transition_count == 0
        && state_after_mismatch_count == 0
        && direction_mismatch_count == 0
        && predicted.metrics.illegal_two_bit_transitions == 0
        && captured_edges_outside_predicted_bracket_count == 0;

    Ok(CaptureReconciliationMetrics {
        capture_id: capture.capture_id.clone(),
        source_field_trace_id: capture.source_field_trace_id.clone(),
        sensor_part_id: capture.sensor_part_id.clone(),
        sensor_configuration_id: capture.sensor_configuration_id.clone(),
        predicted_transition_count,
        captured_transition_count,
        missing_captured_transition_count,
        extra_captured_transition_count,
        initial_state_match,
        captured_illegal_two_bit_transition_count,
        state_after_mismatch_count,
        direction_mismatch_count,
        captured_edges_inside_predicted_bracket_count,
        captured_edges_outside_predicted_bracket_count,
        maximum_angle_distance_to_predicted_bracket_rad,
        maximum_absolute_midpoint_residual_rad,
        mean_signed_midpoint_residual_rad,
        minimum_captured_edge_interval_ns,
        maximum_captured_edge_interval_ns,
        latency_correction_applied_ns: capture.latency_correction_applied_ns,
        latency_standard_uncertainty_ns: capture.latency_standard_uncertainty_ns,
        clock_standard_uncertainty_ns: capture.clock_standard_uncertainty_ns,
        rotor_angle_standard_uncertainty_rad: capture.rotor_angle_standard_uncertainty_rad,
        state_sequence_and_bracket_match,
    })
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
    validate_config(&config)?;
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
        sensor_configuration_id: config.configuration_id.clone(),
        coordinate_frame_id: trace.coordinate_frame_id.clone(),
        operating_condition_id: trace.operating_condition_id.clone(),
        angular_grid_id: trace.angular_grid_id.clone(),
        pole_pairs: config.pole_pairs,
        architecture: config.architecture,
        start_angle_rad: start_angle,
        end_angle_rad: final_angle,
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
            configuration_id: "tmag511x-latch-config-v0".to_owned(),
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
    fn finite_scenario_set_accepts_different_operating_conditions() {
        let nominal = sinusoidal_trace("nominal-gap", 10, 3601, 0.008, 1.0);
        let mut wider_gap = sinusoidal_trace("wider-gap", 10, 3601, 0.007, 1.0);
        wider_gap.operating_condition_id = "gap-4p0mm-temp-25C-v0".to_owned();

        let result = evaluate_scenario_set("air-gap-sweep-v0", &[nominal, wider_gap], config(10))
            .unwrap();

        assert_eq!(result.scenario_count, 2);
        assert_eq!(result.accepted_scenario_count, 2);
        assert!(result.all_scenarios_accepted);
        assert_ne!(
            result.scenarios[0].operating_condition_id,
            result.scenarios[1].operating_condition_id
        );
    }

    #[test]
    fn finite_scenario_set_reports_worst_case_failure_without_yield() {
        let nominal = sinusoidal_trace("nominal", 10, 3601, 0.008, 1.0);
        let missing_signal = sinusoidal_trace("low-field", 10, 3601, 0.00001, 1.0);
        let result =
            evaluate_scenario_set("field-margin-sweep-v0", &[nominal, missing_signal], config(10))
                .unwrap();

        assert_eq!(result.scenario_count, 2);
        assert_eq!(result.accepted_scenario_count, 1);
        assert_eq!(result.rejected_scenario_count, 1);
        assert!(!result.all_scenarios_accepted);
        assert!(result
            .scenarios
            .iter()
            .any(|scenario| !scenario.accepted && scenario.metrics.missing_transition_count > 0));
    }

    #[test]
    fn finite_scenario_set_rejects_mixed_evidence_classes() {
        let physical = sinusoidal_trace("physical-map", 1, 361, 0.008, 1.0);
        let mut synthetic = sinusoidal_trace("synthetic-map", 1, 361, 0.008, 1.0);
        synthetic.evidence_kind = EvidenceKind::SyntheticScenario;
        let mut physical = physical;
        physical.evidence_kind = EvidenceKind::CalibratedFieldMeasurement;

        assert_eq!(
            evaluate_scenario_set("mixed-evidence", &[physical, synthetic], config(1)),
            Err(EvaluationError::MixedScenarioEvidenceKinds)
        );
    }

    #[test]
    fn finite_scenario_set_requires_identical_grid_and_frame() {
        let first = sinusoidal_trace("first", 1, 361, 0.008, 1.0);
        let mut grid_mismatch = sinusoidal_trace("grid-mismatch", 1, 361, 0.008, 1.0);
        grid_mismatch.angular_grid_id = "other-grid".to_owned();
        assert_eq!(
            evaluate_scenario_set("grid-test", &[first.clone(), grid_mismatch], config(1)),
            Err(EvaluationError::ScenarioGridMismatch)
        );

        let mut frame_mismatch = first.clone();
        frame_mismatch.trace_id = "frame-mismatch".to_owned();
        frame_mismatch.coordinate_frame_id = "other-frame".to_owned();
        assert_eq!(
            evaluate_scenario_set("frame-test", &[first, frame_mismatch], config(1)),
            Err(EvaluationError::ScenarioFrameMismatch)
        );
    }

    #[test]
    fn finite_scenario_set_rejects_duplicate_trace_ids_and_empty_set() {
        let trace = sinusoidal_trace("duplicate", 1, 361, 0.008, 1.0);
        assert_eq!(
            evaluate_scenario_set("duplicate-set", &[trace.clone(), trace], config(1)),
            Err(EvaluationError::DuplicateScenarioTraceId)
        );
        assert_eq!(
            evaluate_scenario_set("empty-set", &[], config(1)),
            Err(EvaluationError::EmptyScenarioSet)
        );
    }


    fn captured_trace_from_prediction(
        prediction: &SensorEvaluation,
    ) -> CapturedQuadratureTrace {
        CapturedQuadratureTrace {
            capture_id: "logic-analyzer-capture-001".to_owned(),
            source_field_trace_id: prediction.trace_id.clone(),
            sensor_part_id: "TMAG5111-EVM-001".to_owned(),
            sensor_datasheet_revision: "TMAG5111-datasheet-rev-pinned".to_owned(),
            channel_mapping_id: "A-to-CH1-B-to-CH2".to_owned(),
            sensor_configuration_id: prediction.sensor_configuration_id.clone(),
            coordinate_frame_id: prediction.coordinate_frame_id.clone(),
            operating_condition_id: prediction.operating_condition_id.clone(),
            rotor_reference_record_id: "rotary-reference-cal-001".to_owned(),
            clock_calibration_record_id: "daq-clock-cal-001".to_owned(),
            synchronization_record_id: "daq-rotor-sync-001".to_owned(),
            latency_calibration_record_id: "sensor-latency-cal-001".to_owned(),
            latency_correction_applied_ns: 1200,
            latency_standard_uncertainty_ns: 50.0,
            clock_standard_uncertainty_ns: 10.0,
            rotor_angle_standard_uncertainty_rad: 1.0e-5,
            start_angle_rad: prediction.start_angle_rad,
            end_angle_rad: prediction.end_angle_rad,
            initial_state: prediction.states[0],
            transitions: prediction
                .transitions
                .iter()
                .enumerate()
                .map(|(index, transition)| CapturedQuadratureEdge {
                    timestamp_ns: 1_000_000 + index as u64 * 100_000,
                    mechanical_angle_rad: (transition.lower_angle_rad
                        + transition.upper_angle_rad)
                        / 2.0,
                    state_after: transition.to,
                })
                .collect(),
        }
    }

    #[test]
    fn capture_reconciliation_accepts_exact_synthetic_edge_replay() {
        let trace = sinusoidal_trace("predicted-field-trace-001", 10, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(10)).unwrap();
        let capture = captured_trace_from_prediction(&prediction);

        let metrics = reconcile_captured_output(&prediction, &capture).unwrap();

        assert!(metrics.state_sequence_and_bracket_match);
        assert_eq!(metrics.predicted_transition_count, 40);
        assert_eq!(metrics.captured_transition_count, 40);
        assert_eq!(metrics.captured_edges_inside_predicted_bracket_count, 40);
        assert_eq!(metrics.captured_edges_outside_predicted_bracket_count, 0);
        assert_eq!(metrics.latency_correction_applied_ns, 1200);
    }

    #[test]
    fn capture_reconciliation_counts_missing_and_extra_edges() {
        let trace = sinusoidal_trace("predicted-field-trace-counts", 2, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(2)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.transitions.pop();
        capture.transitions.push(CapturedQuadratureEdge {
            timestamp_ns: 9_000_000,
            mechanical_angle_rad: prediction.end_angle_rad - 1.0e-5,
            state_after: QuadratureState {
                channel_a_high: !capture.transitions.last().unwrap().state_after.channel_a_high,
                channel_b_high: capture.transitions.last().unwrap().state_after.channel_b_high,
            },
        });

        let metrics = reconcile_captured_output(&prediction, &capture).unwrap();
        assert_eq!(metrics.missing_captured_transition_count, 0);
        assert_eq!(metrics.extra_captured_transition_count, 0);
        assert!(!metrics.state_sequence_and_bracket_match);
        assert!(metrics.state_after_mismatch_count > 0 || metrics.captured_edges_outside_predicted_bracket_count > 0);
    }

    #[test]
    fn capture_reconciliation_detects_wrong_state_after_edge() {
        let trace = sinusoidal_trace("predicted-field-trace-state", 1, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(1)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.transitions[0].state_after = QuadratureState {
            channel_a_high: false,
            channel_b_high: false,
        };

        let metrics = reconcile_captured_output(&prediction, &capture).unwrap();
        assert!(metrics.state_after_mismatch_count > 0);
        assert!(!metrics.state_sequence_and_bracket_match);
    }

    #[test]
    fn capture_reconciliation_rejects_sensor_configuration_mismatch() {
        let trace = sinusoidal_trace("predicted-field-trace-config", 1, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(1)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.sensor_configuration_id = "different-config".to_owned();

        assert_eq!(
            reconcile_captured_output(&prediction, &capture),
            Err(EvaluationError::CaptureSensorConfigurationMismatch)
        );
    }

    #[test]
    fn capture_reconciliation_rejects_wrong_field_trace_and_condition() {
        let trace = sinusoidal_trace("predicted-field-trace-identity", 1, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(1)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.source_field_trace_id = "other-trace".to_owned();
        assert_eq!(
            reconcile_captured_output(&prediction, &capture),
            Err(EvaluationError::CaptureFieldTraceIdentityMismatch)
        );

        let mut condition_mismatch = captured_trace_from_prediction(&prediction);
        condition_mismatch.operating_condition_id = "other-air-gap".to_owned();
        assert_eq!(
            reconcile_captured_output(&prediction, &condition_mismatch),
            Err(EvaluationError::CaptureOperatingConditionMismatch)
        );
    }

    #[test]
    fn capture_reconciliation_rejects_nonmonotonic_timestamp_or_angle() {
        let trace = sinusoidal_trace("predicted-field-trace-monotonic", 1, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(1)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.transitions[1].timestamp_ns = capture.transitions[0].timestamp_ns;
        assert_eq!(
            reconcile_captured_output(&prediction, &capture),
            Err(EvaluationError::NonMonotonicCapturedTimestamp { index: 1 })
        );

        let mut angle_mismatch = captured_trace_from_prediction(&prediction);
        angle_mismatch.transitions[1].mechanical_angle_rad =
            angle_mismatch.transitions[0].mechanical_angle_rad;
        assert_eq!(
            reconcile_captured_output(&prediction, &angle_mismatch),
            Err(EvaluationError::NonMonotonicCapturedAngle { index: 1 })
        );
    }

    #[test]
    fn capture_reconciliation_preserves_uncalibrated_latency_metadata() {
        let trace = sinusoidal_trace("predicted-field-trace-latency", 1, 3601, 0.008, 1.0);
        let prediction = evaluate_trace(&trace, config(1)).unwrap();
        let mut capture = captured_trace_from_prediction(&prediction);
        capture.latency_correction_applied_ns = -850;
        capture.latency_standard_uncertainty_ns = 75.0;

        let metrics = reconcile_captured_output(&prediction, &capture).unwrap();
        assert_eq!(metrics.latency_correction_applied_ns, -850);
        assert_eq!(metrics.latency_standard_uncertainty_ns, 75.0);
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
