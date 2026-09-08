// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Registry-owned authority for ecological information policy.
//!
//! [`crate::information`] intentionally exposes a pure policy algebra whose
//! values are easy to construct for tests and measurement. Canonical runtime
//! code must not treat arbitrary caller-constructed profiles as proof. This
//! module provides the next boundary: a sealed registry instance owns the
//! process profiles, representation capabilities, and closure-evidence records
//! that the runtime has elected to trust.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{
    evaluate_processes, CapabilityEvidence, EvidenceLineageToken, ProcessInformationProfile,
    ProcessKey, QualifiedClosureEvidence, RepresentationCapabilities, RepresentationKey,
    SufficiencyReport,
};

/// Identity/version of one authoritative information-policy registry.
///
/// This is an inspectable replay key, not a cryptographic credential. Authority
/// comes from the canonical runtime owning a particular sealed registry instance
/// and refusing caller-supplied substitutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InformationPolicyRegistryKey {
    id: u128,
    version: u32,
}

impl InformationPolicyRegistryKey {
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

/// Whether one recognized closure-evidence lineage is currently authoritative.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureEvidenceStatus {
    Qualified,
    Revoked,
}

/// Registry record for one closure-evidence lineage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RegisteredClosureEvidence {
    evidence: QualifiedClosureEvidence,
    status: ClosureEvidenceStatus,
}

impl RegisteredClosureEvidence {
    pub const fn new(
        evidence: QualifiedClosureEvidence,
        status: ClosureEvidenceStatus,
    ) -> Self {
        Self { evidence, status }
    }

    pub const fn evidence(self) -> QualifiedClosureEvidence {
        self.evidence
    }

    pub const fn status(self) -> ClosureEvidenceStatus {
        self.status
    }

    pub const fn lineage(self) -> EvidenceLineageToken {
        self.evidence.evidence_lineage()
    }
}

/// Bootstrap-only mutable builder for one registry generation.
///
/// Call sites that perform ecological work should receive only the sealed
/// [`InformationPolicyRegistry`]. Building a different registry does not alter
/// the registry instance owned by the canonical runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationPolicyRegistryBuilder {
    key: InformationPolicyRegistryKey,
    processes: BTreeMap<ProcessKey, ProcessInformationProfile>,
    representations: BTreeMap<RepresentationKey, RepresentationCapabilities>,
    closure_evidence: BTreeMap<EvidenceLineageToken, RegisteredClosureEvidence>,
}

impl InformationPolicyRegistryBuilder {
    pub const fn new(key: InformationPolicyRegistryKey) -> Self {
        Self {
            key,
            processes: BTreeMap::new(),
            representations: BTreeMap::new(),
            closure_evidence: BTreeMap::new(),
        }
    }

    /// Register one process contract. Identical retries are idempotent;
    /// conflicting reuse of the same key fails closed.
    pub fn register_process(
        &mut self,
        profile: ProcessInformationProfile,
    ) -> Result<(), InformationRegistryError> {
        let key = profile.key();
        if let Some(existing) = self.processes.get(&key) {
            if existing == &profile {
                return Ok(());
            }
            return Err(InformationRegistryError::ConflictingProcessRegistration { key });
        }
        self.processes.insert(key, profile);
        Ok(())
    }

    /// Register one representation capability contract. Identical retries are
    /// idempotent; conflicting reuse of the same key fails closed.
    pub fn register_representation(
        &mut self,
        capabilities: RepresentationCapabilities,
    ) -> Result<(), InformationRegistryError> {
        let key = capabilities.key();
        if let Some(existing) = self.representations.get(&key) {
            if existing == &capabilities {
                return Ok(());
            }
            return Err(InformationRegistryError::ConflictingRepresentationRegistration {
                key,
            });
        }
        self.representations.insert(key, capabilities);
        Ok(())
    }

