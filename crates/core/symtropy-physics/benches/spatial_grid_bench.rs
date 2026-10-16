// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use std::time::Duration;
use symtropy_physics::{SpatialGrid2d, SpatialPoint};

const QUERY_COUNT: usize = 32;
const CELL_SIZE: f64 = 5.0;
const QUERY_RADIUS: f64 = 5.0;

fn points(count: usize, spacing: f64) -> Vec<SpatialPoint> {
    (0..count)
        .map(|index| SpatialPoint {
            id: index as u64,
            x: (index % 1000) as f64 * spacing,
            y: (index / 1000) as f64 * spacing,
        })
        .collect()
}

fn query_centers(points: &[SpatialPoint]) -> Vec<[f64; 2]> {
    let stride = (points.len() / QUERY_COUNT).max(1);
    points
        .iter()
        .step_by(stride)
        .take(QUERY_COUNT)
        .map(|point| [point.x, point.y])
        .collect()
}

fn exact_count(points: &[SpatialPoint], center: [f64; 2], radius: f64) -> usize {
    let radius_squared = radius * radius;
    points
        .iter()
        .filter(|point| {
            let dx = point.x - center[0];
            let dy = point.y - center[1];
            dx * dx + dy * dy <= radius_squared
        })
        .count()
}

fn bench_grid_build(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_grid_build");
    group.sample_size(10).measurement_time(Duration::from_secs(2));
    for count in [1_000_usize, 10_000, 100_000] {
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let input = points(count, spacing);
            group.throughput(Throughput::Elements(count as u64));
            group.bench_with_input(
                BenchmarkId::new(density, count),
                &input,
                |bench, input| {
                    bench.iter_batched(
                        || input.clone(),
                        |owned| black_box(SpatialGrid2d::new(owned, CELL_SIZE).unwrap()),
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }
    group.finish();
}

fn bench_spatial_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_grid_query_32_centers");
    group.sample_size(10).measurement_time(Duration::from_secs(2));
    for count in [1_000_usize, 10_000, 100_000] {
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let input = points(count, spacing);
            let queries = query_centers(&input);
            let index = SpatialGrid2d::new(input.clone(), CELL_SIZE).unwrap();
            group.throughput(Throughput::Elements(
                (input.len() * queries.len()) as u64,
            ));

            // Validate this benchmark fixture before timing, and emit candidate
            // counts separately so they do not contaminate measured latency.
            let naive_matches: usize = queries
                .iter()
                .map(|center| exact_count(&input, *center, QUERY_RADIUS))
                .sum();
            let mut indexed_matches = 0_usize;
            let mut fixture_candidates = 0_usize;
            let radius_squared = QUERY_RADIUS * QUERY_RADIUS;
            for center in &queries {
                let candidates = index
                    .query_radius_candidates(*center, QUERY_RADIUS)
                    .expect("fixed finite query");
                fixture_candidates += candidates.len();
                indexed_matches += candidates
                    .iter()
                    .filter(|point| {
                        let dx = point.x - center[0];
                        let dy = point.y - center[1];
                        dx * dx + dy * dy <= radius_squared
                    })
                    .count();
            }
            assert_eq!(indexed_matches, naive_matches);
            eprintln!(
                "spatial-grid query fixture: density={density} points={count} queries={} candidates={fixture_candidates} matches={indexed_matches}",
                queries.len()
            );

            group.bench_function(
                BenchmarkId::new("naive_exact_scan", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let mut matched = 0_usize;
                        for center in &queries {
                            matched += exact_count(&input, *center, QUERY_RADIUS);
                        }
                        black_box(matched)
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new(
                    "grid_candidates_plus_exact_filter",
                    format!("{density}_{count}"),
                ),
                |bench| {
                    bench.iter(|| {
                        let mut matched = 0_usize;
                        let mut candidates_seen = 0_usize;
                        for center in &queries {
                            let candidates = index
                                .query_radius_candidates(*center, QUERY_RADIUS)
                                .expect("fixed finite query");
                            candidates_seen += candidates.len();
                            let radius_squared = QUERY_RADIUS * QUERY_RADIUS;
                            matched += candidates
                                .iter()
                                .filter(|point| {
                                    let dx = point.x - center[0];
                                    let dy = point.y - center[1];
                                    dx * dx + dy * dy <= radius_squared
                                })
                                .count();
                        }
                        black_box((matched, candidates_seen))
                    });
                },
            );
        }
    }
    group.finish();
}


fn naive_full_population_count(points: &[SpatialPoint]) -> usize {
    let radius_squared = QUERY_RADIUS * QUERY_RADIUS;
    let mut matched = 0_usize;
    for center in points {
        matched += points
            .iter()
            .filter(|point| {
                let dx = point.x - center.x;
                let dy = point.y - center.y;
                dx * dx + dy * dy <= radius_squared
            })
            .count();
    }
    matched
}

fn indexed_full_population_count(
    points: &[SpatialPoint],
    index: &SpatialGrid2d,
) -> (usize, usize) {
    let radius_squared = QUERY_RADIUS * QUERY_RADIUS;
    let mut matched = 0_usize;
    let mut candidates_seen = 0_usize;
    for center in points {
        let candidates = index
            .query_radius_candidates([center.x, center.y], QUERY_RADIUS)
            .expect("finite benchmark query");
        candidates_seen += candidates.len();
        matched += candidates
            .iter()
            .filter(|point| {
                let dx = point.x - center.x;
                let dy = point.y - center.y;
                dx * dx + dy * dy <= radius_squared
            })
            .count();
    }
    (matched, candidates_seen)
}

/// Full-population neighborhood pass. Naive reference scans are limited to
/// 1k/10k populations; the 100k run reports only the measured indexed workload
/// because the equivalent all-pairs scan requires ten billion distance checks.
fn bench_full_population_rebuild(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_grid_full_population_rebuild");
    group.sample_size(10).measurement_time(Duration::from_secs(2));
    for count in [1_000_usize, 10_000, 100_000] {
        group.throughput(Throughput::Elements(count as u64));
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let input = points(count, spacing);
            let initial_index = SpatialGrid2d::new(input.iter().copied(), CELL_SIZE).unwrap();
            let (indexed_count, candidates_seen) =
                indexed_full_population_count(&input, &initial_index);
            let naive_count = if count <= 10_000 {
                Some(naive_full_population_count(&input))
            } else {
                None
            };

            if let Some(naive_count) = naive_count {
                assert_eq!(
                    indexed_count, naive_count,
                    "indexed full-population results must match naive reference"
                );
            }
            eprintln!(
                "spatial-grid full fixture: density={density} points={count} candidates={candidates_seen} matches={indexed_count} naive_reference={}",
                if naive_count.is_some() { "computed" } else { "omitted at 100k" }
            );

            if naive_count.is_some() {
                group.bench_function(
                    BenchmarkId::new("naive_full_population", format!("{density}_{count}")),
                    |bench| {
                        bench.iter(|| black_box(naive_full_population_count(&input)));
                    },
                );
            }

            group.bench_function(
                BenchmarkId::new("rebuild_grid_full_population", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let index =
                            SpatialGrid2d::new(input.iter().copied(), CELL_SIZE).unwrap();
                        black_box(indexed_full_population_count(&input, &index))
                    });
                },
            );
        }
    }
    group.finish();
}

