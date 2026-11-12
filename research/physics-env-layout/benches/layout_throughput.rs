use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use symtropy_physics_env_layout::{AosoaLayout, AosLayout, SoaLayout};

const SIZES: &[usize] = &[1_000, 10_000, 100_000, 1_000_000];
const STEPS: usize = 20;
const DT: f64 = 1.0 / 120.0;

fn bench_scalar(c: &mut Criterion) {
    let mut group = c.benchmark_group("independent_environment_layout_scalar");
    for &count in SIZES {
        let base = AosLayout::from_count(count);
        group.throughput(Throughput::Elements((count * STEPS) as u64));

        group.bench_with_input(BenchmarkId::new("aos", count), &count, |b, _| {
            b.iter_batched(
                || base.clone(),
                |mut state| {
                    state.step_n(STEPS, DT).expect("AoS step");
                    black_box(&state);
                },
                BatchSize::LargeInput,
            );
        });

        let soa_base = SoaLayout::from_aos(&base);
        group.bench_with_input(BenchmarkId::new("soa", count), &count, |b, _| {
            b.iter_batched(
                || soa_base.clone(),
                |mut state| {
                    state.step_n(STEPS, DT).expect("SoA step");
                    black_box(&state);
                },
                BatchSize::LargeInput,
            );
        });

        for tile in [4, 8, 16] {
            match tile {
                4 => {
                    let initial = AosoaLayout::<4>::from_aos(&base).expect("tile 4");
                    group.bench_with_input(BenchmarkId::new("aosoa4", count), &count, |b, _| {
                        b.iter_batched(
                            || initial.clone(),
                            |mut state| {
                                state.step_n(STEPS, DT).expect("AoSoA4 step");
                                black_box(&state);
                            },
                            BatchSize::LargeInput,
                        );
                    });
                }
                8 => {
                    let initial = AosoaLayout::<8>::from_aos(&base).expect("tile 8");
                    group.bench_with_input(BenchmarkId::new("aosoa8", count), &count, |b, _| {
                        b.iter_batched(
                            || initial.clone(),
                            |mut state| {
                                state.step_n(STEPS, DT).expect("AoSoA8 step");
                                black_box(&state);
                            },
                            BatchSize::LargeInput,
                        );
                    });
                }
                16 => {
                    let initial = AosoaLayout::<16>::from_aos(&base).expect("tile 16");
                    group.bench_with_input(BenchmarkId::new("aosoa16", count), &count, |b, _| {
                        b.iter_batched(
                            || initial.clone(),
                            |mut state| {
                                state.step_n(STEPS, DT).expect("AoSoA16 step");
                                black_box(&state);
                            },
                            BatchSize::LargeInput,
                        );
                    });
                }
                _ => unreachable!("fixed tile set"),
            }
        }
    }
    group.finish();
}

fn bench_parallel(c: &mut Criterion) {
    let mut group = c.benchmark_group("independent_environment_layout_parallel");
    for &count in SIZES {
        let base = AosLayout::from_count(count);
        group.throughput(Throughput::Elements((count * STEPS) as u64));

        for workers in [1, 2, 4, 8] {
            let pool = ThreadPoolBuilder::new()
                .num_threads(workers)
                .build()
                .expect("persistent Rayon pool");
            bench_aos_pool(&mut group, &base, count, workers, &pool);
            bench_soa_pool(&mut group, &base, count, workers, &pool);
            bench_aosoa8_pool(&mut group, &base, count, workers, &pool);
        }
    }
    group.finish();
}

fn bench_aos_pool(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    base: &AosLayout,
    count: usize,
    workers: usize,
    pool: &ThreadPool,
) {
    group.bench_with_input(
        BenchmarkId::new(format!("aos_{workers}w"), count),
        &count,
        |b, _| {
            b.iter_batched(
                || base.clone(),
                |mut state| {
                    pool.install(|| state.step_n_parallel(STEPS, DT).expect("AoS parallel"));
                    black_box(&state);
                },
                BatchSize::LargeInput,
            );
        },
    );
}

fn bench_soa_pool(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    base: &AosLayout,
    count: usize,
    workers: usize,
    pool: &ThreadPool,
) {
    let initial = SoaLayout::from_aos(base);
    group.bench_with_input(
        BenchmarkId::new(format!("soa_{workers}w"), count),
        &count,
        |b, _| {
            b.iter_batched(
                || initial.clone(),
                |mut state| {
                    pool.install(|| state.step_n_parallel(STEPS, DT).expect("SoA parallel"));
                    black_box(&state);
                },
                BatchSize::LargeInput,
            );
        },
    );
}

fn bench_aosoa8_pool(
    group: &mut criterion::BenchmarkGroup<'_, criterion::measurement::WallTime>,
    base: &AosLayout,
    count: usize,
    workers: usize,
    pool: &ThreadPool,
) {
    let initial = AosoaLayout::<8>::from_aos(base).expect("AoSoA 8");
    group.bench_with_input(
        BenchmarkId::new(format!("aosoa8_{workers}w"), count),
        &count,
        |b, _| {
            b.iter_batched(
                || initial.clone(),
                |mut state| {
                    pool.install(|| state.step_n_parallel(STEPS, DT).expect("AoSoA parallel"));
                    black_box(&state);
                },
                BatchSize::LargeInput,
            );
        },
    );
}

criterion_group!(benches, bench_scalar, bench_parallel);
criterion_main!(benches);
