// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! First executable native-vs-Rapier differential harness.
//!
//! This integration test intentionally keeps the shared validity domain narrow:
//! 3D axis-aligned spheres/cuboids, static/dynamic bodies, explicit SI gravity,
//! and position/linear-velocity traces. Rapier is independent evidence, not
//! canonical Symtropy physical truth.

use std::collections::BTreeMap;
use std::fmt;

use bevy::prelude::Vec3;
use rapier3d::prelude::*;
use symtropy_physics::validation::{
    BodyObservation3d, PHYSICS_SCENARIO_SCHEMA_VERSION, PHYSICS_TRACE_SCHEMA_VERSION,
    PhysicsBackendDescriptor, PhysicsSample3d, PhysicsScenario3d, PhysicsTrace3d, ScenarioBody3d,
    ScenarioBodyType, ScenarioShape3d, ScenarioValidationError, ValidationBodyId, compare_traces_3d,
    run_native_scenario_3d,
};
use symtropy_physics::world::NoOpCallback;
use symtropy_rapier3d_bridge::RapierPhysicsBridge;

const RAPIER_REFERENCE_VERSION: &str = "0.18";

#[derive(Clone, Debug, PartialEq, Eq)]
enum RapierValidationError {
    Scenario(ScenarioValidationError),
    OutsideReferenceRange(&'static str),
    MissingBodyIdentity { index: u32, generation: u32 },
}

impl fmt::Display for RapierValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scenario(error) => write!(f, "invalid common scenario: {error}"),
            Self::OutsideReferenceRange(field) => write!(
                f,
                "{field} cannot be represented by the Rapier f32 reference lane"
            ),
            Self::MissingBodyIdentity { index, generation } => write!(
                f,
                "Rapier observation ({index}, {generation}) has no validation body identity"
            ),
        }
    }
}

impl std::error::Error for RapierValidationError {}

impl From<ScenarioValidationError> for RapierValidationError {
    fn from(error: ScenarioValidationError) -> Self {
        Self::Scenario(error)
    }
}

fn run_rapier_scenario_3d(
    scenario: &PhysicsScenario3d,
) -> Result<PhysicsTrace3d, RapierValidationError> {
    let scenario = scenario.normalized()?;
    let gravity = Vec3::new(
        to_reference_f32(scenario.gravity_mps2[0], "gravity_mps2.x")?,
        to_reference_f32(scenario.gravity_mps2[1], "gravity_mps2.y")?,
        to_reference_f32(scenario.gravity_mps2[2], "gravity_mps2.z")?,
    );
    let dt = to_reference_f32(scenario.dt_s, "dt_s")?;

    let mut bodies = RigidBodySet::new();
    let mut colliders = ColliderSet::new();
    let parameters = IntegrationParameters::default();
    let mut islands = IslandManager::new();
    let mut broad_phase = BroadPhase::new();
    let mut narrow_phase = NarrowPhase::new();
    let mut impulse_joints = ImpulseJointSet::new();
    let mut multibody_joints = MultibodyJointSet::new();
    let mut ccd = CCDSolver::new();
    let mut bridge = RapierPhysicsBridge::with_gravity(NoOpCallback, gravity);
    let mut body_ids = BTreeMap::new();

    for spec in &scenario.bodies {
        let handle = insert_body(spec, &mut bodies, &mut colliders)?;
        body_ids.insert(handle.into_raw_parts(), spec.id);
    }

    let mut samples = vec![capture_direct_sample(&bodies, &body_ids, 0, 0.0)?];
    for step in 1..=scenario.steps {
        bridge.step(
            dt,
            &mut bodies,
            &mut colliders,
            &parameters,
            &mut islands,
            &mut broad_phase,
            &mut narrow_phase,
            &mut impulse_joints,
            &mut multibody_joints,
            &mut ccd,
            &(),
            &(),
        );

        if step == scenario.steps || step.is_multiple_of(scenario.sample_every_steps) {
            samples.push(capture_bridge_sample(
                &bridge,
                &body_ids,
                step,
                scenario.dt_s * step as f64,
            )?);
        }
    }

    Ok(PhysicsTrace3d {
        schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
        scenario_id: scenario.scenario_id,
        backend: PhysicsBackendDescriptor {
            engine: "rapier3d".to_owned(),
            engine_version: RAPIER_REFERENCE_VERSION.to_owned(),
            profile: "default-v0.1".to_owned(),
        },
        samples,
    })
}

