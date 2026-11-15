use aeroweift_intent_conformance_oracle::{
    evaluate, Authorization, ClosedInterval, ConformanceAssessment, ConformanceInput,
    ConformancePolicy, ConformanceReason, ConformanceStatus, DeviationExpansion,
    DynamicRestriction, ObservationWindow, PositionBox, TimeInterval, Volume4D,
};

fn ci(lower: i64, upper: i64) -> ClosedInterval {
    ClosedInterval { lower, upper }
}

fn ti(start: i64, end: i64) -> TimeInterval {
    TimeInterval { start, end }
}

fn pos(east: (i64, i64)) -> PositionBox {
    PositionBox {
        east: ci(east.0, east.1),
        north: ci(1, 2),
        up: ci(1, 2),
    }
}

fn volume(time: (i64, i64)) -> Volume4D {
    Volume4D {
        east: ci(0, 10),
        north: ci(0, 10),
        up: ci(0, 10),
        time: ti(time.0, time.1),
    }
}

fn authorization() -> Authorization {
    Authorization {
        authorization_id: "auth-synthetic-1".to_owned(),
        current_authorization_id: "auth-synthetic-1".to_owned(),
        intent_id: "intent-synthetic-1".to_owned(),
        intent_revision: 1,
        current_intent_revision: 1,
        issuer_reference: "synthetic-authority".to_owned(),
        authorization_sha256: "a".repeat(64),
        time_scale_id: "SYNTHETIC-TIME".to_owned(),
        horizontal_frame_id: "SYNTHETIC-ENU-01".to_owned(),
        vertical_datum_id: "SYNTHETIC-UP-01".to_owned(),
        activated_at_tick: 0,
        expires_at_tick: 200,
        withdrawn: false,
        volume: volume((0, 100)),
    }
}

fn policy() -> ConformancePolicy {
    ConformancePolicy {
        policy_id: "synthetic-conformance-policy-v0".to_owned(),
        profile_id: "synthetic-fixed-wing-profile-v0".to_owned(),
        time_scale_id: "SYNTHETIC-TIME".to_owned(),
        evaluation_window: ti(0, 100),
        evaluation_tick: 100,
        maximum_observation_age_ticks: 200,
        contact_loss_timeout_ticks: 10,
        maximum_definite_outside_ppm: 50_000,
        deviation_expansion: DeviationExpansion {
            east: 1,
            north: 1,
            up: 1,
            time: 2,
        },
    }
}

fn observation(
    id: &str,
    start: i64,
    end: i64,
    east: (i64, i64),
) -> ObservationWindow {
    ObservationWindow {
        observation_id: id.to_owned(),
        source_id: "synthetic-receiver-1".to_owned(),
        support_window: ti(start, end),
        observed_at: ci(start, start),
        receipt_tick: end,
        time_scale_id: "SYNTHETIC-TIME".to_owned(),
        position: pos(east),
        pnt_time_qualified: true,
        surveillance_qualified: true,
        horizontal_frame_id: "SYNTHETIC-ENU-01".to_owned(),
        vertical_datum_id: "SYNTHETIC-UP-01".to_owned(),
    }
}

fn input(observations: Vec<ObservationWindow>) -> ConformanceInput {
    ConformanceInput {
        authorization: authorization(),
        policy: policy(),
        observations,
        dynamic_restriction: None,
    }
}

fn assert_status(
    result: &ConformanceAssessment,
    status: ConformanceStatus,
    reason: Option<ConformanceReason>,
) {
    assert_eq!(result.status, status);
    assert_eq!(result.reason, reason);
}

const FIXTURE: &str = include_str!(
    "../../../fixtures/aeroweift-conformance/tiny-intent-conformance-v0.json"
);

