// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use std::time::Duration;
use symtropy_physics::{NeighborList2d, SpatialPoint};

const INTERACTION_RADIUS: f64 = 2.0;
const SKIN: f64 = 1.0;
const CELL_SIZE: f64 = 3.0;

fn points(count: usize, spacing: f64) -> Vec<SpatialPoint> {
    (0..count)
        .map(|index| SpatialPoint {
            id: index as u64,
            x: (index % 1000) as f64 * spacing,
            y: (index / 1000) as f64 * spacing,
        })
        .collect()
}

fn translated(points: &[SpatialPoint], dx: f64, dy: f64) -> Vec<SpatialPoint> {
    points
        .iter()
        .map(|point| SpatialPoint {
            id: point.id,
            x: point.x + dx,
            y: point.y + dy,
        })
        .collect()
}

fn bench_rebuild_vs_reuse_check(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_cache_rebuild_vs_validation");
    group.sample_size(10).measurement_time(Duration::from_secs(2));

    for count in [1_000_usize, 10_000, 100_000] {
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let initial = points(count, spacing);
            let moved = translated(&initial, 0.1, -0.1);
            let cache =
                NeighborList2d::build(initial.iter().copied(), INTERACTION_RADIUS, SKIN, CELL_SIZE)
                    .unwrap();
            assert!(
                !cache
                    .requires_rebuild(&moved, INTERACTION_RADIUS, SKIN, CELL_SIZE)
                    .unwrap()
            );
            group.throughput(Throughput::Elements(count as u64));

            group.bench_function(
                BenchmarkId::new("rebuild_neighbor_list", format!("{density}_{count}")),
                |bench| {
                    bench.iter_batched(
                        || initial.iter().copied().collect::<Vec<_>>(),
                        |input| {
                            black_box(
                                NeighborList2d::build(
                                    input,
                                    INTERACTION_RADIUS,
                                    SKIN,
                                    CELL_SIZE,
                                )
                                .unwrap(),
                            )
                        },
                        BatchSize::LargeInput,
                    );
                },
            );

            group.bench_function(
                BenchmarkId::new("validate_and_reuse_existing_list", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let valid = !cache
                            .requires_rebuild(
                                &moved,
                                INTERACTION_RADIUS,
                                SKIN,
                                CELL_SIZE,
                            )
                            .unwrap();
                        black_box((valid, cache.pairs().len()))
                    });
                },
            );
        }
    }
    group.finish();
}

/// Compares naive rebuild-every-tick behavior against conservative cache reuse
/// over a slowly translating 120-tick tape. The moving positions remain well
/// inside the skin until the configured displacement threshold requires rebuild.
fn bench_low_motion_tape(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_cache_low_motion_120_ticks");
    group.sample_size(10).measurement_time(Duration::from_secs(2));

    for count in [1_000_usize, 10_000] {
        for (density, spacing) in [("sparse", 20.0), ("dense", 1.0)] {
            let initial = points(count, spacing);
            let mut motions = Vec::with_capacity(120);
            for tick in 0..120 {
                motions.push(translated(
                    &initial,
                    (tick + 1) as f64 * 0.04,
                    -((tick + 1) as f64) * 0.03,
                ));
            }

            group.bench_function(
                BenchmarkId::new("rebuild_every_tick", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let mut final_pairs = 0_usize;
                        for current in &motions {
                            let list = NeighborList2d::build(
                                current.iter().copied(),
                                INTERACTION_RADIUS,
                                SKIN,
                                CELL_SIZE,
                            )
                            .unwrap();
                            final_pairs = list.len();
                        }
                        black_box(final_pairs)
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("skin_guarded_reuse", format!("{density}_{count}")),
                |bench| {
                    bench.iter(|| {
                        let mut cached = NeighborList2d::build(
                            initial.iter().copied(),
                            INTERACTION_RADIUS,
                            SKIN,
                            CELL_SIZE,
                        )
                        .unwrap();
                        let mut rebuilds = 1_usize;
                        for current in &motions {
                            if cached
                                .requires_rebuild(
                                    current,
                                    INTERACTION_RADIUS,
                                    SKIN,
                                    CELL_SIZE,
                                )
                                .unwrap()
                            {
                                cached = NeighborList2d::build(
                                    current.iter().copied(),
                                    INTERACTION_RADIUS,
                                    SKIN,
                                    CELL_SIZE,
                                )
                                .unwrap();
                                rebuilds += 1;
                            }
                            black_box(cached.pairs().len());
                        }
                        black_box((rebuilds, cached.len()))
                    });
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_rebuild_vs_reuse_check, bench_low_motion_tape);
criterion_main!(benches);
