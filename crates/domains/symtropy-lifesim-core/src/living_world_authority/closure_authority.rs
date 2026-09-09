// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed authority for R2 qualified closure evidence.
//!
//! The low-level information algebra carries a compact [`QualifiedClosureEvidence`]
//! tuple. This layer gives that evidence scientifically explicit observable,
//! metric, horizon, aggregation and zero-reference semantics. Authenticating a
//! closure here does **not** mean any particular process accepts it; process-side
//! acceptance remains a separate #352 authority gate.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::{
    ManifestBoundTransitionApplicabilityPolicy, TransitionApplicabilityAuthorityStamp,
};
use crate::candidate_evidence_obligations::{
    CandidateCertificationRequestError, CandidateCertificationRequestSet,
    CandidateEvidenceObligation, CandidatePathIdentity,
};
use crate::information::{
    CapabilityEvidence, ClosureDomainToken, ClosureModelVersion, EcologicalInformation, ErrorPpm,
    EvidenceLineageToken, QualifiedClosureEvidence,
};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::{ClosureEvidenceStatus, InformationRegistryError};
use crate::information_transition::{
    InformationTransitionKey, PromotionProvenance, TransitionCapabilityClaim,
};

/// Stable semantic identity for the ecological observable whose error is bounded.
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

/// V0 error metrics. Same numeric bounds under different variants are not
/// interchangeable evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureErrorMetric {
    MeanRelativePpm,
    MaximumRelativePpm,
    EventProbabilityAbsolutePpm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureAggregationSemantics {
    MeanOverHorizon,
    MaximumOverHorizon,
    EventProbabilityOverHorizon,
}

/// Explicit behavior for metrics with a reference denominator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClosureZeroReferencePolicy {
    /// Relative-error qualification is valid only on a domain that excludes a
    /// zero reference value. A zero-reference use must reject rather than divide
    /// by an invented epsilon.
    RejectZeroReference,
    /// Metric has no relative denominator (for example probability absolute
    /// calibration error), so zero-reference policy is not applicable.
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureEvaluationHorizonTicks(u64);

