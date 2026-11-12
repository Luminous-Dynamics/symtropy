// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{Capsule, ConvexHull, HyperBox, Point, Sphere};
use symtropy_physics::action_horizon::{
    ActionExecutionMode, ActionHorizonRequest, HorizonConstraint, HorizonEvidence,
    create_action_commitment, select_action_horizon,
};
use symtropy_physics::PhysicsWorld;
use symtropy_physics::gjk;
use symtropy_physics::raycast;

fn bench_gjk_sphere_sphere(c: &mut Criterion) {
    let a = Sphere::<3>::unit();
    let b = Sphere::<3>::unit();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_sphere_sphere_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_box_box(c: &mut Criterion) {
    let a = ConvexHull::<3>::unit_cube();
    let b = ConvexHull::<3>::unit_cube();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_box_box_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_4d_tesseract(c: &mut Criterion) {
    let a = ConvexHull::<4>::unit_cube();
    let b = ConvexHull::<4>::unit_cube();
    let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0, 0.0]);

    c.bench_function("gjk_tesseract_4d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_physics_step_10_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_10_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..10 {
                    let h = world.add_sphere(Point::new([i as f64 * 3.0, 10.0, 0.0]), 0.5, 1.0);
                    world.body_mut(h).unwrap().linear_velocity = SVector::from([0.0, -5.0, 0.0]);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_physics_step_100_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_100_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..100 {
                    let x = (i % 10) as f64 * 2.5;
                    let y = (i / 10) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y, 0.0]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_physics_step_2d(c: &mut Criterion) {
    c.bench_function("physics_step_50_bodies_2d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, -9.81]));
                for i in 0..50 {
                    let x = (i % 10) as f64 * 2.5;
                    let y = (i / 10) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ═══ New shape benchmarks ═══

fn bench_gjk_capsule_capsule(c: &mut Criterion) {
    let a = Capsule::<3>::y_aligned(1.0, 0.5);
    let b = Capsule::<3>::y_aligned(1.0, 0.5);
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_capsule_capsule_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_hyperbox_hyperbox(c: &mut Criterion) {
    let a = HyperBox::<3>::cube(1.0);
    let b = HyperBox::<3>::cube(1.0);
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_hyperbox_hyperbox_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_hyperbox_4d(c: &mut Criterion) {
    let a = HyperBox::<4>::cube(1.0);
    let b = HyperBox::<4>::cube(1.0);
    let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0, 0.0]);

    c.bench_function("gjk_hyperbox_4d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

// ═══ EPA benchmarks ═══

fn bench_epa_sphere_sphere(c: &mut Criterion) {
    let a = Sphere::<3>::unit();
    let b = Sphere::<3>::unit();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([0.5, 0.0, 0.0]);

    c.bench_function("epa_sphere_sphere_3d", |bench| {
        let result = gjk::intersects(&a, &pa, &b, &pb);
        bench.iter(|| {
            black_box(symtropy_physics::epa::penetration(
                &a,
                &pa,
                &b,
                &pb,
                &result.simplex,
            ));
        });
    });
}

// ═══ Raycast benchmarks ═══

fn bench_raycast_100_bodies(c: &mut Criterion) {
    c.bench_function("raycast_100_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::zeros());
                for i in 0..100 {
                    let x = (i % 10) as f64 * 3.0;
                    let z = (i / 10) as f64 * 3.0;
                    world.add_sphere(Point::new([x, 0.0, z]), 1.0, 1.0);
                }
                world
            },
            |world| {
                let origin = SVector::from([-5.0, 0.0, 15.0]);
                let dir = SVector::from([1.0, 0.0, 0.0]);
                black_box(raycast::raycast(&world, &origin, &dir, 100.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ═══ Stress tests ═══

fn bench_physics_step_500_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_500_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..500 {
                    let x = (i % 25) as f64 * 2.5;
                    let y = (i / 25) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y, 0.0]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::LargeInput,
        );
    });
}

// ═══ Evidence-bounded action-horizon selector ═══

fn make_action_horizon_request(
    execution_mode: ActionExecutionMode,
    p99_latency_ns: u64,
    unknown_thermal_bound: bool,
) -> ActionHorizonRequest {
    let remaining_ns = [
        50_000_000,
        45_000_000,
        40_000_000,
        35_000_000,
        30_000_000,
        25_000_000,
        20_000_000,
        18_000_000,
    ];
    let bounds = std::array::from_fn(|index| {
        if index == 6 && unknown_thermal_bound {
            HorizonEvidence::unknown("thermal model unavailable")
                .expect("unknown fixture must have a reason")
        } else {
            HorizonEvidence::qualified(
                HorizonConstraint::ALL[index],
                remaining_ns[index],
                HorizonConstraint::ALL[index].as_str(),
                "criterion-application-monotonic-epoch",
            )
            .expect("qualified fixture must have source and timebase identities")
        }
    });

    ActionHorizonRequest {
        bounds,
        evaluation_time_ns: 1_000_000,
        requested_cycles: 12,
        controller_period_ns: 1_000_000,
        p99_total_latency_ns: p99_latency_ns,
        latency_profile_id: "criterion-fixed-target-latency-profile".to_owned(),
        execution_mode,
    }
}

fn bench_action_horizon_queued_feasible(c: &mut Criterion) {
    let request = make_action_horizon_request(
        ActionExecutionMode::QueuedOpenLoop,
        10_500_000,
        false,
    );
    c.bench_function("action_horizon_queued_open_loop_8_bounds", |bench| {
        bench.iter(|| {
            black_box(
                select_action_horizon(black_box(&request))
                    .expect("fixture period and latency profile are valid"),
            );
        });
    });
}

fn bench_action_horizon_split_timescale(c: &mut Criterion) {
    let request = make_action_horizon_request(
        ActionExecutionMode::SplitTimescaleLocalExpert,
        250_000_000,
        false,
    );
    c.bench_function("action_horizon_split_timescale_local_expert_8_bounds", |bench| {
        bench.iter(|| {
            black_box(
                select_action_horizon(black_box(&request))
                    .expect("fixture period and latency profile are valid"),
            );
        });
    });
}

fn bench_action_horizon_unknown_rejection(c: &mut Criterion) {
    let request = make_action_horizon_request(
        ActionExecutionMode::QueuedOpenLoop,
        10_500_000,
        true,
    );
    c.bench_function("action_horizon_unknown_thermal_rejection_8_bounds", |bench| {
        bench.iter(|| {
            black_box(
                select_action_horizon(black_box(&request))
                    .expect("fixture period and latency profile are valid"),
            );
        });
    });
}

fn bench_action_commitment_creation(c: &mut Criterion) {
    let request = make_action_horizon_request(
        ActionExecutionMode::QueuedOpenLoop,
        10_500_000,
        false,
    );
    let selection = select_action_horizon(&request)
        .expect("fixture period and latency profile are valid");
    c.bench_function("action_commitment_create_immutable_receipt", |bench| {
        bench.iter(|| {
            black_box(create_action_commitment(
                black_box(selection.clone()),
                [0x11; 32],
                [0x22; 32],
                42,
                1_000_000,
            ))
        });
    });
}

fn bench_action_commitment_validation(c: &mut Criterion) {
    let request = make_action_horizon_request(
        ActionExecutionMode::QueuedOpenLoop,
        10_500_000,
        false,
    );
    let selection = select_action_horizon(&request)
        .expect("fixture period and latency profile are valid");
    let commitment = create_action_commitment(
        selection,
        [0x11; 32],
        [0x22; 32],
        42,
        1_000_000,
    )
    .expect("selector fixture is admissible");

    c.bench_function("action_commitment_validate_exact_proposal_config_and_sequence", |bench| {
        bench.iter(|| {
            black_box(commitment.validate(
                1_001_000,
                "criterion-application-monotonic-epoch",
                &[0x11; 32],
                &[0x22; 32],
                41,
            ))
        });
    });
}

criterion_group!(
    benches,
    // GJK per shape
    bench_gjk_sphere_sphere,
    bench_gjk_box_box,
    bench_gjk_capsule_capsule,
    bench_gjk_hyperbox_hyperbox,
    bench_gjk_hyperbox_4d,
    bench_gjk_4d_tesseract,
    // EPA
    bench_epa_sphere_sphere,
    // Raycast
    bench_raycast_100_bodies,
    // Evidence-bounded action horizon: queued, split-timescale and Unknown rejection
    bench_action_horizon_queued_feasible,
    bench_action_horizon_split_timescale,
    bench_action_horizon_unknown_rejection,
    // Immutable action-commitment issue/validation paths
    bench_action_commitment_creation,
    bench_action_commitment_validation,
    // Physics step at scale
    bench_physics_step_10_bodies,
    bench_physics_step_100_bodies,
    bench_physics_step_500_bodies,
    bench_physics_step_2d,
);
criterion_main!(benches);
