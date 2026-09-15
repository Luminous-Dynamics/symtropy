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

pub mod articulation;
pub mod authority_endpoint;
pub mod authority_namespace;
pub mod authority_observation;
mod authority_time;
pub mod body;
pub mod broadphase;
pub mod ccd;
pub mod constraint;
pub mod contact;
pub mod diagnostics;
pub mod epa;
pub mod evidence_authority;
pub mod evidence_frame;
pub mod gjk;
pub mod identity_authority;
mod identity_mutation;
pub mod integrator;
pub mod island;
pub mod joints;
pub mod manifold_gen;
pub mod qualified_endpoint;
pub mod raycast;
pub mod replay;
pub mod sampled_presence;
pub mod support_map;
pub mod world;

pub use articulation::{ArticulatedChain, ChainBuilder, LinkSpec};
pub use authority_endpoint::{
    AuthorityEndpointBoxObservation, AuthorityEndpointBoxSpec, AuthorityEndpointObservationError,
    EndpointBoxSpecError, EndpointMembership,
};
pub use authority_namespace::{
    LocalAuthorityNamespaceError, LocalNamespacePhysicsAuthorityWorld,
    LocalQualifiedPhysicalAuthority, LocalQualifiedValidatedNetBody, LocalQualifiedWorldGeneration,
};
pub use authority_observation::{
    AuthorityBodySnapshot, AuthorityPairObservationError, AuthorityPairSnapshot,
};
pub use body::{BodyHandle, BodyType, NetId, RigidBody};
pub use broadphase::{Aabb, Lbvh, morton_encode, morton_prefix};
pub use constraint::Constraint;
pub use contact::{CollisionEvent, ContactCache, ContactManifold, SensorEvent};
pub use diagnostics::{InvariantDrift, InvariantSnapshot};
pub use epa::EpaResult;
pub use evidence_authority::{
    LocalEvidenceAuthorityError, LocalEvidenceAuthoritySealFailure,
    LocalEvidencePhysicsAuthorityWorld, LocalQualifiedAuthorityStepStamp,
    LocalTaintedEvidenceAuthority, LocalTemporalIncarnationId,
};
pub use evidence_frame::{
    DecodedPhysicsEvidenceFrameV1, PHYSICS_EVIDENCE_FRAME_V1_MAGIC,
    PHYSICS_EVIDENCE_FRAME_V1_MAX_KIND_DOMAIN_LEN, PHYSICS_EVIDENCE_FRAME_V1_MAX_PAYLOAD_LEN,
    PHYSICS_EVIDENCE_FRAME_V1_VERSION, PhysicsEvidenceFrameKindV1,
    PhysicsEvidenceFrameV1Error, decode_physics_evidence_frame_v1,
    encode_physics_evidence_frame_v1,
};
pub use identity_authority::{
    AuthorityStepStamp, PhysicalAuthorityId, PhysicsAuthorityTemporalError, PhysicsAuthorityWorld,
    PhysicsBodySubject, PhysicsIdentityError, ValidatedNetBody, WorldGenerationId,
};
pub use identity_mutation::NetIdentityMutationError;
pub use integrator::nan_zeroed_count;
pub use joints::{BallJoint, FixedJoint, HingeJoint, MotorDrive, PrismaticJoint};
pub use qualified_endpoint::{
    LocalQualifiedStampedEndpointBoxObservation, LocalQualifiedStampedEndpointObservationError,
};
pub use replay::{ReplayTape, WorldCommand, WorldSnapshot, apply_commands};
pub use sampled_presence::{
    LocalConsecutiveSampledEndpointPresence, LocalConsecutiveSampledPresenceError,
};
pub use world::{NoOpCallback, PhysicsCallback, PhysicsWorld};
