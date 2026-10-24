//! Fail-closed reference evaluator for a lighting-service benchmark.
//!
//! This crate evaluates supplied observations; it does not calculate photometry.
//! Grid summaries are derived from raw samples so candidate-reported aggregates
//! cannot contradict the grid submitted for evaluation.

pub mod experiment_selection;

use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateStatus {
    Pass,
    Fail,
    Unknown,
    Unsupported,
    NotEvaluated,
    Timeout,
    InvalidOutput,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateResult {
    pub name: String,
    pub status: GateStatus,
}

#[derive(Clone, Debug)]
pub struct CaseLimits {
    pub average_illuminance_min_lx: f64,
    pub minimum_illuminance_min_lx: f64,
    pub minimum_to_average_uniformity_min: f64,
    pub maximum_peak_power_w: f64,
    pub maximum_average_power_w: f64,
    /// Frozen by the benchmark case, not selected by an optimizer.
    pub expected_illuminance_points: usize,
    pub required_gate_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CandidateObservation {
    pub candidate_id: String,
    pub energy_wh: f64,
    pub observed_hours: f64,
    pub compliant_task_hours: f64,
    pub peak_wall_power_w: f64,
    /// Raw samples at the case-defined grid positions, in lux.
    pub illuminance_samples_lx: Vec<f64>,
    pub gates: Vec<GateResult>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridSummary {
    pub average_lx: f64,
    pub minimum_lx: f64,
    pub maximum_lx: f64,
    pub minimum_to_average_uniformity: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GridError {
    ZeroExpectedPoints,
    EmptyGrid,
    PointCountMismatch,
    InvalidSample,
    InvalidInterval,
    NonFiniteAggregate,
}

impl GridError {
    fn as_str(self) -> &'static str {
        match self {
            Self::ZeroExpectedPoints => "grid_expected_points_zero",
            Self::EmptyGrid => "grid_empty",
            Self::PointCountMismatch => "grid_point_count_mismatch",
            Self::InvalidSample => "grid_sample_negative_or_non_finite",
            Self::InvalidInterval => "grid_interval_invalid_or_inverted",
            Self::NonFiniteAggregate => "grid_aggregate_non_finite",
        }
    }
}

/// Derive all summary metrics from the raw illuminance grid.
///
/// The expected point count must come from the frozen test case. An empty,
/// incomplete, negative or non-finite grid is invalid and cannot be ranked.
pub fn summarize_illuminance_grid(
    samples_lx: &[f64],
    expected_points: usize,
) -> Result<GridSummary, GridError> {
    if expected_points == 0 {
        return Err(GridError::ZeroExpectedPoints);
    }
    if samples_lx.is_empty() {
        return Err(GridError::EmptyGrid);
    }
    if samples_lx.len() != expected_points {
        return Err(GridError::PointCountMismatch);
    }
    if samples_lx
        .iter()
        .any(|sample| !sample.is_finite() || *sample < 0.0)
    {
        return Err(GridError::InvalidSample);
    }

    let mut sum = 0.0_f64;
    let mut minimum = f64::INFINITY;
    let mut maximum = f64::NEG_INFINITY;
    for sample in samples_lx {
        sum += *sample;
        minimum = minimum.min(*sample);
        maximum = maximum.max(*sample);
    }
    let average = sum / samples_lx.len() as f64;
    if !sum.is_finite()
        || !average.is_finite()
        || !minimum.is_finite()
        || !maximum.is_finite()
    {
        return Err(GridError::NonFiniteAggregate);
    }
    let uniformity = if average > 0.0 {
        minimum / average
    } else {
        0.0
    };
    if !uniformity.is_finite() {
        return Err(GridError::NonFiniteAggregate);
    }

    Ok(GridSummary {
        average_lx: average,
        minimum_lx: minimum,
        maximum_lx: maximum,
        minimum_to_average_uniformity: uniformity,
    })
}

/// A defensible pointwise interval for illuminance, in lux. The interval's
/// meaning (hard bound, expanded measurement uncertainty, or calibrated
/// prediction interval) must be recorded by the run manifest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IlluminanceInterval {
    pub lower_lx: f64,
    pub upper_lx: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RobustGridSummary {
    pub lower_average_lx: f64,
    pub upper_average_lx: f64,
    pub minimum_lower_lx: f64,
    pub maximum_upper_lx: f64,
    /// Conservative lower bound: min(pointwise lower) / mean(pointwise upper).
    pub conservative_minimum_to_average_uniformity: f64,
}

/// Derive conservative summaries from pointwise intervals. This bounds the
/// whole sampled grid only under the declared interval semantics; it does not
/// establish simultaneous coverage or interpolate between grid points.
pub fn summarize_illuminance_bounds(
    intervals: &[IlluminanceInterval],
    expected_points: usize,
) -> Result<RobustGridSummary, GridError> {
    if expected_points == 0 {
        return Err(GridError::ZeroExpectedPoints);
    }
    if intervals.is_empty() {
        return Err(GridError::EmptyGrid);
    }
    if intervals.len() != expected_points {
        return Err(GridError::PointCountMismatch);
    }
    if intervals.iter().any(|interval| {
        !interval.lower_lx.is_finite()
            || !interval.upper_lx.is_finite()
            || interval.lower_lx < 0.0
            || interval.upper_lx < 0.0
    }) {
        return Err(GridError::InvalidSample);
    }
    if intervals
        .iter()
        .any(|interval| interval.upper_lx < interval.lower_lx)
    {
        return Err(GridError::InvalidInterval);
    }

    let mut lower_sum = 0.0_f64;
    let mut upper_sum = 0.0_f64;
    let mut minimum_lower = f64::INFINITY;
    let mut maximum_upper = f64::NEG_INFINITY;
    for interval in intervals {
        lower_sum += interval.lower_lx;
        upper_sum += interval.upper_lx;
        minimum_lower = minimum_lower.min(interval.lower_lx);
        maximum_upper = maximum_upper.max(interval.upper_lx);
    }
    let lower_average = lower_sum / intervals.len() as f64;
    let upper_average = upper_sum / intervals.len() as f64;
    if !lower_sum.is_finite()
        || !upper_sum.is_finite()
        || !lower_average.is_finite()
        || !upper_average.is_finite()
        || !minimum_lower.is_finite()
        || !maximum_upper.is_finite()
    {
        return Err(GridError::NonFiniteAggregate);
    }
    let conservative_uniformity = if upper_average > 0.0 {
        minimum_lower / upper_average
    } else {
        0.0
    };
    if !conservative_uniformity.is_finite() {
        return Err(GridError::NonFiniteAggregate);
    }

    Ok(RobustGridSummary {
        lower_average_lx: lower_average,
        upper_average_lx: upper_average,
        minimum_lower_lx: minimum_lower,
        maximum_upper_lx: maximum_upper,
        conservative_minimum_to_average_uniformity: conservative_uniformity,
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationStatus {
    Compliant,
    NonCompliant,
    Unknown,
    Unsupported,
    NotEvaluated,
    Timeout,
    InvalidOutput,
    InvalidInput,
}

impl EvaluationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compliant => "compliant",
            Self::NonCompliant => "non_compliant",
            Self::Unknown => "unknown",
            Self::Unsupported => "unsupported",
            Self::NotEvaluated => "not_evaluated",
            Self::Timeout => "timeout",
            Self::InvalidOutput => "invalid_output",
            Self::InvalidInput => "invalid_input",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Evaluation {
    pub candidate_id: String,
    pub status: EvaluationStatus,
    pub average_wall_power_w: Option<f64>,
    pub illuminance: Option<GridSummary>,
    pub energy_wh_per_compliant_task_hour: Option<f64>,
    pub reason_codes: Vec<String>,
}

impl Evaluation {
    fn invalid(candidate_id: &str, reason: &str) -> Self {
        Self {
            candidate_id: candidate_id.to_owned(),
            status: EvaluationStatus::InvalidInput,
            average_wall_power_w: None,
            illuminance: None,
            energy_wh_per_compliant_task_hour: None,
            reason_codes: vec![reason.to_owned()],
        }
    }
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn valid_limits(limits: &CaseLimits) -> bool {
    limits.average_illuminance_min_lx.is_finite()
        && limits.average_illuminance_min_lx > 0.0
        && finite_nonnegative(limits.minimum_illuminance_min_lx)
        && limits.minimum_to_average_uniformity_min.is_finite()
        && (0.0..=1.0).contains(&limits.minimum_to_average_uniformity_min)
        && limits.maximum_peak_power_w.is_finite()
        && limits.maximum_peak_power_w > 0.0
        && limits.maximum_average_power_w.is_finite()
        && limits.maximum_average_power_w > 0.0
        && limits.expected_illuminance_points > 0
        && !limits.required_gate_names.is_empty()
        && limits.required_gate_names.iter().all(|name| !name.trim().is_empty())
        && {
            let unique: std::collections::BTreeSet<_> =
                limits.required_gate_names.iter().collect();
            unique.len() == limits.required_gate_names.len()
        }
}

fn status_from_required_gates(
    required_names: &[String],
    gates: &[GateResult],
    reasons: &mut Vec<String>,
) -> EvaluationStatus {
    let mut by_name = BTreeMap::new();
    for gate in gates {
        if gate.name.trim().is_empty() {
            reasons.push("empty_gate_name".to_owned());
            return EvaluationStatus::InvalidInput;
        }
        if by_name.insert(gate.name.as_str(), gate.status).is_some() {
            reasons.push("duplicate_gate_name".to_owned());
            return EvaluationStatus::InvalidInput;
        }
    }

    let mut required_statuses = Vec::with_capacity(required_names.len());
    for name in required_names {
        match by_name.get(name.as_str()) {
            Some(status) => required_statuses.push(*status),
            None => {
                reasons.push(format!("required_gate_missing:{name}"));
                required_statuses.push(GateStatus::Unknown);
            }
        }
    }

    for (name, status) in required_names.iter().zip(required_statuses.iter()) {
        match status {
            GateStatus::Fail => reasons.push(format!("required_gate_failed:{name}")),
            GateStatus::Unknown => reasons.push(format!("required_gate_unknown:{name}")),
            GateStatus::Unsupported => reasons.push(format!("required_gate_unsupported:{name}")),
            GateStatus::NotEvaluated => {
                reasons.push(format!("required_gate_not_evaluated:{name}"))
            }
            GateStatus::Timeout => reasons.push(format!("required_gate_timeout:{name}")),
            GateStatus::InvalidOutput => {
                reasons.push(format!("required_gate_invalid_output:{name}"))
            }
            GateStatus::Pass => {}
        }
    }

    if required_statuses.contains(&GateStatus::Fail) {
        EvaluationStatus::NonCompliant
    } else if required_statuses.contains(&GateStatus::InvalidOutput) {
        EvaluationStatus::InvalidOutput
    } else if required_statuses.contains(&GateStatus::Timeout) {
        EvaluationStatus::Timeout
    } else if required_statuses.contains(&GateStatus::Unsupported) {
        EvaluationStatus::Unsupported
    } else if required_statuses.contains(&GateStatus::NotEvaluated) {
        EvaluationStatus::NotEvaluated
    } else if required_statuses.contains(&GateStatus::Unknown) {
        EvaluationStatus::Unknown
    } else {
        EvaluationStatus::Compliant
    }
}

/// Evaluate an observation against frozen case limits.
///
/// The energy/service metric is withheld unless all numeric constraints and
/// all case-required evidence gates pass. Illuminance metrics are calculated
/// from raw samples, not trusted candidate-supplied summary fields.
pub fn evaluate(candidate: &CandidateObservation, limits: &CaseLimits) -> Evaluation {
    if candidate.candidate_id.trim().is_empty() {
        return Evaluation::invalid("", "empty_candidate_id");
    }
    if !valid_limits(limits) {
        return Evaluation::invalid(&candidate.candidate_id, "invalid_case_limits");
    }
    if !candidate.energy_wh.is_finite()
        || candidate.energy_wh < 0.0
        || !candidate.observed_hours.is_finite()
        || candidate.observed_hours <= 0.0
        || !finite_nonnegative(candidate.compliant_task_hours)
        || candidate.compliant_task_hours > candidate.observed_hours + 1.0e-9
        || !finite_nonnegative(candidate.peak_wall_power_w)
        || candidate.energy_wh
            > candidate.peak_wall_power_w * candidate.observed_hours + 1.0e-6
    {
        return Evaluation::invalid(&candidate.candidate_id, "invalid_numeric_observation");
    }

    let illuminance = match summarize_illuminance_grid(
        &candidate.illuminance_samples_lx,
        limits.expected_illuminance_points,
    ) {
        Ok(summary) => summary,
        Err(error) => return Evaluation::invalid(&candidate.candidate_id, error.as_str()),
    };
    let average_wall_power_w = candidate.energy_wh / candidate.observed_hours;
    let mut reasons = Vec::new();
    let mut numeric_failure = false;

    if illuminance.average_lx < limits.average_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("average_illuminance_below_target".to_owned());
    }
    if illuminance.minimum_lx < limits.minimum_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("minimum_illuminance_below_target".to_owned());
    }
    if illuminance.minimum_to_average_uniformity < limits.minimum_to_average_uniformity_min {
        numeric_failure = true;
        reasons.push("uniformity_below_target".to_owned());
    }
    if candidate.peak_wall_power_w > limits.maximum_peak_power_w {
        numeric_failure = true;
        reasons.push("peak_power_exceeds_limit".to_owned());
    }
    if average_wall_power_w > limits.maximum_average_power_w {
        numeric_failure = true;
        reasons.push("average_power_exceeds_limit".to_owned());
    }

    let gate_status = status_from_required_gates(
        &limits.required_gate_names,
        &candidate.gates,
        &mut reasons,
    );
    if gate_status == EvaluationStatus::InvalidInput {
        return Evaluation {
            candidate_id: candidate.candidate_id.clone(),
            status: gate_status,
            average_wall_power_w: Some(average_wall_power_w),
            illuminance: Some(illuminance),
            energy_wh_per_compliant_task_hour: None,
            reason_codes: reasons,
        };
    }

    let status = if numeric_failure || gate_status == EvaluationStatus::NonCompliant {
        EvaluationStatus::NonCompliant
    } else {
        gate_status
    };
    let status = if status == EvaluationStatus::Compliant && candidate.compliant_task_hours <= 0.0 {
        reasons.push("zero_compliant_task_hours".to_owned());
        EvaluationStatus::NotEvaluated
    } else {
        status
    };
    let energy_metric = if status == EvaluationStatus::Compliant {
        Some(candidate.energy_wh / candidate.compliant_task_hours)
    } else {
        None
    };

    Evaluation {
        candidate_id: candidate.candidate_id.clone(),
        status,
        average_wall_power_w: Some(average_wall_power_w),
        illuminance: Some(illuminance),
        energy_wh_per_compliant_task_hour: energy_metric,
        reason_codes: reasons,
    }
}

/// Candidate values for a conservative interval-based evaluation.
///
/// Energy and peak power are upper bounds. Required quality gates still need
/// their own evidence; this evaluator never infers color, glare, flicker,
/// thermal, or product-safety compliance from illuminance intervals.
#[derive(Clone, Debug)]
pub struct RobustCandidateObservation {
    pub candidate_id: String,
    pub energy_upper_bound_wh: f64,
    pub observed_hours: f64,
    pub compliant_task_hours: f64,
    pub peak_wall_power_upper_bound_w: f64,
    pub illuminance_bounds_lx: Vec<IlluminanceInterval>,
    pub gates: Vec<GateResult>,
}

#[derive(Clone, Debug)]
pub struct RobustEvaluation {
    pub candidate_id: String,
    pub status: EvaluationStatus,
    pub average_wall_power_upper_bound_w: Option<f64>,
    pub illuminance: Option<RobustGridSummary>,
    pub energy_upper_bound_wh_per_compliant_task_hour: Option<f64>,
    pub reason_codes: Vec<String>,
}

/// Evaluate conservative bounds rather than a nominal point estimate.
///
/// A candidate is robustly feasible for this case only if lower-bound
/// illuminance and uniformity satisfy the frozen requirements, upper-bound
/// power stays within limits, and every required independent gate passes.
pub fn evaluate_robust(
    candidate: &RobustCandidateObservation,
    limits: &CaseLimits,
) -> RobustEvaluation {
    let invalid = |reason: &str| RobustEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        status: EvaluationStatus::InvalidInput,
        average_wall_power_upper_bound_w: None,
        illuminance: None,
        energy_upper_bound_wh_per_compliant_task_hour: None,
        reason_codes: vec![reason.to_owned()],
    };

    if candidate.candidate_id.trim().is_empty() {
        return invalid("empty_candidate_id");
    }
    if !valid_limits(limits) {
        return invalid("invalid_case_limits");
    }
    if !finite_nonnegative(candidate.energy_upper_bound_wh)
        || !candidate.observed_hours.is_finite()
        || candidate.observed_hours <= 0.0
        || !finite_nonnegative(candidate.compliant_task_hours)
        || candidate.compliant_task_hours > candidate.observed_hours + 1.0e-9
        || !finite_nonnegative(candidate.peak_wall_power_upper_bound_w)
        || candidate.energy_upper_bound_wh
            > candidate.peak_wall_power_upper_bound_w * candidate.observed_hours + 1.0e-6
    {
        return invalid("invalid_numeric_observation");
    }

    let illuminance = match summarize_illuminance_bounds(
        &candidate.illuminance_bounds_lx,
        limits.expected_illuminance_points,
    ) {
        Ok(summary) => summary,
        Err(error) => return invalid(error.as_str()),
    };
    let average_power_upper = candidate.energy_upper_bound_wh / candidate.observed_hours;
    let mut reasons = Vec::new();
    let mut numeric_failure = false;

    if illuminance.lower_average_lx < limits.average_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("lower_bound_average_illuminance_below_target".to_owned());
    }
    if illuminance.minimum_lower_lx < limits.minimum_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("lower_bound_minimum_illuminance_below_target".to_owned());
    }
    if illuminance.conservative_minimum_to_average_uniformity
        < limits.minimum_to_average_uniformity_min
    {
        numeric_failure = true;
        reasons.push("conservative_uniformity_below_target".to_owned());
    }
    if candidate.peak_wall_power_upper_bound_w > limits.maximum_peak_power_w {
        numeric_failure = true;
        reasons.push("upper_bound_peak_power_exceeds_limit".to_owned());
    }
    if average_power_upper > limits.maximum_average_power_w {
        numeric_failure = true;
        reasons.push("upper_bound_average_power_exceeds_limit".to_owned());
    }

    let gate_status = status_from_required_gates(
        &limits.required_gate_names,
        &candidate.gates,
        &mut reasons,
    );
    if gate_status == EvaluationStatus::InvalidInput {
        return RobustEvaluation {
            candidate_id: candidate.candidate_id.clone(),
            status: gate_status,
            average_wall_power_upper_bound_w: Some(average_power_upper),
            illuminance: Some(illuminance),
            energy_upper_bound_wh_per_compliant_task_hour: None,
            reason_codes: reasons,
        };
    }

    let status = if numeric_failure || gate_status == EvaluationStatus::NonCompliant {
        EvaluationStatus::NonCompliant
    } else {
        gate_status
    };
    let status = if status == EvaluationStatus::Compliant && candidate.compliant_task_hours <= 0.0 {
        reasons.push("zero_compliant_task_hours".to_owned());
        EvaluationStatus::NotEvaluated
    } else {
        status
    };
    let energy_metric = if status == EvaluationStatus::Compliant {
        Some(candidate.energy_upper_bound_wh / candidate.compliant_task_hours)
    } else {
        None
    };

    RobustEvaluation {
        candidate_id: candidate.candidate_id.clone(),
        status,
        average_wall_power_upper_bound_w: Some(average_power_upper),
        illuminance: Some(illuminance),
        energy_upper_bound_wh_per_compliant_task_hour: energy_metric,
        reason_codes: reasons,
    }
}

/// True only when vector a Pareto-dominates vector b for equal-length, finite
/// minimization vectors. Missing objectives must be handled before this call.
pub fn dominates_minimization(a: &[f64], b: &[f64]) -> bool {
    if a.is_empty()
        || a.len() != b.len()
        || a.iter().chain(b.iter()).any(|value| !value.is_finite())
    {
        return false;
    }
    let no_worse = a.iter().zip(b.iter()).all(|(left, right)| left <= right);
    let strictly_better = a.iter().zip(b.iter()).any(|(left, right)| left < right);
    no_worse && strictly_better
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> CaseLimits {
        CaseLimits {
            average_illuminance_min_lx: 500.0,
            minimum_illuminance_min_lx: 300.0,
            minimum_to_average_uniformity_min: 0.6,
            maximum_peak_power_w: 15.0,
            maximum_average_power_w: 15.0,
            expected_illuminance_points: 104,
            required_gate_names: vec![
                "photometry".to_owned(),
                "color_quality".to_owned(),
                "flicker".to_owned(),
                "thermal".to_owned(),
                "electrical_safety".to_owned(),
            ],
        }
    }

    fn passing_candidate() -> CandidateObservation {
        let mut samples = vec![520.0; 104];
        samples[0] = 315.0;
        CandidateObservation {
            candidate_id: "synthetic-candidate-a".to_owned(),
            energy_wh: 8.0,
            observed_hours: 1.0,
            compliant_task_hours: 1.0,
            peak_wall_power_w: 10.0,
            illuminance_samples_lx: samples,
            gates: limits()
                .required_gate_names
                .iter()
                .map(|name| GateResult {
                    name: name.clone(),
                    status: GateStatus::Pass,
                })
                .collect(),
        }
    }

    fn passing_robust_candidate() -> RobustCandidateObservation {
        RobustCandidateObservation {
            candidate_id: "synthetic-robust-candidate".to_owned(),
            energy_upper_bound_wh: 8.0,
            observed_hours: 1.0,
            compliant_task_hours: 1.0,
            peak_wall_power_upper_bound_w: 10.0,
            illuminance_bounds_lx: vec![
                IlluminanceInterval { lower_lx: 510.0, upper_lx: 550.0 };
                104
            ],
            gates: limits()
                .required_gate_names
                .iter()
                .map(|name| GateResult {
                    name: name.clone(),
                    status: GateStatus::Pass,
                })
                .collect(),
        }
    }

    #[test]
    fn interval_summary_uses_conservative_bounds() {
        let summary = summarize_illuminance_bounds(
            &[
                IlluminanceInterval { lower_lx: 8.0, upper_lx: 12.0 },
                IlluminanceInterval { lower_lx: 18.0, upper_lx: 22.0 },
            ],
            2,
        )
        .unwrap();
        assert_eq!(summary.lower_average_lx, 13.0);
        assert_eq!(summary.upper_average_lx, 17.0);
        assert_eq!(summary.minimum_lower_lx, 8.0);
        assert_eq!(summary.maximum_upper_lx, 22.0);
        assert!((summary.conservative_minimum_to_average_uniformity - (8.0 / 17.0)).abs() < 1e-12);
    }

    #[test]
    fn inverted_or_non_finite_interval_is_rejected() {
        assert_eq!(
            summarize_illuminance_bounds(
                &[IlluminanceInterval { lower_lx: 20.0, upper_lx: 10.0 }],
                1
            ),
            Err(GridError::InvalidInterval)
        );
        assert_eq!(
            summarize_illuminance_bounds(
                &[IlluminanceInterval { lower_lx: 0.0, upper_lx: f64::INFINITY }],
                1
            ),
            Err(GridError::InvalidSample)
        );
    }

    #[test]
    fn robust_evaluation_passes_only_on_conservative_bounds() {
        let result = evaluate_robust(&passing_robust_candidate(), &limits());
        assert_eq!(result.status, EvaluationStatus::Compliant);
        assert_eq!(
            result.energy_upper_bound_wh_per_compliant_task_hour,
            Some(8.0)
        );
    }

    #[test]
    fn robust_evaluation_rejects_design_that_only_passes_at_midpoint() {
        let mut candidate = passing_robust_candidate();
        candidate.illuminance_bounds_lx[0] =
            IlluminanceInterval { lower_lx: 280.0, upper_lx: 800.0 };
        let result = evaluate_robust(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::NonCompliant);
        assert_eq!(result.energy_upper_bound_wh_per_compliant_task_hour, None);
        assert!(result.reason_codes.contains(
            &"lower_bound_minimum_illuminance_below_target".to_owned()
        ));
    }

    #[test]
    fn robust_evaluation_keeps_missing_gate_unknown() {
        let mut candidate = passing_robust_candidate();
        candidate.gates.retain(|gate| gate.name != "flicker");
        let result = evaluate_robust(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::Unknown);
        assert_eq!(result.energy_upper_bound_wh_per_compliant_task_hour, None);
    }

    #[test]
    fn grid_summary_is_calculated_from_samples() {
        let summary = summarize_illuminance_grid(&[5.0, 10.0, 15.0], 3).unwrap();
        assert_eq!(summary.average_lx, 10.0);
        assert_eq!(summary.minimum_lx, 5.0);
        assert_eq!(summary.maximum_lx, 15.0);
        assert_eq!(summary.minimum_to_average_uniformity, 0.5);
    }

    #[test]
    fn grid_rejects_wrong_point_count() {
        assert_eq!(
            summarize_illuminance_grid(&[500.0, 510.0], 3),
            Err(GridError::PointCountMismatch)
        );
    }

    #[test]
    fn grid_rejects_non_finite_and_negative_samples() {
        assert_eq!(
            summarize_illuminance_grid(&[500.0, f64::NAN], 2),
            Err(GridError::InvalidSample)
        );
        assert_eq!(
            summarize_illuminance_grid(&[500.0, -1.0], 2),
            Err(GridError::InvalidSample)
        );
    }

    #[test]
    fn compliant_candidate_gets_metric_based_on_raw_grid() {
        let result = evaluate(&passing_candidate(), &limits());
        assert_eq!(result.status, EvaluationStatus::Compliant);
        assert_eq!(result.average_wall_power_w, Some(8.0));
        assert!(result.illuminance.unwrap().average_lx >= 500.0);
        assert_eq!(result.energy_wh_per_compliant_task_hour, Some(8.0));
    }

    #[test]
    fn incomplete_grid_cannot_be_ranked() {
        let mut candidate = passing_candidate();
        candidate.illuminance_samples_lx.pop();
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::InvalidInput);
        assert_eq!(result.energy_wh_per_compliant_task_hour, None);
        assert_eq!(result.reason_codes, vec!["grid_point_count_mismatch"]);
    }

    #[test]
    fn missing_required_gate_cannot_be_ranked() {
        let mut candidate = passing_candidate();
        candidate.gates.retain(|gate| gate.name != "flicker");
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::Unknown);
        assert_eq!(result.energy_wh_per_compliant_task_hour, None);
        assert!(result
            .reason_codes
            .iter()
            .any(|reason| reason == "required_gate_missing:flicker"));
    }

    #[test]
    fn known_numeric_failure_overrides_unknown_gate() {
        let mut candidate = passing_candidate();
        candidate.peak_wall_power_w = 16.0;
        candidate.gates[0].status = GateStatus::Unknown;
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::NonCompliant);
        assert_eq!(result.energy_wh_per_compliant_task_hour, None);
        assert!(result
            .reason_codes
            .iter()
            .any(|reason| reason == "peak_power_exceeds_limit"));
    }

    #[test]
    fn non_finite_numeric_input_is_invalid() {
        let mut candidate = passing_candidate();
        candidate.energy_wh = f64::NAN;
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::InvalidInput);
        assert_eq!(result.energy_wh_per_compliant_task_hour, None);
    }

    #[test]
    fn duplicate_gate_name_is_invalid() {
        let mut candidate = passing_candidate();
        candidate.gates.push(candidate.gates[0].clone());
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::InvalidInput);
        assert!(result.reason_codes.iter().any(|reason| reason == "duplicate_gate_name"));
    }

    #[test]
    fn zero_compliant_hours_cannot_produce_a_metric() {
        let mut candidate = passing_candidate();
        candidate.compliant_task_hours = 0.0;
        let result = evaluate(&candidate, &limits());
        assert_eq!(result.status, EvaluationStatus::NotEvaluated);
        assert_eq!(result.energy_wh_per_compliant_task_hour, None);
    }

    #[test]
    fn pareto_dominance_rejects_missing_or_non_finite_values() {
        assert!(!dominates_minimization(&[1.0], &[1.0, 2.0]));
        assert!(!dominates_minimization(&[f64::NAN, 1.0], &[2.0, 2.0]));
        assert!(!dominates_minimization(&[1.0], &[1.0]));
    }

    #[test]
    fn pareto_dominance_requires_one_strict_improvement() {
        assert!(dominates_minimization(&[2.0, 4.0], &[3.0, 4.0]));
        assert!(!dominates_minimization(&[2.0, 5.0], &[3.0, 4.0]));
    }
}
