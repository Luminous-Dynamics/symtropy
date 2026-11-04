// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Independent-rollout batch/replay throughput benchmark.
//!
//! This benchmark measures independent PhysicsWorld rollouts scheduled across
//! standard-library worker threads. It does not claim that one PhysicsWorld
//! step is internally parallel. Each rollout is identified only by its stable
//! ID; worker assignment and completion order do not influence its input tape.
//!
//! The measured parallel path includes per-batch thread creation/join overhead.
//! Treat it as an end-to-end batch baseline, not as a persistent-pool lower
//! bound. Criterion reports completed rollouts/second for the fixed campaign.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use std::sync::Mutex;
use std::thread;
use symtropy_math::Point;
use symtropy_physics::{
    BatchRolloutSnapshot, PhysicsWorld, ReplayTape, WorldCommand, WorldSnapshot,
    apply_commands, order_batch_results,
};

const ROLLOUTS: usize = 64;
const TICKS: usize = 90;
const DT: f64 = 1.0 / 120.0;
const BODY_COUNT: usize = 5;

fn run_rollout(rollout_id: u64) -> BatchRolloutSnapshot<3> {
    let offset = (rollout_id % 7) as f64 * 0.015;
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
    let mut handles = Vec::with_capacity(BODY_COUNT);

    for body_index in 0..BODY_COUNT {
        let x = (body_index % 3) as f64 * 0.82 + offset;
        let y = 2.0 + (body_index / 3) as f64 * 0.86;
        let mass = 1.0 + (body_index % 3) as f64 * 0.25;
        let handle = world.add_sphere(Point::new([x, y, 0.0]), 0.5, mass);
        if let Some(body) = world.body_mut(handle) {
            body.linear_damping = 0.0;
            body.angular_damping = 0.0;
            let vx = ((rollout_id * 13 + body_index as u64 * 7) % 11) as f64 * 0.01;
            body.linear_velocity = SVector::from([vx, 0.0, 0.0]);
        }
        handles.push(handle);
    }

    // The command tape is determined by rollout_id and tick, not by the
    // worker index or completion schedule.
    let mut tape = ReplayTape::<3>::default();
    for tick in 0..TICKS {
        let commands = if tick % 30 == 0 {
            let body_index = (tick / 30) % handles.len();
            vec![WorldCommand::ApplyForce {
                body: handles[body_index],
                force: Box::new(SVector::from([0.05 + offset, 0.0, 0.0])),
            }]
        } else {
            Vec::new()
        };
        tape.push_frame(DT, commands);
    }

    for frame in &tape.frames {
        apply_commands(&mut world, &frame.commands)
            .expect("frozen rollout tape must reference existing bodies");
        world.step(frame.dt);
    }

    BatchRolloutSnapshot {
        rollout_id,
        snapshot: WorldSnapshot::capture(&world),
    }
}

fn run_sequential() -> Vec<BatchRolloutSnapshot<3>> {
    let results = (0..ROLLOUTS)
        .map(|rollout_id| run_rollout(rollout_id as u64))
        .collect();
    order_batch_results(ROLLOUTS, results).expect("sequential corpus IDs must be complete")
}

fn run_parallel(workers: usize) -> Vec<BatchRolloutSnapshot<3>> {
    assert!(workers > 0, "worker count must be non-zero");
    let completed = Mutex::new(Vec::with_capacity(ROLLOUTS));

    thread::scope(|scope| {
        for worker in 0..workers {
            let completed = &completed;
            scope.spawn(move || {
                for rollout_id in (worker..ROLLOUTS).step_by(workers) {
                    let result = run_rollout(rollout_id as u64);
                    completed.lock().expect("result mutex poisoned").push(result);
                }
            });
        }
    });

    order_batch_results(ROLLOUTS, completed.into_inner().expect("result mutex poisoned"))
        .expect("parallel corpus must contain every rollout ID exactly once")
}

fn bench_independent_rollout_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("independent_rollout_batch");
    group.throughput(Throughput::Elements(ROLLOUTS as u64));

    group.bench_function(BenchmarkId::new("sequential", ROLLOUTS), |bench| {
        bench.iter(|| black_box(run_sequential()));
    });

    for workers in [1, 2, 4, 8] {
        group.bench_function(
            BenchmarkId::new(format!("std_threads_{workers}"), ROLLOUTS),
            |bench| bench.iter(|| black_box(run_parallel(workers))),
        );
    }

    group.finish();
}

criterion_group!(benches, bench_independent_rollout_batch);
criterion_main!(benches);
