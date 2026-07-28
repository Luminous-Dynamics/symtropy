// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic semantic encoding of exact Symtropy physics state.
//!
//! The HDC vector is never authoritative. Positions, velocities, contacts,
//! ownership, replay hashes, and solver state remain exact in
//! `symtropy-physics`. This crate provides a reproducible associative index over
//! that state.

use std::fmt;

pub mod memory;
pub use memory::{EpisodeBuilder, EpisodeMemory, EpisodeMetadata, PhysicsEpisode, RetrievalHit};

use serde::{Deserialize, Serialize};
use symtropy_hdc_core::{
    BipolarBundle, BipolarHV, EncoderSpec, HdcError, ItemMemory, LevelEncoder, StableHash64,
};
use symtropy_math::{Capsule, CompoundShape, ConvexHull, HalfSpace, HyperBox, Shape, Sphere};
use symtropy_physics::{BodyHandle, BodyType, PhysicsWorld, RigidBody};

/// Which identity signal is included in a body vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdentityPolicy {
    /// Omit instance identity so structurally similar scenes can match even
    /// when their body handles differ.
    None,
    /// Encode the transient body handle.
    Handle,
    /// Prefer stable `NetId`; fall back to the body handle when absent.
    NetIdPreferred,
}

/// Reference frame used for positional features.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReferenceFramePolicy {
    World,
    CenterOfDynamicMass,
    Anchor(BodyHandle),
}

/// Versioned numerical ranges and semantic choices for physics encoding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhysicsEncoderConfig {
    pub hdc: EncoderSpec,
    pub identity_policy: IdentityPolicy,
    pub reference_frame: ReferenceFramePolicy,
    pub position_extent: f64,
    pub velocity_extent: f64,
    pub angular_velocity_extent: f64,
    pub mass_extent: f64,
    pub shape_extent: f64,
    pub impulse_extent: f64,
    pub penetration_extent: f64,
    pub energy_extent: f64,
    pub include_tick: bool,
}

impl Default for PhysicsEncoderConfig {
    fn default() -> Self {
        Self {
            hdc: EncoderSpec::default(),
            identity_policy: IdentityPolicy::NetIdPreferred,
            reference_frame: ReferenceFramePolicy::CenterOfDynamicMass,
            position_extent: 1_000.0,
            velocity_extent: 250.0,
            angular_velocity_extent: 50.0,
            mass_extent: 100_000.0,
            shape_extent: 1_000.0,
            impulse_extent: 1_000_000.0,
            penetration_extent: 10.0,
            energy_extent: 1.0e12,
            include_tick: false,
        }
    }
}

impl PhysicsEncoderConfig {
    pub fn validate(&self) -> Result<(), PhysicsHdcError> {
        self.hdc.validate()?;
        for (name, extent) in [
            ("position_extent", self.position_extent),
            ("velocity_extent", self.velocity_extent),
            ("angular_velocity_extent", self.angular_velocity_extent),
            ("mass_extent", self.mass_extent),
            ("shape_extent", self.shape_extent),
            ("impulse_extent", self.impulse_extent),
            ("penetration_extent", self.penetration_extent),
            ("energy_extent", self.energy_extent),
        ] {
            if !extent.is_finite() || extent <= 0.0 {
                return Err(PhysicsHdcError::InvalidConfig(format!(
                    "{name} must be finite and positive"
                )));
            }
        }
        Ok(())
    }

    /// Fingerprint includes both the underlying HDC contract and all physics
    /// semantic/range choices.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = StableHash64::with_seed(self.hdc.fingerprint());
        hash.write_u32(identity_code(self.identity_policy));
        match self.reference_frame {
            ReferenceFramePolicy::World => hash.write_u32(0),
            ReferenceFramePolicy::CenterOfDynamicMass => hash.write_u32(1),
            ReferenceFramePolicy::Anchor(handle) => {
                hash.write_u32(2);
                hash.write_u64(handle.0 as u64);
            }
        }
        for value in [
            self.position_extent,
            self.velocity_extent,
            self.angular_velocity_extent,
            self.mass_extent,
            self.shape_extent,
            self.impulse_extent,
            self.penetration_extent,
            self.energy_extent,
        ] {
            hash.write_u64(value.to_bits());
        }
        hash.write_u32(if self.include_tick { 1 } else { 0 });
        hash.finish()
    }
}