impl ClosureEvaluationHorizonTicks {
    pub const fn new(ticks: u64) -> Result<Self, TypedClosureAuthorityError> {
        if ticks == 0 {
            return Err(TypedClosureAuthorityError::ZeroEvaluationHorizon);
        }
        Ok(Self(ticks))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Scientifically typed error contract for one closure qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureErrorContract {
    observable: ClosureObservableKey,
    metric: ClosureErrorMetric,
    max_error_ppm: ErrorPpm,
    horizon: ClosureEvaluationHorizonTicks,
    aggregation: ClosureAggregationSemantics,
    zero_reference: ClosureZeroReferencePolicy,
}

impl TypedClosureErrorContract {
    pub fn new(
        observable: ClosureObservableKey,
        metric: ClosureErrorMetric,
        max_error_ppm: ErrorPpm,
        horizon: ClosureEvaluationHorizonTicks,
        aggregation: ClosureAggregationSemantics,
        zero_reference: ClosureZeroReferencePolicy,
    ) -> Result<Self, TypedClosureAuthorityError> {
        let expected_aggregation = match metric {
            ClosureErrorMetric::MeanRelativePpm => ClosureAggregationSemantics::MeanOverHorizon,
            ClosureErrorMetric::MaximumRelativePpm => {
                ClosureAggregationSemantics::MaximumOverHorizon
            }
            ClosureErrorMetric::EventProbabilityAbsolutePpm => {
                ClosureAggregationSemantics::EventProbabilityOverHorizon
            }
        };
        if aggregation != expected_aggregation {
            return Err(TypedClosureAuthorityError::MetricAggregationMismatch {
                metric,
                aggregation,
            });
        }

        let expected_zero = match metric {
            ClosureErrorMetric::MeanRelativePpm | ClosureErrorMetric::MaximumRelativePpm => {
                ClosureZeroReferencePolicy::RejectZeroReference
            }
            ClosureErrorMetric::EventProbabilityAbsolutePpm => {
                ClosureZeroReferencePolicy::NotApplicable
            }
        };
        if zero_reference != expected_zero {
            return Err(TypedClosureAuthorityError::MetricZeroReferenceMismatch {
                metric,
                zero_reference,
            });
        }

        Ok(Self {
            observable,
            metric,
            max_error_ppm,
            horizon,
            aggregation,
            zero_reference,
        })
    }

    pub const fn observable(self) -> ClosureObservableKey {
        self.observable
    }

    pub const fn metric(self) -> ClosureErrorMetric {
        self.metric
    }

    pub const fn max_error_ppm(self) -> ErrorPpm {
        self.max_error_ppm
    }

    pub const fn horizon(self) -> ClosureEvaluationHorizonTicks {
        self.horizon
    }

    pub const fn aggregation(self) -> ClosureAggregationSemantics {
        self.aggregation
    }

    pub const fn zero_reference(self) -> ClosureZeroReferencePolicy {
        self.zero_reference
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureQualificationProfileKey {
    id: u128,
    version: u32,
}

impl TypedClosureQualificationProfileKey {
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureModelImplementationFingerprint(Vec<u8>);

impl ClosureModelImplementationFingerprint {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, TypedClosureAuthorityError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(TypedClosureAuthorityError::EmptyImplementationFingerprint);
        }
        Ok(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TypedClosureQualificationStatus {
    Qualified,
    Revoked,
    Superseded,
}

/// Full typed qualification attached to one low-level closure evidence lineage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureQualification {
    evidence: QualifiedClosureEvidence,
    claims: BTreeSet<TransitionCapabilityClaim>,
    error_contract: TypedClosureErrorContract,
    implementation: ClosureModelImplementationFingerprint,
    profile: TypedClosureQualificationProfileKey,
    status: TypedClosureQualificationStatus,
}

impl TypedClosureQualification {
    pub fn new(
        evidence: QualifiedClosureEvidence,
        claims: impl IntoIterator<Item = TransitionCapabilityClaim>,
        error_contract: TypedClosureErrorContract,
        implementation: ClosureModelImplementationFingerprint,
        profile: TypedClosureQualificationProfileKey,
        status: TypedClosureQualificationStatus,
    ) -> Result<Self, TypedClosureAuthorityError> {
        let claims = claims.into_iter().collect::<BTreeSet<_>>();
        if claims.is_empty() {
            return Err(TypedClosureAuthorityError::EmptyClaimSet {
                lineage: evidence.evidence_lineage(),
            });
        }
        for claim in &claims {
            match claim.evidence() {
                CapabilityEvidence::QualifiedClosure(claim_evidence)
                    if claim_evidence == evidence => {}
                _ => {
                    return Err(TypedClosureAuthorityError::ClaimEvidenceMismatch {
                        lineage: evidence.evidence_lineage(),
                        claim: *claim,
                    });
                }
            }
        }
        if error_contract.max_error_ppm() != evidence.error_ppm() {
            return Err(TypedClosureAuthorityError::ErrorBoundMismatch {
                lineage: evidence.evidence_lineage(),
            });
        }

        Ok(Self {
            evidence,
            claims,
            error_contract,
            implementation,
            profile,
            status,
        })
    }

    pub const fn evidence(&self) -> QualifiedClosureEvidence {
        self.evidence
    }

    pub const fn lineage(&self) -> EvidenceLineageToken {
        self.evidence.evidence_lineage()
    }

    pub const fn model_version(&self) -> ClosureModelVersion {
        self.evidence.model_version()
    }

    pub const fn domain(&self) -> ClosureDomainToken {
        self.evidence.domain()
    }

    pub fn claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.claims
    }

    pub const fn error_contract(&self) -> TypedClosureErrorContract {
        self.error_contract
    }

    pub const fn implementation(&self) -> &ClosureModelImplementationFingerprint {
        &self.implementation
    }

    pub const fn profile(&self) -> TypedClosureQualificationProfileKey {
        self.profile
    }

    pub const fn status(&self) -> TypedClosureQualificationStatus {
        self.status
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedClosureRegistryKey {
    id: u128,
    version: u32,
}

impl TypedClosureRegistryKey {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureRegistryBuilder {
    key: TypedClosureRegistryKey,
    qualifications: BTreeMap<EvidenceLineageToken, TypedClosureQualification>,
}

impl TypedClosureRegistryBuilder {
    pub const fn new(key: TypedClosureRegistryKey) -> Self {
        Self {
            key,
            qualifications: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        qualification: TypedClosureQualification,
    ) -> Result<(), TypedClosureAuthorityError> {
        use std::collections::btree_map::Entry;
        let lineage = qualification.lineage();
        match self.qualifications.entry(lineage) {
            Entry::Vacant(entry) => {
                entry.insert(qualification);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &qualification => Ok(()),
            Entry::Occupied(_) => {
                Err(TypedClosureAuthorityError::ConflictingRegistration { lineage })
            }
        }
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<TypedClosureRegistry, TypedClosureAuthorityError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(TypedClosureAuthorityError::PolicyIdentity)?;

        for qualification in self.qualifications.values() {
            validate_against_policy(policy, qualification)?;
        }

        let authority = TypedClosureAuthorityStamp {
            key: self.key,
            policy_authority: policy.authority_stamp().clone(),
            qualifications: self.qualifications.clone(),
        };
        Ok(TypedClosureRegistry {
            qualifications: self.qualifications,
            authority,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureAuthorityStamp {
    key: TypedClosureRegistryKey,
    policy_authority: InformationPolicyAuthorityStamp,
    qualifications: BTreeMap<EvidenceLineageToken, TypedClosureQualification>,
}

impl TypedClosureAuthorityStamp {
    pub const fn key(&self) -> TypedClosureRegistryKey {
        self.key
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedClosureRegistry {
    qualifications: BTreeMap<EvidenceLineageToken, TypedClosureQualification>,
    authority: TypedClosureAuthorityStamp,
}

impl TypedClosureRegistry {
    pub const fn authority_stamp(&self) -> &TypedClosureAuthorityStamp {
        &self.authority
    }

    fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), TypedClosureAuthorityError> {
        self.authority
            .policy_authority
            .validate_registry(policy.registry())
            .map_err(TypedClosureAuthorityError::PolicyIdentity)
    }

    fn authenticate(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        lineage: EvidenceLineageToken,
        required_claims: BTreeSet<TransitionCapabilityClaim>,
    ) -> Result<AuthenticatedTypedClosureEvidence, TypedClosureAuthorityError> {
        self.validate_policy(policy)?;
        let qualification = self
            .qualifications
            .get(&lineage)
            .ok_or(TypedClosureAuthorityError::UnknownLineage { lineage })?;
        if qualification.status() != TypedClosureQualificationStatus::Qualified {
            return Err(TypedClosureAuthorityError::QualificationNotCurrent {
                lineage,
                status: qualification.status(),
            });
        }
        if !required_claims.is_subset(qualification.claims()) {
            return Err(TypedClosureAuthorityError::RequiredClaimsNotQualified {
                lineage,
                required: required_claims,
                qualified: qualification.claims().clone(),
            });
        }

        Ok(AuthenticatedTypedClosureEvidence {
            registry_authority: self.authority.clone(),
            qualification: qualification.clone(),
            required_claims,
        })
    }
}

/// Authenticated R2 evidence. This still requires process-side acceptance (#352).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedTypedClosureEvidence {
    registry_authority: TypedClosureAuthorityStamp,
    qualification: TypedClosureQualification,
    required_claims: BTreeSet<TransitionCapabilityClaim>,
}

impl AuthenticatedTypedClosureEvidence {
    pub const fn lineage(&self) -> EvidenceLineageToken {
        self.qualification.lineage()
    }

    pub const fn qualification(&self) -> &TypedClosureQualification {
        &self.qualification
    }

    pub fn required_claims(&self) -> &BTreeSet<TransitionCapabilityClaim> {
        &self.required_claims
    }

    pub const fn registry_authority(&self) -> &TypedClosureAuthorityStamp {
        &self.registry_authority
    }
}

/// Candidate whose R2 evidence has been authenticated but **not accepted** by a
/// consuming process. `pending_process_acceptance` therefore remains explicit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClosureQualifiedCandidateRequest {
    candidate_index: usize,
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    closure_authority: TypedClosureAuthorityStamp,
    identity: CandidatePathIdentity,
    authenticated: BTreeMap<EvidenceLineageToken, AuthenticatedTypedClosureEvidence>,
    pending_process_acceptance: BTreeSet<EvidenceLineageToken>,
    remaining_nonclosure_obligations: BTreeSet<CandidateEvidenceObligation>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl ClosureQualifiedCandidateRequest {
    pub fn authenticate(
        requests: &CandidateCertificationRequestSet,
        candidate_index: usize,
        registry: &TypedClosureRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    ) -> Result<Self, TypedClosureAuthorityError> {
        requests
            .validate_current(applicability)
            .map_err(TypedClosureAuthorityError::CandidateRequest)?;
        registry.validate_policy(policy)?;
        if requests.information_policy_authority() != registry.authority_stamp().policy_authority() {
            return Err(TypedClosureAuthorityError::PolicyAuthorityMismatch);
        }

        let request = requests
            .requests()
            .get(candidate_index)
            .ok_or(TypedClosureAuthorityError::CandidateIndexOutOfRange {
                index: candidate_index,
                len: requests.requests().len(),
            })?;

        let mut authenticated = BTreeMap::new();
        let mut pending_process_acceptance = BTreeSet::new();
        let mut remaining_nonclosure_obligations = BTreeSet::new();

        for obligation in request.obligations().iter().copied() {
            match obligation {
                CandidateEvidenceObligation::QualifiedClosure(lineage) => {
                    let required_claims = collect_r2_claims(
                        applicability,
                        request.identity(),
                        lineage,
                    )?;
                    authenticated.insert(
                        lineage,
                        registry.authenticate(policy, lineage, required_claims)?,
                    );
                    pending_process_acceptance.insert(lineage);
                }
                other => {
                    remaining_nonclosure_obligations.insert(other);
                }
            }
        }

        Ok(Self {
            candidate_index,
            applicability_authority: requests.applicability_authority().clone(),
            closure_authority: registry.authority_stamp().clone(),
            identity: request.identity().clone(),
            authenticated,
            pending_process_acceptance,
            remaining_nonclosure_obligations,
            discarded_information: request.discarded_information().clone(),
        })
    }

    pub fn authenticated(
        &self,
    ) -> &BTreeMap<EvidenceLineageToken, AuthenticatedTypedClosureEvidence> {
        &self.authenticated
    }

    pub fn pending_process_acceptance(&self) -> &BTreeSet<EvidenceLineageToken> {
        &self.pending_process_acceptance
    }

    pub fn remaining_nonclosure_obligations(&self) -> &BTreeSet<CandidateEvidenceObligation> {
        &self.remaining_nonclosure_obligations
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }

    pub fn validate_current(
        &self,
        requests: &CandidateCertificationRequestSet,
        registry: &TypedClosureRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    ) -> Result<(), TypedClosureAuthorityError> {
        if requests.applicability_authority() != &self.applicability_authority {
            return Err(TypedClosureAuthorityError::ApplicabilityAuthorityChanged);
        }
        if registry.authority_stamp() != &self.closure_authority {
            return Err(TypedClosureAuthorityError::ClosureAuthorityChanged);
        }
        let current = Self::authenticate(requests, self.candidate_index, registry, policy, applicability)?;
        if current != *self {
            return Err(TypedClosureAuthorityError::ResolutionChanged);
        }
        Ok(())
    }
}

fn collect_r2_claims(
    applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    identity: &CandidatePathIdentity,
    lineage: EvidenceLineageToken,
) -> Result<BTreeSet<TransitionCapabilityClaim>, TypedClosureAuthorityError> {
    let mut claims = BTreeSet::new();
    for transition_key in identity.transitions() {
        let transition = applicability
            .transitions()
            .registry()
            .transitions()
            .find_map(|(key, transition)| (key == transition_key).then_some(transition))
            .ok_or(TypedClosureAuthorityError::TransitionMissing {
                transition: *transition_key,
            })?;
        for (claim, provenance) in transition.introductions() {
            if matches!(
                provenance,
                PromotionProvenance::QualifiedClosure { evidence_lineage } if *evidence_lineage == lineage
            ) {
                match claim.evidence() {
                    CapabilityEvidence::QualifiedClosure(evidence)
                        if evidence.evidence_lineage() == lineage => {
                            claims.insert(*claim);
                        }
                    _ => {
                        return Err(TypedClosureAuthorityError::CandidateClaimEvidenceMismatch {
                            lineage,
                            claim: *claim,
                        });
                    }
                }
            }
        }
    }
    if claims.is_empty() {
        return Err(TypedClosureAuthorityError::NoClaimsForLineage { lineage });
    }
    Ok(claims)
}

fn validate_against_policy(
    policy: &ManifestBoundInformationPolicyRegistry<'_>,
    qualification: &TypedClosureQualification,
) -> Result<(), TypedClosureAuthorityError> {
    let lineage = qualification.lineage();
    let record = policy
        .registry()
        .closure_evidence_records()
        .find_map(|(key, record)| (*key == lineage).then_some(*record))
        .ok_or(TypedClosureAuthorityError::PolicyClosureEvidenceMissing { lineage })?;
    if record.evidence() != qualification.evidence() {
        return Err(TypedClosureAuthorityError::PolicyClosureEvidenceMismatch { lineage });
    }
    if record.status() != ClosureEvidenceStatus::Qualified {
        return Err(TypedClosureAuthorityError::PolicyClosureEvidenceRevoked { lineage });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypedClosureAuthorityError {
    ZeroEvaluationHorizon,
    MetricAggregationMismatch {
        metric: ClosureErrorMetric,
        aggregation: ClosureAggregationSemantics,
    },
    MetricZeroReferenceMismatch {
        metric: ClosureErrorMetric,
        zero_reference: ClosureZeroReferencePolicy,
    },
    EmptyImplementationFingerprint,
    EmptyClaimSet { lineage: EvidenceLineageToken },
    ClaimEvidenceMismatch {
        lineage: EvidenceLineageToken,
        claim: TransitionCapabilityClaim,
    },
    ErrorBoundMismatch { lineage: EvidenceLineageToken },
    ConflictingRegistration { lineage: EvidenceLineageToken },
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyClosureEvidenceMissing { lineage: EvidenceLineageToken },
    PolicyClosureEvidenceMismatch { lineage: EvidenceLineageToken },
    PolicyClosureEvidenceRevoked { lineage: EvidenceLineageToken },
    UnknownLineage { lineage: EvidenceLineageToken },
    QualificationNotCurrent {
        lineage: EvidenceLineageToken,
        status: TypedClosureQualificationStatus,
    },
    RequiredClaimsNotQualified {
        lineage: EvidenceLineageToken,
        required: BTreeSet<TransitionCapabilityClaim>,
        qualified: BTreeSet<TransitionCapabilityClaim>,
    },
    CandidateRequest(CandidateCertificationRequestError),
    PolicyAuthorityMismatch,
    CandidateIndexOutOfRange { index: usize, len: usize },
    TransitionMissing { transition: InformationTransitionKey },
    CandidateClaimEvidenceMismatch {
        lineage: EvidenceLineageToken,
        claim: TransitionCapabilityClaim,
    },
    NoClaimsForLineage { lineage: EvidenceLineageToken },
    ApplicabilityAuthorityChanged,
    ClosureAuthorityChanged,
    ResolutionChanged,
    PolicyRegistry { source: InformationRegistryError },
}

impl fmt::Display for TypedClosureAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for TypedClosureAuthorityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(source) => Some(source),
            Self::CandidateRequest(source) => Some(source),
            Self::PolicyRegistry { source } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate_applicability::ApplicableTransitionCandidateSet;
    use crate::information::{
        ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel, RepresentationCapabilities,
    };
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
        RegisteredClosureEvidence,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, InformationTransitionRegistryKey, LosslessTransformKey,
    };
    use crate::transition_applicability::{
        TransitionApplicability, TransitionApplicabilityPolicy, TransitionApplicabilityPolicyBuilder,
        TransitionApplicabilityPolicyKey,
    };
    use crate::transition_candidates::TransitionCandidateSearchLimits;
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(4_000, 1);
    const SOURCE: crate::information::RepresentationKey =
        crate::information::RepresentationKey::new(4_010, 1);
    const TARGET: crate::information::RepresentationKey =
        crate::information::RepresentationKey::new(4_011, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(4_020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(4_021, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(4_030, 1);
    const LINEAGE: EvidenceLineageToken = EvidenceLineageToken(4_040);
    const PROFILE: TypedClosureQualificationProfileKey =
        TypedClosureQualificationProfileKey::new(4_050, 1);
    const REGISTRY: TypedClosureRegistryKey = TypedClosureRegistryKey::new(4_051, 1);
    const OBSERVABLE: ClosureObservableKey = ClosureObservableKey::new(4_060, 1);
    const TRANSFORM: LosslessTransformKey = LosslessTransformKey(4_070, 1);

    fn evidence() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(3),
            ClosureDomainToken(7),
            ErrorPpm::ONE_PERCENT,
            LINEAGE,
        )
    }

    fn closure_claim() -> TransitionCapabilityClaim {
        TransitionCapabilityClaim::new(
            EcologicalInformation::SpatialStructure,
            CapabilityEvidence::QualifiedClosure(evidence()),
        )
    }

    fn capabilities(
        key: crate::information::RepresentationKey,
        claims: impl IntoIterator<Item = (EcologicalInformation, CapabilityEvidence)>,
    ) -> RepresentationCapabilities {
        RepresentationCapabilities::new(key, EcologicalAuthorityLevel::Coarse, claims)
    }

    fn policy(status: ClosureEvidenceStatus) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(evidence(), status))
            .unwrap();
        builder
            .register_representation(capabilities(
                SOURCE,
                [(EcologicalInformation::Headcount, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        builder
            .register_representation(capabilities(
                TARGET,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::SpatialStructure,
                        CapabilityEvidence::QualifiedClosure(evidence()),
                    ),
                    (EcologicalInformation::DiseaseState, CapabilityEvidence::Exact),
                ],
            ))
            .unwrap();
        builder.seal()
    }

    fn transitions(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
        let edge = InformationTransitionDefinition::new(
            EDGE,
            SOURCE,
            TARGET,
            [
                (
                    closure_claim(),
                    PromotionProvenance::QualifiedClosure {
                        evidence_lineage: LINEAGE,
                    },
                ),
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::DiseaseState,
                        CapabilityEvidence::Exact,
                    ),
                    PromotionProvenance::LosslessDerivation {
                        transform: TRANSFORM,
                    },
                ),
            ],
            [EcologicalInformation::Headcount],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder.register_transition(edge).unwrap();
        builder.seal(policy).unwrap()
    }

    fn applicability(transitions: &InformationTransitionRegistry) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APP);
        builder.register(EDGE, TransitionApplicability::Universal).unwrap();
        builder.seal(transitions).unwrap()
    }

    fn error_contract(
        metric: ClosureErrorMetric,
        horizon: u64,
    ) -> TypedClosureErrorContract {
        let aggregation = match metric {
            ClosureErrorMetric::MeanRelativePpm => ClosureAggregationSemantics::MeanOverHorizon,
            ClosureErrorMetric::MaximumRelativePpm => ClosureAggregationSemantics::MaximumOverHorizon,
            ClosureErrorMetric::EventProbabilityAbsolutePpm => {
                ClosureAggregationSemantics::EventProbabilityOverHorizon
            }
        };
        let zero = match metric {
            ClosureErrorMetric::MeanRelativePpm | ClosureErrorMetric::MaximumRelativePpm => {
                ClosureZeroReferencePolicy::RejectZeroReference
            }
            ClosureErrorMetric::EventProbabilityAbsolutePpm => ClosureZeroReferencePolicy::NotApplicable,
        };
        TypedClosureErrorContract::new(
            OBSERVABLE,
            metric,
            ErrorPpm::ONE_PERCENT,
            ClosureEvaluationHorizonTicks::new(horizon).unwrap(),
            aggregation,
            zero,
        )
        .unwrap()
    }

    fn qualification(contract: TypedClosureErrorContract) -> TypedClosureQualification {
        TypedClosureQualification::new(
            evidence(),
            [closure_claim()],
            contract,
            ClosureModelImplementationFingerprint::new(b"closure-model-a".to_vec()).unwrap(),
            PROFILE,
            TypedClosureQualificationStatus::Qualified,
        )
        .unwrap()
    }

    fn exact_context<'a>(
        policy: &'a InformationPolicyRegistry,
        transitions: &'a InformationTransitionRegistry,
        applicability: &'a TransitionApplicabilityPolicy,
    ) -> (
        ManifestBoundInformationPolicyRegistry<'a>,
        ManifestBoundTransitionApplicabilityPolicy<'a>,
        CandidateCertificationRequestSet,
    ) {
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let transitions_exact =
            ManifestBoundInformationTransitionRegistry::new(transitions, &policy_exact).unwrap();
        let applicability_exact =
            ManifestBoundTransitionApplicabilityPolicy::new(applicability, &transitions_exact)
                .unwrap();
        let candidates = transitions_exact
            .registry()
            .discover_candidates(
                transitions_exact.policy().registry(),
                SOURCE,
                TARGET,
                TransitionCandidateSearchLimits::new(4, 8).unwrap(),
            )
            .unwrap();
        let annotated: ApplicableTransitionCandidateSet = applicability_exact
            .policy()
            .annotate_candidate_set(&candidates)
            .unwrap();
        let requests =
            CandidateCertificationRequestSet::prepare(&applicability_exact, &annotated).unwrap();
        (policy_exact, applicability_exact, requests)
    }

    #[test]
    fn typed_closure_authenticates_but_remains_pending_process_acceptance() {
        let policy = policy(ClosureEvidenceStatus::Qualified);
        let transitions = transitions(&policy);
        let applicability = applicability(&transitions);
        let (policy_exact, applicability_exact, requests) =
            exact_context(&policy, &transitions, &applicability);
        let mut builder = TypedClosureRegistryBuilder::new(REGISTRY);
        builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MeanRelativePpm,
                10,
            )))
            .unwrap();
        let registry = builder.seal(&policy_exact).unwrap();

        let resolved = ClosureQualifiedCandidateRequest::authenticate(
            &requests,
            0,
            &registry,
            &policy_exact,
            &applicability_exact,
        )
        .unwrap();

        assert!(resolved.authenticated().contains_key(&LINEAGE));
        assert!(resolved.pending_process_acceptance().contains(&LINEAGE));
        assert!(resolved
            .remaining_nonclosure_obligations()
            .contains(&CandidateEvidenceObligation::LosslessTransform(TRANSFORM)));
        assert!(resolved
            .discarded_information()
            .contains(&EcologicalInformation::Headcount));
    }

    #[test]
    fn same_numeric_error_with_different_metric_is_different_authority() {
        let policy = policy(ClosureEvidenceStatus::Qualified);
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(&policy);
        let mut mean_builder = TypedClosureRegistryBuilder::new(REGISTRY);
        mean_builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MeanRelativePpm,
                10,
            )))
            .unwrap();
        let mean = mean_builder.seal(&policy_exact).unwrap();

        let mut max_builder = TypedClosureRegistryBuilder::new(REGISTRY);
        max_builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MaximumRelativePpm,
                10,
            )))
            .unwrap();
        let max = max_builder.seal(&policy_exact).unwrap();

        assert_ne!(mean.authority_stamp(), max.authority_stamp());
    }

    #[test]
    fn one_step_and_long_horizon_are_different_authority() {
        let policy = policy(ClosureEvidenceStatus::Qualified);
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(&policy);
        let mut short_builder = TypedClosureRegistryBuilder::new(REGISTRY);
        short_builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MeanRelativePpm,
                1,
            )))
            .unwrap();
        let short = short_builder.seal(&policy_exact).unwrap();

        let mut long_builder = TypedClosureRegistryBuilder::new(REGISTRY);
        long_builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MeanRelativePpm,
                1_000,
            )))
            .unwrap();
        let long = long_builder.seal(&policy_exact).unwrap();

        assert_ne!(short.authority_stamp(), long.authority_stamp());
    }

    #[test]
    fn revoked_low_level_closure_lineage_cannot_be_typed_as_current() {
        let policy = policy(ClosureEvidenceStatus::Revoked);
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(&policy);
        let mut builder = TypedClosureRegistryBuilder::new(REGISTRY);
        builder
            .register(qualification(error_contract(
                ClosureErrorMetric::MeanRelativePpm,
                10,
            )))
            .unwrap();
        assert_eq!(
            builder.seal(&policy_exact),
            Err(TypedClosureAuthorityError::PolicyClosureEvidenceRevoked {
                lineage: LINEAGE,
            })
        );
    }
}
