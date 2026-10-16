// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Same-process differential benchmark for median and binned-SAH mesh BVHs.
//!
//! Run: cargo bench -p symtropy-mesh --bench bvh_queries
//! Opt into million-triangle cases with SYMTROPY_BVH_BENCH_LARGE=1.
//! Query percentiles are fixed-size batch means, not individual-query tails.
//! Build time is one sample per strategy; repeat full process runs on the same
//! machine/toolchain before drawing conclusions about construction cost.

use nalgebra::SVector;
use std::hint::black_box;
use std::time::Instant;
use symtropy_mesh::{MeshBvhBuildStrategy, Triangle, TriangleMesh};
use symtropy_physics::broadphase::Aabb;

const QUERY_COUNT: usize = 2_000;
const QUERY_BATCH_SIZE: usize = 25;
const QUERY_REPEATS: usize = 7;
const SAH_BIN_COUNT: usize = 16;

#[derive(Clone, Copy, Debug)]
enum Layout {
    Grid,
    Clustered,
    LongThin,
}

impl Layout {
    fn label(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Clustered => "clustered",
            Self::LongThin => "long_thin",
        }
    }
}

impl MeshBvhBuildStrategy {
    fn label(self) -> &'static str {
        match self {
            Self::Median => "median",
            Self::BinnedSah { .. } => "binned_sah",
        }
    }
}

fn make_grid_triangles(requested_triangles: usize) -> Vec<Triangle> {
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

    triangles
}

fn next_unit(state: &mut u64) -> f64 {
    // Fixed LCG avoids non-reproducible RNG dependencies in the benchmark.
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 32) % 1_000_001) as f64 / 1_000_000.0
}

fn make_clustered_triangles(requested_triangles: usize) -> Vec<Triangle> {
    let mut state = 0x434C_5553_5445_5253u64;
    let mut triangles = Vec::with_capacity(requested_triangles);

    for index in 0..requested_triangles {
        let cluster = index % 8;
        let cluster_x = (cluster % 4) as f64 * 100.0;
        let cluster_z = (cluster / 4) as f64 * 100.0;
        let x = cluster_x + (next_unit(&mut state) - 0.5) * 10.0;
        let z = cluster_z + (next_unit(&mut state) - 0.5) * 10.0;
        let dx = 0.02 + next_unit(&mut state) * 0.48;
        let dz = 0.02 + next_unit(&mut state) * 0.48;
        let skew = next_unit(&mut state);

        let a = SVector::<f64, 3>::from([x, 0.0, z]);
        let b = SVector::<f64, 3>::from([x + dx, 0.0, z]);
        let c = SVector::<f64, 3>::from([x + dx * skew, 0.0, z + dz]);
        triangles.push(Triangle { vertices: [a, b, c] });
    }

    triangles
}

fn make_long_thin_triangles(requested_triangles: usize) -> Vec<Triangle> {
    let mut triangles = Vec::with_capacity(requested_triangles);

    for index in 0..requested_triangles {
        let x = (index % 64) as f64 * 0.2;
        let z = index as f64 * 0.01;
        // A long, thin footprint stresses loose bounds and spatial partitioning.
        let a = SVector::<f64, 3>::from([x, 0.0, z]);
        let b = SVector::<f64, 3>::from([x + 100.0, 0.0, z]);
        let c = SVector::<f64, 3>::from([x + 100.0, 0.0, z + 0.01]);
        triangles.push(Triangle { vertices: [a, b, c] });
    }

    triangles
}

fn make_triangles(layout: Layout, requested_triangles: usize) -> Vec<Triangle> {
    match layout {
        Layout::Grid => make_grid_triangles(requested_triangles),
        Layout::Clustered => make_clustered_triangles(requested_triangles),
        Layout::LongThin => make_long_thin_triangles(requested_triangles),
    }
}

