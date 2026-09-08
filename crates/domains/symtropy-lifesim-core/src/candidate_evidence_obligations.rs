// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Fail-closed evidence obligations for transition candidates.
//!
//! This module is deliberately **not** an admissibility certificate issuer.
//! Full admission depends on authority producers that do not yet exist at this
//! layer (state-domain proofs, qualified R1 transforms, spatiotemporal evidence,
//! typed closure/horizon acceptance, and R0/R3 authority resolution).
//!
//! Instead, one exact applicability authority is used to re-run bounded candidate
//! discovery and applicability annotation, prove that a supplied candidate set is
//! current, and derive the unresolved evidence obligations from that canonical
//! output. There is no `admitted` field, no public certificate constructor, and no
//! path from an obligation request to ecological mutation.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::{
    ApplicabilityPolicyIdentityError, ManifestBoundTransitionApplicabilityPolicy,
    TransitionApplicabilityAuthorityStamp,
};
use crate::candidate_applicability::{
    ApplicableTransitionCandidate, ApplicableTransitionCandidateSet, CandidateApplicabilityError,
};
use crate::information::{EcologicalInformation, EvidenceLineageToken, RepresentationKey};
use crate::information_policy_manifest::InformationPolicyAuthorityStamp;
use crate::information_registry::InformationPolicyRegistryKey;
use crate::information_transition::{
    ConditionalDerivationKey, InformationTransitionKey, InformationTransitionRegistryKey,
    LosslessTransformKey, MeasurementAuthorityKey, PromotionEvidenceRequirement,
    RetainedAuthorityKey,
};
use crate::transition_applicability::{TransitionApplicabilityPolicyKey, TransitionDomainKey};
use crate::transition_candidates::{
    TransitionCandidateError, TransitionCandidateSearchLimits,
};
use crate::transition_policy_manifest::InformationTransitionAuthorityStamp;

/// Exact identity of one candidate path inside one exact request-set context.
///
/// There is intentionally no public constructor. Candidate identities are
/// derived only from canonical #311/#313 output after exact-corpus revalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePathIdentity {
    source: RepresentationKey,
    destination: RepresentationKey,
    transitions: Vec<InformationTransitionKey>,
}

impl CandidatePathIdentity {
    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub fn transitions(&self) -> &[InformationTransitionKey] {
        &self.transitions
    }
}

/// One unresolved evidence/authority obligation implied by a canonical candidate.
///
/// These values name requirements. They are not proof that those requirements
/// are satisfied, qualified, fresh, or allowed by the eventual admission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CandidateEvidenceObligation {
    TransitionDomain(TransitionDomainKey),
    RetainedAuthority(RetainedAuthorityKey),
    LosslessTransform(LosslessTransformKey),
    QualifiedClosure(EvidenceLineageToken),
    MeasurementAuthority(MeasurementAuthorityKey),
    ConditionalDerivation(ConditionalDerivationKey),
}

/// Read-only certification request for one canonical candidate.
///
/// `discarded_information` is separated from evidence obligations because a
/// discard is a semantic consequence that a later admission policy may reject;
/// it is not evidence that can simply be "resolved" by supplying a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateCertificationRequest {
    identity: CandidatePathIdentity,
    obligations: BTreeSet<CandidateEvidenceObligation>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl CandidateCertificationRequest {
    pub const fn identity(&self) -> &CandidatePathIdentity {
        &self.identity
    }

    pub fn obligations(&self) -> &BTreeSet<CandidateEvidenceObligation> {
        &self.obligations
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }
}

/// Complete certification-request set for one exact candidate discovery context.
///
/// Request ordering preserves canonical discovery ordering only for replay and
/// inspection. It is not a preference, score, or selector input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateCertificationRequestSet {
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    source: RepresentationKey,
    destination: RepresentationKey,
    search_limits: TransitionCandidateSearchLimits,
    requests: Vec<CandidateCertificationRequest>,
}

