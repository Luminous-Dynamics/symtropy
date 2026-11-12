// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::{Capsule, ConvexHull, HyperBox, Point, Sphere};
use symtropy_physics::{
    FrameId, PhysicsWorld, ReferencePointId, SpatialInertia3,
};
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

// ═══ Spatial inertia reference microbenchmarks ═══

fn bench_spatial_inertia_sample() -> SpatialInertia3 {
    SpatialInertia3::new(
        FrameId(1),
        ReferencePointId(10),
        4.0,
        SVector::new(0.25, -0.1, 0.4),
        SMatrix::new(
            1.2, 0.1, 0.05,
            0.1, 1.6, 0.08,
            0.05, 0.08, 2.1,
        ),
    )
    .expect("valid benchmark spatial inertia")
}

fn bench_spatial_inertia_matrix_build(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    c.bench_function("spatial_inertia_3d_build_6x6", |bench| {
        bench.iter(|| black_box(inertia.spatial_mass_matrix().unwrap()));
    });
}

fn bench_spatial_inertia_dense_apply(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    let matrix = inertia.spatial_mass_matrix().unwrap();
    let twist = SVector::from([0.3, -0.2, 0.5, 1.1, -0.4, 0.7]);
    c.bench_function("spatial_inertia_3d_cached_6x6_apply", |bench| {
        bench.iter(|| black_box(matrix * black_box(twist)));
    });
}

fn bench_spatial_inertia_checked_apply(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    let twist = SVector::from([0.3, -0.2, 0.5, 1.1, -0.4, 0.7]);
    c.bench_function("spatial_inertia_3d_checked_twist_to_momentum", |bench| {
        bench.iter(|| black_box(inertia.spatial_momentum(&twist).unwrap()));
    });
}

fn bench_spatial_inertia_energy(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    let twist = SVector::from([0.3, -0.2, 0.5, 1.1, -0.4, 0.7]);
    c.bench_function("spatial_inertia_3d_checked_kinetic_energy", |bench| {
        bench.iter(|| black_box(inertia.kinetic_energy(&twist).unwrap()));
    });
}

fn bench_spatial_inertia_reference_shift(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    let offset = SVector::new(-0.4, 0.7, 0.2);
    c.bench_function("spatial_inertia_3d_reference_point_shift", |bench| {
        bench.iter(|| {
            black_box(
                inertia
                    .shift_reference_point(ReferencePointId(11), &offset)
                    .unwrap(),
            );
        });
    });
}

fn bench_spatial_inertia_frame_reexpression(c: &mut Criterion) {
    use symtropy_math::{Bivector, Rotor};
    let inertia = bench_spatial_inertia_sample();
    let mut generator = Bivector::<3>::zero();
    generator.set(0, 1, 0.2);
    generator.set(0, 2, -0.3);
    generator.set(1, 2, 0.1);
    let rotation = Rotor::from_bivector(&generator);
    c.bench_function("spatial_inertia_3d_frame_reexpression", |bench| {
        bench.iter(|| black_box(inertia.reexpress(FrameId(2), &rotation).unwrap()));
    });
}

fn bench_spatial_inertia_contact_effective_mass(c: &mut Criterion) {
    let inertia = bench_spatial_inertia_sample();
    let offset = SVector::new(0.7, -0.25, 0.4);
    let normal = SVector::new(-0.2, 0.9, 0.35);
    c.bench_function("spatial_inertia_3d_contact_inverse_effective_mass", |bench| {
        bench.iter(|| {
            black_box(
                inertia
                    .contact_inverse_effective_mass(&offset, &normal, 1.0e-12)
                    .unwrap(),
            );
        });
    });
}

fn bench_spatial_inertia_composition(c: &mut Criterion) {
    let first = bench_spatial_inertia_sample();
    let second = SpatialInertia3::new(
        FrameId(1),
        ReferencePointId(10),
        2.0,
        SVector::new(-0.6, 0.2, -0.1),
        SMatrix::new(
            0.7, 0.04, 0.0,
            0.04, 0.9, 0.03,
            0.0, 0.03, 1.1,
        ),
    )
    .expect("valid second benchmark inertia");
    c.bench_function("spatial_inertia_3d_compose_at_common_reference", |bench| {
        bench.iter(|| black_box(first.compose_at_common_reference(&second).unwrap()));
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
    // Physics step at scale
    bench_physics_step_10_bodies,
    bench_physics_step_100_bodies,
    bench_physics_step_500_bodies,
    bench_physics_step_2d,
    // Spatial inertia reference operations
    bench_spatial_inertia_matrix_build,
    bench_spatial_inertia_dense_apply,
    bench_spatial_inertia_checked_apply,
    bench_spatial_inertia_energy,
    bench_spatial_inertia_reference_shift,
    bench_spatial_inertia_frame_reexpression,
    bench_spatial_inertia_composition,
    bench_spatial_inertia_contact_effective_mass,
);
criterion_main!(benches);
