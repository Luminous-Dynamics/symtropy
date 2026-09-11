// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{
    AnalysisDomain, CanonicalPlane, CellCoord, CellPartitionObservation, DecompositionError,
    DecompositionProfile, FragmentSide, GeometricDecompositionSnapshot, GeometricInterfaceKind,
    LocalPartitionCensus, LocalPartitionCut, Point3i, RealizedGeometrySnapshotRef,
};
use symtropy_spatial_topology::ExactSourceRef;

fn sid(value: &str) -> StableId {
    StableId::parse(value).expect("stable id")
}

fn exact(authority: &str, subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
    ExactSourceRef::new(sid(authority), sid(subject), revision, digest).expect("exact ref")
}

fn geometry(revision: u64, digest: &str) -> RealizedGeometrySnapshotRef {
    RealizedGeometrySnapshotRef::new(exact(
        "symtropy.test.geometry",
        "coverage-world",
        revision,
        digest,
    ))
    .expect("geometry")
}

fn profile() -> DecompositionProfile {
    DecompositionProfile::reference_v1(sid("pb04b1.coverage.profile"), 1, 10).expect("profile")
}

fn domain(dimensions: [u32; 3]) -> AnalysisDomain {
    AnalysisDomain::new_in_frame(
        sid("pb04b1.coverage.domain"),
        1,
        exact(
            "symtropy.test.frame",
            "coverage-frame",
            1,
            "coverage-frame-v1",
        ),
        Point3i::new(0, 0, 0),
        dimensions,
    )
    .expect("domain")
}

fn coverage(cell: CellCoord, revision: u64, digest: &str) -> ExactSourceRef {
    exact(
        "symtropy.test.coverage",
        &format!("cell-{}-{}-{}", cell.x, cell.y, cell.z),
        revision,
        digest,
    )
}

fn cut(cell: CellCoord, barriers: Vec<ExactSourceRef>) -> LocalPartitionCut {
    LocalPartitionCut::new(
        cell,
        CanonicalPlane::new(1, 0, 0, -5).expect("plane"),
        barriers,
        vec![exact("symtropy.test.separator", "doorway", 1, "doorway-v1")],
    )
    .expect("cut")
}

fn clear(cell: CellCoord, digest: &str) -> CellPartitionObservation {
    CellPartitionObservation::new(cell, coverage(cell, 1, digest), Vec::new())
        .expect("clear observation")
}

fn observed_cut(cell: CellCoord, coverage_digest: &str) -> CellPartitionObservation {
    CellPartitionObservation::new(
        cell,
        coverage(cell, 1, coverage_digest),
        vec![cut(
            cell,
            vec![exact("symtropy.test.barrier", "wall", 1, "wall-v1")],
        )],
    )
    .expect("cut observation")
}

fn census(
    geometry: RealizedGeometrySnapshotRef,
    domain: &AnalysisDomain,
    observations: Vec<CellPartitionObservation>,
) -> LocalPartitionCensus {
    LocalPartitionCensus::new(geometry, domain, observations).expect("complete census")
}

fn derive(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    census: LocalPartitionCensus,
) -> GeometricDecompositionSnapshot {
    GeometricDecompositionSnapshot::derive_from_census(profile, domain, census)
        .expect("decomposition")
}

fn fragment_ids(snapshot: &GeometricDecompositionSnapshot) -> Vec<String> {
    snapshot
        .fragments()
        .iter()
        .map(|fragment| fragment.id().0.as_str().to_owned())
        .collect()
}

fn local_sources(snapshot: &GeometricDecompositionSnapshot) -> Vec<Vec<ExactSourceRef>> {
    snapshot
        .fragments()
        .iter()
        .map(|fragment| fragment.source_refs().to_vec())
        .collect()
}

#[test]
fn schema3_sparse_derive_is_fail_closed() {
    let domain = domain([1, 1, 1]);
    let result = GeometricDecompositionSnapshot::derive(
        geometry(1, "geometry-v1"),
        &profile(),
        &domain,
        Vec::new(),
    );
    assert!(matches!(
        result,
        Err(DecompositionError::CompleteCensusRequired)
    ));
}

#[test]
fn missing_cell_observation_fails_closed() {
    let domain = domain([2, 1, 1]);
    let result = LocalPartitionCensus::new(
        geometry(1, "geometry-v1"),
        &domain,
        vec![clear(CellCoord::new(0, 0, 0), "coverage-a")],
    );
    assert!(matches!(
        result,
        Err(DecompositionError::MissingCellObservation(CellCoord {
            x: 1,
            y: 0,
            z: 0
        }))
    ));
}

#[test]
fn duplicate_cell_observation_fails_closed() {
    let domain = domain([1, 1, 1]);
    let cell = CellCoord::new(0, 0, 0);
    let result = LocalPartitionCensus::new(
        geometry(1, "geometry-v1"),
        &domain,
        vec![clear(cell, "coverage-a"), clear(cell, "coverage-a")],
    );
    assert!(matches!(
        result,
        Err(DecompositionError::DuplicateCellObservation(value)) if value == cell
    ));
}

#[test]
fn observation_outside_domain_fails_closed() {
    let domain = domain([1, 1, 1]);
    let result = LocalPartitionCensus::new(
        geometry(1, "geometry-v1"),
        &domain,
        vec![clear(CellCoord::new(1, 0, 0), "coverage-outside")],
    );
    assert!(matches!(
        result,
        Err(DecompositionError::CellOutsideDomain(CellCoord {
            x: 1,
            y: 0,
            z: 0
        }))
    ));
}

