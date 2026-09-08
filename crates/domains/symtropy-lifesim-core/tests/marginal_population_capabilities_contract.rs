// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable contract for the V0 marginal PopulationState information profile.
//!
//! This is deliberately an oracle before the final schema-owned adapter from
//! #265. It freezes what the current coarse representation may claim through
//! the sealed information-policy registry and, equally importantly, what it
//! must not claim.

use symtropy_lifesim_core::conservation::ConservedQuantity;
use symtropy_lifesim_core::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    PopulationStatisticSet, ProcessInformationProfile, ProcessInformationRequirement, ProcessKey,
    RepresentationCapabilities, RepresentationKey,
};
use symtropy_lifesim_core::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
};

const REGISTRY: InformationPolicyRegistryKey =
    InformationPolicyRegistryKey::new(0x6d6172675f6361705f7265675f763031, 1);
const MARGINAL_POPULATION_V0: RepresentationKey =
    RepresentationKey::new(0x6d617267696e616c5f706f705f763031, 1);

const MARGINAL_PROCESS: ProcessKey = ProcessKey::new(0x1001, 1);
const JOINT_PROCESS: ProcessKey = ProcessKey::new(0x1002, 1);
const ACTIVE_PROCESS: ProcessKey = ProcessKey::new(0x1003, 1);
const PERSISTENT_PROCESS: ProcessKey = ProcessKey::new(0x1004, 1);
const CONSERVATION_PROCESS: ProcessKey = ProcessKey::new(0x1005, 1);

fn marginal_population_v0_capabilities() -> RepresentationCapabilities {
    RepresentationCapabilities::new(
        MARGINAL_POPULATION_V0,
        EcologicalAuthorityLevel::Coarse,
        [
            (EcologicalInformation::Headcount, CapabilityEvidence::Exact),
            (
                EcologicalInformation::ExactLivingBiomass,
                CapabilityEvidence::Exact,
            ),
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
                CapabilityEvidence::Exact,
            ),
        ],
    )
}

fn exact_process(
    key: ProcessKey,
    authority: EcologicalAuthorityLevel,
    requirements: impl IntoIterator<Item = EcologicalInformation>,
) -> ProcessInformationProfile {
    ProcessInformationProfile::new(
        key,
        authority,
        requirements
            .into_iter()
            .map(ProcessInformationRequirement::exact),
    )
}

fn registry_with(
    processes: impl IntoIterator<Item = ProcessInformationProfile>,
) -> InformationPolicyRegistry {
    let mut builder = InformationPolicyRegistryBuilder::new(REGISTRY);
    builder
        .register_representation(marginal_population_v0_capabilities())
        .unwrap();
    for process in processes {
        builder.register_process(process).unwrap();
    }
    builder.seal()
}

#[test]
fn profile_advertises_only_the_five_exact_marginal_capabilities() {
    let capabilities = marginal_population_v0_capabilities();
    let claims = capabilities.claims();

    assert_eq!(capabilities.key(), MARGINAL_POPULATION_V0);
    assert_eq!(
        capabilities.authority_level(),
        EcologicalAuthorityLevel::Coarse
    );
    assert_eq!(claims.len(), 5);

    for information in [
        EcologicalInformation::Headcount,
        EcologicalInformation::ExactLivingBiomass,
        EcologicalInformation::AgeDistribution,
        EcologicalInformation::ConditionDistribution,
        EcologicalInformation::OccupancyDistribution,
    ] {
        let evidence = claims.get(&information).expect("missing exact marginal claim");
        assert_eq!(evidence.len(), 1);
        assert!(evidence.contains(&CapabilityEvidence::Exact));
    }
}

#[test]
fn profile_does_not_advertise_covariance_or_richer_authority() {
    let capabilities = marginal_population_v0_capabilities();
    let claims = capabilities.claims();

    for unavailable in [
        EcologicalInformation::JointPopulationStatistics(PopulationStatisticSet::AGE_CONDITION),
        EcologicalInformation::JointPopulationStatistics(PopulationStatisticSet::AGE_OCCUPANCY),
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::CONDITION_OCCUPANCY,
        ),
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
        ),
        EcologicalInformation::DiseaseState,
        EcologicalInformation::GeneticSummary,
        EcologicalInformation::ContactStructure,
        EcologicalInformation::IndividualActiveState,
        EcologicalInformation::PersistentIdentity,
        EcologicalInformation::ExactConservationAccount(ConservedQuantity::BiomassMass),
    ] {
        assert!(
            !claims.contains_key(&unavailable),
            "marginal schema overclaimed {unavailable:?}"
        );
    }
}

#[test]
fn marginal_only_process_is_authorized_through_the_sealed_registry() {
    let process = exact_process(
        MARGINAL_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [
            EcologicalInformation::Headcount,
            EcologicalInformation::AgeDistribution,
        ],
    );
    let registry = registry_with([process]);

    let report = registry
        .evaluate_registered([MARGINAL_PROCESS], MARGINAL_POPULATION_V0)
        .unwrap();
    assert!(report.is_sufficient(), "failures={:?}", report.report().failures());
}

#[test]
fn joint_process_is_rejected_even_when_both_individual_marginals_exist() {
    let marginal = exact_process(
        MARGINAL_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [
            EcologicalInformation::AgeDistribution,
            EcologicalInformation::ConditionDistribution,
        ],
    );
    let joint = exact_process(
        JOINT_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION,
        )],
    );
    let registry = registry_with([marginal, joint]);

    assert!(
        registry
            .evaluate_registered([MARGINAL_PROCESS], MARGINAL_POPULATION_V0)
            .unwrap()
            .is_sufficient()
    );
    assert!(
        !registry
            .evaluate_registered(
                [MARGINAL_PROCESS, JOINT_PROCESS],
                MARGINAL_POPULATION_V0,
            )
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn active_and_persistent_authority_requirements_fail_closed() {
    let active = exact_process(
        ACTIVE_PROCESS,
        EcologicalAuthorityLevel::Active,
        [EcologicalInformation::Headcount],
    );
    let persistent = exact_process(
        PERSISTENT_PROCESS,
        EcologicalAuthorityLevel::Persistent,
        [EcologicalInformation::Headcount],
    );
    let registry = registry_with([active, persistent]);

    assert!(
        !registry
            .evaluate_registered([ACTIVE_PROCESS], MARGINAL_POPULATION_V0)
            .unwrap()
            .is_sufficient()
    );
    assert!(
        !registry
            .evaluate_registered([PERSISTENT_PROCESS], MARGINAL_POPULATION_V0)
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn aggregate_biomass_is_not_a_conservation_account() {
    let process = exact_process(
        CONSERVATION_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [EcologicalInformation::ExactConservationAccount(
            ConservedQuantity::BiomassMass,
        )],
    );
    let registry = registry_with([process]);

    let report = registry
        .evaluate_registered([CONSERVATION_PROCESS], MARGINAL_POPULATION_V0)
        .unwrap();
    assert!(!report.is_sufficient());
}

#[test]
fn capability_profile_is_schema_metadata_not_runtime_population_state() {
    let a = marginal_population_v0_capabilities();
    let b = marginal_population_v0_capabilities();

    assert_eq!(a, b);
    assert_eq!(a.key(), MARGINAL_POPULATION_V0);
}
