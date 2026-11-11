// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{Bivector, Capsule, ConvexHull, HyperBox, Point, Sphere};
use symtropy_physics::body::{BodyHandle, RigidBody};
use symtropy_physics::mass_properties::RotationalInertiaOperator;
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

// ═══ Rotational inertia benchmarks ═══

fn bench_legacy_scalar_inertia_3d(c: &mut Criterion) {
    let body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.1, 1.0);
    let impulse = Bivector::<3>::unit_plane(0, 1);
    c.bench_function("angular_inertia_legacy_scalar_mean_3d_approximate", |bench| {
        bench.iter(|| {
            let inverse_mean = body.inv_inertia.sum() / 3.0;
            black_box(impulse.scale(inverse_mean));
        });
    });
}

fn bench_validated_principal_inertia_3d(c: &mut Criterion) {
    let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 0.1, 1.0);
    body.inertia = SVector::from([1.0, 2.0, 4.0]);
    body.inv_inertia = SVector::from([1.0, 0.5, 0.25]);
    let impulse = Bivector::<3>::from_wedge(
        &SVector::from([0.7, -0.2, 0.4]),
        &SVector::from([-0.3, 0.8, 0.1]),
    );
    c.bench_function("angular_inertia_principal_axes_world_3d", |bench| {
        bench.iter(|| black_box(body.apply_inverse_angular_inertia(&impulse)));
    });
}

fn bench_validated_bivector_operator_3d(c: &mut Criterion) {
    let operator = RotationalInertiaOperator::<3>::try_from_row_major(vec![
        4.0, 0.5, 0.0,
        0.5, 3.0, 0.25,
        0.0, 0.25, 2.0,
    ])
    .expect("benchmark inertia fixture must be SPD");
    let body = RigidBody::<3>::dynamic_sphere(BodyHandle(2), Point::origin(), 0.1, 1.0)
        .with_rotational_inertia_operator(operator);
    let impulse = Bivector::<3>::from_wedge(
        &SVector::from([0.7, -0.2, 0.4]),
        &SVector::from([-0.3, 0.8, 0.1]),
    );
    c.bench_function("angular_inertia_full_bivector_operator_3d", |bench| {
        bench.iter(|| black_box(body.apply_inverse_angular_inertia(&impulse)));
    });
}

fn bench_validated_bivector_operator_9d(c: &mut Criterion) {
    let operator = RotationalInertiaOperator::<9>::isotropic(1.0)
        .expect("9D isotropic inertia fixture must be SPD");
    let body = RigidBody::<9>::dynamic_sphere(BodyHandle(3), Point::origin(), 0.1, 1.0)
        .with_rotational_inertia_operator(operator);
    let impulse = Bivector::<9>::from_wedge(
        &SVector::from([0.7, -0.2, 0.4, 0.1, -0.5, 0.9, 0.3, 0.2, -0.1]),
        &SVector::from([-0.3, 0.8, 0.1, 0.6, 0.4, -0.7, 0.2, 0.5, 0.9]),
    );
    c.bench_function("angular_inertia_full_bivector_operator_9d_36dof", |bench| {
        bench.iter(|| black_box(body.apply_inverse_angular_inertia(&impulse)));
    });
}

fn bench_anisotropic_contact_world_step(c: &mut Criterion) {
    c.bench_function("anisotropic_contact_world_step_10_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                let operator = || {
                    RotationalInertiaOperator::<3>::try_from_row_major(vec![
                        4.0, 0.5, 0.0,
                        0.5, 3.0, 0.25,
                        0.0, 0.25, 2.0,
                    ])
                    .expect("benchmark inertia fixture must be SPD")
                };
                for index in 0..10 {
                    let x = (index % 5) as f64 * 0.8;
                    let z = (index / 5) as f64 * 0.8;
                    let body = RigidBody::<3>::dynamic_sphere(
                        BodyHandle(index),
                        Point::new([x, 0.45 + z * 0.1, z]),
                        0.5,
                        1.0,
                    )
                    .with_rotational_inertia_operator(operator());
                    world.add_body(body);
                }
                world
            },
            |mut world| {
                world.step(0.008);
                black_box(world.invariant_snapshot());
            },
            criterion::BatchSize::SmallInput,
        );
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
    // Rotational inertia: approximate baseline, 3D and general bivector operator
    bench_legacy_scalar_inertia_3d,
    bench_validated_principal_inertia_3d,
    bench_validated_bivector_operator_3d,
    bench_validated_bivector_operator_9d,
    bench_anisotropic_contact_world_step,
    // Physics step at scale
    bench_physics_step_10_bodies,
    bench_physics_step_100_bodies,
    bench_physics_step_500_bodies,
    bench_physics_step_2d,
);
criterion_main!(benches);
