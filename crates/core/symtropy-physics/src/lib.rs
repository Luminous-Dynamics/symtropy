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
//! - `thermal` — conservative thermodynamic primitives and conductive exchange
//! - `energy` — deterministic double-entry accounting for cross-domain energy transfers

pub mod articulation;
pub mod body;
pub mod broadphase;
pub mod ccd;
pub mod constraint;
pub mod contact;
pub mod diagnostics;
pub mod energy;
pub mod epa;
pub mod gjk;
pub mod integrator;
pub mod island;
pub mod joints;
pub mod manifold_gen;
pub mod raycast;
pub mod replay;
pub mod support_map;
pub mod thermal;
pub mod world;

pub use articulation::{ArticulatedChain, ChainBuilder, LinkSpec};
pub use body::{BodyHandle, BodyType, NetId, RigidBody};
pub use broadphase::{Aabb, Lbvh, morton_encode, morton_prefix};
pub use constraint::Constraint;
pub use contact::{CollisionEvent, ContactCache, ContactManifold, SensorEvent};
pub use diagnostics::{InvariantDrift, InvariantSnapshot};
pub use energy::{
    EnergyAudit, EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransfer,
    EnergyTransferKind, EnergyTransferLedger,
};
pub use epa::EpaResult;
pub use integrator::nan_zeroed_count;
pub use joints::{BallJoint, FixedJoint, HingeJoint, MotorDrive, PrismaticJoint};
pub use replay::{ReplayTape, WorldCommand, WorldSnapshot, apply_commands};
pub use thermal::{
    ABSOLUTE_ZERO_K, HeatExchange, ThermalBody, ThermalError, ThermalMaterial, ThermalState,
    conductive_exchange, conductive_exchange_bodies,
};
pub use world::{NoOpCallback, PhysicsCallback, PhysicsWorld};
