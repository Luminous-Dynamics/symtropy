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

// Versioned, compact records for the D2D adapter fixture. The fingerprints
// below are deterministic fixture identifiers, not cryptographic commitments.
// Mycelix #5900 remains the authority for durable/content-addressed attestations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeDispositionV1 {
    Completed,
    Incomplete,
    Unsupported,
    Failed,
    ResourceBound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProbeResultRecordV1 {
    schema_version: u32,
    branch_id: String,
    disposition: ProbeDispositionV1,
    completed_steps: usize,
    collision_event_count: usize,
    scenario_fingerprint: u64,
    replay_tape_fingerprint: u64,
    trajectory_fingerprint: u64,
    outcome_fingerprint: u64,
    record_fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProbeBranchAuditV1 {
    expected_count: usize,
    reported_count: usize,
    completed_count: usize,
    missing: Vec<String>,
    duplicate: Vec<String>,
    unexpected: Vec<String>,
    non_completed: Vec<String>,
}

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

fn hash_u64(hash: &mut u64, value: u64) {
    hash_bytes(hash, &value.to_le_bytes());
}

fn hash_text(hash: &mut u64, value: &str) {
    hash_u64(hash, value.len() as u64);
    hash_bytes(hash, value.as_bytes());
}

/// Hash every scenario field that changes the declared execution conditions.
fn scenario_fingerprint(spec: ProbeSpec) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_text(&mut hash, "symtropy-d2d-contact-probe-manifest-v1");
    hash_text(&mut hash, env!("CARGO_PKG_NAME"));
    hash_text(&mut hash, env!("CARGO_PKG_VERSION"));
    hash_u64(&mut hash, 3);
    hash_u64(&mut hash, GRAVITY_Y.to_bits());
    hash_u64(&mut hash, 8); // solver_iterations
    hash_u64(&mut hash, 0.0f64.to_bits()); // sleep_threshold
    hash_u64(&mut hash, u64::from(u32::MAX)); // sleep_ticks
    hash_text(&mut hash, "finite_sphere_floor_approximation");
    hash_u64(&mut hash, FLOOR_RADIUS.to_bits());
    hash_u64(&mut hash, FLOOR_CENTER_Y.to_bits());
    hash_u64(&mut hash, BALL_RADIUS.to_bits());
    hash_u64(&mut hash, BALL_MASS.to_bits());
    hash_u64(&mut hash, 1.0f64.to_bits()); // dynamic ball friction
    hash_u64(&mut hash, (spec.surface_friction * spec.surface_friction).to_bits());
    hash_u64(&mut hash, 0.0f64.to_bits()); // restitution
    hash_u64(&mut hash, 0.0f64.to_bits()); // linear damping
    hash_u64(&mut hash, 0.0f64.to_bits()); // angular damping
    hash_u64(&mut hash, DT.to_bits());
    hash_u64(&mut hash, STEPS as u64);
    hash_u64(&mut hash, 0.0f64.to_bits()); // initial x
    hash_u64(&mut hash, BALL_RADIUS.to_bits()); // initial y
    hash_u64(&mut hash, 0.0f64.to_bits()); // initial z
    hash_u64(&mut hash, 0.0f64.to_bits()); // initial velocity x
    hash_u64(&mut hash, 0.0f64.to_bits()); // initial velocity y
    hash_u64(&mut hash, 0.0f64.to_bits()); // initial velocity z
    hash_u64(&mut hash, spec.surface_friction.to_bits());
    hash_u64(&mut hash, spec.tangential_impulse.to_bits());
    hash
}

