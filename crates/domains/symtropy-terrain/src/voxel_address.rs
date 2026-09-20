// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Exact, metric-free address mapping for Terrain's discrete voxel lattice.
//!
//! This module owns only the bijection between one qualified #984 chunk-local
//! voxel address and one global integer Terrain-lattice voxel coordinate. It
//! does not authorize a chunk to exist, bind a Terrain lineage/subject, select
//! physical metric, or place Terrain in an external/world reference frame.

use core::fmt;

use crate::geometry_kernel::{
    EarthChunkLatticeCoord, TERRAIN_GEOMETRY_CHUNK_SIZE, TerrainVoxelIndex,
};

/// Canonical schema for the first exact Terrain voxel-address profile.
pub const TERRAIN_VOXEL_ADDRESS_SCHEMA_VERSION: u32 = 1;

const TERRAIN_VOXEL_ADDRESS_DOMAIN: &[u8] = b"symtropy.terrain.voxel-address.v1\0";
const CHUNK_EDGE: i64 = TERRAIN_GEOMETRY_CHUNK_SIZE as i64;

/// Smallest global voxel coordinate representable by the current i32 chunk domain.
pub const MIN_GLOBAL_TERRAIN_VOXEL_AXIS: i64 = i32::MIN as i64 * 16;
/// Largest global voxel coordinate representable by the current i32 chunk domain.
pub const MAX_GLOBAL_TERRAIN_VOXEL_AXIS: i64 = i32::MAX as i64 * 16 + 15;

/// Exact width of the v1 canonical global-address encoding.
pub const TERRAIN_VOXEL_ADDRESS_CANONICAL_BYTES_LEN: usize =
    TERRAIN_VOXEL_ADDRESS_DOMAIN.len() + 4 + 3 * 8;

/// Axis label used only for fail-closed conversion diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainVoxelAxis {
    X,
    Y,
    Z,
}

/// Exact global Terrain-lattice voxel coordinate.
///
/// This is an integer lattice address only. It is not a physical/world-space
/// point and it is not a durable Terrain subject identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalTerrainVoxelCoord {
    x: i64,
    y: i64,
    z: i64,
}

impl GlobalTerrainVoxelCoord {
    /// Constructs a supported v1 global lattice address.
    ///
    /// Values whose Euclidean chunk quotient would lie outside the current
    /// i32 chunk-locus domain fail closed instead of widening that authority.
    pub fn new(x: i64, y: i64, z: i64) -> Result<Self, TerrainVoxelAddressError> {
        validate_global_axis(TerrainVoxelAxis::X, x)?;
        validate_global_axis(TerrainVoxelAxis::Y, y)?;
        validate_global_axis(TerrainVoxelAxis::Z, z)?;
        Ok(Self { x, y, z })
    }

    pub const fn x(self) -> i64 {
        self.x
    }

    pub const fn y(self) -> i64 {
        self.y
    }

    pub const fn z(self) -> i64 {
        self.z
    }

    /// Converts this validated global address to its unique chunk/local pair.
    ///
    /// Euclidean division is essential for negative coordinates. Truncation
    /// toward zero would map e.g. global -1 to the wrong chunk.
    pub fn to_address(self) -> TerrainVoxelAddressV1 {
        let chunk = EarthChunkLatticeCoord::new(
            self.x.div_euclid(CHUNK_EDGE) as i32,
            self.y.div_euclid(CHUNK_EDGE) as i32,
            self.z.div_euclid(CHUNK_EDGE) as i32,
        );
        let local = TerrainVoxelIndex {
            x: self.x.rem_euclid(CHUNK_EDGE) as u8,
            y: self.y.rem_euclid(CHUNK_EDGE) as u8,
            z: self.z.rem_euclid(CHUNK_EDGE) as u8,
        };
        TerrainVoxelAddressV1 { chunk, local }
    }

    /// Serializer-independent v1 identity bytes for this numeric lattice address.
    ///
    /// Grammar:
    /// `domain || u32le(schema_version) || i64le(x) || i64le(y) || i64le(z)`.
    ///
    /// These bytes identify only the numeric lattice address/profile. Higher
    /// layers must bind Terrain lineage/durable subject separately.
    pub fn canonical_bytes(self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(TERRAIN_VOXEL_ADDRESS_CANONICAL_BYTES_LEN);
        bytes.extend_from_slice(TERRAIN_VOXEL_ADDRESS_DOMAIN);
        bytes.extend_from_slice(&TERRAIN_VOXEL_ADDRESS_SCHEMA_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.x.to_le_bytes());
        bytes.extend_from_slice(&self.y.to_le_bytes());
        bytes.extend_from_slice(&self.z.to_le_bytes());
        debug_assert_eq!(bytes.len(), TERRAIN_VOXEL_ADDRESS_CANONICAL_BYTES_LEN);
        bytes
    }
}

