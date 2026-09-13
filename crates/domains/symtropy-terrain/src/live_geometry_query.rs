// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Point-of-invocation bounded queries over qualified live Terrain geometry.
//!
//! This adapter deliberately contributes no new geometry identity or receipt
//! grammar. It composes the qualified live ECS capture with the dependency-free
//! bounded query kernel. The returned receipt therefore binds the exact snapshot
//! observed during this invocation, but it makes no claim that the mutable world
//! remains unchanged after return.

use bevy::{ecs::world::World, prelude::Entity};
use std::{error::Error, fmt};

use crate::geometry_kernel::{TerrainGeometryError, TerrainGeometryQueryReceipt, TerrainVoxelBox};
use crate::live_geometry_provider::{TerrainLiveGeometryError, capture_live_terrain_geometry};

/// Fail-closed composition errors for a point-of-invocation live query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainLiveQueryError {
    /// The qualified live ECS provider could not produce a complete snapshot.
    Capture(TerrainLiveGeometryError),
    /// The pure geometry kernel rejected or could not complete the bounded query.
    Query(TerrainGeometryError),
}

impl fmt::Display for TerrainLiveQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "live Terrain geometry capture failed: {error}"),
            Self::Query(error) => write!(f, "live Terrain geometry query failed: {error}"),
        }
    }
}

impl Error for TerrainLiveQueryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Capture(error) => Some(error),
            Self::Query(error) => Some(error),
        }
    }
}

impl From<TerrainLiveGeometryError> for TerrainLiveQueryError {
    fn from(error: TerrainLiveGeometryError) -> Self {
        Self::Capture(error)
    }
}

impl From<TerrainGeometryError> for TerrainLiveQueryError {
    fn from(error: TerrainGeometryError) -> Self {
        Self::Query(error)
    }
}

