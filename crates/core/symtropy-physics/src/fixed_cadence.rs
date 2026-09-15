// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Precommitted exact-binary64 fixed-cadence evidence for qualified physics steps.
//!
//! The cadence value is ordinary immutable configuration. Strong cadence
//! provenance comes only from sealing a fresh receipted temporal incarnation
//! with that value before stepping and minting a distinct fixed-cadence receipt.

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
use crate::timed_evidence::{
    LocalQualifiedStepExecutionReceipt, LocalReceiptedEvidencePhysicsAuthorityWorld,
};
use crate::world::PhysicsCallback;

const BINARY64_FRACTION_MASK: u64 = (1_u64 << 52) - 1;
const BINARY64_EXPONENT_MASK: u64 = 0x7ff;
const BINARY64_HIDDEN_BIT: u64 = 1_u64 << 52;

/// Immutable precommitted simulation cadence identified by the exact positive
/// finite binary64 bits that will be passed to the integrator.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct LocalFixedSimulationCadence {
    dt_bits: u64,
}

impl LocalFixedSimulationCadence {
    pub fn new(dt: f64) -> Result<Self, LocalFixedCadenceError> {
        if !dt.is_finite() {
            return Err(LocalFixedCadenceError::NonFiniteDeltaTime);
        }
        if dt <= 0.0 {
            return Err(LocalFixedCadenceError::NonPositiveDeltaTime);
        }
        Ok(Self {
            dt_bits: dt.to_bits(),
        })
    }

    pub const fn dt_bits(self) -> u64 {
        self.dt_bits
    }

    pub fn executed_dt(self) -> f64 {
        f64::from_bits(self.dt_bits)
    }

    /// Lossless exact dyadic interpretation of the configured binary64 value.
    pub fn exact_dyadic(self) -> ExactDyadicCadence {
        exact_dyadic_from_positive_finite_bits(self.dt_bits)
    }
}

/// Canonical exact value `significand * 2^exponent2` for one positive finite
/// binary64 cadence. `significand` is odd, so powers of two have one canonical
/// representation.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct ExactDyadicCadence {
    significand: u64,
    exponent2: i32,
}

impl ExactDyadicCadence {
    pub const fn significand(self) -> u64 {
        self.significand
    }

