//! Coordination-support status for an egress mission under communications faults.
//!
//! This module does not decide whether a passage is physically safe. It keeps
//! route-search state and communications/local-guidance state as separate axes,
//! so connectivity cannot make an unverified route admissible and network loss
//! cannot silently rewrite the route evaluator's result.

use super::{RouteSearchDisposition, TimedEvidence, TimedRead, read_timed};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommunicationsState {
    /// A live path has a current end-to-end acknowledgement.
    LiveAcknowledgedPath,
    /// Messages are queued; no live end-to-end delivery is implied.
    StoreCarryForwardPending,
    /// No current communication path is available.
    Isolated,
    /// Current communications state cannot be established.
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalizationState {
    CurrentFixAvailable,
    NoCurrentFix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocalPlanState {
    /// Local plan matches the declared route-graph version; field clearance is not implied.
    VersionMatchedPlanAvailable,
    MissingOrUnverifiedPlan,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CoordinationReason {
    LocalizationMissing,
    LocalizationStale,
    LocalizationUnavailable,
    LocalPlanMissing,
    LocalPlanStale,
    LocalPlanNotVersionMatched,
    MessagesAwaitingDelivery,
    CommunicationsIsolated,
    CommunicationsStateUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinationSupportStatus {
    /// Fresh location and plan, plus a live acknowledged path.
    LiveCoordinationAvailable,
    /// Fresh location and plan; messages are pending rather than delivered.
    LocalInputsAvailableMessagesDeferred,
    /// Fresh local location and plan remain available without a live network path.
    OfflinePlanAndLocationAvailable,
    /// Required evidence or current communications state is missing, stale, or unknown.
    CoordinationUnknown,
    /// Fresh evidence explicitly says the current location or local plan is unavailable.
    RequiredLocalInputsUnavailable,
}

/// Route admissibility and coordination are separate dimensions. Inspect both;
/// neither is an operational rescue authorization.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EgressCoordinationReport {
    pub route_disposition: RouteSearchDisposition,
    pub coordination_status: CoordinationSupportStatus,
    pub reasons: Vec<CoordinationReason>,
}

/// Assess coordination/guidance support, not whether it is safe to proceed.
/// Future timestamps and empty evidence identifiers are rejected.
pub fn assess_egress_coordination(
    route_disposition: RouteSearchDisposition,
    communications: CommunicationsState,
    localization: Option<&TimedEvidence<LocalizationState>>,
    local_plan: Option<&TimedEvidence<LocalPlanState>>,
    now_ms: u64,
) -> Result<EgressCoordinationReport, &'static str> {
    let location_read = read_timed(localization, now_ms)?;
    let plan_read = read_timed(local_plan, now_ms)?;
    let mut reasons = Vec::new();

    match location_read {
        TimedRead::Missing => reasons.push(CoordinationReason::LocalizationMissing),
        TimedRead::Stale => reasons.push(CoordinationReason::LocalizationStale),
        TimedRead::Fresh(LocalizationState::NoCurrentFix) => {
            reasons.push(CoordinationReason::LocalizationUnavailable)
        }
        TimedRead::Fresh(LocalizationState::CurrentFixAvailable) => {}
    }
    match plan_read {
        TimedRead::Missing => reasons.push(CoordinationReason::LocalPlanMissing),
        TimedRead::Stale => reasons.push(CoordinationReason::LocalPlanStale),
        TimedRead::Fresh(LocalPlanState::MissingOrUnverifiedPlan) => {
            reasons.push(CoordinationReason::LocalPlanNotVersionMatched)
        }
        TimedRead::Fresh(LocalPlanState::VersionMatchedPlanAvailable) => {}
    }

    let location_unavailable = matches!(
        location_read,
        TimedRead::Fresh(LocalizationState::NoCurrentFix)
    );
    let plan_unavailable = matches!(
        plan_read,
        TimedRead::Fresh(LocalPlanState::MissingOrUnverifiedPlan)
    );
    let evidence_unknown = matches!(location_read, TimedRead::Missing | TimedRead::Stale)
        || matches!(plan_read, TimedRead::Missing | TimedRead::Stale);

    let coordination_status = if location_unavailable || plan_unavailable {
        CoordinationSupportStatus::RequiredLocalInputsUnavailable
    } else if evidence_unknown {
        CoordinationSupportStatus::CoordinationUnknown
    } else {
        match communications {
            CommunicationsState::LiveAcknowledgedPath => {
                CoordinationSupportStatus::LiveCoordinationAvailable
            }
            CommunicationsState::StoreCarryForwardPending => {
                reasons.push(CoordinationReason::MessagesAwaitingDelivery);
                CoordinationSupportStatus::LocalInputsAvailableMessagesDeferred
            }
            CommunicationsState::Isolated => {
                reasons.push(CoordinationReason::CommunicationsIsolated);
                CoordinationSupportStatus::OfflinePlanAndLocationAvailable
            }
            CommunicationsState::Unknown => {
                reasons.push(CoordinationReason::CommunicationsStateUnknown);
                CoordinationSupportStatus::CoordinationUnknown
            }
        }
    };
    reasons.sort();
    reasons.dedup();

    Ok(EgressCoordinationReport {
        route_disposition,
        coordination_status,
        reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW_MS: u64 = 150;

    fn localization(value: LocalizationState) -> TimedEvidence<LocalizationState> {
        TimedEvidence {
            value,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: "location-001".to_owned(),
        }
    }
    fn plan_evidence(value: LocalPlanState) -> TimedEvidence<LocalPlanState> {
        TimedEvidence {
            value,
            observed_at_ms: 100,
            max_age_ms: 100,
            evidence_id: "plan-001".to_owned(),
        }
    }
    fn assess(
        route: RouteSearchDisposition,
        comms: CommunicationsState,
        loc: Option<&TimedEvidence<LocalizationState>>,
        plan: Option<&TimedEvidence<LocalPlanState>>,
    ) -> EgressCoordinationReport {
        assess_egress_coordination(route, comms, loc, plan, NOW_MS).expect("valid fixture")
    }

    #[test]
    fn live_coordination_requires_fresh_location_and_matched_plan() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::LiveAcknowledgedPath,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::LiveCoordinationAvailable
        );
    }

    #[test]
    fn isolation_degrades_coordination_without_mutating_route_disposition() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::Isolated,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::OfflinePlanAndLocationAvailable
        );
        assert_eq!(
            r.route_disposition,
            RouteSearchDisposition::AdmissibleRouteFound
        );
        assert!(
            r.reasons
                .contains(&CoordinationReason::CommunicationsIsolated)
        );
    }

    #[test]
    fn store_carry_forward_is_not_live_delivery() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::StoreCarryForwardPending,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::LocalInputsAvailableMessagesDeferred
        );
        assert!(
            r.reasons
                .contains(&CoordinationReason::MessagesAwaitingDelivery)
        );
    }

    #[test]
    fn stale_location_is_unknown_not_current() {
        let mut loc = localization(LocalizationState::CurrentFixAvailable);
        loc.observed_at_ms = 0;
        loc.max_age_ms = 10;
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::LiveAcknowledgedPath,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::CoordinationUnknown
        );
        assert!(r.reasons.contains(&CoordinationReason::LocalizationStale));
    }

    #[test]
    fn missing_plan_evidence_is_unknown() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::Isolated,
            Some(&loc),
            None,
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::CoordinationUnknown
        );
        assert!(r.reasons.contains(&CoordinationReason::LocalPlanMissing));
    }

    #[test]
    fn explicitly_unavailable_location_or_unverified_plan_blocks_guidance_claim() {
        let loc = localization(LocalizationState::NoCurrentFix);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::LiveAcknowledgedPath,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::RequiredLocalInputsUnavailable
        );

        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::MissingOrUnverifiedPlan);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::Isolated,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::RequiredLocalInputsUnavailable
        );
    }

    #[test]
    fn unknown_communications_never_claim_live_coordination() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        let r = assess(
            RouteSearchDisposition::AdmissibleRouteFound,
            CommunicationsState::Unknown,
            Some(&loc),
            Some(&plan),
        );
        assert_eq!(
            r.coordination_status,
            CoordinationSupportStatus::CoordinationUnknown
        );
        assert!(
            r.reasons
                .contains(&CoordinationReason::CommunicationsStateUnknown)
        );
    }

    #[test]
    fn no_route_and_search_limit_remain_distinct_during_isolation() {
        let loc = localization(LocalizationState::CurrentFixAvailable);
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        for route in [
            RouteSearchDisposition::NoAdmissibleRouteInDeclaredGraph,
            RouteSearchDisposition::SearchLimitExceeded,
        ] {
            let r = assess(
                route,
                CommunicationsState::Isolated,
                Some(&loc),
                Some(&plan),
            );
            assert_eq!(r.route_disposition, route);
            assert_eq!(
                r.coordination_status,
                CoordinationSupportStatus::OfflinePlanAndLocationAvailable
            );
        }
    }

    #[test]
    fn future_timestamps_and_empty_evidence_ids_are_rejected() {
        let mut loc = localization(LocalizationState::CurrentFixAvailable);
        loc.observed_at_ms = NOW_MS + 1;
        let plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        assert!(
            assess_egress_coordination(
                RouteSearchDisposition::AdmissibleRouteFound,
                CommunicationsState::LiveAcknowledgedPath,
                Some(&loc),
                Some(&plan),
                NOW_MS
            )
            .is_err()
        );

        let loc = localization(LocalizationState::CurrentFixAvailable);
        let mut plan = plan_evidence(LocalPlanState::VersionMatchedPlanAvailable);
        plan.evidence_id.clear();
        assert!(
            assess_egress_coordination(
                RouteSearchDisposition::AdmissibleRouteFound,
                CommunicationsState::LiveAcknowledgedPath,
                Some(&loc),
                Some(&plan),
                NOW_MS
            )
            .is_err()
        );
    }
}
