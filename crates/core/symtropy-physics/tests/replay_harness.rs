// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use nalgebra::{SMatrix, SVector};
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

fn build_kinematic_world() -> (PhysicsWorld<3>, BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let body = RigidBody::<3>::new(
        BodyHandle(0),
        BodyType::Kinematic,
        Transform::identity(),
        Box::new(Sphere::<3>::new(Point::origin(), 0.5)),
        0.0,
        SVector::zeros(),
    );
    let handle = world.add_body(body);
    (world, handle)
}

fn pose_command(handle: BodyHandle, translation: [f64; 3], rotation: Rotor<3>) -> WorldCommand<3> {
    WorldCommand::SetKinematicPose {
        body: handle,
        transform: Box::new(Transform {
            translation: Point::new(translation),
            rotation,
        }),
        linear_velocity: Box::new(SVector::from([0.5, 0.0, -0.25])),
        angular_velocity: Box::new(Bivector::<3>::unit_plane(1, 2).scale(0.125)),
    }
}

#[test]
fn full_kinematic_pose_command_replays_in_an_independent_world() {
    let (mut record_world, record_handle) = build_kinematic_world();
    let plane = Bivector::<3>::unit_plane(0, 1);
    let rotation = Rotor::from_plane_angle(&plane, 0.375);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(
            record_handle,
            [1.25, 2.5, -0.75],
            rotation.clone(),
        )],
    );
    let context = evidence_context();
    let receipt = execute_with_evidence(&mut record_world, &tape, &context).unwrap();

    assert_eq!(
        record_world.body(record_handle).unwrap().body_type,
        BodyType::Kinematic
    );
    let snapshot = WorldSnapshot::capture(&record_world);
    let recorded_body = snapshot
        .bodies
        .iter()
        .find(|body| body.handle == record_handle)
        .unwrap();
    assert_eq!(
        recorded_body.translation,
        [1.25_f64.to_bits(), 2.5_f64.to_bits(), (-0.75_f64).to_bits()]
    );
    for row in 0..3 {
        for column in 0..3 {
            assert_eq!(
                recorded_body.rotation[row][column],
                rotation.to_matrix()[(row, column)].to_bits()
            );
        }
    }

    let (mut replay_world, replay_handle) = build_kinematic_world();
    let mut replay_tape = ReplayTape::<3>::default();
    replay_tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(replay_handle, [1.25, 2.5, -0.75], rotation)],
    );
    assert!(verify_replay_evidence(&mut replay_world, &replay_tape, &context, &receipt).unwrap());
}

#[test]
fn kinematic_pose_command_rejects_dynamic_body_before_mutation() {
    let (mut world, handles) = build_world();
    let before = WorldSnapshot::capture(&world);
    let plane = Bivector::<3>::unit_plane(0, 1);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(
            handles[0],
            [1.0, 2.0, 3.0],
            Rotor::from_plane_angle(&plane, 0.25),
        )],
    );

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::NonKinematicBody(handles[0])
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn kinematic_pose_command_rejects_reflection_rotation_before_mutation() {
    let (mut world, handle) = build_kinematic_world();
    let before = WorldSnapshot::capture(&world);
    let reflection = Rotor::from_matrix(SMatrix::<f64, 3, 3>::from_diagonal(&SVector::from([
        -1.0, 1.0, 1.0,
    ])));
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(handle, [1.0, 2.0, 3.0], reflection)],
    );

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidRotation(handle)
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn kinematic_pose_command_rejects_nonfinite_payload_before_mutation() {
    let (mut world, handle) = build_kinematic_world();
    let before = WorldSnapshot::capture(&world);
    let mut tape = ReplayTape::<3>::default();
    tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(
            handle,
            [f64::NAN, 2.0, 3.0],
            Rotor::identity(),
        )],
    );

    assert_eq!(
        execute_with_evidence(&mut world, &tape, &evidence_context()).unwrap_err(),
        ReplayEvidenceError::InvalidCommandValue
    );
    assert_eq!(WorldSnapshot::capture(&world), before);
}

#[test]
fn kinematic_pose_payload_changes_replay_tape_digest() {
    let (mut first_world, first_handle) = build_kinematic_world();
    let (mut second_world, second_handle) = build_kinematic_world();
    let context = evidence_context();

    let mut first_tape = ReplayTape::<3>::default();
    first_tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(
            first_handle,
            [1.0, 2.0, 3.0],
            Rotor::identity(),
        )],
    );
    let first = execute_with_evidence(&mut first_world, &first_tape, &context).unwrap();

    let plane = Bivector::<3>::unit_plane(0, 1);
    let mut second_tape = ReplayTape::<3>::default();
    second_tape.push_frame(
        1.0 / 120.0,
        vec![pose_command(
            second_handle,
            [1.0, 2.0, 3.0],
            Rotor::from_plane_angle(&plane, 0.1),
        )],
    );
    let second = execute_with_evidence(&mut second_world, &second_tape, &context).unwrap();

    assert_ne!(first.input_tape_digest, second.input_tape_digest);
    assert_ne!(first.receipt_digest, second.receipt_digest);
}
