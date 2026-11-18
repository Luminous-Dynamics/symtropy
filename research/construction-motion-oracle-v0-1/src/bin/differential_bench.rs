use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, DesignIntentEntity, DispatchEnvelopeBinding, DispatchReadinessRequest,
    FieldAcceptanceProfile, FieldLayoutEntityType, OccupancyAssessmentReceipt, SiteActorRole,
    SiteEventEnvelope, SiteEventPayload, SiteStateLedger, SiteWorkPackage, Vec3,
    chord_collision, check_dispatch_readiness, endpoints_only_collision,
    verify_dispatch_readiness_receipt_current,
};

const BATCH_SIZE: usize = 10_000;
const SAMPLES: usize = 101;

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index]
}

fn measure(label: &str, mut operation: impl FnMut()) {
    let mut durations = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        for _ in 0..BATCH_SIZE {
            operation();
        }
        let elapsed = start.elapsed();
        durations.push(elapsed / BATCH_SIZE as u32);
    }
    durations.sort_unstable();
    println!(
        "{label},p50_ns={},p95_ns={},p99_ns={}",
        percentile(&durations, 50).as_nanos(),
        percentile(&durations, 95).as_nanos(),
        percentile(&durations, 99).as_nanos()
    );
}

fn main() {
    let linear = AnalyticPath::Linear {
        start: Vec3::new(-1_000.0, 0.0, 0.0),
        end: Vec3::new(1_000.0, 0.0, 0.0),
    };
    let arc = AnalyticPath::UpperSemicircle {
        radius_mm: 1_000.0,
        z_mm: 0.0,
    };
    let linear_obstacle = Vec3::new(0.0, 0.0, 0.0);
    let arc_obstacle = Vec3::new(0.0, 1_000.0, 0.0);

    println!("case,metric,value");
    println!(
        "line-thin-barrier,analytic_collision,{}",
        black_box(linear).collides_with_stationary_sphere(
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        )
    );
    println!(
        "line-thin-barrier,endpoint_collision,{}",
        endpoints_only_collision(
            black_box(linear),
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        )
    );
    println!(
        "arc-chord-miss,analytic_collision,{}",
        black_box(arc).collides_with_stationary_sphere(
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        )
    );
    println!(
        "arc-chord-miss,chord_collision,{}",
        chord_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        )
    );

    measure("linear-analytic-ccd", || {
        black_box(linear).collides_with_stationary_sphere(
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        );
    });
    measure("linear-endpoint-only", || {
        endpoints_only_collision(
            black_box(linear),
            black_box(10.0),
            black_box(linear_obstacle),
            black_box(5.0),
        );
    });
    measure("semicircle-analytic-ccd", || {
        black_box(arc).collides_with_stationary_sphere(
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });
    measure("semicircle-endpoint-only", || {
        endpoints_only_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });
    measure("semicircle-chord-ccd", || {
        chord_collision(
            black_box(arc),
            black_box(20.0),
            black_box(arc_obstacle),
            black_box(20.0),
        );
    });
    run_site_ledger_append_scaling_benchmarks();
    run_site_dispatch_readiness_benchmarks();
}


