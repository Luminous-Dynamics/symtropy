// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    BodyHandle, NetId, NetIdentityMutationError, NoOpCallback, PhysicalAuthorityId,
    PhysicsAuthorityTemporalError, PhysicsAuthorityWorld, PhysicsWorld, WorldGenerationId,
};

fn empty_authority() -> PhysicsAuthorityWorld<3> {
    PhysicsAuthorityWorld::new(
        PhysicalAuthorityId::new(0x710E).unwrap(),
        WorldGenerationId::new(23).unwrap(),
        PhysicsWorld::<3>::default(),
    )
}

#[test]
fn authorized_steps_are_monotonic_and_callback_path_shares_sequence() {
    let mut authority = empty_authority();
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let first = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(first.physical_authority_id(), authority.physical_authority_id());
    assert_eq!(first.world_generation_id(), authority.world_generation_id());
    assert_eq!(
        first.temporal_incarnation_id(),
        authority.temporal_incarnation_id()
    );
    assert_eq!(first.mutation_epoch(), 0);
    assert_eq!(first.step_index(), 1);
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let mut callback = NoOpCallback;
    let second = authority
        .step_authorized_with_callback(1.0 / 64.0, &mut callback)
        .unwrap();
    assert_eq!(second.temporal_incarnation_id(), first.temporal_incarnation_id());
    assert!(first.same_temporal_lineage(second));
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), 2);
    assert_eq!(authority.last_authorized_step_stamp(), Some(second));
}

#[test]
fn independently_constructed_wrappers_cannot_alias_full_stamps() {
    let authority_id = PhysicalAuthorityId::new(0x710E).unwrap();
    let generation_id = WorldGenerationId::new(70).unwrap();
    let mut first = PhysicsAuthorityWorld::new(
        authority_id,
        generation_id,
        PhysicsWorld::<3>::default(),
    );
    let mut second = PhysicsAuthorityWorld::new(
        authority_id,
        generation_id,
        PhysicsWorld::<3>::default(),
    );

    let first_stamp = first.step_authorized(1.0 / 64.0).unwrap();
    let second_stamp = second.step_authorized(1.0 / 64.0).unwrap();

    assert_eq!(first_stamp.mutation_epoch(), second_stamp.mutation_epoch());
    assert_eq!(first_stamp.step_index(), second_stamp.step_index());
    assert_ne!(
        first_stamp.temporal_incarnation_id(),
        second_stamp.temporal_incarnation_id()
    );
    assert_ne!(first_stamp, second_stamp);
    assert!(!first_stamp.same_temporal_lineage(second_stamp));
}

#[test]
fn rewrapping_same_raw_world_mints_fresh_temporal_incarnation() {
    let authority_id = PhysicalAuthorityId::new(0x710E).unwrap();
    let generation_id = WorldGenerationId::new(71).unwrap();
    let mut first = PhysicsAuthorityWorld::new(
        authority_id,
        generation_id,
        PhysicsWorld::<3>::default(),
    );
    let first_stamp = first.step_authorized(1.0 / 64.0).unwrap();
    let first_incarnation = first.temporal_incarnation_id();

    let raw = first.into_world();
    let mut second = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);
    let second_stamp = second.step_authorized(1.0 / 64.0).unwrap();

    assert_ne!(first_incarnation, second.temporal_incarnation_id());
    assert_eq!(first_stamp.mutation_epoch(), second_stamp.mutation_epoch());
    assert_eq!(first_stamp.step_index(), second_stamp.step_index());
    assert_ne!(first_stamp, second_stamp);
    assert!(!first_stamp.same_temporal_lineage(second_stamp));
}

#[test]
fn invalid_delta_time_issues_no_new_stamp() {
    let mut authority = empty_authority();

    assert_eq!(
        authority.step_authorized(f64::NAN),
        Err(PhysicsAuthorityTemporalError::NonFiniteDeltaTime)
    );
    assert_eq!(authority.last_authorized_step_stamp(), None);
    assert_eq!(
        authority.step_authorized(0.0),
        Err(PhysicsAuthorityTemporalError::NonPositiveDeltaTime)
    );
    assert_eq!(
        authority.step_authorized(-1.0),
        Err(PhysicsAuthorityTemporalError::NonPositiveDeltaTime)
    );

    let valid = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(valid.step_index(), 1);

    assert_eq!(
        authority.step_authorized(f64::INFINITY),
        Err(PhysicsAuthorityTemporalError::NonFiniteDeltaTime)
    );
    assert_eq!(authority.last_authorized_step_stamp(), Some(valid));
}