#[derive(Debug)]
pub enum PhysicsHdcError {
    Hdc(HdcError),
    MissingAnchor(BodyHandle),
    InvalidConfig(String),
    EncoderMismatch { expected: u64, actual: u64 },
    VectorDimensionMismatch { expected: usize, actual: usize },
    EmptyEpisode,
    DuplicateEpisodeId(String),
}

impl fmt::Display for PhysicsHdcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hdc(error) => write!(f, "{error}"),
            Self::MissingAnchor(handle) => write!(f, "missing reference-frame anchor {handle:?}"),
            Self::InvalidConfig(message) => write!(f, "invalid physics HDC config: {message}"),
            Self::EncoderMismatch { expected, actual } => write!(
                f,
                "HDC encoder mismatch: expected {expected:016x}, got {actual:016x}"
            ),
            Self::VectorDimensionMismatch { expected, actual } => write!(
                f,
                "HDC vector dimension mismatch: expected {expected}, got {actual}"
            ),
            Self::EmptyEpisode => write!(f, "physics episode must contain at least one frame"),
            Self::DuplicateEpisodeId(id) => write!(f, "duplicate physics episode id: {id}"),
        }
    }
}

impl std::error::Error for PhysicsHdcError {}

impl From<HdcError> for PhysicsHdcError {
    fn from(value: HdcError) -> Self {
        Self::Hdc(value)
    }
}

/// Deterministic, non-cryptographic digest of the exact observable body,
/// contact, event, and solver fields covered by algorithm version 1. It is
/// intended for provenance and accidental-divergence detection, not
/// adversarial security or cryptographic authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExactStateDigest {
    pub algorithm_version: u32,
    pub low: u64,
    pub high: u64,
}

impl fmt::Display for ExactStateDigest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "v{}:{:016x}{:016x}",
            self.algorithm_version, self.high, self.low
        )
    }
}

/// HDC representation of one exact physics frame.
#[derive(Debug, Clone)]
pub struct EncodedPhysicsFrame<const D: usize> {
    pub tick: u64,
    pub encoder_fingerprint: u64,
    pub exact_digest: ExactStateDigest,
    pub reference_origin: [f64; D],
    pub body_count: usize,
    pub collision_count: usize,
    pub vector: BipolarHV,
}

/// Stateless deterministic encoder. It may be reused across worlds and runs.
#[derive(Debug, Clone)]
pub struct PhysicsFrameEncoder {
    config: PhysicsEncoderConfig,
    pub(crate) memory: ItemMemory,
    position: LevelEncoder,
    velocity: LevelEncoder,
    angular_velocity: LevelEncoder,
    mass: LevelEncoder,
    shape: LevelEncoder,
    impulse: LevelEncoder,
    penetration: LevelEncoder,
    energy: LevelEncoder,
    unit: LevelEncoder,
}

impl PhysicsFrameEncoder {
    pub fn new(config: PhysicsEncoderConfig) -> Result<Self, PhysicsHdcError> {
        config.validate()?;
        let memory = ItemMemory::new(config.hdc.clone())?;
        Ok(Self {
            position: memory.scalar_encoder("physics.position.v1"),
            velocity: memory.scalar_encoder("physics.velocity.v1"),
            angular_velocity: memory.scalar_encoder("physics.angular-velocity.v1"),
            mass: memory.scalar_encoder("physics.mass.v1"),
            shape: memory.scalar_encoder("physics.shape-extent.v1"),
            impulse: memory.scalar_encoder("physics.impulse.v1"),
            penetration: memory.scalar_encoder("physics.penetration.v1"),
            energy: memory.scalar_encoder("physics.energy.v1"),
            unit: memory.scalar_encoder("physics.unit-interval.v1"),
            memory,
            config,
        })
    }

    pub fn config(&self) -> &PhysicsEncoderConfig {
        &self.config
    }

