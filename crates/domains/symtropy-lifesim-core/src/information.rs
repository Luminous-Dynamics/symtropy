// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Machine-readable ecological process information requirements.
//!
//! An authoritative process may execute only when the representation that will
//! carry it has enough qualified information for that process. Fidelity is
//! therefore a relationship between a process and a representation, not a
//! renderer-distance label.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::conservation::ConservedQuantity;

/// Fixed-point scale used for closure error policy: one million parts = 100%.
pub const PPM_SCALE: u32 = 1_000_000;

/// Bounded integer error fraction used by closure policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ErrorPpm(u32);

impl ErrorPpm {
    pub const ZERO: Self = Self(0);
    pub const ONE_PERCENT: Self = Self(10_000);

    pub const fn new(value: u32) -> Result<Self, InformationPolicyError> {
        if value > PPM_SCALE {
            return Err(InformationPolicyError::ErrorPpmOutOfRange { value });
        }
        Ok(Self(value))
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Opaque version of one qualified closure model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureModelVersion(pub u32);

/// Opaque applicability-domain identity for a closure qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureDomainToken(pub u128);

/// Opaque evidence-lineage identity for one closure qualification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EvidenceLineageToken(pub u128);

/// Population statistics that can participate in a retained joint state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PopulationStatisticSet(u16);

impl PopulationStatisticSet {
    const KNOWN_BITS: u16 = 0b1_1111;

    pub const AGE: Self = Self(0b0_0001);
    pub const CONDITION: Self = Self(0b0_0010);
    pub const OCCUPANCY: Self = Self(0b0_0100);
    pub const DISEASE: Self = Self(0b0_1000);
    pub const GENETICS: Self = Self(0b1_0000);

    pub const AGE_CONDITION: Self = Self(Self::AGE.0 | Self::CONDITION.0);
    pub const AGE_OCCUPANCY: Self = Self(Self::AGE.0 | Self::OCCUPANCY.0);
    pub const CONDITION_OCCUPANCY: Self = Self(Self::CONDITION.0 | Self::OCCUPANCY.0);
    pub const AGE_CONDITION_OCCUPANCY: Self =
        Self(Self::AGE.0 | Self::CONDITION.0 | Self::OCCUPANCY.0);

    pub const fn from_bits(bits: u16) -> Result<Self, InformationPolicyError> {
        if bits == 0 || bits & !Self::KNOWN_BITS != 0 {
            return Err(InformationPolicyError::InvalidPopulationStatisticBits { bits });
        }
        Ok(Self(bits))
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    pub const fn contains_all(self, required: Self) -> bool {
        self.0 & required.0 == required.0
    }
}

/// One category of future-relevant ecological information.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcologicalInformation {
    Headcount,
    ExactLivingBiomass,
    AgeDistribution,
    ConditionDistribution,
    OccupancyDistribution,
    JointPopulationStatistics(PopulationStatisticSet),
    SpatialStructure,
    DiseaseState,
    GeneticSummary,
    ContactStructure,
    IndividualActiveState,
    PersistentIdentity,
    ExactConservationAccount(ConservedQuantity),
}

impl EcologicalInformation {
    /// Whether `self` carries at least the information named by `required`.
    ///
    /// The only V0 implication relation is set inclusion for joint population
    /// statistics. All other capabilities are explicit rather than inferred.
    pub const fn covers(self, required: Self) -> bool {
        match (self, required) {
            (
                Self::JointPopulationStatistics(available),
                Self::JointPopulationStatistics(required),
            ) => available.contains_all(required),
            _ => self as_discriminant_eq required,
        }
    }
}

// Const-friendly equality for non-payload variants without pretending distinct
// conservation quantities are interchangeable.
trait EcologicalInformationEq {
    fn same_information(self, other: Self) -> bool;
}

impl EcologicalInformationEq for EcologicalInformation {
    fn same_information(self, other: Self) -> bool {
        self == other
    }
}

/// Minimum canonical authority level required by a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EcologicalAuthorityLevel {
    Presentation,
    Coarse,
    Active,
    Persistent,
}

/// One closure policy accepted by a process requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClosureAcceptance {
    max_error_ppm: ErrorPpm,
    required_model: Option<ClosureModelVersion>,
    required_domain: Option<ClosureDomainToken>,
}