    /// Register authoritative status for one closure-evidence lineage.
    ///
    /// A registry generation represents one settled view. Changing qualified ↔
    /// revoked status requires a new registry version rather than relying on
    /// bootstrap call ordering.
    pub fn register_closure_evidence(
        &mut self,
        record: RegisteredClosureEvidence,
    ) -> Result<(), InformationRegistryError> {
        let lineage = record.lineage();
        if let Some(existing) = self.closure_evidence.get(&lineage) {
            if existing == &record {
                return Ok(());
            }
            return Err(InformationRegistryError::ConflictingClosureEvidenceRegistration {
                lineage,
            });
        }
        self.closure_evidence.insert(lineage, record);
        Ok(())
    }

    /// Seal bootstrap state into the immutable runtime authority object.
    pub fn seal(self) -> InformationPolicyRegistry {
        InformationPolicyRegistry {
            key: self.key,
            processes: self.processes,
            representations: self.representations,
            closure_evidence: self.closure_evidence,
        }
    }
}

/// Immutable registry used by canonical process-sufficiency gates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationPolicyRegistry {
    key: InformationPolicyRegistryKey,
    processes: BTreeMap<ProcessKey, ProcessInformationProfile>,
    representations: BTreeMap<RepresentationKey, RepresentationCapabilities>,
    closure_evidence: BTreeMap<EvidenceLineageToken, RegisteredClosureEvidence>,
}

impl InformationPolicyRegistry {
    pub const fn key(&self) -> InformationPolicyRegistryKey {
        self.key
    }

    pub fn process_profiles(
        &self,
    ) -> impl Iterator<Item = (&ProcessKey, &ProcessInformationProfile)> {
        self.processes.iter()
    }

    pub fn representation_profiles(
        &self,
    ) -> impl Iterator<Item = (&RepresentationKey, &RepresentationCapabilities)> {
        self.representations.iter()
    }

    pub fn closure_evidence_records(
        &self,
    ) -> impl Iterator<Item = (&EvidenceLineageToken, &RegisteredClosureEvidence)> {
        self.closure_evidence.iter()
    }

    pub fn resolve_process(
        &self,
        key: ProcessKey,
    ) -> Result<ResolvedProcessProfile<'_>, InformationRegistryError> {
        let profile = self
            .processes
            .get(&key)
            .ok_or(InformationRegistryError::UnknownProcess { key })?;
        Ok(ResolvedProcessProfile {
            registry_key: self.key,
            profile,
        })
    }

    pub fn resolve_representation(
        &self,
        key: RepresentationKey,
    ) -> Result<ResolvedRepresentationCapabilities<'_>, InformationRegistryError> {
        let capabilities = self
            .representations
            .get(&key)
            .ok_or(InformationRegistryError::UnknownRepresentation { key })?;

        for evidence in capabilities.claims().values().flatten().copied() {
            if let CapabilityEvidence::QualifiedClosure(closure) = evidence {
                self.validate_closure_evidence(closure)?;
            }
        }

        Ok(ResolvedRepresentationCapabilities {
            registry_key: self.key,
            capabilities,
        })
    }

    /// Resolve authoritative profiles by descriptor and evaluate them through
    /// the pure information algebra.
    ///
    /// Callers can choose descriptors, but cannot inject substitute profiles or
    /// capability maps into this operation.
    pub fn evaluate_registered(
        &self,
        processes: impl IntoIterator<Item = ProcessKey>,
        representation: RepresentationKey,
    ) -> Result<RegisteredSufficiencyReport, InformationRegistryError> {
        let process_keys = processes.into_iter().collect::<BTreeSet<_>>();
        let resolved_representation = self.resolve_representation(representation)?;

        let mut profiles = Vec::with_capacity(process_keys.len());
        for process in &process_keys {
            profiles.push(self.resolve_process(*process)?.profile());
        }

        let report = evaluate_processes(profiles, resolved_representation.capabilities());
        Ok(RegisteredSufficiencyReport {
            registry_key: self.key,
            process_keys,
            representation,
            report,
        })
    }

    fn validate_closure_evidence(
        &self,
        closure: QualifiedClosureEvidence,
    ) -> Result<(), InformationRegistryError> {
        let lineage = closure.evidence_lineage();
        let record = self
            .closure_evidence
            .get(&lineage)
            .ok_or(InformationRegistryError::UnknownClosureEvidence { lineage })?;

        if record.evidence() != closure {
            return Err(InformationRegistryError::ClosureEvidenceMismatch { lineage });
        }
        if record.status() == ClosureEvidenceStatus::Revoked {
            return Err(InformationRegistryError::ClosureEvidenceRevoked { lineage });
        }
        Ok(())
    }
}

