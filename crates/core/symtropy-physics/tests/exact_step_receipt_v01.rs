// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::panic::{AssertUnwindSafe, catch_unwind};

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    AuthorityEndpointBoxSpec, BodyHandle, CollisionEvent, LocalEvidenceAuthorityError,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority,
    LocalQualifiedStampedEndpointBoxObservation, LocalReceiptedEvidencePhysicsAuthorityWorld,
    NetId, NoOpCallback, PhysicsBodySubject, PhysicsCallback, PhysicsWorld,
};

fn empty_namespace() -> LocalNamespacePhysicsAuthorityWorld<3> {
    let mut root = LocalQualifiedPhysicalAuthority::mint().unwrap();
    let generation = root.mint_generation().unwrap();
    LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default())
}

fn endpoint_namespace() -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    PhysicsBodySubject,
) {
    let mut namespace = empty_namespace();
    let anchor_handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::origin(), 0.1, 1.0);
    let subject_handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
    let anchor = namespace
        .authority_world_mut()
        .bind_net_id(anchor_handle, NetId(8501))
        .unwrap();
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(subject_handle, NetId(8502))
        .unwrap();
    (namespace, subject, anchor)
}

fn seal(
    namespace: LocalNamespacePhysicsAuthorityWorld<3>,
) -> LocalReceiptedEvidencePhysicsAuthorityWorld<3> {
    match LocalReceiptedEvidencePhysicsAuthorityWorld::seal(namespace) {
        Ok(timed) => timed,
        Err(failure) => panic!("seal failed: {:?}", failure.error()),
    }
}

#[test]
fn invalid_dt_values_mint_no_receipt_and_preserve_first_step_index() {
    let mut timed = seal(empty_namespace());

    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            timed.step_authorized(invalid),
            Err(LocalEvidenceAuthorityError::NonFiniteDeltaTime)
        );
    }
    for invalid in [0.0, -0.0, -1.0] {
        assert_eq!(
            timed.step_authorized(invalid),
            Err(LocalEvidenceAuthorityError::NonPositiveDeltaTime)
        );
    }
    assert_eq!(timed.last_qualified_step_stamp(), None);

    let receipt = timed.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(receipt.stamp().step_index(), 1);
}

#[test]
fn pure_step_receipt_binds_exact_executed_binary64_value() {
    let mut timed = seal(empty_namespace());
    let dt = 0.015625_f64;
    let receipt = timed.step_authorized(dt).unwrap();

    assert_eq!(receipt.dt_bits(), dt.to_bits());
    assert_eq!(receipt.executed_dt().to_bits(), dt.to_bits());
    assert_eq!(timed.last_qualified_step_stamp(), Some(receipt.stamp()));
}

#[test]
fn callback_path_uses_the_same_exact_receipt_rule() {
    let mut timed = seal(empty_namespace());
    let dt = 1.0_f64 / 60.0;
    let mut callback = NoOpCallback;
    let receipt = timed
        .step_authorized_with_callback(dt, &mut callback)
        .unwrap();

    assert_eq!(receipt.dt_bits(), dt.to_bits());
    assert_eq!(receipt.stamp().step_index(), 1);
    assert_eq!(timed.last_qualified_step_stamp(), Some(receipt.stamp()));
}

#[test]
fn successive_distinct_dt_values_remain_exact_and_step_bound() {
    let mut timed = seal(empty_namespace());
    let first_dt = 1.0_f64 / 64.0;
    let second_dt = 1.0_f64 / 60.0;
    let first = timed.step_authorized(first_dt).unwrap();
    let second = timed.step_authorized(second_dt).unwrap();

    assert_eq!(first.stamp().step_index(), 1);
    assert_eq!(second.stamp().step_index(), 2);
    assert_eq!(first.dt_bits(), first_dt.to_bits());
    assert_eq!(second.dt_bits(), second_dt.to_bits());
    assert_ne!(first.dt_bits(), second.dt_bits());
}

#[test]
fn endpoint_capture_after_receipted_step_binds_the_same_strong_stamp() {
    let (namespace, subject, anchor) = endpoint_namespace();
    let mut timed = seal(namespace);
    let receipt = timed.step_authorized(1.0 / 64.0).unwrap();
    let endpoint = AuthorityEndpointBoxSpec::new(anchor, [0.0; 3], [2.0; 3]).unwrap();
    let sample = LocalQualifiedStampedEndpointBoxObservation::capture(
        timed.evidence_authority(),
        subject,
        endpoint,
    )
    .unwrap();

    assert_eq!(sample.stamp(), receipt.stamp());
}

#[test]
fn clean_exit_and_reseal_breaks_receipt_temporal_lineage() {
    let mut first = seal(empty_namespace());
    let first_receipt = first.step_authorized(1.0 / 64.0).unwrap();
    let namespace = match first.into_namespace() {
        Ok(namespace) => namespace,
        Err(_) => panic!("clean timed evidence unexpectedly quarantined"),
    };

    let mut second = seal(namespace);
    let second_receipt = second.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(first_receipt.stamp().step_index(), 1);
    assert_eq!(second_receipt.stamp().step_index(), 1);
    assert_ne!(
        first_receipt.stamp().temporal_incarnation_id(),
        second_receipt.stamp().temporal_incarnation_id()
    );
    assert_ne!(first_receipt.stamp(), second_receipt.stamp());
}

struct PanicOnForce;

impl PhysicsCallback<3> for PanicOnForce {
    fn modulate_force(&self, _: BodyHandle, _: &SVector<f64, 3>) -> SVector<f64, 3> {
        panic!("intentional callback panic");
    }

    fn modulate_impulse(&self, impulse: f64, _: &SVector<f64, 3>) -> f64 {
        impulse
    }

    fn friction_multiplier(&self, _: &SVector<f64, 3>, _: BodyHandle) -> f64 {
        1.0
    }

    fn on_collision(&mut self, _: &CollisionEvent<3>) {}

    fn record_dissipation(&mut self, _: f64) {}

    fn record_work(&mut self, _: BodyHandle, _: f64) {}

    fn apply_trauma(&mut self, _: &CollisionEvent<3>) {}
}

#[test]
fn caught_callback_panic_mints_no_receipt_and_preserves_quarantine() {
    let mut namespace = empty_namespace();
    let handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::origin(), 0.5, 1.0);
    namespace
        .authority_world_mut()
        .bind_net_id(handle, NetId(8503))
        .unwrap();
    let mut timed = seal(namespace);
    let authority = timed.physical_authority_id();
    let generation = timed.world_generation_id();
    let mut callback = PanicOnForce;

    let unwind = catch_unwind(AssertUnwindSafe(|| {
        let _ = timed.step_authorized_with_callback(1.0 / 64.0, &mut callback);
    }));
    assert!(unwind.is_err());
    assert_eq!(timed.last_qualified_step_stamp(), None);
    assert_eq!(
        timed.step_authorized(1.0 / 64.0),
        Err(LocalEvidenceAuthorityError::InterruptedStepTainted)
    );

    let quarantine = match timed.into_namespace() {
        Ok(_) => panic!("tainted receipted evidence must not regain namespace authority"),
        Err(quarantine) => quarantine,
    };
    assert_eq!(quarantine.physical_authority_id(), authority);
    assert_eq!(quarantine.world_generation_id(), generation);
}