fn insert_body(
    spec: &ScenarioBody3d,
    bodies: &mut RigidBodySet,
    colliders: &mut ColliderSet,
) -> Result<RigidBodyHandle, RapierValidationError> {
    let position = vector![
        to_reference_f32(spec.position_m[0], "position_m.x")?,
        to_reference_f32(spec.position_m[1], "position_m.y")?,
        to_reference_f32(spec.position_m[2], "position_m.z")?,
    ];
    let velocity = vector![
        to_reference_f32(spec.linear_velocity_mps[0], "linear_velocity_mps.x")?,
        to_reference_f32(spec.linear_velocity_mps[1], "linear_velocity_mps.y")?,
        to_reference_f32(spec.linear_velocity_mps[2], "linear_velocity_mps.z")?,
    ];

    let rigid_body = match spec.body_type {
        ScenarioBodyType::Static => RigidBodyBuilder::fixed(),
        ScenarioBodyType::Dynamic => RigidBodyBuilder::dynamic(),
    }
    .translation(position)
    .linvel(velocity)
    .build();
    let handle = bodies.insert(rigid_body);

    let mut collider = match spec.shape {
        ScenarioShape3d::Sphere { radius_m } => {
            ColliderBuilder::ball(to_reference_f32(radius_m, "sphere.radius_m")?)
        }
        ScenarioShape3d::Cuboid { half_extents_m } => ColliderBuilder::cuboid(
            to_reference_f32(half_extents_m[0], "cuboid.half_extents_m.x")?,
            to_reference_f32(half_extents_m[1], "cuboid.half_extents_m.y")?,
            to_reference_f32(half_extents_m[2], "cuboid.half_extents_m.z")?,
        ),
    }
    .friction(to_reference_f32(spec.friction, "friction")?)
    .restitution(to_reference_f32(spec.restitution, "restitution")?);

    if spec.body_type == ScenarioBodyType::Dynamic {
        // Use the declared total collider mass so Rapier derives its shape
        // inertia from the same mass instead of adding mass to default density.
        collider = collider.mass(to_reference_f32(spec.mass_kg, "mass_kg")?);
    }

    colliders.insert_with_parent(collider.build(), handle, bodies);
    Ok(handle)
}

fn capture_direct_sample(
    bodies: &RigidBodySet,
    body_ids: &BTreeMap<(u32, u32), ValidationBodyId>,
    step: u64,
    time_s: f64,
) -> Result<PhysicsSample3d, RapierValidationError> {
    let mut observations = Vec::with_capacity(bodies.len());
    for (handle, body) in bodies.iter() {
        let raw_handle = handle.into_raw_parts();
        let body_id = body_ids.get(&raw_handle).copied().ok_or(
            RapierValidationError::MissingBodyIdentity {
                index: raw_handle.0,
                generation: raw_handle.1,
            },
        )?;
        let position = body.translation();
        let velocity = body.linvel();
        observations.push(BodyObservation3d {
            body_id,
            position_m: [position.x as f64, position.y as f64, position.z as f64],
            linear_velocity_mps: [velocity.x as f64, velocity.y as f64, velocity.z as f64],
            sleeping: body.is_sleeping(),
        });
    }
    observations.sort_by_key(|observation| observation.body_id);

    Ok(PhysicsSample3d {
        step,
        time_s,
        bodies: observations,
    })
}

fn capture_bridge_sample<C: symtropy_physics::world::PhysicsCallback<3>>(
    bridge: &RapierPhysicsBridge<C>,
    body_ids: &BTreeMap<(u32, u32), ValidationBodyId>,
    step: u64,
    time_s: f64,
) -> Result<PhysicsSample3d, RapierValidationError> {
    let mut observations = Vec::with_capacity(bridge.last_observations().len());
    for observation in bridge.last_observations() {
        let body_id = body_ids.get(&observation.raw_handle).copied().ok_or(
            RapierValidationError::MissingBodyIdentity {
                index: observation.raw_handle.0,
                generation: observation.raw_handle.1,
            },
        )?;
        observations.push(BodyObservation3d {
            body_id,
            position_m: observation.translation.map(f64::from),
            linear_velocity_mps: observation.linear_velocity.map(f64::from),
            sleeping: observation.sleeping,
        });
    }
    observations.sort_by_key(|observation| observation.body_id);

    Ok(PhysicsSample3d {
        step,
        time_s,
        bodies: observations,
    })
}

