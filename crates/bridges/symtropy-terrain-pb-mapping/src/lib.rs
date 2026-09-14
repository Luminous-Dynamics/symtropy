// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! PB-04d1 pure exact Terrain voxel-box to PB cell-box mapping theorem.
//!
//! This crate deliberately owns no production metric selection, spatial-placement
//! selection, occupancy interpretation, or complete-domain coverage authority.
//! It only proves whether one exact Terrain voxel box, under caller-supplied pure
//! metric and embedding values, is exactly equal to one and only one PB cell box.
//! Production authorization of those values belongs to later authority layers.

use std::{error::Error, fmt};

use symtropy_spatial_decomposition::{
    AnalysisDomain, CellCoord, DecompositionError, DecompositionProfile, Point3i,
};
use symtropy_terrain::{
    geometry_kernel::{
        EarthChunkLatticeCoord, TerrainVoxelIndex, TERRAIN_GEOMETRY_CHUNK_SIZE,
    },
    terrain_metric::TerrainMetricProfile,
};

pub const TERRAIN_PB_MAPPING_SCHEMA_VERSION: u32 = 1;
pub const TERRAIN_PB_EMBEDDING_CANONICAL_BYTES_LEN: usize = 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    const fn index(self) -> usize {
        match self {
            Self::X => 0,
            Self::Y => 1,
            Self::Z => 2,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AxisSign {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignedAxis {
    source: Axis,
    sign: AxisSign,
}

impl SignedAxis {
    pub const fn positive(source: Axis) -> Self {
        Self {
            source,
            sign: AxisSign::Positive,
        }
    }

    pub const fn negative(source: Axis) -> Self {
        Self {
            source,
            sign: AxisSign::Negative,
        }
    }

    pub const fn source(self) -> Axis {
        self.source
    }

    pub const fn sign(self) -> AxisSign {
        self.sign
    }

    const fn canonical_code(self) -> u8 {
        match (self.source, self.sign) {
            (Axis::X, AxisSign::Positive) => 0,
            (Axis::X, AxisSign::Negative) => 1,
            (Axis::Y, AxisSign::Positive) => 2,
            (Axis::Y, AxisSign::Negative) => 3,
            (Axis::Z, AxisSign::Positive) => 4,
            (Axis::Z, AxisSign::Negative) => 5,
        }
    }
}

/// Pure exact embedding value. Validity is not production authorization.
///
/// `pb_axes[destination_axis]` selects one Terrain source axis and sign for that
/// PB destination axis. Source axes must form a permutation; shear, scale, and
/// floating-point transforms are intentionally unrepresentable in v1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPbEmbeddingProfile {
    schema_version: u32,
    pb_axes: [SignedAxis; 3],
    translation_um: Point3i,
}

impl TerrainPbEmbeddingProfile {
    pub fn new(
        pb_axes: [SignedAxis; 3],
        translation_um: Point3i,
    ) -> Result<Self, TerrainPbMappingError> {
        let mut seen = [false; 3];
        for axis in pb_axes {
            let index = axis.source.index();
            if seen[index] {
                return Err(TerrainPbMappingError::DuplicateSourceAxis(axis.source));
            }
            seen[index] = true;
        }
        Ok(Self {
            schema_version: TERRAIN_PB_MAPPING_SCHEMA_VERSION,
            pb_axes,
            translation_um,
        })
    }

    pub fn identity(translation_um: Point3i) -> Self {
        Self {
            schema_version: TERRAIN_PB_MAPPING_SCHEMA_VERSION,
            pb_axes: [
                SignedAxis::positive(Axis::X),
                SignedAxis::positive(Axis::Y),
                SignedAxis::positive(Axis::Z),
            ],
            translation_um,
        }
    }

    pub const fn schema_version(self) -> u32 {
        self.schema_version
    }

    pub const fn pb_axes(self) -> [SignedAxis; 3] {
        self.pb_axes
    }

    pub const fn translation_um(self) -> Point3i {
        self.translation_um
    }

    /// Language-neutral v1 identity bytes:
    /// `schema_le || axis_code[3] || translation_xyz_i64_le`.
    pub fn canonical_bytes(self) -> [u8; TERRAIN_PB_EMBEDDING_CANONICAL_BYTES_LEN] {
        let schema = self.schema_version.to_le_bytes();
        let x = self.translation_um.x.to_le_bytes();
        let y = self.translation_um.y.to_le_bytes();
        let z = self.translation_um.z.to_le_bytes();
        let mut bytes = [0_u8; TERRAIN_PB_EMBEDDING_CANONICAL_BYTES_LEN];
        bytes[0..4].copy_from_slice(&schema);
        bytes[4] = self.pb_axes[0].canonical_code();
        bytes[5] = self.pb_axes[1].canonical_code();
        bytes[6] = self.pb_axes[2].canonical_code();
        bytes[7..15].copy_from_slice(&x);
        bytes[15..23].copy_from_slice(&y);
        bytes[23..31].copy_from_slice(&z);
        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExactBox3i {
    pub min: Point3i,
    pub max_exclusive: Point3i,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerrainPbCellMatch {
    pub cell: CellCoord,
    pub terrain_global_voxel: [i64; 3],
    pub exact_box: ExactBox3i,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerrainPbMappingError {
    VoxelOutsideChunk(TerrainVoxelIndex),
    EdgeMismatch {
        terrain_voxel_edge_um: i64,
        pb_quantum_um: i64,
    },
    DuplicateSourceAxis(Axis),
    ArithmeticOverflow(&'static str),
    NoExactCellMatch(ExactBox3i),
    MultipleExactCellMatches {
        first: CellCoord,
        second: CellCoord,
    },
    Pb(DecompositionError),
}

impl From<DecompositionError> for TerrainPbMappingError {
    fn from(value: DecompositionError) -> Self {
        Self::Pb(value)
    }
}

impl fmt::Display for TerrainPbMappingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VoxelOutsideChunk(voxel) => {
                write!(f, "Terrain voxel {voxel:?} lies outside its canonical chunk")
            }
            Self::EdgeMismatch {
                terrain_voxel_edge_um,
                pb_quantum_um,
            } => write!(
                f,
                "Terrain voxel edge {terrain_voxel_edge_um} um does not equal PB quantum {pb_quantum_um} um"
            ),
            Self::DuplicateSourceAxis(axis) => {
                write!(f, "embedding reuses Terrain source axis {axis:?}")
            }
            Self::ArithmeticOverflow(stage) => {
                write!(f, "checked Terrain/PB mapping arithmetic overflow at {stage}")
            }
            Self::NoExactCellMatch(bounds) => {
                write!(f, "no PB cell has exact bounds {bounds:?}")
            }
            Self::MultipleExactCellMatches { first, second } => write!(
                f,
                "multiple PB cells have the same exact Terrain bounds: {first:?} and {second:?}"
            ),
            Self::Pb(error) => error.fmt(f),
        }
    }
}

impl Error for TerrainPbMappingError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Pb(error) => Some(error),
            _ => None,
        }
    }
}

/// Proves that one exact Terrain voxel box equals exactly one PB cell box.
///
/// This function is intentionally a bounded reference theorem: it enumerates
/// every cell in the validated PB analysis domain and compares exact half-open
/// integer boxes obtained from PB's qualified `exact_cell_bounds` authority.
/// A future O(1) inverse mapper must be separately proven equivalent to this
/// reference path before it can replace qualification execution.
pub fn map_terrain_voxel_to_pb_cell(
    chunk: EarthChunkLatticeCoord,
    voxel: TerrainVoxelIndex,
    terrain_metric: TerrainMetricProfile,
    embedding: TerrainPbEmbeddingProfile,
    domain: &AnalysisDomain,
    pb_profile: &DecompositionProfile,
) -> Result<TerrainPbCellMatch, TerrainPbMappingError> {
    validate_voxel(voxel)?;

    let terrain_edge = terrain_metric.voxel_edge_um();
    let pb_quantum = pb_profile.quantum_um();
    if terrain_edge != pb_quantum {
        return Err(TerrainPbMappingError::EdgeMismatch {
            terrain_voxel_edge_um: terrain_edge,
            pb_quantum_um: pb_quantum,
        });
    }

    let global = [
        global_axis(chunk.x, voxel.x)?,
        global_axis(chunk.y, voxel.y)?,
        global_axis(chunk.z, voxel.z)?,
    ];
    let source_box = terrain_box(global, terrain_edge)?;
    let target_box = transform_box(source_box, embedding)?;

    let dimensions = domain.dimensions();
    let mut matched = None;
    for z in 0..dimensions[2] {
        for y in 0..dimensions[1] {
            for x in 0..dimensions[0] {
                let cell = CellCoord::new(x, y, z);
                let (min, max_exclusive) = domain.exact_cell_bounds(pb_profile, cell)?;
                if (ExactBox3i { min, max_exclusive }) != target_box {
                    continue;
                }
                if let Some(first) = matched {
                    return Err(TerrainPbMappingError::MultipleExactCellMatches {
                        first,
                        second: cell,
                    });
                }
                matched = Some(cell);
            }
        }
    }

    let cell = matched.ok_or(TerrainPbMappingError::NoExactCellMatch(target_box))?;
    Ok(TerrainPbCellMatch {
        cell,
        terrain_global_voxel: global,
        exact_box: target_box,
    })
}

fn validate_voxel(voxel: TerrainVoxelIndex) -> Result<(), TerrainPbMappingError> {
    let limit = TERRAIN_GEOMETRY_CHUNK_SIZE as u8;
    if voxel.x >= limit || voxel.y >= limit || voxel.z >= limit {
        return Err(TerrainPbMappingError::VoxelOutsideChunk(voxel));
    }
    Ok(())
}

fn global_axis(chunk: i32, local: u8) -> Result<i64, TerrainPbMappingError> {
    i64::from(chunk)
        .checked_mul(TERRAIN_GEOMETRY_CHUNK_SIZE as i64)
        .and_then(|value| value.checked_add(i64::from(local)))
        .ok_or(TerrainPbMappingError::ArithmeticOverflow(
            "chunk_to_global_voxel",
        ))
}

fn terrain_box(
    global: [i64; 3],
    edge_um: i64,
) -> Result<ExactBox3i, TerrainPbMappingError> {
    let mut min = [0_i64; 3];
    let mut max = [0_i64; 3];
    for axis in 0..3 {
        min[axis] = global[axis]
            .checked_mul(edge_um)
            .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                "global_voxel_to_physical_min",
            ))?;
        max[axis] = min[axis]
            .checked_add(edge_um)
            .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                "global_voxel_to_physical_max",
            ))?;
    }
    Ok(ExactBox3i {
        min: Point3i::new(min[0], min[1], min[2]),
        max_exclusive: Point3i::new(max[0], max[1], max[2]),
    })
}

