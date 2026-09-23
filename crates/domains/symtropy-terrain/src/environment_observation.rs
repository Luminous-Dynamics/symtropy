// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Read-only LENV-facing observation over qualified local Terrain geometry.
//!
//! This module deliberately preserves the authority boundaries established by
//! the Terrain geometry line:
//!
//! - an exact Terrain lattice observation is not a world/reference-frame claim;
//! - a valid `TerrainMetricProfile` is not an authorized metric for this world;
//! - material identity is not a body-relative friction/support result;
//! - a Bevy `Entity` is only a point-of-use source handle and is never retained
//!   as canonical environmental identity.
//!
//! The first adapter therefore carries only the exact qualified geometry
//! snapshot and bounded query receipt. Metric binding, spatial embedding,
//! surface-interaction interpretation, and mutation remain separate authorities.

use bevy::{ecs::world::World, prelude::Entity};
use std::{error::Error, fmt};

use crate::{
    TerrainGeometryCurrentnessError, TerrainLiveGeometryError, capture_live_terrain_geometry,
    geometry_kernel::{
        EarthChunkLatticeCoord, TerrainGeometryDigest, TerrainGeometryError,
        TerrainGeometryQueryReceipt, TerrainGeometrySnapshot, TerrainVoxelBox,
        TerrainVoxelObservation,
    },
    verify_live_terrain_geometry_current,
};

/// Exact local-lattice observation suitable for downstream environmental
/// composition that does not require physical scale or shared-frame placement.
///
/// The full snapshot is retained intentionally. Terrain's qualified currentness
/// theorem compares the complete chunk snapshot, so retaining only the bounded
/// receipt would weaken the source theorem when an unqueried voxel changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainLocalObservation {
    snapshot: TerrainGeometrySnapshot,
    receipt: TerrainGeometryQueryReceipt,
}

impl TerrainLocalObservation {
    /// The complete exact Terrain snapshot captured at observation time.
    pub const fn snapshot(&self) -> &TerrainGeometrySnapshot {
        &self.snapshot
    }

    /// The bounded exact query receipt derived from `snapshot`.
    pub const fn receipt(&self) -> &TerrainGeometryQueryReceipt {
        &self.receipt
    }

    /// Exact Terrain-lattice chunk locus. This is not a world-frame coordinate.
    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.receipt.chunk()
    }

    pub const fn bounds(&self) -> TerrainVoxelBox {
        self.receipt.bounds()
    }

    pub const fn snapshot_digest(&self) -> TerrainGeometryDigest {
        self.receipt.snapshot_digest()
    }

    pub const fn query_digest(&self) -> TerrainGeometryDigest {
        self.receipt.digest()
    }

    pub fn observations(&self) -> &[TerrainVoxelObservation] {
        self.receipt.observations()
    }

    /// Revalidates the internally retained receipt grammar.
    pub fn validate(&self) -> Result<(), TerrainGeometryError> {
        self.receipt.validate()?;
        if self.snapshot.chunk() != self.receipt.chunk()
            || self.snapshot.digest() != self.receipt.snapshot_digest()
        {
            return Err(TerrainGeometryError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

/// Fail-closed errors at the Terrain -> local-environment observation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainLocalObservationError {
    Capture(TerrainLiveGeometryError),
    Geometry(TerrainGeometryError),
    Currentness(TerrainGeometryCurrentnessError),
}

impl fmt::Display for TerrainLocalObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Capture(error) => write!(f, "Terrain local observation capture failed: {error}"),
            Self::Geometry(error) => write!(f, "Terrain local observation query failed: {error}"),
            Self::Currentness(error) => {
                write!(f, "Terrain local observation currentness failed: {error}")
            }
        }
    }
}

impl Error for TerrainLocalObservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Capture(error) => Some(error),
            Self::Geometry(error) => Some(error),
            Self::Currentness(error) => Some(error),
        }
    }
}

impl From<TerrainLiveGeometryError> for TerrainLocalObservationError {
    fn from(value: TerrainLiveGeometryError) -> Self {
        Self::Capture(value)
    }
}

impl From<TerrainGeometryError> for TerrainLocalObservationError {
    fn from(value: TerrainGeometryError) -> Self {
        Self::Geometry(value)
    }
}

impl From<TerrainGeometryCurrentnessError> for TerrainLocalObservationError {
    fn from(value: TerrainGeometryCurrentnessError) -> Self {
        Self::Currentness(value)
    }
}

/// Capture one exact bounded local-lattice observation from current Terrain.
///
/// `entity` is used only to resolve the qualified source at this invocation. It
/// is not retained in the returned canonical evidence.
pub fn observe_live_terrain_local(
    world: &World,
    entity: Entity,
    bounds: TerrainVoxelBox,
) -> Result<TerrainLocalObservation, TerrainLocalObservationError> {
    let snapshot = capture_live_terrain_geometry(world, entity)?;
    let receipt = snapshot.query(bounds)?;
    receipt.validate()?;
    let observation = TerrainLocalObservation { snapshot, receipt };
    observation.validate()?;
    Ok(observation)
}

