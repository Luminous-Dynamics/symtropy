// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Snapshot-consistent composition of ecological information authority.
//!
//! Canonical ecological processes often read facts from several stores at once.
//! This module lets those stores satisfy independent process requirements only
//! when they resolve through one sealed information-policy registry and describe
//! one coherent authority scope/snapshot. It never infers cross-store covariance
//! merely because the participating marginals are individually exact.
//!
//! A cross-store relationship such as exact age × disease covariance must itself
//! be represented by a registry-owned authority source. Callers cannot attach an
//! ad hoc exact relation to a context during capture.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{
    evaluate_processes, CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    ProcessKey, RepresentationCapabilities, RepresentationKey, SufficiencyReport,
};
use crate::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryKey, InformationRegistryError,
};

const COMPOSITE_CONTEXT_REPRESENTATION: RepresentationKey =
    RepresentationKey::new(0x636f6d705f6361705f6374785f7631, 1);

/// Non-reused identity for one coherent canonical read boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthoritySnapshotToken(pub u128);

/// Canonical scope whose facts may be composed, such as one population/region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AuthorityScope(pub u128);

/// Stable identity/version for one authority store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilitySourceKey {
    id: u128,
    version: u32,
}

impl CapabilitySourceKey {
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

/// Source-local revision captured at the coherent snapshot boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilitySourceRevision(pub u64);

/// Descriptor supplied to snapshot capture. The representation profile itself
/// is resolved from the sealed registry rather than caller-supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CapabilitySourceDescriptor {
    key: CapabilitySourceKey,
    representation: RepresentationKey,
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    revision: CapabilitySourceRevision,
}

impl CapabilitySourceDescriptor {
    pub const fn new(
        key: CapabilitySourceKey,
        representation: RepresentationKey,
        scope: AuthorityScope,
        snapshot: AuthoritySnapshotToken,
        revision: CapabilitySourceRevision,
    ) -> Self {
        Self {
            key,
            representation,
            scope,
            snapshot,
            revision,
        }
    }

    pub const fn key(self) -> CapabilitySourceKey {
        self.key
    }

    pub const fn representation(self) -> RepresentationKey {
        self.representation
    }

    pub const fn scope(self) -> AuthorityScope {
        self.scope
    }

    pub const fn snapshot(self) -> AuthoritySnapshotToken {
        self.snapshot
    }

    pub const fn revision(self) -> CapabilitySourceRevision {
        self.revision
    }
}

/// One registry-resolved authority store captured into the composite context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCapabilitySource {
    descriptor: CapabilitySourceDescriptor,
    capabilities: RepresentationCapabilities,
}

impl ResolvedCapabilitySource {
    pub const fn descriptor(&self) -> CapabilitySourceDescriptor {
        self.descriptor
    }

    pub const fn key(&self) -> CapabilitySourceKey {
        self.descriptor.key
    }

    pub const fn revision(&self) -> CapabilitySourceRevision {
        self.descriptor.revision
    }

    pub fn capabilities(&self) -> &RepresentationCapabilities {
        &self.capabilities
    }
}

/// Deterministic, read-only capability context captured from mutually coherent
/// canonical authority stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeCapabilityContext {
    registry_key: InformationPolicyRegistryKey,
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    sources: BTreeMap<CapabilitySourceKey, ResolvedCapabilitySource>,
    combined: RepresentationCapabilities,
}

