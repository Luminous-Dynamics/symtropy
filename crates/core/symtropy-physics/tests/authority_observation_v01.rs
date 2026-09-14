// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    AuthorityBodySnapshot, NetId, PhysicalAuthorityId, PhysicsAuthorityWorld, PhysicsBodySubject,
    PhysicsIdentityError, PhysicsWorld, WorldGenerationId,
};

fn ids() -> (PhysicalAuthorityId, WorldGenerationId, NetId) {
    (
        PhysicalAuthorityId::new(0xA11CE).unwrap(),
        WorldGenerationId::new(7).unwrap(),
        NetId(9001),
    )
}

fn build_authority_world(
    insert_dummy_first: bool,
) -> (PhysicsAuthorityWorld<3>, PhysicsBodySubject) {
    let (authority_id, generation_id, net_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();

    if insert_dummy_first {
        raw.add_sphere(Point::new([-100.0, 0.0, 0.0]), 0.25, 1.0);
    }

    let target = raw.add_sphere(Point::new([1.0, 2.0, 3.0]), 0.5, 2.0);
    raw.body_mut(target).unwrap().linear_velocity = SVector::from([4.0, 5.0, 6.0]);

    let mut authority = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let subject = authority.bind_net_id(target, net_id).unwrap();
    (authority, subject)
}

#[test]
fn equivalent_subject_state_is_handle_independent() {
    let (world_a, subject_a) = build_authority_world(false);
    let (world_b, subject_b) = build_authority_world(true);

    {
        let validated_a = world_a.validate_subject(subject_a).unwrap();
        let validated_b = world_b.validate_subject(subject_b).unwrap();
        assert_ne!(validated_a.runtime_handle(), validated_b.runtime_handle());
    }

    let snapshot_a = AuthorityBodySnapshot::capture(&world_a, subject_a).unwrap();
    let snapshot_b = AuthorityBodySnapshot::capture(&world_b, subject_b).unwrap();

    assert_eq!(snapshot_a, snapshot_b);
    assert_eq!(snapshot_a.subject, subject_a);
}

#[test]
fn generation_mismatch_fails_before_capture() {
    let (world, subject) = build_authority_world(false);
    let wrong_generation = WorldGenerationId::new(subject.world_generation_id().get() + 1).unwrap();
    let mismatched = PhysicsBodySubject::new(
        subject.physical_authority_id(),
        wrong_generation,
        subject.net_id(),
    );

    assert_eq!(
        AuthorityBodySnapshot::capture(&world, mismatched),
        Err(PhysicsIdentityError::WorldGenerationMismatch {
            expected: wrong_generation,
            actual: subject.world_generation_id(),
        })
    );
}

#[test]
fn authority_mismatch_fails_before_capture() {
    let (world, subject) = build_authority_world(false);
    let wrong_authority =
        PhysicalAuthorityId::new(subject.physical_authority_id().get() + 1).unwrap();
    let mismatched = PhysicsBodySubject::new(
        wrong_authority,
        subject.world_generation_id(),
        subject.net_id(),
    );

    assert_eq!(
        AuthorityBodySnapshot::capture(&world, mismatched),
        Err(PhysicsIdentityError::PhysicalAuthorityMismatch {
            expected: wrong_authority,
            actual: subject.physical_authority_id(),
        })
    );
}

#[test]
fn ambiguous_live_net_id_fails_closed() {
    let (authority_id, generation_id, net_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();
    let first = raw.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
    let second = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);

    raw.set_net_id(first, net_id);
    raw.set_net_id(second, net_id);

    let world = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let subject = PhysicsBodySubject::new(authority_id, generation_id, net_id);

    assert_eq!(
        AuthorityBodySnapshot::capture(&world, subject),
        Err(PhysicsIdentityError::AmbiguousNetId { net_id })
    );
}
