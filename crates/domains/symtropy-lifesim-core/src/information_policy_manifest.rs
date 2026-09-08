// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact content identity for sealed ecological information-policy registries.
//!
//! A semantic [`InformationPolicyRegistryKey`] names a policy lineage/version, but
//! it does not prove the exact process, representation, or closure-evidence
//! corpus behind that name. This module defines a canonical V1 byte manifest of
//! the sealed registry and uses those bytes themselves as exact content identity.
//!
//! No cryptographic digest is invented here. Higher authority layers may derive a
//! compact digest from these canonical bytes using a separately qualified fixed
//! algorithm, while canonical equality in this low-level crate remains collision
//! free with respect to the encoded V1 policy semantics.

use std::error::Error;
use std::fmt;

use crate::conservation::ConservedQuantity;
use crate::information::{
    CapabilityEvidence, ClosureAcceptance, EcologicalAuthorityLevel, EcologicalInformation,
    EvidenceRequirement, ProcessInformationProfile, ProcessKey, QualifiedClosureEvidence,
    RepresentationCapabilities, RepresentationKey,
};
use crate::information_registry::{
    ClosureEvidenceStatus, InformationPolicyRegistry, InformationPolicyRegistryKey,
    InformationRegistryError, RegisteredClosureEvidence, RegisteredSufficiencyReport,
};

/// Canonical encoding version for the information-policy manifest.
pub const INFORMATION_POLICY_MANIFEST_VERSION: u32 = 1;

const MANIFEST_DOMAIN: &[u8] = b"SYMTROPY_INFORMATION_POLICY_MANIFEST\0";

/// Canonical, portable byte identity for one exact sealed registry corpus.
///
/// Equality compares the complete canonical bytes. The manifest is intentionally
/// not a short hash: no collision argument is required at this layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationPolicyManifest {
    bytes: Vec<u8>,
}

impl InformationPolicyManifest {
    /// Derive the exact V1 manifest from one sealed registry.
    pub fn from_registry(registry: &InformationPolicyRegistry) -> Self {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(MANIFEST_DOMAIN);
        push_u32(&mut bytes, INFORMATION_POLICY_MANIFEST_VERSION);
        encode_registry_key(&mut bytes, registry.key());

        let processes = registry.process_profiles().collect::<Vec<_>>();
        push_len(&mut bytes, processes.len());
        for (_, profile) in processes {
            encode_process_profile(&mut bytes, profile);
        }

        let representations = registry.representation_profiles().collect::<Vec<_>>();
        push_len(&mut bytes, representations.len());
        for (_, capabilities) in representations {
            encode_representation_capabilities(&mut bytes, capabilities);
        }

        let closure_records = registry.closure_evidence_records().collect::<Vec<_>>();
        push_len(&mut bytes, closure_records.len());
        for (_, record) in closure_records {
            encode_registered_closure_evidence(&mut bytes, *record);
        }

        Self { bytes }
    }

    pub const fn version(&self) -> u32 {
        INFORMATION_POLICY_MANIFEST_VERSION
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Exact authority identity for one sealed information-policy corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationPolicyAuthorityStamp {
    key: InformationPolicyRegistryKey,
    manifest: InformationPolicyManifest,
}

impl InformationPolicyAuthorityStamp {
    pub fn from_registry(registry: &InformationPolicyRegistry) -> Self {
        Self {
            key: registry.key(),
            manifest: InformationPolicyManifest::from_registry(registry),
        }
    }

    pub const fn key(&self) -> InformationPolicyRegistryKey {
        self.key
    }

    pub const fn manifest_version(&self) -> u32 {
        INFORMATION_POLICY_MANIFEST_VERSION
    }

    pub const fn manifest(&self) -> &InformationPolicyManifest {
        &self.manifest
    }

    /// Validate that this exact authority stamp still names the supplied sealed
    /// registry corpus. Same semantic key with changed contents fails closed.
    pub fn validate_registry(
        &self,
        registry: &InformationPolicyRegistry,
    ) -> Result<(), InformationPolicyIdentityError> {
        if registry.key() != self.key {
            return Err(InformationPolicyIdentityError::RegistryKeyMismatch {
                expected: self.key,
                actual: registry.key(),
            });
        }

        let actual = InformationPolicyManifest::from_registry(registry);
        if actual != self.manifest {
            return Err(InformationPolicyIdentityError::ManifestMismatch { key: self.key });
        }
        Ok(())
    }
}

/// Read-only exact-content view over one sealed registry.
///
/// Canonical orchestration that needs replay-safe policy identity should carry
/// this view (or its authority stamp) rather than relying on the semantic key
/// alone.
#[derive(Debug, Clone)]
pub struct ManifestBoundInformationPolicyRegistry<'a> {
    registry: &'a InformationPolicyRegistry,
    authority: InformationPolicyAuthorityStamp,
}

impl<'a> ManifestBoundInformationPolicyRegistry<'a> {
    pub fn new(registry: &'a InformationPolicyRegistry) -> Self {
        Self {
            registry,
            authority: InformationPolicyAuthorityStamp::from_registry(registry),
        }
    }