#[test]
fn golden_fixture_identity_binds_all_four_d_scenarios() {
    assert!(FIXTURE.contains("AW-INTENT-CONFORMANCE-TINY-001"));
    assert!(FIXTURE.contains("definite-outside-exceeds-profile-confirmation"));
    assert!(FIXTURE.contains("stale-observation-fresh-receipt-does-not-refresh-it"));
    assert!(FIXTURE.contains("dynamic-restriction-intersects-active-intent"));
    assert!(FIXTURE.contains("half-open-volume-end-is-outside"));
}

#[test]
fn fully_inside_authorized_volume_is_conforming() {
    let result = evaluate(&input(vec![observation("inside", 0, 100, (1, 2))]));
    assert_status(&result, ConformanceStatus::Conforming, None);
    assert_eq!(result.definite_inside_ticks, 100);
    assert_eq!(result.definite_outside_ticks, 0);
    assert_eq!(result.possible_outside_ticks, 0);
    assert_eq!(result.unobserved_ticks, 0);
    assert_eq!(result.qualified_observed_ticks, 100);
    assert_eq!(result.definite_outside_ppm, 0);
    assert_eq!(result.possible_outside_ppm, 0);
}

#[test]
fn nominally_outside_but_inside_explicit_deviation_expansion_is_conforming() {
    let result = evaluate(&input(vec![observation("deviation-margin", 0, 100, (-1, 0))]));
    assert_status(&result, ConformanceStatus::Conforming, None);
    assert_eq!(result.definite_inside_ticks, 100);
}

#[test]
fn boundary_straddling_observation_is_possible_not_definitely_inside() {
    let result = evaluate(&input(vec![observation("ambiguous", 0, 100, (-2, 0))]));
    assert_status(
        &result,
        ConformanceStatus::PotentiallyNonconforming,
        Some(ConformanceReason::ObservedSetStraddlesDeviationBoundary),
    );
    assert_eq!(result.definite_inside_ticks, 0);
    assert_eq!(result.definite_outside_ticks, 0);
    assert_eq!(result.possible_outside_ticks, 100);
}

#[test]
fn definite_outside_duration_above_profile_threshold_is_nonconforming() {
    let result = evaluate(&input(vec![
        observation("outside-six", 0, 6, (12, 13)),
        observation("inside-rest", 6, 100, (1, 2)),
    ]));
    assert_status(
        &result,
        ConformanceStatus::Nonconforming,
        Some(ConformanceReason::DefiniteDeviationThresholdExceeded),
    );
    assert_eq!(result.definite_inside_ticks, 94);
    assert_eq!(result.definite_outside_ticks, 6);
    assert_eq!(result.possible_outside_ticks, 6);
    assert_eq!(result.definite_outside_ppm, 60_000);
}

#[test]
fn isolated_definite_outside_sample_below_threshold_is_not_confirmed() {
    let result = evaluate(&input(vec![
        observation("outside-one", 0, 1, (12, 13)),
        observation("inside-rest", 1, 100, (1, 2)),
    ]));
    assert_status(
        &result,
        ConformanceStatus::PotentiallyNonconforming,
        Some(ConformanceReason::DefiniteDeviationBelowConfirmationThreshold),
    );
    assert_eq!(result.definite_outside_ticks, 1);
    assert_eq!(result.possible_outside_ticks, 1);
    assert_eq!(result.definite_outside_ppm, 10_000);
}

#[test]
fn fresh_receipt_does_not_make_stale_observed_at_time_fresh() {
    let mut sample = observation("stale", 0, 100, (1, 2));
    sample.observed_at = ci(0, 1);
    sample.receipt_tick = 100;
    let mut request = input(vec![sample]);
    request.policy.maximum_observation_age_ticks = 20;

    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::StaleObservation),
    );
    assert_eq!(result.unassessable_ticks, 100);
    assert_eq!(result.possible_outside_ppm, 1_000_000);
    assert_eq!(result.definite_inside_ticks, 0);
    assert!(result.lineage.rejected_observation_ids.contains(&"stale".to_owned()));
}

