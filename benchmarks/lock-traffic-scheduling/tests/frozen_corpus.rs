//! Frozen adversarial admission corpus for lock traffic scheduling v0.1.

use inland_waterway_lock_operations::{
    EvidenceState, PassageDisposition, PassageEvaluation, TimeInterval,
};
use lock_traffic_scheduling::{
    ArrivalWindowEvidence, Direction, LockScheduleProfile, NotScheduledReason, PassageRequest,
    SchedulingPolicy, schedule,
};

fn profile() -> LockScheduleProfile {
    LockScheduleProfile {
        lock_id: "lock-corpus-01".into(),
        lock_revision: "lock-rev-01".into(),
        planning_window: TimeInterval {
            start_tick: 20,
            end_tick: 200,
        },
        tick_seconds: 60,
        service_unit_label: "tonne".into(),
        direction_change_ticks: 5,
        maximum_arrival_evidence_age_ticks: 10,
        unavailable_windows: Vec::new(),
        water_budget_ml: 1_000,
        pump_energy_budget_wh: 1_000,
    }
}

fn compatibility(disposition: PassageDisposition) -> PassageEvaluation {
    PassageEvaluation {
        disposition,
        reasons: Vec::new(),
        required_length_mm: 100,
        required_beam_mm: 20,
        required_depth_mm: 10,
    }
}

fn request(id: &str) -> PassageRequest {
    PassageRequest {
        request_id: id.into(),
        vessel_id: format!("vessel-{id}"),
        vessel_revision: "vessel-rev-01".into(),
        compatibility_receipt_id: format!("compat-{id}"),
        compatibility: compatibility(PassageDisposition::EligibleForSyntheticPassage),
        lock_id: "lock-corpus-01".into(),
        lock_revision: "lock-rev-01".into(),
        participant_id: format!("participant-{id}"),
        service_group: "freight".into(),
        direction: Direction::Upstream,
        arrival_earliest_tick: 20,
        arrival_latest_tick: 20,
        arrival_evidence: ArrivalWindowEvidence {
            evidence_id: format!("arrival-{id}"),
            source_id: "symthaea-arrival-model".into(),
            source_revision: "model-rev-01".into(),
            calibration_receipt_id: "arrival-calibration-receipt-01".into(),
            operating_regime_id: "regime-low-flow".into(),
            observed_at_tick: 20,
            valid_through_tick: Some(100),
            state: EvidenceState::Current,
        },
        earliest_start_tick: 20,
        latest_start_tick: 180,
        maximum_wait_ticks: 100,
        service_duration_ticks: 10,
        service_units: 100,
        water_requirement_ml: 10,
        pump_energy_requirement_wh: 10,
        completion_deadline_tick: Some(190),
    }
}

