// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! N-dimensional rigid body physics engine.
//!
//! Provides dimension-agnostic rigid body dynamics, GJK collision detection,
//! and constraint solving. All types are `const D: usize` parameterized for
//! stack-allocated, SIMD-friendly physics at 2D/3D/4D.
//!
//! # Architecture
//! - `RigidBody<D>` — position, velocity, angular velocity (bivector), mass, collider
//! - `PhysicsWorld<D>` — owns bodies, steps simulation, resolves collisions
//! - `gjk::intersects()` — GJK intersection test for any `Shape<D>`
//! - `contact::ContactManifold<D>` — collision contact data
//! - `integrator` — semi-implicit Euler with bivector angular dynamics
//! - `angular_dynamics` — validated 3D principal-inertia/asymmetric-top reference dynamics
//! - `mass_properties_3d` — checked analytical primitive mass properties
//! - `body_primitives_3d` — atomic 3D primitive geometry/mass-property constructors
//! - `world_energy_3d` — canonical checked 3D kinetic-energy evidence over live world state

pub mod angular_dynamics;
pub mod articulation;
pub mod body;
pub mod body_primitives_3d;
pub mod broadphase;
pub mod ccd;
pub mod constraint;
pub mod contact;
pub mod diagnostics;
pub mod epa;
pub mod gjk;
pub mod integrator;
pub mod island;
pub mod joints;
pub mod manifold_gen;
pub mod mass_properties_3d;
pub mod raycast;
pub mod replay;
pub mod support_map;
pub mod world;
pub mod world_energy_3d;

pub use angular_dynamics::{
    AngularDynamicsError, AngularStep3, PrincipalInertia3, angular_vector_to_bivector,
    angular_velocity_at_offset, angular_velocity_from_world_momentum,
    bivector_to_angular_vector, rotational_kinetic_energy, step_principal_inertia,
    world_angular_momentum,
};
pub use articulation::{ArticulatedChain, ChainBuilder, LinkSpec};
pub use body::{BodyHandle, BodyType, NetId, RigidBody, RigidBodyEnergyError};
pub use body_primitives_3d::{
    PrimitiveBody3Error, dynamic_solid_capsule_3d, dynamic_solid_cuboid_3d,
    dynamic_solid_sphere_3d,
};
pub use broadphase::{Aabb, Lbvh, morton_encode, morton_prefix};
pub use constraint::Constraint;
pub use contact::{CollisionEvent, ContactCache, ContactManifold, SensorEvent};
pub use diagnostics::{InvariantDrift, InvariantSnapshot};
pub use epa::EpaResult;
pub use integrator::nan_zeroed_count;
pub use joints::{BallJoint, FixedJoint, HingeJoint, MotorDrive, PrismaticJoint};
pub use mass_properties_3d::{MassProperties3, MassProperties3Error};
pub use replay::{ReplayTape, WorldCommand, WorldSnapshot, apply_commands};
pub use world::{NoOpCallback, PhysicsCallback, PhysicsWorld};
pub use world_energy_3d::{
    BodyKineticEnergy3, PhysicsWorldEnergy3dExt, WorldEnergy3dError,
};
