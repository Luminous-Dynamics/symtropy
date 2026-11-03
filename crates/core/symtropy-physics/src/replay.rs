// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Determinism helpers: record/replay command streams and bitwise snapshots.
//!
//! This module is intentionally minimal: it provides
//! - a small `WorldCommand` vocabulary for driving a `PhysicsWorld`
//! - a `ReplayTape` of per-tick frames (dt + commands)
//! - `WorldSnapshot`/`BodySnapshot` that capture simulation state as raw `f64` bits
//!
//! The goal is to make it easy to build a replay harness that asserts
//! **bitwise-equal state per tick** across record/replay passes.

use blake3::Hasher;
use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::Bivector;

use crate::body::{BodyHandle, BodyType, RigidBody};
use crate::integrator;
use crate::world::PhysicsWorld;

/// Commands that mutate a physics world at a tick boundary.
#[derive(Clone, Debug)]
pub enum WorldCommand<const D: usize> {
    ApplyForce {
        body: BodyHandle,
        force: Box<SVector<f64, D>>,
    },
    ApplyImpulse {
        body: BodyHandle,
        impulse: Box<SVector<f64, D>>,
    },
    SetLinearVelocity {
        body: BodyHandle,
        velocity: Box<SVector<f64, D>>,
    },
    SetAngularVelocity {
        body: BodyHandle,
        velocity: Box<Bivector<D>>,
    },
    Wake {
        body: BodyHandle,
    },
}

/// A single replay frame: `dt` + ordered list of commands to apply before stepping.
#[derive(Clone, Debug)]
pub struct ReplayFrame<const D: usize> {
    pub dt: f64,
    pub commands: Vec<WorldCommand<D>>,
}

/// A full replay tape: ordered frames.
#[derive(Clone, Debug, Default)]
pub struct ReplayTape<const D: usize> {
    pub frames: Vec<ReplayFrame<D>>,
}

impl<const D: usize> ReplayTape<D> {
    pub fn push_frame(&mut self, dt: f64, commands: Vec<WorldCommand<D>>) {
        self.frames.push(ReplayFrame { dt, commands });
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ApplyCommandError {
    MissingBody(BodyHandle),
}

/// Apply a list of commands to a world.
pub fn apply_commands<const D: usize>(
    world: &mut PhysicsWorld<D>,
    commands: &[WorldCommand<D>],
) -> Result<(), ApplyCommandError> {
    for cmd in commands {
        match cmd {
            WorldCommand::ApplyForce { body, force } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.apply_force(**force);
            }
            WorldCommand::ApplyImpulse { body, impulse } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                integrator::apply_impulse(b, &**impulse);
            }
            WorldCommand::SetLinearVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.linear_velocity = **velocity;
            }
            WorldCommand::SetAngularVelocity { body, velocity } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.angular_velocity = **velocity;
            }
            WorldCommand::Wake { body } => {
                let Some(b) = world.body_mut(*body) else {
                    return Err(ApplyCommandError::MissingBody(*body));
                };
                b.wake();
            }
        }
    }
    Ok(())
}

/// Bitwise snapshot of a rigid body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BodySnapshot<const D: usize> {
    pub handle: BodyHandle,
    pub body_type: BodyType,
    pub translation: [u64; D],
    pub rotation: [[u64; D]; D],
    pub linear_velocity: [u64; D],
    pub angular_velocity: [[u64; D]; D],
    pub sleeping: bool,
    pub sleep_counter: u32,
}

impl<const D: usize> BodySnapshot<D> {
    pub fn from_body(body: &RigidBody<D>) -> Self {
        let translation = std::array::from_fn(|i| body.transform.translation.0[i].to_bits());

        let rot = body.transform.rotation.to_matrix();
        let rotation = std::array::from_fn(|r| std::array::from_fn(|c| rot[(r, c)].to_bits()));

        let linear_velocity = std::array::from_fn(|i| body.linear_velocity[i].to_bits());

        let ang = body.angular_velocity.to_matrix();
        let angular_velocity =
            std::array::from_fn(|r| std::array::from_fn(|c| ang[(r, c)].to_bits()));

        Self {
            handle: body.handle,
            body_type: body.body_type,
            translation,
            rotation,
            linear_velocity,
            angular_velocity,
            sleeping: body.sleeping,
            sleep_counter: body.sleep_counter,
        }
    }
}

