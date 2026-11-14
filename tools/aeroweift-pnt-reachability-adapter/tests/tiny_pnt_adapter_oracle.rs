use aeroweift_pnt_reachability_adapter::{
    evaluate, AdapterInput, AdapterOutcome, AdapterPolicy, BoundKind, ClockQualification,
    IndependenceAssessment, NavigationClaim, NotAssessableReason, PntServiceState,
    PositionBounds, VelocityBounds,
};
use aeroweift_reachability_core::Interval;

const GOLDEN_FIXTURE: &str = include_str!(
    "../../../fixtures/aeroweift-pnt-reachability/tiny-pnt-time-envelope-v0.json"
);

fn iv(lower: f64, upper: f64) -> Interval {
    Interval::new(lower, upper).expect("valid test interval")
}

fn policy() -> AdapterPolicy {
    AdapterPolicy {
        profile_id: "synthetic-fixed-wing-profile-v0".to_owned(),
        horizontal_frame_id: "SYNTHETIC-ENU-01".to_owned(),
        vertical_datum_id: "SYNTHETIC-UP-01".to_owned(),
        maximum_sample_age_ms: 2_500,
        minimum_independent_families: 2,
    }
}

fn claim() -> NavigationClaim {
    NavigationClaim {
        source_id: "synthetic-nav-primary".to_owned(),
        source_claim_sha256: "a".repeat(64),
        profile_id: "synthetic-fixed-wing-profile-v0".to_owned(),
        horizontal_frame_id: "SYNTHETIC-ENU-01".to_owned(),
        vertical_datum_id: "SYNTHETIC-UP-01".to_owned(),
        bound_kind: BoundKind::DeterministicHardBound,
        sample_time_lower_utc_ms: 900,
        sample_time_upper_utc_ms: 1_100,
        sample_time_scale_id: "SYNTHETIC-UTC-LIKE".to_owned(),
        assessment_time_scale_id: "SYNTHETIC-UTC-LIKE".to_owned(),
        clock_qualification: ClockQualification::Qualified,
        clock_correlation_id: Some("synthetic-clock-correlation-a".to_owned()),
        clock_correlation_sha256: Some("b".repeat(64)),
        independence: IndependenceAssessment::QualifiedWithinProfile,
        independent_family_count: Some(2),
        dependency_snapshot_complete: true,
        dependency_snapshot_id: Some("synthetic-dependency-snapshot-a".to_owned()),
        position: PositionBounds {
            east_m: iv(100.0, 101.0),
            north_m: iv(-20.0, -19.0),
            up_m: iv(5.0, 6.0),
        },
        velocity: VelocityBounds {
            east_mps: iv(4.0, 5.0),
            north_mps: iv(-2.0, -1.0),
            up_mps: iv(0.5, 1.0),
        },
    }
}

fn input(state: PntServiceState) -> AdapterInput {
    AdapterInput {
        service_state: state,
        primary: Some(claim()),
        fallback: None,
        assessment_time_lower_utc_ms: 2_990,
        assessment_time_upper_utc_ms: 3_010,
        assessment_time_scale_id: "SYNTHETIC-UTC-LIKE".to_owned(),
        assessment_clock_qualification: ClockQualification::Qualified,
        assessment_clock_correlation_id: Some("synthetic-evaluation-clock-correlation".to_owned()),
        assessment_clock_correlation_sha256: Some("f".repeat(64)),
        service_state_receipt_sha256: "c".repeat(64),
        policy: policy(),
    }
}

fn accepted(outcome: AdapterOutcome) -> (Interval, PositionBounds) {
    match outcome {
        AdapterOutcome::AcceptedPrimary {
            elapsed_seconds,
            position,
            ..
        }
        | AdapterOutcome::AcceptedFallback {
            elapsed_seconds,
            position,
            ..
        } => (elapsed_seconds, position),
        other => panic!("expected accepted hard bound, got {other:?}"),
    }
}