impl CompositeCapabilityContext {
    /// Capture and validate a coherent source set.
    ///
    /// This operation is read-only. It resolves representation claims from the
    /// sealed registry, canonicalizes source ordering, rejects mismatched scope
    /// or snapshot identity, and rejects duplicate exact ownership claims.
    ///
    /// Cross-store relationships are never caller-injected. If exact age ×
    /// disease covariance exists, for example, it must arrive as another
    /// registry-owned source whose representation explicitly advertises that
    /// joint capability.
    pub fn capture(
        registry: &InformationPolicyRegistry,
        sources: impl IntoIterator<Item = CapabilitySourceDescriptor>,
    ) -> Result<Self, CompositeContextError> {
        let mut resolved_sources = BTreeMap::new();
        let mut scope = None;
        let mut snapshot = None;
        let mut combined_claims = Vec::new();
        let mut max_authority = EcologicalAuthorityLevel::Presentation;
        let mut exclusive_owners = BTreeMap::new();

        for descriptor in sources {
            if let Some(expected) = scope {
                if descriptor.scope != expected {
                    return Err(CompositeContextError::ScopeMismatch {
                        expected,
                        actual: descriptor.scope,
                        source: descriptor.key,
                    });
                }
            } else {
                scope = Some(descriptor.scope);
            }

            if let Some(expected) = snapshot {
                if descriptor.snapshot != expected {
                    return Err(CompositeContextError::SnapshotMismatch {
                        expected,
                        actual: descriptor.snapshot,
                        source: descriptor.key,
                    });
                }
            } else {
                snapshot = Some(descriptor.snapshot);
            }

            let capabilities = registry
                .resolve_representation(descriptor.representation)
                .map_err(CompositeContextError::Registry)?
                .capabilities()
                .clone();
            max_authority = max_authority.max(capabilities.authority_level());

            for (information, evidence_set) in capabilities.claims() {
                if is_exclusive_authority_information(*information)
                    && evidence_set.contains(&CapabilityEvidence::Exact)
                {
                    match exclusive_owners.entry(*information) {
                        Entry::Vacant(entry) => {
                            entry.insert(descriptor.key);
                        }
                        Entry::Occupied(entry) => {
                            return Err(CompositeContextError::DuplicateExclusiveAuthority {
                                information: *information,
                                first: *entry.get(),
                                second: descriptor.key,
                            });
                        }
                    }
                }
                for evidence in evidence_set.iter().copied() {
                    combined_claims.push((*information, evidence));
                }
            }

            match resolved_sources.entry(descriptor.key) {
                Entry::Vacant(entry) => {
                    entry.insert(ResolvedCapabilitySource {
                        descriptor,
                        capabilities,
                    });
                }
                Entry::Occupied(_) => {
                    return Err(CompositeContextError::DuplicateSource {
                        source: descriptor.key,
                    });
                }
            }
        }

        let scope = scope.ok_or(CompositeContextError::EmptySourceSet)?;
        let snapshot = snapshot.expect("scope and snapshot are initialized together");
        let combined = RepresentationCapabilities::new(
            COMPOSITE_CONTEXT_REPRESENTATION,
            max_authority,
            combined_claims,
        );

        Ok(Self {
            registry_key: registry.key(),
            scope,
            snapshot,
            sources: resolved_sources,
            combined,
        })
    }

    pub const fn registry_key(&self) -> InformationPolicyRegistryKey {
        self.registry_key
    }

    pub const fn scope(&self) -> AuthorityScope {
        self.scope
    }

    pub const fn snapshot(&self) -> AuthoritySnapshotToken {
        self.snapshot
    }

    pub fn sources(&self) -> &BTreeMap<CapabilitySourceKey, ResolvedCapabilitySource> {
        &self.sources
    }

    /// Derived evaluation view only; this is not a new mutable authority owner.
    pub fn combined_capabilities(&self) -> &RepresentationCapabilities {
        &self.combined
    }

    /// Evaluate registered process contracts against the coherent composite.
    pub fn evaluate_registered(
        &self,
        registry: &InformationPolicyRegistry,
        processes: impl IntoIterator<Item = ProcessKey>,
    ) -> Result<CompositeSufficiencyReport, CompositeContextError> {
        if registry.key() != self.registry_key {
            return Err(CompositeContextError::RegistryKeyMismatch {
                expected: self.registry_key,
                actual: registry.key(),
            });
        }

        let process_keys = processes.into_iter().collect::<BTreeSet<_>>();
        let mut profiles = Vec::with_capacity(process_keys.len());
        for process in &process_keys {
            profiles.push(
                registry
                    .resolve_process(*process)
                    .map_err(CompositeContextError::Registry)?
                    .profile(),
            );
        }

        Ok(CompositeSufficiencyReport {
            registry_key: self.registry_key,
            scope: self.scope,
            snapshot: self.snapshot,
            source_revisions: self
                .sources
                .iter()
                .map(|(key, source)| (*key, source.revision()))
                .collect(),
            process_keys,
            report: evaluate_processes(profiles, &self.combined),
        })
    }

