// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Executable contract for the V1 sparse-strata population information profile.
//!
//! The oracle consumes the schema-owned product adapter rather than duplicating
//! its capability map, so semantic drift in the adapter changes the observable
//! contract directly.

use symtropy_lifesim_core::conservation::ConservedQuantity;
use symtropy_lifesim_core::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    PopulationStatisticSet, ProcessInformationProfile, ProcessInformationRequirement, ProcessKey,
};
use symtropy_lifesim_core::information_registry::{
    InformationPolicyRegistry, InformationPolicyRegistryBuilder, InformationPolicyRegistryKey,
};
use symtropy_lifesim_core::strata::StratifiedPopulationState;
use symtropy_lifesim_core::strata_information::{
    register_strata_population_v1, strata_population_v1_capabilities, STRATIFIED_POPULATION_V1,
};

const REGISTRY: InformationPolicyRegistryKey =
    InformationPolicyRegistryKey::new(0x7374726174615f6361705f7265675f31, 1);

const PAIRWISE_PROCESS: ProcessKey = ProcessKey::new(0x2001, 1);
const FULL_JOINT_PROCESS: ProcessKey = ProcessKey::new(0x2002, 1);
const DISEASE_PROCESS: ProcessKey = ProcessKey::new(0x2003, 1);
const ACTIVE_PROCESS: ProcessKey = ProcessKey::new(0x2004, 1);
const PERSISTENT_PROCESS: ProcessKey = ProcessKey::new(0x2005, 1);
const CONSERVATION_PROCESS: ProcessKey = ProcessKey::new(0x2006, 1);

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
    register_strata_population_v1(&mut builder).unwrap();
    for process in processes {
        builder.register_process(process).unwrap();
    }
    builder.seal()
}

#[test]
fn representation_key_is_bound_to_the_actual_strata_schema_version() {
    assert_eq!(
        STRATIFIED_POPULATION_V1.version(),
        u32::from(StratifiedPopulationState::SCHEMA_VERSION)
    );
}

#[test]
fn profile_advertises_only_exact_coarse_strata_information() {
    let capabilities = strata_population_v1_capabilities();
    let claims = capabilities.claims();

    assert_eq!(capabilities.key(), STRATIFIED_POPULATION_V1);
    assert_eq!(
        capabilities.authority_level(),
        EcologicalAuthorityLevel::Coarse
    );
    assert_eq!(claims.len(), 6);

    for information in [
        EcologicalInformation::Headcount,
        EcologicalInformation::ExactLivingBiomass,
        EcologicalInformation::AgeDistribution,
        EcologicalInformation::ConditionDistribution,
        EcologicalInformation::OccupancyDistribution,
        EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
        ),
    ] {
        let evidence = claims.get(&information).expect("missing exact strata claim");
        assert_eq!(evidence.len(), 1);
        assert!(evidence.contains(&CapabilityEvidence::Exact));
    }
}

#[test]
fn full_joint_claim_satisfies_all_retained_pairwise_requirements() {
    let pairwise = exact_process(
        PAIRWISE_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_CONDITION,
            ),
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::AGE_OCCUPANCY,
            ),
            EcologicalInformation::JointPopulationStatistics(
                PopulationStatisticSet::CONDITION_OCCUPANCY,
            ),
        ],
    );
    let registry = registry_with([pairwise]);

    let report = registry
        .evaluate_registered([PAIRWISE_PROCESS], STRATIFIED_POPULATION_V1)
        .unwrap();
    assert!(report.is_sufficient(), "failures={:?}", report.report().failures());
}

#[test]
fn exact_full_joint_requirement_is_satisfied() {
    let full = exact_process(
        FULL_JOINT_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
        )],
    );
    let registry = registry_with([full]);

    assert!(
        registry
            .evaluate_registered([FULL_JOINT_PROCESS], STRATIFIED_POPULATION_V1)
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn disease_covariance_is_not_invented_by_the_existing_full_joint_axes() {
    let age_disease = PopulationStatisticSet::AGE.union(PopulationStatisticSet::DISEASE);
    let disease = exact_process(
        DISEASE_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [EcologicalInformation::JointPopulationStatistics(age_disease)],
    );
    let registry = registry_with([disease]);

    assert!(
        !registry
            .evaluate_registered([DISEASE_PROCESS], STRATIFIED_POPULATION_V1)
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn richer_authority_levels_still_fail_closed() {
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
            .evaluate_registered([ACTIVE_PROCESS], STRATIFIED_POPULATION_V1)
            .unwrap()
            .is_sufficient()
    );
    assert!(
        !registry
            .evaluate_registered([PERSISTENT_PROCESS], STRATIFIED_POPULATION_V1)
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn exact_biomass_total_is_not_a_conservation_ledger_account() {
    let process = exact_process(
        CONSERVATION_PROCESS,
        EcologicalAuthorityLevel::Coarse,
        [EcologicalInformation::ExactConservationAccount(
            ConservedQuantity::BiomassMass,
        )],
    );
    let registry = registry_with([process]);

    assert!(
        !registry
            .evaluate_registered([CONSERVATION_PROCESS], STRATIFIED_POPULATION_V1)
            .unwrap()
            .is_sufficient()
    );
}

#[test]
fn profile_does_not_depend_on_population_contents_or_iteration_order() {
    let a = strata_population_v1_capabilities();
    let b = strata_population_v1_capabilities();

    assert_eq!(a, b);
    assert_eq!(a.key(), STRATIFIED_POPULATION_V1);
}
