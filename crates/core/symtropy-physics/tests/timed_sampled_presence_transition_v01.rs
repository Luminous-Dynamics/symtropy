// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, LocalConsecutiveSampledEndpointPresence,
    LocalExactTimedSampledPresenceTransition, LocalExactTimedSampledPresenceTransitionError,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority,
    LocalQualifiedStampedEndpointBoxObservation, LocalQualifiedStepExecutionReceipt,
    LocalQualifiedWorldGeneration, LocalReceiptedEvidencePhysicsAuthorityWorld, NetId,
    PhysicsBodySubject, PhysicsWorld,
};

fn build_namespace(
    generation: LocalQualifiedWorldGeneration,
) -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::origin(), 0.1, 1.0);
    let subject_handle = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);

    let mut namespace = LocalNamespacePhysicsAuthorityWorld::bind(generation, raw);
    let anchor = namespace
        .authority_world_mut()
        .bind_net_id(anchor_handle, NetId(8601))
        .unwrap();
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(subject_handle, NetId(8602))
        .unwrap();
    (namespace, subject, anchor)
}

fn fresh_namespace() -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation = root.mint_generation().unwrap();
    build_namespace(generation)
}

fn seal(
    namespace: LocalNamespacePhysicsAuthorityWorld<3>,
) -> LocalReceiptedEvidencePhysicsAuthorityWorld<3> {
    match LocalReceiptedEvidencePhysicsAuthorityWorld::seal(namespace) {
        Ok(timed) => timed,
        Err(failure) => panic!("seal failed: {:?}", failure.error()),
    }
}

fn endpoint(anchor: PhysicsBodySubject) -> AuthorityEndpointBoxSpec<3> {
    AuthorityEndpointBoxSpec::new(anchor, [1.0, 0.0, 0.0], [2.0; 3]).unwrap()
}

fn capture(
    timed: &LocalReceiptedEvidencePhysicsAuthorityWorld<3>,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<3>,
) -> LocalQualifiedStampedEndpointBoxObservation<3> {
    LocalQualifiedStampedEndpointBoxObservation::capture(
        timed.evidence_authority(),
        subject,
        region,
    )
    .unwrap()
}

fn relation_with_receipts(
    mut timed: LocalReceiptedEvidencePhysicsAuthorityWorld<3>,
    subject: PhysicsBodySubject,
    region: AuthorityEndpointBoxSpec<3>,
) -> (
    LocalReceiptedEvidencePhysicsAuthorityWorld<3>,
    LocalConsecutiveSampledEndpointPresence<3>,
    LocalQualifiedStepExecutionReceipt,
    LocalQualifiedStepExecutionReceipt,
) {
    let first_receipt = timed.step_authorized(0.125).unwrap();
    let first_sample = capture(&timed, subject, region);
    let second_receipt = timed.step_authorized(0.25).unwrap();
    let second_sample = capture(&timed, subject, region);
    let relation =
        LocalConsecutiveSampledEndpointPresence::try_from_samples(&first_sample, &second_sample)
            .unwrap();
    (timed, relation, first_receipt, second_receipt)
}

#[test]
fn exact_current_receipt_binds_one_timed_sampled_transition() {
    let (namespace, subject, anchor) = fresh_namespace();
    let region = endpoint(anchor);
    let (_timed, relation, _first_receipt, second_receipt) =
        relation_with_receipts(seal(namespace), subject, region);

    let transition = LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
        &relation,
        &second_receipt,
    )
    .expect("current-step receipt should bind");

    assert_eq!(transition.previous_stamp(), relation.previous_stamp());
    assert_eq!(transition.current_stamp(), relation.current_stamp());
    assert_eq!(transition.subject(), subject);
    assert_eq!(transition.region(), region);
    assert_eq!(transition.dt_bits(), 0.25_f64.to_bits());
    assert_eq!(transition.executed_dt().to_bits(), 0.25_f64.to_bits());
}

