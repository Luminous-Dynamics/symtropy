use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, DesignIntentEntity, FieldAcceptanceProfile, FieldLayoutEntityType,
    GeometryPreflightEvaluation, IdsRequirementEvaluation, IfcCoordinateContext,
    IfcIdsWorkPackageInput, IfcLengthUnit, IfcSourceEntity, SiteActorRegistry, SiteActorRole,
    SiteEventEnvelope, SiteEventPayload, SiteStateLedger, SiteWorkPackage, Vec3,
    SUPPORTED_IDS_VERSION, SUPPORTED_IFC_SCHEMA_RELEASE, chord_collision,
    endpoints_only_collision, preflight_ifc_ids_work_package,
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
    run_ifc_ids_preflight_scaling_benchmarks();
}


fn run_site_ledger_append_scaling_benchmarks() {
    println!("site_ledger_scaling,station_events,total_events,rounds,append_p50_us,append_p95_us,append_p99_us,replay_p50_us,replay_p95_us,replay_p99_us,append_ns_per_event_p50,replay_ns_per_event_p50");
    for station_event_count in [100_u64, 1_000, 10_000] {
        let package_sha256 = "a".repeat(64);
        let package = SiteWorkPackage {
            package_sha256: package_sha256.clone(),
            source_model_sha256: "b".repeat(64),
            source_schema_release: SUPPORTED_IFC_SCHEMA_RELEASE.into(),
            ids_spec_sha256: "c".repeat(64),
            ids_version: SUPPORTED_IDS_VERSION.into(),
            ids_engine_build_sha256: "5".repeat(64),
            coordinate_context_sha256: "6".repeat(64),
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
        let mut registry = SiteActorRegistry::new();
        registry
            .grant("design-authority", SiteActorRole::DesignAuthority)
            .expect("benchmark publisher role must be valid");
        registry
            .grant("surveyor", SiteActorRole::Surveyor)
            .expect("benchmark surveyor role must be valid");

        let mut events = Vec::with_capacity(station_event_count as usize + 1);
        events.push(SiteEventEnvelope {
            event_id: "package-publish".into(),
            sequence: 1,
            previous_event_sha256: None,
            event_sha256: "3".repeat(64),
            actor_id: "design-authority".into(),
            actor_role: SiteActorRole::DesignAuthority,
            payload: SiteEventPayload::WorkPackagePublished {
                package: package.clone(),
            },
        });
        for index in 0..station_event_count {
            let sequence = index + 2;
            let previous_event_sha256 = if sequence == 2 {
                "3".repeat(64)
            } else {
                format!("{:064x}", sequence - 1)
            };
            events.push(SiteEventEnvelope {
                event_id: format!("station-{sequence:08}"),
                sequence,
                previous_event_sha256: Some(previous_event_sha256),
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
            });
        }

        let mut append_durations = Vec::with_capacity(21);
        let mut replay_durations = Vec::with_capacity(21);
        for _round in 0..21 {
            let mut ledger = SiteStateLedger::with_actor_registry(registry.clone());
            let start = Instant::now();
            for event in &events {
                black_box(
                    ledger
                        .apply(event.clone())
                        .expect("benchmark append event must be valid"),
                );
            }
            append_durations.push(start.elapsed());
            assert_eq!(ledger.event_count(), events.len());
            assert_eq!(ledger.last_sequence(), events.len() as u64);

            let start = Instant::now();
            let replayed = SiteStateLedger::replay(&events, registry.clone())
                .expect("benchmark persisted event stream must replay");
            replay_durations.push(start.elapsed());
            assert_eq!(replayed, ledger);
            assert_eq!(replayed.event_history(), events.as_slice());
        }

        append_durations.sort_unstable();
        replay_durations.sort_unstable();
        let append_p50 = percentile(&append_durations, 50);
        let append_p95 = percentile(&append_durations, 95);
        let append_p99 = percentile(&append_durations, 99);
        let replay_p50 = percentile(&replay_durations, 50);
        let replay_p95 = percentile(&replay_durations, 95);
        let replay_p99 = percentile(&replay_durations, 99);
        let total_events = events.len() as u128;
        println!(
            "{},{},{},{},{},{},{},{},{},{},{},{}",
            station_event_count,
            total_events,
            append_durations.len(),
            append_p50.as_micros(),
            append_p95.as_micros(),
            append_p99.as_micros(),
            replay_p50.as_micros(),
            replay_p95.as_micros(),
            replay_p99.as_micros(),
            append_p50.as_nanos() / total_events,
            replay_p50.as_nanos() / total_events,
        );
    }
}

fn build_ifc_ids_benchmark_input(entity_count: usize) -> IfcIdsWorkPackageInput {
    let digest = |value: u64| format!("{value:064x}");
    let frame_sha256 = digest(0x100);
    let transform_sha256 = digest(0x101);
    let geometry_validator_build = digest(0x102);
    let ids_engine_build = digest(0x103);
    let entities = (0..entity_count)
        .map(|index| {
            let entity_number = index as u64 + 1;
            let geometry_sha256 = digest(0x10_000 + entity_number);
            IfcSourceEntity {
                global_id: format!("{entity_number:022}"),
                entity_type: FieldLayoutEntityType::Point,
                geometry_sha256: geometry_sha256.clone(),
                required_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                delivered_ids_properties: vec!["Pset_Layout.MarkingClass".into()],
                ids_applicability_receipt_sha256: digest(0x20_000 + entity_number),
                applicable_ids_requirement_ids: vec!["layout-marking-class".into()],
                ids_requirement_evaluations: vec![IdsRequirementEvaluation {
                    requirement_id: "layout-marking-class".into(),
                    passed: true,
                    evaluation_receipt_sha256: digest(0x30_000 + entity_number),
                }],
                geometry_preflight: GeometryPreflightEvaluation {
                    geometry_sha256,
                    validator_build_sha256: geometry_validator_build.clone(),
                    frame_sha256: frame_sha256.clone(),
                    transform_sha256: transform_sha256.clone(),
                    placements_resolved: true,
                    geometry_valid: true,
                    coordinate_transform_valid: true,
                    clash_check_complete: true,
                    critical_clash_count: 0,
                    unresolved_geometry_issue_count: 0,
                    evaluation_receipt_sha256: digest(0x40_000 + entity_number),
                },
                in_work_scope: true,
            }
        })
        .collect();

    IfcIdsWorkPackageInput {
        package_sha256: digest(1),
        source_model_sha256: digest(2),
        source_schema_release: SUPPORTED_IFC_SCHEMA_RELEASE.into(),
        ids_spec_sha256: digest(3),
        ids_version: SUPPORTED_IDS_VERSION.into(),
        ids_engine_build_sha256: ids_engine_build,
        design_revision_sha256: digest(4),
        frame_id: "benchmark-site-frame".into(),
        frame_sha256: frame_sha256.clone(),
        transform_sha256: transform_sha256.clone(),
        coordinate_context_sha256: digest(5),
        coordinate_context: IfcCoordinateContext {
            model_frame_id: "benchmark-site-frame".into(),
            project_crs_identifier: "EPSG:32735".into(),
            vertical_datum_identifier: "benchmark-local-datum".into(),
            length_unit: IfcLengthUnit::Millimetre,
            model_to_site_rotation: [
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
            model_to_site_translation_mm: [0.0, 0.0, 0.0],
            map_conversion_scale: 1.0,
            frame_sha256,
            transform_sha256,
        },
        entity_manifest_sha256: digest(6),
        acceptance_profile_sha256: digest(7),
        acceptance_profile: FieldAcceptanceProfile {
            max_abs_xy_residual_mm: 1.0,
            max_reference_u95_xy_mm: 0.25,
            max_abs_z_residual_mm: None,
            max_reference_u95_z_mm: None,
            max_abs_endpoint_residual_mm: Some(1.0),
        },
        entities,
    }
}

fn run_ifc_ids_preflight_scaling_benchmarks() {
    println!("ifc_ids_preflight_scaling,entity_count,rounds,p50_us,p95_us,p99_us,entities_per_second_p50");
    for entity_count in [100_usize, 1_000, 10_000] {
        let input = build_ifc_ids_benchmark_input(entity_count);
        let mut durations = Vec::with_capacity(11);
        for _round in 0..11 {
            let sample = input.clone();
            let start = Instant::now();
            let report = preflight_ifc_ids_work_package(sample)
                .expect("synthetic IFC/IDS benchmark corpus must preflight");
            black_box(report);
            durations.push(start.elapsed());
        }
        durations.sort_unstable();
        let p50 = percentile(&durations, 50);
        let p95 = percentile(&durations, 95);
        let p99 = percentile(&durations, 99);
        let entities_per_second = if p50.as_nanos() == 0 {
            0
        } else {
            (entity_count as u128 * 1_000_000_000) / p50.as_nanos()
        };
        println!(
            "{},{},{},{},{},{}",
            entity_count,
            durations.len(),
            p50.as_micros(),
            p95.as_micros(),
            p99.as_micros(),
            entities_per_second
        );
    }
}
