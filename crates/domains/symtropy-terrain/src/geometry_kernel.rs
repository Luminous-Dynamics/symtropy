// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Dependency-free exact identity kernel for Terrain's discrete geometry.
//!
//! This module deliberately knows nothing about Bevy, ECS entities, transforms,
//! meshes, Rapier, async rebuild state, renderer state, or the mutable
//! `EarthChunk` integration surface. It owns only the canonical finite-lattice
//! identity/query theorem. A separate adapter must map real Terrain state into
//! the stable material-code vocabulary below.

use std::{error::Error, fmt};

pub const TERRAIN_GEOMETRY_SCHEMA_VERSION: u32 = 1;
pub const TERRAIN_GEOMETRY_CHUNK_SIZE: usize = 16;
pub const TERRAIN_GEOMETRY_VOXEL_COUNT: usize =
    TERRAIN_GEOMETRY_CHUNK_SIZE * TERRAIN_GEOMETRY_CHUNK_SIZE * TERRAIN_GEOMETRY_CHUNK_SIZE;

const SNAPSHOT_DOMAIN: &[u8] = b"symtropy.terrain.geometry.snapshot.v1\0";
const VOXEL_DOMAIN: &[u8] = b"symtropy.terrain.geometry.voxel.v1\0";
const QUERY_DOMAIN: &[u8] = b"symtropy.terrain.geometry.query.v1\0";

/// Stable v1 code used only at the exact geometry authority boundary.
///
/// These codes deliberately preserve the existing Terrain v0.1 material tags.
/// Adding a new Terrain material requires an explicit schema/adapter decision;
/// callers cannot construct arbitrary unknown codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainMaterialCode(u8);

impl TerrainMaterialCode {
    pub const AIR: Self = Self(0);
    pub const BEDROCK: Self = Self(1);
    pub const DOLOMITE: Self = Self(2);
    pub const PYRITE_TAILING: Self = Self(3);
    pub const QUARTZITE: Self = Self(4);

    pub const fn as_u8(self) -> u8 {
        self.0
    }
}

/// Exact Terrain-lattice locus. This is not yet a world/reference-frame claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        let value = Self { x, y, z };
        value.validate()?;
        Ok(value)
    }

    fn validate(self) -> Result<(), TerrainGeometryError> {
        let limit = TERRAIN_GEOMETRY_CHUNK_SIZE as u8;
        if self.x >= limit || self.y >= limit || self.z >= limit {
            return Err(TerrainGeometryError::VoxelOutsideChunk(self));
        }
        Ok(())
    }
}