    pub fn encode_world<const D: usize>(
        &self,
        tick: u64,
        world: &PhysicsWorld<D>,
    ) -> Result<EncodedPhysicsFrame<D>, PhysicsHdcError> {
        let origin = self.reference_origin(world)?;
        let tie = self.memory.tie_breaker("physics-frame");
        let mut scene = BipolarBundle::new(self.config.hdc.dimension);
        scene.add(&self.memory.item("record-kind", "physics-frame"))?;

        if self.config.include_tick {
            scene.add(&self.bind_item("tick", "tick", &tick.to_string())?)?;
        }

        let mut bodies: Vec<_> = world.bodies.iter().collect();
        bodies.sort_by_key(|body| body.handle);
        for body in bodies {
            scene.add(&self.encode_body(body, &origin)?)?;
        }

        let mut events: Vec<_> = world.collision_events.iter().collect();
        events.sort_by_key(|event| (event.body_a, event.body_b));
        for event in events {
            scene.add(&self.encode_collision(world, event)?)?;
        }

        scene.add(&self.encode_invariants(world)?)?;
        let vector = scene.finish(&tie)?;
        Ok(EncodedPhysicsFrame {
            tick,
            encoder_fingerprint: self.config.fingerprint(),
            exact_digest: exact_world_digest(world),
            reference_origin: std::array::from_fn(|axis| origin[axis]),
            body_count: world.bodies.len(),
            collision_count: world.collision_events.len(),
            vector,
        })
    }

    fn reference_origin<const D: usize>(
        &self,
        world: &PhysicsWorld<D>,
    ) -> Result<[f64; D], PhysicsHdcError> {
        match self.config.reference_frame {
            ReferenceFramePolicy::World => Ok([0.0; D]),
            ReferenceFramePolicy::Anchor(handle) => {
                let body = world
                    .body(handle)
                    .ok_or(PhysicsHdcError::MissingAnchor(handle))?;
                Ok(std::array::from_fn(|axis| {
                    body.transform.translation.0[axis]
                }))
            }
            ReferenceFramePolicy::CenterOfDynamicMass => {
                let mut total_mass = 0.0;
                let mut weighted = [0.0; D];
                for body in &world.bodies {
                    if body.body_type != BodyType::Dynamic || body.mass <= 0.0 {
                        continue;
                    }
                    total_mass += body.mass;
                    for (w, t) in weighted.iter_mut().zip(body.transform.translation.0.iter()) {
                        *w += body.mass * t;
                    }
                }
                if total_mass > 0.0 {
                    for coordinate in &mut weighted {
                        *coordinate /= total_mass;
                    }
                }
                Ok(weighted)
            }
        }
    }

    fn encode_body<const D: usize>(
        &self,
        body: &RigidBody<D>,
        origin: &[f64; D],
    ) -> Result<BipolarHV, PhysicsHdcError> {
        let mut bundle = BipolarBundle::new(self.config.hdc.dimension);
        bundle.add(&self.memory.item("record-kind", "body"))?;
        bundle.add(&self.bind_item("body-type", "body-type", body_type_name(body.body_type))?)?;
        bundle.add(&self.bind_item("shape-kind", "shape-kind", shape_kind(body))?)?;
        bundle.add(&self.bind_item("sleeping", "boolean", boolean_name(body.sleeping))?)?;
        bundle.add(&self.bind_item("sensor", "boolean", boolean_name(body.is_sensor))?)?;

        if let Some(identity) = body_identity(&self.config, body) {
            bundle.add(&self.bind_item("identity", "body-identity", &identity)?)?;
        }

        bundle.add(&self.bind_scalar(
            "mass",
            &self.mass,
            body.mass,
            0.0,
            self.config.mass_extent,
        )?)?;
        bundle.add(&self.bind_scalar("friction", &self.unit, body.friction, 0.0, 1.0)?)?;
        bundle.add(&self.bind_scalar("restitution", &self.unit, body.restitution, 0.0, 1.0)?)?;

        for (axis, orig) in origin.iter().enumerate() {
            let axis_name = axis.to_string();
            let position = body.transform.translation.0[axis] - orig;
            bundle.add(&self.bind_scalar(
                &format!("position.{axis_name}"),
                &self.position,
                position,
                -self.config.position_extent,
                self.config.position_extent,
            )?)?;
            bundle.add(&self.bind_scalar(
                &format!("linear-velocity.{axis_name}"),
                &self.velocity,
                body.linear_velocity[axis],
                -self.config.velocity_extent,
                self.config.velocity_extent,
            )?)?;
        }

        for first in 0..D {
            for second in (first + 1)..D {
                bundle.add(&self.bind_scalar(
                    &format!("angular-velocity.{first}.{second}"),
                    &self.angular_velocity,
                    body.angular_velocity.get(first, second),
                    -self.config.angular_velocity_extent,
                    self.config.angular_velocity_extent,
                )?)?;
            }
        }

        let rotation = body.transform.rotation.to_matrix();
        for row in 0..D {
            for column in 0..D {
                bundle.add(&self.bind_scalar(
                    &format!("rotation.{row}.{column}"),
                    &self.unit,
                    (rotation[(row, column)] + 1.0) * 0.5,
                    0.0,
                    1.0,
                )?)?;
            }
        }

        self.add_shape_features(&mut bundle, body)?;
        bundle
            .finish(&self.memory.tie_breaker("body-record"))
            .map_err(Into::into)
    }