fn assert_contains(value: Interval, lower: f64, upper: f64) {
    assert!(
        value.lower() <= lower + 1.0e-12,
        "lower {} does not contain {lower}",
        value.lower()
    );
    assert!(
        value.upper() >= upper - 1.0e-12,
        "upper {} does not contain {upper}",
        value.upper()
    );
}

fn assert_rejected(outcome: AdapterOutcome, expected: NotAssessableReason) {
    assert_eq!(
        outcome,
        AdapterOutcome::NotAssessable { reason: expected }
    );
}

#[test]
fn golden_fixture_identity_and_claim_ceiling_are_bound() {
    assert!(GOLDEN_FIXTURE.contains("AW-PNT-REACH-TINY-001"));
    assert!(GOLDEN_FIXTURE.contains("normal-fresh-clock-interval"));
    assert!(GOLDEN_FIXTURE.contains("statistical-95-percent-envelope-only"));
    assert!(GOLDEN_FIXTURE.contains("contradictory-independent-witnesses"));
    assert!(GOLDEN_FIXTURE.contains("NOT_ASSESSABLE_FOR_HARD_BOUND") || GOLDEN_FIXTURE.contains("STATISTICAL_ENVELOPE_NOT_HARD_BOUND"));
}


#[test]
fn accepted_receipt_preserves_source_profile_clock_and_dependency_lineage() {
    match evaluate(&input(PntServiceState::Normal)) {
        AdapterOutcome::AcceptedPrimary { lineage, .. } => {
            assert_eq!(lineage.source_id, "synthetic-nav-primary");
            assert_eq!(lineage.profile_id, "synthetic-fixed-wing-profile-v0");
            assert_eq!(lineage.horizontal_frame_id, "SYNTHETIC-ENU-01");
            assert_eq!(lineage.vertical_datum_id, "SYNTHETIC-UP-01");
            assert_eq!(
                lineage.clock_correlation_id,
                "synthetic-clock-correlation-a"
            );
            assert_eq!(
                lineage.assessment_clock_correlation_id,
                "synthetic-evaluation-clock-correlation"
            );
            assert_eq!(lineage.sample_time_interval_utc_ms, [900, 1_100]);
            assert_eq!(lineage.assessment_time_interval_utc_ms, [2_990, 3_010]);
            assert_eq!(
                lineage.dependency_snapshot_id,
                "synthetic-dependency-snapshot-a"
            );
        }
        other => panic!("expected primary receipt, got {other:?}"),
    }
}

#[test]
fn normal_claim_expands_position_by_full_clock_interval_and_velocity_bounds() {
    let (elapsed, position) = accepted(evaluate(&input(PntServiceState::Normal)));

    assert_contains(elapsed, 1.89, 2.11);
    assert_contains(position.east_m, 107.56, 111.55);
    assert_contains(position.north_m, -24.22, -20.89);
    assert_contains(position.up_m, 5.945, 8.11);
}

#[test]
fn exact_sample_time_produces_no_wider_enclosure_than_uncertain_sample_time() {
    let (uncertain_elapsed, uncertain) =
        accepted(evaluate(&input(PntServiceState::Normal)));

    let mut exact_input = input(PntServiceState::Normal);
    let exact_claim = exact_input.primary.as_mut().expect("primary claim");
    exact_claim.sample_time_lower_utc_ms = 1_000;
    exact_claim.sample_time_upper_utc_ms = 1_000;

    let (exact_elapsed, exact) = accepted(evaluate(&exact_input));

    let uncertain_width = uncertain.east_m.upper() - uncertain.east_m.lower();
    let exact_width = exact.east_m.upper() - exact.east_m.lower();

    assert!(exact_width <= uncertain_width);
    assert!(exact_elapsed.lower() >= uncertain_elapsed.lower());
    assert!(exact_elapsed.upper() <= uncertain_elapsed.upper());
    assert_contains(exact.east_m, 107.96, 111.05);
}

