// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Synthetic decision-quality benchmark for stale and conflicting port-call evidence.
//!
//! Policy functions receive EvidenceDecisionInput only. Hidden synthetic arrival
//! truth is passed separately to evaluate_decision, after the action is produced.
//! This module does not mutate a real port schedule or activate a booking.

use super::{
    resolve_observations, validate_observation, NANOS_PER_SECOND, ObservationReport,
    ObservedValue, PcoField, PcoObservation, TimeMeaning, ValidationPolicy,
};
use std::collections::BTreeSet;

pub const NANOS_PER_MINUTE: i128 = 60 * NANOS_PER_SECOND;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct DecisionWindow {
    pub id: String,
    pub starts_at_utc_nanos: i128,
    pub late_grace_minutes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDecisionInput {
    pub validation_policy: ValidationPolicy,
    /// Frozen, externally declared candidate windows. No real reservations are made.
    pub windows: Vec<DecisionWindow>,
    /// Penalty used by the synthetic evaluator when a service window is missed.
    pub missed_window_penalty_minutes: u64,
    /// Explicit service cost for abstaining, so doing nothing is not free.
    pub abstention_cost_minutes: u64,
    /// Observations are the only evidence available to decision policies.
    pub observations: Vec<PcoObservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluationTruth {
    /// Hidden from decide; visible only to the post-decision evaluator.
    pub actual_arrival_utc_nanos: i128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidencePolicy {
    LatestIngestedPointEstimate,
    FreshestValidatedSourcePointEstimate,
    FailClosedUncontestedOnly,
    BoundedWorstCaseContingency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecisionMode {
    PointEstimate,
    UncontestedEstimate,
    RobustAgainstCandidateSet,
    Abstain,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceDecision {
    pub policy: EvidencePolicy,
    pub mode: DecisionMode,
    pub chosen_window_id: Option<String>,
    pub basis_observation_ids: Vec<String>,
    pub rejected_observation_count: usize,
    pub conflicting_group_count: usize,
}

impl EvidenceDecision {
    pub fn abstained(&self) -> bool {
        self.chosen_window_id.is_none()
    }

    fn has_evidence_quality_issue(&self) -> bool {
        self.rejected_observation_count > 0 || self.conflicting_group_count > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvaluatedDecision {
    pub decision: EvidenceDecision,
    pub service_cost_minutes: u64,
    pub oracle_cost_minutes: u64,
    pub regret_minutes: u64,
    pub served: bool,
    /// A point estimate acted while relevant inputs were rejected or conflicted.
    /// This is a benchmark label, not a claim of intent.
    pub actionable_despite_evidence_issue: bool,
}

fn planned_arrival_observations(input: &EvidenceDecisionInput) -> Vec<PcoObservation> {
    input
        .observations
        .iter()
        .filter(|observation| {
            observation.field == PcoField::ArrivalAtPilotBoardingPlace
                && matches!(
                    &observation.value,
                    ObservedValue::Timestamp {
                        meaning: TimeMeaning::Planned,
                        ..
                    }
                )
        })
        .cloned()
        .collect()
}

fn target_report(
    input: &EvidenceDecisionInput,
    arrivals: &[PcoObservation],
) -> ObservationReport {
    resolve_observations(arrivals, &input.validation_policy)
}

fn planned_instant(observation: &PcoObservation) -> Option<i128> {
    match &observation.value {
        ObservedValue::Timestamp {
            meaning: TimeMeaning::Planned,
            value,
        } if observation.field == PcoField::ArrivalAtPilotBoardingPlace => {
            Some(value.utc_unix_nanos())
        }
        _ => None,
    }
}

fn observation_order_key(
    observation: &PcoObservation,
    by_ingestion: bool,
) -> (i128, i128, String, String) {
    let primary = if by_ingestion {
        observation.ingested_at_utc_nanos
    } else {
        observation.source_event_time_utc_nanos
    };
    (
        primary,
        observation.ingested_at_utc_nanos,
        observation.source_id.clone(),
        observation.observation_id.clone(),
    )
}

fn report_counts(report: &ObservationReport) -> (usize, usize) {
    (
        report.rejected.len(),
        report
            .resolutions
            .iter()
            .filter(|resolution| resolution.is_conflict())
            .count(),
    )
}

fn decision(
    policy: EvidencePolicy,
    mode: DecisionMode,
    chosen_window_id: Option<String>,
    mut basis_observation_ids: Vec<String>,
    report: &ObservationReport,
) -> EvidenceDecision {
    let (rejected_observation_count, conflicting_group_count) = report_counts(report);
    basis_observation_ids.sort();
    EvidenceDecision {
        policy,
        mode,
        chosen_window_id,
        basis_observation_ids,
        rejected_observation_count,
        conflicting_group_count,
    }
}

fn choose_window_for_estimate(
    instant: i128,
    input: &EvidenceDecisionInput,
) -> Option<String> {
    input
        .windows
        .iter()
        .min_by(|left, right| {
            score_window(instant, left, input.missed_window_penalty_minutes)
                .cmp(&score_window(
                    instant,
                    right,
                    input.missed_window_penalty_minutes,
                ))
                .then(left.starts_at_utc_nanos.cmp(&right.starts_at_utc_nanos))
                .then(left.id.cmp(&right.id))
        })
        .map(|window| window.id.clone())
}

/// Make a decision from observations and candidate windows only. The evaluator's
/// hidden actual-arrival value is not present in this input type.
///
/// LatestIngestedPointEstimate is deliberately naive. FreshestValidatedSourcePointEstimate
/// validates individual records but still picks a point from conflicting claims.
/// FailClosedUncontestedOnly abstains on rejected or conflicting relevant evidence.
/// BoundedWorstCaseContingency minimizes worst predicted cost over all admissible
/// planned arrival times for the same location context.
pub fn decide(
    policy: EvidencePolicy,
    input: &EvidenceDecisionInput,
) -> EvidenceDecision {
    let arrivals = planned_arrival_observations(input);
    let report = target_report(input, &arrivals);
    if input.windows.is_empty() {
        return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
    }

    match policy {
        EvidencePolicy::LatestIngestedPointEstimate => {
            let selected = arrivals
                .iter()
                .filter(|observation| planned_instant(observation).is_some())
                .max_by_key(|observation| observation_order_key(observation, true));
            match selected {
                Some(observation) => decision(
                    policy,
                    DecisionMode::PointEstimate,
                    choose_window_for_estimate(planned_instant(observation).unwrap(), input),
                    vec![observation.observation_id.clone()],
                    &report,
                ),
                None => decision(policy, DecisionMode::Abstain, None, Vec::new(), &report),
            }
        }
        EvidencePolicy::FreshestValidatedSourcePointEstimate => {
            let selected = report
                .accepted
                .iter()
                .filter(|observation| {
                    validate_observation(observation, &input.validation_policy).is_ok()
                })
                .filter(|observation| planned_instant(observation).is_some())
                .max_by_key(|observation| observation_order_key(observation, false));
            match selected {
                Some(observation) => decision(
                    policy,
                    DecisionMode::PointEstimate,
                    choose_window_for_estimate(planned_instant(observation).unwrap(), input),
                    vec![observation.observation_id.clone()],
                    &report,
                ),
                None => decision(policy, DecisionMode::Abstain, None, Vec::new(), &report),
            }
        }
        EvidencePolicy::FailClosedUncontestedOnly => {
            if !report.rejected.is_empty()
                || report.has_conflicts()
                || report.resolutions.len() != 1
            {
                return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
            }
            let resolution = &report.resolutions[0];
            let Some(ObservedValue::Timestamp {
                meaning: TimeMeaning::Planned,
                value,
            }) = resolution.agreed_value()
            else {
                return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
            };
            let ids = match resolution {
                super::ObservationResolution::Consistent { candidates, .. } => {
                    candidates
                        .iter()
                        .map(|item| item.observation_id.clone())
                        .collect()
                }
                super::ObservationResolution::Conflict { .. } => Vec::new(),
            };
            decision(
                policy,
                DecisionMode::UncontestedEstimate,
                choose_window_for_estimate(value.utc_unix_nanos(), input),
                ids,
                &report,
            )
        }
        EvidencePolicy::BoundedWorstCaseContingency => {
            if report.accepted.is_empty() {
                return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
            }
            // Different location labels are not merely timing uncertainty: one
            // berth/PBP action cannot robustly cover two physical locations.
            let locations = report
                .accepted
                .iter()
                .map(|observation| observation.location_context.clone())
                .collect::<BTreeSet<_>>();
            if locations.len() != 1 {
                return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
            }
            let instants = report
                .accepted
                .iter()
                .filter_map(planned_instant)
                .collect::<Vec<_>>();
            if instants.is_empty() {
                return decision(policy, DecisionMode::Abstain, None, Vec::new(), &report);
            }
            let chosen = input.windows.iter().min_by(|left, right| {
                let left_worst = instants
                    .iter()
                    .map(|instant| {
                        score_window(*instant, left, input.missed_window_penalty_minutes)
                    })
                    .max()
                    .unwrap_or(u64::MAX);
                let right_worst = instants
                    .iter()
                    .map(|instant| {
                        score_window(*instant, right, input.missed_window_penalty_minutes)
                    })
                    .max()
                    .unwrap_or(u64::MAX);
                left_worst
                    .cmp(&right_worst)
                    .then(left.starts_at_utc_nanos.cmp(&right.starts_at_utc_nanos))
                    .then(left.id.cmp(&right.id))
            });
            let ids = report
                .accepted
                .iter()
                .map(|observation| observation.observation_id.clone())
                .collect();
            decision(
                policy,
                DecisionMode::RobustAgainstCandidateSet,
                chosen.map(|window| window.id.clone()),
                ids,
                &report,
            )
        }
    }
}

fn ceil_minutes(nanos: i128) -> u64 {
    if nanos <= 0 {
        return 0;
    }
    let minutes = (nanos + NANOS_PER_MINUTE - 1) / NANOS_PER_MINUTE;
    u64::try_from(minutes).unwrap_or(u64::MAX)
}

/// Synthetic cost model: waiting is rounded up to minutes; lateness within the
/// grace window costs lateness minutes; a missed window adds a declared penalty.
pub fn score_window(
    actual_or_predicted_arrival_utc_nanos: i128,
    window: &DecisionWindow,
    missed_window_penalty_minutes: u64,
) -> u64 {
    if actual_or_predicted_arrival_utc_nanos <= window.starts_at_utc_nanos {
        return ceil_minutes(window.starts_at_utc_nanos - actual_or_predicted_arrival_utc_nanos);
    }
    let lateness =
        ceil_minutes(actual_or_predicted_arrival_utc_nanos - window.starts_at_utc_nanos);
    if lateness <= u64::from(window.late_grace_minutes) {
        lateness
    } else {
        missed_window_penalty_minutes.saturating_add(lateness)
    }
}

/// Evaluate one decision against hidden truth after the decision has been made.
pub fn evaluate_decision(
    input: &EvidenceDecisionInput,
    truth: &EvaluationTruth,
    decision: EvidenceDecision,
) -> EvaluatedDecision {
    let selected_window = decision
        .chosen_window_id
        .as_ref()
        .and_then(|id| input.windows.iter().find(|window| &window.id == id));
    let service_cost_minutes = selected_window
        .map(|window| {
            score_window(
                truth.actual_arrival_utc_nanos,
                window,
                input.missed_window_penalty_minutes,
            )
        })
        .unwrap_or(input.abstention_cost_minutes);
    let oracle_cost_minutes = input
        .windows
        .iter()
        .map(|window| {
            score_window(
                truth.actual_arrival_utc_nanos,
                window,
                input.missed_window_penalty_minutes,
            )
        })
        .min()
        .unwrap_or(input.abstention_cost_minutes);
    let served = selected_window.is_some_and(|window| {
        truth.actual_arrival_utc_nanos
            <= window.starts_at_utc_nanos
                + i128::from(window.late_grace_minutes) * NANOS_PER_MINUTE
    });
    let actionable_despite_evidence_issue = selected_window.is_some()
        && decision.has_evidence_quality_issue()
        && matches!(decision.mode, DecisionMode::PointEstimate);
    EvaluatedDecision {
        decision,
        service_cost_minutes,
        oracle_cost_minutes,
        regret_minutes: service_cost_minutes.saturating_sub(oracle_cost_minutes),
        served,
        actionable_despite_evidence_issue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(value: &str) -> i128 {
        super::super::parse_rfc3339_timestamp(value)
            .expect("frozen test timestamp parses")
            .utc_unix_nanos()
    }

    fn now() -> i128 {
        utc("2026-10-18T08:00:00Z")
    }

    fn observation(
        id: &str,
        source: &str,
        eta: &str,
        source_age_minutes: i64,
        ingest_age_minutes: i64,
    ) -> PcoObservation {
        let current = now();
        PcoObservation {
            observation_id: id.to_owned(),
            port_id: "PORT-SYNTHETIC".to_owned(),
            port_call_id: "CALL-SYNTHETIC".to_owned(),
            ship_imo_number: "IMO-SYNTHETIC-001".to_owned(),
            field: PcoField::ArrivalAtPilotBoardingPlace,
            location_context: Some("PBP-SYNTHETIC-A".to_owned()),
            source_field_code: "plannedTimeOfArrival".to_owned(),
            value: ObservedValue::Timestamp {
                meaning: TimeMeaning::Planned,
                value: super::super::parse_rfc3339_timestamp(eta)
                    .expect("synthetic ETA parses"),
            },
            source_id: source.to_owned(),
            source_event_time_utc_nanos: current
                - i128::from(source_age_minutes) * NANOS_PER_MINUTE,
            ingested_at_utc_nanos: current - i128::from(ingest_age_minutes) * NANOS_PER_MINUTE,
            valid_until_utc_nanos: Some(current + 6 * 60 * NANOS_PER_MINUTE),
            schema_version: super::super::PCO_GUIDE_VERSION.to_owned(),
        }
    }

    fn window(id: &str, start: &str) -> DecisionWindow {
        DecisionWindow {
            id: id.to_owned(),
            starts_at_utc_nanos: utc(start),
            late_grace_minutes: 15,
        }
    }

    fn input(observations: Vec<PcoObservation>) -> EvidenceDecisionInput {
        let mut validation_policy = ValidationPolicy::new(now());
        validation_policy.max_source_age_seconds = 6 * 60 * 60;
        validation_policy.allowed_future_clock_skew_seconds = 120;
        EvidenceDecisionInput {
            validation_policy,
            windows: vec![
                window("WINDOW-12", "2026-10-18T12:00:00Z"),
                window("WINDOW-13", "2026-10-18T13:00:00Z"),
            ],
            missed_window_penalty_minutes: 100,
            abstention_cost_minutes: 130,
            observations,
        }
    }

    #[test]
    fn latest_ingestion_can_have_high_regret_while_robust_policy_uses_the_evidence_envelope() {
        let input = input(vec![
            observation(
                "OBS-13",
                "SOURCE-A",
                "2026-10-18T13:00:00Z",
                10,
                5,
            ),
            observation(
                "OBS-12",
                "SOURCE-B",
                "2026-10-18T12:00:00Z",
                30,
                1,
            ),
        ]);
        let truth = EvaluationTruth {
            actual_arrival_utc_nanos: utc("2026-10-18T12:50:00Z"),
        };

        let latest = evaluate_decision(
            &input,
            &truth,
            decide(EvidencePolicy::LatestIngestedPointEstimate, &input),
        );
        let freshest = evaluate_decision(
            &input,
            &truth,
            decide(EvidencePolicy::FreshestValidatedSourcePointEstimate, &input),
        );
        let fail_closed = evaluate_decision(
            &input,
            &truth,
            decide(EvidencePolicy::FailClosedUncontestedOnly, &input),
        );
        let robust = evaluate_decision(
            &input,
            &truth,
            decide(EvidencePolicy::BoundedWorstCaseContingency, &input),
        );

        assert_eq!(latest.decision.chosen_window_id.as_deref(), Some("WINDOW-12"));
        assert_eq!(latest.service_cost_minutes, 150);
        assert!(latest.actionable_despite_evidence_issue);
        assert_eq!(freshest.decision.chosen_window_id.as_deref(), Some("WINDOW-13"));
        assert_eq!(robust.decision.chosen_window_id.as_deref(), Some("WINDOW-13"));
        assert_eq!(robust.service_cost_minutes, 10);
        assert_eq!(robust.regret_minutes, 0);
        assert!(fail_closed.decision.abstained());
        assert_eq!(fail_closed.service_cost_minutes, 130);
        assert!(!robust.actionable_despite_evidence_issue);
    }

    #[test]
    fn fail_closed_still_assigns_when_sources_agree_and_are_current() {
        let input = input(vec![
            observation(
                "OBS-CLEAN-A",
                "SOURCE-A",
                "2026-10-18T12:00:00Z",
                10,
                5,
            ),
            observation(
                "OBS-CLEAN-B",
                "SOURCE-B",
                "2026-10-18T12:00:00Z",
                12,
                4,
            ),
        ]);
        let decision = decide(EvidencePolicy::FailClosedUncontestedOnly, &input);
        assert_eq!(decision.chosen_window_id.as_deref(), Some("WINDOW-12"));
        assert!(!decision.abstained());
        assert_eq!(decision.conflicting_group_count, 0);
        assert_eq!(decision.rejected_observation_count, 0);
    }

    #[test]
    fn freshness_policy_rejects_a_stale_only_evidence_set() {
        let mut stale = observation(
            "OBS-STALE",
            "SOURCE-A",
            "2026-10-18T13:00:00Z",
            7 * 60,
            1,
        );
        stale.valid_until_utc_nanos = Some(now() + NANOS_PER_MINUTE);
        let input = input(vec![stale]);
        let freshest = decide(EvidencePolicy::FreshestValidatedSourcePointEstimate, &input);
        let robust = decide(EvidencePolicy::BoundedWorstCaseContingency, &input);
        assert!(freshest.abstained());
        assert!(robust.abstained());
        let latest = decide(EvidencePolicy::LatestIngestedPointEstimate, &input);
        assert_eq!(latest.chosen_window_id.as_deref(), Some("WINDOW-13"));
        assert!(latest.has_evidence_quality_issue());
    }

    #[test]
    fn conflicting_vessel_identity_blocks_robust_and_fail_closed_decisions() {
        let a = observation(
            "OBS-IDENTITY-A",
            "SOURCE-A",
            "2026-10-18T12:00:00Z",
            10,
            5,
        );
        let mut b = observation(
            "OBS-IDENTITY-B",
            "SOURCE-B",
            "2026-10-18T13:00:00Z",
            12,
            4,
        );
        b.ship_imo_number = "IMO-SYNTHETIC-002".to_owned();
        let input = input(vec![a, b]);
        assert!(decide(EvidencePolicy::FailClosedUncontestedOnly, &input).abstained());
        assert!(decide(EvidencePolicy::BoundedWorstCaseContingency, &input).abstained());
    }

    #[test]
    fn location_context_conflict_is_not_reduced_to_a_timing_uncertainty_set() {
        let a = observation(
            "OBS-PBP-A",
            "SOURCE-A",
            "2026-10-18T12:00:00Z",
            10,
            5,
        );
        let mut b = observation(
            "OBS-PBP-B",
            "SOURCE-B",
            "2026-10-18T12:00:00Z",
            12,
            4,
        );
        b.location_context = Some("PBP-SYNTHETIC-B".to_owned());
        let input = input(vec![a, b]);
        let robust = decide(EvidencePolicy::BoundedWorstCaseContingency, &input);
        assert!(robust.abstained());
        assert_eq!(robust.conflicting_group_count, 1);
    }

    #[test]
    fn equivalent_input_permutations_keep_decisions_identical() {
        let a = observation(
            "OBS-13",
            "SOURCE-A",
            "2026-10-18T13:00:00Z",
            10,
            5,
        );
        let b = observation(
            "OBS-12",
            "SOURCE-B",
            "2026-10-18T12:00:00Z",
            30,
            1,
        );
        let first = input(vec![a.clone(), b.clone()]);
        let second = input(vec![b, a]);
        for policy in [
            EvidencePolicy::LatestIngestedPointEstimate,
            EvidencePolicy::FreshestValidatedSourcePointEstimate,
            EvidencePolicy::FailClosedUncontestedOnly,
            EvidencePolicy::BoundedWorstCaseContingency,
        ] {
            assert_eq!(decide(policy, &first), decide(policy, &second));
        }
    }

    #[test]
    fn oracle_cost_is_evaluation_only_and_abstention_is_not_free() {
        let input = input(vec![observation(
            "OBS-CLEAN",
            "SOURCE-A",
            "2026-10-18T12:00:00Z",
            10,
            5,
        )]);
        let truth = EvaluationTruth {
            actual_arrival_utc_nanos: utc("2026-10-18T12:05:00Z"),
        };
        let point = decide(EvidencePolicy::FailClosedUncontestedOnly, &input);
        let evaluated = evaluate_decision(&input, &truth, point);
        assert_eq!(evaluated.oracle_cost_minutes, 5);
        assert_eq!(evaluated.service_cost_minutes, 5);
        let no_windows = EvidenceDecisionInput {
            windows: Vec::new(),
            ..input
        };
        let abstained = decide(EvidencePolicy::FailClosedUncontestedOnly, &no_windows);
        let evaluated_abstention = evaluate_decision(&no_windows, &truth, abstained);
        assert_eq!(evaluated_abstention.service_cost_minutes, 130);
        assert_eq!(evaluated_abstention.oracle_cost_minutes, 130);
        assert_eq!(evaluated_abstention.regret_minutes, 0);
    }
}
