// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Point-of-use currentness verification for exact Terrain geometry snapshots.
//!
//! Stable geometry identity remains owned by `geometry_kernel`; this layer does
//! not add a revision, clock, transform, or runtime generation to that identity.
//! Instead it re-captures the requested ECS entity at verification time and
//! requires exact lattice locus and snapshot digest equality.
//!
//! Equality establishes only that `expected` matches the qualified live capture
//! at this invocation. It does not prove where `expected` originally came from,
//! and it does not promise that the mutable world will remain unchanged after
//! this function returns. Callers that require point-of-use geometry should
//! consume the freshly returned snapshot.

use bevy::{ecs::world::World, prelude::Entity};
use std::{error::Error, fmt};

use crate::{
    TerrainLiveGeometryError, capture_live_terrain_geometry,
    geometry_kernel::{EarthChunkLatticeCoord, TerrainGeometryDigest, TerrainGeometrySnapshot},
};

/// Fail-closed currentness errors for a retained exact Terrain snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainGeometryCurrentnessError {
    /// The live provider could not produce a qualified current observation.
    Capture(TerrainLiveGeometryError),
    /// A fresh qualified observation exists but no longer matches `expected`.
    Stale {
        expected_chunk: EarthChunkLatticeCoord,
        current_chunk: EarthChunkLatticeCoord,
        expected_digest: TerrainGeometryDigest,
        current_digest: TerrainGeometryDigest,
    },
}

impl fmt::Display for TerrainGeometryCurrentnessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "live Terrain recapture failed: {error}"),
            Self::Stale {
                expected_chunk,
                current_chunk,
                expected_digest,
                current_digest,
            } => write!(
                f,
                "retained Terrain geometry is stale: expected chunk {expected_chunk:?} digest {expected_digest}, current chunk {current_chunk:?} digest {current_digest}"
            ),
        }
    }
}

impl Error for TerrainGeometryCurrentnessError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Capture(error) => Some(error),
            Self::Stale { .. } => None,
        }
    }
}

impl From<TerrainLiveGeometryError> for TerrainGeometryCurrentnessError {
    fn from(value: TerrainLiveGeometryError) -> Self {
        Self::Capture(value)
    }
}

/// Re-capture `entity` and require it to match `expected` exactly.
///
/// On success, the returned value is the freshly captured snapshot that passed
/// the equality check. The function deliberately returns that observation rather
/// than a boolean so downstream code can consume the exact point-of-use geometry
/// that was verified.
pub fn verify_live_terrain_geometry_current(
    world: &World,
    entity: Entity,
    expected: &TerrainGeometrySnapshot,
) -> Result<TerrainGeometrySnapshot, TerrainGeometryCurrentnessError> {
    let current = capture_live_terrain_geometry(world, entity)?;
    verify_current_snapshot(expected, current)
}

