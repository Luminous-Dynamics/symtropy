// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact, read-only occupancy projection over qualified Terrain material evidence.
//!
//! This module projects the current frozen #984 categorical material vocabulary
//! into one deliberately narrow occupied/empty geometry profile. It owns no
//! Terrain state and establishes no support, strength, friction, permeability,
//! atmosphere, navigation, metric, or material-transition authority.

use core::fmt;

use crate::geometry_kernel::{
    EarthChunkLatticeCoord, TerrainGeometryDigest, TerrainGeometryError,
    TerrainGeometryQueryReceipt, TerrainMaterialCode, TerrainVoxelBox, TerrainVoxelIndex,
};

/// Canonical schema/profile for the first categorical occupancy projection.
pub const TERRAIN_OCCUPANCY_SCHEMA_VERSION: u32 = 1;

const TERRAIN_OCCUPANCY_QUERY_DOMAIN: &[u8] = b"symtropy.terrain.occupancy.query.v1\0";
const OCCUPANCY_OBSERVATION_CANONICAL_BYTES_LEN: usize = 3 + 1 + 32 + 1;

/// Exact occupied/empty result under `CategoricalVoxelOccupancyV1`.
///
/// `Occupied` means only that the Terrain category contributes occupied voxel
/// volume under this profile. It is not a mechanical-support or edit-permission
/// claim. `Empty` does not mean vacuum or absence of fluids/agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerrainVoxelOccupancyV1 {
    Empty,
    Occupied,
}

impl TerrainVoxelOccupancyV1 {
    pub const fn code(self) -> u8 {
        match self {
            Self::Empty => 0,
            Self::Occupied => 1,
        }
    }
}

/// One exact occupancy observation retaining the categorical source evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainOccupancyObservationV1 {
    voxel: TerrainVoxelIndex,
    source_material: TerrainMaterialCode,
    source_observation_digest: TerrainGeometryDigest,
    occupancy: TerrainVoxelOccupancyV1,
}

impl TerrainOccupancyObservationV1 {
    pub const fn voxel(self) -> TerrainVoxelIndex {
        self.voxel
    }

    pub const fn source_material(self) -> TerrainMaterialCode {
        self.source_material
    }

    pub const fn source_observation_digest(self) -> TerrainGeometryDigest {
        self.source_observation_digest
    }

    pub const fn occupancy(self) -> TerrainVoxelOccupancyV1 {
        self.occupancy
    }
}

/// Exact bounded occupancy projection derived from one validated #984 query.
///
/// The receipt intentionally retains both source material/query evidence and the
/// projected occupancy result. It does not introduce a second geometry digest or
/// claim that equal occupancy erases distinct categorical material histories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainOccupancyQueryReceiptV1 {
    schema_version: u32,
    source_snapshot_digest: TerrainGeometryDigest,
    source_query_digest: TerrainGeometryDigest,
    chunk: EarthChunkLatticeCoord,
    bounds: TerrainVoxelBox,
    observations: Vec<TerrainOccupancyObservationV1>,
}

impl TerrainOccupancyQueryReceiptV1 {
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn source_snapshot_digest(&self) -> TerrainGeometryDigest {
        self.source_snapshot_digest
    }

    pub const fn source_query_digest(&self) -> TerrainGeometryDigest {
        self.source_query_digest
    }

    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn bounds(&self) -> TerrainVoxelBox {
        self.bounds
    }

    pub fn observations(&self) -> &[TerrainOccupancyObservationV1] {
        &self.observations
    }