fn run_site_ledger_append_scaling_benchmarks() {
    println!("site_ledger_append_scaling,event_count,rounds,total_p50_us,total_p95_us,total_p99_us,ns_per_event_p50,ns_per_event_p95,ns_per_event_p99");
    for event_count in [100_u64, 1_000, 10_000] {
        let mut round_durations = Vec::with_capacity(31);
        for _round in 0..31 {
            let mut ledger = SiteStateLedger::new();
            let package_sha256 = "a".repeat(64);
            let package = SiteWorkPackage {
                package_sha256: package_sha256.clone(),
                source_model_sha256: "b".repeat(64),
                source_schema_release: "IFC4X3_ADD2".into(),
                ids_spec_sha256: "c".repeat(64),
                ids_version: "1.0".into(),
                ids_engine_build_sha256: "5".repeat(64),
                design_revision_sha256: "d".repeat(64),
                frame_id: "benchmark-site-frame".into(),
                frame_sha256: "e".repeat(64),
                transform_sha256: "f".repeat(64),
                entity_manifest_sha256: "1".repeat(64),
                acceptance_profile_sha256: "2".repeat(64),
                entities: vec![DesignIntentEntity {
                    design_entity_id: "ifc4x3:0000000000000000000001".into(),
                    entity_type: FieldLayoutEntityType::Point,
                }],
                acceptance_profile: FieldAcceptanceProfile {
                    max_abs_xy_residual_mm: 1.0,
                    max_reference_u95_xy_mm: 0.25,
                    max_abs_z_residual_mm: None,
                    max_reference_u95_z_mm: None,
                    max_abs_endpoint_residual_mm: None,
                },
            };
            ledger
                .apply(SiteEventEnvelope {
                    event_id: "package-publish".into(),
                    sequence: 1,
                    previous_event_sha256: None,
                    event_sha256: "3".repeat(64),
                    actor_id: "design-authority".into(),
                    actor_role: SiteActorRole::DesignAuthority,
                    payload: SiteEventPayload::WorkPackagePublished { package },
                })
                .expect("benchmark package publication must be valid");

            let start = Instant::now();
            for index in 0..event_count {
                let sequence = index + 2;
                let event = SiteEventEnvelope {
                    event_id: format!("station-{sequence:08}"),
                    sequence,
                    previous_event_sha256: ledger.last_event_sha256().map(str::to_owned),
                    event_sha256: format!("{sequence:064x}"),
                    actor_id: "surveyor".into(),
                    actor_role: SiteActorRole::Surveyor,
                    payload: SiteEventPayload::StationQualified {
                        package_sha256: package_sha256.clone(),
                        station_id: format!("station-{sequence:08}"),
                        frame_sha256: "e".repeat(64),
                        transform_sha256: "f".repeat(64),
                        calibration_sha256: "4".repeat(64),
                    },
                };
                black_box(
                    ledger
                        .apply(event)
                        .expect("benchmark station qualification must be valid"),
                );
            }
            round_durations.push(start.elapsed());
            assert_eq!(ledger.event_count() as u64, event_count + 1);
            assert_eq!(ledger.last_sequence(), event_count + 1);
        }
        round_durations.sort_unstable();
        let p50 = percentile(&round_durations, 50);
        let p95 = percentile(&round_durations, 95);
        let p99 = percentile(&round_durations, 99);
        println!(
            "{},{},{},{},{},{},{},{},{}",
            event_count,
            round_durations.len(),
            p50.as_micros(),
            p95.as_micros(),
            p99.as_micros(),
            p50.as_nanos() / u128::from(event_count),
            p95.as_nanos() / u128::from(event_count),
            p99.as_nanos() / u128::from(event_count),
        );
    }
}


