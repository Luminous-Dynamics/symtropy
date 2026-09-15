// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, LocalFixedCadenceError,
    LocalFixedCadenceEvidencePhysicsAuthorityWorld, LocalFixedSimulationCadence,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority,
    LocalQualifiedStampedEndpointBoxObservation, NetId, NoOpCallback, PhysicsBodySubject,
    PhysicsWorld,
};

fn endpoint_namespace() -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation = root.mint_generation().unwrap();
    let mut raw = PhysicsWorld::<3>::default();
    let anchor_handle = raw.add_sphere(Point::origin(), 0.1, 1.0);
    let subject_handle = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
    let mut namespace = LocalNamespacePhysicsAuthorityWorld::bind(generation, raw);
    let anchor = namespace
        .authority_world_mut()
        .bind_net_id(anchor_handle, NetId(8701))
        .unwrap();
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(subject_handle, NetId(8702))
        .unwrap();
    (namespace, subject, anchor)
}

fn seal(
    namespace: LocalNamespacePhysicsAuthorityWorld<3>,
    cadence: LocalFixedSimulationCadence,
) -> LocalFixedCadenceEvidencePhysicsAuthorityWorld<3> {
    match LocalFixedCadenceEvidencePhysicsAuthorityWorld::seal(namespace, cadence) {
        Ok(fixed) => fixed,
        Err(failure) => panic!("fixed cadence seal failed: {:?}", failure.error()),
    }
}

#[test]
fn cadence_rejects_nonpositive_or_nonfinite_values_and_preserves_exact_bits() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            LocalFixedSimulationCadence::new(invalid),
            Err(LocalFixedCadenceError::NonFiniteDeltaTime)
        );
    }
    for invalid in [0.0, -0.0, -1.0] {
        assert_eq!(
            LocalFixedSimulationCadence::new(invalid),
            Err(LocalFixedCadenceError::NonPositiveDeltaTime)
        );
    }

    let nominal_sixty_hz = 1.0_f64 / 60.0;
    let cadence = LocalFixedSimulationCadence::new(nominal_sixty_hz).unwrap();
    assert_eq!(cadence.dt_bits(), nominal_sixty_hz.to_bits());
    assert_eq!(cadence.executed_dt().to_bits(), nominal_sixty_hz.to_bits());
}

#[test]
fn exact_dyadic_projection_is_canonical_for_normal_and_subnormal_values() {
    let one_over_64 = LocalFixedSimulationCadence::new(1.0 / 64.0)
        .unwrap()
        .exact_dyadic();
    assert_eq!(one_over_64.significand(), 1);
    assert_eq!(one_over_64.exponent2(), -6);

    let one_and_half = LocalFixedSimulationCadence::new(1.5)
        .unwrap()
        .exact_dyadic();
    assert_eq!(one_and_half.significand(), 3);
    assert_eq!(one_and_half.exponent2(), -1);

    let min_normal = LocalFixedSimulationCadence::new(f64::MIN_POSITIVE)
        .unwrap()
        .exact_dyadic();
    assert_eq!(min_normal.significand(), 1);
    assert_eq!(min_normal.exponent2(), -1022);

    let min_subnormal = LocalFixedSimulationCadence::new(f64::from_bits(1))
        .unwrap()
        .exact_dyadic();
    assert_eq!(min_subnormal.significand(), 1);
    assert_eq!(min_subnormal.exponent2(), -1074);
}

#[test]
fn fixed_steps_issue_precommitment_receipts_with_exact_cadence_bits() {
    let (namespace, _subject, _anchor) = endpoint_namespace();
    let cadence = LocalFixedSimulationCadence::new(1.0 / 64.0).unwrap();
    let mut fixed = seal(namespace, cadence);

    let first = fixed.step_authorized_fixed().unwrap();
    let second = fixed.step_authorized_fixed().unwrap();

    assert_eq!(first.stamp().step_index(), 1);
    assert_eq!(second.stamp().step_index(), 2);
    assert_eq!(first.cadence(), cadence);
    assert_eq!(second.cadence(), cadence);
    assert_eq!(first.dt_bits(), cadence.dt_bits());
    assert_eq!(second.dt_bits(), cadence.dt_bits());
    assert_eq!(first.executed_dt().to_bits(), cadence.dt_bits());
    assert_eq!(first.execution_receipt().stamp(), first.stamp());
    assert_eq!(first.execution_receipt().dt_bits(), cadence.dt_bits());
    assert_eq!(first.exact_dyadic(), cadence.exact_dyadic());
}

#[test]
fn callback_fixed_step_uses_the_same_precommitted_cadence() {
    let (namespace, _subject, _anchor) = endpoint_namespace();
    let cadence = LocalFixedSimulationCadence::new(1.0 / 60.0).unwrap();
    let mut fixed = seal(namespace, cadence);
    let mut callback = NoOpCallback;

    let receipt = fixed
        .step_authorized_fixed_with_callback(&mut callback)
        .unwrap();
    assert_eq!(receipt.stamp().step_index(), 1);
    assert_eq!(receipt.cadence(), cadence);
    assert_eq!(receipt.dt_bits(), cadence.dt_bits());
}

#[test]
fn endpoint_sample_after_fixed_step_has_the_same_strong_stamp() {
    let (namespace, subject, anchor) = endpoint_namespace();
    let cadence = LocalFixedSimulationCadence::new(1.0 / 64.0).unwrap();
    let mut fixed = seal(namespace, cadence);
    let receipt = fixed.step_authorized_fixed().unwrap();
    let region = AuthorityEndpointBoxSpec::new(anchor, [1.0, 0.0, 0.0], [2.0; 3]).unwrap();
    let sample = LocalQualifiedStampedEndpointBoxObservation::capture(
        fixed.evidence_authority(),
        subject,
        region,
    )
    .unwrap();

    assert_eq!(sample.stamp(), receipt.stamp());
}

#[test]
fn reseal_under_same_or_changed_cadence_starts_a_fresh_temporal_incarnation() {
    let (namespace, _subject, _anchor) = endpoint_namespace();
    let cadence_a = LocalFixedSimulationCadence::new(1.0 / 64.0).unwrap();
    let cadence_b = LocalFixedSimulationCadence::new(1.0 / 60.0).unwrap();
    let mut first = seal(namespace, cadence_a);
    let first_receipt = first.step_authorized_fixed().unwrap();

    let namespace = match first.into_namespace() {
        Ok(namespace) => namespace,
        Err(_) => panic!("clean fixed-cadence authority unexpectedly quarantined"),
    };
    let mut same = seal(namespace, cadence_a);
    let same_receipt = same.step_authorized_fixed().unwrap();
    assert_eq!(first_receipt.stamp().step_index(), 1);
    assert_eq!(same_receipt.stamp().step_index(), 1);
    assert_ne!(
        first_receipt.stamp().temporal_incarnation_id(),
        same_receipt.stamp().temporal_incarnation_id()
    );

    let namespace = match same.into_namespace() {
        Ok(namespace) => namespace,
        Err(_) => panic!("clean fixed-cadence authority unexpectedly quarantined"),
    };
    let mut changed = seal(namespace, cadence_b);
    let changed_receipt = changed.step_authorized_fixed().unwrap();
    assert_eq!(changed_receipt.stamp().step_index(), 1);
    assert_eq!(changed_receipt.dt_bits(), cadence_b.dt_bits());
    assert_ne!(changed_receipt.dt_bits(), cadence_a.dt_bits());
    assert_ne!(
        same_receipt.stamp().temporal_incarnation_id(),
        changed_receipt.stamp().temporal_incarnation_id()
    );
}
