// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Engine-neutral rigid-body validation vocabulary and native reference adapter.
//!
//! This module deliberately sits above the hot solver path: it defines small,
//! serializable scenario/trace types and pure comparison logic used by research
//! harnesses. External engines may adapt to the same types, but agreement with
//! any external engine is evidence, not canonical Symtropy physical truth.
//!
//! V0.1 intentionally covers a narrow shared validity domain: 3D, axis-aligned
//! sphere/cuboid bodies, static/dynamic body types, explicit SI gravity and
//! linear velocity, and position/linear-velocity observations. Initial
//! orientation, angular-velocity equivalence, joints, per-body CCD and applied
//! forces remain outside this schema until their cross-engine conventions are
//! explicitly reconciled.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use nalgebra::SVector;
use serde::{Deserialize, Serialize};
use symtropy_math::{HyperBox, Point, Shape, Sphere, Transform};

use crate::body::{BodyHandle, BodyType, NetId, RigidBody};
use crate::world::PhysicsWorld;

/// Current schema version for [`PhysicsScenario3d`].
pub const PHYSICS_SCENARIO_SCHEMA_VERSION: u32 = 1;
/// Current schema version for [`PhysicsTrace3d`].
pub const PHYSICS_TRACE_SCHEMA_VERSION: u32 = 1;

/// Stable, engine-neutral body identity within one validation scenario.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ValidationBodyId(pub u64);

/// Body authority represented by the v0.1 validation schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioBodyType {
    Static,
    Dynamic,
}

/// Collider primitives shared by the initial native/Rapier corpus.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioShape3d {
    Sphere { radius_m: f64 },
    Cuboid { half_extents_m: [f64; 3] },
}

/// One rigid body in a canonical differential-validation scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScenarioBody3d {
    pub id: ValidationBodyId,
    pub body_type: ScenarioBodyType,
    pub shape: ScenarioShape3d,
    /// World-space center/translation in metres. V0.1 orientation is identity.
    pub position_m: [f64; 3],
    /// World-space linear velocity in metres/second.
    pub linear_velocity_mps: [f64; 3],
    /// Dynamic body mass in kilograms. Static bodies must declare `0.0`.
    pub mass_kg: f64,
    pub friction: f64,
    pub restitution: f64,
}

/// Canonical 3D rigid-body fixture shared by independent solver backends.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsScenario3d {
    pub schema_version: u32,
    pub scenario_id: String,
    /// Uniform acceleration in metres/second².
    pub gravity_mps2: [f64; 3],
    pub dt_s: f64,
    pub steps: u64,
    /// Record step zero, every Nth step, and always the final step.
    pub sample_every_steps: u64,
    pub bodies: Vec<ScenarioBody3d>,
}

