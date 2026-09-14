// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::panic::{AssertUnwindSafe, catch_unwind};

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, CollisionEvent, LocalEvidenceAuthorityError, LocalEvidencePhysicsAuthorityWorld,
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedPhysicalAuthority, NetId, NoOpCallback,
    PhysicsBodySubject, PhysicsCallback, PhysicsWorld,
};

fn empty_namespace() -> LocalNamespacePhysicsAuthorityWorld<3> {
    let mut root = LocalQualifiedPhysicalAuthority::mint().expect("root should mint");
    let generation = root.mint_generation().expect("generation should mint");
    LocalNamespacePhysicsAuthorityWorld::bind(generation, PhysicsWorld::<3>::default())
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

fn bound_namespace(
    net_id: NetId,
) -> (
    LocalNamespacePhysicsAuthorityWorld<3>,
    PhysicsBodySubject,
    BodyHandle,
) {
    let mut namespace = empty_namespace();
    let handle = namespace
        .authority_world_mut()
        .world_mut()
        .add_sphere(Point::origin(), 0.5, 1.0);
    let subject = namespace
        .authority_world_mut()
        .bind_net_id(handle, net_id)
        .expect("bind should succeed");
    (namespace, subject, handle)
}

#[test]
fn seal_breaks_legacy_temporal_lineage_and_starts_without_a_qualified_step() {
    let mut namespace = empty_namespace();
    let old = namespace
        .authority_world_mut()
        .step_authorized(1.0 / 64.0)
        .expect("legacy step should succeed");
    assert_eq!(
        namespace.authority_world().last_authorized_step_stamp(),
        Some(old)
    );

    let evidence = seal(namespace);
    assert_eq!(evidence.authority_world().last_authorized_step_stamp(), None);
    assert_eq!(evidence.last_qualified_step_stamp(), None);
}

#[test]
fn clean_resealing_always_mints_a_fresh_temporal_incarnation() {
    let first = seal(empty_namespace());
    let first_id = first.temporal_incarnation_id();
    let namespace = clean_namespace(first);
    let second = seal(namespace);

    assert_ne!(first_id, second.temporal_incarnation_id());
}

#[test]
fn qualified_steps_are_monotonic_and_callback_path_shares_outer_sequence() {
    let mut evidence = seal(empty_namespace());
    let first = evidence.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(first.step_index(), 1);
    assert_eq!(first.physical_authority_id(), evidence.physical_authority_id());
    assert_eq!(first.world_generation_id(), evidence.world_generation_id());
    assert_eq!(
        first.temporal_incarnation_id(),
        evidence.temporal_incarnation_id()
    );
    assert_eq!(evidence.last_qualified_step_stamp(), Some(first));

    let mut callback = NoOpCallback;
    let second = evidence
        .step_authorized_with_callback(1.0 / 64.0, &mut callback)
        .unwrap();
    assert_eq!(second.step_index(), 2);
    assert_eq!(
        second.temporal_incarnation_id(),
        first.temporal_incarnation_id()
    );
    assert_eq!(evidence.last_qualified_step_stamp(), Some(second));
}

#[test]
fn invalid_dt_rejects_without_tainting_outer_sequence() {
    let mut evidence = seal(empty_namespace());

    assert_eq!(
        evidence.step_authorized(f64::NAN),
        Err(LocalEvidenceAuthorityError::NonFiniteDeltaTime)
    );
    assert_eq!(
        evidence.step_authorized(0.0),
        Err(LocalEvidenceAuthorityError::NonPositiveDeltaTime)
    );
    assert_eq!(
        evidence.step_authorized(-1.0),
        Err(LocalEvidenceAuthorityError::NonPositiveDeltaTime)
    );
    assert_eq!(evidence.last_qualified_step_stamp(), None);

    let first = evidence.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(first.step_index(), 1);
}

#[test]
fn equal_numeric_step_indexes_in_different_incarnations_are_distinct_stamps() {
    let mut first = seal(empty_namespace());
    let first_stamp = first.step_authorized(1.0 / 64.0).unwrap();
    let namespace = clean_namespace(first);

    let mut second = seal(namespace);
    let second_stamp = second.step_authorized(1.0 / 64.0).unwrap();

    assert_eq!(first_stamp.step_index(), 1);
    assert_eq!(second_stamp.step_index(), 1);
    assert_ne!(
        first_stamp.temporal_incarnation_id(),
        second_stamp.temporal_incarnation_id()
    );
    assert_ne!(first_stamp, second_stamp);
}

#[test]
fn qualified_subject_validation_remains_available_while_sealed() {
    let net_id = NetId(8101);
    let (namespace, subject, handle) = bound_namespace(net_id);
    let evidence = seal(namespace);

    let validated = evidence
        .validate_subject(subject)
        .expect("qualified validation should succeed");
    assert_eq!(validated.subject(), subject);
    assert_eq!(validated.net_id(), net_id);
    assert_eq!(validated.runtime_handle(), handle);
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
fn caught_inner_panic_quarantines_namespace_instead_of_permitting_reseal() {
    let (namespace, _, _) = bound_namespace(NetId(8102));
    let mut evidence = seal(namespace);
    let authority = evidence.physical_authority_id();
    let generation = evidence.world_generation_id();
    let mut callback = PanicOnForce;

    let unwind = catch_unwind(AssertUnwindSafe(|| {
        let _ = evidence.step_authorized_with_callback(1.0 / 64.0, &mut callback);
    }));
    assert!(unwind.is_err());
    assert_eq!(evidence.last_qualified_step_stamp(), None);
    assert_eq!(
        evidence.step_authorized(1.0 / 64.0),
        Err(LocalEvidenceAuthorityError::InterruptedStepTainted)
    );

    let quarantined = match evidence.into_namespace() {
        Ok(_) => panic!("tainted evidence must not regain qualified namespace authority"),
        Err(quarantined) => quarantined,
    };
    assert_eq!(quarantined.physical_authority_id(), authority);
    assert_eq!(quarantined.world_generation_id(), generation);

    let unqualified = quarantined.into_unqualified();
    assert_eq!(unqualified.physical_authority_id(), authority);
    assert_eq!(unqualified.world_generation_id(), generation);
}
