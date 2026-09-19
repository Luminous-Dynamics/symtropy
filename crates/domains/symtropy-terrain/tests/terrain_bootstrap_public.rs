// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_terrain::{
    geometry_kernel::EarthChunkLatticeCoord,
    terrain_bootstrap::{
        TerrainBootstrapCandidateV1, TerrainBootstrapError, TerrainChunkBootstrapEntryV1,
        TerrainChunkRefV1, TerrainLineageProfileV1, TerrainLineageRefV1,
    },
};

fn id32(first: u8) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    bytes[0] = first;
    bytes
}

#[test]
fn downstream_public_path_is_representation_only_and_order_canonical() {
    let lineage =
        TerrainLineageRefV1::new(TerrainLineageProfileV1::LocalExplicitV1, id32(1)).unwrap();
    let first = TerrainChunkBootstrapEntryV1::new(
        TerrainChunkRefV1::new(id32(1)).unwrap(),
        EarthChunkLatticeCoord::new(4, 5, 6),
    );
    let second = TerrainChunkBootstrapEntryV1::new(
        TerrainChunkRefV1::new(id32(2)).unwrap(),
        EarthChunkLatticeCoord::new(-4, -5, -6),
    );

    let a = TerrainBootstrapCandidateV1::new(lineage, vec![first, second]).unwrap();
    let b = TerrainBootstrapCandidateV1::new(lineage, vec![second, first]).unwrap();

    assert_eq!(a, b);
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
}

#[test]
fn downstream_public_path_rejects_duplicate_chunk_and_locus() {
    let lineage =
        TerrainLineageRefV1::new(TerrainLineageProfileV1::LocalExplicitV1, id32(9)).unwrap();
    let chunk_a = TerrainChunkRefV1::new(id32(3)).unwrap();
    let chunk_b = TerrainChunkRefV1::new(id32(4)).unwrap();
    let locus = EarthChunkLatticeCoord::new(1, 2, 3);

    assert_eq!(
        TerrainBootstrapCandidateV1::new(
            lineage,
            vec![
                TerrainChunkBootstrapEntryV1::new(chunk_a, locus),
                TerrainChunkBootstrapEntryV1::new(
                    chunk_a,
                    EarthChunkLatticeCoord::new(9, 9, 9),
                ),
            ],
        ),
        Err(TerrainBootstrapError::DuplicateChunkRef(chunk_a))
    );

    assert_eq!(
        TerrainBootstrapCandidateV1::new(
            lineage,
            vec![
                TerrainChunkBootstrapEntryV1::new(chunk_a, locus),
                TerrainChunkBootstrapEntryV1::new(chunk_b, locus),
            ],
        ),
        Err(TerrainBootstrapError::DuplicateLatticeLocus(locus))
    );
}
