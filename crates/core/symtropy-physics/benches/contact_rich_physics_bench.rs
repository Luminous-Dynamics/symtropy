// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Contact-rich rigid-body workloads.
//!
//! Existing step benchmarks mostly measure integration and broadphase because
//! their bodies are widely separated. These workloads deliberately produce
//! resting contacts and multi-body contact graphs. The no-contact case is a
//! control. This establishes a reproducible Symtropy baseline that can later
//! be matched to Rapier, MuJoCo and Box2D where collision semantics agree.

use criterion::{
    BatchSize, BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use nalgebra::SVector;
use symtropy_math::{HalfSpace, HyperBox, Point, Transform};
use symtropy_physics::{BodyHandle, BodyType, PhysicsWorld, RigidBody};

const STEPS: usize = 120;
const DT: f64 = 1.0 / 120.0;
const TALL_STACK_STEPS: usize = 1800;
const TALL_STACK_DT: f64 = 1.0 / 60.0;
const TALL_STACK_BODIES: usize = 10;
const STACK_BODIES: usize = 16;
const BED_SIDE: usize = 8;
const SPHERE_RADIUS: f64 = 0.5;
const CONTACT_OVERLAP: f64 = 0.005;

#[derive(Clone, Copy)]
enum Scene {
    NoContact64,
    SphereStack16,
    BoxStack3,
    TenBoxStack8Iterations,
    TenBoxStack30Iterations,
    SphereBed8x8,
}

impl Scene {
    fn name(self) -> &'static str {
        match self {
            Self::NoContact64 => "no_contact_64_spheres",
            Self::SphereStack16 => "resting_sphere_stack_16",
            Self::BoxStack3 => "multi_point_box_stack_3",
            Self::TenBoxStack8Iterations => "known_unqualified_ten_box_stack_8_iterations",
            Self::TenBoxStack30Iterations => "known_unqualified_ten_box_stack_30_iterations",
            Self::SphereBed8x8 => "dense_sphere_bed_8x8",
        }
    }

    fn body_count(self) -> usize {
        match self {
            Self::NoContact64 => 64,
            Self::SphereStack16 => STACK_BODIES,
            Self::BoxStack3 => 3,
            Self::TenBoxStack8Iterations | Self::TenBoxStack30Iterations => TALL_STACK_BODIES,
            Self::SphereBed8x8 => BED_SIDE * BED_SIDE,
        }
    }

    fn has_contacts(self) -> bool {
        !matches!(self, Self::NoContact64)
    }

    fn steps(self) -> usize {
        match self {
            Self::TenBoxStack8Iterations | Self::TenBoxStack30Iterations => TALL_STACK_STEPS,
            _ => STEPS,
        }
    }

    fn timestep_seconds(self) -> f64 {
        match self {
            Self::TenBoxStack8Iterations | Self::TenBoxStack30Iterations => TALL_STACK_DT,
            _ => DT,
        }
    }

    fn solver_iterations(self) -> usize {
        match self {
            Self::TenBoxStack30Iterations => 30,
            _ => 8,
        }
    }

    fn is_tall_box_stack(self) -> bool {
        matches!(
            self,
            Self::TenBoxStack8Iterations | Self::TenBoxStack30Iterations
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct CampaignStats {
    body_steps: u64,
    contact_manifold_steps: u64,
    contact_point_steps: u64,
    collision_events: u64,
    peak_penetration_m: f64,
    max_linear_speed_m_per_s: f64,
    non_finite_body_count: usize,
    final_stack_max_position_error_m: Option<f64>,
}

fn new_world(gravity_y: f64) -> PhysicsWorld<3> {
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, gravity_y, 0.0]));
    world.solver_iterations = 8;
    world.sleep_threshold = 0.0;
    world.sleep_ticks = u32::MAX;
    world
}

fn add_ground(world: &mut PhysicsWorld<3>) {
    let ground = RigidBody::<3>::static_body(
        // PhysicsWorld::add_body assigns its own stable handle.
        BodyHandle(0),
        Point::origin(),
        Box::new(HalfSpace::<3>::ground(1, 0.0)),
    );
    world.add_body(ground);
}