    /// Re-checks the occupancy profile, bounded canonical order, and the exact
    /// material->occupancy mapping retained in this immutable receipt.
    ///
    /// Source observation/query digests are opaque qualified #984 evidence here;
    /// they were validated before construction and are not recomputed by this
    /// derived projection layer.
    pub fn validate(&self) -> Result<(), TerrainOccupancyProjectionError> {
        if self.schema_version != TERRAIN_OCCUPANCY_SCHEMA_VERSION {
            return Err(TerrainOccupancyProjectionError::UnsupportedProfile(
                self.schema_version,
            ));
        }

        let bounds = TerrainVoxelBox::new(self.bounds.min, self.bounds.max_exclusive)
            .map_err(TerrainOccupancyProjectionError::Source)?;
        let expected_count = bounds.voxel_count();
        if self.observations.len() != expected_count {
            return Err(TerrainOccupancyProjectionError::IncompleteProjection {
                expected: expected_count,
                actual: self.observations.len(),
            });
        }

        let mut cursor = 0_usize;
        for x in bounds.min.x..bounds.max_exclusive[0] {
            for y in bounds.min.y..bounds.max_exclusive[1] {
                for z in bounds.min.z..bounds.max_exclusive[2] {
                    let expected_voxel = TerrainVoxelIndex { x, y, z };
                    let observation = self.observations.get(cursor).ok_or(
                        TerrainOccupancyProjectionError::IncompleteProjection {
                            expected: expected_count,
                            actual: cursor,
                        },
                    )?;
                    if observation.voxel != expected_voxel {
                        return Err(TerrainOccupancyProjectionError::NonCanonicalProjectedVoxel {
                            expected: expected_voxel,
                            actual: observation.voxel,
                        });
                    }
                    let expected = project_material_occupancy_v1(observation.source_material)?;
                    if observation.occupancy != expected {
                        return Err(TerrainOccupancyProjectionError::OccupancyMismatch {
                            voxel: observation.voxel,
                            material: observation.source_material,
                            expected,
                            actual: observation.occupancy,
                        });
                    }
                    cursor += 1;
                }
            }
        }
        Ok(())
    }

    /// Serializer-independent exact evidence bytes for this derived receipt.
    ///
    /// Grammar:
    ///
    /// ```text
    /// domain
    /// || u32le(occupancy_schema_version)
    /// || source_snapshot_digest[32]
    /// || source_query_digest[32]
    /// || i32le(chunk_x) || i32le(chunk_y) || i32le(chunk_z)
    /// || bounds[min_xyz,max_exclusive_xyz][6]
    /// || u32le(observation_count)
    /// || repeated canonical observations:
    ///      voxel_xyz[3]
    ///      || source_material_code[1]
    ///      || source_observation_digest[32]
    ///      || occupancy_code[1]
    /// ```
    ///
    /// These bytes are an occupancy **evidence receipt**, not a replacement for
    /// the categorical #984 material identity they retain.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(
            TERRAIN_OCCUPANCY_QUERY_DOMAIN.len()
                + 4
                + 32
                + 32
                + 12
                + 6
                + 4
                + self.observations.len() * OCCUPANCY_OBSERVATION_CANONICAL_BYTES_LEN,
        );
        bytes.extend_from_slice(TERRAIN_OCCUPANCY_QUERY_DOMAIN);
        bytes.extend_from_slice(&self.schema_version.to_le_bytes());
        bytes.extend_from_slice(&self.source_snapshot_digest.bytes());
        bytes.extend_from_slice(&self.source_query_digest.bytes());
        bytes.extend_from_slice(&self.chunk.x.to_le_bytes());
        bytes.extend_from_slice(&self.chunk.y.to_le_bytes());
        bytes.extend_from_slice(&self.chunk.z.to_le_bytes());
        bytes.extend_from_slice(&[
            self.bounds.min.x,
            self.bounds.min.y,
            self.bounds.min.z,
            self.bounds.max_exclusive[0],
            self.bounds.max_exclusive[1],
            self.bounds.max_exclusive[2],
        ]);
        bytes.extend_from_slice(&(self.observations.len() as u32).to_le_bytes());
        for observation in &self.observations {
            bytes.extend_from_slice(&[
                observation.voxel.x,
                observation.voxel.y,
                observation.voxel.z,
                observation.source_material.as_u8(),
            ]);
            bytes.extend_from_slice(&observation.source_observation_digest.bytes());
            bytes.push(observation.occupancy.code());
        }
        bytes
    }
}

