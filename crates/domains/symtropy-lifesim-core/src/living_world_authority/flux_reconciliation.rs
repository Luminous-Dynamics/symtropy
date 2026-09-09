// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical reconciliation of conserved fluxes across fidelity/cadence boundaries.
//!
//! A coarse/reference model and a refined/fast model may both estimate the same
//! physical transfer over one synchronization interval. Those estimates are not
//! independent ecological inputs or outputs. This module binds both sides to one
//! registered boundary, one logical source snapshot, and one canonical interval;
//! accumulates subcycled contributions deterministically; verifies complete
//! non-overlapping temporal coverage; and mints a settlement certificate only
//! when the registered per-quantity mismatch tolerance is satisfied.
//!
//! V0 deliberately does not invent a numerical correction algorithm. Policy must
//! explicitly name which side supplies the canonical settlement amount when the
//! two integrated estimates differ within tolerance. Larger disagreement remains
//! visible and uncertified.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::conservation::ConservedQuantity;

use super::spatiotemporal_information::CanonicalTick;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxReconciliationRegistryKey {
    id: u128,
    version: u32,
}

impl FluxReconciliationRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxBoundaryKey {
    id: u128,
    version: u32,
}

impl FluxBoundaryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxRegisterAuthorityKey {
    id: u128,
    version: u32,
}

impl FluxRegisterAuthorityKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxRegisterRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxSourceKey {
    id: u128,
    version: u32,
}

impl FluxSourceKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Opaque logical snapshot identity. Higher authority layers authenticate this
/// token; this low-level structural layer binds it and refuses to mix tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxSnapshotToken(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxSourceRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxSourceAuthorityStamp {
    source: FluxSourceKey,
    revision: FluxSourceRevision,
    snapshot: FluxSnapshotToken,
}

impl FluxSourceAuthorityStamp {
    pub const fn new(
        source: FluxSourceKey,
        revision: FluxSourceRevision,
        snapshot: FluxSnapshotToken,
    ) -> Self {
        Self {
            source,
            revision,
            snapshot,
        }
    }

    pub const fn source(self) -> FluxSourceKey {
        self.source
    }

    pub const fn revision(self) -> FluxSourceRevision {
        self.revision
    }

    pub const fn snapshot(self) -> FluxSnapshotToken {
        self.snapshot
    }
}

/// Canonical interval semantics are `(start_exclusive, end_inclusive]`, matching
/// the existing ecological cadence catch-up convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SynchronizationInterval {
    start_exclusive: CanonicalTick,
    end_inclusive: CanonicalTick,
}

impl SynchronizationInterval {
    pub fn new(
        start_exclusive: CanonicalTick,
        end_inclusive: CanonicalTick,
    ) -> Result<Self, FluxReconciliationError> {
        if end_inclusive.0 <= start_exclusive.0 {
            return Err(FluxReconciliationError::InvalidSynchronizationInterval {
                start_exclusive,
                end_inclusive,
            });
        }
        Ok(Self {
            start_exclusive,
            end_inclusive,
        })
    }

    pub const fn start_exclusive(self) -> CanonicalTick {
        self.start_exclusive
    }

    pub const fn end_inclusive(self) -> CanonicalTick {
        self.end_inclusive
    }

    pub const fn contains(self, inner: Self) -> bool {
        inner.start_exclusive.0 >= self.start_exclusive.0
            && inner.end_inclusive.0 <= self.end_inclusive.0
            && inner.end_inclusive.0 > inner.start_exclusive.0
    }
}

/// Finite signed time-integrated flux. Sign is interpreted relative to the
/// registered boundary orientation supplied by higher orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegratedFlux(u64);

