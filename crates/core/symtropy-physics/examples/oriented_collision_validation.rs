// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reproducible orientation-aware collision validation cases.
//!
//! Run with:
//! `cargo run --release --example oriented_collision_validation`
//!
//! The program emits CSV and exits unsuccessfully if any declared contract is
//! violated. It is intentionally small enough to run in CI and to reproduce in
//! independent engines or high-precision reference implementations.

use std::f64::consts::FRAC_PI_2;

use nalgebra::SVector;
use symtropy_math::{Bivector, HalfSpace, HyperBox, Point, Rotor, Sphere, Transform};
use symtropy_physics::raycast::raycast;
use symtropy_physics::support_map::{TransformedShape, support_aabb};
use symtropy_physics::{BodyHandle, BodyType, PhysicsWorld, RigidBody, gjk};

fn main() {
    println!("case,observed,expected,absolute_error,pass");

    let mut failures = 0usize;
    failures += rotated_support_aabb_case();
    failures += rotated_gjk_separation_case();
    failures += rotated_raycast_case();
    failures += transformed_halfspace_case();
    failures += oriented_box_sat_case();
    failures += static_cache_invalidation_case();

    if failures != 0 {
        eprintln!("{failures} oriented-collision validation case(s) failed");
        std::process::exit(1);
    }
}

fn report(case: &str, observed: f64, expected: f64, tolerance: f64) -> usize {
    let error = (observed - expected).abs();
    let pass = error <= tolerance;
    println!("{case},{observed:.17},{expected:.17},{error:.17},{pass}");
    usize::from(!pass)
}

fn report_bool(case: &str, observed: bool, expected: bool) -> usize {
    let pass = observed == expected;
    println!(
        "{case},{},{},{},{pass}",
        u8::from(observed),
        u8::from(expected),
        u8::from(!pass)
    );
    usize::from(!pass)
}

fn quarter_turn<const D: usize>() -> Rotor<D> {
    Rotor::from_plane_angle(&Bivector::unit_plane(0, 1), FRAC_PI_2)
}

fn rotated_support_aabb_case() -> usize {
    let shape = HyperBox::<3>::new([2.0, 0.5, 0.25]);
    let transform = Transform {
        translation: Point::new([3.0, -2.0, 0.0]),
        rotation: quarter_turn(),
    };
    let map = TransformedShape::new(&shape, &transform);
    let (min, max) = support_aabb(&map);

    // A quarter turn exchanges the local X/Y half-extents.
    report("rotated_aabb_min_x", min[0], 2.5, 1e-10)
        + report("rotated_aabb_max_x", max[0], 3.5, 1e-10)
        + report("rotated_aabb_min_y", min[1], -4.0, 1e-10)
        + report("rotated_aabb_max_y", max[1], 0.0, 1e-10)
}

fn rotated_gjk_separation_case() -> usize {
    let long_box = HyperBox::<3>::new([2.0, 0.25, 0.25]);
    let sphere = Sphere::<3>::new(Point::origin(), 0.4);
    let box_transform = Transform {
        translation: Point::origin(),
        rotation: quarter_turn(),
    };
    let sphere_transform = Transform::from_translation(Point::new([1.0, 0.0, 0.0]));

    // The rotated box extends only 0.25 along world X, so the 0.4-radius
    // sphere at X=1 is separated. Translation-only collision would report the
    // opposite result because it would incorrectly use the 2.0 local X extent.
    let separated =
        !gjk::intersects_transformed(&long_box, &box_transform, &sphere, &sphere_transform)
            .intersecting;
    report_bool("rotated_gjk_separation", separated, true)
}

fn rotated_raycast_case() -> usize {
    let mut world = PhysicsWorld::<3>::default();
    let body = RigidBody::new(
        BodyHandle(0),
        BodyType::Static,
        Transform {
            translation: Point::origin(),
            rotation: quarter_turn(),
        },
        Box::new(HyperBox::<3>::new([2.0, 0.25, 0.25])),
        0.0,
        SVector::zeros(),
    );
    world.add_body(body);

    let hit = raycast(
        &world,
        &SVector::from([-3.0, 0.0, 0.0]),
        &SVector::from([1.0, 0.0, 0.0]),
        10.0,
    )
    .expect("ray should hit the rotated box");

    report("rotated_box_raycast_distance", hit.distance, 2.75, 1e-10)
}

fn transformed_halfspace_case() -> usize {
    let mut world = PhysicsWorld::<3>::default();
    let plane = RigidBody::new(
        BodyHandle(0),
        BodyType::Static,
        Transform::from_translation(Point::new([0.0, 2.0, 0.0])),
        Box::new(HalfSpace::<3>::ground(1, 0.0)),
        0.0,
        SVector::zeros(),
    );
    let sphere = RigidBody::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform::from_translation(Point::new([0.0, 2.5, 0.0])),
        Box::new(Sphere::<3>::unit()),
        1.0,
        SVector::from_element(1.0),
    );
    world.add_body(plane);
    world.add_body(sphere);
    world.step(1.0 / 120.0);

    let depth = world
        .contacts
        .iter()
        .map(|contact| contact.depth())
        .fold(0.0_f64, f64::max);
    report("translated_halfspace_depth", depth, 0.5, 1e-10)
}

fn oriented_box_sat_case() -> usize {
    let mut world = PhysicsWorld::<3>::default();
    let rotated = RigidBody::new(
        BodyHandle(0),
        BodyType::Dynamic,
        Transform {
            translation: Point::origin(),
            rotation: quarter_turn(),
        },
        Box::new(HyperBox::<3>::new([2.0, 0.25, 0.25])),
        1.0,
        SVector::from_element(1.0),
    );
    let cube = RigidBody::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform::from_translation(Point::new([0.6, 0.0, 0.0])),
        Box::new(HyperBox::<3>::cube(0.5)),
        1.0,
        SVector::from_element(1.0),
    );
    world.add_body(rotated);
    world.add_body(cube);
    world.step(1.0 / 120.0);

    let depth = world
        .contacts
        .iter()
        .map(|contact| contact.depth())
        .fold(0.0_f64, f64::max);
    report("oriented_box_sat_depth", depth, 0.15, 1e-10)
}

fn static_cache_invalidation_case() -> usize {
    let mut world = PhysicsWorld::<3>::default();
    let static_box = RigidBody::new(
        BodyHandle(0),
        BodyType::Static,
        Transform::identity(),
        Box::new(HyperBox::<3>::new([2.0, 0.25, 0.25])),
        0.0,
        SVector::zeros(),
    );
    let probe = RigidBody::new(
        BodyHandle(1),
        BodyType::Dynamic,
        Transform::from_translation(Point::new([0.0, 1.5, 0.0])),
        Box::new(Sphere::<3>::new(Point::origin(), 0.3)),
        1.0,
        SVector::from_element(1.0),
    );
    let static_handle = world.add_body(static_box);
    world.add_body(probe);

    // Build the static cache in the original horizontal orientation.
    world.step(1.0 / 120.0);
    let initially_separated = world.contacts.is_empty();

    // Rotate through the public mutable API. This must invalidate and rebuild
    // the static broadphase bounds before the next narrowphase.
    world
        .body_mut(static_handle)
        .expect("static body inserted above")
        .transform
        .rotation = quarter_turn();
    world.step(1.0 / 120.0);
    let colliding_after_rotation = !world.contacts.is_empty();

    report_bool("static_cache_initial_separation", initially_separated, true)
        + report_bool(
            "static_cache_collision_after_rotation",
            colliding_after_rotation,
            true,
        )
}