#[test]
fn raw_world_mut_breaks_epoch_and_invalidates_last_step() {
    let mut authority = empty_authority();
    let incarnation = authority.temporal_incarnation_id();
    let before = authority.step_authorized(1.0 / 64.0).unwrap();

    authority.world_mut().gravity[0] = 1.0;
    assert_eq!(authority.temporal_incarnation_id(), incarnation);
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let after = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(after.temporal_incarnation_id(), incarnation);
    assert!(!before.same_temporal_lineage(after));
    assert_eq!(after.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after.step_index(), 1);
}

#[test]
fn try_world_mut_has_the_same_explicit_lineage_break() {
    let mut authority = empty_authority();
    let before = authority.step_authorized(1.0 / 64.0).unwrap();

    authority.try_world_mut().unwrap().gravity[1] = -2.0;
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let after = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(
        after.temporal_incarnation_id(),
        before.temporal_incarnation_id()
    );
    assert_eq!(after.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after.step_index(), 1);
}

#[test]
fn identity_change_breaks_epoch_but_idempotent_rebind_does_not() {
    let authority_id = PhysicalAuthorityId::new(0x710E).unwrap();
    let generation_id = WorldGenerationId::new(24).unwrap();
    let mut raw = PhysicsWorld::<3>::default();
    let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
    let mut authority = PhysicsAuthorityWorld::new(authority_id, generation_id, raw);

    let before = authority.step_authorized(1.0 / 64.0).unwrap();
    let subject = authority.bind_net_id(handle, NetId(9801)).unwrap();
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let after_bind = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(
        after_bind.temporal_incarnation_id(),
        before.temporal_incarnation_id()
    );
    assert_eq!(after_bind.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after_bind.step_index(), 1);

    let same_subject = authority.bind_net_id(handle, NetId(9801)).unwrap();
    assert_eq!(same_subject, subject);
    assert_eq!(authority.last_authorized_step_stamp(), Some(after_bind));

    let after_idempotent = authority.step_authorized(1.0 / 64.0).unwrap();
    assert!(after_bind.same_temporal_lineage(after_idempotent));
    assert_eq!(after_idempotent.mutation_epoch(), after_bind.mutation_epoch());
    assert_eq!(after_idempotent.step_index(), 2);
}

#[test]
fn rejected_identity_bind_preserves_temporal_lineage() {
    let mut authority = empty_authority();
    let first = authority.step_authorized(1.0 / 64.0).unwrap();
    let missing = BodyHandle(999_999);

    assert_eq!(
        authority.bind_net_id(missing, NetId(9802)),
        Err(NetIdentityMutationError::UnknownHandle { handle: missing })
    );
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let second = authority.step_authorized(1.0 / 64.0).unwrap();
    assert!(first.same_temporal_lineage(second));
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), first.step_index() + 1);
}

#[test]
fn nonempty_deterministic_insertion_breaks_epoch() {
    let mut authority = empty_authority();
    let before = authority.step_authorized(1.0 / 64.0).unwrap();

    let body = symtropy_physics::RigidBody::<3>::dynamic_sphere(
        symtropy_physics::BodyHandle(999),
        Point::origin(),
        0.5,
        1.0,
    );
    authority
        .add_bodies_deterministic(vec![(NetId(9901), body)])
        .unwrap();
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let after = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(
        after.temporal_incarnation_id(),
        before.temporal_incarnation_id()
    );
    assert_eq!(after.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after.step_index(), 1);
}

#[test]
fn rejected_deterministic_batch_preserves_temporal_lineage() {
    let mut authority = empty_authority();
    let first = authority.step_authorized(1.0 / 64.0).unwrap();
    let net_id = NetId(9902);
    let first_body = symtropy_physics::RigidBody::<3>::dynamic_sphere(
        BodyHandle(10),
        Point::origin(),
        0.5,
        1.0,
    );
    let second_body = symtropy_physics::RigidBody::<3>::dynamic_sphere(
        BodyHandle(11),
        Point::new([1.0, 0.0, 0.0]),
        0.5,
        1.0,
    );

    assert_eq!(
        authority.add_bodies_deterministic(vec![(net_id, first_body), (net_id, second_body)]),
        Err(NetIdentityMutationError::DuplicateBatchNetId { net_id })
    );
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let second = authority.step_authorized(1.0 / 64.0).unwrap();
    assert!(first.same_temporal_lineage(second));
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), first.step_index() + 1);
}

#[test]
fn empty_deterministic_insertion_preserves_temporal_lineage() {
    let mut authority = empty_authority();
    let first = authority.step_authorized(1.0 / 64.0).unwrap();

    let inserted = authority.add_bodies_deterministic(Vec::new()).unwrap();
    assert!(inserted.is_empty());
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let second = authority.step_authorized(1.0 / 64.0).unwrap();
    assert!(first.same_temporal_lineage(second));
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), 2);
}
