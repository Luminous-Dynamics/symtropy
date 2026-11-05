// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact read-only geometry identity for Terrain's discrete voxel occupancy.
//!
//! This module deliberately excludes floating density, Bevy transforms, render
//! meshes, Rapier handles, dirty/rebuild state, and async task identity from the
//! geometry claim. It attests only the complete discrete material occupancy of
//! one exactly addressed 16^3 Terrain chunk.

use bevy::prelude::{Component, Reflect};
use std::{error::Error, fmt};

use super::{CHUNK_SIZE, EarthChunk, SubstrateMaterial};

pub const TERRAIN_GEOMETRY_SCHEMA_VERSION: u32 = 1;
pub const TERRAIN_GEOMETRY_VOXEL_COUNT: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.terrain.geometry.snapshot.v1\0";
const VOXEL_DOMAIN: &[u8] = b"symtropy.terrain.geometry.voxel.v1\0";
const QUERY_DOMAIN: &[u8] = b"symtropy.terrain.geometry.query.v1\0";

/// Exact lattice identity for one Terrain chunk.
///
/// This is intentionally distinct from `GlobalTransform`. A legacy chunk that
/// has no exact lattice coordinate is not eligible for this authority surface.
#[derive(Component, Reflect, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[reflect(Component)]
pub struct EarthChunkLatticeCoord {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl EarthChunkLatticeCoord {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainVoxelIndex {
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl TerrainVoxelIndex {
    pub fn new(x: u8, y: u8, z: u8) -> Result<Self, TerrainGeometryError> {
        let index = Self { x, y, z };
        index.validate()?;
        Ok(index)
    }

    fn validate(self) -> Result<(), TerrainGeometryError> {
        let limit = CHUNK_SIZE as u8;
        if self.x >= limit || self.y >= limit || self.z >= limit {
            return Err(TerrainGeometryError::VoxelOutsideChunk(self));
        }
        Ok(())
    }
}

/// Half-open `[min, max_exclusive)` query box inside one Terrain chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainVoxelBox {
    pub min: TerrainVoxelIndex,
    pub max_exclusive: [u8; 3],
}

impl TerrainVoxelBox {
    pub fn new(
        min: TerrainVoxelIndex,
        max_exclusive: [u8; 3],
    ) -> Result<Self, TerrainGeometryError> {
        min.validate()?;
        let limit = CHUNK_SIZE as u8;
        if max_exclusive
            .into_iter()
            .any(|value| value == 0 || value > limit)
            || min.x >= max_exclusive[0]
            || min.y >= max_exclusive[1]
            || min.z >= max_exclusive[2]
        {
            return Err(TerrainGeometryError::InvalidQueryBounds {
                min,
                max_exclusive,
            });
        }
        Ok(Self { min, max_exclusive })
    }

    pub fn full_chunk() -> Self {
        Self {
            min: TerrainVoxelIndex { x: 0, y: 0, z: 0 },
            max_exclusive: [CHUNK_SIZE as u8; 3],
        }
    }

    pub fn voxel_count(self) -> usize {
        usize::from(self.max_exclusive[0] - self.min.x)
            * usize::from(self.max_exclusive[1] - self.min.y)
            * usize::from(self.max_exclusive[2] - self.min.z)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainGeometryDigest([u8; 32]);

impl TerrainGeometryDigest {
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = String::with_capacity(64);
        for byte in self.0 {
            output.push(HEX[usize::from(byte >> 4)] as char);
            output.push(HEX[usize::from(byte & 0x0f)] as char);
        }
        output
    }
}

impl fmt::Debug for TerrainGeometryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TerrainGeometryDigest")
            .field(&self.to_hex())
            .finish()
    }
}

impl fmt::Display for TerrainGeometryDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainVoxelObservation {
    chunk: EarthChunkLatticeCoord,
    voxel: TerrainVoxelIndex,
    material: SubstrateMaterial,
    digest: TerrainGeometryDigest,
}

impl TerrainVoxelObservation {
    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn voxel(&self) -> TerrainVoxelIndex {
        self.voxel
    }

    pub const fn material(&self) -> SubstrateMaterial {
        self.material
    }

