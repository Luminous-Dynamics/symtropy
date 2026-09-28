// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use symtropy_math::{Point, Sphere};
use symtropy_physics::{BodyHandle, NetId, PhysicsWorld, RigidBody};

fn pair_matches<const D: usize>(
    world: &PhysicsWorld<D>,
    body_a: BodyHandle,
    body_b: BodyHandle,
) -> bool {
    world.contacts.iter().any(|contact| {
        (contact.body_a == body_a && contact.body_b == body_b)
            || (contact.body_a == body_b && contact.body_b == body_a)
    })
}

#[test]
fn deterministic_static_batch_is_visible_to_broadphase_on_next_step() {
    let mut world = PhysicsWorld::<3>::default();

    // Establish a non-empty static cache and clear the world's dirty flag via
    // a normal step. The cached body is deliberately far from the later test
    // pair so it cannot satisfy the assertion by accident.
    world.add_body(RigidBody::static_body(
        BodyHandle(usize::MAX),
        Point::new([100.0, 0.0, 0.0]),
        Box::new(Sphere::<3>::unit()),
    ));
    world.step(1.0 / 60.0);

    // A deterministic Static insertion must invalidate the cached static
    // broadphase exactly like PhysicsWorld::add_body does.
    let static_handle = world
        .add_bodies_deterministic(vec![(
            NetId(7),
            RigidBody::static_body(
                BodyHandle(usize::MAX),
                Point::origin(),
                Box::new(Sphere::<3>::unit()),
            ),
        )])
        .expect("unique deterministic NetId batch")[0];

    let dynamic_handle = world.add_sphere(Point::new([1.5, 0.0, 0.0]), 1.0, 1.0);
    world.step(1.0 / 60.0);

    assert!(
        pair_matches(&world, static_handle, dynamic_handle),
        "a static collider inserted through add_bodies_deterministic must participate on the next step"
    );
}

#[test]
fn deterministic_dynamic_batch_uses_existing_static_cache() {
    let mut world = PhysicsWorld::<3>::default();

    let static_handle = world.add_body(RigidBody::static_body(
        BodyHandle(usize::MAX),
        Point::origin(),
        Box::new(Sphere::<3>::unit()),
    ));
    world.step(1.0 / 60.0);

    // Dynamic-only deterministic insertion does not require invalidating the
    // static cache. It should still collide against the already-cached body.
    let dynamic_handle = world
        .add_bodies_deterministic(vec![(
            NetId(11),
            RigidBody::dynamic_sphere(
                BodyHandle(usize::MAX),
                Point::new([1.5, 0.0, 0.0]),
                1.0,
                1.0,
            ),
        )])
        .expect("unique deterministic NetId batch")[0];
    world.step(1.0 / 60.0);

    assert!(
        pair_matches(&world, static_handle, dynamic_handle),
        "dynamic deterministic insertion must continue using the existing static cache"
    );
}


#[test]
fn clear_scene_removes_stale_static_broadphase_entries() {
    let mut world = PhysicsWorld::<3>::default();

    let static_handle = world.add_body(RigidBody::static_body(
        BodyHandle(usize::MAX),
        Point::origin(),
        Box::new(Sphere::<3>::unit()),
    ));
    world.step(1.0 / 60.0);

    // The static cache is now populated and clean. Clearing the scene must
    // remove that cached collider rather than merely clearing authoritative
    // body storage.
    world.clear_scene().expect("valid world clears successfully");

    // Reusing the same numeric handle is impossible because next_handle is
    // monotonic, so this contact can only come from the newly-created bodies.
    let dynamic_handle = world.add_sphere(Point::new([1.5, 0.0, 0.0]), 1.0, 1.0);
    world.step(1.0 / 60.0);

    assert!(
        !world
            .contacts
            .iter()
            .any(|contact| contact.body_a == dynamic_handle || contact.body_b == dynamic_handle),
        "clear_scene must not leave a stale static collider participating in later steps"
    );
    assert!(
        world.body(static_handle).is_none(),
        "the pre-clear static body must not survive scene teardown"
    );
}