    fn add_shape_features<const D: usize>(
        &self,
        bundle: &mut BipolarBundle,
        body: &RigidBody<D>,
    ) -> Result<(), PhysicsHdcError> {
        if let Some(sphere) = body.collider.as_any().downcast_ref::<Sphere<D>>() {
            bundle.add(&self.bind_scalar(
                "shape.radius",
                &self.shape,
                sphere.radius,
                0.0,
                self.config.shape_extent,
            )?)?;
        } else if let Some(hyperbox) = body.collider.as_any().downcast_ref::<HyperBox<D>>() {
            for (axis, half_extent) in hyperbox.half_extents.iter().copied().enumerate() {
                bundle.add(&self.bind_scalar(
                    &format!("shape.half-extent.{axis}"),
                    &self.shape,
                    half_extent,
                    0.0,
                    self.config.shape_extent,
                )?)?;
            }
        } else if let Some(capsule) = body.collider.as_any().downcast_ref::<Capsule<D>>() {
            bundle.add(&self.bind_scalar(
                "shape.radius",
                &self.shape,
                capsule.radius,
                0.0,
                self.config.shape_extent,
            )?)?;
            bundle.add(&self.bind_scalar(
                "shape.half-height",
                &self.shape,
                capsule.half_height,
                0.0,
                self.config.shape_extent,
            )?)?;
            bundle.add(&self.bind_item("shape.axis", "axis", &capsule.axis.to_string())?)?;
        } else if let Some(halfspace) = body.collider.as_any().downcast_ref::<HalfSpace<D>>() {
            bundle.add(&self.bind_scalar(
                "shape.plane-offset",
                &self.position,
                halfspace.offset,
                -self.config.position_extent,
                self.config.position_extent,
            )?)?;
            for axis in 0..D {
                bundle.add(&self.bind_scalar(
                    &format!("shape.normal.{axis}"),
                    &self.unit,
                    (halfspace.normal[axis] + 1.0) * 0.5,
                    0.0,
                    1.0,
                )?)?;
            }
        } else if let Some(hull) = body.collider.as_any().downcast_ref::<ConvexHull<D>>() {
            bundle.add(&self.bind_scalar(
                "shape.vertex-count",
                &self.shape,
                hull.num_vertices() as f64,
                0.0,
                4_096.0,
            )?)?;
        } else if let Some(compound) = body.collider.as_any().downcast_ref::<CompoundShape<D>>() {
            bundle.add(&self.bind_scalar(
                "shape.child-count",
                &self.shape,
                compound.child_count() as f64,
                0.0,
                256.0,
            )?)?;
        }
        Ok(())
    }

