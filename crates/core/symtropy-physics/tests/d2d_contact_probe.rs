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
use serde::Serialize;
use symtropy_math::{Point, Sphere};
use symtropy_physics::body::{BodyHandle, RigidBody};
use symtropy_physics::replay::{
    ApplyCommandError, ReplayTape, WorldCommand, WorldSnapshot, apply_commands,
};
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 240.0;
const STEPS: usize = 240;
const GRAVITY_Y: f64 = -9.81;
const FLOOR_RADIUS: f64 = 64.0;
const FLOOR_CENTER_Y: f64 = -64.0;
const BALL_RADIUS: f64 = 0.5;
const BALL_MASS: f64 = 1.0;

/// Candidate-visible intervention request. It deliberately contains no
/// evaluator-owned material parameters or hidden world truth.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeRequestV1 {
    tangential_impulse: f64,
}

impl ProbeRequestV1 {
    fn validate(self) -> Result<(), ProbeExecutionError> {
        if !self.tangential_impulse.is_finite() || self.tangential_impulse < 0.0 {
            return Err(ProbeExecutionError::InvalidTangentialImpulse);
        }
        Ok(())
    }

    fn fingerprint(self) -> u64 {
        let mut hash = FNV_OFFSET;
        hash_text(&mut hash, "symtropy-d2d-probe-request-v1");
        hash_u64(&mut hash, self.tangential_impulse.to_bits());
        hash
    }
}

/// Evaluator-owned scenario configuration. This type must remain outside the
/// learner-facing request/response boundary.
#[derive(Clone, Copy, Debug)]
struct EvaluatorScenarioV1 {
    surface_friction: f64,
}

