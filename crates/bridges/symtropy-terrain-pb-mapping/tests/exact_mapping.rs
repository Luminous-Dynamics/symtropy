use symtropy_game_state::StableId;
use symtropy_spatial_decomposition::{AnalysisDomain, CellCoord, DecompositionProfile, Point3i};
use symtropy_terrain::{
    geometry_kernel::{EarthChunkLatticeCoord, TerrainVoxelIndex},
    terrain_metric::TerrainMetricProfile,
};
use symtropy_terrain_pb_mapping::{
    Axis, SignedAxis, TerrainPbEmbeddingProfile, TerrainPbMappingError,
    map_terrain_voxel_to_pb_cell,
};

fn profile(quantum_um: i64) -> DecompositionProfile {
    DecompositionProfile::reference_v1(
        StableId::parse("pb04d1.profile").unwrap(),
        1,
        quantum_um,
    )
    .unwrap()
}

fn domain(origin: Point3i, dimensions: [u32; 3]) -> AnalysisDomain {
    AnalysisDomain::new(
        StableId::parse("pb04d1.domain").unwrap(),
        1,
        origin,
        dimensions,
    )
    .unwrap()
}

#[test]
fn identity_embedding_matches_the_unique_exact_pb_cell() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let domain = domain(Point3i::new(0, 0, 0), [4, 4, 4]);
    let result = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        TerrainVoxelIndex::new(2, 1, 3).unwrap(),
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap();

    assert_eq!(result.cell, CellCoord::new(2, 1, 3));
    assert_eq!(result.terrain_global_voxel, [2, 1, 3]);
    assert_eq!(result.exact_box.min, Point3i::new(20, 10, 30));
    assert_eq!(result.exact_box.max_exclusive, Point3i::new(30, 20, 40));
}

#[test]
fn chunk_and_local_indices_use_checked_exact_global_lattice_arithmetic() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let domain = domain(Point3i::new(170, -140, 30), [4, 4, 4]);
    let result = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(1, -1, 0),
        TerrainVoxelIndex::new(2, 3, 4).unwrap(),
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap();

    assert_eq!(result.terrain_global_voxel, [18, -13, 4]);
    assert_eq!(result.cell, CellCoord::new(1, 1, 1));
    assert_eq!(result.exact_box.min, Point3i::new(180, -130, 40));
    assert_eq!(result.exact_box.max_exclusive, Point3i::new(190, -120, 50));
}

#[test]
fn signed_permutation_reflects_half_open_box_exactly() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let embedding = TerrainPbEmbeddingProfile::new(
        [
            SignedAxis::positive(Axis::Y),
            SignedAxis::negative(Axis::X),
            SignedAxis::positive(Axis::Z),
        ],
        Point3i::new(100, 200, 300),
    )
    .unwrap();
    let domain = domain(Point3i::new(120, 180, 330), [2, 2, 2]);

    let result = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        TerrainVoxelIndex::new(1, 2, 3).unwrap(),
        metric,
        embedding,
        &domain,
        &profile,
    )
    .unwrap();

    assert_eq!(result.cell, CellCoord::new(0, 0, 0));
    assert_eq!(result.exact_box.min, Point3i::new(120, 180, 330));
    assert_eq!(result.exact_box.max_exclusive, Point3i::new(130, 190, 340));
}

#[test]
fn terrain_and_pb_edges_must_be_identical() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(20);
    let domain = domain(Point3i::new(0, 0, 0), [1, 1, 1]);
    let error = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        TerrainVoxelIndex::new(0, 0, 0).unwrap(),
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap_err();

    assert_eq!(
        error,
        TerrainPbMappingError::EdgeMismatch {
            terrain_voxel_edge_um: 10,
            pb_quantum_um: 20,
        }
    );
}

#[test]
fn public_voxel_fields_do_not_bypass_mapping_validation() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let domain = domain(Point3i::new(0, 0, 0), [1, 1, 1]);
    let forged = TerrainVoxelIndex { x: 16, y: 0, z: 0 };
    let error = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        forged,
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap_err();

    assert_eq!(error, TerrainPbMappingError::VoxelOutsideChunk(forged));
}

#[test]
fn physical_coordinate_overflow_fails_closed() {
    let edge = 1_000_000_000;
    let metric = TerrainMetricProfile::new(edge).unwrap();
    let profile = profile(edge);
    let domain = domain(Point3i::new(0, 0, 0), [1, 1, 1]);
    let error = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(i32::MAX, 0, 0),
        TerrainVoxelIndex::new(15, 0, 0).unwrap(),
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap_err();

    assert_eq!(
        error,
        TerrainPbMappingError::ArithmeticOverflow("global_voxel_to_physical_min")
    );
}

#[test]
fn embedding_translation_overflow_fails_closed() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let domain = domain(Point3i::new(0, 0, 0), [1, 1, 1]);
    let embedding = TerrainPbEmbeddingProfile::identity(Point3i::new(i64::MAX, 0, 0));
    let error = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        TerrainVoxelIndex::new(0, 0, 0).unwrap(),
        metric,
        embedding,
        &domain,
        &profile,
    )
    .unwrap_err();

    assert_eq!(
        error,
        TerrainPbMappingError::ArithmeticOverflow("positive_embedding_min")
    );
}

#[test]
fn absent_exact_pb_box_fails_closed_instead_of_rounding() {
    let metric = TerrainMetricProfile::new(10).unwrap();
    let profile = profile(10);
    let domain = domain(Point3i::new(0, 0, 0), [1, 1, 1]);
    let error = map_terrain_voxel_to_pb_cell(
        EarthChunkLatticeCoord::new(0, 0, 0),
        TerrainVoxelIndex::new(1, 0, 0).unwrap(),
        metric,
        TerrainPbEmbeddingProfile::identity(Point3i::new(0, 0, 0)),
        &domain,
        &profile,
    )
    .unwrap_err();

    assert!(matches!(error, TerrainPbMappingError::NoExactCellMatch(_)));
}