#[test]
fn last_valid_observation_beyond_contact_timeout_is_contingent() {
    let mut request = input(vec![observation("last-track", 0, 20, (1, 2))]);
    request.policy.evaluation_tick = 50;
    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::Contingent,
        Some(ConformanceReason::ContactTimeout),
    );
    assert_eq!(result.definite_inside_ticks, 20);
    assert_eq!(result.unobserved_ticks, 80);
    assert_eq!(result.possible_outside_ppm, 800_000);
}

#[test]
fn pnt_time_or_surveillance_loss_is_not_conformance() {
    let mut pnt_bad = observation("pnt-bad", 0, 100, (1, 2));
    pnt_bad.pnt_time_qualified = false;
    let result = evaluate(&input(vec![pnt_bad]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::PntTimeUnqualified),
    );
    assert_eq!(result.unassessable_ticks, 100);
    assert_eq!(result.possible_outside_ppm, 1_000_000);

    let mut surveillance_bad = observation("surv-bad", 0, 100, (1, 2));
    surveillance_bad.surveillance_qualified = false;
    let result = evaluate(&input(vec![surveillance_bad]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::SurveillanceUnqualified),
    );
}

#[test]
fn changed_observation_time_scale_is_not_compared() {
    let mut sample = observation("wrong-scale", 0, 100, (1, 2));
    sample.time_scale_id = "OTHER-TIME".to_owned();
    let result = evaluate(&input(vec![sample]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::TimeScaleMismatch),
    );
}

#[test]
fn superseded_intent_revision_requires_reauthorization() {
    let mut request = input(vec![observation("old-revision", 0, 100, (1, 2))]);
    request.authorization.current_intent_revision = 2;
    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::ReauthorizationRequired,
        Some(ConformanceReason::IntentRevisionSuperseded),
    );
}

#[test]
fn expired_or_withdrawn_authorization_requires_reauthorization() {
    let mut expired = input(vec![observation("expired", 0, 100, (1, 2))]);
    expired.authorization.expires_at_tick = 100;
    let result = evaluate(&expired);
    assert_status(
        &result,
        ConformanceStatus::ReauthorizationRequired,
        Some(ConformanceReason::AuthorizationExpired),
    );

    let mut withdrawn = input(vec![observation("withdrawn", 0, 100, (1, 2))]);
    withdrawn.authorization.withdrawn = true;
    let result = evaluate(&withdrawn);
    assert_status(
        &result,
        ConformanceStatus::ReauthorizationRequired,
        Some(ConformanceReason::AuthorizationWithdrawn),
    );
}

#[test]
fn effective_dynamic_restriction_intersecting_intent_requires_reauthorization() {
    let mut request = input(vec![observation("restricted", 0, 100, (1, 2))]);
    request.dynamic_restriction = Some(DynamicRestriction {
        restriction_id: "restriction-intersection".to_owned(),
        effective_window: ti(50, 60),
        volume: Volume4D {
            east: ci(5, 6),
            north: ci(5, 6),
            up: ci(5, 6),
            time: ti(50, 60),
        },
    });
    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::ReauthorizationRequired,
        Some(ConformanceReason::DynamicRestrictionIntersection),
    );
}

#[test]
fn restriction_outside_authorized_volume_does_not_invalidate_authorization() {
    let mut request = input(vec![observation("not-restricted", 0, 100, (1, 2))]);
    request.dynamic_restriction = Some(DynamicRestriction {
        restriction_id: "restriction-outside".to_owned(),
        effective_window: ti(50, 60),
        volume: Volume4D {
            east: ci(20, 30),
            north: ci(20, 30),
            up: ci(20, 30),
            time: ti(50, 60),
        },
    });
    let result = evaluate(&request);
    assert_status(&result, ConformanceStatus::Conforming, None);
}

