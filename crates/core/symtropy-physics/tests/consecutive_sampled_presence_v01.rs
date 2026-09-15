// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, EndpointMembership, LocalConsecutiveSampledEndpointPresence,
    LocalConsecutiveSampledPresenceError, LocalEvidencePhysicsAuthorityWorld,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority,
    LocalQualifiedStampedEndpointBoxObservation, LocalQualifiedWorldGeneration, NetId,
    PhysicsBodySubject, PhysicsWorld,
};

fn build_namespace(
    generation: LocalQualifiedWorldGeneration,
    anchor_position: [f64; 3],
    anchor_velocity: [f64; 3],
    subject_position: [f64; 3],
    subject_velocity: [f64; 3],
) -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::new(anchor_position), 0.1, 1.0);
    let subject_handle = raw.add_sphere(Point::new(subject_position), 0.1, 1.0);
    raw.body_mut(anchor_handle).unwrap().linear_velocity = SVector::from_row_slice(&anchor_velocity);
    raw.body_mut(subject_handle).unwrap().linear_velocity =
        SVector::from_row_slice(&subject_velocity);

    let mut namespace = LocalNamespacePhysicsAuthorityWorld::bind(generation, raw);
    let anchor = namespace
        .authority_world_mut()
        .bind_net_id(anchor_handle, NetId(8301))
        .unwrap();
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(subject_handle, NetId(8302))
        .unwrap();
    (namespace, subject, anchor)
}

fn fresh_namespace(
    anchor_position: [f64; 3],
    anchor_velocity: [f64; 3],
    subject_position: [f64; 3],
    subject_velocity: [f64; 3],
) -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation = root.mint_generation().unwrap();
    build_namespace(
        generation,
        anchor_position,
        anchor_velocity,
        subject_position,
        subject_velocity,
    )
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

fn region(anchor: PhysicsBodySubject, offset_x: f64, half_extent: f64) -> AuthorityEndpointBoxSpec<3> {
    AuthorityEndpointBoxSpec::new(
        anchor,
        [offset_x, 0.0, 0.0],
        [half_extent, half_extent, half_extent],
    )
    .unwrap()
}

fn capture(
    evidence: &LocalEvidencePhysicsAuthorityWorld<3>,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<3>,
) -> LocalQualifiedStampedEndpointBoxObservation<3> {
    LocalQualifiedStampedEndpointBoxObservation::capture(evidence, subject, region).unwrap()
}

fn step_and_capture(
    evidence: &mut LocalEvidencePhysicsAuthorityWorld<3>,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<3>,
) -> LocalQualifiedStampedEndpointBoxObservation<3> {
    evidence.step_authorized(1.0).unwrap();
    capture(evidence, subject, region)
}

#[test]
fn adjacent_inside_samples_issue_exact_sampled_presence_proof() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let endpoint = region(anchor, 100.0, 2.0);
    let mut evidence = seal(namespace);
    let previous = step_and_capture(&mut evidence, subject, endpoint);
    let current = step_and_capture(&mut evidence, subject, endpoint);

    let proof = LocalConsecutiveSampledEndpointPresence::try_from_samples(&previous, &current)
        .expect("adjacent inside samples should qualify");
    assert_eq!(proof.previous_stamp().step_index(), 1);
    assert_eq!(proof.current_stamp().step_index(), 2);
    assert_eq!(proof.subject(), subject);
    assert_eq!(proof.region(), endpoint);
}

#[test]
fn repeated_same_step_capture_is_duplicate_not_progress() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let endpoint = region(anchor, 100.0, 2.0);
    let mut evidence = seal(namespace);
    evidence.step_authorized(1.0).unwrap();
    let first = capture(&evidence, subject, endpoint);
    let duplicate = capture(&evidence, subject, endpoint);

    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&first, &duplicate),
        Err(LocalConsecutiveSampledPresenceError::DuplicateStep { step_index: 1 })
    );
}

