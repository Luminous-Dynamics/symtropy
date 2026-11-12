// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Distinguishes the information set available at a decision cutoff from
//! records that arrive later and improve only the retrospective archive.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

pub const PERIODS: [u32; 4] = [0, 1, 2, 3];
pub const SUPPORT_MIN: u16 = 0;
pub const SUPPORT_MAX: u16 = 100;
pub const DECISION_CUTOFF_TICK: u32 = 4;
pub const RETROSPECTIVE_AS_OF_TICK: u32 = 5;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingCause {
    PolicySuppressed,
    TelemetryUnavailable,
    CommonCauseOutage,
}

impl MissingCause {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::PolicySuppressed => "policy_suppressed",
            Self::TelemetryUnavailable => "telemetry_unavailable",
            Self::CommonCauseOutage => "common_cause_outage",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LatentOutcome {
    pub policy_id: String,
    pub world_id: String,
    pub event_tick: u32,
    pub true_service_units: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    pub policy_id: String,
    pub world_id: String,
    pub event_tick: u32,
    /// The captured payload, if any. A late payload remains absent from the
    /// decision-time information set until its declared ingest tick.
    pub observed_service_units: Option<u16>,
    pub ingest_tick: Option<u32>,
    pub missing_cause: Option<MissingCause>,
    pub capture_receipt_digest: String,
    pub delivery_receipt_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalObservabilityConfig {
    pub scenario_id: String,
    pub source_digest: String,
    pub service_unit: String,
    pub event_time_unit: String,
    pub decision_cutoff_tick: u32,
    pub retrospective_as_of_tick: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceView {
    DecisionTime,
    Retrospective,
}

impl EvidenceView {
    pub const fn id(self) -> &'static str {
        match self {
            Self::DecisionTime => "decision_time",
            Self::Retrospective => "retrospective",
        }
    }

    const fn as_of(self, config: &TemporalObservabilityConfig) -> u32 {
        match self {
            Self::DecisionTime => config.decision_cutoff_tick,
            Self::Retrospective => config.retrospective_as_of_tick,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobustComparison {
    SuppressedPolicyStrictlyBetter,
    TransparentPolicyStrictlyBetter,
    Unresolved,
}

impl RobustComparison {
    pub const fn id(self) -> &'static str {
        match self {
            Self::SuppressedPolicyStrictlyBetter => "suppressed_strictly_better",
            Self::TransparentPolicyStrictlyBetter => "transparent_strictly_better",
            Self::Unresolved => "unresolved",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentifiedInterval {
    pub lower_numerator: u32,
    pub upper_numerator: u32,
    pub denominator: u32,
}

impl IdentifiedInterval {
    pub fn lower_mean(&self) -> f64 {
        f64::from(self.lower_numerator) / f64::from(self.denominator)
    }

    pub fn upper_mean(&self) -> f64 {
        f64::from(self.upper_numerator) / f64::from(self.denominator)
    }

    pub fn contains(&self, value: f64) -> bool {
        value + 1.0e-9 >= self.lower_mean() && value <= self.upper_mean() + 1.0e-9
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyWorldViewSummary {
    pub scenario_id: String,
    pub policy_id: String,
    pub world_id: String,
    pub view: EvidenceView,
    pub true_mean_service_units: f64,
    pub complete_case_mean_service_units: Option<f64>,
    pub missing_as_zero_mean_service_units: f64,
    pub identified_interval: IdentifiedInterval,
    pub observed_periods: usize,
    pub total_periods: usize,
    pub coverage: f64,
    pub missing_periods: usize,
    pub late_not_yet_available_periods: usize,
    pub observability_debt_service_periods: usize,
    pub truth_contained: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateDecisionObservation {
    pub scenario_id: String,
    pub policy_id: String,
    pub event_tick: u32,
    pub decision_status: String,
    pub observed_service_units: Option<u16>,
    pub decision_cutoff_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvaluatorEventReceipt {
    pub policy_id: String,
    pub world_id: String,
    pub event_tick: u32,
    pub decision_status: String,
    pub retrospective_status: String,
    pub ingest_tick: Option<u32>,
    pub missing_cause: Option<MissingCause>,
    pub capture_receipt_digest: String,
    pub delivery_receipt_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemporalObservabilityReport {
    pub scenario_id: String,
    pub source_digest: String,
    pub summaries: Vec<PolicyWorldViewSummary>,
    pub candidate_observations: Vec<CandidateDecisionObservation>,
    pub evaluator_event_receipts: Vec<EvaluatorEventReceipt>,
    pub decision_time_comparison_by_world: BTreeMap<String, RobustComparison>,
    pub retrospective_comparison_by_world: BTreeMap<String, RobustComparison>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditError {
    InvalidConfig,
    EmptyOutcomeSet,
    EmptyObservationSet,
    DuplicateOutcome,
    DuplicateObservation,
    InvalidOutcome,
    InvalidObservationIdentity,
    InvalidObservedValue,
    InvalidDeliveryTime,
    InvalidMissingState,
    MissingObservationRow,
    UnexpectedObservationRow,
    IncompletePeriodMatrix,
    InconsistentPolicyMatrix,
    ObservationTruthMismatch,
    UnexpectedScenarioIdentity,
}

impl std::fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AuditError {}

fn mean(values: &[u16]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().map(|value| f64::from(*value)).sum::<f64>() / values.len() as f64)
    }
}

fn visible_value(record: &ObservationRecord, as_of_tick: u32) -> Option<u16> {
    match (record.observed_service_units, record.ingest_tick) {
        (Some(value), Some(ingest)) if ingest <= as_of_tick => Some(value),
        _ => None,
    }
}

fn robust_comparison(
    suppressed: &IdentifiedInterval,
    transparent: &IdentifiedInterval,
) -> RobustComparison {
    if suppressed.lower_mean() > transparent.upper_mean() {
        RobustComparison::SuppressedPolicyStrictlyBetter
    } else if transparent.lower_mean() > suppressed.upper_mean() {
        RobustComparison::TransparentPolicyStrictlyBetter
    } else {
        RobustComparison::Unresolved
    }
}

fn validate_config(config: &TemporalObservabilityConfig) -> Result<(), AuditError> {
    if config.scenario_id.trim().is_empty()
        || config.source_digest.trim().is_empty()
        || config.service_unit.trim().is_empty()
        || config.event_time_unit.trim().is_empty()
        || config.decision_cutoff_tick <= PERIODS[PERIODS.len() - 1]
        || config.retrospective_as_of_tick < config.decision_cutoff_tick
    {
        return Err(AuditError::InvalidConfig);
    }
    Ok(())
}

fn validate_inputs(
    config: &TemporalObservabilityConfig,
    outcomes: &[LatentOutcome],
    observations: &[ObservationRecord],
) -> Result<(), AuditError> {
    validate_config(config)?;
    if outcomes.is_empty() {
        return Err(AuditError::EmptyOutcomeSet);
    }
    if observations.is_empty() {
        return Err(AuditError::EmptyObservationSet);
    }

    let mut outcome_map = BTreeMap::new();
    let mut policy_periods: BTreeMap<(&str, &str), BTreeSet<u32>> = BTreeMap::new();
    for outcome in outcomes {
        if outcome.policy_id.trim().is_empty()
            || outcome.world_id.trim().is_empty()
            || !PERIODS.contains(&outcome.event_tick)
            || outcome.true_service_units > SUPPORT_MAX
        {
            return Err(AuditError::InvalidOutcome);
        }
        let key = (
            outcome.policy_id.as_str(),
            outcome.world_id.as_str(),
            outcome.event_tick,
        );
        if outcome_map.insert(key, outcome).is_some() {
            return Err(AuditError::DuplicateOutcome);
        }
        policy_periods
            .entry((outcome.policy_id.as_str(), outcome.world_id.as_str()))
            .or_default()
            .insert(outcome.event_tick);
    }

    if policy_periods
        .values()
        .any(|periods| periods.len() != PERIODS.len() || !PERIODS.iter().all(|p| periods.contains(p)))
    {
        return Err(AuditError::IncompletePeriodMatrix);
    }

    let policies: BTreeSet<&str> = outcomes.iter().map(|row| row.policy_id.as_str()).collect();
    let worlds: BTreeSet<&str> = outcomes.iter().map(|row| row.world_id.as_str()).collect();
    if policies != BTreeSet::from(["transparent_audit", "defer_suppress"])
        || worlds != BTreeSet::from(["moderate_decay", "severe_decay"])
    {
        return Err(AuditError::UnexpectedScenarioIdentity);
    }
    for policy in &policies {
        for world in &worlds {
            if !policy_periods.contains_key(&(*policy, *world)) {
                return Err(AuditError::InconsistentPolicyMatrix);
            }
        }
    }

    let mut observation_map = BTreeMap::new();
    for record in observations {
        if record.policy_id.trim().is_empty()
            || record.world_id.trim().is_empty()
            || !PERIODS.contains(&record.event_tick)
            || record.capture_receipt_digest.trim().is_empty()
        {
            return Err(AuditError::InvalidObservationIdentity);
        }

        match (record.observed_service_units, record.ingest_tick, &record.missing_cause, &record.delivery_receipt_digest) {
            (Some(value), Some(ingest), None, Some(delivery))
                if value <= SUPPORT_MAX
                    && ingest >= record.event_tick
                    && ingest <= config.retrospective_as_of_tick
                    && !delivery.trim().is_empty() => {}
            (None, None, Some(_), None) => {}
            (Some(_), Some(_), _, _) => return Err(AuditError::InvalidMissingState),
            (None, None, None, _) => return Err(AuditError::InvalidMissingState),
            (Some(value), _, _, _) if value > SUPPORT_MAX => return Err(AuditError::InvalidObservedValue),
            _ => return Err(AuditError::InvalidDeliveryTime),
        }

        let key = (
            record.policy_id.as_str(),
            record.world_id.as_str(),
            record.event_tick,
        );
        if observation_map.insert(key, record).is_some() {
            return Err(AuditError::DuplicateObservation);
        }
    }

    if outcome_map.len() != observation_map.len() {
        return Err(AuditError::MissingObservationRow);
    }
    for (key, outcome) in &outcome_map {
        let record = observation_map.get(key).ok_or(AuditError::MissingObservationRow)?;
        if let Some(observed) = record.observed_service_units {
            if observed != outcome.true_service_units {
                return Err(AuditError::ObservationTruthMismatch);
            }
        }
    }
    for key in observation_map.keys() {
        if !outcome_map.contains_key(key) {
            return Err(AuditError::UnexpectedObservationRow);
        }
    }
    Ok(())
}

fn summarize_view(
    config: &TemporalObservabilityConfig,
    policy_id: &str,
    world_id: &str,
    outcomes: &[LatentOutcome],
    observations: &[ObservationRecord],
    view: EvidenceView,
) -> PolicyWorldViewSummary {
    let as_of = view.as_of(config);
    let rows: Vec<&LatentOutcome> = outcomes
        .iter()
        .filter(|row| row.policy_id == policy_id && row.world_id == world_id)
        .collect();
    let events: Vec<&ObservationRecord> = observations
        .iter()
        .filter(|row| row.policy_id == policy_id && row.world_id == world_id)
        .collect();

    let truth_values: Vec<u16> = rows.iter().map(|row| row.true_service_units).collect();
    let true_mean = mean(&truth_values).expect("validated four-period world");
    let observed_values: Vec<u16> = events
        .iter()
        .filter_map(|event| visible_value(event, as_of))
        .collect();
    let observed_sum: u32 = observed_values.iter().map(|value| u32::from(*value)).sum();
    let observed_periods = observed_values.len();
    let missing_periods = PERIODS.len() - observed_periods;
    let interval = IdentifiedInterval {
        lower_numerator: observed_sum,
        upper_numerator: observed_sum + missing_periods as u32 * u32::from(SUPPORT_MAX),
        denominator: PERIODS.len() as u32,
    };
    let complete_case_mean = mean(&observed_values);
    let missing_as_zero_mean = f64::from(observed_sum) / PERIODS.len() as f64;
    let late_not_yet_available_periods = events
        .iter()
        .filter(|event| {
            event.ingest_tick.is_some_and(|ingest| ingest > config.decision_cutoff_tick)
                && view == EvidenceView::DecisionTime
        })
        .count();
    let observability_debt_service_periods = if view == EvidenceView::DecisionTime {
        missing_periods
    } else {
        events.iter().filter(|event| event.observed_service_units.is_none()).count()
    };

    PolicyWorldViewSummary {
        scenario_id: config.scenario_id.clone(),
        policy_id: policy_id.to_owned(),
        world_id: world_id.to_owned(),
        view,
        true_mean_service_units: true_mean,
        complete_case_mean_service_units: complete_case_mean,
        missing_as_zero_mean_service_units: missing_as_zero_mean,
        identified_interval: interval.clone(),
        observed_periods,
        total_periods: PERIODS.len(),
        coverage: observed_periods as f64 / PERIODS.len() as f64,
        missing_periods,
        late_not_yet_available_periods,
        observability_debt_service_periods,
        truth_contained: interval.contains(true_mean),
    }
}

fn build_candidate_observations(
    config: &TemporalObservabilityConfig,
    policies: &BTreeSet<&str>,
    worlds: &BTreeSet<&str>,
    observations: &[ObservationRecord],
) -> Result<Vec<CandidateDecisionObservation>, AuditError> {
    let mut result = Vec::new();
    for policy in policies {
        for tick in PERIODS {
            let mut public_states = BTreeSet::new();
            for world in worlds {
                let record = observations
                    .iter()
                    .find(|row| row.policy_id == *policy && row.world_id == *world && row.event_tick == tick)
                    .ok_or(AuditError::MissingObservationRow)?;
                let value = visible_value(record, config.decision_cutoff_tick);
                let state = (
                    if value.is_some() { "observed" } else { "missing" },
                    value,
                );
                public_states.insert(state);
            }
            for (status, value) in public_states {
                result.push(CandidateDecisionObservation {
                    scenario_id: config.scenario_id.clone(),
                    policy_id: (*policy).to_owned(),
                    event_tick: tick,
                    decision_status: status.to_owned(),
                    observed_service_units: value,
                    decision_cutoff_tick: config.decision_cutoff_tick,
                });
            }
        }
    }
    result.sort_by(|left, right| {
        left.policy_id.cmp(&right.policy_id).then_with(|| left.event_tick.cmp(&right.event_tick))
    });
    Ok(result)
}

fn build_evaluator_receipts(
    config: &TemporalObservabilityConfig,
    observations: &[ObservationRecord],
) -> Vec<EvaluatorEventReceipt> {
    let mut receipts = Vec::new();
    for record in observations {
        let decision_visible = visible_value(record, config.decision_cutoff_tick).is_some();
        let retrospective_visible = visible_value(record, config.retrospective_as_of_tick).is_some();
        receipts.push(EvaluatorEventReceipt {
            policy_id: record.policy_id.clone(),
            world_id: record.world_id.clone(),
            event_tick: record.event_tick,
            decision_status: if decision_visible { "observed" } else { "missing" }.to_owned(),
            retrospective_status: if retrospective_visible { "observed" } else { "missing" }.to_owned(),
            ingest_tick: record.ingest_tick,
            missing_cause: record.missing_cause.clone(),
            capture_receipt_digest: record.capture_receipt_digest.clone(),
            delivery_receipt_digest: record.delivery_receipt_digest.clone(),
        });
    }
    receipts.sort_by(|left, right| {
        left.policy_id.cmp(&right.policy_id)
            .then_with(|| left.world_id.cmp(&right.world_id))
            .then_with(|| left.event_tick.cmp(&right.event_tick))
    });
    receipts
}

pub fn evaluate_temporal_observability(
    config: &TemporalObservabilityConfig,
    outcomes: &[LatentOutcome],
    observations: &[ObservationRecord],
) -> Result<TemporalObservabilityReport, AuditError> {
    validate_inputs(config, outcomes, observations)?;

    let policies: BTreeSet<&str> = outcomes.iter().map(|row| row.policy_id.as_str()).collect();
    let worlds: BTreeSet<&str> = outcomes.iter().map(|row| row.world_id.as_str()).collect();

    let mut summaries = Vec::new();
    for policy in &policies {
        for world in &worlds {
            summaries.push(summarize_view(config, policy, world, outcomes, observations, EvidenceView::DecisionTime));
            summaries.push(summarize_view(config, policy, world, outcomes, observations, EvidenceView::Retrospective));
        }
    }
    summaries.sort_by(|left, right| {
        left.policy_id.cmp(&right.policy_id)
            .then_with(|| left.world_id.cmp(&right.world_id))
            .then_with(|| left.view.id().cmp(right.view.id()))
    });

    let mut decision_time_comparison_by_world = BTreeMap::new();
    let mut retrospective_comparison_by_world = BTreeMap::new();
    for world in &worlds {
        let decision_transparent = summaries.iter().find(|row| {
            row.policy_id == "transparent_audit" && row.world_id == *world && row.view == EvidenceView::DecisionTime
        }).expect("fixture policy IDs validated");
        let decision_defer = summaries.iter().find(|row| {
            row.policy_id == "defer_suppress" && row.world_id == *world && row.view == EvidenceView::DecisionTime
        }).expect("fixture policy IDs validated");
        decision_time_comparison_by_world.insert(
            (*world).to_owned(),
            robust_comparison(&decision_defer.identified_interval, &decision_transparent.identified_interval),
        );

        let retro_transparent = summaries.iter().find(|row| {
            row.policy_id == "transparent_audit" && row.world_id == *world && row.view == EvidenceView::Retrospective
        }).expect("fixture policy IDs validated");
        let retro_defer = summaries.iter().find(|row| {
            row.policy_id == "defer_suppress" && row.world_id == *world && row.view == EvidenceView::Retrospective
        }).expect("fixture policy IDs validated");
        retrospective_comparison_by_world.insert(
            (*world).to_owned(),
            robust_comparison(&retro_defer.identified_interval, &retro_transparent.identified_interval),
        );
    }

    Ok(TemporalObservabilityReport {
        scenario_id: config.scenario_id.clone(),
        source_digest: config.source_digest.clone(),
        summaries,
        candidate_observations: build_candidate_observations(config, &policies, &worlds, observations)?,
        evaluator_event_receipts: build_evaluator_receipts(config, observations),
        decision_time_comparison_by_world,
        retrospective_comparison_by_world,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TemporalObservabilityConfig {
        TemporalObservabilityConfig {
            scenario_id: "temporal-observability-four-period-v1".to_owned(),
            source_digest: "sha256-temporal-observability-fixture".to_owned(),
            service_unit: "service_units".to_owned(),
            event_time_unit: "logical_ticks".to_owned(),
            decision_cutoff_tick: DECISION_CUTOFF_TICK,
            retrospective_as_of_tick: RETROSPECTIVE_AS_OF_TICK,
        }
    }

    fn outcomes() -> Vec<LatentOutcome> {
        let mut rows = Vec::new();
        for world in ["moderate_decay", "severe_decay"] {
            for (tick, value) in PERIODS.into_iter().zip([60, 60, 60, 60]) {
                rows.push(LatentOutcome {
                    policy_id: "transparent_audit".to_owned(),
                    world_id: world.to_owned(),
                    event_tick: tick,
                    true_service_units: value,
                });
            }
        }
        for (world, values) in [
            ("moderate_decay", [100, 20, 20, 50]),
            ("severe_decay", [100, 0, 0, 50]),
        ] {
            for (tick, value) in PERIODS.into_iter().zip(values) {
                rows.push(LatentOutcome {
                    policy_id: "defer_suppress".to_owned(),
                    world_id: world.to_owned(),
                    event_tick: tick,
                    true_service_units: value,
                });
            }
        }
        rows
    }

    fn observation(
        policy_id: &str,
        world_id: &str,
        tick: u32,
        value: Option<u16>,
        ingest_tick: Option<u32>,
        cause: Option<MissingCause>,
    ) -> ObservationRecord {
        ObservationRecord {
            policy_id: policy_id.to_owned(),
            world_id: world_id.to_owned(),
            event_tick: tick,
            observed_service_units: value,
            ingest_tick,
            missing_cause: cause,
            capture_receipt_digest: format!("sha256-capture-{policy_id}-{world_id}-{tick}"),
            delivery_receipt_digest: value.map(|_| format!("sha256-delivery-{policy_id}-{world_id}-{tick}")),
        }
    }

    fn observations() -> Vec<ObservationRecord> {
        let mut rows = Vec::new();
        for world in ["moderate_decay", "severe_decay"] {
            for tick in PERIODS {
                rows.push(observation(
                    "transparent_audit", world, tick, Some(60), Some(tick), None
                ));
            }
        }
        for world in ["moderate_decay", "severe_decay"] {
            rows.push(observation("defer_suppress", world, 0, Some(100), Some(0), None));
            rows.push(observation(
                "defer_suppress", world, 1, None, None, Some(MissingCause::PolicySuppressed)
            ));
            rows.push(observation(
                "defer_suppress", world, 2, None, None, Some(MissingCause::PolicySuppressed)
            ));
            rows.push(observation("defer_suppress", world, 3, Some(50), Some(5), None));
        }
        rows
    }

    fn summary<'a>(
        report: &'a TemporalObservabilityReport,
        policy: &str,
        world: &str,
        view: EvidenceView,
    ) -> &'a PolicyWorldViewSummary {
        report.summaries.iter().find(|row| {
            row.policy_id == policy && row.world_id == world && row.view == view
        }).unwrap()
    }

    #[test]
    fn late_event_is_missing_at_decision_time_but_present_retrospectively() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let decision = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::DecisionTime);
        let retrospective = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::Retrospective);
        assert_eq!(decision.observed_periods, 1);
        assert_eq!(decision.late_not_yet_available_periods, 1);
        assert_eq!(decision.complete_case_mean_service_units, Some(100.0));
        assert_eq!(retrospective.observed_periods, 2);
        assert_eq!(retrospective.complete_case_mean_service_units, Some(75.0));
    }

    #[test]
    fn decision_time_and_retrospective_bounds_remain_distinct_and_truth_containing() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let decision = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::DecisionTime);
        let retrospective = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::Retrospective);
        assert_eq!(decision.identified_interval.lower_numerator, 100);
        assert_eq!(decision.identified_interval.upper_numerator, 400);
        assert_eq!(retrospective.identified_interval.lower_numerator, 150);
        assert_eq!(retrospective.identified_interval.upper_numerator, 350);
        assert!(decision.truth_contained);
        assert!(retrospective.truth_contained);
        assert_eq!(decision.observability_debt_service_periods, 3);
        assert_eq!(retrospective.observability_debt_service_periods, 2);
    }

    #[test]
    fn complete_case_can_reverse_the_true_policy_ranking_but_bounds_refuse_a_winner() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let transparent = summary(&report, "transparent_audit", "moderate_decay", EvidenceView::DecisionTime);
        let defer = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::DecisionTime);
        assert_eq!(transparent.true_mean_service_units, 60.0);
        assert_eq!(defer.true_mean_service_units, 47.5);
        assert!(defer.complete_case_mean_service_units.unwrap() > transparent.true_mean_service_units);
        assert_eq!(
            report.decision_time_comparison_by_world["moderate_decay"],
            RobustComparison::Unresolved
        );
        assert_eq!(
            report.retrospective_comparison_by_world["moderate_decay"],
            RobustComparison::Unresolved
        );
    }

    #[test]
    fn severe_world_remains_in_denominator_and_has_lower_true_service() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let severe = summary(&report, "defer_suppress", "severe_decay", EvidenceView::DecisionTime);
        assert_eq!(severe.true_mean_service_units, 37.5);
        assert_eq!(severe.complete_case_mean_service_units, Some(100.0));
        assert_eq!(severe.observed_periods, 1);
    }

    #[test]
    fn candidate_projection_has_no_world_cause_or_future_ingest_fields() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        assert_eq!(report.candidate_observations.len(), 8);
        let late_tick = report.candidate_observations.iter().find(|row| {
            row.policy_id == "defer_suppress" && row.event_tick == 3
        }).unwrap();
        assert_eq!(late_tick.decision_status, "missing");
        assert_eq!(late_tick.observed_service_units, None);
    }

    #[test]
    fn transparent_policy_has_complete_timely_evidence() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let row = summary(&report, "transparent_audit", "moderate_decay", EvidenceView::DecisionTime);
        assert_eq!(row.true_mean_service_units, 60.0);
        assert_eq!(row.complete_case_mean_service_units, Some(60.0));
        assert_eq!(row.coverage, 1.0);
        assert_eq!(row.observability_debt_service_periods, 0);
    }