impl PhysicsScenario3d {
    /// Validate all v0.1 invariants without changing caller ordering.
    pub fn validate(&self) -> Result<(), ScenarioValidationError> {
        if self.schema_version != PHYSICS_SCENARIO_SCHEMA_VERSION {
            return Err(ScenarioValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        if self.scenario_id.trim().is_empty() {
            return Err(ScenarioValidationError::EmptyScenarioId);
        }
        if !self.dt_s.is_finite() || self.dt_s <= 0.0 {
            return Err(ScenarioValidationError::InvalidTimeStep);
        }
        if self.steps == 0 {
            return Err(ScenarioValidationError::ZeroSteps);
        }
        if self.sample_every_steps == 0 {
            return Err(ScenarioValidationError::ZeroSampleStride);
        }
        if !all_finite(&self.gravity_mps2) {
            return Err(ScenarioValidationError::NonFiniteGravity);
        }
        if self.bodies.is_empty() {
            return Err(ScenarioValidationError::NoBodies);
        }

        let mut ids = BTreeSet::new();
        for body in &self.bodies {
            if !ids.insert(body.id) {
                return Err(ScenarioValidationError::DuplicateBodyId(body.id));
            }
            if !all_finite(&body.position_m) || !all_finite(&body.linear_velocity_mps) {
                return Err(ScenarioValidationError::NonFiniteBodyState(body.id));
            }
            if !body.friction.is_finite() || body.friction < 0.0 {
                return Err(ScenarioValidationError::InvalidFriction(body.id));
            }
            if !body.restitution.is_finite() || !(0.0..=1.0).contains(&body.restitution) {
                return Err(ScenarioValidationError::InvalidRestitution(body.id));
            }

            match body.body_type {
                ScenarioBodyType::Static => {
                    if !body.mass_kg.is_finite() || body.mass_kg != 0.0 {
                        return Err(ScenarioValidationError::InvalidStaticMass(body.id));
                    }
                    if body.linear_velocity_mps.iter().any(|value| *value != 0.0) {
                        return Err(ScenarioValidationError::StaticBodyHasVelocity(body.id));
                    }
                }
                ScenarioBodyType::Dynamic => {
                    if !body.mass_kg.is_finite() || body.mass_kg <= 0.0 {
                        return Err(ScenarioValidationError::InvalidDynamicMass(body.id));
                    }
                }
            }

            match body.shape {
                ScenarioShape3d::Sphere { radius_m } => {
                    if !radius_m.is_finite() || radius_m <= 0.0 {
                        return Err(ScenarioValidationError::InvalidShape(body.id));
                    }
                }
                ScenarioShape3d::Cuboid { half_extents_m } => {
                    if !all_finite(&half_extents_m)
                        || half_extents_m.iter().any(|extent| *extent <= 0.0)
                    {
                        return Err(ScenarioValidationError::InvalidShape(body.id));
                    }
                }
            }
        }

        Ok(())
    }

    /// Return a validated copy in deterministic body-ID order.
    pub fn normalized(&self) -> Result<Self, ScenarioValidationError> {
        self.validate()?;
        let mut normalized = self.clone();
        normalized.bodies.sort_by_key(|body| body.id);
        Ok(normalized)
    }
}

/// Invalid or ambiguous canonical scenario input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScenarioValidationError {
    UnsupportedSchemaVersion(u32),
    EmptyScenarioId,
    InvalidTimeStep,
    ZeroSteps,
    ZeroSampleStride,
    NonFiniteGravity,
    NoBodies,
    DuplicateBodyId(ValidationBodyId),
    NonFiniteBodyState(ValidationBodyId),
    InvalidStaticMass(ValidationBodyId),
    StaticBodyHasVelocity(ValidationBodyId),
    InvalidDynamicMass(ValidationBodyId),
    InvalidFriction(ValidationBodyId),
    InvalidRestitution(ValidationBodyId),
    InvalidShape(ValidationBodyId),
}

impl fmt::Display for ScenarioValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion(version) => {
                write!(f, "unsupported physics scenario schema version {version}")
            }
            Self::EmptyScenarioId => write!(f, "scenario_id must not be empty"),
            Self::InvalidTimeStep => write!(f, "dt_s must be finite and greater than zero"),
            Self::ZeroSteps => write!(f, "steps must be greater than zero"),
            Self::ZeroSampleStride => write!(f, "sample_every_steps must be greater than zero"),
            Self::NonFiniteGravity => write!(f, "gravity_mps2 must be finite"),
            Self::NoBodies => write!(f, "scenario must contain at least one body"),
            Self::DuplicateBodyId(id) => write!(f, "duplicate validation body id {}", id.0),
            Self::NonFiniteBodyState(id) => {
                write!(f, "body {} has non-finite position or velocity", id.0)
            }
            Self::InvalidStaticMass(id) => {
                write!(f, "static body {} must declare mass_kg = 0", id.0)
            }
            Self::StaticBodyHasVelocity(id) => {
                write!(f, "static body {} must start with zero linear velocity", id.0)
            }
            Self::InvalidDynamicMass(id) => {
                write!(f, "dynamic body {} must have finite positive mass", id.0)
            }
            Self::InvalidFriction(id) => {
                write!(f, "body {} must have finite non-negative friction", id.0)
            }
            Self::InvalidRestitution(id) => {
                write!(f, "body {} restitution must be finite and in [0, 1]", id.0)
            }
            Self::InvalidShape(id) => {
                write!(f, "body {} has invalid or non-finite shape dimensions", id.0)
            }
        }
    }
}

