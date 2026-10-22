// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use symtropy_physics::{TickId, TickTransaction};

fn synthetic_digest(value: u64) -> [u8; 32] {
    let mut digest = [0_u8; 32];
    digest[..8].copy_from_slice(&value.to_le_bytes());
    digest[8..16].copy_from_slice(&value.rotate_left(17).to_le_bytes());
    for (index, byte) in digest[16..].iter_mut().enumerate() {
        *byte = value.wrapping_mul(31).wrapping_add(index as u64) as u8;
    }
    digest
}

fn bench_tick_transaction_lifecycle(c: &mut Criterion) {
    let mut group = c.benchmark_group("tick_transaction_lifecycle");
    for ticks in [1_000_u64, 10_000, 100_000] {
        group.throughput(Throughput::Elements(ticks));
        group.bench_with_input(
            BenchmarkId::new("commit_plus_three_idempotent_retries", ticks),
            &ticks,
            |bench, &ticks| {
                bench.iter(|| {
                    let mut transaction = TickTransaction::default();
                    for counter in 0..ticks {
                        let tick = TickId(counter);
                        transaction
                            .begin(
                                tick,
                                synthetic_digest(counter ^ 0xa5a5),
                                synthetic_digest(counter),
                            )
                            .expect("monotonic tick begins");
                        transaction
                            .mark_physics_applied(tick)
                            .expect("physics phase follows begin");
                        let result = synthetic_digest(counter.wrapping_add(1));
                        let receipt = transaction
                            .finalize(tick, result)
                            .expect("physics-applied tick finalizes");
                        for _ in 0..3 {
                            assert_eq!(
                                transaction.finalize(tick, result),
                                Ok(receipt),
                                "a retry must reproduce the original receipt",
                            );
                        }
                    }
                    black_box(transaction.last_finalized_tick());
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_tick_transaction_lifecycle);
criterion_main!(benches);
