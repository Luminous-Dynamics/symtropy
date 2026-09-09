// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Horizon-bounded authority for repeated canonical use of approximate closures.
//!
//! A typed closure qualification is evidence over a declared horizon, not a
//! perpetual license to restart that horizon from its own approximate endpoint.
//! This layer therefore measures qualified use from an explicit validation anchor
//! and refuses to extend beyond the registered horizon unless a new anchor is
//! supplied by a separately qualified authority producer.
//!
//! V0 deliberately does **not** add ppm values or invent an error-composition
//! theorem. It tracks evidence age in canonical ticks. An exhausted horizon
//! requires revalidation, promotion/measurement, or a future explicit composition
//! authority.
//!
//! [`ClosureValidationAnchor`] intentionally has no public constructor. This
//! module consumes anchors; a later exact/measurement/shadow-validation authority
//! must mint them. A caller cannot reset its own approximation horizon with a
//! boolean or arbitrary token.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{
    CapabilityEvidence, EcologicalInformation, EvidenceLineageToken, EvidenceRequirement,
    ProcessKey, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, ManifestBoundInformationPolicyRegistry,
};

use super::spatiotemporal_information::{
    CanonicalTick, SpatiotemporalPolicyAuthorityStamp, SpatiotemporalPolicyError,
    SpatiotemporalPolicyRegistry,
};
use super::typed_closure_process_acceptance::{
    TypedClosureProcessAcceptanceAuthorityStamp, TypedClosureProcessAcceptanceError,
    TypedClosureProcessAcceptanceRegistry,
};
use super::typed_closure_qualification::{
    TypedClosureQualificationAuthorityStamp, TypedClosureQualificationError,
    TypedClosureQualificationRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsagePolicyRegistryKey {
    id: u128,
    version: u32,
}

impl ClosureUsagePolicyRegistryKey {
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
pub struct ClosureUsagePolicyKey {
    id: u128,
    version: u32,
}

impl ClosureUsagePolicyKey {
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
pub struct MaximumClosureUseTicks(u64);

impl MaximumClosureUseTicks {
    pub const fn new(ticks: u64) -> Result<Self, ClosureUsageAuthorityError> {
        if ticks == 0 {
            return Err(ClosureUsageAuthorityError::ZeroMaximumUseHorizon);
        }
        Ok(Self(ticks))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Registry-owned V0 use policy for one exact closure-backed process lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClosureUsagePolicyDefinition {
    key: ClosureUsagePolicyKey,
    process: ProcessKey,
    information: EcologicalInformation,
    lineage: EvidenceLineageToken,
    representation: RepresentationKey,
    maximum_use: MaximumClosureUseTicks,
}

impl ClosureUsagePolicyDefinition {
    pub const fn new(
        key: ClosureUsagePolicyKey,
        process: ProcessKey,
        information: EcologicalInformation,
        lineage: EvidenceLineageToken,
        representation: RepresentationKey,
        maximum_use: MaximumClosureUseTicks,
    ) -> Self {
        Self {
            key,
            process,
            information,
            lineage,
            representation,
            maximum_use,
        }
    }

    pub const fn key(self) -> ClosureUsagePolicyKey {
        self.key
    }

    pub const fn process(self) -> ProcessKey {
        self.process
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn lineage(self) -> EvidenceLineageToken {
        self.lineage
    }

    pub const fn representation(self) -> RepresentationKey {
        self.representation
    }

    pub const fn maximum_use(self) -> MaximumClosureUseTicks {
        self.maximum_use
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureUsagePolicyAuthorityStamp {
    key: ClosureUsagePolicyRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    closure_authority: TypedClosureQualificationAuthorityStamp,
    acceptance_authority: TypedClosureProcessAcceptanceAuthorityStamp,
    spatiotemporal_authority: SpatiotemporalPolicyAuthorityStamp,
    definitions: BTreeMap<ClosureUsagePolicyKey, ClosureUsagePolicyDefinition>,
}

impl ClosureUsagePolicyAuthorityStamp {
    pub const fn key(&self) -> ClosureUsagePolicyRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }

    pub const fn closure_authority(&self) -> &TypedClosureQualificationAuthorityStamp {
        &self.closure_authority
    }

    pub const fn acceptance_authority(&self) -> &TypedClosureProcessAcceptanceAuthorityStamp {
        &self.acceptance_authority
    }

    pub const fn spatiotemporal_authority(&self) -> &SpatiotemporalPolicyAuthorityStamp {
        &self.spatiotemporal_authority
    }

    pub fn definitions(
        &self,
    ) -> &BTreeMap<ClosureUsagePolicyKey, ClosureUsagePolicyDefinition> {
        &self.definitions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureUsagePolicyRegistryBuilder {
    key: ClosureUsagePolicyRegistryKey,
    definitions: BTreeMap<ClosureUsagePolicyKey, ClosureUsagePolicyDefinition>,
}

impl ClosureUsagePolicyRegistryBuilder {
    pub const fn new(key: ClosureUsagePolicyRegistryKey) -> Self {
        Self {
            key,
            definitions: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        definition: ClosureUsagePolicyDefinition,
    ) -> Result<(), ClosureUsageAuthorityError> {
        let key = definition.key();
        if let Some(existing) = self.definitions.get(&key) {
            if existing == &definition {
                return Ok(());
            }
            return Err(ClosureUsageAuthorityError::ConflictingUsagePolicyRegistration {
                policy: key,
            });
        }
        self.definitions.insert(key, definition);
        Ok(())
    }

    /// Seal only policies that point at a current qualified closure lineage,
    /// a representation that actually carries that lineage for the declared
    /// information, and a legacy process lane that permits closure evidence.
    ///
    /// The exact typed process-acceptance corpus is bound here and must later be
    /// used by the qualified anchor producer; V0 does not reach into its private
    /// corpus to manufacture a weaker duplicate acceptance algebra.
    pub fn seal(
        self,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ClosureUsagePolicyRegistry, ClosureUsageAuthorityError> {
        information_policy
            .authority_stamp()
            .validate_registry(information_policy.registry())
            .map_err(ClosureUsageAuthorityError::InformationPolicyIdentity)?;
        closures
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::ClosureQualification)?;
        acceptances
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::ClosureAcceptance)?;
        spatiotemporal
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::SpatiotemporalPolicy)?;

        for definition in self.definitions.values().copied() {
            validate_definition(information_policy, closures, definition)?;
        }

        let authority = ClosureUsagePolicyAuthorityStamp {
            key: self.key,
            information_policy_authority: information_policy.authority_stamp().clone(),
            closure_authority: closures.authority_stamp().clone(),
            acceptance_authority: acceptances.authority_stamp().clone(),
            spatiotemporal_authority: spatiotemporal.authority_stamp().clone(),
            definitions: self.definitions.clone(),
        };
        Ok(ClosureUsagePolicyRegistry {
            authority,
            definitions: self.definitions,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureUsagePolicyRegistry {
    authority: ClosureUsagePolicyAuthorityStamp,
    definitions: BTreeMap<ClosureUsagePolicyKey, ClosureUsagePolicyDefinition>,
}

impl ClosureUsagePolicyRegistry {
    pub const fn authority_stamp(&self) -> &ClosureUsagePolicyAuthorityStamp {
        &self.authority
    }

    pub fn definition(
        &self,
        key: ClosureUsagePolicyKey,
    ) -> Option<ClosureUsagePolicyDefinition> {
        self.definitions.get(&key).copied()
    }

    pub fn validate_current(
        &self,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<(), ClosureUsageAuthorityError> {
        information_policy
            .authority_stamp()
            .validate_registry(information_policy.registry())
            .map_err(ClosureUsageAuthorityError::InformationPolicyIdentity)?;
        closures
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::ClosureQualification)?;
        acceptances
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::ClosureAcceptance)?;
        spatiotemporal
            .validate_current(information_policy)
            .map_err(ClosureUsageAuthorityError::SpatiotemporalPolicy)?;

        if information_policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(ClosureUsageAuthorityError::InformationPolicyAuthorityChanged);
        }
        if closures.authority_stamp() != &self.authority.closure_authority {
            return Err(ClosureUsageAuthorityError::ClosureQualificationAuthorityChanged);
        }
        if acceptances.authority_stamp() != &self.authority.acceptance_authority {
            return Err(ClosureUsageAuthorityError::ClosureAcceptanceAuthorityChanged);
        }
        if spatiotemporal.authority_stamp() != &self.authority.spatiotemporal_authority {
            return Err(ClosureUsageAuthorityError::SpatiotemporalAuthorityChanged);
        }

        for definition in self.definitions.values().copied() {
            validate_definition(information_policy, closures, definition)?;
        }
        Ok(())
    }
}

fn validate_definition(
    information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
    closures: &TypedClosureQualificationRegistry,
    definition: ClosureUsagePolicyDefinition,
) -> Result<(), ClosureUsageAuthorityError> {
    let process = information_policy
        .registry()
        .resolve_process(definition.process())
        .map_err(ClosureUsageAuthorityError::InformationRegistry)?;
    let permits_closure = process.profile().requirements().iter().any(|requirement| {
        requirement.information() == definition.information()
            && matches!(
                requirement.evidence(),
                EvidenceRequirement::ExactOrQualifiedClosure(_)
            )
    });
    if !permits_closure {
        return Err(ClosureUsageAuthorityError::ProcessDoesNotPermitClosure {
            process: definition.process(),
            information: definition.information(),
        });
    }

    let qualification = closures
        .resolve(information_policy, definition.lineage())
        .map_err(ClosureUsageAuthorityError::ClosureQualification)?;
    if definition.maximum_use().get() > qualification.qualification().horizon().get() {
        return Err(ClosureUsageAuthorityError::UsageHorizonExceedsQualification {
            policy: definition.key(),
            maximum_use: definition.maximum_use(),
            qualified_horizon: qualification.qualification().horizon().get(),
        });
    }

    let representation = information_policy
        .registry()
        .resolve_representation(definition.representation())
        .map_err(ClosureUsageAuthorityError::InformationRegistry)?;
    let exact_evidence = qualification.qualification().evidence();
    let carries = representation
        .capabilities()
        .claims()
        .iter()
        .any(|(available, evidence)| {
            available.covers(definition.information())
                && evidence.contains(&CapabilityEvidence::QualifiedClosure(exact_evidence))
        });
    if !carries {
        return Err(ClosureUsageAuthorityError::RepresentationDoesNotCarryClosureLineage {
            representation: definition.representation(),
            information: definition.information(),
            lineage: definition.lineage(),
        });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureValidationAnchorId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureValidationAnchorRevision(pub u64);

/// Opaque proof receipt consumed by this module. There is intentionally no
/// public constructor. A later qualified authority producer must prove the exact
/// richer-state restriction, measurement, shadow validation, or composition
/// theorem before it can mint one of these receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureValidationAnchor {
    id: ClosureValidationAnchorId,
    revision: ClosureValidationAnchorRevision,
    tick: CanonicalTick,
    usage_policy_authority: ClosureUsagePolicyAuthorityStamp,
    usage_policy: ClosureUsagePolicyKey,
}

impl ClosureValidationAnchor {
    pub const fn id(&self) -> ClosureValidationAnchorId {
        self.id
    }

    pub const fn revision(&self) -> ClosureValidationAnchorRevision {
        self.revision
    }

    pub const fn tick(&self) -> CanonicalTick {
        self.tick
    }

    pub const fn usage_policy_authority(&self) -> &ClosureUsagePolicyAuthorityStamp {
        &self.usage_policy_authority
    }

    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey {
        self.usage_policy
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsageAuthorityScope {
    id: u128,
    version: u32,
}

impl ClosureUsageAuthorityScope {
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
pub struct ClosureUsageRevision(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsageRequestId(pub u128);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsageReceiptId {
    scope: ClosureUsageAuthorityScope,
    revision: ClosureUsageRevision,
}

impl ClosureUsageReceiptId {
    pub const fn scope(self) -> ClosureUsageAuthorityScope {
        self.scope
    }

    pub const fn revision(self) -> ClosureUsageRevision {
        self.revision
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsageWindow {
    start_exclusive: CanonicalTick,
    end_inclusive: CanonicalTick,
}

impl ClosureUsageWindow {
    pub fn new(
        start_exclusive: CanonicalTick,
        end_inclusive: CanonicalTick,
    ) -> Result<Self, ClosureUsageAuthorityError> {
        if end_inclusive.0 <= start_exclusive.0 {
            return Err(ClosureUsageAuthorityError::InvalidUsageWindow {
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureUsageRequest {
    id: ClosureUsageRequestId,
    window: ClosureUsageWindow,
}

impl ClosureUsageRequest {
    pub const fn new(id: ClosureUsageRequestId, window: ClosureUsageWindow) -> Self {
        Self { id, window }
    }

    pub const fn id(self) -> ClosureUsageRequestId {
        self.id
    }

    pub const fn window(self) -> ClosureUsageWindow {
        self.window
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureUsageAuthorityStamp {
    scope: ClosureUsageAuthorityScope,
    revision: ClosureUsageRevision,
    usage_policy_authority: ClosureUsagePolicyAuthorityStamp,
    usage_policy: ClosureUsagePolicyKey,
    anchor_id: ClosureValidationAnchorId,
    anchor_revision: ClosureValidationAnchorRevision,
    anchor_tick: CanonicalTick,
    used_through: CanonicalTick,
}

impl ClosureUsageAuthorityStamp {
    pub const fn scope(&self) -> ClosureUsageAuthorityScope {
        self.scope
    }

    pub const fn revision(&self) -> ClosureUsageRevision {
        self.revision
    }

    pub const fn usage_policy_authority(&self) -> &ClosureUsagePolicyAuthorityStamp {
        &self.usage_policy_authority
    }

    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey {
        self.usage_policy
    }

    pub const fn anchor_id(&self) -> ClosureValidationAnchorId {
        self.anchor_id
    }

    pub const fn anchor_revision(&self) -> ClosureValidationAnchorRevision {
        self.anchor_revision
    }

    pub const fn anchor_tick(&self) -> CanonicalTick {
        self.anchor_tick
    }

    pub const fn used_through(&self) -> CanonicalTick {
        self.used_through
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureUsageReceipt {
    id: ClosureUsageReceiptId,
    request: ClosureUsageRequestId,
    source_revision: ClosureUsageRevision,
    resulting_revision: ClosureUsageRevision,
    usage_policy_authority: ClosureUsagePolicyAuthorityStamp,
    usage_policy: ClosureUsagePolicyKey,
    anchor_id: ClosureValidationAnchorId,
    anchor_revision: ClosureValidationAnchorRevision,
    window: ClosureUsageWindow,
}

impl ClosureUsageReceipt {
    pub const fn id(&self) -> ClosureUsageReceiptId {
        self.id
    }

    pub const fn request(&self) -> ClosureUsageRequestId {
        self.request
    }

    pub const fn source_revision(&self) -> ClosureUsageRevision {
        self.source_revision
    }

    pub const fn resulting_revision(&self) -> ClosureUsageRevision {
        self.resulting_revision
    }

    pub const fn usage_policy(&self) -> ClosureUsagePolicyKey {
        self.usage_policy
    }

    pub const fn anchor_id(&self) -> ClosureValidationAnchorId {
        self.anchor_id
    }

    pub const fn anchor_revision(&self) -> ClosureValidationAnchorRevision {
        self.anchor_revision
    }

    pub const fn window(&self) -> ClosureUsageWindow {
        self.window
    }

    pub const fn usage_policy_authority(&self) -> &ClosureUsagePolicyAuthorityStamp {
        &self.usage_policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureRevalidationRequirement {
    anchor_id: ClosureValidationAnchorId,
    anchor_revision: ClosureValidationAnchorRevision,
    anchor_tick: CanonicalTick,
    qualified_end: CanonicalTick,
    requested_end: CanonicalTick,
}

impl ClosureRevalidationRequirement {
    pub const fn anchor_id(&self) -> ClosureValidationAnchorId {
        self.anchor_id
    }

    pub const fn anchor_revision(&self) -> ClosureValidationAnchorRevision {
        self.anchor_revision
    }

    pub const fn anchor_tick(&self) -> CanonicalTick {
        self.anchor_tick
    }

    pub const fn qualified_end(&self) -> CanonicalTick {
        self.qualified_end
    }

    pub const fn requested_end(&self) -> CanonicalTick {
        self.requested_end
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedClosureUsage {
    request: ClosureUsageRequest,
    source_authority: ClosureUsageAuthorityStamp,
}

impl PreparedClosureUsage {
    pub const fn request(&self) -> ClosureUsageRequest {
        self.request
    }

    pub const fn source_authority(&self) -> &ClosureUsageAuthorityStamp {
        &self.source_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClosureUsagePreparationOutcome {
    AlreadyCommitted(ClosureUsageReceipt),
    Ready(PreparedClosureUsage),
    RevalidationRequired(ClosureRevalidationRequirement),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedUsage {
    request: ClosureUsageRequest,
    receipt: ClosureUsageReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureAnchorInstallRequestId(pub u128);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureAnchorInstallRequest {
    id: ClosureAnchorInstallRequestId,
    anchor: ClosureValidationAnchor,
}

impl ClosureAnchorInstallRequest {
    pub const fn new(id: ClosureAnchorInstallRequestId, anchor: ClosureValidationAnchor) -> Self {
        Self { id, anchor }
    }

    pub const fn id(&self) -> ClosureAnchorInstallRequestId {
        self.id
    }

    pub const fn anchor(&self) -> &ClosureValidationAnchor {
        &self.anchor
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureAnchorInstallReceipt {
    request: ClosureAnchorInstallRequestId,
    source_revision: ClosureUsageRevision,
    resulting_revision: ClosureUsageRevision,
    anchor_id: ClosureValidationAnchorId,
    anchor_revision: ClosureValidationAnchorRevision,
    anchor_tick: CanonicalTick,
}

impl ClosureAnchorInstallReceipt {
    pub const fn request(&self) -> ClosureAnchorInstallRequestId {
        self.request
    }

    pub const fn source_revision(&self) -> ClosureUsageRevision {
        self.source_revision
    }

    pub const fn resulting_revision(&self) -> ClosureUsageRevision {
        self.resulting_revision
    }

    pub const fn anchor_id(&self) -> ClosureValidationAnchorId {
        self.anchor_id
    }

    pub const fn anchor_revision(&self) -> ClosureValidationAnchorRevision {
        self.anchor_revision
    }

    pub const fn anchor_tick(&self) -> CanonicalTick {
        self.anchor_tick
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CommittedAnchorInstall {
    request: ClosureAnchorInstallRequest,
    receipt: ClosureAnchorInstallReceipt,
}

/// Canonical owner of one closure-use horizon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeClosureUsageState {
    authority: ClosureUsageAuthorityStamp,
    anchor: ClosureValidationAnchor,
    committed_usage: BTreeMap<ClosureUsageRequestId, CommittedUsage>,
    committed_anchor_installs: BTreeMap<ClosureAnchorInstallRequestId, CommittedAnchorInstall>,
    seen_anchors: BTreeSet<(ClosureValidationAnchorId, ClosureValidationAnchorRevision)>,
}

impl AuthoritativeClosureUsageState {
    pub fn bootstrap(
        scope: ClosureUsageAuthorityScope,
        registry: &ClosureUsagePolicyRegistry,
        anchor: ClosureValidationAnchor,
    ) -> Result<Self, ClosureUsageAuthorityError> {
        validate_anchor(registry, &anchor)?;
        let policy = registry
            .definition(anchor.usage_policy())
            .ok_or(ClosureUsageAuthorityError::UnknownUsagePolicy {
                policy: anchor.usage_policy(),
            })?;
        let authority = ClosureUsageAuthorityStamp {
            scope,
            revision: ClosureUsageRevision(0),
            usage_policy_authority: registry.authority_stamp().clone(),
            usage_policy: policy.key(),
            anchor_id: anchor.id(),
            anchor_revision: anchor.revision(),
            anchor_tick: anchor.tick(),
            used_through: anchor.tick(),
        };
        Ok(Self {
            authority,
            seen_anchors: BTreeSet::from([(anchor.id(), anchor.revision())]),
            anchor,
            committed_usage: BTreeMap::new(),
            committed_anchor_installs: BTreeMap::new(),
        })
    }

    pub const fn authority_stamp(&self) -> &ClosureUsageAuthorityStamp {
        &self.authority
    }

    pub const fn anchor(&self) -> &ClosureValidationAnchor {
        &self.anchor
    }

    #[allow(clippy::too_many_arguments)]
    pub fn prepare_usage(
        &self,
        request: ClosureUsageRequest,
        registry: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ClosureUsagePreparationOutcome, ClosureUsageAuthorityError> {
        if let Some(committed) = self.committed_usage.get(&request.id()) {
            if committed.request == request {
                return Ok(ClosureUsagePreparationOutcome::AlreadyCommitted(
                    committed.receipt.clone(),
                ));
            }
            return Err(ClosureUsageAuthorityError::ConflictingUsageRequestId {
                request: request.id(),
            });
        }

        validate_state_context(
            self,
            registry,
            information_policy,
            closures,
            acceptances,
            spatiotemporal,
        )?;

        if request.window().start_exclusive() != self.authority.used_through() {
            return Err(ClosureUsageAuthorityError::NonContiguousUsageWindow {
                expected_start: self.authority.used_through(),
                actual_start: request.window().start_exclusive(),
            });
        }

        let definition = registry
            .definition(self.authority.usage_policy())
            .ok_or(ClosureUsageAuthorityError::UnknownUsagePolicy {
                policy: self.authority.usage_policy(),
            })?;
        let qualified_end = self
            .authority
            .anchor_tick()
            .0
            .checked_add(definition.maximum_use().get())
            .map(CanonicalTick)
            .ok_or(ClosureUsageAuthorityError::QualifiedHorizonOverflow)?;

        if request.window().end_inclusive().0 > qualified_end.0 {
            return Ok(ClosureUsagePreparationOutcome::RevalidationRequired(
                ClosureRevalidationRequirement {
                    anchor_id: self.authority.anchor_id(),
                    anchor_revision: self.authority.anchor_revision(),
                    anchor_tick: self.authority.anchor_tick(),
                    qualified_end,
                    requested_end: request.window().end_inclusive(),
                },
            ));
        }

        Ok(ClosureUsagePreparationOutcome::Ready(
            PreparedClosureUsage {
                request,
                source_authority: self.authority.clone(),
            },
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn commit_usage(
        &mut self,
        prepared: &PreparedClosureUsage,
        registry: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ClosureUsageReceipt, ClosureUsageAuthorityError> {
        if let Some(committed) = self.committed_usage.get(&prepared.request().id()) {
            if committed.request == prepared.request() {
                return Ok(committed.receipt.clone());
            }
            return Err(ClosureUsageAuthorityError::ConflictingUsageRequestId {
                request: prepared.request().id(),
            });
        }
        if self.authority != *prepared.source_authority() {
            return Err(ClosureUsageAuthorityError::PreparedUsageStale);
        }

        let current = self.prepare_usage(
            prepared.request(),
            registry,
            information_policy,
            closures,
            acceptances,
            spatiotemporal,
        )?;
        let ClosureUsagePreparationOutcome::Ready(current) = current else {
            return Err(ClosureUsageAuthorityError::PreparedUsageNoLongerAuthorized);
        };
        if current != *prepared {
            return Err(ClosureUsageAuthorityError::PreparedUsageStale);
        }

        let source_revision = self.authority.revision();
        let resulting_revision = ClosureUsageRevision(
            source_revision
                .0
                .checked_add(1)
                .ok_or(ClosureUsageAuthorityError::UsageRevisionOverflow)?,
        );
        let receipt = ClosureUsageReceipt {
            id: ClosureUsageReceiptId {
                scope: self.authority.scope(),
                revision: resulting_revision,
            },
            request: prepared.request().id(),
            source_revision,
            resulting_revision,
            usage_policy_authority: self.authority.usage_policy_authority().clone(),
            usage_policy: self.authority.usage_policy(),
            anchor_id: self.authority.anchor_id(),
            anchor_revision: self.authority.anchor_revision(),
            window: prepared.request().window(),
        };

        self.authority.revision = resulting_revision;
        self.authority.used_through = prepared.request().window().end_inclusive();
        self.committed_usage.insert(
            prepared.request().id(),
            CommittedUsage {
                request: prepared.request(),
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn install_anchor(
        &mut self,
        request: ClosureAnchorInstallRequest,
        registry: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
    ) -> Result<ClosureAnchorInstallReceipt, ClosureUsageAuthorityError> {
        if let Some(committed) = self.committed_anchor_installs.get(&request.id()) {
            if committed.request == request {
                return Ok(committed.receipt.clone());
            }
            return Err(ClosureUsageAuthorityError::ConflictingAnchorInstallRequestId {
                request: request.id(),
            });
        }

        validate_state_context(
            self,
            registry,
            information_policy,
            closures,
            acceptances,
            spatiotemporal,
        )?;
        validate_anchor(registry, request.anchor())?;
        if request.anchor().usage_policy() != self.authority.usage_policy() {
            return Err(ClosureUsageAuthorityError::AnchorUsagePolicyMismatch {
                expected: self.authority.usage_policy(),
                actual: request.anchor().usage_policy(),
            });
        }
        if request.anchor().tick().0 < self.authority.used_through().0 {
            return Err(ClosureUsageAuthorityError::AnchorMovesBackward {
                used_through: self.authority.used_through(),
                anchor_tick: request.anchor().tick(),
            });
        }
        let identity = (request.anchor().id(), request.anchor().revision());
        if self.seen_anchors.contains(&identity) {
            return Err(ClosureUsageAuthorityError::AnchorAlreadyUsed {
                anchor: request.anchor().id(),
                revision: request.anchor().revision(),
            });
        }

        let source_revision = self.authority.revision();
        let resulting_revision = ClosureUsageRevision(
            source_revision
                .0
                .checked_add(1)
                .ok_or(ClosureUsageAuthorityError::UsageRevisionOverflow)?,
        );
        let receipt = ClosureAnchorInstallReceipt {
            request: request.id(),
            source_revision,
            resulting_revision,
            anchor_id: request.anchor().id(),
            anchor_revision: request.anchor().revision(),
            anchor_tick: request.anchor().tick(),
        };

        self.authority.revision = resulting_revision;
        self.authority.anchor_id = request.anchor().id();
        self.authority.anchor_revision = request.anchor().revision();
        self.authority.anchor_tick = request.anchor().tick();
        self.authority.used_through = request.anchor().tick();
        self.anchor = request.anchor().clone();
        self.seen_anchors.insert(identity);
        self.committed_anchor_installs.insert(
            request.id(),
            CommittedAnchorInstall {
                request,
                receipt: receipt.clone(),
            },
        );
        Ok(receipt)
    }
}

fn validate_anchor(
    registry: &ClosureUsagePolicyRegistry,
    anchor: &ClosureValidationAnchor,
) -> Result<(), ClosureUsageAuthorityError> {
    if anchor.usage_policy_authority() != registry.authority_stamp() {
        return Err(ClosureUsageAuthorityError::AnchorPolicyAuthorityChanged);
    }
    if registry.definition(anchor.usage_policy()).is_none() {
        return Err(ClosureUsageAuthorityError::UnknownUsagePolicy {
            policy: anchor.usage_policy(),
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_state_context(
    state: &AuthoritativeClosureUsageState,
    registry: &ClosureUsagePolicyRegistry,
    information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
    closures: &TypedClosureQualificationRegistry,
    acceptances: &TypedClosureProcessAcceptanceRegistry,
    spatiotemporal: &SpatiotemporalPolicyRegistry,
) -> Result<(), ClosureUsageAuthorityError> {
    registry.validate_current(
        information_policy,
        closures,
        acceptances,
        spatiotemporal,
    )?;
    if state.authority.usage_policy_authority() != registry.authority_stamp() {
        return Err(ClosureUsageAuthorityError::UsagePolicyAuthorityChanged);
    }
    if state.anchor.usage_policy_authority() != registry.authority_stamp() {
        return Err(ClosureUsageAuthorityError::AnchorPolicyAuthorityChanged);
    }
    if state.anchor.id() != state.authority.anchor_id()
        || state.anchor.revision() != state.authority.anchor_revision()
        || state.anchor.tick() != state.authority.anchor_tick()
        || state.anchor.usage_policy() != state.authority.usage_policy()
    {
        return Err(ClosureUsageAuthorityError::AnchorStateMismatch);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum ClosureUsageAuthorityError {
    ZeroMaximumUseHorizon,
    ConflictingUsagePolicyRegistration {
        policy: ClosureUsagePolicyKey,
    },
    InformationPolicyIdentity(crate::information_policy_manifest::InformationPolicyIdentityError),
    InformationPolicyAuthorityChanged,
    InformationRegistry(crate::information_registry::InformationRegistryError),
    ClosureQualification(TypedClosureQualificationError),
    ClosureQualificationAuthorityChanged,
    ClosureAcceptance(TypedClosureProcessAcceptanceError),
    ClosureAcceptanceAuthorityChanged,
    SpatiotemporalPolicy(SpatiotemporalPolicyError),
    SpatiotemporalAuthorityChanged,
    ProcessDoesNotPermitClosure {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    UsageHorizonExceedsQualification {
        policy: ClosureUsagePolicyKey,
        maximum_use: MaximumClosureUseTicks,
        qualified_horizon: u64,
    },
    RepresentationDoesNotCarryClosureLineage {
        representation: RepresentationKey,
        information: EcologicalInformation,
        lineage: EvidenceLineageToken,
    },
    UnknownUsagePolicy {
        policy: ClosureUsagePolicyKey,
    },
    AnchorPolicyAuthorityChanged,
    AnchorUsagePolicyMismatch {
        expected: ClosureUsagePolicyKey,
        actual: ClosureUsagePolicyKey,
    },
    AnchorStateMismatch,
    InvalidUsageWindow {
        start_exclusive: CanonicalTick,
        end_inclusive: CanonicalTick,
    },
    NonContiguousUsageWindow {
        expected_start: CanonicalTick,
        actual_start: CanonicalTick,
    },
    QualifiedHorizonOverflow,
    ConflictingUsageRequestId {
        request: ClosureUsageRequestId,
    },
    PreparedUsageStale,
    PreparedUsageNoLongerAuthorized,
    UsageRevisionOverflow,
    ConflictingAnchorInstallRequestId {
        request: ClosureAnchorInstallRequestId,
    },
    AnchorMovesBackward {
        used_through: CanonicalTick,
        anchor_tick: CanonicalTick,
    },
    AnchorAlreadyUsed {
        anchor: ClosureValidationAnchorId,
        revision: ClosureValidationAnchorRevision,
    },
}

impl fmt::Display for ClosureUsageAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroMaximumUseHorizon => {
                write!(formatter, "closure usage horizon must be at least one canonical tick")
            }
            Self::ConflictingUsagePolicyRegistration { policy } => write!(
                formatter,
                "conflicting closure-usage policy registration for {}@{}",
                policy.id(), policy.version()
            ),
            Self::InformationPolicyIdentity(error) => {
                write!(formatter, "information-policy identity error: {error}")
            }
            Self::InformationPolicyAuthorityChanged => {
                write!(formatter, "exact information-policy authority changed")
            }
            Self::InformationRegistry(error) => write!(formatter, "information registry error: {error}"),
            Self::ClosureQualification(error) => {
                write!(formatter, "typed closure qualification error: {error}")
            }
            Self::ClosureQualificationAuthorityChanged => {
                write!(formatter, "typed closure qualification authority changed")
            }
            Self::ClosureAcceptance(error) => {
                write!(formatter, "typed closure process-acceptance error: {error}")
            }
            Self::ClosureAcceptanceAuthorityChanged => {
                write!(formatter, "typed closure process-acceptance authority changed")
            }
            Self::SpatiotemporalPolicy(error) => {
                write!(formatter, "spatiotemporal policy error: {error}")
            }
            Self::SpatiotemporalAuthorityChanged => {
                write!(formatter, "spatiotemporal authority changed")
            }
            Self::ProcessDoesNotPermitClosure {
                process,
                information,
            } => write!(
                formatter,
                "process {process:?} does not permit closure evidence for {information:?}"
            ),
            Self::UsageHorizonExceedsQualification {
                policy,
                maximum_use,
                qualified_horizon,
            } => write!(
                formatter,
                "closure-usage policy {}@{} requests {} ticks but qualification covers only {} ticks",
                policy.id(),
                policy.version(),
                maximum_use.get(),
                qualified_horizon
            ),
            Self::RepresentationDoesNotCarryClosureLineage {
                representation,
                information,
                lineage,
            } => write!(
                formatter,
                "representation {representation:?} does not carry closure lineage {} for {information:?}",
                lineage.0
            ),
            Self::UnknownUsagePolicy { policy } => write!(
                formatter,
                "unknown closure-usage policy {}@{}",
                policy.id(), policy.version()
            ),
            Self::AnchorPolicyAuthorityChanged => write!(
                formatter,
                "closure validation anchor was minted under different usage-policy authority"
            ),
            Self::AnchorUsagePolicyMismatch { expected, actual } => write!(
                formatter,
                "closure anchor policy mismatch: expected {}@{}, got {}@{}",
                expected.id(), expected.version(), actual.id(), actual.version()
            ),
            Self::AnchorStateMismatch => write!(
                formatter,
                "canonical closure anchor state does not match its usage authority stamp"
            ),
            Self::InvalidUsageWindow {
                start_exclusive,
                end_inclusive,
            } => write!(
                formatter,
                "closure usage window must advance canonical time: ({}, {}]",
                start_exclusive.0, end_inclusive.0
            ),
            Self::NonContiguousUsageWindow {
                expected_start,
                actual_start,
            } => write!(
                formatter,
                "closure usage must continue from authoritative tick {}; request starts at {}",
                expected_start.0, actual_start.0
            ),
            Self::QualifiedHorizonOverflow => write!(
                formatter,
                "closure validation anchor plus qualified horizon overflows canonical tick"
            ),
            Self::ConflictingUsageRequestId { request } => write!(
                formatter,
                "closure usage request id {} was reused with conflicting semantics",
                request.0
            ),
            Self::PreparedUsageStale => write!(
                formatter,
                "prepared closure usage is stale against current usage authority"
            ),
            Self::PreparedUsageNoLongerAuthorized => write!(
                formatter,
                "prepared closure usage is no longer inside the qualified anchor horizon"
            ),
            Self::UsageRevisionOverflow => write!(formatter, "closure-usage revision overflow"),
            Self::ConflictingAnchorInstallRequestId { request } => write!(
                formatter,
                "closure anchor-install request id {} was reused with conflicting semantics",
                request.0
            ),
            Self::AnchorMovesBackward {
                used_through,
                anchor_tick,
            } => write!(
                formatter,
                "new closure validation anchor at tick {} precedes already authorized use through tick {}",
                anchor_tick.0, used_through.0
            ),
            Self::AnchorAlreadyUsed { anchor, revision } => write!(
                formatter,
                "closure validation anchor {} revision {} has already been used by this authority scope",
                anchor.0, revision.0
            ),
        }
    }
}

impl Error for ClosureUsageAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InformationPolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            Self::ClosureQualification(error) => Some(error),
            Self::ClosureAcceptance(error) => Some(error),
            Self::SpatiotemporalPolicy(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        ClosureAcceptance, ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel,
        ErrorPpm, ProcessInformationProfile, ProcessInformationRequirement,
        QualifiedClosureEvidence, RepresentationCapabilities,
    };
    use crate::information_registry::{
        ClosureEvidenceStatus, InformationPolicyRegistry, InformationPolicyRegistryBuilder,
        InformationPolicyRegistryKey, RegisteredClosureEvidence,
    };
    use crate::living_world_authority::spatiotemporal_information::{
        IntegrationSemantics, MaximumStateAgeTicks, MaximumUpdateIntervalTicks,
        ProcessSpatiotemporalRequirement, RepresentationSpatiotemporalCapability,
        SpatialResolutionUnits, SpatiotemporalPolicyRegistryBuilder, SpatiotemporalPolicyRegistryKey,
    };
    use crate::living_world_authority::typed_closure_process_acceptance::{
        TypedClosureProcessAcceptance, TypedClosureProcessAcceptanceRegistryBuilder,
        TypedClosureProcessAcceptanceRegistryKey,
    };
    use crate::living_world_authority::typed_closure_qualification::{
        ClosureAggregationSemantics, ClosureErrorMetric, ClosureEvaluationHorizonTicks,
        ClosureObservableKey, ClosureQualificationImplementationFingerprint,
        ClosureQualificationProfileKey, RelativeZeroReferencePolicy, TypedClosureQualification,
        TypedClosureQualificationRegistryBuilder, TypedClosureQualificationRegistryKey,
    };

    const INFO_POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(30_000, 1);
    const PROCESS: ProcessKey = ProcessKey::new(30_010, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(30_020, 1);
    const LINEAGE: EvidenceLineageToken = EvidenceLineageToken(30_030);
    const OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(30_040, 1);
    const CLOSURE_REGISTRY: TypedClosureQualificationRegistryKey =
        TypedClosureQualificationRegistryKey::new(30_050, 1);
    const ACCEPTANCE_REGISTRY: TypedClosureProcessAcceptanceRegistryKey =
        TypedClosureProcessAcceptanceRegistryKey::new(30_060, 1);
    const ST_REGISTRY: SpatiotemporalPolicyRegistryKey =
        SpatiotemporalPolicyRegistryKey::new(30_070, 1);
    const USAGE_REGISTRY: ClosureUsagePolicyRegistryKey =
        ClosureUsagePolicyRegistryKey::new(30_080, 1);
    const USAGE_POLICY: ClosureUsagePolicyKey = ClosureUsagePolicyKey::new(30_090, 1);
    const SCOPE: ClosureUsageAuthorityScope = ClosureUsageAuthorityScope::new(30_100, 1);

    fn evidence() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(1),
            ClosureDomainToken(2),
            ErrorPpm::new(10_000).unwrap(),
            LINEAGE,
        )
    }

    fn information_policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(INFO_POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                evidence(),
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::closure_allowed(
                    EcologicalInformation::OccupancyDistribution,
                    ClosureAcceptance::new(
                        ErrorPpm::new(20_000).unwrap(),
                        Some(ClosureModelVersion(1)),
                        Some(ClosureDomainToken(2)),
                    ),
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::OccupancyDistribution,
                    CapabilityEvidence::QualifiedClosure(evidence()),
                )],
            ))
            .unwrap();
        builder.seal()
    }

    fn closures(
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        horizon: u64,
    ) -> TypedClosureQualificationRegistry {
        let mut builder = TypedClosureQualificationRegistryBuilder::new(CLOSURE_REGISTRY);
        builder
            .register(
                TypedClosureQualification::new(
                    evidence(),
                    OBSERVABLE,
                    ClosureErrorMetric::MaximumRelativePpm {
                        zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
                    },
                    ErrorPpm::new(10_000).unwrap(),
                    ClosureEvaluationHorizonTicks::new(horizon).unwrap(),
                    ClosureAggregationSemantics::MaximumOverHorizon,
                    ClosureQualificationProfileKey::new(30_110, 1),
                    ClosureQualificationImplementationFingerprint::new(b"closure-v1".to_vec())
                        .unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(information_policy).unwrap()
    }

    fn acceptances(
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> TypedClosureProcessAcceptanceRegistry {
        let mut builder =
            TypedClosureProcessAcceptanceRegistryBuilder::new(ACCEPTANCE_REGISTRY);
        builder
            .register(TypedClosureProcessAcceptance::new(
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                OBSERVABLE,
                ClosureErrorMetric::MaximumRelativePpm {
                    zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
                },
                ErrorPpm::new(10_000).unwrap(),
                ClosureEvaluationHorizonTicks::new(50).unwrap(),
                ClosureAggregationSemantics::MaximumOverHorizon,
                Some(ClosureModelVersion(1)),
                Some(ClosureDomainToken(2)),
            ))
            .unwrap();
        builder.seal(information_policy).unwrap()
    }

    fn spatiotemporal(
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> SpatiotemporalPolicyRegistry {
        let mut builder = SpatiotemporalPolicyRegistryBuilder::new(ST_REGISTRY);
        builder
            .register_process_requirement(
                ProcessSpatiotemporalRequirement::new(
                    PROCESS,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumStateAgeTicks(2),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder
            .register_representation_capability(
                RepresentationSpatiotemporalCapability::new(
                    REPRESENTATION,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(5).unwrap(),
                    MaximumUpdateIntervalTicks::new(1).unwrap(),
                    IntegrationSemantics::Instantaneous,
                    None,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(information_policy).unwrap()
    }

    struct Fixture {
        information_policy: InformationPolicyRegistry,
        closures: TypedClosureQualificationRegistry,
        acceptances: TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: SpatiotemporalPolicyRegistry,
        usage: ClosureUsagePolicyRegistry,
    }

    fn fixture(maximum_use: u64, qualification_horizon: u64) -> Fixture {
        let information_policy = information_policy();
        let bound = ManifestBoundInformationPolicyRegistry::new(&information_policy);
        let closures = closures(&bound, qualification_horizon);
        let acceptances = acceptances(&bound);
        let spatiotemporal = spatiotemporal(&bound);
        let mut builder = ClosureUsagePolicyRegistryBuilder::new(USAGE_REGISTRY);
        builder
            .register(ClosureUsagePolicyDefinition::new(
                USAGE_POLICY,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                MaximumClosureUseTicks::new(maximum_use).unwrap(),
            ))
            .unwrap();
        let usage = builder
            .seal(&bound, &closures, &acceptances, &spatiotemporal)
            .unwrap();
        Fixture {
            information_policy,
            closures,
            acceptances,
            spatiotemporal,
            usage,
        }
    }

    fn anchor(
        usage: &ClosureUsagePolicyRegistry,
        id: u128,
        revision: u64,
        tick: u64,
    ) -> ClosureValidationAnchor {
        ClosureValidationAnchor {
            id: ClosureValidationAnchorId(id),
            revision: ClosureValidationAnchorRevision(revision),
            tick: CanonicalTick(tick),
            usage_policy_authority: usage.authority_stamp().clone(),
            usage_policy: USAGE_POLICY,
        }
    }

    fn bound(fixture: &Fixture) -> ManifestBoundInformationPolicyRegistry<'_> {
        ManifestBoundInformationPolicyRegistry::new(&fixture.information_policy)
    }

    fn usage_window(start: u64, end: u64) -> ClosureUsageWindow {
        ClosureUsageWindow::new(CanonicalTick(start), CanonicalTick(end)).unwrap()
    }

    fn prepare_ready(
        state: &AuthoritativeClosureUsageState,
        fixture: &Fixture,
        request: ClosureUsageRequest,
    ) -> PreparedClosureUsage {
        let bound = bound(fixture);
        let ClosureUsagePreparationOutcome::Ready(prepared) = state
            .prepare_usage(
                request,
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap()
        else {
            panic!("expected ready closure usage");
        };
        prepared
    }

    fn commit(
        state: &mut AuthoritativeClosureUsageState,
        fixture: &Fixture,
        prepared: &PreparedClosureUsage,
    ) -> ClosureUsageReceipt {
        let bound = bound(fixture);
        state
            .commit_usage(
                prepared,
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap()
    }

    #[test]
    fn split_windows_cannot_extend_qualified_anchor_horizon() {
        let fixture = fixture(100, 100);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();

        let first = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 40)),
        );
        commit(&mut state, &fixture, &first);
        let second = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(2), usage_window(40, 100)),
        );
        commit(&mut state, &fixture, &second);

        let bound = bound(&fixture);
        let outcome = state
            .prepare_usage(
                ClosureUsageRequest::new(ClosureUsageRequestId(3), usage_window(100, 101)),
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap();
        let ClosureUsagePreparationOutcome::RevalidationRequired(requirement) = outcome else {
            panic!("horizon exhaustion must require revalidation");
        };
        assert_eq!(requirement.qualified_end(), CanonicalTick(100));
        assert_eq!(state.authority_stamp().used_through(), CanonicalTick(100));
    }

    #[test]
    fn tiny_requests_do_not_reset_the_horizon() {
        let fixture = fixture(10, 10);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 50),
        )
        .unwrap();

        for offset in 0..10 {
            let prepared = prepare_ready(
                &state,
                &fixture,
                ClosureUsageRequest::new(
                    ClosureUsageRequestId(u128::from(offset + 1)),
                    usage_window(50 + offset, 51 + offset),
                ),
            );
            commit(&mut state, &fixture, &prepared);
        }

        let bound = bound(&fixture);
        assert!(matches!(
            state
                .prepare_usage(
                    ClosureUsageRequest::new(ClosureUsageRequestId(99), usage_window(60, 61)),
                    &fixture.usage,
                    &bound,
                    &fixture.closures,
                    &fixture.acceptances,
                    &fixture.spatiotemporal,
                )
                .unwrap(),
            ClosureUsagePreparationOutcome::RevalidationRequired(_)
        ));
    }

    #[test]
    fn acknowledgement_loss_retry_does_not_consume_horizon_twice() {
        let fixture = fixture(100, 100);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();
        let request = ClosureUsageRequest::new(ClosureUsageRequestId(7), usage_window(0, 20));
        let prepared = prepare_ready(&state, &fixture, request);
        let first = commit(&mut state, &fixture, &prepared);
        let revision = state.authority_stamp().revision();

        let retry = commit(&mut state, &fixture, &prepared);
        assert_eq!(retry, first);
        assert_eq!(state.authority_stamp().revision(), revision);
        assert_eq!(state.authority_stamp().used_through(), CanonicalTick(20));
    }

    #[test]
    fn competing_prepared_windows_from_one_revision_linearize_once() {
        let fixture = fixture(100, 100);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();
        let first = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 10)),
        );
        let second = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(2), usage_window(0, 20)),
        );

        commit(&mut state, &fixture, &first);
        let bound = bound(&fixture);
        assert!(matches!(
            state.commit_usage(
                &second,
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            ),
            Err(ClosureUsageAuthorityError::PreparedUsageStale)
        ));
        assert_eq!(state.authority_stamp().used_through(), CanonicalTick(10));
    }

    #[test]
    fn exhausted_request_has_zero_usage_mutation() {
        let fixture = fixture(10, 10);
        let state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();
        let before = state.clone();
        let bound = bound(&fixture);
        let outcome = state
            .prepare_usage(
                ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 11)),
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap();
        assert!(matches!(
            outcome,
            ClosureUsagePreparationOutcome::RevalidationRequired(_)
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn new_explicit_anchor_resets_horizon_under_new_identity() {
        let fixture = fixture(10, 10);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();
        let prepared = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 10)),
        );
        commit(&mut state, &fixture, &prepared);

        let bound = bound(&fixture);
        let install = ClosureAnchorInstallRequest::new(
            ClosureAnchorInstallRequestId(2),
            anchor(&fixture.usage, 2, 0, 10),
        );
        state
            .install_anchor(
                install.clone(),
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap();
        assert_eq!(state.authority_stamp().anchor_id(), ClosureValidationAnchorId(2));

        let next = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(3), usage_window(10, 20)),
        );
        commit(&mut state, &fixture, &next);
        assert_eq!(state.authority_stamp().used_through(), CanonicalTick(20));

        let revision = state.authority_stamp().revision();
        let retry = state
            .install_anchor(
                install,
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            )
            .unwrap();
        assert_eq!(retry.resulting_revision(), ClosureUsageRevision(2));
        assert_eq!(state.authority_stamp().revision(), revision);
    }

    #[test]
    fn reused_old_anchor_cannot_rewind_or_reset_budget() {
        let fixture = fixture(10, 10);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();
        let prepared = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 5)),
        );
        commit(&mut state, &fixture, &prepared);
        let before = state.clone();
        let bound = bound(&fixture);
        assert!(matches!(
            state.install_anchor(
                ClosureAnchorInstallRequest::new(
                    ClosureAnchorInstallRequestId(2),
                    anchor(&fixture.usage, 1, 0, 5),
                ),
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            ),
            Err(ClosureUsageAuthorityError::AnchorAlreadyUsed { .. })
        ));
        assert_eq!(state, before);
    }

    #[test]
    fn policy_cannot_exceed_qualified_horizon() {
        let information_policy = information_policy();
        let bound = ManifestBoundInformationPolicyRegistry::new(&information_policy);
        let closures = closures(&bound, 10);
        let acceptances = acceptances(&bound);
        let spatiotemporal = spatiotemporal(&bound);
        let mut builder = ClosureUsagePolicyRegistryBuilder::new(USAGE_REGISTRY);
        builder
            .register(ClosureUsagePolicyDefinition::new(
                USAGE_POLICY,
                PROCESS,
                EcologicalInformation::OccupancyDistribution,
                LINEAGE,
                REPRESENTATION,
                MaximumClosureUseTicks::new(11).unwrap(),
            ))
            .unwrap();
        assert!(matches!(
            builder.seal(&bound, &closures, &acceptances, &spatiotemporal),
            Err(ClosureUsageAuthorityError::UsageHorizonExceedsQualification { .. })
        ));
    }

    #[test]
    fn changed_exact_closure_corpus_stales_existing_usage_state() {
        let fixture = fixture(100, 100);
        let state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 0),
        )
        .unwrap();

        let bound = bound(&fixture);
        let changed_closures = closures(&bound, 101);
        assert!(matches!(
            state.prepare_usage(
                ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(0, 1)),
                &fixture.usage,
                &bound,
                &changed_closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            ),
            Err(ClosureUsageAuthorityError::ClosureQualificationAuthorityChanged)
                | Err(ClosureUsageAuthorityError::UsagePolicyAuthorityChanged)
        ));
    }

    #[test]
    fn anchor_type_has_no_public_reset_semantics_and_backward_anchor_rejects() {
        let fixture = fixture(100, 100);
        let mut state = AuthoritativeClosureUsageState::bootstrap(
            SCOPE,
            &fixture.usage,
            anchor(&fixture.usage, 1, 0, 50),
        )
        .unwrap();
        let prepared = prepare_ready(
            &state,
            &fixture,
            ClosureUsageRequest::new(ClosureUsageRequestId(1), usage_window(50, 60)),
        );
        commit(&mut state, &fixture, &prepared);
        let before = state.clone();
        let bound = bound(&fixture);
        assert!(matches!(
            state.install_anchor(
                ClosureAnchorInstallRequest::new(
                    ClosureAnchorInstallRequestId(2),
                    anchor(&fixture.usage, 2, 0, 59),
                ),
                &fixture.usage,
                &bound,
                &fixture.closures,
                &fixture.acceptances,
                &fixture.spatiotemporal,
            ),
            Err(ClosureUsageAuthorityError::AnchorMovesBackward { .. })
        ));
        assert_eq!(state, before);
    }
}