/// Bitwise snapshot of a collision event.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollisionEventSnapshot<const D: usize> {
    pub body_a: BodyHandle,
    pub body_b: BodyHandle,
    pub impulse: u64,
    pub normal: [u64; D],
    pub depth: u64,
}

impl<const D: usize> CollisionEventSnapshot<D> {
    pub fn from_event(event: &crate::contact::CollisionEvent<D>) -> Self {
        Self {
            body_a: event.body_a,
            body_b: event.body_b,
            impulse: event.impulse.to_bits(),
            normal: std::array::from_fn(|i| event.normal[i].to_bits()),
            depth: event.depth.to_bits(),
        }
    }
}

/// Bitwise snapshot of a physics world (bodies + last-step collision events).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldSnapshot<const D: usize> {
    pub bodies: Vec<BodySnapshot<D>>,
    pub collision_events: Vec<CollisionEventSnapshot<D>>,
}

impl<const D: usize> WorldSnapshot<D> {
    pub fn capture(world: &PhysicsWorld<D>) -> Self {
        let mut bodies: Vec<_> = world.bodies.iter().map(BodySnapshot::from_body).collect();
        bodies.sort_by_key(|b| b.handle);

        let mut collision_events: Vec<_> = world
            .collision_events
            .iter()
            .map(CollisionEventSnapshot::from_event)
            .collect();
        collision_events.sort_by_key(|e| (e.body_a, e.body_b, e.impulse, e.depth));

        Self {
            bodies,
            collision_events,
        }
    }
}


/// Maximum number of simulation frames accepted by one evidence receipt.
pub const MAX_REPLAY_EVIDENCE_FRAMES: usize = 1_000_000;
/// Schema identifier for scenario-bound replay evidence.
pub const REPLAY_EVIDENCE_SCHEMA: &str = "symtropy.physics.replay-evidence.v0.1";

/// Caller-supplied immutable context that names the scenario and execution
/// profile under which a tape is being replayed.
///
/// Digests are structural bindings only. The caller must provide authentic
/// artifact identities and verify that the engine actually used these inputs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayEvidenceContext {
    pub scenario_id: String,
    pub scenario_artifact_digest: String,
    pub engine_artifact_digest: String,
    pub initial_world_profile_digest: String,
    pub environment_profile_digest: String,
    pub execution_protocol_digest: String,
}

impl ReplayEvidenceContext {
    pub fn identity_is_valid(&self) -> bool {
        !self.scenario_id.trim().is_empty()
            && digest_is_valid(&self.scenario_artifact_digest)
            && digest_is_valid(&self.engine_artifact_digest)
            && digest_is_valid(&self.initial_world_profile_digest)
            && digest_is_valid(&self.environment_profile_digest)
            && digest_is_valid(&self.execution_protocol_digest)
    }
}

/// Integrity receipt for one complete execution of a replay tape.
///
/// Tick hashes are bitwise world snapshots from the existing physics replay
/// implementation. BLAKE3 fingerprints are not signatures or engine attestation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayEvidenceReceipt {
    pub schema_version: String,
    pub context: ReplayEvidenceContext,
    pub dimension: u32,
    pub frame_count: u64,
    pub input_tape_digest: String,
    pub initial_snapshot_digest: String,
    pub tick_snapshot_digests: Vec<String>,
    pub final_snapshot_digest: String,
    /// BLAKE3 over the preceding fields with this field empty; not a signature.
    pub receipt_digest: String,
}

