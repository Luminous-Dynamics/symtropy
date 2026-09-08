// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit state-domain applicability for information transitions.
//!
//! `information_transition` proves that a source/destination path exists in the
//! registered structural graph. This module adds the independent question of
//! whether each selected edge is universally valid or requires a qualified
//! source-state domain proof. Domain satisfaction remains an external authority
//! requirement: this low-level layer never accepts a caller-authored boolean as
//! canonical proof that a state lies inside a domain.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::RepresentationKey;
use crate::information_registry::InformationPolicyRegistry;
use crate::information_transition::{
    InformationTransitionError, InformationTransitionKey, InformationTransitionPlan,
    InformationTransitionRegistry, InformationTransitionRegistryKey,
};

/// Semantic identity/version of one sealed applicability-policy corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionApplicabilityPolicyKey {
    id: u128,
    version: u32,
}

impl TransitionApplicabilityPolicyKey {
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

/// Semantic identity/version of one qualified source-state applicability domain.
///
/// The key is a descriptor, not proof. Higher orchestration must resolve it to a
/// qualified predicate/profile and bind the exact source revision evaluated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionDomainKey {
    id: u128,
    version: u32,
}

impl TransitionDomainKey {
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

/// Whether one registered transition is valid over the complete source schema or
/// only over a separately qualified subset of source states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransitionApplicability {
    /// The transition is explicitly qualified as valid for every canonical state
    /// admitted by its source representation/schema.
    Universal,
    /// The transition is valid only when the current canonical source state is
    /// proven to satisfy this registered domain.
    RegisteredDomain(TransitionDomainKey),
}

/// Bootstrap-only builder. Every transition edge must receive an explicit
/// applicability declaration; absence never silently means Universal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionApplicabilityPolicyBuilder {
    key: TransitionApplicabilityPolicyKey,
    entries: BTreeMap<InformationTransitionKey, TransitionApplicability>,
}

impl TransitionApplicabilityPolicyBuilder {
    pub const fn new(key: TransitionApplicabilityPolicyKey) -> Self {
        Self {
            key,
            entries: BTreeMap::new(),
        }
    }

    pub fn register(
        &mut self,
        transition: InformationTransitionKey,
        applicability: TransitionApplicability,
    ) -> Result<(), TransitionApplicabilityError> {
        use std::collections::btree_map::Entry;

        match self.entries.entry(transition) {
            Entry::Vacant(entry) => {
                entry.insert(applicability);
                Ok(())
            }
            Entry::Occupied(entry) if *entry.get() == applicability => Ok(()),
            Entry::Occupied(_) => Err(TransitionApplicabilityError::ConflictingRegistration {
                transition,
            }),
        }
    }

    /// Seal only when applicability is total over the exact transition graph.
    pub fn seal(
        self,
        transitions: &InformationTransitionRegistry,
    ) -> Result<TransitionApplicabilityPolicy, TransitionApplicabilityError> {
        let registered = transitions
            .transitions()
            .map(|(key, _)| *key)
            .collect::<BTreeSet<_>>();

        for transition in &registered {
            if !self.entries.contains_key(transition) {
                return Err(TransitionApplicabilityError::MissingApplicability {
                    transition: *transition,
                });
            }
        }

        for transition in self.entries.keys().copied() {
            if !registered.contains(&transition) {
                return Err(TransitionApplicabilityError::UnknownTransition { transition });
            }
        }

        Ok(TransitionApplicabilityPolicy {
            key: self.key,
            transition_registry_key: transitions.key(),
            entries: self.entries,
        })
    }
}

/// Immutable applicability policy bound to one transition-registry generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionApplicabilityPolicy {
    key: TransitionApplicabilityPolicyKey,
    transition_registry_key: InformationTransitionRegistryKey,
    entries: BTreeMap<InformationTransitionKey, TransitionApplicability>,
}

impl TransitionApplicabilityPolicy {
    pub const fn key(&self) -> TransitionApplicabilityPolicyKey {
        self.key
    }

    pub const fn transition_registry_key(&self) -> InformationTransitionRegistryKey {
        self.transition_registry_key
    }

    pub fn applicability(
        &self,
        transition: InformationTransitionKey,
    ) -> Option<TransitionApplicability> {
        self.entries.get(&transition).copied()
    }