    #[test]
    fn missing_as_zero_does_not_hide_the_denominator() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let decision = summary(&report, "defer_suppress", "moderate_decay", EvidenceView::DecisionTime);
        assert_eq!(decision.missing_as_zero_mean_service_units, 25.0);
        assert_eq!(decision.missing_periods, 3);
    }

    #[test]
    fn duplicate_outcome_or_observation_fails_closed() {
        let mut rows = outcomes();
        rows.push(rows[0].clone());
        assert_eq!(
            evaluate_temporal_observability(&config(), &rows, &observations()),
            Err(AuditError::DuplicateOutcome)
        );

        let mut events = observations();
        events.push(events[0].clone());
        assert_eq!(
            evaluate_temporal_observability(&config(), &outcomes(), &events),
            Err(AuditError::DuplicateObservation)
        );
    }

    #[test]
    fn event_cannot_arrive_before_capture_or_after_retrospective_cutoff() {
        let mut events = observations();
        let late = events.iter_mut().find(|row| {
            row.policy_id == "defer_suppress" && row.world_id == "moderate_decay" && row.event_tick == 3
        }).unwrap();
        late.ingest_tick = Some(2);
        assert!(evaluate_temporal_observability(&config(), &outcomes(), &events).is_err());

        let mut events = observations();
        let late = events.iter_mut().find(|row| {
            row.policy_id == "defer_suppress" && row.world_id == "moderate_decay" && row.event_tick == 3
        }).unwrap();
        late.ingest_tick = Some(6);
        assert!(evaluate_temporal_observability(&config(), &outcomes(), &events).is_err());
    }

    #[test]
    fn observation_must_match_evaluator_truth() {
        let mut events = observations();
        events[0].observed_service_units = Some(59);
        assert_eq!(
            evaluate_temporal_observability(&config(), &outcomes(), &events),
            Err(AuditError::ObservationTruthMismatch)
        );
    }

    #[test]
    fn malformed_missing_state_or_payload_without_receipt_fails_closed() {
        let mut events = observations();
        events[0].missing_cause = Some(MissingCause::PolicySuppressed);
        assert!(evaluate_temporal_observability(&config(), &outcomes(), &events).is_err());

        let mut events = observations();
        events[0].delivery_receipt_digest = None;
        assert!(evaluate_temporal_observability(&config(), &outcomes(), &events).is_err());
    }

    #[test]
    fn robust_bounds_use_exact_integer_numerators() {
        let report = evaluate_temporal_observability(&config(), &outcomes(), &observations()).unwrap();
        let decision = summary(&report, "defer_suppress", "severe_decay", EvidenceView::DecisionTime);
        assert_eq!(decision.identified_interval.lower_numerator, 100);
        assert_eq!(decision.identified_interval.upper_numerator, 400);
        assert_eq!(decision.identified_interval.denominator, 4);
    }

    #[test]
    fn output_order_is_independent_of_input_permutation() {
        let config = config();
        let mut a = outcomes();
        let mut b = observations();
        let first = evaluate_temporal_observability(&config, &a, &b).unwrap();
        a.reverse();
        b.reverse();
        let second = evaluate_temporal_observability(&config, &a, &b).unwrap();
        assert_eq!(first.summaries, second.summaries);
        assert_eq!(first.candidate_observations, second.candidate_observations);
        assert_eq!(first.evaluator_event_receipts, second.evaluator_event_receipts);
    }
}