#[test]
fn previous_and_future_receipts_are_not_transition_duration_authority() {
    let (namespace, subject, anchor) = fresh_namespace();
    let region = endpoint(anchor);
    let (mut timed, relation, first_receipt, second_receipt) =
        relation_with_receipts(seal(namespace), subject, region);

    assert_eq!(
        LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
            &relation,
            &first_receipt,
        ),
        Err(LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
            expected_current: relation.current_stamp(),
            actual_receipt: first_receipt.stamp(),
        })
    );

    let third_receipt = timed.step_authorized(0.5).unwrap();
    assert_eq!(
        LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
            &relation,
            &third_receipt,
        ),
        Err(LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
            expected_current: relation.current_stamp(),
            actual_receipt: third_receipt.stamp(),
        })
    );

    let valid = LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
        &relation,
        &second_receipt,
    )
    .unwrap();
    assert_eq!(valid.current_stamp().step_index(), 2);
}

#[test]
fn equal_numeric_step_from_fresh_temporal_incarnation_is_rejected() {
    let (namespace, subject, anchor) = fresh_namespace();
    let region = endpoint(anchor);
    let (timed, relation, _first_receipt, current_receipt) =
        relation_with_receipts(seal(namespace), subject, region);
    let first_incarnation = current_receipt.stamp().temporal_incarnation_id();

    let namespace = match timed.into_namespace() {
        Ok(namespace) => namespace,
        Err(_) => panic!("clean receipted authority unexpectedly quarantined"),
    };
    let mut resealed = seal(namespace);
    let _ = resealed.step_authorized(0.125).unwrap();
    let wrong_receipt = resealed.step_authorized(0.25).unwrap();

    assert_eq!(wrong_receipt.stamp().step_index(), relation.current_stamp().step_index());
    assert_ne!(wrong_receipt.stamp().temporal_incarnation_id(), first_incarnation);
    assert_eq!(
        LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
            &relation,
            &wrong_receipt,
        ),
        Err(LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
            expected_current: relation.current_stamp(),
            actual_receipt: wrong_receipt.stamp(),
        })
    );
}

#[test]
fn equal_numeric_step_from_other_generation_or_authority_is_rejected() {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation_a = root.mint_generation().unwrap();
    let generation_b = root.mint_generation().unwrap();
    let (namespace_a, subject_a, anchor_a) = build_namespace(generation_a);
    let (namespace_b, _subject_b, _anchor_b) = build_namespace(generation_b);

    let region_a = endpoint(anchor_a);
    let (_timed_a, relation, _first_a, _current_a) =
        relation_with_receipts(seal(namespace_a), subject_a, region_a);
    let mut timed_b = seal(namespace_b);
    let _ = timed_b.step_authorized(0.125).unwrap();
    let other_generation_receipt = timed_b.step_authorized(0.25).unwrap();

    assert_eq!(
        other_generation_receipt.stamp().physical_authority_id(),
        relation.current_stamp().physical_authority_id()
    );
    assert_ne!(
        other_generation_receipt.stamp().world_generation_id(),
        relation.current_stamp().world_generation_id()
    );
    assert_eq!(
        LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
            &relation,
            &other_generation_receipt,
        ),
        Err(LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
            expected_current: relation.current_stamp(),
            actual_receipt: other_generation_receipt.stamp(),
        })
    );

    let (other_namespace, _other_subject, _other_anchor) = fresh_namespace();
    let mut other_authority = seal(other_namespace);
    let _ = other_authority.step_authorized(0.125).unwrap();
    let other_authority_receipt = other_authority.step_authorized(0.25).unwrap();
    assert_ne!(
        other_authority_receipt.stamp().physical_authority_id(),
        relation.current_stamp().physical_authority_id()
    );
    assert_eq!(
        LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
            &relation,
            &other_authority_receipt,
        ),
        Err(LocalExactTimedSampledPresenceTransitionError::ReceiptStampMismatch {
            expected_current: relation.current_stamp(),
            actual_receipt: other_authority_receipt.stamp(),
        })
    );
}

#[test]
fn repeated_derivation_is_equal_semantic_evidence_not_progress() {
    let (namespace, subject, anchor) = fresh_namespace();
    let region = endpoint(anchor);
    let (_timed, relation, _first_receipt, current_receipt) =
        relation_with_receipts(seal(namespace), subject, region);

    let first = LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
        &relation,
        &current_receipt,
    )
    .unwrap();
    let repeated = LocalExactTimedSampledPresenceTransition::try_from_relation_and_receipt(
        &relation,
        &current_receipt,
    )
    .unwrap();

    assert_eq!(first, repeated);
    assert_eq!(first.previous_stamp().step_index(), 1);
    assert_eq!(first.current_stamp().step_index(), 2);
}