/// Canonical pair of one exact chunk locus plus one local voxel index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainVoxelAddressV1 {
    chunk: EarthChunkLatticeCoord,
    local: TerrainVoxelIndex,
}

impl TerrainVoxelAddressV1 {
    /// Constructs a validated pair even if a caller bypassed
    /// `TerrainVoxelIndex::new` with a public struct literal.
    pub fn new(
        chunk: EarthChunkLatticeCoord,
        local: TerrainVoxelIndex,
    ) -> Result<Self, TerrainVoxelAddressError> {
        let limit = TERRAIN_GEOMETRY_CHUNK_SIZE as u8;
        if local.x >= limit || local.y >= limit || local.z >= limit {
            return Err(TerrainVoxelAddressError::LocalVoxelOutsideChunk(local));
        }
        Ok(Self { chunk, local })
    }

    pub const fn chunk(self) -> EarthChunkLatticeCoord {
        self.chunk
    }

    pub const fn local(self) -> TerrainVoxelIndex {
        self.local
    }

    /// Converts this pair to its unique exact global integer lattice address.
    pub fn global(self) -> Result<GlobalTerrainVoxelCoord, TerrainVoxelAddressError> {
        let x = forward_axis(TerrainVoxelAxis::X, self.chunk.x, self.local.x)?;
        let y = forward_axis(TerrainVoxelAxis::Y, self.chunk.y, self.local.y)?;
        let z = forward_axis(TerrainVoxelAxis::Z, self.chunk.z, self.local.z)?;
        GlobalTerrainVoxelCoord::new(x, y, z)
    }
}

/// Exact half-open global voxel box occupied by one chunk locus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainChunkGlobalVoxelBoxV1 {
    min: [i64; 3],
    max_exclusive: [i64; 3],
}

impl TerrainChunkGlobalVoxelBoxV1 {
    pub fn from_chunk(
        chunk: EarthChunkLatticeCoord,
    ) -> Result<Self, TerrainVoxelAddressError> {
        let min_x = chunk_axis_min(TerrainVoxelAxis::X, chunk.x)?;
        let min_y = chunk_axis_min(TerrainVoxelAxis::Y, chunk.y)?;
        let min_z = chunk_axis_min(TerrainVoxelAxis::Z, chunk.z)?;
        let max_x = min_x.checked_add(CHUNK_EDGE).ok_or(
            TerrainVoxelAddressError::ArithmeticOverflow(TerrainVoxelAxis::X),
        )?;
        let max_y = min_y.checked_add(CHUNK_EDGE).ok_or(
            TerrainVoxelAddressError::ArithmeticOverflow(TerrainVoxelAxis::Y),
        )?;
        let max_z = min_z.checked_add(CHUNK_EDGE).ok_or(
            TerrainVoxelAddressError::ArithmeticOverflow(TerrainVoxelAxis::Z),
        )?;
        Ok(Self {
            min: [min_x, min_y, min_z],
            max_exclusive: [max_x, max_y, max_z],
        })
    }

    pub const fn min(self) -> [i64; 3] {
        self.min
    }

    pub const fn max_exclusive(self) -> [i64; 3] {
        self.max_exclusive
    }

    pub const fn contains(self, coord: GlobalTerrainVoxelCoord) -> bool {
        coord.x >= self.min[0]
            && coord.x < self.max_exclusive[0]
            && coord.y >= self.min[1]
            && coord.y < self.max_exclusive[1]
            && coord.z >= self.min[2]
            && coord.z < self.max_exclusive[2]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainVoxelAddressError {
    LocalVoxelOutsideChunk(TerrainVoxelIndex),
    GlobalAxisOutsideSupportedDomain {
        axis: TerrainVoxelAxis,
        value: i64,
        minimum: i64,
        maximum: i64,
    },
    ArithmeticOverflow(TerrainVoxelAxis),
}

impl fmt::Display for TerrainVoxelAddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalVoxelOutsideChunk(voxel) => write!(
                f,
                "Terrain local voxel {voxel:?} is outside the {}^3 chunk",
                TERRAIN_GEOMETRY_CHUNK_SIZE
            ),
            Self::GlobalAxisOutsideSupportedDomain {
                axis,
                value,
                minimum,
                maximum,
            } => write!(
                f,
                "Terrain global voxel axis {axis:?} value {value} is outside supported v1 range {minimum}..={maximum}"
            ),
            Self::ArithmeticOverflow(axis) => {
                write!(f, "Terrain voxel-address arithmetic overflow on {axis:?} axis")
            }
        }
    }
}

