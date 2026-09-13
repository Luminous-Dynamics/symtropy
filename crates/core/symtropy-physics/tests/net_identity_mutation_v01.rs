// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, NetId, NetIdentityMutationError, PhysicsWorld, RigidBody,
    add_bodies_deterministic_checked, assign_net_id_checked,
};

fn body_at(x: f64) -> RigidBody<3> {
    RigidBody::dynamic_sphere(
        BodyHandle(0),
        Point::new([x, 0.0, 0.0]),
        0.5,
        1.0,
    )
}

#[test]
fn duplicate_single_assignment_fails_without_mutating_either_body() {
    let mut world = PhysicsWorld::<3>::default();
    let first = world.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
    let second = world.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
    let net_id = NetId(100);

    assign_net_id_checked(&mut world, first, net_id).expect("first assignment succeeds");
    let before_second = world.body(second).expect("second exists").net_id;

    assert_eq!(
        assign_net_id_checked(&mut world, second, net_id),
        Err(NetIdentityMutationError::NetIdAlreadyBound {
            net_id,
            existing_handle: first,
        })
    );

    assert_eq!(world.body(first).expect("first exists").net_id, Some(net_id));
    assert_eq!(world.body(second).expect("second exists").net_id, before_second);
    assert_eq!(world.handle_for_net_id(net_id), Some(first));
}

#[test]
fn idempotent_same_binding_is_success_without_identity_churn() {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let net_id = NetId(101);

    assign_net_id_checked(&mut world, handle, net_id).expect("first assignment succeeds");
    assign_net_id_checked(&mut world, handle, net_id).expect("same binding is idempotent");

    assert_eq!(world.body(handle).expect("body exists").net_id, Some(net_id));
    assert_eq!(world.handle_for_net_id(net_id), Some(handle));
    assert_eq!(world.net_id_for_handle(handle), Some(net_id));
}

#[test]
fn clean_identity_reassignment_is_forbidden_and_preserves_original_binding() {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let original = NetId(102);
    let replacement = NetId(103);

    assign_net_id_checked(&mut world, handle, original).expect("initial bind succeeds");

    assert_eq!(
        assign_net_id_checked(&mut world, handle, replacement),
        Err(NetIdentityMutationError::IdentityReassignmentForbidden {
            handle,
            current: original,
            requested: replacement,
        })
    );

    assert_eq!(world.body(handle).expect("body exists").net_id, Some(original));
    assert_eq!(world.handle_for_net_id(original), Some(handle));
    assert_eq!(world.handle_for_net_id(replacement), None);
}

#[test]
fn stale_current_index_rejects_before_reassignment() {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let indexed = NetId(110);
    let stale_body_id = NetId(111);
    let requested = NetId(112);

    world.set_net_id(handle, indexed);
    world.body_mut(handle).expect("body exists").net_id = Some(stale_body_id);

    assert_eq!(
        assign_net_id_checked(&mut world, handle, requested),
        Err(NetIdentityMutationError::CurrentIdentityIndexMissing {
            net_id: stale_body_id,
            handle,
        })
    );

    assert_eq!(world.body(handle).expect("body exists").net_id, Some(stale_body_id));
    assert_eq!(world.handle_for_net_id(indexed), Some(handle));
    assert_eq!(world.handle_for_net_id(requested), None);
}

#[test]
fn duplicate_batch_is_rejected_before_any_body_is_inserted() {
    let mut world = PhysicsWorld::<3>::default();
    let duplicate = NetId(120);
    let before_count = world.body_count();

    assert_eq!(
        add_bodies_deterministic_checked(
            &mut world,
            vec![(duplicate, body_at(0.0)), (duplicate, body_at(1.0))],
        ),
        Err(NetIdentityMutationError::DuplicateBatchNetId { net_id: duplicate })
    );

    assert_eq!(world.body_count(), before_count);
    assert_eq!(world.handle_for_net_id(duplicate), None);
}

#[test]
fn batch_collision_with_existing_world_is_rejected_before_insertion() {
    let mut world = PhysicsWorld::<3>::default();
    let existing_handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    let existing_id = NetId(130);
    assign_net_id_checked(&mut world, existing_handle, existing_id).expect("bind existing");
    let before_count = world.body_count();

    assert_eq!(
        add_bodies_deterministic_checked(
            &mut world,
            vec![(NetId(129), body_at(-1.0)), (existing_id, body_at(1.0))],
        ),
        Err(NetIdentityMutationError::NetIdAlreadyBound {
            net_id: existing_id,
            existing_handle,
        })
    );

    assert_eq!(world.body_count(), before_count);
    assert_eq!(world.handle_for_net_id(NetId(129)), None);
    assert_eq!(world.handle_for_net_id(existing_id), Some(existing_handle));
}

#[test]
fn conflicting_embedded_body_identity_rejects_before_insertion() {
    let mut world = PhysicsWorld::<3>::default();
    let requested = NetId(140);
    let embedded = NetId(141);
    let mut body = body_at(0.0);
    body.net_id = Some(embedded);

    assert_eq!(
        add_bodies_deterministic_checked(&mut world, vec![(requested, body)]),
        Err(NetIdentityMutationError::BodyCarriesConflictingNetId {
            requested,
            embedded,
        })
    );

    assert_eq!(world.body_count(), 0);
    assert_eq!(world.handle_for_net_id(requested), None);
    assert_eq!(world.handle_for_net_id(embedded), None);
}

#[test]
fn successful_batch_preserves_deterministic_net_id_ordering() {
    let mut world = PhysicsWorld::<3>::default();

    let handles = add_bodies_deterministic_checked(
        &mut world,
        vec![(NetId(152), body_at(2.0)), (NetId(151), body_at(1.0))],
    )
    .expect("preflighted batch succeeds");

    assert_eq!(handles.len(), 2);
    assert_eq!(world.handle_for_net_id(NetId(151)), Some(handles[0]));
    assert_eq!(world.handle_for_net_id(NetId(152)), Some(handles[1]));
    assert_eq!(world.net_id_for_handle(handles[0]), Some(NetId(151)));
    assert_eq!(world.net_id_for_handle(handles[1]), Some(NetId(152)));
}