impl ReplayEvidenceReceipt {
    pub fn identity_is_valid(&self) -> bool {
        if self.schema_version != REPLAY_EVIDENCE_SCHEMA
            || !self.context.identity_is_valid()
            || !(2..=4).contains(&self.dimension)
            || self.frame_count == 0
            || self.frame_count > MAX_REPLAY_EVIDENCE_FRAMES as u64
            || self.tick_snapshot_digests.len() as u64 != self.frame_count
            || !digest_is_valid(&self.input_tape_digest)
            || !digest_is_valid(&self.initial_snapshot_digest)
            || !digest_is_valid(&self.final_snapshot_digest)
            || !self.tick_snapshot_digests.iter().all(|digest| digest_is_valid(digest))
            || self.tick_snapshot_digests.last() != Some(&self.final_snapshot_digest)
            || !digest_is_valid(&self.receipt_digest)
        {
            return false;
        }

        let mut receipt = self.clone();
        let expected = receipt.receipt_digest.clone();
        receipt.receipt_digest.clear();
        serde_json::to_vec(&receipt)
            .ok()
            .map(|bytes| blake3::hash(&bytes).to_hex().to_string() == expected)
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayEvidenceError {
    InvalidContext,
    EmptyTape,
    FrameBudgetExceeded,
    InvalidTimeStep,
    InvalidCommandValue,
    MissingBody(BodyHandle),
    SerializationFailure,
}

/// Execute a deterministic replay tape and return a scenario-bound receipt.
///
/// The tape is preflight-validated in full before the world is mutated. Each
/// frame applies ordered commands, advances the physics world once, then
/// captures a bitwise snapshot. The output receipt binds caller-supplied
/// scenario/engine/environment/profile identities, the exact command tape,
/// initial snapshot, every tick snapshot and the final snapshot.
pub fn execute_with_evidence<const D: usize>(
    world: &mut PhysicsWorld<D>,
    tape: &ReplayTape<D>,
    context: &ReplayEvidenceContext,
) -> Result<ReplayEvidenceReceipt, ReplayEvidenceError> {
    if !context.identity_is_valid() {
        return Err(ReplayEvidenceError::InvalidContext);
    }
    validate_replay_tape(world, tape)?;

    let input_tape_digest = digest_replay_tape(tape);
    let initial_snapshot_digest = digest_world_snapshot(&WorldSnapshot::capture(world));
    let mut tick_snapshot_digests = Vec::with_capacity(tape.frames.len());

    for frame in &tape.frames {
        apply_commands(world, &frame.commands).map_err(|error| match error {
            ApplyCommandError::MissingBody(handle) => ReplayEvidenceError::MissingBody(handle),
        })?;
        world.step(frame.dt);
        tick_snapshot_digests.push(digest_world_snapshot(&WorldSnapshot::capture(world)));
    }

    let final_snapshot_digest = tick_snapshot_digests
        .last()
        .cloned()
        .ok_or(ReplayEvidenceError::EmptyTape)?;

    let mut receipt = ReplayEvidenceReceipt {
        schema_version: REPLAY_EVIDENCE_SCHEMA.into(),
        context: context.clone(),
        dimension: D as u32,
        frame_count: tape.frames.len() as u64,
        input_tape_digest,
        initial_snapshot_digest,
        tick_snapshot_digests,
        final_snapshot_digest,
        receipt_digest: String::new(),
    };
    let bytes =
        serde_json::to_vec(&receipt).map_err(|_| ReplayEvidenceError::SerializationFailure)?;
    receipt.receipt_digest = blake3::hash(&bytes).to_hex().to_string();
    Ok(receipt)
}

/// Execute the supplied tape against a separately constructed initial world
/// and compare its complete receipt with the expected record.
///
/// A true result means bitwise snapshots matched at every tick under the same
/// caller-supplied context. It does not prove cross-platform determinism unless
/// the engine/hardware/profile constraints are separately established.
pub fn verify_replay_evidence<const D: usize>(
    world: &mut PhysicsWorld<D>,
    tape: &ReplayTape<D>,
    context: &ReplayEvidenceContext,
    expected: &ReplayEvidenceReceipt,
) -> Result<bool, ReplayEvidenceError> {
    if !expected.identity_is_valid() || expected.context != *context {
        return Ok(false);
    }
    let actual = execute_with_evidence(world, tape, context)?;
    Ok(actual == *expected)
}

fn validate_replay_tape<const D: usize>(
    world: &PhysicsWorld<D>,
    tape: &ReplayTape<D>,
) -> Result<(), ReplayEvidenceError> {
    if tape.frames.is_empty() {
        return Err(ReplayEvidenceError::EmptyTape);
    }
    if tape.frames.len() > MAX_REPLAY_EVIDENCE_FRAMES {
        return Err(ReplayEvidenceError::FrameBudgetExceeded);
    }

    for frame in &tape.frames {
        if !frame.dt.is_finite() || frame.dt <= 0.0 {
            return Err(ReplayEvidenceError::InvalidTimeStep);
        }
        for command in &frame.commands {
            let (handle, valid_values) = match command {
                WorldCommand::ApplyForce { body, force } => {
                    (*body, force.iter().all(|value| value.is_finite()))
                }
                WorldCommand::ApplyImpulse { body, impulse } => {
                    (*body, impulse.iter().all(|value| value.is_finite()))
                }
                WorldCommand::SetLinearVelocity { body, velocity } => {
                    (*body, velocity.iter().all(|value| value.is_finite()))
                }
                WorldCommand::SetAngularVelocity { body, velocity } => {
                    let matrix = velocity.to_matrix();
                    (*body, matrix.iter().all(|value| value.is_finite()))
                }
                WorldCommand::Wake { body } => (*body, true),
            };
            if !valid_values {
                return Err(ReplayEvidenceError::InvalidCommandValue);
            }
            if !world.bodies.iter().any(|existing| existing.handle == handle) {
                return Err(ReplayEvidenceError::MissingBody(handle));
            }
        }
    }
    Ok(())
}

fn digest_replay_tape<const D: usize>(tape: &ReplayTape<D>) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"symtropy.physics.replay-tape.v1");
    hasher.update(&(D as u64).to_le_bytes());
    hasher.update(&(tape.frames.len() as u64).to_le_bytes());