fn build_scene(scene: Scene) -> PhysicsWorld<3> {
    match scene {
        Scene::NoContact64 => {
            let mut world = new_world(0.0);
            for index in 0..64 {
                let x = (index % 8) as f64 * 2.5;
                let z = (index / 8) as f64 * 2.5;
                world.add_sphere(Point::new([x, 10.0, z]), SPHERE_RADIUS, 1.0);
            }
            world
        }
        Scene::SphereStack16 => {
            let mut world = new_world(-9.81);
            add_ground(&mut world);
            let spacing = 2.0 * SPHERE_RADIUS - CONTACT_OVERLAP;
            for index in 0..STACK_BODIES {
                world.add_sphere(
                    Point::new([
                        0.0,
                        SPHERE_RADIUS - CONTACT_OVERLAP + index as f64 * spacing,
                        0.0,
                    ]),
                    SPHERE_RADIUS,
                    1.0,
                );
            }
            world
        }
        Scene::BoxStack3 => {
            let mut world = new_world(-9.81);
            add_ground(&mut world);
            let side = 1.0;
            let mass = 1.0;
            let spacing = side - CONTACT_OVERLAP;
            for index in 0..3 {
                let body = RigidBody::new(
                    BodyHandle(0),
                    BodyType::Dynamic,
                    Transform::from_translation(Point::new([
                        0.0,
                        0.5 + index as f64 * spacing,
                        0.0,
                    ])),
                    Box::new(HyperBox::<3>::new([0.5, 0.5, 0.5])),
                    mass,
                    SVector::from_element(mass * side * side / 6.0),
                );
                world.add_body(body);
            }
            world
        }
        Scene::TenBoxStack8Iterations | Scene::TenBoxStack30Iterations => {
            let mut world = new_world(-9.81);
            world.solver_iterations = scene.solver_iterations();
            add_ground(&mut world);
            let side = 1.0;
            let mass = 1.0;
            let spacing = side - 0.01;
            for index in 0..TALL_STACK_BODIES {
                let body = RigidBody::new(
                    BodyHandle(0),
                    BodyType::Dynamic,
                    Transform::from_translation(Point::new([
                        0.0,
                        0.5 + index as f64 * spacing,
                        0.0,
                    ])),
                    Box::new(HyperBox::<3>::new([0.5, 0.5, 0.5])),
                    mass,
                    SVector::from_element(mass * side * side / 6.0),
                );
                world.add_body(body);
            }
            world
        }
        Scene::SphereBed8x8 => {
            let mut world = new_world(-9.81);
            add_ground(&mut world);
            let spacing = 2.0 * SPHERE_RADIUS - CONTACT_OVERLAP;
            let origin = -0.5 * (BED_SIDE - 1) as f64 * spacing;
            for row in 0..BED_SIDE {
                for column in 0..BED_SIDE {
                    world.add_sphere(
                        Point::new([
                            origin + column as f64 * spacing,
                            SPHERE_RADIUS - CONTACT_OVERLAP,
                            origin + row as f64 * spacing,
                        ]),
                        SPHERE_RADIUS,
                        1.0,
                    );
                }
            }
            world
        }
    }
}

