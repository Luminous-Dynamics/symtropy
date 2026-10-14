// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use symtropy_math::Point;
use symtropy_physics::{BodyHandle, NetId, PhysicsWorld, RigidBody};

fn dynamic_body(x: f64) -> RigidBody<3> {
    RigidBody::dynamic_sphere(BodyHandle(usize::MAX), Point::new([x, 0.0, 0.0]), 0.5, 1.0)
}

#[test]
fn duplicate_inside_batch_is_transactional() {
    let mut world = PhysicsWorld::<3>::default();

    let result = world.add_bodies_deterministic(vec![
        (NetId(7), dynamic_body(0.0)),
        (NetId(7), dynamic_body(2.0)),
    ]);

    assert!(result.is_err());
    assert_eq!(world.body_count(), 0, "failed batch must add no bodies");
    assert_eq!(
        world.handle_for_net_id(NetId(7)),
        None,
        "failed batch must add no NetId mapping"
    );

    let next = world.add_sphere(Point::new([10.0, 0.0, 0.0]), 0.5, 1.0);
    assert_eq!(
        next,
        BodyHandle(0),
        "failed batch must not consume body handles"
    );
}

#[test]
fn duplicate_against_existing_body_leaves_entire_batch_unapplied() {
    let mut world = PhysicsWorld::<3>::default();
    let inserted = world
        .add_bodies_deterministic(vec![(NetId(20), dynamic_body(0.0))])
        .expect("initial deterministic insertion should succeed");
    assert_eq!(inserted, vec![BodyHandle(0)]);

    let result = world.add_bodies_deterministic(vec![
        (NetId(10), dynamic_body(2.0)),
        (NetId(20), dynamic_body(4.0)),
    ]);

    assert!(result.is_err());
    assert_eq!(world.body_count(), 1, "failed batch must commit no prefix");
    assert_eq!(world.handle_for_net_id(NetId(10)), None);
    assert_eq!(world.handle_for_net_id(NetId(20)), Some(BodyHandle(0)));
    assert_eq!(world.net_id_for_handle(BodyHandle(0)), Some(NetId(20)));

    let next = world.add_sphere(Point::new([10.0, 0.0, 0.0]), 0.5, 1.0);
    assert_eq!(
        next,
        BodyHandle(1),
        "failed batch must leave handle allocation unchanged"
    );
}

#[test]
fn deterministic_batch_assigns_handles_in_sorted_netid_order() {
    let mut world = PhysicsWorld::<3>::default();

    let handles = world
        .add_bodies_deterministic(vec![
            (NetId(30), dynamic_body(3.0)),
            (NetId(10), dynamic_body(1.0)),
            (NetId(20), dynamic_body(2.0)),
        ])
        .expect("unique NetIds should insert successfully");

    assert_eq!(
        handles,
        vec![BodyHandle(0), BodyHandle(1), BodyHandle(2)],
        "returned handles should follow sorted NetId insertion order"
    );
    assert_eq!(world.handle_for_net_id(NetId(10)), Some(BodyHandle(0)));
    assert_eq!(world.handle_for_net_id(NetId(20)), Some(BodyHandle(1)));
    assert_eq!(world.handle_for_net_id(NetId(30)), Some(BodyHandle(2)));
    assert_eq!(world.net_id_for_handle(BodyHandle(0)), Some(NetId(10)));
    assert_eq!(world.net_id_for_handle(BodyHandle(1)), Some(NetId(20)));
    assert_eq!(world.net_id_for_handle(BodyHandle(2)), Some(NetId(30)));
}

#[test]
fn assigning_another_bodys_netid_cannot_steal_the_mapping() {
    let mut world = PhysicsWorld::<3>::default();
    let a = world.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
    let b = world.add_sphere(Point::new([2.0, 0.0, 0.0]), 0.5, 1.0);

    world.set_net_id(a, NetId(100));
    world.set_net_id(b, NetId(200));

    world.set_net_id(b, NetId(100));

    assert_eq!(
        world.handle_for_net_id(NetId(100)),
        Some(a),
        "a rejected reassignment must preserve the original owner"
    );
    assert_eq!(world.handle_for_net_id(NetId(200)), Some(b));
    assert_eq!(world.net_id_for_handle(a), Some(NetId(100)));
    assert_eq!(
        world.net_id_for_handle(b),
        Some(NetId(200)),
        "a rejected reassignment must preserve the target body's old NetId"
    );
}

#[test]
fn assigning_same_netid_to_same_body_is_idempotent() {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);

    world.set_net_id(handle, NetId(42));
    world.set_net_id(handle, NetId(42));

    assert_eq!(world.handle_for_net_id(NetId(42)), Some(handle));
    assert_eq!(world.net_id_for_handle(handle), Some(NetId(42)));
}

#[test]
fn changing_to_free_netid_removes_exactly_the_old_mapping() {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);

    world.set_net_id(handle, NetId(1));
    world.set_net_id(handle, NetId(2));

    assert_eq!(world.handle_for_net_id(NetId(1)), None);
    assert_eq!(world.handle_for_net_id(NetId(2)), Some(handle));
    assert_eq!(world.net_id_for_handle(handle), Some(NetId(2)));
}