fn verify_current_snapshot(
    expected: &TerrainGeometrySnapshot,
    current: TerrainGeometrySnapshot,
) -> Result<TerrainGeometrySnapshot, TerrainGeometryCurrentnessError> {
    if expected.chunk() != current.chunk() || expected.digest() != current.digest() {
        return Err(TerrainGeometryCurrentnessError::Stale {
            expected_chunk: expected.chunk(),
            current_chunk: current.chunk(),
            expected_digest: expected.digest(),
            current_digest: current.digest(),
        });
    }
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EarthChunk, SubstrateMaterial,
        geometry_kernel::{TERRAIN_GEOMETRY_VOXEL_COUNT, TerrainMaterialCode},
        live_geometry_provider::EarthChunkLatticeLocus,
    };
    use bevy::prelude::GlobalTransform;

    fn snapshot(
        chunk: EarthChunkLatticeCoord,
        edit: Option<(usize, TerrainMaterialCode)>,
    ) -> TerrainGeometrySnapshot {
        let mut materials =
            Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT]);
        if let Some((index, material)) = edit {
            materials[index] = material;
        }
        TerrainGeometrySnapshot::from_material_codes(chunk, materials)
    }

    #[test]
    fn equal_exact_observation_is_current_and_returns_fresh_value() {
        let coord = EarthChunkLatticeCoord::new(3, -5, 8);
        let expected = snapshot(coord, None);
        let current = snapshot(coord, None);
        let current_digest = current.digest();

        let verified = verify_current_snapshot(&expected, current).expect("exact match must verify");
        assert_eq!(verified.chunk(), coord);
        assert_eq!(verified.digest(), current_digest);
    }

    #[test]
    fn material_change_is_stale() {
        let coord = EarthChunkLatticeCoord::new(3, -5, 8);
        let expected = snapshot(coord, None);
        let current = snapshot(coord, Some((0, TerrainMaterialCode::AIR)));
        let current_digest = current.digest();

        assert_eq!(
            verify_current_snapshot(&expected, current),
            Err(TerrainGeometryCurrentnessError::Stale {
                expected_chunk: coord,
                current_chunk: coord,
                expected_digest: expected.digest(),
                current_digest,
            })
        );
    }

    #[test]
    fn lattice_locus_change_is_stale_even_with_same_materials() {
        let expected_coord = EarthChunkLatticeCoord::new(3, -5, 8);
        let current_coord = EarthChunkLatticeCoord::new(3, -5, 9);
        let expected = snapshot(expected_coord, None);
        let current = snapshot(current_coord, None);
        let current_digest = current.digest();

        assert_eq!(
            verify_current_snapshot(&expected, current),
            Err(TerrainGeometryCurrentnessError::Stale {
                expected_chunk: expected_coord,
                current_chunk: current_coord,
                expected_digest: expected.digest(),
                current_digest,
            })
        );
    }

    #[test]
    fn live_wrapper_returns_fresh_snapshot_and_ignores_runtime_only_state() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(7, -4, 2),
                GlobalTransform::default(),
            ))
            .id();
        let expected =
            capture_live_terrain_geometry(&world, entity).expect("initial capture must succeed");

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
            .insert(GlobalTransform::from_xyz(500.25, -31.5, 99.0));

        let verified = verify_live_terrain_geometry_current(&world, entity, &expected)
            .expect("runtime-only changes must not stale exact geometry");
        assert_eq!(verified.chunk(), expected.chunk());
        assert_eq!(verified.digest(), expected.digest());
    }

    #[test]
    fn live_wrapper_detects_material_mutation_as_stale() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(7, -4, 2),
            ))
            .id();
        let expected =
            capture_live_terrain_geometry(&world, entity).expect("initial capture must succeed");

        world
            .get_mut::<EarthChunk>(entity)
            .expect("chunk must remain present")
            .voxels[0][0][0] = SubstrateMaterial::Air;

        let error = verify_live_terrain_geometry_current(&world, entity, &expected)
            .expect_err("material mutation must stale retained geometry");
        match error {
            TerrainGeometryCurrentnessError::Stale {
                expected_chunk,
                current_chunk,
                expected_digest,
                current_digest,
            } => {
                assert_eq!(expected_chunk, expected.chunk());
                assert_eq!(current_chunk, expected.chunk());
                assert_eq!(expected_digest, expected.digest());
                assert_ne!(current_digest, expected.digest());
            }
            other => panic!("expected stale currentness error, got {other:?}"),
        }
    }

    #[test]
    fn live_wrapper_fails_closed_when_chunk_has_no_authoritative_locus() {
        let mut world = World::new();
        let entity = world.spawn(EarthChunk::default()).id();
        let expected = snapshot(EarthChunkLatticeCoord::new(0, 0, 0), None);

        assert_eq!(
            verify_live_terrain_geometry_current(&world, entity, &expected),
            Err(TerrainGeometryCurrentnessError::Capture(
                TerrainLiveGeometryError::MissingLatticeLocus(entity)
            ))
        );
    }
}