    pub const fn digest(&self) -> TerrainGeometryDigest {
        self.digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainGeometrySnapshot {
    schema_version: u32,
    chunk: EarthChunkLatticeCoord,
    materials: Box<[SubstrateMaterial; TERRAIN_GEOMETRY_VOXEL_COUNT]>,
    digest: TerrainGeometryDigest,
}

impl TerrainGeometrySnapshot {
    /// Capture the exact discrete material occupancy visible in `chunk` now.
    ///
    /// This method is read-only. Callers that need *current* world truth must
    /// invoke it against the owning ECS state at the point of use; retaining an
    /// old snapshot does not make that snapshot perpetually current.
    pub fn capture(coord: EarthChunkLatticeCoord, chunk: &EarthChunk) -> Self {
        let mut materials = Box::new([SubstrateMaterial::Air; TERRAIN_GEOMETRY_VOXEL_COUNT]);
        let mut cursor = 0_usize;
        for x in 0..CHUNK_SIZE {
            for y in 0..CHUNK_SIZE {
                for z in 0..CHUNK_SIZE {
                    materials[cursor] = chunk.voxels[x][y][z];
                    cursor += 1;
                }
            }
        }
        debug_assert_eq!(cursor, TERRAIN_GEOMETRY_VOXEL_COUNT);
        let digest = snapshot_digest(coord, &materials);
        Self {
            schema_version: TERRAIN_GEOMETRY_SCHEMA_VERSION,
            chunk: coord,
            materials,
            digest,
        }
    }

    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn digest(&self) -> TerrainGeometryDigest {
        self.digest
    }

    pub fn observe(
        &self,
        voxel: TerrainVoxelIndex,
    ) -> Result<TerrainVoxelObservation, TerrainGeometryError> {
        voxel.validate()?;
        let material = self.materials[flat_index(voxel)];
        Ok(TerrainVoxelObservation {
            chunk: self.chunk,
            voxel,
            material,
            digest: voxel_digest(self.chunk, voxel, material),
        })
    }

    pub fn query(
        &self,
        bounds: TerrainVoxelBox,
    ) -> Result<TerrainGeometryQueryReceipt, TerrainGeometryError> {
        // Re-validate even though constructors are checked. This keeps the
        // query boundary fail-closed if a future wire form is added.
        let bounds = TerrainVoxelBox::new(bounds.min, bounds.max_exclusive)?;
        let expected = bounds.voxel_count();
        let mut observations = Vec::with_capacity(expected);
        for x in bounds.min.x..bounds.max_exclusive[0] {
            for y in bounds.min.y..bounds.max_exclusive[1] {
                for z in bounds.min.z..bounds.max_exclusive[2] {
                    observations.push(self.observe(TerrainVoxelIndex { x, y, z })?);
                }
            }
        }
        if observations.len() != expected {
            return Err(TerrainGeometryError::IncompleteQuery {
                expected,
                actual: observations.len(),
            });
        }
        let digest = query_digest(self.digest, bounds, &observations);
        Ok(TerrainGeometryQueryReceipt {
            schema_version: TERRAIN_GEOMETRY_SCHEMA_VERSION,
            snapshot_digest: self.digest,
            chunk: self.chunk,
            bounds,
            observations,
            digest,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainGeometryQueryReceipt {
    schema_version: u32,
    snapshot_digest: TerrainGeometryDigest,
    chunk: EarthChunkLatticeCoord,
    bounds: TerrainVoxelBox,
    observations: Vec<TerrainVoxelObservation>,
    digest: TerrainGeometryDigest,
}

impl TerrainGeometryQueryReceipt {
    pub const fn schema_version(&self) -> u32 {
        self.schema_version
    }

    pub const fn snapshot_digest(&self) -> TerrainGeometryDigest {
        self.snapshot_digest
    }

    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn bounds(&self) -> TerrainVoxelBox {
        self.bounds
    }

    pub fn observations(&self) -> &[TerrainVoxelObservation] {
        &self.observations
    }

    pub const fn digest(&self) -> TerrainGeometryDigest {
        self.digest
    }

    pub fn validate(&self) -> Result<(), TerrainGeometryError> {
        if self.schema_version != TERRAIN_GEOMETRY_SCHEMA_VERSION {
            return Err(TerrainGeometryError::UnsupportedSchema(self.schema_version));
        }
        let bounds = TerrainVoxelBox::new(self.bounds.min, self.bounds.max_exclusive)?;
        let expected = bounds.voxel_count();
        if self.observations.len() != expected {
            return Err(TerrainGeometryError::IncompleteQuery {
                expected,
                actual: self.observations.len(),
            });
        }
        let mut cursor = 0_usize;
        for x in bounds.min.x..bounds.max_exclusive[0] {
            for y in bounds.min.y..bounds.max_exclusive[1] {
                for z in bounds.min.z..bounds.max_exclusive[2] {
                    let expected_voxel = TerrainVoxelIndex { x, y, z };
                    let observation = self
                        .observations
                        .get(cursor)
                        .ok_or(TerrainGeometryError::IncompleteQuery {
                            expected,
                            actual: cursor,
                        })?;
                    if observation.chunk != self.chunk || observation.voxel != expected_voxel {
                        return Err(TerrainGeometryError::NonCanonicalObservation {
                            expected: expected_voxel,
                            actual: observation.voxel,
                        });
                    }
                    if observation.digest
                        != voxel_digest(observation.chunk, observation.voxel, observation.material)
                    {
                        return Err(TerrainGeometryError::ObservationDigestMismatch(
                            observation.voxel,
                        ));
                    }
                    cursor += 1;
                }
            }
        }
        let expected_digest = query_digest(self.snapshot_digest, bounds, &self.observations);
        if expected_digest != self.digest {
            return Err(TerrainGeometryError::ReceiptDigestMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainGeometryError {
    VoxelOutsideChunk(TerrainVoxelIndex),
    InvalidQueryBounds {
        min: TerrainVoxelIndex,
        max_exclusive: [u8; 3],
    },
    UnsupportedSchema(u32),
    IncompleteQuery {
        expected: usize,
        actual: usize,
    },
    NonCanonicalObservation {
        expected: TerrainVoxelIndex,
        actual: TerrainVoxelIndex,
    },
    ObservationDigestMismatch(TerrainVoxelIndex),
    ReceiptDigestMismatch,
}

impl fmt::Display for TerrainGeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VoxelOutsideChunk(voxel) => write!(f, "voxel {voxel:?} is outside Terrain chunk"),
            Self::InvalidQueryBounds { min, max_exclusive } => write!(
                f,
                "invalid Terrain geometry query bounds {min:?}..{max_exclusive:?}"
            ),
            Self::UnsupportedSchema(version) => {
                write!(f, "unsupported Terrain geometry schema {version}")
            }
            Self::IncompleteQuery { expected, actual } => write!(
                f,
                "incomplete Terrain geometry query: expected {expected} observations, got {actual}"
            ),
            Self::NonCanonicalObservation { expected, actual } => write!(
                f,
                "non-canonical Terrain observation: expected {expected:?}, got {actual:?}"
            ),
            Self::ObservationDigestMismatch(voxel) => {
                write!(f, "Terrain voxel observation digest mismatch at {voxel:?}")
            }
            Self::ReceiptDigestMismatch => write!(f, "Terrain geometry receipt digest mismatch"),
        }
    }
}

impl Error for TerrainGeometryError {}

fn flat_index(voxel: TerrainVoxelIndex) -> usize {
    (usize::from(voxel.x) * CHUNK_SIZE * CHUNK_SIZE)
        + (usize::from(voxel.y) * CHUNK_SIZE)
        + usize::from(voxel.z)
}

fn material_tag(material: SubstrateMaterial) -> u8 {
    match material {
        SubstrateMaterial::Air => 0,
        SubstrateMaterial::Bedrock => 1,
        SubstrateMaterial::Dolomite => 2,
        SubstrateMaterial::PyriteTailing => 3,
        SubstrateMaterial::Quartzite => 4,
    }
}

fn push_coord(bytes: &mut Vec<u8>, coord: EarthChunkLatticeCoord) {
    bytes.extend_from_slice(&coord.x.to_le_bytes());
    bytes.extend_from_slice(&coord.y.to_le_bytes());
    bytes.extend_from_slice(&coord.z.to_le_bytes());
}

fn snapshot_digest(
    coord: EarthChunkLatticeCoord,
    materials: &[SubstrateMaterial; TERRAIN_GEOMETRY_VOXEL_COUNT],
) -> TerrainGeometryDigest {
    let mut bytes = Vec::with_capacity(
        SNAPSHOT_DOMAIN.len() + 4 + 12 + 4 + TERRAIN_GEOMETRY_VOXEL_COUNT,
    );
    bytes.extend_from_slice(SNAPSHOT_DOMAIN);
    bytes.extend_from_slice(&TERRAIN_GEOMETRY_SCHEMA_VERSION.to_le_bytes());
    push_coord(&mut bytes, coord);
    bytes.extend_from_slice(&(CHUNK_SIZE as u32).to_le_bytes());
    bytes.extend(materials.iter().copied().map(material_tag));
    TerrainGeometryDigest(sha256(&bytes))
}

fn voxel_digest(
    coord: EarthChunkLatticeCoord,
    voxel: TerrainVoxelIndex,
    material: SubstrateMaterial,
) -> TerrainGeometryDigest {
    let mut bytes = Vec::with_capacity(VOXEL_DOMAIN.len() + 4 + 12 + 4);
    bytes.extend_from_slice(VOXEL_DOMAIN);
    bytes.extend_from_slice(&TERRAIN_GEOMETRY_SCHEMA_VERSION.to_le_bytes());
    push_coord(&mut bytes, coord);
    bytes.extend_from_slice(&[voxel.x, voxel.y, voxel.z, material_tag(material)]);
    TerrainGeometryDigest(sha256(&bytes))
}

fn query_digest(
    snapshot_digest: TerrainGeometryDigest,
    bounds: TerrainVoxelBox,
    observations: &[TerrainVoxelObservation],
) -> TerrainGeometryDigest {
    let mut bytes = Vec::with_capacity(QUERY_DOMAIN.len() + 4 + 32 + 6 + 4 + observations.len() * 32);
    bytes.extend_from_slice(QUERY_DOMAIN);
    bytes.extend_from_slice(&TERRAIN_GEOMETRY_SCHEMA_VERSION.to_le_bytes());
    bytes.extend_from_slice(&snapshot_digest.bytes());
    bytes.extend_from_slice(&[
        bounds.min.x,
        bounds.min.y,
        bounds.min.z,
        bounds.max_exclusive[0],
        bounds.max_exclusive[1],
        bounds.max_exclusive[2],
    ]);
    bytes.extend_from_slice(&(observations.len() as u32).to_le_bytes());
    for observation in observations {
        bytes.extend_from_slice(&observation.digest.bytes());
    }
    TerrainGeometryDigest(sha256(&bytes))
}

/// Minimal dependency-free SHA-256 used only for deterministic content identity.
/// The implementation is checked against the FIPS 180-4 `abc` vector below.
fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1,
        0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786,
        0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147,
        0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a,
        0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h = [
        0x6a09e667_u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (input.len() as u64).wrapping_mul(8);
    let mut padded = Vec::with_capacity(input.len() + 72);
    padded.extend_from_slice(input);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for block in padded.chunks_exact(64) {
        let mut w = [0_u32; 64];
        for (index, word) in w.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let choose = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(choose)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let majority = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(majority);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut output = [0_u8; 32];
    for (index, word) in h.into_iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(coord: EarthChunkLatticeCoord, chunk: &EarthChunk) -> TerrainGeometrySnapshot {
        TerrainGeometrySnapshot::capture(coord, chunk)
    }

    #[test]
    fn sha256_matches_fips_abc_vector() {
        assert_eq!(
            TerrainGeometryDigest(sha256(b"abc")).to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn identical_discrete_state_has_identical_snapshot_identity() {
        let chunk = EarthChunk::default();
        let coord = EarthChunkLatticeCoord::new(4, -2, 9);
        assert_eq!(snapshot(coord, &chunk), snapshot(coord, &chunk));
    }

    #[test]
    fn one_voxel_change_changes_snapshot_and_only_that_local_observation() {
        let mut before_chunk = EarthChunk::default();
        let coord = EarthChunkLatticeCoord::new(1, 2, 3);
        let before = snapshot(coord, &before_chunk);
        let changed = TerrainVoxelIndex::new(2, 3, 4).unwrap();
        let stable = TerrainVoxelIndex::new(8, 8, 8).unwrap();
        let changed_before = before.observe(changed).unwrap();
        let stable_before = before.observe(stable).unwrap();

        before_chunk.voxels[2][3][4] = SubstrateMaterial::Air;
        let after = snapshot(coord, &before_chunk);
        assert_ne!(before.digest(), after.digest());
        assert_ne!(changed_before.digest(), after.observe(changed).unwrap().digest());
        assert_eq!(stable_before.digest(), after.observe(stable).unwrap().digest());
    }

    #[test]
    fn runtime_only_fields_do_not_change_discrete_geometry_identity() {
        let mut chunk = EarthChunk::default();
        let coord = EarthChunkLatticeCoord::new(0, 0, 0);
        let before = snapshot(coord, &chunk);
        chunk.densities[0][0][0] = 0.125;
        chunk.is_dirty = !chunk.is_dirty;
        chunk.is_rebuilding = !chunk.is_rebuilding;
        let after = snapshot(coord, &chunk);
        assert_eq!(before.digest(), after.digest());
    }

    #[test]
    fn exact_chunk_coordinate_participates_in_identity() {
        let chunk = EarthChunk::default();
        let a = snapshot(EarthChunkLatticeCoord::new(0, 0, 0), &chunk);
        let b = snapshot(EarthChunkLatticeCoord::new(1, 0, 0), &chunk);
        assert_ne!(a.digest(), b.digest());
    }

    #[test]
    fn full_chunk_query_is_complete_and_canonical() {
        let mut chunk = EarthChunk::default();
        chunk.voxels[0][0][0] = SubstrateMaterial::Air;
        let snapshot = snapshot(EarthChunkLatticeCoord::new(-1, 7, 2), &chunk);
        let receipt = snapshot.query(TerrainVoxelBox::full_chunk()).unwrap();
        assert_eq!(receipt.observations().len(), TERRAIN_GEOMETRY_VOXEL_COUNT);
        assert_eq!(receipt.observations()[0].voxel(), TerrainVoxelIndex::new(0, 0, 0).unwrap());
        assert_eq!(receipt.observations()[0].material(), SubstrateMaterial::Air);
        assert_eq!(
            receipt.observations().last().unwrap().voxel(),
            TerrainVoxelIndex::new(15, 15, 15).unwrap()
        );
        receipt.validate().unwrap();
    }

    #[test]
    fn bounded_subquery_contains_exact_requested_set() {
        let chunk = EarthChunk::default();
        let snapshot = snapshot(EarthChunkLatticeCoord::new(0, 0, 0), &chunk);
        let bounds = TerrainVoxelBox::new(TerrainVoxelIndex::new(2, 4, 6).unwrap(), [5, 6, 9]).unwrap();
        let receipt = snapshot.query(bounds).unwrap();
        assert_eq!(receipt.observations().len(), 3 * 2 * 3);
        assert_eq!(receipt.observations().first().unwrap().voxel(), TerrainVoxelIndex::new(2, 4, 6).unwrap());
        assert_eq!(receipt.observations().last().unwrap().voxel(), TerrainVoxelIndex::new(4, 5, 8).unwrap());
        receipt.validate().unwrap();
    }

    #[test]
    fn repeated_query_has_exactly_stable_receipt_identity() {
        let chunk = EarthChunk::default();
        let snapshot = snapshot(EarthChunkLatticeCoord::new(3, 3, 3), &chunk);
        let bounds = TerrainVoxelBox::new(TerrainVoxelIndex::new(1, 1, 1).unwrap(), [4, 5, 6]).unwrap();
        assert_eq!(snapshot.query(bounds).unwrap(), snapshot.query(bounds).unwrap());
    }

    #[test]
    fn invalid_bounds_fail_closed() {
        let min = TerrainVoxelIndex::new(4, 4, 4).unwrap();
        assert!(TerrainVoxelBox::new(min, [4, 5, 5]).is_err());
        assert!(TerrainVoxelBox::new(min, [17, 5, 5]).is_err());
        assert!(TerrainVoxelIndex::new(16, 0, 0).is_err());
    }

    #[test]
    fn capture_and_query_are_read_only_for_chunk_state() {
        let chunk = EarthChunk::default();
        let voxels_before = chunk.voxels;
        let densities_before = chunk.densities;
        let dirty_before = chunk.is_dirty;
        let rebuilding_before = chunk.is_rebuilding;

        let snapshot = TerrainGeometrySnapshot::capture(EarthChunkLatticeCoord::new(1, 1, 1), &chunk);
        let _ = snapshot.query(TerrainVoxelBox::full_chunk()).unwrap();

        assert_eq!(chunk.voxels, voxels_before);
        assert_eq!(chunk.densities, densities_before);
        assert_eq!(chunk.is_dirty, dirty_before);
        assert_eq!(chunk.is_rebuilding, rebuilding_before);
    }
}