#[test]
fn explicit_zero_cut_coverage_admits_whole_free_fragment() {
    let domain = domain([1, 1, 1]);
    let snapshot = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-v1"),
            &domain,
            vec![clear(CellCoord::new(0, 0, 0), "coverage-clear")],
        ),
    );
    assert_eq!(snapshot.schema_version(), 3);
    assert_eq!(snapshot.fragments().len(), 1);
    assert_eq!(snapshot.fragments()[0].side(), FragmentSide::Whole);
    assert!(snapshot.interfaces().is_empty());
}

#[test]
fn complete_diagonal_cut_keeps_two_fragments_and_partition() {
    let domain = domain([1, 1, 1]);
    let cell = CellCoord::new(0, 0, 0);
    let observation = CellPartitionObservation::new(
        cell,
        coverage(cell, 1, "coverage-cut"),
        vec![
            LocalPartitionCut::new(
                cell,
                CanonicalPlane::new(1, -1, 0, 0).expect("diagonal"),
                vec![exact(
                    "symtropy.test.barrier",
                    "diagonal-wall",
                    1,
                    "diag-v1",
                )],
                Vec::new(),
            )
            .expect("diagonal cut"),
        ],
    )
    .expect("observation");
    let snapshot = derive(
        &profile(),
        &domain,
        census(geometry(1, "geometry-v1"), &domain, vec![observation]),
    );
    assert_eq!(snapshot.fragments().len(), 2);
    assert_eq!(snapshot.interfaces().len(), 1);
    assert!(matches!(
        snapshot.interfaces()[0].kind(),
        GeometricInterfaceKind::LocalPartition { .. }
    ));
}

#[test]
fn shuffled_census_order_is_exactly_equivalent() {
    let domain = domain([2, 1, 1]);
    let left = observed_cut(CellCoord::new(0, 0, 0), "coverage-left");
    let right = clear(CellCoord::new(1, 0, 0), "coverage-right");
    let first = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-v1"),
            &domain,
            vec![left.clone(), right.clone()],
        ),
    );
    let second = derive(
        &profile(),
        &domain,
        census(geometry(1, "geometry-v1"), &domain, vec![right, left]),
    );
    assert_eq!(first, second);
    assert_eq!(first.content_digest(), second.content_digest());
}

#[test]
fn conflicting_coverage_identity_digest_fails_closed() {
    let domain = domain([2, 1, 1]);
    let shared_a = exact("symtropy.test.coverage", "shared", 1, "coverage-a");
    let shared_b = exact("symtropy.test.coverage", "shared", 1, "coverage-b");
    let result = LocalPartitionCensus::new(
        geometry(1, "geometry-v1"),
        &domain,
        vec![
            CellPartitionObservation::new(CellCoord::new(0, 0, 0), shared_a, Vec::new())
                .expect("left"),
            CellPartitionObservation::new(CellCoord::new(1, 0, 0), shared_b, Vec::new())
                .expect("right"),
        ],
    );
    assert!(matches!(
        result,
        Err(DecompositionError::ConflictingExactRef { .. })
    ));
}

#[test]
fn remote_geometry_change_preserves_local_ids_and_local_provenance() {
    let domain = domain([1, 1, 1]);
    let cell = CellCoord::new(0, 0, 0);
    let observation = observed_cut(cell, "coverage-stable");
    let first = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-before"),
            &domain,
            vec![observation.clone()],
        ),
    );
    let second = derive(
        &profile(),
        &domain,
        census(
            geometry(2, "geometry-after-remote-change"),
            &domain,
            vec![observation],
        ),
    );
    assert_ne!(first.content_digest(), second.content_digest());
    assert_eq!(fragment_ids(&first), fragment_ids(&second));
    assert_eq!(local_sources(&first), local_sources(&second));
}

#[test]
fn changed_coverage_changes_provenance_and_snapshot_not_local_ids() {
    let domain = domain([1, 1, 1]);
    let cell = CellCoord::new(0, 0, 0);
    let first = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-v1"),
            &domain,
            vec![observed_cut(cell, "coverage-before")],
        ),
    );
    let second = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-v1"),
            &domain,
            vec![
                CellPartitionObservation::new(
                    cell,
                    coverage(cell, 2, "coverage-after"),
                    vec![cut(
                        cell,
                        vec![exact("symtropy.test.barrier", "wall", 1, "wall-v1")],
                    )],
                )
                .expect("changed coverage"),
            ],
        ),
    );
    assert_eq!(fragment_ids(&first), fragment_ids(&second));
    assert_ne!(local_sources(&first), local_sources(&second));
    assert_ne!(first.content_digest(), second.content_digest());
}

#[test]
fn open_cross_face_cites_both_cell_coverage_observations() {
    let domain = domain([2, 1, 1]);
    let left = CellCoord::new(0, 0, 0);
    let right = CellCoord::new(1, 0, 0);
    let left_coverage = coverage(left, 1, "coverage-left");
    let right_coverage = coverage(right, 1, "coverage-right");
    let snapshot = derive(
        &profile(),
        &domain,
        census(
            geometry(1, "geometry-v1"),
            &domain,
            vec![
                CellPartitionObservation::new(left, left_coverage.clone(), Vec::new())
                    .expect("left"),
                CellPartitionObservation::new(right, right_coverage.clone(), Vec::new())
                    .expect("right"),
            ],
        ),
    );
    let open = snapshot
        .interfaces()
        .iter()
        .find(|value| matches!(value.kind(), GeometricInterfaceKind::OpenCrossFace { .. }))
        .expect("open cross-face");
    assert!(open.source_refs().contains(&left_coverage));
    assert!(open.source_refs().contains(&right_coverage));
}
