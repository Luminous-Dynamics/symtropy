// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, NetId, NetIdentityMutationError, PhysicalAuthorityId, PhysicsAuthorityWorld,
    PhysicsBodySubject, PhysicsIdentityError, PhysicsWorld, RigidBody, WorldGenerationId,
};

fn authority(value: u128) -> PhysicalAuthorityId {
    PhysicalAuthorityId::new(value).expect("nonzero authority id")
}

fn generation(value: u128) -> WorldGenerationId {
    WorldGenerationId::new(value).expect("nonzero generation id")
}

fn body_at(x: f64) -> RigidBody<3> {
    RigidBody::dynamic_sphere(BodyHandle(0), Point::new([x, 0.0, 0.0]), 0.5, 1.0)
}

#[test]
fn authority_owned_bind_returns_exact_subject_and_validates() {
    let net_id = NetId(41);
    let authority = authority(1001);
    let generation = generation(7);
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority, generation, raw);

    let subject = world
        .bind_net_id(handle, net_id)
        .expect("authority-owned bind succeeds");

    assert_eq!(subject.physical_authority_id(), authority);
    assert_eq!(subject.world_generation_id(), generation);
    assert_eq!(subject.net_id(), net_id);

    let validated = world.validate_subject(subject).expect("exact subject resolves");
    assert_eq!(validated.subject(), subject);
    assert_eq!(validated.net_id(), net_id);
    assert_eq!(validated.body().net_id(), Some(net_id));
    assert_eq!(validated.runtime_handle(), handle);
}

#[test]
fn idempotent_authority_bind_returns_the_same_subject() {
    let net_id = NetId(42);
    let authority = authority(1002);
    let generation = generation(8);
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority, generation, raw);

    let first = world.bind_net_id(handle, net_id).expect("first bind succeeds");
    let second = world
        .bind_net_id(handle, net_id)
        .expect("same binding is idempotent");

    assert_eq!(first, second);
    assert_eq!(world.world().handle_for_net_id(net_id), Some(handle));
}

#[test]
fn authority_owned_reassignment_is_rejected_and_preserves_original_subject() {
    let original = NetId(43);
    let replacement = NetId(44);
    let authority = authority(1003);
    let generation = generation(9);
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority, generation, raw);

    let subject = world
        .bind_net_id(handle, original)
        .expect("initial bind succeeds");

    assert_eq!(
        world.bind_net_id(handle, replacement),
        Err(NetIdentityMutationError::IdentityReassignmentForbidden {
            handle,
            current: original,
            requested: replacement,
        })
    );

    assert_eq!(world.validate_subject(subject).unwrap().runtime_handle(), handle);
    assert_eq!(world.world().handle_for_net_id(replacement), None);
}

#[test]
fn same_net_id_in_different_generation_produces_distinct_subjects() {
    let net_id = NetId(45);
    let authority = authority(1004);
    let original_generation = generation(10);
    let later_generation = generation(11);

    let mut original_raw = PhysicsWorld::<3>::default();
    let original_handle = original_raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut original_world =
        PhysicsAuthorityWorld::new(authority, original_generation, original_raw);
    let original_subject = original_world
        .bind_net_id(original_handle, net_id)
        .expect("original generation bind succeeds");

    let mut later_raw = PhysicsWorld::<3>::default();
    let later_handle = later_raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut later_world = PhysicsAuthorityWorld::new(authority, later_generation, later_raw);
    let later_subject = later_world
        .bind_net_id(later_handle, net_id)
        .expect("later generation bind succeeds");

    assert_ne!(original_subject, later_subject);
    assert_eq!(
        later_world
            .validate_subject(later_subject)
            .unwrap()
            .runtime_handle(),
        later_handle
    );
    assert!(matches!(
        later_world.validate_subject(original_subject),
        Err(PhysicsIdentityError::WorldGenerationMismatch { expected, actual })
            if expected == original_generation && actual == later_generation
    ));
}

