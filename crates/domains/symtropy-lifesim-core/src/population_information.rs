// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Schema-owned information capabilities for the V0 marginal population model.
//!
//! The current [`crate::population::PopulationState`] stores exact aggregate
//! headcount and living biomass plus independent age, condition, and occupancy
//! marginals. It does not retain covariance, individual identity, contact
//! structure, or conservation-ledger account ownership.
//!
//! Canonical runtime code should register this profile through the sealed
//! information-policy registry rather than constructing an ad hoc capability
//! map at the call site.

use crate::information::{
    CapabilityEvidence, EcologicalAuthorityLevel, EcologicalInformation,
    RepresentationCapabilities, RepresentationKey,
};
use crate::information_registry::{InformationPolicyRegistryBuilder, InformationRegistryError};

/// Stable V0 representation identity for the marginal `PopulationState` schema.
///
/// The numeric identity is frozen by the capability oracle. Semantic changes
/// to the advertised information contract require a new representation key or
/// version rather than silently changing the meaning of V0.
pub const MARGINAL_POPULATION_V0: RepresentationKey =
    RepresentationKey::new(0x6d617267696e616c5f706f705f763031, 1);

/// Exact information carried by the V0 marginal population representation.
pub fn marginal_population_v0_capabilities() -> RepresentationCapabilities {
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

/// Register the V0 marginal population capability contract into one bootstrap
/// information-policy registry.
///
/// The builder owns conflict/idempotency semantics. Callers choose whether this
/// schema participates in a registry generation, but they do not rewrite the
/// profile itself.
pub fn register_marginal_population_v0(
    builder: &mut InformationPolicyRegistryBuilder,
) -> Result<(), InformationRegistryError> {
    builder.register_representation(marginal_population_v0_capabilities())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::PopulationStatisticSet;
    use crate::information_registry::InformationPolicyRegistryKey;

    #[test]
    fn schema_profile_is_exactly_coarse_marginal_information() {
        let profile = marginal_population_v0_capabilities();
        let claims = profile.claims();

        assert_eq!(profile.key(), MARGINAL_POPULATION_V0);
        assert_eq!(profile.authority_level(), EcologicalAuthorityLevel::Coarse);
        assert_eq!(claims.len(), 5);
        assert!(!claims.contains_key(&EcologicalInformation::JointPopulationStatistics(
            PopulationStatisticSet::AGE_CONDITION,
        )));
        assert!(!claims.contains_key(&EcologicalInformation::IndividualActiveState));
        assert!(!claims.contains_key(&EcologicalInformation::PersistentIdentity));
    }

    #[test]
    fn registry_registration_is_idempotent_for_the_same_schema_profile() {
        let mut builder = InformationPolicyRegistryBuilder::new(
            InformationPolicyRegistryKey::new(0x6d6172675f7265675f746573745f7631, 1),
        );

        register_marginal_population_v0(&mut builder).unwrap();
        register_marginal_population_v0(&mut builder).unwrap();

        let registry = builder.seal();
        let registered = registry
            .resolve_representation(MARGINAL_POPULATION_V0)
            .unwrap();
        assert_eq!(
            registered.capabilities(),
            &marginal_population_v0_capabilities()
        );
    }
}
