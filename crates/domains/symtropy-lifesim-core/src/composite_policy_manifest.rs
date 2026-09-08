// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Exact information-policy binding for composite capability contexts.
//!
//! [`crate::composite_information`] proves that several registry-resolved
//! authority stores describe one coherent scope/snapshot and can contribute
//! independent information without leaking subject authority. Its V0 context is
//! bound to the semantic registry key. This successor binds the same read-only
//! context and its sufficiency evidence to the exact canonical policy manifest.

use std::error::Error;
use std::fmt;

use crate::composite_information::{
    CapabilitySourceDescriptor, CompositeCapabilityContext, CompositeContextError,
    CompositeSufficiencyReport, CurrentCapabilitySourceRevisions,
};
use crate::information::ProcessKey;
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};

/// Composite capability context bound to the exact information-policy corpus
/// used to resolve all participating representation profiles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestBoundCompositeCapabilityContext {
    policy_authority: InformationPolicyAuthorityStamp,
    context: CompositeCapabilityContext,
}

impl ManifestBoundCompositeCapabilityContext {
    /// Capture one coherent composite source set through an exact-content-bound
    /// information-policy registry.
    pub fn capture(
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        sources: impl IntoIterator<Item = CapabilitySourceDescriptor>,
    ) -> Result<Self, ManifestBoundCompositeError> {
        let context = CompositeCapabilityContext::capture(policy.registry(), sources)
            .map_err(ManifestBoundCompositeError::Composite)?;
        Ok(Self {
            policy_authority: policy.authority_stamp().clone(),
            context,
        })
    }

    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }

    pub const fn context(&self) -> &CompositeCapabilityContext {
        &self.context
    }

    /// Evaluate registered process contracts only after proving that the supplied
    /// policy view still names the exact corpus captured by this context.
    pub fn evaluate_registered(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        processes: impl IntoIterator<Item = ProcessKey>,
    ) -> Result<ManifestBoundCompositeSufficiencyReport, ManifestBoundCompositeError> {
        self.validate_policy(policy)?;
        let report = self
            .context
            .evaluate_registered(policy.registry(), processes)
            .map_err(ManifestBoundCompositeError::Composite)?;
        Ok(ManifestBoundCompositeSufficiencyReport {
            policy_authority: self.policy_authority.clone(),
            report,
        })
    }

    /// Revalidate both the exact policy corpus and every captured source revision
    /// before a later authority boundary consumes a prepared composite context.
    ///
    /// Policy identity is checked first so same-key/different-corpus state cannot
    /// proceed merely because all source revisions happen to remain unchanged.
    pub fn validate_for_commit(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        current_revisions: &CurrentCapabilitySourceRevisions,
    ) -> Result<(), ManifestBoundCompositeError> {
        self.validate_policy(policy)?;
        self.context
            .validate_revisions(current_revisions)
            .map_err(ManifestBoundCompositeError::Composite)
    }

    pub fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), ManifestBoundCompositeError> {
        self.policy_authority
            .validate_registry(policy.registry())
            .map_err(ManifestBoundCompositeError::PolicyIdentity)
    }
}

/// Composite sufficiency evidence bound to exact policy content and the source
/// snapshot/revisions already carried by [`CompositeSufficiencyReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestBoundCompositeSufficiencyReport {
    policy_authority: InformationPolicyAuthorityStamp,
    report: CompositeSufficiencyReport,
}

impl ManifestBoundCompositeSufficiencyReport {
    pub const fn policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.policy_authority
    }

    pub const fn report(&self) -> &CompositeSufficiencyReport {
        &self.report
    }

    pub fn is_sufficient(&self) -> bool {
        self.report.is_sufficient()
    }

    /// Reject replay under a different exact policy corpus even when the
    /// semantic registry key/version was reused.
    pub fn validate_policy(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), ManifestBoundCompositeError> {
        self.policy_authority
            .validate_registry(policy.registry())
            .map_err(ManifestBoundCompositeError::PolicyIdentity)
    }
}

#[derive(Debug)]
pub enum ManifestBoundCompositeError {
    PolicyIdentity(InformationPolicyIdentityError),
    Composite(CompositeContextError),
}

impl fmt::Display for ManifestBoundCompositeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PolicyIdentity(error) => write!(formatter, "information-policy identity error: {error}"),
            Self::Composite(error) => write!(formatter, "composite capability error: {error}"),
        }
    }
}