impl CandidateCertificationRequestSet {
    /// Prepare obligations only after the supplied candidate set is reproduced
    /// exactly from the current exact policy/transition/applicability authority.
    ///
    /// This protects against a candidate set discovered under a different graph
    /// or applicability corpus that reused the same semantic keys.
    pub fn prepare(
        authority: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        supplied: &ApplicableTransitionCandidateSet,
    ) -> Result<Self, CandidateCertificationRequestError> {
        validate_semantic_lineage(authority, supplied)?;
        authority
            .authority_stamp()
            .validate(authority.policy(), authority.transitions())
            .map_err(CandidateCertificationRequestError::ApplicabilityIdentity)?;

        let canonical = rediscover_and_annotate(
            authority,
            supplied.source(),
            supplied.destination(),
            supplied.search_limits(),
        )?;
        if canonical != *supplied {
            return Err(CandidateCertificationRequestError::CandidateSetMismatch);
        }

        Ok(Self::from_canonical(
            authority.authority_stamp().clone(),
            &canonical,
        ))
    }

    pub const fn applicability_authority(&self) -> &TransitionApplicabilityAuthorityStamp {
        &self.applicability_authority
    }

    pub const fn transition_authority(&self) -> &InformationTransitionAuthorityStamp {
        self.applicability_authority.transition_authority()
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        self.applicability_authority
            .transition_authority()
            .policy_authority()
    }

    pub const fn source(&self) -> RepresentationKey {
        self.source
    }

    pub const fn destination(&self) -> RepresentationKey {
        self.destination
    }

    pub const fn search_limits(&self) -> TransitionCandidateSearchLimits {
        self.search_limits
    }

    pub fn requests(&self) -> &[CandidateCertificationRequest] {
        &self.requests
    }

    /// Revalidate exact corpora and re-run candidate discovery/annotation.
    ///
    /// Any corpus drift or change in the complete candidate set makes the request
    /// set stale. This still does not admit any candidate.
    pub fn validate_current(
        &self,
        authority: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    ) -> Result<(), CandidateCertificationRequestError> {
        self.applicability_authority
            .validate(authority.policy(), authority.transitions())
            .map_err(CandidateCertificationRequestError::ApplicabilityIdentity)?;

        let canonical = rediscover_and_annotate(
            authority,
            self.source,
            self.destination,
            self.search_limits,
        )?;
        let current = Self::from_canonical(authority.authority_stamp().clone(), &canonical);
        if current != *self {
            return Err(CandidateCertificationRequestError::RequestSetMismatch);
        }
        Ok(())
    }

    fn from_canonical(
        applicability_authority: TransitionApplicabilityAuthorityStamp,
        canonical: &ApplicableTransitionCandidateSet,
    ) -> Self {
        let requests = canonical
            .candidates()
            .iter()
            .map(request_from_candidate)
            .collect();

        Self {
            applicability_authority,
            source: canonical.source(),
            destination: canonical.destination(),
            search_limits: canonical.search_limits(),
            requests,
        }
    }
}

fn validate_semantic_lineage(
    authority: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    supplied: &ApplicableTransitionCandidateSet,
) -> Result<(), CandidateCertificationRequestError> {
    let applicability = authority.authority_stamp();
    if supplied.applicability_policy_key() != applicability.key() {
        return Err(CandidateCertificationRequestError::ApplicabilityPolicyKeyMismatch {
            expected: applicability.key(),
            actual: supplied.applicability_policy_key(),
        });
    }

    let transition = applicability.transition_authority();
    if supplied.transition_registry_key() != transition.key() {
        return Err(CandidateCertificationRequestError::TransitionRegistryKeyMismatch {
            expected: transition.key(),
            actual: supplied.transition_registry_key(),
        });
    }

    let information = transition.policy_authority();
    if supplied.policy_registry_key() != information.key() {
        return Err(CandidateCertificationRequestError::InformationPolicyKeyMismatch {
            expected: information.key(),
            actual: supplied.policy_registry_key(),
        });
    }

    Ok(())
}

fn rediscover_and_annotate(
    authority: &ManifestBoundTransitionApplicabilityPolicy<'_>,
    source: RepresentationKey,
    destination: RepresentationKey,
    limits: TransitionCandidateSearchLimits,
) -> Result<ApplicableTransitionCandidateSet, CandidateCertificationRequestError> {
    let transitions = authority.transitions();
    let structural = transitions
        .registry()
        .discover_candidates(transitions.policy().registry(), source, destination, limits)
        .map_err(CandidateCertificationRequestError::CandidateDiscovery)?;

    authority
        .policy()
        .annotate_candidate_set(&structural)
        .map_err(CandidateCertificationRequestError::CandidateApplicability)
}