impl ClosureAcceptance {
    pub const fn new(
        max_error_ppm: ErrorPpm,
        required_model: Option<ClosureModelVersion>,
        required_domain: Option<ClosureDomainToken>,
    ) -> Self {
        Self {
            max_error_ppm,
            required_model,
            required_domain,
        }
    }

    pub const fn max_error_ppm(self) -> ErrorPpm {
        self.max_error_ppm
    }

    pub const fn required_model(self) -> Option<ClosureModelVersion> {
        self.required_model
    }

    pub const fn required_domain(self) -> Option<ClosureDomainToken> {
        self.required_domain
    }
}

/// Evidence strength accepted by one process requirement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EvidenceRequirement {
    Exact,
    ExactOrQualifiedClosure(ClosureAcceptance),
}

/// One qualified closure claim carried by a representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QualifiedClosureEvidence {
    model_version: ClosureModelVersion,
    domain: ClosureDomainToken,
    error_ppm: ErrorPpm,
    evidence_lineage: EvidenceLineageToken,
}

impl QualifiedClosureEvidence {
    pub const fn new(
        model_version: ClosureModelVersion,
        domain: ClosureDomainToken,
        error_ppm: ErrorPpm,
        evidence_lineage: EvidenceLineageToken,
    ) -> Self {
        Self {
            model_version,
            domain,
            error_ppm,
            evidence_lineage,
        }
    }

    pub const fn model_version(self) -> ClosureModelVersion {
        self.model_version
    }

    pub const fn domain(self) -> ClosureDomainToken {
        self.domain
    }

    pub const fn error_ppm(self) -> ErrorPpm {
        self.error_ppm
    }

    pub const fn evidence_lineage(self) -> EvidenceLineageToken {
        self.evidence_lineage
    }
}

/// Evidence carried by a representation for one information capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CapabilityEvidence {
    Exact,
    QualifiedClosure(QualifiedClosureEvidence),
    MeasurementOnly,
}

impl EvidenceRequirement {
    fn accepts(self, evidence: CapabilityEvidence) -> bool {
        match (self, evidence) {
            (Self::Exact, CapabilityEvidence::Exact) => true,
            (Self::Exact, _) => false,
            (Self::ExactOrQualifiedClosure(_), CapabilityEvidence::Exact) => true,
            (
                Self::ExactOrQualifiedClosure(policy),
                CapabilityEvidence::QualifiedClosure(closure),
            ) => {
                closure.error_ppm <= policy.max_error_ppm
                    && policy
                        .required_model
                        .is_none_or(|required| required == closure.model_version)
                    && policy
                        .required_domain
                        .is_none_or(|required| required == closure.domain)
            }
            (Self::ExactOrQualifiedClosure(_), CapabilityEvidence::MeasurementOnly) => false,
        }
    }
}

/// Stable process identity plus semantic process version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessKey {
    id: u128,
    version: u32,
}

impl ProcessKey {
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

/// One process requirement. Distinct closure policies for the same information
/// remain distinct requirements instead of being guessed into one compromise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessInformationRequirement {
    information: EcologicalInformation,
    evidence: EvidenceRequirement,
}

impl ProcessInformationRequirement {
    pub const fn exact(information: EcologicalInformation) -> Self {
        Self {
            information,
            evidence: EvidenceRequirement::Exact,
        }
    }

    pub const fn closure_allowed(
        information: EcologicalInformation,
        acceptance: ClosureAcceptance,
    ) -> Self {
        Self {
            information,
            evidence: EvidenceRequirement::ExactOrQualifiedClosure(acceptance),
        }
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn evidence(self) -> EvidenceRequirement {
        self.evidence
    }
}

/// Machine-readable information contract for one authoritative process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessInformationProfile {
    key: ProcessKey,
    minimum_authority: EcologicalAuthorityLevel,
    requirements: BTreeSet<ProcessInformationRequirement>,
}

impl ProcessInformationProfile {
    pub fn new(
        key: ProcessKey,
        minimum_authority: EcologicalAuthorityLevel,
        requirements: impl IntoIterator<Item = ProcessInformationRequirement>,
    ) -> Self {
        Self {
            key,
            minimum_authority,
            requirements: requirements.into_iter().collect(),
        }
    }