#[test]
fn dynamic_restriction_effective_interval_must_intersect_authorized_time() {
    let mut request = input(vec![observation("restriction-late", 0, 100, (1, 2))]);
    request.dynamic_restriction = Some(DynamicRestriction {
        restriction_id: "restriction-effective-late".to_owned(),
        effective_window: ti(120, 130),
        volume: Volume4D {
            east: ci(5, 6),
            north: ci(5, 6),
            up: ci(5, 6),
            time: ti(120, 130),
        },
    });
    let result = evaluate(&request);
    assert_status(&result, ConformanceStatus::Conforming, None);
}

#[test]
fn half_open_authorization_end_is_exclusive() {
    let mut request = input(vec![observation("at-end", 100, 101, (1, 2))]);
    request.authorization.volume.time = ti(0, 100);
    request.policy.evaluation_window = ti(100, 101);
    request.policy.evaluation_tick = 101;
    request.policy.deviation_expansion.time = 0;
    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::Nonconforming,
        Some(ConformanceReason::DefiniteDeviationThresholdExceeded),
    );
    assert_eq!(result.definite_outside_ticks, 1);
}

#[test]
fn nonoverlapping_observation_permutation_preserves_results() {
    let first = observation("first", 0, 50, (1, 2));
    let second = observation("second", 50, 100, (1, 2));
    let chronological = evaluate(&input(vec![first.clone(), second.clone()]));
    let permuted = evaluate(&input(vec![second, first]));
    assert_eq!(chronological, permuted);
    assert_status(&permuted, ConformanceStatus::Conforming, None);
}

#[test]
fn overlapping_observation_windows_are_rejected_not_double_counted() {
    let result = evaluate(&input(vec![
        observation("first", 0, 60, (1, 2)),
        observation("overlap", 50, 100, (1, 2)),
    ]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::OverlappingObservationWindows),
    );
}


#[test]
fn future_receipt_and_future_observation_are_not_accepted() {
    let mut sample = observation("future-receipt", 0, 100, (1, 2));
    sample.receipt_tick = 101;
    let mut request = input(vec![sample]);
    request.policy.evaluation_tick = 100;
    let result = evaluate(&request);
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::ReceiptAfterEvaluation),
    );

    let mut sample = observation("future-sample", 0, 100, (1, 2));
    sample.observed_at = ci(100, 100);
    sample.receipt_tick = 100;
    let result = evaluate(&input(vec![sample]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::InvalidObservedAtInterval),
    );
}

#[test]
fn accepted_receipt_preserves_authorization_and_observation_lineage() {
    let result = evaluate(&input(vec![observation("lineage-observation", 0, 100, (1, 2))]));
    assert_eq!(result.lineage.authorization_id, "auth-synthetic-1");
    assert_eq!(result.lineage.intent_id, "intent-synthetic-1");
    assert_eq!(result.lineage.intent_revision, 1);
    assert_eq!(result.lineage.policy_id, "synthetic-conformance-policy-v0");
    assert_eq!(result.lineage.accepted_observations.len(), 1);
    assert_eq!(
        result.lineage.accepted_observations[0].observation_id,
        "lineage-observation"
    );
    assert_eq!(
        result.lineage.accepted_observations[0].position.east,
        ci(1, 2)
    );
}

#[test]
fn authorization_id_change_and_frame_mismatch_fail_closed() {
    let mut changed = input(vec![observation("changed-id", 0, 100, (1, 2))]);
    changed.authorization.current_authorization_id = "new-authorization".to_owned();
    let result = evaluate(&changed);
    assert_status(
        &result,
        ConformanceStatus::ReauthorizationRequired,
        Some(ConformanceReason::AuthorizationIdentityChanged),
    );

    let mut wrong_frame = observation("wrong-frame", 0, 100, (1, 2));
    wrong_frame.horizontal_frame_id = "OTHER-FRAME".to_owned();
    let result = evaluate(&input(vec![wrong_frame]));
    assert_status(
        &result,
        ConformanceStatus::NotAssessable,
        Some(ConformanceReason::FrameOrDatumMismatch),
    );
}
