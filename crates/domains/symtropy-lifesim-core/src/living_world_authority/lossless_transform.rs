// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Authority-owned qualification for R1 lossless information transforms.
//!
//! A [`LosslessTransformKey`] is only a semantic descriptor. Canonical runtime
//! code may resolve it as R1 evidence only through one sealed registry whose
//! complete qualification records are bound to one exact information-policy
//! authority. No `trusted: bool` or caller-authored `is_lossless` shortcut exists.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{
    CapabilityEvidence, EcologicalInformation, EvidenceLineageToken, RepresentationKey,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::{InformationPolicyRegistry, InformationRegistryError};
use crate::information_transition::{
    InformationTransitionDefinition, InformationTransitionKey, LosslessTransformKey,
    PromotionProvenance, TransitionCapabilityClaim,
};
use crate::transition_applicability::TransitionDomainKey;

/// Semantic identity/version of one sealed R1 qualification-registry generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformRegistryKey {
    id: u128,
    version: u32,
}

impl LosslessTransformRegistryKey {
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

/// Versioned qualification theorem/profile used to establish losslessness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformQualificationProfileKey {
    id: u128,
    version: u32,
}

impl LosslessTransformQualificationProfileKey {
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

/// Opaque exact implementation/artifact identity supplied by the qualification
/// process. The bytes are intentionally not interpreted as a home-grown digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformImplementationFingerprint(Vec<u8>);

impl LosslessTransformImplementationFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, LosslessTransformAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(LosslessTransformAuthorityError::EmptyImplementationFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Optional exact execution-capsule/toolchain identity for transforms whose
/// behavior depends on compiled/runtime environment details.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LosslessTransformExecutionCapsuleFingerprint(Vec<u8>);

impl LosslessTransformExecutionCapsuleFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, LosslessTransformAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(LosslessTransformAuthorityError::EmptyExecutionCapsuleFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Settled authority status for one transform qualification in one immutable
/// registry generation. Status changes require a new generation/stamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LosslessTransformQualificationStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Complete reviewed qualification record for one semantic transform.
///
/// Construction is bootstrap/evidence ingestion, not ecological runtime
/// authorization. Runtime code must resolve through [`LosslessTransformRegistry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessTransformQualificationRecord {
    transform: LosslessTransformKey,
    implementation: LosslessTransformImplementationFingerprint,
    source: RepresentationKey,
    destination: RepresentationKey,
    introduced_claims: BTreeSet<TransitionCapabilityClaim>,
    admitted_domain: Option<TransitionDomainKey>,
    qualification_profile: LosslessTransformQualificationProfileKey,
    evidence_lineage: EvidenceLineageToken,
    execution_capsule: Option<LosslessTransformExecutionCapsuleFingerprint>,
    status: LosslessTransformQualificationStatus,
}

impl LosslessTransformQualificationRecord {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        transform: LosslessTransformKey,
        implementation: LosslessTransformImplementationFingerprint,
        source: RepresentationKey,
        destination: RepresentationKey,
        introduced_claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        admitted_domain: Option<TransitionDomainKey>,
        qualification_profile: LosslessTransformQualificationProfileKey,
        evidence_lineage: EvidenceLineageToken,
        execution_capsule: Option<LosslessTransformExecutionCapsuleFingerprint>,
        status: LosslessTransformQualificationStatus,
    ) -> Result<Self, LosslessTransformAuthorityError> {
        let introduced_claims = introduced_claims.into_iter().collect::<BTreeSet<_>>();
        if introduced_claims.is_empty() {
            return Err(LosslessTransformAuthorityError::EmptyIntroducedClaimSet {
                transform,
            });
        }
        for claim in &introduced_claims {
            if claim.evidence() != CapabilityEvidence::Exact {
                return Err(LosslessTransformAuthorityError::NonExactIntroducedClaim {
                    transform,
                    claim: *claim,
                });
            }
        }

        Ok(Self {
            transform,
            implementation,
            source,
            destination,
            introduced_claims,
            admitted_domain,
            qualification_profile,
            evidence_lineage,
            execution_capsule,
            status,
        })
    }

    pub const fn transform(&self) -> LosslessTransformKey {
        self.transform
    }

    pub const fn implementation(&self) -> &LosslessTransformImplementationFingerprint {
        &self.implementation
    }

    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub const fn introduced_claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.introduced_claims
    }