    for frame in &tape.frames {
        hasher.update(&frame.dt.to_bits().to_le_bytes());
        hasher.update(&(frame.commands.len() as u64).to_le_bytes());
        for command in &frame.commands {
            match command {
                WorldCommand::ApplyForce { body, force } => {
                    hasher.update(&[0]);
                    hash_body_handle(&mut hasher, *body);
                    hash_vector(&mut hasher, force.as_ref());
                }
                WorldCommand::ApplyImpulse { body, impulse } => {
                    hasher.update(&[1]);
                    hash_body_handle(&mut hasher, *body);
                    hash_vector(&mut hasher, impulse.as_ref());
                }
                WorldCommand::SetLinearVelocity { body, velocity } => {
                    hasher.update(&[2]);
                    hash_body_handle(&mut hasher, *body);
                    hash_vector(&mut hasher, velocity.as_ref());
                }
                WorldCommand::SetAngularVelocity { body, velocity } => {
                    hasher.update(&[3]);
                    hash_body_handle(&mut hasher, *body);
                    let matrix = velocity.to_matrix();
                    for row in 0..D {
                        for column in 0..D {
                            hasher.update(&matrix[(row, column)].to_bits().to_le_bytes());
                        }
                    }
                }
                WorldCommand::Wake { body } => {
                    hasher.update(&[4]);
                    hash_body_handle(&mut hasher, *body);
                }
            }
        }
    }
    hasher.finalize().to_hex().to_string()
}

fn digest_world_snapshot<const D: usize>(snapshot: &WorldSnapshot<D>) -> String {
    let mut hasher = Hasher::new();
    hasher.update(b"symtropy.physics.world-snapshot.v1");
    hasher.update(&(D as u64).to_le_bytes());
    hasher.update(&(snapshot.bodies.len() as u64).to_le_bytes());

    for body in &snapshot.bodies {
        hash_body_handle(&mut hasher, body.handle);
        hasher.update(&[match body.body_type {
            BodyType::Static => 0,
            BodyType::Kinematic => 1,
            BodyType::Dynamic => 2,
        }]);
        for value in body.translation {
            hasher.update(&value.to_le_bytes());
        }
        for row in body.rotation {
            for value in row {
                hasher.update(&value.to_le_bytes());
            }
        }
        for value in body.linear_velocity {
            hasher.update(&value.to_le_bytes());
        }
        for row in body.angular_velocity {
            for value in row {
                hasher.update(&value.to_le_bytes());
            }
        }
        hasher.update(&[u8::from(body.sleeping)]);
        hasher.update(&body.sleep_counter.to_le_bytes());
    }

    hasher.update(&(snapshot.collision_events.len() as u64).to_le_bytes());
    for event in &snapshot.collision_events {
        hash_body_handle(&mut hasher, event.body_a);
        hash_body_handle(&mut hasher, event.body_b);
        hasher.update(&event.impulse.to_le_bytes());
        for value in event.normal {
            hasher.update(&value.to_le_bytes());
        }
        hasher.update(&event.depth.to_le_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

fn hash_vector<const D: usize>(hasher: &mut Hasher, vector: &SVector<f64, D>) {
    for value in vector.iter() {
        hasher.update(&value.to_bits().to_le_bytes());
    }
}

fn hash_body_handle(hasher: &mut Hasher, handle: BodyHandle) {
    hasher.update(&(handle.0 as u64).to_le_bytes());
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