    fn encode_collision<const D: usize>(
        &self,
        world: &PhysicsWorld<D>,
        event: &symtropy_physics::CollisionEvent<D>,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        let mut bundle = BipolarBundle::new(self.config.hdc.dimension);
        bundle.add(&self.memory.item("record-kind", "collision"))?;
        if let Some(actor) = event_identity(&self.config, world, event.body_a) {
            bundle.add(&self.bind_item("actor", "body-identity", &actor)?)?;
        }
        if let Some(target) = event_identity(&self.config, world, event.body_b) {
            bundle.add(&self.bind_item("target", "body-identity", &target)?)?;
        }
        bundle.add(&self.bind_scalar(
            "impulse",
            &self.impulse,
            event.impulse,
            0.0,
            self.config.impulse_extent,
        )?)?;
        bundle.add(&self.bind_scalar(
            "penetration",
            &self.penetration,
            event.depth,
            0.0,
            self.config.penetration_extent,
        )?)?;
        for axis in 0..D {
            bundle.add(&self.bind_scalar(
                &format!("normal.{axis}"),
                &self.unit,
                (event.normal[axis] + 1.0) * 0.5,
                0.0,
                1.0,
            )?)?;
        }
        bundle
            .finish(&self.memory.tie_breaker("collision-record"))
            .map_err(Into::into)
    }

    fn encode_invariants<const D: usize>(
        &self,
        world: &PhysicsWorld<D>,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        let invariants = world.invariant_snapshot();
        let mut bundle = BipolarBundle::new(self.config.hdc.dimension);
        bundle.add(&self.memory.item("record-kind", "invariants"))?;
        bundle.add(&self.bind_scalar(
            "kinetic-energy",
            &self.energy,
            invariants.kinetic_energy,
            0.0,
            self.config.energy_extent,
        )?)?;
        bundle.add(&self.bind_scalar(
            "mechanical-energy",
            &self.energy,
            invariants.mechanical_energy,
            -self.config.energy_extent,
            self.config.energy_extent,
        )?)?;
        bundle.add(&self.bind_scalar(
            "max-penetration",
            &self.penetration,
            invariants.max_penetration_depth,
            0.0,
            self.config.penetration_extent,
        )?)?;
        bundle.add(&self.bind_item(
            "numerically-healthy",
            "boolean",
            boolean_name(invariants.is_numerically_healthy(1.0e-8)),
        )?)?;
        bundle
            .finish(&self.memory.tie_breaker("invariant-record"))
            .map_err(Into::into)
    }

    fn bind_item(
        &self,
        role: &str,
        namespace: &str,
        value: &str,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        self.memory
            .role(role)
            .bind(&self.memory.item(namespace, value))
            .map_err(Into::into)
    }

    fn bind_scalar(
        &self,
        role: &str,
        encoder: &LevelEncoder,
        value: f64,
        min: f64,
        max: f64,
    ) -> Result<BipolarHV, PhysicsHdcError> {
        let value = if value.is_finite() { value } else { 0.0 };
        self.memory
            .role(role)
            .bind(&encoder.encode(value, min, max)?)
            .map_err(Into::into)
    }
}

pub fn exact_world_digest<const D: usize>(world: &PhysicsWorld<D>) -> ExactStateDigest {
    let mut low = StableHash64::with_seed(0x4850_5944_4947_4553);
    let mut high = StableHash64::with_seed(0x5453_594d_5452_4f50);
    hash_world(world, &mut low);
    hash_world(world, &mut high);
    ExactStateDigest {
        algorithm_version: 1,
        low: low.finish(),
        high: high.finish(),
    }
}

