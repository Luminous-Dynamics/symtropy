// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Execution-bound exact-`dt` receipts for qualified physics steps.
//!
//! This profile owns the sealed evidence facade and exposes no mutable projection
//! to it. Every successful qualified step performed through this wrapper returns
//! one opaque receipt binding the committed strong step stamp to the exact
//! positive finite binary64 `dt` value passed to the physics engine.

use crate::authority_namespace::{
    LocalNamespacePhysicsAuthorityWorld, LocalQualifiedValidatedNetBody,
};
use crate::evidence_authority::{
    LocalEvidenceAuthorityError, LocalEvidenceAuthoritySealFailure,
    LocalEvidencePhysicsAuthorityWorld, LocalQualifiedAuthorityStepStamp,
    LocalTaintedEvidenceAuthority, LocalTemporalIncarnationId,
};
use crate::identity_authority::{
    PhysicalAuthorityId, PhysicsBodySubject, PhysicsIdentityError, WorldGenerationId,
};
use crate::world::PhysicsCallback;

/// Opaque receipt for one normally completed qualified physics step.
///
/// The receipt is intentionally non-Clone, non-Copy, and non-Serde-authoritative.
/// `dt_bits` is the exact IEEE-754 binary64 representation of the same `dt`
/// value passed to the underlying qualified step call.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalQualifiedStepExecutionReceipt {
    stamp: LocalQualifiedAuthorityStepStamp,
    dt_bits: u64,
}

impl LocalQualifiedStepExecutionReceipt {
    pub const fn stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        self.stamp
    }

    pub const fn dt_bits(&self) -> u64 {
        self.dt_bits
    }

    pub fn executed_dt(&self) -> f64 {
        f64::from_bits(self.dt_bits)
    }
}

/// Sealed evidence authority profile in which every successful strong step is
/// execution-receipted.
///
/// There is deliberately no constructor from an already-running
/// `LocalEvidencePhysicsAuthorityWorld`: v0.1 begins at a fresh seal so every
/// qualified step in this temporal incarnation is forced through a receipted
/// stepping path.
pub struct LocalReceiptedEvidencePhysicsAuthorityWorld<const D: usize> {
    evidence: LocalEvidencePhysicsAuthorityWorld<D>,
}

impl<const D: usize> LocalReceiptedEvidencePhysicsAuthorityWorld<D> {
    /// Begin a fresh temporal incarnation whose only public qualified step paths
    /// return exact execution receipts.
    pub fn seal(
        namespace: LocalNamespacePhysicsAuthorityWorld<D>,
    ) -> Result<Self, LocalEvidenceAuthoritySealFailure<D>> {
        LocalEvidencePhysicsAuthorityWorld::seal(namespace).map(|evidence| Self { evidence })
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.evidence.physical_authority_id()
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.evidence.world_generation_id()
    }

    pub const fn temporal_incarnation_id(&self) -> LocalTemporalIncarnationId {
        self.evidence.temporal_incarnation_id()
    }

    /// Immutable projection for PHYS-OBS-05 capture and diagnostics.
    ///
    /// No mutable projection or conversion to a live unwrapped evidence facade
    /// is provided, so a qualified step cannot bypass receipt issuance while
    /// this temporal incarnation remains in the receipted profile.
    pub const fn evidence_authority(&self) -> &LocalEvidencePhysicsAuthorityWorld<D> {
        &self.evidence
    }

    pub fn validate_subject(
        &self,
        subject: PhysicsBodySubject,
    ) -> Result<LocalQualifiedValidatedNetBody<'_, D>, PhysicsIdentityError> {
        self.evidence.validate_subject(subject)
    }

    pub fn last_qualified_step_stamp(&self) -> Option<LocalQualifiedAuthorityStepStamp> {
        self.evidence.last_qualified_step_stamp()
    }

    /// Execute one pure-physics strong step and return its exact execution receipt
    /// only after normal successful completion.
    pub fn step_authorized(
        &mut self,
        dt: f64,
    ) -> Result<LocalQualifiedStepExecutionReceipt, LocalEvidenceAuthorityError> {
        let stamp = self.evidence.step_authorized(dt)?;
        Ok(LocalQualifiedStepExecutionReceipt {
            stamp,
            dt_bits: dt.to_bits(),
        })
    }

    /// Execute one callback-coupled strong step and return its exact execution
    /// receipt only after normal successful completion.
    pub fn step_authorized_with_callback(
        &mut self,
        dt: f64,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<LocalQualifiedStepExecutionReceipt, LocalEvidenceAuthorityError> {
        let stamp = self
            .evidence
            .step_authorized_with_callback(dt, callback)?;
        Ok(LocalQualifiedStepExecutionReceipt {
            stamp,
            dt_bits: dt.to_bits(),
        })
    }

    /// Leave the receipted evidence profile only by leaving this temporal
    /// incarnation's strong stepping surface altogether.
    ///
    /// Clean state returns the namespace boundary. Tainted state preserves #1086
    /// quarantine semantics and cannot be directly resealed.
    pub fn into_namespace(
        self,
    ) -> Result<LocalNamespacePhysicsAuthorityWorld<D>, LocalTaintedEvidenceAuthority<D>> {
        self.evidence.into_namespace()
    }
}