    pub const fn admitted_domain(&self) -> Option<TransitionDomainKey> {
        self.admitted_domain
    }

    pub const fn qualification_profile(&self) -> LosslessTransformQualificationProfileKey {
        self.qualification_profile
    }

    pub const fn evidence_lineage(&self) -> EvidenceLineageToken {
        self.evidence_lineage
    }

    pub const fn execution_capsule(
        &self,
    ) -> Option<&LosslessTransformExecutionCapsuleFingerprint> {
        self.execution_capsule.as_ref()
    }

    pub const fn status(&self) -> LosslessTransformQualificationStatus {
        self.status
    }
}

/// Bootstrap-only builder. Canonical runtime consumers should receive only the
/// sealed immutable registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessTransformRegistryBuilder {
    key: LosslessTransformRegistryKey,
    records: BTreeMap<LosslessTransformKey, LosslessTransformQualificationRecord>,
}

impl LosslessTransformRegistryBuilder {
    pub const fn new(key: LosslessTransformRegistryKey) -> Self {
        Self {
            key,
            records: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        record: LosslessTransformQualificationRecord,
    ) -> Result<(), LosslessTransformAuthorityError> {
        let transform = record.transform();
        if let Some(existing) = self.records.get(&transform) {
            if existing == &record {
                return Ok(());
            }
            return Err(LosslessTransformAuthorityError::ConflictingRegistration {
                transform,
            });
        }
        self.records.insert(transform, record);
        Ok(())
    }

    /// Seal records against one exact information-policy corpus.
    ///
    /// A semantic representation key is insufficient: if policy capabilities
    /// change under the same key, a separately sealed transform registry binds a
    /// different exact policy authority stamp.
    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<LosslessTransformRegistry, LosslessTransformAuthorityError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(LosslessTransformAuthorityError::PolicyIdentity)?;

        for record in self.records.values() {
            validate_record_against_policy(policy.registry(), record)?;
        }

        let authority = LosslessTransformRegistryAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            records: self.records,
        };
        Ok(LosslessTransformRegistry { authority })
    }
}

/// Exact structured authority identity for one immutable R1 registry generation.
///
/// V0 uses the complete deterministic record structure as identity rather than a
/// short hash. A canonical persistence/federation manifest can be layered on
/// later without weakening in-process exact equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessTransformRegistryAuthorityStamp {
    key: LosslessTransformRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    records: BTreeMap<LosslessTransformKey, LosslessTransformQualificationRecord>,
}

impl LosslessTransformRegistryAuthorityStamp {
    pub const fn key(&self) -> LosslessTransformRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }

    pub fn records(
        &self,
    ) -> impl Iterator<Item = (&LosslessTransformKey, &LosslessTransformQualificationRecord)> {
        self.records.iter()
    }
}

/// Immutable runtime resolver for R1 transform authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LosslessTransformRegistry {
    authority: LosslessTransformRegistryAuthorityStamp,
}

