use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    ArtifactDependencyContract, ArtifactPersistence, ConstructionArtifact,
    ConstructionArtifactGraph, ConstructionArtifactKind, ConstructionChangeSet,
    DependencyContractSet, DependencyDomain, DependencyKey, DependencyTransition,
    VersionedDependency, compile_change_impact_with_contract,
};

const SAMPLES: usize = 25;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn key(domain: DependencyDomain, scope_id: String) -> DependencyKey {
    DependencyKey { domain, scope_id }
}

fn build_graph(entity_count: usize) -> ConstructionArtifactGraph {
    let shared_transform = key(
        DependencyDomain::CoordinateTransform,
        "floor:02".to_owned(),
    );
    let mut artifacts = Vec::with_capacity(entity_count.saturating_mul(3));

    for index in 0..entity_count {
        let entity_id = format!("entity:{index:08}");
        let motion_id = format!("motion:{index:08}");
        let occupancy_id = format!("occupancy:{index:08}");
        let readiness_id = format!("readiness:{index:08}");

        artifacts.push(ConstructionArtifact {
            artifact_id: motion_id.clone(),
            artifact_sha256: digest('a'),
            kind: ConstructionArtifactKind::MotionEnvelope,
            persistence: ArtifactPersistence::Recomputable,
            dependencies: vec![
                VersionedDependency {
                    key: key(
                        DependencyDomain::EntityGeometry,
                        entity_id,
                    ),
                    observed_sha256: digest('1'),
                },
                VersionedDependency {
                    key: shared_transform.clone(),
                    observed_sha256: digest('2'),
                },
            ],
            depends_on_artifact_ids: Vec::new(),
        });
        artifacts.push(ConstructionArtifact {
            artifact_id: occupancy_id.clone(),
            artifact_sha256: digest('b'),
            kind: ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
            persistence: ArtifactPersistence::Recomputable,
            dependencies: vec![
                VersionedDependency {
                    key: key(DependencyDomain::ObstacleMap, format!("region:{index:08}")),
                    observed_sha256: digest('3'),
                },
                VersionedDependency {
                    key: key(DependencyDomain::DynamicActorManifest, format!("region:{index:08}")),
                    observed_sha256: digest('4'),
                },
                VersionedDependency {
                    key: key(DependencyDomain::SiteClockMap, "site".into()),
                    observed_sha256: digest('5'),
                },
                VersionedDependency {
                    key: key(DependencyDomain::OccupancyEngineBuild, "engine".into()),
                    observed_sha256: digest('6'),
                },
                VersionedDependency {
                    key: key(DependencyDomain::ClearancePolicy, "site".into()),
                    observed_sha256: digest('7'),
                },
            ],
            depends_on_artifact_ids: vec![motion_id],
        });
        artifacts.push(ConstructionArtifact {
            artifact_id: readiness_id,
            artifact_sha256: digest('c'),
            kind: ConstructionArtifactKind::DispatchReadiness,
            persistence: ArtifactPersistence::Recomputable,
            dependencies: vec![
                VersionedDependency {
                    key: key(DependencyDomain::WorkPackageSchedule, "work-package".into()),
                    observed_sha256: digest('8'),
                },
                VersionedDependency {
                    key: key(DependencyDomain::AuthorityPolicy, "site".into()),
                    observed_sha256: digest('9'),
                },
            ],
            depends_on_artifact_ids: vec![occupancy_id],
        });
    }

    ConstructionArtifactGraph {
        graph_snapshot_sha256: digest('f'),
        artifacts,
    }
}