impl std::error::Error for ScenarioValidationError {}

/// Declares which independent implementation produced a trace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhysicsBackendDescriptor {
    pub engine: String,
    pub engine_version: String,
    pub profile: String,
}

/// Engine-neutral post-step state for one validation body.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BodyObservation3d {
    pub body_id: ValidationBodyId,
    pub position_m: [f64; 3],
    pub linear_velocity_mps: [f64; 3],
    pub sleeping: bool,
}

impl BodyObservation3d {
    pub fn is_finite(&self) -> bool {
        all_finite(&self.position_m) && all_finite(&self.linear_velocity_mps)
    }
}

/// One deterministic observation point in a run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsSample3d {
    pub step: u64,
    pub time_s: f64,
    /// Must be sorted by [`ValidationBodyId`] in produced traces.
    pub bodies: Vec<BodyObservation3d>,
}

/// Complete sampled trace for one backend executing one scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PhysicsTrace3d {
    pub schema_version: u32,
    pub scenario_id: String,
    pub backend: PhysicsBackendDescriptor,
    pub samples: Vec<PhysicsSample3d>,
}

impl PhysicsTrace3d {
    pub fn final_sample(&self) -> Option<&PhysicsSample3d> {
        self.samples.last()
    }
}

/// Aggregate differential metrics between two traces of the same scenario.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DifferentialMetrics3d {
    pub scenario_id: String,
    pub left_backend: PhysicsBackendDescriptor,
    pub right_backend: PhysicsBackendDescriptor,
    pub compared_samples: u64,
    pub compared_bodies: u64,
    pub max_position_error_m: f64,
    pub rms_position_error_m: f64,
    pub max_linear_velocity_error_mps: f64,
    pub rms_linear_velocity_error_mps: f64,
    pub sleeping_mismatches: u64,
    pub left_non_finite_observations: u64,
    pub right_non_finite_observations: u64,
}

/// Structural mismatch that makes two traces unsafe to compare numerically.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceComparisonError {
    ScenarioMismatch,
    SampleCountMismatch,
    SampleStepMismatch { left: u64, right: u64 },
    BodySetMismatch { step: u64 },
}

impl fmt::Display for TraceComparisonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ScenarioMismatch => write!(f, "trace scenario_id values differ"),
            Self::SampleCountMismatch => write!(f, "trace sample counts differ"),
            Self::SampleStepMismatch { left, right } => {
                write!(f, "trace sample steps differ: left={left}, right={right}")
            }
            Self::BodySetMismatch { step } => {
                write!(f, "trace body identity sets differ at step {step}")
            }
        }
    }
}

impl std::error::Error for TraceComparisonError {}

