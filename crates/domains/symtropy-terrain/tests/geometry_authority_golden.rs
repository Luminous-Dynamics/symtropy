// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! External golden vectors for the dependency-free exact geometry kernel.
//!
//! The filename is retained for provenance from the earlier live-authority
//! experiment, but this test deliberately does not resurrect the superseded
//! caller-supplied `capture(coord, &EarthChunk)` surface. Live ECS authority is
//! qualified separately. These vectors bind only the frozen pure-kernel byte
//! grammar and material-code vocabulary.

#[path = "../src/geometry_kernel.rs"]
mod geometry_kernel;

use geometry_kernel::{
    EarthChunkLatticeCoord, TerrainGeometrySnapshot, TerrainMaterialCode, TerrainVoxelBox,
    TerrainVoxelIndex, TERRAIN_GEOMETRY_CHUNK_SIZE, TERRAIN_GEOMETRY_VOXEL_COUNT,
};

fn default_materials() -> Box<[TerrainMaterialCode; TERRAIN_GEOMETRY_VOXEL_COUNT]> {
    Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT])
}

fn index(x: usize, y: usize, z: usize) -> usize {
    (x * TERRAIN_GEOMETRY_CHUNK_SIZE * TERRAIN_GEOMETRY_CHUNK_SIZE)
        + (y * TERRAIN_GEOMETRY_CHUNK_SIZE)
        + z
}

/// External golden derived independently from the frozen byte grammar:
/// domain || schema_le || coord_i32_le || chunk_size_le || 4096 material tags.
#[test]
fn external_default_chunk_snapshot_vector() {
    let snapshot = TerrainGeometrySnapshot::from_material_codes(
        EarthChunkLatticeCoord::new(4, -2, 9),
        default_materials(),
    );
    assert_eq!(
        snapshot.digest().to_hex(),
        "50d1ec5cdf6d8f2da4cc0ce801cef6f5b62728ca7b8ffcc02fba48bda23cffef"
    );
}

/// Local identity is deliberately independent of the enclosing chunk digest.
#[test]
fn external_air_voxel_vector() {
    let mut materials = default_materials();
    materials[index(2, 3, 4)] = TerrainMaterialCode::AIR;
    let snapshot = TerrainGeometrySnapshot::from_material_codes(
        EarthChunkLatticeCoord::new(1, 2, 3),
        materials,
    );
    let observation = snapshot
        .observe(TerrainVoxelIndex::new(2, 3, 4).expect("valid voxel"))
        .expect("observation");
    assert_eq!(
        observation.digest().to_hex(),
        "fb4cb313a0697b5cdba06f4102ed6dd6f124559c08e35a066834561ee6650841"
    );
}

/// This vector independently binds snapshot -> one-voxel complete query -> receipt.
#[test]
fn external_single_voxel_query_vector() {
    let snapshot = TerrainGeometrySnapshot::from_material_codes(
        EarthChunkLatticeCoord::new(1, 2, 3),
        default_materials(),
    );
    assert_eq!(
        snapshot.digest().to_hex(),
        "f7961398ef1ddf5e450aa191bc64901df06e33609b251da3f60f3948fd72dbf1"
    );

    let bounds = TerrainVoxelBox::new(
        TerrainVoxelIndex::new(2, 3, 4).expect("valid voxel"),
        [3, 4, 5],
    )
    .expect("valid one-voxel query");
    let receipt = snapshot.query(bounds).expect("complete query");
    assert_eq!(receipt.observations().len(), 1);
    assert_eq!(
        receipt.observations()[0].digest().to_hex(),
        "d6c2b78646786404b73ef6aabc4cac6545ba4341f14e509f9745dd187f2eda14"
    );
    assert_eq!(
        receipt.digest().to_hex(),
        "7a505c56667d34eaafa1da76980e066e02e0f4187409f30eedd54f4c5168a117"
    );
    receipt.validate().expect("valid receipt");
}