#[test]
fn reversed_and_skipped_steps_fail_closed() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let endpoint = region(anchor, 100.0, 2.0);
    let mut evidence = seal(namespace);
    let first = step_and_capture(&mut evidence, subject, endpoint);
    let second = step_and_capture(&mut evidence, subject, endpoint);

    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&second, &first),
        Err(LocalConsecutiveSampledPresenceError::ReversedStep {
            previous_step: 2,
            current_step: 1,
        })
    );

    evidence.step_authorized(1.0).unwrap();
    let fourth = step_and_capture(&mut evidence, subject, endpoint);
    assert_eq!(fourth.stamp().step_index(), 4);
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&second, &fourth),
        Err(LocalConsecutiveSampledPresenceError::StepGap {
            previous_step: 2,
            current_step: 4,
        })
    );
}

#[test]
fn reseal_breaks_lineage_even_when_numeric_indexes_look_adjacent() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let endpoint = region(anchor, 100.0, 2.0);
    let mut first_evidence = seal(namespace);
    let previous = step_and_capture(&mut first_evidence, subject, endpoint);

    let namespace = clean_namespace(first_evidence);
    let mut second_evidence = seal(namespace);
    second_evidence.step_authorized(1.0).unwrap();
    let current = step_and_capture(&mut second_evidence, subject, endpoint);
    assert_eq!(previous.stamp().step_index(), 1);
    assert_eq!(current.stamp().step_index(), 2);

    assert!(matches!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&previous, &current),
        Err(LocalConsecutiveSampledPresenceError::TemporalIncarnationMismatch { .. })
    ));
}

#[test]
fn different_authority_and_generation_are_not_one_sampled_lineage() {
    let (namespace_a, subject_a, anchor_a) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let (namespace_b, subject_b, anchor_b) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let mut a = seal(namespace_a);
    let mut b = seal(namespace_b);
    let previous = step_and_capture(&mut a, subject_a, region(anchor_a, 100.0, 2.0));
    b.step_authorized(1.0).unwrap();
    let current = step_and_capture(&mut b, subject_b, region(anchor_b, 100.0, 2.0));
    assert!(matches!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&previous, &current),
        Err(LocalConsecutiveSampledPresenceError::PhysicalAuthorityMismatch { .. })
    ));

    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation_a = root.mint_generation().unwrap();
    let generation_b = root.mint_generation().unwrap();
    let (namespace_a, subject_a, anchor_a) = build_namespace(
        generation_a,
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let (namespace_b, subject_b, anchor_b) = build_namespace(
        generation_b,
        [0.0; 3],
        [0.0; 3],
        [100.0, 0.0, 0.0],
        [0.0; 3],
    );
    let mut a = seal(namespace_a);
    let mut b = seal(namespace_b);
    let previous = step_and_capture(&mut a, subject_a, region(anchor_a, 100.0, 2.0));
    b.step_authorized(1.0).unwrap();
    let current = step_and_capture(&mut b, subject_b, region(anchor_b, 100.0, 2.0));
    assert!(matches!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&previous, &current),
        Err(LocalConsecutiveSampledPresenceError::WorldGenerationMismatch { .. })
    ));
}

