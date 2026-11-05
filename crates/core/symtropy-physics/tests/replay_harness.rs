// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use nalgebra::SVector;
use symtropy_math::{Bivector, Point, Rotor, Sphere, Transform};
use symtropy_physics::PhysicsWorld;
use symtropy_physics::body::{BodyHandle, BodyType, RigidBody};
use symtropy_physics::replay::{
    ReplayEvidenceContext, ReplayEvidenceError, ReplayTape, WorldCommand, WorldSnapshot,
    apply_commands, execute_with_evidence, verify_replay_evidence,
};

fn build_world() -> (PhysicsWorld<3>, [BodyHandle; 3]) {
    // Use power-of-two fractions where possible to reduce noise and keep tape "clean".
    let gravity = SVector::from([0.0, -10.0, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    world.solver_iterations = 6;
    world.sleep_threshold = 0.25;
    world.sleep_ticks = 32;

    // Ground: sphere of radius 64 centered at y=-64 → top at y=0.
    let ground = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::new([0.0, -64.0, 0.0]),
        Box::new(Sphere::<3>::new(Point::origin(), 64.0)),
    );
    world.add_body(ground);

    let a = world.add_sphere(Point::new([-1.5, 6.0, 0.0]), 0.5, 1.0);
    let b = world.add_sphere(Point::new([1.5, 6.0, 0.0]), 0.5, 1.0);
    let c = world.add_sphere(Point::new([0.0, 9.0, 0.0]), 0.5, 2.0);

    world.body_mut(a).unwrap().linear_velocity = SVector::from([2.0, 0.0, 0.0]);
    world.body_mut(b).unwrap().linear_velocity = SVector::from([-2.0, 0.0, 0.0]);

    (world, [a, b, c])
}

fn build_tape(handles: [BodyHandle; 3]) -> ReplayTape<3> {
    let [a, b, c] = handles;
    let dt = 1.0 / 64.0;

    let mut tape = ReplayTape::<3>::default();
    for tick in 0..192u32 {
        let fx = ((tick as i32 % 32) - 16) as f64 / 128.0; // exact multiples of 1/128
        let fz = ((tick as i32 % 16) - 8) as f64 / 256.0; // exact multiples of 1/256

        let mut commands = vec![
            WorldCommand::ApplyForce {
                body: a,
                force: Box::new(SVector::from([fx, 0.0, fz])),
            },
            WorldCommand::ApplyForce {
                body: b,
                force: Box::new(SVector::from([-fx, 0.0, -fz])),
            },
        ];

        // Occasionally "nudge" the third body so collision timing depends on commands.
        if tick % 24 == 0 {
            commands.push(WorldCommand::ApplyImpulse {
                body: c,
                impulse: Box::new(SVector::from([0.25, 0.0, 0.0])),
            });
        }

        tape.push_frame(dt, commands);
    }
    tape
}

#[test]
fn record_and_replay_are_bitwise_identical_each_tick() {
    let (mut record_world, handles) = build_world();
    let tape = build_tape(handles);

    let mut recorded: Vec<WorldSnapshot<3>> = Vec::with_capacity(tape.frames.len());
    for frame in &tape.frames {
        apply_commands(&mut record_world, &frame.commands).unwrap();
        record_world.step(frame.dt);
        recorded.push(WorldSnapshot::capture(&record_world));
    }

    let (mut replay_world, _handles2) = build_world();
    for (tick, frame) in tape.frames.iter().enumerate() {
        apply_commands(&mut replay_world, &frame.commands).unwrap();
        replay_world.step(frame.dt);
        let snapshot = WorldSnapshot::capture(&replay_world);
        assert_eq!(snapshot, recorded[tick], "diverged at tick {tick}");
    }
}

fn evidence_context() -> ReplayEvidenceContext {
    ReplayEvidenceContext {
        scenario_id: "physics-replay-harness-v1".into(),
        scenario_artifact_digest: "a".repeat(64),
        engine_artifact_digest: "b".repeat(64),
        initial_world_profile_digest: "c".repeat(64),
        environment_profile_digest: "d".repeat(64),
        execution_protocol_digest: "e".repeat(64),
    }
}

#[test]
fn scenario_bound_receipt_replays_bitwise_in_a_separate_world() {
    let (mut record_world, record_handles) = build_world();
    let record_tape = build_tape(record_handles);
    let context = evidence_context();
    let receipt = execute_with_evidence(&mut record_world, &record_tape, &context).unwrap();

    assert!(receipt.identity_is_valid());
    assert_eq!(receipt.frame_count as usize, record_tape.frames.len());
    assert_eq!(
        receipt.tick_snapshot_digests.last(),
        Some(&receipt.final_snapshot_digest)
    );

    // The second world is independently constructed from the same declared
    // initial-world profile; it is not the mutated record world.
    let (mut replay_world, replay_handles) = build_world();
    let replay_tape = build_tape(replay_handles);
    assert!(verify_replay_evidence(&mut replay_world, &replay_tape, &context, &receipt,).unwrap());
}

#[test]
fn changed_tape_or_context_cannot_reuse_an_old_receipt() {
    let (mut record_world, record_handles) = build_world();
    let tape = build_tape(record_handles);
    let context = evidence_context();
    let receipt = execute_with_evidence(&mut record_world, &tape, &context).unwrap();

    let (mut changed_tape_world, changed_tape_handles) = build_world();
    let mut changed_tape = build_tape(changed_tape_handles);
    changed_tape.frames[0].commands.pop();
    assert!(
        !verify_replay_evidence(&mut changed_tape_world, &changed_tape, &context, &receipt,)
            .unwrap()
    );

    let (mut changed_context_world, changed_context_handles) = build_world();
    let changed_context_tape = build_tape(changed_context_handles);
    let mut changed_context = context;
    changed_context.environment_profile_digest = "f".repeat(64);
    assert!(
        !verify_replay_evidence(
            &mut changed_context_world,
            &changed_context_tape,
            &changed_context,
            &receipt,
        )
        .unwrap()
    );
}

#[test]
fn invalid_timestep_is_rejected_before_world_mutation() {
    let (mut world, handles) = build_world();
    let before = WorldSnapshot::capture(&world);
    let mut tape = build_tape(handles);
    tape.frames[0].dt = f64::NAN;

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidTimeStep
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn missing_body_reference_is_rejected_before_world_mutation() {
    let (mut world, _) = build_world();
    let before = WorldSnapshot::capture(&world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 64.0,
        vec![WorldCommand::Wake {
            body: BodyHandle(usize::MAX),
        }],
    );

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::MissingBody(BodyHandle(usize::MAX))
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn nonfinite_initial_world_state_is_rejected_before_step() {
    let (mut world, handles) = build_world();
    world.body_mut(handles[0]).unwrap().linear_velocity[0] = f64::NAN;
    let before = WorldSnapshot::capture(&world);
    let tape = build_tape(handles);
    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::NonFiniteWorldState
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn malformed_receipt_digest_is_rejected_before_replay() {
    let (mut record_world, handles) = build_world();
    let tape = build_tape(handles);
    let context = evidence_context();
    let mut receipt = execute_with_evidence(&mut record_world, &tape, &context).unwrap();
    receipt.receipt_digest = "0".repeat(64);
    assert!(!receipt.identity_is_valid());

    let (mut replay_world, replay_handles) = build_world();
    let replay_tape = build_tape(replay_handles);
    assert!(!verify_replay_evidence(&mut replay_world, &replay_tape, &context, &receipt,).unwrap());
}


fn add_kinematic_actor(world: &mut PhysicsWorld<3>) -> BodyHandle {
    let actor = RigidBody::<3>::new(
        BodyHandle(0),
        BodyType::Kinematic,
        Transform::from_translation(Point::new([0.0, 0.0, 0.0])),
        Box::new(Sphere::<3>::new(Point::origin(), 0.25)),
        0.0,
        SVector::zeros(),
    );
    world.add_body(actor)
}

fn pose_command(body: BodyHandle, rotation: Rotor<3>, x: f64) -> WorldCommand<3> {
    WorldCommand::SetKinematicPose {
        body,
        translation: Box::new(SVector::from([x, 0.0, 0.0])),
        rotation: Box::new(rotation),
        linear_velocity: Box::new(SVector::from([1.0, 0.0, 0.0])),
        angular_velocity: Box::new(Bivector::<3>::zero()),
    }
}

#[test]
fn kinematic_pose_updates_orientation_and_replays_in_a_fresh_world() {
    let rotation = Rotor::<3>::from_plane_angle(
        &Bivector::<3>::unit_plane(0, 1),
        std::f64::consts::FRAC_PI_2,
    );
    let mut record_world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut record_world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 64.0,
        vec![pose_command(actor, rotation.clone(), 0.125)],
    );

    let receipt = execute_with_evidence(&mut record_world, &tape, &evidence_context()).unwrap();
    let recorded = record_world.body(actor).unwrap();

    assert_eq!(
        *recorded.transform.rotation.to_matrix(),
        *rotation.to_matrix()
    );
    assert_eq!(
        recorded.transform.translation.0,
        SVector::from([0.125, 0.0, 0.0])
    );

    let mut replay_world = PhysicsWorld::<3>::default();
    let replay_actor = add_kinematic_actor(&mut replay_world);
    let replay_tape = ReplayTape::<3> {
        frames: tape.frames.clone(),
    };
    assert!(
        verify_replay_evidence(
            &mut replay_world,
            &replay_tape,
            &evidence_context(),
            &receipt,
        )
        .unwrap()
    );
    assert_eq!(
        replay_world
            .body(replay_actor)
            .unwrap()
            .transform
            .rotation
            .to_matrix(),
        rotation.to_matrix()
    );
}

#[test]
fn kinematic_pose_rejects_reflection_before_world_mutation() {
    let mut world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut world);
    let before = WorldSnapshot::capture(&world);
    let reflection = Rotor::from_matrix(nalgebra::SMatrix::<f64, 3, 3>::from_diagonal(
        &SVector::from([-1.0, 1.0, 1.0]),
    ));
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(1.0 / 64.0, vec![pose_command(actor, reflection, 1.0)]);

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidCommandValue
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn kinematic_pose_rejects_non_finite_orientation_before_world_mutation() {
    let mut world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut world);
    let before = WorldSnapshot::capture(&world);
    let mut matrix = nalgebra::SMatrix::<f64, 3, 3>::identity();
    matrix[(0, 0)] = f64::NAN;
    let invalid = Rotor::from_matrix(matrix);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(1.0 / 64.0, vec![pose_command(actor, invalid, 1.0)]);

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidCommandValue
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn changing_kinematic_orientation_invalidates_the_old_receipt() {
    let context = evidence_context();
    let first_rotation = Rotor::<3>::identity();
    let second_rotation = Rotor::<3>::from_plane_angle(
        &Bivector::<3>::unit_plane(0, 1),
        std::f64::consts::FRAC_PI_4,
    );

    let mut record_world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut record_world);
    let mut original = ReplayTape::<3>::default();
    original.push_frame(1.0 / 64.0, vec![pose_command(actor, first_rotation, 0.125)]);
    let receipt = execute_with_evidence(&mut record_world, &original, &context).unwrap();

    let mut changed_world = PhysicsWorld::<3>::default();
    let changed_actor = add_kinematic_actor(&mut changed_world);
    let mut changed = ReplayTape::<3>::default();
    changed.push_frame(
        1.0 / 64.0,
        vec![pose_command(changed_actor, second_rotation, 0.125)],
    );
    assert!(!verify_replay_evidence(&mut changed_world, &changed, &context, &receipt,).unwrap());
}

fn state_command(body: BodyHandle, x: f64) -> WorldCommand<3> {
    WorldCommand::SetKinematicState {
        body,
        translation: Box::new(SVector::from([x, 0.0, 0.0])),
        linear_velocity: Box::new(SVector::from([1.0, 0.0, 0.0])),
    }
}

#[test]
fn kinematic_translation_state_replays_in_a_fresh_world() {
    let mut record_world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut record_world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(1.0 / 64.0, vec![state_command(actor, 0.125)]);
    let context = evidence_context();
    let receipt = execute_with_evidence(&mut record_world, &tape, &context).unwrap();
    assert_eq!(
        record_world.body(actor).unwrap().transform.translation.0,
        SVector::from([0.125, 0.0, 0.0])
    );

    let mut replay_world = PhysicsWorld::<3>::default();
    let replay_actor = add_kinematic_actor(&mut replay_world);
    assert!(verify_replay_evidence(&mut replay_world, &tape, &context, &receipt,).unwrap());
    assert_eq!(
        replay_world
            .body(replay_actor)
            .unwrap()
            .transform
            .translation
            .0,
        SVector::from([0.125, 0.0, 0.0])
    );
}

#[test]
fn kinematic_translation_state_rejects_non_kinematic_body() {
    let (mut world, handles) = build_world();
    let before = WorldSnapshot::capture(&world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(1.0 / 64.0, vec![state_command(handles[0], 5.0)]);
    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::ExpectedKinematicBody(handles[0])
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn kinematic_translation_state_rejects_non_finite_values() {
    let mut world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut world);
    let before = WorldSnapshot::capture(&world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 64.0,
        vec![WorldCommand::SetKinematicState {
            body: actor,
            translation: Box::new(SVector::from([f64::NAN, 0.0, 0.0])),
            linear_velocity: Box::new(SVector::from([1.0, 0.0, 0.0])),
        }],
    );
    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidCommandValue
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn changed_kinematic_translation_invalidates_old_receipt() {
    let context = evidence_context();
    let mut record_world = PhysicsWorld::<3>::default();
    let actor = add_kinematic_actor(&mut record_world);
    let mut original = ReplayTape::<3>::default();
    original.push_frame(1.0 / 64.0, vec![state_command(actor, 0.125)]);
    let receipt = execute_with_evidence(&mut record_world, &original, &context).unwrap();

    let mut changed_world = PhysicsWorld::<3>::default();
    let changed_actor = add_kinematic_actor(&mut changed_world);
    let mut changed = ReplayTape::<3>::default();
    changed.push_frame(1.0 / 64.0, vec![state_command(changed_actor, 0.25)]);
    assert!(!verify_replay_evidence(
        &mut changed_world,
        &changed,
        &context,
        &receipt,
    )
    .unwrap());
}