fn replay_tape_fingerprint(tape: &ReplayTape<3>) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_text(&mut hash, "symtropy-replay-tape-v1");
    hash_u64(&mut hash, tape.frames.len() as u64);
    for frame in &tape.frames {
        hash_u64(&mut hash, frame.dt.to_bits());
        hash_u64(&mut hash, frame.commands.len() as u64);
        for command in &frame.commands {
            match command {
                WorldCommand::ApplyImpulse { body, impulse } => {
                    hash_u64(&mut hash, 1); // stable command discriminant
                    hash_u64(&mut hash, body.0 as u64);
                    for value in impulse.iter() {
                        hash_u64(&mut hash, value.to_bits());
                    }
                }
                WorldCommand::ApplyForce { body, force } => {
                    hash_u64(&mut hash, 2);
                    hash_u64(&mut hash, body.0 as u64);
                    for value in force.iter() {
                        hash_u64(&mut hash, value.to_bits());
                    }
                }
                WorldCommand::SetLinearVelocity { body, velocity } => {
                    hash_u64(&mut hash, 3);
                    hash_u64(&mut hash, body.0 as u64);
                    for value in velocity.iter() {
                        hash_u64(&mut hash, value.to_bits());
                    }
                }
                WorldCommand::SetAngularVelocity { body, velocity } => {
                    hash_u64(&mut hash, 4);
                    hash_u64(&mut hash, body.0 as u64);
                    let matrix = velocity.to_matrix();
                    for row in 0..3 {
                        for column in 0..3 {
                            hash_u64(&mut hash, matrix[(row, column)].to_bits());
                        }
                    }
                }
                WorldCommand::Wake { body } => {
                    hash_u64(&mut hash, 5);
                    hash_u64(&mut hash, body.0 as u64);
                }
            }
        }
    }
    hash
}

/// Fingerprint only the fields in the public WorldSnapshot contract:
/// bodies and last-step collision events, not hidden solver/contact-cache state.
fn trajectory_fingerprint(snapshots: &[WorldSnapshot<3>]) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_text(&mut hash, "symtropy-world-snapshot-trace-v1");
    hash_u64(&mut hash, snapshots.len() as u64);
    for snapshot in snapshots {
        hash_u64(&mut hash, snapshot.bodies.len() as u64);
        for body in &snapshot.bodies {
            hash_u64(&mut hash, body.handle.0 as u64);
            let body_type = match body.body_type {
                symtropy_physics::body::BodyType::Static => 1,
                symtropy_physics::body::BodyType::Kinematic => 2,
                symtropy_physics::body::BodyType::Dynamic => 3,
            };
            hash_u64(&mut hash, body_type);
            for value in &body.translation { hash_u64(&mut hash, *value); }
            for row in &body.rotation {
                for value in row { hash_u64(&mut hash, *value); }
            }
            for value in &body.linear_velocity { hash_u64(&mut hash, *value); }
            for row in &body.angular_velocity {
                for value in row { hash_u64(&mut hash, *value); }
            }
            hash_u64(&mut hash, u64::from(body.sleeping));
            hash_u64(&mut hash, u64::from(body.sleep_counter));
        }
        hash_u64(&mut hash, snapshot.collision_events.len() as u64);
        for event in &snapshot.collision_events {
            hash_u64(&mut hash, event.body_a.0 as u64);
            hash_u64(&mut hash, event.body_b.0 as u64);
            hash_u64(&mut hash, event.impulse);
            for value in &event.normal { hash_u64(&mut hash, *value); }
            hash_u64(&mut hash, event.depth);
        }
    }
    hash
}

fn outcome_fingerprint(outcome: ProbeOutcome) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_text(&mut hash, "symtropy-d2d-probe-outcome-v1");
    hash_u64(&mut hash, outcome.terminal_x.to_bits());
    hash_u64(&mut hash, outcome.terminal_y.to_bits());
    hash_u64(&mut hash, outcome.terminal_vx.to_bits());
    hash_u64(&mut hash, outcome.terminal_vy.to_bits());
    hash_u64(&mut hash, outcome.collision_event_count as u64);
    hash
}

fn record_fingerprint(record: &ProbeResultRecordV1) -> u64 {
    let mut hash = FNV_OFFSET;
    hash_text(&mut hash, "symtropy-d2d-probe-result-v1");
    hash_u64(&mut hash, u64::from(record.schema_version));
    hash_text(&mut hash, &record.branch_id);
    let disposition = match record.disposition {
        ProbeDispositionV1::Completed => 1,
        ProbeDispositionV1::Incomplete => 2,
        ProbeDispositionV1::Unsupported => 3,
        ProbeDispositionV1::Failed => 4,
        ProbeDispositionV1::ResourceBound => 5,
    };
    hash_u64(&mut hash, disposition);
    hash_u64(&mut hash, record.completed_steps as u64);
    hash_u64(&mut hash, record.collision_event_count as u64);
    hash_u64(&mut hash, record.scenario_fingerprint);
    hash_u64(&mut hash, record.replay_tape_fingerprint);
    hash_u64(&mut hash, record.trajectory_fingerprint);
    hash_u64(&mut hash, record.outcome_fingerprint);
    hash
}

