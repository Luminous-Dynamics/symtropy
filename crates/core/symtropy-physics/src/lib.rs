// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! N-dimensional rigid body physics engine.
//!
//! Provides dimension-agnostic rigid body dynamics, GJK collision detection,
//! and constraint solving. All types are `const D: usize` parameterized for
//! stack-allocated, SIMD-friendly physics ticks with full SIMD optimization for the common 2D/3D/4D cases.
//!
//! # Architecture
//! - `RigidBody<D>` — position, velocity, angular velocity (bivector), mass, collider
//! - `PhysicsWorld<D>` — owns bodies, steps simulation, resolves collisions
//! - `gjk::intersects()` — GJK intersection test for any `Shape<D>`
//! - `contact::ContactManifold<D>` — collision contact data
//! - `integrator` — semi-implicit Euler with bivector angular dynamics
//! - `body_energy_2d` — checked 2D kinetic-energy evidence under the current solver convention
//! - `angular_dynamics` — validated 3D principal-inertia/asymmetric-top reference dynamics
//! - `world_energy_3d` — canonical checked 3D kinetic-energy evidence over live world state
//! - `mechanical_units` — explicit checked M-L-T calibration from solver energy to SI Joules
//! - `friction_coordinates` — solver-local friction identity without fixed-tick authority
//! - `friction_authority` — pluggable solver friction application/evidence authority
//! - `friction_step` — checked native-index dispatch + typed authority failures
//! - `friction_energy_2d` — checked signed 2D A/B pair-energy evidence
//! - `friction_transition_energy_2d` — stable factored 2D transition-energy evidence
//! - `friction_evidence` — signed pre/post mechanical evidence around one friction impulse
//! - `friction_transaction` — exactly-once friction lifecycle (`Applied` to terminal outcome)
//! - `friction_promotion` — SI-native centered measured-loss promotion into heat + ledger authority
//! - `friction_promotion_calibrated` — explicit solver-unit to SI promotion boundary
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
mod body_energy_2d;
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
pub mod friction_authority;
pub mod friction_coordinates;
pub mod friction_energy_2d;
pub mod friction_evidence;
pub mod friction_promotion;
pub mod friction_promotion_calibrated;
mod friction_solver_execution;
pub mod friction_step;
pub mod friction_transaction;
pub mod friction_transition_energy_2d;
pub mod gjk;
pub mod integrator;
pub mod island;
pub mod joints;
pub mod manifold_gen;
pub mod mechanical_units;
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
pub use body_energy_2d::RigidBodyEnergy2dError;
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
pub use friction_authority::{DirectFrictionImpulseAuthority, FrictionImpulseAuthority};
pub use friction_coordinates::{
    FrictionSolverCoordinateComponent, FrictionSolverCoordinateError, FrictionSolverCoordinates,
};
pub use friction_energy_2d::{
    FrictionPairEnergy2dError, FrictionPairEnergy2dSnapshot, FrictionPairEnergyChange2d,
    FrictionPairEnergyDelta2d, capture_friction_pair_energy_2d_checked,
    classify_friction_pair_energy_change_2d_checked,
};
pub use friction_evidence::{
    BoundFrictionMechanicalObservation, FrictionEvidenceError, FrictionEvidenceRegime,
    FrictionMechanicalDelta, FrictionMechanicalObservation, FrictionTransactionId,
    apply_friction_impulse_measured, apply_friction_impulse_measured_bound,
    classify_friction_evidence_regime,
};
pub use friction_promotion::{
    FrictionPromotionError, FrictionPromotionReceipt, promote_applied_friction_loss_to_heat,
};
pub use friction_promotion_calibrated::{
    CalibratedFrictionPromotionError, promote_applied_friction_loss_to_heat_calibrated,
};
pub use friction_step::{
    FrictionAuthorityFailure, FrictionStepError, execute_friction_impulse_at_indices,
};
pub use friction_transaction::{
    AppliedFrictionTransaction, FrictionApplicationError, FrictionApplicationRollbackError,
    FrictionDiagnosticFinalizeError, FrictionDiagnosticReason, FrictionTransactionJournal,
    FrictionTransactionPhase, FrictionTransactionTransitionError, apply_friction_impulse_once,
    finalize_friction_diagnostic, rollback_applied_friction_impulse,
};
pub use friction_transition_energy_2d::{
    FrictionBodyTransitionBasis2d, FrictionPairTransitionBasis2d,
    FrictionPairTransitionEnergy2d, FrictionTransitionDelta2d,
    FrictionTransitionEnergy2dError, capture_friction_pair_transition_basis_2d_checked,
    classify_friction_pair_transition_2d_checked,
};
pub use integrator::nan_zeroed_count;
pub use joints::{BallJoint, FixedJoint, HingeJoint, MotorDrive, PrismaticJoint};
pub use mechanical_units::{MechanicalUnitCalibration, MechanicalUnitCalibrationError};
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
