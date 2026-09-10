// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! R0 retained-exact common-start authority for Q2 shadow validation.
//!
//! This is intentionally only the first #408 tranche. It proves that a current,
//! exact retained representation was available for the same canonical T0 source
//! context used by an observable-bound Q2 comparison. It does **not** yet prove
//! dynamic continuity from that retained T0 state into the first post-start
//! reference observation, and therefore cannot by itself mint a closure validation
//! anchor.
//!
//! V0 is deliberately strict: the retained candidate must be uniquely identifiable,
//! R0-only, declare no information discards, terminate at the Q2 reference
//! representation, and contain one unambiguous retained exact record whose claims
//! cover the closure-backed information under validation.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
use crate::candidate_evidence_obligations::{
    CandidateCertificationRequest, CandidateCertificationRequestSet, CandidateEvidenceObligation,
    CandidatePathIdentity,
};
use crate::information::{CapabilityEvidence, EcologicalInformation, RepresentationKey};
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::information_transition::RetainedAuthorityKey;
use crate::population::PopulationState;

use super::closure_usage_authority::ClosureUsagePolicyRegistry;
use super::retained_authority::{
    ResolvedRetainedAuthority, RetainedAuthorityError, RetainedAuthorityRegistry,
    RetainedResolvedCandidateRequest,
};
use super::shadow_observable_authority::{
    ObservableBoundShadowValidationCertificate, ShadowObservableAuthorityError,
    ShadowObservableAuthorityRegistry,
};
use super::shadow_validation::ShadowValidationRegistry;
use super::spatiotemporal_information::SpatiotemporalPolicyRegistry;
use super::transition_domain::{
    TransitionDomainAuthorityError, TransitionDomainEvaluationSubject,
};
use super::typed_closure_process_acceptance::TypedClosureProcessAcceptanceRegistry;
use super::typed_closure_qualification::TypedClosureQualificationRegistry;

/// R0-only proof that an exact richer retained state was available at the Q2
/// common-start context.
///
/// This is deliberately weaker than the future full `ShadowCommonStartCertificate`:
/// it authenticates the retained T0 source but does not claim that the reference
/// solver's later state is causally continuous from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedShadowStartCertificate {
    observable_bound_shadow: ObservableBoundShadowValidationCertificate,
    source_subject: TransitionDomainEvaluationSubject,
    candidate_index: usize,
    candidate: CandidatePathIdentity,
    retained_resolution: RetainedResolvedCandidateRequest,
    retained_start: ResolvedRetainedAuthority,
    information: EcologicalInformation,
    coarse_representation: RepresentationKey,
    reference_representation: RepresentationKey,
}

impl RetainedShadowStartCertificate {
    pub const fn observable_bound_shadow(&self) -> &ObservableBoundShadowValidationCertificate {
        &self.observable_bound_shadow
    }

    pub const fn source_subject(&self) -> &TransitionDomainEvaluationSubject {
        &self.source_subject
    }

    pub const fn candidate_index(&self) -> usize {
        self.candidate_index
    }

    pub const fn candidate(&self) -> &CandidatePathIdentity {
        &self.candidate
    }

    pub const fn retained_resolution(&self) -> &RetainedResolvedCandidateRequest {
        &self.retained_resolution
    }

    pub const fn retained_start(&self) -> &ResolvedRetainedAuthority {
        &self.retained_start
    }

    pub const fn information(&self) -> EcologicalInformation {
        self.information
    }

    pub const fn coarse_representation(&self) -> RepresentationKey {
        self.coarse_representation
    }

    pub const fn reference_representation(&self) -> RepresentationKey {
        self.reference_representation
    }

