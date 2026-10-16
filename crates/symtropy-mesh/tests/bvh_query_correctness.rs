// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic regression tests for BVH query conservativeness.
//!
//! The BVH may return false positives (broadphase candidates), but it must not
//! omit any triangle whose exact vertex AABB overlaps the query AABB.

use nalgebra::SVector;
use symtropy_mesh::{MeshBvhBuildStrategy, Triangle, TriangleMesh};
use symtropy_physics::broadphase::Aabb;

fn grid_mesh(width: usize, depth: usize) -> TriangleMesh {
    let mut triangles = Vec::with_capacity(width * depth * 2);
    for z in 0..depth {
        for x in 0..width {
            let x = x as f64;
            let z = z as f64;
            let p00 = SVector::<f64, 3>::from([x, 0.0, z]);
            let p10 = SVector::<f64, 3>::from([x + 1.0, 0.0, z]);
            let p11 = SVector::<f64, 3>::from([x + 1.0, 0.0, z + 1.0]);
            let p01 = SVector::<f64, 3>::from([x, 0.0, z + 1.0]);
            triangles.push(Triangle { vertices: [p00, p10, p11] });
            triangles.push(Triangle { vertices: [p00, p11, p01] });
        }
    }
    TriangleMesh::new(triangles)
}

fn exact_vertex_aabb(triangle: &Triangle) -> Aabb<3> {
    let mut min = triangle.vertices[0];
    let mut max = triangle.vertices[0];
    for vertex in &triangle.vertices[1..] {
        for axis in 0..3 {
            min[axis] = min[axis].min(vertex[axis]);
            max[axis] = max[axis].max(vertex[axis]);
        }
    }
    Aabb { min, max }
}

fn overlaps(a: &Aabb<3>, b: &Aabb<3>) -> bool {
    (0..3).all(|axis| a.min[axis] <= b.max[axis] && a.max[axis] >= b.min[axis])
}

#[test]
fn bvh_never_omits_exact_triangle_aabb_overlaps() {
    let mesh = grid_mesh(48, 37);
    let mut query_count = 0usize;

    // Fixed, reproducible query corpus: narrow boxes, broad boxes, and boxes
    // near grid boundaries. Include partial out-of-mesh regions as well.
    for qz in [-2.0, 0.0, 0.25, 1.0, 7.5, 18.0, 35.5, 40.0] {
        for qx in [-2.0, 0.0, 0.25, 1.0, 13.5, 24.0, 46.5, 50.0] {
            for half_extent in [0.1, 0.75, 2.5] {
                let query = Aabb {
                    min: SVector::<f64, 3>::from([
                        qx - half_extent,
                        -0.25,
                        qz - half_extent,
                    ]),
                    max: SVector::<f64, 3>::from([
                        qx + half_extent,
                        0.25,
                        qz + half_extent,
                    ]),
                };
                let candidates = mesh.query_overlap(&query);
                let candidate_set: std::collections::HashSet<usize> =
                    candidates.iter().copied().collect();

                assert_eq!(
                    candidate_set.len(),
                    candidates.len(),
                    "query returned duplicate triangle indices"
                );

                for (index, triangle) in mesh.triangles.iter().enumerate() {
                    if overlaps(&exact_vertex_aabb(triangle), &query) {
                        assert!(
                            candidate_set.contains(&index),
                            "false negative: triangle {index} overlaps query {query_count}"
                        );
                    }
                }
                query_count += 1;
            }
        }
    }
    assert!(query_count > 100);
}

fn deterministic_coord(state: &mut u64) -> f64 {
    // Fixed LCG: stable across platforms and independent of external crates.
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    let value = ((*state >> 32) % 2_001) as i64 - 1_000;
    value as f64 / 10.0
}

