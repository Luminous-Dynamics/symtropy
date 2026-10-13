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
    ScenarioBodyType, ScenarioShape3d, ScenarioValidationError, ValidationBodyId,
    compare_traces_3d, run_native_scenario_3d,
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

/// Build a free-fall scenario whose total simulated duration is exactly one
/// second. The semi-implicit integrator's position error should decrease as
/// the fixed timestep is refined; the analytical trajectory is independent of
/// either engine implementation.
fn free_fall_at_dt(dt_s: f64) -> PhysicsScenario3d {
    let mut scenario = free_fall_scenario();
    scenario.scenario_id = format!("analytical-free-fall-dt-{dt_s:.6}");
    scenario.dt_s = dt_s;
    scenario.steps = (1.0 / dt_s).round() as u64;
    scenario.sample_every_steps = scenario.steps;
    scenario
}

fn analytical_free_fall_position_error(
    scenario: &PhysicsScenario3d,
    trace: &PhysicsTrace3d,
) -> f64 {
    let body = &scenario.bodies[0];
    let time_s = scenario.dt_s * scenario.steps as f64;
    let initial_y = body.position_m[1];
    let initial_vy = body.linear_velocity_mps[1];
    let gravity_y = scenario.gravity_mps2[1];
    let expected_y = initial_y + initial_vy * time_s + 0.5 * gravity_y * time_s * time_s;
    (trace.final_sample().unwrap().bodies[0].position_m[1] - expected_y).abs()
}

fn assert_free_fall_converges(
    coarse_scenario: &PhysicsScenario3d,
    coarse_trace: &PhysicsTrace3d,
    fine_scenario: &PhysicsScenario3d,
    fine_trace: &PhysicsTrace3d,
    backend: &str,
) {
    let coarse_error = analytical_free_fall_position_error(coarse_scenario, coarse_trace);
    let fine_error = analytical_free_fall_position_error(fine_scenario, fine_trace);
    assert!(coarse_error.is_finite() && fine_error.is_finite());
    assert!(
        coarse_error > 1.0e-3,
        "{backend} coarse error unexpectedly vanished: {coarse_error}"
    );
    assert!(
        fine_error < coarse_error * 0.65,
        "{backend} did not converge under dt refinement: coarse={coarse_error}, fine={fine_error}"
    );
}

/// Contact scenarios use a finite axis-aligned cuboid floor, within the shared
/// v0.1 shape domain. The floor's top surface is y=0.
fn contact_scenario(
    scenario_id: &str,
    sphere_y: f64,
    sphere_vx: f64,
    friction: f64,
    restitution: f64,
    steps: u64,
    dt_s: f64,
    sample_every_steps: u64,
) -> PhysicsScenario3d {
    let material = |id, body_type, shape, position_m, linear_velocity_mps, mass_kg| {
        ScenarioBody3d {
            id: ValidationBodyId(id),
            body_type,
            shape,
            position_m,
            linear_velocity_mps,
            mass_kg,
            friction,
            restitution,
        }
    };
    PhysicsScenario3d {
        schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
        scenario_id: scenario_id.to_owned(),
        gravity_mps2: [0.0, -9.81, 0.0],
        dt_s,
        steps,
        sample_every_steps,
        bodies: vec![
            material(
                1,
                ScenarioBodyType::Static,
                ScenarioShape3d::Cuboid {
                    half_extents_m: [5.0, 0.5, 5.0],
                },
                [0.0, -0.5, 0.0],
                [0.0, 0.0, 0.0],
                0.0,
            ),
            material(
                2,
                ScenarioBodyType::Dynamic,
                ScenarioShape3d::Sphere { radius_m: 0.5 },
                [0.0, sphere_y, 0.0],
                [sphere_vx, 0.0, 0.0],
                1.0,
            ),
        ],
    }
}

fn final_body(trace: &PhysicsTrace3d, id: ValidationBodyId) -> &BodyObservation3d {
    trace
        .final_sample()
        .expect("trace has a final sample")
        .bodies
        .iter()
        .find(|body| body.body_id == id)
        .expect("trace contains the requested validation body")
}

fn post_contact_peak_y(
    trace: &PhysicsTrace3d,
    id: ValidationBodyId,
    after_time_s: f64,
) -> f64 {
    trace
        .samples
        .iter()
        .filter(|sample| sample.time_s >= after_time_s)
        .filter_map(|sample| {
            sample
                .bodies
                .iter()
                .find(|body| body.body_id == id)
                .map(|body| body.position_m[1])
        })
        .fold(f64::NEG_INFINITY, f64::max)
}