    /// Produce one R0 retained-start certificate from an already-derived #406
    /// observable-bound Q2 certificate.
    ///
    /// `current_coarse_population` is the Q2 endpoint used to revalidate #406;
    /// `start_population` is separately bound to the exact T0 population manifest.
    #[allow(clippy::too_many_arguments)]
    pub fn certify(
        requests: &CandidateCertificationRequestSet,
        retained_resolution: &RetainedResolvedCandidateRequest,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_bound_shadow: &ObservableBoundShadowValidationCertificate,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        source_subject: &TransitionDomainEvaluationSubject,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
    ) -> Result<Self, RetainedShadowStartError> {
        observable_bound_shadow
            .validate_current(
                observable_registry,
                shadow_registry,
                usage,
                policy,
                closures,
                acceptances,
                spatiotemporal,
                current_coarse_population,
            )
            .map_err(RetainedShadowStartError::Observable)?;

        retained_resolution
            .validate_current(
                requests,
                retained,
                policy,
                applicability,
                source_subject,
                start_population,
            )
            .map_err(RetainedShadowStartError::Retained)?;

        source_subject
            .validate_population(start_population)
            .map_err(RetainedShadowStartError::SourceState)?;

        let shadow = observable_bound_shadow.shadow();
        if source_subject.population_manifest() != shadow.evidence().start_population_manifest() {
            return Err(RetainedShadowStartError::Q2StartPopulationMismatch);
        }

        let coarse_representation = shadow.resolved_use().representation();
        if source_subject.source_representation() != coarse_representation {
            return Err(RetainedShadowStartError::CoarseRepresentationMismatch {
                expected: coarse_representation,
                actual: source_subject.source_representation(),
            });
        }

        let reference_representation = shadow.profile().reference_representation();
        let information = shadow.resolved_use().information();

        if !retained_resolution.remaining_obligations().is_empty() {
            return Err(RetainedShadowStartError::NonR0ObligationsRemain {
                obligations: retained_resolution.remaining_obligations().clone(),
            });
        }
        if !retained_resolution.discarded_information().is_empty() {
            return Err(RetainedShadowStartError::CandidateDiscardsInformation {
                discarded: retained_resolution.discarded_information().clone(),
            });
        }

        let candidate_index = uniquely_match_resolved_candidate(
            requests,
            retained_resolution,
            coarse_representation,
            reference_representation,
        )?;
        let request = requests
            .requests()
            .get(candidate_index)
            .ok_or(RetainedShadowStartError::CandidateIndexOutOfRange {
                index: candidate_index,
                len: requests.requests().len(),
            })?;

        let retained_start = unique_reference_retained_start(
            retained_resolution,
            reference_representation,
            information,
            source_subject,
        )?;

        Ok(Self {
            observable_bound_shadow: observable_bound_shadow.clone(),
            source_subject: source_subject.clone(),
            candidate_index,
            candidate: request.identity().clone(),
            retained_resolution: retained_resolution.clone(),
            retained_start,
            information,
            coarse_representation,
            reference_representation,
        })
    }

    /// Recompute the complete retained-start proof against current exact inputs.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
    ) -> Result<(), RetainedShadowStartError> {
        let current = Self::certify(
            requests,
            &self.retained_resolution,
            retained,
            applicability,
            &self.observable_bound_shadow,
            observable_registry,
            shadow_registry,
            usage,
            policy,
            closures,
            acceptances,
            spatiotemporal,
            &self.source_subject,
            start_population,
            current_coarse_population,
        )?;
        if current != *self {
            return Err(RetainedShadowStartError::CertificateStale);
        }
        Ok(())
    }
}

fn uniquely_match_resolved_candidate(
    requests: &CandidateCertificationRequestSet,
    resolved: &RetainedResolvedCandidateRequest,
    source: RepresentationKey,
    destination: RepresentationKey,
) -> Result<usize, RetainedShadowStartError> {
    let mut matches = Vec::new();
    for (index, request) in requests.requests().iter().enumerate() {
        if request.identity().source() != source || request.identity().destination() != destination {
            continue;
        }
        if candidate_resolution_shape_matches(request, resolved) {
            matches.push(index);
        }
    }
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(RetainedShadowStartError::ResolvedCandidateNotFound),
        _ => Err(RetainedShadowStartError::ResolvedCandidateAmbiguous { matches }),
    }
}