#[test]
fn seeded_nonuniform_and_degenerate_mesh_matches_bruteforce_overlap_oracle() {
    let mut state = 0x534D_5452_4F50_5955u64;
    let triangle_count = 512usize;
    let mut triangles = Vec::with_capacity(triangle_count);

    for index in 0..triangle_count {
        let a = SVector::<f64, 3>::from([
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
        ]);
        let b = SVector::<f64, 3>::from([
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
        ]);
        let mut c = SVector::<f64, 3>::from([
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
        ]);

        // Preserve deterministic negative controls: explicit zero-area
        // triangles and elongated triangles mixed into a nonuniform cloud.
        if index % 11 == 0 {
            c = a;
        } else if index % 13 == 0 {
            c[0] = a[0] + 100.0;
        }

        triangles.push(Triangle { vertices: [a, b, c] });
    }
    let mesh = TriangleMesh::new(triangles);

    const EXTENTS: [f64; 5] = [0.0, 0.001, 0.1, 3.0, 25.0];
    let query_count = 256usize;

    for query_index in 0..query_count {
        let center = SVector::<f64, 3>::from([
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
            deterministic_coord(&mut state),
        ]);
        let half_extent = EXTENTS[query_index % EXTENTS.len()];
        let query = Aabb {
            min: SVector::<f64, 3>::from([
                center[0] - half_extent,
                center[1] - half_extent,
                center[2] - half_extent,
            ]),
            max: SVector::<f64, 3>::from([
                center[0] + half_extent,
                center[1] + half_extent,
                center[2] + half_extent,
            ]),
        };

        let candidates = mesh.query_overlap(&query);
        let candidate_set: std::collections::HashSet<usize> =
            candidates.iter().copied().collect();

        assert_eq!(
            candidates.len(),
            candidate_set.len(),
            "duplicate candidate index for seeded query {query_index}"
        );
        assert!(
            candidates.iter().all(|&index| index < mesh.triangles.len()),
            "out-of-range triangle index for seeded query {query_index}"
        );

        // Independent O(N) oracle; the BVH is not used to derive expected
        // candidates. For this AABB query API, leaf bounds are exact triangle
        // vertex AABBs, so the candidate set should match the oracle exactly.
        let expected_set: std::collections::HashSet<usize> = mesh
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(triangle_index, triangle)| {
                overlaps(&exact_vertex_aabb(triangle), &query).then_some(triangle_index)
            })
            .collect();

        assert_eq!(
            candidate_set, expected_set,
            "BVH candidates differ from exact triangle-AABB oracle for query {query_index}"
        );
    }
}

#[test]
fn bvh_leaf_bounds_are_exact_vertex_aabbs() {
    let mesh = grid_mesh(17, 11);
    let mut leaf_count = 0usize;

    for node in &mesh.bvh_nodes {
        let Some(triangle_index) = node.triangle_idx else {
            continue;
        };
        leaf_count += 1;
        let expected = exact_vertex_aabb(&mesh.triangles[triangle_index]);
        assert_eq!(node.aabb.min, expected.min, "leaf min bound is not tight");
        assert_eq!(node.aabb.max, expected.max, "leaf max bound is not tight");
    }

    assert_eq!(leaf_count, mesh.triangles.len(), "every triangle needs exactly one leaf");
}

#[test]
fn binned_sah_and_median_preserve_exact_query_candidate_sets() {
    let median_source = grid_mesh(32, 24);
    let triangles = median_source.triangles.clone();
    let median = TriangleMesh::with_bvh_strategy(
        triangles.clone(),
        MeshBvhBuildStrategy::Median,
    );
    let sah = TriangleMesh::with_bvh_strategy(
        triangles,
        MeshBvhBuildStrategy::BinnedSah { bin_count: 16 },
    );

    for qz in [-1.0, 0.0, 0.5, 4.5, 12.0, 23.5, 27.0] {
        for qx in [-1.0, 0.0, 0.5, 8.5, 16.0, 31.5, 35.0] {
            for half_extent in [0.0, 0.2, 1.0, 4.0] {
                let query = Aabb {
                    min: SVector::<f64, 3>::from([
                        qx - half_extent,
                        -0.25,
                        qz - half_extent,
                    ]),
                    max: SVector::<f64, 3>::from([
                        qx + half_extent,
                        0.25,
                        qz + half_extent,
                    ]),
                };
                let median_candidates: std::collections::HashSet<usize> =
                    median.query_overlap(&query).into_iter().collect();
                let sah_candidates: std::collections::HashSet<usize> =
                    sah.query_overlap(&query).into_iter().collect();

                assert_eq!(
                    sah_candidates,
                    median_candidates,
                    "SAH changed query semantics for ({qx}, {qz}, extent {half_extent})"
                );
            }
        }
    }
}