/// Projects one current frozen material code into the v1 occupancy vocabulary.
///
/// The mapping is deliberately explicit rather than `material != AIR`, so a
/// future material code cannot silently join the profile without a versioned
/// review/extension.
pub fn project_material_occupancy_v1(
    material: TerrainMaterialCode,
) -> Result<TerrainVoxelOccupancyV1, TerrainOccupancyProjectionError> {
    if material == TerrainMaterialCode::AIR {
        return Ok(TerrainVoxelOccupancyV1::Empty);
    }
    if material == TerrainMaterialCode::BEDROCK
        || material == TerrainMaterialCode::DOLOMITE
        || material == TerrainMaterialCode::PYRITE_TAILING
        || material == TerrainMaterialCode::QUARTZITE
    {
        return Ok(TerrainVoxelOccupancyV1::Occupied);
    }
    Err(TerrainOccupancyProjectionError::UnsupportedMaterialCode(
        material,
    ))
}

/// Derives one exact occupancy receipt from a fully validated #984 bounded query.
pub fn project_occupancy_query_v1(
    source: &TerrainGeometryQueryReceipt,
) -> Result<TerrainOccupancyQueryReceiptV1, TerrainOccupancyProjectionError> {
    source
        .validate()
        .map_err(TerrainOccupancyProjectionError::Source)?;

    let expected = source.observations().len();
    let mut observations = Vec::with_capacity(expected);
    for observation in source.observations() {
        observations.push(TerrainOccupancyObservationV1 {
            voxel: observation.voxel(),
            source_material: observation.material(),
            source_observation_digest: observation.digest(),
            occupancy: project_material_occupancy_v1(observation.material())?,
        });
    }

    if observations.len() != expected {
        return Err(TerrainOccupancyProjectionError::IncompleteProjection {
            expected,
            actual: observations.len(),
        });
    }

    let receipt = TerrainOccupancyQueryReceiptV1 {
        schema_version: TERRAIN_OCCUPANCY_SCHEMA_VERSION,
        source_snapshot_digest: source.snapshot_digest(),
        source_query_digest: source.digest(),
        chunk: source.chunk(),
        bounds: source.bounds(),
        observations,
    };
    receipt.validate()?;
    Ok(receipt)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainOccupancyProjectionError {
    Source(TerrainGeometryError),
    UnsupportedProfile(u32),
    UnsupportedMaterialCode(TerrainMaterialCode),
    IncompleteProjection {
        expected: usize,
        actual: usize,
    },
    NonCanonicalProjectedVoxel {
        expected: TerrainVoxelIndex,
        actual: TerrainVoxelIndex,
    },
    OccupancyMismatch {
        voxel: TerrainVoxelIndex,
        material: TerrainMaterialCode,
        expected: TerrainVoxelOccupancyV1,
        actual: TerrainVoxelOccupancyV1,
    },
}

impl fmt::Display for TerrainOccupancyProjectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => write!(f, "source Terrain geometry query is invalid: {error}"),
            Self::UnsupportedProfile(version) => {
                write!(f, "unsupported Terrain occupancy profile {version}")
            }
            Self::UnsupportedMaterialCode(material) => write!(
                f,
                "Terrain material code {} is not mapped by occupancy profile v1",
                material.as_u8()
            ),
            Self::IncompleteProjection { expected, actual } => write!(
                f,
                "incomplete Terrain occupancy projection: expected {expected} observations, got {actual}"
            ),
            Self::NonCanonicalProjectedVoxel { expected, actual } => write!(
                f,
                "non-canonical Terrain occupancy order: expected {expected:?}, got {actual:?}"
            ),
            Self::OccupancyMismatch {
                voxel,
                material,
                expected,
                actual,
            } => write!(
                f,
                "Terrain occupancy mismatch at {voxel:?} for material {}: expected {expected:?}, got {actual:?}",
                material.as_u8()
            ),
        }
    }
}

impl std::error::Error for TerrainOccupancyProjectionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Source(error) => Some(error),
            _ => None,
        }
    }
}