fn candidate_resolution_shape_matches(
    request: &CandidateCertificationRequest,
    resolved: &RetainedResolvedCandidateRequest,
) -> bool {
    if request.discarded_information() != resolved.discarded_information() {
        return false;
    }

    let request_r0 = request
        .obligations()
        .iter()
        .filter_map(|obligation| match obligation {
            CandidateEvidenceObligation::RetainedAuthority(authority) => Some(*authority),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let resolved_r0 = resolved.resolved().keys().copied().collect::<BTreeSet<_>>();
    if request_r0 != resolved_r0 {
        return false;
    }

    let request_remaining = request
        .obligations()
        .iter()
        .filter(|obligation| !matches!(obligation, CandidateEvidenceObligation::RetainedAuthority(_)))
        .copied()
        .collect::<BTreeSet<_>>();
    request_remaining == *resolved.remaining_obligations()
}

fn unique_reference_retained_start(
    resolved: &RetainedResolvedCandidateRequest,
    reference_representation: RepresentationKey,
    information: EcologicalInformation,
    source_subject: &TransitionDomainEvaluationSubject,
) -> Result<ResolvedRetainedAuthority, RetainedShadowStartError> {
    let mut matches = resolved
        .resolved()
        .values()
        .filter(|authority| {
            let record = authority.record();
            record.store_representation() == reference_representation
                && record.scope() == source_subject.scope()
                && record.snapshot() == source_subject.snapshot()
                && authority.required_claims().iter().any(|claim| {
                    claim.information().covers(information)
                        && claim.evidence() == CapabilityEvidence::Exact
                })
        });

    let first = matches
        .next()
        .cloned()
        .ok_or(RetainedShadowStartError::NoReferenceRetainedStart {
            representation: reference_representation,
            information,
        })?;
    if matches.next().is_some() {
        return Err(RetainedShadowStartError::AmbiguousReferenceRetainedStart {
            representation: reference_representation,
            information,
        });
    }
    Ok(first)
}

#[derive(Debug)]
pub enum RetainedShadowStartError {
    Observable(ShadowObservableAuthorityError),
    Retained(RetainedAuthorityError),
    SourceState(TransitionDomainAuthorityError),
    Q2StartPopulationMismatch,
    CoarseRepresentationMismatch {
        expected: RepresentationKey,
        actual: RepresentationKey,
    },
    CandidateIndexOutOfRange {
        index: usize,
        len: usize,
    },
    NonR0ObligationsRemain {
        obligations: BTreeSet<CandidateEvidenceObligation>,
    },
    CandidateDiscardsInformation {
        discarded: BTreeSet<EcologicalInformation>,
    },
    ResolvedCandidateNotFound,
    ResolvedCandidateAmbiguous {
        matches: Vec<usize>,
    },
    NoReferenceRetainedStart {
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    AmbiguousReferenceRetainedStart {
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    CertificateStale,
}

impl fmt::Display for RetainedShadowStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observable(error) => write!(f, "observable-bound Q2 authority: {error}"),
            Self::Retained(error) => write!(f, "retained R0 authority: {error}"),
            Self::SourceState(error) => write!(f, "T0 source-state authority: {error}"),
            Self::Q2StartPopulationMismatch => write!(
                f,
                "T0 source population manifest differs from the exact Q2 start population manifest"
            ),
            Self::CoarseRepresentationMismatch { expected, actual } => write!(
                f,
                "T0 source representation {actual:?} differs from Q2 coarse representation {expected:?}"
            ),
            Self::CandidateIndexOutOfRange { index, len } => write!(
                f,
                "resolved retained candidate index {index} exceeds request set length {len}"
            ),
            Self::NonR0ObligationsRemain { obligations } => write!(
                f,
                "retained-start V0 rejects {} unresolved non-R0 obligation(s)",
                obligations.len()
            ),
            Self::CandidateDiscardsInformation { discarded } => write!(
                f,
                "retained-start V0 rejects candidate with {} declared information discard(s)",
                discarded.len()
            ),
            Self::ResolvedCandidateNotFound => write!(
                f,
                "no canonical candidate uniquely matches the current retained resolution"
            ),
            Self::ResolvedCandidateAmbiguous { matches } => write!(
                f,
                "{} canonical candidates match the current retained resolution; V0 refuses ambiguity",
                matches.len()
            ),
            Self::NoReferenceRetainedStart { representation, information } => write!(
                f,
                "no retained Exact record in reference representation {representation:?} covers {information:?} at T0"
            ),
            Self::AmbiguousReferenceRetainedStart { representation, information } => write!(
                f,
                "multiple retained Exact records in reference representation {representation:?} cover {information:?}; V0 requires one unambiguous start"
            ),
            Self::CertificateStale => write!(f, "retained shadow-start certificate is stale"),
        }
    }
}

impl Error for RetainedShadowStartError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Observable(error) => Some(error),
            Self::Retained(error) => Some(error),
            Self::SourceState(error) => Some(error),
            _ => None,
        }
    }
}

/// Exact semantic R0 authority keys consumed by this retained-start proof.
pub fn retained_start_authorities(
    certificate: &RetainedShadowStartCertificate,
) -> BTreeSet<RetainedAuthorityKey> {
    certificate
        .retained_resolution()
        .resolved()
        .keys()
        .copied()
        .collect()
}
