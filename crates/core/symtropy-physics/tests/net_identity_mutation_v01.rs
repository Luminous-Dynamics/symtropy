// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, NetId, NetIdentityMutationError, PhysicalAuthorityId, PhysicsAuthorityWorld,
    PhysicsWorld, RigidBody, WorldGenerationId,
};

fn body_at(x: f64) -> RigidBody<3> {
    RigidBody::dynamic_sphere(
        BodyHandle(0),
        Point::new([x, 0.0, 0.0]),
        0.5,
        1.0,
    )
}

fn authority_world() -> PhysicsAuthorityWorld<3> {
    PhysicsAuthorityWorld::new(
        PhysicalAuthorityId::new(91001).expect("nonzero authority"),
        WorldGenerationId::new(1).expect("nonzero generation"),
        PhysicsWorld::default(),
    )
}

#[test]
fn duplicate_single_assignment_fails_without_mutating_either_body() {
    let mut authority = authority_world();
    let first = authority
        .world_mut()
        .add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
    let second = authority
        .world_mut()
        .add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
    let net_id = NetId(100);

    authority
        .bind_net_id(first, net_id)
        .expect("first assignment succeeds");
    let before_second = authority
        .world()
        .body(second)
        .expect("second exists")
        .net_id();

    assert_eq!(
        authority.bind_net_id(second, net_id),
        Err(NetIdentityMutationError::NetIdAlreadyBound {
            net_id,
            existing_handle: first,
        })
    );

    assert_eq!(authority.world().body(first).expect("first exists").net_id(), Some(net_id));
    assert_eq!(authority.world().body(second).expect("second exists").net_id(), before_second);
    assert_eq!(authority.world().handle_for_net_id(net_id), Some(first));
}

#[test]
fn idempotent_same_binding_is_success_without_identity_churn() {
    let mut authority = authority_world();
    let handle = authority.world_mut().add_sphere(Point::origin(), 0.5, 1.0);
    let net_id = NetId(101);

    let first = authority
        .bind_net_id(handle, net_id)
        .expect("first assignment succeeds");
    let second = authority
        .bind_net_id(handle, net_id)
        .expect("same binding is idempotent");

    assert_eq!(first, second);
    assert_eq!(authority.world().body(handle).expect("body exists").net_id(), Some(net_id));
    assert_eq!(authority.world().handle_for_net_id(net_id), Some(handle));
    assert_eq!(authority.world().net_id_for_handle(handle), Some(net_id));
}

#[test]
fn clean_identity_reassignment_is_forbidden_and_preserves_original_binding() {
    let mut authority = authority_world();
    let handle = authority.world_mut().add_sphere(Point::origin(), 0.5, 1.0);
    let original = NetId(102);
    let replacement = NetId(103);

    authority
        .bind_net_id(handle, original)
        .expect("initial bind succeeds");

    assert_eq!(
        authority.bind_net_id(handle, replacement),
        Err(NetIdentityMutationError::IdentityReassignmentForbidden {
            handle,
            current: original,
            requested: replacement,
        })
    );

    assert_eq!(authority.world().body(handle).expect("body exists").net_id(), Some(original));
    assert_eq!(authority.world().handle_for_net_id(original), Some(handle));
    assert_eq!(authority.world().handle_for_net_id(replacement), None);
}

#[test]
fn duplicate_batch_is_rejected_before_any_body_is_inserted() {
    let mut authority = authority_world();
    let duplicate = NetId(120);
    let before_count = authority.world().body_count();

    assert_eq!(
        authority.add_bodies_deterministic(vec![
            (duplicate, body_at(0.0)),
            (duplicate, body_at(1.0)),
        ]),
        Err(NetIdentityMutationError::DuplicateBatchNetId { net_id: duplicate })
    );

    assert_eq!(authority.world().body_count(), before_count);
    assert_eq!(authority.world().handle_for_net_id(duplicate), None);
}

#[test]
fn batch_collision_with_existing_world_is_rejected_before_insertion() {
    let mut authority = authority_world();
    let existing_handle = authority.world_mut().add_sphere(Point::origin(), 0.5, 1.0);
    let existing_id = NetId(130);
    authority
        .bind_net_id(existing_handle, existing_id)
        .expect("bind existing");
    let before_count = authority.world().body_count();

    assert_eq!(
        authority.add_bodies_deterministic(vec![
            (NetId(129), body_at(-1.0)),
            (existing_id, body_at(1.0)),
        ]),
        Err(NetIdentityMutationError::NetIdAlreadyBound {
            net_id: existing_id,
            existing_handle,
        })
    );

    assert_eq!(authority.world().body_count(), before_count);
    assert_eq!(authority.world().handle_for_net_id(NetId(129)), None);
    assert_eq!(authority.world().handle_for_net_id(existing_id), Some(existing_handle));
}

#[test]
fn successful_batch_preserves_deterministic_net_id_ordering() {
    let mut authority = authority_world();

    let inserted = authority
        .add_bodies_deterministic(vec![
            (NetId(152), body_at(2.0)),
            (NetId(151), body_at(1.0)),
        ])
        .expect("preflighted batch succeeds");

    assert_eq!(inserted.len(), 2);
    let first_handle = inserted[0].0;
    let second_handle = inserted[1].0;
    assert_eq!(inserted[0].1.net_id(), NetId(151));
    assert_eq!(inserted[1].1.net_id(), NetId(152));
    assert_eq!(authority.world().handle_for_net_id(NetId(151)), Some(first_handle));
    assert_eq!(authority.world().handle_for_net_id(NetId(152)), Some(second_handle));
    assert_eq!(authority.world().net_id_for_handle(first_handle), Some(NetId(151)));
    assert_eq!(authority.world().net_id_for_handle(second_handle), Some(NetId(152)));
}