fn request_from_candidate(candidate: &ApplicableTransitionCandidate) -> CandidateCertificationRequest {
    let mut obligations = BTreeSet::new();

    for domain in candidate.required_domains().iter().copied() {
        obligations.insert(CandidateEvidenceObligation::TransitionDomain(domain));
    }

    for requirement in candidate.candidate().external_requirements().iter().copied() {
        match requirement {
            PromotionEvidenceRequirement::RetainedAuthority(authority) => {
                obligations.insert(CandidateEvidenceObligation::RetainedAuthority(authority));
            }
            PromotionEvidenceRequirement::LosslessTransform(transform) => {
                obligations.insert(CandidateEvidenceObligation::LosslessTransform(transform));
            }
            PromotionEvidenceRequirement::MeasurementAuthority(authority) => {
                obligations.insert(CandidateEvidenceObligation::MeasurementAuthority(authority));
            }
        }
    }

    for lineage in candidate.candidate().closure_evidence().iter().copied() {
        obligations.insert(CandidateEvidenceObligation::QualifiedClosure(lineage));
    }

    for model in candidate
        .candidate()
        .conditional_derivations()
        .iter()
        .copied()
    {
        obligations.insert(CandidateEvidenceObligation::ConditionalDerivation(model));
    }

    CandidateCertificationRequest {
        identity: CandidatePathIdentity {
            source: candidate.source(),
            destination: candidate.destination(),
            transitions: candidate.transitions().to_vec(),
        },
        obligations,
        discarded_information: candidate.candidate().discarded_information().clone(),
    }
}

#[derive(Debug)]
pub enum CandidateCertificationRequestError {
    ApplicabilityIdentity(ApplicabilityPolicyIdentityError),
    CandidateDiscovery(TransitionCandidateError),
    CandidateApplicability(CandidateApplicabilityError),
    ApplicabilityPolicyKeyMismatch {
        expected: TransitionApplicabilityPolicyKey,
        actual: TransitionApplicabilityPolicyKey,
    },
    TransitionRegistryKeyMismatch {
        expected: InformationTransitionRegistryKey,
        actual: InformationTransitionRegistryKey,
    },
    InformationPolicyKeyMismatch {
        expected: InformationPolicyRegistryKey,
        actual: InformationPolicyRegistryKey,
    },
    CandidateSetMismatch,
    RequestSetMismatch,
}

impl fmt::Display for CandidateCertificationRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApplicabilityIdentity(error) => {
                write!(formatter, "exact applicability authority error: {error}")
            }
            Self::CandidateDiscovery(error) => {
                write!(formatter, "candidate discovery error: {error}")
            }
            Self::CandidateApplicability(error) => {
                write!(formatter, "candidate applicability error: {error}")
            }
            Self::ApplicabilityPolicyKeyMismatch { expected, actual } => write!(
                formatter,
                "candidate set uses applicability policy {actual:?}, expected {expected:?}"
            ),
            Self::TransitionRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "candidate set uses transition registry {actual:?}, expected {expected:?}"
            ),
            Self::InformationPolicyKeyMismatch { expected, actual } => write!(
                formatter,
                "candidate set uses information policy {actual:?}, expected {expected:?}"
            ),
            Self::CandidateSetMismatch => write!(
                formatter,
                "supplied candidate set does not match exact current discovery/applicability output"
            ),
            Self::RequestSetMismatch => write!(
                formatter,
                "prepared candidate evidence obligations are stale under current exact authority"
            ),
        }
    }
}