fn assert_restitution_is_observable(
    inelastic: &PhysicsTrace3d,
    bouncy: &PhysicsTrace3d,
    backend: &str,
) {
    let inelastic_peak = post_contact_peak_y(inelastic, ValidationBodyId(2), 0.60);
    let bouncy_peak = post_contact_peak_y(bouncy, ValidationBodyId(2), 0.60);
    assert!(inelastic_peak.is_finite() && bouncy_peak.is_finite());
    assert!(
        bouncy_peak > inelastic_peak + 0.20,
        "{backend} restitution response too small: low={inelastic_peak}, high={bouncy_peak}"
    );
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

#[test]
fn analytical_free_fall_position_error_converges_for_both_engines() {
    let coarse_scenario = free_fall_at_dt(0.02);
    let fine_scenario = free_fall_at_dt(0.01);

    let coarse_native = run_native_scenario_3d(&coarse_scenario).unwrap();
    let fine_native = run_native_scenario_3d(&fine_scenario).unwrap();
    assert_free_fall_converges(
        &coarse_scenario,
        &coarse_native,
        &fine_scenario,
        &fine_native,
        "native",
    );

    let coarse_rapier = run_rapier_scenario_3d(&coarse_scenario).unwrap();
    let fine_rapier = run_rapier_scenario_3d(&fine_scenario).unwrap();
    assert_free_fall_converges(
        &coarse_scenario,
        &coarse_rapier,
        &fine_scenario,
        &fine_rapier,
        "Rapier",
    );
}

#[test]
fn native_and_rapier_keep_a_sphere_near_resting_contact() {
    let scenario = contact_scenario(
        "resting-sphere-v0",
        0.55,
        0.0,
        0.8,
        0.0,
        800,
        0.0025,
        40,
    );
    for (backend, trace) in [
        ("native", run_native_scenario_3d(&scenario).unwrap()),
        ("Rapier", run_rapier_scenario_3d(&scenario).unwrap()),
    ] {
        let sphere = final_body(&trace, ValidationBodyId(2));
        assert!(
            (0.38..=0.72).contains(&sphere.position_m[1]),
            "{backend} resting sphere drifted from the support surface: y={}",
            sphere.position_m[1]
        );
        assert!(
            sphere.linear_velocity_mps[1].abs() < 0.75,
            "{backend} resting sphere retained excessive vertical velocity: vy={}",
            sphere.linear_velocity_mps[1]
        );
    }
}

#[test]
fn native_and_rapier_restitution_changes_post_impact_trajectory() {
    let inelastic_scenario = contact_scenario(
        "inelastic-drop-v0",
        2.0,
        0.0,
        0.5,
        0.0,
        560,
        0.0025,
        1,
    );
    let bouncy_scenario = contact_scenario(
        "restitution-drop-v0",
        2.0,
        0.0,
        0.5,
        0.8,
        560,
        0.0025,
        1,
    );
    let native_inelastic = run_native_scenario_3d(&inelastic_scenario).unwrap();
    let native_bouncy = run_native_scenario_3d(&bouncy_scenario).unwrap();
    assert_restitution_is_observable(&native_inelastic, &native_bouncy, "native");

    let rapier_inelastic = run_rapier_scenario_3d(&inelastic_scenario).unwrap();
    let rapier_bouncy = run_rapier_scenario_3d(&bouncy_scenario).unwrap();
    assert_restitution_is_observable(&rapier_inelastic, &rapier_bouncy, "Rapier");
}

#[test]
fn native_and_rapier_friction_reduces_sliding_sphere_speed() {
    let low_friction = contact_scenario(
        "low-friction-slide-v0",
        0.51,
        2.0,
        0.0,
        0.0,
        600,
        0.0025,
        20,
    );
    let high_friction = contact_scenario(
        "high-friction-slide-v0",
        0.51,
        2.0,
        1.0,
        0.0,
        600,
        0.0025,
        20,
    );
    for (backend, low_trace, high_trace) in [
        (
            "native",
            run_native_scenario_3d(&low_friction).unwrap(),
            run_native_scenario_3d(&high_friction).unwrap(),
        ),
        (
            "Rapier",
            run_rapier_scenario_3d(&low_friction).unwrap(),
            run_rapier_scenario_3d(&high_friction).unwrap(),
        ),
    ] {
        let low_speed = final_body(&low_trace, ValidationBodyId(2)).linear_velocity_mps[0].abs();
        let high_speed =
            final_body(&high_trace, ValidationBodyId(2)).linear_velocity_mps[0].abs();
        assert!(low_speed > 0.5, "{backend} zero-friction lane lost too much speed: {low_speed}");
        assert!(
            high_speed + 0.15 < low_speed,
            "{backend} high friction did not reduce slide speed: low={low_speed}, high={high_speed}"
        );
    }
}


fn box_stack_scenario(box_count: usize) -> PhysicsScenario3d {
    assert!(box_count >= 3 && box_count <= 5);
    let mut bodies = vec![ScenarioBody3d {
        id: ValidationBodyId(1),
        body_type: ScenarioBodyType::Static,
        shape: ScenarioShape3d::Cuboid {
            half_extents_m: [4.0, 0.5, 4.0],
        },
        position_m: [0.0, -0.5, 0.0],
        linear_velocity_mps: [0.0, 0.0, 0.0],
        mass_kg: 0.0,
        friction: 0.8,
        restitution: 0.0,
    }];

    for index in 0..box_count {
        bodies.push(ScenarioBody3d {
            id: ValidationBodyId(100 + index as u64),
            body_type: ScenarioBodyType::Dynamic,
            shape: ScenarioShape3d::Cuboid {
                half_extents_m: [0.5, 0.5, 0.5],
            },
            // Deliberately begin with a small, known overlap so this exercises
            // contact resolution immediately rather than only free fall.
            position_m: [0.0, 0.48 + index as f64 * 0.98, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 1.0,
            friction: 0.8,
            restitution: 0.0,
        });
    }

    PhysicsScenario3d {
        schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
        scenario_id: format!("axis-aligned-box-stack-{box_count}-v0"),
        gravity_mps2: [0.0, -9.81, 0.0],
        dt_s: 1.0 / 120.0,
        steps: 600,
        sample_every_steps: 1,
        bodies,
    }
}

fn assert_stack_trace_is_bounded(trace: &PhysicsTrace3d, box_count: usize, backend: &str) {
    assert_eq!(
        trace.samples.len(),
        601,
        "{backend} must preserve every requested per-step observation"
    );
    let expected_body_count = box_count + 1;
    let mut peak_speed = 0.0_f64;

    for sample in &trace.samples {
        assert_eq!(
            sample.bodies.len(),
            expected_body_count,
            "{backend} changed body cardinality at step {}",
            sample.step
        );
        for body in &sample.bodies {
            assert!(
                body.is_finite(),
                "{backend} produced a non-finite state at step {} for body {}",
                sample.step,
                body.body_id.0
            );
            if body.body_id.0 >= 100 {
                let speed = body
                    .linear_velocity_mps
                    .iter()
                    .map(|component| component * component)
                    .sum::<f64>()
                    .sqrt();
                peak_speed = peak_speed.max(speed);
            }
        }
    }

    assert!(
        peak_speed < 5.0,
        "{backend} stack produced an explosive impulse: peak speed={peak_speed} m/s"
    );

    let final_sample = trace.final_sample().expect("stack trace has samples");
    for index in 0..box_count {
        let body_id = ValidationBodyId(100 + index as u64);
        let body = final_sample
            .bodies
            .iter()
            .find(|body| body.body_id == body_id)
            .expect("every stacked box retains its stable identity");
        let expected_y = 0.5 + index as f64;
        assert!(
            (body.position_m[1] - expected_y).abs() < 0.45,
            "{backend} box {index} did not remain near its stack level: y={}, expected={expected_y}",
            body.position_m[1]
        );
    }
}

#[test]
fn three_and_five_box_stacks_remain_finite_and_bounded_for_both_backends() {
    for box_count in [3_usize, 5_usize] {
        let scenario = box_stack_scenario(box_count);

        let native = run_native_scenario_3d(&scenario).unwrap();
        assert_stack_trace_is_bounded(&native, box_count, "native");

        let rapier = run_rapier_scenario_3d(&scenario).unwrap();
        assert_stack_trace_is_bounded(&rapier, box_count, "Rapier");
    }
}


#[test]
fn stack_trace_is_invariant_to_nonsemantic_body_insertion_order() {
    // The scenario contract assigns stable IDs, and both runners normalize by
    // those IDs before constructing the world. Reversing input order must not
    // silently change solver history under this declared deterministic policy.
    let scenario = box_stack_scenario(3);
    let mut permuted = scenario.clone();
    permuted.bodies.reverse();

    assert_eq!(scenario.scenario_id, permuted.scenario_id);
    assert_ne!(scenario.bodies, permuted.bodies);

    let native_reference = run_native_scenario_3d(&scenario).unwrap();
    let native_permuted = run_native_scenario_3d(&permuted).unwrap();
    assert_eq!(
        native_reference, native_permuted,
        "native stack trace changed under a nonsemantic input permutation"
    );

    let rapier_reference = run_rapier_scenario_3d(&scenario).unwrap();
    let rapier_permuted = run_rapier_scenario_3d(&permuted).unwrap();
    assert_eq!(
        rapier_reference, rapier_permuted,
        "Rapier stack trace changed under a nonsemantic input permutation"
    );
}