#[test]
fn same_net_id_under_different_authority_produces_distinct_subjects() {
    let net_id = NetId(46);
    let claimed_authority = authority(2001);
    let actual_authority = authority(2002);
    let generation = generation(1);

    let mut claimed_raw = PhysicsWorld::<3>::default();
    let claimed_handle = claimed_raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut claimed_world =
        PhysicsAuthorityWorld::new(claimed_authority, generation, claimed_raw);
    let claimed_subject = claimed_world
        .bind_net_id(claimed_handle, net_id)
        .expect("claimed authority bind succeeds");

    let mut actual_raw = PhysicsWorld::<3>::default();
    let actual_handle = actual_raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut actual_world = PhysicsAuthorityWorld::new(actual_authority, generation, actual_raw);
    let actual_subject = actual_world
        .bind_net_id(actual_handle, net_id)
        .expect("actual authority bind succeeds");

    assert_ne!(claimed_subject, actual_subject);
    assert!(matches!(
        actual_world.validate_subject(claimed_subject),
        Err(PhysicsIdentityError::PhysicalAuthorityMismatch { expected, actual })
            if expected == claimed_authority && actual == actual_authority
    ));
}

#[test]
fn authority_owned_deterministic_batch_returns_subjects_in_stable_net_id_order() {
    let authority = authority(3001);
    let generation = generation(3);
    let mut world = PhysicsAuthorityWorld::new(
        authority,
        generation,
        PhysicsWorld::<3>::default(),
    );

    let bindings = world
        .add_bodies_deterministic(vec![
            (NetId(152), body_at(2.0)),
            (NetId(151), body_at(1.0)),
        ])
        .expect("checked deterministic batch succeeds");

    assert_eq!(bindings.len(), 2);
    assert_eq!(bindings[0].1.net_id(), NetId(151));
    assert_eq!(bindings[1].1.net_id(), NetId(152));

    for (handle, subject) in bindings {
        assert_eq!(subject.physical_authority_id(), authority);
        assert_eq!(subject.world_generation_id(), generation);
        assert_eq!(
            world.validate_subject(subject).unwrap().runtime_handle(),
            handle
        );
    }
}

#[test]
fn authority_owned_batch_duplicate_fails_before_any_insertion() {
    let authority = authority(3002);
    let generation = generation(4);
    let duplicate = NetId(160);
    let mut world = PhysicsAuthorityWorld::new(
        authority,
        generation,
        PhysicsWorld::<3>::default(),
    );

    assert_eq!(
        world.add_bodies_deterministic(vec![
            (duplicate, body_at(0.0)),
            (duplicate, body_at(1.0)),
        ]),
        Err(NetIdentityMutationError::DuplicateBatchNetId { net_id: duplicate })
    );

    assert_eq!(world.world().body_count(), 0);
    assert_eq!(world.world().handle_for_net_id(duplicate), None);
}

#[test]
fn detached_subject_constructor_remains_a_claim_not_a_capability() {
    let net_id = NetId(170);
    let actual_authority = authority(4001);
    let claimed_authority = authority(4002);
    let generation = generation(1);
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(actual_authority, generation, raw);
    world
        .bind_net_id(handle, net_id)
        .expect("actual authority bind succeeds");

    let detached_claim = PhysicsBodySubject::new(claimed_authority, generation, net_id);
    assert!(matches!(
        world.validate_subject(detached_claim),
        Err(PhysicsIdentityError::PhysicalAuthorityMismatch { expected, actual })
            if expected == claimed_authority && actual == actual_authority
    ));
}

#[test]
fn identity_roots_must_be_nonzero() {
    assert_eq!(
        PhysicalAuthorityId::new(0),
        Err(PhysicsIdentityError::ZeroPhysicalAuthorityId)
    );
    assert_eq!(
        WorldGenerationId::new(0),
        Err(PhysicsIdentityError::ZeroWorldGenerationId)
    );
}