impl Error for CandidateCertificationRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ApplicabilityIdentity(error) => Some(error),
            Self::CandidateDiscovery(error) => Some(error),
            Self::CandidateApplicability(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, ClosureDomainToken, ClosureModelVersion, EcologicalAuthorityLevel,
        ErrorPpm, QualifiedClosureEvidence, RepresentationCapabilities,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        ClosureEvidenceStatus, InformationPolicyRegistry, InformationPolicyRegistryBuilder,
        RegisteredClosureEvidence,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, PromotionProvenance, TransitionCapabilityClaim,
    };
    use crate::transition_applicability::{
        TransitionApplicability, TransitionApplicabilityPolicy,
        TransitionApplicabilityPolicyBuilder,
    };
    use crate::transition_candidates::TransitionCandidateSearchLimits;
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(700, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(701, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(702, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(710, 1);
    const TARGET: RepresentationKey = RepresentationKey::new(711, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(720, 1);
    const DOMAIN: TransitionDomainKey = TransitionDomainKey::new(730, 1);

    const RETAINED: RetainedAuthorityKey = RetainedAuthorityKey(740, 1);
    const LOSSLESS: LosslessTransformKey = LosslessTransformKey(741, 1);
    const MEASUREMENT: MeasurementAuthorityKey = MeasurementAuthorityKey(742, 1);
    const CONDITIONAL: ConditionalDerivationKey = ConditionalDerivationKey(743, 1);
    const CLOSURE_LINEAGE: EvidenceLineageToken = EvidenceLineageToken(744);

    fn closure() -> QualifiedClosureEvidence {
        QualifiedClosureEvidence::new(
            ClosureModelVersion(4),
            ClosureDomainToken(55),
            ErrorPpm::new(1_000).unwrap(),
            CLOSURE_LINEAGE,
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let closure = closure();
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_closure_evidence(RegisteredClosureEvidence::new(
                closure,
                ClosureEvidenceStatus::Qualified,
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                SOURCE,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::SpatialStructure,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                TARGET,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    (
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    (
                        EcologicalInformation::OccupancyDistribution,
                        CapabilityEvidence::QualifiedClosure(closure),
                    ),
                    (EcologicalInformation::DiseaseState, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::GeneticSummary,
                        CapabilityEvidence::MeasurementOnly,
                    ),
                ],
            ))
            .unwrap();
        builder.seal()
    }

    fn graph(
        policy: &InformationPolicyRegistry,
        retained: RetainedAuthorityKey,
    ) -> InformationTransitionRegistry {
        let closure = closure();
        let edge = InformationTransitionDefinition::new(
            EDGE,
            SOURCE,
            TARGET,
            [
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    PromotionProvenance::RetainedExact { authority: retained },
                ),
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::ConditionDistribution,
                        CapabilityEvidence::Exact,
                    ),
                    PromotionProvenance::LosslessDerivation {
                        transform: LOSSLESS,
                    },
                ),
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::OccupancyDistribution,
                        CapabilityEvidence::QualifiedClosure(closure),
                    ),
                    PromotionProvenance::QualifiedClosure {
                        evidence_lineage: CLOSURE_LINEAGE,
                    },
                ),
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::DiseaseState,
                        CapabilityEvidence::Exact,
                    ),
                    PromotionProvenance::MeasurementAssimilation {
                        authority: MEASUREMENT,
                    },
                ),
                (
                    TransitionCapabilityClaim::new(
                        EcologicalInformation::GeneticSummary,
                        CapabilityEvidence::MeasurementOnly,
                    ),
                    PromotionProvenance::ConditionalMicrostate { model: CONDITIONAL },
                ),
            ],
            [EcologicalInformation::SpatialStructure],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder.register_transition(edge).unwrap();
        builder.seal(policy).unwrap()
    }

    fn applicability(
        graph: &InformationTransitionRegistry,
    ) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APP);
        builder
            .register(EDGE, TransitionApplicability::RegisteredDomain(DOMAIN))
            .unwrap();
        builder.seal(graph).unwrap()
    }

    fn candidate_set(
        graph: &InformationTransitionRegistry,
        policy: &InformationPolicyRegistry,
        applicability: &TransitionApplicabilityPolicy,
    ) -> ApplicableTransitionCandidateSet {
        let structural = graph
            .discover_candidates(
                policy,
                SOURCE,
                TARGET,
                TransitionCandidateSearchLimits::new(4, 8).unwrap(),
            )
            .unwrap();
        applicability.annotate_candidate_set(&structural).unwrap()
    }

    #[test]
    fn request_preserves_every_r0_through_r4_and_domain_obligation() {
        let policy = policy();
        let graph = graph(&policy, RETAINED);
        let applicability = applicability(&graph);
        let supplied = candidate_set(&graph, &policy, &applicability);
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        let exact =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability, &bound_graph).unwrap();

        let requests = CandidateCertificationRequestSet::prepare(&exact, &supplied).unwrap();
        let request = &requests.requests()[0];

        assert_eq!(
            request.obligations(),
            &BTreeSet::from([
                CandidateEvidenceObligation::TransitionDomain(DOMAIN),
                CandidateEvidenceObligation::RetainedAuthority(RETAINED),
                CandidateEvidenceObligation::LosslessTransform(LOSSLESS),
                CandidateEvidenceObligation::QualifiedClosure(CLOSURE_LINEAGE),
                CandidateEvidenceObligation::MeasurementAuthority(MEASUREMENT),
                CandidateEvidenceObligation::ConditionalDerivation(CONDITIONAL),
            ])
        );
        assert_eq!(
            request.discarded_information(),
            &BTreeSet::from([EcologicalInformation::SpatialStructure])
        );
        assert_eq!(request.identity().transitions(), &[EDGE]);
        assert_eq!(requests.applicability_authority(), exact.authority_stamp());
        assert_eq!(
            requests.transition_authority(),
            exact.authority_stamp().transition_authority()
        );
        assert_eq!(
            requests.information_policy_authority(),
            exact
                .authority_stamp()
                .transition_authority()
                .policy_authority()
        );
    }

    #[test]
    fn same_semantic_keys_but_changed_graph_rejects_old_candidate_set() {
        let policy = policy();
        let graph_a = graph(&policy, RETAINED);
        let graph_b = graph(&policy, RetainedAuthorityKey(745, 1));
        let applicability_a = applicability(&graph_a);
        let applicability_b = applicability(&graph_b);
        let supplied_a = candidate_set(&graph_a, &policy, &applicability_a);

        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph_b =
            ManifestBoundInformationTransitionRegistry::new(&graph_b, &bound_policy).unwrap();
        let exact_b =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability_b, &bound_graph_b)
                .unwrap();

        assert!(matches!(
            CandidateCertificationRequestSet::prepare(&exact_b, &supplied_a),
            Err(CandidateCertificationRequestError::CandidateSetMismatch)
        ));
    }

    #[test]
    fn prepared_obligations_stale_when_exact_authority_changes() {
        let policy = policy();
        let graph_a = graph(&policy, RETAINED);
        let graph_b = graph(&policy, RetainedAuthorityKey(745, 1));
        let applicability_a = applicability(&graph_a);
        let applicability_b = applicability(&graph_b);
        let supplied_a = candidate_set(&graph_a, &policy, &applicability_a);

        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph_a =
            ManifestBoundInformationTransitionRegistry::new(&graph_a, &bound_policy).unwrap();
        let bound_graph_b =
            ManifestBoundInformationTransitionRegistry::new(&graph_b, &bound_policy).unwrap();
        let exact_a =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability_a, &bound_graph_a)
                .unwrap();
        let exact_b =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability_b, &bound_graph_b)
                .unwrap();

        let requests = CandidateCertificationRequestSet::prepare(&exact_a, &supplied_a).unwrap();
        assert!(matches!(
            requests.validate_current(&exact_b),
            Err(CandidateCertificationRequestError::ApplicabilityIdentity(_))
        ));
    }

    #[test]
    fn identical_exact_context_revalidates_without_admitting_anything() {
        let policy = policy();
        let graph = graph(&policy, RETAINED);
        let applicability = applicability(&graph);
        let supplied = candidate_set(&graph, &policy, &applicability);
        let bound_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let bound_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &bound_policy).unwrap();
        let exact =
            ManifestBoundTransitionApplicabilityPolicy::new(&applicability, &bound_graph).unwrap();

        let requests = CandidateCertificationRequestSet::prepare(&exact, &supplied).unwrap();
        requests.validate_current(&exact).unwrap();
        assert_eq!(requests.requests().len(), 1);
    }
}