/// Compare two traces strictly by scenario, sample step, and stable body ID.
///
/// Non-finite observations are counted and excluded from numeric maxima/RMS so
/// the report itself stays finite; their presence is an explicit hard signal
/// for qualification policy rather than being hidden inside NaN aggregates.
pub fn compare_traces_3d(
    left: &PhysicsTrace3d,
    right: &PhysicsTrace3d,
) -> Result<DifferentialMetrics3d, TraceComparisonError> {
    if left.scenario_id != right.scenario_id {
        return Err(TraceComparisonError::ScenarioMismatch);
    }
    if left.samples.len() != right.samples.len() {
        return Err(TraceComparisonError::SampleCountMismatch);
    }

    let mut metrics = DifferentialMetrics3d {
        scenario_id: left.scenario_id.clone(),
        left_backend: left.backend.clone(),
        right_backend: right.backend.clone(),
        compared_samples: 0,
        compared_bodies: 0,
        max_position_error_m: 0.0,
        rms_position_error_m: 0.0,
        max_linear_velocity_error_mps: 0.0,
        rms_linear_velocity_error_mps: 0.0,
        sleeping_mismatches: 0,
        left_non_finite_observations: 0,
        right_non_finite_observations: 0,
    };
    let mut position_error_sq_sum = 0.0;
    let mut velocity_error_sq_sum = 0.0;
    let mut finite_pairs = 0_u64;

    for (left_sample, right_sample) in left.samples.iter().zip(&right.samples) {
        if left_sample.step != right_sample.step {
            return Err(TraceComparisonError::SampleStepMismatch {
                left: left_sample.step,
                right: right_sample.step,
            });
        }

        let left_bodies: BTreeMap<_, _> = left_sample
            .bodies
            .iter()
            .map(|body| (body.body_id, body))
            .collect();
        let right_bodies: BTreeMap<_, _> = right_sample
            .bodies
            .iter()
            .map(|body| (body.body_id, body))
            .collect();

        if left_bodies.keys().copied().collect::<Vec<_>>()
            != right_bodies.keys().copied().collect::<Vec<_>>()
        {
            return Err(TraceComparisonError::BodySetMismatch {
                step: left_sample.step,
            });
        }

        metrics.compared_samples += 1;
        for (body_id, left_body) in left_bodies {
            let right_body = right_bodies[&body_id];
            metrics.compared_bodies += 1;

            if left_body.sleeping != right_body.sleeping {
                metrics.sleeping_mismatches += 1;
            }

            let left_finite = left_body.is_finite();
            let right_finite = right_body.is_finite();
            if !left_finite {
                metrics.left_non_finite_observations += 1;
            }
            if !right_finite {
                metrics.right_non_finite_observations += 1;
            }
            if !left_finite || !right_finite {
                continue;
            }

            let position_error = euclidean_error(&left_body.position_m, &right_body.position_m);
            let velocity_error = euclidean_error(
                &left_body.linear_velocity_mps,
                &right_body.linear_velocity_mps,
            );
            metrics.max_position_error_m = metrics.max_position_error_m.max(position_error);
            metrics.max_linear_velocity_error_mps =
                metrics.max_linear_velocity_error_mps.max(velocity_error);
            position_error_sq_sum += position_error * position_error;
            velocity_error_sq_sum += velocity_error * velocity_error;
            finite_pairs += 1;
        }
    }

    if finite_pairs > 0 {
        let denominator = finite_pairs as f64;
        metrics.rms_position_error_m = (position_error_sq_sum / denominator).sqrt();
        metrics.rms_linear_velocity_error_mps =
            (velocity_error_sq_sum / denominator).sqrt();
    }

    Ok(metrics)
}

/// Execute a validated v0.1 scenario with the native Symtropy 3D solver.
pub fn run_native_scenario_3d(
    scenario: &PhysicsScenario3d,
) -> Result<PhysicsTrace3d, ScenarioValidationError> {
    let scenario = scenario.normalized()?;
    let mut world = PhysicsWorld::<3>::new(SVector::from(scenario.gravity_mps2));

    for spec in &scenario.bodies {
        let body = build_native_body(spec);
        let handle = world.add_body(body);
        world.set_net_id(handle, NetId(spec.id.0));
    }

    let mut samples = vec![capture_native_sample(&world, 0, 0.0)];
    for step in 1..=scenario.steps {
        world.step(scenario.dt_s);
        if should_sample(step, scenario.steps, scenario.sample_every_steps) {
            samples.push(capture_native_sample(
                &world,
                step,
                scenario.dt_s * step as f64,
            ));
        }
    }

    Ok(PhysicsTrace3d {
        schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
        scenario_id: scenario.scenario_id,
        backend: PhysicsBackendDescriptor {
            engine: "symtropy-physics".to_owned(),
            engine_version: env!("CARGO_PKG_VERSION").to_owned(),
            profile: "native-default-v0.1".to_owned(),
        },
        samples,
    })
}

fn build_native_body(spec: &ScenarioBody3d) -> RigidBody<3> {
    let position = Point::<3>::new(spec.position_m);
    let collider: Box<dyn Shape<3>> = match spec.shape {
        ScenarioShape3d::Sphere { radius_m } => {
            Box::new(Sphere::new(Point::origin(), radius_m))
        }
        ScenarioShape3d::Cuboid { half_extents_m } => Box::new(HyperBox::new(half_extents_m)),
    };

    let mut body = match spec.body_type {
        ScenarioBodyType::Static => RigidBody::static_body(BodyHandle(0), position, collider),
        ScenarioBodyType::Dynamic => {
            let inertia = native_principal_inertia(spec);
            RigidBody::new(
                BodyHandle(0),
                BodyType::Dynamic,
                Transform::from_translation(position),
                collider,
                spec.mass_kg,
                inertia,
            )
        }
    };

    body.linear_velocity = SVector::from(spec.linear_velocity_mps);
    body.friction = spec.friction;
    body.restitution = spec.restitution;
    body
}