    pub const fn key(&self) -> ProcessKey {
        self.key
    }

    pub const fn minimum_authority(&self) -> EcologicalAuthorityLevel {
        self.minimum_authority
    }

    pub fn requirements(&self) -> &BTreeSet<ProcessInformationRequirement> {
        &self.requirements
    }
}

/// Stable representation identity plus schema/version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepresentationKey {
    id: u128,
    version: u32,
}

impl RepresentationKey {
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

/// Inspectable capability claims for one canonical representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepresentationCapabilities {
    key: RepresentationKey,
    authority_level: EcologicalAuthorityLevel,
    claims: BTreeMap<EcologicalInformation, BTreeSet<CapabilityEvidence>>,
}

impl RepresentationCapabilities {
    pub fn new(
        key: RepresentationKey,
        authority_level: EcologicalAuthorityLevel,
        claims: impl IntoIterator<Item = (EcologicalInformation, CapabilityEvidence)>,
    ) -> Self {
        let mut claims_by_information: BTreeMap<
            EcologicalInformation,
            BTreeSet<CapabilityEvidence>,
        > = BTreeMap::new();
        for (information, evidence) in claims {
            claims_by_information
                .entry(information)
                .or_default()
                .insert(evidence);
        }
        Self {
            key,
            authority_level,
            claims: claims_by_information,
        }
    }

    pub const fn key(&self) -> RepresentationKey {
        self.key
    }

    pub const fn authority_level(&self) -> EcologicalAuthorityLevel {
        self.authority_level
    }

    pub fn claims(&self) -> &BTreeMap<EcologicalInformation, BTreeSet<CapabilityEvidence>> {
        &self.claims
    }

    fn compatible_evidence(
        &self,
        required: EcologicalInformation,
    ) -> impl Iterator<Item = CapabilityEvidence> + '_ {
        self.claims.iter().flat_map(move |(available, evidence)| {
            available
                .covers(required)
                .then_some(evidence.iter().copied())
                .into_iter()
                .flatten()
        })
    }

    fn carries_information(&self, required: EcologicalInformation) -> bool {
        self.claims
            .keys()
            .copied()
            .any(|available| available.covers(required))
    }
}

/// Why one enabled process cannot execute authoritatively on a representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SufficiencyFailure {
    AuthorityLevelTooLow {
        process: ProcessKey,
        required: EcologicalAuthorityLevel,
        available: EcologicalAuthorityLevel,
    },
    MissingInformation {
        process: ProcessKey,
        requirement: ProcessInformationRequirement,
    },
    NoCompatibleAuthoritativeEvidence {
        process: ProcessKey,
        requirement: ProcessInformationRequirement,
    },
}

/// Deterministic evaluation result for an enabled process set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SufficiencyReport {
    failures: Vec<SufficiencyFailure>,
}

impl SufficiencyReport {
    pub fn is_sufficient(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn failures(&self) -> &[SufficiencyFailure] {
        &self.failures
    }
}

/// Evaluate all enabled process profiles against one representation.
///
/// Profiles are sorted by stable process key before evaluation, so caller,
/// thread, and insertion ordering cannot alter the result. Requirements within
/// each profile are stored canonically in a `BTreeSet` and remain independent;
/// incompatible closure policies are never silently collapsed together.
pub fn evaluate_processes<'a>(
    profiles: impl IntoIterator<Item = &'a ProcessInformationProfile>,
    representation: &RepresentationCapabilities,
) -> SufficiencyReport {
    let mut profiles = profiles.into_iter().collect::<Vec<_>>();
    profiles.sort_by_key(|profile| profile.key);

    let mut failures = Vec::new();
    for profile in profiles {
        if representation.authority_level < profile.minimum_authority {
            failures.push(SufficiencyFailure::AuthorityLevelTooLow {
                process: profile.key,
                required: profile.minimum_authority,
                available: representation.authority_level,
            });
        }

        for requirement in &profile.requirements {
            if !representation.carries_information(requirement.information) {
                failures.push(SufficiencyFailure::MissingInformation {
                    process: profile.key,
                    requirement: *requirement,
                });
                continue;
            }

            if !representation
                .compatible_evidence(requirement.information)
                .any(|evidence| requirement.evidence.accepts(evidence))
            {
                failures.push(SufficiencyFailure::NoCompatibleAuthoritativeEvidence {
                    process: profile.key,
                    requirement: *requirement,
                });
            }
        }
    }

    SufficiencyReport { failures }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InformationPolicyError {
    ErrorPpmOutOfRange { value: u32 },
    InvalidPopulationStatisticBits { bits: u16 },
}

impl fmt::Display for InformationPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ErrorPpmOutOfRange { value } => {
                write!(formatter, "closure error {value} ppm exceeds {PPM_SCALE} ppm")
            }
            Self::InvalidPopulationStatisticBits { bits } => {
                write!(formatter, "invalid population statistic bitset {bits:#x}")
            }
        }
    }
}

