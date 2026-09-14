// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Physics-owned live identity authority.
//!
//! This module separates durable physical-world identity from one runtime
//! generation and from ephemeral body handles. It deliberately does not claim
//! persistence or historical continuity across generations.

use serde::{Deserialize, Serialize};

use crate::authority_incarnation::{
    TemporalIncarnationAllocationError, TemporalIncarnationId, mint_temporal_incarnation,
};
use crate::authority_time::{AuthorityTemporalState, TemporalCounterError};
use crate::body::{BodyHandle, NetId, RigidBody};
use crate::identity_mutation::{
    NetIdentityMutationError, PreparedNetIdBinding, commit_prepared_deterministic_insertion,
    commit_prepared_net_id_binding, prepare_deterministic_insertion, prepare_net_id_binding,
};
use crate::world::{PhysicsCallback, PhysicsWorld};

/// Durable identity of the physical authority/persistence lineage that owns a
/// namespace of world generations.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicalAuthorityId(u128);

impl PhysicalAuthorityId {
    pub fn new(value: u128) -> Result<Self, PhysicsIdentityError> {
        if value == 0 {
            return Err(PhysicsIdentityError::ZeroPhysicalAuthorityId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Identity of one instantiated world generation within one physical authority
/// lineage.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct WorldGenerationId(u128);

impl WorldGenerationId {
    pub fn new(value: u128) -> Result<Self, PhysicsIdentityError> {
        if value == 0 {
            return Err(PhysicsIdentityError::ZeroWorldGenerationId);
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u128 {
        self.0
    }
}

/// Durable-looking subject reference for one live physics body.
///
/// This reference intentionally contains no [`BodyHandle`]. A body handle is
/// allocated by one runtime `PhysicsWorld` and is not evidence of identity
/// across reconstruction.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PhysicsBodySubject {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    net_id: NetId,
}

impl PhysicsBodySubject {
    pub const fn new(
        physical_authority_id: PhysicalAuthorityId,
        world_generation_id: WorldGenerationId,
        net_id: NetId,
    ) -> Self {
        Self {
            physical_authority_id,
            world_generation_id,
            net_id,
        }
    }

    pub const fn physical_authority_id(self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn net_id(self) -> NetId {
        self.net_id
    }
}

/// Authority-issued ordering stamp for one normally completed physics step.
///
/// The fields are private and there is no public constructor. A caller can hold
/// or copy a stamp already issued by `PhysicsAuthorityWorld`, but cannot mint a
/// detached claimed step through the public safe API.
///
/// `step_index` is an ordering counter within one exact temporal lineage; it is
/// not elapsed physical time. Generic total ordering is intentionally absent:
/// compare step indexes only after [`Self::same_temporal_lineage`] succeeds.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct AuthorityStepStamp {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    temporal_incarnation_id: TemporalIncarnationId,
    mutation_epoch: u64,
    step_index: u64,
}

impl AuthorityStepStamp {
    pub const fn physical_authority_id(self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn temporal_incarnation_id(self) -> TemporalIncarnationId {
        self.temporal_incarnation_id
    }

    pub const fn mutation_epoch(self) -> u64 {
        self.mutation_epoch
    }

    pub const fn step_index(self) -> u64 {
        self.step_index
    }

    /// Whether two issued stamps belong to one exact live temporal mutation
    /// lineage. Only after this returns true does comparing `step_index` carry
    /// temporal ordering meaning.
    pub fn same_temporal_lineage(self, other: Self) -> bool {
        self.physical_authority_id == other.physical_authority_id
            && self.world_generation_id == other.world_generation_id
            && self.temporal_incarnation_id == other.temporal_incarnation_id
            && self.mutation_epoch == other.mutation_epoch
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsAuthorityConstructionError {
    TemporalIncarnationAllocatorPoisoned,
    TemporalIncarnationExhausted,
}

impl From<TemporalIncarnationAllocationError> for PhysicsAuthorityConstructionError {
    fn from(value: TemporalIncarnationAllocationError) -> Self {
        match value {
            TemporalIncarnationAllocationError::AllocatorPoisoned => {
                Self::TemporalIncarnationAllocatorPoisoned
            }
            TemporalIncarnationAllocationError::Exhausted => Self::TemporalIncarnationExhausted,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsAuthorityTemporalError {
    NonFiniteDeltaTime,
    NonPositiveDeltaTime,
    MutationEpochExhausted,
    StepIndexExhausted,
    InterruptedStepTainted,
    InterruptedMutationTainted,
}

impl From<TemporalCounterError> for PhysicsAuthorityTemporalError {
    fn from(value: TemporalCounterError) -> Self {
        match value {
            TemporalCounterError::MutationEpochExhausted => Self::MutationEpochExhausted,
            TemporalCounterError::StepIndexExhausted => Self::StepIndexExhausted,
            TemporalCounterError::InterruptedStepTainted => Self::InterruptedStepTainted,
            TemporalCounterError::InterruptedMutationTainted => Self::InterruptedMutationTainted,
        }
    }
}

/// Physics-owned binding of one live `PhysicsWorld` to its authority lineage,
/// runtime generation, and process-local temporal incarnation.
///
/// Downstream callers cannot supply a detached claimed authority or temporal
/// incarnation alongside an unrelated world: the identities and world are owned
/// by the same object.
pub struct PhysicsAuthorityWorld<const D: usize> {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    temporal_incarnation_id: TemporalIncarnationId,
    world: PhysicsWorld<D>,
    temporal: AuthorityTemporalState,
}

impl<const D: usize> PhysicsAuthorityWorld<D> {
    /// Source-compatible constructor. Prefer [`Self::try_new`] when allocator
    /// exhaustion/poisoning should be handled explicitly.
    ///
    /// This fails closed before returning a wrapper if a fresh process-local
    /// temporal incarnation cannot be minted.
    pub fn new(
        physical_authority_id: PhysicalAuthorityId,
        world_generation_id: WorldGenerationId,
        world: PhysicsWorld<D>,
    ) -> Self {
        Self::try_new(physical_authority_id, world_generation_id, world)
            .expect("physics authority temporal-incarnation allocation unavailable")
    }

    /// Construct one new live authority wrapper with a freshly minted temporal
    /// incarnation. Callers cannot supply or restore the incarnation value.
    pub fn try_new(
        physical_authority_id: PhysicalAuthorityId,
        world_generation_id: WorldGenerationId,
        world: PhysicsWorld<D>,
    ) -> Result<Self, PhysicsAuthorityConstructionError> {
        let temporal_incarnation_id = mint_temporal_incarnation()?;
        Ok(Self {
            physical_authority_id,
            world_generation_id,
            temporal_incarnation_id,
            world,
            temporal: AuthorityTemporalState::new(),
        })
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn temporal_incarnation_id(&self) -> TemporalIncarnationId {
        self.temporal_incarnation_id
    }

    pub fn world(&self) -> &PhysicsWorld<D> {
        &self.world
    }

    /// Obtain broad mutable access to the raw physics world while explicitly
    /// breaking temporal continuity.
    ///
    /// The mutation epoch advances before the borrow is granted, the authorized
    /// step index resets to zero, and any last authorized-step stamp is
    /// invalidated. This is conservative: even if the caller ultimately makes no
    /// change, temporal continuity is intentionally not preserved across the raw
    /// mutation escape hatch.
    ///
    /// After an interrupted prepared typed mutation, this fails closed rather
    /// than using another raw epoch break as structural recovery.
    pub fn try_world_mut(
        &mut self,
    ) -> Result<&mut PhysicsWorld<D>, PhysicsAuthorityTemporalError> {
        let next_epoch = self.temporal.next_mutation_epoch()?;
        self.temporal.commit_mutation_epoch(next_epoch);
        Ok(&mut self.world)
    }

    /// Source-compatible raw mutable-world escape hatch.
    ///
    /// Prefer [`Self::try_world_mut`] when temporal failure handling matters. The
    /// compatibility wrapper still fails closed: it panics before granting a
    /// mutable borrow if the epoch cannot advance or an interrupted typed
    /// mutation has left the wrapper structurally uncertain.
    pub fn world_mut(&mut self) -> &mut PhysicsWorld<D> {
        self.try_world_mut()
            .expect("physics authority mutable-world boundary unavailable")
    }

    pub fn into_world(self) -> PhysicsWorld<D> {
        self.world
    }

    /// Most recent normally completed authority-owned step in the current
    /// mutation epoch and live temporal incarnation.
    ///
    /// Returns `None` before the first authorized step, after any raw/non-step
    /// mutation boundary, and while/after an interrupted step or typed mutation
    /// until the corresponding qualified recovery rule is satisfied.
    pub fn last_authorized_step_stamp(&self) -> Option<AuthorityStepStamp> {
        if !self.temporal.has_authorized_step() {
            return None;
        }
        Some(self.current_step_stamp())
    }

    /// Execute exactly one authority-owned pure-physics step and issue its
    /// ordering stamp after normal completion.
    pub fn step_authorized(
        &mut self,
        dt: f64,
    ) -> Result<AuthorityStepStamp, PhysicsAuthorityTemporalError> {
        Self::validate_step_dt(dt)?;
        let next_step = self.temporal.begin_step()?;
        self.world.step(dt);
        self.temporal.commit_step(next_step);
        Ok(self.current_step_stamp())
    }

    /// Execute exactly one authority-owned callback-coupled physics step and
    /// issue its ordering stamp after normal completion.
    ///
    /// If the underlying step or callback panics and downstream catches the
    /// unwind, the temporal state remains step-tainted: no last stamp is exposed
    /// and another authorized step is rejected until an explicit mutation-epoch
    /// break occurs.
    pub fn step_authorized_with_callback(
        &mut self,
        dt: f64,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<AuthorityStepStamp, PhysicsAuthorityTemporalError> {
        Self::validate_step_dt(dt)?;
        let next_step = self.temporal.begin_step()?;
        self.world.step_with_callback(dt, callback);
        self.temporal.commit_step(next_step);
        Ok(self.current_step_stamp())
    }

    fn validate_step_dt(dt: f64) -> Result<(), PhysicsAuthorityTemporalError> {
        if !dt.is_finite() {
            return Err(PhysicsAuthorityTemporalError::NonFiniteDeltaTime);
        }
        if dt <= 0.0 {
            return Err(PhysicsAuthorityTemporalError::NonPositiveDeltaTime);
        }
        Ok(())
    }

    fn current_step_stamp(&self) -> AuthorityStepStamp {
        AuthorityStepStamp {
            physical_authority_id: self.physical_authority_id,
            world_generation_id: self.world_generation_id,
            temporal_incarnation_id: self.temporal_incarnation_id,
            mutation_epoch: self.temporal.mutation_epoch(),
            step_index: self.temporal.step_index(),
        }
    }

    fn preflight_nonstep_mutation_epoch(&self) -> u64 {
        self.temporal
            .next_mutation_epoch()
            .expect("physics authority typed-mutation boundary unavailable")
    }

    /// Run one already-prepared state-changing typed mutation.
    ///
    /// The temporal epoch is committed and the wrapper is marked mutation-tainted
    /// before `commit` can alter the world. The taint clears only if the entire
    /// closure returns normally. Any caught panic therefore leaves the old stamp
    /// invalid and blocks further stepping/raw mutation until reconstruction.
    fn commit_prepared_nonstep_mutation<R>(
        &mut self,
        commit: impl FnOnce(&mut PhysicsWorld<D>) -> R,
    ) -> R {
        let next_epoch = self.preflight_nonstep_mutation_epoch();
        self.temporal.begin_mutation_commit(next_epoch);
        let result = commit(&mut self.world);
        self.temporal.finish_mutation_commit();
        result
    }

    /// Bind one previously-unbound live body to a stable `NetId` within this
    /// exact physical authority and world generation.
    ///
    /// All recoverable validation occurs before the mutation boundary. A binding
    /// that actually changes identity state then advances the epoch *before* the
    /// prepared commit executes. An already-idempotent `N -> N` bind preserves
    /// the epoch and current stamp.
    pub fn bind_net_id(
        &mut self,
        handle: BodyHandle,
        net_id: NetId,
    ) -> Result<PhysicsBodySubject, NetIdentityMutationError> {
        let prepared = prepare_net_id_binding(&self.world, handle, net_id)?;
        let subject = PhysicsBodySubject::new(
            self.physical_authority_id,
            self.world_generation_id,
            net_id,
        );

        if prepared == PreparedNetIdBinding::NoChange {
            return Ok(subject);
        }

        Ok(self.commit_prepared_nonstep_mutation(|world| {
            commit_prepared_net_id_binding(world, prepared);
            assert_eq!(world.net_id_for_handle(handle), Some(net_id));
            assert_eq!(world.handle_for_net_id(net_id), Some(handle));
            subject
        }))
    }

    /// Insert a deterministic batch under this exact authority/generation.
    ///
    /// All recoverable identity validation occurs before the mutation boundary.
    /// A non-empty prepared batch advances the mutation epoch before legacy
    /// insertion can alter the world; empty batches remain true no-ops.
    pub fn add_bodies_deterministic(
        &mut self,
        bodies: Vec<(NetId, RigidBody<D>)>,
    ) -> Result<Vec<(BodyHandle, PhysicsBodySubject)>, NetIdentityMutationError> {
        let prepared = prepare_deterministic_insertion(&self.world, bodies)?;
        if prepared.is_empty() {
            return Ok(Vec::new());
        }

        let physical_authority_id = self.physical_authority_id;
        let world_generation_id = self.world_generation_id;

        Ok(self.commit_prepared_nonstep_mutation(move |world| {
            let handles = commit_prepared_deterministic_insertion(world, prepared);
            let mut bindings = Vec::with_capacity(handles.len());

            for handle in handles {
                let net_id = world
                    .net_id_for_handle(handle)
                    .expect("prepared deterministic insertion must bind every returned handle");
                bindings.push((
                    handle,
                    PhysicsBodySubject::new(
                        physical_authority_id,
                        world_generation_id,
                        net_id,
                    ),
                ));
            }

            bindings
        }))
    }

    /// Validate one exact body subject against this exact authority/generation.
    ///
    /// Validation does not trust only the `NetId -> BodyHandle` index. It scans
    /// live body state, requires exactly one matching body, and then checks both
    /// forward and reverse indexed views against that body.
    pub fn validate_subject(
        &self,
        subject: PhysicsBodySubject,
    ) -> Result<ValidatedNetBody<'_, D>, PhysicsIdentityError> {
        if subject.physical_authority_id != self.physical_authority_id {
            return Err(PhysicsIdentityError::PhysicalAuthorityMismatch {
                expected: subject.physical_authority_id,
                actual: self.physical_authority_id,
            });
        }
        if subject.world_generation_id != self.world_generation_id {
            return Err(PhysicsIdentityError::WorldGenerationMismatch {
                expected: subject.world_generation_id,
                actual: self.world_generation_id,
            });
        }

        let net_id = subject.net_id;
        let mut unique_handle = None;
        for body in &self.world.bodies {
            if body.net_id() != Some(net_id) {
                continue;
            }
            if unique_handle.replace(body.handle).is_some() {
                return Err(PhysicsIdentityError::AmbiguousNetId { net_id });
            }
        }

        let handle = unique_handle.ok_or(PhysicsIdentityError::UnknownNetId { net_id })?;
        let mapped_handle = self
            .world
            .handle_for_net_id(net_id)
            .ok_or(PhysicsIdentityError::IdentityIndexMissing { net_id, handle })?;
        if mapped_handle != handle {
            return Err(PhysicsIdentityError::IdentityIndexMismatch {
                net_id,
                body_handle: handle,
                mapped_handle,
            });
        }
        if self.world.net_id_for_handle(handle) != Some(net_id) {
            return Err(PhysicsIdentityError::ReverseIdentityMismatch { net_id, handle });
        }

        Ok(ValidatedNetBody {
            authority: self,
            subject,
            handle,
        })
    }
}

/// Non-serializable, lifetime-bound proof that one subject resolves to exactly
/// one consistently indexed live body in one exact authority/world generation.
///
/// Holding this token borrows the authority world immutably, so ordinary safe
/// code cannot obtain `&mut PhysicsAuthorityWorld` until the token is dropped.
pub struct ValidatedNetBody<'a, const D: usize> {
    authority: &'a PhysicsAuthorityWorld<D>,
    subject: PhysicsBodySubject,
    handle: BodyHandle,
}

impl<'a, const D: usize> ValidatedNetBody<'a, D> {
    pub const fn subject(&self) -> PhysicsBodySubject {
        self.subject
    }

    pub const fn net_id(&self) -> NetId {
        self.subject.net_id
    }

    /// Ephemeral runtime handle. This must not be persisted as durable identity.
    pub const fn runtime_handle(&self) -> BodyHandle {
        self.handle
    }

    pub fn body(&self) -> &RigidBody<D> {
        self.authority
            .world
            .body(self.handle)
            .expect("validated body must remain present while authority is immutably borrowed")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum PhysicsIdentityError {
    ZeroPhysicalAuthorityId,
    ZeroWorldGenerationId,
    PhysicalAuthorityMismatch {
        expected: PhysicalAuthorityId,
        actual: PhysicalAuthorityId,
    },
    WorldGenerationMismatch {
        expected: WorldGenerationId,
        actual: WorldGenerationId,
    },
    UnknownNetId {
        net_id: NetId,
    },
    AmbiguousNetId {
        net_id: NetId,
    },
    IdentityIndexMissing {
        net_id: NetId,
        handle: BodyHandle,
    },
    IdentityIndexMismatch {
        net_id: NetId,
        body_handle: BodyHandle,
        mapped_handle: BodyHandle,
    },
    ReverseIdentityMismatch {
        net_id: NetId,
        handle: BodyHandle,
    },
}

#[cfg(test)]
mod privileged_corruption_tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use symtropy_math::Point;

    #[test]
    fn direct_body_identity_corruption_leaving_stale_index_fails_closed() {
        let indexed_id = NetId(45);
        let mutated_id = NetId(46);
        let authority = PhysicalAuthorityId::new(4001).unwrap();
        let generation = WorldGenerationId::new(1).unwrap();
        let mut raw = PhysicsWorld::<3>::default();
        let handle = raw.add_sphere(Point::origin(), 0.5, 1.0);
        raw.set_net_id(handle, indexed_id);
        raw.body_mut(handle).expect("body exists").net_id = Some(mutated_id);

        let world = PhysicsAuthorityWorld::new(authority, generation, raw);
        let subject = PhysicsBodySubject::new(authority, generation, mutated_id);

        assert!(matches!(
            world.validate_subject(subject),
            Err(PhysicsIdentityError::IdentityIndexMissing {
                net_id: observed,
                handle: observed_handle,
            }) if observed == mutated_id && observed_handle == handle
        ));
    }

    #[test]
    fn duplicate_live_net_id_fails_closed_even_if_index_points_to_one_body() {
        let net_id = NetId(44);
        let authority = PhysicalAuthorityId::new(3001).unwrap();
        let generation = WorldGenerationId::new(1).unwrap();
        let mut raw = PhysicsWorld::<3>::default();
        let first = raw.add_sphere(Point::new([0.0, 0.0, 0.0]), 0.5, 1.0);
        let second = raw.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, 1.0);
        raw.set_net_id(first, net_id);
        raw.set_net_id(second, net_id);

        let world = PhysicsAuthorityWorld::new(authority, generation, raw);
        let subject = PhysicsBodySubject::new(authority, generation, net_id);

        assert!(matches!(
            world.validate_subject(subject),
            Err(PhysicsIdentityError::AmbiguousNetId { net_id: observed }) if observed == net_id
        ));
    }

    #[test]
    fn caught_prepared_mutation_panic_taints_wrapper_until_reconstruction() {
        let authority_id = PhysicalAuthorityId::new(5001).unwrap();
        let generation_id = WorldGenerationId::new(1).unwrap();
        let mut authority =
            PhysicsAuthorityWorld::new(authority_id, generation_id, PhysicsWorld::<3>::default());
        let before = authority.step_authorized(1.0 / 64.0).unwrap();

        let outcome = catch_unwind(AssertUnwindSafe(|| {
            authority.commit_prepared_nonstep_mutation(|world| {
                world.gravity[0] = 42.0;
                panic!("injected prepared-mutation failure");
            });
        }));

        assert!(outcome.is_err());
        assert_eq!(authority.world().gravity[0], 42.0);
        assert_eq!(
            authority.temporal.mutation_epoch(),
            before.mutation_epoch() + 1
        );
        assert_eq!(authority.last_authorized_step_stamp(), None);
        assert_eq!(
            authority.step_authorized(1.0 / 64.0),
            Err(PhysicsAuthorityTemporalError::InterruptedMutationTainted)
        );
        assert!(matches!(
            authority.try_world_mut(),
            Err(PhysicsAuthorityTemporalError::InterruptedMutationTainted)
        ));
    }
}
