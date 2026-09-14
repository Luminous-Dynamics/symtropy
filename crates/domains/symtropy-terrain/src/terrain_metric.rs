//! Exact physical metric for the discrete Terrain voxel lattice.
//!
//! This module deliberately owns only the physical edge length of one discrete
//! Terrain voxel. It does not infer scale from Bevy transforms, meshes, Rapier
//! colliders, excavation coordinates, or PB decomposition settings.

use core::fmt;

/// Canonical schema for the first exact Terrain lattice metric profile.
pub const TERRAIN_METRIC_SCHEMA_VERSION: u32 = 1;

/// Operational v1 envelope for one voxel edge, in micrometres.
///
/// This is a representation/validation bound, not a claim about the intended
/// physical size of Terrain voxels. Values outside this envelope require a
/// separately versioned successor instead of implicit widening.
pub const MAX_TERRAIN_VOXEL_EDGE_UM: i64 = 1_000_000_000_000;

/// Canonical byte width of a v1 metric value: u32 schema + i64 edge length.
pub const TERRAIN_METRIC_CANONICAL_BYTES_LEN: usize = 12;

/// Lossless exact physical metric for one step of the discrete Terrain lattice.
///
/// The value itself is the v1 identity. No serializer, floating-point value,
/// runtime handle, transform, renderer output, or external decomposition profile
/// participates in equality or canonical bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TerrainMetricProfile {
    schema_version: u32,
    voxel_edge_um: i64,
}

impl TerrainMetricProfile {
    /// Constructs the current v1 metric profile from an exact micrometre edge.
    pub const fn new(voxel_edge_um: i64) -> Result<Self, TerrainMetricError> {
        if voxel_edge_um <= 0 {
            return Err(TerrainMetricError::NonPositiveVoxelEdge(voxel_edge_um));
        }
        if voxel_edge_um > MAX_TERRAIN_VOXEL_EDGE_UM {
            return Err(TerrainMetricError::VoxelEdgeTooLarge {
                actual_um: voxel_edge_um,
                maximum_um: MAX_TERRAIN_VOXEL_EDGE_UM,
            });
        }

        Ok(Self {
            schema_version: TERRAIN_METRIC_SCHEMA_VERSION,
            voxel_edge_um,
        })
    }

    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    pub const fn voxel_edge_um(self) -> i64 {
        self.voxel_edge_um
    }

    /// Returns serializer-independent v1 canonical bytes.
    ///
    /// Grammar: `schema_version.to_le_bytes() || voxel_edge_um.to_le_bytes()`.
    pub const fn canonical_bytes(self) -> [u8; TERRAIN_METRIC_CANONICAL_BYTES_LEN] {
        let schema = self.schema_version.to_le_bytes();
        let edge = self.voxel_edge_um.to_le_bytes();
        [
            schema[0], schema[1], schema[2], schema[3], edge[0], edge[1], edge[2], edge[3],
            edge[4], edge[5], edge[6], edge[7],
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainMetricError {
    NonPositiveVoxelEdge(i64),
    VoxelEdgeTooLarge { actual_um: i64, maximum_um: i64 },
}

impl fmt::Display for TerrainMetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveVoxelEdge(actual_um) => write!(
                f,
                "Terrain voxel edge must be positive micrometres, got {actual_um}"
            ),
            Self::VoxelEdgeTooLarge {
                actual_um,
                maximum_um,
            } => write!(
                f,
                "Terrain voxel edge {actual_um} um exceeds v1 maximum {maximum_um} um"
            ),
        }
    }
}

impl std::error::Error for TerrainMetricError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smallest_positive_metric_is_valid() {
        let profile = TerrainMetricProfile::new(1).expect("1 um must be valid");
        assert_eq!(profile.schema_version(), TERRAIN_METRIC_SCHEMA_VERSION);
        assert_eq!(profile.voxel_edge_um(), 1);
    }

    #[test]
    fn zero_and_negative_edges_fail_closed() {
        assert_eq!(
            TerrainMetricProfile::new(0),
            Err(TerrainMetricError::NonPositiveVoxelEdge(0))
        );
        assert_eq!(
            TerrainMetricProfile::new(-1),
            Err(TerrainMetricError::NonPositiveVoxelEdge(-1))
        );
    }

    #[test]
    fn exact_operational_maximum_is_inclusive() {
        let profile = TerrainMetricProfile::new(MAX_TERRAIN_VOXEL_EDGE_UM)
            .expect("exact v1 maximum must be valid");
        assert_eq!(profile.voxel_edge_um(), MAX_TERRAIN_VOXEL_EDGE_UM);
    }

    #[test]
    fn over_bound_edge_fails_closed() {
        let actual_um = MAX_TERRAIN_VOXEL_EDGE_UM + 1;
        assert_eq!(
            TerrainMetricProfile::new(actual_um),
            Err(TerrainMetricError::VoxelEdgeTooLarge {
                actual_um,
                maximum_um: MAX_TERRAIN_VOXEL_EDGE_UM,
            })
        );
    }

    #[test]
    fn changed_edge_is_a_different_exact_metric_subject() {
        let one = TerrainMetricProfile::new(1).unwrap();
        let two = TerrainMetricProfile::new(2).unwrap();
        assert_ne!(one, two);
        assert_ne!(one.canonical_bytes(), two.canonical_bytes());
    }

    #[test]
    fn canonical_bytes_have_frozen_little_endian_grammar() {
        let one_million_um = TerrainMetricProfile::new(1_000_000).unwrap();
        assert_eq!(
            one_million_um.canonical_bytes(),
            [1, 0, 0, 0, 0x40, 0x42, 0x0f, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            one_million_um.canonical_bytes().len(),
            TERRAIN_METRIC_CANONICAL_BYTES_LEN
        );
    }

    #[test]
    fn canonical_bytes_round_trip_the_complete_identity_tuple() {
        let profile = TerrainMetricProfile::new(12_345_678).unwrap();
        let bytes = profile.canonical_bytes();

        let schema_version = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let voxel_edge_um = i64::from_le_bytes(bytes[4..12].try_into().unwrap());

        assert_eq!(schema_version, profile.schema_version());
        assert_eq!(voxel_edge_um, profile.voxel_edge_um());
    }
}
