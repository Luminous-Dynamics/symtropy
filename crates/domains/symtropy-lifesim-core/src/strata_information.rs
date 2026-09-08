// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Schema-owned information capabilities for V1 sparse population strata.
//!
//! [`crate::strata::StratifiedPopulationState`] retains exact joint
//! age × condition × occupancy information plus exact aggregate count and
//! living biomass. The profile deliberately does not imply disease, genetics,
//! contact structure, individual authority, persistent identity, or
//! conservation-ledger account ownership.

use crate::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    PopulationStatisticSet, RepresentationCapabilities, RepresentationKey,
};
use crate::information_registry::{InformationPolicyRegistryBuilder, InformationRegistryError};
use crate::strata::StratifiedPopulationState;

/// Stable representation identity for the V1 sparse-strata schema.
///
/// The semantic version is bound directly to the canonical strata schema
/// version. Any incompatible change to retained axes or extensive-state meaning
/// must advance the strata schema and therefore this representation version.
pub const STRATIFIED_POPULATION_V1: RepresentationKey = RepresentationKey::new(
    0x7374726174615f706f705f76315f7631,
    StratifiedPopulationState::SCHEMA_VERSION as u32,
);

/// Exact information carried by the V1 sparse-strata population representation.
pub fn strata_population_v1_capabilities() -> RepresentationCapabilities {
    RepresentationCapabilities::new(
        STRATIFIED_POPULATION_V1,
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
            (
                EcologicalInformation::JointPopulationStatistics(
                    PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
                ),
                CapabilityEvidence::Exact,
            ),
        ],
    )
}

/// Register the V1 sparse-strata capability contract into one bootstrap
/// information-policy registry.
pub fn register_strata_population_v1(
    builder: &mut InformationPolicyRegistryBuilder,
) -> Result<(), InformationRegistryError> {
    builder.register_representation(strata_population_v1_capabilities())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information_registry::InformationPolicyRegistryKey;

    #[test]
    fn representation_version_tracks_strata_schema_version() {
        assert_eq!(
            STRATIFIED_POPULATION_V1.version(),
            u32::from(StratifiedPopulationState::SCHEMA_VERSION)
        );
    }

    #[test]
    fn schema_profile_contains_only_the_retained_full_joint_axes() {
        let profile = strata_population_v1_capabilities();
        let claims = profile.claims();

        assert_eq!(profile.authority_level(), EcologicalAuthorityLevel::Coarse);
        assert_eq!(claims.len(), 6);
        assert!(claims.contains_key(&EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION_OCCUPANCY,
        )));
        assert!(!claims.contains_key(&EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE.union(PopulationStatisticSet::DISEASE),
        )));
        assert!(!claims.contains_key(&EcologicalInformation::IndividualActiveState));
        assert!(!claims.contains_key(&EcologicalInformation::PersistentIdentity));
    }

    #[test]
    fn registry_registration_is_idempotent_for_the_same_schema_profile() {
        let mut builder = InformationPolicyRegistryBuilder::new(
            InformationPolicyRegistryKey::new(0x7374726174615f7265675f746573745f, 1),
        );

        register_strata_population_v1(&mut builder).unwrap();
        register_strata_population_v1(&mut builder).unwrap();

        let registry = builder.seal();
        let registered = registry
            .resolve_representation(STRATIFIED_POPULATION_V1)
            .unwrap();
        assert_eq!(
            registered.capabilities(),
            &strata_population_v1_capabilities()
        );
    }
}