/// Query one exact bounded region from current ECS-owned Terrain state.
///
/// The caller supplies only the world, entity, and already-exact bounded voxel
/// box. Exact chunk locus and material state come from the qualified live capture
/// path. No transform, density, renderer/physics handle, caller-supplied snapshot,
/// revision, timestamp, or PB/world-frame representation participates.
pub fn query_live_terrain_geometry(
    world: &World,
    entity: Entity,
    bounds: TerrainVoxelBox,
) -> Result<TerrainGeometryQueryReceipt, TerrainLiveQueryError> {
    let snapshot = capture_live_terrain_geometry(world, entity)?;
    Ok(snapshot.query(bounds)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EarthChunk, SubstrateMaterial,
        geometry_kernel::{
            EarthChunkLatticeCoord, TERRAIN_GEOMETRY_VOXEL_COUNT, TerrainMaterialCode,
            TerrainVoxelIndex,
        },
        live_geometry_provider::EarthChunkLatticeLocus,
    };
    use bevy::prelude::GlobalTransform;

    fn one_voxel_box(x: u8, y: u8, z: u8) -> TerrainVoxelBox {
        TerrainVoxelBox::new(
            TerrainVoxelIndex::new(x, y, z).expect("test voxel must be bounded"),
            [x + 1, y + 1, z + 1],
        )
        .expect("single-voxel test bounds must be valid")
    }

    #[test]
    fn full_chunk_live_query_returns_complete_valid_receipt() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(4, -2, 9),
            ))
            .id();

        let receipt = query_live_terrain_geometry(&world, entity, TerrainVoxelBox::full_chunk())
            .expect("full live query must succeed");

        assert_eq!(receipt.chunk(), EarthChunkLatticeCoord::new(4, -2, 9));
        assert_eq!(receipt.observations().len(), TERRAIN_GEOMETRY_VOXEL_COUNT);
        receipt
            .validate()
            .expect("live query receipt must validate");
    }

    #[test]
    fn bounded_query_preserves_kernel_canonical_order() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(0, 0, 0),
            ))
            .id();
        let bounds =
            TerrainVoxelBox::new(TerrainVoxelIndex::new(1, 2, 3).unwrap(), [3, 4, 5]).unwrap();

        let receipt = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        let actual: Vec<_> = receipt
            .observations()
            .iter()
            .map(|observation| observation.voxel())
            .collect();
        let expected = vec![
            TerrainVoxelIndex::new(1, 2, 3).unwrap(),
            TerrainVoxelIndex::new(1, 2, 4).unwrap(),
            TerrainVoxelIndex::new(1, 3, 3).unwrap(),
            TerrainVoxelIndex::new(1, 3, 4).unwrap(),
            TerrainVoxelIndex::new(2, 2, 3).unwrap(),
            TerrainVoxelIndex::new(2, 2, 4).unwrap(),
            TerrainVoxelIndex::new(2, 3, 3).unwrap(),
            TerrainVoxelIndex::new(2, 3, 4).unwrap(),
        ];
        assert_eq!(actual, expected);
        receipt.validate().unwrap();
    }

    #[test]
    fn repeated_unchanged_live_query_has_identical_receipt_identity() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(1, 1, 1),
            ))
            .id();
        let bounds = one_voxel_box(2, 3, 4);

        let first = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        let second = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn queried_material_mutation_changes_local_observation_and_receipt() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(2, 3, 4),
            ))
            .id();
        let bounds = one_voxel_box(5, 6, 7);

        let before = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        world.get_mut::<EarthChunk>(entity).unwrap().voxels[5][6][7] = SubstrateMaterial::Air;
        let after = query_live_terrain_geometry(&world, entity, bounds).unwrap();

        assert_ne!(
            before.observations()[0].digest(),
            after.observations()[0].digest()
        );
        assert_ne!(before.snapshot_digest(), after.snapshot_digest());
        assert_ne!(before.digest(), after.digest());
        assert_eq!(after.observations()[0].material(), TerrainMaterialCode::AIR);
    }

    #[test]
    fn remote_mutation_changes_enclosing_provenance_not_local_observation() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(-1, 8, 13),
            ))
            .id();
        let bounds = one_voxel_box(0, 0, 0);

        let before = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        world.get_mut::<EarthChunk>(entity).unwrap().voxels[15][15][15] = SubstrateMaterial::Air;
        let after = query_live_terrain_geometry(&world, entity, bounds).unwrap();

        assert_eq!(before.observations(), after.observations());
        assert_ne!(before.snapshot_digest(), after.snapshot_digest());
        assert_ne!(before.digest(), after.digest());
    }

    #[test]
    fn density_rebuild_flags_and_global_transform_are_not_query_authority() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(7, 8, 9),
                GlobalTransform::default(),
            ))
            .id();
        let bounds = one_voxel_box(1, 1, 1);
        let before = query_live_terrain_geometry(&world, entity, bounds).unwrap();

        {
            let mut chunk = world.get_mut::<EarthChunk>(entity).unwrap();
            chunk.densities[1][1][1] = 0.03125;
            chunk.is_dirty = !chunk.is_dirty;
            chunk.is_rebuilding = !chunk.is_rebuilding;
        }
        world
            .entity_mut(entity)
            .insert(GlobalTransform::from_xyz(900.0, -400.0, 12.5));

        let after = query_live_terrain_geometry(&world, entity, bounds).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn explicit_air_is_returned_as_observation_evidence() {
        let mut world = World::new();
        let mut chunk = EarthChunk::default();
        chunk.voxels[3][4][5] = SubstrateMaterial::Air;
        let entity = world
            .spawn((chunk, EarthChunkLatticeLocus::new_for_test(3, 2, 1)))
            .id();

        let receipt = query_live_terrain_geometry(&world, entity, one_voxel_box(3, 4, 5)).unwrap();
        assert_eq!(receipt.observations().len(), 1);
        assert_eq!(
            receipt.observations()[0].material(),
            TerrainMaterialCode::AIR
        );
    }

    #[test]
    fn missing_chunk_is_a_capture_error() {
        let mut world = World::new();
        let entity = world
            .spawn(EarthChunkLatticeLocus::new_for_test(1, 2, 3))
            .id();

        assert_eq!(
            query_live_terrain_geometry(&world, entity, TerrainVoxelBox::full_chunk()),
            Err(TerrainLiveQueryError::Capture(
                TerrainLiveGeometryError::MissingEarthChunk(entity)
            ))
        );
    }

    #[test]
    fn missing_locus_is_a_capture_error() {
        let mut world = World::new();
        let entity = world.spawn(EarthChunk::default()).id();

        assert_eq!(
            query_live_terrain_geometry(&world, entity, TerrainVoxelBox::full_chunk()),
            Err(TerrainLiveQueryError::Capture(
                TerrainLiveGeometryError::MissingLatticeLocus(entity)
            ))
        );
    }

    #[test]
    fn duplicate_locus_is_a_capture_error() {
        let mut world = World::new();
        let requested = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(-3, 7, 11),
            ))
            .id();
        let conflicting = world
            .spawn(EarthChunkLatticeLocus::new_for_test(-3, 7, 11))
            .id();

        assert_eq!(
            query_live_terrain_geometry(&world, requested, TerrainVoxelBox::full_chunk()),
            Err(TerrainLiveQueryError::Capture(
                TerrainLiveGeometryError::DuplicateLatticeLocus {
                    coord: EarthChunkLatticeCoord::new(-3, 7, 11),
                    requested,
                    conflicting,
                }
            ))
        );
    }

    #[test]
    fn invalid_bounds_are_a_query_error_after_successful_capture() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(0, 0, 0),
            ))
            .id();
        let min = TerrainVoxelIndex::new(4, 4, 4).unwrap();
        let invalid = TerrainVoxelBox {
            min,
            max_exclusive: [4, 5, 5],
        };

        assert_eq!(
            query_live_terrain_geometry(&world, entity, invalid),
            Err(TerrainLiveQueryError::Query(
                TerrainGeometryError::InvalidQueryBounds {
                    min,
                    max_exclusive: [4, 5, 5],
                }
            ))
        );
    }
}
