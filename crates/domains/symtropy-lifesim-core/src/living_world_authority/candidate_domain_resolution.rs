// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Candidate-local resolution of transition-domain evidence obligations.
//!
//! [`crate::candidate_evidence_obligations`] derives every unresolved evidence
//! requirement for each exact transition candidate. This module resolves only
//! `TransitionDomain` requirements through the qualified, revision-scoped domain
//! authority in [`super::transition_domain`]. All R0/R1/R2/R3/R4 obligations and
//! information discards remain untouched, and no `Admitted` state exists here.

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
use crate::information::EcologicalInformation;
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::population::PopulationState;
use crate::transition_applicability::TransitionDomainKey;

use super::transition_domain::{
    TransitionDomainAuthorityError, TransitionDomainAuthorityRegistry,
    TransitionDomainAuthorityStamp, TransitionDomainEvaluationSubject, TransitionDomainProof,
};

/// One canonical candidate after only its explicit transition-domain obligations
/// have been resolved.
///
/// This is **not** an admissibility certificate. `remaining_obligations` may
/// still contain R0/R1/R2/R3/R4 requirements, and information loss remains an
/// independent semantic consequence for later admission policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainResolvedCandidateRequest {
    candidate_index: usize,
    applicability_authority: TransitionApplicabilityAuthorityStamp,
    domain_authority: TransitionDomainAuthorityStamp,
    identity: CandidatePathIdentity,
    subject: TransitionDomainEvaluationSubject,
    resolved_domains: BTreeMap<TransitionDomainKey, TransitionDomainProof>,
    remaining_obligations: BTreeSet<CandidateEvidenceObligation>,
    discarded_information: BTreeSet<EcologicalInformation>,
}

impl DomainResolvedCandidateRequest {
    /// Resolve only the domain obligations of one candidate.
    ///
    /// The complete #335 request set is revalidated first. Domain proofs are then
    /// minted by the sealed #344 authority from the exact current subject and
    /// canonical population. Callers cannot inject proof objects or booleans.
    #[allow(clippy::too_many_arguments)]
    pub fn resolve(
        requests: &CandidateCertificationRequestSet,
        candidate_index: usize,
        domain_registry: &TransitionDomainAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<Self, CandidateDomainResolutionError> {
        requests
            .validate_current(applicability)
            .map_err(CandidateDomainResolutionError::CandidateRequest)?;

        let request = requests.requests().get(candidate_index).ok_or(
            CandidateDomainResolutionError::CandidateIndexOutOfRange {
                index: candidate_index,
                len: requests.requests().len(),
            },
        )?;

        subject
            .validate_population(population)
            .map_err(CandidateDomainResolutionError::DomainAuthority)?;

        let mut resolved_domains = BTreeMap::new();
        let mut remaining_obligations = BTreeSet::new();

        for obligation in request.obligations().iter().copied() {
            match obligation {
                CandidateEvidenceObligation::TransitionDomain(domain) => {
                    let proof = domain_registry
                        .evaluate_population_domain(
                            policy,
                            applicability,
                            domain,
                            subject,
                            population,
                        )
                        .map_err(CandidateDomainResolutionError::DomainAuthority)?;
                    resolved_domains.insert(domain, proof);
                }
                other => {
                    remaining_obligations.insert(other);
                }
            }
        }

        Ok(Self {
            candidate_index,
            applicability_authority: requests.applicability_authority().clone(),
            domain_authority: domain_registry.authority_stamp().clone(),
            identity: request.identity().clone(),
            subject: subject.clone(),
            resolved_domains,
            remaining_obligations,
            discarded_information: request.discarded_information().clone(),
        })
    }

    pub const fn candidate_index(&self) -> usize {
        self.candidate_index
    }

    pub const fn applicability_authority(&self) -> &TransitionApplicabilityAuthorityStamp {
        &self.applicability_authority
    }

    pub const fn domain_authority(&self) -> &TransitionDomainAuthorityStamp {
        &self.domain_authority
    }

    pub const fn identity(&self) -> &CandidatePathIdentity {
        &self.identity
    }

    pub const fn subject(&self) -> &TransitionDomainEvaluationSubject {
        &self.subject
    }

    pub fn resolved_domains(&self) -> &BTreeMap<TransitionDomainKey, TransitionDomainProof> {
        &self.resolved_domains
    }

    pub fn remaining_obligations(&self) -> &BTreeSet<CandidateEvidenceObligation> {
        &self.remaining_obligations
    }

    pub fn discarded_information(&self) -> &BTreeSet<EcologicalInformation> {
        &self.discarded_information
    }

    /// Revalidate and recompute this partial resolution under the exact current
    /// request/domain/applicability/source context.
    ///
    /// This still does not admit or select the candidate.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        requests: &CandidateCertificationRequestSet,
        domain_registry: &TransitionDomainAuthorityRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        subject: &TransitionDomainEvaluationSubject,
        population: &PopulationState,
    ) -> Result<(), CandidateDomainResolutionError> {
        requests
            .validate_current(applicability)
            .map_err(CandidateDomainResolutionError::CandidateRequest)?;

        if requests.applicability_authority() != &self.applicability_authority {
            return Err(CandidateDomainResolutionError::ApplicabilityAuthorityChanged);
        }
        if domain_registry.authority_stamp() != &self.domain_authority {
            return Err(CandidateDomainResolutionError::DomainAuthorityChanged);
        }
        if subject != &self.subject {
            return Err(CandidateDomainResolutionError::EvaluationSubjectChanged);
        }
        subject
            .validate_population(population)
            .map_err(CandidateDomainResolutionError::DomainAuthority)?;

        for proof in self.resolved_domains.values() {
            proof
                .validate_current(
                    domain_registry,
                    policy,
                    applicability,
                    subject,
                    population,
                )
                .map_err(CandidateDomainResolutionError::DomainAuthority)?;
        }

        let current = Self::resolve(
            requests,
            self.candidate_index,
            domain_registry,
            policy,
            applicability,
            subject,
            population,
        )?;
        if current != *self {
            return Err(CandidateDomainResolutionError::ResolutionChanged);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum CandidateDomainResolutionError {
    CandidateRequest(CandidateCertificationRequestError),
    CandidateIndexOutOfRange { index: usize, len: usize },
    DomainAuthority(TransitionDomainAuthorityError),
    ApplicabilityAuthorityChanged,
    DomainAuthorityChanged,
    EvaluationSubjectChanged,
    ResolutionChanged,
}

impl fmt::Display for CandidateDomainResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CandidateRequest(error) => {
                write!(formatter, "candidate certification request error: {error}")
            }
            Self::CandidateIndexOutOfRange { index, len } => write!(
                formatter,
                "candidate index {index} is outside the canonical request set of length {len}"
            ),
            Self::DomainAuthority(error) => {
                write!(formatter, "transition-domain authority error: {error}")
            }
            Self::ApplicabilityAuthorityChanged => write!(
                formatter,
                "exact applicability authority changed after candidate-domain resolution"
            ),
            Self::DomainAuthorityChanged => write!(
                formatter,
                "transition-domain authority corpus changed after candidate-domain resolution"
            ),
            Self::EvaluationSubjectChanged => write!(
                formatter,
                "transition-domain evaluation subject changed after candidate-domain resolution"
            ),
            Self::ResolutionChanged => write!(
                formatter,
                "candidate domain resolution changed under the current exact authority context"
            ),
        }
    }
}

