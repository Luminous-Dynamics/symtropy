// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed scientific semantics for qualified ecological closure evidence.
//!
//! The first information-sufficiency tranche carries a compact [`ErrorPpm`] on
//! [`QualifiedClosureEvidence`]. That is useful evidence plumbing, but a numeric
//! bound is not scientifically meaningful without naming the observable,
//! metric, horizon, aggregation, and zero-reference semantics that were actually
//! qualified. This module adds that authority layer without mutating the frozen
//! low-level algebra.
//!
//! V0 intentionally supports a bounded family of ppm-based metrics. Absolute-unit
//! and distribution-distance metrics should enter through a later manifest/profile
//! version rather than being guessed into these semantics.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::information::{
    ErrorPpm, EvidenceLineageToken, QualifiedClosureEvidence,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::{ClosureEvidenceStatus, InformationRegistryError};

/// Semantic identity/version of one closure observable.
///
/// The observable may be a directly retained ecological quantity or a derived
/// scientific quantity such as extinction probability. It is deliberately not a
/// free-form string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureObservableKey {
    id: u128,
    version: u32,
}

impl ClosureObservableKey {
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

/// Semantic identity/version of one qualification theorem/profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureQualificationProfileKey {
    id: u128,
    version: u32,
}

impl ClosureQualificationProfileKey {
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

/// Semantic identity/version of one immutable typed-closure registry generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureQualificationRegistryKey {
    id: u128,
    version: u32,
}

impl TypedClosureQualificationRegistryKey {
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

/// How a relative-error metric treats zero or near-zero reference values.
///
/// `QualifiedDomainGuaranteesPositiveReference` means the closure's separately
/// bound applicability domain is itself responsible for excluding zero/invalid
/// denominator states. It is not a runtime assertion that the current state is
/// inside that domain; domain applicability remains an independent authority
/// question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RelativeZeroReferencePolicy {
    RejectZeroReference,
    QualifiedDomainGuaranteesPositiveReference,
}

/// V0 error-metric semantics.
///
/// Every variant uses ppm as the numeric scale so it can bind directly to the
/// frozen `QualifiedClosureEvidence::error_ppm`, but identical numeric bounds in
/// different variants are intentionally different scientific evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureErrorMetric {
    MeanRelativePpm {
        zero_reference: RelativeZeroReferencePolicy,
    },
    MaximumRelativePpm {
        zero_reference: RelativeZeroReferencePolicy,
    },
    EventProbabilityCalibrationPpm,
    ThresholdFalseNegativeRatePpm,
    ThresholdFalsePositiveRatePpm,
}

/// Aggregation semantics used when producing the qualified error statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureAggregationSemantics {
    PerStep,
    MeanOverHorizon,
    MaximumOverHorizon,
    TerminalState,
    EventWindow,
}

/// Canonical evaluation horizon of one closure qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureEvaluationHorizonTicks(u64);

impl ClosureEvaluationHorizonTicks {
    pub const fn new(ticks: u64) -> Result<Self, TypedClosureQualificationError> {
        if ticks == 0 {
            return Err(TypedClosureQualificationError::ZeroEvaluationHorizon);
        }
        Ok(Self(ticks))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Opaque exact implementation/artifact identity supplied by the closure
/// qualification process. The bytes are authority-bearing structured identity,
/// not interpreted here as a home-grown cryptographic digest.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureQualificationImplementationFingerprint(Vec<u8>);

impl ClosureQualificationImplementationFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, TypedClosureQualificationError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(TypedClosureQualificationError::EmptyImplementationFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Complete typed interpretation of one legacy-qualified closure lineage.
///
/// `evidence` still carries the frozen model/domain/error/lineage plumbing. The
/// additional fields make the scientific meaning of that numeric bound explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureQualification {
    evidence: QualifiedClosureEvidence,
    observable: ClosureObservableKey,
    metric: ClosureErrorMetric,
    max_error_ppm: ErrorPpm,
    horizon: ClosureEvaluationHorizonTicks,
    aggregation: ClosureAggregationSemantics,
    qualification_profile: ClosureQualificationProfileKey,
    implementation: ClosureQualificationImplementationFingerprint,
}

impl TypedClosureQualification {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        evidence: QualifiedClosureEvidence,
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        max_error_ppm: ErrorPpm,
        horizon: ClosureEvaluationHorizonTicks,
        aggregation: ClosureAggregationSemantics,
        qualification_profile: ClosureQualificationProfileKey,
        implementation: ClosureQualificationImplementationFingerprint,
    ) -> Result<Self, TypedClosureQualificationError> {
        if evidence.error_ppm() != max_error_ppm {
            return Err(TypedClosureQualificationError::LegacyErrorBoundMismatch {
                lineage: evidence.evidence_lineage(),
                legacy: evidence.error_ppm(),
                typed: max_error_ppm,
            });
        }

        Ok(Self {
            evidence,
            observable,
            metric,
            max_error_ppm,
            horizon,
            aggregation,
            qualification_profile,
            implementation,
        })
    }

    pub const fn evidence(&self) -> QualifiedClosureEvidence {
        self.evidence
    }

    pub const fn lineage(&self) -> EvidenceLineageToken {
        self.evidence.evidence_lineage()
    }

    pub const fn observable(&self) -> ClosureObservableKey {
        self.observable
    }

    pub const fn metric(&self) -> ClosureErrorMetric {
        self.metric
    }

    pub const fn max_error_ppm(&self) -> ErrorPpm {
        self.max_error_ppm
    }

    pub const fn horizon(&self) -> ClosureEvaluationHorizonTicks {
        self.horizon
    }

    pub const fn aggregation(&self) -> ClosureAggregationSemantics {
        self.aggregation
    }

    pub const fn qualification_profile(&self) -> ClosureQualificationProfileKey {
        self.qualification_profile
    }

    pub const fn implementation(&self) -> &ClosureQualificationImplementationFingerprint {
        &self.implementation
    }
}

/// Bootstrap-only builder for the typed closure authority corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureQualificationRegistryBuilder {
    key: TypedClosureQualificationRegistryKey,
    qualifications: BTreeMap<EvidenceLineageToken, TypedClosureQualification>,
}