impl LosslessTransformRegistry {
    pub const fn authority_stamp(&self) -> &LosslessTransformRegistryAuthorityStamp {
        &self.authority
    }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), LosslessTransformAuthorityError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(LosslessTransformAuthorityError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(LosslessTransformAuthorityError::PolicyAuthorityMismatch);
        }
        for record in self.authority.records.values() {
            validate_record_against_policy(policy.registry(), record)?;
        }
        Ok(())
    }

    /// Resolve one R1 requirement for one exact transition definition.
    ///
    /// This does not prove a state satisfies `admitted_domain`; #307 remains an
    /// independent required producer. It proves only that the exact transform
    /// implementation/profile currently qualifies the transition's R1 claims.
    pub fn resolve_transition(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        transition: &InformationTransitionDefinition,
        transform: LosslessTransformKey,
    ) -> Result<ResolvedLosslessTransformAuthority, LosslessTransformAuthorityError> {
        self.validate_current(policy)?;

        let record = self
            .authority
            .records
            .get(&transform)
            .ok_or(LosslessTransformAuthorityError::UnknownTransform { transform })?;

        match record.status() {
            LosslessTransformQualificationStatus::Qualified => {}
            LosslessTransformQualificationStatus::Revoked => {
                return Err(LosslessTransformAuthorityError::TransformRevoked { transform });
            }
            LosslessTransformQualificationStatus::Superseded => {
                return Err(LosslessTransformAuthorityError::TransformSuperseded { transform });
            }
        }

        if transition.source() != record.source() {
            return Err(LosslessTransformAuthorityError::TransitionSourceMismatch {
                transform,
                expected: record.source(),
                actual: transition.source(),
            });
        }
        if transition.destination() != record.destination() {
            return Err(LosslessTransformAuthorityError::TransitionDestinationMismatch {
                transform,
                expected: record.destination(),
                actual: transition.destination(),
            });
        }

        let transition_claims = transition
            .introductions()
            .iter()
            .filter_map(|(claim, provenance)| match provenance {
                PromotionProvenance::LosslessDerivation {
                    transform: referenced,
                } if *referenced == transform => Some(*claim),
                _ => None,
            })
            .collect::<BTreeSet<_>>();

        if transition_claims != record.introduced_claims {
            return Err(LosslessTransformAuthorityError::TransitionClaimSetMismatch {
                transform,
            });
        }

        Ok(ResolvedLosslessTransformAuthority {
            registry_authority: self.authority.clone(),
            transition: transition.key(),
            record: record.clone(),
        })
    }
}

/// R1 evidence resolved from one exact immutable registry generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedLosslessTransformAuthority {
    registry_authority: LosslessTransformRegistryAuthorityStamp,
    transition: InformationTransitionKey,
    record: LosslessTransformQualificationRecord,
}

impl ResolvedLosslessTransformAuthority {
    pub const fn registry_authority(&self) -> &LosslessTransformRegistryAuthorityStamp {
        &self.registry_authority
    }

    pub const fn transition(&self) -> InformationTransitionKey {
        self.transition
    }

    pub const fn record(&self) -> &LosslessTransformQualificationRecord {
        &self.record
    }

    /// Domain applicability remains unresolved here. A returned domain key is an
    /// obligation for #307, never a proof boolean.
    pub const fn required_domain(&self) -> Option<TransitionDomainKey> {
        self.record.admitted_domain()
    }

    pub fn validate_current(
        &self,
        registry: &LosslessTransformRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        transition: &InformationTransitionDefinition,
    ) -> Result<(), LosslessTransformAuthorityError> {
        let current = registry.resolve_transition(policy, transition, self.record.transform())?;
        if current != *self {
            return Err(LosslessTransformAuthorityError::ResolvedAuthorityStale {
                transform: self.record.transform(),
            });
        }
        Ok(())
    }
}