/// Measures the per-tick case: rebuild the grid from moving positions, then
/// answer a batch of neighborhood queries. This prevents query-only results
/// from hiding the cost of maintaining a derived index.
fn bench_rebuild_plus_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("spatial_grid_rebuild_plus_32_queries");
    group.sample_size(10).measurement_time(Duration::from_secs(2));
    for count in [1_000_usize, 10_000, 100_000] {
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let input = points(count, spacing);
            let queries = query_centers(&input);
            group.throughput(Throughput::Elements(
                (input.len() * queries.len()) as u64,
            ));

            group.bench_function(
                BenchmarkId::new("naive_scan", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let mut matched = 0_usize;
                        for center in &queries {
                            matched += exact_count(&input, *center, QUERY_RADIUS);
                        }
                        black_box(matched)
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("rebuild_grid_then_query", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let index =
                            SpatialGrid2d::new(input.iter().copied(), CELL_SIZE).unwrap();
                        let mut matched = 0_usize;
                        let mut candidates_seen = 0_usize;
                        let radius_squared = QUERY_RADIUS * QUERY_RADIUS;
                        for center in &queries {
                            let candidates = index
                                .query_radius_candidates(*center, QUERY_RADIUS)
                                .expect("fixed finite query");
                            candidates_seen += candidates.len();
                            matched += candidates
                                .iter()
                                .filter(|point| {
                                    let dx = point.x - center[0];
                                    let dy = point.y - center[1];
                                    dx * dx + dy * dy <= radius_squared
                                })
                                .count();
                        }
                        black_box((matched, candidates_seen))
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_grid_build,
    bench_spatial_queries,
    bench_rebuild_plus_queries,
    bench_full_population_rebuild
);
criterion_main!(benches);
