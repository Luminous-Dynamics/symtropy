// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Currentness composition for anchor-eligible Q2 shadow execution evidence.
//!
//! The lower authority layers deliberately prove different propositions:
//!
//! - #563/#567 authenticate the exact coarse/reference execution pair and T0 ancestry;
//! - #612/#699 prove sampling-invariant one-tick execution continuity;
//! - #701/#702 prove reviewed, verifier-bound executable runner semantics.
//!
//! A consequential caller must not be required to remember three independent
//! revalidation sequences. This module composes those existing theorems into one
//! opaque certificate that can only be minted while every authority is current.
//!
//! This type is still evidence-only. It does not construct `ShadowAnchorClaim`,
//! cannot construct `ClosureValidationAnchor`, and does not mutate canonical
//! ecology. The #392 bridge remains a separate capability boundary.

use std::error::Error;
use std::fmt;

use crate::applicability_policy_manifest::ManifestBoundTransitionApplicabilityPolicy;
use crate::candidate_evidence_obligations::CandidateCertificationRequestSet;
use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
use crate::population::PopulationState;

use super::closure_usage_authority::ClosureUsagePolicyRegistry;
use super::retained_authority::RetainedAuthorityRegistry;
use super::shadow_execution_continuity::{
    PairedShadowContinuityCertificate, ShadowExecutionContinuityError,
};
use super::shadow_execution_lineage::ShadowReferenceExecutionRegistry;
use super::shadow_observable_authority::ShadowObservableAuthorityRegistry;
use super::shadow_paired_execution::{
    PairedShadowExecutionError, PairedShadowExecutionRegistry,
};
use super::shadow_runner_qualification::ShadowRunnerQualificationRegistry;
use super::shadow_runner_semantic_provenance::{
    ShadowRunnerSemanticProvenanceError, ShadowRunnerSemanticRegistry,
    VerifierBoundExecutableShadowRunnerPairAuthority,
};
use super::shadow_validation::ShadowValidationRegistry;
use super::spatiotemporal_information::SpatiotemporalPolicyRegistry;
use super::typed_closure_process_acceptance::TypedClosureProcessAcceptanceRegistry;
use super::typed_closure_qualification::TypedClosureQualificationRegistry;

/// One current conjunction of structural execution, exact continuity and
/// verifier-bound executable runner semantics.
///
/// The object may later become stale. Consequential callers must invoke
/// `validate_current` immediately before use, exactly as with the lower authority
/// certificates it contains.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrentExecutableShadowContinuityCertificate {
    continuity: PairedShadowContinuityCertificate,
    executable: VerifierBoundExecutableShadowRunnerPairAuthority,
}

impl CurrentExecutableShadowContinuityCertificate {
    /// Mint the composite only after all lower authorities revalidate against the
    /// same exact current coarse/reference pair.
    #[allow(clippy::too_many_arguments)]
    pub fn certify_current(
        continuity: &PairedShadowContinuityCertificate,
        executable: &VerifierBoundExecutableShadowRunnerPairAuthority,
        paired_registry: &PairedShadowExecutionRegistry,
        reference_registry: &ShadowReferenceExecutionRegistry,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        semantic_registry: &ShadowRunnerSemanticRegistry,
    ) -> Result<Self, CurrentExecutableShadowContinuityError> {
        let paired = continuity.paired_execution();

        // First prove that the exact #567 pair embedded by continuity is still
        // current against every structural authority it depends on.
        paired
            .validate_current(
                paired_registry,
                reference_registry,
                requests,
                retained,
                applicability,
                observable_registry,
                shadow_registry,
                usage,
                information_policy,
                closures,
                acceptances,
                spatiotemporal,
                start_population,
                current_coarse_population,
            )
            .map_err(CurrentExecutableShadowContinuityError::PairedExecution)?;

        // Then re-certify continuity against that now-current exact pair. This
        // rejects a transcript copied from another run, evidence revision or
        // validation window even if its local chain is internally well formed.
        continuity
            .validate_against_pair(paired)
            .map_err(CurrentExecutableShadowContinuityError::Continuity)?;

        // Finally bind executable qualification to the runner records embedded in
        // that exact current pair. This prevents a valid qualification for some
        // other runner/profile from being attached to otherwise-valid continuity.
        executable
            .validate_current(
                semantic_registry,
                qualification_registry,
                paired.reference().runner(),
                paired.coarse_runner(),
            )
            .map_err(CurrentExecutableShadowContinuityError::RunnerSemantics)?;

        Ok(Self {
            continuity: continuity.clone(),
            executable: executable.clone(),
        })
    }

    pub const fn continuity(&self) -> &PairedShadowContinuityCertificate {
        &self.continuity
    }

    pub const fn executable(&self) -> &VerifierBoundExecutableShadowRunnerPairAuthority {
        &self.executable
    }

    /// Re-run the complete currentness conjunction immediately before a
    /// consequential consumer uses this evidence.
    #[allow(clippy::too_many_arguments)]
    pub fn validate_current(
        &self,
        paired_registry: &PairedShadowExecutionRegistry,
        reference_registry: &ShadowReferenceExecutionRegistry,
        requests: &CandidateCertificationRequestSet,
        retained: &RetainedAuthorityRegistry,
        applicability: &ManifestBoundTransitionApplicabilityPolicy<'_>,
        observable_registry: &ShadowObservableAuthorityRegistry,
        shadow_registry: &ShadowValidationRegistry,
        usage: &ClosureUsagePolicyRegistry,
        information_policy: &ManifestBoundInformationPolicyRegistry<'_>,
        closures: &TypedClosureQualificationRegistry,
        acceptances: &TypedClosureProcessAcceptanceRegistry,
        spatiotemporal: &SpatiotemporalPolicyRegistry,
        start_population: &PopulationState,
        current_coarse_population: &PopulationState,
        qualification_registry: &ShadowRunnerQualificationRegistry,
        semantic_registry: &ShadowRunnerSemanticRegistry,
    ) -> Result<(), CurrentExecutableShadowContinuityError> {
        let current = Self::certify_current(
            &self.continuity,
            &self.executable,
            paired_registry,
            reference_registry,
            requests,
            retained,
            applicability,
            observable_registry,
            shadow_registry,
            usage,
            information_policy,
            closures,
            acceptances,
            spatiotemporal,
            start_population,
            current_coarse_population,
            qualification_registry,
            semantic_registry,
        )?;

        if current != *self {
            return Err(CurrentExecutableShadowContinuityError::CertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum CurrentExecutableShadowContinuityError {
    PairedExecution(PairedShadowExecutionError),
    Continuity(ShadowExecutionContinuityError),
    RunnerSemantics(ShadowRunnerSemanticProvenanceError),
    CertificateStale,
}

impl fmt::Display for CurrentExecutableShadowContinuityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PairedExecution(error) => {
                write!(f, "current paired shadow execution failed: {error}")
            }
            Self::Continuity(error) => {
                write!(f, "current shadow execution continuity failed: {error}")
            }
            Self::RunnerSemantics(error) => {
                write!(f, "current executable shadow runner semantics failed: {error}")
            }
            Self::CertificateStale => {
                write!(f, "current executable shadow continuity certificate is stale")
            }
        }
    }
}

impl Error for CurrentExecutableShadowContinuityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PairedExecution(error) => Some(error),
            Self::Continuity(error) => Some(error),
            Self::RunnerSemantics(error) => Some(error),
            Self::CertificateStale => None,
        }
    }
}