impl Error for ManifestBoundCompositeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::Composite(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composite_information::{
        AuthorityScope, AuthoritySnapshotToken, CapabilitySourceKey, CapabilitySourceRevision,
    };
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        ProcessInformationProfile, ProcessInformationRequirement, RepresentationCapabilities,
        RepresentationKey,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };

    const REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(1200, 1);
    const OTHER_REGISTRY: InformationPolicyRegistryKey =
        InformationPolicyRegistryKey::new(1201, 1);
    const PROCESS: ProcessKey = ProcessKey::new(1202, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(1203, 1);
    const SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(1204, 1);
    const SCOPE: AuthorityScope = AuthorityScope(1205);
    const SNAPSHOT: AuthoritySnapshotToken = AuthoritySnapshotToken(1206);
    const REVISION: CapabilitySourceRevision = CapabilitySourceRevision(7);

    fn registry(
        key: InformationPolicyRegistryKey,
        requirement: EcologicalInformation,
    ) -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(key);
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(requirement)],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
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
            ))
            .unwrap();
        builder.seal()
    }

    fn descriptor() -> CapabilitySourceDescriptor {
        CapabilitySourceDescriptor::subject(
            SOURCE,
            REPRESENTATION,
            SCOPE,
            SNAPSHOT,
            REVISION,
        )
    }

    fn current_revision(revision: CapabilitySourceRevision) -> CurrentCapabilitySourceRevisions {
        CurrentCapabilitySourceRevisions::from_records([(SOURCE, revision)]).unwrap()
    }

    #[test]
    fn capture_and_report_bind_exact_policy_manifest() {
        let registry = registry(REGISTRY, EcologicalInformation::AgeDistribution);
        let policy = ManifestBoundInformationPolicyRegistry::new(&registry);
        let context = ManifestBoundCompositeCapabilityContext::capture(&policy, [descriptor()])
            .unwrap();
        let report = context.evaluate_registered(&policy, [PROCESS]).unwrap();

        assert_eq!(context.policy_authority(), policy.authority_stamp());
        assert_eq!(report.policy_authority(), policy.authority_stamp());
        assert!(report.is_sufficient());
        assert_eq!(report.report().scope(), SCOPE);
        assert_eq!(report.report().snapshot(), SNAPSHOT);
        assert_eq!(report.report().subject(), SOURCE);
    }

    #[test]
    fn same_semantic_key_with_changed_policy_corpus_rejects_before_commit() {
        let original = registry(REGISTRY, EcologicalInformation::AgeDistribution);
        let original_policy = ManifestBoundInformationPolicyRegistry::new(&original);
        let context =
            ManifestBoundCompositeCapabilityContext::capture(&original_policy, [descriptor()])
                .unwrap();

        let changed = registry(REGISTRY, EcologicalInformation::ConditionDistribution);
        let changed_policy = ManifestBoundInformationPolicyRegistry::new(&changed);
        let revisions = current_revision(REVISION);

        assert!(matches!(
            context.validate_for_commit(&changed_policy, &revisions),
            Err(ManifestBoundCompositeError::PolicyIdentity(
                InformationPolicyIdentityError::ManifestMismatch { key: REGISTRY }
            ))
        ));
    }

    #[test]
    fn different_semantic_registry_key_remains_a_distinct_failure() {
        let original = registry(REGISTRY, EcologicalInformation::AgeDistribution);
        let original_policy = ManifestBoundInformationPolicyRegistry::new(&original);
        let context =
            ManifestBoundCompositeCapabilityContext::capture(&original_policy, [descriptor()])
                .unwrap();

        let other = registry(OTHER_REGISTRY, EcologicalInformation::AgeDistribution);
        let other_policy = ManifestBoundInformationPolicyRegistry::new(&other);

        assert!(matches!(
            context.validate_policy(&other_policy),
            Err(ManifestBoundCompositeError::PolicyIdentity(
                InformationPolicyIdentityError::RegistryKeyMismatch {
                    expected: REGISTRY,
                    actual: OTHER_REGISTRY,
                }
            ))
        ));
    }

    #[test]
    fn source_revision_freshness_still_applies_after_policy_validation() {
        let registry = registry(REGISTRY, EcologicalInformation::AgeDistribution);
        let policy = ManifestBoundInformationPolicyRegistry::new(&registry);
        let context = ManifestBoundCompositeCapabilityContext::capture(&policy, [descriptor()])
            .unwrap();

        assert!(context
            .validate_for_commit(&policy, &current_revision(REVISION))
            .is_ok());
        assert!(matches!(
            context.validate_for_commit(
                &policy,
                &current_revision(CapabilitySourceRevision(8)),
            ),
            Err(ManifestBoundCompositeError::Composite(
                CompositeContextError::StaleSourceRevision {
                    source: SOURCE,
                    expected: REVISION,
                    actual: CapabilitySourceRevision(8),
                }
            ))
        ));
    }

    #[test]
    fn report_rejects_replay_under_same_key_changed_corpus() {
        let original = registry(REGISTRY, EcologicalInformation::AgeDistribution);
        let original_policy = ManifestBoundInformationPolicyRegistry::new(&original);
        let context =
            ManifestBoundCompositeCapabilityContext::capture(&original_policy, [descriptor()])
                .unwrap();
        let report = context
            .evaluate_registered(&original_policy, [PROCESS])
            .unwrap();

        let changed = registry(REGISTRY, EcologicalInformation::ConditionDistribution);
        let changed_policy = ManifestBoundInformationPolicyRegistry::new(&changed);

        assert!(matches!(
            report.validate_policy(&changed_policy),
            Err(ManifestBoundCompositeError::PolicyIdentity(
                InformationPolicyIdentityError::ManifestMismatch { key: REGISTRY }
            ))
        ));
    }
}
