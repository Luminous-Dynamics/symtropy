// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{
    AnalysisDomain, CanonicalPlane, CellCoord, CellPartitionObservation, DecompositionProfile,
    GeometricDecompositionSnapshot, GeometricInterfaceKind, LocalPartitionCensus,
    LocalPartitionCut, Point3i, RealizedGeometrySnapshotRef, WorkBudget,
};
use symtropy_spatial_topology::ExactSourceRef;

fn sid(value: &str) -> StableId {
    StableId::parse(value).expect("test stable id")
}

fn exact(authority: &str, subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
    ExactSourceRef::new(sid(authority), sid(subject), revision, digest).expect("test exact ref")
}

fn frame(subject: &str) -> ExactSourceRef {
    exact(
        "symtropy.test.coordinate-frame",
        subject,
        1,
        &format!("{subject}-v1"),
    )
}

fn geometry(revision: u64, digest: &str) -> RealizedGeometrySnapshotRef {
    RealizedGeometrySnapshotRef::new(exact(
        "symtropy.test.geometry",
        "world.snapshot",
        revision,
        digest,
    ))
    .expect("test geometry ref")
}

fn profile(revision: u64) -> DecompositionProfile {
    DecompositionProfile::reference_v1(sid("pb04b.locality.profile"), revision, 10)
        .expect("test profile")
}

fn profile_with(
    id: &str,
    revision: u64,
    backend: &str,
    quantum_um: i64,
    budget: WorkBudget,
) -> DecompositionProfile {
    DecompositionProfile::new(sid(id), revision, sid(backend), quantum_um, budget)
        .expect("test profile")
}

fn domain() -> AnalysisDomain {
    AnalysisDomain::new_in_frame(
        sid("pb04b.locality.domain"),
        1,
        frame("pb04b.locality.frame"),
        Point3i::new(0, 0, 0),
        [1, 1, 1],
    )
    .expect("test domain")
}

fn domain_in_frame(
    id: &str,
    revision: u64,
    coordinate_frame: ExactSourceRef,
    origin: Point3i,
    dimensions: [u32; 3],
) -> AnalysisDomain {
    AnalysisDomain::new_in_frame(sid(id), revision, coordinate_frame, origin, dimensions)
        .expect("test domain")
}

fn separator() -> ExactSourceRef {
    exact(
        "symtropy.test.partition",
        "doorway.separator",
        1,
        "separator-v1",
    )
}

fn barrier(revision: u64, digest: &str) -> ExactSourceRef {
    exact("symtropy.test.partition", "door.leaf", revision, digest)
}

fn doorway_cut_at(cell: CellCoord, barriers: Vec<ExactSourceRef>) -> LocalPartitionCut {
    LocalPartitionCut::new(
        cell,
        CanonicalPlane::new(1, 0, 0, -15).expect("test plane"),
        barriers,
        vec![separator()],
    )
    .expect("test cut")
}

fn doorway_cut(barriers: Vec<ExactSourceRef>) -> LocalPartitionCut {
    LocalPartitionCut::new(
        CellCoord::new(0, 0, 0),
        CanonicalPlane::new(1, 0, 0, -5).expect("test plane"),
        barriers,
        vec![separator()],
    )
    .expect("test cut")
}

fn coverage(cell: CellCoord) -> ExactSourceRef {
    exact(
        "symtropy.test.coverage",
        &format!("cell-{}-{}-{}", cell.x, cell.y, cell.z),
        1,
        &format!("coverage-{}-{}-{}-v1", cell.x, cell.y, cell.z),
    )
}

fn derive_in_domain(
    geometry: RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    cut: LocalPartitionCut,
) -> GeometricDecompositionSnapshot {
    let dimensions = domain.dimensions();
    let mut observations = Vec::new();
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let cell = CellCoord::new(x, y, z);
                let cuts = if cell == cut.cell {
                    vec![cut.clone()]
                } else {
                    Vec::new()
                };
                observations.push(
                    CellPartitionObservation::new(cell, coverage(cell), cuts)
                        .expect("coverage observation"),
                );
            }
        }
    }
    let census = LocalPartitionCensus::new(geometry, domain, observations).expect("census");
    GeometricDecompositionSnapshot::derive_from_census(profile, domain, census)
        .expect("test decomposition")
}

fn derive(
    geometry: RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    cut: LocalPartitionCut,
) -> GeometricDecompositionSnapshot {
    derive_in_domain(geometry, profile, &domain(), cut)
}

fn fragment_ids(snapshot: &GeometricDecompositionSnapshot) -> Vec<String> {
    snapshot
        .fragments()
        .iter()
        .map(|fragment| fragment.id().0.as_str().to_owned())
        .collect()
}