    /// Fail closed if any source used by this context has disappeared or moved
    /// to a different source-local revision before a prepared commit.
    pub fn validate_revisions(
        &self,
        current: impl IntoIterator<Item = (CapabilitySourceKey, CapabilitySourceRevision)>,
    ) -> Result<(), CompositeContextError> {
        let current = current.into_iter().collect::<BTreeMap<_, _>>();
        for (key, source) in &self.sources {
            let actual = current
                .get(key)
                .copied()
                .ok_or(CompositeContextError::MissingCurrentSource { source: *key })?;
            if actual != source.revision() {
                return Err(CompositeContextError::StaleSourceRevision {
                    source: *key,
                    expected: source.revision(),
                    actual,
                });
            }
        }
        Ok(())
    }
}

/// Sufficiency evidence tied to the exact coherent source revisions that were
/// evaluated. Future #291 work will strengthen the registry binding from the
/// semantic key to an exact content-addressed policy stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositeSufficiencyReport {
    registry_key: InformationPolicyRegistryKey,
    scope: AuthorityScope,
    snapshot: AuthoritySnapshotToken,
    source_revisions: BTreeMap<CapabilitySourceKey, CapabilitySourceRevision>,
    process_keys: BTreeSet<ProcessKey>,
    report: SufficiencyReport,
}

impl CompositeSufficiencyReport {
    pub const fn registry_key(&self) -> InformationPolicyRegistryKey {
        self.registry_key
    }

    pub const fn scope(&self) -> AuthorityScope {
        self.scope
    }

    pub const fn snapshot(&self) -> AuthoritySnapshotToken {
        self.snapshot
    }

    pub fn source_revisions(&self) -> &BTreeMap<CapabilitySourceKey, CapabilitySourceRevision> {
        &self.source_revisions
    }

    pub fn process_keys(&self) -> &BTreeSet<ProcessKey> {
        &self.process_keys
    }

    pub const fn report(&self) -> &SufficiencyReport {
        &self.report
    }

    pub fn is_sufficient(&self) -> bool {
        self.report.is_sufficient()
    }
}

fn is_exclusive_authority_information(information: EcologicalInformation) -> bool {
    matches!(
        information,
        EcologicalInformation::ExactConservationAccount(_)
            | EcologicalInformation::IndividualActiveState
            | EcologicalInformation::PersistentIdentity
    )
}

#[derive(Debug)]
pub enum CompositeContextError {
    Registry(InformationRegistryError),
    EmptySourceSet,
    DuplicateSource {
        source: CapabilitySourceKey,
    },
    ScopeMismatch {
        expected: AuthorityScope,
        actual: AuthorityScope,
        source: CapabilitySourceKey,
    },
    SnapshotMismatch {
        expected: AuthoritySnapshotToken,
        actual: AuthoritySnapshotToken,
        source: CapabilitySourceKey,
    },
    DuplicateExclusiveAuthority {
        information: EcologicalInformation,
        first: CapabilitySourceKey,
        second: CapabilitySourceKey,
    },
    RegistryKeyMismatch {
        expected: InformationPolicyRegistryKey,
        actual: InformationPolicyRegistryKey,
    },
    MissingCurrentSource {
        source: CapabilitySourceKey,
    },
    StaleSourceRevision {
        source: CapabilitySourceKey,
        expected: CapabilitySourceRevision,
        actual: CapabilitySourceRevision,
    },
}

impl fmt::Display for CompositeContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => write!(formatter, "information registry error: {error}"),
            Self::EmptySourceSet => write!(formatter, "composite capability context has no sources"),
            Self::DuplicateSource { source } => {
                write!(formatter, "duplicate capability source {source:?}")
            }
            Self::ScopeMismatch {
                expected,
                actual,
                source,
            } => write!(
                formatter,
                "capability source {source:?} has scope {actual:?}, expected {expected:?}"
            ),
            Self::SnapshotMismatch {
                expected,
                actual,
                source,
            } => write!(
                formatter,
                "capability source {source:?} has snapshot {actual:?}, expected {expected:?}"
            ),
            Self::DuplicateExclusiveAuthority {
                information,
                first,
                second,
            } => write!(
                formatter,
                "exclusive authority {information:?} is claimed by both {first:?} and {second:?}"
            ),
            Self::RegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "registry key {actual:?} does not match captured registry {expected:?}"
            ),
            Self::MissingCurrentSource { source } => {
                write!(formatter, "current authority state is missing source {source:?}")
            }
            Self::StaleSourceRevision {
                source,
                expected,
                actual,
            } => write!(
                formatter,
                "source {source:?} moved from revision {expected:?} to {actual:?}"
            ),
        }
    }
}

