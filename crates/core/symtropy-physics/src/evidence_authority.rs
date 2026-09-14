// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Sealed process-local temporal evidence authority.
//!
//! This layer consumes a namespace-qualified authority wrapper and exposes a
//! deliberately smaller surface: authorized stepping, immutable qualified
//! subject validation, and consuming downgrade. Non-step mutation requires
//! leaving this facade and resealing under a fresh temporal incarnation.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::authority_namespace::{
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedValidatedNetBody,
};
use crate::identity_authority::{
    PhysicalAuthorityId, PhysicsAuthorityTemporalError, PhysicsAuthorityWorld, PhysicsBodySubject,
    PhysicsIdentityError, WorldGenerationId,
};
use crate::world::PhysicsCallback;

static NEXT_LOCAL_TEMPORAL_INCARCATION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LocalTemporalIncarnationId(u64);

impl LocalTemporalIncarnationId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LocalQualifiedAuthorityStepStamp {
    physical_authority_id: PhysicalAuthorityId,
    world_generation_id: WorldGenerationId,
    temporal_incarnation_id: LocalTemporalIncarnationId,
    step_index: u64,
}

impl LocalQualifiedAuthorityStepStamp {
    pub const fn physical_authority_id(self) -> PhysicalAuthorityId {
        self.physical_authority_id
    }

    pub const fn world_generation_id(self) -> WorldGenerationId {
        self.world_generation_id
    }

    pub const fn temporal_incarnation_id(self) -> LocalTemporalIncarnationId {
        self.temporal_incarnation_id
    }