#[test]
fn both_builders_match_exact_oracle_on_seeded_nonuniform_degenerate_corpus() {
    let mut geometry_state = 0x534D_5452_4F50_5955u64;
    let triangle_count = 512usize;
    let mut triangles = Vec::with_capacity(triangle_count);

    for index in 0..triangle_count {
        let a = SVector::<f64, 3>::from([
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
        ]);
        let b = SVector::<f64, 3>::from([
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
        ]);
        let mut c = SVector::<f64, 3>::from([
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
            deterministic_coord(&mut geometry_state),
        ]);

        // Keep explicit negative controls in both construction strategies.
        if index % 11 == 0 {
            c = a;
        } else if index % 13 == 0 {
            c[0] = a[0] + 100.0;
        }
        triangles.push(Triangle { vertices: [a, b, c] });
    }

    let median = TriangleMesh::with_bvh_strategy(
        triangles.clone(),
        MeshBvhBuildStrategy::Median,
    );
    let sah = TriangleMesh::with_bvh_strategy(
        triangles,
        MeshBvhBuildStrategy::BinnedSah { bin_count: 16 },
    );

    // Probe the populated geometry, empty space, small boxes, broad boxes, and
    // anisotropic boxes. All query data is deterministic and independent of
    // either BVH's output.
    let half_extents = [
        [0.0, 0.0, 0.0],
        [0.001, 0.001, 0.001],
        [0.1, 0.1, 0.1],
        [1.0, 0.1, 3.0],
        [3.0, 3.0, 3.0],
        [25.0, 25.0, 25.0],
    ];
    let mut query_state = 0x4256_484F_5241_434C_u64;
    for query_index in 0..384usize {
        let center = if query_index % 3 != 2 {
            let triangle_index = query_index.wrapping_mul(73) % median.triangles.len();
            let triangle = &median.triangles[triangle_index];
            (triangle.vertices[0] + triangle.vertices[1] + triangle.vertices[2]) / 3.0
        } else {
            SVector::<f64, 3>::from([
                deterministic_coord(&mut query_state),
                deterministic_coord(&mut query_state),
                deterministic_coord(&mut query_state),
            ])
        };
        let extent = half_extents[query_index % half_extents.len()];
        let query = Aabb {
            min: SVector::<f64, 3>::from([
                center[0] - extent[0],
                center[1] - extent[1],
                center[2] - extent[2],
            ]),
            max: SVector::<f64, 3>::from([
                center[0] + extent[0],
                center[1] + extent[1],
                center[2] + extent[2],
            ]),
        };

        let expected: Vec<usize> = median
            .triangles
            .iter()
            .enumerate()
            .filter_map(|(index, triangle)| {
                overlaps(&exact_vertex_aabb(triangle), &query).then_some(index)
            })
            .collect();

        let mut median_candidates = median.query_overlap(&query);
        let mut sah_candidates = sah.query_overlap(&query);
        median_candidates.sort_unstable();
        sah_candidates.sort_unstable();

        assert_eq!(
            median_candidates, expected,
            "median differs from independent oracle for seeded query {query_index}"
        );
        assert_eq!(
            sah_candidates, expected,
            "SAH differs from independent oracle for seeded query {query_index}"
        );
        assert_eq!(
            median_candidates, sah_candidates,
            "construction strategies disagree for seeded query {query_index}"
        );
    }
}

