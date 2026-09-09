// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Registry-owned conservation authority for ecological fidelity transitions.
//!
//! Information-valid promotion is not enough for canonical commit. A pure
//! representation transition must also preserve every tracked ecological
//! conserved quantity within its registered numerical contract, without hiding
//! representation drift behind external input/output counters.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::conservation::{ConservationError, ConservedQuantity, EcologicalLedger};
use crate::information_transition::InformationTransitionKey;
use crate::transition_policy_manifest::{
    InformationTransitionAuthorityStamp, ManifestBoundInformationTransitionRegistry,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TransitionConservationRegistryKey {
    id: u128,
    version: u32,
}

impl TransitionConservationRegistryKey {
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

/// Canonical identity for a finite, non-negative absolute residual tolerance.
///
/// The validated IEEE-754 bit pattern is stored rather than `f64` directly so
/// conservation policy remains `Eq`/ordered and can participate in authority
/// identity without float comparison semantics. Negative zero is normalized to
/// positive zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AbsoluteConservationTolerance(u64);

impl AbsoluteConservationTolerance {
    pub fn new(value: f64) -> Result<Self, TransitionConservationError> {
        if !value.is_finite() || value < 0.0 {
            return Err(TransitionConservationError::InvalidTolerance(value));
        }
        let normalized = if value == 0.0 { 0.0 } else { value };
        Ok(Self(normalized.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

/// Conservation contract for one exact information-transition edge.
///
/// V0 deliberately requires an explicit tolerance for every quantity tracked by
/// [`ConservedQuantity::ALL`]. A transition cannot become conservation-authorized
/// merely because a caller omitted the quantity that drifted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionConservationRequirement {
    transition: InformationTransitionKey,
    tolerances: BTreeMap<ConservedQuantity, AbsoluteConservationTolerance>,
}

impl TransitionConservationRequirement {
    pub fn new(
        transition: InformationTransitionKey,
        tolerances: impl IntoIterator<Item = (ConservedQuantity, AbsoluteConservationTolerance)>,
    ) -> Result<Self, TransitionConservationError> {
        let mut canonical = BTreeMap::new();
        for (quantity, tolerance) in tolerances {
            if canonical.insert(quantity, tolerance).is_some() {
                return Err(TransitionConservationError::DuplicateQuantityRequirement {
                    transition,
                    quantity,
                });
            }
        }
        for quantity in ConservedQuantity::ALL {
            if !canonical.contains_key(&quantity) {
                return Err(TransitionConservationError::MissingQuantityRequirement {
                    transition,
                    quantity,
                });
            }
        }
        Ok(Self {
            transition,
            tolerances: canonical,
        })
    }

    pub fn preserve_all(
        transition: InformationTransitionKey,
        tolerance: AbsoluteConservationTolerance,
    ) -> Self {
        Self {
            transition,
            tolerances: ConservedQuantity::ALL
                .into_iter()
                .map(|quantity| (quantity, tolerance))
                .collect(),
        }
    }

    pub const fn transition(&self) -> InformationTransitionKey {
        self.transition
    }

    pub fn tolerance(
        &self,
        quantity: ConservedQuantity,
    ) -> AbsoluteConservationTolerance {
        self.tolerances[&quantity]
    }

    pub fn tolerances(
        &self,
    ) -> &BTreeMap<ConservedQuantity, AbsoluteConservationTolerance> {
        &self.tolerances
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionConservationAuthorityStamp {
    key: TransitionConservationRegistryKey,
    transition_authority: InformationTransitionAuthorityStamp,
    requirements: BTreeMap<InformationTransitionKey, TransitionConservationRequirement>,
}

impl TransitionConservationAuthorityStamp {
    pub const fn key(&self) -> TransitionConservationRegistryKey {
        self.key
    }

    pub const fn transition_authority(&self) -> &InformationTransitionAuthorityStamp {
        &self.transition_authority
    }

    pub fn requirements(
        &self,
    ) -> &BTreeMap<InformationTransitionKey, TransitionConservationRequirement> {
        &self.requirements
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionConservationRegistryBuilder {
    key: TransitionConservationRegistryKey,
    requirements: BTreeMap<InformationTransitionKey, TransitionConservationRequirement>,
}

impl TransitionConservationRegistryBuilder {
    pub const fn new(key: TransitionConservationRegistryKey) -> Self {
        Self {
            key,
            requirements: BTreeMap::new(),
        }
    }

    pub fn register_requirement(
        &mut self,
        requirement: TransitionConservationRequirement,
    ) -> Result<(), TransitionConservationError> {
        let transition = requirement.transition();
        if let Some(existing) = self.requirements.get(&transition) {
            if existing == &requirement {
                return Ok(());
            }
            return Err(TransitionConservationError::ConflictingRequirementRegistration {
                transition,
            });
        }
        self.requirements.insert(transition, requirement);
        Ok(())
    }

    /// Seal only when every edge in the exact transition graph has one complete
    /// conservation contract and no contract refers to an unknown edge.
    pub fn seal(
        self,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
    ) -> Result<TransitionConservationRegistry, TransitionConservationError> {
        let known = transitions
            .registry()
            .transitions()
            .map(|(key, _)| *key)
            .collect::<BTreeSet<_>>();

        for transition in self.requirements.keys().copied() {
            if !known.contains(&transition) {
                return Err(TransitionConservationError::UnknownTransition { transition });
            }
        }
        for transition in known {
            if !self.requirements.contains_key(&transition) {
                return Err(TransitionConservationError::MissingTransitionRequirement {
                    transition,
                });
            }
        }

        let authority = TransitionConservationAuthorityStamp {
            key: self.key,
            transition_authority: transitions.authority_stamp().clone(),
            requirements: self.requirements.clone(),
        };
        Ok(TransitionConservationRegistry {
            authority,
            requirements: self.requirements,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransitionConservationRegistry {
    authority: TransitionConservationAuthorityStamp,
    requirements: BTreeMap<InformationTransitionKey, TransitionConservationRequirement>,
}

impl TransitionConservationRegistry {
    pub const fn authority_stamp(&self) -> &TransitionConservationAuthorityStamp {
        &self.authority
    }

    pub fn requirement(
        &self,
        transition: InformationTransitionKey,
    ) -> Option<&TransitionConservationRequirement> {
        self.requirements.get(&transition)
    }

    pub fn validate_current(
        &self,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
    ) -> Result<(), TransitionConservationError> {
        if transitions.authority_stamp() != &self.authority.transition_authority {
            return Err(TransitionConservationError::TransitionAuthorityChanged);
        }
        Ok(())
    }

    /// Evaluate one representation transition without mutating either ledger.
    pub fn evaluate_transition(
        &self,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
        transition: InformationTransitionKey,
        source: &EcologicalLedger,
        destination: &EcologicalLedger,
    ) -> Result<TransitionConservationReport, TransitionConservationError> {
        self.validate_current(transitions)?;
        let requirement = self
            .requirements
            .get(&transition)
            .ok_or(TransitionConservationError::UnknownTransition { transition })?;

        let mut balances = BTreeMap::new();
        for quantity in ConservedQuantity::ALL {
            let source_total = source
                .total(quantity)
                .map_err(TransitionConservationError::Conservation)?;
            let destination_total = destination
                .total(quantity)
                .map_err(TransitionConservationError::Conservation)?;
            let residual = destination_total - source_total;
            if !residual.is_finite() {
                return Err(TransitionConservationError::NonFiniteResidual { quantity });
            }

            let external_input_delta =
                destination.external_input(quantity) - source.external_input(quantity);
            let external_output_delta =
                destination.external_output(quantity) - source.external_output(quantity);
            if !external_input_delta.is_finite() || !external_output_delta.is_finite() {
                return Err(TransitionConservationError::NonFiniteBoundaryDelta { quantity });
            }

            let tolerance = requirement.tolerance(quantity);
            balances.insert(
                quantity,
                TransitionConservationBalance {
                    source_total,
                    destination_total,
                    residual,
                    external_input_delta,
                    external_output_delta,
                    tolerance,
                },
            );
        }

        Ok(TransitionConservationReport {
            transition,
            balances,
        })
    }

    /// Mint an authority-derived certificate only when the exact transition
    /// graph remains current and every registered conservation check passes.
    pub fn certify_transition(
        &self,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
        transition: InformationTransitionKey,
        source: &EcologicalLedger,
        destination: &EcologicalLedger,
    ) -> Result<ConservativeTransitionCertificate, TransitionConservationError> {
        let report = self.evaluate_transition(transitions, transition, source, destination)?;
        if !report.is_satisfied() {
            return Err(TransitionConservationError::ConservationViolation(report));
        }
        Ok(ConservativeTransitionCertificate {
            authority: self.authority.clone(),
            transition,
            report,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransitionConservationBalance {
    pub source_total: f64,
    pub destination_total: f64,
    pub residual: f64,
    pub external_input_delta: f64,
    pub external_output_delta: f64,
    pub tolerance: AbsoluteConservationTolerance,
}

impl TransitionConservationBalance {
    pub fn is_satisfied(self) -> bool {
        self.external_input_delta == 0.0
            && self.external_output_delta == 0.0
            && self.residual.abs() <= self.tolerance.get()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionConservationReport {
    transition: InformationTransitionKey,
    balances: BTreeMap<ConservedQuantity, TransitionConservationBalance>,
}

impl TransitionConservationReport {
    pub const fn transition(&self) -> InformationTransitionKey {
        self.transition
    }

    pub fn balance(&self, quantity: ConservedQuantity) -> TransitionConservationBalance {
        self.balances[&quantity]
    }

    pub fn balances(
        &self,
    ) -> &BTreeMap<ConservedQuantity, TransitionConservationBalance> {
        &self.balances
    }

    pub fn is_satisfied(&self) -> bool {
        self.balances
            .values()
            .copied()
            .all(TransitionConservationBalance::is_satisfied)
    }
}

/// Conservation proof for one exact transition edge.
///
/// There is intentionally no public constructor. A certificate can only be
/// produced by a sealed registry after evaluating concrete source/destination
/// conservation ledgers.
#[derive(Debug, Clone, PartialEq)]
pub struct ConservativeTransitionCertificate {
    authority: TransitionConservationAuthorityStamp,
    transition: InformationTransitionKey,
    report: TransitionConservationReport,
}

impl ConservativeTransitionCertificate {
    pub const fn authority_stamp(&self) -> &TransitionConservationAuthorityStamp {
        &self.authority
    }

    pub const fn transition(&self) -> InformationTransitionKey {
        self.transition
    }

    pub const fn report(&self) -> &TransitionConservationReport {
        &self.report
    }

    pub fn validate_current(
        &self,
        registry: &TransitionConservationRegistry,
        transitions: &ManifestBoundInformationTransitionRegistry<'_>,
        source: &EcologicalLedger,
        destination: &EcologicalLedger,
    ) -> Result<(), TransitionConservationError> {
        if registry.authority_stamp() != &self.authority {
            return Err(TransitionConservationError::ConservationAuthorityChanged);
        }
        let current = registry.certify_transition(
            transitions,
            self.transition,
            source,
            destination,
        )?;
        if current != *self {
            return Err(TransitionConservationError::CertificateStale);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransitionConservationError {
    InvalidTolerance(f64),
    DuplicateQuantityRequirement {
        transition: InformationTransitionKey,
        quantity: ConservedQuantity,
    },
    MissingQuantityRequirement {
        transition: InformationTransitionKey,
        quantity: ConservedQuantity,
    },
    ConflictingRequirementRegistration {
        transition: InformationTransitionKey,
    },
    UnknownTransition {
        transition: InformationTransitionKey,
    },
    MissingTransitionRequirement {
        transition: InformationTransitionKey,
    },
    TransitionAuthorityChanged,
    ConservationAuthorityChanged,
    CertificateStale,
    NonFiniteResidual {
        quantity: ConservedQuantity,
    },
    NonFiniteBoundaryDelta {
        quantity: ConservedQuantity,
    },
    Conservation(ConservationError),
    ConservationViolation(TransitionConservationReport),
}

impl fmt::Display for TransitionConservationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTolerance(value) => write!(
                formatter,
                "conservation tolerance must be finite and non-negative, got {value}"
            ),
            Self::DuplicateQuantityRequirement {
                transition,
                quantity,
            } => write!(
                formatter,
                "transition {}@{} has duplicate conservation requirement for {quantity:?}",
                transition.id(),
                transition.version()
            ),
            Self::MissingQuantityRequirement {
                transition,
                quantity,
            } => write!(
                formatter,
                "transition {}@{} is missing conservation requirement for {quantity:?}",
                transition.id(),
                transition.version()
            ),
            Self::ConflictingRequirementRegistration { transition } => write!(
                formatter,
                "transition {}@{} has conflicting conservation registrations",
                transition.id(),
                transition.version()
            ),
            Self::UnknownTransition { transition } => write!(
                formatter,
                "transition {}@{} is not known to the conservation/transition authority",
                transition.id(),
                transition.version()
            ),
            Self::MissingTransitionRequirement { transition } => write!(
                formatter,
                "transition {}@{} has no conservation requirement",
                transition.id(),
                transition.version()
            ),
            Self::TransitionAuthorityChanged => {
                write!(formatter, "exact information-transition authority changed")
            }
            Self::ConservationAuthorityChanged => {
                write!(formatter, "transition-conservation authority changed")
            }
            Self::CertificateStale => write!(
                formatter,
                "conservative transition certificate no longer matches current ledgers"
            ),
            Self::NonFiniteResidual { quantity } => write!(
                formatter,
                "transition conservation residual became non-finite for {quantity:?}"
            ),
            Self::NonFiniteBoundaryDelta { quantity } => write!(
                formatter,
                "transition external-boundary delta became non-finite for {quantity:?}"
            ),
            Self::Conservation(error) => write!(formatter, "ecological conservation error: {error}"),
            Self::ConservationViolation(report) => write!(
                formatter,
                "transition {}@{} violated its conservation contract",
                report.transition().id(),
                report.transition().version()
            ),
        }
    }
}

impl Error for TransitionConservationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Conservation(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conservation::{EcologicalCompartment, PoolKey};
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
        ProcessInformationProfile, ProcessInformationRequirement, RepresentationCapabilities,
        RepresentationKey,
    };
    use crate::information_policy_manifest::ManifestBoundInformationPolicyRegistry;
    use crate::information_registry::{
        InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
    };
    use crate::information_transition::{
        InformationTransitionDefinition, InformationTransitionRegistry,
        InformationTransitionRegistryBuilder, InformationTransitionRegistryKey,
    };
    use crate::transition_policy_manifest::ManifestBoundInformationTransitionRegistry;

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(8_000, 1);
    const GRAPH: InformationTransitionRegistryKey = InformationTransitionRegistryKey::new(8_001, 1);
    const CONSERVATION: TransitionConservationRegistryKey =
        TransitionConservationRegistryKey::new(8_002, 1);
    const A: RepresentationKey = RepresentationKey::new(8_010, 1);
    const B: RepresentationKey = RepresentationKey::new(8_011, 1);
    const PROCESS: crate::information::ProcessKey = crate::information::ProcessKey::new(8_020, 1);
    const EDGE: InformationTransitionKey = InformationTransitionKey::new(8_030, 1);
    const EDGE_ALT: InformationTransitionKey = InformationTransitionKey::new(8_031, 1);

    fn policy() -> InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::Headcount,
                )],
            ))
            .unwrap();

        let claims = || {
            let mut claims = vec![(
                EcologicalInformation::Headcount,
                CapabilityEvidence::Exact,
            )];
            claims.extend(ConservedQuantity::ALL.into_iter().map(|quantity| {
                (
                    EcologicalInformation::ExactConservationAccount(quantity),
                    CapabilityEvidence::Exact,
                )
            }));
            claims
        };

        for representation in [A, B] {
            builder
                .register_representation(RepresentationCapabilities::new(
                    representation,
                    EcologicalAuthorityLevel::Coarse,
                    claims(),
                ))
                .unwrap();
        }
        builder.seal()
    }

    fn graph(
        policy: &InformationPolicyRegistry,
        edge: InformationTransitionKey,
    ) -> InformationTransitionRegistry {
        let mut builder = InformationTransitionRegistryBuilder::new(GRAPH);
        builder
            .register_transition(
                InformationTransitionDefinition::new(edge, A, B, [], []).unwrap(),
            )
            .unwrap();
        builder.seal(policy).unwrap()
    }

    fn ledger(base: f64, water_delta: f64) -> EcologicalLedger {
        let mut ledger = EcologicalLedger::default();
        for quantity in ConservedQuantity::ALL {
            let amount = if quantity == ConservedQuantity::WaterMass {
                base + water_delta
            } else {
                base
            };
            ledger
                .seed(PoolKey::new(quantity, EcologicalCompartment::Storage), amount)
                .unwrap();
        }
        ledger
    }

    fn exact_requirement(edge: InformationTransitionKey) -> TransitionConservationRequirement {
        TransitionConservationRequirement::preserve_all(
            edge,
            AbsoluteConservationTolerance::new(0.0).unwrap(),
        )
    }

    #[test]
    fn identical_conserved_state_certifies_transition() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder.register_requirement(exact_requirement(EDGE)).unwrap();
        let registry = builder.seal(&exact_graph).unwrap();
        let source = ledger(10.0, 0.0);
        let destination = source.clone();

        let certificate = registry
            .certify_transition(&exact_graph, EDGE, &source, &destination)
            .unwrap();
        assert!(certificate.report().is_satisfied());
        certificate
            .validate_current(&registry, &exact_graph, &source, &destination)
            .unwrap();
    }

    #[test]
    fn drift_beyond_registered_tolerance_rejects() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder.register_requirement(exact_requirement(EDGE)).unwrap();
        let registry = builder.seal(&exact_graph).unwrap();
        let source = ledger(10.0, 0.0);
        let destination = ledger(10.0, 0.01);

        assert!(matches!(
            registry.certify_transition(&exact_graph, EDGE, &source, &destination),
            Err(TransitionConservationError::ConservationViolation(_))
        ));
    }

    #[test]
    fn explicitly_tolerated_small_drift_can_certify() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let tolerance = AbsoluteConservationTolerance::new(0.1).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder
            .register_requirement(TransitionConservationRequirement::preserve_all(
                EDGE, tolerance,
            ))
            .unwrap();
        let registry = builder.seal(&exact_graph).unwrap();
        let source = ledger(10.0, 0.0);
        let destination = ledger(10.0, 0.05);

        assert!(registry
            .certify_transition(&exact_graph, EDGE, &source, &destination)
            .is_ok());
    }

    #[test]
    fn external_boundary_counter_change_rejects_even_when_total_matches() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder.register_requirement(exact_requirement(EDGE)).unwrap();
        let registry = builder.seal(&exact_graph).unwrap();
        let source = ledger(10.0, 0.0);
        let mut destination = source.clone();
        destination
            .input(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Storage,
                1.0,
            )
            .unwrap();
        destination
            .output(
                ConservedQuantity::WaterMass,
                EcologicalCompartment::Storage,
                1.0,
            )
            .unwrap();

        let report = registry
            .evaluate_transition(&exact_graph, EDGE, &source, &destination)
            .unwrap();
        assert_eq!(
            report.balance(ConservedQuantity::WaterMass).residual,
            0.0
        );
        assert!(!report.is_satisfied());
    }

    #[test]
    fn registry_cannot_omit_transition_contract() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let builder = TransitionConservationRegistryBuilder::new(CONSERVATION);

        assert!(matches!(
            builder.seal(&exact_graph),
            Err(TransitionConservationError::MissingTransitionRequirement { transition })
                if transition == EDGE
        ));
    }

    #[test]
    fn changed_exact_transition_graph_stales_registry() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder.register_requirement(exact_requirement(EDGE)).unwrap();
        let registry = builder.seal(&exact_graph).unwrap();

        let changed_graph = graph(&policy, EDGE_ALT);
        let changed_exact =
            ManifestBoundInformationTransitionRegistry::new(&changed_graph, &exact_policy).unwrap();
        assert!(matches!(
            registry.validate_current(&changed_exact),
            Err(TransitionConservationError::TransitionAuthorityChanged)
        ));
    }

    #[test]
    fn evaluation_is_read_only() {
        let policy = policy();
        let graph = graph(&policy, EDGE);
        let exact_policy = ManifestBoundInformationPolicyRegistry::new(&policy);
        let exact_graph =
            ManifestBoundInformationTransitionRegistry::new(&graph, &exact_policy).unwrap();
        let mut builder = TransitionConservationRegistryBuilder::new(CONSERVATION);
        builder.register_requirement(exact_requirement(EDGE)).unwrap();
        let registry = builder.seal(&exact_graph).unwrap();
        let source = ledger(10.0, 0.0);
        let destination = source.clone();
        let source_before = source.clone();
        let destination_before = destination.clone();

        registry
            .evaluate_transition(&exact_graph, EDGE, &source, &destination)
            .unwrap();
        assert_eq!(source, source_before);
        assert_eq!(destination, destination_before);
    }

    #[test]
    fn requirement_order_does_not_change_semantics() {
        let tolerance = AbsoluteConservationTolerance::new(0.0).unwrap();
        let forward = TransitionConservationRequirement::new(
            EDGE,
            ConservedQuantity::ALL
                .into_iter()
                .map(|quantity| (quantity, tolerance)),
        )
        .unwrap();
        let reverse = TransitionConservationRequirement::new(
            EDGE,
            ConservedQuantity::ALL
                .into_iter()
                .rev()
                .map(|quantity| (quantity, tolerance)),
        )
        .unwrap();
        assert_eq!(forward, reverse);
    }
}