#[test]
fn degraded_state_uses_only_qualified_fallback_claim() {
    let mut degraded = input(PntServiceState::Degraded);
    let mut fallback = claim();
    fallback.source_id = "synthetic-independent-fallback".to_owned();
    fallback.source_claim_sha256 = "d".repeat(64);
    fallback.dependency_snapshot_id = Some("synthetic-fallback-snapshot".to_owned());
    fallback.position = PositionBounds {
        east_m: iv(200.0, 201.0),
        north_m: iv(-20.0, -19.0),
        up_m: iv(5.0, 6.0),
    };
    fallback.velocity = VelocityBounds {
        east_mps: iv(-2.0, 3.0),
        north_mps: iv(-1.0, 1.0),
        up_mps: iv(0.0, 1.0),
    };
    degraded.fallback = Some(fallback);

    match evaluate(&degraded) {
        AdapterOutcome::AcceptedFallback {
            lineage,
            position,
            ..
        } => {
            assert_eq!(lineage.source_id, "synthetic-independent-fallback");
            assert_contains(position.east_m, 195.78, 207.33);
            assert_contains(position.north_m, -22.11, -16.89);
            assert_contains(position.up_m, 5.0, 8.11);
        }
        other => panic!("expected fallback admission, got {other:?}"),
    }
}

#[test]
fn integrity_uncertain_correlated_primary_without_fallback_is_not_assessable() {
    assert_rejected(
        evaluate(&input(PntServiceState::IntegrityUncertain)),
        NotAssessableReason::NoQualifiedIndependentFallback,
    );
}

#[test]
fn statistical_prediction_is_never_admitted_as_hard_bound() {
    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().bound_kind = BoundKind::StatisticalPrediction;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::StatisticalEnvelopeNotHardBound,
    );
}

#[test]
fn stale_observation_is_rejected_using_oldest_possible_sample_time() {
    let mut sample = input(PntServiceState::Normal);
    let source = sample.primary.as_mut().unwrap();
    source.sample_time_lower_utc_ms = 300;
    source.sample_time_upper_utc_ms = 500;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::SampleAgeExceedsProfileLimit,
    );
}

#[test]
fn sample_interval_overlapping_assessment_epoch_is_rejected_not_clamped() {
    let mut sample = input(PntServiceState::Normal);
    let source = sample.primary.as_mut().unwrap();
    source.sample_time_lower_utc_ms = 2_900;
    source.sample_time_upper_utc_ms = 3_100;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::SampleTimeOverlapsAssessmentEpoch,
    );
}

#[test]
fn recovering_and_indeterminate_states_never_become_normal_by_default() {
    assert_rejected(
        evaluate(&input(PntServiceState::Recovering)),
        NotAssessableReason::UpstreamStillRecovering,
    );
    assert_rejected(
        evaluate(&input(PntServiceState::Indeterminate)),
        NotAssessableReason::UpstreamIndeterminate,
    );
}

#[test]
fn unknown_clock_or_dependency_evidence_is_not_independence() {
    let mut sample = input(PntServiceState::Normal);
    let source = sample.primary.as_mut().unwrap();
    source.clock_qualification = ClockQualification::Unknown;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::ClockCorrelationUnknown,
    );

    let mut sample = input(PntServiceState::Normal);
    let source = sample.primary.as_mut().unwrap();
    source.independence = IndependenceAssessment::Unknown;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::IndependenceNotQualified,
    );
}


#[test]
fn unknown_evaluation_clock_and_incompatible_time_scales_are_rejected() {
    let mut sample = input(PntServiceState::Normal);
    sample.assessment_clock_qualification = ClockQualification::Unknown;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::AssessmentClockUnqualified,
    );

    let mut sample = input(PntServiceState::Normal);
    sample.assessment_time_scale_id = "OTHER-SCALE".to_owned();
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::TimeScaleMismatch,
    );
}

