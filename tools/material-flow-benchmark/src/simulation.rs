// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reduced-order L1 receiving/put-away episode using Symtropy's existing
//! rigid-body engine and replay primitives.
//!
//! This models a rigid package proxy falling into an open-top tote. It does
//! not model the carrier journey, grasp planning, sensor metrology, contact
//! compliance of real packaging, actuator control, or a certified receiver.

use nalgebra::SVector;
use serde::Serialize;
use sha2::{Digest, Sha256};
use symtropy_math::{HyperBox, Point, Transform};
use symtropy_physics::{
    BodyHandle, BodyType, PhysicsWorld, ReplayTape, RigidBody, WorldCommand, WorldSnapshot,
    apply_commands,
};

use crate::transaction::{
    DispatchGate, PhysicalTransaction, PhysicalTransactionState, TransactionContext,
    TransactionEvent, TransactionEventKind,
};
use crate::{AuthorizationAction, EvidenceLevel, ScenarioManifest, UseLimit};

const FIXED_DT: f64 = 1.0 / 480.0;
const TICK_COUNT: usize = 1_920;
const RELEASE_IMPULSE_NS: f64 = 0.015;
const SETTLED_SPEED_M_S: f64 = 0.08;
const SETTLED_ANGULAR_SPEED_RAD_S: f64 = 0.08;
const MAX_CONTACT_PENETRATION_M: f64 = 0.005;
const ROTATION_HEALTH_TOLERANCE: f64 = 1e-8;
const PLACEMENT_CLEARANCE_M: f64 = 0.005;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FaultSimulationResult {
    pub schema_version: u16,
    pub scenario_id: String,
    pub seed: u64,
    pub fault_id: String,
    pub fault_model: String,
    pub injection_semantics: String,
    pub simulation: SimulationResult,
    pub gate_contact_event_count: usize,
    pub gate_contact_max_impulse_ns: f64,
    pub physical_fault_effect_observed: bool,
    pub blind_retry_blocked: bool,
    pub reconciliation_required: bool,
    pub false_completion_blocked: bool,
    pub fault_oracle_passed: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GateFaultSweepCase {
    pub case_index: usize,
    pub seed: u64,
    pub fault_oracle_passed: bool,
    pub gate_contact_event_count: usize,
    pub gate_contact_max_impulse_ns: f64,
    pub max_contact_penetration_m: f64,
    pub reconciliation_required: bool,
    pub blind_retry_blocked: bool,
    pub false_completion_blocked: bool,
    pub physics_trace_digest: Option<String>,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GateFaultSweepResult {
    pub schema_version: u16,
    pub scenario_id: String,
    pub source_commit: String,
    pub configuration_digest: String,
    pub seed_start: u64,
    pub requested_run_count: usize,
    pub executed_run_count: usize,
    pub passed_run_count: usize,
    pub failed_run_count: usize,
    pub all_fault_oracles_passed: bool,
    pub total_gate_contact_events: u64,
    pub minimum_gate_contact_events: usize,
    pub maximum_gate_contact_events: usize,
    pub maximum_gate_contact_impulse_ns: f64,
    pub maximum_contact_penetration_m: f64,
    pub reconciliation_run_count: usize,
    pub blind_retry_blocked_run_count: usize,
    pub false_completion_blocked_run_count: usize,
    pub cases: Vec<GateFaultSweepCase>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SimulationResult {
    pub schema_version: u16,
    pub scenario_id: String,
    pub seed: u64,
    pub evidence_level: EvidenceLevel,
    pub simulation_track: String,
    pub physics_engine: String,
    pub tick_count: usize,
    pub fixed_dt_seconds: f64,
    pub package_center_m: [f64; 3],
    pub package_linear_speed_m_s: f64,
    pub package_angular_speed_rad_s: f64,
    pub package_world_aabb_half_extents_m: [f64; 3],
    pub max_contact_penetration_m: f64,
    pub max_rotation_orthogonality_error: f64,
    pub max_rotation_determinant_error: f64,
    pub non_finite_body_count: usize,
    pub physics_numerically_healthy: bool,
    pub safe_inside_tote: bool,
    pub settled: bool,
    pub replay_bitwise_equal: bool,
    pub physics_trace_digest: String,
    pub transaction_state: PhysicalTransactionState,
    pub accepted: bool,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
struct EpisodeTrace {
    final_position_m: [f64; 3],
    final_speed_m_s: f64,
    final_angular_speed_rad_s: f64,
    final_world_aabb_half_extents_m: [f64; 3],
    max_contact_penetration_m: f64,
    max_rotation_orthogonality_error: f64,
    max_rotation_determinant_error: f64,
    non_finite_body_count: usize,
    gate_contact_event_count: usize,
    gate_contact_max_impulse_ns: f64,
    digest: String,
    ticks: usize,
}

#[derive(Debug, Clone, Copy)]
struct ToteGeometry {
    inner_half_m: [f64; 3],
    outer_half_m: [f64; 3],
    wall_half_m: [f64; 3],
}

struct BuiltWorld {
    world: PhysicsWorld<3>,
    package_handle: BodyHandle,
    package_half_m: [f64; 3],
    gate_obstruction_handle: Option<BodyHandle>,
}

fn millimetres(value: u32) -> f64 {
    value as f64 / 1_000.0
}

fn tote_geometry(scenario: &ScenarioManifest) -> Result<ToteGeometry, String> {
    // Physics coordinates are X=length, Y=height, Z=width.
    let inner = &scenario.container.internal_dimensions_mm;
    let outer = &scenario.container.external_dimensions_mm;
    let inner_half_m = [
        millimetres(inner.length_mm) / 2.0,
        millimetres(inner.height_mm) / 2.0,
        millimetres(inner.width_mm) / 2.0,
    ];
    let outer_half_m = [
        millimetres(outer.length_mm) / 2.0,
        millimetres(outer.height_mm) / 2.0,
        millimetres(outer.width_mm) / 2.0,
    ];
    let wall_half_m = [
        (outer_half_m[0] - inner_half_m[0]) / 2.0,
        outer_half_m[1] - inner_half_m[1],
        (outer_half_m[2] - inner_half_m[2]) / 2.0,
    ];
    if wall_half_m.iter().any(|thickness| *thickness < 0.003) {
        return Err(
            "L1 tote scene requires at least 6 mm external-minus-internal shell thickness on X/Z and 3 mm base thickness on Y"
                .into(),
        );
    }
    Ok(ToteGeometry {
        inner_half_m,
        outer_half_m,
        wall_half_m,
    })
}

fn seeded_offset(seed: u64, lane: u64) -> f64 {
    let value = (seed / lane.max(1)) % 101;
    (value as f64 - 50.0) * 0.00005
}

fn add_static_box(
    world: &mut PhysicsWorld<3>,
    position_m: [f64; 3],
    half_extents_m: [f64; 3],
) -> BodyHandle {
    let mut body = RigidBody::static_body(
        BodyHandle(0),
        Point::new(position_m),
        Box::new(HyperBox::<3>::new(half_extents_m)),
    );
    body.friction = 0.9;
    body.restitution = 0.0;
    world.add_body(body)
}

fn build_world(
    scenario: &ScenarioManifest,
    closed_gate_obstruction: bool,
) -> Result<BuiltWorld, String> {
    let geometry = tote_geometry(scenario)?;
    let package = &scenario.package;
    let package_half_m = [
        millimetres(package.dimensions_mm.length_mm) / 2.0,
        millimetres(package.dimensions_mm.height_mm) / 2.0,
        millimetres(package.dimensions_mm.width_mm) / 2.0,
    ];
    if package_half_m
        .iter()
        .zip(geometry.inner_half_m.iter())
        .any(|(package_half, inner_half)| package_half >= inner_half)
    {
        return Err("package has no positive clearance inside the L1 tote scene".into());
    }

    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
    world.solver_iterations = 16;
    world.slop = 0.001;
    world.baumgarte = 0.2;

    let [inner_x, inner_y, inner_z] = geometry.inner_half_m;
    let [outer_x, _outer_y, outer_z] = geometry.outer_half_m;
    let [wall_x, base_half_y, wall_z] = geometry.wall_half_m;

    // Open-top tote: base, two long walls, and two end walls.
    add_static_box(
        &mut world,
        [0.0, -base_half_y, 0.0],
        [outer_x, base_half_y, outer_z],
    );
    add_static_box(
        &mut world,
        [-(inner_x + wall_x), inner_y, 0.0],
        [wall_x, inner_y, outer_z],
    );
    add_static_box(
        &mut world,
        [inner_x + wall_x, inner_y, 0.0],
        [wall_x, inner_y, outer_z],
    );
    add_static_box(
        &mut world,
        [0.0, inner_y, -(inner_z + wall_z)],
        [inner_x, inner_y, wall_z],
    );
    add_static_box(
        &mut world,
        [0.0, inner_y, inner_z + wall_z],
        [inner_x, inner_y, wall_z],
    );

    // L1 obstruction proxy: a closed, immobile gate spans the tote opening.
    // This is static collision geometry, not a model of gate actuation or
    // interlock timing. The fault path observes the failed placement after
    // the logical release has been issued.
    let gate_obstruction_handle = if closed_gate_obstruction {
        Some(add_static_box(
            &mut world,
            [0.0, 2.0 * inner_y + 0.01, 0.0],
            [outer_x, 0.01, outer_z],
        ))
    } else {
        None
    };

    let mass_kg = package.mass_g as f64 / 1_000.0;
    let [half_x, half_y, half_z] = package_half_m;
    let inertia = SVector::from([
        mass_kg * (half_y * half_y + half_z * half_z) / 3.0,
        mass_kg * (half_x * half_x + half_z * half_z) / 3.0,
        mass_kg * (half_x * half_x + half_y * half_y) / 3.0,
    ]);
    let start_x = seeded_offset(scenario.seed, 1);
    let start_z = seeded_offset(scenario.seed, 7);
    let start_y = 2.0 * inner_y + half_y + 0.15;
    let initial_position = [start_x, start_y, start_z];

    let mut package_body = RigidBody::new(
        BodyHandle(0),
        BodyType::Dynamic,
        Transform::from_translation(Point::new(initial_position)),
        Box::new(HyperBox::<3>::new(package_half_m)),
        mass_kg,
        inertia,
    );
    package_body.friction = 0.9;
    package_body.restitution = 0.0;
    package_body.linear_damping = 0.08;
    package_body.angular_damping = 0.08;

    let package_handle = world.add_body(package_body);
    Ok(BuiltWorld {
        world,
        package_handle,
        package_half_m,
        gate_obstruction_handle,
    })
}

fn hash_snapshot(hasher: &mut Sha256, tick: usize, snapshot: &WorldSnapshot<3>) {
    hasher.update((tick as u64).to_le_bytes());
    hasher.update((snapshot.bodies.len() as u64).to_le_bytes());
    for body in &snapshot.bodies {
        hasher.update((body.handle.0 as u64).to_le_bytes());
        hasher.update([match body.body_type {
            BodyType::Static => 0,
            BodyType::Kinematic => 1,
            BodyType::Dynamic => 2,
        }]);
        for value in body.translation {
            hasher.update(value.to_le_bytes());
        }
        for row in body.rotation {
            for value in row {
                hasher.update(value.to_le_bytes());
            }
        }
        for value in body.linear_velocity {
            hasher.update(value.to_le_bytes());
        }
        for row in body.angular_velocity {
            for value in row {
                hasher.update(value.to_le_bytes());
            }
        }
        hasher.update([u8::from(body.sleeping)]);
        hasher.update(body.sleep_counter.to_le_bytes());
    }
    hasher.update((snapshot.collision_events.len() as u64).to_le_bytes());
    for event in &snapshot.collision_events {
        hasher.update((event.body_a.0 as u64).to_le_bytes());
        hasher.update((event.body_b.0 as u64).to_le_bytes());
        hasher.update(event.impulse.to_le_bytes());
        for value in event.normal {
            hasher.update(value.to_le_bytes());
        }
        hasher.update(event.depth.to_le_bytes());
    }
}

fn run_episode(
    scenario: &ScenarioManifest,
    closed_gate_obstruction: bool,
) -> Result<EpisodeTrace, String> {
    let BuiltWorld {
        mut world,
        package_handle,
        package_half_m: _package_half,
        gate_obstruction_handle: gate_handle,
    } = build_world(scenario, closed_gate_obstruction)?;

    let impulse = SVector::from([0.0, -RELEASE_IMPULSE_NS, 0.0]);
    let mut tape = ReplayTape::<3>::default();
    for tick in 0..TICK_COUNT {
        let commands = if tick == 0 {
            vec![WorldCommand::ApplyImpulse {
                body: package_handle,
                impulse: Box::new(impulse),
            }]
        } else {
            Vec::new()
        };
        tape.push_frame(FIXED_DT, commands);
    }

    let mut hasher = Sha256::new();
    hasher.update(b"MFB-L1-SYMTROPY-PHYSICS-TRACE-V1\0");
    hasher.update(scenario.scenario_id.as_bytes());
    hasher.update(scenario.seed.to_le_bytes());
    hasher.update((TICK_COUNT as u64).to_le_bytes());
    hasher.update(FIXED_DT.to_bits().to_le_bytes());

    let mut max_contact_penetration_m = 0.0_f64;
    let mut max_rotation_orthogonality_error = 0.0_f64;
    let mut max_rotation_determinant_error = 0.0_f64;
    let mut non_finite_body_count = 0_usize;
    let mut gate_contact_event_count = 0_usize;
    let mut gate_contact_max_impulse_ns = 0.0_f64;
    for (tick, frame) in tape.frames.iter().enumerate() {
        apply_commands(&mut world, &frame.commands)
            .map_err(|error| format!("replay command failed at tick {tick}: {error:?}"))?;
        world.step(frame.dt);
        let snapshot = WorldSnapshot::capture(&world);
        hash_snapshot(&mut hasher, tick, &snapshot);
        if let Some(gate_handle) = gate_handle {
            for event in &snapshot.collision_events {
                let package_gate_pair = (event.body_a == package_handle
                    && event.body_b == gate_handle)
                    || (event.body_a == gate_handle && event.body_b == package_handle);
                if package_gate_pair {
                    gate_contact_event_count += 1;
                    gate_contact_max_impulse_ns =
                        gate_contact_max_impulse_ns.max(event.impulse as f64);
                }
            }
        }

        let invariants = world.invariant_snapshot();
        max_contact_penetration_m = max_contact_penetration_m.max(invariants.max_penetration_depth);
        max_rotation_orthogonality_error =
            max_rotation_orthogonality_error.max(invariants.max_rotation_orthogonality_error);
        max_rotation_determinant_error =
            max_rotation_determinant_error.max(invariants.max_rotation_determinant_error);
        non_finite_body_count = non_finite_body_count.max(invariants.non_finite_body_count);
    }

    let package_body = world
        .body(package_handle)
        .ok_or_else(|| "package body disappeared from physics world".to_string())?;
    let position = package_body.position();
    let final_position_m = [position[0], position[1], position[2]];
    let final_speed_m_s = package_body.linear_velocity.norm();
    let final_angular_speed_rad_s = package_body.angular_velocity.norm();
    let rotation = package_body.transform.rotation.to_matrix();
    let final_world_aabb_half_extents_m = std::array::from_fn(|world_axis| {
        (0..3)
            .map(|local_axis| rotation[(world_axis, local_axis)].abs() * _package_half[local_axis])
            .sum()
    });

    Ok(EpisodeTrace {
        final_position_m,
        final_speed_m_s,
        final_angular_speed_rad_s,
        final_world_aabb_half_extents_m,
        max_contact_penetration_m,
        max_rotation_orthogonality_error,
        max_rotation_determinant_error,
        non_finite_body_count,
        gate_contact_event_count,
        gate_contact_max_impulse_ns,
        digest: format!("sha256:{:x}", hasher.finalize()),
        ticks: tape.frames.len(),
    })
}

fn custody_transaction(
    scenario: &ScenarioManifest,
    trace_digest: &str,
    physical_result_valid: bool,
) -> Result<(PhysicalTransactionState, bool), String> {
    let authorization = &scenario.authorization;
    let reconciler = scenario
        .actors
        .iter()
        .find(|actor| actor.role == crate::ActorRole::Custodian)
        .ok_or_else(|| {
            "L1 simulation requires a declared independent Custodian actor".to_string()
        })?;

    let transaction_id = format!("tx-{}", scenario.scenario_id);
    let command_id = format!("cmd-{}", scenario.scenario_id);
    let mut transaction = PhysicalTransaction::new(TransactionContext {
        transaction_id: transaction_id.clone(),
        expected_command_id: command_id.clone(),
        package_id: scenario.package.package_id.clone(),
        principal_actor_id: authorization.principal_actor_id.clone(),
        source_actor_id: authorization.audience_actor_id.clone(),
        receiver_actor_id: authorization.receiver_device_actor_id.clone(),
        receiver_endpoint_id: authorization.receiver_endpoint_node_id.clone(),
        reconciler_actor_id: reconciler.actor_id.clone(),
    });

    let use_remaining = match authorization.use_limit {
        UseLimit::OneShot => true,
        UseLimit::Bounded { maximum_uses } => maximum_uses > 0,
    };
    let scope_matches = authorization
        .actions
        .contains(&AuthorizationAction::ReceiveExternalPackage)
        && authorization
            .actions
            .contains(&AuthorizationAction::TransferToTote);
    let authorization_event = TransactionEvent {
        transaction_id: transaction_id.clone(),
        event_id: "event-authorization-checked".into(),
        sequence: 1,
        actor_id: authorization.principal_actor_id.clone(),
        kind: TransactionEventKind::AuthorizationGranted {
            not_before_unix_ms: authorization.not_before_unix_ms,
            expires_at_unix_ms: authorization.expires_at_unix_ms,
            checked_at_unix_ms: scenario.started_at_unix_ms,
            audience_matches: true,
            endpoint_matches: true,
            scope_matches,
            use_remaining,
        },
    };
    transaction
        .record_event(authorization_event)
        .map_err(|error| format!("authorization event rejected: {error}"))?;
    transaction
        .request_release_command(&command_id)
        .map_err(|error| format!("release command rejected: {error}"))?;

    transaction
        .record_event(TransactionEvent {
            transaction_id: transaction_id.clone(),
            event_id: "event-package-released".into(),
            sequence: 2,
            actor_id: authorization.audience_actor_id.clone(),
            kind: TransactionEventKind::PackageReleased {
                command_id: command_id.clone(),
                package_id: scenario.package.package_id.clone(),
            },
        })
        .map_err(|error| format!("release observation rejected: {error}"))?;

    if physical_result_valid {
        transaction
            .record_event(TransactionEvent {
                transaction_id: transaction_id.clone(),
                event_id: "event-package-received".into(),
                sequence: 3,
                actor_id: authorization.receiver_device_actor_id.clone(),
                kind: TransactionEventKind::PackageReceived {
                    command_id: command_id.clone(),
                    package_id: scenario.package.package_id.clone(),
                    endpoint_id: authorization.receiver_endpoint_node_id.clone(),
                },
            })
            .map_err(|error| format!("receiver observation rejected: {error}"))?;
        transaction
            .record_event(TransactionEvent {
                transaction_id,
                event_id: "event-custody-acknowledged".into(),
                sequence: 4,
                actor_id: authorization.receiver_device_actor_id.clone(),
                kind: TransactionEventKind::CustodyAcknowledged {
                    command_id: command_id.clone(),
                    package_id: scenario.package.package_id.clone(),
                    evidence_digest: trace_digest.into(),
                },
            })
            .map_err(|error| format!("custody acknowledgement rejected: {error}"))?;
    } else {
        transaction
            .record_event(TransactionEvent {
                transaction_id,
                event_id: "event-physical-observation-disagreement".into(),
                sequence: 3,
                actor_id: authorization.receiver_device_actor_id.clone(),
                kind: TransactionEventKind::SensorDisagreement,
            })
            .map_err(|error| format!("sensor disagreement rejected: {error}"))?;
    }

    let blind_retry_blocked = if physical_result_valid {
        false
    } else {
        matches!(
            transaction.request_release_command(&command_id),
            Ok(DispatchGate::BlockedByState(
                PhysicalTransactionState::ReconciliationRequired
            ))
        )
    };

    Ok((transaction.state, blind_retry_blocked))
}

/// Execute the L1 scene with either the nominal open tote or the static
/// closed-gate obstruction proxy, then replay the same scene bitwise.
fn simulate_scene(
    scenario: &ScenarioManifest,
    closed_gate_obstruction: bool,
) -> Result<(SimulationResult, bool, EpisodeTrace), String> {
    scenario.validate().map_err(|errors| errors.join("; "))?;
    if scenario.evidence_level != EvidenceLevel::L1Simulation {
        return Err("MFB simulation accepts only L1 simulation scenarios".into());
    }

    let first = run_episode(scenario, closed_gate_obstruction)?;
    let replay = run_episode(scenario, closed_gate_obstruction)?;
    let replay_bitwise_equal = first.digest == replay.digest
        && first.final_position_m.map(f64::to_bits) == replay.final_position_m.map(f64::to_bits)
        && first.final_speed_m_s.to_bits() == replay.final_speed_m_s.to_bits();

    let geometry = tote_geometry(scenario)?;
    let [x, y, z] = first.final_position_m;
    let [ix, iy, iz] = geometry.inner_half_m;
    let [aabb_x, aabb_y, aabb_z] = first.final_world_aabb_half_extents_m;
    let safe_inside_tote = x.abs() + aabb_x <= ix + PLACEMENT_CLEARANCE_M
        && z.abs() + aabb_z <= iz + PLACEMENT_CLEARANCE_M
        && y - aabb_y >= -PLACEMENT_CLEARANCE_M
        && y + aabb_y <= 2.0 * iy + PLACEMENT_CLEARANCE_M;
    let settled = first.final_speed_m_s <= SETTLED_SPEED_M_S
        && first.final_angular_speed_rad_s <= SETTLED_ANGULAR_SPEED_RAD_S;
    let physics_numerically_healthy = first.non_finite_body_count == 0
        && first.max_rotation_orthogonality_error <= ROTATION_HEALTH_TOLERANCE
        && first.max_rotation_determinant_error <= ROTATION_HEALTH_TOLERANCE;
    let penetration_within_limit = first.max_contact_penetration_m <= MAX_CONTACT_PENETRATION_M;

    let physical_result_valid = replay_bitwise_equal
        && safe_inside_tote
        && settled
        && physics_numerically_healthy
        && penetration_within_limit;
    let (transaction_state, blind_retry_blocked) =
        custody_transaction(scenario, &first.digest, physical_result_valid)?;
    let accepted =
        physical_result_valid && transaction_state == PhysicalTransactionState::Completed;

    let mut limitations = vec![
        "Reduced-order rigid-box package proxy; not real packaging/contact compliance".into(),
        "Stationary receiver only; no truck, aircraft, route, launch/recovery or fleet simulation".into(),
        "No sensor noise, calibration model, metrology or perception pipeline".into(),
        "No actuator, robot, door, latch, guarding or emergency-stop hardware".into(),
        "Same-host deterministic replay only; cross-platform bitwise equality is not claimed".into(),
        "Asymmetric-body inertia is currently approximate in symtropy-physics; this centered drop is not a tumbling-body qualification".into(),
        "Custody state machine validates declared event structure, not signature authenticity or evidence-source independence".into(),
    ];
    if closed_gate_obstruction {
        limitations.push(
            "Closed gate is a static collision proxy; gate motion, interlock sensing, firmware and actuation timing are not simulated".into(),
        );
    }

    Ok((
        SimulationResult {
            schema_version: scenario.schema_version,
            scenario_id: scenario.scenario_id.clone(),
            seed: scenario.seed,
            evidence_level: scenario.evidence_level,
            simulation_track: if closed_gate_obstruction {
                "stationary_receiver_closed_gate_obstruction_v1".into()
            } else {
                "stationary_receiver_rigid_package_drop_v1".into()
            },
            physics_engine: "symtropy-physics 0.2.1 / deterministic-net Q16.16".into(),
            tick_count: first.ticks,
            fixed_dt_seconds: FIXED_DT,
            package_center_m: first.final_position_m,
            package_linear_speed_m_s: first.final_speed_m_s,
            package_angular_speed_rad_s: first.final_angular_speed_rad_s,
            package_world_aabb_half_extents_m: first.final_world_aabb_half_extents_m,
            max_contact_penetration_m: first.max_contact_penetration_m,
            max_rotation_orthogonality_error: first.max_rotation_orthogonality_error,
            max_rotation_determinant_error: first.max_rotation_determinant_error,
            non_finite_body_count: first.non_finite_body_count,
            physics_numerically_healthy,
            safe_inside_tote,
            settled,
            replay_bitwise_equal,
            physics_trace_digest: first.digest.clone(),
            transaction_state,
            accepted,
            limitations,
        },
        blind_retry_blocked,
        first,
    ))
}

/// Run the nominal stationary-carrier receiving scene twice and require a
/// bitwise-identical replay before accepting custody completion.
pub fn simulate_nominal(scenario: &ScenarioManifest) -> Result<SimulationResult, String> {
    simulate_scene(scenario, false).map(|(result, _, _)| result)
}

/// Execute a real L1 collision-geometry fault: a closed gate obstruction spans
/// the tote opening. The package must remain outside the tote, the handoff must
/// require reconciliation, and a retry of the old command must be blocked.
pub fn simulate_closed_gate_obstruction(
    scenario: &ScenarioManifest,
) -> Result<FaultSimulationResult, String> {
    let (simulation, blind_retry_blocked, episode_trace) = simulate_scene(scenario, true)?;
    let geometry = tote_geometry(scenario)?;
    let package_bottom_m =
        simulation.package_center_m[1] - simulation.package_world_aabb_half_extents_m[1];
    let tote_top_m = 2.0 * geometry.inner_half_m[1];
    let physical_fault_effect_observed = episode_trace.gate_contact_event_count > 0
        && episode_trace.gate_contact_max_impulse_ns > 0.0
        && !simulation.safe_inside_tote
        && package_bottom_m > tote_top_m + 0.001;
    let reconciliation_required =
        simulation.transaction_state == PhysicalTransactionState::ReconciliationRequired;
    let false_completion_blocked = !simulation.accepted && reconciliation_required;
    let fault_oracle_passed = physical_fault_effect_observed
        && simulation.replay_bitwise_equal
        && simulation.settled
        && simulation.physics_numerically_healthy
        && simulation.max_contact_penetration_m <= MAX_CONTACT_PENETRATION_M
        && reconciliation_required
        && blind_retry_blocked
        && false_completion_blocked;

    Ok(FaultSimulationResult {
        schema_version: simulation.schema_version,
        scenario_id: simulation.scenario_id.clone(),
        seed: simulation.seed,
        fault_id: "fault-closed-gate-obstruction-after-release".into(),
        fault_model: "static_closed_gate_collision_proxy".into(),
        injection_semantics: "receiver_observes_placement_failure_after_logical_release".into(),
        simulation,
        gate_contact_event_count: episode_trace.gate_contact_event_count,
        gate_contact_max_impulse_ns: episode_trace.gate_contact_max_impulse_ns,
        physical_fault_effect_observed,
        blind_retry_blocked,
        reconciliation_required,
        false_completion_blocked,
        fault_oracle_passed,
        limitations: vec![
            "This is an L1 rigid-body collision-geometry fault, not a moving gate/interlock controller test".into(),
            "The gate proxy is static and present in the scene; actuation timing and sensor/firmware behavior are not modeled".into(),
            "A passing oracle qualifies only the reduced-order geometry and transaction response under this deterministic case".into(),
        ],
    })
}

/// Run a deterministic seed sweep for the static gate-obstruction scene.
/// Every requested seed is retained in the report, including simulation
/// errors and failed fault oracles; cases are never silently dropped.
pub fn simulate_closed_gate_obstruction_sweep(
    scenario: &ScenarioManifest,
    run_count: usize,
) -> Result<GateFaultSweepResult, String> {
    const MAX_SWEEP_RUNS: usize = 256;
    if !(1..=MAX_SWEEP_RUNS).contains(&run_count) {
        return Err(format!(
            "gate-fault sweep run_count must be in 1..={MAX_SWEEP_RUNS}"
        ));
    }
    scenario.validate().map_err(|errors| errors.join("; "))?;

    let mut cases = Vec::with_capacity(run_count);
    let mut passed_run_count = 0_usize;
    let mut total_gate_contact_events = 0_u64;
    let mut minimum_gate_contact_events = usize::MAX;
    let mut maximum_gate_contact_events = 0_usize;
    let mut maximum_gate_contact_impulse_ns = 0.0_f64;
    let mut maximum_contact_penetration_m = 0.0_f64;
    let mut reconciliation_run_count = 0_usize;
    let mut blind_retry_blocked_run_count = 0_usize;
    let mut false_completion_blocked_run_count = 0_usize;

    for case_index in 0..run_count {
        let seed = scenario
            .seed
            .checked_add(case_index as u64)
            .ok_or_else(|| "gate-fault seed range overflows u64".to_string())?;
        let mut case_scenario = scenario.clone();
        case_scenario.seed = seed;

        match simulate_closed_gate_obstruction(&case_scenario) {
            Ok(result) => {
                total_gate_contact_events = total_gate_contact_events
                    .saturating_add(result.gate_contact_event_count as u64);
                minimum_gate_contact_events =
                    minimum_gate_contact_events.min(result.gate_contact_event_count);
                maximum_gate_contact_events =
                    maximum_gate_contact_events.max(result.gate_contact_event_count);
                maximum_gate_contact_impulse_ns =
                    maximum_gate_contact_impulse_ns.max(result.gate_contact_max_impulse_ns);
                maximum_contact_penetration_m =
                    maximum_contact_penetration_m.max(result.simulation.max_contact_penetration_m);
                if result.reconciliation_required {
                    reconciliation_run_count += 1;
                }
                if result.blind_retry_blocked {
                    blind_retry_blocked_run_count += 1;
                }
                if result.false_completion_blocked {
                    false_completion_blocked_run_count += 1;
                }
                if result.fault_oracle_passed {
                    passed_run_count += 1;
                }
                cases.push(GateFaultSweepCase {
                    case_index,
                    seed,
                    fault_oracle_passed: result.fault_oracle_passed,
                    gate_contact_event_count: result.gate_contact_event_count,
                    gate_contact_max_impulse_ns: result.gate_contact_max_impulse_ns,
                    max_contact_penetration_m: result.simulation.max_contact_penetration_m,
                    reconciliation_required: result.reconciliation_required,
                    blind_retry_blocked: result.blind_retry_blocked,
                    false_completion_blocked: result.false_completion_blocked,
                    physics_trace_digest: Some(result.simulation.physics_trace_digest),
                    failure_reason: (!result.fault_oracle_passed)
                        .then(|| "one or more per-case fault oracle conditions failed".into()),
                });
            }
            Err(error) => {
                cases.push(GateFaultSweepCase {
                    case_index,
                    seed,
                    fault_oracle_passed: false,
                    gate_contact_event_count: 0,
                    gate_contact_max_impulse_ns: 0.0,
                    max_contact_penetration_m: 0.0,
                    reconciliation_required: false,
                    blind_retry_blocked: false,
                    false_completion_blocked: false,
                    physics_trace_digest: None,
                    failure_reason: Some(error),
                });
            }
        }
    }

    let failed_run_count = run_count - passed_run_count;
    if minimum_gate_contact_events == usize::MAX {
        minimum_gate_contact_events = 0;
    }

    Ok(GateFaultSweepResult {
        schema_version: scenario.schema_version,
        scenario_id: scenario.scenario_id.clone(),
        source_commit: scenario.digests.source_commit.clone(),
        configuration_digest: scenario.digests.configuration_digest.clone(),
        seed_start: scenario.seed,
        requested_run_count: run_count,
        executed_run_count: cases.len(),
        passed_run_count,
        failed_run_count,
        all_fault_oracles_passed: passed_run_count == run_count,
        total_gate_contact_events,
        minimum_gate_contact_events,
        maximum_gate_contact_events,
        maximum_gate_contact_impulse_ns,
        maximum_contact_penetration_m,
        reconciliation_run_count,
        blind_retry_blocked_run_count,
        false_completion_blocked_run_count,
        cases,
        limitations: vec![
            "Each case varies the deterministic initial package offsets via scenario seed; it is not an independent home, robot, sensor or hardware sample".into(),
            "All runs use the same static gate geometry, physics engine, package/tote configuration and fixed timestep".into(),
            "This remains an L1 synthetic obstruction study; it does not establish HIL, guarded-rig or field reliability".into(),
            "Execution errors remain explicit failed cases and are not removed from the denominator".into(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/nominal-stationary-carrier.json");

    fn fixture() -> ScenarioManifest {
        serde_json::from_str(FIXTURE).expect("nominal MFB fixture must parse")
    }

    #[test]
    fn nominal_receiver_drop_is_inside_tote_and_settled() {
        let result = simulate_nominal(&fixture()).expect("nominal L1 episode should execute");
        assert!(result.replay_bitwise_equal);
        assert!(result.safe_inside_tote, "{result:?}");
        assert!(result.settled, "{result:?}");
        assert!(result.physics_numerically_healthy, "{result:?}");
        assert!(result.max_contact_penetration_m <= MAX_CONTACT_PENETRATION_M);
        assert_eq!(result.non_finite_body_count, 0);
        assert_eq!(
            result.transaction_state,
            PhysicalTransactionState::Completed
        );
        assert!(result.accepted);
    }

    #[test]
    fn closed_gate_obstruction_is_observed_physically_and_forces_reconciliation() {
        let result = simulate_closed_gate_obstruction(&fixture())
            .expect("closed-gate obstruction episode should execute");
        assert!(result.physical_fault_effect_observed, "{result:?}");
        assert!(result.gate_contact_event_count > 0, "{result:?}");
        assert!(
            result.gate_contact_max_impulse_ns.is_finite()
                && result.gate_contact_max_impulse_ns > 0.0,
            "{result:?}"
        );
        assert!(result.simulation.replay_bitwise_equal, "{result:?}");
        assert!(result.simulation.settled, "{result:?}");
        assert!(result.simulation.physics_numerically_healthy, "{result:?}");
        assert!(
            result.simulation.max_contact_penetration_m <= MAX_CONTACT_PENETRATION_M,
            "{result:?}"
        );
        assert!(!result.simulation.safe_inside_tote, "{result:?}");
        assert_eq!(
            result.simulation.transaction_state,
            PhysicalTransactionState::ReconciliationRequired
        );
        assert!(result.reconciliation_required);
        assert!(result.blind_retry_blocked);
        assert!(result.false_completion_blocked);
        assert!(!result.simulation.accepted);
        assert!(result.fault_oracle_passed, "{result:?}");
    }

    #[test]
    fn small_gate_fault_sweep_retains_all_cases_and_is_repeatable() {
        let first = simulate_closed_gate_obstruction_sweep(&fixture(), 3)
            .expect("three-seed gate sweep should execute");
        let replay = simulate_closed_gate_obstruction_sweep(&fixture(), 3)
            .expect("repeated three-seed gate sweep should execute");
        assert_eq!(first, replay);
        assert_eq!(first.requested_run_count, 3);
        assert_eq!(first.executed_run_count, 3);
        assert_eq!(first.passed_run_count, 3, "{first:?}");
        assert_eq!(first.failed_run_count, 0);
        assert!(first.all_fault_oracles_passed);
        assert_eq!(first.reconciliation_run_count, 3);
        assert_eq!(first.blind_retry_blocked_run_count, 3);
        assert_eq!(first.false_completion_blocked_run_count, 3);
        assert!(first.minimum_gate_contact_events > 0);
        assert!(first.maximum_gate_contact_impulse_ns > 0.0);
        assert!(
            first
                .cases
                .iter()
                .all(|case| case.physics_trace_digest.is_some())
        );
    }

    #[test]
    fn gate_fault_sweep_rejects_zero_and_oversized_run_counts() {
        assert!(simulate_closed_gate_obstruction_sweep(&fixture(), 0).is_err());
        assert!(simulate_closed_gate_obstruction_sweep(&fixture(), 257).is_err());
    }

    #[test]
    fn seed_is_bound_into_the_physics_trace() {
        let baseline = simulate_nominal(&fixture()).expect("baseline episode should execute");
        let mut other_seed = fixture();
        other_seed.seed += 1;
        let changed = simulate_nominal(&other_seed).expect("seeded episode should execute");
        assert_ne!(baseline.physics_trace_digest, changed.physics_trace_digest);
        assert!(changed.replay_bitwise_equal);
    }

    #[test]
    fn non_l1_manifest_is_not_mislabelled_as_simulation() {
        let mut scenario = fixture();
        scenario.evidence_level = EvidenceLevel::L0Contract;
        assert!(simulate_nominal(&scenario).is_err());
    }
}