impl Error for CandidateDomainResolutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CandidateRequest(error) => Some(error),
            Self::DomainAuthority(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::candidate_applicability::ApplicableTransitionCandidateSet;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities, RepresentationKey,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionKey,
        InformationTransitionRegistry, InformationTransitionRegistryBuilder,
        InformationTransitionRegistryKey, LosslessTransformKey, PromotionProvenance,
        TransitionCapabilityClaim,
    };
    use crate::living_world_authority::transition_domain::{
        TransitionDomainAuthorityRegistryBuilder, TransitionDomainAuthorityRegistryKey,
        TransitionDomainAuthorityScope, TransitionDomainAuthorityStatus,
        TransitionDomainEvaluatorFingerprint, TransitionDomainPredicate,
        TransitionDomainPredicateProfileVersion, TransitionDomainQualification,
        TransitionDomainSnapshotId, TransitionDomainSourceRevision,
    };
    use crate::population::{
        CountDistribution, PopulationAgeBand, PopulationCell, PopulationConditionBand,
    };
    use crate::transition_applicability::{
        TransitionApplicability, TransitionApplicabilityPolicy,
        TransitionApplicabilityPolicyBuilder, TransitionApplicabilityPolicyKey,
        TransitionDomainKey,
    };
    use crate::transition_candidates::TransitionCandidateSearchLimits;
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(1_000, 1);
    const SOURCE: RepresentationKey = RepresentationKey::new(1_010, 1);
    const TARGET: RepresentationKey = RepresentationKey::new(1_011, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(1_020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(1_021, 1);
    const APP: TransitionApplicabilityPolicyKey = TransitionApplicabilityPolicyKey::new(1_030, 1);
    const DOMAIN: TransitionDomainKey = TransitionDomainKey::new(1_031, 1);
    const TRANSFORM: LosslessTransformKey = LosslessTransformKey(1_040, 1);
    const DOMAIN_REGISTRY: TransitionDomainAuthorityRegistryKey =
        TransitionDomainAuthorityRegistryKey::new(1_050, 1);
    const SCOPE: TransitionDomainAuthorityScope = TransitionDomainAuthorityScope::new(1_060, 1);

    fn source_capabilities() -> RepresentationCapabilities {
        RepresentationCapabilities::new(
            SOURCE,
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
            ],
        )
    }

    fn target_capabilities() -> RepresentationCapabilities {
        RepresentationCapabilities::new(
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
                    EcologicalInformation::SpatialStructure,
                    CapabilityEvidence::Exact,
                ),
            ],
        )
    }

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder.register_representation(source_capabilities()).unwrap();
        builder.register_representation(target_capabilities()).unwrap();
        builder.seal()
    }

    fn transitions(policy: &InformationPolicyRegistry) -> InformationTransitionRegistry {
        let claim = TransitionCapabilityClaim::new(
            EcologicalInformation::SpatialStructure,
            CapabilityEvidence::Exact,
        );
        let edge = InformationTransitionDefinition::new(
            EDGE,
            SOURCE,
            TARGET,
            [(
                claim,
                PromotionProvenance::LosslessDerivation {
                    transform: TRANSFORM,
                },
            )],
            [],
        )
        .unwrap();
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder.register_transition(edge).unwrap();
        builder.seal(policy).unwrap()
    }

    fn applicability(
        transitions: &InformationTransitionRegistry,
        mode: TransitionApplicability,
    ) -> TransitionApplicabilityPolicy {
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APP);
        builder.register(EDGE, mode).unwrap();
        builder.seal(transitions).unwrap()
    }

    fn distribution<K: Ord>(pairs: impl IntoIterator<Item = (K, u64)>) -> CountDistribution<K> {
        CountDistribution::new(pairs.into_iter().collect::<BTreeMap<_, _>>()).unwrap()
    }

    fn singleton_population() -> PopulationState {
        PopulationState::new(
            10,
            2_500_000,
            distribution([(PopulationAgeBand::Mature, 10)]),
            distribution([(PopulationConditionBand::Stable, 10)]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap()
    }

    fn multibin_population() -> PopulationState {
        PopulationState::new(
            10,
            2_500_000,
            distribution([
                (PopulationAgeBand::Juvenile, 5),
                (PopulationAgeBand::Mature, 5),
            ]),
            distribution([
                (PopulationConditionBand::Stable, 5),
                (PopulationConditionBand::Stressed, 5),
            ]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap()
    }

    fn exact_context<'a>(
        policy: &'a InformationPolicyRegistry,
        transitions: &'a InformationTransitionRegistry,
        applicability: &'a TransitionApplicabilityPolicy,
    ) -> (
        ManifestBoundInformationPolicyRegistry<'a>,
        ManifestBoundInformationTransitionRegistry<'a>,
        ManifestBoundTransitionApplicabilityPolicy<'a>,
        ApplicableTransitionCandidateSet,
        CandidateCertificationRequestSet,
    ) {
        let policy_exact = ManifestBoundInformationPolicyRegistry::new(policy);
        let transitions_exact =
            ManifestBoundInformationTransitionRegistry::new(transitions, &policy_exact).unwrap();
        let applicability_exact =
            ManifestBoundTransitionApplicabilityPolicy::new(applicability, &transitions_exact)
                .unwrap();
        let limits = TransitionCandidateSearchLimits::new(4, 8).unwrap();
        let candidates = transitions_exact
            .registry()
            .discover_candidates(transitions_exact.policy().registry(), SOURCE, TARGET, limits)
            .unwrap();
        let annotated = applicability_exact
            .policy()
            .annotate_candidate_set(&candidates)
            .unwrap();
        let requests =
            CandidateCertificationRequestSet::prepare(&applicability_exact, &annotated).unwrap();
        (
            policy_exact,
            transitions_exact,
            applicability_exact,
            annotated,
            requests,
        )
    }

    fn domain_registry(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        fingerprint: &[u8],
    ) -> TransitionDomainAuthorityRegistry {
        let qualification = TransitionDomainQualification::new(
            DOMAIN,
            SOURCE,
            TransitionDomainPredicate::UniqueAgeConditionFromSingletonMarginals,
            TransitionDomainPredicateProfileVersion(1),
            TransitionDomainEvaluatorFingerprint::new(fingerprint.to_vec()).unwrap(),
            crate::information::EvidenceLineageToken(1_070),
            None,
            TransitionDomainAuthorityStatus::Qualified,
        );
        let mut builder = TransitionDomainAuthorityRegistryBuilder::new(DOMAIN_REGISTRY);
        builder.register(qualification).unwrap();
        builder.seal(policy).unwrap()
    }

    fn subject(population: &PopulationState) -> TransitionDomainEvaluationSubject {
        TransitionDomainEvaluationSubject::from_population(
            SCOPE,
            SOURCE,
            TransitionDomainSnapshotId(1_080),
            TransitionDomainSourceRevision(9),
            population,
        )
        .unwrap()
    }

    #[test]
    fn resolves_explicit_domain_but_leaves_r1_obligation_untouched() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains = domain_registry(&policy_exact, b"domain-evaluator-a");
        let population = singleton_population();
        let subject = subject(&population);

        let resolved = DomainResolvedCandidateRequest::resolve(
            &requests,
            0,
            &domains,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert!(resolved.resolved_domains().contains_key(&DOMAIN));
        assert!(!resolved
            .remaining_obligations()
            .contains(&CandidateEvidenceObligation::TransitionDomain(DOMAIN)));
        assert!(resolved
            .remaining_obligations()
            .contains(&CandidateEvidenceObligation::LosslessTransform(TRANSFORM)));
        resolved
            .validate_current(
                &requests,
                &domains,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            )
            .unwrap();
    }

    #[test]
    fn multibin_population_fails_candidate_local_domain_resolution() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains = domain_registry(&policy_exact, b"domain-evaluator-a");
        let population = multibin_population();
        let subject = subject(&population);

        assert!(matches!(
            DomainResolvedCandidateRequest::resolve(
                &requests,
                0,
                &domains,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(CandidateDomainResolutionError::DomainAuthority(
                TransitionDomainAuthorityError::DomainNotSatisfied { domain: DOMAIN }
            ))
        ));
    }

    #[test]
    fn candidate_without_domain_obligation_resolves_zero_proofs_but_not_r1() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(&transitions, TransitionApplicability::Universal);
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains = domain_registry(&policy_exact, b"domain-evaluator-a");
        let population = singleton_population();
        let subject = subject(&population);

        let resolved = DomainResolvedCandidateRequest::resolve(
            &requests,
            0,
            &domains,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert!(resolved.resolved_domains().is_empty());
        assert!(resolved
            .remaining_obligations()
            .contains(&CandidateEvidenceObligation::LosslessTransform(TRANSFORM)));
    }

    #[test]
    fn changed_domain_authority_stales_old_resolution() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains_a = domain_registry(&policy_exact, b"domain-evaluator-a");
        let domains_b = domain_registry(&policy_exact, b"domain-evaluator-b");
        let population = singleton_population();
        let subject = subject(&population);
        let resolved = DomainResolvedCandidateRequest::resolve(
            &requests,
            0,
            &domains_a,
            &policy_exact,
            &applicability_exact,
            &subject,
            &population,
        )
        .unwrap();

        assert!(matches!(
            resolved.validate_current(
                &requests,
                &domains_b,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(CandidateDomainResolutionError::DomainAuthorityChanged)
        ));
    }

    #[test]
    fn same_revision_different_population_manifest_stales_resolution() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains = domain_registry(&policy_exact, b"domain-evaluator-a");
        let original = singleton_population();
        let original_subject = subject(&original);
        let resolved = DomainResolvedCandidateRequest::resolve(
            &requests,
            0,
            &domains,
            &policy_exact,
            &applicability_exact,
            &original_subject,
            &original,
        )
        .unwrap();

        let changed = PopulationState::new(
            10,
            2_500_001,
            distribution([(PopulationAgeBand::Mature, 10)]),
            distribution([(PopulationConditionBand::Stable, 10)]),
            distribution([(PopulationCell::new(0, 0, 0), 10)]),
        )
        .unwrap();
        let changed_subject = subject(&changed);

        assert!(matches!(
            resolved.validate_current(
                &requests,
                &domains,
                &policy_exact,
                &applicability_exact,
                &changed_subject,
                &changed,
            ),
            Err(CandidateDomainResolutionError::EvaluationSubjectChanged)
        ));
    }

    #[test]
    fn out_of_range_candidate_rejects_before_domain_resolution() {
        let policy = policy();
        let transitions = transitions(&policy);
        let applicability = applicability(
            &transitions,
            TransitionApplicability::RegisteredDomain(DOMAIN),
        );
        let (policy_exact, _transitions_exact, applicability_exact, _candidates, requests) =
            exact_context(&policy, &transitions, &applicability);
        let domains = domain_registry(&policy_exact, b"domain-evaluator-a");
        let population = singleton_population();
        let subject = subject(&population);

        assert!(matches!(
            DomainResolvedCandidateRequest::resolve(
                &requests,
                99,
                &domains,
                &policy_exact,
                &applicability_exact,
                &subject,
                &population,
            ),
            Err(CandidateDomainResolutionError::CandidateIndexOutOfRange { .. })
        ));
    }
}
