use aeroweift_surveillance_observability_oracle::{
    evaluate, CoverageClaim, CoverageStatus, Observation, ObservationStatus,
    ObservabilityAssessment, ObservabilityError, SensorModality, TrafficClass,
};

const GOLDEN_FIXTURE: &str = include_str!(
    "../../../fixtures/aeroweift-surveillance-observability/tiny-coverage-gap-v0.json"
);

fn claim(
    claim_id: &'static str,
    source_id: &'static str,
    modality: SensorModality,
    traffic_class: TrafficClass,
    cell_id: &'static str,
    family: Option<&'static str>,
) -> CoverageClaim {
    CoverageClaim {
        claim_id,
        source_id,
        modality,
        traffic_class,
        cell_id,
        start_tick: 0,
        end_tick: 10,
        coverage_qualified: true,
        environment_qualified: true,
        upstream_degraded: false,
        pnt_time_qualified: true,
        dependency_inventory_complete: true,
        independent_family_id: family,
    }
}

fn observation(
    source_id: &'static str,
    traffic_class: TrafficClass,
    cell_id: &'static str,
    tick: u16,
) -> Observation {
    Observation {
        observation_id: "synthetic-track",
        source_id,
        traffic_class,
        cell_id,
        tick,
        current: true,
        integrity_qualified: true,
    }
}

fn assessment(
    claims: &[CoverageClaim],
    observation: Option<&Observation>,
    traffic_class: TrafficClass,
    cell_id: &str,
    tick: u16,
    minimum_families: usize,
) -> ObservabilityAssessment {
    evaluate(
        claims,
        observation,
        traffic_class,
        cell_id,
        tick,
        minimum_families,
    )
    .expect("fixture policy must be valid")
}

#[test]
fn golden_fixture_identity_binds_the_coverage_gap_cases() {
    assert!(GOLDEN_FIXTURE.contains("AW-SURV-OBS-TINY-001"));
    assert!(GOLDEN_FIXTURE.contains("adsb-only-does-not-cover-noncooperative-crewed-traffic"));
    assert!(GOLDEN_FIXTURE.contains("three-receivers-share-one-critical-failure-domain"));
    assert!(GOLDEN_FIXTURE.contains("missing-report-no-independent-sensor"));
    assert!(GOLDEN_FIXTURE.contains("positive-control-separate-cooperative-and-noncooperative-coverage"));
}

#[test]
fn cooperative_track_requires_scoped_independent_coverage() {
    let claims = [
        claim(
            "adsb-a",
            "receiver-a",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("family-east"),
        ),
        claim(
            "adsb-b",
            "receiver-b",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("family-west"),
        ),
    ];
    let report = observation(
        "receiver-a",
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
    );

    assert_eq!(
        assessment(
            &claims,
            Some(&report),
            TrafficClass::CooperativeCrewed,
            "cell-a",
            5,
            2
        ),
        ObservabilityAssessment {
            coverage_status: CoverageStatus::QualifiedCoverage,
            observation_status: ObservationStatus::QualifiedTrack,
            independent_family_count: 2,
            unresolved_possible_occupancy: false,
        }
    );
}

