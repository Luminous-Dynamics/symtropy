// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{
    AnalysisDomain, CanonicalPlane, CellCoord, CellPartitionObservation, DecompositionError,
    DecompositionProfile, GeometricDecompositionSnapshot, GeometricInterfaceKind,
    IncrementalDecompositionState, IncrementalFullRebuildReason, IncrementalRecomputeOutcome,
    LocalPartitionCensus, LocalPartitionCut, Point3i, RealizedGeometrySnapshotRef,
};
use symtropy_spatial_topology::ExactSourceRef;

fn sid(value: &str) -> StableId {
    StableId::parse(value).expect("stable id")
}

fn exact(authority: &str, subject: &str, revision: u64, digest: &str) -> ExactSourceRef {
    ExactSourceRef::new(sid(authority), sid(subject), revision, digest).expect("exact ref")
}

fn frame() -> ExactSourceRef {
    exact("symtropy.test.frame", "pb04c-frame", 1, "frame-v1")
}

fn geometry(revision: u64, digest: &str) -> RealizedGeometrySnapshotRef {
    RealizedGeometrySnapshotRef::new(exact(
        "symtropy.test.geometry",
        "pb04c-world",
        revision,
        digest,
    ))
    .expect("geometry")
}

fn profile() -> DecompositionProfile {
    DecompositionProfile::reference_v1(sid("pb04c-profile"), 1, 10).expect("profile")
}

fn other_profile() -> DecompositionProfile {
    DecompositionProfile::reference_v1(sid("pb04c-profile-other"), 2, 10).expect("profile")
}

fn domain(width: u32) -> AnalysisDomain {
    AnalysisDomain::new_in_frame(
        sid("pb04c-domain"),
        1,
        frame(),
        Point3i::new(0, 0, 0),
        [width, 1, 1],
    )
    .expect("domain")
}

