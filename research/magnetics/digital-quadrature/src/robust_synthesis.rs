//! Deterministic search over Hall-latch configurations using bounded field errors.
//!
//! The search reuses the parent module's latch evaluator. It does not implement
//! a second state machine, infer uncertainty bounds, or estimate yield.

use std::collections::HashSet;

use super::{
    EncoderTrace, EvidenceKind, EvaluationError, QuadratureConfig, ScenarioSetMetrics,
    evaluate_scenario_set, evaluate_trace, project,
};

#[derive(Clone, Debug, PartialEq)]
pub struct ProjectedFieldErrorBounds {
    pub trace_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub angular_grid_id: String,
    pub field_bound_record_id: String,
    /// Simultaneous Euclidean in-plane field-vector error radius for channel A, tesla; projection error is bounded by this radius.
    pub channel_a_vector_error_radius_t: Vec<f64>,
    /// Simultaneous Euclidean in-plane field-vector error radius for channel B, tesla; projection error is bounded by this radius.
    pub channel_b_vector_error_radius_t: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedQuadratureScenario {
    pub trace: EncoderTrace,
    pub field_bounds: ProjectedFieldErrorBounds,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SynthesisRequirements {
    pub maximum_spacing_error_rad: f64,
    pub maximum_transition_bracket_rad: f64,
    /// Positive lower floor on every state-retention and transition-crossing margin.
    pub minimum_robust_threshold_margin_t: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioSynthesisMetrics {
    pub trace_id: String,
    pub operating_condition_id: String,
    pub field_bound_record_id: String,
    pub nominal_sequence_valid: bool,
    pub accepted_by_nominal_limits: bool,
    pub minimum_state_retention_margin_t: f64,
    pub minimum_transition_crossing_margin_t: Option<f64>,
    pub failed_state_sample_count: usize,
    pub failed_transition_crossing_count: usize,
    pub maximum_spacing_error_upper_bound_rad: Option<f64>,
    pub maximum_transition_bracket_rad: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateSynthesisMetrics {
    pub candidate_id: String,
    pub candidate_configuration: QuadratureConfig,
    pub nominal_all_scenarios_accepted: bool,
    pub robustly_accepted: bool,
    pub nominal_evaluation_error: Option<EvaluationError>,
    pub nominal_metrics: Option<ScenarioSetMetrics>,
    pub minimum_robust_threshold_margin_t: Option<f64>,
    pub worst_spacing_error_upper_bound_rad: Option<f64>,
    pub worst_transition_bracket_rad: Option<f64>,
    pub scenarios: Vec<ScenarioSynthesisMetrics>,
    pub is_pareto_optimal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SynthesisResult {
    pub set_id: String,
    pub evidence_kind: EvidenceKind,
    pub coordinate_frame_id: String,
    pub angular_grid_id: String,
    pub scenario_count: usize,
    pub candidate_count: usize,
    pub robustly_accepted_candidate_count: usize,
    /// Candidate order is the exact input order; no implicit randomized search occurs.
    pub candidates: Vec<CandidateSynthesisMetrics>,
    /// Indices into candidates, preserving candidate order.
    pub pareto_candidate_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SynthesisError {
    InvalidScenarioSetId,
    EmptyScenarioSet,
    EmptyCandidateSet,
    InvalidSynthesisRequirements,
    InvalidTraceMetadata,
    DuplicateTraceId,
    MixedEvidenceKinds,
    FrameIdentityMismatch,
    GridIdentityMismatch,
    GridCoordinatesMismatch,
    InvalidFieldBoundMetadata,
    FieldBoundTraceIdentityMismatch,
    FieldBoundFrameIdentityMismatch,
    FieldBoundConditionIdentityMismatch,
    FieldBoundGridIdentityMismatch,
    FieldBoundSampleCountMismatch,
    NonFiniteOrNegativeFieldBound { sample_index: usize, channel: char },
    DuplicateCandidateId,
    InvalidCandidateId,
}

fn validate_requirements(requirements: SynthesisRequirements) -> Result<(), SynthesisError> {
    if !requirements.maximum_spacing_error_rad.is_finite()
        || requirements.maximum_spacing_error_rad < 0.0
        || !requirements.maximum_transition_bracket_rad.is_finite()
        || requirements.maximum_transition_bracket_rad < 0.0
        || !requirements.minimum_robust_threshold_margin_t.is_finite()
        || requirements.minimum_robust_threshold_margin_t <= 0.0
    {
        return Err(SynthesisError::InvalidSynthesisRequirements);
    }
    Ok(())
}

fn validate_scenarios(scenarios: &[BoundedQuadratureScenario]) -> Result<(), SynthesisError> {
    let first = scenarios.first().ok_or(SynthesisError::EmptyScenarioSet)?;
    let mut trace_ids = HashSet::with_capacity(scenarios.len());

    for scenario in scenarios {
        let trace = &scenario.trace;
        let bounds = &scenario.field_bounds;

        if trace.trace_id.trim().is_empty()
            || trace.coordinate_frame_id.trim().is_empty()
            || trace.operating_condition_id.trim().is_empty()
            || trace.angular_grid_id.trim().is_empty()
        {
            return Err(SynthesisError::InvalidTraceMetadata);
        }
        if !trace_ids.insert(trace.trace_id.as_str()) {
            return Err(SynthesisError::DuplicateTraceId);
        }
        if trace.evidence_kind != first.trace.evidence_kind {
            return Err(SynthesisError::MixedEvidenceKinds);
        }
        if trace.coordinate_frame_id != first.trace.coordinate_frame_id {
            return Err(SynthesisError::FrameIdentityMismatch);
        }
        if trace.angular_grid_id != first.trace.angular_grid_id {
            return Err(SynthesisError::GridIdentityMismatch);
        }
        if trace.samples.len() != first.trace.samples.len()
            || trace.samples.iter().zip(first.trace.samples.iter()).any(|(left, right)| {
                left.mechanical_angle_rad.to_bits() != right.mechanical_angle_rad.to_bits()
            })
        {
            return Err(SynthesisError::GridCoordinatesMismatch);
        }

        if bounds.field_bound_record_id.trim().is_empty() {
            return Err(SynthesisError::InvalidFieldBoundMetadata);
        }
        if bounds.trace_id != trace.trace_id {
            return Err(SynthesisError::FieldBoundTraceIdentityMismatch);
        }
        if bounds.coordinate_frame_id != trace.coordinate_frame_id {
            return Err(SynthesisError::FieldBoundFrameIdentityMismatch);
        }
        if bounds.operating_condition_id != trace.operating_condition_id {
            return Err(SynthesisError::FieldBoundConditionIdentityMismatch);
        }
        if bounds.angular_grid_id != trace.angular_grid_id {
            return Err(SynthesisError::FieldBoundGridIdentityMismatch);
        }
        if bounds.channel_a_vector_error_radius_t.len() != trace.samples.len()
            || bounds.channel_b_vector_error_radius_t.len() != trace.samples.len()
        {
            return Err(SynthesisError::FieldBoundSampleCountMismatch);
        }
        for index in 0..trace.samples.len() {
            for (channel, value) in [
                ('A', bounds.channel_a_vector_error_radius_t[index]),
                ('B', bounds.channel_b_vector_error_radius_t[index]),
            ] {
                if !value.is_finite() || value < 0.0 {
                    return Err(SynthesisError::NonFiniteOrNegativeFieldBound {
                        sample_index: index,
                        channel,
                    });
                }
            }
        }
    }

    Ok(())
}

fn projected_values(
    trace: &EncoderTrace,
    config: &QuadratureConfig,
    sample_index: usize,
) -> (f64, f64) {
    let sample = trace.samples[sample_index];
    (
        project(sample.channel_a_field, config.channel_a.axis_angle_rad),
        project(sample.channel_b_field, config.channel_b.axis_angle_rad),
    )
}

#[derive(Clone, Copy, Debug)]
struct TransitionWindow {
    nominal_sample_after_index: usize,
    earliest_sample_index: usize,
    latest_sample_index: usize,
    lower_angle_rad: f64,
    upper_angle_rad: f64,
}

#[derive(Clone, Debug)]
struct ChannelAnalysis {
    minimum_state_margin_t: f64,
    minimum_transition_margin_t: Option<f64>,
    failed_state_samples: usize,
    failed_transition_crossings: usize,
    windows: Vec<TransitionWindow>,
}

fn analyze_channel(
    values: &[f64],
    vector_error_radii: &[f64],
    angles: &[f64],
    states: &[bool],
    bop: f64,
    brp: f64,
) -> ChannelAnalysis {
    let mut nominal_events = Vec::new();
    for index in 1..states.len() {
        if states[index - 1] != states[index] {
            nominal_events.push(index);
        }
    }

    let mut windows = Vec::with_capacity(nominal_events.len());
    let mut previous_latest = 0_usize;
    let mut failed_transition_crossings = 0_usize;
    let mut minimum_transition_margin = f64::INFINITY;

    for (event_position, nominal_index) in nominal_events.iter().copied().enumerate() {
        let search_end = nominal_events
            .get(event_position + 1)
            .copied()
            .unwrap_or(values.len());
        let rising = !states[nominal_index - 1] && states[nominal_index];

        let possible_index = (previous_latest + 1..search_end).find(|index| {
            let value = values[*index];
            let error = vector_error_radii[*index];
            if rising {
                value + error >= bop
            } else {
                value - error <= brp
            }
        });

        let Some(earliest_index) = possible_index else {
            failed_transition_crossings += 1;
            continue;
        };

        let guaranteed_index = (earliest_index..search_end).find(|index| {
            let value = values[*index];
            let error = vector_error_radii[*index];
            if rising {
                value - error >= bop
            } else {
                value + error <= brp
            }
        });

        let Some(latest_index) = guaranteed_index else {
            failed_transition_crossings += 1;
            continue;
        };

        if !(earliest_index <= nominal_index && nominal_index <= latest_index)
            || earliest_index == 0
            || latest_index <= earliest_index.saturating_sub(1)
        {
            failed_transition_crossings += 1;
            continue;
        }

        let before_index = earliest_index - 1;
        let before_margin = if rising {
            bop - (values[before_index] + vector_error_radii[before_index])
        } else {
            (values[before_index] - vector_error_radii[before_index]) - brp
        };
        let after_margin = if rising {
            (values[latest_index] - vector_error_radii[latest_index]) - bop
        } else {
            brp - (values[latest_index] + vector_error_radii[latest_index])
        };
        let transition_state_margin = (earliest_index..=latest_index)
            .map(|index| {
                if rising {
                    values[index] - vector_error_radii[index] - brp
                } else {
                    bop - (values[index] + vector_error_radii[index])
                }
            })
            .fold(f64::INFINITY, f64::min);
        let crossing_margin = before_margin.min(after_margin).min(transition_state_margin);

        if !before_margin.is_finite()
            || !after_margin.is_finite()
            || !transition_state_margin.is_finite()
            || before_margin <= 0.0
            || after_margin <= 0.0
            || transition_state_margin <= 0.0
        {
            failed_transition_crossings += 1;
        }
        minimum_transition_margin = minimum_transition_margin.min(crossing_margin);

        windows.push(TransitionWindow {
            nominal_sample_after_index: nominal_index,
            earliest_sample_index: earliest_index,
            latest_sample_index: latest_index,
            lower_angle_rad: angles[before_index],
            upper_angle_rad: angles[latest_index],
        });
        previous_latest = latest_index;
    }

    // Away from a transition's uncertainty window, the entire scalar interval
    // must remain inside the active latch state. Samples in a transition window
    // are allowed to vary because the corresponding angular interval is reported.
    let mut in_transition_window = vec![false; values.len()];
    for window in &windows {
        for index in window.earliest_sample_index..=window.latest_sample_index {
            in_transition_window[index] = true;
        }
    }

    let mut minimum_state_margin = f64::INFINITY;
    let mut failed_state_samples = 0_usize;
    for index in 0..values.len() {
        if in_transition_window[index] {
            continue;
        }
        let margin = if states[index] {
            values[index] - vector_error_radii[index] - brp
        } else {
            bop - (values[index] + vector_error_radii[index])
        };
        minimum_state_margin = minimum_state_margin.min(margin);
        if !margin.is_finite() || margin <= 0.0 {
            failed_state_samples += 1;
        }
    }

    if !minimum_state_margin.is_finite() {
        minimum_state_margin = f64::INFINITY;
    }

    ChannelAnalysis {
        minimum_state_margin_t: minimum_state_margin,
        minimum_transition_margin_t: (minimum_transition_margin.is_finite())
            .then_some(minimum_transition_margin),
        failed_state_samples,
        failed_transition_crossings,
        windows,
    }
}

fn interval_spacing_error_upper(
    lower_spacing_rad: f64,
    upper_spacing_rad: f64,
    expected_spacing_rad: f64,
) -> f64 {
    (lower_spacing_rad - expected_spacing_rad)
        .abs()
        .max((upper_spacing_rad - expected_spacing_rad).abs())
}

fn scenario_metrics(
    scenario: &BoundedQuadratureScenario,
    config: &QuadratureConfig,
    requirements: SynthesisRequirements,
) -> Result<ScenarioSynthesisMetrics, EvaluationError> {
    let evaluation = evaluate_trace(&scenario.trace, config.clone())?;
    let mut channel_a_values = Vec::with_capacity(scenario.trace.samples.len());
    let mut channel_b_values = Vec::with_capacity(scenario.trace.samples.len());
    let angles = scenario
        .trace
        .samples
        .iter()
        .map(|sample| sample.mechanical_angle_rad)
        .collect::<Vec<_>>();

    for index in 0..scenario.trace.samples.len() {
        let (a, b) = projected_values(&scenario.trace, config, index);
        channel_a_values.push(a);
        channel_b_values.push(b);
    }

    let states_a = evaluation
        .states
        .iter()
        .map(|state| state.channel_a_high)
        .collect::<Vec<_>>();
    let states_b = evaluation
        .states
        .iter()
        .map(|state| state.channel_b_high)
        .collect::<Vec<_>>();

    let analysis_a = analyze_channel(
        &channel_a_values,
        &scenario.field_bounds.channel_a_vector_error_radius_t,
        &angles,
        &states_a,
        config.channel_a.bop_threshold_t,
        config.channel_a.brp_threshold_t,
    );
    let analysis_b = analyze_channel(
        &channel_b_values,
        &scenario.field_bounds.channel_b_vector_error_radius_t,
        &angles,
        &states_b,
        config.channel_b.bop_threshold_t,
        config.channel_b.brp_threshold_t,
    );

    let mut ordered_windows = analysis_a.windows.clone();
    ordered_windows.extend(analysis_b.windows.iter().copied());
    ordered_windows.sort_by_key(|window| window.nominal_sample_after_index);

    let mut failed_transition_crossings =
        analysis_a.failed_transition_crossings + analysis_b.failed_transition_crossings;
    let mut worst_spacing_error = 0.0_f64;
    let mut maximum_bracket = 0.0_f64;

    for window in &ordered_windows {
        maximum_bracket = maximum_bracket.max(window.upper_angle_rad - window.lower_angle_rad);
    }

    let expected_spacing = std::f64::consts::TAU / evaluation.metrics.expected_transitions_per_revolution as f64;
    for adjacent in ordered_windows.windows(2) {
        let first = adjacent[0];
        let second = adjacent[1];
        if first.upper_angle_rad >= second.lower_angle_rad {
            failed_transition_crossings += 1;
        }
        let lower = second.lower_angle_rad - first.upper_angle_rad;
        let upper = second.upper_angle_rad - first.lower_angle_rad;
        worst_spacing_error =
            worst_spacing_error.max(interval_spacing_error_upper(lower, upper, expected_spacing));
    }
    if ordered_windows.len() >= 2 {
        let first = ordered_windows[0];
        let last = ordered_windows[ordered_windows.len() - 1];
        if last.upper_angle_rad >= first.lower_angle_rad + std::f64::consts::TAU {
            failed_transition_crossings += 1;
        }
        let lower = first.lower_angle_rad + std::f64::consts::TAU - last.upper_angle_rad;
        let upper = first.upper_angle_rad + std::f64::consts::TAU - last.lower_angle_rad;
        worst_spacing_error =
            worst_spacing_error.max(interval_spacing_error_upper(lower, upper, expected_spacing));
    } else {
        failed_transition_crossings += 1;
    }

    let minimum_state_margin =
        analysis_a.minimum_state_margin_t.min(analysis_b.minimum_state_margin_t);
    let minimum_transition_margin = match (
        analysis_a.minimum_transition_margin_t,
        analysis_b.minimum_transition_margin_t,
    ) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    };

    let sequence_valid = evaluation.metrics.quadrature_sequence_valid;
    let nominal_spacing_upper = evaluation.metrics.maximum_spacing_error_upper_bound_rad;
    let nominal_bracket = evaluation.metrics.maximum_transition_bracket_rad;
    let robust_spacing_upper =
        (!ordered_windows.is_empty()).then_some(worst_spacing_error);
    let robust_bracket = (!ordered_windows.is_empty()).then_some(maximum_bracket);

    let accepted_by_nominal_limits = sequence_valid
        && evaluation.metrics.within_edge_rate_policy.is_none_or(|within_limit| within_limit)
        && robust_spacing_upper
            .is_some_and(|value| value <= requirements.maximum_spacing_error_rad)
        && robust_bracket
            .is_some_and(|value| value <= requirements.maximum_transition_bracket_rad)
        && nominal_spacing_upper.is_some()
        && nominal_bracket.is_some();

    Ok(ScenarioSynthesisMetrics {
        trace_id: scenario.trace.trace_id.clone(),
        operating_condition_id: scenario.trace.operating_condition_id.clone(),
        field_bound_record_id: scenario.field_bounds.field_bound_record_id.clone(),
        nominal_sequence_valid: sequence_valid,
        accepted_by_nominal_limits,
        minimum_state_retention_margin_t: minimum_state_margin,
        minimum_transition_crossing_margin_t: minimum_transition_margin,
        failed_state_sample_count: analysis_a.failed_state_samples + analysis_b.failed_state_samples,
        failed_transition_crossing_count: failed_transition_crossings,
        maximum_spacing_error_upper_bound_rad: robust_spacing_upper,
        maximum_transition_bracket_rad: robust_bracket,
    })
}

fn dominates(left: &CandidateSynthesisMetrics, right: &CandidateSynthesisMetrics) -> bool {
    let (Some(left_margin), Some(right_margin)) = (
        left.minimum_robust_threshold_margin_t,
        right.minimum_robust_threshold_margin_t,
    ) else {
        return false;
    };
    let (Some(left_spacing), Some(right_spacing)) = (
        left.worst_spacing_error_upper_bound_rad,
        right.worst_spacing_error_upper_bound_rad,
    ) else {
        return false;
    };
    let (Some(left_bracket), Some(right_bracket)) = (
        left.worst_transition_bracket_rad,
        right.worst_transition_bracket_rad,
    ) else {
        return false;
    };

    let no_worse = left_margin >= right_margin
        && left_spacing <= right_spacing
        && left_bracket <= right_bracket;
    let strictly_better = left_margin > right_margin
        || left_spacing < right_spacing
        || left_bracket < right_bracket;
    no_worse && strictly_better
}

/// Search an explicit, ordered candidate list on one fixed bounded scenario set.
///
/// The existing evaluator remains the single nominal quadrature oracle. This
/// layer adds a set-based obligation: all nominal states and transition brackets
/// must remain invariant throughout the supplied simultaneous scalar intervals.
pub fn synthesize_robust_configurations(
    set_id: &str,
    scenarios: &[BoundedQuadratureScenario],
    candidates: &[QuadratureConfig],
    requirements: SynthesisRequirements,
) -> Result<SynthesisResult, SynthesisError> {
    if set_id.trim().is_empty() {
        return Err(SynthesisError::InvalidScenarioSetId);
    }
    if candidates.is_empty() {
        return Err(SynthesisError::EmptyCandidateSet);
    }
    validate_requirements(requirements)?;
    validate_scenarios(scenarios)?;

    let mut candidate_ids = HashSet::with_capacity(candidates.len());
    for candidate in candidates {
        if candidate.configuration_id.trim().is_empty() {
            return Err(SynthesisError::InvalidCandidateId);
        }
        if !candidate_ids.insert(candidate.configuration_id.as_str()) {
            return Err(SynthesisError::DuplicateCandidateId);
        }
    }

    let traces = scenarios
        .iter()
        .map(|scenario| scenario.trace.clone())
        .collect::<Vec<_>>();
    let mut results = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let nominal = evaluate_scenario_set(set_id, &traces, candidate.clone());
        let (nominal_metrics, nominal_error) = match nominal {
            Ok(metrics) => (Some(metrics), None),
            Err(error) => (None, Some(error)),
        };

        let Some(metrics) = nominal_metrics.as_ref() else {
            results.push(CandidateSynthesisMetrics {
                candidate_id: candidate.configuration_id.clone(),
                candidate_configuration: candidate.clone(),
                nominal_all_scenarios_accepted: false,
                robustly_accepted: false,
                nominal_evaluation_error: nominal_error,
                nominal_metrics: None,
                minimum_robust_threshold_margin_t: None,
                worst_spacing_error_upper_bound_rad: None,
                worst_transition_bracket_rad: None,
                scenarios: Vec::new(),
                is_pareto_optimal: false,
            });
            continue;
        };

        let mut scenario_results = Vec::with_capacity(scenarios.len());
        let mut minimum_margin = f64::INFINITY;
        let mut worst_spacing = 0.0_f64;
        let mut worst_bracket = 0.0_f64;
        let mut all_nominal_limits_pass = metrics.all_scenarios_accepted;

        for scenario in scenarios {
            match scenario_metrics(scenario, candidate, requirements) {
                Ok(result) => {
                    all_nominal_limits_pass &= result.accepted_by_nominal_limits;
                    minimum_margin = minimum_margin.min(
                        result.minimum_state_retention_margin_t.min(
                            result
                                .minimum_transition_crossing_margin_t
                                .unwrap_or(f64::INFINITY),
                        ),
                    );
                    if let Some(value) = result.maximum_spacing_error_upper_bound_rad {
                        worst_spacing = worst_spacing.max(value);
                    } else {
                        all_nominal_limits_pass = false;
                    }
                    if let Some(value) = result.maximum_transition_bracket_rad {
                        worst_bracket = worst_bracket.max(value);
                    } else {
                        all_nominal_limits_pass = false;
                    }
                    scenario_results.push(result);
                }
                Err(_) => {
                    all_nominal_limits_pass = false;
                    scenario_results.push(ScenarioSynthesisMetrics {
                        trace_id: scenario.trace.trace_id.clone(),
                        operating_condition_id: scenario.trace.operating_condition_id.clone(),
                        field_bound_record_id: scenario.field_bounds.field_bound_record_id.clone(),
                        nominal_sequence_valid: false,
                        accepted_by_nominal_limits: false,
                        minimum_state_retention_margin_t: f64::NEG_INFINITY,
                        minimum_transition_crossing_margin_t: None,
                        failed_state_sample_count: scenario.trace.samples.len() * 2,
                        failed_transition_crossing_count: 0,
                        maximum_spacing_error_upper_bound_rad: None,
                        maximum_transition_bracket_rad: None,
                    });
                }
            }
        }

        let failed_states = scenario_results
            .iter()
            .map(|scenario| scenario.failed_state_sample_count)
            .sum::<usize>();
        let failed_crossings = scenario_results
            .iter()
            .map(|scenario| scenario.failed_transition_crossing_count)
            .sum::<usize>();
        let robust_margin_pass = minimum_margin.is_finite()
            && minimum_margin >= requirements.minimum_robust_threshold_margin_t;
        let robustly_accepted = all_nominal_limits_pass
            && robust_margin_pass
            && failed_states == 0
            && failed_crossings == 0;

        results.push(CandidateSynthesisMetrics {
            candidate_id: candidate.configuration_id.clone(),
            candidate_configuration: candidate.clone(),
            nominal_all_scenarios_accepted: all_nominal_limits_pass,
            robustly_accepted,
            nominal_evaluation_error: None,
            nominal_metrics: Some(metrics.clone()),
            minimum_robust_threshold_margin_t: minimum_margin.is_finite().then_some(minimum_margin),
            worst_spacing_error_upper_bound_rad: Some(worst_spacing),
            worst_transition_bracket_rad: Some(worst_bracket),
            scenarios: scenario_results,
            is_pareto_optimal: false,
        });
    }

    for index in 0..results.len() {
        if !results[index].robustly_accepted {
            continue;
        }
        let dominated = results.iter().enumerate().any(|(other_index, other)| {
            other_index != index && other.robustly_accepted && dominates(other, &results[index])
        });
        results[index].is_pareto_optimal = !dominated;
    }

    let pareto_candidate_indices = results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| result.is_pareto_optimal.then_some(index))
        .collect::<Vec<_>>();
    let robustly_accepted_candidate_count =
        results.iter().filter(|result| result.robustly_accepted).count();

    Ok(SynthesisResult {
        set_id: set_id.to_owned(),
        evidence_kind: scenarios[0].trace.evidence_kind,
        coordinate_frame_id: scenarios[0].trace.coordinate_frame_id.clone(),
        angular_grid_id: scenarios[0].trace.angular_grid_id.clone(),
        scenario_count: scenarios.len(),
        candidate_count: candidates.len(),
        robustly_accepted_candidate_count,
        candidates: results,
        pareto_candidate_indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EncoderSample, EvidenceKind, HallLatchConfig, InPlaneField, SensorArchitecture,
    };
    use std::f64::consts::TAU;

    fn sinusoidal_trace(trace_id: &str, condition_id: &str, pole_pairs: u32) -> EncoderTrace {
        let samples = (0..=256)
            .map(|index| {
                let angle = TAU * index as f64 / 256.0;
                let phase = pole_pairs as f64 * angle;
                let field = InPlaneField {
                    radial_t: phase.cos(),
                    tangential_t: phase.sin(),
                };
                EncoderSample {
                    mechanical_angle_rad: angle,
                    timestamp_ns: None,
                    channel_a_field: field,
                    channel_b_field: field,
                }
            })
            .collect();

        EncoderTrace {
            trace_id: trace_id.to_owned(),
            coordinate_frame_id: "frame-001".to_owned(),
            operating_condition_id: condition_id.to_owned(),
            angular_grid_id: "grid-257".to_owned(),
            evidence_kind: EvidenceKind::SyntheticScenario,
            samples,
        }
    }

    fn bounds(trace: &EncoderTrace, radius_t: f64) -> ProjectedFieldErrorBounds {
        ProjectedFieldErrorBounds {
            trace_id: trace.trace_id.clone(),
            coordinate_frame_id: trace.coordinate_frame_id.clone(),
            operating_condition_id: trace.operating_condition_id.clone(),
            angular_grid_id: trace.angular_grid_id.clone(),
            field_bound_record_id: format!("bounds-{}", trace.trace_id),
            channel_a_vector_error_radius_t: vec![radius_t; trace.samples.len()],
            channel_b_vector_error_radius_t: vec![radius_t; trace.samples.len()],
        }
    }

    fn scenario(trace_id: &str, condition_id: &str, error_t: f64) -> BoundedQuadratureScenario {
        let trace = sinusoidal_trace(trace_id, condition_id, 1);
        let field_bounds = bounds(&trace, error_t);
        BoundedQuadratureScenario { trace, field_bounds }
    }

    fn config(configuration_id: &str, bop: f64, brp: f64) -> QuadratureConfig {
        QuadratureConfig {
            configuration_id: configuration_id.to_owned(),
            pole_pairs: 1,
            architecture: SensorArchitecture::IntegratedTwoAxisLatch,
            channel_a: HallLatchConfig {
                axis_angle_rad: 0.0,
                bop_threshold_t: bop,
                brp_threshold_t: brp,
                initial_high: true,
            },
            channel_b: HallLatchConfig {
                axis_angle_rad: std::f64::consts::FRAC_PI_2,
                bop_threshold_t: bop,
                brp_threshold_t: brp,
                initial_high: false,
            },
            edge_rate_policy: None,
        }
    }

    fn requirements() -> SynthesisRequirements {
        SynthesisRequirements {
            maximum_spacing_error_rad: 0.12,
            maximum_transition_bracket_rad: 0.06,
            minimum_robust_threshold_margin_t: 0.005,
        }
    }

    #[test]
    fn ideal_configuration_is_robustly_accepted_with_positive_bounds() {
        let scenarios = vec![
            scenario("trace-nominal", "nominal", 0.01),
            scenario("trace-gap", "increased-gap", 0.02),
        ];
        let candidates = vec![config("candidate-good", 0.25, -0.25)];
        let result = synthesize_robust_configurations(
            "set-001",
            &scenarios,
            &candidates,
            requirements(),
        )
        .expect("valid scenario set");

        assert_eq!(result.robustly_accepted_candidate_count, 1);
        assert_eq!(result.pareto_candidate_indices, vec![0]);
        assert!(result.candidates[0].robustly_accepted);
        assert!(result.candidates[0]
            .minimum_robust_threshold_margin_t
            .is_some_and(|margin| margin >= 0.005));
    }

    #[test]
    fn a_larger_field_error_bound_can_turn_the_same_candidate_into_failure() {
        let scenarios = vec![scenario("trace-weak-margin", "nominal", 0.30)];
        let candidates = vec![config("candidate-margin-fail", 0.25, -0.25)];
        let result = synthesize_robust_configurations(
            "set-002",
            &scenarios,
            &candidates,
            requirements(),
        )
        .expect("finite bounded set");

        assert!(!result.candidates[0].robustly_accepted);
        assert_eq!(result.robustly_accepted_candidate_count, 0);
    }

    #[test]
    fn transition_crossings_must_be_separated_from_both_threshold_sides() {
        let scenarios = vec![scenario("trace-crossing", "nominal", 0.80)];
        let candidates = vec![config("candidate-crossing-fail", 0.25, -0.25)];
        let result = synthesize_robust_configurations(
            "set-003",
            &scenarios,
            &candidates,
            requirements(),
        )
        .expect("finite bounded set");

        assert!(result.candidates[0].scenarios[0].failed_transition_crossing_count > 0);
        assert!(!result.candidates[0].robustly_accepted);
    }

    #[test]
    fn field_bound_must_match_trace_frame_condition_and_grid() {
        let mut input = scenario("trace-identity", "nominal", 0.01);
        input.field_bounds.coordinate_frame_id = "other-frame".to_owned();

        assert_eq!(
            synthesize_robust_configurations(
                "set-004",
                &[input],
                &[config("candidate", 0.25, -0.25)],
                requirements(),
            )
            .expect_err("frame mismatch must fail closed"),
            SynthesisError::FieldBoundFrameIdentityMismatch
        );
    }

    #[test]
    fn nonfinite_and_negative_field_bounds_are_rejected() {
        let mut input = scenario("trace-bad-bound", "nominal", 0.01);
        input.field_bounds.channel_b_vector_error_radius_t[7] = f64::NAN;

        assert_eq!(
            synthesize_robust_configurations(
                "set-005",
                &[input],
                &[config("candidate", 0.25, -0.25)],
                requirements(),
            )
            .expect_err("nonfinite bound must fail closed"),
            SynthesisError::NonFiniteOrNegativeFieldBound {
                sample_index: 7,
                channel: 'B',
            }
        );
    }

    #[test]
    fn empty_and_duplicate_candidates_are_rejected() {
        let input = scenario("trace-candidates", "nominal", 0.01);
        assert_eq!(
            synthesize_robust_configurations("set-006", &[input.clone()], &[], requirements())
                .expect_err("empty candidate list is invalid"),
            SynthesisError::EmptyCandidateSet
        );

        let duplicates = vec![
            config("duplicate-id", 0.25, -0.25),
            config("duplicate-id", 0.30, -0.30),
        ];
        assert_eq!(
            synthesize_robust_configurations(
                "set-006",
                &[input],
                &duplicates,
                requirements(),
            )
            .expect_err("duplicate IDs are invalid"),
            SynthesisError::DuplicateCandidateId
        );
    }

    #[test]
    fn pareto_frontier_preserves_robust_tradeoffs() {
        let a = CandidateSynthesisMetrics {
            candidate_id: "a".to_owned(),
            candidate_configuration: config("a", 0.25, -0.25),
            nominal_all_scenarios_accepted: true,
            robustly_accepted: true,
            nominal_evaluation_error: None,
            nominal_metrics: None,
            minimum_robust_threshold_margin_t: Some(0.20),
            worst_spacing_error_upper_bound_rad: Some(0.04),
            worst_transition_bracket_rad: Some(0.01),
            scenarios: Vec::new(),
            is_pareto_optimal: false,
        };
        let b = CandidateSynthesisMetrics {
            candidate_id: "b".to_owned(),
            candidate_configuration: config("b", 0.25, -0.25),
            nominal_all_scenarios_accepted: true,
            robustly_accepted: true,
            nominal_evaluation_error: None,
            nominal_metrics: None,
            minimum_robust_threshold_margin_t: Some(0.18),
            worst_spacing_error_upper_bound_rad: Some(0.03),
            worst_transition_bracket_rad: Some(0.01),
            scenarios: Vec::new(),
            is_pareto_optimal: false,
        };
        let c = CandidateSynthesisMetrics {
            candidate_id: "c".to_owned(),
            candidate_configuration: config("c", 0.25, -0.25),
            nominal_all_scenarios_accepted: true,
            robustly_accepted: true,
            nominal_evaluation_error: None,
            nominal_metrics: None,
            minimum_robust_threshold_margin_t: Some(0.10),
            worst_spacing_error_upper_bound_rad: Some(0.05),
            worst_transition_bracket_rad: Some(0.02),
            scenarios: Vec::new(),
            is_pareto_optimal: false,
        };

        assert!(!dominates(&a, &b));
        assert!(dominates(&a, &c));
    }
}