impl From<TerrainGeometryError> for TerrainOccupancyProjectionError {
    fn from(value: TerrainGeometryError) -> Self {
        Self::Source(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry_kernel::{
        TERRAIN_GEOMETRY_VOXEL_COUNT, TerrainGeometrySnapshot, TerrainVoxelBox,
    };

    fn flat_index(x: usize, y: usize, z: usize) -> usize {
        (x * 16 * 16) + (y * 16) + z
    }

    fn one_voxel_box(x: u8, y: u8, z: u8) -> TerrainVoxelBox {
        TerrainVoxelBox::new(
            TerrainVoxelIndex::new(x, y, z).expect("test voxel must be bounded"),
            [x + 1, y + 1, z + 1],
        )
        .expect("single-voxel box must be valid")
    }

    fn default_snapshot(chunk: EarthChunkLatticeCoord) -> TerrainGeometrySnapshot {
        TerrainGeometrySnapshot::from_material_codes(
            chunk,
            Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT]),
        )
    }

    fn to_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            output.push(HEX[usize::from(byte >> 4)] as char);
            output.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        output
    }

    #[test]
    fn current_material_vocabulary_has_explicit_frozen_mapping() {
        let cases = [
            (TerrainMaterialCode::AIR, TerrainVoxelOccupancyV1::Empty),
            (
                TerrainMaterialCode::BEDROCK,
                TerrainVoxelOccupancyV1::Occupied,
            ),
            (
                TerrainMaterialCode::DOLOMITE,
                TerrainVoxelOccupancyV1::Occupied,
            ),
            (
                TerrainMaterialCode::PYRITE_TAILING,
                TerrainVoxelOccupancyV1::Occupied,
            ),
            (
                TerrainMaterialCode::QUARTZITE,
                TerrainVoxelOccupancyV1::Occupied,
            ),
        ];
        for (material, expected) in cases {
            assert_eq!(project_material_occupancy_v1(material), Ok(expected));
        }
    }

    #[test]
    fn bounded_projection_preserves_source_order_and_evidence() {
        let snapshot = default_snapshot(EarthChunkLatticeCoord::new(-2, 7, 11));
        let source = snapshot
            .query(
                TerrainVoxelBox::new(TerrainVoxelIndex::new(1, 2, 3).unwrap(), [3, 4, 5])
                    .unwrap(),
            )
            .unwrap();
        let projected = project_occupancy_query_v1(&source).unwrap();

        assert_eq!(projected.chunk(), source.chunk());
        assert_eq!(projected.bounds(), source.bounds());
        assert_eq!(projected.source_snapshot_digest(), source.snapshot_digest());
        assert_eq!(projected.source_query_digest(), source.digest());
        assert_eq!(projected.observations().len(), source.observations().len());
        for (derived, original) in projected.observations().iter().zip(source.observations()) {
            assert_eq!(derived.voxel(), original.voxel());
            assert_eq!(derived.source_material(), original.material());
            assert_eq!(derived.source_observation_digest(), original.digest());
            assert_eq!(derived.occupancy(), TerrainVoxelOccupancyV1::Occupied);
        }
        projected.validate().unwrap();
    }

    #[test]
    fn air_projects_to_empty_without_erasing_source_material() {
        let chunk = EarthChunkLatticeCoord::new(0, 0, 0);
        let mut materials = Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT]);
        materials[flat_index(2, 3, 4)] = TerrainMaterialCode::AIR;
        let snapshot = TerrainGeometrySnapshot::from_material_codes(chunk, materials);
        let source = snapshot.query(one_voxel_box(2, 3, 4)).unwrap();
        let projected = project_occupancy_query_v1(&source).unwrap();
        let observation = projected.observations()[0];

        assert_eq!(observation.source_material(), TerrainMaterialCode::AIR);
        assert_eq!(observation.occupancy(), TerrainVoxelOccupancyV1::Empty);
    }

    #[test]
    fn occupied_to_occupied_material_change_keeps_occupancy_but_not_source_evidence() {
        let chunk = EarthChunkLatticeCoord::new(5, -4, 3);
        let base = Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT]);
        let before = TerrainGeometrySnapshot::from_material_codes(chunk, base.clone());
        let mut changed = base;
        changed[flat_index(8, 9, 10)] = TerrainMaterialCode::QUARTZITE;
        let after = TerrainGeometrySnapshot::from_material_codes(chunk, changed);
        let bounds = one_voxel_box(8, 9, 10);

        let before = project_occupancy_query_v1(&before.query(bounds).unwrap()).unwrap();
        let after = project_occupancy_query_v1(&after.query(bounds).unwrap()).unwrap();

        assert_eq!(
            before.observations()[0].occupancy(),
            TerrainVoxelOccupancyV1::Occupied
        );
        assert_eq!(
            after.observations()[0].occupancy(),
            TerrainVoxelOccupancyV1::Occupied
        );
        assert_ne!(
            before.observations()[0].source_material(),
            after.observations()[0].source_material()
        );
        assert_ne!(before.source_query_digest(), after.source_query_digest());
        assert_ne!(before.canonical_bytes(), after.canonical_bytes());
    }

    #[test]
    fn occupied_to_air_changes_projected_occupancy() {
        let chunk = EarthChunkLatticeCoord::new(-3, 2, 1);
        let base = Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT]);
        let before = TerrainGeometrySnapshot::from_material_codes(chunk, base.clone());
        let mut changed = base;
        changed[flat_index(4, 5, 6)] = TerrainMaterialCode::AIR;
        let after = TerrainGeometrySnapshot::from_material_codes(chunk, changed);
        let bounds = one_voxel_box(4, 5, 6);

        let before = project_occupancy_query_v1(&before.query(bounds).unwrap()).unwrap();
        let after = project_occupancy_query_v1(&after.query(bounds).unwrap()).unwrap();

        assert_eq!(
            before.observations()[0].occupancy(),
            TerrainVoxelOccupancyV1::Occupied
        );
        assert_eq!(
            after.observations()[0].occupancy(),
            TerrainVoxelOccupancyV1::Empty
        );
    }

    #[test]
    fn full_chunk_projection_is_complete_and_valid() {
        let source = default_snapshot(EarthChunkLatticeCoord::new(1, 1, 1))
            .query(TerrainVoxelBox::full_chunk())
            .unwrap();
        let projected = project_occupancy_query_v1(&source).unwrap();
        assert_eq!(projected.observations().len(), TERRAIN_GEOMETRY_VOXEL_COUNT);
        assert!(projected
            .observations()
            .iter()
            .all(|observation| observation.occupancy() == TerrainVoxelOccupancyV1::Occupied));
        projected.validate().unwrap();
    }

    #[test]
    fn repeated_projection_is_exactly_deterministic() {
        let source = default_snapshot(EarthChunkLatticeCoord::new(9, -8, 7))
            .query(one_voxel_box(1, 2, 3))
            .unwrap();
        let first = project_occupancy_query_v1(&source).unwrap();
        let second = project_occupancy_query_v1(&source).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    }

    #[test]
    fn external_single_voxel_receipt_has_frozen_v1_bytes() {
        // Source vectors are already frozen by #984 for this exact fixture:
        // chunk=(1,2,3), all Dolomite, query voxel=(2,3,4).
        let source = default_snapshot(EarthChunkLatticeCoord::new(1, 2, 3))
            .query(one_voxel_box(2, 3, 4))
            .unwrap();
        let projected = project_occupancy_query_v1(&source).unwrap();

        assert_eq!(
            to_hex(&projected.canonical_bytes()),
            "73796d74726f70792e7465727261696e2e6f63637570616e63792e71756572792e76310001000000f7961398ef1ddf5e450aa191bc64901df06e33609b251da3f60f3948fd72dbf17a505c56667d34eaafa1da76980e066e02e0f4187409f30eedd54f4c5168a1170100000002000000030000000203040304050100000002030402d6c2b78646786404b73ef6aabc4cac6545ba4341f14e509f9745dd187f2eda1401"
        );
        assert_eq!(projected.canonical_bytes().len(), 163);
    }
}