fn shifted_domain(width: u32) -> AnalysisDomain {
    AnalysisDomain::new_in_frame(
        sid("pb04c-domain"),
        2,
        frame(),
        Point3i::new(10, 0, 0),
        [width, 1, 1],
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

fn separator(cell: CellCoord) -> ExactSourceRef {
    exact(
        "symtropy.test.separator",
        &format!("cell-{}-{}-{}", cell.x, cell.y, cell.z),
        1,
        "separator-v1",
    )
}

fn barrier(cell: CellCoord, revision: u64, digest: &str) -> ExactSourceRef {
    exact(
        "symtropy.test.barrier",
        &format!("cell-{}-{}-{}", cell.x, cell.y, cell.z),
        revision,
        digest,
    )
}

fn plane_x(cell: CellCoord, local_x: i64) -> CanonicalPlane {
    let world_x = i64::from(cell.x) * 10 + local_x;
    CanonicalPlane::new(1, 0, 0, -world_x).expect("plane")
}

fn clear(cell: CellCoord, revision: u64, digest: &str) -> CellPartitionObservation {
    CellPartitionObservation::new(cell, coverage(cell, revision, digest), Vec::new())
        .expect("clear observation")
}

fn cut_observation(
    cell: CellCoord,
    coverage_revision: u64,
    coverage_digest: &str,
    local_x: i64,
    barriers: Vec<ExactSourceRef>,
    separators: Vec<ExactSourceRef>,
) -> CellPartitionObservation {
    let cut = LocalPartitionCut::new(cell, plane_x(cell, local_x), barriers, separators)
        .expect("cut");
    CellPartitionObservation::new(
        cell,
        coverage(cell, coverage_revision, coverage_digest),
        vec![cut],
    )
    .expect("cut observation")
}

fn clear_observations(width: u32, revision: u64, prefix: &str) -> Vec<CellPartitionObservation> {
    (0..width)
        .map(|x| {
            let cell = CellCoord::new(x, 0, 0);
            clear(cell, revision, &format!("{prefix}-{x}"))
        })
        .collect()
}

fn census(
    geometry: RealizedGeometrySnapshotRef,
    domain: &AnalysisDomain,
    observations: Vec<CellPartitionObservation>,
) -> LocalPartitionCensus {
    LocalPartitionCensus::new(geometry, domain, observations).expect("census")
}

fn clean(
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    census: &LocalPartitionCensus,
) -> GeometricDecompositionSnapshot {
    GeometricDecompositionSnapshot::derive_from_census(profile, domain, census.clone())
        .expect("clean rebuild")
}

fn updated(
    state: &IncrementalDecompositionState,
    profile: &DecompositionProfile,
    domain: &AnalysisDomain,
    successor: LocalPartitionCensus,
) -> symtropy_spatial_decomposition::IncrementalDecompositionUpdate {
    match state
        .advance(profile, domain, successor)
        .expect("incremental advance")
    {
        IncrementalRecomputeOutcome::Updated(update) => update,
        IncrementalRecomputeOutcome::FullRebuildRequired(reason) => {
            panic!("unexpected full rebuild requirement: {reason:?}")
        }
    }
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

fn local_partition_id(
    snapshot: &GeometricDecompositionSnapshot,
    cell: CellCoord,
) -> Option<String> {
    snapshot.interfaces().iter().find_map(|interface| match interface.kind() {
        GeometricInterfaceKind::LocalPartition { cell: candidate, .. } if *candidate == cell => {
            Some(interface.id().0.as_str().to_owned())
        }
        _ => None,
    })
}

#[test]
fn no_local_change_reuses_every_atom_and_matches_clean_rebuild() {
    let profile = profile();
    let domain = domain(3);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(3, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor.clone())
        .expect("state");
    let successor = predecessor.clone();
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert!(update.changed_cells().is_empty());
    assert!(update.recompute_cells().is_empty());
    assert_eq!(update.snapshot(), &expected);
    assert_eq!(update.snapshot().fragments(), state.snapshot().fragments());
    assert_eq!(update.snapshot().interfaces(), state.snapshot().interfaces());
}

#[test]
fn coverage_only_change_preserves_locus_ids_and_matches_clean_rebuild() {
    let profile = profile();
    let domain = domain(3);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(3, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let cell = CellCoord::new(1, 0, 0);
    let mut observations = clear_observations(3, 1, "coverage-v1");
    observations[1] = clear(cell, 2, "coverage-v2-1");
    let successor = census(geometry(1, "geometry-v1"), &domain, observations);
    let expected = clean(&profile, &domain, &successor);
    let before_ids = fragment_ids_for_cell(state.snapshot(), cell);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.changed_cells(), &[cell]);
    assert_eq!(
        update.recompute_cells(),
        &[
            CellCoord::new(0, 0, 0),
            CellCoord::new(1, 0, 0),
            CellCoord::new(2, 0, 0),
        ]
    );
    assert_eq!(fragment_ids_for_cell(update.snapshot(), cell), before_ids);
    assert_eq!(update.snapshot(), &expected);
    assert_ne!(update.snapshot().content_digest(), state.snapshot().content_digest());
}

#[test]
fn clear_to_cut_transition_matches_clean_rebuild() {
    let profile = profile();
    let domain = domain(3);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(3, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let cell = CellCoord::new(1, 0, 0);
    let mut observations = clear_observations(3, 1, "coverage-v1");
    observations[1] = cut_observation(
        cell,
        2,
        "coverage-cut",
        5,
        vec![barrier(cell, 1, "barrier-v1")],
        Vec::new(),
    );
    let successor = census(geometry(2, "geometry-v2"), &domain, observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.snapshot(), &expected);
    assert_eq!(fragment_ids_for_cell(update.snapshot(), cell).len(), 2);
    assert!(local_partition_id(update.snapshot(), cell).is_some());
}

#[test]
fn cut_to_clear_transition_matches_clean_rebuild() {
    let profile = profile();
    let domain = domain(3);
    let cell = CellCoord::new(1, 0, 0);
    let mut predecessor_observations = clear_observations(3, 1, "coverage-v1");
    predecessor_observations[1] = cut_observation(
        cell,
        1,
        "coverage-cut-v1",
        5,
        vec![barrier(cell, 1, "barrier-v1")],
        Vec::new(),
    );
    let predecessor = census(geometry(1, "geometry-v1"), &domain, predecessor_observations);
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let successor = census(
        geometry(2, "geometry-v2"),
        &domain,
        clear_observations(3, 2, "coverage-clear-v2"),
    );
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.snapshot(), &expected);
    assert_eq!(fragment_ids_for_cell(update.snapshot(), cell).len(), 1);
    assert!(local_partition_id(update.snapshot(), cell).is_none());
}

#[test]
fn closed_to_open_barrier_change_preserves_partition_locus_and_matches_clean_rebuild() {
    let profile = profile();
    let domain = domain(3);
    let cell = CellCoord::new(1, 0, 0);
    let mut predecessor_observations = clear_observations(3, 1, "coverage-v1");
    predecessor_observations[1] = cut_observation(
        cell,
        1,
        "coverage-door-v1",
        5,
        vec![barrier(cell, 1, "door-closed")],
        vec![separator(cell)],
    );
    let predecessor = census(geometry(1, "geometry-v1"), &domain, predecessor_observations);
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let before_id = local_partition_id(state.snapshot(), cell).expect("partition");

    let mut successor_observations = clear_observations(3, 1, "coverage-v1");
    successor_observations[1] = cut_observation(
        cell,
        2,
        "coverage-door-v2",
        5,
        Vec::new(),
        vec![separator(cell)],
    );
    let successor = census(geometry(2, "geometry-v2"), &domain, successor_observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.snapshot(), &expected);
    assert_eq!(local_partition_id(update.snapshot(), cell).as_deref(), Some(before_id.as_str()));
}

#[test]
fn local_plane_change_matches_clean_rebuild_and_renames_local_locus() {
    let profile = profile();
    let domain = domain(3);
    let cell = CellCoord::new(1, 0, 0);
    let mut predecessor_observations = clear_observations(3, 1, "coverage-v1");
    predecessor_observations[1] = cut_observation(
        cell,
        1,
        "coverage-cut-v1",
        3,
        vec![barrier(cell, 1, "wall-v1")],
        Vec::new(),
    );
    let predecessor = census(geometry(1, "geometry-v1"), &domain, predecessor_observations);
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let before = local_partition_id(state.snapshot(), cell).expect("partition");

    let mut successor_observations = clear_observations(3, 1, "coverage-v1");
    successor_observations[1] = cut_observation(
        cell,
        2,
        "coverage-cut-v2",
        7,
        vec![barrier(cell, 2, "wall-v2")],
        Vec::new(),
    );
    let successor = census(geometry(2, "geometry-v2"), &domain, successor_observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);
    let after = local_partition_id(update.snapshot(), cell).expect("partition");

    assert_eq!(update.snapshot(), &expected);
    assert_ne!(before, after);
}

#[test]
fn adjacent_edit_recomputes_shared_face_exactly() {
    let profile = profile();
    let domain = domain(3);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(3, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let cell = CellCoord::new(0, 0, 0);
    let mut observations = clear_observations(3, 1, "coverage-v1");
    observations[0] = cut_observation(
        cell,
        2,
        "coverage-cut",
        5,
        vec![barrier(cell, 1, "wall-v1")],
        Vec::new(),
    );
    let successor = census(geometry(2, "geometry-v2"), &domain, observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.snapshot(), &expected);
    assert_eq!(update.changed_cells(), &[cell]);
    assert_eq!(
        update.recompute_cells(),
        &[CellCoord::new(0, 0, 0), CellCoord::new(1, 0, 0)]
    );
}

#[test]
fn separated_dirty_cells_have_canonical_halo_and_exact_output() {
    let profile = profile();
    let domain = domain(5);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(5, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let mut observations = clear_observations(5, 1, "coverage-v1");
    observations[0] = clear(CellCoord::new(0, 0, 0), 2, "left-changed");
    observations[4] = clear(CellCoord::new(4, 0, 0), 2, "right-changed");
    observations.reverse();
    let successor = census(geometry(2, "geometry-v2"), &domain, observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert_eq!(update.snapshot(), &expected);
    assert_eq!(
        update.changed_cells(),
        &[CellCoord::new(0, 0, 0), CellCoord::new(4, 0, 0)]
    );
    assert_eq!(
        update.recompute_cells(),
        &[
            CellCoord::new(0, 0, 0),
            CellCoord::new(1, 0, 0),
            CellCoord::new(3, 0, 0),
            CellCoord::new(4, 0, 0),
        ]
    );
}

#[test]
fn successor_observation_order_does_not_change_incremental_result() {
    let profile = profile();
    let domain = domain(3);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(3, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let cell = CellCoord::new(1, 0, 0);
    let mut forward = clear_observations(3, 1, "coverage-v1");
    forward[1] = clear(cell, 2, "coverage-v2");
    let mut reverse = forward.clone();
    reverse.reverse();

    let first = updated(
        &state,
        &profile,
        &domain,
        census(geometry(2, "geometry-v2"), &domain, forward),
    );
    let second = updated(
        &state,
        &profile,
        &domain,
        census(geometry(2, "geometry-v2"), &domain, reverse),
    );
    assert_eq!(first, second);
}

#[test]
fn global_geometry_revision_alone_reuses_local_atoms_but_updates_exact_snapshot() {
    let profile = profile();
    let domain = domain(3);
    let observations = clear_observations(3, 1, "coverage-v1");
    let predecessor = census(geometry(1, "geometry-v1"), &domain, observations.clone());
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let successor = census(geometry(2, "geometry-v2"), &domain, observations);
    let expected = clean(&profile, &domain, &successor);
    let update = updated(&state, &profile, &domain, successor);

    assert!(update.changed_cells().is_empty());
    assert!(update.recompute_cells().is_empty());
    assert_eq!(update.snapshot(), &expected);
    assert_eq!(update.snapshot().fragments(), state.snapshot().fragments());
    assert_eq!(update.snapshot().interfaces(), state.snapshot().interfaces());
    assert_ne!(update.snapshot().content_digest(), state.snapshot().content_digest());
}

#[test]
fn mismatched_predecessor_snapshot_is_rejected() {
    let profile = profile();
    let domain = domain(2);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(2, 1, "coverage-v1"),
    );
    let different = census(
        geometry(2, "geometry-v2"),
        &domain,
        clear_observations(2, 2, "coverage-v2"),
    );
    let wrong_snapshot = clean(&profile, &domain, &different);
    let result = IncrementalDecompositionState::from_parts(
        &profile,
        &domain,
        predecessor,
        wrong_snapshot,
    );
    assert!(matches!(
        result,
        Err(DecompositionError::PredecessorSnapshotMismatch)
    ));
}

#[test]
fn incompatible_profile_or_domain_requires_full_rebuild() {
    let profile = profile();
    let domain = domain(2);
    let predecessor = census(
        geometry(1, "geometry-v1"),
        &domain,
        clear_observations(2, 1, "coverage-v1"),
    );
    let state = IncrementalDecompositionState::new(&profile, &domain, predecessor).expect("state");
    let successor = census(
        geometry(2, "geometry-v2"),
        &domain,
        clear_observations(2, 2, "coverage-v2"),
    );

    assert!(matches!(
        state.advance(&other_profile(), &domain, successor.clone()).unwrap(),
        IncrementalRecomputeOutcome::FullRebuildRequired(
            IncrementalFullRebuildReason::ProfileChanged
        )
    ));

    let shifted = shifted_domain(2);
    let shifted_successor = census(
        geometry(2, "geometry-v2"),
        &shifted,
        clear_observations(2, 2, "coverage-v2"),
    );
    assert!(matches!(
        state.advance(&profile, &shifted, shifted_successor).unwrap(),
        IncrementalRecomputeOutcome::FullRebuildRequired(
            IncrementalFullRebuildReason::DomainChanged
        )
    ));
}