#[test]
fn target_and_region_mismatches_are_rejected_before_duplicate_step_semantics() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation = root.mint_generation().unwrap();
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::origin(), 0.1, 1.0);
    let other_anchor_handle = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
    let first_handle = raw.add_sphere(Point::new([100.0, 0.0, 0.0]), 0.1, 1.0);
    let second_handle = raw.add_sphere(Point::new([100.5, 0.0, 0.0]), 0.1, 1.0);
    let mut namespace = LocalNamespacePhysicsAuthorityWorld::bind(generation, raw);
    let anchor = namespace.authority_world_mut().bind_net_id(anchor_handle, NetId(8401)).unwrap();
    let other_anchor = namespace
        .authority_world_mut()
        .bind_net_id(other_anchor_handle, NetId(8402))
        .unwrap();
    let first_subject = namespace
        .authority_world_mut()
        .bind_net_id(first_handle, NetId(8403))
        .unwrap();
    let second_subject = namespace
        .authority_world_mut()
        .bind_net_id(second_handle, NetId(8404))
        .unwrap();

    let mut evidence = seal(namespace);
    evidence.step_authorized(1.0).unwrap();
    let endpoint = region(anchor, 100.0, 5.0);
    let first = capture(&evidence, first_subject, endpoint);
    let different_target = capture(&evidence, second_subject, endpoint);
    assert!(matches!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&first, &different_target),
        Err(LocalConsecutiveSampledPresenceError::TargetMismatch { .. })
    ));

    let changed_offset = capture(&evidence, first_subject, region(anchor, 100.25, 5.0));
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&first, &changed_offset),
        Err(LocalConsecutiveSampledPresenceError::RegionMismatch)
    );

    let changed_anchor = capture(&evidence, first_subject, region(other_anchor, 99.0, 5.0));
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&first, &changed_anchor),
        Err(LocalConsecutiveSampledPresenceError::RegionMismatch)
    );
}

#[test]
fn all_outside_membership_combinations_are_rejected() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [110.0, 0.0, 0.0],
        [0.0; 3],
    );
    let endpoint = region(anchor, 100.0, 0.6);
    let mut evidence = seal(namespace);
    let outside_a = step_and_capture(&mut evidence, subject, endpoint);
    let outside_b = step_and_capture(&mut evidence, subject, endpoint);
    assert_eq!(outside_a.observation().membership, EndpointMembership::Outside);
    assert_eq!(outside_b.observation().membership, EndpointMembership::Outside);
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&outside_a, &outside_b),
        Err(LocalConsecutiveSampledPresenceError::PreviousOutside)
    );

    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [97.5, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    );
    let endpoint = region(anchor, 100.0, 0.6);
    let mut evidence = seal(namespace);
    let outside = step_and_capture(&mut evidence, subject, endpoint);
    let inside = step_and_capture(&mut evidence, subject, endpoint);
    assert_eq!(outside.observation().membership, EndpointMembership::Outside);
    assert_eq!(inside.observation().membership, EndpointMembership::Inside);
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&outside, &inside),
        Err(LocalConsecutiveSampledPresenceError::PreviousOutside)
    );

    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [0.0; 3],
        [98.5, 0.0, 0.0],
        [2.0, 0.0, 0.0],
    );
    let endpoint = region(anchor, 100.0, 0.6);
    let mut evidence = seal(namespace);
    let inside = step_and_capture(&mut evidence, subject, endpoint);
    let outside = step_and_capture(&mut evidence, subject, endpoint);
    assert_eq!(inside.observation().membership, EndpointMembership::Inside);
    assert_eq!(outside.observation().membership, EndpointMembership::Outside);
    assert_eq!(
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&inside, &outside),
        Err(LocalConsecutiveSampledPresenceError::CurrentOutside)
    );
}

#[test]
fn moving_anchor_is_allowed_when_exact_anchor_relative_region_is_unchanged() {
    let (namespace, subject, anchor) = fresh_namespace(
        [0.0; 3],
        [1.0, 0.0, 0.0],
        [100.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
    );
    let endpoint = region(anchor, 100.0, 2.0);
    let mut evidence = seal(namespace);
    let previous = step_and_capture(&mut evidence, subject, endpoint);
    let current = step_and_capture(&mut evidence, subject, endpoint);

    assert_ne!(
        previous.observation().anchor.translation,
        current.observation().anchor.translation
    );
    assert_eq!(previous.observation().region, current.observation().region);
    LocalConsecutiveSampledEndpointPresence::try_from_samples(&previous, &current)
        .expect("moving anchor remains valid under unchanged anchor-relative proposition");
}