    pub const fn step_index(self) -> u64 {
        self.step_index
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalEvidenceAuthorityError {
    TemporalIncarnationExhausted,
    NonFiniteDeltaTime,
    NonPositiveDeltaTime,
    StepIndexExhausted,
    InterruptedStepTainted,
    InnerTemporal(PhysicsAuthorityTemporalError),
}

pub struct LocalEvidenceAuthoritySealFailure<const D: usize> {
    namespace: LocalNamespacePhysicsAuthorityWorld<D>,
    error: LocalEvidenceAuthorityError,
}

impl<const D: usize> LocalEvidenceAuthoritySealFailure<D> {
    pub const fn error(&self) -> LocalEvidenceAuthorityError {
        self.error
    }

    pub fn into_namespace(self) -> LocalNamespacePhysicsAuthorityWorld<D> {
        self.namespace
    }
}

/// Quarantine returned when a sealed step was interrupted or otherwise tainted
/// after outer execution began.
///
/// This type deliberately provides no conversion back to
/// `LocalNamespacePhysicsAuthorityWorld`: a caught partial step may have changed
/// world state before unwinding, so a fresh temporal ID alone is not a recovery
/// theorem. The caller may consume this into the weaker legacy authority and
/// later use a separately qualified checked-recovery/adoption path.
pub struct LocalTaintedEvidenceAuthority<const D: usize> {
    namespace: LocalNamespacePhysicsAuthorityWorld<D>,
}

impl<const D: usize> LocalTaintedEvidenceAuthority<D> {
    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.namespace.physical_authority_id()
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.namespace.world_generation_id()
    }

    pub fn into_unqualified(self) -> PhysicsAuthorityWorld<D> {
        self.namespace.into_unqualified()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct LocalEvidenceTemporalState {
    step_index: u64,
    has_authorized_step: bool,
    step_tainted: bool,
}

impl LocalEvidenceTemporalState {
    const fn new() -> Self {
        Self {
            step_index: 0,
            has_authorized_step: false,
            step_tainted: false,
        }
    }

    fn begin_step(&mut self) -> Result<u64, LocalEvidenceAuthorityError> {
        if self.step_tainted {
            return Err(LocalEvidenceAuthorityError::InterruptedStepTainted);
        }
        let next = self
            .step_index
            .checked_add(1)
            .ok_or(LocalEvidenceAuthorityError::StepIndexExhausted)?;
        self.step_tainted = true;
        Ok(next)
    }

    fn commit_step(&mut self, next: u64) {
        debug_assert_eq!(self.step_index.checked_add(1), Some(next));
        self.step_index = next;
        self.has_authorized_step = true;
        self.step_tainted = false;
    }

    const fn has_current_step(self) -> bool {
        self.has_authorized_step && !self.step_tainted
    }
}

/// Evidence-ready authority facade with no public non-step mutable surface.
///
/// Setup/editor/identity mutation requires consuming a clean facade through
/// [`Self::into_namespace`]. A tainted facade is quarantined instead of being
/// eligible for reseal. Clean resealing always creates a fresh temporal
/// incarnation, so mutation cannot be hidden inside one evidence lineage.
pub struct LocalEvidencePhysicsAuthorityWorld<const D: usize> {
    namespace: LocalNamespacePhysicsAuthorityWorld<D>,
    temporal_incarnation_id: LocalTemporalIncarnationId,
    temporal: LocalEvidenceTemporalState,
}

impl<const D: usize> LocalEvidencePhysicsAuthorityWorld<D> {
    pub fn seal(
        mut namespace: LocalNamespacePhysicsAuthorityWorld<D>,
    ) -> Result<Self, LocalEvidenceAuthoritySealFailure<D>> {
        let temporal_incarnation_id = match mint_temporal_incarnation() {
            Ok(id) => id,
            Err(error) => return Err(LocalEvidenceAuthoritySealFailure { namespace, error }),
        };

        if let Err(error) = namespace.authority_world_mut().try_world_mut().map(|_| ()) {
            return Err(LocalEvidenceAuthoritySealFailure {
                namespace,
                error: LocalEvidenceAuthorityError::InnerTemporal(error),
            });
        }

        Ok(Self {
            namespace,
            temporal_incarnation_id,
            temporal: LocalEvidenceTemporalState::new(),
        })
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.namespace.physical_authority_id()
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.namespace.world_generation_id()
    }

    pub const fn temporal_incarnation_id(&self) -> LocalTemporalIncarnationId {
        self.temporal_incarnation_id
    }

    pub fn authority_world(&self) -> &PhysicsAuthorityWorld<D> {
        self.namespace.authority_world()
    }

    pub fn validate_subject(
        &self,
        subject: PhysicsBodySubject,
    ) -> Result<LocalQualifiedValidatedNetBody<'_, D>, PhysicsIdentityError> {
        self.namespace.validate_subject(subject)
    }

    pub fn last_qualified_step_stamp(&self) -> Option<LocalQualifiedAuthorityStepStamp> {
        if !self.temporal.has_current_step() {
            return None;
        }
        Some(self.current_stamp())
    }

    pub fn step_authorized(
        &mut self,
        dt: f64,
    ) -> Result<LocalQualifiedAuthorityStepStamp, LocalEvidenceAuthorityError> {
        Self::validate_dt(dt)?;
        let next = self.temporal.begin_step()?;
        if let Err(error) = self.namespace.authority_world_mut().step_authorized(dt) {
            return Err(LocalEvidenceAuthorityError::InnerTemporal(error));
        }
        self.temporal.commit_step(next);
        Ok(self.current_stamp())
    }

    pub fn step_authorized_with_callback(
        &mut self,
        dt: f64,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<LocalQualifiedAuthorityStepStamp, LocalEvidenceAuthorityError> {
        Self::validate_dt(dt)?;
        let next = self.temporal.begin_step()?;
        if let Err(error) = self
            .namespace
            .authority_world_mut()
            .step_authorized_with_callback(dt, callback)
        {
            return Err(LocalEvidenceAuthorityError::InnerTemporal(error));
        }
        self.temporal.commit_step(next);
        Ok(self.current_stamp())
    }

    /// Consume evidence authority. Clean state returns the namespace-qualified
    /// mutation/setup surface; tainted state is quarantined and cannot be resealed
    /// without a separate recovery/adoption theorem.
    pub fn into_namespace(
        self,
    ) -> Result<LocalNamespacePhysicsAuthorityWorld<D>, LocalTaintedEvidenceAuthority<D>> {
        if self.temporal.step_tainted {
            return Err(LocalTaintedEvidenceAuthority {
                namespace: self.namespace,
            });
        }
        Ok(self.namespace)
    }

    fn validate_dt(dt: f64) -> Result<(), LocalEvidenceAuthorityError> {
        if !dt.is_finite() {
            return Err(LocalEvidenceAuthorityError::NonFiniteDeltaTime);
        }
        if dt <= 0.0 {
            return Err(LocalEvidenceAuthorityError::NonPositiveDeltaTime);
        }
        Ok(())
    }

    fn current_stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        LocalQualifiedAuthorityStepStamp {
            physical_authority_id: self.physical_authority_id(),
            world_generation_id: self.world_generation_id(),
            temporal_incarnation_id: self.temporal_incarnation_id,
            step_index: self.temporal.step_index,
        }
    }
}

fn mint_temporal_incarnation() -> Result<LocalTemporalIncarnationId, LocalEvidenceAuthorityError> {
    let raw = NEXT_LOCAL_TEMPORAL_INCARCATION_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        })
        .map_err(|_| LocalEvidenceAuthorityError::TemporalIncarnationExhausted)?;
    Ok(LocalTemporalIncarnationId(raw))
}

#[cfg(test)]
mod private_tests {
    use super::*;

    #[test]
    fn exhausted_outer_step_counter_fails_before_taint() {
        let mut state = LocalEvidenceTemporalState {
            step_index: u64::MAX,
            has_authorized_step: true,
            step_tainted: false,
        };
        assert_eq!(
            state.begin_step(),
            Err(LocalEvidenceAuthorityError::StepIndexExhausted)
        );
        assert!(!state.step_tainted);
        assert!(state.has_authorized_step);
    }

    #[test]
    fn interrupted_outer_step_blocks_future_begin() {
        let mut state = LocalEvidenceTemporalState::new();
        assert_eq!(state.begin_step().unwrap(), 1);
        assert_eq!(
            state.begin_step(),
            Err(LocalEvidenceAuthorityError::InterruptedStepTainted)
        );
        assert!(!state.has_current_step());
    }
}
