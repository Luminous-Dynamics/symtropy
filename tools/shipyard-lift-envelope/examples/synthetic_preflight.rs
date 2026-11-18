use shipyard_lift_envelope::{
    evaluate, CraneChartCell, EvidenceRef, EvidenceState, LiftPlan, MotionOccupancyBinding,
    RiggingLeg, WeatherEnvelope,
};

fn evidence(
    record_id: &str,
    subject_id: &str,
    subject_revision: &str,
) -> EvidenceRef {
    EvidenceRef {
        source_record_id: record_id.to_owned(),
        owner_id: format!("synthetic-owner::{record_id}"),
        revision: "r1".to_owned(),
        subject_id: subject_id.to_owned(),
        subject_revision: subject_revision.to_owned(),
        sha256: "a".repeat(64),
        validity_clock_domain_id: "yard-monotonic-ms".to_owned(),
        valid_from_tick: 0,
        valid_until_tick: 1_000,
        state: EvidenceState::Current,
    }
}

fn synthetic_plan() -> LiftPlan {
    LiftPlan {
        lift_id: "synthetic-lift-001".to_owned(),
        block_id: "synthetic-block-001".to_owned(),
        block_revision: "block-r7".to_owned(),
        site_epoch_id: "synthetic-yard-epoch-3".to_owned(),
        frame_id: "synthetic-yard-frame-mm".to_owned(),
        clock_domain_id: "yard-monotonic-ms".to_owned(),
        start_tick: 100,
        end_tick: 200,
        mass_upper_kg: 40_000,
        dynamic_factor_ppm: 1_100_000,
        dynamic_factor_profile_id: "synthetic-dynamic-profile-01".to_owned(),
        dynamic_factor_profile_revision: "r1".to_owned(),
        mass_evidence: evidence(
            "synthetic-mass-record-001",
            "synthetic-block-001",
            "block-r7",
        ),
        centre_of_gravity_envelope_evidence: evidence(
            "synthetic-cog-envelope-001",
            "synthetic-block-001",
            "block-r7",
        ),
        pick_point_and_load_share_evidence: evidence(
            "synthetic-pick-load-share-001",
            "synthetic-block-001",
            "block-r7",
        ),
        dynamic_factor_evidence: evidence(
            "synthetic-dynamic-factor-001",
            "synthetic-dynamic-profile-01",
            "r1",
        ),
        crane_id: "synthetic-gantry-01".to_owned(),
        crane_configuration_revision: "gantry-config-r4".to_owned(),
        crane_chart: CraneChartCell {
            crane_id: "synthetic-gantry-01".to_owned(),
            configuration_revision: "gantry-config-r4".to_owned(),
            radius_band_id: "radius-band-10m".to_owned(),
            working_quadrant_id: "quadrant-A".to_owned(),
            net_capacity_mn: 500_000_000,
            record_id: "synthetic-gantry-chart-cell-001".to_owned(),
            evidence: evidence(
                "synthetic-gantry-chart-cell-001",
                "synthetic-gantry-01",
                "gantry-config-r4",
            ),
        },
        crane_radius_band_id: "radius-band-10m".to_owned(),
        crane_working_quadrant_id: "quadrant-A".to_owned(),
        rigging_legs: vec![
            RiggingLeg {
                leg_id: "synthetic-leg-A".to_owned(),
                component_revision: "leg-A-r1".to_owned(),
                maximum_vertical_share_ppm: 500_000,
                minimum_sine_angle_ppm: 707_107,
                working_load_limit_mn: 350_000_000,
                wll_evidence: evidence(
                    "synthetic-wll-leg-A",
                    "synthetic-leg-A",
                    "leg-A-r1",
                ),
            },
            RiggingLeg {
                leg_id: "synthetic-leg-B".to_owned(),
                component_revision: "leg-B-r1".to_owned(),
                maximum_vertical_share_ppm: 500_000,
                minimum_sine_angle_ppm: 707_107,
                working_load_limit_mn: 350_000_000,
                wll_evidence: evidence(
                    "synthetic-wll-leg-B",
                    "synthetic-leg-B",
                    "leg-B-r1",
                ),
            },
        ],
        weather: WeatherEnvelope {
            gust_upper_mm_per_s: 8_000,
            maximum_gust_mm_per_s: 10_000,
            observation_subject_id: "synthetic-weather-station-01".to_owned(),
            observation_subject_revision: "station-config-r3".to_owned(),
            limit_profile_id: "synthetic-wind-limit-profile-01".to_owned(),
            limit_profile_revision: "r1".to_owned(),
            observation_evidence: evidence(
                "synthetic-wind-observation-001",
                "synthetic-weather-station-01",
                "station-config-r3",
            ),
            operating_limit_evidence: evidence(
                "synthetic-wind-limit-001",
                "synthetic-wind-limit-profile-01",
                "r1",
            ),
        },
        motion: MotionOccupancyBinding {
            motion_report_id: "synthetic-motion-report-001".to_owned(),
            motion_report_evidence: evidence(
                "synthetic-motion-report-001",
                "synthetic-block-001",
                "block-r7",
            ),
            occupancy_projection_id: "synthetic-occupancy-projection-001".to_owned(),
            occupancy_evidence: evidence(
                "synthetic-occupancy-projection-001",
                "synthetic-block-001",
                "block-r7",
            ),
            block_id: "synthetic-block-001".to_owned(),
            block_revision: "block-r7".to_owned(),
            payload_id: "synthetic-block-001".to_owned(),
            site_epoch_id: "synthetic-yard-epoch-3".to_owned(),
            frame_id: "synthetic-yard-frame-mm".to_owned(),
            clock_domain_id: "yard-monotonic-ms".to_owned(),
            occupancy_start_tick: 90,
            occupancy_end_tick: 210,
        },
    }
}

fn main() {
    let plan = synthetic_plan();
    let report = evaluate(&plan);

    // The example asserts only the synthetic profile's human-review disposition.
    // It never emits a lift permit, safe-to-lift claim or actuator command.
    assert_eq!(
        report.disposition,
        shipyard_lift_envelope::Disposition::ReadyForHumanReview,
        "synthetic preflight fixture must remain human-review-only"
    );

    // Exact one-line canonical input followed by exact one-line canonical report.
    println!("{}", plan.canonical_json());
    println!("{}", report.canonical_json());
}