/// Process profile resolved from one specific sealed registry generation.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedProcessProfile<'a> {
    registry_key: InformationPolicyRegistryKey,
    profile: &'a ProcessInformationProfile,
}

impl<'a> ResolvedProcessProfile<'a> {
    pub const fn registry_key(self) -> InformationPolicyRegistryKey {
        self.registry_key
    }

    pub const fn profile(self) -> &'a ProcessInformationProfile {
        self.profile
    }
}

/// Representation capabilities resolved from one specific sealed registry.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedRepresentationCapabilities<'a> {
    registry_key: InformationPolicyRegistryKey,
    capabilities: &'a RepresentationCapabilities,
}

impl<'a> ResolvedRepresentationCapabilities<'a> {
    pub const fn registry_key(self) -> InformationPolicyRegistryKey {
        self.registry_key
    }

    pub const fn capabilities(self) -> &'a RepresentationCapabilities {
        self.capabilities
    }
}

/// Sufficiency evidence bound to the registry generation and descriptors that
/// produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredSufficiencyReport {
    registry_key: InformationPolicyRegistryKey,
    process_keys: BTreeSet<ProcessKey>,
    representation: RepresentationKey,
    report: SufficiencyReport,
}

impl RegisteredSufficiencyReport {
    pub const fn registry_key(&self) -> InformationPolicyRegistryKey {
        self.registry_key
    }

    pub fn process_keys(&self) -> &BTreeSet<ProcessKey> {
        &self.process_keys
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn report(&self) -> &SufficiencyReport {
        &self.report
    }

    pub fn is_sufficient(&self) -> bool {
        self.report.is_sufficient()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationRegistryError {
    UnknownProcess { key: ProcessKey },
    UnknownRepresentation { key: RepresentationKey },
    UnknownClosureEvidence { lineage: EvidenceLineageToken },
    ClosureEvidenceMismatch { lineage: EvidenceLineageToken },
    ClosureEvidenceRevoked { lineage: EvidenceLineageToken },
    ConflictingProcessRegistration { key: ProcessKey },
    ConflictingRepresentationRegistration { key: RepresentationKey },
    ConflictingClosureEvidenceRegistration { lineage: EvidenceLineageToken },
}

impl fmt::Display for InformationRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownProcess { key } => write!(
                formatter,
                "unknown ecological process descriptor {}@{}",
                key.id(),
                key.version()
            ),
            Self::UnknownRepresentation { key } => write!(
                formatter,
                "unknown ecological representation descriptor {}@{}",
                key.id(),
                key.version()
            ),
            Self::UnknownClosureEvidence { lineage } => write!(
                formatter,
                "unknown closure evidence lineage {}",
                lineage.0
            ),
            Self::ClosureEvidenceMismatch { lineage } => write!(
                formatter,
                "closure evidence lineage {} does not match its registry record",
                lineage.0
            ),
            Self::ClosureEvidenceRevoked { lineage } => write!(
                formatter,
                "closure evidence lineage {} is revoked",
                lineage.0
            ),
            Self::ConflictingProcessRegistration { key } => write!(
                formatter,
                "conflicting process registration for {}@{}",
                key.id(),
                key.version()
            ),
            Self::ConflictingRepresentationRegistration { key } => write!(
                formatter,
                "conflicting representation registration for {}@{}",
                key.id(),
                key.version()
            ),
            Self::ConflictingClosureEvidenceRegistration { lineage } => write!(
                formatter,
                "conflicting closure evidence registration for lineage {}",
                lineage.0
            ),
        }
    }
}