    /// Produce a structural transition plan plus every unresolved state-domain
    /// proof needed by that exact path.
    ///
    /// This remains read-only. A `TransitionDomainKey` in the result is not proof
    /// that the current source state satisfies the domain.
    pub fn plan(
        &self,
        transitions: &InformationTransitionRegistry,
        information_policy: &InformationPolicyRegistry,
        source: RepresentationKey,
        destination: RepresentationKey,
    ) -> Result<ApplicableTransitionPlan, TransitionApplicabilityError> {
        if transitions.key() != self.transition_registry_key {
            return Err(TransitionApplicabilityError::TransitionRegistryKeyMismatch {
                expected: self.transition_registry_key,
                actual: transitions.key(),
            });
        }

        let structural = transitions
            .plan(information_policy, source, destination)
            .map_err(TransitionApplicabilityError::Transition)?;
        let mut required_domains = BTreeSet::new();

        for transition in structural.transitions() {
            let applicability = self.entries.get(transition).copied().ok_or(
                TransitionApplicabilityError::MissingApplicability {
                    transition: *transition,
                },
            )?;
            if let TransitionApplicability::RegisteredDomain(domain) = applicability {
                required_domains.insert(domain);
            }
        }

        Ok(ApplicableTransitionPlan {
            applicability_policy_key: self.key,
            transition_registry_key: self.transition_registry_key,
            structural,
            required_domains,
        })
    }
}

/// Structural reachability plus unresolved source-state applicability evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicableTransitionPlan {
    applicability_policy_key: TransitionApplicabilityPolicyKey,
    transition_registry_key: InformationTransitionRegistryKey,
    structural: InformationTransitionPlan,
    required_domains: BTreeSet<TransitionDomainKey>,
}

impl ApplicableTransitionPlan {
    pub const fn applicability_policy_key(&self) -> TransitionApplicabilityPolicyKey {
        self.applicability_policy_key
    }

    pub const fn transition_registry_key(&self) -> InformationTransitionRegistryKey {
        self.transition_registry_key
    }

    pub const fn structural_plan(&self) -> &InformationTransitionPlan {
        &self.structural
    }

    pub fn required_domains(&self) -> &BTreeSet<TransitionDomainKey> {
        &self.required_domains
    }

    /// True means the selected structural path contains no state-conditioned
    /// transition. It does not waive the structural plan's R0/R1/R2/R3/R4 or
    /// information-loss requirements.
    pub fn is_unconditionally_applicable(&self) -> bool {
        self.required_domains.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionApplicabilityError {
    Transition(InformationTransitionError),
    ConflictingRegistration {
        transition: InformationTransitionKey,
    },
    MissingApplicability {
        transition: InformationTransitionKey,
    },
    UnknownTransition {
        transition: InformationTransitionKey,
    },
    TransitionRegistryKeyMismatch {
        expected: InformationTransitionRegistryKey,
        actual: InformationTransitionRegistryKey,
    },
}

impl fmt::Display for TransitionApplicabilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transition(error) => write!(formatter, "information transition error: {error}"),
            Self::ConflictingRegistration { transition } => write!(
                formatter,
                "conflicting applicability registration for transition {transition:?}"
            ),
            Self::MissingApplicability { transition } => write!(
                formatter,
                "transition {transition:?} has no explicit applicability declaration"
            ),
            Self::UnknownTransition { transition } => write!(
                formatter,
                "applicability policy references unknown transition {transition:?}"
            ),
            Self::TransitionRegistryKeyMismatch { expected, actual } => write!(
                formatter,
                "applicability policy is bound to transition registry {expected:?}, got {actual:?}"
            ),
        }
    }
}

