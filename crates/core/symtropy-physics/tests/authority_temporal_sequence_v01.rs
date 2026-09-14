// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use symtropy_math::Point;
use symtropy_physics::{
    NetId, NoOpCallback, PhysicalAuthorityId, PhysicsAuthorityTemporalError,
    PhysicsAuthorityWorld, PhysicsWorld, WorldGenerationId,
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
    assert_eq!(first.mutation_epoch(), 0);
    assert_eq!(first.step_index(), 1);
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let mut callback = NoOpCallback;
    let second = authority
        .step_authorized_with_callback(1.0 / 64.0, &mut callback)
        .unwrap();
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), 2);
    assert_eq!(authority.last_authorized_step_stamp(), Some(second));
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
    let before = authority.step_authorized(1.0 / 64.0).unwrap();

    authority.world_mut().gravity[0] = 1.0;
    assert_eq!(authority.last_authorized_step_stamp(), None);

    let after = authority.step_authorized(1.0 / 64.0).unwrap();
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
    assert_eq!(after_bind.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after_bind.step_index(), 1);

    let same_subject = authority.bind_net_id(handle, NetId(9801)).unwrap();
    assert_eq!(same_subject, subject);
    assert_eq!(authority.last_authorized_step_stamp(), Some(after_bind));

    let after_idempotent = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(after_idempotent.mutation_epoch(), after_bind.mutation_epoch());
    assert_eq!(after_idempotent.step_index(), 2);
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
    assert_eq!(after.mutation_epoch(), before.mutation_epoch() + 1);
    assert_eq!(after.step_index(), 1);
}

#[test]
fn empty_deterministic_insertion_preserves_temporal_lineage() {
    let mut authority = empty_authority();
    let first = authority.step_authorized(1.0 / 64.0).unwrap();

    let inserted = authority.add_bodies_deterministic(Vec::new()).unwrap();
    assert!(inserted.is_empty());
    assert_eq!(authority.last_authorized_step_stamp(), Some(first));

    let second = authority.step_authorized(1.0 / 64.0).unwrap();
    assert_eq!(second.mutation_epoch(), first.mutation_epoch());
    assert_eq!(second.step_index(), 2);
}