#[test]
fn contradictory_independent_evidence_is_not_majority_voted_into_a_bound() {
    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().independence =
        IndependenceAssessment::Contradictory;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::IndependenceNotQualified,
    );
}

#[test]
fn frame_datum_profile_and_digest_mismatches_fail_closed() {
    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().horizontal_frame_id = "OTHER-FRAME".to_owned();
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::FrameOrDatumMismatch,
    );

    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().profile_id = "other-profile".to_owned();
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::ProfileMismatch,
    );

    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().source_claim_sha256 = "not-a-digest".to_owned();
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::InvalidDigest,
    );
}

#[test]
fn widening_sample_time_interval_cannot_shrink_the_position_enclosure() {
    let mut narrow_input = input(PntServiceState::Normal);
    let narrow_claim = narrow_input.primary.as_mut().unwrap();
    narrow_claim.sample_time_lower_utc_ms = 950;
    narrow_claim.sample_time_upper_utc_ms = 1_050;
    let (_, narrow) = accepted(evaluate(&narrow_input));

    let mut wide_input = input(PntServiceState::Normal);
    let wide_claim = wide_input.primary.as_mut().unwrap();
    wide_claim.sample_time_lower_utc_ms = 800;
    wide_claim.sample_time_upper_utc_ms = 1_200;
    let (_, wide) = accepted(evaluate(&wide_input));

    assert!(wide.east_m.lower() <= narrow.east_m.lower());
    assert!(wide.east_m.upper() >= narrow.east_m.upper());
    assert!(wide.north_m.lower() <= narrow.north_m.lower());
    assert!(wide.north_m.upper() >= narrow.north_m.upper());
}


#[test]
fn widening_evaluation_time_interval_cannot_shrink_the_position_enclosure() {
    let mut narrow_input = input(PntServiceState::Normal);
    narrow_input.assessment_time_lower_utc_ms = 2_995;
    narrow_input.assessment_time_upper_utc_ms = 3_005;
    let (_, narrow) = accepted(evaluate(&narrow_input));

    let mut wide_input = input(PntServiceState::Normal);
    wide_input.assessment_time_lower_utc_ms = 2_990;
    wide_input.assessment_time_upper_utc_ms = 3_010;
    let (_, wide) = accepted(evaluate(&wide_input));

    assert!(wide.east_m.lower() <= narrow.east_m.lower());
    assert!(wide.east_m.upper() >= narrow.east_m.upper());
    assert!(wide.north_m.lower() <= narrow.north_m.lower());
    assert!(wide.north_m.upper() >= narrow.north_m.upper());
}

#[test]
fn reversed_evaluation_time_interval_is_rejected() {
    let mut sample = input(PntServiceState::Normal);
    sample.assessment_time_lower_utc_ms = 3_010;
    sample.assessment_time_upper_utc_ms = 2_990;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::InvalidAssessmentTimeInterval,
    );
}

#[test]
fn widening_velocity_bounds_cannot_shrink_the_position_enclosure() {
    let mut narrow_input = input(PntServiceState::Normal);
    narrow_input.primary.as_mut().unwrap().velocity.east_mps = iv(4.5, 4.8);
    let (_, narrow) = accepted(evaluate(&narrow_input));

    let mut wide_input = input(PntServiceState::Normal);
    wide_input.primary.as_mut().unwrap().velocity.east_mps = iv(3.0, 6.0);
    let (_, wide) = accepted(evaluate(&wide_input));

    assert!(wide.east_m.lower() <= narrow.east_m.lower());
    assert!(wide.east_m.upper() >= narrow.east_m.upper());
}

#[test]
fn invalid_independent_family_count_and_missing_clock_lineage_are_rejected() {
    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().independent_family_count = Some(1);
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::InsufficientIndependentFamilies,
    );

    let mut sample = input(PntServiceState::Normal);
    sample.primary.as_mut().unwrap().clock_correlation_id = None;
    assert_rejected(
        evaluate(&sample),
        NotAssessableReason::MissingClockCorrelation,
    );
}
