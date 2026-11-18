use std::collections::BTreeMap;
use std::hint::black_box;
use std::time::{Duration, Instant};

use construction_motion_oracle::{
    ArtifactPersistence, ConstructionArtifact, ConstructionArtifactGraph, ConstructionArtifactKind,
    ConstructionChangeSet, DependencyContractSet, DependencyDomain, DependencyKey,
    DependencyTransition, VersionedDependency, compile_change_impact_with_contract,
    minimum_construction_dependency_contracts,
};

const SAMPLES: usize = 25;

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn key(domain: DependencyDomain, scope_id: String) -> DependencyKey {
    DependencyKey { domain, scope_id }
}

fn build_graph(
    entity_count: usize,
    contracts: &[construction_motion_oracle::ArtifactDependencyContract],
) -> ConstructionArtifactGraph {
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
            dependencies: dependencies_for_kind(
                contracts,
                ConstructionArtifactKind::MotionEnvelope,
                index,
                &entity_id,
            ),
            depends_on_artifact_ids: Vec::new(),
        });
        artifacts.push(ConstructionArtifact {
            artifact_id: occupancy_id.clone(),
            artifact_sha256: digest('b'),
            kind: ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
            persistence: ArtifactPersistence::Recomputable,
            dependencies: dependencies_for_kind(
                contracts,
                ConstructionArtifactKind::SpaceTimeOccupancyAssessment,
                index,
                &entity_id,
            ),
            depends_on_artifact_ids: vec![motion_id],
        });
        artifacts.push(ConstructionArtifact {
            artifact_id: readiness_id,
            artifact_sha256: digest('c'),
            kind: ConstructionArtifactKind::DispatchReadiness,
            persistence: ArtifactPersistence::Recomputable,
            dependencies: dependencies_for_kind(
                contracts,
                ConstructionArtifactKind::DispatchReadiness,
                index,
                &entity_id,
            ),
            depends_on_artifact_ids: vec![occupancy_id],
        });
    }

    let mut current_by_key = BTreeMap::<DependencyKey, String>::new();
    for artifact in &artifacts {
        for dependency in &artifact.dependencies {
            current_by_key
                .entry(dependency.key.clone())
                .or_insert_with(|| dependency.observed_sha256.clone());
        }
    }
    ConstructionArtifactGraph {
        graph_snapshot_sha256: digest('f'),
        current_dependencies: current_by_key
            .into_iter()
            .map(|(key, observed_sha256)| VersionedDependency {
                key,
                observed_sha256,
            })
            .collect(),
        artifacts,
    }
}

fn domain_version(domain: DependencyDomain) -> char {
    match domain {
        DependencyDomain::EntityGeometry => '1',
        DependencyDomain::CoordinateTransform => '2',
        DependencyDomain::CoordinateFrame => '3',
        DependencyDomain::DesignRevision => '4',
        DependencyDomain::RobotConfiguration => '5',
        DependencyDomain::ToolConfiguration => '6',
        DependencyDomain::PayloadConfiguration => '7',
        DependencyDomain::TrajectoryConfiguration => '8',
        DependencyDomain::SiteClockMap => '9',
        DependencyDomain::ObstacleMap => 'a',
        DependencyDomain::DynamicActorManifest => 'b',
        DependencyDomain::OccupancyEngineBuild => 'c',
        DependencyDomain::ClearancePolicy => 'd',
        DependencyDomain::SiteLedgerState => 'e',
        DependencyDomain::WorkPackageSchedule => 'f',
        DependencyDomain::AuthorityPolicy => '0',
        _ => '1',
    }
}

fn domain_scope(domain: DependencyDomain, index: usize, entity_id: &str) -> String {
    match domain {
        DependencyDomain::EntityGeometry => entity_id.to_owned(),
        DependencyDomain::CoordinateTransform => "floor:02".into(),
        DependencyDomain::CoordinateFrame => "site-frame:02".into(),
        DependencyDomain::DesignRevision => "floor:02".into(),
        DependencyDomain::RobotConfiguration => "robot:unit-01".into(),
        DependencyDomain::ToolConfiguration => "robot:unit-01:tool".into(),
        DependencyDomain::PayloadConfiguration => "robot:unit-01:payload".into(),
        DependencyDomain::TrajectoryConfiguration => "robot:unit-01:trajectory".into(),
        DependencyDomain::SiteClockMap => "site-clock".into(),
        DependencyDomain::ObstacleMap => format!("region:{index:08}"),
        DependencyDomain::DynamicActorManifest => format!("region:{index:08}"),
        DependencyDomain::OccupancyEngineBuild => "occupancy-engine".into(),
        DependencyDomain::ClearancePolicy => "site".into(),
        DependencyDomain::SiteLedgerState => "site-ledger".into(),
        DependencyDomain::WorkPackageSchedule => "work-package".into(),
        DependencyDomain::AuthorityPolicy => "site-authority".into(),
        _ => format!("{}:{index:08}", format!("{domain:?}").to_lowercase()),
    }
}

fn dependencies_for_kind(
    contracts: &[construction_motion_oracle::ArtifactDependencyContract],
    kind: ConstructionArtifactKind,
    index: usize,
    entity_id: &str,
) -> Vec<VersionedDependency> {
    let contract = contracts
        .iter()
        .find(|contract| contract.artifact_kind == kind)
        .expect("every benchmark artifact kind has a minimum dependency contract");
    contract
        .required_domains
        .iter()
        .map(|domain| VersionedDependency {
            key: key(*domain, domain_scope(*domain, index, entity_id)),
            observed_sha256: digest(domain_version(*domain)),
        })
        .collect()
}

fn dependency_contracts() -> DependencyContractSet {
    DependencyContractSet {
        contract_sha256: digest('c'),
        contracts: minimum_construction_dependency_contracts(),
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
            key: key(DependencyDomain::CoordinateTransform, "floor:02".into()),
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
        entity_count * 3
    };
    let expected_transitive = if case == "localized-entity" { 2 } else { 0 };

    let check = compile_change_impact_with_contract(
        black_box(graph),
        black_box(contracts),
        black_box(change_set),
    )
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
    assert!(
        invalidated_counts
            .iter()
            .all(|count| { *count == expected_direct + expected_transitive })
    );

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
        let contracts = dependency_contracts();
        let graph = build_graph(entity_count, &contracts.contracts);
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
