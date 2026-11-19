use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    AnalyticPath, DesignIntentEntity, EvidenceDependency, EvidenceDependencyGraph, EvidenceNode,
    EvidenceNodeKind, EvidenceRevisionChange, FieldAcceptanceProfile, FieldLayoutEntityType,
    SiteActorRole, SiteEventEnvelope, SiteEventPayload, SiteStateLedger, SiteWorkPackage, Vec3,
    chord_collision, endpoints_only_collision,
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
    run_dependency_invalidation_scaling_benchmarks();
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


fn build_dependency_forest(node_count: usize, component_count: usize) -> EvidenceDependencyGraph {
    assert!(component_count > 0);
    assert!(node_count >= component_count);
    assert_eq!(node_count % component_count, 0);
    let component_size = node_count / component_count;
    let mut nodes = Vec::with_capacity(node_count);
    let mut dependencies = Vec::with_capacity(node_count - component_count);

    for component in 0..component_count {
        for offset in 0..component_size {
            let kind = match offset {
                0 => EvidenceNodeKind::SourceModel,
                1 => EvidenceNodeKind::SourceEntity,
                2 => EvidenceNodeKind::IdsEvaluation,
                3 => EvidenceNodeKind::WorkPackage,
                4 => EvidenceNodeKind::TaskPlan,
                5 => EvidenceNodeKind::MotionEnvelope,
                6 => EvidenceNodeKind::ExecutionEvidence,
                7 => EvidenceNodeKind::InspectionEvidence,
                _ => EvidenceNodeKind::AcceptedOutput,
            };
            let node_id = format!("component-{component:02}-node-{offset:06}");
            nodes.push(EvidenceNode {
                node_id: node_id.clone(),
                kind,
                version_sha256: format!("{:064x}", component * component_size + offset + 1),
            });
            if offset > 0 {
                dependencies.push(EvidenceDependency {
                    dependent_node_id: node_id,
                    prerequisite_node_id: format!("component-{component:02}-node-{:06}", offset - 1),
                });
            }
        }
    }

    EvidenceDependencyGraph::new(nodes, dependencies)
        .expect("synthetic benchmark dependency forest must be valid")
}

fn run_dependency_invalidation_scaling_benchmarks() {
    println!(
        "dependency_invalidation,topology,nodes,components,changed_roots,affected_dependents,unaffected_nodes,global_invalidation_baseline,scoped_vs_global_pct,plan_p50_us,plan_p95_us,plan_p99_us"
    );

    for node_count in [128_usize, 512, 2_048, 8_192] {
        const COMPONENTS: usize = 8;
        let mut graph_build_durations = Vec::with_capacity(31);
        for _ in 0..31 {
            let start = Instant::now();
            black_box(build_dependency_forest(node_count, COMPONENTS));
            graph_build_durations.push(start.elapsed());
        }
        graph_build_durations.sort_unstable();

        let graph = build_dependency_forest(node_count, COMPONENTS);
        let change = EvidenceRevisionChange {
            node_id: "component-00-node-000000".into(),
            new_version_sha256: "f".repeat(64),
            reason: "synthetic source-model revision".into(),
        };
        let mut plan_durations = Vec::with_capacity(101);
        let mut affected_count = 0_usize;
        let mut unaffected_count = 0_usize;

        for _ in 0..101 {
            let start = Instant::now();
            let plan = graph
                .plan_revision(std::slice::from_ref(&change))
                .expect("synthetic revision must yield an invalidation plan");
            plan_durations.push(start.elapsed());
            affected_count = plan.requalification_order.len();
            unaffected_count = plan.unaffected_nodes.len();
            black_box(plan);
        }

        plan_durations.sort_unstable();
        let p50 = percentile(&plan_durations, 50);
        let p95 = percentile(&plan_durations, 95);
        let p99 = percentile(&plan_durations, 99);
        let global_baseline = node_count - 1;
        let scoped_percentage = (affected_count as f64 / global_baseline as f64) * 100.0;
        let build_p50 = percentile(&graph_build_durations, 50);
        println!(
            "forest,{},{},{},{},{},{},{},{:.2},{},{},{},graph_build_p50_us={}",
            "8-independent-work-packages",
            node_count,
            COMPONENTS,
            1,
            affected_count,
            unaffected_count,
            global_baseline,
            scoped_percentage,
            p50.as_micros(),
            p95.as_micros(),
            p99.as_micros(),
            build_p50.as_micros()
        );
    }
}
