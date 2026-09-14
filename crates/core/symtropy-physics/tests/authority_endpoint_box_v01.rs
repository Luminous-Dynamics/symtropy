// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::f64::consts::FRAC_PI_2;

use symtropy_math::{Bivector, Point, Rotor};
use symtropy_physics::{
    AuthorityEndpointBoxObservation, AuthorityEndpointBoxSpec, AuthorityEndpointObservationError,
    EndpointBoxSpecError, EndpointMembership, NetId, PhysicalAuthorityId, PhysicsAuthorityWorld,
    PhysicsBodySubject, PhysicsIdentityError, PhysicsWorld, WorldGenerationId,
};

fn ids() -> (PhysicalAuthorityId, WorldGenerationId) {
    (
        PhysicalAuthorityId::new(0xE11D).unwrap(),
        WorldGenerationId::new(17).unwrap(),
    )
}

fn build_world(
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

    let anchor_handle = raw.add_sphere(Point::new([10.0, 20.0, 30.0]), 0.5, 1.0);
    let subject_handle = raw.add_sphere(Point::new([12.0, 22.0, 30.0]), 0.5, 1.0);

    let mut authority = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let anchor = authority.bind_net_id(anchor_handle, NetId(9301)).unwrap();
    let subject = authority.bind_net_id(subject_handle, NetId(9302)).unwrap();
    (authority, subject, anchor)
}

fn region(anchor: PhysicsBodySubject) -> AuthorityEndpointBoxSpec<3> {
    AuthorityEndpointBoxSpec::new(anchor, [1.0, 0.0, 0.0], [2.0, 3.0, 1.0]).unwrap()
}

fn set_position(
    world: &mut PhysicsAuthorityWorld<3>,
    subject: PhysicsBodySubject,
    position: [f64; 3],
) {
    let handle = {
        let validated = world.validate_subject(subject).unwrap();
        validated.runtime_handle()
    };
    world
        .world_mut()
        .body_mut(handle)
        .unwrap()
        .transform
        .translation = Point::new(position);
}

#[test]
fn spec_rejects_invalid_values_and_canonicalizes_signed_zero() {
    let (authority_id, generation_id) = ids();
    let anchor = PhysicsBodySubject::new(authority_id, generation_id, NetId(1));

    let spec = AuthorityEndpointBoxSpec::<3>::new(
        anchor,
        [-0.0, 1.0, 0.0],
        [1.0, 2.0, 3.0],
    )
    .unwrap();
    assert_eq!(spec.center_offset()[0].to_bits(), 0.0_f64.to_bits());

    assert_eq!(
        AuthorityEndpointBoxSpec::<3>::new(anchor, [f64::NAN, 0.0, 0.0], [1.0; 3]),
        Err(EndpointBoxSpecError::NonFiniteCenterOffset { axis: 0 })
    );
    assert_eq!(
        AuthorityEndpointBoxSpec::<3>::new(anchor, [0.0; 3], [1.0, f64::INFINITY, 1.0]),
        Err(EndpointBoxSpecError::NonFiniteHalfExtent { axis: 1 })
    );
    assert_eq!(
        AuthorityEndpointBoxSpec::<3>::new(anchor, [0.0; 3], [1.0, 0.0, 1.0]),
        Err(EndpointBoxSpecError::NonPositiveHalfExtent { axis: 1 })
    );
    assert_eq!(
        AuthorityEndpointBoxSpec::<3>::new(anchor, [0.0; 3], [1.0, -1.0, 1.0]),
        Err(EndpointBoxSpecError::NonPositiveHalfExtent { axis: 1 })
    );
}

#[test]
fn equivalent_endpoint_observation_is_handle_independent() {
    let (world_a, subject_a, anchor_a) = build_world(false);
    let (world_b, subject_b, anchor_b) = build_world(true);

    {
        let subject_a_valid = world_a.validate_subject(subject_a).unwrap();
        let subject_b_valid = world_b.validate_subject(subject_b).unwrap();
        let anchor_a_valid = world_a.validate_subject(anchor_a).unwrap();
        let anchor_b_valid = world_b.validate_subject(anchor_b).unwrap();
        assert_ne!(subject_a_valid.runtime_handle(), subject_b_valid.runtime_handle());
        assert_ne!(anchor_a_valid.runtime_handle(), anchor_b_valid.runtime_handle());
    }

    let region_a = region(anchor_a);
    let region_b = region(anchor_b);
    assert_eq!(region_a, region_b);

    let observation_a =
        AuthorityEndpointBoxObservation::capture(&world_a, subject_a, region_a).unwrap();
    let observation_b =
        AuthorityEndpointBoxObservation::capture(&world_b, subject_b, region_b).unwrap();

    assert_eq!(observation_a, observation_b);
    assert_eq!(observation_a.membership, EndpointMembership::Inside);
    assert_eq!(
        observation_a.region_center,
        [11.0_f64.to_bits(), 20.0_f64.to_bits(), 30.0_f64.to_bits()]
    );
    assert_eq!(
        observation_a.offset_from_center,
        [1.0_f64.to_bits(), 2.0_f64.to_bits(), 0.0_f64.to_bits()]
    );
}

