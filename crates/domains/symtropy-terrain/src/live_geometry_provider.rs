// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Live ECS adapter for the qualified dependency-free Terrain geometry kernel.
//!
//! The caller supplies only an ECS entity. Exact lattice identity is resolved
//! from the owning world together with the live `EarthChunk`; callers cannot
//! inject a coordinate at capture time. `GlobalTransform`, densities, render
//! and Rapier handles, dirty/rebuild flags, and asynchronous rebuild cadence are
//! deliberately outside the exact geometry identity.

use bevy::{
    ecs::world::World,
    prelude::{Component, Entity},
};
use std::{error::Error, fmt};

use super::{CHUNK_SIZE, EarthChunk, SubstrateMaterial};
use crate::geometry_kernel::{
    EarthChunkLatticeCoord, TERRAIN_GEOMETRY_CHUNK_SIZE, TERRAIN_GEOMETRY_VOXEL_COUNT,
    TerrainGeometrySnapshot, TerrainMaterialCode,
};

/// Exact integer Terrain-lattice locus owned by an ECS entity.
///
/// This component is intentionally distinct from `GlobalTransform`. Moving,
/// rendering, rebuilding, or re-colliding a chunk cannot silently rewrite its
/// exact geometry identity. World construction may assign or replace this
/// component explicitly; read-side capture never accepts a caller coordinate.
///
/// It intentionally does not implement Bevy `Reflect` and is not registered as
/// a reflected component. Generic scene/inspector mutation must not silently
/// become a spatial-authority write path. A future persistence/import boundary
/// must assign loci explicitly and qualify that mapping separately.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EarthChunkLatticeLocus {
    x: i32,
    y: i32,
    z: i32,
}

impl EarthChunkLatticeLocus {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn x(self) -> i32 {
        self.x
    }

    pub const fn y(self) -> i32 {
        self.y
    }

    pub const fn z(self) -> i32 {
        self.z
    }

    pub const fn coord(self) -> EarthChunkLatticeCoord {
        EarthChunkLatticeCoord::new(self.x, self.y, self.z)
    }
}

/// Fail-closed errors at the live ECS geometry-authority boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainLiveGeometryError {
    MissingEarthChunk(Entity),
    MissingLatticeLocus(Entity),
    DuplicateLatticeLocus {
        coord: EarthChunkLatticeCoord,
        requested: Entity,
        conflicting: Entity,
    },
    ChunkDimensionMismatch {
        live: usize,
        kernel: usize,
    },
    IncompleteMaterialProjection {
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for TerrainLiveGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingEarthChunk(entity) => {
                write!(f, "entity {entity:?} has no live EarthChunk")
            }
            Self::MissingLatticeLocus(entity) => {
                write!(f, "entity {entity:?} has no exact Terrain lattice locus")
            }
            Self::DuplicateLatticeLocus {
                coord,
                requested,
                conflicting,
            } => write!(
                f,
                "Terrain lattice locus {coord:?} is claimed by requested entity {requested:?} and conflicting entity {conflicting:?}"
            ),
            Self::ChunkDimensionMismatch { live, kernel } => write!(
                f,
                "live Terrain chunk size {live} does not match qualified kernel size {kernel}"
            ),
            Self::IncompleteMaterialProjection { expected, actual } => write!(
                f,
                "incomplete Terrain material projection: expected {expected} cells, got {actual}"
            ),
        }
    }
}

impl Error for TerrainLiveGeometryError {}

/// Capture the current exact discrete geometry of one ECS-owned Terrain chunk.
///
/// Authority comes from the world at invocation: both `EarthChunk` and
/// `EarthChunkLatticeLocus` are resolved from `entity`, and the locus must be
/// unique among all locus-bearing entities. No transform or caller-provided
/// coordinate participates in the snapshot identity.
pub fn capture_live_terrain_geometry(
    world: &World,
    entity: Entity,
) -> Result<TerrainGeometrySnapshot, TerrainLiveGeometryError> {
    if CHUNK_SIZE != TERRAIN_GEOMETRY_CHUNK_SIZE {
        return Err(TerrainLiveGeometryError::ChunkDimensionMismatch {
            live: CHUNK_SIZE,
            kernel: TERRAIN_GEOMETRY_CHUNK_SIZE,
        });
    }

    let chunk = world
        .get::<EarthChunk>(entity)
        .ok_or(TerrainLiveGeometryError::MissingEarthChunk(entity))?;
    let locus = world
        .get::<EarthChunkLatticeLocus>(entity)
        .copied()
        .ok_or(TerrainLiveGeometryError::MissingLatticeLocus(entity))?;
    let coord = locus.coord();

    for entity_ref in world.iter_entities() {
        let conflicting = entity_ref.id();
        if conflicting == entity {
            continue;
        }
        if entity_ref
            .get::<EarthChunkLatticeLocus>()
            .is_some_and(|other| other.coord() == coord)
        {
            return Err(TerrainLiveGeometryError::DuplicateLatticeLocus {
                coord,
                requested: entity,
                conflicting,
            });
        }
    }

    let mut materials = Box::new([TerrainMaterialCode::AIR; TERRAIN_GEOMETRY_VOXEL_COUNT]);
    let mut cursor = 0_usize;
    for x in 0..CHUNK_SIZE {
        for y in 0..CHUNK_SIZE {
            for z in 0..CHUNK_SIZE {
                let slot = materials.get_mut(cursor).ok_or(
                    TerrainLiveGeometryError::IncompleteMaterialProjection {
                        expected: TERRAIN_GEOMETRY_VOXEL_COUNT,
                        actual: cursor,
                    },
                )?;
                *slot = stable_material_code(chunk.voxels[x][y][z]);
                cursor += 1;
            }
        }
    }

    if cursor != TERRAIN_GEOMETRY_VOXEL_COUNT {
        return Err(TerrainLiveGeometryError::IncompleteMaterialProjection {
            expected: TERRAIN_GEOMETRY_VOXEL_COUNT,
            actual: cursor,
        });
    }

    Ok(TerrainGeometrySnapshot::from_material_codes(
        coord, materials,
    ))
}