impl std::error::Error for TerrainVoxelAddressError {}

fn validate_global_axis(
    axis: TerrainVoxelAxis,
    value: i64,
) -> Result<(), TerrainVoxelAddressError> {
    if !(MIN_GLOBAL_TERRAIN_VOXEL_AXIS..=MAX_GLOBAL_TERRAIN_VOXEL_AXIS).contains(&value) {
        return Err(TerrainVoxelAddressError::GlobalAxisOutsideSupportedDomain {
            axis,
            value,
            minimum: MIN_GLOBAL_TERRAIN_VOXEL_AXIS,
            maximum: MAX_GLOBAL_TERRAIN_VOXEL_AXIS,
        });
    }
    Ok(())
}

fn chunk_axis_min(axis: TerrainVoxelAxis, chunk: i32) -> Result<i64, TerrainVoxelAddressError> {
    i64::from(chunk)
        .checked_mul(CHUNK_EDGE)
        .ok_or(TerrainVoxelAddressError::ArithmeticOverflow(axis))
}

fn forward_axis(
    axis: TerrainVoxelAxis,
    chunk: i32,
    local: u8,
) -> Result<i64, TerrainVoxelAddressError> {
    let min = chunk_axis_min(axis, chunk)?;
    min.checked_add(i64::from(local))
        .ok_or(TerrainVoxelAddressError::ArithmeticOverflow(axis))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(x: u8, y: u8, z: u8) -> TerrainVoxelIndex {
        TerrainVoxelIndex::new(x, y, z).expect("test local coordinate must be bounded")
    }

    fn address(chunk_x: i32, local_x: u8) -> TerrainVoxelAddressV1 {
        TerrainVoxelAddressV1::new(
            EarthChunkLatticeCoord::new(chunk_x, 0, 0),
            local(local_x, 0, 0),
        )
        .expect("test address must be valid")
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
    fn current_profile_is_exactly_sixteen_voxels_per_chunk_axis() {
        assert_eq!(TERRAIN_GEOMETRY_CHUNK_SIZE, 16);
        assert_eq!(CHUNK_EDGE, 16);
        assert_eq!(MIN_GLOBAL_TERRAIN_VOXEL_AXIS, -34_359_738_368);
        assert_eq!(MAX_GLOBAL_TERRAIN_VOXEL_AXIS, 34_359_738_367);
    }

    #[test]
    fn origin_and_positive_boundary_round_trip() {
        let cases = [(0, 0, 0), (0, 15, 15), (1, 0, 16), (1, 15, 31)];
        for (chunk_x, local_x, global_x) in cases {
            let pair = address(chunk_x, local_x);
            let global = pair.global().expect("forward mapping must succeed");
            assert_eq!(global.x(), global_x);
            assert_eq!(global.to_address(), pair);
        }
    }

    #[test]
    fn negative_boundaries_use_euclidean_division() {
        let cases = [(-1, 15, -1), (-1, 0, -16), (-2, 15, -17)];
        for (chunk_x, local_x, global_x) in cases {
            let pair = address(chunk_x, local_x);
            let global = pair.global().expect("forward mapping must succeed");
            assert_eq!(global.x(), global_x);
            assert_eq!(
                GlobalTerrainVoxelCoord::new(global_x, 0, 0)
                    .unwrap()
                    .to_address(),
                pair
            );
        }

        // Truncation toward zero is the known wrong inverse for global -1.
        assert_eq!(-1_i64 / 16, 0);
        assert_eq!(-1_i64.div_euclid(16), -1);
    }

    #[test]
    fn mixed_sign_three_axis_address_round_trips() {
        let pair = TerrainVoxelAddressV1::new(
            EarthChunkLatticeCoord::new(-7, 11, -13),
            local(15, 0, 8),
        )
        .unwrap();
        let global = pair.global().unwrap();
        assert_eq!((global.x(), global.y(), global.z()), (-97, 176, -200));
        assert_eq!(global.to_address(), pair);
    }

    #[test]
    fn supported_i32_chunk_extremes_round_trip() {
        for (chunk, local_x, expected) in [
            (i32::MIN, 0, MIN_GLOBAL_TERRAIN_VOXEL_AXIS),
            (i32::MIN, 15, MIN_GLOBAL_TERRAIN_VOXEL_AXIS + 15),
            (i32::MAX, 0, MAX_GLOBAL_TERRAIN_VOXEL_AXIS - 15),
            (i32::MAX, 15, MAX_GLOBAL_TERRAIN_VOXEL_AXIS),
        ] {
            let pair = address(chunk, local_x);
            let global = pair.global().unwrap();
            assert_eq!(global.x(), expected);
            assert_eq!(global.to_address(), pair);
        }
    }

    #[test]
    fn global_values_outside_supported_chunk_domain_fail_closed() {
        assert!(matches!(
            GlobalTerrainVoxelCoord::new(MIN_GLOBAL_TERRAIN_VOXEL_AXIS - 1, 0, 0),
            Err(TerrainVoxelAddressError::GlobalAxisOutsideSupportedDomain {
                axis: TerrainVoxelAxis::X,
                ..
            })
        ));
        assert!(matches!(
            GlobalTerrainVoxelCoord::new(MAX_GLOBAL_TERRAIN_VOXEL_AXIS + 1, 0, 0),
            Err(TerrainVoxelAddressError::GlobalAxisOutsideSupportedDomain {
                axis: TerrainVoxelAxis::X,
                ..
            })
        ));
    }

    #[test]
    fn direct_invalid_local_struct_literal_still_fails_closed() {
        let invalid = TerrainVoxelIndex { x: 16, y: 0, z: 0 };
        assert_eq!(
            TerrainVoxelAddressV1::new(EarthChunkLatticeCoord::new(0, 0, 0), invalid),
            Err(TerrainVoxelAddressError::LocalVoxelOutsideChunk(invalid))
        );
    }

    #[test]
    fn adjacent_chunk_boundaries_are_consecutive_on_positive_and_negative_axes() {
        assert_eq!(address(0, 15).global().unwrap().x(), 15);
        assert_eq!(address(1, 0).global().unwrap().x(), 16);
        assert_eq!(address(-1, 15).global().unwrap().x(), -1);
        assert_eq!(address(0, 0).global().unwrap().x(), 0);
    }

    #[test]
    fn chunk_boxes_are_half_open_touching_and_non_overlapping() {
        let negative =
            TerrainChunkGlobalVoxelBoxV1::from_chunk(EarthChunkLatticeCoord::new(-1, 0, 0))
                .unwrap();
        let zero = TerrainChunkGlobalVoxelBoxV1::from_chunk(EarthChunkLatticeCoord::new(0, 0, 0))
            .unwrap();

        assert_eq!(negative.min()[0], -16);
        assert_eq!(negative.max_exclusive()[0], 0);
        assert_eq!(zero.min()[0], 0);
        assert_eq!(zero.max_exclusive()[0], 16);
        assert_eq!(negative.max_exclusive()[0], zero.min()[0]);

        assert!(negative.contains(GlobalTerrainVoxelCoord::new(-1, 0, 0).unwrap()));
        assert!(!negative.contains(GlobalTerrainVoxelCoord::new(0, 0, 0).unwrap()));
        assert!(zero.contains(GlobalTerrainVoxelCoord::new(0, 0, 0).unwrap()));
        assert!(zero.contains(GlobalTerrainVoxelCoord::new(15, 15, 15).unwrap()));
    }

    #[test]
    fn every_selected_local_voxel_lies_in_exactly_its_chunk_box() {
        let chunks = [i32::MIN, -2, -1, 0, 1, 2, i32::MAX];
        let locals = [0_u8, 1, 7, 15];
        for chunk_x in chunks {
            let box_ =
                TerrainChunkGlobalVoxelBoxV1::from_chunk(EarthChunkLatticeCoord::new(chunk_x, 0, 0))
                    .unwrap();
            for local_x in locals {
                let global = address(chunk_x, local_x).global().unwrap();
                assert!(box_.contains(global));
            }
        }
    }

    #[test]
    fn canonical_bytes_have_frozen_v1_grammar() {
        let coord = GlobalTerrainVoxelCoord::new(-1, 16, -17).unwrap();
        let bytes = coord.canonical_bytes();
        assert_eq!(bytes.len(), TERRAIN_VOXEL_ADDRESS_CANONICAL_BYTES_LEN);
        assert_eq!(
            to_hex(&bytes),
            "73796d74726f70792e7465727261696e2e766f78656c2d616464726573732e76310001000000ffffffffffffffff1000000000000000efffffffffffffff"
        );
    }

    #[test]
    fn different_supported_pairs_do_not_alias_in_boundary_fixture() {
        let pairs = [
            address(-2, 15),
            address(-1, 0),
            address(-1, 15),
            address(0, 0),
            address(0, 15),
            address(1, 0),
        ];
        let mut globals = Vec::new();
        for pair in pairs {
            let global = pair.global().unwrap();
            assert!(!globals.contains(&global));
            globals.push(global);
        }
    }
}
