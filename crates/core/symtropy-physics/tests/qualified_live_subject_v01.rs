// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority, NetId,
    PhysicalAuthorityId, PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError,
    PhysicsWorld, WorldGenerationId,
};

fn qualified_world(
    net_id: NetId,
) -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    BodyHandle,
) {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = LocalNamespacePhysicsAuthorityWorld::bind(generation, raw);
    let subject = world
        .authority_world_mut()
        .bind_net_id(handle, net_id)
        .expect("identity bind should succeed");
    (world, subject, handle)
}

#[test]
fn qualified_wrapper_mints_qualified_live_subject_token() {
    let net_id = NetId(7001);
    let (world, subject, handle) = qualified_world(net_id);

    let validated = world
        .validate_subject(subject)
        .expect("qualified validation should succeed");

    assert_eq!(validated.subject(), subject);
    assert_eq!(validated.net_id(), net_id);
    assert_eq!(validated.runtime_handle(), handle);
    assert_eq!(validated.body().net_id(), Some(net_id));
}

#[test]
fn qualified_validation_preserves_physical_authority_mismatch_error() {
    let net_id = NetId(7002);
    let (world, subject, _) = qualified_world(net_id);
    let wrong_authority = PhysicalAuthorityId::new(
        world
            .physical_authority_id()
            .get()
            .checked_add(10_000)
            .expect("test authority arithmetic"),
    )
    .expect("nonzero authority");
    let wrong = PhysicsBodySubject::new(
        wrong_authority,
        subject.world_generation_id(),
        subject.net_id(),
    );

    assert!(matches!(
        world.validate_subject(wrong),
        Err(PhysicsIdentityError::PhysicalAuthorityMismatch { expected, actual })
            if expected == wrong_authority && actual == world.physical_authority_id()
    ));
}

#[test]
fn qualified_validation_preserves_generation_mismatch_error() {
    let net_id = NetId(7003);
    let (world, subject, _) = qualified_world(net_id);
    let wrong_generation = WorldGenerationId::new(
        world
            .world_generation_id()
            .get()
            .checked_add(10_000)
            .expect("test generation arithmetic"),
    )
    .expect("nonzero generation");
    let wrong = PhysicsBodySubject::new(
        subject.physical_authority_id(),
        wrong_generation,
        subject.net_id(),
    );

    assert!(matches!(
        world.validate_subject(wrong),
        Err(PhysicsIdentityError::WorldGenerationMismatch { expected, actual })
            if expected == wrong_generation && actual == world.world_generation_id()
    ));
}

#[test]
fn qualified_validation_preserves_unknown_net_id_error() {
    let (world, subject, _) = qualified_world(NetId(7004));
    let missing = NetId(7999);
    let wrong = PhysicsBodySubject::new(
        subject.physical_authority_id(),
        subject.world_generation_id(),
        missing,
    );

    assert!(matches!(
        world.validate_subject(wrong),
        Err(PhysicsIdentityError::UnknownNetId { net_id }) if net_id == missing
    ));
}

#[test]
fn equal_plain_subject_can_exist_in_legacy_and_qualified_worlds_without_equal_provenance() {
    let net_id = NetId(7005);
    let (qualified, subject, qualified_handle) = qualified_world(net_id);
    let authority = subject.physical_authority_id();
    let generation = subject.world_generation_id();

    let mut legacy_raw = PhysicsWorld::<3>::default();
    let legacy_handle = legacy_raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut legacy = PhysicsAuthorityWorld::new(authority, generation, legacy_raw);
    let legacy_subject = legacy
        .bind_net_id(legacy_handle, net_id)
        .expect("legacy bind should succeed");

    assert_eq!(legacy_subject, subject);

    let qualified_token = qualified
        .validate_subject(subject)
        .expect("qualified token should mint");
    let legacy_token = legacy
        .validate_subject(legacy_subject)
        .expect("legacy validation should succeed");

    assert_eq!(qualified_token.subject(), legacy_token.subject());
    assert_eq!(qualified_token.runtime_handle(), qualified_handle);
    assert_eq!(legacy_token.runtime_handle(), legacy_handle);
}