fn query_for(triangles: &[Triangle], index: usize) -> Aabb<3> {
    // Center each query on a populated triangle so every family gets useful
    // queries, even when it is clustered or highly elongated.
    let triangle_index = index.wrapping_mul(7_919) % triangles.len();
    let triangle = &triangles[triangle_index];
    let center = (triangle.vertices[0] + triangle.vertices[1] + triangle.vertices[2]) / 3.0;
    Aabb {
        min: SVector::<f64, 3>::from([
            center[0] - 0.4,
            center[1] - 0.1,
            center[2] - 0.4,
        ]),
        max: SVector::<f64, 3>::from([
            center[0] + 0.4,
            center[1] + 0.1,
            center[2] + 0.4,
        ]),
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

#[derive(Debug)]
struct Measurement {
    strategy: MeshBvhBuildStrategy,
    build_ms: f64,
    mean_query_us: f64,
    p50_query_us: f64,
    p95_query_us: f64,
    p99_query_us: f64,
    mean_candidates: f64,
    checksum: usize,
}

fn measure_strategy(
    triangles: &[Triangle],
    queries: &[Aabb<3>],
    strategy: MeshBvhBuildStrategy,
) -> Measurement {
    // Clone input before timing so the measurement covers BVH construction,
    // not fixture allocation/copying.
    let owned_triangles = triangles.to_vec();
    let build_start = Instant::now();
    let mesh = TriangleMesh::with_bvh_strategy(owned_triangles, strategy);
    let build_ms = build_start.elapsed().as_secs_f64() * 1_000.0;
    black_box(mesh.bvh_nodes.len());

    // Warm-up uses the same corpus for both implementations.
    for query in queries.iter().take(QUERY_BATCH_SIZE) {
        black_box(mesh.query_overlap(black_box(query)));
    }

    let mut batch_mean_query_us =
        Vec::with_capacity(QUERY_REPEATS * (QUERY_COUNT / QUERY_BATCH_SIZE));
    let mut expected_candidates = None;
    let mut expected_checksum = None;

    for repeat in 0..QUERY_REPEATS {
        let mut total_candidates = 0usize;
        let mut checksum = 0usize;

        for batch_start in (0..QUERY_COUNT).step_by(QUERY_BATCH_SIZE) {
            let start = Instant::now();
            for offset in 0..QUERY_BATCH_SIZE {
                // Each repetition visits the whole query corpus in a different
                // deterministic order to expose order-sensitive cache effects.
                let query_index = (batch_start + offset)
                    .wrapping_mul(7_919)
                    .wrapping_add(repeat * 101)
                    % QUERY_COUNT;
                let candidates = black_box(mesh.query_overlap(black_box(&queries[query_index])));
                total_candidates += candidates.len();
                checksum = checksum.wrapping_add(candidates.iter().copied().sum::<usize>());
            }
            batch_mean_query_us
                .push(start.elapsed().as_secs_f64() * 1_000_000.0 / QUERY_BATCH_SIZE as f64);
        }

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

    let mean_query_us =
        batch_mean_query_us.iter().sum::<f64>() / batch_mean_query_us.len() as f64;
    Measurement {
        strategy,
        build_ms,
        mean_query_us,
        p50_query_us: quantile(&batch_mean_query_us, 0.50),
        p95_query_us: quantile(&batch_mean_query_us, 0.95),
        p99_query_us: quantile(&batch_mean_query_us, 0.99),
        mean_candidates: expected_candidates.expect("at least one repeat") as f64
            / QUERY_COUNT as f64,
        checksum: expected_checksum.expect("at least one repeat"),
    }
}

fn print_measurement(layout: Layout, requested_triangles: usize, m: &Measurement) {
    println!(
        "{},{},{},{},{:.3},{},{},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
        layout.label(),
        requested_triangles,
        requested_triangles,
        m.strategy.label(),
        m.build_ms,
        QUERY_COUNT,
        QUERY_REPEATS,
        m.mean_query_us,
        m.p50_query_us,
        m.p95_query_us,
        m.p99_query_us,
        m.mean_candidates,
        m.checksum,
    );
}

fn run_pair(layout: Layout, requested_triangles: usize, reverse_order: bool) {
    let triangles = make_triangles(layout, requested_triangles);
    let queries: Vec<Aabb<3>> = (0..QUERY_COUNT).map(|i| query_for(&triangles, i)).collect();
    let median_strategy = MeshBvhBuildStrategy::Median;
    let sah_strategy = MeshBvhBuildStrategy::BinnedSah {
        bin_count: SAH_BIN_COUNT,
    };

    let (first, second) = if reverse_order {
        (
            measure_strategy(&triangles, &queries, sah_strategy),
            measure_strategy(&triangles, &queries, median_strategy),
        )
    } else {
        (
            measure_strategy(&triangles, &queries, median_strategy),
            measure_strategy(&triangles, &queries, sah_strategy),
        )
    };

    // Exact leaf bounds mean the strategies must produce the same triangle
    // AABB candidate sets and aggregate checksum; only the tree traversal and
    // construction cost are allowed to differ.
    assert_eq!(
        first.mean_candidates, second.mean_candidates,
        "strategies disagree on mean candidates for {} / {requested_triangles}",
        layout.label()
    );
    assert_eq!(
        first.checksum, second.checksum,
        "strategies disagree on query checksum for {} / {requested_triangles}",
        layout.label()
    );

    print_measurement(layout, requested_triangles, &first);
    print_measurement(layout, requested_triangles, &second);
}

fn main() {
    println!(
        "layout,requested_triangles,actual_triangles,strategy,build_ms_single_sample,queries_per_repeat,repeats,mean_batch_query_us,p50_batch_query_us,p95_batch_query_us,p99_batch_query_us,mean_candidates,checksum"
    );

    for (size_index, triangle_count) in [10_000, 100_000].into_iter().enumerate() {
        for (layout_index, layout) in [Layout::Grid, Layout::Clustered, Layout::LongThin]
            .into_iter()
            .enumerate()
        {
            // Reverse A/B order across cases to reduce systematic order bias.
            run_pair(layout, triangle_count, (size_index + layout_index) % 2 == 1);
        }
    }

    if std::env::var("SYMTROPY_BVH_BENCH_LARGE").as_deref() == Ok("1") {
        for layout in [Layout::Grid, Layout::Clustered, Layout::LongThin] {
            run_pair(layout, 1_000_000, false);
        }
    }
}
