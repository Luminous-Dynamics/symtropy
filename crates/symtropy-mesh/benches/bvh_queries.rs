// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-free, deterministic BVH build/query benchmark.
//!
//! Run: cargo bench -p symtropy-mesh --bench bvh_queries
//! Opt into the million-triangle case with SYMTROPY_BVH_BENCH_LARGE=1.
//! Query latency is reported as percentiles of fixed-size batch means, not
//! per-query timings. Build time is a single observation; repeat the command
//! in fresh processes before drawing conclusions about build-time changes.

use nalgebra::SVector;
use std::hint::black_box;
use std::time::Instant;
use symtropy_mesh::{Triangle, TriangleMesh};
use symtropy_physics::broadphase::Aabb;

const QUERY_COUNT: usize = 2_000;
const QUERY_BATCH_SIZE: usize = 25;
const QUERY_REPEATS: usize = 7;

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

fn query_for(index: usize, cells: usize, side: usize) -> Aabb<3> {
    // A fixed modular sequence makes the workload reproducible and spreads
    // queries over populated cells only, including for non-square meshes.
    let cell = index.wrapping_mul(7_919) % cells;
    let x = (cell % side) as f64 + 0.5;
    let z = (cell / side) as f64 + 0.5;
    Aabb {
        min: SVector::<f64, 3>::from([x - 0.4, -0.1, z - 0.4]),
        max: SVector::<f64, 3>::from([x + 0.4, 0.1, z + 0.4]),
    }
}

/// Nearest-rank quantile over observed batch means.
fn quantile(samples: &[f64], q: f64) -> f64 {
    assert!(!samples.is_empty(), "quantile requires at least one sample");
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = (q * sorted.len() as f64).ceil() as usize;
    sorted[rank.saturating_sub(1).min(sorted.len() - 1)]
}

fn run_case(requested_triangles: usize) {
    let build_start = Instant::now();
    let mesh = make_grid_mesh(requested_triangles);
    let build_ms = build_start.elapsed().as_secs_f64() * 1_000.0;
    black_box(mesh.triangles.len());

    let cells = requested_triangles.div_ceil(2);
    let side = (cells as f64).sqrt().ceil() as usize;
    let queries: Vec<Aabb<3>> = (0..QUERY_COUNT)
        .map(|i| query_for(i, cells, side))
        .collect();

    // A short warm-up avoids treating first-touch effects as typical latency.
    for query in queries.iter().take(QUERY_BATCH_SIZE) {
        black_box(mesh.query_overlap(black_box(query)));
    }

    let mut batch_mean_query_us = Vec::with_capacity(
        QUERY_REPEATS * (QUERY_COUNT / QUERY_BATCH_SIZE),
    );
    let mut expected_candidates = None;
    let mut expected_checksum = None;

    for repeat in 0..QUERY_REPEATS {
        let mut total_candidates = 0usize;
        let mut checksum = 0usize;

        for batch_start in (0..QUERY_COUNT).step_by(QUERY_BATCH_SIZE) {
            let start = Instant::now();
            for offset in 0..QUERY_BATCH_SIZE {
                // 7_919 is coprime to QUERY_COUNT, so each repeat visits the
                // same complete query set in a different deterministic order.
                let query_index = (batch_start + offset)
                    .wrapping_mul(7_919)
                    .wrapping_add(repeat * 101)
                    % QUERY_COUNT;
                let candidates = black_box(mesh.query_overlap(black_box(&queries[query_index])));
                total_candidates += candidates.len();
                checksum = checksum.wrapping_add(candidates.iter().copied().sum::<usize>());
            }
            let elapsed = start.elapsed();
            batch_mean_query_us.push(
                elapsed.as_secs_f64() * 1_000_000.0 / QUERY_BATCH_SIZE as f64,
            );
        }

        // The same corpus must produce the same aggregate results regardless
        // of query order. A mismatch is a benchmark failure, not a speedup.
        if let Some(expected) = expected_candidates {
            assert_eq!(total_candidates, expected, "candidate count changed between repeats");
        } else {
            expected_candidates = Some(total_candidates);
        }
        if let Some(expected) = expected_checksum {
            assert_eq!(checksum, expected, "query checksum changed between repeats");
        } else {
            expected_checksum = Some(checksum);
        }
    }

    black_box(expected_checksum);
    let mean_query_us =
        batch_mean_query_us.iter().sum::<f64>() / batch_mean_query_us.len() as f64;
    let mean_candidates =
        expected_candidates.expect("at least one repeat") as f64 / QUERY_COUNT as f64;

    println!(
        "{},{},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{},{}",
        requested_triangles,
        mesh.triangles.len(),
        build_ms,
        QUERY_COUNT,
        QUERY_REPEATS,
        mean_query_us,
        quantile(&batch_mean_query_us, 0.50),
        quantile(&batch_mean_query_us, 0.95),
        quantile(&batch_mean_query_us, 0.99),
        mean_candidates,
        expected_checksum.expect("at least one repeat"),
        QUERY_BATCH_SIZE,
    );
}

fn main() {
    println!(
        "requested_triangles,actual_triangles,build_ms_single_sample,queries_per_repeat,repeats,mean_batch_query_us,p50_batch_query_us,p95_batch_query_us,p99_batch_query_us,mean_candidates,checksum,batch_size"
    );
    run_case(10_000);
    run_case(100_000);
    if std::env::var("SYMTROPY_BVH_BENCH_LARGE").as_deref() == Ok("1") {
        run_case(1_000_000);
    }
}