fn run_site_dispatch_readiness_benchmarks() {
    let package_sha256 = "a".repeat(64);
    let mut ledger = SiteStateLedger::new();
    let package = SiteWorkPackage {
        package_sha256: package_sha256.clone(),
        source_model_sha256: "b".repeat(64),
        source_schema_release: "IFC4X3_ADD2".into(),
        ids_spec_sha256: "c".repeat(64),
        ids_version: "1.0".into(),
        ids_engine_build_sha256: "5".repeat(64),
        design_revision_sha256: "d".repeat(64),
        frame_id: "benchmark-site-frame".into(),
        frame_sha256: "e".repeat(64),
        transform_sha256: "f".repeat(64),
        entity_manifest_sha256: "1".repeat(64),
        acceptance_profile_sha256: "2".repeat(64),
        entities: vec![DesignIntentEntity {
            design_entity_id: "ifc4x3:0000000000000000000001".into(),
            entity_type: FieldLayoutEntityType::Point,
        }],
        acceptance_profile: FieldAcceptanceProfile {
            max_abs_xy_residual_mm: 1.0,
            max_reference_u95_xy_mm: 0.25,
            max_abs_z_residual_mm: None,
            max_reference_u95_z_mm: None,
            max_abs_endpoint_residual_mm: None,
        },
    };
    ledger
        .apply(SiteEventEnvelope {
            event_id: "package-publish".into(),
            sequence: 1,
            previous_event_sha256: None,
            event_sha256: "3".repeat(64),
            actor_id: "design-authority".into(),
            actor_role: SiteActorRole::DesignAuthority,
            payload: SiteEventPayload::WorkPackagePublished { package },
        })
        .expect("dispatch benchmark package must publish");

    let request = DispatchReadinessRequest {
        schema_version: "construction-dispatch-readiness-request/0.1".into(),
        envelope: DispatchEnvelopeBinding {
            package_sha256,
            design_revision_sha256: "d".repeat(64),
            entity_id: "ifc4x3:0000000000000000000001".into(),
            frame_sha256: "e".repeat(64),
            transform_sha256: "f".repeat(64),
            robot_id: "robot-benchmark".into(),
            robot_configuration_sha256: "4".repeat(64),
            tool_configuration_sha256: "5".repeat(64),
            payload_configuration_sha256: "6".repeat(64),
            primitive_manifest_sha256: "7".repeat(64),
            trajectory_sha256: "8".repeat(64),
            envelope_sha256: "9".repeat(64),
            site_clock_domain_id: "site-clock-benchmark".into(),
            generated_at_tick: 10_000,
            valid_until_tick: 20_000,
        },
        trusted_occupancy_engine_build_sha256: "a".repeat(64),
        required_clearance_policy_sha256: "b".repeat(64),
        occupancy: OccupancyAssessmentReceipt {
            assessed_envelope_sha256: "9".repeat(64),
            primitive_manifest_sha256: "7".repeat(64),
            site_snapshot_sha256: "c".repeat(64),
            site_ledger_sequence: ledger.last_sequence(),
            site_ledger_event_sha256: ledger.last_event_sha256().unwrap().to_owned(),
            dynamic_occupancy_manifest_sha256: "d".repeat(64),
            actor_manifest_sha256: "e".repeat(64),
            occupancy_engine_build_sha256: "a".repeat(64),
            clearance_policy_sha256: "b".repeat(64),
            site_clock_domain_id: "site-clock-benchmark".into(),
            checked_at_tick: 10_100,
            valid_until_tick: 19_000,
            primitive_coverage_complete: true,
            dynamic_actor_coverage_complete: true,
            broadphase_pair_coverage_complete: true,
            narrowphase_coverage_complete: true,
            candidate_pair_count: 0,
            narrowphase_evaluated_pair_count: 0,
            conflict_count: 0,
            unresolved_pair_count: 0,
            invalid_bound_count: 0,
            unqualified_bound_count: 0,
            stale_actor_count: 0,
            unmodelled_occupant_count: 0,
            assessment_receipt_sha256: "f".repeat(64),
        },
        now_tick: 10_200,
    };
    let readiness = check_dispatch_readiness(&ledger, &request)
        .expect("synthetic dispatch readiness benchmark must be valid");

    measure("site-dispatch-readiness-check", || {
        black_box(
            check_dispatch_readiness(black_box(&ledger), black_box(&request))
                .expect("readiness check remains valid"),
        );
    });

    measure("site-dispatch-readiness-consume-revalidation", || {
        black_box(
            verify_dispatch_readiness_receipt_current(
                black_box(&ledger),
                black_box(&readiness),
                black_box(&request.trusted_occupancy_engine_build_sha256),
                black_box(&request.required_clearance_policy_sha256),
                black_box(request.now_tick + 1),
            )
            .expect("consume-time revalidation remains valid"),
        );
    });
}