fn hash_world<const D: usize>(world: &PhysicsWorld<D>, hash: &mut StableHash64) {
    hash.write_u64(D as u64);
    for component in world.gravity.iter() {
        hash.write_u64(component.to_bits());
    }
    hash.write_u64(world.solver_iterations as u64);
    hash.write_u64(world.sleep_threshold.to_bits());
    hash.write_u32(world.sleep_ticks);
    hash.write_u64(world.slop.to_bits());
    hash.write_u64(world.baumgarte.to_bits());
    hash.write_u64(world.compliance.to_bits());
    hash.write_u64(world.constraints.len() as u64);

    let mut bodies: Vec<_> = world.bodies.iter().collect();
    bodies.sort_by_key(|body| body.handle);
    hash.write_u64(bodies.len() as u64);
    for body in bodies {
        hash.write_u64(body.handle.0 as u64);
        hash.write_u64(body.net_id.map(|id| id.0).unwrap_or(u64::MAX));
        hash.write_u32(body_type_code(body.body_type));
        for coordinate in body.transform.translation.0.iter() {
            hash.write_u64(coordinate.to_bits());
        }
        let rotation = body.transform.rotation.to_matrix();
        for value in rotation.iter() {
            hash.write_u64(value.to_bits());
        }
        for value in body.linear_velocity.iter() {
            hash.write_u64(value.to_bits());
        }
        for first in 0..D {
            for second in (first + 1)..D {
                hash.write_u64(body.angular_velocity.get(first, second).to_bits());
                hash.write_u64(body.torque_accumulator.get(first, second).to_bits());
            }
        }
        for value in body.force_accumulator.iter() {
            hash.write_u64(value.to_bits());
        }
        for value in body.inertia.iter() {
            hash.write_u64(value.to_bits());
        }
        for value in body.inv_inertia.iter() {
            hash.write_u64(value.to_bits());
        }
        for value in [
            body.mass,
            body.inv_mass,
            body.friction,
            body.restitution,
            body.linear_damping,
            body.angular_damping,
        ] {
            hash.write_u64(value.to_bits());
        }
        hash.write_u32(if body.sleeping { 1 } else { 0 });
        hash.write_u64(f64::from(body.sleep_timer).to_bits());
        hash.write_u32(body.sleep_counter);
        hash.write_u32(if body.is_sensor { 1 } else { 0 });
        hash.write_u32(body.collision_group);
        hash.write_u32(body.collision_mask);
        hash_shape(body.collider.as_ref(), hash);
    }

    let mut contacts: Vec<_> = world.contacts.iter().collect();
    contacts.sort_by_key(|contact| (contact.body_a, contact.body_b));
    hash.write_u64(contacts.len() as u64);
    for contact in contacts {
        hash.write_u64(contact.body_a.0 as u64);
        hash.write_u64(contact.body_b.0 as u64);
        for component in contact.normal.iter() {
            hash.write_u64(component.to_bits());
        }
        hash.write_u64(contact.elasticity.unwrap_or(f64::NAN).to_bits());
        hash.write_u64(contact.points.len() as u64);
        for point in &contact.points {
            for component in point.position.iter() {
                hash.write_u64(component.to_bits());
            }
            hash.write_u64(point.depth.to_bits());
            hash.write_u64(point.lambda.to_bits());
            hash.write_u64(point.restitution_bias.to_bits());
        }
    }

    let mut events: Vec<_> = world.collision_events.iter().collect();
    events.sort_by(|left, right| {
        (
            left.body_a,
            left.body_b,
            left.impulse.to_bits(),
            left.depth.to_bits(),
        )
            .cmp(&(
                right.body_a,
                right.body_b,
                right.impulse.to_bits(),
                right.depth.to_bits(),
            ))
            .then_with(|| {
                for axis in 0..D {
                    let order = left.normal[axis]
                        .to_bits()
                        .cmp(&right.normal[axis].to_bits());
                    if !order.is_eq() {
                        return order;
                    }
                }
                std::cmp::Ordering::Equal
            })
    });
    hash.write_u64(events.len() as u64);
    for event in events {
        hash.write_u64(event.body_a.0 as u64);
        hash.write_u64(event.body_b.0 as u64);
        hash.write_u64(event.impulse.to_bits());
        hash.write_u64(event.depth.to_bits());
        for component in event.normal.iter() {
            hash.write_u64(component.to_bits());
        }
    }

    let mut sensors: Vec<_> = world.sensor_events.iter().collect();
    sensors.sort_by_key(|event| (event.sensor, event.other));
    hash.write_u64(sensors.len() as u64);
    for event in sensors {
        hash.write_u64(event.sensor.0 as u64);
        hash.write_u64(event.other.0 as u64);
    }
}