impl Error for InformationPolicyError {}

#[cfg(test)]
mod tests {
    use super::*;

    const DROUGHT: ProcessKey = ProcessKey::new(1, 1);
    const DISEASE: ProcessKey = ProcessKey::new(2, 1);
    const COLLISION: ProcessKey = ProcessKey::new(3, 1);
    const MARGINALS: RepresentationKey = RepresentationKey::new(10, 1);
    const STRATA: RepresentationKey = RepresentationKey::new(11, 1);

    fn exact_profile(
        process: ProcessKey,
        information: EcologicalInformation,
    ) -> ProcessInformationProfile {
        ProcessInformationProfile::new(
            process,
            EcologicalAuthorityLevel::Coarse,
            [ProcessInformationRequirement::exact(information)],
        )
    }

    fn closure(
        model: u32,
        domain: u128,
        error_ppm: u32,
        lineage: u128,
    ) -> CapabilityEvidence {
        CapabilityEvidence::QualifiedClosure(QualifiedClosureEvidence::new(
            ClosureModelVersion(model),
            ClosureDomainToken(domain),
            ErrorPpm::new(error_ppm).unwrap(),
            EvidenceLineageToken(lineage),
        ))
    }

    fn closure_requirement(
        information: EcologicalInformation,
        max_error_ppm: u32,
        model: Option<u32>,
        domain: Option<u128>,
    ) -> ProcessInformationRequirement {
        ProcessInformationRequirement::closure_allowed(
            information,
            ClosureAcceptance::new(
                ErrorPpm::new(max_error_ppm).unwrap(),
                model.map(ClosureModelVersion),
                domain.map(ClosureDomainToken),
            ),
        )
    }