fn run_campaign(mut world: PhysicsWorld<3>, scene: Scene) -> CampaignStats {
    let mut stats = CampaignStats {
        body_steps: 0,
        contact_manifold_steps: 0,
        contact_point_steps: 0,
        collision_events: 0,
        peak_penetration_m: 0.0,
        max_linear_speed_m_per_s: 0.0,
        non_finite_body_count: 0,
        final_stack_max_position_error_m: None,
    };

    for _ in 0..scene.steps() {
        world.step(scene.timestep_seconds());
        stats.body_steps += world.body_count() as u64;
        stats.contact_manifold_steps += world.contacts.len() as u64;
        stats.contact_point_steps += world
            .contacts
            .iter()
            .map(|manifold| manifold.points.len() as u64)
            .sum::<u64>();
        stats.collision_events += world.collision_events.len() as u64;
        stats.peak_penetration_m = stats.peak_penetration_m.max(
            world
                .contacts
                .iter()
                .map(|manifold| manifold.depth())
                .fold(0.0, f64::max),
        );

        let invariants = world.invariant_snapshot();
        stats.max_linear_speed_m_per_s = stats
            .max_linear_speed_m_per_s
            .max(invariants.max_linear_speed);
        stats.non_finite_body_count += invariants.non_finite_body_count;
        assert!(
            invariants.is_numerically_healthy(1e-8),
            "{} produced a numerically unhealthy state",
            scene.name()
        );
    }

    if scene.has_contacts() {
        assert!(
            stats.contact_point_steps > 0,
            "{} did not exercise the contact solver",
            scene.name()
        );
    } else {
        assert_eq!(
            stats.contact_point_steps, 0,
            "no-contact control unexpectedly generated contact points"
        );
    }
    assert_eq!(stats.non_finite_body_count, 0);
    assert!(stats.max_linear_speed_m_per_s.is_finite());

    if scene.is_tall_box_stack() {
        // Ground occupies the first body slot. The stack's target center
        // heights are 0.5, 1.5, ... 9.5 m. This is a known unqualified stress
        // case, so measure final residual instead of asserting a false pass.
        let max_error = world
            .bodies
            .iter()
            .skip(1)
            .take(TALL_STACK_BODIES)
            .enumerate()
            .map(|(index, body)| {
                let expected_height = 0.5 + index as f64;
                (body.position()[1] - expected_height).abs()
            })
            .fold(0.0, f64::max);
        assert!(max_error.is_finite());
        stats.final_stack_max_position_error_m = Some(max_error);
    }

    stats
}

fn bench_contact_rich_scenes(c: &mut Criterion) {
    let mut group = c.benchmark_group("contact_rich_physics");
    for scene in [
        Scene::NoContact64,
        Scene::SphereStack16,
        Scene::BoxStack3,
        Scene::TenBoxStack8Iterations,
        Scene::TenBoxStack30Iterations,
        Scene::SphereBed8x8,
    ] {
        // Untimed calibration captures actual contact complexity for the run
        // and embeds it in the benchmark ID. This prevents a scene that
        // unexpectedly loses all contacts from looking like a speed win.
        let calibration = run_campaign(build_scene(scene), scene);
        let steps = scene.steps();
        let average_points_per_step = calibration.contact_point_steps as f64 / steps as f64;
        let average_manifolds_per_step = calibration.contact_manifold_steps as f64 / steps as f64;
        let stack_error_label = calibration
            .final_stack_max_position_error_m
            .map(|error| format!("{:.1}mm_stack_error", error * 1000.0))
            .unwrap_or_else(|| "no_stack_error_metric".to_owned());
        let benchmark_name = format!(
            "{}_{}hz_{}bodies_{}iters_{:.1}manifolds_{:.1}points_{:.3}m_pen_{}events_{}",
            scene.name(),
            (1.0 / scene.timestep_seconds()).round() as usize,
            scene.body_count(),
            scene.solver_iterations(),
            average_manifolds_per_step,
            average_points_per_step,
            calibration.peak_penetration_m,
            calibration.collision_events,
            stack_error_label
        );

        // Same throughput unit across all scenes: body-steps. Contact
        // complexity is independently recorded in the benchmark name.
        group.throughput(Throughput::Elements(calibration.body_steps));
        group.bench_function(BenchmarkId::new(benchmark_name, steps), |bench| {
            bench.iter_batched(
                || build_scene(scene),
                |world| black_box(run_campaign(world, scene)),
                BatchSize::LargeInput,
            );
        });

        // Keep the calibration output observable for future result receipts.
        black_box(calibration);
    }
    group.finish();
}

criterion_group!(benches, bench_contact_rich_scenes);
criterion_main!(benches);