impl Error for InformationRegistryError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        ClosureAcceptance, ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel,
        EcologicalInformation, ErrorPpm, EvidenceRequirement, PopulationStatisticSet,
        ProcessInformationRequirement,
    };

    const REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(99, 1);
    const DROUGHT: ProcessKey = ProcessKey::new(1, 1);
    const DISEASE: ProcessKey = ProcessKey::new(2, 1);
    const MARGINALS: RepresentationKey = RepresentationKey::new(10, 1);

    fn exact_process(
        key: ProcessKey,
        information: EcologicalInformation,
    ) -> ProcessInformationProfile {
        ProcessInformationProfile::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::exact(information)],
        )
    }

    fn closure_evidence(lineage: u128) -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(4),
            ClosureDomainToken(55),
            ErrorPpm::new(1_000).unwrap(),
            EvidenceLineageToken(lineage),
        )
    }

    fn closure_process() -> ProcessInformationProfile {
        ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::closure_allowed(
                EcologicalInformation::AgeDistribution,
                ClosureAcceptance::new(
                    ErrorPpm::new(10_000).unwrap(),
                    Some(ClosureModelVersion(4)),
                    Some(ClosureDomainToken(55)),
                ),
            )],
        )
    }

    fn build_exact_registry() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_process(exact_process(
                DROUGHT,
                EcologicalInformation::AgeDistribution,
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        builder.seal()
    }

    #[test]
    fn registered_profiles_authorize_normal_evaluation() {
        let registry = build_exact_registry();
        let report = registry.evaluate_registered([DROUGHT], MARGINALS).unwrap();

        assert!(report.is_sufficient());
        assert_eq!(report.registry_key(), REGISTRY);
        assert_eq!(report.representation(), MARGINALS);
        assert_eq!(report.process_keys(), &BTreeSet::from([DROUGHT]));
    }

    #[test]
    fn unknown_process_or_representation_fails_closed() {
        let registry = build_exact_registry();

        assert_eq!(
            registry.evaluate_registered([DISEASE], MARGINALS),
            Err(InformationRegistryError::UnknownProcess { key: DISEASE })
        );
        assert_eq!(
            registry.evaluate_registered([DROUGHT], RepresentationKey::new(999, 1)),
            Err(InformationRegistryError::UnknownRepresentation {
                key: RepresentationKey::new(999, 1)
            })
        );
    }

    #[test]
    fn caller_capability_self_assertion_cannot_change_registered_truth() {
        let registry = build_exact_registry();
        let disease = exact_process(
            DISEASE,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        let fake_richer_representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Persistent,
            [(
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
                CapabilityEvidence::Exact,
            )],
        );

        assert!(evaluate_processes([&disease], &fake_richer_representation).is_sufficient());
        assert_eq!(
            registry.evaluate_registered([DISEASE], MARGINALS),
            Err(InformationRegistryError::UnknownProcess { key: DISEASE })
        );
    }

    #[test]
    fn caller_weakened_process_profile_cannot_replace_registered_profile() {
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        let registered = exact_process(
            DISEASE,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        builder.register_process(registered).unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        let registry = builder.seal();

        let weakened = exact_process(DISEASE, EcologicalInformation::AgeDistribution);
        assert!(evaluate_processes(
            [&weakened],
            registry.resolve_representation(MARGINALS).unwrap().capabilities()
        )
        .is_sufficient());
        assert!(!registry
            .evaluate_registered([DISEASE], MARGINALS)
            .unwrap()
            .is_sufficient());
    }

    #[test]
    fn unknown_or_revoked_closure_evidence_rejects_before_evaluation() {
        let evidence = closure_evidence(500);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::QualifiedClosure(evidence),
            )],
        );

        let mut unknown_builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        unknown_builder.register_process(closure_process()).unwrap();
        unknown_builder
            .register_representation(representation.clone())
            .unwrap();
        let unknown_registry = unknown_builder.seal();
        assert_eq!(
            unknown_registry.evaluate_registered([DROUGHT], MARGINALS),
            Err(InformationRegistryError::UnknownClosureEvidence {
                lineage: EvidenceLineageToken(500)
            })
        );

        let mut revoked_builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        revoked_builder.register_process(closure_process()).unwrap();
        revoked_builder
            .register_representation(representation)
            .unwrap();
        revoked_builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                evidence,
                ClosureEvidenceStatus::Revoked,
            ))
            .unwrap();
        let revoked_registry = revoked_builder.seal();
        assert_eq!(
            revoked_registry.evaluate_registered([DROUGHT], MARGINALS),
            Err(InformationRegistryError::ClosureEvidenceRevoked {
                lineage: EvidenceLineageToken(500)
            })
        );
    }

    #[test]
    fn same_lineage_with_different_evidence_is_rejected() {
        let claimed = closure_evidence(500);
        let registered = QualifiedClosureEvidence::new(
            ClosureModelVersion(5),
            ClosureDomainToken(55),
            ErrorPpm::new(1_000).unwrap(),
            EvidenceLineageToken(500),
        );
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder.register_process(closure_process()).unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::QualifiedClosure(claimed),
                )],
            ))
            .unwrap();
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                registered,
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        let registry = builder.seal();

        assert_eq!(
            registry.evaluate_registered([DROUGHT], MARGINALS),
            Err(InformationRegistryError::ClosureEvidenceMismatch {
                lineage: EvidenceLineageToken(500)
            })
        );
    }

    #[test]
    fn qualified_registered_closure_can_authorize_permitted_process() {
        let evidence = closure_evidence(500);
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder.register_process(closure_process()).unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::QualifiedClosure(evidence),
                )],
            ))
            .unwrap();
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                evidence,
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        let registry = builder.seal();

        assert!(registry
            .evaluate_registered([DROUGHT], MARGINALS)
            .unwrap()
            .is_sufficient());
    }

    #[test]
    fn conflicting_key_reuse_fails_while_identical_retry_is_idempotent() {
        let first = exact_process(DROUGHT, EcologicalInformation::AgeDistribution);
        let conflicting = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Active,
            [ProcessInformationRequirement::exact(
                EcologicalInformation::IndividualActiveState,
            )],
        );
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);

        builder.register_process(first.clone()).unwrap();
        builder.register_process(first).unwrap();
        assert_eq!(
            builder.register_process(conflicting),
            Err(InformationRegistryError::ConflictingProcessRegistration {
                key: DROUGHT
            })
        );
    }

    #[test]
    fn registration_order_does_not_change_sealed_registry() {
        let process = exact_process(DROUGHT, EcologicalInformation::AgeDistribution);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::Exact,
            )],
        );
        let evidence = RegisteredClosureEvidence::new(
            closure_evidence(500),
            ClosureEvidenceStatus::Qualified,
        );

        let mut a = InformationPolicyRegistryBuilder::new(REGISTRY);
        a.register_process(process.clone()).unwrap();
        a.register_representation(representation.clone()).unwrap();
        a.register_closure_evidence(evidence).unwrap();

        let mut b = InformationPolicyRegistryBuilder::new(REGISTRY);
        b.register_closure_evidence(evidence).unwrap();
        b.register_representation(representation).unwrap();
        b.register_process(process).unwrap();

        assert_eq!(a.seal(), b.seal());
    }

    #[test]
    fn resolved_views_are_bound_to_registry_identity() {
        let registry = build_exact_registry();

        assert_eq!(registry.resolve_process(DROUGHT).unwrap().registry_key(), REGISTRY);
        assert_eq!(
            registry
                .resolve_representation(MARGINALS)
                .unwrap()
                .registry_key(),
            REGISTRY
        );
    }

    #[test]
    fn registry_does_not_turn_measurement_only_into_authority() {
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        let process = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::closure_allowed(
                EcologicalInformation::AgeDistribution,
                ClosureAcceptance::new(ErrorPpm::ONE_PERCENT, None, None),
            )],
        );
        builder.register_process(process).unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                MARGINALS,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::AgeDistribution,
                    CapabilityEvidence::MeasurementOnly,
                )],
            ))
            .unwrap();
        let registry = builder.seal();

        assert!(!registry
            .evaluate_registered([DROUGHT], MARGINALS)
            .unwrap()
            .is_sufficient());
    }

    #[test]
    fn process_requirement_metadata_remains_inspectable() {
        let registry = build_exact_registry();
        let profile = registry.resolve_process(DROUGHT).unwrap();
        let requirement = profile.profile().requirements().iter().next().unwrap();

        assert_eq!(requirement.evidence(), EvidenceRequirement::Exact);
    }
}