#[test]
fn frozen_arrival_and_eligibility_cases_fail_closed() {
    let cases: Vec<(
        &str,
        LockScheduleProfile,
        PassageRequest,
        Option<NotScheduledReason>,
    )> = {
        let base_profile = profile();
        let mut cases = Vec::new();

        cases.push((
            "current evidence and eligible vessel",
            base_profile.clone(),
            request("current"),
            None,
        ));

        let mut expired = request("expired");
        expired.arrival_evidence.valid_through_tick = Some(19);
        cases.push((
            "expired arrival evidence",
            base_profile.clone(),
            expired,
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview),
        ));

        let mut stale = request("stale");
        stale.arrival_evidence.observed_at_tick = 0;
        cases.push((
            "stale arrival evidence",
            base_profile.clone(),
            stale,
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview),
        ));

        let mut future = request("future");
        future.arrival_evidence.observed_at_tick = 21;
        cases.push((
            "future-dated arrival evidence",
            base_profile.clone(),
            future,
            Some(NotScheduledReason::ArrivalEvidenceInvalid),
        ));

        let mut disputed = request("disputed");
        disputed.arrival_evidence.state = EvidenceState::Disputed;
        cases.push((
            "disputed arrival evidence",
            base_profile.clone(),
            disputed,
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview),
        ));

        let mut unknown = request("unknown");
        unknown.arrival_evidence.state = EvidenceState::Unknown;
        cases.push((
            "unknown arrival evidence",
            base_profile.clone(),
            unknown,
            Some(NotScheduledReason::ArrivalEvidenceNeedsReview),
        ));

        let mut revoked = request("revoked");
        revoked.arrival_evidence.state = EvidenceState::Revoked;
        cases.push((
            "revoked arrival evidence",
            base_profile.clone(),
            revoked,
            Some(NotScheduledReason::ArrivalEvidenceRevoked),
        ));

        let mut missing_receipt = request("missing-receipt");
        missing_receipt
            .arrival_evidence
            .calibration_receipt_id
            .clear();
        cases.push((
            "missing calibration receipt reference",
            base_profile.clone(),
            missing_receipt,
            Some(NotScheduledReason::ArrivalEvidenceInvalid),
        ));

        let mut missing_regime = request("missing-regime");
        missing_regime.arrival_evidence.operating_regime_id.clear();
        cases.push((
            "missing operating regime reference",
            base_profile.clone(),
            missing_regime,
            Some(NotScheduledReason::ArrivalEvidenceInvalid),
        ));

        let mut blocked = request("blocked");
        blocked.compatibility = compatibility(PassageDisposition::BlockedByHardConstraint);
        cases.push((
            "upstream physical compatibility block",
            base_profile.clone(),
            blocked,
            Some(NotScheduledReason::PassageBlocked),
        ));

        let mut upstream_unknown = request("upstream-unknown");
        upstream_unknown.compatibility = compatibility(PassageDisposition::UnknownReviewRequired);
        cases.push((
            "upstream eligibility unknown",
            base_profile.clone(),
            upstream_unknown,
            Some(NotScheduledReason::PassageNeedsReview),
        ));

        let mut inconsistent = request("inconsistent");
        inconsistent.compatibility = compatibility(PassageDisposition::EvidenceInconsistent);
        cases.push((
            "upstream evidence inconsistent",
            base_profile.clone(),
            inconsistent,
            Some(NotScheduledReason::PassageNeedsReview),
        ));

        let mut unsupported = request("unsupported");
        unsupported.compatibility = compatibility(PassageDisposition::UnsupportedOperatingProfile);
        cases.push((
            "unsupported physical operating profile",
            base_profile.clone(),
            unsupported,
            Some(NotScheduledReason::PassageUnsupported),
        ));

        let mut wrong_lock = request("wrong-lock");
        wrong_lock.lock_revision = "old-revision".into();
        cases.push((
            "wrong lock revision",
            base_profile.clone(),
            wrong_lock,
            Some(NotScheduledReason::LockBindingMismatch),
        ));

        let mut bad_arrival_order = request("bad-arrival-order");
        bad_arrival_order.arrival_earliest_tick = 30;
        bad_arrival_order.arrival_latest_tick = 25;
        cases.push((
            "inverted arrival interval",
            base_profile.clone(),
            bad_arrival_order,
            Some(NotScheduledReason::InvalidRequest),
        ));

        let mut no_water = request("no-water");
        no_water.water_requirement_ml = 1_001;
        cases.push((
            "water budget exceeded",
            base_profile.clone(),
            no_water,
            Some(NotScheduledReason::WaterBudgetExceeded),
        ));

        let mut no_energy = request("no-energy");
        no_energy.pump_energy_requirement_wh = 1_001;
        cases.push((
            "pump energy budget exceeded",
            base_profile,
            no_energy,
            Some(NotScheduledReason::PumpEnergyBudgetExceeded),
        ));

        cases
    };

    assert!(
        cases.len() >= 16,
        "the frozen corpus must not silently shrink"
    );
    for (case_name, case_profile, case_request, expected_reason) in cases {
        let plan = schedule(
            &case_profile,
            &[case_request],
            SchedulingPolicy::FirstComeFirstServed,
        )
        .unwrap_or_else(|error| panic!("{case_name}: invalid fixture profile: {error:?}"));
        assert_eq!(
            plan.outcomes.len(),
            1,
            "{case_name}: every input must have exactly one outcome"
        );
        match expected_reason {
            None => {
                assert_eq!(plan.schedule.len(), 1, "{case_name}: expected admission");
                assert_eq!(
                    plan.outcomes[0].reason, None,
                    "{case_name}: unexpected rejection"
                );
            }
            Some(reason) => {
                assert!(
                    plan.schedule.is_empty(),
                    "{case_name}: rejected request was scheduled"
                );
                assert_eq!(
                    plan.outcomes[0].reason,
                    Some(reason),
                    "{case_name}: wrong rejection reason"
                );
            }
        }
    }
}

#[test]
fn duplicate_identifiers_reject_every_ambiguous_record() {
    let duplicate = request("duplicate");
    let plan = schedule(
        &profile(),
        &[duplicate.clone(), duplicate],
        SchedulingPolicy::FirstComeFirstServed,
    )
    .expect("valid fixture profile");
    assert!(plan.schedule.is_empty());
    assert_eq!(plan.outcomes.len(), 2);
    assert!(
        plan.outcomes
            .iter()
            .all(|outcome| { outcome.reason == Some(NotScheduledReason::DuplicateRequestId) })
    );
}