    #[test]
    fn exact_capability_satisfies_exact_requirement() {
        let profile = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact)],
        );

        assert!(evaluate_processes([&profile], &representation).is_sufficient());
    }

    #[test]
    fn closure_cannot_satisfy_exact_requirement() {
        let profile = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, closure(1, 7, 1_000, 99))],
        );

        assert_eq!(
            evaluate_processes([&profile], &representation).failures(),
            &[SufficiencyFailure::NoCompatibleAuthoritativeEvidence {
                process: DROUGHT,
                requirement: ProcessInformationRequirement::exact(
                    EcologicalInformation::AgeDistribution
                ),
            }]
        );
    }

    #[test]
    fn compatible_closure_satisfies_permitted_requirement() {
        let requirement = closure_requirement(
            EcologicalInformation::AgeDistribution,
            20_000,
            Some(4),
            Some(55),
        );
        let profile = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [requirement],
        );
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, closure(4, 55, 12_000, 88))],
        );

        assert!(evaluate_processes([&profile], &representation).is_sufficient());
    }

    #[test]
    fn excessive_closure_error_rejects() {
        let requirement = closure_requirement(
            EcologicalInformation::AgeDistribution,
            10_000,
            Some(4),
            Some(55),
        );
        let profile = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [requirement],
        );
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, closure(4, 55, 10_001, 88))],
        );

        assert!(!evaluate_processes([&profile], &representation).is_sufficient());
    }

    #[test]
    fn wrong_closure_model_or_domain_rejects() {
        let requirement = closure_requirement(
            EcologicalInformation::AgeDistribution,
            20_000,
            Some(4),
            Some(55),
        );
        let profile = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [requirement],
        );
        let wrong_model = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, closure(5, 55, 1_000, 88))],
        );
        let wrong_domain = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, closure(4, 56, 1_000, 88))],
        );

        assert!(!evaluate_processes([&profile], &wrong_model).is_sufficient());
        assert!(!evaluate_processes([&profile], &wrong_domain).is_sufficient());
    }

    #[test]
    fn measurement_only_never_authorizes_process() {
        let requirement = closure_requirement(
            EcologicalInformation::AgeDistribution,
            20_000,
            None,
            None,
        );
        let profile = ProcessInformationProfile::new(
            DROUGHT,
            EcologicalAuthorityLevel::Coarse,
            [requirement],
        );
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::AgeDistribution,
                CapabilityEvidence::MeasurementOnly,
            )],
        );

        assert!(!evaluate_processes([&profile], &representation).is_sufficient());
    }

    #[test]
    fn missing_information_fails_closed() {
        let profile = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [],
        );

        assert_eq!(
            evaluate_processes([&profile], &representation).failures(),
            &[SufficiencyFailure::MissingInformation {
                process: DROUGHT,
                requirement: ProcessInformationRequirement::exact(
                    EcologicalInformation::AgeDistribution
                ),
            }]
        );
    }

    #[test]
    fn process_order_does_not_change_composed_result() {
        let drought = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let disease = exact_profile(
            DISEASE,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact)],
        );

        let a = evaluate_processes([&drought, &disease], &representation);
        let b = evaluate_processes([&disease, &drought], &representation);
        assert_eq!(a, b);
    }

    #[test]
    fn adding_joint_requirement_makes_marginal_representation_insufficient() {
        let drought = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let disease = exact_profile(
            DISEASE,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [
                (EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact),
                (
                    EcologicalInformation::ConditionDistribution,
                    CapabilityEvidence::Exact,
                ),
            ],
        );

        assert!(evaluate_processes([&drought], &representation).is_sufficient());
        assert!(!evaluate_processes([&drought, &disease], &representation).is_sufficient());
    }

    #[test]
    fn richer_joint_capability_covers_weaker_joint_requirement() {
        let disease = exact_profile(
            DISEASE,
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
        );
        let representation = RepresentationCapabilities::new(
            STRATA,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
                ),
                CapabilityEvidence::Exact,
            )],
        );

        assert!(evaluate_processes([&disease], &representation).is_sufficient());
    }

    #[test]
    fn authority_level_is_an_independent_requirement() {
        let collision = ProcessInformationProfile::new(
            COLLISION,
            EcologicalAuthorityLevel::Active,
            [ProcessInformationRequirement::exact(
                EcologicalInformation::IndividualActiveState,
            )],
        );
        let coarse = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(
                EcologicalInformation::IndividualActiveState,
                CapabilityEvidence::Exact,
            )],
        );

        assert_eq!(
            evaluate_processes([&collision], &coarse).failures(),
            &[SufficiencyFailure::AuthorityLevelTooLow {
                process: COLLISION,
                required: EcologicalAuthorityLevel::Active,
                available: EcologicalAuthorityLevel::Coarse,
            }]
        );
    }

    #[test]
    fn evaluation_is_read_only() {
        let profile = exact_profile(DROUGHT, EcologicalInformation::AgeDistribution);
        let representation = RepresentationCapabilities::new(
            MARGINALS,
            EcologicalAuthorityLevel::Coarse,
            [(EcologicalInformation::AgeDistribution, CapabilityEvidence::Exact)],
        );
        let before_profile = profile.clone();
        let before_representation = representation.clone();

        let _ = evaluate_processes([&profile], &representation);

        assert_eq!(profile, before_profile);
        assert_eq!(representation, before_representation);
    }

    #[test]
    fn policy_scalars_fail_closed_outside_valid_range() {
        assert_eq!(ErrorPpm::new(PPM_SCALE).unwrap().get(), PPM_SCALE);
        assert_eq!(
            ErrorPpm::new(PPM_SCALE + 1),
            Err(InformationPolicyError::ErrorPpmOutOfRange {
                value: PPM_SCALE + 1
            })
        );
        assert!(PopulationStatisticSet::from_bits(0).is_err());
        assert!(PopulationStatisticSet::from_bits(0x80).is_err());
    }
}