/// Revalidate a retained local observation against qualified live Terrain state.
///
/// Currentness remains exactly as strong as the underlying Terrain theorem: the
/// complete retained snapshot must still match at point of use. On success a
/// fresh snapshot and a freshly derived receipt are returned rather than a
/// boolean or the old evidence.
pub fn verify_live_terrain_local_observation_current(
    world: &World,
    entity: Entity,
    expected: &TerrainLocalObservation,
) -> Result<TerrainLocalObservation, TerrainLocalObservationError> {
    let snapshot = verify_live_terrain_geometry_current(world, entity, expected.snapshot())?;
    let receipt = snapshot.query(expected.bounds())?;
    receipt.validate()?;
    let current = TerrainLocalObservation { snapshot, receipt };
    current.validate()?;
    Ok(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        EarthChunk, SubstrateMaterial,
        geometry_kernel::{TerrainMaterialCode, TerrainVoxelIndex},
        live_geometry_provider::EarthChunkLatticeLocus,
    };
    use bevy::prelude::GlobalTransform;

    fn one_voxel_box(x: u8, y: u8, z: u8) -> TerrainVoxelBox {
        TerrainVoxelBox::new(
            TerrainVoxelIndex::new(x, y, z).expect("test voxel must be bounded"),
            [x + 1, y + 1, z + 1],
        )
        .expect("single-voxel bounds must be valid")
    }

    #[test]
    fn captures_exact_snapshot_and_bounded_receipt_without_world_frame_claim() {
        let mut world = World::new();
        let mut chunk = EarthChunk::default();
        chunk.voxels[2][3][4] = SubstrateMaterial::Air;
        let entity = world
            .spawn((chunk, EarthChunkLatticeLocus::new_for_test(7, -3, 11)))
            .id();

        let observed = observe_live_terrain_local(&world, entity, one_voxel_box(2, 3, 4))
            .expect("qualified local observation must succeed");

        observed
            .validate()
            .expect("returned evidence must validate");
        assert_eq!(observed.chunk(), EarthChunkLatticeCoord::new(7, -3, 11));
        assert_eq!(observed.observations().len(), 1);
        assert_eq!(
            observed.observations()[0].material(),
            TerrainMaterialCode::AIR
        );
        assert_eq!(observed.snapshot().digest(), observed.snapshot_digest());
        assert_eq!(observed.receipt().digest(), observed.query_digest());
    }

    #[test]
    fn repeated_unchanged_observation_is_exactly_equal() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(1, 2, 3),
            ))
            .id();
        let bounds = one_voxel_box(4, 5, 6);

        let first = observe_live_terrain_local(&world, entity, bounds).unwrap();
        let second = observe_live_terrain_local(&world, entity, bounds).unwrap();
        assert_eq!(first, second);

        let verified =
            verify_live_terrain_local_observation_current(&world, entity, &first).unwrap();
        assert_eq!(verified, first);
    }

    #[test]
    fn queried_material_mutation_fails_point_of_use_currentness() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(4, 4, 4),
            ))
            .id();
        let expected = observe_live_terrain_local(&world, entity, one_voxel_box(1, 2, 3)).unwrap();

        world.get_mut::<EarthChunk>(entity).unwrap().voxels[1][2][3] = SubstrateMaterial::Air;

        assert!(matches!(
            verify_live_terrain_local_observation_current(&world, entity, &expected),
            Err(TerrainLocalObservationError::Currentness(
                TerrainGeometryCurrentnessError::Stale { .. }
            ))
        ));
    }

    #[test]
    fn remote_mutation_preserves_local_value_but_stales_enclosing_snapshot() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(-2, 8, 5),
            ))
            .id();
        let bounds = one_voxel_box(0, 0, 0);
        let expected = observe_live_terrain_local(&world, entity, bounds).unwrap();

        world.get_mut::<EarthChunk>(entity).unwrap().voxels[15][15][15] = SubstrateMaterial::Air;
        let after = observe_live_terrain_local(&world, entity, bounds).unwrap();

        assert_eq!(expected.observations(), after.observations());
        assert_ne!(expected.snapshot_digest(), after.snapshot_digest());
        assert_ne!(expected.query_digest(), after.query_digest());
        assert!(matches!(
            verify_live_terrain_local_observation_current(&world, entity, &expected),
            Err(TerrainLocalObservationError::Currentness(
                TerrainGeometryCurrentnessError::Stale { .. }
            ))
        ));
    }

    #[test]
    fn runtime_only_state_does_not_stale_exact_local_geometry() {
        let mut world = World::new();
        let entity = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(6, -1, 2),
                GlobalTransform::default(),
            ))
            .id();
        let expected = observe_live_terrain_local(&world, entity, one_voxel_box(3, 3, 3)).unwrap();

        {
            let mut chunk = world.get_mut::<EarthChunk>(entity).unwrap();
            chunk.densities[3][3][3] = 0.125;
            chunk.is_dirty = !chunk.is_dirty;
            chunk.is_rebuilding = !chunk.is_rebuilding;
        }
        world
            .entity_mut(entity)
            .insert(GlobalTransform::from_xyz(100.0, -50.0, 7.5));

        let current =
            verify_live_terrain_local_observation_current(&world, entity, &expected).unwrap();
        assert_eq!(current, expected);
    }

    #[test]
    fn duplicate_exact_lattice_locus_remains_a_source_error() {
        let mut world = World::new();
        let coord = (9, 9, 9);
        let first = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(coord.0, coord.1, coord.2),
            ))
            .id();
        let second = world
            .spawn((
                EarthChunk::default(),
                EarthChunkLatticeLocus::new_for_test(coord.0, coord.1, coord.2),
            ))
            .id();

        assert!(matches!(
            observe_live_terrain_local(&world, first, one_voxel_box(0, 0, 0)),
            Err(TerrainLocalObservationError::Capture(
                TerrainLiveGeometryError::DuplicateLatticeLocus {
                    requested,
                    conflicting,
                    ..
                }
            )) if requested == first && conflicting == second
        ));
    }
}