fn native_principal_inertia(spec: &ScenarioBody3d) -> SVector<f64, 3> {
    match spec.shape {
        ScenarioShape3d::Sphere { radius_m } => {
            SVector::from_element(0.4 * spec.mass_kg * radius_m * radius_m)
        }
        ScenarioShape3d::Cuboid { half_extents_m } => {
            let [hx, hy, hz] = half_extents_m;
            // I = m/12 * (full_side_a² + full_side_b²)
            //   = m/3  * (half_extent_a² + half_extent_b²).
            SVector::from([
                spec.mass_kg / 3.0 * (hy * hy + hz * hz),
                spec.mass_kg / 3.0 * (hx * hx + hz * hz),
                spec.mass_kg / 3.0 * (hx * hx + hy * hy),
            ])
        }
    }
}

fn capture_native_sample(world: &PhysicsWorld<3>, step: u64, time_s: f64) -> PhysicsSample3d {
    let mut bodies: Vec<_> = world
        .bodies
        .iter()
        .map(|body| {
            let net_id = body
                .net_id
                .expect("validation runner assigns every body a stable NetId");
            BodyObservation3d {
                body_id: ValidationBodyId(net_id.0),
                position_m: body.position().into(),
                linear_velocity_mps: body.linear_velocity.into(),
                sleeping: body.sleeping,
            }
        })
        .collect();
    bodies.sort_by_key(|body| body.body_id);

    PhysicsSample3d {
        step,
        time_s,
        bodies,
    }
}

fn should_sample(step: u64, final_step: u64, stride: u64) -> bool {
    step == final_step || step.is_multiple_of(stride)
}