#[test]
fn missing_report_outside_coverage_is_not_empty_airspace() {
    let claims = [claim(
        "other-cell",
        "receiver-a",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-a"),
    )];

    let result = assessment(
        &claims,
        None,
        TrafficClass::CooperativeCrewed,
        "cell-b",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(result.observation_status, ObservationStatus::NoObservation);
    assert!(result.unresolved_possible_occupancy);
}


#[test]
fn no_report_even_inside_qualified_coverage_is_not_an_empty_sky() {
    let claims = [
        claim(
            "receiver-a",
            "rx-a",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("family-a"),
        ),
        claim(
            "receiver-b",
            "rx-b",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("family-b"),
        ),
    ];
    let result = assessment(
        &claims,
        None,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        2,
    );
    assert_eq!(result.coverage_status, CoverageStatus::QualifiedCoverage);
    assert_eq!(result.observation_status, ObservationStatus::NoObservation);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn adsb_only_claim_never_qualifies_noncooperative_traffic() {
    let claims = [claim(
        "cooperative-only",
        "adsb-rx",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-adsb"),
    )];

    let result = assessment(
        &claims,
        None,
        TrafficClass::NonCooperativeCrewed,
        "cell-a",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(result.observation_status, ObservationStatus::NoObservation);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn eo_ir_track_is_valid_only_under_its_declared_environment_claim() {
    let claim = claim(
        "eoir-day",
        "eoir-1",
        SensorModality::Eoir,
        TrafficClass::NonCooperativeCrewed,
        "cell-day",
        Some("family-eoir"),
    );
    let report = observation(
        "eoir-1",
        TrafficClass::NonCooperativeCrewed,
        "cell-day",
        5,
    );

    let result = assessment(
        &[claim],
        Some(&report),
        TrafficClass::NonCooperativeCrewed,
        "cell-day",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::QualifiedCoverage);
    assert_eq!(result.observation_status, ObservationStatus::QualifiedTrack);
    assert!(!result.unresolved_possible_occupancy);

    let mut out_of_profile = claim;
    out_of_profile.environment_qualified = false;
    let result = assessment(
        &[out_of_profile],
        Some(&report),
        TrafficClass::NonCooperativeCrewed,
        "cell-day",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(result.observation_status, ObservationStatus::InvalidOrStaleTrack);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn receiver_count_with_one_common_failure_family_is_only_partial_coverage() {
    let claims = [
        claim(
            "receiver-1",
            "rx-1",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("shared-time-power-network"),
        ),
        claim(
            "receiver-2",
            "rx-2",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("shared-time-power-network"),
        ),
        claim(
            "receiver-3",
            "rx-3",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            Some("shared-time-power-network"),
        ),
    ];

    let result = assessment(
        &claims,
        None,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        2,
    );
    assert_eq!(result.coverage_status, CoverageStatus::PartialCoverage);
    assert_eq!(result.independent_family_count, 1);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn unknown_tdoa_time_dependency_is_indeterminate_not_independent() {
    let mut tdoa_a = claim(
        "tdoa-a",
        "mlat-a",
        SensorModality::TdoaMlat,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        None,
    );
    tdoa_a.dependency_inventory_complete = false;

    let mut tdoa_b = claim(
        "tdoa-b",
        "mlat-b",
        SensorModality::TdoaMlat,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        None,
    );
    tdoa_b.dependency_inventory_complete = false;

    let result = assessment(
        &[tdoa_a, tdoa_b],
        None,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        2,
    );
    assert_eq!(result.coverage_status, CoverageStatus::Indeterminate);
    assert_eq!(result.independent_family_count, 0);
    assert!(result.unresolved_possible_occupancy);
}


#[test]
fn explicitly_degraded_sensor_service_is_not_full_coverage() {
    let mut degraded = claim(
        "degraded-adsb",
        "receiver-degraded",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-degraded",
        Some("family-degraded"),
    );
    degraded.upstream_degraded = true;

    let result = assessment(
        &[degraded],
        None,
        TrafficClass::CooperativeCrewed,
        "cell-degraded",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::Degraded);
    assert_eq!(result.observation_status, ObservationStatus::NoObservation);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn pnt_time_loss_invalidates_a_previously_qualified_report() {
    let mut claim = claim(
        "adsb-time-loss",
        "receiver-time-uncertain",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-a"),
    );
    claim.pnt_time_qualified = false;
    let report = observation(
        "receiver-time-uncertain",
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
    );

    let result = assessment(
        &[claim],
        Some(&report),
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::Indeterminate);
    assert_eq!(result.observation_status, ObservationStatus::InvalidOrStaleTrack);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn no_report_with_no_claim_is_never_an_empty_set() {
    let result = assessment(
        &[],
        None,
        TrafficClass::Unknown,
        "cell-a",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(result.observation_status, ObservationStatus::NoObservation);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn positive_control_accepts_two_classes_separately() {
    let claims = [
        claim(
            "adsb-coop",
            "adsb-positive",
            SensorModality::Adsb,
            TrafficClass::CooperativeCrewed,
            "cell-positive",
            Some("family-adsb"),
        ),
        claim(
            "eoir-noncoop",
            "eoir-positive",
            SensorModality::Eoir,
            TrafficClass::NonCooperativeCrewed,
            "cell-positive",
            Some("family-eoir"),
        ),
    ];
    let coop = observation(
        "adsb-positive",
        TrafficClass::CooperativeCrewed,
        "cell-positive",
        3,
    );
    let noncoop = observation(
        "eoir-positive",
        TrafficClass::NonCooperativeCrewed,
        "cell-positive",
        3,
    );

    let coop_result = assessment(
        &claims,
        Some(&coop),
        TrafficClass::CooperativeCrewed,
        "cell-positive",
        3,
        1,
    );
    let noncoop_result = assessment(
        &claims,
        Some(&noncoop),
        TrafficClass::NonCooperativeCrewed,
        "cell-positive",
        3,
        1,
    );

    assert_eq!(coop_result.coverage_status, CoverageStatus::QualifiedCoverage);
    assert_eq!(noncoop_result.coverage_status, CoverageStatus::QualifiedCoverage);
    assert_eq!(coop_result.observation_status, ObservationStatus::QualifiedTrack);
    assert_eq!(noncoop_result.observation_status, ObservationStatus::QualifiedTrack);
}

#[test]
fn stale_report_and_expired_half_open_claim_are_not_accepted() {
    let claim = claim(
        "short-claim",
        "receiver-a",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-a"),
    );
    let mut expired = claim;
    expired.end_tick = 5;
    let report = observation(
        "receiver-a",
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
    );

    let result = assessment(
        &[expired],
        Some(&report),
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        1,
    );
    assert_eq!(result.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(result.observation_status, ObservationStatus::InvalidOrStaleTrack);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn insufficient_families_and_unknown_dependency_cannot_promote_to_qualified() {
    let single = claim(
        "single",
        "receiver-a",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-a"),
    );
    let mut unknown = claim(
        "unknown",
        "receiver-b",
        SensorModality::Adsb,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        Some("family-b"),
    );
    unknown.dependency_inventory_complete = false;

    let result = assessment(
        &[single, unknown],
        None,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        2,
    );
    assert_eq!(result.coverage_status, CoverageStatus::Indeterminate);
    assert!(result.unresolved_possible_occupancy);
}

#[test]
fn invalid_policy_is_rejected_and_unknown_class_remains_unresolved() {
    assert_eq!(
        evaluate(&[], None, TrafficClass::CooperativeCrewed, "cell-a", 1, 0),
        Err(ObservabilityError::InvalidMinimumIndependentFamilies)
    );
    let unknown = assessment(&[], None, TrafficClass::Unknown, "cell-a", 1, 1);
    assert_eq!(unknown.coverage_status, CoverageStatus::NotQualified);
    assert_eq!(unknown.observation_status, ObservationStatus::NoObservation);
    assert!(unknown.unresolved_possible_occupancy);
}

#[test]
fn input_order_permutation_preserves_semantic_assessment() {
    let claims = [
        claim("one", "rx1", SensorModality::Adsb, TrafficClass::CooperativeCrewed, "cell-a", Some("family-1")),
        claim("two", "rx2", SensorModality::Adsb, TrafficClass::CooperativeCrewed, "cell-a", Some("family-2")),
    ];
    let baseline = assessment(
        &claims,
        None,
        TrafficClass::CooperativeCrewed,
        "cell-a",
        5,
        2,
    );
    let reversed = [claims[1], claims[0]];
    assert_eq!(
        baseline,
        assessment(
            &reversed,
            None,
            TrafficClass::CooperativeCrewed,
            "cell-a",
            5,
            2
        )
    );
}
