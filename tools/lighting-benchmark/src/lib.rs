//! Fail-closed reference evaluator for a lighting-service benchmark.
//!
//! This crate evaluates supplied observations; it does not calculate photometry.

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
    /// This list comes from the frozen case, not from the candidate.
    pub required_gate_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CandidateObservation {
    pub candidate_id: String,
    pub energy_wh: f64,
    pub observed_hours: f64,
    pub compliant_task_hours: f64,
    pub peak_wall_power_w: f64,
    pub average_illuminance_lx: f64,
    pub minimum_illuminance_lx: f64,
    pub minimum_to_average_uniformity: f64,
    pub gates: Vec<GateResult>,
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
    pub energy_wh_per_compliant_task_hour: Option<f64>,
    pub reason_codes: Vec<String>,
}

impl Evaluation {
    fn invalid(candidate_id: &str, reason: &str) -> Self {
        Self {
            candidate_id: candidate_id.to_owned(),
            status: EvaluationStatus::InvalidInput,
            average_wall_power_w: None,
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

    // A known failure is enough to reject rankability. Otherwise, preserve the
    // most concrete unresolved evidence state instead of turning it into a pass.
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
/// The energy/service metric is withheld unless every numeric constraint and
/// every case-required evidence gate passes.
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
        || !finite_nonnegative(candidate.average_illuminance_lx)
        || !finite_nonnegative(candidate.minimum_illuminance_lx)
        || !candidate.minimum_to_average_uniformity.is_finite()
        || !(0.0..=1.0).contains(&candidate.minimum_to_average_uniformity)
        || candidate.energy_wh
            > candidate.peak_wall_power_w * candidate.observed_hours + 1.0e-6
    {
        return Evaluation::invalid(&candidate.candidate_id, "invalid_numeric_observation");
    }

    let average_wall_power_w = candidate.energy_wh / candidate.observed_hours;
    let mut reasons = Vec::new();
    let mut numeric_failure = false;

    if candidate.average_illuminance_lx < limits.average_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("average_illuminance_below_target".to_owned());
    }
    if candidate.minimum_illuminance_lx < limits.minimum_illuminance_min_lx {
        numeric_failure = true;
        reasons.push("minimum_illuminance_below_target".to_owned());
    }
    if candidate.minimum_to_average_uniformity < limits.minimum_to_average_uniformity_min {
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
        energy_wh_per_compliant_task_hour: energy_metric,
        reason_codes: reasons,
    }
}

/// True only when vector a Pareto-dominates vector b for equal-length, finite
/// minimization vectors. Missing objectives must be handled before this call;
/// they are never replaced with zero.
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
        CandidateObservation {
            candidate_id: "synthetic-candidate-a".to_owned(),
            energy_wh: 8.0,
            observed_hours: 1.0,
            compliant_task_hours: 1.0,
            peak_wall_power_w: 10.0,
            average_illuminance_lx: 520.0,
            minimum_illuminance_lx: 310.0,
            minimum_to_average_uniformity: 0.62,
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
    fn compliant_candidate_gets_energy_service_metric() {
        let result = evaluate(&passing_candidate(), &limits());
        assert_eq!(result.status, EvaluationStatus::Compliant);
        assert_eq!(result.average_wall_power_w, Some(8.0));
        assert_eq!(result.energy_wh_per_compliant_task_hour, Some(8.0));
        assert!(result.reason_codes.is_empty());
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
        candidate.average_illuminance_lx = f64::NAN;
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
    fn missing_and_non_finite_pareto_objectives_never_dominate() {
        assert!(!dominates_minimization(&[1.0], &[1.0, 2.0]));
        assert!(!dominates_minimization(&[f64::NAN, 1.0], &[2.0, 2.0]));
        assert!(!dominates_minimization(&[1.0], &[1.0]));
    }

    #[test]
    fn pareto_dominance_requires_no_worse_and_one_strict_improvement() {
        assert!(dominates_minimization(&[2.0, 4.0], &[3.0, 4.0]));
        assert!(!dominates_minimization(&[2.0, 5.0], &[3.0, 4.0]));
    }
}