fn dependency_contracts() -> DependencyContractSet {
    DependencyContractSet {
        contract_sha256: digest('c'),
        contracts: vec![
            ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::MotionEnvelope,
                required_domains: vec![
                    DependencyDomain::EntityGeometry,
                    DependencyDomain::CoordinateTransform,
                ],
            },
            ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
                required_domains: vec![
                    DependencyDomain::ObstacleMap,
                    DependencyDomain::DynamicActorManifest,
                    DependencyDomain::SiteClockMap,
                    DependencyDomain::OccupancyEngineBuild,
                    DependencyDomain::ClearancePolicy,
                ],
            },
            ArtifactDependencyContract {
                artifact_kind: ConstructionArtifactKind::DispatchReadiness,
                required_domains: vec![
                    DependencyDomain::WorkPackageSchedule,
                    DependencyDomain::AuthorityPolicy,
                ],
            },
        ],
    }
}

fn localized_change_set(entity_count: usize) -> ConstructionChangeSet {
    let changed_index = entity_count / 2;
    ConstructionChangeSet {
        change_id: "localized-entity-geometry-change".into(),
        source_event_sha256: digest('e'),
        changes: vec![DependencyTransition {
            key: key(
                DependencyDomain::EntityGeometry,
                format!("entity:{changed_index:08}"),
            ),
            previous_sha256: digest('1'),
            current_sha256: digest('4'),
        }],
    }
}

fn global_change_set() -> ConstructionChangeSet {
    ConstructionChangeSet {
        change_id: "shared-floor-transform-change".into(),
        source_event_sha256: digest('d'),
        changes: vec![DependencyTransition {
            key: key(
                DependencyDomain::CoordinateTransform,
                "floor:02".into(),
            ),
            previous_sha256: digest('2'),
            current_sha256: digest('5'),
        }],
    }
}

fn percentile(values: &[Duration], percentile: usize) -> Duration {
    let index = ((values.len() - 1) * percentile).div_ceil(100);
    values[index]
}

fn measure_case(
    entity_count: usize,
    case: &str,
    graph: &ConstructionArtifactGraph,
    contracts: &DependencyContractSet,
    change_set: &ConstructionChangeSet,
) {
    let expected_direct = if case == "localized-entity" {
        1
    } else {
        entity_count
    };
    let expected_transitive = expected_direct * 2;

    let check = compile_change_impact_with_contract(black_box(graph), black_box(contracts), black_box(change_set))
        .expect("benchmark graph and change set must be valid");
    assert_eq!(check.direct_invalidations.len(), expected_direct);
    assert_eq!(check.transitive_invalidations.len(), expected_transitive);
    assert!(check.unmatched_change_keys.is_empty());
    assert!(!check.source_records_mutated);

    // Warm up the exact input before timing.
    black_box(
        compile_change_impact_with_contract(
            black_box(graph),
            black_box(contracts),
            black_box(change_set),
        )
        .expect("warm-up graph, contract and change set must be valid"),
    );

    let mut durations = Vec::with_capacity(SAMPLES);
    let mut invalidated_counts = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        let report = compile_change_impact_with_contract(
            black_box(graph),
            black_box(contracts),
            black_box(change_set),
        )
        .expect("timed graph, contract and change set must be valid");
        durations.push(start.elapsed());
        invalidated_counts.push(report.invalidated_artifact_count());
    }
    durations.sort_unstable();
    assert!(invalidated_counts.iter().all(|count| {
        *count == expected_direct + expected_transitive
    }));

    println!(
        "{case},{entity_count},{},{},{},{},{},{},{}",
        entity_count * 3,
        expected_direct,
        expected_transitive,
        expected_direct + expected_transitive,
        percentile(&durations, 50).as_micros(),
        percentile(&durations, 95).as_micros(),
        percentile(&durations, 99).as_micros()
    );
}

fn main() {
    println!(
        "case,entities,artifacts,direct_invalidations,transitive_invalidations,total_invalidations,p50_us,p95_us,p99_us"
    );
    for entity_count in [1_000_usize, 10_000, 50_000] {
        let graph = build_graph(entity_count);
        let contracts = dependency_contracts();
        measure_case(
            entity_count,
            "localized-entity",
            &graph,
            &contracts,
            &localized_change_set(entity_count),
        );
        measure_case(
            entity_count,
            "global-transform",
            &graph,
            &contracts,
            &global_change_set(),
        );
    }
}