#[test]
fn instrumented_overlap_query_matches_hot_path_and_counts_traversal_work() {
    let mesh = TriangleMesh::with_bvh_strategy(
        grid_mesh(29, 23).triangles,
        MeshBvhBuildStrategy::BinnedSah { bin_count: 16 },
    );

    for qz in [-2.0, 0.0, 0.5, 8.5, 20.0, 26.0] {
        for qx in [-2.0, 0.0, 0.5, 11.5, 25.0, 31.0] {
            for half_extent in [0.0, 0.25, 1.5, 6.0] {
                let query = Aabb {
                    min: SVector::<f64, 3>::from([
                        qx - half_extent,
                        -0.25,
                        qz - half_extent,
                    ]),
                    max: SVector::<f64, 3>::from([
                        qx + half_extent,
                        0.25,
                        qz + half_extent,
                    ]),
                };

                let mut plain = mesh.query_overlap(&query);
                let (mut measured, stats) = mesh.query_overlap_with_stats(&query);
                plain.sort_unstable();
                measured.sort_unstable();

                assert_eq!(measured, plain, "instrumentation changed candidate semantics");
                assert_eq!(stats.candidate_count, measured.len());
                assert_eq!(stats.overlapping_leaves, measured.len());
                assert!(
                    stats.bounds_tested
                        >= stats.overlapping_internal_nodes + stats.overlapping_leaves,
                    "every overlapping node must have had its bound tested"
                );
                assert!(
                    stats.bounds_tested <= mesh.bvh_nodes.len(),
                    "a tree node should be tested at most once per query"
                );
            }
        }
    }
}

#[test]
fn binned_sah_degenerate_centroids_fall_back_without_losing_candidates() {
    let p0 = SVector::<f64, 3>::from([0.0, 0.0, 0.0]);
    let p1 = SVector::<f64, 3>::from([1.0, 0.0, 0.0]);
    let p2 = SVector::<f64, 3>::from([1.0, 0.0, 1.0]);
    let triangles = (0..64)
        .map(|_| Triangle { vertices: [p0, p1, p2] })
        .collect::<Vec<_>>();

    // A zero bin request is clamped to the implementation minimum. All
    // centroids coincide, so the builder must take its deterministic median
    // fallback rather than produce an empty child or recurse without progress.
    let mesh = TriangleMesh::with_bvh_strategy(
        triangles.clone(),
        MeshBvhBuildStrategy::BinnedSah { bin_count: 0 },
    );
    let repeated = TriangleMesh::with_bvh_strategy(
        triangles,
        MeshBvhBuildStrategy::BinnedSah { bin_count: 2 },
    );

    let intersects = Aabb {
        min: SVector::<f64, 3>::from([0.25, -0.1, 0.25]),
        max: SVector::<f64, 3>::from([0.75, 0.1, 0.75]),
    };
    let misses = Aabb {
        min: SVector::<f64, 3>::from([10.0, 10.0, 10.0]),
        max: SVector::<f64, 3>::from([11.0, 11.0, 11.0]),
    };

    let first = mesh.query_overlap(&intersects);
    let second = repeated.query_overlap(&intersects);
    assert_eq!(first.len(), 64);
    assert_eq!(
        first.into_iter().collect::<std::collections::HashSet<_>>(),
        second.into_iter().collect::<std::collections::HashSet<_>>()
    );
    assert!(mesh.query_overlap(&misses).is_empty());
    assert!(repeated.query_overlap(&misses).is_empty());
}

#[test]
fn empty_spatial_query_returns_no_candidates() {
    let mesh = grid_mesh(2, 2);
    let query = Aabb {
        min: SVector::<f64, 3>::from([100.0, 100.0, 100.0]),
        max: SVector::<f64, 3>::from([101.0, 101.0, 101.0]),
    };
    assert!(mesh.query_overlap(&query).is_empty());
}