impl IntegratedFlux {
    pub fn new(value: f64) -> Result<Self, FluxReconciliationError> {
        if !value.is_finite() {
            return Err(FluxReconciliationError::NonFiniteFlux(value));
        }
        let normalized = if value == 0.0 { 0.0 } else { value };
        Ok(Self(normalized.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbsoluteFluxTolerance(u64);

impl AbsoluteFluxTolerance {
    pub fn new(value: f64) -> Result<Self, FluxReconciliationError> {
        if !value.is_finite() {
            return Err(FluxReconciliationError::NonFiniteTolerance(value));
        }
        if value < 0.0 {
            return Err(FluxReconciliationError::NegativeTolerance(value));
        }
        let normalized = if value == 0.0 { 0.0 } else { value };
        Ok(Self(normalized.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FluxSide {
    Reference,
    Refined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalSettlementSide {
    Reference,
    Refined,
}

impl CanonicalSettlementSide {
    const fn as_flux_side(self) -> FluxSide {
        match self {
            Self::Reference => FluxSide::Reference,
            Self::Refined => FluxSide::Refined,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FluxReconciliationRequirement {
    quantity: ConservedQuantity,
    tolerance: AbsoluteFluxTolerance,
    settlement_side: CanonicalSettlementSide,
}

impl FluxReconciliationRequirement {
    pub const fn new(
        quantity: ConservedQuantity,
        tolerance: AbsoluteFluxTolerance,
        settlement_side: CanonicalSettlementSide,
    ) -> Self {
        Self {
            quantity,
            tolerance,
            settlement_side,
        }
    }

    pub const fn quantity(self) -> ConservedQuantity {
        self.quantity
    }

    pub const fn tolerance(self) -> AbsoluteFluxTolerance {
        self.tolerance
    }

    pub const fn settlement_side(self) -> CanonicalSettlementSide {
        self.settlement_side
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluxBoundaryDefinition {
    key: FluxBoundaryKey,
    reference_source: FluxSourceKey,
    refined_source: FluxSourceKey,
    requirements: BTreeMap<ConservedQuantity, FluxReconciliationRequirement>,
}

impl FluxBoundaryDefinition {
    pub fn new(
        key: FluxBoundaryKey,
        reference_source: FluxSourceKey,
        refined_source: FluxSourceKey,
        requirements: impl IntoIterator<Item = FluxReconciliationRequirement>,
    ) -> Result<Self, FluxReconciliationError> {
        if reference_source == refined_source {
            return Err(FluxReconciliationError::IdenticalBoundarySources {
                boundary: key,
                source: reference_source,
            });
        }
        let mut requirement_map = BTreeMap::new();
        for requirement in requirements {
            let quantity = requirement.quantity();
            if requirement_map.insert(quantity, requirement).is_some() {
                return Err(FluxReconciliationError::DuplicateQuantityRequirement {
                    boundary: key,
                    quantity,
                });
            }
        }
        if requirement_map.is_empty() {
            return Err(FluxReconciliationError::EmptyBoundaryRequirements { boundary: key });
        }
        Ok(Self {
            key,
            reference_source,
            refined_source,
            requirements: requirement_map,
        })
    }

    pub const fn key(&self) -> FluxBoundaryKey {
        self.key
    }

    pub const fn reference_source(&self) -> FluxSourceKey {
        self.reference_source
    }

    pub const fn refined_source(&self) -> FluxSourceKey {
        self.refined_source
    }

    pub fn requirements(
        &self,
    ) -> &BTreeMap<ConservedQuantity, FluxReconciliationRequirement> {
        &self.requirements
    }
}

/// Exact policy identity stores the canonical ordered boundary corpus itself.
/// Equality is therefore collision-free with respect to these V0 semantics and
/// independent of builder insertion order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluxReconciliationAuthorityStamp {
    key: FluxReconciliationRegistryKey,
    boundaries: BTreeMap<FluxBoundaryKey, FluxBoundaryDefinition>,
}

impl FluxReconciliationAuthorityStamp {
    pub const fn key(&self) -> FluxReconciliationRegistryKey {
        self.key
    }

    pub fn boundaries(&self) -> &BTreeMap<FluxBoundaryKey, FluxBoundaryDefinition> {
        &self.boundaries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluxReconciliationRegistryBuilder {
    key: FluxReconciliationRegistryKey,
    boundaries: BTreeMap<FluxBoundaryKey, FluxBoundaryDefinition>,
}

impl FluxReconciliationRegistryBuilder {
    pub const fn new(key: FluxReconciliationRegistryKey) -> Self {
        Self {
            key,
            boundaries: BTreeMap::new(),
        }
    }

    pub fn register_boundary(
        &mut self,
        boundary: FluxBoundaryDefinition,
    ) -> Result<(), FluxReconciliationError> {
        let key = boundary.key();
        if let Some(existing) = self.boundaries.get(&key) {
            if existing == &boundary {
                return Ok(());
            }
            return Err(FluxReconciliationError::ConflictingBoundaryRegistration { boundary: key });
        }
        self.boundaries.insert(key, boundary);
        Ok(())
    }

    pub fn seal(self) -> FluxReconciliationRegistry {
        let authority = FluxReconciliationAuthorityStamp {
            key: self.key,
            boundaries: self.boundaries.clone(),
        };
        FluxReconciliationRegistry {
            authority,
            boundaries: self.boundaries,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluxReconciliationRegistry {
    authority: FluxReconciliationAuthorityStamp,
    boundaries: BTreeMap<FluxBoundaryKey, FluxBoundaryDefinition>,
}

impl FluxReconciliationRegistry {
    pub const fn authority_stamp(&self) -> &FluxReconciliationAuthorityStamp {
        &self.authority
    }

    pub fn boundary(&self, key: FluxBoundaryKey) -> Option<&FluxBoundaryDefinition> {
        self.boundaries.get(&key)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxContributionKey {
    id: u128,
    version: u32,
}

impl FluxContributionKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxContributionRequestId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxContribution {
    key: FluxContributionKey,
    side: FluxSide,
    quantity: ConservedQuantity,
    interval: SynchronizationInterval,
    amount: IntegratedFlux,
    source_authority: FluxSourceAuthorityStamp,
}

impl FluxContribution {
    pub const fn new(
        key: FluxContributionKey,
        side: FluxSide,
        quantity: ConservedQuantity,
        interval: SynchronizationInterval,
        amount: IntegratedFlux,
        source_authority: FluxSourceAuthorityStamp,
    ) -> Self {
        Self {
            key,
            side,
            quantity,
            interval,
            amount,
            source_authority,
        }
    }

    pub const fn key(self) -> FluxContributionKey {
        self.key
    }

    pub const fn side(self) -> FluxSide {
        self.side
    }

    pub const fn quantity(self) -> ConservedQuantity {
        self.quantity
    }

    pub const fn interval(self) -> SynchronizationInterval {
        self.interval
    }

    pub const fn amount(self) -> IntegratedFlux {
        self.amount
    }

    pub const fn source_authority(self) -> FluxSourceAuthorityStamp {
        self.source_authority
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxContributionRequest {
    id: FluxContributionRequestId,
    contribution: FluxContribution,
}

impl FluxContributionRequest {
    pub const fn new(id: FluxContributionRequestId, contribution: FluxContribution) -> Self {
        Self { id, contribution }
    }

    pub const fn id(self) -> FluxContributionRequestId {
        self.id
    }

    pub const fn contribution(self) -> FluxContribution {
        self.contribution
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxContributionReceipt {
    request: FluxContributionRequestId,
    contribution: FluxContributionKey,
    resulting_revision: FluxRegisterRevision,
}

impl FluxContributionReceipt {
    pub const fn request(self) -> FluxContributionRequestId {
        self.request
    }

    pub const fn contribution(self) -> FluxContributionKey {
        self.contribution
    }

    pub const fn resulting_revision(self) -> FluxRegisterRevision {
        self.resulting_revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FluxFinalizeRequestId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ReconciledFluxSettlementId {
    register: FluxRegisterAuthorityKey,
    revision: FluxRegisterRevision,
}

impl ReconciledFluxSettlementId {
    pub const fn register(self) -> FluxRegisterAuthorityKey {
        self.register
    }

    pub const fn revision(self) -> FluxRegisterRevision {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluxReconciliationBalance {
    quantity: ConservedQuantity,
    reference_total: IntegratedFlux,
    refined_total: IntegratedFlux,
    mismatch: IntegratedFlux,
    tolerance: AbsoluteFluxTolerance,
    settlement_side: CanonicalSettlementSide,
}

impl FluxReconciliationBalance {
    pub const fn quantity(self) -> ConservedQuantity {
        self.quantity
    }

    pub const fn reference_total(self) -> IntegratedFlux {
        self.reference_total
    }

    pub const fn refined_total(self) -> IntegratedFlux {
        self.refined_total
    }

    pub const fn mismatch(self) -> IntegratedFlux {
        self.mismatch
    }

    pub const fn tolerance(self) -> AbsoluteFluxTolerance {
        self.tolerance
    }

    pub const fn settlement_side(self) -> CanonicalSettlementSide {
        self.settlement_side
    }

    pub fn within_tolerance(self) -> bool {
        self.mismatch.get().abs() <= self.tolerance.get()
    }

    pub const fn settlement_amount(self) -> IntegratedFlux {
        match self.settlement_side {
            CanonicalSettlementSide::Reference => self.reference_total,
            CanonicalSettlementSide::Refined => self.refined_total,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FluxMismatchReport {
    balances: BTreeMap<ConservedQuantity, FluxReconciliationBalance>,
}

impl FluxMismatchReport {
    pub fn balances(&self) -> &BTreeMap<ConservedQuantity, FluxReconciliationBalance> {
        &self.balances
    }

    pub fn balance(&self, quantity: ConservedQuantity) -> Option<FluxReconciliationBalance> {
        self.balances.get(&quantity).copied()
    }

    pub fn within_tolerance(&self) -> bool {
        self.balances.values().all(|balance| balance.within_tolerance())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconciledFlux {
    quantity: ConservedQuantity,
    amount: IntegratedFlux,
    settlement_side: CanonicalSettlementSide,
    reference_total: IntegratedFlux,
    refined_total: IntegratedFlux,
}

impl ReconciledFlux {
    pub const fn quantity(self) -> ConservedQuantity {
        self.quantity
    }

    pub const fn amount(self) -> IntegratedFlux {
        self.amount
    }

    pub const fn settlement_side(self) -> CanonicalSettlementSide {
        self.settlement_side
    }

    pub const fn reference_total(self) -> IntegratedFlux {
        self.reference_total
    }

    pub const fn refined_total(self) -> IntegratedFlux {
        self.refined_total
    }
}

/// Stable settlement input. It does not mutate an ecological ledger itself;
/// the eventual boundary owner must consume this receipt idempotently so the
/// reconciled transfer is applied exactly once to canonical state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciledFluxCertificate {
    id: ReconciledFluxSettlementId,
    request: FluxFinalizeRequestId,
    registry_authority: FluxReconciliationAuthorityStamp,
    boundary: FluxBoundaryKey,
    interval: SynchronizationInterval,
    snapshot: FluxSnapshotToken,
    reference_authority: FluxSourceAuthorityStamp,
    refined_authority: FluxSourceAuthorityStamp,
    source_revision: FluxRegisterRevision,
    resulting_revision: FluxRegisterRevision,
    fluxes: BTreeMap<ConservedQuantity, ReconciledFlux>,
}

impl ReconciledFluxCertificate {
    pub const fn id(&self) -> ReconciledFluxSettlementId {
        self.id
    }

    pub const fn request(&self) -> FluxFinalizeRequestId {
        self.request
    }

    pub const fn registry_authority(&self) -> &FluxReconciliationAuthorityStamp {
        &self.registry_authority
    }

    pub const fn boundary(&self) -> FluxBoundaryKey {
        self.boundary
    }

    pub const fn interval(&self) -> SynchronizationInterval {
        self.interval
    }

    pub const fn snapshot(&self) -> FluxSnapshotToken {
        self.snapshot
    }

    pub const fn reference_authority(&self) -> FluxSourceAuthorityStamp {
        self.reference_authority
    }

    pub const fn refined_authority(&self) -> FluxSourceAuthorityStamp {
        self.refined_authority
    }

    pub const fn source_revision(&self) -> FluxRegisterRevision {
        self.source_revision
    }

    pub const fn resulting_revision(&self) -> FluxRegisterRevision {
        self.resulting_revision
    }

    pub fn fluxes(&self) -> &BTreeMap<ConservedQuantity, ReconciledFlux> {
        &self.fluxes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedContribution {
    request: FluxContributionRequest,
    receipt: FluxContributionReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedFinalization {
    request: FluxFinalizeRequestId,
    certificate: ReconciledFluxCertificate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FluxFinalizeOutcome {
    Mismatch(FluxMismatchReport),
    Certified(ReconciledFluxCertificate),
    AlreadyCertified(ReconciledFluxCertificate),
}

/// Single owner of one in-flight canonical synchronization register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeFluxRegister {
    key: FluxRegisterAuthorityKey,
    revision: FluxRegisterRevision,
    registry_authority: FluxReconciliationAuthorityStamp,
    boundary: FluxBoundaryKey,
    interval: SynchronizationInterval,
    reference_authority: FluxSourceAuthorityStamp,
    refined_authority: FluxSourceAuthorityStamp,
    contributions: BTreeMap<FluxContributionKey, FluxContribution>,
    committed_requests: BTreeMap<FluxContributionRequestId, CommittedContribution>,
    finalization: Option<CommittedFinalization>,
}

impl AuthoritativeFluxRegister {
    pub fn open(
        key: FluxRegisterAuthorityKey,
        registry: &FluxReconciliationRegistry,
        boundary: FluxBoundaryKey,
        interval: SynchronizationInterval,
        reference_authority: FluxSourceAuthorityStamp,
        refined_authority: FluxSourceAuthorityStamp,
    ) -> Result<Self, FluxReconciliationError> {
        let definition = registry
            .boundary(boundary)
            .ok_or(FluxReconciliationError::UnknownBoundary { boundary })?;
        if reference_authority.source() != definition.reference_source() {
            return Err(FluxReconciliationError::UnexpectedSourceAuthority {
                side: FluxSide::Reference,
                expected: definition.reference_source(),
                actual: reference_authority.source(),
            });
        }
        if refined_authority.source() != definition.refined_source() {
            return Err(FluxReconciliationError::UnexpectedSourceAuthority {
                side: FluxSide::Refined,
                expected: definition.refined_source(),
                actual: refined_authority.source(),
            });
        }
        if reference_authority.snapshot() != refined_authority.snapshot() {
            return Err(FluxReconciliationError::SnapshotMismatch {
                reference: reference_authority.snapshot(),
                refined: refined_authority.snapshot(),
            });
        }
        Ok(Self {
            key,
            revision: FluxRegisterRevision(0),
            registry_authority: registry.authority_stamp().clone(),
            boundary,
            interval,
            reference_authority,
            refined_authority,
            contributions: BTreeMap::new(),
            committed_requests: BTreeMap::new(),
            finalization: None,
        })
    }

    pub const fn key(&self) -> FluxRegisterAuthorityKey {
        self.key
    }

    pub const fn revision(&self) -> FluxRegisterRevision {
        self.revision
    }

    pub const fn boundary(&self) -> FluxBoundaryKey {
        self.boundary
    }

    pub const fn interval(&self) -> SynchronizationInterval {
        self.interval
    }

    pub const fn snapshot(&self) -> FluxSnapshotToken {
        self.reference_authority.snapshot()
    }

    pub const fn reference_authority(&self) -> FluxSourceAuthorityStamp {
        self.reference_authority
    }

    pub const fn refined_authority(&self) -> FluxSourceAuthorityStamp {
        self.refined_authority
    }

    pub fn is_finalized(&self) -> bool {
        self.finalization.is_some()
    }

    fn validate_registry(
        &self,
        registry: &FluxReconciliationRegistry,
    ) -> Result<&FluxBoundaryDefinition, FluxReconciliationError> {
        if registry.authority_stamp() != &self.registry_authority {
            return Err(FluxReconciliationError::RegistryAuthorityChanged);
        }
        registry
            .boundary(self.boundary)
            .ok_or(FluxReconciliationError::UnknownBoundary {
                boundary: self.boundary,
            })
    }

    pub fn record_contribution(
        &mut self,
        registry: &FluxReconciliationRegistry,
        request: FluxContributionRequest,
    ) -> Result<FluxContributionReceipt, FluxReconciliationError> {
        self.validate_registry(registry)?;

        if let Some(committed) = self.committed_requests.get(&request.id()) {
            if committed.request == request {
                return Ok(committed.receipt);
            }
            return Err(FluxReconciliationError::ConflictingContributionRequestId {
                request: request.id(),
            });
        }
        if self.finalization.is_some() {
            return Err(FluxReconciliationError::RegisterAlreadyFinalized {
                register: self.key,
            });
        }

        let definition = registry
            .boundary(self.boundary)
            .expect("validated exact registry retains boundary");
        let contribution = request.contribution();
        if !definition.requirements().contains_key(&contribution.quantity()) {
            return Err(FluxReconciliationError::UnregisteredFluxQuantity {
                boundary: self.boundary,
                quantity: contribution.quantity(),
            });
        }
        if !self.interval.contains(contribution.interval()) {
            return Err(FluxReconciliationError::ContributionOutsideSynchronizationInterval {
                contribution: contribution.key(),
                outer: self.interval,
                inner: contribution.interval(),
            });
        }
        let expected_authority = match contribution.side() {
            FluxSide::Reference => self.reference_authority,
            FluxSide::Refined => self.refined_authority,
        };
        if contribution.source_authority() != expected_authority {
            return Err(FluxReconciliationError::ContributionSourceAuthorityMismatch {
                contribution: contribution.key(),
                side: contribution.side(),
                expected: expected_authority,
                actual: contribution.source_authority(),
            });
        }
        if let Some(existing) = self.contributions.get(&contribution.key()) {
            return if *existing == contribution {
                Err(FluxReconciliationError::DuplicateContributionKey {
                    contribution: contribution.key(),
                })
            } else {
                Err(FluxReconciliationError::ConflictingContributionKey {
                    contribution: contribution.key(),
                })
            };
        }

        let next_revision = FluxRegisterRevision(
            self.revision
                .0
                .checked_add(1)
                .ok_or(FluxReconciliationError::FluxRegisterRevisionOverflow)?,
        );
        let receipt = FluxContributionReceipt {
            request: request.id(),
            contribution: contribution.key(),
            resulting_revision: next_revision,
        };
        self.contributions.insert(contribution.key(), contribution);
        self.revision = next_revision;
        self.committed_requests.insert(
            request.id(),
            CommittedContribution { request, receipt },
        );
        Ok(receipt)
    }

    /// Evaluate complete integrated fluxes without mutating the register.
    pub fn evaluate(
        &self,
        registry: &FluxReconciliationRegistry,
    ) -> Result<FluxMismatchReport, FluxReconciliationError> {
        let definition = self.validate_registry(registry)?;
        let mut balances = BTreeMap::new();
        for (quantity, requirement) in definition.requirements() {
            validate_temporal_partition(
                self.interval,
                self.contributions.values().copied().filter(|contribution| {
                    contribution.side() == FluxSide::Reference
                        && contribution.quantity() == *quantity
                }),
                FluxSide::Reference,
                *quantity,
            )?;
            validate_temporal_partition(
                self.interval,
                self.contributions.values().copied().filter(|contribution| {
                    contribution.side() == FluxSide::Refined
                        && contribution.quantity() == *quantity
                }),
                FluxSide::Refined,
                *quantity,
            )?;

            let reference_total = sum_contributions(
                self.contributions.iter().filter_map(|(_, contribution)| {
                    (contribution.side() == FluxSide::Reference
                        && contribution.quantity() == *quantity)
                        .then_some(contribution.amount())
                }),
                *quantity,
            )?;
            let refined_total = sum_contributions(
                self.contributions.iter().filter_map(|(_, contribution)| {
                    (contribution.side() == FluxSide::Refined
                        && contribution.quantity() == *quantity)
                        .then_some(contribution.amount())
                }),
                *quantity,
            )?;
            let mismatch_value = refined_total.get() - reference_total.get();
            if !mismatch_value.is_finite() {
                return Err(FluxReconciliationError::NonFiniteFluxArithmetic {
                    quantity: *quantity,
                });
            }
            let mismatch = IntegratedFlux::new(mismatch_value)?;
            balances.insert(
                *quantity,
                FluxReconciliationBalance {
                    quantity: *quantity,
                    reference_total,
                    refined_total,
                    mismatch,
                    tolerance: requirement.tolerance(),
                    settlement_side: requirement.settlement_side(),
                },
            );
        }
        Ok(FluxMismatchReport { balances })
    }

    /// Finalize only an already-complete, tolerance-qualified register. Larger
    /// mismatch remains a report and does not consume the finalize request or
    /// freeze the register.
    pub fn finalize(
        &mut self,
        registry: &FluxReconciliationRegistry,
        request: FluxFinalizeRequestId,
    ) -> Result<FluxFinalizeOutcome, FluxReconciliationError> {
        if let Some(finalized) = &self.finalization {
            if finalized.request == request {
                return Ok(FluxFinalizeOutcome::AlreadyCertified(
                    finalized.certificate.clone(),
                ));
            }
            return Err(FluxReconciliationError::RegisterAlreadyFinalized {
                register: self.key,
            });
        }

        let report = self.evaluate(registry)?;
        if !report.within_tolerance() {
            return Ok(FluxFinalizeOutcome::Mismatch(report));
        }

        let source_revision = self.revision;
        let resulting_revision = FluxRegisterRevision(
            source_revision
                .0
                .checked_add(1)
                .ok_or(FluxReconciliationError::FluxRegisterRevisionOverflow)?,
        );
        let fluxes = report
            .balances()
            .iter()
            .map(|(quantity, balance)| {
                (
                    *quantity,
                    ReconciledFlux {
                        quantity: *quantity,
                        amount: balance.settlement_amount(),
                        settlement_side: balance.settlement_side(),
                        reference_total: balance.reference_total(),
                        refined_total: balance.refined_total(),
                    },
                )
            })
            .collect();
        let certificate = ReconciledFluxCertificate {
            id: ReconciledFluxSettlementId {
                register: self.key,
                revision: resulting_revision,
            },
            request,
            registry_authority: self.registry_authority.clone(),
            boundary: self.boundary,
            interval: self.interval,
            snapshot: self.snapshot(),
            reference_authority: self.reference_authority,
            refined_authority: self.refined_authority,
            source_revision,
            resulting_revision,
            fluxes,
        };
        self.revision = resulting_revision;
        self.finalization = Some(CommittedFinalization {
            request,
            certificate: certificate.clone(),
        });
        Ok(FluxFinalizeOutcome::Certified(certificate))
    }
}

fn validate_temporal_partition(
    outer: SynchronizationInterval,
    contributions: impl IntoIterator<Item = FluxContribution>,
    side: FluxSide,
    quantity: ConservedQuantity,
) -> Result<(), FluxReconciliationError> {
    let mut intervals = contributions
        .into_iter()
        .map(|contribution| (contribution.interval(), contribution.key()))
        .collect::<Vec<_>>();
    if intervals.is_empty() {
        return Err(FluxReconciliationError::MissingFluxCoverage { side, quantity });
    }
    intervals.sort_by_key(|(interval, key)| {
        (
            interval.start_exclusive().0,
            interval.end_inclusive().0,
            *key,
        )
    });

    let mut cursor = outer.start_exclusive();
    for (interval, contribution) in intervals {
        if interval.start_exclusive().0 > cursor.0 {
            return Err(FluxReconciliationError::FluxCoverageGap {
                side,
                quantity,
                expected_start: cursor,
                actual_start: interval.start_exclusive(),
            });
        }
        if interval.start_exclusive().0 < cursor.0 {
            return Err(FluxReconciliationError::FluxCoverageOverlap {
                side,
                quantity,
                contribution,
                previous_end: cursor,
                actual_start: interval.start_exclusive(),
            });
        }
        cursor = interval.end_inclusive();
    }
    if cursor != outer.end_inclusive() {
        return Err(FluxReconciliationError::FluxCoverageGap {
            side,
            quantity,
            expected_start: cursor,
            actual_start: outer.end_inclusive(),
        });
    }
    Ok(())
}

fn sum_contributions(
    amounts: impl IntoIterator<Item = IntegratedFlux>,
    quantity: ConservedQuantity,
) -> Result<IntegratedFlux, FluxReconciliationError> {
    let mut total = 0.0;
    for amount in amounts {
        total += amount.get();
        if !total.is_finite() {
            return Err(FluxReconciliationError::NonFiniteFluxArithmetic { quantity });
        }
    }
    IntegratedFlux::new(total)
}

#[derive(Debug, Clone, PartialEq)]
pub enum FluxReconciliationError {
    InvalidSynchronizationInterval {
        start_exclusive: CanonicalTick,
        end_inclusive: CanonicalTick,
    },
    NonFiniteFlux(f64),
    NonFiniteTolerance(f64),
    NegativeTolerance(f64),
    IdenticalBoundarySources {
        boundary: FluxBoundaryKey,
        source: FluxSourceKey,
    },
    DuplicateQuantityRequirement {
        boundary: FluxBoundaryKey,
        quantity: ConservedQuantity,
    },
    EmptyBoundaryRequirements {
        boundary: FluxBoundaryKey,
    },
    ConflictingBoundaryRegistration {
        boundary: FluxBoundaryKey,
    },
    UnknownBoundary {
        boundary: FluxBoundaryKey,
    },
    RegistryAuthorityChanged,
    UnexpectedSourceAuthority {
        side: FluxSide,
        expected: FluxSourceKey,
        actual: FluxSourceKey,
    },
    SnapshotMismatch {
        reference: FluxSnapshotToken,
        refined: FluxSnapshotToken,
    },
    UnregisteredFluxQuantity {
        boundary: FluxBoundaryKey,
        quantity: ConservedQuantity,
    },
    ContributionOutsideSynchronizationInterval {
        contribution: FluxContributionKey,
        outer: SynchronizationInterval,
        inner: SynchronizationInterval,
    },
    ContributionSourceAuthorityMismatch {
        contribution: FluxContributionKey,
        side: FluxSide,
        expected: FluxSourceAuthorityStamp,
        actual: FluxSourceAuthorityStamp,
    },
    ConflictingContributionRequestId {
        request: FluxContributionRequestId,
    },
    DuplicateContributionKey {
        contribution: FluxContributionKey,
    },
    ConflictingContributionKey {
        contribution: FluxContributionKey,
    },
    FluxRegisterRevisionOverflow,
    MissingFluxCoverage {
        side: FluxSide,
        quantity: ConservedQuantity,
    },
    FluxCoverageGap {
        side: FluxSide,
        quantity: ConservedQuantity,
        expected_start: CanonicalTick,
        actual_start: CanonicalTick,
    },
    FluxCoverageOverlap {
        side: FluxSide,
        quantity: ConservedQuantity,
        contribution: FluxContributionKey,
        previous_end: CanonicalTick,
        actual_start: CanonicalTick,
    },
    NonFiniteFluxArithmetic {
        quantity: ConservedQuantity,
    },
    RegisterAlreadyFinalized {
        register: FluxRegisterAuthorityKey,
    },
}

impl fmt::Display for FluxReconciliationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSynchronizationInterval {
                start_exclusive,
                end_inclusive,
            } => write!(
                formatter,
                "flux synchronization interval must advance canonical time: ({}, {}]",
                start_exclusive.0, end_inclusive.0
            ),
            Self::NonFiniteFlux(value) => {
                write!(formatter, "integrated flux must be finite, got {value}")
            }
            Self::NonFiniteTolerance(value) => {
                write!(formatter, "flux tolerance must be finite, got {value}")
            }
            Self::NegativeTolerance(value) => {
                write!(formatter, "flux tolerance must be non-negative, got {value}")
            }
            Self::IdenticalBoundarySources { boundary, source } => write!(
                formatter,
                "flux boundary {}@{} cannot use the same source {}@{} on both sides",
                boundary.id(), boundary.version(), source.id(), source.version()
            ),
            Self::DuplicateQuantityRequirement { boundary, quantity } => write!(
                formatter,
                "flux boundary {}@{} repeats requirement for {quantity:?}",
                boundary.id(), boundary.version()
            ),
            Self::EmptyBoundaryRequirements { boundary } => write!(
                formatter,
                "flux boundary {}@{} has no conserved-quantity requirements",
                boundary.id(), boundary.version()
            ),
            Self::ConflictingBoundaryRegistration { boundary } => write!(
                formatter,
                "conflicting flux boundary registration for {}@{}",
                boundary.id(), boundary.version()
            ),
            Self::UnknownBoundary { boundary } => write!(
                formatter,
                "unknown flux boundary {}@{}",
                boundary.id(), boundary.version()
            ),
            Self::RegistryAuthorityChanged => {
                write!(formatter, "exact flux-reconciliation policy authority changed")
            }
            Self::UnexpectedSourceAuthority {
                side,
                expected,
                actual,
            } => write!(
                formatter,
                "unexpected {side:?} flux source: expected {}@{}, got {}@{}",
                expected.id(), expected.version(), actual.id(), actual.version()
            ),
            Self::SnapshotMismatch { reference, refined } => write!(
                formatter,
                "flux sides do not share one logical snapshot: reference {}, refined {}",
                reference.0, refined.0
            ),
            Self::UnregisteredFluxQuantity { boundary, quantity } => write!(
                formatter,
                "flux boundary {}@{} does not register {quantity:?}",
                boundary.id(), boundary.version()
            ),
            Self::ContributionOutsideSynchronizationInterval {
                contribution,
                outer,
                inner,
            } => write!(
                formatter,
                "flux contribution {}@{} interval ({}, {}] lies outside synchronization interval ({}, {}]",
                contribution.id(),
                contribution.version(),
                inner.start_exclusive().0,
                inner.end_inclusive().0,
                outer.start_exclusive().0,
                outer.end_inclusive().0
            ),
            Self::ContributionSourceAuthorityMismatch {
                contribution,
                side,
                expected,
                actual,
            } => write!(
                formatter,
                "flux contribution {}@{} has stale/incompatible {side:?} source authority: expected {}@{} rev {}, got {}@{} rev {}",
                contribution.id(),
                contribution.version(),
                expected.source().id(),
                expected.source().version(),
                expected.revision().0,
                actual.source().id(),
                actual.source().version(),
                actual.revision().0
            ),
            Self::ConflictingContributionRequestId { request } => write!(
                formatter,
                "flux contribution request id {} was reused with conflicting semantics",
                request.0
            ),
            Self::DuplicateContributionKey { contribution } => write!(
                formatter,
                "flux contribution {}@{} was submitted again under a different request id",
                contribution.id(), contribution.version()
            ),
            Self::ConflictingContributionKey { contribution } => write!(
                formatter,
                "flux contribution {}@{} was reused with conflicting semantics",
                contribution.id(), contribution.version()
            ),
            Self::FluxRegisterRevisionOverflow => write!(formatter, "flux-register revision overflow"),
            Self::MissingFluxCoverage { side, quantity } => write!(
                formatter,
                "missing {side:?} temporal flux coverage for {quantity:?}"
            ),
            Self::FluxCoverageGap {
                side,
                quantity,
                expected_start,
                actual_start,
            } => write!(
                formatter,
                "{side:?} temporal flux coverage for {quantity:?} has a gap after tick {} before tick {}",
                expected_start.0, actual_start.0
            ),
            Self::FluxCoverageOverlap {
                side,
                quantity,
                contribution,
                previous_end,
                actual_start,
            } => write!(
                formatter,
                "{side:?} temporal flux coverage for {quantity:?} overlaps at contribution {}@{}: prior end {}, next start {}",
                contribution.id(),
                contribution.version(),
                previous_end.0,
                actual_start.0
            ),
            Self::NonFiniteFluxArithmetic { quantity } => write!(
                formatter,
                "integrated flux arithmetic became non-finite for {quantity:?}"
            ),
            Self::RegisterAlreadyFinalized { register } => write!(
                formatter,
                "flux register {}@{} is already finalized",
                register.id(), register.version()
            ),
        }
    }
}

impl Error for FluxReconciliationError {}

#[cfg(test)]
mod tests {
    use super::*;

    const REGISTRY: FluxReconciliationRegistryKey =
        FluxReconciliationRegistryKey::new(20_000, 1);
    const BOUNDARY: FluxBoundaryKey = FluxBoundaryKey::new(20_010, 1);
    const REFERENCE_SOURCE: FluxSourceKey = FluxSourceKey::new(20_020, 1);
    const REFINED_SOURCE: FluxSourceKey = FluxSourceKey::new(20_021, 1);
    const REGISTER: FluxRegisterAuthorityKey = FluxRegisterAuthorityKey::new(20_030, 1);
    const SNAPSHOT: FluxSnapshotToken = FluxSnapshotToken(20_040);

    fn interval(start: u64, end: u64) -> SynchronizationInterval {
        SynchronizationInterval::new(CanonicalTick(start), CanonicalTick(end)).unwrap()
    }

    fn reference_authority() -> FluxSourceAuthorityStamp {
        FluxSourceAuthorityStamp::new(REFERENCE_SOURCE, FluxSourceRevision(7), SNAPSHOT)
    }

    fn refined_authority() -> FluxSourceAuthorityStamp {
        FluxSourceAuthorityStamp::new(REFINED_SOURCE, FluxSourceRevision(11), SNAPSHOT)
    }

    fn registry(
        tolerance: f64,
        settlement_side: CanonicalSettlementSide,
    ) -> FluxReconciliationRegistry {
        let boundary = FluxBoundaryDefinition::new(
            BOUNDARY,
            REFERENCE_SOURCE,
            REFINED_SOURCE,
            [FluxReconciliationRequirement::new(
                ConservedQuantity::WaterMass,
                AbsoluteFluxTolerance::new(tolerance).unwrap(),
                settlement_side,
            )],
        )
        .unwrap();
        let mut builder = FluxReconciliationRegistryBuilder::new(REGISTRY);
        builder.register_boundary(boundary).unwrap();
        builder.seal()
    }

    fn register(registry: &FluxReconciliationRegistry) -> AuthoritativeFluxRegister {
        AuthoritativeFluxRegister::open(
            REGISTER,
            registry,
            BOUNDARY,
            interval(0, 10),
            reference_authority(),
            refined_authority(),
        )
        .unwrap()
    }

    fn contribution(
        id: u128,
        side: FluxSide,
        start: u64,
        end: u64,
        amount: f64,
    ) -> FluxContribution {
        let authority = match side {
            FluxSide::Reference => reference_authority(),
            FluxSide::Refined => refined_authority(),
        };
        FluxContribution::new(
            FluxContributionKey::new(id, 1),
            side,
            ConservedQuantity::WaterMass,
            interval(start, end),
            IntegratedFlux::new(amount).unwrap(),
            authority,
        )
    }

    fn record(
        register: &mut AuthoritativeFluxRegister,
        registry: &FluxReconciliationRegistry,
        request: u128,
        contribution: FluxContribution,
    ) -> FluxContributionReceipt {
        register
            .record_contribution(
                registry,
                FluxContributionRequest::new(FluxContributionRequestId(request), contribution),
            )
            .unwrap()
    }

    #[test]
    fn matching_reference_and_subcycled_refined_flux_certify() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        record(
            &mut register,
            &registry,
            1,
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut register,
            &registry,
            2,
            contribution(101, FluxSide::Refined, 0, 5, 4.0),
        );
        record(
            &mut register,
            &registry,
            3,
            contribution(102, FluxSide::Refined, 5, 10, 6.0),
        );

        let report = register.evaluate(&registry).unwrap();
        assert!(report.within_tolerance());
        assert_eq!(
            report.balance(ConservedQuantity::WaterMass).unwrap().mismatch().get(),
            0.0
        );

        let FluxFinalizeOutcome::Certified(certificate) = register
            .finalize(&registry, FluxFinalizeRequestId(500))
            .unwrap()
        else {
            panic!("matching flux should certify");
        };
        let flux = certificate.fluxes()[&ConservedQuantity::WaterMass];
        assert_eq!(flux.amount().get(), 10.0);
        assert_eq!(flux.settlement_side(), CanonicalSettlementSide::Refined);
        assert!(register.is_finalized());
    }

    #[test]
    fn contribution_insertion_order_does_not_change_report() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mut first = register(&registry);
        let mut second = register(&registry);

        let reference = contribution(100, FluxSide::Reference, 0, 10, 10.0);
        let fine_a = contribution(101, FluxSide::Refined, 0, 5, 4.0);
        let fine_b = contribution(102, FluxSide::Refined, 5, 10, 6.0);

        record(&mut first, &registry, 1, reference);
        record(&mut first, &registry, 2, fine_a);
        record(&mut first, &registry, 3, fine_b);

        record(&mut second, &registry, 30, fine_b);
        record(&mut second, &registry, 20, fine_a);
        record(&mut second, &registry, 10, reference);

        assert_eq!(first.evaluate(&registry).unwrap(), second.evaluate(&registry).unwrap());
    }

    #[test]
    fn missing_or_overlapping_subcycle_coverage_fails_closed() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);

        let mut gap = register(&registry);
        record(
            &mut gap,
            &registry,
            1,
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut gap,
            &registry,
            2,
            contribution(101, FluxSide::Refined, 0, 4, 4.0),
        );
        record(
            &mut gap,
            &registry,
            3,
            contribution(102, FluxSide::Refined, 5, 10, 6.0),
        );
        assert!(matches!(
            gap.evaluate(&registry),
            Err(FluxReconciliationError::FluxCoverageGap { .. })
        ));

        let mut overlap = register(&registry);
        record(
            &mut overlap,
            &registry,
            10,
            contribution(110, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut overlap,
            &registry,
            20,
            contribution(111, FluxSide::Refined, 0, 6, 5.0),
        );
        record(
            &mut overlap,
            &registry,
            30,
            contribution(112, FluxSide::Refined, 5, 10, 5.0),
        );
        assert!(matches!(
            overlap.evaluate(&registry),
            Err(FluxReconciliationError::FluxCoverageOverlap { .. })
        ));
    }

    #[test]
    fn source_snapshot_and_revision_are_bound_before_accumulation() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mismatched_snapshot = FluxSourceAuthorityStamp::new(
            REFINED_SOURCE,
            FluxSourceRevision(11),
            FluxSnapshotToken(999),
        );
        assert!(matches!(
            AuthoritativeFluxRegister::open(
                REGISTER,
                &registry,
                BOUNDARY,
                interval(0, 10),
                reference_authority(),
                mismatched_snapshot,
            ),
            Err(FluxReconciliationError::SnapshotMismatch { .. })
        ));

        let mut register = register(&registry);
        let stale = FluxSourceAuthorityStamp::new(
            REFINED_SOURCE,
            FluxSourceRevision(10),
            SNAPSHOT,
        );
        let stale_contribution = FluxContribution::new(
            FluxContributionKey::new(200, 1),
            FluxSide::Refined,
            ConservedQuantity::WaterMass,
            interval(0, 10),
            IntegratedFlux::new(10.0).unwrap(),
            stale,
        );
        let before = register.clone();
        assert!(matches!(
            register.record_contribution(
                &registry,
                FluxContributionRequest::new(FluxContributionRequestId(1), stale_contribution),
            ),
            Err(FluxReconciliationError::ContributionSourceAuthorityMismatch { .. })
        ));
        assert_eq!(register, before);
    }

    #[test]
    fn mismatch_beyond_tolerance_is_visible_and_does_not_finalize() {
        let registry = registry(0.1, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        record(
            &mut register,
            &registry,
            1,
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut register,
            &registry,
            2,
            contribution(101, FluxSide::Refined, 0, 10, 10.4),
        );
        let before_revision = register.revision();

        let FluxFinalizeOutcome::Mismatch(report) = register
            .finalize(&registry, FluxFinalizeRequestId(500))
            .unwrap()
        else {
            panic!("large mismatch must not certify");
        };
        assert!(!report.within_tolerance());
        assert_eq!(register.revision(), before_revision);
        assert!(!register.is_finalized());
    }

    #[test]
    fn tolerated_mismatch_uses_registered_settlement_side() {
        let registry = registry(0.5, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        record(
            &mut register,
            &registry,
            1,
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut register,
            &registry,
            2,
            contribution(101, FluxSide::Refined, 0, 10, 10.4),
        );

        let FluxFinalizeOutcome::Certified(certificate) = register
            .finalize(&registry, FluxFinalizeRequestId(500))
            .unwrap()
        else {
            panic!("registered tolerance should certify");
        };
        assert_eq!(
            certificate.fluxes()[&ConservedQuantity::WaterMass]
                .amount()
                .get(),
            10.4
        );
    }

    #[test]
    fn contribution_retry_is_idempotent_and_conflicting_reuse_rejects() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        let request = FluxContributionRequest::new(
            FluxContributionRequestId(7),
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        let first = register.record_contribution(&registry, request).unwrap();
        let revision = register.revision();
        let retry = register.record_contribution(&registry, request).unwrap();
        assert_eq!(retry, first);
        assert_eq!(register.revision(), revision);

        let conflicting = FluxContributionRequest::new(
            FluxContributionRequestId(7),
            contribution(101, FluxSide::Reference, 0, 10, 9.0),
        );
        assert!(matches!(
            register.record_contribution(&registry, conflicting),
            Err(FluxReconciliationError::ConflictingContributionRequestId { .. })
        ));
        assert_eq!(register.revision(), revision);
    }

    #[test]
    fn finalization_retry_returns_same_certificate_and_freezes_contributions() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        record(
            &mut register,
            &registry,
            1,
            contribution(100, FluxSide::Reference, 0, 10, 10.0),
        );
        record(
            &mut register,
            &registry,
            2,
            contribution(101, FluxSide::Refined, 0, 10, 10.0),
        );

        let FluxFinalizeOutcome::Certified(first) = register
            .finalize(&registry, FluxFinalizeRequestId(9))
            .unwrap()
        else {
            panic!("first finalization should certify");
        };
        let revision = register.revision();
        let FluxFinalizeOutcome::AlreadyCertified(retry) = register
            .finalize(&registry, FluxFinalizeRequestId(9))
            .unwrap()
        else {
            panic!("retry should return prior certificate");
        };
        assert_eq!(retry, first);
        assert_eq!(register.revision(), revision);

        let late = FluxContributionRequest::new(
            FluxContributionRequestId(10),
            contribution(102, FluxSide::Refined, 0, 10, 0.0),
        );
        assert!(matches!(
            register.record_contribution(&registry, late),
            Err(FluxReconciliationError::RegisterAlreadyFinalized { .. })
        ));
    }

    #[test]
    fn changed_exact_registry_policy_stales_open_register() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let register = register(&registry);
        let changed = registry(1.0, CanonicalSettlementSide::Refined);
        assert!(matches!(
            register.evaluate(&changed),
            Err(FluxReconciliationError::RegistryAuthorityChanged)
        ));
    }

    #[test]
    fn invalid_numeric_policy_and_interval_fail_closed() {
        assert!(IntegratedFlux::new(f64::NAN).is_err());
        assert!(AbsoluteFluxTolerance::new(-1.0).is_err());
        assert!(AbsoluteFluxTolerance::new(f64::INFINITY).is_err());
        assert!(SynchronizationInterval::new(CanonicalTick(4), CanonicalTick(4)).is_err());
        assert!(SynchronizationInterval::new(CanonicalTick(5), CanonicalTick(4)).is_err());
    }

    #[test]
    fn policy_insertion_order_is_canonical() {
        let first = FluxBoundaryDefinition::new(
            BOUNDARY,
            REFERENCE_SOURCE,
            REFINED_SOURCE,
            [
                FluxReconciliationRequirement::new(
                    ConservedQuantity::WaterMass,
                    AbsoluteFluxTolerance::new(0.0).unwrap(),
                    CanonicalSettlementSide::Refined,
                ),
                FluxReconciliationRequirement::new(
                    ConservedQuantity::CarbonMass,
                    AbsoluteFluxTolerance::new(0.1).unwrap(),
                    CanonicalSettlementSide::Reference,
                ),
            ],
        )
        .unwrap();
        let second = FluxBoundaryDefinition::new(
            BOUNDARY,
            REFERENCE_SOURCE,
            REFINED_SOURCE,
            [
                FluxReconciliationRequirement::new(
                    ConservedQuantity::CarbonMass,
                    AbsoluteFluxTolerance::new(0.1).unwrap(),
                    CanonicalSettlementSide::Reference,
                ),
                FluxReconciliationRequirement::new(
                    ConservedQuantity::WaterMass,
                    AbsoluteFluxTolerance::new(0.0).unwrap(),
                    CanonicalSettlementSide::Refined,
                ),
            ],
        )
        .unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn unregistered_quantity_cannot_enter_register() {
        let registry = registry(0.0, CanonicalSettlementSide::Refined);
        let mut register = register(&registry);
        let carbon = FluxContribution::new(
            FluxContributionKey::new(100, 1),
            FluxSide::Reference,
            ConservedQuantity::CarbonMass,
            interval(0, 10),
            IntegratedFlux::new(1.0).unwrap(),
            reference_authority(),
        );
        assert!(matches!(
            register.record_contribution(
                &registry,
                FluxContributionRequest::new(FluxContributionRequestId(1), carbon),
            ),
            Err(FluxReconciliationError::UnregisteredFluxQuantity { .. })
        ));
    }
}