impl TypedClosureQualificationRegistryBuilder {
    pub const fn new(key: TypedClosureQualificationRegistryKey) -> Self {
        Self {
            key,
            qualifications: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        qualification: TypedClosureQualification,
    ) -> Result<(), TypedClosureQualificationError> {
        use std::collections::btree_map::Entry;

        let lineage = qualification.lineage();
        match self.qualifications.entry(lineage) {
            Entry::Vacant(entry) => {
                entry.insert(qualification);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(_) => {
                Err(TypedClosureQualificationError::ConflictingQualificationRegistration {
                    lineage,
                })
            }
        }
    }

    /// Seal typed semantics only when every lineage is the exact currently
    /// qualified closure record recognized by the exact information-policy corpus.
    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<TypedClosureQualificationRegistry, TypedClosureQualificationError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(TypedClosureQualificationError::PolicyIdentity)?;

        for qualification in self.qualifications.values() {
            validate_against_policy(policy, qualification)?;
        }

        let authority = TypedClosureQualificationAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            qualifications: self.qualifications,
        };
        Ok(TypedClosureQualificationRegistry { authority })
    }
}

/// Exact structured authority identity of one immutable typed closure corpus.
///
/// V0 keeps the complete ordered qualification records rather than reducing them
/// to a weak short hash. A canonical persistence/federation encoding can be
/// layered on later without weakening in-process equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureQualificationAuthorityStamp {
    key: TypedClosureQualificationRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    qualifications: BTreeMap<EvidenceLineageToken, TypedClosureQualification>,
}

impl TypedClosureQualificationAuthorityStamp {
    pub const fn key(&self) -> TypedClosureQualificationRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }

    pub fn qualifications(
        &self,
    ) -> impl Iterator<Item = (&EvidenceLineageToken, &TypedClosureQualification)> {
        self.qualifications.iter()
    }
}

/// Immutable runtime authority for typed closure qualification semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureQualificationRegistry {
    authority: TypedClosureQualificationAuthorityStamp,
}

impl TypedClosureQualificationRegistry {
    pub const fn authority_stamp(&self) -> &TypedClosureQualificationAuthorityStamp {
        &self.authority
    }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TypedClosureQualificationError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(TypedClosureQualificationError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(TypedClosureQualificationError::PolicyAuthorityMismatch);
        }

        for qualification in self.authority.qualifications.values() {
            validate_against_policy(policy, qualification)?;
        }
        Ok(())
    }

    pub fn resolve(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        lineage: EvidenceLineageToken,
    ) -> Result<ResolvedTypedClosureQualification, TypedClosureQualificationError> {
        self.validate_current(policy)?;
        let qualification = self
            .authority
            .qualifications
            .get(&lineage)
            .ok_or(TypedClosureQualificationError::UnknownTypedQualification { lineage })?;

        Ok(ResolvedTypedClosureQualification {
            authority: self.authority.clone(),
            qualification: qualification.clone(),
        })
    }
}

/// One typed closure qualification resolved from the exact current authority corpus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTypedClosureQualification {
    authority: TypedClosureQualificationAuthorityStamp,
    qualification: TypedClosureQualification,
}