fn audit_expected_branches(
    expected: &[&str],
    observed: &[ProbeResultRecordV1],
) -> ProbeBranchAuditV1 {
    use std::collections::{BTreeSet, HashMap};

    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let mut duplicate = BTreeSet::new();
    let mut unexpected = BTreeSet::new();
    let mut non_completed = BTreeSet::new();
    let mut completed_count = 0;

    for record in observed {
        let count = counts.entry(record.branch_id.as_str()).or_default();
        *count += 1;
        if *count > 1 {
            duplicate.insert(record.branch_id.clone());
        }
        if !expected_set.contains(record.branch_id.as_str()) {
            unexpected.insert(record.branch_id.clone());
        }
        if record.disposition == ProbeDispositionV1::Completed {
            completed_count += 1;
        } else {
            non_completed.insert(record.branch_id.clone());
        }
    }

    let missing = expected_set
        .iter()
        .filter(|branch_id| !counts.contains_key(**branch_id))
        .map(|branch_id| (*branch_id).to_string())
        .collect();

    ProbeBranchAuditV1 {
        expected_count: expected.len(),
        reported_count: observed.len(),
        completed_count,
        missing,
        duplicate: duplicate.into_iter().collect(),
        unexpected: unexpected.into_iter().collect(),
        non_completed: non_completed.into_iter().collect(),
    }
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

fn execute_probe_with_record(
    branch_id: &str,
    spec: ProbeSpec,
) -> (Vec<WorldSnapshot<3>>, ProbeOutcome, ProbeResultRecordV1) {
    spec.validate();
    let (mut world, ball) = build_world(spec.surface_friction);
    let tape = build_tape(ball, spec.tangential_impulse);
    let scenario_fingerprint = scenario_fingerprint(spec);
    let replay_tape_fingerprint = replay_tape_fingerprint(&tape);

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
    let mut record = ProbeResultRecordV1 {
        schema_version: 1,
        branch_id: branch_id.to_string(),
        disposition: ProbeDispositionV1::Completed,
        completed_steps: snapshots.len(),
        collision_event_count,
        scenario_fingerprint,
        replay_tape_fingerprint,
        trajectory_fingerprint: trajectory_fingerprint(&snapshots),
        outcome_fingerprint: outcome_fingerprint(outcome),
        record_fingerprint: 0,
    };
    record.record_fingerprint = record_fingerprint(&record);
    (snapshots, outcome, record)
}

fn execute_probe(spec: ProbeSpec) -> (Vec<WorldSnapshot<3>>, ProbeOutcome) {
    let (snapshots, outcome, _) = execute_probe_with_record("unlabelled-fixture-run", spec);
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


#[test]
fn contact_probe_emits_versioned_identity_bound_result_records() {
    let spec = ProbeSpec {
        surface_friction: 0.45,
        tangential_impulse: 3.0,
    };
    let (trace_a, outcome_a, receipt_a) = execute_probe_with_record("replay-a", spec);
    let (trace_b, outcome_b, receipt_b) = execute_probe_with_record("replay-a", spec);

    assert_eq!(receipt_a.schema_version, 1);
    assert_eq!(receipt_a.completed_steps, STEPS);
    assert_eq!(trace_a, trace_b);
    assert_eq!(outcome_a, outcome_b);
    assert_eq!(receipt_a, receipt_b);
    assert_eq!(
        receipt_a.record_fingerprint,
        record_fingerprint(&ProbeResultRecordV1 {
            record_fingerprint: 0,
            ..receipt_a.clone()
        })
    );
}

#[test]
fn factorial_contact_probe_covers_both_materials_and_both_interventions() {
    let expected = [
        "low-gentle",
        "low-strong",
        "high-gentle",
        "high-strong",
    ];
    let conditions = [
        ("low-gentle", ProbeSpec { surface_friction: 0.05, tangential_impulse: 1.0 }),
        ("low-strong", ProbeSpec { surface_friction: 0.05, tangential_impulse: 3.0 }),
        ("high-gentle", ProbeSpec { surface_friction: 0.90, tangential_impulse: 1.0 }),
        ("high-strong", ProbeSpec { surface_friction: 0.90, tangential_impulse: 3.0 }),
    ];

    let mut outcomes = Vec::new();
    let mut records = Vec::new();
    for (branch_id, spec) in conditions {
        let (_, outcome, record) = execute_probe_with_record(branch_id, spec);
        assert_eq!(record.disposition, ProbeDispositionV1::Completed);
        outcomes.push(outcome);
        records.push(record);
    }

    let audit = audit_expected_branches(&expected, &records);
    assert_eq!(audit.expected_count, 4);
    assert_eq!(audit.reported_count, 4);
    assert_eq!(audit.completed_count, 4);
    assert!(audit.missing.is_empty());
    assert!(audit.duplicate.is_empty());
    assert!(audit.unexpected.is_empty());
    assert!(audit.non_completed.is_empty());

    // Within each material condition, the impulse intervention must affect the
    // measured response; at each impulse level, material friction must affect it.
    for (gentle, strong) in [(outcomes[0], outcomes[1]), (outcomes[2], outcomes[3])] {
        assert!(
            (gentle.terminal_x - strong.terminal_x).abs() > 1e-5
                || (gentle.terminal_vx - strong.terminal_vx).abs() > 1e-5
        );
    }
    for (low, high) in [(outcomes[0], outcomes[2]), (outcomes[1], outcomes[3])] {
        assert!(
            (low.terminal_x - high.terminal_x).abs() > 1e-5
                || (low.terminal_vx - high.terminal_vx).abs() > 1e-5
        );
    }

    // The scenario identity includes friction; the replay tape identity does not.
    assert_ne!(records[0].scenario_fingerprint, records[2].scenario_fingerprint);
    assert_eq!(records[0].replay_tape_fingerprint, records[2].replay_tape_fingerprint);
    assert_ne!(records[0].replay_tape_fingerprint, records[1].replay_tape_fingerprint);
}

#[test]
fn branch_denominator_audit_preserves_missing_unsupported_and_unexpected_cases() {
    let expected = ["branch-a", "branch-b", "branch-c"];
    let observed = [
        ProbeResultRecordV1 {
            schema_version: 1,
            branch_id: "branch-a".to_string(),
            disposition: ProbeDispositionV1::Completed,
            completed_steps: STEPS,
            collision_event_count: 1,
            scenario_fingerprint: 11,
            replay_tape_fingerprint: 12,
            trajectory_fingerprint: 13,
            outcome_fingerprint: 14,
            record_fingerprint: 15,
        },
        ProbeResultRecordV1 {
            schema_version: 1,
            branch_id: "branch-b".to_string(),
            disposition: ProbeDispositionV1::Unsupported,
            completed_steps: 0,
            collision_event_count: 0,
            scenario_fingerprint: 21,
            replay_tape_fingerprint: 22,
            trajectory_fingerprint: 0,
            outcome_fingerprint: 0,
            record_fingerprint: 0,
        },
        ProbeResultRecordV1 {
            schema_version: 1,
            branch_id: "unexpected".to_string(),
            disposition: ProbeDispositionV1::Failed,
            completed_steps: 0,
            collision_event_count: 0,
            scenario_fingerprint: 31,
            replay_tape_fingerprint: 32,
            trajectory_fingerprint: 0,
            outcome_fingerprint: 0,
            record_fingerprint: 0,
        },
    ];

    let audit = audit_expected_branches(&expected, &observed);
    assert_eq!(audit.expected_count, 3);
    assert_eq!(audit.reported_count, 3);
    assert_eq!(audit.completed_count, 1);
    assert_eq!(audit.missing, vec!["branch-c".to_string()]);
    assert!(audit.duplicate.is_empty());
    assert_eq!(audit.unexpected, vec!["unexpected".to_string()]);
    assert_eq!(
        audit.non_completed,
        vec!["branch-b".to_string(), "unexpected".to_string()]
    );
}