impl Error for CompositeContextError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Registry(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conservation::ConservedQuantity;
    use crate::information::{
        EcologicalInformation, PopulationStatisticSet, ProcessInformationProfile,
        ProcessInformationRequirement,
    };
    use crate::information_registry::{
        InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };

    const REGISTRY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(900, 1);
    const SCOPE: AuthorityScope = AuthorityScope(901);
    const SNAPSHOT: AuthoritySnapshotToken = AuthoritySnapshotToken(902);

    const POP_SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(10, 1);
    const LEDGER_SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(11, 1);
    const DISEASE_SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(12, 1);
    const RELATION_SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(13, 1);
    const SECOND_LEDGER_SOURCE: CapabilitySourceKey = CapabilitySourceKey::new(14, 1);

    const POP_REP: RepresentationKey = RepresentationKey::new(100, 1);
    const LEDGER_REP: RepresentationKey = RepresentationKey::new(101, 1);
    const DISEASE_REP: RepresentationKey = RepresentationKey::new(102, 1);
    const RELATION_REP: RepresentationKey = RepresentationKey::new(103, 1);
    const SECOND_LEDGER_REP: RepresentationKey = RepresentationKey::new(104, 1);

    const MIXED_PROCESS: ProcessKey = ProcessKey::new(200, 1);
    const JOINT_PROCESS: ProcessKey = ProcessKey::new(201, 1);

    fn age_disease() -> EcologicalInformation {
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE.union(PopulationStatisticSet::DISEASE),
        )
    }

    fn source(
        key: CapabilitySourceKey,
        representation: RepresentationKey,
        revision: u64,
    ) -> CapabilitySourceDescriptor {
        CapabilitySourceDescriptor::new(
            key,
            representation,
            SCOPE,
            SNAPSHOT,
            CapabilitySourceRevision(revision),
        )
    }

    fn registry() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
        builder
            .register_representation(RepresentationCapabilities::new(
                POP_REP,
                EcologicalAuthorityLevel::Coarse,
                [
                    (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
                    (
                        EcologicalInformation::AgeDistribution,
                        CapabilityEvidence::Exact,
                    ),
                ],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                LEDGER_REP,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::ExactConservationAccount(
                        ConservedQuantity::CarbonMass,
                    ),
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                DISEASE_REP,
                EcologicalAuthorityLevel::Coarse,
                [(EcologicalInformation::DiseaseState, CapabilityEvidence::Exact)],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                RELATION_REP,
                EcologicalAuthorityLevel::Coarse,
                [(age_disease(), CapabilityEvidence::Exact)],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                SECOND_LEDGER_REP,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::ExactConservationAccount(
                        ConservedQuantity::CarbonMass,
                    ),
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();

        builder
            .register_process(ProcessInformationProfile::new(
                MIXED_PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [
                    ProcessInformationRequirement::exact(EcologicalInformation::Headcount),
                    ProcessInformationRequirement::exact(
                        EcologicalInformation::ExactConservationAccount(
                            ConservedQuantity::CarbonMass,
                        ),
                    ),
                ],
            ))
            .unwrap();
        builder
            .register_process(ProcessInformationProfile::new(
                JOINT_PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(age_disease())],
            ))
            .unwrap();
        builder.seal()
    }

    #[test]
    fn independent_requirements_can_be_satisfied_by_two_coherent_sources() {
        let registry = registry();
        let context = CompositeCapabilityContext::capture(
            &registry,
            [source(POP_SOURCE, POP_REP, 4), source(LEDGER_SOURCE, LEDGER_REP, 9)],
        )
        .unwrap();

        let report = context
            .evaluate_registered(&registry, [MIXED_PROCESS])
            .unwrap();
        assert!(report.is_sufficient(), "failures={:?}", report.report().failures());
    }

    #[test]
    fn mismatched_snapshot_fails_before_capability_union() {
        let registry = registry();
        let other_snapshot = CapabilitySourceDescriptor::new(
            LEDGER_SOURCE,
            LEDGER_REP,
            SCOPE,
            AuthoritySnapshotToken(SNAPSHOT.0 + 1),
            CapabilitySourceRevision(9),
        );

        assert!(matches!(
            CompositeCapabilityContext::capture(
                &registry,
                [source(POP_SOURCE, POP_REP, 4), other_snapshot],
            ),
            Err(CompositeContextError::SnapshotMismatch { .. })
        ));
    }

    #[test]
    fn mismatched_scope_fails_before_capability_union() {
        let registry = registry();
        let other_scope = CapabilitySourceDescriptor::new(
            LEDGER_SOURCE,
            LEDGER_REP,
            AuthorityScope(SCOPE.0 + 1),
            SNAPSHOT,
            CapabilitySourceRevision(9),
        );

        assert!(matches!(
            CompositeCapabilityContext::capture(
                &registry,
                [source(POP_SOURCE, POP_REP, 4), other_scope],
            ),
            Err(CompositeContextError::ScopeMismatch { .. })
        ));
    }

    #[test]
    fn separate_exact_marginals_do_not_create_cross_store_covariance() {
        let registry = registry();
        let context = CompositeCapabilityContext::capture(
            &registry,
            [
                source(POP_SOURCE, POP_REP, 4),
                source(DISEASE_SOURCE, DISEASE_REP, 5),
            ],
        )
        .unwrap();

        assert!(
            !context
                .evaluate_registered(&registry, [JOINT_PROCESS])
                .unwrap()
                .is_sufficient()
        );
    }

    #[test]
    fn registered_relation_source_can_satisfy_joint_requirement() {
        let registry = registry();
        let context = CompositeCapabilityContext::capture(
            &registry,
            [
                source(DISEASE_SOURCE, DISEASE_REP, 5),
                source(RELATION_SOURCE, RELATION_REP, 7),
                source(POP_SOURCE, POP_REP, 4),
            ],
        )
        .unwrap();

        assert!(
            context
                .evaluate_registered(&registry, [JOINT_PROCESS])
                .unwrap()
                .is_sufficient()
        );
    }

    #[test]
    fn duplicate_exact_exclusive_ownership_fails_closed() {
        let registry = registry();
        assert!(matches!(
            CompositeCapabilityContext::capture(
                &registry,
                [
                    source(LEDGER_SOURCE, LEDGER_REP, 9),
                    source(SECOND_LEDGER_SOURCE, SECOND_LEDGER_REP, 2),
                ],
            ),
            Err(CompositeContextError::DuplicateExclusiveAuthority { .. })
        ));
    }

    #[test]
    fn source_order_does_not_change_context_identity_or_evaluation() {
        let registry = registry();
        let a = CompositeCapabilityContext::capture(
            &registry,
            [source(POP_SOURCE, POP_REP, 4), source(LEDGER_SOURCE, LEDGER_REP, 9)],
        )
        .unwrap();
        let b = CompositeCapabilityContext::capture(
            &registry,
            [source(LEDGER_SOURCE, LEDGER_REP, 9), source(POP_SOURCE, POP_REP, 4)],
        )
        .unwrap();

        assert_eq!(a, b);
        assert_eq!(
            a.evaluate_registered(&registry, [MIXED_PROCESS]).unwrap(),
            b.evaluate_registered(&registry, [MIXED_PROCESS]).unwrap()
        );
    }

    #[test]
    fn one_stale_source_invalidates_the_whole_prepared_context() {
        let registry = registry();
        let context = CompositeCapabilityContext::capture(
            &registry,
            [source(POP_SOURCE, POP_REP, 4), source(LEDGER_SOURCE, LEDGER_REP, 9)],
        )
        .unwrap();

        assert!(matches!(
            context.validate_revisions([
                (POP_SOURCE, CapabilitySourceRevision(4)),
                (LEDGER_SOURCE, CapabilitySourceRevision(10)),
            ]),
            Err(CompositeContextError::StaleSourceRevision {
                source: LEDGER_SOURCE,
                ..
            })
        ));
    }
}