fn hash_shape<const D: usize>(shape: &dyn Shape<D>, hash: &mut StableHash64) {
    if let Some(sphere) = shape.as_any().downcast_ref::<Sphere<D>>() {
        hash.write(b"sphere");
        for value in sphere.center.0.iter() {
            hash.write_u64(value.to_bits());
        }
        hash.write_u64(sphere.radius.to_bits());
    } else if let Some(hyperbox) = shape.as_any().downcast_ref::<HyperBox<D>>() {
        hash.write(b"hyperbox");
        for value in hyperbox.half_extents {
            hash.write_u64(value.to_bits());
        }
    } else if let Some(capsule) = shape.as_any().downcast_ref::<Capsule<D>>() {
        hash.write(b"capsule");
        hash.write_u64(capsule.half_height.to_bits());
        hash.write_u64(capsule.radius.to_bits());
        hash.write_u64(capsule.axis as u64);
    } else if let Some(halfspace) = shape.as_any().downcast_ref::<HalfSpace<D>>() {
        hash.write(b"halfspace");
        for value in halfspace.normal.iter() {
            hash.write_u64(value.to_bits());
        }
        hash.write_u64(halfspace.offset.to_bits());
    } else if let Some(hull) = shape.as_any().downcast_ref::<ConvexHull<D>>() {
        hash.write(b"convex-hull");
        hash.write_u64(hull.vertices.len() as u64);
        for vertex in &hull.vertices {
            for value in vertex.iter() {
                hash.write_u64(value.to_bits());
            }
        }
    } else if let Some(compound) = shape.as_any().downcast_ref::<CompoundShape<D>>() {
        hash.write(b"compound");
        hash.write_u64(compound.child_count() as u64);
        for (transform, child) in compound.children() {
            for value in transform.translation.0.iter() {
                hash.write_u64(value.to_bits());
            }
            let rotation = transform.rotation.to_matrix();
            for value in rotation.iter() {
                hash.write_u64(value.to_bits());
            }
            hash_shape(child.as_ref(), hash);
        }
    } else {
        hash.write(b"custom-shape");
        let (center, radius) = shape.bounding_sphere();
        for value in center.0.iter() {
            hash.write_u64(value.to_bits());
        }
        hash.write_u64(radius.to_bits());
    }
}

fn body_identity<const D: usize>(
    config: &PhysicsEncoderConfig,
    body: &RigidBody<D>,
) -> Option<String> {
    match config.identity_policy {
        IdentityPolicy::None => None,
        IdentityPolicy::Handle => Some(format!("handle:{}", body.handle.0)),
        IdentityPolicy::NetIdPreferred => Some(match body.net_id {
            Some(net_id) => format!("net:{}", net_id.0),
            None => format!("handle:{}", body.handle.0),
        }),
    }
}

fn event_identity<const D: usize>(
    config: &PhysicsEncoderConfig,
    world: &PhysicsWorld<D>,
    handle: BodyHandle,
) -> Option<String> {
    match config.identity_policy {
        IdentityPolicy::None => None,
        IdentityPolicy::Handle => Some(format!("handle:{}", handle.0)),
        IdentityPolicy::NetIdPreferred => {
            Some(match world.body(handle).and_then(|body| body.net_id) {
                Some(net_id) => format!("net:{}", net_id.0),
                None => format!("handle:{}", handle.0),
            })
        }
    }
}

fn shape_kind<const D: usize>(body: &RigidBody<D>) -> &'static str {
    let shape = body.collider.as_any();
    if shape.is::<Sphere<D>>() {
        "sphere"
    } else if shape.is::<HyperBox<D>>() {
        "hyperbox"
    } else if shape.is::<Capsule<D>>() {
        "capsule"
    } else if shape.is::<HalfSpace<D>>() {
        "halfspace"
    } else if shape.is::<ConvexHull<D>>() {
        "convex-hull"
    } else if shape.is::<CompoundShape<D>>() {
        "compound"
    } else {
        "custom"
    }
}

fn body_type_name(body_type: BodyType) -> &'static str {
    match body_type {
        BodyType::Static => "static",
        BodyType::Kinematic => "kinematic",
        BodyType::Dynamic => "dynamic",
    }
}