impl ResolvedTypedClosureQualification {
    pub const fn authority(&self) -> &TypedClosureQualificationAuthorityStamp {
        &self.authority
    }

    pub const fn qualification(&self) -> &TypedClosureQualification {
        &self.qualification
    }

    pub fn validate_current(
        &self,
        registry: &TypedClosureQualificationRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TypedClosureQualificationError> {
        let current = registry.resolve(policy, self.qualification.lineage())?;
        if current != *self {
            return Err(TypedClosureQualificationError::ResolvedQualificationStale {
                lineage: self.qualification.lineage(),
            });
        }
        Ok(())
    }
}

fn validate_against_policy(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    qualification: &TypedClosureQualification,
) -> Result<(), TypedClosureQualificationError> {
    let lineage = qualification.lineage();
    let legacy = policy
        .registry()
        .closure_evidence_records()
        .find_map(|(candidate, record)| (*candidate == lineage).then_some(*record))
        .ok_or(TypedClosureQualificationError::UnknownLegacyClosureEvidence { lineage })?;

    if legacy.evidence() != qualification.evidence() {
        return Err(TypedClosureQualificationError::LegacyClosureEvidenceMismatch { lineage });
    }
    if legacy.status() != ClosureEvidenceStatus::Qualified {
        return Err(TypedClosureQualificationError::LegacyClosureEvidenceRevoked { lineage });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedClosureQualificationError {
    ZeroEvaluationHorizon,
    EmptyImplementationFingerprint,
    LegacyErrorBoundMismatch {
        lineage: EvidenceLineageToken,
        legacy: ErrorPpm,
        typed: ErrorPpm,
    },
    ConflictingQualificationRegistration {
        lineage: EvidenceLineageToken,
    },
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyAuthorityMismatch,
    UnknownLegacyClosureEvidence {
        lineage: EvidenceLineageToken,
    },
    LegacyClosureEvidenceMismatch {
        lineage: EvidenceLineageToken,
    },
    LegacyClosureEvidenceRevoked {
        lineage: EvidenceLineageToken,
    },
    UnknownTypedQualification {
        lineage: EvidenceLineageToken,
    },
    ResolvedQualificationStale {
        lineage: EvidenceLineageToken,
    },
    InformationRegistry(InformationRegistryError),
}

impl fmt::Display for TypedClosureQualificationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroEvaluationHorizon => {
                write!(formatter, "closure qualification horizon must be at least one tick")
            }
            Self::EmptyImplementationFingerprint => {
                write!(formatter, "closure qualification implementation fingerprint is empty")
            }
            Self::LegacyErrorBoundMismatch {
                lineage,
                legacy,
                typed,
            } => write!(
                formatter,
                "typed closure bound {} ppm does not match legacy bound {} ppm for lineage {}",
                typed.get(),
                legacy.get(),
                lineage.0
            ),
            Self::ConflictingQualificationRegistration { lineage } => write!(
                formatter,
                "conflicting typed closure qualification registration for lineage {}",
                lineage.0
            ),
            Self::PolicyIdentity(error) => {
                write!(formatter, "information-policy identity error: {error}")
            }
            Self::PolicyAuthorityMismatch => write!(
                formatter,
                "typed closure authority was sealed under a different exact information-policy corpus"
            ),
            Self::UnknownLegacyClosureEvidence { lineage } => write!(
                formatter,
                "typed closure lineage {} is not registered in the information-policy corpus",
                lineage.0
            ),
            Self::LegacyClosureEvidenceMismatch { lineage } => write!(
                formatter,
                "typed closure lineage {} does not match the exact registered legacy closure evidence",
                lineage.0
            ),
            Self::LegacyClosureEvidenceRevoked { lineage } => write!(
                formatter,
                "typed closure lineage {} is revoked in the information-policy corpus",
                lineage.0
            ),
            Self::UnknownTypedQualification { lineage } => write!(
                formatter,
                "no typed closure qualification is registered for lineage {}",
                lineage.0
            ),
            Self::ResolvedQualificationStale { lineage } => write!(
                formatter,
                "resolved typed closure qualification for lineage {} is stale",
                lineage.0
            ),
            Self::InformationRegistry(error) => {
                write!(formatter, "information registry error: {error}")
            }
        }
    }
}

