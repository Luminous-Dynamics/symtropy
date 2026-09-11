// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{
    AnalysisDomain, CanonicalPlane, CellCoord, DecompositionProfile,
    GeometricDecompositionSnapshot, GeometricInterfaceKind, LocalPartitionCut, Point3i,
    RealizedGeometrySnapshotRef,
};
use symtropy_spatial_topology::ExactSourceRef;

fn sid(value: &str) -> StableId {
    StableId::parse(value).expect("test stable id")
}

fn exact(authority: &str, subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
    ExactSourceRef::new(sid(authority), sid(subject), revision, digest).expect("test exact ref")
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

fn domain() -> AnalysisDomain {
    AnalysisDomain::new(
        sid("pb04b.locality.domain"),
        1,
        Point3i::new(0, 0, 0),
        [1, 1, 1],
    )
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

fn doorway_cut(barriers: Vec<ExactSourceRef>) -> LocalPartitionCut {
    LocalPartitionCut::new(
        CellCoord::new(0, 0, 0),
        CanonicalPlane::new(1, 0, 0, -5).expect("test plane"),
        barriers,
        vec![separator()],
    )
    .expect("test cut")
}

fn derive(
    geometry: RealizedGeometrySnapshotRef,
    profile: &DecompositionProfile,
    cut: LocalPartitionCut,
) -> GeometricDecompositionSnapshot {
    GeometricDecompositionSnapshot::derive(geometry, profile, &domain(), vec![cut])
        .expect("test decomposition")
}

fn fragment_ids(snapshot: &GeometricDecompositionSnapshot) -> Vec<String> {
    snapshot
        .fragments()
        .iter()
        .map(|fragment| fragment.id().0.as_str().to_owned())
        .collect()
}

fn interface_ids(snapshot: &GeometricDecompositionSnapshot) -> Vec<String> {
    snapshot
        .interfaces()
        .iter()
        .map(|interface| interface.id().0.as_str().to_owned())
        .collect()
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
fn topology_affecting_profile_identity_still_renames_local_loci() {
    let geometry = geometry(30, "same-geometry");
    let profile_v1 = profile(1);
    let profile_v2 = profile(2);
    let first = derive(
        geometry.clone(),
        &profile_v1,
        doorway_cut(vec![barrier(8, "same-door")]),
    );
    let second = derive(
        geometry,
        &profile_v2,
        doorway_cut(vec![barrier(8, "same-door")]),
    );

    assert_ne!(first.content_digest(), second.content_digest());
    assert_ne!(fragment_ids(&first), fragment_ids(&second));
    assert_ne!(interface_ids(&first), interface_ids(&second));
}