fn to_reference_f32(value: f64, field: &'static str) -> Result<f32, RapierValidationError> {
    let narrowed = value as f32;
    if !narrowed.is_finite() {
        return Err(RapierValidationError::OutsideReferenceRange(field));
    }
    Ok(narrowed)
}

fn free_fall_scenario() -> PhysicsScenario3d {
    PhysicsScenario3d {
        schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
        scenario_id: "shared-free-fall-v0".to_owned(),
        gravity_mps2: [0.0, -9.81, 0.0],
        dt_s: 0.01,
        steps: 10,
        sample_every_steps: 5,
        bodies: vec![ScenarioBody3d {
            id: ValidationBodyId(42),
            body_type: ScenarioBodyType::Dynamic,
            shape: ScenarioShape3d::Sphere { radius_m: 0.5 },
            position_m: [0.0, 10.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 1.0,
            friction: 0.5,
            restitution: 0.0,
        }],
    }
}

#[test]
fn rapier_adapter_executes_common_free_fall_scenario() {
    let trace = run_rapier_scenario_3d(&free_fall_scenario()).unwrap();
    assert_eq!(trace.backend.engine, "rapier3d");
    assert_eq!(trace.backend.engine_version, RAPIER_REFERENCE_VERSION);
    assert_eq!(trace.samples.len(), 3);
    assert_eq!(trace.samples[0].bodies[0].body_id, ValidationBodyId(42));
    assert!(trace.final_sample().unwrap().bodies[0].position_m[1] < 10.0);
    assert!(trace.final_sample().unwrap().bodies[0].linear_velocity_mps[1] < 0.0);
}

#[test]
fn native_and_rapier_share_identity_schedule_and_finite_metrics() {
    let scenario = free_fall_scenario();
    let native = run_native_scenario_3d(&scenario).unwrap();
    let rapier = run_rapier_scenario_3d(&scenario).unwrap();
    let metrics = compare_traces_3d(&native, &rapier).unwrap();

    assert_eq!(metrics.compared_samples, 3);
    assert_eq!(metrics.compared_bodies, 3);
    assert_eq!(metrics.left_non_finite_observations, 0);
    assert_eq!(metrics.right_non_finite_observations, 0);
    assert!(metrics.max_position_error_m.is_finite());
    assert!(metrics.max_linear_velocity_error_mps.is_finite());

    // Broad sanity envelope only. This is deliberately not promoted as a
    // physical-accuracy threshold or solver qualification result.
    assert!(metrics.max_position_error_m < 0.01);
    assert!(metrics.max_linear_velocity_error_mps < 0.01);
}

#[test]
fn adapter_preserves_static_and_dynamic_validation_ids() {
    let mut scenario = free_fall_scenario();
    scenario.bodies.push(ScenarioBody3d {
        id: ValidationBodyId(3),
        body_type: ScenarioBodyType::Static,
        shape: ScenarioShape3d::Cuboid {
            half_extents_m: [1.0, 0.1, 1.0],
        },
        position_m: [0.0, -2.0, 0.0],
        linear_velocity_mps: [0.0, 0.0, 0.0],
        mass_kg: 0.0,
        friction: 0.5,
        restitution: 0.0,
    });

    let trace = run_rapier_scenario_3d(&scenario).unwrap();
    let ids: Vec<_> = trace.samples[0]
        .bodies
        .iter()
        .map(|body| body.body_id)
        .collect();
    assert_eq!(ids, vec![ValidationBodyId(3), ValidationBodyId(42)]);
}

#[test]
fn adapter_rejects_values_that_overflow_reference_precision() {
    let mut scenario = free_fall_scenario();
    scenario.bodies[0].position_m[0] = f64::MAX;
    assert_eq!(
        run_rapier_scenario_3d(&scenario),
        Err(RapierValidationError::OutsideReferenceRange("position_m.x"))
    );
}
