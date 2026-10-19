//! Deterministic causal replay for timed communications and egress state changes.
//!
//! This replays supplied mission-state events. It does not simulate radio physics,
//! packet delivery, human biomechanics, or physical route safety.

use std::collections::BTreeSet;

use super::egress::{
    CommunicationsState, CoordinationReason, CoordinationSupportStatus, EgressCoordinationReport,
    LocalPlanState, LocalizationState, assess_egress_coordination,
};
use super::{RouteSearchDisposition, TimedEvidence};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionedLocalPlanEvidence {
    /// Exact route-graph version against which this local plan was prepared.
    pub graph_version: String,
    pub evidence: TimedEvidence<LocalPlanState>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MissionTraceEventKind {
    CommunicationsChanged(CommunicationsState),
    LocalizationObserved(TimedEvidence<LocalizationState>),
    LocalPlanObserved(VersionedLocalPlanEvidence),
    /// Declare a new topology/evidence graph version. A plan for another version
    /// immediately becomes unavailable until replaced by a matching plan.
    RouteGraphVersionChanged(String),
    /// Route-search outcome is a separate axis from topology version.
    RouteDispositionChanged(RouteSearchDisposition),
    /// Advance simulated time and reevaluate evidence freshness without
    /// fabricating a new observation or communication event.
    Checkpoint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionTraceEvent {
    pub id: String,
    /// Simulated event/arrival time, in milliseconds.
    pub at_ms: u64,
    pub kind: MissionTraceEventKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceReplayDisposition {
    Complete,
    EventLimitExceeded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionTraceSample {
    pub event_id: String,
    pub at_ms: u64,
    pub current_graph_version: Option<String>,
    pub plan_graph_version: Option<String>,
    pub report: EgressCoordinationReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionTraceReport {
    pub disposition: TraceReplayDisposition,
    pub input_events: usize,
    pub processed_events: usize,
    /// Empty when replay did not complete. Never expose a misleading prefix
    /// as if it were a complete mission trace.
    pub samples: Vec<MissionTraceSample>,
}

/// Replay a fully specified, time-ordered mission-state trace with a strict
/// event cap. The authored order of events sharing a timestamp is significant.
/// Evidence observation time is kept separate from event arrival time, allowing
/// delayed evidence to be stale when it is received.
///
/// A local plan is usable only when its graph version matches the current graph
/// version. Route-search disposition is supplied by a separate evaluator; this
/// function does not independently establish route safety.
pub fn replay_mission_trace(
    initial_route_disposition: RouteSearchDisposition,
    events: &[MissionTraceEvent],
    max_events: usize,
) -> Result<MissionTraceReport, &'static str> {
    if max_events == 0 {
        return Err("trace event cap must be positive");
    }
    if events.len() > max_events {
        return Ok(MissionTraceReport {
            disposition: TraceReplayDisposition::EventLimitExceeded,
            input_events: events.len(),
            processed_events: 0,
            samples: Vec::new(),
        });
    }

    // Validate the complete bounded input before producing replay samples.
    // Equal timestamps are allowed; fixture order is the deterministic tie-break.
    let mut ids = BTreeSet::new();
    let mut previous_at_ms: Option<u64> = None;
    for event in events {
        if event.id.trim().is_empty() {
            return Err("trace event id must not be empty");
        }
        if !ids.insert(event.id.as_str()) {
            return Err("trace event ids must be unique");
        }
        if previous_at_ms.is_some_and(|previous| event.at_ms < previous) {
            return Err("trace events must be ordered by non-decreasing simulated time");
        }
        match &event.kind {
            MissionTraceEventKind::RouteGraphVersionChanged(version)
                if version.trim().is_empty() =>
            {
                return Err("route graph version must not be empty");
            }
            MissionTraceEventKind::LocalPlanObserved(plan)
                if plan.graph_version.trim().is_empty() =>
            {
                return Err("local plan graph version must not be empty");
            }
            _ => {}
        }
        previous_at_ms = Some(event.at_ms);
    }

    let mut communications = CommunicationsState::Unknown;
    let mut localization: Option<TimedEvidence<LocalizationState>> = None;
    let mut local_plan: Option<VersionedLocalPlanEvidence> = None;
    let mut current_graph_version: Option<String> = None;
    let mut route_disposition = initial_route_disposition;
    let mut samples = Vec::with_capacity(events.len());

    for event in events {
        match &event.kind {
            MissionTraceEventKind::CommunicationsChanged(next) => communications = *next,
            MissionTraceEventKind::LocalizationObserved(observation) => {
                localization = Some(observation.clone());
            }
            MissionTraceEventKind::LocalPlanObserved(observation) => {
                local_plan = Some(observation.clone());
            }
            MissionTraceEventKind::RouteGraphVersionChanged(version) => {
                current_graph_version = Some(version.clone());
            }
            MissionTraceEventKind::RouteDispositionChanged(next) => {
                route_disposition = *next;
            }
            MissionTraceEventKind::Checkpoint => {}
        }

        let plan_evidence = local_plan.as_ref().map(|plan| plan.evidence.clone());
        let plan_version_mismatch = local_plan.as_ref().is_some_and(|plan| {
            current_graph_version.as_deref() != Some(plan.graph_version.as_str())
        });

        let mut report = assess_egress_coordination(
            route_disposition,
            communications,
            localization.as_ref(),
            plan_evidence.as_ref(),
            event.at_ms,
        )?;

        // A known graph-version mismatch is an explicit invalidation, even when
        // the old plan's timestamp is fresh. Do not leave its earlier "available"
        // label intact merely because no replacement plan has arrived yet.
        if plan_version_mismatch {
            report.coordination_status = CoordinationSupportStatus::RequiredLocalInputsUnavailable;
            report
                .reasons
                .push(CoordinationReason::LocalPlanNotVersionMatched);
            report.reasons.sort();
            report.reasons.dedup();
        }

        samples.push(MissionTraceSample {
            event_id: event.id.clone(),
            at_ms: event.at_ms,
            current_graph_version: current_graph_version.clone(),
            plan_graph_version: local_plan.as_ref().map(|plan| plan.graph_version.clone()),
            report,
        });
    }

    Ok(MissionTraceReport {
        disposition: TraceReplayDisposition::Complete,
        input_events: events.len(),
        processed_events: events.len(),
        samples,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(id: &str, at_ms: u64, kind: MissionTraceEventKind) -> MissionTraceEvent {
        MissionTraceEvent {
            id: id.to_owned(),
            at_ms,
            kind,
        }
    }

    fn location(observed_at_ms: u64, max_age_ms: u64) -> TimedEvidence<LocalizationState> {
        TimedEvidence {
            value: LocalizationState::CurrentFixAvailable,
            observed_at_ms,
            max_age_ms,
            evidence_id: format!("location-{observed_at_ms}"),
        }
    }

    fn plan(
        graph_version: &str,
        observed_at_ms: u64,
        max_age_ms: u64,
    ) -> VersionedLocalPlanEvidence {
        VersionedLocalPlanEvidence {
            graph_version: graph_version.to_owned(),
            evidence: TimedEvidence {
                value: LocalPlanState::VersionMatchedPlanAvailable,
                observed_at_ms,
                max_age_ms,
                evidence_id: format!("plan-{graph_version}-{observed_at_ms}"),
            },
        }
    }

    fn base_events() -> Vec<MissionTraceEvent> {
        vec![
            event(
                "graph-v1",
                100,
                MissionTraceEventKind::RouteGraphVersionChanged("graph-v1".to_owned()),
            ),
            event(
                "loc",
                100,
                MissionTraceEventKind::LocalizationObserved(location(100, 1000)),
            ),
            event(
                "plan",
                100,
                MissionTraceEventKind::LocalPlanObserved(plan("graph-v1", 100, 1000)),
            ),
            event(
                "comms-live",
                100,
                MissionTraceEventKind::CommunicationsChanged(
                    CommunicationsState::LiveAcknowledgedPath,
                ),
            ),
        ]
    }

    #[test]
    fn identical_input_trace_replays_to_identical_ordered_report() {
        let mut events = base_events();
        events.push(event(
            "isolate",
            200,
            MissionTraceEventKind::CommunicationsChanged(CommunicationsState::Isolated),
        ));
        let first = replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
            .expect("valid trace");
        let second =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(first, second);
        assert_eq!(first.disposition, TraceReplayDisposition::Complete);
        assert_eq!(first.processed_events, events.len());
    }

    #[test]
    fn communications_isolation_changes_coordination_not_route_disposition() {
        let mut events = base_events();
        events.push(event(
            "isolate",
            101,
            MissionTraceEventKind::CommunicationsChanged(CommunicationsState::Isolated),
        ));
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[3].report.coordination_status,
            CoordinationSupportStatus::LiveCoordinationAvailable
        );
        assert_eq!(
            report.samples[4].report.coordination_status,
            CoordinationSupportStatus::OfflinePlanAndLocationAvailable
        );
        assert_eq!(
            report.samples[4].report.route_disposition,
            RouteSearchDisposition::AdmissibleRouteFound
        );
    }

    #[test]
    fn checkpoint_can_make_previously_fresh_location_stale() {
        let events = vec![
            event(
                "graph-v1",
                100,
                MissionTraceEventKind::RouteGraphVersionChanged("graph-v1".to_owned()),
            ),
            event(
                "loc",
                100,
                MissionTraceEventKind::LocalizationObserved(location(100, 10)),
            ),
            event(
                "plan",
                100,
                MissionTraceEventKind::LocalPlanObserved(plan("graph-v1", 100, 1000)),
            ),
            event(
                "isolate",
                100,
                MissionTraceEventKind::CommunicationsChanged(CommunicationsState::Isolated),
            ),
            event("tick", 111, MissionTraceEventKind::Checkpoint),
        ];
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[3].report.coordination_status,
            CoordinationSupportStatus::OfflinePlanAndLocationAvailable
        );
        assert_eq!(
            report.samples[4].report.coordination_status,
            CoordinationSupportStatus::CoordinationUnknown
        );
        assert!(
            report.samples[4]
                .report
                .reasons
                .contains(&CoordinationReason::LocalizationStale)
        );
    }

    #[test]
    fn pending_store_carry_forward_never_becomes_live_delivery() {
        let mut events = base_events();
        events.push(event(
            "delay",
            101,
            MissionTraceEventKind::CommunicationsChanged(
                CommunicationsState::StoreCarryForwardPending,
            ),
        ));
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[4].report.coordination_status,
            CoordinationSupportStatus::LocalInputsAvailableMessagesDeferred
        );
    }

    #[test]
    fn graph_change_invalidates_old_plan_until_a_matching_plan_is_observed() {
        let mut events = base_events();
        events.push(event(
            "graph-v2",
            101,
            MissionTraceEventKind::RouteGraphVersionChanged("graph-v2".to_owned()),
        ));
        events.push(event(
            "plan-v2",
            102,
            MissionTraceEventKind::LocalPlanObserved(plan("graph-v2", 102, 1000)),
        ));
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[4].report.coordination_status,
            CoordinationSupportStatus::RequiredLocalInputsUnavailable
        );
        assert!(
            report.samples[4]
                .report
                .reasons
                .contains(&CoordinationReason::LocalPlanNotVersionMatched)
        );
        assert_eq!(
            report.samples[5].report.coordination_status,
            CoordinationSupportStatus::LiveCoordinationAvailable
        );
        assert_eq!(
            report.samples[5].current_graph_version.as_deref(),
            Some("graph-v2")
        );
        assert_eq!(
            report.samples[5].plan_graph_version.as_deref(),
            Some("graph-v2")
        );
    }

    #[test]
    fn route_closure_and_search_limit_remain_distinct_during_isolation() {
        let mut events = base_events();
        events.push(event(
            "isolate",
            101,
            MissionTraceEventKind::CommunicationsChanged(CommunicationsState::Isolated),
        ));
        events.push(event(
            "graph-v2",
            102,
            MissionTraceEventKind::RouteGraphVersionChanged("graph-v2".to_owned()),
        ));
        events.push(event(
            "closed",
            103,
            MissionTraceEventKind::RouteDispositionChanged(
                RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph,
            ),
        ));
        events.push(event(
            "limited",
            104,
            MissionTraceEventKind::RouteDispositionChanged(
                RouteSearchDisposition::SearchLimitExceeded,
            ),
        ));
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[6].report.route_disposition,
            RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph
        );
        assert_eq!(
            report.samples[7].report.route_disposition,
            RouteSearchDisposition::SearchLimitExceeded
        );
        assert_eq!(
            report.samples[7].report.coordination_status,
            CoordinationSupportStatus::RequiredLocalInputsUnavailable
        );
    }

    #[test]
    fn same_timestamp_events_preserve_fixture_order() {
        let mut events = base_events();
        events.push(event(
            "isolate",
            100,
            MissionTraceEventKind::CommunicationsChanged(CommunicationsState::Isolated),
        ));
        let report =
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 20)
                .expect("valid trace");
        assert_eq!(
            report.samples[3].report.coordination_status,
            CoordinationSupportStatus::LiveCoordinationAvailable
        );
        assert_eq!(
            report.samples[4].report.coordination_status,
            CoordinationSupportStatus::OfflinePlanAndLocationAvailable
        );
        assert_eq!(report.samples[3].at_ms, report.samples[4].at_ms);
    }

    #[test]
    fn route_and_coordination_axes_pass_frozen_cross_product_matrix() {
        let routes = [
            RouteSearchDisposition::AdmissibleRouteFound,
            RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph,
            RouteSearchDisposition::SearchLimitExceeded,
        ];
        let communications = [
            (
                CommunicationsState::LiveAcknowledgedPath,
                CoordinationSupportStatus::LiveCoordinationAvailable,
            ),
            (
                CommunicationsState::StoreCarryForwardPending,
                CoordinationSupportStatus::LocalInputsAvailableMessagesDeferred,
            ),
            (
                CommunicationsState::Isolated,
                CoordinationSupportStatus::OfflinePlanAndLocationAvailable,
            ),
            (
                CommunicationsState::Unknown,
                CoordinationSupportStatus::CoordinationUnknown,
            ),
        ];

        // 3 route outcomes x 4 communications states = 12 cross-product cases.
        // Changing one axis must not silently mutate the other axis.
        for route in routes {
            for (index, (comms, expected_coordination)) in communications.iter().enumerate() {
                let mut events = base_events();
                events.push(event(
                    &format!("comms-{index}"),
                    100,
                    MissionTraceEventKind::CommunicationsChanged(*comms),
                ));
                let report = replay_mission_trace(route, &events, 20).expect("valid matrix trace");
                let last = report.samples.last().expect("one sample per event");
                assert_eq!(last.report.route_disposition, route);
                assert_eq!(last.report.coordination_status, *expected_coordination);
            }
        }
    }

    #[test]
    fn out_of_order_events_are_rejected() {
        let events = vec![
            event("later", 200, MissionTraceEventKind::Checkpoint),
            event("earlier", 199, MissionTraceEventKind::Checkpoint),
        ];
        assert!(
            replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &events, 10).is_err()
        );
    }

    #[test]
    fn duplicate_event_ids_are_rejected() {
        let events = vec![
            event("duplicate", 100, MissionTraceEventKind::Checkpoint),
            event("duplicate", 101, MissionTraceEventKind::Checkpoint),
        ];
        assert!(
            replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &events, 10).is_err()
        );
    }

    #[test]
    fn empty_event_id_is_rejected() {
        let events = vec![event("  ", 100, MissionTraceEventKind::Checkpoint)];
        assert!(
            replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &events, 10).is_err()
        );
    }

    #[test]
    fn empty_graph_version_and_empty_plan_version_are_rejected() {
        let graph_event = vec![event(
            "graph",
            100,
            MissionTraceEventKind::RouteGraphVersionChanged(" ".to_owned()),
        )];
        assert!(
            replay_mission_trace(
                RouteSearchDisposition::SearchLimitExceeded,
                &graph_event,
                10
            )
            .is_err()
        );

        let plan_event = vec![event(
            "plan",
            100,
            MissionTraceEventKind::LocalPlanObserved(plan(" ", 100, 1000)),
        )];
        assert!(
            replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &plan_event, 10)
                .is_err()
        );
    }

    #[test]
    fn event_cap_overflow_returns_no_partial_trace() {
        let events = vec![
            event("a", 100, MissionTraceEventKind::Checkpoint),
            event("b", 101, MissionTraceEventKind::Checkpoint),
        ];
        let report = replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &events, 1)
            .expect("limit is an explicit result");
        assert_eq!(
            report.disposition,
            TraceReplayDisposition::EventLimitExceeded
        );
        assert_eq!(report.processed_events, 0);
        assert!(report.samples.is_empty());
    }

    #[test]
    fn zero_event_cap_is_invalid() {
        assert!(replay_mission_trace(RouteSearchDisposition::SearchLimitExceeded, &[], 0).is_err());
    }

    #[test]
    fn future_dated_evidence_is_rejected_at_replay_event_time() {
        let events = vec![event(
            "future-loc",
            100,
            MissionTraceEventKind::LocalizationObserved(location(101, 1000)),
        )];
        assert!(
            replay_mission_trace(RouteSearchDisposition::AdmissibleRouteFound, &events, 10)
                .is_err()
        );
    }
}
