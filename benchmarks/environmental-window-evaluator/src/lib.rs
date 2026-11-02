//! Synthetic-first source–pathway–receptor work-window eligibility evaluator.
//!
//! This crate evaluates policy/evidence contracts. It does not model physical
//! environmental exposure, grant permits, authorize work or dispatch machinery.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityScope {
    pub activity_id: String,
    pub activity_revision: String,
    pub site_id: String,
    pub site_revision: String,
    pub context_id: String,
    pub receptor_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceBinding {
    pub scope: ActivityScope,
    pub model_id: String,
    pub model_revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeInterval {
    /// Inclusive start tick.
    pub start_tick: u64,
    /// Exclusive end tick.
    pub end_tick: u64,
}

impl TimeInterval {
    pub fn is_valid(self) -> bool {
        self.start_tick < self.end_tick
    }

    pub fn contains(self, other: Self) -> bool {
        self.is_valid()
            && other.is_valid()
            && self.start_tick <= other.start_tick
            && self.end_tick >= other.end_tick
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticExposure {
    pub unit_id: String,
    pub value_microunits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SprModelProfile {
    pub model_id: String,
    pub model_revision: String,
    pub receptor_id: String,
    pub supported_contexts: Vec<String>,
    pub threshold: SyntheticExposure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowPolicy {
    Fixed,
    Reactive,
    Proactive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HardGate {
    Clear,
    Blocked { gate_id: String },
    Unknown { gate_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonitoringEvidence {
    pub evidence_id: String,
    pub binding: EvidenceBinding,
    pub observed_at_tick: u64,
    /// Declared session coverage, not a claim that one sample is continuous monitoring.
    pub covered_interval: TimeInterval,
    pub calibrated: bool,
    pub spatial_coverage_complete: bool,
    pub maximum_exposure: SyntheticExposure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForecastEvidence {
    pub evidence_id: String,
    pub binding: EvidenceBinding,
    pub issued_at_tick: u64,
    pub valid_interval: TimeInterval,
    pub maximum_exposure: SyntheticExposure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentalWindowCase {
    pub scope: ActivityScope,
    pub planned_interval: TimeInterval,
    pub allowed_intervals: Vec<TimeInterval>,
    pub evaluation_tick: u64,
    pub maximum_monitoring_age_ticks: u64,
    pub maximum_forecast_age_ticks: u64,
    pub minimum_proactive_lead_ticks: u64,
    pub policy: WindowPolicy,
    pub hard_gate: HardGate,
    pub monitoring: Option<MonitoringEvidence>,
    pub forecast: Option<ForecastEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowDisposition {
    EligibleForSyntheticWindow,
    BlockedByHardConstraint,
    UnknownReviewRequired,
    EvidenceInconsistent,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowReason {
    InvalidScope,
    InvalidInterval,
    InvalidModel,
    EmptyAllowedIntervals,
    InvalidAllowedInterval,
    OutsideAllowedInterval,
    HardGateBlocked,
    HardGateUnknown,
    UnsupportedContext,
    MissingMonitoring,
    InvalidEvidenceIdentity,
    MonitoringBindingMismatch,
    FutureObservation,
    MonitoringStale,
    MonitoringSessionDoesNotCoverWindow,
    MonitoringNotCalibrated,
    MonitoringCoverageIncomplete,
    ExposureUnitMismatch,
    ExposureThresholdExceeded,
    MissingForecast,
    ForecastBindingMismatch,
    FutureForecast,
    ForecastStale,
    ForecastDoesNotCoverWindow,
    ProactiveLeadTimeInsufficient,
    ForecastThresholdExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowEvaluation {
    pub disposition: WindowDisposition,
    /// Reasons have deterministic ordering; no score or hidden weighting is used.
    pub reasons: Vec<WindowReason>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn valid_scope(scope: &ActivityScope) -> bool {
    [
        scope.activity_id.as_str(),
        scope.activity_revision.as_str(),
        scope.site_id.as_str(),
        scope.site_revision.as_str(),
        scope.context_id.as_str(),
        scope.receptor_id.as_str(),
    ]
    .iter()
    .all(|value| nonempty(value))
}

fn binding_matches(
    binding: &EvidenceBinding,
    scope: &ActivityScope,
    model: &SprModelProfile,
) -> bool {
    binding.scope == *scope
        && binding.model_id == model.model_id
        && binding.model_revision == model.model_revision
}

fn evaluate_monitoring(
    case: &EnvironmentalWindowCase,
    model: &SprModelProfile,
    reasons: &mut Vec<WindowReason>,
) -> (bool, bool, bool) {
    let Some(evidence) = &case.monitoring else {
        reasons.push(WindowReason::MissingMonitoring);
        return (false, true, false);
    };

    let mut inconsistent = false;
    let mut unknown = false;
    let mut blocked = false;

    if !nonempty(&evidence.evidence_id) {
        reasons.push(WindowReason::InvalidEvidenceIdentity);
        inconsistent = true;
    }
    if !binding_matches(&evidence.binding, &case.scope, model) {
        reasons.push(WindowReason::MonitoringBindingMismatch);
        inconsistent = true;
    }
    if evidence.observed_at_tick > case.evaluation_tick {
        reasons.push(WindowReason::FutureObservation);
        inconsistent = true;
    } else if case.evaluation_tick - evidence.observed_at_tick
        > case.maximum_monitoring_age_ticks
    {
        reasons.push(WindowReason::MonitoringStale);
        unknown = true;
    }
    if !evidence.covered_interval.contains(case.planned_interval) {
        reasons.push(WindowReason::MonitoringSessionDoesNotCoverWindow);
        unknown = true;
    }
    if !evidence.calibrated {
        reasons.push(WindowReason::MonitoringNotCalibrated);
        unknown = true;
    }
    if !evidence.spatial_coverage_complete {
        reasons.push(WindowReason::MonitoringCoverageIncomplete);
        unknown = true;
    }
    if evidence.maximum_exposure.unit_id != model.threshold.unit_id
        || !nonempty(&evidence.maximum_exposure.unit_id)
    {
        reasons.push(WindowReason::ExposureUnitMismatch);
        inconsistent = true;
    } else if !inconsistent && !unknown
        && evidence.maximum_exposure.value_microunits >= model.threshold.value_microunits
    {
        reasons.push(WindowReason::ExposureThresholdExceeded);
        blocked = true;
    }

    (inconsistent, unknown, blocked)
}

fn evaluate_forecast(
    case: &EnvironmentalWindowCase,
    model: &SprModelProfile,
    reasons: &mut Vec<WindowReason>,
) -> (bool, bool, bool) {
    let Some(evidence) = &case.forecast else {
        reasons.push(WindowReason::MissingForecast);
        return (false, true, false);
    };

    let mut inconsistent = false;
    let mut unknown = false;
    let mut blocked = false;

    if !nonempty(&evidence.evidence_id) {
        reasons.push(WindowReason::InvalidEvidenceIdentity);
        inconsistent = true;
    }
    if !binding_matches(&evidence.binding, &case.scope, model) {
        reasons.push(WindowReason::ForecastBindingMismatch);
        inconsistent = true;
    }
    if evidence.issued_at_tick > case.evaluation_tick {
        reasons.push(WindowReason::FutureForecast);
        inconsistent = true;
    } else if case.evaluation_tick - evidence.issued_at_tick
        > case.maximum_forecast_age_ticks
    {
        reasons.push(WindowReason::ForecastStale);
        unknown = true;
    }
    if !evidence.valid_interval.contains(case.planned_interval) {
        reasons.push(WindowReason::ForecastDoesNotCoverWindow);
        unknown = true;
    }
    if evidence.maximum_exposure.unit_id != model.threshold.unit_id
        || !nonempty(&evidence.maximum_exposure.unit_id)
    {
        reasons.push(WindowReason::ExposureUnitMismatch);
        inconsistent = true;
    } else if !inconsistent && !unknown
        && evidence.maximum_exposure.value_microunits >= model.threshold.value_microunits
    {
        reasons.push(WindowReason::ForecastThresholdExceeded);
        blocked = true;
    }

    (inconsistent, unknown, blocked)
}

/// Deterministic synthetic window evaluation. This is never a permit or dispatch decision.
#[must_use]
pub fn evaluate_window(
    case: &EnvironmentalWindowCase,
    model: &SprModelProfile,
) -> WindowEvaluation {
    let mut reasons = Vec::new();
    let mut inconsistent = false;
    let mut unknown = false;
    let mut blocked = false;
    let mut unsupported = false;

    if !valid_scope(&case.scope) {
        reasons.push(WindowReason::InvalidScope);
        inconsistent = true;
    }
    if !case.planned_interval.is_valid() {
        reasons.push(WindowReason::InvalidInterval);
        inconsistent = true;
    }
    if !nonempty(&model.model_id)
        || !nonempty(&model.model_revision)
        || !nonempty(&model.receptor_id)
        || !nonempty(&model.threshold.unit_id)
        || model.threshold.value_microunits == 0
        || model.supported_contexts.is_empty()
    {
        reasons.push(WindowReason::InvalidModel);
        inconsistent = true;
    }
    if model.receptor_id != case.scope.receptor_id {
        reasons.push(WindowReason::UnsupportedContext);
        unsupported = true;
    }
    if !model.supported_contexts.iter().any(|context| context == &case.scope.context_id) {
        reasons.push(WindowReason::UnsupportedContext);
        unsupported = true;
    }
    if case.allowed_intervals.is_empty() {
        reasons.push(WindowReason::EmptyAllowedIntervals);
        unknown = true;
    }
    if case.allowed_intervals.iter().any(|interval| !interval.is_valid()) {
        reasons.push(WindowReason::InvalidAllowedInterval);
        inconsistent = true;
    }
    if !case.allowed_intervals.iter().any(|window| window.contains(case.planned_interval)) {
        reasons.push(WindowReason::OutsideAllowedInterval);
        blocked = true;
    }

    match &case.hard_gate {
        HardGate::Clear => {}
        HardGate::Blocked { gate_id } => {
            if !nonempty(gate_id) {
                reasons.push(WindowReason::InvalidScope);
                inconsistent = true;
            } else {
                reasons.push(WindowReason::HardGateBlocked);
                blocked = true;
            }
        }
        HardGate::Unknown { gate_id } => {
            if !nonempty(gate_id) {
                reasons.push(WindowReason::InvalidScope);
                inconsistent = true;
            } else {
                reasons.push(WindowReason::HardGateUnknown);
                unknown = true;
            }
        }
    }

    if case.policy != WindowPolicy::Fixed {
        let (bad, needs_review, threshold_blocked) =
            evaluate_monitoring(case, model, &mut reasons);
        inconsistent |= bad;
        unknown |= needs_review;
        blocked |= threshold_blocked;
    }

    if case.policy == WindowPolicy::Proactive {
        if case.planned_interval.start_tick < case.evaluation_tick
            || case.planned_interval.start_tick - case.evaluation_tick
                < case.minimum_proactive_lead_ticks
        {
            reasons.push(WindowReason::ProactiveLeadTimeInsufficient);
            unknown = true;
        }
        let (bad, needs_review, threshold_blocked) =
            evaluate_forecast(case, model, &mut reasons);
        inconsistent |= bad;
        unknown |= needs_review;
        blocked |= threshold_blocked;
    }

    let disposition = if inconsistent {
        WindowDisposition::EvidenceInconsistent
    } else if unsupported {
        WindowDisposition::Unsupported
    } else if blocked {
        WindowDisposition::BlockedByHardConstraint
    } else if unknown {
        WindowDisposition::UnknownReviewRequired
    } else {
        WindowDisposition::EligibleForSyntheticWindow
    };

    WindowEvaluation { disposition, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ActivityScope {
        ActivityScope {
            activity_id: "dredge-campaign-01".into(),
            activity_revision: "r4".into(),
            site_id: "inlet-east".into(),
            site_revision: "geo-12".into(),
            context_id: "temperate-estuary-synthetic".into(),
            receptor_id: "receptor-fish-01".into(),
        }
    }

    fn binding() -> EvidenceBinding {
        EvidenceBinding {
            scope: scope(),
            model_id: "spr-model-01".into(),
            model_revision: "m2".into(),
        }
    }

    fn model() -> SprModelProfile {
        SprModelProfile {
            model_id: "spr-model-01".into(),
            model_revision: "m2".into(),
            receptor_id: "receptor-fish-01".into(),
            supported_contexts: vec!["temperate-estuary-synthetic".into()],
            threshold: SyntheticExposure {
                unit_id: "synthetic-exposure-u1".into(),
                value_microunits: 100,
            },
        }
    }

    fn case(policy: WindowPolicy) -> EnvironmentalWindowCase {
        EnvironmentalWindowCase {
            scope: scope(),
            planned_interval: TimeInterval {
                start_tick: 120,
                end_tick: 160,
            },
            allowed_intervals: vec![TimeInterval {
                start_tick: 100,
                end_tick: 200,
            }],
            evaluation_tick: 100,
            maximum_monitoring_age_ticks: 10,
            maximum_forecast_age_ticks: 10,
            minimum_proactive_lead_ticks: 15,
            policy,
            hard_gate: HardGate::Clear,
            monitoring: None,
            forecast: None,
        }
    }

    fn monitoring() -> MonitoringEvidence {
        MonitoringEvidence {
            evidence_id: "monitor-1".into(),
            binding: binding(),
            observed_at_tick: 98,
            covered_interval: TimeInterval {
                start_tick: 110,
                end_tick: 170,
            },
            calibrated: true,
            spatial_coverage_complete: true,
            maximum_exposure: SyntheticExposure {
                unit_id: "synthetic-exposure-u1".into(),
                value_microunits: 80,
            },
        }
    }

    fn forecast() -> ForecastEvidence {
        ForecastEvidence {
            evidence_id: "forecast-1".into(),
            binding: binding(),
            issued_at_tick: 99,
            valid_interval: TimeInterval {
                start_tick: 110,
                end_tick: 170,
            },
            maximum_exposure: SyntheticExposure {
                unit_id: "synthetic-exposure-u1".into(),
                value_microunits: 70,
            },
        }
    }

    #[test]
    fn fixed_policy_accepts_interval_inside_declared_window() {
        assert_eq!(
            evaluate_window(&case(WindowPolicy::Fixed), &model()).disposition,
            WindowDisposition::EligibleForSyntheticWindow
        );
    }

    #[test]
    fn fixed_policy_blocks_interval_outside_declared_window() {
        let mut c = case(WindowPolicy::Fixed);
        c.planned_interval = TimeInterval {
            start_tick: 190,
            end_tick: 210,
        };
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn reactive_policy_requires_monitoring_evidence() {
        let r = evaluate_window(&case(WindowPolicy::Reactive), &model());
        assert_eq!(r.disposition, WindowDisposition::UnknownReviewRequired);
        assert!(r.reasons.contains(&WindowReason::MissingMonitoring));
    }

    #[test]
    fn reactive_policy_accepts_fresh_calibrated_complete_monitoring() {
        let mut c = case(WindowPolicy::Reactive);
        c.monitoring = Some(monitoring());
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EligibleForSyntheticWindow
        );
    }

    #[test]
    fn reactive_policy_blocks_at_threshold_equality() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.maximum_exposure.value_microunits = 100;
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn reactive_policy_blocks_above_threshold() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.maximum_exposure.value_microunits = 101;
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn stale_monitoring_requires_review() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.observed_at_tick = 80;
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn future_monitoring_is_inconsistent() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.observed_at_tick = 101;
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn uncalibrated_or_incomplete_monitoring_never_opens_window() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.calibrated = false;
        e.spatial_coverage_complete = false;
        c.monitoring = Some(e);
        let r = evaluate_window(&c, &model());
        assert_eq!(r.disposition, WindowDisposition::UnknownReviewRequired);
        assert!(r.reasons.contains(&WindowReason::MonitoringNotCalibrated));
        assert!(r.reasons.contains(&WindowReason::MonitoringCoverageIncomplete));
    }

    #[test]
    fn monitoring_session_must_cover_whole_planned_interval() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.covered_interval.end_tick = 150;
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn monitoring_bound_to_another_site_is_inconsistent() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.binding.scope.site_revision = "geo-11".into();
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn monitoring_unit_mismatch_is_inconsistent() {
        let mut c = case(WindowPolicy::Reactive);
        let mut e = monitoring();
        e.maximum_exposure.unit_id = "other-unit".into();
        c.monitoring = Some(e);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn proactive_policy_requires_forecast_and_lead_time() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        let r = evaluate_window(&c, &model());
        assert_eq!(r.disposition, WindowDisposition::UnknownReviewRequired);
        assert!(r.reasons.contains(&WindowReason::MissingForecast));
    }

    #[test]
    fn proactive_policy_accepts_matching_low_forecast() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        c.forecast = Some(forecast());
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EligibleForSyntheticWindow
        );
    }

    #[test]
    fn proactive_policy_blocks_forecast_threshold_crossing() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        let mut f = forecast();
        f.maximum_exposure.value_microunits = 100;
        c.forecast = Some(f);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn proactive_forecast_must_cover_the_entire_work_window() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        let mut f = forecast();
        f.valid_interval.end_tick = 150;
        c.forecast = Some(f);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn proactive_policy_requires_intervention_lead_time() {
        let mut c = case(WindowPolicy::Proactive);
        c.planned_interval.start_tick = 105;
        c.monitoring = Some(monitoring());
        c.forecast = Some(forecast());
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn future_forecast_is_inconsistent() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        let mut f = forecast();
        f.issued_at_tick = 101;
        c.forecast = Some(f);
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn unknown_hard_gate_cannot_be_weighted_away() {
        let mut c = case(WindowPolicy::Fixed);
        c.hard_gate = HardGate::Unknown {
            gate_id: "rights-review".into(),
        };
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::UnknownReviewRequired
        );
    }

    #[test]
    fn blocked_hard_gate_cannot_be_weighted_away() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        c.forecast = Some(forecast());
        c.hard_gate = HardGate::Blocked {
            gate_id: "authority-prohibition".into(),
        };
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::BlockedByHardConstraint
        );
    }

    #[test]
    fn model_outside_declared_context_is_unsupported() {
        let mut c = case(WindowPolicy::Fixed);
        c.scope.context_id = "arid-mangrove".into();
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::Unsupported
        );
    }

    #[test]
    fn invalid_intervals_are_rejected_as_inconsistent() {
        let mut c = case(WindowPolicy::Fixed);
        c.planned_interval.end_tick = c.planned_interval.start_tick;
        assert_eq!(
            evaluate_window(&c, &model()).disposition,
            WindowDisposition::EvidenceInconsistent
        );
    }

    #[test]
    fn identical_inputs_have_identical_canonical_decisions() {
        let mut c = case(WindowPolicy::Proactive);
        c.monitoring = Some(monitoring());
        c.forecast = Some(forecast());
        assert_eq!(evaluate_window(&c, &model()), evaluate_window(&c, &model()));
    }
}