fn stable_material_code(material: SubstrateMaterial) -> TerrainMaterialCode {
    match material {
        SubstrateMaterial::Air => TerrainMaterialCode::AIR,
        SubstrateMaterial::Bedrock => TerrainMaterialCode::BEDROCK,
        SubstrateMaterial::Dolomite => TerrainMaterialCode::DOLOMITE,
        SubstrateMaterial::PyriteTailing => TerrainMaterialCode::PYRITE_TAILING,
        SubstrateMaterial::Quartzite => TerrainMaterialCode::QUARTZITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_kernel::{TerrainVoxelBox, TerrainVoxelIndex};
    use bevy::prelude::GlobalTransform;

    #[test]
    fn captures_complete_current_material_projection_from_ecs_locus() {
        let mut world = World::new();
        let mut chunk = EarthChunk::default();
        chunk.voxels[2][3][4] = SubstrateMaterial::Air;
        let entity = world
            .spawn((chunk, EarthChunkLatticeLocus::new(4, -2, 9)))
            .id();

        let snapshot = capture_live_terrain_geometry(&world, entity).expect("capture must succeed");
        assert_eq!(snapshot.chunk(), EarthChunkLatticeCoord::new(4, -2, 9));
        let air = snapshot
            .observe(TerrainVoxelIndex::new(2, 3, 4).expect("bounded index"))
            .expect("observation must succeed");
        assert_eq!(air.material(), TerrainMaterialCode::AIR);

        let receipt = snapshot
            .query(TerrainVoxelBox::full_chunk())
            .expect("full chunk query must succeed");
        assert_eq!(
            receipt.observations().len(),
            TERRAIN_GEOMETRY_VOXEL_COUNT
        );
        receipt.validate().expect("complete receipt must validate");
    }

    #[test]
    fn recapture_tracks_voxel_change_but_ignores_runtime_state() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new(1, 2, 3),
                GlobalTransform::default(),
            ))
            .id();

        let baseline = capture_live_terrain_geometry(&world, entity)
            .expect("baseline capture must succeed")
            .digest();

        {
            let mut chunk = world
                .get_mut::<EarthChunk>(entity)
                .expect("chunk must remain present");
            chunk.densities[0][0][0] = 0.125;
            chunk.is_dirty = !chunk.is_dirty;
            chunk.is_rebuilding = !chunk.is_rebuilding;
        }
        world
            .entity_mut(entity)
            .insert(GlobalTransform::from_xyz(1000.5, -17.25, 42.0));

        let runtime_only = capture_live_terrain_geometry(&world, entity)
            .expect("runtime-only recapture must succeed")
            .digest();
        assert_eq!(baseline, runtime_only);

        world
            .get_mut::<EarthChunk>(entity)
            .expect("chunk must remain present")
            .voxels[0][0][0] = SubstrateMaterial::Air;
        let material_edit = capture_live_terrain_geometry(&world, entity)
            .expect("material recapture must succeed")
            .digest();
        assert_ne!(baseline, material_edit);
    }

    #[test]
    fn missing_locus_fails_closed() {
        let mut world = World::new();
        let entity = world.spawn(EarthChunk::default()).id();

        assert_eq!(
            capture_live_terrain_geometry(&world, entity),
            Err(TerrainLiveGeometryError::MissingLatticeLocus(entity))
        );
    }

    #[test]
    fn missing_chunk_fails_closed() {
        let mut world = World::new();
        let entity = world.spawn(EarthChunkLatticeLocus::new(8, 9, 10)).id();

        assert_eq!(
            capture_live_terrain_geometry(&world, entity),
            Err(TerrainLiveGeometryError::MissingEarthChunk(entity))
        );
    }

    #[test]
    fn duplicate_locus_fails_closed_even_if_conflict_has_no_chunk() {
        let mut world = World::new();
        let requested = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new(-3, 7, 11),
            ))
            .id();
        let conflicting = world
            .spawn(EarthChunkLatticeLocus::new(-3, 7, 11))
            .id();

        assert_eq!(
            capture_live_terrain_geometry(&world, requested),
            Err(TerrainLiveGeometryError::DuplicateLatticeLocus {
                coord: EarthChunkLatticeCoord::new(-3, 7, 11),
                requested,
                conflicting,
            })
        );
    }

    #[test]
    fn same_materials_at_different_ecs_loci_have_different_identity() {
        let mut world = World::new();
        let first = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new(0, 0, 0),
            ))
            .id();
        let second = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new(0, 0, 1),
            ))
            .id();

        let first =
            capture_live_terrain_geometry(&world, first).expect("first capture must succeed");
        let second =
            capture_live_terrain_geometry(&world, second).expect("second capture must succeed");
        assert_ne!(first.digest(), second.digest());
    }
}
