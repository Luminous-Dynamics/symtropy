// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    AuthorityPairObservationError, AuthorityPairSnapshot, NetId, PhysicalAuthorityId,
    PhysicsAuthorityWorld, PhysicsBodySubject, PhysicsIdentityError, PhysicsWorld,
    WorldGenerationId,
};

fn ids() -> (PhysicalAuthorityId, WorldGenerationId) {
    (
        PhysicalAuthorityId::new(0xBEEF).unwrap(),
        WorldGenerationId::new(11).unwrap(),
    )
}

fn build_pair_world(
    insert_dummy_first: bool,
) -> (
    PhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let (authority_id, generation_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();

    if insert_dummy_first {
        raw.add_sphere(Point::new([-100.0, 0.0, 0.0]), 0.25, 1.0);
    }

    let handle_a = raw.add_sphere(Point::new([1.0, 2.0, 3.0]), 0.5, 1.0);
    let handle_b = raw.add_sphere(Point::new([4.0, 6.0, 3.0]), 0.5, 2.0);

    let mut authority = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let subject_a = authority.bind_net_id(handle_a, NetId(9101)).unwrap();
    let subject_b = authority.bind_net_id(handle_b, NetId(9102)).unwrap();
    (authority, subject_a, subject_b)
}

#[test]
fn equivalent_pair_state_is_handle_independent() {
    let (world_a, subject_a1, subject_b1) = build_pair_world(false);
    let (world_b, subject_a2, subject_b2) = build_pair_world(true);

    {
        let a1 = world_a.validate_subject(subject_a1).unwrap();
        let b1 = world_a.validate_subject(subject_b1).unwrap();
        let a2 = world_b.validate_subject(subject_a2).unwrap();
        let b2 = world_b.validate_subject(subject_b2).unwrap();
        assert_ne!(a1.runtime_handle(), a2.runtime_handle());
        assert_ne!(b1.runtime_handle(), b2.runtime_handle());
    }

    let snapshot_a = AuthorityPairSnapshot::capture(&world_a, subject_a1, subject_b1).unwrap();
    let snapshot_b = AuthorityPairSnapshot::capture(&world_b, subject_a2, subject_b2).unwrap();

    assert_eq!(snapshot_a, snapshot_b);
    assert_eq!(
        snapshot_a.relative_translation,
        [3.0_f64.to_bits(), 4.0_f64.to_bits(), 0.0_f64.to_bits()]
    );
    assert_eq!(snapshot_a.separation_squared, 25.0_f64.to_bits());
}

#[test]
fn same_subject_pair_is_rejected() {
    let (world, subject_a, _) = build_pair_world(false);

    assert_eq!(
        AuthorityPairSnapshot::capture(&world, subject_a, subject_a),
        Err(AuthorityPairObservationError::SameSubject { subject: subject_a })
    );
}

#[test]
fn subject_b_generation_mismatch_is_attributed_to_b() {
    let (world, subject_a, subject_b) = build_pair_world(false);
    let wrong_generation = WorldGenerationId::new(subject_b.world_generation_id().get() + 1).unwrap();
    let mismatched_b = PhysicsBodySubject::new(
        subject_b.physical_authority_id(),
        wrong_generation,
        subject_b.net_id(),
    );

    assert_eq!(
        AuthorityPairSnapshot::capture(&world, subject_a, mismatched_b),
        Err(AuthorityPairObservationError::SubjectB(
            PhysicsIdentityError::WorldGenerationMismatch {
                expected: wrong_generation,
                actual: subject_b.world_generation_id(),
            }
        ))
    );
}

#[test]
fn unknown_subject_a_is_attributed_to_a() {
    let (world, subject_a, subject_b) = build_pair_world(false);
    let unknown_net_id = NetId(subject_a.net_id().0 + 1000);
    let unknown_a = PhysicsBodySubject::new(
        subject_a.physical_authority_id(),
        subject_a.world_generation_id(),
        unknown_net_id,
    );

    assert_eq!(
        AuthorityPairSnapshot::capture(&world, unknown_a, subject_b),
        Err(AuthorityPairObservationError::SubjectA(
            PhysicsIdentityError::UnknownNetId {
                net_id: unknown_net_id,
            }
        ))
    );
}

#[test]
fn non_finite_relative_translation_fails_closed() {
    let (mut world, subject_a, subject_b) = build_pair_world(false);
    let handle_b = world
        .validate_subject(subject_b)
        .unwrap()
        .runtime_handle();
    world
        .world_mut()
        .body_mut(handle_b)
        .unwrap()
        .transform
        .translation
        .0[0] = f64::NAN;

    assert_eq!(
        AuthorityPairSnapshot::capture(&world, subject_a, subject_b),
        Err(AuthorityPairObservationError::NonFiniteRelativeTranslation { axis: 0 })
    );
}

#[test]
fn finite_delta_with_overflowing_separation_fails_closed() {
    let (mut world, subject_a, subject_b) = build_pair_world(false);
    let handle_b = world
        .validate_subject(subject_b)
        .unwrap()
        .runtime_handle();
    world
        .world_mut()
        .body_mut(handle_b)
        .unwrap()
        .transform
        .translation
        .0 = SVector::from([f64::MAX / 2.0, 0.0, 0.0]);

    assert_eq!(
        AuthorityPairSnapshot::capture(&world, subject_a, subject_b),
        Err(AuthorityPairObservationError::NonFiniteSeparationSquared)
    );
}