fn fragment_ids_for_cell(
    snapshot: &GeometricDecompositionSnapshot,
    cell: CellCoord,
) -> Vec<String> {
    let mut ids = snapshot
        .fragments()
        .iter()
        .filter(|fragment| fragment.cell() == cell)
        .map(|fragment| fragment.id().0.as_str().to_owned())
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn interface_ids(snapshot: &GeometricDecompositionSnapshot) -> Vec<String> {
    snapshot
        .interfaces()
        .iter()
        .map(|interface| interface.id().0.as_str().to_owned())
        .collect()
}

fn local_partition_id(snapshot: &GeometricDecompositionSnapshot) -> String {
    snapshot
        .interfaces()
        .iter()
        .find(|interface| {
            matches!(
                interface.kind(),
                GeometricInterfaceKind::LocalPartition { .. }
            )
        })
        .expect("local partition")
        .id()
        .0
        .as_str()
        .to_owned()
}

#[test]
fn successor_uses_schema_v3() {
    let snapshot = derive(
        geometry(1, "schema3-geometry"),
        &profile(1),
        doorway_cut(vec![barrier(1, "schema3-door")]),
    );
    assert_eq!(snapshot.schema_version(), 3);
}

#[test]
fn remote_geometry_revision_changes_snapshot_but_not_local_locus_ids() {
    let profile = profile(1);
    let before = derive(
        geometry(10, "geometry-before"),
        &profile,
        doorway_cut(vec![barrier(4, "closed-door")]),
    );
    let after = derive(
        geometry(11, "geometry-after-unrelated-remote-edit"),
        &profile,
        doorway_cut(vec![barrier(4, "closed-door")]),
    );

    assert_ne!(before.content_digest(), after.content_digest());
    assert_eq!(fragment_ids(&before), fragment_ids(&after));
    assert_eq!(interface_ids(&before), interface_ids(&after));
}

#[test]
fn persistent_separator_keeps_doorway_locus_ids_across_closed_to_open_evidence_change() {
    let profile = profile(1);
    let closed = derive(
        geometry(20, "same-geometry"),
        &profile,
        doorway_cut(vec![barrier(7, "door-closed")]),
    );
    let open = derive(
        geometry(20, "same-geometry"),
        &profile,
        doorway_cut(Vec::new()),
    );

    assert_ne!(closed.content_digest(), open.content_digest());
    assert_eq!(fragment_ids(&closed), fragment_ids(&open));
    assert_eq!(interface_ids(&closed), interface_ids(&open));

    let closed_partition = closed
        .interfaces()
        .iter()
        .find(|interface| {
            matches!(
                interface.kind(),
                GeometricInterfaceKind::LocalPartition { .. }
            )
        })
        .expect("closed doorway partition");
    let open_partition = open
        .interfaces()
        .iter()
        .find(|interface| {
            matches!(
                interface.kind(),
                GeometricInterfaceKind::LocalPartition { .. }
            )
        })
        .expect("open doorway partition");

    assert!(closed_partition.has_material_barrier());
    assert!(closed_partition.has_portal_separator());
    assert!(!open_partition.has_material_barrier());
    assert!(open_partition.has_portal_separator());
    assert_eq!(closed_partition.id(), open_partition.id());
}

#[test]
fn execution_profile_metadata_and_budget_do_not_rename_local_loci() {
    let roomy = profile_with(
        "pb04b.execution.profile.roomy",
        1,
        "pb04b.fixed-cut-cell.v1",
        10,
        WorkBudget::reference_v1(),
    );
    let tight = profile_with(
        "pb04b.execution.profile.tight",
        99,
        "pb04b.fixed-cut-cell.v1",
        10,
        WorkBudget {
            max_cells: 16,
            max_partition_facts: 16,
            max_interfaces: 16,
        },
    );
    let first = derive(
        geometry(30, "same-geometry"),
        &roomy,
        doorway_cut(vec![barrier(8, "same-door")]),
    );
    let second = derive(
        geometry(30, "same-geometry"),
        &tight,
        doorway_cut(vec![barrier(8, "same-door")]),
    );

    assert_ne!(first.content_digest(), second.content_digest());
    assert_eq!(fragment_ids(&first), fragment_ids(&second));
    assert_eq!(interface_ids(&first), interface_ids(&second));
}

#[test]
fn topology_backend_semantics_still_rename_local_loci() {
    let first_profile = profile_with(
        "pb04b.semantic.profile.a",
        1,
        "pb04b.fixed-cut-cell.v1",
        10,
        WorkBudget::reference_v1(),
    );
    let second_profile = profile_with(
        "pb04b.semantic.profile.b",
        1,
        "pb04b.fixed-cut-cell.v2-test",
        10,
        WorkBudget::reference_v1(),
    );
    let first = derive(
        geometry(31, "same-geometry"),
        &first_profile,
        doorway_cut(vec![barrier(8, "same-door")]),
    );
    let second = derive(
        geometry(31, "same-geometry"),
        &second_profile,
        doorway_cut(vec![barrier(8, "same-door")]),
    );

    assert_ne!(fragment_ids(&first), fragment_ids(&second));
    assert_ne!(interface_ids(&first), interface_ids(&second));
}

#[test]
fn shifted_analysis_windows_preserve_the_same_physical_locus_ids() {
    let profile = profile(1);
    let shared_frame = frame("pb04b.shared.frame");
    let wide = domain_in_frame(
        "pb04b.window.wide",
        1,
        shared_frame.clone(),
        Point3i::new(0, 0, 0),
        [2, 1, 1],
    );
    let shifted = domain_in_frame(
        "pb04b.window.shifted",
        7,
        shared_frame,
        Point3i::new(10, 0, 0),
        [1, 1, 1],
    );
    let wide_snapshot = derive_in_domain(
        geometry(40, "same-geometry"),
        &profile,
        &wide,
        doorway_cut_at(CellCoord::new(1, 0, 0), vec![barrier(9, "same-door")]),
    );
    let shifted_snapshot = derive_in_domain(
        geometry(40, "same-geometry"),
        &profile,
        &shifted,
        doorway_cut_at(CellCoord::new(0, 0, 0), vec![barrier(9, "same-door")]),
    );

    assert_ne!(
        wide_snapshot.content_digest(),
        shifted_snapshot.content_digest()
    );
    assert_eq!(
        fragment_ids_for_cell(&wide_snapshot, CellCoord::new(1, 0, 0)),
        fragment_ids_for_cell(&shifted_snapshot, CellCoord::new(0, 0, 0))
    );
    assert_eq!(
        local_partition_id(&wide_snapshot),
        local_partition_id(&shifted_snapshot)
    );
}

#[test]
fn exact_coordinate_frame_change_renames_local_loci() {
    let profile = profile(1);
    let first_domain = domain_in_frame(
        "pb04b.frame.domain",
        1,
        frame("pb04b.frame.alpha"),
        Point3i::new(0, 0, 0),
        [1, 1, 1],
    );
    let second_domain = domain_in_frame(
        "pb04b.frame.domain",
        1,
        frame("pb04b.frame.beta"),
        Point3i::new(0, 0, 0),
        [1, 1, 1],
    );
    let first = derive_in_domain(
        geometry(41, "same-geometry"),
        &profile,
        &first_domain,
        doorway_cut(vec![barrier(10, "same-door")]),
    );
    let second = derive_in_domain(
        geometry(41, "same-geometry"),
        &profile,
        &second_domain,
        doorway_cut(vec![barrier(10, "same-door")]),
    );

    assert_ne!(fragment_ids(&first), fragment_ids(&second));
    assert_ne!(interface_ids(&first), interface_ids(&second));
}

#[test]
fn pb04a_adapter_context_changes_boundary_subject_without_renaming_local_loci() {
    let profile = profile(1);
    let snapshot = derive(
        geometry(50, "same-geometry"),
        &profile,
        doorway_cut(vec![barrier(11, "closed-door")]),
    );
    let frame_a = exact("symtropy.test.frame", "frame-a", 1, "frame-a-v1");
    let frame_b = exact("symtropy.test.frame", "frame-b", 1, "frame-b-v1");
    let environment_a = exact(
        "symtropy.test.environment",
        "environment-a",
        1,
        "environment-a-v1",
    );
    let environment_b = exact(
        "symtropy.test.environment",
        "environment-b",
        1,
        "environment-b-v1",
    );

    let first = snapshot
        .to_pb04a_boundary(frame_a.clone(), environment_a.clone())
        .expect("first boundary");
    let repeated = snapshot
        .to_pb04a_boundary(frame_a, environment_a)
        .expect("repeated boundary");
    let other_frame = snapshot
        .to_pb04a_boundary(frame_b, environment_b.clone())
        .expect("other frame boundary");
    let other_environment = snapshot
        .to_pb04a_boundary(
            exact("symtropy.test.frame", "frame-a", 1, "frame-a-v1"),
            environment_b,
        )
        .expect("other environment boundary");

    assert_eq!(first.exact_ref(), repeated.exact_ref());
    assert_ne!(first.snapshot_id(), other_frame.snapshot_id());
    assert_ne!(first.snapshot_id(), other_environment.snapshot_id());
    assert_ne!(first.content_digest(), other_frame.content_digest());
    assert_ne!(first.content_digest(), other_environment.content_digest());
    assert_eq!(snapshot.fragments().len(), first.regions().len());
    assert_eq!(snapshot.interfaces().len(), first.interfaces().len());
}
