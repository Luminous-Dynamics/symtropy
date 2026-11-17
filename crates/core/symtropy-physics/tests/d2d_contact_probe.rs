// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Dream-to-Discovery contact-probe fixture.
//!
//! This test exercises the native Symtropy rigid-body contact solver with paired
//! friction/probe settings and the existing replay tape/snapshot contract.
//!
//! The static floor is represented by the same large-sphere approximation used
//! in replay_harness.rs. This is a deliberately local contact fixture, not an
//! infinite plane, general manipulator, real material calibration, or robot
//! safety test.

use nalgebra::SVector;
use symtropy_math::{Point, Sphere};
use symtropy_physics::body::{BodyHandle, RigidBody};
use symtropy_physics::replay::{ReplayTape, WorldCommand, WorldSnapshot, apply_commands};
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 240.0;
const STEPS: usize = 240;
const GRAVITY_Y: f64 = -9.81;
const FLOOR_RADIUS: f64 = 64.0;
const FLOOR_CENTER_Y: f64 = -64.0;
const BALL_RADIUS: f64 = 0.5;
const BALL_MASS: f64 = 1.0;

/// One frozen contact-probe experiment specification.
///
/// surface_friction is the intended effective Coulomb coefficient. The
/// dynamic ball is assigned coefficient 1.0 and the floor coefficient is
/// surface_friction squared because the current solver combines body
/// coefficients as sqrt(mu_a * mu_b).
#[derive(Clone, Copy, Debug)]
struct ProbeSpec {
    surface_friction: f64,
    tangential_impulse: f64,
}

impl ProbeSpec {
    fn validate(self) {
        assert!(
            self.surface_friction.is_finite()
                && (0.0..=1.0).contains(&self.surface_friction),
            "surface friction must be finite and in [0, 1]"
        );
        assert!(
            self.tangential_impulse.is_finite() && self.tangential_impulse >= 0.0,
            "tangential impulse must be finite and non-negative"
        );
    }
}

/// Simulation-relative observation returned by the contact-probe execution.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ProbeOutcome {
    terminal_x: f64,
    terminal_y: f64,
    terminal_vx: f64,
    terminal_vy: f64,
    collision_event_count: usize,
}

fn build_world(surface_friction: f64) -> (PhysicsWorld<3>, BodyHandle) {
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, GRAVITY_Y, 0.0]));
    world.solver_iterations = 8;
    world.sleep_threshold = 0.0;
    world.sleep_ticks = u32::MAX;

    // Explicit finite-radius spherical floor, matching the established replay
    // fixture. The contact geometry is therefore curved, not an infinite plane.
    let floor = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::new([0.0, FLOOR_CENTER_Y, 0.0]),
        Box::new(Sphere::<3>::new(Point::origin(), FLOOR_RADIUS)),
    );
    let floor_handle = world.add_body(floor);
    world.body_mut(floor_handle).expect("floor inserted").friction =
        surface_friction * surface_friction;

    let ball = world.add_sphere(
        Point::new([0.0, BALL_RADIUS, 0.0]),
        BALL_RADIUS,
        BALL_MASS,
    );
    {
        let body = world.body_mut(ball).expect("ball inserted");
        body.friction = 1.0;
        body.restitution = 0.0;
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    (world, ball)
}

fn build_tape(body: BodyHandle, tangential_impulse: f64) -> ReplayTape<3> {
    let mut tape = ReplayTape::<3>::default();
    for tick in 0..STEPS {
        let commands = if tick == 0 {
            vec![WorldCommand::ApplyImpulse {
                body,
                impulse: Box::new(SVector::from([tangential_impulse, 0.0, 0.0])),
            }]
        } else {
            Vec::new()
        };
        tape.push_frame(DT, commands);
    }
    tape
}

fn execute_probe(spec: ProbeSpec) -> (Vec<WorldSnapshot<3>>, ProbeOutcome) {
    spec.validate();
    let (mut world, ball) = build_world(spec.surface_friction);
    let tape = build_tape(ball, spec.tangential_impulse);

    let mut snapshots = Vec::with_capacity(tape.frames.len());
    let mut collision_event_count = 0usize;
    for frame in &tape.frames {
        apply_commands(&mut world, &frame.commands).expect("probe body exists");
        world.step(frame.dt);
        collision_event_count += world.collision_events.len();
        snapshots.push(WorldSnapshot::capture(&world));
    }

    let body = world.body(ball).expect("ball remains in world");
    let position = body.position();
    let outcome = ProbeOutcome {
        terminal_x: position[0],
        terminal_y: position[1],
        terminal_vx: body.linear_velocity[0],
        terminal_vy: body.linear_velocity[1],
        collision_event_count,
    };
    (snapshots, outcome)
}

#[test]
fn same_frozen_contact_probe_replays_bitwise_at_each_tick() {
    let spec = ProbeSpec {
        surface_friction: 0.45,
        tangential_impulse: 3.0,
    };

    let (recorded, recorded_outcome) = execute_probe(spec);
    let (replayed, replayed_outcome) = execute_probe(spec);

    assert_eq!(recorded.len(), STEPS);
    assert_eq!(recorded, replayed, "body/event snapshot diverged during replay");
    assert_eq!(recorded_outcome, replayed_outcome);
    assert!(
        recorded_outcome.collision_event_count > 0,
        "the probe must exercise contact, not free flight only"
    );
}

#[test]
fn surface_friction_change_changes_the_executed_probe_observation() {
    let (low_replay, low) = execute_probe(ProbeSpec {
        surface_friction: 0.05,
        tangential_impulse: 3.0,
    });
    let (high_replay, high) = execute_probe(ProbeSpec {
        surface_friction: 0.90,
        tangential_impulse: 3.0,
    });

    assert_eq!(low_replay.len(), STEPS);
    assert_eq!(high_replay.len(), STEPS);
    assert!(
        (low.terminal_vx - high.terminal_vx).abs() > 1e-5
            || (low.terminal_x - high.terminal_x).abs() > 1e-5,
        "the declared friction intervention should change the simulated outcome: low={low:?}, high={high:?}"
    );
}

#[test]
fn tangential_probe_magnitude_changes_the_executed_observation() {
    let (_, gentle) = execute_probe(ProbeSpec {
        surface_friction: 0.45,
        tangential_impulse: 1.0,
    });
    let (_, strong) = execute_probe(ProbeSpec {
        surface_friction: 0.45,
        tangential_impulse: 3.0,
    });

    assert!(
        (gentle.terminal_vx - strong.terminal_vx).abs() > 1e-5
            || (gentle.terminal_x - strong.terminal_x).abs() > 1e-5,
        "the declared probe intervention should change the simulated outcome: gentle={gentle:?}, strong={strong:?}"
    );
}
