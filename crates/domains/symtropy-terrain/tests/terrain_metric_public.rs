// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_terrain::terrain_metric::{
    MAX_TERRAIN_VOXEL_EDGE_UM, TERRAIN_METRIC_CANONICAL_BYTES_LEN, TERRAIN_METRIC_SCHEMA_VERSION,
    TerrainMetricError, TerrainMetricProfile,
};

#[test]
fn downstream_public_path_preserves_exact_metric_semantics() {
    let profile = TerrainMetricProfile::new(1_000_000).expect("qualified metric value");
    assert_eq!(profile.schema_version(), TERRAIN_METRIC_SCHEMA_VERSION);
    assert_eq!(profile.voxel_edge_um(), 1_000_000);
    assert_eq!(
        profile.canonical_bytes().len(),
        TERRAIN_METRIC_CANONICAL_BYTES_LEN
    );
    assert_eq!(
        profile.canonical_bytes(),
        [1, 0, 0, 0, 0x40, 0x42, 0x0f, 0, 0, 0, 0, 0]
    );
}

#[test]
fn downstream_public_path_preserves_fail_closed_bounds() {
    assert_eq!(
        TerrainMetricProfile::new(0),
        Err(TerrainMetricError::NonPositiveVoxelEdge(0))
    );
    assert_eq!(
        TerrainMetricProfile::new(MAX_TERRAIN_VOXEL_EDGE_UM + 1),
        Err(TerrainMetricError::VoxelEdgeTooLarge {
            actual_um: MAX_TERRAIN_VOXEL_EDGE_UM + 1,
            maximum_um: MAX_TERRAIN_VOXEL_EDGE_UM,
        })
    );
}

#[test]
fn public_export_does_not_supply_a_default_metric_value() {
    let one = TerrainMetricProfile::new(1).unwrap();
    let two = TerrainMetricProfile::new(2).unwrap();
    assert_ne!(one, two);
    assert_ne!(one.canonical_bytes(), two.canonical_bytes());
}