fn body_type_code(body_type: BodyType) -> u32 {
    match body_type {
        BodyType::Static => 0,
        BodyType::Kinematic => 1,
        BodyType::Dynamic => 2,
    }
}

fn identity_code(policy: IdentityPolicy) -> u32 {
    match policy {
        IdentityPolicy::None => 0,
        IdentityPolicy::Handle => 1,
        IdentityPolicy::NetIdPreferred => 2,
    }
}

fn boolean_name(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_math::Point;

    fn test_config() -> PhysicsEncoderConfig {
        let mut config = PhysicsEncoderConfig::default();
        config.hdc.dimension = 4_096;
        config.hdc.scalar_levels = 129;
        config.identity_policy = IdentityPolicy::None;
        config
    }

    #[test]
    fn exact_digest_changes_when_exact_state_changes() {
        let mut world = PhysicsWorld::<3>::default();
        let body = world.add_sphere(Point::origin(), 0.5, 1.0);
        let before = exact_world_digest(&world);
        world.body_mut(body).unwrap().linear_velocity[0] = 1.0;
        let after = exact_world_digest(&world);
        assert_ne!(before, after);
    }

    #[test]
    fn encoding_does_not_mutate_authoritative_world() {
        let mut world = PhysicsWorld::<3>::default();
        world.add_sphere(Point::new([1.0, 2.0, 3.0]), 0.5, 2.0);
        let encoder = PhysicsFrameEncoder::new(test_config()).unwrap();
        let before = exact_world_digest(&world);
        let _ = encoder.encode_world(7, &world).unwrap();
        let after = exact_world_digest(&world);
        assert_eq!(before, after);
    }

    #[test]
    fn encoding_is_deterministic() {
        let mut world = PhysicsWorld::<3>::default();
        world.add_sphere(Point::new([1.0, 2.0, 3.0]), 0.5, 2.0);
        let encoder = PhysicsFrameEncoder::new(test_config()).unwrap();
        assert_eq!(
            encoder.encode_world(7, &world).unwrap().vector,
            encoder.encode_world(7, &world).unwrap().vector
        );
    }

    #[test]
    fn center_of_mass_reference_is_translation_invariant() {
        let encoder = PhysicsFrameEncoder::new(test_config()).unwrap();
        let mut left = PhysicsWorld::<3>::default();
        left.add_sphere(Point::new([-1.0, 0.0, 0.0]), 0.5, 1.0);
        left.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
        let mut right = PhysicsWorld::<3>::default();
        right.add_sphere(Point::new([99.0, -20.0, 5.0]), 0.5, 1.0);
        right.add_sphere(Point::new([101.0, -20.0, 5.0]), 0.5, 1.0);
        let a = encoder.encode_world(0, &left).unwrap();
        let b = encoder.encode_world(0, &right).unwrap();
        assert_eq!(a.vector, b.vector);
        assert_ne!(a.exact_digest, b.exact_digest);
    }

    #[test]
    fn world_reference_preserves_absolute_translation() {
        let mut config = test_config();
        config.reference_frame = ReferenceFramePolicy::World;
        let encoder = PhysicsFrameEncoder::new(config).unwrap();
        let mut left = PhysicsWorld::<2>::default();
        left.add_sphere(Point::new([0.0, 0.0]), 0.5, 1.0);
        let mut right = PhysicsWorld::<2>::default();
        right.add_sphere(Point::new([10.0, 0.0]), 0.5, 1.0);
        let similarity = encoder
            .encode_world(0, &left)
            .unwrap()
            .vector
            .similarity(&encoder.encode_world(0, &right).unwrap().vector)
            .unwrap();
        assert!(similarity < 1.0);
    }

    #[test]
    fn missing_anchor_is_reported() {
        let mut config = test_config();
        config.reference_frame = ReferenceFramePolicy::Anchor(BodyHandle(42));
        let encoder = PhysicsFrameEncoder::new(config).unwrap();
        let world = PhysicsWorld::<3>::default();
        assert!(matches!(
            encoder.encode_world(0, &world),
            Err(PhysicsHdcError::MissingAnchor(BodyHandle(42)))
        ));
    }
}