/// Half-open `[min, max_exclusive)` exact query within one chunk.
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
        let limit = TERRAIN_GEOMETRY_CHUNK_SIZE as u8;
        if max_exclusive
            .into_iter()
            .any(|value| value == 0 || value > limit)
            || min.x >= max_exclusive[0]
            || min.y >= max_exclusive[1]
            || min.z >= max_exclusive[2]
        {
            return Err(TerrainGeometryError::InvalidQueryBounds { min, max_exclusive });
        }
        Ok(Self { min, max_exclusive })
    }

    pub const fn full_chunk() -> Self {
        Self {
            min: TerrainVoxelIndex { x: 0, y: 0, z: 0 },
            max_exclusive: [TERRAIN_GEOMETRY_CHUNK_SIZE as u8; 3],
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
    material: TerrainMaterialCode,
    digest: TerrainGeometryDigest,
}

impl TerrainVoxelObservation {
    pub const fn chunk(&self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn voxel(&self) -> TerrainVoxelIndex {
        self.voxel
    }

    pub const fn material(&self) -> TerrainMaterialCode {
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
    materials: Box<[TerrainMaterialCode; TERRAIN_GEOMETRY_VOXEL_COUNT]>,
    digest: TerrainGeometryDigest,
}

impl TerrainGeometrySnapshot {
    /// Seal one complete canonical 16^3 material-code observation.
    ///
    /// World currentness is intentionally outside this pure kernel. The real
    /// Terrain adapter must obtain these complete codes from the owning state at
    /// the point where current geometry is required.
    pub fn from_material_codes(
        chunk: EarthChunkLatticeCoord,
        materials: Box<[TerrainMaterialCode; TERRAIN_GEOMETRY_VOXEL_COUNT]>,
    ) -> Self {
        let digest = snapshot_digest(chunk, &materials);
        Self {
            schema_version: TERRAIN_GEOMETRY_SCHEMA_VERSION,
            chunk,
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
                    let observation = self.observations.get(cursor).ok_or(
                        TerrainGeometryError::IncompleteQuery {
                            expected,
                            actual: cursor,
                        },
                    )?;
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
    (usize::from(voxel.x) * TERRAIN_GEOMETRY_CHUNK_SIZE * TERRAIN_GEOMETRY_CHUNK_SIZE)
        + (usize::from(voxel.y) * TERRAIN_GEOMETRY_CHUNK_SIZE)
        + usize::from(voxel.z)
}

fn push_coord(bytes: &mut Vec<u8>, coord: EarthChunkLatticeCoord) {
    bytes.extend_from_slice(&coord.x.to_le_bytes());
    bytes.extend_from_slice(&coord.y.to_le_bytes());
    bytes.extend_from_slice(&coord.z.to_le_bytes());
}

fn snapshot_digest(
    coord: EarthChunkLatticeCoord,
    materials: &[TerrainMaterialCode; TERRAIN_GEOMETRY_VOXEL_COUNT],
) -> TerrainGeometryDigest {
    let mut bytes =
        Vec::with_capacity(SNAPSHOT_DOMAIN.len() + 4 + 12 + 4 + TERRAIN_GEOMETRY_VOXEL_COUNT);
    bytes.extend_from_slice(SNAPSHOT_DOMAIN);
    bytes.extend_from_slice(&TERRAIN_GEOMETRY_SCHEMA_VERSION.to_le_bytes());
    push_coord(&mut bytes, coord);
    bytes.extend_from_slice(&(TERRAIN_GEOMETRY_CHUNK_SIZE as u32).to_le_bytes());
    bytes.extend(materials.iter().map(|material| material.as_u8()));
    TerrainGeometryDigest(sha256(&bytes))
}

fn voxel_digest(
    coord: EarthChunkLatticeCoord,
    voxel: TerrainVoxelIndex,
    material: TerrainMaterialCode,
) -> TerrainGeometryDigest {
    let mut bytes = Vec::with_capacity(VOXEL_DOMAIN.len() + 4 + 12 + 4);
    bytes.extend_from_slice(VOXEL_DOMAIN);
    bytes.extend_from_slice(&TERRAIN_GEOMETRY_SCHEMA_VERSION.to_le_bytes());
    push_coord(&mut bytes, coord);
    bytes.extend_from_slice(&[voxel.x, voxel.y, voxel.z, material.as_u8()]);
    TerrainGeometryDigest(sha256(&bytes))
}

fn query_digest(
    snapshot_digest: TerrainGeometryDigest,
    bounds: TerrainVoxelBox,
    observations: &[TerrainVoxelObservation],
) -> TerrainGeometryDigest {
    let mut bytes =
        Vec::with_capacity(QUERY_DOMAIN.len() + 4 + 32 + 6 + 4 + observations.len() * 32);
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
/// Checked against FIPS 180-4 and independent Terrain byte-grammar vectors.
fn sha256(input: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
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

    fn default_materials() -> Box<[TerrainMaterialCode; TERRAIN_GEOMETRY_VOXEL_COUNT]> {
        Box::new([TerrainMaterialCode::DOLOMITE; TERRAIN_GEOMETRY_VOXEL_COUNT])
    }

    fn index(x: usize, y: usize, z: usize) -> usize {
        (x * TERRAIN_GEOMETRY_CHUNK_SIZE * TERRAIN_GEOMETRY_CHUNK_SIZE)
            + (y * TERRAIN_GEOMETRY_CHUNK_SIZE)
            + z
    }

    #[test]
    fn sha256_matches_fips_abc_vector() {
        assert_eq!(
            TerrainGeometryDigest(sha256(b"abc")).to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

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

    #[test]
    fn one_voxel_change_is_local_and_matches_external_air_vector() {
        let coord = EarthChunkLatticeCoord::new(1, 2, 3);
        let before = TerrainGeometrySnapshot::from_material_codes(coord, default_materials());
        let changed = TerrainVoxelIndex::new(2, 3, 4).unwrap();
        let stable = TerrainVoxelIndex::new(8, 8, 8).unwrap();
        let stable_before = before.observe(stable).unwrap().digest();

        let mut materials = default_materials();
        materials[index(2, 3, 4)] = TerrainMaterialCode::AIR;
        let after = TerrainGeometrySnapshot::from_material_codes(coord, materials);

        assert_ne!(before.digest(), after.digest());
        assert_eq!(stable_before, after.observe(stable).unwrap().digest());
        assert_eq!(
            after.observe(changed).unwrap().digest().to_hex(),
            "fb4cb313a0697b5cdba06f4102ed6dd6f124559c08e35a066834561ee6650841"
        );
    }

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
        let bounds =
            TerrainVoxelBox::new(TerrainVoxelIndex::new(2, 3, 4).unwrap(), [3, 4, 5]).unwrap();
        let receipt = snapshot.query(bounds).unwrap();
        assert_eq!(receipt.observations().len(), 1);
        assert_eq!(
            receipt.observations()[0].digest().to_hex(),
            "d6c2b78646786404b73ef6aabc4cac6545ba4341f14e509f9745dd187f2eda14"
        );
        assert_eq!(
            receipt.digest().to_hex(),
            "7a505c56667d34eaafa1da76980e066e02e0f4187409f30eedd54f4c5168a117"
        );
        receipt.validate().unwrap();
    }

    #[test]
    fn full_query_proves_explicit_air_and_complete_order() {
        let mut materials = default_materials();
        materials[0] = TerrainMaterialCode::AIR;
        let snapshot = TerrainGeometrySnapshot::from_material_codes(
            EarthChunkLatticeCoord::new(-1, 7, 2),
            materials,
        );
        let receipt = snapshot.query(TerrainVoxelBox::full_chunk()).unwrap();
        assert_eq!(receipt.observations().len(), TERRAIN_GEOMETRY_VOXEL_COUNT);
        assert_eq!(
            receipt.observations()[0].voxel(),
            TerrainVoxelIndex::new(0, 0, 0).unwrap()
        );
        assert_eq!(
            receipt.observations()[0].material(),
            TerrainMaterialCode::AIR
        );
        assert_eq!(
            receipt.observations().last().unwrap().voxel(),
            TerrainVoxelIndex::new(15, 15, 15).unwrap()
        );
        receipt.validate().unwrap();
    }

    #[test]
    fn bounded_query_and_invalid_bounds_are_fail_closed() {
        let snapshot = TerrainGeometrySnapshot::from_material_codes(
            EarthChunkLatticeCoord::new(0, 0, 0),
            default_materials(),
        );
        let bounds =
            TerrainVoxelBox::new(TerrainVoxelIndex::new(2, 4, 6).unwrap(), [5, 6, 9]).unwrap();
        let receipt = snapshot.query(bounds).unwrap();
        assert_eq!(receipt.observations().len(), 18);
        assert_eq!(
            receipt.observations().first().unwrap().voxel(),
            TerrainVoxelIndex::new(2, 4, 6).unwrap()
        );
        assert_eq!(
            receipt.observations().last().unwrap().voxel(),
            TerrainVoxelIndex::new(4, 5, 8).unwrap()
        );
        receipt.validate().unwrap();

        let min = TerrainVoxelIndex::new(4, 4, 4).unwrap();
        assert!(TerrainVoxelBox::new(min, [4, 5, 5]).is_err());
        assert!(TerrainVoxelBox::new(min, [17, 5, 5]).is_err());
        assert!(TerrainVoxelIndex::new(16, 0, 0).is_err());
    }

    #[test]
    fn coordinate_changes_snapshot_identity_but_repeat_is_deterministic() {
        let a = TerrainGeometrySnapshot::from_material_codes(
            EarthChunkLatticeCoord::new(0, 0, 0),
            default_materials(),
        );
        let repeat = TerrainGeometrySnapshot::from_material_codes(
            EarthChunkLatticeCoord::new(0, 0, 0),
            default_materials(),
        );
        let b = TerrainGeometrySnapshot::from_material_codes(
            EarthChunkLatticeCoord::new(1, 0, 0),
            default_materials(),
        );
        assert_eq!(a, repeat);
        assert_ne!(a.digest(), b.digest());
    }
}