    pub const fn registry(&self) -> &'a InformationPolicyRegistry {
        self.registry
    }

    pub const fn authority_stamp(&self) -> &InformationPolicyAuthorityStamp {
        &self.authority
    }

    /// Evaluate registered policy and bind the result to the exact policy corpus
    /// that produced it.
    pub fn evaluate_registered(
        &self,
        processes: impl IntoIterator<Item = ProcessKey>,
        representation: RepresentationKey,
    ) -> Result<ManifestBoundSufficiencyReport, InformationRegistryError> {
        let report = self
            .registry
            .evaluate_registered(processes, representation)?;
        Ok(ManifestBoundSufficiencyReport {
            authority: self.authority.clone(),
            report,
        })
    }
}

/// Sufficiency evidence bound to exact policy content rather than semantic key
/// alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestBoundSufficiencyReport {
    authority: InformationPolicyAuthorityStamp,
    report: RegisteredSufficiencyReport,
}

impl ManifestBoundSufficiencyReport {
    pub const fn authority_stamp(&self) -> &InformationPolicyAuthorityStamp {
        &self.authority
    }

    pub const fn report(&self) -> &RegisteredSufficiencyReport {
        &self.report
    }

    pub fn is_sufficient(&self) -> bool {
        self.report.is_sufficient()
    }

    /// Revalidate the policy corpus before a later authority boundary consumes
    /// this report.
    pub fn validate_registry(
        &self,
        registry: &InformationPolicyRegistry,
    ) -> Result<(), InformationPolicyIdentityError> {
        self.authority.validate_registry(registry)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationPolicyIdentityError {
    RegistryKeyMismatch {
        expected: InformationPolicyRegistryKey,
        actual: InformationPolicyRegistryKey,
    },
    ManifestMismatch {
        key: InformationPolicyRegistryKey,
    },
}

impl fmt::Display for InformationPolicyIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "information-policy registry key mismatch: expected {}@{}, got {}@{}",
                expected.id(),
                expected.version(),
                actual.id(),
                actual.version()
            ),
            Self::ManifestMismatch { key } => write!(
                formatter,
                "information-policy registry {}@{} has different exact manifest content",
                key.id(),
                key.version()
            ),
        }
    }
}

impl Error for InformationPolicyIdentityError {}

fn push_len(bytes: &mut Vec<u8>, len: usize) {
    let len = u64::try_from(len).expect("information-policy manifest count exceeds u64");
    bytes.extend_from_slice(&len.to_le_bytes());
}

