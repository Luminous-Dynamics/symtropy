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
//! - `world_energy_3d` — canonical checked 3D kinetic-energy evidence over live world state
//! - `thermal` — conservative thermodynamic primitives and conductive exchange
//! - `energy` — deterministic double-entry accounting for cross-domain energy transfers
//! - `energy_checked` — overflow-aware deterministic ledger reductions
//! - `energy_state` — measured reservoir state reconciled against the causal ledger
//! - `energy_reconciliation_checked` — revalidation for serialized reconciliation evidence
//! - `thermal_audit` — transactional thermal couplings and second-law diagnostics
//! - `external_heat` — audited energy exchange across the simulation boundary
//! - `dissipation` — measured mechanical loss converted into audited sensible heat

pub mod angular_dynamics;
pub mod articulation;
pub mod body;
mod body_energy_3d;
pub mod broadphase;
pub mod ccd;
pub mod constraint;
pub mod contact;
pub mod diagnostics;
pub mod dissipation;
pub mod energy;
pub mod energy_checked;
pub mod energy_reconciliation_checked;
pub mod energy_state;
pub mod epa;
pub mod external_heat;
pub mod gjk;
pub mod integrator;
pub mod island;
pub mod joints;
pub mod manifold_gen;
pub mod raycast;
pub mod replay;
pub mod support_map;
pub mod thermal;
pub mod thermal_audit;
pub mod world;
pub mod world_energy_3d;

pub use angular_dynamics::{
    AngularDynamicsError, AngularStep3, PrincipalInertia3, angular_vector_to_bivector,
    angular_velocity_at_offset, angular_velocity_from_world_momentum,
    bivector_to_angular_vector, rotational_kinetic_energy, step_principal_inertia,
    world_angular_momentum,
};
pub use articulation::{ArticulatedChain, ChainBuilder, LinkSpec};
pub use body::{BodyHandle, BodyType, NetId, RigidBody};
pub use body_energy_3d::RigidBodyEnergyError;
pub use broadphase::{Aabb, Lbvh, morton_encode, morton_prefix};
pub use constraint::Constraint;
pub use contact::{CollisionEvent, ContactCache, ContactManifold, SensorEvent};
pub use diagnostics::{InvariantDrift, InvariantSnapshot};
pub use dissipation::{
    DissipationError, FrictionHeatResult, HeatPartition, apply_friction_impulse_with_heat,
};
pub use energy::{
    EnergyAudit, EnergyForm, EnergyLedgerError, EnergyOwner, EnergyPort, EnergyTransfer,
    EnergyTransferKind, EnergyTransferLedger,
};
pub use energy_checked::{EnergyAggregateError, EnergyTransferLedgerCheckedExt};
pub use energy_reconciliation_checked::{
    EnergyReconciliationEvidenceError, EnergyReconciliationEvidenceExt,
};
pub use energy_state::{
    EnergyReconciliationAudit, EnergyStateAuditError, EnergyStateSnapshot, ReservoirEnergy,
    ReservoirPresenceChange, ReservoirPresenceChangeKind, ReservoirReconciliation,
};
pub use epa::EpaResult;
pub use external_heat::{
    EXTERNAL_HEAT_TRANSFER_KIND, ExternalHeatError, exchange_external_heat_audited,
};
pub use integrator::nan_zeroed_count;
pub use joints::{BallJoint, FixedJoint, HingeJoint, MotorDrive, PrismaticJoint};
pub use replay::{
    ReplayTape, WorldCommand, WorldSnapshot, apply_commands, apply_commands_audited,
};
pub use thermal::{
    ABSOLUTE_ZERO_K, HeatExchange, ThermalBody, ThermalError, ThermalMaterial, ThermalState,
    conductive_exchange, conductive_exchange_bodies,
};
pub use thermal_audit::{
    AuditedThermalError, EntropyAuditError, PairEntropyAudit, constant_cp_pair_entropy_audit,
    conductive_exchange_bodies_audited,
};
pub use world::{NoOpCallback, PhysicsCallback, PhysicsWorld};
pub use world_energy_3d::{BodyKineticEnergy3, PhysicsWorldEnergy3dExt, WorldEnergy3dError};