impl EvaluatorScenarioV1 {
    fn validate(self) -> Result<(), ProbeExecutionError> {
        if !self.surface_friction.is_finite()
            || !(0.0..=1.0).contains(&self.surface_friction)
        {
            return Err(ProbeExecutionError::InvalidSurfaceFriction);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ProbeExecutionError {
    InvalidSurfaceFriction,
    InvalidTangentialImpulse,
    MissingFloor(BodyHandle),
    MissingBall(BodyHandle),
    Command(ApplyCommandError),
    MissingFinalBall(BodyHandle),
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

// This is the only response shape intended to cross back to the learner.
// It deliberately excludes EvaluatorScenarioV1, ProbeManifestV1,
// scenario_fingerprint, hidden friction, and evaluator-private trace digests.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeObservationV1 {
    terminal_x: f64,
    terminal_y: f64,
    terminal_vx: f64,
    terminal_vy: f64,
    collision_event_count: usize,
}

impl From<ProbeOutcome> for ProbeObservationV1 {
    fn from(outcome: ProbeOutcome) -> Self {
        Self {
            terminal_x: outcome.terminal_x,
            terminal_y: outcome.terminal_y,
            terminal_vx: outcome.terminal_vx,
            terminal_vy: outcome.terminal_vy,
            collision_event_count: outcome.collision_event_count,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbePublicFailureV1 {
    InvalidRequest,
    ExecutionFailed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeResponseV1 {
    schema_version: u32,
    branch_id: String,
    request_fingerprint: u64,
    disposition: ProbeDispositionV1,
    completed_steps: usize,
    observation: Option<ProbeObservationV1>,
    failure: Option<ProbePublicFailureV1>,
}

// Kept on the evaluator side. Do not serialize/return this container to the
// learner: its private record includes evaluator manifest and scenario identity.
struct ProbeDispatchChannelsV1 {
    learner_response: ProbeResponseV1,
    evaluator_record: ProbeResultRecordV1,
}

// Versioned, compact records for the D2D adapter fixture. The fingerprints
// below are deterministic fixture identifiers, not cryptographic commitments.
// Mycelix #5900 remains the authority for durable/content-addressed attestations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ProbeDispositionV1 {
    Completed,
    Incomplete,
    Unsupported,
    Failed,
    ResourceBound,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeManifestV1 {
    schema_version: u32,
    scenario_id: String,
    engine_package: String,
    engine_version: String,
    build_context_sha: String,
    build_ref: String,
    dimension: u32,
    gravity: [f64; 3],
    solver_iterations: usize,
    sleep_threshold: f64,
    sleep_ticks: u32,
    floor_geometry: String,
    floor_radius: f64,
    floor_center: [f64; 3],
    ball_radius: f64,
    ball_mass: f64,
    ball_friction: f64,
    floor_friction: f64,
    restitution: f64,
    linear_damping: f64,
    angular_damping: f64,
    initial_position: [f64; 3],
    initial_linear_velocity: [f64; 3],
    timestep: f64,
    step_count: usize,
    surface_friction: f64,
    tangential_impulse: f64,
    impulse_tick: usize,
}

// Evaluator-private manifest: includes hidden world parameters. It must never
// be sent to the learner; a later public receipt may expose only an authorized
// observation and opaque manifest/trace references.
impl ProbeManifestV1 {
    fn from_request_and_scenario(
        request: ProbeRequestV1,
        scenario: EvaluatorScenarioV1,
    ) -> Self {
        Self {
            schema_version: 1,
            scenario_id: "contact-probe-sphere-floor-v1".to_string(),
            engine_package: env!("CARGO_PKG_NAME").to_string(),
            engine_version: env!("CARGO_PKG_VERSION").to_string(),
            build_context_sha: option_env!("GITHUB_SHA")
                .unwrap_or("UNPINNED_LOCAL_BUILD")
                .to_string(),
            build_ref: option_env!("GITHUB_REF")
                .unwrap_or("UNPINNED_LOCAL_BUILD")
                .to_string(),
            dimension: 3,
            gravity: [0.0, GRAVITY_Y, 0.0],
            solver_iterations: 8,
            sleep_threshold: 0.0,
            sleep_ticks: u32::MAX,
            floor_geometry: "finite_sphere_approximation".to_string(),
            floor_radius: FLOOR_RADIUS,
            floor_center: [0.0, FLOOR_CENTER_Y, 0.0],
            ball_radius: BALL_RADIUS,
            ball_mass: BALL_MASS,
            ball_friction: 1.0,
            floor_friction: scenario.surface_friction * scenario.surface_friction,
            restitution: 0.0,
            linear_damping: 0.0,
            angular_damping: 0.0,
            initial_position: [0.0, BALL_RADIUS, 0.0],
            initial_linear_velocity: [0.0; 3],
            timestep: DT,
            step_count: STEPS,
            surface_friction: scenario.surface_friction,
            tangential_impulse: request.tangential_impulse,
            impulse_tick: 0,
        }
    }

    fn fingerprint(&self) -> u64 {
        let mut hash = FNV_OFFSET;
        hash_text(&mut hash, "symtropy-d2d-contact-manifest-v1");
        hash_u64(&mut hash, u64::from(self.schema_version));
        hash_text(&mut hash, &self.scenario_id);
        hash_text(&mut hash, &self.engine_package);
        hash_text(&mut hash, &self.engine_version);
        hash_text(&mut hash, &self.build_context_sha);
        hash_text(&mut hash, &self.build_ref);
        hash_u64(&mut hash, u64::from(self.dimension));
        for value in self.gravity {
            hash_u64(&mut hash, value.to_bits());
        }
        hash_u64(&mut hash, self.solver_iterations as u64);
        hash_u64(&mut hash, self.sleep_threshold.to_bits());
        hash_u64(&mut hash, u64::from(self.sleep_ticks));
        hash_text(&mut hash, &self.floor_geometry);
        hash_u64(&mut hash, self.floor_radius.to_bits());
        for value in self.floor_center {
            hash_u64(&mut hash, value.to_bits());
        }
        hash_u64(&mut hash, self.ball_radius.to_bits());
        hash_u64(&mut hash, self.ball_mass.to_bits());
        hash_u64(&mut hash, self.ball_friction.to_bits());
        hash_u64(&mut hash, self.floor_friction.to_bits());
        hash_u64(&mut hash, self.restitution.to_bits());
        hash_u64(&mut hash, self.linear_damping.to_bits());
        hash_u64(&mut hash, self.angular_damping.to_bits());
        for value in self.initial_position {
            hash_u64(&mut hash, value.to_bits());
        }
        for value in self.initial_linear_velocity {
            hash_u64(&mut hash, value.to_bits());
        }
        hash_u64(&mut hash, self.timestep.to_bits());
        hash_u64(&mut hash, self.step_count as u64);
        hash_u64(&mut hash, self.surface_friction.to_bits());
        hash_u64(&mut hash, self.tangential_impulse.to_bits());
        hash_u64(&mut hash, self.impulse_tick as u64);
        hash
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeResultRecordV1 {
    schema_version: u32,
    branch_id: String,
    manifest: ProbeManifestV1,
    disposition: ProbeDispositionV1,
    completed_steps: usize,
    collision_event_count: usize,
    scenario_fingerprint: u64,
    replay_tape_fingerprint: u64,
    trajectory_fingerprint: u64,
    outcome_fingerprint: u64,
    record_fingerprint: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProbeBranchAuditV1 {
    expected_count: usize,
    reported_count: usize,
    completed_count: usize,
    expected_branches: Vec<String>,
    reported_branches: Vec<String>,
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
            for value in &body.translation {
                hash_u64(&mut hash, *value);
            }
            for row in &body.rotation {
                for value in row { hash_u64(&mut hash, *value); }
            }
            for value in &body.linear_velocity {
                hash_u64(&mut hash, *value);
            }
            for row in &body.angular_velocity {
                for value in row { hash_u64(&mut hash, *value); }
            }
            hash_u64(&mut hash, body.sleeping as u64);
            hash_u64(&mut hash, u64::from(body.sleep_counter));
        }
        hash_u64(&mut hash, snapshot.collision_events.len() as u64);
        for event in &snapshot.collision_events {
            hash_u64(&mut hash, event.body_a.0 as u64);
            hash_u64(&mut hash, event.body_b.0 as u64);
            hash_u64(&mut hash, event.impulse);
            for value in &event.normal {
                hash_u64(&mut hash, *value);
            }
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
    hash_u64(&mut hash, record.manifest.fingerprint());
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

    let expected_branches: Vec<String> = expected
        .iter()
        .map(|branch| (*branch).to_string())
        .collect();
    let expected_set: BTreeSet<String> = expected_branches.iter().cloned().collect();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut duplicate = BTreeSet::new();
    let mut unexpected = BTreeSet::new();
    let mut non_completed = BTreeSet::new();
    let mut completed_count = 0;

    for record in observed {
        let count = counts.entry(record.branch_id.clone()).or_default();
        *count += 1;
        if *count > 1 {
            duplicate.insert(record.branch_id.clone());
        }
        let is_expected = expected_set.contains(&record.branch_id);
        if !is_expected {
            unexpected.insert(record.branch_id.clone());
        }
        if record.disposition == ProbeDispositionV1::Completed {
            // Count only the first completion for an expected branch. Duplicate
            // and unexpected reports remain visible but cannot inflate coverage.
            if is_expected && *count == 1 {
                completed_count += 1;
            }
        } else {
            non_completed.insert(record.branch_id.clone());
        }
    }

    let missing: Vec<String> = expected_set
        .iter()
        .filter(|branch_id| !counts.contains_key(*branch_id))
        .cloned()
        .collect();
    let reported_branches = observed
        .iter()
        .map(|record| record.branch_id.clone())
        .collect();

    ProbeBranchAuditV1 {
        expected_count: expected.len(),
        reported_count: observed.len(),
        completed_count,
        expected_branches,
        reported_branches,
        missing,
        duplicate: duplicate.into_iter().collect(),
        unexpected: unexpected.into_iter().collect(),
        non_completed: non_completed.into_iter().collect(),
    }
}

fn build_world(
    surface_friction: f64,
) -> Result<(PhysicsWorld<3>, BodyHandle), ProbeExecutionError> {
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
    let Some(floor_body) = world.body_mut(floor_handle) else {
        return Err(ProbeExecutionError::MissingFloor(floor_handle));
    };
    floor_body.friction = surface_friction * surface_friction;

    let ball = world.add_sphere(
        Point::new([0.0, BALL_RADIUS, 0.0]),
        BALL_RADIUS,
        BALL_MASS,
    );
    let Some(body) = world.body_mut(ball) else {
        return Err(ProbeExecutionError::MissingBall(ball));
    };
    body.friction = 1.0;
    body.restitution = 0.0;
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;

    Ok((world, ball))
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
    request: ProbeRequestV1,
    scenario: EvaluatorScenarioV1,
) -> Result<(Vec<WorldSnapshot<3>>, ProbeOutcome, ProbeResultRecordV1), ProbeExecutionError> {
    request.validate()?;
    scenario.validate()?;
    let (mut world, ball) = build_world(scenario.surface_friction)?;
    let tape = build_tape(ball, request.tangential_impulse);
    let manifest = ProbeManifestV1::from_request_and_scenario(request, scenario);
    let scenario_fingerprint = manifest.fingerprint();
    let replay_tape_fingerprint = replay_tape_fingerprint(&tape);

    let mut snapshots = Vec::with_capacity(tape.frames.len());
    let mut collision_event_count = 0usize;
    for frame in &tape.frames {
        apply_commands(&mut world, &frame.commands)
            .map_err(ProbeExecutionError::Command)?;
        world.step(frame.dt);
        collision_event_count += world.collision_events.len();
        snapshots.push(WorldSnapshot::capture(&world));
    }

    let body = world
        .body(ball)
        .ok_or(ProbeExecutionError::MissingFinalBall(ball))?;
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
        manifest,
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
    Ok((snapshots, outcome, record))
}

/// Evaluator dispatcher. It consumes hidden scenario truth internally and returns
/// only the learner-facing response DTO. This helper still runs in-process;
/// actual evaluator isolation requires a process/permission boundary.
fn failed_evaluator_record(
    branch_id: &str,
    request: ProbeRequestV1,
    scenario: EvaluatorScenarioV1,
) -> ProbeResultRecordV1 {
    let manifest = ProbeManifestV1::from_request_and_scenario(request, scenario);
    let scenario_fingerprint = manifest.fingerprint();
    let mut record = ProbeResultRecordV1 {
        schema_version: 1,
        branch_id: branch_id.to_string(),
        manifest,
        disposition: ProbeDispositionV1::Failed,
        completed_steps: 0,
        collision_event_count: 0,
        scenario_fingerprint,
        // Zero is reserved here for a tape/trace that was not produced.
        replay_tape_fingerprint: 0,
        trajectory_fingerprint: 0,
        outcome_fingerprint: 0,
        record_fingerprint: 0,
    };
    record.record_fingerprint = record_fingerprint(&record);
    record
}

/// Evaluator dispatcher. The learner response and evaluator record are
/// returned in separate, typed channels. This helper still runs in-process;
/// actual evaluator isolation requires a process/permission boundary.
fn dispatch_probe_request(
    branch_id: &str,
    request: ProbeRequestV1,
    scenario: EvaluatorScenarioV1,
) -> ProbeDispatchChannelsV1 {
    let request_fingerprint = request.fingerprint();
    if request.validate().is_err() {
        return ProbeDispatchChannelsV1 {
            learner_response: ProbeResponseV1 {
                schema_version: 1,
                branch_id: branch_id.to_string(),
                request_fingerprint,
                disposition: ProbeDispositionV1::Failed,
                completed_steps: 0,
                observation: None,
                failure: Some(ProbePublicFailureV1::InvalidRequest),
            },
            evaluator_record: failed_evaluator_record(branch_id, request, scenario),
        };
    }

    match execute_probe_with_record(branch_id, request, scenario) {
        Ok((snapshots, outcome, evaluator_record)) => ProbeDispatchChannelsV1 {
            learner_response: ProbeResponseV1 {
                schema_version: 1,
                branch_id: branch_id.to_string(),
                request_fingerprint,
                disposition: ProbeDispositionV1::Completed,
                completed_steps: snapshots.len(),
                observation: Some(outcome.into()),
                failure: None,
            },
            evaluator_record,
        },
        Err(_private_error) => ProbeDispatchChannelsV1 {
            learner_response: ProbeResponseV1 {
                schema_version: 1,
                branch_id: branch_id.to_string(),
                request_fingerprint,
                disposition: ProbeDispositionV1::Failed,
                completed_steps: 0,
                observation: None,
                failure: Some(ProbePublicFailureV1::ExecutionFailed),
            },
            evaluator_record: failed_evaluator_record(branch_id, request, scenario),
        },
    }
}

fn execute_probe(
    request: ProbeRequestV1,
    scenario: EvaluatorScenarioV1,
) -> (Vec<WorldSnapshot<3>>, ProbeOutcome) {
    let (snapshots, outcome, _) =
        execute_probe_with_record("unlabelled-fixture-run", request, scenario)
            .expect("test fixture probe should execute");
    (snapshots, outcome)
}

#[test]
fn learner_response_contains_only_authorized_observation_and_request_identity() {
    let request = ProbeRequestV1 {
        tangential_impulse: 3.0,
    };
    let channels = dispatch_probe_request(
        "learner-visible-branch-1",
        request,
        EvaluatorScenarioV1 {
            surface_friction: 0.05,
        },
    );
    let response = channels.learner_response;

    assert_eq!(channels.evaluator_record.disposition, ProbeDispositionV1::Completed);
    assert_ne!(channels.evaluator_record.scenario_fingerprint, 0);
    assert_eq!(response.schema_version, 1);
    assert_eq!(response.branch_id, "learner-visible-branch-1");
    assert_eq!(response.request_fingerprint, request.fingerprint());
    assert_eq!(response.disposition, ProbeDispositionV1::Completed);
    assert_eq!(response.completed_steps, STEPS);
    assert!(response.failure.is_none());
    let observation = response.observation.expect("completed response has an observation");
    assert!(observation.collision_event_count > 0);

    // Compile-time response-shape boundary: ProbeResponseV1 contains no
    // scenario friction, evaluator manifest, scenario fingerprint or trace digest.
}

#[test]
fn failed_evaluator_attempt_returns_visible_failure_disposition_not_dropped_error() {
    let request = ProbeRequestV1 {
        tangential_impulse: 1.0,
    };
    let channels = dispatch_probe_request(
        "expected-branch-invalid-world",
        request,
        EvaluatorScenarioV1 {
            surface_friction: f64::NAN,
        },
    );
    let response = channels.learner_response;

    assert_eq!(channels.evaluator_record.branch_id, "expected-branch-invalid-world");
    assert_eq!(channels.evaluator_record.disposition, ProbeDispositionV1::Failed);
    assert_eq!(channels.evaluator_record.completed_steps, 0);
    assert_eq!(response.branch_id, "expected-branch-invalid-world");
    assert_eq!(response.request_fingerprint, request.fingerprint());
    assert_eq!(response.disposition, ProbeDispositionV1::Failed);
    assert_eq!(response.completed_steps, 0);
    assert!(response.observation.is_none());
    assert_eq!(
        response.failure,
        Some(ProbePublicFailureV1::ExecutionFailed)
    );
}

#[test]
fn invalid_learner_request_is_rejected_before_evaluator_dispatch() {
    let invalid_request = ProbeRequestV1 {
        tangential_impulse: f64::INFINITY,
    };
    let channels = dispatch_probe_request(
        "invalid-learner-request",
        invalid_request,
        EvaluatorScenarioV1 {
            surface_friction: 0.5,
        },
    );
    let response = channels.learner_response;

    assert_eq!(channels.evaluator_record.disposition, ProbeDispositionV1::Failed);
    assert_eq!(response.disposition, ProbeDispositionV1::Failed);
    assert_eq!(
        response.failure,
        Some(ProbePublicFailureV1::InvalidRequest)
    );
    assert!(response.observation.is_none());
    assert_eq!(response.completed_steps, 0);
}

#[test]
fn same_frozen_contact_probe_replays_bitwise_at_each_tick() {
    let request = ProbeRequestV1 {
        tangential_impulse: 3.0,
    };
    let scenario = EvaluatorScenarioV1 {
        surface_friction: 0.45,
    };

    let (recorded, recorded_outcome) = execute_probe(request, scenario);
    let (replayed, replayed_outcome) = execute_probe(request, scenario);

    assert_eq!(recorded.len(), STEPS);
    assert_eq!(
        recorded, replayed,
        "body/event snapshot diverged during replay"
    );
    assert_eq!(recorded_outcome, replayed_outcome);
    assert!(
        recorded_outcome.collision_event_count > 0,
        "the probe must exercise contact, not free flight only"
    );
}

#[test]
fn surface_friction_change_changes_the_executed_probe_observation() {
    let request = ProbeRequestV1 {
        tangential_impulse: 3.0,
    };
    let (low_replay, low) = execute_probe(
        request,
        EvaluatorScenarioV1 {
            surface_friction: 0.05,
        },
    );
    let (high_replay, high) = execute_probe(
        request,
        EvaluatorScenarioV1 {
            surface_friction: 0.90,
        },
    );

    assert_eq!(low_replay.len(), STEPS);
    assert_eq!(high_replay.len(), STEPS);
    assert!(
        (low.terminal_vx - high.terminal_vx).abs() > 1e-5
            || (low.terminal_x - high.terminal_x).abs() > 1e-5,
        "declared friction intervention should change simulated outcome: low={low:?}, high={high:?}"
    );
}

#[test]
fn tangential_probe_magnitude_changes_the_executed_observation() {
    let scenario = EvaluatorScenarioV1 {
        surface_friction: 0.45,
    };
    let (_, gentle) = execute_probe(
        ProbeRequestV1 {
            tangential_impulse: 1.0,
        },
        scenario,
    );
    let (_, strong) = execute_probe(
        ProbeRequestV1 {
            tangential_impulse: 3.0,
        },
        scenario,
    );

    assert!(
        (gentle.terminal_vx - strong.terminal_vx).abs() > 1e-5
            || (gentle.terminal_x - strong.terminal_x).abs() > 1e-5,
        "declared probe intervention should change simulated outcome: \
         gentle={gentle:?}, strong={strong:?}"
    );
}

#[test]
fn invalid_probe_spec_returns_typed_error_without_panicking() {
    let valid_request = ProbeRequestV1 {
        tangential_impulse: 1.0,
    };
    let invalid_scenario = EvaluatorScenarioV1 {
        surface_friction: f64::NAN,
    };
    assert!(matches!(
        execute_probe_with_record("invalid-scenario", valid_request, invalid_scenario),
        Err(ProbeExecutionError::InvalidSurfaceFriction)
    ));

    let invalid_request = ProbeRequestV1 {
        tangential_impulse: f64::INFINITY,
    };
    let valid_scenario = EvaluatorScenarioV1 {
        surface_friction: 0.5,
    };
    assert!(matches!(
        execute_probe_with_record("invalid-request", invalid_request, valid_scenario),
        Err(ProbeExecutionError::InvalidTangentialImpulse)
    ));
}

#[test]
fn contact_probe_emits_versioned_identity_bound_result_records() {
    let request = ProbeRequestV1 {
        tangential_impulse: 3.0,
    };
    let scenario = EvaluatorScenarioV1 {
        surface_friction: 0.45,
    };
    let (trace_a, outcome_a, receipt_a) =
        execute_probe_with_record("replay-a", request, scenario).unwrap();
    let (trace_b, outcome_b, receipt_b) =
        execute_probe_with_record("replay-a", request, scenario).unwrap();

    assert_eq!(receipt_a.schema_version, 1);
    assert_eq!(receipt_a.completed_steps, STEPS);
    assert_eq!(
        receipt_a.scenario_fingerprint,
        receipt_a.manifest.fingerprint()
    );
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

    let mut changed_manifest = receipt_a.clone();
    changed_manifest.manifest.solver_iterations += 1;
    assert_ne!(
        receipt_a.record_fingerprint,
        record_fingerprint(&changed_manifest),
        "a changed solver profile must invalidate the result fingerprint"
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
        (
            "low-gentle",
            ProbeRequestV1 {
                tangential_impulse: 1.0,
            },
            EvaluatorScenarioV1 {
                surface_friction: 0.05,
            },
        ),
        (
            "low-strong",
            ProbeRequestV1 {
                tangential_impulse: 3.0,
            },
            EvaluatorScenarioV1 {
                surface_friction: 0.05,
            },
        ),
        (
            "high-gentle",
            ProbeRequestV1 {
                tangential_impulse: 1.0,
            },
            EvaluatorScenarioV1 {
                surface_friction: 0.90,
            },
        ),
        (
            "high-strong",
            ProbeRequestV1 {
                tangential_impulse: 3.0,
            },
            EvaluatorScenarioV1 {
                surface_friction: 0.90,
            },
        ),
    ];

    let mut outcomes = Vec::new();
    let mut records = Vec::new();
    for (branch_id, request, scenario) in conditions {
        let (_, outcome, record) =
            execute_probe_with_record(branch_id, request, scenario).unwrap();
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
            manifest: ProbeManifestV1::from_request_and_scenario(
                ProbeRequestV1 {
                    tangential_impulse: 2.0,
                },
                EvaluatorScenarioV1 {
                    surface_friction: 0.3,
                },
            ),
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
            branch_id: "branch-a".to_string(),
            manifest: ProbeManifestV1::from_request_and_scenario(
                ProbeRequestV1 {
                    tangential_impulse: 2.0,
                },
                EvaluatorScenarioV1 {
                    surface_friction: 0.3,
                },
            ),
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
            manifest: ProbeManifestV1::from_request_and_scenario(
                ProbeRequestV1 {
                    tangential_impulse: 2.0,
                },
                EvaluatorScenarioV1 {
                    surface_friction: 0.4,
                },
            ),
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
            manifest: ProbeManifestV1::from_request_and_scenario(
                ProbeRequestV1 {
                    tangential_impulse: 2.0,
                },
                EvaluatorScenarioV1 {
                    surface_friction: 0.5,
                },
            ),
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
    assert_eq!(audit.reported_count, 4);
    assert_eq!(
        audit.completed_count, 1,
        "duplicate completion must not inflate coverage"
    );
    assert_eq!(
        audit.expected_branches,
        vec![
            "branch-a".to_string(),
            "branch-b".to_string(),
            "branch-c".to_string(),
        ]
    );
    assert_eq!(
        audit.reported_branches,
        vec![
            "branch-a".to_string(),
            "branch-a".to_string(),
            "branch-b".to_string(),
            "unexpected".to_string()
        ]
    );
    assert_eq!(audit.missing, vec!["branch-c".to_string()]);
    assert_eq!(audit.duplicate, vec!["branch-a".to_string()]);
    assert_eq!(audit.unexpected, vec!["unexpected".to_string()]);
    assert_eq!(
        audit.non_completed,
        vec!["branch-b".to_string(), "unexpected".to_string()]
    );
}