fn validate_record_against_policy(
    policy: &InformationPolicyRegistry,
    record: &LosslessTransformQualificationRecord,
) -> Result<(), LosslessTransformAuthorityError> {
    policy
        .resolve_representation(record.source())
        .map_err(LosslessTransformAuthorityError::InformationRegistry)?;
    let destination = policy
        .resolve_representation(record.destination())
        .map_err(LosslessTransformAuthorityError::InformationRegistry)?;

    for claim in record.introduced_claims() {
        let exact_is_registered = destination
            .capabilities()
            .claims()
            .get(&claim.information())
            .is_some_and(|evidence| evidence.contains(&CapabilityEvidence::Exact));
        if !exact_is_registered {
            return Err(
                LosslessTransformAuthorityError::DestinationMissingExactQualificationClaim {
                    transform: record.transform(),
                    information: claim.information(),
                },
            );
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LosslessTransformAuthorityError {
    EmptyImplementationFingerprint,
    EmptyExecutionCapsuleFingerprint,
    EmptyIntroducedClaimSet {
        transform: LosslessTransformKey,
    },
    NonExactIntroducedClaim {
        transform: LosslessTransformKey,
        claim: TransitionCapabilityClaim,
    },
    ConflictingRegistration {
        transform: LosslessTransformKey,
    },
    PolicyIdentity(InformationPolicyIdentityError),
    InformationRegistry(InformationRegistryError),
    PolicyAuthorityMismatch,
    DestinationMissingExactQualificationClaim {
        transform: LosslessTransformKey,
        information: EcologicalInformation,
    },
    UnknownTransform {
        transform: LosslessTransformKey,
    },
    TransformRevoked {
        transform: LosslessTransformKey,
    },
    TransformSuperseded {
        transform: LosslessTransformKey,
    },
    TransitionSourceMismatch {
        transform: LosslessTransformKey,
        expected: RepresentationKey,
        actual: RepresentationKey,
    },
    TransitionDestinationMismatch {
        transform: LosslessTransformKey,
        expected: RepresentationKey,
        actual: RepresentationKey,
    },
    TransitionClaimSetMismatch {
        transform: LosslessTransformKey,
    },
    ResolvedAuthorityStale {
        transform: LosslessTransformKey,
    },
}

impl fmt::Display for LosslessTransformAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyImplementationFingerprint => {
                write!(formatter, "lossless transform implementation fingerprint is empty")
            }
            Self::EmptyExecutionCapsuleFingerprint => {
                write!(formatter, "lossless transform execution-capsule fingerprint is empty")
            }
            Self::EmptyIntroducedClaimSet { transform } => write!(
                formatter,
                "lossless transform {:?} qualifies no introduced Exact claims",
                transform
            ),
            Self::NonExactIntroducedClaim { transform, claim } => write!(
                formatter,
                "lossless transform {:?} attempted to qualify non-Exact claim {:?}",
                transform, claim
            ),
            Self::ConflictingRegistration { transform } => write!(
                formatter,
                "conflicting lossless-transform qualification for {:?}",
                transform
            ),
            Self::PolicyIdentity(error) => write!(formatter, "information-policy identity: {error}"),
            Self::InformationRegistry(error) => {
                write!(formatter, "information-policy registry: {error}")
            }
            Self::PolicyAuthorityMismatch => write!(
                formatter,
                "lossless-transform registry is bound to different exact information-policy authority"
            ),
            Self::DestinationMissingExactQualificationClaim {
                transform,
                information,
            } => write!(
                formatter,
                "lossless transform {:?} qualifies {:?}, but destination policy lacks that Exact claim",
                transform, information
            ),
            Self::UnknownTransform { transform } => {
                write!(formatter, "unknown qualified lossless transform {:?}", transform)
            }
            Self::TransformRevoked { transform } => {
                write!(formatter, "lossless transform {:?} is revoked", transform)
            }
            Self::TransformSuperseded { transform } => {
                write!(formatter, "lossless transform {:?} is superseded", transform)
            }
            Self::TransitionSourceMismatch {
                transform,
                expected,
                actual,
            } => write!(
                formatter,
                "lossless transform {:?} expects source {:?}, transition uses {:?}",
                transform, expected, actual
            ),
            Self::TransitionDestinationMismatch {
                transform,
                expected,
                actual,
            } => write!(
                formatter,
                "lossless transform {:?} expects destination {:?}, transition uses {:?}",
                transform, expected, actual
            ),
            Self::TransitionClaimSetMismatch { transform } => write!(
                formatter,
                "transition R1 claims do not exactly match qualification for {:?}",
                transform
            ),
            Self::ResolvedAuthorityStale { transform } => write!(
                formatter,
                "resolved lossless-transform authority for {:?} is stale",
                transform
            ),
        }
    }
}