impl Error for TypedClosureQualificationError {
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
        ClosureDomainToken, ClosureModelVersion,
    };
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
        RegisteredClosureEvidence,
    };

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(4_000, 1);
    const REGISTRY: TypedClosureQualificationRegistryKey =
        TypedClosureQualificationRegistryKey::new(4_010, 1);
    const LINEAGE: EvidenceLineageToken = EvidenceLineageToken(4_020);
    const OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(4_030, 1);
    const OTHER_OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(4_031, 1);
    const PROFILE: ClosureQualificationProfileKey = ClosureQualificationProfileKey::new(4_040, 1);

    fn legacy_evidence() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(7),
            ClosureDomainToken(8),
            ErrorPpm::new(10_000).unwrap(),
            LINEAGE,
        )
    }

    fn policy(status: ClosureEvidenceStatus) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(legacy_evidence(), status))
            .unwrap();
        builder.seal()
    }

    fn qualification(
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        horizon: u64,
    ) -> TypedClosureQualification {
        TypedClosureQualification::new(
            legacy_evidence(),
            observable,
            metric,
            ErrorPpm::new(10_000).unwrap(),
            ClosureEvaluationHorizonTicks::new(horizon).unwrap(),
            ClosureAggregationSemantics::MeanOverHorizon,
            PROFILE,
            ClosureQualificationImplementationFingerprint::new(b"closure-v1".to_vec()).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn same_numeric_bound_with_different_metric_is_different_authority() {
        let mean = qualification(
            OBSERVABLE,
            ClosureErrorMetric::MeanRelativePpm {
                zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
            },
            10,
        );
        let maximum = qualification(
            OBSERVABLE,
            ClosureErrorMetric::MaximumRelativePpm {
                zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
            },
            10,
        );
        assert_ne!(mean, maximum);
    }

    #[test]
    fn same_metric_and_bound_with_different_observable_is_different_authority() {
        let metric = ClosureErrorMetric::MeanRelativePpm {
            zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
        };
        assert_ne!(
            qualification(OBSERVABLE, metric, 10),
            qualification(OTHER_OBSERVABLE, metric, 10)
        );
    }

    #[test]
    fn horizon_is_authority_bearing() {
        let metric = ClosureErrorMetric::MeanRelativePpm {
            zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
        };
        assert_ne!(
            qualification(OBSERVABLE, metric, 1),
            qualification(OBSERVABLE, metric, 1_000)
        );
    }

    #[test]
    fn legacy_bound_must_match_typed_bound_exactly() {
        let error = TypedClosureQualification::new(
            legacy_evidence(),
            OBSERVABLE,
            ClosureErrorMetric::EventProbabilityCalibrationPpm,
            ErrorPpm::new(20_000).unwrap(),
            ClosureEvaluationHorizonTicks::new(10).unwrap(),
            ClosureAggregationSemantics::EventWindow,
            PROFILE,
            ClosureQualificationImplementationFingerprint::new(b"closure-v1".to_vec()).unwrap(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            TypedClosureQualificationError::LegacyErrorBoundMismatch { .. }
        ));
    }

    #[test]
    fn revoked_legacy_lineage_cannot_be_sealed_as_typed_authority() {
        let legacy = policy(ClosureEvidenceStatus::Revoked);
        let exact = ManifestBoundInformationPolicyRegistry::new(&legacy);
        let mut builder = TypedClosureQualificationRegistryBuilder::new(REGISTRY);
        builder
            .register(qualification(
                OBSERVABLE,
                ClosureErrorMetric::MeanRelativePpm {
                    zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
                },
                10,
            ))
            .unwrap();
        assert!(matches!(
            builder.seal(&exact),
            Err(TypedClosureQualificationError::LegacyClosureEvidenceRevoked { .. })
        ));
    }

    #[test]
    fn changed_typed_semantics_under_same_registry_key_stale_old_resolution() {
        let legacy = policy(ClosureEvidenceStatus::Qualified);
        let exact = ManifestBoundInformationPolicyRegistry::new(&legacy);

        let mut first_builder = TypedClosureQualificationRegistryBuilder::new(REGISTRY);
        first_builder
            .register(qualification(
                OBSERVABLE,
                ClosureErrorMetric::MeanRelativePpm {
                    zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
                },
                10,
            ))
            .unwrap();
        let first = first_builder.seal(&exact).unwrap();
        let resolved = first.resolve(&exact, LINEAGE).unwrap();

        let mut second_builder = TypedClosureQualificationRegistryBuilder::new(REGISTRY);
        second_builder
            .register(qualification(
                OBSERVABLE,
                ClosureErrorMetric::MaximumRelativePpm {
                    zero_reference: RelativeZeroReferencePolicy::RejectZeroReference,
                },
                10,
            ))
            .unwrap();
        let second = second_builder.seal(&exact).unwrap();

        assert!(matches!(
            resolved.validate_current(&second, &exact),
            Err(TypedClosureQualificationError::ResolvedQualificationStale { .. })
        ));
    }
}