fn push_u8(bytes: &mut Vec<u8>, value: u8) {
    bytes.push(value);
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u128(bytes: &mut Vec<u8>, value: u128) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn encode_registry_key(bytes: &mut Vec<u8>, key: InformationPolicyRegistryKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_process_key(bytes: &mut Vec<u8>, key: ProcessKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_representation_key(bytes: &mut Vec<u8>, key: RepresentationKey) {
    push_u128(bytes, key.id());
    push_u32(bytes, key.version());
}

fn encode_authority_level(bytes: &mut Vec<u8>, authority: EcologicalAuthorityLevel) {
    let tag = match authority {
        EcologicalAuthorityLevel::Presentation => 0,
        EcologicalAuthorityLevel::Coarse => 1,
        EcologicalAuthorityLevel::Active => 2,
        EcologicalAuthorityLevel::Persistent => 3,
    };
    push_u8(bytes, tag);
}

fn encode_conserved_quantity(bytes: &mut Vec<u8>, quantity: ConservedQuantity) {
    let tag = match quantity {
        ConservedQuantity::BiomassMass => 0,
        ConservedQuantity::CarbonMass => 1,
        ConservedQuantity::WaterMass => 2,
        ConservedQuantity::MineralNutrientMass => 3,
    };
    push_u8(bytes, tag);
}

fn encode_information(bytes: &mut Vec<u8>, information: EcologicalInformation) {
    match information {
        EcologicalInformation::Headcount => push_u8(bytes, 0),
        EcologicalInformation::ExactLivingBiomass => push_u8(bytes, 1),
        EcologicalInformation::AgeDistribution => push_u8(bytes, 2),
        EcologicalInformation::ConditionDistribution => push_u8(bytes, 3),
        EcologicalInformation::OccupancyDistribution => push_u8(bytes, 4),
        EcologicalInformation::JointPopulationStatistics(statistics) => {
            push_u8(bytes, 5);
            push_u16(bytes, statistics.bits());
        }
        EcologicalInformation::SpatialStructure => push_u8(bytes, 6),
        EcologicalInformation::DiseaseState => push_u8(bytes, 7),
        EcologicalInformation::GeneticSummary => push_u8(bytes, 8),
        EcologicalInformation::ContactStructure => push_u8(bytes, 9),
        EcologicalInformation::IndividualActiveState => push_u8(bytes, 10),
        EcologicalInformation::PersistentIdentity => push_u8(bytes, 11),
        EcologicalInformation::ExactConservationAccount(quantity) => {
            push_u8(bytes, 12);
            encode_conserved_quantity(bytes, quantity);
        }
    }
}

fn encode_optional_u32(bytes: &mut Vec<u8>, value: Option<u32>) {
    match value {
        None => push_u8(bytes, 0),
        Some(value) => {
            push_u8(bytes, 1);
            push_u32(bytes, value);
        }
    }
}

fn encode_optional_u128(bytes: &mut Vec<u8>, value: Option<u128>) {
    match value {
        None => push_u8(bytes, 0),
        Some(value) => {
            push_u8(bytes, 1);
            push_u128(bytes, value);
        }
    }
}

fn encode_closure_acceptance(bytes: &mut Vec<u8>, acceptance: ClosureAcceptance) {
    push_u32(bytes, acceptance.max_error_ppm().get());
    encode_optional_u32(
        bytes,
        acceptance.required_model().map(|version| version.0),
    );
    encode_optional_u128(
        bytes,
        acceptance.required_domain().map(|domain| domain.0),
    );
}

fn encode_evidence_requirement(bytes: &mut Vec<u8>, evidence: EvidenceRequirement) {
    match evidence {
        EvidenceRequirement::Exact => push_u8(bytes, 0),
        EvidenceRequirement::ExactOrQualifiedClosure(acceptance) => {
            push_u8(bytes, 1);
            encode_closure_acceptance(bytes, acceptance);
        }
    }
}

fn encode_process_profile(bytes: &mut Vec<u8>, profile: &ProcessInformationProfile) {
    encode_process_key(bytes, profile.key());
    encode_authority_level(bytes, profile.minimum_authority());
    push_len(bytes, profile.requirements().len());
    for requirement in profile.requirements() {
        encode_information(bytes, requirement.information());
        encode_evidence_requirement(bytes, requirement.evidence());
    }
}

fn encode_qualified_closure(bytes: &mut Vec<u8>, closure: QualifiedClosureEvidence) {
    push_u32(bytes, closure.model_version().0);
    push_u128(bytes, closure.domain().0);
    push_u32(bytes, closure.error_ppm().get());
    push_u128(bytes, closure.evidence_lineage().0);
}

fn encode_capability_evidence(bytes: &mut Vec<u8>, evidence: CapabilityEvidence) {
    match evidence {
        CapabilityEvidence::Exact => push_u8(bytes, 0),
        CapabilityEvidence::QualifiedClosure(closure) => {
            push_u8(bytes, 1);
            encode_qualified_closure(bytes, closure);
        }
        CapabilityEvidence::MeasurementOnly => push_u8(bytes, 2),
    }
}

fn encode_representation_capabilities(
    bytes: &mut Vec<u8>,
    capabilities: &RepresentationCapabilities,
) {
    encode_representation_key(bytes, capabilities.key());
    encode_authority_level(bytes, capabilities.authority_level());
    push_len(bytes, capabilities.claims().len());
    for (information, evidence_set) in capabilities.claims() {
        encode_information(bytes, *information);
        push_len(bytes, evidence_set.len());
        for evidence in evidence_set {
            encode_capability_evidence(bytes, *evidence);
        }
    }
}

fn encode_registered_closure_evidence(bytes: &mut Vec<u8>, record: RegisteredClosureEvidence) {
    encode_qualified_closure(bytes, record.evidence());
    push_u8(
        bytes,
        match record.status() {
            ClosureEvidenceStatus::Qualified => 0,
            ClosureEvidenceStatus::Revoked => 1,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        ClosureDomainToken, ClosureModelVersion, ErrorPpm, EvidenceLineageToken,
        PopulationStatisticSet, ProcessInformationRequirement,
    };
    use crate::information_registry::InformationPolicyRegistryBuilder;

    const REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(70, 3);
    const OTHER_REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(71, 3);
    const PROCESS_A: ProcessKey = ProcessKey::new(1, 1);
    const PROCESS_B: ProcessKey = ProcessKey::new(2, 1);
    const REPRESENTATION_A: RepresentationKey = RepresentationKey::new(10, 1);
    const REPRESENTATION_B: RepresentationKey = RepresentationKey::new(11, 1);

    fn process(
        key: ProcessKey,
        information: EcologicalInformation,
    ) -> ProcessInformationProfile {
        ProcessInformationProfile::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::exact(information)],
        )
    }

    fn representation(
        key: RepresentationKey,
        information: EcologicalInformation,
    ) -> RepresentationCapabilities {
        RepresentationCapabilities::new(
            key,
            EcologicalAuthorityLevel::Coarse,
            [(information, CapabilityEvidence::Exact)],
        )
    }

    fn closure_record(status: ClosureEvidenceStatus) -> RegisteredClosureEvidence {
        RegisteredClosureEvidence::new(
            QualifiedClosureEvidence::new(
                ClosureModelVersion(4),
                ClosureDomainToken(55),
                ErrorPpm::new(1_000).unwrap(),
                EvidenceLineageToken(500),
            ),
            status,
        )
    }

    fn registry_in_order(reverse: bool) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        let process_a = process(PROCESS_A, EcologicalInformation::AgeDistribution);
        let process_b = process(
            PROCESS_B,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        let representation_a = representation(
            REPRESENTATION_A,
            EcologicalInformation::AgeDistribution,
        );
        let representation_b = representation(
            REPRESENTATION_B,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );

        if reverse {
            builder.register_process(process_b).unwrap();
            builder.register_process(process_a).unwrap();
            builder.register_representation(representation_b).unwrap();
            builder.register_representation(representation_a).unwrap();
        } else {
            builder.register_process(process_a).unwrap();
            builder.register_process(process_b).unwrap();
            builder.register_representation(representation_a).unwrap();
            builder.register_representation(representation_b).unwrap();
        }
        builder
            .register_closure_evidence(closure_record(ClosureEvidenceStatus::Qualified))
            .unwrap();
        builder.seal()
    }

    #[test]
    fn registration_order_does_not_change_manifest_identity() {
        let forward = registry_in_order(false);
        let reverse = registry_in_order(true);

        assert_eq!(
            InformationPolicyManifest::from_registry(&forward),
            InformationPolicyManifest::from_registry(&reverse)
        );
        assert_eq!(
            InformationPolicyAuthorityStamp::from_registry(&forward),
            InformationPolicyAuthorityStamp::from_registry(&reverse)
        );
    }

    #[test]
    fn changing_one_process_requirement_changes_exact_identity() {
        let original = registry_in_order(false);
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_process(process(PROCESS_A, EcologicalInformation::ConditionDistribution))
            .unwrap();
        builder
            .register_process(process(
                PROCESS_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_A,
                EcologicalInformation::AgeDistribution,
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_closure_evidence(closure_record(ClosureEvidenceStatus::Qualified))
            .unwrap();
        let changed = builder.seal();

        assert_ne!(
            InformationPolicyAuthorityStamp::from_registry(&original),
            InformationPolicyAuthorityStamp::from_registry(&changed)
        );
    }

    #[test]
    fn changing_representation_claim_changes_exact_identity() {
        let original = registry_in_order(false);
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_process(process(PROCESS_A, EcologicalInformation::AgeDistribution))
            .unwrap();
        builder
            .register_process(process(
                PROCESS_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_A,
                EcologicalInformation::ConditionDistribution,
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_closure_evidence(closure_record(ClosureEvidenceStatus::Qualified))
            .unwrap();
        let changed = builder.seal();

        assert_ne!(
            InformationPolicyAuthorityStamp::from_registry(&original),
            InformationPolicyAuthorityStamp::from_registry(&changed)
        );
    }

    #[test]
    fn revoking_closure_evidence_changes_exact_identity() {
        let original = registry_in_order(false);
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_process(process(PROCESS_A, EcologicalInformation::AgeDistribution))
            .unwrap();
        builder
            .register_process(process(
                PROCESS_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_A,
                EcologicalInformation::AgeDistribution,
            ))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_B,
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION,
                ),
            ))
            .unwrap();
        builder
            .register_closure_evidence(closure_record(ClosureEvidenceStatus::Revoked))
            .unwrap();
        let changed = builder.seal();

        assert_ne!(
            InformationPolicyAuthorityStamp::from_registry(&original),
            InformationPolicyAuthorityStamp::from_registry(&changed)
        );
    }

    #[test]
    fn same_semantic_key_with_changed_corpus_fails_validation() {
        let original = registry_in_order(false);
        let stamp = InformationPolicyAuthorityStamp::from_registry(&original);

        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_process(process(PROCESS_A, EcologicalInformation::ConditionDistribution))
            .unwrap();
        builder
            .register_representation(representation(
                REPRESENTATION_A,
                EcologicalInformation::ConditionDistribution,
            ))
            .unwrap();
        let changed = builder.seal();

        assert_eq!(
            stamp.validate_registry(&changed),
            Err(InformationPolicyIdentityError::ManifestMismatch { key: REGISTRY })
        );
    }

    #[test]
    fn semantic_key_mismatch_is_distinct_from_content_mismatch() {
        let registry = registry_in_order(false);
        let stamp = InformationPolicyAuthorityStamp::from_registry(&registry);
        let other = InformationPolicyRegistryBuilder::new(OTHER_REGISTRY).seal();

        assert_eq!(
            stamp.validate_registry(&other),
            Err(InformationPolicyIdentityError::RegistryKeyMismatch {
                expected: REGISTRY,
                actual: OTHER_REGISTRY,
            })
        );
    }

    #[test]
    fn sufficiency_report_binds_exact_policy_corpus() {
        let registry = registry_in_order(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&registry);
        let report = bound
            .evaluate_registered([PROCESS_A], REPRESENTATION_A)
            .unwrap();

        assert!(report.is_sufficient());
        assert_eq!(report.authority_stamp(), bound.authority_stamp());
        assert_eq!(report.report().registry_key(), REGISTRY);
        assert_eq!(report.validate_registry(&registry), Ok(()));
    }

    #[test]
    fn manifest_has_explicit_domain_and_version_prefix() {
        let registry = registry_in_order(false);
        let manifest = InformationPolicyManifest::from_registry(&registry);

        assert!(manifest.as_bytes().starts_with(MANIFEST_DOMAIN));
        let version_offset = MANIFEST_DOMAIN.len();
        assert_eq!(
            &manifest.as_bytes()[version_offset..version_offset + 4],
            &INFORMATION_POLICY_MANIFEST_VERSION.to_le_bytes()
        );
    }
}
