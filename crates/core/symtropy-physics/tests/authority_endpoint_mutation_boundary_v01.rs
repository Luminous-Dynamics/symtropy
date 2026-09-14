// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxObservation, AuthorityEndpointBoxSpec, EndpointMembership, NetId,
    PhysicalAuthorityId, PhysicsAuthorityWorld, PhysicsWorld, WorldGenerationId,
};

#[test]
fn raw_mechanics_mutation_changes_membership_without_proving_temporal_continuity() {
    let authority_id = PhysicalAuthorityId::new(0xE11D_04).unwrap();
    let generation_id = WorldGenerationId::new(1).unwrap();
    let mut raw = PhysicsWorld::<3>::default();

    let anchor_handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let target_handle = raw.add_sphere(Point::new([5.0, 0.0, 0.0]), 0.5, 1.0);

    let mut authority = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let anchor = authority.bind_net_id(anchor_handle, NetId(9701)).unwrap();
    let target = authority.bind_net_id(target_handle, NetId(9702)).unwrap();
    let endpoint = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [1.0; 3]).unwrap();

    let outside = AuthorityEndpointBoxObservation::capture(&authority, target, endpoint).unwrap();
    assert_eq!(outside.membership, EndpointMembership::Outside);

    let runtime_handle = {
        let validated = authority.validate_subject(target).unwrap();
        validated.runtime_handle()
    };
    authority
        .world_mut()
        .body_mut(runtime_handle)
        .unwrap()
        .transform
        .translation = Point::new([0.5, 0.0, 0.0]);

    let inside = AuthorityEndpointBoxObservation::capture(&authority, target, endpoint).unwrap();
    assert_eq!(inside.membership, EndpointMembership::Inside);

    // Both observations are valid current-state captures. There is intentionally
    // no authority-owned step stamp, mutation epoch, or transition proof linking
    // them. PHYS-OBS-04 / #1042 must establish that temporal provenance before
    // endpoint sequences can be promoted to dwell or arrival evidence.
    assert_eq!(outside.subject.subject, inside.subject.subject);
    assert_eq!(outside.anchor.subject, inside.anchor.subject);
}
