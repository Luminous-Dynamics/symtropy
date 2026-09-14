// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    NetId, PhysicalAuthorityId, PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError,
    PhysicsWorld, WorldGenerationId,
};

fn authority(value: u128) -> PhysicalAuthorityId {
    PhysicalAuthorityId::new(value).expect("nonzero authority id")
}

fn generation(value: u128) -> WorldGenerationId {
    WorldGenerationId::new(value).expect("nonzero generation id")
}

fn world_with_body(net_id: NetId) -> PhysicsWorld<3> {
    let mut world = PhysicsWorld::<3>::default();
    let handle = world.add_sphere(Point::origin(), 0.5, 1.0);
    world.set_net_id(handle, net_id);
    world
}

#[test]
fn exact_authority_generation_and_net_id_resolve_one_live_body() {
    let net_id = NetId(41);
    let authority = authority(1001);
    let generation = generation(7);
    let world = PhysicsAuthorityWorld::new(authority, generation, world_with_body(net_id));
    let subject = PhysicsBodySubject::new(authority, generation, net_id);

    let validated = world.validate_subject(subject).expect("exact subject resolves");

    assert_eq!(validated.subject(), subject);
    assert_eq!(validated.net_id(), net_id);
    assert_eq!(validated.body().net_id(), Some(net_id));
    assert_eq!(validated.body().handle, validated.runtime_handle());
}

#[test]
fn same_net_id_in_different_generation_cannot_satisfy_original_subject() {
    let net_id = NetId(42);
    let authority = authority(1002);
    let original_generation = generation(10);
    let later_generation = generation(11);
    let later_world = PhysicsAuthorityWorld::new(
        authority,
        later_generation,
        world_with_body(net_id),
    );
    let original_subject = PhysicsBodySubject::new(authority, original_generation, net_id);

    assert!(matches!(
        later_world.validate_subject(original_subject),
        Err(PhysicsIdentityError::WorldGenerationMismatch { expected, actual })
            if expected == original_generation && actual == later_generation
    ));
}

#[test]
fn detached_authority_claim_cannot_be_applied_to_another_authority_world() {
    let net_id = NetId(43);
    let claimed_authority = authority(2001);
    let actual_authority = authority(2002);
    let generation = generation(1);
    let world = PhysicsAuthorityWorld::new(
        actual_authority,
        generation,
        world_with_body(net_id),
    );
    let subject = PhysicsBodySubject::new(claimed_authority, generation, net_id);

    assert!(matches!(
        world.validate_subject(subject),
        Err(PhysicsIdentityError::PhysicalAuthorityMismatch { expected, actual })
            if expected == claimed_authority && actual == actual_authority
    ));
}

#[test]
fn duplicate_live_net_id_fails_closed_even_if_index_points_to_one_body() {
    let net_id = NetId(44);
    let authority = authority(3001);
    let generation = generation(1);
    let mut raw = PhysicsWorld::<3>::default();
    let first = raw.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
    let second = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
    raw.set_net_id(first, net_id);
    raw.set_net_id(second, net_id);

    let world = PhysicsAuthorityWorld::new(authority, generation, raw);
    let subject = PhysicsBodySubject::new(authority, generation, net_id);

    assert!(matches!(
        world.validate_subject(subject),
        Err(PhysicsIdentityError::AmbiguousNetId { net_id: observed }) if observed == net_id
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