fn transform_box(
    source: ExactBox3i,
    embedding: TerrainPbEmbeddingProfile,
) -> Result<ExactBox3i, TerrainPbMappingError> {
    let source_min = [source.min.x, source.min.y, source.min.z];
    let source_max = [
        source.max_exclusive.x,
        source.max_exclusive.y,
        source.max_exclusive.z,
    ];
    let translation = [
        embedding.translation_um.x,
        embedding.translation_um.y,
        embedding.translation_um.z,
    ];
    let mut target_min = [0_i64; 3];
    let mut target_max = [0_i64; 3];

    for destination in 0..3 {
        let mapping = embedding.pb_axes[destination];
        let source_index = mapping.source.index();
        let (min, max) = match mapping.sign {
            AxisSign::Positive => (
                source_min[source_index]
                    .checked_add(translation[destination])
                    .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                        "positive_embedding_min",
                    ))?,
                source_max[source_index]
                    .checked_add(translation[destination])
                    .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                        "positive_embedding_max",
                    ))?,
            ),
            AxisSign::Negative => (
                source_max[source_index]
                    .checked_neg()
                    .and_then(|value| value.checked_add(translation[destination]))
                    .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                        "negative_embedding_min",
                    ))?,
                source_min[source_index]
                    .checked_neg()
                    .and_then(|value| value.checked_add(translation[destination]))
                    .ok_or(TerrainPbMappingError::ArithmeticOverflow(
                        "negative_embedding_max",
                    ))?,
            ),
        };
        target_min[destination] = min;
        target_max[destination] = max;
    }

    Ok(ExactBox3i {
        min: Point3i::new(target_min[0], target_min[1], target_min[2]),
        max_exclusive: Point3i::new(target_max[0], target_max[1], target_max[2]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_source_axes_fail_closed() {
        let result = TerrainPbEmbeddingProfile::new(
            [
                SignedAxis::positive(Axis::X),
                SignedAxis::negative(Axis::X),
                SignedAxis::positive(Axis::Z),
            ],
            Point3i::new(0, 0, 0),
        );
        assert_eq!(result, Err(TerrainPbMappingError::DuplicateSourceAxis(Axis::X)));
    }

    #[test]
    fn canonical_embedding_bytes_are_frozen() {
        let profile = TerrainPbEmbeddingProfile::new(
            [
                SignedAxis::positive(Axis::Y),
                SignedAxis::negative(Axis::X),
                SignedAxis::positive(Axis::Z),
            ],
            Point3i::new(10, -20, 30),
        )
        .unwrap();
        let bytes = profile.canonical_bytes();
        assert_eq!(bytes.len(), TERRAIN_PB_EMBEDDING_CANONICAL_BYTES_LEN);
        assert_eq!(&bytes[0..4], &1_u32.to_le_bytes());
        assert_eq!(&bytes[4..7], &[2, 1, 4]);
        assert_eq!(&bytes[7..15], &10_i64.to_le_bytes());
        assert_eq!(&bytes[15..23], &(-20_i64).to_le_bytes());
        assert_eq!(&bytes[23..31], &30_i64.to_le_bytes());
    }

    #[test]
    fn reflected_half_open_bounds_use_negative_max_then_negative_min() {
        let source = ExactBox3i {
            min: Point3i::new(10, 20, 30),
            max_exclusive: Point3i::new(20, 30, 40),
        };
        let embedding = TerrainPbEmbeddingProfile::new(
            [
                SignedAxis::positive(Axis::Y),
                SignedAxis::negative(Axis::X),
                SignedAxis::positive(Axis::Z),
            ],
            Point3i::new(100, 200, 300),
        )
        .unwrap();
        assert_eq!(
            transform_box(source, embedding).unwrap(),
            ExactBox3i {
                min: Point3i::new(120, 180, 330),
                max_exclusive: Point3i::new(130, 190, 340),
            }
        );
    }
}
