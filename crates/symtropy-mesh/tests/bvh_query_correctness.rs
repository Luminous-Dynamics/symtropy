// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic regression tests for BVH query conservativeness.
//!
//! The BVH may return false positives (broadphase candidates), but it must not
//! omit any triangle whose exact vertex AABB overlaps the query AABB.

use nalgebra::SVector;
use symtropy_mesh::{Triangle, TriangleMesh};
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
        // candidates. False positives are allowed, false negatives are not.
        for (triangle_index, triangle) in mesh.triangles.iter().enumerate() {
            if overlaps(&exact_vertex_aabb(triangle), &query) {
                assert!(
                    candidate_set.contains(&triangle_index),
                    "seeded oracle false negative: query {query_index}, triangle {triangle_index}"
                );
            }
        }
    }
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
