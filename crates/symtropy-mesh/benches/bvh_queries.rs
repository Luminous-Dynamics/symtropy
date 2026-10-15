// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-free, deterministic BVH build/query benchmark.
//!
//! Run: cargo bench -p symtropy-mesh --bench bvh_queries
//! Opt into the million-triangle case with SYMTROPY_BVH_BENCH_LARGE=1.
//! This reports measurements; it does not compare against a previous commit.

use nalgebra::SVector;
use std::hint::black_box;
use std::time::Instant;
use symtropy_mesh::{Triangle, TriangleMesh};
use symtropy_physics::broadphase::Aabb;

fn make_grid_mesh(requested_triangles: usize) -> TriangleMesh {
    let cells = requested_triangles.div_ceil(2);
    let side = (cells as f64).sqrt().ceil() as usize;
    let mut triangles = Vec::with_capacity(requested_triangles);

    for cell in 0..cells {
        let x = (cell % side) as f64;
        let z = (cell / side) as f64;
        let p00 = SVector::<f64, 3>::from([x, 0.0, z]);
        let p10 = SVector::<f64, 3>::from([x + 1.0, 0.0, z]);
        let p11 = SVector::<f64, 3>::from([x + 1.0, 0.0, z + 1.0]);
        let p01 = SVector::<f64, 3>::from([x, 0.0, z + 1.0]);

        triangles.push(Triangle { vertices: [p00, p10, p11] });
        if triangles.len() < requested_triangles {
            triangles.push(Triangle { vertices: [p00, p11, p01] });
        }
    }

    TriangleMesh::new(triangles)
}

fn query_for(index: usize, side: usize) -> Aabb<3> {
    // A fixed modular sequence makes the workload reproducible and spreads
    // queries throughout the surface without needing a random-number crate.
    let cell = index.wrapping_mul(7_919) % (side * side);
    let x = (cell % side) as f64 + 0.5;
    let z = (cell / side) as f64 + 0.5;
    Aabb {
        min: SVector::<f64, 3>::from([x - 0.4, -0.1, z - 0.4]),
        max: SVector::<f64, 3>::from([x + 0.4, 0.1, z + 0.4]),
    }
}

fn run_case(requested_triangles: usize) {
    let build_start = Instant::now();
    let mesh = make_grid_mesh(requested_triangles);
    let build_time = build_start.elapsed();

    let cells = requested_triangles.div_ceil(2);
    let side = (cells as f64).sqrt().ceil() as usize;
    let query_count = 2_000usize;
    let query_start = Instant::now();
    let mut total_candidates = 0usize;
    let mut checksum = 0usize;

    for i in 0..query_count {
        let query = query_for(i, side);
        let candidates = black_box(mesh.query_overlap(black_box(&query)));
        total_candidates += candidates.len();
        checksum = checksum.wrapping_add(candidates.iter().copied().sum::<usize>());
    }

    let query_elapsed = query_start.elapsed();
    black_box(checksum);
    let mean_query_us =
        query_elapsed.as_secs_f64() * 1_000_000.0 / query_count as f64;
    let mean_candidates = total_candidates as f64 / query_count as f64;

    println!(
        "{},{},{:.3},{},{:.3},{:.3},{}",
        requested_triangles,
        mesh.triangles.len(),
        build_time.as_secs_f64() * 1_000.0,
        query_count,
        mean_query_us,
        mean_candidates,
        checksum
    );
}

fn main() {
    println!(
        "requested_triangles,actual_triangles,build_ms,queries,mean_query_us,mean_candidates,checksum"
    );
    run_case(10_000);
    run_case(100_000);
    if std::env::var("SYMTROPY_BVH_BENCH_LARGE").as_deref() == Ok("1") {
        run_case(1_000_000);
    }
}