impl Error for TransitionApplicabilityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Transition(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        RepresentationCapabilities,
    };
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistryBuilder,
        InformationTransitionRegistryKey,
    };

    const INFORMATION_POLICY: InformationPolicyRegistryKey =
        InformationPolicyRegistryKey::new(1, 1);
    const TRANSITION_REGISTRY: InformationTransitionRegistryKey =
        InformationTransitionRegistryKey::new(2, 1);
    const APPLICABILITY_POLICY: TransitionApplicabilityPolicyKey =
        TransitionApplicabilityPolicyKey::new(3, 1);

    const A: RepresentationKey = RepresentationKey::new(10, 1);
    const B: RepresentationKey = RepresentationKey::new(11, 1);
    const C: RepresentationKey = RepresentationKey::new(12, 1);
    const AB: InformationTransitionKey = InformationTransitionKey::new(20, 1);
    const BC: InformationTransitionKey = InformationTransitionKey::new(21, 1);
    const UNKNOWN: InformationTransitionKey = InformationTransitionKey::new(22, 1);
    const DEGENERATE_DOMAIN: TransitionDomainKey = TransitionDomainKey::new(30, 1);

    fn information_policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(INFORMATION_POLICY);
        for representation in [A, B, C] {
            builder
                .register_representation(RepresentationCapabilities::new(
                    representation,
                    EcologicalAuthorityLevel::Coarse,
                    [(
                        EcologicalInformation::Headcount,
                        CapabilityEvidence::Exact,
                    )],
                ))
                .unwrap();
        }
        builder.seal()
    }

    fn transition_registry(
        information_policy: &InformationPolicyRegistry,
    ) -> InformationTransitionRegistry {
        let mut builder = InformationTransitionRegistryBuilder::new(TRANSITION_REGISTRY);
        for (key, source, destination) in [(AB, A, B), (BC, B, C)] {
            builder
                .register_transition(
                    InformationTransitionDefinition::new(
                        key,
                        source,
                        destination,
                        [],
                        [],
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        builder.seal(information_policy).unwrap()
    }

    #[test]
    fn every_transition_requires_an_explicit_applicability_declaration() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder.register(AB, TransitionApplicability::Universal).unwrap();

        assert!(matches!(
            builder.seal(&transitions),
            Err(TransitionApplicabilityError::MissingApplicability {
                transition: BC
            })
        ));
    }

    #[test]
    fn policy_cannot_smuggle_applicability_for_an_unknown_transition() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder.register(AB, TransitionApplicability::Universal).unwrap();
        builder.register(BC, TransitionApplicability::Universal).unwrap();
        builder
            .register(UNKNOWN, TransitionApplicability::Universal)
            .unwrap();

        assert!(matches!(
            builder.seal(&transitions),
            Err(TransitionApplicabilityError::UnknownTransition {
                transition: UNKNOWN
            })
        ));
    }

    #[test]
    fn registered_domain_is_an_unresolved_requirement_not_a_boolean_proof() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder
            .register(
                AB,
                TransitionApplicability::RegisteredDomain(DEGENERATE_DOMAIN),
            )
            .unwrap();
        builder.register(BC, TransitionApplicability::Universal).unwrap();
        let policy = builder.seal(&transitions).unwrap();
        let plan = policy.plan(&transitions, &information, A, C).unwrap();

        assert_eq!(
            plan.required_domains(),
            &BTreeSet::from([DEGENERATE_DOMAIN])
        );
        assert!(!plan.is_unconditionally_applicable());
        assert_eq!(plan.structural_plan().transitions(), &[AB, BC]);
    }

    #[test]
    fn all_universal_edges_produce_no_domain_requirement() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder.register(AB, TransitionApplicability::Universal).unwrap();
        builder.register(BC, TransitionApplicability::Universal).unwrap();
        let policy = builder.seal(&transitions).unwrap();
        let plan = policy.plan(&transitions, &information, A, C).unwrap();

        assert!(plan.required_domains().is_empty());
        assert!(plan.is_unconditionally_applicable());
    }

    #[test]
    fn applicability_registration_order_does_not_change_sealed_policy_or_plan() {
        let information = information_policy();
        let transitions = transition_registry(&information);

        let mut first = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        first
            .register(
                AB,
                TransitionApplicability::RegisteredDomain(DEGENERATE_DOMAIN),
            )
            .unwrap();
        first.register(BC, TransitionApplicability::Universal).unwrap();

        let mut second = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        second.register(BC, TransitionApplicability::Universal).unwrap();
        second
            .register(
                AB,
                TransitionApplicability::RegisteredDomain(DEGENERATE_DOMAIN),
            )
            .unwrap();

        let first = first.seal(&transitions).unwrap();
        let second = second.seal(&transitions).unwrap();
        assert_eq!(first, second);
        assert_eq!(
            first.plan(&transitions, &information, A, C).unwrap(),
            second.plan(&transitions, &information, A, C).unwrap()
        );
    }

    #[test]
    fn zero_edge_identity_path_requires_no_state_domain() {
        let information = information_policy();
        let transitions = transition_registry(&information);
        let mut builder = TransitionApplicabilityPolicyBuilder::new(APPLICABILITY_POLICY);
        builder.register(AB, TransitionApplicability::Universal).unwrap();
        builder.register(BC, TransitionApplicability::Universal).unwrap();
        let policy = builder.seal(&transitions).unwrap();
        let plan = policy.plan(&transitions, &information, A, A).unwrap();

        assert!(plan.structural_plan().transitions().is_empty());
        assert!(plan.required_domains().is_empty());
    }
}
