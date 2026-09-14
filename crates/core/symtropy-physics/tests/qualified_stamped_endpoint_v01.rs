// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, AuthorityEndpointObservationError, EndpointMembership,
    LocalEvidencePhysicsAuthorityWorld, LocalNamespacePhysicsAuthorityWorld,
    LocalQualifiedPhysicalAuthority, LocalQualifiedStampedEndpointBoxObservation,
    LocalQualifiedStampedEndpointObservationError, NetId, PhysicsBodySubject, PhysicsIdentityError,
    PhysicsWorld, WorldGenerationId,
};

fn prepared_namespace() -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    let mut namespace =
        LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default());

    let anchor_handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::origin(), 0.5, 1.0);
    let subject_handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);

    let anchor = namespace
        .authority_world_mut()
        .bind_net_id(anchor_handle, NetId(8201))
        .expect("anchor bind should succeed");
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(subject_handle, NetId(8202))
        .expect("subject bind should succeed");

    (namespace, subject, anchor)
}

fn seal(
    namespace: LocalNamespacePhysicsAuthorityWorld<3>,
) -> LocalEvidencePhysicsAuthorityWorld<3> {
    match LocalEvidencePhysicsAuthorityWorld::seal(namespace) {
        Ok(evidence) => evidence,
        Err(failure) => panic!("seal failed: {:?}", failure.error()),
    }
}

fn clean_namespace(
    evidence: LocalEvidencePhysicsAuthorityWorld<3>,
) -> LocalNamespacePhysicsAuthorityWorld<3> {
    match evidence.into_namespace() {
        Ok(namespace) => namespace,
        Err(_) => panic!("clean evidence facade unexpectedly quarantined"),
    }
}

fn region(anchor: PhysicsBodySubject) -> AuthorityEndpointBoxSpec<3> {
    AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap()
}

#[test]
fn stamped_capture_requires_a_current_qualified_step() {
    let (namespace, subject, anchor) = prepared_namespace();
    let evidence = seal(namespace);

    assert_eq!(
        LocalQualifiedStampedEndpointBoxObservation::capture(
            &evidence,
            subject,
            region(anchor)
        ),
        Err(LocalQualifiedStampedEndpointObservationError::NoQualifiedStep)
    );
}

#[test]
fn stamped_capture_binds_current_step_and_reuses_endpoint_semantics() {
    let (namespace, subject, anchor) = prepared_namespace();
    let mut evidence = seal(namespace);
    let issued_step = evidence
        .step_authorized(1.0 / 64.0)
        .expect("qualified step should succeed");

    let token = LocalQualifiedStampedEndpointBoxObservation::capture(
        &evidence,
        subject,
        region(anchor),
    )
    .expect("stamped endpoint capture should succeed");

    assert_eq!(token.stamp(), issued_step);
    assert_eq!(token.observation().membership, EndpointMembership::Inside);
    assert_eq!(
        token.observation().subject.subject.physical_authority_id(),
        issued_step.physical_authority_id()
    );
    assert_eq!(
        token.observation().subject.subject.world_generation_id(),
        issued_step.world_generation_id()
    );
    assert_eq!(
        token.observation().anchor.subject.physical_authority_id(),
        issued_step.physical_authority_id()
    );
    assert_eq!(
        token.observation().anchor.subject.world_generation_id(),
        issued_step.world_generation_id()
    );
}

#[test]
fn repeated_capture_at_one_step_is_duplicate_evidence_not_temporal_progression() {
    let (namespace, subject, anchor) = prepared_namespace();
    let mut evidence = seal(namespace);
    evidence.step_authorized(1.0 / 64.0).unwrap();

    let first = LocalQualifiedStampedEndpointBoxObservation::capture(
        &evidence,
        subject,
        region(anchor),
    )
    .unwrap();
    let second = LocalQualifiedStampedEndpointBoxObservation::capture(
        &evidence,
        subject,
        region(anchor),
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.stamp().step_index(), 1);

    evidence.step_authorized(1.0 / 64.0).unwrap();
    let next = LocalQualifiedStampedEndpointBoxObservation::capture(
        &evidence,
        subject,
        region(anchor),
    )
    .unwrap();
    assert_eq!(next.stamp().step_index(), 2);
    assert_ne!(first.stamp(), next.stamp());
}

#[test]
fn same_step_different_region_is_a_different_proposition() {
    let (namespace, subject, anchor) = prepared_namespace();
    let mut evidence = seal(namespace);
    evidence.step_authorized(1.0 / 64.0).unwrap();

    let region_a = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap();
    let region_b = AuthorityEndpointBoxSpec::new(anchor, [0.5, 0.0, 0.0], [2.0; 3]).unwrap();
    let a = LocalQualifiedStampedEndpointBoxObservation::capture(&evidence, subject, region_a)
        .unwrap();
    let b = LocalQualifiedStampedEndpointBoxObservation::capture(&evidence, subject, region_b)
        .unwrap();

    assert_eq!(a.stamp(), b.stamp());
    assert_ne!(a.observation().region, b.observation().region);
    assert_ne!(a, b);
}

#[test]
fn qualified_anchor_identity_failure_preserves_endpoint_error_attribution() {
    let (namespace, subject, anchor) = prepared_namespace();
    let mut evidence = seal(namespace);
    evidence.step_authorized(1.0 / 64.0).unwrap();

    let wrong_generation =
        WorldGenerationId::new(anchor.world_generation_id().get() + 1).unwrap();
    let wrong_anchor = PhysicsBodySubject::new(
        anchor.physical_authority_id(),
        wrong_generation,
        anchor.net_id(),
    );

    assert_eq!(
        LocalQualifiedStampedEndpointBoxObservation::capture(
            &evidence,
            subject,
            region(wrong_anchor)
        ),
        Err(LocalQualifiedStampedEndpointObservationError::Endpoint(
            AuthorityEndpointObservationError::Anchor(
                PhysicsIdentityError::WorldGenerationMismatch {
                    expected: wrong_generation,
                    actual: anchor.world_generation_id(),
                }
            )
        ))
    );
}

#[test]
fn reseal_breaks_stamped_endpoint_temporal_lineage() {
    let (namespace, subject, anchor) = prepared_namespace();
    let mut first = seal(namespace);
    first.step_authorized(1.0 / 64.0).unwrap();
    let first_token = LocalQualifiedStampedEndpointBoxObservation::capture(
        &first,
        subject,
        region(anchor),
    )
    .unwrap();

    let namespace = clean_namespace(first);
    let mut second = seal(namespace);
    second.step_authorized(1.0 / 64.0).unwrap();
    let second_token = LocalQualifiedStampedEndpointBoxObservation::capture(
        &second,
        subject,
        region(anchor),
    )
    .unwrap();

    assert_eq!(first_token.stamp().step_index(), 1);
    assert_eq!(second_token.stamp().step_index(), 1);
    assert_ne!(
        first_token.stamp().temporal_incarnation_id(),
        second_token.stamp().temporal_incarnation_id()
    );
    assert_ne!(first_token.stamp(), second_token.stamp());
}