impl Error for LosslessTransformAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        EcologicalAuthorityLevel, EcologicalInformation, RepresentationCapabilities,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(1, 1);
    const REGISTRY: LosslessTransformRegistryKey = LosslessTransformRegistryKey::new(2, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(10, 1);
    const DESTINATION: RepresentationKey = RepresentationKey::new(11, 1);
    const OTHER_DESTINATION: RepresentationKey = RepresentationKey::new(12, 1);
    const TRANSFORM: LosslessTransformKey = LosslessTransformKey(20, 1);
    const OTHER_TRANSFORM: LosslessTransformKey = LosslessTransformKey(21, 1);
    const TRANSITION: InformationTransitionKey = InformationTransitionKey::new(30, 1);
    const DOMAIN: TransitionDomainKey = TransitionDomainKey::new(40, 1);
    const PROFILE: LosslessTransformQualificationProfileKey =
        LosslessTransformQualificationProfileKey::new(50, 1);
    const EVIDENCE: EvidenceLineageToken = EvidenceLineageToken(60);

    fn exact_biomass_claim() -> TransitionCapabilityClaim {
        TransitionCapabilityClaim::new(
            EcologicalInformation::ExactLivingBiomass,
            CapabilityEvidence::Exact,
        )
    }

    fn policy(extra_destination_claim: bool) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_representation(RepresentationCapabilities::new(
                SOURCE,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::ExactLivingBiomass,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();

        let mut destination_claims = vec![
            (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
            (
                EcologicalInformation::ExactLivingBiomass,
                CapabilityEvidence::Exact,
            ),
        ];
        if extra_destination_claim {
            destination_claims.push((EcologicalInformation::DiseaseState, CapabilityEvidence::Exact));
        }
        builder
            .register_representation(RepresentationCapabilities::new(
                DESTINATION,
                EcologicalAuthorityLevel::Active,
                destination_claims,
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                OTHER_DESTINATION,
                EcologicalAuthorityLevel::Active,
                [(
                    EcologicalInformation::ExactLivingBiomass,
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        builder.seal()
    }

    fn transition(
        destination: RepresentationKey,
        transform: LosslessTransformKey,
        claim: TransitionCapabilityClaim,
    ) -> InformationTransitionDefinition {
        InformationTransitionDefinition::new(
            TRANSITION,
            SOURCE,
            destination,
            [(claim, PromotionProvenance::LosslessDerivation { transform })],
            [],
        )
        .unwrap()
    }

    fn record(
        transform: LosslessTransformKey,
        fingerprint: &[u8],
        status: LosslessTransformQualificationStatus,
    ) -> LosslessTransformQualificationRecord {
        LosslessTransformQualificationRecord::new(
            transform,
            LosslessTransformImplementationFingerprint::new(fingerprint.to_vec()).unwrap(),
            SOURCE,
            DESTINATION,
            [exact_biomass_claim()],
            Some(DOMAIN),
            PROFILE,
            EVIDENCE,
            None,
            status,
        )
        .unwrap()
    }

    fn registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        record: LosslessTransformQualificationRecord,
    ) -> LosslessTransformRegistry {
        let mut builder = LosslessTransformRegistryBuilder::new(REGISTRY);
        builder.register(record).unwrap();
        builder.seal(policy).unwrap()
    }

    #[test]
    fn qualified_transform_resolves_without_claiming_domain_satisfaction() {
        let policy = policy(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&policy);
        let registry = registry(
            &bound,
            record(
                TRANSFORM,
                b"qualified-implementation-a",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );
        let transition = transition(DESTINATION, TRANSFORM, exact_biomass_claim());

        let resolved = registry
            .resolve_transition(&bound, &transition, TRANSFORM)
            .unwrap();

        assert_eq!(resolved.record().transform(), TRANSFORM);
        assert_eq!(resolved.transition(), TRANSITION);
        assert_eq!(resolved.required_domain(), Some(DOMAIN));
        assert_eq!(resolved.record().evidence_lineage(), EVIDENCE);
        assert_eq!(resolved.validate_current(&registry, &bound, &transition), Ok(()));
    }

    #[test]
    fn transform_key_alone_cannot_resolve_unknown_authority() {
        let policy = policy(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&policy);
        let registry = registry(
            &bound,
            record(
                TRANSFORM,
                b"qualified-implementation-a",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );
        let transition = transition(DESTINATION, OTHER_TRANSFORM, exact_biomass_claim());

        assert_eq!(
            registry.resolve_transition(&bound, &transition, OTHER_TRANSFORM),
            Err(LosslessTransformAuthorityError::UnknownTransform {
                transform: OTHER_TRANSFORM,
            })
        );
    }

    #[test]
    fn revoked_or_superseded_qualification_fails_closed() {
        let policy = policy(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&policy);
        let transition = transition(DESTINATION, TRANSFORM, exact_biomass_claim());

        for status in [
            LosslessTransformQualificationStatus::Revoked,
            LosslessTransformQualificationStatus::Superseded,
        ] {
            let registry = registry(
                &bound,
                record(TRANSFORM, b"implementation", status),
            );
            let error = registry
                .resolve_transition(&bound, &transition, TRANSFORM)
                .unwrap_err();
            assert!(matches!(
                (status, error),
                (
                    LosslessTransformQualificationStatus::Revoked,
                    LosslessTransformAuthorityError::TransformRevoked { .. }
                ) | (
                    LosslessTransformQualificationStatus::Superseded,
                    LosslessTransformAuthorityError::TransformSuperseded { .. }
                )
            ));
        }
    }

    #[test]
    fn non_exact_claim_cannot_be_qualified_as_r1() {
        let claim = TransitionCapabilityClaim::new(
            EcologicalInformation::ExactLivingBiomass,
            CapabilityEvidence::MeasurementOnly,
        );
        let result = LosslessTransformQualificationRecord::new(
            TRANSFORM,
            LosslessTransformImplementationFingerprint::new(b"implementation".to_vec()).unwrap(),
            SOURCE,
            DESTINATION,
            [claim],
            None,
            PROFILE,
            EVIDENCE,
            None,
            LosslessTransformQualificationStatus::Qualified,
        );

        assert_eq!(
            result,
            Err(LosslessTransformAuthorityError::NonExactIntroducedClaim {
                transform: TRANSFORM,
                claim,
            })
        );
    }

    #[test]
    fn transition_must_match_qualified_source_destination_and_claims() {
        let policy = policy(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&policy);
        let registry = registry(
            &bound,
            record(
                TRANSFORM,
                b"implementation",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );

        let wrong_destination = transition(OTHER_DESTINATION, TRANSFORM, exact_biomass_claim());
        assert!(matches!(
            registry.resolve_transition(&bound, &wrong_destination, TRANSFORM),
            Err(LosslessTransformAuthorityError::TransitionDestinationMismatch { .. })
        ));

        let wrong_claim = TransitionCapabilityClaim::new(
            EcologicalInformation::Headcount,
            CapabilityEvidence::Exact,
        );
        let wrong_claim = transition(DESTINATION, TRANSFORM, wrong_claim);
        assert_eq!(
            registry.resolve_transition(&bound, &wrong_claim, TRANSFORM),
            Err(LosslessTransformAuthorityError::TransitionClaimSetMismatch {
                transform: TRANSFORM,
            })
        );
    }

    #[test]
    fn exact_policy_content_drift_stales_old_registry_even_with_same_policy_key() {
        let original = policy(false);
        let original_bound = ManifestBoundInformationPolicyRegistry::new(&original);
        let registry = registry(
            &original_bound,
            record(
                TRANSFORM,
                b"implementation",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );

        let drifted = policy(true);
        let drifted_bound = ManifestBoundInformationPolicyRegistry::new(&drifted);
        assert_eq!(
            registry.validate_current(&drifted_bound),
            Err(LosslessTransformAuthorityError::PolicyIdentity(
                InformationPolicyIdentityError::ManifestMismatch { key: POLICY }
            ))
        );
    }

    #[test]
    fn implementation_fingerprint_drift_stales_prepared_authority() {
        let policy = policy(false);
        let bound = ManifestBoundInformationPolicyRegistry::new(&policy);
        let original_registry = registry(
            &bound,
            record(
                TRANSFORM,
                b"implementation-a",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );
        let transition = transition(DESTINATION, TRANSFORM, exact_biomass_claim());
        let resolved = original_registry
            .resolve_transition(&bound, &transition, TRANSFORM)
            .unwrap();

        let replacement_registry = registry(
            &bound,
            record(
                TRANSFORM,
                b"implementation-b",
                LosslessTransformQualificationStatus::Qualified,
            ),
        );

        assert_eq!(
            resolved.validate_current(&replacement_registry, &bound, &transition),
            Err(LosslessTransformAuthorityError::ResolvedAuthorityStale {
                transform: TRANSFORM,
            })
        );
    }

    #[test]
    fn conflicting_reuse_of_transform_key_fails_during_bootstrap() {
        let mut builder = LosslessTransformRegistryBuilder::new(REGISTRY);
        builder
            .register(record(
                TRANSFORM,
                b"implementation-a",
                LosslessTransformQualificationStatus::Qualified,
            ))
            .unwrap();

        assert_eq!(
            builder.register(record(
                TRANSFORM,
                b"implementation-b",
                LosslessTransformQualificationStatus::Qualified,
            )),
            Err(LosslessTransformAuthorityError::ConflictingRegistration {
                transform: TRANSFORM,
            })
        );
    }
}
