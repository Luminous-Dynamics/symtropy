// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{
    AnalysisDomain, CellCoord, DecompositionError, DecompositionProfile, MAX_ABS_COORD_UM, Point3i,
};

fn id(value: &str) -> StableId {
    StableId::parse(value).unwrap()
}

fn profile(quantum_um: i64) -> DecompositionProfile {
    DecompositionProfile::reference_v1(id("pb04c1-public-profile"), 1, quantum_um).unwrap()
}

#[test]
fn public_query_returns_exact_origin_and_last_cell_bounds() {
    let profile = profile(10);
    let domain = AnalysisDomain::new(
        id("pb04c1-public-domain"),
        4,
        Point3i::new(-20, 30, -40),
        [2, 3, 4],
    )
    .unwrap();
    assert_eq!(
        domain.exact_cell_bounds(&profile, CellCoord::new(0, 0, 0)),
        Ok((Point3i::new(-20, 30, -40), Point3i::new(-10, 40, -30)))
    );
    assert_eq!(
        domain.exact_cell_bounds(&profile, CellCoord::new(1, 2, 3)),
        Ok((Point3i::new(-10, 50, -10), Point3i::new(0, 60, 0)))
    );
}

#[test]
fn public_query_fails_closed_outside_domain() {
    let profile = profile(10);
    let domain = AnalysisDomain::new(
        id("pb04c1-outside-domain"),
        1,
        Point3i::new(0, 0, 0),
        [2, 2, 2],
    )
    .unwrap();
    let cell = CellCoord::new(2, 0, 0);
    assert_eq!(
        domain.exact_cell_bounds(&profile, cell),
        Err(DecompositionError::CellOutsideDomain(cell))
    );
}

#[test]
fn public_query_preserves_nonzero_negative_origin_exactly() {
    let profile = profile(25);
    let domain = AnalysisDomain::new(
        id("pb04c1-negative-origin"),
        2,
        Point3i::new(-1_000, 500, -250),
        [4, 4, 4],
    )
    .unwrap();
    assert_eq!(
        domain.exact_cell_bounds(&profile, CellCoord::new(3, 2, 1)),
        Ok((Point3i::new(-925, 550, -225), Point3i::new(-900, 575, -200)))
    );
}

#[test]
fn public_query_uses_the_exact_selected_profile_quantum() {
    let domain = AnalysisDomain::new(
        id("pb04c1-quantum-domain"),
        1,
        Point3i::new(100, 200, 300),
        [2, 1, 1],
    )
    .unwrap();
    let cell = CellCoord::new(1, 0, 0);
    assert_eq!(
        domain.exact_cell_bounds(&profile(10), cell),
        Ok((Point3i::new(110, 200, 300), Point3i::new(120, 210, 310)))
    );
    assert_eq!(
        domain.exact_cell_bounds(&profile(20), cell),
        Ok((Point3i::new(120, 200, 300), Point3i::new(140, 220, 320)))
    );
}

#[test]
fn public_query_preserves_existing_coordinate_bound_failure() {
    let profile = profile(1);
    let domain = AnalysisDomain::new(
        id("pb04c1-bound-domain"),
        1,
        Point3i::new(MAX_ABS_COORD_UM, 0, 0),
        [1, 1, 1],
    )
    .unwrap();
    assert_eq!(
        domain.exact_cell_bounds(&profile, CellCoord::new(0, 0, 0)),
        Err(DecompositionError::CoordinateOutOfRange(
            MAX_ABS_COORD_UM + 1
        ))
    );
}

#[test]
fn public_query_is_repeatable_and_side_effect_free() {
    let profile = profile(100);
    let domain = AnalysisDomain::new(
        id("pb04c1-repeat-domain"),
        9,
        Point3i::new(-500, -500, -500),
        [2, 2, 2],
    )
    .unwrap();
    let before = domain.exact_ref();
    let first = domain
        .exact_cell_bounds(&profile, CellCoord::new(1, 1, 1))
        .unwrap();
    for _ in 0..32 {
        assert_eq!(
            domain
                .exact_cell_bounds(&profile, CellCoord::new(1, 1, 1))
                .unwrap(),
            first
        );
    }
    assert_eq!(domain.exact_ref(), before);
}