    pub const fn exponent2(self) -> i32 {
        self.exponent2
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum LocalFixedCadenceError {
    NonFiniteDeltaTime,
    NonPositiveDeltaTime,
}

/// Opaque proof that one normally completed strong step was executed inside a
/// temporal incarnation sealed under one precommitted fixed cadence.
///
/// This type is deliberately stronger than the generic PHYS-OBS-07 receipt and
/// cannot be constructed from a generic receipt plus a cadence after the fact.
#[derive(Debug, PartialEq, Eq)]
pub struct LocalFixedCadenceStepReceipt {
    execution: LocalQualifiedStepExecutionReceipt,
    cadence: LocalFixedSimulationCadence,
}

impl LocalFixedCadenceStepReceipt {
    pub const fn stamp(&self) -> LocalQualifiedAuthorityStepStamp {
        self.execution.stamp()
    }

    pub const fn dt_bits(&self) -> u64 {
        self.execution.dt_bits()
    }

    pub fn executed_dt(&self) -> f64 {
        self.execution.executed_dt()
    }

    pub const fn cadence(&self) -> LocalFixedSimulationCadence {
        self.cadence
    }

    pub fn exact_dyadic(&self) -> ExactDyadicCadence {
        self.cadence.exact_dyadic()
    }

    /// Read-only downward projection to the weaker generic execution receipt.
    /// No reverse promotion is provided.
    pub const fn execution_receipt(&self) -> &LocalQualifiedStepExecutionReceipt {
        &self.execution
    }
}

/// Fresh temporal evidence profile in which every successful strong step uses
/// one exact cadence precommitted before the incarnation began.
pub struct LocalFixedCadenceEvidencePhysicsAuthorityWorld<const D: usize> {
    receipted: LocalReceiptedEvidencePhysicsAuthorityWorld<D>,
    cadence: LocalFixedSimulationCadence,
}

impl<const D: usize> LocalFixedCadenceEvidencePhysicsAuthorityWorld<D> {
    /// Seal a fresh receipted temporal incarnation under one already-validated
    /// exact fixed cadence.
    pub fn seal(
        namespace: LocalNamespacePhysicsAuthorityWorld<D>,
        cadence: LocalFixedSimulationCadence,
    ) -> Result<Self, LocalEvidenceAuthoritySealFailure<D>> {
        LocalReceiptedEvidencePhysicsAuthorityWorld::seal(namespace)
            .map(|receipted| Self { receipted, cadence })
    }

    pub const fn cadence(&self) -> LocalFixedSimulationCadence {
        self.cadence
    }

    pub const fn physical_authority_id(&self) -> PhysicalAuthorityId {
        self.receipted.physical_authority_id()
    }

    pub const fn world_generation_id(&self) -> WorldGenerationId {
        self.receipted.world_generation_id()
    }

    pub const fn temporal_incarnation_id(&self) -> LocalTemporalIncarnationId {
        self.receipted.temporal_incarnation_id()
    }

    /// Immutable projection for endpoint capture and other weaker observations.
    pub const fn evidence_authority(&self) -> &LocalEvidencePhysicsAuthorityWorld<D> {
        self.receipted.evidence_authority()
    }

    pub fn validate_subject(
        &self,
        subject: PhysicsBodySubject,
    ) -> Result<LocalQualifiedValidatedNetBody<'_, D>, PhysicsIdentityError> {
        self.receipted.validate_subject(subject)
    }

    pub fn last_qualified_step_stamp(&self) -> Option<LocalQualifiedAuthorityStepStamp> {
        self.receipted.last_qualified_step_stamp()
    }

    /// Execute exactly one pure-physics step at the precommitted cadence.
    pub fn step_authorized_fixed(
        &mut self,
    ) -> Result<LocalFixedCadenceStepReceipt, LocalEvidenceAuthorityError> {
        let execution = self.receipted.step_authorized(self.cadence.executed_dt())?;
        Ok(LocalFixedCadenceStepReceipt {
            execution,
            cadence: self.cadence,
        })
    }

    /// Execute exactly one callback-coupled step at the precommitted cadence.
    pub fn step_authorized_fixed_with_callback(
        &mut self,
        callback: &mut dyn PhysicsCallback<D>,
    ) -> Result<LocalFixedCadenceStepReceipt, LocalEvidenceAuthorityError> {
        let execution = self
            .receipted
            .step_authorized_with_callback(self.cadence.executed_dt(), callback)?;
        Ok(LocalFixedCadenceStepReceipt {
            execution,
            cadence: self.cadence,
        })
    }

    /// Leave the fixed profile only by returning to the namespace boundary.
    /// A later seal, even with equal cadence bits, receives a fresh temporal
    /// incarnation; a changed cadence therefore cannot be hidden mid-lineage.
    pub fn into_namespace(
        self,
    ) -> Result<LocalNamespacePhysicsAuthorityWorld<D>, LocalTaintedEvidenceAuthority<D>> {
        self.receipted.into_namespace()
    }
}

fn exact_dyadic_from_positive_finite_bits(bits: u64) -> ExactDyadicCadence {
    let raw_exponent = ((bits >> 52) & BINARY64_EXPONENT_MASK) as i32;
    let fraction = bits & BINARY64_FRACTION_MASK;

    let (mut significand, mut exponent2) = if raw_exponent == 0 {
        (fraction, -1074)
    } else {
        (
            BINARY64_HIDDEN_BIT | fraction,
            raw_exponent - 1023 - 52,
        )
    };

    debug_assert_ne!(significand, 0);
    while significand & 1 == 0 {
        significand >>= 1;
        exponent2 += 1;
    }

    ExactDyadicCadence {
        significand,
        exponent2,
    }
}
