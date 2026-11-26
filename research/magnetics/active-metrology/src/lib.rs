//! Decision-directed deterministic planning for magnetic field measurements.
//!
//! This crate ranks supplied scan locations using qualified model disagreement,
//! functional-margin sensitivity, measurement error bounds and acquisition cost.
//! It never consumes blind holdout or qualification evidence.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementRole {
    AdaptiveExploration,
    ModelCalibration,
    BlindHoldout,
    PostSelectionQualification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanCandidate {
    pub candidate_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub probe_calibration_record_id: String,
    pub qualified_model_set_id: String,
    pub functional_requirement_id: String,
    pub role: MeasurementRole,
    pub x_m: f64,
    pub y_m: f64,
    pub z_m: f64,
    pub standoff_m: f64,
    /// Upper bound on the pointwise diameter of applicable qualified model predictions, tesla.
    pub model_disagreement_bound_t: Option<f64>,
    /// Absolute sensitivity of the named functional margin to local field, radians/tesla.
    pub functional_margin_sensitivity_rad_per_t: Option<f64>,
    /// Simultaneous absolute measurement error bound at this point, tesla.
    pub measurement_error_bound_t: f64,
    pub acquisition_cost_s: f64,
    pub already_measured: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanPolicy {
    pub plan_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub probe_calibration_record_id: String,
    pub qualified_model_set_id: String,
    pub functional_requirement_id: String,
    /// Minimum Euclidean separation between selected probe locations, metres.
    pub minimum_separation_m: f64,
    pub maximum_acquisition_cost_s: f64,
    pub maximum_point_count: usize,
    pub minimum_priority_rad_per_s: f64,
    pub allow_revisit: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRankableReason {
    MissingQualifiedModelDisagreement,
    MissingFunctionalSensitivity,
    NoPositiveDecisionImpact,
    MeasurementBoundMasksModelDisagreement,
    ProtectedEvidenceRole,
    AlreadyMeasured,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusionReason {
    ProtectedEvidenceRole,
    AlreadyMeasured,
    BelowPriorityFloor,
    InsufficientRemainingBudget,
    MinimumSeparationConflict,
    MaximumPointCountReached,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CandidateDisposition {
    Selected { order: usize },
    NotSelected(ExclusionReason),
    NotRankable(NotRankableReason),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanCandidateMetrics {
    pub candidate_id: String,
    pub input_index: usize,
    pub model_disagreement_bound_t: Option<f64>,
    pub measurement_error_bound_t: f64,
    pub functional_margin_sensitivity_rad_per_t: Option<f64>,
    /// Conservative upper bound on the effect of model disagreement on the named margin.
    pub decision_impact_bound_rad: Option<f64>,
    /// Portion of model separation that exceeds two-sided measurement error.
    pub resolvable_model_separation_bound_t: Option<f64>,
    pub resolvable_decision_impact_bound_rad: Option<f64>,
    /// Resolvable decision impact divided by acquisition cost.
    pub priority_rad_per_s: Option<f64>,
    pub disposition: CandidateDisposition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScanPlan {
    pub plan_id: String,
    pub coordinate_frame_id: String,
    pub operating_condition_id: String,
    pub probe_calibration_record_id: String,
    pub qualified_model_set_id: String,
    pub functional_requirement_id: String,
    pub candidate_count: usize,
    pub selected_point_count: usize,
    pub selected_acquisition_cost_s: f64,
    pub remaining_acquisition_budget_s: f64,
    /// Candidate IDs in acquisition order; no randomized tie-breaking is used.
    pub selected_candidate_ids: Vec<String>,
    /// Metrics preserve the original candidate input order.
    pub candidates: Vec<ScanCandidateMetrics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlanningError {
    InvalidPolicy,
    InvalidCandidateMetadata { index: usize },
    DuplicateCandidateId,
    SourceIdentityMismatch { index: usize },
    InvalidCoordinate { index: usize },
    InvalidMeasurementBound { index: usize },
    InvalidAcquisitionCost { index: usize },
    InvalidModelDisagreement { index: usize },
    InvalidFunctionalSensitivity { index: usize },
    NumericOverflow { index: usize },
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_policy(policy: &ScanPolicy) -> Result<(), PlanningError> {
    if !nonempty(&policy.plan_id)
        || !nonempty(&policy.coordinate_frame_id)
        || !nonempty(&policy.operating_condition_id)
        || !nonempty(&policy.probe_calibration_record_id)
        || !nonempty(&policy.qualified_model_set_id)
        || !nonempty(&policy.functional_requirement_id)
        || !policy.minimum_separation_m.is_finite()
        || policy.minimum_separation_m < 0.0
        || !policy.maximum_acquisition_cost_s.is_finite()
        || policy.maximum_acquisition_cost_s <= 0.0
        || policy.maximum_point_count == 0
        || !policy.minimum_priority_rad_per_s.is_finite()
        || policy.minimum_priority_rad_per_s < 0.0
    {
        return Err(PlanningError::InvalidPolicy);
    }
    Ok(())
}

fn distance_squared(left: &ScanCandidate, right: &ScanCandidate) -> f64 {
    let dx = left.x_m - right.x_m;
    let dy = left.y_m - right.y_m;
    let dz = left.z_m - right.z_m;
    dx.mul_add(dx, dy.mul_add(dy, dz * dz))
}

fn validate_candidate(
    candidate: &ScanCandidate,
    index: usize,
    policy: &ScanPolicy,
) -> Result<(), PlanningError> {
    if !nonempty(&candidate.candidate_id)
        || !nonempty(&candidate.coordinate_frame_id)
        || !nonempty(&candidate.operating_condition_id)
        || !nonempty(&candidate.probe_calibration_record_id)
        || !nonempty(&candidate.qualified_model_set_id)
        || !nonempty(&candidate.functional_requirement_id)
    {
        return Err(PlanningError::InvalidCandidateMetadata { index });
    }

    if candidate.coordinate_frame_id != policy.coordinate_frame_id
        || candidate.operating_condition_id != policy.operating_condition_id
        || candidate.probe_calibration_record_id != policy.probe_calibration_record_id
        || candidate.qualified_model_set_id != policy.qualified_model_set_id
        || candidate.functional_requirement_id != policy.functional_requirement_id
    {
        return Err(PlanningError::SourceIdentityMismatch { index });
    }

    if [
        candidate.x_m,
        candidate.y_m,
        candidate.z_m,
        candidate.standoff_m,
    ]
    .into_iter()
    .any(|value| !value.is_finite())
        || candidate.standoff_m < 0.0
    {
        return Err(PlanningError::InvalidCoordinate { index });
    }

    if !candidate.measurement_error_bound_t.is_finite()
        || candidate.measurement_error_bound_t < 0.0
    {
        return Err(PlanningError::InvalidMeasurementBound { index });
    }

    if !candidate.acquisition_cost_s.is_finite() || candidate.acquisition_cost_s <= 0.0 {
        return Err(PlanningError::InvalidAcquisitionCost { index });
    }

    if candidate
        .model_disagreement_bound_t
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(PlanningError::InvalidModelDisagreement { index });
    }

    if candidate
        .functional_margin_sensitivity_rad_per_t
        .is_some_and(|value| !value.is_finite() || value < 0.0)
    {
        return Err(PlanningError::InvalidFunctionalSensitivity { index });
    }

    Ok(())
}

fn candidate_metrics(
    candidate: &ScanCandidate,
    input_index: usize,
) -> Result<ScanCandidateMetrics, PlanningError> {
    let not_rankable = |reason| ScanCandidateMetrics {
        candidate_id: candidate.candidate_id.clone(),
        input_index,
        model_disagreement_bound_t: candidate.model_disagreement_bound_t,
        measurement_error_bound_t: candidate.measurement_error_bound_t,
        functional_margin_sensitivity_rad_per_t: candidate
            .functional_margin_sensitivity_rad_per_t,
        decision_impact_bound_rad: None,
        resolvable_model_separation_bound_t: None,
        resolvable_decision_impact_bound_rad: None,
        priority_rad_per_s: None,
        disposition: CandidateDisposition::NotRankable(reason),
    };

    if candidate.role != MeasurementRole::AdaptiveExploration {
        return Ok(not_rankable(NotRankableReason::ProtectedEvidenceRole));
    }
    if candidate.already_measured {
        return Ok(not_rankable(NotRankableReason::AlreadyMeasured));
    }

    let Some(disagreement) = candidate.model_disagreement_bound_t else {
        return Ok(not_rankable(NotRankableReason::MissingQualifiedModelDisagreement));
    };
    let Some(sensitivity) = candidate.functional_margin_sensitivity_rad_per_t else {
        return Ok(not_rankable(NotRankableReason::MissingFunctionalSensitivity));
    };
    if sensitivity <= 0.0 || disagreement <= 0.0 {
        return Ok(not_rankable(NotRankableReason::NoPositiveDecisionImpact));
    }

    // Two model predictions with separation D are distinguishable by a
    // measurement with absolute error bound u only when D > 2u.
    let resolvable_separation =
        (disagreement - 2.0 * candidate.measurement_error_bound_t).max(0.0);
    if resolvable_separation <= 0.0 {
        return Ok(not_rankable(
            NotRankableReason::MeasurementBoundMasksModelDisagreement,
        ));
    }

    let decision_impact = disagreement * sensitivity;
    let resolvable_impact = resolvable_separation * sensitivity;
    let priority = resolvable_impact / candidate.acquisition_cost_s;
    if !decision_impact.is_finite()
        || !resolvable_impact.is_finite()
        || !priority.is_finite()
    {
        return Err(PlanningError::NumericOverflow { index: input_index });
    }

    Ok(ScanCandidateMetrics {
        candidate_id: candidate.candidate_id.clone(),
        input_index,
        model_disagreement_bound_t: Some(disagreement),
        measurement_error_bound_t: candidate.measurement_error_bound_t,
        functional_margin_sensitivity_rad_per_t: Some(sensitivity),
        decision_impact_bound_rad: Some(decision_impact),
        resolvable_model_separation_bound_t: Some(resolvable_separation),
        resolvable_decision_impact_bound_rad: Some(resolvable_impact),
        priority_rad_per_s: Some(priority),
        disposition: CandidateDisposition::NotSelected(ExclusionReason::BelowPriorityFloor),
    })
}

/// Produce a deterministic scan order from supplied exploratory candidates.
///
/// The planner uses a transparent worst-case decision-impact bound:
///
///   max(0, model_disagreement - 2 * measurement_error) * |d margin / d B|
///
/// divided by acquisition cost. This is a deterministic screening heuristic,
/// not expected information gain, a probability or a claim about model truth.
/// Protected calibration/holdout/qualification candidates are never selected.
pub fn plan_decision_directed_scan(
    candidates: &[ScanCandidate],
    policy: &ScanPolicy,
) -> Result<ScanPlan, PlanningError> {
    validate_policy(policy)?;

    let mut identifiers = std::collections::HashSet::with_capacity(candidates.len());
    let mut metrics = Vec::with_capacity(candidates.len());
    let mut ranked = Vec::new();

    for (index, candidate) in candidates.iter().enumerate() {
        validate_candidate(candidate, index, policy)?;
        if !identifiers.insert(candidate.candidate_id.as_str()) {
            return Err(PlanningError::DuplicateCandidateId);
        }

        let mut result = candidate_metrics(candidate, index)?;
        if let Some(priority) = result.priority_rad_per_s {
            if priority >= policy.minimum_priority_rad_per_s {
                ranked.push(index);
                result.disposition = CandidateDisposition::NotSelected(
                    ExclusionReason::BelowPriorityFloor,
                );
            }
        }
        metrics.push(result);
    }

    // Explicit deterministic order: score descending, then stable candidate ID,
    // then original input index. No randomness or hidden model-generated tie-break.
    ranked.sort_by(|left, right| {
        let left_score = metrics[*left].priority_rad_per_s.unwrap_or(0.0);
        let right_score = metrics[*right].priority_rad_per_s.unwrap_or(0.0);
        right_score
            .total_cmp(&left_score)
            .then_with(|| candidates[*left].candidate_id.cmp(&candidates[*right].candidate_id))
            .then_with(|| left.cmp(right))
    });

    let mut selected = Vec::new();
    let mut selected_candidate_ids = Vec::new();
    let mut total_cost = 0.0_f64;

    for candidate_index in ranked {
        let candidate = &candidates[candidate_index];
        if selected.len() >= policy.maximum_point_count {
            metrics[candidate_index].disposition = CandidateDisposition::NotSelected(
                ExclusionReason::MaximumPointCountReached,
            );
            continue;
        }

        let remaining_budget = policy.maximum_acquisition_cost_s - total_cost;
        if candidate.acquisition_cost_s > remaining_budget {
            metrics[candidate_index].disposition = CandidateDisposition::NotSelected(
                ExclusionReason::InsufficientRemainingBudget,
            );
            continue;
        }

        let minimum_distance_squared = policy.minimum_separation_m.powi(2);
        let conflicts = selected.iter().any(|selected_index| {
            distance_squared(candidate, &candidates[*selected_index])
                < minimum_distance_squared
        });
        if conflicts {
            metrics[candidate_index].disposition = CandidateDisposition::NotSelected(
                ExclusionReason::MinimumSeparationConflict,
            );
            continue;
        }

        let order = selected.len();
        selected.push(candidate_index);
        selected_candidate_ids.push(candidate.candidate_id.clone());
        total_cost += candidate.acquisition_cost_s;
        metrics[candidate_index].disposition = CandidateDisposition::Selected { order };
    }

    Ok(ScanPlan {
        plan_id: policy.plan_id.clone(),
        coordinate_frame_id: policy.coordinate_frame_id.clone(),
        operating_condition_id: policy.operating_condition_id.clone(),
        probe_calibration_record_id: policy.probe_calibration_record_id.clone(),
        qualified_model_set_id: policy.qualified_model_set_id.clone(),
        functional_requirement_id: policy.functional_requirement_id.clone(),
        candidate_count: candidates.len(),
        selected_point_count: selected.len(),
        selected_acquisition_cost_s: total_cost,
        remaining_acquisition_budget_s: policy.maximum_acquisition_cost_s - total_cost,
        selected_candidate_ids,
        candidates: metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> ScanPolicy {
        ScanPolicy {
            plan_id: "scan-plan-001".to_owned(),
            coordinate_frame_id: "frame-001".to_owned(),
            operating_condition_id: "condition-001".to_owned(),
            probe_calibration_record_id: "probe-cal-001".to_owned(),
            qualified_model_set_id: "models-001".to_owned(),
            functional_requirement_id: "encoder-phase-margin-001".to_owned(),
            minimum_separation_m: 0.002,
            maximum_acquisition_cost_s: 10.0,
            maximum_point_count: 4,
            minimum_priority_rad_per_s: 0.0,
            allow_revisit: false,
        }
    }

    fn candidate(
        id: &str,
        x: f64,
        disagreement_t: f64,
        sensitivity: f64,
        error_t: f64,
        cost_s: f64,
    ) -> ScanCandidate {
        ScanCandidate {
            candidate_id: id.to_owned(),
            coordinate_frame_id: "frame-001".to_owned(),
            operating_condition_id: "condition-001".to_owned(),
            probe_calibration_record_id: "probe-cal-001".to_owned(),
            qualified_model_set_id: "models-001".to_owned(),
            functional_requirement_id: "encoder-phase-margin-001".to_owned(),
            role: MeasurementRole::AdaptiveExploration,
            x_m: x,
            y_m: 0.0,
            z_m: 0.0,
            standoff_m: 0.003,
            model_disagreement_bound_t: Some(disagreement_t),
            functional_margin_sensitivity_rad_per_t: Some(sensitivity),
            measurement_error_bound_t: error_t,
            acquisition_cost_s: cost_s,
            already_measured: false,
        }
    }

    #[test]
    fn higher_decision_impact_per_cost_is_selected_first() {
        let points = vec![
            candidate("low", 0.0, 0.010, 1.0, 0.001, 2.0),
            candidate("high", 0.01, 0.020, 3.0, 0.001, 1.0),
        ];
        let result = plan_decision_directed_scan(&points, &policy()).expect("valid plan");
        assert_eq!(result.selected_candidate_ids, vec!["high", "low"]);
        assert!(matches!(
            result.candidates[1].disposition,
            CandidateDisposition::Selected { order: 0 }
        ));
    }

    #[test]
    fn field_disagreement_without_functional_sensitivity_is_not_ranked() {
        let mut point = candidate("no-sensitivity", 0.0, 0.05, 0.0, 0.001, 1.0);
        point.functional_margin_sensitivity_rad_per_t = None;
        let result = plan_decision_directed_scan(&[point], &policy()).expect("valid plan");
        assert!(matches!(
            result.candidates[0].disposition,
            CandidateDisposition::NotRankable(
                NotRankableReason::MissingFunctionalSensitivity
            )
        ));
        assert_eq!(result.selected_point_count, 0);
    }

    #[test]
    fn measurement_error_bound_must_not_mask_model_disagreement() {
        let point = candidate("noise-masked", 0.0, 0.010, 5.0, 0.006, 1.0);
        let result = plan_decision_directed_scan(&[point], &policy()).expect("valid plan");
        assert!(matches!(
            result.candidates[0].disposition,
            CandidateDisposition::NotRankable(
                NotRankableReason::MeasurementBoundMasksModelDisagreement
            )
        ));
    }

    #[test]
    fn blind_holdout_and_qualification_points_are_never_selected() {
        let mut holdout = candidate("blind", 0.0, 1.0, 10.0, 0.001, 0.1);
        holdout.role = MeasurementRole::BlindHoldout;
        let mut qualification = candidate("qualification", 0.01, 1.0, 10.0, 0.001, 0.1);
        qualification.role = MeasurementRole::PostSelectionQualification;
        let result =
            plan_decision_directed_scan(&[holdout, qualification], &policy()).expect("valid plan");
        assert_eq!(result.selected_point_count, 0);
        assert!(result.candidates.iter().all(|candidate| matches!(
            candidate.disposition,
            CandidateDisposition::NotRankable(NotRankableReason::ProtectedEvidenceRole)
        )));
    }

    #[test]
    fn minimum_separation_is_enforced_in_three_dimensions() {
        let points = vec![
            candidate("first", 0.0, 0.02, 1.0, 0.001, 1.0),
            candidate("too-close", 0.001, 0.03, 1.0, 0.001, 1.0),
            candidate("far", 0.01, 0.015, 1.0, 0.001, 1.0),
        ];
        let result = plan_decision_directed_scan(&points, &policy()).expect("valid plan");
        assert_eq!(result.selected_candidate_ids, vec!["too-close", "far"]);
        assert!(matches!(
            result.candidates[0].disposition,
            CandidateDisposition::NotSelected(ExclusionReason::MinimumSeparationConflict)
        ));
    }

    #[test]
    fn scan_budget_is_respected_and_remaining_budget_is_reported() {
        let points = vec![
            candidate("expensive", 0.0, 0.04, 1.0, 0.001, 8.0),
            candidate("small-a", 0.01, 0.02, 1.0, 0.001, 3.0),
            candidate("small-b", 0.02, 0.015, 1.0, 0.001, 3.0),
        ];
        let mut p = policy();
        p.maximum_acquisition_cost_s = 6.0;
        let result = plan_decision_directed_scan(&points, &p).expect("valid plan");
        assert_eq!(result.selected_candidate_ids, vec!["small-a", "small-b"]);
        assert!((result.selected_acquisition_cost_s - 6.0).abs() < 1.0e-12);
        assert!(result.remaining_acquisition_budget_s.abs() < 1.0e-12);
    }

    #[test]
    fn exact_ties_use_stable_candidate_id_not_random_order() {
        let points = vec![
            candidate("zeta", 0.0, 0.02, 1.0, 0.001, 1.0),
            candidate("alpha", 0.01, 0.02, 1.0, 0.001, 1.0),
        ];
        let result = plan_decision_directed_scan(&points, &policy()).expect("valid plan");
        assert_eq!(result.selected_candidate_ids, vec!["alpha", "zeta"]);
    }

    #[test]
    fn source_identity_mismatch_fails_closed() {
        let mut point = candidate("wrong-frame", 0.0, 0.02, 1.0, 0.001, 1.0);
        point.coordinate_frame_id = "other-frame".to_owned();
        assert!(matches!(
            plan_decision_directed_scan(&[point], &policy()),
            Err(PlanningError::SourceIdentityMismatch { index: 0 })
        ));
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let points = vec![
            candidate("duplicate", 0.0, 0.02, 1.0, 0.001, 1.0),
            candidate("duplicate", 0.01, 0.03, 1.0, 0.001, 1.0),
        ];
        assert_eq!(
            plan_decision_directed_scan(&points, &policy()).expect_err("duplicate IDs"),
            PlanningError::DuplicateCandidateId
        );
    }

    #[test]
    fn nonfinite_geometry_and_negative_error_bounds_are_rejected() {
        let mut point = candidate("invalid", 0.0, 0.02, 1.0, 0.001, 1.0);
        point.x_m = f64::NAN;
        assert!(matches!(
            plan_decision_directed_scan(&[point], &policy()),
            Err(PlanningError::InvalidCoordinate { index: 0 })
        ));

        let mut point = candidate("invalid-error", 0.0, 0.02, 1.0, 0.001, 1.0);
        point.measurement_error_bound_t = -0.001;
        assert!(matches!(
            plan_decision_directed_scan(&[point], &policy()),
            Err(PlanningError::InvalidMeasurementBound { index: 0 })
        ));
    }

    #[test]
    fn previously_measured_points_are_not_revisited_by_default() {
        let mut point = candidate("repeated", 0.0, 0.02, 1.0, 0.001, 1.0);
        point.already_measured = true;
        let result = plan_decision_directed_scan(&[point], &policy()).expect("valid plan");
        assert_eq!(result.selected_point_count, 0);
        assert!(matches!(
            result.candidates[0].disposition,
            CandidateDisposition::NotRankable(NotRankableReason::AlreadyMeasured)
        ));
    }

    #[test]
    fn missing_model_disagreement_is_not_replaced_with_a_guess() {
        let mut point = candidate("no-model-bound", 0.0, 0.02, 1.0, 0.001, 1.0);
        point.model_disagreement_bound_t = None;
        let result = plan_decision_directed_scan(&[point], &policy()).expect("valid plan");
        assert!(matches!(
            result.candidates[0].disposition,
            CandidateDisposition::NotRankable(
                NotRankableReason::MissingQualifiedModelDisagreement
            )
        ));
    }
}