fn all_finite<const N: usize>(values: &[f64; N]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn euclidean_error<const N: usize>(left: &[f64; N], right: &[f64; N]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| {
            let delta = left - right;
            delta * delta
        })
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dynamic_sphere(id: u64, y: f64) -> ScenarioBody3d {
        ScenarioBody3d {
            id: ValidationBodyId(id),
            body_type: ScenarioBodyType::Dynamic,
            shape: ScenarioShape3d::Sphere { radius_m: 0.5 },
            position_m: [0.0, y, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 1.0,
            friction: 0.5,
            restitution: 0.0,
        }
    }

    fn free_fall_scenario() -> PhysicsScenario3d {
        PhysicsScenario3d {
            schema_version: PHYSICS_SCENARIO_SCHEMA_VERSION,
            scenario_id: "free-fall-v0".to_owned(),
            gravity_mps2: [0.0, -9.81, 0.0],
            dt_s: 0.01,
            steps: 10,
            sample_every_steps: 5,
            bodies: vec![dynamic_sphere(7, 10.0)],
        }
    }

    #[test]
    fn validation_rejects_duplicate_body_ids() {
        let mut scenario = free_fall_scenario();
        scenario.bodies.push(dynamic_sphere(7, 12.0));
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::DuplicateBodyId(ValidationBodyId(7)))
        );
    }

    #[test]
    fn validation_rejects_non_finite_and_ambiguous_inputs() {
        let mut scenario = free_fall_scenario();
        scenario.gravity_mps2[0] = f64::NAN;
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::NonFiniteGravity)
        );

        let mut scenario = free_fall_scenario();
        scenario.bodies[0].restitution = 1.01;
        assert_eq!(
            scenario.validate(),
            Err(ScenarioValidationError::InvalidRestitution(
                ValidationBodyId(7)
            ))
        );
    }

    #[test]
    fn normalization_orders_bodies_by_stable_identity() {
        let mut scenario = free_fall_scenario();
        scenario.bodies = vec![dynamic_sphere(9, 9.0), dynamic_sphere(2, 2.0)];
        let normalized = scenario.normalized().unwrap();
        assert_eq!(normalized.bodies[0].id, ValidationBodyId(2));
        assert_eq!(normalized.bodies[1].id, ValidationBodyId(9));
    }

    #[test]
    fn native_runner_advances_free_fall_and_always_samples_final_step() {
        let trace = run_native_scenario_3d(&free_fall_scenario()).unwrap();
        assert_eq!(trace.samples.len(), 3);
        assert_eq!(trace.samples[0].step, 0);
        assert_eq!(trace.samples[1].step, 5);
        assert_eq!(trace.samples[2].step, 10);
        assert!(trace.samples[2].bodies[0].position_m[1] < 10.0);
        assert!(trace.samples[2].bodies[0].linear_velocity_mps[1] < 0.0);
    }

    #[test]
    fn native_static_body_does_not_move_under_gravity() {
        let mut scenario = free_fall_scenario();
        scenario.bodies = vec![ScenarioBody3d {
            id: ValidationBodyId(1),
            body_type: ScenarioBodyType::Static,
            shape: ScenarioShape3d::Cuboid {
                half_extents_m: [1.0, 0.5, 1.0],
            },
            position_m: [0.0, 3.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            mass_kg: 0.0,
            friction: 0.5,
            restitution: 0.0,
        }];

        let trace = run_native_scenario_3d(&scenario).unwrap();
        assert_eq!(trace.final_sample().unwrap().bodies[0].position_m, [0.0, 3.0, 0.0]);
    }

    #[test]
    fn trace_comparison_matches_by_body_id_not_vector_order() {
        let backend = PhysicsBackendDescriptor {
            engine: "test".to_owned(),
            engine_version: "1".to_owned(),
            profile: "default".to_owned(),
        };
        let body_a = BodyObservation3d {
            body_id: ValidationBodyId(1),
            position_m: [1.0, 0.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            sleeping: false,
        };
        let body_b = BodyObservation3d {
            body_id: ValidationBodyId(2),
            position_m: [2.0, 0.0, 0.0],
            linear_velocity_mps: [0.0, 0.0, 0.0],
            sleeping: true,
        };
        let left = PhysicsTrace3d {
            schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
            scenario_id: "identity".to_owned(),
            backend: backend.clone(),
            samples: vec![PhysicsSample3d {
                step: 0,
                time_s: 0.0,
                bodies: vec![body_a.clone(), body_b.clone()],
            }],
        };
        let right = PhysicsTrace3d {
            backend,
            samples: vec![PhysicsSample3d {
                step: 0,
                time_s: 0.0,
                bodies: vec![body_b, body_a],
            }],
            ..left.clone()
        };

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.compared_bodies, 2);
        assert_eq!(metrics.max_position_error_m, 0.0);
        assert_eq!(metrics.max_linear_velocity_error_mps, 0.0);
        assert_eq!(metrics.sleeping_mismatches, 0);
    }

    #[test]
    fn non_finite_observation_is_explicit_instead_of_poisoning_metrics() {
        let backend = PhysicsBackendDescriptor {
            engine: "test".to_owned(),
            engine_version: "1".to_owned(),
            profile: "default".to_owned(),
        };
        let left = PhysicsTrace3d {
            schema_version: PHYSICS_TRACE_SCHEMA_VERSION,
            scenario_id: "nan".to_owned(),
            backend: backend.clone(),
            samples: vec![PhysicsSample3d {
                step: 1,
                time_s: 1.0,
                bodies: vec![BodyObservation3d {
                    body_id: ValidationBodyId(1),
                    position_m: [f64::NAN, 0.0, 0.0],
                    linear_velocity_mps: [0.0, 0.0, 0.0],
                    sleeping: false,
                }],
            }],
        };
        let mut right = left.clone();
        right.backend = backend;
        right.samples[0].bodies[0].position_m = [0.0, 0.0, 0.0];

        let metrics = compare_traces_3d(&left, &right).unwrap();
        assert_eq!(metrics.left_non_finite_observations, 1);
        assert_eq!(metrics.right_non_finite_observations, 0);
        assert_eq!(metrics.max_position_error_m, 0.0);
        assert!(metrics.rms_position_error_m.is_finite());
    }
}