#[test]
fn boundary_is_closed_and_outside_is_distinct() {
    let (mut world, subject, anchor) = build_world(false);
    let endpoint = region(anchor);

    set_position(&mut world, subject, [13.0, 23.0, 31.0]);
    let boundary = AuthorityEndpointBoxObservation::capture(&world, subject, endpoint).unwrap();
    assert_eq!(boundary.membership, EndpointMembership::Inside);

    set_position(&mut world, subject, [13.000_001, 23.0, 31.0]);
    let outside = AuthorityEndpointBoxObservation::capture(&world, subject, endpoint).unwrap();
    assert_eq!(outside.membership, EndpointMembership::Outside);
}

#[test]
fn anchor_rotation_is_explicitly_ignored() {
    let (mut world, subject, anchor) = build_world(false);
    let endpoint = region(anchor);
    let before = AuthorityEndpointBoxObservation::capture(&world, subject, endpoint).unwrap();

    let anchor_handle = {
        let validated = world.validate_subject(anchor).unwrap();
        validated.runtime_handle()
    };
    world.world_mut().body_mut(anchor_handle).unwrap().transform.rotation =
        Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), FRAC_PI_2);

    let after = AuthorityEndpointBoxObservation::capture(&world, subject, endpoint).unwrap();
    assert_eq!(before.membership, after.membership);
    assert_eq!(before.region_center, after.region_center);
    assert_eq!(before.offset_from_center, after.offset_from_center);
    assert_ne!(before.anchor.rotation, after.anchor.rotation);
}

#[test]
fn same_subject_cannot_prove_its_own_endpoint_membership() {
    let (world, subject, _) = build_world(false);
    let endpoint = region(subject);

    assert_eq!(
        AuthorityEndpointBoxObservation::capture(&world, subject, endpoint),
        Err(AuthorityEndpointObservationError::SameSubject { subject })
    );
}

#[test]
fn anchor_identity_failure_is_attributed_to_anchor() {
    let (world, subject, anchor) = build_world(false);
    let wrong_generation = WorldGenerationId::new(anchor.world_generation_id().get() + 1).unwrap();
    let wrong_anchor = PhysicsBodySubject::new(
        anchor.physical_authority_id(),
        wrong_generation,
        anchor.net_id(),
    );
    let endpoint = region(wrong_anchor);

    assert_eq!(
        AuthorityEndpointBoxObservation::capture(&world, subject, endpoint),
        Err(AuthorityEndpointObservationError::Anchor(
            PhysicsIdentityError::WorldGenerationMismatch {
                expected: wrong_generation,
                actual: anchor.world_generation_id(),
            }
        ))
    );
}

#[test]
fn region_center_overflow_fails_closed() {
    let (authority_id, generation_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::new([f64::MAX, 0.0, 0.0]), 0.5, 1.0);
    let subject_handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let anchor = world.bind_net_id(anchor_handle, NetId(9401)).unwrap();
    let subject = world.bind_net_id(subject_handle, NetId(9402)).unwrap();
    let endpoint = AuthorityEndpointBoxSpec::new(anchor, [f64::MAX, 0.0, 0.0], [1.0; 3]).unwrap();

    assert_eq!(
        AuthorityEndpointBoxObservation::capture(&world, subject, endpoint),
        Err(AuthorityEndpointObservationError::NonFiniteRegionCenter { axis: 0 })
    );
}

#[test]
fn subject_offset_overflow_fails_closed() {
    let (authority_id, generation_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::new([-f64::MAX, 0.0, 0.0]), 0.5, 1.0);
    let subject_handle = raw.add_sphere(Point::new([f64::MAX, 0.0, 0.0]), 0.5, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let anchor = world.bind_net_id(anchor_handle, NetId(9501)).unwrap();
    let subject = world.bind_net_id(subject_handle, NetId(9502)).unwrap();
    let endpoint = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [1.0; 3]).unwrap();

    assert_eq!(
        AuthorityEndpointBoxObservation::capture(&world, subject, endpoint),
        Err(AuthorityEndpointObservationError::NonFiniteOffsetFromCenter { axis: 0 })
    );
}

#[test]
fn membership_is_body_origin_only_not_collider_containment() {
    let (authority_id, generation_id) = ids();
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    // The large target sphere overlaps the endpoint box, but its origin is far outside.
    let subject_handle = raw.add_sphere(Point::new([5.0, 0.0, 0.0]), 10.0, 1.0);
    let mut world = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let anchor = world.bind_net_id(anchor_handle, NetId(9601)).unwrap();
    let subject = world.bind_net_id(subject_handle, NetId(9602)).unwrap();
    let endpoint = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [1.0; 3]).unwrap();

    let observation =
        AuthorityEndpointBoxObservation::capture(&world, subject, endpoint).unwrap();
    assert_eq!(observation.membership, EndpointMembership::Outside);
}
